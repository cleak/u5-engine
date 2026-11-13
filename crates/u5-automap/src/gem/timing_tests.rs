//! M3-17 profiling (design 10.6 "Performance"): the probe and observe pair
//! stays under 20 us per command, and a full 16:9 render (the shown set
//! opened, the map, the marks and labels, the party marker) stays under
//! 8 ms, once per open and per page. The limits hold in the player's
//! (optimised) build, `cargo test --release -p u5-automap timing`; a debug
//! build only reports its numbers, as display/timing_tests.rs does.
//!
//! Every case is a worst case of its kind on synthetic data: outdoors on a
//! varied grid, a town whose whole side window has been seen, and a
//! dungeon whose eight levels are all remembered and marked, so the stack
//! and the labels are full.

use std::time::{Duration, Instant};

use u5_runtime::enhancements::{GemMapToggles, GlyphPolicy};
use u5_runtime::test_fixtures::{
    dungeon_state, open_dungeon_record, open_grid, open_world_grid, synthetic_tile_atlas,
    test_state, world_state,
};
use u5_runtime::{
    Area, CH_FONT_LEN, Direction, FixedCellFont, IBM_CH_FILE, PlayState, RUNES_CH_FILE, Scene,
    TileGraphicsDepth, ViewOverlay, ViewOverlayKind, ViewOverlayMode, WORLD_SIDE, WorldPlane,
    parse_ch_font,
};

use crate::gem::dungeon::DungeonInputs;
use crate::gem::knowledge::{
    DungeonMark, GemKnowledge, KnowEvent, MarkKind, SIDE_WINDOW_HEIGHT, SIDE_WINDOW_WIDTH,
};
use crate::gem::layout::GemCanvas;
use crate::gem::raster::{GemImage, GemRender, MapArt, render_gem_map};
use crate::gem::session::GemSession;
use crate::gem::shown::{GemViewState, TownInputs};

/// The 16:9 canvas (design 10.2.4) and the widest one.
const W169: GemCanvas = GemCanvas { w: 512, h: 240 };
const W219: GemCanvas = GemCanvas { w: 688, h: 240 };

/// Per command, for the probe before it and the observe after it.
const PROBE_LIMIT: Duration = Duration::from_micros(20);
/// Per open or page, for the shown set and the full render.
const RENDER_LIMIT: Duration = Duration::from_millis(8);

fn font(seed: u8) -> FixedCellFont {
    let bytes = (0..CH_FONT_LEN)
        .map(|i| {
            let (code, row) = ((i / 8) as u8, (i % 8) as u8);
            (code.wrapping_mul(31) ^ row.wrapping_mul(0x11) ^ seed) | 0x81
        })
        .collect::<Vec<_>>();
    let name = if seed == 0 {
        IBM_CH_FILE
    } else {
        RUNES_CH_FILE
    };
    parse_ch_font(&bytes, name).unwrap()
}

fn art() -> MapArt {
    MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16)).with_fonts(font(0), font(0x5A))
}

/// Everything on, labels included.
fn all_on() -> GemMapToggles {
    GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    }
}

fn gem_overlay(kind: ViewOverlayKind) -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind,
        mode: ViewOverlayMode::GemView,
    })
}

/// Plain terrain (tiles 0..16, none of them a feature) with `features`
/// cells of the place-marker, sign and moongate tiles scattered over it:
/// every `spacing`-th cell along a diagonal walk.
fn terrain_with_features(len: usize, spacing: usize) -> Vec<u8> {
    const FEATURE_TILES: [u8; 6] = [0x10, 0x13, 0x17, 0x1A, 0x89, 0xDC];
    (0..len)
        .map(|index| {
            if spacing > 0 && index % spacing == spacing / 2 {
                FEATURE_TILES[index / spacing % FEATURE_TILES.len()]
            } else {
                (index % 16) as u8
            }
        })
        .collect()
}

/// The world as the player meets it: about one feature in 700 cells, so
/// a 21:9 window holds a dozen or so (the real map holds fewer; the
/// real-file sweep in tests/gem_real_assets.rs measures it).
const REAL_SPACING: usize = 701;

fn outdoors() -> PlayState {
    outdoors_spaced(REAL_SPACING)
}

fn outdoors_spaced(spacing: usize) -> PlayState {
    let grid = terrain_with_features(open_world_grid().len(), spacing);
    let mut state = world_state(grid, 100, 100);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state
}

fn britain() -> Scene {
    Scene::from_key("TOWNE:1").unwrap()
}

fn in_britain() -> PlayState {
    let mut grid = open_grid();
    for (index, cell) in grid.iter_mut().enumerate() {
        *cell = 0x10 + (index % 7) as u8;
    }
    let mut state = test_state(grid, 15, 17);
    state.area = Area::Town {
        scene: britain(),
        floor: 0,
    };
    state
}

fn world_map() -> Vec<u8> {
    terrain_with_features(WORLD_SIDE * WORLD_SIDE, REAL_SPACING)
}

fn lit_dungeon() -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
    state.torch_counter = 50;
    state.player.facing = Direction::North;
    state
}

fn dungeon_record(state: &PlayState) -> u8 {
    match state.area {
        Area::Dungeon { scene, .. } => scene.record as u8,
        _ => unreachable!("a dungeon"),
    }
}

/// A record holding every side cell of Britain, every cell of every level
/// of the dungeon, and a mark of each kind on each level.
fn full_record(dungeon: u8) -> GemKnowledge {
    let mut know = GemKnowledge::new();
    let cells = u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT);
    for index in 0..cells {
        know.apply(&KnowEvent::SideCellSeen {
            scene: britain().byte,
            index,
        });
    }
    for level in 0..8 {
        know.apply(&KnowEvent::DungeonCellsSeen {
            record: dungeon,
            level,
            rows: [0xFF; 8],
        });
        for (i, kind) in [
            MarkKind::Ladder,
            MarkKind::Fountain,
            MarkKind::Chest,
            MarkKind::Trap,
            MarkKind::SecretDoor,
        ]
        .into_iter()
        .enumerate()
        {
            know.apply(&KnowEvent::DungeonMarked(DungeonMark {
                record: dungeon,
                level,
                x: i as u8,
                y: (i as u8 * 3) % 8,
                kind,
            }));
        }
    }
    know
}

/// The worst of `runs` timings of `f`.
fn worst(runs: usize, mut f: impl FnMut()) -> Duration {
    (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .max()
        .unwrap_or_default()
}

/// The mean of `runs` timings of `f`.
fn mean(runs: u32, mut f: impl FnMut()) -> Duration {
    let start = Instant::now();
    for _ in 0..runs {
        f();
    }
    start.elapsed() / runs
}

/// One command's probe and observe, per kind of place, with the view shut
/// and with it open (the gem view's own footprint is folded on the frame it
/// opens).
#[test]
fn probe_and_observe_take_under_20_us_per_command() {
    let mut session = GemSession::new();
    let dungeon = lit_dungeon();
    let mut cases: Vec<(&str, PlayState)> = vec![
        ("outdoors", outdoors()),
        ("town", in_britain()),
        ("dungeon", dungeon.clone()),
    ];
    let mut open = outdoors();
    open.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    cases.push(("outdoors, gem view open", open));
    let mut open = dungeon;
    open.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level: 3 });
    cases.push(("dungeon, gem view open", open));
    let mut slowest = Duration::ZERO;
    for (name, state) in &cases {
        session.before(state);
        session.after(state, 1);
        let took = mean(500, || {
            session.before(state);
            session.after(state, 1);
        });
        eprintln!("gem probe + observe, {name}: mean {took:?}");
        slowest = slowest.max(took);
    }
    if cfg!(debug_assertions) {
        return;
    }
    assert!(slowest < PROBE_LIMIT, "{slowest:?}");
}

/// The full 16:9 render of each kind of view, from the shown set to the
/// last label, over the fullest record, and at the widest canvas too.
#[test]
fn a_full_render_takes_under_8_ms() {
    let art = art();
    let t = all_on();
    let world = world_map();
    let dungeon = lit_dungeon();
    let know = full_record(dungeon_record(&dungeon));

    let mut outdoor = outdoors();
    outdoor.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    let mut town = in_britain();
    town.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    let mut cave = dungeon;
    cave.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level: 3 });

    let cases = [
        ("outdoors", &outdoor, GemRender::Outdoor),
        ("town", &town, GemRender::Town),
        ("dungeon stack", &cave, GemRender::Dungeon),
    ];
    let mut out = GemImage::default();
    let mut slowest = Duration::ZERO;
    for canvas in [W169, W219] {
        for (name, state, kind) in cases {
            let mut open_and_render = || {
                let view = GemViewState::open_all(
                    state,
                    canvas,
                    100,
                    TownInputs {
                        know: Some(&know),
                        world: Some(&world),
                        surroundings: true,
                    },
                    DungeonInputs {
                        know: Some(&know),
                        glyphs: GlyphPolicy::Perceived,
                    },
                );
                assert_eq!(
                    render_gem_map(state, &know, &art, &t, &view, &mut out),
                    kind
                );
            };
            // The first render also reduces the town's tile size once.
            open_and_render();
            let took = worst(20, open_and_render);
            eprintln!(
                "gem map open + render, {name}, {}x{}: worst {took:?}",
                canvas.w, canvas.h
            );
            slowest = slowest.max(took);
        }
    }
    if cfg!(debug_assertions) {
        return;
    }
    assert!(slowest < RENDER_LIMIT, "{slowest:?}");
}

/// A page turn on a one-panel dungeon map renders again from the same
/// shown set: under the same 8 ms.
#[test]
fn a_page_turn_takes_under_8_ms() {
    let art = art();
    let t = all_on();
    let mut cave = lit_dungeon();
    let know = full_record(dungeon_record(&cave));
    cave.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level: 3 });
    let mut view = GemViewState::open_all(
        &cave,
        GemCanvas { w: 320, h: 200 },
        100,
        TownInputs::default(),
        DungeonInputs {
            know: Some(&know),
            glyphs: GlyphPolicy::Perceived,
        },
    );
    assert!(view.is_one_panel_dungeon(true), "4:3 shows one panel");
    let mut out = GemImage::default();
    let mut up = true;
    let took = worst(20, || {
        if !view.turn_page(up, true) {
            up = !up;
            view.turn_page(up, true);
        }
        render_gem_map(&cave, &know, &art, &t, &view, &mut out);
    });
    eprintln!("gem map page turn + render, 320x200: worst {took:?}");
    if cfg!(debug_assertions) {
        return;
    }
    assert!(took < RENDER_LIMIT, "{took:?}");
}

/// A crowded world, one feature in every 13 cells (hundreds of "?" in one
/// window, far beyond any real map): reported only. The label layout grows
/// with the square of the tags, so this is where it would show first.
#[test]
fn a_crowded_world_reports_its_render_time() {
    if cfg!(debug_assertions) {
        // Seconds in a debug build: the optimised build reports it.
        return;
    }
    let art = art();
    let t = all_on();
    let know = GemKnowledge::new();
    let mut state = outdoors_spaced(13);
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    let mut out = GemImage::default();
    let took = worst(3, || {
        let view = GemViewState::open(&state, W169, 100);
        render_gem_map(&state, &know, &art, &t, &view, &mut out);
    });
    eprintln!("gem map open + render, crowded outdoors, 512x240: worst {took:?}");
}
