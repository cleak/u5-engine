//! Tile passability/water/lava/door predicates plus table-match helpers used during runtime checks.

use crate::*;

/// `doors-and-z-transitions.md §2`: exact dispersed top-down door
/// ids accepted by the live command handlers. The broad `96..=103`
/// catalog row is not a command predicate.
pub const fn is_town_door_tile(tile: u8) -> bool {
    town_command_door_tile(tile)
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

/// `formats/brit-dat.md §9.1` decisions for one loaded 16x16 chunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiveChunkSubstitutionPolicy {
    pub seal_entrances: bool,
    pub ruin_shrine: bool,
}

impl LiveChunkSubstitutionPolicy {
    pub const NONE: Self = Self {
        seal_entrances: false,
        ruin_shrine: false,
    };
}

/// Apply the two independent quest-gated tile substitutions to a copied cell.
/// The caller derives the policy from the chunk owner and save-backed flags;
/// the on-disk chunk is never changed.
pub const LIVE_CHUNK_SUBSTITUTION_TARGET_DF: u8 = 0xDF;
pub const LIVE_CHUNK_SUBSTITUTION_TARGET_1A: u8 = 0x1A;

pub const fn live_chunk_substituted_tile(tile: u8, policy: LiveChunkSubstitutionPolicy) -> u8 {
    match tile {
        0x16..=0x18 if policy.seal_entrances => LIVE_CHUNK_SUBSTITUTION_TARGET_DF,
        0x19 if policy.ruin_shrine => LIVE_CHUNK_SUBSTITUTION_TARGET_1A,
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

/// `movement.md §4` on-foot/avatar static terrain predicate.
/// Dynamic occupants and command-specific effects are checked after this
/// static tile-id acceptance layer.
pub const fn foot_terrain_accepts(tile: u8) -> bool {
    matches!(
        tile,
        0x00
            | 0x04..=0x0B
            | 0x0E..=0x19
            | 0x1B
            | 0x1D..=0x26
            | 0x2C..=0x2D
            | 0x30..=0x37
            | 0x39
            | 0x3E
            | 0x40
            | 0x44..=0x45
            | 0x47..=0x49
            | 0x6A..=0x6B
            | 0x86..=0x87
            | 0x8C
            | 0x8F..=0x93
            | 0xAA..=0xAC
            | 0xBC
            | 0xC4..=0xC9
            | 0xDC..=0xDD
            | 0xF9
            | 0xFF
    )
}

/// `movement.md §4` mounted-horse static terrain predicate.
pub const fn horse_terrain_accepts(tile: u8) -> bool {
    matches!(
        tile,
        0x00
            | 0x05..=0x0B
            | 0x0E..=0x19
            | 0x1B
            | 0x1D..=0x26
            | 0x2C..=0x2D
            | 0x30..=0x37
            | 0x39
            | 0x3E
            | 0x40
            | 0x44..=0x45
            | 0x47..=0x49
            | 0x6A..=0x6B
            | 0x86..=0x87
            | 0x8C
            | 0xAA..=0xAC
            | 0xBC
            | 0xC4..=0xC9
            | 0xDC..=0xDD
            | 0xF9
            | 0xFF
    )
}

/// `movement.md §4` magic-carpet static terrain predicate.
pub const fn carpet_terrain_accepts(tile: u8) -> bool {
    matches!(
        tile,
        0x00..=0x0B
            | 0x0E..=0x19
            | 0x1B
            | 0x1D..=0x26
            | 0x2C..=0x2D
            | 0x30..=0x37
            | 0x39
            | 0x3E
            | 0x40
            | 0x44..=0x45
            | 0x47..=0x49
            | 0x60..=0x6F
            | 0x86..=0x87
            | 0x8C
            | 0x8F
            | 0xAA..=0xAC
            | 0xBC
            | 0xC4..=0xC9
            | 0xDC..=0xDD
            | 0xF9
            | 0xFF
    )
}

/// `movement.md §4` skiff static terrain predicate. The low two bits
/// of the skiff query marker select the north/east/south/west mask.
pub const fn skiff_terrain_accepts(tile: u8, facing_index: u8) -> bool {
    match facing_index & TRANSPORT_FACING_MASK {
        0 => matches!(
            tile,
            0x00..=0x03 | 0x36..=0x37 | 0x60 | 0x63..=0x64 | 0x66..=0x68 | 0x6A | 0x6C
        ),
        1 => matches!(
            tile,
            0x00..=0x03 | 0x34 | 0x37 | 0x61 | 0x64..=0x65 | 0x67..=0x69 | 0x6B | 0x6D
        ),
        2 => matches!(
            tile,
            0x00..=0x03 | 0x34..=0x35 | 0x60 | 0x62 | 0x65..=0x66 | 0x68..=0x6A | 0x6E
        ),
        _ => matches!(
            tile,
            0x00..=0x03 | 0x35..=0x36 | 0x61..=0x63 | 0x66..=0x67 | 0x69 | 0x6B | 0x6F
        ),
    }
}

/// `movement.md §4` chair-tile force-reject range `0x90..=0x93`. The
/// base bitset would otherwise allow these ids; most query classes
/// force-reject them. Two query families exempt themselves from the
/// force reject: the on-foot/avatar family `0x1C..=0x1F` and the
/// `0x40` query family (single-id query).
pub const MOVEMENT_CHAIR_FORCE_REJECT_FIRST: u8 = 0x90;
/// `movement.md §4` chair-tile force-reject range upper bound. The
/// range covers four facings (N/E/S/W) of the chair tile, so it
/// extends FIRST + TRANSPORT_FACING_MASK = `0x90 + 0b11` = `0x93`.
/// Anchored to `MOVEMENT_CHAIR_FORCE_REJECT_FIRST +
/// TRANSPORT_FACING_MASK` so the chair-tile range derives from the
/// transport facing-mask convention.
pub const MOVEMENT_CHAIR_FORCE_REJECT_LAST: u8 =
    MOVEMENT_CHAIR_FORCE_REJECT_FIRST + TRANSPORT_FACING_MASK;

/// `movement.md §4` on-foot/avatar query family (low two bits select
/// facing). This family is exempt from the chair-tile force-reject so
/// the avatar can sit on chair variants; see `vehicles.md §2` for the
/// matching transport-marker range.
pub const MOVEMENT_QUERY_FOOT_AVATAR_FIRST: u8 = 0x1C;
/// `movement.md §4` on-foot/avatar query family upper bound. The
/// family covers four facings (low two bits), so it extends
/// FIRST + TRANSPORT_FACING_MASK = `0x1C + 0b11` = `0x1F`.
/// Anchored to `MOVEMENT_QUERY_FOOT_AVATAR_FIRST +
/// TRANSPORT_FACING_MASK` so the on-foot query range derives from
/// the transport facing-mask convention.
pub const MOVEMENT_QUERY_FOOT_AVATAR_LAST: u8 =
    MOVEMENT_QUERY_FOOT_AVATAR_FIRST + TRANSPORT_FACING_MASK;

/// `movement.md §4` single-id `0x40` query class. The chair-tile
/// force-reject does not apply to this family either.
pub const MOVEMENT_QUERY_SINGLE_TILE_0X40: u8 = 0x40;

/// `movement.md §4`: returns `true` when the static-terrain
/// dispatcher's force-reject for the chair tile range applies for
/// this query class. The on-foot family and the `0x40` query are
/// exempt; everything else respects the reject.
pub const fn movement_chair_force_reject_applies(query_class: u8, tile: u8) -> bool {
    if tile < MOVEMENT_CHAIR_FORCE_REJECT_FIRST || tile > MOVEMENT_CHAIR_FORCE_REJECT_LAST {
        return false;
    }
    if (query_class >= MOVEMENT_QUERY_FOOT_AVATAR_FIRST
        && query_class <= MOVEMENT_QUERY_FOOT_AVATAR_LAST)
        || query_class == MOVEMENT_QUERY_SINGLE_TILE_0X40
    {
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

/// `movement.md §4` outdoor active-object immobile family
/// `0xE8..=0xEB`. The static-terrain predicate for this query family
/// rejects every tile id, so a slot in this class never accepts an
/// outdoor step. The spec leaves the family's promoted name open;
/// callers should treat it as "never-pass" rather than as an art
/// label.
pub const fn outdoor_active_object_class_immobile(class_byte: u8) -> bool {
    matches!(class_byte, 0xE8..=0xEB)
}

/// `vehicles.md §2` transport/action marker family. Classifies the
/// party transport state byte into one of the documented ranges. The
/// low two bits within each family encode N/E/S/W facing using the
/// `0` north, `1` east, `2` south, `3` west convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportFamily {
    /// `0x12..=0x13` — mounted horse. Two frames only, east/west.
    MountedHorse,
    /// `0x14..=0x15` — magic carpet. Two frames only, east/west.
    MagicCarpet,
    /// `0x1C..=0x1D` — foot/avatar. Only `0x1C` is ever written;
    /// `0x1D` is accepted as defensive breadth.
    Foot,
    /// `0x20..=0x23` — ship under sail (hoisted, wind-controlled).
    ShipHoisted,
    /// `0x24..=0x27` — ship furled / manually handled.
    ShipFurled,
    /// `0x28..=0x2B` — skiff.
    Skiff,
}

/// `vehicles.md §2` transport/action marker ranges. The persistent
/// value set is **closed**: only the ship-hoisted, ship-furled and
/// skiff families carry full four-way facing in their low two bits.
/// The horse and the magic carpet have **two frames only** and the
/// on-foot family persists a single value, so their bands are two
/// markers wide, not four.
///
/// The ship-hoisted/ship-furled/skiff bands tile contiguously from
/// 0x20 upward; anchor their *_LAST to FIRST + TRANSPORT_FACING_MASK
/// and chain SHIP_FURLED/SKIFF *_FIRST to the previous family's
/// *_LAST + 1.
pub const TRANSPORT_MARKER_MAGIC_CARPET_FIRST: u8 = 0x14;
/// `vehicles.md §2`: the carpet has two frames only - `0x14` east and
/// `0x15` west. `0x16` and `0x17` are values the original engine
/// cannot produce, so nothing here may write them.
pub const TRANSPORT_MARKER_MAGIC_CARPET_LAST: u8 =
    TRANSPORT_MARKER_MAGIC_CARPET_FIRST + TRANSPORT_TWO_FRAME_WEST_BIAS;
pub const TRANSPORT_MARKER_SHIP_HOISTED_FIRST: u8 = 0x20;
pub const TRANSPORT_MARKER_SHIP_HOISTED_LAST: u8 =
    TRANSPORT_MARKER_SHIP_HOISTED_FIRST + TRANSPORT_FACING_MASK;
pub const TRANSPORT_MARKER_SHIP_FURLED_FIRST: u8 = TRANSPORT_MARKER_SHIP_HOISTED_LAST + 1;
pub const TRANSPORT_MARKER_SHIP_FURLED_LAST: u8 =
    TRANSPORT_MARKER_SHIP_FURLED_FIRST + TRANSPORT_FACING_MASK;
pub const TRANSPORT_MARKER_SKIFF_FIRST: u8 = TRANSPORT_MARKER_SHIP_FURLED_LAST + 1;
pub const TRANSPORT_MARKER_SKIFF_LAST: u8 = TRANSPORT_MARKER_SKIFF_FIRST + TRANSPORT_FACING_MASK;

/// `vehicles.md §2` low-bit mask the transport-marker facing decoder
/// applies. Bit 0 selects east/west; bit 1 selects south/north; the
/// pair yields the published `0` north / `1` east / `2` south /
/// `3` west convention. Only the ship and skiff families use the
/// full two-bit form.
pub const TRANSPORT_FACING_MASK: u8 = 0b0000_0011;

/// `vehicles.md §2` two-frame families (horse `0x12`/`0x13`, carpet
/// `0x14`/`0x15`, foot `0x1C`/`0x1D`): the family's first marker is
/// the east frame and `FIRST + 1` is the west frame. Moving north or
/// south leaves the frame unchanged.
pub const TRANSPORT_TWO_FRAME_WEST_BIAS: u8 = 1;

/// `vehicles.md §2`: resolve a two-frame family's marker for an
/// announced move. East selects `first`, west selects `first + 1`,
/// and north/south leave `previous` exactly as it was.
pub const fn transport_two_frame_marker(first: u8, previous: u8, facing: Direction) -> u8 {
    match facing {
        Direction::East => first,
        Direction::West => first + TRANSPORT_TWO_FRAME_WEST_BIAS,
        _ => previous,
    }
}

/// `vehicles.md §2`: classify a transport/action marker byte into
/// its family. Returns `None` for marker values outside the known
/// transport ranges (those remain opaque transport state, per spec).
pub const fn transport_family(marker: u8) -> Option<TransportFamily> {
    Some(match marker {
        HORSE_TRANSPORT_FIRST..=HORSE_TRANSPORT_LAST => TransportFamily::MountedHorse,
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST..=TRANSPORT_MARKER_MAGIC_CARPET_LAST => {
            TransportFamily::MagicCarpet
        }
        TRANSPORT_MARKER_FOOT_FIRST..=TRANSPORT_MARKER_FOOT_LAST => TransportFamily::Foot,
        TRANSPORT_MARKER_SHIP_HOISTED_FIRST..=TRANSPORT_MARKER_SHIP_HOISTED_LAST => {
            TransportFamily::ShipHoisted
        }
        TRANSPORT_MARKER_SHIP_FURLED_FIRST..=TRANSPORT_MARKER_SHIP_FURLED_LAST => {
            TransportFamily::ShipFurled
        }
        TRANSPORT_MARKER_SKIFF_FIRST..=TRANSPORT_MARKER_SKIFF_LAST => TransportFamily::Skiff,
        _ => return None,
    })
}

/// `vehicles.md §2`: low two bits decode transport facing as
/// north (0) / east (1) / south (2) / west (3). Returns `None` for
/// markers outside the recognised transport families.
pub const fn transport_facing_index(marker: u8) -> Option<u8> {
    if transport_family(marker).is_some() {
        Some(marker & TRANSPORT_FACING_MASK)
    } else {
        None
    }
}

/// `vehicles.md §5`: X-Xit refuses while the ship is in the
/// wind-control sail range (`0x20..=0x23`). The player must furl
/// sails through Y-Yell before X-Xit is accepted. Returns `true`
/// only for the four hoisted-sail markers; the furled-ship markers
/// `0x24..=0x27`, horse, carpet, skiff, and foot all return `false`.
pub const fn ship_xit_refused_under_sail(marker: u8) -> bool {
    matches!(transport_family(marker), Some(TransportFamily::ShipHoisted))
}

/// `vehicles.md §6` Y-Yell ship-sail toggle. Maps a hoisted-sail
/// marker `0x20..=0x23` to the matching furled marker `0x24..=0x27`
/// (and vice-versa) by flipping bit `0x04`. The low two bits encode
/// heading and stay intact across the toggle. Returns `None` for
/// markers outside the ship range so callers can fall through to
/// the no-effect case without an extra range test.
pub const fn ship_sail_toggle_marker(current_marker: u8) -> Option<u8> {
    match transport_family(current_marker) {
        Some(TransportFamily::ShipHoisted) | Some(TransportFamily::ShipFurled) => {
            Some(current_marker ^ 0x04)
        }
        _ => None,
    }
}

// `vehicles.md §4` ship-boarding warnings and the shipwright-
// purchased Frigate's starting hull/skiff state live in
// `transport.rs`, next to the boarding precondition they belong to:
// [`crate::SHIP_BOARDING_HULL_WARNING_THRESHOLD`] /
// [`crate::ship_boarding_warnings`] and [`crate::FRIGATE_PURCHASE_HULL`]
// / [`crate::FRIGATE_PURCHASE_SKIFFS`]. This module previously carried
// a second copy of both under word-swapped names; the duplicate hull
// constant also read `100` where `vehicles.md` publishes `99`.

/// `vehicles.md §2`: typed [`Direction`] for the transport marker's
/// facing. Decodes the low two bits via [`transport_facing_index`]
/// and maps the four indices to the four cardinal directions:
/// `0 -> North`, `1 -> East`, `2 -> South`, `3 -> West`. Returns
/// `None` for markers outside the recognised transport families.
pub const fn transport_marker_facing(marker: u8) -> Option<Direction> {
    let Some(index) = transport_facing_index(marker) else {
        return None;
    };
    Some(match index {
        0 => Direction::North,
        1 => Direction::East,
        2 => Direction::South,
        _ => Direction::West,
    })
}

/// `vehicles.md §2`: rewrite the marker's facing bits while preserving
/// the known transport family, following the published per-family
/// facing contract rather than one shared four-way compose.
///
/// - Mounted horse and magic carpet have **two frames only**: `FIRST`
///   when the last announced move was east, `FIRST + 1` when it was
///   west, and the previous frame unchanged for north or south.
/// - Foot persists exactly `0x1C`; nothing ever writes `0x1D`.
/// - Ship (hoisted and furled) and skiff carry full four-way facing in
///   the low two bits.
///
/// An earlier clean-engine revision composed a four-way index into
/// every family and inverted the horse's east/west bit, which could
/// persist `0x16`, `0x17`, `0x1E` and `0x1F` - values §11 says the
/// original engine cannot produce.
pub const fn transport_marker_with_facing(marker: u8, facing: Direction) -> Option<u8> {
    let Some(index) = facing.cardinal_facing_index() else {
        return None;
    };
    let Some(family) = transport_family(marker) else {
        return None;
    };
    Some(match family {
        TransportFamily::MountedHorse => {
            transport_two_frame_marker(HORSE_TRANSPORT_FIRST, marker, facing)
        }
        TransportFamily::MagicCarpet => {
            transport_two_frame_marker(TRANSPORT_MARKER_MAGIC_CARPET_FIRST, marker, facing)
        }
        TransportFamily::Foot => TRANSPORT_MARKER_FOOT_FIRST,
        TransportFamily::ShipHoisted => TRANSPORT_MARKER_SHIP_HOISTED_FIRST + index,
        TransportFamily::ShipFurled => TRANSPORT_MARKER_SHIP_FURLED_FIRST + index,
        TransportFamily::Skiff => TRANSPORT_MARKER_SKIFF_FIRST + index,
    })
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
        Some(byte & TRANSPORT_FACING_MASK)
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

/// `encounters.md §6` / `monster-bestiary.md`: sprite family used for
/// the selected sleep-ambush monster row.
pub const fn sleep_ambush_monster_sprite(monster: SleepAmbushMonster) -> u8 {
    match monster {
        SleepAmbushMonster::GiantRat => 0x90,
        SleepAmbushMonster::Troll => 0xE4,
        SleepAmbushMonster::Bat => 0x94,
        SleepAmbushMonster::Slime => 0xA0,
        SleepAmbushMonster::GiantSpider => 0x98,
        SleepAmbushMonster::Gremlin => 0xA4,
        SleepAmbushMonster::Headless => 0xD0,
    }
}

/// `encounters.md §6`: PRNG outcome that flips the rest loop into the
/// sleep-ambush branch. The shared integer PRNG produces sixty-four
/// outcomes per eligible predicate invocation; only the zero outcome
/// interrupts. Caller passes the raw draw over the closed interval
/// `[0, 63]` — sixty-four outcomes, not `[0, 64]`.
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
/// `D*2..=D*2+1`, with low bit = room id 0. Per-dungeon byte
/// count = sixteen room bits packed at eight bits per byte = 2.
/// Anchored to ceil(SAVE_DUNGEON_ROOM_CLEAR_ROOMS_PER_DUNGEON / 8)
/// so resizing the per-dungeon room count automatically widens
/// the per-dungeon byte stride.
pub const SAVE_DUNGEON_ROOM_CLEAR_BYTES_PER_DUNGEON: usize =
    SAVE_DUNGEON_ROOM_CLEAR_ROOMS_PER_DUNGEON.div_ceil(8);
/// `formats/saved-gam.md §10` rooms-per-dungeon ("sixteen room
/// ids `0..15`") matches the dungeon-format room-arena slot
/// count. Anchored to [`crate::DUNGEON_ROOM_SLOTS_PER_BANK`] so
/// the save bitmap layout and the dungeon record layout stay one
/// value.
pub const SAVE_DUNGEON_ROOM_CLEAR_ROOMS_PER_DUNGEON: usize = crate::DUNGEON_ROOM_SLOTS_PER_BANK;

/// `formats/saved-gam.md §10`: returns the (byte_offset_within_bitmap,
/// bit_mask) pair for a (dungeon, room_id) coordinate. Returns
/// `None` for out-of-range coordinates (dungeon `>= 8` or room id
/// `>= 16`).
pub const fn dungeon_room_clear_bit_position(dungeon: u8, room_id: u8) -> Option<(usize, u8)> {
    if dungeon >= 8 || room_id >= 16 {
        return None;
    }
    let byte =
        dungeon as usize * SAVE_DUNGEON_ROOM_CLEAR_BYTES_PER_DUNGEON + (room_id as usize) / 8;
    let mask = 1u8 << (room_id % 8);
    Some((byte, mask))
}

/// `encounters.md §4` candidate-terrain branch the encounter
/// spawner takes once a coordinate has passed the separation gate.
/// Caller supplies the world tile id and the underworld plane flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnTerrainBranch {
    /// Surface tile `0x01` after the low-tile allowance — 1-in-8
    /// special whirlpool roll, otherwise surface default/aquatic
    /// bucket.
    SurfaceTile1WhirlpoolOrAquatic,
    /// Terrain tile `0x07` (parched desert) — 1-in-4 Sand Trap special
    /// roll; failure rejects the candidate.
    ///
    /// *Was* `SeaSerpentAdjacency`, on the strength of an earlier
    /// `encounters.md §4` revision that read "one-in-three chance of the
    /// outdoor sea-serpent adjacency class". **Both halves of that are
    /// withdrawn**: the current text gives "**One-in-four** chance of the
    /// **Sand Trap** sprite run `0xE0..0xE3`", and
    /// `active-objects.md §8` adds that calling `0xE0..0xE3` a
    /// sea-serpent family "is withdrawn and was backwards" — the Sea
    /// Serpent run is `0x88..0x8B`, which reaches the overworld only
    /// through the water buckets.
    SandTrapParchedDesert,
    /// Terrain tile `0x04` on the full underworld plane — direct Rot
    /// Worm sprite-run selection.
    UnderworldTile4RotWorm,
    /// Surface town-outline tile `0x0C`/`0x0D` — reject.
    HardReject,
    /// Water / river / waterfall / open-water tile that needs the
    /// sixteen-in-sixty-five allowance die before bucket selection.
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
        return SpawnTerrainBranch::SandTrapParchedDesert;
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

/// `encounters.md §4` whirlpool-special chance gate on surface tile 1:
/// "**One-in-eight** chance of a special animated active-object class
/// whose outdoor engagement is the whirlpool/forced-underworld
/// branch". An earlier revision said one-in-seven; that is withdrawn,
/// "the shared range draw is inclusive on both bounds, so a draw over
/// the closed interval `[0, 7]` accepted on one value is one in
/// eight." Passed to `random_mod_u8`, which draws `[0, N - 1]`, so the
/// denominator is the interval's *size*, not its width.
pub const SPAWN_WHIRLPOOL_DENOMINATOR: u8 = 8;

/// `encounters.md §4` Sand Trap chance gate on terrain tile 7
/// (parched desert); failure rejects the candidate. "The draw is over
/// the closed interval `[0, 3]` accepted on one value, which is one in
/// four" — an earlier revision's one-in-three is withdrawn along with
/// the sea-serpent naming.
pub const SPAWN_SAND_TRAP_DENOMINATOR: u8 = 4;

/// `encounters.md §4` low-tile allowance die on the water / river /
/// waterfall / open-water bands; failure rejects the candidate.
///
/// This is deliberately **not** a `_DENOMINATOR`, because the
/// published rule is not one-in-N: "The die is a draw over the closed
/// interval `[0, 64]`, inclusive, accepted when the result is below
/// sixteen — **sixteen outcomes in sixty-five**." An earlier revision
/// called it "one-in-four"; that is withdrawn as an approximation.
/// `16/65 ~= 0.246` sits close enough to `1/4` that the wrong shape
/// read as correct — a near miss is harder to spot than a wild one.
/// See [`spawn_low_tile_allowance_accepts`].
pub const SPAWN_LOW_TILE_ALLOWANCE_DRAW_HIGH: u8 = 64;
/// `encounters.md §4` low-tile allowance acceptance bound: the draw is
/// accepted when it is strictly below this value.
pub const SPAWN_LOW_TILE_ALLOWANCE_ACCEPT_BELOW: u8 = 16;

/// `encounters.md §4`: returns `true` when a low-tile allowance draw
/// over the closed interval
/// `[0, SPAWN_LOW_TILE_ALLOWANCE_DRAW_HIGH]` allows the candidate
/// through to bucket selection. Sixteen of the sixty-five outcomes
/// accept; the other forty-nine reject the candidate.
pub const fn spawn_low_tile_allowance_accepts(roll: u8) -> bool {
    roll < SPAWN_LOW_TILE_ALLOWANCE_ACCEPT_BELOW
}

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
pub const ENCOUNTER_SPAWNER_MIN_SEPARATION: u8 = ENCOUNTER_SPAWN_MIN_SEPARATION as u8;
pub const ENCOUNTER_SPAWNER_MAX_SEPARATION: u8 = ENCOUNTER_SPAWN_MAX_SEPARATION as u8;

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
/// with thresholds 0 and 1 both producing no encounter. Anchored
/// to [`RANDOM_ENCOUNTER_DIE`] so the spawner-side die and the
/// helper-side roll bound stay one value.
pub const RANDOM_ENCOUNTER_ROLL_BOUND: u8 = RANDOM_ENCOUNTER_DIE;

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
    if threshold == 0 { 0 } else { threshold - 1 }
}

/// `vehicles.md §2` clean-seed foot/avatar transport marker. The
/// shipped `INIT.GAM` party transport state starts at this value;
/// the low two bits encode facing (0 = north).
pub const TRANSPORT_MARKER_FOOT_DEFAULT: u8 = 0x1C;

/// `vehicles.md §2` foot/avatar transport-family byte range. Any
/// byte in this band identifies the party as on foot.
///
/// Only `0x1C` is ever written: it is the clean seed and the single
/// persistent on-foot value. The adjacent `0x1D` is the second frame
/// of the on-foot sprite pair and is accepted by the two "party is on
/// foot" predicates as defensive breadth, but nothing produces it, and
/// `0x1E`/`0x1F` are outside the published set entirely.
pub const TRANSPORT_MARKER_FOOT_FIRST: u8 = 0x1C;
pub const TRANSPORT_MARKER_FOOT_LAST: u8 =
    TRANSPORT_MARKER_FOOT_FIRST + TRANSPORT_TWO_FRAME_WEST_BIAS;

/// `encounters.md §4` encounter-spawn coordinate-separation bounds.
/// A candidate spawn coordinate is accepted only when both axes'
/// absolute separation from the party is strictly greater than
/// [`ENCOUNTER_SPAWN_MIN_SEPARATION`] and strictly less than
/// [`ENCOUNTER_SPAWN_MAX_SEPARATION`]. The first bound keeps the
/// spawn outside the immediate visible centre; the second bound
/// rejects wrapped-near coordinates on the 256-by-256 torus.
pub const ENCOUNTER_SPAWN_MIN_SEPARATION: u16 = 6;
/// `encounters.md §4`: the max-separation bound is the
/// world-side minus the min-separation bound — coordinates
/// closer than `MIN_SEPARATION` to the wrapped party position
/// on the 256-cell torus are also rejected. Anchored to
/// `crate::WORLD_SIDE - ENCOUNTER_SPAWN_MIN_SEPARATION` so the
/// torus-wrap bound derives from the world side and the
/// minimum separation.
pub const ENCOUNTER_SPAWN_MAX_SEPARATION: u16 =
    crate::WORLD_SIDE as u16 - ENCOUNTER_SPAWN_MIN_SEPARATION;

/// `encounters.md §4`: returns `true` when a candidate spawn
/// coordinate's `(dx, dy)` axis separations from the party fall in
/// the accepted band (strictly between
/// [`ENCOUNTER_SPAWN_MIN_SEPARATION`] and
/// [`ENCOUNTER_SPAWN_MAX_SEPARATION`] on both axes).
pub const fn encounter_spawn_separation_accepts(dx_abs: u16, dy_abs: u16) -> bool {
    dx_abs > ENCOUNTER_SPAWN_MIN_SEPARATION
        && dx_abs < ENCOUNTER_SPAWN_MAX_SEPARATION
        && dy_abs > ENCOUNTER_SPAWN_MIN_SEPARATION
        && dy_abs < ENCOUNTER_SPAWN_MAX_SEPARATION
}

/// `encounters.md §4` encounter-spawn retry budget. After this many
/// rejected candidate coordinates, the spawner returns silently
/// without writing a monster record. Anchored to
/// [`ENCOUNTER_SPAWNER_RETRY_LIMIT`] so the same retry budget
/// applies to both the spawner retry loop and the coordinate
/// retry loop.
pub const ENCOUNTER_SPAWN_RETRY_BUDGET: u16 = ENCOUNTER_SPAWNER_RETRY_LIMIT as u16;

/// `overworld.md §7` / `encounters.md §3` random-encounter probe die.
/// The mode loop draws a uniform integer in `[1, RANDOM_ENCOUNTER_DIE]`
/// and fires the spawner when [`random_encounter_threshold`] exceeds
/// the draw.
pub const RANDOM_ENCOUNTER_DIE: u8 = 30;

/// `encounters.md §3` last hour of the surface night-boost band.
/// Hours `0..=RANDOM_ENCOUNTER_NIGHT_HOUR_LAST` add the published
/// night-time boost to the surface encounter threshold; daytime
/// hours use the lower base threshold. The same hour boundary
/// the town dawn/dusk substitution uses (town-mode.md §5,§6).
/// Anchored to [`crate::TOWN_NIGHT_BAND_DAWN_HOUR`] so the
/// engine-wide "dawn hour" has one source of truth.
pub const RANDOM_ENCOUNTER_NIGHT_HOUR_LAST: u8 = crate::TOWN_NIGHT_BAND_DAWN_HOUR;

/// `encounters.md §3` underworld-plane fixed encounter threshold. The
/// underworld uses this value regardless of hour or tile.
pub const RANDOM_ENCOUNTER_UNDERWORLD_THRESHOLD: u8 = 3;

/// `encounters.md §3` surface no-encounter tile band (roads and similar
/// safe surfaces). Tiles in this range suppress daytime encounters and
/// take the smallest night-time boost.
pub const RANDOM_ENCOUNTER_SAFE_TILE_FIRST: u8 = 0x20;
pub const RANDOM_ENCOUNTER_SAFE_TILE_LAST: u8 = 0x26;

/// `encounters.md §3` daytime threshold for the safe-tile band. Zero
/// means the probe cannot fire at all during the day on these tiles.
pub const RANDOM_ENCOUNTER_SAFE_DAY_THRESHOLD: u8 = 0;
/// `encounters.md §3` night-time threshold for the safe-tile band
/// (hours `0..=RANDOM_ENCOUNTER_NIGHT_HOUR_LAST`).
pub const RANDOM_ENCOUNTER_SAFE_NIGHT_THRESHOLD: u8 = 3;

/// `encounters.md §3` surface wilderness/swamp tile (`0x04`).
pub const RANDOM_ENCOUNTER_WILDERNESS_SWAMP: u8 = 0x04;
/// `encounters.md §3` surface wilderness band first tile (`0x09`).
pub const RANDOM_ENCOUNTER_WILDERNESS_BAND_FIRST: u8 = 0x09;
/// `encounters.md §3` surface wilderness band last tile (`0x0F`).
pub const RANDOM_ENCOUNTER_WILDERNESS_BAND_LAST: u8 = 0x0F;
/// `encounters.md §3` daytime threshold for wilderness/swamp tiles.
pub const RANDOM_ENCOUNTER_WILDERNESS_DAY_THRESHOLD: u8 = 2;
/// `encounters.md §3` night-time threshold for wilderness/swamp tiles.
pub const RANDOM_ENCOUNTER_WILDERNESS_NIGHT_THRESHOLD: u8 = 5;

/// `encounters.md §3` daytime threshold for any other surface tile not
/// in the safe band or the wilderness/swamp band.
pub const RANDOM_ENCOUNTER_DEFAULT_DAY_THRESHOLD: u8 = 1;
/// `encounters.md §3` night-time threshold for any other surface tile.
pub const RANDOM_ENCOUNTER_DEFAULT_NIGHT_THRESHOLD: u8 = 4;

/// `overworld.md §7`: returns `true` when the random-encounter probe
/// fires for the given threshold and uniform `1..=RANDOM_ENCOUNTER_DIE`
/// draw. The spawner runs when `threshold` is nonzero and strictly
/// greater than the draw.
pub const fn random_encounter_probe_fires(threshold: u8, roll_1_to_30: u8) -> bool {
    threshold != 0 && threshold > roll_1_to_30
}

pub const fn random_encounter_threshold(underworld: bool, tile: u8, hour: u8) -> u8 {
    if underworld {
        return RANDOM_ENCOUNTER_UNDERWORLD_THRESHOLD;
    }
    let night = hour <= RANDOM_ENCOUNTER_NIGHT_HOUR_LAST;
    match tile {
        RANDOM_ENCOUNTER_SAFE_TILE_FIRST..=RANDOM_ENCOUNTER_SAFE_TILE_LAST => {
            if night {
                RANDOM_ENCOUNTER_SAFE_NIGHT_THRESHOLD
            } else {
                RANDOM_ENCOUNTER_SAFE_DAY_THRESHOLD
            }
        }
        RANDOM_ENCOUNTER_WILDERNESS_SWAMP
        | RANDOM_ENCOUNTER_WILDERNESS_BAND_FIRST..=RANDOM_ENCOUNTER_WILDERNESS_BAND_LAST => {
            if night {
                RANDOM_ENCOUNTER_WILDERNESS_NIGHT_THRESHOLD
            } else {
                RANDOM_ENCOUNTER_WILDERNESS_DAY_THRESHOLD
            }
        }
        _ => {
            if night {
                RANDOM_ENCOUNTER_DEFAULT_NIGHT_THRESHOLD
            } else {
                RANDOM_ENCOUNTER_DEFAULT_DAY_THRESHOLD
            }
        }
    }
}

pub fn is_probe_walkable(tile: u8) -> bool {
    if is_npc_start_marker(tile) {
        return true;
    }
    // This broad probe is used by diagnostics, smoke pathfinding, and
    // combat arena helpers that predate the promoted top-down movement
    // query families. Player world/town movement should use
    // `is_tile_walkable_for_transport`, which applies the exact
    // movement.md static tile sets for the current transport.
    !matches!(
        tile,
        0 | 1..=3
            | 0x0c
            | 0x0d
            | 24..=27
            | 39..=41
            | 43
            | 45..=47
            | 56..=63
            | 64..=71
            | 74..=79
            | 88..=95
            | 96..=103
            | 120..=127
    )
}

/// Combat-arena terrain uses its own collision query (`combat.md §2`), not
/// the broad diagnostic/world probe above. The authored cobble floor used by
/// town conflicts is `0x44` (with `0x45` the equivalent family stamp), and
/// `0x40` is another published foot-passable floor used by arena seats.
pub fn is_combat_arena_tile_walkable(tile: u8) -> bool {
    is_probe_walkable(tile) || matches!(tile, 0x40 | 0x44 | 0x45)
}

pub fn is_tile_walkable(tile: u8, passability: Option<&TilePassability>) -> bool {
    is_tile_walkable_for_transport(tile, passability, TransportState::Foot)
}

pub fn is_base_tile_passable(tile: u8, passability: Option<&TilePassability>) -> bool {
    if is_npc_start_marker(tile) {
        return true;
    }
    passability
        .map(|passability| passability.is_passable(tile))
        .unwrap_or_else(|| foot_terrain_accepts(tile))
}

pub fn is_tile_walkable_for_transport(
    tile: u8,
    _passability: Option<&TilePassability>,
    transport: TransportState,
) -> bool {
    match transport {
        TransportState::Foot => foot_terrain_accepts(tile),
        TransportState::Horse { .. } => horse_terrain_accepts(tile),
        TransportState::Ship { .. } => ship_terrain_accepts(tile),
        TransportState::Skiff { type_byte, .. } => {
            skiff_terrain_accepts(tile, type_byte & TRANSPORT_FACING_MASK)
        }
        TransportState::Carpet { .. } => carpet_terrain_accepts(tile),
        // `vehicles.md §2`: "**There is no balloon and no sixth vehicle
        // family.**" The removed balloon arm accepted every tile; §11
        // ("Do not invent boarding, landing, or wind-driven balloon
        // movement") forbids re-homing that fly-over-anything acceptance
        // on any surviving family, so it is simply gone.
        // `vehicles.md §2` gives every other marker family an explicit
        // "[o]rdinary terrain queries use the ... predicate family" line
        // and gives marker `0x00` none: the sprite-suppressed party is
        // reached only by drowning, and `vehicles.md §6` records that
        // "[w]hat runs after the loop exits was not traced". Accepting no
        // terrain withholds a movement capability rather than inventing
        // one.
        TransportState::SpriteSuppressed => false,
    }
}

pub fn tile_class_dispatcher_accepts(tile: u8, query: u8) -> bool {
    match query {
        0x10..=0x13 => horse_terrain_accepts(tile),
        0x14..=0x17 => carpet_terrain_accepts(tile),
        0x1c..=0x1f => foot_terrain_accepts(tile),
        0x20..=0x27 => ship_terrain_accepts(tile),
        0x28..=0x2b => skiff_terrain_accepts(tile, query & TRANSPORT_FACING_MASK),
        _ => is_tile_walkable(tile, None),
    }
}

/// True if the tile is open-ocean water that blocks foot movement and
/// requires a ship or skiff. Swamp (0x04) is NOT water for movement
/// purposes -- swamp is walkable terrain that poisons the party.
/// This matches LOOK2.DAT (water 0x01-0x03 vs swamp 0x04).
pub fn is_water_tile(tile: u8) -> bool {
    (1..=3).contains(&tile)
}

/// Which `animation.md §6` family owns `tile`, or `None` when the tile is
/// not animated by the world-tick tile animator.
///
/// The family list is [`STATIC_TILE_ANIMATION_FAMILIES`]. It includes the
/// published terrain-domain fire/light source runs `0xB0..0xB3` and
/// `0xBC..0xBF` as well as the decorative selector families.
///
/// Dungeon-mode and combat-mode effect tiles (fire field, poison field,
/// sleep / energy field) are owned by per-effect handlers, not by this
/// pass — see `catalogs/tile-catalog.md §4`.
pub fn static_tile_animation_family(tile: u8) -> Option<StaticTileAnimationFamily> {
    STATIC_TILE_ANIMATION_FAMILIES
        .iter()
        .find(|spec| spec.family.contains(tile))
        .map(|spec| spec.family)
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
    matches!(tile, 24..=79) || town_command_door_tile(tile)
}

/// `conversation.md §2` step 3: the shipped talk-through white-list.
/// When the facing tile holds no NPC, Talk advances `(dx, dy)` once
/// more only if the tile is one of these counter-height and
/// waist-height furniture ids; every other tile id is opaque to Talk.
///
/// This is a discrete id set, not a contiguous band. An earlier
/// clean-engine revision modelled it as `0x40..=0x47`, which shares no
/// member with the published set: it made real shop counters opaque
/// and let Talk reach through the wall/closed-door band instead.
///
/// Note that the mirror tile `0x9D` sits deliberately *outside* the
/// set, so Talk never reaches past a mirror - it can only resolve
/// *onto* one, where the §2 step-4 status gate prints "No response!".
pub const TALK_THROUGH_TILES: [u8; 17] = [
    0x29, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0xA5, 0xAE, 0xBA, 0xBB, 0xBE, 0xCA,
    0xCB,
];
/// `conversation.md §2`: the mirror tile, immediately past the
/// `0x94..0x9C` run and deliberately excluded from the white-list.
pub const TALK_MIRROR_TILE: u8 = 0x9D;

pub const fn is_talk_through_tile(tile: u8) -> bool {
    let mut index = 0;
    while index < TALK_THROUGH_TILES.len() {
        if TALK_THROUGH_TILES[index] == tile {
            return true;
        }
        index += 1;
    }
    false
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

/// NOT a visibility predicate. This is the *physical* line-blocker used
/// by the town F-Fire cannon scan (`vehicles.md §8`: the handler "scans a
/// short fixed line for the first blocking target"). `visibility.md §6` is
/// explicit that the sight rule "is its own classifier" and "is not
/// derived from movement passability", so the centre-out sight carve must
/// use [`tile_blocks_sight_propagation`] and
/// [`tile_propagates_sight_only_when_adjacent`] instead. The band this
/// covers broad wall and high tile bands and therefore swallows ordinary interior
/// floor tiles such as the brick floor `0x44`, so wiring it into the carve
/// collapses every indoor scene to the player's own 3x3 neighbourhood.
/// Kept under a projectile name so it cannot drift back into the
/// visibility path.
pub fn surface_tile_blocks_projectile(tile: u8) -> bool {
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

/// `vehicles.md §8` town F-Fire moral-standing penalty after a
/// successful active-object hit. The local cannon path subtracts
/// five units from the shared moral-standing selector, floored at
/// zero. The penalty does not apply to door-destroyed hits or to
/// projectiles that scan empty cells.
pub const TOWN_FIRE_ACTIVE_OBJECT_HIT_KARMA_DEBIT: u8 = 5;

/// `vehicles.md §8`: apply the published town F-Fire karma debit
/// to a standing byte. Returns the post-debit standing with the
/// published five-unit subtraction floored at zero.
pub const fn town_fire_active_object_hit_standing(standing: u8) -> u8 {
    standing.saturating_sub(TOWN_FIRE_ACTIVE_OBJECT_HIT_KARMA_DEBIT)
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
        .or_else(|| intrinsic_world_damage_tile_entry(plane, x, y, tile))
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

/// `tile-catalog.md` §6: town bed head/foot tiles. The H-Hole-up
/// command accepts these only in scenes that the public shop table
/// identifies as inns; clean sidecar rows may still authorize
/// additional test/custom beds.
pub const TOWN_REST_BED_TILE_FIRST: u8 = 0x48;
pub const TOWN_REST_BED_TILE_LAST: u8 = 0x49;

pub const fn is_town_rest_bed_tile(tile: u8) -> bool {
    tile >= TOWN_REST_BED_TILE_FIRST && tile <= TOWN_REST_BED_TILE_LAST
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

pub const fn town_poison_gas_live_tile_matches(tile: u8, transport_marker: u8) -> bool {
    tile == TOWN_POISON_GAS_LIVE_TILE && transport_marker == TOWN_POISON_GAS_VEHICLE_BYTE
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
