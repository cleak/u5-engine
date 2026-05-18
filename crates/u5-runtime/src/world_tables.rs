//! Data structures for the world TSV tables (locations, plane transitions, get-tiles, pickups, waterfalls, damage, encounters, shrines).

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldLocationEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub target: PlayTarget,
    pub town_entry_y: Option<usize>,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShrineEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub virtue: ShrineVirtue,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodexUrnEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPlaneTransitionEntry {
    pub from_plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub to_plane: WorldPlane,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGetTileEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub replacement_tile: u8,
    pub expected_tile: Option<u8>,
    pub grant: Option<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectPickupKind {
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
}

impl ObjectPickupKind {
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_lowercase().as_str() {
            "food" | "ration" | "rations" => Some(Self::Food),
            "gold" | "coin" | "coins" => Some(Self::Gold),
            "key" | "keys" => Some(Self::Keys),
            "gem" | "gems" => Some(Self::Gems),
            "torch" | "torches" => Some(Self::Torches),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Gold => "gold",
            Self::Keys => "keys",
            Self::Gems => "gems",
            Self::Torches => "torches",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectPickupGrant {
    pub kind: ObjectPickupKind,
    pub amount: u8,
}

pub fn tile_get_message(
    prefix: String,
    replacement_tile: u8,
    grant: Option<ObjectPickupGrant>,
) -> String {
    match grant {
        Some(grant) => format!(
            "{prefix}; replaced with tile {replacement_tile}; added {} {}.",
            grant.amount,
            grant.kind.label()
        ),
        None => format!("{prefix}; replaced with tile {replacement_tile}."),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectPickupEntry {
    pub target: PlayTarget,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub kind: ObjectPickupKind,
    pub amount: u8,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldWaterfallEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
    pub steps: u8,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldWaterfallSweep {
    Settled {
        steps: u8,
    },
    PlaneTransition {
        steps: u8,
        entry: WorldPlaneTransitionEntry,
    },
    Moongate {
        steps: u8,
        entry: MoongateEntry,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldDamageEffect {
    Lava,
    Drowning,
}

impl WorldDamageEffect {
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_uppercase().as_str() {
            "LAVA" => Some(Self::Lava),
            "DROWNING" | "WATER" => Some(Self::Drowning),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Lava => "lava",
            Self::Drowning => "drowning",
        }
    }

    pub fn allows_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(
                transport,
                TransportState::Carpet { .. } | TransportState::Balloon { .. }
            ),
            Self::Drowning => matches!(
                transport,
                TransportState::Foot
                    | TransportState::Ship { .. }
                    | TransportState::Skiff { .. }
                    | TransportState::Carpet { .. }
                    | TransportState::Balloon { .. }
            ),
        }
    }

    pub fn damages_transport(self, transport: TransportState) -> bool {
        match self {
            Self::Lava => matches!(transport, TransportState::Carpet { .. }),
            Self::Drowning => matches!(transport, TransportState::Foot),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldDamageTileEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub effect: WorldDamageEffect,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldEncounterEntry {
    pub plane: WorldPlane,
    pub tile: u8,
    pub threshold: u8,
    pub type_byte: u8,
    pub dx: i8,
    pub dy: i8,
    pub phase: u8,
}
