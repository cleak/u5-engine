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
                if humility {
                    6
                } else {
                    3
                }
            }
            KarmaAction::TownChestOpened => -2,
            KarmaAction::CropOrTableFoodTaken => -1,
            KarmaAction::TownCannonHit => -5,
            KarmaAction::HelpedNpcThankYou => 2,
            KarmaAction::TollMilestone {
                left_party_with_zero_gold,
            } => {
                if left_party_with_zero_gold {
                    3
                } else {
                    1
                }
            }
        }
    }
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
