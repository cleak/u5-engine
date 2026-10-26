// rc14: the cleak/u5-spec#302 answers the pass-11 branches left open (Q2 and
// Q3), and the inline `C` intercept's dungeon loop-head minute.

/// A one-member party of `class_byte` at level 2, for the attribute page.
fn rc14_z_stats_level_row(class_byte: u8) -> String {
    let mut state = test_state(open_grid(), 5, 5);
    state.party = vec![PartyMember {
        slot: 0,
        class_byte,
        status: b'G',
        climb_stat: 11,
        mana: 7,
        hp: 5,
        max_hp: 300,
        level: 2,
    }];
    state.z_stats_panel_rows(&ZStatsSession::new(0))[0].clone()
}

/// `inventory.md §4.7` (issue #302 Q2, `RETRACTIONS.md` R622): "The
/// level-line indent, in spaces, by class: Avatar 1, Mage 2, Bard 2,
/// Fighter 1, Druid 1, Tinker 1, Paladin 1, Ranger 1, Shepherd 0. It comes
/// from a per-class table in the shipped data, not from the name length."
/// Centring the line gave the twelve-cell Druid line two spaces.
#[test]
fn spec302_q2_level_line_takes_the_per_class_indent() {
    let glyph = char::from(SAVE_GENDER_MALE_BYTE);
    for (class_byte, indent, name) in [
        (b'A', 1, "Avatar"),
        (b'M', 2, "Mage"),
        (b'B', 2, "Bard"),
        (b'F', 1, "Fighter"),
        (b'D', 1, "Druid"),
        (b'T', 1, "Tinker"),
        (b'P', 1, "Paladin"),
        (b'R', 1, "Ranger"),
        (b'S', 0, "Shepherd"),
    ] {
        assert_eq!(
            rc14_z_stats_level_row(class_byte),
            format!("{}{glyph} Lv-2 {name}", " ".repeat(indent)),
            "class {}",
            char::from(class_byte)
        );
    }
}

/// `inventory.md §4.7` attribute-page rows (issue #302 Q2, R622): row 2 is
/// the status name alone, centred in columns 24 to 38, with no `Status:`
/// label; `__HP:` is current and `__HM:` maximum hit points in four cells;
/// `____Magic:` is followed by current magic points in two cells. The
/// executed example: `Str=16  HP: 240` / `Int=12  HM: 300` / `Dex=15  Ex: 123`
/// / blank / `    Magic: 7` (settles pass-10 R03 and pass-11 S04).
#[test]
fn spec302_q2_attribute_page_rows_match_the_executed_example() {
    let mut state = test_state(open_grid(), 5, 5);
    state.party = vec![PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: 15,
        mana: 7,
        hp: 240,
        max_hp: 300,
        level: 2,
    }];
    state.party_strengths = vec![16];
    state.party_intelligence = vec![12];
    state.party_experience = vec![123];
    let rows = state.z_stats_panel_rows(&ZStatsSession::new(0));
    assert_eq!(rows[1], "  Good Health");
    assert!(!rows.iter().any(|row| row.contains("Status")));
    assert_eq!(rows[2], "");
    assert_eq!(rows[3], "Str=16  HP: 240");
    assert_eq!(rows[4], "Int=12  HM: 300");
    assert_eq!(rows[5], "Dex=15  Ex: 123");
    assert_eq!(rows[6], "");
    assert_eq!(rows[7], "    Magic: 7");
    state.party[0].mana = 25;
    let rows = state.z_stats_panel_rows(&ZStatsSession::new(0));
    assert_eq!(rows[7], "    Magic:25");
}

/// A SIGNS.DAT image with one block per `(scene, records)` entry, laid back
/// to back in the order given, each record `(z, x, y, body)`.
fn rc14_signs_bytes(blocks: &[(u8, &[(u8, u8, u8, &[u8])])]) -> Vec<u8> {
    let mut bytes = vec![0u8; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
    for (scene, records) in blocks {
        let offset = u16::try_from(bytes.len()).unwrap();
        let slot = usize::from(*scene) * 2;
        bytes[slot..slot + 2].copy_from_slice(&offset.to_le_bytes());
        for (z, x, y, body) in *records {
            bytes.extend_from_slice(&[*scene, *z, *x, *y]);
            bytes.extend_from_slice(body);
            bytes.push(0);
        }
    }
    bytes
}

/// `formats/signs-dat.md §5` (issue #302 Q3, R623): the window is scanned on
/// into later scenes' records, "Only the record's `z`, `x` and `y` bytes are
/// compared", and the first match is printed; a lookup never reaches an
/// earlier scene's block.
#[test]
fn spec302_q3_window_reaches_later_scenes_and_compares_only_the_coordinate() {
    let bytes = rc14_signs_bytes(&[
        (3, &[(0, 1, 1, b"Three")]),
        (4, &[(0, 2, 2, b"Four"), (0, 1, 1, b"FourDup")]),
    ]);
    let found = |scene, z, y, x| find_sign_in_window(&bytes, scene, z, y, x).map(|r| r.body);
    // Executed on shipped data: a scene-3 lookup at a coordinate only a
    // scene-4 record has printed that scene-4 sign.
    assert_eq!(found(3, 0, 2, 2).as_deref(), Some("Four"));
    assert_eq!(found(3, 0, 1, 1).as_deref(), Some("Three"));
    assert_eq!(found(4, 0, 1, 1).as_deref(), Some("FourDup"));
    assert_eq!(found(4, 1, 1, 1), None);
    // A zero directory entry and a scene outside the directory find nothing.
    assert_eq!(found(5, 0, 2, 2), None);
    assert_eq!(found(40, 0, 2, 2), None);
}

/// `formats/signs-dat.md §5` (issue #302 Q3, R623): "The window is 2000
/// bytes", and "A shipped record can match only if its whole four-byte
/// header lies inside the window". The clean contract ends the scan at the
/// window's end (step 7) and may print a matched record's text in full from
/// the file even where it runs past the window.
#[test]
fn spec302_q3_window_is_two_thousand_bytes_and_needs_the_whole_header() {
    assert_eq!(SIGNS_DAT_READ_WINDOW_BYTES, 2000);
    let filler_body = |len: usize| vec![b'x'; len];
    // One filler record of total length `lead`, then the probe record.
    let image = |lead: usize, probe_body: &[u8]| {
        let filler = filler_body(lead - 4 - 1);
        let mut bytes = rc14_signs_bytes(&[(1, &[(0, 9, 9, &filler), (0, 3, 4, probe_body)])]);
        bytes.extend_from_slice(&[0; 8]);
        bytes
    };
    let probe = |bytes: &[u8]| find_sign_in_window(bytes, 1, 0, 4, 3).map(|r| r.body);
    // Header in the window's last four bytes: still matchable.
    let bytes = image(SIGNS_DAT_READ_WINDOW_BYTES - 4, b"Edge");
    assert_eq!(probe(&bytes).as_deref(), Some("Edge"));
    // Its text runs past the window and is printed whole from the file.
    let long = b"TextPastTheWindowEnd";
    let bytes = image(SIGNS_DAT_READ_WINDOW_BYTES - 4, long);
    assert_eq!(probe(&bytes).as_deref(), Some("TextPastTheWindowEnd"));
    // One byte later the header straddles the window's end: no match.
    let bytes = image(SIGNS_DAT_READ_WINDOW_BYTES - 3, b"Edge");
    assert_eq!(probe(&bytes), None);
    let bytes = image(SIGNS_DAT_READ_WINDOW_BYTES, b"Edge");
    assert_eq!(probe(&bytes), None);
}

/// `formats/signs-dat.md §5` (issue #302 Q3, R623): the clean contract
/// ignores the zero-padding pseudo-record, which the original would print as
/// an empty sign for a lookup at level 0, `(0, 0)`; the scan carries on past
/// it to later headers.
#[test]
fn spec302_q3_zero_padding_pseudo_record_is_ignored() {
    let mut bytes = rc14_signs_bytes(&[(18, &[(1, 5, 5, b"Own")])]);
    bytes.extend_from_slice(&[0, 0, 0, 0, 0]);
    bytes.extend_from_slice(&[19, 0, 0, 0]);
    bytes.extend_from_slice(b"Later\0");
    let found = find_sign_in_window(&bytes, 18, 0, 0, 0).map(|r| r.body);
    assert_eq!(found.as_deref(), Some("Later"));
    let mut bytes = rc14_signs_bytes(&[(18, &[(1, 5, 5, b"Own")])]);
    bytes.extend_from_slice(&[0, 0, 0, 0, 0]);
    assert_eq!(find_sign_in_window(&bytes, 18, 0, 0, 0), None);
}

/// `formats/signs-dat.md §5` step 5 through the Look path: a town lookup
/// whose own block has no record for the coordinate prints the later
/// scene's record, not the placard (issue #302 Q3).
#[test]
fn spec302_q3_look_prints_a_later_scenes_record_inside_the_window() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    // `test_state` stands in scene 17; the sign is at (x 2, y 1) on floor 0.
    let bytes = rc14_signs_bytes(&[
        (17, &[(0, 9, 9, b"Elsewhere")]),
        (18, &[(0, 2, 1, b"Later")]),
    ]);
    fs::write(dir.join(SIGNS_DAT_FILE), bytes).unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 0xa0;
    let mut state = test_state(grid, 1, 1);

    state
        .look_direction_with_game_dir(Direction::East, &dir)
        .unwrap();

    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\n\nLater"));
    let _ = fs::remove_dir_all(dir);
}

/// `dungeon-mode.md §4` step 1 / `§15`: the dungeon minute sits at the head
/// of every iteration. The inline `C` form is read by the shared intercept,
/// so after the no-action `W` it found the previous head's charge standing
/// and spent no minute - the J defect fixed in 16981e15 (playtest S16).
#[test]
fn dungeon_inline_cast_after_a_no_action_command_still_spends_its_minute() {
    let clock_after = |keys: &[char]| {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        for key in keys {
            handle_play_key_input(&mut state, *key, "", Path::new("")).unwrap();
        }
        handle_play_key_input(&mut state, 'C', "1IL", Path::new("")).unwrap();
        state.clock
    };
    let start = dungeon_state(open_dungeon_record(), 0, 1, 1).clock;
    let minutes = |count: u8| {
        let mut clock = start;
        clock.advance_minutes(count);
        clock
    };
    assert_eq!(clock_after(&[]), minutes(1), "C alone spends one minute");
    assert_eq!(clock_after(&['W']), minutes(2), "W;C is two iterations");
    assert_eq!(clock_after(&[' ']), minutes(2));
}
