//! The sidecar store: the container codec, freshness, the atomic write and
//! the rules that keep a sidecar from ever failing or changing a save
//! (design 1.3.10, P7).

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::test_fixtures::{debug_game_dir, saved_game_seed_bytes};

/// A save image of `avatar` on the given day and minute.
fn image(avatar: &[u8], year: u16, day: u8, minute: u8) -> Vec<u8> {
    let mut image = saved_game_seed_bytes(0, 0, 15, 15);
    let name = &mut image[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 9];
    name.fill(0);
    name[..avatar.len()].copy_from_slice(avatar);
    image[SAVE_YEAR_OFFSET..SAVE_YEAR_OFFSET + 2].copy_from_slice(&year.to_le_bytes());
    image[SAVE_MONTH_OFFSET] = 3;
    image[SAVE_DAY_OFFSET] = day;
    image[SAVE_HOUR_OFFSET] = 10;
    image[SAVE_MINUTE_OFFSET] = minute;
    image
}

fn stamp(avatar: &[u8], year: u16, day: u8, minute: u8) -> SaveStamp {
    SaveStamp::of_image(&image(avatar, year, day, minute))
}

/// A save folder holding `image` as `SAVED.GAM`, and its store.
fn store_with(image: &[u8]) -> (PathBuf, SidecarStore) {
    let dir = debug_game_dir();
    fs::write(dir.join(SAVED_GAM_FILENAME), image).unwrap();
    let store = SidecarStore::open(&dir).expect("a writable save folder opens");
    (dir, store)
}

fn sidecar_path(dir: &Path, tag: [u8; 4]) -> PathBuf {
    dir.join(SIDECAR_DIR).join(sidecar_file_name(tag).unwrap())
}

fn payload(version: u16, bytes: &[u8]) -> Option<SidecarPayload> {
    Some(SidecarPayload {
        version,
        bytes: bytes.to_vec(),
    })
}

#[test]
fn the_folder_and_file_names_are_the_designed_ones() {
    assert_eq!(SIDECAR_DIR, ".u5-engine-enh");
    assert_eq!(sidecar_file_name(*b"AMAP"), Some("automap.u5e"));
    assert_eq!(sidecar_file_name(*b"META"), Some("save-meta.u5e"));
    assert_eq!(sidecar_file_name(*b"QJRN"), Some("journal.u5e"));
    assert_eq!(sidecar_file_name(*b"GEMK"), Some("gemmap.u5e"));
    assert_eq!(sidecar_file_name(*b"XXXX"), None);
    assert_eq!(SIDECAR_FILES.len(), 4, "no other per-save file exists");
}

#[test]
fn the_codec_round_trips_with_the_documented_layout() {
    let stamp = stamp(b"Ava", 139, 7, 41);
    let bytes = encode_container(TAG_AUTOMAP, &stamp, 3, b"payload").unwrap();
    assert_eq!(bytes.len(), 42 + 7 + 8);
    assert_eq!(&bytes[0..4], b"U5EH");
    assert_eq!(bytes[4], CONTAINER_VERSION);
    assert_eq!(&bytes[5..9], b"AMAP");
    assert_eq!(&bytes[9..11], &3_u16.to_le_bytes());
    assert_eq!(&bytes[11..19], &stamp.fnv.to_le_bytes());
    assert_eq!(&bytes[19..23], &stamp.len.to_le_bytes());
    assert_eq!(&bytes[23..32], &stamp.avatar);
    assert_eq!(&bytes[32..38], &stamp.date);
    assert_eq!(&bytes[38..42], &7_u32.to_le_bytes());
    assert_eq!(&bytes[42..49], b"payload");
    assert_eq!(&bytes[49..], &fnv1a64(&bytes[..49]).to_le_bytes());

    assert_eq!(
        decode_container(&bytes),
        Ok(Container {
            tag: TAG_AUTOMAP,
            payload_version: 3,
            stamp,
            payload: b"payload".to_vec(),
        })
    );
    let empty = encode_container(TAG_JOURNAL, &stamp, 1, &[]).unwrap();
    assert_eq!(decode_container(&empty).unwrap().payload, Vec::<u8>::new());
}

#[test]
fn truncation_bad_magic_bad_checksum_and_trailing_bytes_are_refused() {
    let stamp = stamp(b"Ava", 139, 7, 41);
    let bytes = encode_container(TAG_AUTOMAP, &stamp, 1, b"memory").unwrap();
    for len in 0..bytes.len() {
        assert_eq!(
            decode_container(&bytes[..len]),
            Err(ContainerError::Truncated),
            "cut at {len}"
        );
    }

    let mut bad_magic = bytes.clone();
    bad_magic[0] = b'X';
    assert_eq!(decode_container(&bad_magic), Err(ContainerError::BadMagic));

    let mut zero_version = bytes.clone();
    zero_version[4] = 0;
    assert_eq!(
        decode_container(&zero_version),
        Err(ContainerError::BadVersion)
    );

    for offset in [5, 12, 30, 44, bytes.len() - 1] {
        let mut flipped = bytes.clone();
        flipped[offset] ^= 0x10;
        assert_eq!(
            decode_container(&flipped),
            Err(ContainerError::BadChecksum),
            "byte {offset} flipped"
        );
    }

    let mut long = bytes.clone();
    long.push(0);
    assert_eq!(decode_container(&long), Err(ContainerError::TrailingBytes));
}

#[test]
fn a_newer_container_is_never_read_overwritten_or_renamed() {
    let now = stamp(b"Ava", 139, 7, 41);
    let (dir, store) = store_with(&image(b"Ava", 139, 7, 41));
    let mut newer = encode_container(TAG_AUTOMAP, &now, 1, b"from a newer build").unwrap();
    newer[4] = CONTAINER_VERSION + 1;
    assert_eq!(
        decode_container(&newer),
        Err(ContainerError::NewerContainer(CONTAINER_VERSION + 1))
    );
    let path = sidecar_path(&dir, TAG_AUTOMAP);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, &newer).unwrap();

    assert_eq!(store.read(TAG_AUTOMAP, &now), (Freshness::Missing, None));
    assert_eq!(
        store.write(TAG_AUTOMAP, &now, 1, b"older build").unwrap(),
        SidecarWrite::KeptNewer
    );
    assert_eq!(fs::read(&path).unwrap(), newer, "the newer file is kept");
    assert!(!path.with_extension("u5e.bad").exists());
}

#[test]
fn a_newer_payload_is_read_but_never_overwritten() {
    let now = stamp(b"Ava", 139, 7, 41);
    let (dir, store) = store_with(&image(b"Ava", 139, 7, 41));
    assert_eq!(
        store.write(TAG_JOURNAL, &now, 2, b"version two").unwrap(),
        SidecarWrite::Written
    );
    let written = fs::read(sidecar_path(&dir, TAG_JOURNAL)).unwrap();

    assert_eq!(
        store.write(TAG_JOURNAL, &now, 1, b"version one").unwrap(),
        SidecarWrite::KeptNewer
    );
    assert_eq!(fs::read(sidecar_path(&dir, TAG_JOURNAL)).unwrap(), written);
    assert_eq!(
        store.read(TAG_JOURNAL, &now),
        (Freshness::Fresh, payload(2, b"version two")),
        "the reader still gets the memory, to use in RAM"
    );

    assert_eq!(
        store.write(TAG_JOURNAL, &now, 2, b"same version").unwrap(),
        SidecarWrite::Written
    );
    assert_eq!(
        store.write(TAG_JOURNAL, &now, 3, b"newer still").unwrap(),
        SidecarWrite::Written
    );
    assert_eq!(store.read(TAG_JOURNAL, &now).1, payload(3, b"newer still"));
}

#[test]
fn freshness_compares_the_stamp_with_the_save_on_disk() {
    let saved = stamp(b"Ava", 139, 7, 41);
    assert_eq!(Freshness::of(&saved, &saved), Freshness::Fresh);
    assert_eq!(
        Freshness::of(&saved, &stamp(b"Ava", 139, 7, 42)),
        Freshness::SameLineage,
        "same Avatar, later: saved elsewhere"
    );
    assert_eq!(
        Freshness::of(&saved, &stamp(b"Ava", 140, 1, 0)),
        Freshness::SameLineage
    );
    assert_eq!(
        Freshness::of(&saved, &stamp(b"Iolo", 139, 7, 42)),
        Freshness::Foreign,
        "another Avatar"
    );
    assert_eq!(
        Freshness::of(&saved, &stamp(b"Ava", 139, 7, 40)),
        Freshness::Foreign,
        "same name, earlier date: another character"
    );
    assert_eq!(
        Freshness::of(&saved, &stamp(b"Ava", 138, 9, 59)),
        Freshness::Foreign,
        "the year outranks the day"
    );
}

#[test]
fn the_store_reads_each_freshness_case() {
    let saved_image = image(b"Ava", 139, 7, 41);
    let saved = SaveStamp::of_image(&saved_image);
    let (dir, store) = store_with(&saved_image);

    assert_eq!(store.read(TAG_AUTOMAP, &saved), (Freshness::Missing, None));
    store.write(TAG_AUTOMAP, &saved, 1, b"map").unwrap();
    assert_eq!(
        store.read(TAG_AUTOMAP, &saved),
        (Freshness::Fresh, payload(1, b"map"))
    );
    assert_eq!(
        store.read(TAG_AUTOMAP, &stamp(b"Ava", 139, 8, 0)),
        (Freshness::SameLineage, payload(1, b"map"))
    );
    assert_eq!(
        store.read(TAG_AUTOMAP, &stamp(b"Iolo", 139, 8, 0)),
        (Freshness::Foreign, payload(1, b"map"))
    );
    assert_eq!(
        store.read(*b"XXXX", &saved),
        (Freshness::Missing, None),
        "a tag no feature owns"
    );

    let path = sidecar_path(&dir, TAG_AUTOMAP);
    let mut damaged = fs::read(&path).unwrap();
    damaged[43] ^= 1;
    fs::write(&path, &damaged).unwrap();
    assert_eq!(store.read(TAG_AUTOMAP, &saved), (Freshness::Corrupt, None));
    assert!(!path.exists(), "the corrupt file is moved aside");
    let bad = dir.join(SIDECAR_DIR).join("automap.u5e.bad");
    assert_eq!(fs::read(&bad).unwrap(), damaged);
    assert_eq!(store.read(TAG_AUTOMAP, &saved), (Freshness::Missing, None));
    store.write(TAG_AUTOMAP, &saved, 1, b"map again").unwrap();
    assert_eq!(
        store.read(TAG_AUTOMAP, &saved),
        (Freshness::Fresh, payload(1, b"map again")),
        "and replaced at the next save"
    );

    fs::write(
        &path,
        encode_container(TAG_JOURNAL, &saved, 1, b"x").unwrap(),
    )
    .unwrap();
    assert_eq!(
        store.read(TAG_AUTOMAP, &saved),
        (Freshness::Corrupt, None),
        "another feature's container under this name"
    );
}

#[test]
fn a_new_character_resets_the_enabled_features_and_reads_the_rest_as_foreign() {
    let played = stamp(b"Ava", 139, 7, 41);
    let (dir, store) = store_with(&image(b"Ava", 139, 7, 41));
    store.write(TAG_AUTOMAP, &played, 1, b"map").unwrap();
    store.write(TAG_JOURNAL, &played, 1, b"clues").unwrap();

    // Character creation writes a fresh SAVED.GAM at the factory date,
    // with the same name; only the automap is on.
    let new_image = image(b"Ava", 139, 4, 0);
    fs::write(dir.join(SAVED_GAM_FILENAME), &new_image).unwrap();
    let fresh = SaveStamp::of_image(&new_image);
    store.reset(&[TAG_AUTOMAP]).unwrap();
    store.reset(&[TAG_AUTOMAP, TAG_GEM_MAP]).unwrap();

    assert_eq!(store.read(TAG_AUTOMAP, &fresh), (Freshness::Missing, None));
    assert_eq!(
        store.read(TAG_JOURNAL, &fresh),
        (Freshness::Foreign, payload(1, b"clues")),
        "a disabled feature's file is untouched, and is another character's"
    );
}

#[test]
fn a_crash_between_the_temp_file_and_the_rename_keeps_the_old_sidecar() {
    let first = stamp(b"Ava", 139, 7, 41);
    let second = stamp(b"Ava", 139, 7, 50);
    let (dir, store) = store_with(&image(b"Ava", 139, 7, 41));
    store.write(TAG_AUTOMAP, &first, 1, b"old map").unwrap();
    let old = fs::read(sidecar_path(&dir, TAG_AUTOMAP)).unwrap();

    store
        .write_staged(TAG_AUTOMAP, &second, 1, b"new map", false)
        .unwrap();
    let temp = dir.join(SIDECAR_DIR).join("automap.u5e.tmp");
    assert!(temp.exists(), "the crash left the temporary file");
    assert_eq!(fs::read(sidecar_path(&dir, TAG_AUTOMAP)).unwrap(), old);
    assert_eq!(
        store.read(TAG_AUTOMAP, &first),
        (Freshness::Fresh, payload(1, b"old map")),
        "the old sidecar is whole and still matches its save"
    );

    store.write(TAG_AUTOMAP, &second, 1, b"new map").unwrap();
    assert!(!temp.exists());
    assert_eq!(
        store.read(TAG_AUTOMAP, &second),
        (Freshness::Fresh, payload(1, b"new map"))
    );

    store
        .write_staged(TAG_AUTOMAP, &second, 1, b"again", false)
        .unwrap();
    store.reset(&[TAG_AUTOMAP]).unwrap();
    assert!(!temp.exists(), "a reset clears a stale temporary file");
}

#[test]
fn a_write_failure_never_fails_or_changes_the_save() {
    let saved_image = image(b"Ava", 139, 7, 41);
    let (dir, store) = store_with(&saved_image);
    // A file where the folder should be: every write fails.
    fs::write(dir.join(SIDECAR_DIR), b"not a folder").unwrap();

    let saved = SaveStamp::of_image(&saved_image);
    assert!(store.write(TAG_AUTOMAP, &saved, 1, b"map").is_err());
    assert!(store.write(*b"XXXX", &saved, 1, b"map").is_err());
    assert_eq!(fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap(), saved_image);
    assert_eq!(store.read(TAG_AUTOMAP, &saved), (Freshness::Missing, None));
}

#[test]
fn the_store_refuses_a_read_only_save_and_a_missing_folder() {
    let saved_image = image(b"Ava", 139, 7, 41);
    let (dir, store) = store_with(&saved_image);
    assert_eq!(store.dir(), dir.join(SIDECAR_DIR));
    assert!(
        !store.dir().exists(),
        "the folder is made on the first write"
    );

    let path = dir.join(SAVED_GAM_FILENAME);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions.clone()).unwrap();
    assert_eq!(SidecarStore::open(&dir), None, "a read-only save");
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions).unwrap();

    assert_eq!(SidecarStore::open(&dir.join("absent")), None);
    assert_eq!(SidecarStore::open(&path), None, "a file is not a folder");
}

#[test]
fn save_meta_round_trips_through_its_container() {
    assert_eq!(SaveMeta::decode(&[]), None);
    assert_eq!(SaveMeta::decode(&[0]), Some(SaveMeta::default()));
    let used = SaveMeta { cheats_used: true };
    assert_eq!(SaveMeta::decode(&used.encode()), Some(used));
    assert_eq!(
        SaveMeta::decode(&[0xff, 9, 9]),
        Some(used),
        "unknown flags and later bytes are skipped"
    );

    let saved = stamp(b"Ava", 139, 7, 41);
    let (_dir, store) = store_with(&image(b"Ava", 139, 7, 41));
    store
        .write(
            TAG_SAVE_META,
            &saved,
            SAVE_META_PAYLOAD_VERSION,
            &used.encode(),
        )
        .unwrap();
    let (freshness, read) = store.read(TAG_SAVE_META, &saved);
    assert_eq!(freshness, Freshness::Fresh);
    assert_eq!(SaveMeta::decode(&read.unwrap().bytes), Some(used));
}
