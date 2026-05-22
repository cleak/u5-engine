//! Eight-way compass direction, used by movement, facing, and wind.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    NorthWest,
    North,
    NorthEast,
    West,
    East,
    SouthWest,
    South,
    SouthEast,
}

impl Direction {
    pub fn delta(self) -> (isize, isize) {
        match self {
            Self::NorthWest => (-1, -1),
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::West => (-1, 0),
            Self::East => (1, 0),
            Self::SouthWest => (-1, 1),
            Self::South => (0, 1),
            Self::SouthEast => (1, 1),
        }
    }

    pub fn from_play_key(key: char) -> Option<Self> {
        if let Some(byte) = input_byte_from_char(key) {
            if let Some(direction) = crate::input_code_direction(byte) {
                match direction {
                    crate::InputDirection::North => return Some(Self::North),
                    crate::InputDirection::South => return Some(Self::South),
                    crate::InputDirection::East => return Some(Self::East),
                    crate::InputDirection::West => return Some(Self::West),
                    crate::InputDirection::Northwest => return Some(Self::NorthWest),
                    crate::InputDirection::Northeast => return Some(Self::NorthEast),
                    crate::InputDirection::Southwest => return Some(Self::SouthWest),
                    crate::InputDirection::Southeast => return Some(Self::SouthEast),
                }
            }
        }
        match key.to_ascii_lowercase() {
            '7' | 'y' => Some(Self::NorthWest),
            '8' | 'w' => Some(Self::North),
            '9' | 'u' => Some(Self::NorthEast),
            '4' | 'a' => Some(Self::West),
            '6' | 'd' => Some(Self::East),
            '1' | 'b' | 'z' => Some(Self::SouthWest),
            '2' | 's' => Some(Self::South),
            '3' | 'n' | 'c' => Some(Self::SouthEast),
            _ => None,
        }
    }

    pub fn is_cardinal(self) -> bool {
        matches!(self, Self::North | Self::East | Self::South | Self::West)
    }

    pub const fn cardinal_facing_index(self) -> Option<u8> {
        Some(match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
            _ => return None,
        })
    }

    pub fn opposite_cardinal(self) -> Option<Self> {
        match self {
            Self::North => Some(Self::South),
            Self::East => Some(Self::West),
            Self::South => Some(Self::North),
            Self::West => Some(Self::East),
            _ => None,
        }
    }

    pub fn turn_left_cardinal(self) -> Option<Self> {
        match self {
            Self::North => Some(Self::West),
            Self::West => Some(Self::South),
            Self::South => Some(Self::East),
            Self::East => Some(Self::North),
            _ => None,
        }
    }

    pub fn turn_right_cardinal(self) -> Option<Self> {
        match self {
            Self::North => Some(Self::East),
            Self::East => Some(Self::South),
            Self::South => Some(Self::West),
            Self::West => Some(Self::North),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::NorthWest => "Northwest",
            Self::North => "North",
            Self::NorthEast => "Northeast",
            Self::West => "West",
            Self::East => "East",
            Self::SouthWest => "Southwest",
            Self::South => "South",
            Self::SouthEast => "Southeast",
        }
    }
}

const fn input_byte_from_char(key: char) -> Option<u8> {
    let scalar = key as u32;
    if scalar <= u8::MAX as u32 {
        Some(scalar as u8)
    } else {
        None
    }
}
