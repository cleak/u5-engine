//! Intro-menu key dispatch per `intro.md` §6.

use crate::input_case_fold;

/// `intro.md §3`: title-screen 320x200 pixel placement for one
/// bitmap slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitleBitPlacement {
    pub asset: TitleBitAsset,
    pub slot: u8,
    pub top_left_x: u16,
    pub top_left_y: u16,
    pub width: u16,
    pub height: u16,
}

/// `intro.md §3`: which compressed-bitmap resource a placement
/// references.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleBitAsset {
    Title,
    British,
}

/// `intro.md §3` initial title mark — `TITLE.BIT` slots 0..=6 drawn
/// in ascending order.
pub const TITLE_BIT_INITIAL_PLACEMENTS: [TitleBitPlacement; 7] = [
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 0, top_left_x: 148, top_left_y: 0, width: 24, height: 3 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 1, top_left_x: 140, top_left_y: 3, width: 40, height: 7 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 2, top_left_x: 124, top_left_y: 10, width: 72, height: 11 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 3, top_left_x: 104, top_left_y: 21, width: 112, height: 20 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 4, top_left_x: 84, top_left_y: 41, width: 152, height: 32 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 5, top_left_x: 52, top_left_y: 73, width: 216, height: 45 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 6, top_left_x: 20, top_left_y: 118, width: 280, height: 61 },
];

/// `intro.md §3` four `BRITISH.PTH` pen origins, in the order the
/// path walker is called.
pub const BRITISH_PTH_PEN_ORIGINS: [(u8, u8); 4] =
    [(68, 44), (94, 64), (78, 143), (105, 167)];

/// `intro.md §3` remaining title-sequence bitmap placements drawn
/// after the seven-slot initial title mark. Order is `TITLE.BIT` 7,
/// `TITLE.BIT` 8, `BRITISH.BIT` 0, `TITLE.BIT` 9. The lower-band
/// clear at [`TITLE_LOWER_BAND_CLEAR_Y`] runs before slot 7.
pub const TITLE_BIT_REMAINING_PLACEMENTS: [TitleBitPlacement; 4] = [
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 7, top_left_x: 108, top_left_y: 140, width: 104, height: 33 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 8, top_left_x: 152, top_left_y: 0, width: 16, height: 15 },
    TitleBitPlacement { asset: TitleBitAsset::British, slot: 0, top_left_x: 24, top_left_y: 66, width: 272, height: 62 },
    TitleBitPlacement { asset: TitleBitAsset::Title, slot: 9, top_left_x: 104, top_left_y: 160, width: 112, height: 33 },
];

/// `intro.md §3` lower-screen Y where the title flow clears the
/// lower band before drawing `TITLE.BIT` slot 7.
pub const TITLE_LOWER_BAND_CLEAR_Y: u16 = 140;

/// `intro.md §5` title-tick frame rectangle. The intro menu's idle
/// title-tick path draws one driver-local frame strip over the
/// title screen at this fixed pixel rectangle, then advances the
/// driver-local frame index modulo four. The replacement frames
/// belong to a cleanroom renderer; the cadence and destination
/// rectangle are part of the public contract.
pub const TITLE_TICK_FRAME_X: u16 = 0;
pub const TITLE_TICK_FRAME_Y: u16 = 65;
pub const TITLE_TICK_FRAME_WIDTH: u16 = 320;
pub const TITLE_TICK_FRAME_HEIGHT: u16 = 49;
pub const TITLE_TICK_FRAME_COUNT: u8 = 4;

/// `intro.md §5`: advance the title-tick frame index modulo four.
pub const fn title_tick_next_frame(current_frame: u8) -> u8 {
    (current_frame + 1) % TITLE_TICK_FRAME_COUNT
}

/// `intro.md §12`: Return-to-View loads `MISCMAPS.DAT`. The first
/// four records are 19-by-4 map strips followed by a 655-byte
/// command stream driving preview actors and animation beats.
pub const MISCMAPS_DAT_FILE: &str = "MISCMAPS.DAT";
pub const RTV_STRIP_COUNT: usize = 4;
pub const RTV_STRIP_ROWS: usize = 19;
pub const RTV_STRIP_COLUMNS: usize = 4;
pub const RTV_COMMAND_STREAM_BYTES: usize = 655;
/// `intro.md §12`: Return-to-View command stream is interpreted as a
/// 16-command preview bytecode, not the gameplay TLK runner.
pub const RTV_COMMAND_COUNT: usize = 16;

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
