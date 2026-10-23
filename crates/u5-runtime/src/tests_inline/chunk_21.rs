/// A conversation reply is recorded on the message transcript; the
/// one-line slot carries the prompt that follows it.
fn transcript_has(state: &PlayState, needle: &str) -> bool {
    state
        .message_entries()
        .iter()
        .any(|entry| entry.text.contains(needle))
}

#[test]
fn dungeon_get_refuses_chest_room_trigger_or_unrelated_cell_without_turn() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x42;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x42);
    assert_eq!(state.turn, 0);
    // `commands.md §5.3`: the `Get` row itself comes from the dungeon arm of
    // the shared echo table, so the handler contributes only what follows it
    // (measured `qa/paired/dungeon-chest-refusals.tsv`).
    assert_eq!(state.message, DUNGEON_CHEST_GET_MUST_OPEN_FIRST);

    state.grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
    assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
    assert_eq!(state.turn, 0);
    // `commands.md §5.8`: "Get on a non-chest prints `Get\nNot here!\n`", the
    // `Get` row coming from the echo table.
    assert_eq!(state.message, DUNGEON_CHEST_GET_NOT_HERE);

    state.grid[dungeon_cell_index(0, 1, 1)] = 0x00;
    assert_eq!(state.get_dungeon_underfoot(scene, 0), MoveOutcome::Blocked);

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x00);
    assert_eq!(state.turn, 0);
    // `commands.md §5.8`: "Get on a non-chest prints `Get\nNot here!\n`", the
    // `Get` row coming from the echo table.
    assert_eq!(state.message, DUNGEON_CHEST_GET_NOT_HERE);
}

#[test]
fn dungeon_g_key_routes_to_underfoot_get() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x7d;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert!(state.handle_dungeon_key('g', Path::new("")).unwrap());

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x08);
    assert_eq!(state.turn, 1);
    assert!(state.diagnostics.iter().any(|note| note.contains("Got dungeon chest") || note.contains("got dungeon chest")));
}

#[test]
fn dungeon_open_unrelated_cell_is_not_a_turn() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

    assert_eq!(state.open_facing(), MoveOutcome::Blocked);

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x00);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, DUNGEON_CHEST_OPEN_WHAT);
}

#[test]
fn dungeon_open_preserves_room_trigger_without_turn() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(state.open_facing(), MoveOutcome::Blocked);

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, DUNGEON_CHEST_OPEN_WHAT);
}

#[test]
fn dungeon_open_underfoot_passage_chest_variant_reports_chest_opened() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x70;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x70);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, DUNGEON_CHEST_OPEN_ALREADY_OPEN);
}

#[test]
fn dungeon_jimmy_preserves_room_trigger_and_commits_one_action() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xf2;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(
        state
            .jimmy_facing_with_game_dir_and_member(None, Some(0))
            .unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xf2);
    assert_eq!(state.keys, DEFAULT_KEY_STOCK);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, DUNGEON_CHEST_JIMMY_WHAT);
}

#[test]
fn dungeon_e_cells_are_pass_through_visual_variants_not_openable_doors() {
    let mut open_grid = open_dungeon_record();
    open_grid[dungeon_cell_index(0, 1, 1)] = 0xe2;
    let mut open_state = dungeon_state(open_grid, 0, 1, 1);

    assert_eq!(open_state.open_facing(), MoveOutcome::Blocked);
    assert_eq!(open_state.grid[dungeon_cell_index(0, 1, 1)], 0xe2);
    assert_eq!(open_state.turn, 0);
    assert_eq!(open_state.message, DUNGEON_CHEST_OPEN_WHAT);

    let mut jimmy_grid = open_dungeon_record();
    jimmy_grid[dungeon_cell_index(0, 1, 1)] = 0xe2;
    let mut jimmy_state = dungeon_state(jimmy_grid, 0, 1, 1);
    assert_eq!(
        jimmy_state
            .jimmy_facing_with_game_dir_and_member(None, Some(0))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(jimmy_state.grid[dungeon_cell_index(0, 1, 1)], 0xe2);
    assert_eq!(jimmy_state.turn, 1);
    assert_eq!(jimmy_state.keys, DEFAULT_KEY_STOCK);
    assert_eq!(jimmy_state.message, DUNGEON_CHEST_JIMMY_WHAT);

    let mut step_grid = open_dungeon_record();
    step_grid[dungeon_cell_index(0, 2, 1)] = 0xe2;
    let mut step_state = dungeon_state(step_grid, 0, 1, 1);
    assert_eq!(step_state.step(Direction::East), MoveOutcome::Moved);
    assert_eq!((step_state.player.x, step_state.player.y), (2, 1));
    assert_eq!(step_state.turn, 1);
    assert_eq!(step_state.message, "");
}

#[test]
fn stale_dungeon_door_sidecar_file_does_not_block_f_room_trigger() {
    let dir = debug_game_dir();
    fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 2 1 0x70 0xF2\n").unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xF2;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xA2);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, DUNGEON_ROOM_ENTRY_NARRATION);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_open_ignores_stale_f_room_trigger_sidecar_without_turn() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 1 1 0x70 0xF2\n").unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.visibility_dirty = false;

    assert_eq!(
        state.open_facing_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xF2);
    assert_eq!(state.turn, 0);
    assert_eq!(state.door_tracker, None);
    assert!(!state.visibility_dirty);
    assert_eq!(state.message, DUNGEON_CHEST_OPEN_WHAT);
    assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_jimmy_key_on_f_room_trigger_ignores_stale_sidecar() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 1 1 0x70 0xF2\n").unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xF2;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.visibility_dirty = false;

    assert!(state.handle_dungeon_key('J', &dir).unwrap());

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xA2);
    assert_eq!(state.turn, 1);
    assert_eq!(state.keys, DEFAULT_KEY_STOCK);
    assert_eq!(state.door_tracker, None);
    assert!(state.visibility_dirty);
    assert_eq!(state.message, DUNGEON_ROOM_ENTRY_NARRATION);
    assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stale_sidecar_open_f_cell_key_runs_underfoot_trigger_first() {
    let dir = debug_game_dir();
    fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 1 1 0xF1 0xF2\n").unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xF1;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.visibility_dirty = false;

    assert!(state.handle_dungeon_key('J', &dir).unwrap());

    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xA1);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, DUNGEON_ROOM_ENTRY_NARRATION);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stale_sidecar_open_f_cell_still_enters_room_trigger() {
    let dir = debug_game_dir();
    fs::write(dir.join("dungeon_doors.tsv"), "DUNGEON:0 0 2 1 0xF1 0xF2\n").unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xF1;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xA1);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, DUNGEON_ROOM_ENTRY_NARRATION);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_look_reports_facing_actor_without_spending_turn() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert!(state.message.contains("actor tile 192"));
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_look_sign_class_object_prints_its_object_description() {
    // `view.md §3` entry dispatch: a per-map object (row 4) wins over the
    // sign path (row 5), which is keyed on the live terrain tile only.
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[])).unwrap();
    fs::write(
        dir.join(SIGNS_DAT_FILE),
        signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"Posted notice")]),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0xa0,
        tile: 0xa0,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert!(state.message.starts_with(LOOK_RESULT_PREFIX), "{}", state.message);
    assert!(!state.message.contains("Posted notice"));
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_look_sign_terrain_tile_reads_signs_dat_or_the_blank_sign() {
    // `view.md §3` row 5: live tile `0x89`/`0x8A`/`0xA0`/`0xA4`/`0xF8`
    // selects the sign path; "If no record matches, the sign path prints
    // the blank-sign fallback."
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(0x5a, "a sign")])).unwrap();
    fs::write(
        dir.join(SIGNS_DAT_FILE),
        signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"Posted notice"), (17, 0, 2, 1, b"Hidden")]),
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 0x89;
    grid[3 * 32 + 1] = 0xf8;
    // A non-sign tile with a SIGNS.DAT record is described, not read.
    grid[2 * 32 + 1] = 0x5a;
    let mut state = test_state(grid, 1, 1);

    state.look_direction_with_game_dir(Direction::East, &dir).unwrap();
    assert_eq!(state.message, "Sign:\nPosted notice");

    state.look_direction_with_game_dir(Direction::South, &dir).unwrap();
    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\na sign"));

    state.player.y = 2;
    state.look_direction_with_game_dir(Direction::South, &dir).unwrap();
    assert_eq!(state.message, "Sign:\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_look_redirect_tiles_chain_to_the_final_cell() {
    // `view.md §3` terrain-description path row 1: `0xE0` north, `0xE1`
    // east, `0xE2` west; "The redirect can chain".
    let table = parse_look2_dat(&look2_bytes(&[(0x40, "a target"), (0xe1, "redirect")])).unwrap();
    let mut grid = open_grid();
    // Look east from (1, 3) onto (2, 3) = E1 -> (3, 3) = E0 -> (3, 2).
    grid[3 * 32 + 2] = LOOK_REDIRECT_EAST_TILE;
    grid[3 * 32 + 3] = LOOK_REDIRECT_NORTH_TILE;
    grid[2 * 32 + 3] = 0x40;
    let mut state = test_state(grid, 1, 3);

    state.look_direction_with_table(Direction::East, Some(&table));
    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\na target"));

    // West redirect lands on (1, 3)... set up a fresh one-step chain.
    let mut grid = open_grid();
    grid[5 * 32 + 5] = LOOK_REDIRECT_WEST_TILE;
    grid[5 * 32 + 4] = 0x40;
    let mut state = test_state(grid, 5, 4);
    state.look_direction_with_table(Direction::South, Some(&table));
    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\na target"));
}

#[test]
fn world_look_redirect_wraps_across_the_map_edge() {
    let table = parse_look2_dat(&look2_bytes(&[(0x40, "a target")])).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(0, 1)] = LOOK_REDIRECT_WEST_TILE;
    grid[world_cell_index(255, 1)] = 0x40;
    let mut state = britannia_state(grid, 1, 1);

    state.look_direction_with_table(Direction::West, Some(&table));
    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\na target"));
}

#[test]
fn yew_wanted_poster_exception_uses_party_names_without_signs_dat() {
    // `view.md §3` row 5: the poster is reached through the live sign
    // tile; the fixed coordinate is tested only after that.
    let mut grid = open_grid();
    grid[21 * 32 + 17] = 0xa0;
    let mut state = test_state(grid, 16, 21);
    state.area = Area::Town {
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert_eq!(
        state.message,
        [
            "abbbbbbbbbbbbbc",
            "g   Wanted:   g",
            "g             g",
            "g   AVATAR    g",
            "g    IOLO     g",
            "g             g",
            "g             g",
            "gDead or Aliveg",
            "deeeeeeeeeeeeeef",
        ]
        .join("\n")
    );
    assert_eq!(state.turn, 0);
}

#[test]
fn town_look_routes_death_vision_terrain_tile_before_object_description() {
    // `view.md §3` entry-dispatch row 2: "Live tile `0x29` (the
    // crystal-sphere tile)" — a **live terrain-layer** byte, tested
    // before the row-4 per-map object lookup. This test previously
    // pinned the object-descriptor reading §3 warns against.
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.party_intelligence[0] = 31;
    state.grid[1 * 32 + 2] = DEATH_VISION_LOOK_TILE;
    // An ordinary active object sharing the cell must not decide the
    // row: the terrain byte alone does.
    state.active_objects.push(ActiveObject {
        type_byte: 0x10,
        tile: 0x10,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    // One eligible member is selected silently (`view.md §3`), so the
    // result is one line with no `Player: ` exchange.
    assert!(state.active_direction_prompt.is_none());
    assert!(state.active_party_selector.is_none());
    assert_eq!(state.message, DEATH_VISION_STRANGE_LINE);
    assert!(state.active_view_overlay.is_some());
    assert!(!state.message.contains("actor tile"));
    assert_eq!(state.turn, 0);
}

#[test]
fn town_look_object_byte_0x29_does_not_trigger_the_death_vision() {
    // `view.md §3`: the row-2 byte is "a single terrain-layer byte
    // ... never an active-object or creature descriptor", and §3
    // warns about the identical two-domain confusion for the
    // `0xD8..0xDB` fountain/Daemon band: "Same four numbers, two
    // different lookup domains, no relationship between them."
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.party_intelligence[0] = 31;
    state.active_objects.push(ActiveObject {
        type_byte: DEATH_VISION_LOOK_TILE,
        tile: DEATH_VISION_LOOK_TILE,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert!(state.active_direction_prompt.is_none());
    assert!(!state.message.contains("death-vision member"));
}

#[test]
fn death_vision_failure_prints_the_measured_line_without_an_overlay() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.party_intelligence[0] = 0;

    assert_eq!(
        state.apply_death_vision_look_for_member(2, 1, 0),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, DEATH_VISION_DEATH_LINE);
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.turn, 0);
}

#[test]
fn death_vision_failure_costs_the_member_one_hit_point() {
    // `view.md §3`: the failed arm applies "one HP of damage to that
    // member with the ordinary hit feedback"; HP reaching zero sets the
    // member Dead and clears an override naming that member.
    let mut state = test_state(open_grid(), 1, 1);
    state.party_intelligence[0] = 0;
    state.party[0].hp = 5;
    state.apply_death_vision_look_for_member(2, 1, 0);
    assert_eq!(state.party[0].hp, 4);

    state.party[0].hp = 1;
    state.active_player = Some(0);
    state.apply_death_vision_look_for_member(2, 1, 0);
    assert_eq!(state.party[0].hp, 0);
    assert_eq!(state.party[0].status, CharacterStatus::Dead.save_byte());
    assert_eq!(state.active_player, None);
}

#[test]
fn death_vision_with_two_eligible_members_asks_player_and_resumes_for_the_pick() {
    // `view.md §3`: "two or more eligible members do produce `Player: `;
    // the crystal sphere does not bypass that shared selection rule".
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.grid[1 * 32 + 2] = DEATH_VISION_LOOK_TILE;
    let mut second = state.party[0];
    second.slot = 1;
    second.hp = 7;
    state.party.push(second);
    state.party_intelligence = vec![31, 0];

    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert!(state.active_party_selector.is_some());
    assert!(state.active_view_overlay.is_none());

    // Pick the second member (Intelligence 0): the vision fails for them.
    assert!(state.step_active_party_selector('2', ""));
    assert!(state.step_active_party_selector('\r', ""));
    assert_eq!(state.message, DEATH_VISION_DEATH_LINE);
    assert_eq!(state.party[1].hp, 6);
}

#[test]
fn death_vision_selector_cancel_adds_no_vision_result() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.grid[1 * 32 + 2] = DEATH_VISION_LOOK_TILE;
    let mut second = state.party[0];
    second.slot = 1;
    state.party.push(second);
    state.party_intelligence = vec![31, 31];
    let hp = state.party[0].hp;

    state.look_facing();
    assert!(state.step_active_party_selector('\u{1b}', ""));
    assert!(!state.message.contains("vision"));
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.party[0].hp, hp);
}

#[test]
fn town_look_direction_samples_selected_direction_without_turn_or_turning() {
    let table = parse_look2_dat(&look2_bytes(&[(16, "east road"), (17, "south road")])).unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 16;
    grid[2 * 32 + 1] = 17;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::South;

    assert_eq!(
        state.look_direction_with_table(Direction::East, Some(&table)),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\neast road"));
    assert_eq!(state.player.facing, Direction::South);
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_look_visibility_gate_hides_dark_unlit_target() {
    let table = parse_look2_dat(&look2_bytes(&[(16, "east road")])).unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = 16;
    let mut hidden = test_state(grid.clone(), 1, 1);
    hidden.player.facing = Direction::East;
    // `visibility.md §4`: zero light is the pitch-dark branch — the
    // carve is skipped and even the adjacent cell stays hidden.
    // `FULL_DARKNESS` (2) is a threshold that still lights it.
    hidden.ambient_light = 0;
    hidden.message = "previous".to_string();

    assert_eq!(
        hidden.look_direction_with_table(Direction::East, Some(&table)),
        MoveOutcome::Observed
    );

    assert!(hidden.message.is_empty());
    assert_eq!(hidden.turn, 0);

    let mut lit = test_state(grid, 1, 1);
    lit.player.facing = Direction::East;
    lit.ambient_light = FULL_DARKNESS;
    lit.torch_counter = 1;
    lit.recompute_daylight();

    assert_eq!(
        lit.look_direction_with_table(Direction::East, Some(&table)),
        MoveOutcome::Observed
    );

    assert_eq!(lit.message, format!("{LOOK_RESULT_PREFIX}\neast road"));
    assert_eq!(lit.turn, 0);
}

#[test]
fn town_look_uses_look2_description_when_available() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(16, "stone path")])).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert!(state.message.contains("stone path"));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
}

#[test]
fn town_look_at_surface_fountain_prompts_for_drinker_without_spending_turn() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xd8;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    // A capture of the stock game runs this on the shared party-member
    // selector - the `Select:` border label with the chosen row inverted -
    // not on a second direction-prompt stage.
    assert_eq!(
        state
            .active_party_selector
            .as_ref()
            .map(|session| session.target),
        Some(PartySelectorTarget::FountainDrink {
            direction: Direction::East
        })
    );
    assert!(state.active_direction_prompt.is_none());
    assert_eq!(
        state.message,
        format!(
            "{LOOK_RESULT_PREFIX}\n{FOUNTAIN_LOOK_DESCRIPTION}\n\n{FOUNTAIN_DRINK_PROMPT}"
        )
    );
    assert_eq!(state.roster_box_label().as_deref(), Some("Select:"));
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_look_at_surface_wishing_well_prompts_for_coin_without_spending_turn() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert_eq!(
        state
            .active_wishing_well
            .as_ref()
            .map(|session| (session.direction, session.coin_accepted)),
        Some((Direction::East, false))
    );
    assert!(state.message.ends_with(WISHING_WELL_COIN_PROMPT));
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_look_visibility_gate_precedes_special_surface_prompts() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut hidden = test_state(grid.clone(), 1, 1);
    hidden.player.facing = Direction::East;
    // `visibility.md §4`: zero light is the pitch-dark branch — the
    // carve is skipped and even the adjacent cell stays hidden.
    // `FULL_DARKNESS` (2) is a threshold that still lights it.
    hidden.ambient_light = 0;
    hidden.gold = 1;

    assert_eq!(hidden.look_facing(), MoveOutcome::Observed);

    assert!(hidden.message.is_empty());
    assert!(hidden.active_wishing_well.is_none());

    let mut lit = test_state(grid, 1, 1);
    lit.player.facing = Direction::East;
    lit.ambient_light = FULL_DARKNESS;
    lit.torch_counter = 1;
    lit.recompute_daylight();

    assert_eq!(lit.look_facing(), MoveOutcome::Observed);

    assert_eq!(
        lit.active_wishing_well
            .as_ref()
            .map(|session| (session.direction, session.coin_accepted)),
        Some((Direction::East, false))
    );
}

#[test]
fn town_surface_wishing_well_decline_or_empty_purse_has_no_effect() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut decline = test_state(grid.clone(), 1, 1);
    decline.player.facing = Direction::East;
    decline.gold = 7;
    assert_eq!(decline.look_facing(), MoveOutcome::Observed);

    assert_eq!(
        decline.step_active_wishing_well('N', ""),
        Some(MoveOutcome::Observed)
    );
    assert!(decline.active_wishing_well.is_none());
    assert_eq!(decline.gold, 7);
    // Measured: the declined coin completes the prompt row.
    assert!(decline.message.ends_with(WISHING_WELL_COIN_NO_REPLY));
    assert_eq!(decline.turn, 0);

    let mut empty = test_state(grid, 1, 1);
    empty.player.facing = Direction::East;
    empty.gold = 0;
    assert_eq!(empty.look_facing(), MoveOutcome::Observed);

    assert_eq!(
        empty.step_active_wishing_well('Y', ""),
        Some(MoveOutcome::Observed)
    );
    assert!(empty.active_wishing_well.is_none());
    assert_eq!(empty.gold, 0);
    // An empty purse cannot pay the coin; its own line is
    // unmeasured, so nothing prints.
    assert!(empty.message.is_empty());
    assert_eq!(empty.turn, 0);
}

#[test]
fn town_surface_wishing_well_coin_then_wish_consumes_coin_without_turn() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(0x16).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 2;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert_eq!(state.step_active_wishing_well('Y', ""), None);
    assert_eq!(state.gold, 1);
    assert_eq!(
        state
            .active_wishing_well
            .as_ref()
            .map(|session| (session.direction, session.coin_accepted)),
        Some((Direction::East, true))
    );
    assert!(state.message.ends_with(WISHING_WELL_WISH_PROMPT));

    assert_eq!(
        state.step_active_wishing_well('H', "orse"),
        Some(MoveOutcome::Observed)
    );
    assert!(state.active_wishing_well.is_none());
    assert_eq!(state.gold, 1);
    assert!(state.message.ends_with(WISHING_WELL_GRANT_LINE));
    let horse = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == HORSE_PARKED_FIRST)
        .expect("accepted wishing-well wish should spawn a horse");
    assert_eq!(
        (horse.type_byte, horse.x, horse.y, horse.z, horse.aux1),
        (HORSE_PARKED_FIRST, 2, 1, 0, 0)
    );
    assert_eq!(
        state
            .boardable_vehicle_slot_at(2, 1)
            .map(|candidate| candidate.transport),
        Some(TransportState::Horse {
            type_byte: HORSE_MOUNTED_FIRST,
            tile: FIRST_PLAYABLE_HORSE_TILE,
        })
    );
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_surface_wishing_well_rejects_unknown_wish_after_coin() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.gold = 2;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    assert_eq!(state.step_active_wishing_well('Y', ""), None);

    assert_eq!(
        state.step_active_wishing_well('A', "vatar"),
        Some(MoveOutcome::Observed)
    );

    assert!(state.active_wishing_well.is_none());
    assert_eq!(state.gold, 1);
    // Measured: an unaccepted wish answers `No effect...` under the
    // upper-cased echo.
    assert!(state.message.ends_with(WISHING_WELL_NO_EFFECT_LINE));
    assert!(
        state
            .active_objects
            .iter()
            .skip(1)
            .all(|object| object.type_byte != HORSE_PARKED_FIRST)
    );
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_surface_wishing_well_rejects_accepted_wish_outside_grant_scenes() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.gold = 2;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    assert_eq!(state.step_active_wishing_well('Y', ""), None);

    assert_eq!(
        state.step_active_wishing_well('H', "orse"),
        Some(MoveOutcome::Observed)
    );

    assert_eq!(state.gold, 1);
    // Measured: an accepted word outside a granting scene answers the
    // same `No effect...` line.
    assert!(state.message.ends_with(WISHING_WELL_NO_EFFECT_LINE));
    assert!(state.boardable_vehicle_slot_at(2, 1).is_none());
    assert_eq!(state.turn, 0);
}

#[test]
fn town_surface_wishing_well_car_wish_grants_horse_family_object() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(0x1f).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 2;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    assert_eq!(state.step_active_wishing_well('Y', ""), None);

    assert_eq!(
        state.step_active_wishing_well('F', "errari"),
        Some(MoveOutcome::Observed)
    );

    assert!(state.message.ends_with(WISHING_WELL_GRANT_LINE));
    assert!(state.boardable_vehicle_slot_at(2, 1).is_some());
}

#[test]
fn town_surface_fountain_drink_refreshes_living_member_without_mutating() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xd9;
    let mut state = test_state(grid, 1, 1);
    state.party[0].status = CharacterStatus::Poisoned.save_byte();
    state.party[0].hp = 12;
    state.party[0].max_hp = 90;
    let before = state.party[0];

    assert_eq!(
        state.look_surface_fountain_with_drinker(Direction::East, 0),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, FOUNTAIN_DRINK_REFRESHED);
    assert_eq!(state.party[0], before);
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_surface_fountain_drink_refuses_incapacitated_member_without_mutating() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xda;
    let mut state = test_state(grid, 1, 1);
    state.party[0].status = CharacterStatus::Sleeping.save_byte();
    state.party[0].hp = 12;
    let before = state.party[0];

    assert_eq!(
        state.look_surface_fountain_with_drinker(Direction::East, 0),
        MoveOutcome::Observed
    );

    // `view.md §3`, since `cleak/u5-spec#197` published the literals: "A
    // Dead or Asleep member prints `Incapacitated!` and a blank row".
    // It is still a refusal: no refresh line, and no party state written.
    assert_eq!(state.message, FOUNTAIN_DRINK_INCAPACITATED);
    assert_ne!(state.message, FOUNTAIN_DRINK_REFRESHED);
    assert_eq!(state.party[0], before);
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_surface_fountain_prompt_digit_routes_refresh_result() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xdb;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    // `cleak/u5-spec#192`: a digit moves the bar without committing, so
    // the drinker is taken on Return.
    assert!(state.step_active_party_selector('1', ""));
    assert!(state.active_party_selector.is_some());
    assert!(state.step_active_party_selector('\r', ""));

    assert!(state.active_party_selector.is_none());
    assert_eq!(state.message, FOUNTAIN_DRINK_REFRESHED);
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_surface_fountain_prompt_cancel_prints_no_one_result() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xd8;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert!(state.step_active_party_selector('\u{1b}', ""));

    assert!(state.active_party_selector.is_none());
    // The prompt punctuates itself, so the universal cancel word opens the
    // next line instead of continuing it.
    assert_eq!(state.message, SELECTION_CANCELLED_LITERAL);
    assert_eq!(state.turn, 0);
}

fn signs_dat_bytes_for_test(records: &[(u8, u8, u8, u8, &[u8])]) -> Vec<u8> {
    let mut bytes = vec![0; SIGNS_DAT_SCENE_DIRECTORY_BYTES];
    if let Some((scene, ..)) = records.first() {
        let offset = SIGNS_DAT_SCENE_DIRECTORY_BYTES as u16;
        bytes[*scene as usize * 2..*scene as usize * 2 + 2].copy_from_slice(&offset.to_le_bytes());
    }
    for (scene, z, y, x, body) in records {
        bytes.extend_from_slice(&[*scene, *z, *x, *y]);
        bytes.extend_from_slice(body);
        bytes.push(0);
    }
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    bytes
}

#[test]
fn town_look_renders_matching_signs_dat_record_and_spends_its_turn() {
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(0xa0, "a sign")])).unwrap();
    fs::write(
        dir.join(SIGNS_DAT_FILE),
        signs_dat_bytes_for_test(&[(17, 0, 1, 2, b"North Road")]),
    )
    .unwrap();
    let mut grid = open_grid();
    // `view.md §3` row 5: the sign path is keyed on the live tile.
    grid[32 + 2] = 0xa0;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, "Sign:\nNorth Road");
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_look_telescope_routes_to_night_sky() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(LOOK2_DAT_FILE),
        look2_bytes(&[(TELESCOPE_LOOK_TRIGGER_TILE as usize, "telescope")]),
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = TELESCOPE_LOOK_TRIGGER_TILE;
    let mut state = britannia_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.clock = GameClock::new(20, 0).unwrap();

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, "the night sky! ");
    assert!(!state.message.contains("telescope"));
    assert!(
        state
            .active_view_overlay
            .as_ref()
            .is_some_and(|overlay| matches!(overlay.kind, ViewOverlayKind::Sky(_)))
    );
    let viewport = state
        .render_active_view_overlay(TileGraphicsDepth::Ega16)
        .expect("look-triggered night sky should install a renderable overlay");
    assert_eq!(viewport.cells_wide, SKY_VIEW_COLUMNS);
    assert_eq!(viewport.cells_high, SKY_VIEW_ROWS);
    assert!(viewport.pixels.iter().any(|pixel| *pixel != 0));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(20, 2).unwrap());

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.turn, 1);
    assert!(state.message.is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_look_telescope_daylight_shows_sun_and_applies_damage() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(LOOK2_DAT_FILE),
        look2_bytes(&[(TELESCOPE_LOOK_TRIGGER_TILE as usize, "telescope")]),
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = TELESCOPE_LOOK_TRIGGER_TILE;
    let mut state = britannia_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.clock = GameClock::new(12, 2).unwrap();
    state.active_player = None;
    let hp_before = state.party[0].hp;

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, "the sun!");
    assert_eq!(state.active_player, Some(0));
    assert_eq!(state.party[0].hp, hp_before - 1);
    assert!(state.active_view_overlay.is_none());
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_look_telescope_enters_the_sky_renderer_instead_of_a_description() {
    // view.md §3 terrain-description path row 2: "Live tile `0x59`, a
    // telescope | Enter the sky renderer of Section 4.2 instead of
    // printing any description text." The table is not scene-scoped, and
    // §3 records that all three shipped telescopes are indoors - "in
    // Moonglow, in Skara Brae, and in West Britanny" - so the town-family
    // arm is the only arm ordinary play can reach it through.
    let dir = debug_game_dir();
    fs::write(
        dir.join(LOOK2_DAT_FILE),
        look2_bytes(&[(TELESCOPE_LOOK_TRIGGER_TILE as usize, "telescope")]),
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TELESCOPE_LOOK_TRIGGER_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.clock = GameClock::new(20, 0).unwrap();

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, "the night sky! ");
    assert!(!state.message.contains("telescope"));
    assert!(
        state
            .active_view_overlay
            .as_ref()
            .is_some_and(|overlay| matches!(overlay.kind, ViewOverlayKind::Sky(_)))
    );
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_look_telescope_is_tested_above_the_wishing_well_row() {
    // view.md §3: the telescope is row 2 and the wishing well row 3, and
    // "an earlier revision of this document glossed the telescope tile as
    // a wishing well, and that label is withdrawn". They are two different
    // fixtures routed to two different handlers.
    let dir = debug_game_dir();
    fs::write(
        dir.join(LOOK2_DAT_FILE),
        look2_bytes(&[(TELESCOPE_LOOK_TRIGGER_TILE as usize, "telescope")]),
    )
    .unwrap();
    let mut grid = open_grid();
    grid[32 + 2] = TELESCOPE_LOOK_TRIGGER_TILE;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.clock = GameClock::new(12, 0).unwrap();
    state.active_player = None;
    let hp_before = state.party[0].hp;

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    // The §4.2 daylight branch, not a coin-and-wish prompt and not a
    // LOOK2 description line.
    assert_eq!(state.message, "the sun!");
    assert_eq!(state.party[0].hp, hp_before - 1);
    assert!(state.active_wishing_well.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn look_clock_tiles_append_twelve_hour_time_context() {
    let table = parse_look2_dat(&look2_bytes(&[(0xfa, "a clock, showing: ")])).unwrap();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);

    state.clock = GameClock::new(0, 7).unwrap();
    assert_eq!(
        state.look_description(0xfa, Some(&table), 0),
        "a clock, showing: 12:07 AM"
    );

    state.clock = GameClock::new(12, 0).unwrap();
    assert_eq!(
        state.look_description(0xfa, Some(&table), 0),
        "a clock, showing: 12:00 PM"
    );

    state.clock = GameClock::new(23, 59).unwrap();
    assert_eq!(
        state.look_description(0xfa, Some(&table), 0),
        "a clock, showing: 11:59 PM"
    );
}

#[test]
fn look_shrine_band_tiles_print_bare_description_without_shrine_appender() {
    // `formats/look2-dat.md §5` / `view.md §3` rows 5a..5c: only
    // `0xFA`/`0xFB`, `0xDE` and `0xDF` have appenders, and
    // `catalogs/gazetteer.md §7` denies that `0x88..0x8F` encodes the
    // surface shrines (audit A009).
    let table = parse_look2_dat(&look2_bytes(&[
        (SHRINE_ALTAR_TILE_FIRST as usize, "an altar"),
        (SHRINE_ALTAR_TILE_LAST as usize, "an altar"),
        (0x19, "a shrine"),
    ]))
    .unwrap();
    let state = test_state(open_grid(), 1, 1);

    assert_eq!(
        state.look_description(SHRINE_ALTAR_TILE_FIRST, Some(&table), 0),
        "an altar"
    );
    assert_eq!(
        state.look_description(SHRINE_ALTAR_TILE_LAST, Some(&table), 0),
        "an altar"
    );
    assert_eq!(state.look_description(0x19, Some(&table), 81), "a shrine");
}

#[test]
fn look_sentinel_rows_print_the_selected_string_not_a_range_class() {
    // `formats/look2-dat.md §5` (audit A007): "Use the offset table
    // directly; do not derive descriptions from tile or object ranges."
    // Terrain `0xC0` and object `0x12`/`0x74` are sentinel rows; each
    // prints its own domain's selected sentinel string.
    let table = parse_look2_dat(&look2_bytes(&[
        (LOOK2_DAT_TERRAIN_ENTRIES, "x"),
        (LOOK2_DAT_TERRAIN_ENTRIES + 0x12, "x"),
        (LOOK2_DAT_TERRAIN_ENTRIES + 0x74, "x"),
    ]))
    .unwrap();
    let state = test_state(open_grid(), 1, 1);

    assert_eq!(state.look_description(0xc0, Some(&table), 0), "*");
    assert_eq!(state.look_object_description(0x12, Some(&table)), "x");
    assert_eq!(state.look_object_description(0x74, Some(&table)), "x");
}

#[test]
fn look_shrine_altar_tiles_take_no_appender() {
    // `view.md §3` rows 5a-5c: only `0xFA`/`0xFB`, `0xDE` and `0xDF`
    // append anything; the former `(Shrine of X)` suffix is not published.
    let table = parse_look2_dat(&look2_bytes(&[(
        SHRINE_ALTAR_TILE_FIRST as usize,
        "an altar",
    )]))
    .unwrap();
    let state = test_state(open_grid(), 1, 1);

    assert_eq!(
        state.look_description(SHRINE_ALTAR_TILE_FIRST, Some(&table), 0),
        "an altar"
    );
    // `formats/look2-dat.md §5` (audit A007): with no table there is no
    // tile-range class to fall back on.
    assert_eq!(state.look_description(SHRINE_ALTAR_TILE_LAST, None, 0), "");
}

#[test]
fn world_look_wraps_and_reports_facing_object_without_spending_turn() {
    let mut state = world_state(open_world_grid(), 255, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 170,
        tile: 170,
        x: 0,
        y: 0,
        z: -1,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.look_facing(), MoveOutcome::Observed);

    assert!(state.message.contains("object tile 170"));
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn world_look_visibility_gate_hides_dark_unlit_wrapped_object() {
    let mut hidden = world_state(open_world_grid(), 255, 0);
    hidden.player.facing = Direction::East;
    // `visibility.md §4`: zero light is the pitch-dark branch — the
    // carve is skipped and even the adjacent cell stays hidden.
    // `FULL_DARKNESS` (2) is a threshold that still lights it.
    hidden.ambient_light = 0;
    hidden.message = "previous".to_string();
    hidden.active_objects.push(ActiveObject {
        type_byte: 170,
        tile: 170,
        x: 0,
        y: 0,
        z: -1,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(hidden.look_facing(), MoveOutcome::Observed);

    assert!(hidden.message.is_empty());
    assert_eq!(hidden.turn, 0);

    let mut lit = world_state(open_world_grid(), 255, 0);
    lit.player.facing = Direction::East;
    lit.ambient_light = FULL_DARKNESS;
    lit.torch_counter = 1;
    lit.recompute_daylight();
    lit.active_objects.push(ActiveObject {
        type_byte: 170,
        tile: 170,
        x: 0,
        y: 0,
        z: -1,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(lit.look_facing(), MoveOutcome::Observed);

    assert!(lit.message.contains("object tile 170"));
    assert_eq!(lit.turn, 0);
}

#[test]
fn world_look_uses_look2_description_for_wrapped_object() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(LOOK2_DAT_FILE),
        look2_bytes(&[
            (170, "terrain frigate"),
            (LOOK2_DAT_TERRAIN_ENTRIES + 170, "object frigate"),
        ]),
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 255, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 170,
        tile: 170,
        x: 0,
        y: 0,
        z: -1,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert!(state.message.contains("object frigate"));
    assert!(!state.message.contains("terrain frigate"));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
}

#[test]
fn world_look_dungeon_mouth_appends_name_keyed_on_target_x() {
    // `view.md §3` row 5c (audit A008): the name is chosen by the target
    // cell's X alone, with no location-table row and no game dir needed.
    let table = parse_look2_dat(&look2_bytes(&[(0xdf, "a dungeon mouth")])).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(126, 1)] = 0xdf;
    grid[world_cell_index(240, 1)] = 0xdf;
    grid[world_cell_index(3, 1)] = 0xdf;

    let mut state = britannia_state(grid.clone(), 125, 1);
    state.player.facing = Direction::East;
    state
        .look_direction_with_resources(Direction::East, Some(&table), None)
        .unwrap();
    assert!(state.message.ends_with("\na dungeon mouth Wrong"), "{:?}", state.message);

    let mut state = britannia_state(grid.clone(), 239, 1);
    state
        .look_direction_with_resources(Direction::East, Some(&table), None)
        .unwrap();
    assert!(state.message.ends_with("\na dungeon mouth Deceit"), "{:?}", state.message);

    let mut state = britannia_state(grid, 2, 1);
    state
        .look_direction_with_resources(Direction::East, Some(&table), None)
        .unwrap();
    assert!(state.message.ends_with("\na dungeon mouth"), "{:?}", state.message);

    // The same arm serves town Look.
    let state = test_state(open_grid(), 1, 1);
    assert_eq!(
        state.look_description(0xdf, Some(&table), 58),
        "a dungeon mouth Shame"
    );
    assert_eq!(state.look_description(0xdf, Some(&table), 59), "a dungeon mouth");
}

#[test]
fn world_look_at_shrine_table_cell_prints_bare_description() {
    // `view.md §3` rows 5a..5c have no shrine appender (audit A009): a
    // shrine-table coordinate prints its plain `LOOK2.DAT` string.
    let dir = debug_game_dir();
    fs::write(dir.join(LOOK2_DAT_FILE), look2_bytes(&[(0x19, "a shrine")])).unwrap();
    fs::write(
        dir.join(SHRINE_TABLE_FILE),
        "BRITANNIA 2 1 COMPASSION 0x19\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = 0x19;
    let mut state = britannia_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert!(state.message.ends_with("\na shrine"), "{:?}", state.message);
    assert!(!state.message.contains("Shrine of"));
    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\na shrine"));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_look_shrine_altar_appends_no_virtue_context() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(LOOK2_DAT_FILE),
        look2_bytes(&[(SHRINE_ALTAR_TILE_FIRST as usize, "an altar")]),
    )
    .unwrap();
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 2 1 HONESTY 0x88\n").unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = SHRINE_ALTAR_TILE_FIRST;
    let mut state = britannia_state(grid, 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.look_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Observed
    );

    assert_eq!(state.message, format!("{LOOK_RESULT_PREFIX}\nan altar"));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn overworld_talk_reports_no_response_without_tlk_lookup_or_turn() {
    let mut state = britannia_state(open_world_grid(), 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state
            .talk_facing_with_game_dir(Path::new(r"C:\missing-u5-clean-room-test"))
            .unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "Funny, no response!");
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
}

#[test]
fn town_talk_reports_facing_npc_envelope_and_consumes_turn() {
    let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
        2,
        &["Ada", "a test smith", "Greetings", "JOB", "Bye"],
    )]))
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    // The greeting is the envelope's own text: description, greeting and
    // the `Your interest?` prompt, with no engine-composed speaker prefix.
    assert!(state.message.contains("Greetings"));
    assert!(state.message.contains("Your interest?"));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
}

#[test]
fn parsed_tlk_gives_every_id_its_own_blob_including_id_one() {
    // There is no sentinel row and no id-1 alias: each header row's id
    // addresses its own blob. Confirmed against the shipped files, whose
    // ids run exactly 1..=count.
    let dialogue = parse_tlk_bytes(&tlk_bytes(&[
        (
            1,
            &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
        ),
        (2, &["Bry", "a test baker", "Hello", "I bake bread", "Bye"]),
    ]))
    .unwrap();

    assert_eq!(dialogue.len(), 2);
    assert_eq!(dialogue[&1][0], "Ada");
    assert_eq!(dialogue[&2][0], "Bry");
    assert_ne!(dialogue.get(&1), dialogue.get(&2));
}

#[test]
fn parsed_tlk_caps_each_blob_to_runtime_window() {
    let long_name = "A".repeat(1100);
    let bytes = tlk_bytes(&[(2, &[long_name.as_str()])]);
    let dialogue = parse_tlk_bytes(&bytes).unwrap();

    assert_eq!(dialogue[&2][0].len(), 1024);
}

/// `formats/tlk.md §7`: the loaded 1,024-byte window bounds a blob, not a
/// record count. The withdrawn "between two and twenty-six keyword/response
/// pairs" (issue #299 Q22, `RETRACTIONS.md` R582) sized a fifty-seven-field
/// cap that cut off the labelled blocks at the end of a long blob.
#[test]
fn parsed_tlk_keeps_every_record_inside_the_window() {
    let names: Vec<String> = (0..60).map(|index| format!("R{index}")).collect();
    let fields: Vec<&str> = names.iter().map(String::as_str).collect();
    let bytes = tlk_bytes(&[(2, &fields)]);

    let decoded = parse_tlk_bytes(&bytes).unwrap();
    let raw = parse_tlk_blob_fields_raw(&bytes).unwrap();
    assert_eq!(decoded[&2].len(), 60);
    assert_eq!(decoded[&2][59], "R59");
    assert_eq!(raw[&2].len(), 60);
}

#[test]
fn shipped_tlk_corpus_parses_and_runner_smokes_sanitized_fields() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let game_dir = game_dir.as_path();
    if !game_dir.join(TOWNE_TLK_FILENAME).exists() {
        return;
    }

    let corpora = [
        (TOWNE_TLK_FILENAME, TlkFileClass::Towne.shipped_npc_count()),
        (
            DWELLING_TLK_FILENAME,
            TlkFileClass::Dwelling.shipped_npc_count(),
        ),
        (
            CASTLE_TLK_FILENAME,
            TlkFileClass::Castle.shipped_npc_count(),
        ),
        (KEEP_TLK_FILENAME, TlkFileClass::Keep.shipped_npc_count()),
    ];
    let inputs = crate::tlk_runner::TlkRunInputs {
        avatar_name: "Avatar",
        moral_standing: 99,
        dictionary: Some(&PUBLISHED_COMMON_WORD_DICTIONARY),
        gold_available: Some(9999),
        ..Default::default()
    };
    let mut total_npcs = 0usize;
    let mut total_fields = 0usize;
    let mut dictionary_fields = 0usize;
    let mut control_counts = [(0u8, 0usize); 7];
    for (slot, code) in [0x84, 0x85, 0x86, 0x87, 0x88, 0x8c, 0xfe]
        .into_iter()
        .enumerate()
    {
        control_counts[slot].0 = code;
    }

    for (file_name, expected_npcs) in corpora {
        let bytes = fs::read(game_dir.join(file_name)).unwrap();
        let decoded = parse_tlk_bytes(&bytes).unwrap();
        let raw = parse_tlk_blob_fields_raw(&bytes).unwrap();

        assert_eq!(decoded.len(), expected_npcs, "{file_name}");
        assert_eq!(raw.len(), expected_npcs, "{file_name}");
        total_npcs += raw.len();

        for npc_id in 1..=expected_npcs as u16 {
            let fields = raw
                .get(&npc_id)
                .unwrap_or_else(|| panic!("{file_name} missing NPC id {npc_id}"));
            assert!(
                fields.len() >= 5,
                "{file_name} NPC id {npc_id} has too few fields"
            );
            total_fields += fields.len();

            if tlk_fields_use_common_word_dictionary(fields) {
                dictionary_fields += 1;
            }

            for field in fields {
                for byte in field.iter().copied() {
                    if let Some((_, count)) =
                        control_counts.iter_mut().find(|(code, _)| *code == byte)
                    {
                        *count += 1;
                    }
                }

                let output = crate::tlk_runner::run_tlk_stream(field, &inputs);
                assert!(
                    !matches!(
                        output.stop,
                        crate::tlk_runner::TlkRunStop::MalformedIntroducer(_)
                    ),
                    "{file_name} NPC id {npc_id} stopped at {:?}",
                    output.stop
                );
                assert!(
                    !output.text.contains("[w"),
                    "{file_name} NPC id {npc_id} left an unresolved dictionary token"
                );
            }
        }
    }

    assert_eq!(
        total_npcs,
        TOWNE_TLK_NPCS + DWELLING_TLK_NPCS + CASTLE_TLK_NPCS + KEEP_TLK_NPCS
    );
    assert!(total_fields > total_npcs * 5);
    assert!(dictionary_fields > 0);
    assert!(control_counts.iter().any(|(_, count)| *count > 0));
}

#[test]
fn shipped_tlk_corpus_contains_public_action_payment_and_flag_controls() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let game_dir = game_dir.as_path();
    if !game_dir.join(TOWNE_TLK_FILENAME).exists() {
        return;
    }

    let corpora = [
        (TOWNE_TLK_FILENAME, TlkFileClass::Towne.shipped_npc_count()),
        (
            DWELLING_TLK_FILENAME,
            TlkFileClass::Dwelling.shipped_npc_count(),
        ),
        (
            CASTLE_TLK_FILENAME,
            TlkFileClass::Castle.shipped_npc_count(),
        ),
        (KEEP_TLK_FILENAME, TlkFileClass::Keep.shipped_npc_count()),
    ];
    let inputs = crate::tlk_runner::TlkRunInputs {
        avatar_name: "Avatar",
        moral_standing: 99,
        dictionary: Some(&PUBLISHED_COMMON_WORD_DICTIONARY),
        gold_available: Some(9999),
        ..Default::default()
    };

    let mut action_arg_counts = [0usize; 11];
    let mut surfaced_action_count = 0usize;
    let mut gold_payment_controls = 0usize;
    let mut surfaced_gold_payments = 0usize;
    let mut keyword_alias_controls = 0usize;
    let mut if_else_controls = 0usize;
    let mut recruit_speaker_controls = 0usize;

    for (file_name, expected_npcs) in corpora {
        let bytes = fs::read(game_dir.join(file_name)).unwrap();
        let raw = parse_tlk_blob_fields_raw(&bytes).unwrap();

        for npc_id in 2..=expected_npcs as u16 {
            let fields = raw
                .get(&npc_id)
                .unwrap_or_else(|| panic!("{file_name} missing NPC id {npc_id}"));
            for field in fields {
                let mut idx = 0usize;
                while idx < field.len() {
                    match field[idx] {
                        TLK_CODE_ACTION_DISPATCH if idx + 1 < field.len() => {
                            let arg = field[idx + 1] & 0x7F;
                            if (b'A'..=b'K').contains(&arg) {
                                action_arg_counts[usize::from(arg - b'A')] += 1;
                            }
                            idx += 2;
                        }
                        TLK_CODE_GOLD_PAYMENT if idx + 3 < field.len() => {
                            gold_payment_controls += 1;
                            idx += 4;
                        }
                        TLK_CODE_KEYWORD_ALIAS => {
                            keyword_alias_controls += 1;
                            idx += 1;
                        }
                        TLK_CODE_RECRUIT_SPEAKER => {
                            recruit_speaker_controls += 1;
                            idx += 1;
                        }
                        TLK_CODE_IF_ELSE if idx + 1 < field.len() => {
                            if_else_controls += 1;
                            idx += 2;
                        }
                        TLK_CODE_IF_ELSE_ALT if idx + 2 < field.len() => {
                            if_else_controls += 1;
                            idx += 3;
                        }
                        _ => idx += 1,
                    }
                }

                let output = crate::tlk_runner::run_tlk_stream(field, &inputs);
                surfaced_action_count += output.action_grants.len();
                surfaced_gold_payments += output
                    .events
                    .iter()
                    .filter(|event| {
                        matches!(event, crate::tlk_runner::TlkRunEvent::GoldPayment { .. })
                    })
                    .count();
            }
        }
    }

    for (letter, description) in [
        (b'A', "food grants"),
        (b'C', "ordinary key grants"),
        (b'F', "Grapple/Klimb gear grants"),
        (b'J', "Black Badge grants"),
        (b'K', "skull/special-key grants"),
    ] {
        let index = usize::from(letter - b'A');
        assert!(
            action_arg_counts[index] > 0,
            "asset TLK corpus should contain public {description}"
        );
    }
    assert!(surfaced_action_count > 0);
    assert!(gold_payment_controls > 0);
    assert!(surfaced_gold_payments > 0);
    assert!(keyword_alias_controls > 0);
    assert!(if_else_controls > 0);
    assert!(recruit_speaker_controls > 0);
}

#[test]
fn shipped_npc_roster_corpus_matches_public_catalog_counts() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let game_dir = game_dir.as_path();
    if !game_dir.join(TOWNE_NPC_FILENAME).exists() {
        return;
    }

    let corpora = [
        (
            TOWNE_NPC_FILENAME,
            TOWNE_TLK_FILENAME,
            1u8..=8u8,
            107usize,
            48usize,
            (31usize, 28usize),
        ),
        (
            DWELLING_NPC_FILENAME,
            DWELLING_TLK_FILENAME,
            9u8..=16u8,
            18usize,
            15usize,
            (3usize, 0usize),
        ),
        (
            CASTLE_NPC_FILENAME,
            CASTLE_TLK_FILENAME,
            17u8..=24u8,
            112usize,
            45usize,
            (42usize, 25usize),
        ),
        (
            KEEP_NPC_FILENAME,
            KEEP_TLK_FILENAME,
            25u8..=32u8,
            88usize,
            32usize,
            (50usize, 6usize),
        ),
    ];
    let expected_tags = [
        0x01, 0x0e, 0x10, 0x11, 0x1b, 0x1e, 0x28, 0x40, 0x44, 0x48, 0x50, 0x54, 0x58, 0x5c, 0x68,
        0x6c, 0x70, 0x78, 0x90, 0x94, 0xb5, 0xb6, 0xb8, 0xd8, 0xfc,
    ];

    let mut total_occupied = 0usize;
    let mut total_named = 0usize;
    let mut total_dialog_zero = 0usize;
    let mut total_high_special = 0usize;
    let mut distinct_tags = Vec::<u8>::new();

    for (
        npc_file,
        tlk_file,
        scenes,
        expected_occupied,
        expected_named,
        (expected_zero, expected_high),
    ) in corpora
    {
        let npc_len = fs::metadata(game_dir.join(npc_file)).unwrap().len() as usize;
        assert_eq!(npc_len, NPC_FILE_LEN, "{npc_file}");

        let tlk = parse_tlk(&game_dir.join(tlk_file)).unwrap();

        let mut family_occupied = 0usize;
        let mut family_named = 0usize;
        let mut family_dialog_zero = 0usize;
        let mut family_high_special = 0usize;

        for scene_byte in scenes {
            let scene = Scene::new(scene_byte).unwrap();
            assert_eq!(npc_roster_filename(scene_byte), Some(npc_file));
            assert_eq!(npc_tlk_filename(scene_byte), Some(tlk_file));

            let slots = parse_npc_block(game_dir, scene, &tlk).unwrap();
            assert_eq!(slots.len(), NPC_SLOTS_PER_SUB_MAP);
            assert_eq!(slots[NPC_SENTINEL_SLOT].slot, NPC_SENTINEL_SLOT);

            for slot in slots.iter().skip(1) {
                assert!(slot.slot < NPC_SLOTS_PER_SUB_MAP);
                let occupied = npc_type_byte_occupied(slot.type_byte);
                assert_eq!(
                    occupied,
                    npc_type_byte_class(slot.type_byte) != NpcTypeByteClass::Empty
                );
                if !occupied {
                    continue;
                }

                family_occupied += 1;
                if !distinct_tags.contains(&slot.type_byte) {
                    distinct_tags.push(slot.type_byte);
                }

                for waypoint in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
                    let ai = slot.schedule[NPC_SCHEDULE_AI_OFFSET + waypoint];
                    let x = slot.schedule[NPC_SCHEDULE_X_OFFSET + waypoint];
                    let y = slot.schedule[NPC_SCHEDULE_Y_OFFSET + waypoint];
                    let z = slot.schedule[NPC_SCHEDULE_Z_OFFSET + waypoint];
                    assert!(
                        npc_ai_behavior(ai).is_some(),
                        "{npc_file} scene {scene_byte} slot {} waypoint {waypoint} has invalid AI byte {ai}",
                        slot.slot
                    );
                    assert!(
                        x < TOWN_GRID_SIDE as u8 && y < TOWN_GRID_SIDE as u8,
                        "{npc_file} scene {scene_byte} slot {} waypoint {waypoint} has out-of-grid coordinate ({x},{y})",
                        slot.slot
                    );
                    assert!(
                        z <= 7 || z == u8::MAX,
                        "{npc_file} scene {scene_byte} slot {} waypoint {waypoint} has unexpected floor byte {z}",
                        slot.slot
                    );
                }

                for boundary in 0..NPC_SCHEDULE_TIME_BOUNDARY_COUNT {
                    let hour = slot.schedule[NPC_SCHEDULE_TIME_OFFSET + boundary];
                    assert!(
                        hour < 24,
                        "{npc_file} scene {scene_byte} slot {} has out-of-day boundary {hour}",
                        slot.slot
                    );
                }

                let time = [
                    slot.schedule[NPC_SCHEDULE_TIME_OFFSET],
                    slot.schedule[NPC_SCHEDULE_TIME_OFFSET + 1],
                    slot.schedule[NPC_SCHEDULE_TIME_OFFSET + 2],
                    slot.schedule[NPC_SCHEDULE_TIME_OFFSET + 3],
                ];
                for hour in 0..24 {
                    assert!(npc_schedule_waypoint_for_hour(time, hour) < 3);
                }

                match npc_dialog_id_kind(slot.dialog_id) {
                    NpcDialogIdKind::NoDialogue => family_dialog_zero += 1,
                    NpcDialogIdKind::OrdinaryBlobId => {
                        assert!(
                            tlk.contains_key(&(slot.dialog_id as u16)),
                            "{npc_file} scene {scene_byte} slot {} references missing TLK id {}",
                            slot.slot,
                            slot.dialog_id
                        );
                        family_named += 1;
                    }
                    NpcDialogIdKind::HighSpecial => {
                        assert!(
                            slot.dialog_id == NPC_DIALOG_ID_HIGH_FALLBACK
                                || npc_shop_trigger(slot.dialog_id).is_some()
                                || (NPC_DIALOG_ID_HIGH_FIRST..=NPC_DIALOG_ID_HIGH_LAST)
                                    .contains(&slot.dialog_id),
                            "{npc_file} scene {scene_byte} slot {} has unexpected high dialog id {}",
                            slot.slot,
                            slot.dialog_id
                        );
                        family_high_special += 1;
                    }
                }
            }
        }

        assert_eq!(family_occupied, expected_occupied, "{npc_file}");
        assert_eq!(family_named, expected_named, "{npc_file}");
        assert_eq!(family_dialog_zero, expected_zero, "{npc_file}");
        assert_eq!(family_high_special, expected_high, "{npc_file}");

        total_occupied += family_occupied;
        total_named += family_named;
        total_dialog_zero += family_dialog_zero;
        total_high_special += family_high_special;
    }

    distinct_tags.sort_unstable();
    assert_eq!(distinct_tags, expected_tags);
    assert_eq!(total_occupied, 325);
    // formats/npc.md §7: "Dialog index `1` is **not** reserved. It
    // addresses an ordinary authored blob like any other id, and
    // exactly one occupied roster slot in each of the four class
    // files carries it: `TOWNE:0` slot 3, `DWELLING:0` slot 1,
    // `CASTLE:0` slot 13, and `KEEP:0` slot 1." Those four slots
    // are counted as ordinary named speakers (and asserted to
    // resolve to a real `.TLK` record), so the named total is 140,
    // not 136 plus a reserved bucket of 4.
    assert_eq!(total_named, 140);
    assert_eq!(total_dialog_zero, 126);
    assert_eq!(total_high_special, 59);
    assert_eq!(
        total_named + total_dialog_zero + total_high_special,
        total_occupied
    );
}

#[test]
fn town_talk_dialog_id_one_reads_its_own_blob() {
    // Dialog id 1 is an ordinary NPC with its own blob, not an alias onto
    // the first surviving entry.
    let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
        1,
        &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
    )]))
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 1,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("Your interest?"));
    assert_eq!(state.turn, 1);
}

#[test]
fn talk_shop_trigger_maps_public_shop_roles() {
    assert_eq!(
        talk_shop_trigger(0x81),
        Some(("Weaponsmith / armourer", "Arms stock arm"))
    );
    assert_eq!(
        talk_shop_trigger(0x84),
        Some(("Ship broker / shipwright", "Shipwright sale arm"))
    );
    assert_eq!(talk_shop_trigger(0xff), None);
}

/// `conversation.md §6` steps 4 and 5 (u5-spec#299 Q20, R577 / R578): a
/// word-start first occurrence with nothing after it checked.
#[test]
fn talk_keyword_match_is_the_word_start_rule() {
    assert!(talk_keyword_matches("JOB", "job"));
    assert!(talk_keyword_matches("JOB", "job news"));
    assert!(talk_keyword_matches("JOB", "jobber"));
    assert!(talk_keyword_matches("JOB", "my job"));
    assert!(!talk_keyword_matches("JOB", "nojob"));
    assert!(talk_keyword_matches("WHO ART THOU", "who art thou friend"));
    assert!(talk_keyword_matches("WHO", "whom"));
}

#[test]
fn talk_keyword_response_resolves_reserved_aliases_and_pairs() {
    let fields = vec![
        "Ada".to_string(),
        "a test smith".to_string(),
        "Greetings".to_string(),
        "I mend gear".to_string(),
        "Farewell".to_string(),
        "GRAN".to_string(),
        "Short answer".to_string(),
        "GRANDPA".to_string(),
        "Long answer".to_string(),
    ];

    assert_eq!(talk_keyword_response(&fields, "name"), Some("Ada"));
    assert_eq!(talk_keyword_response(&fields, "job"), Some("I mend gear"));
    assert_eq!(talk_keyword_response(&fields, "work"), Some("I mend gear"));
    assert_eq!(talk_keyword_response(&fields, "bye"), Some("Farewell"));
    assert_eq!(talk_keyword_response(&fields, "thank"), Some("Farewell"));
    // u5-spec#299 Q20 (R577): `grandpa` matches `gran`, and the earlier
    // keyword wins.
    assert_eq!(
        talk_keyword_response(&fields, "grandpa"),
        Some("Short answer")
    );
    assert_eq!(
        talk_keyword_response(&fields, "gran news"),
        Some("Short answer")
    );
    assert_eq!(talk_keyword_response(&fields, "granite"), Some("Short answer"));
    assert_eq!(talk_keyword_response(&fields, "migrant"), None);
}

#[test]
fn resolve_keyword_response_field_index_matches_reserved_and_pair_keywords() {
    let fields = vec![
        "Ada".to_string(),
        "smith".to_string(),
        "Greetings".to_string(),
        "I mend gear".to_string(),
        "Farewell".to_string(),
        "GRAN".to_string(),
        "Short answer".to_string(),
        "GRANDPA".to_string(),
        "Long answer".to_string(),
    ];

    assert_eq!(
        resolve_keyword_response_field_index(&fields, "name"),
        Some(0)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "job"),
        Some(3)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "work"),
        Some(3)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "bye"),
        Some(4)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "thank"),
        Some(4)
    );
    // u5-spec#299 Q20 (R577): the earlier `gran` answers `grandpa`.
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "grandpa"),
        Some(6)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "gran news"),
        Some(6)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "granite"),
        Some(6)
    );
    assert_eq!(
        resolve_keyword_response_field_index(&fields, "migrant"),
        None
    );
}

#[test]
fn parse_tlk_blob_fields_raw_round_trips_a_minimal_blob() {
    // Synthetic minimal TLK in the shipped shape: a two-byte count then
    // that many four-byte `(npc id, blob offset)` rows. No sentinel.
    let blob_offset: u16 = 6;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u16.to_le_bytes()); // count
    bytes.extend_from_slice(&0x0042u16.to_le_bytes()); // npc id 0x42
    bytes.extend_from_slice(&blob_offset.to_le_bytes()); // its blob offset
    // Two fields: "Ada\0" then "smith\0" (XOR-encoded).
    let xor = 0x80u8;
    let field_a = b"Ada";
    let field_b = b"smith";
    for b in field_a {
        bytes.push(*b ^ xor);
    }
    bytes.push(0);
    for b in field_b {
        bytes.push(*b ^ xor);
    }
    bytes.push(0);

    let parsed = parse_tlk_blob_fields_raw(&bytes).unwrap();
    let fields = parsed.get(&0x0042).expect("npc 0x42 missing");
    assert!(fields.len() >= 2);
    // Each field's bytes are still XOR-encoded; running through the
    // byte-runner produces "Ada" and "smith".
    let inputs = crate::tlk_runner::TlkRunInputs {
        avatar_name: "X",
        ..Default::default()
    };
    let out0 = crate::tlk_runner::run_tlk_stream(&fields[0], &inputs);
    let out1 = crate::tlk_runner::run_tlk_stream(&fields[1], &inputs);
    assert_eq!(out0.text, "Ada");
    assert_eq!(out1.text, "smith");
}

#[test]
fn parse_tlk_rejects_malformed_headers() {
    // A zero count is a legal empty file; a count that overruns the file
    // is not.
    assert!(parse_tlk_bytes(&[0, 0]).unwrap().is_empty());
    let mut count_overruns = Vec::new();
    count_overruns.extend_from_slice(&1u16.to_le_bytes());
    count_overruns.extend_from_slice(&1u16.to_le_bytes());
    assert!(parse_tlk_bytes(&count_overruns).is_err());
    assert!(parse_tlk_bytes(&[0]).is_err());

    let mut bad_sentinel = Vec::new();
    bad_sentinel.extend_from_slice(&2u16.to_le_bytes());
    bad_sentinel.extend_from_slice(&0u16.to_le_bytes());
    bad_sentinel.extend_from_slice(&8u16.to_le_bytes());
    bad_sentinel.extend_from_slice(&2u16.to_le_bytes());
    bad_sentinel.push(0);
    assert!(parse_tlk_bytes(&bad_sentinel).is_err());

    let mut bad_offset = Vec::new();
    bad_offset.extend_from_slice(&2u16.to_le_bytes());
    bad_offset.extend_from_slice(&1u16.to_le_bytes());
    bad_offset.extend_from_slice(&4u16.to_le_bytes());
    bad_offset.extend_from_slice(&2u16.to_le_bytes());
    bad_offset.push(0);
    assert!(parse_tlk_bytes(&bad_offset).is_err());

    let mut unsorted_ids = Vec::new();
    unsorted_ids.extend_from_slice(&3u16.to_le_bytes());
    unsorted_ids.extend_from_slice(&1u16.to_le_bytes());
    unsorted_ids.extend_from_slice(&12u16.to_le_bytes());
    unsorted_ids.extend_from_slice(&3u16.to_le_bytes());
    unsorted_ids.extend_from_slice(&13u16.to_le_bytes());
    unsorted_ids.extend_from_slice(&2u16.to_le_bytes());
    unsorted_ids.extend_from_slice(&[0, 0]);
    assert!(parse_tlk_blob_fields_raw(&unsorted_ids).is_err());
}

#[test]
fn talk_response_text_and_actions_strips_action_markers() {
    assert_eq!(
        talk_response_text_and_actions("Take this {ACTION:F} friend"),
        ("Take this friend".to_string(), vec!['F'])
    );
}

#[test]
fn talk_branch_flags_use_32_bit_scene_slot_and_zero_mask_out_of_range() {
    assert_eq!(talk_branch_flag_mask(0), 1);
    assert_eq!(talk_branch_flag_mask(31), 0x8000_0000);
    assert_eq!(talk_branch_flag_mask(32), 0);
    assert_eq!(talk_branch_flag_mask(255), 0);

    let mut slot = 0u32;
    assert!(!talk_branch_flag_is_set(slot, 5));
    assert!(set_talk_branch_flag(&mut slot, 5));
    assert_eq!(slot, 0x20);
    assert!(talk_branch_flag_is_set(slot, 5));
    assert!(!set_talk_branch_flag(&mut slot, 5));
    assert_eq!(slot, 0x20);

    assert!(!set_talk_branch_flag(&mut slot, 32));
    assert_eq!(slot, 0x20);
    assert!(!talk_branch_flag_is_set(0xffff_ffff, 32));
}

#[test]
fn play_state_keeps_talk_branch_flags_per_town_scene() {
    let mut state = test_state(open_grid(), 1, 1);
    let first_scene = match state.area {
        Area::Town { scene, .. } => scene,
        _ => unreachable!(),
    };
    let second_scene = Scene::new(first_scene.byte + 1).unwrap();

    assert_eq!(state.talk_branch_slot_for_scene(first_scene), 0);
    assert!(!state.active_talk_branch_flag_is_set(3));
    assert!(state.set_active_talk_branch_flag(3));
    assert!(!state.set_active_talk_branch_flag(3));
    assert!(state.active_talk_branch_flag_is_set(3));
    assert_eq!(state.talk_branch_slot_for_scene(first_scene), 0x08);
    assert_eq!(state.talk_branch_slot_for_scene(second_scene), 0);

    assert!(state.set_talk_branch_flag_for_scene(second_scene, 5));
    assert_eq!(state.talk_branch_slot_for_scene(first_scene), 0x08);
    assert_eq!(state.talk_branch_slot_for_scene(second_scene), 0x20);

    state.area = Area::Town {
        scene: second_scene,
        floor: 0,
    };
    assert!(!state.active_talk_branch_flag_is_set(3));
    assert!(state.active_talk_branch_flag_is_set(5));
}

#[test]
fn play_state_talk_branch_flags_ignore_non_town_and_out_of_range_bits() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

    assert!(!state.active_talk_branch_flag_is_set(0));
    assert!(!state.set_active_talk_branch_flag(0));

    let scene = Scene::new(0x11).unwrap();
    assert!(!state.set_talk_branch_flag_for_scene(scene, 32));
    assert_eq!(state.talk_branch_slot_for_scene(scene), 0);
}

#[test]
fn town_talk_inline_keyword_uses_decoded_tlk_response() {
    let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
        2,
        &[
            "Ada",
            "a test smith",
            "Greetings",
            "I mend gear",
            "Bye",
            "TRADE",
            "Bring iron",
        ],
    )]))
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword(&dialogue, Some("trade now")),
        MoveOutcome::Talked
    );

    assert_eq!(state.message, "Bring iron");
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
}

#[test]
fn town_talk_action_dispatch_grants_confirmed_special_item_flags() {
    fn push_text(bytes: &mut Vec<u8>, text: &str) {
        for byte in text.bytes() {
            bytes.push(byte | 0x80);
        }
        bytes.push(0);
    }

    // Shipped header shape: count word, then one (npc id, blob offset)
    // row. No sentinel.
    let mut bytes = vec![0; 6];
    bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
    bytes[2..4].copy_from_slice(&2u16.to_le_bytes());
    bytes[4..6].copy_from_slice(&6u16.to_le_bytes());
    for field in [
        "Ada",
        "a test smith",
        "Greetings",
        "I mend gear",
        "Bye",
        "GIFT",
    ] {
        push_text(&mut bytes, field);
    }
    push_text(&mut bytes, "Take this");
    let terminator = bytes.pop().unwrap();
    assert_eq!(terminator, 0);
    bytes.push(0x86);
    bytes.push(b'F' | 0x80);
    bytes.push(0x86);
    bytes.push(b'H' | 0x80);
    bytes.push(0x86);
    bytes.push(b'I' | 0x80);
    bytes.push(0x86);
    bytes.push(b'J' | 0x80);
    bytes.push(0);

    let dialogue = parse_tlk_bytes(&bytes).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.climbing_gear = 0;
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword(&dialogue, Some("gift")),
        MoveOutcome::Talked
    );

    assert_eq!(state.climbing_gear, 1);
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
    assert_eq!(state.message, "Take this");
}

#[test]
fn tlk_action_dispatch_grants_use_published_caps_and_slots() {
    let mut state = test_state(open_grid(), 1, 1);
    state.food = PARTY_FOOD_CAP;
    state.gold = PARTY_GOLD_CAP;
    state.keys = PARTY_BYTE_STOCK_CAP;
    state.gems = PARTY_BYTE_STOCK_CAP;
    state.torches = PARTY_BYTE_STOCK_CAP;
    state.climbing_gear = 0;
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = PARTY_BYTE_STOCK_CAP;
    state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = PARTY_BYTE_STOCK_CAP;
    state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 0;
    state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 0;
    state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = 0;

    state.apply_tlk_action_grants(&[
        TlkActionDispatchVerb::RaiseFood,
        TlkActionDispatchVerb::RaiseGold,
        TlkActionDispatchVerb::RaiseKeys,
        TlkActionDispatchVerb::RaiseGems,
        TlkActionDispatchVerb::RaiseTorches,
        TlkActionDispatchVerb::SetGrappleGate,
        TlkActionDispatchVerb::RaiseCarpets,
        TlkActionDispatchVerb::SetSextantCarried,
        TlkActionDispatchVerb::SetSpyglassCarried,
        TlkActionDispatchVerb::SetBlackBadgeCarried,
        TlkActionDispatchVerb::RaiseSkullKeys,
    ]);

    assert_eq!(state.food, PARTY_FOOD_CAP);
    assert_eq!(state.gold, PARTY_GOLD_CAP);
    assert_eq!(state.keys, PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.gems, PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.torches, PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.climbing_gear, 1);
    assert_eq!(
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
        PARTY_BYTE_STOCK_CAP
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX],
        PARTY_BYTE_STOCK_CAP
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
}

#[test]
fn object_pickup_parser_accepts_extended_inventory_grants() {
    let entries = parse_object_pickup_entries(
        "CASTLE:0 0 1 2 POTION:3 2 0x42\n\
             CASTLE:0 0 2 2 SCROLL_4 1\n\
             CASTLE:0 0 3 2 EQUIP-27 1\n\
             CASTLE:0 0 4 2 SHARD:2 1\n\
             CASTLE:0 0 5 2 SANDALWOOD_BOX 1\n",
    )
    .unwrap();

    assert_eq!(entries[0].kind, ObjectPickupKind::Potion(3));
    assert_eq!(entries[0].amount, 2);
    assert_eq!(entries[0].expected_tile, Some(0x42));
    assert_eq!(entries[1].kind, ObjectPickupKind::Scroll(4));
    assert_eq!(
        entries[2].kind,
        ObjectPickupKind::Equipment(EQUIPMENT_ID_ARROWS)
    );
    assert_eq!(entries[3].kind, ObjectPickupKind::ShadowlordShard(2));
    assert_eq!(entries[4].kind, ObjectPickupKind::SandalwoodBox);
}

#[test]
fn object_pickup_inventory_grants_cover_caps_equipment_and_story_items() {
    let mut state = test_state(open_grid(), 1, 1);
    state.food = PARTY_FOOD_CAP - 1;
    state.gold = PARTY_GOLD_CAP - 1;
    state.keys = PARTY_BYTE_STOCK_CAP - 1;
    state.gems = PARTY_BYTE_STOCK_CAP - 1;
    state.torches = PARTY_BYTE_STOCK_CAP - 1;
    state.potion_stock[3] = PARTY_BYTE_STOCK_CAP - 1;
    state.scroll_stock[4] = PARTY_BYTE_STOCK_CAP - 1;
    state.equipment_stock[EQUIPMENT_ID_ARROWS] = PARTY_BYTE_STOCK_CAP - 3;
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = PARTY_BYTE_STOCK_CAP - 1;

    state.apply_object_pickup(ObjectPickupKind::Food, 5);
    state.apply_object_pickup(ObjectPickupKind::Gold, 5);
    state.apply_object_pickup(ObjectPickupKind::Keys, 5);
    state.apply_object_pickup(ObjectPickupKind::Gems, 5);
    state.apply_object_pickup(ObjectPickupKind::Torches, 5);
    state.apply_object_pickup(ObjectPickupKind::Potion(3), 5);
    state.apply_object_pickup(ObjectPickupKind::Scroll(4), 5);
    state.apply_object_pickup(ObjectPickupKind::Equipment(EQUIPMENT_ID_ARROWS), 1);
    state.apply_object_pickup(ObjectPickupKind::MagicCarpet, 5);
    state.apply_object_pickup(ObjectPickupKind::SkullKeys, 2);
    state.apply_object_pickup(ObjectPickupKind::HmsCapePlans, 1);
    state.apply_object_pickup(ObjectPickupKind::SandalwoodBox, 1);
    state.apply_object_pickup(ObjectPickupKind::CrownOfLordBritish, 1);
    state.apply_object_pickup(ObjectPickupKind::SceptreOfLordBritish, 1);
    state.apply_object_pickup(ObjectPickupKind::AmuletOfLordBritish, 1);
    state.apply_object_pickup(ObjectPickupKind::ShadowlordShard(2), 1);

    assert_eq!(state.food, PARTY_FOOD_CAP);
    assert_eq!(state.gold, PARTY_GOLD_CAP);
    assert_eq!(state.keys, PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.gems, PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.torches, PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.potion_stock[3], PARTY_BYTE_STOCK_CAP);
    assert_eq!(state.scroll_stock[4], PARTY_BYTE_STOCK_CAP);
    assert_eq!(
        state.equipment_stock[EQUIPMENT_ID_ARROWS],
        PARTY_BYTE_STOCK_CAP
    );
    assert_eq!(
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX],
        PARTY_BYTE_STOCK_CAP
    );
    assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 2);
    assert_eq!(state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX], 1);
    assert_eq!(state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX], 1);
    assert_eq!(state.special_items[SPECIAL_ITEM_CROWN_LB_INDEX], 1);
    assert_eq!(state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX], 1);
    assert_eq!(state.special_items[SPECIAL_ITEM_AMULET_LB_INDEX], 1);
    // containers.md §8 class 0xB4: the shard flag gets the no-quantity
    // marker 255, not a count (playtest F53).
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SHARD_COWARDICE_INDEX],
        crate::commands::USE_PICKER_NO_QUANTITY
    );
}

#[test]
fn play_input_talk_suffix_routes_to_one_shot_keyword_lookup() {
    let dir = debug_game_dir();
    fs::write(
        dir.join("CASTLE.TLK"),
        tlk_bytes(&[(
            2,
            &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
        )]),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        handle_play_key_input(&mut state, 'T', "JOB", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "I mend gear");
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn play_input_talk_suffix_routes_reserved_aliases() {
    let dir = debug_game_dir();
    fs::write(
        dir.join("CASTLE.TLK"),
        tlk_bytes(&[(
            2,
            &[
                "Ada",
                "a test smith",
                "Greetings",
                "I mend gear",
                "Farewell",
            ],
        )]),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        handle_play_key_input(&mut state, 'T', "WORK", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.message, "I mend gear");

    assert_eq!(
        handle_play_key_input(&mut state, 'T', "THANK", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.message, "Farewell");
    assert_eq!(state.turn, 2);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn play_input_talk_without_suffix_opens_keyword_session() {
    let dir = debug_game_dir();
    fs::write(
        dir.join("CASTLE.TLK"),
        tlk_bytes(&[(
            2,
            &[
                "Ada",
                "a test smith",
                "Greetings",
                "I mend gear",
                "Farewell",
            ],
        )]),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        handle_play_key_input(&mut state, 'T', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.message, "Talk-");
    assert!(state.active_direction_prompt.is_some());
    assert!(state.active_conversation.is_none());
    assert_eq!(state.turn, 0);
    state.set_active_talk_branch_flag(1);

    assert_eq!(
        handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.message.contains("Greetings"));
    assert!(state.active_direction_prompt.is_none());
    assert!(state.active_conversation.is_some());
    assert_eq!(state.turn, 1);
    // commands.md section 5.3: the `-` suffix means "a direction is
    // awaited. The chosen direction's name is appended on the same
    // line", so the transcript keeps one `Talk-East` echo rather than a
    // bare `Talk-` plus a separate direction line.
    assert!(
        state
            .message_entries()
            .iter()
            .any(|entry| entry.is_command_echo && entry.text == "Talk-East"),
        "{:?}",
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        handle_play_key_input(&mut state, 'J', "OB", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(transcript_has(&state, "I mend gear"));
    assert_eq!(state.message, TLK_KEYWORD_PROMPT);
    assert!(state.active_conversation.is_some());
    assert_eq!(state.turn, 1);

    assert_eq!(
        handle_play_key_input(&mut state, 'X', "YZZY", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(transcript_has(&state, TLK_NO_KEYWORD_MATCH_MESSAGE.trim()));
    assert_eq!(state.message, TLK_KEYWORD_PROMPT);
    assert!(state.active_conversation.is_some());
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn play_input_conversation_empty_line_emits_bye_envelope_and_closes() {
    let dir = debug_game_dir();
    fs::write(
        dir.join("CASTLE.TLK"),
        tlk_bytes(&[(
            2,
            &[
                "Ada",
                "a test smith",
                "Greetings",
                "I mend gear",
                "Farewell",
            ],
        )]),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    handle_play_key_input(&mut state, 'T', "", &dir).unwrap();
    handle_play_key_input(&mut state, '6', "", &dir).unwrap();
    assert!(state.active_conversation.is_some());

    assert_eq!(
        handle_play_key_input(&mut state, '\n', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    // `conversation.md §6`: the shortcut's word lands on the keyword prompt's
    // own row - measured `:BYE` - so the slot carries only the two feeds that
    // close that row and the Bye entry beneath it.
    assert_eq!(state.message, "\n\"Farewell\"\n\n");
    assert!(
        state
            .message_entries()
            .iter()
            .any(|entry| entry.text.ends_with(TLK_EMPTY_INPUT_BYE_ECHO)),
        "the prompt row carries the echoed word"
    );
    assert!(state.active_conversation.is_none());
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_talk_can_reach_npc_behind_counter_tile() {
    let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
        2,
        &["Ada", "a test smith", "Greetings", "JOB", "Bye"],
    )]))
    .unwrap();
    let mut grid = npc_open_grid();
    // conversation.md §2 step 3: 0x94 is in the published
    // talk-through white-list, so Talk advances one more cell
    // past the counter to reach the NPC behind it.
    grid[1 * 32 + 2] = 0x94;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 3, 3, 3, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    // The greeting is the envelope's own text: description, greeting and
    // the `Your interest?` prompt, with no engine-composed speaker prefix.
    assert!(state.message.contains("Greetings"));
    assert!(state.message.contains("Your interest?"));
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
}

#[test]
fn town_talk_reports_nobody_and_still_spends_the_ordinary_turn() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, TALK_NOBODY_HERE_LINE);
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
}

#[test]
fn town_talk_liveness_gate_blocks_before_lookup_without_printing() {
    let dialogue = parse_tlk_bytes(&tlk_bytes(&[(
        2,
        &["Ada", "a test smith", "Greetings", "I mend gear", "Bye"],
    )]))
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.message = "previous line".to_string();
    state.player.facing = Direction::East;
    state.active_player = Some(0);
    state.party[0].status = b'S';
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: Some("Ada".to_string()),
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "previous line");
    assert_eq!(state.turn, 1);
}

#[test]
fn town_talk_reports_public_shop_trigger_dispatch_family() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(21).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
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
            dialog_id: 0x84,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("The Oaken Oar"));
    assert!(state.active_shop.is_some());
    assert!(!state.message.contains("out of scope"));
    assert_eq!(state.turn, 1);
}

#[test]
fn town_raw_tlk_shop_trigger_opens_active_shop_session() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(21).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
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
            dialog_id: 0x84,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("now open"));
    assert!(matches!(
        state.active_shop,
        Some(crate::shop_session::ActiveShopSession::ShipBroker(_))
    ));
    assert_eq!(state.turn, 1);
}

#[test]
fn talk_shop_entry_uses_shared_preamble_record_when_shoppe_dat_is_loaded() {
    let dir = debug_game_dir();
    let scene = Scene::new(22).unwrap();
    fs::write(
        dir.join(format!("{}.TLK", scene.family.stem())),
        tlk_bytes(&[]),
    )
    .unwrap();
    fs::write(
        dir.join("SHOPPE.DAT"),
        shoppe_dat_with_records(&[(149, b"Guild preamble two.")]),
    )
    .unwrap();

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town { scene, floor: 0 };
    state.player.facing = Direction::East;
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut prng = *seed;
            u5_prng_range_u16(&mut prng, 0, 3) == 1
        })
        .unwrap();
    let mut expected_prng = state.prng_state;
    let _ = u5_prng_range_u16(&mut expected_prng, 0, 3);
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
            dialog_id: 0x86,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Talked
    );

    // `shops.md §8.B`: a non-arms entry "first emit[s] an opening double
    // quote, render[s] one record from their shared entry row, then place[s]
    // the input continuation according to the resulting window-local cursor
    // column". This record wraps to a four-column final row, which takes the
    // one-space continuation rather than a colon. The invented key summary
    // that used to follow the record is gone.
    // `§8.0`'s Talk entry newline opens the row the greeting prints on.
    assert_eq!(state.message, "\n\"Guild preamble two. ");
    assert_eq!(state.prng_state, expected_prng);
    assert!(state.active_shop.is_some());
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_raw_tlk_status_tile_filter_runs_before_shop_dispatch() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();

    for (status_tile, expected) in [
        (TALK_STATUS_TILE_SLEEPING, TALK_SLEEPING_MESSAGE),
        (TALK_STATUS_TILE_PRAYING, TALK_NO_RESPONSE_MESSAGE),
    ] {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
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
                dialog_id: 0x84,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        // conversation.md §2 step 4: "The test object is a map tile, not
        // an NPC sprite ... the byte comes from the same live-map tile
        // query that movement and Look use." The NPC keeps its ordinary
        // sprite; the resolved cell (2, 1) is what carries the mirror or
        // bed byte.
        let object_slot = state.npcs[0]
            .active_object
            .expect("scheduled NPC should link an active object");
        state.active_objects[object_slot].type_byte = 1;
        state.grid[32 + 2] = status_tile;

        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Blocked
        );
        // `text-output.md §10.4`: the refusal prints under the
        // `Talk-<direction>` echo, so it opens with the feed that leaves a
        // blank row between them (`cleak/u5-engine#5`).
        assert_eq!(state.message, format!("\n{expected}"));
        assert!(state.active_shop.is_none());
        assert_eq!(state.turn, 1);
    }
}

#[test]
fn town_talk_status_tile_gate_ignores_the_npc_sprite_byte() {
    // conversation.md §2 step 4: "An implementation that stores a
    // per-NPC 'asleep' flag and tests that instead will diverge." An NPC
    // whose renderer frame happens to equal a status byte, standing on an
    // ordinary floor cell, still reaches shop dispatch.
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x81,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    let object_slot = state.npcs[0]
        .active_object
        .expect("scheduled NPC should link an active object");
    state.active_objects[object_slot].type_byte = 1;
    state.active_objects[object_slot].tile = TALK_STATUS_TILE_SLEEPING;
    assert_eq!(state.grid[32 + 2], open_grid()[32 + 2]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    assert_ne!(state.message, TALK_SLEEPING_MESSAGE);
    assert!(state.active_shop.is_some());
}

#[test]
fn town_raw_tlk_shop_trigger_horseback_refusal_does_not_open_session() {
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

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Blocked
    );

    // shops.md §2 publishes the refusal verbatim as two lines, and it is
    // a fixed merchant line rather than one worded per shop role.
    assert_eq!(
        state.message,
        "A merchant says:\n\"GET THAT HORSE OUT OF HERE!\""
    );
    assert!(state.active_shop.is_none());
    assert_eq!(state.turn, 1);
}

#[test]
fn town_talk_reserved_guard_dialog_opens_default_tribute_demand() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0xFF,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    state.gold = 100;
    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    assert_eq!(state.message, "\nA guard demands a 10 gp tribute to Blackthorn!\n\nDost thou pay?\n\n:");
    assert_eq!(state.turn, 1);
    assert!(matches!(
        state.active_blackthorn_guard_demand,
        Some(ActiveBlackthornGuardDemand {
            prompt: BlackthornGuardDemandPrompt::Tribute { amount: 10 },
            ..
        })
    ));
    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('Y', "", None),
        Some(MoveOutcome::Talked)
    );
    assert_eq!(state.gold, 90);
    assert!(state.pending_town_arrest.is_none());
}

#[test]
fn town_arrest_opener_branches_on_location_before_printing() {
    // `town-mode.md §14`: "The arrest sequence itself branches on the current
    // location **before it prints anything**." Everywhere but Blackthorn's
    // castle it prints the challenge; the castle plays the audience, which
    // needs the game directory, so a caller with no assets keeps the
    // challenge rather than silently dropping the arrest.
    let arrest_at = |scene_byte: u8| crate::TownArrestPrompt {
        scene_byte,
        floor: 0,
        npc_slot: 1,
    };

    let mut state = test_state(open_grid(), 1, 1);
    let outcome = state
        .open_town_arrest(arrest_at(crate::SCENE_MINOC), Some(std::path::Path::new("/")))
        .expect("no assets are read on the challenge branch");
    assert_eq!(outcome, MoveOutcome::Used);
    assert_eq!(state.message, crate::TOWN_ARREST_SURRENDER_PROMPT);
    assert_eq!(
        state.pending_town_arrest.map(|prompt| prompt.scene_byte),
        Some(crate::SCENE_MINOC)
    );

    let mut state = test_state(open_grid(), 1, 1);
    let outcome = state
        .open_town_arrest(
            arrest_at(crate::blackthorn::BLACKTHORN_CAPTIVE_CELL_SCENE),
            None,
        )
        .expect("the no-assets fallback reads nothing either");
    assert_eq!(outcome, MoveOutcome::Used);
    assert_eq!(state.message, crate::TOWN_ARREST_SURRENDER_PROMPT);
    assert!(state.pending_town_arrest.is_some());
}

#[test]
fn town_raw_tlk_reserved_guard_dialog_refusal_requests_arrest_cleanup() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0xFF,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    state.gold = 5;
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    assert_eq!(state.message, "\nA guard demands a 10 gp tribute to Blackthorn!\n\nDost thou pay?\n\n:");
    assert_eq!(state.turn, 1);
    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('Y', "", None),
        Some(MoveOutcome::Used)
    );
    assert_eq!(state.gold, 5);
    assert!(state.active_blackthorn_guard_demand.is_none());
    assert_eq!(state.pending_town_arrest.unwrap().npc_slot, 1);
}

#[test]
fn blackthorn_palace_guard_requires_active_badge_code_and_accepts_four_letter_prefix() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(SCENE_LORD_BLACKTHORNS_CASTLE).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
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
            dialog_id: BLACKTHORN_GUARD_DEMAND_DIALOG_ID,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Used
    );
    assert!(state.pending_town_arrest.is_some());

    state.pending_town_arrest = None;
    state.active_effect_tag = Some(BLACK_BADGE_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = PERMANENT_ACTIVE_EFFECT_DURATION;
    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert!(matches!(
        state.active_blackthorn_guard_demand,
        Some(ActiveBlackthornGuardDemand {
            prompt: BlackthornGuardDemandPrompt::PalacePassword,
            ..
        })
    ));
    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('i', "mpeachment", None),
        Some(MoveOutcome::Talked)
    );
    assert_eq!(state.message, "\"Pass, friend!\"");
    assert!(state.pending_town_arrest.is_none());
}

#[test]
fn minoc_guard_charity_halves_gold_on_yes() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(SCENE_MINOC).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 101;
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
            dialog_id: BLACKTHORN_GUARD_DEMAND_DIALOG_ID,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('y', "", None),
        Some(MoveOutcome::Talked)
    );
    assert_eq!(state.gold, 50);
}

#[test]
fn town_raw_tlk_no_keyword_opens_runner_backed_conversation_session() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "GIFT".to_string(),
            "Take this gift".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut gift_response = enc("Take this gift");
    gift_response.push(0x86);
    gift_response.push(b'H' | 0x80);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("GIFT"),
            gift_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.set_active_talk_branch_flag(1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("You see a quiet sage"));
    assert!(state.message.contains("Greetings"));
    assert!(state.message.ends_with(TLK_KEYWORD_PROMPT));
    assert!(state.active_conversation.is_some());
    assert_eq!(state.turn, 1);

    let (text, ended) = state.submit_active_conversation_keyword("gift");
    assert_eq!(text, conversation_framed_response("Take this gift"));
    assert!(!ended);
    assert_eq!(
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX],
        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
    );
}

#[test]
fn town_raw_tlk_opening_runs_description_stream_before_greeting() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut description = enc("a sage watching ");
    description.push(TLK_CODE_PRINT_AVATAR_NAME);
    description.push(TLK_CODE_ACTION_DISPATCH);
    description.push(6);
    // The Description runs to its NUL. It used to end on `0xFF`, but that
    // opens the keyword prompt inside the Description and the greeting never
    // runs (u5-spec#299 Q1, `conversation.md §7.2`, R561).
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            description,
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.set_active_talk_branch_flag(1);
    state.party_names = vec![*b"AVATAR\0\0\0"];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("You see a sage watching AVATAR"));
    assert!(state.message.contains("\"Greetings\"\n\nYour interest?\n:"));
    assert_eq!(state.conversation_signal_flags[6], 1);
    assert!(state.active_conversation.is_some());
}

#[test]
fn active_conversation_recruit_speaker_runs_inline_without_a_name_prompt() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "JOIN".to_string(),
            "I shall come.".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut join_response = enc("I shall come.");
    join_response.push(TLK_CODE_RECRUIT_SPEAKER);
    join_response.extend(enc(" Accepted."));
    join_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("JOIN"),
            join_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'B',
        status: b'G',
        climb_stat: 10,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: 1,
    });
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    // `conversation.md §7.6`: `0x84` has "no player prompt and no
    // input read", so the response emits in one turn. No reserve roster
    // record matches the speaker, and u5-spec#299 Q20: "The no-match
    // diagnostic stops" - the rest of the stream does not run and the
    // conversation is over.
    handle_play_key_input(&mut state, 'J', "OIN", Path::new("")).unwrap();
    assert!(transcript_has(&state, "I shall come."));
    assert!(!transcript_has(&state, "Accepted."));
    assert_ne!(state.message, TLK_KEYWORD_PROMPT);
    assert!(state.active_conversation.is_none());
}

/// A Talk session whose `JOIN` answer GOTOs a labelled yes/no block shaped
/// like the shipped recruit blocks (`conversation.md §7.7.1`, Julia).
fn scoped_join_talk_state() -> PlayState {
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
    head.extend(enc("Shall I join thee now?"));
    let mut yes = enc("At last!\"");
    yes.push(TLK_CODE_RECRUIT_SPEAKER);
    let raw_fields = vec![
        enc("Gwenno"),
        enc("a bard"),
        enc("Greetings"),
        enc("I sing"),
        enc("Farewell"),
        enc("JOIN"),
        vec![0x91],
        head,
        enc("Just let me know."),
        enc("y"),
        yes,
        vec![TLK_CODE_LABEL_RECORD, TLK_LABEL_LAST, 0xC0],
    ];
    let decoded: Vec<String> = raw_fields
        .iter()
        .map(|field| field.iter().map(|b| char::from(b ^ 0x80)).collect())
        .collect();
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(0x10, decoded);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(0x10, raw_fields);

    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A'),
        conversation_test_roster_record(1, b"GWENNO\0\0\0", b'B'),
    ];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    state
}

/// u5-spec#299 Q20, `conversation.md §7.7` step 2: the scoped prompt is
/// `You respond-` (R573), and an empty line there prints `"What didst thou
/// say?` and re-asks - no `BYE` echo, no Bye entry, nothing ends (R575).
#[test]
fn a_scoped_prompt_reads_you_respond_and_an_empty_line_only_re_asks() {
    let mut state = scoped_join_talk_state();
    handle_play_key_input(&mut state, 'J', "OIN", Path::new("")).unwrap();
    assert!(transcript_has(&state, "Shall I join thee now?"));
    assert_eq!(state.message, "You respond-\n:");

    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(transcript_has(&state, "\"What didst thou say?"));
    assert!(!transcript_has(&state, "BYE"));
    assert!(!transcript_has(&state, "Farewell"));
    assert!(state.active_conversation.is_some());
    assert_eq!(state.message, "You respond-\n:");

    // §7.7.1: `Y` at the scoped prompt runs the recruit, which ends the
    // conversation with no `BYE` and no Bye entry.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(transcript_has(&state, "At last!"));
    assert!(!transcript_has(&state, "Farewell"));
    assert!(state.active_conversation.is_none());
    assert_eq!(state.party.len(), 2);
}

#[test]
fn active_conversation_ask_who_consumes_next_line_as_answer() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "WHO".to_string(),
            "Name the keeper.".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut who_response = enc("Name the keeper.");
    who_response.push(TLK_CODE_ASK_WHO);
    who_response.extend(enc(" Accepted."));
    who_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("WHO"),
            who_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'B',
        status: b'G',
        climb_stat: 10,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: 1,
    });
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    handle_play_key_input(&mut state, 'W', "HO", Path::new("")).unwrap();
    assert!(transcript_has(&state, "Name the keeper."));
    assert_eq!(state.message, TLK_ASK_WHO_PROMPT);
    handle_play_key_input(&mut state, 'i', "olo", Path::new("")).unwrap();
    // The reply is on the transcript; the slot carries the prompt that
    // follows it.
    assert!(transcript_has(&state, "Accepted."));
    assert_eq!(state.message, TLK_KEYWORD_PROMPT);
    assert!(state.active_conversation.is_some());
    assert_eq!(state.active_conversation_join_candidate, None);
}

fn conversation_test_roster_record(
    slot: u8,
    name: &[u8; SAVE_CHARACTER_NAME_LEN],
    class_byte: u8,
) -> PartyRosterRecord {
    PartyRosterRecord {
        member: PartyMember {
            slot,
            class_byte,
            status: b'G',
            climb_stat: 10 + slot,
            mana: slot,
            hp: 20 + u16::from(slot),
            max_hp: 30 + u16::from(slot),
            level: 1 + slot,
        },
        name: *name,
        // `formats/saved-gam.md §3.1` record offset `0x09`. This helper
        // builds synthetic conversation rosters, so it takes the male
        // value; the gender-selection tests build their own records.
        gender: SAVE_GENDER_MALE_BYTE,
        experience: u16::from(slot) * 100,
        stay_counter: slot,
        strength: 15 + slot,
        intelligence: 18 + slot,
        equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
        defense: 0,
        inn_marker: 0,
    }
}

#[test]
fn conversation_join_adds_inactive_roster_companion() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Gwenno".to_string(),
            "a bard".to_string(),
            "Greetings".to_string(),
            "I sing".to_string(),
            "Farewell".to_string(),
            "JOIN".to_string(),
            "I shall come.".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut join_response = enc("I shall come.");
    join_response.push(TLK_CODE_RECRUIT_SPEAKER);
    join_response.extend(enc(" Accepted."));
    join_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Gwenno"),
            enc("a bard"),
            enc("Greetings"),
            enc("I sing"),
            enc("Farewell"),
            enc("JOIN"),
            join_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A'),
        conversation_test_roster_record(1, b"GWENNO\0\0\0", b'B'),
    ];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );

    // §7.6: the recruit happens on the control byte, in the same
    // turn as the response text; nothing asks the player for a name.
    let (text, ended) = state.submit_active_conversation_keyword("JOIN");

    assert!(text.contains("I shall come."));
    // u5-spec#299 Q20, §7.6's `0x84` row: "A successful recruit reports a
    // stop: the conversation ends at once with no `BYE` and no Bye entry".
    assert!(!text.contains("Accepted."), "{text:?}");
    assert!(!text.contains("Farewell"), "{text:?}");
    // §7.6 publishes no success line for `0x84` (playtest F06).
    assert!(!text.contains("joined."), "{text:?}");
    assert!(ended);
    assert_eq!(state.party.len(), 2);
    assert_eq!(state.party_names[1], *b"GWENNO\0\0\0");
}

#[test]
fn conversation_join_is_triggered_by_the_control_byte_not_the_join_keyword() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Gwenno".to_string(),
            "a bard".to_string(),
            "Greetings".to_string(),
            "I sing".to_string(),
            "Farewell".to_string(),
            "HELP".to_string(),
            "I shall come.".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut help_response = enc("I shall come.");
    help_response.push(TLK_CODE_RECRUIT_SPEAKER);
    help_response.extend(enc(" Accepted."));
    help_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Gwenno"),
            enc("a bard"),
            enc("Greetings"),
            enc("I sing"),
            enc("Farewell"),
            enc("HELP"),
            help_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A'),
        conversation_test_roster_record(1, b"GWENNO\0\0\0", b'B'),
    ];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);

    // §7.6: "the visible `JOIN` topic is an ordinary per-NPC keyword
    // whose response stream emits this code" — any keyword whose
    // response emits `0x84` recruits, not just the word JOIN.
    let (text, ended) = state.submit_active_conversation_keyword("help");

    assert!(text.contains("I shall come."));
    // u5-spec#299 Q20: a successful recruit stops the stream and ends the
    // conversation.
    assert!(!text.contains("Accepted."), "{text:?}");
    // §7.6 publishes no success line for `0x84` (playtest F06).
    assert!(!text.contains("joined."), "{text:?}");
    assert!(ended);
    assert_eq!(state.party.len(), 2);
    assert_eq!(state.party_names[1], *b"GWENNO\0\0\0");
}

#[test]
fn conversation_recruit_speaker_for_non_roster_npc_recruits_nobody() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a sage".to_string(),
            "Greetings".to_string(),
            "I teach".to_string(),
            "Farewell".to_string(),
            "HELP".to_string(),
            "I shall come.".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut help_response = enc("I shall come.");
    help_response.push(TLK_CODE_RECRUIT_SPEAKER);
    help_response.extend(enc(" Accepted."));
    help_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a sage"),
            enc("Greetings"),
            enc("I teach"),
            enc("Farewell"),
            enc("HELP"),
            help_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![conversation_test_roster_record(0, b"AVATAR\0\0\0", b'A')];
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);

    // §7.6: "If no reserve record matches the speaker's name the
    // engine ... recruits nobody", and (u5-spec#299 Q20) "The no-match
    // diagnostic stops". The diagnostic's text is not published, so none
    // is printed.
    let (text, ended) = state.submit_active_conversation_keyword("help");

    assert_eq!(text, conversation_framed_response("I shall come."));
    assert!(ended);
    assert_eq!(state.party.len(), 1);
    assert_eq!(state.active_conversation_join_candidate, None);
}

/// **Withdrawn behaviour, still pinned.** `conversation.md §7.6` now
/// says a `0x84` at the six-member cap "prints the ... refusal and
/// recruits nobody" — it never ejects a companion. The conversation
/// session already refuses at the cap and stops calling this helper,
/// so the swap arm below is unreachable from play; the helper itself
/// still has to be rewritten in `play_state_impl/chunk_04.rs`.
#[test]
fn conversation_join_full_party_replaces_answered_companion() {
    let names: [[u8; SAVE_CHARACTER_NAME_LEN]; SAVE_PARTY_SIZE_MAX as usize] = [
        *b"AVATAR\0\0\0",
        *b"IOLO\0\0\0\0\0",
        *b"SHAMINO\0\0",
        *b"MARIAH\0\0\0",
        *b"JULIA\0\0\0\0",
        *b"GEOFFREY\0",
    ];
    let mut state = test_state(open_grid(), 1, 1);
    state.party = (0..SAVE_PARTY_SIZE_MAX)
        .map(|slot| conversation_test_roster_record(slot, &names[slot as usize], b'B').member)
        .collect();
    state.party_names = names.to_vec();
    state.party_experience = (0..SAVE_PARTY_SIZE_MAX)
        .map(|slot| u16::from(slot) * 100)
        .collect();
    state.party_stay_counters = (0..SAVE_PARTY_SIZE_MAX).collect();
    state.party_strengths = (0..SAVE_PARTY_SIZE_MAX).map(|slot| 15 + slot).collect();
    state.party_intelligence = (0..SAVE_PARTY_SIZE_MAX).map(|slot| 18 + slot).collect();
    state.party_equipment = default_party_equipment(SAVE_PARTY_SIZE_MAX as usize);
    state.party_roster = (0..SAVE_PARTY_SIZE_MAX)
        .map(|slot| conversation_test_roster_record(slot, &names[slot as usize], b'B'))
        .collect();
    state.party_roster.push(conversation_test_roster_record(
        SAVE_PARTY_SIZE_MAX,
        b"GWENNO\0\0\0",
        b'D',
    ));
    state.active_player = Some(1);

    let text = state
        .apply_conversation_join_candidate("Gwenno", 2)
        .unwrap();

    assert_eq!(text, "GWENNO joined; IOLO left.");
    assert_eq!(state.party.len(), SAVE_PARTY_SIZE_MAX as usize);
    assert_eq!(state.party_names[1], *b"GWENNO\0\0\0");
    assert_eq!(
        state.party_roster[SAVE_PARTY_SIZE_MAX as usize].name,
        *b"IOLO\0\0\0\0\0"
    );
    assert_eq!(state.active_player, None);
}

/// `conversation.md §7.6`: an unaffordable payment stops before its
/// in-place success tail; no text or side effect after the third digit
/// is executed, and the nested prompt ends the conversation on return.
#[test]
fn town_raw_tlk_unaffordable_payment_skips_success_tail_and_unwinds() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "PAY".to_string(),
            "placeholder".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut pay_response = vec![0x85, b'0', b'5', b'0'];
    pay_response.extend([0x8A, 0x8A, 0x8A]);
    pay_response.extend(enc("Paid"));
    pay_response.push(0xFF);

    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("PAY"),
            pay_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.gold = 30;
    state.moral_standing = 40;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);
    let (text, ended) = state.submit_active_conversation_keyword("pay");
    assert!(!ended);
    assert_eq!(text, conversation_framed_response(TLK_GOLD_PAYMENT_REFUSAL_MESSAGE));
    assert_eq!(state.gold, 30);
    assert_eq!(state.moral_standing, 40);
    assert!(matches!(
        state
            .active_conversation
            .as_ref()
            .map(|session| session.phase),
        Some(crate::conversation_session::ConversationSessionPhase::AwaitingGoldRefusalKeyword)
    ));

    let (_, ended) = state.submit_active_conversation_keyword("anything");
    assert!(!ended, "an unmatched nested keyword reprompts");
    assert!(state.active_conversation.is_some());
    let (_, ended) = state.submit_active_conversation_keyword("");
    assert!(ended, "empty nested input runs Bye once and unwinds");
    assert!(state.active_conversation.is_none());
}

#[test]
fn town_raw_tlk_gold_payment_debits_only_affordable_accepted_payment() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "PAY".to_string(),
            "placeholder".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut pay_response = vec![0x85, b'0', b'2', b'5'];
    pay_response.extend(enc("Paid"));
    pay_response.push(0xFF);

    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("PAY"),
            pay_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.gold = 30;
    state.moral_standing = 40;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);
    let (text, ended) = state.submit_active_conversation_keyword("pay");
    assert_eq!(text, conversation_framed_response("Paid"));
    assert!(!ended);
    assert_eq!(state.gold, 5);
    // This fixture does not pre-seed a toll-progress milestone boundary,
    // so accepted payment debits gold without changing moral standing.
    assert_eq!(state.moral_standing, 40);

    let mut poor_state = state.clone();
    poor_state.gold = 10;
    poor_state.moral_standing = 40;
    poor_state.open_conversation_session(&dialogue, &raw);
    let (text, ended) = poor_state.submit_active_conversation_keyword("pay");
    assert_eq!(text, conversation_framed_response(TLK_GOLD_PAYMENT_REFUSAL_MESSAGE));
    assert!(!ended);
    assert_eq!(poor_state.gold, 10);
    assert!(matches!(
        poor_state
            .active_conversation
            .as_ref()
            .map(|session| session.phase),
        Some(crate::conversation_session::ConversationSessionPhase::AwaitingGoldRefusalKeyword)
    ));
}

#[test]
fn town_raw_tlk_one_shot_keyword_records_numeric_signal_flag() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "MARK".to_string(),
            "Marked".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut mark_response = enc("Marked");
    mark_response.push(TLK_CODE_ACTION_DISPATCH);
    mark_response.push(5);
    mark_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("MARK"),
            mark_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, Some("mark")),
        MoveOutcome::Talked
    );

    assert_eq!(state.message, "Marked");
    assert_eq!(state.conversation_signal_flags[5], 1);
    assert!(state.active_conversation.is_none());
}

#[test]
fn tlk_numeric_action_dispatch_increments_signal_slots_to_cap() {
    let mut state = test_state(open_grid(), 1, 1);
    state.conversation_signal_flags[5] = TLK_GENERIC_SIGNAL_CAP - 1;
    state.conversation_signal_flags[6] = TLK_GENERIC_SIGNAL_CAP;

    state.record_tlk_signal_flags(&[5, 5, 6, 64]);

    assert_eq!(state.conversation_signal_flags[5], TLK_GENERIC_SIGNAL_CAP);
    assert_eq!(state.conversation_signal_flags[6], TLK_GENERIC_SIGNAL_CAP);
}

/// `conversation.md §6`'s response framing, as these tests expect to see
/// it: a leading line feed, the response in double quotes, and trailing
/// line feeds left outside the closing quote.
fn conversation_framed_response(text: &str) -> String {
    let mut out = if text.trim_start().starts_with('"') {
        format!("\n{text}")
    } else {
        let body = text.trim_end_matches('\n');
        format!("\n\"{body}\"{}", &text[body.len()..])
    };
    while !out.ends_with("\n\n") {
        out.push('\n');
    }
    out
}

#[test]
fn active_conversation_keeps_numeric_signal_separate_from_falsehood_theft() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
            "MARK".to_string(),
            "Marked".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut mark_response = enc("Marked");
    mark_response.push(TLK_CODE_ACTION_DISPATCH);
    mark_response.push(5);
    mark_response.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
            enc("MARK"),
            mark_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        // `time.md §7`: a new game seeds every Shadowlord slot to `0`
        // ("not yet placed"), so name the host town explicitly. Hideout
        // id 4 is the Yew town scene byte.
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.resident_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);

    let (text, ended) = state.submit_active_conversation_keyword("mark");
    assert_eq!(text, conversation_framed_response("Marked"));
    assert!(!ended);
    assert_eq!(state.conversation_signal_flags[5], 1);

    let (text, ended) = state.submit_active_conversation_keyword("bye");
    assert!(ended);
    assert!(state.active_conversation.is_none());
    assert_eq!(state.conversation_signal_flags[5], 1);
    assert_eq!(text, "\n\"Farewell\"\n\n Stolen goods.");
}

#[test]
fn final_conversation_cleanup_suppresses_on_nonzero_shared_sentinel() {
    let mut state = test_state(open_grid(), 1, 1);
    state.record_tlk_signal_flags(&[7]);
    let gold_before = state.gold;

    assert_eq!(
        state.shared_town_conversation_sentinel(),
        CONVERSATION_SHARED_NO_SLOT_SENTINEL
    );
    assert_eq!(state.run_final_conversation_cleanup(), None);
    assert_eq!(state.conversation_signal_flags[7], 1);
    assert_eq!(state.gold, gold_before);
}

#[test]
fn final_conversation_cleanup_uses_inventory_cascade_and_fresh_seed() {
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        // `time.md §7`: a new game seeds every Shadowlord slot to `0`
        // ("not yet placed"), so name the host town explicitly. Hideout
        // id 4 is the Yew town scene byte.
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.resident_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    state.keys = 0;
    state.gems = 2;
    state.torches = 0;
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.scroll_stock = [0; SCROLL_COUNT];
    state.potion_stock = [0; POTION_COUNT];
    state.gold = 10;

    assert_eq!(
        state.run_final_conversation_cleanup_with_seed(0x0123),
        Some("Stolen goods.".to_string())
    );
    assert_eq!(state.gems, 1);
    assert_eq!(state.gold, 10);

    state.keys = 0;
    state.gems = 0;
    state.torches = 0;
    state.equipment_stock[2] = 1;
    state.equipment_stock[47] = 2;
    state.scroll_stock[7] = 1;
    state.potion_stock[7] = 1;
    state.run_final_conversation_cleanup_with_seed(0x0456);
    assert_eq!(state.equipment_stock[47], 1);
    assert_eq!(state.equipment_stock[2], 1);
    assert_eq!(state.scroll_stock[7], 1);
    assert_eq!(state.prng_state, 0x0456);

    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.run_final_conversation_cleanup_with_seed(0x0789);
    assert_eq!(state.scroll_stock[7], 0);
    assert_eq!(state.potion_stock[7], 1);
    assert_eq!(state.prng_state, 0x0789);

    state.run_final_conversation_cleanup_with_seed(0x0321);
    assert_eq!(state.potion_stock[7], 0);
    assert_eq!(state.prng_state, 0x0321);

    let mut expected_prng = 0x0abc;
    let debit = u5_prng_range_u16(&mut expected_prng, 1, 15);
    state.run_final_conversation_cleanup_with_seed(0x0abc);
    assert_eq!(state.gold, 10u16.saturating_sub(debit));
    assert_eq!(state.prng_state, expected_prng);
}

#[test]
fn town_talk_horse_mounted_refuses_non_horse_trader_shops() {
    // shops.md §2: ordinary shop arms refuse before opening their menu when
    // the party is mounted on a horse; only the 0x83 horse trader remains.
    let dialogue = HashMap::new();
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
            dialog_id: 0x85, // herbalist
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Blocked
    );

    // shops.md §2 publishes the refusal verbatim, as a fixed two-line
    // merchant line rather than one worded per shop role.
    assert_eq!(
        state.message,
        "A merchant says:
\"GET THAT HORSE OUT OF HERE!\""
    );
    assert_eq!(state.turn, 1);
    assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
}

#[test]
fn town_talk_horse_mounted_still_reaches_horse_trader() {
    // shops.md §2: the 0x83 horse-trader vehicle-sale arm remains
    // available while mounted. A placement cell south keeps the stables open
    // (`shops.md §8.10`).
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(6).unwrap(),
        floor: 0,
    };
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
            dialog_id: 0x83, // horse trader
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("Horse & Rider"));
    assert!(state.active_shop.is_some());
    assert_eq!(state.turn, 1);
}

#[test]
fn end_to_end_horse_trader_purchase_places_boardable_horse() {
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(20).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 143;
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
            dialog_id: 0x83,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("143 gold"));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    // `shops.md §8.C`: the sale ends on the purchase-completed farewell, which
    // needs `SHOPPE.DAT`; there is no resident success line of its own.
    assert_eq!(state.gold, 0);
    assert!(state.active_shop.is_none());
    assert!(!state.message.contains("Thy horse awaits outside"));
    let horse = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == HORSE_PARKED_FIRST)
        .copied()
        .expect("horse object was not placed");
    assert_eq!((horse.x, horse.y, horse.z), (1, 2, 0));
    assert!(matches!(
        state
            .boardable_vehicle_slot_at(1, 2)
            .map(|candidate| candidate.transport),
        Some(TransportState::Horse { .. })
    ));
}

#[test]
fn horse_trader_purchase_uses_published_adjacent_probe_order_and_skips_occupied() {
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05; // south: first probe, but occupied below
    grid[1] = 0x44; // north: first free accepted marker
    grid[32 + 2] = 0x45; // east: talk target is here
    grid[32] = 0x05; // west: later fallback
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(20).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 143;
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
            dialog_id: 0x83,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.active_objects.push(ActiveObject {
        type_byte: 0x42,
        tile: 0x42,
        x: 1,
        y: 2,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    let horse = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == HORSE_PARKED_FIRST)
        .copied()
        .expect("horse object was not placed");
    assert_eq!((horse.x, horse.y, horse.z), (1, 0, 0));
}

#[test]
fn horse_trader_purchase_refuses_without_local_marker_and_preserves_gold() {
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x01;
    grid[1] = 0x01;
    grid[32 + 2] = 0x01;
    grid[32] = 0x01;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(20).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 143;
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
            dialog_id: 0x83,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    // `shops.md §8.C` "Horse traders": no suitable adjacent placement cell
    // prints `The stables are closed.\n` "before any greeting, then exit".
    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "\nThe stables are closed.\n");
    assert!(state.active_shop.is_none());
    assert_eq!(state.gold, 143);
    assert!(
        !state
            .active_objects
            .iter()
            .any(|object| object.type_byte == HORSE_PARKED_FIRST)
    );
}

/// `shops.md §8.B`/`§8.C` horse traders: other entry keys re-poll, short
/// funds print the feed-it line with `yells $.` and end the visit silently,
/// and an inner `N` exits the visit.
#[test]
fn horse_trader_entry_short_funds_and_decline_end_the_visit() {
    let open = |gold: u16| {
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x05;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.player.facing = Direction::East;
        state.gold = gold;
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
                dialog_id: 0x83,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        assert_eq!(
            state.talk_facing_with_dialogue(&HashMap::new()),
            MoveOutcome::Talked
        );
        state
    };

    let mut state = open(10);
    for ignored in ['B', 'X'] {
        handle_play_key_input(&mut state, ignored, "", Path::new("")).unwrap();
        assert!(matches!(
            state.active_shop,
            Some(crate::shop_session::ActiveShopSession::HorseTrader(
                crate::shop_runtime::HorseTraderState::Greeting { .. }
            ))
        ));
    }
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.ends_with("\n\nDeal?\" "), "{:?}", state.message);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
    assert_eq!(state.gold, 10);
    assert!(
        state
            .message
            .starts_with("\n\n\"Thou couldst not afford to feed it!\"\nyells "),
        "{:?}",
        state.message
    );

    let mut state = open(500);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
    assert_eq!(state.gold, 500);
}

#[test]
fn town_talk_guild_shop_uses_scene_local_prices() {
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(24).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 500;
    state.keys = 0;
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
            dialog_id: 0x86,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert!(state.message.contains("The Nemesis"));

    // `shops.md §8.2`: The Nemesis's key bundle base is 185, adjusted by the
    // speaker's Intelligence, and one offer sells three keys.
    let price = crate::shops::guild_bundle_price(
        GuildShop::TheNemesis,
        crate::shops::GuildCommodity::Keys,
        crate::input_dispatch::active_speaker_intelligence(&state),
    );
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    assert!(state.message.contains("Interested?"), "{:?}", state.message);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 500 - price);
    assert_eq!(state.keys, 3);
}

#[test]
fn town_talk_herbalist_uses_scene_local_reagent_menu() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(23).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 100;
    state.active_effect_tag = Some(CROWN_LB_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = PERMANENT_ACTIVE_EFFECT_DURATION;
    state.reagents = [0; REAGENT_COUNT];
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

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert!(state.message.contains("Mysticism"));

    // `shops.md §6`: Mysticism's first stocked reagent is Spider Silk, two
    // for a base of 6.
    let quote = crate::shops::quote_reagent_purchase(
        Herbalist::Mysticism,
        crate::shops::Reagent::SpiderSilk,
        crate::input_dispatch::active_speaker_intelligence(&state),
    )
    .unwrap();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("A...Spider Silk"), "{:?}", state.message);
    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 100 - quote.total_price);
    assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 2);
}

#[test]
fn town_talk_horse_trader_uses_scene_local_stable_price() {
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(20).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 200;
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
            dialog_id: 0x83,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert!(state.message.contains("The Stablehouse"));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("143 gold"));
}

#[test]
fn town_talk_horse_trader_quotes_active_speaker_intelligence() {
    let dialogue = HashMap::new();
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(20).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 300;
    state.party_intelligence = vec![30, 10];
    state.active_player = Some(1);
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
            dialog_id: 0x83,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("221 gold"));
}

#[test]
fn open_conversation_session_renders_greeting_and_stores_session() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
        ],
    );
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.set_active_talk_branch_flag(1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    let greeting = state.open_conversation_session(&dialogue, &raw);
    assert!(greeting.is_some());
    assert!(state.message.contains("Greetings"));
    assert!(state.active_conversation.is_some());
}

#[test]
fn conversation_opening_reseeds_only_for_strangers_and_uses_name_coin_flip() {
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let raw = vec![
        enc("Maris"),
        enc("a quiet sage"),
        enc("Greetings"),
        enc("I read books"),
        enc("Farewell"),
    ];
    let decoded = vec![
        "Maris".to_string(),
        "a quiet sage".to_string(),
        "Greetings".to_string(),
        "I read books".to_string(),
        "Farewell".to_string(),
    ];
    let mut state = test_state(open_grid(), 1, 1);
    state.active_conversation_npc_slot = Some(1);
    state.set_active_talk_branch_flag(1);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw.clone(), decoded.clone()),
    ));
    state.prng_state = 0x0aaa;

    let known = state.active_conversation_greeting_rendered_with_host_seed(0x0123);
    // The Description entry runs first now (`cleak/u5-spec#198`), so the
    // opening carries it ahead of the greeting.
    assert_eq!(known.text, "a quiet sage\n\n\"Greetings\"");
    assert_eq!(state.prng_state, 0x0aaa);

    state.talk_branch_flags.clear();
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw, decoded),
    ));
    let mut expected_stream = 0x0456;
    let introduces = u5_prng_range_u16(&mut expected_stream, 0, 1) != 0;
    let stranger = state.active_conversation_greeting_rendered_with_host_seed(0x0456);
    assert_eq!(
        stranger.text,
        if introduces {
            "a quiet sage\n\n\"I am called Maris\""
        } else {
            "a quiet sage"
        }
    );
    assert_eq!(state.prng_state, expected_stream);
}

#[test]
fn conversation_opening_drops_a_quote_that_follows_the_entry_closing_quote() {
    // `conversation.md §9`: "The renderer drops a quote that would
    // immediately follow another quote, so a Greeting entry that ends
    // with its own closing quote does not produce a doubled one".
    // Playtest E02 (`"I am called Eb""`).
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let raw = vec![
        enc("Eb\""),
        enc("a farmer"),
        enc("Hail\""),
        enc("I farm"),
        enc("Bye"),
    ];
    let decoded = vec![
        "Eb\"".to_string(),
        "a farmer".to_string(),
        "Hail\"".to_string(),
        "I farm".to_string(),
        "Bye".to_string(),
    ];
    let mut state = test_state(open_grid(), 1, 1);
    state.active_conversation_npc_slot = Some(1);
    state.set_active_talk_branch_flag(1);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw.clone(), decoded.clone()),
    ));
    let known = state.active_conversation_greeting_rendered_with_host_seed(0x0123);
    assert_eq!(known.text, "a farmer\n\n\"Hail\"");

    // Find a host seed whose coin flip takes the self-introduction.
    let seed = (0u16..64)
        .find(|seed| {
            let mut stream = *seed;
            u5_prng_range_u16(&mut stream, 0, 1) != 0
        })
        .expect("some seed introduces");
    state.talk_branch_flags.clear();
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw, decoded),
    ));
    let stranger = state.active_conversation_greeting_rendered_with_host_seed(seed);
    assert_eq!(stranger.text, "a farmer\n\n\"I am called Eb\"");
}

/// u5-spec#299 Q1 (`conversation.md §9` step 2, R560): a Description that
/// stops on `0x82` makes the opener return - no blank rows, no reseed or
/// coin flip, no greeting and no prompt - and the conversation is closed, so
/// the next key is a command.
#[test]
fn a_description_that_stops_ends_the_conversation_at_the_opener() {
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut description = enc("a grave wizard.");
    description.extend([TLK_CODE_LITERAL_NEWLINE, TLK_CODE_END_STREAM]);
    let raw = vec![
        enc("Annon"),
        description,
        enc("Well met."),
        enc("I served."),
        enc("Good luck!"),
    ];
    let decoded = raw
        .iter()
        .map(|field| field.iter().map(|b| char::from(b ^ 0x80)).collect())
        .collect();
    let mut state = test_state(open_grid(), 1, 1);
    state.active_conversation_npc_slot = Some(1);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw, decoded),
    ));
    state.prng_state = 0x0aaa;

    let opening = state.advance_active_conversation_greeting_with_seed(Some(0x0456));
    assert!(opening.ends_with("a grave wizard.\n"), "{opening:?}");
    assert!(!opening.contains(TLK_KEYWORD_PROMPT), "{opening:?}");
    assert!(!opening.contains("Well met") && !opening.contains("I am called"));
    assert!(state.active_conversation.is_none(), "the conversation is over");
    assert_eq!(state.prng_state, 0x0aaa, "no reseed on the stop path");
}

/// Playtest F04. `conversation.md §9` step 3: the stranger's
/// self-introduction "runs the Name entry ... through the byte runner".
/// Gruman's Name entry ends on the introduce-yourself guard - quote, two
/// feeds, `0x8C 0xFF`, `0x88` - so with the bit clear the ASK-WHO runs in
/// full and the opening ends on §7.6's name prompt, not `Your interest?`.
#[test]
fn a_self_introduction_whose_name_entry_asks_who_closes_on_the_name_prompt() {
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut name = enc("Grum\"");
    name.extend([
        TLK_CODE_LITERAL_NEWLINE,
        TLK_CODE_LITERAL_NEWLINE,
        TLK_CODE_IF_ELSE,
        0xFF,
        TLK_CODE_ASK_WHO,
    ]);
    let raw = vec![
        name,
        enc("a knight."),
        enc("Well met."),
        enc("I guard."),
        enc("Farewell."),
        enc("job"),
        enc("Walls."),
    ];
    let decoded = raw
        .iter()
        .map(|field| field.iter().map(|b| char::from(b ^ 0x80)).collect())
        .collect();
    let seed = (0u16..64)
        .find(|seed| {
            let mut stream = *seed;
            u5_prng_range_u16(&mut stream, 0, 1) != 0
        })
        .expect("some seed introduces");
    let mut state = test_state(open_grid(), 1, 1);
    state.active_conversation_npc_slot = Some(1);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw, decoded),
    ));

    let opening = state.advance_active_conversation_greeting_with_seed(Some(seed));
    assert!(opening.contains("\"I am called Grum\""), "{opening:?}");
    assert!(opening.ends_with(crate::TLK_ASK_WHO_PROMPT), "{opening:?}");
    assert!(!opening.contains(TLK_KEYWORD_PROMPT), "{opening:?}");

    // The first typed word answers the name prompt, not the keyword loop.
    let (answer, _) = state.submit_active_conversation_keyword("job");
    assert!(answer.contains(crate::TLK_ASK_WHO_DISMISSIVE_LINE), "{answer:?}");
}

/// Playtest G02. `conversation.md §9` step 3 runs the Name entry "through
/// the byte runner", §7.4 makes `0x8D` a forced literal newline and §7.6
/// gives ASK-WHO no feed of its own, so the Name entry's two row breaks stand
/// between its closing quote and the question: `"I am called` / `Grum"` / a
/// blank row / `"What is thy` / `name?"`. The opening trimmed them and the
/// question ran onto the name row (`Trian"What is`).
#[test]
fn a_self_introduction_keeps_the_name_entry_row_breaks_before_ask_who() {
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut name = enc("Grum\"");
    name.extend([
        TLK_CODE_LITERAL_NEWLINE,
        TLK_CODE_LITERAL_NEWLINE,
        TLK_CODE_IF_ELSE,
        0xFF,
        TLK_CODE_ASK_WHO,
    ]);
    let raw = vec![
        name,
        enc("a knight."),
        enc("Well met."),
        enc("I guard."),
        enc("Farewell."),
    ];
    let decoded = raw
        .iter()
        .map(|field| field.iter().map(|b| char::from(b ^ 0x80)).collect())
        .collect();
    let seed = (0u16..64)
        .find(|seed| {
            let mut stream = *seed;
            u5_prng_range_u16(&mut stream, 0, 1) != 0
        })
        .expect("some seed introduces");
    let mut state = test_state(open_grid(), 1, 1);
    state.active_conversation_npc_slot = Some(1);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw, decoded),
    ));

    let opening = state.advance_active_conversation_greeting_with_seed(Some(seed));
    assert!(
        opening.ends_with(&format!(
            "\"I am called Grum\"\n\n{}",
            crate::TLK_ASK_WHO_PROMPT
        )),
        "{opening:?}"
    );
}

#[test]
fn conversation_keyword_line_is_capped_at_fifteen_characters() {
    // `conversation.md §6` step 2: the reader "accepts up to fifteen
    // characters", and `input.md §8` echoes only accepted bytes.
    // Playtest E04: seventeen typed characters echoed over two rows.
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let fields = ["Ada", "a smith", "Hail", "I mend gear", "Farewell"];
    let raw = fields.iter().map(|f| enc(f)).collect::<Vec<_>>();
    let decoded = fields.iter().map(|f| f.to_string()).collect::<Vec<_>>();
    let mut state = test_state(open_grid(), 1, 1);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(raw, decoded),
    ));
    state.advance_active_conversation_greeting();

    state.submit_active_conversation_keyword("abcdefghijklmnopq");

    assert!(transcript_has(&state, "ABCDEFGHIJKLMNO"));
    assert!(
        !state
            .message_entries()
            .iter()
            .any(|entry| entry.text.contains('P') && entry.text.contains('Q')),
        "{:?}",
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn raw_conversation_session_expands_loaded_common_word_dictionary() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "fallback greeting".to_string(),
            "fallback job".to_string(),
            "Farewell".to_string(),
        ],
    );
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            vec![0x01],
            enc("I read books"),
            enc("Farewell"),
        ],
    );

    let mut dictionary = std::array::from_fn(|_| String::new());
    dictionary[0] = "Greetings".to_string();

    let mut state = test_state(open_grid(), 1, 1);
    state.set_active_talk_branch_flag(1);
    state.common_word_dictionary = Some(dictionary);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    assert!(state.message.contains("Greetings"));
    assert!(!state.message.contains("[w00]"));
}

fn tokenized_tlk_bytes_for_test() -> Vec<u8> {
    fn push_text(bytes: &mut Vec<u8>, text: &str) {
        for byte in text.bytes() {
            bytes.push(byte | 0x80);
        }
        bytes.push(0);
    }

    // Shipped header shape: count word, then one (npc id, blob offset)
    // row. No sentinel.
    let mut bytes = vec![0; 6];
    bytes[0..2].copy_from_slice(&1u16.to_le_bytes());
    bytes[2..4].copy_from_slice(&2u16.to_le_bytes());
    bytes[4..6].copy_from_slice(&6u16.to_le_bytes());
    push_text(&mut bytes, "Ada");
    push_text(&mut bytes, "a test speaker");
    bytes.push(0x01);
    bytes.push(0);
    push_text(&mut bytes, "I speak");
    push_text(&mut bytes, "Bye");
    bytes
}

fn complete_common_word_dictionary_text(first_word: &str) -> String {
    (0..COMMON_WORD_DICTIONARY_ENTRIES)
        .map(|index| {
            let word = if index == 0 {
                first_word.to_string()
            } else {
                format!("word{index}")
            };
            format!("{index}\t{word}\n")
        })
        .collect()
}

#[test]
fn game_dir_talk_uses_published_dictionary_for_tokenized_raw_tlk() {
    let dir = debug_game_dir();
    fs::write(dir.join("CASTLE.TLK"), tokenized_tlk_bytes_for_test()).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.set_active_talk_branch_flag(1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("the"));
    assert!(!state.message.contains("[w01]"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn game_dir_talk_sidecar_dictionary_overrides_published_token() {
    let dir = debug_game_dir();
    fs::write(dir.join("CASTLE.TLK"), tokenized_tlk_bytes_for_test()).unwrap();
    fs::write(
        dir.join(COMMON_WORD_DICTIONARY_FILE),
        complete_common_word_dictionary_text("custom"),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.set_active_talk_branch_flag(1);
    state.player.facing = Direction::East;
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
            dialog_id: 2,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Talked
    );

    assert!(state.message.contains("custom"));
    assert!(!state.message.contains("[w01]"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn tlk_and_shoppe_tokens_share_dictionary_entry_zero() {
    let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
    dict[0] = "the";
    let tlk = crate::tlk_runner::run_tlk_stream(
        &[0x01],
        &crate::tlk_runner::TlkRunInputs {
            dictionary: Some(&dict),
            ..Default::default()
        },
    );
    let shoppe = crate::shoppe_bark::render_shoppe_bark(
        &[0x80],
        &crate::shoppe_bark::ShoppeBarkContext {
            dictionary: Some(&dict),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(tlk.text, " the");
    assert_eq!(shoppe, " the");
}

#[test]
fn active_conversation_preserves_protected_run_font_in_message_transcript() {
    let mut state = test_state(vec![0; 32 * 32], 1, 1);
    let mut fields = vec![Vec::new(); 5];
    fields[2] = vec![TLK_CODE_PROTECT_RUN];
    fields[2].extend("INOP".bytes().map(|byte| byte ^ TLK_TEXT_XOR_MASK));
    fields[2].push(TLK_CODE_PROTECT_RUN);
    fields[2].push(TLK_CODE_END_OF_RESPONSE);
    state.active_conversation = Some(Box::new(
        crate::conversation_session::ConversationSession::new(fields, vec![String::new(); 5]),
    ));

    // The opening is the composed §9 envelope now: the `You see `
    // lead-in, the Description and Greeting entries, and §6's prompt.
    assert_eq!(
        state.advance_active_conversation_greeting(),
        // §9 step 3 speaks inside the quote wrapper; the description
        // ahead of it is narration and stays bare (`cleak/u5-engine#5`).
        format!("\n{TLK_OPENING_DESCRIPTION_PREFIX}\"INOP\"\n\n{TLK_KEYWORD_PROMPT}")
    );
    let entry = state
        .message_entries()
        .iter()
        .rev()
        .find(|entry| entry.text.contains("INOP"))
        .unwrap();
    // The row also carries the `You see ` lead-in, which is ordinary
    // text; the protected run is the `INOP` span, which must stay runic.
    let start = entry.text.find("INOP").expect("protected run is on the row");
    assert!(
        entry.glyphs[start..start + "INOP".len()]
            .iter()
            .all(|glyph| glyph.font == TlkGlyphFont::Runic)
    );
}

#[test]
fn submit_conversation_keyword_returns_job_response() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a quiet sage".to_string(),
            "Greetings".to_string(),
            "I read books".to_string(),
            "Farewell".to_string(),
        ],
    );
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a quiet sage"),
            enc("Greetings"),
            enc("I read books"),
            enc("Farewell"),
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.open_conversation_session(&dialogue, &raw);
    let (text, ended) = state.submit_active_conversation_keyword("job");
    assert!(text.contains("read books"));
    assert!(!ended);
}

#[test]
fn submit_conversation_keyword_bye_ends_session() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Maris".to_string(),
            "a sage".to_string(),
            "Greetings".to_string(),
            "books".to_string(),
            "Farewell".to_string(),
        ],
    );
    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    raw.insert(
        0x10,
        vec![
            enc("Maris"),
            enc("a sage"),
            enc("Greetings"),
            enc("books"),
            enc("Farewell"),
        ],
    );
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.open_conversation_session(&dialogue, &raw);
    let (text, ended) = state.submit_active_conversation_keyword("bye");
    // The header belongs to the empty-input shortcut; a typed `BYE` has
    // already echoed onto the `:` row.
    assert!(!text.starts_with(TLK_EMPTY_INPUT_BYE_MESSAGE));
    assert!(text.contains("Farewell"));
    assert!(ended);
    assert!(state.active_conversation.is_none());
}

#[test]
fn end_to_end_innkeeper_session_through_input_dispatcher() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(2).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 100;
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
            dialog_id: 0x88,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    // Talk opens the inn.
    assert_eq!(
        state.talk_facing_with_dialogue(&dialogue),
        MoveOutcome::Talked
    );
    assert!(state.active_shop.is_some());
    assert_eq!(state.active_effect_tag, None);
    assert_eq!(state.active_effect_counter, 0);
    // `shops.md §8`'s entry table: the greeting takes `Y`, `N` or Space, and
    // Yes opens the Pick up / Leave / Rest question. The engine used to read
    // this first key as a service letter.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("Rest for the"), "{}", state.message);
    // Then 'R' selects inn rest.
    handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
    // `shops.md §8.C` "Rest offer | Opening quote, room record, then
    // `\nWilt thou take\nit?" `".
    assert!(state.message.ends_with("\nWilt thou take\nit?\" "), "{:?}", state.message);
    // 'Y' again to confirm.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    // "Paid Rest | `"Have a pleasant\nnight, `, honorific, token-expanded
    // `!"\nsays $.\n\n`, then the rest sequence. On completion the visit
    // ends without the inn continuation question or shared farewell."
    assert!(
        state.message.contains("\"Have a pleasant\nnight, "),
        "{:?}",
        state.message
    );
    assert!(state.message.ends_with(&format!(
        "{}\n\n{}",
        crate::commands::PAID_INN_REST_SLEEP_LINE,
        crate::commands::PAID_INN_REST_MORNING_LINE
    )));
    assert!(state.gold < 100);
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_innkeeper_decline_ends_the_visit_without_charge() {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 100;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ConfirmRest {
        inn: Inn::TheWayfarerInn,
        base_room_rate: 2,
        total_price: 2,
    }));
    // Pass 'n' via the suffix so the bare-N New-Order intercept in
    // the outer dispatcher does not eat the key before the shop
    // session sees it.
    handle_play_key_input(&mut state, ' ', "n", Path::new("")).unwrap();
    assert_eq!(state.gold, 100);
    // `shops.md §8.C` "Declined Rest offer": the visit ends through the
    // ordinary farewell - one of the no-sale records `178..181` (measured
    // 2026-09-08 at Hotel Brittany as `Perhaps another time...`), which this
    // asset-free fixture cannot render.
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_innkeeper_leave_companion_moves_roster_to_registry() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 100;
    state.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: 10,
            mana: 8,
            hp: 30,
            max_hp: 30,
            level: 5,
        },
        PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: 7,
            mana: 3,
            hp: 12,
            max_hp: 28,
            level: 3,
        },
    ];
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
    state.party_stay_counters = vec![8, 9];
    state.party_strengths = vec![30, 17];
    state.party_intelligence = vec![30, 19];
    state.party_experience = vec![0, 700];
    state.party_equipment = default_party_equipment(2);
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::for_inn(
        Inn::HotelBrittany,
    )));

    // `shops.md §8`'s entry answer opens the service question; the service
    // letter is read there.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'L', "", Path::new("")).unwrap();
    // Measured 2026-09-08 (`qa/paired/nb-inn-companion.tsv`): the branch asks
    // `"Who will stay?"` and quotes no deposit; the selection happens in the
    // panel's framed `Select:` picker.
    assert!(state.message.contains("\"Who will\nstay?\" "), "{:?}", state.message);
    handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();
    // `inventory.md §4.3`: the digit only moves the highlight; Return commits.
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    // Measured 2026-09-08 (`qa/paired/nb-inn-commit.tsv`): the deposit is
    // quoted as a monthly rate due at check-out, ending on the shared
    // `Wilt thou take it?"` prompt.
    assert!(state.message.contains("per month, due at check-out"));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    // `shops.md §8.4`: "Leave performs no affordability check or gold
    // debit" (R420).
    assert_eq!(state.gold, 100);
    assert_eq!(state.party.len(), 1);
    assert_eq!(state.party_names, vec![*b"AVATAR\0\0\0"]);
    assert_eq!(state.inn_registry.len(), 1);
    assert_eq!(state.inn_registry[0].scene_marker, 0x11);
    assert_eq!(state.inn_registry[0].name, *b"IOLO\0\0\0\0\0");
    // Measured: the innkeeper thanks the party and asks his follow-up
    // question; the slot number and the deposit are not reported back.
    assert!(state.message.contains("\"I thank thee.\""));
    assert!(
        state
            .message
            .contains("\"Is there\nanything more\nI can do for\nthee?\" ")
    );
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Innkeeper(InnkeeperState::Continuation { .. }))
    ));
    // §8.C: "N or Space echoes `No` and exits."
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_innkeeper_pickup_restores_matching_guest() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 100;
    state.inn_registry.push(InnGuestRecord {
        registry_slot: 0,
        scene_marker: 0x11,
        name: *b"IOLO\0\0\0\0\0",
        member: PartyMember {
            slot: 4,
            class_byte: b'B',
            status: b'P',
            climb_stat: 7,
            mana: 3,
            hp: 12,
            max_hp: 28,
            level: 3,
        },
        strength: 17,
        intelligence: 19,
        experience: 700,
        equipment: [1, 2, 3, 4, 5, 6],
        stay_counter: 0,
    });
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::default()));
    // `shops.md §8`'s entry answer opens the service question.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    // Measured 2026-09-08 (`qa/paired/nb-inn-pickup.tsv`): the branch key
    // itself charges the bill and returns the guest, so the message to check
    // is the one that keypress produces.
    handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
    assert!(state.message.contains("22 gold"));
    assert!(state.message.contains("has died"));

    assert_eq!(state.gold, 78);
    assert!(state.inn_registry.is_empty());
    assert_eq!(state.party.len(), 2);
    assert_eq!(state.party[1].status, b'D');
    assert_eq!(state.party[1].hp, 0);
    assert_eq!(state.party_names[1], *b"IOLO\0\0\0\0\0");
}

#[test]
fn end_to_end_innkeeper_pickup_bill_uses_stay_units_not_leave_deposit() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 100;
    state.party_intelligence[0] = 25;
    state.inn_registry.push(InnGuestRecord {
        registry_slot: 0,
        scene_marker: 0x11,
        name: *b"IOLO\0\0\0\0\0",
        member: PartyMember {
            slot: 4,
            class_byte: b'B',
            status: b'G',
            climb_stat: 7,
            mana: 3,
            hp: 12,
            max_hp: 28,
            level: 3,
        },
        strength: 17,
        intelligence: 19,
        experience: 700,
        equipment: [1, 2, 3, 4, 5, 6],
        stay_counter: 3,
    });
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::default()));
    // `shops.md §8`'s entry answer opens the service question.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
    // Measured: the bill is charged on the branch key itself and the
    // innkeeper wishes the party well; the slot number and a repeat of the
    // figure are not printed.
    assert!(state.message.contains("\"That will be 75 gold, please.\""));

    assert_eq!(state.gold, 25);
    assert!(state.inn_registry.is_empty());
    assert_eq!(state.party.len(), 2);
}

#[test]
fn end_to_end_tavern_blue_boar_fixed_drink_debits_gold() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheBlueBoarTavern,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("Blue Boar"));
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    // Measured 2026-09-08 (`qa/paired/wb-blueboar.tsv`): the branch lists six
    // named drinks with their prices, and the served drink answers with one
    // gendered line rather than repeating the price.
    assert!(state.message.contains("a) Rose.......18"));
    assert!(state.message.contains("f) Chablis....98"));
    handle_play_key_input(&mut state, 'F', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 102);
    assert!(state.message.contains("Ah, a fine\nchoice"));
    assert!(state.active_shop.is_some());
}

/// `shops.md §8.A`: the tavern's closing bark draws the nothing-bought row
/// `61..64` until a purchase completes in the visit, then the
/// purchase-completed row `65..68`; the per-visit flag resets on entry.
#[test]
fn tavern_closing_bark_switches_to_purchase_completed_row_after_a_sale() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    assert_eq!(
        crate::input_dispatch::tavern_closing_bark_records(false),
        (61, 64)
    );
    assert_eq!(
        crate::input_dispatch::tavern_closing_bark_records(true),
        (65, 68)
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheBlueBoarTavern,
    )));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(!state.arms_transaction_completed);
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'F', "", Path::new("")).unwrap();
    assert_eq!(state.gold, 102);
    assert!(state.arms_transaction_completed);
}

#[test]
fn end_to_end_tavern_provisions_partially_fill_to_food_cap() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 50;
    state.food = SHOP_FOOD_STOCK_CAP - 1;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheWayfarerTavern,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
    assert!(state.message.contains("16 gold each"));
    handle_play_key_input(&mut state, '5', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 34);
    assert_eq!(state.food, SHOP_FOOD_STOCK_CAP);
    // `shops.md §8.C`: "Requested packs completed / food cap reached |
    // `\n\n`; no extra success sentence", then the continuation question.
    assert_eq!(state.message, "\n\n\"Anything else\nfor thee?\" ");
}

#[test]
fn end_to_end_tavern_provisions_accept_multi_digit_inline_quantity() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 1000;
    state.food = 0;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheWayfarerTavern,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '1', "2", Path::new("")).unwrap();

    assert_eq!(state.gold, 808);
    assert_eq!(state.food, 300);
    assert_eq!(state.message, "\n\n\"Anything else\nfor thee?\" ");
    assert!(state.active_shop.is_some());
}

#[test]
fn end_to_end_healer_mission_cure_bypasses_gold_path() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 0;
    state.party[0].status = b'P';
    state.party[0].hp = 7;
    state.party[0].max_hp = 20;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::TheHealersMission,
    ));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    // **Measured** at Cove: a one-member party is treated straight off the
    // service letter, with no member prompt to answer.
    handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 0);
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(state.party[0].hp, 7);
    // `shops.md §8.C` "Minoc free Cure/Heal | `\n\n"Receive now the
    // Light!"`, followed by treatment effects", then the continuation.
    assert_eq!(
        state.message,
        "\n\n\"Receive now the Light!\"\n\n\"Is there any other way in which I may\naid thee?\" "
    );
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Healer(HealerShopState::Continuation, _))
    ));
}

#[test]
fn end_to_end_healer_mission_cure_bypasses_shadowlord_surcharge() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        // `time.md §7`: a new game seeds every Shadowlord slot to `0`
        // ("not yet placed"), so name the host town explicitly. Hideout
        // id 4 is the Yew town scene byte.
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.gold = 50;
    state.party[0].status = b'P';
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::TheHealersMission,
    ));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    // **Measured** at Cove: a one-member party is treated straight off the
    // service letter, with no member prompt to answer.
    handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 50);
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(
        state.message,
        "\n\n\"Receive now the Light!\"\n\n\"Is there any other way in which I may\naid thee?\" "
    );
    assert!(!state.message.contains("Surcharge"));
}

#[test]
fn end_to_end_paid_healer_uses_local_fee_and_play_state_treatment() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 60;
    state.party[0].status = b'P';
    state.party[0].hp = 5;
    state.party[0].max_hp = 22;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::TheShieldOfTruth,
    ));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    // A one-member party needs no member prompt; the quote follows the
    // service letter directly.
    handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap();
    assert!(state.message.contains("60 gold"));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 0);
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[0].hp, 22);
    // `shops.md §8.3`: the paid branch applies the treatment "and asks the
    // continuation Y/N question" - no invented treatment report. §8.C's
    // continuation keeps both leading feeds (playtest F47).
    assert_eq!(
        state.message,
        "\n\n\"Is there any other way in which I may\naid thee?\" "
    );
    assert!(state.active_shop.is_some());

    // §8.C: "Yes repeats the service question; No ends the visit."
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(
        state.message.contains("What is the nature of thy need?"),
        "{:?}",
        state.message
    );
    handle_play_key_input(&mut state, 'Q', "", Path::new("")).unwrap();
    assert!(state.message.is_empty(), "{:?}", state.message);
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
}

/// `shops.md §8.B`: "The healer entry accepts Y/N only and ignores Space";
/// other keys re-poll. §8.C: an untreatable selection leads to the
/// continuation Y/N gate, whose `N` ends the visit.
#[test]
fn healer_entry_ignores_space_and_continuation_is_a_yes_no_gate() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 60;
    state.party[0].status = b'G';
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::TheShieldOfTruth,
    ));

    for key in [' ', 'H', '\r'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::Healer(HealerShopState::Greeting, _))
        ));
    }
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    // A healthy member needs no cure: the refusal, then the continuation.
    handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();
    assert!(state.message.contains("Thou hast no need of this art!"));
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Healer(HealerShopState::Continuation, _))
    ));
    handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Healer(HealerShopState::Continuation, _))
    ));
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
    assert_eq!(state.gold, 60);
}

#[test]
fn active_shop_surcharge_applies_only_for_zero_shadowlord_sentinel() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        // `time.md §7`: a new game seeds every Shadowlord slot to `0`
        // ("not yet placed"), so name the host town explicitly. Hideout
        // id 4 is the Yew town scene byte.
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.resident_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    state.gold = 100;
    state.prng_state = 0;
    let expected_prng_state = u5_prng_advance_state(state.prng_state);
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheHonestMeal,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 78);
    assert_eq!(state.prng_state, expected_prng_state);
    // `shops.md` §8.C's published bill, resident text with no record.
    assert!(state.message.contains("\"That will be 3 gold for the"));
    assert!(state.message.contains("of ye,"));
    // `shops.md` §6.2: the surcharge "is a gold-side effect only; it does not
    // change shop stock, item quantities, or the quoted record text." The
    // gold assertion above is the whole of the evidence it ran - 100 less the
    // 3-gold bill less the 19-gold draw - and the window says nothing.
    assert!(!state.message.contains("Surcharge"));
}

#[test]
fn active_shop_surcharge_suppresses_without_zero_shadowlord_sentinel() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 100;
    state.prng_state = 0x1234;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheHonestMeal,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 97);
    assert_eq!(state.prng_state, 0x1234);
    // `shops.md` §8.C's published bill, resident text with no record.
    assert!(state.message.contains("\"That will be 3 gold for the"));
    assert!(state.message.contains("of ye,"));
    assert!(!state.message.contains("Surcharge"));
}

/// `conversation.md §7.3` (R520): "Every response stream ... goes through the
/// same byte runner, so both codes behave identically on every path" - `0x83`
/// is a timed, key-ended pause and `0x8F` a one-key wait, and neither ever
/// stops a response. The legacy keyword-carrying raw path ran the runner
/// with no hold markers, so a pause vanished and the whole reply printed at
/// once (audit A071 follow-up).
#[test]
fn legacy_raw_keyword_reply_holds_at_pause_and_wait_key() {
    let enc = |text: &str| -> Vec<u8> {
        text.bytes()
            .map(|byte| byte ^ TLK_TEXT_XOR_MASK)
            .collect()
    };
    let mut job = enc("First");
    job.push(crate::tlk_control_codes::TLK_CODE_PAUSE);
    job.extend(enc("Second"));
    job.push(crate::tlk_control_codes::TLK_CODE_WAIT_KEY);
    job.extend(enc("Third"));
    job.push(crate::tlk_control_codes::TLK_CODE_END_OF_RESPONSE);
    let fields: Vec<String> = ["Bob", "a man", "Hi", "FirstSecondThird", "Bye"]
        .iter()
        .map(|field| field.to_string())
        .collect();
    let raw_fields: Vec<Vec<u8>> = vec![
        enc("Bob"),
        enc("a man"),
        enc("Hi"),
        job,
        enc("Bye"),
    ];
    let dialogue = HashMap::from([(5u16, fields)]);
    let raw = HashMap::from([(5u16, raw_fields)]);

    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            type_byte: 0x54,
            dialog_id: 5,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_direction_with_dialogue_and_keyword_raw(
            Direction::East,
            &dialogue,
            &raw,
            Some("JOB"),
        ),
        MoveOutcome::Talked
    );
    let shown = transcript_texts(&state).join("\n");
    assert!(
        shown.contains("First") || state.message.contains("First"),
        "{shown:?} / {:?}",
        state.message
    );
    assert!(
        !shown.contains("Second") && !state.message.contains("Second"),
        "the pause holds the rest of the reply: {shown:?} / {:?}",
        state.message
    );
    assert!(
        !crate::text_hold::text_has_hold_markers(&state.message)
            && !crate::text_hold::text_has_hold_markers(&shown),
        "markers never print"
    );
    assert!(state.pending_text_hold.is_some());
}

#[test]
fn talk_display_marker_tile_does_not_preempt_real_shop() {
    use crate::shop_session::ActiveShopSession;

    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(21).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
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
            dialog_id: 0x84,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.grid[2 * 32 + 1] = 0x05;
    state.active_objects.push(ActiveObject {
        type_byte: 0x05,
        tile: 0x05,
        x: 1,
        y: 2,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.talk_direction_with_dialogue_and_keyword_raw(Direction::East, &dialogue, &raw, None),
        MoveOutcome::Talked
    );

    // The session variant is what identifies the shop. The message used
    // to be asserted on the word "Ship", which only appeared because the
    // opening line carried an engine-internal `Dispatch family:` suffix.
    assert!(state.message.contains("now open"));
    assert_eq!(state.turn, 1);
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ShipBroker(_))
    ));
}

#[test]
fn end_to_end_sage_rumour_quotes_confirms_debits_and_renders() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

    handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
    assert_eq!(state.gold, 200);
    assert_eq!(state.message, "Fair 'nuff?\" ");
    assert!(state.active_shop.is_some());

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert!(state.gold <= 150);
    assert!(state.message.contains("Malik"));
    assert!(state.message.contains("Moonglow"));
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_tavern_menu_lore_letter_reaches_paid_sage_lookup() {
    use crate::shop_runtime::{SageState, TavernState};
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheWayfarerTavern,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("lore"));
    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    assert!(state.message.contains("\"That will be 1 gold for the"));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();

    // Measured 2026-09-08 at The Cat's Lair (`qa/paired/paws-sage.tsv`): the
    // question is quoted and gendered, and a typed-input prompt follows it.
    // `shops.md` §8.C: the lore letter echoes onto the follow-up record's open
    // row and the question follows two line feeds - the first closes the
    // echo's row, the second is the blank row (measured 2026-09-10,
    // `paws-sage/lore`).
    assert_eq!(
        state.message,
        "\n\n\"Of what wouldst thou hear my lore, sir?\"\n\nYou respond:"
    );
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Sage(SageState::Prompt { .. }))
    ));

    handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
    assert_eq!(state.message, "Fair 'nuff?\" ");
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert!(state.gold <= 150);
    assert!(state.message.contains("Malik"));
    assert!(state.message.contains("Moonglow"));
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_sage_rumour_prefers_shoppe_record_rendering() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[
        (
            SAGE_RUMOUR_FEE_QUOTE_RECORD,
            b"Asset fee: % gold?".as_slice(),
        ),
        (85, b"Asset says: seek & below *.".as_slice()),
        (86, b"Asset says: seek & below *.".as_slice()),
        (87, b"Asset says: seek & below *.".as_slice()),
        (88, b"Asset says: seek & below *.".as_slice()),
        (
            SAGE_RUMOUR_SHORT_FUNDS_RECORD,
            b"Asset says no credit.".as_slice(),
        ),
    ]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

    handle_play_key_input(&mut state, 'H', "ONE", &dir).unwrap();
    assert_eq!(state.message, "Asset fee: 50 gold?\n\nFair 'nuff?\" ");
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();

    assert!(
        state
            .message
            .contains("Asset says: seek Malik below Moonglow.")
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn end_to_end_sage_short_funds_prefers_shoppe_record_rendering() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[
        (
            SAGE_RUMOUR_FEE_QUOTE_RECORD,
            b"Asset fee: % gold?".as_slice(),
        ),
        (
            SAGE_RUMOUR_SHORT_FUNDS_RECORD,
            b"Asset says no credit.".as_slice(),
        ),
    ]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 49;
    state.prng_state = 0x2468;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

    handle_play_key_input(&mut state, 'H', "ONE", &dir).unwrap();
    assert_eq!(state.message, "Asset fee: 50 gold?\n\nFair 'nuff?\" ");
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();

    assert_eq!(state.gold, 49);
    assert_eq!(state.prng_state, 0x2468);
    // `shops.md §8.C`: "Short funds | `"Sorry, `, honorific, record `91`".
    assert_eq!(state.message, "\"Sorry, sirAsset says no credit.");
    assert!(state.active_shop.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cowering_talk_is_canned_without_a_tlk_record() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            type_byte: 0x50,
            dialog_id: TOWN_NPC_COWERING_DIALOG_ID,
            schedule: [3, 3, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0, 6, 12, 18, 22],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_dialogue(&HashMap::new()),
        MoveOutcome::Talked
    );
    assert_eq!(state.message, TOWN_NPC_COWERING_RESPONSE);
    assert_eq!(state.turn, 1);
    assert!(state.active_conversation.is_none());
}

#[test]
fn brush_off_talk_skips_tlk_loading_and_attempts_forced_flight() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            type_byte: 0x50,
            dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
            schedule: [7, 7, 7, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ]);

    assert_eq!(
        state.talk_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Talked
    );
    assert_eq!(state.message, TOWN_NPC_BRUSHOFF_RESPONSE);
    assert_eq!(state.npcs[0].dialog_id, TOWN_NPC_COWERING_DIALOG_ID);
    assert_eq!(&state.npcs[0].schedule[..3], &[3, 3, 3]);
    assert_eq!(state.turn, 1);
    let _ = fs::remove_dir_all(dir);
}

/// `shops.md §8.5` (R432): the provision quote is "drawn uniformly from a
/// seven-record pool - `SHOPPE.DAT` ordinals `77` through `83`", so every one
/// of the seven is reachable. The pool's upper bound was `82` (playtest E57).
#[test]
fn tavern_provision_quote_draw_reaches_all_seven_records() {
    use crate::shops::{TAVERN_PROVISION_QUOTE_RECORD_FIRST, TAVERN_PROVISION_QUOTE_RECORD_LAST};

    let mut state = test_state(open_grid(), 1, 1);
    state.prng_state = 0x1357;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..500 {
        seen.insert(usize::from(state.random_range_u8(
            TAVERN_PROVISION_QUOTE_RECORD_FIRST as u8,
            TAVERN_PROVISION_QUOTE_RECORD_LAST as u8,
        )));
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), (77..=83).collect::<Vec<_>>());
}

#[test]
fn end_to_end_tavern_renders_state_menu_quote_and_follow_up_records() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[
        (69, b"Asset tavern menu.".as_slice()),
        (73, b"Asset tavern follow-up.".as_slice()),
        (77, b"Asset pack costs % gold.".as_slice()),
        (78, b"Asset pack costs % gold.".as_slice()),
        (79, b"Asset pack costs % gold.".as_slice()),
        (80, b"Asset pack costs % gold.".as_slice()),
        (81, b"Asset pack costs % gold.".as_slice()),
        (82, b"Asset pack costs % gold.".as_slice()),
        (83, b"Asset pack costs % gold.".as_slice()),
    ]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 100;
    state.food = 0;
    state.prng_state = 0x2468;
    let expected_prng_state = u5_prng_advance_state(state.prng_state);
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheWayfarerTavern,
    )));

    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    // `shops.md §8`'s Tavern entry row: the state menu record is wrapped in
    // an opening quote and "a separately emitted closing quote and space",
    // the same shape as the follow-up record asserted below.
    assert_eq!(state.message, "\"Asset tavern menu.\" ");
    handle_play_key_input(&mut state, 'R', "", &dir).unwrap();
    // `shops.md §8.C`, "Provision offer": "Opening quote, one uniform record
    // from `77..83`, then `\n\nHow many wouldst\nthou like?" `". The two
    // leading feeds close the `R` echo's row and leave the blank under it.
    assert_eq!(
        state.message,
        "\n\n\"Asset pack costs 16 gold.\n\nHow many wouldst\nthou like?\" "
    );
    assert_eq!(state.prng_state, expected_prng_state);
    handle_play_key_input(&mut state, '1', "", &dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert_eq!(state.food, 25);
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    // `shops.md` §8.C: "Y prints `Yes\n\n\"`, the applicable follow-up
    // record, a closing quote and a space." The trailing space is the cell
    // the next accepted menu letter echoes into. The `Yes` and its blank row
    // are echoed onto the previous question's open row rather than carried
    // in the slot, so they appear in the transcript rather than here.
    assert_eq!(state.message, "\"Asset tavern follow-up.\" ");
    let tail: Vec<_> = state
        .message_entries()
        .iter()
        .rev()
        .take(3)
        .map(|entry| entry.text.clone())
        .collect();
    // The purchase now asks `"Anything else\nfor thee?" ` (playtest E47), so
    // the `Yes` continues that question's open row.
    assert!(
        tail.contains(&"for thee?\" Yes".to_string()),
        "continuation echo missing from the transcript: {tail:?}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn end_to_end_partial_provision_sale_skips_falsehood_surcharge() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        // `time.md §7`: a new game seeds every Shadowlord slot to `0`
        // ("not yet placed"), so name the host town explicitly. Hideout
        // id 4 is the Yew town scene byte.
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.gold = 20;
    state.food = 0;
    state.prng_state = 0x2468;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheWayfarerTavern,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
    let state_after_quote_draw = state.prng_state;
    handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 4);
    assert_eq!(state.food, 25);
    assert_eq!(state.prng_state, state_after_quote_draw);
    assert!(!state.message.contains("Surcharge"));
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Tavern(TavernState::AnythingElse {
            continuation_ready: true,
            ..
        }))
    ));
}

#[test]
fn end_to_end_unaffordable_provisions_grant_only_the_published_charity_case() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    for (starting_food, expected_food, expects_charity) in [(2, 3, true), (3, 3, false)] {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 0;
        state.food = starting_food;
        state.active_shop = Some(ActiveShopSession::Tavern(
            TavernState::PickProvisionQuantity {
                tavern: Tavern::TheHonestMeal,
                unit_price: 10,
                continuation_ready: false,
                entry: Default::default(),
            },
        ));

        handle_play_key_input(&mut state, '1', "\r", Path::new("")).unwrap();

        assert_eq!(state.gold, 0);
        assert_eq!(state.food, expected_food);
        assert_eq!(state.message.contains("table scraps"), expects_charity);
        assert!(state.active_shop.is_none());
    }
}

/// `shops.md §8.C`'s tavern provisions outcome table, the two arms
/// `u5-engine#43` found unwired: a partial purchase and the no-gold/enough-
/// food refusal. Both need a real `SHOPPE.DAT` for the vendor-name
/// attribution row, so this is gated like
/// `local_shoppe_dat_asset_renders_sanitized_audit_when_present`.
#[test]
fn end_to_end_tavern_provisions_print_the_published_partial_and_refusal_arms() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let game_dir = game_dir.as_path();
    if !game_dir.join("SHOPPE.DAT").exists() {
        return;
    }

    // Scene 22 (Paws) under tavern dialog id `0x82` is Dr. Cat's The Cat's
    // Lair, the same pairing `shop_vendor_name_for_scene`'s own test table
    // uses - unit price 20 per `tavern_provision_unit_price`.
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 45;
    state.food = 10;
    state.active_shop = Some(ActiveShopSession::Tavern(
        TavernState::PickProvisionQuantity {
            tavern: Tavern::TheCatsLair,
            unit_price: 20,
            continuation_ready: false,
            entry: Default::default(),
        },
    ));

    // Five packs requested at 20 gold each; 45 gold only affords two.
    handle_play_key_input(&mut state, '5', "\r", game_dir).unwrap();

    assert_eq!(state.gold, 5);
    assert_eq!(state.food, 60);
    assert!(
        state.message.contains("\"Thou canst\nafford only 2!\"\n\n"),
        "partial-afford text missing: {:?}",
        state.message
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 0;
    state.food = 10;
    state.active_shop = Some(ActiveShopSession::Tavern(
        TavernState::PickProvisionQuantity {
            tavern: Tavern::TheCatsLair,
            unit_price: 20,
            continuation_ready: false,
            entry: Default::default(),
        },
    ));

    handle_play_key_input(&mut state, '1', "\r", game_dir).unwrap();

    assert_eq!(state.gold, 0);
    assert_eq!(state.food, 10);
    assert!(
        state
            .message
            .contains("\"Thou hast\nneither gold nor\nneed! Out!\"\nyells Dr. Cat.\n"),
        "no-need refusal attribution missing: {:?}",
        state.message
    );
    assert!(state.active_shop.is_none());
}

/// `shops.md §4.1`/`§6`: the reagent offer record's `^` is the fixed bundle
/// quantity the quoted price buys (Mysticism sells Spider Silk in twos), not
/// zero.
#[test]
fn reagent_offer_renders_the_bundle_quantity_placeholder() {
    use crate::shop_runtime::ReagentShopState;
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    // Record `139 + 3`: Spider Silk in the canonical eight-reagent order.
    fs::write(
        dir.join("SHOPPE.DAT"),
        shoppe_dat_with_records(&[(142, b"Silk, % gp for ^ spools.")]),
    )
    .unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
        Herbalist::Mysticism,
    )));
    let quote = crate::shops::quote_reagent_purchase(
        Herbalist::Mysticism,
        crate::shops::Reagent::SpiderSilk,
        crate::input_dispatch::active_speaker_intelligence(&state),
    )
    .unwrap();

    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'A', "", &dir).unwrap();

    assert!(
        state.message.contains(&format!(
            "Silk, {} gp for 2 spools. Is this thy need?",
            quote.total_price
        )),
        "{:?}",
        state.message
    );
    let _ = fs::remove_dir_all(dir);
}

fn shoppe_dat_with_records(records: &[(usize, &[u8])]) -> Vec<u8> {
    assert!(!records.is_empty());
    let payload_len: usize = records.iter().map(|(_, record)| record.len()).sum();
    for (record_id, _) in records {
        assert!(*record_id < SHOPPE_DAT_NONEMPTY_RECORDS);
    }
    let filler_record_count = SHOPPE_DAT_NONEMPTY_RECORDS - 1 - records.len();
    let record_zero_len =
        SHOPPE_DAT_LEN - SHOPPE_DAT_RECORD_SLOTS - filler_record_count - payload_len;
    let mut bytes = Vec::with_capacity(SHOPPE_DAT_LEN);
    bytes.extend(std::iter::repeat_n(b'a', record_zero_len));
    bytes.push(0);
    for id in 1..SHOPPE_DAT_NONEMPTY_RECORDS {
        if let Some((_, record)) = records.iter().find(|(record_id, _)| *record_id == id) {
            bytes.extend_from_slice(record);
        } else {
            bytes.push(b'x');
        }
        bytes.push(0);
    }
    for _ in SHOPPE_DAT_NONEMPTY_RECORDS..SHOPPE_DAT_RECORD_SLOTS {
        bytes.push(0);
    }
    assert_eq!(bytes.len(), SHOPPE_DAT_LEN);
    bytes
}

/// `shops.md §8.9`/`§8.C`: a displayed letter offers the herbalist's fixed
/// bundle, `Y` buys it once at the adjusted price, and the stock list follows
/// the thank-you.
#[test]
fn end_to_end_reagent_vendor_sells_one_bundle_per_offer() {
    use crate::shop_runtime::ReagentShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    let quote = crate::shops::quote_reagent_purchase(
        Herbalist::Mysticism,
        crate::shops::Reagent::SpiderSilk,
        crate::input_dispatch::active_speaker_intelligence(&state),
    )
    .unwrap();
    state.gold = quote.total_price + 5;
    state.reagents = [0; REAGENT_COUNT];
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
        Herbalist::Mysticism,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("Spider Silk"), "{:?}", state.message);
    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    assert!(state.message.contains("Is this thy need?"), "{:?}", state.message);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 5);
    assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 2);
    assert!(state.message.contains("\"I thank thee!\""), "{:?}", state.message);
    assert!(state.message.contains("Thy interest?"), "{:?}", state.message);
    assert!(state.active_shop.is_some());

    // Return leaves the list for the farewell path.
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
}

/// `shops.md §8.2`/`§8.C`: the guild sells one fixed bundle per Y/N offer
/// at the Intelligence-adjusted price, then returns to the stock menu.
#[test]
fn end_to_end_guildmaster_sells_one_bundle_per_offer() {
    use crate::shop_runtime::GuildShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 500;
    state.keys = 0;
    let speaker_intelligence = crate::input_dispatch::active_speaker_intelligence(&state);
    let price = crate::shops::guild_bundle_price(
        GuildShop::TheDen,
        crate::shops::GuildCommodity::Keys,
        speaker_intelligence,
    );
    state.gold = price + 7;
    state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::for_shop(
        GuildShop::TheDen,
    )));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("Thy concern?"), "{:?}", state.message);
    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    assert!(state.message.contains("Interested?"), "{:?}", state.message);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 7);
    assert_eq!(state.keys, 3);
    assert!(state.message.contains("\"Sold!\""), "{:?}", state.message);
    assert!(state.message.contains("Thy concern?"), "{:?}", state.message);
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Guild(GuildShopState::PickItem { .. }))
    ));

    // A second offer the party cannot afford ends the visit.
    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert_eq!(state.gold, 7);
    assert_eq!(state.keys, 3);
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_sage_short_funds_does_not_draw_success_record() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 49;
    state.prng_state = 0x2468;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

    handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
    // The fee lives in `SHOPPE.DAT` record `84`, which this fixture has no
    // assets to render; §8.C's resident confirmation is what remains.
    assert!(state.message.contains("Fair 'nuff?"), "{}", state.message);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 49);
    assert_eq!(state.prng_state, 0x2468);
    assert_eq!(state.message, "Beat it!");
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_sage_success_draws_record_after_debit_gate() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 50;
    state.prng_state = 0x2468;
    let expected_prng = u5_prng_advance_state(state.prng_state);
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

    handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
    assert_eq!(state.prng_state, 0x2468);
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 0);
    assert_eq!(state.prng_state, expected_prng);
    assert!(state.message.contains("Malik"));
    assert!(state.message.contains("Moonglow"));
    assert!(state.active_shop.is_none());
}

#[test]
fn end_to_end_arms_shop_exit_clears_session() {
    let dialogue = HashMap::new();
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
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
            dialog_id: 0x81,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue(&dialogue);
    assert!(state.active_shop.is_some());
    // `shops.md §8.B` stage 2: the welcome line waits for one key before the
    // attribution and greeting print, so Space here releases the pause rather
    // than answering the Buy/Sell prompt.
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert!(state.message.contains("Buy or Sell"), "{}", state.message);
    // Space at the greeting exits the shop.
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
    // The closing flourish is a `SHOPPE.DAT` record, so a test with no game
    // directory has nothing to render and the window stays empty.
    assert_eq!(state.message, "");
}

#[test]
fn end_to_end_stocked_arms_shop_buys_by_menu_letter() {
    use crate::shop_runtime::ArmsShopState;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::ArmsStockTable;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 1000;
    state.party_intelligence[0] = 10;
    state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
    ));

    handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();
    assert!(state.message.contains("a...Short Sword"));
    assert!(state.message.contains("b...Mace"));

    handle_play_key_input(&mut state, 'b', "", Path::new("")).unwrap();
    assert!(state.message.contains("Mace costs"));
    assert!((0..4).any(|roll| {
        state
            .message
            .contains(crate::shops::arms_buy_confirmation_prompt_for_roll(roll))
    }));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert_eq!(state.equipment_stock[24], 1);
    assert_eq!(state.equipment_stock[1], 0);
    assert!(state.gold < 1000);
    assert!(state.message.contains("Sold!"));
    assert!(state.active_shop.is_some());
}

#[test]
fn end_to_end_stocked_arms_shop_rejects_empty_stock_letters() {
    use crate::shop_runtime::ArmsShopState;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::ArmsStockTable;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 1000;
    state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
    ));

    handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'd', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 1000);
    assert!(state.equipment_stock.iter().all(|count| *count == 0));
    assert!(state.message.contains("a...Short Sword"));
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::BuyPickItem,
            _
        ))
    ));
}

#[test]
fn end_to_end_stocked_arms_shop_escape_exits_buy_and_sell_submenus() {
    use crate::shop_runtime::ArmsShopState;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::ArmsStockTable;

    let mut buy_state = test_state(open_grid(), 1, 1);
    buy_state.gold = 1000;
    buy_state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
    ));

    handle_play_key_input(&mut buy_state, 'B', "", Path::new("")).unwrap();
    handle_play_key_input(&mut buy_state, '\x1b', "", Path::new("")).unwrap();
    assert!(buy_state.active_shop.is_none());
    assert_eq!(buy_state.gold, 1000);
    assert!(buy_state.equipment_stock.iter().all(|count| *count == 0));
    // See above: the flourish needs the data file this test does not load.
    assert_eq!(buy_state.message, "");

    let mut sell_state = test_state(open_grid(), 1, 1);
    sell_state.gold = 1000;
    sell_state.equipment_stock[23] = 1;
    sell_state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
    ));

    handle_play_key_input(&mut sell_state, 'S', "", Path::new("")).unwrap();
    handle_play_key_input(&mut sell_state, '\x1b', "", Path::new("")).unwrap();
    assert!(sell_state.active_shop.is_none());
    assert_eq!(sell_state.gold, 1000);
    assert_eq!(sell_state.equipment_stock[23], 1);
    // Measured 2026-09-07: the goodbye is quoted and attributed to the
    // shopkeeper (`qa/paired/shop-arms-sell-flow.tsv`). Measured 2026-09-10
    // (`shop-arms-sell-zero-price/after`): it follows two line feeds, so the
    // rendered message opens with the one that leaves the blank row.
    assert!(
        sell_state.message.starts_with('\n'),
        "the goodbye follows a blank row: {:?}",
        sell_state.message
    );
    let sell_goodbye = sell_state.message.trim_start_matches('\n');
    assert!(
        [
            "\"Good-bye...\"",
            "\"Mayhap another time...\"",
            "\"Godspeed...\"",
            "\"Fare thee well...\"",
        ]
        .iter()
        .any(|line| sell_goodbye.starts_with(line))
    );
}

#[test]
fn end_to_end_arms_sell_browser_moves_selects_with_space_and_draws_only_at_contract_points() {
    use crate::shop_runtime::ArmsShopState;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::ArmsStockTable;

    let mut state = test_state(open_grid(), 1, 1);
    state.equipment_stock[2] = 1;
    state.equipment_stock[5] = 2;
    state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([2, 5, 0, 0, 0, 0, 0, 0], 2),
    ));

    handle_play_key_input(&mut state, 'S', "", Path::new("")).unwrap();
    let after_entry_draw = state.prng_state;
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::SellPickItem(_),
            _
        ))
    ));

    handle_play_key_input(
        &mut state,
        char::from(crate::INPUT_CODE_SOUTH),
        "",
        Path::new(""),
    )
    .unwrap();
    assert_eq!(
        state.prng_state, after_entry_draw,
        "movement consumes no draw"
    );

    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert_ne!(
        state.prng_state, after_entry_draw,
        "ordinary offer draws once"
    );
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::SellConfirm { item: 5, .. },
            _
        ))
    ));

    let visible_quote = state.message.clone();
    let after_offer_draw = state.prng_state;
    handle_play_key_input(&mut state, 'X', "", Path::new("")).unwrap();
    assert_eq!(state.message, visible_quote);
    assert_eq!(state.prng_state, after_offer_draw);

    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    assert_ne!(
        state.prng_state, after_offer_draw,
        "decline continuation draws once"
    );
    assert_eq!(state.equipment_stock[5], 2);
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::SellPickItem(_),
            _
        ))
    ));
}

#[test]
fn end_to_end_stocked_arms_shop_confirmation_ignores_non_yes_no_keys() {
    use crate::shop_runtime::ArmsShopState;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::ArmsStockTable;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 1000;
    state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
    ));

    handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'b', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'x', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 1000);
    assert_eq!(state.equipment_stock[24], 0);
    assert!(state.message.contains("Mace costs"));
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::BuyConfirm { item: 24, .. },
            _
        ))
    ));
}

/// `shops.md` §8.7: `Y` at the shipwright greeting prints the Frigate/Skiff
/// menu from record `119`. Measured 2026-09-10 (`bd-shipwright/yes`, both
/// sides drawing the same greeting record): the original prints the menu and
/// this engine printed nothing at all.
///
/// The cause is the dispatcher gating the whole ship-broker branch on
/// `return_world` being present, and `return_world` is only populated when
/// entering a *dungeon*. A shipwright is in a town, so the live game never
/// satisfies that gate. The existing end-to-end test below sets the field by
/// hand, which is why it passed throughout.
#[test]
fn end_to_end_shipwright_greeting_opens_the_menu_without_a_dungeon_return() {
    use crate::shop_runtime::ShipBrokerState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 3, 4);
    state.gold = 700;
    state.active_shop = Some(ActiveShopSession::ShipBroker(
        ShipBrokerState::for_shipwright(Shipwright::TheRustyBucket),
    ));
    // A town visit, so there is no dungeon return record.
    state.return_world = None;

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(
        !state.message.trim().is_empty(),
        "Y at the greeting must print the Frigate/Skiff menu, got {:?}",
        state.message
    );
}

#[test]
fn end_to_end_shipwright_frigate_queues_published_dock_delivery() {
    use crate::shop_runtime::ShipBrokerState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 3, 4);
    state.gold = 770;
    state.active_shop = Some(ActiveShopSession::ShipBroker(
        ShipBrokerState::menu_for_shipwright(Shipwright::TheRustyBucket),
    ));
    state.return_world = Some(WorldReturn {
        plane: WorldPlane::Britannia,
        x: 12,
        y: 21,
        transport: TransportState::Foot,
        sail_cadence: 0,
        grid: open_world_grid(),
        active_objects: vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 12,
            y: 21,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        pending_vehicle: None,
    });

    handle_play_key_input(&mut state, 'F', "", Path::new("")).unwrap();
    // `shops.md §6.1`: the base row is adjusted by the speaker's
    // Intelligence before it is quoted.
    assert!(state.message.contains("770 gold") || state.message.contains("770"));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 0);
    // `shops.md §8.C`: records `123` and `124` (absent without `SHOPPE.DAT`),
    // then the resident `sir?" ` continuation; the session stays open on it.
    assert!(state.message.ends_with("sir?\" "), "{:?}", state.message);
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ShipBroker(ShipBrokerState::Continuation { .. }))
    ));
    assert_eq!(
        state
            .return_world
            .as_ref()
            .and_then(|world| world.pending_vehicle),
        // `shops.md §8.7`: The Rusty Bucket's published delivery cell.
        Some(PendingVehicleAcquisition::Frigate {
            x: 138,
            y: 159,
            skiffs: 2,
        })
    );
}

/// `shops.md §8.7`/`§8.C`: a Frigate while a delivery is pending is the
/// record-`125` quote; Yes tests the fixed `10000`, so it refuses (record
/// `122` plus `yells $.`), ends the visit and leaves gold and the pending
/// watercraft unchanged.
#[test]
fn end_to_end_shipwright_pending_frigate_dock_space_always_refuses() {
    use crate::shop_runtime::ShipBrokerState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 3, 4);
    state.gold = 9999;
    state.active_shop = Some(ActiveShopSession::ShipBroker(
        ShipBrokerState::menu_for_shipwright(Shipwright::TheRustyBucket),
    ));
    let pending = PendingVehicleAcquisition::Skiff {
        x: 138,
        y: 159,
        aux3: 0,
    };
    state.return_world = Some(WorldReturn {
        plane: WorldPlane::Britannia,
        x: 12,
        y: 21,
        transport: TransportState::Foot,
        sail_cadence: 0,
        grid: open_world_grid(),
        active_objects: Vec::new(),
        pending_vehicle: Some(pending),
    });

    handle_play_key_input(&mut state, 'F', "", Path::new("")).unwrap();
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::ShipBroker(ShipBrokerState::ConfirmDockSpace { .. }))
    ));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
    assert_eq!(state.gold, 9999);
    assert!(!state.message.contains("dock space"), "{:?}", state.message);
    assert_eq!(
        state.return_world.as_ref().and_then(|world| world.pending_vehicle),
        Some(pending)
    );
}

/// `shops.md §8.C` "Shipwrights": "Record `119` is the F/S menu. F and S
/// echo with `\n\n`". The letter lands on the menu's open prompt row, that
/// row is closed and one blank row follows before the offer record.
#[test]
fn shipwright_menu_letter_echo_is_followed_by_a_blank_row() {
    use crate::shop_runtime::ShipBrokerState;
    use crate::shop_session::ActiveShopSession;

    for letter in ['F', 'S'] {
        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 50;
        state.active_shop = Some(ActiveShopSession::ShipBroker(
            ShipBrokerState::menu_for_shipwright(Shipwright::TheRustyBucket),
        ));
        state.return_world = None;
        // Record `119` ends on its own prompt with the cursor mid-row.
        state.emit_message_line_continuing_row("like to see?\" ");
        state.open_message_row();

        handle_play_key_input(&mut state, letter, "", Path::new("")).unwrap();
        let rows: Vec<(String, bool)> = state
            .message_transcript
            .iter()
            .map(|entry| (entry.text.clone(), entry.explicit_blank))
            .collect();
        let echo = rows
            .iter()
            .position(|(text, _)| text == &format!("like to see?\" {letter}"))
            .unwrap_or_else(|| panic!("{letter} must echo onto the menu row: {rows:?}"));
        assert!(
            rows.get(echo + 1).is_some_and(|(text, blank)| *blank && text.is_empty()),
            "{letter}'s `\\n\\n` echo leaves one blank row under it: {rows:?}"
        );
    }
}

/// `animation.md §6` (spec HEAD `c00bf63`): "These are render
/// selectors, not map edits; the authored map byte remains the
/// phase-zero tile id." The grid must never be rewritten for an
/// animated cell.
///
/// This test used to drive tile `1` (water) and read the change out
/// of the text view's `~`/`=` glyphs. Both halves are withdrawn:
/// water does not animate, and every id in every surviving family
/// shares a text-view glyph with its own frames, so the assertion
/// reads the resolved selector directly.
#[test]
fn render_resolves_static_animation_without_mutating_grid() {
    let mut grid = open_world_grid();
    // 0xD4: first id of the waterfall family.
    grid[world_cell_index(6, 5)] = 0xD4;
    let mut state = britannia_state(grid, 5, 5);
    state.ambient_light = FULL_DAYLIGHT;

    let _ = state.render_text_view(1);
    assert_eq!(state.grid[world_cell_index(6, 5)], 0xD4);
    assert_eq!(state.animation.resolve_static_tile(0xD4), 0xD4);

    for expected in [0xD5u8, 0xD6, 0xD7, 0xD4] {
        state.animation.tick_static_tiles();
        let _ = state.render_text_view(1);
        assert_eq!(
            state.grid[world_cell_index(6, 5)],
            0xD4,
            "the authored map byte must stay the phase-zero tile id"
        );
        assert_eq!(state.animation.resolve_static_tile(0xD4), expected);
    }
}

#[test]
fn active_object_phase_respects_steady_countdown_and_decision() {
    let mut steady = ActiveObject {
        type_byte: PLAYER_TILE,
        tile: PLAYER_TILE,
        x: 0,
        y: 0,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    assert_eq!(steady.tick_phase(), PhaseTick::Steady);
    assert_eq!(steady.phase, STEADY_PHASE);

    let mut animated = ActiveObject {
        type_byte: PLAYER_TILE,
        tile: PLAYER_TILE,
        x: 0,
        y: 0,
        z: 0,
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    };
    assert_eq!(animated.tick_phase(), PhaseTick::Countdown);
    assert_eq!(animated.phase, 0x21);

    let mut decision = ActiveObject {
        type_byte: PLAYER_TILE,
        tile: PLAYER_TILE,
        x: 0,
        y: 0,
        z: 0,
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    };
    assert_eq!(decision.tick_phase(), PhaseTick::DecisionPoint);
    assert_eq!(decision.phase, 0x20);
}

/// Issue #298 Q29: the displayed frame is written only by the animator's
/// script steps; the outdoor walker's countdown leaves it alone.
#[test]
fn active_object_walker_countdown_leaves_the_frame_to_the_animator() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    });

    state.advance_turn();

    state.apply_pending_world_object_walker_pass();

    let object = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == 168)
        .unwrap();
    // `active-objects.md §3`/`§8`: byte 6 is "Advanced by the animator"; the
    // walker "performs no drawing, no frame advance and no cleanup at all",
    // so neither the countdown nor the frame moves on a walker pass.
    assert_eq!(object.phase, 0x22);
    assert_eq!(object.tile, 168);
}

/// Issue #298 Q29: the walker no longer re-derives the frame from byte 6,
/// whose high nibble is the script position, not a frame selector.
#[test]
fn active_object_walker_decision_point_leaves_the_frame_to_the_animator() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 171,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    });

    state.advance_turn();

    state.apply_pending_world_object_walker_pass();

    let object = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == 168)
        .unwrap();
    assert_eq!(object.phase, 0x20);
    assert_eq!(object.tile, 171);
}

#[test]
fn active_ship_drifts_with_matching_wind() {
    // Issue #298 Q32: the wind-driven family is `0x2C..0x2F`, `0x2D` is the
    // east frame, and the cadence counter is record byte 7.
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 5);
    state.wind = WindState::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0x2d,
        tile: 0x2d,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    });

    state.advance_turn();

    state.apply_pending_world_object_walker_pass();

    let object = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == 0x2d)
        .unwrap();
    assert_eq!((object.x, object.y), (6, 5));
    // `weather.md §7`: an east frame in an east wind faces the wind
    // source, so it runs the 2-of-3 cadence. The move spends one of
    // the two, recorded in byte 7; byte 6 plays no part.
    assert_eq!(object.aux3, 1);
    assert_eq!(object.phase, 0x20);
    assert_eq!(object.tile, 0x2d);
}

#[test]
fn active_ship_stalls_without_wind() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    });

    state.advance_turn();

    let object = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == 168)
        .unwrap();
    assert_eq!((object.x, object.y), (5, 5));
    assert_eq!(object.phase, 0x20);
}

#[test]
/// `weather.md §7`: a west frame in an east wind faces away from the
/// wind source, so it runs the 3-of-4 cadence - "moves on three
/// eligible passes, then resets and skips one". The counter is record
/// byte 7 and the west frame is `0x2F` (issue #298 Q32).
///
/// This replaced an earlier pin on a one-turn phase countdown that
/// let the same frame move only every other turn.
fn active_ship_away_from_wind_uses_three_of_four_cadence() {
    let mut state = world_state(vec![1; WORLD_CELLS], 10, 10);
    state.wind = WindState::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0x2f,
        tile: 0x2f,
        x: 20,
        y: 10,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x60,
        aux1: 0,
        aux3: 0,
    });

    let mut moved = Vec::new();
    for _ in 0..8 {
        let before = {
            let object = state
                .active_objects
                .iter()
                .find(|object| object.type_byte == 0x2f)
                .unwrap();
            (object.x, object.y)
        };
        state.advance_turn();
        state.apply_pending_world_object_walker_pass();
        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 0x2f)
            .unwrap();
        moved.push((object.x, object.y) != before);
    }

    assert_eq!(
        moved,
        vec![true, true, true, false, true, true, true, false]
    );
}

#[test]
fn active_ship_drift_respects_water_and_player_collision() {
    let mut terrain_blocked = world_state(open_world_grid(), 10, 10);
    terrain_blocked.wind = WindState::East;
    terrain_blocked.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    });

    terrain_blocked.advance_turn();
    let object = terrain_blocked
        .active_objects
        .iter()
        .find(|object| object.type_byte == 168)
        .unwrap();
    assert_eq!((object.x, object.y), (5, 5));

    let mut player_blocked = world_state(vec![1; WORLD_CELLS], 6, 5);
    player_blocked.wind = WindState::East;
    player_blocked.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    });

    player_blocked.advance_turn();
    let object = player_blocked
        .active_objects
        .iter()
        .find(|object| object.type_byte == 168)
        .unwrap();
    assert_eq!((object.x, object.y), (5, 5));
}

#[test]
fn escape_is_ignored_at_the_adjacent_tile_direction_prompt() {
    // `input.md §10`: "**Escape does not cancel this prompt.** Space is
    // the only pass key here. The original contains a cancel arm for
    // Escape, but its accept filter never releases the key to that arm,
    // so pressing Escape simply causes another read like any other
    // rejected key." `commands.md §5.4`: "Escape does not reach a
    // cancellation arm: it emits nothing and the prompt reads again. An
    // earlier revision of this table listed `Space` **or** `Esc` as
    // producing `Pass` and a cancelled result ... both are retracted."
    let dir = debug_game_dir();

    for (key, prefix) in [
        ('G', "Get-"),
        ('J', "Jimmy-"),
        ('O', "Open-"),
        ('S', "Search-"),
        ('T', "Talk-"),
        ('L', "Look-"),
    ] {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::South;
        assert_eq!(
            handle_play_key_input(&mut state, key, "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.message, prefix);

        // Escape: no echo, no result, prompt still waiting, no turn.
        assert_eq!(
            handle_play_key_input(&mut state, '\u{1b}', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(
            state.active_direction_prompt.is_some(),
            "{prefix} must still be waiting after Escape"
        );
        assert_eq!(state.message, prefix, "{prefix} must not echo for Escape");
        assert_ne!(state.message, "Pass");
        assert_eq!(state.turn, 0);

        // Space remains the one pass key, and it still echoes `Pass`.
        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_none());
        // `commands.md §5.4`: `Space` prints `Pass` on the open verb line -
        // "A cancelled Look therefore renders as the verb, the hyphen and
        // the cancel word **on one line**" - so the slot holds the whole
        // row, not a second bare `Pass` under it.
        assert_eq!(state.message, format!("{prefix}Pass"));
    }

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn escape_is_ignored_at_the_shared_spell_direction_prompt() {
    // `input.md §11`: "Other keys are ignored and the prompt reads again
    // ... Escape is not the confirmed cancellation key for this shared spell
    // prompt." `inventory.md §7.1`: the Wind Change scroll and the Skull Key
    // use that prompt; `magic.md §8`: it "cannot be escaped except by a
    // cardinal or Space". (Audit A096.)
    let dir = debug_game_dir();
    for pending in [
        UsePendingAction::SkullKeyDirection,
        UsePendingAction::ScrollWindDirection {
            index: SCROLL_WIND_CHANGE_INDEX,
        },
    ] {
        let mut state = test_state(open_grid(), 4, 4);
        state.active_use = Some(UseSession {
            cursor: 0,
            pending: Some(pending),
            strip_scrolls: 0,
        });
        assert!(state.step_active_use('\u{1b}', "", &dir).unwrap());
        assert_eq!(
            state.active_use.and_then(|session| session.pending),
            Some(pending),
            "{pending:?} must still be waiting after Escape"
        );
        assert_eq!(state.turn, 0);
    }

    for spell in ["HR", "IP", "AY"] {
        let mut state = test_state(open_grid(), 4, 4);
        state.active_cast_followup = Some(CastFollowupSession::new(
            0,
            spell.to_string(),
            CastFollowupKind::Direction { pass_allowed: true },
            None,
            false,
        ));
        let _ = state.step_active_cast_followup('\u{1b}', "", &dir).unwrap();
        assert!(
            state.active_cast_followup.is_some(),
            "{spell}: Escape must not cancel the Direction- prompt"
        );
        assert_ne!(state.message, "None!");
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn escape_is_ignored_at_the_push_direction_prompt() {
    // `commands.md §8.1` row A: "Escape pressed at the direction prompt
    // | `Push-` remains open; Escape emits no byte | Prompt remains
    // active; there is no cancellation or continuation".
    let dir = debug_game_dir();
    let mut grid = open_grid();
    grid[32 + 2] = 0x5b;
    let mut push = test_state(grid, 1, 1);
    push.player.facing = Direction::South;
    assert_eq!(
        handle_play_key_input(&mut push, 'P', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(push.message, "Push-");

    assert_eq!(
        handle_play_key_input(&mut push, '\u{1b}', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(push.active_direction_prompt.is_some());
    assert_eq!(push.message, "Push-");
    assert_eq!(push.turn, 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn completed_inn_rest_sleeps_in_the_bed_cell_and_wakes_one_tile_east() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    // `shops.md §8.4` (issue #190), the first of the rest's three
    // world-state effects: "The party's map position is written to the
    // inn's bed cell for the duration, so the party is standing on the
    // bed while the sequence plays", and "on the completed-rest path the
    // handler steps the party **one tile east** of the bed cell before
    // returning".
    //
    // "**The floor byte is not written.** The rest happens on whatever
    // floor the inn menu was opened from. Do not reset the party to the
    // entry floor as part of the rest." The party starts on floor 2 here
    // for exactly that reason.
    let inn = Inn::TheSmugglersInn;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 2,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ConfirmRest {
        inn,
        base_room_rate: inn.base_room_rate(),
        total_price: 4,
    }));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert!(state.message.ends_with(&format!(
        "{}\n\n{}",
        crate::commands::PAID_INN_REST_SLEEP_LINE,
        crate::commands::PAID_INN_REST_MORNING_LINE
    )));
    let (wake_x, wake_y) = inn.bed_wake_cell();
    assert_eq!(
        (state.player.x, state.player.y),
        (usize::from(wake_x), usize::from(wake_y)),
        "the party wakes one tile east of the bed cell, not in it"
    );
    assert_ne!(
        (state.player.x, state.player.y),
        (
            usize::from(inn.bed_cell().0),
            usize::from(inn.bed_cell().1)
        )
    );
    assert_eq!(
        state.area,
        Area::Town {
            scene: Scene::new(22).unwrap(),
            floor: 2,
        },
        "the floor byte is not written by the rest"
    );
}

#[test]
fn refused_or_unaffordable_inn_rest_never_moves_the_party() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    // `shops.md §8.4` (issue #190): "The eastward step is on the
    // completed path only. All three of the handler's early exits - the
    // pre-menu helper declining, an answer other than `Y` at the
    // confirmation prompt, and gold below the quoted charge - return
    // without moving the party at all. A refused or unaffordable stay
    // never moves the party into the bed cell at all, and leaves it
    // exactly where it stood."
    let inn = Inn::TheWayfarerInn;

    // Answer other than `Y`.
    let mut declined = test_state(open_grid(), 1, 1);
    declined.gold = 200;
    declined.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ConfirmRest {
        inn,
        base_room_rate: inn.base_room_rate(),
        total_price: 2,
    }));
    handle_play_key_input(&mut declined, ' ', "n", Path::new("")).unwrap();
    assert_eq!((declined.player.x, declined.player.y), (1, 1));
    assert_eq!(declined.gold, 200);

    // Gold below the quoted charge.
    let mut broke = test_state(open_grid(), 1, 1);
    broke.gold = 1;
    broke.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ConfirmRest {
        inn,
        base_room_rate: inn.base_room_rate(),
        total_price: 99,
    }));
    handle_play_key_input(&mut broke, 'Y', "", Path::new("")).unwrap();
    assert_eq!(
        (broke.player.x, broke.player.y),
        (1, 1),
        "an unaffordable stay never moves the party into the bed cell"
    );
    assert_eq!(broke.gold, 1);
}

/// `shops.md §8.C` sage result table: "empty input returns to tavern
/// continuation without an extra farewell" and "decline returns to tavern
/// continuation" (R436); only short funds ends the visit.
#[test]
fn sage_empty_topic_and_declined_fee_return_to_tavern_continuation() {
    use crate::shop_runtime::{SageState, TavernState};
    use crate::shop_session::ActiveShopSession;

    for declined in [false, true] {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(22).unwrap(),
            floor: 0,
        };
        state.gold = 200;
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        if declined {
            handle_play_key_input(&mut state, 'H', "ONE", Path::new("")).unwrap();
            handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
        } else {
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
        }

        assert_eq!(state.gold, 200);
        assert!(
            state.message.contains("Anything else\nfor thee?"),
            "{:?}",
            state.message
        );
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::Tavern(TavernState::AnythingElse {
                tavern: Tavern::TheCatsLair,
                continuation_ready: true,
            }))
        ));
    }
}

/// `shops.md §8.C` tavern result table, "Cannot afford that bill": the bill,
/// then `"\n\n"CAN'T PAY?\nBeat it!"\nyells `, vendor name, `.\n`; the visit
/// ends without the shared farewell.
#[test]
fn tavern_unaffordable_round_prints_cant_pay_and_ends_the_visit() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 0;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
        tavern: Tavern::TheCatsLair,
        continuation_ready: false,
    }));
    let round = crate::shops::tavern_menu_letters(Tavern::TheCatsLair).round;

    handle_play_key_input(&mut state, round, "", Path::new("")).unwrap();

    assert!(
        state
            .message
            .ends_with("\"\n\n\"CAN'T PAY?\nBeat it!\"\nyells Dr. Cat.\n"),
        "{:?}",
        state.message
    );
    assert!(state.message.contains("\"That will be"));
    assert!(state.active_shop.is_none());
}

/// `shops.md §8.C`/`§8.4`: Rest and Leave first check this inn's lodged-guest
/// capacity; Leave refuses Avatar (repeat the question) and a dead companion
/// (record `192`, visit ends); Escape at the picker prints `Nobody`.
#[test]
fn inn_capacity_and_leave_target_refusals_follow_the_published_rules() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    let companion = |slot: u8, status: u8| PartyMember {
        slot,
        class_byte: b'B',
        status,
        climb_stat: 7,
        mana: 3,
        hp: if status == b'D' { 0 } else { 12 },
        max_hp: 28,
        level: 3,
    };
    let fresh = || {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(20).unwrap(),
            floor: 0,
        };
        state.gold = 100;
        let avatar = state.party[0];
        state.party = vec![avatar, companion(1, b'G'), companion(2, b'D')];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"SHAMINO\0\0"];
        state.party_stay_counters = vec![0; 3];
        state.party_strengths = vec![30; 3];
        state.party_intelligence = vec![30; 3];
        state.party_experience = vec![0; 3];
        state.party_equipment = default_party_equipment(3);
        state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ServiceMenu {
            inn: Inn::HotelBrittany,
        }));
        state
    };

    // Hotel Brittany lodges at most two guests.
    let mut state = fresh();
    let guest = InnGuestRecord {
        registry_slot: 0,
        scene_marker: 20,
        name: *b"DUPRE\0\0\0\0",
        member: companion(4, b'G'),
        strength: 17,
        intelligence: 19,
        experience: 0,
        equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
        stay_counter: 0,
    };
    state.inn_registry = vec![guest, InnGuestRecord { registry_slot: 1, ..guest }];
    for key in ['R', 'L'] {
        state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ServiceMenu {
            inn: Inn::HotelBrittany,
        }));
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(
            state.message.contains("but we\nhave no room\navailable."),
            "{:?}",
            state.message
        );
        assert!(matches!(
            state.active_shop,
            Some(ActiveShopSession::Innkeeper(InnkeeperState::Continuation { .. }))
        ));
    }
    assert_eq!(state.gold, 100);

    // Avatar cannot stay: the plural refusal, then the question again.
    let mut state = fresh();
    handle_play_key_input(&mut state, 'L', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
    // `inventory.md §4.3`: the digit only moves the highlight; Return commits.
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(
        state.message.contains("Thy friends will not leave thee!"),
        "{:?}",
        state.message
    );
    assert!(state.message.contains("\"Who will\nstay?\" "));
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Innkeeper(InnkeeperState::PickLeaveCompanion { .. }))
    ));
    // Escape cancels with `Nobody` and reaches the continuation.
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Innkeeper(InnkeeperState::Continuation { .. }))
    ));

    // A dead companion ends the visit (record 192) without leaving anyone.
    let mut state = fresh();
    handle_play_key_input(&mut state, 'L', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '3', "", Path::new("")).unwrap();
    // `inventory.md §4.3`: the digit only moves the highlight; Return commits.
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(state.active_shop.is_none());
    assert_eq!(state.party.len(), 3);
    assert!(state.inn_registry.is_empty());
}

/// `shops.md §8.C`: multi-guest pickup asks "Who will check out?" over the
/// register panel, takes no Y/N confirmation, and short funds render record
/// `193` and end the visit.
#[test]
fn inn_multi_guest_pickup_bills_without_confirmation() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(2).unwrap(),
        floor: 0,
    };
    let member = PartyMember {
        slot: 4,
        class_byte: b'B',
        status: b'G',
        climb_stat: 7,
        mana: 3,
        hp: 12,
        max_hp: 28,
        level: 3,
    };
    let guest = InnGuestRecord {
        registry_slot: 0,
        scene_marker: 2,
        name: *b"IOLO\0\0\0\0\0",
        member,
        strength: 17,
        intelligence: 19,
        experience: 0,
        equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
        stay_counter: 0,
    };
    state.inn_registry = vec![guest, InnGuestRecord { registry_slot: 1, ..guest }];
    let bill = crate::shops::inn_pickup_bill_for_speaker(
        Inn::TheWayfarerInn,
        0,
        crate::input_dispatch::active_speaker_intelligence(&state),
    );
    state.gold = bill;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ServiceMenu {
        inn: Inn::TheWayfarerInn,
    }));

    handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
    assert!(state.message.contains("\"Who will\ncheck out?\" "), "{:?}", state.message);
    // `shops.md §8.C` "Guest Register keys" (u5-spec#299 Q15): "Digits
    // never move the highlight and never commit"; South moves it down and
    // Return commits.
    handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();
    assert_eq!(state.party.len(), 1);
    handle_play_key_input(&mut state, char::from(crate::INPUT_CODE_SOUTH), "", Path::new(""))
        .unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert_eq!(state.gold, 0);
    assert_eq!(state.party.len(), 2);
    assert!(state.message.contains("I hope thou hast found thy stay enjoyable,"));
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Innkeeper(InnkeeperState::Continuation { .. }))
    ));

    // Yes returns to the service question; the last guest's bill is short.
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
    assert_eq!(state.party.len(), 2);
    assert!(state.active_shop.is_none());
}

/// `shops.md §8.C` / §8.3 (u5-spec#298 Q19): "Ordinary short funds | Record
/// `173`, then the continuation question below, exactly as after a declined
/// fee; no gold, status or HP changes." The visit continues.
#[test]
fn paid_healer_short_of_funds_asks_the_continuation_question() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 59;
    state.party[0].status = b'G';
    state.party[0].hp = 5;
    state.party[0].max_hp = 22;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::TheShieldOfTruth,
    ));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap();
    assert!(state.message.contains("60 gold"));
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert_eq!(state.gold, 59);
    assert_eq!(state.party[0].hp, 5);
    assert_eq!(state.party[0].status, b'G');
    assert!(
        state
            .message
            .ends_with("\n\n\"Is there any other way in which I may\naid thee?\" "),
        "{:?}",
        state.message
    );
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Healer(HealerShopState::Continuation, _))
    ));
    // "Yes returns to the service question".
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.contains("What is the nature of thy need?"));
}

/// `shops.md §8.C` (u5-spec#298 Q19): "in Skara Brae (scene `7`, The Spirit
/// Healers) only, when the fee is at most 100 and party gold is below it,
/// record `173` is not printed, the treatment is applied with no gold
/// deducted, the post-payment surcharge step does not run".
#[test]
fn spirit_healers_low_fee_charity_treats_without_charging() {
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 44;
    state.party[0].status = b'G';
    state.party[0].hp = 5;
    state.party[0].max_hp = 22;

    let outcome = state
        .buy_healer_treatment(Healer::TheSpiritHealers, HealerTreatment::Heal, 0)
        .expect("the charity treats the member");
    assert!(outcome.charity);
    assert_eq!(state.gold, 44);
    assert_eq!(state.party[0].hp, 22);

    // A fee above 100 is an ordinary short-funds refusal, even there.
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    assert!(matches!(
        state.buy_healer_treatment(Healer::TheSpiritHealers, HealerTreatment::Resurrect, 0),
        Err(HealerTreatmentError::InsufficientGold { .. })
    ));
    // And no other healer is charitable.
    state.party[0].status = b'G';
    state.party[0].hp = 5;
    assert!(!healer_low_fee_charity_applies(Healer::TheShieldOfTruth, 60, 0));
}

/// `shops.md §8.C`: after a paid round the continuation question
/// `"Anything else\nfor thee?" ` waits on its own row, and "Y prints `Yes`"
/// onto it. The round's text carries embedded blank rows, and flushing them
/// left the row-open flag closed, so once the continuing-row emitter honoured
/// that flag `Yes` dropped to a row of its own. Measured 2026-10-16
/// (`paws-sage`, beat `again`): the original reads `for thee?" Yes`.
#[test]
fn tavern_continuation_yes_echoes_onto_the_question_row() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
        tavern: Tavern::TheCatsLair,
        continuation_ready: false,
    }));
    let round = crate::shops::tavern_menu_letters(Tavern::TheCatsLair).round;
    handle_play_key_input(&mut state, round, "", Path::new("")).unwrap();
    assert!(
        state.message.ends_with("Anything else\nfor thee?\" "),
        "{:?}",
        state.message
    );

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(
        rows.iter().any(|row| row.ends_with("for thee?\" Yes")),
        "{rows:?}"
    );
    let question = rows
        .iter()
        .position(|row| *row == "\"Anything else")
        .expect("the continuation question is on the transcript");
    assert_eq!(rows[question + 1], "for thee?\" Yes", "{rows:?}");
}

/// `shops.md §8.B`: "the accepted answer appends directly after" the entry
/// greeting's continuation, so `Y` at a `:` row reads `:Yes`.
#[test]
fn shop_entry_answer_echoes_onto_the_greeting_row() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Greeting {
        tavern: Tavern::TheCatsLair,
    }));
    state.emit_message_line("\n\"Welcome!\"\n\n:");
    state.close_message_row();

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(rows.contains(&":Yes"), "{rows:?}");
}

/// `shops.md §8.B`: "Space means No where accepted. Every No response echoes
/// `No` without a line feed", appended to the greeting's continuation row.
/// Measured 2026-10-16 (`shop-inn-after-entry`, beat `after-space`): the
/// original reads `:No`; this engine put `No` on a row of its own.
#[test]
fn inn_entry_space_echoes_no_onto_the_greeting_row() {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::Greeting {
        inn: Inn::TheWayfarerInn,
    }));
    state.emit_message_line("\n\"Welcome!\"\n\n:");
    state.close_message_row();

    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();

    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(rows.contains(&":No"), "{rows:?}");
}

/// `shops.md §8.C`: the bill follows "the action-letter echo and `\n\n`".
/// The letter lands on the menu's open row (`thy travels?" A`), so the first
/// feed closes that row and the second leaves the blank the original shows
/// above `"That will be` (`paws-sage`, beat `ale`).
#[test]
fn tavern_menu_letter_echo_is_followed_by_a_blank_row() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
        tavern: Tavern::TheCatsLair,
        continuation_ready: false,
    }));
    state.emit_message_line("\n\"What'll it be?\" ");
    state.open_message_row();
    let round = crate::shops::tavern_menu_letters(Tavern::TheCatsLair).round;
    handle_play_key_input(&mut state, round, "", Path::new("")).unwrap();
    state.flush_message_slot();

    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let echo = rows
        .iter()
        .position(|row| *row == format!("\"What'll it be?\" {round}"))
        .unwrap_or_else(|| panic!("the letter echoes onto the menu row: {rows:?}"));
    assert_eq!(rows[echo + 1], "", "{rows:?}");
    assert!(rows[echo + 2].starts_with("\"That will be"), "{rows:?}");
}

/// `shops.md §8.5`: "Space, Escape, or Return from that wait exits directly
/// to the same no-sale closing-bark row", and `§8.A`'s nothing-bought row is
/// `61..64` inside the shared `says $.` envelope. Exiting the fresh list -
/// not only the wait a follow-up record resumes - used to end the visit with
/// no bark at all (playtest E51).
#[test]
fn tavern_fresh_list_exit_keys_draw_the_no_sale_closing_bark() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[
        (61, b"Asset no-sale.\"".as_slice()),
        (62, b"Asset no-sale.\"".as_slice()),
        (63, b"Asset no-sale.\"".as_slice()),
        (64, b"Asset no-sale.\"".as_slice()),
    ]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
    for key in [' ', '\u{1b}', '\r'] {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(22).unwrap(),
            floor: 0,
        };
        state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
            tavern: Tavern::TheCatsLair,
            continuation_ready: false,
        }));
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
        assert_eq!(
            state.message, "\n\n\"Asset no-sale.\"\nsays Dr. Cat.",
            "key {key:?}"
        );
        assert!(state.active_shop.is_none());
    }
    let _ = fs::remove_dir_all(dir);
}

/// `shops.md §8.C`, "Fourth secondary drink attempt, when prior count is
/// exactly three": the warning carries a `says <vendor>.` row between its two
/// quoted parts. It went straight from `pardon, sir,"` to `"But haven't`
/// (playtest E53).
#[test]
fn tavern_fourth_drink_warning_is_attributed_to_the_vendor() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.tavern_secondary_drink_count = 3;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::PostListWait {
        tavern: Tavern::TheCatsLair,
        continuation_ready: true,
    }));
    let secondary = crate::shops::tavern_menu_letters(Tavern::TheCatsLair).secondary;
    handle_play_key_input(&mut state, secondary, "", Path::new("")).unwrap();
    assert_eq!(
        state.message,
        "\n\n\"I beg thy\npardon, sir,\"\nsays Dr. Cat.\n\"But haven't\nye had enough\nto drink?\" "
    );
}

/// `shops.md §8.C` sage result table: "Unknown topic | `"That, I cannot help
/// thee with.\n\n`, then repeat the topic question beginning with `Of what
/// wouldst`". The reply had no opening quote and no repeated question
/// (playtest E55); the prompt itself keeps waiting for another topic.
#[test]
fn sage_unknown_topic_quotes_its_reply_and_repeats_the_question() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

    handle_play_key_input(&mut state, 'Z', "ZZ", Path::new("")).unwrap();

    assert_eq!(
        state.message,
        "\"That, I cannot help thee with.\n\nOf what wouldst thou hear my lore, sir?\"\n\nYou respond:"
    );
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Sage(SageState::Prompt { .. }))
    ));
    assert_eq!(state.gold, 200);
}

/// `shops.md §8.C`: sage "Short funds | `"Sorry, `, honorific, record `91`"
/// and tavern "No packs affordable, fewer than three food servings | `\n\n`,
/// then record `90`", each record's `$` substituting the vendor (`§4.1`).
/// Both printed the bare record with an empty vendor (playtest E50).
#[test]
fn sage_short_funds_and_table_scraps_name_the_vendor() {
    use crate::shop_runtime::{SageState, TavernState};
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[
        (90, b"Asset scraps.\"\norders $.".as_slice()),
        (91, b",\nAsset busy.\"\nsays $.".as_slice()),
    ]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 0;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
    handle_play_key_input(&mut state, 'C', "OMP", &dir).unwrap();
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    assert_eq!(
        state.message,
        "\"Sorry, sir,\nAsset busy.\"\nsays Dr. Cat."
    );
    assert!(state.active_shop.is_none());

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 0;
    state.food = 2;
    state.active_shop = Some(ActiveShopSession::Tavern(
        TavernState::PickProvisionQuantity {
            tavern: Tavern::TheCatsLair,
            unit_price: 20,
            continuation_ready: false,
            entry: Default::default(),
        },
    ));
    handle_play_key_input(&mut state, '1', "\r", &dir).unwrap();
    assert_eq!(state.message, "\n\nAsset scraps.\"\norders Dr. Cat.");
    assert!(state.active_shop.is_none());

    let _ = fs::remove_dir_all(dir);
}

/// `shops.md §8.C`: "The Blue Boar list begins `"Our wine list,\n`,
/// honorific, `.\n\n`, then" the six rows and `Thy choice?"`. The heading
/// and its opening quote were missing (playtest E52).
#[test]
fn blue_boar_wine_list_opens_with_its_quoted_heading() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
        tavern: Tavern::TheBlueBoarTavern,
        continuation_ready: false,
    }));
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    assert!(
        state
            .message
            .starts_with("\"Our wine list,\nsir.\n\na) Rose.......18\n"),
        "{:?}",
        state.message
    );
    assert!(state.message.ends_with("f) Chablis....98\n\nThy choice?\" "));
}

/// `shops.md §8.5`, "Which reader takes the number": the provision question
/// is the shared two-digit reader - "two digits accepted and a third
/// dropped", Backspace erasing one, "Return the only exit with an empty
/// entry submitting as zero". `§8.5`/`§8.C`: a completed or partial purchase
/// then asks `"Anything else\nfor thee?" `. The engine bought on the first
/// digit, ignored a bare Return, and asked nothing after a sale (playtest
/// E47).
#[test]
fn tavern_provision_quantity_reads_two_digits_until_return() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let fresh = |gold: u16| {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = gold;
        state.food = 0;
        state.active_shop = Some(ActiveShopSession::Tavern(
            TavernState::PickProvisionQuantity {
                tavern: Tavern::TheHonestMeal,
                unit_price: 10,
                continuation_ready: false,
                entry: Default::default(),
            },
        ));
        state.emit_message_line("\nHow many wouldst");
        state.emit_message_line_continuing_row("thou like?\" ");
        state.open_message_row();
        state
    };
    let last_row = |state: &PlayState| state.message_transcript.last().unwrap().text.clone();

    // Two digits wait for Return; a third is dropped; Backspace erases one.
    let mut state = fresh(1000);
    for key in ['1', '2', '3'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert_eq!(state.food, 0, "nothing is bought before Return");
    }
    assert_eq!(last_row(&state), "thou like?\" 12");
    handle_play_key_input(&mut state, '\u{8}', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '3', "", Path::new("")).unwrap();
    assert_eq!(last_row(&state), "thou like?\" 13");
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert_eq!(state.food, 13 * 25);
    assert_eq!(state.gold, 1000 - 130);
    assert_eq!(state.message, "\n\n\"Anything else\nfor thee?\" ");
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Tavern(TavernState::AnythingElse {
            continuation_ready: true,
            ..
        }))
    ));

    // Gold runs out part way: the partial line, then the same question.
    let mut state = fresh(25);
    handle_play_key_input(&mut state, '9', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert_eq!(state.food, 50);
    assert_eq!(
        state.message,
        "\n\n\"Thou canst\nafford only 2!\"\n\n\"Anything else\nfor thee?\" "
    );

    // A bare Return submits zero.
    let mut state = fresh(1000);
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert_eq!(state.food, 0);
    assert!(state.message.starts_with("\n\n\"Hrumph.\""), "{:?}", state.message);
}

/// `input.md §8`: in the free-text reader "Escape clears the whole current
/// line but does **not** submit or terminate it", and `text-output.md §10.6`
/// makes that true of every reader. The sage's topic line and the well's wish
/// line both submitted the Escape as their typed text: the sage answered
/// "That, I cannot help thee with." and the well spent the wish on
/// `No effect...` (playtest G15).
#[test]
fn escape_at_the_sage_topic_and_well_wish_lines_submits_nothing() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let mut sage = test_state(open_grid(), 1, 1);
    sage.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    sage.gold = 200;
    sage.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
    let entries_before = sage.message_entries().len();
    let message_before = sage.message.clone();
    handle_play_key_input(&mut sage, '\u{1b}', "", Path::new("")).unwrap();
    assert_eq!(sage.message_entries().len(), entries_before);
    assert_eq!(sage.message, message_before);
    assert!(matches!(
        sage.active_shop,
        Some(ActiveShopSession::Sage(SageState::Prompt { .. }))
    ));

    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut well = test_state(grid, 1, 1);
    well.player.facing = Direction::East;
    well.gold = 2;
    assert_eq!(well.look_facing(), MoveOutcome::Observed);
    handle_play_key_input(&mut well, 'Y', "", Path::new("")).unwrap();
    let message_before = well.message.clone();
    handle_play_key_input(&mut well, '\u{1b}', "", Path::new("")).unwrap();
    assert_eq!(well.message, message_before);
    assert!(
        well.active_wishing_well
            .as_ref()
            .is_some_and(|session| session.coin_accepted),
        "the wish prompt keeps reading"
    );
    assert_eq!(well.gold, 1);
}

/// `view.md §3`: "The coin prompt accepts Y or N; other keys continue
/// waiting", and `shops.md §8.A`'s shared Y/N row: "Other keys print nothing
/// and keep polling". Escape answered `No` and ended the Look, and any other
/// key printed `Drop a coin?` a second time on a row of its own (playtest
/// G17).
#[test]
fn wishing_well_coin_prompt_waits_silently_on_keys_other_than_y_or_n() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xa1;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.gold = 2;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    let message_before = state.message.clone();
    for key in ['\u{1b}', 'x', ' '] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert_eq!(state.message, message_before, "{key:?}");
        assert!(
            state
                .active_wishing_well
                .as_ref()
                .is_some_and(|session| !session.coin_accepted),
            "{key:?} leaves the coin prompt waiting"
        );
    }
    assert_eq!(state.gold, 2);
    handle_play_key_input(&mut state, 'n', "", Path::new("")).unwrap();
    assert!(state.active_wishing_well.is_none());
}

/// `view.md §3` wish-outcome table: "Empty wish | `Nothing\n`"; `No
/// effect...` is the nonempty unmatched wish's line. A bare Enter at
/// `Thy wish?` printed `No effect...` (playtest G18).
#[test]
fn wishing_well_empty_wish_prints_nothing_line() {
    for enter in ['\r', '\n'] {
        let mut grid = open_grid();
        grid[32 + 2] = 0xa1;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.gold = 2;
        assert_eq!(state.look_facing(), MoveOutcome::Observed);
        assert_eq!(state.step_active_wishing_well('Y', ""), None);
        assert_eq!(
            state.step_active_wishing_well(enter, ""),
            Some(MoveOutcome::Observed)
        );
        assert!(state.active_wishing_well.is_none());
        assert_eq!(state.message, WISHING_WELL_EMPTY_WISH_LINE);
        assert_eq!(state.gold, 1, "the coin is still spent");
    }
}

/// `shops.md §8.8`: the sage's `You respond:` line "accepts up to fifteen
/// characters". All twenty-six letters of a long topic were echoed across
/// two rows (playtest G16).
#[test]
fn sage_topic_echo_is_capped_at_fifteen_characters() {
    use crate::shop_runtime::SageState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
    handle_play_key_input(&mut state, 'a', "bcdefghijklmnopqrstuvwxyz", Path::new("")).unwrap();
    let echoes = state
        .message_entries()
        .iter()
        .filter(|entry| entry.text.contains("ABCDE"))
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    assert_eq!(echoes, vec!["ABCDEFGHIJKLMNO".to_string()]);
    assert!(
        !state
            .message_entries()
            .iter()
            .any(|entry| entry.text.contains('P')),
        "nothing past the fifteenth character is echoed"
    );
}

/// `karma.md §4.1`: a qualifying payment with the counter at one hundred
/// resets it and raises standing by one, and `conversation.md §7.4` makes each
/// following `0x89` a further `+1`. Ava's bonus record is exactly that - the
/// payment, then five raise-standing bytes - and from standing zero it ended
/// at five: the counter was reset but the milestone's `+1` was overwritten by
/// the runner's standing, counted from before the payment (playtest G04).
#[test]
fn town_raw_tlk_payment_milestone_survives_following_raise_standing_bytes() {
    let mut dialogue: HashMap<u16, Vec<String>> = HashMap::new();
    dialogue.insert(
        0x10,
        vec![
            "Ava".to_string(),
            "a beggar".to_string(),
            "Greetings".to_string(),
            "I beg".to_string(),
            "Farewell".to_string(),
            "ALMS".to_string(),
            "placeholder".to_string(),
        ],
    );

    let enc = |s: &str| s.bytes().map(|b| b ^ 0x80).collect::<Vec<u8>>();
    let mut pay_response = vec![0x85, b'0', b'0', b'5'];
    pay_response.extend([crate::tlk_control_codes::TLK_CODE_STANDING_UP; 5]);
    pay_response.extend(enc("Bless thee"));
    pay_response.push(0xFF);

    let mut raw: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    raw.insert(
        0x10,
        vec![
            enc("Ava"),
            enc("a beggar"),
            enc("Greetings"),
            enc("I beg"),
            enc("Farewell"),
            enc("ALMS"),
            pay_response,
        ],
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.gold = 150;
    state.moral_standing = 0;
    state.toll_progress = TOLL_PROGRESS_MILESTONE;
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
            type_byte: TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS,
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None);
    let _ = state.submit_active_conversation_keyword("alms");
    assert_eq!(state.gold, 145);
    assert_eq!(state.toll_progress, 0, "the milestone fired");
    assert_eq!(state.moral_standing, 6, "0 + 1 (milestone) + 5 (0x89 x5)");

    // A later raise-standing run in the same conversation starts from the
    // live standing, milestone included: the session's carried selector
    // must not hand the runner the pre-milestone value again.
    let _ = state.submit_active_conversation_keyword("alms");
    assert_eq!(state.gold, 140);
    assert_eq!(state.toll_progress, 0, "no second milestone");
    assert_eq!(state.moral_standing, 11, "6 + 5 (0x89 x5)");
}
