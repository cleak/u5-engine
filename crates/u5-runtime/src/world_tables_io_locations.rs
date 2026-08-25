//! Loaders/parsers for world location, shrine, and plane-transition tables.

use std::fs;
use std::io;
use std::path::Path;

use crate::*;

type PublishedWorldLocationRow = (
    u8,
    WorldPlane,
    usize,
    usize,
    WorldEntryNarrationClass,
    Option<&'static str>,
    Option<u8>,
    u8,
    bool,
);

const PUBLISHED_WORLD_LOCATION_ROWS: &[PublishedWorldLocationRow] = &[
    (
        1,
        WorldPlane::Britannia,
        232,
        135,
        WorldEntryNarrationClass::Towne,
        Some("MOONGLOW"),
        Some(4),
        0x14,
        false,
    ),
    (
        2,
        WorldPlane::Britannia,
        81,
        106,
        WorldEntryNarrationClass::Towne,
        Some("BRITAIN"),
        Some(4),
        0x14,
        false,
    ),
    (
        3,
        WorldPlane::Britannia,
        36,
        222,
        WorldEntryNarrationClass::Towne,
        Some("JHELOM"),
        Some(5),
        0x14,
        false,
    ),
    (
        4,
        WorldPlane::Britannia,
        58,
        43,
        WorldEntryNarrationClass::Towne,
        Some("YEW"),
        Some(6),
        0x14,
        false,
    ),
    (
        5,
        WorldPlane::Britannia,
        159,
        20,
        WorldEntryNarrationClass::Towne,
        Some("MINOC"),
        Some(5),
        0x14,
        false,
    ),
    (
        6,
        WorldPlane::Britannia,
        106,
        184,
        WorldEntryNarrationClass::Towne,
        Some("TRINSIC"),
        Some(4),
        0x14,
        false,
    ),
    (
        7,
        WorldPlane::Britannia,
        22,
        128,
        WorldEntryNarrationClass::Towne,
        Some("SKARA BRAE"),
        Some(3),
        0x14,
        false,
    ),
    (
        8,
        WorldPlane::Britannia,
        187,
        169,
        WorldEntryNarrationClass::Village,
        Some("NEW MAGINCIA"),
        Some(2),
        0x13,
        false,
    ),
    (
        9,
        WorldPlane::Britannia,
        88,
        120,
        WorldEntryNarrationClass::Lighthouse,
        Some("FOGSBANE"),
        Some(4),
        0x1B,
        false,
    ),
    (
        10,
        WorldPlane::Britannia,
        152,
        24,
        WorldEntryNarrationClass::Lighthouse,
        Some("STORMCROW"),
        Some(3),
        0x1B,
        false,
    ),
    (
        11,
        WorldPlane::Britannia,
        104,
        216,
        WorldEntryNarrationClass::Lighthouse,
        Some("GREYHAVEN"),
        Some(3),
        0x1B,
        false,
    ),
    (
        12,
        WorldPlane::Britannia,
        216,
        120,
        WorldEntryNarrationClass::Lighthouse,
        Some("WAVEGUIDE"),
        Some(3),
        0x1B,
        false,
    ),
    (
        13,
        WorldPlane::Britannia,
        45,
        62,
        WorldEntryNarrationClass::Hut,
        Some("IOLO'S HUT"),
        Some(3),
        0x10,
        false,
    ),
    (
        14,
        WorldPlane::Britannia,
        176,
        208,
        WorldEntryNarrationClass::Hut,
        None,
        None,
        0x10,
        false,
    ),
    (
        15,
        WorldPlane::Britannia,
        201,
        59,
        WorldEntryNarrationClass::Hut,
        None,
        None,
        0x10,
        false,
    ),
    (
        16,
        WorldPlane::Britannia,
        153,
        91,
        WorldEntryNarrationClass::Hut,
        None,
        None,
        0x10,
        false,
    ),
    (
        17,
        WorldPlane::Britannia,
        86,
        107,
        WorldEntryNarrationClass::LordBritish,
        None,
        None,
        0x3E,
        false,
    ),
    (
        18,
        WorldPlane::Britannia,
        196,
        245,
        WorldEntryNarrationClass::Blackthorn,
        None,
        None,
        0x39,
        false,
    ),
    (
        19,
        WorldPlane::Britannia,
        84,
        106,
        WorldEntryNarrationClass::Village,
        Some("WEST BRITANNY"),
        Some(1),
        0x13,
        false,
    ),
    (
        20,
        WorldPlane::Britannia,
        86,
        105,
        WorldEntryNarrationClass::Village,
        Some("NORTH BRITANNY"),
        Some(1),
        0x13,
        false,
    ),
    (
        21,
        WorldPlane::Britannia,
        88,
        106,
        WorldEntryNarrationClass::Village,
        Some("EAST BRITANNY"),
        Some(1),
        0x13,
        false,
    ),
    (
        22,
        WorldPlane::Britannia,
        98,
        145,
        WorldEntryNarrationClass::Village,
        Some("PAWS"),
        Some(6),
        0x13,
        false,
    ),
    (
        23,
        WorldPlane::Britannia,
        136,
        90,
        WorldEntryNarrationClass::Village,
        Some("COVE"),
        Some(6),
        0x13,
        false,
    ),
    (
        24,
        WorldPlane::Britannia,
        136,
        158,
        WorldEntryNarrationClass::Towne,
        Some("BUCCANEER'S DEN"),
        Some(0),
        0x14,
        false,
    ),
    (
        25,
        WorldPlane::Underworld,
        49,
        58,
        WorldEntryNarrationClass::Keep,
        Some("ARARAT"),
        Some(5),
        0x12,
        false,
    ),
    (
        26,
        WorldPlane::Britannia,
        15,
        160,
        WorldEntryNarrationClass::Keep,
        Some("BORDERMARCH"),
        Some(2),
        0x12,
        false,
    ),
    (
        27,
        WorldPlane::Britannia,
        64,
        240,
        WorldEntryNarrationClass::Keep,
        Some("FARTHING"),
        Some(4),
        0x12,
        false,
    ),
    (
        28,
        WorldPlane::Britannia,
        248,
        8,
        WorldEntryNarrationClass::Keep,
        Some("WINDEMERE"),
        Some(3),
        0x12,
        false,
    ),
    (
        29,
        WorldPlane::Britannia,
        148,
        74,
        WorldEntryNarrationClass::Keep,
        Some("STONEGATE"),
        Some(3),
        0x12,
        false,
    ),
    (
        30,
        WorldPlane::Britannia,
        218,
        107,
        WorldEntryNarrationClass::Castle,
        Some("THE LYCAEUM"),
        Some(2),
        0x15,
        false,
    ),
    (
        31,
        WorldPlane::Britannia,
        28,
        50,
        WorldEntryNarrationClass::Castle,
        Some("EMPATH ABBEY"),
        Some(2),
        0x15,
        false,
    ),
    (
        32,
        WorldPlane::Britannia,
        146,
        241,
        WorldEntryNarrationClass::Castle,
        Some("SERPENT'S HOLD"),
        Some(1),
        0x15,
        false,
    ),
    (
        33,
        WorldPlane::Britannia,
        240,
        73,
        WorldEntryNarrationClass::Dungeon,
        Some("DECEIT"),
        Some(5),
        0x18,
        true,
    ),
    (
        34,
        WorldPlane::Britannia,
        91,
        67,
        WorldEntryNarrationClass::Cave,
        Some("DESPISE"),
        Some(4),
        0x16,
        true,
    ),
    (
        35,
        WorldPlane::Britannia,
        72,
        168,
        WorldEntryNarrationClass::Cave,
        Some("DESTARD"),
        Some(4),
        0x16,
        true,
    ),
    (
        36,
        WorldPlane::Britannia,
        126,
        20,
        WorldEntryNarrationClass::Dungeon,
        Some("WRONG"),
        Some(5),
        0x18,
        true,
    ),
    (
        37,
        WorldPlane::Britannia,
        156,
        27,
        WorldEntryNarrationClass::Dungeon,
        Some("COVETOUS"),
        Some(4),
        0x18,
        true,
    ),
    (
        38,
        WorldPlane::Britannia,
        58,
        102,
        WorldEntryNarrationClass::Mine,
        Some("SHAME"),
        Some(5),
        0x17,
        true,
    ),
    (
        39,
        WorldPlane::Britannia,
        239,
        240,
        WorldEntryNarrationClass::Mine,
        Some("HYTHLOTH"),
        Some(4),
        0x17,
        true,
    ),
    (
        40,
        WorldPlane::Underworld,
        128,
        128,
        WorldEntryNarrationClass::Cave,
        None,
        None,
        0x16,
        false,
    ),
];

pub fn published_world_location_entries() -> Vec<WorldLocationEntry> {
    PUBLISHED_WORLD_LOCATION_ROWS
        .iter()
        .copied()
        .map(
            |(
                scene_byte,
                plane,
                x,
                y,
                narration_class,
                proper_name,
                name_column,
                stock_tile,
                accepts_both_world_planes,
            )| {
                let target = if scene_byte <= SCENE_TOWN_FAMILY_LAST {
                    PlayTarget::Town(Scene::new(scene_byte).expect("published town scene is valid"))
                } else {
                    PlayTarget::Dungeon(
                        DungeonScene::new(scene_byte).expect("published dungeon scene is valid"),
                    )
                };
                WorldLocationEntry {
                    plane,
                    x,
                    y,
                    target,
                    town_entry_y: None,
                    expected_tile: Some(stock_tile),
                    narration_class: Some(narration_class),
                    proper_name,
                    name_column,
                    accepts_both_world_planes,
                }
            },
        )
        .collect()
}

pub fn effective_world_location_entries(game_dir: &Path) -> io::Result<Vec<WorldLocationEntry>> {
    effective_world_location_entries_with_sidecar_status(game_dir).map(|(entries, _)| entries)
}

pub fn effective_world_location_entries_with_sidecar_status(
    game_dir: &Path,
) -> io::Result<(Vec<WorldLocationEntry>, bool)> {
    let Some(mut entries) = load_world_location_entries(game_dir)? else {
        return Ok((published_world_location_entries(), false));
    };
    let published: Vec<_> = published_world_location_entries()
        .into_iter()
        .filter(|published| {
            !entries.iter().any(|entry| {
                entry.target == published.target
                    || (entry.plane == published.plane
                        && entry.x == published.x
                        && entry.y == published.y)
            })
        })
        .collect();
    entries.extend(published);
    Ok((entries, true))
}

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
        if !(4..=7).contains(&parts.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} must be: PLANE X Y TARGET [TOWN_ENTRY_Y] [TILE] [NARRATION_CLASS]"
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
        if matches!(target, PlayTarget::Dungeon(_)) && parts.len() == 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} line {line_number} entry Y is only valid for town-family targets"
                ),
            ));
        }
        let (town_entry_y, expected_tile, narration_index) = match target {
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
                let expected_tile = if parts.len() >= 6 {
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
                (
                    Some(entry_y),
                    expected_tile,
                    (parts.len() == 7).then_some(6),
                )
            }
            PlayTarget::Town(_) => (None, None, None),
            PlayTarget::Dungeon(_) if parts.len() >= 5 => {
                let expected_tile = parse_u8_literal(parts[4]).map_err(|err| {
                    io::Error::new(
                        err.kind(),
                        format!(
                            "{WORLD_LOCATION_TABLE_FILE} line {line_number} has invalid tile `{}`: {err}",
                            parts[4]
                        ),
                    )
                })?;
                (None, Some(expected_tile), (parts.len() == 6).then_some(5))
            }
            PlayTarget::Dungeon(_) => (None, None, None),
            PlayTarget::World(_) => unreachable!(),
        };
        let narration_class = narration_index
            .map(|index| {
                WorldEntryNarrationClass::from_key(parts[index]).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{WORLD_LOCATION_TABLE_FILE} line {line_number} has unknown narration class `{}`",
                            parts[index]
                        ),
                    )
                })
            })
            .transpose()?;
        if let Some(class) = narration_class {
            let target_helper = match target {
                PlayTarget::Town(_) => WorldEntryHelper::Town,
                PlayTarget::Dungeon(_) => WorldEntryHelper::Dungeon,
                PlayTarget::World(_) => unreachable!(),
            };
            if class.helper() != target_helper {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{WORLD_LOCATION_TABLE_FILE} line {line_number} narration class `{}` belongs to the opposite entry helper",
                        parts[narration_index.expect("present narration has an index")]
                    ),
                ));
            }
        }
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
            narration_class,
            proper_name: None,
            name_column: None,
            accepts_both_world_planes: false,
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
