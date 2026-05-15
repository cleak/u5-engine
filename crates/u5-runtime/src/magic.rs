//! Cast/Mix dispatcher gate helpers per `magic.md` §7.

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
