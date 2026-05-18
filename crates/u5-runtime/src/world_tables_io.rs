//! Loaders and parsers for the world TSV tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_world_waterfall_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<WorldWaterfallEntry>>> {
    let path = game_dir.join(WORLD_WATERFALL_TABLE_FILE);
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
    parse_world_waterfall_entries(&text).map(Some)
}

pub fn parse_world_waterfall_entries(text: &str) -> io::Result<Vec<WorldWaterfallEntry>> {
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
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} must be: PLANE X Y DIRECTION STEPS [TILE]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let direction = parse_cardinal_direction(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid direction `{}`: {err}",
                    parts[3]
                ),
            )
        })?;
        let steps = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid step count `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        if steps == 0 || steps > 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} step count must be in 1..16, got {steps}"
                ),
            ));
        }
        let expected_tile = if let Some(tile) = parts.get(5) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_WATERFALL_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries
            .iter()
            .any(|entry: &WorldWaterfallEntry| entry.plane == plane && entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_WATERFALL_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldWaterfallEntry {
            plane,
            x,
            y,
            direction,
            steps,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_world_damage_tile_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<WorldDamageTileEntry>>> {
    let path = game_dir.join(WORLD_DAMAGE_TILE_TABLE_FILE);
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
    parse_world_damage_tile_entries(&text).map(Some)
}

pub fn parse_world_damage_tile_entries(text: &str) -> io::Result<Vec<WorldDamageTileEntry>> {
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
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} must be: PLANE X Y EFFECT [TILE]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let effect = WorldDamageEffect::from_key(parts[3]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has unknown effect `{}`",
                    parts[3]
                ),
            )
        })?;
        let expected_tile = if let Some(tile) = parts.get(4) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &WorldDamageTileEntry| {
            entry.plane == plane && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_DAMAGE_TILE_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldDamageTileEntry {
            plane,
            x,
            y,
            effect,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_world_encounter_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<WorldEncounterEntry>>> {
    let path = game_dir.join(WORLD_ENCOUNTER_TABLE_FILE);
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
    parse_world_encounter_entries(&text).map(Some)
}

pub fn parse_world_encounter_entries(text: &str) -> io::Result<Vec<WorldEncounterEntry>> {
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
        if !matches!(parts.len(), 6 | 7) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} must be: PLANE TILE THRESHOLD TYPE DX DY [PHASE]"
                ),
            ));
        }

        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let tile = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let threshold = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid threshold `{}`: {err}",
                    parts[2]
                ),
            )
        })?;
        if threshold > 30 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} threshold must be in 0..30, got {threshold}"
                ),
            ));
        }
        let type_byte = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid type `{}`: {err}",
                    parts[3]
                ),
            )
        })?;
        if !(192..=255).contains(&type_byte) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} type must be a monster/NPC sprite byte in 192..255, got {type_byte}"
                ),
            ));
        }
        let dx = parse_i8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid DX `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let dy = parse_i8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid DY `{}`: {err}",
                    parts[5]
                ),
            )
        })?;
        if dx == 0 && dy == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} offset cannot be 0,0"),
            ));
        }
        if dx.unsigned_abs() > ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS as u8
            || dy.unsigned_abs() > ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS as u8
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} offset must stay within +/-{ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS}"
                ),
            ));
        }
        let phase = if let Some(phase) = parts.get(6) {
            let phase = parse_u8_literal(phase).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} has invalid phase `{phase}`: {err}"
                    ),
                )
            })?;
            if direction_from_active_object_phase(phase).is_none() || (phase & 0x0f) == STEADY_PHASE
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} phase must encode a wander direction with a non-steady low nibble"
                    ),
                ));
            }
            phase
        } else {
            active_object_phase_toward_player(dx, dy)
        };
        if entries
            .iter()
            .any(|entry: &WorldEncounterEntry| entry.plane == plane && entry.tile == tile)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_ENCOUNTER_TABLE_FILE} line {line_number} duplicates {}/tile {tile}",
                    plane.key()
                ),
            ));
        }
        entries.push(WorldEncounterEntry {
            plane,
            tile,
            threshold,
            type_byte,
            dx,
            dy,
            phase,
        });
    }
    Ok(entries)
}
