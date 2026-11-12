//! M3-10 shown-set tests (10.2.3, D-M3.5): the outdoor window samples the
//! live terrain only, wraps at the 0/255 seam on both planes, keeps the
//! live moongate, and is empty for anything but an open gem view.

use u5_runtime::test_fixtures::{
    dungeon_state, open_dungeon_record, open_grid, open_world_grid, test_state, world_state,
};
use u5_runtime::{
    ActiveObject, Area, PlayState, ViewOverlay, ViewOverlayKind, ViewOverlayMode, WorldPlane,
    world_cell_index,
};

use super::*;

/// The grass the open world grid is made of.
const GRASS: u8 = 5;
/// A tile only an object would put on a cell (a creature in the actor
/// range); never in the grid.
const CREATURE: u8 = 0x90;
/// The live moongate the runtime writes into the world grid.
const LIVE_GATE: u8 = 0xDC;

const BOX: GemCanvas = GemCanvas { w: 512, h: 240 };

fn overlay(kind: ViewOverlayKind, mode: ViewOverlayMode) -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind,
        mode,
    })
}

fn world(plane: WorldPlane, x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World { plane };
    state.active_objects[0].z = plane.save_floor();
    state.active_view_overlay = overlay(ViewOverlayKind::Surface, ViewOverlayMode::GemView);
    state
}

fn set_cell(state: &mut PlayState, x: usize, y: usize, tile: u8) {
    state.grid[world_cell_index(x, y)] = tile;
}

fn creature_at(state: &mut PlayState, x: usize, y: usize) {
    let z = state.active_objects[0].z;
    state.active_objects.push(ActiveObject {
        type_byte: CREATURE,
        tile: CREATURE,
        x,
        y,
        z,
        ..ActiveObject::empty()
    });
}

fn outdoor(view: &GemViewState) -> &OutdoorShown {
    match &view.shown {
        GemShown::Outdoor(shown) => shown,
        other => panic!("expected an outdoor shown set, got {other:?}"),
    }
}

#[test]
fn every_canvas_cell_is_sampled_terrain_only() {
    let mut state = world(WorldPlane::Britannia, 100, 100);
    for (i, (x, y)) in [(40, 75), (100, 100), (164, 125), (120, 90)]
        .into_iter()
        .enumerate()
    {
        set_cell(&mut state, x, y, 0x10 + i as u8);
    }
    let view = GemViewState::open(&state, BOX, 100);
    let shown = outdoor(&view);
    assert_eq!(shown.party, (100, 100));
    assert_eq!(shown.cells().count(), 129 * 61);
    for (dx, dy, tile) in shown.cells() {
        let expected = state.gem_sample_tile(100 + dx as isize, 100 + dy as isize, true);
        assert_eq!(tile, expected, "cell {dx},{dy}");
    }
    assert_eq!(shown.tile(-60, -25), Some(0x10));
    assert_eq!(shown.tile(64, 25), Some(0x12));
    assert_eq!(shown.tile(65, 0), None);
}

#[test]
fn the_window_wraps_at_the_seam() {
    let mut state = world(WorldPlane::Britannia, 1, 254);
    set_cell(&mut state, 255, 253, 0x21); // west of the seam, north
    set_cell(&mut state, 0, 0, 0x22); // east and south across both seams
    set_cell(&mut state, 3, 255, 0x23);
    let view = GemViewState::open(&state, BOX, 100);
    let shown = outdoor(&view);
    assert_eq!(shown.tile(-2, -1), Some(0x21));
    assert_eq!(shown.world_cell(-2, -1), (255, 253));
    assert_eq!(shown.tile(-1, 2), Some(0x22));
    assert_eq!(shown.world_cell(-1, 2), (0, 0));
    assert_eq!(shown.tile(2, 1), Some(0x23));
    assert!(shown.contains(255, 253));
    assert!(shown.contains(0, 0));
    assert!(shown.contains(65, 28));
    assert!(!shown.contains(66, 0)); // 65 east of the party: outside
    assert!(!shown.contains(1, 30)); // 32 south: outside
}

#[test]
fn the_underworld_draws_the_same_way() {
    let mut state = world(WorldPlane::Underworld, 128, 128);
    set_cell(&mut state, 130, 127, 0x31);
    let view = GemViewState::open(&state, BOX, 100);
    let shown = outdoor(&view);
    assert_eq!(shown.plane, WorldPlane::Underworld);
    assert_eq!(shown.tile(2, -1), Some(0x31));
}

#[test]
fn an_object_over_grass_draws_grass() {
    let mut state = world(WorldPlane::Britannia, 100, 100);
    creature_at(&mut state, 103, 101);
    // The faithful lookup shows the creature: the test is meaningful.
    assert_eq!(state.gem_sample_tile(103, 101, false), CREATURE);
    let view = GemViewState::open(&state, BOX, 100);
    let shown = outdoor(&view);
    let grass = state.animation.resolve_static_tile(GRASS);
    assert_eq!(shown.tile(3, 1), Some(grass));
    assert!(shown.cells().all(|(_, _, tile)| tile != CREATURE));
    // The party's own object never shows either: its cell is terrain.
    assert_eq!(shown.tile(0, 0), Some(grass));
}

#[test]
fn the_live_moongate_draws() {
    let mut state = world(WorldPlane::Britannia, 100, 100);
    set_cell(&mut state, 96, 102, LIVE_GATE);
    let view = GemViewState::open(&state, BOX, 100);
    let shown = outdoor(&view);
    let gate = state.animation.resolve_static_tile(LIVE_GATE);
    assert_eq!(shown.tile(-4, 2), Some(gate));
    assert_ne!(gate, state.animation.resolve_static_tile(GRASS));
}

#[test]
fn nothing_is_shown_without_an_open_gem_view() {
    let base = world(WorldPlane::Britannia, 100, 100);
    for overlay in [
        None,
        overlay(ViewOverlayKind::Surface, ViewOverlayMode::PeerSpell),
        overlay(ViewOverlayKind::Surface, ViewOverlayMode::XRaySpell),
        overlay(ViewOverlayKind::Surface, ViewOverlayMode::SurfaceLook),
    ] {
        let mut state = base.clone();
        state.active_view_overlay = overlay.clone();
        assert!(!gem_view_open(&state), "{overlay:?}");
        assert_eq!(GemViewState::open(&state, BOX, 100).shown, GemShown::Empty);
    }
    assert!(gem_view_open(&base));
}

#[test]
fn towns_and_dungeons_have_no_shown_set_yet() {
    let mut town = test_state(open_grid(), 5, 5);
    town.active_view_overlay = overlay(ViewOverlayKind::Surface, ViewOverlayMode::GemView);
    assert_eq!(GemViewState::open(&town, BOX, 100).shown, GemShown::Empty);
    let mut cave = dungeon_state(open_dungeon_record(), 0, 1, 1);
    cave.active_view_overlay = overlay(
        ViewOverlayKind::Dungeon { level: 0 },
        ViewOverlayMode::GemView,
    );
    assert_eq!(GemViewState::open(&cave, BOX, 100).shown, GemShown::Empty);
}

#[test]
fn opening_reads_the_game_and_changes_nothing() {
    let mut state = world(WorldPlane::Britannia, 7, 250);
    creature_at(&mut state, 8, 250);
    let before = u5_runtime::enhancements::trajectory_digest(&state);
    let prng = state.prng_state;
    let a = GemViewState::open(&state, BOX, 70);
    let b = GemViewState::open(&state, BOX, 70);
    assert_eq!(a, b);
    assert_eq!(u5_runtime::enhancements::trajectory_digest(&state), before);
    assert_eq!(state.prng_state, prng);
}
