//! Chapter 7's hardening tests (design 1.4.7, 7.4.3, 7.4.5, 7.6; build plan
//! M8-06):
//!
//! - **Spoilers.** From a fresh save with every cheat key off there is no
//!   Cheats page, no menu item and no teleport list. With `teleport` on the
//!   Teleport face's title says `SPOILERS` and its warning line is drawn.
//!   With only `state_watch` on, a scripted walk records no quest-flag,
//!   seal, Shadowlord or hideout field.
//! - **Equivalence.** Each row of the matrix's cheats block, with the page
//!   opened and closed and nothing used, equals the all-off run.
//! - **Labelled divergence.** Two runs that apply the same cheats on the
//!   same frames are identical, and a cheated run differs from the run
//!   without the cheats only in the fields those cheats list.
//! - **The parity lock.** Every cheat asked for, plus the lock: no plugin,
//!   no page and no marker I/O. (The host's one locked line is checked in
//!   `u5-tui`, which prints it.)
//!
//! The fresh-save run uses the player's real files when `U5_TEST_ASSET_DIR`
//! names a writable copy, and the synthetic town otherwise.

use std::fs;

use bevy::input::ButtonState;
use u5_runtime::cheats::{WatchField, watch_diff, watch_snapshot};
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockReason, LockState, ResolveInput, ResolvedEnhancements,
    SIDECAR_DIR, TOGGLES, cheats_block, resolve,
};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, copy_asset_writable, open_grid, test_state, unique_scratch_dir,
};
use u5_runtime::{ChargenStats, PlayState, commit_chargen_save, load_play_options_from_save};

use super::teleport::{SPOILER_LINES, TELEPORT_ENTRY, TELEPORT_TITLE};
use super::tests::{ctrl_f12, game, has_system, meta_path, only, page, q_y_save, save_dir, tap};
use super::*;
use crate::enh::equivalence::{
    FrameRecord, Owner, Scene, Shell, Timeline, check_shell_equivalence,
};
use crate::enh::input::{EnhAction, KeyMap};
use crate::enh::{EnhancementsPlugin, add_enhancements};

/// A synthetic town at its idle prompt, the party wounded and short of
/// gold, so Heal all and Max gold each have work.
fn town() -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    for member in &mut state.party {
        member.status = b'G';
        member.max_hp = 100;
        member.hp = 10;
    }
    state.gold = 5;
    state
}

/// What a host resolves from `settings` with no file.
fn resolved(settings: &[CliSetting], lock: LockState) -> ResolvedEnhancements {
    resolve(ResolveInput {
        stored: None,
        cli: settings,
        lock,
        caps: HostCapabilities::ALL,
    })
}

fn launch_of(enhancements: ResolvedEnhancements) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

/// `--enable` for every `[cheats]` key.
fn every_cheat_asked() -> Vec<CliSetting> {
    TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("cheats."))
        .map(|spec| CliSetting::Enable(spec.key.into()))
        .collect()
}

/// Ctrl+F12 at `frame`, owned by `owner`.
fn chord(timeline: &mut Timeline, frame: usize, owner: Owner) {
    timeline.event(
        frame,
        KeyCode::ControlLeft,
        ButtonState::Pressed,
        false,
        owner,
    );
    timeline.tap(frame + 1, KeyCode::F12, owner);
    timeline.event(
        frame + 3,
        KeyCode::ControlLeft,
        ButtonState::Released,
        false,
        owner,
    );
}

/// The player walks: north, west, a pass, south, east.
fn walk(timeline: &mut Timeline, from: usize) {
    for (i, key) in [
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
    ]
    .into_iter()
    .enumerate()
    {
        timeline.game_tap(from + i * 15, key);
    }
}

// ---------------------------------------------------------------------------
// Spoilers (1.4.7).

/// The player's fresh save after character creation, when real files are
/// configured.
fn fresh_save() -> Option<(PlayState, Scene, std::path::PathBuf)> {
    let dir = configured_original_asset_dir()?;
    if !dir.join("INIT.GAM").exists() {
        return None;
    }
    let game = unique_scratch_dir("cheats-spoiler");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    commit_chargen_save(
        &game,
        b"Tester",
        true,
        ChargenStats {
            strength: 15,
            dexterity: 15,
            intelligence: 15,
        },
    )
    .unwrap();
    let options = load_play_options_from_save(&game).unwrap();
    let state = PlayState::load_scene(&game, options).unwrap();
    let scene = Scene::from_game_dir(&game).unwrap();
    Some((state, scene, game))
}

/// A fresh install's settings (no file, no flag) give no Cheats page, no
/// way to ask for it and no teleport list, and Ctrl+F12 there is the
/// player's key, equal to Classic.
#[test]
fn a_fresh_save_with_every_cheat_off_has_no_page_item_or_list() {
    let fresh = resolved(&[], LockState::Off);
    assert_eq!(fresh.cheats(), CheatToggles::default());
    assert!(!page_available(fresh.toggles));
    assert!(!EnhAction::OpenCheats.enabled(fresh.toggles));
    let launch = launch_of(fresh);
    assert!(!cheats_wanted(&launch));

    let real = fresh_save();
    let (start, scene) = match &real {
        Some((state, scene, _)) => (state.clone(), scene.clone()),
        None => (town(), Scene::synthetic()),
    };
    let mut timeline = Timeline::new(60);
    chord(&mut timeline, 5, Owner::Game);
    chord(&mut timeline, 30, Owner::Game);
    let mut shell = Shell::new(start.clone(), &scene, &launch, |_| {});
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let world = shell.app.world();
        assert!(!world.contains_resource::<CheatPage>(), "frame {frame}");
        assert!(!world.contains_resource::<CheatSession>(), "frame {frame}");
        assert!(
            world
                .get_resource::<OverlayState>()
                .is_none_or(|overlay| overlay.open().is_none()),
            "frame {frame}"
        );
    }
    assert!(!shell.app.is_plugin_added::<CheatsPlugin>());
    if let Some(keymap) = shell.app.world().get_resource::<KeyMap>() {
        assert!(keymap.iter().all(|(_, a)| a != EnhAction::OpenCheats));
    }
    let mismatches = check_shell_equivalence(&start, &scene, &timeline, &launch, |_| {});
    assert_eq!(mismatches, []);
    if let Some((_, _, dir)) = real {
        let _ = fs::remove_dir_all(dir);
    }
}

/// With `teleport` on, `[T]` opens the face whose title says `SPOILERS`
/// with its warning line under it; with it off, `[T]` is neither drawn nor
/// heard, and there is no teleport list.
#[test]
fn teleport_on_shows_spoilers_and_its_warning() {
    assert!(TELEPORT_TITLE.contains("SPOILERS"));
    assert!(
        Toggle::CheatsTeleport
            .spec()
            .label
            .starts_with("Teleport (Spoiler"),
        "the settings row names the spoiler"
    );

    let mut on = game(only(&[Toggle::CheatsEnabled, Toggle::CheatsTeleport]), None);
    ctrl_f12(&mut on);
    assert_eq!(on.open(), Some(OverlayId::Cheats));
    let texts = page::page_view_for_tests(
        (320, 200),
        &cheat_toggles(only(&[Toggle::CheatsEnabled, Toggle::CheatsTeleport])),
        &CheatSession::default(),
    )
    .texts();
    assert!(texts.iter().any(|t| t == TELEPORT_ENTRY));
    tap(&mut on, KeyCode::KeyT);
    assert_eq!(page(&on).mode(), PageMode::Teleport);
    let form = page(&on).teleport().expect("the teleport list");
    let runs = form.runs(&on.visual().state);
    let lines: Vec<&str> = runs.iter().take(2).map(|r| r.text.as_str()).collect();
    assert_eq!(
        lines, SPOILER_LINES,
        "the warning is the face's first lines"
    );
    assert_eq!((runs[0].row, runs[1].row), (1, 2), "under the title");

    let mut off = game(only(&[Toggle::CheatsEnabled, Toggle::CheatsHealAll]), None);
    ctrl_f12(&mut off);
    assert_eq!(off.open(), Some(OverlayId::Cheats));
    let texts = page::page_view_for_tests(
        (320, 200),
        &cheat_toggles(only(&[Toggle::CheatsEnabled, Toggle::CheatsHealAll])),
        &CheatSession::default(),
    )
    .texts();
    assert!(!texts.iter().any(|t| t == TELEPORT_ENTRY));
    tap(&mut off, KeyCode::KeyT);
    assert_eq!(page(&off).mode(), PageMode::Main);
    assert!(page(&off).teleport().is_none());
}

/// Words that would name a quest step or the endgame (1.4.7).
const SECRET_WORDS: [&str; 6] = ["quest", "flag", "seal", "shadow", "hideout", "virtue"];

fn names_a_secret(label: &str) -> bool {
    let label = label.to_ascii_lowercase();
    SECRET_WORDS.iter().any(|word| label.contains(word))
}

/// With only the watch on, a walk records changes, and none of them is a
/// quest flag, a seal, a Shadowlord or a hideout; neither is any field the
/// watch can name.
#[test]
fn the_watch_alone_records_no_secret_field() {
    let mut game = game(only(&[Toggle::CheatsStateWatch]), None);
    assert!(!game.app.world().contains_resource::<CheatSession>());
    for key in [
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::Space,
    ] {
        tap(&mut game, key);
        for _ in 0..8 {
            game.frame();
        }
    }
    let log = game.app.world().resource::<WatchLog>();
    assert!(!log.is_empty(), "the walk must record something");
    for entry in log.entries() {
        let label = entry.field.label();
        assert!(!names_a_secret(&label), "{label}");
        assert!(!names_a_secret(&format!("{:?}", entry.field)), "{label}");
    }
    for (field, _) in watch_snapshot(&game.visual().state) {
        assert!(!names_a_secret(&field.label()), "{}", field.label());
        assert!(!names_a_secret(&format!("{field:?}")), "{field:?}");
    }
    assert!(!names_a_secret(&log.log_text()));
}

// ---------------------------------------------------------------------------
// Equivalence (7.4.3).

/// Every row of the matrix's cheats block, with the page opened and closed
/// and nothing used, equals the all-off run frame for frame.
#[test]
fn every_cheats_block_row_with_the_page_opened_unused_equals_all_off() {
    let mut timeline = Timeline::new(110);
    walk(&mut timeline, 5);
    chord(&mut timeline, 80, Owner::Feature);
    timeline.feature_tap(92, KeyCode::Escape);
    let block = cheats_block();
    assert_eq!(block.len(), 6);
    for row in block {
        let enhancements = resolved(&row.settings, LockState::Off);
        assert!(
            enhancements.notes.is_empty(),
            "{}: {:?}",
            row.name,
            enhancements.notes
        );
        assert!(enhancements.cheats().any_on(), "{}", row.name);
        let launch = launch_of(enhancements);
        let mut shell = Shell::new(town(), &Scene::synthetic(), &launch, |_| {});
        let mut opened = false;
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
            opened |=
                shell.app.world().resource::<OverlayState>().open() == Some(OverlayId::Cheats);
        }
        assert!(opened, "{}: the page opened", row.name);
        let mismatches =
            check_shell_equivalence(&town(), &Scene::synthetic(), &timeline, &launch, |_| {});
        assert_eq!(mismatches, [], "{}", row.name);
    }
}

// ---------------------------------------------------------------------------
// Labelled divergence (7.4.3, 7.6).

/// The walk with Heal all and Max gold used through the page between steps.
fn cheated_timeline() -> Timeline {
    let mut timeline = Timeline::new(150);
    walk(&mut timeline, 5);
    chord(&mut timeline, 80, Owner::Feature);
    timeline.feature_tap(88, KeyCode::KeyH);
    timeline.feature_tap(94, KeyCode::KeyG);
    timeline.feature_tap(100, KeyCode::Escape);
    for (i, key) in [KeyCode::ArrowUp, KeyCode::Space, KeyCode::ArrowDown]
        .into_iter()
        .enumerate()
    {
        timeline.game_tap(110 + i * 12, key);
    }
    timeline
}

fn run(launch: &VisualLaunchConfig, timeline: &Timeline) -> (Vec<FrameRecord>, PlayState) {
    let mut shell = Shell::new(town(), &Scene::synthetic(), launch, |_| {});
    let frames = (0..timeline.len())
        .map(|frame| shell.step(timeline.frame(frame)))
        .collect();
    (frames, shell.visual().state.clone())
}

#[test]
fn the_same_cheats_on_the_same_frames_reproduce_and_change_only_their_fields() {
    let launch = launch_of(resolved(
        &[
            CliSetting::Enable("cheats.enabled".into()),
            CliSetting::Enable("cheats.heal_all".into()),
            CliSetting::Enable("cheats.max_gold".into()),
        ],
        LockState::Off,
    ));
    let timeline = cheated_timeline();
    let (first, cheated) = run(&launch, &timeline);
    let (second, again) = run(&launch, &timeline);
    assert_eq!(first, second, "the same cheats on the same frames");
    assert_eq!(watch_snapshot(&cheated), watch_snapshot(&again));

    let (_, baseline) = run(&VisualLaunchConfig::classic(), &timeline.without_feature());
    assert_ne!(cheated.gold, baseline.gold, "Max gold was used");
    let diff = watch_diff(&watch_snapshot(&baseline), &watch_snapshot(&cheated));
    assert!(!diff.is_empty());
    for (field, old, new) in &diff {
        assert!(
            matches!(field, WatchField::Gold | WatchField::MemberHp(_)),
            "{field:?} {old:?} -> {new:?} is not a field Heal all or Max gold lists"
        );
    }
    assert!(diff.iter().any(|(f, ..)| *f == WatchField::Gold));
    assert!(
        diff.iter()
            .any(|(f, ..)| matches!(f, WatchField::MemberHp(_)))
    );
    assert_eq!(cheated.prng_state, baseline.prng_state);
    assert_eq!(cheated.turn, baseline.turn);
}

// ---------------------------------------------------------------------------
// The parity lock (7.4.5).

/// Every cheat asked for, plus the lock: the resolver gives none, the
/// foundation plugin and the cheats plugin are absent, Ctrl+F12 opens
/// nothing, and no marker is read or written.
#[test]
fn every_cheat_asked_for_under_the_lock_adds_nothing_and_touches_no_marker() {
    let locked = resolved(&every_cheat_asked(), LockState::Locked(LockReason::EnvVar));
    assert_eq!(locked.cheats(), CheatToggles::default());
    assert!(locked.is_classic());

    for corrupt in [false, true] {
        let dir = save_dir();
        let sidecars = dir.join(SIDECAR_DIR);
        if corrupt {
            fs::create_dir_all(&sidecars).unwrap();
            fs::write(meta_path(&dir), b"not a sidecar").unwrap();
        }
        let mut game = crate::enh::overlay::tests::Game::new(false);
        game.visual_mut().game_dir = dir.clone();
        let launch = VisualLaunchConfig {
            save_dir: Some(dir.clone()),
            ..launch_of(locked.clone())
        };
        add_enhancements(&mut game.app, &launch);
        game.frame();
        assert!(!game.app.is_plugin_added::<EnhancementsPlugin>());
        assert!(!game.app.is_plugin_added::<CheatsPlugin>());
        assert!(!has_system(&mut game.app, "cheat_marker"));
        assert!(!has_system(&mut game.app, "cheat_apply"));
        ctrl_f12(&mut game);
        assert!(!game.app.world().contains_resource::<CheatPage>());
        assert!(!game.app.world().contains_resource::<OverlayState>());
        game.visual_mut().state.gold = 2;
        q_y_save(&mut game);
        if corrupt {
            assert_eq!(fs::read(meta_path(&dir)).unwrap(), b"not a sidecar");
        } else {
            assert!(!sidecars.exists(), "the sidecar folder is never made");
        }
        let _ = fs::remove_dir_all(dir);
    }
}
