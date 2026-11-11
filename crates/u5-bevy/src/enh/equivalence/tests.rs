//! Layer B of the equivalence harness on synthetic states, and on real
//! game files when `U5_TEST_ASSET_DIR` names a copy (design 1.3.12, plan
//! M0-19): Classic and every matrix row equal the off run, a Live overlay
//! equals a player who stopped typing, fed keys replay as typed keys, and
//! features that write the game, its frame or its key queue are caught.

use std::sync::{Arc, Mutex};

use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockState, MATRIX_FILE, ResolveInput, ResolvedEnhancements,
    TOGGLES, Toggle, matrix_tsv, resolve, toggle_matrix,
};
use u5_runtime::test_fixtures::{configured_original_asset_dir, open_grid, test_state};
use u5_runtime::{GameClock, PlayOptions, PlayTarget, WorldPlane};

use super::*;
use crate::enh::input::is_modifier;
use crate::enh::input::{EnhAction, EnhActionEvent, KeyMap, OverlayKey, install_input_router};
use crate::enh::overlay::{
    OverlayId, OverlayRequest, OverlayState, apply_overlay_requests, install_overlay_model,
};
use crate::enh::synthetic::{SyntheticKey, SyntheticOwner, install_synthetic_input};
use crate::enh::{EnhancementsPlugin, configure_enh_sets};

/// A synthetic town at its idle command prompt.
fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

/// Stands in for a feature's toggle: with it on the foundation plugin is
/// in the app. The shell does not care that it ships later.
const T: Toggle = Toggle::EnhancementsFreezeOverlays;

fn launch_with(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

/// The player walks about: north, west, a pass, south, east.
fn walk(frames: usize) -> Timeline {
    let mut timeline = Timeline::new(frames);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::Space)
        .game_tap(50, KeyCode::ArrowDown)
        .game_tap(65, KeyCode::ArrowRight);
    timeline
}

fn run(launch: &VisualLaunchConfig, timeline: &Timeline) -> ShellRun {
    run_shell(town(), &Scene::synthetic(), launch, |_| {}, timeline, None)
}

/// The run is not vacuous: the game state really moves under the timeline.
fn assert_the_game_moved(run: &ShellRun) {
    let first = run.frames.first().unwrap().digest;
    assert!(
        run.frames.iter().any(|frame| frame.digest != first),
        "the timeline must change the game"
    );
}

// ---------------------------------------------------------------------------
// The timeline.

#[test]
fn the_off_timeline_keeps_every_frame_and_drops_the_features_events() {
    let mut timeline = Timeline::new(4);
    timeline
        .game_tap(0, KeyCode::ArrowUp)
        .feature_tap(1, KeyCode::Tab)
        .feature_hold(3, 10, KeyCode::KeyA);
    assert_eq!(timeline.len(), 11);
    let off = timeline.without_feature();
    assert_eq!(off.len(), timeline.len(), "Live intervals stay as frames");
    let events: Vec<(usize, KeyCode)> = (0..off.len())
        .flat_map(|frame| off.frame(frame).iter().map(move |e| (frame, e.key)))
        .collect();
    assert_eq!(events, [(0, KeyCode::ArrowUp), (1, KeyCode::ArrowUp)]);
    // The hold repeats every third frame between press and release.
    let repeats = (0..timeline.len())
        .filter(|&frame| timeline.frame(frame).iter().any(|e| e.repeat))
        .collect::<Vec<_>>();
    assert_eq!(repeats, [6, 9]);
}

// ---------------------------------------------------------------------------
// Equal runs.

#[test]
fn classic_equals_the_off_run_frame_for_frame() {
    let timeline = walk(90);
    let on = run(&VisualLaunchConfig::classic(), &timeline);
    assert_the_game_moved(&on);
    assert!(on.fed.is_empty(), "no feed without a synthetic feature");
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &VisualLaunchConfig::classic(),
        |_| {},
    );
    assert_eq!(mismatches, []);
}

/// The foundation plugin with every catalogue toggle on, whether shipped or
/// not, leaves the game as the 1.0.1 app has it.
#[test]
fn every_toggle_on_equals_the_off_run() {
    let all: Vec<Toggle> = TOGGLES.iter().map(|spec| spec.toggle).collect();
    let launch = launch_with(&all);
    let shell = Shell::new(town(), &Scene::synthetic(), &launch, |_| {});
    assert!(shell.app.is_plugin_added::<EnhancementsPlugin>());
    let mismatches =
        check_shell_equivalence(&town(), &Scene::synthetic(), &walk(90), &launch, |_| {});
    assert_eq!(mismatches, []);
}

/// Every row of the toggle matrix, resolved as a host resolves it, equals
/// the off run.
#[test]
fn every_matrix_row_equals_the_off_run() {
    let timeline = walk(80);
    let off = run(&VisualLaunchConfig::classic(), &timeline);
    assert_the_game_moved(&off);
    for row in toggle_matrix() {
        let resolved = resolve(ResolveInput {
            stored: None,
            cli: &row.settings,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        });
        let launch = VisualLaunchConfig {
            enhancements: resolved,
            ..VisualLaunchConfig::classic()
        };
        let on = run(&launch, &timeline);
        assert!(on.fed.iter().all(Vec::is_empty), "{}", row.name);
        assert_eq!(compare_shell_runs(&off, &on), [], "{}", row.name);
    }
}

/// The committed `qa/enhancements/matrix.tsv` (build plan M2-08, design
/// 1.3.12 and 9.4.3): it is the generator's current output, so it holds
/// chapter 9's rows, and every row it lists, read from the file as a host
/// reads `--set KEY=VALUE`, equals the off run through
/// [`check_shell_equivalence`] in a 16:9 window, where the presentation
/// steps (the wider view, compositing, brightness) really draw.
#[test]
fn every_row_of_the_committed_matrix_equals_the_off_run_in_a_window() {
    let path = u5_runtime::test_fixtures::workspace_file(MATRIX_FILE);
    let text = std::fs::read_to_string(&path).expect("the committed matrix");
    assert_eq!(
        text,
        matrix_tsv(&toggle_matrix()),
        "{MATRIX_FILE} is out of date with the catalogue"
    );
    for key in [
        "display.brightness=",
        "display.brightness_scope=",
        "display.sprite_composite=",
        "display.composite_halo=",
    ] {
        assert!(text.contains(key), "{MATRIX_FILE} has no {key} row");
    }
    let timeline = walk(80);
    let window = |app: &mut App| {
        app.world_mut().spawn(Window {
            resolution: bevy::window::WindowResolution::new(2560.0, 1440.0),
            ..default()
        });
    };
    let (mut rows, mut lit, mut composited) = (0, 0, 0);
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let (name, settings) = line.split_once('\t').expect("NAME<TAB>SETTINGS");
        let settings: Vec<CliSetting> = settings
            .split(',')
            .filter(|setting| !setting.is_empty())
            .map(|setting| {
                let (key, value) = setting.split_once('=').expect("KEY=VALUE");
                CliSetting::Set {
                    key: key.into(),
                    value: value.into(),
                }
            })
            .collect();
        let resolved = resolve(ResolveInput {
            stored: None,
            cli: &settings,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        });
        let light = resolved.display().light;
        lit += usize::from(light.brightness != 100);
        composited += usize::from(light.composite);
        let launch = VisualLaunchConfig {
            enhancements: resolved,
            ..VisualLaunchConfig::classic()
        };
        let mismatches =
            check_shell_equivalence(&town(), &Scene::synthetic(), &timeline, &launch, window);
        assert_eq!(mismatches, [], "{name}");
        rows += 1;
    }
    assert_eq!(rows, toggle_matrix().len());
    assert!(
        lit > 0 && composited > 0,
        "brightness {lit}, compositing {composited}"
    );
}

// ---------------------------------------------------------------------------
// A Live overlay.

/// The test overlay asks for the map on its hotkey and closes on Escape,
/// through the model, as a real overlay feature does.
fn map_overlay(
    mut actions: EventReader<EnhActionEvent>,
    mut keys: EventReader<OverlayKey>,
    state: Res<OverlayState>,
    mut requests: EventWriter<OverlayRequest>,
) {
    for action in actions.read() {
        if action.0 == EnhAction::OpenMap {
            requests.write(OverlayRequest::Open(OverlayId::Map));
        }
    }
    for key in keys.read() {
        if key.0 == KeyCode::Escape
            && let Some(id) = state.open()
        {
            requests.write(OverlayRequest::Close(id));
        }
    }
}

/// Whether the overlay was open at the end of each frame.
#[derive(Resource, Clone, Default)]
struct OpenFrames(Arc<Mutex<Vec<bool>>>);

fn note_open(state: Res<OverlayState>, frames: Res<OpenFrames>) {
    frames.0.lock().unwrap().push(state.is_open());
}

/// The router with every default key bound, the overlay model and the
/// test overlay.
fn install_map_overlay(app: &mut App, open: OpenFrames) {
    configure_enh_sets(app);
    if !app.world().contains_resource::<KeyMap>() {
        let (keymap, notes) = KeyMap::build(ResolvedEnhancements::classic().keys.iter(), |_| true);
        assert!(notes.is_empty());
        install_input_router(app, keymap);
    }
    install_overlay_model(app);
    app.insert_resource(open).add_systems(
        Update,
        (
            map_overlay
                .in_set(EnhSet::Apply)
                .before(apply_overlay_requests),
            note_open.after(EnhSet::Observe),
        ),
    );
}

/// Design 1.3.12: a Live overlay interval is empty frames in the off run.
/// The player walks, opens the map, types into it (a tap, a held arrow
/// with OS repeat), closes it and walks on; the game sees only the walk.
#[test]
fn a_live_overlay_run_equals_the_off_run() {
    let mut timeline = walk(200);
    timeline
        .feature_tap(80, KeyCode::Tab)
        .feature_tap(95, KeyCode::KeyN)
        .feature_hold(100, 130, KeyCode::ArrowDown)
        .feature_tap(140, KeyCode::Space)
        .feature_tap(150, KeyCode::Escape)
        .game_tap(165, KeyCode::ArrowUp)
        .game_tap(180, KeyCode::ArrowLeft);
    let open = OpenFrames::default();
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &launch_with(&[T]),
        |app| install_map_overlay(app, open.clone()),
    );
    assert_eq!(mismatches, []);
    let open = open.0.lock().unwrap();
    assert!(!open[79], "closed before the hotkey");
    assert!(open[80..150].iter().all(|o| *o), "open from the hotkey");
    assert!(!open[150], "closed by Escape");
    assert!(open[151..].iter().all(|o| !*o));
}

/// The same overlay without its events removed from the off run: the keys
/// typed into it would have moved the Avatar, so the harness must see a
/// difference. Proves the comparison is not vacuous for overlays.
#[test]
fn overlay_keys_left_in_the_off_run_differ() {
    let mut timeline = walk(140);
    timeline
        .feature_tap(80, KeyCode::Tab)
        .feature_tap(95, KeyCode::ArrowDown)
        .feature_tap(110, KeyCode::Escape);
    let on = run_shell(
        town(),
        &Scene::synthetic(),
        &launch_with(&[T]),
        |app| install_map_overlay(app, OpenFrames::default()),
        &timeline,
        None,
    );
    // The off run sees every key as the player's.
    let mut all_game = Timeline::new(timeline.len());
    for frame in 0..timeline.len() {
        for event in timeline.frame(frame) {
            all_game.event(frame, event.key, event.state, event.repeat, Owner::Game);
        }
    }
    let off = run(&VisualLaunchConfig::classic(), &all_game);
    assert!(
        compare_shell_runs(&off, &on)
            .iter()
            .any(|m| m.what == "digest"),
    );
}

// ---------------------------------------------------------------------------
// Class (b): fed keys.

/// A class (b) dummy: at frame 30 it takes the synthetic token and types a
/// step north and a pass, as automatic combat types its commands.
fn auto_typer(mut owner: ResMut<SyntheticInputOwner>, mut frames: Local<usize>) {
    *frames += 1;
    if *frames == 31 {
        owner.take(SyntheticOwner::AutoCombat).unwrap();
        owner
            .queue(
                SyntheticOwner::AutoCombat,
                [
                    SyntheticKey::plain(KeyCode::ArrowUp),
                    SyntheticKey::plain(KeyCode::Space),
                ],
            )
            .unwrap();
    }
    if *frames == 60 {
        owner.release(SyntheticOwner::AutoCombat);
    }
}

fn install_auto_typer(app: &mut App) {
    install_synthetic_input(app);
    app.add_systems(Update, auto_typer.in_set(EnhSet::Apply));
}

#[test]
fn fed_keys_replay_as_typed_keys() {
    let timeline = walk(120);
    let on = run_shell(
        town(),
        &Scene::synthetic(),
        &launch_with(&[T]),
        install_auto_typer,
        &timeline,
        None,
    );
    let fed: Vec<(usize, usize)> = on
        .fed
        .iter()
        .enumerate()
        .filter(|(_, keys)| !keys.is_empty())
        .map(|(frame, keys)| (frame, keys.len()))
        .collect();
    assert_eq!(
        fed,
        [(30, 2)],
        "fed once, both keys, in the frame they were queued"
    );
    // Without the replay the off run does not see the feature's keys.
    let plain = run(&VisualLaunchConfig::classic(), &timeline);
    assert!(!compare_shell_runs(&plain, &on).is_empty());
    // With it, the runs are the same.
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &launch_with(&[T]),
        install_auto_typer,
    );
    assert_eq!(mismatches, []);
}

// ---------------------------------------------------------------------------
// Offenders.

/// The deliberate offender (design 1.3.12): a feature that claims to only
/// show things but writes the game state once, at frame 40.
fn sneaky_state_writer(mut visual: ResMut<VisualState>, mut frames: Local<usize>) {
    *frames += 1;
    if *frames == 41 {
        visual.state.gold = visual.state.gold.wrapping_add(1);
    }
}

#[test]
fn a_state_writing_feature_is_caught() {
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &walk(90),
        &launch_with(&[T]),
        |app| {
            app.add_systems(Update, sneaky_state_writer.in_set(EnhSet::Observe));
        },
    );
    let digest = mismatches.iter().find(|m| m.what == "digest");
    assert_eq!(digest.map(|m| m.frame), Some(40), "{mismatches:?}");
}

/// A feature that writes the original 320x200 frame (design 1.4.3: "No
/// change to ... the original frame"), at frame 20.
fn frame_scribbler(
    visual: Res<VisualState>,
    mut images: ResMut<Assets<Image>>,
    mut frames: Local<usize>,
) {
    *frames += 1;
    if *frames == 21
        && let Some(data) = images
            .get_mut(&visual.image_handle)
            .and_then(|image| image.data.as_mut())
    {
        data[0] ^= 0xff;
    }
}

#[test]
fn a_feature_that_draws_on_the_original_frame_is_caught() {
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &walk(30),
        &launch_with(&[T]),
        |app| {
            app.add_systems(Update, frame_scribbler.in_set(EnhSet::Present));
        },
    );
    let what: Vec<(&str, usize)> = mismatches.iter().map(|m| (m.what, m.frame)).collect();
    assert_eq!(what, [("original frame", 20)], "{mismatches:?}");
}

/// A feature that pushes a key into the game's queue itself, past the
/// synthetic-key owner, so nothing records or replays it.
fn queue_jumper(mut visual: ResMut<VisualState>, mut frames: Local<usize>) {
    *frames += 1;
    if *frames == 25 {
        visual.typed_keys.push_back(TypedKey {
            key: KeyCode::ArrowDown,
            shift: false,
            control: false,
        });
    }
}

#[test]
fn a_key_typed_past_the_owner_is_caught() {
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &walk(60),
        &launch_with(&[T]),
        |app| {
            app.add_systems(Update, queue_jumper.in_set(EnhSet::Feed));
        },
    );
    assert!(
        mismatches
            .iter()
            .any(|m| m.frame == 24 && matches!(m.what, "digest" | "key queue")),
        "{mismatches:?}"
    );
}

// ---------------------------------------------------------------------------
// Real game files.

/// The overlay and the walk on the Britannia meadow, with the real atlas
/// and fonts. Runs only when `U5_TEST_ASSET_DIR` names a copy of the game.
#[test]
fn a_live_overlay_equals_the_off_run_on_real_files() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    if !dir.join("BRIT.DAT").exists() {
        return;
    }
    let scene = Scene::from_game_dir(&dir).unwrap();
    let mut state = PlayState::load_scene(
        &dir,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            start: Some((86, 105)),
            clock: GameClock::new(13, 0).unwrap(),
            ..PlayOptions::default()
        },
    )
    .unwrap();
    state.prng_state = 0x1234;
    let mut timeline = walk(200);
    timeline
        .feature_tap(80, KeyCode::Tab)
        .feature_hold(100, 130, KeyCode::ArrowDown)
        .feature_tap(150, KeyCode::Escape)
        .game_tap(165, KeyCode::ArrowUp);
    let open = OpenFrames::default();
    let mismatches =
        check_shell_equivalence(&state, &scene, &timeline, &launch_with(&[T]), |app| {
            install_map_overlay(app, open.clone())
        });
    assert_eq!(mismatches, []);
    assert!(open.0.lock().unwrap()[80..150].iter().all(|o| *o));
    let caught = check_shell_equivalence(&state, &scene, &walk(90), &launch_with(&[T]), |app| {
        app.add_systems(Update, sneaky_state_writer.in_set(EnhSet::Observe));
    });
    assert!(caught.iter().any(|m| m.what == "digest" && m.frame == 40));
}

/// The queue compared leaves out lone modifiers, because the game never
/// reads one: in both input modes a modifier maps to no byte, whatever
/// Shift and Ctrl say, so every read drops it and goes on.
#[test]
fn a_lone_modifier_is_no_key_the_game_reads() {
    use crate::{key_code_to_command_char, key_code_to_input_byte, key_code_to_line_input_byte};
    for key in [
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ] {
        assert!(is_modifier(key));
        for shift in [false, true] {
            for control in [false, true] {
                assert_eq!(
                    key_code_to_command_char(key, shift, control),
                    None,
                    "{key:?}"
                );
                assert_eq!(key_code_to_input_byte(key, shift, control), None, "{key:?}");
                assert_eq!(
                    key_code_to_line_input_byte(key, shift, control),
                    None,
                    "{key:?}"
                );
            }
        }
    }
    // A queued modifier changes the compared queue no more than an empty
    // one; a real key does.
    let mut shell = Shell::new(
        town(),
        &Scene::synthetic(),
        &VisualLaunchConfig::classic(),
        |_| {},
    );
    shell.step(&[]);
    let empty = shell.record().queue;
    let modifier = TypedKey {
        key: KeyCode::ControlLeft,
        shift: false,
        control: true,
    };
    shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .typed_keys
        .push_back(modifier);
    assert_eq!(shell.record().queue, empty);
    shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .typed_keys
        .push_back(TypedKey {
            key: KeyCode::KeyP,
            ..modifier
        });
    assert_ne!(shell.record().queue, empty);
}
