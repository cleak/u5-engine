//! Timing status tag, save-template source, dungeon field effect.

use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimingStatusTag {
    #[default]
    Normal,
    HalfTime,
    NoMinuteLight,
    Opaque(u8),
}

impl TimingStatusTag {
    pub fn from_save_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Normal,
            b'Q' => Self::HalfTime,
            b'T' => Self::NoMinuteLight,
            other => Self::Opaque(other),
        }
    }

    pub fn for_transport(transport: TransportState) -> Self {
        if matches!(transport, TransportState::Skiff { .. }) {
            Self::HalfTime
        } else {
            Self::Normal
        }
    }

    pub fn effective_minutes(self, base: u8) -> u8 {
        match self {
            Self::Normal => base,
            Self::HalfTime if base == 0 => 0,
            Self::HalfTime => (base / 2).max(1),
            Self::NoMinuteLight => 0,
            Self::Opaque(_) => base,
        }
    }

    pub fn save_byte(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::HalfTime => b'Q',
            Self::NoMinuteLight => b'T',
            Self::Opaque(byte) => byte,
        }
    }

    pub fn status_label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::HalfTime => "half-time",
            Self::NoMinuteLight => "no-minute-light",
            Self::Opaque(_) => "opaque",
        }
    }

    /// `time.md §4` / `overworld.md §6`: the saved `Q` tag lets the
    /// overworld active-object and encounter epilogue run on alternate
    /// turns, while `T` returns before that epilogue.
    pub const fn world_object_epilogue_runs(self, turn_before: u64) -> bool {
        match self {
            Self::Normal => true,
            Self::HalfTime => turn_before % 2 == 1,
            Self::NoMinuteLight => false,
            Self::Opaque(_) => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveTemplateSource {
    PreferSavedGame,
    SavedGame,
    InitGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonFieldEffect {
    Sleep,
    PoisonGas,
    Fire,
    Electric,
    Energy,
}

impl DungeonFieldEffect {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sleep => "sleep field",
            Self::PoisonGas => "poison gas field",
            Self::Fire => "wall of fire",
            Self::Electric => "electric field",
            Self::Energy => "energy field",
        }
    }

    pub fn status(self) -> Option<u8> {
        match self {
            Self::Sleep => Some(b'S'),
            Self::PoisonGas => Some(b'P'),
            Self::Fire | Self::Electric | Self::Energy => None,
        }
    }

    pub fn is_damage_field(self) -> bool {
        matches!(self, Self::Fire | Self::Electric)
    }

    pub fn damage_seed_bias(self) -> u8 {
        match self {
            Self::Fire => 19,
            Self::Electric => 29,
            Self::Sleep | Self::PoisonGas | Self::Energy => 0,
        }
    }
}
