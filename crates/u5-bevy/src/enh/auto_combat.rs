//! Automatic combat in the Bevy shell (Phase B design 6.3.4; build plan
//! M7-05).
//!
//! The planner lives in `u5_runtime::auto_combat` and answers with an
//! [`AutoKey`]. The shell types that key into the game exactly as a key
//! the player typed: one [`TypedKey`] pushed into
//! `VisualState::typed_keys` in `EnhSet::Feed`, which `drive_visual` then
//! drains through `visual_play_command_char` and `handle_play_key_input`,
//! the per-key path of every typed key (the foundation's synthetic-input
//! rule, R3.5). [`auto_typed_key`] is the one table between the two; its
//! test pins `visual_play_command_char` of each mapped key to
//! [`AutoKey::input_char`], so the terminal and the window cannot drift.
//!
//! [`AutoCombatPlugin`] is added by the foundation only when `combat.auto`
//! is on at launch, and never under the parity lock (the foundation itself
//! is absent then). Its systems, each in a foundation set:
//!
//! - `auto_combat_attach` (`Apply`): inserts [`AutoCombatShell`] when the
//!   toggle is on and the shell is absent (turned on in play);
//! - `auto_combat_detach` (`Apply`): runs only while the toggle is off and
//!   the shell is left, and removes it and the badge in the frame the
//!   toggle turns off;
//! - `auto_combat_actions` (`Apply`): the three hotkeys, as the router's
//!   [`EnhActionEvent`]s, change the engagement, write
//!   [`ShellBadges::auto`] and post the 6.2.2 toasts; outside combat a
//!   hotkey only posts the refusal; the first combat prompt of a session
//!   posts the ready note;
//! - `auto_combat_focus` (`Apply`): `WindowFocused` sets
//!   [`AutoCombatShell::focus_paused`] while `pause_on_focus_loss` is on.
//!
//! All but the detach run only while `combat.auto` is on. Nothing here
//! reads the keyboard, writes the game state or draws: the chords reach
//! the plugin only as router actions, the badge and toasts are the
//! foundation's, and every reader takes `&PlayState`. The feed, the
//! interrupt and the observer arrive with M7-06.

use bevy::prelude::*;
use bevy::window::WindowFocused;
use u5_runtime::InputDirection;
use u5_runtime::PlayState;
use u5_runtime::auto_combat::{
    AutoEngagement, AutoFightRuntime, AutoKey, AutoPrompt, CombatSettings, auto_prompt,
};
use u5_runtime::enhancements::Toggle;

use super::input::{EnhAction, EnhActionEvent};
use super::toast::{ShellBadges, Toasts, install_shell_services};
use super::{EnhLaunch, EnhSet, VisualLaunchConfig, feature_wanted, toggle_on};
use crate::{TypedKey, VisualState};

/// The toggle this module belongs to.
const AUTO: Toggle = Toggle::CombatAuto;
/// Roster positions, the numbering the stats panel shows.
const ROSTER_POSITIONS: usize = 6;

/// The engagement confirmations (6.2.2), 2 s each.
pub const AUTO_MEMBER: [&str; ROSTER_POSITIONS] = [
    "Auto: member 1",
    "Auto: member 2",
    "Auto: member 3",
    "Auto: member 4",
    "Auto: member 5",
    "Auto: member 6",
];
/// One member handed back by Ctrl+A while others stay engaged.
pub const AUTO_MEMBER_OFF: [&str; ROSTER_POSITIONS] = [
    "Auto off: member 1",
    "Auto off: member 2",
    "Auto off: member 3",
    "Auto off: member 4",
    "Auto off: member 5",
    "Auto off: member 6",
];
pub const AUTO_PARTY: &str = "Auto: party";
pub const AUTO_UNTIL_DONE: &str = "Auto: until done";
pub const AUTO_OFF: &str = "Auto off";
/// A hotkey outside combat: nothing changes.
pub const AUTO_NOT_IN_COMBAT: &str = "Auto-combat: not in combat";
/// Ctrl+A in combat while no member's command prompt is open (the
/// terminal's `@auto` says the same).
pub const AUTO_NO_PROMPT: &str = "Auto-combat: no member's prompt is open";
/// The once-per-session note at the first combat prompt (6.2.1), 5 s.
pub const AUTO_READY: &str =
    "Auto-combat ready: Ctrl+A member, Ctrl+Shift+A party, Ctrl+F until done";

/// The shell's auto-combat state (6.3.4). Exists only while `combat.auto`
/// is on, and never under the parity lock. The planner's engagement and
/// per-fight counters are shell-owned and never in `PlayState`.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct AutoCombatShell {
    pub settings: CombatSettings,
    pub engaged: AutoEngagement,
    pub fight: AutoFightRuntime,
    /// The window lost focus while `pause_on_focus_loss` is on.
    pub focus_paused: bool,
}

impl AutoCombatShell {
    pub fn new(settings: CombatSettings) -> Self {
        Self {
            settings,
            engaged: AutoEngagement::default(),
            fight: AutoFightRuntime::default(),
            focus_paused: false,
        }
    }

    /// Clears every engagement and the fight's counters.
    fn disengage(&mut self) {
        self.engaged = AutoEngagement::default();
        self.fight = AutoFightRuntime::default();
    }

    /// Applies one hotkey to the engagement against `state`, and returns
    /// the toast it shows. Outside combat nothing changes.
    pub fn apply_action(&mut self, action: EnhAction, state: &PlayState) -> Option<&'static str> {
        if !matches!(
            action,
            EnhAction::AutoMember | EnhAction::AutoParty | EnhAction::AutoUntilDone
        ) {
            return None;
        }
        if !state.combat_active {
            return Some(AUTO_NOT_IN_COMBAT);
        }
        let all = [true; ROSTER_POSITIONS];
        Some(match action {
            EnhAction::AutoMember => {
                let AutoPrompt::Command {
                    roster: Some(roster),
                    ..
                } = auto_prompt(state)
                else {
                    return Some(AUTO_NO_PROMPT);
                };
                let Some(member) = self.engaged.members.get_mut(roster) else {
                    return Some(AUTO_NO_PROMPT);
                };
                *member = !*member;
                if *member {
                    AUTO_MEMBER[roster]
                } else if self.engaged.any() {
                    AUTO_MEMBER_OFF[roster]
                } else {
                    self.disengage();
                    AUTO_OFF
                }
            }
            EnhAction::AutoParty => {
                if self.engaged.members == all && !self.engaged.until_done {
                    self.disengage();
                    AUTO_OFF
                } else {
                    self.engaged = AutoEngagement {
                        members: all,
                        until_done: false,
                    };
                    AUTO_PARTY
                }
            }
            _ => {
                if self.engaged.until_done {
                    self.disengage();
                    AUTO_OFF
                } else {
                    self.engaged = AutoEngagement {
                        members: all,
                        until_done: true,
                    };
                    AUTO_UNTIL_DONE
                }
            }
        })
    }
}

/// Adds the plugin when `combat.auto` is on at launch. Called from
/// [`super::EnhancementsPlugin`], which is itself absent under Classic and
/// under the parity lock.
pub(super) fn add_auto_combat(app: &mut App, launch: &VisualLaunchConfig) {
    if feature_wanted(launch.enhancements.toggles, AUTO) {
        app.add_plugins(AutoCombatPlugin {
            settings: launch.enhancements.combat(),
        });
    }
}

/// Automatic combat's feature plugin.
pub struct AutoCombatPlugin {
    settings: CombatSettings,
}

impl Plugin for AutoCombatPlugin {
    fn build(&self, app: &mut App) {
        install_shell_services(app);
        app.add_event::<WindowFocused>()
            .add_event::<EnhActionEvent>()
            .insert_resource(AutoCombatShell::new(self.settings.clone()))
            .add_systems(
                Update,
                (
                    auto_combat_detach.run_if(not(toggle_on(AUTO)).and(shell_left)),
                    auto_combat_attach.run_if(toggle_on(AUTO).and(not(shell_left))),
                    auto_combat_actions.run_if(toggle_on(AUTO)),
                    auto_combat_focus.run_if(toggle_on(AUTO)),
                )
                    .chain()
                    // A toggle changed this frame is seen this frame.
                    .after(super::apply_pending_toggles)
                    .in_set(EnhSet::Apply),
            );
    }
}

fn shell_left(shell: Option<Res<AutoCombatShell>>) -> bool {
    shell.is_some()
}

/// `combat.auto` turned on in play: a fresh shell with the launch's
/// `[combat]` settings, nothing engaged.
fn auto_combat_attach(mut commands: Commands, launch: Res<EnhLaunch>) {
    commands.insert_resource(AutoCombatShell::new(launch.0.enhancements.combat()));
}

/// `combat.auto` turned off: the shell and the badge go in the same
/// frame. The router's keys were released by the foundation's rebuild.
fn auto_combat_detach(mut commands: Commands, badges: Option<ResMut<ShellBadges>>) {
    commands.remove_resource::<AutoCombatShell>();
    if let Some(mut badges) = badges
        && badges.auto
    {
        badges.auto = false;
    }
}

/// The hotkeys, the badge, the toasts and the ready note (6.2.1, 6.2.2).
fn auto_combat_actions(
    mut actions: EventReader<EnhActionEvent>,
    shell: Option<ResMut<AutoCombatShell>>,
    visual: Option<Res<VisualState>>,
    mut toasts: ResMut<Toasts>,
    mut badges: ResMut<ShellBadges>,
    mut ready_noted: Local<bool>,
) {
    let (Some(mut shell), Some(visual)) = (shell, visual) else {
        actions.clear();
        return;
    };
    let state = &visual.state;
    for EnhActionEvent(action) in actions.read() {
        if let Some(toast) = shell.apply_action(*action, state) {
            toasts.short(toast);
            // The player knows the keys now.
            *ready_noted = true;
        }
    }
    if !*ready_noted && matches!(auto_prompt(state), AutoPrompt::Command { .. }) {
        *ready_noted = true;
        toasts.note(AUTO_READY);
    }
    let engaged = shell.engaged.any();
    if badges.auto != engaged {
        badges.auto = engaged;
    }
}

/// Focus loss pauses auto while `pause_on_focus_loss` is on (6.2.8); focus
/// regained resumes it.
fn auto_combat_focus(
    mut focus: EventReader<WindowFocused>,
    shell: Option<ResMut<AutoCombatShell>>,
) {
    let Some(mut shell) = shell else {
        focus.clear();
        return;
    };
    let Some(last) = focus.read().last() else {
        return;
    };
    let paused = shell.settings.pause_on_focus_loss && !last.focused;
    if shell.focus_paused != paused {
        shell.focus_paused = paused;
    }
}

/// The physical key the shell types for `key` (6.3.4): no modifier, so
/// the command prompt and the targeting cursor read it as the player's
/// own keypress.
///
/// | AutoKey | Key |
/// |---|---|
/// | Attack | A |
/// | Pass | Space |
/// | Step / Cursor | arrows, Home, PgUp, End, PgDn |
/// | Confirm | Enter |
/// | Cancel | Escape |
// The feed (M7-06) is the first caller outside the tests.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn auto_typed_key(key: AutoKey) -> TypedKey {
    let code = match key {
        AutoKey::Attack => KeyCode::KeyA,
        AutoKey::Pass => KeyCode::Space,
        AutoKey::Step(direction) | AutoKey::Cursor(direction) => match direction {
            InputDirection::North => KeyCode::ArrowUp,
            InputDirection::South => KeyCode::ArrowDown,
            InputDirection::East => KeyCode::ArrowRight,
            InputDirection::West => KeyCode::ArrowLeft,
            InputDirection::Northwest => KeyCode::Home,
            InputDirection::Northeast => KeyCode::PageUp,
            InputDirection::Southwest => KeyCode::End,
            InputDirection::Southeast => KeyCode::PageDown,
        },
        AutoKey::Confirm => KeyCode::Enter,
        AutoKey::Cancel => KeyCode::Escape,
    };
    TypedKey {
        key: code,
        shift: false,
        control: false,
    }
}

#[cfg(test)]
mod feed_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod shell_tests;
