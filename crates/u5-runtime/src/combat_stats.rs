//! Published combat-class stat rows and encounter count helpers.

pub const COMBAT_CLASS_COUNT: usize = 48;
/// `combat.md §5` shared spawn-count cap: "The reroll arm ends with
/// a defensive cap at twenty-six." It is a clamp on the rolled
/// count, not a combatant capacity — `active-objects.md §7` states
/// combat has "no reserved player slot and no twenty-six-combatant
/// cap". With shipped class data the clamp is unreachable: "The
/// twenty-six cap is therefore unreachable defensive code, placement
/// slot indexes sixteen through twenty-five are never used, and a
/// conforming engine may treat the sixteen placement slots as
/// sufficient for every terrain encounter."
pub const COMBAT_SPAWN_COUNT_CAP: u8 = 26;

/// `combat.md §5` per-arena spawn-count exact-count sentinels. The
/// terrain setup helper treats these byte values as the exact spawn
/// count and uses them unchanged; any other nonzero value is treated
/// as a maximum and uniformly rolled in `[1, max]`. The
/// fortunes-of-war flag re-rolls the maximum case once before the
/// shared cap clamp.
pub const COMBAT_SPAWN_COUNT_EXACT_VALUES: [u8; 3] = [1, 8, 16];

/// `combat.md §11` / `catalogs/monster-bestiary.md §3`: the **melee**
/// sentinel value of the class-indexed range/effect selector. On the
/// shared spell/weapon dispatcher's non-party-side arm "value `1` is
/// folded to zero and selects the **melee / Aim-cursor arm**".
///
/// `RETRACTIONS.md` R360 inverts the polarity this constant used to
/// carry: it was published as "the zero-damage sentinel that routes into
/// the cast/effect branch", and that reading is withdrawn. It is a *non*-
/// zero selector above the melee value that routes into the
/// projectile/effect arm.
pub const RANGED_EFFECT_MELEE_SELECTOR: u8 = 1;

/// `combat.md §11`, the per-consumer selector table, **spell/weapon
/// dispatcher** row: selector `1` is "folded to zero, selecting the
/// **melee / Aim-cursor arm**", while "a selector above `1`" selects
/// "the **cast/effect arm unconditionally, at every distance including
/// one** - this routine contains no distance test at all".
///
/// This is the dispatcher's contract only. The AI attack resolver reads
/// the same byte as an inclusive maximum range and routes distance one to
/// melee ([`crate::resolve_combat_ai_attack_route`]); `combat.md §11`
/// forbids merging the two into one rule.
pub const fn combat_dispatcher_selector_routes_to_cast_effect(selector: u8) -> bool {
    selector > RANGED_EFFECT_MELEE_SELECTOR
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatClassStats {
    pub class: u8,
    pub name: &'static str,
    pub tier: u8,
    pub speed_seed: u8,
    pub endurance: u8,
    pub defense: u8,
    /// `combat.md §13` (R336): the class stat row's byte `+4`, renamed
    /// from `Attack cap`. "A monster's attack value is that class byte
    /// used flat, with no random draw at all" - `RETRACTIONS.md` R336.
    /// Only the *party* side of the damage roller randomizes, and it
    /// does so from the readied item's `Attack max`.
    pub attack_value: u8,
    pub max_hp: u8,
    pub default_spawn_count: u8,
    pub default_drop_cap: u8,
}

/// `catalogs/monster-bestiary.md §1` reward-unit derivation divisor.
/// The raw value the damage/death handler returns when a hostile
/// class dies is `floor(class_max_hp / MONSTER_REWARD_UNIT_HP_DIVISOR)
/// + MONSTER_REWARD_UNIT_BIAS`. Combat callers consume the returned
/// value immediately as party-attacker experience, capped at 9999.
pub const MONSTER_REWARD_UNIT_HP_DIVISOR: u8 = 4;
/// `catalogs/monster-bestiary.md §1` reward-unit derivation bias.
/// Added to the floored HP divisor so every classed kill credits at
/// least one unit of attacker experience.
pub const MONSTER_REWARD_UNIT_BIAS: u8 = 1;

impl CombatClassStats {
    pub const fn reward_unit(self) -> u8 {
        (self.max_hp / MONSTER_REWARD_UNIT_HP_DIVISOR) + MONSTER_REWARD_UNIT_BIAS
    }

    pub const fn raw_row(self) -> [u8; 8] {
        [
            self.tier,
            self.speed_seed,
            self.endurance,
            self.defense,
            self.attack_value,
            self.max_hp,
            self.default_spawn_count,
            self.default_drop_cap,
        ]
    }

    pub const fn mass_charm_threshold(self) -> u8 {
        self.endurance
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatRangedEffectStats {
    pub class: u8,
    pub name: &'static str,
    pub range_effect_selector: u8,
    pub payload: u8,
    pub scene_resistance: bool,
    pub food_theft_branch: bool,
    pub pre_gate_bypass: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatClassTraits {
    pub class: u8,
    pub name: &'static str,
    pub splits: bool,
    pub physical_half: bool,
    pub physical_immune: bool,
    /// `combat.md §11` / `§13`: the per-class flag the shared actor-rating
    /// selector tests. "monster whose class carries the `zero-selector stat
    /// row` trait | the class **combat tier**" - the six classes carrying it
    /// in the analyzed v1 data are Mimic, Reaper, Gargoyle, Orc, Ettin and
    /// Headless; every other class feeds its combat weight into the attacker
    /// term. Renamed from `team_override` by `RETRACTIONS.md` R337.
    pub zero_selector_stat_row: bool,
    pub vanish_branch: bool,
    /// `combat.md §6.3` "Death-marker tile bytes": the class-flag
    /// word's low bit, "call it the *incorporeal* bit". `§12`
    /// "Special-class death paths": "Incorporeal death (low bit set,
    /// vanish bit clear ...) releases the slot immediately and leaves
    /// **no tile marker and no drop at all**. This is a distinct
    /// branch, not a variant of the default kill."
    pub incorporeal: bool,
    pub special_death: bool,
    pub possess: bool,
    pub blink: bool,
    pub summon_daemon: bool,
    pub poison_status_attack: bool,
    pub turnable_attack: bool,
    pub teleport_capable: bool,
    /// `combat.md §9` / `magic.md §8` / `catalogs/monster-bestiary.md §4`:
    /// "Repel Undead is the same sweep with one extra condition: the
    /// actor's class must also carry the undead class-flag bit." The
    /// bestiary marks exactly two shipped classes as undead encounters -
    /// Ghost (23, "Undead or spectral encounter") and Skeleton (33,
    /// "Undead encounter") - so those two rows carry the bit here.
    pub undead: bool,
}

macro_rules! stats {
    ($class:literal, $name:literal, $tier:literal, $speed:literal, $hp_cmp:literal, $defense:literal, $attack:literal, $hp:literal, $spawn:literal, $drop:literal) => {
        CombatClassStats {
            class: $class,
            name: $name,
            tier: $tier,
            speed_seed: $speed,
            endurance: $hp_cmp,
            defense: $defense,
            attack_value: $attack,
            max_hp: $hp,
            default_spawn_count: $spawn,
            default_drop_cap: $drop,
        }
    };
}

macro_rules! ranged {
    ($class:literal, $name:literal, $selector:literal, $payload:literal, $scene_resistance:literal, $theft:literal, $pre_gate_bypass:literal) => {
        CombatRangedEffectStats {
            class: $class,
            name: $name,
            range_effect_selector: $selector,
            payload: $payload,
            scene_resistance: $scene_resistance,
            food_theft_branch: $theft,
            pre_gate_bypass: $pre_gate_bypass,
        }
    };
}

macro_rules! traits {
    ($class:literal, $name:literal) => {
        CombatClassTraits {
            class: $class,
            name: $name,
            ..CombatClassTraits::empty()
        }
    };
    ($class:literal, $name:literal, $($field:ident),+) => {{
        let mut traits = CombatClassTraits {
            class: $class,
            name: $name,
            ..CombatClassTraits::empty()
        };
        $(
            traits.$field = true;
        )+
        traits
    }};
}

impl CombatClassTraits {
    pub const fn empty() -> Self {
        Self {
            class: 0,
            name: "",
            splits: false,
            physical_half: false,
            physical_immune: false,
            zero_selector_stat_row: false,
            vanish_branch: false,
            incorporeal: false,
            special_death: false,
            possess: false,
            blink: false,
            summon_daemon: false,
            poison_status_attack: false,
            turnable_attack: false,
            teleport_capable: false,
            undead: false,
        }
    }
}

pub fn combat_class_stats(class: u8) -> Option<CombatClassStats> {
    match class {
        0 => Some(stats!(0, "Mage", 10, 15, 20, 0, 15, 10, 3, 20)),
        1 => Some(stats!(1, "Bard", 15, 20, 10, 4, 12, 15, 9, 10)),
        2 => Some(stats!(2, "Fighter", 20, 15, 10, 8, 15, 20, 6, 15)),
        3 => Some(stats!(3, "Avatar", 25, 25, 25, 7, 30, 20, 1, 25)),
        4 => Some(stats!(4, "Villager", 12, 12, 12, 0, 6, 8, 1, 10)),
        5 => Some(stats!(5, "Merchant", 12, 12, 18, 0, 6, 8, 1, 10)),
        6 => Some(stats!(6, "Jester", 12, 18, 12, 0, 6, 8, 1, 10)),
        7 => Some(stats!(7, "Bard (second row)", 12, 16, 14, 0, 6, 8, 1, 10)),
        8 => Some(stats!(8, "Pirate", 12, 12, 12, 0, 0, 5, 1, 0)),
        9 => Some(stats!(9, "Unnamed reserved", 12, 12, 12, 0, 0, 5, 1, 0)),
        10 => Some(stats!(10, "Child", 8, 8, 8, 0, 0, 5, 1, 0)),
        11 => Some(stats!(11, "Beggar", 8, 8, 8, 0, 0, 5, 1, 0)),
        12 => Some(stats!(12, "Guard", 22, 30, 10, 6, 30, 99, 8, 5)),
        13 => Some(stats!(13, "Wanderer", 30, 30, 30, 30, 99, 99, 1, 0)),
        14 => Some(stats!(14, "Blackthorn", 30, 30, 30, 30, 30, 99, 1, 0)),
        15 => Some(stats!(15, "Lord British", 30, 30, 30, 30, 99, 99, 1, 0)),
        16 => Some(stats!(16, "Sea Horse", 17, 20, 20, 2, 10, 30, 3, 0)),
        17 => Some(stats!(17, "Squid", 24, 20, 8, 0, 20, 50, 2, 0)),
        18 => Some(stats!(18, "Sea Serpent", 17, 17, 8, 2, 30, 70, 1, 0)),
        19 => Some(stats!(19, "Shark", 20, 17, 5, 0, 8, 22, 10, 0)),
        20 => Some(stats!(20, "Giant Rat", 5, 20, 5, 0, 6, 10, 10, 5)),
        21 => Some(stats!(21, "Bat", 5, 30, 5, 0, 6, 5, 16, 0)),
        22 => Some(stats!(22, "Giant Spider", 10, 10, 5, 0, 8, 10, 4, 5)),
        23 => Some(stats!(23, "Ghost", 1, 20, 10, 0, 12, 20, 6, 0)),
        24 => Some(stats!(24, "Slime", 6, 6, 2, 0, 4, 10, 16, 0)),
        25 => Some(stats!(25, "Gremlin", 10, 21, 10, 2, 4, 10, 13, 12)),
        26 => Some(stats!(26, "Mimic", 20, 30, 12, 3, 15, 30, 1, 20)),
        27 => Some(stats!(27, "Reaper", 20, 25, 12, 4, 20, 40, 3, 25)),
        28 => Some(stats!(28, "Gazer", 8, 10, 25, 0, 10, 20, 4, 0)),
        29 => Some(stats!(29, "Crawler", 17, 15, 12, 0, 15, 35, 4, 0)),
        30 => Some(stats!(30, "Gargoyle", 20, 10, 5, 15, 20, 40, 1, 0)),
        31 => Some(stats!(31, "Insect Swarm", 1, 30, 1, 0, 4, 5, 10, 0)),
        32 => Some(stats!(32, "Orc", 15, 13, 10, 2, 12, 10, 10, 11)),
        33 => Some(stats!(33, "Skeleton", 10, 20, 5, 0, 12, 20, 8, 13)),
        34 => Some(stats!(34, "Python", 5, 18, 8, 1, 8, 10, 4, 0)),
        35 => Some(stats!(35, "Ettin", 20, 15, 12, 3, 15, 30, 6, 17)),
        36 => Some(stats!(36, "Headless", 19, 12, 8, 2, 12, 20, 8, 12)),
        37 => Some(stats!(37, "Wisp", 8, 30, 20, 0, 20, 40, 4, 0)),
        38 => Some(stats!(38, "Daemon", 25, 25, 25, 5, 20, 75, 4, 0)),
        39 => Some(stats!(39, "Dragon", 30, 25, 25, 10, 30, 99, 2, 30)),
        40 => Some(stats!(40, "Sand Trap", 25, 25, 5, 10, 30, 80, 1, 25)),
        41 => Some(stats!(41, "Troll", 18, 17, 9, 4, 15, 15, 4, 15)),
        42 => Some(stats!(42, "Reserved gap", 0, 0, 0, 0, 0, 0, 0, 0)),
        43 => Some(stats!(43, "Reserved gap", 0, 0, 0, 0, 0, 0, 0, 0)),
        44 => Some(stats!(44, "Mongbat", 10, 30, 15, 4, 20, 20, 16, 5)),
        45 => Some(stats!(45, "Corpser", 17, 10, 8, 0, 15, 40, 4, 0)),
        46 => Some(stats!(46, "Rot Worm", 5, 17, 6, 0, 6, 5, 10, 0)),
        47 => Some(stats!(47, "Shadow Lord", 25, 30, 30, 10, 30, 99, 1, 0)),
        _ => None,
    }
}

/// `combat.md §6.3` Death-marker table, Incorporeal-class row: the
/// classes "whose class-flag word has the low bit set but **not** the
/// vanish bit" are "Sea Horse, Squid, Sea Serpent, Shark, Bat, Ghost,
/// Slime, Insect Swarm, Wisp, Daemon" — classes 16, 17, 18, 19, 21,
/// 23, 24, 31, 37 and 38 of this table.
pub fn combat_class_traits(class: u8) -> Option<CombatClassTraits> {
    match class {
        0 => Some(traits!(0, "Mage", turnable_attack)),
        // `combat.md §13`: the per-class flag word is "Sixteen bits per
        // class" over the same forty-eight-row class space as the stat
        // record, and "party combat classes, special NPC classes, and
        // monsters share the same eight-byte row shape". Classes 1..11 are
        // the remaining party sprites and the townsfolk/NPC actor rows;
        // `catalogs/monster-bestiary.md §4` records "only the class traits
        // confirmed by the damage, spell, target-picker, movement, and
        // monster-special readers" and confirms none for these rows, so
        // their flag word is all zero. It is **not** absent: a consumer that
        // propagates a missing row instead of reading an all-zero flag word
        // leaves class 1 - the `encounters.md §4` ship/pirate family's combat
        // class - unable to be damaged or to attack.
        1 => Some(traits!(1, "Bard")),
        2 => Some(traits!(2, "Fighter")),
        3 => Some(traits!(3, "Avatar")),
        4 => Some(traits!(4, "Villager")),
        5 => Some(traits!(5, "Merchant")),
        6 => Some(traits!(6, "Jester")),
        7 => Some(traits!(7, "Bard (second row)")),
        8 => Some(traits!(8, "Pirate")),
        9 => Some(traits!(9, "Unnamed reserved")),
        10 => Some(traits!(10, "Child")),
        11 => Some(traits!(11, "Beggar")),
        12 => Some(traits!(12, "Guard")),
        13 => Some(traits!(
            13,
            "Wanderer",
            physical_immune,
            blink,
            teleport_capable,
            turnable_attack,
            vanish_branch
        )),
        14 => Some(traits!(
            14,
            "Blackthorn",
            physical_immune,
            possess,
            teleport_capable,
            turnable_attack,
            vanish_branch
        )),
        15 => Some(traits!(
            15,
            "Lord British",
            physical_immune,
            blink,
            teleport_capable,
            turnable_attack,
            vanish_branch
        )),
        16 => Some(traits!(16, "Sea Horse", incorporeal, turnable_attack)),
        17 => Some(traits!(17, "Squid", incorporeal, poison_status_attack)),
        18 => Some(traits!(18, "Sea Serpent", incorporeal)),
        19 => Some(traits!(19, "Shark", incorporeal)),
        20 => Some(traits!(20, "Giant Rat", poison_status_attack)),
        21 => Some(traits!(21, "Bat", incorporeal)),
        22 => Some(traits!(22, "Giant Spider", poison_status_attack)),
        23 => Some(traits!(
            23,
            "Ghost",
            physical_half,
            incorporeal,
            blink,
            undead
        )),
        24 => Some(traits!(24, "Slime", splits, incorporeal)),
        25 => Some(traits!(25, "Gremlin")),
        26 => Some(traits!(26, "Mimic", zero_selector_stat_row)),
        27 => Some(traits!(
            27,
            "Reaper",
            zero_selector_stat_row,
            turnable_attack
        )),
        28 => Some(traits!(
            28,
            "Gazer",
            special_death,
            possess,
            turnable_attack
        )),
        29 => Some(traits!(29, "Crawler")),
        30 => Some(traits!(
            30,
            "Gargoyle",
            splits,
            zero_selector_stat_row,
            special_death
        )),
        31 => Some(traits!(31, "Insect Swarm", incorporeal)),
        32 => Some(traits!(32, "Orc", zero_selector_stat_row)),
        33 => Some(traits!(33, "Skeleton", physical_half, undead)),
        34 => Some(traits!(34, "Python", poison_status_attack)),
        35 => Some(traits!(35, "Ettin", zero_selector_stat_row)),
        36 => Some(traits!(36, "Headless", zero_selector_stat_row)),
        37 => Some(traits!(37, "Wisp", incorporeal, possess, teleport_capable)),
        38 => Some(traits!(
            38,
            "Daemon",
            physical_half,
            incorporeal,
            possess,
            turnable_attack
        )),
        39 => Some(traits!(39, "Dragon", summon_daemon)),
        40 => Some(traits!(40, "Sand Trap")),
        41 => Some(traits!(41, "Troll")),
        42 => Some(traits!(42, "Reserved gap")),
        43 => Some(traits!(43, "Reserved gap")),
        44 => Some(traits!(44, "Mongbat")),
        45 => Some(traits!(45, "Corpser")),
        46 => Some(traits!(46, "Rot Worm", poison_status_attack)),
        47 => Some(traits!(
            47,
            "Shadow Lord",
            physical_half,
            possess,
            teleport_capable,
            turnable_attack,
            vanish_branch
        )),
        _ => None,
    }
}

pub fn combat_ranged_effect_stats(class: u8) -> Option<CombatRangedEffectStats> {
    match class {
        0 => Some(ranged!(0, "Mage", 7, 4, true, false, false)),
        1 => Some(ranged!(1, "Bard", 3, 0, false, false, false)),
        2 => Some(ranged!(2, "Fighter", 1, 0, false, false, false)),
        3 => Some(ranged!(3, "Avatar", 1, 0, false, false, false)),
        4 => Some(ranged!(4, "Villager", 1, 0, false, false, false)),
        5 => Some(ranged!(5, "Merchant", 1, 0, false, false, false)),
        6 => Some(ranged!(6, "Jester", 1, 0, false, false, false)),
        7 => Some(ranged!(7, "Bard (second row)", 1, 0, false, false, false)),
        8 => Some(ranged!(8, "Pirate", 1, 0, false, false, false)),
        9 => Some(ranged!(9, "Unnamed reserved", 1, 0, false, false, false)),
        10 => Some(ranged!(10, "Child", 1, 0, false, false, false)),
        11 => Some(ranged!(11, "Beggar", 1, 0, false, false, false)),
        12 => Some(ranged!(12, "Guard", 15, 2, false, false, false)),
        13 => Some(ranged!(13, "Wanderer", 9, 4, true, false, false)),
        14 => Some(ranged!(14, "Blackthorn", 9, 3, true, false, false)),
        15 => Some(ranged!(15, "Lord British", 9, 4, true, false, false)),
        16 => Some(ranged!(16, "Sea Horse", 5, 4, true, false, false)),
        17 => Some(ranged!(17, "Squid", 7, 4, false, false, false)),
        18 => Some(ranged!(18, "Sea Serpent", 9, 3, false, false, false)),
        19 => Some(ranged!(19, "Shark", 1, 0, false, false, false)),
        20 => Some(ranged!(20, "Giant Rat", 1, 0, false, false, false)),
        21 => Some(ranged!(21, "Bat", 1, 0, false, false, false)),
        22 => Some(ranged!(22, "Giant Spider", 1, 0, false, false, false)),
        23 => Some(ranged!(23, "Ghost", 1, 0, false, false, false)),
        24 => Some(ranged!(24, "Slime", 1, 0, false, false, false)),
        25 => Some(ranged!(25, "Gremlin", 1, 0, false, true, false)),
        26 => Some(ranged!(26, "Mimic", 2, 5, false, false, true)),
        27 => Some(ranged!(27, "Reaper", 9, 4, true, false, false)),
        28 => Some(ranged!(28, "Gazer", 5, 6, true, false, false)),
        29 => Some(ranged!(29, "Crawler", 1, 0, false, false, false)),
        30 => Some(ranged!(30, "Gargoyle", 9, 7, false, false, false)),
        31 => Some(ranged!(31, "Insect Swarm", 1, 0, false, false, false)),
        32 => Some(ranged!(32, "Orc", 1, 0, false, false, false)),
        33 => Some(ranged!(33, "Skeleton", 1, 0, false, false, false)),
        34 => Some(ranged!(34, "Python", 3, 5, false, false, false)),
        35 => Some(ranged!(35, "Ettin", 5, 7, false, false, false)),
        36 => Some(ranged!(36, "Headless", 1, 0, false, false, false)),
        37 => Some(ranged!(37, "Wisp", 1, 0, false, false, false)),
        38 => Some(ranged!(38, "Daemon", 9, 3, true, false, false)),
        39 => Some(ranged!(39, "Dragon", 9, 3, false, false, false)),
        40 => Some(ranged!(40, "Sand Trap", 1, 0, false, false, false)),
        41 => Some(ranged!(41, "Troll", 5, 2, false, false, false)),
        42 => Some(ranged!(42, "Reserved gap", 1, 0, false, false, false)),
        43 => Some(ranged!(43, "Reserved gap", 1, 0, false, false, false)),
        44 => Some(ranged!(44, "Mongbat", 1, 0, false, false, false)),
        45 => Some(ranged!(45, "Corpser", 1, 0, false, false, false)),
        46 => Some(ranged!(46, "Rot Worm", 1, 0, false, false, false)),
        47 => Some(ranged!(47, "Shadow Lord", 9, 3, true, false, false)),
        _ => None,
    }
}

pub fn combat_class_for_sprite_byte(byte: u8) -> Option<u8> {
    match byte {
        0x70..=0x73 => Some(12),
        0x74..=0x77 => Some(13),
        0x78..=0x7b => Some(14),
        0x7c..=0x7f => Some(15),
        0x80..=0x83 => Some(16),
        0x84..=0x87 => Some(17),
        0x88..=0x8b => Some(18),
        0x8c..=0x8f => Some(19),
        0x90..=0x93 => Some(20),
        0x94..=0x97 => Some(21),
        0x98..=0x9b => Some(22),
        0x9c..=0x9f => Some(23),
        0xa0..=0xa3 => Some(24),
        0xa4..=0xa7 => Some(25),
        0xa8..=0xab => Some(26),
        0xac..=0xaf => Some(27),
        0xb0..=0xb3 => Some(28),
        0xb4..=0xb7 => Some(29),
        0xb8..=0xbb => Some(30),
        0xbc..=0xbf => Some(31),
        0xc0..=0xc3 => Some(32),
        0xc4..=0xc7 => Some(33),
        0xc8..=0xcb => Some(34),
        0xcc..=0xcf => Some(35),
        0xd0..=0xd3 => Some(36),
        0xd4..=0xd7 => Some(37),
        0xd8..=0xdb => Some(38),
        0xdc..=0xdf => Some(39),
        0xe0..=0xe3 => Some(40),
        0xe4..=0xe7 => Some(41),
        0xf0..=0xf3 => Some(44),
        0xf4..=0xf7 => Some(45),
        0xf8..=0xfb => Some(46),
        0xfc..=0xff => Some(47),
        _ => None,
    }
}

pub fn combat_class_stats_for_sprite_byte(byte: u8) -> Option<CombatClassStats> {
    combat_class_for_sprite_byte(byte).and_then(combat_class_stats)
}

pub fn combat_class_traits_for_sprite_byte(byte: u8) -> Option<CombatClassTraits> {
    combat_class_for_sprite_byte(byte).and_then(combat_class_traits)
}

pub fn resolve_physical_damage_for_class(class: u8, damage: u8, magical: bool) -> u8 {
    if magical {
        return damage;
    }
    let Some(traits) = combat_class_traits(class) else {
        return damage;
    };
    if traits.physical_immune {
        0
    } else if traits.physical_half {
        damage / 2
    } else {
        damage
    }
}

/// `encounters.md §5` + `combat.md §5` early-game encounter-size
/// damper. Both rolls draw a uniform integer in `[1, n]`, but the
/// second one draws over **the first roll's result**, so applying it
/// "can only *lower* the count. It is a damper, not a doubler."
/// The three exact-count sentinels `1`, `8` and `16` skip both rolls
/// entirely, and the reroll arm ends with the defensive cap at 26.
pub fn resolve_combat_spawn_count(
    base_count: u8,
    first_roll_seed: u8,
    fortunes_second_roll_seed: Option<u8>,
) -> u8 {
    let count = match base_count {
        0 => 0,
        1 | 8 | 16 => base_count,
        max => {
            let first = 1 + (first_roll_seed % max);
            match fortunes_second_roll_seed {
                // `first` is always at least one, so the second
                // modulus is never zero.
                Some(seed) => 1 + (seed % first),
                None => first,
            }
        }
    };
    count.min(COMBAT_SPAWN_COUNT_CAP)
}

/// `catalogs/monster-bestiary.md §2.1` forty-eight-entry companion
/// class table, indexed by class id. Its values are **class ids**,
/// not tile ids: it is the "and a few of something else showed up"
/// table consumed by the terrain-combat early-spawn substitution
/// (`combat.md §5`). Eighteen classes are their own companion, which
/// makes the substitution a no-op for them.
pub const COMBAT_CLASS_COMPANION: [u8; COMBAT_CLASS_COUNT] = [
    33, 1, 1, 3, 4, 4, 4, 4, 4, 4, 10, 4, 12, 13, 14, 15, 17, 16, 17, 19, 33, 21, 20, 33, 24, 26,
    35, 21, 21, 24, 30, 24, 41, 0, 22, 36, 35, 23, 39, 39, 40, 20, 42, 43, 44, 45, 20, 38,
];

/// `catalogs/monster-bestiary.md §2.1`: the companion class for a
/// base combat class id, or `None` for ids outside the forty-eight
/// row table. The substitution is keyed to the base class and never
/// to the arena index.
pub const fn combat_class_companion(class: u8) -> Option<u8> {
    if (class as usize) < COMBAT_CLASS_COUNT {
        Some(COMBAT_CLASS_COMPANION[class as usize])
    } else {
        None
    }
}

#[cfg(test)]
mod incorporeal_class_flag_tests {
    use super::*;

    /// `combat.md §6.3` Death-marker table, Incorporeal-class row:
    /// "Monster whose class-flag word has the low bit set but **not**
    /// the vanish bit — Sea Horse, Squid, Sea Serpent, Shark, Bat,
    /// Ghost, Slime, Insect Swarm, Wisp, Daemon".
    const PUBLISHED_INCORPOREAL_CLASSES: [u8; 10] = [16, 17, 18, 19, 21, 23, 24, 31, 37, 38];

    #[test]
    fn exactly_the_ten_published_classes_carry_the_incorporeal_bit() {
        for class in 0..COMBAT_CLASS_COUNT as u8 {
            let Some(traits) = combat_class_traits(class) else {
                continue;
            };
            assert_eq!(
                traits.incorporeal,
                PUBLISHED_INCORPOREAL_CLASSES.contains(&class),
                "class {class} ({})",
                traits.name
            );
        }
    }

    #[test]
    fn no_class_carries_both_the_incorporeal_and_the_vanish_bit() {
        // `combat.md §6.3`: the incorporeal row is selected by "the low
        // bit set but **not** the vanish bit", and "inside that pair,
        // vanish wins over incorporeal". The four published vanish
        // classes must therefore stay off the incorporeal roster.
        for class in 0..COMBAT_CLASS_COUNT as u8 {
            let Some(traits) = combat_class_traits(class) else {
                continue;
            };
            assert!(
                !(traits.incorporeal && traits.vanish_branch),
                "class {class} ({}) claims both death bits",
                traits.name
            );
        }
        for vanish_class in [13u8, 14, 15, 47] {
            assert!(combat_class_traits(vanish_class).unwrap().vanish_branch);
        }
    }
}
