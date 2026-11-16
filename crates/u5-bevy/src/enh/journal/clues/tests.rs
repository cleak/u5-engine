//! The clue log's Bevy wiring (design 8.6: T3 files, T3b resources and
//! systems, T6 purity, T9 shell cases; build plan M6-02). Every blob is
//! synthetic: invented words in the `formats/tlk.md` layout.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use bevy::ecs::system::RunSystemOnce;
use u5_runtime::enhancements::clues::{RecordAddr, decode_payload};
use u5_runtime::enhancements::{
    LockReason, LockState, SIDECAR_DIR, SaveStamp, decode_container, encode_container,
    trajectory_digest,
};
use u5_runtime::test_fixtures::{debug_game_dir, saved_game_seed_bytes, write_empty_ool_mirrors};
use u5_runtime::{Direction, NpcSlot, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, SAVED_OOL_LEN};

use super::*;
use crate::enh::overlay::tests::Game;
use crate::enh::sidecar_io::install_save_watcher;
use crate::enh::{add_enhancements, configure_enh_sets};

const DIALOG_ID: u8 = 0x10;

fn toggles(clues: bool, highlight: bool) -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    toggles.set(CLUES, clues);
    toggles.set(HIGHLIGHT, highlight);
    toggles
}

fn launch(toggles: FeatureToggles, save_dir: Option<&Path>) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles = toggles;
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

fn journal_file(dir: &Path) -> PathBuf {
    dir.join(SIDECAR_DIR).join("journal.u5e")
}

fn saved_stamp(dir: &Path) -> SaveStamp {
    SaveStamp::of_image(&fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap())
}

/// Bit 7 set, as the file stores text.
fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

/// An invented person: the five leading records and two pairs.
fn fields() -> Vec<Vec<u8>> {
    vec![
        text("Quorvel"),
        text("a dusty quillwright"),
        text("Hail."),
        text("I mend quills."),
        text("Fare well."),
        text("gran"),
        text("Old one."),
        text("mill"),
        text("Wheels turn."),
    ]
}

fn decoded(fields: &[Vec<u8>]) -> Vec<String> {
    fields
        .iter()
        .map(|field| field.iter().map(|b| char::from(b & 0x7F)).collect())
        .collect()
}

/// The gameplay driver on a synthetic town whose saves go to `dir`, with
/// the foundation's sets, the save watcher and the clue plugin as the
/// enhancements plugin would add them.
fn clue_game(dir: &Path, toggles: FeatureToggles) -> Game {
    let mut game = Game::new(false);
    game.visual_mut().game_dir = dir.to_path_buf();
    {
        let state = &mut game.visual_mut().state;
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: DIALOG_ID,
                schedule: [0, 0, 0, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
    }
    configure_enh_sets(&mut game.app);
    let launch = launch(toggles, Some(dir));
    game.app
        .insert_resource(Toggles(toggles))
        .insert_resource(EnhLaunch(launch.clone()));
    install_save_watcher(&mut game.app, dir.to_path_buf(), toggles);
    add_clue_log(&mut game.app, &launch);
    game.frame();
    game
}

/// Opens the conversation in place, as Talk would, then lets a frame pass.
fn open_talk(game: &mut Game) {
    let fields = fields();
    let dialogue = HashMap::from([(u16::from(DIALOG_ID), decoded(&fields))]);
    let raw = HashMap::from([(u16::from(DIALOG_ID), fields)]);
    let opened = game
        .visual_mut()
        .state
        .open_conversation_session(&dialogue, &raw);
    assert!(opened.is_some());
    game.frame();
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

fn letter(c: char) -> KeyCode {
    match c {
        'a' => KeyCode::KeyA,
        'b' => KeyCode::KeyB,
        'd' => KeyCode::KeyD,
        'e' => KeyCode::KeyE,
        'g' => KeyCode::KeyG,
        'i' => KeyCode::KeyI,
        'j' => KeyCode::KeyJ,
        'l' => KeyCode::KeyL,
        'm' => KeyCode::KeyM,
        'n' => KeyCode::KeyN,
        'o' => KeyCode::KeyO,
        'r' => KeyCode::KeyR,
        'y' => KeyCode::KeyY,
        other => panic!("no key for {other:?}"),
    }
}

/// Types `line` and Enter at the keyword prompt, then a few idle frames.
fn say(game: &mut Game, line: &str) {
    for c in line.chars() {
        tap(game, letter(c));
    }
    tap(game, KeyCode::Enter);
    for _ in 0..3 {
        game.frame();
    }
}

fn save(game: &mut Game) {
    shifted_tap(game, KeyCode::KeyQ);
    shifted_tap(game, KeyCode::KeyY);
    for _ in 0..3 {
        game.frame();
    }
}

fn logged(game: &Game) -> Vec<RecordAddr> {
    game.app
        .world()
        .resource::<ClueLogResource>()
        .log()
        .entries()
        .iter()
        .map(|entry| entry.record)
        .collect()
}

fn clue_systems(app: &App) -> Vec<String> {
    let mut names: Vec<String> = app
        .world()
        .resource::<bevy::ecs::schedule::Schedules>()
        .iter()
        .flat_map(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .map(|(_, system, _)| system.name().to_string())
                .filter(|name| name.contains("journal::clues"))
                .collect::<Vec<_>>()
        })
        .collect();
    names.sort();
    names
}

fn has<R: Resource>(game: &Game) -> bool {
    game.app.world().contains_resource::<R>()
}

// ---------------------------------------------------------------------------
// T3b: resources and systems absent per key.

#[test]
fn t3b_classic_the_lock_and_other_features_add_nothing() {
    let mut gem = FeatureToggles::CLASSIC;
    gem.set(Toggle::GemMapEnhanced, true);
    let mut locked = launch(toggles(true, true), None);
    locked.enhancements.lock = LockState::Locked(LockReason::CliFlag);
    for launch in [VisualLaunchConfig::classic(), locked, launch(gem, None)] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(clue_systems(&app).is_empty(), "{launch:?}");
        let world = app.world();
        assert!(!world.contains_resource::<ClueObserver>());
        assert!(!world.contains_resource::<ClueLogResource>());
        assert!(!world.contains_resource::<TlkLiveCache>());
    }
}

#[test]
fn t3b_each_key_brings_only_its_own_resources() {
    let dir = save_dir();
    let highlight = clue_game(&dir, toggles(false, true));
    assert!(has::<ClueObserver>(&highlight));
    assert!(!has::<ClueLogResource>(&highlight), "session-only: no log");
    assert!(!has::<TlkLiveCache>(&highlight));
    let mut app = App::new();
    add_enhancements(&mut app, &launch(toggles(false, true), None));
    assert!(
        !clue_systems(&app).is_empty(),
        "highlighting alone runs the observer"
    );

    let clues = clue_game(&dir, toggles(true, false));
    assert!(has::<ClueObserver>(&clues));
    assert!(has::<ClueLogResource>(&clues));
    assert!(has::<TlkLiveCache>(&clues));
}

#[test]
fn t3b_turning_keys_off_in_play_removes_their_resources() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, true));
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(false, true);
    game.frame();
    assert!(!has::<ClueLogResource>(&game));
    assert!(!has::<TlkLiveCache>(&game));
    assert!(has::<ClueObserver>(&game));
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(false, false);
    game.frame();
    assert!(!has::<ClueObserver>(&game));
    // And on again: the log comes back and reads the save's file.
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(true, false);
    game.frame();
    assert!(has::<ClueObserver>(&game));
    assert!(has::<ClueLogResource>(&game));
}

// ---------------------------------------------------------------------------
// Capture through the real input path.

#[test]
fn a_conversation_is_logged_through_the_game_keys() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "job");
    say(&mut game, "gran");
    say(&mut game, "name");
    say(&mut game, "gran");
    assert_eq!(
        logged(&game),
        [
            RecordAddr::Leading(3),
            RecordAddr::Pair(0),
            RecordAddr::Leading(0)
        ]
    );
    let log = game.app.world().resource::<ClueLogResource>().log();
    assert_eq!(log.entries()[1].count, 2, "a repeat folds");
    assert_eq!(log.unsaved(), 4);
    say(&mut game, "");
    assert!(game.visual().state.active_conversation.is_none());
    assert!(
        !game
            .app
            .world()
            .resource::<ClueObserver>()
            .in_conversation()
    );
}

#[test]
fn session_only_mode_keeps_an_asked_set_for_the_conversation() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(false, true));
    open_talk(&mut game);
    say(&mut game, "mill");
    let asked: Vec<RecordAddr> = game
        .app
        .world()
        .resource::<ClueObserver>()
        .asked()
        .iter()
        .map(|(_, _, record)| *record)
        .collect();
    assert_eq!(asked, [RecordAddr::Pair(1)]);
    say(&mut game, "");
    assert!(
        game.app
            .world()
            .resource::<ClueObserver>()
            .asked()
            .is_empty(),
        "dropped when the conversation ends"
    );
}

#[test]
fn keys_typed_for_the_player_are_not_captured() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    crate::enh::synthetic::install_synthetic_input(&mut game.app);
    open_talk(&mut game);
    game.app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .take(crate::enh::synthetic::SyntheticOwner::StartKeys)
        .unwrap();
    say(&mut game, "job");
    assert!(logged(&game).is_empty(), "a replay is not logged (D8.10)");
    game.app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .release(crate::enh::synthetic::SyntheticOwner::StartKeys);
    say(&mut game, "gran");
    assert_eq!(logged(&game), [RecordAddr::Pair(0)]);
}

#[test]
fn the_tlk_cache_reads_a_family_once_from_the_game_directory() {
    let dir = debug_game_dir();
    // A one-blob `.TLK`: the entry count, one (id, offset) row, the blob.
    let mut file = vec![1, 0, DIALOG_ID, 0, 6, 0];
    for field in fields() {
        file.extend(field);
        file.push(0);
    }
    fs::write(dir.join(u5_runtime::npc_tlk_filename(17).unwrap()), file).unwrap();
    let mut cache = TlkLiveCache::default();
    let castle = DialogKey {
        family: 2,
        dialog_id: DIALOG_ID,
    };
    assert_eq!(cache.blob(&dir, castle).map(<[_]>::len), Some(9));
    assert_eq!(cache.blob(&dir, castle).unwrap()[5], text("gran"));
    let towne = DialogKey {
        family: 0,
        dialog_id: 1,
    };
    assert!(cache.blob(&dir, towne).is_none(), "no file: nothing");
    assert_eq!(cache.families_read(), 2);
}

// ---------------------------------------------------------------------------
// T3: files.

#[test]
fn t3_clues_off_never_touches_the_file_even_with_highlighting() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(false, true));
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    save(&mut game);
    assert_ne!(
        saved_stamp(&dir).fnv,
        SaveStamp::of_image(&saved_game_seed_bytes(0, 0, 5, 5)).fnv,
        "the game saved"
    );
    assert_eq!(io_probe::touches(&dir), 0, "no open, stat or write");
    assert!(!dir.join(SIDECAR_DIR).exists());
}

#[test]
fn t3_writes_only_on_a_new_save_stamp() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    let reads = io_probe::touches(&dir);
    assert_eq!(reads, 1, "read once at gameplay start");
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    for _ in 0..5 {
        game.frame();
    }
    assert!(!journal_file(&dir).exists(), "captures stay in RAM");
    assert_eq!(io_probe::touches(&dir), reads);
    save(&mut game);
    let bytes = fs::read(journal_file(&dir)).unwrap();
    let container = decode_container(&bytes).unwrap();
    assert_eq!(container.stamp, saved_stamp(&dir), "stamped with the save");
    let saved = decode_payload(&container.payload).unwrap();
    assert_eq!(saved.entries().len(), 1);
    let log = game.app.world().resource::<ClueLogResource>().log();
    assert_eq!(log.unsaved(), 0);
    let touches = io_probe::touches(&dir);
    for _ in 0..10 {
        game.frame();
    }
    assert_eq!(io_probe::touches(&dir), touches, "no stamp, no write");
}

#[test]
fn t3_turning_clues_off_writes_nothing() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    let touches = io_probe::touches(&dir);
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(false, false);
    game.frame();
    save(&mut game);
    assert_eq!(io_probe::touches(&dir), touches);
    assert!(!journal_file(&dir).exists());
}

// ---------------------------------------------------------------------------
// T9: the shell's reconcile cases.

#[test]
fn t9_a_slot_copy_restores_its_own_log() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    save(&mut game);
    // The menu's slot copy: the save and its sidecar folder, as opaque
    // bytes.
    let slot = debug_game_dir();
    fs::copy(dir.join(SAVED_GAM_FILENAME), slot.join(SAVED_GAM_FILENAME)).unwrap();
    fs::create_dir_all(slot.join(SIDECAR_DIR)).unwrap();
    fs::copy(journal_file(&dir), journal_file(&slot)).unwrap();

    // More captures and a later save in play.
    open_talk(&mut game);
    say(&mut game, "mill");
    say(&mut game, "");
    save(&mut game);
    assert_eq!(logged(&game), [RecordAddr::Pair(0), RecordAddr::Pair(1)]);

    // Restore the slot over the save folder, as a slot load does.
    fs::copy(slot.join(SAVED_GAM_FILENAME), dir.join(SAVED_GAM_FILENAME)).unwrap();
    fs::copy(journal_file(&slot), journal_file(&dir)).unwrap();
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .request_read();
    game.app
        .world_mut()
        .resource_mut::<SaveWatch>()
        .request_rewrite_poll();
    game.frame();
    assert_eq!(logged(&game), [RecordAddr::Pair(0)], "the slot's own log");
}

#[test]
fn t9_unsaved_captures_are_dropped_on_a_reread_and_on_turn_off() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    save(&mut game);
    open_talk(&mut game);
    say(&mut game, "mill");
    say(&mut game, "");
    assert_eq!(logged(&game).len(), 2);
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .request_read();
    game.frame();
    assert_eq!(
        logged(&game),
        [RecordAddr::Pair(0)],
        "re-read: unsaved gone"
    );

    open_talk(&mut game);
    say(&mut game, "mill");
    say(&mut game, "");
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(false, false);
    game.frame();
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(true, false);
    game.frame();
    assert_eq!(
        logged(&game),
        [RecordAddr::Pair(0)],
        "turn-off: unsaved gone"
    );
}

#[test]
fn t9_a_new_character_resets_the_log_and_its_file() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    save(&mut game);
    assert!(journal_file(&dir).exists());
    let stamp = saved_stamp(&dir);
    game.app.world_mut().send_event(SaveStamped {
        stamp,
        new_game: true,
    });
    game.frame();
    assert!(logged(&game).is_empty());
    assert!(!journal_file(&dir).exists());
}

#[test]
fn t9_an_unreadable_payload_is_renamed_bad_and_the_log_starts_empty() {
    let dir = save_dir();
    fs::create_dir_all(dir.join(SIDECAR_DIR)).unwrap();
    let garbage = encode_container(TAG_JOURNAL, &saved_stamp(&dir), 1, &[9, 9, 9]).unwrap();
    fs::write(journal_file(&dir), garbage).unwrap();
    let game = clue_game(&dir, toggles(true, false));
    assert!(logged(&game).is_empty());
    assert!(!journal_file(&dir).exists());
    let mut bad = journal_file(&dir).into_os_string();
    bad.push(".bad");
    assert!(Path::new(&bad).exists());
}

// ---------------------------------------------------------------------------
// T6: purity.

#[test]
fn t6_observer_systems_leave_the_game_state_alone() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "gran");
    // A frame's worth of keys already in the state: type one more line
    // through the game, then run the observer's systems by hand around it.
    let world = game.app.world_mut();
    let before = trajectory_digest(&world.resource::<VisualState>().state);
    world.run_system_once(clue_observe_pre).unwrap();
    assert_eq!(
        trajectory_digest(&world.resource::<VisualState>().state),
        before
    );
    world
        .resource_mut::<VisualState>()
        .state
        .submit_active_conversation_keyword("mill");
    let mid = trajectory_digest(&world.resource::<VisualState>().state);
    world.run_system_once(clue_observe_post).unwrap();
    assert_eq!(
        trajectory_digest(&world.resource::<VisualState>().state),
        mid
    );
    world.run_system_once(clue_read).unwrap();
    assert_eq!(
        trajectory_digest(&world.resource::<VisualState>().state),
        mid
    );
}

#[test]
fn t6_source_scan_finds_no_writer_of_the_game() {
    let forbidden = [
        format!("&mut {}", "PlayState"),
        format!("ResMut<{}>", "VisualState"),
        format!("run_tlk_{}", "stream_from"),
    ];
    // Every source file of `enh/journal/` and `enhancements/clues/`.
    let sources: [(&str, &str); 14] = [
        ("enh/journal.rs", include_str!("../../journal.rs")),
        ("enh/journal/clues.rs", include_str!("../clues.rs")),
        (
            "enh/journal/scrollback.rs",
            include_str!("../scrollback.rs"),
        ),
        (
            "clues/text.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/text.rs"),
        ),
        (
            "clues/scrollback.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/scrollback.rs"),
        ),
        ("enh/journal/clues/tests.rs", include_str!("tests.rs")),
        (
            "clues/mod.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/mod.rs"),
        ),
        (
            "clues/attribute.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/attribute.rs"),
        ),
        (
            "clues/attribute_tests.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/attribute_tests.rs"),
        ),
        (
            "clues/codec.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/codec.rs"),
        ),
        (
            "clues/dependence.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/dependence.rs"),
        ),
        (
            "clues/fixtures.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/fixtures.rs"),
        ),
        (
            "clues/reconcile.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/reconcile.rs"),
        ),
        (
            "clues/tests.rs",
            include_str!("../../../../../u5-runtime/src/enhancements/clues/tests.rs"),
        ),
    ];
    for (name, source) in sources {
        for pattern in &forbidden {
            assert!(!source.contains(pattern.as_str()), "{name} has {pattern}");
        }
    }
}

// ---------------------------------------------------------------------------
// M6-04: the toast, misses, the session's text and TEXT after `kept`.

fn with(mut toggles: FeatureToggles, extra: &[Toggle]) -> FeatureToggles {
    for toggle in extra {
        toggles.set(*toggle, true);
    }
    toggles
}

fn toast(game: &Game) -> Option<String> {
    game.app
        .world()
        .resource::<Toasts>()
        .current()
        .map(str::to_owned)
}

fn file_log(dir: &Path) -> ClueLog {
    let bytes = fs::read(journal_file(dir)).unwrap();
    decode_payload(&decode_container(&bytes).unwrap().payload).unwrap()
}

#[test]
fn m6_04_one_toast_per_close_of_a_conversation_that_logged() {
    let dir = save_dir();
    let mut game = clue_game(
        &dir,
        with(toggles(true, false), &[Toggle::JournalClueToast]),
    );
    game.app.insert_resource(Toasts::default());
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "mill");
    assert_eq!(toast(&game), None, "nothing while the conversation runs");
    say(&mut game, "");
    assert!(game.visual().state.active_conversation.is_none());
    assert_eq!(
        toast(&game).as_deref(),
        Some(CLUE_TOAST),
        "one toast at the close"
    );
    game.app.insert_resource(Toasts::default());
    for _ in 0..5 {
        game.frame();
    }
    assert_eq!(toast(&game), None, "once per close");
    // A conversation that logged nothing says nothing.
    open_talk(&mut game);
    say(&mut game, "");
    assert_eq!(toast(&game), None);
    // And with the key off, no toast at all.
    let mut quiet = clue_game(&dir, toggles(true, false));
    quiet.app.insert_resource(Toasts::default());
    open_talk(&mut quiet);
    say(&mut quiet, "gran");
    say(&mut quiet, "");
    assert_eq!(toast(&quiet), None);
}

#[test]
fn m6_04_misses_are_kept_only_with_the_key_and_never_in_the_file_otherwise() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    open_talk(&mut game);
    say(&mut game, "bole");
    say(&mut game, "");
    let log = game.app.world().resource::<ClueLogResource>().log();
    assert!(log.misses().is_empty(), "no miss kept with the key off");
    assert!(log.entries().is_empty());
    save(&mut game);
    assert!(file_log(&dir).misses().is_empty());

    let dir = save_dir();
    let mut game = clue_game(
        &dir,
        with(toggles(true, false), &[Toggle::JournalClueMisses]),
    );
    open_talk(&mut game);
    say(&mut game, "bole");
    say(&mut game, "gran");
    say(&mut game, "");
    let log = game.app.world().resource::<ClueLogResource>().log();
    assert_eq!(log.misses().len(), 1);
    assert_eq!(log.misses()[0].typed, b"BOLE");
    assert_eq!(logged(&game), [RecordAddr::Pair(0)], "a miss is not a clue");
    save(&mut game);
    assert_eq!(file_log(&dir).misses().len(), 1, "MISSES with the key on");
    // Turning the key off: the next save leaves MISSES out.
    game.app.world_mut().resource_mut::<Toggles>().0 = toggles(true, false);
    game.frame();
    open_talk(&mut game);
    say(&mut game, "mill");
    say(&mut game, "");
    // A different save, so a new stamp.
    game.visual_mut().state.gold += 1;
    save(&mut game);
    assert!(file_log(&dir).misses().is_empty());
}

#[test]
fn m6_04_live_text_keeps_the_rows_seen_in_ram_only() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .set_text(ClueText::Live);
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    let log = game.app.world().resource::<ClueLogResource>();
    let entry = log.log().entries()[0];
    let rows = log
        .session_text()
        .rows(&entry)
        .expect("rows seen this session");
    let seen: Vec<String> = rows
        .iter()
        .map(|row| row.bytes.iter().map(|b| char::from(*b)).collect())
        .collect();
    assert!(seen.iter().any(|row| row.contains("Old one.")), "{seen:?}");
    save(&mut game);
    let bytes = fs::read(journal_file(&dir)).unwrap();
    assert!(
        !bytes.windows(8).any(|w| w == b"Old one."),
        "live text is never written"
    );
    assert!(file_log(&dir).text().is_empty());
    // Off drops the session's rows.
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .set_text(ClueText::Off);
    assert!(
        game.app
            .world()
            .resource::<ClueLogResource>()
            .session_text()
            .is_empty()
    );
}

#[test]
fn t9_shell_text_is_dropped_at_the_save_after_leaving_kept() {
    let dir = save_dir();
    let mut game = clue_game(&dir, toggles(true, false));
    // `kept` is not offered by the resolver until D8.1; the plumbing is
    // driven directly here.
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .set_text(ClueText::Kept);
    open_talk(&mut game);
    say(&mut game, "gran");
    say(&mut game, "");
    save(&mut game);
    let kept = file_log(&dir);
    assert_eq!(kept.text().len(), 1, "TEXT written under kept");
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .set_text(ClueText::Live);
    open_talk(&mut game);
    say(&mut game, "mill");
    say(&mut game, "");
    game.visual_mut().state.gold += 1;
    save(&mut game);
    let after = file_log(&dir);
    assert_eq!(after.entries().len(), 2, "the second save was written");
    assert!(after.text().is_empty(), "TEXT dropped at the next save");
    assert!(after.entries().iter().all(|entry| {
        !entry
            .flags
            .has(u5_runtime::enhancements::clues::EntryFlags::HAS_TEXT)
    }));
    assert!(
        game.app
            .world()
            .resource::<ClueLogResource>()
            .log()
            .text()
            .is_empty()
    );
}
