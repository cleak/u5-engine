//! Runtime NPC + door tracker + location markers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

/// Per `npc-schedules.md §7`: NPC schedule state-machine values `0..=8`.
/// Empty / idle / in-plane move / replaying queue / descend toward target /
/// ascend toward target / climb up off-floor / climb down off-floor /
/// parked off-floor.
pub const NPC_STATE_EMPTY: u8 = 0;
pub const NPC_STATE_IDLE: u8 = 1;
pub const NPC_STATE_INPLANE_MOVE: u8 = 2;
pub const NPC_STATE_REPLAY_QUEUE: u8 = 3;
pub const NPC_STATE_DESCEND_TOWARD_TARGET: u8 = 4;
pub const NPC_STATE_ASCEND_TOWARD_TARGET: u8 = 5;
pub const NPC_STATE_CLIMB_UP_OFF_FLOOR: u8 = 6;
pub const NPC_STATE_CLIMB_DOWN_OFF_FLOOR: u8 = 7;
pub const NPC_STATE_PARKED_OFF_FLOOR: u8 = 8;
/// `npc-schedules.md §6`: classify a real boundary transition into the
/// movement state byte the per-tick walker switches on, given the NPC's
/// current floor, the new waypoint's floor, and the location's current
/// floor. Returns the state byte from the floor-classification table:
///   - both equal → 2 (in-plane move)
///   - equal/below → 7 (climb-down off this floor)
///   - equal/above → 6 (climb-up off this floor)
///   - below/equal → 5 (ascend toward target floor)
///   - above/equal → 4 (descend toward target floor)
///   - neither/neither → 8 (parked off-floor / replan needed)
/// "Below" means a floor index numerically greater than the map's
/// current floor; "above" means numerically smaller. Caller still
/// applies the already-on-waypoint short-circuit and only invokes the
/// classifier when a real transition has been detected.
pub const fn schedule_floor_state(
    npc_floor: u8,
    target_floor: u8,
    map_floor: u8,
) -> u8 {
    let npc_eq = npc_floor == map_floor;
    let target_eq = target_floor == map_floor;
    if npc_eq && target_eq {
        NPC_STATE_INPLANE_MOVE
    } else if npc_eq && target_floor > map_floor {
        NPC_STATE_CLIMB_DOWN_OFF_FLOOR
    } else if npc_eq && target_floor < map_floor {
        NPC_STATE_CLIMB_UP_OFF_FLOOR
    } else if target_eq && npc_floor > map_floor {
        NPC_STATE_ASCEND_TOWARD_TARGET
    } else if target_eq && npc_floor < map_floor {
        NPC_STATE_DESCEND_TOWARD_TARGET
    } else {
        NPC_STATE_PARKED_OFF_FLOOR
    }
}

/// Per `npc-schedules.md §4`: stuck counter threshold for forced replan.
/// When the counter exceeds this value the move queue is reset to inactive
/// and a fresh route is requested on a later tick.
pub const NPC_STUCK_REPLAN_THRESHOLD: u16 = 3;

/// Per `npc-schedules.md §10` movement constraint: dynamic-obstacle scan
/// radius. An occupied active-object cell is reported as blocked only when
/// the occupant is within Manhattan distance less than this value from the
/// NPC's runtime destination. Cells outside this radius are treated as
/// walkable by the pathfinding workspace.
pub const NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS: usize = 4;

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
