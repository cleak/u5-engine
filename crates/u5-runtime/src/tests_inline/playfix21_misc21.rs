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

/// Playtest P21-06. `vehicles.md §8` (issue #313 Q3): the probe examines
/// "exactly the four cells cardinally adjacent to the party ... Diagonal
/// cells and the party's own cell are never examined". A cannon standing
/// only diagonally next to the party is no source, so F-Fire answers `What?`.
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

/// The transcript plus the unflushed slot, one string per row, as the shell
/// composes the message window.
/// Issue #313 Q3. `vehicles.md §8`: the probe examines the four cardinal
/// cells "in the order **north, east, south, west**", "This is not the Yell
/// command's order", and "The first qualifying cell in that order is the
/// source". The shot "runs in the cannon's facing, whichever side of the
/// party the cannon stands on". The probe used Yell's west, south, east,
/// north order (playtest P21-06).
#[test]
fn issue_313_town_fire_probe_order_is_north_east_south_west() {
    let scene = Scene::new(17).unwrap();
    // Cannons on all four sides of the party at (5,5), each facing a
    // different way: north cell faces west, east cell faces south, south
    // cell faces east, west cell faces north.
    let mut grid = open_grid();
    grid[4 * 32 + 5] = TOWN_CANNON_TILE_FIRST + 3;
    grid[5 * 32 + 6] = TOWN_CANNON_TILE_FIRST + 2;
    grid[6 * 32 + 5] = TOWN_CANNON_TILE_FIRST + 1;
    grid[5 * 32 + 4] = TOWN_CANNON_TILE_FIRST;
    let mut state = test_state(grid, 5, 5);

    let mut expected = vec![
        ((5, 4), Direction::West),
        ((6, 5), Direction::South),
        ((5, 6), Direction::East),
        ((4, 5), Direction::North),
    ];
    for ((x, y), direction) in expected.drain(..) {
        let source = state
            .adjacent_town_cannon_fire_source(scene, 0)
            .expect("a cardinal cannon");
        assert_eq!((source.x, source.y), (x, y));
        assert_eq!(source.direction, direction, "the cannon's own facing");
        // Clear this source so the next one in the order is found.
        state.grid[y * 32 + x] = open_grid()[y * 32 + x];
    }
    assert!(state.adjacent_town_cannon_fire_source(scene, 0).is_none());
}

/// Issue #313 Q3. `vehicles.md §8`: the shot "steps along the line before
/// each test, so the cannon's own cell is never tested and at most four
/// cells beyond it are". A door four cells beyond the cannon is reached; one
/// five cells beyond is not.
#[test]
fn issue_313_town_fire_shot_reaches_four_cells_beyond_the_cannon() {
    let dir = debug_game_dir();
    let mut grid = open_grid();
    // Party at (0,1), an east-facing cannon at (1,1), a door at (5,1).
    grid[32 + 1] = TOWN_CANNON_TILE_FIRST + 1;
    grid[32 + 5] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid.clone(), 0, 1);
    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);
    assert_eq!(state.grid[32 + 5], TOWN_DOOR_CLEARED_TILE);

    grid[32 + 5] = open_grid()[32 + 5];
    grid[32 + 6] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    let mut state = test_state(grid, 0, 1);
    assert_eq!(state.fire_command(None, &dir).unwrap(), MoveOutcome::Fired);
    assert_eq!(state.grid[32 + 6], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
    assert_eq!(state.message, crate::commands::CANNON_BOOOM_LINE);
    let _ = fs::remove_dir_all(dir);
}

fn p21_window_history(state: &PlayState) -> Vec<String> {
    let mut rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    if state.message_slot_needs_flush() {
        rows.extend(state.message.split('\n').map(str::to_string));
    }
    rows
}

fn p21_misc_row_index(rows: &[String], text: &str) -> usize {
    rows.iter()
        .rposition(|row| row.contains(text))
        .unwrap_or_else(|| panic!("{text:?} missing from {rows:?}"))
}

/// Playtest P21-04. `endgame.md §3.1`/`§5.1`: the rite pages print into the
/// ordinary message window, which scrolls; `He says:` follows record 3 with
/// one blank row before it. The forty-tick pause overwrote record 3 in the
/// unflushed slot with the `He says:` page, and the next page key then lost
/// that page too, so the window jumped back to the box question.
#[test]
fn playtest_p21_04_rite_pages_scroll_instead_of_replacing_each_other() {
    let dir = debug_game_dir();
    let mut state = test_state(endgame_tableau_test_grid(), 5, 5);
    state.party_names = vec![*b"Avatar\0\0\0"];
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.enter_endgame_with_messages(Some(endgame_window_synthetic_messages()));
    state.finish_endgame_entry_presentation();
    state.flush_staged_narration();
    state.resolve_endgame_confirmation(true);
    let mut frames = 0;
    while state.endgame.as_ref().unwrap().cinematic.step
        == crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(0)
    {
        state.flush_staged_narration();
        frames += 1;
        assert!(frames < 500);
        assert!(state.advance_endgame_display_frame());
    }
    state.flush_staged_narration();
    // State B: record 3 scrolled up above the lead-in.
    let rows = p21_window_history(&state);
    let rite1 = p21_misc_row_index(&rows, "Rite 1");
    let he_says = p21_misc_row_index(&rows, "He says:");
    let rite2 = p21_misc_row_index(&rows, "Rite 2");
    assert!(rite1 < he_says && he_says < rite2, "{rows:?}");
    assert_eq!(&rows[rite1 + 1..he_says], [""], "one blank row before `He says:`");
    assert_eq!(&rows[he_says + 1..rite2], [""], "one blank row after it");

    // State C: the page key keeps the `He says:` page above record 5.
    handle_play_key_input(&mut state, 'x', "", &dir).unwrap();
    state.flush_staged_narration();
    while state.advance_endgame_display_frame() {
        state.flush_staged_narration();
    }
    let rows = p21_window_history(&state);
    let rite1 = p21_misc_row_index(&rows, "Rite 1");
    let he_says = p21_misc_row_index(&rows, "He says:");
    let rite2 = p21_misc_row_index(&rows, "Rite 2");
    let rite3 = p21_misc_row_index(&rows, "Rite 3");
    assert!(rite1 < he_says && he_says < rite2 && rite2 < rite3, "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}
