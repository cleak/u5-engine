//! The Teleport and Watch faces through the real input driver and idle
//! pump (design 7.2.7, 7.2.8, 7.3.8, 7.6; build plan M8-05).

use u5_runtime::cheats::{TeleportArea, TeleportTarget};
use u5_runtime::enhancements::trajectory_digest;
use u5_runtime::{Area, WorldPlane};

use super::*;
use crate::enh::cheats::teleport::{LOAD_FAILED, TELEPORT_ENTRY, TELEPORTED};
use crate::enh::cheats::watch::WATCH_ENTRY;

fn with(extra: &[Toggle]) -> FeatureToggles {
    let mut toggles = every_cheat();
    for toggle in extra {
        toggles.set(*toggle, true);
    }
    toggles
}

fn main_texts(toggles: FeatureToggles) -> Vec<String> {
    page::page_view_for_tests(
        (320, 200),
        &cheat_toggles(toggles),
        &CheatSession::default(),
    )
    .texts()
}

#[test]
fn the_teleport_entry_is_absent_while_teleport_is_off() {
    let off = every_cheat();
    assert!(!main_texts(off).contains(&TELEPORT_ENTRY.to_string()));
    let mut plain = game(off, None);
    open_page(&mut plain);
    tap(&mut plain, KeyCode::KeyT);
    assert_eq!(page(&plain).mode(), PageMode::Main, "[T] does nothing");
    assert!(page(&plain).teleport().is_none());

    // The section switch off: no entry either, though the key is set.
    let mut switch_off = only(&[Toggle::CheatsTeleport]);
    switch_off.set(Toggle::CheatsStateWatch, true);
    assert!(!main_texts(switch_off).contains(&TELEPORT_ENTRY.to_string()));

    let on = with(&[Toggle::CheatsTeleport]);
    assert!(main_texts(on).contains(&TELEPORT_ENTRY.to_string()));
    let mut game = game(on, None);
    open_page(&mut game);
    tap(&mut game, KeyCode::KeyT);
    assert_eq!(page(&game).mode(), PageMode::Teleport);
    assert!(page(&game).teleport().is_some());
    tap(&mut game, KeyCode::Escape);
    assert_eq!(page(&game).mode(), PageMode::Main);
    assert_eq!(game.open(), Some(OverlayId::Cheats), "Esc goes back first");
}

#[test]
fn the_more_entries_flow_inside_the_box() {
    let all = with(&[
        Toggle::CheatsStatEdit,
        Toggle::CheatsTeleport,
        Toggle::CheatsStateWatch,
    ]);
    let texts = main_texts(all);
    for entry in ["[E] Edit party", TELEPORT_ENTRY, WATCH_ENTRY] {
        assert!(texts.contains(&entry.to_string()), "{entry}");
    }
}

#[test]
fn teleport_through_the_page_moves_the_party_without_a_turn() {
    let dir = save_dir();
    let mut game = game(with(&[Toggle::CheatsTeleport]), Some(&dir));
    let turn = game.visual().state.turn;
    open_page(&mut game);
    tap(&mut game, KeyCode::KeyT);
    // Where: Location, Left twice is Britannia.
    tap(&mut game, KeyCode::ArrowLeft);
    tap(&mut game, KeyCode::ArrowLeft);
    let form = page(&game).teleport().unwrap();
    assert!(matches!(
        form.area(),
        Some(TeleportArea::World(WorldPlane::Britannia))
    ));
    tap(&mut game, KeyCode::Enter);
    let state = &game.visual().state;
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        },
        "{:?}",
        page(&game).result()
    );
    assert_eq!(state.turn, turn, "no turn passes");
    assert_eq!(page(&game).result(), Some(TELEPORTED));
    assert!(session(&game).tainted);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_teleport_that_cannot_load_changes_nothing() {
    // The synthetic game has no files: the plan cannot load the place.
    let mut game = game(with(&[Toggle::CheatsTeleport]), None);
    game.visual_mut().game_dir = PathBuf::from("/nonexistent-u5-game");
    open_page(&mut game);
    let before = format!("{:?}", game.visual().state);
    game.app
        .world_mut()
        .resource_mut::<CheatSession>()
        .queued
        .push(CheatAction::Teleport(TeleportTarget {
            area: TeleportArea::World(WorldPlane::Britannia),
            x: 10,
            y: 10,
        }));
    game.frame();
    assert_eq!(page(&game).result(), Some(LOAD_FAILED));
    assert_eq!(format!("{:?}", game.visual().state), before);
    assert!(!session(&game).tainted);
}

#[test]
fn the_watch_alone_records_with_no_session_and_no_cheat_apply_work() {
    let mut game = game_with_watch_only();
    assert!(game.app.world().contains_resource::<WatchLog>());
    assert!(!game.app.world().contains_resource::<CheatSession>());
    assert!(!cheat_toggles(only(&[Toggle::CheatsStateWatch])).any_state_altering());
    tap(&mut game, KeyCode::ArrowRight);
    for _ in 0..10 {
        game.frame();
    }
    let log = game.app.world().resource::<WatchLog>();
    assert!(
        log.entries()
            .any(|e| e.field == u5_runtime::cheats::WatchField::Position),
        "the step was recorded"
    );
    assert!(!game.app.world().contains_resource::<CheatSession>());
    // `[W]` opens the face; no other key acts without a session.
    open_page(&mut game);
    tap(&mut game, KeyCode::KeyG);
    assert_eq!(page(&game).result(), None);
    tap(&mut game, KeyCode::KeyW);
    assert_eq!(page(&game).mode(), PageMode::Watch);
    tap(&mut game, KeyCode::Space);
    assert!(game.app.world().resource::<WatchLog>().paused());
    // Switching the watch off removes the log.
    game.app.insert_resource(Toggles(FeatureToggles::CLASSIC));
    game.frame();
    assert!(!game.app.world().contains_resource::<WatchLog>());
}

/// Class (a): scripted turns with the watch on and off give the same
/// digest every frame and the same `SAVED.GAM` bytes.
#[test]
fn the_watch_changes_neither_the_game_nor_the_save() {
    let (off_dir, on_dir) = (save_dir(), save_dir());
    let mut off = game(FeatureToggles::CLASSIC, Some(&off_dir));
    let mut on = game(only(&[Toggle::CheatsStateWatch]), Some(&on_dir));
    assert!(on.app.world().contains_resource::<WatchLog>());
    for key in [
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::Space,
        KeyCode::ArrowLeft,
    ] {
        for game in [&mut off, &mut on] {
            tap(game, key);
            for _ in 0..4 {
                game.frame();
            }
        }
        assert_eq!(
            trajectory_digest(&off.visual().state),
            trajectory_digest(&on.visual().state)
        );
        assert_eq!(off.game_view(), on.game_view());
    }
    q_y_save(&mut off);
    q_y_save(&mut on);
    let saved = |dir: &Path| fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(saved(&off_dir), saved(&on_dir));
    assert_ne!(
        saved(&on_dir),
        saved_game_seed_bytes(0, 0, 5, 5),
        "the save was rewritten"
    );
    assert!(!on.app.world().resource::<WatchLog>().is_empty());
    let _ = fs::remove_dir_all(off_dir);
    let _ = fs::remove_dir_all(on_dir);
}
