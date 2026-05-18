//! Blackthorn audience challenge state machine per
//! `systems/blackthorn.md` §4. Wraps the per-prompt helpers in
//! [`crate::blackthorn`] into a four-prompt interactive flow.

use crate::blackthorn::{
    blackthorn_challenge_answer_matches, blackthorn_challenge_prompt,
    BLACKTHORN_CHALLENGE_PROMPT_COUNT,
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

#[derive(Clone, Copy, Debug, Default)]
pub struct BlackthornChallenge {
    pub phase: BlackthornChallengePhase,
    pub correct_count: u8,
}

impl BlackthornChallenge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin the challenge. Returns the first prompt.
    pub fn begin(&mut self) -> BlackthornChallengeOutcome {
        let prompt = blackthorn_challenge_prompt(0)
            .expect("blackthorn challenge has at least one prompt")
            .0;
        self.phase = BlackthornChallengePhase::PresentingPrompt { ordinal: 0 };
        BlackthornChallengeOutcome::PromptPresented { ordinal: 0, prompt }
    }

    pub fn submit(&mut self, typed: &str) -> BlackthornChallengeOutcome {
        match self.phase {
            BlackthornChallengePhase::Punished { .. } => {
                BlackthornChallengeOutcome::AlreadyPunished
            }
            BlackthornChallengePhase::Survived => BlackthornChallengeOutcome::AlreadySurvived,
            BlackthornChallengePhase::Aborted => BlackthornChallengeOutcome::AlreadyAborted,
            BlackthornChallengePhase::AwaitingAudience => self.begin(),
            BlackthornChallengePhase::PresentingPrompt { ordinal } => {
                let Some((prompt, expected)) = blackthorn_challenge_prompt(ordinal) else {
                    self.phase = BlackthornChallengePhase::Survived;
                    return BlackthornChallengeOutcome::Survived;
                };
                let _ = prompt;
                if blackthorn_challenge_answer_matches(typed, expected) {
                    self.correct_count = self.correct_count.saturating_add(1);
                    let next_ordinal = ordinal + 1;
                    if (next_ordinal as usize) >= BLACKTHORN_CHALLENGE_PROMPT_COUNT {
                        self.phase = BlackthornChallengePhase::Survived;
                        BlackthornChallengeOutcome::Survived
                    } else {
                        let next_prompt = blackthorn_challenge_prompt(next_ordinal)
                            .expect("ordinal already bounds-checked")
                            .0;
                        self.phase = BlackthornChallengePhase::PresentingPrompt {
                            ordinal: next_ordinal,
                        };
                        let _ = next_prompt;
                        BlackthornChallengeOutcome::Correct { ordinal }
                    }
                } else {
                    self.phase = BlackthornChallengePhase::Punished {
                        failed_ordinal: ordinal,
                    };
                    BlackthornChallengeOutcome::Wrong { ordinal, expected }
                }
            }
        }
    }

    pub fn current_prompt(&self) -> Option<(u8, &'static str)> {
        let BlackthornChallengePhase::PresentingPrompt { ordinal } = self.phase else {
            return None;
        };
        blackthorn_challenge_prompt(ordinal).map(|(prompt, _)| (ordinal, prompt))
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn four_correct_answers_marks_survived() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        c.submit("Ahm");
        c.submit("Mu");
        c.submit("Ra");
        let outcome = c.submit("Beh");
        assert_eq!(outcome, BlackthornChallengeOutcome::Survived);
        assert!(c.has_survived());
    }

    #[test]
    fn wrong_answer_marks_punished() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        let outcome = c.submit("wrong");
        assert!(matches!(
            outcome,
            BlackthornChallengeOutcome::Wrong { ordinal: 0, .. }
        ));
        assert!(c.is_punished());
    }

    #[test]
    fn submit_after_punishment_returns_sentinel() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        c.submit("wrong");
        assert_eq!(c.submit("Ahm"), BlackthornChallengeOutcome::AlreadyPunished);
    }

    #[test]
    fn submit_after_survival_returns_sentinel() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        c.submit("Ahm");
        c.submit("Mu");
        c.submit("Ra");
        c.submit("Beh");
        assert_eq!(c.submit("Beh"), BlackthornChallengeOutcome::AlreadySurvived);
    }

    #[test]
    fn case_insensitive_substring_match() {
        let mut c = BlackthornChallenge::new();
        c.begin();
        let outcome = c.submit("the word is AHM, my lord");
        assert!(matches!(
            outcome,
            BlackthornChallengeOutcome::Correct { ordinal: 0 }
        ));
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
        c.submit("Mu");
        assert_eq!(c.correct_count, 2);
    }
}
