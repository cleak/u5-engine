//! The M4-10 render check: a slot save in the real shell over real engine
//! frames, for the user to look at beside the c04f mockups (build plan
//! M4-10). Review output only, never committed, and `#[ignore]`d, as the
//! M4-05 shots ([`super::review_tests`]):
//!
//! ```text
//! U5_MOCKUP_PROFILE=<copy> U5_MENU_SHOTS=<folder> \
//!   cargo test -p u5-bevy --lib menu::inject_review_tests -- --ignored
//! ```
//!
//! On the c04f forest road, at 4:3 and on the grow-left 16:9 canvas: the
//! top menu with Save offered and selected, then the save the menu types,
//! one whole-window shot per frame from the request to a second after the
//! save (the menu's close, `Save game?`, `Yes`, `Saving...` and `Done.` in
//! the game's own window, the "Saving..." toast), 4x close-ups of the toast
//! and the message window, and the slot's thumbnail at 1x and 4x.

use std::fs;
use std::sync::Arc;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{SHIPPED_PHASES, Toggle};
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayState, PlayTarget, load_play_options_from_save};

use super::host::NoStore;
use super::inject::{SlotSaveFinished, SlotSaveRequest};
use super::render::{MenuView, frame_rect};
use super::review_tests::{CASES, crop, launch, save_png, scaled, window_shot};
use super::thumb::{THUMB_HEIGHT, THUMB_WIDTH};
use super::{MenuShell, MenuState};
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey, Timeline};

fn key(shell: &mut Shell, key: KeyCode, state: ButtonState) {
    shell.step(&[TimedKey {
        key,
        state,
        repeat: false,
        owner: Owner::Game,
    }]);
}

/// The frame sprite's rectangle in physical window pixels.
fn frame_in_window(shell: &mut Shell) -> Rect {
    let world = shell.app.world_mut();
    let window = world.query::<&Window>().iter(world).next().unwrap().clone();
    let (sprite, transform) = world
        .query::<(&Sprite, &Transform)>()
        .iter(world)
        .next()
        .map(|(s, t)| (s.clone(), *t))
        .unwrap();
    let rect = frame_rect(&window, &sprite, &transform).unwrap();
    let scale = window.resolution.scale_factor();
    Rect::from_corners(rect.min * scale, rect.max * scale)
}

/// The toast strip's rectangle in physical window pixels, if one shows.
fn toast_in_window(shell: &mut Shell) -> Option<Rect> {
    let world = shell.app.world_mut();
    let scale = world
        .query::<&Window>()
        .iter(world)
        .next()?
        .resolution
        .scale_factor();
    let mut strips = world.query_filtered::<&Node, With<crate::enh::toast::ToastStrip>>();
    let node = strips.iter(world).next()?;
    let px = |v: Val| match v {
        Val::Px(px) => px,
        _ => 0.0,
    };
    let min = Vec2::new(px(node.left), px(node.top));
    let size = Vec2::new(px(node.width), px(node.height));
    Some(Rect::from_corners(min * scale, (min + size) * scale))
}

#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_MENU_SHOTS"]
fn slot_save_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("slot-save-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    for case in [&CASES[0], &CASES[1], &CASES[3]] {
        let mut options = load_play_options_from_save(&game).unwrap();
        options.target = PlayTarget::from_key("BRITANNIA").unwrap();
        options.saved_active_objects = None;
        options.start = Some((74, 111));
        options.clock = GameClock::new(11, 50).unwrap();
        let state = PlayState::load_scene(&game, options).unwrap();
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
        let mut launch = launch(case.grow);
        launch
            .enhancements
            .toggles
            .set(Toggle::MenuNamedSlots, true);
        launch.menu_host = Some(Arc::new(NoStore));
        let mut shell = Shell::new(state, &scene, &launch, |app| {
            app.insert_resource(MenuShell {
                intro_shell: true,
                slots: true,
                shipped: SHIPPED_PHASES,
            });
        });
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
            ..default()
        });
        for index in 0..timeline.len() {
            shell.step(timeline.frame(index));
        }
        let label = case.label;
        let write = |name: &str, (w, h, rgba): &(u32, u32, Vec<u8>)| {
            save_png(&out.join(format!("{label}-{name}.png")), *w, *h, rgba);
        };
        let closeup = |name: &str, (w, h, rgba): &(u32, u32, Vec<u8>), area: Rect| {
            let area = (
                area.min.x as i64,
                area.min.y as i64,
                area.width() as u32,
                area.height() as u32,
            );
            let (cw, ch, part) = crop(*w, *h, rgba, area);
            save_png(
                &out.join(format!("{label}-closeup-{name}-4x.png")),
                cw * 4,
                ch * 4,
                &scaled(cw, ch, &part, 4),
            );
        };

        // The menu, with Save offered, then selected.
        key(&mut shell, KeyCode::F12, ButtonState::Pressed);
        key(&mut shell, KeyCode::F12, ButtonState::Released);
        while shell
            .app
            .world()
            .resource::<MenuView>()
            .sweep
            .step
            .is_some()
        {
            shell.step(&[]);
        }
        write("menu-top", &window_shot(&mut shell));
        key(&mut shell, KeyCode::ArrowDown, ButtonState::Pressed);
        key(&mut shell, KeyCode::ArrowDown, ButtonState::Released);
        let selected = window_shot(&mut shell);
        write("menu-save-selected", &selected);
        assert!(shell.app.world().resource::<MenuState>().model().is_some());

        // The save, frame by frame. The Save page (M4-11) sends the request;
        // here it is sent as the page will.
        let frame = frame_in_window(&mut shell);
        shell.app.world_mut().send_event(SlotSaveRequest::default());
        let mut done_at = None;
        let mut capture = None;
        for n in 0..90 {
            shell.step(&[]);
            let finished: Vec<SlotSaveFinished> = shell
                .app
                .world()
                .resource::<Events<SlotSaveFinished>>()
                .iter_current_update_events()
                .cloned()
                .collect();
            for event in finished {
                if let SlotSaveFinished::Saved(saved) = event {
                    capture = Some(saved);
                    done_at = Some(n);
                } else {
                    panic!("{label}: the save did not finish: {event:?}");
                }
            }
            let shot = window_shot(&mut shell);
            if n < 6 || done_at.is_some_and(|at| n == at + 1 || n == at + 60) {
                write(&format!("save-{n:02}"), &shot);
            }
            if done_at.is_some_and(|at| n == at + 1) {
                if let Some(toast) = toast_in_window(&mut shell) {
                    let pad = 24.0;
                    closeup(
                        "saving-toast",
                        &shot,
                        Rect::from_corners(toast.min - pad, toast.max + pad),
                    );
                    // The toast's left and right ends, at the cell grid.
                    closeup(
                        "saving-toast-left",
                        &shot,
                        Rect::from_corners(
                            toast.min - pad,
                            Vec2::new(toast.min.x + 48.0, toast.max.y + pad),
                        ),
                    );
                }
                // The message window: the frame's right quarter, lower
                // half, where `Save game? Yes`, `Saving...` and `Done.`
                // print.
                let w = frame.width();
                let h = frame.height();
                closeup(
                    "message-window",
                    &shot,
                    Rect::from_corners(
                        Vec2::new(frame.max.x - w * 0.42, frame.min.y + h * 0.5),
                        frame.max,
                    ),
                );
            }
        }
        let capture = capture.unwrap_or_else(|| panic!("{label}: no save"));
        eprintln!(
            "{label}: saved at frame {:?}, turn {}, avatar {:?}",
            done_at, capture.turn, capture.summary.avatar
        );
        let png = capture.thumbnail_png.expect("a thumbnail");
        fs::write(out.join(format!("{label}-thumbnail.png")), &png).unwrap();
        let thumb = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(thumb.dimensions(), (THUMB_WIDTH, THUMB_HEIGHT));
        save_png(
            &out.join(format!("{label}-thumbnail-4x.png")),
            THUMB_WIDTH * 4,
            THUMB_HEIGHT * 4,
            &scaled(THUMB_WIDTH, THUMB_HEIGHT, thumb.as_raw(), 4),
        );
    }
    let _ = fs::remove_dir_all(&game);
}
