//! Loaders/parsers for blink targets, moongates, location floor pages, location entry-y, world overlay objects.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

pub fn load_blink_target_entries(game_dir: &Path) -> io::Result<Option<Vec<BlinkTargetEntry>>> {
    let path = game_dir.join(BLINK_TARGET_TABLE_FILE);
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
    parse_blink_target_entries(&text).map(Some)
}

pub fn parse_blink_target_entries(text: &str) -> io::Result<Vec<BlinkTargetEntry>> {
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
        if !(7..=9).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} must be: TARGET FLOOR FROM_X FROM_Y DIRECTION TO_X TO_Y [FROM_TILE|*] [TO_TILE|*]"
                ),
            ));
        }

        let target = PlayTarget::from_key(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid target `{}`: {err}",
                    parts[0]
                ),
            )
        })?;
        let floor = parse_i8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid floor `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let from_x = parse_u8_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid source X `{}`: {err}",
                    parts[2]
                ),
            )
        })? as usize;
        let from_y = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid source Y `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        let direction = parse_cardinal_direction_field(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid direction `{}`: {err}",
                    parts[4]
                ),
            )
        })?;
        let to_x = parse_u8_literal(parts[5]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[5]
                ),
            )
        })? as usize;
        let to_y = parse_u8_literal(parts[6]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[6]
                ),
            )
        })? as usize;
        validate_blink_target_bounds(target, floor, from_x, from_y, to_x, to_y, line_number)?;
        let expected_from_tile = parts
            .get(7)
            .map(|value| {
                parse_optional_u8_literal(
                    value,
                    BLINK_TARGET_TABLE_FILE,
                    line_number,
                    "source tile",
                )
            })
            .transpose()?
            .flatten();
        let expected_to_tile = parts
            .get(8)
            .map(|value| {
                parse_optional_u8_literal(
                    value,
                    BLINK_TARGET_TABLE_FILE,
                    line_number,
                    "destination tile",
                )
            })
            .transpose()?
            .flatten();
        if entries.iter().any(|entry: &BlinkTargetEntry| {
            entry.target == target
                && entry.floor == floor
                && entry.from_x == from_x
                && entry.from_y == from_y
                && entry.direction == direction
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{BLINK_TARGET_TABLE_FILE} line {line_number} duplicates {} floor {floor} at ({from_x}, {from_y}) {}",
                    target.key(),
                    direction.name()
                ),
            ));
        }
        entries.push(BlinkTargetEntry {
            target,
            floor,
            from_x,
            from_y,
            direction,
            to_x,
            to_y,
            expected_from_tile,
            expected_to_tile,
        });
    }
    Ok(entries)
}

pub fn load_karma_records(game_dir: &Path) -> io::Result<Option<Vec<String>>> {
    let path = game_dir.join(KARMA_DAT_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
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

pub fn validate_blink_target_bounds(
    target: PlayTarget,
    floor: i8,
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
    line_number: usize,
) -> io::Result<()> {
    match target {
        PlayTarget::World(plane) => {
            if floor != plane.save_floor() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{BLINK_TARGET_TABLE_FILE} line {line_number} floor must be {} for {}",
                        plane.save_floor(),
                        plane.key()
                    ),
                ));
            }
            if from_x >= WORLD_SIDE
                || from_y >= WORLD_SIDE
                || to_x >= WORLD_SIDE
                || to_y >= WORLD_SIDE
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{BLINK_TARGET_TABLE_FILE} line {line_number} world coordinates must be inside 0..255"
                    ),
                ));
            }
        }
        PlayTarget::Town(scene) => {
            if from_x >= 32 || from_y >= 32 || to_x >= 32 || to_y >= 32 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{BLINK_TARGET_TABLE_FILE} line {line_number} {} coordinates must be inside 0..31",
                        scene.key()
                    ),
                ));
            }
        }
        PlayTarget::Dungeon(scene) => {
            if !(0..=7).contains(&floor) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{BLINK_TARGET_TABLE_FILE} line {line_number} {} level must be 0..7",
                        scene.key()
                    ),
                ));
            }
            if from_x >= DUNGEON_SIDE
                || from_y >= DUNGEON_SIDE
                || to_x >= DUNGEON_SIDE
                || to_y >= DUNGEON_SIDE
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{BLINK_TARGET_TABLE_FILE} line {line_number} dungeon coordinates must be inside 0..7"
                    ),
                ));
            }
        }
    }
    Ok(())
}

pub fn parse_cardinal_direction_field(value: &str) -> io::Result<Direction> {
    match value.to_ascii_lowercase().as_str() {
        "n" | "north" | "8" => Ok(Direction::North),
        "e" | "east" | "6" => Ok(Direction::East),
        "s" | "south" | "2" => Ok(Direction::South),
        "w" | "west" | "4" => Ok(Direction::West),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected N/E/S/W or 8/6/2/4",
        )),
    }
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

pub fn load_moongate_entries(game_dir: &Path) -> io::Result<Option<Vec<MoongateEntry>>> {
    let path = game_dir.join(MOONGATE_TABLE_FILE);
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
    parse_moongate_entries(&text).map(Some)
}

pub fn parse_moongate_entries(text: &str) -> io::Result<Vec<MoongateEntry>> {
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
                    "{MOONGATE_TABLE_FILE} line {line_number} must be: ORIGIN_X ORIGIN_Y DEST_PLANE DEST_X DEST_Y [TILE] or ORIGIN_X ORIGIN_Y DEST_PLANE DEST_X DEST_Y START_HOUR END_HOUR [TILE]"
                ),
            ));
        }
        let x = parse_u8_literal(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{MOONGATE_TABLE_FILE} line {line_number} has invalid origin X `{}`: {err}",
                    parts[0]
                ),
            )
        })? as usize;
        let y = parse_u8_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{MOONGATE_TABLE_FILE} line {line_number} has invalid origin Y `{}`: {err}",
                    parts[1]
                ),
            )
        })? as usize;
        let destination_plane = WorldPlane::from_key(parts[2]).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{MOONGATE_TABLE_FILE} line {line_number} has unknown destination plane `{}`",
                    parts[2]
                ),
            )
        })?;
        let destination_x = parse_u8_literal(parts[3]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{MOONGATE_TABLE_FILE} line {line_number} has invalid destination X `{}`: {err}",
                    parts[3]
                ),
            )
        })? as usize;
        let destination_y = parse_u8_literal(parts[4]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{MOONGATE_TABLE_FILE} line {line_number} has invalid destination Y `{}`: {err}",
                    parts[4]
                ),
            )
        })? as usize;
        let (active_hours, expected_tile) = match parts.len() {
            5 => (None, None),
            6 => (
                None,
                Some(parse_moongate_tile_field(parts[5], line_number)?),
            ),
            7 | 8 => {
                let start = parse_hour_field(parts[5], MOONGATE_TABLE_FILE, line_number, "start")?;
                let end = parse_hour_field(parts[6], MOONGATE_TABLE_FILE, line_number, "end")?;
                let expected_tile = if parts.len() == 8 {
                    Some(parse_moongate_tile_field(parts[7], line_number)?)
                } else {
                    None
                };
                (Some((start, end)), expected_tile)
            }
            _ => unreachable!("validated moongate row length"),
        };
        if entries
            .iter()
            .any(|entry: &MoongateEntry| entry.x == x && entry.y == y)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{MOONGATE_TABLE_FILE} line {line_number} duplicates {x},{y}"),
            ));
        }
        entries.push(MoongateEntry {
            x,
            y,
            destination_plane,
            destination_x,
            destination_y,
            active_hours,
            expected_tile,
        });
    }
    Ok(entries)
}

pub fn parse_moongate_tile_field(value: &str, line_number: usize) -> io::Result<u8> {
    parse_u8_literal(value).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{MOONGATE_TABLE_FILE} line {line_number} has invalid tile `{value}`: {err}"),
        )
    })
}

pub fn parse_hour_field(
    value: &str,
    table: &str,
    line_number: usize,
    label: &str,
) -> io::Result<u8> {
    let hour = parse_u8_literal(value).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{table} line {line_number} has invalid {label} hour `{value}`: {err}"),
        )
    })?;
    if hour > 23 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{table} line {line_number} {label} hour must be 0..23, got {hour}"),
        ));
    }
    Ok(hour)
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
