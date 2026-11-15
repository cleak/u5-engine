//! The journal's trajectory equivalence (design 5.6 E1-E4; build plan
//! M5-06), on the foundation harness ([`crate::enh::equivalence`]).
//!
//! Each run is compared with the all-off run frame by frame on the
//! trajectory digest, the original frame and the game's key queue, and
//! then on the bytes of every save file a scripted `Q`, `Y` writes from the
//! final state.
//!
//! - E1: the page opened after a step and kept open for N frames, against
//!   the off run with those N frames idle, with the open on a tick
//!   boundary and mid-interval;
//! - E2: as E1, with combat starting under the open page, which then
//!   closes itself;
//! - E3: the side panel on (page off) in a window that grants it;
//! - E4: the page on but never opened, Ctrl+J typed where the game is not
//!   at its idle prompt.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::input::ButtonState;
use bevy::window::{Window, WindowResolution};
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, JOURNAL_P3, JOURNAL_P3B, LockState, ResolveInput,
    resolve_as_shipped,
};
use u5_runtime::test_fixtures::{
    open_grid, saved_game_seed_bytes, test_state, unique_scratch_dir, write_empty_ool_mirrors,
};
use u5_runtime::{
    BRIT_OOL_FILENAME, COMBAT_ACTOR_FLAG_SELECTABLE_80, CombatActorDescriptor, PlayState,
    SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, UNDER_OOL_FILENAME, handle_play_key_input,
};

use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, ShellRun, Timeline, compare_shell_runs};
use crate::enh::side::{PanelKind, SideLayout};

const SAVE_FILES: [&str; 4] = [
    SAVED_GAM_FILENAME,
    SAVED_OOL_FILENAME,
    BRIT_OOL_FILENAME,
    UNDER_OOL_FILENAME,
];

/// The journal's keys, resolved as a host would once chapter 5 ships.
fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    let enhancements = resolve_as_shipped(
        ResolveInput {
            stored: None,
            cli,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        },
        &[JOURNAL_P3, JOURNAL_P3B],
    );
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

fn on(key: &str) -> CliSetting {
    CliSetting::Enable(key.into())
}

fn page_on() -> VisualLaunchConfig {
    launch(&[on("journal.page")])
}

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

/// A scratch folder holding the synthetic seed save, removed when dropped.
struct Seed(PathBuf);

impl Seed {
    fn new() -> Self {
        let dir = unique_scratch_dir("journal-equivalence");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x01, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&dir);
        Self(dir)
    }
}

impl Drop for Seed {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `Q`, `Y` from `state` into a copy of the seed save: each file's bytes.
fn scripted_save(mut state: PlayState, seed: &Path) -> Vec<(&'static str, Option<Vec<u8>>)> {
    let dir = unique_scratch_dir("journal-equivalence-save");
    fs::create_dir_all(&dir).unwrap();
    for entry in fs::read_dir(seed).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
    }
    handle_play_key_input(&mut state, 'Q', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    let saves = SAVE_FILES
        .iter()
        .map(|name| (*name, fs::read(dir.join(name)).ok()))
        .collect();
    let _ = fs::remove_dir_all(&dir);
    saves
}

/// What a run is compared on, and what the journal did in it.
struct Trace {
    run: ShellRun,
    saves: Vec<(&'static str, Option<Vec<u8>>)>,
    /// Whether the page was open, per frame.
    open: Vec<bool>,
    /// The frames on which the world's animation clock stepped.
    ticks: Vec<usize>,
    /// Whether the journal's panel was granted, per frame.
    panel: Vec<bool>,
    state: PlayState,
}

/// The test-only system both runs of a case get (the game's own doing,
/// not the feature's): run once on its frame.
type Inject = fn(&mut PlayState);

#[derive(Resource)]
struct Injection(usize, Inject);

fn inject(mut visual: ResMut<VisualState>, injection: Res<Injection>, mut frames: Local<usize>) {
    if *frames == injection.0 {
        (injection.1)(&mut visual.state);
    }
    *frames += 1;
}

fn run(
    launch: &VisualLaunchConfig,
    timeline: &Timeline,
    window: Option<(f32, f32)>,
    injection: Option<(usize, Inject)>,
    seed: &Seed,
) -> Trace {
    let mut shell = Shell::new(town(), &Scene::synthetic(), launch, |app| {
        if let Some((frame, apply)) = injection {
            app.insert_resource(Injection(frame, apply))
                .add_systems(First, inject);
        }
    });
    if let Some((w, h)) = window {
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(w, h),
            ..default()
        });
    }
    let mut frames = Vec::with_capacity(timeline.len());
    let mut open = Vec::with_capacity(timeline.len());
    let mut ticks = Vec::new();
    let mut panel = Vec::with_capacity(timeline.len());
    let mut clock = None;
    for index in 0..timeline.len() {
        frames.push(shell.step(timeline.frame(index)));
        let world = shell.app.world();
        open.push(
            world
                .get_resource::<OverlayState>()
                .is_some_and(|state| state.open() == Some(OverlayId::Journal)),
        );
        panel.push(world.get_resource::<SideLayout>().is_some_and(|layout| {
            layout
                .granted
                .iter()
                .any(|(kind, _)| *kind == PanelKind::Journal)
        }));
        let now = shell.visual().state.animation.frame;
        if clock.is_some_and(|clock| clock != now) {
            ticks.push(index);
        }
        clock = Some(now);
    }
    let state = shell.visual().state.clone();
    Trace {
        run: ShellRun {
            frames,
            fed: Vec::new(),
        },
        saves: scripted_save(state.clone(), &seed.0),
        open,
        ticks,
        panel,
        state,
    }
}

/// Every difference between the off run and `on`, as lines.
fn differences(off: &Trace, on: &Trace) -> Vec<String> {
    let mut lines: Vec<String> = compare_shell_runs(&off.run, &on.run)
        .iter()
        .map(ToString::to_string)
        .collect();
    for ((name, a), (_, b)) in off.saves.iter().zip(&on.saves) {
        if a != b {
            lines.push(format!("{name} differs after the scripted save"));
        }
    }
    if off.state.turn != on.state.turn {
        lines.push(format!("turn {} / {}", off.state.turn, on.state.turn));
    }
    lines
}

/// The all-off run of `timeline` with the feature's events removed.
fn off_run(
    timeline: &Timeline,
    window: Option<(f32, f32)>,
    injection: Option<(usize, Inject)>,
    seed: &Seed,
) -> Trace {
    let off = run(
        &VisualLaunchConfig::classic(),
        &timeline.without_feature(),
        window,
        injection,
        seed,
    );
    assert!(off.open.iter().all(|open| !*open));
    assert!(
        off.saves[0].1.is_some(),
        "the scripted save must write SAVED.GAM"
    );
    off
}

fn ctrl_j(timeline: &mut Timeline, frame: usize, owner: Owner) {
    for key in [KeyCode::ControlLeft, KeyCode::KeyJ] {
        timeline.event(frame, key, ButtonState::Pressed, false, owner);
    }
    for key in [KeyCode::KeyJ, KeyCode::ControlLeft] {
        timeline.event(frame + 1, key, ButtonState::Released, false, owner);
    }
}

/// Script S: a short town walk, one tap every 15 frames from frame 5.
const WALK: [KeyCode; 6] = [
    KeyCode::ArrowUp,
    KeyCode::ArrowLeft,
    KeyCode::Space,
    KeyCode::ArrowDown,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
];

/// S with the page opened at `open`, used for `frames` frames, closed by
/// Esc; the walk resumes after it.
fn s_with_page(open: usize, frames: usize) -> Timeline {
    let mut timeline = Timeline::new(open + frames + WALK.len() * 15 + 60);
    // The first three steps before the open, the rest after the close.
    for (index, key) in WALK.iter().enumerate() {
        let frame = if index < 3 {
            5 + index * 15
        } else {
            open + frames + 10 + (index - 3) * 15
        };
        timeline.game_tap(frame, *key);
    }
    ctrl_j(&mut timeline, open, Owner::Feature);
    let keys = [
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::Enter,
        KeyCode::KeyA,
    ];
    for (index, key) in keys.iter().enumerate() {
        let at = open + 3 + index * 4;
        if at + 2 < open + frames {
            timeline.feature_tap(at, *key);
        }
    }
    timeline.feature_tap(open + frames, KeyCode::Escape);
    timeline
}

/// The tick frames of an idle Classic run past the first three steps, to
/// place E1's open on a boundary and between two.
fn idle_ticks(seed: &Seed) -> Vec<usize> {
    let timeline = s_with_page(60, 120).without_feature();
    off_run(&timeline, None, None, seed)
        .ticks
        .into_iter()
        .filter(|frame| *frame >= 50)
        .collect()
}

/// E1: an open/close after step 3 with N frames open equals the off run
/// with N idle frames there, frame for frame and in the save, with the
/// open on a tick boundary and mid-interval.
#[test]
fn e1_an_open_page_equals_idle_frames_at_a_boundary_and_mid_interval() {
    let seed = Seed::new();
    let ticks = idle_ticks(&seed);
    assert!(
        ticks.len() >= 3,
        "the world must tick while idle: {ticks:?}"
    );
    let boundary = ticks[0];
    let interval = ticks[1] - ticks[0];
    assert!(interval >= 2, "{ticks:?}");
    let mid = boundary + interval / 2;
    for (label, open, frames) in [
        ("boundary", boundary, 60),
        ("mid-interval", mid, 60),
        ("boundary, long", boundary, 3 * interval + 1),
    ] {
        let timeline = s_with_page(open, frames);
        let off = off_run(&timeline, None, None, &seed);
        let on = run(&page_on(), &timeline, None, None, &seed);
        assert_eq!(differences(&off, &on), Vec::<String>::new(), "{label}");
        assert!(!on.open[open - 1], "{label}: closed before Ctrl+J");
        assert!(
            on.open[open + 1..open + frames].iter().all(|o| *o),
            "{label}: open for the N frames"
        );
        assert!(
            on.open[open + frames + 1..].iter().all(|o| !*o),
            "{label}: closed by Esc"
        );
        assert!(
            on.ticks.iter().any(|t| (open..open + frames).contains(t)),
            "{label}: the world ticked under the page"
        );
        let first = off.run.frames[0].digest;
        assert!(off.run.frames.iter().any(|frame| frame.digest != first));
    }
}

/// Combat begins under the page: the state a monster's attack leaves, at
/// a command prompt for the first member (as G7's combat prompt).
fn combat_begins(state: &mut PlayState) {
    state.combat_active = true;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.pending_combat_actor_slot = Some(0);
}

/// E2: as E1, with combat starting under the open page (the same change
/// on the same frame of both runs). The page closes itself, no page key
/// reaches the game, and the runs stay equal.
#[test]
fn e2_combat_under_the_page_closes_it_and_changes_nothing() {
    let seed = Seed::new();
    let open = 60;
    let starts = open + 30;
    // The page is used, then combat starts under it and closes it; the
    // keys after that are the player's again, which both runs see.
    let mut timeline = s_with_page(open, 80);
    timeline = {
        let mut cut = Timeline::new(timeline.len());
        for index in 0..timeline.len() {
            for event in timeline.frame(index) {
                if event.owner == Owner::Game || index < starts {
                    cut.event(index, event.key, event.state, event.repeat, event.owner);
                }
            }
        }
        cut.game_tap(starts + 20, KeyCode::Space);
        cut
    };
    let injection = Some((starts, combat_begins as Inject));
    let off = off_run(&timeline, None, injection, &seed);
    let on = run(&page_on(), &timeline, None, injection, &seed);
    assert!(on.state.combat_active && off.state.combat_active);
    assert!(on.open[open + 2..starts].iter().all(|o| *o), "open");
    assert!(
        on.open[starts + 1..].iter().all(|o| !*o),
        "closed by the combat"
    );
    assert_eq!(differences(&off, &on), Vec::<String>::new());
}

/// E3: the side panel on, the page off, in a 16:9 window where the panel
/// is granted: the digests, the save bytes and the play image on every
/// frame equal the off run.
#[test]
fn e3_the_side_panel_changes_nothing() {
    let seed = Seed::new();
    let mut timeline = Timeline::new(5 + WALK.len() * 15 + 40);
    for (index, key) in WALK.iter().enumerate() {
        timeline.game_tap(5 + index * 15, *key);
    }
    // Ctrl+J with the page off is the player's key, which the game sees.
    ctrl_j(&mut timeline, 70, Owner::Game);
    let window = Some((1920.0, 1080.0));
    for side in ["auto", "left", "right"] {
        let launch = launch(&[
            on("journal.side_panel"),
            CliSetting::Set {
                key: "journal.side_panel_side".into(),
                value: side.into(),
            },
        ]);
        assert!(!launch.enhancements.toggles.on(Toggle::JournalPage));
        let off = off_run(&timeline, window, None, &seed);
        let on = run(&launch, &timeline, window, None, &seed);
        assert!(on.panel.iter().skip(2).all(|p| *p), "{side}: panel granted");
        assert!(on.open.iter().all(|o| !*o), "{side}: no page");
        assert_eq!(differences(&off, &on), Vec::<String>::new(), "{side}");
    }
}

/// E4: the page on but never opened. Ctrl+J typed at a pending command
/// prompt, in the same frame as a step and behind a queued key is refused
/// and reaches the game as the player's key, as in the off run.
#[test]
fn e4_ctrl_j_away_from_the_idle_prompt_never_opens_the_page() {
    let seed = Seed::new();
    let mut timeline = Timeline::new(200);
    timeline.game_tap(5, KeyCode::ArrowUp);
    // A: the game asks for a direction; Ctrl+J at that prompt; Space
    // answers it.
    timeline.game_tap(20, KeyCode::KeyA);
    ctrl_j(&mut timeline, 24, Owner::Game);
    timeline.game_tap(30, KeyCode::Space);
    // In the same frame as a step, after it.
    timeline.game_tap(50, KeyCode::ArrowLeft);
    ctrl_j(&mut timeline, 50, Owner::Game);
    // Talk, then Ctrl+J while the game asks whom.
    timeline.game_tap(80, KeyCode::KeyT);
    ctrl_j(&mut timeline, 83, Owner::Game);
    timeline.game_tap(90, KeyCode::Escape);
    timeline.game_tap(120, KeyCode::ArrowDown);
    timeline.game_tap(150, KeyCode::ArrowRight);
    let off = off_run(&timeline, None, None, &seed);
    let on = run(&page_on(), &timeline, None, None, &seed);
    let opened: Vec<usize> = (0..on.open.len()).filter(|at| on.open[*at]).collect();
    assert_eq!(opened, Vec::<usize>::new(), "the page never opened");
    assert_eq!(differences(&off, &on), Vec::<String>::new());
}

/// The cases see what they compare: a page key typed into the off run's
/// game, or a changed save, is reported.
#[test]
fn a_page_key_in_the_game_or_a_changed_save_is_caught() {
    let seed = Seed::new();
    let timeline = s_with_page(60, 40);
    let off = off_run(&timeline, None, None, &seed);
    // The page's own keys given to the game: the off run sees them.
    let mut leaked = Timeline::new(timeline.len());
    for index in 0..timeline.len() {
        for event in timeline.frame(index) {
            leaked.event(index, event.key, event.state, event.repeat, Owner::Game);
        }
    }
    let leak = run(&VisualLaunchConfig::classic(), &leaked, None, None, &seed);
    assert!(!differences(&off, &leak).is_empty());
    let mut changed = run(&page_on(), &timeline, None, None, &seed);
    assert_eq!(differences(&off, &changed), Vec::<String>::new());
    if let Some(bytes) = changed.saves[0].1.as_mut() {
        bytes[0] ^= 1;
    }
    assert_eq!(
        differences(&off, &changed),
        ["SAVED.GAM differs after the scripted save"]
    );
}
