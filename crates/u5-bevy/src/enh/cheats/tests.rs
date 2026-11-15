//! The cheats plugin through the real input driver and idle pump (design
//! 7.5 Phase 2 tests; build plan M8-02).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use bevy::ecs::schedule::Schedules;
use u5_runtime::enhancements::{
    FeatureToggles, LockReason, LockState, SaveMeta, SaveStamp, SidecarStore, TAG_SAVE_META,
    Toggle, encode_container,
};
use u5_runtime::test_fixtures::{debug_game_dir, saved_game_seed_bytes, write_empty_ool_mirrors};
use u5_runtime::z_stats::{YesNoPromptKind, YesNoPromptSession};
use u5_runtime::{
    COMBAT_ACTOR_FLAG_SELECTABLE_80, CombatActorDescriptor, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME,
    SAVED_OOL_LEN,
};

use super::*;
use crate::enh::input::{EnhAction, KeyMap};
use crate::enh::overlay::tests::Game;
use crate::enh::overlay::{CloseReason, OverlayChanged, OverlayRefused};
use crate::enh::toast::{ShellBadges, Toasts};
use crate::enh::{EnhancementsPlugin, PendingToggleChanges, add_enhancements};
use crate::{add_visual_intro_update_systems, add_visual_update_systems};

/// The switch and every cheat of M8-02 (quick actions and locks).
fn every_cheat() -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    for spec in u5_runtime::enhancements::TOGGLES {
        let key = spec.toggle.key();
        if key.starts_with("cheats.")
            && !matches!(
                spec.toggle,
                Toggle::CheatsStatEdit | Toggle::CheatsTeleport | Toggle::CheatsStateWatch
            )
        {
            toggles.set(spec.toggle, true);
        }
    }
    toggles
}

fn launch_with(toggles: FeatureToggles) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles = toggles;
    launch
}

fn only(list: &[Toggle]) -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    for toggle in list {
        toggles.set(*toggle, true);
    }
    toggles
}

/// The synthetic town at its idle prompt with the enhancements of
/// `toggles`, as the real builders add them.
fn game(toggles: FeatureToggles, save_dir: Option<&Path>) -> Game {
    let mut game = Game::new(false);
    let mut launch = launch_with(toggles);
    if let Some(dir) = save_dir {
        game.visual_mut().game_dir = dir.to_path_buf();
        launch.save_dir = Some(dir.to_path_buf());
    }
    add_enhancements(&mut game.app, &launch);
    game.frame();
    game.frame();
    game
}

fn ctrl_f12(game: &mut Game) {
    game.press(KeyCode::ControlLeft);
    game.press(KeyCode::F12);
    game.frame();
    game.release(KeyCode::F12);
    game.release(KeyCode::ControlLeft);
    game.frame();
}

fn tap(game: &mut Game, key: KeyCode) {
    game.press(key);
    game.frame();
    game.release(key);
    game.frame();
}

fn open_page(game: &mut Game) {
    ctrl_f12(game);
    assert_eq!(game.open(), Some(OverlayId::Cheats), "the page opens");
}

fn session(game: &Game) -> &CheatSession {
    game.app.world().resource::<CheatSession>()
}

fn page(game: &Game) -> &CheatPage {
    game.app.world().resource::<CheatPage>()
}

fn has_system(app: &mut App, name: &str) -> bool {
    app.world_mut()
        .resource_mut::<Schedules>()
        .iter()
        .any(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains(name))
        })
}

fn wound(game: &mut Game) {
    for member in &mut game.visual_mut().state.party {
        member.status = b'G';
        member.max_hp = 100;
        member.hp = 10;
    }
}

fn hp(game: &Game) -> Vec<u16> {
    game.visual().state.party.iter().map(|m| m.hp).collect()
}

// ---------------------------------------------------------------------------
// Zero footprint.

#[test]
fn no_plugin_with_cheats_off_locked_or_only_sub_options() {
    let freeze = only(&[Toggle::EnhancementsFreezeOverlays]);
    let mut locked = launch_with(every_cheat());
    locked.enhancements.lock = LockState::Locked(LockReason::QaEnv);
    let sub_options = only(&[
        Toggle::CheatsEnabled,
        Toggle::CheatsRefillQuarrels,
        Toggle::CheatsRememberLocks,
    ]);
    for launch in [
        VisualLaunchConfig::classic(),
        launch_with(freeze),
        launch_with(only(&[Toggle::CheatsHealAll])),
        launch_with(sub_options),
        locked,
    ] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.is_plugin_added::<CheatsPlugin>(), "{launch:?}");
        assert!(!has_system(&mut app, "cheat_apply"));
        assert!(!has_system(&mut app, "cheat_marker"));
        assert!(!app.world().contains_resource::<CheatSession>());
        assert!(!app.world().contains_resource::<CheatPage>());
        // D-M8.5: the chord is bound only while a cheat or the watch is on.
        if let Some(keymap) = app.world().get_resource::<KeyMap>() {
            assert!(keymap.iter().all(|(_, a)| a != EnhAction::OpenCheats));
        }
    }
    assert!(!EnhAction::OpenCheats.enabled(sub_options));
    assert!(!page_available(sub_options));
}

#[test]
fn the_plugin_and_its_resources_follow_the_toggles() {
    let mut game = game(only(&[Toggle::CheatsEnabled, Toggle::CheatsMaxGold]), None);
    assert!(game.app.is_plugin_added::<CheatsPlugin>());
    assert!(game.app.world().contains_resource::<CheatSession>());
    assert!(game.app.world().contains_resource::<CheatPage>());

    // The watch alone: the page, but no session.
    let watch = game_with_watch_only();
    assert!(watch.app.world().contains_resource::<CheatPage>());
    assert!(!watch.app.world().contains_resource::<CheatSession>());

    // The last cheat switched off at runtime removes both.
    game.app
        .insert_resource(Toggles(only(&[Toggle::CheatsEnabled])));
    game.frame();
    assert!(!game.app.world().contains_resource::<CheatSession>());
    assert!(!game.app.world().contains_resource::<CheatPage>());
}

fn game_with_watch_only() -> Game {
    game(only(&[Toggle::CheatsStateWatch]), None)
}

/// With every cheat off the base game's schedules are the 1.0.1 ones and
/// Ctrl+F12 is inert: the game, its queue and its frame are those of a run
/// without the chord.
#[test]
fn ctrl_f12_is_inert_with_cheats_off() {
    let sub_options = only(&[Toggle::CheatsEnabled, Toggle::CheatsRefillQuarrels]);
    for toggles in [FeatureToggles::CLASSIC, sub_options] {
        let mut quiet = game(toggles, None);
        let mut chord = game(toggles, None);
        assert_eq!(quiet.game_view(), chord.game_view());
        for frame in 0..30 {
            match frame {
                2 => chord.press(KeyCode::ControlLeft),
                3 => chord.press(KeyCode::F12),
                4 => chord.release(KeyCode::F12),
                5 => chord.release(KeyCode::ControlLeft),
                _ => {}
            }
            quiet.frame();
            chord.frame();
            assert_eq!(quiet.game_view(), chord.game_view(), "frame {frame}");
        }
        assert!(chord.visual().typed_keys.is_empty());
    }
}

#[test]
fn the_systems_sit_in_both_app_chains() {
    for intro in [false, true] {
        let mut app = App::new();
        if intro {
            add_visual_intro_update_systems(&mut app);
        } else {
            add_visual_update_systems(&mut app);
        }
        let mut launch = launch_with(every_cheat());
        launch.save_dir = Some(PathBuf::from("."));
        add_enhancements(&mut app, &launch);
        assert!(app.is_plugin_added::<EnhancementsPlugin>());
        for name in [
            "cheats_sync",
            "cheats_open",
            "page_keys",
            "cheat_apply",
            "page_present",
            "cheat_marker_attach",
            "cheat_marker",
        ] {
            assert!(has_system(&mut app, name), "{name}, intro {intro}");
        }
        // The schedule builds: no cycle between the sets and the chain.
        app.world_mut()
            .schedule_scope(Update, |world, schedule| schedule.initialize(world))
            .unwrap();
    }
}

// ---------------------------------------------------------------------------
// Opening and closing.

#[test]
fn the_page_opens_at_the_exploration_prompt_and_closes_on_esc() {
    let mut game = game(every_cheat(), None);
    open_page(&mut game);
    assert_eq!(page(&game).mode(), PageMode::Main);
    assert!(page(&game).image().is_some(), "the page drew");
    tap(&mut game, KeyCode::Escape);
    assert_eq!(game.open(), None);
    open_page(&mut game);
    ctrl_f12(&mut game);
    assert_eq!(game.open(), None, "Ctrl+F12 closes it too");
}

#[test]
fn the_page_is_refused_away_from_an_idle_prompt() {
    let refusals: [(&str, fn(&mut Game)); 3] = [
        ("yes/no prompt", |game| {
            game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
                kind: YesNoPromptKind::SaveGame,
            });
        }),
        ("idle redraw owed", |game| {
            game.visual_mut().sweep_idle_redraw_pending = true;
        }),
        ("party asleep", |game| {
            for member in &mut game.visual_mut().state.party {
                member.status = b'S';
            }
        }),
    ];
    for (label, setup) in refusals {
        let mut game = game(every_cheat(), None);
        setup(&mut game);
        game.press(KeyCode::ControlLeft);
        game.press(KeyCode::F12);
        game.frame();
        assert_eq!(game.open(), None, "{label}");
        assert_eq!(
            game.events::<OverlayRefused>(),
            [OverlayRefused(OverlayId::Cheats)],
            "{label}"
        );
        assert_eq!(
            game.app.world().resource::<Toasts>().current(),
            Some(OverlayId::Cheats.refusal())
        );
    }
}

fn enter_combat_prompt(game: &mut Game) {
    let mut visual = game.visual_mut();
    let state = &mut visual.state;
    state.combat_active = true;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.pending_combat_actor_slot = Some(0);
}

#[test]
fn the_page_opens_at_a_combat_prompt_and_closes_when_it_ends() {
    let mut game = game(every_cheat(), None);
    enter_combat_prompt(&mut game);
    game.frame();
    open_page(&mut game);
    game.visual_mut().state.pending_combat_actor_slot = None;
    game.frame();
    assert_eq!(game.open(), None);
}

#[test]
fn close_when_not_idle_closes_the_page() {
    let mut game = game(every_cheat(), None);
    open_page(&mut game);
    game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    game.frame();
    assert_eq!(game.open(), None);
    assert!(
        game.events::<OverlayChanged>()
            .contains(&OverlayChanged::Closed {
                id: OverlayId::Cheats,
                why: CloseReason::NotIdle,
            })
    );
}

/// Page keys never reach the game, and a key held over the close does not
/// type.
#[test]
fn page_keys_never_reach_typed_keys() {
    let mut game = game(every_cheat(), None);
    let start = game.where_the_avatar_is();
    open_page(&mut game);
    for key in [
        KeyCode::KeyH,
        KeyCode::KeyG,
        KeyCode::Digit3,
        KeyCode::ArrowUp,
        KeyCode::KeyQ,
        KeyCode::Space,
    ] {
        game.press(key);
        game.frame();
        assert!(game.visual().typed_keys.is_empty(), "{key:?}");
        game.release(key);
        game.frame();
    }
    let (x, y, _, _) = game.where_the_avatar_is();
    assert_eq!((x, y), (start.0, start.1));
    // Held over the close.
    game.press(KeyCode::ArrowUp);
    game.frame();
    game.press(KeyCode::Escape);
    game.frame();
    assert_eq!(game.open(), None);
    for _ in 0..30 {
        game.repeat(KeyCode::ArrowUp);
        game.frame();
        assert!(game.visual().typed_keys.is_empty());
    }
    let (x, y, _, _) = game.where_the_avatar_is();
    assert_eq!((x, y), (start.0, start.1));
}

// ---------------------------------------------------------------------------
// Applying.

#[test]
fn a_quick_action_applies_through_the_page_and_taints() {
    let mut game = game(every_cheat(), None);
    game.visual_mut().state.gold = 5;
    let turn = game.visual().state.turn;
    open_page(&mut game);
    assert_eq!(session(&game).header(), MarkerState::Clean);
    tap(&mut game, KeyCode::KeyG);
    assert_eq!(game.visual().state.gold, 9999);
    assert_eq!(page(&game).result(), Some("Gold 9999."));
    assert!(session(&game).tainted);
    assert_eq!(session(&game).header(), MarkerState::Modified);
    assert!(game.app.world().resource::<ShellBadges>().modified);
    // The idle ticks draw randomness while the page is open, as for a
    // player who stopped typing; the edit itself draws none (runtime tests).
    assert_eq!(game.visual().state.turn, turn, "no turn passes");
    // Again: nothing to change, and the line says so.
    tap(&mut game, KeyCode::KeyG);
    assert_eq!(page(&game).result(), Some("Nothing to change."));
}

#[test]
fn queued_actions_apply_in_order() {
    let mut game = game(every_cheat(), None);
    game.visual_mut().state.gold = 1;
    game.visual_mut().state.food = 1;
    game.app.world_mut().resource_mut::<CheatSession>().queued = vec![
        CheatAction::Quick(QuickAction::MaxFood),
        CheatAction::Quick(QuickAction::MaxGold),
        CheatAction::Quick(QuickAction::MaxFood),
    ];
    game.frame();
    assert_eq!(game.visual().state.gold, 9999);
    assert_eq!(game.visual().state.food, 9999);
    assert!(session(&game).queued.is_empty());
    assert_eq!(
        page(&game).result(),
        Some("Nothing to change."),
        "the last action's line: the second Max food changed nothing"
    );
}

#[test]
fn a_queued_action_waits_for_the_idle_prompt() {
    let mut game = game(every_cheat(), None);
    game.visual_mut().state.gold = 1;
    game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    game.app
        .world_mut()
        .resource_mut::<CheatSession>()
        .queued
        .push(CheatAction::Quick(QuickAction::MaxGold));
    for _ in 0..5 {
        game.frame();
    }
    assert_eq!(game.visual().state.gold, 1);
    game.visual_mut().state.active_yes_no_prompt = None;
    game.frame();
    assert_eq!(game.visual().state.gold, 9999);
}

#[test]
fn locks_apply_once_per_epoch_and_never_away_from_the_prompt() {
    let mut game = game(every_cheat(), None);
    wound(&mut game);
    open_page(&mut game);
    tap(&mut game, KeyCode::Digit1);
    assert!(session(&game).is_engaged(LockKind::Health));
    assert!(
        hp(&game).iter().all(|hp| *hp == 100),
        "checking applies at once"
    );
    assert_eq!(page(&game).result(), Some("Health lock on."));
    tap(&mut game, KeyCode::Escape);

    // Idle frames in the same turn do not apply it again.
    wound(&mut game);
    for _ in 0..10 {
        game.frame();
    }
    assert!(hp(&game).iter().all(|hp| *hp == 10));

    // Not at a modal prompt, even in a new turn.
    game.visual_mut().state.turn += 1;
    game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    for _ in 0..5 {
        game.frame();
    }
    assert!(hp(&game).iter().all(|hp| *hp == 10));

    // Once at the prompt of the new turn.
    game.visual_mut().state.active_yes_no_prompt = None;
    game.frame();
    assert!(hp(&game).iter().all(|hp| *hp == 100));
    wound(&mut game);
    game.frame();
    assert!(hp(&game).iter().all(|hp| *hp == 10));

    // A combat round is its own epoch.
    enter_combat_prompt(&mut game);
    game.frame();
    assert!(hp(&game).iter().all(|hp| *hp == 100));
    wound(&mut game);
    game.frame();
    assert!(hp(&game).iter().all(|hp| *hp == 10));
    game.visual_mut().state.combat_round_counter += 1;
    game.frame();
    assert!(hp(&game).iter().all(|hp| *hp == 100));

    // Unchecked: nothing more.
    game.visual_mut().state.combat_active = false;
    game.visual_mut().state.pending_combat_actor_slot = None;
    game.frame();
    open_page(&mut game);
    tap(&mut game, KeyCode::Digit1);
    assert_eq!(page(&game).result(), Some("Health lock off."));
    tap(&mut game, KeyCode::Escape);
    wound(&mut game);
    game.visual_mut().state.turn += 1;
    game.frame();
    assert!(hp(&game).iter().all(|hp| *hp == 10));
}

#[test]
fn one_cheat_switched_off_at_runtime_stops_while_the_others_stay_on() {
    let mut game = game(every_cheat(), None);
    {
        let mut session = game.app.world_mut().resource_mut::<CheatSession>();
        session.engaged[LockKind::Health as usize] = true;
        session.engaged[LockKind::Gold as usize] = true;
    }
    game.frame();
    let mut toggles = every_cheat();
    toggles.set(Toggle::CheatsLockHp, false);
    toggles.set(Toggle::CheatsMaxFood, false);
    game.app.insert_resource(Toggles(toggles));
    game.frame();
    wound(&mut game);
    game.visual_mut().state.gold = 3;
    game.visual_mut().state.food = 3;
    game.visual_mut().state.turn += 1;
    game.app
        .world_mut()
        .resource_mut::<CheatSession>()
        .queued
        .push(CheatAction::Quick(QuickAction::MaxFood));
    game.frame();
    assert_eq!(game.visual().state.gold, 9999, "the gold lock holds");
    assert!(
        hp(&game).iter().all(|hp| *hp == 10),
        "the health lock stopped"
    );
    assert_eq!(game.visual().state.food, 3, "Max food is off");
    assert_eq!(page(&game).result(), Some("That cheat is off."));
}

/// 7.3.5 entry check: a repaint at the idle prompt leaves the PRNG, the
/// pending queues and the transcript as they were.
#[test]
fn a_repaint_at_the_idle_prompt_changes_no_game_state() {
    let mut game = game(every_cheat(), None);
    let repaint = |game: &mut Game| {
        game.app
            .world_mut()
            .resource_scope(|world, mut images: Mut<Assets<Image>>| {
                let mut visual = world.resource_mut::<VisualState>();
                crate::repaint_visual_frame(&mut visual, &mut images, "test");
            });
    };
    // The first repaint may fill the renderer's own caches (the visibility
    // grid, the last viewport), as any natural redraw does; it draws
    // nothing and touches no queue or transcript.
    let prng = game.visual().state.prng_state;
    let message = game.visual().state.message.clone();
    let turn = game.visual().state.turn;
    repaint(&mut game);
    assert_eq!(game.visual().state.prng_state, prng);
    assert_eq!(game.visual().state.message, message);
    assert_eq!(game.visual().state.turn, turn);
    // At a settled prompt a repaint changes nothing at all.
    let before = format!("{:?}", game.visual().state);
    repaint(&mut game);
    assert_eq!(format!("{:?}", game.visual().state), before);
}

#[test]
fn the_first_person_dungeon_waits_for_the_natural_redraw() {
    let mut state = game(every_cheat(), None).visual().state.clone();
    assert!(!apply::repaint_must_wait(&state));
    state.area = u5_runtime::Area::Dungeon {
        scene: u5_runtime::DungeonScene::from_record(0).unwrap(),
        level: 0,
    };
    assert!(apply::repaint_must_wait(&state));
    state.combat_active = true;
    assert!(
        !apply::repaint_must_wait(&state),
        "dungeon combat is top-down"
    );
}

// ---------------------------------------------------------------------------
// The confirmation.

#[test]
fn allow_cheats_asks_and_y_turns_the_switch_on() {
    let mut game = game_with_watch_only();
    game.app.world_mut().send_event(AllowCheatsRequest);
    game.frame();
    assert_eq!(game.open(), Some(OverlayId::Cheats));
    assert_eq!(page(&game).mode(), PageMode::Confirm);
    tap(&mut game, KeyCode::KeyY);
    assert_eq!(game.open(), None);
    game.frame();
    assert!(
        game.app
            .world()
            .resource::<Toggles>()
            .0
            .on(Toggle::CheatsEnabled)
    );
    assert!(
        game.app
            .world()
            .resource::<PendingToggleChanges>()
            .0
            .is_empty()
    );

    let mut game = game_with_watch_only();
    game.app.world_mut().send_event(AllowCheatsRequest);
    game.frame();
    tap(&mut game, KeyCode::KeyN);
    assert_eq!(game.open(), None);
    game.frame();
    assert!(
        !game
            .app
            .world()
            .resource::<Toggles>()
            .0
            .on(Toggle::CheatsEnabled)
    );
}

// ---------------------------------------------------------------------------
// The marker.

/// A save folder with a seed save from well in the past.
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

fn shifted_tap(game: &mut Game, key: KeyCode) {
    game.press(KeyCode::ShiftLeft);
    tap(game, key);
    game.release(KeyCode::ShiftLeft);
    game.frame();
}

fn q_y_save(game: &mut Game) {
    shifted_tap(game, KeyCode::KeyQ);
    shifted_tap(game, KeyCode::KeyY);
    for _ in 0..5 {
        game.frame();
    }
}

fn meta_path(dir: &Path) -> PathBuf {
    SidecarStore::open(dir)
        .unwrap()
        .path(TAG_SAVE_META)
        .unwrap()
}

fn stamp(dir: &Path) -> SaveStamp {
    SaveStamp::of_image(&fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap())
}

#[test]
fn a_q_save_after_a_cheat_writes_a_matching_marker() {
    let dir = save_dir();
    let mut game = game(every_cheat(), Some(&dir));
    q_y_save(&mut game);
    assert!(
        !dir.join(u5_runtime::enhancements::SIDECAR_DIR).exists(),
        "an untainted save is not marked"
    );
    game.visual_mut().state.gold = 2;
    open_page(&mut game);
    tap(&mut game, KeyCode::KeyG);
    tap(&mut game, KeyCode::Escape);
    q_y_save(&mut game);
    let store = SidecarStore::open(&dir).unwrap();
    let (freshness, payload) = store.read(TAG_SAVE_META, &stamp(&dir));
    assert_eq!(freshness, u5_runtime::enhancements::Freshness::Fresh);
    assert_eq!(
        SaveMeta::decode(&payload.unwrap().bytes),
        Some(SaveMeta { cheats_used: true })
    );
    let _ = fs::remove_dir_all(dir);
}

fn write_meta(dir: &Path, stamp: &SaveStamp, used: bool) {
    let path = meta_path(dir);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = encode_container(
        TAG_SAVE_META,
        stamp,
        u5_runtime::enhancements::SAVE_META_PAYLOAD_VERSION,
        &SaveMeta { cheats_used: used }.encode(),
    )
    .unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn the_loaded_marker_gives_modified_unknown_or_clean() {
    // Fresh with the flag: tainted from the start, the badge on.
    let dir = save_dir();
    write_meta(&dir, &stamp(&dir), true);
    let game_ = game(every_cheat(), Some(&dir));
    assert_eq!(session(&game_).header(), MarkerState::Modified);
    assert!(session(&game_).tainted);
    assert!(game_.app.world().resource::<ShellBadges>().modified);
    let _ = fs::remove_dir_all(dir);

    // Another save's stamp: unknown, untainted.
    let dir = save_dir();
    let mut other = stamp(&dir);
    other.fnv ^= 1;
    write_meta(&dir, &other, true);
    let game_ = game(every_cheat(), Some(&dir));
    assert_eq!(session(&game_).header(), MarkerState::Unknown);
    assert!(!session(&game_).tainted);
    let _ = fs::remove_dir_all(dir);

    // A corrupt file: unknown, and renamed aside.
    let dir = save_dir();
    let path = meta_path(&dir);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a sidecar").unwrap();
    let game_ = game(every_cheat(), Some(&dir));
    assert_eq!(session(&game_).header(), MarkerState::Unknown);
    assert!(!path.exists());
    let _ = fs::remove_dir_all(dir);

    // None: clean.
    let dir = save_dir();
    let game_ = game(every_cheat(), Some(&dir));
    assert_eq!(session(&game_).header(), MarkerState::Clean);
    let _ = fs::remove_dir_all(dir);
}

/// With every cheat off no marker is read or written: the sidecar folder is
/// never made, a marker already there is left as it is, and no marker
/// system exists.
#[test]
fn no_marker_io_with_cheats_off() {
    for toggles in [FeatureToggles::CLASSIC, only(&[Toggle::GemMapEnhanced])] {
        let dir = save_dir();
        let mut game = game(toggles, Some(&dir));
        assert!(!has_system(&mut game.app, "cheat_marker"));
        game.visual_mut().state.gold = 2;
        q_y_save(&mut game);
        assert!(!meta_path(&dir).exists());
        let _ = fs::remove_dir_all(dir);
    }
    // A corrupt marker is not even read (it would be renamed).
    let dir = save_dir();
    let path = meta_path(&dir);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a sidecar").unwrap();
    let mut game = game(FeatureToggles::CLASSIC, Some(&dir));
    q_y_save(&mut game);
    assert_eq!(fs::read(&path).unwrap(), b"not a sidecar");
    let _ = fs::remove_dir_all(dir);
}

// ---------------------------------------------------------------------------
// Drawing.

#[test]
fn the_page_is_drawn_in_the_kit_s_colours_on_the_cell_grid() {
    use crate::EGA_PALETTE_RGB;
    let toggles = every_cheat();
    let cheats = cheat_toggles(toggles);
    let font = u5_runtime::parse_ch_font(&vec![0xff; u5_runtime::CH_FONT_LEN], "IBM.CH").unwrap();
    let mut session = CheatSession::default();
    session.engaged[0] = true;
    let view = page::page_view_for_tests((320, 200), &cheats, &session);
    let img = page::draw_page(&view, &font);
    assert_eq!((img.w, img.h), (320, 200));
    let at = |x: u32, y: u32| {
        let i = ((y * img.w + x) * 4) as usize;
        [
            img.rgba[i],
            img.rgba[i + 1],
            img.rgba[i + 2],
            img.rgba[i + 3],
        ]
    };
    let ega = |i: usize| {
        let [r, g, b] = EGA_PALETTE_RGB[i];
        [r, g, b, 0xff]
    };
    // The rules are the game's: x 7 and 312, y 7 and 184.
    assert_eq!(at(7, 100), ega(15));
    assert_eq!(at(312, 100), ega(15));
    assert_eq!(at(3, 100), ega(1));
    assert_eq!(at(100, 184), ega(15));
    // Divider bands at rows 2 and 19: rule, blue, rule.
    assert_eq!(at(100, 16), ega(15));
    assert_eq!(at(100, 19), ega(1));
    assert_eq!(at(100, 23), ega(15));
    assert_eq!(at(100, 152), ega(15));
    // Carved corner, then the dimmed backdrop below the ring.
    assert_eq!(at(0, 0)[3], 153);
    assert_eq!(at(100, 196), [0, 0, 0, 153]);
    // Every opaque pixel is an EGA colour.
    for pixel in img.rgba.as_chunks::<4>().0 {
        if pixel[3] == 0xff {
            assert!(EGA_PALETTE_RGB.iter().any(|c| c[..] == pixel[..3]));
        }
    }
    // Disabled cheats close up: with only Max gold the first item is it.
    let gold = cheat_toggles(only(&[Toggle::CheatsEnabled, Toggle::CheatsMaxGold]));
    let view = page::page_view_for_tests((320, 200), &gold, &session);
    assert!(view.texts().contains(&"[G] Max gold".to_string()));
    assert!(!view.texts().contains(&"[H] Heal all".to_string()));
    assert!(!view.texts().iter().any(|t| t.contains("Health")));
    let all = page::page_view_for_tests((320, 200), &cheats, &session);
    assert!(all.texts().contains(&"[x] 1 Health".to_string()));
}

#[path = "faces_tests.rs"]
mod faces;
