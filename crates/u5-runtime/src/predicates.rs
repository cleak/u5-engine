//! Tile passability/water/lava/door predicates plus table-match helpers used during runtime checks.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

/// `catalogs/tile-catalog.md` §6: any tile id in the door family
/// (`96..=103`) is a town/world door tile that blocks movement when closed
/// and dispatches the door-interaction handler.
pub const fn is_town_door_tile(tile: u8) -> bool {
    tile >= TOWN_DOOR_TILE_FIRST && tile <= TOWN_DOOR_TILE_LAST
}

/// `catalogs/tile-catalog.md` §6: town stair tile-id family
/// (`0xC4..=0xC7`). The low two bits of the tile id are the
/// movement-wrapper-normalised facing.
pub const fn is_town_stair_tile(tile: u8) -> bool {
    tile >= TOWN_STAIR_TILE_FIRST && tile <= TOWN_STAIR_TILE_LAST
}

/// `catalogs/tile-catalog.md` §6: NPC floor-link marker tiles consumed by
/// the schedule pathfinder's tile-ID variant. The two bytes are
/// authored-only annotations, not ordinary furniture; do not treat them as
/// passable terrain without consulting the schedule spec.
pub const fn is_npc_floor_link_tile(tile: u8) -> bool {
    tile == NPC_FLOOR_LINK_TILE_A || tile == NPC_FLOOR_LINK_TILE_B
}

/// `vehicles.md` §6: a ship-transport marker byte (either hoisted
/// `0x20..=0x23` or furled `0x24..=0x27`).
pub const fn is_ship_transport_marker(byte: u8) -> bool {
    (byte >= SHIP_TRANSPORT_HOISTED_FIRST && byte <= SHIP_TRANSPORT_HOISTED_LAST)
        || (byte >= SHIP_TRANSPORT_FURLED_FIRST && byte <= SHIP_TRANSPORT_FURLED_LAST)
}

/// `vehicles.md` §6: ship hoisted/wind-control marker byte `0x20..=0x23`.
pub const fn is_ship_transport_hoisted(byte: u8) -> bool {
    byte >= SHIP_TRANSPORT_HOISTED_FIRST && byte <= SHIP_TRANSPORT_HOISTED_LAST
}

/// `vehicles.md` §6: ship furled/manual marker byte `0x24..=0x27`.
pub const fn is_ship_transport_furled(byte: u8) -> bool {
    byte >= SHIP_TRANSPORT_FURLED_FIRST && byte <= SHIP_TRANSPORT_FURLED_LAST
}

/// `overworld.md §3` per-chunk live-substitution rewrite. After a
/// chunk is loaded into the live 16x16 chunk buffer, the loader walks
/// the cells and applies a fixed substitution pass:
///
/// - Tile ids `0x16..=0x18` rewrite to `0xDF` unconditionally.
/// - Tile id `0x19` rewrites to `0x1A` only when the chunk
///   high-byte classifier accepts the current chunk descriptor.
///
/// The substitution affects only the live chunk buffer; the on-disk
/// chunk is unchanged. The classifier acceptance state is supplied
/// by the caller (the `chunk_classifier_accepts` flag).
pub const LIVE_CHUNK_SUBSTITUTION_TARGET_DF: u8 = 0xDF;
pub const LIVE_CHUNK_SUBSTITUTION_TARGET_1A: u8 = 0x1A;

pub const fn live_chunk_substituted_tile(
    tile: u8,
    chunk_classifier_accepts: bool,
) -> u8 {
    match tile {
        0x16..=0x18 => LIVE_CHUNK_SUBSTITUTION_TARGET_DF,
        0x19 if chunk_classifier_accepts => LIVE_CHUNK_SUBSTITUTION_TARGET_1A,
        other => other,
    }
}

/// `movement.md §4` ship terrain predicate (under sail or furled).
/// Ships accept only the sentinel and the deep-water / water tile
/// ids `0x00..=0x02`; sail state changes cadence and X-Xit rules,
/// not the static terrain query.
pub const SHIP_TERRAIN_ACCEPTED_TILES: [u8; 3] = [0x00, 0x01, 0x02];

/// `movement.md §4`: returns `true` when a ship-class actor can
/// stand on the supplied static map tile. Used by the shared
/// tile-class dispatcher for both the under-sail (`0x20..=0x23`)
/// and furled (`0x24..=0x27`) ship query families.
pub const fn ship_terrain_accepts(tile: u8) -> bool {
    matches!(tile, 0x00..=0x02)
}

/// `movement.md §4` water-creature / pirate-ship active-object
/// terrain predicate (`0x2C..=0x2F` query family). Same accepted
/// set as the ship predicate.
pub const fn water_creature_terrain_accepts(tile: u8) -> bool {
    ship_terrain_accepts(tile)
}

/// `movement.md §4` chair-tile force-reject range `0x90..=0x93`. The
/// base bitset would otherwise allow these ids; most query classes
/// force-reject them. Two query families exempt themselves from the
/// force reject: the on-foot/avatar family `0x1C..=0x1F` and the
/// `0x40` query family (single-id query).
pub const MOVEMENT_CHAIR_FORCE_REJECT_FIRST: u8 = 0x90;
pub const MOVEMENT_CHAIR_FORCE_REJECT_LAST: u8 = 0x93;

/// `movement.md §4`: returns `true` when the static-terrain
/// dispatcher's force-reject for the chair tile range applies for
/// this query class. The on-foot family and the `0x40` query are
/// exempt; everything else respects the reject.
pub const fn movement_chair_force_reject_applies(query_class: u8, tile: u8) -> bool {
    if tile < MOVEMENT_CHAIR_FORCE_REJECT_FIRST
        || tile > MOVEMENT_CHAIR_FORCE_REJECT_LAST
    {
        return false;
    }
    // Exempt families: on-foot avatar 0x1C..=0x1F and 0x40 query.
    if (query_class >= 0x1C && query_class <= 0x1F) || query_class == 0x40 {
        return false;
    }
    true
}

/// `movement.md §4` outdoor active-object query families that
/// accept exactly one static tile id rather than running a wider
/// predicate. Returns `Some(tile_id)` for the four named single-
/// tile families and `None` for any other class byte.
pub const fn outdoor_active_object_single_tile_query(class_byte: u8) -> Option<u8> {
    Some(match class_byte {
        0xE0..=0xE3 => 0x07,
        0xEC..=0xEF => 0x01,
        0xF4..=0xF7 => 0x05,
        0xF8..=0xFB => 0x04,
        _ => return None,
    })
}

/// `vehicles.md §2` transport/action marker family. Classifies the
/// party transport state byte into one of the documented ranges. The
/// low two bits within each family encode N/E/S/W facing using the
/// `0` north, `1` east, `2` south, `3` west convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportFamily {
    /// `0x12..=0x13` — mounted horse.
    MountedHorse,
    /// `0x14..=0x17` — magic carpet.
    MagicCarpet,
    /// `0x1C..=0x1F` — foot/avatar (clean default `0x1C` faces north).
    Foot,
    /// `0x20..=0x23` — ship under sail (hoisted, wind-controlled).
    ShipHoisted,
    /// `0x24..=0x27` — ship furled / manually handled.
    ShipFurled,
    /// `0x28..=0x2B` — skiff.
    Skiff,
}

/// `vehicles.md §2`: classify a transport/action marker byte into
/// its family. Returns `None` for marker values outside the known
/// transport ranges (those remain opaque transport state, per spec).
pub const fn transport_family(marker: u8) -> Option<TransportFamily> {
    Some(match marker {
        0x12..=0x13 => TransportFamily::MountedHorse,
        0x14..=0x17 => TransportFamily::MagicCarpet,
        0x1C..=0x1F => TransportFamily::Foot,
        0x20..=0x23 => TransportFamily::ShipHoisted,
        0x24..=0x27 => TransportFamily::ShipFurled,
        0x28..=0x2B => TransportFamily::Skiff,
        _ => return None,
    })
}

/// `vehicles.md §2`: low two bits decode transport facing as
/// north (0) / east (1) / south (2) / west (3). Returns `None` for
/// markers outside the recognised transport families.
pub const fn transport_facing_index(marker: u8) -> Option<u8> {
    if transport_family(marker).is_some() {
        Some(marker & 0x03)
    } else {
        None
    }
}

/// `stats-panel.md §5` middle-counter selection. The bottom block's
/// middle counter shows the saved party gold word in ordinary and
/// combat scenes; when the transport/action marker byte is in the
/// ship family `0x20..=0x27`, that slot instead shows the current
/// ship hull condition from active-object byte `+5`. The classifier
/// uses only the marker family — there is no separate parked-object
/// validation before reading the active vehicle hull byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatsPanelMiddleCounter {
    /// Show party gold word, right-aligned.
    PartyGold,
    /// Show ship-status label and hull condition byte.
    ShipHullCondition,
}

/// `stats-panel.md §5`: classify the middle-counter slot from the
/// transport/action marker byte.
pub const fn stats_panel_middle_counter(transport_marker: u8) -> StatsPanelMiddleCounter {
    if is_ship_transport_marker(transport_marker) {
        StatsPanelMiddleCounter::ShipHullCondition
    } else {
        StatsPanelMiddleCounter::PartyGold
    }
}

/// `vehicles.md` §6: low two bits of a ship transport marker decode heading
/// as north (0), east (1), south (2), west (3). Returns `None` for non-ship
/// bytes.
pub const fn ship_transport_heading_index(byte: u8) -> Option<u8> {
    if is_ship_transport_marker(byte) {
        Some(byte & 0x03)
    } else {
        None
    }
}

/// `formats/under-dat.md §2,§3`: file offset for tile `(x, y)` in the
/// underworld map. Every logical chunk is stored (no chunk-index
/// table) so the offset is `chunk_slot * 256 + offset_in_chunk`.
pub const fn under_file_offset(x: u8, y: u8) -> usize {
    brit_chunk_slot(x, y) * CHUNK_BYTES + brit_offset_in_chunk(x, y)
}

/// `formats/brit-dat.md §3`: logical chunk slot for the world
/// coordinate `(x, y)`. Coordinates are wrapped modulo the
/// 256-cell world side first.
pub const fn brit_chunk_slot(x: u8, y: u8) -> usize {
    let cx = (x as usize) / CHUNK_SIDE;
    let cy = (y as usize) / CHUNK_SIDE;
    cy * WORLD_CHUNKS_PER_SIDE + cx
}

/// `formats/brit-dat.md §3`: byte offset within a 256-byte stored
/// chunk for the world coordinate `(x, y)`.
pub const fn brit_offset_in_chunk(x: u8, y: u8) -> usize {
    let lx = (x as usize) % CHUNK_SIDE;
    let ly = (y as usize) % CHUNK_SIDE;
    ly * CHUNK_SIDE + lx
}

/// `formats/brit-dat.md §3`: file offset for tile `(x, y)` given the
/// chunk-index table entry for that tile's chunk slot. Returns `None`
/// when the table entry is the [`BRIT_WATER_SENTINEL`] (the loader
/// substitutes deep water for that case rather than reading from
/// disk).
pub const fn brit_file_offset(table_entry: u8, x: u8, y: u8) -> Option<usize> {
    if table_entry == BRIT_WATER_SENTINEL {
        return None;
    }
    Some((table_entry as usize) * CHUNK_BYTES + brit_offset_in_chunk(x, y))
}

/// `formats/look2-dat.md §3`: byte offset of the terrain-domain
/// table entry for `tile_id` inside the LOOK2.DAT offset table.
pub const fn look2_terrain_table_offset(tile_id: u8) -> usize {
    (tile_id as usize) * 2
}

/// `formats/look2-dat.md §3`: byte offset of the object-domain table
/// entry for `object_id` inside the LOOK2.DAT offset table.
pub const fn look2_object_table_offset(object_id: u8) -> usize {
    LOOK2_DAT_OBJECT_DOMAIN_BASE + (object_id as usize) * 2
}

/// `encounters.md §6`: which monster a sleep-ambush picks for a given
/// uniform 0..8 row roll. Giant Rat occupies rows 0 and 1 (so 2/8); the
/// remaining six rows are Troll, Bat, Slime, Giant Spider, Gremlin, and
/// Headless. Returns `None` for out-of-range rolls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleepAmbushMonster {
    GiantRat,
    Troll,
    Bat,
    Slime,
    GiantSpider,
    Gremlin,
    Headless,
}

/// `encounters.md §6`: select the sleep-ambush monster for a uniform row
/// roll in `0..8`. Returns `None` for `roll >= 8`.
pub const fn sleep_ambush_monster(row: u8) -> Option<SleepAmbushMonster> {
    Some(match row {
        0 | 1 => SleepAmbushMonster::GiantRat,
        2 => SleepAmbushMonster::Troll,
        3 => SleepAmbushMonster::Bat,
        4 => SleepAmbushMonster::Slime,
        5 => SleepAmbushMonster::GiantSpider,
        6 => SleepAmbushMonster::Gremlin,
        7 => SleepAmbushMonster::Headless,
        _ => return None,
    })
}

/// `encounters.md §6`: PRNG outcome that flips the rest loop into the
/// sleep-ambush branch. The shared integer PRNG produces sixty-four
/// outcomes per eligible predicate invocation; only the zero outcome
/// interrupts. Caller passes the raw 0..64 roll.
pub const SLEEP_AMBUSH_INTERRUPT_DENOMINATOR: u8 = 64;
pub const fn sleep_ambush_rest_interrupted(roll: u8) -> bool {
    roll == 0
}

/// `encounters.md §3`: random-encounter spawn threshold for an outdoor
/// per-turn block. Caller rolls `random(1, 30)` and spawns when
/// `roll < threshold`. Returns the threshold per the public table:
///   - Underworld plane: 3 (no hour adjustment).
///   - Surface no-encounter band 0x20..=0x26: 0 by day, 3 at hours 0..=4.
///   - Surface tile 0x04 or wilderness band 0x09..=0x0F: 2 by day, 5 at
///     hours 0..=4.
///   - Any other surface tile: 1 by day, 4 at hours 0..=4.
/// `formats/saved-gam.md §10` dungeon room-clear bitmap shape. The
/// 16-byte bitmap covers eight dungeons (`0..7`) by sixteen room
/// ids (`0..15`), giving 128 bits total. Layout is dungeon-major
/// then room-major: dungeon `D` occupies the bits at byte offsets
/// `D*2..=D*2+1`, with low bit = room id 0.
pub const SAVE_DUNGEON_ROOM_CLEAR_BYTES_PER_DUNGEON: usize = 2;
pub const SAVE_DUNGEON_ROOM_CLEAR_ROOMS_PER_DUNGEON: usize = 16;

/// `formats/saved-gam.md §10`: returns the (byte_offset_within_bitmap,
/// bit_mask) pair for a (dungeon, room_id) coordinate. Returns
/// `None` for out-of-range coordinates (dungeon `>= 8` or room id
/// `>= 16`).
pub const fn dungeon_room_clear_bit_position(
    dungeon: u8,
    room_id: u8,
) -> Option<(usize, u8)> {
    if dungeon >= 8 || room_id >= 16 {
        return None;
    }
    let byte = dungeon as usize * SAVE_DUNGEON_ROOM_CLEAR_BYTES_PER_DUNGEON
        + (room_id as usize) / 8;
    let mask = 1u8 << (room_id % 8);
    Some((byte, mask))
}

/// `encounters.md §4` candidate-terrain branch the encounter
/// spawner takes once a coordinate has passed the separation gate.
/// Caller supplies the world tile id and the underworld plane flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnTerrainBranch {
    /// Surface tile `0x01` after the low-tile allowance — 1/7 special
    /// whirlpool roll, otherwise surface default/aquatic bucket.
    SurfaceTile1WhirlpoolOrAquatic,
    /// Terrain tile `0x07` — 1/3 sea-serpent adjacency special roll;
    /// failure rejects the candidate.
    SeaSerpentAdjacency,
    /// Terrain tile `0x04` on the full underworld plane — direct Rot
    /// Worm sprite-run selection.
    UnderworldTile4RotWorm,
    /// Surface town-outline tile `0x0C`/`0x0D` — reject.
    HardReject,
    /// Low/shore/road/bridge tile that needs the 1/4 allowance die
    /// before bucket selection.
    LowTileAllowance,
    /// Tile passes through to the land bucket selected by world
    /// plane (`0x00..=0x0F` after the special/hard-reject cases plus
    /// `0x30..=0x33`).
    LandBucket,
    /// Tile id at or above `0x10` not otherwise listed — reject.
    HighTileReject,
}

/// `encounters.md §4`: classify a candidate world tile into the
/// spawner's terrain branch. Caller supplies the underworld flag
/// (`true` only when the saved Z byte indicates the underworld
/// plane). Tile-4 reaches `UnderworldTile4RotWorm` only on the
/// underworld; other tile-4 cases continue to the land bucket.
pub const fn spawn_terrain_branch(tile: u8, underworld: bool) -> SpawnTerrainBranch {
    if underworld && tile == 0x04 {
        return SpawnTerrainBranch::UnderworldTile4RotWorm;
    }
    if tile == 0x0C || tile == 0x0D {
        return SpawnTerrainBranch::HardReject;
    }
    if tile == 0x07 {
        return SpawnTerrainBranch::SeaSerpentAdjacency;
    }
    if tile == 0x01 {
        return SpawnTerrainBranch::SurfaceTile1WhirlpoolOrAquatic;
    }
    if tile < 0x04
        || (tile >= 0x60 && tile <= 0x6F)
        || (tile >= 0xD4 && tile <= 0xD7)
        || (tile >= 0xE4 && tile <= 0xE7)
    {
        return SpawnTerrainBranch::LowTileAllowance;
    }
    if tile < 0x10 || (tile >= 0x30 && tile <= 0x33) {
        return SpawnTerrainBranch::LandBucket;
    }
    SpawnTerrainBranch::HighTileReject
}

/// `encounters.md §4` whirlpool-special chance gate (1-in-7 on
/// surface tile 1).
pub const SPAWN_WHIRLPOOL_DENOMINATOR: u8 = 7;

/// `encounters.md §4` sea-serpent adjacency chance gate (1-in-3
/// on terrain tile 7); failure rejects the candidate.
pub const SPAWN_SEA_SERPENT_DENOMINATOR: u8 = 3;

/// `encounters.md §4` low-tile allowance die (1-in-4 on the low /
/// shore / road / bridge bands); failure rejects the candidate.
pub const SPAWN_LOW_TILE_ALLOWANCE_DENOMINATOR: u8 = 4;

/// `encounters.md §4` encounter-spawner retry budget. The retry
/// loop returns silently after this many rejected candidates.
pub const ENCOUNTER_SPAWNER_RETRY_LIMIT: u8 = 128;

/// `encounters.md §4` sea-creature spawner auxiliary seed. The
/// pirate-ship / water-creature facing-frame family `0x2C..=0x2F`
/// initialises its auxiliary byte to this value, which seeds the
/// outdoor animation/wander counter. Every other spawned monster
/// class initialises the auxiliary byte to zero.
pub const SEA_CREATURE_SPAWN_AUX_SEED: u8 = 100;

/// `encounters.md §4`: returns `true` when a newly-spawned
/// active-object's class byte belongs to the pirate-ship /
/// water-creature facing-frame family `0x2C..=0x2F` and should
/// therefore receive [`SEA_CREATURE_SPAWN_AUX_SEED`] in its auxiliary
/// byte rather than the default zero seed.
pub const fn sea_creature_spawn_seeds_aux(class_byte: u8) -> bool {
    matches!(class_byte, 0x2C..=0x2F)
}

/// `encounters.md §4` minimum/maximum candidate-coordinate
/// separation from the party. Both X and Y separations must be
/// strictly greater than `MIN` (keeps the spawn outside the
/// immediate visible centre) and strictly less than `MAX` (rejects
/// wrapped-near coordinates on the 256-by-256 torus).
pub const ENCOUNTER_SPAWNER_MIN_SEPARATION: u8 = 6;
pub const ENCOUNTER_SPAWNER_MAX_SEPARATION: u8 = 250;

/// `encounters.md §4`: returns `true` when a candidate `(slot_x,
/// slot_y)` passes the spawner's separation gate against the
/// party's `(player_x, player_y)`. Both axes must be in the
/// `(MIN, MAX)` open interval; either axis failing rejects the
/// candidate.
pub const fn encounter_spawner_separation_ok(
    slot_x: u8,
    slot_y: u8,
    player_x: u8,
    player_y: u8,
) -> bool {
    let dx = slot_x.abs_diff(player_x);
    let dy = slot_y.abs_diff(player_y);
    dx > ENCOUNTER_SPAWNER_MIN_SEPARATION
        && dx < ENCOUNTER_SPAWNER_MAX_SEPARATION
        && dy > ENCOUNTER_SPAWNER_MIN_SEPARATION
        && dy < ENCOUNTER_SPAWNER_MAX_SEPARATION
}

/// `encounters.md §4` sea-creature wander-counter seed. Sea-creature
/// class spawns receive this auxiliary value to seed their outdoor
/// animation/wander counter; other classes start with zero.
pub const SEA_CREATURE_WANDER_SEED: u8 = 100;

/// `encounters.md §4` random-spawn bucket weight tables. The picker
/// rolls on a `0..=255` scale, walks weights in the order below,
/// and returns the first row whose cumulative weight covers the
/// roll. Each entry pairs a weight with the first byte of the
/// payload sprite run (the rest of the run follows the canonical
/// `(base, base+1, base+2, base+3)` four-frame layout).
pub const SURFACE_AQUATIC_BUCKET: [(u8, u8); 5] = [
    (72, 0x8C), // Shark
    (72, 0x84), // Squid
    (40, 0x88), // Sea Serpent
    (38, 0x80), // Sea Horse
    (34, 0x2C), // Pirate-ship / water-creature facing frames
];

pub const UNDERWORLD_AQUATIC_BUCKET: [(u8, u8); 2] = [
    (128, 0x84), // Squid
    (128, 0x88), // Sea Serpent
];

pub const SURFACE_LAND_BUCKET: [(u8, u8); 12] = [
    (60, 0xC0), // Orc
    (50, 0xC8), // Python
    (40, 0x90), // Giant Rat
    (30, 0x98), // Giant Spider
    (20, 0xBC), // Insect Swarm
    (15, 0xC4), // Skeleton
    (15, 0xD0), // Headless
    (10, 0xE4), // Troll
    (10, 0xCC), // Ettin
    (3, 0xD4),  // Wisp
    (2, 0xDC),  // Dragon
    (1, 0xD8),  // Daemon
];

pub const UNDERWORLD_LAND_BUCKET: [(u8, u8); 7] = [
    (64, 0x94), // Bat
    (56, 0x90), // Giant Rat
    (56, 0x98), // Giant Spider
    (32, 0xF0), // Mongbat
    (32, 0xF4), // Corpser
    (8, 0xD8),  // Daemon
    (8, 0xDC),  // Dragon
];

/// `encounters.md §4`: walk a weighted bucket and return the first
/// payload whose cumulative weight covers the supplied `0..=255`
/// roll. The bucket entries pair `(weight, payload)`. Returns the
/// last entry's payload as a defensive fallback when the roll
/// exceeds the cumulative weight (the spec lists no overflow case;
/// shipped buckets sum to ~256 so this matters only for short
/// custom buckets).
pub const fn pick_random_spawn_bucket(bucket: &[(u8, u8)], roll: u8) -> Option<u8> {
    if bucket.is_empty() {
        return None;
    }
    let mut cumulative: u16 = 0;
    let mut i: usize = 0;
    while i < bucket.len() {
        let (weight, payload) = bucket[i];
        cumulative += weight as u16;
        if (roll as u16) < cumulative {
            return Some(payload);
        }
        i += 1;
    }
    Some(bucket[bucket.len() - 1].1)
}

/// `encounters.md §3` random-encounter roll bound. The probe rolls
/// uniformly in `[1, 30]`; spawn fires when `roll < threshold`. The
/// effective per-eligible-turn chance is `(threshold - 1) / 30`,
/// with thresholds 0 and 1 both producing no encounter.
pub const RANDOM_ENCOUNTER_ROLL_BOUND: u8 = 30;

/// `encounters.md §3`: returns `true` when a `random(1, 30)` roll
/// fires the encounter spawner. Spawn fires when `roll < threshold`.
pub const fn random_encounter_probe_spawns(roll: u8, threshold: u8) -> bool {
    roll < threshold
}

/// `encounters.md §3`: per-eligible-turn spawn count out of 30
/// outcomes. Returns `0` for thresholds `0` and `1` (no spawn);
/// returns `threshold - 1` for thresholds `2..=30`. Caller divides by
/// 30 to get the spawn probability.
pub const fn random_encounter_spawn_outcomes(threshold: u8) -> u8 {
    if threshold == 0 {
        0
    } else {
        threshold - 1
    }
}

pub const fn random_encounter_threshold(underworld: bool, tile: u8, hour: u8) -> u8 {
    if underworld {
        return 3;
    }
    let night = hour <= 4;
    match tile {
        0x20..=0x26 => {
            if night { 3 } else { 0 }
        }
        0x04 | 0x09..=0x0F => {
            if night { 5 } else { 2 }
        }
        _ => {
            if night { 4 } else { 1 }
        }
    }
}

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

/// `visibility.md` §6: tile identities that fully stop the centre-out sight
/// carve. These are spec-listed family members (forest 0x09, hill/mountain
/// /lava-rock 0x0A/0x0C/0x0D, bookshelf/dresser/vanity/trunk 0x4D..=0x4F,
/// sign-post 0x5A, monster sprite frames Bat 0x97 / Gargoyle 0xB8..=0xB9 /
/// Insect Swarm 0xBC / Headless 0xD0..=0xD3 / Rot Worm 0xF8 / Shadow Lord
/// 0xFE..=0xFF). Tiles not in this set or in the orthogonal-only set use
/// the ordinary propagation rule.
pub const fn tile_blocks_sight_propagation(tile: u8) -> bool {
    matches!(
        tile,
        0x09 | 0x0A | 0x0C | 0x0D
            | 0x4D..=0x4F
            | 0x5A
            | 0x97
            | 0xB8 | 0xB9
            | 0xBC
            | 0xD0..=0xD3
            | 0xF8
            | 0xFE | 0xFF,
    )
}

/// `visibility.md` §6: tile identities that propagate the carve only when
/// orthogonally adjacent to the centre cell — bookshelf/dresser
/// 0x4A..=0x4B, Giant Spider frame 0x98, Gargoyle frames 0xBA..=0xBB.
pub const fn tile_propagates_sight_only_when_adjacent(tile: u8) -> bool {
    matches!(tile, 0x4A | 0x4B | 0x98 | 0xBA | 0xBB)
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
