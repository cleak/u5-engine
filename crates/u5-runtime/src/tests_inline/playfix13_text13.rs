// Regressions for the pass-13 text exploratory-playtest findings (P13-##).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// The rows from the last `>Get-North` echo: the echo, then what follows it.
fn p13text_get_north_rows(state: &mut PlayState, dir: &Path) -> Vec<String> {
    handle_play_key_input(state, 'G', "", dir).unwrap();
    handle_play_key_input(state, char::from(INPUT_CODE_NORTH), "", dir).unwrap();
    state.flush_staged_narration();
    p4window_rows_from(&p4window_rows(state), ">Get-North")
}

/// P13-07, overworld entry: `town-mode.md §13` "The notice's exact shape" is
/// a line feed, `An air of`, a line feed, the word, ` doth surround thee...`
/// "and a line feed", and "the next command prompt follows after the
/// ordinary blank row between commands". The notice left its full-width last
/// row flagged open, which swallowed the leading feed of every later result.
#[test]
fn playtest_p13_07_entry_notice_closes_its_row_for_later_commands() {
    let dir = debug_game_dir();
    write_all_location_family_fixtures(&dir);
    let entry = published_world_location_entries()
        .into_iter()
        .find(|entry| matches!(entry.target, PlayTarget::Town(scene) if scene.byte == 2))
        .expect("Britain has a published overworld entry");
    let mut grid = open_world_grid();
    grid[world_cell_index(entry.x, entry.y)] = entry.expected_tile.unwrap();
    let mut state = world_state(grid, entry.x, entry.y);
    state.area = Area::World { plane: entry.plane };
    state.active_objects[0].z = entry.plane.save_floor();
    state.shadowlord_hideouts = [2, 6, 1];

    state.begin_command_echo_for(Command::Enter);
    state.enter_current_location(&dir).unwrap();
    assert!(!state.message_row_open_mid_line);

    for _ in 0..2 {
        let tail = p13text_get_north_rows(&mut state, &dir);
        assert_eq!(tail.get(1).map(String::as_str), Some(""), "{tail:?}");
        assert!(tail.len() > 2, "{tail:?}");
    }
    let rows = p4window_rows(&state);
    let get = rows
        .iter()
        .position(|row| row == ">Get-North")
        .expect("first Get echo on screen");
    if let Some(notice) = rows.iter().position(|row| row == "surround thee...") {
        assert_eq!(get, notice + 2, "one blank row under the notice: {rows:?}");
    }
    let _ = fs::remove_dir_all(dir);
}

/// P13-07, a save loaded inside a hosting town: §13 "A save loaded inside a
/// hideout town prints the notice too", and it has the same closing feed.
#[test]
fn playtest_p13_07_load_time_notice_closes_its_row_for_later_commands() {
    let dir = debug_game_dir();
    let scene = Scene::new(1).unwrap();
    fs::write(dir.join("TOWNE.DAT"), open_grid()).unwrap();
    fs::write(dir.join("TOWNE.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
    fs::write(dir.join("TOWNE.TLK"), [0, 0]).unwrap();
    let mut options = PlayOptions::default();
    options.target = PlayTarget::Town(scene);
    options.start = Some((5, 5));
    options.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    options.saved_active_objects = None;

    let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();
    assert!(state.message.contains("An air of\nfalsehood doth"));
    assert!(!state.message_row_open_mid_line);
    for _ in 0..2 {
        let tail = p13text_get_north_rows(&mut state, &dir);
        assert_eq!(tail.get(1).map(String::as_str), Some(""), "{tail:?}");
        assert!(tail.len() > 2, "{tail:?}");
    }
    let _ = fs::remove_dir_all(dir);
}

/// P13-08: `conversation.md §9` step 2, "The dispatcher first emits a single
/// line feed", and §2: every line the dispatcher prints in place of a
/// conversation "follows the dispatcher's own leading line feed". The
/// mounted shop refusal is printed by that dispatcher after the schedule
/// gate (`shops.md §2`), so it keeps the blank row under `Talk-` the closed
/// refusal already had.
#[test]
fn playtest_p13_08_mounted_shop_refusal_keeps_the_dispatcher_feed() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.player.transport = TransportState::Horse {
        type_byte: FIRST_PLAYABLE_HORSE_TILE,
        tile: FIRST_PLAYABLE_HORSE_TILE,
    };
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x85,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.emit_command_echo_line("Talk-East");
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);
    state.flush_message_slot();

    let tail = p4window_rows_from(&p4window_rows(&state), ">Talk-East");
    assert_eq!(
        tail[..5].to_vec(),
        vec![
            ">Talk-East".to_string(),
            String::new(),
            "A merchant says:".to_string(),
            "\"GET THAT HORSE".to_string(),
            "OUT OF HERE!\"".to_string(),
        ]
    );
}
