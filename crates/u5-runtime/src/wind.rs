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

    pub fn status_message(self) -> &'static str {
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
            return Some(0);
        }
        if wind == heading {
            // Sailing into the wind.
            Some(2)
        } else {
            // Sailing with the wind (heading == opposite of wind source).
            Some(1)
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
            return Some((1, 1));
        }
        if wind == heading {
            // Frame faces directly into wind — 2 of 3 turns.
            Some((2, 3))
        } else {
            // Frame faces with wind (away from source) — 3 of 4 turns.
            Some((3, 4))
        }
    }

    /// `weather.md §2` autonomous wind-drift acceptance: only the
    /// zero outer roll over `0..=63` advances to candidate selection.
    pub const fn autonomous_drift_outer_accepted(outer_roll: u8) -> bool {
        outer_roll == 0
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
                if calm_followup_roll >= 192 {
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
