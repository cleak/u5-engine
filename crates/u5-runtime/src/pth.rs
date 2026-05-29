//! Pen-stroke decoder for `BRITISH.PTH` per `formats/pth.md` §3-§5.
//!
//! The intro path file is a NUL-segmented stream of one-byte pen
//! deltas. Each non-zero byte splits into two four-bit nibbles, each
//! carrying a sign bit and a three-bit magnitude. The pen is "down"
//! (paints at the new position) only when both nibble magnitudes are
//! `0..=2`; otherwise the byte is a pen-up move.

use std::io;
use std::path::Path;

use crate::read_disk_file;

/// `formats/pth.md §1`: shipped DOS file name.
pub const BRITISH_PTH_FILE: &str = "BRITISH.PTH";
/// `formats/pth.md §1`: shipped DOS file size in bytes.
pub const BRITISH_PTH_LEN: usize = 2_783;
/// `formats/pth.md §2`: number of segments the four NUL terminators
/// divide the stream into.
pub const BRITISH_PTH_SEGMENT_COUNT: usize = 4;
/// `systems/timing.md §5.1`: the title-screen signature walker emits
/// one ~55 ms BIOS-tick wait per 32 consumed path bytes, so 32
/// strokes are walked and painted per tick. The chunk counter
/// resets per segment (see [`BritishPth::next_signature_progress`]).
pub const BRITISH_PTH_STROKES_PER_TICK: usize = 32;

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

/// Decoded `BRITISH.PTH` stream split at the four NUL segment
/// terminators. Each segment restarts from the matching title-screen
/// pen origin supplied by `intro.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BritishPth {
    pub segments: Vec<Vec<PenStroke>>,
}

impl BritishPth {
    pub fn segment(&self, index: usize) -> Option<&[PenStroke]> {
        self.segments.get(index).map(Vec::as_slice)
    }

    /// Total decoded pen strokes (non-NUL bytes) across all four
    /// segments.
    pub fn total_strokes(&self) -> usize {
        self.segments.iter().map(Vec::len).sum()
    }

    /// `systems/timing.md §5.1`: advance the signature reveal by one
    /// ~55 ms BIOS-tick chunk. The original walker emits one
    /// hardware-tick wait per 32 consumed path bytes
    /// (`(byte_index & 0x1F) == 0`), and `byte_index` is local to
    /// each per-segment walker call, so the 32-stroke chunk
    /// alignment resets at every segment boundary.
    ///
    /// Given the current cumulative stroke `progress`, returns the
    /// progress after one more tick: the next 32-stroke boundary
    /// within the segment that `progress` falls in, clamped to that
    /// segment's end. Once `progress` reaches a segment boundary the
    /// next tick begins the following segment's first chunk. At or
    /// past the final stroke the value is unchanged (caller treats
    /// reaching `total_strokes()` as completion).
    pub fn next_signature_progress(&self, progress: usize) -> usize {
        let mut base = 0;
        for segment in &self.segments {
            let seg_len = segment.len();
            if progress < base + seg_len {
                let within = progress - base;
                let next_within = within
                    .checked_div(BRITISH_PTH_STROKES_PER_TICK)
                    .map(|chunks| (chunks + 1) * BRITISH_PTH_STROKES_PER_TICK)
                    .unwrap_or(BRITISH_PTH_STROKES_PER_TICK)
                    .min(seg_len);
                return base + next_within;
            }
            base += seg_len;
        }
        base
    }
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

pub fn load_british_pth(game_dir: &Path) -> io::Result<BritishPth> {
    parse_british_pth(&read_disk_file(&game_dir.join(BRITISH_PTH_FILE))?)
}

pub fn parse_british_pth(bytes: &[u8]) -> io::Result<BritishPth> {
    if bytes.len() != BRITISH_PTH_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{BRITISH_PTH_FILE} must be {BRITISH_PTH_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }

    let mut segments = Vec::with_capacity(BRITISH_PTH_SEGMENT_COUNT);
    let mut current = Vec::new();
    for byte in bytes {
        let Some(stroke) = pth_decode_byte(*byte) else {
            if current.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{BRITISH_PTH_FILE} contains an empty path segment"),
                ));
            }
            segments.push(current);
            current = Vec::new();
            continue;
        };
        current.push(stroke);
    }

    if !current.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{BRITISH_PTH_FILE} is missing its final segment terminator"),
        ));
    }
    if segments.len() != BRITISH_PTH_SEGMENT_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{BRITISH_PTH_FILE} must contain {BRITISH_PTH_SEGMENT_COUNT} segments, got {}",
                segments.len()
            ),
        ));
    }

    Ok(BritishPth { segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_british_pth_bytes() -> Vec<u8> {
        let mut bytes = Vec::with_capacity(BRITISH_PTH_LEN);
        for len in [856usize, 548, 411, 964] {
            bytes.extend(std::iter::repeat(0x11).take(len));
            bytes.push(0);
        }
        bytes
    }

    #[test]
    fn parse_british_pth_splits_four_spec_segments() {
        let pth = parse_british_pth(&synthetic_british_pth_bytes()).unwrap();

        assert_eq!(pth.segments.len(), BRITISH_PTH_SEGMENT_COUNT);
        assert_eq!(pth.segment(0).unwrap().len(), 856);
        assert_eq!(pth.segment(1).unwrap().len(), 548);
        assert_eq!(pth.segment(2).unwrap().len(), 411);
        assert_eq!(pth.segment(3).unwrap().len(), 964);
        assert_eq!(
            pth.segment(0).unwrap()[0],
            PenStroke {
                dx: 1,
                dy: 1,
                pen_down: true,
            }
        );
        assert!(pth.segment(4).is_none());
    }

    #[test]
    fn next_signature_progress_chunks_32_per_tick_resetting_per_segment() {
        // `systems/timing.md §5.1`: 32 strokes per BIOS tick, chunk
        // counter resets per segment. The shipped path has segments
        // of 856 / 548 / 411 / 964 strokes.
        let pth = parse_british_pth(&synthetic_british_pth_bytes()).unwrap();
        assert_eq!(pth.total_strokes(), 856 + 548 + 411 + 964);

        // Within segment 1: 0 -> 32 -> 64 ...
        assert_eq!(pth.next_signature_progress(0), 32);
        assert_eq!(pth.next_signature_progress(32), 64);
        assert_eq!(pth.next_signature_progress(64), 96);

        // Segment 1 tail: 856 = 26*32 + 24. From 832 the next chunk
        // clamps to the segment end (856), not 864.
        assert_eq!(pth.next_signature_progress(832), 856);
        // At the segment-1 boundary, the next tick starts segment 2's
        // first 32-stroke chunk (856 -> 856 + 32 = 888).
        assert_eq!(pth.next_signature_progress(856), 888);

        // Count the ticks to fully reveal the signature; per-segment
        // chunking gives ceil(856/32)+ceil(548/32)+ceil(411/32)+
        // ceil(964/32) = 27 + 18 + 13 + 31 = 89 ticks.
        let total = pth.total_strokes();
        let mut progress = 0;
        let mut ticks = 0;
        while progress < total {
            progress = pth.next_signature_progress(progress);
            ticks += 1;
            assert!(ticks <= total, "tick loop must terminate");
        }
        assert_eq!(ticks, 27 + 18 + 13 + 31);
    }

    #[test]
    fn parse_british_pth_rejects_bad_shape() {
        let err = parse_british_pth(&[0x11, 0]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        let mut missing_final_terminator = synthetic_british_pth_bytes();
        *missing_final_terminator.last_mut().unwrap() = 0x11;
        let err = parse_british_pth(&missing_final_terminator).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        let mut empty_segment = synthetic_british_pth_bytes();
        empty_segment[0] = 0;
        let err = parse_british_pth(&empty_segment).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
