//! Loaders/parsers for location floor pages, location entry-y, world overlay objects.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_karma_records(game_dir: &Path) -> io::Result<Option<Vec<String>>> {
    let path = game_dir.join(KARMA_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_karma_dat(&bytes).map(Some)
}

pub fn parse_karma_dat(bytes: &[u8]) -> io::Result<Vec<String>> {
    let mut records = Vec::with_capacity(KARMA_RECORD_COUNT);
    let mut start = 0;
    for record_index in 0..KARMA_RECORD_COUNT {
        let Some(relative_end) = bytes[start..].iter().position(|byte| *byte == 0) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{KARMA_DAT_FILE} record {record_index} is not NUL-terminated"),
            ));
        };
        let end = start + relative_end;
        let record = &bytes[start..end];
        if let Some(byte) = record
            .iter()
            .copied()
            .find(|byte| !matches!(*byte, b'\t' | b'\n' | b'\r' | 0x20..=0x7e))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{KARMA_DAT_FILE} record {record_index} has non-ASCII byte 0x{byte:02x}"),
            ));
        }
        let text = String::from_utf8(record.to_vec()).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{KARMA_DAT_FILE} record {record_index} is not UTF-8: {err}"),
            )
        })?;
        records.push(text);
        start = end + 1;
    }
    Ok(records)
}

pub fn parse_optional_u8_literal(
    value: &str,
    table: &str,
    line_number: usize,
    label: &str,
) -> io::Result<Option<u8>> {
    match value.to_ascii_uppercase().as_str() {
        "*" | "ANY" | "-" => Ok(None),
        _ => parse_u8_literal(value).map(Some).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("{table} line {line_number} has invalid {label} `{value}`: {err}"),
            )
        }),
    }
}

pub fn load_location_floor_entries(game_dir: &Path) -> io::Result<Option<Vec<LocationFloorEntry>>> {
    let path = game_dir.join(LOCATION_FLOOR_TABLE_FILE);
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
    parse_location_floor_entries(&text).map(Some)
}

pub fn parse_location_floor_entries(text: &str) -> io::Result<Vec<LocationFloorEntry>> {
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
        if parts.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOCATION_FLOOR_TABLE_FILE} line {line_number} must be: SCENE BASE_PAGE"),
            ));
        }
        let scene = Scene::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{LOCATION_FLOOR_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })?;
        let base_page = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{LOCATION_FLOOR_TABLE_FILE} line {line_number} has invalid base page `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        if base_page >= 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{LOCATION_FLOOR_TABLE_FILE} line {line_number} base page must be inside 0..15, got {base_page}"
                ),
            ));
        }
        if entries
            .iter()
            .any(|entry: &LocationFloorEntry| entry.scene == scene)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{LOCATION_FLOOR_TABLE_FILE} line {line_number} duplicates {}",
                    scene.key()
                ),
            ));
        }
        entries.push(LocationFloorEntry { scene, base_page });
    }
    Ok(entries)
}

pub fn load_location_entry_y(game_dir: &Path, scene: Scene) -> io::Result<Option<usize>> {
    Ok(
        load_location_entry_y_entries(game_dir)?.and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.scene == scene)
                .map(|entry| entry.y)
        }),
    )
}

pub fn load_location_entry_y_entries(
    game_dir: &Path,
) -> io::Result<Option<Vec<LocationEntryYEntry>>> {
    let path = game_dir.join(LOCATION_ENTRY_Y_TABLE_FILE);
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
    parse_location_entry_y_entries(&text).map(Some)
}

pub fn parse_location_entry_y_entries(text: &str) -> io::Result<Vec<LocationEntryYEntry>> {
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
        if parts.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOCATION_ENTRY_Y_TABLE_FILE} line {line_number} must be: SCENE ENTRY_Y"),
            ));
        }
        let scene = Scene::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{LOCATION_ENTRY_Y_TABLE_FILE} line {line_number} has invalid scene `{}`: {err}",
                    parts[0]
                ),
            )
        })?;
        let y = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{LOCATION_ENTRY_Y_TABLE_FILE} line {line_number} has invalid entry Y `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        if y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{LOCATION_ENTRY_Y_TABLE_FILE} line {line_number} entry Y must be inside 0..31, got {y}"
                ),
            ));
        }
        if entries
            .iter()
            .any(|entry: &LocationEntryYEntry| entry.scene == scene)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{LOCATION_ENTRY_Y_TABLE_FILE} line {line_number} duplicates {}",
                    scene.key()
                ),
            ));
        }
        entries.push(LocationEntryYEntry { scene, y });
    }
    Ok(entries)
}

pub fn load_world_overlay_objects(
    game_dir: &Path,
    plane: WorldPlane,
) -> io::Result<Vec<ActiveObject>> {
    let saved_ool = game_dir.join("SAVED.OOL");
    if saved_ool.exists() {
        let bytes = read_saved_ool_bytes(game_dir)?;
        let start = match plane {
            WorldPlane::Britannia => 0,
            WorldPlane::Underworld => OOL_PLANE_LEN,
        };
        return decode_ool_plane_objects(&bytes[start..start + OOL_PLANE_LEN]);
    }

    let plane_file = game_dir.join(match plane {
        WorldPlane::Britannia => BRIT_OOL_FILENAME,
        WorldPlane::Underworld => UNDER_OOL_FILENAME,
    });
    if !plane_file.exists() {
        return Ok(Vec::new());
    }
    let bytes = read(&plane_file)?;
    decode_ool_plane_objects(&bytes)
}

pub fn load_world_overlay_mirror_objects(
    game_dir: &Path,
    plane: WorldPlane,
) -> io::Result<Vec<ActiveObject>> {
    let plane_file = game_dir.join(match plane {
        WorldPlane::Britannia => BRIT_OOL_FILENAME,
        WorldPlane::Underworld => UNDER_OOL_FILENAME,
    });
    if plane_file.exists() {
        let bytes = read(&plane_file)?;
        return decode_ool_plane_objects(&bytes);
    }

    load_world_overlay_objects(game_dir, plane)
}

/// Load all 32 records from the current canonical plane mirror.
///
/// Town exit must replace the live table, including slot zero, and must not
/// consult the in-memory overlay cache.  A missing per-plane mirror falls
/// back to the matching `SAVED.OOL` half so a directly loaded town save has
/// the same exit path after ordinary save-mirror reconstruction.
pub fn load_world_active_object_mirror_table(
    game_dir: &Path,
    plane: WorldPlane,
) -> io::Result<Vec<ActiveObject>> {
    let plane_file = game_dir.join(match plane {
        WorldPlane::Britannia => BRIT_OOL_FILENAME,
        WorldPlane::Underworld => UNDER_OOL_FILENAME,
    });
    if plane_file.exists() {
        return decode_full_ool_plane_table(&read(&plane_file)?);
    }

    let saved_ool = game_dir.join(SAVED_OOL_FILENAME);
    if !saved_ool.exists() {
        return Ok(vec![ActiveObject::empty(); OOL_SLOTS]);
    }
    let bytes = read_saved_ool_bytes(game_dir)?;
    let start = match plane {
        WorldPlane::Britannia => 0,
        WorldPlane::Underworld => OOL_PLANE_LEN,
    };
    decode_full_ool_plane_table(&bytes[start..start + OOL_PLANE_LEN])
}

pub fn load_init_overlay_objects(game_dir: &Path) -> io::Result<Vec<ActiveObject>> {
    let path = game_dir.join(INIT_OOL_FILENAME);
    let bytes = read(&path)?;
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{INIT_OOL_FILENAME} must be {OOL_PLANE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    decode_ool_plane_objects(&bytes)
}
