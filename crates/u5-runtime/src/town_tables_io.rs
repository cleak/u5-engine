//! Loaders and parsers for the town TSV tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_stationary_display_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<StationaryDisplayEntry>>> {
    let path = game_dir.join(STATIONARY_DISPLAY_TABLE_FILE);
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
    parse_stationary_display_entries(&text).map(Some)
}

pub fn parse_stationary_display_entries(text: &str) -> io::Result<Vec<StationaryDisplayEntry>> {
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
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y ITEM AMOUNT PRICE [TILE] or SCENE FLOOR marker:TILE ORDINAL ITEM AMOUNT PRICE [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let (x, y, marker_tile, marker_ordinal, item_part) = if let Some(marker) =
            parts[2].strip_prefix("marker:")
        {
            let marker_tile = parse_u8_literal(marker).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid marker tile `{marker}`: {err}"
                        ),
                    )
                })?;
            let marker_ordinal = parse_u8_literal(parts[3]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid marker ordinal `{}`: {err}",
                            parts[3]
                        ),
                    )
                })? as usize;
            (None, None, Some(marker_tile), Some(marker_ordinal), 4usize)
        } else {
            let x = parse_u8_literal(parts[2]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                            parts[2]
                        ),
                    )
                })? as usize;
            let y = parse_u8_literal(parts[3]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                            parts[3]
                        ),
                    )
                })? as usize;
            if x >= 32 || y >= 32 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                    ),
                ));
            }
            (Some(x), Some(y), None, None, 4usize)
        };
        let kind = ObjectPickupKind::from_key(parts[item_part]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has unknown item `{}`",
                    parts[item_part]
                ),
            )
        })?;
        let amount = parse_u8_literal(parts[item_part + 1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid amount `{}`: {err}",
                    parts[item_part + 1]
                ),
            )
        })?;
        if amount == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} amount must be nonzero"
                ),
            ));
        }
        let price = parse_u16_literal(parts[item_part + 2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid price `{}`: {err}",
                    parts[item_part + 2]
                ),
            )
        })?;
        let expected_tile = if let Some(tile) = parts.get(item_part + 3) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &StationaryDisplayEntry| {
            entry.scene == scene
                && entry.floor == floor
                && ((entry.x == x && entry.y == y && x.is_some() && y.is_some())
                    || (entry.marker_tile == marker_tile
                        && entry.marker_ordinal == marker_ordinal
                        && marker_tile.is_some()
                        && marker_ordinal.is_some()))
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{STATIONARY_DISPLAY_TABLE_FILE} line {line_number} duplicates {} floor {floor}",
                    scene.key()
                ),
            ));
        }
        entries.push(StationaryDisplayEntry {
            scene,
            floor,
            x,
            y,
            marker_tile,
            marker_ordinal,
            grant: ObjectPickupGrant { kind, amount },
            price,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_town_fire_source_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<TownFireSourceEntry>>> {
    let path = game_dir.join(TOWN_FIRE_SOURCE_TABLE_FILE);
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
    parse_town_fire_source_entries(&text).map(Some)
}

pub fn parse_town_fire_source_entries(text: &str) -> io::Result<Vec<TownFireSourceEntry>> {
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
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y DIRECTION [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let direction = parse_cardinal_direction(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} has invalid direction `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let expected_tile = if parts.len() == 6 {
            Some(parse_u8_literal(parts[5]).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                        parts[5]
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownFireSourceEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_FIRE_SOURCE_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownFireSourceEntry {
            scene,
            floor,
            x,
            y,
            direction,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_town_pushable_entries(game_dir: &Path) -> io::Result<Option<Vec<TownPushableEntry>>> {
    let path = game_dir.join(TOWN_PUSHABLE_TABLE_FILE);
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
    parse_town_pushable_entries(&text).map(Some)
}

pub fn parse_town_pushable_entries(text: &str) -> io::Result<Vec<TownPushableEntry>> {
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
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(4) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownPushableEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_PUSHABLE_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownPushableEntry {
            scene,
            floor,
            x,
            y,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_town_get_tile_entries(game_dir: &Path) -> io::Result<Option<Vec<TownGetTileEntry>>> {
    let path = game_dir.join(TOWN_GET_TILE_TABLE_FILE);
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
    parse_town_get_tile_entries(&text).map(Some)
}

pub fn parse_town_get_tile_entries(text: &str) -> io::Result<Vec<TownGetTileEntry>> {
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
        if !(5..=8).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y REPLACEMENT_TILE [TILE] [ITEM AMOUNT]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_GET_TILE_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let replacement_tile = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} has invalid replacement tile `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let (expected_tile, grant) =
            parse_tile_get_tail(TOWN_GET_TILE_TABLE_FILE, line_number, &parts[5..])?;
        if expected_tile == Some(replacement_tile) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} replacement tile must differ from guarded tile"
                ),
            ));
        }
        if entries.iter().any(|entry: &TownGetTileEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_GET_TILE_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownGetTileEntry {
            scene,
            floor,
            x,
            y,
            replacement_tile,
            expected_tile,
            grant,
        });
    }
    Ok(entries)
}

pub fn load_town_rest_bed_entries(game_dir: &Path) -> io::Result<Option<Vec<TownRestBedEntry>>> {
    let path = game_dir.join(TOWN_REST_BED_TABLE_FILE);
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
    parse_town_rest_bed_entries(&text).map(Some)
}

pub fn parse_town_rest_bed_entries(text: &str) -> io::Result<Vec<TownRestBedEntry>> {
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
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} must be: SCENE FLOOR X Y [TILE]"
                ),
            ));
        }

        let scene = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::Town(scene) => scene,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{TOWN_REST_BED_TABLE_FILE} line {line_number} requires a town-family scene"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(4) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{TOWN_REST_BED_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &TownRestBedEntry| {
            entry.scene == scene && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{TOWN_REST_BED_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    scene.key()
                ),
            ));
        }
        entries.push(TownRestBedEntry {
            scene,
            floor,
            x,
            y,
            expected_tile,
        });
    }
    Ok(entries)
}
