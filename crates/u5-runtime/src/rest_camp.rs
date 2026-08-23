//! Completed-long-camp recovery bounds per `systems/rest-and-camp.md` §5.
//!
//! The H-Hole-up command itself lives on `PlayState`
//! (`hole_up_command` -> `hole_up_town_command` / `rest_with_watch`),
//! which owns the prompts, the elapse loops, the §6 sleep-ambush
//! interruption, and the recovery walk. This module holds only the
//! published numeric bounds that walk consumes, so the spec values sit
//! in one place instead of as literals inside the command.
//!
//! This module previously also carried a second, parallel rest model
//! (`resolve_rest_tick` / `run_rest_session`) that nothing reachable
//! from either binary called. It advanced rest an hour at a time and
//! reported zero HP and zero MP for every tick — correct for §5's
//! town-bed and rest-with-watch paths, but wrong for the completed
//! camp, which §5 publishes as a real recovery block. Its
//! `keeping_watch` input was decorative: both branches of the `if`
//! that read it returned the same value. It has been removed in favour
//! of the live path.

/// `rest-and-camp.md §5`: completed-camp recovery requires an accepted
/// duration **greater than five hours** — "five or fewer never
/// recovers" — so the walk runs from six hours upward.
pub const COMPLETED_LONG_CAMP_MIN_HOURS: u8 = 6;
/// `rest-and-camp.md §5`: each eligible member gains a uniform random
/// `1..63` HP, rolled independently per member and capped at maximum
/// HP.
pub const COMPLETED_LONG_CAMP_HP_GAIN_MIN: u8 = 1;
pub const COMPLETED_LONG_CAMP_HP_GAIN_MAX: u8 = 63;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_long_camp_bounds_match_published_values() {
        // `rest-and-camp.md §5`: "The accepted duration is greater
        // than five hours. Five or fewer never recovers."
        assert_eq!(COMPLETED_LONG_CAMP_MIN_HOURS, 6);
        // "adds a uniform random `1..63` HP, rolled independently per
        // member"
        assert_eq!(COMPLETED_LONG_CAMP_HP_GAIN_MIN, 1);
        assert_eq!(COMPLETED_LONG_CAMP_HP_GAIN_MAX, 63);
    }
}
