//! The gem map's Bevy wiring (plan M3-08): absent under Classic and the
//! lock, the record attached at gameplay start, written after a save and
//! only then, a new game's reset, OFF leaving an old file alone, the save
//! bytes equal on and off, and the game equal to the off run.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use u5_automap::gem::{AttachOutcome, FeatureKey, attach};
use u5_runtime::enhancements::{
    LockReason, LockState, SIDECAR_DIR, SaveStamp, SidecarStore, Toggle,
};
use u5_runtime::test_fixtures::{
    debug_game_dir, open_grid, saved_game_seed_bytes, test_state, write_empty_ool_mirrors,
};
use u5_runtime::{Area, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, SAVED_OOL_LEN};

use super::*;
use crate::enh::equivalence::{Scene, Timeline, check_shell_equivalence};
use crate::enh::overlay::tests::Game;
use crate::enh::sidecar_io::install_save_watcher;
use crate::enh::{EnhLaunch, Toggles, add_enhancements, configure_enh_sets};

fn only_gem() -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    toggles.set(GEM, true);
    toggles
}

fn gem_launch(save_dir: Option<&Path>) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles = only_gem();
    launch.save_dir = save_dir.map(Path::to_path_buf);
    launch
}

/// A save folder with a seed save whose modification time is in the past.
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

fn gem_file(dir: &Path) -> PathBuf {
    dir.join(SIDECAR_DIR).join("gemmap.u5e")
}

fn saved_stamp(dir: &Path) -> SaveStamp {
    SaveStamp::of_image(&fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap())
}

/// The gameplay input driver on a synthetic town that saves into `dir`,
/// with the foundation's sets, the save watcher and the gem map plugin as
/// [`super::super::EnhancementsPlugin`] would add them (or none of it).
fn gem_game(dir: &Path, on: bool) -> Game {
    let mut game = Game::new(false);
    game.visual_mut().game_dir = dir.to_path_buf();
    if on {
        configure_enh_sets(&mut game.app);
        game.app
            .insert_resource(Toggles(only_gem()))
            .insert_resource(EnhLaunch(gem_launch(Some(dir))));
        install_save_watcher(&mut game.app, dir.to_path_buf(), only_gem());
        add_gem_map(&mut game.app, &gem_launch(Some(dir)));
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

fn save(game: &mut Game) {
    shifted_tap(game, KeyCode::KeyQ);
    shifted_tap(game, KeyCode::KeyY);
    for _ in 0..3 {
        game.frame();
    }
}

fn gem_systems(app: &App) -> Vec<String> {
    let mut names: Vec<String> = app
        .world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .flat_map(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .map(|(_, system, _)| system.name().to_string())
                .filter(|name| name.contains("gem_map"))
                .collect::<Vec<_>>()
        })
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------------
// Zero footprint.

#[test]
fn classic_and_the_lock_schedule_no_gem_system() {
    for launch in [VisualLaunchConfig::classic(), {
        let mut locked = gem_launch(None);
        locked.enhancements.lock = LockState::Locked(LockReason::CliFlag);
        locked
    }] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(gem_systems(&app).is_empty(), "{launch:?}");
        assert!(!app.world().contains_resource::<GemMap>());
    }
    // Another feature on, the gem map off: still nothing of it.
    let mut other = VisualLaunchConfig::classic();
    other
        .enhancements
        .toggles
        .set(Toggle::EnhancementsFreezeOverlays, true);
    let mut app = App::new();
    add_enhancements(&mut app, &other);
    assert!(gem_systems(&app).is_empty());
}

#[test]
fn the_plugin_adds_its_four_systems_only_when_on() {
    let mut app = App::new();
    add_enhancements(&mut app, &gem_launch(None));
    assert!(app.is_plugin_added::<GemMapPlugin>());
    let names = gem_systems(&app);
    for system in ["gem_attach", "gem_probe_pre", "gem_probe_post"] {
        assert!(names.iter().any(|n| n.ends_with(system)), "{names:?}");
    }
    assert!(
        !names.iter().any(|n| n.ends_with("gem_persist")),
        "no save folder, no watcher, no persist"
    );

    let dir = save_dir();
    let mut app = App::new();
    add_enhancements(&mut app, &gem_launch(Some(&dir)));
    assert!(gem_systems(&app).iter().any(|n| n.ends_with("gem_persist")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn every_system_waits_for_the_toggle() {
    let dir = save_dir();
    let mut game = gem_game(&dir, true);
    game.app.insert_resource(Toggles(FeatureToggles::CLASSIC));
    for _ in 0..3 {
        game.frame();
    }
    let gem = game.app.world().resource::<GemMap>();
    assert!(gem.outcome().is_none(), "nothing attached while off");
    assert!(gem.knowledge().is_empty());
    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// The record.

#[test]
fn gameplay_start_attaches_and_learns_the_town() {
    let dir = save_dir();
    let mut game = gem_game(&dir, true);
    game.frame();
    let Area::Town { scene, .. } = game.visual().state.area else {
        panic!("a town");
    };
    let gem = game.app.world().resource::<GemMap>();
    assert!(gem.outcome().is_some());
    assert!(
        gem.knowledge()
            .is_known(&FeatureKey::Place { scene: scene.byte })
    );
    assert!(!gem_file(&dir).exists(), "nothing is written before a save");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_save_writes_the_record_stamped_with_it() {
    let dir = save_dir();
    let mut game = gem_game(&dir, true);
    game.frame();
    tap(&mut game, KeyCode::KeyD);
    assert!(!gem_file(&dir).exists());
    save(&mut game);
    assert!(gem_file(&dir).exists(), "the save wrote the record");
    let store = SidecarStore::open(&dir).unwrap();
    let (know, outcome) = attach(&store, &saved_stamp(&dir));
    assert_eq!(outcome, AttachOutcome::Restored);
    assert_eq!(&know, game.app.world().resource::<GemMap>().knowledge());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_slot_load_reads_the_older_record() {
    let dir = save_dir();
    let mut game = gem_game(&dir, true);
    game.frame();
    save(&mut game);
    let older = game.app.world().resource::<GemMap>().knowledge().clone();
    // The slot: the save and its sidecar folder, copied aside.
    let slot = dir.join("slot");
    fs::create_dir_all(slot.join(SIDECAR_DIR)).unwrap();
    fs::copy(dir.join(SAVED_GAM_FILENAME), slot.join(SAVED_GAM_FILENAME)).unwrap();
    fs::copy(gem_file(&dir), gem_file(&slot)).unwrap();

    // Play on and save again: another save, another stamp.
    let first = saved_stamp(&dir);
    tap(&mut game, KeyCode::ArrowLeft);
    save(&mut game);
    assert_ne!(saved_stamp(&dir), first);

    // Load the slot: both files come back, and the attach reads them.
    fs::copy(slot.join(SAVED_GAM_FILENAME), dir.join(SAVED_GAM_FILENAME)).unwrap();
    fs::copy(gem_file(&slot), gem_file(&dir)).unwrap();
    game.app
        .world_mut()
        .resource_mut::<GemMap>()
        .request_attach();
    game.frame();
    let gem = game.app.world().resource::<GemMap>();
    // Restored, not the newer save's record kept as stale.
    assert_eq!(gem.outcome(), Some(AttachOutcome::Restored));
    assert_eq!(gem.knowledge(), &older);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_new_game_stamp_removes_the_old_record() {
    let dir = save_dir();
    let mut game = gem_game(&dir, true);
    game.frame();
    save(&mut game);
    assert!(gem_file(&dir).exists());
    game.app.world_mut().send_event(SaveStamped {
        stamp: saved_stamp(&dir),
        new_game: true,
    });
    game.frame();
    assert!(!gem_file(&dir).exists(), "a new character starts empty");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn two_keys_in_one_frame_count_as_two_commands_and_a_modifier_as_none() {
    let dir = save_dir();
    let mut game = gem_game(&dir, true);
    game.frame();
    game.press(KeyCode::ShiftLeft);
    game.press(KeyCode::KeyQ);
    game.frame();
    assert_eq!(game.app.world().resource::<GemMap>().commands, 1);
    game.release(KeyCode::KeyQ);
    game.release(KeyCode::ShiftLeft);
    game.press(KeyCode::Escape);
    game.frame();
    game.release(KeyCode::Escape);
    game.frame();
    game.press(KeyCode::ArrowUp);
    game.press(KeyCode::ArrowLeft);
    game.frame();
    assert_eq!(
        game.app.world().resource::<GemMap>().commands,
        2,
        "the observer keeps only the unambiguous events"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_toggle_turning_on_asks_for_an_attach() {
    let mut gem = GemMap {
        attach_requested: false,
        ..GemMap::default()
    };
    gem.toggles_changed(only_gem(), only_gem());
    assert!(!gem.attach_requested);
    gem.toggles_changed(FeatureToggles::CLASSIC, only_gem());
    assert!(gem.attach_requested);
}

// ---------------------------------------------------------------------------
// OFF and the game.

#[test]
fn off_leaves_an_old_record_byte_for_byte_and_its_mtime() {
    let dir = save_dir();
    {
        let mut game = gem_game(&dir, true);
        game.frame();
        save(&mut game);
    }
    let file = gem_file(&dir);
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000))
        .unwrap();
    let bytes = fs::read(&file).unwrap();
    let mtime = fs::metadata(&file).unwrap().modified().unwrap();

    let mut game = gem_game(&dir, false);
    game.frame();
    tap(&mut game, KeyCode::KeyD);
    save(&mut game);
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), mtime);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_save_bytes_are_the_same_on_and_off() {
    let play = |on: bool| {
        let dir = save_dir();
        let mut game = gem_game(&dir, on);
        game.frame();
        tap(&mut game, KeyCode::KeyD);
        save(&mut game);
        let bytes = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        let turn = game.visual().state.turn;
        let _ = fs::remove_dir_all(dir);
        (bytes, turn)
    };
    assert_eq!(play(true), play(false));
}

#[test]
fn the_game_equals_the_off_run_frame_for_frame() {
    let mut timeline = Timeline::new(90);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::KeyL)
        .game_tap(40, KeyCode::ArrowUp)
        .game_tap(55, KeyCode::ArrowDown)
        .game_tap(70, KeyCode::KeyV);
    let start = test_state(open_grid(), 5, 5);
    let mismatches = check_shell_equivalence(
        &start,
        &Scene::synthetic(),
        &timeline,
        &gem_launch(None),
        |_| {},
    );
    assert_eq!(mismatches, []);
}
