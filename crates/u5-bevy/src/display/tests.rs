//! The display seam (design 2.3.4, 2.6; build plan M1-01): the plugin is
//! added only for a display toggle, and the default-mode baseline (the
//! faithful image, the sprite's size, the window and the registered
//! systems) is the 1.0.1 one with the plugin absent and with it present and
//! every key off.

use bevy::window::{MonitorSelection, PresentMode, WindowMode, WindowResolution};
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, DISPLAY_TOGGLES, HostCapabilities, LockReason,
    LockState, ResolveInput, ResolvedEnhancements, Toggle, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, test_state};

use super::*;
use crate::enh::EnhancementsPlugin;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};

fn launch_with(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

/// A launch resolved from `cli` as if every `[display]` phase had shipped.
fn resolved_launch(cli: &[CliSetting]) -> VisualLaunchConfig {
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

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn shell(launch: &VisualLaunchConfig, install: impl FnOnce(&mut App)) -> Shell {
    Shell::new(
        test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        launch,
        install,
    )
}

/// Forces the display plugin into the app with every key at its default,
/// standing in for the menu overlay until chapter 3 (M4) brings it.
fn force_display_plugin(app: &mut App) {
    app.add_plugins(DisplayPlugin::new(DisplaySettings::default()));
}

/// Every system and set of every schedule, sorted.
fn schedules_outline(app: &App) -> Vec<String> {
    let mut outline: Vec<String> = app
        .world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .flat_map(|(label, schedule)| {
            let graph = schedule.graph();
            let systems = graph
                .systems()
                .map(move |(_, system, _)| format!("{label:?} system {}", system.name()));
            let sets = graph
                .system_sets()
                .map(move |(_, set, _)| format!("{label:?} set {set:?}"));
            systems.chain(sets).collect::<Vec<_>>()
        })
        .collect();
    outline.sort();
    outline
}

fn display_systems(app: &App) -> Vec<String> {
    schedules_outline(app)
        .into_iter()
        .filter(|line| line.contains("u5_bevy::display"))
        .collect()
}

/// The frame sprite's size and the faithful image's bytes this frame.
fn layout(shell: &Shell) -> (Vec<Option<Vec2>>, Vec<u8>) {
    let world = shell.app.world();
    let mut sizes: Vec<Option<Vec2>> = world
        .try_query::<&Sprite>()
        .map(|mut query| query.iter(world).map(|sprite| sprite.custom_size).collect())
        .unwrap_or_default();
    sizes.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    let bytes = world
        .resource::<Assets<Image>>()
        .get(&shell.visual().image_handle)
        .and_then(|image| image.data.clone())
        .unwrap_or_default();
    (sizes, bytes)
}

/// The player walks about.
fn walk(frames: usize) -> Timeline {
    let mut timeline = Timeline::new(frames);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::ArrowDown)
        .game_tap(50, KeyCode::ArrowRight);
    timeline
}

// --- When the plugin is added. ------------------------------------------

#[test]
fn classic_and_the_lock_add_no_display_plugin() {
    let mut locked = launch_with(DISPLAY_TOGGLES);
    locked.enhancements.lock = LockState::Locked(LockReason::QaEnv);
    for launch in [VisualLaunchConfig::classic(), locked] {
        let shell = shell(&launch, |_| {});
        assert!(!shell.app.is_plugin_added::<EnhancementsPlugin>());
        assert!(!shell.app.is_plugin_added::<DisplayPlugin>());
        assert!(!shell.app.world().contains_resource::<Display>());
        assert_eq!(display_systems(&shell.app), Vec::<String>::new());
    }
}

#[test]
fn another_feature_alone_adds_no_display_plugin() {
    let launch = launch_with(&[Toggle::EnhancementsFreezeOverlays]);
    assert!(!display_wanted(launch.enhancements.toggles));
    let shell = shell(&launch, |_| {});
    assert!(shell.app.is_plugin_added::<EnhancementsPlugin>());
    assert!(!shell.app.is_plugin_added::<DisplayPlugin>());
    assert!(!shell.app.world().contains_resource::<Display>());
}

#[test]
fn each_display_toggle_adds_the_plugin_with_its_settings() {
    for toggle in DISPLAY_TOGGLES {
        let launch = launch_with(&[*toggle]);
        assert!(display_wanted(launch.enhancements.toggles), "{toggle:?}");
        let mut app = App::new();
        crate::enh::add_enhancements(&mut app, &launch);
        assert!(app.is_plugin_added::<DisplayPlugin>(), "{toggle:?}");
        assert_eq!(
            app.world().resource::<Display>().0,
            launch.enhancements.display(),
            "{toggle:?}"
        );
        // The seam registers no system yet.
        assert_eq!(display_systems(&app), Vec::<String>::new(), "{toggle:?}");
    }
    // Resolved as a host would: the wider view and its settings.
    let launch = resolved_launch(&[
        CliSetting::Enable("display.grow_left".into()),
        set("display.crawl", "false"),
    ]);
    let mut app = App::new();
    crate::enh::add_enhancements(&mut app, &launch);
    let settings = &app.world().resource::<Display>().0;
    assert!(settings.grow_left.on);
    assert!(!settings.grow_left.crawl);
}

#[test]
fn a_sub_key_of_an_off_feature_adds_no_plugin() {
    for cli in [
        vec![set("display.crawl", "false")],
        vec![set("display.centre_party", "true")],
        vec![set("display.grow_left_light", "faithful")],
        vec![
            set("display.dither", "false"),
            set("display.typed_colour", "false"),
        ],
        vec![set("display.pillar_color", "#AA5500")],
        vec![set("display.raster", "tandy")],
    ] {
        let launch = resolved_launch(&cli);
        assert!(launch.enhancements.notes.is_empty(), "{cli:?}");
        assert!(!launch.wants_enhancements(), "{cli:?}");
        let shell = shell(&launch, |_| {});
        assert!(
            !shell.app.is_plugin_added::<EnhancementsPlugin>(),
            "{cli:?}"
        );
        assert!(!shell.app.is_plugin_added::<DisplayPlugin>(), "{cli:?}");
    }
}

// --- The default-mode baseline (2.6). -----------------------------------

#[test]
fn the_plugin_present_with_every_key_off_is_the_1_0_1_shell() {
    let classic = VisualLaunchConfig::classic();
    let mut off = shell(&classic, |_| {});
    let mut forced = shell(&classic, force_display_plugin);
    assert!(forced.app.is_plugin_added::<DisplayPlugin>());
    assert_eq!(
        forced.app.world().resource::<Display>().0,
        DisplaySettings::default()
    );
    // No display system is registered, so none can run on any frame, and
    // the schedules are the 1.0.1 schedules.
    assert_eq!(display_systems(&forced.app), Vec::<String>::new());
    assert_eq!(schedules_outline(&forced.app), schedules_outline(&off.app));

    // Frame by frame: the same sprite size and faithful image bytes.
    let timeline = walk(70);
    let mut moved = false;
    let mut first = None;
    for frame in 0..timeline.len() {
        off.step(timeline.frame(frame));
        forced.step(timeline.frame(frame));
        let (off_sizes, off_bytes) = layout(&off);
        let (forced_sizes, forced_bytes) = layout(&forced);
        assert_eq!(forced_sizes, off_sizes, "sprite sizes at frame {frame}");
        assert!(!off_sizes.is_empty(), "the frame sprite exists");
        assert_eq!(forced_bytes, off_bytes, "faithful image at frame {frame}");
        let first = first.get_or_insert_with(|| off_bytes.clone());
        moved |= off_bytes != *first;
    }
    assert!(moved, "the walk must change the frame");

    // And the game itself: digest, frame hash and key queue.
    let start = test_state(open_grid(), 5, 5);
    let mismatches = check_shell_equivalence(
        &start,
        &Scene::synthetic(),
        &walk(70),
        &classic,
        force_display_plugin,
    );
    assert_eq!(mismatches, []);
}

#[test]
fn every_display_toggle_on_leaves_the_game_alone() {
    // The plugin is empty in this package, so even with every display
    // toggle on the game, its frame and its key queue equal the off run.
    let start = test_state(open_grid(), 5, 5);
    let mismatches = check_shell_equivalence(
        &start,
        &Scene::synthetic(),
        &walk(70),
        &launch_with(DISPLAY_TOGGLES),
        |_| {},
    );
    assert_eq!(mismatches, []);
}

// --- The window. ---------------------------------------------------------

/// The 1.0.1 gameplay window, as `run_visual_loop_with` builds it.
fn window_1_0_1() -> Window {
    Window {
        title: "Ultima V".into(),
        resolution: WindowResolution::new(960.0, 720.0),
        resizable: true,
        ..default()
    }
}

fn window_fields(window: &Window) -> (String, WindowMode, PresentMode, f32, f32, bool) {
    (
        window.title.clone(),
        window.mode,
        window.present_mode,
        window.resolution.width(),
        window.resolution.height(),
        window.resizable,
    )
}

#[test]
fn the_window_is_the_1_0_1_window_under_classic() {
    let base = window_fields(&window_1_0_1());
    assert_eq!(base.1, WindowMode::Windowed);
    assert_eq!(base.2, PresentMode::Fifo, "Bevy's default, the 1.0.1 mode");
    let mut locked = resolved_launch(&[
        CliSetting::Enable("display.fullscreen".into()),
        set("display.vsync", "false"),
    ]);
    locked.enhancements.lock = LockState::Locked(LockReason::CliFlag);
    for launch in [
        VisualLaunchConfig::classic(),
        VisualLaunchConfig {
            enhancements: ResolvedEnhancements::locked(LockReason::EnvVar),
            ..VisualLaunchConfig::classic()
        },
        locked,
        // Display toggles on whose keys leave the window alone.
        launch_with(&[Toggle::DisplayGrowLeft, Toggle::DisplayScaleMode]),
        resolved_launch(&[set("display.crawl", "false")]),
    ] {
        assert_eq!(
            window_fields(&initial_window(window_1_0_1(), &launch)),
            base,
            "{launch:?}"
        );
    }
}

#[test]
fn fullscreen_and_vsync_set_the_first_window() {
    let window = initial_window(
        window_1_0_1(),
        &resolved_launch(&[CliSetting::Enable("display.fullscreen".into())]),
    );
    assert_eq!(
        window.mode,
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    );
    assert_eq!(window.present_mode, PresentMode::Fifo);
    assert_eq!(
        window.resolution.width(),
        960.0,
        "the windowed size is kept"
    );

    let window = initial_window(
        window_1_0_1(),
        &resolved_launch(&[CliSetting::Disable("display.vsync".into())]),
    );
    assert_eq!(window.mode, WindowMode::Windowed);
    assert_eq!(window.present_mode, PresentMode::AutoNoVsync);
}
