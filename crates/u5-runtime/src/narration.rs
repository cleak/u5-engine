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
//! seconds rather than ticks so a frontend advances them with its own frame
//! delta and no assumption about how often it pumps; the published counts are
//! in ~55 ms BIOS ticks (`timing.md §4`) and convert with
//! [`bios_ticks_secs`].

use std::collections::VecDeque;

/// The ~55 ms BIOS tick of `timing.md §4`, as seconds.
pub const BIOS_TICK_SECS: f32 = 65_536.0 / 1_193_182.0;

/// A published BIOS-tick count as a delay in seconds.
pub fn bios_ticks_secs(ticks: u16) -> f32 {
    f32::from(ticks) * BIOS_TICK_SECS
}

/// One staged line: how long to hold first, then what to print.
#[derive(Clone, Debug, PartialEq)]
pub struct NarrationBeat {
    /// Seconds to wait before this beat's text prints.
    pub delay_secs: f32,
    /// The line, with its own leading and trailing feeds exactly as the
    /// producer would have passed them to `emit_message_line`.
    pub text: String,
}

/// The queue plus how far into the current beat's wait we are.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StagedNarration {
    beats: VecDeque<NarrationBeat>,
    elapsed_secs: f32,
}

impl StagedNarration {
    pub fn is_empty(&self) -> bool {
        self.beats.is_empty()
    }

    pub fn push_secs(&mut self, delay_secs: f32, text: impl Into<String>) {
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
        self.elapsed_secs = 0.0;
    }

    /// Advance by one frame's worth of real time and return every beat whose
    /// wait completed in it, in order. A frame long enough to cover two beats
    /// releases both rather than dropping one; the surplus time carries into
    /// the next beat's wait so a slow host does not stretch the sequence.
    pub fn advance(&mut self, delta_secs: f32) -> Vec<NarrationBeat> {
        let mut released = Vec::new();
        if self.beats.is_empty() {
            return released;
        }
        self.elapsed_secs += delta_secs.max(0.0);
        while self
            .beats
            .front()
            .is_some_and(|beat| self.elapsed_secs >= beat.delay_secs)
        {
            let beat = self.beats.pop_front().expect("checked above");
            self.elapsed_secs -= beat.delay_secs;
            released.push(beat);
        }
        if self.beats.is_empty() {
            self.elapsed_secs = 0.0;
        }
        released
    }

    /// Release every remaining beat at once, in order. Used when something
    /// has to reach the end state now - a save, a test, or a frontend with no
    /// pump.
    pub fn drain_all(&mut self) -> Vec<NarrationBeat> {
        self.elapsed_secs = 0.0;
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
        assert!(staged.advance(0.1).is_empty());
        assert!(staged.advance(0.1).is_empty());
        let released: Vec<_> = staged.advance(0.1).into_iter().map(|b| b.text).collect();
        assert_eq!(released, vec!["first".to_string()]);
        assert!(staged.advance(0.1).is_empty());
        let released: Vec<_> = staged.advance(0.1).into_iter().map(|b| b.text).collect();
        assert_eq!(released, vec!["second".to_string()]);
        assert!(staged.is_empty());
        assert!(staged.advance(1.0).is_empty());
    }

    #[test]
    fn a_slow_frame_releases_every_beat_it_covers() {
        let mut staged = StagedNarration::default();
        staged.push_secs(0.3, "first");
        staged.push_secs(0.2, "second");
        let released: Vec<_> = staged.advance(1.0).into_iter().map(|b| b.text).collect();
        assert_eq!(released, vec!["first".to_string(), "second".to_string()]);
        assert!(staged.is_empty());
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
