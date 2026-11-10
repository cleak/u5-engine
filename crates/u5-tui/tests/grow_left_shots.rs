//! Phase B M1-09 and M1-10 review shots: the grow-left canvas (chrome, ring,
//! faithful core, the ribbon and the tabs) for the approved mockup scenes,
//! rendered by the engine's own frame renderer and compositor next to the
//! approved c04f mockups.
//!
//! The G and G6 shots edit the party as the mockups do (Iolo poisoned; for
//! G6 three more members with the mockup's names and numbers) before the
//! frame is drawn, so the ribbon can be compared cell for cell.
//!
//! Review output only, never committed: the scene is rebuilt at test time
//! from a local game-data copy (`U5_MOCKUP_PROFILE`), exactly as the
//! mockups and the M1-07 ring test build it, and the shell's faithful frame
//! and its canvas come from `render_shell_frame_and_canvas`: the shell's
//! play-frame render and `compose_for_state`, the function the
//! `[grow-left compose]` step runs. Each shot is written to
//! `U5_GROW_SHOTS` as `<name>-engine.png`, next to a copy of the mockup and
//! a side-by-side strip, and the differing pixels are counted per region.
//! Skipped unless `U5_MOCKUP_PROFILE`, `U5_GROW_MOCKUPS` (the c04f folder)
//! and `U5_GROW_SHOTS` are set.

#![cfg(feature = "visual")]

use std::path::{Path, PathBuf};

use u5_bevy::display::grow::{Canvas, compose, render_shell_frame_and_canvas};
use u5_runtime::enhancements::PixelAspect;
use u5_runtime::*;
use u5_tui::{
    complete_headless_blocking_presentations, parse_start_arg, parse_time_arg,
    replay_play_script_commands, split_play_script,
};

const UP: &str = "\x1b[A";
const RIGHT: &str = "\x1b[C";

/// How the shot edits the save's party (the mockups' G and G6 states).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Party {
    /// The save's own three members.
    Save,
    /// Iolo poisoned (G).
    IoloPoisoned,
    /// Iolo poisoned and three more members (G6).
    Six,
}

struct Shot {
    name: &'static str,
    mockup: &'static str,
    window: (u32, u32),
    time: &'static str,
    torch: Option<u8>,
    party: Party,
}

const fn shot(name: &'static str, mockup: &'static str, window: (u32, u32), party: Party) -> Shot {
    Shot {
        name,
        mockup,
        window,
        time: "11:50",
        torch: None,
        party,
    }
}

const W16: (u32, u32) = (2560, 1440);
const W21: (u32, u32) = (3440, 1440);

const SHOTS: [Shot; 7] = [
    shot("S1-16x9", "c04f-A-S1-quiet.png", W16, Party::Save),
    shot("S1-21x9", "c04f-uw-A-S1-quiet.png", W21, Party::Save),
    Shot {
        name: "S3-16x9",
        mockup: "c04f-E-S3-night.png",
        window: W16,
        time: "21:50",
        torch: Some(90),
        party: Party::Save,
    },
    shot("G-16x9", "c04f-G-status.png", W16, Party::IoloPoisoned),
    shot("G-21x9", "c04f-uw-G-status.png", W21, Party::IoloPoisoned),
    shot("G6-16x9", "c04f-G6-status.png", W16, Party::Six),
    shot("G6-21x9", "c04f-uw-G6-status.png", W21, Party::Six),
];

/// The mockups' party edits (make_c04f.py: G_PARTY, G6_PARTY).
fn edit_party(state: &mut PlayState, party: Party) {
    if party == Party::Save {
        return;
    }
    state.party[2].status = b'P';
    if party == Party::Six {
        for (name, status, hp, max_hp) in [
            (&b"Jaana"[..], b'G', 18, 90),
            (&b"Dupre"[..], b'P', 120, 150),
            (&b"Geoffrey"[..], b'G', 240, 240),
        ] {
            let mut member = state.party[1];
            member.status = status;
            member.hp = hp;
            member.max_hp = max_hp;
            state.party.push(member);
            let mut bytes = [0; SAVE_CHARACTER_NAME_LEN];
            bytes[..name.len()].copy_from_slice(name);
            state.party_names.push(bytes);
        }
    }
    state.repaint_stats_panel();
}

fn env_dir(name: &str) -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os(name)?))
}

/// S1 and S3 of the mockups: the forest road west of Britain, walked
/// up, up, right, right, up.
fn load_scene(profile: &Path, shot: &Shot, atlas: &TileAtlas) -> PlayState {
    let mut options = load_play_options_from_save(profile).unwrap();
    options.target = PlayTarget::from_key("BRITANNIA").unwrap();
    options.saved_active_objects = None;
    options.start = Some(parse_start_arg("74,111").unwrap());
    options.clock = parse_time_arg(shot.time).unwrap();
    if let Some(torch) = shot.torch {
        options.torch_counter = torch;
    }
    let mut state = PlayState::load_scene(profile, options).unwrap();
    let script = [UP, UP, RIGHT, RIGHT, UP].join(";");
    let commands = split_play_script(&script);
    replay_play_script_commands(&mut state, profile, &commands, |_, _, _| Ok(())).unwrap();
    complete_headless_blocking_presentations(&mut state, Some(atlas)).unwrap();
    state
}

fn save(path: &Path, w: u32, h: u32, rgba: Vec<u8>) {
    image::RgbaImage::from_raw(w, h, rgba)
        .unwrap()
        .save(path)
        .unwrap();
}

#[test]
fn grow_left_review_shots() {
    let (Some(profile), Some(mockups), Some(out)) = (
        env_dir("U5_MOCKUP_PROFILE"),
        env_dir("U5_GROW_MOCKUPS"),
        env_dir("U5_GROW_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE, U5_GROW_MOCKUPS and U5_GROW_SHOTS");
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    let atlas = load_tile_atlas(&profile, TileGraphicsDepth::Ega16).unwrap();
    let ibm = load_ibm_ch_font(&profile).unwrap();
    let runes = load_runes_ch_font(&profile).unwrap();
    for shot in &SHOTS {
        let mut state = load_scene(&profile, shot, &atlas);
        edit_party(&mut state, shot.party);
        let canvas = Canvas::for_window(shot.window.0, shot.window.1, PixelAspect::Dos).unwrap();
        // The shell's own frame and canvas: the bytes the play upload
        // sites write, in the shell's palette.
        let (_, ours) = render_shell_frame_and_canvas(
            &mut state,
            &atlas,
            (&ibm, &runes),
            &profile,
            &canvas,
            PeekLightPolicy::Scaled,
        );
        let ours = ours.expect("the top-down view");
        let (w, h) = (canvas.w, canvas.h);
        let mockup = image::open(mockups.join(shot.mockup)).unwrap().to_rgba8();
        assert_eq!((mockup.width(), mockup.height()), (w, h), "{}", shot.name);
        let theirs = mockup.into_raw();

        // Count differing pixels per region: the chrome bands (the ribbon
        // and the tabs), the core, the ring (where the crawl of a later
        // package goes).
        let (cx, cy, side) = compose::core_rect(&canvas);
        let (mx0, my0, mx1, my1) = canvas.map_window();
        let mut diff = vec![0u8; (w * h * 4) as usize];
        let (mut bands, mut core, mut ring) = (0, 0, 0);
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let same = ours[i..i + 3] == theirs[i..i + 3];
                let px = if same {
                    // Dimmed context.
                    [ours[i] / 4, ours[i + 1] / 4, ours[i + 2] / 4, 0xff]
                } else {
                    [0xff, 0x00, 0xff, 0xff]
                };
                diff[i..i + 4].copy_from_slice(&px);
                if same {
                    continue;
                }
                let in_core = (cx..cx + side).contains(&(x as usize))
                    && (cy..cy + side).contains(&(y as usize));
                let in_map = (mx0..=mx1).contains(&x) && (my0..=my1).contains(&y);
                if in_core {
                    core += 1;
                } else if in_map {
                    ring += 1;
                } else {
                    bands += 1;
                }
            }
        }
        eprintln!(
            "{}: {}x{} canvas; differing pixels: bands {bands}, core {core}, ring {ring}",
            shot.name, w, h
        );
        save(
            &out.join(format!("{}-engine.png", shot.name)),
            w,
            h,
            ours.clone(),
        );
        save(
            &out.join(format!("{}-mockup.png", shot.name)),
            w,
            h,
            theirs.clone(),
        );
        save(&out.join(format!("{}-diff.png", shot.name)), w, h, diff);
        // Side by side, mockup above the engine, 2x.
        let mut strip = image::RgbaImage::new(w * 2, h * 4 + 8);
        for (row, rgba) in [(0, &theirs), (1, &ours)] {
            for y in 0..h * 2 {
                for x in 0..w * 2 {
                    let i = (((y / 2) * w + x / 2) * 4) as usize;
                    strip.put_pixel(
                        x,
                        y + row * (h * 2 + 8),
                        image::Rgba([rgba[i], rgba[i + 1], rgba[i + 2], 0xff]),
                    );
                }
            }
        }
        strip
            .save(out.join(format!("{}-side-by-side.png", shot.name)))
            .unwrap();
    }
}
