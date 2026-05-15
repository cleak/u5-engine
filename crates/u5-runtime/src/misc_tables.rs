//! Misc TSV-table data structures: blink targets, town fire targets, moongates, tile descriptions, location floor/entry-y.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlinkTargetEntry {
    pub target: PlayTarget,
    pub floor: i8,
    pub from_x: usize,
    pub from_y: usize,
    pub direction: Direction,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_from_tile: Option<u8>,
    pub expected_to_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownFireTarget {
    Object { slot: usize, object: ActiveObject },
    Door { x: usize, y: usize, tile: u8 },
    Wall { x: usize, y: usize, tile: u8 },
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoongateEntry {
    pub x: usize,
    pub y: usize,
    pub destination_plane: WorldPlane,
    pub destination_x: usize,
    pub destination_y: usize,
    pub active_hours: Option<(u8, u8)>,
    pub expected_tile: Option<u8>,
}

impl MoongateEntry {
    pub fn is_active_at(self, hour: u8) -> bool {
        match self.active_hours {
            Some((start, end)) if start <= end => (start..=end).contains(&hour),
            Some((start, end)) => hour >= start || hour <= end,
            None => true,
        }
    }

    pub fn is_single_ended(self) -> bool {
        self.destination_x == u8::MAX as usize && self.destination_y == u8::MAX as usize
    }

    pub fn matches_origin_tile(self, tile: u8) -> bool {
        self.expected_tile
            .map_or(true, |expected_tile| expected_tile == tile)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookTable {
    pub descriptions: Vec<String>,
}

impl LookTable {
    pub fn description(&self, tile: usize) -> Option<&str> {
        self.descriptions.get(tile).map(String::as_str)
    }

    pub fn is_sentinel(&self, description: &str) -> bool {
        self.description(0)
            .map(|sentinel| description == sentinel)
            .unwrap_or(false)
    }
}

pub fn blackthorn_karma_record_index(moral_standing: u8) -> usize {
    usize::from(moral_standing / 20).min(4)
}

pub fn lord_british_camp_karma_record_index(moral_standing: u8) -> usize {
    let band = moral_standing / 20;
    if band >= 4 { 5 } else { usize::from(band) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationFloorEntry {
    pub scene: Scene,
    pub base_page: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationEntryYEntry {
    pub scene: Scene,
    pub y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePassability {
    pub bytes: [u8; TILE_PASSABILITY_LEN],
}

impl TilePassability {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != TILE_PASSABILITY_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TILE_PASSABILITY_FILE} must contain exactly {TILE_PASSABILITY_LEN} bytes, got {}",
                    bytes.len()
                ),
            ));
        }
        let mut out = [0; TILE_PASSABILITY_LEN];
        out.copy_from_slice(bytes);
        Ok(Self { bytes: out })
    }

    pub fn is_passable(&self, tile: u8) -> bool {
        let byte = self.bytes[(tile >> 3) as usize];
        let mask = 0x80u8 >> (tile & 7);
        byte & mask != 0
    }
}
