//! Tile passability/water/lava/door predicates plus table-match helpers used during runtime checks.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

pub fn is_probe_walkable(tile: u8) -> bool {
    if is_location_entry_marker(tile) {
        return true;
    }
    // Class boundaries derived from canonical LOOK2.DAT and actual U5
    // gameplay (cross-checked with u5-spec/catalogs/tile-catalog.md and
    // systems/visibility.md). Notable corrections to the old code:
    //   0x04 swamp           -- walkable on foot (poisons the party)
    //   0x0a tropical forest -- walkable, BUT blocks sight (dense)
    //   0x0b/0x0e/0x0f foothills -- walkable hills
    //   0x0c mountains, 0x0d high peaks -- impassable except balloon
    !matches!(
        tile,
        // Sentinel.
        0
        // Open water: deep water, coastal water, shoals (impassable on
        // foot; 0x04 swamp is walkable so it is NOT in this set).
        | 1..=3
        // True mountains and high peaks. Foothills (0x0b/0x0e/0x0f),
        // tropical forest (0x0a), and swamp (0x04) are all walkable.
        | 0x0c | 0x0d
        // Dungeon entrance, mystic shrine, ruined shrine, lighthouse
        // (landmarks the player can E-Enter but not step over).
        | 24..=27
        // Roofs and crystal sphere.
        | 39..=41
        // Hollow stump, crops, fruit tree, cactus.
        | 43 | 45..=47
        // Gargoyle landmark and "a mighty castle" tile band.
        | 56..=63
        // Town interior surfaces that act as obstacles: planks, codex,
        // mast, rail, cobble, pillar, pier (but NOT bridges).
        | 64..=71
        // Walls, arrow slits, windows, piles of rocks.
        | 74..=79
        // Signs, wells, brazier, fireplace.
        | 88..=95
        // Doors (id-dependent; closed/locked block).
        | 96..=103
        // Decorative obstructions in the upper decoration band.
        | 120..=127
    )
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

/// True if the tile is open-ocean water that blocks foot movement and
/// requires a ship or skiff. Swamp (0x04) is NOT water for movement
/// purposes -- swamp is walkable terrain that poisons the party.
/// This matches LOOK2.DAT (water 0x01-0x03 vs swamp 0x04).
pub fn is_water_tile(tile: u8) -> bool {
    (1..=3).contains(&tile)
}

/// Returns `(family_base, cycle_length)` for an animated-static tile. The
/// renderer cycles the displayed sprite within `[base, base + cycle)` while
/// preserving each cell's per-tile identity offset. Returns `None` for
/// static tiles.
///
/// Only water actually animates in U5's 0..=255 map-tile range. Per a
/// LOOK2.DAT canonical cross-check:
///   * 0x01..=0x03 -- "deep water" / "water" / "shoals". 3-frame cycle.
///   * 0x04        -- "swamp". Static terrain, NOT a water frame.
///   * 0x0a..=0x0f -- "tropical forest" / "foothills" / "mountains" /
///                    "high peaks" / "foothills" / "foothills". The spec
///                    listed this band as a 4-frame lava cycle but the
///                    game data has six distinct static terrain types
///                    here. Mountains do not animate.
///   * 0x5c..=0x5f -- bookshelves and similar furniture (static).
///   * 0x98..=0x9b -- odd door / portcullis / tables with food (static).
///   * 0x9c..=0x9f -- tables with food / mirror (static).
/// Other animation families (fire field, poison field, sleep / energy
/// field) may exist in dungeon-mode and combat-mode tile spaces but those
/// run through separate animators.
pub fn static_tile_animation_family(tile: u8) -> Option<(u8, u8)> {
    match tile {
        1..=3 => Some((1, 3)),
        _ => None,
    }
}

pub fn is_lava_tile(tile: u8) -> bool {
    // Per LOOK2.DAT, tile 0x8F is "molten lava" (a single sprite). The
    // claim in the original code that 0x0a..=0x0f is lava came from the
    // tile-catalog spec; the actual game labels those ids as terrain
    // (tropical forest / foothills / mountains / high peaks / foothills).
    tile == 0x8f
}

/// True if the tile is an actual mountain or high peak per LOOK2.DAT.
/// Excludes foothills (which are walkable hills) and tropical forest
/// (which is a dense forest, not mountain). Used for impassability,
/// sight-blocking, and outdoor-climb gating.
pub fn is_mountain_tile(tile: u8) -> bool {
    matches!(tile, 0x0c | 0x0d)
}

/// True if the tile is "tropical forest" (dense forest interior). Per
/// the visibility spec, dense forest blocks sight but isn't a mountain.
pub fn is_dense_forest_tile(tile: u8) -> bool {
    tile == 0x0a
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

/// Sight-blocking predicate scoped to the overworld. Per
/// u5-spec/systems/visibility.md Section 6:
///   * Forest interior (deep woods) blocks sight.
///   * Mountains always block.
///   * Open ground (grass, sand, paths, water) does not.
///   * Hills (foothills) do NOT block sight -- the "see over the
///     mountain from a hill" mechanic doesn't exist but hills
///     themselves are transparent.
/// Indoor wall/door tile ranges are town-interior fixtures; the same
/// tile ids on the overworld are landmark icons (towns, signs, coastal
/// markers, dwellings) that should be visible from a distance.
pub fn world_surface_tile_blocks_sight(tile: u8) -> bool {
    is_mountain_tile(tile) || is_dense_forest_tile(tile)
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
    // Prefer a cell that has at least one walkable neighbour. The bare "first
    // walkable cell in linear scan" was landing on 1x1 islands surrounded by
    // water, leaving the player unable to move in any direction.
    let safe = |x: usize, y: usize| -> bool {
        let tile = grid[world_cell_index(x, y)];
        if let Some(entry) = world_damage_tile_entry_at(damage_tiles, plane, x, y, tile) {
            entry.effect.allows_transport(transport) && !entry.effect.damages_transport(transport)
        } else {
            is_tile_walkable_for_transport(tile, passability, transport)
        }
    };
    // Require enough walkable cells in the 3x3 neighbourhood that the player
    // can actually explore. Peninsulas with a single walkable neighbour are
    // technically valid but produce a near-stuck experience.
    let with_neighbours = grid.iter().enumerate().find(|&(idx, _)| {
        let x = idx % WORLD_SIDE;
        let y = idx / WORLD_SIDE;
        if !safe(x, y) {
            return false;
        }
        let mut count = 0;
        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                // World wraps.
                let nx = ((x as isize + dx).rem_euclid(WORLD_SIDE as isize)) as usize;
                let ny = ((y as isize + dy).rem_euclid(WORLD_SIDE as isize)) as usize;
                if safe(nx, ny) {
                    count += 1;
                }
            }
        }
        count >= 5
    });
    if let Some((idx, _)) = with_neighbours {
        return Some((idx % WORLD_SIDE, idx / WORLD_SIDE));
    }
    // Last-ditch fallback: take any walkable cell at all (degenerate map).
    grid.iter()
        .enumerate()
        .find(|&(idx, _)| safe(idx % WORLD_SIDE, idx / WORLD_SIDE))
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
