//! Misc TSV-table data structures: blink targets, town fire targets, moongates, tile descriptions, location floor/entry-y.

use std::io;

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

/// `movement.md §4`: the base terrain bitset packs one bit per tile id,
/// read most-significant first within each byte. Bit 7 of byte
/// `(tile_id >> 3)` corresponds to tile `(byte_index * 8 + 0)`, bit 6
/// to tile `+1`, and so on through bit 0 for tile `+7`. Promote the
/// MSB-first base mask so the lookup helper does not encode `0x80` as
/// a bare literal.
pub const TILE_PASSABILITY_BIT_MSB: u8 = 0x80;
/// `movement.md §4`: low-bit mask used to extract the within-byte bit
/// index from a tile id (`tile & 7`). The tile-id-to-byte mapping
/// itself uses `tile >> TILE_PASSABILITY_BIT_INDEX_SHIFT` to pick
/// the byte; the low three bits then select which bit within that
/// byte applies.
pub const TILE_PASSABILITY_BIT_INDEX_MASK: u8 = 7;
/// `movement.md §4`: tile-id right-shift that selects the bitset
/// byte. Each base bitset byte holds eight tile-id bits, so dividing
/// the tile id by eight (= `>> 3`) yields the byte index. Anchored
/// here so the lookup helper does not encode the `3` as a bare
/// literal.
pub const TILE_PASSABILITY_BIT_INDEX_SHIFT: u32 = 3;

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
        let byte = self.bytes[(tile >> TILE_PASSABILITY_BIT_INDEX_SHIFT) as usize];
        let mask = TILE_PASSABILITY_BIT_MSB >> (tile & TILE_PASSABILITY_BIT_INDEX_MASK);
        byte & mask != 0
    }
}
