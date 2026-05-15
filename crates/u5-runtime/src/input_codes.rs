//! Final input-layer direction codes per `input.md` §5. The keyboard
//! peek translates numpad cardinals/diagonals, extended arrow keys, and
//! shifted top-row digits into the eight high-byte direction codes used
//! by the upper layers; gameplay mode loops handle these inline before
//! the central command dispatcher sees ordinary letter keys.

/// `input.md §5` direction codes (eight directions plus the "no direction"
/// case that the upper layer represents by simply not seeing one of these
/// bytes).
pub const INPUT_CODE_NORTHWEST: u8 = 0xD3;
pub const INPUT_CODE_SOUTHWEST: u8 = 0xD4;
pub const INPUT_CODE_NORTHEAST: u8 = 0xD5;
pub const INPUT_CODE_SOUTHEAST: u8 = 0xD6;
pub const INPUT_CODE_WEST: u8 = 0xFB;
pub const INPUT_CODE_EAST: u8 = 0xFC;
pub const INPUT_CODE_NORTH: u8 = 0xFD;
pub const INPUT_CODE_SOUTH: u8 = 0xFE;

/// `input.md §5` direction-code classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputDirection {
    North,
    South,
    East,
    West,
    Northwest,
    Northeast,
    Southwest,
    Southeast,
}

impl InputDirection {
    /// Whether this direction is one of the four cardinals; world, town,
    /// dungeon, and combat movement consumers accept only this subset.
    pub const fn is_cardinal(self) -> bool {
        matches!(
            self,
            InputDirection::North
                | InputDirection::South
                | InputDirection::East
                | InputDirection::West
        )
    }
}

/// `input.md §5`: classify a final input-layer byte as a direction.
/// Returns `None` for any byte outside the eight published direction
/// codes; non-direction bytes are letters, function keys, or the prompt
/// dispatcher's own control bytes.
pub const fn input_code_direction(byte: u8) -> Option<InputDirection> {
    Some(match byte {
        INPUT_CODE_NORTH => InputDirection::North,
        INPUT_CODE_SOUTH => InputDirection::South,
        INPUT_CODE_EAST => InputDirection::East,
        INPUT_CODE_WEST => InputDirection::West,
        INPUT_CODE_NORTHWEST => InputDirection::Northwest,
        INPUT_CODE_NORTHEAST => InputDirection::Northeast,
        INPUT_CODE_SOUTHWEST => InputDirection::Southwest,
        INPUT_CODE_SOUTHEAST => InputDirection::Southeast,
        _ => return None,
    })
}

/// `input.md §6` case fold: lowercase ASCII letters are folded to upper
/// case by simple subtraction; other bytes pass through unchanged. This
/// is locale-free and table-free, and is a no-op for higher-byte codes
/// (function keys, direction codes, control bytes).
pub const fn input_case_fold(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - (b'a' - b'A')
    } else {
        byte
    }
}
