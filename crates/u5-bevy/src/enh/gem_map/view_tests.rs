//! The gem map in the shell (plan M3-11): the node above the frame, the
//! sweep and its steps, the canvas and resize, the fallbacks, and the game
//! equal to the off run frame for frame while the map is up.

use std::time::Duration;

use bevy::window::WindowResolution;
use u5_automap::gem::{GemCanvas, GemImage};
use u5_runtime::PlayState;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, PixelAspect, ResolveInput, resolve,
};
use u5_runtime::test_fixtures::{open_grid, open_world_grid, test_state, world_state};

use super::*;
use crate::enh::equivalence::{FRAME, FrameRecord, Scene, Shell, Timeline};
use crate::enh::{Toggles, VisualLaunchConfig};

// ---------------------------------------------------------------------------
// Helpers.

fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    let resolved = resolve(ResolveInput {
        stored: None,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(resolved.notes.is_empty(), "{:?}", resolved.notes);
    VisualLaunchConfig {
        enhancements: resolved,
        ..VisualLaunchConfig::classic()
    }
}

fn gem_on() -> VisualLaunchConfig {
    launch(&[CliSetting::Enable("gem_map.enhanced".into())])
}

fn gem_no_sweep() -> VisualLaunchConfig {
    launch(&[
        CliSetting::Enable("gem_map.enhanced".into()),
        CliSetting::Disable("gem_map.sweep".into()),
    ])
}

const W169: (f32, f32) = (2560.0, 1440.0);
const W43: (f32, f32) = (1600.0, 1200.0);
const W219: (f32, f32) = (3440.0, 1440.0);

fn spawn_window(app: &mut App, size: (f32, f32)) {
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(size.0, size.1),
        ..default()
    });
}

fn resize(shell: &mut Shell, size: (f32, f32)) {
    let world = shell.app.world_mut();
    let mut windows = world.query::<&mut Window>();
    for mut window in windows.iter_mut(world) {
        window.resolution.set(size.0, size.1);
    }
}

/// Outdoors, on foot, with three gems.
fn outdoors() -> PlayState {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.gems = 3;
    state
}

fn shell(state: PlayState, launch: &VisualLaunchConfig, window: Option<(f32, f32)>) -> Shell {
    let mut shell = Shell::new(state, &Scene::synthetic(), launch, |_| {});
    if let Some(size) = window {
        spawn_window(&mut shell.app, size);
    }
    shell
}

/// Everything compared per frame: the harness record, then the gem count,
/// the view overlay, the idle polls and the random state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Beat {
    record: FrameRecord,
    gems: u8,
    overlay: String,
    idle_polls: u32,
    prng: u16,
}

fn beat(shell: &Shell) -> Beat {
    let state = &shell.visual().state;
    Beat {
        record: shell.record(),
        gems: state.gems,
        overlay: format!("{:?}", state.active_view_overlay),
        idle_polls: state.view_overlay_idle_polls,
        prng: state.prng_state,
    }
}

/// What the map shows after each frame.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    gem_open: bool,
    nodes: usize,
    befores: usize,
    step: Option<u8>,
    sweep_open: bool,
    picture: Option<Vec<u8>>,
}

fn map_nodes(shell: &mut Shell) -> Vec<(Handle<Image>, GlobalZIndex, Node)> {
    let world = shell.app.world_mut();
    let mut query = world.query_filtered::<(&ImageNode, &GlobalZIndex, &Node), With<GemMapNode>>();
    query
        .iter(world)
        .map(|(image, z, node)| (image.image.clone(), *z, node.clone()))
        .collect()
}

fn before_nodes(shell: &mut Shell) -> Vec<(Handle<Image>, GlobalZIndex, Node)> {
    let world = shell.app.world_mut();
    let mut query =
        world.query_filtered::<(&ImageNode, &GlobalZIndex, &Node), With<GemBeforeNode>>();
    query
        .iter(world)
        .map(|(image, z, node)| (image.image.clone(), *z, node.clone()))
        .collect()
}

fn image_bytes(shell: &Shell, handle: &Handle<Image>) -> (UVec2, Vec<u8>) {
    let image = shell
        .app
        .world()
        .resource::<Assets<Image>>()
        .get(handle)
        .expect("the image");
    (image.size(), image.data.clone().unwrap())
}

fn shown(shell: &mut Shell) -> Shown {
    let maps = map_nodes(shell);
    let befores = before_nodes(shell);
    let picture = maps
        .first()
        .map(|(handle, ..)| image_bytes(shell, handle).1);
    let show = shell.app.world().resource::<GemShow>();
    Shown {
        gem_open: gem_view_open(&shell.visual().state),
        nodes: maps.len(),
        befores: befores.len(),
        step: show.sweep.step,
        sweep_open: show.sweep.open,
        picture,
    }
}

fn run(
    state: &PlayState,
    launch: &VisualLaunchConfig,
    window: Option<(f32, f32)>,
    timeline: &Timeline,
) -> (Vec<Beat>, Vec<Shown>) {
    let mut shell = shell(state.clone(), launch, window);
    let mut beats = Vec::new();
    let mut shows = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        beats.push(beat(&shell));
        if shell.app.world().contains_resource::<GemShow>() {
            shows.push(shown(&mut shell));
        }
    }
    (beats, shows)
}

fn first_frame(beats: &[Beat], open: bool) -> Option<usize> {
    beats
        .iter()
        .position(|beat| beat.overlay.contains("GemView") == open)
}

/// The mockup's own sweep mask (`make_gem_final.py`, `sweep_mask`): at
/// step `k` of 8 a pixel shows the map when `0.72 * (H - 1 - y) / H +
/// 0.28 * BAYER4[y % 4][x % 4] / 16 < k / 8`.
fn mockup_shows(x: u32, y: u32, h: u32, k: u8) -> bool {
    const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let base = 0.72 * f64::from(h - 1 - y) / f64::from(h);
    let t = f64::from(k) / 8.0;
    base + 0.28 * f64::from(BAYER4[(y % 4) as usize][(x % 4) as usize]) / 16.0 < t
}

// ---------------------------------------------------------------------------
// Pure parts.

#[test]
fn every_sweep_pixel_lands_on_the_mockup_step() {
    for canvas in [
        GemCanvas { w: 320, h: 200 },
        GemCanvas { w: 512, h: 240 },
        GemCanvas { w: 688, h: 240 },
        GemCanvas { w: 480, h: 216 },
    ] {
        let steps = SweepSteps::new(canvas);
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                let step = steps.at(x, y);
                assert!((1..=8).contains(&step));
                for k in 1..8u8 {
                    assert_eq!(
                        step <= k,
                        mockup_shows(x, y, canvas.h, k),
                        "{canvas:?} ({x}, {y}) step {k}"
                    );
                }
            }
        }
        // It rises: the bottom row arrives first, the top row last.
        let steps = &steps;
        let row = |y: u32| (0..canvas.w).map(move |x| steps.at(x, y));
        assert_eq!(row(canvas.h - 1).min(), Some(1));
        assert_eq!(row(0).max(), Some(8));
        assert!(row(canvas.h - 1).max() <= row(0).min());
    }
}

#[test]
fn the_picture_is_the_map_where_the_sweep_has_arrived() {
    let canvas = GemCanvas { w: 32, h: 24 };
    let full = GemImage {
        w: 32,
        h: 24,
        rgba: (0..32 * 24 * 4).map(|i| (i % 251) as u8 | 1).collect(),
    };
    let steps = SweepSteps::new(canvas);
    let mut out = Vec::new();
    sweep_picture(&full, &steps, None, &mut out);
    assert_eq!(out, full.rgba, "no step: the whole map");
    for k in 1..8u8 {
        sweep_picture(&full, &steps, Some(k), &mut out);
        for y in 0..24 {
            for x in 0..32 {
                let at = ((y * 32 + x) * 4) as usize;
                let expect = if steps.at(x, y) <= k {
                    &full.rgba[at..at + 4]
                } else {
                    &[0, 0, 0, 0][..]
                };
                assert_eq!(&out[at..at + 4], expect, "({x}, {y}) step {k}");
            }
        }
    }
}

#[test]
fn the_sweep_steps_every_60_ms_and_reverses_from_its_step() {
    let mut sweep = GemSweep::default();
    assert!(!sweep.visible());
    sweep.start_open(true);
    assert_eq!((sweep.open, sweep.step), (true, Some(1)));
    // 60 ms at a time: 1, 2, ..., 7, then the whole map.
    for k in 2..8 {
        sweep.tick(GEM_SWEEP_STEP, true);
        assert_eq!(sweep.step, Some(k));
    }
    sweep.tick(GEM_SWEEP_STEP, true);
    assert_eq!((sweep.open, sweep.step), (true, None));
    assert!(sweep.visible());
    // Holding: nothing moves.
    sweep.tick(Duration::from_secs(5), true);
    assert_eq!(sweep.step, None);

    // Closing: 7 down to 1, then nothing.
    sweep.start_close(true);
    assert_eq!(sweep.step, Some(7));
    for k in (1..7).rev() {
        sweep.tick(GEM_SWEEP_STEP, true);
        assert_eq!(sweep.step, Some(k));
    }
    sweep.tick(GEM_SWEEP_STEP, true);
    assert!(!sweep.visible());

    // Reversal: a close at step 3 of the opening goes 3, 2, 1, gone.
    sweep.start_open(true);
    sweep.tick(GEM_SWEEP_STEP * 2, true);
    assert_eq!(sweep.step, Some(3));
    sweep.start_close(true);
    assert_eq!((sweep.open, sweep.step), (false, Some(3)));
    sweep.tick(GEM_SWEEP_STEP, true);
    assert_eq!(sweep.step, Some(2));
    // And back again from there.
    sweep.start_open(true);
    assert_eq!((sweep.open, sweep.step), (true, Some(2)));

    // On the shell's 60 Hz clock a step lasts three or four frames.
    let mut sweep = GemSweep::default();
    sweep.start_open(true);
    let mut frames = vec![0u32; 9];
    let mut frame = 0;
    while let Some(k) = sweep.step {
        frames[k as usize] += 1;
        sweep.tick(FRAME, true);
        frame += 1;
        assert!(frame < 100);
    }
    assert!(
        frames[1..8].iter().all(|n| (3..=4).contains(n)),
        "{frames:?}"
    );
    assert_eq!(frames[1..8].iter().sum::<u32>(), 26, "{frames:?}");

    // Without the sweep: at once, both ways; turning it off ends one.
    let mut sweep = GemSweep::default();
    sweep.start_open(false);
    assert_eq!((sweep.open, sweep.step), (true, None));
    sweep.start_close(false);
    assert!(!sweep.visible());
    sweep.start_open(true);
    sweep.tick(FRAME, false);
    assert_eq!((sweep.open, sweep.step), (true, None));
}

#[test]
fn the_canvas_follows_the_window() {
    let canvas = |w, h| window_gem_canvas(UVec2::new(w, h), PixelAspect::Dos);
    assert_eq!(canvas(2560, 1440), Some(GemCanvas { w: 512, h: 240 }));
    assert_eq!(canvas(1920, 1080), Some(GemCanvas { w: 480, h: 216 }));
    assert_eq!(canvas(3440, 1440), Some(GemCanvas { w: 688, h: 240 }));
    // 4:3 is the original screen (mockup O3), not the foundation's 384x240.
    assert_eq!(canvas(1600, 1200), Some(GemCanvas { w: 320, h: 200 }));
    assert_eq!(canvas(1440, 1080), Some(GemCanvas { w: 320, h: 200 }));
    assert_eq!(
        window_gem_canvas(UVec2::new(1600, 1200), PixelAspect::Square),
        Some(GemCanvas { w: 320, h: 200 })
    );
    // Just wider than 4:3: the foundation's canvas.
    assert_eq!(canvas(1700, 1200), Some(GemCanvas { w: 400, h: 240 }));
    // 16:10 is wider than the DOS screen's 4:3.
    assert_eq!(canvas(640, 400), Some(GemCanvas { w: 448, h: 240 }));
    // Below the foundation's 320x240 minimum: the classic size.
    assert_eq!(canvas(320, 200), Some(GemCanvas { w: 320, h: 200 }));
    assert_eq!(canvas(400, 230), Some(GemCanvas { w: 320, h: 200 }));
    assert_eq!(canvas(319, 200), None);
    assert_eq!(canvas(320, 199), None);
}

// ---------------------------------------------------------------------------
// The node and the sweep in the shell.

fn v_then(frames: usize, more: &[(usize, KeyCode)]) -> Timeline {
    let mut timeline = Timeline::new(frames);
    timeline.game_tap(5, KeyCode::KeyV);
    for &(frame, key) in more {
        timeline.game_tap(frame, key);
    }
    timeline
}

#[test]
fn a_gem_opens_the_map_over_the_whole_window_above_the_frame() {
    let mut shell = shell(outdoors(), &gem_on(), Some(W169));
    let timeline = v_then(60, &[]);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    assert!(gem_view_open(&shell.visual().state), "the gem is up");
    assert_eq!(shell.visual().state.gems, 2, "one gem spent");
    let maps = map_nodes(&mut shell);
    assert_eq!(maps.len(), 1);
    let (handle, z, node) = &maps[0];
    assert_eq!(*z, GlobalZIndex(i32::MAX - 2));
    assert!(z.0 > -1, "above the side fill");
    assert!(z.0 < i32::MAX - 1, "below the toasts");
    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!((node.left, node.top), (Val::Px(0.0), Val::Px(0.0)));
    assert_eq!(
        (node.width, node.height),
        (Val::Percent(100.0), Val::Percent(100.0))
    );
    let (size, bytes) = image_bytes(&shell, handle);
    assert_eq!(size, UVec2::new(512, 240), "the 16:9 canvas");
    assert!(bytes.chunks(4).all(|p| p[3] == 255), "held: opaque");
    assert!(before_nodes(&mut shell).is_empty(), "no before while held");
    // Still a single sprite, and it still shows the faithful frame.
    let world = shell.app.world_mut();
    let mut sprites = world.query::<&Sprite>();
    assert_eq!(sprites.iter(world).count(), 1);
}

#[test]
fn the_open_and_close_sweeps_step_on_the_frame_clock() {
    let timeline = v_then(120, &[(70, KeyCode::Space)]);
    let (beats, shows) = run(&outdoors(), &gem_on(), Some(W169), &timeline);
    let opened = first_frame(&beats, true).expect("the view opens");
    let closed = opened + first_frame(&beats[opened..], false).expect("and closes");
    // The opening: steps 1..=7, three or four frames each, then the map.
    let steps: Vec<Option<u8>> = shows[opened..closed].iter().map(|s| s.step).collect();
    let mut seen: Vec<u8> = steps.iter().flatten().copied().collect();
    seen.dedup();
    assert_eq!(seen, [1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(steps[0], Some(1), "step 1 on the open frame");
    assert!(
        steps
            .iter()
            .skip_while(|s| s.is_some())
            .all(Option::is_none)
    );
    // While opening the before picture is under the map, then gone.
    for show in &shows[opened..closed] {
        assert_eq!(show.nodes, 1);
        assert_eq!(show.befores, usize::from(show.step.is_some()), "{show:?}");
    }
    // Each step's picture is the held map through that step's mask.
    let held = shows[closed - 1].picture.clone().unwrap();
    let canvas = GemCanvas { w: 512, h: 240 };
    let full = GemImage {
        w: 512,
        h: 240,
        rgba: held,
    };
    let sweep = SweepSteps::new(canvas);
    for show in &shows[opened..closed] {
        let mut expect = Vec::new();
        sweep_picture(&full, &sweep, show.step, &mut expect);
        assert_eq!(show.picture.as_ref(), Some(&expect), "{:?}", show.step);
    }
    // The closing: from 7 down to 1 over the live frame, then no node.
    let after: Vec<Option<u8>> = shows[closed..].iter().map(|s| s.step).collect();
    let mut down: Vec<u8> = after.iter().flatten().copied().collect();
    down.dedup();
    assert_eq!(down, [7, 6, 5, 4, 3, 2, 1]);
    for show in &shows[closed..] {
        assert!(!show.sweep_open);
        assert_eq!(show.befores, 0, "closing shows the live frame");
        assert_eq!(show.nodes, usize::from(show.step.is_some()));
    }
    assert_eq!(shows.last().unwrap().nodes, 0);
}

#[test]
fn a_key_during_the_open_sweep_closes_the_view_on_the_off_frame_and_reverses() {
    // V at 5; another key three frames later, mid-sweep.
    let timeline = v_then(60, &[(8, KeyCode::Space)]);
    let (on, shows) = run(&outdoors(), &gem_on(), Some(W169), &timeline);
    let (off, none) = run(
        &outdoors(),
        &VisualLaunchConfig::classic(),
        Some(W169),
        &timeline,
    );
    assert!(none.is_empty(), "Classic has no gem map");
    assert_eq!(on, off, "the game, its frame and its keys, frame for frame");
    let opened = first_frame(&on, true).unwrap();
    let closed = opened + first_frame(&on[opened..], false).unwrap();
    assert!(closed - opened <= 4, "closed mid-sweep");
    let at_close = shows[closed - 1].step.expect("still sweeping in");
    assert!(at_close < 7);
    // The sweep turns round from the step it was on.
    assert_eq!(shows[closed].step, Some(at_close));
    assert!(!shows[closed].sweep_open);
    let mut down: Vec<u8> = shows[closed..].iter().filter_map(|s| s.step).collect();
    down.dedup();
    assert_eq!(down, (1..=at_close).rev().collect::<Vec<_>>());
    assert_eq!(shows.last().unwrap().nodes, 0);
}

#[test]
fn keys_during_the_close_sweep_reach_the_game() {
    // Close at 60, then walk during the close sweep and after it.
    let timeline = v_then(
        110,
        &[
            (60, KeyCode::Space),
            (63, KeyCode::ArrowUp),
            (66, KeyCode::ArrowLeft),
            (90, KeyCode::ArrowUp),
        ],
    );
    let (on, shows) = run(&outdoors(), &gem_on(), Some(W169), &timeline);
    let (off, _) = run(
        &outdoors(),
        &VisualLaunchConfig::classic(),
        Some(W169),
        &timeline,
    );
    assert_eq!(on, off);
    let closed = first_frame(&on[10..], false).unwrap() + 10;
    assert!(shows[closed + 2].step.is_some(), "still sweeping out");
    let moved = on[closed..]
        .windows(2)
        .any(|pair| pair[0].record.digest != pair[1].record.digest);
    assert!(moved, "the walk ran");
}

#[test]
fn the_game_is_the_off_run_at_4_3_16_9_and_21_9_and_without_the_sweep() {
    let timeline = v_then(
        100,
        &[
            (40, KeyCode::Space),
            (45, KeyCode::ArrowDown),
            (55, KeyCode::KeyV),
            (60, KeyCode::KeyV),
            (64, KeyCode::ArrowUp),
            (80, KeyCode::KeyV),
        ],
    );
    for window in [W43, W169, W219] {
        let (off, _) = run(
            &outdoors(),
            &VisualLaunchConfig::classic(),
            Some(window),
            &timeline,
        );
        for launch in [gem_on(), gem_no_sweep()] {
            let (on, shows) = run(&outdoors(), &launch, Some(window), &timeline);
            assert_eq!(on, off, "{window:?}");
            assert!(shows.iter().any(|s| s.nodes == 1), "{window:?}");
        }
        assert_eq!(off.last().unwrap().gems, 0, "three gems spent");
    }
}

#[test]
fn without_the_sweep_the_map_comes_and_goes_at_once_and_nothing_is_captured() {
    let timeline = v_then(40, &[(20, KeyCode::Space)]);
    let (beats, shows) = run(&outdoors(), &gem_no_sweep(), Some(W169), &timeline);
    for (beat, show) in beats.iter().zip(&shows) {
        let open = beat.overlay.contains("GemView");
        assert_eq!(show.nodes, usize::from(open));
        assert_eq!(show.step, None);
        assert_eq!(show.befores, 0);
    }
    let mut shell = shell(outdoors(), &gem_no_sweep(), Some(W169));
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    let show = shell.app.world().resource::<GemShow>();
    assert!(show.before.image.is_none(), "no GemBefore");
}

#[test]
fn the_before_picture_is_the_frame_from_before_the_view() {
    let mut shell = shell(outdoors(), &gem_on(), Some(W169));
    for _ in 0..5 {
        shell.step(&[]);
    }
    let frame = {
        let world = shell.app.world_mut();
        let mut sprites = world.query::<&Sprite>();
        sprites.single(world).unwrap().image.clone()
    };
    let before_v = image_bytes(&shell, &frame).1;
    let timeline = v_then(8, &[]);
    for index in 5..8 {
        shell.step(timeline.frame(index));
    }
    assert!(gem_view_open(&shell.visual().state));
    let befores = before_nodes(&mut shell);
    assert_eq!(befores.len(), 1, "an open sweep runs");
    let (handle, z, node) = &befores[0];
    assert_eq!(*z, GlobalZIndex(i32::MAX - 3), "directly under the map");
    assert_eq!(image_bytes(&shell, handle).1, before_v);
    assert_ne!(
        image_bytes(&shell, &frame).1,
        before_v,
        "the frame itself now shows the 1.0.1 gem view"
    );
    // Over the frame sprite's rectangle: the 320x200 frame fitted to the
    // window.
    assert!(matches!(node.width, Val::Px(w) if w > 0.0));
    assert!(matches!(node.height, Val::Px(h) if h > 0.0));
}

#[test]
fn resizing_while_open_lays_the_map_out_again() {
    let mut shell = shell(outdoors(), &gem_on(), Some(W169));
    let timeline = v_then(40, &[]);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    let size = |shell: &mut Shell| {
        let handle = map_nodes(shell)[0].0.clone();
        image_bytes(shell, &handle).0
    };
    assert_eq!(size(&mut shell), UVec2::new(512, 240));
    let renders = shell.app.world().resource::<GemShow>().renders;
    resize(&mut shell, W219);
    shell.step(&[]);
    assert_eq!(size(&mut shell), UVec2::new(688, 240), "the next frame");
    resize(&mut shell, W43);
    shell.step(&[]);
    assert_eq!(size(&mut shell), UVec2::new(320, 200));
    let show = shell.app.world().resource::<GemShow>();
    assert_eq!(show.renders, renders + 2, "one render per new canvas");
    // Held with nothing changing: no render, no compose.
    let composes = show.composes;
    for _ in 0..10 {
        shell.step(&[]);
    }
    let show = shell.app.world().resource::<GemShow>();
    assert_eq!((show.renders, show.composes), (renders + 2, composes));
    // Too small: the 1.0.1 view.
    resize(&mut shell, (300.0, 180.0));
    shell.step(&[]);
    assert!(map_nodes(&mut shell).is_empty());
    assert!(
        gem_view_open(&shell.visual().state),
        "the game's view stays"
    );
}

#[test]
fn a_small_window_or_no_window_keeps_the_1_0_1_view() {
    for window in [Some((300.0, 180.0)), None] {
        let timeline = v_then(30, &[]);
        let (beats, shows) = run(&outdoors(), &gem_on(), window, &timeline);
        assert!(beats.last().unwrap().overlay.contains("GemView"));
        assert!(shows.iter().all(|s| s.nodes == 0 && s.befores == 0));
    }
}

#[test]
fn a_view_this_build_does_not_draw_keeps_the_1_0_1_view() {
    // A town gem (M3-12 draws towns).
    let mut town = test_state(open_grid(), 5, 5);
    town.gems = 2;
    let timeline = v_then(30, &[]);
    let (beats, shows) = run(&town, &gem_on(), Some(W169), &timeline);
    assert!(beats.last().unwrap().overlay.contains("GemView"));
    assert!(shows.iter().all(|s| s.nodes == 0 && s.befores == 0));
}

#[test]
fn turning_the_toggle_off_with_the_map_up_hides_it_next_frame() {
    let mut shell = shell(outdoors(), &gem_on(), Some(W169));
    let timeline = v_then(40, &[]);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    assert_eq!(map_nodes(&mut shell).len(), 1);
    let before = beat(&shell);
    shell
        .app
        .insert_resource(Toggles(u5_runtime::enhancements::FeatureToggles::CLASSIC));
    shell.step(&[]);
    assert!(map_nodes(&mut shell).is_empty(), "gone on the next frame");
    assert!(before_nodes(&mut shell).is_empty());
    assert!(
        gem_view_open(&shell.visual().state),
        "the game is unaffected"
    );
    assert_eq!(beat(&shell).gems, before.gems);
    // Off: no system of the map runs again.
    let renders = shell.app.world().resource::<GemShow>().renders;
    for _ in 0..5 {
        shell.step(&[]);
    }
    assert_eq!(shell.app.world().resource::<GemShow>().renders, renders);
    assert!(map_nodes(&mut shell).is_empty());
}

#[test]
fn off_has_no_node_resource_or_system() {
    let timeline = v_then(40, &[]);
    let mut shell = shell(outdoors(), &VisualLaunchConfig::classic(), Some(W169));
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    assert!(gem_view_open(&shell.visual().state));
    assert!(map_nodes(&mut shell).is_empty());
    assert!(before_nodes(&mut shell).is_empty());
    assert!(!shell.app.world().contains_resource::<GemShow>());
    let names: Vec<String> = shell
        .app
        .world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .flat_map(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .map(|(_, system, _)| system.name().to_string())
                .collect::<Vec<_>>()
        })
        .filter(|name| name.contains("gem_map"))
        .collect();
    assert!(names.is_empty(), "{names:?}");

    // On: the map's four systems are in the schedule.
    let on = Shell::new(outdoors(), &Scene::synthetic(), &gem_on(), |_| {});
    let names: Vec<String> = on
        .app
        .world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .flat_map(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .map(|(_, system, _)| system.name().to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    for system in ["gem_capture_pre", "gem_render", "gem_layout", "gem_hide"] {
        assert!(names.iter().any(|n| n.ends_with(system)), "{system}");
    }
}

#[test]
fn the_intro_is_inert() {
    let dir = u5_runtime::test_fixtures::debug_game_dir();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    crate::enh::add_enhancements(&mut app, &gem_on());
    spawn_window(&mut app, W169);
    let intro = crate::visual_intro_suite_state(
        crate::VisualIntroPanel::Menu,
        &dir,
        u5_runtime::TileGraphicsDepth::Ega16,
        true,
    )
    .unwrap();
    app.insert_resource(intro);
    for _ in 0..10 {
        app.update();
    }
    let world = app.world_mut();
    let show = world.resource::<GemShow>();
    assert_eq!((show.renders, show.composes), (0, 0));
    assert!(show.before.image.is_none());
    let mut nodes = world.query::<(Option<&GemMapNode>, Option<&GemBeforeNode>)>();
    assert!(
        nodes
            .iter(world)
            .all(|(map, before)| map.is_none() && before.is_none())
    );
    let _ = std::fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Review shots (never committed).

fn save_png(path: &std::path::Path, size: UVec2, rgba: Vec<u8>) {
    image::RgbaImage::from_raw(size.x, size.y, rgba)
        .unwrap()
        .save(path)
        .unwrap();
}

/// The render check (plan M3-11): the mockups' S1, the forest road west of
/// Britain at 12:06, a gem used in the real shell at 16:9, 21:9 and 4:3.
/// Writes the held map, every sweep step's picture and the before picture
/// to `U5_GEM_SHOTS` for the side-by-side with `O1`, `O2` and `O3`. Review
/// output only: skipped unless `U5_MOCKUP_PROFILE` and `U5_GEM_SHOTS` are
/// set.
#[test]
fn gem_shell_review_shots() {
    use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_GEM_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_GEM_SHOTS");
        return;
    };
    let game = unique_scratch_dir("gem-shell-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    for (name, window, grow) in [
        ("O1-169", W169, true),
        ("O2-219", W219, true),
        ("O3-43", W43, false),
    ] {
        let mut state = PlayState::load_scene(
            &game,
            u5_runtime::PlayOptions {
                target: u5_runtime::PlayTarget::World(u5_runtime::WorldPlane::Britannia),
                start: Some((76, 108)),
                clock: u5_runtime::GameClock::new(12, 6).unwrap(),
                ..u5_runtime::PlayOptions::default()
            },
        )
        .unwrap();
        state.gems = 1;
        let mut cli = vec![CliSetting::Enable("gem_map.enhanced".into())];
        if grow {
            cli.push(CliSetting::Enable("display.grow_left".into()));
        }
        let mut shell = Shell::new(state, &scene, &launch(&cli), |_| {});
        spawn_window(&mut shell.app, window);
        let timeline = v_then(130, &[(80, KeyCode::Space)]);
        let mut last_step = None;
        let mut saved_before = false;
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
            let show = shell.app.world().resource::<GemShow>();
            let (step, open) = (show.sweep.step, show.sweep.open);
            let Some(handle) = show.image().cloned() else {
                continue;
            };
            if !saved_before && let Some((before, ..)) = before_nodes(&mut shell).first().cloned() {
                let (size, bytes) = image_bytes(&shell, &before);
                save_png(&out.join(format!("engine-{name}-before.png")), size, bytes);
                saved_before = true;
            }
            if last_step == Some((step, open)) {
                continue;
            }
            last_step = Some((step, open));
            let (size, bytes) = image_bytes(&shell, &handle);
            let file = match step {
                None => format!("engine-{name}-held.png"),
                Some(k) if open => format!("engine-{name}-open-{k}.png"),
                Some(k) => format!("engine-{name}-close-{k}.png"),
            };
            save_png(&out.join(file), size, bytes);
        }
        assert!(saved_before, "{name}: the before picture");
        assert!(
            out.join(format!("engine-{name}-held.png")).exists(),
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(game);
}
