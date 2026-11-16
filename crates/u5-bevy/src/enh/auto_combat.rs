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
//! - `auto_combat_interrupt` (`Input`, after the router): the first key
//!   the player presses that yields a game byte while auto runs stops it
//!   (6.2.4). Swallow resets that key; while an auto attempt is open the
//!   frame's byte keys are reset and kept in [`AutoCombatShell::held_keys`];
//! - `auto_combat_feed` (`Feed`): at most one planned key per frame, only
//!   when every feed condition holds (6.3.4); on a hand-back it re-feeds the
//!   held keys one per frame, then releases the token and posts the toast;
//! - `auto_combat_observe` (`Observe`): the key's progress print in the
//!   same frame it was fed, victory and the action cap, and the reset when
//!   the fight is over.
//!
//! Fast-forward (Phase 4d): each fed key opens a [`CueSpan`] of
//! `sound_effect_serial`s, closed when the shell is quiet again. While one
//! is open in "until done" with `fast_forward` on, `play_speaker_effects`
//! asks [`AutoCombatShell::speaker_cue`] for each blocking cue and plays
//! its first [`FAST_FORWARD_CUE`]; the order is unchanged. That one
//! `Option<Res<AutoCombatShell>>` parameter is the chapter's only edit to a
//! faithful system; with the shell absent it takes its 1.0.1 path.
//!
//! All but the detach run only while `combat.auto` is on. Nothing here
//! writes the game state or draws: a planned key reaches the game only
//! through `typed_keys`, the badge and toasts are the foundation's, and
//! every reader takes `&PlayState`. The keyboard is read only by the
//! interrupt, which resets keys on `ButtonInput` as the router does.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use bevy::ecs::event::EventCursor;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::WindowFocused;
use u5_runtime::InputDirection;
use u5_runtime::PlayState;
use u5_runtime::audio::SpeakerProgram;
use u5_runtime::auto_combat::{
    AutoEngagement, AutoFightRuntime, AutoKey, AutoPlan, AutoPrompt, CombatSettings, HandBack,
    InterruptMode, auto_prompt, observe_after_auto_key, plan_next_auto_key,
    record_print_before_auto_key,
};
use u5_runtime::combat_actor::combat_side_census;
use u5_runtime::enhancements::Toggle;

use super::idle::{load_holding, shell_quiescent, speaker_playing};
use super::input::{EnhAction, EnhActionEvent, enh_input_router, is_modifier};
use super::overlay::OverlayState;
use super::synthetic::{SyntheticInputOwner, SyntheticOwner, install_synthetic_input};
use super::toast::{ShellBadges, Toasts, install_shell_services};
use super::{EnhLaunch, EnhSet, VisualLaunchConfig, feature_wanted, toggle_on};
use crate::{
    EmulatorLoadLatency, SpeakerQueue, SpeakerVoice, TypedKey, VisualState, key_code_to_input_byte,
    key_code_to_line_input_byte, keys_pressed_in_typed_order, visual_line_prompt_active,
};

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
/// Shorter than chapter 6's wording (73 cells), so it wraps whole at the
/// font's own scale and is never squeezed to fit (integration polish).
pub const AUTO_READY: &str = "Auto: Ctrl+A one, Ctrl+Shift+A all, Ctrl+F to end";

/// The hand-back toasts (6.2.5), 2 s each. [`HandBack::Hurt`] names the
/// member and is built from the roster name.
pub const AUTO_OFF_KEY: &str = "Auto off: key pressed";
pub const AUTO_OFF_VICTORY: &str = "Victory - auto off";
pub const AUTO_OFF_NO_TARGET: &str = "Auto off: no target in sight";
pub const AUTO_OFF_STUCK: &str = "Auto off: stuck";
pub const AUTO_OFF_CAP: &str = "Auto off: action limit";
pub const AUTO_OFF_UNEXPECTED: &str = "Auto off: unexpected prompt";

/// The token owner this module feeds as.
const ME: SyntheticOwner = SyntheticOwner::AutoCombat;

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
    /// Keys the player typed while auto finished an open attempt, or while
    /// the held keys go back; re-fed in order after the hand-back.
    pub(crate) held_keys: VecDeque<TypedKey>,
    /// The key fed this frame, awaiting its observation.
    pub fed: Option<AutoKey>,
    /// When the last auto key was fed (real time).
    pub last_auto_key: Option<Duration>,
    /// A hand-back in progress: the held keys go back, then the token is
    /// released and this reason's toast is posted.
    pub handing_back: Option<HandBack>,
    /// Whether a live hostile remained at the last frame of combat, for
    /// the victory toast at a fight's end (the actor rows are cleared by
    /// then).
    pub foes_remained: bool,
    /// The `sound_effect_serial` range the last auto key produced, open
    /// until the shell is quiet again (Phase 4d).
    pub cue_span: Option<CueSpan>,
    /// Whether the last command prompt or cursor seen was auto's, so a key
    /// at a prompt that is neither (a picker, a line prompt, the round
    /// between turns) stops auto only when it followed auto's own turn, and
    /// never when it is the player's own menu for a member not handed over.
    pub last_prompt_auto: bool,
}

/// The `sound_effect_serial` range one auto key produced (design 6.3.4,
/// Phase 4d): every effect after `after`, through `through` once the shell
/// is quiet again.
///
/// A key is fed only while the shell is quiet: no voice, nothing queued
/// and every recorded effect already sent. So every effect the speaker
/// receives while the span is open has a serial in it: the key's own cues
/// and those of the round it set going, up to the next quiet frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CueSpan {
    /// The serial when the key was fed; the span holds the effects after it.
    pub after: u64,
    /// The last serial in the span, set when the shell is quiet again.
    pub through: Option<u64>,
}

/// The fixed short envelope a blocking cue plays in fast-forward (Phase 4d):
/// the cue's first 40 ms, then its stop.
pub const FAST_FORWARD_CUE: Duration = Duration::from_millis(40);

impl AutoCombatShell {
    pub fn new(settings: CombatSettings) -> Self {
        Self {
            settings,
            engaged: AutoEngagement::default(),
            fight: AutoFightRuntime::default(),
            focus_paused: false,
            held_keys: VecDeque::new(),
            fed: None,
            last_auto_key: None,
            handing_back: None,
            foes_remained: true,
            cue_span: None,
            last_prompt_auto: false,
        }
    }

    /// Whether the speaker shortens blocking cues now: `fast_forward` and
    /// "until done" are both set, and an auto key's span is open.
    pub fn fast_forwards_cues(&self) -> bool {
        self.settings.fast_forward
            && self.engaged.until_done
            && self.cue_span.is_some_and(|span| span.through.is_none())
    }

    /// The cue the speaker plays for `program` (Phase 4d). In fast-forward a
    /// blocking cue plays its first [`FAST_FORWARD_CUE`] and stops; its
    /// viewport inversion starts no later than half way through. Anything
    /// else, and every cue outside fast-forward, is returned unchanged. The
    /// caller has already built the program, so the jitter stream and the
    /// cue order are the same as at normal pace.
    pub(crate) fn speaker_cue(
        &self,
        program: SpeakerProgram,
        viewport_flash: Option<Duration>,
        audible: bool,
        blocking: bool,
    ) -> (SpeakerProgram, Option<Duration>) {
        if !blocking || !self.fast_forwards_cues() {
            return (program, viewport_flash);
        }
        let nanos = FAST_FORWARD_CUE.as_nanos() as u64;
        (
            program.truncated_at(nanos, audible),
            viewport_flash.map(|start| start.min(FAST_FORWARD_CUE / 2)),
        )
    }

    /// The span of the key fed now, at `serial`.
    fn open_cue_span(&mut self, serial: u64) {
        self.cue_span = Some(CueSpan {
            after: serial,
            through: None,
        });
    }

    /// The shell is quiet at `serial`: an open span ends there.
    fn close_cue_span(&mut self, serial: u64) {
        if let Some(span) = self.cue_span.as_mut()
            && span.through.is_none()
        {
            span.through = Some(serial);
        }
    }

    /// Clears every engagement and the fight's counters.
    fn disengage(&mut self) {
        self.engaged = AutoEngagement::default();
        self.fight = AutoFightRuntime::default();
        self.fed = None;
        self.last_prompt_auto = false;
    }

    /// Auto gives the prompt back for `reason`: nothing stays engaged, and
    /// the held keys go back before the toast.
    fn hand_back(&mut self, reason: HandBack) {
        self.disengage();
        self.handing_back = Some(reason);
    }

    /// Whether an attempt auto opened is still open.
    fn attempt_open(&self, state: &PlayState) -> bool {
        self.fight.owns_open_attempt && matches!(auto_prompt(state), AutoPrompt::Cursor { .. })
    }

    /// Whether auto owns the command prompt or cursor open now; `None` at
    /// any other prompt.
    fn owns_prompt(&self, state: &PlayState) -> Option<bool> {
        match auto_prompt(state) {
            AutoPrompt::Cursor { .. } => Some(self.fight.owns_open_attempt),
            AutoPrompt::Command { .. } => Some(!matches!(
                plan_next_auto_key(state, &self.settings, &self.engaged, &self.fight),
                AutoPlan::NotOurs
            )),
            AutoPrompt::None | AutoPrompt::Other => None,
        }
    }

    /// Whether auto is running, so a key the player presses stops it: it
    /// owns the prompt open now, its attempt is open, or the fight is
    /// between prompts after auto's own turn. A prompt, cursor, picker or
    /// line prompt of a member not handed over is the player's.
    fn running(&self, state: &PlayState) -> bool {
        if !self.engaged.any() || !state.combat_active {
            return false;
        }
        self.owns_prompt(state).unwrap_or(self.last_prompt_auto)
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
        install_synthetic_input(app);
        app.add_event::<WindowFocused>()
            .add_event::<EnhActionEvent>()
            .add_event::<KeyboardInput>()
            .insert_resource(AutoCombatShell::new(self.settings.clone()))
            .add_systems(
                PreUpdate,
                auto_combat_interrupt
                    .run_if(toggle_on(AUTO))
                    .after(enh_input_router)
                    .in_set(EnhSet::Input),
            )
            .add_systems(
                Update,
                auto_combat_feed
                    .run_if(toggle_on(AUTO))
                    .in_set(EnhSet::Feed),
            )
            .add_systems(
                Update,
                auto_combat_observe
                    .run_if(toggle_on(AUTO))
                    .in_set(EnhSet::Observe),
            )
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
                    .after(super::apply_pending_changes)
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

/// `combat.auto` turned off: the shell, the token and the badge go in the
/// same frame. The router's keys were released by the foundation's rebuild.
fn auto_combat_detach(
    mut commands: Commands,
    badges: Option<ResMut<ShellBadges>>,
    token: Option<ResMut<SyntheticInputOwner>>,
) {
    commands.remove_resource::<AutoCombatShell>();
    if let Some(mut token) = token
        && token.holder() == Some(ME)
    {
        token.release(ME);
    }
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

/// The byte `key` gives the game now, if any: the line editor's map while
/// a line prompt is open, the command map otherwise. A modifier gives none.
fn byte_of(key: KeyCode, shift: bool, control: bool, line: bool) -> Option<u8> {
    if line {
        key_code_to_line_input_byte(key, shift, control)
    } else {
        key_code_to_input_byte(key, shift, control)
    }
}

/// Shift and Ctrl as the game's readers see them this frame.
fn modifiers(keyboard: &ButtonInput<KeyCode>) -> (bool, bool) {
    (
        keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
    )
}

/// The interrupt (6.2.4, 6.3.4), after the router took the chords.
fn auto_combat_interrupt(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    key_events: Option<Res<Events<KeyboardInput>>>,
    mut key_cursor: Local<EventCursor<KeyboardInput>>,
    shell: Option<ResMut<AutoCombatShell>>,
    visual: Option<Res<VisualState>>,
) {
    // Read every frame, so no event of an earlier frame is taken as fresh.
    let typed = keys_pressed_in_typed_order(&keyboard, key_events.as_deref(), &mut key_cursor);
    let (Some(mut shell), Some(visual)) = (shell, visual) else {
        return;
    };
    let state = &visual.state;
    let (shift, control) = modifiers(&keyboard);
    let line = visual_line_prompt_active(state);
    let bytes: Vec<KeyCode> = typed
        .into_iter()
        .filter(|key| !is_modifier(*key) && byte_of(*key, shift, control, line).is_some())
        .collect();
    if bytes.is_empty() {
        return;
    }
    let hold = |shell: &mut AutoCombatShell, keyboard: &mut ButtonInput<KeyCode>, key| {
        keyboard.reset(key);
        shell.held_keys.push_back(TypedKey {
            key,
            shift,
            control,
        });
    };
    // The held keys are going back: what follows them waits behind them.
    if shell.handing_back.is_some() {
        for key in bytes {
            hold(&mut shell, &mut keyboard, key);
        }
        return;
    }
    if !shell.running(state) {
        return;
    }
    let attempt_open = shell.attempt_open(state);
    for key in bytes {
        if shell.fight.stopping.is_none() {
            shell.fight.stopping = Some(HandBack::Interrupted);
            if shell.settings.interrupt == InterruptMode::Swallow {
                keyboard.reset(key);
                continue;
            }
        }
        if attempt_open {
            hold(&mut shell, &mut keyboard, key);
        }
    }
}

/// The hand-back toast for `reason`. A hurt member's toast is built once
/// per name and kept, so the toast service's `&'static str` holds it.
fn hand_back_toast(
    reason: HandBack,
    state: &PlayState,
    names: &mut HashMap<String, &'static str>,
) -> &'static str {
    match reason {
        HandBack::Interrupted => AUTO_OFF_KEY,
        HandBack::Victory => AUTO_OFF_VICTORY,
        HandBack::Hurt(roster) => {
            let name = state
                .party_names
                .get(roster)
                .map(|name| {
                    let end = name
                        .iter()
                        .position(|byte| *byte == 0)
                        .unwrap_or(name.len());
                    String::from_utf8_lossy(&name[..end]).into_owned()
                })
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| format!("member {}", roster + 1));
            names.entry(name).or_insert_with_key(|name| {
                Box::leak(format!("Auto off: {name} is hurt").into_boxed_str())
            })
        }
        HandBack::NoTarget => AUTO_OFF_NO_TARGET,
        HandBack::Stuck => AUTO_OFF_STUCK,
        HandBack::ActionCap => AUTO_OFF_CAP,
        HandBack::UnexpectedPrompt => AUTO_OFF_UNEXPECTED,
        HandBack::FightOver => AUTO_OFF,
    }
}

/// Whether a key other than a lone modifier waits for the game's read.
fn keys_waiting(visual: &VisualState) -> bool {
    visual
        .typed_keys
        .iter()
        .any(|typed| !is_modifier(typed.key))
}

/// The feed (6.3.4): one key per frame at most, only when every condition
/// holds; the hand-back's held keys, one per frame, then the token and the
/// toast.
#[allow(clippy::too_many_arguments)]
fn auto_combat_feed(
    mut shell: ResMut<AutoCombatShell>,
    visual: Option<ResMut<VisualState>>,
    overlay: Option<Res<OverlayState>>,
    mut token: ResMut<SyntheticInputOwner>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    speaker_queue: Option<Res<SpeakerQueue>>,
    speaker_voices: Query<(), With<SpeakerVoice>>,
    load_latency: Option<Res<EmulatorLoadLatency>>,
    mut toasts: ResMut<Toasts>,
    mut names: Local<HashMap<String, &'static str>>,
) {
    let Some(mut visual) = visual else {
        return;
    };
    let speaker_busy = speaker_playing(speaker_queue.as_deref(), &speaker_voices)
        || visual.sound_effect_cursor != visual.state.sound_effect_serial;
    let quiet = shell_quiescent(&visual, speaker_busy, load_holding(load_latency.as_deref()));
    if quiet {
        shell.close_cue_span(visual.state.sound_effect_serial);
    }
    let overlay_open = overlay.is_some_and(|overlay| overlay.is_open());
    if let Some(reason) = shell.handing_back {
        if overlay_open || keys_waiting(&visual) {
            return;
        }
        if let Some(key) = shell.held_keys.pop_front() {
            visual.typed_keys.push_back(key);
            return;
        }
        shell.handing_back = None;
        token.release(ME);
        toasts.short(hand_back_toast(reason, &visual.state, &mut names));
        return;
    }
    if !shell.engaged.any() {
        if token.holder() == Some(ME) {
            token.release(ME);
        }
        return;
    }
    let now = time.elapsed();
    let step = Duration::from_millis(u64::from(shell.settings.step_ms));
    let (shift, control) = modifiers(&keyboard);
    let line = visual_line_prompt_active(&visual.state);
    let byte_pressed = keyboard
        .get_just_pressed()
        .any(|key| !is_modifier(*key) && byte_of(*key, shift, control, line).is_some());
    let ready = !overlay_open
        && token.free_or(ME)
        && !shell.focus_paused
        && shell
            .last_auto_key
            .is_none_or(|last| now.saturating_sub(last) >= step)
        && !keys_waiting(&visual)
        && !byte_pressed
        && quiet;
    if !ready {
        return;
    }
    let shell = &mut *shell;
    match plan_next_auto_key(&visual.state, &shell.settings, &shell.engaged, &shell.fight) {
        AutoPlan::Key(key) => {
            if token.take(ME).is_err() {
                return;
            }
            record_print_before_auto_key(&visual.state, &mut shell.fight);
            visual.typed_keys.push_back(auto_typed_key(key));
            shell.open_cue_span(visual.state.sound_effect_serial);
            shell.fed = Some(key);
            shell.last_auto_key = Some(now);
        }
        AutoPlan::HandBack(reason) => shell.hand_back(reason),
        AutoPlan::NotOurs => {}
    }
}

/// The observer (6.3.4): the fed key's print in the frame the game read it,
/// victory and the cap once no auto attempt is open, and the reset when the
/// fight is over (engagement too, unless `keep_between_fights`; never for
/// "until done").
fn auto_combat_observe(mut shell: ResMut<AutoCombatShell>, visual: Option<Res<VisualState>>) {
    let Some(visual) = visual else {
        return;
    };
    let state = &visual.state;
    if let Some(key) = shell.fed
        && !keys_waiting(&visual)
    {
        shell.fed = None;
        observe_after_auto_key(state, key, &mut shell.fight);
    }
    if state.combat_active {
        if let Some(ours) = shell.owns_prompt(state)
            && shell.last_prompt_auto != ours
        {
            shell.last_prompt_auto = ours;
        }
        let foes = combat_side_census(&state.combat_actors).foes_remain();
        shell.foes_remained = foes;
        if shell.engaged.any() && shell.fed.is_none() && !shell.attempt_open(state) {
            if !foes {
                shell.hand_back(HandBack::Victory);
            } else if shell.fight.actions >= shell.settings.action_cap {
                shell.hand_back(HandBack::ActionCap);
            }
        }
        return;
    }
    // The fight is over.
    let foes_remained = std::mem::replace(&mut shell.foes_remained, true);
    shell.fight = AutoFightRuntime::default();
    shell.fed = None;
    if shell.last_prompt_auto {
        shell.last_prompt_auto = false;
    }
    if !shell.engaged.any() || shell.handing_back.is_some() {
        return;
    }
    if shell.settings.keep_between_fights && !shell.engaged.until_done {
        return;
    }
    shell.hand_back(if foes_remained {
        HandBack::FightOver
    } else {
        HandBack::Victory
    });
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
mod engaged_tests;
#[cfg(test)]
mod fast_forward_tests;
#[cfg(test)]
mod feed_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod shell_tests;
