//! Helpers for container/Get behaviour per `containers.md`. Currently
//! covers the dungeon-chest reward generator (§6) and the directional
//! table-food consumption rule (§7).

/// `containers.md §8` shared inventory-add result family classified
/// from the found-item class code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryAddClass {
    /// `0x01` — closed-container "must open first" refusal.
    MustOpenFirst,
    /// `0x02` — gold (party gold counter, capped at 9999).
    Gold,
    /// `0x03` — potion of the supplied subtype (cap 99 per colour).
    Potion,
    /// `0x04` — scroll of the supplied subtype OR the HMS Cape plans
    /// flag.
    ScrollOrPlans,
    /// `0x05`, `0x06`, `0x09..=0x0C` — equipment row N. Arrows and
    /// Quarrels grant 5; other equipment grants 1.
    Equipment,
    /// `0x07` — key (cap 99). Marked odd-key subtypes route to the
    /// special-key counter.
    Key,
    /// `0x08` — gem.
    Gem,
    /// `0x0D` — torch (cap 99).
    Torch,
    /// `0x0E` — Sandalwood Box flag.
    SandalwoodBox,
    /// `0x0F` — food/grain.
    Food,
    /// `0x19` — Moonstone flag for the supplied slot.
    Moonstone,
    /// `0x1B` — Magic Carpet ownership.
    MagicCarpet,
    /// `0xB4` — Shadowlord shard (Falsehood/Hatred/Cowardice).
    ShadowlordShard,
    /// `0xB5` — Crown of Lord British.
    CrownOfLordBritish,
    /// `0xB6` — Sceptre of Lord British.
    SceptreOfLordBritish,
    /// `0xB7` — Amulet of Lord British.
    AmuletOfLordBritish,
    /// Any other class code — prints the nothing-to-get refusal and
    /// leaves inventory unchanged.
    NothingToGet,
}

/// `containers.md §8`: classify a found-item class code into the
/// inventory-add result family the dispatcher applies.
pub const fn inventory_add_class(class_code: u8) -> InventoryAddClass {
    match class_code {
        0x01 => InventoryAddClass::MustOpenFirst,
        0x02 => InventoryAddClass::Gold,
        0x03 => InventoryAddClass::Potion,
        0x04 => InventoryAddClass::ScrollOrPlans,
        0x05 | 0x06 | 0x09..=0x0C => InventoryAddClass::Equipment,
        0x07 => InventoryAddClass::Key,
        0x08 => InventoryAddClass::Gem,
        0x0D => InventoryAddClass::Torch,
        0x0E => InventoryAddClass::SandalwoodBox,
        0x0F => InventoryAddClass::Food,
        0x19 => InventoryAddClass::Moonstone,
        0x1B => InventoryAddClass::MagicCarpet,
        0xB4 => InventoryAddClass::ShadowlordShard,
        0xB5 => InventoryAddClass::CrownOfLordBritish,
        0xB6 => InventoryAddClass::SceptreOfLordBritish,
        0xB7 => InventoryAddClass::AmuletOfLordBritish,
        _ => InventoryAddClass::NothingToGet,
    }
}

/// `containers.md §8` per-row equipment-grant size: arrows (`0x05`)
/// and quarrels (`0x06`) grant 5 units per award; other equipment
/// rows (`0x09..=0x0C`) grant 1.
pub const fn equipment_grant_quantity(class_code: u8) -> u8 {
    match class_code {
        0x05 | 0x06 => 5,
        _ => 1,
    }
}

/// `containers.md §5` Search location-prefix classification for the
/// live tile under the searched coordinate. The prefix names the
/// scenery the search narration mentions before any found-object
/// text. Returns `None` for tiles that take the generic find prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchLocationPrefix {
    /// `0x2B` — stump.
    Stump,
    /// `0x4F` — wall.
    Wall,
    /// `0x5A` — shelf.
    Shelf,
    /// `0x5C..=0x5D` — bookshelf.
    Bookshelf,
    /// `0xA1` — well.
    Well,
    /// `0xA5` — desk.
    Desk,
    /// `0xA6` — barrel.
    Barrel,
    /// `0xA8` — vanity.
    Vanity,
    /// `0xAB..=0xAC` — under bed.
    UnderBed,
    /// `0xAD` — dresser.
    Dresser,
    /// `0xAF` — trunk.
    Trunk,
    /// `0xB2` — brazier.
    Brazier,
    /// `0xBC` — fireplace.
    Fireplace,
}

/// `containers.md §5`: classify a live tile as one of the named
/// location-prefix scenery cells. Returns `None` for ordinary tiles
/// (the search narration uses the generic find prefix).
pub const fn search_location_prefix(tile: u8) -> Option<SearchLocationPrefix> {
    Some(match tile {
        0x2B => SearchLocationPrefix::Stump,
        0x4F => SearchLocationPrefix::Wall,
        0x5A => SearchLocationPrefix::Shelf,
        0x5C | 0x5D => SearchLocationPrefix::Bookshelf,
        0xA1 => SearchLocationPrefix::Well,
        0xA5 => SearchLocationPrefix::Desk,
        0xA6 => SearchLocationPrefix::Barrel,
        0xA8 => SearchLocationPrefix::Vanity,
        0xAB | 0xAC => SearchLocationPrefix::UnderBed,
        0xAD => SearchLocationPrefix::Dresser,
        0xAF => SearchLocationPrefix::Trunk,
        0xB2 => SearchLocationPrefix::Brazier,
        0xBC => SearchLocationPrefix::Fireplace,
        _ => return None,
    })
}

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
