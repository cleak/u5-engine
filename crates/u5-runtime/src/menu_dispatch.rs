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
    /// The gameplay PRNG state for this run, owned by the intro controller.
    ///
    /// `prng.md §3`: the program "assigns the resulting state **once per
    /// run** in the intro controller", before the first menu poll, and
    /// "Returning to the menu from a menu sub-flow resumes after the seed
    /// assignment and does not sample again". Nothing re-seeds at Journey
    /// Onward or at the save load, and the state word "is not part of the
    /// saved game" - so a menu sub-flow that draws from the shared generator
    /// (`chargen.md §6`'s virtue picker) leaves the stream advanced for the
    /// Journey Onward that follows it.
    gameplay_prng: Option<u16>,
    /// `u4-transfer.md §5.3`: the Avatarhood flag "is set at most once and
    /// is **never cleared** for the remainder of the run ... do not assume
    /// it resets between attempts" (playtest P17-06).
    u4_avatarhood_latched: bool,
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

    /// The run's gameplay PRNG state, taking the intro controller's single
    /// host-clock sample (`prng.md §3`) the first time it is asked for.
    ///
    /// The frontend asks once the start/menu screen is prepared, which is
    /// where the original samples; the early Journey Onward path "jumps
    /// directly to the same sample rather than bypassing it", which the
    /// lazy sample here covers.
    pub fn gameplay_prng_state(&mut self) -> u16 {
        *self
            .gameplay_prng
            .get_or_insert_with(crate::prng::host_clock_prng_seed_now)
    }

    /// Install the run's gameplay PRNG state directly, in place of the host
    /// clock sample - for a harness that wants a known stream.
    pub fn set_gameplay_prng_state(&mut self, state: u16) {
        self.gameplay_prng = Some(state);
    }

    /// Account for `draws` advances a menu sub-flow took from the run's
    /// gameplay stream (the chargen virtue picker's `random(0, 7)` calls).
    pub fn consume_gameplay_prng_draws(&mut self, draws: usize) {
        let mut state = self.gameplay_prng_state();
        for _ in 0..draws {
            state = crate::prng::u5_prng_advance_state(state);
        }
        self.gameplay_prng = Some(state);
    }

    /// `u4-transfer.md §5.3`: fold one transfer attempt's Avatarhood test
    /// into the run's one-shot latch and return the latched value. A
    /// passing attempt sets it; nothing clears it.
    pub fn latch_u4_avatarhood(&mut self, attempt_passed: bool) -> bool {
        self.u4_avatarhood_latched |= attempt_passed;
        self.u4_avatarhood_latched
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

    /// `prng.md §3`: one host-clock sample per run; a sub-flow's draws carry
    /// forward and nothing samples again.
    #[test]
    fn the_run_samples_its_gameplay_seed_once_and_carries_sub_flow_draws() {
        let mut d = UnifiedMenuDispatch::new();
        let seed = d.gameplay_prng_state();
        assert_eq!(
            d.gameplay_prng_state(),
            seed,
            "a second ask does not re-sample"
        );

        d.consume_gameplay_prng_draws(3);
        let mut expected = seed;
        for _ in 0..3 {
            expected = crate::prng::u5_prng_advance_state(expected);
        }
        assert_eq!(d.gameplay_prng_state(), expected);

        d.consume_gameplay_prng_draws(0);
        assert_eq!(d.gameplay_prng_state(), expected);
    }

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
    fn blackthorn_one_correct_answer_marks_survived() {
        // `blackthorn.md §4`: "**The loop asks about ONE shrine, up to
        // four times.**" One correct answer resolves the interrogation;
        // this test previously demanded four different mantras, which
        // §4's withdrawal box retires.
        let mut d = UnifiedMenuDispatch::new();
        d.open_blackthorn();
        assert_eq!(
            d.submit_blackthorn_answer("Ahm"),
            UnifiedMenuStep::BlackthornEnded { survived: true }
        );
        // The loop is over: further submissions do not re-open it.
        assert_eq!(d.submit_blackthorn_answer("Ahm"), UnifiedMenuStep::Ignored);
    }

    #[test]
    fn blackthorn_wrong_answers_escalate_before_punishing() {
        // `blackthorn.md §4`: "The first wrong answer produces a threat
        // naming the companion at risk. Later wrong answers stamp a
        // tile into the cutscene map, and the fourth wrong answer
        // **kills** the named companion." This test previously ended
        // the interrogation on the first wrong answer.
        let mut d = UnifiedMenuDispatch::new();
        d.open_blackthorn();
        for _ in 0..4 {
            assert_eq!(
                d.submit_blackthorn_answer("wrong"),
                UnifiedMenuStep::BlackthornEnded { survived: false }
            );
        }
        assert_eq!(d.blackthorn.as_ref().map(|c| c.is_punished()), Some(true));
    }

    #[test]
    fn blackthorn_answer_limit_applies_through_menu_dispatch() {
        let mut d = UnifiedMenuDispatch::new();
        d.open_blackthorn();
        assert_eq!(
            d.submit_blackthorn_answer("xxxxxxxxxxxxxxAhm"),
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
