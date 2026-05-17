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

/// `u4-transfer.md §7` / `magic.md §8` level-from-experience scaler.
/// Level starts at 1; experience is divided by this base to get the
/// first quotient, and the level advances once for each subsequent
/// halving step while that quotient remains nonzero (so 100..199
/// XP = level 2, 200..399 = level 3, 400..799 = level 4, ...).
pub const LEVEL_FROM_EXPERIENCE_BASE_DIVISOR: u16 = 100;
/// `u4-transfer.md §7` / `magic.md §8` level-from-experience halving
/// step. After dividing experience by [`LEVEL_FROM_EXPERIENCE_BASE_DIVISOR`],
/// the quotient is repeatedly divided by this step; each nonzero
/// quotient raises the resulting level by one.
pub const LEVEL_FROM_EXPERIENCE_HALVING_STEP: u16 = 2;

pub fn recompute_level_from_experience(experience: u16) -> u8 {
    let mut level = 1;
    let mut quotient = experience / LEVEL_FROM_EXPERIENCE_BASE_DIVISOR;
    while quotient != 0 {
        level += 1;
        quotient /= LEVEL_FROM_EXPERIENCE_HALVING_STEP;
    }
    level
}

/// `magic.md §8` Resurrection class-refresh table: Avatar (A), Mage (M),
/// and the default class branch receive mana equal to Intelligence; Bard
/// (B) receives half Intelligence. Returns `None` only when the spec asks
/// the caller to leave the existing MP value alone, which the current
/// trace does not promote — every U5 class letter resolves through this
/// table.
pub fn class_refreshed_mana(class_byte: u8, intelligence: u8) -> Option<u8> {
    match class_byte {
        b'B' => Some(intelligence / 2),
        // Avatar, Mage, and any other class fall through to the default
        // full-Intelligence branch per spec.
        _ => Some(intelligence),
    }
}

pub const fn heal_spell_amount_from_raw_roll(raw_roll: u8) -> u16 {
    let amount = raw_roll / 2;
    if amount == 0 { 1 } else { amount as u16 }
}

/// `magic.md §8` per-level maximum-HP factor used when a successful
/// Resurrect rebuilds the revived member's record. Maximum HP is set
/// to `30 * level` after the experience-driven level recompute.
pub const RESURRECTION_MAX_HP_PER_LEVEL: u16 = 30;

/// `magic.md §8` post-rebuild current HP. The spell path stands the
/// resurrected member up with exactly one hit point; healer-shop
/// callers may immediately top the same member back up after
/// invoking the spell helper, but the helper itself returns the
/// member at 1 HP.
pub const RESURRECTION_REBUILT_CURRENT_HP: u16 = 1;

/// `magic.md §8`: maximum HP a resurrected member receives for the
/// supplied recomputed level. Saturates at `u16::MAX` so callers do
/// not need to guard absurd levels.
pub const fn resurrection_max_hp_for_level(level: u8) -> u16 {
    (level as u16).saturating_mul(RESURRECTION_MAX_HP_PER_LEVEL)
}

/// `magic.md §8` / `karma.md §5`: revived member's experience after
/// the resurrection rescale. Per `magic.md §8`, when the moral-
/// standing selector is below 98 the helper "rescales the target's
/// experience by multiplying by 100 and dividing by the selector
/// before recomputing level"; selector `>= 98` skips the rescale.
/// The `magic.md` wording is the explicit mathematical formula and
/// is preserved here; the `karma.md §5` "scaled down by the
/// selector percentage" phrasing is the narrative summary and
/// resolves to the same expression.
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

/// `inventory.md §7` potion variation roll mask. The variation roll
/// is reduced modulo [`POTION_VARIATION_DENOMINATOR`] (16) by masking
/// the low four bits before the threshold compare.
pub const POTION_VARIATION_ROLL_MASK: u8 = 0x0F;
/// `inventory.md §7` last roll value (inclusive) that keeps the
/// selected colour's effect. Fourteen of the sixteen outcomes
/// (`0..=13`) preserve the chosen potion's effect; the remaining two
/// outcomes force Orange or substitute a random potion.
pub const POTION_VARIATION_SELECTED_THRESHOLD: u8 = 13;
/// `inventory.md §7` exact roll value that forces the Orange sleep
/// effect.
pub const POTION_VARIATION_FORCED_ORANGE_ROLL: u8 = 14;
/// `inventory.md §7` random-replacement mask. The random potion
/// substitution masks the secondary roll to `POTION_COUNT - 1`
/// (`0x07`) to pick uniformly from rows `0..=7`.
pub const POTION_VARIATION_RANDOM_INDEX_MASK: u8 = (POTION_COUNT - 1) as u8;

pub fn potion_effect_index_after_variation(
    selected_index: usize,
    variation_roll: u8,
    random_roll: u8,
) -> usize {
    match variation_roll & POTION_VARIATION_ROLL_MASK {
        roll if roll <= POTION_VARIATION_SELECTED_THRESHOLD => selected_index,
        POTION_VARIATION_FORCED_ORANGE_ROLL => POTION_ORANGE_INDEX,
        _ => (random_roll as usize) & POTION_VARIATION_RANDOM_INDEX_MASK as usize,
    }
}
