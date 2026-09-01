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

use crate::audio::{DissolveToneState, SoundEffect};

/// `display-driver-abi.md §9.6`: the dissolve is issued as **one blocking
/// call**. No world tick, no title tick and no gameplay time advances while it
/// runs, whatever the rectangle's size.
///
/// `timing.md §5.1` publishes a rate for one of the two cases and not for the
/// other, so the two must not be described together:
///
/// - **Ungated** (every dissolve after the gate is cleared — the intro story
///   step-1 transition, the endgame fade, all three map-viewport sites):
///   "Self-paced inside one driver call, with no delay at all ... it has no
///   wait of any kind in its inner loop — it transfers one pixel per iteration
///   as fast as the host manages, so on a modern host it is close to
///   instantaneous." There is no published rate to pace *these* by.
/// - **Gated** (the first start/menu logo reveal): "*Correction: the 'no wait
///   of any kind' row above describes the ungated dissolve only.* While the
///   driver-local sound/abort gate is still set, every second visited pixel
///   retunes the speaker and pays about 50 to 60 microseconds of calibrated
///   hold — one outer unit at the shift-four subdivision of section 6.2 — plus
///   the retune and poll work." See [`GATED_DISSOLVE_CLICK_HOLD_NANOS`].
pub const RECTANGLE_DISSOLVE_IS_ONE_BLOCKING_CALL: bool = true;

/// `timing.md §5.1`: the calibrated hold a **gated** dissolve pays on every
/// second visited pixel — "about 50 to 60 microseconds ... one outer unit at
/// the shift-four subdivision of section 6.2".
///
/// §6.2's shift-four subdivision is the calibration count divided by sixteen,
/// truncated; §7.3 converts that class to real time and gets the same figure
/// for both of its other members: "A random-rumble step unit costs about 60
/// microseconds" and "A title-sequence ignition burst pitch hold costs about 60
/// microseconds." The click hold is that same class, so it is carried at the
/// top of the published 50-to-60 band.
///
/// This is the only part of the gated call's cost published as fact. §5.1 adds
/// that "A hand-built cycle model puts the whole gated call at roughly 8 to
/// 14 s, but it omits display-memory wait states entirely and is
/// **unverified**", so [`gated_dissolve_hold_nanos`] deliberately reports only
/// the calibrated holds and is a floor on the real duration, not an estimate of
/// it.
pub const GATED_DISSOLVE_CLICK_HOLD_NANOS: u64 = 60_000;

/// The calibrated hold a gated dissolve of `clicked_visits` clicking visits
/// pays in total.
///
/// `timing.md §5.1`: "For the 32,320-pixel logo rectangle that is 16,160 such
/// visits."
pub const fn gated_dissolve_hold_nanos(clicked_visits: u32) -> u64 {
    clicked_visits as u64 * GATED_DISSOLVE_CLICK_HOLD_NANOS
}

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

/// Visit indices `display-driver-abi.md §9.6` publishes outright, keyed by the
/// rectangle's pixel count.
///
/// The substituted LFSR below is free to scatter however it likes - §9.6 only
/// binds an engine that does not want frame-for-frame parity to the four
/// visible bullets - but where the section names an exact visit, that is a fact
/// about the *order*, not about a caller:
///
/// > Thus a key already pending when the first start/menu dissolve begins
/// > leaves exactly one pixel transferred before abort: the first visit,
/// > `(1,0)`.
///
/// The start/menu rectangle `(0, 0)..(319, 100)` is 320 by 101, so `(1,0)` is
/// linear index 1 of 32320. Bullet 3 makes the order a function of the
/// rectangle's dimensions alone, so this leads every dissolve of that size -
/// armed gate or not, driver-surface entry or caller-side wrapper.
const PUBLISHED_FIRST_VISITS: [(u32, u32); 1] = [(320 * 101, 1)];

const fn published_first_visit(count: u32) -> Option<u32> {
    let mut index = 0;
    while index < PUBLISHED_FIRST_VISITS.len() {
        let (pixels, first) = PUBLISHED_FIRST_VISITS[index];
        if pixels == count {
            return Some(first);
        }
        index += 1;
    }
    None
}

/// The shared visit order behind every rectangle dissolve in the engine.
///
/// Yields each index in `0..count` exactly once, in the pseudo-random order
/// `display-driver-abi.md` section 9.6 describes, and nothing else needs to
/// know how that order is produced. Both the driver-surface entry
/// (`display_driver::EgaDissolveState`, the dispatch `0x66` operation) and the
/// caller-side [`RectangleDissolve`] wrap this, so a dissolve scatters
/// identically whichever path issues it - including the published first visit
/// of [`PUBLISHED_FIRST_VISITS`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DissolveVisitOrder {
    state: u32,
    mask: u32,
    taps: u32,
    count: u32,
    visited: u32,
    /// `§9.6`'s published first visit for this pixel count, if it names one.
    /// It is emitted before the walk and suppressed when the walk reaches it,
    /// so every index still comes out exactly once.
    published_first: Option<u32>,
    lead_pending: bool,
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
        let published_first = published_first_visit(count).filter(|first| *first < count);
        Ok(Self {
            state: 1,
            mask: (1u32 << bits) - 1,
            taps,
            count,
            visited: 0,
            published_first,
            lead_pending: published_first.is_some(),
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
        if self.visited >= self.count {
            return None;
        }
        if self.lead_pending {
            self.lead_pending = false;
            if let Some(first) = self.published_first {
                self.visited += 1;
                return Some(first as usize);
            }
        }
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
            // The published first visit already came out ahead of the walk.
            if index < self.count && self.published_first != Some(index) {
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

    /// The next pixel paired with the click and keyboard poll `gate` schedules
    /// for it, or `None` once every pixel has been visited.
    ///
    /// `audio.md §8.6`: the click is emitted at the same points that poll, and
    /// only "every second visited pixel" while the gate is enabled.
    pub fn next_gated_pixel(&mut self, gate: &mut DissolveAbortGate) -> Option<DissolveVisit> {
        let (x, y) = self.next_pixel()?;
        let copied_pixels = self.visited();
        Some(DissolveVisit {
            copied_pixels,
            x,
            y,
            samples_input: gate.samples_input_after_copy(copied_pixels),
            sound: gate.click_after_copy(copied_pixels),
        })
    }

    /// Runs the gated transfer, handing each visit to `visit` after the pixel
    /// has been selected and reporting where it stopped.
    ///
    /// `audio.md §8.6.1`: "a pending key aborts after the current copied pixel,
    /// and that exit silences the speaker". The abort therefore completes this
    /// visit — including its retune — before returning.
    ///
    /// The speaker is **not** left stopped by the click itself: §8.6.1 puts the
    /// single silencing point on "the dissolve's shared exit block, reached by
    /// both the abort path and normal completion". Callers lower the collected
    /// run with [`audio::dissolve_click_run`], which emits that one stop; a
    /// caller assembling the run itself ends it with
    /// [`SoundEffect::DissolveExit`].
    ///
    /// `visit` answers with [`DissolveVisit::poll`], so only a visit the
    /// alternating flag actually checked can stop the transfer.
    pub fn run_gated(
        &mut self,
        gate: &mut DissolveAbortGate,
        mut visit: impl FnMut(&DissolveVisit) -> DissolveControl,
    ) -> DissolveOutcome {
        let mut clicks = 0u32;
        while let Some(step) = self.next_gated_pixel(gate) {
            if step.sound.is_some() {
                clicks += 1;
            }
            if visit(&step).is_abort() {
                return DissolveOutcome {
                    copied_pixels: step.copied_pixels,
                    clicks,
                    aborted: true,
                };
            }
        }
        DissolveOutcome {
            copied_pixels: self.visited(),
            clicks,
            aborted: false,
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
    /// `audio.md §8.6.1`: the tone state is **driver-local**, not per-dissolve.
    /// The band-width counter "is never reset by the dissolve", and the pitch
    /// state is shared with the subtitle ignition, so it lives on the gate —
    /// the other piece of driver-image state with the same lifetime — rather
    /// than on the [`Dissolve`], which is created per call.
    tone: DissolveToneState,
}

impl Default for DissolveAbortGate {
    fn default() -> Self {
        Self::on_driver_load()
    }
}

impl DissolveAbortGate {
    /// The gate as it stands when the driver image is first loaded.
    pub const fn on_driver_load() -> Self {
        Self {
            armed: true,
            tone: DissolveToneState::on_driver_load(),
        }
    }

    /// The driver-local tone state, for handing on to the subtitle ignition.
    pub const fn tone(self) -> DissolveToneState {
        self.tone
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

    /// `audio.md §8.6.1`: "every second visited pixel advances a driver-local
    /// pitch state and retunes a continuously running speaker carrier; the
    /// speaker is enabled at the first click and silenced only at the
    /// dissolve's shared exit."
    ///
    /// The click and the poll are the *same* points. Each click advances the
    /// driver-local state and yields that click's frequency outright — §8.6.1
    /// withdrew the reading where the fraction of the rectangle copied selected
    /// the pitch, so this deliberately takes no `total_pixels`: "Not the
    /// fraction of the rectangle copied, and not the pixel coordinate."
    /// A disarmed gate is silent, which is the same clause's "later dissolves
    /// in the same run are silent".
    pub fn click_after_copy(&mut self, copied_pixels: u32) -> Option<SoundEffect> {
        if !self.samples_input_after_copy(copied_pixels) {
            return None;
        }
        Some(SoundEffect::DissolveClick {
            frequency_hz: self.tone.next_click_hz(),
        })
    }
}

/// One visited pixel of a gated dissolve.
///
/// `audio.md §8.6` binds the copy, the click and the keyboard poll into one
/// ordered beat: the pixel is copied first, then the sampled visits click and
/// poll, then a pending key aborts.
///
/// Only [`RectangleDissolve::next_gated_pixel`] builds one, so the fields stay
/// private: a visit always carries the click and poll schedule its gate
/// actually produced rather than one a caller assembled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DissolveVisit {
    copied_pixels: u32,
    x: u16,
    y: u16,
    samples_input: bool,
    sound: Option<SoundEffect>,
}

impl DissolveVisit {
    /// One-based count of pixels transferred including this one.
    pub const fn copied_pixels(&self) -> u32 {
        self.copied_pixels
    }

    pub const fn x(&self) -> u16 {
        self.x
    }

    pub const fn y(&self) -> u16 {
        self.y
    }

    /// Whether this visit polls keyboard status.
    pub const fn samples_input(&self) -> bool {
        self.samples_input
    }

    /// The click this visit emits. `Some` exactly when [`Self::samples_input`]
    /// is true.
    pub fn sound(&self) -> Option<&SoundEffect> {
        self.sound.as_ref()
    }

    /// The driver's status test for this visit, given whether a key is pending
    /// when it runs.
    ///
    /// `display-driver-abi.md §9.6`: "Both the click and the poll sit behind
    /// the same alternating flag, so neither happens on every pixel." An
    /// unchecked visit performs no status test at all, so `key_pending` cannot
    /// reach it and the transfer continues; the key simply stays queued for the
    /// next checked visit, or for the caller's own consuming read. This is the
    /// only way to obtain an aborting [`DissolveControl`], which is what keeps
    /// the pairing a property of the type rather than of caller discipline.
    pub const fn poll(&self, key_pending: bool) -> DissolveControl {
        DissolveControl(self.samples_input && key_pending)
    }
}

/// What the caller wants after copying and polling one visit.
///
/// [`DissolveControl::CONTINUE`] is the only value a caller can name directly;
/// an abort exists only as the answer [`DissolveVisit::poll`] gives for a visit
/// that actually polled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DissolveControl(bool);

impl DissolveControl {
    /// Carry on to the next pixel.
    pub const CONTINUE: Self = Self(false);

    /// `§9.6`: a key was pending at a checked visit's status test.
    pub const fn is_abort(self) -> bool {
        self.0
    }
}

/// The result of a gated dissolve run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DissolveOutcome {
    pub copied_pixels: u32,
    pub clicks: u32,
    /// `§9.6`: an abort leaves the rectangle partly transferred.
    pub aborted: bool,
}

impl DissolveOutcome {
    /// The calibrated hold this run paid, from `timing.md §5.1`'s published
    /// per-click cost.
    ///
    /// A gated run is **not** instantaneous: the withdrawn reading that the
    /// start/menu reveal "has no wait of any kind in its inner loop" describes
    /// the ungated dissolve only. An ungated run clicks nothing and so reports
    /// zero here, which is the case that genuinely has no published rate.
    pub const fn hold_nanos(&self) -> u64 {
        gated_dissolve_hold_nanos(self.clicks)
    }
}

#[cfg(test)]
mod published_visit_order_tests {
    use super::*;
    use std::collections::HashSet;

    const START_MENU_RECT: (u16, u16, u16, u16) = (0, 0, 319, 100);

    #[test]
    fn the_start_menu_rectangle_leads_with_the_published_first_visit() {
        // `display-driver-abi.md §9.6`: "a key already pending when the first
        // start/menu dissolve begins leaves exactly one pixel transferred
        // before abort: the first visit, `(1,0)`." That is a property of the
        // visit order for this rectangle's dimensions, so it belongs to the
        // shared generator rather than to any caller.
        let mut dissolve = RectangleDissolve::new(START_MENU_RECT).unwrap();
        assert_eq!(dissolve.pixel_count(), 320 * 101);
        assert_eq!(dissolve.next_pixel(), Some((1, 0)));

        // `§9.6` bullet 3 makes the order a function of the rectangle's
        // dimensions, so the driver-surface entry leads with it too.
        let mut driver = crate::display_driver::EgaDissolveState::new(crate::DisplayPixelRect {
            x0: 0,
            y0: 0,
            x1: 319,
            y1: 100,
        });
        assert_eq!(driver.next_pixel(), Some((1, 0)));
    }

    /// `timing.md §5.1`, start/menu logo reveal, **first (gated) call**:
    /// "*Correction: the 'no wait of any kind' row above describes the ungated
    /// dissolve only.* While the driver-local sound/abort gate is still set,
    /// every second visited pixel retunes the speaker and pays about 50 to 60
    /// microseconds of calibrated hold — one outer unit at the shift-four
    /// subdivision of section 6.2 — plus the retune and poll work. For the
    /// 32,320-pixel logo rectangle that is 16,160 such visits."
    ///
    /// The withdrawn reading — that the start/menu reveal has no wait of any
    /// kind and is close to instantaneous on a modern host — applies only to
    /// the ungated case, which is pinned below.
    #[test]
    fn the_gated_start_menu_reveal_pays_the_published_per_click_hold() {
        let mut dissolve = RectangleDissolve::new(START_MENU_RECT).unwrap();
        assert_eq!(dissolve.pixel_count(), 32_320);

        let outcome = dissolve.run_gated(&mut DissolveAbortGate::on_driver_load(), |_| {
            DissolveControl::CONTINUE
        });

        assert!(!outcome.aborted);
        assert_eq!(outcome.copied_pixels, 32_320);
        // "For the 32,320-pixel logo rectangle that is 16,160 such visits."
        assert_eq!(outcome.clicks, 16_160);
        assert_eq!(
            outcome.hold_nanos(),
            16_160 * GATED_DISSOLVE_CLICK_HOLD_NANOS
        );
        // "about 50 to 60 microseconds" per visit.
        let per_click = outcome.hold_nanos() / u64::from(outcome.clicks);
        assert!(
            (50_000..=60_000).contains(&per_click),
            "per-click hold {per_click} ns is outside the published 50..60 us band"
        );
        // The published holds alone already take most of a second, so the
        // gated reveal cannot be modelled as instantaneous. `timing.md §5.1`
        // puts the whole call at "roughly 8 to 14 s", but that total "is
        // **unverified**", so only this floor is asserted.
        assert!(outcome.hold_nanos() > 900_000_000);
    }

    /// `timing.md §5.1`, start/menu logo reveal, **ungated**: "Self-paced
    /// inside one driver call, with no delay at all ... it has no wait of any
    /// kind in its inner loop — it transfers one pixel per iteration as fast as
    /// the host manages, so on a modern host it is close to instantaneous."
    ///
    /// The same clause covers the story step-1 reveal, the endgame fade and the
    /// three map-viewport dissolves: once the gate is cleared there is no
    /// published rate to pace any of them by.
    #[test]
    fn an_ungated_dissolve_has_no_published_rate_to_pace_it_by() {
        let mut dissolve = RectangleDissolve::new(START_MENU_RECT).unwrap();
        let mut gate = DissolveAbortGate::on_driver_load();
        gate.note_fixed_cell_glyph_drawn();

        let outcome = dissolve.run_gated(&mut gate, |_| DissolveControl::CONTINUE);

        assert_eq!(outcome.copied_pixels, 32_320);
        assert_eq!(outcome.clicks, 0);
        assert_eq!(outcome.hold_nanos(), 0);
    }

    #[test]
    fn the_published_first_visit_is_not_revisited_later() {
        // `§9.6` bullet 1: leading with `(1,0)` must not cost the rectangle a
        // pixel or hand one out twice.
        let mut dissolve = RectangleDissolve::new(START_MENU_RECT).unwrap();
        let count = dissolve.pixel_count() as usize;
        let mut seen = HashSet::with_capacity(count);
        while let Some(pixel) = dissolve.next_pixel() {
            assert!(seen.insert(pixel), "{pixel:?} visited twice");
        }
        assert_eq!(seen.len(), count);
        assert!(dissolve.is_complete());
    }

    #[test]
    fn no_other_rectangle_claims_a_published_first_visit() {
        // The lead-in is published for the start/menu rectangle only. The
        // other five call sites of `§9.6`'s caller census take the substituted
        // walk's own first index.
        for rect in [
            (40u16, 86u16, 75u16, 120u16),
            (0, 0, 319, 199),
            (8, 8, 183, 183),
        ] {
            let mut dissolve = RectangleDissolve::new(rect).unwrap();
            assert_eq!(published_first_visit(dissolve.pixel_count()), None);
            assert!(dissolve.next_pixel().is_some());
        }
    }
}
