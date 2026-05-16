//! Cast/Mix dispatcher gate helpers per `magic.md` §7.

use crate::*;

/// `magic.md §4` per-spell circle index (`1..=8`). Returns `None`
/// for out-of-range spell ids. The circle determines mana cost and
/// minimum caster level: a circle-N spell costs N magic points and
/// requires the caster to be at least level N.
pub const fn spell_circle_for(spell_id: u8) -> Option<u8> {
    if (spell_id as usize) >= SPELL_COUNT {
        return None;
    }
    Some(spell_id / SPELLS_PER_CIRCLE as u8 + 1)
}

/// `magic.md §4`: a circle-N spell costs N magic points.
pub const fn spell_mana_cost(circle: u8) -> u8 {
    circle
}

/// `magic.md §8` In Wis / Locate sextant-style coordinate letter
/// pair. The shared coordinate printer splits a one-byte map
/// coordinate into its high and low nibbles and maps each nibble
/// `0..=15` to letters `A..=P`. Returns `(high_letter, low_letter)`
/// as ASCII bytes; the caller composes the surrounding apostrophe,
/// comma, double-quote, and newline punctuation. Used by Locate,
/// the Sextant U-Use, and any other coordinate-printer caller that
/// shares the same letter convention.
pub const fn sextant_coordinate_letters(coordinate: u8) -> (u8, u8) {
    let high = b'A' + ((coordinate >> 4) & 0x0F);
    let low = b'A' + (coordinate & 0x0F);
    (high, low)
}

/// `magic.md §8` field-placement spell -> energy-field cell byte
/// the placement writes into the dungeon image. `In Flam Grav`
/// (spell 14, Fire Field) writes `0x82`; `In Nox Grav` (spell 15,
/// Poison Field) writes `0x81`; `In Zu Grav` (spell 16, Sleep
/// Field) writes `0x80`. Magic field placement preserves the
/// dungeon visit-marker bit, producing the matching `0x88..=0x8A`
/// variants when the cell already carries that bit; the marker-bit
/// preservation is the caller's responsibility, not this helper's.
/// Returns `None` for any other spell index — including `Dispel
/// Field` (18), which removes a placed field rather than writing
/// one.
pub const fn spell_field_placement_byte(spell_index: usize) -> Option<u8> {
    Some(match spell_index {
        FIRE_FIELD_SPELL_INDEX => 0x82,
        POISON_FIELD_SPELL_INDEX => 0x81,
        SLEEP_FIELD_SPELL_INDEX => 0x80,
        _ => return None,
    })
}

/// `magic.md §8` field-placement spell -> combat field-kind byte
/// the shared arena helper consumes in combat / non-dungeon scenes.
/// This is a separate dispatch table from the dungeon image bytes
/// returned by [`spell_field_placement_byte`]: the dungeon path
/// writes 0x80..0x83 directly into the loaded tile buffer, while
/// the combat path passes the kind byte to the arena helper which
/// then splits placement from per-field contact / application work.
/// Returns `None` for any spell index that is not one of the four
/// field-placement spells (Dispel Field 18 has its own removal
/// helper).
pub const fn spell_combat_field_kind(spell_index: usize) -> Option<u8> {
    Some(match spell_index {
        FIRE_FIELD_SPELL_INDEX => 0x35,
        POISON_FIELD_SPELL_INDEX => 0x33,
        SLEEP_FIELD_SPELL_INDEX => 0x34,
        ENERGY_FIELD_SPELL_INDEX => 0x36,
        _ => return None,
    })
}

/// `magic.md §4`: minimum caster level required for a spell of the
/// supplied circle (level == circle). The level gate accepts the
/// cast and debits mana even when below — only the *effect* fails.
pub const fn spell_min_caster_level(circle: u8) -> u8 {
    circle
}

/// `magic.md §9` per-spell scene allow-mask bits. Each spell carries
/// a four-bit mask; the dispatcher tests the active scene's bit and
/// rejects with `Not here!` when the bit is clear.
pub const SPELL_SCENE_BIT_DUNGEON: u8 = 0x01;
pub const SPELL_SCENE_BIT_COMBAT: u8 = 0x02;
pub const SPELL_SCENE_BIT_INDOOR: u8 = 0x04;
pub const SPELL_SCENE_BIT_OVERWORLD: u8 = 0x08;

/// `magic.md §9` scene-class enum aligned with the allow-mask bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSceneClass {
    /// `0x01` — dungeon scene.
    Dungeon,
    /// `0x02` — combat-class scene (the temporary 0xFF marker).
    Combat,
    /// `0x04` — indoor / town-mode scene.
    Indoor,
    /// `0x08` — overworld / underworld travel mode.
    Overworld,
}

impl SpellSceneClass {
    /// `magic.md §9`: returns the bit position the dispatcher tests
    /// for this scene class against the per-spell allow-mask.
    pub const fn allow_mask_bit(self) -> u8 {
        match self {
            Self::Dungeon => SPELL_SCENE_BIT_DUNGEON,
            Self::Combat => SPELL_SCENE_BIT_COMBAT,
            Self::Indoor => SPELL_SCENE_BIT_INDOOR,
            Self::Overworld => SPELL_SCENE_BIT_OVERWORLD,
        }
    }
}

/// `magic.md §9`: returns `true` when the spell's allow mask permits
/// casting in the supplied scene class. The dispatcher tests this
/// gate after the scene byte has selected the active class.
pub const fn spell_allowed_in_scene(allow_mask: u8, scene: SpellSceneClass) -> bool {
    (allow_mask & scene.allow_mask_bit()) != 0
}

/// `magic.md §8` directed-spell wind family. Sleep, Poison Wind,
/// Death Wind, and Flame Wind share one shared target-walk layer
/// that builds a directed set of arena cells from the caster/target
/// state, then iterates the matching combat actors. The common
/// layer skips empty actors, actors masked by disqualifying status
/// flags, and actors already processed by this same spell pass; it
/// does not run the friend/foe faction lookup, so same-faction
/// actors and the caster are eligible if their cells are in the
/// directed area and they pass the non-faction gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectedWindSpell {
    /// `In Zu` — Sleep (single-target sleep status branch).
    Sleep,
    /// `In Nox Hur` — Poison Wind (poison status with resistance gate).
    PoisonWind,
    /// `In Vas Grav Corp` — Death Wind (instant-kill via decimal 99).
    DeathWind,
    /// `In Flam Hur` — Flame Wind (raw 1..=30 damage roll).
    FlameWind,
}

/// `magic.md §8` directed-target-walk maximum cell count. The
/// shared target-walk layer builds up to twenty-one arena cells
/// from the caster/target state.
pub const DIRECTED_WIND_MAX_CELLS: usize = 21;

impl DirectedWindSpell {
    /// `magic.md §8`: returns `true` when this wind spell credits
    /// returned monster-kill reward units to the caster's
    /// experience word (the two damage winds — Death Wind and
    /// Flame Wind — credit XP; Sleep and Poison Wind do not).
    pub const fn credits_kill_xp(self) -> bool {
        matches!(self, Self::DeathWind | Self::FlameWind)
    }
}

/// `magic.md §8` field-spell kind. The four field-placement spells
/// share one helper that dispatches by this kind in non-dungeon
/// scenes (combat byte tables) and writes a per-spell base byte
/// directly into the live dungeon image in dungeon scenes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldSpellKind {
    /// `In Flam Grav` — Fire Field.
    Fire,
    /// `In Nox Grav` — Poison Field.
    Poison,
    /// `In Zu Grav` — Sleep Field.
    Sleep,
    /// `In Sanct Grav` — Energy Field.
    Energy,
}

impl FieldSpellKind {
    /// `magic.md §8` dungeon base field byte. The dungeon helper
    /// overwrites the live cell with this byte when the cell is the
    /// open passage `0x00`.
    pub const fn dungeon_base_byte(self) -> u8 {
        match self {
            Self::Fire => 0x82,
            Self::Poison => 0x81,
            Self::Sleep => 0x80,
            Self::Energy => 0x83,
        }
    }

    /// `magic.md §8` dungeon visit-marker-preserving field byte.
    /// The dungeon helper writes this variant when the live cell
    /// already carries the visit bit (`0x08`).
    pub const fn dungeon_marker_byte(self) -> u8 {
        self.dungeon_base_byte() | 0x08
    }

    /// `magic.md §8` combat field-kind byte the shared field helper
    /// receives in non-dungeon scenes (a separate kind table from
    /// the dungeon byte mapping).
    pub const fn combat_kind_byte(self) -> u8 {
        match self {
            Self::Fire => 0x35,
            Self::Poison => 0x33,
            Self::Sleep => 0x34,
            Self::Energy => 0x36,
        }
    }
}

/// `magic.md §8`: classify a dungeon field byte (with or without
/// the visit marker bit) into its field-spell kind. Returns `None`
/// for bytes outside the four field families.
pub const fn field_spell_kind_for_dungeon_byte(byte: u8) -> Option<FieldSpellKind> {
    Some(match byte & !0x08 {
        0x82 => FieldSpellKind::Fire,
        0x81 => FieldSpellKind::Poison,
        0x80 => FieldSpellKind::Sleep,
        0x83 => FieldSpellKind::Energy,
        _ => return None,
    })
}

/// `magic.md §8` shared active-effect tag installed by combat
/// buff/debuff spells. The shared helper stores one global
/// (tag, counter) pair the round walker consults; later actions
/// that consume the tag look it up by byte value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveEffectTag {
    /// `'P'` — Protection (`In Sanct`).
    Protection,
    /// `'Q'` — Quickness (`Rel Tym`).
    Quickness,
    /// `'C'` — Mass Charm (`Quas An Wis`).
    MassCharm,
    /// `'N'` — Negate Magic (`In An`).
    NegateMagic,
    /// `'T'` — Negate Time (`An Tym`); installed by the Negate Time
    /// spell and the Negate Time scroll.
    NegateTime,
}

impl ActiveEffectTag {
    /// `magic.md §8`: ASCII byte the engine writes into the global
    /// shared visible tag slot.
    pub const fn ascii_byte(self) -> u8 {
        match self {
            Self::Protection => b'P',
            Self::Quickness => b'Q',
            Self::MassCharm => b'C',
            Self::NegateMagic => b'N',
            Self::NegateTime => b'T',
        }
    }

    /// `magic.md §8`: counter value installed when the spell-side
    /// helper writes the tag from a successful C-Cast (Protection
    /// 20, Quickness 30, Mass Charm 20, Negate Magic 10, Negate Time
    /// has its own scene-aware path with no separate spell-side
    /// constant promoted here).
    pub const fn spell_install_counter(self) -> Option<u8> {
        match self {
            Self::Protection => Some(20),
            Self::Quickness => Some(30),
            Self::MassCharm => Some(20),
            Self::NegateMagic => Some(10),
            Self::NegateTime => None,
        }
    }

    /// `inventory.md §7`: counter value installed when a U-Use
    /// scroll writes the tag. Scrolls use different counters from
    /// the C-Cast spell path - `IS` Protection installs P/100, `AI`
    /// Negate Magic installs N/20, `AT` Negate Time installs T/20
    /// (except in Stonegate and Doom, where the scroll reports no
    /// effect and does not write the tag). Quickness and Mass Charm
    /// do not have shipped scroll variants. The scene gate for
    /// Negate Time remains a caller responsibility; this helper
    /// only encodes the install-counter value.
    pub const fn scroll_install_counter(self) -> Option<u8> {
        match self {
            Self::Protection => Some(100),
            Self::NegateMagic => Some(20),
            Self::NegateTime => Some(20),
            Self::Quickness | Self::MassCharm => None,
        }
    }
}

/// `magic.md §8`: classify the global active-effect tag byte.
/// Returns `None` for byte values that are not one of the five
/// confirmed shared tags.
pub const fn active_effect_tag_for_byte(byte: u8) -> Option<ActiveEffectTag> {
    Some(match byte {
        b'P' => ActiveEffectTag::Protection,
        b'Q' => ActiveEffectTag::Quickness,
        b'C' => ActiveEffectTag::MassCharm,
        b'N' => ActiveEffectTag::NegateMagic,
        b'T' => ActiveEffectTag::NegateTime,
        _ => return None,
    })
}

/// `magic.md §5` C-Cast spell-name selector cap. The compact
/// letter-coded form accepts at most four selector letters before
/// the parser auto-completes; longer typed input is truncated to
/// this cap.
pub const SPELL_SELECTOR_MAX_LEN: usize = 4;

/// `magic.md §5` selector letters that the C-Cast prompt rejects
/// outright (no rune syllable is keyed by them). Pressing one of
/// these is a no-op, not a typed-then-rejected character.
pub const SPELL_SELECTOR_IGNORED_LETTERS: &[u8] = b"JO";

/// `magic.md §5`: returns `true` when the supplied selector letter
/// is silently ignored by the prompt (not stored in the buffer).
/// Match is case-insensitive.
pub const fn spell_selector_is_ignored(letter: u8) -> bool {
    matches!(letter, b'J' | b'j' | b'O' | b'o')
}

/// `magic.md §7` combat interference-gate predicate. The gate runs
/// *before* the C-Cast prompt inside a combat round and blocks the
/// cast (printing `<target> interferes!`) only when ALL five
/// conditions hold:
///
/// 1. The per-slot combat target map contains a non-sentinel target.
/// 2. The target slot holds a valid live actor.
/// 3. The target is visible/awake (not hidden, not sleeping).
/// 4. The Negate-Time `T` runtime tag is NOT active (it suppresses
///    interference).
/// 5. The caster and target are at distance one in the 11x11 arena.
///
/// Any failing condition allows the cast to continue to the
/// dispatcher.
pub const fn combat_interference_blocks(
    target_mapped: bool,
    target_valid: bool,
    target_visible_and_awake: bool,
    negate_time_active: bool,
    caster_target_distance: u8,
) -> bool {
    if !target_mapped {
        return false;
    }
    if !target_valid {
        return false;
    }
    if !target_visible_and_awake {
        return false;
    }
    if negate_time_active {
        return false;
    }
    caster_target_distance == 1
}

/// `magic.md §7` four dispatcher gate outcomes for the C-Cast pipeline.
/// Each variant names the player-visible message; the comments document
/// the resource-debit asymmetry (spec calls it "intended message is the
/// same; underlying penalties differ").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastGateOutcome {
    /// All four gates passed; the spell handler runs. Caller still
    /// applies the per-spell scene-specific narration and effect.
    Cast,
    /// Scene-gate refusal — `Not here!`. No charge spent.
    NotHere,
    /// Charges-gate refusal — `None mixed!`. No charge spent.
    NoneMixed,
    /// Mana-gate refusal — `M.P. too low!`. The charge has already been
    /// consumed; mana is *not* spent.
    ManaTooLowChargeOnly,
    /// Level-gate refusal — `M.P. too low!`. The charge AND the mana have
    /// both been consumed.
    LevelTooLowChargeAndMana,
}

impl CastGateOutcome {
    /// Whether this outcome consumed the per-spell charge counter.
    pub const fn consumed_charge(self) -> bool {
        matches!(
            self,
            CastGateOutcome::Cast
                | CastGateOutcome::ManaTooLowChargeOnly
                | CastGateOutcome::LevelTooLowChargeAndMana
        )
    }

    /// Whether this outcome consumed the caster's mana.
    pub const fn consumed_mana(self) -> bool {
        matches!(
            self,
            CastGateOutcome::Cast | CastGateOutcome::LevelTooLowChargeAndMana
        )
    }

    /// Player-visible message string; multiple outcomes share the
    /// `M.P. too low!` text per the spec.
    pub const fn message(self) -> &'static str {
        match self {
            CastGateOutcome::Cast => "",
            CastGateOutcome::NotHere => "Not here!",
            CastGateOutcome::NoneMixed => "None mixed!",
            CastGateOutcome::ManaTooLowChargeOnly | CastGateOutcome::LevelTooLowChargeAndMana => {
                "M.P. too low!"
            }
        }
    }
}

/// `magic.md §7`: run the four dispatcher gates in order. Returns the
/// outcome the dispatcher reports back to the player. Caller passes:
///   - `scene_allowed`: per-spell scene-allow mask matched the current
///     scene byte (the scene gate's only input);
///   - `charges`: pre-decrement per-spell charge counter;
///   - `mana`: caster's MP before this cast;
///   - `level`: caster's level;
///   - `circle`: spell circle index in `0..=7` (mana cost and level
///     requirement).
pub const fn cast_dispatcher_gate(
    scene_allowed: bool,
    charges: u8,
    mana: u8,
    level: u8,
    circle: u8,
) -> CastGateOutcome {
    if !scene_allowed {
        return CastGateOutcome::NotHere;
    }
    if charges == 0 {
        return CastGateOutcome::NoneMixed;
    }
    // Charges decrement here in the original; the spec only cares that
    // the message "M.P. too low!" can be reached with charge already
    // consumed.
    if mana < circle {
        return CastGateOutcome::ManaTooLowChargeOnly;
    }
    if level < circle {
        return CastGateOutcome::LevelTooLowChargeAndMana;
    }
    CastGateOutcome::Cast
}

/// `catalogs/spell-list.md §4` indoor short-circuits before the
/// scene-mask comparison: Lord Blackthorn's Castle absorbs casts while
/// the Crown of Lord British ownership flag is clear, and Stonegate
/// absorbs casts unconditionally. Returns `true` when the dispatcher
/// should print "Absorbed!" and abort before consuming a charge or
/// mana.
pub const fn spell_indoor_absorbs(scene_blackthorn_castle: bool, has_crown: bool, scene_stonegate: bool) -> bool {
    if scene_stonegate {
        return true;
    }
    scene_blackthorn_castle && !has_crown
}

/// `magic.md §8` Conjure spell weighted-summon class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConjureSummon {
    /// Six of fifteen outcomes — Giant Rat.
    GiantRat,
    /// Five of fifteen outcomes — Giant Spider.
    GiantSpider,
    /// Three of fifteen outcomes — Bat.
    Bat,
    /// One of fifteen outcomes — Python.
    Python,
}

/// `magic.md §8` Conjure outcome bound — fifteen weighted outcomes.
pub const CONJURE_OUTCOME_COUNT: u8 = 15;

/// `magic.md §8`: classify the Conjure-roll outcome. Caller passes
/// the raw `0..=14` roll the spell makes against its fifteen-row
/// weighted table. Returns `None` for any roll outside the bound.
pub const fn conjure_summon_for_roll(roll: u8) -> Option<ConjureSummon> {
    Some(match roll {
        0..=5 => ConjureSummon::GiantRat,
        6..=10 => ConjureSummon::GiantSpider,
        11..=13 => ConjureSummon::Bat,
        14 => ConjureSummon::Python,
        _ => return None,
    })
}

/// `magic.md §6,§7` per-spell charge-counter add. After M-Mix's
/// recipe-match step the requested quantity is added to the
/// per-spell charge counter, capped at `SPELL_CHARGE_CAP` (99).
/// Returns the new counter value clamped to the cap. Caller has
/// already validated the recipe match and debited reagents.
pub const fn spell_charge_add_capped(current: u8, qty: u8) -> u8 {
    let sum = current.saturating_add(qty);
    if sum > SPELL_CHARGE_CAP { SPELL_CHARGE_CAP } else { sum }
}

/// `magic.md §3` canonical Britannian magic-rune syllable vocabulary.
/// The twenty-four entries are returned in the spec's table order.
pub const RUNE_SYLLABLE_VOCABULARY: [&str; 24] = [
    "An", "Bet", "Corp", "Des", "Ex", "Flam", "Grav", "Hur", "In", "Kal", "Lor", "Mani",
    "Nox", "Por", "Quas", "Rel", "Sanct", "Tym", "Uus", "Vas", "Wis", "Xen", "Ylem", "Zu",
];

/// `magic.md §3`: predicate accepting one of the twenty-four resident
/// rune syllables. Comparison is case-insensitive ASCII; the older
/// Ultima lore syllables `Jux` and `Ort` are deliberately rejected.
pub fn is_resident_rune_syllable(token: &str) -> bool {
    RUNE_SYLLABLE_VOCABULARY
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(token))
}

/// `magic.md §4` canonical common-name for one spell index `0..=47`.
/// Returns `None` for out-of-range indices. Matches the spec's circle
/// table verbatim and is used to render "<spell>" strings in the
/// player-facing dispatcher and in unit tests.
pub const fn spell_common_name(index: usize) -> Option<&'static str> {
    Some(match index {
        // Circle 1
        0 => "Light",
        1 => "Magic Missile",
        2 => "Awaken",
        3 => "Cure",
        4 => "Heal",
        5 => "Vanish",
        // Circle 2
        6 => "Open",
        7 => "Repel Undead",
        8 => "Wind Change",
        9 => "Locate",
        10 => "Conjure",
        11 => "Create Food",
        // Circle 3
        12 => "Great Light",
        13 => "Fireball",
        14 => "Fire Field",
        15 => "Poison Field",
        16 => "Sleep Field",
        17 => "Blink",
        // Circle 4
        18 => "Dispel Field",
        19 => "Protection",
        20 => "Energy Field",
        21 => "Up",
        22 => "Down",
        23 => "Reveal",
        // Circle 5
        24 => "Swarm",
        25 => "Magic Lock",
        26 => "Unlock Magic",
        27 => "Great Heal",
        28 => "Sleep",
        29 => "Quickness",
        // Circle 6
        30 => "Tremor",
        31 => "Mass Charm",
        32 => "Negate Magic",
        33 => "X-Ray",
        34 => "Charm",
        35 => "Polymorph",
        // Circle 7
        36 => "Invisibility",
        37 => "Kill",
        38 => "Clone",
        39 => "Peer",
        40 => "Poison Wind",
        41 => "Cause Fear",
        // Circle 8
        42 => "Resurrect",
        43 => "Summon",
        44 => "Death Wind",
        45 => "Flame Wind",
        46 => "Gate Travel",
        47 => "Negate Time",
        _ => return None,
    })
}

/// `magic.md §8` Heal: random `0..=60` roll, halved (integer
/// truncation), with a zero result promoted to one. Returns the heal
/// amount in inclusive range `1..=30`.
pub const fn heal_spell_amount_from_raw_roll_u8(raw_roll_0_to_60: u8) -> u8 {
    let halved = raw_roll_0_to_60 / 2;
    if halved == 0 {
        1
    } else {
        halved
    }
}
