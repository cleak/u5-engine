//! M3-12 town tests (10.2.5, 10.2.3 town rows): the floor at 7 or 6 px,
//! centred, terrain only; the land beside it from the static world map, cut
//! at the gazetteer entrance column, only where the record has seen it,
//! dimmed to 55% of the map; the strips are the floor's south-east corner
//! cell; `town_surroundings` off leaves the sides black.

use u5_runtime::enhancements::{BrightnessLut, GemMapToggles, trajectory_digest};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::{
    ActiveObject, Area, PlayState, Scene, TileGraphicsDepth, ViewOverlay, ViewOverlayKind,
    ViewOverlayMode, WORLD_SIDE,
};

use super::*;
use crate::gem::knowledge::{KnowEvent, SIDE_WINDOW_HEIGHT, SIDE_WINDOW_WIDTH};
use crate::gem::raster::{
    GemImage, GemRender, MapArt, PARTY_YELLOW, TOWN_SIDE_PERCENT, render_gem_map, town_side_lut,
};
use crate::gem::seen::{SideWindow, frame_seen_events, gem_view_events};

const W169: GemCanvas = GemCanvas { w: 512, h: 240 };
const W43: GemCanvas = GemCanvas { w: 320, h: 200 };
const W219: GemCanvas = GemCanvas { w: 688, h: 240 };
const CANVASES: [GemCanvas; 5] = [
    W43,
    W169,
    W219,
    GemCanvas { w: 480, h: 216 },
    GemCanvas { w: 688, h: 216 },
];

/// A tile only an object puts on a cell (a creature); never in the grid.
const CREATURE: u8 = 0x90;
/// The floor's south-east corner tile in these tests.
const CORNER: u8 = 0x44;

fn britain() -> Scene {
    Scene::from_key("TOWNE:1").unwrap()
}

fn window() -> SideWindow {
    *side_window(britain().byte).expect("Britain has one gazetteer entrance")
}

fn gem_overlay() -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Surface,
        mode: ViewOverlayMode::GemView,
    })
}

/// Britain's floor 0 with the gem view up, the party at (15, 17), a floor
/// of distinct terrain and a marked corner cell.
fn in_britain() -> PlayState {
    let mut grid = open_grid();
    for (index, cell) in grid.iter_mut().enumerate() {
        *cell = 0x10 + (index % 7) as u8;
    }
    grid[31 * 32 + 31] = CORNER;
    let mut state = test_state(grid, 15, 17);
    state.area = Area::Town {
        scene: britain(),
        floor: 0,
    };
    state.active_view_overlay = gem_overlay();
    state
}

/// A world map whose every cell names its position.
fn world_map() -> Vec<u8> {
    (0..WORLD_SIDE * WORLD_SIDE)
        .map(|index| {
            let (x, y) = (index % WORLD_SIDE, index / WORLD_SIDE);
            ((x * 7 + y * 13) % 251) as u8
        })
        .collect()
}

fn world_tile(world: &[u8], x: u8, y: u8) -> u8 {
    world[usize::from(y) * WORLD_SIDE + usize::from(x)]
}

/// A record that has seen every cell of Britain's side window.
fn seen_all() -> GemKnowledge {
    let mut know = GemKnowledge::new();
    let cells = u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT);
    for index in 0..cells {
        know.apply(&KnowEvent::SideCellSeen {
            scene: britain().byte,
            index,
        });
    }
    know
}

fn inputs<'a>(know: &'a GemKnowledge, world: &'a [u8]) -> TownInputs<'a> {
    TownInputs {
        know: Some(know),
        world: Some(world),
        surroundings: true,
    }
}

fn town(state: &PlayState, canvas: GemCanvas, town: TownInputs<'_>) -> TownShown {
    match GemViewState::open_with(state, canvas, 100, town).shown {
        GemShown::Town(shown) => shown,
        other => panic!("expected a town shown set, got {other:?}"),
    }
}

/// The world cell beside floor cell `(fx, fy)`, or `None` on the floor's
/// own columns.
fn beside(fx: i32, fy: i32) -> Option<(u8, u8)> {
    let (ex, ey) = window().entrance;
    let dx = TownLayout::entrance_dx(fx)?;
    let wrap = |v: i32| v.rem_euclid(WORLD_SIDE as i32) as u8;
    Some((
        wrap(i32::from(ex) + dx),
        wrap(i32::from(ey) + fy - TOWN_ENTRANCE_FLOOR_ROW),
    ))
}

fn side_cells(shown: &TownShown) -> Vec<(i32, i32, Option<u8>)> {
    shown
        .cells()
        .filter(|(_, _, part, _)| *part == TownPart::Side)
        .map(|(fx, fy, _, tile)| (fx, fy, tile))
        .collect()
}

// ---- layout ----------------------------------------------------------------

#[test]
fn the_floor_is_centred_at_the_largest_whole_size() {
    let expected = [
        (W43, 6, (64, 4)),
        (W169, 7, (144, 8)),
        (W219, 7, (232, 8)),
        (GemCanvas { w: 480, h: 216 }, 6, (144, 12)),
        (GemCanvas { w: 688, h: 216 }, 6, (248, 12)),
    ];
    for (canvas, cell, floor_px) in expected {
        let layout = TownLayout::new(canvas).unwrap();
        assert_eq!(
            (layout.cell, layout.floor_px),
            (cell, floor_px),
            "{canvas:?}"
        );
        let side = 32 * cell as i32;
        assert_eq!(2 * floor_px.0 + side, canvas.w as i32, "{canvas:?}");
        assert_eq!(2 * floor_px.1 + side, canvas.h as i32, "{canvas:?}");
    }
    assert_eq!(TownLayout::new(GemCanvas { w: 31, h: 240 }), None);
}

#[test]
fn britain_side_widths_at_16_9_and_4_3() {
    // The approved mockup's report: 21 and 20 cells at 16:9, 11 and 10 at
    // 4:3 (the west side counts its partial column).
    let wide = TownLayout::new(W169).unwrap();
    assert_eq!(wide.side_cells(), (21, 20));
    assert_eq!(wide.drawn_sides(), (21, 21));
    let classic = TownLayout::new(W43).unwrap();
    assert_eq!(classic.side_cells(), (11, 10));
    assert_eq!(classic.drawn_sides(), (11, 11));
    // At 240 lines the canvas rows are the side window's 36 rows.
    assert_eq!(wide.rows, (-2, 33));
    assert_eq!(
        (wide.rows.1 - wide.rows.0 + 1) as u8,
        SIDE_WINDOW_HEIGHT,
        "36 rows"
    );
    assert_eq!(classic.rows, (-1, 32));
}

#[test]
fn every_canvas_pixel_is_in_exactly_one_cell() {
    for canvas in CANVASES {
        let layout = TownLayout::new(canvas).unwrap();
        let mut hits = vec![0u8; canvas.pixels()];
        for fy in layout.rows.0..=layout.rows.1 {
            for fx in layout.cols.0..=layout.cols.1 {
                let (x0, y0, w, h) = layout.cell_rect(fx, fy);
                for y in y0..y0 + h as i32 {
                    for x in x0..x0 + w as i32 {
                        if (0..canvas.w as i32).contains(&x) && (0..canvas.h as i32).contains(&y) {
                            hits[y as usize * canvas.w as usize + x as usize] += 1;
                        }
                    }
                }
            }
        }
        assert!(hits.iter().all(|&n| n == 1), "{canvas:?}");
    }
}

// ---- the shown set ---------------------------------------------------------

#[test]
fn the_floor_is_the_live_terrain_of_every_cell() {
    let state = in_britain();
    let shown = town(&state, W169, TownInputs::default());
    assert_eq!(
        (shown.scene, shown.floor, shown.party),
        (britain().byte, 0, (15, 17))
    );
    let mut floor = 0;
    for (fx, fy, part, tile) in shown.cells() {
        if part == TownPart::Floor {
            floor += 1;
            let expected = state.gem_sample_tile(fx as isize, fy as isize, true);
            assert_eq!(tile, Some(expected), "{fx},{fy}");
        }
    }
    assert_eq!(floor, 32 * 32);
}

#[test]
fn britain_sides_are_the_land_around_the_entrance_cut_at_its_column() {
    let state = in_britain();
    let (world, know) = (world_map(), seen_all());
    let (ex, ey) = window().entrance;
    for canvas in [W169, W43] {
        let shown = town(&state, canvas, inputs(&know, &world));
        let sides = side_cells(&shown);
        assert!(!sides.is_empty());
        for (fx, fy, tile) in sides {
            let (x, y) = beside(fx, fy).unwrap();
            assert_ne!(x, ex, "the entrance column is never drawn ({fx},{fy})");
            assert_eq!(tile, Some(world_tile(&world, x, y)), "{canvas:?} {fx},{fy}");
        }
        // The split: the column touching the floor's west edge is the one
        // west of the entrance, the east edge's the one east of it, and
        // the entrance row is beside floor row 16.
        assert_eq!(shown.tile(-1, 16), Some(world_tile(&world, ex - 1, ey)));
        assert_eq!(shown.tile(32, 16), Some(world_tile(&world, ex + 1, ey)));
        let (west, east) = shown.layout.drawn_sides();
        let far_west = -(west as i32);
        let far_east = 31 + east as i32;
        assert_eq!(
            shown.tile(far_west, 16),
            Some(world_tile(&world, ex - west as u8, ey))
        );
        assert_eq!(
            shown.tile(far_east, 16),
            Some(world_tile(&world, ex + east as u8, ey))
        );
    }
}

#[test]
fn past_the_side_window_the_land_is_black() {
    // 21:9 shows 34 columns a side; the record keeps 24.
    let state = in_britain();
    let (world, know) = (world_map(), seen_all());
    let shown = town(&state, W219, inputs(&know, &world));
    for (fx, fy, tile) in side_cells(&shown) {
        let dx = TownLayout::entrance_dx(fx).unwrap();
        assert_eq!(tile.is_some(), dx.abs() <= 24, "{fx},{fy}");
    }
}

#[test]
fn the_strips_equal_the_corner_cell() {
    let state = in_britain();
    assert_eq!(state.gem_sample_tile(31, 31, true), CORNER);
    let (world, know) = (world_map(), seen_all());
    for canvas in CANVASES {
        for surroundings in [true, false] {
            let town_inputs = TownInputs {
                surroundings,
                ..inputs(&know, &world)
            };
            let shown = town(&state, canvas, town_inputs);
            assert_eq!(shown.corner, CORNER);
            let strips: Vec<_> = shown
                .cells()
                .filter(|(_, _, part, _)| *part == TownPart::Strip)
                .collect();
            assert_eq!(
                strips.len(),
                32 * (shown.layout.rows.1 - shown.layout.rows.0 - 31) as usize
            );
            assert!(
                strips.iter().all(|cell| cell.3 == Some(CORNER)),
                "{canvas:?}"
            );
        }
    }
}

#[test]
fn no_townsfolk_or_object_is_drawn_on_the_floor() {
    let mut state = in_britain();
    let z = state.active_objects[0].z;
    state.active_objects.push(ActiveObject {
        type_byte: CREATURE,
        tile: CREATURE,
        x: 10,
        y: 12,
        z,
        ..ActiveObject::empty()
    });
    assert_eq!(
        state.gem_sample_tile(10, 12, false),
        CREATURE,
        "the NPC is there"
    );
    let shown = town(&state, W169, TownInputs::default());
    assert_eq!(shown.tile(10, 12), Some(state.grid[12 * 32 + 10]));
    assert!(shown.cells().all(|cell| cell.3 != Some(CREATURE)));
    // Nor in the picture.
    let art = MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16));
    let view = GemViewState::open(&state, W169, 100);
    let mut out = GemImage::default();
    render_gem_map(&state, &GemKnowledge::new(), &art, &on(), &view, &mut out);
    let terrain = state.grid[12 * 32 + 10];
    let (x, y, _, _) = view_layout(&view).cell_rect(10, 12);
    assert_eq!(
        out.pixel(x as u32 + 3, y as u32 + 3).unwrap(),
        dimmed(&art, 7, terrain, &BrightnessLut::for_percent(70))
    );
}

#[test]
fn on_the_fresh_record_every_side_cell_is_black() {
    // A gem used in the starting town: the town itself records no side
    // cell, by frame or by gem, so the record stays empty.
    let state = in_britain();
    let mut know = GemKnowledge::new();
    for event in frame_seen_events(&state)
        .iter()
        .chain(gem_view_events(&state, None).iter())
    {
        know.apply(event);
    }
    assert!(know.is_empty());
    let world = world_map();
    for canvas in CANVASES {
        let shown = town(&state, canvas, inputs(&know, &world));
        let sides = side_cells(&shown);
        assert!(!sides.is_empty());
        assert!(sides.iter().all(|cell| cell.2.is_none()), "{canvas:?}");
    }
}

#[test]
fn only_the_seen_side_cells_are_drawn() {
    let state = in_britain();
    let world = world_map();
    let w = window();
    let (ex, ey) = w.entrance;
    let mut know = GemKnowledge::new();
    let seen = [(ex - 1, ey), (ex + 3, ey - 5), (ex - 20, ey + 10)];
    for (x, y) in seen {
        know.apply(&KnowEvent::SideCellSeen {
            scene: britain().byte,
            index: w.index_of(x, y).unwrap(),
        });
    }
    // Another town's bits draw nothing here.
    know.apply(&KnowEvent::SideCellSeen {
        scene: britain().byte + 1,
        index: w.index_of(ex - 2, ey).unwrap(),
    });
    let shown = town(&state, W169, inputs(&know, &world));
    let drawn: Vec<(u8, u8)> = side_cells(&shown)
        .into_iter()
        .filter(|cell| cell.2.is_some())
        .map(|(fx, fy, _)| beside(fx, fy).unwrap())
        .collect();
    assert_eq!(drawn.len(), seen.len());
    for cell in seen {
        assert!(drawn.contains(&cell), "{cell:?}");
    }
}

#[test]
fn with_town_surroundings_off_the_sides_are_black() {
    let state = in_britain();
    let (world, know) = (world_map(), seen_all());
    for canvas in CANVASES {
        let shown = town(
            &state,
            canvas,
            TownInputs {
                surroundings: false,
                ..inputs(&know, &world)
            },
        );
        assert!(side_cells(&shown).iter().all(|cell| cell.2.is_none()));
    }
}

#[test]
fn a_missing_world_map_or_a_town_with_no_window_leaves_black_sides() {
    let state = in_britain();
    let know = seen_all();
    for world in [None, Some(&[5u8; 100][..])] {
        let shown = town(
            &state,
            W169,
            TownInputs {
                know: Some(&know),
                world,
                surroundings: true,
            },
        );
        assert!(side_cells(&shown).iter().all(|cell| cell.2.is_none()));
    }
    // The plane to read is the window's.
    assert_eq!(town_world_plane(&state), Some(window().plane));
    let mut outside = state.clone();
    outside.area = Area::World {
        plane: window().plane,
    };
    assert_eq!(town_world_plane(&outside), None);
}

#[test]
fn opening_a_town_reads_the_game_and_changes_nothing() {
    let state = in_britain();
    let before = trajectory_digest(&state);
    let prng = state.prng_state;
    let (world, know) = (world_map(), seen_all());
    let a = GemViewState::open_with(&state, W169, 70, inputs(&know, &world));
    let b = GemViewState::open_with(&state, W169, 70, inputs(&know, &world));
    assert_eq!(a, b);
    assert_eq!(trajectory_digest(&state), before);
    assert_eq!(state.prng_state, prng);
}

// ---- the raster ------------------------------------------------------------

fn on() -> GemMapToggles {
    GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    }
}

fn view_layout(view: &GemViewState) -> TownLayout {
    match &view.shown {
        GemShown::Town(shown) => shown.layout,
        other => panic!("expected a town, got {other:?}"),
    }
}

fn dimmed(art: &MapArt, size: u32, tile: u8, lut: &BrightnessLut) -> [u8; 4] {
    let cells = art.cells_at(size);
    let rgb = cells.tile(usize::from(tile)).unwrap()[3 * cells.size() + 3];
    [lut.apply(rgb[0]), lut.apply(rgb[1]), lut.apply(rgb[2]), 255]
}

#[test]
fn the_picture_dims_the_land_to_55_percent_of_the_map() {
    let state = in_britain();
    let art = MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16));
    let world = world_map();
    let w = window();
    let (ex, ey) = w.entrance;
    let mut know = GemKnowledge::new();
    know.apply(&KnowEvent::SideCellSeen {
        scene: britain().byte,
        index: w.index_of(ex - 1, ey).unwrap(),
    });
    assert_eq!(TOWN_SIDE_PERCENT, 55);
    for (canvas, size) in [(W169, 7), (W43, 6)] {
        let view = GemViewState::open_with(&state, canvas, 100, inputs(&know, &world));
        let layout = view_layout(&view);
        assert_eq!(layout.cell, size);
        let mut out = GemImage::default();
        assert_eq!(
            render_gem_map(&state, &know, &art, &on(), &view, &mut out),
            GemRender::Town
        );
        assert!(
            out.rgba.chunks(4).all(|p| p[3] == 255),
            "opaque edge to edge"
        );
        let at = |fx: i32, fy: i32| {
            let (x, y, _, _) = layout.cell_rect(fx, fy);
            out.pixel((x + 3) as u32, (y + 3) as u32).unwrap()
        };
        let map = BrightnessLut::for_percent(70);
        let side = town_side_lut(70);
        assert_eq!(side, BrightnessLut::new(0.7 * 0.55));
        // A floor cell at 70%.
        assert_eq!(at(3, 3), dimmed(&art, size, state.grid[3 * 32 + 3], &map));
        // The one seen side cell at 55% of that.
        let land = world_tile(&world, ex - 1, ey);
        assert_eq!(at(-1, 16), dimmed(&art, size, land, &side));
        // Unseen land is black.
        assert_eq!(at(-1, 15), [0, 0, 0, 255]);
        assert_eq!(at(32, 16), [0, 0, 0, 255]);
        // The strips at the side brightness.
        assert_eq!(at(5, -1), dimmed(&art, size, CORNER, &side));
        assert_eq!(at(5, 32), dimmed(&art, size, CORNER, &side));
        // The party marker on the party's cell, at full brightness: the
        // 2 px yellow ring 3 px outside its 8 px sprite.
        let (px, py, _, _) = layout.cell_rect(15, 17);
        let sprite = (px + size as i32 / 2 - 4, py + size as i32 / 2 - 4);
        let ring = out
            .pixel((sprite.0 - 3) as u32, (sprite.1 + 4) as u32)
            .unwrap();
        assert_eq!(
            ring,
            [PARTY_YELLOW[0], PARTY_YELLOW[1], PARTY_YELLOW[2], 255]
        );
    }
}

#[test]
fn the_town_sizes_are_reduced_once_and_kept() {
    let art = MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16));
    let first: *const _ = art.cells_at(7);
    assert_eq!(art.cells_at(7).size(), 7);
    assert!(std::ptr::eq(first, art.cells_at(7)));
    assert_eq!(art.cells_at(6).size(), 6);
    assert!(std::ptr::eq(art.cells_at(4), &art.cells));
    assert!(std::ptr::eq(art.cells_at(8), &art.sprites));
}
