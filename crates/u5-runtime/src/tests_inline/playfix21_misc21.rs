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
