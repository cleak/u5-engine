//! The P8 timing check (Phase B design 2.4.3, 2.7 "Frame time and idle-tick
//! interleaving"; build plan M1-16, with chapter 9's compositing and
//! brightness added by M2-08): with every display feature on at 4K,
//! the shell's frames stay short enough that the gameplay pump
//! (`AnimationPump`) never has to catch up, so the logic tick count over any
//! stretch of wall time is the 1.0.1 count.
//!
//! The pump fires at most once per host frame and keeps at most one
//! interval of debt (`advance_gameplay_animation_pump`). So a frame shorter
//! than one world tick (about 55 ms) can never drop a tick, whatever the
//! frame costs; a longer one can. The first test pins that rule on the
//! pump itself; the second measures the frame cost with everything on at
//! 3840x2160 and checks it against the rule.

use std::time::{Duration, Instant};

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{CliSetting, HostCapabilities, LockState, ResolveInput, resolve};
use u5_runtime::test_fixtures::{configured_original_asset_dir, open_grid, test_state};
use u5_runtime::{GameClock, PlayOptions, PlayState, PlayTarget, WorldPlane};

use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline};
use crate::{AnimationPump, GAMEPLAY_WORLD_TICK_INTERVAL_SECS, advance_gameplay_animation_pump};

/// Ticks the pump fires over `wall` seconds of frames `frame` seconds long.
fn ticks_over(wall: f64, frame: f64) -> u64 {
    let mut pump = AnimationPump::default();
    let interval = GAMEPLAY_WORLD_TICK_INTERVAL_SECS;
    let (mut elapsed, mut ticks) = (0.0, 0);
    while elapsed + frame <= wall {
        elapsed += frame;
        if advance_gameplay_animation_pump(&mut pump, frame as f32, interval, false) {
            ticks += 1;
        }
    }
    ticks
}

#[test]
fn a_frame_shorter_than_a_tick_never_costs_the_pump_a_tick() {
    let interval = f64::from(GAMEPLAY_WORLD_TICK_INTERVAL_SECS);
    let wall = 60.0;
    let expected = (wall / interval).floor() as u64;
    // 60 Hz and 144 Hz, with nothing added and with up to just under one
    // interval of compose cost on top: the same ticks per minute (one may
    // fall either side of the minute's end).
    for refresh in [60.0, 144.0] {
        for cost_ms in [0.0, 1.0, 4.0, 10.0, 20.0, 30.0] {
            let frame = 1.0 / refresh + cost_ms / 1000.0;
            assert!(frame < interval);
            let ticks = ticks_over(wall, frame);
            assert!(
                ticks.abs_diff(expected) <= 1,
                "{refresh} Hz + {cost_ms} ms: {ticks} ticks, {expected} expected"
            );
        }
    }
    // The control: frames longer than a tick lose ticks, so the rule is the
    // real bound.
    assert!(ticks_over(wall, interval * 1.5) + 10 < expected);
}

/// The longest frame and the mean, over `frames` frames after the first
/// ten.
fn frame_costs(
    state: &PlayState,
    scene: &Scene,
    launch: &VisualLaunchConfig,
    timeline: &Timeline,
) -> (Duration, Duration) {
    let mut shell = Shell::new(state.clone(), scene, launch, |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(3840.0, 2160.0),
        ..default()
    });
    let mut costs = Vec::with_capacity(timeline.len());
    for index in 0..timeline.len() {
        let start = Instant::now();
        shell.step(timeline.frame(index));
        costs.push(start.elapsed());
    }
    let measured = &costs[10.min(costs.len())..];
    let max = measured.iter().copied().max().unwrap_or_default();
    let mean = measured.iter().sum::<Duration>() / measured.len().max(1) as u32;
    (max, mean)
}

/// Everything on at 4K (sharp scaling, the ribbon fill, sprite compositing
/// and brightness too), the column
/// opened and closed by Ctrl+P (with its effect) while the player walks: in
/// an optimised build no frame comes near a world tick
/// (`cargo test --release -p u5-bevy --lib timing_tests`). Every
/// frame is timed whole (the game's own work, the compose, the crawl and
/// the effect), so the bound is conservative. Uses the real files when
/// `U5_TEST_ASSET_DIR` names a copy, else the synthetic town.
#[test]
fn every_feature_on_at_4k_keeps_each_frame_under_a_tick() {
    let real = configured_original_asset_dir().filter(|dir| dir.join("BRIT.DAT").exists());
    let (state, scene) = match &real {
        Some(dir) => (
            PlayState::load_scene(
                dir,
                PlayOptions {
                    target: PlayTarget::World(WorldPlane::Britannia),
                    start: Some((86, 105)),
                    clock: GameClock::new(12, 0).unwrap(),
                    ..PlayOptions::default()
                },
            )
            .unwrap(),
            Scene::from_game_dir(dir).unwrap(),
        ),
        None => (test_state(open_grid(), 5, 5), Scene::synthetic()),
    };
    let on = |key: &str| CliSetting::Enable(key.into());
    let settings = [
        on("display.grow_left"),
        on("display.crawl"),
        on("display.typed_colour"),
        on("display.ribbon_hp_colours"),
        on("display.dither"),
        on("display.centre_party"),
        on("display.fullscreen_key"),
        // Chapter 9 (build plan M2-08): compositing with its outline, and
        // the suggested brightness over the whole frame (the larger region).
        on("display.sprite_composite"),
        CliSetting::Set {
            key: "display.brightness".into(),
            value: "70".into(),
        },
        CliSetting::Set {
            key: "display.brightness_scope".into(),
            value: "frame".into(),
        },
        CliSetting::Set {
            key: "display.scale_mode".into(),
            value: "sharp".into(),
        },
        CliSetting::Set {
            key: "display.pillar_fill".into(),
            value: "ribbon".into(),
        },
    ];
    let enhancements = resolve(ResolveInput {
        stored: None,
        cli: &settings,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(enhancements.notes.is_empty());
    let launch = VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    };
    let keys = [
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::Space,
        KeyCode::ArrowUp,
    ];
    let mut timeline = Timeline::new(240);
    for (index, key) in keys.iter().enumerate() {
        timeline.game_tap(10 + index * 30, *key);
        // Ctrl+P between the steps, so the effect runs over the walk.
        for (at, code, state) in [
            (25 + index * 30, KeyCode::ControlLeft, ButtonState::Pressed),
            (25 + index * 30, KeyCode::KeyP, ButtonState::Pressed),
            (26 + index * 30, KeyCode::KeyP, ButtonState::Released),
            (26 + index * 30, KeyCode::ControlLeft, ButtonState::Released),
        ] {
            timeline.event(at, code, state, false, Owner::Feature);
        }
    }
    let (off_max, off_mean) = frame_costs(
        &state,
        &scene,
        &VisualLaunchConfig::classic(),
        &timeline.without_feature(),
    );
    let (on_max, on_mean) = frame_costs(&state, &scene, &launch, &timeline);
    eprintln!(
        "4K frame cost ({}): Classic max {off_max:?} mean {off_mean:?}; \
         everything on max {on_max:?} mean {on_mean:?}; one tick {:?}",
        if real.is_some() {
            "real files"
        } else {
            "synthetic"
        },
        Duration::from_secs_f32(GAMEPLAY_WORLD_TICK_INTERVAL_SECS)
    );
    // The bound is for the player's (optimised) build: the longest frame
    // stays under one tick, so the pump, which fires at most once a frame,
    // never falls behind. (A vsynced frame lasts the longer of the refresh
    // and its work, not their sum.) An unoptimised test build only reports
    // its numbers: in debug the 4K sharp prescale alone takes tens of
    // milliseconds.
    let tick = Duration::from_secs_f32(GAMEPLAY_WORLD_TICK_INTERVAL_SECS);
    if cfg!(debug_assertions) {
        return;
    }
    assert!(on_max < tick, "{on_max:?} at 4K");
}
