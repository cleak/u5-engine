//! Combat arena (`*.CBT`) record decoder.

use std::io;
use std::path::Path;

use crate::{DUNGEON_ROOM_SLOTS_PER_BANK, read_disk_file};

/// `formats/cbt.md §3`: each combat arena is an 11x11 visible
/// grid — the same dimension as the main-mode active viewport.
/// Anchored to [`crate::VIEWPORT_SIDE`] so the combat-arena side
/// and the main-mode viewport side share one source of truth.
pub const COMBAT_ARENA_SIDE: usize = crate::VIEWPORT_SIDE;
pub const COMBAT_ARENA_ROW_STRIDE: usize = 32;
/// `formats/cbt.md §3`: each arena row carries `COMBAT_ARENA_SIDE`
/// visible terrain bytes followed by 21 metadata bytes. The
/// metadata band starts immediately after the visible terrain
/// columns; anchor to [`COMBAT_ARENA_SIDE`] so the visible/metadata
/// split has one source of truth.
pub const COMBAT_ARENA_METADATA_START: usize = COMBAT_ARENA_SIDE;
pub const COMBAT_ARENA_METADATA_LEN: usize = COMBAT_ARENA_ROW_STRIDE - COMBAT_ARENA_METADATA_START;
pub const COMBAT_ARENA_RECORD_LEN: usize = COMBAT_ARENA_SIDE * COMBAT_ARENA_ROW_STRIDE;
pub const BRIT_CBT_RECORDS: usize = 16;
/// `formats/cbt.md §2`: number of dungeon-room arena banks in
/// `DUNGEON.CBT`. Seven of the eight stock dungeons have authored
/// room triggers (Despise carries none); each contributing dungeon
/// owns one 16-slot bank. Anchored to
/// [`crate::DUNGEON_DAT_RECORD_COUNT`] - 1 so the bank count
/// derives from the dungeon-record count with the published
/// Despise exception captured as the single subtraction.
pub const DUNGEON_CBT_BANK_COUNT: usize = crate::DUNGEON_DAT_RECORD_COUNT - 1;
pub const DUNGEON_CBT_RECORDS: usize = DUNGEON_CBT_BANK_COUNT * DUNGEON_ROOM_SLOTS_PER_BANK;
pub const BRIT_CBT_FILE: &str = "BRIT.CBT";
pub const DUNGEON_CBT_FILE: &str = "DUNGEON.CBT";
pub const DUNGEON_ROOM_SOURCE_ROW: usize = 5;
pub const DUNGEON_ROOM_PARTY_POSITION_COUNT: usize = 6;
pub const DUNGEON_ROOM_PARTY_COLUMN_X: usize = COMBAT_ARENA_METADATA_START;
pub const DUNGEON_ROOM_PARTY_COLUMN_Y: usize = 17;
/// `formats/cbt.md §5`: the dungeon-room source band starts in
/// the metadata column (one past the visible 11-cell terrain
/// band). Anchored to [`COMBAT_ARENA_METADATA_START`] so the
/// dungeon-room source column derives from the visible/metadata
/// split.
pub const DUNGEON_ROOM_SOURCE_COLUMN: usize = COMBAT_ARENA_METADATA_START;
pub const DUNGEON_ROOM_SOURCE_X_ROW: usize = 6;
pub const DUNGEON_ROOM_SOURCE_Y_ROW: usize = 7;
/// `formats/cbt.md §5` dungeon-room source slot count per bank.
/// Equal to [`crate::DUNGEON_ROOM_SLOTS_PER_BANK`] — each bank's
/// source records occupy the same 16 slots as the bank's room
/// table. Anchored through to that shared slot count.
pub const DUNGEON_ROOM_SOURCE_COUNT: usize = crate::DUNGEON_ROOM_SLOTS_PER_BANK;
pub const DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE: u8 = 0x3c;
pub const DUNGEON_ROOM_ABSORBABLE_FIELD_CLASS_MASK: u8 = 0xfc;
pub const DUNGEON_ROOM_ORDINARY_SOURCE_FIRST: u8 = 0x40;
pub const DUNGEON_ROOM_SPECIAL_SOURCE_MASK: u8 = 0xfc;
pub const DUNGEON_ROOM_SPECIAL_SOURCE_B4: u8 = 0xb4;
pub const DUNGEON_ROOM_SPECIAL_SOURCE_E8: u8 = 0xe8;
pub const DUNGEON_ROOM_SPECIAL_SOURCE_EC: u8 = 0xec;

/// `formats/cbt.md §5` outdoor metadata band slices. Per-arena setup
/// tables A and B sit on row 3 at columns 11..=16 and 17..=22; the
/// sixteen placement-slot X/Y coordinates sit on rows 6 and 7 at
/// columns 11..=26.
pub const CBT_SETUP_TABLE_ROW: usize = 3;
/// `formats/cbt.md §5` outdoor metadata band column slices. Table
/// A starts at the metadata-start column; tables B and the
/// placement band tile up across the metadata row. Anchor each
/// range to COMBAT_ARENA_METADATA_START so resizing the visible
/// terrain band automatically shifts the column ranges.
pub const CBT_SETUP_TABLE_A_COLUMNS: std::ops::RangeInclusive<usize> =
    COMBAT_ARENA_METADATA_START..=16;
pub const CBT_SETUP_TABLE_B_COLUMNS: std::ops::RangeInclusive<usize> = 17..=22;
pub const CBT_PLACEMENT_X_ROW: usize = 6;
pub const CBT_PLACEMENT_Y_ROW: usize = 7;
pub const CBT_PLACEMENT_COLUMNS: std::ops::RangeInclusive<usize> = COMBAT_ARENA_METADATA_START..=26;
pub const CBT_PLACEMENT_SLOT_COUNT: usize = 16;
/// `formats/cbt.md §2` expected total file lengths.
pub const BRIT_CBT_FILE_LEN: usize = COMBAT_ARENA_RECORD_LEN * BRIT_CBT_RECORDS;
pub const DUNGEON_CBT_FILE_LEN: usize = COMBAT_ARENA_RECORD_LEN * DUNGEON_CBT_RECORDS;

/// `formats/cbt.md §3` file arithmetic: byte offset of arena
/// `arena_index` inside a `.CBT` file. The arena record is
/// `COMBAT_ARENA_RECORD_LEN` (352) bytes long with no leading
/// directory; the offset is therefore a plain index-times-stride
/// multiply with no further normalisation.
pub const fn combat_arena_file_offset(arena_index: usize) -> usize {
    arena_index * COMBAT_ARENA_RECORD_LEN
}

/// `formats/cbt.md §3` file arithmetic: byte offset of row `row`
/// inside arena `arena_index`. Each row is `COMBAT_ARENA_ROW_STRIDE`
/// (32) bytes — the 11 visible terrain cells followed by 21 bytes of
/// metadata. Caller adds the column index for a per-cell offset.
pub const fn combat_arena_row_offset(arena_index: usize, row: usize) -> usize {
    combat_arena_file_offset(arena_index) + row * COMBAT_ARENA_ROW_STRIDE
}
pub const DEFAULT_COMBAT_ARENA_TERRAIN: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] =
    [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

/// `dungeon-mode.md §14.1`: dungeon wandering-monster ambush combat
/// synthesises its arena in the room buffer instead of loading
/// `DUNGEON.CBT` - "No arena record is read from disk on this path."
/// The painter fills the eleven-by-eleven terrain grid with the current
/// corridor fill byte before stamping its outline/corner/centre/per-side
/// treatments, whose byte values §14.1 does not publish.
pub const DUNGEON_AMBUSH_ARENA_FLOOR_TILE: u8 = 0x04;
pub const DUNGEON_AMBUSH_ARENA_TERRAIN: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] =
    [[DUNGEON_AMBUSH_ARENA_FLOOR_TILE; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

/// `dungeon-mode.md §14.1` party-entry rows for the synthesised ambush
/// arena. "Metadata rows one through four receive fixed six-entry X and
/// Y sequences", indexed here by `row - 1`. The entry-facing seed the
/// setup helper later reads is the party's current dungeon facing, and
/// the rows are arranged so the row that facing selects places the party
/// on the side of the arena *behind* its facing.
pub const DUNGEON_AMBUSH_PARTY_ENTRY_X: [[u8; DUNGEON_ROOM_PARTY_POSITION_COUNT]; 4] = [
    [6, 7, 7, 8, 8, 8],
    [4, 3, 3, 2, 2, 2],
    [5, 4, 6, 3, 5, 7],
    [5, 4, 6, 3, 5, 7],
];
pub const DUNGEON_AMBUSH_PARTY_ENTRY_Y: [[u8; DUNGEON_ROOM_PARTY_POSITION_COUNT]; 4] = [
    [5, 4, 6, 3, 5, 7],
    [5, 4, 6, 3, 5, 7],
    [6, 7, 7, 8, 8, 8],
    [4, 3, 3, 2, 2, 2],
];

/// `dungeon-mode.md §14.1` source-coordinate rows, "arranged so monsters
/// start on the side the party is facing". Facing south swaps the east
/// pair and facing west swaps the north pair.
pub const DUNGEON_AMBUSH_SOURCE_X_NORTH: [u8; DUNGEON_ROOM_SOURCE_COUNT] =
    [5, 4, 6, 3, 7, 2, 8, 5, 2, 8, 3, 7, 2, 4, 6, 8];
pub const DUNGEON_AMBUSH_SOURCE_Y_NORTH: [u8; DUNGEON_ROOM_SOURCE_COUNT] =
    [2, 2, 2, 3, 3, 4, 4, 1, 2, 2, 1, 1, 0, 0, 0, 0];
pub const DUNGEON_AMBUSH_SOURCE_X_EAST: [u8; DUNGEON_ROOM_SOURCE_COUNT] =
    [8, 8, 8, 7, 7, 6, 6, 9, 8, 8, 9, 9, 10, 10, 10, 10];
pub const DUNGEON_AMBUSH_SOURCE_Y_EAST: [u8; DUNGEON_ROOM_SOURCE_COUNT] =
    [5, 4, 6, 3, 7, 2, 8, 5, 2, 8, 7, 3, 2, 4, 6, 8];

/// `dungeon-mode.md §14.1`: the facing-selected sixteen-entry source
/// coordinate rows, or `None` for "a facing value outside zero through
/// three", which "leaves both rows untouched".
pub const fn dungeon_ambush_source_rows(
    facing_seed: u8,
) -> Option<(
    [u8; DUNGEON_ROOM_SOURCE_COUNT],
    [u8; DUNGEON_ROOM_SOURCE_COUNT],
)> {
    match facing_seed {
        0 => Some((DUNGEON_AMBUSH_SOURCE_X_NORTH, DUNGEON_AMBUSH_SOURCE_Y_NORTH)),
        1 => Some((DUNGEON_AMBUSH_SOURCE_X_EAST, DUNGEON_AMBUSH_SOURCE_Y_EAST)),
        2 => Some((DUNGEON_AMBUSH_SOURCE_Y_EAST, DUNGEON_AMBUSH_SOURCE_X_EAST)),
        3 => Some((DUNGEON_AMBUSH_SOURCE_Y_NORTH, DUNGEON_AMBUSH_SOURCE_X_NORTH)),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatArenaRecord {
    rows: [[u8; COMBAT_ARENA_ROW_STRIDE]; COMBAT_ARENA_SIDE],
}

impl CombatArenaRecord {
    pub fn from_record_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != COMBAT_ARENA_RECORD_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "combat arena record must be {COMBAT_ARENA_RECORD_LEN} bytes, got {}",
                    bytes.len()
                ),
            ));
        }
        let mut rows = [[0u8; COMBAT_ARENA_ROW_STRIDE]; COMBAT_ARENA_SIDE];
        for (row_index, row) in rows.iter_mut().enumerate() {
            let start = row_index * COMBAT_ARENA_ROW_STRIDE;
            row.copy_from_slice(&bytes[start..start + COMBAT_ARENA_ROW_STRIDE]);
        }
        Ok(Self { rows })
    }

    /// `dungeon-mode.md §14.1` step 3: synthesise the wandering-monster
    /// ambush arena in the room buffer. The terrain grid is filled with
    /// the corridor fill byte, metadata rows one through four receive the
    /// published party-entry sequences, rows six and seven receive the
    /// facing-selected source coordinate sequences, and the source band
    /// receives `count` copies of the ordinary source byte
    /// `class * 4 + 0x40` written "into the first `count` permuted
    /// slots". The record is then read back by the ordinary room-combat
    /// setup helper, which recovers the same class the painter encoded.
    ///
    /// The outline, corner markers and the four per-side treatments §14.1
    /// also describes are not stamped here: that section publishes no byte
    /// values for them and flags the filler byte as inferred rather than
    /// traced.
    ///
    /// `centre_icon` is the one part of the painter's underfoot-class
    /// centre icon that *is* published as a value. §14.1: "The underfoot
    /// class is the high half of the room-layout cell under the party, and
    /// it selects one byte from a small icon table that is stamped into the
    /// arena grid's centre cell", and "**The chest class stamps the byte the
    /// combat setup pass tests at the arena centre** ... That is the only
    /// way that byte ever reaches the centre cell - no shipped arena record
    /// carries it". The caller supplies `Some(0xDC)` for a chest and `None`
    /// for every other class, whose icon bytes §14.1 does not publish.
    pub fn synthesise_dungeon_ambush(
        fill_byte: u8,
        facing_seed: u8,
        class: u8,
        count: u8,
        permutation: [u8; DUNGEON_ROOM_SOURCE_COUNT],
        centre_icon: Option<u8>,
    ) -> Self {
        let mut rows = [[0u8; COMBAT_ARENA_ROW_STRIDE]; COMBAT_ARENA_SIDE];
        for row in rows.iter_mut() {
            for cell in row[..COMBAT_ARENA_SIDE].iter_mut() {
                *cell = fill_byte;
            }
        }
        for (index, row) in rows
            .iter_mut()
            .take(DUNGEON_AMBUSH_PARTY_ENTRY_X.len() + 1)
            .enumerate()
            .skip(1)
        {
            let entry = index - 1;
            for slot in 0..DUNGEON_ROOM_PARTY_POSITION_COUNT {
                row[DUNGEON_ROOM_PARTY_COLUMN_X + slot] = DUNGEON_AMBUSH_PARTY_ENTRY_X[entry][slot];
                row[DUNGEON_ROOM_PARTY_COLUMN_Y + slot] = DUNGEON_AMBUSH_PARTY_ENTRY_Y[entry][slot];
            }
        }
        if let Some((source_x, source_y)) = dungeon_ambush_source_rows(facing_seed) {
            for slot in 0..DUNGEON_ROOM_SOURCE_COUNT {
                rows[DUNGEON_ROOM_SOURCE_X_ROW][DUNGEON_ROOM_SOURCE_COLUMN + slot] = source_x[slot];
                rows[DUNGEON_ROOM_SOURCE_Y_ROW][DUNGEON_ROOM_SOURCE_COLUMN + slot] = source_y[slot];
            }
        }
        let source_byte = class
            .wrapping_mul(4)
            .wrapping_add(DUNGEON_ROOM_ORDINARY_SOURCE_FIRST);
        for placed in 0..usize::from(count).min(DUNGEON_ROOM_SOURCE_COUNT) {
            let slot = usize::from(permutation[placed]);
            if slot < DUNGEON_ROOM_SOURCE_COUNT {
                rows[DUNGEON_ROOM_SOURCE_ROW][DUNGEON_ROOM_SOURCE_COLUMN + slot] = source_byte;
            }
        }
        if let Some(icon) = centre_icon {
            let (row, column) = crate::COMBAT_ARENA_CENTRE_CELL;
            rows[row][column] = icon;
        }
        Self { rows }
    }

    pub fn row(&self, y: usize) -> Option<&[u8; COMBAT_ARENA_ROW_STRIDE]> {
        self.rows.get(y)
    }

    pub fn record_bytes(&self) -> [u8; COMBAT_ARENA_RECORD_LEN] {
        let mut bytes = [0u8; COMBAT_ARENA_RECORD_LEN];
        for (row_index, row) in self.rows.iter().enumerate() {
            let start = row_index * COMBAT_ARENA_ROW_STRIDE;
            bytes[start..start + COMBAT_ARENA_ROW_STRIDE].copy_from_slice(row);
        }
        bytes
    }

    pub fn terrain(&self, x: usize, y: usize) -> Option<u8> {
        if x >= COMBAT_ARENA_SIDE {
            return None;
        }
        self.rows.get(y).map(|row| row[x])
    }

    pub fn terrain_grid(&self) -> [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] {
        let mut terrain = DEFAULT_COMBAT_ARENA_TERRAIN;
        for (y, terrain_row) in terrain.iter_mut().enumerate() {
            terrain_row.copy_from_slice(&self.rows[y][..COMBAT_ARENA_SIDE]);
        }
        terrain
    }

    pub fn metadata(&self, row: usize, column: usize) -> Option<u8> {
        if !(COMBAT_ARENA_METADATA_START..COMBAT_ARENA_ROW_STRIDE).contains(&column) {
            return None;
        }
        self.rows.get(row).map(|arena_row| arena_row[column])
    }

    pub fn outdoor_setup_table_a(&self) -> [u8; 6] {
        self.slice_from_row::<6>(CBT_SETUP_TABLE_ROW, *CBT_SETUP_TABLE_A_COLUMNS.start())
    }

    pub fn outdoor_setup_table_b(&self) -> [u8; 6] {
        self.slice_from_row::<6>(CBT_SETUP_TABLE_ROW, *CBT_SETUP_TABLE_B_COLUMNS.start())
    }

    pub fn outdoor_placement_x(&self) -> [u8; CBT_PLACEMENT_SLOT_COUNT] {
        self.slice_from_row::<CBT_PLACEMENT_SLOT_COUNT>(
            CBT_PLACEMENT_X_ROW,
            *CBT_PLACEMENT_COLUMNS.start(),
        )
    }

    pub fn outdoor_placement_y(&self) -> [u8; CBT_PLACEMENT_SLOT_COUNT] {
        self.slice_from_row::<CBT_PLACEMENT_SLOT_COUNT>(
            CBT_PLACEMENT_Y_ROW,
            *CBT_PLACEMENT_COLUMNS.start(),
        )
    }

    pub fn dungeon_room_sources(&self) -> [u8; 16] {
        self.slice_from_row::<DUNGEON_ROOM_SOURCE_COUNT>(
            DUNGEON_ROOM_SOURCE_ROW,
            DUNGEON_ROOM_SOURCE_COLUMN,
        )
    }

    pub fn dungeon_room_source_x(&self) -> [u8; 16] {
        self.slice_from_row::<DUNGEON_ROOM_SOURCE_COUNT>(
            DUNGEON_ROOM_SOURCE_X_ROW,
            DUNGEON_ROOM_SOURCE_COLUMN,
        )
    }

    pub fn dungeon_room_source_y(&self) -> [u8; 16] {
        self.slice_from_row::<DUNGEON_ROOM_SOURCE_COUNT>(
            DUNGEON_ROOM_SOURCE_Y_ROW,
            DUNGEON_ROOM_SOURCE_COLUMN,
        )
    }

    pub fn dungeon_room_party_positions_for_seed(
        &self,
        entry_seed: u8,
    ) -> [(u8, u8); DUNGEON_ROOM_PARTY_POSITION_COUNT] {
        let row = dungeon_room_party_position_row(entry_seed);
        let x = self
            .slice_from_row::<DUNGEON_ROOM_PARTY_POSITION_COUNT>(row, DUNGEON_ROOM_PARTY_COLUMN_X);
        let y = self
            .slice_from_row::<DUNGEON_ROOM_PARTY_POSITION_COUNT>(row, DUNGEON_ROOM_PARTY_COLUMN_Y);
        let mut positions = [(0u8, 0u8); DUNGEON_ROOM_PARTY_POSITION_COUNT];
        for (slot, position) in positions.iter_mut().enumerate() {
            *position = (x[slot], y[slot]);
        }
        positions
    }

    pub fn dungeon_room_setup_sources(&self) -> Vec<DungeonRoomSetupSource> {
        self.dungeon_room_setup_sources_with_scan(true)
    }

    pub fn dungeon_room_setup_sources_with_scan(
        &self,
        scan_sources: bool,
    ) -> Vec<DungeonRoomSetupSource> {
        if !scan_sources {
            return Vec::new();
        }
        let sources = self.dungeon_room_sources();
        let x = self.dungeon_room_source_x();
        let y = self.dungeon_room_source_y();
        sources
            .into_iter()
            .enumerate()
            .filter_map(|(slot, source)| {
                DungeonRoomSetupSource::new(slot, source, x[slot], y[slot])
            })
            .collect()
    }

    fn slice_from_row<const N: usize>(&self, row: usize, start: usize) -> [u8; N] {
        let mut out = [0u8; N];
        out.copy_from_slice(&self.rows[row][start..start + N]);
        out
    }
}

pub const fn dungeon_room_party_position_row(entry_seed: u8) -> usize {
    match entry_seed {
        3 => 1,
        1 => 2,
        0 | 5 => 3,
        _ => 4,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonRoomSetupSource {
    pub slot: usize,
    pub source: u8,
    pub x: u8,
    pub y: u8,
    pub kind: DungeonRoomSetupSourceKind,
}

impl DungeonRoomSetupSource {
    pub fn new(slot: usize, source: u8, x: u8, y: u8) -> Option<Self> {
        if source == 0 || slot >= DUNGEON_ROOM_SOURCE_COUNT {
            return None;
        }
        Some(Self {
            slot,
            source,
            x,
            y,
            kind: DungeonRoomSetupSourceKind::from_source(source),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonRoomSetupSourceKind {
    /// `formats/cbt.md §5` ordinary room source. `palette_selector` is
    /// `Some(i)` only for the `0xEC..0xEF` vermin family, whose derived
    /// setup class is replaced by the `i`-th pre-rolled palette id
    /// before placement; the placement path itself is unchanged.
    OrdinaryCombatant {
        setup_class: u8,
        palette_selector: Option<u8>,
    },
    AbsorbableField,
    SpecialPlacement(DungeonRoomSpecialPlacement),
}

impl DungeonRoomSetupSourceKind {
    pub fn from_source(source: u8) -> Self {
        if dungeon_room_absorbable_field_family(source) {
            Self::AbsorbableField
        } else if let Some(setup_class) = dungeon_room_ordinary_setup_class(source) {
            // `formats/cbt.md §5` + `dungeon-mode.md §14`: sources in
            // the `0xEC..0xEF` family "pass the ordinary/special test,
            // take the ordinary path, and only have their derived setup
            // class replaced by one of four setup ids pre-rolled from a
            // small vermin palette".
            Self::OrdinaryCombatant {
                setup_class,
                palette_selector: dungeon_room_vermin_palette_selector(source),
            }
        } else {
            Self::SpecialPlacement(DungeonRoomSpecialPlacement::from_setup_id(source))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonRoomSpecialPlacement {
    pub setup_id: u8,
    pub post_write: DungeonRoomSpecialPostWrite,
}

impl DungeonRoomSpecialPlacement {
    pub const fn from_setup_id(setup_id: u8) -> Self {
        Self {
            setup_id,
            post_write: dungeon_room_special_post_write(setup_id),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonRoomSpecialPostWrite {
    LevelTimesThreePlusSeven,
    LevelScaledRandom,
    RandomRange { low: u8, high: u8 },
    RandomRangePlus { base: u8, low: u8, high: u8 },
    Constant(u8),
    DegenerateDrawThenConstant(u8),
    None,
}

pub const DUNGEON_ROOM_RANDOM_SPECIAL_SETUP_PALETTE: [u8; 8] = [20, 21, 22, 34, 33, 24, 31, 24];
pub const DUNGEON_ROOM_RANDOM_SPECIAL_ROLL_COUNT: usize = 4;

/// `formats/cbt.md §5` ordinary/special test: "if the source is at
/// least `0x40`, and its masked family is neither `0xB4` nor `0xE8`,
/// the setup class is `(source - 0x40) / 4` and the actor is placed
/// through the ordinary room-combat path. **The `0xEC..0xEF` family
/// is not excluded by this test and therefore takes this path.**"
pub const fn dungeon_room_ordinary_setup_class(source: u8) -> Option<u8> {
    let family = source & DUNGEON_ROOM_SPECIAL_SOURCE_MASK;
    if source >= DUNGEON_ROOM_ORDINARY_SOURCE_FIRST
        && family != DUNGEON_ROOM_SPECIAL_SOURCE_B4
        && family != DUNGEON_ROOM_SPECIAL_SOURCE_E8
    {
        Some((source - DUNGEON_ROOM_ORDINARY_SOURCE_FIRST) / 4)
    } else {
        None
    }
}

/// `formats/cbt.md §5` vermin-palette class override. Sources in the
/// `0xEC..0xEF` family stay on the ordinary placement path; only their
/// derived setup class is overwritten with the pre-rolled palette id
/// selected by the source's low two bits.
pub const fn dungeon_room_vermin_palette_selector(source: u8) -> Option<u8> {
    if source & DUNGEON_ROOM_SPECIAL_SOURCE_MASK == DUNGEON_ROOM_SPECIAL_SOURCE_EC {
        Some(source & 0x03)
    } else {
        None
    }
}

pub const fn dungeon_room_special_post_write(setup_id: u8) -> DungeonRoomSpecialPostWrite {
    match setup_id {
        1 => DungeonRoomSpecialPostWrite::LevelTimesThreePlusSeven,
        2 => DungeonRoomSpecialPostWrite::LevelScaledRandom,
        3 | 4 => DungeonRoomSpecialPostWrite::RandomRange { low: 0, high: 7 },
        5 => DungeonRoomSpecialPostWrite::RandomRangePlus {
            base: 30,
            low: 0,
            high: 3,
        },
        6 => DungeonRoomSpecialPostWrite::RandomRangePlus {
            base: 4,
            low: 0,
            high: 2,
        },
        7 | 8 | 13 | 15 => DungeonRoomSpecialPostWrite::RandomRange { low: 1, high: 7 },
        9 => DungeonRoomSpecialPostWrite::RandomRange { low: 0, high: 3 },
        10 => DungeonRoomSpecialPostWrite::RandomRangePlus {
            base: 42,
            low: 0,
            high: 2,
        },
        11 => DungeonRoomSpecialPostWrite::RandomRangePlus {
            base: 9,
            low: 0,
            high: 5,
        },
        12 => DungeonRoomSpecialPostWrite::RandomRangePlus {
            base: 45,
            low: 0,
            high: 2,
        },
        14 => DungeonRoomSpecialPostWrite::DegenerateDrawThenConstant(1),
        _ => DungeonRoomSpecialPostWrite::None,
    }
}

pub const fn dungeon_room_absorbable_field_family(byte: u8) -> bool {
    byte & DUNGEON_ROOM_ABSORBABLE_FIELD_CLASS_MASK == DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatArenaBank {
    pub resource_name: String,
    pub records: Vec<CombatArenaRecord>,
}

impl CombatArenaBank {
    pub fn record(&self, index: usize) -> Option<&CombatArenaRecord> {
        self.records.get(index)
    }
}

pub fn parse_combat_arena_bank(
    resource_name: &str,
    bytes: &[u8],
    expected_records: usize,
) -> io::Result<CombatArenaBank> {
    let expected_len = expected_records * COMBAT_ARENA_RECORD_LEN;
    if bytes.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} must be {expected_len} bytes ({expected_records} records), got {}",
                bytes.len()
            ),
        ));
    }

    let mut records = Vec::with_capacity(expected_records);
    for chunk in bytes.chunks_exact(COMBAT_ARENA_RECORD_LEN) {
        records.push(CombatArenaRecord::from_record_bytes(chunk)?);
    }

    Ok(CombatArenaBank {
        resource_name: resource_name.to_string(),
        records,
    })
}

pub fn load_brit_cbt(game_dir: &Path) -> io::Result<CombatArenaBank> {
    let path = game_dir.join(BRIT_CBT_FILE);
    let bytes = read_disk_file(&path)?;
    parse_combat_arena_bank(BRIT_CBT_FILE, &bytes, BRIT_CBT_RECORDS)
}

pub fn load_dungeon_cbt(game_dir: &Path) -> io::Result<CombatArenaBank> {
    let path = game_dir.join(DUNGEON_CBT_FILE);
    let bytes = read_disk_file(&path)?;
    parse_combat_arena_bank(DUNGEON_CBT_FILE, &bytes, DUNGEON_CBT_RECORDS)
}
