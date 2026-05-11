//! Animation clock, active object, phase ticking, active-ship wind state.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationClock {
    pub frame: u8,
    pub moongate_frame: u8,
}

impl AnimationClock {
    pub fn tick_static_tiles(&mut self) {
        self.frame = self.frame.wrapping_add(1) & 3;
    }

    pub fn tick_moongate(&mut self) {
        self.moongate_frame = self.moongate_frame.wrapping_add(1) & (MOONGATE_ANIMATION_FRAMES - 1);
    }

    pub fn resolve_static_tile(self, tile: u8) -> u8 {
        // Per the tile-catalog spec (Section 4): the animator shifts the
        // displayed sprite within a fixed family run while preserving each
        // cell's per-tile identity offset. Water is 3 frames (deep water /
        // water / shoals); swamp is a separate static terrain. Lava / fire
        // / wind families remain 4-frame.
        if let Some((base, cycle)) = static_tile_animation_family(tile) {
            let offset = (tile - base) % cycle;
            let advanced = (offset + (self.frame % cycle)) % cycle;
            base + advanced
        } else {
            tile
        }
    }

    pub fn resolve_moongate_tile(self) -> u8 {
        MOONGATE_TILE_BASE + self.moongate_frame
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveObject {
    pub type_byte: u8,
    pub tile: u8,
    pub x: usize,
    pub y: usize,
    pub z: i8,
    pub phase: u8,
    pub aux1: u8,
    pub aux3: u8,
}

impl ActiveObject {
    pub fn empty() -> Self {
        Self {
            type_byte: 0,
            tile: 0,
            x: 0,
            y: 0,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        }
    }

    pub fn moonstone_pickup(slot_index: usize, x: usize, y: usize, z: i8) -> Self {
        Self {
            type_byte: FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE,
            tile: FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: slot_index as u8,
            aux3: MOONSTONE_PICKUP_AUX3,
        }
    }

    pub fn free(&mut self) {
        self.type_byte = 0;
    }

    pub fn tick_phase(&mut self) -> PhaseTick {
        let low = self.phase & 0x0f;
        if low == STEADY_PHASE {
            PhaseTick::Steady
        } else if low > 0 {
            self.phase = (self.phase & 0xf0) | (low - 1);
            PhaseTick::Countdown
        } else {
            PhaseTick::DecisionPoint
        }
    }

    pub fn is_player(self) -> bool {
        self.type_byte == PLAYER_TILE
    }

    pub fn is_player_phantom(self) -> bool {
        self.type_byte == PLAYER_NPC_SENTINEL_TYPE
    }

    pub fn is_empty(self) -> bool {
        self.type_byte == 0
    }

    pub fn moonstone_slot_index(self) -> Option<usize> {
        let slot_index = self.aux1 as usize;
        (self.type_byte == FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE
            && self.tile == FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE
            && self.aux3 == MOONSTONE_PICKUP_AUX3
            && slot_index < MOONSTONE_SLOT_COUNT)
            .then_some(slot_index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseTick {
    Steady,
    Countdown,
    DecisionPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveShipWind {
    None,
    Stalled,
    Drifted,
}
