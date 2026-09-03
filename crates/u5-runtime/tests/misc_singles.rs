//! Focused pins for the `misc-singles` published-spec gap batch.
//!
//! Each test cites the spec section it pins.

use u5_runtime::test_fixtures::{open_grid, open_world_grid, test_state, world_state};
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
        // `active-objects.md §8` (R316): the wind cadence for ship-like frames
        // is the *outdoor per-turn walker's*, not the animator's. The animator
        // "never writes a slot's column or row".
        state.advance_outdoor_active_objects();
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

    state.advance_outdoor_active_objects();
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

    state.advance_outdoor_active_objects();
    assert_eq!(counter(&state), 2);

    // Third pass is the skip; the counter resets.
    state.advance_outdoor_active_objects();
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
/// tick must not drift it either — and neither may the animator, on any
/// path. `active-objects.md §8` (R316): the per-slot animator "cannot move
/// anything ... it never writes a slot's column or row". The drift belongs
/// to the outdoor per-turn walker, which is the `weather.md §7` cadence the
/// ship tests above pin.
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
        state.animate_active_objects();
    }
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        placed,
        "a perpendicular frame drifts every turn, but never from the animator"
    );

    // The movement-bearing pass is the outdoor per-turn walker.
    state.advance_outdoor_active_objects();
    assert_ne!(
        (state.active_objects[1].x, state.active_objects[1].y),
        placed,
        "the outdoor per-turn walker still drifts the ship"
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
// `visibility.md §8.1` variant lifetime — the composite pass re-rolls.
// ---------------------------------------------------------------------------

/// One ordinary humanoid NPC sprite byte inside the merging band: at or above
/// `0x40` so `visibility.md §8`'s terrain-aware entry test accepts it, and
/// below `0x80` so the table does not route it to the plain stamp before the
/// terrain rows are consulted.
const SEATED_NPC_SPRITE: u8 = 0x60;

/// Build a town floor with one chair and the neighbouring-row furniture the
/// `§8` predicate reads: the `0x92` chair reads the row *below* it.
fn seated_town_grid(chair: u8, neighbour: u8, chair_x: usize, chair_y: usize) -> Vec<u8> {
    let mut grid = open_grid();
    grid[chair_y * 32 + chair_x] = chair;
    grid[(chair_y + 1) * 32 + chair_x] = neighbour;
    grid
}

/// A town state with an NPC seated on `(10, 20)` and the party three cells
/// south of it, so the seat and its neighbouring row are both well inside the
/// eleven-by-eleven viewport.
fn seated_npc_state(chair: u8, neighbour: u8) -> PlayState {
    let mut state = test_state(seated_town_grid(chair, neighbour, 10, 20), 10, 23);
    state.active_objects.push(ActiveObject {
        type_byte: SEATED_NPC_SPRITE,
        tile: SEATED_NPC_SPRITE,
        x: 10,
        y: 20,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.visibility_buffers_ready = false;
    state
}

/// Run one composite pass over the live buffers and report the byte left in
/// the companion terrain band under the seat, together with the number of
/// `0..3` draws the pass took off the shared gameplay stream.
fn idle_composite_pass(state: &mut PlayState) -> (u8, u32) {
    let before = state.prng_state;
    state.refresh_top_down_visibility_buffers(TopDownRenderArea::Town, VIEWPORT_PLAYER_ROW);
    let mut probe = before;
    let mut draws = 0u32;
    while probe != state.prng_state {
        let _ = u5_prng_range_u16(&mut probe, 0, 3);
        draws += 1;
        assert!(draws < 64, "a composite pass drew far more than one value");
    }
    let row = 20 + VIEWPORT_PLAYER_ROW - 23;
    let col = 10 + VIEWPORT_PLAYER_COL - 10;
    let stamped = state.terrain_band[terrain_band_active_index(row, col).unwrap()];
    (stamped, draws)
}

/// `visibility.md §8.1`, normative: for an actor whose composite lands on one
/// of the five selecting rows the variant is drawn "once per composite pass —
/// not once per placement — and it is never cached anywhere." The section is
/// explicit that "There is no cache to find", that the drawing path "derives
/// the variant afresh from the terrain beneath the actor every time the actor
/// is drawn", and that on the idle cheap path "every pass re-enters the same
/// arm and draws again". `§8.3` puts the observable consequence at "about
/// three passes in four", from "the probability that two draws separated by
/// one to five steps differ is 0.7508".
///
/// This replaces an earlier pin that asserted the opposite — a variant held
/// stable across animation ticks — on the strength of a runtime capture
/// reported as `cleak/u5-spec#179`. `cleak/u5-spec#182` settled that capture
/// rather than leaving the conflict open: the seat it used is a `§8`
/// *fall-through*, whose fixed tile correctly never changes, and the
/// named-cell recapture on seats that do qualify measured transitions on
/// every one of them. `RETRACTIONS.md` R329 re-scoped which rows select; it
/// did not withdraw the per-pass re-roll on the rows that do.
///
/// The pin is taken on `refresh_top_down_visibility_buffers`, the redraw's
/// composite pass, because that is the only path `§8.1`'s per-pass count is
/// defined over. The `&self` query helper reached from `top_down_render_cell`
/// deliberately takes no draw — a query must not advance the single global
/// stream — so a pin taken there could not observe a re-roll at all.
#[test]
fn the_compositor_variant_is_redrawn_on_every_composite_pass() {
    // A `0x92` chair whose row below holds the laden table `0x9C`: one of the
    // two qualifying seats among `§8`'s five selecting rows.
    let mut state = seated_npc_state(0x92, 0x9C);

    const PASSES: usize = 4_000;
    let mut seen = [0usize; 4];
    let mut transitions = 0usize;
    let mut previous: Option<u8> = None;
    for pass in 0..PASSES {
        let (stamped, draws) = idle_composite_pass(&mut state);
        assert_eq!(
            draws, 1,
            "pass {pass}: `§8.1` charges exactly one draw for a qualifying seat"
        );
        assert!(
            (0x34..=0x37).contains(&stamped),
            "pass {pass}: `§8` stamps 0x34..0x37 on this row, not {stamped:#04x}"
        );
        seen[usize::from(stamped - 0x34)] += 1;
        if previous.is_some_and(|earlier| earlier != stamped) {
            transitions += 1;
        }
        previous = Some(stamped);
    }

    // "it is never cached anywhere" — a cached variant would pin every pass
    // to one entry and leave three of these at zero.
    for (entry, count) in seen.iter().enumerate() {
        assert!(
            *count > 0,
            "entry {:#04x} never appeared over {PASSES} passes: the variant is being cached",
            0x34 + entry
        );
    }

    // `§8.3`'s "about three passes in four", the same 0.7508 the published
    // figure is computed from.
    let rate = transitions as f64 / (PASSES - 1) as f64;
    assert!(
        (rate - ACTIVE_OBJECT_VARIANT_TRANSITION_PROBABILITY).abs() < 0.03,
        "transition rate {rate} over {PASSES} idle passes is not the published ~0.75"
    );

    // The passes above ran with the animation clock untouched, so the re-roll
    // is a property of the pass and not of the animation phase. Driving the
    // clock changes nothing about it.
    let mut state = seated_npc_state(0x92, 0x9C);
    let mut phase_seen = [0usize; 4];
    for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
        state.animation = AnimationClock::at_static_tile_phase(phase);
        let (stamped, draws) = idle_composite_pass(&mut state);
        assert_eq!(draws, 1, "phase {phase} still takes exactly one draw");
        assert!((0x34..=0x37).contains(&stamped));
        phase_seen[usize::from(stamped - 0x34)] += 1;
    }
    assert!(
        phase_seen.iter().filter(|count| **count > 0).count() > 1,
        "the variant never moved across the animation period: it is being cached"
    );
}

/// The other half of `§8.3`: "a seated actor on any other chair is painted the
/// same fixed tile every pass and correctly never changes", and `§8.1` charges
/// it no draw. `RETRACTIONS.md` R329: "a seated actor that never changes tile
/// is the expected result for the majority of seats, not a defect."
#[test]
fn a_fall_through_seat_is_one_fixed_tile_on_every_pass_and_takes_no_draw() {
    // The same `0x92` chair over the plain table `0x95` — the shape of the
    // cell that produced the original null observation.
    let mut state = seated_npc_state(0x92, 0x95);

    for pass in 0..64 {
        let (stamped, draws) = idle_composite_pass(&mut state);
        assert_eq!(
            draws, 0,
            "pass {pass}: a fall-through row must not touch the shared stream"
        );
        assert_eq!(
            stamped, 0x32,
            "pass {pass}: `§8` stamps the fixed occupied-chair tile 0x32 here"
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

/// `animation.md §12.4` guard against a silent no-op on the shipped
/// artwork. Every other fire test here builds a synthetic atlas with a
/// hand-drawn mask, so all of them would still pass if the real masks
/// `0xC0..0xC3` / `0xCC..0xCF` were blank and the fire were static in the
/// actual game.
///
/// "Each mask is a small shape sitting exactly over its fixture's flame, so
/// only pixels inside the flame silhouette are ever touched", and a masked
/// pixel is XORed with `random_bit x (mask_pixel_colour AND planes)`. So a
/// fixture flickers in play only if its shipped mask has at least one pixel
/// whose colour intersects that fixture's noise-tile planes. This asserts
/// exactly that, for all nine published ids, against the shipped tiles.
///
/// Gated on local assets being present, like the other clean-asset smokes:
/// no artwork is committed and nothing is dumped.
#[test]
fn shipped_fire_masks_have_pixels_inside_their_noise_planes() {
    let game_dir = std::path::Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join(TILES_EGA_FILE).exists() {
        return;
    }
    let atlas = load_tile_atlas(game_dir, TileGraphicsDepth::Ega16).unwrap();

    for fixture in FIRE_FIXTURES {
        let planes = fire_noise_tile_planes(fixture.noise)
            .expect("every published fixture names a published noise tile");
        let base = usize::from(fixture.mask) * TILE_ATLAS_TILE_PIXELS;
        let live = atlas.pixels[base..base + TILE_ATLAS_TILE_PIXELS]
            .iter()
            .filter(|colour| **colour & planes != 0)
            .count();
        assert!(
            live > 0,
            "shipped mask 0x{:02X} for fixture 0x{:02X} has no pixel inside noise planes {planes}: \
             the fire would be static in play",
            fixture.mask,
            fixture.tile
        );
        assert!(
            live < TILE_ATLAS_TILE_PIXELS,
            "shipped mask 0x{:02X} covers the whole tile; §12.4 calls it \
             'a small shape sitting exactly over its fixture's flame'",
            fixture.mask
        );
    }
}

// ---------------------------------------------------------------------------
// `catalogs/item-list.md §7.2`, `systems/magic.md §8`, `systems/visibility.md
// §3`/`§4` — the shared spell/potion visibility sweep (R318, R327).
// ---------------------------------------------------------------------------

/// A 32x32 town grid whose party cell is walled in on all eight sides by the
/// `visibility.md §6` sight blocker `0x09`, with a distinct marker tile in the
/// far north-west corner of the eleven-by-eleven window.
fn walled_in_grid(px: usize, py: usize, marker: u8) -> Vec<u8> {
    let mut grid = u5_runtime::test_fixtures::open_grid();
    for dy in -1isize..=1 {
        for dx in -1isize..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            grid[(py as isize + dy) as usize * 32 + (px as isize + dx) as usize] = 0x09;
        }
    }
    grid[(py - 5) * 32 + (px - 5)] = marker;
    grid
}

/// `systems/visibility.md §3`, Negative row (corrected by R327): "Full-fill
/// path — populate every cell from the world map with no carve, no threshold
/// and no line-of-sight test."
///
/// `catalogs/item-list.md §7.2`: "There is no distance test, no propagation
/// frontier, and no blocker rule on this branch: a wall does not stop the
/// reveal, and a cell in the far corner is revealed exactly as readily as the
/// party's own."
///
/// R318 withdrew the earlier reading of this effect — the "inclusive
/// squared-Euclidean gate `dx*dx + dy*dy <= 32`" admitting "101 of the 121
/// cells", with "a blocker inside the gate visible but stopping propagation
/// past itself".
#[test]
fn the_negative_light_producer_branch_fills_all_121_cells_through_walls() {
    let state = u5_runtime::test_fixtures::test_state(walled_in_grid(10, 10, 0x23), 10, 10);

    let full_fill = state.surface_visibility_produce(
        10,
        10,
        VIEWPORT_PLAYER_ROW,
        VISIBILITY_NO_LINE_OF_SIGHT_LIGHT,
        false,
    );
    assert_eq!(full_fill.len(), VIEWPORT_SIDE * VIEWPORT_SIDE);
    assert_eq!(
        full_fill.iter().filter(|cell| **cell).count(),
        VIEWPORT_SIDE * VIEWPORT_SIDE,
        "the full-fill branch reveals all 121 cells"
    );
    assert!(full_fill.iter().all(|cell| *cell));

    // The withdrawn contract, evaluated on the same grid, for contrast: the
    // ring of blockers stops the carve dead, so a positive threshold of 32
    // reveals the party's cell and its eight walls and nothing else.
    let withdrawn = state.surface_visibility_produce(10, 10, VIEWPORT_PLAYER_ROW, 32, false);
    assert_eq!(
        withdrawn.iter().filter(|cell| **cell).count(),
        9,
        "the retracted threshold reading is what the full-fill branch is not"
    );

    // `§4` Stage 2, threshold zero: "the carve is skipped outright and the
    // grid is left fully obscured, including the player's own cell."
    let blackout = state.surface_visibility_produce(10, 10, VIEWPORT_PLAYER_ROW, 0, false);
    assert!(blackout.iter().all(|cell| !*cell));
}

/// `systems/magic.md §8`: "X-Ray (*Wis An Ylem*) is one of the two callers of
/// the shared visibility sweep — the other is the White potion — and that
/// sweep is a full reveal of the whole eleven-by-eleven viewport window
/// straight from the map, ignoring line of sight."
///
/// R327: "An engine that implemented the branch as dead compatibility code has
/// no working White potion and no working X-Ray."
#[test]
fn both_sweep_callers_reveal_the_same_121_cells() {
    let grid = walled_in_grid(10, 10, 0x23);

    let mut potion = u5_runtime::test_fixtures::test_state(grid.clone(), 10, 10);
    potion.potion_stock[POTION_WHITE_INDEX] = 1;
    assert_eq!(
        potion.use_potion(POTION_WHITE_INDEX, Some(0)),
        MoveOutcome::Observed
    );

    let mut spell = u5_runtime::test_fixtures::test_state(grid, 10, 10);
    spell.spell_charges[X_RAY_SPELL_INDEX] = 1;
    spell.party[0].mana = X_RAY_COST;
    spell.party[0].level = X_RAY_COST;
    assert_eq!(spell.cast_x_ray(0), MoveOutcome::Observed);

    let from_potion = potion.visibility_sweep.expect("White starts the sweep");
    let from_spell = spell.visibility_sweep.expect("X-Ray starts the sweep");

    assert_eq!(from_potion.visible_cells, from_spell.visible_cells);
    assert!(from_spell.visible_cells.iter().all(|cell| *cell));
    assert_eq!(
        from_spell.frames_remaining, POTION_WHITE_SWEEP_FRAMES,
        "both callers run the same twenty repaint frames"
    );
    assert_eq!(
        from_spell.pause_bios_ticks_per_frame,
        POTION_WHITE_SWEEP_BIOS_TICKS_PER_FRAME
    );
    assert!(
        spell.active_view_overlay.is_none(),
        "`item-list.md §7.2`: the sweep branch does not enter the modal View overlay"
    );
}

// ---------------------------------------------------------------------------
// `systems/animation.md §13.5` — blocking presentations and actor movement.
// ---------------------------------------------------------------------------

/// `systems/animation.md §13.5`, claim 1: "No blocking presentation runs the
/// town NPC schedule processor, the town object walker that moves loose
/// horse-family objects, or the outdoor per-turn creature walker ...
/// **Exceptions: none.**"
///
/// `catalogs/item-list.md §7.2` says the same of the sweep's own per-frame
/// step: it "advances sprite appearance only and **moves no actor**".
///
/// The presentation still pays for everything `§13.2` lists — the sprite
/// phases and the tile layers are expected to move here; only coordinates are
/// not.
#[test]
fn a_presentation_sweep_leaves_every_actor_coordinate_unchanged() {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.active_objects.truncate(1);
    for (type_byte, x, y) in [
        // A wandering land monster, orthogonally adjacent to the party.
        (176u8, 101usize, 100usize),
        // A wind-driven ship on the same plane.
        (168, 104, 100),
        // A loose horse-family object, the town object walker's only class.
        (0x10, 98, 101),
    ] {
        state.active_objects.push(ActiveObject {
            type_byte,
            tile: type_byte,
            x,
            y,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
    }
    state.wind = WindState::North;

    let placed: Vec<(usize, usize)> = state
        .active_objects
        .iter()
        .map(|object| (object.x, object.y))
        .collect();
    let party_before = (state.player.x, state.player.y);
    let turn_before = state.turn;
    let clock_before = state.clock;

    state.start_visibility_sweep();
    let mut frames = 0usize;
    while state.visibility_sweep.is_some() {
        state.advance_presentation_frame();
        frames += 1;
        assert_eq!(
            state
                .active_objects
                .iter()
                .map(|object| (object.x, object.y))
                .collect::<Vec<_>>(),
            placed,
            "frame {frames} of a blocking presentation moved an actor"
        );
    }

    assert_eq!(usize::from(POTION_WHITE_SWEEP_FRAMES), frames);
    assert_eq!((state.player.x, state.player.y), party_before);
    assert_eq!(
        state.turn, turn_before,
        "neither the loop nor the final idle redraw spends a turn"
    );
    assert_eq!(state.clock, clock_before);
    assert!(
        u32::from(state.animation.frame) > 0,
        "`§13.5`: a presentation that pumps the world redraw still pays the tile layers"
    );
}
