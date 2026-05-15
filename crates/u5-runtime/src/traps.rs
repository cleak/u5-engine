use crate::TRAP_NON_COMBAT_EFFECT_TABLE;

pub fn shared_trap_effect_id_from_index(index: u8, combat_active: bool) -> u8 {
    if combat_active {
        index & 1
    } else {
        TRAP_NON_COMBAT_EFFECT_TABLE[usize::from(index & 7)]
    }
}

pub fn shared_trap_damage_from_index(index: u8, max_damage: u8) -> u8 {
    1 + (index % max_damage)
}
