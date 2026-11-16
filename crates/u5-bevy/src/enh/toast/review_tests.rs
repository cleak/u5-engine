//! Review shots of the shell's toast and badges (Phase B design 1.3.7;
//! the integration polish after M7 and M8): the real `--visual` shell,
//! headless, on the c04f forest road, with a toast posted and both badges
//! set, composed as the window shows it (the presented picture, then every
//! UI image node in z order, nearest-neighbour).
//!
//! Review output only, never committed, and `#[ignore]`d: run with
//! `U5_MOCKUP_PROFILE` naming a `test-assets` or `prepare-profile` game copy
//! and `U5_SHELL_SHOTS` an output folder:
//!
//! ```text
//! U5_MOCKUP_PROFILE=<copy> U5_SHELL_SHOTS=<folder> \
//!   cargo test -p u5-bevy --lib enh::toast::review_tests -- --ignored
//! ```
//!
//! The windows are 960 pixels tall, so the 320x200 frame (and the
//! grow-left canvas) shows at 4x across.

use std::fs;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, ResolveInput, Toggle, resolve,
};
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayState, PlayTarget, load_play_options_from_save};

use super::{ShellBadges, Toasts};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline};
use crate::menu::review_tests::{game_picture, save_png};

struct Case {
    label: &'static str,
    window: (u32, u32),
    grow: bool,
}

const CASES: [Case; 6] = [
    Case {
        label: "43",
        window: (1280, 960),
        grow: false,
    },
    Case {
        label: "169",
        window: (1706, 960),
        grow: false,
    },
    Case {
        label: "219",
        window: (2240, 960),
        grow: false,
    },
    Case {
        label: "43-grow",
        window: (1280, 960),
        grow: true,
    },
    Case {
        label: "169-grow",
        window: (2048, 960),
        grow: true,
    },
    Case {
        label: "219-grow",
        window: (2240, 960),
        grow: true,
    },
];

/// The toasts each case shows: a hand-back that names a member, and the
/// once-only ready note, the longest auto-combat line.
const TOASTS: [(&str, &str); 2] = [
    ("hurt", "Auto off: Avatar is hurt"),
    ("ready", crate::enh::auto_combat::AUTO_READY),
];

fn launch(grow: bool) -> VisualLaunchConfig {
    let cli: Vec<CliSetting> = if grow {
        vec![CliSetting::Enable("display.grow_left".into())]
    } else {
        Vec::new()
    };
    let mut enhancements = resolve(ResolveInput {
        stored: None,
        cli: &cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    // The watch brings the toast line and leaves both badges to the shot
    // (auto-combat would clear `auto` outside a fight).
    enhancements.toggles.set(Toggle::CheatsStateWatch, true);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

fn px(value: Val) -> f32 {
    match value {
        Val::Px(px) => px,
        _ => 0.0,
    }
}

/// The window as the GPU would show it: black, the frame sprite sampled
/// nearest over its rectangle, then every image node over it in z order.
fn window_shot(shell: &mut Shell) -> (u32, u32, Vec<u8>) {
    let (size, game) = game_picture(shell);
    let world = shell.app.world_mut();
    let window = world.query::<&Window>().iter(world).next().unwrap().clone();
    let (sprite, transform) = world
        .query::<(&Sprite, &Transform)>()
        .iter(world)
        .next()
        .map(|(s, t)| (s.clone(), *t))
        .unwrap();
    let (ww, wh) = (
        window.resolution.physical_width(),
        window.resolution.physical_height(),
    );
    let frame = {
        let size = sprite.custom_size.unwrap() * transform.scale.truncate();
        let centre = Vec2::new(
            ww as f32 / 2.0 + transform.translation.x,
            wh as f32 / 2.0 - transform.translation.y,
        );
        Rect::from_center_size(centre, size)
    };
    let mut nodes: Vec<(i32, Rect, UVec2, Vec<u8>)> = Vec::new();
    let mut query = world.query::<(&ImageNode, &Node, Option<&GlobalZIndex>)>();
    for (image, node, z) in query.iter(world) {
        let rect = Rect::new(
            px(node.left),
            px(node.top),
            px(node.left) + px(node.width),
            px(node.top) + px(node.height),
        );
        let Some(image) = world.resource::<Assets<Image>>().get(&image.image) else {
            continue;
        };
        nodes.push((
            z.map_or(0, |z| z.0),
            rect,
            image.size(),
            image.data.clone().unwrap_or_default(),
        ));
    }
    nodes.sort_by_key(|(z, ..)| *z);
    let sample = |r: Rect, size: UVec2, rgba: &[u8], x: u32, y: u32| -> Option<[u8; 4]> {
        let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        if !r.contains(point) || rgba.is_empty() {
            return None;
        }
        let u = (((point.x - r.min.x) / r.width()) * size.x as f32) as u32;
        let v = (((point.y - r.min.y) / r.height()) * size.y as f32) as u32;
        let i = ((v.min(size.y - 1) * size.x + u.min(size.x - 1)) * 4) as usize;
        Some([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
    };
    let mut out = Vec::with_capacity((ww * wh * 4) as usize);
    for y in 0..wh {
        for x in 0..ww {
            let mut pixel = sample(frame, size, &game, x, y).unwrap_or([0, 0, 0, 0xff]);
            for (_, rect, nsize, rgba) in &nodes {
                if let Some(src) = sample(*rect, *nsize, rgba, x, y) {
                    let a = u32::from(src[3]);
                    for c in 0..3 {
                        pixel[c] = ((u32::from(src[c]) * a + u32::from(pixel[c]) * (255 - a) + 127)
                            / 255) as u8;
                    }
                }
            }
            pixel[3] = 0xff;
            out.extend_from_slice(&pixel);
        }
    }
    (ww, wh, out)
}

#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_SHELL_SHOTS"]
fn toast_and_badge_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_SHELL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_SHELL_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("shell-badge-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    for case in &CASES {
        for (name, toast) in TOASTS {
            let mut options = load_play_options_from_save(&game).unwrap();
            options.target = PlayTarget::from_key("BRITANNIA").unwrap();
            options.saved_active_objects = None;
            options.start = Some((74, 111));
            options.clock = GameClock::new(11, 50).unwrap();
            let state = PlayState::load_scene(&game, options).unwrap();
            let walk = [KeyCode::ArrowUp, KeyCode::ArrowUp, KeyCode::ArrowRight];
            let mut timeline = Timeline::new(10 + walk.len() * 30 + 40);
            for (index, key) in walk.iter().enumerate() {
                timeline.game_tap(10 + index * 30, *key);
            }
            let mut shell = Shell::new(state, &scene, &launch(case.grow), |_| {});
            shell.app.world_mut().spawn(Window {
                resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
                ..default()
            });
            for index in 0..timeline.len() {
                shell.step(timeline.frame(index));
            }
            {
                let world = shell.app.world_mut();
                *world.resource_mut::<ShellBadges>() = ShellBadges {
                    auto: true,
                    modified: true,
                };
                world.resource_mut::<Toasts>().note(toast);
            }
            shell.step(&[]);
            shell.step(&[]);
            let (w, h, rgba) = window_shot(&mut shell);
            save_png(
                &out.join(format!("shell-{}-{name}.png", case.label)),
                w,
                h,
                &rgba,
            );
        }
    }
}

/// The window with the journal page sprite over the frame when it shows:
/// [`window_shot`] samples the frame sprite only, and the page is a sprite
/// of its own over it.
fn window_shot_with_page(shell: &mut Shell) -> (u32, u32, Vec<u8>) {
    use crate::quest_journal_view::JournalPageSprite;
    let (w, h, mut out) = window_shot(shell);
    let world = shell.app.world_mut();
    let page = world
        .query_filtered::<(&Sprite, &Transform, &Visibility), With<JournalPageSprite>>()
        .iter(world)
        .next()
        .filter(|(_, _, visible)| **visible != Visibility::Hidden)
        .map(|(sprite, transform, _)| (sprite.clone(), *transform));
    let Some((sprite, transform)) = page else {
        return (w, h, out);
    };
    let Some(image) = world.resource::<Assets<Image>>().get(&sprite.image) else {
        return (w, h, out);
    };
    let size = image.size();
    let rgba = image.data.clone().unwrap_or_default();
    let rect = Rect::from_center_size(
        Vec2::new(
            w as f32 / 2.0 + transform.translation.x,
            h as f32 / 2.0 - transform.translation.y,
        ),
        sprite.custom_size.unwrap() * transform.scale.truncate(),
    );
    for y in 0..h {
        for x in 0..w {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if !rect.contains(point) {
                continue;
            }
            let u = (((point.x - rect.min.x) / rect.width()) * size.x as f32) as u32;
            let v = (((point.y - rect.min.y) / rect.height()) * size.y as f32) as u32;
            let i = ((v.min(size.y - 1) * size.x + u.min(size.x - 1)) * 4) as usize;
            let a = u32::from(rgba[i + 3]);
            let o = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                out[o + c] =
                    ((u32::from(rgba[i + c]) * a + u32::from(out[o + c]) * (255 - a) + 127) / 255)
                        as u8;
            }
        }
    }
    (w, h, out)
}

/// The Phase B final review set (M5-M8): the journal page, the clue log,
/// the cheats page, teleport and the change watch, each in a 4:3 window
/// and a 16:9 window with the grow-left layout, and the journal side
/// panel in a plain 16:9 window (grow-left hides the panels by design).
/// Review output only: `U5_MOCKUP_PROFILE` names a game copy with a save
/// and `U5_FINAL_SHOTS` a folder; nothing is committed.
#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_FINAL_SHOTS"]
fn phase_b_final_review_shots() {
    use bevy::input::ButtonState;
    use u5_runtime::enhancements::clues::{
        ClueEntry, DialogKey, EntryFlags, GameMinute, NpcKey, RecordAddr, TestBits, game_minute,
    };
    use u5_runtime::{PlayOptions, Scene as GameScene};

    use crate::enh::equivalence::Owner;
    use crate::enh::journal::clues::ClueLogResource;

    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_FINAL_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_FINAL_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("phase-b-final-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    let town = || {
        let mut state = PlayState::load_scene(
            &game,
            PlayOptions {
                target: PlayTarget::Town(GameScene::from_key("TOWNE:1").unwrap()),
                floor: 0,
                start: Some((15, 17)),
                clock: GameClock::new(10, 0).unwrap(),
                ..PlayOptions::default()
            },
        )
        .unwrap();
        state.shrine_ordained_mask = 0b0000_0110;
        state.shrine_codex_mask = 0b1001_0011;
        state.shrine_ruin_flags[6] = 0x80;
        for member in &mut state.party {
            member.hp = member.max_hp / 3;
        }
        state
    };
    // Three people of the town family with topics, by dialog id.
    let towne = u5_runtime::parse_tlk_raw(&game.join("TOWNE.TLK")).unwrap();
    let mut ids: Vec<u16> = towne
        .iter()
        .filter(|(_, fields)| fields.len() >= 11)
        .map(|(id, _)| *id)
        .collect();
    ids.sort_unstable();
    let now = game_minute(&town()).0;
    let mut entries = Vec::new();
    for (person, id) in ids.iter().take(3).enumerate() {
        let npc = NpcKey {
            scene: 1,
            slot: 2 + person as u8,
        };
        let dialog = DialogKey {
            family: 0,
            dialog_id: *id as u8,
        };
        for (k, minutes) in [(0, 1), (1, 3), (2, 7)] {
            let mut entry = ClueEntry::heard(
                npc,
                dialog,
                RecordAddr::Pair(k),
                0,
                TestBits::default(),
                GameMinute(now - 2000 * (3 - person as u32) + minutes),
            );
            if person == 1 && k == 1 {
                entry.count = 2;
            }
            entries.push(entry);
        }
    }
    let launch_with = |grow: bool, toggles: &[Toggle]| {
        let mut launch = launch(grow);
        launch
            .enhancements
            .toggles
            .set(Toggle::CheatsStateWatch, false);
        for toggle in toggles {
            launch.enhancements.toggles.set(*toggle, true);
        }
        launch
    };
    let ctrl = |timeline: &mut Timeline, frame: usize, key: KeyCode| {
        timeline.event(
            frame,
            KeyCode::ControlLeft,
            ButtonState::Pressed,
            false,
            Owner::Feature,
        );
        timeline.feature_tap(frame + 1, key);
        timeline.event(
            frame + 3,
            KeyCode::ControlLeft,
            ButtonState::Released,
            false,
            Owner::Feature,
        );
    };
    let save = |name: &str, shot: (u32, u32, Vec<u8>)| {
        save_png(&out.join(format!("{name}.png")), shot.0, shot.1, &shot.2);
    };
    let cases = [
        ("43", (1280u32, 960u32), false),
        ("169-grow", (2048, 960), true),
    ];
    for (label, (ww, wh), grow) in cases {
        let new_shell = |toggles: &[Toggle]| {
            let mut shell = Shell::new(town(), &scene, &launch_with(grow, toggles), |_| {});
            shell.app.world_mut().spawn(Window {
                resolution: WindowResolution::new(ww as f32, wh as f32),
                ..default()
            });
            for _ in 0..30 {
                shell.step(&[]);
            }
            shell
        };
        // The journal page and the clue log.
        let mut shell = new_shell(&[
            Toggle::JournalPage,
            Toggle::JournalClues,
            Toggle::JournalQuestTracker,
        ]);
        {
            let mut log = shell.app.world_mut().resource_mut::<ClueLogResource>();
            for entry in &entries {
                log.log_mut().fold(*entry);
            }
            log.log_mut().set_flag(2, EntryFlags::STAR, true);
        }
        let mut timeline = Timeline::new(60);
        ctrl(&mut timeline, 2, KeyCode::KeyJ);
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
        }
        save(
            &format!("journal-quests-{label}"),
            window_shot_with_page(&mut shell),
        );
        let mut timeline = Timeline::new(40);
        timeline.feature_tap(2, KeyCode::Tab);
        timeline.feature_tap(6, KeyCode::Enter);
        timeline.feature_tap(10, KeyCode::ArrowDown);
        timeline.feature_tap(14, KeyCode::ArrowDown);
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
        }
        save(
            &format!("journal-clues-{label}"),
            window_shot_with_page(&mut shell),
        );

        // The cheats page, teleport and the change watch.
        let mut cheats: Vec<Toggle> = u5_runtime::enhancements::TOGGLES
            .iter()
            .filter(|spec| spec.toggle.key().starts_with("cheats."))
            .map(|spec| spec.toggle)
            .collect();
        cheats.retain(|toggle| *toggle != Toggle::CheatsRememberLocks);
        let mut shell = new_shell(&cheats);
        let mut timeline = Timeline::new(90);
        ctrl(&mut timeline, 2, KeyCode::F12);
        timeline.feature_tap(10, KeyCode::KeyH);
        timeline.feature_tap(16, KeyCode::KeyT);
        timeline.feature_tap(22, KeyCode::ArrowRight);
        timeline.feature_tap(28, KeyCode::Escape);
        timeline.feature_tap(34, KeyCode::KeyW);
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
            match frame {
                14 => save(&format!("cheats-main-{label}"), window_shot(&mut shell)),
                26 => save(&format!("cheats-teleport-{label}"), window_shot(&mut shell)),
                _ => {}
            }
        }
        save(&format!("cheats-watch-{label}"), window_shot(&mut shell));
    }

    // The journal side panel in a plain 16:9 window.
    let mut shell = Shell::new(
        town(),
        &scene,
        &launch_with(false, &[Toggle::JournalSidePanel, Toggle::JournalClues]),
        |_| {},
    );
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(1706.0, 960.0),
        ..default()
    });
    for _ in 0..40 {
        shell.step(&[]);
    }
    save("journal-side-panel-169", window_shot(&mut shell));
    let _ = fs::remove_dir_all(game);
}
