//! Shared deterministic game-logic PRNG.

use chrono::{Local, Timelike};

/// `prng.md §2` state-advance constants. The advance is:
///   1. `state.wrapping_add(PRNG_STATE_ADD)`
///   2. rotate right by `PRNG_STATE_ROTATE_BITS`
///   3. XOR with `PRNG_STATE_ADD`
///   4. `wrapping_add(PRNG_STATE_FINAL_BIAS)`
/// Result becomes the new 16-bit state word.
pub const PRNG_STATE_ADD: u16 = 0x9248;
pub const PRNG_STATE_ROTATE_BITS: u32 = 3;
pub const PRNG_STATE_FINAL_BIAS: u16 = 0x0011;
/// `prng.md §2` non-negative mask. The 16-bit advanced state is
/// masked to its low fifteen bits before the modulo reduction so
/// the range reduction always sees a non-negative value, matching
/// the original signed-arithmetic safety.
pub const PRNG_NON_NEGATIVE_MASK: u16 = 0x7FFF;
/// `prng.md §3` shipped initialized value. The gameplay PRNG state is
/// not save-backed: zero is the pre-boot value, and the intro replaces
/// it with a host-clock-derived seed before presenting the main menu.
pub const DEFAULT_PRNG_STATE: u16 = 0;
/// `prng.md §3` fixed XOR and final twelve-bit mask used by every
/// host-clock seed assignment.
pub const PRNG_HOST_CLOCK_XOR: u16 = 0x91EB;
pub const PRNG_HOST_CLOCK_MASK: u16 = 0x0FFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostClockTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub hundredth: u8,
}

impl HostClockTime {
    /// Take the one local time-of-day sample used by a host-clock seed event.
    pub fn now() -> Self {
        let now = Local::now();
        Self {
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: now.second() as u8,
            hundredth: (now.nanosecond() / 10_000_000) as u8,
        }
    }

    pub const fn seed(self) -> u16 {
        host_clock_prng_seed(self.hour, self.minute, self.second, self.hundredth)
    }
}

/// `prng.md §3` exact host-clock seed equation. Each shifted field is
/// truncated to one byte before the two packed words are added with 16-bit
/// wrapping; XOR and the twelve-bit mask follow the addition.
pub const fn host_clock_prng_seed(hour: u8, minute: u8, second: u8, hundredth: u8) -> u16 {
    let seconds_and_hundredths = (((second.wrapping_shl(3)) as u16) << 8) | hundredth as u16;
    let hours_and_minutes = (((hour.wrapping_shl(1)) as u16) << 8) | minute.wrapping_shl(2) as u16;
    (seconds_and_hundredths.wrapping_add(hours_and_minutes) ^ PRNG_HOST_CLOCK_XOR)
        & PRNG_HOST_CLOCK_MASK
}

/// Sample the host clock once and transform the four fields as one seed event.
pub fn host_clock_prng_seed_now() -> u16 {
    HostClockTime::now().seed()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct U5Prng {
    state: u16,
}

impl U5Prng {
    pub fn new(state: u16) -> Self {
        Self { state }
    }

    pub fn state(self) -> u16 {
        self.state
    }

    pub fn set_state(&mut self, state: u16) {
        self.state = state;
    }

    pub fn advance(&mut self) -> u16 {
        self.state = u5_prng_advance_state(self.state);
        self.state
    }

    pub fn next_range_u16(&mut self, low: u16, high: u16) -> u16 {
        let state = self.advance();
        let width = high.wrapping_sub(low).wrapping_add(1);
        low.wrapping_add((state & PRNG_NON_NEGATIVE_MASK) % width)
    }

    pub fn next_range_u8(&mut self, low: u8, high: u8) -> u8 {
        self.next_range_u16(u16::from(low), u16::from(high)) as u8
    }
}

pub fn u5_prng_advance_state(state: u16) -> u16 {
    let summed = state.wrapping_add(PRNG_STATE_ADD);
    let rotated = summed.rotate_right(PRNG_STATE_ROTATE_BITS);
    (rotated ^ PRNG_STATE_ADD).wrapping_add(PRNG_STATE_FINAL_BIAS)
}

pub fn u5_prng_range_u16(state: &mut u16, low: u16, high: u16) -> u16 {
    let mut prng = U5Prng::new(*state);
    let value = prng.next_range_u16(low, high);
    *state = prng.state();
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_clock_seed_matches_all_published_vectors() {
        assert_eq!(host_clock_prng_seed(0, 0, 0, 0), 0x01EB);
        assert_eq!(host_clock_prng_seed(12, 34, 56, 78), 0x093D);
        assert_eq!(host_clock_prng_seed(23, 59, 59, 99), 0x06A4);
    }

    #[test]
    fn host_clock_seed_truncates_shifted_fields_before_packing() {
        let seconds = 59u8.wrapping_shl(3);
        let minutes = 59u8.wrapping_shl(2);
        assert_eq!(seconds, 0xD8);
        assert_eq!(minutes, 0xEC);
        assert_eq!(host_clock_prng_seed(23, 59, 59, 99), 0x06A4);
    }
}
