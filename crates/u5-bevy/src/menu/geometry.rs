//! Where the menu's panels sit (Phase B design 3.3.4; build plan M4-01,
//! D-M4.1).
//!
//! The menu image is the size of the picture the frame sprite shows: the
//! 320x200 frame on 4:3 and in the intro, the grow-left canvas otherwise
//! ([`menu_picture`]). It is placed exactly over the frame sprite, so one
//! menu pixel is one game pixel at every scale. Every panel is a whole
//! rectangle of 8 px text cells of that picture ([`CellRect`]): its ring
//! (the blue band and the white rule, one cell deep) and everything in it
//! sit on the picture's cell grid by construction.
//!
//! **The panels** ([`panel_rect`]):
//!
//! - On the 320x200 frame a narrow page (the top menu, Save) is cells 0-23
//!   on both axes: its ring lies on the frame's own ring round the 11x11
//!   map view, so its rules are the game's rules at x 7 and 184, y 7 and
//!   184, and its interior is the map view (cells 1-22). The roster and the
//!   messages stay in sight, dimmed. A wide page (Load, Settings) is cells
//!   0-39 by 0-23, the whole frame's ring (rules at x 7 and 312). The title
//!   layer is the title menu's own frame (`INTRO_MENU_FRAME_*`, cells 0-39
//!   by 15-24). A dialog is [`DIALOG_CELLS`] wide, centred.
//! - On a grow-left canvas a page keeps its width in cells and is centred
//!   over the visible map, as the sky and wind tabs are: the canvas centre
//!   with the column shut, the centre of the map right of the column with
//!   it open. When the page is wider than the visible map it is centred
//!   over the whole map window instead. Its top ring is the canvas's tab
//!   row (rule at y 15) and its bottom ring the canvas's bottom band (rule
//!   at `H - 8`), so a taller canvas shows more rows of a list.
//!
//! Panel interiors are black; everything outside a panel is dimmed by the
//! backdrop (`frame::dim_backdrop`).

use crate::VISUAL_PLAY_FRAME_HEIGHT;
use crate::VISUAL_PLAY_FRAME_WIDTH;
use crate::display::grow::column::{COLUMN_RULE_X, ColumnView};
use crate::display::grow::{BOTTOM_BAND, Canvas, SIDE_BORDER, TOP_BAND};

/// One text cell's side in picture pixels.
pub const CELL: u32 = 8;

/// A narrow page's width in cells: the frame's map view and its ring.
pub const NARROW_CELLS: u32 = 24;
/// A wide page's width in cells: the whole frame (`INTRO_MENU_FRAME_WIDTH_CELLS`).
pub const WIDE_CELLS: u32 = u5_runtime::INTRO_MENU_FRAME_WIDTH_CELLS as u32;
/// A dialog's width in cells: a 24-character slot name in quotes, its
/// margins and the ring.
pub const DIALOG_CELLS: u32 = 32;
/// The 320x200 frame's ring rows a narrow or wide page covers: cells 0-23
/// (row 24 is black in play).
pub const FRAME_PAGE_ROWS: u32 = 24;

/// A rectangle of whole text cells, `w` by `h` from `(col, row)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellRect {
    pub col: u32,
    pub row: u32,
    pub w: u32,
    pub h: u32,
}

impl CellRect {
    pub const fn new(col: u32, row: u32, w: u32, h: u32) -> Self {
        Self { col, row, w, h }
    }

    /// The last column, inclusive.
    pub const fn right(&self) -> u32 {
        self.col + self.w - 1
    }

    /// The last row, inclusive.
    pub const fn bottom(&self) -> u32 {
        self.row + self.h - 1
    }

    /// Whether cell `(col, row)` is inside.
    pub const fn contains(&self, col: u32, row: u32) -> bool {
        col >= self.col && col <= self.right() && row >= self.row && row <= self.bottom()
    }

    /// The pixel rectangle, `(x0, y0, x1, y1)` inclusive.
    pub const fn pixels(&self) -> (u32, u32, u32, u32) {
        (
            self.col * CELL,
            self.row * CELL,
            (self.col + self.w) * CELL - 1,
            (self.row + self.h) * CELL - 1,
        )
    }

    /// The rectangle less one cell on every side (a panel's interior).
    pub const fn inset(&self) -> Self {
        Self::new(self.col + 1, self.row + 1, self.w - 2, self.h - 2)
    }
}

/// The picture the menu image covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuPicture {
    /// Width in pixels: 320, or the canvas's.
    pub w: u32,
    /// Height in pixels: 200, or the canvas's.
    pub h: u32,
    /// The grow-left canvas, when the frame sprite shows one.
    pub canvas: Option<Canvas>,
}

impl MenuPicture {
    /// The 320x200 frame (4:3, the intro, a fixed screen).
    pub const FRAME: Self = Self {
        w: VISUAL_PLAY_FRAME_WIDTH,
        h: VISUAL_PLAY_FRAME_HEIGHT,
        canvas: None,
    };

    /// Columns of text cells.
    pub const fn cols(&self) -> u32 {
        self.w / CELL
    }

    /// Rows of text cells.
    pub const fn rows(&self) -> u32 {
        self.h / CELL
    }
}

/// The picture for the frame sprite's current image: the grow-left canvas
/// when one is shown, otherwise the 320x200 frame (D-M4.1).
pub fn menu_picture(canvas: Option<&Canvas>) -> MenuPicture {
    match canvas {
        Some(canvas) => MenuPicture {
            w: canvas.w,
            h: canvas.h,
            canvas: Some(*canvas),
        },
        None => MenuPicture::FRAME,
    }
}

/// The shape of a page's panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageShape {
    /// The top menu and Save: [`NARROW_CELLS`] wide.
    Narrow,
    /// Load and Settings: [`WIDE_CELLS`] wide.
    Wide,
    /// The title layer: the title menu's frame.
    Title,
    /// A dialog with `lines` interior rows.
    Dialog { lines: u32 },
}

impl PageShape {
    /// The panel's width in cells.
    pub const fn width(&self) -> u32 {
        match self {
            Self::Narrow => NARROW_CELLS,
            Self::Wide | Self::Title => WIDE_CELLS,
            Self::Dialog { .. } => DIALOG_CELLS,
        }
    }
}

/// The first and last cell columns of the map a panel may centre over:
/// right of the column's rule while the column is shown, else the whole
/// map window.
fn map_cols(canvas: &Canvas, column: ColumnView) -> (u32, u32) {
    let last = (canvas.w - SIDE_BORDER) / CELL - 1;
    if column.shown() {
        ((COLUMN_RULE_X + 1) / CELL, last)
    } else {
        (SIDE_BORDER / CELL, last)
    }
}

/// The column that centres `w` cells between `first` and `last`, or `None`
/// when they do not fit.
fn centred(first: u32, last: u32, w: u32) -> Option<u32> {
    let room = (last + 1).checked_sub(first)?;
    let spare = room.checked_sub(w)?;
    Some(first + spare / 2)
}

/// The panel of a `shape` page on `picture`, with the docked column in
/// `column`'s state (ignored on the 320x200 frame). Always inside the
/// picture and on its cell grid (D-M4.1).
pub fn panel_rect(shape: PageShape, picture: &MenuPicture, column: ColumnView) -> CellRect {
    let w = shape.width().min(picture.cols());
    let Some(canvas) = picture.canvas else {
        return match shape {
            PageShape::Narrow => CellRect::new(0, 0, w, FRAME_PAGE_ROWS),
            PageShape::Wide => CellRect::new(0, 0, w, FRAME_PAGE_ROWS),
            PageShape::Title => CellRect::new(
                u32::from(u5_runtime::INTRO_MENU_FRAME_ANCHOR_COLUMN),
                u32::from(u5_runtime::INTRO_MENU_FRAME_ANCHOR_ROW),
                w,
                u32::from(u5_runtime::INTRO_MENU_FRAME_HEIGHT_CELLS),
            ),
            PageShape::Dialog { lines } => {
                let h = (lines + 2).min(FRAME_PAGE_ROWS);
                CellRect::new((picture.cols() - w) / 2, (FRAME_PAGE_ROWS - h) / 2, w, h)
            }
        };
    };
    let (first, last) = map_cols(&canvas, column);
    let (all_first, all_last) = map_cols(&canvas, ColumnView::default());
    let col = centred(first, last, w)
        .or_else(|| centred(all_first, all_last, w))
        .unwrap_or_else(|| (picture.cols() - w) / 2);
    // The canvas's tab row (rule at TOP_BAND - 1) to its bottom band (rule
    // at H - BOTTOM_BAND).
    let top = TOP_BAND / CELL - 1;
    let bottom = (canvas.h - BOTTOM_BAND) / CELL;
    match shape {
        PageShape::Narrow | PageShape::Wide => CellRect::new(col, top, w, bottom + 1 - top),
        PageShape::Title => {
            let h = u32::from(u5_runtime::INTRO_MENU_FRAME_HEIGHT_CELLS);
            CellRect::new(col, bottom + 1 - h, w, h)
        }
        PageShape::Dialog { lines } => {
            // Centred over the map window's rows (top + 1 ..= bottom - 1).
            let h = (lines + 2).min(bottom + 1 - top);
            let first_row = top + 1;
            let rows = bottom - 1 + 1 - first_row;
            let row = if h <= rows {
                first_row + (rows - h) / 2
            } else {
                top
            };
            CellRect::new(col, row, w, h)
        }
    }
}
