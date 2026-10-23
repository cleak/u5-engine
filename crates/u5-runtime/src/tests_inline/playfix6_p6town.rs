// Regressions for the p6town exploratory play-test findings (K01..K19).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// Playtest K03/K10. `town-mode.md §13`: "After the map has been loaded and
/// the host recorded, a settlement that hosts a Shadowlord prints one line
/// naming which — an air of falsehood, of hatred, or of cowardice doth
/// surround thee". An overworld `E`-Enter replaced the message slot the
/// load-time pass had printed into, so the line never reached the window.
#[test]
fn p6town_entering_a_hideout_town_prints_its_air_line() {
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
    // Falsehood in Britain, Hatred in Trinsic, Cowardice in Moonglow.
    state.shadowlord_hideouts = [2, 6, 1];

    state.begin_command_echo_for(Command::Enter);
    state.enter_current_location(&dir).unwrap();

    let rows: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let name = rows
        .iter()
        .rposition(|row| *row == "BRITAIN")
        .expect("entry narration row");
    assert_eq!(
        &rows[name + 1..],
        &["An air of", "falsehood doth", "surround thee..."]
    );
    assert!(state.message_row_open_mid_line);
    assert!(!state.message.contains("hatred"));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest K03/K10, the other half of §13: "a settlement hosting none prints
/// nothing".
#[test]
fn p6town_entering_a_town_hosting_no_shadowlord_prints_no_air_line() {
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
    state.shadowlord_hideouts = [4, 6, 1];

    state.begin_command_echo_for(Command::Enter);
    state.enter_current_location(&dir).unwrap();

    assert!(!state
        .message_entries()
        .iter()
        .any(|entry| entry.text.starts_with("An air of")));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest K05. `time.md §10`: the surrender path "advances time in repeated
/// twenty-minute cleanup calls until the hour byte reaches 08:00", and
/// `npc-schedules.md §5` names the schedule processor's only callers as the
/// town turn loop and the H-Hole-Up burst. The wait must not walk NPCs.
#[test]
fn p6town_arrest_wait_to_morning_runs_no_schedule_pass() {
    // The same NPC engages and steps on a single schedule pass
    // (`a133_mode_four_engages_from_the_waypoint_distance_alone`).
    let mut state = g6_town_with(vec![g6_npc(1, 15, 5, 4, (6, 6, 0))], 5, 5);
    state.clock = GameClock::new(6, 30).unwrap();
    let ticks = state.advance_town_arrest_wait_to_morning();
    assert!(ticks > 0);
    assert_eq!(state.clock.hour, 8);
    assert_eq!(g6_pos(&state, 0), (15, 5), "no NPC moves during the wait");
}

/// Playtest K16. `npc-schedules.md §9.2` / `commands.md §3`: contact is
/// evaluated in the town epilogue *after* the command, so a conversation it
/// opens follows the command's own result instead of replacing it.
#[test]
fn p6town_contact_conversation_keeps_the_commands_result_line() {
    let dir = debug_game_dir();
    let keeper = NpcSlot {
        slot: 1,
        type_byte: 0x54,
        dialog_id: 0x81,
        schedule: [4, 4, 4, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
        name: None,
    };
    let idle = NpcSlot {
        slot: 0,
        type_byte: 0,
        dialog_id: 0,
        schedule: [0; 16],
        name: None,
    };
    let mut state = test_state(open_grid(), 5, 5);
    let Area::Town { scene, floor } = state.area else {
        unreachable!()
    };
    state.load_scheduled_npcs(&[idle, keeper]);
    state.begin_command_echo(top_down_command_echo('O').unwrap());
    state.message = "Nothing to open!".to_string();
    state.advance_npc_schedules();
    assert!(state.pending_town_npc_contact.is_some());
    let outcome = state.apply_town_npc_contact_event(scene, floor, &dir).unwrap();
    assert!(outcome.is_some());
    assert!(state.diagnostics_contain("opens conversation contact"));
    state.flush_message_slot();
    let rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    assert!(
        rows.iter().any(|row| row.ends_with("Nothing to open!")),
        "the command's result survives the contact: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Playtest K08. `town-mode.md §7`: A-Attack "looks for a live NPC linked to
/// the target sprite ... Invalid targets produce the ordinary failure text",
/// which `combat.md §8.2` gives as `Nothing to attack!`. A resident
/// Shadowlord (roster type `0xFC`) is refused, and the refusal used to print
/// nothing after `Attack-East`.
#[test]
fn p6town_attack_on_a_refused_town_npc_prints_the_ordinary_failure_text() {
    let mut shadowlord = g6_npc(31, 6, 5, 0, (6, 5, 0));
    shadowlord.type_byte = SHADOWLORD_ACTOR_TILE;
    let mut state = g6_town_with(vec![shadowlord], 5, 5);
    assert!(state.town_attack_target_at(0, 6, 5).is_some());

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.diagnostics_contain("no attackable town NPC"));
    assert_eq!(state.message, ATTACK_NOTHING_TO_ATTACK_REFUSAL);
    assert_eq!(state.npcs.len(), 1, "the refused target is left in place");
}
