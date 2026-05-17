//! Shared moral-standing selector deltas for confirmed karma-adjusting
//! actions per `karma.md` §4. The selector is a single byte with a hard
//! upper clamp of [`MORAL_STANDING_MAX`] and a floor of `0`. This module
//! exposes the canonical (action, delta) table and a pure clamp helper so
//! callers can stay aligned with the spec text.

use crate::*;

/// `karma.md §4` confirmed moral-standing-selector mutators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KarmaAction {
    /// Shrine offering after the virtue's quest is already complete. The
    /// gold digit (1..=9) is added to the selector.
    CompletedShrineOffering { digit: u8 },
    /// Codex shrine turn-in: +3 (Humility receives an additional +3, so
    /// callers pass `humility = true` for that virtue).
    CodexShrineTurnIn { humility: bool },
    /// Town-family chest opening: -2, floored at zero.
    TownChestOpened,
    /// Picking a crop cell or eating reachable table food: -1 when nonzero.
    CropOrTableFoodTaken,
    /// F-Fire local cannon hit on a town active object: -5, floored at
    /// zero.
    TownCannonHit,
    /// Helped/pickpocket-style NPC thank-you path: +2.
    HelpedNpcThankYou,
    /// Three-digit conversation gold-payment milestone: +1, plus another
    /// +2 if the payment leaves the party with zero gold.
    TollMilestone { left_party_with_zero_gold: bool },
}

impl KarmaAction {
    /// Signed delta this action contributes to the selector before the
    /// canonical 0..=[`MORAL_STANDING_MAX`] clamp is applied. Returns the
    /// delta as `i16` so combined toll bonuses (+3) and Humility turn-ins
    /// (+6) round-trip without overflowing.
    pub const fn signed_delta(self) -> i16 {
        match self {
            KarmaAction::CompletedShrineOffering { digit } => digit as i16,
            KarmaAction::CodexShrineTurnIn { humility } => {
                let base = ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE as i16;
                if humility { base * 2 } else { base }
            }
            KarmaAction::TownChestOpened => -(TOWN_CHEST_OPEN_KARMA_DEBIT as i16),
            KarmaAction::CropOrTableFoodTaken => -(KARMA_CROP_OR_TABLE_FOOD_DEBIT as i16),
            KarmaAction::TownCannonHit => -(TOWN_CANNON_HIT_KARMA_DEBIT as i16),
            KarmaAction::HelpedNpcThankYou => KARMA_HELPED_NPC_THANK_YOU_GAIN as i16,
            KarmaAction::TollMilestone {
                left_party_with_zero_gold,
            } => {
                let base = KARMA_TOLL_MILESTONE_GAIN as i16;
                if left_party_with_zero_gold {
                    base + KARMA_TOLL_MILESTONE_ZERO_GOLD_BONUS as i16
                } else {
                    base
                }
            }
        }
    }
}

/// `karma.md §4` shared moral-standing debit applied when the party
/// picks a crop cell or eats reachable table food. The selector is
/// reduced by this many units (floored at zero) on each successful
/// take; if the selector is already zero, the path is a no-op rather
/// than a wrap.
pub const KARMA_CROP_OR_TABLE_FOOD_DEBIT: u8 = 1;
/// `karma.md §4` shared moral-standing bonus the helped/pickpocket-style
/// NPC thank-you path adds to the selector.
pub const KARMA_HELPED_NPC_THANK_YOU_GAIN: u8 = 2;
/// `karma.md §4` shared moral-standing bonus the three-digit
/// conversation gold-payment milestone adds when the toll-progress
/// counter has reached its milestone.
pub const KARMA_TOLL_MILESTONE_GAIN: u8 = 1;
/// `karma.md §4` extra moral-standing bonus added on top of
/// [`KARMA_TOLL_MILESTONE_GAIN`] when the milestone payment leaves
/// the party with zero gold.
pub const KARMA_TOLL_MILESTONE_ZERO_GOLD_BONUS: u8 = 2;

/// `karma.md §7` shrine meditation mantra-input cap. The handler
/// reads up to twelve characters before comparing against the
/// expected per-virtue mantra.
pub const SHRINE_MANTRA_INPUT_LIMIT: usize = 12;

/// `karma.md §7` per-virtue expected mantra. The shrine meditation
/// handler matches the typed input against this fixed table; a
/// wrong or blank input prints the no-effect branch.
pub const fn shrine_mantra_for(virtue: ShrineVirtue) -> &'static str {
    match virtue {
        ShrineVirtue::Honesty => "Ahm",
        ShrineVirtue::Compassion => "Mu",
        ShrineVirtue::Valor => "Ra",
        ShrineVirtue::Justice => "Beh",
        ShrineVirtue::Sacrifice => "Cah",
        ShrineVirtue::Honor => "Summ",
        ShrineVirtue::Spirituality => "Om",
        ShrineVirtue::Humility => "Lum",
    }
}

/// `karma.md §7` Avatar stat-reward unit applied per touched stat
/// during a Codex-read shrine turn-in. Each touched stat increments
/// by one and clamps at thirty. Returns the (str, dex, int) deltas
/// the turn-in writes onto the Avatar record. The same virtue
/// columns chargen scores at `+2` per question are scored at `+1`
/// here, except Humility which still grants no stat reward.
pub const CODEX_TURNIN_STAT_INCREMENT: u8 = 1;
pub const CODEX_TURNIN_STAT_CAP: u8 = 30;
pub const fn codex_turnin_stat_reward(virtue: ShrineVirtue) -> (u8, u8, u8) {
    match virtue {
        ShrineVirtue::Honesty => (0, 0, 1),
        ShrineVirtue::Compassion => (0, 1, 0),
        ShrineVirtue::Valor => (1, 0, 0),
        ShrineVirtue::Justice => (0, 1, 1),
        ShrineVirtue::Sacrifice => (1, 1, 0),
        ShrineVirtue::Honor => (1, 0, 1),
        ShrineVirtue::Spirituality => (1, 1, 1),
        ShrineVirtue::Humility => (0, 0, 0),
    }
}

/// `karma.md §5` resurrection-penalty threshold. At a moral-standing
/// selector of 98 or higher, the revived member's XP is unchanged;
/// below 98 the XP is scaled down by the selector percentage.
pub const RESURRECTION_PENALTY_SKIP_THRESHOLD: u8 = 98;

/// `karma.md §5`: returns `true` when the selector is high enough to
/// skip the resurrection XP penalty.
pub const fn resurrection_penalty_skipped(standing: u8) -> bool {
    standing >= RESURRECTION_PENALTY_SKIP_THRESHOLD
}

/// `karma.md §5`: revived member's experience after the resurrection
/// XP scale. With selector >= 98 the XP is unchanged; otherwise the
/// XP is multiplied by the selector / 100. Computed in u32 to avoid
/// `u16 * u16` overflow before the divide.
pub const fn resurrection_scaled_xp(standing: u8, current_xp: u16) -> u16 {
    if resurrection_penalty_skipped(standing) {
        return current_xp;
    }
    let product = (current_xp as u32) * (standing as u32);
    (product / 100) as u16
}

/// `karma.md §4` clamp policy: shrine and toll-style increments cap at
/// [`MORAL_STANDING_MAX`]; chest/crop/cannon decrements floor at zero;
/// the crop/table-food path is a no-op when the selector is already zero.
pub fn apply_karma_action(standing: u8, action: KarmaAction) -> u8 {
    match action {
        KarmaAction::CropOrTableFoodTaken if standing == 0 => 0,
        _ => {
            let next = i16::from(standing).saturating_add(action.signed_delta());
            next.clamp(0, MORAL_STANDING_MAX as i16) as u8
        }
    }
}
