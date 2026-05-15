//! Final input-layer direction codes per `input.md` §5. The keyboard
//! peek translates numpad cardinals/diagonals, extended arrow keys, and
//! shifted top-row digits into the eight high-byte direction codes used
//! by the upper layers; gameplay mode loops handle these inline before
//! the central command dispatcher sees ordinary letter keys.

/// `input.md §4` function-key remap. F1 through F10 become the
/// contiguous internal byte range `0xC9..=0xD2`, disjoint from
/// printable ASCII and the direction codes.
pub const INPUT_CODE_F1: u8 = 0xC9;
pub const INPUT_CODE_F10: u8 = 0xD2;
pub const INPUT_CODE_FUNCTION_FIRST: u8 = INPUT_CODE_F1;
pub const INPUT_CODE_FUNCTION_LAST: u8 = INPUT_CODE_F10;

/// `input.md §4`: returns `Some(1..=10)` for the function-key index a
/// remapped byte represents, or `None` for non-function bytes. F1 maps
/// to 1, F10 maps to 10.
pub const fn input_function_key_index(byte: u8) -> Option<u8> {
    if byte < INPUT_CODE_FUNCTION_FIRST || byte > INPUT_CODE_FUNCTION_LAST {
        None
    } else {
        Some(byte - INPUT_CODE_FUNCTION_FIRST + 1)
    }
}

/// `input.md §3` cursor-blink defaults: glyph code `4` is the first
/// frame; the blink modulus wraps the phase counter every `4658` poll
/// iterations. These are mutable resident values; callers that need
/// real-time pacing derive a visually similar cadence from elapsed
/// time while preserving the no-advance/erase contract.
pub const CURSOR_BLINK_BASE_GLYPH: u8 = 4;
pub const CURSOR_BLINK_MODULUS: u16 = 4658;

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
