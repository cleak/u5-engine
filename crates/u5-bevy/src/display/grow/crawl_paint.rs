//! The grow-left compositor, step 7 (Phase B design 2.2.4, 2.3.7; build
//! plan M1-12): the message crawl over the map.
//!
//! Up to six rows rise from the bottom of the map window, the live row
//! last, the bottom row on y `H - 16 .. H - 9` (224..231 on 240-row
//! canvases, 200..207 on the 216-row one). Text starts at x 16. There is
//! no backing: every glyph gets a 1 px black outline (its eight
//! neighbours), clipped to the map window, and the map shows through
//! everywhere else. The live row carries the original's end cap at
//! x 8..15, merged into the left border: where the cap's first column is
//! band blue, the rule pixel at x 7 beside it is blue too, so the border's
//! blue flows into the cap and the white rule traces its outside.
//!
//! Colours (2.2.4): game text white, then grey at ages 1-2, then dark grey
//! at 3-4, gone from 5; typed text bright cyan, cyan, dark grey with
//! `display.typed_colour` on, the game colours with it off. A prompt row
//! and the live row are fresh. With `display.crawl` off only the live row
//! is drawn.

use u5_runtime::CH_CELL_SIDE;

use super::Canvas;
use super::compose::BAND_BLUE;
use crate::EGA_PALETTE_RGB;
use crate::display::crawl::ages::tier_of;
use crate::display::crawl::layer::{
    CRAWL_ROWS, CURSOR_COLOUR, CrawlLine, CrawlRows, GAME_COLOURS, LineKind, TYPED_COLOURS,
};

/// The text column: one cell right of the left border.
pub const TEXT_X: u32 = 16;
/// The end cap's column, merged into the left border.
pub const CAP_X: u32 = 8;
const CELL: u32 = CH_CELL_SIDE as u32;

/// What step 7 draws from.
#[derive(Clone, Copy)]
pub struct CrawlPaint<'a> {
    pub rows: &'a CrawlRows,
    /// `display.crawl`: the committed rows too, not only the live row.
    pub crawl: bool,
    /// `display.typed_colour`.
    pub typed_colour: bool,
}

/// The top of the bottom crawl row: 8 rows above the bottom band.
pub fn crawl_bottom_y(canvas: &Canvas) -> u32 {
    canvas.map_window().3 + 1 - CELL
}

/// The colour of one cell of `line`, or `None` when the line has aged out.
pub fn cell_colour(line: &CrawlLine, typed: bool, typed_colour: bool) -> Option<u8> {
    let tier = match line.kind {
        LineKind::Text => tier_of(line.age)?,
        LineKind::Prompt | LineKind::Live => 0,
    };
    let colours = if typed && typed_colour {
        TYPED_COLOURS
    } else {
        GAME_COLOURS
    };
    Some(colours[tier])
}

/// The lines the crawl shows, top to bottom: at most [`CRAWL_ROWS`], the
/// newest, leaving out the rows that have aged out; with the crawl off,
/// the live row alone.
pub fn shown_lines(rows: &CrawlRows, crawl: bool) -> Vec<&CrawlLine> {
    let shown: Vec<&CrawlLine> = rows
        .lines
        .iter()
        .filter(|line| match line.kind {
            LineKind::Live => true,
            LineKind::Prompt => crawl,
            LineKind::Text => crawl && tier_of(line.age).is_some(),
        })
        .collect();
    let skip = shown.len().saturating_sub(CRAWL_ROWS);
    shown[skip..].to_vec()
}

/// One pixel of glyph ink on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ink {
    pub x: u32,
    pub y: u32,
    pub colour: u8,
}

/// The rectangle a text block's outline is clipped to, `(x0, y0, x1, y1)`
/// inclusive: the map window for the crawl, the column's message window
/// for the open column.
pub type Clip = (u32, u32, u32, u32);

/// What [`paint_lines`] drew: the y of each line, top to bottom, and every
/// glyph pixel (the cursor's included; the end cap's not).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Painted {
    pub ys: Vec<u32>,
    pub ink: Vec<Ink>,
}

/// Step 7: the crawl (see the module text). Returns the y of each line
/// drawn, top to bottom.
pub fn paint_crawl(rgba: &mut [u8], canvas: &Canvas, paint: CrawlPaint<'_>) -> Vec<u32> {
    paint_crawl_painted(rgba, canvas, paint).ys
}

/// [`paint_crawl`], returning everything it drew (the dither effect's text
/// mask is its glyphs and their outlines).
pub fn paint_crawl_painted(rgba: &mut [u8], canvas: &Canvas, paint: CrawlPaint<'_>) -> Painted {
    let lines = shown_lines(paint.rows, paint.crawl);
    let typed_colour = paint.typed_colour;
    paint_lines(
        rgba,
        canvas,
        &lines,
        paint.rows,
        canvas.map_window(),
        |line, typed| cell_colour(line, typed, typed_colour),
    )
}

/// Draw `lines` (top to bottom) so that the last one sits on
/// [`crawl_bottom_y`], text from x 16, each glyph with a 1 px black
/// outline clipped to `clip`, in the colour `colour(line, typed)` gives
/// (`None`: the cell is not drawn). The live line, when it is among them,
/// carries the cursor of `rows` and the end cap at x 8..15, merged into the
/// border at x 7. Shared by the crawl (step 7) and the open column's
/// message window (step 8), so a row is on the same pixels in both.
pub fn paint_lines(
    rgba: &mut [u8],
    canvas: &Canvas,
    lines: &[&CrawlLine],
    rows: &CrawlRows,
    clip: Clip,
    colour: impl Fn(&CrawlLine, bool) -> Option<u8>,
) -> Painted {
    if lines.is_empty() {
        return Painted::default();
    }
    let bottom = crawl_bottom_y(canvas);
    let top = bottom - CELL * (lines.len() as u32 - 1);
    let w = canvas.w;
    let mut ink = Vec::new();
    let mut ys = Vec::with_capacity(lines.len());
    let mut live_y = None;
    for (index, line) in lines.iter().enumerate() {
        let y = top + CELL * index as u32;
        ys.push(y);
        let mut cells: Vec<(u8, [u8; 8], u8)> = line
            .cells
            .iter()
            .filter_map(|cell| Some((cell.cell, cell.mask, colour(line, cell.typed)?)))
            .collect();
        if line.kind == LineKind::Live {
            live_y = Some(y);
            if let Some(cursor) = &rows.cursor {
                cells.push((cursor.cell, cursor.mask, CURSOR_COLOUR));
            }
        }
        for (cell, mask, colour) in cells {
            let x0 = TEXT_X + CELL * u32::from(cell);
            for (row, bits) in mask.iter().enumerate() {
                for col in 0..CELL {
                    if bits & (0x80 >> col) != 0 && x0 + col < w {
                        ink.push(Ink {
                            x: x0 + col,
                            y: y + row as u32,
                            colour,
                        });
                    }
                }
            }
        }
    }
    // The outline: every ink pixel's eight neighbours (and itself), black,
    // clipped.
    let (cx0, cy0, cx1, cy1) = clip;
    let black = rgb(0);
    for pixel in &ink {
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (x, y) = (i64::from(pixel.x) + dx, i64::from(pixel.y) + dy);
                if x < i64::from(cx0)
                    || x > i64::from(cx1)
                    || y < i64::from(cy0)
                    || y > i64::from(cy1)
                {
                    continue;
                }
                put(rgba, w, x as u32, y as u32, black);
            }
        }
    }
    for pixel in &ink {
        put(rgba, w, pixel.x, pixel.y, rgb(pixel.colour));
    }
    if let (Some(y), Some(cap)) = (live_y, rows.cap.as_ref()) {
        for (row, pixels) in cap.iter().enumerate() {
            let py = y + row as u32;
            for (col, pixel) in pixels.iter().enumerate() {
                if let Some(index) = pixel {
                    put(rgba, w, CAP_X + col as u32, py, rgb(*index));
                }
            }
            // The border's blue flows into the cap.
            if pixels[0] == Some(BAND_BLUE) {
                put(rgba, w, CAP_X - 1, py, rgb(BAND_BLUE));
            }
        }
    }
    Painted { ys, ink }
}

fn rgb(index: u8) -> [u8; 3] {
    EGA_PALETTE_RGB[usize::from(index & 0x0f)]
}

fn put(rgba: &mut [u8], w: u32, x: u32, y: u32, colour: [u8; 3]) {
    let at = ((y * w + x) * 4) as usize;
    if at + 3 < rgba.len() {
        rgba[at..at + 3].copy_from_slice(&colour);
    }
}

#[cfg(test)]
#[path = "crawl_paint_tests.rs"]
mod tests;
