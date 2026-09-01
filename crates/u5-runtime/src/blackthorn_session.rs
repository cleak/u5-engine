//! Blackthorn audience challenge state machine per
//! `systems/blackthorn.md` §4. Wraps the per-prompt helpers in
//! [`crate::blackthorn`] into the published one-shrine, up-to-four-
//! prompt interactive flow.

use crate::blackthorn::{
    BLACKTHORN_CHALLENGE_PROMPT_COUNT, BlackthornChallengeWording,
    blackthorn_challenge_answer_matches, blackthorn_challenge_limited_input,
    blackthorn_challenge_wording, blackthorn_shrine_mantra,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlackthornChallengePhase {
    #[default]
    AwaitingAudience,
    PresentingPrompt {
        ordinal: u8,
    },
    Punished {
        failed_ordinal: u8,
    },
    Survived,
    Aborted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornChallengeOutcome {
    PromptPresented { ordinal: u8, prompt: &'static str },
    Correct { ordinal: u8 },
    Wrong { ordinal: u8, expected: &'static str },
    Survived,
    AlreadyPunished,
    AlreadySurvived,
    AlreadyAborted,
}

/// `blackthorn.md §4` wrong-answer escalation ladder. "The first wrong
/// answer produces a threat naming the companion at risk. Later wrong
/// answers stamp a tile into the cutscene map, and the fourth wrong
/// answer **kills** the named companion with the pendulum-blade
/// narration."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackthornWrongEscalation {
    /// First wrong answer — a threat naming the companion at risk.
    Threat,
    /// Second and third wrong answers — stamp a tile into the cutscene
    /// map and re-ask.
    TileStamp,
    /// Fourth wrong answer — the named companion is killed.
    Kill,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BlackthornChallenge {
    pub phase: BlackthornChallengePhase,
    /// `blackthorn.md §3` step 2 / §4: "The shrine index is fixed
    /// before the loop starts and never changes inside it." The
    /// audience setup picks it by scanning the eight shrine ruin flags
    /// for the first that is exactly clear.
    pub shrine_index: u8,
    pub correct_count: u8,
    /// `blackthorn.md §4`: how many wrong answers have been given, which
    /// is what selects the escalation step.
    pub wrong_count: u8,
}

impl BlackthornChallenge {
    /// Interrogate the first shrine. Callers that have run the
    /// `blackthorn.md §3` step-2 shrine-ruin-flag scan should use
    /// [`BlackthornChallenge::for_shrine`] with the scanned index.
    pub fn new() -> Self {
        Self::for_shrine(0)
    }

    /// `blackthorn.md §4`: fix the interrogated shrine before the loop
    /// starts. The index never changes inside the loop.
    pub fn for_shrine(shrine_index: u8) -> Self {
        Self {
            phase: BlackthornChallengePhase::AwaitingAudience,
            shrine_index,
            correct_count: 0,
            wrong_count: 0,
        }
    }

    /// `blackthorn.md §4`: the virtue name of the interrogated shrine.
    /// It is the same on all four prompts.
    pub fn shrine_virtue(&self) -> &'static str {
        blackthorn_shrine_mantra(self.shrine_index)
            .map(|(virtue, _)| virtue)
            .unwrap_or("Virtue")
    }

    /// `blackthorn.md §4`: "The expected answer is the selected
    /// shrine's mantra, and it is the same on all four prompts."
    pub fn expected_mantra(&self) -> &'static str {
        blackthorn_shrine_mantra(self.shrine_index)
            .map(|(_, mantra)| mantra)
            .unwrap_or("")
    }

    /// `blackthorn.md §4`: which of the four escalating wordings the
    /// current prompt uses. The ordinal selects wording only.
    pub fn current_wording(&self) -> Option<BlackthornChallengeWording> {
        let BlackthornChallengePhase::PresentingPrompt { ordinal } = self.phase else {
            return None;
        };
        blackthorn_challenge_wording(ordinal)
    }

    /// `blackthorn.md §4`: the escalation the most recent wrong answer
    /// produced, or `None` before any wrong answer.
    pub fn wrong_escalation(&self) -> Option<BlackthornWrongEscalation> {
        escalation_for_wrong_count(self.wrong_count)
    }

    /// Begin the challenge. Returns the first prompt.
    pub fn begin(&mut self) -> BlackthornChallengeOutcome {
        self.phase = BlackthornChallengePhase::PresentingPrompt { ordinal: 0 };
        BlackthornChallengeOutcome::PromptPresented {
            ordinal: 0,
            prompt: self.shrine_virtue(),
        }
    }

    /// `blackthorn.md §4`: compare the typed answer against the
    /// interrogated shrine's mantra.
    ///
    /// A correct answer resolves the interrogation — §4 makes it "ruin
    /// that shrine", debit moral standing, and decide a companion's
    /// fate, all once — so the loop does not ask again. A wrong answer
    /// escalates through [`BlackthornWrongEscalation`], re-asking the
    /// same question with the next wording until the fourth wrong
    /// answer punishes.
    ///
    /// The "wrong answer, when few companions remain, ends the
    /// interrogation" branch depends on live party state, so the caller
    /// owns it through [`BlackthornChallenge::abort`].
    pub fn submit(&mut self, typed: &str) -> BlackthornChallengeOutcome {
        match self.phase {
            BlackthornChallengePhase::Punished { .. } => {
                BlackthornChallengeOutcome::AlreadyPunished
            }
            BlackthornChallengePhase::Survived => BlackthornChallengeOutcome::AlreadySurvived,
            BlackthornChallengePhase::Aborted => BlackthornChallengeOutcome::AlreadyAborted,
            BlackthornChallengePhase::AwaitingAudience => self.begin(),
            BlackthornChallengePhase::PresentingPrompt { ordinal } => {
                let expected = self.expected_mantra();
                let typed = blackthorn_challenge_limited_input(typed);
                if blackthorn_challenge_answer_matches(&typed, expected) {
                    self.correct_count = self.correct_count.saturating_add(1);
                    self.phase = BlackthornChallengePhase::Survived;
                    BlackthornChallengeOutcome::Survived
                } else {
                    self.wrong_count = self.wrong_count.saturating_add(1);
                    let next_ordinal = ordinal + 1;
                    if (next_ordinal as usize) >= BLACKTHORN_CHALLENGE_PROMPT_COUNT {
                        self.phase = BlackthornChallengePhase::Punished {
                            failed_ordinal: ordinal,
                        };
                    } else {
                        self.phase = BlackthornChallengePhase::PresentingPrompt {
                            ordinal: next_ordinal,
                        };
                    }
                    BlackthornChallengeOutcome::Wrong { ordinal, expected }
                }
            }
        }
    }

    /// The current prompt as `(ordinal, virtue)`. The virtue is the
    /// interrogated shrine's and is identical on all four ordinals.
    pub fn current_prompt(&self) -> Option<(u8, &'static str)> {
        let BlackthornChallengePhase::PresentingPrompt { ordinal } = self.phase else {
            return None;
        };
        Some((ordinal, self.shrine_virtue()))
    }

    pub fn abort(&mut self) {
        self.phase = BlackthornChallengePhase::Aborted;
    }

    pub fn is_punished(&self) -> bool {
        matches!(self.phase, BlackthornChallengePhase::Punished { .. })
    }

    pub fn has_survived(&self) -> bool {
        matches!(self.phase, BlackthornChallengePhase::Survived)
    }
}

/// `blackthorn.md §4` escalation step for the `n`-th wrong answer
/// (`wrong_count` counts from one).
pub const fn escalation_for_wrong_count(wrong_count: u8) -> Option<BlackthornWrongEscalation> {
    Some(match wrong_count {
        0 => return None,
        1 => BlackthornWrongEscalation::Threat,
        2 | 3 => BlackthornWrongEscalation::TileStamp,
        _ => BlackthornWrongEscalation::Kill,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackthorn::BLACKTHORN_SHRINE_MANTRAS;

    #[test]
    fn begin_presents_first_prompt() {
        let mut c = BlackthornChallenge::new();
        let outcome = c.begin();
        assert!(matches!(
            outcome,
            BlackthornChallengeOutcome::PromptPresented { ordinal: 0, .. }
        ));
    }

    #[test]
    fn all_eight_shrines_are_live_and_answer_by_shrine_not_ordinal() {
        // `blackthorn.md §4`: "**The expected answer is the selected
        // shrine's mantra, and it is the same on all four prompts.**
        // All eight virtue/mantra pairs are live". The withdrawal box
        // retires the old "indexed by prompt ordinal" reading, which
        // this test previously encoded as four different answers.
        for (index, (virtue, mantra)) in BLACKTHORN_SHRINE_MANTRAS.iter().enumerate() {
            let mut c = BlackthornChallenge::for_shrine(index as u8);
            c.begin();
            assert_eq!(c.shrine_virtue(), *virtue);
            assert_eq!(c.expected_mantra(), *mantra);
            assert_eq!(c.submit(mantra), BlackthornChallengeOutcome::Survived);
            assert!(c.has_survived());
        }
    }

    #[test]
    fn one_correct_answer_resolves_the_interrogation() {
        // §4: the loop asks about ONE shrine, so one correct answer
        // ends it. The old model demanded four different mantras.
        let mut c = BlackthornChallenge::for_shrine(3);
        c.begin();
        assert_eq!(c.submit("Beh"), BlackthornChallengeOutcome::Survived);
        assert_eq!(c.correct_count, 1);
        assert!(c.has_survived());
    }

    #[test]
    fn the_same_mantra_is_accepted_at_every_ordinal() {
        // §4: "The shrine index is fixed before the loop starts and
        // never changes inside it ... the same on all four prompts."
        for skip_wrongs in 0..BLACKTHORN_CHALLENGE_PROMPT_COUNT {
            let mut c = BlackthornChallenge::for_shrine(1);
            c.begin();
            for _ in 0..skip_wrongs {
                assert!(matches!(
                    c.submit("nonsense"),
                    BlackthornChallengeOutcome::Wrong { .. }
                ));
            }
            assert_eq!(c.submit("Mu"), BlackthornChallengeOutcome::Survived);
        }
    }

    #[test]
    fn wrong_answers_escalate_and_only_the_fourth_punishes() {
        // §4: "The first wrong answer produces a threat naming the
        // companion at risk. Later wrong answers stamp a tile into the
        // cutscene map, and the fourth wrong answer **kills** the named
        // companion". The old model punished on the first wrong answer.
        let mut c = BlackthornChallenge::for_shrine(0);
        c.begin();

        assert_eq!(
            c.submit("nope"),
            BlackthornChallengeOutcome::Wrong {
                ordinal: 0,
                expected: "Ahm"
            }
        );
        assert!(!c.is_punished());
        assert_eq!(
            c.wrong_escalation(),
            Some(BlackthornWrongEscalation::Threat)
        );
        assert_eq!(
            c.current_wording(),
            Some(BlackthornChallengeWording::Repeat)
        );

        assert!(matches!(
            c.submit("nope"),
            BlackthornChallengeOutcome::Wrong { ordinal: 1, .. }
        ));
        assert!(!c.is_punished());
        assert_eq!(
            c.wrong_escalation(),
            Some(BlackthornWrongEscalation::TileStamp)
        );
        assert_eq!(
            c.current_wording(),
            Some(BlackthornChallengeWording::ImpatientDemand)
        );

        assert!(matches!(
            c.submit("nope"),
            BlackthornChallengeOutcome::Wrong { ordinal: 2, .. }
        ));
        assert!(!c.is_punished());
        assert_eq!(
            c.wrong_escalation(),
            Some(BlackthornWrongEscalation::TileStamp)
        );
        assert_eq!(
            c.current_wording(),
            Some(BlackthornChallengeWording::ShoutedFinalDemand)
        );

        assert!(matches!(
            c.submit("nope"),
            BlackthornChallengeOutcome::Wrong { ordinal: 3, .. }
        ));
        assert!(c.is_punished());
        assert_eq!(c.wrong_count, 4);
        assert_eq!(c.wrong_escalation(), Some(BlackthornWrongEscalation::Kill));
        assert_eq!(c.current_wording(), None);
    }

    #[test]
    fn submit_after_punishment_returns_sentinel() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        for _ in 0..BLACKTHORN_CHALLENGE_PROMPT_COUNT {
            c.submit("wrong");
        }
        assert_eq!(c.submit("Ahm"), BlackthornChallengeOutcome::AlreadyPunished);
    }

    #[test]
    fn submit_after_survival_returns_sentinel() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        c.submit("Ahm");
        assert_eq!(c.submit("Ahm"), BlackthornChallengeOutcome::AlreadySurvived);
    }

    #[test]
    fn case_insensitive_substring_match() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        assert_eq!(c.submit("word AHM"), BlackthornChallengeOutcome::Survived);
    }

    #[test]
    fn input_limit_applies_before_substring_match() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        let outcome = c.submit("xxxxxxxxxxxxxxAhm");
        assert_eq!(
            outcome,
            BlackthornChallengeOutcome::Wrong {
                ordinal: 0,
                expected: "Ahm"
            }
        );
        // §4's escalation means one wrong answer is a threat, not the
        // punishment.
        assert!(!c.is_punished());
        assert_eq!(
            c.wrong_escalation(),
            Some(BlackthornWrongEscalation::Threat)
        );
    }

    #[test]
    fn input_limit_keeps_answers_inside_fourteen_characters() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        let outcome = c.submit("xxxxxxxxxxxAhm trailing text");
        assert_eq!(outcome, BlackthornChallengeOutcome::Survived);
    }

    #[test]
    fn abort_explicitly_terminates_challenge() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        c.abort();
        assert!(matches!(c.phase, BlackthornChallengePhase::Aborted));
        assert_eq!(c.submit("Ahm"), BlackthornChallengeOutcome::AlreadyAborted);
    }

    #[test]
    fn correct_count_tracks_successful_answers() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        c.submit("Ahm");
        assert_eq!(c.correct_count, 1);
    }
}
