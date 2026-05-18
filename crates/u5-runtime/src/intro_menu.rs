//! Intro-menu state machine per `systems/intro.md` §6.
//!
//! The intro menu sits between the title presentation and live
//! gameplay. It polls one keystroke at a time and dispatches into one
//! of six sub-flows (Journey Onward, Create New Character, Transfer,
//! Story, Acknowledgements, Return-to-View) before returning to itself
//! to await the next selection.
//!
//! This module is the orchestrator: it owns the menu's current sub-
//! flow phase, the most-recent cached selection (for Enter-repeat),
//! and a thin enum of "what should the harness do next?" outputs. The
//! sub-flows themselves live in [`crate::chargen`], [`crate::u4_transfer`],
//! and [`crate::intro`]; the menu just sequences them.

use crate::intro::{IntroMenuAction, intro_menu_action};

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
#[derive(Clone, Copy, Debug, Default)]
pub struct IntroMenu {
    pub phase: IntroMenuPhase,
    pub cached_selection: Option<IntroSubflow>,
}

impl IntroMenu {
    pub fn new() -> Self {
        Self::default()
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
    pub fn step(&mut self, key: u8) -> IntroMenuOutput {
        if !matches!(self.phase, IntroMenuPhase::AwaitingSelection) {
            return IntroMenuOutput::IgnoredKey;
        }
        let Some(action) = intro_menu_action(key) else {
            return IntroMenuOutput::IgnoredKey;
        };
        let resolved = match action {
            IntroMenuAction::RepeatCachedSelection => match self.cached_selection {
                Some(sub) => sub,
                None => return IntroMenuOutput::IgnoredKey,
            },
            IntroMenuAction::JourneyOnward => IntroSubflow::JourneyOnward,
            IntroMenuAction::CreateNewCharacter => IntroSubflow::CharacterCreation,
            IntroMenuAction::TransferFromUltimaIv => IntroSubflow::UltimaIvTransfer,
            IntroMenuAction::UltimaVIntroduction => IntroSubflow::StorySlides,
            IntroMenuAction::Acknowledgements => IntroSubflow::Acknowledgements,
            IntroMenuAction::ReturnToView => IntroSubflow::ReturnToView,
        };
        self.cached_selection = Some(resolved);
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
        assert_eq!(menu.cached_selection, Some(IntroSubflow::JourneyOnward));
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

    #[test]
    fn enter_key_repeats_cached_selection() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        menu.step(b'A');
        menu.complete_subflow(
            IntroSubflow::Acknowledgements,
            IntroSubflowResult::ReturnedToMenu,
        );
        assert_eq!(
            menu.step(b'\r'),
            IntroMenuOutput::EnterSubflow(IntroSubflow::Acknowledgements)
        );
    }

    #[test]
    fn enter_key_without_cached_selection_is_ignored() {
        let mut menu = IntroMenu::new();
        menu.dismiss_title();
        assert_eq!(menu.step(b'\r'), IntroMenuOutput::IgnoredKey);
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
