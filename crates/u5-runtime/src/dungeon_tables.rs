//! Data structures for the dungeon TSV tables (deeper transitions, teleports, chests, wind, exits, doors, secret doors).

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonDeeperTransitionEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub to_plane: WorldPlane,
    pub to_x: usize,
    pub to_y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonTeleportEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub to_level: u8,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonChestContentEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub expected_cell: Option<u8>,
    pub grants: Vec<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonWindTileEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonExitTileEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonDoorEntry {
    pub scene: DungeonScene,
    pub level: u8,
    pub x: usize,
    pub y: usize,
    pub open_cell: u8,
    pub expected_cell: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretDoorEntry {
    Town {
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        reveal_tile: u8,
        expected_tile: Option<u8>,
    },
    Dungeon {
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        reveal_cell: u8,
        expected_cell: Option<u8>,
    },
}
