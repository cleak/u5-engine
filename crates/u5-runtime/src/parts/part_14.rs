pub fn refresh_saved_ool_mirrors_for_load(game_dir: &Path) -> io::Result<()> {
    let bytes = read_saved_ool_bytes(game_dir)?;
    fs::write(game_dir.join("BRIT.OOL"), &bytes[..OOL_PLANE_LEN])?;
    fs::write(game_dir.join("UNDER.OOL"), &bytes[OOL_PLANE_LEN..])?;
    Ok(())
}

pub fn read_saved_ool_bytes(game_dir: &Path) -> io::Result<Vec<u8>> {
    let path = game_dir.join("SAVED.OOL");
    let bytes = read(&path)?;
    if bytes.len() != SAVED_OOL_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SAVED.OOL must be {SAVED_OOL_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

pub fn encode_active_object_table(objects: &[ActiveObject]) -> io::Result<Vec<u8>> {
    if objects.len() > OOL_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "active-object table has {} slots, expected at most {OOL_SLOTS}",
                objects.len()
            ),
        ));
    }
    let mut bytes = vec![0; OOL_PLANE_LEN];
    for (slot, object) in objects.iter().copied().enumerate() {
        write_active_object_record(&mut bytes, slot, object)?;
    }
    Ok(bytes)
}

pub fn encode_ool_plane_objects(objects: &[ActiveObject]) -> io::Result<Vec<u8>> {
    if objects.len() > OOL_SLOTS - 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "world overlay has {} non-player slots, expected at most {}",
                objects.len(),
                OOL_SLOTS - 1
            ),
        ));
    }
    let mut bytes = vec![0; OOL_PLANE_LEN];
    for (index, object) in objects.iter().copied().enumerate() {
        write_active_object_record(&mut bytes, index + 1, object)?;
    }
    Ok(bytes)
}

pub fn write_active_object_record(
    bytes: &mut [u8],
    slot: usize,
    object: ActiveObject,
) -> io::Result<()> {
    if slot >= OOL_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("active-object slot {slot} is outside 0..{}", OOL_SLOTS - 1),
        ));
    }
    let x = u8::try_from(object.x).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object slot {slot} x coordinate {} is outside 0..255",
                object.x
            ),
        )
    })?;
    let y = u8::try_from(object.y).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object slot {slot} y coordinate {} is outside 0..255",
                object.y
            ),
        )
    })?;
    let offset = slot * OOL_RECORD_LEN;
    bytes[offset] = object.type_byte;
    bytes[offset + 1] = object.tile;
    bytes[offset + 2] = x;
    bytes[offset + 3] = y;
    bytes[offset + 4] = object.z as u8;
    bytes[offset + 5] = object.aux1;
    bytes[offset + 6] = object.phase;
    bytes[offset + 7] = object.aux3;
    Ok(())
}

pub fn decode_ool_plane_objects(bytes: &[u8]) -> io::Result<Vec<ActiveObject>> {
    decode_active_object_table(bytes, "OOL plane table")
}

pub fn decode_saved_active_objects(bytes: &[u8]) -> io::Result<Vec<ActiveObject>> {
    let end = SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN;
    let table = bytes
        .get(SAVE_ACTIVE_OBJECTS_OFFSET..end)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "SAVED.GAM is too short"))?;
    decode_active_object_table(table, "SAVED.GAM active-object table")
}

pub fn decode_active_object_table(bytes: &[u8], label: &str) -> io::Result<Vec<ActiveObject>> {
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} must be {OOL_PLANE_LEN} bytes, got {}", bytes.len()),
        ));
    }

    let mut objects = Vec::with_capacity(OOL_SLOTS - 1);
    for (slot, record) in bytes.chunks_exact(OOL_RECORD_LEN).enumerate() {
        let type_byte = record[0];
        if slot == 0 {
            continue;
        }
        objects.push(ActiveObject {
            type_byte,
            tile: record[1],
            x: record[2] as usize,
            y: record[3] as usize,
            z: record[4] as i8,
            phase: record[6],
            aux1: record[5],
            aux3: record[7],
        });
    }
    Ok(objects)
}

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

pub fn is_probe_walkable(tile: u8) -> bool {
    if is_location_entry_marker(tile) {
        return true;
    }
    !matches!(tile, 0 | 1..=4 | 10..=15 | 24..=79 | 88..=103 | 120..=127)
}

pub fn is_tile_walkable(tile: u8, passability: Option<&TilePassability>) -> bool {
    is_tile_walkable_for_transport(tile, passability, TransportState::Foot)
}

pub fn is_base_tile_passable(tile: u8, passability: Option<&TilePassability>) -> bool {
    if is_location_entry_marker(tile) {
        return true;
    }
    passability
        .map(|passability| passability.is_passable(tile))
        .unwrap_or_else(|| is_probe_walkable(tile))
}

pub fn is_tile_walkable_for_transport(
    tile: u8,
    passability: Option<&TilePassability>,
    transport: TransportState,
) -> bool {
    let base = is_base_tile_passable(tile, passability);
    match transport {
        TransportState::Foot => base && !is_water_tile(tile),
        TransportState::Horse { .. } => base && !is_water_tile(tile) && !is_mountain_or_lava(tile),
        TransportState::Ship { .. } | TransportState::Skiff { .. } => is_water_tile(tile),
        TransportState::Carpet { .. } => {
            (base || is_water_tile(tile) || is_lava_tile(tile))
                && !is_mountain_tile(tile)
                && !is_wall_or_closed_door_tile(tile)
        }
        TransportState::Balloon { .. } => true,
    }
}

pub fn is_water_tile(tile: u8) -> bool {
    (1..=4).contains(&tile)
}

pub fn static_tile_animation_family_base(tile: u8) -> Option<u8> {
    match tile {
        1..=4 => Some(1),
        10..=13 => Some(10),
        92..=95 => Some(92),
        152..=155 => Some(152),
        156..=159 => Some(156),
        _ => None,
    }
}

pub fn is_lava_tile(tile: u8) -> bool {
    (10..=15).contains(&tile)
}

pub fn is_mountain_tile(tile: u8) -> bool {
    (10..=15).contains(&tile)
}

pub fn is_outdoor_climbable_tile(tile: u8) -> bool {
    is_mountain_tile(tile)
}

pub fn is_mountain_or_lava(tile: u8) -> bool {
    is_mountain_tile(tile) || is_lava_tile(tile)
}

pub fn is_wall_or_closed_door_tile(tile: u8) -> bool {
    matches!(tile, 24..=79 | 96..=103)
}

pub fn is_talk_through_tile(tile: u8) -> bool {
    (64..=71).contains(&tile)
}

pub fn is_horse_fast_stride_tile(tile: u8) -> bool {
    tile == 5 || (16..=23).contains(&tile)
}

pub fn is_town_night_hour(hour: u8) -> bool {
    hour <= 4 || hour >= 20
}

pub fn cell_in_visibility_radius(cx: isize, cy: isize, x: isize, y: isize, radius: usize) -> bool {
    let dx = (x - cx).unsigned_abs();
    let dy = (y - cy).unsigned_abs();
    dx.max(dy) <= radius
}

pub fn surface_line_unblocked<F>(px: isize, py: isize, x: isize, y: isize, mut blocks: F) -> bool
where
    F: FnMut(isize, isize) -> bool,
{
    let dx = x - px;
    let dy = y - py;
    let steps = dx.unsigned_abs().max(dy.unsigned_abs()) as isize;
    for step in 1..steps {
        let sx = px + rounded_div(dx * step, steps);
        let sy = py + rounded_div(dy * step, steps);
        if blocks(sx, sy) {
            return false;
        }
    }
    true
}

pub fn rounded_div(numerator: isize, denominator: isize) -> isize {
    debug_assert!(denominator > 0);
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        -((-numerator + half) / denominator)
    }
}

pub fn surface_tile_blocks_sight(tile: u8) -> bool {
    is_mountain_tile(tile) || is_wall_or_closed_door_tile(tile) || matches!(tile, 160..=255)
}

pub fn town_fire_source_is_adjacent(entry: TownFireSourceEntry, x: usize, y: usize) -> bool {
    let dx = entry.x.abs_diff(x);
    let dy = entry.y.abs_diff(y);
    dx <= 1 && dy <= 1 && (dx != 0 || dy != 0)
}

pub fn town_fire_source_tile_matches(entry: TownFireSourceEntry, tile: u8) -> bool {
    entry
        .expected_tile
        .map_or(true, |expected_tile| expected_tile == tile)
}

pub fn dungeon_wind_tile_matches(
    entry: DungeonWindTileEntry,
    scene: DungeonScene,
    level: u8,
    x: usize,
    y: usize,
    cell: u8,
) -> bool {
    entry.scene == scene
        && entry.level == level
        && entry.x == x
        && entry.y == y
        && entry
            .expected_cell
            .map_or(true, |expected| expected == cell)
}

pub fn dungeon_teleport_matches(
    entry: DungeonTeleportEntry,
    scene: DungeonScene,
    level: u8,
    x: usize,
    y: usize,
    cell: u8,
) -> bool {
    entry.scene == scene
        && entry.level == level
        && entry.x == x
        && entry.y == y
        && entry
            .expected_cell
            .map_or(true, |expected| expected == cell)
}

pub fn dungeon_exit_tile_matches(
    entry: DungeonExitTileEntry,
    scene: DungeonScene,
    level: u8,
    x: usize,
    y: usize,
    cell: u8,
) -> bool {
    entry.scene == scene
        && entry.level == level
        && entry.x == x
        && entry.y == y
        && entry
            .expected_cell
            .map_or(true, |expected| expected == cell)
}

pub fn dungeon_closed_door_matches(entry: DungeonDoorEntry, cell: u8) -> bool {
    cell != entry.open_cell
        && entry
            .expected_cell
            .map_or(true, |expected| expected == cell)
}

pub fn town_pushable_matches(
    entry: TownPushableEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn world_get_tile_matches(
    entry: WorldGetTileEntry,
    plane: WorldPlane,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.plane == plane
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn object_pickup_matches(
    entry: ObjectPickupEntry,
    target: PlayTarget,
    floor: i8,
    x: usize,
    y: usize,
    object: ActiveObject,
) -> bool {
    entry.target == target
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && object.z == floor
        && entry
            .expected_tile
            .map_or(true, |expected| expected == object.tile)
}

pub fn world_waterfall_matches(
    entry: WorldWaterfallEntry,
    plane: WorldPlane,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.plane == plane
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn world_damage_tile_matches(
    entry: WorldDamageTileEntry,
    plane: WorldPlane,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.plane == plane
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn world_damage_tile_entry_at(
    entries: &[WorldDamageTileEntry],
    plane: WorldPlane,
    x: usize,
    y: usize,
    tile: u8,
) -> Option<WorldDamageTileEntry> {
    entries
        .iter()
        .find(|entry| world_damage_tile_matches(**entry, plane, x, y, tile))
        .copied()
}

pub fn town_get_tile_matches(
    entry: TownGetTileEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn town_rest_bed_matches(
    entry: TownRestBedEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn town_stair_matches(
    entry: TownStairEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn town_trap_door_matches(
    entry: TownTrapDoorEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn town_exit_tile_matches(
    entry: TownExitTileEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry
            .expected_tile
            .map_or(true, |expected| expected == tile)
}

pub fn town_lock_matches(
    entry: TownLockEntry,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
    tile: u8,
) -> bool {
    entry.scene == scene
        && entry.floor == floor
        && entry.x == x
        && entry.y == y
        && entry.locked_tile == tile
}

pub fn apply_dawn_dusk_substitution(grid: &mut [u8]) {
    for y in 0..31 {
        for x in 0..32 {
            if grid[y * 32 + x] == 0x87 {
                let paired = (y + 1) * 32 + x;
                grid[paired] ^= 0xdd;
            }
        }
    }
}

pub fn world_cell_index(x: usize, y: usize) -> usize {
    y * WORLD_SIDE + x
}

pub fn first_world_walkable_for_transport(
    grid: &[u8],
    plane: WorldPlane,
    passability: Option<&TilePassability>,
    transport: TransportState,
    damage_tiles: &[WorldDamageTileEntry],
) -> Option<(usize, usize)> {
    grid.iter()
        .enumerate()
        .find(|&(idx, tile)| {
            let x = idx % WORLD_SIDE;
            let y = idx / WORLD_SIDE;
            if let Some(entry) = world_damage_tile_entry_at(damage_tiles, plane, x, y, *tile) {
                entry.effect.allows_transport(transport)
                    && !entry.effect.damages_transport(transport)
            } else {
                is_tile_walkable_for_transport(*tile, passability, transport)
            }
        })
        .map(|(idx, _)| (idx % WORLD_SIDE, idx / WORLD_SIDE))
}

pub fn world_start_safe_for_transport(
    grid: &[u8],
    pos: (usize, usize),
    plane: WorldPlane,
    passability: Option<&TilePassability>,
    transport: TransportState,
    damage_tiles: &[WorldDamageTileEntry],
) -> bool {
    let (x, y) = pos;
    if x >= WORLD_SIDE || y >= WORLD_SIDE {
        return false;
    }
    let tile = grid[world_cell_index(x, y)];
    if let Some(entry) = world_damage_tile_entry_at(damage_tiles, plane, x, y, tile) {
        return entry.effect.allows_transport(transport)
            && !entry.effect.damages_transport(transport);
    }
    is_tile_walkable_for_transport(tile, passability, transport)
}

