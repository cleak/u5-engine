//! Terminal endgame state helpers.

use crate::*;

/// `catalogs/quest-graph.md §5` Eternal-Flame principle the
/// destruction of one Shadowlord's shard must use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EternalFlame {
    /// Falsehood's opposed flame.
    Truth,
    /// Hatred's opposed flame.
    Love,
    /// Cowardice's opposed flame.
    Courage,
}

/// `catalogs/quest-graph.md §5`: the Eternal-Flame paired with each
/// Shadowlord (by zero-based slot 0=Falsehood / 1=Hatred / 2=Cowardice).
pub const fn eternal_flame_for_shadowlord(slot: usize) -> Option<EternalFlame> {
    Some(match slot {
        0 => EternalFlame::Truth,
        1 => EternalFlame::Love,
        2 => EternalFlame::Courage,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §5` typed Shadowlord identity. The three
/// hideout slots are tied to a fixed virtue principle: slot 0 is
/// Falsehood (Fauline), slot 1 is Hatred (Astaroth), slot 2 is
/// Cowardice (Nosfentor). The principle, the name, and the opposed
/// Eternal-Flame are stable across the quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowlordPrinciple {
    /// Slot 0 — Fauline, opposed by the Flame of Truth.
    Falsehood,
    /// Slot 1 — Astaroth, opposed by the Flame of Love.
    Hatred,
    /// Slot 2 — Nosfentor, opposed by the Flame of Courage.
    Cowardice,
}

impl ShadowlordPrinciple {
    /// Slot order published in `catalogs/quest-graph.md §5`.
    pub const ALL: [Self; 3] = [Self::Falsehood, Self::Hatred, Self::Cowardice];

    /// `catalogs/quest-graph.md §5`: hideout slot index `0..=2` the
    /// shipped roster assigns to this principle.
    pub const fn slot(self) -> usize {
        match self {
            Self::Falsehood => 0,
            Self::Hatred => 1,
            Self::Cowardice => 2,
        }
    }

    /// `catalogs/quest-graph.md §5`: opposed Eternal Flame.
    pub const fn eternal_flame(self) -> EternalFlame {
        match self {
            Self::Falsehood => EternalFlame::Truth,
            Self::Hatred => EternalFlame::Love,
            Self::Cowardice => EternalFlame::Courage,
        }
    }
}

/// `catalogs/quest-graph.md §5`: classify a hideout slot index into
/// the typed Shadowlord principle, or `None` for indices outside
/// the published `0..=2` set.
pub const fn shadowlord_principle_for_slot(slot: usize) -> Option<ShadowlordPrinciple> {
    Some(match slot {
        0 => ShadowlordPrinciple::Falsehood,
        1 => ShadowlordPrinciple::Hatred,
        2 => ShadowlordPrinciple::Cowardice,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §2`: the four main-quest requirements
/// the web-shaped progression converges on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainQuestRequirement {
    /// 1. Recover the royal artifacts: Crown, Sceptre, and Amulet.
    RoyalArtifacts,
    /// 2. Learn and use the eight dungeon Words of Power.
    DungeonWords,
    /// 3. Recover and destroy the three evil shards by using the
    ///    Eternal Flames and the Shadowlords' names.
    ShardsAndShadowlords,
    /// 4. Preserve and use the hidden sandalwood-box object that
    ///    enables Lord British's return.
    SandalwoodBox,
}

impl MainQuestRequirement {
    /// `catalogs/quest-graph.md §2` ordered list of the four
    /// requirements as they appear in the spec.
    pub const ALL: [Self; 4] = [
        Self::RoyalArtifacts,
        Self::DungeonWords,
        Self::ShardsAndShadowlords,
        Self::SandalwoodBox,
    ];
}

/// `catalogs/quest-graph.md §7`: shipped Sandalwood Box pickup —
/// non-speaking object slot 31 in `CASTLE:0` at local (X=18, Y=12, Z=2)
/// with object tag `0x0E` (the pickup runs through the shared
/// item-add path, sets the save-backed box flag).
/// `catalogs/quest-graph.md §7`: the Sandalwood Box ships in
/// Lord British's Castle (`CASTLE:0`, scene byte 17). Anchored
/// to [`crate::SCENE_LORD_BRITISHS_CASTLE`] so the box's scene
/// and the named scene constant share one source of truth.
pub const SANDALWOOD_BOX_PICKUP_SCENE: u8 = crate::SCENE_LORD_BRITISHS_CASTLE;
pub const SANDALWOOD_BOX_PICKUP_X: u8 = 18;
pub const SANDALWOOD_BOX_PICKUP_Y: u8 = 12;
pub const SANDALWOOD_BOX_PICKUP_Z: u8 = 2;
/// `catalogs/quest-graph.md §7` shipped Sandalwood Box pickup —
/// the box lives in the last active-object slot of the
/// 32-record CASTLE:0 active-object table. Anchored to
/// [`crate::OOL_SLOTS`] - 1 so the highest active-object slot
/// index has one source of truth.
pub const SANDALWOOD_BOX_PICKUP_OBJECT_SLOT: usize = crate::OOL_SLOTS - 1;
pub const SANDALWOOD_BOX_PICKUP_TAG: u8 = 0x0E;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndgameState {
    pub first_confirmation: Option<bool>,
    pub final_confirmation: Option<bool>,
    pub outcome: Option<EndgameOutcome>,
    pub certificate: Option<String>,
    pub cinematic: crate::endgame_cinematic::EndgameCinematic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndgameOutcome {
    Victory,
    MissingBoxOrRefused,
}

impl EndgameState {
    pub fn awaiting_first_confirmation() -> Self {
        Self {
            first_confirmation: None,
            final_confirmation: None,
            outcome: None,
            certificate: None,
            cinematic: crate::endgame_cinematic::EndgameCinematic::default(),
        }
    }

    pub fn awaiting_final_confirmation(first_confirmation: bool) -> Self {
        Self {
            first_confirmation: Some(first_confirmation),
            final_confirmation: None,
            outcome: None,
            certificate: None,
            cinematic: crate::endgame_cinematic::EndgameCinematic::default(),
        }
    }

    pub fn terminal(
        first_confirmation: bool,
        final_confirmation: bool,
        has_sandalwood_box: bool,
        certificate: String,
    ) -> Self {
        let outcome = endgame_outcome(final_confirmation, has_sandalwood_box);
        let cinematic = if outcome == EndgameOutcome::Victory {
            crate::endgame_cinematic::EndgameCinematic::start()
        } else {
            crate::endgame_cinematic::EndgameCinematic::default()
        };
        Self {
            first_confirmation: Some(first_confirmation),
            final_confirmation: Some(final_confirmation),
            outcome: Some(outcome),
            certificate: (outcome == EndgameOutcome::Victory).then_some(certificate),
            cinematic,
        }
    }

    /// Advance the post-victory cinematic by one keystroke. Returns
    /// the new step's banner label for caller-side display.
    pub fn advance_cinematic(&mut self) -> &'static str {
        self.cinematic.advance();
        self.cinematic.banner_label()
    }

    /// `true` when the post-victory cinematic has presented every
    /// screen and the engine should remain on the terminal final panel.
    pub fn cinematic_is_finished(&self) -> bool {
        self.cinematic.is_finished()
    }

    pub fn is_terminal(&self) -> bool {
        self.outcome.is_some()
    }
}

/// `endgame.md §4`: returns `true` when a party slot needs the
/// dead-member restoration pass before the throne-room tableau. The
/// pass announces the restoration, flips the slot back to a Good
/// status, and restores current HP from the stored maximum. Only
/// Dead status triggers it; Ashes remains outside the restoration
/// per the documented per-slot loop, and other statuses keep
/// whatever HP/status they brought into the endgame.
pub const fn endgame_needs_tableau_restoration(status: CharacterStatus) -> bool {
    matches!(status, CharacterStatus::Dead)
}

pub fn endgame_outcome(final_confirmation: bool, has_sandalwood_box: bool) -> EndgameOutcome {
    if final_confirmation && has_sandalwood_box {
        EndgameOutcome::Victory
    } else {
        EndgameOutcome::MissingBoxOrRefused
    }
}

/// `endgame.md §9` certificate elapsed-time baseline. The certificate
/// computes campaign elapsed time by subtracting this fixed date from
/// the saved world clock under the thirteen-month / twenty-eight-day
/// calendar model. Year 139, month 4, day 5 corresponds to the
/// campaign-start date the certificate calls the "beginning of the
/// quest".
pub const ENDGAME_CAMPAIGN_START_YEAR: u16 = crate::CHARGEN_STARTING_YEAR;
pub const ENDGAME_CAMPAIGN_START_MONTH: u8 = crate::CHARGEN_STARTING_MONTH;
pub const ENDGAME_CAMPAIGN_START_DAY: u8 = crate::CHARGEN_STARTING_DAY;

pub fn endgame_elapsed_campaign_time(clock: GameClock) -> (u16, u8, u8) {
    let mut years = clock.year.saturating_sub(ENDGAME_CAMPAIGN_START_YEAR);
    let mut months = clock.month as i16 - ENDGAME_CAMPAIGN_START_MONTH as i16;
    let mut days = clock.day as i16 - ENDGAME_CAMPAIGN_START_DAY as i16;

    if days < 0 {
        days += DAYS_PER_MONTH as i16;
        months -= 1;
    }
    if months < 0 {
        months += MONTHS_PER_YEAR as i16;
        years = years.saturating_sub(1);
    }

    (years, months as u8, days as u8)
}

pub fn endgame_certificate_summary(leader_name: &str, clock: GameClock) -> String {
    let (years, months, days) = endgame_elapsed_campaign_time(clock);
    let day = endgame_ordinal_word(clock.day).unwrap_or_else(|| clock.day.to_string());
    let month = endgame_ordinal_word(clock.month).unwrap_or_else(|| clock.month.to_string());
    let year = endgame_cardinal_word(clock.year);
    format!(
        "Certificate: On the {day} day of the {month} month in the year {year}, {leader_name} restored Lord British, the people, and the land. Quest time: {}. Report this completed quest to Origin.",
        elapsed_time_label(years, months, days)
    )
}

pub fn endgame_ordinal_word(value: u8) -> Option<String> {
    let word = match value {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        11 => "eleventh",
        12 => "twelfth",
        13 => "thirteenth",
        14 => "fourteenth",
        15 => "fifteenth",
        16 => "sixteenth",
        17 => "seventeenth",
        18 => "eighteenth",
        19 => "nineteenth",
        20 => "twentieth",
        21 => "twenty-first",
        22 => "twenty-second",
        23 => "twenty-third",
        24 => "twenty-fourth",
        25 => "twenty-fifth",
        26 => "twenty-sixth",
        27 => "twenty-seventh",
        28 => "twenty-eighth",
        _ => return None,
    };
    Some(word.to_string())
}

pub fn endgame_cardinal_word(value: u16) -> String {
    match value {
        0 => "zero".to_string(),
        1..=19 => small_cardinal_word(value as u8).unwrap().to_string(),
        20..=99 => two_digit_cardinal_word(value as u8),
        100..=999 => {
            let hundreds = value / 100;
            let remainder = value % 100;
            if remainder == 0 {
                format!("{} hundred", endgame_cardinal_word(hundreds))
            } else {
                format!(
                    "{} hundred {}",
                    endgame_cardinal_word(hundreds),
                    endgame_cardinal_word(remainder)
                )
            }
        }
        1000..=9999 => {
            let thousands = value / 1000;
            let remainder = value % 1000;
            if remainder == 0 {
                format!("{} thousand", endgame_cardinal_word(thousands))
            } else {
                format!(
                    "{} thousand {}",
                    endgame_cardinal_word(thousands),
                    endgame_cardinal_word(remainder)
                )
            }
        }
        _ => value.to_string(),
    }
}

fn small_cardinal_word(value: u8) -> Option<&'static str> {
    Some(match value {
        0 => "zero",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        11 => "eleven",
        12 => "twelve",
        13 => "thirteen",
        14 => "fourteen",
        15 => "fifteen",
        16 => "sixteen",
        17 => "seventeen",
        18 => "eighteen",
        19 => "nineteen",
        _ => return None,
    })
}

fn two_digit_cardinal_word(value: u8) -> String {
    let tens = match value / 10 {
        2 => "twenty",
        3 => "thirty",
        4 => "forty",
        5 => "fifty",
        6 => "sixty",
        7 => "seventy",
        8 => "eighty",
        9 => "ninety",
        _ => return small_cardinal_word(value).unwrap_or("zero").to_string(),
    };
    match value % 10 {
        0 => tens.to_string(),
        ones => format!("{tens}-{}", small_cardinal_word(ones).unwrap()),
    }
}

pub fn elapsed_time_label(years: u16, months: u8, days: u8) -> String {
    let mut parts = Vec::new();
    if years != 0 {
        parts.push(format!("{years} {}", plural("year", years)));
    }
    if months != 0 {
        parts.push(format!("{months} {}", plural("month", u16::from(months))));
    }
    if days != 0 {
        parts.push(format!("{days} {}", plural("day", u16::from(days))));
    }
    if parts.is_empty() {
        "0 days".to_string()
    } else {
        parts.join(", ")
    }
}

fn plural(label: &'static str, amount: u16) -> &'static str {
    if amount == 1 {
        label
    } else {
        match label {
            "year" => "years",
            "month" => "months",
            "day" => "days",
            _ => label,
        }
    }
}

/// One-call grid step toward a target per `endgame.md` §7: examine the slot's
/// current `(x, y)` and choose the axis with the greater remaining distance.
/// Ties prefer the X axis. Returns the new `(x, y)`. When already on the
/// target, returns the input unchanged.
pub fn endgame_step_toward_target(
    current: (isize, isize),
    target: (isize, isize),
) -> (isize, isize) {
    let dx = target.0 - current.0;
    let dy = target.1 - current.1;
    if dx == 0 && dy == 0 {
        return current;
    }
    if dx.unsigned_abs() >= dy.unsigned_abs() {
        if dx == 0 {
            (current.0, current.1 + dy.signum())
        } else {
            (current.0 + dx.signum(), current.1)
        }
    } else if dy == 0 {
        (current.0 + dx.signum(), current.1)
    } else {
        (current.0, current.1 + dy.signum())
    }
}

impl PlayState {
    pub fn enter_endgame(&mut self) -> MoveOutcome {
        self.pending_moongate = None;
        self.combat_active = false;
        self.pending_combat_actor_slot = None;
        self.pending_combat_terrain_trigger_slot = None;
        // endgame.md §10: dead party members are mutated into a present /
        // restored state for the ending tableau, with current health restored
        // from the stored maximum.
        self.restore_party_for_endgame_tableau();
        self.endgame = Some(EndgameState::awaiting_first_confirmation());
        self.message =
            "Endgame: Lord British asks whether thou hast brought his box. (Y/N)".to_string();
        MoveOutcome::EndgameEntered
    }

    /// endgame.md section 10: restore Dead travelling-party members to Good status with
    /// HP equal to their stored maximum. Non-Dead statuses keep their entry
    /// state. This mutation is cinematic only and is not committed to disk.
    pub fn restore_party_for_endgame_tableau(&mut self) {
        for member in &mut self.party {
            if character_status_for_byte(member.status)
                .is_some_and(endgame_needs_tableau_restoration)
            {
                member.status = CharacterStatus::Good.save_byte();
                member.hp = member.max_hp;
            }
        }
    }

    pub fn resolve_endgame_confirmation(&mut self, answer: bool) -> MoveOutcome {
        let Some(current) = self.endgame.clone() else {
            self.message = "No endgame confirmation is pending.".to_string();
            return MoveOutcome::Blocked;
        };
        if current.is_terminal() {
            // Victory branch advances the cinematic page-flip on every
            // keystroke until the closer is finished, then keeps the terminal
            // final panel active.
            if matches!(current.outcome, Some(EndgameOutcome::Victory)) {
                if let Some(state) = self.endgame.as_mut() {
                    if state.cinematic_is_finished() {
                        self.message = state
                            .certificate
                            .clone()
                            .unwrap_or_else(|| "The victory ending is complete.".to_string());
                    } else {
                        let banner = state.advance_cinematic();
                        self.message = if state.cinematic_is_finished() {
                            state
                                .certificate
                                .clone()
                                .unwrap_or_else(|| "The victory ending is complete.".to_string())
                        } else {
                            banner.to_string()
                        };
                    }
                    return MoveOutcome::Observed;
                }
            }
            self.message = match current.outcome {
                Some(EndgameOutcome::Victory) => current
                    .certificate
                    .unwrap_or_else(|| "The victory ending is complete.".to_string()),
                Some(EndgameOutcome::MissingBoxOrRefused) => {
                    "Lord British waits with thee in the ending tableau.".to_string()
                }
                None => unreachable!("terminal endgame has an outcome"),
            };
            return MoveOutcome::Observed;
        }

        if current.first_confirmation.is_none() {
            self.endgame = Some(EndgameState::awaiting_final_confirmation(answer));
            self.message =
                "Endgame: Lord British asks again for the sandalwood box. (Y/N)".to_string();
            return MoveOutcome::Used;
        }

        let first = current.first_confirmation.unwrap_or(false);
        let has_box = self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0;
        let certificate = endgame_certificate_summary(&self.party_leader_name(), self.clock);
        let next = EndgameState::terminal(first, answer, has_box, certificate.clone());
        self.message = match next.outcome {
            Some(EndgameOutcome::Victory) => certificate,
            Some(EndgameOutcome::MissingBoxOrRefused) => {
                "Endgame: the sandalwood box handoff failed; the ending tableau is terminal."
                    .to_string()
            }
            None => unreachable!("terminal endgame has an outcome"),
        };
        self.endgame = Some(next);
        MoveOutcome::EndgameEntered
    }

    pub fn party_leader_name(&self) -> String {
        self.party_names
            .first()
            .and_then(|name| party_name_to_string(name))
            .unwrap_or_else(|| "Avatar".to_string())
    }
}
