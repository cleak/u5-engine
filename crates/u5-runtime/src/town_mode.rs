//! Town-family scene classification per `town-mode.md` §2.

use crate::npc_runtime::{NPC_FLOOR_LINK_TILE_C8, NPC_FLOOR_LINK_TILE_C9};

/// `town-mode.md §2` four classes the eight-per-class scene-byte band
/// `1..=32` divides into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownLocationClass {
    Town,
    Dwelling,
    Castle,
    Keep,
}

impl TownLocationClass {
    /// Per-class file-family name used by the four-entry pointer table.
    /// (Returned in canonical lower-case singular form; the loader
    /// composes the actual filename with the per-class extension.)
    pub const fn family_name(self) -> &'static str {
        match self {
            TownLocationClass::Town => "town",
            TownLocationClass::Dwelling => "dwelling",
            TownLocationClass::Castle => "castle",
            TownLocationClass::Keep => "keep",
        }
    }
}

/// `town-mode.md §2`: classify a scene byte (`1..=32`) into its
/// town-family class. Returns `None` for the overworld scene `0` and
/// for any value outside the town-family range.
pub const fn town_location_class(scene_byte: u8) -> Option<TownLocationClass> {
    Some(match scene_byte {
        1..=8 => TownLocationClass::Town,
        9..=16 => TownLocationClass::Dwelling,
        17..=24 => TownLocationClass::Castle,
        25..=32 => TownLocationClass::Keep,
        _ => return None,
    })
}

/// `town-mode.md §4`: zero-based per-class location index used to index
/// the roster and dialogue files. Returns `None` for non-town-family
/// scene bytes.
pub const fn town_per_class_index(scene_byte: u8) -> Option<u8> {
    match scene_byte {
        1..=8 => Some(scene_byte - 1),
        9..=16 => Some(scene_byte - 9),
        17..=24 => Some(scene_byte - 17),
        25..=32 => Some(scene_byte - 25),
        _ => None,
    }
}

/// `catalogs/gazetteer.md §5` resident town/dwelling/castle/keep
/// names indexed by scene byte `1..=32`. Returns `None` for the
/// overworld scene `0` and for the few dwelling/castle slots whose
/// resident name is blank in the public gazetteer.
pub const fn town_resident_name(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        // Towns
        1 => "Moonglow",
        2 => "Britain",
        3 => "Jhelom",
        4 => "Yew",
        5 => "Minoc",
        6 => "Trinsic",
        7 => "Skara Brae",
        8 => "New Magincia",
        // Dwellings
        9 => "Fogsbane",
        10 => "Stormcrow",
        11 => "Greyhaven",
        12 => "Waveguide",
        13 => "Iolo's Hut",
        // 14, 15, 16 — blank resident names
        // Castles
        17 => "Lord British's Castle",
        18 => "Lord Blackthorn's Castle",
        19 => "West Britanny",
        20 => "North Britanny",
        21 => "East Britanny",
        22 => "Paws",
        23 => "Cove",
        24 => "Buccaneer's Den",
        // Keeps
        25 => "Ararat",
        26 => "Bordermarch",
        27 => "Farthing",
        28 => "Windemere",
        29 => "Stonegate",
        30 => "The Lycaeum",
        31 => "Empath Abbey",
        32 => "Serpent's Hold",
        _ => return None,
    })
}

/// `town-mode.md §3`: per-location grid dimensions and floor byte size.
pub const TOWN_GRID_SIDE: usize = 32;
pub const TOWN_GRID_BYTES: usize = TOWN_GRID_SIDE * TOWN_GRID_SIDE;

/// `town-mode.md §3`: signed-eight-bit interpretation of the runtime
/// floor byte. Returns the signed offset from the scene's resident base
/// page; values `0..=127` are non-negative floors, values `128..=255`
/// are negative offsets.
pub const fn town_floor_offset(floor_byte: u8) -> i8 {
    floor_byte as i8
}

/// `formats/location-dat.md §3` per-class location-data file layout.
/// Each `*.DAT` is 16384 bytes containing eight 2048-byte per-location
/// blocks. Each block stores two consecutive 1024-byte floor pages.
pub const LOCATION_DAT_FILE_LEN: usize = 16_384;
pub const LOCATION_DAT_BLOCK_LEN: usize = 2_048;
pub const LOCATION_DAT_BLOCKS_PER_FILE: usize = 8;
pub const LOCATION_DAT_FLOOR_PAGE_LEN: usize = 1_024;
pub const LOCATION_DAT_FLOOR_PAGES_PER_BLOCK: usize = 2;

/// `formats/location-dat.md §2`: filename loaded for a town-family
/// scene byte's location data. Returns `None` for scene bytes outside
/// `1..=32`.
pub const fn location_dat_filename(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        1..=8 => "TOWNE.DAT",
        9..=16 => "DWELLING.DAT",
        17..=24 => "CASTLE.DAT",
        25..=32 => "KEEP.DAT",
        _ => return None,
    })
}

/// `formats/npc.md §2` published per-family `.NPC` roster filenames.
/// The town-family scene byte selects one of four classes; the
/// engine reads the matching roster file at location-load time.
pub const TOWNE_NPC_FILENAME: &str = "TOWNE.NPC";
pub const DWELLING_NPC_FILENAME: &str = "DWELLING.NPC";
pub const CASTLE_NPC_FILENAME: &str = "CASTLE.NPC";
pub const KEEP_NPC_FILENAME: &str = "KEEP.NPC";

/// `formats/tlk.md §2` published per-family `.TLK` dialog filenames.
/// Parallel to the `.NPC` rosters above; the conversation engine
/// loads the matching file for a town/dwelling/castle/keep scene.
pub const TOWNE_TLK_FILENAME: &str = "TOWNE.TLK";
pub const DWELLING_TLK_FILENAME: &str = "DWELLING.TLK";
pub const CASTLE_TLK_FILENAME: &str = "CASTLE.TLK";
pub const KEEP_TLK_FILENAME: &str = "KEEP.TLK";

/// `formats/npc.md §2`: filename loaded for a town-family scene
/// byte's NPC roster. Returns `None` for scene bytes outside `1..=32`.
pub const fn npc_roster_filename(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        1..=8 => TOWNE_NPC_FILENAME,
        9..=16 => DWELLING_NPC_FILENAME,
        17..=24 => CASTLE_NPC_FILENAME,
        25..=32 => KEEP_NPC_FILENAME,
        _ => return None,
    })
}

/// `formats/npc.md §2`: filename loaded for a town-family scene
/// byte's TLK dialog file. Returns `None` for scene bytes outside
/// `1..=32`.
pub const fn npc_tlk_filename(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        1..=8 => TOWNE_TLK_FILENAME,
        9..=16 => DWELLING_TLK_FILENAME,
        17..=24 => CASTLE_TLK_FILENAME,
        25..=32 => KEEP_TLK_FILENAME,
        _ => return None,
    })
}

/// `formats/npc.md §3,§4` per-class NPC file layout. The four `.NPC`
/// files are 4608 bytes each; each contains eight 576-byte sub-maps,
/// each holding 32 schedule records (16 bytes each) plus a 32-byte
/// type array and a 32-byte dialog index array.
/// `overworld.md §8` `WorldLocationTable` row layout. The first 32
/// rows map to town-family scenes (rows 0..=31 -> scenes 1..=32);
/// the next 8 rows map to the dungeon-family scenes 33..=40 in
/// shipped `DUNGEON.DAT` record order.
/// `overworld.md §8` WorldLocationTable row layout. Town-family
/// rows map to scenes 1..=SCENE_TOWN_FAMILY_LAST (32 rows);
/// dungeon-family rows map to the eight DUNGEON.DAT records.
/// Anchor each block to its scene-byte/record-count constant
/// so the table layout derives from the scene partition.
pub const WORLD_LOCATION_TABLE_TOWN_ROWS: usize = crate::SCENE_TOWN_FAMILY_LAST as usize;
pub const WORLD_LOCATION_TABLE_DUNGEON_ROWS: usize = crate::DUNGEON_DAT_RECORD_COUNT;
pub const WORLD_LOCATION_TABLE_TOTAL_ROWS: usize =
    WORLD_LOCATION_TABLE_TOWN_ROWS + WORLD_LOCATION_TABLE_DUNGEON_ROWS;

/// `overworld.md §8`: returns the scene byte that the matched
/// `WorldLocationTable` row binds to. Rows 0..=31 produce scene
/// bytes 1..=32 (town-family); rows 32..=39 produce 33..=40
/// (dungeons). Out-of-range rows return `None`.
pub const fn world_location_table_scene_for_row(row: usize) -> Option<u8> {
    if row < WORLD_LOCATION_TABLE_TOTAL_ROWS {
        Some((row as u8) + 1)
    } else {
        None
    }
}

/// `overworld.md §2` town-mover scene byte that lands the party on
/// the underworld plane after an interior exit. Ordinary town
/// exits restore the surface plane; this is the one traced
/// exception (Stonegate's interior egress, scene `0x19`).
pub const TOWN_EXIT_UNDERWORLD_SCENE: u8 = 0x19;

/// `town-mode.md §5` Yew-jail surrender destination. The arrest
/// path sends the party to scene Yew (`TOWNE:3`, scene byte 4 =
/// SCENE_YEW) at floor 0 cell `(25, 4)`. The town setup pass
/// recognises this local `Y == 4` as a special case that skips the
/// permanent-location queue lookup before allocating a phantom NPC.
pub const TOWN_ARREST_JAIL_SCENE: u8 = SCENE_YEW;
pub const TOWN_ARREST_JAIL_FLOOR: u8 = 0;
pub const TOWN_ARREST_JAIL_X: u8 = 25;
pub const TOWN_ARREST_JAIL_Y: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownArrestPrompt {
    pub scene_byte: u8,
    pub floor: i8,
    pub npc_slot: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownNpcAlarmState {
    Fortified,
    Fleeing,
    Pacified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownNpcAlarmMarker {
    pub scene_byte: u8,
    pub floor: i8,
    pub npc_slot: usize,
    pub state: TownNpcAlarmState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownNpcAttackResolution {
    DeathMask,
    AlarmOnly,
    Refused,
}

/// `town-mode.md §4, §10` and `catalogs/npc-roster.md §4`: only
/// the rare `0x0E` actor class participates in the town
/// activation/death mask. Ordinary town actors may still be live
/// NPCs, but attacking them must not create a persistent removed
/// marker for scene re-entry.
pub const fn town_npc_activation_mask_eligible(type_byte: u8) -> bool {
    type_byte == 0x0E
}

pub const fn town_npc_type_guard_like(type_byte: u8) -> bool {
    matches!(type_byte, 0x70..=0x7f)
}

pub const fn town_npc_attack_resolution(type_byte: u8) -> TownNpcAttackResolution {
    if town_npc_activation_mask_eligible(type_byte) {
        TownNpcAttackResolution::DeathMask
    } else if town_npc_type_guard_like(type_byte) {
        TownNpcAttackResolution::AlarmOnly
    } else {
        TownNpcAttackResolution::Refused
    }
}

/// `town-mode.md §5`: returns `true` when town entry hit the
/// jail-wakeup branch — local floor 0 cell with `Y == TOWN_ARREST_JAIL_Y`
/// in the Yew scene. The phantom-attach helper skips the queue
/// lookup on this path.
pub const fn town_entry_is_jail_wakeup(scene_byte: u8, floor: u8, y: u8) -> bool {
    scene_byte == TOWN_ARREST_JAIL_SCENE
        && floor == TOWN_ARREST_JAIL_FLOOR
        && y == TOWN_ARREST_JAIL_Y
}

/// `overworld.md §2`: returns `true` when an interior exit from
/// the supplied scene byte should restore the underworld plane
/// rather than the surface plane.
pub const fn town_exit_lands_underworld(scene_byte: u8) -> bool {
    scene_byte == TOWN_EXIT_UNDERWORLD_SCENE
}

/// `town-mode.md §7` town-family exit-threshold tile id. Stepping
/// onto a `0x59` cell prompts the player; accepting clears the
/// scene byte and maps the interior exit back to the location's
/// overworld coordinate.
pub const TOWN_EXIT_THRESHOLD_TILE: u8 = 0x59;

/// `town-mode.md §7` stair tile family (`0xC4..=0xC7`). The low two
/// bits encode the matching facing direction: matching the
/// movement code moves up one floor, matching that code's
/// opposite-facing value moves down one floor, and crossing the
/// stair from either side is just a normal walk.
pub const TOWN_STAIR_TILE_FIRST: u8 = 0xC4;
pub const TOWN_STAIR_TILE_LAST: u8 = 0xC7;

/// `town-mode.md §7` town stair K-Klimb intent decoded from the
/// stair tile and the avatar's current movement facing code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownStairIntent {
    /// Movement facing matches the stair's encoded facing — go up
    /// one floor.
    Up,
    /// Movement facing is the opposite of the stair's encoded
    /// facing — go down one floor.
    Down,
    /// Stair tile but neither facing match — crossing the stair as
    /// an ordinary walk; no floor change.
    Cross,
}

/// `town-mode.md §7`: classify a town stair-walk intent. Returns
/// `None` for any tile outside `TOWN_STAIR_TILE_FIRST..=_LAST`.
/// Caller passes the stair tile and the avatar's normalised
/// facing code (0..=3); the stair's encoded facing is the tile's
/// low two bits.
pub const fn town_stair_intent(tile: u8, facing: u8) -> Option<TownStairIntent> {
    if tile < TOWN_STAIR_TILE_FIRST || tile > TOWN_STAIR_TILE_LAST {
        return None;
    }
    let stair_facing = tile & 0x03;
    let opposite = (stair_facing + 2) & 0x03;
    Some(if facing == stair_facing {
        TownStairIntent::Up
    } else if facing == opposite {
        TownStairIntent::Down
    } else {
        TownStairIntent::Cross
    })
}

/// `town-mode.md §5,§6` dawn/dusk substitution night-band hours.
/// The shipped maps store gate cells in their daytime/open form;
/// the location-load pass toggles paired archway cells into their
/// closed/night form when the current hour is at or after 20 (8 PM)
/// or at or before 4 (4 AM).
pub const TOWN_NIGHT_BAND_DUSK_HOUR: u8 = 20;
pub const TOWN_NIGHT_BAND_DAWN_HOUR: u8 = 4;

/// `town-mode.md §5,§6`: returns `true` when the current hour is
/// inside the night band that drives the dawn/dusk gate
/// substitution. The band wraps midnight: hours `20..=23` and
/// `0..=4` are night; `5..=19` are day.
pub const fn town_dawn_dusk_substitution_active(hour: u8) -> bool {
    hour >= TOWN_NIGHT_BAND_DUSK_HOUR || hour <= TOWN_NIGHT_BAND_DAWN_HOUR
}

/// `town-mode.md §6` dawn/dusk gate marker tile. The town-mode loader's
/// substitution pass scans for this byte; for every match it XORs the
/// tile immediately south with [`TOWN_DAWN_DUSK_GATE_TOGGLE_XOR`]. The
/// marker cell itself is not rewritten.
pub const TOWN_DAWN_DUSK_GATE_MARKER_TILE: u8 = 0x87;

/// `town-mode.md §6` XOR mask the substitution pass applies to the
/// marker's southern paired cell. The shipped pair is `0x44`
/// (cobble) <-> `0x99` (portcullis); applying the mask twice
/// returns the cell to its original byte. The pass does not
/// validate the paired byte before XORing it.
pub const TOWN_DAWN_DUSK_GATE_TOGGLE_XOR: u8 = 0xDD;

/// `town-mode.md §6` shipped paired bytes. The substitution pass
/// converts the cobble byte (open gate) to the portcullis byte
/// (closed gate) and back; both directions are the same XOR.
pub const TOWN_DAWN_DUSK_GATE_OPEN_TILE: u8 = 0x44;
pub const TOWN_DAWN_DUSK_GATE_CLOSED_TILE: u8 = 0x99;

/// `town-mode.md §6`: returns the new byte after one application of
/// the substitution pass to the supplied paired-cell byte.
/// Idempotent on a second application.
pub const fn town_dawn_dusk_gate_toggle(paired_byte: u8) -> u8 {
    paired_byte ^ TOWN_DAWN_DUSK_GATE_TOGGLE_XOR
}

/// `town-mode.md §6`: returns `true` when the dawn/dusk substitution
/// pass should re-fire as a hour-change boundary event. Town stays
/// in the night band for hours `20..=23` and `0..=4`; when the
/// per-turn epilogue observes the new hour as either `5` (dawn out
/// of band) or `20` (dusk into band), it runs the same XOR pass
/// against the live tile buffer to toggle the shipped paired bytes.
pub const fn town_dawn_dusk_gate_pass_fires_at_hour(hour: u8) -> bool {
    hour == TOWN_NIGHT_BAND_DAWN_HOUR + 1 || hour == TOWN_NIGHT_BAND_DUSK_HOUR
}

/// `formats/npc.md §2,§3`: an `.NPC` file packs eight 576-byte
/// sub-maps for a total of 4,608 bytes. Each sub-map carries a
/// schedule array, type array, and dialog array. Anchor the file
/// length to NPC_SUB_MAPS_PER_FILE × NPC_SUB_MAP_LEN, and the
/// sub-map length to the sum of its three blocks, so the file
/// layout derives from the per-block constants.
pub const NPC_FILE_LEN: usize = NPC_SUB_MAPS_PER_FILE * NPC_SUB_MAP_LEN;
pub const NPC_SUB_MAP_LEN: usize =
    NPC_SCHEDULE_ARRAY_LEN + NPC_TYPE_ARRAY_LEN + NPC_DIALOG_ARRAY_LEN;
pub const NPC_SUB_MAPS_PER_FILE: usize = 8;
/// `formats/npc.md §5` schedule-record byte length. Each record
/// packs four 3-byte arrays (AI/X/Y/Z) and one 4-byte time-of-day
/// boundary array, for 4 × 3 + 4 = 16 bytes total. Anchored to
/// `4 * NPC_SCHEDULE_WAYPOINT_COUNT + NPC_SCHEDULE_TIME_BOUNDARY_COUNT`
/// so resizing the waypoint count or time boundary count
/// automatically shifts the record stride.
pub const NPC_SCHEDULE_RECORD_LEN: usize =
    4 * NPC_SCHEDULE_WAYPOINT_COUNT + NPC_SCHEDULE_TIME_BOUNDARY_COUNT;
/// `formats/npc.md §3` per-NPC-block layout. Each sub-map ships
/// 32 NPC slots; the schedule array packs 32 records of 16
/// bytes (= 512 bytes), then the type and dialog arrays each
/// hold 32 bytes immediately after. Anchor each offset/length to
/// NPC_SLOTS_PER_SUB_MAP (or the previous-block end) so adding
/// or resizing any block automatically shifts the later offsets.
pub const NPC_SCHEDULE_ARRAY_LEN: usize = NPC_SLOTS_PER_SUB_MAP * NPC_SCHEDULE_RECORD_LEN;
pub const NPC_TYPE_ARRAY_OFFSET: usize = NPC_SCHEDULE_ARRAY_LEN;
pub const NPC_TYPE_ARRAY_LEN: usize = NPC_SLOTS_PER_SUB_MAP;
pub const NPC_DIALOG_ARRAY_OFFSET: usize = NPC_TYPE_ARRAY_OFFSET + NPC_TYPE_ARRAY_LEN;
pub const NPC_DIALOG_ARRAY_LEN: usize = NPC_SLOTS_PER_SUB_MAP;
pub const NPC_SLOTS_PER_SUB_MAP: usize = 32;

/// `formats/npc.md §5` schedule-record sub-field widths. Each
/// 16-byte schedule record packs three waypoints (`AI[3]`, `X[3]`,
/// `Y[3]`, `Z[3]`) plus four hour-of-day boundaries (`time[4]`).
pub const NPC_SCHEDULE_WAYPOINT_COUNT: usize = 3;
pub const NPC_SCHEDULE_TIME_BOUNDARY_COUNT: usize = 4;

/// `formats/npc.md §5` schedule-record field offsets (in bytes from
/// the start of the 16-byte record). The four arrays are packed
/// back-to-back in the order AI, X, Y, Z, time.
pub const NPC_SCHEDULE_AI_OFFSET: usize = 0;
pub const NPC_SCHEDULE_X_OFFSET: usize = NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT;
pub const NPC_SCHEDULE_Y_OFFSET: usize = NPC_SCHEDULE_X_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT;
pub const NPC_SCHEDULE_Z_OFFSET: usize = NPC_SCHEDULE_Y_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT;
pub const NPC_SCHEDULE_TIME_OFFSET: usize = NPC_SCHEDULE_Z_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT;
/// Slot zero of every sub-map is the unused-sentinel slot the schedule
/// processor skips; effective capacity per sub-map is therefore 31.
pub const NPC_SENTINEL_SLOT: usize = 0;
pub const NPC_EFFECTIVE_SLOTS_PER_SUB_MAP: usize = NPC_SLOTS_PER_SUB_MAP - 1;

/// `formats/npc.md §3` per-file byte offset of the supplied sub-map
/// index within a `*.NPC` class file. The eight sub-maps are packed
/// back-to-back with no header at `index * NPC_SUB_MAP_LEN`.
pub const fn npc_sub_map_offset(sub_map_index: usize) -> usize {
    sub_map_index * NPC_SUB_MAP_LEN
}

/// `formats/npc.md §4` per-sub-map byte offset of the supplied
/// schedule-record slot. The schedule array sits at the start of
/// each sub-map and stores 32 records of 16 bytes each.
pub const fn npc_schedule_record_offset(sub_map_index: usize, slot: usize) -> usize {
    npc_sub_map_offset(sub_map_index) + slot * NPC_SCHEDULE_RECORD_LEN
}

/// `formats/npc.md §4` per-sub-map byte offset of the supplied
/// type-array slot. The 32-byte type array follows the schedule
/// array at sub-map offset `NPC_TYPE_ARRAY_OFFSET`.
pub const fn npc_type_byte_offset(sub_map_index: usize, slot: usize) -> usize {
    npc_sub_map_offset(sub_map_index) + NPC_TYPE_ARRAY_OFFSET + slot
}

/// `formats/npc.md §4` per-sub-map byte offset of the supplied
/// dialog-index slot. The 32-byte dialog-index array follows the
/// type array at sub-map offset `NPC_DIALOG_ARRAY_OFFSET`.
pub const fn npc_dialog_index_offset(sub_map_index: usize, slot: usize) -> usize {
    npc_sub_map_offset(sub_map_index) + NPC_DIALOG_ARRAY_OFFSET + slot
}

/// `town-mode.md §4`: NPC roster size — up to 31 active NPC slots
/// (slot zero is a sentinel) with three parallel 16/1/1-byte sub-blocks
/// per slot for a total of 576 bytes per location.
pub const TOWN_NPC_ROSTER_SLOTS: usize = 31;
pub const TOWN_NPC_BLOCK_BYTES: usize = 576;

/// `formats/npc.md §2` number of `.NPC` file classes (one per
/// town/dwelling/castle/keep partition). The four-way split mirrors
/// the same partition used by `.DAT` and `.TLK` files.
pub const NPC_FILE_CLASS_COUNT: usize = 4;

/// `formats/npc.md §4` upper bound on the world's named-location
/// NPC roster: thirty-one effective slots per sub-map, eight
/// sub-maps per file, four file classes — `31 * 8 * 4 = 992`.
pub const NPC_WORLD_ROSTER_MAX: usize =
    NPC_EFFECTIVE_SLOTS_PER_SUB_MAP * NPC_SUB_MAPS_PER_FILE * NPC_FILE_CLASS_COUNT;

/// `catalogs/npc-roster.md §1` named scene-byte constants for the
/// stock locations the engine ships with. These match the scene-byte
/// to-place-name table; runtime callers that need a specific scene
/// can refer to these instead of magic numbers.
///
/// Moonglow is the first town-family scene — the slot immediately
/// after the overworld. Anchored to
/// [`crate::SCENE_TOWN_FAMILY_FIRST`] so the first town and the
/// town-family band share one source of truth.
pub const SCENE_MOONGLOW: u8 = crate::SCENE_TOWN_FAMILY_FIRST;
pub const SCENE_BRITAIN: u8 = SCENE_MOONGLOW + 1;
pub const SCENE_JHELOM: u8 = SCENE_BRITAIN + 1;
pub const SCENE_YEW: u8 = SCENE_JHELOM + 1;
pub const SCENE_MINOC: u8 = SCENE_YEW + 1;
pub const SCENE_TRINSIC: u8 = SCENE_MINOC + 1;
pub const SCENE_SKARA_BRAE: u8 = SCENE_TRINSIC + 1;
pub const SCENE_NEW_MAGINCIA: u8 = SCENE_SKARA_BRAE + 1;

pub const SCENE_FOGSBANE: u8 = SCENE_NEW_MAGINCIA + 1;
pub const SCENE_STORMCROW: u8 = SCENE_FOGSBANE + 1;
pub const SCENE_GREYHAVEN: u8 = SCENE_STORMCROW + 1;
pub const SCENE_WAVEGUIDE: u8 = SCENE_GREYHAVEN + 1;
pub const SCENE_IOLOS_HUT: u8 = SCENE_WAVEGUIDE + 1;

pub const SCENE_LORD_BRITISHS_CASTLE: u8 = 17;
pub const SCENE_LORD_BLACKTHORNS_CASTLE: u8 = SCENE_LORD_BRITISHS_CASTLE + 1;
pub const SCENE_WEST_BRITANNY: u8 = SCENE_LORD_BLACKTHORNS_CASTLE + 1;
pub const SCENE_NORTH_BRITANNY: u8 = SCENE_WEST_BRITANNY + 1;
pub const SCENE_EAST_BRITANNY: u8 = SCENE_NORTH_BRITANNY + 1;
pub const SCENE_PAWS: u8 = SCENE_EAST_BRITANNY + 1;
pub const SCENE_COVE: u8 = SCENE_PAWS + 1;
pub const SCENE_BUCCANEERS_DEN: u8 = SCENE_COVE + 1;

pub const SCENE_ARARAT: u8 = SCENE_BUCCANEERS_DEN + 1;
pub const SCENE_BORDERMARCH: u8 = SCENE_ARARAT + 1;
pub const SCENE_FARTHING: u8 = SCENE_BORDERMARCH + 1;
pub const SCENE_WINDEMERE: u8 = SCENE_FARTHING + 1;
pub const SCENE_STONEGATE: u8 = SCENE_WINDEMERE + 1;
pub const SCENE_THE_LYCAEUM: u8 = SCENE_STONEGATE + 1;
pub const SCENE_EMPATH_ABBEY: u8 = SCENE_THE_LYCAEUM + 1;
pub const SCENE_SERPENTS_HOLD: u8 = SCENE_EMPATH_ABBEY + 1;

/// `town-mode.md §3` per-cell tile-buffer markers the location-load
/// pipeline harvests, rewrites, or consumes. These bytes appear in
/// the on-disk `.DAT` floor and are interpreted at marker-harvest
/// time before normal play begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownTileMarker {
    /// `0x48` — NPC start marker (variant A).
    NpcStartA,
    /// `0x49` — NPC start marker (variant B).
    NpcStartB,
    /// `0x2A` (`*`) — spawn marker. The first asterisk is the primary
    /// spawn (overworld entrance); the second is the secondary spawn
    /// (alternate exit or stairway-up landing).
    SpawnAsterisk,
    /// `0x2D` (`-`) — dash marker, processed by the cosmetic variation
    /// pass after the player has been placed.
    DashCosmetic,
    /// `0x2E` (`.`) — period marker, same processing as the dash.
    PeriodCosmetic,
    /// `0xC8` — NPC floor-link marker (variant A) consumed by the
    /// schedule processor's tile-id pathfinder.
    FloorLinkC8,
    /// `0xC9` — NPC floor-link marker (variant B).
    FloorLinkC9,
}

pub const TOWN_TILE_NPC_START_A: u8 = 0x48;
pub const TOWN_TILE_NPC_START_B: u8 = 0x49;
pub const TOWN_TILE_SPAWN_ASTERISK: u8 = b'*';
pub const TOWN_TILE_DASH_MARKER: u8 = b'-';
pub const TOWN_TILE_PERIOD_MARKER: u8 = b'.';

/// `town-mode.md §3`: classify a tile byte as one of the harvest
/// markers, or `None` for ordinary terrain bytes the renderer paints
/// directly.
pub const fn town_tile_marker(byte: u8) -> Option<TownTileMarker> {
    Some(match byte {
        TOWN_TILE_NPC_START_A => TownTileMarker::NpcStartA,
        TOWN_TILE_NPC_START_B => TownTileMarker::NpcStartB,
        TOWN_TILE_SPAWN_ASTERISK => TownTileMarker::SpawnAsterisk,
        TOWN_TILE_DASH_MARKER => TownTileMarker::DashCosmetic,
        TOWN_TILE_PERIOD_MARKER => TownTileMarker::PeriodCosmetic,
        NPC_FLOOR_LINK_TILE_C8 => TownTileMarker::FloorLinkC8,
        NPC_FLOOR_LINK_TILE_C9 => TownTileMarker::FloorLinkC9,
        _ => return None,
    })
}

/// `npc-schedules.md §3` schedule waypoint selection for the current
/// hour. Each NPC's 16-byte schedule record carries four hour
/// boundaries `time[0..=3]` that carve the 24-hour day into four
/// segments, each mapped to a waypoint:
///
///   [time[0], time[1]) -> waypoint 0
///   [time[1], time[2]) -> waypoint 1
///   [time[2], time[3]) -> waypoint 2
///   [time[3], time[0]) (wraps midnight) -> waypoint 1
///
/// Returns the active waypoint index (0..=2) for the supplied hour
/// (0..=23). The selection follows the spec's "most recent past
/// boundary, with 24-hour wraparound" rule, so equality with a
/// boundary picks that segment's waypoint.
/// `npc-schedules.md §6`: boundary-equality test. Returns `true`
/// when the current `hour` exactly matches any of the four schedule
/// `time` boundary bytes. The boundary trigger only fires for an
/// idle NPC on a tick where the hour equals one of these bytes;
/// hours strictly between boundaries leave the NPC idle.
pub const fn npc_schedule_hour_at_boundary(time: [u8; 4], hour: u8) -> bool {
    let h = hour % 24;
    time[0] % 24 == h || time[1] % 24 == h || time[2] % 24 == h || time[3] % 24 == h
}

pub const fn npc_schedule_waypoint_for_hour(time: [u8; 4], hour: u8) -> u8 {
    // Map (boundary -> waypoint) per the spec table.
    let waypoints: [u8; 4] = [0, 1, 2, 1];
    // Score each segment's start by how recently it occurred (mod 24).
    // Prefer the segment whose start has the smallest "hour - start"
    // remainder. The wraparound boundary (time[3]) maps to waypoint 1,
    // matching the night band that returns the NPC to the home/sleep
    // waypoint until the next morning.
    let mut best_idx: usize = 0;
    let mut best_recency: u8 = 24;
    let mut i: usize = 0;
    while i < 4 {
        let start = time[i] % 24;
        let recency = (hour + 24 - start) % 24;
        if recency < best_recency {
            best_recency = recency;
            best_idx = i;
        }
        i += 1;
    }
    waypoints[best_idx]
}
