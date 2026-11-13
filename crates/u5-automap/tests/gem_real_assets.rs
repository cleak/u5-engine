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

/// Despise, the mockups' D scenes: level 3 at (6, 1) facing north, a gem
/// view up.
fn real_despise(dir: &std::path::Path) -> PlayState {
    let scene = u5_runtime::DungeonScene::from_record(1).unwrap();
    let mut state = PlayState::load_scene(
        dir,
        u5_runtime::PlayOptions {
            target: u5_runtime::PlayTarget::Dungeon(scene),
            floor: 2,
            start: Some((6, 1)),
            clock: u5_runtime::GameClock::new(12, 0).unwrap(),
            ..u5_runtime::PlayOptions::default()
        },
    )
    .unwrap();
    state.player.facing = u5_runtime::Direction::North;
    state.active_view_overlay = Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind: ViewOverlayKind::Dungeon { level: 2 },
        mode: ViewOverlayMode::GemView,
    });
    state
}

/// With `U5_MOCKUP_PROFILE` naming a game copy: the Despise stack at the
/// three canvases, with and without the wrap rows. The remembered cells
/// come from `U5_GEM_DUNGEON_MEMORY`, a review-only list of `cell L X Y`
/// and `trap L X Y` lines (the mockups' demo walk), when it is set; else
/// only the flood shows. With `U5_GEM_SHOTS`, the renders are written there
/// as PPM files.
#[test]
fn real_assets_render_despise() {
    use u5_automap::gem::{DungeonInputs, DungeonMark, GemShown, KnowEvent, MarkKind, TownInputs};
    use u5_runtime::enhancements::GlyphPolicy;
    let Some(dir) = env_dir("U5_MOCKUP_PROFILE") else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let atlas = u5_runtime::load_tile_atlas(&dir, TileGraphicsDepth::Ega16).unwrap();
    let art = MapArt::build(&atlas).with_fonts(
        u5_runtime::load_ibm_ch_font(&dir).unwrap(),
        u5_runtime::load_runes_ch_font(&dir).unwrap(),
    );
    let state = real_despise(&dir);
    let mut know = GemKnowledge::new();
    if let Some(path) = std::env::var_os("U5_GEM_DUNGEON_MEMORY") {
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            let [kind, l, x, y] = words[..] else { continue };
            let (l, x, y): (u8, u8, u8) =
                (l.parse().unwrap(), x.parse().unwrap(), y.parse().unwrap());
            match kind {
                "cell" => {
                    let mut rows = [0; 8];
                    rows[usize::from(y)] = 1 << x;
                    know.apply(&KnowEvent::DungeonCellsSeen {
                        record: 1,
                        level: l,
                        rows,
                    });
                }
                "trap" => {
                    know.apply(&KnowEvent::DungeonMarked(DungeonMark {
                        record: 1,
                        level: l,
                        x,
                        y,
                        kind: MarkKind::Trap,
                    }));
                }
                _ => {}
            }
        }
    }
    let shots = env_dir("U5_GEM_SHOTS");
    for (name, canvas) in [
        ("D3-43", CANVASES[0]),
        ("D1-169", CANVASES[1]),
        ("D2-219", CANVASES[2]),
    ] {
        for wrap in [true, false] {
            let mut t = on();
            t.opts.wrap_rows = wrap;
            let view = GemViewState::open_all(
                &state,
                canvas,
                100,
                TownInputs::default(),
                DungeonInputs {
                    know: Some(&know),
                    glyphs: GlyphPolicy::Perceived,
                },
            );
            assert!(matches!(view.shown, GemShown::Dungeon(_)));
            let mut out = GemImage::default();
            assert_eq!(
                render_gem_map(&state, &know, &art, &t, &view, &mut out),
                GemRender::Dungeon
            );
            assert!(out.rgba.chunks(4).all(|p| p[3] == 255));
            if let Some(shots) = &shots {
                let tag = if wrap { "" } else { "-nowrap" };
                write_ppm(shots, &format!("engine-{name}{tag}.ppm"), &out);
            }
            // The lone 4:3 panel pages (M3-14): PgUp and PgDn from the
            // party's level, as the mockups' D3-pgup and D3-pgdn.
            if canvas == CANVASES[0] {
                for (up, page) in [(true, "pgup"), (false, "pgdn")] {
                    let mut paged = view.clone();
                    if !paged.turn_page(up, true) {
                        continue;
                    }
                    let mut out = GemImage::default();
                    assert_eq!(
                        render_gem_map(&state, &know, &art, &t, &paged, &mut out),
                        GemRender::Dungeon
                    );
                    assert!(out.rgba.chunks(4).all(|p| p[3] == 255));
                    if let Some(shots) = &shots {
                        let tag = if wrap { "" } else { "-nowrap" };
                        write_ppm(shots, &format!("engine-D3-{page}{tag}.ppm"), &out);
                    }
                }
            }
        }
    }
}

/// A review-only record from `U5_GEM_KNOWLEDGE` (the mockups' demo
/// knowledge, never committed): one event per line, `place S`, `shrine V`,
/// `codex`, `entrance R`, `gate P X Y`, `sign SCENE FLOOR X Y`,
/// `trade SCENE CODE`, `keeper SCENE CODE FLOOR X Y`,
/// `link SCENE FLOOR X Y up|down`, `exit SCENE N|E|S|W AT`, `sides SCENE`
/// (every side cell seen), `cell R L X Y` and `mark R L X Y KIND` (kind
/// code as `MarkKind`).
fn demo_knowledge() -> u5_automap::gem::GemKnowledge {
    use u5_automap::gem::{
        DungeonMark, EdgeSide, ExitRun, GateCell, KnowEvent, LinkDir, MarkKind, SIDE_WINDOW_HEIGHT,
        SIDE_WINDOW_WIDTH, SignCell, TownLink, Trade,
    };
    let mut know = GemKnowledge::new();
    let Some(path) = std::env::var_os("U5_GEM_KNOWLEDGE") else {
        return know;
    };
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        let n = |i: usize| -> i64 { words[i].parse().unwrap() };
        let events = match words.first().copied() {
            Some("place") => vec![KnowEvent::PlaceEntered { scene: n(1) as u8 }],
            Some("shrine") => vec![KnowEvent::ShrineEntered { virtue: n(1) as u8 }],
            Some("codex") => vec![KnowEvent::CodexEntered],
            Some("entrance") => vec![KnowEvent::EntranceNamed { record: n(1) as u8 }],
            Some("gate") => vec![KnowEvent::GateUsed(GateCell {
                plane: n(1) as u8,
                x: n(2) as u8,
                y: n(3) as u8,
            })],
            Some("sign") => vec![KnowEvent::SignRead(SignCell {
                scene: n(1) as u8,
                floor: n(2) as i8,
                x: n(3) as u8,
                y: n(4) as u8,
            })],
            Some("trade") => vec![KnowEvent::ShopOpened {
                scene: n(1) as u8,
                trade: Trade::from_code(n(2) as u8).unwrap(),
                keeper: None,
            }],
            Some("keeper") => vec![KnowEvent::ShopOpened {
                scene: n(1) as u8,
                trade: Trade::from_code(n(2) as u8).unwrap(),
                keeper: Some((n(3) as i8, n(4) as u8, n(5) as u8)),
            }],
            Some("link") => vec![KnowEvent::TownLinkUsed(TownLink {
                scene: n(1) as u8,
                floor: n(2) as i8,
                x: n(3) as u8,
                y: n(4) as u8,
                dir: if words[5] == "up" {
                    LinkDir::Up
                } else {
                    LinkDir::Down
                },
            })],
            Some("exit") => vec![KnowEvent::TownExit(ExitRun {
                scene: n(1) as u8,
                side: match words[2] {
                    "N" => EdgeSide::North,
                    "E" => EdgeSide::East,
                    "S" => EdgeSide::South,
                    _ => EdgeSide::West,
                },
                at: n(3) as u8,
            })],
            Some("sides") => (0..u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT))
                .map(|index| KnowEvent::SideCellSeen {
                    scene: n(1) as u8,
                    index,
                })
                .collect(),
            Some("cell") => {
                let mut rows = [0; 8];
                rows[n(4) as usize] = 1 << n(3);
                vec![KnowEvent::DungeonCellsSeen {
                    record: n(1) as u8,
                    level: n(2) as u8,
                    rows,
                }]
            }
            Some("mark") => vec![KnowEvent::DungeonMarked(DungeonMark {
                record: n(1) as u8,
                level: n(2) as u8,
                x: n(3) as u8,
                y: n(4) as u8,
                kind: [
                    MarkKind::Ladder,
                    MarkKind::Fountain,
                    MarkKind::Chest,
                    MarkKind::Trap,
                    MarkKind::SecretDoor,
                ][n(5) as usize],
            })],
            _ => vec![],
        };
        for event in &events {
            know.apply(event);
        }
    }
    know
}

/// With `U5_MOCKUP_PROFILE` naming a game copy: the marks and "?" tags
/// (M3-15) over the mockups' O, T and D scenes, from the record in
/// `U5_GEM_KNOWLEDGE` (empty when unset). Invariants only: every "?" lies
/// on a shown cell, and nothing undiscovered has a label. With
/// `U5_GEM_SHOTS`, the renders are written there as PPM files.
#[test]
fn real_assets_render_marks() {
    use u5_automap::StaticMaps;
    use u5_automap::gem::{DungeonInputs, TownInputs, gem_features, town_world_plane};
    use u5_runtime::enhancements::GlyphPolicy;
    let Some(dir) = env_dir("U5_MOCKUP_PROFILE") else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let atlas = u5_runtime::load_tile_atlas(&dir, TileGraphicsDepth::Ega16).unwrap();
    let art = MapArt::build(&atlas).with_fonts(
        u5_runtime::load_ibm_ch_font(&dir).unwrap(),
        u5_runtime::load_runes_ch_font(&dir).unwrap(),
    );
    let know = demo_knowledge();
    let shots = env_dir("U5_GEM_SHOTS");
    let outdoors = real_world(&dir, (76, 108), (12, 6));
    let britain = real_britain(&dir);
    let despise = real_despise(&dir);
    let mut maps = StaticMaps::new(&dir);
    let world = maps
        .world_map(town_world_plane(&britain).unwrap())
        .unwrap()
        .to_vec();
    let scenes: [(&str, &PlayState, GemCanvas); 8] = [
        ("O1-169", &outdoors, CANVASES[1]),
        ("O2-219", &outdoors, CANVASES[2]),
        ("O3-43", &outdoors, CANVASES[0]),
        ("T1-169", &britain, CANVASES[1]),
        ("T3-43", &britain, CANVASES[0]),
        ("D1-169", &despise, CANVASES[1]),
        ("D2-219", &despise, CANVASES[2]),
        ("D3-43", &despise, CANVASES[0]),
    ];
    for (name, state, canvas) in scenes {
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
        for feature in gem_features(&view.shown, &know) {
            assert!(feature.known() || feature.label().is_none());
        }
        let mut out = GemImage::default();
        assert_ne!(
            render_gem_map(state, &know, &art, &on(), &view, &mut out),
            GemRender::Cleared
        );
        if let Some(shots) = &shots {
            write_ppm(shots, &format!("engine-marks-{name}.ppm"), &out);
        }
    }
}
