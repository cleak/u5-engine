//! The display driver's rectangle dissolve
//! (`systems/display-driver-abi.md` section 9.6, published in answer to
//! `cleak/u5-spec#53`).
//!
//! Dispatch offset `0x66` with carry clear copies pixels from the hidden
//! surface to the visible page in pseudo-random order until every pixel in the
//! requested inclusive rectangle has been copied exactly once. The published
//! visible contract is:
//!
//! 1. every pixel inside the inclusive rectangle is visited exactly once;
//! 2. after the entry returns, the front buffer matches the back buffer inside
//!    the rectangle;
//! 3. the visit order is deterministic and reproducible across calls with the
//!    same rectangle dimensions;
//! 4. the order is not row-major, not column-major and not a clean spiral - it
//!    reads as scattered single-pixel updates.
//!
//! The original selects the next pixel with a Galois LFSR indexed by the
//! rectangle's pixel count. This implementation uses the same shape - a
//! maximal-length Galois LFSR over the smallest register that spans the pixel
//! count, skipping states past the end - which satisfies all four bullets.
//! The spec explicitly allows any order that does, and notes that only an
//! engine wanting frame-for-frame parity needs the original tap inventory.
//!
//! **This replaces the withdrawn left-to-right column sweep.** The earlier
//! contract's one-column-per-title-tick schedule, and its 36-tick and 320-tick
//! figures, were retracted in full for both intro callers; there is no column
//! rate to publish for any caller.

use std::io;

/// `display-driver-abi.md §9.6`: the dissolve is issued as **one blocking
/// call**. No world tick, no title tick and no gameplay time advances while it
/// runs, whatever the rectangle's size, and its wall-clock duration is
/// whatever the machine needs to visit that many pixels. The spec publishes no
/// measured duration for any rectangle, so an engine has no published rate to
/// pace it by.
pub const RECTANGLE_DISSOLVE_IS_ONE_BLOCKING_CALL: bool = true;

/// Maximal-length Galois tap words for register widths 2..=24 bits. Each is a
/// primitive polynomial over GF(2), so the register cycles through every
/// nonzero state exactly once before repeating.
const GALOIS_TAPS: [u32; 23] = [
    0x3,      // 2 bits
    0x6,      // 3
    0xC,      // 4
    0x14,     // 5
    0x30,     // 6
    0x60,     // 7
    0xB8,     // 8
    0x110,    // 9
    0x240,    // 10
    0x500,    // 11
    0xE08,    // 12
    0x1C80,   // 13
    0x3802,   // 14
    0x6000,   // 15
    0xD008,   // 16
    0x12000,  // 17
    0x20400,  // 18
    0x40023,  // 19
    0x90000,  // 20
    0x140000, // 21
    0x300000, // 22
    0x420000, // 23
    0xE10000, // 24
];

/// The shared visit order behind every rectangle dissolve in the engine.
///
/// Yields each index in `0..count` exactly once, in the pseudo-random order
/// `display-driver-abi.md` section 9.6 describes, and nothing else needs to
/// know how that order is produced. Both the driver-surface entry
/// (`display_driver::EgaDissolveState`, the dispatch `0x66` operation) and the
/// caller-side [`RectangleDissolve`] wrap this, so a dissolve scatters
/// identically whichever path issues it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DissolveVisitOrder {
    state: u32,
    mask: u32,
    taps: u32,
    count: u32,
    visited: u32,
}

impl DissolveVisitOrder {
    /// A generator over `count` indices. A count of zero is immediately
    /// finished.
    pub fn new(count: usize) -> io::Result<Self> {
        let count = u32::try_from(count).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "dissolve pixel count exceeds the addressable range",
            )
        })?;
        let mut bits = 2u32;
        while bits < 32 && (1u32 << bits) - 1 < count {
            bits += 1;
        }
        let taps = *GALOIS_TAPS.get(bits as usize - 2).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("dissolve pixel count {count} exceeds the published tap inventory"),
            )
        })?;
        Ok(Self {
            state: 1,
            mask: (1u32 << bits) - 1,
            taps,
            count,
            visited: 0,
        })
    }

    pub const fn count(&self) -> usize {
        self.count as usize
    }

    pub const fn visited(&self) -> usize {
        self.visited as usize
    }

    pub const fn remaining(&self) -> usize {
        (self.count - self.visited) as usize
    }

    pub const fn is_finished(&self) -> bool {
        self.visited >= self.count
    }

    /// The next index to copy, or `None` once every index has been visited.
    pub fn next_index(&mut self) -> Option<usize> {
        while self.visited < self.count {
            // Galois step: shift right, xor the taps back in on carry-out.
            let lsb = self.state & 1;
            self.state >>= 1;
            if lsb == 1 {
                self.state ^= self.taps;
            }
            self.state &= self.mask;
            // The register never reaches 0, so index = state - 1 covers
            // 0..=mask-1 exactly once; indices past the end are skipped.
            let index = self.state - 1;
            if index < self.count {
                self.visited += 1;
                return Some(index as usize);
            }
        }
        None
    }
}

/// An in-progress rectangle dissolve over an inclusive pixel rectangle.
///
/// Stepping is exposed so a caller can copy the visited pixels itself, but the
/// original issues the whole transfer as one blocking call - see
/// [`RECTANGLE_DISSOLVE_IS_ONE_BLOCKING_CALL`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RectangleDissolve {
    left: u16,
    top: u16,
    width: u32,
    height: u32,
    order: DissolveVisitOrder,
}

impl RectangleDissolve {
    /// Starts a dissolve over the inclusive rectangle `(x0, y0, x1, y1)`.
    ///
    /// `display-driver-abi.md §9.6`: all four edges are inclusive, and the
    /// caller-side wrapper normalises and clamps the rectangle before the
    /// driver sees it, so this rejects an inverted rectangle rather than
    /// silently walking nothing.
    pub fn new(rect: (u16, u16, u16, u16)) -> io::Result<Self> {
        let (x0, y0, x1, y1) = rect;
        if x1 < x0 || y1 < y0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("rectangle dissolve bounds ({x0}, {y0})..({x1}, {y1}) are inverted"),
            ));
        }
        let width = u32::from(x1 - x0) + 1;
        let height = u32::from(y1 - y0) + 1;
        Ok(Self {
            left: x0,
            top: y0,
            width,
            height,
            order: DissolveVisitOrder::new((width * height) as usize)?,
        })
    }

    /// Total pixels the dissolve will visit.
    pub fn pixel_count(&self) -> u32 {
        self.width * self.height
    }

    /// Pixels visited so far.
    pub fn visited(&self) -> u32 {
        self.order.visited() as u32
    }

    pub fn is_complete(&self) -> bool {
        self.order.is_finished()
    }

    /// The next pixel to copy from the hidden surface to the visible page, or
    /// `None` once every pixel has been visited exactly once.
    pub fn next_pixel(&mut self) -> Option<(u16, u16)> {
        let index = self.order.next_index()? as u32;
        Some((
            self.left + (index % self.width) as u16,
            self.top + (index / self.width) as u16,
        ))
    }

    /// Runs the transfer to completion, handing each visited pixel to `copy`.
    ///
    /// This is the shape every caller of the rectangle dissolve uses: one
    /// blocking call that returns with the visible page matching the hidden
    /// surface inside the rectangle.
    pub fn run_to_completion(&mut self, mut copy: impl FnMut(u16, u16)) {
        while let Some((x, y)) = self.next_pixel() {
            copy(x, y);
        }
    }
}

/// `display-driver-abi.md §9.6` abort gate.
///
/// The gate is enabled when the driver image is first loaded and is cleared
/// permanently the first time any character is drawn through the driver's
/// fixed-cell glyph entry; nothing ever re-enables it. While enabled, the
/// dissolve copies the current pixel, then emits a speaker click and samples
/// keyboard status on alternating visits. The zero-initialized phase makes
/// visits **1, 3, 5, ...** the sampled visits. A pending keystroke aborts the
/// call after that visit, leaving the rectangle partly transferred. The abort
/// only tests for a pending key and does not consume it.
///
/// In a normal session that makes exactly one dissolve interruptible - the
/// first start/menu reveal, before any menu text has been drawn. Model the
/// gate rather than special-casing "the first call", because a path that draws
/// text earlier changes which call is affected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DissolveAbortGate {
    armed: bool,
}

impl Default for DissolveAbortGate {
    fn default() -> Self {
        Self::on_driver_load()
    }
}

impl DissolveAbortGate {
    /// The gate as it stands when the driver image is first loaded.
    pub const fn on_driver_load() -> Self {
        Self { armed: true }
    }

    /// Whether a dissolve issued now would click, poll and be abortable.
    pub const fn is_armed(self) -> bool {
        self.armed
    }

    /// Clears the gate permanently, as drawing any character through the
    /// driver's fixed-cell glyph entry does.
    pub fn note_fixed_cell_glyph_drawn(&mut self) {
        self.armed = false;
    }

    /// `§9.6`: `copied_pixels` is the one-based number of pixels already
    /// transferred. The click and keyboard poll occur after copies 1, 3, 5,
    /// ... and only there.
    pub const fn samples_input_after_copy(self, copied_pixels: u32) -> bool {
        self.armed && copied_pixels % 2 == 1
    }
}
