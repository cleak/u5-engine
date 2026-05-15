//! Lord British outdoor-camp level-up event per
//! `rest-and-camp.md §7`. The event triggers on a 25% roll on the
//! eligible normal camp-success path; for each non-Dead member it
//! recomputes the displayed level from experience, refreshes HP, and
//! grants one primary-stat reward.

/// Trigger roll bound: `random(0, 99)`.
pub const LORD_BRITISH_CAMP_EVENT_ROLL_BOUND: u8 = 100;
/// Threshold below which the event fires (results `0..=24`, 25%).
pub const LORD_BRITISH_CAMP_EVENT_THRESHOLD: u8 = 25;

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

/// `rest-and-camp.md §7`: per-stat increase cap.
pub const LORD_BRITISH_CAMP_STAT_REWARD_CAP: u8 = 30;

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
