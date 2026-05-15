//! Equipment catalog metadata and R-Ready helpers.

use crate::*;

pub const EQUIPMENT_NAMES: [&str; EQUIPMENT_COUNT] = [
    "Leather Helm",
    "Chain Coif",
    "Iron Helm",
    "Spiked Helm",
    "Small Shield",
    "Large Shield",
    "Spiked Shield",
    "Magic Shield",
    "Jewel Shield",
    "Cloth Armour",
    "Leather Armour",
    "Ring Mail",
    "Scale Mail",
    "Chain Mail",
    "Plate Mail",
    "Mystic Armour",
    "Dagger",
    "Sling",
    "Club",
    "Flaming Oil",
    "Main Gauche",
    "Spear",
    "Throwing Axe",
    "Short Sword",
    "Mace",
    "Morning Star",
    "Bow",
    "Arrows",
    "Crossbow",
    "Quarrels",
    "Long Sword",
    "2H Hammer",
    "2H Axe",
    "2H Sword",
    "Halberd",
    "Sword of Chaos",
    "Magic Bow",
    "Silver Sword",
    "Magic Axe",
    "Glass Sword",
    "Jeweled Sword",
    "Mystic Sword",
    "Ring of Invisibility",
    "Ring of Protection",
    "Ring of Regeneration",
    "Amulet/Turning",
    "Spiked Collar",
    "Ankh",
];

pub const EQUIPMENT_CLASS_TAGS: [u8; EQUIPMENT_COUNT] = [
    0x80, 0x80, 0x80, 0x80, 0x20, 0x20, 0x20, 0x20, 0x20, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40,
    0x20, 0x30, 0x20, 0x30, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x30, 0x00, 0x30, 0x00, 0x20, 0x30,
    0x30, 0x30, 0x30, 0x30, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x02, 0x02, 0x02, 0x04, 0x04, 0x04,
];

pub const EQUIPMENT_READY_BURDENS: [u8; EQUIPMENT_COUNT] = [
    0, 1, 2, 3, 2, 3, 4, 0, 0, 0, 2, 4, 6, 10, 12, 0, 1, 2, 3, 2, 3, 4, 6, 5, 7, 8, 8, 0, 6, 0, 9,
    16, 15, 13, 18, 0, 0, 8, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub const EQUIPMENT_ATTACK_MAXES: [u8; EQUIPMENT_COUNT] = [
    0, 0, 0, 4, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 6, 8, 8, 8, 10, 10, 12, 15, 15, 10, 1, 12,
    1, 15, 20, 20, 20, 30, 99, 15, 12, 20, 99, 1, 30, 0, 0, 0, 0, 0, 0,
];

pub const EQUIPMENT_BASE_PRICES: [u16; EQUIPMENT_COUNT] = [
    15, 50, 120, 150, 40, 70, 120, 2000, 0, 20, 50, 100, 150, 300, 700, 0, 1, 10, 5, 5, 15, 7, 3,
    40, 50, 60, 75, 10, 150, 15, 70, 85, 150, 200, 250, 0, 800, 250, 1000, 0, 0, 0, 450, 500, 200,
    900, 240, 0,
];

pub const EQUIPMENT_WEAPON_RANGE_CAPS: [u8; EQUIPMENT_COUNT] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 4, 0, 4, 0, 5, 4, 0, 0, 2, 7, 0, 8, 0, 0, 0,
    0, 0, 2, 0, 15, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub const EQUIPMENT_WEAPON_EFFECT_CODES: [u8; EQUIPMENT_COUNT] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn equipment_name(item_id: usize) -> &'static str {
    EQUIPMENT_NAMES
        .get(item_id)
        .copied()
        .unwrap_or("Unknown equipment")
}

pub fn equipment_attack_max(item_id: usize) -> Option<u8> {
    EQUIPMENT_ATTACK_MAXES.get(item_id).copied()
}

pub fn equipment_base_price(item_id: usize) -> Option<u16> {
    EQUIPMENT_BASE_PRICES.get(item_id).copied()
}

pub fn equipment_weapon_range_cap(item_id: usize) -> Option<u8> {
    EQUIPMENT_WEAPON_RANGE_CAPS.get(item_id).copied()
}

pub fn equipment_weapon_effect_code(item_id: usize) -> Option<u8> {
    EQUIPMENT_WEAPON_EFFECT_CODES.get(item_id).copied()
}

pub fn default_party_strengths(party_len: usize) -> Vec<u8> {
    vec![AVATAR_STAT_MAX; party_len]
}

pub fn default_party_equipment(party_len: usize) -> Vec<[u8; EQUIPMENT_SLOT_COUNT]> {
    vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]; party_len]
}

pub fn equipment_stock_summary(stock: &[u8; EQUIPMENT_COUNT]) -> String {
    let entries = stock
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(item_id, count)| format!("{}={count}", equipment_name(item_id)))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

pub fn readied_equipment_summary(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> String {
    let entries = equipment
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(slot, item)| {
            (item != EQUIPMENT_EMPTY)
                .then(|| format!("{}={}", slot_name(slot), equipment_name(item as usize)))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

pub fn slot_name(slot: usize) -> &'static str {
    match slot {
        EQUIP_SLOT_HELM => "helm",
        EQUIP_SLOT_ARMOUR => "armour",
        EQUIP_SLOT_WEAPON => "weapon",
        EQUIP_SLOT_OFFHAND => "offhand",
        EQUIP_SLOT_RING => "ring",
        EQUIP_SLOT_AMULET => "amulet",
        _ => "slot",
    }
}

pub fn ready_burden(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> u8 {
    equipment
        .iter()
        .copied()
        .filter(|item| *item != EQUIPMENT_EMPTY)
        .filter_map(|item| EQUIPMENT_READY_BURDENS.get(item as usize).copied())
        .fold(0u8, u8::saturating_add)
}

pub fn is_amulet_turning_readied(equipment: &[u8; EQUIPMENT_SLOT_COUNT]) -> bool {
    equipment[EQUIP_SLOT_AMULET] == EQUIPMENT_ID_AMULET_TURNING as u8
}

pub fn is_shield_item(item_id: usize) -> bool {
    (4..=8).contains(&item_id)
}

pub fn is_magic_vanish_ring(item_id: usize) -> bool {
    matches!(
        item_id,
        EQUIPMENT_ID_RING_INVISIBILITY | EQUIPMENT_ID_RING_REGENERATION
    )
}
