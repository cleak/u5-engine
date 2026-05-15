//! Intro-menu key dispatch per `intro.md` §6.

use crate::input_case_fold;

/// `intro.md §6`: the six accepted intro-menu actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroMenuAction {
    /// `J` — load the active save and return to the main loop on success.
    JourneyOnward,
    /// `C` — enter character creation through the proportional-font /
    /// chargen flow.
    CreateNewCharacter,
    /// `T` — enter the Ultima IV transfer/roster path.
    TransferFromUltimaIv,
    /// `U` — play the story slide sequence and return to the menu.
    UltimaVIntroduction,
    /// `A` — show acknowledgements/credits and return to the menu.
    Acknowledgements,
    /// `R` — run the non-interactive Return-to-View preview and return.
    ReturnToView,
    /// Repeat the most-recent cached selection (Enter when a cache is
    /// present); caller maintains the cache and resolves it back to one
    /// of the six actions above. This variant signals the intent rather
    /// than the resolved action.
    RepeatCachedSelection,
}

/// `intro.md §6`: classify a raw key byte into an intro-menu action.
/// Keys are case-folded before dispatch (matching `input.md §6`).
/// Returns `None` for invalid keys, which the menu silently ignores.
pub fn intro_menu_action(byte: u8) -> Option<IntroMenuAction> {
    let folded = input_case_fold(byte);
    Some(match folded {
        b'J' => IntroMenuAction::JourneyOnward,
        b'C' => IntroMenuAction::CreateNewCharacter,
        b'T' => IntroMenuAction::TransferFromUltimaIv,
        b'U' => IntroMenuAction::UltimaVIntroduction,
        b'A' => IntroMenuAction::Acknowledgements,
        b'R' => IntroMenuAction::ReturnToView,
        // Enter (CR or LF) reuses the cached selection if any.
        b'\r' | b'\n' => IntroMenuAction::RepeatCachedSelection,
        _ => return None,
    })
}
