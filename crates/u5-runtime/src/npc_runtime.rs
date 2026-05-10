//! Runtime NPC + door tracker + location markers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeNpc {
    pub slot: usize,
    pub type_byte: u8,
    pub dialog_id: u8,
    pub schedule: [u8; 16],
    pub x: usize,
    pub y: usize,
    pub z: u8,
    pub cached_wp: usize,
    pub active_object: Option<usize>,
    pub player_phantom: bool,
}

impl RuntimeNpc {
    pub fn from_slot(slot: &NpcSlot, hour: u8) -> Self {
        let wp = waypoint_for_hour(&slot.schedule, hour);
        Self {
            slot: slot.slot,
            type_byte: slot.type_byte,
            dialog_id: slot.dialog_id,
            schedule: slot.schedule,
            x: slot.schedule[3 + wp] as usize,
            y: slot.schedule[6 + wp] as usize,
            z: slot.schedule[9 + wp],
            cached_wp: wp,
            active_object: None,
            player_phantom: false,
        }
    }

    pub fn from_player_phantom(x: usize, y: usize, z: u8, hour: u8) -> Self {
        let mut schedule = [0u8; 16];
        for wp in 0..3 {
            schedule[3 + wp] = x as u8;
            schedule[6 + wp] = y as u8;
            schedule[9 + wp] = z;
        }
        let cached_wp = waypoint_for_hour(&schedule, hour);
        Self {
            slot: PLAYER_NPC_SLOT,
            type_byte: PLAYER_NPC_SENTINEL_TYPE,
            dialog_id: PLAYER_NPC_DIALOG_ID,
            schedule,
            x,
            y,
            z,
            cached_wp,
            active_object: None,
            player_phantom: true,
        }
    }

    pub fn is_player_phantom(&self) -> bool {
        self.player_phantom
    }

    pub fn sync_player_phantom_floor(&mut self, floor: u8, hour: u8) {
        self.z = floor;
        for wp in 0..3 {
            self.schedule[9 + wp] = floor;
        }
        self.cached_wp = waypoint_for_hour(&self.schedule, hour);
    }

    pub fn waypoint_position(&self, wp: usize) -> (usize, usize, u8) {
        (
            self.schedule[3 + wp] as usize,
            self.schedule[6 + wp] as usize,
            self.schedule[9 + wp],
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorTracker {
    pub previous_tile: u8,
    pub x: usize,
    pub y: usize,
    pub turns_remaining: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationMarkers {
    pub npc_markers: Vec<(usize, usize)>,
    pub spawn_markers: Vec<(usize, usize)>,
}
