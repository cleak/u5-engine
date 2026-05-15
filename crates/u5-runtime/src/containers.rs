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

/// `containers.md §8`: counter cap the inventory-add dispatcher
/// applies for each result family. Returns `None` for families that
/// are flag/event-only (no quantity counter), refusal-only families,
/// or families the spec leaves uncapped (gems and food: §8 lists no
/// cap). Gold uses the party gold cap of 9999; per-counter quantity
/// families (potion, scroll, equipment, key, torch) cap at 99.
pub const fn inventory_add_class_cap(class: InventoryAddClass) -> Option<u16> {
    match class {
        InventoryAddClass::Gold => Some(9999),
        InventoryAddClass::Potion
        | InventoryAddClass::ScrollOrPlans
        | InventoryAddClass::Equipment
        | InventoryAddClass::Key
        | InventoryAddClass::Torch => Some(99),
        InventoryAddClass::Gem
        | InventoryAddClass::Food
        | InventoryAddClass::MustOpenFirst
        | InventoryAddClass::SandalwoodBox
        | InventoryAddClass::Moonstone
        | InventoryAddClass::MagicCarpet
        | InventoryAddClass::ShadowlordShard
        | InventoryAddClass::CrownOfLordBritish
        | InventoryAddClass::SceptreOfLordBritish
        | InventoryAddClass::AmuletOfLordBritish
        | InventoryAddClass::NothingToGet => None,
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

/// `containers.md §5` Search trap-detection threshold for a per-map
/// object's slot metadata. The `trappable` flag distinguishes the
/// two formulas; both halve the unsigned-word intermediate value.
/// Caller rolls `1..=30` and the detection bit is set when the roll
/// is greater than or equal to the threshold.
pub const fn search_trap_detection_threshold(
    trappable: bool,
    difficulty: u8,
    member_trap_detection: u8,
) -> u8 {
    if trappable {
        let raw = (difficulty as i16) - (member_trap_detection as i16) + 30;
        if raw < 0 {
            0
        } else {
            (raw as u16 / 2) as u8
        }
    } else {
        let raw = 30i16 - member_trap_detection as i16;
        if raw < 0 {
            0
        } else {
            (raw as u16 / 2) as u8
        }
    }
}

/// `containers.md §5` Visible result of the trap-detection narrator
/// for a per-map object slot. The classifier consumes the trappable
/// flag, the slot's low difficulty value, and the detection bit
/// (set when the `1..=30` roll is `>=` the threshold).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchTrapVisibility {
    /// "No trap." — trappable + bit clear, or non-trappable + bit set.
    NoTrap,
    /// "Simple trap." — trappable + bit set + difficulty `< 10`.
    SimpleTrap,
    /// "Complex trap." — trappable + bit set + difficulty `> 20`.
    ComplexTrap,
    /// "Generic trap." — trappable + bit set + difficulty `10..=20`,
    /// or non-trappable + bit clear (false positive).
    GenericTrap,
}

/// `containers.md §5`: classify the visible trap narration result from
/// the slot's trappable flag, low difficulty value, and detection bit.
pub const fn search_trap_visibility(
    trappable: bool,
    difficulty: u8,
    detection_bit: bool,
) -> SearchTrapVisibility {
    if trappable {
        if !detection_bit {
            SearchTrapVisibility::NoTrap
        } else if difficulty < 10 {
            SearchTrapVisibility::SimpleTrap
        } else if difficulty > 20 {
            SearchTrapVisibility::ComplexTrap
        } else {
            SearchTrapVisibility::GenericTrap
        }
    } else if detection_bit {
        SearchTrapVisibility::NoTrap
    } else {
        SearchTrapVisibility::GenericTrap
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

/// `containers.md §6` upper endpoint the gold row passes to the
/// shared one-based random helper: `8 * dungeon_depth`. At
/// `dungeon_depth == 0` this collapses to a `1..0` zero-width range
/// — compatible implementations preserve the PRNG advance and the
/// original divide-by-zero edge rather than clamping to 1.
pub const fn dungeon_chest_gold_upper(dungeon_depth: u8) -> u8 {
    8u8.wrapping_mul(dungeon_depth)
}

/// `containers.md §6`: returns `true` when the gold row would invoke
/// the shared range helper with an invalid (zero-width) upper bound.
pub const fn dungeon_chest_gold_is_zero_width(dungeon_depth: u8) -> bool {
    dungeon_chest_gold_upper(dungeon_depth) == 0
}

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
