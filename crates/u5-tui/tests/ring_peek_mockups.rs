//! Phase B M1-07: the ring peek against the approved mockup scenes S1-S4
//! (design 2.3.6, P3 tests).
//!
//! Each scene is rebuilt at test time from a local game-data copy, exactly
//! as the mockup renderer built it: the profile's saved party, the scene
//! loader with the scene's place, clock, transport and torch, then the key
//! script through the engine's own play-script runner. The faithful frame
//! is composed, the peek is taken for every approved `scaled` wide render
//! of the scene, and the ring is rasterised and compared cell by cell with
//! the approved PNG.
//!
//! The ring may differ from the approved render only in the cases design
//! 2.7 lists:
//!
//! * **visibility seams**: a ring cell the approved full re-carve reached
//!   and the second-stage flood does not (the flood is seeded only from the
//!   faithful core edge and never routes round a wall through cells outside
//!   the core). The ring is then black where the render shows a tile; the
//!   conservative side.
//! * **variant 0**: the approved render drew a seated actor's variant from
//!   the shared stream; the peek always takes variant 0.
//!
//! Anything else (a lit cell the approved render shows black, or a lit cell
//! with different pixels) fails the test. Nothing from the game data is
//! committed: the test reads the profile and the approved renders from the
//! paths in `U5_MOCKUP_PROFILE` and `U5_MOCKUP_BASE`, and is skipped when
//! either is unset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use u5_runtime::*;
use u5_tui::{
    complete_headless_blocking_presentations, compose_gameplay_screen, parse_start_arg,
    parse_time_arg, parse_transport_arg, replay_play_script_commands, split_play_script,
};

const UP: &str = "\x1b[A";
const RIGHT: &str = "\x1b[C";
const LEFT: &str = "\x1b[D";

struct Scene {
    name: &'static str,
    target: &'static str,
    floor: Option<i8>,
    at: &'static str,
    time: &'static str,
    transport: Option<&'static str>,
    torch: Option<u8>,
    script: &'static [&'static str],
}

/// The standard scenes of the mockups README (`compose.py` `SCENES`): the
/// four the design names, and the castle hall and Britain at night, which
/// put townsfolk and lamp light in the ring.
const SCENES: [Scene; 6] = [
    Scene {
        name: "S1",
        target: "BRITANNIA",
        floor: None,
        at: "74,111",
        time: "11:50",
        transport: None,
        torch: None,
        script: &[UP, UP, RIGHT, RIGHT, UP],
    },
    Scene {
        name: "S2",
        target: "TOWNE:1",
        floor: None,
        at: "15,19",
        time: "12:00",
        transport: None,
        torch: None,
        script: &[UP, UP],
    },
    Scene {
        name: "S3",
        target: "BRITANNIA",
        floor: None,
        at: "74,111",
        time: "21:50",
        transport: None,
        torch: Some(90),
        script: &[UP, UP, RIGHT, RIGHT, UP],
    },
    Scene {
        name: "S4",
        target: "BRITANNIA",
        floor: None,
        at: "86,113",
        time: "12:00",
        transport: Some("ship"),
        torch: None,
        script: &[LEFT, LEFT],
    },
    Scene {
        name: "S2b",
        target: "CASTLE:0",
        floor: Some(0),
        at: "15,24",
        time: "12:00",
        transport: None,
        torch: None,
        script: &[UP, UP],
    },
    Scene {
        name: "S3b",
        target: "TOWNE:1",
        floor: None,
        at: "15,19",
        time: "22:00",
        transport: None,
        torch: None,
        script: &[UP, UP],
    },
];

fn env_dir(name: &str) -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os(name)?);
    dir.is_dir().then_some(dir)
}

fn load_scene(profile: &Path, scene: &Scene, atlas: &TileAtlas) -> PlayState {
    let mut options = load_play_options_from_save(profile).unwrap();
    options.target = PlayTarget::from_key(scene.target).unwrap();
    options.saved_active_objects = None;
    options.start = Some(parse_start_arg(scene.at).unwrap());
    options.clock = parse_time_arg(scene.time).unwrap();
    if let Some(transport) = scene.transport {
        options.transport = parse_transport_arg(transport).unwrap();
    }
    if let Some(floor) = scene.floor {
        options.floor = floor;
    }
    if let Some(torch) = scene.torch {
        options.torch_counter = torch;
    }
    let mut state = PlayState::load_scene(profile, options).unwrap();
    let script: Vec<&str> = scene.script.to_vec();
    let commands = split_play_script(&script.join(";"));
    replay_play_script_commands(&mut state, profile, &commands, |_, _, _| Ok(())).unwrap();
    complete_headless_blocking_presentations(&mut state, Some(atlas)).unwrap();
    state
}

/// The approved renders the mockups were composed from: every `scaled`
/// wide map of the scene that reaches past the core.
fn approved_windows(base: &Path, scene: &str) -> Vec<(PeekWindow, PathBuf)> {
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(base).unwrap() {
        let path = entry.unwrap().path();
        let Some(file) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(size) = file
            .strip_prefix(&format!("{scene}_wide_"))
            .and_then(|rest| rest.strip_suffix("_scaled.png"))
        else {
            continue;
        };
        let Some((cols, rows)) = size.split_once('x') else {
            continue;
        };
        let (Ok(cols), Ok(rows)) = (cols.parse::<u8>(), rows.parse::<u8>()) else {
            continue;
        };
        let window = PeekWindow {
            west: cols / 2,
            east: cols / 2,
            north: rows / 2,
            south: rows / 2,
        };
        if window.is_valid() && (cols > 11 || rows > 11) {
            found.insert((cols, rows), (window, path));
        }
    }
    found.into_values().collect()
}

/// RGB of one 16x16 cell, with the windowed game's brown for palette slot
/// six, as the approved renders were saved.
fn cell_rgb(rgba: &[u8], width: usize, col: usize, row: usize) -> Vec<[u8; 3]> {
    let mut out = Vec::with_capacity(TILE_ATLAS_SIDE * TILE_ATLAS_SIDE);
    for y in 0..TILE_ATLAS_SIDE {
        for x in 0..TILE_ATLAS_SIDE {
            let i = ((row * TILE_ATLAS_SIDE + y) * width + col * TILE_ATLAS_SIDE + x) * 4;
            let rgb = [rgba[i], rgba[i + 1], rgba[i + 2]];
            out.push(if rgb == SHIPPED_PALETTE_DARK_YELLOW {
                STOCK_EGA_BROWN
            } else {
                rgb
            });
        }
    }
    out
}

fn is_black(cell: &[[u8; 3]]) -> bool {
    cell.iter().all(|rgb| *rgb == [0, 0, 0])
}

/// The pixels the faithful blit would draw for atlas id `id`.
fn tile_rgb(state: &PlayState, atlas: &TileAtlas, id: u16) -> Vec<[u8; 3]> {
    let window = PeekWindow {
        west: 5,
        east: 6,
        north: 5,
        south: 5,
    };
    let mut ring = state
        .peek_top_down_ring(window, PeekLightPolicy::Scaled)
        .expect("a top-down scene");
    let index = window.index_of(6, 0).unwrap();
    ring.cells[index] = Some(PeekCell {
        tile: id as u8,
        shade: PeekShade::Lit,
        atlas: Some(id),
    });
    let raster = state.render_peek_ring(&ring, atlas).unwrap();
    cell_rgb(&raster.to_rgba(), raster.width, window.columns() - 1, 5)
}

/// Review output only (`U5_MOCKUP_SHOTS`): the ring raster with the core
/// copied from the faithful frame's viewport, as the compositor will place
/// it, saved as `<scene>_ring_<cols>x<rows>.png` with the brown remap.
fn save_shot(dir: &Path, scene: &str, window: PeekWindow, ring: &[u8], faithful: &[u8]) {
    const FRAME_WIDTH: usize = 320;
    const VIEWPORT_ORIGIN: usize = 8;
    let width = window.columns() * TILE_ATLAS_SIDE;
    let height = window.rows() * TILE_ATLAS_SIDE;
    let mut out = ring.to_vec();
    let core_x = usize::from(window.west - 5) * TILE_ATLAS_SIDE;
    let core_y = usize::from(window.north - 5) * TILE_ATLAS_SIDE;
    for y in 0..11 * TILE_ATLAS_SIDE {
        for x in 0..11 * TILE_ATLAS_SIDE {
            let from = ((VIEWPORT_ORIGIN + y) * FRAME_WIDTH + VIEWPORT_ORIGIN + x) * 4;
            let to = ((core_y + y) * width + core_x + x) * 4;
            out[to..to + 4].copy_from_slice(&faithful[from..from + 4]);
        }
    }
    for px in out.as_chunks_mut::<4>().0 {
        if px[..3] == SHIPPED_PALETTE_DARK_YELLOW {
            px[..3].copy_from_slice(&STOCK_EGA_BROWN);
        }
    }
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!(
        "{scene}_ring_{}x{}.png",
        window.columns(),
        window.rows()
    ));
    image::RgbaImage::from_raw(width as u32, height as u32, out)
        .unwrap()
        .save(path)
        .unwrap();
}

#[derive(Default, Debug)]
struct Tally {
    compared: usize,
    same: usize,
    seam: usize,
    variant: usize,
    bad: Vec<String>,
}

#[test]
fn the_ring_matches_the_approved_mockup_scenes() {
    let (Some(profile), Some(base)) = (env_dir("U5_MOCKUP_PROFILE"), env_dir("U5_MOCKUP_BASE"))
    else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MOCKUP_BASE");
        return;
    };
    let atlas = load_tile_atlas(&profile, TileGraphicsDepth::Ega16).unwrap();
    let ibm = load_ibm_ch_font(&profile).unwrap();
    let runes = load_runes_ch_font(&profile).unwrap();
    let mut windows_checked = 0;
    for scene in &SCENES {
        let mut state = load_scene(&profile, scene, &atlas);
        // The faithful render first: the peek reads the buffers it leaves.
        let faithful = compose_gameplay_screen(&mut state, &profile, &atlas, &ibm, &runes).unwrap();
        assert!(state.top_down_world_view_active(), "{}", scene.name);
        let windows = approved_windows(&base, scene.name);
        assert!(
            !windows.is_empty(),
            "no approved renders for {}",
            scene.name
        );
        for (window, path) in windows {
            let approved = image::open(&path).unwrap().to_rgba8();
            assert_eq!(
                approved.width() as usize,
                window.columns() * TILE_ATLAS_SIDE,
                "{}",
                path.display()
            );
            let approved_width = approved.width() as usize;
            let approved = approved.into_raw();
            let ring = state
                .peek_top_down_ring(window, PeekLightPolicy::Scaled)
                .unwrap();
            let raster = state.render_peek_ring(&ring, &atlas).unwrap();
            let ours = raster.to_rgba();
            if let Some(shots) = std::env::var_os("U5_MOCKUP_SHOTS") {
                save_shot(Path::new(&shots), scene.name, window, &ours, &faithful);
            }
            let mut tally = Tally::default();
            for (index, cell) in ring.cells.iter().enumerate() {
                let Some(cell) = cell else {
                    continue;
                };
                let (dx, dy) = window.offset_of(index).unwrap();
                let (col, row) = (index % window.columns(), index / window.columns());
                let mine = cell_rgb(&ours, raster.width, col, row);
                let theirs = cell_rgb(&approved, approved_width, col, row);
                tally.compared += 1;
                if mine == theirs {
                    tally.same += 1;
                    continue;
                }
                match (cell.shade, cell.atlas) {
                    (PeekShade::Hidden, _) => tally.seam += 1,
                    (PeekShade::Lit, Some(id))
                        if id >= 256
                            && (1..4).any(|k| tile_rgb(&state, &atlas, id + k) == theirs) =>
                    {
                        tally.variant += 1
                    }
                    _ if is_black(&theirs) => tally.bad.push(format!(
                        "({dx}, {dy}) atlas {:?}: lit here, black in the approved render",
                        cell.atlas
                    )),
                    _ => tally.bad.push(format!(
                        "({dx}, {dy}) atlas {:?}: different pixels",
                        cell.atlas
                    )),
                }
            }
            let lit = ring
                .cells
                .iter()
                .flatten()
                .filter(|c| c.shade == PeekShade::Lit);
            let actors = lit.clone().filter(|c| c.atlas.is_some_and(|id| id >= 256));
            eprintln!(
                "{} {}x{}: {} ring cells ({} lit, {} actors), {} same, {} seam, {} variant, {} other",
                scene.name,
                window.columns(),
                window.rows(),
                tally.compared,
                lit.count(),
                actors.count(),
                tally.same,
                tally.seam,
                tally.variant,
                tally.bad.len()
            );
            assert!(
                tally.bad.is_empty(),
                "{} {}x{}: {:#?}",
                scene.name,
                window.columns(),
                window.rows(),
                tally.bad
            );
            windows_checked += 1;
        }
    }
    assert!(windows_checked >= SCENES.len());
}
