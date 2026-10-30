// Regressions for the pass-21 exploratory playtest findings (misc21).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// Playtest P21-05. `view.md §3`: "Matching wish in a granting scene |
/// `\nPoof!\n`, then the sound/object creation"; `No effect...` belongs only
/// to an unmatched wish or an ineligible scene. A horse already standing on
/// the grant cell used to turn the wish into `No effect...`.
#[test]
fn playtest_p21_05_matching_wish_on_an_occupied_cell_still_poofs() {
    let mut grid = open_grid();
    // The Paws layout: well west of the party, grant cell east of it.
    grid[32 + 1] = 0xa1;
    let mut state = test_state(grid, 2, 1);
    state.area = Area::Town {
        scene: Scene::new(0x16).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::West;
    state.gold = 2;
    let occupant = state
        .allocate_active_object_slot(horse_purchase_active_object(3, 1, 0))
        .expect("seed occupant");
    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    assert_eq!(state.step_active_wishing_well('Y', ""), None);
    assert_eq!(
        state.step_active_wishing_well('H', "orse"),
        Some(MoveOutcome::Observed)
    );

    assert_eq!(state.gold, 1);
    assert!(state.message.ends_with(WISHING_WELL_GRANT_LINE));
    assert!(!state.message.contains(WISHING_WELL_NO_EFFECT_LINE));
    let horses = state
        .active_objects
        .iter()
        .enumerate()
        .filter(|(slot, object)| {
            *slot != occupant && !object.is_empty() && object.x == 3 && object.y == 1
        })
        .count();
    assert_eq!(horses, 1, "the grant creates its horse east of the caller");
}

/// Playtest P21-06. `town-mode.md §13`: the town Fire source test is "the
/// same four-neighbour probe the Fire and Yell commands use", the four cells
/// cardinally adjacent to the party (`commands.md §11`). A cannon standing
/// only diagonally next to the party is no source, so F-Fire answers `What?`
/// (`vehicles.md §8`).
#[test]
fn playtest_p21_06_town_fire_ignores_a_diagonal_cannon() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    // The tower-roof layout: cannons at (3,1) and (1,3), party at (2,2).
    grid[32 + 3] = TOWN_CANNON_TILE_FIRST + 1;
    grid[3 * 32 + 1] = TOWN_CANNON_TILE_FIRST + 1;
    let mut state = test_state(grid, 2, 2);

    assert_eq!(
        state.fire_command(None, &dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "What?");
    assert_eq!(state.turn, 0);

    let fire_source = TownFireSourceEntry {
        scene: Scene::new(17).unwrap(),
        floor: 0,
        x: 3,
        y: 1,
        direction: Direction::East,
        expected_tile: None,
    };
    assert!(!town_fire_source_is_adjacent(fire_source, 2, 2));
    assert!(town_fire_source_is_adjacent(fire_source, 3, 2));
    let _ = fs::remove_dir_all(dir);
}
