//! Helpers for container/Get behaviour per `containers.md`. Currently
//! covers the dungeon-chest reward generator (§6) and the directional
//! table-food consumption rule (§7).

/// `containers.md §4` surface/town chest content roll die. Both the
/// primary-pool per-row gate and the secondary-pool per-attempt gate
/// roll uniformly in `1..=CHEST_CONTENT_ROLL_DIE` (1..=30).
pub const CHEST_CONTENT_ROLL_DIE: u8 = 30;

/// `containers.md §4` secondary-pool attempt divisor. Attempt count
/// is `floor(chest_class / CHEST_SECONDARY_POOL_ATTEMPT_DIVISOR) +
/// CHEST_SECONDARY_POOL_ATTEMPT_BIAS`, so a chest class of zero
/// still runs one attempt.
pub const CHEST_SECONDARY_POOL_ATTEMPT_DIVISOR: u8 = 2;
/// `containers.md §4` secondary-pool attempt bias.
pub const CHEST_SECONDARY_POOL_ATTEMPT_BIAS: u8 = 1;

/// `containers.md §4` surface/town chest content primary-pool roll
/// gate. A pool row is eligible only when its threshold is less
/// than or equal to the chest class; eligible rows then roll
/// `1..=30` and succeed when the roll is greater than or equal to
/// the same threshold. Caller passes the chest's seven-bit class
/// (after the trap bit is removed) and the per-row threshold.
pub const fn chest_primary_pool_row_succeeds(
    chest_class: u8,
    threshold: u8,
    roll_1_to_30: u8,
) -> bool {
    threshold <= chest_class && roll_1_to_30 >= threshold
}

/// `containers.md §4` surface/town chest content secondary-pool
/// attempt count. After the primary pool is evaluated, the
/// secondary pool runs `floor(chest_class / 2) + 1` independent
/// attempts at random rows from the 48-entry equipment table.
pub const fn chest_secondary_pool_attempts(chest_class: u8) -> u8 {
    chest_class / CHEST_SECONDARY_POOL_ATTEMPT_DIVISOR + CHEST_SECONDARY_POOL_ATTEMPT_BIAS
}

/// `containers.md §4` chest primary-pool published row count and
/// per-row thresholds. Row ordering is one-based in the spec; this
/// array is zero-indexed (row 1 → index 0, row 8 → index 7).
pub const CHEST_PRIMARY_POOL_ROW_COUNT: usize = 8;
pub const CHEST_PRIMARY_POOL_THRESHOLDS: [u8; CHEST_PRIMARY_POOL_ROW_COUNT] = [
    7,  // 1 Food
    7,  // 2 Torches
    15, // 3 Gems
    9,  // 4 Keys
    17, // 5 Scroll
    17, // 6 Potion
    3,  // 7 Gold
    25, // 8 Chest marker
];

/// `containers.md §4` chest secondary-pool published row count.
/// The pool indices are the same `0..=47` equipment ids passed
/// to the inventory-add path on success. Anchored to
/// [`crate::EQUIPMENT_COUNT`] so the chest pool size and the
/// equipment catalog stay one value.
pub const CHEST_SECONDARY_POOL_ROW_COUNT: usize = crate::EQUIPMENT_COUNT;

/// `containers.md §4` chest secondary-pool per-row threshold table.
/// `None` is the "Disabled" sentinel — those rows never succeed for
/// ordinary chest classes. Indexed by the published 0..=47 pool
/// index, which is also the equipment subtype passed to the
/// inventory-add path on success.
pub const CHEST_SECONDARY_POOL_THRESHOLDS: [Option<u8>; CHEST_SECONDARY_POOL_ROW_COUNT] = [
    // 0..=8 Helm / Shield band
    Some(10),
    Some(10),
    Some(15),
    Some(20),
    Some(10),
    Some(15),
    Some(20),
    Some(28),
    None,
    // 9..=15 Armour band
    Some(15),
    Some(15),
    Some(20),
    Some(20),
    Some(20),
    Some(24),
    None,
    // 16..=41 Weapon band
    Some(5),
    Some(10),
    Some(10),
    Some(10),
    Some(10),
    Some(10),
    Some(10),
    Some(10),
    Some(15),
    Some(15),
    Some(15),
    Some(10),
    Some(15),
    Some(10),
    Some(20),
    Some(20),
    Some(20),
    Some(20),
    Some(20),
    None,
    Some(23),
    Some(23),
    Some(23),
    None,
    None,
    None,
    // 42..=47 Ring / Amulet band
    Some(23),
    Some(23),
    Some(23),
    Some(23),
    Some(15),
    None,
];

/// `containers.md §4`: returns the secondary-pool row's threshold,
/// or `None` for the published "Disabled" sentinel rows that never
/// succeed for ordinary chest classes.
pub const fn chest_secondary_pool_threshold(row_index: usize) -> Option<u8> {
    if row_index >= CHEST_SECONDARY_POOL_ROW_COUNT {
        return None;
    }
    CHEST_SECONDARY_POOL_THRESHOLDS[row_index]
}

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
/// `containers.md §8` per-grant unit count the inventory-add
/// dispatcher applies to one equipment-class result. The
/// ammunition rows (Arrows id 27, Quarrels id 29) grant five units
/// per award; every other equipment row grants one unit.
pub const INVENTORY_ADD_AMMO_UNITS: u8 = 5;
pub const INVENTORY_ADD_EQUIPMENT_UNITS: u8 = 1;

/// `containers.md §8`: returns the per-grant unit count for one
/// equipment-row inventory-add. Caller passes the equipment id; the
/// helper returns five for Arrows or Quarrels and one for every
/// other row.
pub const fn inventory_add_equipment_units(equipment_id: usize) -> u8 {
    if equipment_id == crate::EQUIPMENT_ID_ARROWS || equipment_id == crate::EQUIPMENT_ID_QUARRELS {
        INVENTORY_ADD_AMMO_UNITS
    } else {
        INVENTORY_ADD_EQUIPMENT_UNITS
    }
}

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

/// `dungeon-mode.md §8` dungeon-chest Search trap tier classifier.
/// Once the dungeon-chest Search has derived a tier value, this
/// helper maps it into the visible-narration band: tier `< 4` is
/// simple, tier `>= 7` is complex, the middle band is generic. The
/// tier itself is either a fresh `1..=8` roll (when the first roll
/// is at or below the threshold) or the current depth Z (when the
/// chest byte is already marked); both inputs share this band map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonChestTrapTier {
    Simple,
    Generic,
    Complex,
}

pub const fn dungeon_chest_trap_tier(tier: u8) -> DungeonChestTrapTier {
    if tier < 4 {
        DungeonChestTrapTier::Simple
    } else if tier >= 7 {
        DungeonChestTrapTier::Complex
    } else {
        DungeonChestTrapTier::Generic
    }
}

/// `dungeon-mode.md §8` Search-on-bomb-trap (exact byte `0x62`)
/// outcome. Search rolls `1..=30` against the shared dungeon-chest
/// threshold; a roll *above* the threshold springs the bomb and
/// clears the searched cell to `0x00`, while a roll at or below the
/// threshold leaves the cell unchanged with the generic "nothing"
/// reply. The bomb-spring branch reuses the published byte `0x00`
/// (passage / empty) for the cleared cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonBombSearchOutcome {
    /// Roll at or below the threshold — leaves the cell as `0x62`
    /// and reports the generic "nothing on the pit" preamble.
    NothingOnPit,
    /// Roll above the threshold — springs the bomb, reports it, and
    /// clears the searched cell to passage (`0x00`).
    SpringBomb,
}

/// `dungeon-mode.md §8`: resolve the Search-on-bomb-trap outcome
/// from the shared dungeon-chest threshold (computed by
/// `dungeon_chest_jimmy_threshold`) and the `1..=30` die roll. The
/// "spring" branch is `roll > threshold`; equal-or-below leaves the
/// cell alone.
pub const fn dungeon_bomb_search_outcome(
    threshold: u8,
    roll_1_to_30: u8,
) -> DungeonBombSearchOutcome {
    if roll_1_to_30 > threshold {
        DungeonBombSearchOutcome::SpringBomb
    } else {
        DungeonBombSearchOutcome::NothingOnPit
    }
}

/// `dungeon-mode.md §8` dungeon-chest Search outcome. Search shares
/// the dungeon Jimmy threshold formula
/// (`(2*depth - member_lockpick + 30) / 2`); the visible result then
/// depends on whether the first roll cleared the threshold and
/// whether the searched chest byte is already marked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonChestSearchOutcome {
    /// Plain closed chest byte and first roll above the threshold —
    /// Search reports "no trap".
    NoTrap,
    /// Trap-tier classification once a tier value has been derived.
    Trap(DungeonChestTrapTier),
}

/// `dungeon-mode.md §8`: resolve the dungeon-chest Search visible
/// result from the published inputs. `first_roll_1_to_30` is the
/// `1..=30` die roll; `fresh_tier_1_to_8` is the fresh `1..=8` roll
/// used when the first roll is at or below the threshold and the
/// chest byte is unmarked; `current_depth_z` is the active dungeon
/// depth used when the chest byte is already marked.
///
/// Per the spec: the no-trap branch fires only on `roll > threshold`
/// *and* `!chest_byte_marked`. Otherwise tier is the fresh roll on
/// an unmarked chest or the current Z on a marked chest, and is
/// then classified by [`dungeon_chest_trap_tier`].
pub const fn dungeon_chest_search_outcome(
    threshold: u8,
    first_roll_1_to_30: u8,
    fresh_tier_1_to_8: u8,
    current_depth_z: u8,
    chest_byte_marked: bool,
) -> DungeonChestSearchOutcome {
    if first_roll_1_to_30 > threshold && !chest_byte_marked {
        return DungeonChestSearchOutcome::NoTrap;
    }
    let tier = if chest_byte_marked {
        current_depth_z
    } else {
        fresh_tier_1_to_8
    };
    DungeonChestSearchOutcome::Trap(dungeon_chest_trap_tier(tier))
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

/// `containers.md §6` dungeon-chest gold-row multiplier. The gold
/// row's upper roll bound is computed as `MULTIPLIER * dungeon_depth`
/// before being passed to the shared one-based random helper.
pub const DUNGEON_CHEST_GOLD_DEPTH_MULTIPLIER: u8 = 8;

/// `containers.md §6` upper endpoint the gold row passes to the
/// shared one-based random helper: `8 * dungeon_depth`. At
/// `dungeon_depth == 0` this collapses to a `1..0` zero-width range
/// — compatible implementations preserve the PRNG advance and the
/// original divide-by-zero edge rather than clamping to 1.
pub const fn dungeon_chest_gold_upper(dungeon_depth: u8) -> u8 {
    DUNGEON_CHEST_GOLD_DEPTH_MULTIPLIER.wrapping_mul(dungeon_depth)
}

/// `containers.md §6`: returns `true` when the gold row would invoke
/// the shared range helper with an invalid (zero-width) upper bound.
pub const fn dungeon_chest_gold_is_zero_width(dungeon_depth: u8) -> bool {
    dungeon_chest_gold_upper(dungeon_depth) == 0
}

/// `containers.md §4` town-family chest moral-standing penalty.
/// When the chest helper opens a matching object-table chest in a
/// town-family scene, it reduces the shared moral-standing selector
/// by this many units, clamped at zero. Overworld chests do not
/// apply this penalty.
pub const TOWN_CHEST_OPEN_KARMA_DEBIT: u8 = 2;

/// `containers.md §4`: returns the post-debit moral-standing
/// selector after opening one town-family object-table chest. The
/// helper subtracts [`TOWN_CHEST_OPEN_KARMA_DEBIT`] with the
/// published zero-floor.
pub const fn town_chest_open_standing(standing: u8) -> u8 {
    standing.saturating_sub(TOWN_CHEST_OPEN_KARMA_DEBIT)
}

/// `containers.md §5` rare-reagent harvest quantity bounds. On a
/// successful midnight harvest at a published harvest point the
/// generator rolls in the inclusive `2..=15` range and adds the
/// rolled amount to the matching reagent counter with the published
/// 99-unit cap.
pub const RARE_REAGENT_HARVEST_QUANTITY_MIN: u8 = 2;
pub const RARE_REAGENT_HARVEST_QUANTITY_MAX: u8 = 15;
pub const RARE_REAGENT_HARVEST_HOUR: u8 = 0;
/// `containers.md §5` minute bound the harvest pass accepts.
/// Anchored to [`crate::MINUTES_PER_HOUR`] so the harvest minute
/// bound and the published hour length share one source of truth
/// — any minute in the hour-0 hour is eligible.
pub const RARE_REAGENT_HARVEST_MINUTE_BOUND: u8 = crate::MINUTES_PER_HOUR;

/// `containers.md §5`: width of the rare-reagent harvest band
/// (`MAX - MIN + 1`). Used as the modulus on the harvest-quantity
/// seed so every value in `[MIN, MAX]` is reachable.
pub const RARE_REAGENT_HARVEST_QUANTITY_SPAN: u8 =
    RARE_REAGENT_HARVEST_QUANTITY_MAX - RARE_REAGENT_HARVEST_QUANTITY_MIN + 1;

/// `containers.md §5`: returns the harvest-quantity amount for one
/// uniform seed byte. The shipped helper rolls `2 + (seed % 14)`,
/// which produces every value in `2..=15` either 18 or 19 times
/// across the 256-value seed domain.
pub const fn rare_reagent_harvest_quantity(seed: u8) -> u8 {
    RARE_REAGENT_HARVEST_QUANTITY_MIN + (seed % RARE_REAGENT_HARVEST_QUANTITY_SPAN)
}

/// `containers.md §5`: returns `true` when the in-game hour qualifies
/// as midnight for the rare-reagent harvest gate. The shipped check
/// requires `hour == 0` regardless of minute.
pub const fn rare_reagent_harvest_hour_accepted(hour: u8) -> bool {
    hour == RARE_REAGENT_HARVEST_HOUR
}

/// `containers.md §6` per-row reward quantity / subtype ranges
/// the dungeon chest generator passes to the shared random helper
/// after the gate succeeds.
///
/// Food rolls in `1..=DUNGEON_CHEST_FOOD_MAX`; Keys / Gems /
/// Torches roll in `1..=DUNGEON_CHEST_SMALL_MAX`; potion and
/// scroll subtypes roll in `0..=DUNGEON_CHEST_SUBTYPE_MAX`.
/// Gold's `1..=(8 * depth)` rule is in [`dungeon_chest_gold_upper`].
pub const DUNGEON_CHEST_FOOD_MAX: u8 = 31;
pub const DUNGEON_CHEST_SMALL_MAX: u8 = 3;
pub const DUNGEON_CHEST_SUBTYPE_MAX: u8 = 7;

/// `containers.md §6` per-row gate roll formula multiplier and bias.
/// The first roll is uniform in `1..=(MULTIPLIER * dungeon_depth + BIAS)`;
/// the row is awarded when its gate threshold is `<=` the roll.
pub const DUNGEON_CHEST_ROW_GATE_DEPTH_MULTIPLIER: u8 = 4;
pub const DUNGEON_CHEST_ROW_GATE_BIAS: u8 = 4;

/// `containers.md §6`: per-row gate. The first roll is uniform in
/// `1..=(4 * dungeon_depth + 4)`; the row is awarded when its threshold is
/// `<=` the roll. Caller passes the raw die roll and the row.
pub const fn dungeon_chest_row_gate_max(dungeon_depth: u8) -> u8 {
    DUNGEON_CHEST_ROW_GATE_DEPTH_MULTIPLIER * dungeon_depth + DUNGEON_CHEST_ROW_GATE_BIAS
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
