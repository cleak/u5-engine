//! Active-effect timing projection, save-template source, dungeon field effect.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimingStatusTag {
    #[default]
    Normal,
    HalfTime,
    NoMinuteLight,
    Opaque(u8),
}

impl TimingStatusTag {
    pub const fn from_save_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Normal,
            b'Q' => Self::HalfTime,
            b'T' => Self::NoMinuteLight,
            other => Self::Opaque(other),
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

    /// `time.md §4` / `overworld.md §6`: active effect `Q` lets the
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

/// `npc-schedules.md §5` / `encounters.md §2.1`: which of the three per-turn
/// effect gates stopped a mode's walkers, if any.
///
/// Both modes test the same three gates, but in opposite orders - town tests
/// "**Transport marker.** ... **Negate Time.** ... **Quickness.**" while the
/// outdoor block tests "**Negate Time.** ... **Quickness.** ... **The
/// transport marker.**" - and "the order is behaviourally load-bearing,
/// because a gate that returns early leaves the later gates' parity bits
/// un-flipped: in town the transport parity advances even on a turn that
/// Negate Time or Quickness then suppresses, while outdoors it does not. The
/// two modes' alternate-turn phases therefore drift apart, and an engine that
/// shares one gate routine between them will eventually put NPC and creature
/// movement on the wrong turns." The two orders therefore get two routines
/// here, not one with a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkerEffectGate {
    /// No gate fired: this turn's walkers run.
    Run,
    /// `npc-schedules.md §5`: the party's transport marker is inside the
    /// four-value window and the stored parity bit came up set.
    SkippedByTransportMarker,
    /// Negate Time is active. "While that timed effect is active, the loop
    /// skips both town walkers outright. Nothing in town moves."
    SkippedByNegateTime,
    /// Quickness is active and its stored parity bit came up set.
    SkippedByQuickness,
}

impl WalkerEffectGate {
    /// `RETRACTIONS.md` R328: each of the three gates "can skip the schedule
    /// processor *and* the town object walker for the turn", so one answer
    /// covers both walkers. `npc-schedules.md §5` says the same in its own
    /// words - the gates "sit in the town loop's per-turn epilogue, ahead of
    /// both town walkers - the object walker that moves loose horse-family
    /// objects and this schedule processor".
    pub const fn walkers_run(self) -> bool {
        matches!(self, Self::Run)
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
