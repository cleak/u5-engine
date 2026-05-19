//! Loaders/parsers for world location, shrine, and plane-transition tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn parse_world_location_entries(text: &str) -> io::Result<Vec<WorldLocationEntry>> {
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
        if !matches!(parts.len(), 4 | 5 | 6) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} must be: PLANE X Y TARGET [TOWN_ENTRY_Y] [TILE]"
                ),
            ));
        }
        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let target = PlayTarget::from_key(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid target `{}`: {err}",
                    parts[3]
                ),
            )
        })?;
        if matches!(target, PlayTarget::World(_)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} target must be a town or dungeon scene"
                ),
            ));
        }
        if matches!(target, PlayTarget::Town(_)) && plane != WorldPlane::Britannia {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} town-family entries must be on BRITANNIA"
                ),
            ));
        }
        let (town_entry_y, expected_tile) = match target {
            PlayTarget::Town(_) if parts.len() >= 5 => {
                let entry_y = parse_u8_literal(parts[4]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid entry Y `{}`: {err}",
                            parts[4]
                        ),
                    )
                })? as usize;
                if entry_y >= 32 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{WORLD_LOCATION_TABLE_FILE} line {line_number} entry Y must be inside 0..31, got {entry_y}"
                        ),
                    ));
                }
                let expected_tile = if parts.len() == 6 {
                    Some(parse_u8_literal(parts[5]).map_err(|err| {
                        io::Error::new(
                            err.kind(),
                            format!(
                                "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                                parts[5]
                            ),
                        )
                    })?)
                } else {
                    None
                };
                (Some(entry_y), expected_tile)
            }
            PlayTarget::Town(_) => (None, None),
            PlayTarget::Dungeon(_) if parts.len() == 5 => {
                let expected_tile = parse_u8_literal(parts[4]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                            parts[4]
                        ),
                    )
                })?;
                (None, Some(expected_tile))
            }
            PlayTarget::Dungeon(_) if parts.len() == 6 => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{WORLD_LOCATION_TABLE_FILE} line {line_number} entry Y is only valid for town-family targets"
                    ),
                ));
            }
            PlayTarget::Dungeon(_) => (None, None),
            PlayTarget::World(_) => unreachable!(),
        };
        if entries
            .iter()
            .any(|entry: &WorldLocationEntry| entry.plane == plane && entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        if entries
            .iter()
            .any(|entry: &WorldLocationEntry| entry.target == target)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} duplicates return target {}",
                    target.key()
                ),
            ));
        }
        entries.push(WorldLocationEntry {
            plane,
            x,
            y,
            target,
            town_entry_y,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_shrine_entries(game_dir: &Path) -> io::Result<Option<Vec<ShrineEntry>>> {
    let path = game_dir.join(SHRINE_TABLE_FILE);
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
    parse_shrine_entries(&text).map(Some)
}

pub fn parse_shrine_entries(text: &str) -> io::Result<Vec<ShrineEntry>> {
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
        if !matches!(parts.len(), 4 | 5) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{SHRINE_TABLE_FILE} line {line_number} must be: PLANE X Y VIRTUE [TILE]"),
            ));
        }
        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SHRINE_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        if plane != WorldPlane::Britannia {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{SHRINE_TABLE_FILE} line {line_number} shrine rows must be on BRITANNIA"),
            ));
        }
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{SHRINE_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{SHRINE_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let virtue = ShrineVirtue::from_key(parts[3]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SHRINE_TABLE_FILE} line {line_number} has unknown virtue `{}`",
                    parts[3]
                ),
            )
        })?;
        let expected_tile = if parts.len() == 5 {
            Some(parse_u8_literal(parts[4]).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{SHRINE_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                        parts[4]
                    ),
                )
            })?)
        } else {
            None
        };
        if entries
            .iter()
            .any(|entry: &ShrineEntry| entry.plane == plane && entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SHRINE_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        if entries
            .iter()
            .any(|entry: &ShrineEntry| entry.virtue == virtue)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SHRINE_TABLE_FILE} line {line_number} duplicates shrine of {}",
                    virtue.name()
                ),
            ));
        }
        entries.push(ShrineEntry {
            plane,
            x,
            y,
            virtue,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_codex_urn_entries(game_dir: &Path) -> io::Result<Option<Vec<CodexUrnEntry>>> {
    let path = game_dir.join(CODEX_URN_TABLE_FILE);
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
    parse_codex_urn_entries(&text).map(Some)
}

pub fn parse_codex_urn_entries(text: &str) -> io::Result<Vec<CodexUrnEntry>> {
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
        if !matches!(parts.len(), 3 | 4) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{CODEX_URN_TABLE_FILE} line {line_number} must be: PLANE X Y [TILE]"),
            ));
        }
        let plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{CODEX_URN_TABLE_FILE} line {line_number} has unknown plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{CODEX_URN_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{CODEX_URN_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let expected_tile = if parts.len() == 4 {
            Some(parse_u8_literal(parts[3]).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{CODEX_URN_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                        parts[3]
                    ),
                )
            })?)
        } else {
            None
        };
        if entries
            .iter()
            .any(|entry: &CodexUrnEntry| entry.plane == plane && entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{CODEX_URN_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    plane.key()
                ),
            ));
        }
        entries.push(CodexUrnEntry {
            plane,
            x,
            y,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_eternal_flame_entries(game_dir: &Path) -> io::Result<Option<Vec<EternalFlameEntry>>> {
    let path = game_dir.join(ETERNAL_FLAME_TABLE_FILE);
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
    parse_eternal_flame_entries(&text).map(Some)
}

pub fn parse_eternal_flame_entries(text: &str) -> io::Result<Vec<EternalFlameEntry>> {
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
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} must be: TARGET FLOOR X Y FLAME [TILE]"
                ),
            ));
        }

        let target = PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} has invalid target `{}`: {err}",
                    parts[0]
                ),
            )
        })?;
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        match target {
            PlayTarget::Town(_) if x >= 32 || y >= 32 => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{ETERNAL_FLAME_TABLE_FILE} line {line_number} town coordinate must be inside 0..31, got ({x}, {y})"
                    ),
                ));
            }
            PlayTarget::Dungeon(_) if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{ETERNAL_FLAME_TABLE_FILE} line {line_number} dungeon coordinate must be inside 0..7, got ({x}, {y})"
                    ),
                ));
            }
            _ => {}
        }
        let flame = EternalFlame::from_key(parts[4]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} has unknown flame `{}`",
                    parts[4]
                ),
            )
        })?;
        let expected_tile = if let Some(tile) = parts.get(5) {
            Some(parse_u8_literal(tile).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{ETERNAL_FLAME_TABLE_FILE} line {line_number} has invalid tile `{tile}`: {err}"
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &EternalFlameEntry| {
            entry.target == target && entry.floor == floor && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ETERNAL_FLAME_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({x}, {y})",
                    target.key()
                ),
            ));
        }
        entries.push(EternalFlameEntry {
            target,
            floor,
            x,
            y,
            flame,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_world_plane_transition_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<WorldPlaneTransitionEntry>>> {
    let path = game_dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE);
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
    parse_world_plane_transition_entries(&text).map(Some)
}

pub fn parse_world_plane_transition_entries(
    text: &str,
) -> io::Result<Vec<WorldPlaneTransitionEntry>> {
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
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} must be: FROM_PLANE X Y TO_PLANE TO_X TO_Y [TILE]"
                ),
            ));
        }
        let from_plane = WorldPlane::from_key(parts[0]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has unknown source plane `{}`",
                    parts[0]
                ),
            )
        })?;
        let x = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has invalid X `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has invalid Y `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let to_plane = WorldPlane::from_key(parts[3]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has unknown destination plane `{}`",
                    parts[3]
                ),
            )
        })?;
        if from_plane == to_plane {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} must change world plane"
                ),
            ));
        }
        let to_x = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[4]
                ),
            )
        })? as usize;
        let to_y = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[5]
                ),
            )
        })? as usize;
        let expected_tile = if parts.len() == 7 {
            Some(parse_u8_literal(parts[6]).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!(
                        "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                        parts[6]
                    ),
                )
            })?)
        } else {
            None
        };
        if entries.iter().any(|entry: &WorldPlaneTransitionEntry| {
            entry.from_plane == from_plane && entry.x == x && entry.y == y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} duplicates {}/{x},{y}",
                    from_plane.key()
                ),
            ));
        }
        if entries.iter().any(|entry: &WorldPlaneTransitionEntry| {
            entry.to_plane == to_plane && entry.to_x == to_x && entry.to_y == to_y
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_PLANE_TRANSITION_TABLE_FILE} line {line_number} duplicates destination {}/{to_x},{to_y}",
                    to_plane.key()
                ),
            ));
        }
        entries.push(WorldPlaneTransitionEntry {
            from_plane,
            x,
            y,
            to_plane,
            to_x,
            to_y,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn load_world_location_entries(game_dir: &Path) -> io::Result<Option<Vec<WorldLocationEntry>>> {
    let path = game_dir.join(WORLD_LOCATION_TABLE_FILE);
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
    parse_world_location_entries(&text).map(Some)
}
