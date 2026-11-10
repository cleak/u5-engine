// Phase B M1-07: the ring peek's objects, purity and timing (design
// 2.3.6, 2.6).
mod ring_peek_objects {
    use super::*;

    const WIDE: PeekWindow = PeekWindow {
        west: 15,
        east: 15,
        north: 7,
        south: 7,
    };

    /// The 21:9 window of design 2.3.6: 43 by 15 cells.
    const ULTRAWIDE: PeekWindow = PeekWindow {
        west: 21,
        east: 21,
        north: 7,
        south: 7,
    };

    const POLICIES: [PeekLightPolicy; 2] = [PeekLightPolicy::Scaled, PeekLightPolicy::Faithful];

    const TOWN_WALL: u8 = 0x4E;
    /// An ordinary townsperson: the default helper, not terrain-aware.
    const PERSON: u8 = 0x5A;
    /// A seated-capable actor (terrain-aware, below 0x80).
    const SITTER: u8 = 0x44;
    const CHAIR_FACING_DOWN: u8 = 0x92;
    const LADEN_TABLE: u8 = 0x9A;
    const TRAPPED_SOUL_ROW: u8 = 0x9D;

    fn rendered(mut state: PlayState) -> PlayState {
        let area = state.top_down_render_area().unwrap();
        state.refresh_top_down_visibility_buffers(area, VIEWPORT_PLAYER_ROW);
        assert!(state.visibility_buffers_ready);
        state
    }

    fn town_at_noon(grid: Vec<u8>) -> PlayState {
        let mut state = test_state(grid, 16, 16);
        state.ambient_light = FULL_DAYLIGHT;
        state.rebuild_surface_local_light_mask();
        state
    }

    fn object(type_byte: u8, x: usize, y: usize, z: i8) -> ActiveObject {
        ActiveObject {
            type_byte,
            tile: type_byte,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    }

    /// Put `objects` into slots 1.. (slot 0 is the party).
    fn with_objects(mut state: PlayState, objects: &[ActiveObject]) -> PlayState {
        state.active_objects.truncate(1);
        state.active_objects.extend_from_slice(objects);
        state
    }

    fn cell(ring: &TopDownPeekRing, dx: isize, dy: isize) -> PeekCell {
        ring.cells[ring.window.index_of(dx, dy).unwrap()].unwrap()
    }

    fn actor(byte: u8) -> PeekCell {
        PeekCell {
            tile: byte,
            shade: PeekShade::Lit,
            atlas: Some(u16::from(byte) + 256),
        }
    }

    #[test]
    fn an_object_in_a_lit_ring_cell_draws_its_actor_tile() {
        let state = rendered(with_objects(
            town_at_noon(open_grid()),
            &[object(PERSON, 16 + 9, 16 - 3, 0)],
        ));
        let open = state.animation.resolve_static_tile(0x10);
        for light in POLICIES {
            let ring = state.peek_top_down_ring(WIDE, light).unwrap();
            let peeked = cell(&ring, 9, -3);
            if light == PeekLightPolicy::Scaled {
                assert_eq!(peeked, actor(PERSON));
                // Its neighbours keep their terrain.
                assert_eq!(cell(&ring, 8, -3).atlas, Some(u16::from(open)));
            } else {
                // Outside the faithful noon disc the cell is unlit, and an
                // object never shows on a cell the party cannot see.
                assert_eq!(peeked, PeekCell::HIDDEN);
            }
        }
    }

    #[test]
    fn objects_show_only_on_visible_ring_cells_of_the_current_floor() {
        let mut grid = open_grid();
        // A wall at the east core edge: everything east of it is hidden.
        for y in 0..32 {
            grid[y * 32 + 21] = TOWN_WALL;
        }
        let state = rendered(with_objects(
            town_at_noon(grid),
            &[
                object(PERSON, 16 + 8, 16, 0),  // behind the wall
                object(PERSON, 16 - 8, 16, 1),  // another floor
                object(PERSON, 16 - 8, 16 - 6, 0), // visible
                object(PERSON, 16 + 2, 16 + 2, 0), // in the core
            ],
        ));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
            .unwrap();
        assert_eq!(cell(&ring, 8, 0), PeekCell::HIDDEN);
        assert_eq!(cell(&ring, -8, 0).atlas, Some(0x10));
        assert_eq!(cell(&ring, -8, -6), actor(PERSON));
        // The core stays the faithful frame's.
        assert!(ring.cells[WIDE.index_of(2, 2).unwrap()].is_none());
        let actors = ring
            .cells
            .iter()
            .flatten()
            .filter(|cell| cell.atlas.is_some_and(|id| id >= 256))
            .count();
        assert_eq!(actors, 1);
    }

    #[test]
    fn a_lower_slot_paints_over_a_higher_one() {
        let state = rendered(with_objects(
            town_at_noon(open_grid()),
            &[object(0x50, 16 + 7, 16, 0), object(PERSON, 16 + 7, 16, 0)],
        ));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
            .unwrap();
        // Slot 1 (0x50) is composited last, as the faithful pass walks
        // thirty-one down to one.
        assert_eq!(cell(&ring, 7, 0), actor(0x50));
    }

    #[test]
    fn world_objects_are_placed_by_the_wrapped_delta() {
        let (px, py) = (3_usize, 100_usize);
        let mut state = world_state(open_world_grid(), px, py);
        state.ambient_light = FULL_DAYLIGHT;
        let z = state.current_floor().unwrap();
        // Ten cells west of the party, across the 256 seam.
        let state = rendered(with_objects(state, &[object(PERSON, 249, py + 1, z)]));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
            .unwrap();
        assert_eq!(cell(&ring, -10, 1), actor(PERSON));
    }

    /// `visibility.md §8.1`: the peek takes variant 0, as the query-path
    /// compositor does, and never draws from the shared stream.
    #[test]
    fn seated_actors_use_variant_zero() {
        let mut grid = open_grid();
        let (cx, cy) = (16 + 8, 16 + 1);
        grid[cy * 32 + cx] = CHAIR_FACING_DOWN;
        grid[(cy + 1) * 32 + cx] = LADEN_TABLE;
        let mut state = rendered(with_objects(
            town_at_noon(grid),
            &[object(SITTER, cx, cy, 0)],
        ));
        // The faithful selector would choose among 0x34..=0x37 here.
        assert_eq!(
            active_object_default_variant_base(
                SITTER,
                CHAIR_FACING_DOWN,
                Some(0x10),
                Some(LADEN_TABLE)
            ),
            Some(0x34)
        );
        for seed in [DEFAULT_PRNG_STATE, 1, 0x1234, 0x7FFF] {
            state.prng_state = seed;
            let ring = state
                .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
                .unwrap();
            assert_eq!(cell(&ring, 8, 1), actor(0x34), "seed {seed:?}");
            assert_eq!(state.prng_state, seed);
        }
    }

    #[test]
    fn a_previous_row_marker_never_writes_into_the_core() {
        let mut grid = open_grid();
        // The trapped-soul row: an ordinary actor under a 0x9D cell marks
        // the cell above with 0x9E.
        grid[(16 + 5) * 32 + 16] = TRAPPED_SOUL_ROW; // core row dy = 5
        grid[(16 + 6) * 32 + 16 + 7] = TRAPPED_SOUL_ROW; // ring row dy = 6
        let state = rendered(with_objects(
            town_at_noon(grid),
            &[object(PERSON, 16, 16 + 6, 0), object(PERSON, 16 + 7, 16 + 7, 0)],
        ));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
            .unwrap();
        // Under the core edge: stamped, but the core above is untouched.
        assert_eq!(cell(&ring, 0, 6), actor(PERSON));
        assert!(ring.cells[WIDE.index_of(0, 5).unwrap()].is_none());
        // Fully in the ring: the cell above takes the direct marker.
        assert_eq!(cell(&ring, 7, 7), actor(PERSON));
        assert_eq!(
            cell(&ring, 7, 6),
            PeekCell {
                tile: 0x9E,
                shade: PeekShade::Lit,
                atlas: Some(0x9E),
            }
        );
    }

    #[test]
    fn the_draw_nothing_byte_leaves_the_terrain_showing() {
        let state = rendered(with_objects(
            town_at_noon(open_grid()),
            &[object(ACTOR_TILE_TRANSPARENT_BYTE, 16 + 9, 16, 0)],
        ));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
            .unwrap();
        assert_eq!(cell(&ring, 9, 0).atlas, Some(0x10));
        assert!(
            ring.cells
                .iter()
                .flatten()
                .all(|cell| (cell.shade == PeekShade::Lit) == cell.atlas.is_some())
        );
    }

    #[test]
    fn the_animated_flag_follows_the_lit_ring_cells() {
        let state = rendered(town_at_noon(open_grid()));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
            .unwrap();
        assert!(!ring.animated);

        // Water in the ring animates.
        let water = (0..=u8::MAX)
            .find(|&tile| water_pass_rotates_tile(tile))
            .expect("a rotated water id");
        let mut grid = open_grid();
        grid[16 * 32 + 16 + 9] = water;
        let state = rendered(town_at_noon(grid.clone()));
        assert!(
            state
                .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
                .unwrap()
                .animated
        );
        // The same water out of sight does not.
        for y in 0..32 {
            grid[y * 32 + 21] = TOWN_WALL;
        }
        let state = rendered(town_at_noon(grid));
        assert!(
            !state
                .peek_top_down_ring(WIDE, PeekLightPolicy::Scaled)
                .unwrap()
                .animated
        );
    }

    #[test]
    fn the_rasteriser_draws_lit_ring_cells_and_leaves_the_rest_black() {
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let state = rendered(with_objects(
            town_at_noon(open_grid()),
            &[object(PERSON, 16 + 9, 16 - 3, 0)],
        ));
        let ring = state
            .peek_top_down_ring(WIDE, PeekLightPolicy::Faithful)
            .unwrap();
        let before = format!("{state:?}");
        let raster = state.render_peek_ring(&ring, &atlas).unwrap();
        assert_eq!(format!("{state:?}"), before);
        assert_eq!(raster.width, WIDE.columns() * TILE_ATLAS_SIDE);
        assert_eq!(raster.height, WIDE.rows() * TILE_ATLAS_SIDE);
        let limit = atlas.depth.pixel_limit();
        for (index, peeked) in ring.cells.iter().enumerate() {
            let x = (index % WIDE.columns()) * TILE_ATLAS_SIDE + 3;
            let y = (index / WIDE.columns()) * TILE_ATLAS_SIDE + 5;
            let expected = match peeked {
                Some(PeekCell {
                    atlas: Some(id), ..
                }) => (*id as usize as u8) % limit,
                _ => 0,
            };
            assert_eq!(raster.pixels[y * raster.width + x], expected, "cell {index}");
        }
    }

    /// Design 2.6: clone a state, call the peek 100 times, and compare the
    /// whole state, including `prng_state`.
    #[test]
    fn the_peek_with_objects_is_pure() {
        let mut grid = open_grid();
        let (cx, cy) = (16 + 8, 16 + 1);
        grid[cy * 32 + cx] = CHAIR_FACING_DOWN;
        grid[(cy + 1) * 32 + cx] = LADEN_TABLE;
        let state = rendered(with_objects(
            town_at_noon(grid),
            &[
                object(SITTER, cx, cy, 0),
                object(PERSON, 16 - 9, 16 + 4, 0),
                object(PERSON, 16 + 3, 16 - 7, 0),
            ],
        ));
        let pristine = state.clone();
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let first = state.peek_top_down_ring(ULTRAWIDE, PeekLightPolicy::Scaled);
        for call in 0..100 {
            let light = POLICIES[call % 2];
            let ring = state.peek_top_down_ring(ULTRAWIDE, light).unwrap();
            if light == PeekLightPolicy::Scaled {
                assert_eq!(Some(&ring), first.as_ref());
            }
            state.render_peek_ring(&ring, &atlas).unwrap();
        }
        assert_eq!(state.prng_state, pristine.prng_state);
        assert_eq!(format!("{state:?}"), format!("{pristine:?}"));
    }

    /// Design 2.6 and 2.7: the peek at the 21:9 window (43 by 15 cells, a
    /// 524-cell ring) is cheap next to a frame. The bound is loose enough
    /// for an unoptimised test build on a loaded host; the release cost is
    /// far below it.
    #[test]
    fn the_ultrawide_peek_is_cheap() {
        assert_eq!(ULTRAWIDE.cell_count() - 121, 524);
        let (px, py) = (100_usize, 100_usize);
        let mut state = world_state(open_world_grid(), px, py);
        state.ambient_light = FULL_DAYLIGHT;
        let z = state.current_floor().unwrap();
        let objects: Vec<ActiveObject> = (0..31)
            .map(|i| object(PERSON, px - 20 + (i * 3) % 41, py - 7 + i % 15, z))
            .collect();
        let state = rendered(with_objects(state, &objects));
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let mut samples = Vec::new();
        for _ in 0..9 {
            let started = std::time::Instant::now();
            for _ in 0..10 {
                let ring = state
                    .peek_top_down_ring(ULTRAWIDE, PeekLightPolicy::Scaled)
                    .unwrap();
                std::hint::black_box(state.render_peek_ring(&ring, &atlas).unwrap());
            }
            samples.push(started.elapsed() / 10);
        }
        samples.sort();
        let median = samples[samples.len() / 2];
        eprintln!("ring peek + raster at 43x15: median {median:?}");
        assert!(
            median < std::time::Duration::from_millis(20),
            "the 21:9 ring peek took {median:?}"
        );
    }
}
