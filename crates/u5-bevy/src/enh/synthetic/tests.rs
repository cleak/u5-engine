//! The synthetic-key owner: one holder at a time, no feed while an overlay
//! is open, and fed keys are typed keys (design 1.3.7, plan M0-13).

use u5_runtime::enhancements::TOGGLES;

use super::*;
use crate::enh::configure_enh_sets;
use crate::enh::overlay::tests::Game;
use crate::enh::overlay::{OverlayId, OverlayRequest};
use crate::enh::{VisualLaunchConfig, add_enhancements};

const UP: SyntheticKey = SyntheticKey::plain(KeyCode::ArrowUp);

/// The input driver and idle pump; with `overlay`, the router, the overlay
/// model and the test overlay too; and the synthetic owner.
fn game(overlay: bool) -> Game {
    let mut game = Game::new(overlay);
    if !overlay {
        configure_enh_sets(&mut game.app);
    }
    install_synthetic_input(&mut game.app);
    game.frame();
    game
}

fn owner(game: &mut Game) -> Mut<'_, SyntheticInputOwner> {
    game.app.world_mut().resource_mut::<SyntheticInputOwner>()
}

#[test]
fn one_owner_holds_the_token_at_a_time() {
    use SyntheticOwner::*;
    let mut token = SyntheticInputOwner::default();
    assert_eq!(token.holder(), None);
    for me in [MenuSlotSave, StartKeys, AutoCombat, TitleJ] {
        assert!(token.free_or(me), "{me:?}: a free token");
    }
    assert_eq!(token.take(AutoCombat), Ok(()));
    assert_eq!(token.take(AutoCombat), Ok(()), "taking it again is fine");
    for other in [MenuSlotSave, StartKeys, TitleJ] {
        assert!(!token.free_or(other), "{other:?}");
        assert_eq!(token.take(other), Err(SyntheticRefused::HeldBy(AutoCombat)));
        assert_eq!(
            token.queue(other, [UP]),
            Err(SyntheticRefused::HeldBy(AutoCombat))
        );
        assert!(!token.release(other), "{other:?} cannot release it");
    }
    assert!(token.free_or(AutoCombat));
    assert_eq!(token.holder(), Some(AutoCombat));
    assert_eq!(token.queued(), 0, "no refused key was queued");

    assert!(token.release(AutoCombat));
    assert_eq!(token.holder(), None);
    assert_eq!(token.take(MenuSlotSave), Ok(()));
    assert_eq!(token.holder(), Some(MenuSlotSave));
}

#[test]
fn only_the_holder_queues_and_its_keys_go_with_it() {
    let mut token = SyntheticInputOwner::default();
    assert_eq!(
        token.queue(SyntheticOwner::StartKeys, [UP]),
        Err(SyntheticRefused::NotHolder)
    );
    token.take(SyntheticOwner::StartKeys).unwrap();
    token
        .queue(
            SyntheticOwner::StartKeys,
            [UP, SyntheticKey::shifted(KeyCode::KeyQ)],
        )
        .unwrap();
    assert_eq!(token.queued(), 2);
    assert!(token.may_feed(false));
    assert!(!token.may_feed(true), "no feed while an overlay is open");
    assert!(token.release(SyntheticOwner::StartKeys));
    assert_eq!(token.queued(), 0, "no key outlives its owner");
    assert!(!token.may_feed(false));
}

#[test]
fn no_token_unless_a_feature_installs_it() {
    let mut launch = VisualLaunchConfig::classic();
    for spec in TOGGLES {
        launch.enhancements.toggles.set(spec.toggle, true);
    }
    for launch in [VisualLaunchConfig::classic(), launch] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<SyntheticInputOwner>());
    }
    let mut app = App::new();
    install_synthetic_input(&mut app);
    install_synthetic_input(&mut app);
    let feeds: usize = app
        .world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter()
        .map(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .filter(|(_, system, _)| system.name().contains("feed_synthetic_keys"))
                .count()
        })
        .sum();
    assert_eq!(feeds, 1);
}

/// A fed key takes the typed key's path: the game ends exactly where a
/// player who pressed it would.
#[test]
fn a_fed_key_is_a_typed_key() {
    let mut typed = game(false);
    let mut fed = game(false);
    let start = typed.where_the_avatar_is();

    typed.press(KeyCode::ArrowUp);
    owner(&mut fed).take(SyntheticOwner::AutoCombat).unwrap();
    owner(&mut fed)
        .queue(SyntheticOwner::AutoCombat, [UP])
        .unwrap();
    typed.frame();
    fed.frame();
    typed.release(KeyCode::ArrowUp);
    assert_ne!(
        typed.where_the_avatar_is(),
        start,
        "the key moved the Avatar"
    );
    for frame in 0..30 {
        assert_eq!(
            format!("{:?} {:?}", typed.visual().state, typed.visual().typed_keys),
            format!("{:?} {:?}", fed.visual().state, fed.visual().typed_keys),
            "frame {frame}"
        );
        typed.frame();
        fed.frame();
    }
    assert_eq!(owner(&mut fed).queued(), 0);
}

/// No key is fed while an overlay is open, however long; it waits and is
/// fed once the overlay closes.
#[test]
fn no_feed_while_an_overlay_is_open() {
    let mut game = game(true);
    game.tap_map_key();
    assert_eq!(game.open(), Some(OverlayId::Map));
    let start = game.where_the_avatar_is();

    owner(&mut game).take(SyntheticOwner::AutoCombat).unwrap();
    owner(&mut game)
        .queue(SyntheticOwner::AutoCombat, [UP])
        .unwrap();
    for frame in 0..60 {
        game.frame();
        assert!(game.visual().typed_keys.is_empty(), "frame {frame}");
        assert_eq!(owner(&mut game).queued(), 1, "frame {frame}");
    }
    assert_eq!(game.where_the_avatar_is(), start);

    // Closed in `Apply`; the feed in `Feed` sees it closed the same frame.
    game.request(OverlayRequest::Close(OverlayId::Map));
    game.frame();
    assert_eq!(game.open(), None);
    assert_eq!(owner(&mut game).queued(), 0);
    assert_ne!(game.where_the_avatar_is(), start, "fed after the close");
}

/// An overlay that opens in the frame the keys were queued keeps them: the
/// feed runs after the overlay bookkeeping.
#[test]
fn an_overlay_opening_this_frame_holds_the_feed() {
    let mut game = game(true);
    let start = game.where_the_avatar_is();
    owner(&mut game).take(SyntheticOwner::StartKeys).unwrap();
    owner(&mut game)
        .queue(SyntheticOwner::StartKeys, [UP])
        .unwrap();
    game.tap_map_key();
    assert_eq!(game.open(), Some(OverlayId::Map));
    assert_eq!(owner(&mut game).queued(), 1);
    assert!(game.visual().typed_keys.is_empty());
    assert_eq!(game.where_the_avatar_is(), start);
}

/// Before gameplay there is no key queue to feed: the keys wait.
#[test]
fn keys_wait_while_there_is_no_play_state() {
    let mut app = App::new();
    configure_enh_sets(&mut app);
    install_synthetic_input(&mut app);
    let mut token = app.world_mut().resource_mut::<SyntheticInputOwner>();
    token.take(SyntheticOwner::TitleJ).unwrap();
    token
        .queue(SyntheticOwner::TitleJ, [SyntheticKey::plain(KeyCode::KeyJ)])
        .unwrap();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<SyntheticInputOwner>().queued(), 1);
}

/// A feed held behind an open overlay does not mark `VisualState` changed,
/// so a change-driven reader such as the save watcher sees nothing new.
#[test]
fn a_held_feed_leaves_the_play_state_unmarked() {
    let mut images = Assets::<Image>::default();
    let visual = crate::enh::overlay::tests::idle_visual_state(&mut images);
    let mut app = App::new();
    configure_enh_sets(&mut app);
    install_synthetic_input(&mut app);
    app.insert_resource(visual)
        .insert_resource(OverlayState::opened(OverlayId::Map));
    let mut token = app.world_mut().resource_mut::<SyntheticInputOwner>();
    token.take(SyntheticOwner::AutoCombat).unwrap();
    token.queue(SyntheticOwner::AutoCombat, [UP]).unwrap();
    app.update();
    let changed = |app: &App| {
        app.world()
            .get_resource_change_ticks::<VisualState>()
            .expect("VisualState is in the app")
            .changed
    };
    let before = changed(&app);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(changed(&app), before, "nothing was fed");
    assert_eq!(app.world().resource::<SyntheticInputOwner>().queued(), 1);

    app.world_mut().insert_resource(OverlayState::default());
    app.update();
    assert_ne!(
        changed(&app),
        before,
        "the feed marks it once it moves keys"
    );
    assert_eq!(app.world().resource::<VisualState>().typed_keys.len(), 1);
}
