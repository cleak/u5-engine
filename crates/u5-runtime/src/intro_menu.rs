//! Intro-menu state machine per `systems/intro.md` §6.
//!
//! The intro menu sits between the title presentation and live
//! gameplay. It polls one keystroke at a time and dispatches into one
//! of six sub-flows (Journey Onward, Create New Character, Transfer,
//! Story, Acknowledgements, Return-to-View) before returning to itself
//! to await the next selection.
//!
//! This module is the orchestrator: it owns the menu's current sub-
//! flow phase, the highlighted row index, the no-key idle-pass
//! counter, and a thin enum of "what should the harness do next?"
//! outputs. The sub-flows themselves live in [`crate::chargen`],
//! [`crate::u4_transfer`], and [`crate::intro`]; the menu just
//! sequences them.
//!
//! `intro.md §6.2`: "The menu has one input model with three entry
//! points, all of which operate on that same highlight index. ... the
//! row index is load-bearing, because Enter, Space and the idle
//! timeout all resolve through it. The claim that the menu keeps a
//! 'recent-selection cache' that Enter replays is withdrawn as well;
//! there is no such cache."

use crate::intro::{
    INTRO_MENU_IDLE_TIMEOUT_PASSES, INTRO_MENU_INITIAL_HIGHLIGHT_ROW, INTRO_MENU_ROW_COUNT,
    INTRO_MENU_ROW_LETTERS, IntroMenuAction, intro_menu_action,
};

/// Where the intro menu currently sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IntroMenuPhase {
    /// Title presentation is on screen; the menu is awaiting its
    /// first keystroke.
    #[default]
    Title,
    /// Six-key menu is presented; awaiting a key.
    AwaitingSelection,
    /// `J` flow: caller is loading `SAVED.GAM`/`SAVED.OOL`. Returns to
    /// `AwaitingSelection` on failure, transitions to `LaunchedGameplay`
    /// on success.
    JourneyOnwardLoading,
    /// `C` flow: caller is running the chargen tournament + name +
    /// gender prompts. Returns to `AwaitingSelection` on cancel or
    /// commit; the player must choose Journey Onward explicitly.
    CharacterCreation,
    /// `T` flow: caller is running the Ultima IV transfer path.
    UltimaIvTransfer,
    /// `U` flow: caller is paging the story slide sequence.
    StorySlides,
    /// `A` flow: caller is showing acknowledgements.
    Acknowledgements,
    /// `R` flow: caller is rendering the Return-to-View preview.
    ReturnToView,
    /// Terminal: gameplay loop has taken control.
    LaunchedGameplay,
}

/// What the menu wants the harness to do next, returned from each
/// `step`/`begin_*` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroMenuOutput {
    /// Present the title-screen idle animation; await a key.
    PresentTitle,
    /// Present the six-key menu; await a key.
    PresentMenu,
    /// Run the named sub-flow next. The harness reports success/failure
    /// back through `complete_subflow`.
    EnterSubflow(IntroSubflow),
    /// Switch to gameplay mode. The intro menu is done.
    LaunchGameplay,
    /// Most recent key was unrecognised — silently re-present the menu.
    IgnoredKey,
}

/// The six sub-flows the menu can dispatch to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroSubflow {
    JourneyOnward,
    CharacterCreation,
    UltimaIvTransfer,
    StorySlides,
    Acknowledgements,
    ReturnToView,
}

/// Result the harness passes back when a sub-flow completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroSubflowResult {
    /// The sub-flow finished without producing a loadable save.
    /// Returns to the menu.
    ReturnedToMenu,
    /// The sub-flow completed with a save the engine should load.
    /// Used by Journey Onward (success) and Character Creation
    /// (after the new save is written).
    SaveReady,
    /// Sub-flow was cancelled by the player (Escape from chargen,
    /// missing save for Journey Onward, transfer aborted, etc.).
    Cancelled,
}

/// Per-call state.
///
/// `intro.md §6.2`: "**The initial highlight is row 0, `Journey
/// Onward`**, and the highlight index survives across poll passes."
#[derive(Clone, Copy, Debug)]
pub struct IntroMenu {
    pub phase: IntroMenuPhase,
    /// Index of the row drawn in inverse video, `0..=5`.
    pub highlight_row: u8,
    /// Consecutive no-key menu poll passes seen so far; two hundred of
    /// them commit `Return to the View`.
    pub idle_passes: u16,
}

impl Default for IntroMenu {
    fn default() -> Self {
        Self {
            phase: IntroMenuPhase::default(),
            highlight_row: INTRO_MENU_INITIAL_HIGHLIGHT_ROW,
            idle_passes: 0,
        }
    }
}

/// `intro.md §6.2`: resolve a row index through the fixed six-entry
/// row-to-letter table (`J`, `C`, `T`, `U`, `A`, `R`).
fn subflow_for_row(row: u8) -> IntroSubflow {
    let letter = INTRO_MENU_ROW_LETTERS[usize::from(row) % INTRO_MENU_ROW_COUNT];
    match intro_menu_action(letter) {
        Some(IntroMenuAction::JourneyOnward) => IntroSubflow::JourneyOnward,
        Some(IntroMenuAction::CreateNewCharacter) => IntroSubflow::CharacterCreation,
        Some(IntroMenuAction::TransferFromUltimaIv) => IntroSubflow::UltimaIvTransfer,
        Some(IntroMenuAction::UltimaVIntroduction) => IntroSubflow::StorySlides,
        Some(IntroMenuAction::Acknowledgements) => IntroSubflow::Acknowledgements,
        Some(IntroMenuAction::ReturnToView) => IntroSubflow::ReturnToView,
        _ => unreachable!("every row-to-letter entry is a published menu letter"),
    }
}

impl IntroMenu {
    pub fn new() -> Self {
        Self::default()
    }

    /// The sub-flow the currently highlighted row would commit.
    pub fn highlight(&self) -> IntroSubflow {
        subflow_for_row(self.highlight_row)
    }

    /// Title animation tick: the engine has presented one title-tick
    /// frame and is asking what to do next. Always returns
    /// `PresentTitle` while the phase is `Title`.
    pub fn tick_title(&self) -> IntroMenuOutput {
        match self.phase {
            IntroMenuPhase::Title => IntroMenuOutput::PresentTitle,
            _ => IntroMenuOutput::PresentMenu,
        }
    }

    /// Advance out of the title screen when the player presses any
    /// key. Transitions to `AwaitingSelection` and asks the harness
    /// to present the menu.
    pub fn dismiss_title(&mut self) -> IntroMenuOutput {
        if matches!(self.phase, IntroMenuPhase::Title) {
            self.phase = IntroMenuPhase::AwaitingSelection;
            IntroMenuOutput::PresentMenu
        } else {
            self.tick_title()
        }
    }

    /// Feed one menu keystroke. Returns the next harness action.
    ///
    /// `intro.md §6.2`: a letter hotkey "Move[s] the highlight to that
    /// row **and** commit[s] it in the same pass"; the arrows move the
    /// highlight and keep polling; Enter and Space "Commit whichever
    /// row is currently highlighted, resolved through the row-to-letter
    /// table"; any other key is discarded.
    pub fn step(&mut self, key: u8) -> IntroMenuOutput {
        if !matches!(self.phase, IntroMenuPhase::AwaitingSelection) {
            return IntroMenuOutput::IgnoredKey;
        }
        let Some(action) = intro_menu_action(key) else {
            return IntroMenuOutput::IgnoredKey;
        };
        self.idle_passes = 0;
        let rows = INTRO_MENU_ROW_COUNT as u8;
        let row = match action {
            IntroMenuAction::MoveHighlightUp => {
                self.highlight_row = (self.highlight_row + rows - 1) % rows;
                return IntroMenuOutput::PresentMenu;
            }
            IntroMenuAction::MoveHighlightDown => {
                self.highlight_row = (self.highlight_row + 1) % rows;
                return IntroMenuOutput::PresentMenu;
            }
            IntroMenuAction::CommitHighlight => self.highlight_row,
            letter => letter
                .letter_row()
                .expect("letter hotkeys always name a published row"),
        };
        self.commit_row(row)
    }

    /// `intro.md §6.2`: "Two hundred consecutive no-key passes | Commit
    /// `Return to the View` exactly as though `R` had been pressed."
    /// Feed one no-key menu poll pass.
    pub fn idle_pass(&mut self) -> IntroMenuOutput {
        if !matches!(self.phase, IntroMenuPhase::AwaitingSelection) {
            return IntroMenuOutput::IgnoredKey;
        }
        self.idle_passes = self.idle_passes.saturating_add(1);
        if self.idle_passes < INTRO_MENU_IDLE_TIMEOUT_PASSES {
            return IntroMenuOutput::PresentMenu;
        }
        self.idle_passes = 0;
        self.commit_row((INTRO_MENU_ROW_COUNT - 1) as u8)
    }

    /// Move the highlight to `row` and commit it.
    fn commit_row(&mut self, row: u8) -> IntroMenuOutput {
        self.highlight_row = row % (INTRO_MENU_ROW_COUNT as u8);
        let resolved = subflow_for_row(self.highlight_row);
        self.phase = match resolved {
            IntroSubflow::JourneyOnward => IntroMenuPhase::JourneyOnwardLoading,
            IntroSubflow::CharacterCreation => IntroMenuPhase::CharacterCreation,
            IntroSubflow::UltimaIvTransfer => IntroMenuPhase::UltimaIvTransfer,
            IntroSubflow::StorySlides => IntroMenuPhase::StorySlides,
            IntroSubflow::Acknowledgements => IntroMenuPhase::Acknowledgements,
            IntroSubflow::ReturnToView => IntroMenuPhase::ReturnToView,
        };
        IntroMenuOutput::EnterSubflow(resolved)
    }

    /// Report the sub-flow's outcome. Returns the next harness action.
    pub fn complete_subflow(
        &mut self,
        sub: IntroSubflow,
        result: IntroSubflowResult,
    ) -> IntroMenuOutput {
        self.idle_passes = 0;
        match (sub, result) {
            (IntroSubflow::JourneyOnward, IntroSubflowResult::SaveReady) => {
                self.phase = IntroMenuPhase::LaunchedGameplay;
                IntroMenuOutput::LaunchGameplay
            }
            (IntroSubflow::CharacterCreation, IntroSubflowResult::SaveReady) => {
                self.phase = IntroMenuPhase::AwaitingSelection;
                IntroMenuOutput::PresentMenu
            }
            (_, IntroSubflowResult::SaveReady) => {
                self.phase = IntroMenuPhase::AwaitingSelection;
                IntroMenuOutput::PresentMenu
            }
            (_, IntroSubflowResult::ReturnedToMenu) | (_, IntroSubflowResult::Cancelled) => {
                self.phase = IntroMenuPhase::AwaitingSelection;
                IntroMenuOutput::PresentMenu
            }
        }
    }

    /// Returns `true` when the menu has handed control to gameplay.
    pub fn is_launched(&self) -> bool {
        matches!(self.phase, IntroMenuPhase::LaunchedGameplay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_menu_starts_on_title_phase() {
        let menu = IntroMenu::new();
        assert_eq!(menu.phase, IntroMenuPhase::Title);
        assert_eq!(menu.tick_title(), IntroMenuOutput::PresentTitle);
    }

    #[test]
    fn dismiss_title_transitions_to_awaiting_selection() {
        let mut menu = IntroMenu::new();
        assert_eq!(menu.dismiss_title(), IntroMenuOutput::PresentMenu);
        assert_eq!(menu.phase, IntroMenuPhase::AwaitingSelection);
    }

    #[test]
    fn step_with_j_enters_journey_onward_subflow() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(
            menu.step(b'J'),
            IntroMenuOutput::EnterSubflow(IntroSubflow::JourneyOnward)
        );
        assert_eq!(menu.phase, IntroMenuPhase::JourneyOnwardLoading);
        assert_eq!(menu.highlight_row, 0);
        assert_eq!(menu.highlight(), IntroSubflow::JourneyOnward);
    }

    #[test]
    fn step_with_c_enters_character_creation_subflow() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(
            menu.step(b'c'),
            IntroMenuOutput::EnterSubflow(IntroSubflow::CharacterCreation)
        );
    }

    #[test]
    fn step_with_unknown_key_is_ignored() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(menu.step(b'X'), IntroMenuOutput::IgnoredKey);
        assert_eq!(menu.phase, IntroMenuPhase::AwaitingSelection);
    }

    /// `intro.md §6.2`: "Letter hotkeys and the highlight model
    /// therefore coexist rather than competing: a letter both moves the
    /// bar and activates the row, so the bar always reflects the last
    /// selection made."
    ///
    /// Enter then re-commits whatever the bar shows — not because a
    /// "recent-selection cache" is replayed (that reading is withdrawn:
    /// "there is no such cache") but because Enter resolves the
    /// highlight index through the row-to-letter table.
    #[test]
    fn enter_key_commits_the_highlighted_row() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'A');
        assert_eq!(menu.highlight_row, 4);
        menu.complete_subflow(
            IntroSubflow::Acknowledgements,
            IntroSubflowResult::ReturnedToMenu,
        );
        assert_eq!(
            menu.step(b'\r'),
            IntroMenuOutput::EnterSubflow(IntroSubflow::Acknowledgements)
        );
    }

    /// `intro.md §6.2`: "**The initial highlight is row 0, `Journey
    /// Onward`**", and "Enter, Space | Commit whichever row is
    /// currently highlighted, resolved through the row-to-letter
    /// table."
    ///
    /// A freshly presented menu therefore always has a row to commit;
    /// the withdrawn reading ignored Enter until a letter had been
    /// pressed.
    #[test]
    fn enter_key_on_a_fresh_menu_commits_row_zero() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(menu.highlight_row, 0);
        assert_eq!(
            menu.step(b'\r'),
            IntroMenuOutput::EnterSubflow(IntroSubflow::JourneyOnward)
        );
    }

    /// `intro.md §6.2`: "Enter, Space | Commit whichever row is
    /// currently highlighted, resolved through the row-to-letter
    /// table." Space is an accepted input, not a discarded key.
    #[test]
    fn space_commits_the_highlighted_row() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(
            menu.step(b' '),
            IntroMenuOutput::EnterSubflow(IntroSubflow::JourneyOnward)
        );
    }

    /// `intro.md §6.2`: "Up arrow, left arrow | Move the highlight one
    /// row toward row 0, wrapping from row 0 to row 5; repaint the
    /// labels; keep polling." and the mirrored down/right row.
    #[test]
    fn arrow_keys_move_the_highlight_with_wraparound() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(
            menu.step(crate::INPUT_CODE_SOUTH),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.highlight_row, 1);
        assert_eq!(
            menu.step(crate::INPUT_CODE_EAST),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.highlight_row, 2);
        assert_eq!(
            menu.step(crate::INPUT_CODE_NORTH),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.highlight_row, 1);
        assert_eq!(
            menu.step(crate::INPUT_CODE_WEST),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.highlight_row, 0);
        // Wrap from row 0 back to row 5, and from row 5 forward to 0.
        assert_eq!(
            menu.step(crate::INPUT_CODE_NORTH),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.highlight_row, 5);
        assert_eq!(menu.highlight(), IntroSubflow::ReturnToView);
        assert_eq!(
            menu.step(crate::INPUT_CODE_SOUTH),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.highlight_row, 0);
        // Moving the highlight commits nothing.
        assert_eq!(menu.phase, IntroMenuPhase::AwaitingSelection);
    }

    /// `intro.md §6.2`: "Two hundred consecutive no-key passes | Commit
    /// `Return to the View` exactly as though `R` had been pressed."
    #[test]
    fn two_hundred_no_key_passes_commit_return_to_the_view() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        for pass in 1..crate::INTRO_MENU_IDLE_TIMEOUT_PASSES {
            assert_eq!(
                menu.idle_pass(),
                IntroMenuOutput::PresentMenu,
                "pass {pass}"
            );
        }
        assert_eq!(
            menu.idle_pass(),
            IntroMenuOutput::EnterSubflow(IntroSubflow::ReturnToView)
        );
        assert_eq!(menu.highlight_row, 5);
    }

    /// `intro.md §6.2`: the timeout counts *consecutive* no-key passes,
    /// so any accepted key restarts the count.
    #[test]
    fn a_keystroke_restarts_the_idle_pass_count() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        for _ in 0..(crate::INTRO_MENU_IDLE_TIMEOUT_PASSES - 1) {
            menu.idle_pass();
        }
        assert_eq!(
            menu.step(crate::INPUT_CODE_SOUTH),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.idle_passes, 0);
        assert_eq!(menu.idle_pass(), IntroMenuOutput::PresentMenu);
    }

    #[test]
    fn complete_subflow_with_save_ready_launches_gameplay() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'J');
        assert_eq!(
            menu.complete_subflow(IntroSubflow::JourneyOnward, IntroSubflowResult::SaveReady),
            IntroMenuOutput::LaunchGameplay
        );
        assert!(menu.is_launched());
    }

    #[test]
    fn character_creation_save_ready_returns_to_menu() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'C');
        assert_eq!(
            menu.complete_subflow(
                IntroSubflow::CharacterCreation,
                IntroSubflowResult::SaveReady
            ),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.phase, IntroMenuPhase::AwaitingSelection);
        assert!(!menu.is_launched());
    }

    #[test]
    fn complete_subflow_with_cancel_returns_to_menu() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'C');
        assert_eq!(
            menu.complete_subflow(
                IntroSubflow::CharacterCreation,
                IntroSubflowResult::Cancelled
            ),
            IntroMenuOutput::PresentMenu
        );
        assert_eq!(menu.phase, IntroMenuPhase::AwaitingSelection);
    }

    #[test]
    fn complete_subflow_with_returned_to_menu_re_arms_for_input() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'U');
        assert_eq!(
            menu.complete_subflow(
                IntroSubflow::StorySlides,
                IntroSubflowResult::ReturnedToMenu
            ),
            IntroMenuOutput::PresentMenu
        );
        // Another key should now produce a fresh sub-flow.
        assert_eq!(
            menu.step(b'R'),
            IntroMenuOutput::EnterSubflow(IntroSubflow::ReturnToView)
        );
    }

    #[test]
    fn step_inside_subflow_is_ignored() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'A');
        // Sub-flow has not yet reported back; keystrokes routed back to
        // the menu are ignored (they should go to the sub-flow instead).
        assert_eq!(menu.step(b'J'), IntroMenuOutput::IgnoredKey);
    }
}
