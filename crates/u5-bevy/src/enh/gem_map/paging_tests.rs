//! PgUp and PgDn on the gem map (plan M3-14, design 10.6 "Paging
//! equivalence", foundation 1.3.6 item 5): they page a one-panel dungeon
//! map and the game sees nothing, which equals the off run with those
//! presses removed; at every other time they reach the game as in 1.0.1.

use bevy::window::WindowResolution;
use u5_runtime::PlayState;
use u5_runtime::dungeon_cell_index;
use u5_runtime::enhancements::{
    CliSetting, FeatureToggles, HostCapabilities, LockState, ResolveInput, resolve,
};
use u5_runtime::test_fixtures::{dungeon_state, open_dungeon_record, open_world_grid, world_state};

use super::super::view::GemShow;
use crate::enh::equivalence::{Scene, Shell, Timeline};
use crate::enh::input::{EnhAction, KeyChord, KeyMap};
use crate::enh::{Toggles, VisualLaunchConfig};
use crate::key_code_to_input_byte;
use bevy::prelude::*;

const W43: (f32, f32) = (1600.0, 1200.0);
const W169: (f32, f32) = (2560.0, 1440.0);

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

fn gem_without(row: &str) -> VisualLaunchConfig {
    launch(&[
        CliSetting::Enable("gem_map.enhanced".into()),
        CliSetting::Disable(row.into()),
    ])
}

fn shell(state: &PlayState, launch: &VisualLaunchConfig, window: (f32, f32)) -> Shell {
    let mut shell = Shell::new(state.clone(), &Scene::synthetic(), launch, |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(window.0, window.1),
        ..default()
    });
    shell
}

/// Level 2 of an open dungeon, the party at (1, 1) with a ladder up at
/// (2, 1) and one down at (1, 2) beside it: the levels above and below have
/// (black) panels. Two gems.
fn ladders() -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), 2, 1, 1);
    state.grid[dungeon_cell_index(2, 2, 1)] = 0x10;
    state.grid[dungeon_cell_index(2, 1, 2)] = 0x20;
    state.gems = 2;
    state
}

fn outdoors() -> PlayState {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.gems = 2;
    state
}

/// What the game is compared on per frame (as `view_tests`' `Beat`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Beat {
    record: crate::enh::equivalence::FrameRecord,
    gems: u8,
    overlay: String,
    idle_polls: u32,
    prng: u16,
}

/// The map's side of a frame: the page shown and whether the pair is
/// bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Paging {
    page: Option<i8>,
    bound: bool,
    renders: u64,
}

fn run(
    state: &PlayState,
    launch: &VisualLaunchConfig,
    window: (f32, f32),
    timeline: &Timeline,
) -> (Vec<Beat>, Vec<Paging>) {
    let mut shell = shell(state, launch, window);
    let mut beats = Vec::new();
    let mut paging = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let visual = shell.visual();
        assert!(
            visual
                .typed_keys
                .iter()
                .all(|typed| !matches!(typed.key, KeyCode::PageUp | KeyCode::PageDown))
                || !launch_pages(&shell),
            "frame {frame}: a paging key reached the game's queue"
        );
        let state = &visual.state;
        beats.push(Beat {
            record: shell.record(),
            gems: state.gems,
            overlay: format!("{:?}", state.active_view_overlay),
            idle_polls: state.view_overlay_idle_polls,
            prng: state.prng_state,
        });
        let world = shell.app.world();
        if let Some(show) = world.get_resource::<GemShow>() {
            paging.push(Paging {
                page: show.view().map(|view| view.page),
                bound: world
                    .get_resource::<KeyMap>()
                    .is_some_and(KeyMap::gem_paging),
                renders: show.renders,
            });
        }
    }
    (beats, paging)
}

/// Whether the pair is bound in this shell now.
fn launch_pages(shell: &Shell) -> bool {
    shell
        .app
        .world()
        .get_resource::<KeyMap>()
        .is_some_and(KeyMap::gem_paging)
}

fn gem_open(beat: &Beat) -> bool {
    beat.overlay.contains("GemView")
}

/// V at 5, then the feature's PgUp, PgDn, PgDn, PgDn and PgUp inside the
/// one-panel map, then the player's Space closes it at 110.
fn paging_script() -> Timeline {
    let mut timeline = Timeline::new(140);
    timeline.game_tap(5, KeyCode::KeyV);
    for (frame, key) in [
        (50, KeyCode::PageUp),
        (60, KeyCode::PageDown),
        (70, KeyCode::PageDown),
        (80, KeyCode::PageDown),
        (90, KeyCode::PageUp),
    ] {
        timeline.feature_tap(frame, key);
    }
    timeline.game_tap(110, KeyCode::Space);
    timeline
}

#[test]
fn paging_inside_a_one_panel_map_equals_the_off_run_without_the_presses() {
    let timeline = paging_script();
    let off_timeline = timeline.without_feature();
    for launch in [gem_on(), gem_without("gem_map.sweep")] {
        let (on, paging) = run(&ladders(), &launch, W43, &timeline);
        let (off, _) = run(
            &ladders(),
            &VisualLaunchConfig::classic(),
            W43,
            &off_timeline,
        );
        assert_eq!(
            on, off,
            "digest, frame, queue, gems, view, idle polls, PRNG"
        );
        // The view stayed up through every press and closed on Space.
        assert!(on[45..110].iter().all(gem_open));
        assert!(!gem_open(on.last().unwrap()));
        assert_eq!(on.last().unwrap().gems, 1, "one gem spent");
        // The pages: up to level 2 (page -1), back, down to level 4, no
        // further (nothing below has a panel), and up again.
        let page_at = |frame: usize| paging[frame].page;
        assert_eq!(page_at(45), Some(0));
        assert_eq!(page_at(55), Some(-1));
        assert_eq!(page_at(65), Some(0));
        assert_eq!(page_at(75), Some(1));
        assert_eq!(page_at(85), Some(1), "level 5 has no panel");
        assert_eq!(page_at(95), Some(0));
        // One render per turned page, none for the press that went nowhere.
        assert_eq!(paging[105].renders - paging[45].renders, 4);
        // Bound while the map is up, released on the frame it closes.
        assert!(paging[45..110].iter().all(|p| p.bound));
        assert!(!paging.last().unwrap().bound);
        assert!(paging.last().unwrap().page.is_none());
    }
}

#[test]
fn pgup_at_any_other_time_reaches_the_game_and_closes_the_view() {
    // The player's PgUp: in a three-panel dungeon map, outdoors at 4:3 and
    // at 16:9, and with level paging off in a one-panel dungeon map.
    let mut timeline = Timeline::new(90);
    timeline.game_tap(5, KeyCode::KeyV);
    timeline.game_tap(50, KeyCode::PageUp);
    timeline.game_tap(70, KeyCode::PageDown);
    for (state, launch, window) in [
        (ladders(), gem_on(), W169),
        (outdoors(), gem_on(), W43),
        (outdoors(), gem_on(), W169),
        (ladders(), gem_without("gem_map.level_paging"), W43),
        (
            ladders(),
            launch(&[
                CliSetting::Enable("gem_map.enhanced".into()),
                CliSetting::Disable("gem_map.dungeon_stack".into()),
                CliSetting::Disable("gem_map.level_paging".into()),
            ]),
            W169,
        ),
    ] {
        let (on, paging) = run(&state, &launch, window, &timeline);
        let (off, _) = run(&state, &VisualLaunchConfig::classic(), window, &timeline);
        assert_eq!(on, off, "{window:?}");
        assert!(gem_open(&on[45]), "{window:?}: the view was up");
        assert!(!gem_open(&on[55]), "{window:?}: PgUp closed it");
        assert!(paging.iter().all(|p| !p.bound), "{window:?}: nothing bound");
    }
}

#[test]
fn with_the_stack_off_the_lone_panel_pages_at_16_9_too() {
    let timeline = paging_script();
    let on_launch = gem_without("gem_map.dungeon_stack");
    let (on, paging) = run(&ladders(), &on_launch, W169, &timeline);
    let (off, _) = run(
        &ladders(),
        &VisualLaunchConfig::classic(),
        W169,
        &timeline.without_feature(),
    );
    assert_eq!(on, off);
    assert_eq!(paging[55].page, Some(-1));
    assert_eq!(paging[75].page, Some(1));
}

#[test]
fn a_paging_key_never_reaches_the_game_while_paging() {
    let mut shell = shell(&ladders(), &gem_on(), W43);
    let mut timeline = Timeline::new(60);
    timeline.game_tap(5, KeyCode::KeyV);
    for frame in 0..45 {
        shell.step(timeline.frame(frame));
    }
    assert!(launch_pages(&shell), "bound while the one-panel map is up");
    let polls = shell.visual().state.view_overlay_idle_polls;
    let mut press = Timeline::new(2);
    press.feature_tap(0, KeyCode::PageUp);
    shell.step(press.frame(0));
    {
        let visual = shell.visual();
        assert!(
            visual
                .typed_keys
                .iter()
                .all(|typed| typed.key != KeyCode::PageUp)
        );
        assert!(gem_open_state(&visual.state), "the view stays up");
        assert!(visual.state.view_overlay_idle_polls >= polls, "it waits");
        let keyboard = shell.app.world().resource::<ButtonInput<KeyCode>>();
        assert!(
            !keyboard.just_pressed(KeyCode::PageUp),
            "reset by the router"
        );
    }
    shell.step(press.frame(1));
    let show = shell.app.world().resource::<GemShow>();
    assert_eq!(show.view().map(|view| view.page), Some(-1));
    // The mapping function still maps PgUp and PgDn when nothing is
    // bound: north-east and south-east, as in 1.0.1.
    assert_eq!(
        key_code_to_input_byte(KeyCode::PageUp, false, false),
        Some(u5_runtime::INPUT_CODE_NORTHEAST)
    );
    assert!(key_code_to_input_byte(KeyCode::PageDown, false, false).is_some());
    assert!(KeyChord::new(KeyCode::PageUp).reserved());
}

fn gem_open_state(state: &PlayState) -> bool {
    u5_automap::gem::gem_view_open(state)
}

#[test]
fn with_level_paging_off_nothing_is_bound_and_no_router_is_added() {
    // Only the gem map on: no router at all.
    let shell = shell(&ladders(), &gem_without("gem_map.level_paging"), W43);
    assert!(!shell.app.world().contains_resource::<KeyMap>());
    let names = system_names(&shell);
    assert!(!names.iter().any(|n| n.ends_with("gem_page")), "{names:?}");
    assert!(!names.iter().any(|n| n.ends_with("gem_paging_keys")));
    assert!(!names.iter().any(|n| n.ends_with("enh_input_router")));
    // With paging on, the router is there for it (D-M3.6), its keymap
    // empty until a one-panel map opens.
    let shell = shell_for(&gem_on());
    let keymap = shell.app.world().resource::<KeyMap>();
    assert!(keymap.is_empty() && !keymap.gem_paging());
    let names = system_names(&shell);
    for system in ["gem_page", "gem_paging_keys", "enh_input_router"] {
        assert!(names.iter().any(|n| n.ends_with(system)), "{system}");
    }
    // Classic: none of it.
    let shell = shell_for(&VisualLaunchConfig::classic());
    assert!(!shell.app.world().contains_resource::<KeyMap>());
    // With another feature's key bound and paging off, the pair never
    // appears, through a whole one-panel dungeon map.
    let both = launch(&[
        CliSetting::Enable("gem_map.enhanced".into()),
        CliSetting::Disable("gem_map.level_paging".into()),
        CliSetting::Enable("display.grow_left".into()),
    ]);
    let mut shell = shell_with(&ladders(), &both, W43);
    let mut timeline = Timeline::new(60);
    timeline.game_tap(5, KeyCode::KeyV);
    for frame in 0..60 {
        shell.step(timeline.frame(frame));
        let keymap = shell.app.world().resource::<KeyMap>();
        assert!(!keymap.gem_paging());
        assert_eq!(keymap.action(KeyChord::new(KeyCode::PageUp)), None);
        assert_eq!(keymap.action(KeyChord::new(KeyCode::PageDown)), None);
    }
    assert!(gem_open_state(&shell.visual().state));
}

fn shell_for(launch: &VisualLaunchConfig) -> Shell {
    shell_with(&ladders(), launch, W43)
}

fn shell_with(state: &PlayState, launch: &VisualLaunchConfig, window: (f32, f32)) -> Shell {
    shell(state, launch, window)
}

fn system_names(shell: &Shell) -> Vec<String> {
    shell
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
        .collect()
}

#[test]
fn the_pair_goes_with_the_toggle_and_the_view() {
    let mut shell = shell(&ladders(), &gem_on(), W43);
    let mut timeline = Timeline::new(50);
    timeline.game_tap(5, KeyCode::KeyV);
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
    }
    assert!(launch_pages(&shell));
    let keymap = shell.app.world().resource::<KeyMap>();
    assert_eq!(
        keymap.action(KeyChord::new(KeyCode::PageUp)),
        Some(EnhAction::GemPageUp)
    );
    assert_eq!(
        keymap.action(KeyChord::new(KeyCode::PageDown)),
        Some(EnhAction::GemPageDown)
    );
    // The toggle goes off with the map up: released on that frame, and the
    // player's PgUp then closes the game's view as in 1.0.1.
    shell.app.insert_resource(Toggles(FeatureToggles::CLASSIC));
    shell.step(&[]);
    assert!(!launch_pages(&shell));
    let keymap = shell.app.world().resource::<KeyMap>();
    assert!(keymap.is_empty());
    let mut press = Timeline::new(2);
    press.game_tap(0, KeyCode::PageUp);
    shell.step(press.frame(0));
    shell.step(press.frame(1));
    assert!(!gem_open_state(&shell.visual().state));
}

// ---------------------------------------------------------------------------
// Review shots (never committed).

/// The render check (plan M3-14): the mockups' D3 scene, Despise level 3
/// at (6, 1) facing north, in the real shell on a 4:3 window, the record
/// seeded with the mockups' demo walk from `U5_GEM_DUNGEON_MEMORY` (`cell L
/// X Y` and `trap L X Y` lines). V, then PgUp, PgDn and PgDn: the held map
/// of each page is written to `U5_GEM_SHOTS` for the side-by-side with
/// `D3`, `D3-pgup`, `D3-pgdn` and the GIF. Skipped unless
/// `U5_MOCKUP_PROFILE` and `U5_GEM_SHOTS` are set.
#[test]
fn gem_paging_review_shots() {
    use u5_automap::gem::{DungeonMark, KnowEvent, MarkKind};
    use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_GEM_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_GEM_SHOTS");
        return;
    };
    let game = unique_scratch_dir("gem-paging-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    let mut state = PlayState::load_scene(
        &game,
        u5_runtime::PlayOptions {
            target: u5_runtime::PlayTarget::Dungeon(
                u5_runtime::DungeonScene::from_record(1).unwrap(),
            ),
            floor: 2,
            start: Some((6, 1)),
            clock: u5_runtime::GameClock::new(12, 0).unwrap(),
            ..u5_runtime::PlayOptions::default()
        },
    )
    .unwrap();
    state.player.facing = u5_runtime::Direction::North;
    state.gems = 1;
    let mut events = Vec::new();
    if let Some(path) = configured_test_dir("U5_GEM_DUNGEON_MEMORY") {
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            let [kind, l, x, y] = words[..] else { continue };
            let (l, x, y): (u8, u8, u8) =
                (l.parse().unwrap(), x.parse().unwrap(), y.parse().unwrap());
            match kind {
                "cell" => {
                    let mut rows = [0; 8];
                    rows[usize::from(y)] = 1 << x;
                    events.push(KnowEvent::DungeonCellsSeen {
                        record: 1,
                        level: l,
                        rows,
                    });
                }
                "trap" => events.push(KnowEvent::DungeonMarked(DungeonMark {
                    record: 1,
                    level: l,
                    x,
                    y,
                    kind: MarkKind::Trap,
                })),
                _ => {}
            }
        }
    }
    let mut shell = Shell::new(state, &scene, &gem_on(), |_| {});
    shell.app.world_mut().spawn(Window {
        resolution: WindowResolution::new(W43.0, W43.1),
        ..default()
    });
    let mut timeline = Timeline::new(120);
    timeline.game_tap(8, KeyCode::KeyV);
    timeline.feature_tap(50, KeyCode::PageUp);
    timeline.feature_tap(70, KeyCode::PageDown);
    timeline.feature_tap(90, KeyCode::PageDown);
    let shots = [(45, "level3"), (65, "pgup"), (85, "back"), (105, "pgdn")];
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        if frame == 2 {
            shell
                .app
                .world_mut()
                .resource_mut::<super::super::GemMap>()
                .learn(&events);
        }
        if let Some(&(_, name)) = shots.iter().find(|(at, _)| *at == frame) {
            let show = shell.app.world().resource::<GemShow>();
            let handle = show.image().cloned().expect("the map shows");
            let image = shell
                .app
                .world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .unwrap();
            let size = image.size();
            image::RgbaImage::from_raw(size.x, size.y, image.data.clone().unwrap())
                .unwrap()
                .save(out.join(format!("engine-shell-D3-{name}.png")))
                .unwrap();
        }
    }
    assert!(u5_automap::gem::gem_view_open(&shell.visual().state));
    let _ = std::fs::remove_dir_all(game);
}
