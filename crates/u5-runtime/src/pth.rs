//! Pen-stroke decoder for `BRITISH.PTH` per `formats/pth.md` §3-§5.
//!
//! The intro path file is a NUL-segmented stream of one-byte pen
//! deltas. Each non-zero byte splits into two four-bit nibbles, each
//! carrying a sign bit and a three-bit magnitude. The pen is "down"
//! (paints at the new position) only when both nibble magnitudes are
//! `0..=2`; otherwise the byte is a pen-up move.

/// `formats/pth.md §1`: shipped DOS file size in bytes.
pub const BRITISH_PTH_LEN: usize = 2_783;
/// `formats/pth.md §2`: number of segments the four NUL terminators
/// divide the stream into.
pub const BRITISH_PTH_SEGMENT_COUNT: usize = 4;

/// `formats/pth.md §5` action the pen-stroke walker takes for one
/// non-zero byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PenStroke {
    /// Signed horizontal delta `-7..=7`.
    pub dx: i8,
    /// Signed vertical delta `-7..=7`.
    pub dy: i8,
    /// `true` when the pen is "down" and the new position should be
    /// painted; `false` for pen-up skip moves.
    pub pen_down: bool,
}

/// `formats/pth.md §3` per-nibble magnitude mask (low three bits).
/// Bit 3 of the nibble is the sign bit; bits 2..0 are the magnitude
/// in the unsigned range `0..=7`.
pub const PTH_NIBBLE_MAGNITUDE_MASK: u8 = 0x07;
/// `formats/pth.md §3` horizontal-nibble sign bit (high byte bit 7).
pub const PTH_BYTE_SIGN_X: u8 = 0x80;
/// `formats/pth.md §3` vertical-nibble sign bit (high byte bit 3).
pub const PTH_BYTE_SIGN_Y: u8 = 0x08;
/// `formats/pth.md §5` pen-down magnitude threshold. A byte paints
/// only when both nibble magnitudes are at or below this value; one
/// magnitude above it lifts the pen for that byte without advancing
/// the stroke into a sticky pen-up state.
pub const PTH_PEN_DOWN_MAX_MAGNITUDE: i8 = 2;

/// `formats/pth.md §3,§5`: decode one stream byte. Returns `None` for
/// the NUL segment terminator (the caller advances to the next
/// segment); otherwise returns the decoded pen-stroke action.
pub const fn pth_decode_byte(byte: u8) -> Option<PenStroke> {
    if byte == 0 {
        return None;
    }
    let high_mag = ((byte >> 4) & PTH_NIBBLE_MAGNITUDE_MASK) as i8;
    let low_mag = (byte & PTH_NIBBLE_MAGNITUDE_MASK) as i8;
    let dx = if byte & PTH_BYTE_SIGN_X != 0 {
        -high_mag
    } else {
        high_mag
    };
    let dy = if byte & PTH_BYTE_SIGN_Y != 0 {
        -low_mag
    } else {
        low_mag
    };
    // §5: both magnitudes <= PTH_PEN_DOWN_MAX_MAGNITUDE → pen down;
    // either above it → pen up for this byte only.
    let pen_down = high_mag <= PTH_PEN_DOWN_MAX_MAGNITUDE && low_mag <= PTH_PEN_DOWN_MAX_MAGNITUDE;
    Some(PenStroke { dx, dy, pen_down })
}
