use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Town { scene: Scene, floor: i8 },
    Dungeon { scene: DungeonScene, level: u8 },
    World { plane: WorldPlane },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Player {
    pub x: usize,
    pub y: usize,
    pub facing: Direction,
    pub transport: TransportState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartyMember {
    pub slot: u8,
    pub status: u8,
    pub climb_stat: u8,
    pub mana: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub level: u8,
}

