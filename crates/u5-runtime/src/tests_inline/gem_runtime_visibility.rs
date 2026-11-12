// Gem map M3-03 (design 10.3.1, automap 4.3.1): the runtime's
// visibility-only additions. `gem_sample_tile` carries the body of the
// faithful local-view lookup, `view_overlay_footprint` mirrors the dungeon
// View flood, and the two gem-view painters are public. None of them may
// change a faithful result or the game state.

/// The faithful local-view lookup exactly as it read before M3-03 moved its
/// body into `gem_sample_tile`: the oracle the delegation is pinned to.
fn gem_legacy_surface_view_tile_at(state: &PlayState, cell_x: usize, cell_y: usize) -> u8 {
    let px = state.player.x as isize;
    let py = state.player.y as isize;
    let side = LOCAL_VIEW_OVERLAY_SIDE as isize;
    let x = px - side / 2 + cell_x as isize;
    let y = py - side / 2 + cell_y as isize;
    match state.area {
        Area::Town { .. } => {
            let (x, y) = (cell_x as isize, cell_y as isize);
            if !(0..32).contains(&x) || !(0..32).contains(&y) {
                return 0;
            }
            if let Some(object) = state.object_at_current_floor(x as usize, y as usize) {
                object.tile
            } else {
                let tile = state.grid[y as usize * 32 + x as usize];
                state.animation.resolve_static_tile(tile)
            }
        }
        Area::World { .. } => {
            let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
            let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
            if let Some(object) = state.world_object_at(wx, wy) {
                object.tile
            } else {
                let tile = state.grid[world_cell_index(wx, wy)];
                state.animation.resolve_static_tile(tile)
            }
        }
        Area::Dungeon { .. } => 0,
    }
}

/// A small deterministic generator for the corpora (xorshift32).
struct GemCorpusRng(u32);

impl GemCorpusRng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
}

fn gem_test_object(tile: u8, x: usize, y: usize, z: i8) -> ActiveObject {
    ActiveObject {
        type_byte: tile,
        tile,
        x,
        y,
        z,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    }
}

/// Animated and plain terrain under the objects, so `resolve_static_tile`
/// has something to resolve.
fn gem_varied_grid(len: usize, seed: u32) -> Vec<u8> {
    let mut rng = GemCorpusRng(seed);
    (0..len).map(|_| (rng.next() & 0x7f) as u8).collect()
}

/// Fills the state's object slots past the party with objects whose tiles
/// (`0xa0..`) differ from every terrain tile of `gem_varied_grid`, at the
/// given cells, on the party's own floor or plane.
fn gem_place_objects(state: &mut PlayState, cells: &[(usize, usize)]) {
    let z = state.current_floor().expect("a surface floor");
    for (index, &(x, y)) in cells.iter().enumerate() {
        let tile = 0xa0 + (index % 0x20) as u8;
        state.active_objects.push(gem_test_object(tile, x, y, z));
    }
}

fn gem_assert_delegation(state: &PlayState, context: &str) {
    for cell_y in 0..LOCAL_VIEW_OVERLAY_SIDE {
        for cell_x in 0..LOCAL_VIEW_OVERLAY_SIDE {
            let expected = gem_legacy_surface_view_tile_at(state, cell_x, cell_y);
            assert_eq!(
                state.surface_view_tile_at(cell_x, cell_y),
                expected,
                "{context}: window cell ({cell_x}, {cell_y})"
            );
            let (x, y) = state.surface_view_cell_coordinate(cell_x, cell_y);
            assert_eq!(
                state.gem_sample_tile(x, y, false),
                expected,
                "{context}: gem_sample_tile at window cell ({cell_x}, {cell_y})"
            );
        }
    }
}

#[test]
fn gem_sample_tile_equals_the_faithful_lookup_on_every_town_window_cell() {
    // Party at the floor's corners and edges as well as inside it.
    for (px, py) in [(0, 0), (31, 31), (0, 31), (31, 0), (16, 16), (5, 9), (30, 1)] {
        let mut state = test_state(gem_varied_grid(32 * 32, 0x5eed_0001), px, py);
        let cells: Vec<_> = [(0, 0), (31, 31), (0, 31), (31, 0), (px, py), (6, 5), (17, 3)]
            .into_iter()
            .collect();
        gem_place_objects(&mut state, &cells);
        for frame in 0..3 {
            gem_assert_delegation(&state, &format!("town party ({px}, {py}) frame {frame}"));
            state.animation.frame = state.animation.frame.wrapping_add(1);
        }
    }
}

#[test]
fn gem_sample_tile_equals_the_faithful_lookup_on_every_world_window_cell() {
    // The window wraps at 256 on both axes around these positions.
    for (px, py) in [(0, 0), (255, 255), (0, 255), (255, 0), (128, 128), (3, 200), (250, 7)] {
        let mut state = world_state(gem_varied_grid(WORLD_SIDE * WORLD_SIDE, 0x5eed_0002), px, py);
        let mut cells = vec![(0, 0), (255, 255), (255, 0), (0, 255)];
        cells.push(((px + 1) % 256, py));
        cells.push(((px + 240) % 256, (py + 250) % 256));
        cells.push(((px + 15) % 256, (py + 15) % 256));
        gem_place_objects(&mut state, &cells);
        for frame in 0..3 {
            gem_assert_delegation(&state, &format!("world party ({px}, {py}) frame {frame}"));
            state.animation.frame = state.animation.frame.wrapping_add(1);
        }
    }
}

#[test]
fn gem_sample_tile_wraps_the_world_and_blanks_off_the_town_floor() {
    let state = world_state(gem_varied_grid(WORLD_SIDE * WORLD_SIDE, 0x5eed_0003), 10, 10);
    for (x, y) in [(0isize, 0isize), (255, 255), (17, 230)] {
        for terrain_only in [false, true] {
            let base = state.gem_sample_tile(x, y, terrain_only);
            assert_eq!(state.gem_sample_tile(x - 256, y, terrain_only), base);
            assert_eq!(state.gem_sample_tile(x + 256, y + 512, terrain_only), base);
            assert_eq!(state.gem_sample_tile(x, y - 256, terrain_only), base);
        }
    }

    let town = test_state(gem_varied_grid(32 * 32, 0x5eed_0004), 4, 4);
    for (x, y) in [(-1isize, 0isize), (0, -1), (32, 0), (0, 32), (-40, 99)] {
        assert_eq!(town.gem_sample_tile(x, y, false), 0);
        assert_eq!(town.gem_sample_tile(x, y, true), 0);
    }

    let dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    assert_eq!(dungeon.gem_sample_tile(1, 1, false), 0);
    assert_eq!(dungeon.gem_sample_tile(1, 1, true), 0);
}

#[test]
fn gem_sample_tile_terrain_only_never_returns_an_object_tile() {
    let mut town = test_state(gem_varied_grid(32 * 32, 0x5eed_0005), 8, 8);
    let town_cells = [(8, 8), (9, 8), (0, 0), (31, 31), (12, 20)];
    gem_place_objects(&mut town, &town_cells);
    let mut world = world_state(gem_varied_grid(WORLD_SIDE * WORLD_SIDE, 0x5eed_0006), 0, 0);
    let world_cells = [(0, 0), (1, 0), (255, 255), (0, 255), (40, 41)];
    gem_place_objects(&mut world, &world_cells);

    for (state, cells, label) in [
        (&town, &town_cells[..], "town"),
        (&world, &world_cells[..], "world"),
    ] {
        for frame in 0..4u32 {
            let mut state = state.clone();
            state.animation.frame = state.animation.frame.wrapping_add(frame as _);
            for (index, &(x, y)) in cells.iter().enumerate() {
                let object_tile = 0xa0 + index as u8;
                let (sx, sy) = (x as isize, y as isize);
                assert_eq!(
                    state.gem_sample_tile(sx, sy, false),
                    object_tile,
                    "{label} ({x}, {y}): the faithful lookup shows the object"
                );
                let index = match state.area {
                    Area::World { .. } => world_cell_index(x, y),
                    _ => y * 32 + x,
                };
                let terrain = state.animation.resolve_static_tile(state.grid[index]);
                let sampled = state.gem_sample_tile(sx, sy, true);
                assert_eq!(sampled, terrain, "{label} ({x}, {y}) frame {frame}");
                assert_ne!(sampled, object_tile, "{label} ({x}, {y})");
            }
            // Every other cell: terrain-only is the animated grid tile, and
            // equals the faithful lookup where no object stands.
            let side = if label == "town" { 32 } else { WORLD_SIDE };
            for y in (0..side).step_by(7) {
                for x in (0..side).step_by(5) {
                    let index = if label == "town" { y * 32 + x } else { world_cell_index(x, y) };
                    let terrain = state.animation.resolve_static_tile(state.grid[index]);
                    let (sx, sy) = (x as isize, y as isize);
                    assert_eq!(state.gem_sample_tile(sx, sy, true), terrain);
                    if !cells.contains(&(x, y)) {
                        assert_eq!(state.gem_sample_tile(sx, sy, false), terrain);
                    }
                }
            }
        }
    }
}

/// Dungeon bytes whose glyph is `None` but which let the flood through: a
/// plain corridor without its visit bit and the `0x7?` and `0x9?` classes.
fn gem_flood_passes_without_glyph(tile: u8) -> bool {
    dungeon_minimap_glyph(tile).is_none() && dungeon_minimap_expands(tile)
}

/// The wrapped level cells of the `Some` glyphs of the faithful flood.
fn gem_some_glyph_cells(state: &PlayState, level: u8) -> Vec<(u8, u8)> {
    let side = DUNGEON_GEM_VIEW_GRID_SIDE;
    let (party_x, party_y) = DUNGEON_GEM_VIEW_PARTY_CELL;
    let glyphs = state.dungeon_vision_glyphs(level);
    let mut cells = Vec::new();
    for scratch_y in 0..side {
        for scratch_x in 0..side {
            if glyphs[scratch_y * side + scratch_x].is_none() {
                continue;
            }
            let dx = scratch_x as isize - party_x as isize;
            let dy = scratch_y as isize - party_y as isize;
            let x = (state.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as u8;
            let y = (state.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as u8;
            cells.push((x, y));
        }
    }
    cells.sort_unstable();
    cells.dedup();
    cells
}

fn gem_open_dungeon_view(state: &mut PlayState, level: u8) {
    state.gems = 1;
    assert_eq!(state.view_gem(), MoveOutcome::Observed);
    assert_eq!(
        state.active_view_overlay.as_ref().map(|overlay| overlay.kind),
        Some(ViewOverlayKind::Dungeon { level })
    );
}

fn gem_dungeon_footprint(state: &PlayState) -> (u8, Vec<(u8, u8)>) {
    match state.view_overlay_footprint() {
        Some(ViewOverlayFootprint::Dungeon { level, cells }) => (level, cells),
        other => panic!("expected a dungeon footprint, got {other:?}"),
    }
}

/// Checks the footprint against the faithful glyphs at every cell of every
/// level of one dungeon record.
///
/// A flood cell whose byte has no glyph (a plain corridor) is `None` in the
/// glyphs just like an unflooded cell, so the black-box check repaints, on a
/// clone, every such byte as `0x10`: it has a glyph and expands exactly the
/// same, so the flood cannot change, and on the repainted record the
/// footprint must equal the `Some` cells exactly. On the original record the
/// footprint is the same set, and holds every `Some` cell.
fn gem_assert_footprint_matches_glyphs(record: &[u8], context: &str) {
    assert_eq!(record.len(), DUNGEON_RECORD_LEN);
    let mut repainted = record.to_vec();
    for byte in &mut repainted {
        if gem_flood_passes_without_glyph(*byte) {
            *byte = 0x10;
        }
    }
    assert!(dungeon_minimap_glyph(0x10).is_some() && dungeon_minimap_expands(0x10));
    for level in 0..DUNGEON_LEVELS_PER_RECORD as u8 {
        for y in 0..DUNGEON_SIDE {
            for x in 0..DUNGEON_SIDE {
                let mut state = dungeon_state(record.to_vec(), level, x, y);
                gem_open_dungeon_view(&mut state, level);
                let (footprint_level, footprint) = gem_dungeon_footprint(&state);
                assert_eq!(footprint_level, level);
                assert!(footprint.contains(&(x as u8, y as u8)), "{context}: party cell");
                let some_cells = gem_some_glyph_cells(&state, level);
                assert!(
                    some_cells.iter().all(|cell| footprint.contains(cell)),
                    "{context} level {level} ({x}, {y}): a Some glyph cell is outside the footprint"
                );

                let mut painted = dungeon_state(repainted.clone(), level, x, y);
                gem_open_dungeon_view(&mut painted, level);
                let (_, painted_footprint) = gem_dungeon_footprint(&painted);
                assert_eq!(
                    painted_footprint, footprint,
                    "{context} level {level} ({x}, {y}): repainting changed the flood"
                );
                assert_eq!(
                    painted_footprint,
                    gem_some_glyph_cells(&painted, level),
                    "{context} level {level} ({x}, {y}): footprint != Some glyph cells"
                );
            }
        }
    }
}

/// Synthetic dungeon records: eight of them, drawn from every class
/// nibble with walls weighted up so floods stop at varied places.
fn gem_synthetic_dungeon_records() -> Vec<Vec<u8>> {
    let mut rng = GemCorpusRng(0x0d0e_5eed);
    (0..8)
        .map(|record| {
            (0..DUNGEON_RECORD_LEN)
                .map(|_| match rng.below(10 + record) {
                    0..=2 => 0x00,
                    3 => (rng.next() & 0xff) as u8,
                    4 | 5 => 0xb0 | (rng.next() & 0x0f) as u8,
                    6 => 0xc0,
                    7 => 0xd0,
                    _ => [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xe0, 0xf0]
                        [rng.below(12)],
                })
                .collect()
        })
        .collect()
}

#[test]
fn view_overlay_footprint_equals_the_flood_of_the_faithful_glyphs() {
    for (index, record) in gem_synthetic_dungeon_records().iter().enumerate() {
        gem_assert_footprint_matches_glyphs(record, &format!("synthetic dungeon {index}"));
    }
}

#[test]
fn view_overlay_footprint_equals_the_flood_in_all_eight_shipped_dungeons() {
    // Asset-backed: skipped without `U5_TEST_ASSET_DIR`, like every test
    // that reads original game data. Nothing of the file is kept.
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Ok(bytes) = fs::read(dir.join("DUNGEON.DAT")) else {
        return;
    };
    assert_eq!(bytes.len(), DUNGEON_DAT_RECORD_COUNT * DUNGEON_RECORD_LEN);
    for (index, record) in bytes.chunks(DUNGEON_RECORD_LEN).enumerate() {
        gem_assert_footprint_matches_glyphs(record, &format!("dungeon record {index}"));
    }
}

#[test]
fn view_overlay_footprint_covers_the_whole_level_when_the_flood_fills_the_grid() {
    // An all-corridor level floods the whole 22 x 22 scratch grid, the
    // frontier-cap case (`dungeon-mode.md §12.2`), and so wraps onto every
    // cell of the level from any position.
    for (x, y) in [(0, 0), (7, 7), (3, 4)] {
        let mut state = dungeon_state(vec![0x00; DUNGEON_RECORD_LEN], 2, x, y);
        gem_open_dungeon_view(&mut state, 2);
        let (level, cells) = gem_dungeon_footprint(&state);
        assert_eq!(level, 2);
        assert_eq!(cells.len(), DUNGEON_SIDE * DUNGEON_SIDE);
    }
    // Walled in: the party cell and its eight neighbours only.
    let mut grid = vec![0xb0; DUNGEON_RECORD_LEN];
    grid[dungeon_cell_index(0, 0, 0)] = 0x00;
    let mut state = dungeon_state(grid, 0, 0, 0);
    gem_open_dungeon_view(&mut state, 0);
    let (_, cells) = gem_dungeon_footprint(&state);
    let mut expected = Vec::new();
    for dy in [-1isize, 0, 1] {
        for dx in [-1isize, 0, 1] {
            expected.push((dx.rem_euclid(8) as u8, dy.rem_euclid(8) as u8));
        }
    }
    expected.sort_unstable();
    assert_eq!(cells, expected);
}

#[test]
fn view_overlay_footprint_of_surface_views_is_the_sampled_window() {
    // Town: the whole floor, wherever the party stands.
    let mut town = test_state(open_grid(), 3, 29);
    town.gems = 1;
    assert_eq!(town.view_gem(), MoveOutcome::Observed);
    let Some(ViewOverlayFootprint::Surface { cells }) = town.view_overlay_footprint() else {
        panic!("town footprint");
    };
    assert_eq!(cells.len(), 32 * 32);
    assert!(cells.iter().all(|&(x, y)| x < 32 && y < 32));

    // World: the 32 x 32 window centred on the party, wrapped at 256.
    let mut world = world_state(open_world_grid(), 0, 255);
    world.gems = 1;
    assert_eq!(world.view_gem(), MoveOutcome::Observed);
    let Some(ViewOverlayFootprint::Surface { cells }) = world.view_overlay_footprint() else {
        panic!("world footprint");
    };
    let mut expected = Vec::new();
    for dy in -16isize..16 {
        for dx in -16isize..16 {
            expected.push((dx.rem_euclid(256) as u8, (255 + dy).rem_euclid(256) as u8));
        }
    }
    expected.sort_unstable();
    assert_eq!(cells, expected);

    // No overlay, no footprint.
    world.clear_active_view_overlay();
    assert_eq!(world.view_overlay_footprint(), None);
}

#[test]
fn gem_runtime_readers_are_pure_including_the_prng() {
    let mut world = world_state(gem_varied_grid(WORLD_SIDE * WORLD_SIDE, 0x5eed_0007), 9, 250);
    gem_place_objects(&mut world, &[(9, 251), (10, 250)]);
    world.gems = 1;
    assert_eq!(world.view_gem(), MoveOutcome::Observed);
    let mut town = test_state(gem_varied_grid(32 * 32, 0x5eed_0008), 7, 7);
    gem_place_objects(&mut town, &[(7, 8)]);
    town.gems = 1;
    assert_eq!(town.view_gem(), MoveOutcome::Observed);
    let mut dungeon = dungeon_state(gem_synthetic_dungeon_records()[3].clone(), 4, 2, 6);
    gem_open_dungeon_view(&mut dungeon, 4);

    for mut state in [world, town, dungeon] {
        state.prng_state = 0x1234;
        let before = state.clone();
        let mut results = Vec::new();
        for call in 0..100isize {
            results.push(state.gem_sample_tile(call - 50, call * 3, false));
            results.push(state.gem_sample_tile(call * 7, call - 20, true));
            results.push(u8::from(state.view_overlay_footprint().is_some()));
        }
        assert_eq!(state.prng_state, before.prng_state);
        assert_eq!(
            crate::enhancements::trajectory_digest(&state),
            crate::enhancements::trajectory_digest(&before)
        );
        assert_eq!(format!("{state:?}"), format!("{before:?}"));
        // Every repeat answered the same.
        let first = state.view_overlay_footprint();
        for _ in 0..100 {
            assert_eq!(state.view_overlay_footprint(), first);
        }
        assert!(!results.is_empty());
    }
}

#[test]
fn gem_view_painters_are_public_and_paint_what_the_overlays_paint() {
    // Through the crate root, as the automap and gem map crates call them.
    for (class, tile, marker) in [(0x01u8, 0x05u8, false), (0x0f, 0xdc, false), (0x00, 0x00, true)] {
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let mut viewport = TileViewport {
            depth: TileGraphicsDepth::Ega16,
            cells_wide: 1,
            cells_high: 1,
            width: scale,
            height: scale,
            pixels: vec![0; scale * scale],
        };
        crate::draw_surface_view_cell(
            &mut viewport,
            0,
            0,
            scale,
            class,
            tile,
            marker,
            ViewOverlayMode::GemView,
        );
        let reference = PlayState::render_surface_view_class_cell_for_mode(
            TileGraphicsDepth::Ega16,
            class,
            tile,
            marker,
            ViewOverlayMode::GemView,
        );
        assert_eq!(viewport.pixels, reference.pixels);
    }
    for tile in [0x08u8, 0x10, 0x50, 0x68, 0x80, 0xb0, 0xe0] {
        let glyph = dungeon_minimap_glyph(tile);
        let scale = DUNGEON_GEM_VIEW_CELL_PIXELS;
        let mut viewport = TileViewport {
            depth: TileGraphicsDepth::Ega16,
            cells_wide: 1,
            cells_high: 1,
            width: scale,
            height: scale,
            pixels: vec![0; scale * scale],
        };
        crate::draw_dungeon_view_glyph(&mut viewport, 0, 0, scale, glyph, None);
        let reference = PlayState::render_dungeon_view_glyph_cell_for_mode(
            TileGraphicsDepth::Ega16,
            glyph,
            ViewOverlayMode::GemView,
        );
        assert_eq!(viewport.pixels, reference.pixels, "tile {tile:#04x}");
    }
}
