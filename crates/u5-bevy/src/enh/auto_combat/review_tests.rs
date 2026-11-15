//! Review shots of the auto-combat toasts (plan M7-05). Review only:
//! skipped unless `U5_MOCKUP_PROFILE` names a game copy and
//! `U5_AUTO_SHOTS` a folder; nothing is committed.
//!
//! Each shot runs the real `--visual` shell headless with `combat.auto`
//! on, types the hotkey, and lays the shell's own original frame and the
//! foundation's toast strip (its CPU-built IBM.CH image, at the rect the
//! foundation places it) into a window-sized picture, nearest-neighbour,
//! as the window shows them.

use bevy::input::ButtonState;
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{PlayOptions, PlayState};

use super::feed_tests::arena;
use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline};
use crate::enh::toast::{toast_strip_rect, toast_strip_rgba};
use crate::{VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, fitted_display_size};

fn launch() -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(AUTO, true);
    launch
}

/// Lays `src` (w x h RGBA) into `rect` of `canvas`, nearest-neighbour.
fn blit(canvas: &mut image::RgbaImage, src: &[u8], w: u32, h: u32, rect: Rect) {
    let (x0, y0) = (rect.min.x.round() as i64, rect.min.y.round() as i64);
    let (x1, y1) = (rect.max.x.round() as i64, rect.max.y.round() as i64);
    for y in y0.max(0)..y1.min(i64::from(canvas.height())) {
        for x in x0.max(0)..x1.min(i64::from(canvas.width())) {
            let sx = ((x - x0) as f32 * w as f32 / (x1 - x0) as f32) as u32;
            let sy = ((y - y0) as f32 * h as f32 / (y1 - y0) as f32) as u32;
            let at = ((sy.min(h - 1) * w + sx.min(w - 1)) * 4) as usize;
            canvas.put_pixel(
                x as u32,
                y as u32,
                image::Rgba([src[at], src[at + 1], src[at + 2], 0xff]),
            );
        }
    }
}

fn shot(shell: &Shell, window: Vec2) -> image::RgbaImage {
    let world = shell.app.world();
    let visual = shell.visual();
    let frame = world
        .resource::<Assets<Image>>()
        .get(&visual.image_handle)
        .and_then(|image| image.data.clone())
        .unwrap();
    let mut canvas = image::RgbaImage::from_pixel(
        window.x as u32,
        window.y as u32,
        image::Rgba([0, 0, 0, 0xff]),
    );
    let size = fitted_display_size(VISUAL_PLAY_FRAME_WIDTH, VISUAL_PLAY_FRAME_HEIGHT, window);
    blit(
        &mut canvas,
        &frame,
        VISUAL_PLAY_FRAME_WIDTH,
        VISUAL_PLAY_FRAME_HEIGHT,
        Rect::from_center_size(window / 2.0, size),
    );
    if let Some(text) = world.resource::<Toasts>().current() {
        let (w, h, rgba) = toast_strip_rgba(&visual.text_font, text);
        blit(
            &mut canvas,
            &rgba,
            w,
            h,
            toast_strip_rect(window, None, text.len()),
        );
    }
    canvas
}

fn chord(shell: &mut Shell, keys: &[KeyCode]) {
    let mut timeline = Timeline::new(0);
    for key in keys {
        timeline.event(0, *key, ButtonState::Pressed, false, Owner::Game);
    }
    for key in keys.iter().rev() {
        timeline.event(1, *key, ButtonState::Released, false, Owner::Game);
    }
    shell.step(timeline.frame(0));
    shell.step(timeline.frame(1));
}

#[test]
fn auto_combat_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_AUTO_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_AUTO_SHOTS");
        return;
    };
    let game = unique_scratch_dir("auto-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            copy_asset_writable(&path, &game.join(path.file_name().unwrap())).unwrap();
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    let scene = Scene::from_game_dir(&game).unwrap();
    let explore = PlayState::load_scene(&game, PlayOptions::default()).unwrap();
    let ctrl_a = [KeyCode::ControlLeft, KeyCode::KeyA];
    let ctrl_shift_a = [KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::KeyA];
    let ctrl_f = [KeyCode::ControlLeft, KeyCode::KeyF];
    let shots: [(&str, PlayState, &[KeyCode]); 5] = [
        ("not-in-combat", explore, &ctrl_a),
        ("ready", arena(), &[]),
        ("member", arena(), &ctrl_a),
        ("party", arena(), &ctrl_shift_a),
        ("until-done", arena(), &ctrl_f),
    ];
    for (name, state, keys) in shots {
        let mut shell = Shell::new(state, &scene, &launch(), |_| {});
        for _ in 0..30 {
            shell.step(&[]);
        }
        if !keys.is_empty() {
            chord(&mut shell, keys);
        }
        eprintln!(
            "{name}: {:?}",
            shell.app.world().resource::<Toasts>().current()
        );
        for (label, window) in [
            ("169", Vec2::new(2560.0, 1440.0)),
            ("43", Vec2::new(1600.0, 1200.0)),
        ] {
            shot(&shell, window)
                .save(out.join(format!("auto-{name}-{label}.png")))
                .unwrap();
        }
    }
}
