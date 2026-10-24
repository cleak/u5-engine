// Regressions for the pass-8 exploratory-playtest findings (p8play, N02-N26).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `commands.md §3`: "`B` Board | \"Acted\" on every path - all four mounts
/// and both refusals - except the final unknown-target fallthrough". The
/// dungeon arm prints `Not here!` (`vehicles.md §4.1`), not the fallthrough,
/// so it spends the turn that drives the dungeon post-action pass (N06).
#[test]
fn playtest_n06_dungeon_board_refusal_reports_acted() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    let clock_before = state.clock;

    handle_play_key_input(&mut state, 'B', "", &dir).unwrap();

    assert_eq!(state.turn, 1, "dungeon Board refusal must report acted");
    let mut expected = clock_before;
    expected.advance_minutes(1);
    assert_eq!(state.clock, expected, "only the loop-head minute");
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §3`: H is not one of the six forwarding routes and neither of
/// its overworld refusals is a published "no action" producer, so `Sails
/// must be lowered!` and `On land or ship!` each run the overworld per-turn
/// epilogue (N11). The town `Only in bed!` refusal is the published
/// exception (`rest-and-camp.md §3`) and is not touched here.
#[test]
fn playtest_n11_overworld_hole_up_refusals_report_acted() {
    let dir = debug_game_dir();
    let mut ship = britannia_state(open_world_grid(), 45, 62);
    ship.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 77,
        skiffs: 0,
    };
    handle_play_key_input(&mut ship, 'H', "", &dir).unwrap();
    assert_eq!(ship.turn, 1, "Sails must be lowered! must report acted");

    let mut skiff = britannia_state(open_world_grid(), 45, 62);
    skiff.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    handle_play_key_input(&mut skiff, 'H', "", &dir).unwrap();
    assert_eq!(skiff.turn, 1, "On land or ship! must report acted");
    assert!(skiff.active_rest.is_none());
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §3`: I does not forward its handler's value, and
/// `lighting.md §8` only says the `None owned!` refusal leaves the light
/// state unchanged, so the refusal still costs the turn in town and on the
/// overworld (N12).
#[test]
fn playtest_n12_ignite_without_torches_reports_acted() {
    let dir = debug_game_dir();
    let mut world = britannia_state(open_world_grid(), 45, 62);
    world.torches = 0;
    handle_play_key_input(&mut world, 'I', "", &dir).unwrap();
    assert_eq!(world.turn, 1, "overworld None owned! must report acted");
    assert_eq!(world.torch_counter, 0);

    let mut town = test_state(open_grid(), 5, 5);
    town.torches = 0;
    handle_play_key_input(&mut town, 'I', "", &dir).unwrap();
    assert_eq!(town.turn, 1, "town None owned! must report acted");
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §5.5`: the H family's refusal literal is
/// `On_land_or_ship!\n\n`; it completes the `Hole up- ` echo row and its
/// second feed is a producer blank row of its own, ahead of the turn's
/// derived blank - the same shape `lowered!\n\n` already renders (N13).
#[test]
fn playtest_n13_on_land_or_ship_refusal_keeps_its_producer_blank_row() {
    let dir = debug_game_dir();
    let mut skiff = britannia_state(open_world_grid(), 45, 62);
    skiff.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    handle_play_key_input(&mut skiff, 'H', "", &dir).unwrap();

    let entries = skiff.message_entries();
    let at = entries
        .iter()
        .position(|entry| entry.text.ends_with(HOLE_UP_ON_LAND_OR_SHIP_REFUSAL))
        .unwrap_or_else(|| panic!("no refusal row in {entries:?}"));
    assert!(
        entries[at].text.starts_with("Hole up-"),
        "the refusal completes the echo row: {:?}",
        entries[at].text
    );
    let next = entries
        .get(at + 1)
        .unwrap_or_else(|| panic!("no row after the refusal in {entries:?}"));
    assert!(next.explicit_blank && next.producer_blank, "{entries:?}");
    let _ = fs::remove_dir_all(dir);
}
