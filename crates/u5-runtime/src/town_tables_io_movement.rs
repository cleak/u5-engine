//! Loaders/parsers for town stair, trap-door, exit, and lock tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_town_stair_entries(game_dir: &Path) -> io::Result<Option<Vec<TownStairEntry>>> {
    let path = game_dir.join(TOWN_STAIR_TABLE_FILE);
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
    parse_town_stair_entries(&text).map(Some)
}

pub fn parse_town_stair_entries(text: &str) -> io::Result<Vec<TownStairEntry>> {
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
        if !(5..=6).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y UP|DOWN|BOTH [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_STAIR_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let kind = parse_town_stair_kind(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} has invalid stair direction `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let expected_tile = if let Some(tile) = parts.get(5) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_STAIR_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownStairEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_STAIR_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownStairEntry {
            scene,
            floor,
            x,
            y,
            kind,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn parse_town_stair_kind(value: &str) -> io::Result<TownStairKind> {
    match value.to_ascii_uppercase().as_str() {
        "UP" | "<" => Ok(TownStairKind::Up),
        "DOWN" | ">" => Ok(TownStairKind::Down),
        "BOTH" | "TWO_WAY" | "TWO-WAY" | "UPDOWN" | "UP_DOWN" | "UP-DOWN" => {
            Ok(TownStairKind::Both)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected UP, DOWN, or BOTH",
        )),
    }
}

pub fn load_town_trap_door_entries(game_dir: &Path) -> io::Result<Option<Vec<TownTrapDoorEntry>>> {
    let path = game_dir.join(TOWN_TRAP_DOOR_TABLE_FILE);
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
    parse_town_trap_door_entries(&text).map(Some)
}

pub fn parse_town_trap_door_entries(text: &str) -> io::Result<Vec<TownTrapDoorEntry>> {
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
        if !(5..=6).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y TO_FLOOR [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let to_floor = parse_i8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} has invalid target floor `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        if to_floor == floor {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} target floor must differ"),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(5) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownTrapDoorEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_TRAP_DOOR_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownTrapDoorEntry {
            scene,
            floor,
            x,
            y,
            to_floor,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_town_poison_gas_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<TownPoisonGasEntry>>> {
    let path = game_dir.join(TOWN_POISON_GAS_TABLE_FILE);
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
    parse_town_poison_gas_entries(&text).map(Some)
}

pub fn parse_town_poison_gas_entries(text: &str) -> io::Result<Vec<TownPoisonGasEntry>> {
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
        if !(4..=5).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(4) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownPoisonGasEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_POISON_GAS_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownPoisonGasEntry {
            scene,
            floor,
            x,
            y,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_town_exit_tile_entries(game_dir: &Path) -> io::Result<Option<Vec<TownExitTileEntry>>> {
    let path = game_dir.join(TOWN_EXIT_TILE_TABLE_FILE);
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
    parse_town_exit_tile_entries(&text).map(Some)
}

pub fn parse_town_exit_tile_entries(text: &str) -> io::Result<Vec<TownExitTileEntry>> {
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
        if !(4..=5).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(4) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownExitTileEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_EXIT_TILE_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownExitTileEntry {
            scene,
            floor,
            x,
            y,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_town_lock_entries(game_dir: &Path) -> io::Result<Option<Vec<TownLockEntry>>> {
    let path = game_dir.join(TOWN_LOCK_TABLE_FILE);
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
    parse_town_lock_entries(&text).map(Some)
}

pub fn parse_town_lock_entries(text: &str) -> io::Result<Vec<TownLockEntry>> {
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
        if !(6..=7).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y LOCKED_TILE UNLOCKED_TILE [LOCKED|MAGIC]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_LOCK_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let locked_tile = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid locked tile `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let unlocked_tile = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid unlocked tile `{}`: {err}",
                    parts[5]
                ),
            )
        })?;
        if !(96..=103).contains(&locked_tile) || !(96..=103).contains(&unlocked_tile) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} lock tiles must be in the public door range 96..103"
                ),
            ));
        }
        if locked_tile == unlocked_tile {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} locked and unlocked tiles must differ"
                ),
            ));
        }
        let kind = parts
            .get(6)
            .map_or(Ok(TownLockKind::Locked), |kind| {
                parse_town_lock_kind(kind).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{TOWN_LOCK_TABLE_FILE} line {line_number} has invalid lock kind `{kind}`: {err}"
                        ),
                    )
                })
            })?;
        if entries.iter().any(|entry: &TownLockEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_LOCK_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownLockEntry {
            scene,
            floor,
            x,
            y,
            locked_tile,
            unlocked_tile,
            kind,
        });
    }
    Ok(entries)
}

pub fn parse_town_lock_kind(value: &str) -> io::Result<TownLockKind> {
    match value.to_ascii_uppercase().as_str() {
        "LOCKED" => Ok(TownLockKind::Locked),
        "MAGIC" | "MAGIC_LOCKED" | "MAGIC-LOCKED" => Ok(TownLockKind::Magic),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected LOCKED or MAGIC",
        )),
    }
}
