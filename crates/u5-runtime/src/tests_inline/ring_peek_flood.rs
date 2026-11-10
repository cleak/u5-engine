// Phase B M1-06: the ring peek's sight flood and light (design 2.3.6,
// 2.2.9).
mod ring_peek_flood {
    use super::*;

    const WIDE: PeekWindow = PeekWindow {
        west: 15,
        east: 15,
        north: 7,
        south: 7,
    };

    const POLICIES: [PeekLightPolicy; 2] = [PeekLightPolicy::Scaled, PeekLightPolicy::Faithful];

    /// Open town floor (0x10) and open overworld ground (0x05) both
    /// propagate sight; 0x4E and 0x0C are sight blockers.
    const TOWN_OPEN: u8 = 0x10;
    const TOWN_WALL: u8 = 0x4E;
    const WORLD_WALL: u8 = 0x0C;
    const TOWN_LIGHT_SOURCE: u8 = 0xB0;

    fn squared(dx: isize, dy: isize) -> u32 {
        (dx * dx + dy * dy) as u32
    }

    /// The faithful render's visibility refresh, which the peek reads.
    fn rendered(mut state: PlayState) -> PlayState {
        let area = state.top_down_render_area().unwrap();
        state.refresh_top_down_visibility_buffers(area, VIEWPORT_PLAYER_ROW);
        assert!(state.visibility_buffers_ready);
        state
    }

    fn peek(state: &PlayState, light: PeekLightPolicy) -> TopDownPeekRing {
        let ring = state.peek_top_down_ring(WIDE, light).unwrap();
        assert_eq!(ring.window, WIDE);
        assert_eq!(ring.cells.len(), WIDE.cell_count());
        for (index, cell) in ring.cells.iter().enumerate() {
            let (dx, dy) = WIDE.offset_of(index).unwrap();
            // Core cells are always `None`; ring cells never are.
            assert_eq!(
                cell.is_none(),
                PeekWindow::is_core(dx, dy),
                "core/ring split at ({dx}, {dy})"
            );
        }
        ring
    }

    fn cell(ring: &TopDownPeekRing, dx: isize, dy: isize) -> PeekCell {
        ring.cells[ring.window.index_of(dx, dy).unwrap()].unwrap()
    }

    fn ring_offsets(window: PeekWindow) -> impl Iterator<Item = (isize, isize)> {
        (0..window.cell_count())
            .map(move |index| window.offset_of(index).unwrap())
            .filter(|&(dx, dy)| !PeekWindow::is_core(dx, dy))
    }

    fn assert_all_hidden(ring: &TopDownPeekRing, label: &str) {
        for (dx, dy) in ring_offsets(ring.window) {
            assert_eq!(cell(ring, dx, dy), PeekCell::HIDDEN, "{label} ({dx}, {dy})");
        }
    }

    fn town(grid: Vec<u8>, x: usize, y: usize, ambient: u8) -> PlayState {
        let mut state = test_state(grid, x, y);
        state.ambient_light = ambient;
        state.rebuild_surface_local_light_mask();
        rendered(state)
    }

    #[test]
    fn noon_scaled_shows_the_whole_window_and_faithful_a_disc() {
        let state = town(open_grid(), 16, 16, FULL_DAYLIGHT);
        let open = state.animation.resolve_static_tile(TOWN_OPEN);
        let scaled = peek(&state, PeekLightPolicy::Scaled);
        for (dx, dy) in ring_offsets(WIDE) {
            assert_eq!(
                cell(&scaled, dx, dy),
                PeekCell {
                    tile: open,
                    shade: PeekShade::Lit
                },
                "scaled ({dx}, {dy})"
            );
        }
        let faithful = peek(&state, PeekLightPolicy::Faithful);
        for (dx, dy) in ring_offsets(WIDE) {
            let shade = cell(&faithful, dx, dy).shade;
            let expected = if squared(dx, dy) <= u32::from(FULL_DAYLIGHT) {
                PeekShade::Lit
            } else {
                PeekShade::Hidden
            };
            assert_eq!(shade, expected, "faithful ({dx}, {dy})");
        }
        // The design's figure: about seven cells in radius at noon.
        assert_eq!(cell(&faithful, 7, 1).shade, PeekShade::Lit);
        assert_eq!(cell(&faithful, 6, 4).shade, PeekShade::Hidden);
    }

    #[test]
    fn a_wall_at_the_core_edge_hides_what_is_behind_it_on_all_four_sides() {
        // (label, wall cells, what is behind the wall)
        type Side = (&'static str, fn(isize, isize) -> bool, fn(isize, isize) -> bool);
        let sides: [Side; 4] = [
            ("west", |dx, _| dx == -5, |dx, _| dx < -5),
            ("east", |dx, _| dx == 5, |dx, _| dx > 5),
            ("north", |_, dy| dy == -5, |_, dy| dy < -5),
            ("south", |_, dy| dy == 5, |_, dy| dy > 5),
        ];
        let (px, py) = (16_isize, 16_isize);
        for (label, wall, behind) in sides {
            let mut grid = open_grid();
            for y in 0..32_isize {
                for x in 0..32_isize {
                    if wall(x - px, y - py) {
                        grid[(y * 32 + x) as usize] = TOWN_WALL;
                    }
                }
            }
            let state = town(grid, px as usize, py as usize, FULL_DAYLIGHT);
            let wall_tile = state.animation.resolve_static_tile(TOWN_WALL);
            for light in POLICIES {
                let ring = peek(&state, light);
                let mut wall_seen = 0;
                for (dx, dy) in ring_offsets(WIDE) {
                    let peeked = cell(&ring, dx, dy);
                    if behind(dx, dy) {
                        assert_eq!(peeked, PeekCell::HIDDEN, "{label} {light:?} ({dx}, {dy})");
                    } else if wall(dx, dy) && peeked.shade == PeekShade::Lit {
                        // The wall itself is painted, never seen through.
                        assert_eq!(peeked.tile, wall_tile);
                        wall_seen += 1;
                    }
                }
                // At noon the scaled policy has the whole wall in range;
                // the faithful disc leaves the ring part of it unlit.
                assert_eq!(
                    wall_seen > 0,
                    light == PeekLightPolicy::Scaled,
                    "{label} {light:?}"
                );
            }
            // The open side opposite the wall still shows at noon.
            let ring = peek(&state, PeekLightPolicy::Scaled);
            let lit_open = ring_offsets(WIDE)
                .filter(|&(dx, dy)| !behind(dx, dy) && !wall(dx, dy))
                .all(|(dx, dy)| cell(&ring, dx, dy).shade == PeekShade::Lit);
            assert!(lit_open, "{label}");
        }
    }

    #[test]
    fn a_walled_core_edge_shows_no_ring_at_all() {
        let (px, py) = (16_isize, 16_isize);
        let mut grid = open_grid();
        for y in 0..32_isize {
            for x in 0..32_isize {
                let (dx, dy) = (x - px, y - py);
                if dx.abs().max(dy.abs()) == 5 {
                    grid[(y * 32 + x) as usize] = TOWN_WALL;
                }
            }
        }
        let state = town(grid, px as usize, py as usize, FULL_DAYLIGHT);
        for light in POLICIES {
            assert_all_hidden(&peek(&state, light), "walled core edge");
        }
    }

    /// A small deterministic generator for test maps; it never touches
    /// the game's own stream.
    struct Lcg(u32);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            self.0 >> 8
        }
    }

    /// The no-reveal rule for the map (2.2.9): a lit ring cell must be
    /// reachable from a visible, sight-propagating core-edge cell through
    /// ring cells whose terrain propagates sight, or sit next to one.
    #[test]
    fn no_ring_cell_is_visible_unless_reachable_from_a_visible_core_edge_cell() {
        let mut rng = Lcg(0x5EED_1234);
        let ambients = [FULL_DAYLIGHT, 49, 34, 20, 18, 10, 5];
        let mut lit_total = 0;
        for round in 0..60 {
            let mut grid = open_grid();
            for tile in grid.iter_mut() {
                *tile = match rng.next() % 100 {
                    0..=29 => TOWN_WALL,
                    30..=32 => TOWN_LIGHT_SOURCE,
                    _ => TOWN_OPEN,
                };
            }
            let px = 6 + (rng.next() % 20) as usize;
            let py = 6 + (rng.next() % 20) as usize;
            grid[py * 32 + px] = TOWN_OPEN;
            let ambient = ambients[round % ambients.len()];
            let state = town(grid, px, py, ambient);
            let (px, py) = (px as isize, py as isize);
            // The town viewport read: the floor, or its southeast corner
            // off the floor (town-mode.md 15).
            let tile_at = |dx: isize, dy: isize| {
                let (x, y) = (px + dx, py + dy);
                Some(if (0..32).contains(&x) && (0..32).contains(&y) {
                    state.grid[(y * 32 + x) as usize]
                } else {
                    state.grid[TOWN_VIEWPORT_OFF_GRID_SAMPLE_INDEX]
                })
            };
            // At the core edge and beyond no cell is adjacent to the party,
            // so a tile propagates exactly when it neither blocks nor needs
            // adjacency.
            let propagates = |dx: isize, dy: isize| {
                debug_assert!(squared(dx, dy) > 1);
                tile_at(dx, dy).is_some_and(|tile| {
                    !tile_blocks_sight_propagation(tile)
                        && !tile_propagates_sight_only_when_adjacent(tile)
                })
            };

            // The expandable set: visible propagating core-edge seeds, then
            // every ring cell reachable from them over propagating ring
            // terrain. A necessary condition, independent of the flood.
            let mut expandable = vec![false; WIDE.cell_count()];
            let mut queue = std::collections::VecDeque::new();
            for dy in -5_isize..=5 {
                for dx in -5_isize..=5 {
                    if dx.abs().max(dy.abs()) != 5 {
                        continue;
                    }
                    let grid_index =
                        visibility_grid_active_index((dy + 5) as usize, (dx + 5) as usize).unwrap();
                    if state.visibility_grid[grid_index] != VISIBILITY_HIDDEN && propagates(dx, dy)
                    {
                        expandable[WIDE.index_of(dx, dy).unwrap()] = true;
                        queue.push_back((dx, dy));
                    }
                }
            }
            while let Some((cx, cy)) = queue.pop_front() {
                for (ndx, ndy) in VISIBILITY_CARVE_NEIGHBOR_ORDER {
                    let (x, y) = (cx + isize::from(ndx), cy + isize::from(ndy));
                    if PeekWindow::is_core(x, y) {
                        continue;
                    }
                    let Some(index) = WIDE.index_of(x, y) else {
                        continue;
                    };
                    if !expandable[index] && propagates(x, y) {
                        expandable[index] = true;
                        queue.push_back((x, y));
                    }
                }
            }

            for light in POLICIES {
                let ring = peek(&state, light);
                for (dx, dy) in ring_offsets(WIDE) {
                    let peeked = cell(&ring, dx, dy);
                    if peeked.shade == PeekShade::Hidden {
                        assert_eq!(peeked, PeekCell::HIDDEN);
                        continue;
                    }
                    lit_total += 1;
                    let reached = VISIBILITY_CARVE_NEIGHBOR_ORDER.iter().any(|&(ndx, ndy)| {
                        WIDE.index_of(dx + isize::from(ndx), dy + isize::from(ndy))
                            .is_some_and(|index| expandable[index])
                    });
                    assert!(reached, "round {round} {light:?} ({dx}, {dy}) seen without a path");
                    assert_eq!(
                        Some(peeked.tile),
                        tile_at(dx, dy).map(|tile| state.animation.resolve_static_tile(tile)),
                        "round {round} ({dx}, {dy})"
                    );
                }
            }
        }
        // The maps are not so closed that nothing is ever seen.
        assert!(lit_total > 1000, "{lit_total}");
    }

    #[test]
    fn pitch_dark_gives_an_all_hidden_ring() {
        let mut state = town(open_grid(), 16, 16, 0);
        assert!(state.surface_visibility_pitch_dark());
        for light in POLICIES {
            assert_all_hidden(&peek(&state, light), "pitch dark");
        }
        // The gate does not lean on the empty core: even a buffer that
        // claims the core edge is visible seeds nothing in the dark.
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                state.visibility_grid[visibility_grid_active_index(row, col).unwrap()] =
                    VISIBILITY_CLEAR;
            }
        }
        for light in POLICIES {
            assert_all_hidden(&peek(&state, light), "pitch dark, forced core");
        }
    }

    #[test]
    fn the_underfoot_blackout_gives_an_all_hidden_ring() {
        let (px, py) = (40_usize, 60_usize);
        let mut grid = open_world_grid();
        grid[world_cell_index(px, py)] = OVERWORLD_UNDERFOOT_BLACKOUT_TILE;
        let mut state = world_state(grid, px, py);
        state.ambient_light = FULL_DAYLIGHT;
        assert!(state.world_underfoot_blackout_active());
        let mut state = rendered(state);
        for light in POLICIES {
            assert_all_hidden(&peek(&state, light), "underfoot blackout");
        }
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                state.visibility_grid[visibility_grid_active_index(row, col).unwrap()] =
                    VISIBILITY_CLEAR;
            }
        }
        for light in POLICIES {
            assert_all_hidden(&peek(&state, light), "underfoot blackout, forced core");
        }

        // Off the blackout tile the same daylight shows the ring.
        let mut state = world_state(open_world_grid(), px, py);
        state.ambient_light = FULL_DAYLIGHT;
        let state = rendered(state);
        let ring = peek(&state, PeekLightPolicy::Scaled);
        assert!(ring_offsets(WIDE).all(|(dx, dy)| cell(&ring, dx, dy).shade == PeekShade::Lit));
    }

    /// Below full daylight both policies use the true squared distance, so
    /// dusk and a torch at night are radii in world cells.
    #[test]
    fn a_torch_at_night_lights_a_radius_in_world_cells_under_both_policies() {
        // The torch floor (18) reaches no ring cell; dusk levels reach the
        // cells whose squared distance is within the threshold.
        for ambient in [18_u8, 20, 34, 49] {
            let state = town(open_grid(), 16, 16, ambient);
            for light in POLICIES {
                let ring = peek(&state, light);
                for (dx, dy) in ring_offsets(WIDE) {
                    let expected = if squared(dx, dy) <= u32::from(ambient) {
                        PeekShade::Lit
                    } else {
                        PeekShade::Hidden
                    };
                    assert_eq!(
                        cell(&ring, dx, dy).shade,
                        expected,
                        "ambient {ambient} {light:?} ({dx}, {dy})"
                    );
                }
            }
        }
        // The same holds outdoors.
        let mut state = world_state(open_world_grid(), 40, 60);
        state.ambient_light = 34;
        let state = rendered(state);
        for light in POLICIES {
            let ring = peek(&state, light);
            for (dx, dy) in ring_offsets(WIDE) {
                assert_eq!(
                    cell(&ring, dx, dy).shade == PeekShade::Lit,
                    squared(dx, dy) <= 34,
                    "world {light:?} ({dx}, {dy})"
                );
            }
        }
    }

    /// Beyond the threshold the local light mask decides, as in the
    /// faithful carve: a lamp past the core edge lights its own disc.
    #[test]
    fn a_light_source_past_the_core_lights_its_disc_at_dusk() {
        let mut grid = open_grid();
        grid[16 * 32 + 24] = TOWN_LIGHT_SOURCE;
        let state = town(grid, 16, 16, 34);
        for light in POLICIES {
            let ring = peek(&state, light);
            for (dx, dy) in ring_offsets(WIDE) {
                let in_range = squared(dx, dy) <= 34;
                let in_lamp = squared(dx - 8, dy) <= LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD;
                assert_eq!(
                    cell(&ring, dx, dy).shade == PeekShade::Lit,
                    in_range || in_lamp,
                    "{light:?} ({dx}, {dy})"
                );
            }
        }
        // Without the lamp the same cells are dark.
        let state = town(open_grid(), 16, 16, 34);
        let ring = peek(&state, PeekLightPolicy::Scaled);
        assert_eq!(cell(&ring, 9, 0), PeekCell::HIDDEN);
    }

    #[test]
    fn town_cells_off_the_floor_take_the_southeast_corner_terrain() {
        const CORNER: u8 = 0x05;
        let mut grid = open_grid();
        grid[TOWN_VIEWPORT_OFF_GRID_SAMPLE_INDEX] = CORNER;
        // Party at the west edge of the floor: cells west of x = 0 are off
        // the floor.
        let state = town(grid, 2, 16, FULL_DAYLIGHT);
        let corner = state.animation.resolve_static_tile(CORNER);
        let open = state.animation.resolve_static_tile(TOWN_OPEN);
        let ring = peek(&state, PeekLightPolicy::Scaled);
        for (dx, dy) in ring_offsets(WIDE) {
            let x = 2 + dx;
            let y = 16 + dy;
            let off_floor = !(0..32).contains(&x) || !(0..32).contains(&y);
            let expected = if off_floor { corner } else { open };
            assert_eq!(
                cell(&ring, dx, dy),
                PeekCell {
                    tile: expected,
                    shade: PeekShade::Lit
                },
                "({dx}, {dy})"
            );
        }
    }

    #[test]
    fn the_world_wraps_at_the_256_seam() {
        const SEAM: u8 = 0x06;
        let (px, py) = (2_usize, 100_usize);
        let mut grid = open_world_grid();
        // A band of different ground just across the west seam, then a
        // wall at world x = 247 (dx = -11).
        for y in 0..WORLD_SIDE {
            for x in 248..WORLD_SIDE {
                grid[world_cell_index(x, y)] = SEAM;
            }
            grid[world_cell_index(247, y)] = WORLD_WALL;
        }
        let mut state = world_state(grid, px, py);
        state.ambient_light = FULL_DAYLIGHT;
        let state = rendered(state);
        let seam = state.animation.resolve_static_tile(SEAM);
        let open = state.animation.resolve_static_tile(0x05);
        let wall = state.animation.resolve_static_tile(WORLD_WALL);
        let ring = peek(&state, PeekLightPolicy::Scaled);
        for (dx, dy) in ring_offsets(WIDE) {
            let peeked = cell(&ring, dx, dy);
            let world_x = (px as isize + dx).rem_euclid(WORLD_SIDE as isize);
            match dx {
                ..=-12 => assert_eq!(peeked, PeekCell::HIDDEN, "({dx}, {dy})"),
                -11 => assert_eq!(peeked.tile, wall, "({dx}, {dy})"),
                _ => {
                    let expected = if world_x >= 248 { seam } else { open };
                    assert_eq!(
                        peeked,
                        PeekCell {
                            tile: expected,
                            shade: PeekShade::Lit
                        },
                        "({dx}, {dy})"
                    );
                }
            }
        }
        // The seam band really is across the wrap.
        assert_eq!(cell(&ring, -6, 0).tile, seam);
        assert_eq!(cell(&ring, 6, 0).tile, open);
    }

    #[test]
    fn the_flood_leaves_the_state_untouched() {
        let mut grid = open_grid();
        grid[16 * 32 + 24] = TOWN_LIGHT_SOURCE;
        let state = town(grid, 16, 16, 34);
        let before = format!("{state:?}");
        let prng = state.prng_state;
        let first = state.peek_top_down_ring(WIDE, PeekLightPolicy::Scaled);
        for call in 0..100 {
            let light = POLICIES[call % 2];
            let ring = state.peek_top_down_ring(WIDE, light);
            if light == PeekLightPolicy::Scaled {
                assert_eq!(ring, first);
            }
        }
        assert_eq!(state.prng_state, prng);
        assert_eq!(format!("{state:?}"), before);
    }
}
