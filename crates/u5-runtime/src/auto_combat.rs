//! Optional auto-combat: the runtime types both shells share (Phase B
//! design 6.3.2; build plan M7-01).
//!
//! Auto-combat is class (b): it acts only through keys a player could type.
//! This module holds the plain settings value [`CombatSettings`] that
//! `resolved.combat()` returns (it never parses anything), the shell-owned
//! bookkeeping [`AutoEngagement`] and [`AutoFightRuntime`], the planned key
//! [`AutoKey`] with the exact byte each shell feeds for it, and the
//! planner's answers [`AutoPlan`], [`HandBack`] and [`AutoPrompt`].
//!
//! Nothing here is stored in [`PlayState`](crate::PlayState), and nothing
//! here reads the transcript. The planner and the observer arrive with
//! build plan M7-02 and M7-03.

use crate::input_codes::{InputDirection, input_direction_code};

/// `combat.step_ms`'s default: the least time between two auto keys.
pub const STEP_MS_DEFAULT: u16 = 150;
/// `combat.step_ms`'s range (6.2.8).
pub const STEP_MS_MIN: u16 = 0;
pub const STEP_MS_MAX: u16 = 2000;
/// `combat.pause_below_hp`'s default, in percent; 0 is off.
pub const PAUSE_BELOW_HP_DEFAULT: u8 = 25;
pub const PAUSE_BELOW_HP_MAX: u8 = 100;
/// `combat.action_cap`'s default and range: auto keys in one fight.
pub const ACTION_CAP_DEFAULT: u16 = 200;
pub const ACTION_CAP_MIN: u16 = 20;
pub const ACTION_CAP_MAX: u16 = 1000;

/// Resolved `[combat]` settings: the typed view `resolved.combat()`
/// returns this. Never parsed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatSettings {
    /// The feature toggle, `combat.auto`.
    pub auto: bool,
    /// Phase 5: prompted allied monsters.
    pub allies: bool,
    /// Phase 4d: shorter cues (deviation tier).
    pub fast_forward: bool,
    pub tactic: AutoTactic,
    pub targeting: AutoTargeting,
    pub interrupt: InterruptMode,
    /// Least milliseconds between two auto keys, `0..=2000`.
    pub step_ms: u16,
    /// Hand a member back below this share of its HP; 0 is off.
    pub pause_below_hp_percent: u8,
    /// Auto keys in one fight before hand-back, `20..=1000`.
    pub action_cap: u16,
    pub victory: VictoryAction,
    pub keep_between_fights: bool,
    pub pause_on_focus_loss: bool,
    /// Phase 4a opt-in.
    pub potions: bool,
    pub mp_reserve: MpReserve,
    /// Spell codes as `catalogs/spell-list.md` section 5 prints them
    /// ("GP"). The engine has no spell-code type yet; the session's
    /// `spell_code` is a `String` too.
    pub spell_allow_list: Vec<String>,
}

impl Default for CombatSettings {
    /// The 6.2.8 defaults; `auto: false` is the faithful game.
    fn default() -> Self {
        Self {
            auto: false,
            allies: false,
            fast_forward: false,
            tactic: AutoTactic::Fight,
            targeting: AutoTargeting::Nearest,
            interrupt: InterruptMode::Swallow,
            step_ms: STEP_MS_DEFAULT,
            pause_below_hp_percent: PAUSE_BELOW_HP_DEFAULT,
            action_cap: ACTION_CAP_DEFAULT,
            victory: VictoryAction::Stop,
            keep_between_fights: false,
            pause_on_focus_loss: true,
            potions: false,
            mp_reserve: MpReserve::OneHeal,
            spell_allow_list: Vec::new(),
        }
    }
}

/// `combat.tactic`. Support and Magic arrive with Phase 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoTactic {
    #[default]
    Fight,
    Support,
    Magic,
}

/// `combat.targeting`. Weakest arrives with Phase 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoTargeting {
    #[default]
    Nearest,
    Weakest,
}

/// `combat.interrupt`: what a key the player presses while auto owns the
/// prompt does. Swallow hands back and drops the key; Deliver hands back
/// and passes the key to the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InterruptMode {
    #[default]
    Swallow,
    Deliver,
}

/// `combat.victory`. Exit arrives with Phase 4c.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VictoryAction {
    #[default]
    Stop,
    Exit,
}

/// `combat.mp_reserve` (Phases 4a and 4b): `none`, or `heal`, which keeps
/// enough magic for one healing spell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MpReserve {
    None,
    #[default]
    OneHeal,
}

/// Which roster positions the player handed over. Shell-owned; never in
/// `PlayState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoEngagement {
    pub members: [bool; 6],
    pub until_done: bool,
}

impl AutoEngagement {
    /// Whether any roster position is handed over.
    pub fn any(&self) -> bool {
        self.members.iter().any(|member| *member)
    }
}

/// A small read-only fingerprint of live state, taken just before and
/// just after an auto key, that recognises "this key changed nothing"
/// (6.3.2). Built from state by the observer (M7-02); never from the
/// transcript.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProgressPrint {
    pub pending_slot: Option<usize>,
    pub next_slot: usize,
    pub round_counter: u8,
    /// The open targeting cursor's cell, if one is open.
    pub cursor: Option<(u8, u8)>,
    pub prng_state: u16,
    /// A hash of the 32 combat actor rows.
    pub actors_hash: u64,
    /// Each party member's HP, MP and status.
    pub party: [(u16, u8, u8); 6],
}

/// Per-fight counters; reset when `combat_active` goes false.
/// Shell-owned; never in `PlayState`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoFightRuntime {
    pub actions: u16,
    pub no_progress: u8,
    /// Cardinal bitmask of steps refused this turn.
    pub blocked_dirs: u8,
    pub turn_slot: Option<usize>,
    pub last_print: Option<ProgressPrint>,
    pub owns_open_attempt: bool,
    pub stopping: Option<HandBack>,
}

/// One planned key. `input_char` is the exact char the per-key dispatcher
/// receives for the same physical key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoKey {
    /// `A`.
    Attack,
    /// Space.
    Pass,
    /// A step at the command prompt: cardinals only, `0x01..=0x04`.
    Step(InputDirection),
    /// A targeting-cursor move: `0x01..=0x04` and the diagonals
    /// `0xD3..=0xD6`.
    Cursor(InputDirection),
    /// Return.
    Confirm,
    /// Escape.
    Cancel,
}

impl AutoKey {
    /// The char the per-key dispatcher receives for this key, the same as
    /// for the physical key (6.3.4).
    pub fn input_char(self) -> char {
        match self {
            AutoKey::Attack => 'A',
            AutoKey::Pass => ' ',
            AutoKey::Step(direction) | AutoKey::Cursor(direction) => {
                char::from(input_direction_code(direction))
            }
            AutoKey::Confirm => '\r',
            AutoKey::Cancel => '\u{1b}',
        }
    }

    /// The `--play-script` token that reaches the dispatcher as
    /// [`input_char`](Self::input_char) (6.3.5). Only forms 1.0.1 already
    /// parses: `A`, `pass`, the ANSI navigation sequences, `enter` and a
    /// bare Escape byte.
    pub fn script_token(self) -> &'static str {
        match self {
            AutoKey::Attack => "A",
            AutoKey::Pass => "pass",
            AutoKey::Step(direction) | AutoKey::Cursor(direction) => match direction {
                InputDirection::North => "\x1b[A",
                InputDirection::South => "\x1b[B",
                InputDirection::East => "\x1b[C",
                InputDirection::West => "\x1b[D",
                InputDirection::Northwest => "\x1b[H",
                InputDirection::Northeast => "\x1b[5~",
                InputDirection::Southwest => "\x1b[F",
                InputDirection::Southeast => "\x1b[6~",
            },
            AutoKey::Confirm => "enter",
            AutoKey::Cancel => "\x1b",
        }
    }

    /// Whether the planner may emit this key: a step is cardinal only.
    pub fn is_well_formed(self) -> bool {
        match self {
            AutoKey::Step(direction) => direction.is_cardinal(),
            _ => true,
        }
    }
}

/// Why auto-combat gave a prompt back to the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandBack {
    /// The player pressed a key.
    Interrupted,
    Victory,
    /// The roster position whose HP fell below the limit.
    Hurt(usize),
    NoTarget,
    Stuck,
    ActionCap,
    UnexpectedPrompt,
    FightOver,
}

/// The planner's answer for the current prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPlan {
    Key(AutoKey),
    HandBack(HandBack),
    /// Not an auto-owned prompt: leave it to the human.
    NotOurs,
}

/// Which prompt is open, read from state only (6.3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoPrompt {
    None,
    /// The combat command prompt for actor `slot`, roster position
    /// `roster`.
    Command {
        slot: usize,
        roster: usize,
    },
    /// An open targeting cursor for actor `slot`.
    Cursor {
        slot: usize,
    },
    /// A picker, line reader, cast session, ...
    Other,
}

#[cfg(test)]
mod tests;
