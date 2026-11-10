//! The grow-left compositor, step 8 (Phase B design 2.2.2, 2.2.5, 2.2.7,
//! 2.3.7; build plan M1-13): the docked left column, opened and closed by
//! Ctrl+P (the router's `ToggleColumn`).
//!
//! The column is the original's right column moved to the left and laid
//! along the screen's own left border:
//!
//! - **Chrome.** Band blue over x 7..150 from the top rule to the bottom
//!   rule, and a white rule at x 151 from y 15 to the bottom rule. Three
//!   black windows, each with the original's 1 px white rule one pixel
//!   outside it: the roster (x 8..143, y 16..63), food/gold/date
//!   (y 72..87) and the message window (y 96 to the bottom of the map
//!   window, which keeps the screen's bottom rule as its own). The rule at
//!   x 7 is the screen's left rule, so between the windows (y 65..70 and
//!   89..94) the band's blue runs into the outer border, as the original's
//!   bands meet its outer frame.
//! - **Junctions.** Between the column and the map is the original's
//!   6-pixel blue band between two rules (x 144 and x 151). Where it meets
//!   the top and bottom bands the blue runs through the rules (x 145..150
//!   on y 15 and on the bottom rule), as the original's middle band meets
//!   its top and bottom bands.
//! - **Content.** The roster and food/gold/date windows show pixel crops of
//!   the faithful frame's own windows (frame x 192..311, y 8..55 and
//!   y 64..79), placed 8 pixels inside each window as the original places
//!   them, so they are always exactly what the original shows. The message
//!   window shows the crawl's rows on the very pixels the crawl drew them,
//!   each in its class's fresh colour, with older rows above them up to the
//!   window's height (17 rows on 240-row canvases, 14 on 216); the live row
//!   keeps its triangle.
//! - **The map does not move.** The column covers the map's left part; the
//!   faithful core stays visible on every canvas (the canvas rule pans a
//!   narrow canvas for it). The sky and wind tabs re-centre over the
//!   visible map (step 6, the bands' `open`).
//!
//! **State.** [`ColumnView`] is the session's state, in memory only (D2.9):
//! Ctrl+P flips it and never writes the file. With `display.crawl` off the
//! column starts open, since the open window is then where messages are
//! read. The open and close effect (`display.dither`, M1-15) builds on
//! this; until it lands, open and close take one frame.
//!
//! **Page mode** (M1-14, 2.2.5, D2.20). While the original uses its right
//! column for more than the roster ([`page_mode_active`]: a Z-stats page, a
//! U-Use or R-Ready picker, the inn's Guest Register or any shop session),
//! the column is shown whatever the player's state. A page does not keep
//! to the roster and food/gold/date windows: Z-stats, the pickers and the
//! inn's register make one window of frame y 8..79, and the arms-sell
//! browser puts its page badge on the band between them. So in page mode
//! the panel rows (frame x 192..311, y 8..80, [`PAGE_PANEL`]) are copied
//! whole, each on the canvas row 8 below it as the column's windows already
//! sit, and each row's 8-pixel margins and rules follow the frame's own row
//! ([`page_row_kind`]): a window row gets black margins inside the rules at
//! x 7 and 144, a rule row white, a band row blue. The page's title on the
//! frame's top border (frame y 0..7) is not shown: the ribbon holds that
//! band. The message window shows a crop of the original's message window
//! (frame x 192..319, y 88..191) in place of the crawl's rows and the
//! history, bottom-aligned so the original's live row sits on the crawl's
//! bottom row. When the page goes the column shows the player's own state
//! again: page mode never changes [`ColumnView::open`], and Ctrl+P during a
//! page flips only that.
//!
//! Every pixel placed comes from the faithful frame, the runtime-loaded
//! font or the flat chrome geometry.

use std::sync::{Arc, Mutex, MutexGuard};

use bevy::prelude::*;
use u5_runtime::PlayState;

use super::Canvas;
use super::compose::{BAND_BLUE, RULE_WHITE};
use super::crawl_paint::{Painted, paint_lines};
use crate::display::crawl::layer::{CrawlLine, CrawlRows, GAME_COLOURS, TYPED_COLOURS};
use crate::enh::input::{EnhAction, EnhActionEvent};
use crate::{EGA_PALETTE_RGB, VISUAL_PLAY_FRAME_WIDTH};

/// The column's blue starts on the screen's left rule.
pub const COLUMN_X0: u32 = 7;
/// The column's last blue column; its right rule is one right of it.
pub const COLUMN_BLUE_X1: u32 = 150;
/// The rule between the column's band and the map.
pub const COLUMN_RULE_X: u32 = 151;
/// The column's windows: interior x 8..143, their right rule at x 144.
pub const WINDOW_X0: u32 = 8;
pub const WINDOW_X1: u32 = 143;
/// The roster window's rows.
pub const ROSTER_Y: (u32, u32) = (16, 63);
/// The food/gold/date window's rows.
pub const COUNTERS_Y: (u32, u32) = (72, 87);
/// The message window's first row; it ends at the map window's bottom.
pub const MESSAGE_Y0: u32 = 96;
/// Where the crops land: 8 pixels inside each window.
pub const CROP_X: u32 = 16;
/// The faithful roster window: frame x 192..311, y 8..55.
pub const ROSTER_CROP: FrameRect = FrameRect {
    x: 192,
    y: 8,
    w: 120,
    h: 48,
};
/// The faithful food/gold/date window: frame x 192..311, y 64..79.
pub const COUNTERS_CROP: FrameRect = FrameRect {
    x: 192,
    y: 64,
    w: 120,
    h: 16,
};

/// The faithful message window: frame x 192..319, y 88..191 (thirteen
/// 16-cell rows), shown in page mode.
pub const MESSAGE_CROP: FrameRect = FrameRect {
    x: 192,
    y: 88,
    w: 128,
    h: 104,
};

/// The faithful right panel a page uses: frame x 192..311, y 8..80 (the
/// roster, the band, food/gold/date and the bottom rule), shown whole in
/// page mode on canvas y 16..88.
pub const PAGE_PANEL: FrameRect = FrameRect {
    x: 192,
    y: 8,
    w: 120,
    h: 73,
};

/// The column's windows sit this many rows below the frame's.
pub const PANEL_DROP: u32 = 8;

/// A rectangle of the 320x200 faithful frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// The column's state for one frame (2.3.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColumnView {
    /// Open (Ctrl+P), for the session.
    pub open: bool,
    /// Dither step 0..=8 while an open or close effect runs (M1-15);
    /// `None` here, where open and close take one frame.
    pub step: Option<u8>,
    /// The original's right column is showing a page
    /// ([`page_mode_active`]): the column is shown whatever `open` says.
    /// Taken from the state on every compose, never stored.
    pub page_mode: bool,
}

impl ColumnView {
    /// The view a session starts with: closed, or open when the crawl is
    /// off, so the messages can be read (2.2.4).
    pub fn at_launch(crawl: bool) -> Self {
        Self {
            open: !crawl,
            ..Self::default()
        }
    }

    /// Whether the column is drawn: the player opened it, or a page is up.
    pub fn shown(&self) -> bool {
        self.open || self.page_mode
    }
}

/// Whether the original is using its right column for a page (2.2.5,
/// D2.20): a Z-stats page, a U-Use or R-Ready picker, the inn's Guest
/// Register, or any shop session. Read-only.
pub fn page_mode_active(state: &PlayState) -> bool {
    u5_runtime::stats_panel::z_stats_page_panel_rows(state).is_some()
        || u5_runtime::stats_panel::active_panel_picker(state).is_some()
        || u5_runtime::stats_panel::inn_pickup_register_active(state)
        || state.active_shop.is_some()
}

/// The session's [`ColumnView`], shared between the systems that change it
/// and the `[grow-left compose]` step that draws it. No threads: the lock is
/// never contended. Exists only while `display.grow_left` is on.
#[derive(Resource, Clone, Debug, Default)]
pub struct ColumnState(Arc<Mutex<ColumnView>>);

impl ColumnState {
    pub fn new(view: ColumnView) -> Self {
        Self(Arc::new(Mutex::new(view)))
    }

    fn lock(&self) -> MutexGuard<'_, ColumnView> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The view as it stands.
    pub fn view(&self) -> ColumnView {
        *self.lock()
    }

    /// Open or close the column.
    pub fn set_open(&self, open: bool) {
        self.lock().open = open;
    }

    /// Ctrl+P: flip the player's column state (during a page too, where it
    /// shows once the page goes).
    pub fn toggle(&self) {
        let mut view = self.lock();
        view.open = !view.open;
    }
}

/// `ToggleColumn` (Ctrl+P) opens or closes the column for the session. It
/// writes no setting and sends the game nothing: the router consumed the
/// key before the input driver saw it.
pub(crate) fn column_actions(mut actions: EventReader<EnhActionEvent>, column: Res<ColumnState>) {
    for action in actions.read() {
        if action.0 == EnhAction::ToggleColumn {
            column.toggle();
        }
    }
}

/// The message window's rows, `(y0, y1)` inclusive: y 96 to the bottom of
/// the map window.
pub fn message_window_y(canvas: &Canvas) -> (u32, u32) {
    (MESSAGE_Y0, canvas.map_window().3)
}

/// How many rows the open message window shows: 17 on 240-row canvases,
/// 14 on the 216-row one (2.2.5).
pub fn open_window_rows(canvas: &Canvas) -> usize {
    let (y0, y1) = message_window_y(canvas);
    ((y1 + 1 - y0) / u5_runtime::CH_CELL_SIDE as u32) as usize
}

/// The lines the open window shows, top to bottom: the older rows, then
/// every line the crawl placed (aged out or not), the newest `max` of them.
pub fn open_lines(rows: &CrawlRows, max: usize) -> Vec<&CrawlLine> {
    let all: Vec<&CrawlLine> = rows.older.iter().chain(rows.lines.iter()).collect();
    let skip = all.len().saturating_sub(max);
    all[skip..].to_vec()
}

/// A cell's colour in the open window: its class's fresh colour (game
/// white, typed bright cyan with `display.typed_colour` on), whatever its
/// age.
pub fn fresh_colour(typed: bool, typed_colour: bool) -> u8 {
    if typed && typed_colour {
        TYPED_COLOURS[0]
    } else {
        GAME_COLOURS[0]
    }
}

fn rgba_of(index: u8) -> [u8; 4] {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index)];
    [r, g, b, 0xff]
}

fn fill(rgba: &mut [u8], canvas: &Canvas, (x0, y0, x1, y1): (u32, u32, u32, u32), index: u8) {
    let colour = rgba_of(index);
    let w = canvas.w as usize;
    for y in y0 as usize..=y1 as usize {
        for x in x0 as usize..=x1 as usize {
            let at = (y * w + x) * 4;
            rgba[at..at + 4].copy_from_slice(&colour);
        }
    }
}

/// A black window with interior x 8..143 and rows `y0..=y1`, ruled in
/// white one pixel outside it on the top, left and right, and on the
/// bottom when `bottom` (the message window uses the screen's bottom rule).
fn window(rgba: &mut [u8], canvas: &Canvas, (y0, y1): (u32, u32), bottom: bool) {
    let last = if bottom { y1 + 1 } else { y1 };
    fill(
        rgba,
        canvas,
        (WINDOW_X0 - 1, y0 - 1, WINDOW_X1 + 1, last),
        RULE_WHITE,
    );
    fill(rgba, canvas, (WINDOW_X0, y0, WINDOW_X1, y1), 0);
}

/// The column's chrome (see the module text): the band, its rule at
/// x 151, the three windows and the junctions with the top and bottom
/// bands.
pub fn paint_column_chrome(rgba: &mut [u8], canvas: &Canvas) {
    let (_, top, _, map_bottom) = canvas.map_window();
    let (rule_top, rule_bottom) = (top - 1, map_bottom + 1);
    fill(
        rgba,
        canvas,
        (COLUMN_X0, top, COLUMN_BLUE_X1, map_bottom),
        BAND_BLUE,
    );
    fill(
        rgba,
        canvas,
        (COLUMN_RULE_X, rule_top, COLUMN_RULE_X, rule_bottom),
        RULE_WHITE,
    );
    window(rgba, canvas, ROSTER_Y, true);
    window(rgba, canvas, COUNTERS_Y, true);
    window(rgba, canvas, message_window_y(canvas), false);
    // The junctions: the band's blue runs through the top and bottom rules.
    for y in [rule_top, rule_bottom] {
        fill(
            rgba,
            canvas,
            (WINDOW_X1 + 2, y, COLUMN_BLUE_X1, y),
            BAND_BLUE,
        );
    }
}

/// Copy the faithful frame's `rect` to the canvas with its top-left at
/// `(x0, y0)`.
fn copy_crop(
    rgba: &mut [u8],
    canvas: &Canvas,
    faithful: &[u8],
    rect: FrameRect,
    (x0, y0): (u32, u32),
) {
    let frame_w = VISUAL_PLAY_FRAME_WIDTH as usize;
    let w = canvas.w as usize;
    let width = rect.w as usize * 4;
    for row in 0..rect.h as usize {
        let from = ((rect.y as usize + row) * frame_w + rect.x as usize) * 4;
        let at = ((y0 as usize + row) * w + x0 as usize) * 4;
        rgba[at..at + width].copy_from_slice(&faithful[from..from + width]);
    }
}

/// The roster and food/gold/date crops, byte for byte from the faithful
/// frame.
pub fn paint_column_crops(rgba: &mut [u8], canvas: &Canvas, faithful: &[u8]) {
    copy_crop(rgba, canvas, faithful, ROSTER_CROP, (CROP_X, ROSTER_Y.0));
    copy_crop(
        rgba,
        canvas,
        faithful,
        COUNTERS_CROP,
        (CROP_X, COUNTERS_Y.0),
    );
}

/// What step 8 draws the message window from.
#[derive(Clone, Copy)]
pub struct ColumnText<'a> {
    pub rows: &'a CrawlRows,
    /// `display.typed_colour`.
    pub typed_colour: bool,
}

/// The open message window's text: [`open_lines`] on the crawl's pixels
/// (the bottom line on the crawl's bottom row, text from x 16), each cell
/// in its class's fresh colour, outlines clipped to the window.
pub fn paint_open_window(rgba: &mut [u8], canvas: &Canvas, text: ColumnText<'_>) -> Painted {
    let lines = open_lines(text.rows, open_window_rows(canvas));
    let (y0, y1) = message_window_y(canvas);
    let typed_colour = text.typed_colour;
    paint_lines(
        rgba,
        canvas,
        &lines,
        text.rows,
        (WINDOW_X0, y0, WINDOW_X1, y1),
        |_, typed| Some(fresh_colour(typed, typed_colour)),
    )
}

/// Where page mode's message crop lands: 8 pixels inside the window, its
/// bottom row on the window's bottom row (the crawl's bottom row).
pub fn message_crop_origin(canvas: &Canvas) -> (u32, u32) {
    let (_, y1) = message_window_y(canvas);
    (CROP_X, y1 + 1 - MESSAGE_CROP.h)
}

/// What one frame row of the right panel is, read off the frame's own
/// pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageRow {
    /// Inside a window: the rule at frame x 191, black or text after it.
    Window,
    /// A window's top or bottom rule (a badge may break it).
    Rule,
    /// The band between windows: blue at frame x 191.
    Band,
}

/// Frame row `y` of the right panel: a band row when frame x 191 is not
/// the rule's white; a rule row when the row is white at both ends of the
/// panel (x 192 and 311; text never reaches both, a glyph's ink starts a
/// pixel in) and white over at least half of it; a window row otherwise.
pub fn page_row_kind(faithful: &[u8], y: u32) -> PageRow {
    let index = |x: u32| {
        let at = ((y * VISUAL_PLAY_FRAME_WIDTH + x) * 4) as usize;
        crate::ega_palette_index_from_rgba(&faithful[at..at + 4])
    };
    if index(PAGE_PANEL.x - 1) != RULE_WHITE {
        return PageRow::Band;
    }
    let (x0, x1) = (PAGE_PANEL.x, PAGE_PANEL.x + PAGE_PANEL.w - 1);
    let white = (x0..=x1).filter(|x| index(*x) == RULE_WHITE).count() as u32;
    if index(x0) == RULE_WHITE && index(x1) == RULE_WHITE && 2 * white >= PAGE_PANEL.w {
        PageRow::Rule
    } else {
        PageRow::Window
    }
}

/// Page mode's panel: [`PAGE_PANEL`] copied row by row to canvas
/// `(CROP_X, y + 8)`, with each row's margins (x 7..15 and 136..144)
/// following [`page_row_kind`].
pub fn paint_page_panel(rgba: &mut [u8], canvas: &Canvas, faithful: &[u8]) {
    let left = (COLUMN_X0, CROP_X - 1);
    let right = (CROP_X + PAGE_PANEL.w, WINDOW_X1 + 1);
    for fy in PAGE_PANEL.y..PAGE_PANEL.y + PAGE_PANEL.h {
        let y = fy + PANEL_DROP;
        match page_row_kind(faithful, fy) {
            PageRow::Window => {
                fill(rgba, canvas, (left.0, y, left.0, y), RULE_WHITE);
                fill(rgba, canvas, (left.0 + 1, y, left.1, y), 0);
                fill(rgba, canvas, (right.0, y, right.1 - 1, y), 0);
                fill(rgba, canvas, (right.1, y, right.1, y), RULE_WHITE);
            }
            PageRow::Rule => {
                fill(rgba, canvas, (left.0, y, left.1, y), RULE_WHITE);
                fill(rgba, canvas, (right.0, y, right.1, y), RULE_WHITE);
            }
            PageRow::Band => {
                fill(rgba, canvas, (left.0, y, left.1, y), BAND_BLUE);
                fill(rgba, canvas, (right.0, y, right.1, y), BAND_BLUE);
            }
        }
        let rect = FrameRect {
            y: fy,
            h: 1,
            ..PAGE_PANEL
        };
        copy_crop(rgba, canvas, faithful, rect, (CROP_X, y));
    }
}

/// Page mode's message window: the faithful message window, byte for byte.
pub fn paint_page_window(rgba: &mut [u8], canvas: &Canvas, faithful: &[u8]) {
    copy_crop(
        rgba,
        canvas,
        faithful,
        MESSAGE_CROP,
        message_crop_origin(canvas),
    );
}

/// Step 8: the column, when shown: its chrome, the faithful crops (in page
/// mode the whole panel, [`paint_page_panel`]) and the message window,
/// which in page mode is the faithful message window and otherwise, with
/// `text`, the crawl's rows and the history.
pub fn paint_column(
    rgba: &mut [u8],
    canvas: &Canvas,
    faithful: &[u8],
    text: Option<ColumnText<'_>>,
    page_mode: bool,
) {
    paint_column_chrome(rgba, canvas);
    if page_mode {
        paint_page_panel(rgba, canvas, faithful);
        paint_page_window(rgba, canvas, faithful);
        return;
    }
    paint_column_crops(rgba, canvas, faithful);
    if let Some(text) = text {
        paint_open_window(rgba, canvas, text);
    }
}

#[cfg(test)]
#[path = "column_tests.rs"]
mod tests;
