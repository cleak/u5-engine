//! The `GEMK` attach over the foundation's sidecar store (design 10.3.7,
//! M3-07): each freshness class, a bad checksum, truncation and a future
//! payload version, on real files in a scratch save folder. No game data:
//! the save folder holds a dummy `SAVED.GAM` and synthetic stamps.

use std::fs;
use std::path::{Path, PathBuf};

use u5_automap::gem::{
    AttachOutcome, EmptyReason, GEM_PAYLOAD_VERSION, GEM_SIDECAR_TAG, GemKnowledge, KnowEvent,
    attach, attach_payload, encode_payload,
};
use u5_runtime::enhancements::{
    Freshness, SIDECAR_DIR, SaveStamp, SidecarPayload, SidecarStore, SidecarWrite,
    sidecar_file_name,
};

const NOW: SaveStamp = SaveStamp {
    fnv: 0x1111_2222_3333_4444,
    len: 4192,
    avatar: *b"AVATAR\0\0\0",
    date: [0x8B, 0x01, 4, 12, 9, 30], // year 395
};

fn stamp_with(fnv: u64, avatar: &[u8; 9], date: [u8; 6]) -> SaveStamp {
    SaveStamp {
        fnv,
        avatar: *avatar,
        date,
        ..NOW
    }
}

/// A fresh save folder under Cargo's test temp folder, removed on drop.
struct SaveDir(PathBuf);

impl SaveDir {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("u5-automap-gemk-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SAVED.GAM"), [0u8; 16]).unwrap();
        Self(dir)
    }
    fn store(&self) -> SidecarStore {
        SidecarStore::open(&self.0).expect("a writable save folder")
    }
    fn file(&self) -> PathBuf {
        file_in(&self.0)
    }
}

fn file_in(dir: &Path) -> PathBuf {
    dir.join(SIDECAR_DIR)
        .join(sidecar_file_name(GEM_SIDECAR_TAG).unwrap())
}

impl Drop for SaveDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn small_record() -> GemKnowledge {
    let mut k = GemKnowledge::new();
    k.apply(&KnowEvent::PlaceEntered { scene: 4 });
    k.apply(&KnowEvent::DungeonCellsSeen {
        record: 1,
        level: 0,
        rows: [3; 8],
    });
    k
}

#[test]
fn the_file_is_gemmap_u5e() {
    assert_eq!(sidecar_file_name(GEM_SIDECAR_TAG), Some("gemmap.u5e"));
}

#[test]
fn no_file_attaches_empty() {
    let dir = SaveDir::new("missing");
    let (k, outcome) = attach(&dir.store(), &NOW);
    assert!(k.is_empty());
    assert_eq!(outcome, AttachOutcome::Empty(EmptyReason::Missing));
    assert!(outcome.may_write());
    assert!(!dir.0.join(SIDECAR_DIR).exists(), "attach writes nothing");
}

#[test]
fn a_fresh_record_is_restored() {
    let dir = SaveDir::new("fresh");
    let store = dir.store();
    let k = small_record();
    let written = store.write(
        GEM_SIDECAR_TAG,
        &NOW,
        GEM_PAYLOAD_VERSION,
        &encode_payload(&k),
    );
    assert_eq!(written.unwrap(), SidecarWrite::Written);
    let (back, outcome) = attach(&store, &NOW);
    assert_eq!(back, k);
    assert_eq!(outcome, AttachOutcome::Restored);
    assert!(outcome.may_write() && !outcome.shows_stale_note());
}

#[test]
fn the_same_lineage_is_kept_with_the_note() {
    let dir = SaveDir::new("lineage");
    let store = dir.store();
    let k = small_record();
    let older = stamp_with(1, b"AVATAR\0\0\0", [0x8B, 0x01, 4, 12, 8, 0]);
    store
        .write(
            GEM_SIDECAR_TAG,
            &older,
            GEM_PAYLOAD_VERSION,
            &encode_payload(&k),
        )
        .unwrap();
    let (back, outcome) = attach(&store, &NOW);
    assert_eq!(back, k);
    assert_eq!(outcome, AttachOutcome::RestoredStale);
    assert!(outcome.may_write() && outcome.shows_stale_note());
}

#[test]
fn a_foreign_record_is_ignored_and_left_for_the_next_save() {
    let dir = SaveDir::new("foreign");
    let store = dir.store();
    let other = stamp_with(1, b"SOMEONE\0\0", NOW.date);
    let bytes = encode_payload(&small_record());
    store
        .write(GEM_SIDECAR_TAG, &other, GEM_PAYLOAD_VERSION, &bytes)
        .unwrap();
    let before = fs::read(dir.file()).unwrap();
    let (k, outcome) = attach(&store, &NOW);
    assert!(k.is_empty());
    assert_eq!(outcome, AttachOutcome::Empty(EmptyReason::Foreign));
    assert!(outcome.may_write());
    assert_eq!(fs::read(dir.file()).unwrap(), before, "not touched");
    // An earlier date of the same Avatar is another character too.
    let later = stamp_with(1, &NOW.avatar, [0x8C, 0x01, 1, 1, 0, 0]);
    store
        .write(GEM_SIDECAR_TAG, &later, GEM_PAYLOAD_VERSION, &bytes)
        .unwrap();
    assert_eq!(
        attach(&store, &NOW).1,
        AttachOutcome::Empty(EmptyReason::Foreign)
    );
}

#[test]
fn a_bad_checksum_is_renamed_bad_and_starts_empty() {
    let dir = SaveDir::new("checksum");
    let store = dir.store();
    store
        .write(
            GEM_SIDECAR_TAG,
            &NOW,
            GEM_PAYLOAD_VERSION,
            &encode_payload(&small_record()),
        )
        .unwrap();
    let mut bytes = fs::read(dir.file()).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x5A;
    fs::write(dir.file(), &bytes).unwrap();
    let (k, outcome) = attach(&store, &NOW);
    assert!(k.is_empty());
    assert_eq!(outcome, AttachOutcome::Empty(EmptyReason::Corrupt));
    assert!(!dir.file().exists());
    let mut bad = dir.file().into_os_string();
    bad.push(".bad");
    assert_eq!(fs::read(PathBuf::from(bad)).unwrap(), bytes);
    // The next save writes a new file.
    assert_eq!(
        store
            .write(GEM_SIDECAR_TAG, &NOW, GEM_PAYLOAD_VERSION, &[0, 0])
            .unwrap(),
        SidecarWrite::Written
    );
}

#[test]
fn a_truncated_file_is_corrupt() {
    let dir = SaveDir::new("truncated");
    let store = dir.store();
    store
        .write(
            GEM_SIDECAR_TAG,
            &NOW,
            GEM_PAYLOAD_VERSION,
            &encode_payload(&small_record()),
        )
        .unwrap();
    let bytes = fs::read(dir.file()).unwrap();
    fs::write(dir.file(), &bytes[..bytes.len() - 9]).unwrap();
    assert_eq!(
        attach(&store, &NOW).1,
        AttachOutcome::Empty(EmptyReason::Corrupt)
    );
}

#[test]
fn a_damaged_payload_in_a_sound_container_starts_empty() {
    let dir = SaveDir::new("damaged");
    let store = dir.store();
    store
        .write(GEM_SIDECAR_TAG, &NOW, GEM_PAYLOAD_VERSION, &[1, 0, 9])
        .unwrap();
    let (k, outcome) = attach(&store, &NOW);
    assert!(k.is_empty());
    assert_eq!(outcome, AttachOutcome::Empty(EmptyReason::Damaged));
    assert!(outcome.may_write());
}

#[test]
fn a_future_version_is_read_only() {
    let dir = SaveDir::new("future");
    let store = dir.store();
    let k = small_record();
    let mut bytes = encode_payload(&k);
    bytes.extend_from_slice(&[0xEE; 3]);
    store
        .write(GEM_SIDECAR_TAG, &NOW, GEM_PAYLOAD_VERSION + 1, &bytes)
        .unwrap();
    let before = fs::read(dir.file()).unwrap();
    let (back, outcome) = attach(&store, &NOW);
    assert_eq!(back, k, "used in RAM");
    assert_eq!(outcome, AttachOutcome::ReadOnly { stale: false });
    assert!(!outcome.may_write());
    // Even a write that ignored the outcome would be refused by the store.
    assert_eq!(
        store
            .write(GEM_SIDECAR_TAG, &NOW, GEM_PAYLOAD_VERSION, &[0, 0])
            .unwrap(),
        SidecarWrite::KeptNewer
    );
    assert_eq!(fs::read(dir.file()).unwrap(), before);
    // From the same lineage it is read-only and stale.
    let (_, outcome) = attach_payload(
        Freshness::SameLineage,
        Some(SidecarPayload {
            version: GEM_PAYLOAD_VERSION + 1,
            bytes,
        }),
    );
    assert_eq!(outcome, AttachOutcome::ReadOnly { stale: true });
    assert!(outcome.shows_stale_note());
}
