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
    /// `conversation.md §7.4` `0x89` STANDING-UP control byte: +1,
    /// capped at ninety-nine. Scripts stack the byte to grant more
    /// than one.
    ConversationStandingUp,
    /// `conversation.md §7.4` `0x8A` STANDING-DOWN control byte: -1,
    /// floored at zero. Scripts stack the byte to deduct more than one.
    ConversationStandingDown,
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
            KarmaAction::ConversationStandingUp => KARMA_CONVERSATION_STANDING_STEP as i16,
            KarmaAction::ConversationStandingDown => -(KARMA_CONVERSATION_STANDING_STEP as i16),
        }
    }
}

/// `karma.md §4` / `conversation.md §7.4` magnitude each of the two
/// conversation standing control bytes moves the shared selector. Both
/// codes move it by exactly one; scripts stack the byte to move it
/// further, so this is deliberately not a per-record total.
pub const KARMA_CONVERSATION_STANDING_STEP: u8 = 1;

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
/// `karma.md §7` Codex turn-in stat-reward cap. The reward clamps
/// each touched stat at the engine-wide Avatar stat ceiling
/// (thirty). Anchored to [`crate::AVATAR_STAT_MAX`] so the
/// turn-in cap and the global stat ceiling stay one value.
pub const CODEX_TURNIN_STAT_CAP: u8 = crate::AVATAR_STAT_MAX;
/// `karma.md §8` (u5-spec#298 Q5, `RETRACTIONS.md` R545/R546): the
/// whereabouts value Blackthorn's execution parks a companion under, which
/// "the shrine/Codex presentation reads".
pub const INTERRED_WHEREABOUTS: u8 = 0x7F;

/// `karma.md §8`: the scan "covers roster slots 1..15 (every record but the
/// Avatar's) whether travelling or not".
pub const INTERRED_SCAN_SLOTS: std::ops::RangeInclusive<usize> = 1..=15;

/// `karma.md §8` (u5-spec#298 Q5): the Codex interred-party page. "the
/// presentation prints `\n\nThou_dost_see\n`, then `an_urn_marked:\n\n`
/// when exactly one record matches or `urns_marked:\n\n` for two or more,
/// then each matching record's name in roster-slot order, upper-cased byte
/// by byte, each followed by one line feed. The name is copied up to its
/// first zero byte", and the page "is skipped entirely when the count is
/// zero". `names` are the matching records' name fields in slot order.
///
/// Not reproduced: "a name that fills all nine bytes runs on into the record
/// bytes after the name field until a zero byte is met"; this copy stops at
/// the nine-byte field.
pub fn codex_interred_party_page(names: &[[u8; crate::SAVE_CHARACTER_NAME_LEN]]) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let mut page = String::from("\n\nThou dost see\n");
    page.push_str(if names.len() == 1 {
        "an urn marked:\n\n"
    } else {
        "urns marked:\n\n"
    });
    for name in names {
        for byte in name.iter().copied().take_while(|byte| *byte != 0) {
            page.push(char::from(byte.to_ascii_uppercase()));
        }
        page.push('\n');
    }
    Some(page)
}

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
/// below 98 the XP is scaled down by the selector percentage. The
/// threshold sits one selector step below [`crate::MORAL_STANDING_MAX`]
/// (`99`), so only the top two selector values (`98` and `99`) skip
/// the penalty. Anchored to `MORAL_STANDING_MAX - 1` so the skip
/// band derives from the published standing maximum.
pub const RESURRECTION_PENALTY_SKIP_THRESHOLD: u8 = crate::MORAL_STANDING_MAX - 1;

/// `karma.md §5` resurrection-penalty percentage divisor. The
/// revived member's experience is multiplied by `standing` and then
/// divided by this value to apply the selector percentage; promoting
/// it lets the helper name the divisor instead of repeating `100`
/// as a bare literal. The divisor is one above the maximum moral-
/// standing selector value, so the formula `xp * standing /
/// (MORAL_STANDING_MAX + 1)` treats standing as a percentage of the
/// full XP. Anchored to [`crate::MORAL_STANDING_MAX`] + 1 so the
/// percent divisor derives from the published standing maximum.
pub const RESURRECTION_PENALTY_PERCENT_DIVISOR: u32 = crate::MORAL_STANDING_MAX as u32 + 1;

/// `karma.md §5`: returns `true` when the selector is high enough to
/// skip the resurrection XP penalty.
pub const fn resurrection_penalty_skipped(standing: u8) -> bool {
    standing >= RESURRECTION_PENALTY_SKIP_THRESHOLD
}

/// `karma.md §5`: revived member's experience after the resurrection
/// XP scale. With selector >= 98 the XP is unchanged; otherwise the
/// XP is multiplied by the selector and divided by
/// [`RESURRECTION_PENALTY_PERCENT_DIVISOR`]. Computed in u32 to avoid
/// `u16 * u16` overflow before the divide.
pub const fn resurrection_scaled_xp(standing: u8, current_xp: u16) -> u16 {
    if resurrection_penalty_skipped(standing) {
        return current_xp;
    }
    let product = (current_xp as u32) * (standing as u32);
    (product / RESURRECTION_PENALTY_PERCENT_DIVISOR) as u16
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

#[cfg(test)]
mod codex_interred_page_spec_tests {
    use super::*;

    fn name(text: &str) -> [u8; crate::SAVE_CHARACTER_NAME_LEN] {
        let mut out = [0; crate::SAVE_CHARACTER_NAME_LEN];
        out[..text.len()].copy_from_slice(text.as_bytes());
        out
    }

    /// `karma.md §8` (u5-spec#298 Q5): `\n\nThou_dost_see\n`, then
    /// `an_urn_marked:\n\n` for one record or `urns_marked:\n\n` for two or
    /// more, then each name upper-cased with one line feed; nothing at all
    /// when the count is zero.
    #[test]
    fn interred_page_literals_and_plural_split() {
        assert_eq!(codex_interred_party_page(&[]), None);
        assert_eq!(
            codex_interred_party_page(&[name("Iolo")]).as_deref(),
            Some("\n\nThou dost see\nan urn marked:\n\nIOLO\n")
        );
        assert_eq!(
            codex_interred_party_page(&[name("Shamino"), name("Jaana")]).as_deref(),
            Some("\n\nThou dost see\nurns marked:\n\nSHAMINO\nJAANA\n")
        );
        // "a matching record whose name starts with a zero byte prints only
        // the line feed".
        assert_eq!(
            codex_interred_party_page(&[[0; crate::SAVE_CHARACTER_NAME_LEN]]).as_deref(),
            Some("\n\nThou dost see\nan urn marked:\n\n\n")
        );
    }
}
