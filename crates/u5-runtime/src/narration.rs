//! Staged narration: text a presentation prints on a timer rather than all at
//! once inside its key handler.
//!
//! `main-loop.md §9` has presentations calling the world tick directly -
//! "presentations, cutscene beats and paced turn loops call it directly,
//! dozens of call sites" - so a handler that prints three records with waits
//! between them is holding the loop, not returning to it. This engine had no
//! way to express that: every handler printed its whole sequence in one call
//! and the window jumped straight to the end state.
//!
//! A beat is a delay followed by one line of text. Delays are held in
//! seconds rather than ticks so no frontend has to assume a pump rate; the
//! published counts are in ~55 ms BIOS ticks (`timing.md §4`) and convert
//! with [`bios_ticks_secs`].
//!
//! The queue is advanced against an **absolute** clock reading rather than by
//! accumulating frame deltas. Accumulating was the obvious shape and it was
//! wrong: the visual shell fed the same frame's delta in twice and every
//! staged sequence ran at double speed (`cleak/u5-engine#27`). A deadline
//! computed from the clock cannot be double-counted, so the pacing no longer
//! depends on how many times a frontend calls in per frame.

use std::collections::VecDeque;

/// The ~55 ms BIOS tick of `timing.md §4`, as seconds.
pub const BIOS_TICK_SECS: f64 = 65_536.0 / 1_193_182.0;

/// A published BIOS-tick count as a delay in seconds.
pub fn bios_ticks_secs(ticks: u16) -> f64 {
    f64::from(ticks) * BIOS_TICK_SECS
}

/// One staged line: how long to hold first, then what to print.
#[derive(Clone, Debug, PartialEq)]
pub struct NarrationBeat {
    /// Seconds to wait before this beat's text prints.
    pub delay_secs: f64,
    /// The line, with its own leading and trailing feeds exactly as the
    /// producer would have passed them to `emit_message_line`.
    pub text: String,
}

/// The queue plus how far into the current beat's wait we are.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StagedNarration {
    beats: VecDeque<NarrationBeat>,
    /// Clock reading at which the head beat is due. `None` until the first
    /// advance after the head becomes current, which is what makes the queue
    /// start its wait from the frontend's clock rather than from zero.
    due_at_secs: Option<f64>,
}

impl StagedNarration {
    pub fn is_empty(&self) -> bool {
        self.beats.is_empty()
    }

    pub fn push_secs(&mut self, delay_secs: f64, text: impl Into<String>) {
        self.beats.push_back(NarrationBeat {
            delay_secs,
            text: text.into(),
        });
    }

    /// Push a beat whose delay is a published BIOS-tick count.
    pub fn push_ticks(&mut self, delay_bios_ticks: u16, text: impl Into<String>) {
        self.push_secs(bios_ticks_secs(delay_bios_ticks), text);
    }

    pub fn clear(&mut self) {
        self.beats.clear();
        self.due_at_secs = None;
    }

    /// Advance to an absolute clock reading and return every beat whose wait
    /// has completed, in order.
    ///
    /// `now_secs` is monotonic seconds from any epoch the frontend likes -
    /// the value only has to increase. Calling twice with the same reading
    /// releases nothing the second time, which is the property the delta
    /// form did not have.
    pub fn advance_to(&mut self, now_secs: f64) -> Vec<NarrationBeat> {
        let mut released = Vec::new();
        while let Some(beat) = self.beats.front() {
            let due = *self
                .due_at_secs
                .get_or_insert_with(|| now_secs + beat.delay_secs);
            if now_secs < due {
                break;
            }
            let beat = self.beats.pop_front().expect("checked above");
            // The next beat's wait starts when this one was *due*, not when
            // the frontend noticed: a late frame must not stretch the
            // sequence beat by beat.
            self.due_at_secs = self.beats.front().map(|next| due + next.delay_secs);
            released.push(beat);
        }
        released
    }

    /// Release every remaining beat at once, in order. Used when something
    /// has to reach the end state now - a save, a test, or a frontend with no
    /// pump.
    pub fn drain_all(&mut self) -> Vec<NarrationBeat> {
        self.due_at_secs = None;
        self.beats.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beat_is_released_when_its_wait_completes() {
        let mut staged = StagedNarration::default();
        staged.push_secs(0.3, "first");
        staged.push_secs(0.2, "second");
        assert!(staged.advance_to(100.0).is_empty());
        assert!(staged.advance_to(100.2).is_empty());
        let released: Vec<_> = staged
            .advance_to(100.31)
            .into_iter()
            .map(|b| b.text)
            .collect();
        assert_eq!(released, vec!["first".to_string()]);
        assert!(staged.advance_to(100.4).is_empty());
        let released: Vec<_> = staged
            .advance_to(100.51)
            .into_iter()
            .map(|b| b.text)
            .collect();
        assert_eq!(released, vec!["second".to_string()]);
        assert!(staged.is_empty());
    }

    /// `cleak/u5-engine#27`: the visual shell fed one frame's delta in twice
    /// and every staged sequence ran at double speed. An absolute reading
    /// cannot be double-counted.
    #[test]
    fn advancing_twice_to_the_same_reading_releases_nothing_extra() {
        let mut staged = StagedNarration::default();
        staged.push_secs(0.3, "first");
        staged.push_secs(0.3, "second");
        assert!(staged.advance_to(10.0).is_empty());
        assert!(staged.advance_to(10.1).is_empty());
        assert!(staged.advance_to(10.1).is_empty());
        assert!(staged.advance_to(10.2).is_empty());
        assert!(staged.advance_to(10.2).is_empty());
        assert_eq!(staged.advance_to(10.31).len(), 1);
        assert!(staged.advance_to(10.31).is_empty());
        assert!(staged.advance_to(10.5).is_empty());
        assert_eq!(staged.advance_to(10.61).len(), 1);
        assert!(staged.is_empty());
    }

    #[test]
    fn a_slow_frame_releases_every_beat_it_covers() {
        let mut staged = StagedNarration::default();
        staged.push_secs(0.3, "first");
        staged.push_secs(0.2, "second");
        let released: Vec<_> = staged.advance_to(0.0).into_iter().map(|b| b.text).collect();
        assert!(released.is_empty());
        let released: Vec<_> = staged.advance_to(9.0).into_iter().map(|b| b.text).collect();
        assert_eq!(released, vec!["first".to_string(), "second".to_string()]);
        assert!(staged.is_empty());
    }

    #[test]
    fn a_late_frame_does_not_stretch_the_rest_of_the_sequence() {
        let mut staged = StagedNarration::default();
        staged.push_secs(1.0, "first");
        staged.push_secs(1.0, "second");
        assert!(staged.advance_to(0.0).is_empty());
        // The host stalls and only looks again half a second late.
        assert_eq!(staged.advance_to(1.5).len(), 1);
        // The second beat is still due at 2.0, not at 2.5.
        assert_eq!(staged.advance_to(2.01).len(), 1);
    }

    #[test]
    fn published_tick_counts_convert_to_the_measured_interval() {
        // `timing.md §4`: one BIOS tick is about 55 ms.
        assert!((bios_ticks_secs(10) - 0.549).abs() < 5e-3);
    }

    #[test]
    fn draining_releases_the_rest_in_order() {
        let mut staged = StagedNarration::default();
        staged.push_ticks(40, "a");
        staged.push_ticks(10, "b");
        let rest: Vec<_> = staged.drain_all().into_iter().map(|b| b.text).collect();
        assert_eq!(rest, vec!["a".to_string(), "b".to_string()]);
        assert!(staged.is_empty());
    }
}
