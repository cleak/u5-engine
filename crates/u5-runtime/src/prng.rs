//! Shared deterministic game-logic PRNG.

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
