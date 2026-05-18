//! Stone of Castigation / Codex challenge state machine.
//!
//! The shrine-of-the-Codex challenge presents three sequential
//! single-word challenges to the party (Truth, Love, Courage —
//! one per Eternal Flame). A correct word advances to the next
//! challenge; a wrong word resets. Once all three are answered
//! correctly, the Codex is revealed and the endgame trigger arms.
//!
//! This module is the pure state machine: the caller drives input
//! one typed word at a time and applies the resulting outcome to
//! PlayState.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CodexChallengePhase {
    #[default]
    AwaitingTruthWord,
    AwaitingLoveWord,
    AwaitingCourageWord,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexChallengeOutcome {
    Advanced(CodexChallengePhase),
    Completed,
    WrongAnswer,
    AlreadyCompleted,
    AlreadyFailed,
}

/// Three Words of Power the challenge accepts. Each is a single
/// short Britannian word the player learns through dungeon glyphs.
/// These are stable across the campaign and the spec calls them out
/// in `catalogs/quest-graph.md §4`.
pub const CODEX_WORD_TRUTH: &str = "VERAMOCOR";
pub const CODEX_WORD_LOVE: &str = "AMORE";
pub const CODEX_WORD_COURAGE: &str = "FORTITUDO";

#[derive(Clone, Copy, Debug, Default)]
pub struct CodexChallenge {
    pub phase: CodexChallengePhase,
    pub attempts: u32,
}

impl CodexChallenge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn submit(&mut self, typed: &str) -> CodexChallengeOutcome {
        let typed = typed.trim();
        self.attempts = self.attempts.saturating_add(1);
        match self.phase {
            CodexChallengePhase::Completed => CodexChallengeOutcome::AlreadyCompleted,
            CodexChallengePhase::Failed => CodexChallengeOutcome::AlreadyFailed,
            CodexChallengePhase::AwaitingTruthWord => {
                if typed.eq_ignore_ascii_case(CODEX_WORD_TRUTH) {
                    self.phase = CodexChallengePhase::AwaitingLoveWord;
                    CodexChallengeOutcome::Advanced(self.phase)
                } else {
                    self.phase = CodexChallengePhase::Failed;
                    CodexChallengeOutcome::WrongAnswer
                }
            }
            CodexChallengePhase::AwaitingLoveWord => {
                if typed.eq_ignore_ascii_case(CODEX_WORD_LOVE) {
                    self.phase = CodexChallengePhase::AwaitingCourageWord;
                    CodexChallengeOutcome::Advanced(self.phase)
                } else {
                    self.phase = CodexChallengePhase::Failed;
                    CodexChallengeOutcome::WrongAnswer
                }
            }
            CodexChallengePhase::AwaitingCourageWord => {
                if typed.eq_ignore_ascii_case(CODEX_WORD_COURAGE) {
                    self.phase = CodexChallengePhase::Completed;
                    CodexChallengeOutcome::Completed
                } else {
                    self.phase = CodexChallengePhase::Failed;
                    CodexChallengeOutcome::WrongAnswer
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.phase = CodexChallengePhase::AwaitingTruthWord;
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.phase, CodexChallengePhase::Completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_challenge_starts_awaiting_truth_word() {
        let c = CodexChallenge::new();
        assert_eq!(c.phase, CodexChallengePhase::AwaitingTruthWord);
    }

    #[test]
    fn correct_sequence_completes_challenge() {
        let mut c = CodexChallenge::new();
        assert!(matches!(
            c.submit(CODEX_WORD_TRUTH),
            CodexChallengeOutcome::Advanced(CodexChallengePhase::AwaitingLoveWord)
        ));
        assert!(matches!(
            c.submit(CODEX_WORD_LOVE),
            CodexChallengeOutcome::Advanced(CodexChallengePhase::AwaitingCourageWord)
        ));
        assert_eq!(
            c.submit(CODEX_WORD_COURAGE),
            CodexChallengeOutcome::Completed
        );
        assert!(c.is_completed());
    }

    #[test]
    fn case_insensitive_word_matching() {
        let mut c = CodexChallenge::new();
        c.submit("veramocor");
        c.submit("Amore");
        assert_eq!(c.submit("FORTITUDO"), CodexChallengeOutcome::Completed);
    }

    #[test]
    fn wrong_word_marks_failure() {
        let mut c = CodexChallenge::new();
        assert_eq!(c.submit("WRONG"), CodexChallengeOutcome::WrongAnswer);
        assert_eq!(c.phase, CodexChallengePhase::Failed);
    }

    #[test]
    fn reset_after_failure_re_arms_first_word() {
        let mut c = CodexChallenge::new();
        c.submit("nope");
        c.reset();
        assert_eq!(c.phase, CodexChallengePhase::AwaitingTruthWord);
    }

    #[test]
    fn submit_after_completion_returns_sentinel() {
        let mut c = CodexChallenge::new();
        c.submit(CODEX_WORD_TRUTH);
        c.submit(CODEX_WORD_LOVE);
        c.submit(CODEX_WORD_COURAGE);
        assert_eq!(
            c.submit(CODEX_WORD_TRUTH),
            CodexChallengeOutcome::AlreadyCompleted
        );
    }

    #[test]
    fn submit_after_failure_returns_sentinel() {
        let mut c = CodexChallenge::new();
        c.submit("nope");
        assert_eq!(
            c.submit(CODEX_WORD_TRUTH),
            CodexChallengeOutcome::AlreadyFailed
        );
    }

    #[test]
    fn attempts_counter_increments_on_each_submit() {
        let mut c = CodexChallenge::new();
        c.submit("a");
        c.submit("b");
        assert_eq!(c.attempts, 2);
    }
}
