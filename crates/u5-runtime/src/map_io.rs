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
    Ok(out)
}

/// `.TLK` header: a two-byte entry count followed by exactly that many
/// four-byte `(npc id, blob offset)` rows.
///
/// There is **no sentinel row**. Verified by decoding the shipped files:
/// `CASTLE.TLK` (40), `TOWNE.TLK` (48), `DWELLING.TLK` (15) and `KEEP.TLK`
/// (32) each carry ids exactly `1..=count`, in order, with row one's offset
/// equal to the header length - so every id, id 1 included, addresses its own
/// blob.
///
/// This previously read row zero as a sentinel, required its id to be 1,
/// skipped it, and then paired each row's *offset* with the *next* row's id.
/// That shifted every NPC's dialogue by one id and dropped the last NPC
/// entirely; the missing id 1 was then papered over by aliasing it onto the
/// first surviving blob.
fn parse_tlk_header_entries(bytes: &[u8]) -> io::Result<Vec<TlkHeaderEntry>> {
    if bytes.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "short TLK"));
    }
    let count = u16_at(bytes, 0) as usize;
    if count == 0 {
        return Ok(Vec::new());
    }
    let header_len = 2usize
        .checked_add(count.checked_mul(4).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "TLK header length overflows")
        })?)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "TLK header length overflows"))?;
    if header_len > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "TLK header length {header_len} exceeds the {} byte file",
                bytes.len()
            ),
        ));
    }

    let mut entries = Vec::with_capacity(count);
    let mut last_id = 0u16;
    for row in 0..count {
        let id = u16_at(bytes, 2 + 4 * row);
        let off = u16_at(bytes, 4 + 4 * row) as usize;
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

pub fn effective_npc_slots(slots: &[NpcSlot]) -> impl Iterator<Item = &NpcSlot> {
    slots.iter().filter(|slot| slot.slot != NPC_SENTINEL_SLOT)
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

/// `visibility.md §12.6` indoor beacon sources, harvested from the **raw**
/// floor before [`normalize_town_runtime_floor`] rewrites any cell.
///
/// This exists because the normalisation pass scrubs the location-entry
/// markers, and the bright-light tile the beacon looks for currently shares
/// a byte with the asterisk marker `formats/location-dat.md §6` describes.
/// `load_town_scene` already harvested from the raw grid; every floor
/// *transition* went through `load_town_runtime_floor` and then harvested
/// from the scrubbed result, so the two entry paths disagreed and a floor
/// reached by stairs lost its beacon source. Lighthouses have stairs, and
/// the four floors carrying that tile are lighthouse lantern rooms, so the
/// transition path was the one that mattered.
pub fn load_town_runtime_floor_with_beacon_sources(
    game_dir: &Path,
    scene: Scene,
    floor: i8,
    hour: u8,
) -> io::Result<(Vec<u8>, [Option<(u8, u8)>; 2])> {
    let mut grid = load_floor(game_dir, scene, floor)?;
    let sources = crate::harvest_location_beacon_sources(&grid);
    normalize_town_runtime_floor(&mut grid, hour);
    Ok((grid, sources))
}

pub fn normalize_town_runtime_floor(grid: &mut [u8], hour: u8) {
    scrub_location_npc_start_markers(grid);
    if is_town_night_hour(hour) {
        apply_dawn_dusk_substitution(grid);
    }
}

/// `formats/location-dat.md §4.1`: one location's page ownership inside
/// its class file. `base_page` is the page loaded when the floor byte is
/// zero; `first_page..=last_page` is the run of 1024-byte pages the
/// location owns.
///
/// The base page is *not* always the lowest page of the run: exactly four
/// locations enter above the bottom of theirs (Yew, both large castles,
/// and Serpent's Hold), which is what makes negative floor values
/// ordinary rather than exotic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPageRun {
    pub base_page: usize,
    pub first_page: usize,
    pub last_page: usize,
}

impl LocationPageRun {
    /// `formats/location-dat.md §4`: the signed floor values that address
    /// this run. A higher page index is a higher floor, so the lowest page
    /// is the most negative floor.
    pub const fn floor_range(&self) -> (i8, i8) {
        (
            (self.first_page as isize - self.base_page as isize) as i8,
            (self.last_page as isize - self.base_page as isize) as i8,
        )
    }

    pub const fn floor_count(&self) -> usize {
        self.last_page - self.first_page + 1
    }

    pub const fn contains_page(&self, page: usize) -> bool {
        self.first_page <= page && page <= self.last_page
    }
}

/// `formats/location-dat.md §4.1`: the complete per-scene base
/// floor-page binding for the shipped DOS data, indexed by `scene.byte -
/// 1`. Slot zero of the resident table is the overworld and is never
/// consulted by this path, so it is not represented here.
///
/// This is a **lookup, not a derivation**. The withdrawn model was
/// `page = sub_map_index * 2 + floor`, which is wrong for twenty-two of
/// the thirty-two locations and merely happens to be right for the other
/// ten — which is why it survived so long. `cleak/u5-spec#80` published
/// this table and withdrew both that arithmetic and the
/// "eight 2048-byte blocks, each a pair of floors" file model behind it:
/// a class file is a flat array of sixteen 1024-byte pages, seven
/// locations own three-page runs, two own five-page runs, and nine runs
/// cross a 2048-byte boundary. Do not reintroduce a block index here;
/// the unit is the page. (`Scene::block` survives only as the `.NPC` /
/// `.TLK` roster index, where the pairing does still hold.)
///
/// Two structural properties are asserted by the tests and are worth
/// relying on: the sixty-four pages **partition exactly** across the four
/// class files, and the four rows most likely to expose a `2 * index`
/// implementation are Yew, Iolo's Hut, and the two large castles.
pub const LOCATION_PAGE_RUNS: [LocationPageRun; 32] = [
    //  1 Moonglow
    LocationPageRun {
        base_page: 0,
        first_page: 0,
        last_page: 1,
    },
    //  2 Britain
    LocationPageRun {
        base_page: 2,
        first_page: 2,
        last_page: 3,
    },
    //  3 Jhelom
    LocationPageRun {
        base_page: 4,
        first_page: 4,
        last_page: 5,
    },
    //  4 Yew
    LocationPageRun {
        base_page: 7,
        first_page: 6,
        last_page: 7,
    },
    //  5 Minoc
    LocationPageRun {
        base_page: 8,
        first_page: 8,
        last_page: 9,
    },
    //  6 Trinsic
    LocationPageRun {
        base_page: 10,
        first_page: 10,
        last_page: 11,
    },
    //  7 Skara Brae
    LocationPageRun {
        base_page: 12,
        first_page: 12,
        last_page: 13,
    },
    //  8 New Magincia
    LocationPageRun {
        base_page: 14,
        first_page: 14,
        last_page: 15,
    },
    //  9 Fogsbane
    LocationPageRun {
        base_page: 0,
        first_page: 0,
        last_page: 2,
    },
    // 10 Stormcrow
    LocationPageRun {
        base_page: 3,
        first_page: 3,
        last_page: 5,
    },
    // 11 Greyhaven
    LocationPageRun {
        base_page: 6,
        first_page: 6,
        last_page: 8,
    },
    // 12 Waveguide
    LocationPageRun {
        base_page: 9,
        first_page: 9,
        last_page: 11,
    },
    // 13 Iolo's Hut
    LocationPageRun {
        base_page: 12,
        first_page: 12,
        last_page: 12,
    },
    // 14 DWELLING:5 (blank name)
    LocationPageRun {
        base_page: 13,
        first_page: 13,
        last_page: 13,
    },
    // 15 DWELLING:6 (blank name)
    LocationPageRun {
        base_page: 14,
        first_page: 14,
        last_page: 14,
    },
    // 16 DWELLING:7 (blank name)
    LocationPageRun {
        base_page: 15,
        first_page: 15,
        last_page: 15,
    },
    // 17 Lord British's Castle
    LocationPageRun {
        base_page: 1,
        first_page: 0,
        last_page: 4,
    },
    // 18 Lord Blackthorn's Castle
    LocationPageRun {
        base_page: 6,
        first_page: 5,
        last_page: 9,
    },
    // 19 West Britanny
    LocationPageRun {
        base_page: 10,
        first_page: 10,
        last_page: 10,
    },
    // 20 North Britanny
    LocationPageRun {
        base_page: 11,
        first_page: 11,
        last_page: 11,
    },
    // 21 East Britanny
    LocationPageRun {
        base_page: 12,
        first_page: 12,
        last_page: 12,
    },
    // 22 Paws
    LocationPageRun {
        base_page: 13,
        first_page: 13,
        last_page: 13,
    },
    // 23 Cove
    LocationPageRun {
        base_page: 14,
        first_page: 14,
        last_page: 14,
    },
    // 24 Buccaneer's Den
    LocationPageRun {
        base_page: 15,
        first_page: 15,
        last_page: 15,
    },
    // 25 Ararat
    LocationPageRun {
        base_page: 0,
        first_page: 0,
        last_page: 1,
    },
    // 26 Bordermarch
    LocationPageRun {
        base_page: 2,
        first_page: 2,
        last_page: 3,
    },
    // 27 Farthing
    LocationPageRun {
        base_page: 4,
        first_page: 4,
        last_page: 4,
    },
    // 28 Windemere
    LocationPageRun {
        base_page: 5,
        first_page: 5,
        last_page: 5,
    },
    // 29 Stonegate
    LocationPageRun {
        base_page: 6,
        first_page: 6,
        last_page: 6,
    },
    // 30 The Lycaeum
    LocationPageRun {
        base_page: 7,
        first_page: 7,
        last_page: 9,
    },
    // 31 Empath Abbey
    LocationPageRun {
        base_page: 10,
        first_page: 10,
        last_page: 12,
    },
    // 32 Serpent's Hold
    LocationPageRun {
        base_page: 14,
        first_page: 13,
        last_page: 15,
    },
];

/// `formats/location-dat.md §4.1`: page run owned by `scene`. Total for
/// every `Scene`, which is constructible only for scene bytes 1..=32.
pub fn location_page_run(scene: Scene) -> LocationPageRun {
    LOCATION_PAGE_RUNS[usize::from(scene.byte) - 1]
}

/// `formats/location-dat.md §4.1`: base page for the scene's logical
/// floor zero.
///
/// A `location_floor_pages.tsv` beside the game data still overrides the
/// published table, but it is now an override for *modified* assets, not
/// a fallback for missing spec: with no file present the published table
/// answers every scene, so there is nothing left to derive and no warning
/// to emit.
pub fn resolve_location_base_page(game_dir: &Path, scene: Scene) -> io::Result<usize> {
    if let Some(base_page) = load_location_floor_entries(game_dir)?.and_then(|entries| {
        entries
            .iter()
            .find(|entry| entry.scene == scene)
            .map(|entry| entry.base_page)
    }) {
        return Ok(base_page);
    }

    Ok(location_page_run(scene).base_page)
}

/// `formats/location-dat.md §4`: `page = base_page[scene] +
/// sign_extend_8(floor_byte)`, and exactly 1024 bytes are read at
/// `page * 1024`.
///
/// `floor` is signed in this use: `0xFF` is one floor *below* the entry
/// floor, not floor 255. A higher page index is a higher floor, so a
/// transition that raises the floor byte goes up and one that lowers it
/// goes down. Inverting this puts every basement above its ground floor.
pub fn resolve_location_floor_page(game_dir: &Path, scene: Scene, floor: i8) -> io::Result<usize> {
    let run = location_page_run(scene);
    let base_page = resolve_location_base_page(game_dir, scene)?;
    let page = base_page as i16 + floor as i16;

    // `formats/location-dat.md §4.1`: the sixty-four pages partition
    // exactly, so a page outside this scene's own run belongs to a
    // *different* location. Reading it renders someone else's map —
    // exactly the class of bug the published table exists to prevent
    // (`block * 2` sent Lord Blackthorn's Castle into Lord British's).
    // This is also what makes "is there a floor above/below me?" a real
    // question: the answer is the run, not the 0..15 page bound.
    //
    // Only enforced when the base came from the published table. A
    // `location_floor_pages.tsv` override is for modified assets, where
    // the shipped run no longer describes the file, so it keeps the
    // plain page bound below.
    if base_page == run.base_page && !run.contains_page(page.clamp(0, 255) as usize) {
        let (lowest, highest) = run.floor_range();
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} floor {} is outside its published floor range {}..={} (pages {}..={})",
                scene.key(),
                floor,
                lowest,
                highest,
                run.first_page,
                run.last_page
            ),
        ));
    }

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
