// Closed-audit group g7: rules in `systems/magic.md`, `systems/visibility.md`,
// `systems/text-output.md`, `systems/view.md`, `catalogs/quest-graph.md`,
// `formats/signs-dat.md`, `systems/main-loop.md`, `formats/question-dat.md`
// and `catalogs/sage-rumours.md` that the engine implements but no earlier test
// asserted directly. Each test names its file and section. Text in these tests
// is either a literal the spec publishes or an invented placeholder.

// ---------------------------------------------------------------------------
// formats/question-dat.md
// ---------------------------------------------------------------------------

fn g7_question_bytes(records: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for record in records {
        bytes.extend_from_slice(record);
        bytes.push(0);
    }
    bytes
}

fn g7_thirty_question_records() -> Vec<Vec<u8>> {
    (0..30).map(|index| format!("rec{index}").into_bytes()).collect()
}

/// `formats/question-dat.md §7`: "A malformed record without a NUL terminator
/// should be rejected rather than allowing the renderer to read into following
/// records."
#[test]
fn g7_question_dat_s7_unterminated_final_record_is_rejected() {
    let records = g7_thirty_question_records();
    let refs: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    let mut bytes = g7_question_bytes(&refs);
    assert!(parse_question_records(&bytes).is_ok());
    // Drop the last terminator.
    bytes.pop();
    assert!(parse_question_records(&bytes).is_err());
}

/// `formats/question-dat.md §5`: a loader must preserve "Thirty nonempty
/// records"; §7: records two through twenty-nine must exist for every pairing,
/// and a missing dilemma record must not silently shift to another pairing.
#[test]
fn g7_question_dat_s5_empty_required_record_is_rejected() {
    let mut records = g7_thirty_question_records();
    records[17].clear();
    let refs: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    assert!(parse_question_records(&g7_question_bytes(&refs)).is_err());
}

/// `formats/question-dat.md §2`: "A reader can enumerate records by scanning
/// from the beginning of the file, splitting at NUL bytes, and ignoring any
/// empty trailing records if future data adds padding."
#[test]
fn g7_question_dat_s2_trailing_empty_padding_is_ignored() {
    let records = g7_thirty_question_records();
    let refs: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    let mut bytes = g7_question_bytes(&refs);
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    let parsed = parse_question_records(&bytes).expect("zero padding is accepted");
    assert_eq!(parsed.records.len(), 30);
    assert_eq!(parsed.dilemma(29), Some("rec29"));
}

/// `formats/question-dat.md §3`: "Line-feed bytes, if present, should be
/// treated as hard line breaks by the renderer" - so the loader keeps them.
#[test]
fn g7_question_dat_s3_line_feed_survives_loading() {
    let mut records = g7_thirty_question_records();
    records[0] = b"first\nsecond".to_vec();
    let refs: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    let parsed = parse_question_records(&g7_question_bytes(&refs)).unwrap();
    assert_eq!(parsed.gypsy_arrival(), Some("first\nsecond"));
}

/// `formats/question-dat.md §9`: "high-bit bytes do not occur in the shipped
/// file. Tooling should reject or explicitly flag those bytes in strict
/// compatibility mode rather than assigning them new runtime meaning."
#[test]
fn g7_question_dat_s9_high_bit_byte_is_rejected() {
    let mut records = g7_thirty_question_records();
    records[5] = vec![b'a', 0xC1, b'b'];
    let refs: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    assert!(parse_question_records(&g7_question_bytes(&refs)).is_err());
}

/// `formats/question-dat.md §4`: the full twenty-eight-row pair table, in the
/// canonical virtue order (Honesty .. Humility = 0 .. 7).
#[test]
fn g7_question_dat_s4_every_pair_resolves_to_its_published_record() {
    let mut expected = 2usize;
    for lo in 0..8usize {
        for hi in (lo + 1)..8usize {
            assert_eq!(
                question_dat_dilemma_record_for_pair(lo, hi),
                Some(expected),
                "pair ({lo}, {hi})"
            );
            expected += 1;
        }
        // The diagonal never has a record.
        assert_eq!(question_dat_dilemma_record_for_pair(lo, lo), None);
    }
    assert_eq!(expected, 30);
    // Spot rows quoted from the table.
    assert_eq!(question_dat_dilemma_record_for_pair(2, 4), Some(16)); // Valor / Sacrifice
    assert_eq!(question_dat_dilemma_record_for_pair(3, 7), Some(23)); // Justice / Humility
    assert_eq!(question_dat_dilemma_record_for_pair(5, 7), Some(28)); // Honor / Humility
}

// ---------------------------------------------------------------------------
// catalogs/sage-rumours.md
// ---------------------------------------------------------------------------

/// `catalogs/sage-rumours.md §2`: the per-tavern menu state and sage letter,
/// for all nine taverns.
#[test]
fn g7_sage_rumours_s2_every_tavern_selects_its_published_sage_letter() {
    for (tavern, state, letter) in [
        (Tavern::TheHonestMeal, 0, 'C'),
        (Tavern::TheWayfarerTavern, 0, 'C'),
        (Tavern::TheSwordAndKeg, 0, 'C'),
        (Tavern::TheSlaughteredLamb, 2, 'H'),
        (Tavern::TheHumblePalate, 3, 'A'),
        (Tavern::TheBlueBoarTavern, 1, 'T'),
        (Tavern::TheCatsLair, 0, 'C'),
        (Tavern::TheFallenVirgin, 2, 'H'),
        (Tavern::TheFolleyTap, 0, 'C'),
    ] {
        assert_eq!(tavern_menu_state_index(tavern), state, "{tavern:?}");
        assert_eq!(tavern_lore_menu_letter(tavern), letter, "{tavern:?}");
        assert_eq!(
            crate::shop_runtime::tavern_sage_lore_menu_letter(tavern),
            letter,
            "{tavern:?}"
        );
    }
}

/// `catalogs/sage-rumours.md §1/§4`: every row's four-letter key matches its
/// own row, case-insensitively, alone or followed by a space.
#[test]
fn g7_sage_rumours_s4_every_keyword_resolves_to_its_own_row() {
    for (row, entry) in SAGE_RUMOUR_TABLE.iter().enumerate() {
        for typed in [
            entry.keyword.to_string(),
            entry.keyword.to_ascii_uppercase(),
            format!("{} more", entry.keyword),
        ] {
            let quote = find_sage_topic(&SAGE_RUMOUR_TABLE, &typed)
                .unwrap_or_else(|err| panic!("row {row} input {typed:?}: {err:?}"));
            assert_eq!(quote.entry, *entry, "row {row} input {typed:?}");
        }
    }
}

/// `catalogs/sage-rumours.md §1`: "longer words that merely start with the
/// four-letter key do not match."
#[test]
fn g7_sage_rumours_s1_longer_word_never_matches_any_row() {
    for entry in SAGE_RUMOUR_TABLE.iter() {
        let typed = format!("{}q", entry.keyword);
        assert_eq!(
            find_sage_topic(&SAGE_RUMOUR_TABLE, &typed).map(|quote| quote.entry),
            Err(SageRumourError::NoTopicMatch),
            "{typed}"
        );
    }
}

/// `catalogs/sage-rumours.md §3`: the four shared paid-success records are
/// 85..88; 84 is the fee quote and 91 the refusal, neither a success record.
#[test]
fn g7_sage_rumours_s3_success_records_are_exactly_85_to_88() {
    for record in 0..120usize {
        assert_eq!(
            sage_rumour_success_record_id_accepted(record),
            (85..=88).contains(&record),
            "record {record}"
        );
    }
}

/// `catalogs/sage-rumours.md §1/§3`: the row subject fills `&` and the row
/// location fills `*`, whichever of the four templates is drawn.
#[test]
fn g7_sage_rumours_s3_templates_fill_subject_and_location() {
    let entry = SAGE_RUMOUR_TABLE[12];
    assert_eq!((entry.subject, entry.destination), ("the mother of Rew", "Minoc"));
    for record in 85..=88usize {
        let rendered = render_sage_rumour(entry, record);
        assert!(rendered.contains("the mother of Rew"), "{record}: {rendered}");
        assert!(rendered.contains("Minoc"), "{record}: {rendered}");
        assert!(!rendered.contains('&') && !rendered.contains('*'), "{record}");
    }
    assert_eq!(
        render_sage_rumour(SAGE_RUMOUR_TABLE[0], 85),
        "Seek ye Malik in Moonglow!"
    );
}

// ---------------------------------------------------------------------------
// formats/signs-dat.md
// ---------------------------------------------------------------------------

/// `formats/signs-dat.md §3`: "The only end test is a 255 byte read where the
/// next header would start, right after a record's terminating zero."
#[test]
fn g7_signs_dat_s3_ff_at_a_header_position_ends_the_scan() {
    let mut bytes = rc14_signs_bytes(&[(2, &[(0, 1, 1, b"First")])]);
    bytes.push(0xFF);
    bytes.extend_from_slice(&[2, 0, 3, 3]);
    bytes.extend_from_slice(b"Beyond\0");
    assert_eq!(
        find_sign_in_window(&bytes, 2, 0, 1, 1).map(|r| r.body).as_deref(),
        Some("First")
    );
    assert_eq!(find_sign_in_window(&bytes, 2, 0, 3, 3), None);
}

/// `formats/signs-dat.md §4` step 2: "The control tests then compare the raw
/// byte, high bit included" - a high-bit separator, macro or pause byte is an
/// ordinary character.
#[test]
fn g7_signs_dat_s4_high_bit_control_values_are_ordinary_characters() {
    assert_eq!(sign_body_byte_kind(0x26), SignBodyByteKind::SeparatorGlyph);
    assert_eq!(sign_body_byte_kind(0xA6), SignBodyByteKind::Character(0x26));
    assert_eq!(sign_body_byte_kind(0xA7), SignBodyByteKind::Character(0x27));
    for raw in 0x29u8..=0x31 {
        assert_eq!(sign_body_byte_kind(raw), SignBodyByteKind::Macro(raw));
        assert_eq!(
            sign_body_byte_kind(raw | 0x80),
            SignBodyByteKind::Character(raw),
            "{raw:#04x} with the high bit"
        );
    }
    assert_eq!(sign_body_byte_kind(0x0D), SignBodyByteKind::PauseForKey);
    assert_eq!(sign_body_byte_kind(0x8D), SignBodyByteKind::Character(0x0D));
    assert_eq!(sign_body_byte_kind(0x00), SignBodyByteKind::EndOfRecord);
}

/// `formats/signs-dat.md §4` step 3: "`0x8A` is a line feed".
#[test]
fn g7_signs_dat_s4_high_bit_line_feed_breaks_the_row() {
    let rendered = sign_payload_rendered(&[b'A', 0x8A, b'B']);
    assert_eq!(rendered.text, "A\nB");
    assert_eq!(decode_sign_payload(&[b'A', 0x8A, b'B']), "A\nB");
}

/// `formats/signs-dat.md §6`: "a fixed six-row framed placard ... Every row is
/// sixteen cells".
#[test]
fn g7_signs_dat_s6_placard_is_six_sixteen_cell_rows() {
    assert_eq!(SIGN_NO_RECORD_PLACARD_ROWS.len(), 6);
    for row in SIGN_NO_RECORD_PLACARD_ROWS {
        assert_eq!(row.len(), 16, "{row:?}");
    }
    assert_eq!(SIGN_NO_RECORD_PLACARD_ROWS[2], "g  LIVE@BY@[E  g");
    assert_eq!(SIGN_NO_RECORD_PLACARD_ROWS[3], "g  EIGHT@LAWS  g");
}

/// `formats/signs-dat.md §6`: the poster's three name rows "use current party
/// slots `0..2` in order and are centered by starting each name at column
/// `7 - floor(name_length / 2)` before emitting the right border at column
/// `14`"; `view.md §3`: "Slots `3+` are not listed, and status does not filter
/// a listed slot."
#[test]
fn g7_signs_dat_s6_wanted_poster_centres_three_slots_and_ignores_the_fourth() {
    let mut grid = open_grid();
    grid[21 * 32 + 17] = 0xa0;
    let mut state = test_state(grid, 16, 21);
    state.area = Area::Town {
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.party_names = vec![
        *b"ABCDEFGH\0",
        *b"XYZ\0\0\0\0\0\0",
        *b"MNOPQRS\0\0",
        *b"ZZZZ\0\0\0\0\0",
    ];
    for member in [0usize, 1, 2, 3] {
        if state.party.len() <= member {
            let mut extra = state.party[0];
            extra.slot = member as u8;
            state.party.push(extra);
        }
    }
    state.party[1].status = b'D';

    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    let rows: Vec<&str> = state.message.split('\n').collect();
    let wanted = rows
        .iter()
        .position(|row| *row == "g   Wanted:   g")
        .expect("the poster heading row");
    // Heading, spacer, then the three name rows.
    assert_eq!(rows[wanted + 1], "g             g");
    assert_eq!(rows[wanted + 2], "g  ABCDEFGH   g");
    assert_eq!(rows[wanted + 3], "g     XYZ     g");
    assert_eq!(rows[wanted + 4], "g   MNOPQRS   g");
    assert_eq!(rows[wanted + 5], "g             g");
    assert!(!state.message.contains("ZZZZ"));
}

// ---------------------------------------------------------------------------
// systems/main-loop.md
// ---------------------------------------------------------------------------

fn g7_member(status: u8) -> PartyMember {
    PartyMember {
        slot: 0,
        class_byte: b'F',
        status,
        climb_stat: 10,
        mana: 0,
        hp: 10,
        max_hp: 10,
        level: 1,
    }
}

/// `systems/main-loop.md §6`: "A member counts as able to act when their
/// status is Good or Poisoned ... the check also records which member that
/// was"; sleepers only matter when nobody can act.
#[test]
fn g7_main_loop_s6_capability_records_the_first_able_member() {
    let party = [g7_member(b'S'), g7_member(b'D'), g7_member(b'P'), g7_member(b'G')];
    assert_eq!(
        party_capability(&party),
        PartyCapability::CanAct { member_index: 2 }
    );
    let sleepers = [g7_member(b'D'), g7_member(b'S')];
    assert_eq!(party_capability(&sleepers), PartyCapability::Sleeping);
    let none = [g7_member(b'D'), g7_member(b'D')];
    assert_eq!(party_capability(&none), PartyCapability::Defeated);
}

/// `systems/main-loop.md §3`: "stock scenes `33..40` map to the eight named
/// dungeons; subtract `33` for the zero-based `DUNGEON.DAT` record index."
#[test]
fn g7_main_loop_s3_named_dungeon_record_index_is_scene_minus_33() {
    for scene in 0u8..=255 {
        let expected = (33..=40).contains(&scene).then(|| scene - 33);
        assert_eq!(dungeon_record_index(scene), expected, "scene {scene}");
    }
}

/// `systems/main-loop.md §6` step 4: "two minutes for overworld, one minute
/// elsewhere"; combat does not advance the clock per action (§6).
#[test]
fn g7_main_loop_s6_minute_increment_by_scene_byte() {
    for scene in 0u8..=255 {
        let expected = match scene_route(scene) {
            SceneRoute::Overworld => Some(2),
            SceneRoute::TownFamily | SceneRoute::Dungeon => Some(1),
            _ => None,
        };
        assert_eq!(mode_minute_increment(scene_route(scene)), expected);
    }
    assert_eq!(mode_minute_increment(scene_route(0)), Some(2));
    assert_eq!(mode_minute_increment(scene_route(17)), Some(1));
    assert_eq!(mode_minute_increment(scene_route(40)), Some(1));
    assert_eq!(mode_minute_increment(scene_route(0xFF)), None);
}

/// `systems/main-loop.md §8` step 2: "days through a 28-day month and a
/// 13-month year".
#[test]
fn g7_main_loop_s8_cleanup_cascades_minutes_through_a_13_month_year() {
    let mut clock = GameClock::with_date(139, 13, 28, 23, 59).unwrap();
    clock.advance_minutes(2);
    assert_eq!(clock, GameClock::with_date(140, 1, 1, 0, 1).unwrap());
    let mut clock = GameClock::with_date(139, 4, 28, 22, 30).unwrap();
    clock.advance_minutes(120);
    assert_eq!(clock, GameClock::with_date(139, 5, 1, 0, 30).unwrap());
}

// ---------------------------------------------------------------------------
// systems/text-output.md
// ---------------------------------------------------------------------------

fn g7_row(system: &TextWindowSystem, y: u8, x1: u8, x2: u8) -> String {
    (x1..=x2)
        .map(|x| char::from(system.cell(x, y).map(|cell| cell.byte).unwrap_or(b'.')))
        .collect()
}

/// `systems/text-output.md §9` step 2: every window boots to the full 40x25
/// rectangle, cursor `(0,0)`, foreground 15 on background 0, flags clear,
/// and window 0 is active.
#[test]
fn g7_text_output_s9_boot_descriptor_defaults() {
    let system = TextWindowSystem::new();
    assert_eq!(system.active_window_index(), 0);
    for index in 0..4 {
        let window = system.window(index).unwrap();
        assert_eq!(
            (window.top_left_x, window.top_left_y, window.bottom_right_x, window.bottom_right_y),
            (0, 0, 39, 24)
        );
        assert_eq!((window.cursor_x, window.cursor_y), (0, 0));
        assert_eq!(text_color_foreground(window.color), 15);
        assert_eq!(text_color_background(window.color), 0);
        assert_eq!(window.flags, 0);
    }
    assert_eq!(system.window(4), None);
}

/// `systems/text-output.md §9`: the rectangle setter "clamps each X to 0-39
/// and each Y to 0-24, then swaps the X pair if the supplied left exceeds the
/// supplied right and swaps the Y pair if the supplied top exceeds the
/// supplied bottom", leaving cursor, colour and flags untouched; "An
/// out-of-range window index is a silent no-op."
#[test]
fn g7_text_output_s9_rect_setter_clamps_swaps_and_keeps_other_state() {
    let mut system = TextWindowSystem::new();
    system.set_active_window(1);
    system.set_active_cursor(3, 2);
    system.set_active_color(0x1E);
    system.set_active_flags(TEXT_WINDOW_FLAG_INVERSE);

    system.set_window_rect(1, 60, 30, 10, 5);
    let window = system.window(1).unwrap();
    assert_eq!(
        (window.top_left_x, window.top_left_y, window.bottom_right_x, window.bottom_right_y),
        (10, 5, 39, 24)
    );
    assert_eq!((window.cursor_x, window.cursor_y), (3, 2));
    assert_eq!(window.color, 0x1E);
    assert_eq!(window.flags, TEXT_WINDOW_FLAG_INVERSE);

    let before: Vec<_> = (0..4).map(|index| system.window(index)).collect();
    system.set_window_rect(4, 1, 1, 2, 2);
    system.set_window_rect(200, 1, 1, 2, 2);
    let after: Vec<_> = (0..4).map(|index| system.window(index)).collect();
    assert_eq!(before, after);
}

/// `systems/text-output.md §5`: "A carriage-return byte returns the cursor to
/// the window's left edge without changing the row and without emitting a
/// glyph."
#[test]
fn g7_text_output_s5_carriage_return_moves_the_column_only() {
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 4, 4, 20, 10);
    system.set_active_cursor(5, 3);
    system.emit_byte(b'\r');
    assert_eq!(system.active_cursor(), (0, 3));
    assert!((4..=20).all(|x| system.cell(x, 7).is_none()));
}

/// `systems/text-output.md §3/§5`: `0xFB` clears centre-output, and the style
/// controls "do not emit glyph pixels and do not advance the cursor"; other
/// high-bit bytes outside the confirmed range have no glyph meaning.
#[test]
fn g7_text_output_s5_controls_and_other_high_bytes_leave_the_cursor() {
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 0, 0, 15, 3);
    system.emit_byte(TEXT_CTRL_CENTRE_ON);
    system.emit_byte(TEXT_CTRL_UNDERLINE_TOGGLE);
    assert!(system.active_window().centre_enabled());
    system.emit_byte(TEXT_CTRL_CENTRE_OFF);
    assert!(!system.active_window().centre_enabled());
    assert!(system.active_window().underline_enabled(), "0xFB touches centre only");
    for byte in [TEXT_CTRL_CENTRE_ON, TEXT_CTRL_CENTRE_OFF, 0x80, 0xA0, 0xFA] {
        system.emit_byte(byte);
        assert_eq!(system.active_cursor(), (0, 0), "{byte:#04x}");
    }
    assert!((0..=15).all(|x| system.cell(x, 0).is_none()));
}

/// `systems/text-output.md §5`: "An empty string is a no-op."
#[test]
fn g7_text_output_s5_empty_string_is_a_no_op() {
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 0, 0, 15, 3);
    system.set_active_cursor(4, 1);
    system.print_wrapped_string("");
    assert_eq!(system.active_cursor(), (4, 1));
}

/// `systems/text-output.md §5`: the padded numeric printer is signed 16-bit
/// and "The field width is clamped at 39".
#[test]
fn g7_text_output_s5_numeric_printer_is_signed_and_clamps_width() {
    assert_eq!(format_signed_number(i16::MIN, 0, ' '), "-32768");
    assert_eq!(format_signed_number(0x7FFF, 7, '0'), "0032767");
    assert_eq!(format_signed_number((-2i32) as i16, 3, ' '), " -2");

    let mut system = TextWindowSystem::new();
    system.print_number(5, 60, b'.');
    // Thirty-nine cells: thirty-eight pad bytes and the digit.
    assert_eq!(g7_row(&system, 0, 0, 39), format!("{}5.", ".".repeat(38)));
    assert_eq!(system.cell(38, 0).unwrap().byte, b'5');
    assert_eq!(system.cell(39, 0), None);
}

/// `systems/text-output.md §4`: "A window whose corners are columns 6 and 33
/// therefore holds **twenty-eight** characters on one row, and the
/// twenty-ninth is what forces the wrap."
#[test]
fn g7_text_output_s4_six_to_thirty_three_holds_twenty_eight() {
    assert_eq!(crate::text_wrap::text_window_capacity(6, 33), 28);
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 6, 12, 33, 13);
    for _ in 0..28 {
        system.emit_byte(b'x');
    }
    assert_eq!(system.cell(33, 12).unwrap().byte, b'x');
    assert_eq!(system.active_cursor(), (0, 1));
    system.emit_byte(b'y');
    assert_eq!(system.cell(6, 13).unwrap().byte, b'y');
}

/// `systems/text-output.md §6`: "a row-filling string that carries its own
/// trailing line feed leaves no blank row, while the same text printed and
/// then followed by a separate line-feed emit does leave one."
#[test]
fn g7_text_output_s6_own_trailing_feed_after_a_full_row_is_suppressed() {
    let mut own = TextWindowSystem::new();
    own.set_window_rect(0, 0, 0, 3, 5);
    own.print_wrapped_string("abcd\n");
    assert_eq!(own.active_cursor(), (0, 1));

    let mut separate = TextWindowSystem::new();
    separate.set_window_rect(0, 0, 0, 3, 5);
    separate.print_wrapped_string("abcd");
    separate.emit_byte(b'\n');
    assert_eq!(separate.active_cursor(), (0, 2));
}

/// `systems/text-output.md §5/§6`: "Line-feed and carriage-return bytes
/// embedded in the source string force an immediate flush at that point and
/// pass through to the per-cell emitter so the cursor moves accordingly." A
/// short line ending in its own feed therefore leaves the cursor at column 0
/// of the next row.
#[test]
fn g7_text_output_s6_trailing_feed_on_a_short_line_reaches_the_emitter() {
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 0, 0, 15, 5);
    system.print_wrapped_string("ab\n");
    assert_eq!(system.active_cursor(), (0, 1));
}

/// `systems/text-output.md §10.4`: "A line feed moves the cursor; it never
/// erases."
#[test]
fn g7_text_output_s10_4_line_feed_never_erases() {
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 0, 0, 7, 3);
    system.set_active_cursor(0, 1);
    for byte in b"old" {
        system.emit_byte(*byte);
    }
    system.set_active_cursor(0, 0);
    system.emit_byte(b'\n');
    system.emit_byte(b'\n');
    assert_eq!(system.active_cursor(), (0, 2));
    assert_eq!(g7_row(&system, 1, 0, 2), "old");
}

/// `systems/text-output.md §10.4`: "A string that is only line feeds spends
/// one row per feed."
#[test]
fn g7_text_output_s10_4_feed_only_string_spends_one_row_per_feed() {
    let mut system = TextWindowSystem::new();
    system.set_window_rect(0, 0, 0, 15, 8);
    system.set_active_cursor(5, 1);
    system.print_wrapped_string("\n\n");
    assert_eq!(system.active_cursor(), (0, 3));
}

/// `systems/text-output.md §10.5`: "The emitter's overflow tail ... reads and
/// writes no other descriptor ... that window's descriptor was bit-identical
/// across the overflow."
#[test]
fn g7_text_output_s10_5_overflow_moves_only_the_active_cursor() {
    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    let message_before = system.window(MESSAGE_TEXT_WINDOW_INDEX).unwrap();
    system.set_active_window(STATS_PANEL_TEXT_WINDOW_INDEX);
    let stats = system.active_window();
    system.set_active_cursor(0, stats.bottom_right_y - stats.top_left_y);
    system.emit_byte(b'\n');
    assert_eq!(
        system.active_cursor().1,
        stats.bottom_right_y - stats.top_left_y,
        "the active cursor stays clamped to its bottom row"
    );
    assert_eq!(system.window(MESSAGE_TEXT_WINDOW_INDEX).unwrap(), message_before);
}

/// `systems/text-output.md §10.7` step 1: the stats-panel label's "opening cap
/// in column `30 - (L / 2)`", then the caption, then the closing cap.
#[test]
fn g7_text_output_s10_7_stats_label_cap_formula() {
    for length in 1usize..=12 {
        let (open, first, close) = stats_label_strip_span(length);
        assert_eq!(usize::from(open), 30 - length / 2, "L={length}");
        assert_eq!(first, open + 1);
        assert_eq!(usize::from(close), usize::from(first) + length);
        if length % 2 == 1 {
            // An odd-length caption is centred on column 31.
            assert_eq!(usize::from(first) + length / 2, 31, "L={length}");
        }
    }
}

/// `systems/text-output.md §10.6`: the input cursor "cycles through four
/// consecutive fixed-cell glyph codes, `0x05` through `0x08`".
#[test]
fn g7_text_output_s10_6_input_cursor_glyph_band() {
    assert_eq!(CURSOR_BLINK_BASE_GLYPH, 0x05);
    assert_eq!(CURSOR_BLINK_CYCLE_LENGTH, 4);
    assert_eq!(
        CURSOR_BLINK_BASE_GLYPH + (CURSOR_BLINK_CYCLE_LENGTH as u8 - 1),
        0x08
    );
}

fn g7_uniform_widths(width: u8) -> ProportionalWidthTable {
    let mut widths = [0u8; PROPORTIONAL_WIDTH_TABLE_LEN];
    for code in (PCS_FIRST_CODE + 1)..=b'z' {
        widths[usize::from(code)] = width;
    }
    widths[usize::from(b' ')] = 0;
    ProportionalWidthTable::new(widths)
}

fn g7_line_text(placed: &[PlacedProportionalGlyph], y: u16) -> String {
    placed
        .iter()
        .filter(|glyph| glyph.y == y)
        .map(|glyph| char::from(glyph.code))
        .collect()
}

/// `systems/text-output.md §8.3`: "a line is kept only while its total advance
/// is strictly less than the available width."
#[test]
fn g7_text_output_s8_3_right_edge_is_exclusive() {
    let widths = g7_uniform_widths(4);
    // `aa bb` measures 5+5+5+5+5 = 25; a third word follows so the line does
    // not end at NUL (a NUL-terminated last line ends cleanly).
    let fits = ProportionalLayoutDescriptor {
        right_a: 26,
        right_b: 26,
        ..ProportionalLayoutDescriptor::full_width(0, 0)
    };
    let placed = layout_proportional_paragraph_glyphs(&widths, &fits, b"aa bb cc\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
    assert_eq!(g7_line_text(&placed, 0), "aabb");

    let exact = ProportionalLayoutDescriptor {
        right_a: 25,
        right_b: 25,
        ..ProportionalLayoutDescriptor::full_width(0, 0)
    };
    let placed = layout_proportional_paragraph_glyphs(&widths, &exact, b"aa bb cc\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
    assert_eq!(g7_line_text(&placed, 0), "aa");
    // The paragraph's last line ends at NUL, which ends it cleanly.
    assert_eq!(g7_line_text(&placed, PROPORTIONAL_LINE_STRIDE), "bbcc");
}

/// `systems/text-output.md §8.3`: "a single space after a wrap is dropped, and
/// a run of two or more leaves the extras as leading spaces on the next line."
#[test]
fn g7_text_output_s8_3_only_one_break_byte_is_skipped_after_a_wrap() {
    let widths = g7_uniform_widths(4);
    let boxed = ProportionalLayoutDescriptor {
        right_a: 14,
        right_b: 14,
        ..ProportionalLayoutDescriptor::full_width(0, 0)
    };
    let single = layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
    let second: Vec<_> = single.iter().filter(|g| g.y == PROPORTIONAL_LINE_STRIDE).collect();
    assert_eq!(second[0].x, 0);

    let double = layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa  bb\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
    let second: Vec<_> = double.iter().filter(|g| g.y == PROPORTIONAL_LINE_STRIDE).collect();
    assert_eq!(second.first().map(|g| g.code), Some(b'b'));
    assert_eq!(
        second[0].x,
        u16::from(PROPORTIONAL_DEFAULT_SPACE_ADVANCE),
        "the extra space stays as leading space"
    );
}

/// `systems/text-output.md §8.2`: "Any other byte at or below `0x20` |
/// same as space | same as space", and §8.5: "The line advance is a fixed
/// **9** pixels"; §8.2: brace "a fixed **15** pixels"; §8.5 clip at **192**.
#[test]
fn g7_text_output_s8_2_low_control_bytes_measure_as_space() {
    assert_eq!(PROPORTIONAL_LINE_STRIDE, 9);
    assert_eq!(PROPORTIONAL_BRACE_INDENT, 15);
    assert_eq!(PROPORTIONAL_DRAW_CLIP_Y, 192);
    assert_eq!(PROPORTIONAL_DEFAULT_SPACE_ADVANCE, 5);
    let widths = g7_uniform_widths(4);
    let boxed = ProportionalLayoutDescriptor::full_width(0, 0);
    let with_space = layout_proportional_paragraph_glyphs(&widths, &boxed, b"a b\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
    let with_control = layout_proportional_paragraph_glyphs(&widths, &boxed, b"a\x01b\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
    assert_eq!(with_space, with_control);
    assert!(with_control.iter().all(|glyph| glyph.code != 0x01));
}

// ---------------------------------------------------------------------------
// systems/visibility.md
// ---------------------------------------------------------------------------

/// `systems/visibility.md §3`: the complete plateau table of cells inside the
/// gate, over open terrain where nothing blocks.
#[test]
fn g7_visibility_s3_threshold_plateau_table() {
    let state = britannia_state(open_world_grid(), 100, 100);
    let expected = |light: u32| -> usize {
        match light {
            0 => 0,
            1 => 5,
            2..=3 => 9,
            4 => 13,
            5..=7 => 21,
            8 => 25,
            9 => 29,
            10..=12 => 37,
            13..=15 => 45,
            16 => 49,
            17 => 57,
            18..=19 => 61,
            20..=24 => 69,
            25 => 81,
            26..=28 => 89,
            29..=31 => 97,
            32..=33 => 101,
            34..=40 => 109,
            41..=49 => 117,
            _ => 121,
        }
    };
    for light in 0u32..=60 {
        let carve = state.surface_visibility_produce(100, 100, 5, light as i32, true);
        assert_eq!(carve.len(), 121);
        assert_eq!(
            carve.iter().filter(|lit| **lit).count(),
            expected(light),
            "L = {light}"
        );
    }
}

/// `systems/visibility.md §4` Stage 2: zero light leaves "every cell = 0xFF,
/// the player's own cell included"; negative light populates "every cell ...
/// without any visibility carve, distance gate or blocker test".
#[test]
fn g7_visibility_s4_zero_blackout_and_negative_full_fill() {
    let mut grid = open_grid();
    for x in 0..32 {
        grid[14 * 32 + x] = 0x4D;
        grid[16 * 32 + x] = 0x4D;
    }
    for y in 0..32 {
        grid[y * 32 + 14] = 0x4D;
        grid[y * 32 + 16] = 0x4D;
    }
    let state = test_state(grid, 15, 15);
    let dark = state.surface_visibility_produce(15, 15, 5, 0, false);
    assert!(dark.iter().all(|lit| !*lit));
    let full = state.surface_visibility_produce(15, 15, 5, -1, false);
    assert_eq!(full.len(), 121);
    assert!(full.iter().all(|lit| *lit));
    let carved = state.surface_visibility_produce(15, 15, 5, 50, false);
    assert!(carved.iter().filter(|lit| **lit).count() < 121);
}

/// A town grid of walls (`0x4D`, a §6 blocker) with the listed cells opened to
/// floor and the listed overrides applied.
fn g7_walled_town(open: &[(usize, usize)], overrides: &[(usize, usize, u8)]) -> PlayState {
    let mut grid = vec![0x4Du8; 1024];
    for &(x, y) in open {
        grid[y * 32 + x] = 0x44;
    }
    for &(x, y, tile) in overrides {
        grid[y * 32 + x] = tile;
    }
    test_state(grid, 15, 15)
}

/// `systems/visibility.md §5/§6`: the five adjacent-only ids "propagate only
/// when they are orthogonally adjacent to the centre cell, which is the case
/// where the squared-distance helper returns `1`."
#[test]
fn g7_visibility_s6_adjacent_only_tiles_propagate_only_at_distance_one() {
    for tile in [0x4Au8, 0x4B, 0x98, 0xBA, 0xBB] {
        // Orthogonally adjacent: the cell beyond is reached.
        let state = g7_walled_town(&[(15, 15), (17, 15)], &[(16, 15, tile)]);
        assert!(
            state.town_cell_visible_with_light_threshold(15, 15, 17, 15, 5, 50),
            "{tile:#04x} at d2 = 1"
        );
        // Diagonal neighbour (d2 = 2): no propagation past it.
        let state = g7_walled_town(&[(15, 15), (17, 17)], &[(16, 16, tile)]);
        assert!(state.town_cell_visible_with_light_threshold(15, 15, 16, 16, 5, 50));
        assert!(
            !state.town_cell_visible_with_light_threshold(15, 15, 17, 17, 5, 50),
            "{tile:#04x} at d2 = 2"
        );
        // Two cells out on a row (d2 = 4): no propagation either.
        let state = g7_walled_town(&[(15, 15), (16, 15), (18, 15)], &[(17, 15, tile)]);
        assert!(state.town_cell_visible_with_light_threshold(15, 15, 17, 15, 5, 50));
        assert!(
            !state.town_cell_visible_with_light_threshold(15, 15, 18, 15, 5, 50),
            "{tile:#04x} at d2 = 4"
        );
    }
    // An ordinary floor on the diagonal does propagate.
    let state = g7_walled_town(&[(15, 15), (16, 16), (17, 17)], &[]);
    assert!(state.town_cell_visible_with_light_threshold(15, 15, 17, 17, 5, 50));
}

/// `systems/visibility.md §6`: membership of the adjacent-only set, over every
/// terrain id.
#[test]
fn g7_visibility_s6_adjacent_only_set_is_exactly_five_ids() {
    for tile in 0u8..=255 {
        assert_eq!(
            tile_propagates_sight_only_when_adjacent(tile),
            matches!(tile, 0x4A | 0x4B | 0x98 | 0xBA | 0xBB),
            "{tile:#04x}"
        );
        assert_eq!(
            tile_blocks_sight_propagation(tile),
            matches!(
                tile,
                0x09 | 0x0A | 0x0C | 0x0D | 0x4D..=0x4F | 0x5A | 0x97 | 0xB8 | 0xB9 | 0xBC
                    | 0xD0..=0xD3 | 0xF8 | 0xFE | 0xFF
            ),
            "{tile:#04x}"
        );
    }
}

/// `systems/visibility.md §7`: "the clear-marker core is therefore
/// **twenty-one cells**".
#[test]
fn g7_visibility_s7_clear_core_is_twenty_one_cells() {
    let mut core = 0;
    for row in 0u8..11 {
        for col in 0u8..11 {
            let dx = i32::from(col) - 5;
            let dy = i32::from(row) - 5;
            let d2 = (dx * dx + dy * dy) as u32;
            assert_eq!(fog_refine_squared_distance(col, row), d2, "({col},{row})");
            assert_eq!(fog_refine_inside_clear_core(col, row), d2 <= 5);
            if fog_refine_inside_clear_core(col, row) {
                core += 1;
            }
        }
    }
    assert_eq!(core, 21);
}

/// `systems/visibility.md §12.2`: one source's light "covers thirty-seven
/// cells: every offset with `|dx| <= 3` and `|dy| <= 3` **except** the twelve
/// corner-ish offsets".
#[test]
fn g7_visibility_s12_2_single_source_lights_a_37_cell_disc() {
    let mut grid = open_grid();
    grid[15 * 32 + 15] = 0xBD;
    let mut state = test_state(grid, 15, 15);
    state.rebuild_surface_local_light_mask();
    let mut lit = Vec::new();
    for y in 0..32usize {
        for x in 0..32usize {
            if state.local_light_mask[y * 32 + x] {
                lit.push((x as isize - 15, y as isize - 15));
            }
        }
    }
    assert_eq!(lit.len(), 37);
    for (dx, dy) in lit {
        assert!(dx * dx + dy * dy <= 10, "({dx},{dy})");
    }
    assert_eq!(LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD, 10);
    assert_eq!(LOCAL_LIGHT_SOURCE_CELL_COUNT, 37);
}

/// `systems/visibility.md §12.3`: the light-source set is exactly `0xB0..0xB3`,
/// `0xBC..0xBF`, `0xDC` and `0xDE`.
#[test]
fn g7_visibility_s12_3_local_light_source_set_is_exact() {
    for tile in 0u8..=255 {
        assert_eq!(
            is_local_light_source_tile(tile),
            matches!(tile, 0xB0..=0xB3 | 0xBC..=0xBF | 0xDC | 0xDE),
            "{tile:#04x}"
        );
    }
}

/// `systems/visibility.md §2`: the grid's active region is 11x11 with a
/// 32-byte row stride, and the terrain band is 11 rows of 16 bytes.
#[test]
fn g7_visibility_s2_grid_and_band_geometry() {
    assert_eq!(VIEWPORT_SIDE, 11);
    assert_eq!(VIEWPORT_ROW_STRIDE, 32);
    assert_eq!(TERRAIN_BAND_ROW_STRIDE, 16);
    assert_eq!(visibility_grid_active_index(5, 5), Some(5 * 32 + 5));
    assert_eq!(visibility_grid_active_index(10, 10), Some(10 * 32 + 10));
    assert_eq!(visibility_grid_active_index(0, 11), None);
    assert_eq!(terrain_band_active_index(10, 10), Some(10 * 16 + 10));
    assert_eq!(terrain_band_active_index(11, 0), None);
}

// ---------------------------------------------------------------------------
// systems/view.md
// ---------------------------------------------------------------------------

/// `systems/view.md §4`: the resident view-class table, over every terrain id.
#[test]
fn g7_view_s4_view_class_table_covers_every_terrain_id() {
    let class_of = |tile: u8| -> u8 {
        match tile {
            0x00 | 0xC0..=0xC3 | 0xCC..=0xCF | 0xFF => 0,
            0x05 | 0x30..=0x37 => 1,
            0x09..=0x0A | 0x2D => 2,
            0x07 | 0x1C | 0x1E..=0x1F | 0x40 | 0x44 | 0x48..=0x49 | 0x6A..=0x6B | 0x70..=0x7F
            | 0x87 | 0x8C | 0x8F | 0xAA | 0xBC | 0xDD => 3,
            0x1D | 0x38 | 0x47 | 0x5A | 0x5C..=0x5D | 0x94..=0x96 | 0x9A..=0x9C | 0xAB..=0xAC
            | 0xBE => 4,
            0x10..=0x1B | 0x29..=0x2B | 0x2E..=0x2F | 0x41..=0x43 | 0x4C | 0x58..=0x59 | 0x5B
            | 0x5E..=0x5F | 0x80..=0x85 | 0x88..=0x8B | 0x8D..=0x8E | 0x90..=0x93 | 0x9D..=0xA9
            | 0xAD..=0xB7 | 0xBD | 0xBF | 0xC8..=0xCB | 0xDE..=0xDF | 0xE8..=0xEB
            | 0xFA..=0xFD => 5,
            0x0D | 0x45 | 0x4A..=0x4B | 0x86 | 0x97..=0x99 | 0xB8..=0xBB | 0xC4..=0xC7
            | 0xEC..=0xF9 => 6,
            0x0C | 0x27..=0x28 | 0x39..=0x3F | 0x46 | 0x4D..=0x57 | 0xD0..=0xD3 | 0xFE => 7,
            0x0B | 0x0E..=0x0F => 8,
            0x06 | 0x08 | 0x2C => 9,
            0x03 | 0x60..=0x69 | 0x6C..=0x6F | 0xE4..=0xE7 => 0x0A,
            0x02 | 0xD4..=0xD7 => 0x0B,
            0x01 => 0x0C,
            0x04 => 0x0D,
            0xE0..=0xE3 => 0x0E,
            0xD8..=0xDC => 0x0F,
            0x20..=0x26 => 0x10,
            _ => unreachable!("tile {tile:#04x} has no published class"),
        }
    };
    for tile in 0u8..=255 {
        assert_eq!(
            crate::view_classes::tile_view_class(tile),
            class_of(tile),
            "tile {tile:#04x}"
        );
    }
}

/// `systems/view.md §3` row 5c: the dungeon name appended to `0xDF` by the
/// target cell's map X coordinate; any other X appends nothing.
#[test]
fn g7_view_s3_sealed_entrance_names_follow_the_x_table() {
    let state = test_state(open_grid(), 1, 1);
    let expected = [
        (58usize, "Shame"),
        (72, "Destard"),
        (91, "Despise"),
        (126, "Wrong"),
        (128, "Doom"),
        (156, "Covetous"),
        (239, "Hythloth"),
        (240, "Deceit"),
    ];
    for x in 0usize..=255 {
        let appended = expected.iter().find(|(cell, _)| *cell == x).map(|(_, name)| *name);
        assert_eq!(
            state.look_description(WORD_OF_POWER_SEALED_TILE, None, x),
            appended.unwrap_or(""),
            "x = {x}"
        );
    }
}

/// `systems/view.md §3` row 5b: "scene `30` appends Truth, scene `31` appends
/// Love, scene `32` appends Courage. In any other scene the base description is
/// printed with no appended word."
#[test]
fn g7_view_s3_flame_virtue_word_by_scene() {
    for scene in 0u8..=255 {
        let expected = match scene {
            30 => Some("Truth"),
            31 => Some("Love"),
            32 => Some("Courage"),
            _ => None,
        };
        assert_eq!(eternal_flame_word_for_scene(scene), expected, "scene {scene}");
    }
}

/// `systems/view.md §3` row 5a: "hour reduced to a twelve-hour value (zero
/// displayed as twelve), a colon, two-digit minutes, then an `AM` suffix for
/// hours zero through eleven and a `PM` suffix otherwise."
#[test]
fn g7_view_s3_clock_appender_twelve_hour_format() {
    let mut state = test_state(open_grid(), 1, 1);
    for (hour, minute, expected) in [
        (0u8, 5u8, "12:05 AM"),
        (11, 59, "11:59 AM"),
        (12, 0, "12:00 PM"),
        (13, 7, "1:07 PM"),
    ] {
        state.clock = GameClock::new(hour, minute).unwrap();
        assert_eq!(state.look_description(0xfa, None, 0), expected);
        assert_eq!(state.look_description(0xfb, None, 0), expected);
    }
}

/// `systems/view.md §3`: "The only granting scenes are Paws (`22`) and Empath
/// Abbey (`31`)."
#[test]
fn g7_view_s3_wishing_well_grants_only_in_two_scenes() {
    for scene in 0u8..=255 {
        assert_eq!(wishing_well_grant_scene(scene), matches!(scene, 22 | 31), "{scene}");
    }
}

/// `systems/view.md §4.2.1`: "When the hour is in `6..17` inclusive" the
/// daylight path runs.
#[test]
fn g7_view_s4_2_1_daylight_band_is_six_to_seventeen() {
    for hour in 0u8..24 {
        assert_eq!(sky_view_is_daylight(hour), (6..=17).contains(&hour), "{hour}");
    }
}

/// `systems/view.md §4.2.3`: "`elapsed` is `(years * 364) + (months * 28) +
/// days` measured from the epoch", year 39 month 4 day 5, with the eight
/// published row origins and the "return periods 3, 5, 7, 11, 13, 17, 19 and
/// 22 days".
#[test]
fn g7_view_s4_2_3_elapsed_days_and_row_periods() {
    let at = |year, month, day| GameClock::with_date(year, month, day, 20, 0).unwrap();
    assert_eq!(sky_elapsed_days(at(139, 4, 5)), 0);
    assert_eq!(sky_elapsed_days(at(139, 5, 5)), 28);
    assert_eq!(sky_elapsed_days(at(140, 4, 5)), 364);
    assert_eq!(sky_elapsed_days(at(141, 6, 7)), 2 * 364 + 2 * 28 + 2);
    let origins: Vec<u8> = SKY_ROW_SPECS.iter().map(|spec| spec.y_origin).collect();
    assert_eq!(origins, vec![144, 136, 120, 104, 88, 64, 40, 8]);
    let periods: Vec<usize> = SKY_ROW_SPECS
        .iter()
        .map(|spec| spec.permitted_columns.len())
        .collect();
    assert_eq!(periods, vec![3, 5, 7, 11, 13, 17, 19, 22]);
    for spec in SKY_ROW_SPECS {
        assert!(spec.permitted_columns.contains(&spec.start_column));
    }
}

// ---------------------------------------------------------------------------
// systems/magic.md
// ---------------------------------------------------------------------------

/// `systems/magic.md §8`: "position `(x=66, y=233)` prints the runic
/// equivalent of `O'J", E'C"`", latitude first, with a newline before and
/// after the pair.
#[test]
fn g7_magic_s8_locate_worked_example() {
    assert_eq!(sextant_coordinate_pair_line(233, 66), "\nO'J\", E'C\"\n");
}

/// `systems/magic.md §5` step 2: the published selector examples, and "the
/// parser sorts the typed letters before lookup ... `FV` and `VF` both
/// resolve to *Vas Flam*".
#[test]
fn g7_magic_s5_selector_examples_resolve_to_their_spells() {
    for (typed, rune) in [
        ("IL", "In Lor"),
        ("GP", "Grav Por"),
        ("IPVY", "In Vas Por Ylem"),
        ("PRV", "Vas Rel Por"),
        ("M", "Mani"),
        ("FV", "Vas Flam"),
        ("VF", "Vas Flam"),
        ("YVPI", "In Vas Por Ylem"),
    ] {
        let index = spell_index_from_code(&inline_spell_code(typed))
            .unwrap_or_else(|| panic!("{typed} resolves"));
        assert_eq!(spell_rune_name(index), Some(rune), "{typed}");
    }
    assert_eq!(spell_index_from_code(&inline_spell_code("JO")), None);
}

/// `systems/magic.md §4`: "The mana cost for any spell is `(spell_index / 6) +
/// 1`"; the table pairs each id's circle with its runes, from `In Lor` at id 0
/// to `An Tym` at id 47.
#[test]
fn g7_magic_s4_table_endpoints_and_circle_costs() {
    assert_eq!(spell_rune_name(0), Some("In Lor"));
    assert_eq!(spell_rune_name(47), Some("An Tym"));
    assert_eq!(spell_rune_name(48), None);
    for index in 0..48usize {
        assert_eq!(spell_mp_cost(index), Some((index / 6) as u8 + 1));
    }
}

/// `systems/magic.md §3`: "Jux and Ort ... are not resident U5 spell selectors";
/// the vocabulary is the twenty-four syllables of the table.
#[test]
fn g7_magic_s3_vocabulary_is_the_twenty_four_syllables() {
    let published = [
        "An", "Bet", "Corp", "Des", "Ex", "Flam", "Grav", "Hur", "In", "Kal", "Lor", "Mani", "Nox",
        "Por", "Quas", "Rel", "Sanct", "Tym", "Uus", "Vas", "Wis", "Xen", "Ylem", "Zu",
    ];
    assert_eq!(RUNE_SYLLABLE_VOCABULARY.len(), 24);
    for syllable in published {
        assert!(is_resident_rune_syllable(syllable), "{syllable}");
    }
    assert!(!is_resident_rune_syllable("Jux"));
    assert!(!is_resident_rune_syllable("Ort"));
}

// ---------------------------------------------------------------------------
// catalogs/quest-graph.md
// ---------------------------------------------------------------------------

/// `catalogs/quest-graph.md §5`: the destruction doom line "prints a separate
/// mixed-case set (`Faulinei`, `Astaroth`, `Nosfentor`)", while the Yell
/// keywords are capitals compared with a case-folding matcher.
#[test]
fn g7_quest_graph_s5_doom_names_and_yell_keywords() {
    assert_eq!(crate::commands::use_shard_doom_name(0), Some("Faulinei"));
    assert_eq!(crate::commands::use_shard_doom_name(1), Some("Astaroth"));
    assert_eq!(crate::commands::use_shard_doom_name(2), Some("Nosfentor"));
    assert_eq!(crate::commands::use_shard_doom_name(3), None);
    for (slot, name) in ["FAULINEI", "ASTAROTH", "NOSFENTOR"].into_iter().enumerate() {
        // Stored in capitals; the case folding belongs to the shared Yell
        // matcher (`commands.md §11.3`), not to this table.
        assert_eq!(shadowlord_name_for_slot(slot), Some(name));
        assert_eq!(shadowlord_slot_for_name(name), Some(slot));
    }
}

/// `catalogs/quest-graph.md §5`: "Consumers test the high bit, so any value
/// with the high bit set reads as vanquished." The Underworld placement pass
/// places a shard only while "Faulinei's slot is not vanquished".
#[test]
fn g7_quest_graph_s5_any_high_bit_slot_reads_vanquished_for_shard_placement() {
    let mut state = world_state(open_world_grid(), 10, 10);
    state.area = Area::World {
        plane: WorldPlane::Underworld,
    };
    state.shadowlord_hideouts = [0x80, 3, 4];
    state.active_objects.truncate(1);
    state.place_underworld_fixed_objects(WorldPlane::Underworld);
    assert!(
        !state
            .active_objects
            .iter()
            .any(|object| object.x == 192 && object.y == 80),
        "a high-bit slot reads as vanquished, so its shard is not placed"
    );
}

/// `catalogs/quest-graph.md §5`: Stonegate's "every non-vanquished slot
/// contributes that Shadowlord's 'air of' atmospheric line, while vanquished
/// slots are silent" - with "any value with the high bit set" reading as
/// vanquished.
#[test]
fn g7_quest_graph_s5_any_high_bit_slot_is_silent_at_stonegate() {
    let mut state = test_state(open_grid(), 1, 1);
    state.shadowlord_hideouts = [0xFF, 0x80, 0xFE];
    assert!(state.stonegate_entry_presentation_lines().is_empty());
}

/// `catalogs/quest-graph.md §3`: `DAWN` and `IMPERA` "belong to opposite
/// social graphs" and are not interchangeable.
#[test]
fn g7_quest_graph_s3_passwords_are_distinct() {
    use crate::quest_flags::{ConversationPassword, conversation_password};
    assert_eq!(conversation_password("dawn"), Some(ConversationPassword::Dawn));
    assert_eq!(conversation_password("IMPERA"), Some(ConversationPassword::Impera));
    assert_ne!(conversation_password("DAWN"), conversation_password("IMPERA"));
    assert_eq!(conversation_password("DAWNS"), None);
}
