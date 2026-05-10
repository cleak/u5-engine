//! Transport state (foot/horse/ship/skiff/carpet/balloon), pending-vehicle acquisitions, and board-vehicle candidates.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportState {
    #[default]
    Foot,
    Horse {
        type_byte: u8,
        tile: u8,
    },
    Ship {
        type_byte: u8,
        tile: u8,
        sails_hoisted: bool,
        hull: u8,
        skiffs: u8,
    },
    Skiff {
        type_byte: u8,
        tile: u8,
    },
    Carpet {
        type_byte: u8,
        tile: u8,
    },
    Balloon {
        type_byte: u8,
        tile: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingVehicleAcquisition {
    Frigate { x: usize, y: usize, skiffs: u8 },
    Skiff { x: usize, y: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardVehicleCandidate {
    pub slot: usize,
    pub transport: TransportState,
    pub blocked_by_occupant: bool,
}

impl PendingVehicleAcquisition {
    pub fn active_object(self, z: i8) -> ActiveObject {
        match self {
            Self::Frigate { x, y, skiffs } => ActiveObject {
                type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
                aux3: skiffs,
            },
            Self::Skiff { x, y } => ActiveObject {
                type_byte: FIRST_PLAYABLE_SKIFF_TILE,
                tile: FIRST_PLAYABLE_SKIFF_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
        }
    }
}

impl TransportState {
    pub fn is_foot(self) -> bool {
        matches!(self, Self::Foot)
    }

    pub fn is_horse(self) -> bool {
        matches!(self, Self::Horse { .. })
    }

    pub fn is_ship_under_sail(self) -> bool {
        matches!(
            self,
            Self::Ship {
                sails_hoisted: true,
                ..
            }
        )
    }

    pub fn is_balloon(self) -> bool {
        matches!(self, Self::Balloon { .. })
    }

    pub fn avatar_tile(self) -> u8 {
        match self {
            Self::Foot => PLAYER_TILE,
            Self::Horse { tile, .. }
            | Self::Ship { tile, .. }
            | Self::Skiff { tile, .. }
            | Self::Carpet { tile, .. }
            | Self::Balloon { tile, .. } => tile,
        }
    }

    pub fn kind_name(self) -> &'static str {
        match self {
            Self::Foot => "foot",
            Self::Horse { .. } => "horse",
            Self::Ship { .. } => "ship",
            Self::Skiff { .. } => "skiff",
            Self::Carpet { .. } => "carpet",
            Self::Balloon { .. } => "balloon",
        }
    }

    pub fn status_label(self) -> String {
        match self {
            Self::Foot => "foot".to_string(),
            Self::Horse { tile, .. } => format!("horse tile {tile}"),
            Self::Ship {
                tile,
                sails_hoisted,
                hull,
                skiffs,
                ..
            } => format!(
                "ship tile {tile} sails={} hull={hull} skiffs={skiffs}",
                if sails_hoisted { "hoisted" } else { "furled" }
            ),
            Self::Skiff { tile, .. } => format!("skiff tile {tile}"),
            Self::Carpet { tile, .. } => format!("magic carpet tile {tile}"),
            Self::Balloon { tile, .. } => format!("balloon tile {tile}"),
        }
    }

    pub fn can_board(self, target: Self) -> bool {
        match target {
            Self::Ship { .. } => {
                matches!(self, Self::Foot | Self::Ship { .. } | Self::Skiff { .. })
            }
            Self::Horse { .. } | Self::Skiff { .. } | Self::Carpet { .. } => self.is_foot(),
            Self::Balloon { .. } => false,
            Self::Foot => false,
        }
    }

    pub fn save_marker(self) -> u8 {
        match self {
            Self::Foot => FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER,
            Self::Horse { tile, .. }
            | Self::Ship { tile, .. }
            | Self::Skiff { tile, .. }
            | Self::Carpet { tile, .. } => tile,
            Self::Balloon { .. } => FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER,
        }
    }

    pub fn parked_object(self, x: usize, y: usize, z: i8) -> Option<ActiveObject> {
        let (type_byte, tile, aux1, aux3) = match self {
            Self::Foot => return None,
            Self::Horse {
                type_byte, tile, ..
            }
            | Self::Skiff {
                type_byte, tile, ..
            }
            | Self::Carpet {
                type_byte, tile, ..
            }
            | Self::Balloon {
                type_byte, tile, ..
            } => (type_byte, tile, 0, 0),
            Self::Ship {
                type_byte,
                tile,
                hull,
                skiffs,
                ..
            } => (type_byte, tile, hull, skiffs),
        };
        Some(ActiveObject {
            type_byte,
            tile,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1,
            aux3,
        })
    }

    pub fn append_ship_auxiliary_warnings(self, message: &mut String) {
        if let Self::Ship { hull: 0, .. } = self {
            message.push(' ');
            message.push_str(SHIP_BADLY_DAMAGED_WARNING);
        }
        if let Self::Ship { skiffs: 0, .. } = self {
            message.push(' ');
            message.push_str(SHIP_NO_SKIFFS_WARNING);
        }
    }
}

