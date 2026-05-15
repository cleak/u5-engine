//! Town-family scene classification per `town-mode.md` §2.

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

/// `formats/npc.md §2`: filename loaded for a town-family scene
/// byte's NPC roster. Returns `None` for scene bytes outside `1..=32`.
pub const fn npc_roster_filename(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        1..=8 => "TOWNE.NPC",
        9..=16 => "DWELLING.NPC",
        17..=24 => "CASTLE.NPC",
        25..=32 => "KEEP.NPC",
        _ => return None,
    })
}

/// `formats/npc.md §2`: filename loaded for a town-family scene
/// byte's TLK dialog file. Returns `None` for scene bytes outside
/// `1..=32`.
pub const fn npc_tlk_filename(scene_byte: u8) -> Option<&'static str> {
    Some(match scene_byte {
        1..=8 => "TOWNE.TLK",
        9..=16 => "DWELLING.TLK",
        17..=24 => "CASTLE.TLK",
        25..=32 => "KEEP.TLK",
        _ => return None,
    })
}

/// `formats/npc.md §3,§4` per-class NPC file layout. The four `.NPC`
/// files are 4608 bytes each; each contains eight 576-byte sub-maps,
/// each holding 32 schedule records (16 bytes each) plus a 32-byte
/// type array and a 32-byte dialog index array.
pub const NPC_FILE_LEN: usize = 4608;
pub const NPC_SUB_MAP_LEN: usize = 576;
pub const NPC_SUB_MAPS_PER_FILE: usize = 8;
pub const NPC_SCHEDULE_RECORD_LEN: usize = 16;
pub const NPC_SCHEDULE_ARRAY_LEN: usize = 512;
pub const NPC_TYPE_ARRAY_OFFSET: usize = 512;
pub const NPC_TYPE_ARRAY_LEN: usize = 32;
pub const NPC_DIALOG_ARRAY_OFFSET: usize = 544;
pub const NPC_DIALOG_ARRAY_LEN: usize = 32;
pub const NPC_SLOTS_PER_SUB_MAP: usize = 32;
/// Slot zero of every sub-map is the unused-sentinel slot the schedule
/// processor skips; effective capacity per sub-map is therefore 31.
pub const NPC_SENTINEL_SLOT: usize = 0;
pub const NPC_EFFECTIVE_SLOTS_PER_SUB_MAP: usize = NPC_SLOTS_PER_SUB_MAP - 1;

/// `town-mode.md §4`: NPC roster size — up to 31 active NPC slots
/// (slot zero is a sentinel) with three parallel 16/1/1-byte sub-blocks
/// per slot for a total of 576 bytes per location.
pub const TOWN_NPC_ROSTER_SLOTS: usize = 31;
pub const TOWN_NPC_BLOCK_BYTES: usize = 576;

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
