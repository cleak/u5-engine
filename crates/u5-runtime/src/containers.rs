//! Helpers for container/Get behaviour per `containers.md`. Currently
//! covers the dungeon-chest reward generator (§6) and the directional
//! table-food consumption rule (§7).

/// `containers.md §6`: inventory family one of the seven dungeon-chest
/// reward rows can grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonChestReward {
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
    Potion,
    Scroll,
}

/// `containers.md §6`: a single dungeon-chest reward row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonChestRow {
    pub gate_threshold: u8,
    pub reward: DungeonChestReward,
}

/// `containers.md §6`: the seven reward rows in iteration order.
pub const DUNGEON_CHEST_ROWS: [DungeonChestRow; 7] = [
    DungeonChestRow {
        gate_threshold: 2,
        reward: DungeonChestReward::Food,
    },
    DungeonChestRow {
        gate_threshold: 4,
        reward: DungeonChestReward::Gold,
    },
    DungeonChestRow {
        gate_threshold: 5,
        reward: DungeonChestReward::Keys,
    },
    DungeonChestRow {
        gate_threshold: 10,
        reward: DungeonChestReward::Gems,
    },
    DungeonChestRow {
        gate_threshold: 20,
        reward: DungeonChestReward::Torches,
    },
    DungeonChestRow {
        gate_threshold: 25,
        reward: DungeonChestReward::Potion,
    },
    DungeonChestRow {
        gate_threshold: 25,
        reward: DungeonChestReward::Scroll,
    },
];

/// `containers.md §6`: per-row gate. The first roll is uniform in
/// `1..=(4 * dungeon_depth + 4)`; the row is awarded when its threshold is
/// `<=` the roll. Caller passes the raw die roll and the row.
pub const fn dungeon_chest_row_gate_max(dungeon_depth: u8) -> u8 {
    4 * dungeon_depth + 4
}
pub const fn dungeon_chest_row_awarded(row: DungeonChestRow, gate_roll: u8) -> bool {
    row.gate_threshold <= gate_roll
}

/// `containers.md §7`: directional table-food consumption. Returns the
/// resulting tile id when the Get is allowed from the given relative
/// direction `(dx, dy)`; returns `None` for any invalid reach (horizontal,
/// diagonal, or wrong tile id), in which case caller prints the
/// cannot-reach-plate feedback and leaves the tile unchanged.
pub const TABLE_FOOD_TILE_A: u8 = 0x9B;
pub const TABLE_FOOD_TILE_B: u8 = 0x9C;
pub const fn table_food_get_resulting_tile(tile: u8, dx: i8, dy: i8) -> Option<u8> {
    if dx != 0 {
        return None;
    }
    match (tile, dy) {
        (TABLE_FOOD_TILE_A, -1) => Some(0x95),
        (TABLE_FOOD_TILE_B, -1) => Some(0x9A),
        (TABLE_FOOD_TILE_B, 1) => Some(0x9B),
        _ => None,
    }
}
