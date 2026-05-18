//! Unified menu-loop dispatcher that orchestrates the five intro
//! sub-flow state machines (intro menu, chargen tournament, U4
//! transfer, codex challenge, blackthorn audience). A TUI shell or
//! a Bevy frontend wraps one of these and feeds keystrokes / typed
//! lines through `step_*` calls.

use crate::blackthorn_session::BlackthornChallenge;
use crate::codex_challenge::CodexChallenge;
use crate::intro_menu::{IntroMenu, IntroMenuOutput, IntroSubflow, IntroSubflowResult};
use crate::u4_transfer_session::U4TransferSession;

/// Combined menu / sub-flow state owned by the harness.
#[derive(Debug, Default)]
pub struct UnifiedMenuDispatch {
    pub intro: IntroMenu,
    pub u4: Option<U4TransferSession>,
    pub codex: Option<CodexChallenge>,
    pub blackthorn: Option<BlackthornChallenge>,
}

/// One reported state-change to the harness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnifiedMenuStep {
    /// Title-screen idle tick; harness should keep animating.
    PresentTitle,
    /// Title dismissed → menu now visible.
    PresentMenu,
    /// Harness should run the named sub-flow next (caller drives it
    /// via the matching `step_*` calls).
    EnteredSubflow(IntroSubflow),
    /// Sub-flow completed normally; menu re-presented.
    ReturnedToMenu,
    /// Save image is loaded; harness should switch to gameplay.
    LaunchGameplay,
    /// Codex challenge advanced (correct or wrong).
    CodexAdvanced(crate::codex_challenge::CodexChallengePhase),
    /// Codex challenge completed.
    CodexCompleted,
    /// Blackthorn challenge advanced.
    BlackthornAdvanced,
    /// Blackthorn challenge ended (survived or punished).
    BlackthornEnded { survived: bool },
    /// U4 transfer step processed; harness should re-render preview /
    /// confirmation as appropriate.
    U4Stepped,
    /// Input was unrecognised; harness should silently re-present.
    Ignored,
}

impl UnifiedMenuDispatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Title-screen tick.
    pub fn tick_title(&self) -> UnifiedMenuStep {
        match self.intro.tick_title() {
            IntroMenuOutput::PresentTitle => UnifiedMenuStep::PresentTitle,
            _ => UnifiedMenuStep::PresentMenu,
        }
    }

    /// First-key handler that dismisses the title screen.
    pub fn dismiss_title(&mut self) -> UnifiedMenuStep {
        match self.intro.dismiss_title() {
            IntroMenuOutput::PresentMenu => UnifiedMenuStep::PresentMenu,
            IntroMenuOutput::PresentTitle => UnifiedMenuStep::PresentTitle,
            _ => UnifiedMenuStep::Ignored,
        }
    }

    /// Menu keystroke dispatcher. Routes the supplied key into the
    /// intro menu and surfaces the sub-flow it dispatches to.
    pub fn submit_menu_key(&mut self, key: u8) -> UnifiedMenuStep {
        match self.intro.step(key) {
            IntroMenuOutput::EnterSubflow(sub) => UnifiedMenuStep::EnteredSubflow(sub),
            IntroMenuOutput::PresentMenu => UnifiedMenuStep::PresentMenu,
            IntroMenuOutput::IgnoredKey => UnifiedMenuStep::Ignored,
            IntroMenuOutput::LaunchGameplay => UnifiedMenuStep::LaunchGameplay,
            IntroMenuOutput::PresentTitle => UnifiedMenuStep::PresentTitle,
        }
    }

    /// Report a sub-flow outcome back to the menu and return the
    /// resulting transition.
    pub fn complete_subflow(
        &mut self,
        sub: IntroSubflow,
        result: IntroSubflowResult,
    ) -> UnifiedMenuStep {
        match self.intro.complete_subflow(sub, result) {
            IntroMenuOutput::PresentMenu => UnifiedMenuStep::ReturnedToMenu,
            IntroMenuOutput::LaunchGameplay => UnifiedMenuStep::LaunchGameplay,
            IntroMenuOutput::PresentTitle => UnifiedMenuStep::PresentTitle,
            IntroMenuOutput::IgnoredKey | IntroMenuOutput::EnterSubflow(_) => {
                UnifiedMenuStep::Ignored
            }
        }
    }

    // ---- Codex flow ----

    pub fn open_codex(&mut self) {
        self.codex = Some(CodexChallenge::new());
    }

    pub fn submit_codex_word(&mut self, word: &str) -> UnifiedMenuStep {
        let Some(codex) = self.codex.as_mut() else {
            return UnifiedMenuStep::Ignored;
        };
        use crate::codex_challenge::CodexChallengeOutcome;
        match codex.submit(word) {
            CodexChallengeOutcome::Advanced(phase) => UnifiedMenuStep::CodexAdvanced(phase),
            CodexChallengeOutcome::Completed => UnifiedMenuStep::CodexCompleted,
            CodexChallengeOutcome::WrongAnswer => UnifiedMenuStep::Ignored,
            CodexChallengeOutcome::AlreadyCompleted | CodexChallengeOutcome::AlreadyFailed => {
                UnifiedMenuStep::Ignored
            }
        }
    }

    // ---- Blackthorn flow ----

    pub fn open_blackthorn(&mut self) {
        let mut c = BlackthornChallenge::new();
        c.begin();
        self.blackthorn = Some(c);
    }

    pub fn submit_blackthorn_answer(&mut self, typed: &str) -> UnifiedMenuStep {
        let Some(challenge) = self.blackthorn.as_mut() else {
            return UnifiedMenuStep::Ignored;
        };
        use crate::blackthorn_session::BlackthornChallengeOutcome;
        match challenge.submit(typed) {
            BlackthornChallengeOutcome::Correct { .. }
            | BlackthornChallengeOutcome::PromptPresented { .. } => {
                UnifiedMenuStep::BlackthornAdvanced
            }
            BlackthornChallengeOutcome::Survived => {
                UnifiedMenuStep::BlackthornEnded { survived: true }
            }
            BlackthornChallengeOutcome::Wrong { .. } => {
                UnifiedMenuStep::BlackthornEnded { survived: false }
            }
            _ => UnifiedMenuStep::Ignored,
        }
    }

    // ---- U4 transfer flow ----

    pub fn open_u4_transfer(&mut self) {
        self.u4 = Some(U4TransferSession::new());
    }

    pub fn submit_u4_input(
        &mut self,
        input: crate::u4_transfer_session::U4TransferInput,
    ) -> UnifiedMenuStep {
        let Some(session) = self.u4.as_mut() else {
            return UnifiedMenuStep::Ignored;
        };
        let _event = session.step(input);
        UnifiedMenuStep::U4Stepped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_challenge::*;

    #[test]
    fn new_dispatch_starts_on_title() {
        let d = UnifiedMenuDispatch::new();
        assert_eq!(d.tick_title(), UnifiedMenuStep::PresentTitle);
    }

    #[test]
    fn dismiss_title_then_pick_create_character() {
        let mut d = UnifiedMenuDispatch::new();
        d.dismiss_title();
        let step = d.submit_menu_key(b'C');
        assert_eq!(
            step,
            UnifiedMenuStep::EnteredSubflow(IntroSubflow::CharacterCreation)
        );
    }

    #[test]
    fn complete_subflow_with_save_ready_launches_gameplay() {
        let mut d = UnifiedMenuDispatch::new();
        d.dismiss_title();
        d.submit_menu_key(b'J');
        let step = d.complete_subflow(IntroSubflow::JourneyOnward, IntroSubflowResult::SaveReady);
        assert_eq!(step, UnifiedMenuStep::LaunchGameplay);
    }

    #[test]
    fn character_creation_save_ready_returns_to_menu_step() {
        let mut d = UnifiedMenuDispatch::new();
        d.dismiss_title();
        d.submit_menu_key(b'C');
        let step = d.complete_subflow(
            IntroSubflow::CharacterCreation,
            IntroSubflowResult::SaveReady,
        );
        assert_eq!(step, UnifiedMenuStep::ReturnedToMenu);
    }

    #[test]
    fn codex_flow_walks_three_words_to_completion() {
        let mut d = UnifiedMenuDispatch::new();
        d.open_codex();
        assert_eq!(
            d.submit_codex_word(CODEX_WORD_TRUTH),
            UnifiedMenuStep::CodexAdvanced(CodexChallengePhase::AwaitingLoveWord)
        );
        assert_eq!(
            d.submit_codex_word(CODEX_WORD_LOVE),
            UnifiedMenuStep::CodexAdvanced(CodexChallengePhase::AwaitingCourageWord)
        );
        assert_eq!(
            d.submit_codex_word(CODEX_WORD_COURAGE),
            UnifiedMenuStep::CodexCompleted
        );
    }

    #[test]
    fn codex_wrong_answer_reports_ignored_step() {
        let mut d = UnifiedMenuDispatch::new();
        d.open_codex();
        assert_eq!(d.submit_codex_word("nope"), UnifiedMenuStep::Ignored);
    }

    #[test]
    fn blackthorn_four_correct_answers_marks_survived() {
        let mut d = UnifiedMenuDispatch::new();
        d.open_blackthorn();
        assert_eq!(
            d.submit_blackthorn_answer("Ahm"),
            UnifiedMenuStep::BlackthornAdvanced
        );
        assert_eq!(
            d.submit_blackthorn_answer("Mu"),
            UnifiedMenuStep::BlackthornAdvanced
        );
        assert_eq!(
            d.submit_blackthorn_answer("Ra"),
            UnifiedMenuStep::BlackthornAdvanced
        );
        assert_eq!(
            d.submit_blackthorn_answer("Beh"),
            UnifiedMenuStep::BlackthornEnded { survived: true }
        );
    }

    #[test]
    fn blackthorn_wrong_answer_marks_punished() {
        let mut d = UnifiedMenuDispatch::new();
        d.open_blackthorn();
        assert_eq!(
            d.submit_blackthorn_answer("wrong"),
            UnifiedMenuStep::BlackthornEnded { survived: false }
        );
    }

    #[test]
    fn u4_transfer_step_records_session_advancement() {
        use crate::u4_transfer_session::U4TransferInput;
        let mut d = UnifiedMenuDispatch::new();
        d.open_u4_transfer();
        assert_eq!(
            d.submit_u4_input(U4TransferInput::SourceFileLoaded),
            UnifiedMenuStep::U4Stepped
        );
    }

    #[test]
    fn submit_codex_word_without_opening_is_ignored() {
        let mut d = UnifiedMenuDispatch::new();
        assert_eq!(d.submit_codex_word("VERAMOCOR"), UnifiedMenuStep::Ignored);
    }

    #[test]
    fn submit_blackthorn_answer_without_opening_is_ignored() {
        let mut d = UnifiedMenuDispatch::new();
        assert_eq!(d.submit_blackthorn_answer("Ahm"), UnifiedMenuStep::Ignored);
    }
}
