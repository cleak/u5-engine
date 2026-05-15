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
