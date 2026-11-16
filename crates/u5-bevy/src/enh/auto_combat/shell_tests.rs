//! Phase 3b, first half (design 6.2.1, 6.2.2, 6.3.4, 6.6.3; plan M7-05
//! commit 2): the plugin and its shell exist only while `combat.auto` is
//! on, the hotkeys toggle the engagement and the badge follows it, focus
//! loss pauses, and with the toggle off the chords are unbound and reach
//! no game reader. The plugin's systems run in both app chains.

use bevy::input::ButtonState;
use u5_runtime::enhancements::{LockReason, LockState};
use u5_runtime::test_fixtures::{open_grid, test_state};

use super::feed_tests::arena;
use super::*;
use crate::enh::equivalence::{
    Owner, Scene, Shell, Timeline, check_shell_equivalence, compare_shell_runs, run_shell,
};
use crate::enh::input::{KeyChord, KeyMap};
use crate::enh::toast::ToastKind;
use crate::enh::{PendingToggleChanges, Toggles, add_enhancements};
use crate::{
    add_visual_intro_update_systems, add_visual_update_systems, key_code_to_input_byte,
    key_code_to_line_input_byte,
};

/// Another toggle, so the foundation is in the app with auto off.
const OTHER: Toggle = Toggle::EnhancementsFreezeOverlays;

pub(super) fn launch(auto: bool) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(OTHER, true);
    launch.enhancements.toggles.set(AUTO, auto);
    launch
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Chord {
    Member,
    Party,
    UntilDone,
}

impl Chord {
    pub(super) const ALL: [Chord; 3] = [Chord::Member, Chord::Party, Chord::UntilDone];

    pub(super) fn shift(self) -> bool {
        matches!(self, Chord::Party)
    }

    pub(super) fn key(self) -> KeyCode {
        match self {
            Chord::Member | Chord::Party => KeyCode::KeyA,
            Chord::UntilDone => KeyCode::KeyF,
        }
    }

    /// Ctrl (and Shift) down, the key down at `frame`; all up at the next.
    pub(super) fn tap(self, timeline: &mut Timeline, frame: usize, owner: Owner) {
        let mut mods = vec![KeyCode::ControlLeft];
        if self.shift() {
            mods.push(KeyCode::ShiftLeft);
        }
        for key in mods.iter().copied().chain([self.key()]) {
            timeline.event(frame, key, ButtonState::Pressed, false, owner);
        }
        for key in mods.iter().copied().chain([self.key()]).rev() {
            timeline.event(frame + 1, key, ButtonState::Released, false, owner);
        }
    }
}

pub(super) struct Game {
    pub(super) shell: Shell,
    pub(super) timeline: Timeline,
    pub(super) frame: usize,
}

impl Game {
    pub(super) fn new(state: PlayState, launch: &VisualLaunchConfig) -> Self {
        Self {
            shell: Shell::new(state, &Scene::synthetic(), launch, |_| {}),
            timeline: Timeline::new(0),
            frame: 0,
        }
    }

    pub(super) fn frames(&mut self, count: usize) {
        for _ in 0..count {
            let events = if self.frame < self.timeline.len() {
                self.timeline.frame(self.frame).to_vec()
            } else {
                Vec::new()
            };
            self.shell.step(&events);
            self.frame += 1;
        }
    }

    /// Types `chord` now and runs until it is released.
    pub(super) fn chord(&mut self, chord: Chord) {
        chord.tap(&mut self.timeline, self.frame, Owner::Feature);
        self.frames(2);
    }

    pub(super) fn world(&self) -> &World {
        self.shell.app.world()
    }

    pub(super) fn auto(&self) -> Option<&AutoCombatShell> {
        self.world().get_resource::<AutoCombatShell>()
    }

    pub(super) fn engaged(&self) -> AutoEngagement {
        self.auto().expect("the shell is in the app").engaged
    }

    pub(super) fn toast(&self) -> Option<&str> {
        self.world().resource::<Toasts>().current()
    }

    pub(super) fn badge(&self) -> bool {
        self.world().resource::<ShellBadges>().auto
    }

    pub(super) fn focus(&mut self, focused: bool) {
        self.shell.app.world_mut().send_event(WindowFocused {
            window: Entity::PLACEHOLDER,
            focused,
        });
        self.frames(1);
    }
}

fn member(roster: usize) -> AutoEngagement {
    let mut engaged = AutoEngagement::default();
    engaged.members[roster] = true;
    engaged
}

const PARTY: AutoEngagement = AutoEngagement {
    members: [true; 6],
    until_done: false,
};
const UNTIL_DONE: AutoEngagement = AutoEngagement {
    members: [true; 6],
    until_done: true,
};

#[test]
fn the_shell_exists_only_while_auto_is_on_and_unlocked() {
    let game = Game::new(arena(), &launch(true));
    assert!(game.shell.app.is_plugin_added::<AutoCombatPlugin>());
    let auto = game.auto().unwrap();
    assert_eq!(auto.engaged, AutoEngagement::default());
    assert_eq!(auto.settings, CombatSettings::default().with_auto());

    let off = Game::new(arena(), &launch(false));
    assert!(!off.shell.app.is_plugin_added::<AutoCombatPlugin>());
    assert!(off.auto().is_none());

    let mut locked = launch(true);
    locked.enhancements.lock = LockState::Locked(LockReason::CliFlag);
    let locked = Game::new(arena(), &locked);
    assert!(!locked.shell.app.is_plugin_added::<AutoCombatPlugin>());
    assert!(locked.auto().is_none());
    assert!(!locked.world().contains_resource::<KeyMap>());
}

trait WithAuto {
    fn with_auto(self) -> Self;
}

impl WithAuto for CombatSettings {
    fn with_auto(mut self) -> Self {
        self.auto = true;
        self
    }
}

#[test]
fn the_hotkeys_toggle_the_engagement_and_the_badge_follows() {
    let mut game = Game::new(arena(), &launch(true));
    game.frames(3);
    assert!(!game.badge());

    game.chord(Chord::Member);
    assert_eq!(game.engaged(), member(0));
    assert_eq!(game.toast(), Some(AUTO_MEMBER[0]));
    assert!(game.badge());

    game.chord(Chord::Member);
    assert_eq!(game.engaged(), AutoEngagement::default());
    assert_eq!(game.toast(), Some(AUTO_OFF));
    assert!(!game.badge());

    game.chord(Chord::Party);
    assert_eq!(game.engaged(), PARTY);
    assert_eq!(game.toast(), Some(AUTO_PARTY));
    assert!(game.badge());

    // Ctrl+A on the prompted member of an engaged party hands only it back.
    game.chord(Chord::Member);
    let mut rest = PARTY;
    rest.members[0] = false;
    assert_eq!(game.engaged(), rest);
    assert_eq!(game.toast(), Some(AUTO_MEMBER_OFF[0]));
    assert!(game.badge());

    game.chord(Chord::UntilDone);
    assert_eq!(game.engaged(), UNTIL_DONE);
    assert_eq!(game.toast(), Some(AUTO_UNTIL_DONE));
    assert_eq!(
        game.world().resource::<Toasts>().kind(),
        Some(ToastKind::Short)
    );

    game.chord(Chord::UntilDone);
    assert_eq!(game.engaged(), AutoEngagement::default());
    assert_eq!(game.toast(), Some(AUTO_OFF));
    assert!(!game.badge());

    game.chord(Chord::Party);
    game.chord(Chord::Party);
    assert_eq!(game.engaged(), AutoEngagement::default());
    assert_eq!(game.toast(), Some(AUTO_OFF));
}

#[test]
fn outside_combat_a_hotkey_only_refuses() {
    let mut game = Game::new(test_state(open_grid(), 5, 5), &launch(true));
    game.frames(3);
    assert_eq!(game.toast(), None, "no ready note outside combat");
    for chord in Chord::ALL {
        game.chord(chord);
        assert_eq!(game.toast(), Some(AUTO_NOT_IN_COMBAT), "{chord:?}");
        assert_eq!(game.engaged(), AutoEngagement::default());
        assert!(!game.badge());
    }
}

#[test]
fn ctrl_a_without_a_members_prompt_is_refused() {
    // The player's own `A` has opened a targeting cursor: no command prompt
    // is open, so Ctrl+A has no member to hand over.
    let mut game = Game::new(arena(), &launch(true));
    game.timeline.game_tap(2, KeyCode::KeyA);
    game.frames(10);
    assert!(matches!(
        auto_prompt(&game.shell.visual().state),
        AutoPrompt::Cursor { .. }
    ));
    game.chord(Chord::Member);
    assert_eq!(game.toast(), Some(AUTO_NO_PROMPT));
    assert_eq!(game.engaged(), AutoEngagement::default());
    assert!(!game.badge());
}

#[test]
fn the_first_combat_prompt_shows_the_ready_note_once() {
    let mut game = Game::new(arena(), &launch(true));
    game.frames(1);
    assert_eq!(game.toast(), Some(AUTO_READY));
    assert_eq!(
        game.world().resource::<Toasts>().kind(),
        Some(ToastKind::Note)
    );
    game.chord(Chord::Party);
    game.chord(Chord::Party);
    // Real time never moves in the headless shell: clear the line by hand.
    game.shell.app.insert_resource(Toasts::default());
    game.frames(5);
    assert_eq!(game.toast(), None, "once per session");
}

#[test]
fn hotkeys_and_engagement_leave_the_game_alone() {
    // Every chord the plugin takes is invisible to the game, frame by
    // frame. The feed is held by a focus pause and the player's Space is
    // delivered, so only the chords differ between the runs (the engaged
    // feed has its own tests).
    let mut timeline = Timeline::new(60);
    for (index, chord) in Chord::ALL.into_iter().cycle().take(6).enumerate() {
        chord.tap(&mut timeline, 5 + index * 8, Owner::Feature);
    }
    timeline.game_tap(55, KeyCode::Space);
    let mismatches = check_shell_equivalence(
        &arena(),
        &Scene::synthetic(),
        &timeline,
        &launch(true),
        |app| {
            let mut shell = app.world_mut().resource_mut::<AutoCombatShell>();
            shell.focus_paused = true;
            shell.settings.interrupt = u5_runtime::auto_combat::InterruptMode::Deliver;
        },
    );
    assert_eq!(mismatches, []);
}

#[test]
fn focus_loss_pauses_and_focus_resumes() {
    let mut game = Game::new(arena(), &launch(true));
    game.frames(1);
    assert!(!game.auto().unwrap().focus_paused);
    game.focus(false);
    assert!(game.auto().unwrap().focus_paused);
    game.focus(true);
    assert!(!game.auto().unwrap().focus_paused);

    let mut game = Game::new(arena(), &launch(true));
    game.shell
        .app
        .world_mut()
        .resource_mut::<AutoCombatShell>()
        .settings
        .pause_on_focus_loss = false;
    game.focus(false);
    assert!(!game.auto().unwrap().focus_paused);
}

const CHORDS: [(&str, KeyCode, bool); 3] = [
    ("Ctrl+A", KeyCode::KeyA, false),
    ("Ctrl+Shift+A", KeyCode::KeyA, true),
    ("Ctrl+F", KeyCode::KeyF, false),
];

#[test]
fn off_the_chords_are_unbound_and_map_to_no_byte() {
    for (text, key, shift) in CHORDS {
        assert_eq!(key_code_to_input_byte(key, shift, true), None, "{text}");
        assert_eq!(
            key_code_to_line_input_byte(key, shift, true),
            None,
            "{text}"
        );
        assert!(!KeyChord::parse(text).unwrap().reserved(), "{text}");
    }
    let on = Game::new(arena(), &launch(true));
    let keymap = on.world().resource::<KeyMap>();
    for ((text, _, _), action) in CHORDS.into_iter().zip([
        EnhAction::AutoMember,
        EnhAction::AutoParty,
        EnhAction::AutoUntilDone,
    ]) {
        assert_eq!(keymap.action(KeyChord::parse(text).unwrap()), Some(action));
    }
    let off = Game::new(arena(), &launch(false));
    if let Some(keymap) = off.world().get_resource::<KeyMap>() {
        for (text, _, _) in CHORDS {
            assert_eq!(
                keymap.action(KeyChord::parse(text).unwrap()),
                None,
                "{text}"
            );
        }
    }

    // The chords typed with auto off are the player's keys: the game's key
    // queue and state follow the Classic app exactly.
    let mut timeline = Timeline::new(40);
    for (index, chord) in Chord::ALL.into_iter().enumerate() {
        chord.tap(&mut timeline, 5 + index * 8, Owner::Game);
    }
    let scene = Scene::synthetic();
    let classic = run_shell(
        arena(),
        &scene,
        &VisualLaunchConfig::classic(),
        |_| {},
        &timeline,
        None,
    );
    let other = run_shell(arena(), &scene, &launch(false), |_| {}, &timeline, None);
    assert_eq!(compare_shell_runs(&classic, &other), []);
}

#[test]
fn turning_auto_off_in_play_removes_the_shell_badge_and_keys() {
    let mut game = Game::new(arena(), &launch(true));
    game.frames(1);
    game.chord(Chord::Party);
    assert!(game.badge());
    assert_eq!(AUTO.spec().apply, u5_runtime::enhancements::ApplyAt::Frame);
    game.shell
        .app
        .world_mut()
        .resource_mut::<PendingToggleChanges>()
        .0
        .push((AUTO, false));
    game.frames(1);
    assert!(!game.world().resource::<Toggles>().0.on(AUTO));
    assert!(game.auto().is_none(), "gone in the frame it turns off");
    assert!(!game.badge());
    let keymap = game.world().resource::<KeyMap>();
    assert_eq!(keymap.action(KeyChord::parse("Ctrl+A").unwrap()), None);
    game.chord(Chord::Party);
    assert!(game.auto().is_none());
    assert!(!game.badge());

    game.shell
        .app
        .world_mut()
        .resource_mut::<PendingToggleChanges>()
        .0
        .push((AUTO, true));
    game.frames(1);
    assert_eq!(
        game.engaged(),
        AutoEngagement::default(),
        "back on, nothing engaged"
    );
    game.chord(Chord::Party);
    assert_eq!(game.engaged(), PARTY);
}

/// The `Update` systems of `app` in executable order.
fn update_order(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut schedule = world
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .remove(Update)
        .unwrap();
    schedule.initialize(world).unwrap();
    let order = schedule
        .systems()
        .unwrap()
        .map(|(_, system)| system.name().into_owned())
        .collect();
    world
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .insert(schedule);
    order
}

#[test]
fn the_plugin_runs_in_both_app_chains() {
    let builders: [(&str, fn(&mut App), &[&str]); 2] = [
        ("--visual", add_visual_update_systems, &["drive_visual"]),
        (
            "intro",
            add_visual_intro_update_systems,
            &["drive_visual", "drive_visual_intro"],
        ),
    ];
    for (label, build, drivers) in builders {
        let mut app = App::new();
        build(&mut app);
        add_enhancements(&mut app, &launch(true));
        let order = update_order(&mut app);
        let at = |name: &str| {
            order
                .iter()
                .position(|system| system.rsplit("::").next() == Some(name))
                .unwrap_or_else(|| panic!("{label}: {name} not scheduled"))
        };
        for ours in [
            "auto_combat_detach",
            "auto_combat_attach",
            "auto_combat_actions",
            "auto_combat_focus",
        ] {
            for driver in drivers {
                assert!(at(ours) < at(driver), "{label}: {ours} before {driver}");
            }
        }

        let mut off = App::new();
        build(&mut off);
        add_enhancements(&mut off, &launch(false));
        assert!(
            update_order(&mut off)
                .iter()
                .all(|system| !system.contains("auto_combat")),
            "{label}: nothing of auto-combat while it is off"
        );
    }
}
