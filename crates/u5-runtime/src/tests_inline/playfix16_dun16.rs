// Regressions for the sixteenth exploratory playtest's dungeon-room findings
// (P16-04..P16-09). Each test names the finding and the published sentence
// it rests on.

/// A three-member dungeon party standing south of a `0xF7` room cell in
/// scene 35, stepped north into the room so the room fight is running.
fn playfix16_room_fight(dir: &std::path::Path) -> PlayState {
    let scene = DungeonScene::new(35).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.area = Area::Dungeon { scene, level: 0 };
    handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", dir).unwrap();
    assert!(state.combat_active, "the room fight started");
    state
}

/// P16-04: `formats/dungeon-dat.md §4` patches the trigger after room combat
/// and "also sets a saved room-clear bit keyed by dungeon and room id", with
/// no outcome condition, and `combat.md §14` has the framer discard the
/// round loop's word. A room the party walked out of while foes remained is
/// already `0xA?` for the visit, so its durable bit is set too.
#[test]
fn playfix16_p16_04_a_lost_room_fight_still_sets_the_room_clear_bit() {
    let dir = playfix2_room_game_dir();
    let scene = DungeonScene::new(35).unwrap();
    let mut state = playfix16_room_fight(&dir);
    assert!(!dungeon_room_clear_bit_is_set(
        &state.dungeon_room_clear_bitmap,
        scene,
        7
    ));

    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Defeat);

    assert!(!state.combat_active);
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
    assert!(
        dungeon_room_clear_bit_is_set(&state.dungeon_room_clear_bitmap, scene, 7),
        "the durable bit agrees with the visit-local 0xA? patch"
    );
    let _ = fs::remove_dir_all(dir);
}

/// P16-05: `systems/save-load.md` - "save/load restores room completion by
/// replaying the cleared-room demotion pass" - and `dungeon-mode.md §5`'s
/// "When a level is loaded or reloaded". Resuming inside the dungeon keeps
/// the saved working copy, but a cleared room's other `0xF?` cells in it are
/// demoted; an uncleared room's cell and every other byte are kept.
#[test]
fn playfix16_p16_05_a_save_resume_replays_the_room_clear_demotion() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(37).unwrap();
    let mut buffer = open_dungeon_record();
    buffer[dungeon_cell_index(0, 3, 6)] = 0xf4;
    buffer[dungeon_cell_index(1, 7, 6)] = 0xf4;
    buffer[dungeon_cell_index(0, 5, 5)] = 0xf6;
    buffer[dungeon_cell_index(0, 4, 4)] = 0x68;
    let mut options = PlayOptions {
        target: PlayTarget::Dungeon(scene),
        floor: 0,
        start: Some((2, 6)),
        saved_dungeon_working_buffer: Some(buffer.clone()),
        ..PlayOptions::default()
    };
    assert!(set_dungeon_room_clear_bit(
        &mut options.dungeon_room_clear_bitmap,
        scene,
        4
    ));

    let state = PlayState::load_dungeon_scene(&dir, scene, options).unwrap();

    assert_eq!(state.grid[dungeon_cell_index(0, 3, 6)], 0xa4);
    assert_eq!(state.grid[dungeon_cell_index(1, 7, 6)], 0xa4);
    assert_eq!(state.grid[dungeon_cell_index(0, 5, 5)], 0xf6, "room 6 is not cleared");
    assert_eq!(state.grid[dungeon_cell_index(0, 4, 4)], 0x68, "saved bytes are kept");
    let _ = fs::remove_dir_all(dir);
}
