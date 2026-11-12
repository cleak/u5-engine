//! The save watcher through the real input driver: one stamp per save, none
//! without one, and nothing at all while no sidecar feature is on (design
//! 1.3.9, plan M0-16).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use u5_runtime::enhancements::{LockState, Toggle};
use u5_runtime::test_fixtures::{debug_game_dir, saved_game_seed_bytes, write_empty_ool_mirrors};
use u5_runtime::{
    ChargenSessionResult, ChargenStats, ChargenTournamentOutcome, SAVED_GAM_FILENAME,
    SAVED_OOL_FILENAME, SAVED_OOL_LEN, ShrineVirtue, TileGraphicsDepth,
};

use super::*;
use crate::enh::overlay::tests::Game;
use crate::enh::{PendingToggleChanges, VisualLaunchConfig, add_enhancements, configure_enh_sets};
use crate::{
    IntroSubflow, IntroSubflowResult, VisualIntroPanel, VisualIntroPanelOutcome,
    outcome_writes_new_game,
};

/// Stands in for a sidecar feature until chapter 4 adds the first one: the
/// watcher runs while this toggle is on.
const T: Toggle = Toggle::EnhancementsFreezeOverlays;

fn only(toggle: Toggle) -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    toggles.set(toggle, true);
    toggles
}

/// A save folder with a seed save whose modification time is well in the
/// past.
fn save_dir() -> PathBuf {
    let dir = debug_game_dir();
    fs::write(
        dir.join(SAVED_GAM_FILENAME),
        saved_game_seed_bytes(0, 0, 5, 5),
    )
    .unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    fs::File::options()
        .write(true)
        .open(dir.join(SAVED_GAM_FILENAME))
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000))
        .unwrap();
    dir
}

fn saved_stamp(dir: &Path) -> SaveStamp {
    SaveStamp::of_image(&fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap())
}

/// Every stamp sent so far.
#[derive(Resource, Default)]
struct Seen(Vec<SaveStamped>);

fn collect(mut events: EventReader<SaveStamped>, mut seen: ResMut<Seen>) {
    seen.0.extend(events.read().copied());
}

/// The gameplay input driver on a synthetic town that saves into `dir`,
/// with the watcher installed for [`T`].
fn watched_game(dir: &Path, watch: bool) -> Game {
    let mut game = Game::new(false);
    game.visual_mut().game_dir = dir.to_path_buf();
    if watch {
        configure_enh_sets(&mut game.app);
        game.app.insert_resource(Toggles(only(T)));
        install_save_watcher(&mut game.app, dir.to_path_buf(), only(T));
        game.app
            .init_resource::<Seen>()
            .add_systems(Update, collect.after(EnhSet::Observe));
    }
    game
}

fn tap(game: &mut Game, key: KeyCode) {
    game.press(key);
    game.frame();
    game.release(key);
    game.frame();
}

fn shifted_tap(game: &mut Game, key: KeyCode) {
    game.press(KeyCode::ShiftLeft);
    tap(game, key);
    game.release(KeyCode::ShiftLeft);
    game.frame();
}

fn seen(game: &Game) -> Vec<SaveStamped> {
    game.app.world().resource::<Seen>().0.clone()
}

fn has_system(app: &mut App, name: &str) -> bool {
    app.world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter()
        .any(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains(name))
        })
}

// ---------------------------------------------------------------------------
// Zero footprint.

#[test]
fn no_watcher_while_no_sidecar_feature_is_on() {
    let dir = save_dir();
    let mut on = VisualLaunchConfig::classic();
    on.enhancements.toggles.set(T, true);
    on.save_dir = Some(dir.clone());
    for launch in [VisualLaunchConfig::classic(), on] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<SaveWatch>());
        assert!(!app.world().contains_resource::<Events<SaveStamped>>());
        assert!(!has_system(&mut app, "save_watcher"));
    }
    // The launch-time rule: a sidecar toggle and a save folder, both.
    let mut app = App::new();
    add_save_watcher(&mut app, FeatureToggles::CLASSIC, Some(&dir));
    add_save_watcher(&mut app, only(T), Some(&dir));
    assert!(!app.world().contains_resource::<SaveWatch>());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_watcher_runs_only_while_a_requesting_toggle_is_on() {
    let dir = save_dir();
    let mut app = App::new();
    configure_enh_sets(&mut app);
    install_save_watcher(&mut app, dir.clone(), only(T));
    install_save_watcher(&mut app, PathBuf::from("ignored"), FeatureToggles::CLASSIC);
    assert!(has_system(&mut app, "save_watcher"));
    let watch = app.world().resource::<SaveWatch>();
    assert_eq!(watch.save_dir(), dir, "the first install's folder stays");
    assert!(watch.wanted(only(T)));
    assert!(!watch.wanted(FeatureToggles::CLASSIC));
    // Chapter 10's `gem_map.enhanced` is the first sidecar toggle with a
    // catalogue row (M3-01); it is hidden, so no launch resolves it on.
    assert_eq!(
        sidecar_toggles().iter_on().collect::<Vec<_>>(),
        [Toggle::GemMapEnhanced]
    );
    assert!(!Toggle::GemMapEnhanced.spec().is_shipped());
    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Saves in play.

#[test]
fn a_q_y_save_through_the_prompt_gives_one_stamp() {
    let dir = save_dir();
    let mut game = watched_game(&dir, true);
    game.frame();
    assert_eq!(
        game.app.world().resource::<SaveWatch>().last_stamp(),
        Some(saved_stamp(&dir)),
        "gameplay start knows the save on disk"
    );
    tap(&mut game, KeyCode::KeyD);
    shifted_tap(&mut game, KeyCode::KeyQ);
    assert!(seen(&game).is_empty(), "an open prompt is not a save");
    shifted_tap(&mut game, KeyCode::KeyY);
    for _ in 0..5 {
        game.frame();
    }
    assert_eq!(
        seen(&game),
        [SaveStamped {
            stamp: saved_stamp(&dir),
            new_game: false
        }]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_buffered_q_y_inside_one_frame_gives_one_stamp() {
    let dir = save_dir();
    let mut game = watched_game(&dir, true);
    // The game's Buffer setting on, so the Y typed with the Q is kept.
    game.visual_mut().state.typeahead_buffer_enabled = true;
    game.frame();
    game.press(KeyCode::ShiftLeft);
    game.press(KeyCode::KeyQ);
    game.press(KeyCode::KeyY);
    for _ in 0..5 {
        game.frame();
    }
    assert_eq!(
        seen(&game),
        [SaveStamped {
            stamp: saved_stamp(&dir),
            new_game: false
        }]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn no_stamp_without_a_save() {
    let dir = save_dir();
    let mut game = watched_game(&dir, true);
    for key in [KeyCode::KeyD, KeyCode::KeyA, KeyCode::Space] {
        tap(&mut game, key);
    }
    shifted_tap(&mut game, KeyCode::KeyQ);
    shifted_tap(&mut game, KeyCode::KeyN);
    for _ in 0..10 {
        game.frame();
    }
    assert!(seen(&game).is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_save_bytes_are_the_same_with_and_without_the_watcher() {
    let play = |watch: bool| {
        let dir = save_dir();
        let mut game = watched_game(&dir, watch);
        tap(&mut game, KeyCode::KeyD);
        shifted_tap(&mut game, KeyCode::KeyQ);
        shifted_tap(&mut game, KeyCode::KeyY);
        let bytes = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        let turn = game.visual().state.turn;
        let _ = fs::remove_dir_all(dir);
        (bytes, turn)
    };
    assert_eq!(play(true), play(false));
}

#[test]
fn a_slot_load_gives_one_stamp() {
    let dir = save_dir();
    let mut game = watched_game(&dir, true);
    game.frame();
    let slot = saved_game_seed_bytes(0, 0, 9, 9);
    fs::write(dir.join(SAVED_GAM_FILENAME), &slot).unwrap();
    game.app
        .world_mut()
        .resource_mut::<SaveWatch>()
        .request_rewrite_poll();
    for _ in 0..3 {
        game.frame();
    }
    assert_eq!(
        seen(&game),
        [SaveStamped {
            stamp: SaveStamp::of_image(&slot),
            new_game: false
        }]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_save_made_while_the_feature_was_off_gives_no_stamp() {
    let dir = save_dir();
    let mut game = watched_game(&dir, true);
    game.frame();
    game.app.insert_resource(Toggles(FeatureToggles::CLASSIC));
    shifted_tap(&mut game, KeyCode::KeyQ);
    shifted_tap(&mut game, KeyCode::KeyY);
    // Back on, as `apply_pending_toggles` does it.
    game.app.insert_resource(Toggles(only(T)));
    game.app
        .world_mut()
        .resource_mut::<SaveWatch>()
        .toggles_changed(FeatureToggles::CLASSIC, only(T));
    for _ in 0..3 {
        game.frame();
    }
    assert!(seen(&game).is_empty(), "{:?}", seen(&game));
    assert_eq!(
        game.app.world().resource::<SaveWatch>().last_stamp(),
        Some(saved_stamp(&dir)),
        "the save made while off is the one now known"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn apply_pending_toggles_tells_the_watcher() {
    let dir = save_dir();
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(T, true);
    launch.enhancements.lock = LockState::Off;
    launch.save_dir = Some(dir.clone());
    let mut app = App::new();
    add_enhancements(&mut app, &launch);
    install_save_watcher(&mut app, dir.clone(), only(T));
    let push = |app: &mut App, on: bool| {
        app.world_mut()
            .resource_mut::<PendingToggleChanges>()
            .0
            .push((T, on));
        app.update();
    };
    push(&mut app, false);
    assert!(!app.world().resource::<SaveWatch>().prime_requested);
    push(&mut app, true);
    assert!(
        app.world().resource::<SaveWatch>().prime_requested,
        "no gameplay here, so the request waits for it"
    );
    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// New games.

#[test]
fn a_new_game_in_the_intro_resets_and_gives_one_new_game_stamp() {
    let dir = save_dir();
    let mut app = App::new();
    configure_enh_sets(&mut app);
    app.insert_resource(Toggles(only(T)));
    install_save_watcher(&mut app, dir.clone(), only(T));
    app.init_resource::<Seen>()
        .add_systems(Update, collect.after(EnhSet::Observe));
    let intro = crate::visual_intro_suite_state(
        VisualIntroPanel::Menu,
        &dir,
        TileGraphicsDepth::Ega16,
        true,
    )
    .unwrap();
    app.insert_resource(intro);
    app.update();
    assert!(
        app.world().resource::<Seen>().0.is_empty(),
        "the menu alone"
    );

    // Character creation writes a new SAVED.GAM, even one with the very
    // bytes of the old one.
    let created = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    fs::write(dir.join(SAVED_GAM_FILENAME), &created).unwrap();
    app.world_mut()
        .resource_mut::<VisualIntroState>()
        .new_game_saves += 1;
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<Seen>().0,
        [SaveStamped {
            stamp: SaveStamp::of_image(&created),
            new_game: true
        }]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn chargen_and_the_u4_transfer_count_as_new_games() {
    let chargen = VisualIntroPanelOutcome::CommitChargen(ChargenSessionResult {
        name: *b"Avatar\0\0\0",
        entered_name: b"Avatar".to_vec(),
        male: true,
        tournament: ChargenTournamentOutcome {
            questions: Vec::new(),
            stats: ChargenStats {
                strength: 15,
                dexterity: 15,
                intelligence: 15,
            },
            final_winner: ShrineVirtue::Honesty,
        },
    });
    assert!(outcome_writes_new_game(&chargen));
    let back = |subflow, result| VisualIntroPanelOutcome::ReturnToMenu {
        subflow,
        result,
        message: String::new(),
    };
    assert!(outcome_writes_new_game(&back(
        IntroSubflow::UltimaIvTransfer,
        IntroSubflowResult::SaveReady
    )));
    assert!(!outcome_writes_new_game(&back(
        IntroSubflow::UltimaIvTransfer,
        IntroSubflowResult::Cancelled
    )));
    assert!(!outcome_writes_new_game(&back(
        IntroSubflow::ReturnToView,
        IntroSubflowResult::ReturnedToMenu
    )));
    assert!(!outcome_writes_new_game(&VisualIntroPanelOutcome::Stay));
}
