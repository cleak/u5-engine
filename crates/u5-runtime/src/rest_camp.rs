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

use std::io;
use std::path::Path;

use crate::{DATA_OVL_FILENAME, read_disk_file};

/// `rest-and-camp.md §5`: completed-camp recovery requires an accepted
/// duration **greater than five hours** — "five or fewer never
/// recovers" — so the walk runs from six hours upward.
pub const COMPLETED_LONG_CAMP_MIN_HOURS: u8 = 6;
/// `rest-and-camp.md §5`: each eligible member gains a uniform random
/// `1..63` HP, rolled independently per member and capped at maximum
/// HP.
pub const COMPLETED_LONG_CAMP_HP_GAIN_MIN: u8 = 1;
pub const COMPLETED_LONG_CAMP_HP_GAIN_MAX: u8 = 63;

/// `rest-and-camp.md §5` camp cooldown counter arming value. The
/// counter "is set to 14 whenever a camp completes and is reduced by
/// one, floored at zero, at every hour rollover. A second camp begun
/// inside fourteen game hours of the previous one therefore prints the
/// no-effect line and recovers nothing."
pub const COMPLETED_LONG_CAMP_COOLDOWN_HOURS: u8 = 14;

/// `rest-and-camp.md §5`: shipped `DATA.OVL` offsets of the mutually
/// exclusive completed-camp result lines. Runtime code reads these
/// NUL-terminated strings rather than copying their copyrighted text
/// into the clean repository.
pub const CAMP_SUCCESS_MESSAGE_OFFSET: usize = 0x41fc;
pub const CAMP_NO_EFFECT_MESSAGE_OFFSET: usize = 0x420b;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampResultMessages {
    pub success: String,
    pub no_effect: String,
}

fn parse_nul_terminated_ascii_at(bytes: &[u8], offset: usize, label: &str) -> io::Result<String> {
    let tail = bytes.get(offset..).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{DATA_OVL_FILENAME} is too short for the camp {label} line at 0x{offset:04X}"),
        )
    })?;
    let length = tail.iter().position(|byte| *byte == 0).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{DATA_OVL_FILENAME} camp {label} line at 0x{offset:04X} has no NUL terminator"
            ),
        )
    })?;
    if length == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{DATA_OVL_FILENAME} camp {label} line at 0x{offset:04X} is empty"),
        ));
    }
    let line = std::str::from_utf8(&tail[..length]).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{DATA_OVL_FILENAME} camp {label} line at 0x{offset:04X} is not ASCII text"),
        )
    })?;
    // `text-output.md` uses literal LF as the hard-newline control,
    // and both published camp records end with one. Reject every other
    // control byte so a bad offset cannot masquerade as message text.
    if !line.is_ascii() || line.chars().any(|ch| ch.is_control() && ch != '\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{DATA_OVL_FILENAME} camp {label} line at 0x{offset:04X} contains a control byte"
            ),
        ));
    }
    Ok(line.to_string())
}

pub fn parse_camp_result_messages(bytes: &[u8]) -> io::Result<CampResultMessages> {
    Ok(CampResultMessages {
        success: parse_nul_terminated_ascii_at(bytes, CAMP_SUCCESS_MESSAGE_OFFSET, "success")?,
        no_effect: parse_nul_terminated_ascii_at(
            bytes,
            CAMP_NO_EFFECT_MESSAGE_OFFSET,
            "no-effect",
        )?,
    })
}

pub fn load_camp_result_messages(game_dir: &Path) -> io::Result<CampResultMessages> {
    let bytes = read_disk_file(&game_dir.join(DATA_OVL_FILENAME))?;
    parse_camp_result_messages(&bytes)
}

/// `rest-and-camp.md §5`: the completed-camp recovery walk runs only
/// while the camp cooldown counter reads zero.
pub const fn camp_cooldown_blocks_recovery(cooldown: u8) -> bool {
    cooldown != 0
}

/// `rest-and-camp.md §5`: the camp cooldown counter is "reduced by one,
/// floored at zero, at every hour rollover".
pub const fn camp_cooldown_after_hour_rollover(cooldown: u8) -> u8 {
    cooldown.saturating_sub(1)
}

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

    #[test]
    fn camp_cooldown_arms_at_fourteen_and_decays_one_per_hour_floored_at_zero() {
        // `rest-and-camp.md §5`: "set to 14 whenever a camp completes
        // and ... reduced by one, floored at zero, at every hour
        // rollover."
        assert_eq!(COMPLETED_LONG_CAMP_COOLDOWN_HOURS, 14);
        assert!(camp_cooldown_blocks_recovery(
            COMPLETED_LONG_CAMP_COOLDOWN_HOURS
        ));
        assert!(camp_cooldown_blocks_recovery(1));
        assert!(!camp_cooldown_blocks_recovery(0));

        // Fourteen rollovers take a freshly armed counter to zero, and
        // the fourteenth is the first hour a second camp can recover.
        let mut cooldown = COMPLETED_LONG_CAMP_COOLDOWN_HOURS;
        for elapsed in 1..=COMPLETED_LONG_CAMP_COOLDOWN_HOURS {
            cooldown = camp_cooldown_after_hour_rollover(cooldown);
            assert_eq!(
                cooldown,
                COMPLETED_LONG_CAMP_COOLDOWN_HOURS - elapsed,
                "after {elapsed} hour rollover(s)"
            );
        }
        assert_eq!(cooldown, 0);
        // Floored, not wrapped.
        assert_eq!(camp_cooldown_after_hour_rollover(0), 0);
    }

    #[test]
    fn camp_result_lines_are_read_from_the_two_published_data_ovl_offsets() {
        let mut bytes = vec![0; CAMP_NO_EFFECT_MESSAGE_OFFSET + 16];
        bytes[CAMP_SUCCESS_MESSAGE_OFFSET..CAMP_SUCCESS_MESSAGE_OFFSET + 9]
            .copy_from_slice(b"RESTED!\n\0");
        bytes[CAMP_NO_EFFECT_MESSAGE_OFFSET..CAMP_NO_EFFECT_MESSAGE_OFFSET + 12]
            .copy_from_slice(b"NO EFFECT!\n\0");

        assert_eq!(
            parse_camp_result_messages(&bytes).unwrap(),
            CampResultMessages {
                success: "RESTED!\n".to_string(),
                no_effect: "NO EFFECT!\n".to_string(),
            }
        );
    }
}
