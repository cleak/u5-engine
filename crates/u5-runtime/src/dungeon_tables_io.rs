//! Loaders and parsers for the dungeon TSV tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_dungeon_exit_tile_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<DungeonExitTileEntry>>> {
    let path = game_dir.join(DUNGEON_EXIT_TILE_TABLE_FILE);
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
    parse_dungeon_exit_tile_entries(&text).map(Some)
}

pub fn parse_dungeon_exit_tile_entries(text: &str) -> io::Result<Vec<DungeonExitTileEntry>> {
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
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} must be: DUNGEON LEVEL X Y [CELL]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} has invalid dungeon `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Dungeon(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} source must be a dungeon"
                    ),
                ));
            }
        };
        let level = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if level >= DUNGEON_SIDE as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                ),
            ));
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} coordinate must be inside 0..7, got ({x}, {y})"
                ),
            ));
        }
        let expected_cell = if let Some(cell) = parts.get(4) {
            Some(parse_u8_literal(cell).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} has invalid cell `{cell}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &DungeonExitTileEntry| {
            entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{DUNGEON_EXIT_TILE_TABLE_FILE} line {line_number} duplicates {} level {level} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(DungeonExitTileEntry {
            scene,
            level,
            x,
            y,
            expected_cell,
        });
    }
    Ok(entries)
}

pub fn load_secret_door_entries(game_dir: &Path) -> io::Result<Option<Vec<SecretDoorEntry>>> {
    let path = game_dir.join(SECRET_DOOR_TABLE_FILE);
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
    parse_secret_door_entries(&text).map(Some)
}

pub fn parse_secret_door_entries(text: &str) -> io::Result<Vec<SecretDoorEntry>> {
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
                    "{SECRET_DOOR_TABLE_FILE} line {line_number} must be: TOWN SCENE FLOOR X Y REVEAL_TILE [TILE] or DUNGEON SCENE LEVEL X Y REVEAL_CELL [CELL]"
                ),
            ));
        }

        match parts[0].to_ascii_uppercase().as_str() {
            "TOWN" | "LOCATION" => {
                let scene = match PlayTarget::from_key(parts[1]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid town scene `{}`: {err}",
                            parts[1]
                        ),
                    )
                })? {
                    PlayTarget::Town(scene) => scene,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "{SECRET_DOOR_TABLE_FILE} line {line_number} TOWN row requires a town-family scene"
                            ),
                        ));
                    }
                };
                let floor = parse_i8_literal(parts[2]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                            parts[2]
                        ),
                    )
                })?;
                let x = parse_u8_literal(parts[3]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                            parts[3]
                        ),
                    )
                })? as usize;
                let y = parse_u8_literal(parts[4]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                            parts[4]
                        ),
                    )
                })? as usize;
                if x >= 32 || y >= 32 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} town coordinate must be inside 0..31, got ({x}, {y})"
                        ),
                    ));
                }
                let reveal_tile = parse_u8_literal(parts[5]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid reveal tile `{}`: {err}",
                            parts[5]
                        ),
                    )
                })?;
                if !openable_town_door(reveal_tile) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} town reveal tile must be a native unlocked door tile, got {reveal_tile}"
                        ),
                    ));
                }
                let expected_tile = if parts.len() == 7 {
                    Some(parse_u8_literal(parts[6]).map_err(|err| {
                        io::Error::new(
                            err.kind(),
                            format!(
                                "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                                parts[6]
                            ),
                        )
                    })?)
                } else {
                    None
                };
                entries.push(SecretDoorEntry::Town {
                    scene,
                    floor,
                    x,
                    y,
                    reveal_tile,
                    expected_tile,
                });
            }
            "DUNGEON" => {
                let scene = match PlayTarget::from_key(parts[1]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid dungeon scene `{}`: {err}",
                            parts[1]
                        ),
                    )
                })? {
                    PlayTarget::Dungeon(scene) => scene,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "{SECRET_DOOR_TABLE_FILE} line {line_number} DUNGEON row requires a dungeon scene"
                            ),
                        ));
                    }
                };
                let level = parse_u8_literal(parts[2]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid level `{}`: {err}",
                            parts[2]
                        ),
                    )
                })?;
                if level >= DUNGEON_SIDE as u8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} level must be inside 0..7, got {level}"
                        ),
                    ));
                }
                let x = parse_u8_literal(parts[3]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                            parts[3]
                        ),
                    )
                })? as usize;
                let y = parse_u8_literal(parts[4]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                            parts[4]
                        ),
                    )
                })? as usize;
                if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} dungeon coordinate must be inside 0..7, got ({x}, {y})"
                        ),
                    ));
                }
                let reveal_cell = parse_u8_literal(parts[5]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid reveal cell `{}`: {err}",
                            parts[5]
                        ),
                    )
                })?;
                let expected_cell = if parts.len() == 7 {
                    Some(parse_u8_literal(parts[6]).map_err(|err| {
                        io::Error::new(
                            err.kind(),
                            format!(
                                "{SECRET_DOOR_TABLE_FILE} line {line_number} has invalid cell `{}`: {err}",
                                parts[6]
                            ),
                        )
                    })?)
                } else {
                    None
                };
                entries.push(SecretDoorEntry::Dungeon {
                    scene,
                    level,
                    x,
                    y,
                    reveal_cell,
                    expected_cell,
                });
            }
            mode => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{SECRET_DOOR_TABLE_FILE} line {line_number} has unknown mode `{mode}`"
                    ),
                ));
            }
        }
    }
    Ok(entries)
}
