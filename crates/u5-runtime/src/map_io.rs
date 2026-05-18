//! TLK parsing, NPC block parsing, and scene/dungeon/world map loaders + decoders.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TlkHeaderEntry {
    npc_id: u16,
    blob_offset: usize,
}

pub fn parse_tlk(path: &Path) -> io::Result<HashMap<u16, Vec<String>>> {
    let bytes = read(path)?;
    parse_tlk_bytes(&bytes)
}

/// Like [`parse_tlk_bytes`] but returns the raw bytes of each NUL-terminated
/// field in every NPC blob. The bytes are still bit-7 XOR-encoded in their
/// on-disk form so callers can feed them directly into
/// [`crate::tlk_runner::run_tlk_stream`] without round-tripping through the
/// string decoder. Each inner `Vec<u8>` does **not** include the NUL
/// separator itself; that boundary is preserved by the outer split.
pub fn parse_tlk_blob_fields_raw(bytes: &[u8]) -> io::Result<HashMap<u16, Vec<Vec<u8>>>> {
    let entries = parse_tlk_header_entries(bytes)?;
    let mut span_entries = entries.clone();
    span_entries.sort_by_key(|entry| entry.blob_offset);
    let mut out: HashMap<u16, Vec<Vec<u8>>> = HashMap::new();
    for (idx, entry) in span_entries.iter().enumerate() {
        let nominal_end = span_entries
            .get(idx + 1)
            .map(|next| next.blob_offset)
            .unwrap_or(bytes.len());
        let end = nominal_end.min(entry.blob_offset.saturating_add(1024));
        let mut fields: Vec<Vec<u8>> = Vec::new();
        let mut pos = entry.blob_offset;
        let mut current: Vec<u8> = Vec::new();
        while pos < end && fields.len() < 40 {
            let byte = bytes[pos];
            pos += 1;
            if byte == 0 {
                fields.push(std::mem::take(&mut current));
                if fields.len() >= 5 && current.is_empty() && pos >= end {
                    break;
                }
                continue;
            }
            current.push(byte);
        }
        if !current.is_empty() {
            fields.push(current);
        }
        out.insert(entry.npc_id, fields);
    }
    if !out.contains_key(&1) {
        if let Some(first) = entries.first() {
            if let Some(fields) = out.get(&first.npc_id).cloned() {
                out.insert(1, fields);
            }
        }
    }
    Ok(out)
}

/// Convenience wrapper: read the supplied path and parse the raw-bytes
/// fields per NPC id.
pub fn parse_tlk_raw(path: &Path) -> io::Result<HashMap<u16, Vec<Vec<u8>>>> {
    let bytes = read(path)?;
    parse_tlk_blob_fields_raw(&bytes)
}

pub fn parse_tlk_bytes(bytes: &[u8]) -> io::Result<HashMap<u16, Vec<String>>> {
    let entries = parse_tlk_header_entries(bytes)?;
    let mut span_entries = entries.clone();
    span_entries.sort_by_key(|entry| entry.blob_offset);
    let mut out = HashMap::new();
    for (idx, entry) in span_entries.iter().enumerate() {
        let nominal_end = span_entries
            .get(idx + 1)
            .map(|next| next.blob_offset)
            .unwrap_or(bytes.len());
        let end = nominal_end.min(entry.blob_offset.saturating_add(1024));
        let mut fields = Vec::new();
        let mut pos = entry.blob_offset;
        while pos < end && fields.len() < 40 {
            let (field, next) = decode_tlk_field(&bytes, pos, end);
            fields.push(field);
            pos = next;
            if pos == end {
                break;
            }
        }
        out.insert(entry.npc_id, fields);
    }
    if !out.contains_key(&1) {
        if let Some(first) = entries.first() {
            if let Some(fields) = out.get(&first.npc_id).cloned() {
                out.insert(1, fields);
            }
        }
    }
    Ok(out)
}

fn parse_tlk_header_entries(bytes: &[u8]) -> io::Result<Vec<TlkHeaderEntry>> {
    if bytes.len() < 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "short TLK"));
    }
    let count = u16_at(&bytes, 0) as usize;
    if count < 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("TLK header count must include at least the sentinel, got {count}"),
        ));
    }
    let header_len = count
        .checked_mul(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "TLK header length overflows"))?;
    if header_len > bytes.len() || header_len > 512 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("TLK header length {header_len} is outside the available fixed header window"),
        ));
    }
    if count == 1 {
        return Ok(Vec::new());
    }
    let sentinel_id = u16_at(bytes, 2);
    if sentinel_id != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("TLK leading sentinel id must be 1, got {sentinel_id}"),
        ));
    }
    let mut entries = Vec::new();
    let mut last_id = 1u16;
    for k in 1..count {
        let off = u16_at(bytes, 4 * k) as usize;
        let id = u16_at(bytes, 4 * k + 2);
        if id <= last_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TLK header npc id {id} is not strictly after {last_id}"),
            ));
        }
        if off < header_len || off >= bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TLK header npc id {id} has invalid blob offset {off}"),
            ));
        }
        entries.push(TlkHeaderEntry {
            npc_id: id,
            blob_offset: off,
        });
        last_id = id;
    }
    Ok(entries)
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
            0x86 => {
                if pos < end {
                    let action = (bytes[pos] & 0x7f) as char;
                    s.push_str(&format!("{{ACTION:{action}}}"));
                    pos += 1;
                }
            }
            0x8c => pos = (pos + 1).min(end),
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

/// Like [`talk_keyword_response`] but returns the matched field's index
/// into the blob (0-based). Used by callers that need the raw bytes for
/// the byte-runner rather than the decoded string. Returns the index of
/// the response field, not the keyword field.
pub fn resolve_keyword_response_field_index(fields: &[String], keyword: &str) -> Option<usize> {
    if talk_keyword_matches("NAME", keyword) {
        return Some(0);
    }
    if talk_keyword_matches("JOB", keyword) || talk_keyword_matches("WORK", keyword) {
        return Some(3);
    }
    if talk_keyword_matches("BYE", keyword) || talk_keyword_matches("THANK", keyword) {
        return Some(4);
    }
    fields
        .get(5..)
        .unwrap_or_default()
        .chunks_exact(2)
        .enumerate()
        .find_map(|(pair_idx, pair)| {
            talk_keyword_matches(&pair[0], keyword).then_some(5 + pair_idx * 2 + 1)
        })
}

pub fn talk_keyword_response<'a>(fields: &'a [String], keyword: &str) -> Option<&'a str> {
    if talk_keyword_matches("NAME", keyword) {
        return fields.first().map(String::as_str);
    }
    if talk_keyword_matches("JOB", keyword) || talk_keyword_matches("WORK", keyword) {
        return fields.get(3).map(String::as_str);
    }
    if talk_keyword_matches("BYE", keyword) || talk_keyword_matches("THANK", keyword) {
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

pub fn talk_response_text_and_actions(response: &str) -> (String, Vec<char>) {
    let mut text = String::new();
    let mut actions = Vec::new();
    let mut rest = response;
    while let Some(start) = rest.find("{ACTION:") {
        text.push_str(&rest[..start]);
        let after = &rest[start + "{ACTION:".len()..];
        if let Some(end) = after.find('}') {
            if let Some(action) = after[..end].chars().next() {
                actions.push(action.to_ascii_uppercase());
            }
            rest = &after[end + 1..];
        } else {
            text.push_str(&rest[start..]);
            rest = "";
        }
    }
    text.push_str(rest);
    (compact(&text), actions)
}

/// `conversation.md §10`: per-scene TALK branch-flag bank width.
/// The IF/ELSE branch (`0x8C`) tests one of 32 bits within the active
/// scene's TALK branch-flag slot. Bit indices at or above this value
/// build a zero mask rather than wrapping, so such IF tests read as
/// clear and such SET-FLAG writes are no-ops.
pub const TALK_BRANCH_FLAG_BANK_BITS: u8 = 32;

pub const fn talk_branch_flag_mask(bit_index: u8) -> u32 {
    if bit_index < TALK_BRANCH_FLAG_BANK_BITS {
        1u32 << bit_index
    } else {
        0
    }
}

pub const fn talk_branch_flag_is_set(slot: u32, bit_index: u8) -> bool {
    let mask = talk_branch_flag_mask(bit_index);
    mask != 0 && slot & mask != 0
}

pub fn set_talk_branch_flag(slot: &mut u32, bit_index: u8) -> bool {
    let mask = talk_branch_flag_mask(bit_index);
    let before = *slot;
    *slot |= mask;
    *slot != before
}

pub fn talk_shop_trigger(dialog_id: u8) -> Option<(&'static str, &'static str)> {
    match dialog_id {
        0x81 => Some(("Weaponsmith / armourer", "Arms stock arm")),
        0x82 => Some((
            "Tavern / meal counter / sage-style rumour flow",
            "Interactive tavern arm",
        )),
        0x83 => Some(("Horse trader", "Vehicle-sale arm")),
        0x84 => Some(("Ship broker / shipwright", "Shipwright sale arm")),
        0x85 => Some(("Herbalist", "Reagent arm")),
        0x86 => Some(("Guildmaster", "Guild arm")),
        0x87 => Some(("Healer / sanctum", "Healer arm")),
        0x88 => Some(("Innkeeper", "Inn arm")),
        _ => None,
    }
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
    let bytes = read(&game_dir.join(DUNGEON_DAT_FILENAME))?;
    if bytes.len() != DUNGEON_DAT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{DUNGEON_DAT_FILENAME} must be {DUNGEON_DAT_LEN} bytes, got {}",
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
            let data = read(&game_dir.join(DATA_OVL_FILENAME))?;
            let chunk_index = find_britannia_chunk_index(&data)?;
            decode_britannia_map_bytes(&bytes, &chunk_index)
        }
    }
}
