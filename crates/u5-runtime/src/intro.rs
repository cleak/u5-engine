//! Intro-menu key dispatch per `intro.md` §6.

use std::io;
use std::path::Path;

use crate::{
    GraphicImage, GraphicImageDirectory, INPUT_CODE_EAST, INPUT_CODE_NORTH, INPUT_CODE_SOUTH,
    INPUT_CODE_WEST, TileGraphicsDepth, input_case_fold, load_graphic_image_directory,
    read_optional_disk_file,
};

/// `intro.md §3` title-screen surface dimensions. The title flow
/// places its bitmap slots inside a fixed 320-by-200 pixel coordinate
/// system with the origin at the upper-left corner. Promote the
/// width and height so the title-tick rectangle and the bitmap
/// placements share one named source of truth instead of comparing
/// against bare `320` / `200` literals.
pub const TITLE_SURFACE_WIDTH: u16 = 320;
pub const TITLE_SURFACE_HEIGHT: u16 = 200;

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

/// `intro.md §3` hidden initial title source placements. `TITLE.BIT`
/// slots 0..=6 are stacked into this off-screen source surface before
/// the title animation player reveals selected rows.
pub const TITLE_BIT_INITIAL_SOURCE_PLACEMENTS: [TitleBitPlacement; 7] = [
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 0,
        top_left_x: 148,
        top_left_y: 0,
        width: 24,
        height: 3,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 1,
        top_left_x: 140,
        top_left_y: 3,
        width: 40,
        height: 7,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 2,
        top_left_x: 124,
        top_left_y: 10,
        width: 72,
        height: 11,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 3,
        top_left_x: 104,
        top_left_y: 21,
        width: 112,
        height: 20,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 4,
        top_left_x: 84,
        top_left_y: 41,
        width: 152,
        height: 32,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 5,
        top_left_x: 52,
        top_left_y: 73,
        width: 216,
        height: 45,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 6,
        top_left_x: 20,
        top_left_y: 118,
        width: 280,
        height: 61,
    },
];

/// `intro.md §3` visible initial title mark placements. `TITLE.BIT`
/// slots 0..=6 are presented one at a time from the hidden source
/// surface, replacing the previous visible flourish frame rather than
/// accumulating.
pub const TITLE_BIT_INITIAL_PLACEMENTS: [TitleBitPlacement; 7] = [
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 0,
        top_left_x: 148,
        top_left_y: 75,
        width: 24,
        height: 3,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 1,
        top_left_x: 140,
        top_left_y: 72,
        width: 40,
        height: 7,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 2,
        top_left_x: 124,
        top_left_y: 71,
        width: 72,
        height: 11,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 3,
        top_left_x: 104,
        top_left_y: 66,
        width: 112,
        height: 20,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 4,
        top_left_x: 84,
        top_left_y: 60,
        width: 152,
        height: 32,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 5,
        top_left_x: 52,
        top_left_y: 53,
        width: 216,
        height: 45,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 6,
        top_left_x: 20,
        top_left_y: 46,
        width: 280,
        height: 61,
    },
];

/// `intro.md §3` "Flourish playback: seven frames, seven reveal steps,
/// six erase steps" (`cleak/u5-spec#67`, correcting the earlier
/// 67-group table).
///
/// The shipped presentation script has eight row groups per frame, 56
/// in total, but the first group of every frame is always empty and
/// the groups are consumed in reverse, so exactly **seven reveal
/// steps** per frame are ever presented. Between two consecutive
/// frames — after frames 0..=5, never after frame 6 — the driver runs
/// **six erase steps** on the frame just shown. Erase step `j` removes
/// the rows named in reveal column `8 - j`, so the visible set walks
/// back down through the same cumulative unions. Total:
/// `7 * 7 + 6 * 6 = 85` presentation steps.
///
/// Row numbers are relative to the frame's own top row, and the sets
/// are cumulative: reveal step `k` shows the union of columns
/// `1..=k`.
pub const TITLE_FLOURISH_FRAME_COUNT: usize = 7;
pub const TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME: usize = 7;
pub const TITLE_FLOURISH_ERASE_STEPS_PER_FRAME: usize = 6;

pub const TITLE_FLOURISH_REVEAL_SETS: [[&[u8]; TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME];
    TITLE_FLOURISH_FRAME_COUNT] = [
    // Frame 0 (3 rows)
    [&[0, 2], &[], &[], &[], &[1], &[], &[]],
    // Frame 1 (7 rows)
    [&[0, 6], &[3], &[], &[2, 4], &[], &[1, 5], &[]],
    // Frame 2 (11 rows)
    [&[0, 10], &[5], &[4, 6], &[1, 9], &[3, 7], &[2, 8], &[]],
    // Frame 3 (20 rows)
    [
        &[0, 19],
        &[9, 10],
        &[3, 6, 13, 16],
        &[2, 8, 11, 17],
        &[5, 14],
        &[1, 7, 12, 18],
        &[4, 15],
    ],
    // Frame 4 (32 rows)
    [
        &[0, 31],
        &[5, 10, 15, 16, 21, 26],
        &[4, 9, 14, 17, 22, 27],
        &[1, 6, 11, 20, 25, 30],
        &[3, 8, 13, 18, 23, 28],
        &[2, 12, 19, 29],
        &[7, 24],
    ],
    // Frame 5 (45 rows). Shipped-data quirk, part of the contract:
    // row 19 is named twice (reveals 3 and 6) and row 29 is never
    // named, so row 29 stays blank for the whole of frame 5.
    [
        &[0, 44],
        &[7, 14, 21, 23, 30, 37],
        &[2, 5, 9, 12, 16, 19, 25, 28, 32, 35, 39, 42],
        &[3, 10, 17, 22, 27, 34, 41],
        &[6, 13, 20, 24, 31, 38],
        &[1, 8, 15, 19, 36, 43],
        &[4, 11, 18, 26, 33, 40],
    ],
    // Frame 6 (61 rows)
    [
        &[0, 60],
        &[30, 40, 50, 20, 10],
        &[25, 15, 5, 35, 45, 55],
        &[27, 22, 17, 12, 7, 2, 33, 38, 43, 48, 53, 58],
        &[29, 24, 19, 14, 9, 4, 31, 36, 41, 46, 51, 56],
        &[26, 21, 16, 11, 6, 1, 34, 39, 44, 49, 54, 59],
        &[28, 23, 18, 13, 8, 3, 32, 37, 42, 47, 52, 57],
    ],
];

/// `intro.md §3`: total presentation steps in the flourish script.
pub const fn title_flourish_total_steps() -> usize {
    TITLE_FLOURISH_FRAME_COUNT * TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME
        + (TITLE_FLOURISH_FRAME_COUNT - 1) * TITLE_FLOURISH_ERASE_STEPS_PER_FRAME
}

/// One presentation step of the flourish script: which frame is on
/// screen, and how many cumulative reveal columns of that frame are
/// currently visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitleFlourishStep {
    /// `TITLE.BIT` slot 0..=6.
    pub frame: usize,
    /// Visible set is the union of reveal columns `1..=revealed_columns`.
    /// Reveal step `k` gives `k`; erase step `j` gives `7 - j`, so the
    /// reveal-1 set is never erased.
    pub revealed_columns: usize,
    /// True while the frame is still filling, false during its erase
    /// tail. Presentation is identical either way; this is only for
    /// diagnostics and tests.
    pub revealing: bool,
}

/// `intro.md §3`: resolve a global presentation-step index to its
/// frame and cumulative reveal depth. Returns `None` past the end of
/// the script.
pub fn title_flourish_step_state(step: usize) -> Option<TitleFlourishStep> {
    let reveals = TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME;
    let erases = TITLE_FLOURISH_ERASE_STEPS_PER_FRAME;
    let mut remaining = step;
    for frame in 0..TITLE_FLOURISH_FRAME_COUNT {
        if remaining < reveals {
            return Some(TitleFlourishStep {
                frame,
                revealed_columns: remaining + 1,
                revealing: true,
            });
        }
        remaining -= reveals;
        if frame + 1 == TITLE_FLOURISH_FRAME_COUNT {
            break;
        }
        if remaining < erases {
            return Some(TitleFlourishStep {
                frame,
                // Erase step j = remaining + 1 removes reveal column
                // `8 - j`, leaving columns 1..=(7 - j) visible.
                revealed_columns: reveals - (remaining + 1),
                revealing: false,
            });
        }
        remaining -= erases;
    }
    None
}

/// `intro.md §3`: the frame-local source rows visible at a given
/// cumulative reveal depth, ascending and deduplicated. Frame 5 names
/// row 19 twice, so deduplication is part of the contract.
pub fn title_flourish_visible_rows(frame: usize, revealed_columns: usize) -> Vec<u8> {
    let sets = TITLE_FLOURISH_REVEAL_SETS
        .get(frame)
        .unwrap_or_else(|| panic!("title flourish frame {frame} is outside 0..7"));
    assert!(
        revealed_columns <= TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME,
        "title flourish reveal depth {revealed_columns} exceeds {TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME}"
    );
    let mut rows: Vec<u8> = sets
        .iter()
        .take(revealed_columns)
        .flat_map(|set| set.iter().copied())
        .collect();
    rows.sort_unstable();
    rows.dedup();
    rows
}

/// `intro.md §3`: the inclusive destination band a presentation of
/// `frame` repaints, as `(top_row, height)`. Rows are copied and
/// blanked at the full 320-pixel screen width.
///
/// Odd frames are filled bottom-up, which draws them vertically
/// mirrored and shifted one row down: their band is
/// `band_top + 1 ..= band_top + height` instead of
/// `band_top ..= band_top + height - 1`.
pub fn title_flourish_band(frame: usize) -> (usize, usize) {
    let placement = TITLE_BIT_INITIAL_PLACEMENTS
        .iter()
        .find(|placement| usize::from(placement.slot) == frame)
        .unwrap_or_else(|| panic!("title flourish frame {frame} has no visible placement"));
    let top = usize::from(placement.top_left_y) + usize::from(title_flourish_band_shift(frame));
    (top, usize::from(placement.height))
}

/// `intro.md §3`: odd frames are shifted one row down.
pub const fn title_flourish_band_shift(frame: usize) -> u8 {
    (frame % 2) as u8
}

/// `intro.md §3`: even frames fill top-down, odd frames bottom-up.
pub const fn title_flourish_fills_top_down(frame: usize) -> bool {
    frame % 2 == 0
}

/// `intro.md §3`: the band slot each element of the packed, centred
/// content column is written to.
///
/// The content column is `floor(c / 2)` blank rows, then every visible
/// source row in ascending order, then `ceil(c / 2)` blank rows, where
/// `c` is the number of hidden rows. On an even frame element `k`
/// lands at `band_top + k`; on an odd frame the band is written from
/// its last row upward, so element `k` lands at `band_top + height - k`.
pub fn title_flourish_content_row(frame: usize, index: usize) -> usize {
    let placement = TITLE_BIT_INITIAL_PLACEMENTS
        .iter()
        .find(|placement| usize::from(placement.slot) == frame)
        .unwrap_or_else(|| panic!("title flourish frame {frame} has no visible placement"));
    let band_top = usize::from(placement.top_left_y);
    let height = usize::from(placement.height);
    assert!(
        index < height,
        "title flourish content index {index} exceeds frame {frame} height {height}"
    );
    if title_flourish_fills_top_down(frame) {
        band_top + index
    } else {
        band_top + height - index
    }
}

/// `intro.md §3` four `BRITISH.PTH` pen origins, in the order the
/// path walker is called.
pub const BRITISH_PTH_PEN_ORIGINS: [(u8, u8); 4] = [(68, 44), (94, 64), (78, 143), (105, 167)];

/// `intro.md §6.1` lower intro menu/text-window frame: anchor cell
/// (column, row) where the 40-wide × 10-tall rectangle begins.
pub const INTRO_MENU_FRAME_ANCHOR_COLUMN: u8 = 0;
pub const INTRO_MENU_FRAME_ANCHOR_ROW: u8 = 15;
pub const INTRO_MENU_FRAME_WIDTH_CELLS: u8 = 40;
pub const INTRO_MENU_FRAME_HEIGHT_CELLS: u8 = 10;
/// `intro.md §6.1` horizontal-rule pixel coordinates drawn through
/// the display-driver line primitive immediately under the top-edge
/// row.
pub const INTRO_MENU_FRAME_RULE_Y: u16 = 127;
pub const INTRO_MENU_FRAME_RULE_X0: u16 = 7;
pub const INTRO_MENU_FRAME_RULE_X1: u16 = 312;
/// Runtime observation of the original's lower intro menu frame,
/// pending the spec correction tracked as `cleak/u5-spec#78`.
///
/// `systems/intro.md §6.1` describes this frame as a single-line
/// rectangle built from five reserved box-drawing glyphs in the
/// intro's bright foreground index. A black-box capture of the
/// original shows instead the same rounded blue chrome the gameplay
/// border uses: pixel rows 120..=199 filled with EGA index 1 behind a
/// 1-pixel index-15 rectangle, a black interior, and two captions
/// drawn over the border rows.
pub const INTRO_MENU_FRAME_BORDER_COLOR: u8 = 0x01;
pub const INTRO_MENU_FRAME_OUTLINE_COLOR: u8 = 0x0f;
pub const INTRO_MENU_FRAME_INTERIOR_COLOR: u8 = 0x00;
/// First and last pixel row of the blue border band. Anchored to the
/// published `§6.1` cell rectangle: rows 15..=24 of the 8-pixel text
/// grid.
pub const INTRO_MENU_FRAME_TOP_Y: u16 = INTRO_MENU_FRAME_ANCHOR_ROW as u16 * 8;
pub const INTRO_MENU_FRAME_BOTTOM_Y: u16 =
    INTRO_MENU_FRAME_TOP_Y + INTRO_MENU_FRAME_HEIGHT_CELLS as u16 * 8 - 1;
/// `cleak/u5-spec#78` corner-rounding profile: the left-edge column
/// at which the blue fill starts, for each of the first six rows of
/// the band. Rows past the profile start at column 0, and the bottom
/// six rows mirror the profile in reverse. The right edge mirrors
/// each entry about the surface centre.
///
/// The gameplay border frame carves its outer corners with the same
/// measured staircase, so the numbers live once in
/// [`crate::gameplay_chrome::CHROME_CORNER_PROFILE`] and this name
/// stays as the `§6.1`-facing alias.
pub const INTRO_MENU_FRAME_CORNER_PROFILE: [u16; 6] = crate::gameplay_chrome::CHROME_CORNER_PROFILE;
/// `intro.md §6.1` horizontal-rule pixel coordinates. Observation
/// confirms the published top rule and adds the matching bottom rule
/// plus the two verticals that close the rectangle.
pub const INTRO_MENU_FRAME_BOTTOM_RULE_Y: u16 = 192;
pub const INTRO_MENU_FRAME_OUTLINE_LEFT_X: u16 = INTRO_MENU_FRAME_RULE_X0;
pub const INTRO_MENU_FRAME_OUTLINE_RIGHT_X: u16 = INTRO_MENU_FRAME_RULE_X1;
pub const INTRO_MENU_FRAME_INTERIOR_TOP_Y: u16 = INTRO_MENU_FRAME_RULE_Y + 1;
pub const INTRO_MENU_FRAME_INTERIOR_BOTTOM_Y: u16 = INTRO_MENU_FRAME_BOTTOM_RULE_Y - 1;
pub const INTRO_MENU_FRAME_INTERIOR_LEFT_X: u16 = INTRO_MENU_FRAME_OUTLINE_LEFT_X + 1;
pub const INTRO_MENU_FRAME_INTERIOR_RIGHT_X: u16 = INTRO_MENU_FRAME_OUTLINE_RIGHT_X - 1;

/// `cleak/u5-spec#78` border captions. Both are drawn as ordinary
/// white-on-black fixed cells over the blue border rows and visibly
/// interrupt the white rules, exactly like the gameplay border's wind
/// label.
pub const INTRO_MENU_SELECT_CAPTION_PREFIX: &str = ">Select:";
pub const INTRO_MENU_SELECT_CAPTION_SUFFIX: &str = "<";
/// One sample of the caption's cursor cell: `IBM.CH` glyph 8, measured
/// directly from the capture's cell 23 of text row 15.
///
/// It is **one phase of a four-phase cycle**, not a fixed glyph.
/// `intro.md §6.1` "The cursor cell": "The cell parked at row 15,
/// column 23 is not an on/off blink. It cycles the same four
/// consecutive fixed-cell glyph codes `0x05` through `0x08` that the
/// gameplay message window's input cursor uses (`text-output.md`
/// section 10.6), one phase per menu poll pass." Resolve the live cell
/// with [`intro_menu_select_caption_cursor_glyph`]; this constant
/// remains only as the measured sample.
pub const INTRO_MENU_SELECT_CAPTION_CURSOR_GLYPH: u8 =
    crate::gameplay_chrome::PROMPT_CURSOR_FRAME_GLYPHS[3];

/// `intro.md §6.1` "The cursor cell": the caption's cursor cell cycles
/// the four consecutive fixed-cell glyph codes `0x05..=0x08` - "the
/// same four ... that the gameplay message window's input cursor uses"
/// - advancing **one phase per menu poll pass**. Each of the four is a
/// diagonal hatch of two-pixel steps shifted two pixels along, so the
/// cell reads as a diagonal pattern marching across it.
///
/// Because the cycle is shared with the gameplay prompt this delegates
/// to [`crate::gameplay_chrome::prompt_cursor_glyph`] rather than
/// restating the table.
///
/// The spec publishes the four codes, their order and the cadence, but
/// **not** which phase the menu's very first poll pass shows; pass `0`
/// is taken as `0x05`, the first code of the published run. The
/// measured [`INTRO_MENU_SELECT_CAPTION_CURSOR_GLYPH`] sample is
/// consistent with any origin, since a capture catches an arbitrary
/// pass.
pub fn intro_menu_select_caption_cursor_glyph(pass: u64) -> u8 {
    crate::gameplay_chrome::prompt_cursor_glyph(pass)
}

/// `intro.md §6.1` "The cursor cell": "The instant a poll returns a key
/// the cell is overwritten with a space."
pub const INTRO_MENU_SELECT_CAPTION_CURSOR_BLANK: u8 = b' ';
pub const INTRO_MENU_SELECT_CAPTION_COLUMN: u8 = 15;
pub const INTRO_MENU_SELECT_CAPTION_ROW: u8 = 15;
pub const INTRO_MENU_COPYRIGHT_CAPTION: &str = ">Copyright 1988 Lord British<";
pub const INTRO_MENU_COPYRIGHT_CAPTION_COLUMN: u8 = 5;
pub const INTRO_MENU_COPYRIGHT_CAPTION_ROW: u8 = 24;

/// `cleak/u5-spec#78`: the left-edge column at which the blue border
/// fill starts on `row`, or `None` when the row is outside the band.
pub fn intro_menu_frame_border_start_column(row: u16) -> Option<u16> {
    if row < INTRO_MENU_FRAME_TOP_Y || row > INTRO_MENU_FRAME_BOTTOM_Y {
        return None;
    }
    let profile_len = INTRO_MENU_FRAME_CORNER_PROFILE.len() as u16;
    let from_top = row - INTRO_MENU_FRAME_TOP_Y;
    let from_bottom = INTRO_MENU_FRAME_BOTTOM_Y - row;
    if from_top < profile_len {
        Some(INTRO_MENU_FRAME_CORNER_PROFILE[from_top as usize])
    } else if from_bottom < profile_len {
        Some(INTRO_MENU_FRAME_CORNER_PROFILE[from_bottom as usize])
    } else {
        Some(0)
    }
}

/// `intro.md §3` remaining title-sequence bitmap placements drawn
/// after the seven-slot initial title mark. Order is `TITLE.BIT` 7,
/// `TITLE.BIT` 8, `BRITISH.BIT` 0, `TITLE.BIT` 9. The lower-band
/// clear at [`TITLE_LOWER_BAND_CLEAR_Y`] runs before slot 7.
pub const TITLE_BIT_REMAINING_PLACEMENTS: [TitleBitPlacement; 4] = [
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 7,
        top_left_x: 108,
        top_left_y: 140,
        width: 104,
        height: 33,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 8,
        top_left_x: 152,
        top_left_y: 0,
        width: 16,
        height: 15,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::British,
        slot: 0,
        top_left_x: 24,
        top_left_y: 66,
        width: 272,
        height: 62,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 9,
        top_left_x: 104,
        top_left_y: 160,
        width: 112,
        height: 33,
    },
];

/// `intro.md §3` lower-screen Y where the title flow clears the
/// lower band before drawing `TITLE.BIT` slot 7.
pub const TITLE_LOWER_BAND_CLEAR_Y: u16 = 140;

/// `intro.md §5` title-tick frame rectangle. The intro menu's idle
/// title-tick path draws one frame strip over the title screen at
/// this fixed pixel rectangle, then advances the frame index modulo
/// four. The destination rectangle is the published 320-by-49 band
/// at `(0, 65)`; the source pixels only cover its central 288
/// columns (see [`TITLE_TICK_SOURCE_X`]).
pub const TITLE_TICK_FRAME_X: u16 = 0;
pub const TITLE_TICK_FRAME_Y: u16 = 65;
pub const TITLE_TICK_FRAME_WIDTH: u16 = TITLE_SURFACE_WIDTH;
pub const TITLE_TICK_FRAME_HEIGHT: u16 = 49;
pub const TITLE_TICK_FRAME_COUNT: u8 = 4;

/// Runtime observation of the shipped `ULTIMA.16` asset, pending the
/// spec correction tracked as `cleak/u5-spec#78`.
///
/// `systems/intro.md §5` and `systems/display-driver.md §8` claim the
/// four flaming "Warriors of Destiny" bands live only inside the EGA
/// driver's runtime back-buffer and cannot be read from an external
/// art file. Decoding the local `ULTIMA.16` image directory
/// contradicts that: it carries five panels — slot 0 is the 319-by-61
/// "Ultima V" logo and slots 1..=4 are the four 288-wide title-tick
/// bands (three at 49 rows, the last at 50). A black-box capture of
/// the original running the same assets matches those panels exactly
/// at `(0, 0)` and `(16, 65)` respectively, so the engine renders from
/// the asset rather than from authored replacement art.
pub const ULTIMA_PANEL_STEM: &str = "ULTIMA";
/// `cleak/u5-spec#78`: `ULTIMA` slot 0 is the menu logo, blitted at
/// the surface origin.
pub const ULTIMA_LOGO_SLOT: u8 = 0;
pub const ULTIMA_LOGO_WIDTH: usize = 319;
pub const ULTIMA_LOGO_HEIGHT: usize = 61;
/// `cleak/u5-spec#78`: `ULTIMA` slots 1..=4 are title-tick frames
/// 0..=3. Frame 0 is slot 1 — the slot-to-frame mapping is assumed to
/// be the natural directory order because the captures only pin slots
/// 2 and 3 (which match the settled menu), and the four-frame loop
/// makes any rotation visually equivalent after one cycle.
pub const ULTIMA_TITLE_TICK_FIRST_SLOT: u8 = 1;
/// `cleak/u5-spec#65`: staging offset of the 288-wide record inside
/// the hidden surface. The loader clears the hidden surface and draws
/// records 1..=4 at `(16, 0)`, `(16, 50)`, `(16, 100)`, `(16, 150)`;
/// each tick then copies 49 rows at the **full 320-pixel width** from
/// hidden row `50 * frame` to visible rows `65..=113`. Columns
/// `0..=15` and `304..=319` are part of the destination rectangle and
/// carry the cleared staging background, so the tick is an opaque
/// full-rectangle overwrite.
pub const TITLE_TICK_SOURCE_X: u16 = 16;
pub const TITLE_TICK_SOURCE_WIDTH: u16 = 288;
/// `cleak/u5-spec#65` hidden-surface row pitch between consecutive
/// staged bands. It is a driver constant, not a record height: in the
/// `.4` depth every record is 49 rows tall and the 50th row of each
/// band is simply staging background.
pub const TITLE_TICK_SOURCE_ROW_PITCH: usize = 50;
/// `cleak/u5-spec#65` staging background: the hidden surface is
/// cleared before the records are drawn.
pub const TITLE_TICK_STAGING_BACKGROUND: u8 = 0;
/// `cleak/u5-spec#78`: `ULTIMA` slot 4 is authored with a 50th row
/// that the destination rectangle does not consume; only the upper
/// [`TITLE_TICK_FRAME_HEIGHT`] rows of each panel are copied, which
/// corroborates the published "50-row source stride, upper 49 rows
/// copied" rule.
pub const TITLE_TICK_SOURCE_MAX_HEIGHT: usize = 50;

/// A staged frame is the full published destination rectangle, not
/// the 288-wide record: `cleak/u5-spec#65` is explicit that the flanks
/// are part of the rectangle and are repainted every tick.
pub const TITLE_TICK_FRAME_PIXELS: usize =
    TITLE_TICK_FRAME_WIDTH as usize * TITLE_TICK_FRAME_HEIGHT as usize;
pub const TITLE_TICK_FRAME_SET_BYTES: usize =
    TITLE_TICK_FRAME_PIXELS * TITLE_TICK_FRAME_COUNT as usize;

/// `intro.md §5`: advance the title-tick frame index modulo four.
pub const fn title_tick_next_frame(current_frame: u8) -> u8 {
    (current_frame + 1) % TITLE_TICK_FRAME_COUNT
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleTickFrameSet {
    pixels: Vec<u8>,
}

impl TitleTickFrameSet {
    pub fn from_palette_indices(pixels: Vec<u8>, source: &str) -> io::Result<Self> {
        if pixels.len() != TITLE_TICK_FRAME_SET_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{source}: title-tick frame set must be exactly {TITLE_TICK_FRAME_SET_BYTES} bytes ({} frames of {}x{}), found {}",
                    TITLE_TICK_FRAME_COUNT,
                    TITLE_TICK_FRAME_WIDTH,
                    TITLE_TICK_FRAME_HEIGHT,
                    pixels.len()
                ),
            ));
        }
        if let Some((index, value)) = pixels
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| *value > 0x0f)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{source}: authored title-tick frame byte {index} has palette index 0x{value:02x}; expected EGA index 0x00..0x0f"
                ),
            ));
        }
        Ok(Self { pixels })
    }

    pub fn frame_pixels(&self, frame: u8) -> &[u8] {
        let frame = usize::from(frame % TITLE_TICK_FRAME_COUNT);
        let start = frame * TITLE_TICK_FRAME_PIXELS;
        &self.pixels[start..start + TITLE_TICK_FRAME_PIXELS]
    }
}

/// `cleak/u5-spec#78` title-tick source loader. Reads the local
/// `ULTIMA` image directory and emits the four-frame strip from slots
/// [`ULTIMA_TITLE_TICK_FIRST_SLOT`]..=`+3`, taking the upper
/// [`TITLE_TICK_FRAME_HEIGHT`] rows of each 288-wide panel.
///
/// This is a runtime read of a local asset, not authored replacement
/// art: the panels are the original flaming "Warriors of Destiny"
/// bands, and a black-box capture of the original confirms them at
/// `(16, 65)`.
pub fn load_ultima_title_tick_frames(
    game_dir: &Path,
    depth: TileGraphicsDepth,
) -> io::Result<TitleTickFrameSet> {
    let directory = load_graphic_image_directory(game_dir, ULTIMA_PANEL_STEM, depth)?;
    parse_ultima_title_tick_frames(&directory)
}

/// `cleak/u5-spec#78`: extract the four title-tick frames from an
/// already-decoded `ULTIMA` image directory.
pub fn parse_ultima_title_tick_frames(
    directory: &GraphicImageDirectory,
) -> io::Result<TitleTickFrameSet> {
    let record_width = TITLE_TICK_SOURCE_WIDTH as usize;
    let band_width = TITLE_TICK_FRAME_WIDTH as usize;
    let height = TITLE_TICK_FRAME_HEIGHT as usize;
    let record_x = TITLE_TICK_SOURCE_X as usize;
    // `cleak/u5-spec#65` gives two equivalent implementations of the
    // staging: reproduce the hidden surface literally, or composite
    // each record onto a background-filled 320-wide canvas at x = 16
    // and blit that. This takes the second; the result is the exact
    // 320-by-49 destination rectangle the tick overwrites.
    let mut pixels = vec![TITLE_TICK_STAGING_BACKGROUND; TITLE_TICK_FRAME_SET_BYTES];
    for frame in 0..TITLE_TICK_FRAME_COUNT {
        let slot = usize::from(ULTIMA_TITLE_TICK_FIRST_SLOT + frame);
        let panel = directory
            .images
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{ULTIMA_PANEL_STEM} image directory is missing title-tick panel slot {slot}"
                    ),
                )
            })?;
        // The `.16` depth stores record 4 with 50 rows and the `.4`
        // depth stores it with 49; the 50-row pitch is a driver
        // constant, so either is well-formed and only the upper
        // `height` rows are ever shown.
        if panel.width != record_width
            || panel.height < height
            || panel.height > TITLE_TICK_SOURCE_MAX_HEIGHT
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ULTIMA_PANEL_STEM} title-tick panel slot {slot} is {}x{}, expected {record_width} wide and {height}..={TITLE_TICK_SOURCE_MAX_HEIGHT} rows tall",
                    panel.width, panel.height
                ),
            ));
        }
        let frame_base = usize::from(frame) * TITLE_TICK_FRAME_PIXELS;
        for row in 0..height {
            let dst = frame_base + row * band_width + record_x;
            let src = row * record_width;
            pixels[dst..dst + record_width].copy_from_slice(&panel.pixels[src..src + record_width]);
        }
    }
    TitleTickFrameSet::from_palette_indices(pixels, "ULTIMA title-tick panels")
}

/// `cleak/u5-spec#78` menu-logo loader. Reads `ULTIMA` slot 0, the
/// 319-by-61 "Ultima V" logo the original blits at the surface origin
/// on the start/menu screen.
pub fn load_ultima_logo_panel(
    game_dir: &Path,
    depth: TileGraphicsDepth,
) -> io::Result<GraphicImage> {
    let directory = load_graphic_image_directory(game_dir, ULTIMA_PANEL_STEM, depth)?;
    let panel = directory
        .images
        .get(usize::from(ULTIMA_LOGO_SLOT))
        .and_then(Option::as_ref)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ULTIMA_PANEL_STEM} image directory is missing logo slot {ULTIMA_LOGO_SLOT}"
                ),
            )
        })?;
    if (panel.width, panel.height) != (ULTIMA_LOGO_WIDTH, ULTIMA_LOGO_HEIGHT) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{ULTIMA_PANEL_STEM} logo slot {ULTIMA_LOGO_SLOT} is {}x{}, expected {ULTIMA_LOGO_WIDTH}x{ULTIMA_LOGO_HEIGHT}",
                panel.width, panel.height
            ),
        ));
    }
    Ok(panel.clone())
}

/// Four-frame strip of all-zero (black) pixels satisfying the public
/// destination contract (288×49 per frame, four frames, in-range EGA
/// palette indices). Test-only scaffolding for the destination-blit
/// geometry; no render path uses it.
pub fn placeholder_title_tick_frames() -> TitleTickFrameSet {
    TitleTickFrameSet::from_palette_indices(
        vec![0u8; TITLE_TICK_FRAME_SET_BYTES],
        "placeholder title-tick frames",
    )
    .expect("placeholder title-tick frame set is well-formed by construction")
}

/// `intro.md §12`: Return-to-View loads `MISCMAPS.DAT`. The first
/// four records are the preview map strips, followed by a 655-byte
/// command stream driving preview actors and animation beats.
///
/// `formats/location-dat.md §11`: "Each record is **four 32-byte
/// rows**; within a row the first nineteen bytes carry tile data and
/// the trailing thirteen bytes are unused padding. The strip is
/// therefore wide and short, which is also what the preview displays:
/// nineteen tiles across by four tiles down." At sixteen pixels per
/// tile the strip is 304 x 64 pixels. The earlier 4-columns-by-19-rows
/// reading is transposed and withdrawn; the strip geometry lives in
/// [`crate::return_to_view`] as `RTV_STRIP_VISIBLE_COLUMNS` (19) and
/// `RTV_STRIP_VISIBLE_ROWS` (4), which are the single source of truth.
pub const MISCMAPS_DAT_FILE: &str = "MISCMAPS.DAT";
pub const RTV_STRIP_COUNT: usize = 4;
pub const RTV_COMMAND_STREAM_BYTES: usize = 655;

/// `formats/location-dat.md §11` MISCMAPS section offsets. The file
/// concatenates three sections back-to-back: four cutscene maps
/// (704 bytes), four Return-to-View strips (512 bytes), and the
/// 655-byte Return-to-View command stream. Promote the offsets and
/// per-section lengths so loader code does not bake them as bare
/// literals.
pub const MISCMAPS_CUTSCENE_SECTION_OFFSET: usize = 0;
/// `formats/location-dat.md §11` byte length of the cutscene-map
/// section: four maps of `16 × 11 = 176` bytes each (eleven 16-byte
/// rows with five trailing pad bytes), totalling 704 bytes.
pub const MISCMAPS_CUTSCENE_SECTION_BYTES: usize = 704;
/// `formats/location-dat.md §11` per-cutscene-map row stride. Each
/// cutscene map is authored as eleven 16-byte rows; the first 11
/// bytes of each row carry tile data and the trailing five bytes
/// are zero-padded.
pub const MISCMAPS_CUTSCENE_ROW_STRIDE: usize = 16;
pub const MISCMAPS_CUTSCENE_ROWS: usize = 11;
pub const MISCMAPS_CUTSCENE_VISIBLE_COLUMNS: usize = 11;
pub const MISCMAPS_CUTSCENE_RECORD_COUNT: usize = 4;
pub const MISCMAPS_CUTSCENE_RECORD_BYTES: usize =
    MISCMAPS_CUTSCENE_ROW_STRIDE * MISCMAPS_CUTSCENE_ROWS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiscmapsCutsceneMap {
    pub record_index: usize,
    pub tiles: Vec<u8>,
}

impl MiscmapsCutsceneMap {
    pub fn tile(&self, x: usize, y: usize) -> Option<u8> {
        (x < MISCMAPS_CUTSCENE_VISIBLE_COLUMNS && y < MISCMAPS_CUTSCENE_ROWS)
            .then(|| self.tiles[y * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS + x])
    }
}

pub fn load_miscmaps_cutscene_map(
    game_dir: &Path,
    record_index: usize,
) -> io::Result<Option<MiscmapsCutsceneMap>> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_miscmaps_cutscene_map_file(&bytes, record_index).map(Some)
}

pub fn require_miscmaps_cutscene_map(
    game_dir: &Path,
    record_index: usize,
) -> io::Result<MiscmapsCutsceneMap> {
    load_miscmaps_cutscene_map(game_dir, record_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("{MISCMAPS_DAT_FILE}: required cutscene map resource is missing"),
        )
    })
}

pub fn parse_miscmaps_cutscene_map_file(
    bytes: &[u8],
    record_index: usize,
) -> io::Result<MiscmapsCutsceneMap> {
    if record_index >= MISCMAPS_CUTSCENE_RECORD_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "MISCMAPS cutscene record must be 0..{}, got {record_index}",
                MISCMAPS_CUTSCENE_RECORD_COUNT - 1
            ),
        ));
    }
    let start = MISCMAPS_CUTSCENE_SECTION_OFFSET + record_index * MISCMAPS_CUTSCENE_RECORD_BYTES;
    let end = start + MISCMAPS_CUTSCENE_RECORD_BYTES;
    if bytes.len() < end {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!(
                "{MISCMAPS_DAT_FILE}: expected at least {end} bytes for cutscene record {record_index}, found {}",
                bytes.len()
            ),
        ));
    }

    let mut tiles = Vec::with_capacity(MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS);
    for row in 0..MISCMAPS_CUTSCENE_ROWS {
        let row_start = start + row * MISCMAPS_CUTSCENE_ROW_STRIDE;
        tiles.extend_from_slice(&bytes[row_start..row_start + MISCMAPS_CUTSCENE_VISIBLE_COLUMNS]);
    }
    Ok(MiscmapsCutsceneMap {
        record_index,
        tiles,
    })
}

/// `formats/location-dat.md §11` Return-to-View map strip section
/// offset (immediately after the cutscene section).
pub const MISCMAPS_RTV_STRIP_SECTION_OFFSET: usize = MISCMAPS_CUTSCENE_SECTION_BYTES;
/// `formats/location-dat.md §11` Return-to-View map strip section
/// byte length: four strips stored as four 32-byte source columns each,
/// totalling `4 * 32 * 4 = 512` bytes. The Return-to-View loader
/// loads the visible 4x19 source cells into the public 4x19
/// preview geometry.
pub const MISCMAPS_RTV_STRIP_SECTION_BYTES: usize = 512;
/// `formats/location-dat.md §11` per-strip column stride. Each strip
/// is authored as four 32-byte source columns; the first 19 bytes per
/// column carry tile data and the trailing 13 bytes are zero-padded.
pub const MISCMAPS_RTV_STRIP_ROW_STRIDE: usize = 32;

/// `formats/location-dat.md §11` Return-to-View command stream
/// section offset (immediately after the strip section).
pub const MISCMAPS_RTV_COMMAND_SECTION_OFFSET: usize =
    MISCMAPS_RTV_STRIP_SECTION_OFFSET + MISCMAPS_RTV_STRIP_SECTION_BYTES;
/// `intro.md §12`: Return-to-View command stream is interpreted as a
/// 16-command preview bytecode, not the gameplay TLK runner.
pub const RTV_COMMAND_COUNT: usize = 16;

/// `intro.md §11` acknowledgements gate for callers with **no pixel
/// surface**.
///
/// `cleak/u5-spec#72` is closed and fully answered: the acknowledgement
/// screen is a pre-rendered image page whose credit lines are drawn
/// into the `STARTSC` bitmap. Nothing typesets the credits - no font
/// selection, no text rectangle, no printable character contributes to
/// the credits page - so there is no exact text to publish, no per-line
/// layout, and no pagination. The graphical intro draws that artwork
/// through the §11.2 phase sequence in
/// [`crate::intro_acknowledgements`].
///
/// The terminal harness has no pixel surface, and printing
/// clean-room-authored credit lines in its place would invent the one
/// thing the original never typesets. It refuses instead.
pub fn require_graphical_acknowledgements_surface() -> ! {
    panic!(
        "intro acknowledgements are the §11 credits artwork screen: the credit lines are drawn into the STARTSC bitmap, so they can only be presented by the graphical intro renderer. The terminal harness has no pixel surface to draw them on, and substituting clean-room-authored placeholder credits is a forbidden fallback; see cleak/u5-spec#72"
    )
}

/// `intro.md §6.2`: the menu's six rows, and the fixed row-to-letter
/// table through which Enter, Space and the idle timeout resolve.
///
/// "What exists is a fixed six-entry row-to-letter table: rows `0`
/// through `5` map to `J`, `C`, `T`, `U`, `A`, `R`."
pub const INTRO_MENU_ROW_COUNT: usize = 6;
pub const INTRO_MENU_ROW_LETTERS: [u8; INTRO_MENU_ROW_COUNT] = [b'J', b'C', b'T', b'U', b'A', b'R'];

/// `intro.md §6.2`: "The initial highlight is row 0, `Journey
/// Onward`, and the highlight index survives across poll passes."
pub const INTRO_MENU_INITIAL_HIGHLIGHT_ROW: u8 = 0;

/// `intro.md §6.2`: "Two hundred consecutive no-key passes | Commit
/// `Return to the View` exactly as though `R` had been pressed."
pub const INTRO_MENU_IDLE_TIMEOUT_PASSES: u16 = 200;

/// `intro.md §6`: the accepted intro-menu actions.
///
/// §6.2's input model is a highlight index plus letter hotkeys, not a
/// pure key dispatch: "An earlier revision of this section said
/// dispatch was purely by key and that 'the row number only controls
/// presentation'; that is withdrawn — the row index is load-bearing,
/// because Enter, Space and the idle timeout all resolve through it.
/// The claim that the menu keeps a 'recent-selection cache' that Enter
/// replays is withdrawn as well; there is no such cache."
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
    /// Up arrow or left arrow: "Move the highlight one row toward row
    /// 0, wrapping from row 0 to row 5; repaint the labels; keep
    /// polling."
    MoveHighlightUp,
    /// Down arrow or right arrow: "Move the highlight one row toward
    /// row 5, wrapping from row 5 to row 0; repaint the labels; keep
    /// polling."
    MoveHighlightDown,
    /// Enter or Space: "Commit whichever row is currently highlighted,
    /// resolved through the row-to-letter table."
    CommitHighlight,
}

impl IntroMenuAction {
    /// The row this action selects, for the six letter hotkeys.
    /// `None` for the highlight-motion and commit actions, which
    /// resolve against the menu's own highlight index instead.
    pub fn letter_row(self) -> Option<u8> {
        let letter = match self {
            IntroMenuAction::JourneyOnward => b'J',
            IntroMenuAction::CreateNewCharacter => b'C',
            IntroMenuAction::TransferFromUltimaIv => b'T',
            IntroMenuAction::UltimaVIntroduction => b'U',
            IntroMenuAction::Acknowledgements => b'A',
            IntroMenuAction::ReturnToView => b'R',
            IntroMenuAction::MoveHighlightUp
            | IntroMenuAction::MoveHighlightDown
            | IntroMenuAction::CommitHighlight => return None,
        };
        INTRO_MENU_ROW_LETTERS
            .iter()
            .position(|candidate| *candidate == letter)
            .map(|row| row as u8)
    }
}

/// `intro.md §6.2`: classify a raw key byte into an intro-menu action.
/// Keys are case-folded before dispatch (matching `input.md §6`).
/// Returns `None` for invalid keys — "Any other key | Discarded."
pub fn intro_menu_action(byte: u8) -> Option<IntroMenuAction> {
    let folded = input_case_fold(byte);
    Some(match folded {
        b'J' => IntroMenuAction::JourneyOnward,
        b'C' => IntroMenuAction::CreateNewCharacter,
        b'T' => IntroMenuAction::TransferFromUltimaIv,
        b'U' => IntroMenuAction::UltimaVIntroduction,
        b'A' => IntroMenuAction::Acknowledgements,
        b'R' => IntroMenuAction::ReturnToView,
        // "Enter, Space | Commit whichever row is currently
        // highlighted, resolved through the row-to-letter table."
        b'\r' | b'\n' | b' ' => IntroMenuAction::CommitHighlight,
        INPUT_CODE_NORTH | INPUT_CODE_WEST => IntroMenuAction::MoveHighlightUp,
        INPUT_CODE_SOUTH | INPUT_CODE_EAST => IntroMenuAction::MoveHighlightDown,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Locks in the clean-room first-playable defaults that stand in for
    /// not-yet-published spec values. Each row pairs the engine's
    /// current intentional clean-room value with the open spec issue
    /// that gates exact-parity ratification.
    ///
    /// If a spec issue is closed and the public spec publishes the
    /// authoritative value, update the corresponding constant *and*
    /// this test in the same patch.
    #[test]
    fn clean_room_policy_constants_match_documented_defaults() {
        // `cleak/u5-spec#49` — Create Food grant per cast is a
        // uniform `1..=3`; successful casts never grant zero food.
        assert_eq!(crate::CREATE_FOOD_MIN_GRANT, 1);
        assert_eq!(crate::CREATE_FOOD_MAX_GRANT, 3);
        // `cleak/u5-spec#50` — Hourly poison damage per Poisoned living
        // member is a deterministic `-1` (not RNG-rolled).
        assert_eq!(crate::FIRST_PLAYABLE_HOURLY_POISON_DAMAGE, 1);
        // `cleak/u5-spec#50` — Hourly starvation damage is now the
        // PRNG roll `prng_range(1, 8)` per non-dead party slot.
        assert_eq!(crate::HOURLY_STARVATION_DAMAGE_MIN, 1);
        assert_eq!(crate::HOURLY_STARVATION_DAMAGE_MAX, 8);
    }

    #[test]
    #[should_panic(expected = "forbidden fallback")]
    fn acknowledgements_refuses_placeholder_lines_without_a_pixel_surface() {
        require_graphical_acknowledgements_surface();
    }

    #[test]
    fn placeholder_title_tick_frames_satisfy_destination_contract() {
        let frames = placeholder_title_tick_frames();
        for frame in 0..TITLE_TICK_FRAME_COUNT {
            let pixels = frames.frame_pixels(frame);
            assert_eq!(pixels.len(), TITLE_TICK_FRAME_PIXELS);
            assert!(pixels.iter().all(|p| *p == 0));
        }
    }

    /// `cleak/u5-spec#78`: the shipped `ULTIMA` image directory holds
    /// five panels - the 319x61 menu logo followed by the four
    /// title-tick bands (288x49, 288x49, 288x49, 288x50). The tests
    /// read the local clean asset directory when it is present and
    /// skip when it is not, so a checkout without game files still
    /// passes.
    fn local_ultima_directory() -> Option<GraphicImageDirectory> {
        let game_dir = Path::new(crate::DEFAULT_GAME_DIR);
        if !game_dir
            .join(crate::tile_graphics_file_name(
                ULTIMA_PANEL_STEM,
                TileGraphicsDepth::Ega16,
            ))
            .exists()
        {
            return None;
        }
        Some(
            load_graphic_image_directory(game_dir, ULTIMA_PANEL_STEM, TileGraphicsDepth::Ega16)
                .expect("local ULTIMA image directory decodes"),
        )
    }

    #[test]
    fn local_ultima_directory_has_the_published_five_panel_shape() {
        let Some(directory) = local_ultima_directory() else {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        };
        let shapes: Vec<(usize, usize)> = directory
            .images
            .iter()
            .map(|image| {
                let image = image.as_ref().expect("ULTIMA panels are all populated");
                (image.width, image.height)
            })
            .collect();
        assert_eq!(
            shapes,
            vec![(319, 61), (288, 49), (288, 49), (288, 49), (288, 50)],
            "ULTIMA slot 0 is the menu logo and slots 1..=4 are the title-tick bands"
        );
    }

    #[test]
    fn ultima_title_tick_panels_fill_the_320_by_49_destination_band() {
        let Some(directory) = local_ultima_directory() else {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        };
        let frames =
            parse_ultima_title_tick_frames(&directory).expect("ULTIMA title-tick panels decode");
        // `cleak/u5-spec#65`: a staged frame is the whole published
        // 320-by-49 destination rectangle, with the 288-wide record
        // composited at x = 16 over the cleared staging background.
        let band_width = TITLE_TICK_FRAME_WIDTH as usize;
        let record_x = TITLE_TICK_SOURCE_X as usize;
        let record_width = TITLE_TICK_SOURCE_WIDTH as usize;
        assert_eq!(
            TITLE_TICK_FRAME_PIXELS,
            band_width * TITLE_TICK_FRAME_HEIGHT as usize
        );
        for frame in 0..TITLE_TICK_FRAME_COUNT {
            let pixels = frames.frame_pixels(frame);
            assert_eq!(
                pixels.len(),
                TITLE_TICK_FRAME_PIXELS,
                "frame {frame} is {band_width}x{}",
                TITLE_TICK_FRAME_HEIGHT
            );
            assert!(pixels.iter().all(|index| *index <= 0x0f));
            assert!(
                pixels.iter().any(|index| *index != 0),
                "frame {frame} must carry the flaming band, not an empty strip"
            );
            for row in 0..TITLE_TICK_FRAME_HEIGHT as usize {
                let base = row * band_width;
                assert!(
                    pixels[base..base + record_x]
                        .iter()
                        .all(|index| *index == TITLE_TICK_STAGING_BACKGROUND),
                    "frame {frame} row {row} left flank is staging background"
                );
                assert!(
                    pixels[base + record_x + record_width..base + band_width]
                        .iter()
                        .all(|index| *index == TITLE_TICK_STAGING_BACKGROUND),
                    "frame {frame} row {row} right flank is staging background"
                );
            }
        }
    }

    #[test]
    fn ultima_title_tick_panels_take_only_the_upper_rows_of_the_50_row_band() {
        let Some(directory) = local_ultima_directory() else {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        };
        let frames =
            parse_ultima_title_tick_frames(&directory).expect("ULTIMA title-tick panels decode");
        let last_slot = usize::from(ULTIMA_TITLE_TICK_FIRST_SLOT + TITLE_TICK_FRAME_COUNT - 1);
        let panel = directory.images[last_slot]
            .as_ref()
            .expect("last title-tick panel is populated");
        assert_eq!(panel.height, TITLE_TICK_SOURCE_MAX_HEIGHT);
        let record_width = TITLE_TICK_SOURCE_WIDTH as usize;
        let band_width = TITLE_TICK_FRAME_WIDTH as usize;
        let record_x = TITLE_TICK_SOURCE_X as usize;
        let height = TITLE_TICK_FRAME_HEIGHT as usize;
        let staged = frames.frame_pixels(TITLE_TICK_FRAME_COUNT - 1);
        for row in 0..height {
            let staged_row = &staged[row * band_width + record_x..][..record_width];
            assert_eq!(
                staged_row,
                &panel.pixels[row * record_width..][..record_width],
                "only the upper {height} rows of the 50-row source band reach the destination"
            );
        }
    }

    #[test]
    fn ultima_logo_panel_is_the_published_319_by_61_menu_art() {
        let game_dir = Path::new(crate::DEFAULT_GAME_DIR);
        if local_ultima_directory().is_none() {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        }
        let logo = load_ultima_logo_panel(game_dir, TileGraphicsDepth::Ega16)
            .expect("local ULTIMA logo panel decodes");
        assert_eq!(
            (logo.width, logo.height),
            (ULTIMA_LOGO_WIDTH, ULTIMA_LOGO_HEIGHT)
        );
        assert_eq!(logo.pixels.len(), ULTIMA_LOGO_WIDTH * ULTIMA_LOGO_HEIGHT);
    }

    #[test]
    fn ultima_title_tick_parser_rejects_a_directory_without_the_bands() {
        let directory = GraphicImageDirectory {
            depth: TileGraphicsDepth::Ega16,
            images: vec![None],
        };
        let err = parse_ultima_title_tick_frames(&directory)
            .expect_err("a directory without title-tick panels must fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn title_flourish_script_is_seven_frames_of_seven_reveals_and_six_erases() {
        // `cleak/u5-spec#67`: 7 x 7 reveal steps + 6 x 6 erase steps.
        assert_eq!(title_flourish_total_steps(), 85);
        assert_eq!(TITLE_FLOURISH_FRAME_COUNT, 7);
        assert_eq!(TITLE_FLOURISH_REVEAL_STEPS_PER_FRAME, 7);
        assert_eq!(TITLE_FLOURISH_ERASE_STEPS_PER_FRAME, 6);
        assert_eq!(title_flourish_step_state(85), None);

        // Frame 0: seven reveals, then six erases walking back down.
        for step in 0..7 {
            let state = title_flourish_step_state(step).unwrap();
            assert_eq!((state.frame, state.revealed_columns), (0, step + 1));
            assert!(state.revealing);
        }
        for (offset, expected) in (7..13).zip([6, 5, 4, 3, 2, 1]) {
            let state = title_flourish_step_state(offset).unwrap();
            assert_eq!((state.frame, state.revealed_columns), (0, expected));
            assert!(!state.revealing);
        }
        // Frame 1 starts on step 13; each of frames 0..=5 owns 13
        // steps and frame 6 owns only its seven reveals.
        assert_eq!(title_flourish_step_state(13).unwrap().frame, 1);
        let last = title_flourish_step_state(84).unwrap();
        assert_eq!(
            (last.frame, last.revealed_columns, last.revealing),
            (6, 7, true)
        );
    }

    #[test]
    fn title_flourish_reveal_sets_partition_each_frame_exactly_once() {
        // Every frame's seven reveal sets cover its band height, once
        // each — with the one shipped-data quirk in frame 5.
        for frame in 0..TITLE_FLOURISH_FRAME_COUNT {
            let (_, height) = title_flourish_band(frame);
            let mut named: Vec<u8> = TITLE_FLOURISH_REVEAL_SETS[frame]
                .iter()
                .flat_map(|set| set.iter().copied())
                .collect();
            named.sort_unstable();
            let visible = title_flourish_visible_rows(frame, 7);
            assert!(
                visible.iter().all(|row| usize::from(*row) < height),
                "frame {frame} names a row outside its {height}-row band"
            );
            if frame == 5 {
                // Row 19 named twice, row 29 never named.
                assert_eq!(named.len(), height);
                assert_eq!(visible.len(), height - 1);
                assert_eq!(named.iter().filter(|row| **row == 19).count(), 2);
                assert!(!visible.contains(&29));
            } else {
                assert_eq!(named.len(), height, "frame {frame} row count");
                assert_eq!(visible.len(), height, "frame {frame} distinct row count");
                for row in 0..height {
                    assert!(visible.contains(&(row as u8)), "frame {frame} row {row}");
                }
            }
        }
    }

    #[test]
    fn title_flourish_reveal_one_set_is_the_frame_top_and_bottom_rows() {
        // The reveal-1 set is never erased, so it is what stays on
        // screen through a frame's whole erase tail.
        for frame in 0..TITLE_FLOURISH_FRAME_COUNT {
            let (_, height) = title_flourish_band(frame);
            assert_eq!(
                title_flourish_visible_rows(frame, 1),
                vec![0u8, (height - 1) as u8],
                "frame {frame} reveal 1"
            );
        }
    }

    #[test]
    fn title_flourish_odd_frames_are_mirrored_and_shifted_one_row_down() {
        // `cleak/u5-spec#67`: even frames fill top-down from the band
        // top; odd frames fill bottom-up, so their band runs
        // `band_top + 1 ..= band_top + height`.
        for frame in 0..TITLE_FLOURISH_FRAME_COUNT {
            let (top, height) = title_flourish_band(frame);
            let first = title_flourish_content_row(frame, 0);
            let last = title_flourish_content_row(frame, height - 1);
            if title_flourish_fills_top_down(frame) {
                assert_eq!(title_flourish_band_shift(frame), 0, "frame {frame}");
                assert_eq!((first, last), (top, top + height - 1), "frame {frame}");
            } else {
                assert_eq!(title_flourish_band_shift(frame), 1, "frame {frame}");
                assert_eq!((first, last), (top + height - 1, top), "frame {frame}");
            }
        }
        // Frame 6 is even, so the finished mark is neither mirrored
        // nor shifted and sits exactly at (20, 46).
        assert!(title_flourish_fills_top_down(6));
        assert_eq!(title_flourish_band(6), (46, 61));
    }

    #[test]
    fn intro_menu_frame_border_profile_rounds_both_ends_and_fills_the_middle() {
        // `cleak/u5-spec#78` measured profile: the blue fill starts at
        // column 5 on the first band row, 3, 2, 1, 1, then 0 from the
        // sixth row on, and the bottom six rows mirror it.
        assert_eq!(intro_menu_frame_border_start_column(119), None);
        assert_eq!(intro_menu_frame_border_start_column(120), Some(5));
        assert_eq!(intro_menu_frame_border_start_column(121), Some(3));
        assert_eq!(intro_menu_frame_border_start_column(122), Some(2));
        assert_eq!(intro_menu_frame_border_start_column(123), Some(1));
        assert_eq!(intro_menu_frame_border_start_column(124), Some(1));
        for y in 125..=194 {
            assert_eq!(intro_menu_frame_border_start_column(y), Some(0), "row {y}");
        }
        assert_eq!(intro_menu_frame_border_start_column(195), Some(1));
        assert_eq!(intro_menu_frame_border_start_column(196), Some(1));
        assert_eq!(intro_menu_frame_border_start_column(197), Some(2));
        assert_eq!(intro_menu_frame_border_start_column(198), Some(3));
        assert_eq!(intro_menu_frame_border_start_column(199), Some(5));
        assert_eq!(intro_menu_frame_border_start_column(200), None);
    }

    #[test]
    fn intro_menu_frame_rectangle_matches_the_measured_geometry() {
        assert_eq!(INTRO_MENU_FRAME_TOP_Y, 120);
        assert_eq!(INTRO_MENU_FRAME_BOTTOM_Y, 199);
        assert_eq!(INTRO_MENU_FRAME_RULE_Y, 127);
        assert_eq!(INTRO_MENU_FRAME_BOTTOM_RULE_Y, 192);
        assert_eq!(INTRO_MENU_FRAME_RULE_X0, 7);
        assert_eq!(INTRO_MENU_FRAME_RULE_X1, 312);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_LEFT_X, 8);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_RIGHT_X, 311);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_TOP_Y, 128);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_BOTTOM_Y, 191);
        assert_eq!(INTRO_MENU_FRAME_BORDER_COLOR, 0x01);
        assert_eq!(INTRO_MENU_FRAME_OUTLINE_COLOR, 0x0f);
    }

    #[test]
    fn intro_menu_border_captions_occupy_the_measured_cells() {
        // `>Select:` + cursor + `<` fills cells 15..=24 of row 15;
        // the copyright caption fills cells 5..=33 of row 24.
        let select_cells =
            INTRO_MENU_SELECT_CAPTION_PREFIX.len() + 1 + INTRO_MENU_SELECT_CAPTION_SUFFIX.len();
        assert_eq!(select_cells, 10);
        assert_eq!(INTRO_MENU_SELECT_CAPTION_COLUMN, 15);
        assert_eq!(
            usize::from(INTRO_MENU_SELECT_CAPTION_COLUMN) + select_cells - 1,
            24
        );
        assert_eq!(INTRO_MENU_SELECT_CAPTION_ROW, 15);
        assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION.len(), 29);
        assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION_COLUMN, 5);
        assert_eq!(
            usize::from(INTRO_MENU_COPYRIGHT_CAPTION_COLUMN) + INTRO_MENU_COPYRIGHT_CAPTION.len()
                - 1,
            33
        );
        assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION_ROW, 24);
    }
}
