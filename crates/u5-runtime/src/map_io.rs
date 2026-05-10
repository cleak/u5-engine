//! TLK parsing, NPC block parsing, and scene/dungeon/world map loaders + decoders.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn parse_tlk(path: &Path) -> io::Result<HashMap<u16, Vec<String>>> {
    let bytes = read(path)?;
    parse_tlk_bytes(&bytes)
}

pub fn parse_tlk_bytes(bytes: &[u8]) -> io::Result<HashMap<u16, Vec<String>>> {
    if bytes.len() < 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "short TLK"));
    }
    let count = u16_at(&bytes, 0) as usize;
    let mut entries = Vec::new();
    for k in 1..count {
        let off = u16_at(&bytes, 4 * k) as usize;
        let id = u16_at(&bytes, 4 * k + 2);
        entries.push((id, off));
    }
    entries.sort_by_key(|(_, off)| *off);
    let mut out = HashMap::new();
    for (idx, (id, off)) in entries.iter().enumerate() {
        let end = entries
            .get(idx + 1)
            .map(|(_, next)| *next)
            .unwrap_or(bytes.len());
        if *off >= bytes.len() || *off >= end {
            continue;
        }
        let mut fields = Vec::new();
        let mut pos = *off;
        while pos < end && fields.len() < 40 {
            let (field, next) = decode_tlk_field(&bytes, pos, end);
            fields.push(field);
            pos = next;
            if pos == end {
                break;
            }
        }
        out.insert(*id, fields);
    }
    Ok(out)
}

pub fn decode_tlk_field(bytes: &[u8], mut pos: usize, end: usize) -> (String, usize) {
    let mut s = String::new();
    while pos < end {
        let b = bytes[pos];
        pos += 1;
        if b == 0 {
            break;
        }
        match b {
            0x85 => pos = (pos + 3).min(end),
            0x86 | 0x8c => pos = (pos + 1).min(end),
            0xfe => pos = (pos + 2).min(end),
            0xa0..=0xfd => s.push((b ^ 0x80) as char),
            0x01..=0x9d => s.push(' '),
            _ => {}
        }
    }
    (compact(&s), pos)
}

pub fn non_empty_talk_keyword(keyword: &str) -> Option<&str> {
    let keyword = keyword.trim();
    (!keyword.is_empty()).then_some(keyword)
}

pub fn talk_keyword_response<'a>(fields: &'a [String], keyword: &str) -> Option<&'a str> {
    if talk_keyword_matches("JOB", keyword) {
        return fields.get(3).map(String::as_str);
    }
    if talk_keyword_matches("BYE", keyword) {
        return fields.get(4).map(String::as_str);
    }

    fields
        .get(5..)
        .unwrap_or_default()
        .chunks_exact(2)
        .find_map(|pair| talk_keyword_matches(&pair[0], keyword).then_some(pair[1].as_str()))
}

pub fn talk_keyword_matches(stored_keyword: &str, input: &str) -> bool {
    let stored = talk_keyword_compare_text(stored_keyword.trim());
    if stored.is_empty() {
        return false;
    }
    let input = talk_keyword_compare_text(input.trim_start());
    input.starts_with(&stored)
        && input
            .as_bytes()
            .get(stored.len())
            .is_none_or(|byte| *byte == b' ')
}

pub fn talk_keyword_compare_text(value: &str) -> String {
    value
        .bytes()
        .map(|byte| (byte & 0x7f).to_ascii_uppercase() as char)
        .collect()
}

pub fn parse_npc_block(
    game_dir: &Path,
    scene: Scene,
    tlk: &HashMap<u16, Vec<String>>,
) -> io::Result<Vec<NpcSlot>> {
    let bytes = read(&game_dir.join(format!("{}.NPC", scene.family.stem())))?;
    let base = scene.block * 576;
    if base + 576 > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "short NPC block",
        ));
    }
    let mut slots = Vec::new();
    for slot in 0..32 {
        let mut schedule = [0u8; 16];
        schedule.copy_from_slice(&bytes[base + slot * 16..base + slot * 16 + 16]);
        let type_byte = bytes[base + 512 + slot];
        let dialog_id = bytes[base + 544 + slot];
        let name = tlk
            .get(&(dialog_id as u16))
            .and_then(|fields| fields.first())
            .filter(|name| !name.is_empty())
            .cloned();
        slots.push(NpcSlot {
            slot,
            type_byte,
            dialog_id,
            schedule,
            name,
        });
    }
    Ok(slots)
}

pub fn load_floor(game_dir: &Path, scene: Scene, floor: i8) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join(format!("{}.DAT", scene.family.stem())))?;
    let page = resolve_location_floor_page(game_dir, scene, floor)?;
    let start = page * 1024;
    if start + 1024 > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}.DAT is too short for {} floor {} page {}",
                scene.family.stem(),
                scene.key(),
                floor,
                page
            ),
        ));
    }
    Ok(bytes[start..start + 1024].to_vec())
}

pub fn load_town_runtime_floor(
    game_dir: &Path,
    scene: Scene,
    floor: i8,
    hour: u8,
) -> io::Result<Vec<u8>> {
    let mut grid = load_floor(game_dir, scene, floor)?;
    normalize_town_runtime_floor(&mut grid, hour);
    Ok(grid)
}

pub fn normalize_town_runtime_floor(grid: &mut [u8], hour: u8) {
    scrub_location_entry_markers(grid);
    if is_town_night_hour(hour) {
        apply_dawn_dusk_substitution(grid);
    }
}

pub fn resolve_location_floor_page(game_dir: &Path, scene: Scene, floor: i8) -> io::Result<usize> {
    let base_page = load_location_floor_entries(game_dir)?
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.scene == scene)
                .map(|entry| entry.base_page)
        })
        .unwrap_or_else(|| scene.block * 2);
    let page = base_page as i16 + floor as i16;
    if !(0..16).contains(&page) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} maps {} floor {} to page {}, outside 0..15",
                LOCATION_FLOOR_TABLE_FILE,
                scene.key(),
                floor,
                page
            ),
        ));
    }
    Ok(page as usize)
}

pub fn load_dungeon_record(game_dir: &Path, scene: DungeonScene) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join("DUNGEON.DAT"))?;
    if bytes.len() != DUNGEON_DAT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "DUNGEON.DAT must be {DUNGEON_DAT_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    let start = scene.record * DUNGEON_RECORD_LEN;
    Ok(bytes[start..start + DUNGEON_RECORD_LEN].to_vec())
}

pub fn load_world_map(game_dir: &Path, plane: WorldPlane) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join(plane.file_name()))?;
    match plane {
        WorldPlane::Underworld => decode_world_map_bytes(plane, &bytes),
        WorldPlane::Britannia => {
            let data = read(&game_dir.join("DATA.OVL"))?;
            let chunk_index = find_britannia_chunk_index(&data)?;
            decode_britannia_map_bytes(&bytes, &chunk_index)
        }
    }
}
