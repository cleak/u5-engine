// Regressions for the eighth-pass save-file exploratory-playtest findings
// (`p8save`).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// N15: `input.md §6` - "Three things write the setting and nothing else
/// does" (Ctrl+B, the combat toggle, the free-text reader's save/restore) -
/// and `audio.md §3`: the sound setting "is not part of the saved game; a new
/// process starts with sound enabled again". A moongate transit and a
/// location entry rebuild the scene, and both used to arrive with type-ahead
/// off and sound on.
#[test]
fn playtest_n15_scene_rebuilds_keep_typeahead_and_sound() {
    let dir = debug_game_dir();

    // Location entry (`enter_world_target`).
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 7;
    let mut state = world_state(grid, 10, 20);
    state.typeahead_buffer_enabled = true;
    state.music_enabled = false;
    assert_eq!(
        state
            .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );
    assert!(state.typeahead_buffer_enabled, "entry keeps Buffer On");
    assert!(!state.music_enabled, "entry keeps Sound Off");

    // Gate travel (`apply_gate_travel`, the moongate and spell transit).
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.typeahead_buffer_enabled = true;
    state.music_enabled = false;
    state
        .apply_gate_travel(&dir, 0, PlayTarget::Town(scene), 0, (6, 7))
        .unwrap();
    assert!(matches!(state.area, Area::Town { .. }), "{:?}", state.area);
    assert!(state.typeahead_buffer_enabled, "transit keeps Buffer On");
    assert!(!state.music_enabled, "transit keeps Sound Off");
    let _ = fs::remove_dir_all(dir);
}
