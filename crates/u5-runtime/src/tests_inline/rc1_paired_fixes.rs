// Regressions for the rc1 full paired sweep of 2026-10-17. Each test names
// the stock capture it reproduces and the published sentence it rests on.

fn blackthorn_audience_fixture_map() -> MiscmapsCutsceneMap {
    let mut tiles = vec![0x44; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS];
    // The audience grid's manacles under the two party actors' start cells.
    for (x, y) in [(3usize, 5usize), (7, 5), (0, 1)] {
        tiles[y * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS + x] = 0x85;
    }
    MiscmapsCutsceneMap {
        record_index: BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD,
        tiles,
    }
}

/// `bt-correct` (artifact bt-correct-20261017-004820): on every audience
/// beat the stock game draws the Avatar at `(3,5)` and the second member at
/// `(7,5)` as the manacled prisoner. `blackthorn.md §6.3` puts slot 0 at
/// `(3,5)` and §5 the second-member actor at `(7,5)`; both cells are the
/// manacles `0x85`, and `visibility.md §8` stamps "one of `0x64..0x67`" for a
/// terrain-aware actor there. The world's slot-zero refresh must not drag
/// the Avatar actor off the cutscene grid to the party's town position.
#[test]
fn rc1_blackthorn_audience_party_actors_draw_as_manacled_prisoners() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.blackthorn_audience_map = Some(blackthorn_audience_fixture_map());
    state.install_blackthorn_audience_actors();

    state.sync_player_object();
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (3, 5),
        "the cutscene owns slot zero while the audience runs"
    );

    let prisoner = 0x164..=0x167usize;
    for slot in [0usize, 1] {
        let tile = state.blackthorn_audience_actor_atlas_tile(slot).unwrap();
        assert!(prisoner.contains(&tile), "slot {slot} drew {tile:#x}");
    }
    // A guard on cobble is stamped unchanged.
    assert_eq!(
        state.blackthorn_audience_actor_atlas_tile(6),
        Some(0x100 + usize::from(BLACKTHORN_GUARD_ACTOR_BYTE))
    );
    // The suppressed tableau draws nothing.
    assert_eq!(state.blackthorn_audience_actor_atlas_tile(8), None);

    // `visibility.md §8.1`: one fresh draw per selecting actor per pass,
    // and the frame follows the drawn variant.
    state.blackthorn_audience_actor_variants = [0; 9];
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..64 {
        state.run_blackthorn_audience_composite_pass();
        seen.insert(state.blackthorn_audience_actor_atlas_tile(0).unwrap());
    }
    assert!(seen.len() > 1, "the prisoner re-rolls its frame: {seen:?}");
    assert!(seen.iter().all(|tile| prisoner.contains(tile)));
}

/// Outside the audience the slot-zero refresh still runs.
#[test]
fn rc1_slot_zero_refresh_still_runs_outside_the_audience() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.install_blackthorn_audience_actors();
    state.sync_player_object();
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (12, 20)
    );
}
