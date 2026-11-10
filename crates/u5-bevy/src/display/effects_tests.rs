//! The dither effects (design 2.2.6, 2.2.7, 2.3.10): the formulas (every
//! pixel's step against the approved mockup script is checked by
//! `u5-tui/tests/grow_left_shots.rs`, a host crate that may run it), the one-step text recolour, the two
//! tabs on one mask, the step clock and reversal, and the game untouched by
//! a running effect.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, HostCapabilities, LockState, PixelAspect,
    ResolveInput, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, TileGraphicsDepth, parse_ch_font};

use super::*;
use crate::display::grow::column::ColumnState;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline, check_shell_equivalence};

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

#[test]
fn the_formulas_are_the_designs() {
    // 2.3.10, written out for a few pixels.
    let (top, bottom) = (15, 233);
    assert_eq!(sweep_t(7, 232, top, bottom), 0.28 * 10.0 / 16.0);
    assert_eq!(
        sweep_t(8, 15, top, bottom),
        0.72 * 217.0 / 218.0 + 0.28 * 15.0 / 16.0
    );
    assert_eq!(tab_t(0, 8), 0.4 * 7.0 / 8.0);
    assert_eq!(tab_t(3, 15), 0.6 * 5.0 / 16.0);
    // The bottom row arrives first, the top rule last.
    assert_eq!(sweep_step(4, 232, top, bottom), 1);
    assert_eq!(sweep_step(4, 15, top, bottom), SWEEP_STEPS);
    assert_eq!(sweep_step(7, 15, top, bottom), SWEEP_STEPS - 1);
    // Every column pixel is open by the end: step 8 is the open image.
    for y in top..bottom {
        for x in SWEEP_X0..SWEEP_X1 {
            assert!((1..=SWEEP_STEPS).contains(&sweep_step(x, y, top, bottom)));
        }
    }
    // The 216-row canvas sweeps from its own bottom rule.
    let c = canvas(1920, 1080);
    assert_eq!(sweep_rows(&c), (15, 209));
}

/// Solid images: closed index 2, open index 4, neutral index 1; the tabs
/// black (0) where `tabs` puts them.
fn solid(c: &Canvas, index: u8, tabs: Option<(u32, u32)>) -> Vec<u8> {
    let [r, g, b] = crate::EGA_PALETTE_RGB[usize::from(index)];
    let mut rgba = [r, g, b, 0xff].repeat((c.w * c.h) as usize);
    if let Some((x0, x1)) = tabs {
        for y in (8..16).chain(c.h - 8..c.h) {
            for x in x0..x1 {
                let at = ((y * c.w + x) * 4) as usize;
                rgba[at..at + 4].copy_from_slice(&[0, 0, 0, 0xff]);
            }
        }
    }
    rgba
}

fn index_at(rgba: &[u8], c: &Canvas, x: u32, y: u32) -> u8 {
    let at = ((y * c.w + x) * 4) as usize;
    crate::ega_palette_index_from_rgba(&rgba[at..at + 4])
}

/// The opening's nine frames: the closed image, steps 1-7, the open image.
fn frames(c: &Canvas, images: EffectImages<'_>, text: &TextMask) -> Vec<Vec<u8>> {
    let mut out = vec![images.closed.to_vec()];
    out.extend((1..SWEEP_STEPS).map(|k| blend(images, c, k, text)));
    out.push(images.opened.to_vec());
    out
}

#[test]
fn the_text_switches_on_one_step() {
    for (w, h) in [(2560, 1440), (3440, 1440), (1920, 1080)] {
        let c = canvas(w, h);
        let (closed, opened, neutral) =
            (solid(&c, 2, None), solid(&c, 4, None), solid(&c, 1, None));
        let images = EffectImages {
            closed: &closed,
            opened: &opened,
            neutral: &neutral,
        };
        // Six crawl rows and their outlines, x 15..144.
        let bottom = c.map_window().3 + 1 - 8;
        let mut text = TextMask::empty(&c);
        for y in bottom - 41..=bottom + 7 {
            for x in 15..145 {
                text.set(x, y);
            }
        }
        let swap = swap_step(&text, &c);
        assert!((2..SWEEP_STEPS).contains(&swap), "{w}x{h}: {swap}");
        for (k, frame) in frames(&c, images, &text).iter().enumerate() {
            let colours: std::collections::BTreeSet<u8> = (bottom - 41..=bottom + 7)
                .flat_map(|y| (15..145).map(move |x| (x, y)))
                .map(|(x, y)| index_at(frame, &c, x, y))
                .collect();
            let want = if k as u8 >= swap { 4 } else { 2 };
            assert_eq!(colours, [want].into(), "{w}x{h} frame {k}");
        }
    }
}

#[test]
fn both_tabs_change_on_the_same_pixels_and_are_absent_on_one_frame() {
    for ((w, h), old, new) in [
        ((2560, 1440), (200, 312), (272, 384)),
        ((3440, 1440), (288, 400), (360, 472)),
        ((1920, 1080), (184, 296), (256, 368)),
    ] {
        let c = canvas(w, h);
        let closed = solid(&c, 1, Some(old));
        let opened = solid(&c, 1, Some(new));
        let neutral = solid(&c, 1, None);
        let images = EffectImages {
            closed: &closed,
            opened: &opened,
            neutral: &neutral,
        };
        let text = TextMask::empty(&c);
        let mut absent = Vec::new();
        for (k, frame) in frames(&c, images, &text).iter().enumerate() {
            let mut any = false;
            for row in 0..8 {
                let (sky, wind) = (8 + row, c.h - 8 + row);
                let sky_px: Vec<u32> = (0..c.w)
                    .filter(|x| index_at(frame, &c, *x, sky) == 0)
                    .collect();
                let wind_px: Vec<u32> = (0..c.w)
                    .filter(|x| index_at(frame, &c, *x, wind) == 0)
                    .collect();
                assert_eq!(sky_px, wind_px, "{w}x{h} frame {k} row {row}");
                any |= !sky_px.is_empty();
            }
            if !any {
                absent.push(k);
            }
            // Old and new pixels never meet: before step 4 only the old
            // tab's columns, from it only the new one's.
            let cols: Vec<u32> = (0..c.w)
                .filter(|x| (8..16).any(|y| index_at(frame, &c, *x, y) == 0))
                .collect();
            let range = if (k as u8) < PAN_STEP { old } else { new };
            assert!(
                cols.iter().all(|x| (range.0..range.1).contains(x)),
                "{w}x{h} frame {k}"
            );
        }
        assert_eq!(absent, [TABS_OUT_LAST as usize], "{w}x{h}");
    }
}

#[test]
fn the_rest_of_the_canvas_changes_on_step_four() {
    let c = canvas(2560, 1440);
    let (closed, opened, neutral) = (solid(&c, 2, None), solid(&c, 4, None), solid(&c, 1, None));
    let images = EffectImages {
        closed: &closed,
        opened: &opened,
        neutral: &neutral,
    };
    let text = TextMask::empty(&c);
    for k in 1..SWEEP_STEPS {
        let frame = blend(images, &c, k, &text);
        let want = if k >= PAN_STEP { 4 } else { 2 };
        assert_eq!(index_at(&frame, &c, 300, 120), want, "step {k}");
        assert_eq!(index_at(&frame, &c, 153, 120), want, "step {k}");
    }
}

#[test]
fn nine_frames_at_fifty_ms_and_reversal_from_the_current_step() {
    let frame = Duration::from_nanos(1_000_000_000 / 60);
    let mut view = ColumnView::default();
    let mut elapsed = Duration::ZERO;
    toggle(&mut view, true);
    assert!(view.open);
    let mut shown = vec![view.step];
    while view.step.is_some() {
        tick(&mut view, &mut elapsed, frame, true);
        shown.push(view.step);
    }
    // Steps 1-7 each three sixtieths (50 ms), then the open image.
    let want: Vec<Option<u8>> = (1..SWEEP_STEPS)
        .flat_map(|k| [Some(k); 3])
        .chain([None])
        .collect();
    assert_eq!(shown, want);
    assert!(view.open);
    // Closing runs backwards.
    toggle(&mut view, true);
    assert_eq!(view.step, Some(SWEEP_STEPS - 1));
    tick(&mut view, &mut elapsed, STEP, true);
    tick(&mut view, &mut elapsed, STEP, true);
    assert_eq!(view.step, Some(5));
    // Pressed again mid-effect: it reverses from the step it is on.
    toggle(&mut view, true);
    assert!(view.open);
    assert_eq!(view.step, Some(5));
    tick(&mut view, &mut elapsed, STEP, true);
    assert_eq!(view.step, Some(6));
    // Dither turned off mid-effect: the column lands at once.
    tick(&mut view, &mut elapsed, frame, false);
    assert_eq!(view.step, None);
    assert!(view.open);
    // Off from the start: one frame.
    toggle(&mut view, false);
    assert_eq!((view.open, view.step), (false, None));
    // Time without an effect is not held.
    tick(&mut view, &mut elapsed, STEP * 3, true);
    assert_eq!((view.step, elapsed), (None, Duration::ZERO));
}

// ---------------------------------------------------------------- the shell

fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &[DISPLAY_P1, DISPLAY_P2, DISPLAY_P8],
        ),
        ..VisualLaunchConfig::classic()
    }
}

fn scene() -> Scene {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    for code in 0..CH_FONT_LEN / 8 {
        let c = code as u8;
        let rows = [c | 1, !c, c | 0x80, c, !c | 1, c ^ 0x55, c | 0x10, 0x81];
        bytes[code * 8..code * 8 + 8].copy_from_slice(&rows);
    }
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    Scene {
        game_dir: ".".into(),
        atlas: synthetic_tile_atlas(TileGraphicsDepth::Ega16),
        text_font: font.clone(),
        rune_font: font,
    }
}

fn spawn_window(app: &mut App) {
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(2560.0, 1440.0),
        ..default()
    });
}

fn ctrl_p(timeline: &mut Timeline, frame: usize) {
    for (at, state) in [
        (frame, ButtonState::Pressed),
        (frame + 1, ButtonState::Released),
    ] {
        for key in [KeyCode::ControlLeft, KeyCode::KeyP] {
            timeline.event(at, key, state, false, Owner::Feature);
        }
    }
}

fn presented(shell: &mut Shell) -> Vec<u8> {
    let world = shell.app.world_mut();
    let mut sprites = world.query::<&Sprite>();
    let handle = sprites.single(world).unwrap().image.clone();
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    image.data.clone().unwrap()
}

#[test]
fn the_shell_runs_the_effect_on_its_frame_clock() {
    let grow = launch(&[CliSetting::Enable("display.grow_left".into())]);
    let mut shell = Shell::new(test_state(open_grid(), 5, 5), &scene(), &grow, |_| {});
    spawn_window(&mut shell.app);
    let mut timeline = Timeline::new(80);
    ctrl_p(&mut timeline, 10);
    ctrl_p(&mut timeline, 50);
    let mut steps = Vec::new();
    let mut images = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let view = shell.app.world().resource::<ColumnState>().view();
        steps.push((view.open, view.step));
        images.push(presented(&mut shell));
    }
    // Opening: frames 10..=30 show steps 1-7, three frames each; frame 31
    // is the open image.
    for (k, frame) in (1..SWEEP_STEPS).zip((10..31).step_by(3)) {
        for (at, seen) in steps.iter().enumerate().skip(frame).take(3) {
            assert_eq!(*seen, (true, Some(k)), "frame {at}");
        }
    }
    assert_eq!(steps[31], (true, None));
    let c = canvas(2560, 1440);
    // Step 1 already shows the column's bottom rows (the sweep rises from
    // the crawl); the end frames show the whole column or none of it.
    assert_ne!(images[10], images[9]);
    assert_eq!(index_at(&images[31], &c, 7, 67), 1);
    assert_eq!(index_at(&images[31], &c, 151, 120), 15);
    assert_eq!(index_at(&images[9], &c, 7, 67), 15);
    // Mid-way (step 4) the band between the windows at y 67 has not
    // arrived but the message window's top rule at y 95 has.
    assert_eq!(index_at(&images[19], &c, 7, 67), 15);
    // Closing: the same steps backwards, ending on the collapsed canvas.
    for (k, frame) in (1..SWEEP_STEPS).rev().zip((50..71).step_by(3)) {
        for (at, seen) in steps.iter().enumerate().skip(frame).take(3) {
            assert_eq!(*seen, (false, Some(k)), "frame {at}");
        }
    }
    assert_eq!(steps[71], (false, None));
    assert_eq!(index_at(&images[71], &c, 7, 67), 15);
}

#[test]
fn with_dither_off_the_column_switches_in_one_frame() {
    let grow = launch(&[
        CliSetting::Enable("display.grow_left".into()),
        CliSetting::Disable("display.dither".into()),
    ]);
    let mut shell = Shell::new(test_state(open_grid(), 5, 5), &scene(), &grow, |_| {});
    spawn_window(&mut shell.app);
    let mut timeline = Timeline::new(20);
    ctrl_p(&mut timeline, 5);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let view = shell.app.world().resource::<ColumnState>().view();
        assert_eq!((view.open, view.step), (frame >= 5, None), "frame {frame}");
    }
}

#[test]
fn a_key_typed_during_an_effect_reaches_the_game_on_the_same_tick() {
    // Keys during the opening, during the closing, and around a reversal
    // mid-effect, walking and with the effect on and off: the game's
    // trajectory, frame and queue are Classic's on every frame.
    let mut walk = Timeline::new(110);
    ctrl_p(&mut walk, 10);
    walk.game_tap(11, KeyCode::ArrowUp);
    walk.game_tap(17, KeyCode::ArrowLeft);
    walk.game_tap(25, KeyCode::ArrowDown);
    ctrl_p(&mut walk, 40);
    walk.game_tap(41, KeyCode::ArrowRight);
    walk.game_tap(46, KeyCode::ArrowUp);
    ctrl_p(&mut walk, 50);
    walk.game_tap(52, KeyCode::ArrowLeft);
    ctrl_p(&mut walk, 56);
    walk.game_tap(58, KeyCode::ArrowDown);
    walk.game_tap(90, KeyCode::ArrowRight);
    for dither in [true, false] {
        let mut cli = vec![CliSetting::Enable("display.grow_left".into())];
        if !dither {
            cli.push(CliSetting::Disable("display.dither".into()));
        }
        let mismatches = check_shell_equivalence(
            &test_state(open_grid(), 5, 5),
            &scene(),
            &walk,
            &launch(&cli),
            spawn_window,
        );
        assert_eq!(mismatches, [], "dither {dither}");
    }
}
