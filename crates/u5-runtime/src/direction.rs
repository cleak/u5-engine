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

    /// `commands.md §5.4`: the shared direction prompt "accepts only the
    /// four directions and Space" - the translated direction codes of
    /// `input.md §5` (arrows, numpad, shifted top-row digits). The
    /// terminal harness's letter aliases in [`Self::from_play_key`] are
    /// not directions to the original and are discarded here.
    pub fn from_prompt_key(key: char) -> Option<Self> {
        if let Some(byte) = input_byte_from_char(key) {
            if crate::input_code_direction(byte).is_some() {
                return Self::from_play_key(key);
            }
        }
        match key {
            '7' | '8' | '9' | '4' | '6' | '1' | '2' | '3' => Self::from_play_key(key),
            _ => None,
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

/// `dungeon-mode.md §4.1` (`cleak/u5-spec#81`) dungeon facing-label
/// field.
///
/// The bottom border band prints the literal `Dir:` and then the facing
/// name in a fixed five-character field. The names are **left**-padded:
/// `East` and `West` carry their own leading space inside the field and
/// `North` and `South` do not, which is what produces two spaces after
/// the colon for east and west and one for north and south. (It is the
/// mirror image of the wind banner, whose names pad on the right.)
///
/// Only the four cardinals can be a dungeon facing; anything else is the
/// published invalid-facing fallback, which keeps the field's width so
/// the label never changes length.
pub const fn dungeon_facing_label_field(facing: Direction) -> &'static str {
    match facing {
        Direction::North => "North",
        Direction::South => "South",
        Direction::East => " East",
        Direction::West => " West",
        _ => DUNGEON_FACING_LABEL_INVALID,
    }
}

/// `dungeon-mode.md §4.1`: a leading space followed by four question
/// marks — the same five-character width as a real facing name.
pub const DUNGEON_FACING_LABEL_INVALID: &str = " ????";
/// The literal that precedes the facing field in the bottom band.
pub const DUNGEON_FACING_LABEL_PREFIX: &str = "Dir:";
/// `dungeon-mode.md §4.1`: facing is encoded north, east, south, west.
pub const fn dungeon_facing_from_encoding(byte: u8) -> Option<Direction> {
    match byte {
        0 => Some(Direction::North),
        1 => Some(Direction::East),
        2 => Some(Direction::South),
        3 => Some(Direction::West),
        _ => None,
    }
}

/// `dungeon-mode.md §4.1`: the level is stored zero-based and displayed
/// one-based, range one through eight, printed as a single digit over
/// the `L` literal's placeholder cell.
pub const fn dungeon_level_label_digit(level: u8) -> Option<u8> {
    let shown = level as u16 + 1;
    if shown >= 1 && shown <= 8 {
        Some(shown as u8)
    } else {
        None
    }
}
