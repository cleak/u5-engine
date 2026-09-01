//! Post-victory endgame cinematic state machine per
//! `systems/endgame.md`. The framer walks through the six fixed
//! `END.DAT` narrative windows, the late certificate rectangle
//! operation, the certificate body scroll, and the separate final
//! report panel (`endgame.md §9`). Narrative panels advance by key; the rectangle operation is
//! a display event between the last narrative panel and certificate.
//!
//! This is the page-flip presenter only; party restoration, throne-
//! room tableau setup, and the binary "did you bring the box?"
//! confirmation flow stay in [`crate::endgame`].

/// One presentation step in the post-victory cinematic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EndgameCinematicStep {
    /// No cinematic active.
    #[default]
    Inactive,
    /// One Lord British victory-rite message from `ENDMSG.DAT` is on screen,
    /// indexed `0..rite_message_count`.
    RiteMessage(u8),
    /// Throne-room tableau is on screen; awaiting first key to begin
    /// the narrative scroll.
    ThroneTableau,
    /// One of the six fixed `END.DAT` narrative windows is on screen,
    /// indexed `0..6`. Pressing a key advances to the next.
    NarrativeWindow(u8),
    /// `endgame.md §7.1` full-screen fade to black, entering the final
    /// narrative presentation. Not an intro-style timed column wipe and
    /// not a no-op: see [`ENDGAME_FADE_TO_BLACK_RECT`].
    FadeToBlack,
    /// Certificate body scroll is on screen (`endgame.md §9`).
    Certificate,
    /// `endgame.md §9`: after the certificate body the scroll clears or
    /// advances to a separate final report panel carrying the elapsed
    /// campaign time and the closing Origin report line.
    FinalReport,
    /// Cinematic complete; the engine remains on the terminal final panel.
    Finished,
}

/// Total number of `END.DAT` narrative windows (per `endgame.md` §8).
pub const ENDGAME_NARRATIVE_WINDOW_COUNT: u8 = 6;
/// `endgame.md §5`: shared world-animation ticks between the first rite
/// page and the fixed `He says:` lead-in for the second page.
pub const ENDGAME_RITE_LEAD_IN_PAUSE_TICKS: u8 = 40;
pub const ENDGAME_RITE_LEAD_IN: &str = "He says:";
/// `endgame.md §7.1` full-screen fade to black, inclusive
/// `(0, 0)..(319, 199)`.
///
/// `cleak/u5-spec#53` retracted the earlier reading that this rectangle
/// "produces no visible change at all" and could be omitted. It is a
/// two-part beat and both halves are load-bearing:
///
/// 1. the victory sequence points the render target at the hidden
///    surface, sets the colour to palette index 0, releases the active
///    graphics asset segment, fills this rectangle, and points back at
///    the visible page — invisible in isolation, which is what made the
///    earlier trace mis-read it;
/// 2. control passes straight into the final narrative presentation
///    helper, which — after acquiring its three presentation resources
///    and before drawing anything else, on an unconditional
///    straight-line path — dissolves this same rectangle from the
///    hidden surface to the visible page.
///
/// Net player-visible effect: the whole screen dissolves to black, in
/// the driver's pseudo-random per-pixel order, **before the first
/// `END.DAT` window**. Skipping the fill would dissolve stale offscreen
/// content onto the screen instead.
///
/// Both calls are blocking and self-paced: no tick pacing, no title
/// tick, and no keyboard poll anywhere in the beat, so it cannot be
/// interrupted. `§8` records the sequencing consequence — this happens
/// once, before window one, and the six windows themselves have no
/// page-in rectangle of their own.
pub const ENDGAME_FADE_TO_BLACK_RECT: (u16, u16, u16, u16) = (0, 0, 319, 199);

/// Run state for the cinematic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EndgameCinematic {
    pub step: EndgameCinematicStep,
    /// Number of Lord British victory-rite message records to page through
    /// before the fixed final narrative windows.
    pub rite_message_count: u8,
    /// Number of keystrokes consumed since the cinematic began. Useful
    /// for testing and for the UI's page indicator.
    pub keystrokes: u32,
    /// Pending `§7.1` full-screen fade-to-black beat, entering the
    /// final narrative presentation.
    pub fade_to_black_rect: Option<(u16, u16, u16, u16)>,
    /// Display-driven pause remaining after rite page zero. Input is not
    /// sampled while this blocking helper is active.
    pub rite_pause_ticks_remaining: u8,
    /// The second rite page alone carries the fixed lead-in.
    pub rite_lead_in_visible: bool,
}

impl EndgameCinematic {
    /// Begin the cinematic. Returns the new state.
    pub fn start() -> Self {
        Self::start_with_rite_messages(0)
    }

    /// Begin the cinematic after `rite_message_count` Lord British message
    /// records. A count of zero preserves the legacy direct-to-tableau path.
    pub fn start_with_rite_messages(rite_message_count: u8) -> Self {
        Self {
            step: if rite_message_count == 0 {
                EndgameCinematicStep::ThroneTableau
            } else {
                EndgameCinematicStep::RiteMessage(0)
            },
            rite_message_count,
            keystrokes: 0,
            fade_to_black_rect: None,
            rite_pause_ticks_remaining: if rite_message_count > 1 {
                ENDGAME_RITE_LEAD_IN_PAUSE_TICKS
            } else {
                0
            },
            rite_lead_in_visible: false,
        }
    }

    /// Advance one display tick of the first-to-second rite-page pause. When
    /// the shared world-animation gate is disabled, the helper skips the loop
    /// and publishes the second page immediately.
    pub fn advance_rite_pause_tick(&mut self, world_animation_enabled: bool) -> bool {
        if !matches!(self.step, EndgameCinematicStep::RiteMessage(0))
            || self.rite_pause_ticks_remaining == 0
        {
            return false;
        }
        if world_animation_enabled && self.rite_pause_ticks_remaining > 1 {
            self.rite_pause_ticks_remaining -= 1;
            return true;
        }
        self.rite_pause_ticks_remaining = 0;
        self.rite_lead_in_visible = true;
        self.step = EndgameCinematicStep::RiteMessage(1);
        true
    }

    /// Advance one step. Returns the new step. Pressing a key while
    /// already `Finished` is a no-op.
    pub fn advance(&mut self) -> EndgameCinematicStep {
        if matches!(self.step, EndgameCinematicStep::RiteMessage(0))
            && self.rite_pause_ticks_remaining != 0
        {
            return self.step;
        }
        self.keystrokes = self.keystrokes.saturating_add(1);
        self.fade_to_black_rect = None;
        self.rite_lead_in_visible = false;
        self.step = match self.step {
            EndgameCinematicStep::Inactive | EndgameCinematicStep::Finished => self.step,
            EndgameCinematicStep::RiteMessage(idx) => {
                let next = idx.saturating_add(1);
                if next < self.rite_message_count {
                    EndgameCinematicStep::RiteMessage(next)
                } else {
                    EndgameCinematicStep::ThroneTableau
                }
            }
            // `§7.1` / `§8`: the fade to black runs once, on the way
            // out of the throne tableau and into the final narrative
            // presentation - before window one, not after window six.
            EndgameCinematicStep::ThroneTableau => {
                self.fade_to_black_rect = Some(ENDGAME_FADE_TO_BLACK_RECT);
                EndgameCinematicStep::FadeToBlack
            }
            // `§7.1`: the fade samples no input, so a keystroke does
            // not carry it - only `advance_fade_to_black` does. A key
            // arriving mid-beat stays queued for the window that
            // follows.
            EndgameCinematicStep::FadeToBlack => self.step,
            EndgameCinematicStep::NarrativeWindow(idx) => {
                let next = idx.saturating_add(1);
                if next >= ENDGAME_NARRATIVE_WINDOW_COUNT {
                    // `§8`: the windows have no page-in rectangle of
                    // their own, and none is issued on the way to the
                    // certificate either.
                    EndgameCinematicStep::Certificate
                } else {
                    EndgameCinematicStep::NarrativeWindow(next)
                }
            }
            EndgameCinematicStep::Certificate => EndgameCinematicStep::FinalReport,
            EndgameCinematicStep::FinalReport => EndgameCinematicStep::Finished,
        };
        self.step
    }

    /// Run the `§7.1` fade-to-black beat. It consumes no keystroke -
    /// the beat samples no input at all - so the caller drives it from
    /// the display pump, and both blocking halves complete within it.
    pub fn advance_fade_to_black(&mut self) -> bool {
        if !matches!(self.step, EndgameCinematicStep::FadeToBlack)
            || self.fade_to_black_rect.is_none()
        {
            return false;
        }
        self.fade_to_black_rect = None;
        self.step = EndgameCinematicStep::NarrativeWindow(0);
        true
    }

    /// Returns `true` when the cinematic has presented every screen
    /// and the caller should hold the terminal final panel.
    pub fn is_finished(&self) -> bool {
        matches!(self.step, EndgameCinematicStep::Finished)
    }

    /// Returns the human-readable banner the UI should display for
    /// the current step. Tests use this to assert pacing without
    /// depending on the full rendered text of each panel.
    pub fn banner_label(&self) -> &'static str {
        match self.step {
            EndgameCinematicStep::Inactive => "(no cinematic)",
            EndgameCinematicStep::RiteMessage(_) => "Lord British rite",
            EndgameCinematicStep::ThroneTableau => "Throne-room tableau",
            EndgameCinematicStep::NarrativeWindow(0) => "Return-home arc (1)",
            EndgameCinematicStep::NarrativeWindow(1) => "Return-home arc (2)",
            EndgameCinematicStep::NarrativeWindow(2) => "Return-home arc (3)",
            EndgameCinematicStep::NarrativeWindow(3) => "Blackthorn judgment (1)",
            EndgameCinematicStep::NarrativeWindow(4) => "Blackthorn judgment (2)",
            EndgameCinematicStep::NarrativeWindow(5) => "Blackthorn judgment (3)",
            EndgameCinematicStep::NarrativeWindow(_) => "Narrative window",
            EndgameCinematicStep::FadeToBlack => "Fade to black",
            EndgameCinematicStep::Certificate => "Quest certificate",
            EndgameCinematicStep::FinalReport => "Final report panel",
            EndgameCinematicStep::Finished => "Cinematic finished",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `§7.1`: leaving the throne tableau always runs the fade to black
    /// first, and it clears without a keystroke.
    fn started_at_first_narrative_window() -> EndgameCinematic {
        let mut cin = EndgameCinematic::start();
        assert_eq!(cin.advance(), EndgameCinematicStep::FadeToBlack);
        assert_eq!(cin.fade_to_black_rect, Some(ENDGAME_FADE_TO_BLACK_RECT));
        let keystrokes = cin.keystrokes;
        assert!(cin.advance_fade_to_black());
        assert_eq!(cin.step, EndgameCinematicStep::NarrativeWindow(0));
        assert_eq!(cin.fade_to_black_rect, None);
        assert_eq!(cin.keystrokes, keystrokes, "the fade consumes no input");
        cin
    }

    #[test]
    fn start_initialises_to_throne_tableau_with_zero_keystrokes() {
        let cin = EndgameCinematic::start();
        assert_eq!(cin.step, EndgameCinematicStep::ThroneTableau);
        assert_eq!(cin.keystrokes, 0);
        assert_eq!(cin.fade_to_black_rect, None);
        assert!(!cin.is_finished());
    }

    #[test]
    fn the_fade_to_black_runs_once_before_the_first_narrative_window() {
        // `cleak/u5-spec#53` retraction: the full-screen rectangle is
        // NOT after window six, and it is NOT omittable. It is the fade
        // to black entering the final narrative presentation.
        let mut cin = started_at_first_narrative_window();
        for expected in 1..ENDGAME_NARRATIVE_WINDOW_COUNT {
            assert_eq!(
                cin.advance(),
                EndgameCinematicStep::NarrativeWindow(expected)
            );
            assert_eq!(
                cin.fade_to_black_rect, None,
                "`§8`: the six windows have no page-in rectangle of their own"
            );
        }
        // Window six goes straight to the certificate: no second
        // full-screen operation on the way out.
        assert_eq!(cin.advance(), EndgameCinematicStep::Certificate);
        assert_eq!(cin.fade_to_black_rect, None);
    }

    #[test]
    fn the_fade_to_black_covers_the_whole_inclusive_surface() {
        assert_eq!(ENDGAME_FADE_TO_BLACK_RECT, (0, 0, 319, 199));
        let mut cin = EndgameCinematic::start();
        assert_eq!(cin.advance(), EndgameCinematicStep::FadeToBlack);
        assert_eq!(cin.fade_to_black_rect, Some(ENDGAME_FADE_TO_BLACK_RECT));
        assert!(cin.advance_fade_to_black());
        assert!(!cin.advance_fade_to_black());
    }

    #[test]
    fn advance_walks_all_six_narrative_windows_in_order() {
        let mut cin = started_at_first_narrative_window();
        for expected in 1..ENDGAME_NARRATIVE_WINDOW_COUNT {
            assert_eq!(
                cin.advance(),
                EndgameCinematicStep::NarrativeWindow(expected)
            );
            assert_eq!(cin.fade_to_black_rect, None);
        }
    }

    #[test]
    fn ordinary_narrative_windows_do_not_install_intro_style_page_wipes() {
        let mut cin = started_at_first_narrative_window();
        for _ in 1..ENDGAME_NARRATIVE_WINDOW_COUNT {
            assert!(matches!(
                cin.advance(),
                EndgameCinematicStep::NarrativeWindow(_)
            ));
            assert_eq!(cin.fade_to_black_rect, None);
        }
    }

    #[test]
    fn certificate_then_final_report_then_finished() {
        let mut cin = started_at_first_narrative_window();
        for _ in 1..ENDGAME_NARRATIVE_WINDOW_COUNT {
            cin.advance();
        }
        assert_eq!(cin.advance(), EndgameCinematicStep::Certificate);
        assert_eq!(cin.advance(), EndgameCinematicStep::FinalReport);
        assert_eq!(cin.advance(), EndgameCinematicStep::Finished);
        assert!(cin.is_finished());
    }

    #[test]
    fn advance_after_finished_is_noop() {
        let mut cin = EndgameCinematic::start();
        for _ in 0..16 {
            cin.advance();
            cin.advance_fade_to_black();
        }
        let before = cin.step;
        assert_eq!(cin.advance(), before);
    }

    #[test]
    fn keystroke_counter_records_each_advance() {
        let mut cin = EndgameCinematic::start();
        cin.advance();
        cin.advance();
        assert_eq!(cin.keystrokes, 2);
    }

    #[test]
    fn banner_label_named_for_every_step_type() {
        let mut cin = EndgameCinematic::start();
        assert_eq!(cin.banner_label(), "Throne-room tableau");
        cin.advance();
        assert_eq!(cin.banner_label(), "Fade to black");
        assert!(cin.advance_fade_to_black());
        assert_eq!(cin.banner_label(), "Return-home arc (1)");
        for _ in 1..ENDGAME_NARRATIVE_WINDOW_COUNT {
            cin.advance();
        }
        cin.advance();
        assert_eq!(cin.banner_label(), "Quest certificate");
        cin.advance();
        assert_eq!(cin.banner_label(), "Final report panel");
        cin.advance();
        assert_eq!(cin.banner_label(), "Cinematic finished");
    }

    #[test]
    fn rite_messages_page_before_the_tableau_and_the_fade() {
        let mut cin = EndgameCinematic::start_with_rite_messages(3);
        assert_eq!(cin.step, EndgameCinematicStep::RiteMessage(0));
        assert_eq!(cin.advance(), EndgameCinematicStep::RiteMessage(0));
        assert_eq!(cin.keystrokes, 0);
        for _ in 0..ENDGAME_RITE_LEAD_IN_PAUSE_TICKS - 1 {
            assert!(cin.advance_rite_pause_tick(true));
            assert_eq!(cin.step, EndgameCinematicStep::RiteMessage(0));
        }
        assert!(cin.advance_rite_pause_tick(true));
        assert_eq!(cin.step, EndgameCinematicStep::RiteMessage(1));
        assert!(cin.rite_lead_in_visible);
        assert_eq!(cin.advance(), EndgameCinematicStep::RiteMessage(2));
        assert!(!cin.rite_lead_in_visible);
        assert_eq!(cin.advance(), EndgameCinematicStep::ThroneTableau);
        assert_eq!(cin.fade_to_black_rect, None);
        assert_eq!(cin.advance(), EndgameCinematicStep::FadeToBlack);
    }

    #[test]
    fn disabled_world_animation_skips_the_rite_pause_loop() {
        let mut cin = EndgameCinematic::start_with_rite_messages(7);
        assert!(cin.advance_rite_pause_tick(false));
        assert_eq!(cin.step, EndgameCinematicStep::RiteMessage(1));
        assert_eq!(cin.rite_pause_ticks_remaining, 0);
        assert!(cin.rite_lead_in_visible);
    }
}
