// Closed-audit follow-up, group g5: rules the engine implements that no
// existing test asserted. Spec files: `formats/font-hcs.md`,
// `systems/boot.md`, `formats/shoppe-dat.md`, `systems/chargen.md`,
// `systems/u4-transfer.md`, `systems/moons.md`, `systems/active-objects.md`,
// `systems/doors-and-z-transitions.md` and `systems/inventory.md`.
//
// Every test names its file and section. No game text appears here beyond
// literals the spec publishes; record bodies are invented placeholders.

// ---------------------------------------------------------------------------
// formats/font-hcs.md
// ---------------------------------------------------------------------------

fn g5_blank_hcs_body() -> Vec<u8> {
    vec![0; FIXED_FONT_GLYPH_COUNT * HCS_FONT_CELL_HEIGHT * (HCS_FONT_CELL_WIDTH / 8)]
}

fn g5_parse_hcs(body: &[u8]) -> io::Result<FixedFont> {
    parse_fixed_font_body(body, "fixture.hcs", HCS_FONT_CELL_WIDTH, HCS_FONT_CELL_HEIGHT)
}

/// `font-hcs.md §2` / `§7`: a strict reader rejects any length other than
/// exactly 3,072 bytes, one byte short or one byte long included.
#[test]
fn g5_font_hcs_s7_strict_reader_rejects_any_length_but_3072() {
    let body = g5_blank_hcs_body();
    assert_eq!(body.len(), 3072);
    assert!(g5_parse_hcs(&body).is_ok());
    assert!(g5_parse_hcs(&body[..3071]).is_err());
    let mut long = body.clone();
    long.push(0);
    assert!(g5_parse_hcs(&long).is_err());
    assert!(g5_parse_hcs(&[]).is_err());
}

/// `font-hcs.md §4`/`§5`: glyph `n` starts at byte `n * 24`; rows are stored
/// top to bottom, two bytes per row; the second row byte holds pixels 8..15
/// with bit 7 first.
#[test]
fn g5_font_hcs_s4_s5_glyph_offset_row_order_and_second_byte_bits() {
    let mut body = g5_blank_hcs_body();
    // Glyph 127, last row (row 11): only bit 7 of the second byte -> pixel 8.
    body[127 * 24 + 11 * 2 + 1] = 0b1000_0000;
    // Glyph 64, row 1 (bytes 2 and 3): bit 0 of the first byte -> pixel 7.
    body[64 * 24 + 2] = 0b0000_0001;
    let font = g5_parse_hcs(&body).unwrap();

    let last = font.glyph(127).unwrap();
    assert_eq!((last.width, last.height), (16, 12));
    for (index, pixel) in last.pixels.iter().enumerate() {
        let expected = u8::from(index == 11 * 16 + 8);
        assert_eq!(*pixel, expected, "glyph 127 pixel {index}");
    }
    let mid = font.glyph(64).unwrap();
    for (index, pixel) in mid.pixels.iter().enumerate() {
        let expected = u8::from(index == 16 + 7);
        assert_eq!(*pixel, expected, "glyph 64 pixel {index}");
    }
    // Neighbouring glyphs are untouched: the stride is exactly 24 bytes.
    assert!(font.glyph(126).unwrap().pixels.iter().all(|p| *p == 0));
    assert!(font.glyph(63).unwrap().pixels.iter().all(|p| *p == 0));
}

/// `font-hcs.md §7`: "Treat all bytes as bitmap data. There are no reserved
/// byte values", and indices outside `0..=127` are rejected.
#[test]
fn g5_font_hcs_s7_every_byte_is_bitmap_and_high_codes_are_rejected() {
    let body = vec![0xFF; 3072];
    let font = g5_parse_hcs(&body).unwrap();
    assert_eq!(font.glyphs.len(), 128);
    for code in 0..128u8 {
        assert!(font.glyph(code).unwrap().pixels.iter().all(|p| *p == 1));
    }
    assert!(font.glyph(128).is_none());
    assert!(font.glyph(0xFF).is_none());
    assert!(prepare_fixed_text_cell(&font, 0x80, TextCellStyle::default()).is_err());
}

/// `font-hcs.md §2` against the shipped pair, asset-gated.
#[test]
fn g5_font_hcs_s2_shipped_fonts_are_exactly_3072_bytes() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    for name in [IBM_HCS_FILE, RUNES_HCS_FILE] {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        assert_eq!(fs::metadata(&path).unwrap().len(), 3072, "{name}");
    }
}

// ---------------------------------------------------------------------------
// systems/boot.md
// ---------------------------------------------------------------------------

/// `boot.md §5`: the parser "looks only at the first character of the first
/// argument, folds that character to uppercase" - every letter folds, and a
/// recognised letter in second position is not a selector.
#[test]
fn g5_boot_s5_selector_folds_every_letter_and_reads_only_the_first() {
    assert_eq!(parse_explicit_driver_selector(Some("c")), Some(DisplayDriverFamily::Cga));
    assert_eq!(parse_explicit_driver_selector(Some("t")), Some(DisplayDriverFamily::Tandy));
    assert_eq!(parse_explicit_driver_selector(Some("H")), Some(DisplayDriverFamily::Hercules));
    assert_eq!(parse_explicit_driver_selector(Some("E")), Some(DisplayDriverFamily::Ega));
    assert_eq!(parse_explicit_driver_selector(Some("xE")), None);
    assert_eq!(parse_explicit_driver_selector(Some(" E")), None);
}

/// `boot.md §5`: the EGA sentinel takes no driver-load path only "if no
/// explicit selector rewrites the display family" - an explicit selector
/// wins over the sentinel exactly as it wins over any detection.
#[test]
fn g5_boot_s5_explicit_selector_rewrites_the_ega_sentinel() {
    for family in [
        DisplayDriverFamily::Cga,
        DisplayDriverFamily::Ega,
        DisplayDriverFamily::Tandy,
        DisplayDriverFamily::Hercules,
    ] {
        assert_eq!(
            resolve_driver_family(Some(family), GraphicsCapability::EgaSentinel),
            Some(family)
        );
        assert_eq!(
            resolve_driver_family(Some(family), GraphicsCapability::Hercules),
            Some(family)
        );
    }
}

// ---------------------------------------------------------------------------
// formats/shoppe-dat.md
// ---------------------------------------------------------------------------

/// `shoppe-dat.md §5`: "A shop token `t` selects catalog index `t - 0x7F`
/// ... a conversation token is the catalog index itself. Shop token `0x80`
/// and conversation token `0x01` are the same entry." Exhaustive.
#[test]
fn g5_shoppe_dat_s5_shop_token_bias_matches_conversation_token_for_all_128() {
    for token in 0x80u8..=0xFF {
        let shop = shoppe_dictionary_index(token);
        let talk = tlk_dictionary_index(token - 0x7F);
        assert!(shop.is_some(), "token {token:#04x}");
        assert_eq!(shop, talk, "token {token:#04x}");
    }
    for low in 0x00u8..0x80 {
        assert_eq!(shoppe_dictionary_index(low), None, "byte {low:#04x}");
    }
}

/// `shoppe-dat.md §3`: "This includes `0xFF`, which is a valid token byte in
/// the shared dictionary" - it is not an end-of-record marker, so text after
/// it still renders. `§5` spacing: a leading space always, a trailing space
/// only before ordinary text.
#[test]
fn g5_shoppe_dat_s3_token_0xff_expands_and_does_not_terminate() {
    let word = PUBLISHED_COMMON_WORD_DICTIONARY[127];
    assert!(!word.is_empty());
    let rendered =
        crate::shoppe_bark::render_shoppe_bark(&[b'a', 0xFF, b'b'], &crate::shoppe_bark::ShoppeBarkContext::default()).unwrap();
    assert_eq!(rendered, format!("a {word} b"));
    // Followed by the record terminator: no trailing space.
    let rendered = crate::shoppe_bark::render_shoppe_bark(&[b'a', 0xFF, 0], &crate::shoppe_bark::ShoppeBarkContext::default()).unwrap();
    assert_eq!(rendered, format!("a {word}"));
}

/// `shoppe-dat.md §2`: "Parsers should split on the terminator and stop at
/// end of file rather than expecting a fixed slot count." A three-record
/// buffer is three records.
#[test]
#[ignore = "divergence: g5-shoppe-fixed-slot-count"]
fn g5_shoppe_dat_s2_parser_stops_at_end_of_file_not_a_fixed_slot_count() {
    let records = crate::shoppe_bark::parse_shoppe_records(b"one\0two\0three\0");
    assert_eq!(records.slot_count(), 3);
    assert!(records.record(3).is_none());
}

/// `shoppe-dat.md §2`: the shipped file is 10,135 bytes, holds exactly 195
/// NUL terminators (ordinals `0..=194`), and its last byte terminates the
/// last record. Asset-gated; reports counts only.
#[test]
fn g5_shoppe_dat_s2_shipped_file_has_195_terminated_records() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let path = dir.join("SHOPPE.DAT");
    if !path.is_file() {
        return;
    }
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 10_135);
    assert_eq!(bytes.iter().filter(|b| **b == 0).count(), 195);
    assert_eq!(*bytes.last().unwrap(), 0);
}

/// `shoppe-dat.md §7` selection table, shipwright and tavern rows: menu body
/// `119`; Frigate `117` / Skiff `118` quote then shared confirmation `126`;
/// pending cases `125`, `120`, `121`; post-sale `123` then `124`; the
/// table-scraps outcome `90`.
#[test]
fn g5_shoppe_dat_s7_shipwright_and_scraps_record_ids() {
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_MENU, 119);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_FRIGATE_OFFER, 117);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_SKIFF_OFFER, 118);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_CONFIRM, 126);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_DOCK_SPACE, 125);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_SKIFF_STOWED, 120);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_ALREADY_SKIFF, 121);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_DELIVERY_QUEUED, 123);
    assert_eq!(SHOPPE_RECORD_SHIPWRIGHT_CONTINUATION, 124);
    assert_eq!(TAVERN_TABLE_SCRAPS_RECORD_ID, 90);
    assert_eq!(TAVERN_PROVISION_QUOTE_RECORD_FIRST, 77);
    assert_eq!(TAVERN_PROVISION_QUOTE_RECORD_LAST, 83);
}

// ---------------------------------------------------------------------------
// systems/chargen.md
// ---------------------------------------------------------------------------

/// `chargen.md §4`: gender polls `M` or `F`, "upper or lower case, folded to
/// upper case"; `§6`: answers `A` or `B` likewise; every other key loops.
#[test]
fn g5_chargen_s4_s6_gender_and_answer_keys_fold_case() {
    assert_eq!(chargen_gender_key(b'M'), Some(true));
    assert_eq!(chargen_gender_key(b'm'), Some(true));
    assert_eq!(chargen_gender_key(b'F'), Some(false));
    assert_eq!(chargen_gender_key(b'f'), Some(false));
    assert_eq!(chargen_answer_key(b'A'), Some(true));
    assert_eq!(chargen_answer_key(b'a'), Some(true));
    assert_eq!(chargen_answer_key(b'B'), Some(false));
    assert_eq!(chargen_answer_key(b'b'), Some(false));
    for key in 0u8..=255 {
        if !matches!(key, b'M' | b'm' | b'F' | b'f') {
            assert_eq!(chargen_gender_key(key), None, "gender key {key}");
        }
        if !matches!(key, b'A' | b'a' | b'B' | b'b') {
            assert_eq!(chargen_answer_key(key), None, "answer key {key}");
        }
    }
}

fn g5_chargen_records() -> Vec<String> {
    (0..30).map(|index| format!("placeholder {index}")).collect()
}

/// `chargen.md §6`: "any other key loops silently with no redraw and no
/// error"; a lower-case `b` picks the right-hand (larger-index) virtue.
#[test]
fn g5_chargen_s6_lowercase_b_answers_and_other_keys_are_ignored() {
    let pool: Vec<u8> = (0..64).collect();
    let mut session = ChargenSession::new(g5_chargen_records(), pool).unwrap();
    session.submit_name("Hero");
    session.submit_gender_key(b'f');
    let ChargenSessionStep::PresentQuestion(first) = session.advance_intro() else {
        panic!("expected the first question");
    };
    assert!(first.option_a.index() < first.option_b.index());
    assert_eq!(session.submit_answer_key(b'x'), ChargenSessionStep::Ignored);
    assert_eq!(session.submit_answer_key(0x1B), ChargenSessionStep::Ignored);
    // The question is still current after the ignored keys.
    assert_eq!(
        session.current_step(),
        ChargenSessionStep::PresentQuestion(first.clone())
    );
    let mut step = session.submit_answer_key(b'b');
    let mut answered = 1;
    while let ChargenSessionStep::PresentQuestion(_) = step {
        step = session.submit_answer_key(b'b');
        answered += 1;
    }
    assert_eq!(answered, CHARGEN_QUESTION_COUNT);
    let _ = session.advance_intro();
    let result = session.result().expect("completed");
    let q0 = &result.tournament.questions[0];
    assert!(!q0.chose_a);
    assert_eq!(q0.winner, first.option_b);
    assert_eq!(q0.loser, first.option_a);
    assert!(!result.male);
}

/// `chargen.md §4`: "names exactly eight characters fill the entered-name
/// slice with no terminator"; the ninth byte remains seed padding.
#[test]
fn g5_chargen_s4_eight_character_name_has_no_terminator() {
    let mut save = saved_game_seed_bytes(0, 0, 10, 20);
    let record = SAVE_ROSTER_OFFSET;
    save[record + 8] = 0x5A;
    let stats = ChargenStats {
        strength: 20,
        dexterity: 15,
        intelligence: 15,
    };
    apply_chargen_to_save(&mut save, b"ABCDEFGH", true, stats).unwrap();
    assert_eq!(&save[record..record + 8], b"ABCDEFGH");
    assert_eq!(save[record + 8], 0x5A);
}

/// `chargen.md §6`/`§7`: all three tallies start at the seed's 15; STR is
/// floored to twenty, DEX and INT are written as tallied. Humility adds
/// nothing, so a Humility-only winner list is the bare seed.
#[test]
fn g5_chargen_s7_tallies_start_at_fifteen_and_only_strength_is_floored() {
    for winners in [vec![], vec![ShrineVirtue::Humility; 7]] {
        let stats = chargen_stats_from_winners(&winners);
        assert_eq!(stats.strength, 20);
        assert_eq!(stats.dexterity, 15);
        assert_eq!(stats.intelligence, 15);
    }
    // Seven Valor wins: 15 + 14 = 29, above the floor.
    let stats = chargen_stats_from_winners(&[ShrineVirtue::Valor; 7]);
    assert_eq!(stats.strength, 29);
}

/// `chargen.md §5.1.2`: the per-question paragraph starts at pen `(0, 152)`
/// with a nine-pixel advance, so "its lines land at y = 152, 161, 170, 179
/// and 188 before the clip at 192 stops output".
#[test]
fn g5_chargen_s5_1_2_question_paragraph_lines_land_on_published_rows() {
    let text = "word ".repeat(400);
    let placed = layout_proportional_paragraph_glyphs(
        &PROPORTIONAL_WIDTH_TABLE,
        &CHARGEN_QUESTION_PARAGRAPH_BOX,
        text.as_bytes(),
        PROPORTIONAL_DRAW_CLIP_Y,
    )
    .unwrap();
    let rows: std::collections::BTreeSet<u16> = placed.iter().map(|glyph| glyph.y).collect();
    assert_eq!(rows.into_iter().collect::<Vec<_>>(), vec![152, 161, 170, 179, 188]);
}

/// `chargen.md §7`/`§8`, asset-gated: the shipped seed's record 0 carries
/// STR/DEX/INT 15, class `A`, status `G`, HP 60/60, XP 150, level 2, and
/// only four class letters occur in the roster (Shamino `F`, Iolo `B`,
/// Mariah `M`).
#[test]
fn g5_chargen_s7_s8_init_gam_avatar_seed_and_roster_classes() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let path = dir.join(INIT_GAM_FILENAME);
    if !path.is_file() {
        return;
    }
    let save = fs::read(&path).unwrap();
    let record = |slot: usize| SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
    let avatar = record(0);
    assert_eq!(save[avatar + SAVE_CHARACTER_STR_OFFSET], 15);
    assert_eq!(save[avatar + SAVE_CHARACTER_DEX_OFFSET], 15);
    assert_eq!(save[avatar + SAVE_CHARACTER_INT_OFFSET], 15);
    assert_eq!(save[avatar + SAVE_CHARACTER_CLASS_OFFSET], b'A');
    assert_eq!(save[avatar + SAVE_CHARACTER_STATUS_OFFSET], b'G');
    let word = |at: usize| u16::from_le_bytes([save[at], save[at + 1]]);
    assert_eq!(word(avatar + SAVE_CHARACTER_HP_OFFSET), 60);
    assert_eq!(word(avatar + SAVE_CHARACTER_MAX_HP_OFFSET), 60);
    assert_eq!(word(avatar + SAVE_CHARACTER_EXPERIENCE_OFFSET), 150);
    assert_eq!(save[avatar + SAVE_CHARACTER_LEVEL_OFFSET], 2);
    for slot in 0..16 {
        let class = save[record(slot) + SAVE_CHARACTER_CLASS_OFFSET];
        assert!(matches!(class, b'A' | b'M' | b'B' | b'F'), "slot {slot}");
    }
    assert_eq!(save[record(1) + SAVE_CHARACTER_CLASS_OFFSET], b'F');
    assert_eq!(save[record(2) + SAVE_CHARACTER_CLASS_OFFSET], b'B');
    assert_eq!(save[record(3) + SAVE_CHARACTER_CLASS_OFFSET], b'M');
}

/// `chargen.md §8` factory-seed readied equipment table, asset-gated.
#[test]
fn g5_chargen_s8_init_gam_readied_equipment_matches_published_table() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let path = dir.join(INIT_GAM_FILENAME);
    if !path.is_file() {
        return;
    }
    let save = fs::read(&path).unwrap();
    const E: &str = "Empty";
    let table: [[&str; 6]; 16] = [
        ["Chain Coif", "Chain Mail", "Long Sword", E, E, "Ankh"],
        ["Leather Helm", "Ring Mail", "Short Sword", "Small Shield", E, E],
        ["Leather Helm", "Leather Armour", "Main Gauche", "Short Sword", E, E],
        [E, "Cloth Armour", "Dagger", E, E, E],
        ["Spiked Helm", "Scale Mail", "Mace", "Spiked Shield", E, "Spiked Collar"],
        [E, "Cloth Armour", "Dagger", E, E, E],
        ["Chain Coif", "Leather Armour", "Spear", E, E, E],
        ["Iron Helm", "Scale Mail", "2H Sword", E, E, E],
        [E, "Cloth Armour", "Club", E, E, E],
        ["Leather Helm", "Leather Armour", "Short Sword", "Small Shield", E, E],
        ["Leather Helm", "Leather Armour", "Sling", E, E, E],
        [E, "Leather Armour", "Dagger", E, E, "Amulet/Turning"],
        ["Iron Helm", "Scale Mail", "Club", "Large Shield", E, E],
        ["Chain Coif", "Leather Armour", "Throwing Axe", "Small Shield", E, E],
        [E, "Cloth Armour", "Main Gauche", E, "Ring of Protection", E],
        ["Iron Helm", "Scale Mail", "Morning Star", E, E, E],
    ];
    for (slot, row) in table.iter().enumerate() {
        let base = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN + SAVE_CHARACTER_EQUIPMENT_OFFSET;
        for (column, expected) in row.iter().enumerate() {
            let byte = save[base + column];
            let got = if byte == EQUIPMENT_EMPTY {
                E
            } else {
                equipment_name(byte as usize)
            };
            assert_eq!(got, *expected, "roster slot {slot} column {column}");
        }
    }
}

// ---------------------------------------------------------------------------
// systems/u4-transfer.md
// ---------------------------------------------------------------------------

fn g5_u4_source(class_index: u8, is_avatar: bool) -> U4TransferSource {
    U4TransferSource {
        name: b"HERO".to_vec(),
        male: true,
        class_index,
        strength: 3,
        dexterity: 3,
        intelligence: 0,
        experience: 9,
        is_avatar,
    }
}

/// `u4-transfer.md §7`: class index `0..7` translates to Mage, Bard,
/// Fighter, Druid, Tinker, Paladin, Ranger, Shepherd; a passed Avatarhood
/// test then overwrites the letter with the Avatar letter.
#[test]
fn g5_u4_transfer_s7_class_index_translation_and_avatar_override() {
    let letters = [b'M', b'B', b'F', b'D', b'T', b'P', b'R', b'S'];
    for (index, letter) in letters.iter().enumerate() {
        let mut save = saved_game_seed_bytes(0, 0, 10, 20);
        let avatar =
            apply_u4_transfer_to_save(&mut save, &g5_u4_source(index as u8, false), None).unwrap();
        assert_eq!(avatar.class_byte, *letter, "class index {index}");
        assert_eq!(save[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_CLASS_OFFSET], *letter);

        let mut save = saved_game_seed_bytes(0, 0, 10, 20);
        let avatar =
            apply_u4_transfer_to_save(&mut save, &g5_u4_source(index as u8, true), None).unwrap();
        assert_eq!(avatar.class_byte, b'A', "Avatar override on index {index}");
    }
}

/// `u4-transfer.md §7`: STR alone is floored to 20; DEX and INT keep a
/// sub-ten source value; MP copies INT; status forced Good; XP divided by
/// ten truncating; level 1 below 100 scaled XP; HP `30 * level`.
#[test]
fn g5_u4_transfer_s7_low_stats_unfloored_and_progress_recalculated() {
    let mut save = saved_game_seed_bytes(0, 0, 10, 20);
    let record = SAVE_ROSTER_OFFSET;
    save[record + SAVE_CHARACTER_STATUS_OFFSET] = b'D';
    let avatar = apply_u4_transfer_to_save(&mut save, &g5_u4_source(2, false), None).unwrap();
    assert_eq!(avatar.strength, 20);
    assert_eq!(avatar.dexterity, 3);
    assert_eq!(avatar.intelligence, 0);
    assert_eq!(save[record + SAVE_CHARACTER_MANA_OFFSET], 0);
    assert_eq!(save[record + SAVE_CHARACTER_STATUS_OFFSET], b'G');
    assert_eq!(avatar.experience, 0);
    assert_eq!(avatar.level, 1);
    assert_eq!(avatar.hp, 30);

    // 9999 source XP -> 999 scaled -> quotient 9 halves 4 times -> level 5.
    let mut source = g5_u4_source(2, false);
    source.experience = 9999;
    let mut save = saved_game_seed_bytes(0, 0, 10, 20);
    let avatar = apply_u4_transfer_to_save(&mut save, &source, None).unwrap();
    assert_eq!(avatar.experience, 999);
    assert_eq!(avatar.level, 5);
    assert_eq!(avatar.hp, 150);
    let word = |at: usize| u16::from_le_bytes([save[at], save[at + 1]]);
    assert_eq!(word(record + SAVE_CHARACTER_HP_OFFSET), 150);
    assert_eq!(word(record + SAVE_CHARACTER_MAX_HP_OFFSET), 150);
}

/// `u4-transfer.md §7`: "Preserve the source male marker as U5 male; any
/// other source value becomes U5 female."
#[test]
fn g5_u4_transfer_s7_only_the_male_marker_imports_as_male() {
    let source = U4PreviewSource {
        name: "HERO".to_string(),
        male: true,
        class_index: 2,
        strength: 20,
        dexterity: 20,
        intelligence: 20,
        experience: 100,
        max_hit_points: 200,
        is_avatar: false,
    };
    let sex_at = U4_PREVIEW_RECORD_OFFSET + U4_PREVIEW_RECORD_SEX_OFFSET;
    for (marker, male) in [(0x0B, true), (0x0C, false), (0x00, false), (b'M', false), (0xFF, false)] {
        let mut bytes = synthetic_party_sav_fixture(&source);
        bytes[sex_at] = marker;
        assert_eq!(parse_u4_preview_source(&bytes).unwrap().male, male, "marker {marker:#04x}");
    }
}

// ---------------------------------------------------------------------------
// systems/moons.md
// ---------------------------------------------------------------------------

/// `moons.md §2.2`: the published visible-hours/cell-position table,
/// checked exhaustively over all twenty-four hours for all three markers.
#[test]
fn g5_moons_s2_2_marker_positions_for_every_hour() {
    for hour in 0u8..24 {
        let fixed = (6..=17).contains(&hour).then(|| 17 - hour);
        let trammel = if hour <= 8 {
            Some(8 - hour)
        } else if hour >= 21 {
            Some(32 - hour)
        } else {
            None
        };
        let felucca = if hour <= 2 {
            Some(2 - hour)
        } else if hour >= 15 {
            Some(26 - hour)
        } else {
            None
        };
        assert_eq!(sky_strip_marker_position(hour, SkyStripMarker::FixedHour), fixed, "hour {hour}");
        assert_eq!(sky_strip_marker_position(hour, SkyStripMarker::Trammel), trammel, "hour {hour}");
        assert_eq!(sky_strip_marker_position(hour, SkyStripMarker::Felucca), felucca, "hour {hour}");
    }
}

/// `moons.md §2.2` worked example: date `4-5-139` at hour 8 - hour marker in
/// cell 9, Trammel in cell 0 showing the day-5 glyph `'2'`, Felucca not
/// drawn; every other cell blank.
#[test]
fn g5_moons_s2_2_worked_example_day_five_hour_eight() {
    let glyphs = cached_moon_glyph_bytes_for_day(5).unwrap();
    assert_eq!(glyphs, [b'2', b'3']);
    let cells = sky_strip_cells(8, glyphs);
    assert_eq!(cells[9].unwrap().rune_code, SKY_STRIP_HOUR_MARKER_RUNE);
    assert_eq!(cells[0].unwrap().rune_code, b'2');
    for (index, cell) in cells.iter().enumerate() {
        if index != 0 && index != 9 {
            assert!(cell.is_none(), "cell {index}");
        }
    }
}

/// `moons.md §2.2`: the hour marker uses user-interface colour slot 5 (EGA
/// value 14) and both moons slot 6 (EGA value 7); the marker byte is `*`.
#[test]
fn g5_moons_s2_2_strip_colours_are_ui_slots_five_and_six() {
    assert_eq!(ui_colour_slot(5, true), 14);
    assert_eq!(ui_colour_slot(6, true), 7);
    assert_eq!(SKY_STRIP_HOUR_MARKER_INDEX, ui_colour_slot(5, true));
    assert_eq!(SKY_STRIP_MOON_INDEX, ui_colour_slot(6, true));
    assert_eq!(SKY_STRIP_HOUR_MARKER_RUNE, b'*');
    assert_eq!(SKY_STRIP_ROW, 0);
}

// ---------------------------------------------------------------------------
// systems/active-objects.md
// ---------------------------------------------------------------------------

/// `active-objects.md §8` (issue #305 Q4): "Orthogonal means exactly one
/// cell apart on one axis and zero on the other ... Diagonal neighbours
/// never take the adjacency reaction".
#[test]
fn g5_active_objects_s8_adjacency_is_orthogonal_only() {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        assert!(outdoor_offsets_are_orthogonally_adjacent(dx, dy), "({dx},{dy})");
    }
    for (dx, dy) in [(0, 0), (1, 1), (-1, 1), (1, -1), (-1, -1), (2, 0), (0, 2), (-2, 0), (2, 1)] {
        assert!(!outdoor_offsets_are_orthogonally_adjacent(dx, dy), "({dx},{dy})");
    }
}

/// `active-objects.md §8`: frames `0x2C..0x2F` and exactly Bat `0x94`,
/// Daemon `0xD8`, Dragon `0xDC` and Mongbat `0xF0` bypass the terrain
/// chance gates; "Sibling animation frames in those monster classes are not
/// part of this bypass test."
#[test]
fn g5_active_objects_s8_chance_gate_bypass_is_exact() {
    for byte in 0u8..=255 {
        let expected = matches!(byte, 0x2C..=0x2F | 0x94 | 0xD8 | 0xDC | 0xF0);
        assert_eq!(type_bypasses_terrain_chance_gate(byte), expected, "type {byte:#04x}");
    }
}

/// `active-objects.md §8`: the whole `0x80..0xFF` actor band maps to
/// bestiary classes `16..47` as `(type - 0x40) / 4`, discarding the low two
/// frame bits; the pirate family takes class 1.
#[test]
fn g5_active_objects_s8_type_byte_to_combat_class() {
    for byte in 0x80u8..=0xFF {
        assert_eq!(outdoor_combat_class_id(byte), Some((byte - 0x40) / 4), "type {byte:#04x}");
    }
    assert_eq!(outdoor_combat_class_id(0x80), Some(16));
    assert_eq!(outdoor_combat_class_id(0xFF), Some(47));
    for byte in 0x2Cu8..=0x2F {
        assert_eq!(outdoor_combat_class_id(byte), Some(1));
    }
}

// ---------------------------------------------------------------------------
// systems/doors-and-z-transitions.md
// ---------------------------------------------------------------------------

/// `doors-and-z-transitions.md §2`/`§4`: Open on exact tile `0xAF` prints
/// "It's open!", changes nothing, spends no key and still commits the
/// action.
#[test]
fn g5_doors_s4_open_on_0xaf_reports_already_open_and_commits() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xAF;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    let keys = state.keys;

    state.open_facing();

    assert_eq!(state.message, "It's open!");
    assert_eq!(state.grid[32 + 2], 0xAF);
    assert_eq!(state.keys, keys);
    assert_eq!(state.turn, 1);
    assert_eq!(state.door_tracker, None);
}

/// `doors-and-z-transitions.md §4`: locked doors of both families and both
/// magic-locked forms answer "Locked!"; Open does not pick, changes no tile,
/// spends no key, and commits a turn.
#[test]
fn g5_doors_s4_open_refuses_every_locked_form() {
    for tile in [0xB9u8, 0xBB, 0x97, 0x98] {
        let mut grid = open_grid();
        grid[32 + 2] = tile;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        let keys = state.keys;

        state.open_facing();

        assert_eq!(state.message, "Locked!", "tile {tile:#04x}");
        assert_eq!(state.grid[32 + 2], tile);
        assert_eq!(state.keys, keys);
        assert_eq!(state.turn, 1);
    }
}

/// `doors-and-z-transitions.md §4`/`§5`: the windowed unlocked door `0xBA`
/// opens to cleared tile `0x44` with the tracker saving the previous tile
/// and a countdown of four.
#[test]
fn g5_doors_s4_s5_windowed_door_opens_with_tracker() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xBA;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    state.open_facing();

    assert_eq!(state.message, "Opened!");
    assert_eq!(state.grid[32 + 2], 0x44);
    let tracker = state.door_tracker.expect("tracker armed");
    assert_eq!(tracker.previous_tile, 0xBA);
    assert_eq!((tracker.x, tracker.y), (2, 1));
    assert_eq!(tracker.turns_remaining, 4);
}

/// `doors-and-z-transitions.md §3.1`: success chance is Dexterity over
/// thirty: Dexterity zero never picks, thirty or more always picks, over
/// the whole zero-based thirty-outcome die.
#[test]
fn g5_doors_s3_1_flat_roll_extremes() {
    for roll in JIMMY_DOOR_DIE_LOW..=JIMMY_DOOR_DIE_HIGH {
        assert!(!jimmy_door_succeeds(0, roll));
        assert!(jimmy_door_succeeds(30, roll));
        assert!(jimmy_door_succeeds(99, roll));
    }
    for dex in 0u8..=30 {
        let wins = (JIMMY_DOOR_DIE_LOW..=JIMMY_DOOR_DIE_HIGH)
            .filter(|roll| jimmy_door_succeeds(dex, *roll))
            .count();
        assert_eq!(wins, dex as usize, "dex {dex}");
    }
}

// ---------------------------------------------------------------------------
// systems/inventory.md
// ---------------------------------------------------------------------------

fn g5_ready_state() -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.party_strengths = vec![99];
    state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]];
    state
}

/// `inventory.md §5.2`: occupied helm, ring and amulet slots each answer
/// their own refusal, framed by two line feeds, and change nothing.
#[test]
fn g5_inventory_s5_2_occupied_slot_refusals() {
    // (worn item, slot, candidate, refusal)
    let cases = [
        (0u8, EQUIP_SLOT_HELM, 2usize, "Remove first thy present helm!"),
        (43, EQUIP_SLOT_RING, 42, "Only one magic ring may be worn at a time!"),
        (46, EQUIP_SLOT_AMULET, 47, "Thou must remove thine other amulet!"),
    ];
    for (worn, slot, candidate, refusal) in cases {
        let mut state = g5_ready_state();
        state.party_equipment[0][slot] = worn;
        state.equipment_stock[candidate] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix(&format!("1/{candidate}")),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, format!("\n\n{refusal}"));
        assert_eq!(state.party_equipment[0][slot], worn);
        assert_eq!(state.equipment_stock[candidate], 1);
    }
}

/// `inventory.md §5.2`: "Two-handed item selected while either hand is
/// occupied" answers the both-hands refusal - weapon hand or off hand.
#[test]
fn g5_inventory_s5_2_two_hander_needs_both_hands_free() {
    // 33 is the Two-handed Sword; 18 a Club; 4 a Small Shield.
    for (slot, held) in [(EQUIP_SLOT_WEAPON, 18u8), (EQUIP_SLOT_OFFHAND, 4u8)] {
        let mut state = g5_ready_state();
        state.party_equipment[0][slot] = held;
        state.equipment_stock[33] = 1;
        assert_eq!(state.ready_equipment_from_suffix("1/33"), MoveOutcome::Blocked);
        assert_eq!(
            state.message,
            "\n\nBoth hands must be free before thou canst wield that!"
        );
        assert_eq!(state.equipment_stock[33], 1);
        assert_eq!(state.party_equipment[0][slot], held);
    }
}

/// `inventory.md §5.2`: "an occupied-slot or hand-conflict message precedes
/// the strength refusal when both would apply."
#[test]
fn g5_inventory_s5_2_occupied_slot_precedes_strength_refusal() {
    let mut state = g5_ready_state();
    state.party_strengths = vec![0];
    state.party_equipment[0][EQUIP_SLOT_HELM] = 0;
    state.equipment_stock[3] = 1;
    assert_eq!(state.ready_equipment_from_suffix("1/3"), MoveOutcome::Blocked);
    assert_eq!(state.message, "\n\nRemove first thy present helm!");

    // With the helm slot empty the same pick reaches the strength gate.
    let mut state = g5_ready_state();
    state.party_strengths = vec![0];
    state.equipment_stock[3] = 1;
    assert_eq!(state.ready_equipment_from_suffix("1/3"), MoveOutcome::Blocked);
    assert_eq!(state.message, "\n\nThou art not strong enough!");
}

/// `inventory.md §6`: an accepted equip writes the slot and decrements the
/// carried counter, prints nothing; an unequip clears the slot and returns
/// one copy while the counter is below 99, but not at 99.
#[test]
fn g5_inventory_s6_equip_and_unequip_are_mass_conserving_up_to_99() {
    let mut state = g5_ready_state();
    state.equipment_stock[1] = 2;
    assert_eq!(state.ready_equipment_from_suffix("1/1"), MoveOutcome::Used);
    assert_eq!(state.party_equipment[0][EQUIP_SLOT_HELM], 1);
    assert_eq!(state.equipment_stock[1], 1);
    assert!(state.message.is_empty());

    assert_eq!(state.ready_equipment_from_suffix("1/1"), MoveOutcome::Used);
    assert_eq!(state.party_equipment[0][EQUIP_SLOT_HELM], EQUIPMENT_EMPTY);
    assert_eq!(state.equipment_stock[1], 2);

    let mut state = g5_ready_state();
    state.party_equipment[0][EQUIP_SLOT_HELM] = 1;
    state.equipment_stock[1] = 99;
    assert_eq!(state.ready_equipment_from_suffix("1/1"), MoveOutcome::Used);
    assert_eq!(state.party_equipment[0][EQUIP_SLOT_HELM], EQUIPMENT_EMPTY);
    assert_eq!(state.equipment_stock[1], 99);
}

/// `inventory.md §7.1`: Moonstone burial accepts only scene `0x00..0x20`
/// and the explicit tile ids `4..10`, `44` and `45`.
#[test]
fn g5_inventory_s7_1_moonstone_burial_tile_and_scene_sets() {
    for tile in 0u8..=255 {
        let expected = matches!(tile, 4..=10 | 44 | 45);
        assert_eq!(moonstone_bury_tile_allowed(tile), expected, "tile {tile}");
    }
    for scene in 0u8..=255 {
        assert_eq!(moonstone_bury_scene_allowed(scene), scene <= 0x20, "scene {scene:#04x}");
    }
}

/// `inventory.md §4.7`: the Spells page labels, verbatim, by spell id, no
/// label longer than ten cells.
#[test]
fn g5_inventory_s4_7_spell_page_labels() {
    let published = [
        "In Lor", "Grav Por", "An Zu", "An Nox", "Mani", "An Ylem", "An Sanct", "An Xen Cor",
        "Rel Hur", "In Wis", "Kal Xen", "In Xen Man", "Vas Lor", "Vas Flam", "In Flam Gr",
        "In Nox Gr", "In Zu Grav", "In Por", "An Grav", "In Sanct", "In Sanct G", "Uus Por",
        "Des Por", "Wis Quas", "In Bet Xen", "An Ex Por", "In Ex Por", "Vas Mani", "In Zu",
        "Rel Tym", "In Vas P Y", "Quas An Wi", "In An", "Wis An Yle", "An Xen Ex", "Rel Xen Be",
        "Sanct Lo", "Xen Corp", "In Quas Xe", "In Quas Wi", "In Nox Hur", "In Quas Co",
        "In Mani Co", "Kal Xen Co", "In Vas G C", "In Flam Hu", "Vas Rel Po", "An Tym",
    ];
    assert_eq!(Z_STATS_SPELL_LABELS, published);
    assert!(Z_STATS_SPELL_LABELS.iter().all(|label| label.len() <= 10));
}
