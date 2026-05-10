//! Loaders/parsers for dungeon deeper-transition, teleport, and chest tables.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn load_dungeon_deeper_transition_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<DungeonDeeperTransitionEntry>>> {
    let path = game_dir.join(DUNGEON_DEEPER_TRANSITION_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_dungeon_deeper_transition_entries(&text).map(Some)
}

pub fn parse_dungeon_deeper_transition_entries(
    text: &str,
) -> io::Result<Vec<DungeonDeeperTransitionEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() != 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} must be: DUNGEON LEVEL X Y TO_PLANE TO_X TO_Y"
                ),
            ));
        }
        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid dungeon `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Dungeon(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} source must be a dungeon"
                    ),
                ));
            }
        };
        let level = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                ),
            ));
        }
        if level != (DUNGEON_SIDE - 1) as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} must use bottom level 7 for a deeper transition, got {level}"
                ),
            ));
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} source coordinate must be inside 0..7, got ({x}, {y})"
                ),
            ));
        }
        let to_plane = WorldPlane::from_key(parts[4]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has unknown destination plane `{}`",
                    parts[4]
                ),
            )
        })?;
        let to_x = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[5]
                ),
            )
        })? as usize;
        let to_y = parse_u8_literal(parts[6]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[6]
                ),
            )
        })? as usize;
        if entries.iter().any(|entry: &DungeonDeeperTransitionEntry| {
            entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_DEEPER_TRANSITION_TABLE_FILE} line {line_number} duplicates {} level {level} ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(DungeonDeeperTransitionEntry {
            scene,
            level,
            x,
            y,
            to_plane,
            to_x,
            to_y,
        });
    }
    Ok(entries)
}

pub fn load_dungeon_teleport_entries(game_dir: &Path) -> io::Result<Option<Vec<DungeonTeleportEntry>>> {
    let path = game_dir.join(DUNGEON_TELEPORT_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_dungeon_teleport_entries(&text).map(Some)
}

pub fn parse_dungeon_teleport_entries(text: &str) -> io::Result<Vec<DungeonTeleportEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if !(7..=8).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} must be: DUNGEON LEVEL X Y TO_LEVEL TO_X TO_Y [CELL]"
                ),
            ));
        }
        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid dungeon `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Dungeon(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} source must be a dungeon"
                    ),
                ));
            }
        };
        let level = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                ),
            ));
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} source coordinate must be inside 0..7, got ({x}, {y})"
                ),
            ));
        }
        let to_level = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid destination level `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        if to_level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} destination level must be inside 0..7, got {to_level}"
                ),
            ));
        }
        if to_level == level {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} must change dungeon level"
                ),
            ));
        }
        let to_x = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[5]
                ),
            )
        })? as usize;
        let to_y = parse_u8_literal(parts[6]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[6]
                ),
            )
        })? as usize;
        if to_x >= DUNGEON_SIDE || to_y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} destination coordinate must be inside 0..7, got ({to_x}, {to_y})"
                ),
            ));
        }
        let expected_cell = if let Some(cell) = parts.get(7) {
            Some(parse_u8_literal(cell).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} has invalid cell `{cell}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &DungeonTeleportEntry| {
            entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_TELEPORT_TABLE_FILE} line {line_number} duplicates {} level {level} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(DungeonTeleportEntry {
            scene,
            level,
            x,
            y,
            to_level,
            to_x,
            to_y,
            expected_cell,
        });
    }
    Ok(entries)
}

pub fn load_dungeon_chest_content_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<DungeonChestContentEntry>>> {
    let path = game_dir.join(DUNGEON_CHEST_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_dungeon_chest_content_entries(&text).map(Some)
}

pub fn parse_dungeon_chest_content_entries(text: &str) -> io::Result<Vec<DungeonChestContentEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() < 7 || (parts.len() - 5) % 2 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} must be: DUNGEON LEVEL X Y CELL ITEM AMOUNT [ITEM AMOUNT ...]"
                ),
            ));
        }
        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} has invalid dungeon `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Dungeon(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_CHEST_TABLE_FILE} line {line_number} source must be a dungeon"
                    ),
                ));
            }
        };
        let level = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                ),
            ));
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} coordinate must be inside 0..7, got ({x}, {y})"
                ),
            ));
        }
        let expected_cell = if parts[4] == "*" {
            None
        } else {
            Some(parse_u8_literal(parts[4]).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{DUNGEON_CHEST_TABLE_FILE} line {line_number} has invalid cell `{}`: {err}",
                        parts[4]
                    ),
                )
            })?)
        };
        let mut grants = Vec::new();
        for pair in parts[5..].chunks_exact(2) {
            let grant =
                parse_tile_get_grant(DUNGEON_CHEST_TABLE_FILE, line_number, pair[0], pair[1])?;
            if grants
                .iter()
                .any(|existing: &ObjectPickupGrant| existing.kind == grant.kind)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_CHEST_TABLE_FILE} line {line_number} duplicates {} grant",
                        grant.kind.label()
                    ),
                ));
            }
            grants.push(grant);
        }
        if entries.iter().any(|entry: &DungeonChestContentEntry| {
            entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_CHEST_TABLE_FILE} line {line_number} duplicates {} level {level} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(DungeonChestContentEntry {
            scene,
            level,
            x,
            y,
            expected_cell,
            grants,
        });
    }
    Ok(entries)
}
