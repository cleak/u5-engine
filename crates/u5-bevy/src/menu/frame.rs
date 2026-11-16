//! The menu's frame kit (Phase B design 3.3.4 "Rendering"; build plan
//! M4-01, D3.6, D-M4.1, D-M4.10).
//!
//! The menu is a CPU image ([`MenuImage`]) the size of the picture the frame
//! sprite shows, drawn only with the game's own pieces:
//!
//! - **Panels** ([`draw_panel`]) are the faithful chrome's model: a blue
//!   band (EGA 1) filling the panel, black boxes cut into it on whole text
//!   cells, and a 1 px white rule (EGA 15) on every band pixel that touches a
//!   box, diagonals included. The ring is one cell deep, so the rule is the
//!   inner pixel of the ring cell: x 7 and y 7 of a top-left ring, x 0 and
//!   y 0 of a bottom-right one, as the gameplay frame and the title menu's
//!   frame have it. The outer corners are carved with
//!   [`CHROME_CORNER_PROFILE`], the staircase both of the original's blue
//!   frames use.
//! - **Dividers** ([`Panel::divider`], [`Panel::split`]) are one-cell bands
//!   across the panel, as the stats panel's divider rows 7 and 10 are,
//!   except that each box's black reaches a pixel into the band, so the
//!   rules stand a pixel off the text: black, a rule, four rows of blue, a
//!   rule, black. Where one meets the ring the rules turn
//!   the corner and the blues merge, which is the faithful junction pixel
//!   for pixel; nothing is doubled and nothing is missing.
//! - **Tabs** ([`draw_label_tab`]) punch a black gap through a ring or
//!   divider row and close it with the ribbon end caps
//!   ([`ribbon_cap_sprite`]), as `>Select:<` and `>Calm Winds<` sit on their
//!   rules; the same piece makes the footer key tabs, the Settings tab strip
//!   ([`draw_tab_strip`]) and the scroll marks ([`draw_scroll_mark`]).
//! - **Rows** ([`Line`], [`draw_line`]) are IBM.CH cells only: labels, the
//!   `[x]` check box, the `[####----]` slider, values and the apply tags.
//!   The selected row is drawn in inverse video across the whole interior
//!   of its box ([`draw_selection_bar`]), as the shop pickers invert their
//!   row across the stats panel's window. A disabled row is grey (EGA 8);
//!   the one accent is brown (EGA 6, `#AA5500`), for the "Spoiler:" and
//!   "Cheats" words.
//! - **The thumbnail** ([`draw_thumbnail_box`]) is the 80x50 picture in a
//!   1 px white rule, on a 10x7-cell box.
//! - **The backdrop** ([`dim_backdrop`]): every pixel no panel drew is black
//!   at alpha 0.6, which dims the live game behind the menu.
//! - **The sweep** ([`sweep_image`], D-M4.10): the M1 dither law over the
//!   panel's rectangle only, rising on open and falling on close, in 8
//!   steps of [`MENU_SWEEP_STEP`].
//!
//! Every opaque pixel is one of the shell's 16 EGA colours, except the
//! thumbnail's picture, which is the box-filtered frame.

use std::time::Duration;

use u5_runtime::{
    CH_CELL_SIDE, CHROME_CORNER_PROFILE, FixedCellFont, RibbonCapDirection, ribbon_cap_sprite,
};

use super::geometry::{CELL, CellRect, MenuPicture};
use crate::EGA_PALETTE_RGB;
use crate::display::effects::{step_of, sweep_t};

/// The band's blue (the chrome colour).
pub const BAND: u8 = u5_runtime::CHROME_RIBBON_INDEX;
/// The rules' white (the accent colour), also the text's.
pub const RULE: u8 = u5_runtime::CHROME_RULE_INDEX;
/// Box interiors.
pub const PAPER: u8 = 0;
/// Row text.
pub const INK: u8 = 15;
/// Help lines, values and apply tags: the text window's light grey.
pub const HELP_INK: u8 = 7;
/// Disabled rows and unselected tabs.
pub const GREY: u8 = 8;
/// The one accent: brown, `#AA5500`.
pub const ACCENT: u8 = 6;

/// The backdrop's opacity: black at 0.6 (3.4.4).
pub const DIM_ALPHA: u8 = 153;
/// A backdrop pixel.
pub const DIM: [u8; 4] = [0, 0, 0, DIM_ALPHA];
/// A pixel nothing has drawn yet.
pub const CLEAR: [u8; 4] = [0; 4];

/// One sweep step (D-M4.10): 8 steps of 25 ms, 200 ms in all.
pub const MENU_SWEEP_STEP: Duration = Duration::from_millis(25);

/// The thumbnail's picture: the 320x200 frame box-filtered by 4.
pub const THUMB_W: u32 = 80;
/// See [`THUMB_W`].
pub const THUMB_H: u32 = 50;
/// The thumbnail box in cells: 80x56, the picture centred with 3 rows of
/// black above and below.
pub const THUMB_CELLS: (u32, u32) = (10, 7);
/// The rows of black above the picture in its box.
pub const THUMB_PAD: u32 = (THUMB_CELLS.1 * CELL - THUMB_H) / 2;

/// The shell's opaque pixel for EGA colour `index`.
pub fn ega(index: u8) -> [u8; 4] {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index & 0x0f)];
    [r, g, b, 0xff]
}

/// The EGA colour of an opaque pixel, if it is one of the 16.
pub fn ega_index(pixel: [u8; 4]) -> Option<u8> {
    if pixel[3] != 0xff {
        return None;
    }
    (0..16u8).find(|&index| ega(index) == pixel)
}

/// The menu's picture: RGBA, transparent until drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl MenuImage {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            rgba: vec![0; (w * h * 4) as usize],
        }
    }

    /// An empty image the size of `picture`.
    pub fn for_picture(picture: &MenuPicture) -> Self {
        Self::new(picture.w, picture.h)
    }

    /// The pixel at `(x, y)`.
    pub fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// Set the pixel at `(x, y)`; outside the image it is dropped.
    pub fn set(&mut self, x: u32, y: u32, pixel: [u8; 4]) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&pixel);
    }

    /// Fill a pixel rectangle, both ends inclusive.
    pub fn fill(&mut self, (x0, y0, x1, y1): (u32, u32, u32, u32), pixel: [u8; 4]) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, pixel);
            }
        }
    }

    /// Fill whole cells.
    pub fn fill_cells(&mut self, cells: CellRect, index: u8) {
        self.fill(cells.pixels(), ega(index));
    }
}

/// A vertical divider band at `col`, joining the horizontal bands (the ring
/// or a divider) at rows `top` and `bottom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Split {
    pub col: u32,
    pub top: u32,
    pub bottom: u32,
}

/// A panel: its rectangle, the divider rows across it and the vertical
/// splits inside it. The black boxes are what is left of the interior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub rect: CellRect,
    pub dividers: Vec<u32>,
    pub splits: Vec<Split>,
    /// Whether a box's black reaches a pixel into a divider's row above or
    /// below it (the default), so the divider's rules stand off the text.
    /// [`Panel::tight`] turns it off.
    pub breathe: bool,
}

impl Panel {
    pub fn new(rect: CellRect) -> Self {
        assert!(rect.w >= 3 && rect.h >= 3, "a panel needs an interior");
        Self {
            rect,
            dividers: Vec::new(),
            splits: Vec::new(),
            breathe: true,
        }
    }

    /// Every rule on its band cell's edge pixel, dividers too, as the
    /// gameplay frame's stats panel has them; text beside a band then keeps
    /// blank margin rows instead.
    pub fn tight(mut self) -> Self {
        self.breathe = false;
        self
    }

    /// A divider band across the panel at cell row `row`.
    pub fn divider(mut self, row: u32) -> Self {
        assert!(
            row > self.rect.row + 1 && row + 1 < self.rect.bottom(),
            "a divider leaves a box on each side"
        );
        self.dividers.push(row);
        self.dividers.sort_unstable();
        self
    }

    /// A vertical band at `col` from the band at row `top` to the band at
    /// row `bottom`.
    pub fn split(mut self, col: u32, top: u32, bottom: u32) -> Self {
        let is_band = |row: u32| {
            row == self.rect.row || row == self.rect.bottom() || self.dividers.contains(&row)
        };
        assert!(is_band(top) && is_band(bottom), "a split joins two bands");
        assert!(
            col > self.rect.col + 1 && col + 1 < self.rect.right(),
            "a split leaves a box on each side"
        );
        self.splits.push(Split { col, top, bottom });
        self
    }

    /// The interior: the rectangle less the ring.
    pub fn interior(&self) -> CellRect {
        self.rect.inset()
    }

    /// The black boxes, top to bottom, left to right.
    pub fn boxes(&self) -> Vec<CellRect> {
        let inner = self.interior();
        let mut edges = vec![self.rect.row];
        edges.extend(self.dividers.iter().copied());
        edges.push(self.rect.bottom());
        let mut boxes = Vec::new();
        for pair in edges.windows(2) {
            let (top, bottom) = (pair[0] + 1, pair[1] - 1);
            let mut cols: Vec<u32> = self
                .splits
                .iter()
                .filter(|split| split.top < top && split.bottom > bottom)
                .map(|split| split.col)
                .collect();
            cols.sort_unstable();
            let mut left = inner.col;
            for col in cols.into_iter().chain([inner.right() + 1]) {
                boxes.push(CellRect::new(left, top, col - left, bottom + 1 - top));
                left = col + 1;
            }
        }
        boxes
    }

    /// Whether cell `(col, row)` is inside a box.
    pub fn is_box_cell(&self, col: u32, row: u32) -> bool {
        self.boxes().iter().any(|b| b.contains(col, row))
    }
}

/// Draw `panel`: the band, the boxes, the rules round them and the rounded
/// outer corners. Everything it draws is opaque except the carved corners,
/// which stay as they were (the backdrop fills them).
pub fn draw_panel(img: &mut MenuImage, panel: &Panel) {
    let rect = panel.rect;
    let (x0, y0, x1, y1) = rect.pixels();
    let boxes = panel.boxes();
    let cols = rect.w as usize;
    let mut cell_is_box = vec![false; (rect.w * rect.h) as usize];
    for b in &boxes {
        for row in b.row..=b.bottom() {
            for col in b.col..=b.right() {
                cell_is_box[((row - rect.row) * rect.w + (col - rect.col)) as usize] = true;
            }
        }
    }
    let box_cell = |col: u32, row: u32| -> bool {
        row < rect.h && cell_is_box[row as usize * cols + col as usize]
    };
    // Beside a divider, a box's black reaches one pixel into the divider's
    // row, so the divider's rules stand a pixel off the text: a capital in
    // the footer's first help line and a descender in the row above never
    // touch them, as on the cheats page. The ring keeps its rules on the
    // gameplay frame's own (the pages lie on the frame's rings), and
    // across, the text already keeps a margin cell. No row moves.
    let divider = |row: u32| panel.dividers.contains(&(rect.row + row));
    let is_box = |x: i64, y: i64| -> bool {
        if x < i64::from(x0) || y < i64::from(y0) || x > i64::from(x1) || y > i64::from(y1) {
            return false;
        }
        let col = (x as u32 - x0) / CELL;
        let row = (y as u32 - y0) / CELL;
        let line = (y as u32 - y0) % CELL;
        box_cell(col, row)
            || (panel.breathe
                && divider(row)
                && ((line == CELL - 1 && box_cell(col, row + 1))
                    || (line == 0 && box_cell(col, row - 1))))
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (xi, yi) = (i64::from(x), i64::from(y));
            let index = if is_box(xi, yi) {
                PAPER
            } else if (-1..=1).any(|dy| (-1..=1).any(|dx| is_box(xi + dx, yi + dy))) {
                RULE
            } else {
                BAND
            };
            img.set(x, y, ega(index));
        }
    }
    carve_corners(img, rect);
}

/// Take the staircase of [`CHROME_CORNER_PROFILE`] out of all four corners:
/// on the row `r` rows in from the top or bottom edge, the band starts
/// `CHROME_CORNER_PROFILE[r]` pixels in from the left and right edges.
fn carve_corners(img: &mut MenuImage, rect: CellRect) {
    let (x0, y0, x1, y1) = rect.pixels();
    for (r, first) in CHROME_CORNER_PROFILE.iter().enumerate() {
        let r = r as u32;
        for d in 0..u32::from(*first) {
            for (x, y) in [
                (x0 + d, y0 + r),
                (x1 - d, y0 + r),
                (x0 + d, y1 - r),
                (x1 - d, y1 - r),
            ] {
                img.set(x, y, CLEAR);
            }
        }
    }
}

/// Draw one 8x8 glyph mask at cell `(col, row)`: set bits in `fg`, clear
/// bits in `bg` (left as they are when `None`).
fn draw_mask(img: &mut MenuImage, mask: [u8; 8], col: u32, row: u32, fg: u8, bg: Option<u8>) {
    let (bx, by) = (col * CELL, row * CELL);
    for (gy, bits) in mask.iter().enumerate() {
        for gx in 0..CH_CELL_SIDE {
            let set = bits & (0x80 >> gx) != 0;
            let index = if set { Some(fg) } else { bg };
            if let Some(index) = index {
                img.set(bx + gx as u32, by + gy as u32, ega(index));
            }
        }
    }
}

/// A glyph's eight rows; a code the font lacks is blank.
pub fn glyph(font: &FixedCellFont, code: u8) -> [u8; 8] {
    let mut mask = [0u8; 8];
    for (row, bits) in mask.iter_mut().enumerate() {
        *bits = font.glyph_row(code & 0x7f, row).unwrap_or(0);
    }
    mask
}

/// Draw `text` from cell `(col, row)`, one byte a cell, in `fg` on `bg`
/// (transparent where `None`).
pub fn draw_text(
    img: &mut MenuImage,
    font: &FixedCellFont,
    col: u32,
    row: u32,
    text: &str,
    fg: u8,
    bg: Option<u8>,
) {
    for (i, byte) in text.bytes().enumerate() {
        draw_mask(img, glyph(font, byte), col + i as u32, row, fg, bg);
    }
}

/// Draw the glyph `code` at cell `(col, row)` in `fg` on `bg`, for cells
/// that hold raw IBM.CH codes rather than text.
pub fn draw_code(
    img: &mut MenuImage,
    font: &FixedCellFont,
    (col, row): (u32, u32),
    code: u8,
    fg: u8,
    bg: Option<u8>,
) {
    draw_mask(img, glyph(font, code), col, row, fg, bg);
}

/// Draw a ribbon end cap at cell `(col, row)` over black: the triangle in
/// the band's blue and its two strokes in the rule's white, exactly the
/// faithful bracket primitive.
pub fn draw_cap(
    img: &mut MenuImage,
    font: &FixedCellFont,
    direction: RibbonCapDirection,
    col: u32,
    row: u32,
) {
    let sprite = ribbon_cap_sprite(font, direction);
    img.fill_cells(CellRect::new(col, row, 1, 1), PAPER);
    draw_mask(img, sprite.ribbon, col, row, BAND, None);
    draw_mask(img, sprite.white, col, row, RULE, None);
}

/// The cells a tab of `len` text cells takes from `col`: the two caps and
/// the text between them.
pub const fn tab_cells(len: usize) -> u32 {
    len as u32 + 2
}

/// A label tab on a band row (the ring or a divider): a black gap from
/// `col`, closed by a right-pointing cap, `text` in white, and a
/// left-pointing cap, as the original's `>Select:<` and `>Calm Winds<`.
/// Returns the last cell column it took.
pub fn draw_label_tab(
    img: &mut MenuImage,
    font: &FixedCellFont,
    col: u32,
    row: u32,
    text: &str,
    ink: u8,
) -> u32 {
    let last = col + tab_cells(text.len()) - 1;
    img.fill_cells(CellRect::new(col, row, last + 1 - col, 1), PAPER);
    draw_cap(img, font, RibbonCapDirection::Right, col, row);
    draw_text(img, font, col + 1, row, text, ink, Some(PAPER));
    draw_cap(img, font, RibbonCapDirection::Left, last, row);
    last
}

/// The first column that centres a tab of `len` cells on `rect`'s ring.
pub fn centred_tab_col(rect: CellRect, len: usize) -> u32 {
    rect.col + (rect.w - tab_cells(len)) / 2
}

/// The title tab, centred on the top ring.
pub fn draw_title_tab(img: &mut MenuImage, font: &FixedCellFont, rect: CellRect, text: &str) {
    draw_label_tab(
        img,
        font,
        centred_tab_col(rect, text.len()),
        rect.row,
        text,
        INK,
    );
}

/// The footer key tab, centred on the bottom ring.
pub fn draw_footer_tab(img: &mut MenuImage, font: &FixedCellFont, rect: CellRect, text: &str) {
    draw_label_tab(
        img,
        font,
        centred_tab_col(rect, text.len()),
        rect.bottom(),
        text,
        INK,
    );
}

/// Cells a tab strip of `items` takes: each item's tab and one band cell
/// between neighbours.
pub fn tab_strip_cells(items: &[&str]) -> u32 {
    items.iter().map(|item| tab_cells(item.len())).sum::<u32>() + items.len() as u32 - 1
}

/// The Settings tab strip on the top ring, centred: every item a tab, the
/// selected one's text in white and the others in light grey, one band
/// cell between them. Each item keeps its place whichever is selected.
pub fn draw_tab_strip(
    img: &mut MenuImage,
    font: &FixedCellFont,
    rect: CellRect,
    items: &[&str],
    selected: usize,
) {
    let mut col = rect.col + (rect.w - tab_strip_cells(items)) / 2;
    for (i, item) in items.iter().enumerate() {
        let ink = if i == selected { INK } else { HELP_INK };
        col = draw_label_tab(img, font, col, rect.row, item, ink) + 2;
    }
}

/// Which way a list scrolls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scroll {
    Up,
    Down,
    Both,
}

impl Scroll {
    /// The page glyph the original's arms browser shows in the same kind of
    /// three-cell tab.
    pub const fn glyph(self) -> u8 {
        match self {
            Self::Up => u5_runtime::ARMS_SELL_BROWSER_PAGE_GLYPH_UP,
            Self::Down => u5_runtime::ARMS_SELL_BROWSER_PAGE_GLYPH_DOWN,
            Self::Both => u5_runtime::ARMS_SELL_BROWSER_PAGE_GLYPH_BOTH,
        }
    }
}

/// A scroll mark: a three-cell tab on a band row, ending at `last_col`.
pub fn draw_scroll_mark(
    img: &mut MenuImage,
    font: &FixedCellFont,
    last_col: u32,
    row: u32,
    scroll: Scroll,
) {
    let text = [scroll.glyph()];
    let text = std::str::from_utf8(&text).expect("page glyphs are ASCII");
    draw_label_tab(img, font, last_col - 2, row, text, INK);
}

/// What a cell of a [`Line`] is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    /// White: labels.
    Plain,
    /// Light grey: values' units, tags, help.
    Help,
    /// Brown: the "Spoiler:" and "Cheats" words.
    Accent,
    /// Dark grey: disabled.
    Grey,
}

impl Ink {
    /// The colour on black.
    pub const fn colour(self) -> u8 {
        match self {
            Self::Plain => INK,
            Self::Help => HELP_INK,
            Self::Accent => ACCENT,
            Self::Grey => GREY,
        }
    }

    /// The glyph colour in the selection bar: the inverse for plain and help
    /// text (black on white); brown and grey keep their colour on white.
    pub const fn inverse(self) -> u8 {
        match self {
            Self::Plain | Self::Help => PAPER,
            Self::Accent => ACCENT,
            Self::Grey => GREY,
        }
    }
}

/// One row of cells, as wide as the box it is drawn in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub cells: Vec<(u8, Ink)>,
}

impl Line {
    /// A blank row of `width` cells.
    pub fn new(width: u32) -> Self {
        Self {
            cells: vec![(b' ', Ink::Plain); width as usize],
        }
    }

    /// `text` from cell `at`, cut at the row's end.
    pub fn text(mut self, at: u32, text: &str, ink: Ink) -> Self {
        for (i, byte) in text.bytes().enumerate() {
            if let Some(cell) = self.cells.get_mut(at as usize + i) {
                *cell = (byte, ink);
            }
        }
        self
    }

    /// `text` ending at cell `end` (inclusive).
    pub fn right(self, end: u32, text: &str, ink: Ink) -> Self {
        let at = end + 1 - text.len() as u32;
        self.text(at, text, ink)
    }

    /// A check box, `[x]` or `[ ]`, from cell `at`.
    pub fn check(self, at: u32, on: bool) -> Self {
        self.text(at, if on { "[x]" } else { "[ ]" }, Ink::Plain)
    }

    /// A slider of `inner` cells from cell `at`: `[####----]` filled to
    /// `percent`, rounded to the nearest cell.
    pub fn slider(self, at: u32, inner: u32, percent: u8) -> Self {
        let filled = (u32::from(percent.min(100)) * inner + 50) / 100;
        let mut bar = String::with_capacity(inner as usize + 2);
        bar.push('[');
        for i in 0..inner {
            bar.push(if i < filled { '#' } else { '-' });
        }
        bar.push(']');
        self.text(at, &bar, Ink::Plain)
    }

    /// A raw glyph code at cell `at`.
    pub fn glyph(mut self, at: u32, code: u8, ink: Ink) -> Self {
        if let Some(cell) = self.cells.get_mut(at as usize) {
            *cell = (code, ink);
        }
        self
    }

    /// The whole row greyed (a disabled row).
    pub fn greyed(mut self) -> Self {
        for cell in &mut self.cells {
            cell.1 = Ink::Grey;
        }
        self
    }

    /// The row's width in cells.
    pub fn width(&self) -> u32 {
        self.cells.len() as u32
    }
}

/// Draw `line` from cell `(col, row)`, plain (its inks on black) or as the
/// selection bar (white, glyphs in their inverse ink).
pub fn draw_line(
    img: &mut MenuImage,
    font: &FixedCellFont,
    col: u32,
    row: u32,
    line: &Line,
    selected: bool,
) {
    for (i, (code, ink)) in line.cells.iter().enumerate() {
        let (fg, bg) = if selected {
            (ink.inverse(), RULE)
        } else {
            (ink.colour(), PAPER)
        };
        draw_mask(img, glyph(font, *code), col + i as u32, row, fg, Some(bg));
    }
}

/// The selection bar: `line` across the whole interior of box `bx` on
/// `row`, in inverse video.
pub fn draw_selection_bar(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    row: u32,
    line: &Line,
) {
    assert_eq!(line.width(), bx.w, "the bar spans its box's interior");
    assert!(bx.contains(bx.col, row), "the bar is a row of its box");
    draw_line(img, font, bx.col, row, line, true);
}

/// A plain row of box `bx`.
pub fn draw_row(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    row: u32,
    line: &Line,
    selected: bool,
) {
    assert_eq!(line.width(), bx.w, "a row spans its box's interior");
    draw_line(img, font, bx.col, row, line, selected);
}

/// The text input line's prompt: the faithful right-pointing cap alone,
/// as the message window prefixes its input line.
pub fn draw_input_cap(img: &mut MenuImage, font: &FixedCellFont, col: u32, row: u32) {
    draw_cap(img, font, RibbonCapDirection::Right, col, row);
}

/// What a thumbnail box shows.
#[derive(Clone, Copy, Debug)]
pub enum Thumb<'a> {
    /// An 80x50 RGBA picture ([`thumbnail_from_frame`]).
    Picture(&'a [u8]),
    /// A slot imported from a DOS save: the word in the box.
    Imported,
    /// Nothing (a damaged slot): an empty box.
    Empty,
}

/// The 80x50 thumbnail of a 320x200 RGBA frame: each 4x4 block averaged.
pub fn thumbnail_from_frame(frame: &[u8]) -> Vec<u8> {
    let fw = crate::VISUAL_PLAY_FRAME_WIDTH as usize;
    assert_eq!(
        frame.len(),
        fw * crate::VISUAL_PLAY_FRAME_HEIGHT as usize * 4
    );
    let mut out = Vec::with_capacity((THUMB_W * THUMB_H * 4) as usize);
    for ty in 0..THUMB_H as usize {
        for tx in 0..THUMB_W as usize {
            let mut sum = [0u32; 3];
            for dy in 0..4 {
                for dx in 0..4 {
                    let i = ((ty * 4 + dy) * fw + tx * 4 + dx) * 4;
                    for (c, s) in sum.iter_mut().enumerate() {
                        *s += u32::from(frame[i + c]);
                    }
                }
            }
            out.extend(sum.map(|s| ((s + 8) / 16) as u8));
            out.push(0xff);
        }
    }
    out
}

/// The thumbnail box with its top-left cell at `(col, row)`: a 10x7-cell
/// black box in a 1 px white rule just outside it (on the last pixel of the
/// cells before and the first of the cells after, as a panel's rule), with
/// the picture centred in it.
pub fn draw_thumbnail_box(
    img: &mut MenuImage,
    font: &FixedCellFont,
    col: u32,
    row: u32,
    thumb: Thumb<'_>,
) {
    let cells = CellRect::new(col, row, THUMB_CELLS.0, THUMB_CELLS.1);
    let (x0, y0, x1, y1) = cells.pixels();
    let white = ega(RULE);
    img.fill((x0 - 1, y0 - 1, x1 + 1, y0 - 1), white);
    img.fill((x0 - 1, y1 + 1, x1 + 1, y1 + 1), white);
    img.fill((x0 - 1, y0, x0 - 1, y1), white);
    img.fill((x1 + 1, y0, x1 + 1, y1), white);
    img.fill_cells(cells, PAPER);
    match thumb {
        Thumb::Picture(rgba) => {
            assert_eq!(rgba.len(), (THUMB_W * THUMB_H * 4) as usize);
            for y in 0..THUMB_H {
                for x in 0..THUMB_W {
                    let i = ((y * THUMB_W + x) * 4) as usize;
                    img.set(
                        x0 + x,
                        y0 + THUMB_PAD + y,
                        [rgba[i], rgba[i + 1], rgba[i + 2], 0xff],
                    );
                }
            }
        }
        Thumb::Imported => {
            let word = super::text::IMPORTED;
            let at = col + (THUMB_CELLS.0 - word.len() as u32) / 2;
            draw_text(
                img,
                font,
                at,
                row + THUMB_CELLS.1 / 2,
                word,
                HELP_INK,
                Some(PAPER),
            );
        }
        Thumb::Empty => {}
    }
}

/// The backdrop: every pixel still clear becomes black at alpha 0.6.
pub fn dim_backdrop(img: &mut MenuImage) {
    for pixel in img.rgba.as_chunks_mut::<4>().0.iter_mut() {
        if pixel[3] == 0 {
            *pixel = DIM;
        }
    }
}

/// `menu` over an opaque `picture` of the same size, as the GPU blends the
/// node: `menu * a + picture * (1 - a)`.
pub fn composite_over(picture: &[u8], menu: &MenuImage) -> Vec<u8> {
    assert_eq!(
        picture.len(),
        menu.rgba.len(),
        "the menu covers the picture"
    );
    let mut out = picture.to_vec();
    for (dst, src) in out
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(menu.rgba.as_chunks::<4>().0)
    {
        let a = u32::from(src[3]);
        for c in 0..3 {
            dst[c] = ((u32::from(src[c]) * a + u32::from(dst[c]) * (255 - a) + 127) / 255) as u8;
        }
        dst[3] = 0xff;
    }
    out
}

/// The sweep step (1..=8) from which pixel `(x, y)` of `panel` shows the
/// panel: `step_of(sweep_t(x, y, top, bottom))` over the panel's rows, the
/// M1 law with `bottom` exclusive (D-M4.10).
pub fn panel_step(panel: CellRect, x: u32, y: u32) -> u8 {
    let (_, y0, _, y1) = panel.pixels();
    step_of(sweep_t(x, y, y0, y1 + 1))
}

/// The menu at sweep step `k` (`None`: the whole menu): inside `panel` a
/// pixel shows the menu where its step is `k` or less and the backdrop
/// elsewhere; outside it the menu (the backdrop) as it is. Opening runs
/// `k` up from 1, closing down from 7.
pub fn sweep_image(full: &MenuImage, panel: CellRect, k: Option<u8>) -> MenuImage {
    sweep_image_over(full, panel, k, DIM)
}

/// [`sweep_image`] with `hidden` for the panel pixels not yet shown: the
/// backdrop's [`DIM`] in an image that carries its own backdrop, [`CLEAR`]
/// in the shell's, whose backdrop is a node of its own.
pub fn sweep_image_over(
    full: &MenuImage,
    panel: CellRect,
    k: Option<u8>,
    hidden: [u8; 4],
) -> MenuImage {
    let mut out = full.clone();
    let Some(k) = k else {
        return out;
    };
    let (x0, y0, x1, y1) = panel.pixels();
    for y in y0..=y1.min(full.h - 1) {
        for x in x0..=x1.min(full.w - 1) {
            if panel_step(panel, x, y) > k {
                out.set(x, y, hidden);
            }
        }
    }
    out
}
