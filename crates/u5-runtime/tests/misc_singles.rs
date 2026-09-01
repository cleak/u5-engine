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

// ---------------------------------------------------------------------------
// Idle world tick — runtime observation, `cleak/u5-spec#179`.
// ---------------------------------------------------------------------------

/// Runtime observation, `cleak/u5-spec#179`: over a 28 s idle sample a
/// hostile creature standing in the tile adjacent to the party animated
/// its sprite continuously but never moved to another tile and never
/// attacked; combat began only once the player passed a turn. Over a
/// separate 160 s idle sample the date, food, gold, sun/moon indicator
/// and every status row were bit-identical. Actor movement is driven by
/// player turns, not by wall-clock time.
///
/// `input.md §2` agrees for the scheduled half: "NPC schedules and the
/// in-world clock do not advance from this idle tick."
#[test]
fn the_idle_visual_tick_animates_sprites_without_moving_actors() {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.active_objects.truncate(1);
    state.active_objects.push(ActiveObject {
        type_byte: 176,
        tile: 176,
        x: 101,
        y: 100,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    });

    let placed = (state.active_objects[1].x, state.active_objects[1].y);
    let turn_before = state.turn;
    let clock_before = state.clock;

    // Four full water cycles' worth of idle ticks, plus one so neither
    // shared counter lands back on zero: far more phase-zero
    // opportunities than the 28 s sample gave the observed creature.
    let ticks = 65u32;
    for _ in 0..ticks {
        state.advance_visual_tick();
    }

    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        placed,
        "an idle tick must not move an active object"
    );
    assert_eq!(state.turn, turn_before, "an idle tick spends no turn");
    assert_eq!(state.clock, clock_before, "an idle tick spends no minutes");
    assert_eq!(
        u32::from(state.animation.frame),
        ticks % u32::from(STATIC_TILE_ANIMATION_PERIOD_TICKS),
        "the `animation.md §6` counter must still be advancing"
    );
    assert_eq!(
        u32::from(state.water_scroll.phase),
        ticks % u32::from(WATER_SCROLL_PHASE_COUNT),
        "the water-scroll counter must still be advancing"
    );
}

/// A wind-driven ship is world movement too, so the idle presentation
/// tick must not drift it either. The per-turn animator still does; that
/// is the `weather.md §7` cadence the ship tests above pin.
#[test]
fn the_idle_visual_tick_does_not_drift_a_wind_driven_ship() {
    let mut state = world_state(vec![0x01; WORLD_CELLS], 100, 100);
    state.wind = WindState::North;
    state.active_objects.truncate(1);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 120,
        y: 120,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x20,
        aux1: 0,
        aux3: 0,
    });
    let placed = (state.active_objects[1].x, state.active_objects[1].y);

    for _ in 0..16 {
        state.animate_active_object_sprites_only();
    }
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        placed,
        "a perpendicular frame drifts every turn, but never on an idle tick"
    );

    // The movement-bearing pass is unchanged.
    state.animate_active_objects();
    assert_ne!(
        (state.active_objects[1].x, state.active_objects[1].y),
        placed,
        "the per-turn animator still drifts the ship"
    );
}

// ---------------------------------------------------------------------------
// Water-surface scroll — runtime observation, `cleak/u5-spec#179`.
// ---------------------------------------------------------------------------

/// A 512-tile atlas whose every tile paints its row index, so a vertical
/// rotation is visible as a permuted column and nothing else is.
fn row_ramp_atlas() -> TileAtlas {
    let mut pixels = vec![0u8; 512 * TILE_ATLAS_TILE_PIXELS];
    for tile in 0..512 {
        for row in 0..TILE_ATLAS_SIDE {
            for x in 0..TILE_ATLAS_SIDE {
                pixels[tile * TILE_ATLAS_TILE_PIXELS + row * TILE_ATLAS_SIDE + x] = row as u8;
            }
        }
    }
    TileAtlas {
        depth: TileGraphicsDepth::Ega16,
        pixels,
        dungeon_billboards: None,
        dungeon_sprites: None,
    }
}

/// `cleak/u5-spec#179`: stage one rotates the water and lava ids one pixel
/// row downward per world tick through sixteen phases, with every rotated
/// tile on screen in lockstep on a single global counter.
///
/// This is a display-layer treatment, so it must show up in the rendered
/// pixels while leaving the map byte and the resolved tile id alone —
/// `animation.md §6` and `RETRACTIONS.md` R148 keep water out of the five
/// tile-id families.
#[test]
fn rotated_water_moves_one_row_down_per_world_tick_in_lockstep() {
    let atlas = row_ramp_atlas();
    let radius = VIEWPORT_PLAYER_ROW;
    let authored: Vec<u8> = (0..TILE_ATLAS_SIDE as u8).collect();

    // Every rotated id, water and lava alike, takes the identical path.
    for rotated_tile in WATER_ROTATED_TILES {
        let mut state = world_state(vec![rotated_tile; WORLD_CELLS], 100, 100);

        let column_of = |viewport: &TileViewport, cell_x: usize, cell_y: usize| -> Vec<u8> {
            let x = cell_x * TILE_ATLAS_SIDE;
            (0..TILE_ATLAS_SIDE)
                .map(|row| viewport.pixels[(cell_y * TILE_ATLAS_SIDE + row) * viewport.width + x])
                .collect()
        };

        let first = state
            .render_top_down_viewport(radius, &atlas)
            .unwrap()
            .expect("a world viewport");
        assert_eq!(
            column_of(&first, radius, radius - 1),
            authored,
            "0x{rotated_tile:02x}: phase zero draws the authored tile"
        );

        for tick in 1..=WATER_SCROLL_PHASE_COUNT {
            state.advance_visual_tick();
            let viewport = state
                .render_top_down_viewport(radius, &atlas)
                .unwrap()
                .expect("a world viewport");
            let shift = usize::from(tick) % TILE_ATLAS_SIDE;
            let above = column_of(&viewport, radius, radius - 1);
            let far = column_of(&viewport, 0, 0);
            assert_eq!(
                above, far,
                "0x{rotated_tile:02x} tick {tick}: one global counter"
            );
            let expected: Vec<u8> = (0..TILE_ATLAS_SIDE)
                .map(|y| authored[(y + TILE_ATLAS_SIDE - shift) % TILE_ATLAS_SIDE])
                .collect();
            assert_eq!(
                above, expected,
                "0x{rotated_tile:02x} tick {tick}: rotated down {shift} row(s)"
            );
        }

        assert_eq!(state.water_scroll.phase, 0, "sixteen ticks close the cycle");
        assert_eq!(
            state.grid[world_cell_index(100, 99)],
            rotated_tile,
            "the map byte is never rewritten"
        );
        assert_eq!(
            static_tile_animation_family(rotated_tile),
            None,
            "0x{rotated_tile:02x} is still not a §6 tile-id family"
        );
    }
}

/// `cleak/u5-spec#179`: stage two rebuilds each composite destination as
/// `dest = (dest & !mask) | (rotated_shoals & mask)`, pairing destination
/// and mask index for index, with the third set seeing the complement.
///
/// The mask geometry is shipped-atlas data, so this test supplies its own
/// distinctive masks — one solid row per destination — and checks that the
/// render path routes the right mask to the right destination with the
/// right polarity. That the *shipped* masks reproduce the measured coast
/// and river shapes is verified against the captures, not here.
#[test]
fn composite_destinations_take_the_rotated_shoals_through_their_mask_tile() {
    let radius = VIEWPORT_PLAYER_ROW;
    let source_id = usize::from(WATER_COMPOSITE_SOURCE_TILE);

    // Source carries its row index; destinations are a flat value; each
    // mask tile is solid on exactly one row, a different row per id.
    let mut pixels = vec![0u8; 512 * TILE_ATLAS_TILE_PIXELS];
    for row in 0..TILE_ATLAS_SIDE {
        for x in 0..TILE_ATLAS_SIDE {
            pixels[source_id * TILE_ATLAS_TILE_PIXELS + row * TILE_ATLAS_SIDE + x] = row as u8;
        }
    }
    for set in WATER_COMPOSITE_SETS {
        for offset in 0..set.count {
            let dest = usize::from(set.first_dest + offset) * TILE_ATLAS_TILE_PIXELS;
            pixels[dest..dest + TILE_ATLAS_TILE_PIXELS].fill(0x0A);
            let mask = usize::from(set.first_mask + offset) * TILE_ATLAS_TILE_PIXELS;
            let solid_row = usize::from(offset) % TILE_ATLAS_SIDE;
            for x in 0..TILE_ATLAS_SIDE {
                pixels[mask + solid_row * TILE_ATLAS_SIDE + x] = 0x0F;
            }
        }
    }
    let atlas = TileAtlas {
        depth: TileGraphicsDepth::Ega16,
        pixels,
        dungeon_billboards: None,
        dungeon_sprites: None,
    };

    for set in WATER_COMPOSITE_SETS {
        for offset in 0..set.count {
            let dest_tile = set.first_dest + offset;
            let solid_row = usize::from(offset) % TILE_ATLAS_SIDE;

            let mut grid = vec![0x05u8; WORLD_CELLS];
            grid[world_cell_index(100, 99)] = dest_tile;
            let mut state = world_state(grid, 100, 100);

            for tick in 0..TILE_ATLAS_SIDE {
                let viewport = state
                    .render_top_down_viewport(radius, &atlas)
                    .unwrap()
                    .expect("a world viewport");
                let at = |x: usize, y: usize| {
                    viewport.pixels[((radius - 1) * TILE_ATLAS_SIDE + y) * viewport.width
                        + radius * TILE_ATLAS_SIDE
                        + x]
                };
                // The frame the rotated source is showing this tick.
                let shift = state.water_scroll.row_shift();
                let source_row = |y: usize| ((y + TILE_ATLAS_SIDE - shift) % TILE_ATLAS_SIDE) as u8;

                for y in 0..TILE_ATLAS_SIDE {
                    // Solid where the mask is set, unless this set is
                    // composited through the complement.
                    let takes_source = (y == solid_row) != set.mask_inverted;
                    let expected = if takes_source { source_row(y) } else { 0x0A };
                    assert_eq!(
                        at(0, y),
                        expected,
                        "0x{dest_tile:02x} tick {tick} row {y}: mask 0x{:02x}{}",
                        set.first_mask + offset,
                        if set.mask_inverted {
                            " (complement)"
                        } else {
                            ""
                        }
                    );
                }
                state.advance_visual_tick();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `visibility.md §8` variant lifetime// ---------------------------------------------------------------------------
// `visibility.md §8` variant lifetime — runtime observation,
// `cleak/u5-spec#179`.
// ---------------------------------------------------------------------------

/// `visibility.md §8`'s four-entry compositor variant must not be
/// re-drawn by the animation tick.
///
/// Runtime observation, `cleak/u5-spec#179`: outside combat, actor
/// sprites are static while the player is idle — the party sprite on the
/// overworld and the Avatar seated in a chair were both bit-identical
/// across 160 s idle windows with zero transitions. The engine used to
/// mix the animation phase into the selector, so a stationary actor on
/// one of the four-entry terrains re-stamped itself on every animation
/// tick; at the measured 18.2 Hz world tick that is a visible flicker.
#[test]
fn the_compositor_variant_is_stable_across_animation_ticks() {
    let mut grid = open_world_grid();
    grid[world_cell_index(101, 100)] = 0x84;
    let mut state = world_state(grid, 100, 100);
    state.active_objects.truncate(1);
    state.active_objects.push(ActiveObject {
        type_byte: 0x44,
        tile: 0x44,
        x: 101,
        y: 100,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    let area = state.top_down_render_area().expect("a world area");

    let stamped = |state: &PlayState| {
        state
            .top_down_render_cell(area, 100, 100, 101, 100, VIEWPORT_PLAYER_ROW)
            .expect("the adjacent cell is visible")
            .1
            .expect("the actor is composited over the terrain")
    };

    let first = stamped(&state);
    assert!(
        (0x60..=0x63).contains(&first),
        "`visibility.md §8`: terrain 0x84 selects one of 0x60..0x63, got 0x{first:02x}"
    );

    for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
        state.animation = AnimationClock::at_static_tile_phase(phase);
        assert_eq!(
            stamped(&state),
            first,
            "phase {phase} must not re-roll the variant"
        );
    }
}
