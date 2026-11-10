//! Review shots through the whole shell (build plan M1-16): the approved
//! c04f mockup scenes played in the real `--visual` app, headless, with the
//! settings a host resolves for `--grow-left` from the shipped catalogue,
//! and the presented image read back from the presentation pipeline. The
//! composer-level shots (`u5-tui/tests/grow_left_shots.rs`) draw the same
//! scenes by calling the compositor directly; these prove the app draws
//! them the same way, with the keys typed through Bevy's keyboard path.
//!
//! Review output only, never committed: skipped unless `U5_MOCKUP_PROFILE`
//! (the mockups' game-data copy), `U5_GROW_MOCKUPS` (the c04f folder) and
//! `U5_GROW_SHOTS` (the output folder) are set. Each shot is written as
//! `shell-<name>.png`, with `shell-<name>-side-by-side.png` (mockup above,
//! shell below, 2x), and the differing pixels are counted per region; where
//! the composer-level shot `<name>-engine.png` is in the output folder, the
//! shell's image is compared with it too.

use std::fs;
use std::path::Path;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, PixelAspect, ResolveInput, resolve,
};
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{
    GameClock, PlayState, PlayTarget, SAVE_CHARACTER_NAME_LEN, load_play_options_from_save,
};

use super::grow::column::ColumnState;
use super::grow::{Canvas, compose};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline};
use crate::enh::present::PresentHost;

const W16: (u32, u32) = (2560, 1440);
const W21: (u32, u32) = (3440, 1440);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Party {
    Save,
    IoloPoisoned,
    Six,
}

struct Shot {
    name: &'static str,
    mockup: &'static str,
    window: (u32, u32),
    party: Party,
    /// Ctrl+P after the walk, and the effect left to finish.
    column: bool,
    centre: bool,
}

const fn shot(name: &'static str, mockup: &'static str, window: (u32, u32)) -> Shot {
    Shot {
        name,
        mockup,
        window,
        party: Party::Save,
        column: false,
        centre: false,
    }
}

const SHOTS: [Shot; 9] = [
    shot("B-16x9", "c04f-B-S1-crawl.png", W16),
    shot("B-21x9", "c04f-uw-B-S1-crawl.png", W21),
    Shot {
        party: Party::IoloPoisoned,
        ..shot("G-16x9", "c04f-G-status.png", W16)
    },
    Shot {
        party: Party::IoloPoisoned,
        ..shot("G-21x9", "c04f-uw-G-status.png", W21)
    },
    Shot {
        party: Party::Six,
        ..shot("G6-16x9", "c04f-G6-status.png", W16)
    },
    Shot {
        party: Party::Six,
        ..shot("G6-21x9", "c04f-uw-G6-status.png", W21)
    },
    Shot {
        column: true,
        ..shot("D-16x9", "c04f-D-S1-open.png", W16)
    },
    Shot {
        column: true,
        ..shot("D-21x9", "c04f-uw-D-S1-open.png", W21)
    },
    Shot {
        column: true,
        centre: true,
        ..shot("D-centred-16x9", "c04f-D-centred.png", W16)
    },
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

fn launch(centre: bool) -> VisualLaunchConfig {
    let mut cli = vec![CliSetting::Enable("display.grow_left".into())];
    if centre {
        cli.push(CliSetting::Enable("display.centre_party".into()));
    }
    let enhancements = resolve(ResolveInput {
        stored: None,
        cli: &cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

fn save_png(path: &Path, w: u32, h: u32, rgba: Vec<u8>) {
    image::RgbaImage::from_raw(w, h, rgba)
        .unwrap()
        .save(path)
        .unwrap();
}

/// S1 of the mockups: the forest road west of Britain at 11:50, walked
/// up, up, right, right, up through the shell's keyboard.
#[test]
fn grow_left_shell_review_shots() {
    let (Some(profile), Some(mockups), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_GROW_MOCKUPS"),
        configured_test_dir("U5_GROW_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE, U5_GROW_MOCKUPS and U5_GROW_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("display-shell-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    for shot in &SHOTS {
        let mut options = load_play_options_from_save(&game).unwrap();
        options.target = PlayTarget::from_key("BRITANNIA").unwrap();
        options.saved_active_objects = None;
        options.start = Some((74, 111));
        options.clock = GameClock::new(11, 50).unwrap();
        let mut state = PlayState::load_scene(&game, options).unwrap();
        edit_party(&mut state, shot.party);

        let walk = [
            KeyCode::ArrowUp,
            KeyCode::ArrowUp,
            KeyCode::ArrowRight,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
        ];
        let mut timeline = Timeline::new(10 + walk.len() * 30 + 80);
        for (index, key) in walk.iter().enumerate() {
            timeline.game_tap(10 + index * 30, *key);
        }
        if shot.column {
            let at = 10 + walk.len() * 30 + 10;
            for (frame, key, state) in [
                (at, KeyCode::ControlLeft, ButtonState::Pressed),
                (at, KeyCode::KeyP, ButtonState::Pressed),
                (at + 1, KeyCode::KeyP, ButtonState::Released),
                (at + 1, KeyCode::ControlLeft, ButtonState::Released),
            ] {
                timeline.event(frame, key, state, false, Owner::Feature);
            }
        }
        let mut shell = Shell::new(state, &scene, &launch(shot.centre), |_| {});
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(shot.window.0 as f32, shot.window.1 as f32),
            ..default()
        });
        for index in 0..timeline.len() {
            shell.step(timeline.frame(index));
        }
        let view = shell.app.world().resource::<ColumnState>().view();
        assert_eq!(view.open, shot.column, "{}", shot.name);
        assert_eq!(view.step, None, "{}: the effect has finished", shot.name);
        let world = shell.app.world();
        let handle = world
            .resource::<PresentHost>()
            .presented()
            .expect("the canvas is presented")
            .clone();
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        let (w, h) = (image.size().x, image.size().y);
        let ours = image.data.clone().unwrap();
        let canvas = Canvas::for_window(shot.window.0, shot.window.1, PixelAspect::Dos).unwrap();
        assert_eq!((w, h), (canvas.w, canvas.h), "{}", shot.name);

        let mockup = image::open(mockups.join(shot.mockup)).unwrap().to_rgba8();
        assert_eq!((mockup.width(), mockup.height()), (w, h), "{}", shot.name);
        let theirs = mockup.into_raw();
        let (cx, cy, side) = compose::core_rect(&canvas.for_column(view, shot.centre));
        let (mx0, my0, mx1, my1) = canvas.map_window();
        let (mut bands, mut core, mut ring) = (0, 0, 0);
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if ours[i..i + 3] == theirs[i..i + 3] {
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
        let composer = out.join(format!("{}-engine.png", shot.name));
        let against_composer = image::open(&composer).ok().map(|image| {
            let theirs = image.to_rgba8().into_raw();
            ours.chunks(4)
                .zip(theirs.chunks(4))
                .filter(|(a, b)| a[..3] != b[..3])
                .count()
        });
        eprintln!(
            "shell {}: {w}x{h} canvas; against the mockup: bands {bands}, core {core}, ring \
             {ring}; against the composer shot: {}",
            shot.name,
            against_composer.map_or("not rendered".to_string(), |n| format!("{n} pixels"))
        );
        save_png(
            &out.join(format!("shell-{}.png", shot.name)),
            w,
            h,
            ours.clone(),
        );
        // Mockup above, the shell below, 2x.
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
            .save(out.join(format!("shell-{}-side-by-side.png", shot.name)))
            .unwrap();
    }
    let _ = fs::remove_dir_all(&game);
}
