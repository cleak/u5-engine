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

// ---------------------------------------------------------------------------
// `systems/animation.md §12.4` — the fire fixtures' cumulative masked-noise
// XOR, on the render path.
// ---------------------------------------------------------------------------

/// Build a 512-entry atlas whose fire fixture and mask tiles carry
/// distinctive artwork. `mask_rows` is the flame silhouette: for each row,
/// how many pixels from column zero the mask covers.
fn fire_test_atlas(
    fixture: u8,
    mask_id: u8,
    mask_colour: u8,
    mask_rows: &[(usize, usize)],
) -> TileAtlas {
    let mut pixels = vec![0u8; 512 * TILE_ATLAS_TILE_PIXELS];
    let base = usize::from(fixture) * TILE_ATLAS_TILE_PIXELS;
    for index in 0..TILE_ATLAS_TILE_PIXELS {
        pixels[base + index] = (index % 16) as u8;
    }
    let mask = usize::from(mask_id) * TILE_ATLAS_TILE_PIXELS;
    for (row, width) in mask_rows {
        for x in 0..*width {
            pixels[mask + row * TILE_ATLAS_SIDE + x] = mask_colour;
        }
    }
    TileAtlas {
        depth: TileGraphicsDepth::Ega16,
        pixels,
        dungeon_billboards: None,
        dungeon_sprites: None,
    }
}

fn fire_cell_pixels(state: &mut PlayState, atlas: &TileAtlas, radius: usize) -> Vec<u8> {
    let viewport = state
        .render_top_down_viewport(radius, atlas)
        .unwrap()
        .expect("a world viewport");
    (0..TILE_ATLAS_TILE_PIXELS)
        .map(|index| {
            let y = index / TILE_ATLAS_SIDE;
            let x = index % TILE_ATLAS_SIDE;
            viewport.pixels[((radius - 1) * TILE_ATLAS_SIDE + y) * viewport.width
                + radius * TILE_ATLAS_SIDE
                + x]
        })
        .collect()
}

/// `animation.md §12.4`: "for each fire fixture, over the whole 16x16 tile:
/// `fixture ^= (noise AND mask)`", where "each mask is a small shape sitting
/// exactly over its fixture's flame, so only pixels inside the flame
/// silhouette are ever touched" and the XOR is "cumulative and ... never
/// undone".
///
/// The three properties, on the render path: the drawn tile changes between
/// successive world ticks, about half the mask's pixels change per tick, and
/// nothing outside the mask ever moves. Capture measured "about 12.8 of
/// those 26 pixels change per update" over the brazier's 26-pixel region.
#[test]
fn a_fire_fixture_flickers_inside_its_mask_and_nowhere_else() {
    let radius = VIEWPORT_PLAYER_ROW;
    // The measured brazier region: rows 2 through 6, four to six pixels a
    // row, 26 pixels in all.
    let mask_rows = [(2usize, 4usize), (3, 5), (4, 6), (5, 6), (6, 5)];
    let fixture = fire_fixture_spec(0xB2).expect("the brazier is a published fixture");
    assert_eq!(fixture.mask, 0xC2);
    assert_eq!(fixture.noise, FIRE_NOISE_TILE);

    let atlas = fire_test_atlas(fixture.tile, fixture.mask, FIRE_NOISE_PLANES, &mask_rows);
    let authored: Vec<u8> = atlas
        .tile_pixels(usize::from(fixture.tile))
        .expect("fixture artwork")
        .to_vec();

    let mut grid = vec![0x05u8; WORLD_CELLS];
    grid[world_cell_index(100, 99)] = fixture.tile;
    let mut state = world_state(grid, 100, 100);

    let masked: Vec<usize> = mask_rows
        .iter()
        .flat_map(|(row, width)| (0..*width).map(move |x| row * TILE_ATLAS_SIDE + x))
        .collect();
    assert_eq!(masked.len(), 26, "the measured brazier region");

    // Before any world step the shipped artwork is drawn unaltered.
    assert_eq!(
        fire_cell_pixels(&mut state, &atlas, radius),
        authored,
        "a fresh clock draws the shipped art"
    );

    let ticks = 200usize;
    let mut previous = fire_cell_pixels(&mut state, &atlas, radius);
    let mut changed_ticks = 0usize;
    let mut total_changed = 0usize;
    for tick in 0..ticks {
        state.advance_visual_tick();
        let now = fire_cell_pixels(&mut state, &atlas, radius);

        for index in 0..TILE_ATLAS_TILE_PIXELS {
            if !masked.contains(&index) {
                assert_eq!(
                    now[index], authored[index],
                    "tick {tick} pixel {index} is outside the mask"
                );
            }
            // Only the planes noise tile `0x1EA` occupies may ever move.
            assert_eq!(
                (now[index] ^ authored[index]) & !FIRE_NOISE_PLANES,
                0,
                "tick {tick} pixel {index}: 0x1EA supplies only planes 0b1100"
            );
        }

        let changed = masked.iter().filter(|i| now[**i] != previous[**i]).count();
        total_changed += changed;
        if changed > 0 {
            changed_ticks += 1;
        }
        previous = now;
    }

    assert_eq!(
        changed_ticks, ticks,
        "every world tick must change the fixture"
    );
    let mean = total_changed as f64 / ticks as f64;
    assert!(
        (mean - 13.0).abs() < 2.0,
        "about half of 26 masked pixels should change per tick, saw {mean:.2}"
    );
    assert_eq!(
        state.grid[world_cell_index(100, 99)],
        fixture.tile,
        "the map byte is never rewritten"
    );
    assert_eq!(
        static_tile_animation_family(fixture.tile),
        None,
        "the fixture is not a §6 tile-id family"
    );
}

/// `animation.md §12.4`: the fire stage touches the published fixtures and
/// nothing else. A neighbouring terrain id — and a mask id itself — must
/// draw exactly its shipped artwork at every tick.
#[test]
fn non_fire_tiles_are_untouched_by_the_fire_stage() {
    let radius = VIEWPORT_PLAYER_ROW;
    let mask_rows = [(2usize, 4usize), (3, 5), (4, 6)];
    let base_atlas = fire_test_atlas(0xB2, 0xC2, FIRE_NOISE_PLANES, &mask_rows);

    // `0xB4` sits just past the torch/brazier/spit run, `0xC2` is a mask
    // rather than a fixture, `0xDD` neighbours the shrine flame and `0x44`
    // is ordinary floor.
    for tile in [0xB4u8, 0xC2, 0xDD, 0x44] {
        assert!(!fire_pass_animates_tile(tile), "0x{tile:02x}");
        let mut pixels = base_atlas.pixels.clone();
        let base = usize::from(tile) * TILE_ATLAS_TILE_PIXELS;
        for index in 0..TILE_ATLAS_TILE_PIXELS {
            pixels[base + index] = ((index * 7) % 16) as u8;
        }
        let atlas = TileAtlas {
            depth: TileGraphicsDepth::Ega16,
            pixels,
            dungeon_billboards: None,
            dungeon_sprites: None,
        };
        let authored: Vec<u8> = atlas
            .tile_pixels(usize::from(tile))
            .expect("artwork")
            .to_vec();

        let mut grid = vec![0x05u8; WORLD_CELLS];
        grid[world_cell_index(100, 99)] = tile;
        let mut state = world_state(grid, 100, 100);

        for tick in 0..40 {
            state.advance_visual_tick();
            assert_eq!(
                fire_cell_pixels(&mut state, &atlas, radius),
                authored,
                "0x{tile:02x} tick {tick} must not flicker"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// `systems/timing.md §8.2` — the idle pass's world step, and where it stops.
// ---------------------------------------------------------------------------

/// `timing.md §8.2`: "The shared wait tests the current scene value and
/// performs no world step for values `0x21` through `0x7F` **inclusive**;
/// both the bound and its inclusiveness are exact." "Implement the gate as a
/// numeric range test on the scene value, **not** as an 'is this dungeon
/// mode' test: the band is a strict superset of the dungeon scenes, and the
/// intro, character-creation and Return-to-View animation states (`0x40`,
/// `0x41`, `0x42`) also lie inside it." "Combat sets scene value `0xFF` and
/// does run the world step."
#[test]
fn the_idle_world_step_is_suppressed_for_the_published_scene_band() {
    assert_eq!(IDLE_WORLD_STEP_SUPPRESSED_FIRST, 0x21);
    assert_eq!(IDLE_WORLD_STEP_SUPPRESSED_LAST, 0x7F);
    for scene in 0x00u8..=0xFF {
        assert_eq!(
            idle_world_step_suppressed_for_scene(scene),
            (0x21..=0x7F).contains(&scene),
            "scene 0x{scene:02x}"
        );
    }
    for scene in [0x21u8, 0x28, 0x40, 0x41, 0x42, 0x7F] {
        assert!(idle_world_step_suppressed_for_scene(scene));
    }
    for scene in [SCENE_OVERWORLD, 0x01, 0x20, 0x80, SCENE_COMBAT_TEMPORARY] {
        assert!(!idle_world_step_suppressed_for_scene(scene));
    }
}

/// `timing.md §8.2`: "First-person dungeon scenes occupy `0x21..0x28` and
/// therefore get no idle world step". No world step means no tile animation
/// of any kind underground — neither the `§6` frame-selector pass nor the
/// `§12` driver pass.
#[test]
fn a_dungeon_idle_tick_runs_no_world_step_at_all() {
    let mut state = u5_runtime::test_fixtures::dungeon_state(
        u5_runtime::test_fixtures::open_dungeon_record(),
        0,
        1,
        1,
    );
    assert!(idle_world_step_suppressed_for_scene(
        state.current_scene_byte()
    ));

    let animation = state.animation;
    let water = state.water_scroll;
    let fire = state.fire_flicker.clone();
    for _ in 0..40 {
        state.advance_visual_tick();
    }
    assert_eq!(state.animation, animation, "no §6 frame-selector pass");
    assert_eq!(state.water_scroll, water, "no §12.2/§12.3 water pass");
    assert_eq!(state.fire_flicker, fire, "no §12.4 fire pass");
    assert_eq!(state.fire_flicker.steps(), 0);

    // The same tick on the overworld does step the world, so the gate is
    // the scene value and not a blanket freeze.
    let mut surface = world_state(open_world_grid(), 100, 100);
    surface.advance_visual_tick();
    assert_eq!(surface.water_scroll.phase, 1);
    assert_eq!(surface.fire_flicker.steps(), 1);
}

/// `timing.md §8.2`: "On the overworld the input helper performs one
/// scripted step-and-wait — one world step followed by one one-tick wait —
/// before either entering the command wait or, when sails are set,
/// performing a bare cursor poll instead; so an **under-sail auto-advance
/// pass costs two ticks and one world step and never enters the command wait
/// at all**."
///
/// So under sail the world advances once per two idle passes. Off sail every
/// pass is one tick and one world step.
#[test]
fn an_under_sail_idle_pass_costs_two_ticks_and_one_world_step() {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 77,
        skiffs: 2,
    };
    assert!(state.player.transport.is_ship_under_sail());

    for pass in 1..=32u32 {
        state.advance_visual_tick();
        assert_eq!(
            state.fire_flicker.steps(),
            pass.div_ceil(2),
            "pass {pass}: one world step per two ticks under sail"
        );
        assert_eq!(
            u32::from(state.water_scroll.phase),
            pass.div_ceil(2) % u32::from(WATER_SCROLL_PHASE_COUNT)
        );
    }

    // Furling the sails restores one world step per tick immediately.
    state.player.transport = TransportState::Foot;
    let before = state.fire_flicker.steps();
    for pass in 1..=8u32 {
        state.advance_visual_tick();
        assert_eq!(
            state.fire_flicker.steps(),
            before + pass,
            "pass {pass} on foot"
        );
    }
}
