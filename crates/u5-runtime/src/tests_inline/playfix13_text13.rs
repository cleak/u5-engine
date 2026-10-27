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

/// P13-10: `overworld.md §7.1` step 3 prints `Dost thou pay?` "with no
/// trailing feed. Wait for a key", and `text-output.md §10.2`: a prompt a
/// command waits at mid-run gets no marker row of its own - "a marker-and-
/// cursor row that appears beneath such a prompt ... is the next command
/// cycle's pair". So the question is the window's last row while it waits.
#[test]
fn playtest_p13_10_troll_toll_question_keeps_the_cursor_inline() {
    let mut state = britannia_state(open_world_grid(), 2, 1);
    state.gold = 100;
    state.emit_message_line("Caught!\n\nThe trolls demand a 54 gp toll!\n\nDost thou pay?");
    state.active_yes_no_prompt = Some(YesNoPromptSession::new(
        YesNoPromptKind::TrollBridgeToll { toll: 54 },
    ));

    assert!(state.message_window_live_row_suppressed());
    let rows = p4window_rows(&state);
    assert_eq!(
        rows.iter().rev().find(|row| !row.is_empty()).map(String::as_str),
        Some(TROLL_BRIDGE_TOLL_QUESTION),
        "{rows:#?}"
    );
}

/// P13-11: `view.md §4`, the local view overlay "waits for a keypress"
/// inside the command, and the shared modal wait "emits neither a line feed
/// nor a prompt marker while it waits". The `View_a_gem!\n` echo's own feed
/// opened the row that key is read on, as `Strange vision!\n` does for the
/// death vision, so neither §10.4's blank nor an end cap is drawn under it.
#[test]
fn playtest_p13_11_gem_view_wait_draws_no_blank_or_marker_row() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.gems = 2;
    handle_play_key_input(&mut state, 'V', "", &dir).unwrap();
    state.flush_message_slot();
    assert!(state.active_view_overlay.is_some());

    assert!(combat_prompt_row_follows_history(&state));
    assert!(shop_pause_row_is_continuation(&state));
    let rows = p4window_rows(&state);
    let echo = rows
        .iter()
        .rposition(|row| row == ">View a gem!")
        .expect("echo on screen");
    // The wait row - drawn here as the live row - follows the echo with no
    // blank row between them.
    assert_eq!(
        rows.get(echo + 1).map(String::as_str),
        Some(">"),
        "the wait row follows the echo: {rows:?}"
    );

    // The dismissing key ends the wait, and the next command takes the
    // ordinary shape again.
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(state.active_view_overlay.is_none());
    assert!(!shop_pause_row_is_continuation(&state));
    let _ = fs::remove_dir_all(dir);
}

/// P13-11, the Peer spell: `magic.md §5.1` "submitting the name advances to
/// the next row", Peer prints no completion line, and the overlay's wait is
/// the same modal one.
#[test]
fn playtest_p13_11_peer_view_wait_draws_no_blank_or_marker_row() {
    let mut state = world_state(open_world_grid(), 10, 20);
    arm_spell(&mut state, PEER_SPELL_INDEX);
    assert_eq!(state.cast_peer(0), MoveOutcome::Observed);
    assert!(state.active_view_overlay.is_some());
    assert!(state.view_overlay_row_opened_by_result);
    assert!(combat_prompt_row_follows_history(&state));
    assert!(shop_pause_row_is_continuation(&state));
}

/// P13-16: `combat.md §8.2`, "When two or three items qualify, each attempt
/// additionally prints a newline, that item's name, and a colon on its own
/// line". After a result line, which ends with its own feed, that newline
/// spends a blank row - as it already did after `Nothing!`.
#[test]
fn playtest_p13_16_next_item_line_after_a_result_has_a_blank_row() {
    let dir = debug_game_dir();
    let mut state = combat_player_command_state(6, 5);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_HELM] = 3; // Spiked Helm
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = 23; // Short Sword
    state.party_experience = vec![0];
    state.combat_actors[8].hp_or_wound = 200;
    state.pending_combat_actor_slot = Some(0);

    handle_play_key_input(&mut state, 'A', "", &dir).unwrap();
    assert!(state.active_combat_targeting.is_some());
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert!(
        state.active_combat_targeting.is_some(),
        "the Short Sword attempt is open"
    );

    let texts: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let sword = texts
        .iter()
        .rposition(|text| *text == "Short Sword:")
        .unwrap_or_else(|| panic!("second item line: {texts:?}"));
    assert_eq!(texts[sword - 1], "", "{texts:?}");
    assert_ne!(texts[sword - 2], "", "{texts:?}");
    let _ = fs::remove_dir_all(dir);
}

/// P13-07 in Stonegate: §13's notice shape holds "in a hideout town and in
/// Stonegate alike". An `E`-Enter reprints only the first notice under the
/// narration; the later ones are still staged behind the previous tone, so
/// each prints once, one blank row apart, and the next result keeps its
/// blank row.
#[test]
fn playtest_p13_07_stonegate_entry_notices_close_their_rows() {
    let dir = debug_game_dir();
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    fs::write(dir.join("KEEP.DAT"), location_pages()).unwrap();
    fs::write(dir.join("KEEP.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
    fs::write(dir.join("KEEP.TLK"), [0, 0]).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 KEEP:4 7 0x12 KEEP\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x12;
    let mut state = britannia_state(grid, 10, 20);
    state.shadowlord_hideouts = [0, SHADOWLORD_VANQUISHED, 3];
    state.begin_command_echo_for(Command::Enter);
    state.enter_current_location(&dir).unwrap();
    assert!(!state.message_row_open_mid_line);
    state.flush_staged_narration();
    assert!(!state.message_row_open_mid_line);

    let tail = p13text_get_north_rows(&mut state, &dir);
    assert_eq!(&tail[..3], &[">Get-North", "", "Nothing to get!"], "{tail:?}");
    let rows = p4window_rows(&state);
    let get = rows.iter().rposition(|row| row == ">Get-North").unwrap();
    assert_eq!(
        &rows[get - 8..get],
        &[
            "An air of",
            "cowardice doth",
            "surround thee...",
            "",
            "An air of",
            "falsehood doth",
            "surround thee...",
            "",
        ],
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// P13-07, a save loaded inside Stonegate: the load-time notices close their
/// rows too, so later results keep their blank row.
#[test]
fn playtest_p13_07_stonegate_load_notices_close_their_rows() {
    let dir = debug_game_dir();
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    fs::write(dir.join("KEEP.DAT"), location_pages()).unwrap();
    fs::write(dir.join("KEEP.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
    fs::write(dir.join("KEEP.TLK"), [0, 0]).unwrap();
    let mut options = PlayOptions::default();
    options.target = PlayTarget::Town(scene);
    options.start = Some((1, 1));
    options.shadowlord_hideouts = [0, SHADOWLORD_VANQUISHED, 3];
    let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();
    assert!(!state.message_row_open_mid_line);
    state.flush_staged_narration();
    assert!(!state.message_row_open_mid_line);
    for _ in 0..2 {
        let tail = p13text_get_north_rows(&mut state, &dir);
        assert_eq!(&tail[..3], &[">Get-North", "", "Nothing to get!"], "{tail:?}");
    }
    let _ = fs::remove_dir_all(dir);
}
