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
struct Ink {
    x: u32,
    y: u32,
    colour: u8,
}

/// Step 7: the crawl (see the module text). Returns the y of each line
/// drawn, top to bottom.
pub fn paint_crawl(rgba: &mut [u8], canvas: &Canvas, paint: CrawlPaint<'_>) -> Vec<u32> {
    let lines = shown_lines(paint.rows, paint.crawl);
    if lines.is_empty() {
        return Vec::new();
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
            .filter_map(|cell| {
                let colour = cell_colour(line, cell.typed, paint.typed_colour)?;
                Some((cell.cell, cell.mask, colour))
            })
            .collect();
        if line.kind == LineKind::Live {
            live_y = Some(y);
            if let Some(cursor) = &paint.rows.cursor {
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
    // clipped to the map window.
    let (mx0, my0, mx1, my1) = canvas.map_window();
    let black = rgb(0);
    for pixel in &ink {
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (x, y) = (i64::from(pixel.x) + dx, i64::from(pixel.y) + dy);
                if x < i64::from(mx0)
                    || x > i64::from(mx1)
                    || y < i64::from(my0)
                    || y > i64::from(my1)
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
    if let (Some(y), Some(cap)) = (live_y, paint.rows.cap.as_ref()) {
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
    ys
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
