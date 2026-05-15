//! Area, party roster, avatar stats, moonstone gate slots.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Town { scene: Scene, floor: i8 },
    Dungeon { scene: DungeonScene, level: u8 },
    World { plane: WorldPlane },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Player {
    pub x: usize,
    pub y: usize,
    pub facing: Direction,
    pub transport: TransportState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartyMember {
    pub slot: u8,
    pub class_byte: u8,
    pub status: u8,
    pub climb_stat: u8,
    pub mana: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub level: u8,
}

impl PartyMember {
    pub fn living(self) -> bool {
        self.hp > 0 && !matches!(self.status, b'D' | b'A')
    }

    pub fn conscious(self) -> bool {
        self.living() && self.status != b'S'
    }

    pub fn apply_damage(&mut self, damage: u8) -> u16 {
        let damage = damage as u16;
        let applied = self.hp.min(damage);
        self.hp -= applied;
        if self.hp == 0 {
            self.status = b'D';
        }
        applied
    }

    pub fn heal_by(&mut self, hp: u16) -> u16 {
        let applied = self.max_hp.saturating_sub(self.hp).min(hp);
        self.hp += applied;
        applied
    }

    pub fn recover_mana_by(&mut self, mana: u8) -> u8 {
        let applied = REST_MANA_CAP.saturating_sub(self.mana).min(mana);
        self.mana += applied;
        applied
    }

    pub fn heal_to_max(&mut self) -> (u16, u16) {
        let before = self.hp;
        self.hp = self.max_hp;
        (before, self.hp)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoonstoneGateSlot {
    pub scene: u8,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl MoonstoneGateSlot {
    pub const fn invalid() -> Self {
        Self {
            scene: MOONSTONE_INVALID_SCENE,
            x: 0,
            y: 0,
            z: 0,
        }
    }

    pub fn is_valid(self) -> bool {
        self.scene != MOONSTONE_INVALID_SCENE
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvatarStats {
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
}

impl AvatarStats {
    pub fn capped_seed() -> Self {
        Self {
            strength: AVATAR_STAT_MAX,
            dexterity: AVATAR_STAT_MAX,
            intelligence: AVATAR_STAT_MAX,
        }
    }

    pub fn increase_strength(&mut self) -> bool {
        increase_capped_stat(&mut self.strength)
    }

    pub fn increase_dexterity(&mut self) -> bool {
        increase_capped_stat(&mut self.dexterity)
    }

    pub fn increase_intelligence(&mut self) -> bool {
        increase_capped_stat(&mut self.intelligence)
    }
}

pub fn increase_capped_stat(stat: &mut u8) -> bool {
    if *stat >= AVATAR_STAT_MAX {
        false
    } else {
        *stat += 1;
        true
    }
}

impl Default for AvatarStats {
    fn default() -> Self {
        Self::capped_seed()
    }
}

pub fn default_party() -> Vec<PartyMember> {
    vec![PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 8,
        hp: DEFAULT_PARTY_HP,
        max_hp: DEFAULT_PARTY_MAX_HP,
        level: 8,
    }]
}

pub fn default_party_names(party_len: usize) -> Vec<[u8; SAVE_CHARACTER_NAME_LEN]> {
    let mut names = vec![[0; SAVE_CHARACTER_NAME_LEN]; party_len];
    if let Some(leader) = names.get_mut(0) {
        leader[..6].copy_from_slice(b"Avatar");
    }
    names
}

pub fn party_name_to_string(name: &[u8]) -> Option<String> {
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    let display = String::from_utf8_lossy(&name[..end]).trim_end().to_string();
    (!display.is_empty()).then_some(display)
}

pub fn default_party_experience(party_len: usize) -> Vec<u16> {
    vec![0; party_len]
}

pub fn default_party_stay_counters(party_len: usize) -> Vec<u8> {
    vec![0; party_len]
}

pub fn default_party_intelligence(party_len: usize) -> Vec<u8> {
    vec![AVATAR_STAT_MAX; party_len]
}

pub fn recompute_level_from_experience(experience: u16) -> u8 {
    let mut level = 1;
    let mut quotient = experience / 100;
    while quotient != 0 {
        level += 1;
        quotient /= 2;
    }
    level
}

pub fn class_refreshed_mana(class_byte: u8, intelligence: u8) -> Option<u8> {
    match class_byte {
        b'A' | b'M' => Some(intelligence),
        b'B' => Some(intelligence / 2),
        _ => None,
    }
}

pub const fn heal_spell_amount_from_raw_roll(raw_roll: u8) -> u16 {
    let amount = raw_roll / 2;
    if amount == 0 { 1 } else { amount as u16 }
}

pub fn resurrection_adjusted_experience(experience: u16, moral_standing: u8) -> u16 {
    if moral_standing >= 98 {
        return experience;
    }

    let divisor = u32::from(moral_standing.max(1));
    ((u32::from(experience) * 100) / divisor).min(u32::from(u16::MAX)) as u16
}

pub fn party_status_name(status: u8) -> &'static str {
    match status {
        b'G' => "good",
        b'P' => "poisoned",
        b'S' => "asleep",
        b'D' => "dead",
        b'A' => "ashes",
        _ => "status-tagged",
    }
}

pub fn party_member_unavailable_message(party_len: usize) -> String {
    format!(
        "Party has {} member{}.",
        party_len,
        if party_len == 1 { "" } else { "s" }
    )
}

pub fn potion_label(index: usize) -> &'static str {
    const LABELS: [&str; POTION_COUNT] = [
        "blue", "yellow", "red", "green", "orange", "purple", "black", "white",
    ];
    LABELS.get(index).copied().unwrap_or("unknown")
}

pub fn potion_effect_index_after_variation(
    selected_index: usize,
    variation_roll: u8,
    random_roll: u8,
) -> usize {
    match variation_roll & 0x0f {
        0..=13 => selected_index,
        14 => POTION_ORANGE_INDEX,
        _ => (random_roll as usize) & 0x07,
    }
}
