//! Loaders/parsers for world get-tile and object-pickup tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_world_get_tile_entries(game_dir: &Path) -> io::Result<Option<Vec<WorldGetTileEntry>>> {
    let path = game_dir.join(WORLD_GET_TILE_TABLE_FILE);
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
    parse_world_get_tile_entries(&text).map(Some)
}

pub fn parse_world_get_tile_entries(text: &str) -> io::Result<Vec<WorldGetTileEntry>> {
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
        if !(4..=7).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} must be: PLANE X Y REPLACEMENT_TILE [TILE] [ITEM AMOUNT]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let replacement_tile = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} has invalid replacement tile `{}`: {err}",
                    parts[3]
                ),
            )
        })?;
        let (expected_tile, grant) =
            parse_tile_get_tail(WORLD_GET_TILE_TABLE_FILE, line_number, &parts[4..])?;
        if expected_tile == Some(replacement_tile) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} replacement tile must differ from guarded tile"
                ),
            ));
        }
        if entries
            .iter()
            .any(|entry: &WorldGetTileEntry| entry.plane == plane && entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_GET_TILE_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldGetTileEntry {
            plane,
            x,
            y,
            replacement_tile,
            expected_tile,
            grant,
        });
    }
    Ok(entries)
}

pub fn parse_tile_get_tail(
    table_file: &str,
    line_number: usize,
    parts: &[&str],
) -> io::Result<(Option<u8>, Option<ObjectPickupGrant>)> {
    match parts {
        [] => Ok((None, None)),
        [tile] => Ok((
            Some(parse_tile_get_guard(table_file, line_number, tile)?),
            None,
        )),
        [item, amount] => Ok((
            None,
            Some(parse_tile_get_grant(table_file, line_number, item, amount)?),
        )),
        [tile, item, amount] => Ok((
            Some(parse_tile_get_guard(table_file, line_number, tile)?),
            Some(parse_tile_get_grant(table_file, line_number, item, amount)?),
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{table_file} line {line_number} must be: ... REPLACEMENT_TILE [TILE] [ITEM AMOUNT]"
            ),
        )),
    }
}

pub fn parse_tile_get_guard(table_file: &str, line_number: usize, tile: &str) -> io::Result<u8> {
    parse_u8_literal(tile).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{table_file} line {line_number} has invalid tile `{tile}`: {err}"),
        )
    })
}

pub fn parse_tile_get_grant(
    table_file: &str,
    line_number: usize,
    item: &str,
    amount: &str,
) -> io::Result<ObjectPickupGrant> {
    let kind = ObjectPickupKind::from_key(item).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{table_file} line {line_number} has unsupported item `{item}`"),
        )
    })?;
    let amount = parse_u8_literal(amount).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{table_file} line {line_number} has invalid amount `{amount}`: {err}"),
        )
    })?;
    if amount == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{table_file} line {line_number} amount must be nonzero"),
        ));
    }
    Ok(ObjectPickupGrant { kind, amount })
}

pub fn load_object_pickup_entries(game_dir: &Path) -> io::Result<Option<Vec<ObjectPickupEntry>>> {
    let path = game_dir.join(OBJECT_PICKUP_TABLE_FILE);
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
    parse_object_pickup_entries(&text).map(Some)
}

pub fn parse_object_pickup_entries(text: &str) -> io::Result<Vec<ObjectPickupEntry>> {
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
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} must be: TARGET FLOOR X Y ITEM AMOUNT [TILE]"
                ),
            ));
        }

        let target = match PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has invalid target `{}`: {err}",
                    parts[0]
                ),
            )
        })? {
            PlayTarget::World(plane) => PlayTarget::World(plane),
            PlayTarget::Town(scene) => PlayTarget::Town(scene),
            PlayTarget::Dungeon(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{OBJECT_PICKUP_TABLE_FILE} line {line_number} requires a town-family or world target"
                    ),
                ));
            }
        };
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        if let PlayTarget::World(plane) = target {
            let expected_floor = plane.save_floor();
            if floor != expected_floor {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{OBJECT_PICKUP_TABLE_FILE} line {line_number} world target {} requires floor {expected_floor}, got {floor}",
                        plane.key()
                    ),
                ));
            }
        }
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        if matches!(target, PlayTarget::Town(_)) && (x >= 32 || y >= 32) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} town coordinate must be inside 0..31, got ({x}, {y})"
                ),
            ));
        }
        let kind = ObjectPickupKind::from_key(parts[4]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has unsupported item `{}`",
                    parts[4]
                ),
            )
        })?;
        let amount = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has invalid amount `{}`: {err}",
                    parts[5]
                ),
            )
        })?;
        if amount == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{OBJECT_PICKUP_TABLE_FILE} line {line_number} amount must be nonzero"),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(6) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{OBJECT_PICKUP_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &ObjectPickupEntry| {
            entry.target == target && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{OBJECT_PICKUP_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    target.key()
                ),
            ));
        }
        entries.push(ObjectPickupEntry {
            target,
            floor,
            x,
            y,
            kind,
            amount,
            expected_tile,
        });
    }
    Ok(entries)
}
