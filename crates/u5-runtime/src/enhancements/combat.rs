//! The `[combat]` settings as the engine sees them (Phase B design 6.2.8,
//! 6.3.1, 6.4.1; build plan M7-01).
//!
//! The rows are in the catalogue ([`super::catalogue`]): the toggles
//! `combat.auto`, `combat.allies` and `combat.fast_forward`, and twelve
//! valued options, each inert while `combat.auto` is off. This module holds
//! the options' word lists and the typed view
//! [`ResolvedEnhancements::combat`], which turns the resolved values into the
//! plain [`CombatSettings`] both shells read. It is plain data: no serde, no
//! Bevy, no paths.

use super::catalogue::Toggle;
use super::resolve::ResolvedEnhancements;
use crate::auto_combat::{
    ACTION_CAP_MAX, ACTION_CAP_MIN, AutoTactic, AutoTargeting, CombatSettings, InterruptMode,
    MpReserve, PAUSE_BELOW_HP_MAX, STEP_MS_MAX, STEP_MS_MIN, VictoryAction,
};

pub const AUTO_KEY: &str = "combat.auto";
pub const ALLIES_KEY: &str = "combat.allies";
pub const FAST_FORWARD_KEY: &str = "combat.fast_forward";
pub const TACTIC_KEY: &str = "combat.tactic";
pub const TARGETING_KEY: &str = "combat.targeting";
pub const INTERRUPT_KEY: &str = "combat.interrupt";
pub const STEP_MS_KEY: &str = "combat.step_ms";
pub const PAUSE_BELOW_HP_KEY: &str = "combat.pause_below_hp";
pub const ACTION_CAP_KEY: &str = "combat.action_cap";
pub const VICTORY_KEY: &str = "combat.victory";
pub const KEEP_BETWEEN_FIGHTS_KEY: &str = "combat.keep_between_fights";
pub const PAUSE_ON_FOCUS_LOSS_KEY: &str = "combat.pause_on_focus_loss";
pub const POTIONS_KEY: &str = "combat.potions";
pub const MP_RESERVE_KEY: &str = "combat.mp_reserve";
pub const SPELLS_KEY: &str = "combat.spells";

/// The fifteen `[combat]` rows of 6.2.8, in its order.
pub const COMBAT_KEYS: &[&str] = &[
    AUTO_KEY,
    ALLIES_KEY,
    FAST_FORWARD_KEY,
    TACTIC_KEY,
    TARGETING_KEY,
    INTERRUPT_KEY,
    STEP_MS_KEY,
    PAUSE_BELOW_HP_KEY,
    ACTION_CAP_KEY,
    VICTORY_KEY,
    KEEP_BETWEEN_FIGHTS_KEY,
    PAUSE_ON_FOCUS_LOSS_KEY,
    POTIONS_KEY,
    MP_RESERVE_KEY,
    SPELLS_KEY,
];

/// `combat.tactic`, in [`AutoTactic`] order.
pub const TACTIC_WORDS: &[&str] = &["fight", "support", "magic"];
/// `combat.targeting`, in [`AutoTargeting`] order.
pub const TARGETING_WORDS: &[&str] = &["nearest", "weakest"];
/// `combat.interrupt`, in [`InterruptMode`] order.
pub const INTERRUPT_WORDS: &[&str] = &["swallow", "deliver"];
/// `combat.victory`, in [`VictoryAction`] order.
pub const VICTORY_WORDS: &[&str] = &["stop", "exit"];
/// `combat.mp_reserve`, in [`MpReserve`] order.
pub const MP_RESERVE_WORDS: &[&str] = &["none", "heal"];

/// The variant at `word`'s position in `words`, if it is one of them.
fn word<T: Copy>(words: &[&str], variants: &[T], value: Option<&str>) -> Option<T> {
    let value = value?;
    words
        .iter()
        .position(|candidate| *candidate == value)
        .map(|index| variants[index])
}

impl ResolvedEnhancements {
    /// The `[combat]` section as [`CombatSettings`] (6.3.1). As for
    /// [`gem_map`](Self::gem_map), a missing or unexpected value reads as
    /// the key's default and an out-of-range number is clamped. A row that
    /// has not shipped has already been resolved to its default (U13), and
    /// the parity lock gives the defaults.
    pub fn combat(&self) -> CombatSettings {
        let values = &self.values;
        let defaults = CombatSettings::default();
        let flag = |key: &str, default: bool| values.bool(key).unwrap_or(default);
        let int = |key: &str, default: u16, min: u16, max: u16| {
            values.int(key).map_or(default, |value| {
                value.clamp(i64::from(min), i64::from(max)) as u16
            })
        };
        CombatSettings {
            auto: self.toggles.on(Toggle::CombatAuto),
            allies: self.toggles.on(Toggle::CombatAllies),
            fast_forward: self.toggles.on(Toggle::CombatFastForward),
            tactic: word(
                TACTIC_WORDS,
                &[AutoTactic::Fight, AutoTactic::Support, AutoTactic::Magic],
                values.str(TACTIC_KEY),
            )
            .unwrap_or(defaults.tactic),
            targeting: word(
                TARGETING_WORDS,
                &[AutoTargeting::Nearest, AutoTargeting::Weakest],
                values.str(TARGETING_KEY),
            )
            .unwrap_or(defaults.targeting),
            interrupt: word(
                INTERRUPT_WORDS,
                &[InterruptMode::Swallow, InterruptMode::Deliver],
                values.str(INTERRUPT_KEY),
            )
            .unwrap_or(defaults.interrupt),
            step_ms: int(STEP_MS_KEY, defaults.step_ms, STEP_MS_MIN, STEP_MS_MAX),
            pause_below_hp_percent: int(
                PAUSE_BELOW_HP_KEY,
                u16::from(defaults.pause_below_hp_percent),
                0,
                u16::from(PAUSE_BELOW_HP_MAX),
            ) as u8,
            action_cap: int(
                ACTION_CAP_KEY,
                defaults.action_cap,
                ACTION_CAP_MIN,
                ACTION_CAP_MAX,
            ),
            victory: word(
                VICTORY_WORDS,
                &[VictoryAction::Stop, VictoryAction::Exit],
                values.str(VICTORY_KEY),
            )
            .unwrap_or(defaults.victory),
            keep_between_fights: flag(KEEP_BETWEEN_FIGHTS_KEY, defaults.keep_between_fights),
            pause_on_focus_loss: flag(PAUSE_ON_FOCUS_LOSS_KEY, defaults.pause_on_focus_loss),
            potions: flag(POTIONS_KEY, defaults.potions),
            mp_reserve: word(
                MP_RESERVE_WORDS,
                &[MpReserve::None, MpReserve::OneHeal],
                values.str(MP_RESERVE_KEY),
            )
            .unwrap_or(defaults.mp_reserve),
            spell_allow_list: values
                .list(SPELLS_KEY)
                .map_or(defaults.spell_allow_list, <[String]>::to_vec),
        }
    }
}

#[cfg(test)]
mod tests;
