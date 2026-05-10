//! Britannia/underworld chunk decoders, BRIT.DAT chunk index finder, map analysis (analyze_map, harvest_location_markers, etc.), pathfinding helpers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn decode_world_map_bytes(plane: WorldPlane, bytes: &[u8]) -> io::Result<Vec<u8>> {
    match plane {
        WorldPlane::Underworld => decode_underworld_map_bytes(bytes),
        WorldPlane::Britannia => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "direct BRIT.DAT decoding also needs DATA.OVL; use load_world_map",
        )),
    }
}

pub fn decode_underworld_map_bytes(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.len() != UNDER_DAT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "UNDER.DAT must be {UNDER_DAT_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes.to_vec())
}

pub fn find_britannia_chunk_index(data: &[u8]) -> io::Result<[u8; WORLD_CHUNK_COUNT]> {
    if data.len() < WORLD_CHUNK_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DATA.OVL is too short to contain a Britannia chunk-index table",
        ));
    }
    let mut found: Option<usize> = None;
    for offset in 0..=data.len() - WORLD_CHUNK_COUNT {
        if validate_britannia_chunk_index(&data[offset..offset + WORLD_CHUNK_COUNT]).is_ok() {
            if found.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "DATA.OVL contains multiple Britannia chunk-index candidates",
                ));
            }
            found = Some(offset);
        }
    }
    let offset = found.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "DATA.OVL contains no Britannia chunk-index candidate",
        )
    })?;
    let mut table = [0; WORLD_CHUNK_COUNT];
    table.copy_from_slice(&data[offset..offset + WORLD_CHUNK_COUNT]);
    Ok(table)
}

pub fn validate_britannia_chunk_index(table: &[u8]) -> io::Result<()> {
    if table.len() != WORLD_CHUNK_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Britannia chunk index must be {WORLD_CHUNK_COUNT} bytes, got {}",
                table.len()
            ),
        ));
    }
    let mut seen = vec![false; BRIT_STORED_CHUNKS];
    let mut stored = 0usize;
    let mut water = 0usize;
    for &entry in table {
        if entry == BRIT_WATER_SENTINEL {
            water += 1;
        } else {
            let entry = entry as usize;
            if entry >= BRIT_STORED_CHUNKS || seen[entry] {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid Britannia chunk-index entry",
                ));
            }
            seen[entry] = true;
            stored += 1;
        }
    }
    if stored != BRIT_STORED_CHUNKS || water != WORLD_CHUNK_COUNT - BRIT_STORED_CHUNKS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Britannia chunk-index counts do not match BRIT.DAT shape",
        ));
    }
    if seen.iter().any(|value| !value) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Britannia chunk-index does not reference every stored chunk",
        ));
    }
    Ok(())
}

pub fn decode_britannia_map_bytes(bytes: &[u8], table: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.len() != BRIT_DAT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("BRIT.DAT must be {BRIT_DAT_LEN} bytes, got {}", bytes.len()),
        ));
    }
    validate_britannia_chunk_index(table)?;
    let mut out = vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS];
    for (chunk_slot, &entry) in table.iter().enumerate() {
        let chunk_x = chunk_slot % WORLD_CHUNKS_PER_SIDE;
        let chunk_y = chunk_slot / WORLD_CHUNKS_PER_SIDE;
        for local_y in 0..CHUNK_SIDE {
            let dst_y = chunk_y * CHUNK_SIDE + local_y;
            let dst_start = dst_y * WORLD_SIDE + chunk_x * CHUNK_SIDE;
            if entry == BRIT_WATER_SENTINEL {
                out[dst_start..dst_start + CHUNK_SIDE].fill(BRIT_DEEP_WATER_TILE);
            } else {
                let src_start = entry as usize * CHUNK_BYTES + local_y * CHUNK_SIDE;
                out[dst_start..dst_start + CHUNK_SIDE]
                    .copy_from_slice(&bytes[src_start..src_start + CHUNK_SIDE]);
            }
        }
    }
    Ok(out)
}

pub fn analyze_map(scene: Scene, floor: usize, grid: &[u8]) -> MapStats {
    let LocationMarkers {
        npc_markers,
        spawn_markers,
    } = harvest_location_markers(grid);
    let mut door_count = 0;
    let mut stair_count = 0;
    let mut class_histogram: HashMap<&'static str, usize> = HashMap::new();
    let mut hash = 0xcbf29ce484222325u64;

    for y in 0..32 {
        for x in 0..32 {
            let tile = grid[y * 32 + x];
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

pub fn harvest_location_markers(grid: &[u8]) -> LocationMarkers {
    let mut npc_markers = Vec::new();
    let mut spawn_markers = Vec::new();
    for x in 0..32 {
        for y in 0..32 {
            let tile = grid[y * 32 + x];
            if is_npc_start_marker(tile) {
                npc_markers.push((x, y));
            }
            if is_spawn_marker(tile) {
                spawn_markers.push((x, y));
            }
        }
    }
    LocationMarkers {
        npc_markers,
        spawn_markers,
    }
}

pub fn scrub_location_entry_markers(grid: &mut [u8]) {
    for tile in grid {
        if is_location_entry_marker(*tile) {
            *tile = LOCATION_MARKER_CLEANUP_TILE;
        }
    }
}

pub fn is_location_entry_marker(tile: u8) -> bool {
    is_spawn_marker(tile) || is_npc_start_marker(tile)
}

pub fn is_spawn_marker(tile: u8) -> bool {
    tile == 0x2a
}

pub fn is_npc_start_marker(tile: u8) -> bool {
    (tile & 0xfe) == 0x48
}

pub fn append_map_stats(report: &mut String, stats: &MapStats) {
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

pub fn find_path(
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

pub fn door_probe(grid: &[u8]) -> Option<((usize, usize), bool)> {
    let idx = grid.iter().position(|tile| (96..=103).contains(tile))?;
    let mut live = grid.to_vec();
    // The exact original open-door tile is intentionally not asserted here.
    // This smoke probe exercises the spec's tile-id rewrite model without
    // publishing raw map data or pinning unresolved door variants.
    live[idx] = 16;
    Some(((idx % 32, idx / 32), is_probe_walkable(live[idx])))
}

pub fn neighbors(x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
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

pub fn first_walkable(grid: &[u8], passability: Option<&TilePassability>) -> Option<(usize, usize)> {
    grid.iter()
        .position(|tile| is_tile_walkable(*tile, passability))
        .map(|idx| (idx % 32, idx / 32))
}

pub fn first_distinct_walkable(grid: &[u8], start: (usize, usize)) -> Option<(usize, usize)> {
    grid.iter()
        .enumerate()
        .find(|(idx, tile)| {
            let pos = (idx % 32, idx / 32);
            pos != start && is_probe_walkable(**tile)
        })
        .map(|(idx, _)| (idx % 32, idx / 32))
}
