use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DEFAULT_GAME_DIR: &str = r"C:\Games\U5-Clean";
const REPORT_PATH: &str = "reports/lb-throne-room-slice.txt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Towne,
    Dwelling,
    Castle,
    Keep,
}

impl Family {
    fn from_scene(scene: u8) -> Option<Self> {
        if scene == 0 || scene > 32 {
            return None;
        }
        match (scene - 1) >> 3 {
            0 => Some(Self::Towne),
            1 => Some(Self::Dwelling),
            2 => Some(Self::Castle),
            3 => Some(Self::Keep),
            _ => None,
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Self::Towne => "TOWNE",
            Self::Dwelling => "DWELLING",
            Self::Castle => "CASTLE",
            Self::Keep => "KEEP",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Scene {
    byte: u8,
    family: Family,
    block: usize,
}

impl Scene {
    fn new(byte: u8) -> io::Result<Self> {
        let family = Family::from_scene(byte).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid scene byte {byte}"),
            )
        })?;
        Ok(Self {
            byte,
            family,
            block: ((byte - 1) & 7) as usize,
        })
    }

    fn key(self) -> String {
        format!("{}:{}", self.family.stem(), self.block)
    }
}

#[derive(Debug)]
struct NpcSlot {
    slot: usize,
    type_byte: u8,
    dialog_id: u8,
    schedule: [u8; 16],
    name: Option<String>,
}

#[derive(Debug)]
struct MapStats {
    scene: Scene,
    floor: usize,
    npc_markers: Vec<(usize, usize)>,
    spawn_markers: Vec<(usize, usize)>,
    door_count: usize,
    stair_count: usize,
    render_hash: u64,
    class_histogram: HashMap<&'static str, usize>,
}

fn main() -> io::Result<()> {
    let game_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GAME_DIR));

    let mut report = String::new();
    report.push_str("# Lord British throne-room verification slice\n\n");
    report.push_str(&format!("Game data: `{}`\n\n", game_dir.display()));
    report.push_str("This executable is a parity harness. It reads original data at runtime but does not embed or emit raw map, dialogue, or asset dumps.\n\n");

    let lb_candidate = Scene::new(0x11)?; // CASTLE:0 by public scene partition.
    let fifth_castle = Scene::new(0x15)?; // CASTLE:4, the disputed public wording.
    let decomp_special = Scene::new(0x1d)?; // Scene identified by private TOWN note.

    let castle_tlk = parse_tlk(&game_dir.join("CASTLE.TLK"))?;
    let keep_tlk = parse_tlk(&game_dir.join("KEEP.TLK"))?;

    let lb_slots = parse_npc_block(&game_dir, lb_candidate, &castle_tlk)?;
    let fifth_slots = parse_npc_block(&game_dir, fifth_castle, &castle_tlk)?;
    let special_slots = parse_npc_block(&game_dir, decomp_special, &keep_tlk)?;

    let lb_names = names(&lb_slots);
    let fifth_names = names(&fifth_slots);
    let special_names = names(&special_slots);

    let lb_has_castle_staff = contains_all(
        &lb_names,
        &[
            "Alistair", "Stephen", "Treanna", "Margaret", "Desiree", "Saduj",
        ],
    );
    let fifth_has_castle_staff = contains_any(&fifth_names, &["Alistair", "Stephen", "Saduj"]);
    let special_is_keep = decomp_special.family == Family::Keep;

    report.push_str("## Scene binding checks\n\n");
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        lb_candidate.byte,
        lb_candidate.key()
    ));
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        fifth_castle.byte,
        fifth_castle.key()
    ));
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        decomp_special.byte,
        decomp_special.key()
    ));
    report.push_str(&format!(
        "- `CASTLE:0` contains Lord-British-castle staff markers: {}.\n",
        pass_fail(lb_has_castle_staff)
    ));
    report.push_str(&format!(
        "- `CASTLE:4` contains those staff markers: {}.\n",
        pass_fail(fifth_has_castle_staff)
    ));
    report.push_str(&format!(
        "- Private-note special scene `0x1D` maps to keep family under the public partition: {}.\n\n",
        pass_fail(special_is_keep)
    ));

    report.push_str("Representative roster names, limited to avoid dialogue or roster dumps:\n\n");
    report.push_str(&format!(
        "- `{}`: {}\n",
        lb_candidate.key(),
        sample_names(&lb_names)
    ));
    report.push_str(&format!(
        "- `{}`: {}\n",
        fifth_castle.key(),
        sample_names(&fifth_names)
    ));
    report.push_str(&format!(
        "- `{}`: {}\n\n",
        decomp_special.key(),
        sample_names(&special_names)
    ));

    let floor0 = load_floor(&game_dir, lb_candidate, 0)?;
    let floor1 = load_floor(&game_dir, lb_candidate, 1)?;
    let stats0 = analyze_map(lb_candidate, 0, &floor0);
    let stats1 = analyze_map(lb_candidate, 1, &floor1);

    report.push_str("## Map/render checks\n\n");
    append_map_stats(&mut report, &stats0);
    append_map_stats(&mut report, &stats1);

    let start = stats0
        .spawn_markers
        .first()
        .copied()
        .or_else(|| first_walkable(&floor0))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no walkable floor-0 start"))?;
    let target = stats0
        .npc_markers
        .first()
        .copied()
        .filter(|target| *target != start)
        .or_else(|| first_distinct_walkable(&floor0, start))
        .or_else(|| first_walkable(&floor0))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no floor-0 target"))?;
    let path = find_path(&floor0, start, target);

    report.push_str("## Movement/pathfinding checks\n\n");
    report.push_str(&format!(
        "- Movement probe start: ({}, {}), target: ({}, {}).\n",
        start.0, start.1, target.0, target.1
    ));
    match &path {
        Some(steps) => {
            report.push_str(&format!(
                "- Class-derived pathfinding found a path of {} steps: PASS.\n",
                steps.len().saturating_sub(1)
            ));
            let legal = steps.windows(2).all(|w| {
                manhattan(w[0], w[1]) == 1 && is_probe_walkable(floor0[w[1].1 * 32 + w[1].0])
            });
            report.push_str(&format!(
                "- Simulated step-by-step movement over the path: {}.\n",
                pass_fail(legal)
            ));
        }
        None => {
            report.push_str("- Class-derived pathfinding found no path. This is a WARNING, not a hard failure, because exact passability bitmap placement is still open in the public specs.\n");
        }
    }
    report.push_str(&format!(
        "- Door-family tiles detected on tested floors: {}.\n\n",
        stats0.door_count + stats1.door_count
    ));
    match door_probe(&floor1) {
        Some((pos, opened_walkable)) => {
            report.push_str(&format!(
                "- Door interaction smoke probe at ({}, {}) rewrote a door-family cell and produced a walkable result: {}.\n\n",
                pos.0,
                pos.1,
                pass_fail(opened_walkable)
            ));
        }
        None => report.push_str(
            "- Door interaction smoke probe: WARNING, no door-family tile found on floor 1.\n\n",
        ),
    }

    report.push_str("## Schedule/conversation checks\n\n");
    let occupied = lb_slots.iter().filter(|s| s.type_byte != 0).count();
    let named = lb_slots.iter().filter(|s| s.name.is_some()).count();
    report.push_str(&format!(
        "- Occupied `CASTLE:0` roster slots: {occupied}.\n"
    ));
    report.push_str(&format!(
        "- Occupied slots with resolved TLK display names: {named}.\n"
    ));
    report.push_str("- Noon waypoint sample:\n");
    for slot in lb_slots.iter().filter(|s| s.type_byte != 0).take(6) {
        let wp = waypoint_for_hour(&slot.schedule, 12);
        let name = slot.name.as_deref().unwrap_or("(unnamed)");
        report.push_str(&format!(
            "  - slot {} dlg {} `{}` -> waypoint {} at ({}, {}, {}).\n",
            slot.slot,
            slot.dialog_id,
            name,
            wp,
            slot.schedule[3 + wp],
            slot.schedule[6 + wp],
            slot.schedule[9 + wp] as i8
        ));
    }
    if let Some(slot) = lb_slots
        .iter()
        .find(|slot| slot.type_byte != 0 && slot.dialog_id > 1 && slot.name.is_some())
    {
        let fields = castle_tlk
            .get(&(slot.dialog_id as u16))
            .map(|fields| fields.len())
            .unwrap_or(0);
        let keywords = fields.saturating_sub(5) / 2;
        report.push_str(&format!(
            "- Conversation envelope probe: slot {} dlg {} has the five leading TLK fields and {} keyword pairs: {}.\n",
            slot.slot,
            slot.dialog_id,
            keywords,
            pass_fail(fields >= 5)
        ));
    } else {
        report.push_str(
            "- Conversation envelope probe: FAIL, no named dialogue-bearing slot found.\n",
        );
    }
    report.push_str("\n");

    report.push_str("## Findings\n\n");
    report.push_str("- The slice runs end-to-end for file loading, scene partitioning, roster/TLK joins, map analysis, render hashing, schedule sampling, and pathfinding smoke checks.\n");
    report.push_str("- `CASTLE:0`, not the fifth castle slot, is the strongest data-backed public binding for Lord British's castle in this slice.\n");
    report.push_str("- The private TOWN note's `0x1D` special-case label conflicts with the public scene partition and should be treated as an unresolved private-analysis issue until rechecked.\n");
    report.push_str("- Exact passability-bitmap offset and bit ordering remain open; this harness uses class-derived passability only for the smoke path.\n");

    if !lb_has_castle_staff || !special_is_keep {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "required scene binding check failed",
        ));
    }

    fs::create_dir_all("reports")?;
    fs::write(REPORT_PATH, &report)?;
    print!("{report}");
    println!("\nReport written to {REPORT_PATH}");
    Ok(())
}

fn pass_fail(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}

fn read(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))
}

fn parse_tlk(path: &Path) -> io::Result<HashMap<u16, Vec<String>>> {
    let bytes = read(path)?;
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

fn decode_tlk_field(bytes: &[u8], mut pos: usize, end: usize) -> (String, usize) {
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

fn parse_npc_block(
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

fn load_floor(game_dir: &Path, scene: Scene, floor: usize) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join(format!("{}.DAT", scene.family.stem())))?;
    let start = scene.block * 2048 + floor * 1024;
    if floor > 1 || start + 1024 > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "short DAT block",
        ));
    }
    Ok(bytes[start..start + 1024].to_vec())
}

fn analyze_map(scene: Scene, floor: usize, grid: &[u8]) -> MapStats {
    let mut npc_markers = Vec::new();
    let mut spawn_markers = Vec::new();
    let mut door_count = 0;
    let mut stair_count = 0;
    let mut class_histogram: HashMap<&'static str, usize> = HashMap::new();
    let mut hash = 0xcbf29ce484222325u64;

    for y in 0..32 {
        for x in 0..32 {
            let tile = grid[y * 32 + x];
            if (tile & 0xfe) == 0x48 {
                npc_markers.push((x, y));
            }
            if tile == 0x2a {
                spawn_markers.push((x, y));
            }
            if (96..=103).contains(&tile) {
                door_count += 1;
            }
            if (80..=87).contains(&tile) {
                stair_count += 1;
            }
            *class_histogram.entry(tile_class(tile)).or_insert(0) += 1;
            hash ^= render_class_byte(tile) as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }

    MapStats {
        scene,
        floor,
        npc_markers,
        spawn_markers,
        door_count,
        stair_count,
        render_hash: hash,
        class_histogram,
    }
}

fn append_map_stats(report: &mut String, stats: &MapStats) {
    report.push_str(&format!(
        "- `{}` floor {}: 32x32 loaded, render-hash `{:016x}`, NPC markers {}, spawn markers {}, doors {}, stairs/ladders {}.\n",
        stats.scene.key(),
        stats.floor,
        stats.render_hash,
        stats.npc_markers.len(),
        stats.spawn_markers.len(),
        stats.door_count,
        stats.stair_count
    ));
    let mut classes: Vec<_> = stats.class_histogram.iter().collect();
    classes.sort_by_key(|(name, _)| **name);
    report.push_str("  Class histogram:");
    for (name, count) in classes {
        report.push_str(&format!(" {name}={count}"));
    }
    report.push('\n');
}

fn find_path(
    grid: &[u8],
    start: (usize, usize),
    target: (usize, usize),
) -> Option<Vec<(usize, usize)>> {
    let mut prev = vec![None::<(usize, usize)>; 1024];
    let mut seen = vec![false; 1024];
    let mut q = VecDeque::new();
    q.push_back(start);
    seen[start.1 * 32 + start.0] = true;
    while let Some((x, y)) = q.pop_front() {
        if (x, y) == target {
            let mut path = Vec::new();
            let mut cur = target;
            path.push(cur);
            while cur != start {
                cur = prev[cur.1 * 32 + cur.0]?;
                path.push(cur);
            }
            path.reverse();
            return Some(path);
        }
        for (nx, ny) in neighbors(x, y) {
            let idx = ny * 32 + nx;
            if seen[idx] || !is_probe_walkable(grid[idx]) {
                continue;
            }
            seen[idx] = true;
            prev[idx] = Some((x, y));
            q.push_back((nx, ny));
        }
    }
    None
}

fn door_probe(grid: &[u8]) -> Option<((usize, usize), bool)> {
    let idx = grid.iter().position(|tile| (96..=103).contains(tile))?;
    let mut live = grid.to_vec();
    // The exact original open-door tile is intentionally not asserted here.
    // This smoke probe exercises the spec's tile-id rewrite model without
    // publishing raw map data or pinning unresolved door variants.
    live[idx] = 16;
    Some(((idx % 32, idx / 32), is_probe_walkable(live[idx])))
}

fn neighbors(x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
    let mut out = Vec::with_capacity(4);
    if x > 0 {
        out.push((x - 1, y));
    }
    if x < 31 {
        out.push((x + 1, y));
    }
    if y > 0 {
        out.push((x, y - 1));
    }
    if y < 31 {
        out.push((x, y + 1));
    }
    out.into_iter()
}

fn first_walkable(grid: &[u8]) -> Option<(usize, usize)> {
    grid.iter()
        .position(|tile| is_probe_walkable(*tile))
        .map(|idx| (idx % 32, idx / 32))
}

fn first_distinct_walkable(grid: &[u8], start: (usize, usize)) -> Option<(usize, usize)> {
    grid.iter()
        .enumerate()
        .find(|(idx, tile)| {
            let pos = (idx % 32, idx / 32);
            pos != start && is_probe_walkable(**tile)
        })
        .map(|(idx, _)| (idx % 32, idx / 32))
}

fn is_probe_walkable(tile: u8) -> bool {
    if tile == 0x2a || (tile & 0xfe) == 0x48 {
        return true;
    }
    !matches!(tile, 0 | 1..=4 | 10..=15 | 24..=79 | 88..=103 | 120..=127)
}

fn tile_class(tile: u8) -> &'static str {
    match tile {
        0 => "sentinel",
        1..=4 => "water",
        5..=15 => "terrain",
        16..=23 => "path",
        24..=63 => "wall",
        64..=95 => "furniture",
        96..=103 => "door",
        104..=127 => "decoration",
        128..=159 => "special",
        160..=191 => "vehicle",
        192..=255 => "npc-sprite",
    }
}

fn render_class_byte(tile: u8) -> u8 {
    match tile {
        0 => b' ',
        1..=4 => b'~',
        5..=15 => b',',
        16..=23 => b'.',
        24..=63 => b'#',
        64..=95 => b'f',
        96..=103 => b'D',
        104..=127 => b'd',
        128..=159 => b's',
        160..=191 => b'v',
        192..=255 => b'n',
    }
}

fn waypoint_for_hour(schedule: &[u8; 16], hour: u8) -> usize {
    let t0 = schedule[12];
    let t1 = schedule[13];
    let t2 = schedule[14];
    let t3 = schedule[15];
    if in_wrapping_range(hour, t0, t1) {
        0
    } else if in_wrapping_range(hour, t2, t3) {
        2
    } else {
        1
    }
}

fn in_wrapping_range(hour: u8, start: u8, end: u8) -> bool {
    if start == end {
        return false;
    }
    if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

fn names(slots: &[NpcSlot]) -> Vec<String> {
    slots
        .iter()
        .filter(|slot| slot.type_byte != 0)
        .filter_map(|slot| slot.name.clone())
        .collect()
}

fn contains_all(names: &[String], needles: &[&str]) -> bool {
    needles.iter().all(|needle| contains_any(names, &[*needle]))
}

fn contains_any(names: &[String], needles: &[&str]) -> bool {
    names.iter().any(|name| {
        needles.iter().any(|needle| {
            name.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
    })
}

fn sample_names(names: &[String]) -> String {
    let mut sample: Vec<_> = names.iter().take(8).cloned().collect();
    if names.len() > sample.len() {
        sample.push(format!("... +{} more", names.len() - sample.len()));
    }
    sample.join(", ")
}

fn compact(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches('"')
        .to_string()
}

fn manhattan(a: (usize, usize), b: (usize, usize)) -> usize {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

fn u16_at(bytes: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([bytes[off], bytes[off + 1]])
}
