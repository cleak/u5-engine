//! Combat arena (`*.CBT`) record decoder.

use std::fs;
use std::io;
use std::path::Path;

pub const COMBAT_ARENA_SIDE: usize = 11;
pub const COMBAT_ARENA_ROW_STRIDE: usize = 32;
pub const COMBAT_ARENA_METADATA_START: usize = 11;
pub const COMBAT_ARENA_METADATA_LEN: usize = COMBAT_ARENA_ROW_STRIDE - COMBAT_ARENA_METADATA_START;
pub const COMBAT_ARENA_RECORD_LEN: usize = COMBAT_ARENA_SIDE * COMBAT_ARENA_ROW_STRIDE;
pub const BRIT_CBT_RECORDS: usize = 16;
pub const DUNGEON_CBT_RECORDS: usize = 112;
pub const BRIT_CBT_FILE: &str = "BRIT.CBT";
pub const DUNGEON_CBT_FILE: &str = "DUNGEON.CBT";
pub const DUNGEON_ROOM_SOURCE_ROW: usize = 5;
pub const DUNGEON_ROOM_SOURCE_COLUMN: usize = 11;
pub const DUNGEON_ROOM_SOURCE_COUNT: usize = 16;
pub const DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE: u8 = 0x3c;

/// `formats/cbt.md §5` outdoor metadata band slices. Per-arena setup
/// tables A and B sit on row 3 at columns 11..=16 and 17..=22; the
/// sixteen placement-slot X/Y coordinates sit on rows 6 and 7 at
/// columns 11..=26.
pub const CBT_SETUP_TABLE_ROW: usize = 3;
pub const CBT_SETUP_TABLE_A_COLUMNS: std::ops::RangeInclusive<usize> = 11..=16;
pub const CBT_SETUP_TABLE_B_COLUMNS: std::ops::RangeInclusive<usize> = 17..=22;
pub const CBT_PLACEMENT_X_ROW: usize = 6;
pub const CBT_PLACEMENT_Y_ROW: usize = 7;
pub const CBT_PLACEMENT_COLUMNS: std::ops::RangeInclusive<usize> = 11..=26;
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

    pub fn row(&self, y: usize) -> Option<&[u8; COMBAT_ARENA_ROW_STRIDE]> {
        self.rows.get(y)
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

    pub fn dungeon_room_setup_sources(&self) -> Vec<DungeonRoomSetupSource> {
        self.dungeon_room_sources()
            .into_iter()
            .enumerate()
            .filter_map(|(slot, source)| DungeonRoomSetupSource::new(slot, source))
            .collect()
    }

    fn slice_from_row<const N: usize>(&self, row: usize, start: usize) -> [u8; N] {
        let mut out = [0u8; N];
        out.copy_from_slice(&self.rows[row][start..start + N]);
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonRoomSetupSource {
    pub slot: usize,
    pub source: u8,
    pub kind: DungeonRoomSetupSourceKind,
}

impl DungeonRoomSetupSource {
    pub fn new(slot: usize, source: u8) -> Option<Self> {
        if source == 0 || slot >= DUNGEON_ROOM_SOURCE_COUNT {
            return None;
        }
        Some(Self {
            slot,
            source,
            kind: DungeonRoomSetupSourceKind::from_source(source),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonRoomSetupSourceKind {
    OrdinaryCombatant,
    AbsorbableField,
    SpecialPlacement,
}

impl DungeonRoomSetupSourceKind {
    pub fn from_source(source: u8) -> Self {
        if source == DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE {
            Self::AbsorbableField
        } else if (0x40..=0x7f).contains(&source) {
            Self::OrdinaryCombatant
        } else {
            Self::SpecialPlacement
        }
    }
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
    let bytes = fs::read(&path)
        .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))?;
    parse_combat_arena_bank(BRIT_CBT_FILE, &bytes, BRIT_CBT_RECORDS)
}

pub fn load_dungeon_cbt(game_dir: &Path) -> io::Result<CombatArenaBank> {
    let path = game_dir.join(DUNGEON_CBT_FILE);
    let bytes = fs::read(&path)
        .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))?;
    parse_combat_arena_bank(DUNGEON_CBT_FILE, &bytes, DUNGEON_CBT_RECORDS)
}
