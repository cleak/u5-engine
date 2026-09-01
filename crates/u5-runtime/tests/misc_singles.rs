//! Focused pins for the `misc-singles` published-spec gap batch.
//!
//! Each test cites the spec section it pins.

use u5_runtime::test_fixtures::{open_world_grid, world_state};
use u5_runtime::*;

// ---------------------------------------------------------------------------
// `systems/time.md §7` — Shadowlord slot value 0 is the new-game start value.
// ---------------------------------------------------------------------------

/// `systems/time.md §7`: "A newly created game starts with all three
/// slots at `0`, so no Shadowlord is anywhere until the first midnight
/// pass assigns hideouts." `0` is "neither 'in a town' nor
/// 'vanquished'".
#[test]
fn new_game_shadowlord_slots_start_unplaced() {
    assert_eq!(DEFAULT_SHADOWLORD_HIDEOUTS, [0, 0, 0]);
    assert_eq!(PlayOptions::default().shadowlord_hideouts, [0, 0, 0]);

    for slot in DEFAULT_SHADOWLORD_HIDEOUTS {
        // Not "in a town": no living hideout id.
        assert!(!PlayState::shadowlord_slot_is_living(slot));
        // Not "vanquished".
        assert!(!PlayState::shadowlord_slot_is_vanquished(slot));
        // "the reroll walker rewrites it on the first day rollover".
        assert!(PlayState::shadowlord_slot_is_rerollable(slot));
    }
}

/// A default-options state has no resident Shadowlord anywhere until
/// the midnight walker runs, and the walker then places all three.
#[test]
fn default_state_places_no_shadowlord_until_the_first_reroll() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.shadowlord_hideouts = DEFAULT_SHADOWLORD_HIDEOUTS;
    for index in 0..SHADOWLORD_COUNT {
        assert!(!state.shadowlord_alive(index));
        assert!(!state.shadowlord_vanquished(index));
    }

    let rerolled = state.reroll_shadowlord_hideouts();
    assert_eq!(rerolled, SHADOWLORD_COUNT);
    for index in 0..SHADOWLORD_COUNT {
        assert!(state.shadowlord_alive(index));
    }
}

// ---------------------------------------------------------------------------
// `catalogs/quest-graph.md §5` — fixed Underworld placement.
// ---------------------------------------------------------------------------

fn placed_at(state: &PlayState, x: usize, y: usize) -> Option<ActiveObject> {
    state
        .active_objects
        .iter()
        .skip(1)
        .copied()
        .find(|object| !object.is_empty() && object.x == x && object.y == y)
}

/// `catalogs/quest-graph.md §5`, "Where the shards are: fixed
/// Underworld placement": four records on the Underworld plane at
/// (105,225), (192,80), (130,65) and (176,184).
#[test]
fn underworld_setup_pass_places_amulet_and_three_shards() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    state.shadowlord_hideouts = DEFAULT_SHADOWLORD_HIDEOUTS;
    state.active_objects.truncate(1);

    state.place_underworld_fixed_objects(WorldPlane::Underworld);

    let amulet = placed_at(&state, 105, 225).expect("Amulet of Lord British placed at (105,225)");
    assert_eq!(amulet.type_byte, INVENTORY_ADD_CLASS_AMULET_LORD_BRITISH);
    assert_eq!(amulet.z, WorldPlane::Underworld.save_floor());

    for (x, y, shard_index) in [
        (192usize, 80usize, SHADOWLORD_FALSEHOOD_INDEX),
        (130, 65, SHADOWLORD_HATRED_INDEX),
        (176, 184, SHADOWLORD_COWARDICE_INDEX),
    ] {
        let shard = placed_at(&state, x, y).expect("shard placed at its published cell");
        assert_eq!(shard.type_byte, INVENTORY_ADD_CLASS_SHADOWLORD_SHARD);
        assert_eq!(shard.aux1, shard_index as u8);
        assert_eq!(shard.z, WorldPlane::Underworld.save_floor());
    }
}

/// §5: "The pass is a placement pass, not a respawn: once the carried
/// flag is set the object is never emitted again."
#[test]
fn underworld_setup_pass_skips_carried_objects() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    state.special_items[SPECIAL_ITEM_AMULET_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.special_items[SPECIAL_ITEM_SHARD_HATRED_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.active_objects.truncate(1);

    state.place_underworld_fixed_objects(WorldPlane::Underworld);

    assert!(placed_at(&state, 105, 225).is_none());
    assert!(placed_at(&state, 130, 65).is_none());
    assert!(placed_at(&state, 192, 80).is_some());
    assert!(placed_at(&state, 176, 184).is_some());
}

/// §5: "the carried-flag test alone would happily re-place the shard in
/// the Underworld on the party's next visit and hand the player an
/// infinite supply. The alive test is what suppresses that." Destruction
/// clears the carried flag *and* vanquishes the slot, so after a
/// destruction the shard must not come back.
#[test]
fn underworld_setup_pass_does_not_respawn_a_spent_shard() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    // Destruction consumed the Shard of Cowardice: carried flag clear,
    // Nosfentor's slot vanquished.
    state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX] = SHADOWLORD_VANQUISHED;
    state.active_objects.truncate(1);

    state.place_underworld_fixed_objects(WorldPlane::Underworld);

    assert!(placed_at(&state, 176, 184).is_none());
    assert!(placed_at(&state, 192, 80).is_some());
    assert!(placed_at(&state, 130, 65).is_some());
}

/// A fresh game holds `0` in every Shadowlord slot (`time.md §7`), which
/// is "neither 'in a town' nor 'vanquished'", so all three shards are
/// still placed before the first midnight pass.
#[test]
fn underworld_setup_pass_places_shards_for_unplaced_shadowlord_slots() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    state.shadowlord_hideouts = [0; SHADOWLORD_COUNT];
    state.active_objects.truncate(1);

    state.place_underworld_fixed_objects(WorldPlane::Underworld);

    assert!(placed_at(&state, 192, 80).is_some());
    assert!(placed_at(&state, 130, 65).is_some());
    assert!(placed_at(&state, 176, 184).is_some());
}

/// The pass runs only on the Underworld plane; every record it writes
/// is on "floor byte `255`".
#[test]
fn underworld_setup_pass_is_underworld_only() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.special_items = [0; SPECIAL_ITEM_COUNT];
    state.active_objects.truncate(1);

    state.place_underworld_fixed_objects(WorldPlane::Britannia);

    assert_eq!(state.active_objects.len(), 1);
}

// ---------------------------------------------------------------------------
// `systems/weather.md §7` — active-ship wind cadence.
// ---------------------------------------------------------------------------

fn ship_drift_moves_over(wind: WindState, frame_phase: u8, turns: usize) -> usize {
    let mut state = world_state(vec![0x01; WORLD_CELLS], 4, 5);
    state.wind = wind;
    state.active_objects.truncate(1);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 100,
        y: 100,
        z: WorldPlane::Underworld.save_floor(),
        phase: frame_phase,
        aux1: 0,
        aux3: 0,
    });

    let mut moves = 0;
    for _ in 0..turns {
        let before = (state.active_objects[1].x, state.active_objects[1].y);
        state.animate_active_objects();
        let after = (state.active_objects[1].x, state.active_objects[1].y);
        if before != after {
            moves += 1;
        }
    }
    moves
}

/// `systems/weather.md §7` cadence table. A frame facing the wind source
/// moves "2 of 3 turns"; a frame facing away moves "3 of 4 turns"; a
/// perpendicular frame moves "every turn" and "bypasses the counter".
#[test]
fn active_ship_wind_cadence_counts_match_the_weather_table() {
    // North-facing frame, north wind: 2 of 3.
    assert_eq!(ship_drift_moves_over(WindState::North, 0x00, 3), 2);
    assert_eq!(ship_drift_moves_over(WindState::North, 0x00, 6), 4);

    // South-facing frame, north wind: 3 of 4.
    assert_eq!(ship_drift_moves_over(WindState::North, 0x40, 4), 3);
    assert_eq!(ship_drift_moves_over(WindState::North, 0x40, 8), 6);

    // East-facing frame, north wind: perpendicular, every turn. This is
    // the row the previous heading-versus-wind test stalled forever.
    assert_eq!(ship_drift_moves_over(WindState::North, 0x20, 6), 6);

    // "Calm wind suppresses this movement."
    assert_eq!(ship_drift_moves_over(WindState::Calm, 0x20, 6), 0);
}

/// §7: "The cadence counter is stored per active-object slot ... and is
/// persisted with the object, so it survives save and reload." It lives
/// in the slot's own bytes, not in shared engine state.
#[test]
fn active_ship_cadence_counter_lives_in_the_object_slot() {
    let mut state = world_state(vec![0x01; WORLD_CELLS], 4, 5);
    state.wind = WindState::North;
    state.active_objects.truncate(1);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 100,
        y: 100,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x00,
        aux1: 0,
        aux3: 0,
    });

    let counter = |state: &PlayState| {
        (state.active_objects[1].phase & ACTIVE_SHIP_CADENCE_PHASE_MASK)
            >> ACTIVE_SHIP_CADENCE_PHASE_SHIFT
    };

    state.animate_active_objects();
    assert_eq!(
        state.active_objects[1].phase & 0xf0,
        0x00,
        "frame heading is preserved"
    );
    assert_eq!(
        state.active_objects[1].phase & 0x03,
        0x00,
        "the drawn frame selector is untouched"
    );
    assert_eq!(counter(&state), 1, "one move of the 2-of-3 cycle spent");

    state.animate_active_objects();
    assert_eq!(counter(&state), 2);

    // Third pass is the skip; the counter resets.
    state.animate_active_objects();
    assert_eq!(counter(&state), 0);
}
