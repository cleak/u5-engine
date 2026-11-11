// Phase B M2-07: ring compositing, P5b (design 9.4.4, D9.11): a sprite
// composited in the ring equals the same sprite on the same terrain in the
// core, hidden ring cells are unchanged, the same eligibility applies, off
// is M1's raster, and each ring `under` is the tile the cell shows empty.
mod ring_composite {
    use super::*;
    use crate::enhancements::sprite_mask::{KEEP_THE_BOX, SpriteMasks, mask_bit};

    const WIDE: PeekWindow = PeekWindow {
        west: 15,
        east: 15,
        north: 7,
        south: 7,
    };

    const PERSON: u8 = 0x5A;
    const SITTER: u8 = 0x44;
    const CHAIR_FACING_DOWN: u8 = 0x92;
    const TOWN_WALL: u8 = 0x4E;
    const TRAPPED_SOUL_ROW: u8 = 0x9D;
    const GRASS: u8 = 0x05;

    /// The synthetic atlas with every actor-half tile drawn as an 8x8
    /// coloured square on a black box. Terrain tiles stay solid
    /// (`id % 16`), so grass (5) shows through a composited box.
    fn boxed_atlas() -> TileAtlas {
        let mut atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        for tile in ACTOR_TILE_BANK_BASE..TILE_ATLAS_TILE_COUNT {
            let colour = (tile % 15) as u8 + 1;
            let base = tile * TILE_ATLAS_TILE_PIXELS;
            for y in 0..TILE_ATLAS_SIDE {
                for x in 0..TILE_ATLAS_SIDE {
                    let inside = (4..12).contains(&x) && (4..12).contains(&y);
                    atlas.pixels[base + y * TILE_ATLAS_SIDE + x] = if inside { colour } else { 0 };
                }
            }
        }
        atlas
    }

    fn object(tile: u8, x: usize, y: usize, z: i8) -> ActiveObject {
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

    fn with_objects(mut state: PlayState, objects: &[ActiveObject]) -> PlayState {
        state.active_objects.truncate(1);
        state.active_objects.extend_from_slice(objects);
        state
    }

    fn town_at_noon(grid: Vec<u8>) -> PlayState {
        let mut state = test_state(grid, 16, 16);
        state.ambient_light = FULL_DAYLIGHT;
        state.rebuild_surface_local_light_mask();
        state
    }

    /// The faithful render, as the shell draws it before the peeks.
    fn render(state: &mut PlayState, atlas: &TileAtlas) -> TileViewport {
        state
            .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, atlas)
            .unwrap()
            .unwrap()
    }

    /// The 16x16 palette indices of cell `(cell_x, cell_y)` of `viewport`.
    fn cell_pixels(viewport: &TileViewport, cell_x: usize, cell_y: usize) -> Vec<u8> {
        (0..TILE_ATLAS_TILE_PIXELS)
            .map(|i| {
                let x = cell_x * TILE_ATLAS_SIDE + i % TILE_ATLAS_SIDE;
                let y = cell_y * TILE_ATLAS_SIDE + i / TILE_ATLAS_SIDE;
                viewport.pixels[y * viewport.width + x]
            })
            .collect()
    }

    /// The window cell of offset `(dx, dy)`.
    fn ring_cell(dx: isize, dy: isize) -> (usize, usize) {
        let index = WIDE.index_of(dx, dy).unwrap();
        (index % WIDE.columns(), index / WIDE.columns())
    }

    /// The core composite (design 9.3.8) on the faithful palette viewport:
    /// each peeked cell's mask pixels that are still 0 take `under`.
    fn composite_core(viewport: &mut TileViewport, peek: &ObjectCellPeek, masks: &SpriteMasks, halo: bool) {
        for cell in &peek.cells {
            let mask = masks.background(usize::from(cell.drawn), halo);
            for y in 0..TILE_ATLAS_SIDE {
                for x in 0..TILE_ATLAS_SIDE {
                    let at = (usize::from(cell.row) * TILE_ATLAS_SIDE + y) * viewport.width
                        + usize::from(cell.col) * TILE_ATLAS_SIDE
                        + x;
                    if mask_bit(mask, x, y) && viewport.pixels[at] == 0 {
                        viewport.pixels[at] = cell.under_pixels[y * TILE_ATLAS_SIDE + x];
                    }
                }
            }
        }
    }

    #[test]
    fn a_ring_sprite_equals_the_same_sprite_on_the_same_terrain_in_the_core() {
        let atlas = boxed_atlas();
        let masks = SpriteMasks::from_atlas(&atlas);
        let mut state = with_objects(
            town_at_noon(vec![GRASS; 1024]),
            &[object(PERSON, 16 + 2, 16, 0), object(PERSON, 16 + 8, 16 - 3, 0)],
        );
        let faithful = render(&mut state, &atlas);
        let peek = state.peek_object_cells(&atlas, &masks).unwrap();
        let ring = state.peek_top_down_ring(WIDE, PeekLightPolicy::Scaled).unwrap();
        let (rx, ry) = ring_cell(8, -3);
        let plain = state.render_peek_ring(&ring, &atlas).unwrap();
        for halo in [true, false] {
            let mut core = faithful.clone();
            composite_core(&mut core, &peek, &masks, halo);
            let raster = state
                .render_peek_ring_composited(&ring, &atlas, Some((&masks, halo)))
                .unwrap();
            let in_core = cell_pixels(&core, 5 + 2, 5);
            assert_ne!(in_core, cell_pixels(&faithful, 7, 5), "the core composites");
            assert_eq!(cell_pixels(&raster, rx, ry), in_core, "halo {halo}");
            assert_ne!(cell_pixels(&raster, rx, ry), cell_pixels(&plain, rx, ry));
        }
    }

    #[test]
    fn hidden_ring_cells_are_unchanged() {
        let atlas = boxed_atlas();
        let masks = SpriteMasks::from_atlas(&atlas);
        let mut grid = vec![GRASS; 1024];
        // A wall down the whole town at dx = 5 hides everything east of it.
        for y in 0..32 {
            grid[y * 32 + 21] = TOWN_WALL;
        }
        let mut state = with_objects(
            town_at_noon(grid),
            &[object(PERSON, 16 + 9, 16, 0), object(PERSON, 16 - 8, 16 + 2, 0)],
        );
        render(&mut state, &atlas);
        let ring = state.peek_top_down_ring(WIDE, PeekLightPolicy::Scaled).unwrap();
        assert_eq!(ring.cells[WIDE.index_of(9, 0).unwrap()], Some(PeekCell::HIDDEN));
        let plain = state.render_peek_ring(&ring, &atlas).unwrap();
        let raster = state
            .render_peek_ring_composited(&ring, &atlas, Some((&masks, true)))
            .unwrap();
        let mut composited = 0;
        for (index, cell) in ring.cells.iter().enumerate() {
            let (cx, cy) = (index % WIDE.columns(), index / WIDE.columns());
            let (before, after) = (cell_pixels(&plain, cx, cy), cell_pixels(&raster, cx, cy));
            match cell {
                Some(PeekCell {
                    shade: PeekShade::Lit,
                    under: Some(_),
                    atlas: Some(id),
                    ..
                }) => {
                    let mask = masks.background(usize::from(*id), true);
                    for i in 0..TILE_ATLAS_TILE_PIXELS {
                        let (x, y) = (i % TILE_ATLAS_SIDE, i / TILE_ATLAS_SIDE);
                        if before[i] != after[i] {
                            assert!(mask_bit(mask, x, y) && before[i] == 0, "cell {index} ({x}, {y})");
                        }
                    }
                    composited += usize::from(before != after);
                }
                Some(PeekCell {
                    shade: PeekShade::Hidden,
                    ..
                }) => {
                    assert_eq!(after, before, "hidden cell {index}");
                    assert!(after.iter().all(|&p| p == 0), "hidden cell {index} is black");
                }
                _ => assert_eq!(after, before, "cell {index}"),
            }
        }
        assert_eq!(composited, 1, "the lit person only");
    }

    #[test]
    fn the_ring_applies_the_same_eligibility() {
        let atlas = boxed_atlas();
        let masks = SpriteMasks::from_atlas(&atlas);
        let mut grid = vec![GRASS; 1024];
        let chair = (16 + 9, 16 + 2);
        grid[chair.1 * 32 + chair.0] = CHAIR_FACING_DOWN;
        let listed = (KEEP_THE_BOX[0] - ACTOR_TILE_BANK_BASE) as u8;
        let mut state = with_objects(
            town_at_noon(grid),
            &[
                object(SITTER, chair.0, chair.1, 0),
                object(listed, 16 - 9, 16 - 2, 0),
                object(PERSON, 16 + 9, 16 - 4, 0),
            ],
        );
        render(&mut state, &atlas);
        let ring = state.peek_top_down_ring(WIDE, PeekLightPolicy::Scaled).unwrap();
        let plain = state.render_peek_ring(&ring, &atlas).unwrap();
        let raster = state
            .render_peek_ring_composited(&ring, &atlas, Some((&masks, false)))
            .unwrap();
        let same = |dx: isize, dy: isize| {
            let (cx, cy) = ring_cell(dx, dy);
            cell_pixels(&plain, cx, cy) == cell_pixels(&raster, cx, cy)
        };
        let under = |dx: isize, dy: isize| ring.cells[WIDE.index_of(dx, dy).unwrap()].unwrap().under;
        // The seated actor keeps its box: the chair is furniture.
        assert_eq!(under(9, 2), Some(CHAIR_FACING_DOWN));
        assert!(same(9, 2), "furniture keeps its box");
        // The listed tile keeps its box.
        assert_eq!(under(-9, -2), Some(GRASS));
        assert!(same(-9, -2), "a listed tile keeps its box");
        // An ordinary person on grass composites.
        assert_eq!(under(9, -4), Some(GRASS));
        assert!(!same(9, -4), "a person composites");
    }

    #[test]
    fn with_compositing_off_the_ring_raster_is_m1s() {
        let atlas = boxed_atlas();
        let mut state = with_objects(
            town_at_noon(vec![GRASS; 1024]),
            &[object(PERSON, 16 + 8, 16, 0), object(PERSON, 16 - 12, 16 + 6, 0)],
        );
        render(&mut state, &atlas);
        let ring = state.peek_top_down_ring(WIDE, PeekLightPolicy::Scaled).unwrap();
        let off = state.render_peek_ring_composited(&ring, &atlas, None).unwrap();
        assert_eq!(off, state.render_peek_ring(&ring, &atlas).unwrap());
        // M1's raster: each lit cell is its atlas tile as the blit draws it,
        // black box included; everything else is black.
        for (index, cell) in ring.cells.iter().enumerate() {
            let (cx, cy) = (index % WIDE.columns(), index / WIDE.columns());
            let expected = match cell {
                Some(PeekCell {
                    shade: PeekShade::Lit,
                    atlas: Some(id),
                    ..
                }) => atlas.tile_pixels(usize::from(*id)).unwrap().to_vec(),
                _ => vec![0; TILE_ATLAS_TILE_PIXELS],
            };
            assert_eq!(cell_pixels(&off, cx, cy), expected, "cell {index}");
        }
        let (cx, cy) = ring_cell(8, 0);
        assert_eq!(cell_pixels(&off, cx, cy)[0], 0, "the box stays black");
    }

    /// Spoiler (9.6): each ring `under` is exactly the tile that ring cell
    /// shows on the same frame with the objects gone, so compositing never
    /// shows terrain the empty ring would not.
    #[test]
    fn each_ring_under_is_the_tile_the_cell_shows_when_empty() {
        let atlas = boxed_atlas();
        let water = (0..=u8::MAX)
            .find(|&tile| water_pass_rotates_tile(tile))
            .expect("a rotated water id");
        let mut grid = vec![GRASS; 1024];
        grid[(16 + 3) * 32 + 16 + 11] = water;
        grid[(16 - 5) * 32 + 16 - 10] = 0x10;
        grid[(16 + 6) * 32 + 16 + 7] = TRAPPED_SOUL_ROW;
        let objects = [
            object(PERSON, 16 + 11, 16 + 3, 0),
            object(PERSON, 16 - 10, 16 - 5, 0),
            object(PERSON, 16 + 7, 16 + 7, 0),
            object(PERSON + 1, 16 - 14, 16 + 7, 0),
            object(PERSON, 16 + 3, 16 - 7, 0),
        ];
        let mut busy = with_objects(town_at_noon(grid.clone()), &objects);
        render(&mut busy, &atlas);
        let mut empty = with_objects(town_at_noon(grid), &[]);
        render(&mut empty, &atlas);
        let worlds = {
            let mut world = world_state(open_world_grid(), 100, 100);
            world.ambient_light = FULL_DAYLIGHT;
            let z = world.current_floor().unwrap();
            let mut busy_world = with_objects(
                world.clone(),
                &[object(PERSON, 100 + 12, 100 - 2, z), object(PERSON, 100 - 7, 100 + 7, z)],
            );
            render(&mut busy_world, &atlas);
            let mut empty_world = with_objects(world, &[]);
            render(&mut empty_world, &atlas);
            (busy_world, empty_world)
        };
        for (label, busy, empty) in [("town", &busy, &empty), ("world", &worlds.0, &worlds.1)] {
            for light in [PeekLightPolicy::Scaled, PeekLightPolicy::Faithful] {
                let ring = busy.peek_top_down_ring(WIDE, light).unwrap();
                let bare = empty.peek_top_down_ring(WIDE, light).unwrap();
                let mut seen = 0;
                for (index, cell) in ring.cells.iter().enumerate() {
                    let Some(cell) = cell else {
                        continue;
                    };
                    let Some(under) = cell.under else {
                        continue;
                    };
                    seen += 1;
                    assert!(cell.atlas.unwrap() >= ACTOR_TILE_BANK_BASE as u16, "{label} {index}");
                    assert_eq!(
                        bare.cells[index],
                        Some(PeekCell::lit_terrain(under)),
                        "{label} {light:?} cell {index}"
                    );
                }
                if light == PeekLightPolicy::Scaled {
                    assert!(seen >= 2, "{label}: objects in the ring carry an under");
                }
                // Terrain, marker and hidden cells carry none.
                for cell in ring.cells.iter().flatten() {
                    if cell.atlas.is_none_or(|id| id < ACTOR_TILE_BANK_BASE as u16) {
                        assert_eq!(cell.under, None, "{label}");
                    }
                }
            }
        }
        // The trapped-soul row (an actor under a 0x9D cell): the stamped
        // cell has an under, the cell above it, marked with a direct tile,
        // has none.
        let ring = busy.peek_top_down_ring(WIDE, PeekLightPolicy::Scaled).unwrap();
        let at = |dx, dy| ring.cells[WIDE.index_of(dx, dy).unwrap()].unwrap();
        assert_eq!(at(7, 7).under, Some(GRASS));
        assert_eq!(at(7, 6).atlas, Some(0x9E));
        assert_eq!(at(7, 6).under, None);
        // Water under a ring object animates the ring.
        assert_eq!(at(11, 3).under, Some(busy.animation.resolve_static_tile(water)));
        assert!(ring.animated);
    }
}
