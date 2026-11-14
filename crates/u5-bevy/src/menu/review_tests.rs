//! The M4-05 render check: the menu in the real shell over real engine
//! frames, read back from the shell's own images (the presented frame or
//! canvas under the backdrop under the menu node), for the user to hold
//! beside the M4-01 sheet (build plan M4-05).
//!
//! Review output only, never committed, and `#[ignore]`d: run with
//! `U5_MOCKUP_PROFILE` naming a `test-assets` or `prepare-profile` game copy
//! and `U5_MENU_SHOTS` an output folder:
//!
//! ```text
//! U5_MOCKUP_PROFILE=<copy> U5_MENU_SHOTS=<folder> \
//!   cargo test -p u5-bevy --lib menu::review_tests -- --ignored
//! ```
//!
//! The game copy is copied to a scratch folder first. The scene is the
//! c04f mockups' forest road west of Britain at 11:50, walked up, up,
//! right, right, up through the shell's keyboard. Each case opens the menu
//! with F12 and writes, at the picture's size and at 2x: the top menu, the
//! bar on Settings, the Settings page, the open sweep's frames and the
//! close sweep's; 4x close-ups of the corners, the tabs, the divider's
//! junctions and both ends of the bar; and one shot of the whole window,
//! bars and all, as the GPU would place the nodes.

use std::fs;
use std::path::Path;

use bevy::input::ButtonState;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, ResolveInput, Toggle, resolve,
};
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayState, PlayTarget, load_play_options_from_save};

use super::frame::{CLEAR, DIM, MenuImage, composite_over};
use super::geometry::CellRect;
use super::render::{MenuBackdrop, MenuNode, MenuView, frame_rect};
use super::{MenuShell, MenuState};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline};
use crate::enh::present::PresentHost;
use bevy::prelude::*;

pub(super) fn save_png(path: &Path, w: u32, h: u32, rgba: &[u8]) {
    image::RgbaImage::from_raw(w, h, rgba.to_vec())
        .unwrap()
        .save(path)
        .unwrap();
}

pub(super) fn scaled(w: u32, h: u32, rgba: &[u8], k: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * k * k * 4) as usize);
    for y in 0..h * k {
        for x in 0..w * k {
            let i = (((y / k) * w + x / k) * 4) as usize;
            out.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    out
}

pub(super) fn crop(
    w: u32,
    h: u32,
    rgba: &[u8],
    (x0, y0, cw, ch): (i64, i64, u32, u32),
) -> (u32, u32, Vec<u8>) {
    let x0 = x0.clamp(0, i64::from(w) - 1) as u32;
    let y0 = y0.clamp(0, i64::from(h) - 1) as u32;
    let (cw, ch) = (cw.min(w - x0), ch.min(h - y0));
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in y0..y0 + ch {
        let i = ((y * w + x0) * 4) as usize;
        out.extend_from_slice(&rgba[i..i + (cw * 4) as usize]);
    }
    (cw, ch, out)
}

pub(super) struct Case {
    pub(super) label: &'static str,
    pub(super) window: (u32, u32),
    pub(super) grow: bool,
    pub(super) column: bool,
}

pub(super) const CASES: [Case; 5] = [
    Case {
        label: "43-1600x1200",
        window: (1600, 1200),
        grow: false,
        column: false,
    },
    Case {
        label: "169-2560x1440",
        window: (2560, 1440),
        grow: true,
        column: false,
    },
    Case {
        label: "169-2560x1440-column",
        window: (2560, 1440),
        grow: true,
        column: true,
    },
    Case {
        label: "219-3440x1440",
        window: (3440, 1440),
        grow: true,
        column: false,
    },
    Case {
        label: "43-1366x768-bars",
        window: (1366, 768),
        grow: false,
        column: false,
    },
];

pub(super) fn launch(grow: bool) -> VisualLaunchConfig {
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
    enhancements.toggles.set(Toggle::MenuOverlay, true);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

/// The picture the window shows under the menu: the presented image, or
/// the faithful frame without a pipeline.
pub(super) fn game_picture(shell: &Shell) -> (UVec2, Vec<u8>) {
    let world = shell.app.world();
    let handle = world
        .get_resource::<PresentHost>()
        .and_then(PresentHost::presented)
        .cloned()
        .unwrap_or_else(|| shell.visual().image_handle.clone());
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    (image.size(), image.data.clone().unwrap())
}

/// The menu node's picture and whether the backdrop is up.
fn menu_picture(shell: &mut Shell) -> (Option<(UVec2, Vec<u8>)>, bool) {
    let world = shell.app.world_mut();
    let mut nodes = world.query_filtered::<&ImageNode, With<MenuNode>>();
    let handle = nodes.iter(world).next().map(|node| node.image.clone());
    let mut backdrops = world.query_filtered::<(), With<MenuBackdrop>>();
    let backdrop = backdrops.iter(world).next().is_some();
    let picture = handle.map(|handle| {
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        (image.size(), image.data.clone().unwrap())
    });
    (picture, backdrop)
}

/// The picture as the window shows it: the game, the backdrop's dim, the
/// menu over both.
pub(super) fn composed(shell: &mut Shell) -> (u32, u32, Vec<u8>) {
    let (mut size, mut game) = game_picture(shell);
    let (menu, backdrop) = menu_picture(shell);
    // A prescaled presentation (`scale_mode = sharp`) shows the canvas
    // scaled up; the menu is drawn on the canvas's own grid. Sample the
    // presented picture at each canvas pixel's centre.
    if let Some((msize, _)) = &menu
        && *msize != size
    {
        let mut small = Vec::with_capacity((msize.x * msize.y * 4) as usize);
        for y in 0..msize.y {
            for x in 0..msize.x {
                let gx = ((2 * x + 1) * size.x) / (2 * msize.x);
                let gy = ((2 * y + 1) * size.y) / (2 * msize.y);
                let i = ((gy * size.x + gx) * 4) as usize;
                small.extend_from_slice(&game[i..i + 4]);
            }
        }
        size = *msize;
        game = small;
    }
    let mut out = game;
    if backdrop {
        let mut dim = MenuImage::new(size.x, size.y);
        dim.rgba.chunks_mut(4).for_each(|p| p.copy_from_slice(&DIM));
        out = composite_over(&out, &dim);
    }
    if let Some((msize, rgba)) = menu {
        assert_eq!(msize, size, "the menu is the picture's size");
        out = composite_over(
            &out,
            &MenuImage {
                w: size.x,
                h: size.y,
                rgba,
            },
        );
    }
    (size.x, size.y, out)
}

/// The whole window as the nodes place it: black, the frame sprite
/// sampled nearest over its rectangle, the backdrop over everything and
/// the menu node over the sprite's rectangle.
pub(super) fn window_shot(shell: &mut Shell) -> (u32, u32, Vec<u8>) {
    let (size, game) = game_picture(shell);
    let (menu, backdrop) = menu_picture(shell);
    let world = shell.app.world_mut();
    let window = world.query::<&Window>().iter(world).next().unwrap().clone();
    let (sprite, transform) = world
        .query::<(&Sprite, &Transform)>()
        .iter(world)
        .next()
        .map(|(s, t)| (s.clone(), *t))
        .unwrap();
    let rect = frame_rect(&window, &sprite, &transform).unwrap();
    let mut node_rect = None;
    let mut q = world.query_filtered::<&Node, With<MenuNode>>();
    if let Some(node) = q.iter(world).next() {
        let px = |v: Val| match v {
            Val::Px(px) => px,
            _ => panic!(),
        };
        node_rect = Some(Rect::new(
            px(node.left),
            px(node.top),
            px(node.left) + px(node.width),
            px(node.top) + px(node.height),
        ));
    }
    let mut toast = None;
    let mut strips =
        world.query_filtered::<(&ImageNode, &Node), With<crate::enh::toast::ToastStrip>>();
    if let Some((image, node)) = strips.iter(world).next() {
        let px = |v: Val| match v {
            Val::Px(px) => px,
            _ => panic!(),
        };
        let rect = Rect::new(
            px(node.left),
            px(node.top),
            px(node.left) + px(node.width),
            px(node.top) + px(node.height),
        );
        let image = world.resource::<Assets<Image>>().get(&image.image).unwrap();
        toast = Some((rect, image.size(), image.data.clone().unwrap()));
    }
    let (ww, wh) = (
        window.resolution.physical_width(),
        window.resolution.physical_height(),
    );
    let scale = window.resolution.scale_factor();
    let sample = |r: Rect, rgba: &[u8], x: u32, y: u32| -> Option<[u8; 4]> {
        let (lx, ly) = ((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
        if !r.contains(Vec2::new(lx, ly)) {
            return None;
        }
        let u = (((lx - r.min.x) / r.width()) * size.x as f32) as u32;
        let v = (((ly - r.min.y) / r.height()) * size.y as f32) as u32;
        let i = ((v.min(size.y - 1) * size.x + u.min(size.x - 1)) * 4) as usize;
        Some([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
    };
    let mut out = Vec::with_capacity((ww * wh * 4) as usize);
    for y in 0..wh {
        for x in 0..ww {
            let mut pixel = sample(rect, &game, x, y).unwrap_or([0, 0, 0, 0xff]);
            let blend = |dst: &mut [u8; 4], src: [u8; 4]| {
                let a = u32::from(src[3]);
                for c in 0..3 {
                    dst[c] =
                        ((u32::from(src[c]) * a + u32::from(dst[c]) * (255 - a) + 127) / 255) as u8;
                }
            };
            // The window's side fill is black without a fill feature.
            if backdrop {
                blend(&mut pixel, DIM);
            }
            if let (Some(r), Some((_, rgba))) = (node_rect, menu.as_ref())
                && let Some(src) = sample(r, rgba, x, y)
                && src != CLEAR
            {
                blend(&mut pixel, src);
            }
            // The toast strip is the top node (M4-10 shots show it).
            if let Some((r, tsize, rgba)) = toast.as_ref() {
                let (lx, ly) = ((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
                if r.contains(Vec2::new(lx, ly)) {
                    let u = (((lx - r.min.x) / r.width()) * tsize.x as f32) as u32;
                    let v = (((ly - r.min.y) / r.height()) * tsize.y as f32) as u32;
                    let i = ((v.min(tsize.y - 1) * tsize.x + u.min(tsize.x - 1)) * 4) as usize;
                    pixel = [rgba[i], rgba[i + 1], rgba[i + 2], 0xff];
                }
            }
            out.extend_from_slice(&pixel);
        }
    }
    (ww, wh, out)
}

struct Writer<'a> {
    out: &'a Path,
    label: &'static str,
}

impl Writer<'_> {
    fn shot(&self, name: &str, (w, h, rgba): &(u32, u32, Vec<u8>)) {
        save_png(
            &self.out.join(format!("{}-{name}.png", self.label)),
            *w,
            *h,
            rgba,
        );
        save_png(
            &self.out.join(format!("{}-{name}-2x.png", self.label)),
            w * 2,
            h * 2,
            &scaled(*w, *h, rgba, 2),
        );
    }

    fn closeup(&self, name: &str, (w, h, rgba): &(u32, u32, Vec<u8>), area: (i64, i64, u32, u32)) {
        let (cw, ch, part) = crop(*w, *h, rgba, area);
        save_png(
            &self
                .out
                .join(format!("{}-closeup-{name}-4x.png", self.label)),
            cw * 4,
            ch * 4,
            &scaled(cw, ch, &part, 4),
        );
    }
}

/// The close-ups of one page's panel, in picture pixels.
fn closeups(
    rect: CellRect,
    divider_rows_from_bottom: u32,
) -> Vec<(&'static str, (i64, i64, u32, u32))> {
    let (x0, y0, x1, y1) = rect.pixels();
    let (x0, y0, x1, y1) = (i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1));
    let mid = (x0 + x1) / 2;
    let divider = i64::from((rect.bottom() - divider_rows_from_bottom) * 8);
    vec![
        ("corner-top-left", (x0 - 8, y0 - 8, 40, 32)),
        ("corner-top-right", (x1 - 31, y0 - 8, 40, 32)),
        ("corner-bottom-left", (x0 - 8, y1 - 23, 40, 32)),
        ("corner-bottom-right", (x1 - 31, y1 - 23, 40, 32)),
        ("title-tab", (mid - 40, y0 - 4, 80, 20)),
        ("footer-tab", (mid - 56, y1 - 15, 112, 20)),
        ("junction-left", (x0 - 4, divider - 12, 32, 32)),
        ("junction-right", (x1 - 27, divider - 12, 32, 32)),
        ("bar-left", (x0, y0 + 8, 48, 32)),
        ("bar-right", (x1 - 47, y0 + 8, 48, 32)),
    ]
}

#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_MENU_SHOTS"]
fn menu_shell_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("menu-review-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    for case in &CASES {
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
        if case.column {
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
        // The default launch's menu (with the intro), slots off.
        let mut shell = Shell::new(state, &scene, &launch(case.grow), |app| {
            app.insert_resource(MenuShell {
                intro_shell: true,
                slots: false,
                shipped: u5_runtime::enhancements::SHIPPED_PHASES,
            });
        });
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
            ..default()
        });
        for index in 0..timeline.len() {
            shell.step(timeline.frame(index));
        }
        let w = Writer {
            out: &out,
            label: case.label,
        };
        w.shot("before", &composed(&mut shell));

        let key = |shell: &mut Shell, key: KeyCode, state: ButtonState| {
            shell.step(&[crate::enh::equivalence::TimedKey {
                key,
                state,
                repeat: false,
                owner: Owner::Game,
            }]);
        };
        // The open sweep, one frame at a time.
        key(&mut shell, KeyCode::F12, ButtonState::Pressed);
        let mut n = 0;
        loop {
            let step = shell.app.world().resource::<MenuView>().sweep.step;
            w.shot(&format!("open-{n:02}"), &composed(&mut shell));
            n += 1;
            if step.is_none() {
                break;
            }
            if n == 1 {
                key(&mut shell, KeyCode::F12, ButtonState::Released);
            } else {
                shell.step(&[]);
            }
        }
        let top = composed(&mut shell);
        w.shot("top", &top);
        let drawn = shell
            .app
            .world()
            .resource::<MenuView>()
            .drawn()
            .unwrap()
            .clone();
        for (name, area) in closeups(drawn.panel, 4) {
            w.closeup(name, &top, area);
        }
        let win = window_shot(&mut shell);
        save_png(
            &out.join(format!("{}-window-top.png", case.label)),
            win.0,
            win.1,
            &win.2,
        );

        key(&mut shell, KeyCode::ArrowDown, ButtonState::Pressed);
        key(&mut shell, KeyCode::ArrowDown, ButtonState::Released);
        let on_settings = composed(&mut shell);
        w.shot("top-settings-selected", &on_settings);

        key(&mut shell, KeyCode::Enter, ButtonState::Pressed);
        key(&mut shell, KeyCode::Enter, ButtonState::Released);
        assert!(
            shell
                .app
                .world()
                .resource::<MenuState>()
                .model()
                .is_some_and(|m| m.stack().len() == 2)
        );
        let settings = composed(&mut shell);
        w.shot("settings", &settings);
        let drawn = shell
            .app
            .world()
            .resource::<MenuView>()
            .drawn()
            .unwrap()
            .clone();
        for (name, area) in closeups(drawn.panel, 5) {
            w.closeup(&format!("settings-{name}"), &settings, area);
        }

        // Back, then the close sweep.
        key(&mut shell, KeyCode::Escape, ButtonState::Pressed);
        key(&mut shell, KeyCode::Escape, ButtonState::Released);
        key(&mut shell, KeyCode::Escape, ButtonState::Pressed);
        let mut n = 0;
        loop {
            let visible = shell.app.world().resource::<MenuView>().sweep.visible();
            w.shot(&format!("close-{n:02}"), &composed(&mut shell));
            n += 1;
            if !visible {
                break;
            }
            if n == 1 {
                key(&mut shell, KeyCode::Escape, ButtonState::Released);
            } else {
                shell.step(&[]);
            }
        }
        eprintln!("{}: written", case.label);
    }
}

/// The M4-07 render check: every Settings tab in the real shell over the
/// same real frames, for the user to hold beside the M4-01 sheet (build
/// plan M4-07). As [`menu_shell_review_shots`], review output only and
/// `#[ignore]`d; `U5_MENU_SHOTS` names the output folder. The menu's own
/// phases are listed as shipped (they ship in M4-13), so the Audio tab and
/// the menu's rows show; their values are the defaults.
#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_MENU_SHOTS"]
fn settings_review_shots() {
    use super::pages::{Page, RowKind, SettingsTab, page_rows};
    use super::world_snapshot;

    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("menu-settings-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    let cases = [&CASES[0], &CASES[1], &CASES[2], &CASES[3]];
    for case in cases {
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
        if case.column {
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
        let mut shell = Shell::new(state, &scene, &launch(case.grow), |app| {
            app.insert_resource(MenuShell {
                intro_shell: true,
                slots: false,
                shipped: super::pages_settings_tests::with_menu_phases(),
            });
        });
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
            ..default()
        });
        for index in 0..timeline.len() {
            shell.step(timeline.frame(index));
        }
        let w = Writer {
            out: &out,
            label: case.label,
        };
        let tap = |shell: &mut Shell, key: KeyCode| {
            for state in [ButtonState::Pressed, ButtonState::Released] {
                shell.step(&[crate::enh::equivalence::TimedKey {
                    key,
                    state,
                    repeat: false,
                    owner: Owner::Game,
                }]);
            }
        };
        let current = |shell: &Shell| {
            shell
                .app
                .world()
                .resource::<MenuState>()
                .model()
                .unwrap()
                .current()
        };
        // Down (or up) to the row labelled `label` on the page showing.
        let to = |shell: &mut Shell, label: &str| {
            let page = current(shell).page;
            let rows = page_rows(page, &world_snapshot(shell.app.world()));
            let at = rows
                .iter()
                .position(|row| row.label == label)
                .unwrap_or_else(|| panic!("no row {label} on {page:?}"));
            for _ in 0..rows.len() {
                let cursor = current(shell).cursor;
                if cursor == at {
                    break;
                }
                tap(
                    shell,
                    if cursor < at {
                        KeyCode::ArrowDown
                    } else {
                        KeyCode::ArrowUp
                    },
                );
            }
            assert_eq!(current(shell).cursor, at, "{label}");
        };
        // A whole shot, its 4x and the panel's close-ups.
        let shot = |shell: &mut Shell, name: &str, divider_rows: &[u32]| {
            let picture = composed(shell);
            w.shot(name, &picture);
            save_png(
                &out.join(format!("{}-{name}-4x.png", case.label)),
                picture.0 * 4,
                picture.1 * 4,
                &scaled(picture.0, picture.1, &picture.2, 4),
            );
            let drawn = shell
                .app
                .world()
                .resource::<MenuView>()
                .drawn()
                .unwrap()
                .clone();
            for (i, rows) in divider_rows.iter().enumerate() {
                for (part, area) in closeups(drawn.panel, *rows) {
                    if i > 0 && !part.starts_with("junction") {
                        continue;
                    }
                    let part = if i > 0 {
                        format!("upper-{part}")
                    } else {
                        part.to_string()
                    };
                    w.closeup(&format!("{name}-{part}"), &picture, area);
                }
            }
        };
        let open = |shell: &mut Shell| {
            tap(shell, KeyCode::F12);
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
        };

        open(&mut shell);
        shot(&mut shell, "top", &[4]);
        to(&mut shell, crate::menu::text::SETTINGS);
        tap(&mut shell, KeyCode::Enter);
        assert_eq!(current(&shell).page, Page::Settings(SettingsTab::Audio));
        shot(&mut shell, "audio", &[5]);
        to(&mut shell, crate::menu::text::MUTE);
        shot(&mut shell, "audio-mute", &[5]);

        tap(&mut shell, KeyCode::ArrowRight);
        assert_eq!(current(&shell).page, Page::Settings(SettingsTab::Display));
        shot(&mut shell, "display", &[5]);
        to(&mut shell, "Wider world view");
        shot(&mut shell, "display-grow", &[5]);
        to(&mut shell, "Brightness");
        for _ in 0..6 {
            tap(&mut shell, KeyCode::ArrowLeft);
        }
        shell.step(&[]);
        shot(&mut shell, "display-brightness", &[5]);
        to(&mut shell, "Fullscreen");

        tap(&mut shell, KeyCode::ArrowRight);
        assert_eq!(current(&shell).page, Page::Settings(SettingsTab::Game));
        shot(&mut shell, "game", &[5]);
        tap(&mut shell, KeyCode::ArrowRight);
        assert_eq!(
            current(&shell).page,
            Page::Settings(SettingsTab::Enhancements)
        );
        // The panel's height, for the upper divider's close-ups.
        let panel_h = shell
            .app
            .world()
            .resource::<MenuView>()
            .drawn()
            .unwrap()
            .panel
            .h;
        let upper = panel_h - 1 - (1 + super::render::PRESET_ROWS);
        shot(&mut shell, "extras-preset", &[5, upper]);
        to(&mut shell, "Messages over map");
        shot(&mut shell, "extras", &[5, upper]);
        to(&mut shell, "Spoiler: symbols");
        shot(&mut shell, "extras-spoiler", &[5, upper]);
        let rows = page_rows(current(&shell).page, &world_snapshot(shell.app.world()));
        assert!(matches!(rows[0].kind, RowKind::Preset { .. }));
        to(&mut shell, crate::menu::text::PRESET);
        tap(&mut shell, KeyCode::ArrowRight);
        shell.step(&[]);
        shot(&mut shell, "extras-recommended", &[5, upper]);
        tap(&mut shell, KeyCode::ArrowRight);
        shell.step(&[]);
        shot(&mut shell, "extras-everything", &[5, upper]);
        to(&mut shell, "Sprites on terrain");
        tap(&mut shell, KeyCode::Space);
        shell.step(&[]);
        to(&mut shell, crate::menu::text::PRESET);
        shot(&mut shell, "extras-custom", &[5, upper]);
        eprintln!("{}: written", case.label);
    }
}
