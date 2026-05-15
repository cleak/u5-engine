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

/// `formats/pth.md §3,§5`: decode one stream byte. Returns `None` for
/// the NUL segment terminator (the caller advances to the next
/// segment); otherwise returns the decoded pen-stroke action.
pub const fn pth_decode_byte(byte: u8) -> Option<PenStroke> {
    if byte == 0 {
        return None;
    }
    let high_mag = ((byte >> 4) & 0x07) as i8;
    let low_mag = (byte & 0x07) as i8;
    let dx = if byte & 0x80 != 0 { -high_mag } else { high_mag };
    let dy = if byte & 0x08 != 0 { -low_mag } else { low_mag };
    // §5: both magnitudes 0..=2 → pen down; either > 2 → pen up.
    let pen_down = high_mag <= 2 && low_mag <= 2;
    Some(PenStroke { dx, dy, pen_down })
}
