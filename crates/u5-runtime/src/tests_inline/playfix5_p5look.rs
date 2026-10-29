// Regressions for the Look/command exploratory-playtest findings (p5look,
// J02-J29).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `view.md §3` rows 5a-5c: each appender row prints the base `LOOK2.DAT`
/// description and then "Append[s]" the time, virtue word or dungeon name.
/// The shipped base strings for these rows carry their own trailing space,
/// so the appender adds no separator of its own, and the clock time is "a
/// colon, two-digit minutes, then an AM suffix" with no brackets (J02/J07).
#[test]
fn playtest_j02_look_appenders_add_no_separator_or_brackets() {
    let table = parse_look2_dat(&look2_bytes(&[
        (0xfa, "clock placeholder: "),
        (ETERNAL_FLAME_LOOK_TILE as usize, "the Flame of "),
        (
            WORD_OF_POWER_SEALED_TILE as usize,
            "the collapsed entrance to the dungeon ",
        ),
    ]))
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);

    state.clock = GameClock::new(15, 7).unwrap();
    assert_eq!(
        state.look_description(0xfa, Some(&table), 0),
        "clock placeholder: 3:07 PM"
    );
    assert_eq!(
        state.look_description(WORD_OF_POWER_SEALED_TILE, Some(&table), 240),
        "the collapsed entrance to the dungeon Deceit"
    );

    state.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 2,
    };
    assert_eq!(
        state.look_description(ETERNAL_FLAME_LOOK_TILE, Some(&table), 15),
        "the Flame of Truth"
    );
}

/// `formats/signs-dat.md §5`: "The active scene indexes the `SIGNS.DAT`
/// scene directory" and the first matching record is printed (R598).
/// Shipped blocks are back to back with no sentinel between them; a scene-k
/// record must still be collected once, not once per non-empty directory
/// slot at or before k (J04).
#[test]
fn playtest_j04_sign_scene_blocks_stop_at_the_next_block() {
    let mut bytes = vec![0u8; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
    let first = SIGNS_DAT_SCENE_DIRECTORY_BYTES as u16;
    let second = first + 4 + 3 + 1;
    let third = second + 4 + 3 + 1;
    bytes[2 * 2..2 * 2 + 2].copy_from_slice(&first.to_le_bytes());
    bytes[3 * 2..3 * 2 + 2].copy_from_slice(&second.to_le_bytes());
    bytes[4 * 2..4 * 2 + 2].copy_from_slice(&third.to_le_bytes());
    bytes.extend_from_slice(&[2, 0, 15, 28]);
    bytes.extend_from_slice(b"Two\0");
    bytes.extend_from_slice(&[3, 0, 15, 28]);
    bytes.extend_from_slice(b"Thr\0");
    bytes.extend_from_slice(&[4, 0, 17, 17]);
    bytes.extend_from_slice(b"Fou\0");
    bytes.push(0);

    let records = parse_sign_records(&bytes).unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(find_sign(&records, 3, 0, 28, 15).map(|r| r.body.as_str()), Some("Thr"));
    assert_eq!(find_sign(&records, 4, 0, 17, 17).map(|r| r.body.as_str()), Some("Fou"));
}

/// `view.md §3` entry dispatch: row 3 prints the shared "thou dost see"
/// preamble for the sign row too, and row 5 "emit[s] a line break, then
/// render[s] the sign" - a second feed after the preamble's own, so a blank
/// row stands under `Thou dost see` (u5-spec#300 Q13). `formats/signs-dat.md §6`: the authored stream is laid
/// out without "proportional-font wrapping, implicit line breaking", so its
/// sixteen-cell rows stay intact rather than being re-flowed at word breaks.
/// No `Sign:` label is published (J05).
#[test]
fn playtest_j05_sign_look_prints_the_preamble_and_keeps_authored_rows() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    fs::write(
        dir.join(SIGNS_DAT_FILE),
        signs_dat_bytes_for_test(&[(
            17,
            0,
            1,
            2,
            b"g LAW OF VALOR ggDEATH OR THOU  g",
        )]),
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 0xa0;
    let mut state = test_state(grid, 1, 1);

    state.look_direction_with_game_dir(Direction::East, &dir).unwrap();

    assert_eq!(
        state.message,
        format!("{LOOK_RESULT_PREFIX}\n\ng LAW OF VALOR g\ngDEATH OR THOU  \ng")
    );
    let _ = fs::remove_dir_all(dir);
}

/// `formats/signs-dat.md §4`: a decoder "should preserve explicit
/// newlines", and §3's alias bridge is the stream's "line-separator marker".
/// Shipped signs narrower than the window end each authored row with a
/// high-bit line feed (`0x8A`), and every shipped bridge is a lone `0x0A`.
/// The line feed is a row break with a column reset (`text-output.md §5`),
/// so the rows stay aligned instead of running on cell by cell (J05 review).
#[test]
fn playtest_j05_sign_line_feeds_break_rows_and_bridge_aliases() {
    let mut bytes = vec![0u8; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
    let first = SIGNS_DAT_SCENE_DIRECTORY_BYTES as u16;
    bytes[4 * 2..4 * 2 + 2].copy_from_slice(&first.to_le_bytes());
    bytes.extend_from_slice(&[4, 0, 16, 28]);
    bytes.extend_from_slice(&[0x0a, 0]);
    bytes.extend_from_slice(&[4, 0, 14, 28]);
    bytes.extend_from_slice(b"g ROW g");
    bytes.push(0x8a);
    bytes.extend_from_slice(b"g TWO g");
    bytes.push(0);
    bytes.push(0);

    let records = parse_sign_records(&bytes).unwrap();
    assert_eq!(
        find_sign(&records, 4, 0, 28, 14).map(|r| r.body.as_str()),
        Some("g ROW g\ng TWO g")
    );
    assert_eq!(
        find_sign(&records, 4, 0, 28, 16).map(|r| r.body.as_str()),
        Some("g ROW g\ng TWO g")
    );

    assert_eq!(crate::play_state_impl::sign_stream_rows("15-cells-wide-r\nnext"), "15-cells-wide-r\nnext");
    // A line feed right after the sixteenth cell follows the edge wrap, so it
    // steps down a second row.
    assert_eq!(
        crate::play_state_impl::sign_stream_rows("sixteen-cells-ab\nnext"),
        "sixteen-cells-ab\n\nnext"
    );
}

/// `view.md §3`: the entry dispatch prints the shared "thou dost see"
/// preamble at row 3, before row 6's terrain path, whose row 2 sends a
/// telescope to the sky renderer "instead of printing any description
/// text". Only the description is replaced, not the preamble (J06).
#[test]
fn playtest_j06_telescope_look_keeps_the_thou_dost_see_preamble() {
    let mut state = test_state(open_grid(), 1, 1);

    state.clock = GameClock::new(22, 0).unwrap();
    state.look_through_telescope();
    assert_eq!(
        state.message,
        format!("{LOOK_RESULT_PREFIX}\nthe night sky! ")
    );

    state.active_view_overlay = None;
    state.clock = GameClock::new(10, 0).unwrap();
    state.look_through_telescope();
    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\nthe sun!"));
}

/// `dungeon-mode.md §4` step 3 routes only the cardinal codes, Enter and `.`
/// to movement and "everything else ... to the resident command
/// dispatcher"; `input.md §5` has diagonals "fall through as non-movement
/// input", and `commands.md §5.2` answers "any unmapped key" with `What?\n`.
/// Each diagonal therefore prints its own `What?`, never an invented sentence
/// (J03).
#[test]
fn playtest_j03_dungeon_diagonal_keys_print_what() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

    handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTHWEST), "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_SOUTHEAST), "", &dir).unwrap();

    let texts = transcript_texts(&state).join("\n");
    assert!(!texts.contains("Dungeon movement"), "{texts:?}");
    assert_eq!(texts.matches("What?").count(), 2, "{texts:?}");
    assert_eq!((state.player.x, state.player.y), (1, 1));
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §3`: "Most letters discard whatever their handler returned
/// and report the default \"acted\"" and the dungeon loop "[t]ests only \"is
/// it non-zero\", which selects the dungeon's post-action pass"; "The `E`
/// refusal inside a town or dungeon retains the default **acted** result".
/// So E, T and F refusals underground each spend the turn that drives the
/// post-action pass (the wake roll and the monster step), and the loop-head
/// minute is not charged twice (J08).
#[test]
fn playtest_j08_dungeon_acted_refusals_spend_the_post_action_turn() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    let clock_before = state.clock;

    for key in ['E', 'T', 'F'] {
        let turn_before = state.turn;
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
        assert_eq!(state.turn, turn_before + 1, "{key} must report acted");
    }

    let mut expected = clock_before;
    expected.advance_minutes(3);
    assert_eq!(state.clock, expected, "one loop-head minute per command");
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §3`: status `1` "Acted" is "kept by every letter that does
/// not forward or refuse", and "no action" comes only from unknown input,
/// `D`, `W`, `Q`, the typeahead toggle, dungeon `P` and refusing *forwarded*
/// handlers. `F`, `V` and `T` do not forward, so their refusals - `What?`,
/// `You have none!`, the outdoor `Funny, no response!` - still run the
/// per-turn epilogue (J09).
#[test]
fn playtest_j09_fire_view_and_outdoor_talk_refusals_spend_a_turn() {
    let dir = debug_game_dir();
    let mut world = britannia_state(open_world_grid(), 45, 62);
    world.gems = 0;
    for key in ['F', 'V', 't', 'f', 'v'] {
        let turn_before = world.turn;
        handle_play_key_input(&mut world, key, "", &dir).unwrap();
        assert_eq!(world.turn, turn_before + 1, "overworld {key} must report acted");
    }

    let mut town = test_state(open_grid(), 5, 5);
    town.gems = 0;
    for key in ['F', 'V'] {
        let turn_before = town.turn;
        handle_play_key_input(&mut town, key, "", &dir).unwrap();
        assert_eq!(town.turn, turn_before + 1, "town {key} must report acted");
    }
    let _ = fs::remove_dir_all(dir);
}

/// `input.md §6`: lowercase ASCII letters are folded to uppercase before
/// dispatch, and `commands.md §5.2` gives every dispatcher arm its verb echo.
/// The legacy lowercase diagonal letters must therefore echo `Z-stats...`,
/// `Cast...` and `New Order` like their uppercase twins, not the diagonal's
/// empty movement echo (J29).
#[test]
fn playtest_j29_lowercase_diagonal_letters_echo_their_verb() {
    for key in ['z', 'c', 'n', 'y', 'u', 'b'] {
        let upper = key.to_ascii_uppercase();
        assert_eq!(
            top_down_command_echo(key).map(|echo| echo.text),
            top_down_command_echo(upper).map(|echo| echo.text),
            "{key}"
        );
        assert!(top_down_command_echo(key).is_some(), "{key}");
    }
    // The cardinal harness letters keep their movement echo.
    assert_eq!(top_down_command_echo('w').map(|echo| echo.text), Some("North"));

    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    handle_play_key_input(&mut state, 'z', "", &dir).unwrap();
    let texts = transcript_texts(&state);
    assert_eq!(
        texts.first().map(String::as_str),
        top_down_command_echo('Z').map(|echo| echo.text),
        "{texts:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `formats/signs-dat.md §4` (corrected 2026-10-25, u5-spec#300 Q13 and Q21,
/// `RETRACTIONS.md` R600): the high bit selects the font per byte (clear:
/// runic, set: Roman), the control tests use the raw byte, and `0x8D` is a
/// carriage return, not a pause.
#[test]
fn spec300_q13_sign_bytes_pick_their_font_and_only_raw_controls_act() {
    use crate::tlk_runner::TlkGlyphFont::{Ordinary, Runic};
    let rendered = sign_payload_rendered(&[b'A', b'A' | 0x80, 0x26, 0x27, 0xa6, 0xa7, 0xa9, 0x8d, b'B']);
    let cells: Vec<(u8, _)> = rendered.glyphs.iter().map(|g| (g.byte, g.font)).collect();
    assert_eq!(
        cells,
        vec![
            (b'A', Runic),
            (b'A', Ordinary),
            (SIGN_SEPARATOR_RUNIC_CODE_POINT, Runic),
            (SIGN_SEPARATOR_RUNIC_CODE_POINT, Runic),
            (b'&', Ordinary),
            (b'\'', Ordinary),
            (b')', Ordinary),
            (b'B', Runic),
        ]
    );
    // Only raw `0x0D` pauses.
    assert!(
        sign_payload_rendered(&[b'A', 0x0d, b'B'])
            .glyphs
            .iter()
            .any(|g| g.byte == crate::text_hold::TEXT_HOLD_WAIT_KEY_MARKER)
    );
    assert_eq!(decode_sign_payload(&[b'A', 0x8d, b'B']), "AB");
}

/// `formats/signs-dat.md §5` step 6 (u5-spec#300 Q13, R598): only the first
/// matching record is printed; §4: the sign rows keep their runic cells in
/// the message window.
#[test]
fn spec300_q13_look_prints_only_the_first_match_in_its_own_font() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    fs::write(
        dir.join(SIGNS_DAT_FILE),
        signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"FIRST"), (17, 0, 1, 2, b"SECOND")]),
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 0xa0;
    let mut state = test_state(grid, 1, 1);

    state.look_direction_with_game_dir(Direction::East, &dir).unwrap();

    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\n\nFIRST"));
    let row = state
        .message_entries()
        .iter()
        .rev()
        .find(|entry| entry.text == "FIRST")
        .expect("the sign row reached the transcript");
    assert!(
        row.glyphs
            .iter()
            .all(|glyph| glyph.font == crate::tlk_runner::TlkGlyphFont::Runic),
        "{:?}",
        row.glyphs
    );
    assert!(!state.message_row_open_mid_line);
    let _ = fs::remove_dir_all(dir);
}
