//! Validation of start coordinates against passability, plus tiny IO/format helpers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn validate_start(
    grid: &[u8],
    pos: (usize, usize),
    passability: Option<&TilePassability>,
) -> io::Result<()> {
    if pos.0 >= 32 || pos.1 >= 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "start coordinate must be inside 0..31, got ({}, {})",
                pos.0, pos.1
            ),
        ));
    }
    let tile = grid[pos.1 * 32 + pos.0];
    if !is_tile_walkable(tile, passability) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                tile_class(tile)
            ),
        ));
    }
    Ok(())
}

pub fn validate_dungeon_start(
    grid: &[u8],
    scene: DungeonScene,
    level: u8,
    pos: (usize, usize),
) -> io::Result<()> {
    if pos.0 >= DUNGEON_SIDE || pos.1 >= DUNGEON_SIDE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "dungeon coordinate must be inside 0..7, got ({}, {})",
                pos.0, pos.1
            ),
        ));
    }
    let tile = grid[dungeon_cell_index(level, pos.0, pos.1)];
    if !is_dungeon_walkable(tile) && !is_public_dungeon_reaction_seed(scene, level, pos, tile) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "dungeon start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                dungeon_cell_class(tile)
            ),
        ));
    }
    Ok(())
}

pub fn is_public_dungeon_reaction_seed(
    scene: DungeonScene,
    level: u8,
    pos: (usize, usize),
    tile: u8,
) -> bool {
    let is_surface_seed = level == 0 && pos == (1, 1);
    let is_underworld_seed = scene.record != 7 && level == 7 && pos == (7, 7);
    (is_surface_seed || is_underworld_seed) && matches!(tile >> 4, 0x0a | 0x0f)
}

pub fn validate_world_start_for_transport(
    grid: &[u8],
    pos: (usize, usize),
    plane: WorldPlane,
    passability: Option<&TilePassability>,
    transport: TransportState,
    damage_tiles: &[WorldDamageTileEntry],
) -> io::Result<()> {
    if pos.0 >= WORLD_SIDE || pos.1 >= WORLD_SIDE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "world coordinate must be inside 0..255, got ({}, {})",
                pos.0, pos.1
            ),
        ));
    }
    let tile = grid[world_cell_index(pos.0, pos.1)];
    if let Some(entry) = world_damage_tile_entry_at(damage_tiles, plane, pos.0, pos.1, tile) {
        if entry.effect.allows_transport(transport) {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "world start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                entry.effect.label()
            ),
        ));
    }
    if !is_tile_walkable_for_transport(tile, passability, transport) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "world start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                tile_class(tile)
            ),
        ));
    }
    Ok(())
}

pub fn pass_fail(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}

pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))
}

pub fn load_tile_passability(game_dir: &Path) -> io::Result<Option<TilePassability>> {
    let path = game_dir.join(TILE_PASSABILITY_FILE);
    match fs::read(&path) {
        Ok(bytes) => TilePassability::from_bytes(&bytes).map(Some),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io::Error::new(
            err.kind(),
            format!("{}: {err}", path.display()),
        )),
    }
}

pub fn load_look_table(game_dir: &Path) -> io::Result<LookTable> {
    parse_look2_dat(&read(&game_dir.join(LOOK2_DAT_FILE))?)
}

pub fn parse_look2_dat(bytes: &[u8]) -> io::Result<LookTable> {
    if bytes.len() < LOOK2_TABLE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{LOOK2_DAT_FILE} must be at least {LOOK2_TABLE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    let meaningful_len = if bytes.last() == Some(&DOS_EOF_MARKER) {
        bytes.len() - 1
    } else {
        bytes.len()
    };
    if meaningful_len < LOOK2_TABLE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{LOOK2_DAT_FILE} has no string pool"),
        ));
    }

    let mut descriptions = Vec::with_capacity(LOOK2_TILE_COUNT);
    for tile in 0..LOOK2_TILE_COUNT {
        let offset = u16_at(bytes, tile * 2) as usize;
        if offset < LOOK2_TABLE_LEN || offset >= meaningful_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOOK2_DAT_FILE} tile {tile} has invalid string offset {offset}"),
            ));
        }
        let raw = &bytes[offset..meaningful_len];
        let Some(end) = raw.iter().position(|byte| *byte == 0) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOOK2_DAT_FILE} tile {tile} string is not NUL-terminated"),
            ));
        };
        let raw = &raw[..end];
        if !raw.iter().all(u8::is_ascii) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOOK2_DAT_FILE} tile {tile} string is not plain ASCII"),
            ));
        }
        let description = std::str::from_utf8(raw)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
            .to_string();
        descriptions.push(description);
    }
    Ok(LookTable { descriptions })
}
