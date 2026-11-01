// Closed-audit follow-up, group 4: normative rules from
// `systems/blackthorn.md`, `catalogs/tile-catalog.md`, `systems/encounters.md`,
// `systems/display-driver.md`, `catalogs/spell-list.md`,
// `systems/save-load.md`, `formats/ool.md`,
// `catalogs/common-word-dictionary.md` and `formats/end-dat.md` that the
// engine implements but no earlier test asserted directly.
//
// Every test names its spec file and section. Asset-backed tests follow the
// `U5_TEST_ASSET_DIR` pattern and only ever read the configured directory.
// No unpublished game text appears here: literals are the ones the spec
// prints, otherwise invented placeholders.

// ---------------------------------------------------------------------------
// catalogs/common-word-dictionary.md
// ---------------------------------------------------------------------------

/// The catalog is 1-based; the engine table is 0-based (catalog index - 1).
fn g4_word(catalog_index: usize) -> &'static str {
    PUBLISHED_COMMON_WORD_DICTIONARY[catalog_index - 1]
}

/// `common-word-dictionary.md §6` invariants 1-3 and `§3`: 128 addressable
/// indices, exactly 118 populated, and exactly ten empty slots at the listed
/// indices, in that order and no others.
#[test]
fn g4_common_words_s6_census_is_118_populated_and_ten_listed_empty_slots() {
    assert_eq!(PUBLISHED_COMMON_WORD_DICTIONARY.len(), 128);
    let populated = PUBLISHED_COMMON_WORD_DICTIONARY
        .iter()
        .filter(|word| !word.is_empty())
        .count();
    assert_eq!(populated, 118);
    let empty: Vec<usize> = (1..=128).filter(|&index| g4_word(index).is_empty()).collect();
    assert_eq!(empty, vec![8, 28, 50, 65, 67, 70, 72, 73, 74, 75]);
    assert_eq!(
        empty,
        vec![0x08, 0x1C, 0x32, 0x41, 0x43, 0x46, 0x48, 0x49, 0x4A, 0x4B]
    );
}

/// `common-word-dictionary.md §6` invariant 5: every populated entry is two
/// to twelve ASCII characters with no whitespace and no bit-seven byte; the
/// longest is `Blackthorn's` (12), then `Shadowlords` (11) and `Blackthorn`
/// (10); nineteen entries are two letters long.
#[test]
fn g4_common_words_s6_word_content_lengths_and_character_set() {
    let mut lengths = Vec::new();
    for (slot, word) in PUBLISHED_COMMON_WORD_DICTIONARY.iter().enumerate() {
        if word.is_empty() {
            continue;
        }
        assert!(
            (2..=12).contains(&word.len()),
            "catalog index {} has length {}",
            slot + 1,
            word.len()
        );
        assert!(word.bytes().all(|byte| byte < 0x80 && !byte.is_ascii_whitespace()));
        lengths.push((word.len(), *word));
    }
    lengths.sort_by(|a, b| b.0.cmp(&a.0));
    assert_eq!(lengths[0], (12, "Blackthorn's"));
    assert_eq!(lengths[1], (11, "Shadowlords"));
    assert_eq!(lengths[2], (10, "Blackthorn"));
    assert_eq!(lengths.iter().filter(|(len, _)| *len == 2).count(), 19);
}

/// `common-word-dictionary.md §4`: punctuation-bearing entries carry the
/// punctuation in the word itself - index `0x39` is `thee,` and index `0x2F`
/// is `Blackthorn's`.
#[test]
fn g4_common_words_s4_punctuation_bearing_entries_sit_at_their_indices() {
    assert_eq!(g4_word(0x39), "thee,");
    assert_eq!(g4_word(0x2F), "Blackthorn's");
}

/// `common-word-dictionary.md §2` and `§6` invariant 6: dialogue token `0x01`
/// and shop token `0x80` reach the same entry (`the`); dialogue token `0x80`
/// and shop token `0xFF` reach the same entry (`work`).
#[test]
fn g4_common_words_s6_bias_check_reaches_first_and_last_entries_from_both_consumers() {
    let dialogue_first = tlk_dictionary_index(0x01).unwrap();
    let shop_first = shoppe_dictionary_index(0x80).unwrap();
    assert_eq!(dialogue_first, shop_first);
    assert_eq!(PUBLISHED_COMMON_WORD_DICTIONARY[dialogue_first], "the");
    let dialogue_last = tlk_dictionary_index(0x80).unwrap();
    let shop_last = shoppe_dictionary_index(0xFF).unwrap();
    assert_eq!(dialogue_last, shop_last);
    assert_eq!(PUBLISHED_COMMON_WORD_DICTIONARY[dialogue_last], "work");
}

/// `common-word-dictionary.md §2`: the highest dialogue token `0x80` has bit
/// seven set yet still takes the dictionary path, and `§4`: conversation
/// emits a space before every expansion. With the published table the token
/// renders as ` work`.
#[test]
fn g4_common_words_s2_dialogue_token_0x80_expands_through_the_published_table() {
    let out = crate::tlk_runner::run_tlk_stream(
        &[0x80, TLK_CODE_END_OF_RESPONSE],
        &crate::tlk_runner::TlkRunInputs {
            dictionary: Some(&PUBLISHED_COMMON_WORD_DICTIONARY),
            ..Default::default()
        },
    );
    assert_eq!(out.text, " work");
}

/// `common-word-dictionary.md §2` and `§4`: shop token `0xFF` is catalog index
/// 128 (`work`); the shop renderer emits one space before the word and, at
/// the end of a record, no trailing space.
#[test]
fn g4_common_words_s4_shop_token_0xff_expands_with_leading_space_only_at_record_end() {
    let rendered = crate::shoppe_bark::render_shoppe_bark(
        &[b'Q', 0xFF, 0x00],
        &crate::shoppe_bark::ShoppeBarkContext::default(),
    )
    .unwrap();
    assert_eq!(rendered, "Q work");
}

// ---------------------------------------------------------------------------
// formats/end-dat.md
// ---------------------------------------------------------------------------

/// `end-dat.md §5`: a reader "should require the file to be valid low-ASCII
/// text"; a byte with bit seven set is rejected, while trailing NUL padding is
/// tolerated.
#[test]
fn g4_end_dat_s5_reader_rejects_high_bit_bytes_and_tolerates_nul_padding() {
    assert!(parse_end_narrative(b"Placeholder text\0\0").is_ok());
    let err = parse_end_narrative(b"Placeholder \x80text\0").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

/// `end-dat.md §5`: "If the endgame requests a seek window outside the asset
/// ... the runtime should report a missing endgame-text asset error rather
/// than continuing with a blank cinematic page." A window whose published
/// range runs past a short file yields no page.
#[test]
fn g4_end_dat_s5_window_outside_the_asset_yields_no_page() {
    let narrative = EndNarrative::new(b"Short placeholder record\0".to_vec());
    assert_eq!(narrative.window_by_number(6), None);
    assert_eq!(narrative.layout_window_by_number(6), None);
}

/// `end-dat.md §5`: a missing `END.DAT` is reported as a missing endgame-text
/// asset error, not a blank page.
#[test]
fn g4_end_dat_s5_missing_file_is_reported_as_an_error() {
    let dir = debug_game_dir();
    let err = require_end_narrative(&dir).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert_eq!(load_end_narrative(&dir).unwrap(), None);
    let _ = fs::remove_dir_all(dir);
}

/// `end-dat.md §2`, `§3` and `§4` against the shipped file: 3,698 bytes; six
/// NUL-terminated records plus two NUL bytes of tail padding; eleven `{`,
/// 139 `_`, seven line feeds and no `}`; per-record brace counts 1,1,1,2,3,3;
/// only records 3 and 5 open with a line feed; and walking the NUL boundaries
/// (segments longer than ten bytes) gives exactly the published windows.
/// Skipped when `U5_TEST_ASSET_DIR` is unset.
#[test]
fn g4_end_dat_s2_s4_shipped_file_census_matches_the_published_layout() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Ok(bytes) = fs::read(dir.join(END_DAT_FILE)) else {
        return;
    };
    assert_eq!(bytes.len(), 3_698);
    assert_eq!(bytes.iter().filter(|&&b| b == b'{').count(), 11);
    assert_eq!(bytes.iter().filter(|&&b| b == b'_').count(), 139);
    assert_eq!(bytes.iter().filter(|&&b| b == b'\n').count(), 7);
    assert!(!bytes.contains(&b'}'));
    assert!(bytes.iter().all(|&b| b < 0x80));
    assert_eq!(&bytes[bytes.len() - 2..], &[0, 0]);

    let mut walked = Vec::new();
    let mut start = 0;
    for (offset, &byte) in bytes.iter().enumerate() {
        if byte == 0 {
            if offset - start > 10 {
                walked.push((start, offset));
            }
            start = offset + 1;
        }
    }
    let published: Vec<(usize, usize)> = END_DAT_PUBLISHED_WINDOW_RANGES
        .iter()
        .map(|range| (range.start, range.end))
        .collect();
    assert_eq!(walked, published);

    let braces: Vec<usize> = published
        .iter()
        .map(|&(s, e)| bytes[s..e].iter().filter(|&&b| b == b'{').count())
        .collect();
    assert_eq!(braces, vec![1, 1, 1, 2, 3, 3]);
    let leading_feed: Vec<bool> = published.iter().map(|&(s, _)| bytes[s] == b'\n').collect();
    assert_eq!(leading_feed, vec![false, false, true, false, true, false]);
    let longest = published.iter().map(|&(s, e)| e - s).max().unwrap();
    assert_eq!(longest, 764);
}

// ---------------------------------------------------------------------------
// formats/ool.md and systems/save-load.md
// ---------------------------------------------------------------------------

/// `ool.md §3`, `§8` and `§9`: a plane table is exactly 256 bytes of 32
/// eight-byte records, slot order is stable, an empty slot's non-zero
/// remaining bytes are preserved, auxiliary bytes `+5`, `+6` and `+7` are
/// preserved verbatim and the `0xFF` Z sentinel survives as a byte.
#[test]
fn g4_ool_s8_plane_table_round_trip_preserves_every_byte_and_slot_order() {
    let mut plane = vec![0u8; OOL_PLANE_LEN];
    // An "empty" slot (zero type byte) whose remaining bytes are not zero.
    plane[3 * 8..3 * 8 + 8].copy_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
    // A ship-like record: hull `+5`, packed animator byte `+6`, skiffs `+7`,
    // and the above-ground Z sentinel.
    plane[23 * 8..23 * 8 + 8].copy_from_slice(&[0x24, 0x25, 14, 242, 0xFF, 97, 0x3A, 2]);
    plane[31 * 8..31 * 8 + 8].copy_from_slice(&[0x1E, 0x1F, 103, 226, 0xFF, 0, 0x0F, 0]);

    let objects = decode_full_active_object_table(&plane, "test plane").unwrap();
    assert_eq!(objects.len(), 32);
    assert_eq!(objects[23].x, 14);
    assert_eq!(objects[23].aux1, 97);
    assert_eq!(objects[23].phase, 0x3A);
    assert_eq!(objects[23].aux3, 2);
    assert_eq!(objects[31].type_byte, 0x1E);
    assert_eq!(encode_active_object_table(&objects).unwrap(), plane);

    assert!(decode_full_active_object_table(&plane[..255], "short").is_err());
    let mut long = plane.clone();
    long.push(0);
    assert!(decode_full_active_object_table(&long, "long").is_err());
}

/// `ool.md §8`: `SAVED.OOL` is exactly 512 bytes; a 256-byte file is
/// rejected rather than read as one half.
#[test]
fn g4_ool_s8_saved_ool_must_be_512_bytes() {
    let dir = debug_game_dir();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0u8; OOL_PLANE_LEN]).unwrap();
    let err = read_saved_ool_bytes(&dir).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert!(write_saved_ool_mirrors(&dir, &vec![0u8; SAVED_OOL_LEN - 1]).is_err());
    let _ = fs::remove_dir_all(dir);
}

/// `ool.md §8` and `§9`: `BRIT.OOL` and `UNDER.OOL` are exactly 256 bytes
/// each; the save path's per-plane reads reject a wrong-size mirror.
#[test]
fn g4_ool_s9_save_staging_rejects_a_wrong_size_per_plane_file() {
    let dir = debug_game_dir();
    fs::write(dir.join(BRIT_OOL_FILENAME), vec![0u8; OOL_PLANE_LEN - 1]).unwrap();
    fs::write(dir.join(UNDER_OOL_FILENAME), vec![0u8; OOL_PLANE_LEN]).unwrap();
    let err = stage_saved_ool_for_save(&dir, RequiredDisk::Program).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    let _ = fs::remove_dir_all(dir);
}

/// `ool.md §6` load steps 2-4 and `save-load.md §4.2` steps 3-5: load reads
/// all of `SAVED.OOL` and writes the first half to `BRIT.OOL` and the second
/// to `UNDER.OOL`, in that fixed order, never rotated; the underworld
/// disk-swap branch rewrites `UNDER.OOL` once more.
#[test]
fn g4_ool_s6_load_refresh_mirrors_surface_first_then_underworld() {
    let dir = debug_game_dir();
    let mut saved = vec![0u8; SAVED_OOL_LEN];
    for (index, byte) in saved.iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    fs::write(dir.join(SAVED_OOL_FILENAME), &saved).unwrap();
    let counts = refresh_saved_ool_mirrors_for_load(&dir, false).unwrap();
    assert_eq!(counts.brit_ool, 1);
    assert_eq!(counts.under_ool, 1);
    assert_eq!(fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(), saved[..OOL_PLANE_LEN]);
    assert_eq!(fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(), saved[OOL_PLANE_LEN..]);
    let swap = refresh_saved_ool_mirrors_for_load(&dir, true).unwrap();
    assert_eq!(swap.under_ool, 2);
    assert_eq!(fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(), saved[OOL_PLANE_LEN..]);
    let _ = fs::remove_dir_all(dir);
}

/// `ool.md §1`, `§2` and `§7` against the shipped files: `BRIT.OOL` is 256
/// zero bytes; `UNDER.OOL` holds exactly the five published seed records in
/// slots 23-27 with every other byte zero; `INIT.OOL` is byte-identical to
/// `UNDER.OOL`; and the clean-install `SAVED.OOL` is an empty surface table
/// followed by the underworld seed. Skipped when `U5_TEST_ASSET_DIR` is unset.
#[test]
fn g4_ool_s7_shipped_seed_tables_match_the_published_records() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let read = |name: &str| fs::read(dir.join(name)).ok();
    let (Some(brit), Some(under), Some(init)) = (
        read(BRIT_OOL_FILENAME),
        read(UNDER_OOL_FILENAME),
        read(INIT_OOL_FILENAME),
    ) else {
        return;
    };
    assert_eq!(brit, vec![0u8; OOL_PLANE_LEN]);
    assert_eq!(init, under);
    let mut expected = vec![0u8; OOL_PLANE_LEN];
    for (slot, record) in [
        (23usize, [0x29u8, 0x29, 14, 242, 0xFF, 0, 0, 0]),
        (24, [0x1E, 0x1E, 103, 226, 0xFF, 0, 0, 0]),
        (25, [0x1E, 0x1E, 105, 227, 0xFF, 0, 0, 0]),
        (26, [0x1E, 0x1E, 107, 227, 0xFF, 0, 0, 0]),
        (27, [0x1E, 0x1E, 108, 225, 0xFF, 0, 0, 0]),
    ] {
        expected[slot * 8..slot * 8 + 8].copy_from_slice(&record);
    }
    assert_eq!(under, expected);
    if let Some(saved) = read(SAVED_OOL_FILENAME) {
        assert_eq!(saved.len(), SAVED_OOL_LEN);
        assert_eq!(saved[..OOL_PLANE_LEN], brit[..]);
        assert_eq!(saved[OOL_PLANE_LEN..], under[..]);
    }
}

fn g4_save_dir() -> std::path::PathBuf {
    let dir = debug_game_dir();
    let mut template = saved_game_seed_bytes(0, 0, 10, 20);
    template[SAVE_AVATAR_NAME_OFFSET] = b'Q';
    fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0u8; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    dir
}

/// `save-load.md §5.2` step 1: "On `N`, it prints `No`, returns to gameplay,
/// and the player remains in the same scene with no disk activity."
#[test]
fn g4_save_load_s5_2_declining_the_save_prints_no_and_touches_no_file() {
    let dir = g4_save_dir();
    let before: Vec<Vec<u8>> = [SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, BRIT_OOL_FILENAME, UNDER_OOL_FILENAME]
        .iter()
        .map(|name| fs::read(dir.join(name)).unwrap())
        .collect();
    let mut state = world_state(open_world_grid(), 10, 20);
    state.gold = 777;
    assert_eq!(
        state.save_game_command(&dir, Some(false)).unwrap(),
        MoveOutcome::PromptDeclined
    );
    assert!(state.message.ends_with(SAVE_PROMPT_NO_REPLY), "{:?}", state.message);
    assert!(!state.message.contains(SAVE_IN_PROGRESS_MESSAGE));
    let after: Vec<Vec<u8>> = [SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, BRIT_OOL_FILENAME, UNDER_OOL_FILENAME]
        .iter()
        .map(|name| fs::read(dir.join(name)).unwrap())
        .collect();
    assert_eq!(before, after);
    let _ = fs::remove_dir_all(dir);
}

/// `save-load.md §5.3`: "The save flow does not write `INIT.GAM` or
/// `INIT.OOL` - those are shipped read-only." `§2.3`: the engine never
/// touches `INIT.GAM` again after chargen.
#[test]
fn g4_save_load_s5_3_a_save_never_writes_the_factory_seeds() {
    let dir = g4_save_dir();
    let init_gam: Vec<u8> = (0..SAVED_GAM_LEN).map(|i| (i % 7) as u8).collect();
    let init_ool: Vec<u8> = (0..OOL_PLANE_LEN).map(|i| (i % 5) as u8 + 1).collect();
    fs::write(dir.join(INIT_GAM_FILENAME), &init_gam).unwrap();
    fs::write(dir.join(INIT_OOL_FILENAME), &init_ool).unwrap();
    let mut state = world_state(open_world_grid(), 10, 20);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    assert_eq!(fs::read(dir.join(INIT_GAM_FILENAME)).unwrap(), init_gam);
    assert_eq!(fs::read(dir.join(INIT_OOL_FILENAME)).unwrap(), init_ool);
    let _ = fs::remove_dir_all(dir);
}

/// `save-load.md §5.2` steps 4-7 and `ool.md §8`: after a confirmed save,
/// `SAVED.OOL` is the 512-byte concatenation of `BRIT.OOL` and `UNDER.OOL` as
/// they stood when the save began (surface first), and `BRIT.OOL` is never
/// written; `UNDER.OOL` is rewritten with its own bytes.
#[test]
fn g4_save_load_s5_2_saved_ool_is_the_per_plane_files_concatenated_surface_first() {
    let dir = g4_save_dir();
    let brit: Vec<u8> = (0..OOL_PLANE_LEN).map(|i| (i % 13) as u8).collect();
    let under: Vec<u8> = (0..OOL_PLANE_LEN).map(|i| 0x80 | (i % 11) as u8).collect();
    fs::write(dir.join(BRIT_OOL_FILENAME), &brit).unwrap();
    fs::write(dir.join(UNDER_OOL_FILENAME), &under).unwrap();
    let mut state = world_state(open_world_grid(), 10, 20);
    state.save_game_command(&dir, Some(true)).unwrap();
    let saved = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
    assert_eq!(saved.len(), 512);
    assert_eq!(saved[..OOL_PLANE_LEN], brit[..]);
    assert_eq!(saved[OOL_PLANE_LEN..], under[..]);
    assert_eq!(fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(), brit);
    assert_eq!(fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(), under);
    assert_eq!(fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap().len(), 4192);
    let _ = fs::remove_dir_all(dir);
}

/// `save-load.md §5.2` steps 2 and 8: on `Y` the handler prints `Yes`, then
/// `Saving...`, then `Done.`, in that order.
#[test]
fn g4_save_load_s5_2_confirmed_save_prints_yes_saving_done_in_order() {
    let dir = g4_save_dir();
    let mut state = world_state(open_world_grid(), 10, 20);
    state.save_game_command(&dir, Some(true)).unwrap();
    state.flush_message_slot();
    let rows: Vec<String> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .filter(|text| !text.trim().is_empty())
        .collect();
    let position = |needle: &str| rows.iter().rposition(|row| row.contains(needle));
    let yes = position(SAVE_PROMPT_YES_REPLY).expect("Yes");
    let saving = position(SAVE_IN_PROGRESS_MESSAGE).expect("Saving...");
    let done = position(SAVE_DONE_MESSAGE).expect("Done.");
    assert!(yes < saving && saving < done, "{rows:?}");
    assert_eq!(SAVE_IN_PROGRESS_MESSAGE, "Saving...");
    assert_eq!(SAVE_DONE_MESSAGE, "Done.");
    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// catalogs/spell-list.md
// ---------------------------------------------------------------------------

/// `spell-list.md §3` reagent bit order.
fn g4_reagent_bit(name: &str) -> u8 {
    match name {
        "Sulfur Ash" => 0x80,
        "Ginseng" => 0x40,
        "Garlic" => 0x20,
        "Spider Silk" => 0x10,
        "Blood Moss" => 0x08,
        "Black Pearl" => 0x04,
        "Nightshade" => 0x02,
        "Mandrake" => 0x01,
        other => panic!("unknown reagent {other}"),
    }
}

/// `spell-list.md §4` scene-mask legend (corrected: `0x01` combat, `0x02`
/// dungeon).
fn g4_allowed_mask(labels: &str) -> u8 {
    labels
        .split('/')
        .map(|label| match label {
            "C" => 0x01,
            "D" => 0x02,
            "I" => 0x04,
            "O" => 0x08,
            other => panic!("unknown scene label {other}"),
        })
        .fold(0, |mask, bit| mask | bit)
}

/// `spell-list.md §5` spell table: (code, rune name, common name, circle,
/// recipe, allowed).
const G4_SPELL_TABLE: [(&str, &str, &str, u8, &[&str], &str); 48] = [
    ("IL", "In Lor", "Light", 1, &["Sulfur Ash"], "D/I/O"),
    ("GP", "Grav Por", "Magic Missile", 1, &["Sulfur Ash", "Black Pearl"], "C"),
    ("AZ", "An Zu", "Awaken", 1, &["Ginseng", "Garlic"], "C/D/I/O"),
    ("AN", "An Nox", "Cure", 1, &["Ginseng", "Garlic"], "C/D/I/O"),
    ("M", "Mani", "Heal", 1, &["Ginseng", "Spider Silk"], "C/D/I/O"),
    ("AY", "An Ylem", "Vanish", 1, &["Garlic", "Blood Moss"], "C/I"),
    ("AS", "An Sanct", "Open", 2, &["Sulfur Ash", "Blood Moss"], "C/D/I/O"),
    ("ACX", "An Xen Corp", "Repel Undead", 2, &["Sulfur Ash", "Garlic"], "C"),
    ("HR", "Rel Hur", "Wind Change", 2, &["Sulfur Ash", "Blood Moss"], "O"),
    ("IW", "In Wis", "Locate", 2, &["Nightshade"], "O"),
    ("KX", "Kal Xen", "Conjure", 2, &["Spider Silk", "Mandrake"], "C"),
    ("IMX", "In Xen Mani", "Create Food", 2, &["Ginseng", "Garlic", "Mandrake"], "C/D/I/O"),
    ("LV", "Vas Lor", "Great Light", 3, &["Sulfur Ash", "Mandrake"], "D/I/O"),
    ("FV", "Vas Flam", "Fireball", 3, &["Sulfur Ash", "Black Pearl"], "C"),
    ("FGI", "In Flam Grav", "Fire Field", 3, &["Sulfur Ash", "Spider Silk", "Black Pearl"], "C/D"),
    ("GIN", "In Nox Grav", "Poison Field", 3, &["Spider Silk", "Black Pearl", "Nightshade"], "C/D"),
    ("GIZ", "In Zu Grav", "Sleep Field", 3, &["Ginseng", "Spider Silk", "Black Pearl"], "C/D"),
    ("IP", "In Por", "Blink", 3, &["Spider Silk", "Blood Moss"], "C/O"),
    ("AG", "An Grav", "Dispel Field", 4, &["Sulfur Ash", "Black Pearl"], "C/D"),
    ("IS", "In Sanct", "Protection", 4, &["Sulfur Ash", "Ginseng", "Garlic"], "C/D/I/O"),
    ("GIS", "In Sanct Grav", "Energy Field", 4, &["Spider Silk", "Black Pearl", "Mandrake"], "C/D"),
    ("PU", "Uus Por", "Up", 4, &["Spider Silk", "Blood Moss"], "D"),
    ("DP", "Des Por", "Down", 4, &["Spider Silk", "Blood Moss"], "D"),
    ("QW", "Wis Quas", "Reveal", 4, &["Spider Silk", "Nightshade"], "C"),
    ("BIX", "In Bet Xen", "Swarm", 5, &["Sulfur Ash", "Spider Silk", "Blood Moss"], "C"),
    ("AEP", "An Ex Por", "Magic Lock", 5, &["Sulfur Ash", "Garlic", "Blood Moss"], "C/I"),
    ("EIP", "In Ex Por", "Unlock Magic", 5, &["Sulfur Ash", "Blood Moss"], "C/I"),
    ("MV", "Vas Mani", "Great Heal", 5, &["Ginseng", "Spider Silk", "Mandrake"], "C/D/I/O"),
    ("IZ", "In Zu", "Sleep", 5, &["Ginseng", "Spider Silk", "Nightshade"], "C"),
    ("RT", "Rel Tym", "Quickness", 5, &["Sulfur Ash", "Blood Moss", "Mandrake"], "C/D/I/O"),
    ("IPVY", "In Vas Por Ylem", "Tremor", 6, &["Sulfur Ash", "Blood Moss", "Mandrake"], "C"),
    ("AQW", "Quas An Wis", "Mass Charm", 6, &["Nightshade", "Mandrake"], "C"),
    ("AI", "In An", "Negate Magic", 6, &["Sulfur Ash", "Garlic", "Mandrake"], "C/D/I/O"),
    ("AWY", "Wis An Ylem", "X-Ray", 6, &["Sulfur Ash", "Mandrake"], "I/O"),
    ("AEX", "An Xen Ex", "Charm", 6, &["Spider Silk", "Black Pearl", "Nightshade"], "C"),
    ("BRX", "Rel Xen Bet", "Polymorph", 6, &["Sulfur Ash", "Spider Silk", "Nightshade", "Mandrake"], "C"),
    ("LS", "Sanct Lor", "Invisibility", 7, &["Blood Moss", "Nightshade", "Mandrake"], "C"),
    ("CX", "Xen Corp", "Kill", 7, &["Black Pearl", "Nightshade"], "C"),
    ("IQX", "In Quas Xen", "Clone", 7, &["Sulfur Ash", "Ginseng", "Spider Silk", "Blood Moss", "Mandrake"], "C"),
    ("IQW", "In Quas Wis", "Peer", 7, &["Nightshade", "Mandrake"], "D/I/O"),
    ("HIN", "In Nox Hur", "Poison Wind", 7, &["Sulfur Ash", "Blood Moss", "Nightshade"], "C"),
    ("CIQ", "In Quas Corp", "Cause Fear", 7, &["Garlic", "Nightshade", "Mandrake"], "C"),
    ("CIM", "In Mani Corp", "Resurrect", 8, &["Sulfur Ash", "Ginseng", "Garlic", "Spider Silk", "Blood Moss", "Mandrake"], "D/I/O"),
    ("CKX", "Kal Xen Corp", "Summon", 8, &["Garlic", "Spider Silk", "Blood Moss", "Mandrake"], "C"),
    ("CGIV", "In Vas Grav Corp", "Death Wind", 8, &["Sulfur Ash", "Nightshade", "Mandrake"], "C"),
    ("FHI", "In Flam Hur", "Flame Wind", 8, &["Sulfur Ash", "Blood Moss", "Mandrake"], "C"),
    ("PRV", "Vas Rel Por", "Gate Travel", 8, &["Sulfur Ash", "Black Pearl", "Mandrake"], "D/I/O"),
    ("AT", "An Tym", "Negate Time", 8, &["Garlic", "Blood Moss", "Mandrake"], "C/D/I/O"),
];

/// `spell-list.md §3` and `§5`: every spell's resident recipe byte is the OR
/// of its listed reagents' bits in the published bit order.
#[test]
fn g4_spell_list_s5_recipe_masks_match_every_published_row() {
    for (id, (code, _, _, _, recipe, _)) in G4_SPELL_TABLE.iter().enumerate() {
        let mask = recipe.iter().fold(0u8, |mask, reagent| mask | g4_reagent_bit(reagent));
        assert_eq!(SPELL_RECIPE_MASKS[id], mask, "spell {id} {code}");
    }
}

/// `spell-list.md §4` and `§5`: every spell's scene allow mask carries the
/// `Allowed` column under the corrected legend.
#[test]
fn g4_spell_list_s5_scene_masks_match_every_published_allowed_column() {
    for (id, (code, _, _, _, _, allowed)) in G4_SPELL_TABLE.iter().enumerate() {
        assert_eq!(SPELL_SCENE_MASKS[id], g4_allowed_mask(allowed), "spell {id} {code}");
    }
}

/// `spell-list.md §5` and `§6`: the table is ordered by engine spell id; the
/// compact token, rune name and common label of every row, and `§1`'s
/// `circle = id / 6 + 1`.
#[test]
fn g4_spell_list_s5_codes_rune_names_labels_and_circles_match_every_row() {
    for (id, (code, rune, common, circle, _, _)) in G4_SPELL_TABLE.iter().enumerate() {
        assert_eq!(SPELL_CODES[id], *code, "code {id}");
        assert_eq!(spell_rune_name(id), Some(*rune), "rune {id}");
        assert_eq!(spell_common_name(id), Some(*common), "label {id}");
        assert_eq!(spell_circle_for(id as u8), Some(*circle), "circle {id}");
        assert_eq!(*circle, (id / 6 + 1) as u8);
    }
}

/// `spell-list.md §1`: "for every id, sort the letters of the parser token and
/// compare them with the sorted initials of the phrase's words. It holds
/// forty-eight times out of forty-eight", and `§2`: every incantation word is
/// one of the twenty-four resident syllables.
#[test]
fn g4_spell_list_s1_parser_tokens_are_the_sorted_initials_of_their_phrases() {
    for id in 0..SPELL_COUNT {
        let phrase = spell_rune_name(id).unwrap();
        let mut initials: Vec<char> = phrase
            .split_whitespace()
            .map(|word| word.chars().next().unwrap())
            .collect();
        initials.sort_unstable();
        let mut token: Vec<char> = SPELL_CODES[id].chars().collect();
        token.sort_unstable();
        assert_eq!(token, initials, "spell {id} {phrase}");
        for word in phrase.split_whitespace() {
            assert!(is_resident_rune_syllable(word), "{word} in spell {id}");
        }
    }
}

/// `spell-list.md §3`: "Recipes are not unique. Several distinct spells share
/// an identical reagent set" - Awaken and Cure share Ginseng + Garlic; Open,
/// Wind Change and Unlock Magic share Sulfur Ash + Blood Moss.
#[test]
fn g4_spell_list_s3_recipes_are_not_unique() {
    assert_eq!(SPELL_RECIPE_MASKS[2], SPELL_RECIPE_MASKS[3]);
    assert_eq!(SPELL_RECIPE_MASKS[6], SPELL_RECIPE_MASKS[8]);
    assert_eq!(SPELL_RECIPE_MASKS[6], SPELL_RECIPE_MASKS[26]);
    let distinct: std::collections::HashSet<u8> = SPELL_RECIPE_MASKS.iter().copied().collect();
    assert!(distinct.len() < SPELL_COUNT);
}

/// `spell-list.md §4`: scene byte `0` is overworld, `1..32` indoor, `33..127`
/// dungeon and `0xFF` combat-class, each selecting exactly one bit.
#[test]
fn g4_spell_list_s4_scene_byte_bands_use_the_published_literals() {
    for (byte, class, bit) in [
        (0u8, SpellSceneClass::Overworld, 0x08u8),
        (1, SpellSceneClass::Indoor, 0x04),
        (32, SpellSceneClass::Indoor, 0x04),
        (33, SpellSceneClass::Dungeon, 0x02),
        (127, SpellSceneClass::Dungeon, 0x02),
        (0xFF, SpellSceneClass::Combat, 0x01),
    ] {
        assert_eq!(spell_scene_class_for_scene_byte(byte), class, "scene {byte}");
        assert_eq!(class.allow_mask_bit(), bit);
    }
}

/// `spell-list.md §6`: `IS`/`RT`/`AQW`/`AI` install the cast-time tag and
/// counter pairs `P`/20, `Q`/30, `C`/20 and `N`/10, and `AT` installs `T`
/// with a 10-count countdown.
#[test]
fn g4_spell_list_s6_timed_effect_tags_and_cast_counters() {
    for (tag, letter, counter) in [
        (ActiveEffectTag::Protection, b'P', 20u8),
        (ActiveEffectTag::Quickness, b'Q', 30),
        (ActiveEffectTag::MassCharm, b'C', 20),
        (ActiveEffectTag::NegateMagic, b'N', 10),
        (ActiveEffectTag::NegateTime, b'T', 10),
    ] {
        assert_eq!(tag.ascii_byte(), letter);
        assert_eq!(tag.spell_install_counter(), Some(counter), "{tag:?}");
    }
}

// ---------------------------------------------------------------------------
// systems/display-driver.md
// ---------------------------------------------------------------------------

/// `display-driver.md §2`: the seven-slot UI colour table for the high-colour
/// family (EGA, Tandy) and the low-colour family (CGA, Hercules); every
/// low-colour value lies inside `0..3`.
#[test]
fn g4_display_driver_s2_ui_colour_table_values_for_both_families() {
    let high = [4u8, 15, 1, 2, 5, 14, 7];
    let low = [2u8, 3, 1, 1, 2, 3, 3];
    assert_eq!(UI_COLOUR_TABLE_SLOTS, 7);
    for slot in 0..7 {
        assert_eq!(ui_colour_slot(slot, true), high[slot], "high slot {slot}");
        assert_eq!(ui_colour_slot(slot, false), low[slot], "low slot {slot}");
        assert!(ui_colour_slot(slot, false) <= 3);
    }
}

/// `display-driver.md §2`: the dungeon view's band pens and other consumers
/// take the same slots "biased into the bright half of the palette by adding
/// eight".
#[test]
fn g4_display_driver_s2_bright_bias_adds_eight() {
    for slot in 0..7 {
        assert_eq!(ui_colour_slot_bright(slot, true), ui_colour_slot(slot, true) + 8);
    }
}

/// `display-driver.md §7` primitive requirements: "Lines include both
/// endpoints and cover all eight octants".
#[test]
fn g4_display_driver_s7_lines_include_both_endpoints_in_all_octants() {
    let centre = (160i32, 100i32);
    for (dx, dy) in [
        (30, 7),
        (7, 30),
        (-7, 30),
        (-30, 7),
        (-30, -7),
        (-7, -30),
        (7, -30),
        (30, -7),
    ] {
        let mut surface = EgaDisplaySurface::new();
        surface.set_current_color(9);
        let end = (centre.0 + dx, centre.1 + dy);
        surface.draw_line(centre.0, centre.1, end.0, end.1);
        assert_eq!(surface.read_pixel(centre.0 as usize, centre.1 as usize), Some(9));
        assert_eq!(surface.read_pixel(end.0 as usize, end.1 as usize), Some(9));
        let painted = surface.front_pixels().iter().filter(|&&p| p == 9).count();
        assert_eq!(painted, dx.abs().max(dy.abs()) as usize + 1, "octant ({dx},{dy})");
    }
}

/// `display-driver.md §7` phase 1 and resulting zones: the bottom ribbon runs
/// on the viewport side only (`(0,185)-(191,191)`), the message window has no
/// right or bottom rule and runs to the screen edge, and the world viewport,
/// roster box and counters box interiors are left black.
#[test]
fn g4_display_driver_s7_bottom_ribbon_is_viewport_side_only_and_message_window_is_open() {
    let rgba = chrome_frame(&GameplayChromeContent::default());
    assert_eq!(chrome_index_at(&rgba, 100, 188), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 188, 188), CHROME_RIBBON_INDEX);
    for (x, y) in [(250, 188), (319, 150), (319, 191), (250, 191), (300, 120)] {
        assert_eq!(chrome_index_at(&rgba, x, y), 0, "message window ({x}, {y})");
    }
    for (x, y) in [(100, 100), (250, 30), (250, 72)] {
        assert_eq!(chrome_index_at(&rgba, x, y), 0, "box interior ({x}, {y})");
    }
}

/// `display-driver.md §7` phase 3: box A `(7,7)-(184,184)`, box C
/// `(191,7)-(312,56)` and box D `(191,63)-(312,80)` are closed rule boxes in
/// the accent colour; box B is the open "L" `x = 191` over `87..191` and
/// `y = 87` over `191..319`.
#[test]
fn g4_display_driver_s7_phase3_rule_boxes() {
    let rgba = chrome_frame(&GameplayChromeContent::default());
    for (x, y) in [
        // Box A, all four sides.
        (100, 7),
        (100, 184),
        (7, 100),
        (184, 100),
        // Box C top/bottom/sides.
        (250, 7),
        (250, 56),
        (191, 30),
        (312, 30),
        // Box D.
        (250, 63),
        (250, 80),
        (191, 70),
        (312, 70),
        // Box B's two rules.
        (191, 150),
        (319, 87),
    ] {
        assert_eq!(chrome_index_at(&rgba, x, y), CHROME_RULE_INDEX, "rule at ({x}, {y})");
    }
}

/// `display-driver.md §7` "Filled extents versus visible extents": the centre
/// divider shows chrome over `x = 185..190`; the upper stats band over
/// `y = 57..62`; the lower stats band over `y = 81..86`; the right ribbon over
/// `y = 0..86`; the top ribbon over `y = 0..6`.
#[test]
fn g4_display_driver_s7_visible_band_extents_after_overpaint() {
    let rgba = chrome_frame(&GameplayChromeContent::default());
    for x in 185..=190 {
        assert_eq!(chrome_index_at(&rgba, x, 100), CHROME_RIBBON_INDEX, "divider x {x}");
    }
    for y in 57..=62 {
        assert_eq!(chrome_index_at(&rgba, 200, y), CHROME_RIBBON_INDEX, "upper band y {y}");
    }
    for y in 81..=86 {
        assert_eq!(chrome_index_at(&rgba, 200, y), CHROME_RIBBON_INDEX, "lower band y {y}");
    }
    // Rows 0..7 at the far right carry the top-right bevel; test below it.
    for y in 8..=86 {
        assert_eq!(chrome_index_at(&rgba, 315, y), CHROME_RIBBON_INDEX, "right ribbon y {y}");
    }
    for y in 0..=6 {
        assert_eq!(chrome_index_at(&rgba, 100, y), CHROME_RIBBON_INDEX, "top ribbon y {y}");
    }
}

/// `display-driver.md §7` phase 2: the bevel profile at the left edge -
/// column `x = 0` chrome from `y = 5`, `x = 1` from `y = 3`, `x = 2` from
/// `y = 2`, `x = 3` and `x = 4` from `y = 1`, `x = 5` onward from `y = 0`.
#[test]
fn g4_display_driver_s7_corner_bevel_profile_literals() {
    assert_eq!(CHROME_CORNER_PROFILE, [5, 3, 2, 1, 1, 0]);
}

/// `display-driver.md §7` resulting zones: message window cols 24..39 rows
/// 11..23, roster box cols 24..38 rows 1..6, counters box cols 24..38 rows
/// 8..9, the timed-effect slot on row 7 and the plain lower divider on row 10.
#[test]
fn g4_display_driver_s7_text_cell_zones() {
    assert_eq!(
        (MESSAGE_WINDOW_LEFT, MESSAGE_WINDOW_TOP, MESSAGE_WINDOW_RIGHT, MESSAGE_WINDOW_BOTTOM),
        (24, 11, 39, 23)
    );
    assert_eq!((STATS_ROSTER_TOP, STATS_ROSTER_BOTTOM), (1, 6));
    assert_eq!((STATS_COUNTER_TOP, STATS_COUNTER_BOTTOM), (8, 9));
    assert_eq!(TIMING_GLYPH_ROW, 7);
    assert_eq!(LOWER_DIVIDER_ROW, 10);
}

// ---------------------------------------------------------------------------
// systems/encounters.md
// ---------------------------------------------------------------------------

/// Find a seed whose first `1..30` draw is `roll`.
fn g4_seed_with_first_encounter_roll(roll: u16) -> u16 {
    (1u16..)
        .find(|seed| {
            let mut probe = *seed;
            u5_prng_range_u16(&mut probe, 1, 30) == roll
        })
        .unwrap()
}

/// `encounters.md §2.1` and `§3`: the probe rolls `[1, 30]` once and spawns
/// only when `roll < threshold`; the Underworld threshold is 3, so a roll of 2
/// spawns and a roll of 3 does not. A probe that does not fire consumes
/// exactly one draw and writes nothing ("the probe ... writes nothing
/// anywhere").
#[test]
fn g4_encounters_s3_underworld_probe_spawns_only_below_threshold_three() {
    let mut quiet = world_state(vec![0x05; WORLD_CELLS], 100, 100);
    quiet.active_objects.truncate(1);
    let seed = g4_seed_with_first_encounter_roll(3);
    quiet.prng_state = seed;
    let mut expected = seed;
    let _ = u5_prng_range_u16(&mut expected, 1, 30);
    assert_eq!(quiet.apply_native_world_encounter_probe(WorldPlane::Underworld), None);
    assert_eq!(quiet.prng_state, expected);
    assert_eq!(quiet.active_objects.len(), 1);

    let mut fires = world_state(vec![0x05; WORLD_CELLS], 100, 100);
    fires.active_objects.truncate(1);
    fires.prng_state = g4_seed_with_first_encounter_roll(2);
    assert!(
        fires
            .apply_native_world_encounter_probe(WorldPlane::Underworld)
            .is_some()
    );
}

/// `encounters.md §3`: the surface road band `0x20..0x26` has threshold 0 by
/// day, and thresholds 0 and 1 both produce no encounter - for every roll.
#[test]
fn g4_encounters_s3_daytime_road_never_spawns() {
    for roll in 1..=30u16 {
        let mut state = world_state(vec![0x20; WORLD_CELLS], 100, 100);
        state.active_objects.truncate(1);
        state.clock = GameClock::new(12, 0).unwrap();
        state.prng_state = g4_seed_with_first_encounter_roll(roll);
        assert_eq!(
            state.apply_native_world_encounter_probe(WorldPlane::Britannia),
            None,
            "roll {roll}"
        );
    }
}

/// `encounters.md §4`: on success the spawner writes one monster record with
/// the picked base frame in **both** the type byte and the frame byte, the
/// current plane's Z, and a zero auxiliary byte for a land family; the
/// coordinates are more than six cells from the party on both axes.
#[test]
fn g4_encounters_s4_spawned_land_record_carries_frame_in_type_and_tile() {
    let mut state = world_state(vec![0x05; WORLD_CELLS], 100, 100);
    state.active_objects.truncate(1);
    state.prng_state = 0x1234;
    let slot = state
        .spawn_native_world_encounter(WorldPlane::Britannia)
        .expect("grass spawns from the surface land bucket");
    let object = state.active_objects[slot];
    assert_eq!(object.type_byte, object.tile);
    assert_eq!(object.aux1, 0);
    assert_eq!(object.z, WorldPlane::Britannia.save_floor());
    let land_families = [0xC0u8, 0xC8, 0x90, 0x98, 0xBC, 0xC4, 0xD0, 0xE4, 0xCC, 0xD4, 0xDC, 0xD8];
    assert!(land_families.contains(&(object.type_byte & 0xFC)), "{:#04x}", object.type_byte);
    assert!(object.x.abs_diff(100) > 6 && object.y.abs_diff(100) > 6);
}

/// `encounters.md §4` arena inputs: `water` is true for the open-water tiles
/// below 4 and for the river family `0x60..0x6F` except the two bridge tiles
/// `0x6A` and `0x6B`.
#[test]
fn g4_encounters_s4_water_predicate_excludes_the_bridge_pair() {
    for tile in 1u8..=3 {
        assert!(outdoor_combat_terrain_is_water(tile), "tile {tile}");
    }
    for tile in 0x60u8..=0x6F {
        assert_eq!(
            outdoor_combat_terrain_is_water(tile),
            tile != 0x6A && tile != 0x6B,
            "tile {tile:#04x}"
        );
    }
    for tile in [4u8, 5, 0x1D, 0x44, 0x70] {
        assert!(!outdoor_combat_terrain_is_water(tile), "tile {tile:#04x}");
    }
}

/// `encounters.md §4` arena inputs: `ship_target` is a sprite byte whose
/// `0xFC` mask equals `0x2C`, that is any raw byte in `0x2C..0x2F`.
#[test]
fn g4_encounters_s4_ship_target_is_the_masked_0x2c_family() {
    for byte in 0u8..=255 {
        assert_eq!(outdoor_type_is_pirate(byte), (0x2C..=0x2F).contains(&byte), "{byte:#04x}");
    }
}

/// `encounters.md §2.1` gate 3: the transport-marker gate is the value window
/// `0x12..0x15`, not a vehicle-family test.
#[test]
fn g4_encounters_s2_1_transport_gate_is_the_value_window_0x12_to_0x15() {
    assert_eq!(WALKER_GATE_TRANSPORT_MARKER_FIRST, 0x12);
    assert_eq!(WALKER_GATE_TRANSPORT_MARKER_LAST, 0x15);
    for marker in 0u8..=255 {
        assert_eq!(
            transport_marker_gates_per_turn_walkers(marker),
            (0x12..=0x15).contains(&marker),
            "{marker:#04x}"
        );
    }
}

/// `encounters.md §6`: the sleep-ambush chooser picks one of eight rows with
/// equal probability; Giant Rat occupies two rows (2 in 8), each other listed
/// monster one, and there is no ninth row.
#[test]
fn g4_encounters_s6_sleep_ambush_rows_give_giant_rat_two_in_eight() {
    let rows: Vec<SleepAmbushMonster> = (0..8).map(|row| sleep_ambush_monster(row).unwrap()).collect();
    assert_eq!(rows.iter().filter(|m| **m == SleepAmbushMonster::GiantRat).count(), 2);
    for monster in [
        SleepAmbushMonster::Troll,
        SleepAmbushMonster::Bat,
        SleepAmbushMonster::Slime,
        SleepAmbushMonster::GiantSpider,
        SleepAmbushMonster::Gremlin,
        SleepAmbushMonster::Headless,
    ] {
        assert_eq!(rows.iter().filter(|m| **m == monster).count(), 1, "{monster:?}");
    }
    assert_eq!(sleep_ambush_monster(8), None);
}

// ---------------------------------------------------------------------------
// catalogs/tile-catalog.md
// ---------------------------------------------------------------------------

/// `tile-catalog.md §3.1`: combat seating gives each class letter a person-art
/// family - Avatar `A` the avatar family, Bard/Shepherd/Tinker the minstrel
/// family, Druid/Mage the wizard family, Fighter/Paladin/Ranger the fighter
/// family - and every other letter gets no sprite. The families are combat
/// classes 0..3, whose actor bytes are `class * 4 + 0x40` (`§7`).
#[test]
fn g4_tile_catalog_s3_1_class_letters_select_the_four_person_art_families() {
    for (letter, class) in [
        (b'M', 0u8),
        (b'D', 0),
        (b'B', 1),
        (b'S', 1),
        (b'T', 1),
        (b'F', 2),
        (b'P', 2),
        (b'R', 2),
        (b'A', 3),
    ] {
        assert_eq!(combat_party_class_id(letter), Some(class), "{}", letter as char);
    }
    assert_eq!(0u16 * 4 + 0x40 + 256, 320);
    assert_eq!(3u16 * 4 + 0x40 + 256, 332);
    for letter in [b'G', b'X', b'Z', b'0', b' '] {
        assert_eq!(combat_party_class_id(letter), None, "{}", letter as char);
    }
}

/// `tile-catalog.md §3.1` and `§14` against the shipped `LOOK2.DAT`: the
/// terrain-half placeholder rows are exactly `0x00`, `0x59`, `0x89`, `0x8A`,
/// `0xA0`, `0xA4`, `0xC0..0xC3`, `0xCC..0xD3`, `0xD8..0xDB` and `0xF8`; the
/// actor-half placeholder rows are exactly `0x100`, `0x112..0x116`,
/// `0x11C..0x11D`, `0x174..0x177` and `0x17C..0x17F`; the table uses 216
/// distinct string offsets. Skipped when `U5_TEST_ASSET_DIR` is unset.
#[test]
fn g4_tile_catalog_s3_1_shipped_look2_placeholder_rows_and_offset_census() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Ok(bytes) = fs::read(dir.join(LOOK2_DAT_FILE)) else {
        return;
    };
    let table = parse_look2_dat(&bytes).unwrap();
    assert_eq!(table.descriptions.len(), 512);
    let terrain_placeholder = table.descriptions[0].clone();
    let actor_placeholder = table.descriptions[0x100].clone();
    let rows = |range: std::ops::Range<usize>, needle: &str| -> Vec<usize> {
        range.filter(|&i| table.descriptions[i] == needle).collect()
    };
    let mut terrain: Vec<usize> = vec![0x00, 0x59, 0x89, 0x8A, 0xA0, 0xA4, 0xF8];
    terrain.extend(0xC0..=0xC3);
    terrain.extend(0xCC..=0xD3);
    terrain.extend(0xD8..=0xDB);
    terrain.sort_unstable();
    assert_eq!(rows(0..256, &terrain_placeholder), terrain);
    let mut actor: Vec<usize> = vec![0x100];
    actor.extend(0x112..=0x116);
    actor.extend(0x11C..=0x11D);
    actor.extend(0x174..=0x177);
    actor.extend(0x17C..=0x17F);
    assert_eq!(rows(256..512, &actor_placeholder), actor);
    let offsets: std::collections::HashSet<u16> = (0..512)
        .map(|tile| u16::from_le_bytes([bytes[tile * 2], bytes[tile * 2 + 1]]))
        .collect();
    assert_eq!(offsets.len(), 216);
}

// ---------------------------------------------------------------------------
// systems/blackthorn.md
// ---------------------------------------------------------------------------

/// `blackthorn.md §3` step 8: the release order joins `GUARD! Release this
/// good`, a gender piece (` lady ` for female `0x0C`, `man ` for male `0x0B`,
/// nothing otherwise) and `at once!"`, giving exactly the three published
/// lines.
#[test]
fn g4_blackthorn_s3_release_line_has_the_three_published_forms() {
    let line = |gender| crate::blackthorn::blackthorn_release_line(gender);
    assert_eq!(line(0x0C), "\n\nGUARD! Release this good lady at once!\"");
    assert_eq!(line(0x0B), "\n\nGUARD! Release this goodman at once!\"");
    assert_eq!(line(0x00), "\n\nGUARD! Release this goodat once!\"");
    assert_eq!(line(0x41), "\n\nGUARD! Release this goodat once!\"");
}

/// `blackthorn.md §3` step 8: the greeting addresses roster record 0's name
/// between `\n\nBlackthorn says:\n\n"Ah, ` and `!\n'Tis indeed an honour to
/// meet thee at last! ` (one trailing space, no line feed).
#[test]
fn g4_blackthorn_s3_greeting_line_wraps_the_name_in_the_published_literals() {
    assert_eq!(
        crate::blackthorn::blackthorn_greeting_line("Placeholder"),
        "\n\nBlackthorn says:\n\n\"Ah, Placeholder!\n'Tis indeed an honour to meet thee at last! "
    );
}

/// `blackthorn.md §4`: the first wrong answer threatens, later wrong answers
/// stamp the hourglass, and the fourth wrong answer kills.
#[test]
fn g4_blackthorn_s4_wrong_answer_escalation_ladder() {
    use crate::blackthorn_session::{BlackthornWrongEscalation, escalation_for_wrong_count};
    assert_eq!(escalation_for_wrong_count(0), None);
    assert_eq!(escalation_for_wrong_count(1), Some(BlackthornWrongEscalation::Threat));
    assert_eq!(escalation_for_wrong_count(2), Some(BlackthornWrongEscalation::TileStamp));
    assert_eq!(escalation_for_wrong_count(3), Some(BlackthornWrongEscalation::TileStamp));
    assert_eq!(escalation_for_wrong_count(4), Some(BlackthornWrongEscalation::Kill));
}

/// `blackthorn.md §6.1`: the map ships the spent hourglass `0xE8`; the first
/// wrong answer's script writes `0xE9`; later stamps write `0xEB` at index 1
/// and `0xE8` at index 2; index 3 executes rather than stamping. The visible
/// all-wrong sequence is `0xE8 -> 0xE9 -> 0xEB -> 0xE8`.
#[test]
fn g4_blackthorn_s6_1_hourglass_sequence_for_an_all_wrong_run() {
    assert_eq!(BLACKTHORN_HOURGLASS_CELL, (5, 9));
    assert_eq!(BLACKTHORN_HOURGLASS_SPENT_TILE, 0xE8);
    assert_eq!(BLACKTHORN_HOURGLASS_TILE, 0xE9);
    assert_eq!(blackthorn_hourglass_stamp(1), Some(0xEB));
    assert_eq!(blackthorn_hourglass_stamp(2), Some(0xE8));
    assert_eq!(blackthorn_hourglass_stamp(3), None);
    let sequence = [
        BLACKTHORN_HOURGLASS_SPENT_TILE,
        BLACKTHORN_HOURGLASS_TILE,
        blackthorn_hourglass_stamp(1).unwrap(),
        blackthorn_hourglass_stamp(2).unwrap(),
    ];
    assert_eq!(sequence, [0xE8, 0xE9, 0xEB, 0xE8]);
}

fn g4_vm_after(beats: &[BlackthornCutsceneBeat]) -> BlackthornCutsceneVm {
    let mut vm = BlackthornCutsceneVm::with_audience_setup(vec![0; 32 * 11]);
    for beat in beats {
        let mut next = BlackthornCutsceneVm::new(vm.tile_buffer.clone());
        next.actors = vm.actors;
        next.visible_actors = vm.actors;
        next.run(blackthorn_cutscene_beat_commands(*beat));
        vm = next;
    }
    vm
}

/// `blackthorn.md §3` step 14 and `§6.3` "Guard approach": the guards start at
/// `(4,10)` and `(6,10)`; one stinger, a paired north step, then three paired
/// steps with slot 6 west and slot 7 east, then quiet pause 8 - five stings in
/// all - leaving the guards at `(1,9)` and `(9,9)`.
#[test]
fn g4_blackthorn_s6_3_guard_approach_separates_the_guards() {
    let setup = g4_vm_after(&[]);
    let cell = |vm: &BlackthornCutsceneVm, actor| vm.actor(actor).map(|a| (a.x, a.y));
    assert_eq!(cell(&setup, BlackthornCutsceneActor::LeftGuard), Some((4, 10)));
    assert_eq!(cell(&setup, BlackthornCutsceneActor::RightGuard), Some((6, 10)));
    let vm = g4_vm_after(&[BlackthornCutsceneBeat::AudienceThroneApproach]);
    assert_eq!(vm.stinger_count, 5);
    assert_eq!(vm.world_ticks, 5 * 2 + 8);
    assert_eq!(cell(&vm, BlackthornCutsceneActor::LeftGuard), Some((1, 9)));
    assert_eq!(cell(&vm, BlackthornCutsceneActor::RightGuard), Some((9, 9)));
}

/// `blackthorn.md §6.3` "Guard release route": quiet pause 11, then the
/// acting guard moves west once, north four times and east once with a
/// stinger-plus-two-tick redraw after every step; Blackthorn stays seated in
/// slot 8. Six stings, 11 + 12 world ticks and as many one-tick delays.
#[test]
fn g4_blackthorn_s6_3_guard_release_route_counts_and_end_cell() {
    let approach = g4_vm_after(&[BlackthornCutsceneBeat::AudienceThroneApproach]);
    let vm = g4_vm_after(&[
        BlackthornCutsceneBeat::AudienceThroneApproach,
        BlackthornCutsceneBeat::GuardReleaseRoute,
    ]);
    assert_eq!(vm.stinger_count, 6);
    assert_eq!(vm.world_ticks, 11 + 6 * 2);
    assert_eq!(vm.bios_ticks, 11 + 6 * 2);
    let start = approach.actor(BlackthornCutsceneActor::LeftGuard).unwrap();
    let end = vm.actor(BlackthornCutsceneActor::LeftGuard).unwrap();
    assert_eq!((end.x as i32 - start.x as i32, end.y as i32 - start.y as i32), (0, -4));
    assert_eq!(
        vm.actor(BlackthornCutsceneActor::SeatedBlackthorn),
        approach.actor(BlackthornCutsceneActor::SeatedBlackthorn)
    );
}

/// `blackthorn.md §6.3` "Conditional Blackthorn-tableau cleanup": quiet pause
/// 4; the tableau moves east once and south five times with an animated
/// redraw after every step; slot 8 is cleared; one explicit world tick.
/// Six stings, 4 + 12 + 1 world ticks and 4 + 12 one-tick delays.
#[test]
fn g4_blackthorn_s6_3_conditional_cleanup_counts_and_clear() {
    let mut vm = BlackthornCutsceneVm::with_audience_setup(vec![0; 32 * 11]);
    vm.set_actor(
        BlackthornCutsceneActor::SeatedBlackthorn,
        BlackthornCutsceneActorState {
            x: 5,
            y: 5,
            actor_byte: BLACKTHORN_SEATED_ACTOR_BYTE,
        },
    );
    vm.visible_actors = vm.actors;
    vm.run(blackthorn_cutscene_beat_commands(
        BlackthornCutsceneBeat::ConditionalThroneCleanup,
    ));
    assert_eq!(vm.stinger_count, 6);
    assert_eq!(vm.world_ticks, 4 + 6 * 2 + 1);
    assert_eq!(vm.bios_ticks, 4 + 6 * 2);
    assert_eq!(vm.actor(BlackthornCutsceneActor::SeatedBlackthorn), None);
    let positions: Vec<(usize, usize)> = vm
        .actors_at_each_sting
        .iter()
        .filter_map(|actors| actors[8].map(|a| (a.x, a.y)))
        .collect();
    assert_eq!(
        positions,
        vec![(6, 5), (6, 6), (6, 7), (6, 8), (6, 9), (6, 10)]
    );
}

/// `blackthorn.md §7a` branch 3: the default tribute is ten gold per
/// **living** party member; a party that cannot pay the full amount is
/// refused, nothing is taken.
#[test]
fn g4_blackthorn_s7a_tribute_is_ten_per_living_member_and_unaffordable_takes_nothing() {
    assert_eq!(BLACKTHORN_GUARD_TRIBUTE_PER_LIVING_MEMBER, 10);
    let start = begin_blackthorn_guard_demand(crate::SCENE_YEW, false, 3);
    let BlackthornGuardDemandStart::Prompt(prompt) = start else {
        panic!("tribute prompt expected");
    };
    assert_eq!(prompt, BlackthornGuardDemandPrompt::Tribute { amount: 30 });
    assert_eq!(
        resolve_blackthorn_guard_demand(prompt, "Y", 29),
        BlackthornGuardDemandResolution::Refused { gold: 29 }
    );
    assert_eq!(
        resolve_blackthorn_guard_demand(prompt, "Y", 30),
        BlackthornGuardDemandResolution::PaidOrPassed { gold: 0 }
    );
}

/// `blackthorn.md §7a` branch 1: up to fourteen characters may be typed but
/// only the first four are compared, with letter case folded, so any word
/// sharing the password's first four letters is accepted.
#[test]
fn g4_blackthorn_s7a_palace_password_compares_four_case_folded_characters() {
    assert_eq!(BLACKTHORN_GUARD_PASSWORD_INPUT_MAX, 14);
    assert_eq!(BLACKTHORN_GUARD_PASSWORD_COMPARE_LEN, 4);
    let prefix: String = BLACKTHORN_GUARD_PASSWORD.chars().take(4).collect();
    let prompt = BlackthornGuardDemandPrompt::PalacePassword;
    for typed in [
        prefix.to_ascii_lowercase(),
        format!("{}zzzz", prefix.to_ascii_lowercase()),
        BLACKTHORN_GUARD_PASSWORD.to_string(),
    ] {
        assert!(
            matches!(
                resolve_blackthorn_guard_demand(prompt, &typed, 5),
                BlackthornGuardDemandResolution::PaidOrPassed { gold: 5 }
            ),
            "{typed}"
        );
    }
    let short: String = prefix.chars().take(3).collect();
    assert_eq!(
        resolve_blackthorn_guard_demand(prompt, &short, 5),
        BlackthornGuardDemandResolution::Refused { gold: 5 }
    );
}

/// `encounters.md §4` "Spawn count": the two identity-gap classes carry a
/// default spawn count of zero, which the original would hand to its range
/// roller as `[1, 0]`; "A conforming engine should treat a zero default spawn
/// count as 'place the single lead actor and no others' rather than
/// reproducing the fault."
#[test]
fn g4_encounters_s4_zero_default_spawn_count_places_only_the_lead_actor() {
    for (fortunes, first_seed) in [(0u8, 0u8), (0, 29), (1, 7)] {
        assert_eq!(
            resolve_terrain_combat_setup_count(0, fortunes, first_seed, 3, false),
            1,
            "fortunes {fortunes} seed {first_seed}"
        );
    }
}
