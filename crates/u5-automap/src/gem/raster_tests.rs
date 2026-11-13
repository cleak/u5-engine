//! M3-10 raster tests (10.2.4, 10.2.7 party marker, 10.2.10 brightness):
//! the map is the reduced terrain dimmed to the map brightness, the
//! marker sits at the exact centre at full brightness, no object reaches
//! the picture, the render is cleared unless a gem view is open, and it
//! is pure and fast.

use std::time::{Duration, Instant};

use u5_runtime::enhancements::{GemMapToggles, trajectory_digest};
use u5_runtime::test_fixtures::{
    open_grid, open_world_grid, synthetic_tile_atlas, test_state, world_state,
};
use u5_runtime::{
    ActiveObject, Area, PLAYER_TILE, PlayState, TileGraphicsDepth, TransportState, ViewOverlay,
    ViewOverlayKind, ViewOverlayMode, WorldPlane, world_cell_index,
};

use super::*;
use crate::gem::layout::OutdoorLayout;

const GRASS: u8 = 5;
const CREATURE: u8 = 0x90;
const LIVE_GATE: u8 = 0xDC;

const CANVASES: [GemCanvas; 3] = [
    GemCanvas { w: 320, h: 200 },
    GemCanvas { w: 512, h: 240 },
    GemCanvas { w: 688, h: 240 },
];

fn art() -> MapArt {
    MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16))
}

/// The feature on, with `labels` off: these tests read the map layer, and
/// the marks over it are M3-15's (`features_tests.rs`).
fn on() -> GemMapToggles {
    GemMapToggles {
        enhanced: true,
        labels: false,
        ..GemMapToggles::default()
    }
}

fn gem_overlay() -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Surface,
        mode: ViewOverlayMode::GemView,
    })
}

fn world(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state.active_view_overlay = gem_overlay();
    state
}

fn render(
    state: &PlayState,
    t: &GemMapToggles,
    canvas: GemCanvas,
    display_map: u8,
) -> (GemImage, GemRender) {
    let view = GemViewState::open(state, canvas, display_map);
    let mut out = GemImage::default();
    let drawn = render_gem_map(state, &GemKnowledge::new(), &art(), t, &view, &mut out);
    (out, drawn)
}

/// The dimmed colour of `tile`'s reduced pixel `(i, j)` at `percent`.
fn map_pixel(art: &MapArt, tile: u8, i: usize, j: usize, percent: u8) -> [u8; 4] {
    let lut = BrightnessLut::for_percent(percent);
    let rgb = art.cells.tile(usize::from(tile)).unwrap()[j * art.cells.size() + i];
    [lut.apply(rgb[0]), lut.apply(rgb[1]), lut.apply(rgb[2]), 255]
}

/// The canvas pixel `(i, j)` inside the cell at offset `(dx, dy)`.
fn cell_pixel(image: &GemImage, dx: i32, dy: i32, i: i32, j: i32) -> [u8; 4] {
    let layout = OutdoorLayout::new(GemCanvas {
        w: image.w,
        h: image.h,
    });
    let (x, y, _, _) = layout.cell_rect(dx, dy);
    image.pixel((x + i) as u32, (y + j) as u32).unwrap()
}

fn rgba(rgb: [u8; 3]) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], 255]
}

// ---- cleared ---------------------------------------------------------------

#[test]
fn the_image_is_cleared_unless_a_gem_view_is_open() {
    let state = world(100, 100);
    let canvas = CANVASES[1];
    // Feature off.
    let (image, drawn) = render(&state, &GemMapToggles::default(), canvas, 100);
    assert_eq!(drawn, GemRender::Cleared);
    assert!(image.is_cleared());
    assert_eq!((image.w, image.h), (512, 240));
    // No overlay, Peer, X-Ray.
    for overlay in [
        None,
        Some(ViewOverlay {
            mode: ViewOverlayMode::PeerSpell,
            ..gem_overlay().unwrap()
        }),
        Some(ViewOverlay {
            mode: ViewOverlayMode::XRaySpell,
            ..gem_overlay().unwrap()
        }),
    ] {
        let mut other = state.clone();
        other.active_view_overlay = overlay;
        let (image, drawn) = render(&other, &on(), canvas, 100);
        assert_eq!(drawn, GemRender::Cleared);
        assert!(image.is_cleared());
    }
    // A town gem draws its town (M3-12), and only with the feature on.
    let mut town = test_state(open_grid(), 5, 5);
    town.active_view_overlay = gem_overlay();
    assert_eq!(render(&town, &on(), canvas, 100).1, GemRender::Town);
    let (image, drawn) = render(&town, &GemMapToggles::default(), canvas, 100);
    assert_eq!(drawn, GemRender::Cleared);
    assert!(image.is_cleared());
}

#[test]
fn a_view_that_closed_after_opening_renders_cleared() {
    let mut state = world(100, 100);
    let view = GemViewState::open(&state, CANVASES[0], 100);
    state.active_view_overlay = None;
    let mut out = GemImage::default();
    let drawn = render_gem_map(&state, &GemKnowledge::new(), &art(), &on(), &view, &mut out);
    assert_eq!(drawn, GemRender::Cleared);
    assert!(out.is_cleared());
}

// ---- the map layer ------------------------------------------------------------

#[test]
fn every_pixel_is_drawn_and_opaque_edge_to_edge() {
    let state = world(100, 100);
    for canvas in CANVASES {
        let (image, drawn) = render(&state, &on(), canvas, 100);
        assert_eq!(drawn, GemRender::Outdoor);
        assert_eq!((image.w, image.h), (canvas.w, canvas.h));
        assert!(image.rgba.chunks(4).all(|p| p[3] == 255), "{canvas:?}");
    }
}

#[test]
fn each_cell_is_its_reduced_tile_dimmed_to_seventy_percent() {
    let mut state = world(100, 100);
    state.grid[world_cell_index(110, 95)] = 0x0A;
    let art = art();
    let (image, _) = render(&state, &on(), CANVASES[1], 100);
    let grass = state.animation.resolve_static_tile(GRASS);
    for (dx, dy, tile) in [(10, -5, 0x0A), (-20, 7, grass)] {
        for j in 0..4 {
            for i in 0..4 {
                assert_eq!(
                    cell_pixel(&image, dx, dy, i, j),
                    map_pixel(&art, tile, i as usize, j as usize, 70)
                );
            }
        }
    }
    // The 70% is a real dimming of a lit tile.
    assert_ne!(
        map_pixel(&art, 0x0A, 0, 0, 70),
        map_pixel(&art, 0x0A, 0, 0, 100)
    );
}

#[test]
fn the_cut_edge_cells_show_their_inner_two_pixels() {
    let mut state = world(100, 100);
    state.grid[world_cell_index(36, 100)] = 0x0B; // dx = -64, the west edge
    state.grid[world_cell_index(164, 100)] = 0x0C; // dx = 64, the east edge
    let art = art();
    let (image, _) = render(&state, &on(), CANVASES[1], 100);
    // Pixels 2 and 3 of the west cell are canvas columns 0 and 1.
    let y = 118;
    assert_eq!(image.pixel(0, y).unwrap(), map_pixel(&art, 0x0B, 2, 0, 70));
    assert_eq!(image.pixel(1, y).unwrap(), map_pixel(&art, 0x0B, 3, 0, 70));
    assert_eq!(
        image.pixel(510, y).unwrap(),
        map_pixel(&art, 0x0C, 0, 0, 70)
    );
    assert_eq!(
        image.pixel(511, y).unwrap(),
        map_pixel(&art, 0x0C, 1, 0, 70)
    );
}

#[test]
fn the_seam_wraps_in_the_picture() {
    let mut state = world(0, 0);
    // Clear of the 16 px marker, across both seams.
    state.grid[world_cell_index(250, 251)] = 0x0D;
    state.grid[world_cell_index(5, 4)] = 0x0E;
    let art = art();
    let (image, _) = render(&state, &on(), CANVASES[0], 100);
    assert_eq!(
        cell_pixel(&image, -6, -5, 0, 0),
        map_pixel(&art, 0x0D, 0, 0, 70)
    );
    assert_eq!(
        cell_pixel(&image, 5, 4, 3, 3),
        map_pixel(&art, 0x0E, 3, 3, 70)
    );
}

#[test]
fn an_object_over_grass_draws_grass() {
    let mut state = world(100, 100);
    let z = state.active_objects[0].z;
    state.active_objects.push(ActiveObject {
        type_byte: CREATURE,
        tile: CREATURE,
        x: 112,
        y: 100,
        z,
        ..ActiveObject::empty()
    });
    assert_eq!(state.gem_sample_tile(112, 100, false), CREATURE);
    let art = art();
    let grass = state.animation.resolve_static_tile(GRASS);
    assert_ne!(
        art.cells.tile(usize::from(grass)),
        art.cells.tile(usize::from(CREATURE)),
        "the fixture tells grass from the creature"
    );
    let (image, _) = render(&state, &on(), CANVASES[1], 100);
    for j in 0..4 {
        for i in 0..4 {
            assert_eq!(
                cell_pixel(&image, 12, 0, i, j),
                map_pixel(&art, grass, i as usize, j as usize, 70)
            );
        }
    }
}

#[test]
fn the_live_gate_draws() {
    let mut state = world(100, 100);
    state.grid[world_cell_index(96, 102)] = LIVE_GATE;
    let gate = state.animation.resolve_static_tile(LIVE_GATE);
    let art = art();
    let (image, _) = render(&state, &on(), CANVASES[1], 100);
    assert_eq!(
        cell_pixel(&image, -4, 2, 1, 1),
        map_pixel(&art, gate, 1, 1, 70)
    );
    assert_ne!(
        map_pixel(&art, gate, 1, 1, 70),
        map_pixel(&art, GRASS, 1, 1, 70)
    );
}

// ---- brightness -------------------------------------------------------------

#[test]
fn the_map_brightness_setting_and_the_link() {
    let state = world(100, 100);
    let art = art();
    let at = |t: &GemMapToggles, display: u8| {
        cell_pixel(&render(&state, t, CANVASES[0], display).0, 9, 9, 0, 0)
    };
    let grass = state.animation.resolve_static_tile(GRASS);
    let mut t = on();
    t.opts.map_brightness = 40;
    assert_eq!(at(&t, 100), map_pixel(&art, grass, 0, 0, 40));
    // The link is off: display.map_brightness is ignored.
    assert_eq!(at(&t, 55), map_pixel(&art, grass, 0, 0, 40));
    t.brightness_link = true;
    // With the link on and the display key at 100, nothing changes.
    assert_eq!(
        at(&t, 100),
        at(
            &{
                let mut t = t;
                t.brightness_link = false;
                t
            },
            100
        )
    );
    assert_eq!(at(&t, 100), map_pixel(&art, grass, 0, 0, 40));
    // Below 100 the display key wins.
    assert_eq!(at(&t, 55), map_pixel(&art, grass, 0, 0, 55));
    // 100 is undimmed: the reduced tile itself.
    t.brightness_link = false;
    t.opts.map_brightness = 100;
    let raw = art.cells.tile(usize::from(grass)).unwrap()[0];
    assert_eq!(at(&t, 100), rgba(raw));
}

// ---- the party marker ---------------------------------------------------------

/// The marker's rows and columns from its outer edge: black, yellow,
/// yellow, black, eight sprite pixels, black, yellow, yellow, black.
fn ring_of(k: i32) -> Option<[u8; 3]> {
    match k {
        0 | 3 | 12 | 15 => Some([0, 0, 0]),
        1 | 2 | 13 | 14 => Some(PARTY_YELLOW),
        _ => None,
    }
}

#[test]
fn the_marker_is_a_steady_16_px_box_at_the_exact_centre() {
    let state = world(100, 100);
    let art = art();
    let sprite = art
        .sprites
        .tile(party_sprite_tile(&state).unwrap())
        .unwrap()
        .to_vec();
    for canvas in CANVASES {
        let (image, _) = render(&state, &on(), canvas, 100);
        let (x0, y0) = (canvas.w as i32 / 2 - 8, canvas.h as i32 / 2 - 8);
        for j in 0..PARTY_MARKER_PX as i32 {
            for i in 0..PARTY_MARKER_PX as i32 {
                let got = image.pixel((x0 + i) as u32, (y0 + j) as u32).unwrap();
                // The frames nest, so the nearer edge decides.
                let edge = |k: i32| k.min(15 - k);
                let nearer = if edge(i) <= edge(j) { i } else { j };
                let expected = ring_of(nearer)
                    .filter(|_| edge(nearer) < 4)
                    .unwrap_or_else(|| sprite[((j - 4) * 8 + (i - 4)) as usize]);
                assert_eq!(got, rgba(expected), "{canvas:?} at {i},{j}");
            }
        }
        // The box's centre is the canvas centre: 8 px each way.
        assert_eq!(2 * x0 + PARTY_MARKER_PX as i32, canvas.w as i32);
        assert_eq!(2 * y0 + PARTY_MARKER_PX as i32, canvas.h as i32);
    }
}

#[test]
fn the_map_is_dimmed_and_the_marker_is_not() {
    let state = world(100, 100);
    let art = art();
    for percent in [25, 70, 100] {
        let mut t = on();
        t.opts.map_brightness = percent;
        let (image, _) = render(&state, &t, CANVASES[1], 100);
        // The yellow ring and the sprite at full brightness at every level.
        assert_eq!(image.pixel(256 - 7, 120).unwrap(), rgba(PARTY_YELLOW));
        let sprite = art
            .sprites
            .tile(party_sprite_tile(&state).unwrap())
            .unwrap();
        assert_eq!(image.pixel(256 - 4, 120 - 4).unwrap(), rgba(sprite[0]));
        let grass = state.animation.resolve_static_tile(GRASS);
        assert_eq!(
            cell_pixel(&image, 20, 0, 0, 0),
            map_pixel(&art, grass, 0, 0, percent)
        );
    }
    assert_eq!(PARTY_YELLOW, [0xFF, 0xFF, 0x55]);
}

#[test]
fn the_sprite_comes_from_the_transport_not_an_object() {
    let mut state = world(100, 100);
    // The party's own object byte says something else: it is not read.
    state.active_objects[0].tile = CREATURE;
    assert_eq!(
        party_sprite_tile(&state),
        Some(u5_runtime::ACTOR_TILE_BANK_BASE + usize::from(PLAYER_TILE))
    );
    let ship_tile = 0x20;
    state.player.transport = TransportState::Ship {
        type_byte: 0x20,
        tile: ship_tile,
        sails_hoisted: false,
        hull: 99,
        skiffs: 1,
    };
    assert_eq!(
        party_sprite_tile(&state),
        Some(u5_runtime::ACTOR_TILE_BANK_BASE + usize::from(ship_tile))
    );
    // A transport that draws nothing leaves the box black inside.
    state.player.transport = TransportState::SpriteSuppressed;
    assert_eq!(party_sprite_tile(&state), None);
    let (image, _) = render(&state, &on(), CANVASES[1], 100);
    for j in 0..8 {
        for i in 0..8 {
            assert_eq!(image.pixel(252 + i, 116 + j).unwrap(), [0, 0, 0, 255]);
        }
    }
}

// ---- purity and time ------------------------------------------------------------

#[test]
fn the_render_is_pure() {
    let mut state = world(3, 252);
    let z = state.active_objects[0].z;
    state.active_objects.push(ActiveObject {
        type_byte: CREATURE,
        tile: CREATURE,
        x: 4,
        y: 252,
        z,
        ..ActiveObject::empty()
    });
    let before = trajectory_digest(&state);
    let prng = state.prng_state;
    let copy = state.clone();
    let view = GemViewState::open(&state, CANVASES[2], 100);
    let art = art();
    let (mut a, mut b) = (GemImage::default(), GemImage::default());
    render_gem_map(&state, &GemKnowledge::new(), &art, &on(), &view, &mut a);
    render_gem_map(&state, &GemKnowledge::new(), &art, &on(), &view, &mut b);
    assert_eq!(a, b);
    assert_eq!(trajectory_digest(&state), before);
    assert_eq!(state.prng_state, prng);
    assert_eq!(state.active_objects, copy.active_objects);
    assert_eq!(state.grid, copy.grid);
    // A reused image is redrawn whole.
    let mut reused = GemImage::new(GemCanvas { w: 16, h: 16 });
    reused.rgba.fill(7);
    render_gem_map(
        &state,
        &GemKnowledge::new(),
        &art,
        &on(),
        &view,
        &mut reused,
    );
    assert_eq!(reused, a);
}

/// The render at the widest canvas stays well inside a frame in the
/// player's (optimised) build (`cargo test --release -p u5-automap`). A
/// debug build only reports its number, as display/timing_tests.rs does.
#[test]
fn a_render_at_688_by_240_takes_under_8_ms() {
    let state = world(100, 100);
    let view = GemViewState::open(&state, CANVASES[2], 100);
    let art = art();
    let mut out = GemImage::default();
    let mut worst = Duration::ZERO;
    for _ in 0..20 {
        let start = Instant::now();
        render_gem_map(&state, &GemKnowledge::new(), &art, &on(), &view, &mut out);
        worst = worst.max(start.elapsed());
    }
    eprintln!("gem map render, 688x240: worst {worst:?}");
    if cfg!(debug_assertions) {
        return;
    }
    assert!(worst < Duration::from_millis(8), "{worst:?}");
}
