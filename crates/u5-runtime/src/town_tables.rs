//! Data structures for the town TSV tables (fire sources, pushables, get-tiles, rest beds, stairs, trap doors, exits, locks).

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownFireSourceEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownPushableEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownGetTileEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub replacement_tile: u8,
    pub expected_tile: Option<u8>,
    pub grant: Option<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownRestBedEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownStairKind {
    Up,
    Down,
    Both,
}

impl TownStairKind {
    pub fn allows(self, intent: ClimbIntent) -> bool {
        matches!(
            (self, intent),
            (Self::Up, ClimbIntent::Up) | (Self::Down, ClimbIntent::Down) | (Self::Both, _)
        )
    }

    pub fn intents(self) -> &'static [ClimbIntent] {
        match self {
            Self::Up => &[ClimbIntent::Up],
            Self::Down => &[ClimbIntent::Down],
            Self::Both => &[ClimbIntent::Up, ClimbIntent::Down],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownStairEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub kind: TownStairKind,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownTrapDoorEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub to_floor: i8,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownExitTileEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownLockKind {
    Locked,
    Magic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownLockEntry {
    pub scene: Scene,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub locked_tile: u8,
    pub unlocked_tile: u8,
    pub kind: TownLockKind,
}
