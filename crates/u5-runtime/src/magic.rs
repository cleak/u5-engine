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
