//! Eight-virtue shrine system: parsing, indexing, mantras.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShrineVirtue {
    Honesty,
    Compassion,
    Valor,
    Justice,
    Sacrifice,
    Honor,
    Spirituality,
    Humility,
}

impl ShrineVirtue {
    pub fn from_key(value: &str) -> Option<Self> {
        let value = match value.split_once(':') {
            Some((prefix, suffix)) if prefix.eq_ignore_ascii_case("SHRINE") => suffix,
            _ => value,
        };
        match value.to_ascii_uppercase().as_str() {
            "HONESTY" => Some(Self::Honesty),
            "COMPASSION" => Some(Self::Compassion),
            "VALOR" | "VALOUR" => Some(Self::Valor),
            "JUSTICE" => Some(Self::Justice),
            "SACRIFICE" => Some(Self::Sacrifice),
            "HONOR" | "HONOUR" => Some(Self::Honor),
            "SPIRITUALITY" => Some(Self::Spirituality),
            "HUMILITY" => Some(Self::Humility),
            _ => None,
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        Some(match index {
            0 => Self::Honesty,
            1 => Self::Compassion,
            2 => Self::Valor,
            3 => Self::Justice,
            4 => Self::Sacrifice,
            5 => Self::Honor,
            6 => Self::Spirituality,
            7 => Self::Humility,
            _ => return None,
        })
    }

    pub fn index(self) -> usize {
        match self {
            Self::Honesty => 0,
            Self::Compassion => 1,
            Self::Valor => 2,
            Self::Justice => 3,
            Self::Sacrifice => 4,
            Self::Honor => 5,
            Self::Spirituality => 6,
            Self::Humility => 7,
        }
    }

    pub fn bit(self) -> u8 {
        1 << self.index()
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Honesty => "Honesty",
            Self::Compassion => "Compassion",
            Self::Valor => "Valor",
            Self::Justice => "Justice",
            Self::Sacrifice => "Sacrifice",
            Self::Honor => "Honor",
            Self::Spirituality => "Spirituality",
            Self::Humility => "Humility",
        }
    }

    pub fn mantra(self) -> &'static str {
        match self {
            Self::Honesty => "Ahm",
            Self::Compassion => "Mu",
            Self::Valor => "Ra",
            Self::Justice => "Beh",
            Self::Sacrifice => "Cah",
            Self::Honor => "Summ",
            Self::Spirituality => "Om",
            Self::Humility => "Lum",
        }
    }

    /// Iteration order used by `karma.md §8` (the standard virtue
    /// order). The array length is anchored to
    /// [`crate::VIRTUE_COUNT`] so the catalog size and the
    /// iteration array stay one value.
    pub const ALL: [Self; crate::VIRTUE_COUNT] = [
        Self::Honesty,
        Self::Compassion,
        Self::Valor,
        Self::Justice,
        Self::Sacrifice,
        Self::Honor,
        Self::Spirituality,
        Self::Humility,
    ];

    /// `karma.md §10` per-virtue shrine quest state derived from the
    /// (ordained, codex-read) bit pair stored in the two save-backed
    /// shrine masks.
    pub const fn shrine_quest_state(ordained: bool, codex_read: bool) -> ShrineQuestState {
        match (ordained, codex_read) {
            (false, false) => ShrineQuestState::NotStarted,
            (true, false) => ShrineQuestState::Ordained,
            (true, true) => ShrineQuestState::CodexRead,
            (false, true) => ShrineQuestState::Complete,
        }
    }

    /// `karma.md §4`: Codex shrine turn-in raises the shared moral-
    /// standing selector by this many units. Humility receives this
    /// increase again as a follow-up bonus; other virtues receive
    /// only the base increase. The same +3 is also added to the
    /// matching per-virtue shrine standing slot.
    pub const SHRINE_CODEX_TURN_IN_MORAL_INCREASE: u8 = 3;

    /// `karma.md §7` Codex turn-in Avatar stat reward. Each touched stat
    /// increments by one and clamps at thirty (the avatar stat cap).
    /// Returns `(strength_step, dexterity_step, intelligence_step)` —
    /// each entry is `1` if the virtue rewards that stat or `0`
    /// otherwise.
    pub const fn codex_turn_in_stat_steps(self) -> (u8, u8, u8) {
        match self {
            Self::Honesty => (0, 0, 1),
            Self::Compassion => (0, 1, 0),
            Self::Valor => (1, 0, 0),
            Self::Justice => (0, 1, 1),
            Self::Sacrifice => (1, 1, 0),
            Self::Honor => (1, 0, 1),
            Self::Spirituality => (1, 1, 1),
            Self::Humility => (0, 0, 0),
        }
    }

    /// `karma.md §7` completed-shrine offering gold cost. For a digit
    /// `1..=9` the prompt deducts `digit * 100` gold and adds `digit`
    /// to the shared moral-standing selector. Digit `0` is the
    /// no-effect exit and returns `None` (caller should leave the
    /// prompt without prompting again or charging gold).
    pub const fn shrine_offering_cost(digit: u8) -> Option<u16> {
        if digit == 0 || digit > 9 {
            return None;
        }
        Some(digit as u16 * 100)
    }

    /// `karma.md §7`: Humility's Codex turn-in adds an additional `+3`
    /// to the shared moral-standing selector after the stat step. All
    /// other virtues add only the base `+3`.
    pub const fn codex_turn_in_humility_bonus(self) -> u8 {
        match self {
            Self::Humility => 3,
            _ => 0,
        }
    }

    /// `karma.md §9`: traditional virtue-to-companion pairing. The avatar's
    /// own class is always Avatar regardless of the winning virtue per
    /// `chargen.md`; this pairing only describes companion roster slots.
    pub const fn companion(self) -> (&'static str, &'static str) {
        match self {
            Self::Honesty => ("Mariah", "Mage"),
            Self::Compassion => ("Iolo", "Bard"),
            Self::Valor => ("Geoffrey", "Fighter"),
            Self::Justice => ("Jaana", "Druid"),
            Self::Sacrifice => ("Julia", "Tinker"),
            Self::Honor => ("Dupre", "Paladin"),
            Self::Spirituality => ("Shamino", "Ranger"),
            Self::Humility => ("Katrina", "Shepherd"),
        }
    }
}

/// `karma.md §10` per-virtue shrine quest state encoded by the
/// (ordained, codex-read) bit pair in the two shrine masks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShrineQuestState {
    /// Both bits clear — virtue's quest path has not begun.
    NotStarted,
    /// Ordained bit set — must visit the Codex.
    Ordained,
    /// Both bits set — Codex page read; must return to the shrine.
    CodexRead,
    /// Codex bit set, ordained bit clear — quest complete.
    Complete,
}

/// `karma.md §10`: the player's "all virtues complete" terminal state
/// has every codex-read bit set and every ordained bit clear. Both
/// inputs are eight-bit masks with one bit per virtue index `0..=7`.
pub const fn all_virtues_complete(ordained_mask: u8, codex_read_mask: u8) -> bool {
    ordained_mask == 0 && codex_read_mask == 0xFF
}

/// `karma.md §7` mantra-matching outcome for one shrine meditation
/// attempt. A correct mantra dispatches by the virtue's current
/// [`ShrineQuestState`]; a wrong or blank mantra prints the no-effect
/// meditation branch and does nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShrineMeditationOutcome {
    /// Wrong or blank mantra — no shrine-side state change.
    NoEffect,
    /// First-time shrine visit (NotStarted). Sets the virtue's
    /// ordained bit; no gold prompt, stat increase, or standing
    /// change is applied.
    Ordain,
    /// Already ordained but Codex not yet read. Leaves the ordained
    /// bit set and produces no further change.
    AlreadyOrdained,
    /// Codex-read turn-in. Clears the ordained bit, applies the
    /// +3 standing reward (and the extra +3 for Humility), and
    /// stamps the per-virtue Avatar stat rewards listed in the
    /// catalog table.
    CodexTurnIn,
    /// Quest complete for this virtue — runs the ordinary gold
    /// offering path. The caller then prompts for a digit `1..=9`
    /// (zero exits) and applies `digit * 100` gold for `+digit`
    /// standing if the party can afford it.
    GoldOffering,
}

/// `karma.md §7`: classify the shrine handler's mantra-matched
/// branch from the (ordained, codex_read) bit pair plus a
/// `mantra_matches` predicate. Returns [`NoEffect`] for a wrong or
/// blank mantra without consulting the bits.
pub const fn shrine_meditation_outcome(
    mantra_matches: bool,
    ordained: bool,
    codex_read: bool,
) -> ShrineMeditationOutcome {
    if !mantra_matches {
        return ShrineMeditationOutcome::NoEffect;
    }
    match ShrineVirtue::shrine_quest_state(ordained, codex_read) {
        ShrineQuestState::NotStarted => ShrineMeditationOutcome::Ordain,
        ShrineQuestState::Ordained => ShrineMeditationOutcome::AlreadyOrdained,
        ShrineQuestState::CodexRead => ShrineMeditationOutcome::CodexTurnIn,
        ShrineQuestState::Complete => ShrineMeditationOutcome::GoldOffering,
    }
}

/// Result of one Codex urn read per `karma.md §8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexUrnReadOutcome {
    /// All eight Codex-read bits are set; the reader takes its completed
    /// branch and the saved masks are unchanged.
    Completed,
    /// No virtue currently has its ordained bit set; nothing happens.
    NoOrdained,
    /// The first ordained, not-yet-Codex-read virtue had its bit stamped.
    Stamped(ShrineVirtue),
}

/// `karma.md §8`: walk virtues in the standard order, pick the first virtue
/// whose ordained bit is set and Codex-read bit is not set, set the matching
/// Codex-read bit, and report the chosen virtue. If all eight Codex-read
/// bits are already set, take the completed branch instead. If no virtue is
/// ordained, do nothing. Updates `*codex_mask` in place.
pub fn read_codex_urn(ordained_mask: u8, codex_mask: &mut u8) -> CodexUrnReadOutcome {
    if *codex_mask == 0xFF {
        return CodexUrnReadOutcome::Completed;
    }
    for virtue in ShrineVirtue::ALL {
        let bit = virtue.bit();
        if ordained_mask & bit != 0 && *codex_mask & bit == 0 {
            *codex_mask |= bit;
            return CodexUrnReadOutcome::Stamped(virtue);
        }
    }
    CodexUrnReadOutcome::NoOrdained
}
