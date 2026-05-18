//! Post-victory endgame cinematic state machine per
//! `systems/endgame.md`. The framer walks through the six fixed
//! `END.DAT` narrative windows, the certificate scroll, and the
//! Origin attribution closer with one keystroke between each panel.
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
    /// Certificate scroll is on screen.
    Certificate,
    /// Origin attribution closer is on screen.
    OriginCloser,
    /// Cinematic complete; the engine remains on the terminal final panel.
    Finished,
}

/// Total number of `END.DAT` narrative windows (per `endgame.md` §8).
pub const ENDGAME_NARRATIVE_WINDOW_COUNT: u8 = 6;

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
        }
    }

    /// Advance one step. Returns the new step. Pressing a key while
    /// already `Finished` is a no-op.
    pub fn advance(&mut self) -> EndgameCinematicStep {
        self.keystrokes = self.keystrokes.saturating_add(1);
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
            EndgameCinematicStep::ThroneTableau => EndgameCinematicStep::NarrativeWindow(0),
            EndgameCinematicStep::NarrativeWindow(idx) => {
                let next = idx.saturating_add(1);
                if next >= ENDGAME_NARRATIVE_WINDOW_COUNT {
                    EndgameCinematicStep::Certificate
                } else {
                    EndgameCinematicStep::NarrativeWindow(next)
                }
            }
            EndgameCinematicStep::Certificate => EndgameCinematicStep::OriginCloser,
            EndgameCinematicStep::OriginCloser => EndgameCinematicStep::Finished,
        };
        self.step
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
            EndgameCinematicStep::Certificate => "Quest certificate",
            EndgameCinematicStep::OriginCloser => "Origin closer",
            EndgameCinematicStep::Finished => "Cinematic finished",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_initialises_to_throne_tableau_with_zero_keystrokes() {
        let cin = EndgameCinematic::start();
        assert_eq!(cin.step, EndgameCinematicStep::ThroneTableau);
        assert_eq!(cin.keystrokes, 0);
        assert!(!cin.is_finished());
    }

    #[test]
    fn advance_walks_all_six_narrative_windows_in_order() {
        let mut cin = EndgameCinematic::start();
        assert_eq!(cin.advance(), EndgameCinematicStep::NarrativeWindow(0));
        for expected in 1..ENDGAME_NARRATIVE_WINDOW_COUNT {
            assert_eq!(
                cin.advance(),
                EndgameCinematicStep::NarrativeWindow(expected)
            );
        }
    }

    #[test]
    fn advance_transitions_to_certificate_after_six_windows() {
        let mut cin = EndgameCinematic::start();
        for _ in 0..(1 + ENDGAME_NARRATIVE_WINDOW_COUNT) {
            cin.advance();
        }
        assert_eq!(cin.step, EndgameCinematicStep::Certificate);
    }

    #[test]
    fn certificate_then_origin_closer_then_finished() {
        let mut cin = EndgameCinematic::start();
        for _ in 0..(1 + ENDGAME_NARRATIVE_WINDOW_COUNT) {
            cin.advance();
        }
        assert_eq!(cin.advance(), EndgameCinematicStep::OriginCloser);
        assert_eq!(cin.advance(), EndgameCinematicStep::Finished);
        assert!(cin.is_finished());
    }

    #[test]
    fn advance_after_finished_is_noop() {
        let mut cin = EndgameCinematic::start();
        for _ in 0..16 {
            cin.advance();
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
        assert_eq!(cin.banner_label(), "Return-home arc (1)");
        for _ in 0..ENDGAME_NARRATIVE_WINDOW_COUNT {
            cin.advance();
        }
        assert_eq!(cin.banner_label(), "Quest certificate");
        cin.advance();
        assert_eq!(cin.banner_label(), "Origin closer");
        cin.advance();
        assert_eq!(cin.banner_label(), "Cinematic finished");
    }
}
