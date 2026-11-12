//! The outdoor gem map over the player's own files (10.2.4, M3-10).
//!
//! Invariants only: nothing of the game is printed or committed. Skipped
//! unless `U5_MOCKUP_PROFILE` names a validated game-data copy
//! (`game-dev-u5 test-assets` or `prepare-profile`). With `U5_GEM_SHOTS`
//! set to a directory, the renders are written there as review-only PPM
//! files for the side-by-side mockup check (never committed).

use u5_automap::gem::raster::PARTY_YELLOW;
use u5_automap::gem::{
    GemCanvas, GemImage, GemKnowledge, GemRender, GemViewState, MapArt, render_gem_map,
};
use u5_runtime::enhancements::GemMapToggles;
use u5_runtime::{
    PlayState, TileGraphicsDepth, ViewOverlay, ViewOverlayKind, ViewOverlayMode, WorldPlane,
};

const CANVASES: [GemCanvas; 3] = [
    GemCanvas { w: 320, h: 200 },
    GemCanvas { w: 512, h: 240 },
    GemCanvas { w: 688, h: 240 },
];

fn on() -> GemMapToggles {
    GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    }
}

fn rgba(rgb: [u8; 3]) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], 255]
}

fn gem_overlay() -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Surface,
        mode: ViewOverlayMode::GemView,
    })
}

fn env_dir(name: &str) -> Option<std::path::PathBuf> {
    let dir = std::path::PathBuf::from(std::env::var_os(name)?);
    dir.is_dir().then_some(dir)
}

fn real_world(dir: &std::path::Path, at: (usize, usize), time: (u8, u8)) -> PlayState {
    let mut state = PlayState::load_scene(
        dir,
        u5_runtime::PlayOptions {
            target: u5_runtime::PlayTarget::World(WorldPlane::Britannia),
            start: Some(at),
            clock: u5_runtime::GameClock::new(time.0, time.1).unwrap(),
            ..u5_runtime::PlayOptions::default()
        },
    )
    .unwrap();
    state.active_view_overlay = gem_overlay();
    state
}

/// With `U5_MOCKUP_PROFILE` naming a game copy: the real atlas and
/// Britannia render on all three canvases, opaque, with the marker at the
/// centre. When `U5_GEM_SHOTS` names a folder, the renders are also
/// written there as PPM files for the side-by-side check (never committed).
#[test]
fn real_assets_render_outdoors() {
    let Some(dir) = env_dir("U5_MOCKUP_PROFILE") else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let atlas = u5_runtime::load_tile_atlas(&dir, TileGraphicsDepth::Ega16).unwrap();
    let art = MapArt::build(&atlas);
    // The mockups' S1: the forest road west of Britain, at 12:06.
    let state = real_world(&dir, (76, 108), (12, 6));
    let shots = env_dir("U5_GEM_SHOTS");
    for (name, canvas) in [
        ("O3-43", CANVASES[0]),
        ("O1-169", CANVASES[1]),
        ("O2-219", CANVASES[2]),
    ] {
        for percent in [70, 100] {
            let mut t = on();
            t.opts.map_brightness = percent;
            let view = GemViewState::open(&state, canvas, 100);
            let mut out = GemImage::default();
            assert_eq!(
                render_gem_map(&state, &GemKnowledge::new(), &art, &t, &view, &mut out),
                GemRender::Outdoor
            );
            assert!(out.rgba.chunks(4).all(|p| p[3] == 255));
            let centre = out.pixel(canvas.w / 2 - 7, canvas.h / 2).unwrap();
            assert_eq!(centre, rgba(PARTY_YELLOW));
            if let Some(shots) = &shots {
                let mut ppm = format!("P6\n{} {}\n255\n", out.w, out.h).into_bytes();
                for p in out.rgba.chunks(4) {
                    ppm.extend_from_slice(&p[..3]);
                }
                std::fs::write(shots.join(format!("engine-{name}-{percent:03}.ppm")), ppm).unwrap();
            }
        }
    }
}

fn real_britain(dir: &std::path::Path) -> PlayState {
    let mut state = PlayState::load_scene(
        dir,
        u5_runtime::PlayOptions {
            target: u5_runtime::PlayTarget::Town(u5_runtime::Scene::from_key("TOWNE:1").unwrap()),
            floor: 0,
            // The mockups' S2: Britain, party at (15, 17).
            start: Some((15, 17)),
            clock: u5_runtime::GameClock::new(10, 0).unwrap(),
            ..u5_runtime::PlayOptions::default()
        },
    )
    .unwrap();
    state.active_view_overlay = gem_overlay();
    state
}

fn write_ppm(dir: &std::path::Path, name: &str, out: &GemImage) {
    let mut ppm = format!("P6\n{} {}\n255\n", out.w, out.h).into_bytes();
    for p in out.rgba.chunks(4) {
        ppm.extend_from_slice(&p[..3]);
    }
    std::fs::write(dir.join(name), ppm).unwrap();
}

/// With `U5_MOCKUP_PROFILE` naming a game copy: Britain at 16:9 and 4:3
/// (the mockups' T1 and T3), once with every cell of its side window seen
/// and once on a fresh record, where every side cell is black. With
/// `U5_GEM_SHOTS`, the renders are written there as PPM files.
#[test]
fn real_assets_render_britain() {
    use u5_automap::StaticMaps;
    use u5_automap::gem::{
        KnowEvent, SIDE_WINDOW_HEIGHT, SIDE_WINDOW_WIDTH, TownInputs, TownPart, side_window,
        town_world_plane,
    };
    let Some(dir) = env_dir("U5_MOCKUP_PROFILE") else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let atlas = u5_runtime::load_tile_atlas(&dir, TileGraphicsDepth::Ega16).unwrap();
    let art = MapArt::build(&atlas);
    let state = real_britain(&dir);
    let scene = u5_runtime::Scene::from_key("TOWNE:1").unwrap().byte;
    assert!(side_window(scene).is_some());
    let mut maps = StaticMaps::new(&dir);
    let world = maps
        .world_map(town_world_plane(&state).unwrap())
        .unwrap()
        .to_vec();
    let mut seen = GemKnowledge::new();
    for index in 0..u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT) {
        seen.apply(&KnowEvent::SideCellSeen { scene, index });
    }
    let fresh = GemKnowledge::new();
    let shots = env_dir("U5_GEM_SHOTS");
    for (name, canvas) in [("T3-43", CANVASES[0]), ("T1-169", CANVASES[1])] {
        for (record, know) in [("seen", &seen), ("fresh", &fresh)] {
            for percent in [70, 100] {
                let mut t = on();
                t.opts.map_brightness = percent;
                let inputs = TownInputs {
                    know: Some(know),
                    world: Some(&world),
                    surroundings: true,
                };
                let view = GemViewState::open_with(&state, canvas, 100, inputs);
                let u5_automap::gem::GemShown::Town(shown) = &view.shown else {
                    panic!("a town view");
                };
                let sides = shown
                    .cells()
                    .filter(|cell| cell.2 == TownPart::Side)
                    .collect::<Vec<_>>();
                assert!(!sides.is_empty());
                if record == "fresh" {
                    assert!(sides.iter().all(|cell| cell.3.is_none()));
                } else {
                    assert!(sides.iter().all(|cell| cell.3.is_some()));
                }
                let mut out = GemImage::default();
                assert_eq!(
                    render_gem_map(&state, know, &art, &t, &view, &mut out),
                    GemRender::Town
                );
                assert!(out.rgba.chunks(4).all(|p| p[3] == 255));
                if let Some(shots) = &shots {
                    write_ppm(
                        shots,
                        &format!("engine-{name}-{record}-{percent:03}.ppm"),
                        &out,
                    );
                }
            }
        }
    }
}
