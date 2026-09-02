//! Wind state: direction (or calm), parsing, and Rel Hur targets.

use std::io;

use crate::Direction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindState {
    #[default]
    Calm,
    North,
    South,
    East,
    West,
}

impl WindState {
    pub fn from_key(value: &str) -> io::Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "calm" | "none" | "0" => Ok(Self::Calm),
            "north" | "n" => Ok(Self::North),
            "south" | "s" => Ok(Self::South),
            "east" | "e" => Ok(Self::East),
            "west" | "w" => Ok(Self::West),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("wind must be calm, north, south, east, or west, got `{value}`"),
            )),
        }
    }

    pub fn from_save_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Calm,
            1 => Self::North,
            2 => Self::South,
            3 => Self::East,
            4 => Self::West,
            _ => Self::Calm,
        }
    }

    pub fn save_byte(self) -> u8 {
        match self {
            Self::Calm => 0,
            Self::North => 1,
            Self::South => 2,
            Self::East => 3,
            Self::West => 4,
        }
    }

    pub fn direction(self) -> Option<Direction> {
        match self {
            Self::Calm => None,
            Self::North => Some(Direction::North),
            Self::South => Some(Direction::South),
            Self::East => Some(Direction::East),
            Self::West => Some(Direction::West),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Calm => "Calm",
            Self::North => "North",
            Self::South => "South",
            Self::East => "East",
            Self::West => "West",
        }
    }

    pub const fn status_message(self) -> &'static str {
        match self {
            Self::Calm => "Calm Winds",
            Self::North => "North Winds",
            Self::South => "South Winds",
            Self::East => "East Winds",
            Self::West => "West Winds",
        }
    }

    /// `weather.md §5`: number of wait ticks the input helper inserts
    /// before the hoisted-sail player ship is released to move in
    /// `sail_heading` under this wind. Returns `None` for Calm wind
    /// (movement never released) and for non-cardinal headings.
    pub fn player_sail_wait_ticks(self, sail_heading: Direction) -> Option<u8> {
        let wind = self.direction()?;
        let heading = match sail_heading {
            Direction::North | Direction::South | Direction::East | Direction::West => sail_heading,
            _ => return None,
        };
        let perpendicular = matches!(
            (wind, heading),
            (Direction::North, Direction::East)
                | (Direction::North, Direction::West)
                | (Direction::South, Direction::East)
                | (Direction::South, Direction::West)
                | (Direction::East, Direction::North)
                | (Direction::East, Direction::South)
                | (Direction::West, Direction::North)
                | (Direction::West, Direction::South)
        );
        if perpendicular {
            return Some(PLAYER_SAIL_WAIT_TICKS_PERPENDICULAR);
        }
        if wind == heading {
            // Sailing into the wind.
            Some(PLAYER_SAIL_WAIT_TICKS_INTO_WIND)
        } else {
            // Sailing with the wind (heading == opposite of wind source).
            Some(PLAYER_SAIL_WAIT_TICKS_WITH_WIND)
        }
    }

    /// `weather.md §5` per-wait-pass cleanup minute increment for a
    /// hoisted-sail player ship that is still waiting for the wind
    /// release. Without the HMS Cape rigging flag the wait pass uses
    /// the ordinary two-minute outdoor cleanup increment. With the
    /// rigging flag active the wait pass uses a one-minute increment
    /// and alternates the active-object epilogue.
    pub const fn sailing_wait_pass_minutes(rigging_active: bool) -> u8 {
        if rigging_active {
            crate::MINUTES_PER_INDOOR_TURN
        } else {
            crate::MINUTES_PER_OUTDOOR_TURN
        }
    }

    /// `weather.md §7`: cadence cap for an autonomous active-ship slot
    /// under prevailing wind, given the slot's current frame heading.
    /// Returns `(numerator, denominator)` where the slot moves on
    /// `numerator` of every `denominator` eligible cleanup passes. Returns
    /// `None` for Calm wind (post-animate movement suppressed).
    pub fn active_ship_cadence(self, frame_heading: Direction) -> Option<(u8, u8)> {
        let wind = self.direction()?;
        let heading = match frame_heading {
            Direction::North | Direction::South | Direction::East | Direction::West => {
                frame_heading
            }
            _ => return None,
        };
        let perpendicular = matches!(
            (wind, heading),
            (Direction::North, Direction::East)
                | (Direction::North, Direction::West)
                | (Direction::South, Direction::East)
                | (Direction::South, Direction::West)
                | (Direction::East, Direction::North)
                | (Direction::East, Direction::South)
                | (Direction::West, Direction::North)
                | (Direction::West, Direction::South)
        );
        if perpendicular {
            // "Every turn" — bypasses the counter entirely.
            return Some(ACTIVE_SHIP_CADENCE_EVERY_TURN);
        }
        if wind == heading {
            // Frame faces directly into wind — 2 of 3 turns.
            Some(ACTIVE_SHIP_CADENCE_INTO_WIND)
        } else {
            // Frame faces with wind (away from source) — 3 of 4 turns.
            Some(ACTIVE_SHIP_CADENCE_WITH_WIND)
        }
    }

    /// `weather.md §2` autonomous wind-drift acceptance: only the
    /// zero outer roll over `0..=63` advances to candidate selection.
    pub const fn autonomous_drift_outer_accepted(outer_roll: u8) -> bool {
        (outer_roll & WIND_DRIFT_OUTER_ROLL_MASK) == 0
    }

    /// `weather.md §2` autonomous wind-drift candidate gate. Cardinal
    /// candidates `1..=4` are accepted immediately; candidate `0`
    /// (Calm) is accepted only when the follow-up roll over `0..=255`
    /// is at least `192` (so Calm is much rarer than any cardinal).
    /// Returns the accepted state, or `None` to repeat the candidate
    /// selection.
    pub const fn autonomous_drift_accept_candidate(
        candidate: u8,
        calm_followup_roll: u8,
    ) -> Option<Self> {
        match candidate {
            0 => {
                if calm_followup_roll >= WIND_DRIFT_CALM_ACCEPT_MIN {
                    Some(Self::Calm)
                } else {
                    None
                }
            }
            1 => Some(Self::North),
            2 => Some(Self::South),
            3 => Some(Self::East),
            4 => Some(Self::West),
            _ => None,
        }
    }

    pub fn rel_hur_target(direction: Direction) -> Option<Self> {
        match direction {
            Direction::North => Some(Self::West),
            Direction::East => Some(Self::East),
            Direction::South => Some(Self::South),
            Direction::West => Some(Self::North),
            _ => None,
        }
    }
}

/// `weather.md §2`: surface wind presentation for the saved/runtime
/// wind byte. Public values print a direction label plus the shared
/// suffix; preserved out-of-range values print only the suffix.
pub const fn wind_status_message_from_save_byte(byte: u8) -> &'static str {
    match byte {
        0 => "Calm Winds",
        1 => "North Winds",
        2 => "South Winds",
        3 => "East Winds",
        4 => "West Winds",
        _ => "Winds",
    }
}

/// `weather.md §2`: surface wind presentation for live state plus the
/// preserved save byte. Public in-memory states use the semantic wind
/// value; an out-of-range saved byte keeps the corrupted-state banner
/// visibly incomplete until a setter writes a public value.
pub const fn wind_status_message_from_state_and_save_byte(
    wind: WindState,
    byte: u8,
) -> &'static str {
    if byte <= 4 {
        wind.status_message()
    } else {
        wind_status_message_from_save_byte(byte)
    }
}

/// `weather.md §5` published hoisted-sail player-ship wait-tick
/// counts. The release-table cells classify into three cases:
/// perpendicular wind/heading releases immediately (zero waits);
/// "with the wind" (heading is the opposite of the wind source)
/// releases after one wait; "into the wind" (heading matches the
/// wind source) releases after two waits.
pub const PLAYER_SAIL_WAIT_TICKS_PERPENDICULAR: u8 = 0;
pub const PLAYER_SAIL_WAIT_TICKS_WITH_WIND: u8 = 1;
pub const PLAYER_SAIL_WAIT_TICKS_INTO_WIND: u8 = 2;

/// `weather.md §7` per-frame active-ship cadence caps. Each pair is
/// `(numerator, denominator)`: the slot moves on `numerator` of every
/// `denominator` eligible cleanup passes. The "every turn" case is
/// `(1, 1)` and bypasses the counter; the spec uses two non-trivial
/// cadence ratios — "into the wind" (frame faces directly into the
/// wind source) and "with the wind" (frame faces away from the wind
/// source).
pub const ACTIVE_SHIP_CADENCE_EVERY_TURN: (u8, u8) = (1, 1);
pub const ACTIVE_SHIP_CADENCE_INTO_WIND: (u8, u8) = (2, 3);
pub const ACTIVE_SHIP_CADENCE_WITH_WIND: (u8, u8) = (3, 4);

/// `weather.md §2` autonomous wind-drift outer-roll mask. The selector
/// rolls in `0..=63`; only the zero roll advances. Masking the low six
/// bits of an arbitrary `u8` roll preserves this `0..=63` window without
/// requiring callers to range-check the raw byte.
pub const WIND_DRIFT_OUTER_ROLL_MASK: u8 = 0x3F;

/// `weather.md §2`: the inclusive high end of the outer roll's `0..63`
/// window, for callers drawing it straight off the gameplay PRNG rather
/// than masking an arbitrary byte.
pub const WIND_DRIFT_OUTER_ROLL_MAX: u8 = WIND_DRIFT_OUTER_ROLL_MASK;

/// `weather.md §2` autonomous wind-drift candidate modulus. After the
/// outer roll accepts, the selector picks a candidate in `0..=4` (Calm
/// plus four cardinals) via a modulo-five reduction of a fresh roll.
pub const WIND_DRIFT_CANDIDATE_MODULUS: u8 = 5;

/// `weather.md §2` autonomous wind-drift Calm acceptance threshold.
/// A `0` (Calm) candidate is accepted only when a follow-up roll over
/// `0..=255` is at least this value, so Calm is much rarer than any
/// cardinal.
pub const WIND_DRIFT_CALM_ACCEPT_MIN: u8 = 192;

/// `weather.md §3` shared wind-setter outcome. Rel Hur, the Wind
/// Change scroll, and any caller that programmatically targets a
/// wind state all funnel through the same setter; the setter's
/// behaviour is determined by the `old → new` transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindSetterOutcome {
    /// Calm → Calm: the setter does nothing. No sound, no display
    /// update, no stored-wind write.
    NoOp,
    /// Any other accepted target: the setter plays the wind sound,
    /// stores the new wind, and refreshes its display.
    Apply,
}

/// `weather.md §3`: classify a setter call by its `old → new`
/// transition. Returns `WindSetterOutcome::NoOp` when both sides
/// are `Calm`; otherwise `WindSetterOutcome::Apply`.
pub const fn wind_setter_outcome(old: WindState, new: WindState) -> WindSetterOutcome {
    match (old, new) {
        (WindState::Calm, WindState::Calm) => WindSetterOutcome::NoOp,
        _ => WindSetterOutcome::Apply,
    }
}
