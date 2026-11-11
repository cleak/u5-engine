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
    BrightnessScope, CliSetting, HostCapabilities, LockState, PixelAspect, ResolveInput, resolve,
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

/// One brightness review scene of the display mockups (make_display.py,
/// part A): the target, the start, the clock, the torch and the walk.
struct LightScene {
    name: &'static str,
    target: &'static str,
    at: (usize, usize),
    clock: (u8, u8),
    torch: Option<u8>,
    walk: &'static [KeyCode],
}

const S1_WALK: [KeyCode; 5] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowUp,
    KeyCode::ArrowRight,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
];

const LIGHT_SCENES: [LightScene; 3] = [
    LightScene {
        name: "S1",
        target: "BRITANNIA",
        at: (74, 111),
        clock: (11, 50),
        torch: None,
        walk: &S1_WALK,
    },
    LightScene {
        name: "S2",
        target: "TOWNE:1",
        at: (15, 19),
        clock: (12, 0),
        torch: None,
        walk: &[KeyCode::ArrowUp, KeyCode::ArrowUp],
    },
    LightScene {
        name: "S3",
        target: "BRITANNIA",
        at: (74, 111),
        clock: (21, 50),
        torch: Some(90),
        walk: &S1_WALK,
    },
];

/// The grow-left canvas (the crawl on, the column collapsed; the walk's
/// rows show over the map, where the mockups' state A shows only the live
/// prompt) and `brightness` at `percent` over `scope`.
fn light_launch(percent: u8, scope: &str) -> VisualLaunchConfig {
    let cli = vec![
        CliSetting::Enable("display.grow_left".into()),
        CliSetting::Set {
            key: "display.brightness".into(),
            value: percent.to_string(),
        },
        CliSetting::Set {
            key: "display.brightness_scope".into(),
            value: scope.into(),
        },
    ];
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

/// Build plan M2-03: the brightness step through the whole shell, in the
/// display mockups' scenes (S1 overworld by day, S2 Britain, S3 night with
/// a torch) at 16:9, at 100%, 70% on the map and 70% on the whole frame,
/// against `display-A-<scene>-b100.png`, `-map070.png` and `-b070.png`.
/// Review output only: skipped unless `U5_MOCKUP_PROFILE`,
/// `U5_DISPLAY_MOCKUPS` (the display mockups folder) and `U5_LIGHT_SHOTS`
/// (the output folder) are set. Writes `light-<scene>-<variant>.png` and
/// `light-<scene>-<variant>-side-by-side.png` (mockup above, shell below,
/// 2x), and reports the differing pixels per region, and how many pixels
/// the step changed in each image against its own 100% image.
#[test]
fn brightness_shell_review_shots() {
    let (Some(profile), Some(mockups), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_DISPLAY_MOCKUPS"),
        configured_test_dir("U5_LIGHT_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE, U5_DISPLAY_MOCKUPS and U5_LIGHT_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("display-light-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    let canvas = Canvas::for_window(W16.0, W16.1, PixelAspect::Dos).unwrap();
    let (w, h) = (canvas.w, canvas.h);
    for light in &LIGHT_SCENES {
        // One shell per scene, launched at 70% on the map; the menu's
        // changes (the `Display` resource) then give the whole-frame and the
        // 100% images of the same game state, one frame later each.
        let mut options = load_play_options_from_save(&game).unwrap();
        options.target = PlayTarget::from_key(light.target).unwrap();
        options.saved_active_objects = None;
        options.start = Some(light.at);
        options.clock = GameClock::new(light.clock.0, light.clock.1).unwrap();
        if let Some(torch) = light.torch {
            options.torch_counter = torch;
        }
        let state = PlayState::load_scene(&game, options).unwrap();
        let mut timeline = Timeline::new(10 + light.walk.len() * 30 + 40);
        for (index, key) in light.walk.iter().enumerate() {
            timeline.game_tap(10 + index * 30, *key);
        }
        let mut shell = Shell::new(state, &scene, &light_launch(70, "map"), |_| {});
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(W16.0 as f32, W16.1 as f32),
            ..default()
        });
        for index in 0..timeline.len() {
            shell.step(timeline.frame(index));
        }
        // Stop the clock, so the three images show one game state.
        shell
            .app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::ZERO,
            ));
        shell.step(&[]);
        let mut shots: Vec<(&str, Vec<u8>, Vec<u8>)> = Vec::new();
        for (variant, percent, scope) in [
            ("map070", 70, BrightnessScope::Map),
            ("b070", 70, BrightnessScope::Frame),
            ("b100", 100, BrightnessScope::Map),
        ] {
            {
                let mut display = shell.app.world_mut().resource_mut::<super::Display>();
                if display.0.light.brightness != percent || display.0.light.scope != scope {
                    display.0.light.brightness = percent;
                    display.0.light.scope = scope;
                }
            }
            shell.step(&[]);
            let world = shell.app.world();
            let handle = world
                .resource::<PresentHost>()
                .presented()
                .expect("the canvas is presented")
                .clone();
            let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
            assert_eq!((image.size().x, image.size().y), (w, h), "{}", light.name);
            let ours = image.data.clone().unwrap();
            let name = format!("display-A-{}-{variant}.png", light.name);
            let theirs = image::open(mockups.join(&name)).unwrap().to_rgba8();
            assert_eq!((theirs.width(), theirs.height()), (w, h), "{name}");
            shots.push((variant, ours, theirs.into_raw()));
        }
        // The 100% pair is the base each variant is measured against.
        shots.rotate_right(1);
        let (mx0, my0, mx1, my1) = canvas.map_window();
        let base = (shots[0].1.clone(), shots[0].2.clone());
        for (variant, ours, theirs) in &shots {
            let (mut map, mut chrome) = (0, 0);
            // Where the two 100% images agree, the step must agree too.
            let (mut same_base, mut step_differs) = (0, 0);
            let (mut ours_changed, mut theirs_changed) = (0, 0);
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) * 4) as usize;
                    let px = |rgba: &Vec<u8>| [rgba[i], rgba[i + 1], rgba[i + 2]];
                    if px(ours) != px(&base.0) {
                        ours_changed += 1;
                    }
                    if px(theirs) != px(&base.1) {
                        theirs_changed += 1;
                    }
                    if px(&base.0) == px(&base.1) {
                        same_base += 1;
                        if px(ours) != px(theirs) {
                            step_differs += 1;
                        }
                    }
                    if px(ours) == px(theirs) {
                        continue;
                    }
                    if (mx0..=mx1).contains(&x) && (my0..=my1).contains(&y) {
                        map += 1;
                    } else {
                        chrome += 1;
                    }
                }
            }
            eprintln!(
                "light {} {variant}: against the mockup: map {map}, chrome {chrome}; the step \
                 changed {ours_changed} (mockup {theirs_changed}); where the 100% images \
                 agree ({same_base}), {step_differs} differ",
                light.name
            );
            save_png(
                &out.join(format!("light-{}-{variant}.png", light.name)),
                w,
                h,
                ours.clone(),
            );
            let mut strip = image::RgbaImage::new(w * 2, h * 4 + 8);
            for (row, rgba) in [(0, theirs), (1, ours)] {
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
                .save(out.join(format!("light-{}-{variant}-side-by-side.png", light.name)))
                .unwrap();
        }
    }
    let _ = fs::remove_dir_all(&game);
}
