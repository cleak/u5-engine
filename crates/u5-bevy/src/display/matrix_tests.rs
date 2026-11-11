//! The display chapter's trajectory matrix (Phase B design 2.4.3 and 2.6,
//! build plan M1-16).
//!
//! Every script is run headless through the real `--visual` app, once with
//! every key at its default (the off run) and once per configuration:
//!
//! - **alone**: each `[display]` toggle, and each non-default value of a
//!   valued key, with nothing else on. A grow-left sub-toggle is run with
//!   `display.grow_left` on, since it is inert without it; `grow_left` itself
//!   also runs at the named window sizes and on a narrow and a 4K window;
//! - **column**: everything at its grow-left default with `dither` on, and
//!   Ctrl+P typed after every fifth game key, so the column opens and
//!   closes with its effect all through the script;
//! - **pairs** (the shortest script): every pair of the chapter's keys, one
//!   value each, with Ctrl+P as in the column run, and each chapter 9
//!   value with the wider view and with sharp scaling.
//!
//! Each run is compared with the off run frame by frame on the
//! [`TrajectoryDigest`](u5_runtime::enhancements::TrajectoryDigest) (which
//! holds `prng_state`), the original 320x200 frame and the game's key
//! queue, then on the logic tick count after every frame (each world tick
//! steps the animation clock) and on the bytes of every save file a
//! scripted `Q`, `Y` writes from the final state. Ctrl+P is the feature's
//! own key, so the off run does not see it.
//!
//! The scripts run on synthetic states in every `cargo test`, and on the
//! real game files when `U5_TEST_ASSET_DIR` names a writable copy.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, ResolveInput, fnv1a64, resolve,
};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, copy_asset_writable, open_dungeon_record, open_grid,
    open_world_grid, saved_game_seed_bytes, test_state, unique_scratch_dir, world_state,
    write_empty_ool_mirrors,
};
use u5_runtime::{
    BRIT_OOL_FILENAME, CH_FONT_LEN, DungeonScene, GameClock, IBM_CH_FILE, PlayOptions, PlayState,
    PlayTarget, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, TileGraphicsDepth, UNDER_OOL_FILENAME,
    WorldPlane, handle_play_key_input, parse_ch_font, potion_flash_playback,
};

use super::composite::CompositeMasks;
use super::grow::column::ColumnState;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{
    FrameRecord, Owner, Scene, Shell, ShellRun, Timeline, compare_shell_runs,
};
use crate::enh::present::{PresentHost, PresentSteps, StepOrder};

/// The window most runs use: the named 16:9 layout (a 512x240 canvas).
const W16: (f32, f32) = (2560.0, 1440.0);

/// The save files the scripted save is compared on.
const SAVE_FILES: [&str; 4] = [
    SAVED_GAM_FILENAME,
    SAVED_OOL_FILENAME,
    BRIT_OOL_FILENAME,
    UNDER_OOL_FILENAME,
];

// ---------------------------------------------------------------------------
// Configurations.

fn on(key: &str) -> CliSetting {
    CliSetting::Enable(key.into())
}

fn off(key: &str) -> CliSetting {
    CliSetting::Disable(key.into())
}

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.into(),
        value: value.into(),
    }
}

/// One configuration of the chapter's keys.
#[derive(Clone)]
struct Config {
    name: String,
    settings: Vec<CliSetting>,
    window: (f32, f32),
    /// Ctrl+P after every fifth game key.
    column_keys: bool,
}

impl Config {
    fn new(name: &str, settings: Vec<CliSetting>) -> Self {
        Self {
            name: name.to_string(),
            settings,
            window: W16,
            column_keys: false,
        }
    }

    fn grow_left(&self) -> bool {
        self.settings
            .iter()
            .any(|s| matches!(s, CliSetting::Enable(key) if key == "display.grow_left"))
    }

    /// Whether the settings dim the map view (chapter 9).
    fn brightness(&self) -> bool {
        self.settings.iter().any(
            |s| matches!(s, CliSetting::Set { key, value } if key == "display.brightness" && value != "100"),
        )
    }

    /// Whether the settings composite sprites over terrain (chapter 9).
    fn composite(&self) -> bool {
        self.settings
            .iter()
            .any(|s| matches!(s, CliSetting::Enable(key) if key == "display.sprite_composite"))
    }

    /// The launch a host resolves for these settings, with the shipped
    /// catalogue. Every key must resolve with no note.
    fn launch(&self) -> VisualLaunchConfig {
        let enhancements = resolve(ResolveInput {
            stored: None,
            cli: &self.settings,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        });
        assert!(
            enhancements.notes.is_empty(),
            "{}: {:?}",
            self.name,
            enhancements.notes
        );
        VisualLaunchConfig {
            enhancements,
            ..VisualLaunchConfig::classic()
        }
    }
}

/// The grow-left sub-keys, each away from its default, with the layout on.
fn grow_sub(key: &str, value: CliSetting) -> Config {
    Config::new(key, vec![on("display.grow_left"), value])
}

/// Each `[display]` key alone (2.6 "run once per toggle"), every
/// non-default value of a valued key, and `grow_left` on every named
/// window, a narrow one and 4K.
fn alone() -> Vec<Config> {
    let mut configs = vec![
        Config::new("scale_mode=sharp", vec![set("display.scale_mode", "sharp")]),
        Config::new(
            "scale_mode=integer-square",
            vec![set("display.scale_mode", "integer-square")],
        ),
        Config::new(
            "scale_mode=integer-hybrid",
            vec![set("display.scale_mode", "integer-hybrid")],
        ),
        Config::new(
            "scale_mode=stretch",
            vec![set("display.scale_mode", "stretch")],
        ),
        Config::new(
            "pixel_aspect=square",
            vec![set("display.pixel_aspect", "square")],
        ),
        Config::new("fullscreen", vec![on("display.fullscreen")]),
        Config::new("fullscreen_key", vec![on("display.fullscreen_key")]),
        Config::new(
            "pillar_fill=solid",
            vec![
                set("display.pillar_fill", "solid"),
                set("display.pillar_color", "#AA5500"),
            ],
        ),
        Config::new(
            "pillar_fill=ribbon",
            vec![set("display.pillar_fill", "ribbon")],
        ),
        Config::new(
            "pillar_fill=bezel",
            vec![set("display.pillar_fill", "bezel")],
        ),
        Config::new("vsync=false", vec![off("display.vsync")]),
        Config::new("grow_left", vec![on("display.grow_left")]),
        grow_sub(
            "grow_left_light=faithful",
            set("display.grow_left_light", "faithful"),
        ),
        grow_sub("crawl=false", off("display.crawl")),
        grow_sub("typed_colour=false", off("display.typed_colour")),
        grow_sub("ribbon_hp_colours=false", off("display.ribbon_hp_colours")),
        grow_sub("dither=false", off("display.dither")),
        grow_sub("centre_party=true", on("display.centre_party")),
    ];
    configs.extend(light_alone());
    for (name, window) in [
        ("grow_left@3440x1440", (3440.0, 1440.0)),
        ("grow_left@1920x1080", (1920.0, 1080.0)),
        ("grow_left@1280x1024", (1280.0, 1024.0)),
        ("grow_left@3840x2160", (3840.0, 2160.0)),
    ] {
        configs.push(Config {
            window,
            ..Config::new(name, vec![on("display.grow_left")])
        });
    }
    configs
}

/// Chapter 9's keys (design 9.4.3, build plan M2-08), each alone: the
/// suggested and the lowest brightness, the whole-frame scope (with the
/// suggested level, since the scope is inert without it), compositing, and
/// compositing without its outline.
fn light_values() -> Vec<(&'static str, Vec<CliSetting>)> {
    vec![
        ("brightness=70", vec![set("display.brightness", "70")]),
        ("brightness=25", vec![set("display.brightness", "25")]),
        (
            "brightness_scope=frame",
            vec![
                set("display.brightness", "70"),
                set("display.brightness_scope", "frame"),
            ],
        ),
        ("sprite_composite", vec![on("display.sprite_composite")]),
        (
            "composite_halo=false",
            vec![
                on("display.sprite_composite"),
                off("display.composite_halo"),
            ],
        ),
    ]
}

fn light_alone() -> Vec<Config> {
    light_values()
        .into_iter()
        .map(|(name, settings)| Config::new(name, settings))
        .collect()
}

/// Each of [`light_values`] with the wider world view and with sharp
/// scaling (build plan M2-08), Ctrl+P typed as in [`column`]. The pairs
/// among the chapter's own keys and with every other display key come
/// from [`pair_values`].
fn light_pairs() -> Vec<Config> {
    let partners = [
        ("grow_left", vec![on("display.grow_left")]),
        ("scale_mode=sharp", vec![set("display.scale_mode", "sharp")]),
    ];
    let mut configs = Vec::new();
    for (light, settings) in light_values() {
        for (partner, with) in &partners {
            let mut both = settings.clone();
            both.extend(with.iter().cloned());
            configs.push(Config {
                column_keys: true,
                ..Config::new(&format!("{light} + {partner}"), both)
            });
        }
    }
    configs
}

/// The grow-left layout at its defaults (dither on) with the column opened
/// and closed by Ctrl+P after every fifth game key (2.6).
fn column() -> Config {
    Config {
        column_keys: true,
        ..Config::new("column (Ctrl+P every fifth key, dither on)", {
            vec![on("display.grow_left"), on("display.dither")]
        })
    }
}

/// One value per key of the chapter, for the pairwise matrix. A grow-left
/// sub-key brings `grow_left` with it.
fn pair_values() -> Vec<(&'static str, Vec<CliSetting>)> {
    vec![
        ("scale_mode=sharp", vec![set("display.scale_mode", "sharp")]),
        (
            "pixel_aspect=square",
            vec![set("display.pixel_aspect", "square")],
        ),
        ("fullscreen", vec![on("display.fullscreen")]),
        ("fullscreen_key", vec![on("display.fullscreen_key")]),
        (
            "pillar_fill=ribbon",
            vec![set("display.pillar_fill", "ribbon")],
        ),
        ("vsync=false", vec![off("display.vsync")]),
        ("grow_left", vec![on("display.grow_left")]),
        (
            "grow_left_light=faithful",
            vec![
                on("display.grow_left"),
                set("display.grow_left_light", "faithful"),
            ],
        ),
        (
            "crawl=false",
            vec![on("display.grow_left"), off("display.crawl")],
        ),
        (
            "typed_colour=false",
            vec![on("display.grow_left"), off("display.typed_colour")],
        ),
        (
            "ribbon_hp_colours=false",
            vec![on("display.grow_left"), off("display.ribbon_hp_colours")],
        ),
        (
            "dither=false",
            vec![on("display.grow_left"), off("display.dither")],
        ),
        (
            "centre_party=true",
            vec![on("display.grow_left"), on("display.centre_party")],
        ),
        ("brightness=70", vec![set("display.brightness", "70")]),
        (
            "brightness_scope=frame",
            vec![
                set("display.brightness", "25"),
                set("display.brightness_scope", "frame"),
            ],
        ),
        ("sprite_composite", vec![on("display.sprite_composite")]),
        (
            "composite_halo=false",
            vec![
                on("display.sprite_composite"),
                off("display.composite_halo"),
            ],
        ),
    ]
}

/// Every pair of [`pair_values`], with Ctrl+P typed as in [`column`].
fn pairs() -> Vec<Config> {
    let values = pair_values();
    let mut configs = Vec::new();
    for (i, (a, first)) in values.iter().enumerate() {
        for (b, second) in &values[i + 1..] {
            let mut settings = first.clone();
            for setting in second {
                if !settings.contains(setting) {
                    settings.push(setting.clone());
                }
            }
            configs.push(Config {
                column_keys: true,
                ..Config::new(&format!("{a} + {b}"), settings)
            });
        }
    }
    configs
}

// ---------------------------------------------------------------------------
// Scripts.

/// A start state, the files it draws with, a folder holding the seed save
/// the scripted save writes over, and the player's keys.
struct Script {
    name: &'static str,
    state: PlayState,
    scene: Scene,
    save_seed: PathBuf,
    keys: Vec<KeyCode>,
    /// The plain top-down view, where the grow-left canvas is drawn.
    top_down: bool,
}

/// Frames between the player's keys, and after the last.
const KEY_SPACING: usize = 15;
const TAIL: usize = 40;

impl Script {
    /// The player's keys, one tap every [`KEY_SPACING`] frames; with
    /// `column_keys`, Ctrl+P after every fifth.
    fn timeline(&self, column_keys: bool) -> Timeline {
        let mut timeline = Timeline::new(5 + self.keys.len() * KEY_SPACING + TAIL);
        for (index, key) in self.keys.iter().enumerate() {
            let frame = 5 + index * KEY_SPACING;
            timeline.game_tap(frame, *key);
            if column_keys && index % 5 == 4 {
                ctrl_p(&mut timeline, frame + 5);
            }
        }
        timeline
    }
}

/// Ctrl+P at `frame`: Ctrl held around a tap of P, all the feature's.
fn ctrl_p(timeline: &mut Timeline, frame: usize) {
    for (at, key, state) in [
        (frame, KeyCode::ControlLeft, ButtonState::Pressed),
        (frame, KeyCode::KeyP, ButtonState::Pressed),
        (frame + 1, KeyCode::KeyP, ButtonState::Released),
        (frame + 1, KeyCode::ControlLeft, ButtonState::Released),
    ] {
        timeline.event(at, key, state, false, Owner::Feature);
    }
}

/// A scratch folder, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        Self(unique_scratch_dir(&format!("display-matrix-{label}")))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A font whose glyphs all have ink and differ, so text on the frame and
/// the canvas means something.
fn inked_scene() -> Scene {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    for code in 0..CH_FONT_LEN / 8 {
        let c = code as u8;
        let rows = [c | 1, !c, c | 0x80, c, !c | 1, c ^ 0x55, c | 0x10, 0x81];
        bytes[code * 8..code * 8 + 8].copy_from_slice(&rows);
    }
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    Scene {
        game_dir: ".".into(),
        atlas: u5_runtime::test_fixtures::synthetic_tile_atlas(TileGraphicsDepth::Ega16),
        text_font: font.clone(),
        rune_font: font,
    }
}

/// A folder holding the synthetic seed save.
fn synthetic_save_seed(scratch: &Scratch) -> PathBuf {
    let dir = scratch.path().join("seed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(SAVED_GAM_FILENAME),
        saved_game_seed_bytes(0x01, 0, 5, 5),
    )
    .unwrap();
    write_empty_ool_mirrors(&dir);
    dir
}

const WALK: [KeyCode; 10] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowLeft,
    KeyCode::Space,
    KeyCode::ArrowDown,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowUp,
    KeyCode::ArrowRight,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
];

/// The talk route's keyword typing (J, O, B, a correction, Enter), twice.
const TALK: [KeyCode; 12] = [
    KeyCode::KeyJ,
    KeyCode::KeyO,
    KeyCode::KeyB,
    KeyCode::Backspace,
    KeyCode::KeyB,
    KeyCode::Enter,
    KeyCode::KeyN,
    KeyCode::KeyA,
    KeyCode::KeyM,
    KeyCode::KeyE,
    KeyCode::Enter,
    KeyCode::Enter,
];

const DUNGEON: [KeyCode; 10] = [
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyD,
    KeyCode::KeyS,
    KeyCode::ArrowUp,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowLeft,
    KeyCode::Space,
    KeyCode::ArrowUp,
];

fn at_clock(mut state: PlayState, hour: u8, minute: u8) -> PlayState {
    state.clock = GameClock::new(hour, minute).unwrap();
    state
}

/// The synthetic scripts of 2.6: a town walk, an overworld walk at noon
/// and at night, a conversation with typed keywords, a dungeon (a fixed
/// screen), the camp flash and the potion sweep.
fn synthetic_scripts(scratch: &Scratch) -> Vec<Script> {
    let seed = synthetic_save_seed(scratch);
    let scene = inked_scene();
    let script = |name, state, keys: &[KeyCode], top_down| Script {
        name,
        state,
        scene: scene.clone(),
        save_seed: seed.clone(),
        keys: keys.to_vec(),
        top_down,
    };
    let town = || test_state(open_grid(), 5, 5);
    let world = || world_state(open_world_grid(), 40, 40);
    let mut talking = town();
    crate::seed_visual_key_route_conversation(&mut talking);
    let mut flash = town();
    flash.pending_potion_flash = potion_flash_playback(0);
    let mut sweep = town();
    sweep.start_visibility_sweep();
    vec![
        script("town walk", town(), &WALK, true),
        script(
            "overworld walk at noon",
            at_clock(world(), 12, 0),
            &WALK,
            true,
        ),
        script(
            "overworld walk at night",
            at_clock(world(), 22, 30),
            &WALK,
            true,
        ),
        script("conversation", talking, &TALK, true),
        script(
            "dungeon",
            u5_runtime::test_fixtures::dungeon_state(open_dungeon_record(), 0, 1, 1),
            &DUNGEON,
            false,
        ),
        script("camp flash", flash, &WALK, true),
        script("potion sweep", sweep, &WALK, true),
    ]
}

// ---------------------------------------------------------------------------
// Runs.

/// What one run is compared on.
struct Trace {
    run: ShellRun,
    /// Logic ticks so far, after each frame.
    ticks: Vec<u64>,
    /// Each save file's hash after the scripted `Q`, `Y`.
    saves: Vec<(&'static str, Option<u64>)>,
    /// Times the grow-left compose step drew a canvas.
    composes: u64,
    /// Frames on which the docked column was open.
    open_frames: usize,
    /// The presentation steps registered at the end of the run.
    steps: Vec<StepOrder>,
    /// Whether the compositing masks were built from the atlas.
    masks_built: bool,
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &to.join(entry.file_name())).unwrap();
        }
    }
}

/// `Q`, `Y` from `state` into a fresh copy of the seed save, then the hash
/// of each save file there (the terminal layer's scripted save).
fn scripted_save(mut state: PlayState, seed: &Path) -> Vec<(&'static str, Option<u64>)> {
    let scratch = Scratch::new("save");
    copy_dir(seed, scratch.path());
    handle_play_key_input(&mut state, 'Q', "", scratch.path()).unwrap();
    handle_play_key_input(&mut state, 'Y', "", scratch.path()).unwrap();
    SAVE_FILES
        .iter()
        .map(|name| {
            let bytes = fs::read(scratch.path().join(name)).ok();
            (*name, bytes.map(|b| fnv1a64(&b)))
        })
        .collect()
}

fn run(
    script: &Script,
    launch: &VisualLaunchConfig,
    window: (f32, f32),
    timeline: &Timeline,
) -> Trace {
    let mut shell = Shell::new(script.state.clone(), &script.scene, launch, |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(window.0, window.1),
        ..default()
    });
    let mut frames: Vec<FrameRecord> = Vec::with_capacity(timeline.len());
    let mut ticks = Vec::with_capacity(timeline.len());
    let mut count = 0;
    // The game state is inserted on the first frame.
    let mut clock = None;
    let mut open_frames = 0;
    for index in 0..timeline.len() {
        frames.push(shell.step(timeline.frame(index)));
        let now = shell.visual().state.animation.frame;
        if clock.is_some_and(|clock| clock != now) {
            count += 1;
        }
        clock = Some(now);
        ticks.push(count);
        if shell
            .app
            .world()
            .get_resource::<ColumnState>()
            .is_some_and(|column| column.view().shown())
        {
            open_frames += 1;
        }
    }
    let composes = shell
        .app
        .world()
        .get_resource::<PresentHost>()
        .map_or(0, PresentHost::composes);
    let steps = shell
        .app
        .world()
        .get_resource::<PresentSteps>()
        .map_or_else(Vec::new, PresentSteps::orders);
    let masks_built = shell
        .app
        .world()
        .get_resource::<CompositeMasks>()
        .is_some_and(|masks| masks.get().is_some());
    Trace {
        run: ShellRun {
            frames,
            fed: Vec::new(),
        },
        ticks,
        saves: scripted_save(shell.visual().state.clone(), &script.save_seed),
        composes,
        open_frames,
        steps,
        masks_built,
    }
}

/// Every difference between the off run and `on`, as lines.
fn differences(off: &Trace, on: &Trace) -> Vec<String> {
    let mut lines: Vec<String> = compare_shell_runs(&off.run, &on.run)
        .iter()
        .map(ToString::to_string)
        .collect();
    if let Some(frame) =
        (0..off.ticks.len().min(on.ticks.len())).find(|&frame| off.ticks[frame] != on.ticks[frame])
    {
        lines.push(format!(
            "frame {frame} logic ticks: off {} / on {}",
            off.ticks[frame], on.ticks[frame]
        ));
    }
    if off.saves != on.saves {
        lines.push(format!(
            "save files: off {:?} / on {:?}",
            off.saves, on.saves
        ));
    }
    lines
}

/// Runs `configs` on `script` against its off run; returns every mismatch,
/// and checks each run is not vacuous.
fn check(script: &Script, configs: &[Config]) -> Vec<String> {
    let mut mismatches = Vec::new();
    let mut offs: Vec<((bool, (u32, u32)), Trace)> = Vec::new();
    for config in configs {
        let timeline = script.timeline(config.column_keys);
        let window_key = (config.window.0 as u32, config.window.1 as u32);
        // The off run: Classic, the feature's keys removed, on the same
        // window (the 1.0.1 shell ignores it).
        let off_key = (config.column_keys, window_key);
        if !offs.iter().any(|(key, _)| *key == off_key) {
            let off = run(
                script,
                &VisualLaunchConfig::classic(),
                config.window,
                &timeline.without_feature(),
            );
            let first = off.run.frames.first().unwrap().digest;
            assert!(
                off.run.frames.iter().any(|frame| frame.digest != first),
                "{}: the script must change the game",
                script.name
            );
            assert!(
                off.ticks.last().copied().unwrap_or(0) > 0,
                "{}: the world must tick",
                script.name
            );
            assert!(
                off.saves.iter().any(|(_, hash)| hash.is_some()),
                "{}: the scripted save must write",
                script.name
            );
            assert_eq!(off.composes, 0);
            assert!(off.steps.is_empty() && !off.masks_built);
            offs.push((off_key, off));
        }
        let off = &offs.iter().find(|(key, _)| *key == off_key).unwrap().1;
        let on = run(script, &config.launch(), config.window, &timeline);
        for line in differences(off, &on) {
            mismatches.push(format!("{} / {}: {line}", script.name, config.name));
        }
        if config.grow_left() && script.top_down {
            assert!(
                on.composes > 0,
                "{} / {}: the canvas was never drawn",
                script.name,
                config.name
            );
        }
        // Chapter 9's steps really ran: the brightness step is registered,
        // and compositing built its masks wherever a map was drawn.
        assert_eq!(
            on.steps.contains(&StepOrder::BRIGHTNESS),
            config.brightness(),
            "{} / {}: the brightness step",
            script.name,
            config.name
        );
        assert_eq!(
            on.steps.contains(&StepOrder::COMPOSITE),
            config.composite(),
            "{} / {}: the composite step",
            script.name,
            config.name
        );
        if config.composite() && script.top_down {
            assert!(
                on.masks_built,
                "{} / {}: the sprite masks were never built",
                script.name, config.name
            );
        }
        if config.column_keys && config.grow_left() && script.top_down {
            assert!(
                on.open_frames > 0,
                "{} / {}: Ctrl+P never opened the column",
                script.name,
                config.name
            );
        }
    }
    mismatches
}

// ---------------------------------------------------------------------------
// The matrix.

/// Every key alone and the column on one synthetic script.
fn alone_and_column_on(name: &str) {
    let scratch = Scratch::new("alone");
    let script = synthetic_scripts(&scratch)
        .into_iter()
        .find(|script| script.name == name)
        .unwrap();
    let mut configs = alone();
    configs.push(column());
    let mismatches = check(&script, &configs);
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn the_town_walk_is_left_alone() {
    alone_and_column_on("town walk");
}

#[test]
fn the_overworld_walk_at_noon_is_left_alone() {
    alone_and_column_on("overworld walk at noon");
}

#[test]
fn the_overworld_walk_at_night_is_left_alone() {
    alone_and_column_on("overworld walk at night");
}

#[test]
fn the_conversation_is_left_alone() {
    alone_and_column_on("conversation");
}

#[test]
fn the_dungeon_is_left_alone() {
    alone_and_column_on("dungeon");
}

#[test]
fn the_camp_flash_is_left_alone() {
    alone_and_column_on("camp flash");
}

#[test]
fn the_potion_sweep_is_left_alone() {
    alone_and_column_on("potion sweep");
}

#[test]
fn every_pair_of_display_keys_leaves_the_shortest_script_alone() {
    let scratch = Scratch::new("pairs");
    let mut configs = pairs();
    assert_eq!(configs.len(), 17 * 16 / 2);
    configs.extend(light_pairs());
    let script = synthetic_scripts(&scratch)
        .into_iter()
        .find(|script| script.name == "town walk")
        .unwrap();
    let mismatches = check(&script, &configs);
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// The ticks and the scripted save are compared, not only the digest: a
/// feature that steps the world once more, or writes a byte into the save,
/// is caught even where the frame digest would hide it.
#[test]
fn an_extra_tick_and_a_changed_save_are_caught() {
    let scratch = Scratch::new("caught");
    let script = synthetic_scripts(&scratch).remove(0);
    let timeline = script.timeline(false);
    let launch = VisualLaunchConfig::classic();
    let off = run(&script, &launch, W16, &timeline);
    let mut extra = run(&script, &launch, W16, &timeline);
    assert!(differences(&off, &extra).is_empty());
    extra.ticks[50] += 1;
    extra.saves[0].1 = Some(0);
    let lines = differences(&off, &extra);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("frame 50 logic ticks"));
    assert!(lines[1].starts_with("save files"));
}

// ---------------------------------------------------------------------------
// Real game files.

/// The 2.6 scripts on the real files: the overworld at noon and at night,
/// Britain at night with its lamps, a conversation, a combat, a dungeon,
/// the camp flash and the potion sweep, each with every key alone and the
/// column; the pairs on the overworld walk. Runs only when
/// `U5_TEST_ASSET_DIR` names a copy of the game.
#[test]
fn the_matrix_leaves_the_real_game_alone() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    if !dir.join("BRIT.DAT").exists() {
        return;
    }
    let scratch = Scratch::new("real");
    let game = scratch.path().join("game");
    copy_dir(&dir, &game);
    let scene = Scene::from_game_dir(&game).unwrap();
    let load = |options: PlayOptions| {
        let mut state = PlayState::load_scene(&game, options).unwrap();
        state.prng_state = 0x1234;
        state
    };
    let world = |hour, minute| PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((86, 105)),
        clock: GameClock::new(hour, minute).unwrap(),
        ..PlayOptions::default()
    };
    let britain = PlayOptions {
        target: PlayTarget::from_key("TOWNE:1").unwrap(),
        start: Some((15, 19)),
        clock: GameClock::new(21, 50).unwrap(),
        ..PlayOptions::default()
    };
    let mut talking = load(PlayOptions::default());
    crate::seed_visual_key_route_conversation(&mut talking);
    let mut combat = load(world(12, 0));
    crate::seed_visual_suite_combat(&mut combat, &game).unwrap();
    let dungeon = load(PlayOptions {
        target: PlayTarget::Dungeon(DungeonScene::new(0x21).unwrap()),
        ..PlayOptions::default()
    });
    let mut flash = load(world(12, 0));
    flash.pending_potion_flash = potion_flash_playback(0);
    let mut sweep = load(world(12, 0));
    sweep.start_visibility_sweep();
    let script = |name, state, keys: &[KeyCode], top_down| Script {
        name,
        state,
        scene: scene.clone(),
        save_seed: game.clone(),
        keys: keys.to_vec(),
        top_down,
    };
    const COMBAT: [KeyCode; 10] = [
        KeyCode::Space,
        KeyCode::ArrowUp,
        KeyCode::Space,
        KeyCode::ArrowLeft,
        KeyCode::Space,
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::Space,
        KeyCode::ArrowRight,
        KeyCode::Space,
    ];
    let scripts = [
        script("overworld at noon", load(world(12, 0)), &WALK, true),
        script("overworld at night", load(world(22, 30)), &WALK, true),
        script("Britain at night", load(britain), &WALK, true),
        script("conversation", talking, &TALK, true),
        script("combat", combat, &COMBAT, false),
        script("dungeon", dungeon, &DUNGEON, false),
        script("camp flash", flash, &WALK, true),
        script("potion sweep", sweep, &WALK, true),
    ];
    let mut configs = alone();
    configs.push(column());
    // One thread per script; the pairs on the first.
    let mismatches: Vec<String> = std::thread::scope(|scope| {
        let configs = &configs;
        let runs: Vec<_> = scripts
            .iter()
            .map(|script| scope.spawn(move || check(script, configs)))
            .chain(std::iter::once(scope.spawn(|| {
                let mut pairs = pairs();
                pairs.extend(light_pairs());
                check(&scripts[0], &pairs)
            })))
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().unwrap())
            .collect()
    });
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
