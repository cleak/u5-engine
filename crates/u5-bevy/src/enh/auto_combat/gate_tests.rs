//! The M7 gate (design 6.4.2, 6.4.5, 6.6.3, 6.6.4, foundation 1.4.7; plan
//! M7-07 commit 2): with Phase 3c shipped the presets turn auto-combat on,
//! the lock still removes it, an engaged fight beside every other shipped
//! feature replays as the off run with the badge outside the frame, and
//! auto never answers to a monster the player cannot see or names one.

use u5_runtime::combat_actor::{
    COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER, COMBAT_ACTOR_FLAG_SELECTABLE_40,
};
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockReason, LockState, Preset, ResolveInput, TOGGLES, Tier,
    resolve, trajectory_digest,
};

use super::engaged_tests::{fast, game_in, weak_arena};
use super::feed_tests::{SKELETON_CLASS, arena, place};
use super::shell_tests::{Chord, Game};
use super::*;
use crate::enh::equivalence::{Owner, Scene, Timeline, check_shell_equivalence};
use crate::enh::input::KeyMap;

fn preset_launch(preset: Preset, lock: LockState) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve(ResolveInput {
            stored: None,
            cli: &[CliSetting::Preset(preset)],
            lock,
            caps: HostCapabilities::ALL,
        }),
        ..VisualLaunchConfig::classic()
    }
}

#[test]
fn the_presets_bring_auto_and_fast_forward_and_the_lock_takes_them_away() {
    for preset in [Preset::Recommended, Preset::Everything] {
        let game = Game::new(weak_arena(), &preset_launch(preset, LockState::Off));
        assert!(
            game.shell.app.is_plugin_added::<AutoCombatPlugin>(),
            "{preset:?}"
        );
        let auto = game.auto().unwrap();
        assert!(
            auto.settings.auto && auto.settings.fast_forward,
            "{preset:?}"
        );
        assert!(!auto.settings.allies, "{preset:?}");

        let locked = preset_launch(preset, LockState::Locked(LockReason::CliFlag));
        let mut game = Game::new(weak_arena(), &locked);
        assert!(!game.shell.app.is_plugin_added::<AutoCombatPlugin>());
        assert!(game.auto().is_none());
        assert!(!game.world().contains_resource::<KeyMap>(), "no chords");
        // The chords reach nothing: the game is Classic's.
        let mut classic = Game::new(weak_arena(), &VisualLaunchConfig::classic());
        for game in [&mut game, &mut classic] {
            game.frames(2);
            game.chord(Chord::UntilDone);
            game.chord(Chord::Party);
            game.frames(30);
        }
        assert_eq!(
            trajectory_digest(&game.shell.visual().state),
            trajectory_digest(&classic.shell.visual().state)
        );
        assert!(game.auto().is_none());
    }
    let classic = Game::new(
        weak_arena(),
        &preset_launch(Preset::Classic, LockState::Off),
    );
    assert!(!classic.shell.app.is_plugin_added::<AutoCombatPlugin>());
}

/// Whether the badge was up at the end of any frame.
#[derive(Resource, Default)]
struct BadgeSeen(bool);

fn see_badge(badges: Option<Res<ShellBadges>>, mut seen: ResMut<BadgeSeen>) {
    seen.0 |= badges.is_some_and(|badges| badges.auto);
}

/// Every other shipped toggle that is not a cheat, plus none.
fn companions() -> Vec<Option<Toggle>> {
    let mut companions = vec![None];
    companions.extend(
        TOGGLES
            .iter()
            .filter(|spec| spec.ships.is_shipped() && !spec.driven)
            .filter(|spec| spec.tier != Tier::Cheat && !spec.key.starts_with("combat."))
            .map(|spec| Some(spec.toggle)),
    );
    companions
}

#[test]
fn an_engaged_fight_beside_every_shipped_feature_replays_as_the_off_run() {
    // 6.6.4: for auto the "on" leg is the engaged replay. Ctrl+F at frame
    // 3, then the whole fight; the off run is Classic with the fed keys
    // put where the feed put them. The composed 320x200 frame is compared
    // every frame, so the badge, up for the whole fight, is never in it.
    let companions = companions();
    assert!(companions.len() >= 4, "{companions:?}");
    let mut timeline = Timeline::new(400);
    Chord::UntilDone.tap(&mut timeline, 3, Owner::Feature);
    for companion in companions {
        let mut launch = VisualLaunchConfig::classic();
        launch.enhancements.toggles.set(AUTO, true);
        if let Some(toggle) = companion {
            launch.enhancements.toggles.set(toggle, true);
        }
        let seen = std::sync::Arc::new(std::sync::Mutex::new(false));
        let shared = seen.clone();
        let mismatches = check_shell_equivalence(
            &weak_arena(),
            &Scene::synthetic(),
            &timeline,
            &launch,
            move |app| {
                app.init_resource::<BadgeSeen>()
                    .add_systems(Last, see_badge)
                    .add_systems(Last, move |seen: Res<BadgeSeen>| {
                        *shared.lock().unwrap() |= seen.0;
                    });
                app.world_mut()
                    .resource_mut::<AutoCombatShell>()
                    .settings
                    .step_ms = 0;
                app.world_mut()
                    .resource_mut::<AutoCombatShell>()
                    .settings
                    .pause_below_hp_percent = 0;
                app.world_mut().spawn(Window {
                    resolution: bevy::window::WindowResolution::new(2560.0, 1440.0),
                    ..default()
                });
            },
        );
        assert_eq!(mismatches, [], "auto with {companion:?}");
        assert!(*seen.lock().unwrap(), "{companion:?}: the badge was up");
    }
}

// ---------------------------------------------------------------------------
// No spoiler (foundation 1.4.7, 6.6.1 (i)), in the window.

/// The fed keys of a party engagement from `state`, up to and including
/// the first Enter, the point where the game itself resolves the attack,
/// or the first `limit` keys.
fn keys_until_the_game_reacts(state: PlayState, limit: usize) -> (Vec<TypedKey>, Vec<String>) {
    let mut game = game_in(state, &Scene::synthetic(), fast);
    game.frames(2);
    game.engage(Chord::Party);
    let (mut keys, mut toasts) = (Vec::new(), Vec::new());
    for step in 0..300 {
        if step > 0 {
            game.frames(1);
        }
        if let Some(toast) = game.toast().map(str::to_owned)
            && toasts.last() != Some(&toast)
        {
            toasts.push(toast);
        }
        for typed in game.fed_now().to_vec() {
            keys.push(typed);
            if typed.key == KeyCode::Enter || keys.len() == limit {
                return (keys, toasts);
            }
        }
    }
    panic!("no attack in 300 frames: {keys:?}");
}

fn with_invisible(slot: usize, x: u8, y: u8) -> PlayState {
    let mut state = arena();
    place(
        &mut state,
        slot,
        0x90,
        [
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER,
            SKELETON_CLASS,
            slot as u8,
            0,
            x,
            y,
        ],
    );
    state
}

#[test]
fn an_invisible_hostile_changes_no_key_and_no_note_in_the_window() {
    // The Bevy twin of the runtime's (i): the same keys and notes with an
    // unseen monster anywhere, nearer or in a lower slot than the visible
    // one, until the game itself resolves the attack.
    let expected = keys_until_the_game_reacts(arena(), usize::MAX);
    assert!(expected.0.len() >= 3, "{expected:?}");
    assert_eq!(
        expected.0.last().map(|typed| typed.key),
        Some(KeyCode::Enter)
    );
    // Off the approach: in front, behind, beside, in the corners, next to
    // the visible foe.
    for (slot, x, y) in [
        (9, 4, 5),
        (20, 5, 4),
        (9, 5, 6),
        (9, 0, 0),
        (20, 10, 10),
        (9, 8, 5),
        (9, 7, 6),
    ] {
        assert_eq!(
            keys_until_the_game_reacts(with_invisible(slot, x, y), usize::MAX),
            expected,
            "invisible slot {slot} at ({x}, {y})"
        );
    }
    // On the step cell the same step is planned; the game itself then
    // blocks it, as it would block the player's own step.
    let (first, _) = keys_until_the_game_reacts(with_invisible(9, 6, 5), 1);
    assert_eq!(first[..], expected.0[..1]);
}

#[test]
fn every_note_of_a_whole_fight_is_fixed_words_or_a_party_name() {
    let fixed: Vec<&str> = AUTO_MEMBER
        .iter()
        .chain(AUTO_MEMBER_OFF.iter())
        .copied()
        .chain([
            AUTO_PARTY,
            AUTO_UNTIL_DONE,
            AUTO_OFF,
            AUTO_NOT_IN_COMBAT,
            AUTO_NO_PROMPT,
            AUTO_READY,
            AUTO_OFF_KEY,
            AUTO_OFF_VICTORY,
            AUTO_OFF_NO_TARGET,
            AUTO_OFF_STUCK,
            AUTO_OFF_CAP,
            AUTO_OFF_UNEXPECTED,
        ])
        .collect();
    // A hurt member is named; that is the only name a note holds.
    let mut state = weak_arena();
    state.party[0].hp = 3;
    let mut seen = Vec::new();
    for (state, settings) in [
        (weak_arena(), fast as fn(&mut CombatSettings)),
        (state, |settings: &mut CombatSettings| {
            settings.step_ms = 0;
        }),
    ] {
        let mut game = game_in(state, &Scene::synthetic(), settings);
        game.frames(2);
        game.chord(Chord::UntilDone);
        for _ in 0..2000 {
            game.frames(1);
            if let Some(toast) = game.toast().map(str::to_owned)
                && seen.last() != Some(&toast)
            {
                seen.push(toast);
            }
            let auto = game.auto().unwrap();
            if !auto.engaged.any() && auto.handing_back.is_none() {
                break;
            }
        }
    }
    let names: Vec<String> = weak_arena()
        .party_names
        .iter()
        .map(|name| {
            let end = name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len());
            String::from_utf8_lossy(&name[..end]).into_owned()
        })
        .collect();
    assert!(
        seen.iter().any(|toast| toast == AUTO_OFF_VICTORY),
        "{seen:?}"
    );
    assert!(
        seen.iter().any(|toast| toast.ends_with(" is hurt")),
        "{seen:?}"
    );
    for toast in seen {
        let named = toast
            .strip_prefix("Auto off: ")
            .and_then(|rest| rest.strip_suffix(" is hurt"))
            .is_some_and(|name| {
                names.iter().any(|party| party == name) || name.starts_with("member ")
            });
        assert!(fixed.contains(&toast.as_str()) || named, "{toast}");
    }
}
