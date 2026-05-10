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
}
