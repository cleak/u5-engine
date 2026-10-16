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

/// The longest gap between one beat's release and a chained push's first
/// look that still counts as the same presentation stepping on. A frontend
/// handoff is a frame or two plus one world-tick pump interval (about 55 ms);
/// a quarter second leaves room for a slow host frame and is still shorter
/// than any gap a player could put between two presentations.
pub const CHAIN_HANDOFF_LIMIT_SECS: f64 = 0.25;

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
    /// The text is written against a row the previous print left open
    /// mid-line, so its first feed closes that row rather than blanking a
    /// fresh one, and no fresh row is drawn while it waits. Stated by the
    /// producer: the row state is not inferable from the text.
    pub continues_open_row: bool,
    /// The wait runs from when the previous released beat was *due* rather
    /// than from the frontend's first look at this one. For a presentation
    /// loop whose step pushes its own next hold - the endgame tableau walk -
    /// the frames between a release and the next push are not time the
    /// original spends; see [`StagedNarration::push_ticks_chained`].
    pub chained: bool,
    /// Counted world steps the hold before this beat stands for
    /// (`karma.md §7`: every presentation interval is "a counted repetition
    /// of one world step plus one one-tick delay request"). On release this
    /// is the number of those steps not yet handed out by
    /// [`StagedNarration::take_due_world_steps`].
    pub world_steps: u16,
}

/// The queue plus how far into the current beat's wait we are.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StagedNarration {
    beats: VecDeque<NarrationBeat>,
    /// Clock reading at which the head beat is due. `None` until the first
    /// advance after the head becomes current, which is what makes the queue
    /// start its wait from the frontend's clock rather than from zero.
    due_at_secs: Option<f64>,
    /// When the most recently released beat was due, kept after the queue
    /// empties so a [`NarrationBeat::chained`] push can continue from it.
    last_due_secs: Option<f64>,
    /// Counted world steps of the head beat already handed out.
    head_steps_taken: u16,
}

impl StagedNarration {
    pub fn is_empty(&self) -> bool {
        self.beats.is_empty()
    }

    /// The queued beats, head first, without advancing anything.
    pub fn beats(&self) -> impl Iterator<Item = &NarrationBeat> {
        self.beats.iter()
    }

    pub fn push_secs(&mut self, delay_secs: f64, text: impl Into<String>) {
        self.beats.push_back(NarrationBeat {
            delay_secs,
            text: text.into(),
            continues_open_row: false,
            chained: false,
            world_steps: 0,
        });
    }

    /// Push a beat written against a row the previous print left open; see
    /// [`NarrationBeat::continues_open_row`].
    pub fn push_secs_on_open_row(&mut self, delay_secs: f64, text: impl Into<String>) {
        self.beats.push_back(NarrationBeat {
            delay_secs,
            text: text.into(),
            continues_open_row: true,
            chained: false,
            world_steps: 0,
        });
    }

    /// Whether the next beat to print continues an open row.
    pub fn head_continues_open_row(&self) -> bool {
        self.beats
            .front()
            .is_some_and(|beat| beat.continues_open_row)
    }

    /// Push a beat whose delay is a published BIOS-tick count.
    pub fn push_ticks(&mut self, delay_bios_ticks: u16, text: impl Into<String>) {
        self.push_secs(bios_ticks_secs(delay_bios_ticks), text);
    }

    /// Push a pure hold of a published BIOS-tick count whose wait continues
    /// from the previous beat's due time.
    ///
    /// `audio.md §8.7`: each tableau step is "two shared world-animation
    /// ticks, the short two-part sting ..., then three more", and the next
    /// step follows directly - the train's stepping time is "the sting count
    /// times five BIOS ticks". The engine's step runs on a frame after the
    /// previous hold releases, and an ordinary push would start its wait
    /// from the first look after that, so every step paid the frontend's
    /// handoff on top of its five ticks. Measured 2026-10-15 over
    /// `doom-endgame-audio`: a 350 ms median sting spacing against the
    /// original's 280 ms.
    ///
    /// The chain only holds across a handoff of at most
    /// [`CHAIN_HANDOFF_LIMIT_SECS`]. A previous due time further back than
    /// that belongs to some earlier sequence, and anchoring to it would
    /// collapse the new hold into a catch-up burst, so the wait then starts
    /// at the first look as usual.
    pub fn push_ticks_chained(&mut self, delay_bios_ticks: u16) {
        self.beats.push_back(NarrationBeat {
            delay_secs: bios_ticks_secs(delay_bios_ticks),
            text: String::new(),
            continues_open_row: false,
            chained: true,
            world_steps: 0,
        });
    }

    /// Push a beat whose hold is `world_steps` counted world steps,
    /// spread across `delay_bios_ticks` of wall clock.
    pub fn push_ticks_counting_world_steps(
        &mut self,
        delay_bios_ticks: u16,
        world_steps: u16,
        text: impl Into<String>,
    ) {
        self.beats.push_back(NarrationBeat {
            delay_secs: bios_ticks_secs(delay_bios_ticks),
            text: text.into(),
            continues_open_row: false,
            chained: false,
            world_steps,
        });
    }

    /// Counted world steps of the head beat that have come due by
    /// `now_secs`, spread evenly across its hold; each step is handed out
    /// once. The rest are carried on the beat when it is released.
    pub fn take_due_world_steps(&mut self, now_secs: f64) -> u16 {
        let Some(beat) = self.beats.front() else {
            return 0;
        };
        if beat.world_steps == 0 {
            return 0;
        }
        let (delay_secs, world_steps) = (beat.delay_secs, beat.world_steps);
        let due = self.head_due_secs(now_secs);
        let started = due - delay_secs;
        let target = if delay_secs <= 0.0 || now_secs >= due {
            world_steps
        } else {
            let fraction = ((now_secs - started) / delay_secs).clamp(0.0, 1.0);
            ((f64::from(world_steps) * fraction).floor() as u16).min(world_steps)
        };
        let owed = target.saturating_sub(self.head_steps_taken);
        self.head_steps_taken = self.head_steps_taken.max(target);
        owed
    }

    /// When the head beat comes due, fixing it on the first look. A
    /// [`NarrationBeat::chained`] head continues from the previous release's
    /// due time when that handoff is short enough; anything else starts its
    /// wait at `now_secs`. Shared by release and world-step accounting so a
    /// chained beat that also counts world steps spreads them over the same
    /// window it is released on. Callers check the queue is non-empty.
    fn head_due_secs(&mut self, now_secs: f64) -> f64 {
        let beat = self.beats.front().expect("head beat present");
        let anchor = match self.last_due_secs {
            Some(last) if beat.chained && now_secs - last <= CHAIN_HANDOFF_LIMIT_SECS => last,
            _ => now_secs,
        };
        let delay_secs = beat.delay_secs;
        *self
            .due_at_secs
            .get_or_insert_with(|| anchor + delay_secs)
    }

    pub fn clear(&mut self) {
        self.beats.clear();
        self.due_at_secs = None;
        self.last_due_secs = None;
        self.head_steps_taken = 0;
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
        while !self.beats.is_empty() {
            let due = self.head_due_secs(now_secs);
            if now_secs < due {
                break;
            }
            let mut beat = self.beats.pop_front().expect("checked above");
            beat.world_steps = beat
                .world_steps
                .saturating_sub(std::mem::take(&mut self.head_steps_taken));
            // The next beat's wait starts when this one was *due*, not when
            // the frontend noticed: a late frame must not stretch the
            // sequence beat by beat.
            self.due_at_secs = self.beats.front().map(|next| due + next.delay_secs);
            self.last_due_secs = Some(due);
            released.push(beat);
        }
        released
    }

    /// Release every remaining beat at once, in order. Used when something
    /// has to reach the end state now - a save, a test, or a frontend with no
    /// pump.
    pub fn drain_all(&mut self) -> Vec<NarrationBeat> {
        self.due_at_secs = None;
        self.last_due_secs = None;
        let taken = std::mem::take(&mut self.head_steps_taken);
        let mut beats: Vec<NarrationBeat> = self.beats.drain(..).collect();
        if let Some(head) = beats.first_mut() {
            head.world_steps = head.world_steps.saturating_sub(taken);
        }
        beats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counted_world_steps_are_handed_out_once_across_the_hold() {
        let mut staged = StagedNarration::default();
        staged.push_ticks_counting_world_steps(10, 10, "after");
        let hold = bios_ticks_secs(10);
        assert_eq!(staged.take_due_world_steps(0.0), 0);
        assert_eq!(staged.take_due_world_steps(hold * 0.35), 3);
        assert_eq!(staged.take_due_world_steps(hold * 0.35), 0);
        assert!(staged.advance_to(hold * 0.5).is_empty());
        let released = staged.advance_to(hold);
        assert_eq!(released.len(), 1);
        // The steps not yet handed out travel with the released beat.
        assert_eq!(released[0].world_steps, 7);

        staged.push_ticks_counting_world_steps(10, 10, "flushed");
        assert_eq!(staged.take_due_world_steps(5.0), 0);
        assert_eq!(staged.take_due_world_steps(5.0 + hold * 0.25), 2);
        assert_eq!(staged.drain_all()[0].world_steps, 8);
    }

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

#[cfg(test)]
mod rescue_pacing_tests {
    use super::*;

    /// `blackthorn.md §7`'s rescue holds, as arithmetic over the queue.
    ///
    /// The paired scenarios cannot see every one of these individually: a
    /// capture only shows a difference when a hold is long enough to move a
    /// beat across the sample. `stonegate-death-hold` sees the 26.5 s death
    /// sweep at `t10` and the 11.18 s envelope sequence at `t32`, and cannot
    /// see the 1.72 s paired flash at `t50` at all. This asserts the queue
    /// does what those constants say, which is a different question from
    /// whether the total matches the original.
    #[test]
    fn a_queued_hold_delays_every_beat_behind_it() {
        let mut queue = StagedNarration::default();
        queue.push_ticks(10, "first");
        queue.push_ticks(31, "second");

        // Nothing is due before the first wait completes. The first look sets
        // the epoch the head's wait runs from.
        assert!(queue.advance_to(0.0).is_empty());
        let first_due = bios_ticks_secs(10);
        assert!(queue.advance_to(first_due - 0.001).is_empty());

        let released = queue.advance_to(first_due);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].text, "first");

        // The second beat's own wait starts from when the first was *due*,
        // so a hold in front of it delays it by exactly its own ticks.
        let second_due = first_due + bios_ticks_secs(31);
        assert!(queue.advance_to(second_due - 0.001).is_empty());
        let released = queue.advance_to(second_due);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].text, "second");
        assert!(queue.is_empty());
    }

    /// A pure hold carries no text and still spends its time, which is how
    /// the death sweep and the envelope sequence are staged: both are
    /// durations with nothing to print.
    #[test]
    fn a_pure_hold_spends_its_time_without_printing() {
        let mut queue = StagedNarration::default();
        queue.push_ticks(483, "");
        queue.push_ticks(0, "after the sweep");

        // The head's wait starts at the frontend's *first* look, not at zero:
        // `due_at_secs` is filled in on the first `advance_to`. That is the
        // absolute-clock design - a queue staged during one frame must not
        // lose the time before the next one asks.
        let epoch = 100.0;
        assert!(queue.advance_to(epoch).is_empty());
        let hold = epoch + bios_ticks_secs(483);
        assert!(queue.advance_to(hold - 0.001).is_empty());
        let released = queue.advance_to(hold);
        assert_eq!(released.len(), 2);
        assert_eq!(released[0].text, "");
        assert_eq!(released[1].text, "after the sweep");
    }

    /// A chained hold pushed a few frames after the previous one released
    /// runs from that release's due time, so a presentation stepping on
    /// frames keeps exact five-tick spacing.
    #[test]
    fn a_chained_hold_continues_from_the_previous_due_time() {
        let mut queue = StagedNarration::default();
        queue.push_ticks_chained(5);
        assert!(queue.advance_to(10.0).is_empty());
        let first_due = 10.0 + bios_ticks_secs(5);
        assert_eq!(queue.advance_to(first_due).len(), 1);

        // The frontend steps 75 ms later and only then pushes the next hold.
        queue.push_ticks_chained(5);
        let handoff = first_due + 0.075;
        assert!(queue.advance_to(handoff).is_empty());
        let second_due = first_due + bios_ticks_secs(5);
        assert!(queue.advance_to(second_due - 0.001).is_empty());
        assert_eq!(queue.advance_to(second_due).len(), 1);
    }

    /// A chain anchor older than the handoff limit is an earlier sequence's, not a
    /// handoff; the hold then waits its full time from the first look.
    #[test]
    fn a_stale_chain_anchor_is_ignored() {
        let mut queue = StagedNarration::default();
        queue.push_ticks(1, "earlier");
        assert!(queue.advance_to(0.0).is_empty());
        assert_eq!(queue.advance_to(1.0).len(), 1);

        queue.push_ticks_chained(5);
        assert!(queue.advance_to(30.0).is_empty());
        assert!(
            queue
                .advance_to(30.0 + bios_ticks_secs(5) - 0.001)
                .is_empty()
        );
        assert_eq!(queue.advance_to(30.0 + bios_ticks_secs(5)).len(), 1);
    }

    /// An ordinary push never chains, however recent the last release.
    #[test]
    fn an_unchained_push_waits_from_the_first_look() {
        let mut queue = StagedNarration::default();
        queue.push_ticks(5, "");
        assert!(queue.advance_to(0.0).is_empty());
        let first_due = bios_ticks_secs(5);
        assert_eq!(queue.advance_to(first_due).len(), 1);
        queue.push_ticks(5, "");
        let look = first_due + 0.075;
        assert!(queue.advance_to(look).is_empty());
        assert!(
            queue
                .advance_to(look + bios_ticks_secs(5) - 0.001)
                .is_empty()
        );
        assert_eq!(queue.advance_to(look + bios_ticks_secs(5)).len(), 1);
    }
}
