//! The crawl's drawable rows (Phase B design 2.2.4, 2.3.8; build plan
//! M1-12): the rows the faithful message window placed, classified and aged,
//! as cells ready for the compositor's step 7.
//!
//! [`crawl_lines`] is pure. It keeps every placed row except the ones the
//! crawl never draws (a prompt row whose `:` row is not live, an empty row,
//! a committed bare `:`), and records for each cell its glyph mask and
//! whether the player typed it. Rows that have aged out are kept too, with
//! their age, so the open column (M1-13) can show them in their fresh
//! colours on the same pixels.

use u5_runtime::{MESSAGE_WINDOW_LEFT, MessageWindowRow, TLK_KEYWORD_PROMPT_OPEN_LINE};

use super::RowClass;

/// Rows the crawl shows at most, the live row included (2.2.4).
pub const CRAWL_ROWS: usize = 6;
/// Game text: white, grey, dark grey (EGA 15, 7, 8).
pub const GAME_COLOURS: [u8; 3] = [15, 7, 8];
/// Player-typed text: bright cyan, cyan, dark grey (EGA 11, 3, 8).
pub const TYPED_COLOURS: [u8; 3] = [11, 3, 8];
/// The live row's cursor, the original's white.
pub const CURSOR_COLOUR: u8 = 15;

/// What a crawl line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// A committed row of game or typed text; it ages.
    Text,
    /// The first line of an open prompt, drawn in fresh game white only
    /// while the `:` row below it is live (2.2.4).
    Prompt,
    /// The row the player types on: the triangle, the prompt in white, the
    /// typed characters in the typed colour, the cursor.
    Live,
}

/// One glyph cell of a crawl line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrawlCell {
    /// Cells right of the text column (x 16).
    pub cell: u8,
    /// The byte the faithful frame drew.
    pub byte: u8,
    /// The glyph's eight rows, from the font the faithful frame drew it
    /// with (runic text stays runic).
    pub mask: [u8; 8],
    /// Whether the player typed it.
    pub typed: bool,
}

/// One line of the crawl, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrawlLine {
    pub kind: LineKind,
    /// Turns since the row first appeared; 0 for prompt and live rows.
    pub age: u64,
    pub cells: Vec<CrawlCell>,
    /// The screen row of the faithful window it was placed on.
    pub frame_row: u8,
}

impl CrawlLine {
    /// The bytes the line draws, in cell order.
    pub fn text(&self) -> String {
        self.cells
            .iter()
            .map(|cell| char::from(cell.byte))
            .collect()
    }
}

/// The live row's end cap, the original's two-colour sprite: per pixel the
/// palette index it paints (band blue or rule white), `None` where the map
/// shows through.
pub type CapSprite = [[Option<u8>; 8]; 8];

/// Everything the compositor's step 7 draws, rebuilt when the faithful
/// frame or the input line changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrawlRows {
    pub lines: Vec<CrawlLine>,
    /// The cursor glyph on the live line (the last line), when the faithful
    /// frame draws one.
    pub cursor: Option<CrawlCell>,
    /// The end cap the live line carries at x 8..15.
    pub cap: Option<CapSprite>,
    /// The open column's older rows (2.2.5), oldest first: what the
    /// transcript and the history buffer held above `lines`, projected at
    /// the window's width. Built only while the column wants them
    /// ([`super::sync::CrawlState::set_history_wanted`]); empty otherwise.
    pub older: Vec<CrawlLine>,
    /// Bumped on every rebuild that changed something.
    pub revision: u64,
}

impl CrawlRows {
    /// The live line, when there is one.
    pub fn live(&self) -> Option<&CrawlLine> {
        self.lines.last().filter(|line| line.kind == LineKind::Live)
    }
}

/// The rows the faithful window placed this frame, and where its cursor is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlacedRows {
    pub rows: Vec<MessageWindowRow>,
    /// Whether the layout added its own live row (the bottom row).
    pub live_row_placed: bool,
    /// The cursor's absolute `(column, row)`, when the frame draws one.
    pub cursor: Option<(u8, u8)>,
}

impl PlacedRows {
    /// Whether the last row is the one the player types on: the layout's
    /// live row, or the row carrying the cursor inline.
    pub fn last_row_live(&self) -> bool {
        let Some(last) = self.rows.last() else {
            return false;
        };
        self.live_row_placed || self.cursor.is_some_and(|(_, row)| row == last.row)
    }
}

/// The first cell column of a placed row: the end-cap column is not a text
/// cell, so a prefixed row's text starts in the crawl's first cell too.
fn row_base(row: &MessageWindowRow) -> u8 {
    MESSAGE_WINDOW_LEFT + u8::from(row.prefixed)
}

/// Whether a committed row is only the open prompt's `:` (nothing was
/// typed): the colon belongs to the prompt, which is gone.
fn bare_open_line(row: &MessageWindowRow) -> bool {
    row.text.trim_end() == TLK_KEYWORD_PROMPT_OPEN_LINE
}

/// The crawl's lines for `placed`, with `classes` from
/// [`super::classify_rows`] (the last row `Live` exactly when
/// [`PlacedRows::last_row_live`]), `ages` per placed row, the line being
/// typed (`typing`, the shell's input line), the glyph masks and the
/// cursor's glyph.
///
/// A cursor below the last placed row (a prompt whose trailing space
/// wrapped) gets a live line of its own.
pub fn crawl_lines(
    placed: &PlacedRows,
    classes: &[RowClass],
    ages: &[u64],
    typing: &str,
    mut mask_of: impl FnMut(&MessageWindowRow, usize) -> [u8; 8],
    cursor_glyph: (u8, [u8; 8]),
) -> (Vec<CrawlLine>, Option<CrawlCell>) {
    let rows = &placed.rows;
    let mut lines = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let class = classes.get(index).copied().unwrap_or(RowClass::Game);
        let offset = row.column.saturating_sub(row_base(row));
        let cell_of = |at: usize| offset.saturating_add(at.min(usize::from(u8::MAX)) as u8);
        let mut cells: Vec<CrawlCell> = row
            .glyphs
            .iter()
            .enumerate()
            .map(|(at, glyph)| CrawlCell {
                cell: cell_of(at),
                byte: glyph.byte,
                mask: mask_of(row, at),
                typed: false,
            })
            .collect();
        let (kind, age) = match class {
            RowClass::Live => {
                // What the player has typed so far is the row's tail.
                let typed = typing.as_bytes();
                let tail = cells.len().checked_sub(typed.len()).filter(|&from| {
                    !typed.is_empty()
                        && cells[from..]
                            .iter()
                            .map(|cell| cell.byte)
                            .collect::<Vec<_>>()
                            .eq_ignore_ascii_case(typed)
                });
                if let Some(from) = tail {
                    for cell in &mut cells[from..] {
                        cell.typed = true;
                    }
                }
                (LineKind::Live, 0)
            }
            RowClass::Prompt => {
                let below_live = classes.get(index + 1) == Some(&RowClass::Live);
                if !below_live {
                    continue;
                }
                (LineKind::Prompt, 0)
            }
            RowClass::Typed => {
                for cell in &mut cells {
                    cell.typed = true;
                }
                (LineKind::Text, ages.get(index).copied().unwrap_or(0))
            }
            RowClass::Mixed { from } => {
                let from = usize::from(from);
                for cell in cells.iter_mut().skip(from) {
                    cell.typed = true;
                }
                // A committed `:word` keeps only the word: the colon
                // belongs to the prompt (2.2.4).
                if from == TLK_KEYWORD_PROMPT_OPEN_LINE.len()
                    && row.text.starts_with(TLK_KEYWORD_PROMPT_OPEN_LINE)
                {
                    cells.drain(..from);
                    for cell in &mut cells {
                        cell.cell = cell.cell.saturating_sub(from as u8);
                    }
                }
                (LineKind::Text, ages.get(index).copied().unwrap_or(0))
            }
            RowClass::Game => {
                if bare_open_line(row) {
                    continue;
                }
                (LineKind::Text, ages.get(index).copied().unwrap_or(0))
            }
        };
        if kind == LineKind::Text && cells.iter().all(|cell| cell.byte == b' ') {
            continue;
        }
        lines.push(CrawlLine {
            kind,
            age,
            cells,
            frame_row: row.row,
        });
    }
    let cursor = placed.cursor.and_then(|(column, row)| {
        let base = match lines.last() {
            Some(live) if live.kind == LineKind::Live && live.frame_row == row => rows
                .iter()
                .rev()
                .find(|placed| placed.row == row)
                .map_or(MESSAGE_WINDOW_LEFT, row_base),
            _ => {
                // The cursor waits on a row of its own below the text.
                let below = rows.last().is_none_or(|last| row > last.row);
                if !below {
                    return None;
                }
                lines.push(CrawlLine {
                    kind: LineKind::Live,
                    age: 0,
                    cells: Vec::new(),
                    frame_row: row,
                });
                MESSAGE_WINDOW_LEFT
            }
        };
        Some(CrawlCell {
            cell: column.saturating_sub(base),
            byte: cursor_glyph.0,
            mask: cursor_glyph.1,
            typed: false,
        })
    });
    (lines, cursor)
}

#[cfg(test)]
#[path = "layer_tests.rs"]
mod tests;
