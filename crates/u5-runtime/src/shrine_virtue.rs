//! Eight-virtue shrine system: parsing, indexing, mantras.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Iteration order used by `karma.md §8` (the standard virtue order).
    pub const ALL: [Self; 8] = [
        Self::Honesty,
        Self::Compassion,
        Self::Valor,
        Self::Justice,
        Self::Sacrifice,
        Self::Honor,
        Self::Spirituality,
        Self::Humility,
    ];
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
