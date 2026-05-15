//! Shared deterministic game-logic PRNG.

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
        low.wrapping_add((state & 0x7fff) % width)
    }

    pub fn next_range_u8(&mut self, low: u8, high: u8) -> u8 {
        self.next_range_u16(u16::from(low), u16::from(high)) as u8
    }
}

pub fn u5_prng_advance_state(state: u16) -> u16 {
    let summed = state.wrapping_add(0x9248);
    let rotated = summed.rotate_right(3);
    (rotated ^ 0x9248).wrapping_add(0x0011)
}

pub fn u5_prng_range_u16(state: &mut u16, low: u16, high: u16) -> u16 {
    let mut prng = U5Prng::new(*state);
    let value = prng.next_range_u16(low, high);
    *state = prng.state();
    value
}
