//! Lord British outdoor-camp level-up event per
//! `rest-and-camp.md §7`. The event triggers on a 25% roll on the
//! eligible normal camp-success path; for each non-Dead member it
//! recomputes the displayed level from experience, refreshes HP, and
//! grants one primary-stat reward.

/// Trigger roll bound: `random(0, 99)`.
pub const LORD_BRITISH_CAMP_EVENT_ROLL_BOUND: u8 = 100;
/// Threshold below which the event fires (results `0..=24`, 25%).
/// `rest-and-camp.md §7` specifies a 25% probability, i.e. one quarter
/// of the `random(0, 99)` range. Anchored to `ROLL_BOUND / 4` so the
/// threshold derives from the roll bound and the published 25%
/// probability has one source of truth.
pub const LORD_BRITISH_CAMP_EVENT_THRESHOLD: u8 = LORD_BRITISH_CAMP_EVENT_ROLL_BOUND / 4;

/// `rest-and-camp.md §7`: returns `true` when the `random(0, 99)`
/// roll selects the Lord British camp event.
pub const fn lord_british_camp_event_triggered(roll: u8) -> bool {
    roll < LORD_BRITISH_CAMP_EVENT_THRESHOLD
}

/// `rest-and-camp.md §7`: per-eligible-member level recomputation
/// from experience. Start at level 1, divide XP by 100, then
/// increment the level for each halving step while the quotient
/// remains nonzero.
///
/// Yields level 1 for `0..=99`, level 2 for `100..=199`, level 3 for
/// `200..=399`, level 4 for `400..=799`, etc. (level == 1 + bit-length
/// of `xp / 100`).
pub const fn level_for_experience(xp: u32) -> u8 {
    let mut quotient = xp / 100;
    let mut level: u8 = 1;
    while quotient != 0 {
        level = level.saturating_add(1);
        quotient >>= 1;
    }
    level
}

/// `rest-and-camp.md §7`: HP refresh value when the recomputed level
/// differs from the stored level. Both current and maximum HP are
/// set to `30 * level`.
pub const fn lord_british_camp_event_hp_for_level(level: u8) -> u16 {
    30u16.wrapping_mul(level as u16)
}

/// `rest-and-camp.md §7` primary-stat reward selected by `random(1, 3)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LordBritishCampStatReward {
    Strength,
    Dexterity,
    Intelligence,
}

/// `rest-and-camp.md §7`: per-stat increase cap. Anchored to
/// [`crate::AVATAR_STAT_MAX`] so the camp stat-reward ceiling and
/// the engine-wide Avatar stat ceiling stay one value.
pub const LORD_BRITISH_CAMP_STAT_REWARD_CAP: u8 = crate::AVATAR_STAT_MAX;

/// `rest-and-camp.md §7`: classify a `random(1, 3)` roll into the
/// stat that is incremented by one. Returns `None` for rolls outside
/// the `1..=3` range (callers should treat that as a programming
/// error rather than a no-op).
pub const fn lord_british_camp_stat_reward(roll: u8) -> Option<LordBritishCampStatReward> {
    Some(match roll {
        1 => LordBritishCampStatReward::Strength,
        2 => LordBritishCampStatReward::Dexterity,
        3 => LordBritishCampStatReward::Intelligence,
        _ => return None,
    })
}

/// `rest-and-camp.md §7`: after the Lord British camp event reaches
/// the class-refresh branch, Avatar and Mage set current MP from
/// Intelligence, Bard sets current MP from half Intelligence, and
/// other classes leave MP unchanged.
pub const fn lord_british_camp_refreshed_mana(class_byte: u8, intelligence: u8) -> Option<u8> {
    match class_byte {
        b'A' | b'M' => Some(intelligence),
        b'B' => Some(intelligence / 2),
        _ => None,
    }
}

/// `rest-and-camp.md §7.1` step 1 (u5-spec#298 Q10): "Print
/// `An_apparition!\n`".
pub const LORD_BRITISH_CAMP_APPARITION_LINE: &str = "An apparition!\n";

/// `rest-and-camp.md §7.1` step 7: after the verdict's key wait,
/// "`\n\nThe_strangely_familiar_old_man_vanishes...\n`".
pub const LORD_BRITISH_CAMP_VANISH_LINE: &str = "\n\nThe strangely familiar old man vanishes...\n";

impl LordBritishCampStatReward {
    /// `rest-and-camp.md §7.1` step 5: "exactly one of `stronger!`,
    /// `quicker!` or `wiser!` (the `random(1, 3)` roll: Strength,
    /// Dexterity, Intelligence)".
    pub const fn level_line_word(self) -> &'static str {
        match self {
            Self::Strength => "stronger!",
            Self::Dexterity => "quicker!",
            Self::Intelligence => "wiser!",
        }
    }
}

/// `rest-and-camp.md §7.1` step 5 (u5-spec#298 Q10): the level line, "only
/// when that member's level changed: `\n"Hail,_`, the member's name,
/// `!\nFor_thy_valiant_deeds,_I_shall_reward_thee!\n`, `Thou_art_now_level_`,
/// the new level as a plain decimal number, `,_and\n`, then exactly one of
/// `stronger!`, `quicker!` or `wiser!` ..., then `"_` (the closing quote and a
/// space, on the stat word's own row). Then a blocking key wait, then one line
/// feed." The key wait is the caller's.
pub fn lord_british_camp_level_line(
    name: &str,
    level: u8,
    reward: LordBritishCampStatReward,
) -> String {
    format!(
        "\n\"Hail, {name}!\nFor thy valiant deeds, I shall reward thee!\nThou art now level {level}, and\n{}\" ",
        reward.level_line_word()
    )
}

/// `rest-and-camp.md §7.1` software envelopes, in the `audio.md` tuple order
/// `(delta, initial comparison, iterations, idle, period)`.
pub const LORD_BRITISH_CAMP_APPARITION_ENVELOPE: crate::audio::EnvelopeSegment =
    crate::audio::EnvelopeSegment::new(6, 2500, 10_000, 1, 2620);
/// Step 2: "the envelope `(13, 200, 5000, 1, P)` for P = 2620, 2620, 2620,
/// 3700, 3900, 4160, in that order."
pub const LORD_BRITISH_CAMP_FIGURE_PERIODS: [u16; 6] = [2620, 2620, 2620, 3700, 3900, 4160];
pub const fn lord_british_camp_figure_note(period: u16) -> crate::audio::EnvelopeSegment {
    crate::audio::EnvelopeSegment::new(13, 200, 5_000, 1, period)
}
/// Step 4: "The published 'two-tone sting' is that pair of envelopes, both at
/// period 5500, a short one of 5,000 iterations and a long one of 60,000."
pub const LORD_BRITISH_CAMP_STING_SHORT: crate::audio::EnvelopeSegment =
    crate::audio::EnvelopeSegment::new(13, 200, 5_000, 1, 5500);
pub const LORD_BRITISH_CAMP_STING_LONG: crate::audio::EnvelopeSegment =
    crate::audio::EnvelopeSegment::new(1, 2500, 60_000, 1, 5500);
