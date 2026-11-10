//! The crawl's shell side (Phase B design 2.3.8, 2.3.9; build plan M1-12):
//! [`CrawlState`] and the `sync_crawl` system.
//!
//! **Where the rows come from.** The faithful frame's message rows are
//! placed by `render_integrated_status_framebuffer`, which the faithful
//! paint drops after painting. The crawl does not edit that path: it calls
//! the same function itself, on a clone of the game (the function resets
//! the active-player selector on the state it is given), with the shell's
//! input line, cursor visibility and cursor frame, and reads the rows it
//! returns. The cursor's cell is found where that surface drew the cursor
//! glyph.
//!
//! **When.** Only on a frame whose faithful bytes, input line, turn or
//! conversation changed, in `EnhSet::Present` before the pipeline host, so
//! a row reaches the crawl on the frame the original window shows it.
//!
//! **No reveal (2.2.9).** Every row is checked against the faithful frame
//! itself before it is kept: each of its glyphs must be on the frame, white
//! on black, in the cell the layout placed it in. A row the frame does not
//! show (yet) is left out, so the crawl only ever draws text the original
//! screen is showing.

use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use bevy::prelude::*;
use u5_runtime::enhancements::fnv1a64;
use u5_runtime::{
    CH_CELL_SIDE, CHROME_RULE_INDEX, FixedCellFont, MESSAGE_WINDOW_BOTTOM, MESSAGE_WINDOW_LEFT,
    MESSAGE_WINDOW_RIGHT, MESSAGE_WINDOW_TOP, MessageWindowRow, PlayState, TlkGlyphFont,
    prompt_cursor_glyph,
};

use super::ages::CrawlAges;
use super::layer::{CapSprite, CrawlRows, PlacedRows, crawl_lines};
use super::{
    TypedLog, classify_rows, echo_row_flags, entry_log, render_outputs, row_lines, tagged_outputs,
};
use crate::{EGA_PALETTE_RGB, VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, VisualState};

const FRAME_W: usize = VISUAL_PLAY_FRAME_WIDTH as usize;
const FRAME_H: usize = VISUAL_PLAY_FRAME_HEIGHT as usize;

/// The crawl rows the `[grow-left compose]` step draws, shared between
/// [`CrawlState`] (which writes them) and the step (which reads them). No
/// threads: the lock is never contended.
#[derive(Clone, Debug, Default)]
pub struct CrawlFeed(Arc<Mutex<CrawlRows>>);

impl CrawlFeed {
    pub fn new() -> Self {
        Self::default()
    }

    /// The rows as last synced.
    pub fn rows(&self) -> MutexGuard<'_, CrawlRows> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// What the faithful frame was drawn with, besides the game.
#[derive(Clone, Copy)]
pub struct CrawlInputs<'a> {
    pub ibm: &'a FixedCellFont,
    pub runes: &'a FixedCellFont,
    pub game_dir: &'a Path,
    /// The live line's barber-pole frame.
    pub cursor_frame: u64,
    /// The shell's input line.
    pub input_line: &'a str,
    pub prompt_cursor_visible: bool,
}

/// The crawl's session state: the ages, the typed-line log, the rows.
/// Exists only while `display.grow_left` is on (D2.9: memory only).
#[derive(Resource, Debug, Default)]
pub struct CrawlState {
    ages: CrawlAges,
    typed: TypedLog,
    /// What the rows were last built from.
    key: Option<u64>,
    feed: CrawlFeed,
    /// Rows left out so far because the faithful frame did not show them.
    unshown: usize,
}

impl CrawlState {
    pub fn new(feed: CrawlFeed) -> Self {
        Self {
            feed,
            ..Self::default()
        }
    }

    /// The feed the compose step reads.
    pub fn feed(&self) -> &CrawlFeed {
        &self.feed
    }

    /// A copy of the rows as last synced.
    pub fn rows(&self) -> CrawlRows {
        self.feed.rows().clone()
    }

    /// Rows left out so far because the faithful frame did not show them.
    pub fn unshown_rows(&self) -> usize {
        self.unshown
    }

    /// The typed-line log.
    pub fn typed(&self) -> &TypedLog {
        &self.typed
    }

    /// The crawl's clock (turns outside conversations).
    pub fn clock(&self) -> u64 {
        self.ages.clock()
    }

    /// Note the shell's input line; called every frame, so a submit (the
    /// line cleared) is seen.
    pub fn observe_input(&mut self, input_line: &str) {
        self.typed.observe(input_line);
    }

    /// Rebuild the rows from `state` and its faithful frame `faithful`
    /// (320x200 RGBA) when anything they depend on changed. Read-only on
    /// the game: the render runs on a clone. Returns whether it rebuilt.
    pub fn sync(&mut self, state: &PlayState, faithful: &[u8], inputs: CrawlInputs<'_>) -> bool {
        if faithful.len() != FRAME_W * FRAME_H * 4 {
            return false;
        }
        let key = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            fnv1a64(faithful).hash(&mut hasher);
            inputs.input_line.hash(&mut hasher);
            inputs.prompt_cursor_visible.hash(&mut hasher);
            inputs.cursor_frame.hash(&mut hasher);
            state.turn.hash(&mut hasher);
            state.active_conversation.is_some().hash(&mut hasher);
            hasher.finish()
        };
        if self.key == Some(key) {
            return false;
        }
        self.key = Some(key);
        self.typed.observe(inputs.input_line);
        let built = build_rows(state, faithful, inputs, &mut self.ages, &self.typed);
        self.unshown += built.unshown;
        let mut rows = self.feed.rows();
        let mut next = built.rows;
        next.revision = rows.revision;
        if *rows != next {
            next.revision = rows.revision.wrapping_add(1);
            *rows = next;
        }
        true
    }
}

/// The game as the faithful message window sees it: the copy the integrated
/// surface builds, with the display message the shell shows.
fn display_state(state: &PlayState) -> PlayState {
    let mut display = state.clone();
    display.refresh_cached_moon_glyphs();
    if !display.message.is_empty() {
        display.message = crate::visual_display_message(&display);
    }
    display
}

/// The rows `render_integrated_status_framebuffer` places for `state`, and
/// its cursor cell.
pub fn placed_rows(state: &PlayState, inputs: CrawlInputs<'_>) -> PlacedRows {
    let ctx = crate::PlayFrameContext {
        ibm: inputs.ibm,
        runes: inputs.runes,
        game_dir: inputs.game_dir,
        cursor_frame: inputs.cursor_frame,
    };
    let mut copy = state.clone();
    let surface = crate::render_integrated_status_framebuffer(
        &mut copy,
        inputs.input_line,
        "",
        ctx,
        inputs.prompt_cursor_visible,
    );
    let live_row_placed = !display_state(state).message_window_live_row_suppressed();
    let cursor_mask = glyph_mask(inputs.ibm, prompt_cursor_glyph(inputs.cursor_frame));
    let cursor = find_cursor(&surface.rgba, &cursor_mask);
    PlacedRows {
        rows: surface.message_rows,
        live_row_placed,
        cursor,
    }
}

/// The eight rows of `code` in `font`; empty rows where the font has none.
pub fn glyph_mask(font: &FixedCellFont, code: u8) -> [u8; 8] {
    let mut mask = [0u8; 8];
    for (row, bits) in mask.iter_mut().enumerate() {
        *bits = font.glyph_row(code, row).unwrap_or(0);
    }
    mask
}

fn row_mask(row: &MessageWindowRow, at: usize, inputs: CrawlInputs<'_>) -> [u8; 8] {
    let glyph = row.glyphs[at];
    let font = match glyph.font {
        TlkGlyphFont::Ordinary => inputs.ibm,
        TlkGlyphFont::Runic => inputs.runes,
    };
    glyph_mask(font, glyph.byte)
}

/// Whether cell `(column, row)` of a 320x200 RGBA frame shows `mask` in the
/// text white on black, exactly.
pub fn cell_shows(frame: &[u8], column: u8, row: u8, mask: &[u8; 8]) -> bool {
    let white = EGA_PALETTE_RGB[usize::from(CHROME_RULE_INDEX)];
    let (x0, y0) = (
        usize::from(column) * CH_CELL_SIDE,
        usize::from(row) * CH_CELL_SIDE,
    );
    if x0 + CH_CELL_SIDE > FRAME_W || y0 + CH_CELL_SIDE > FRAME_H {
        return false;
    }
    mask.iter().enumerate().all(|(dy, bits)| {
        (0..CH_CELL_SIDE).all(|dx| {
            let at = ((y0 + dy) * FRAME_W + x0 + dx) * 4;
            let pixel = [frame[at], frame[at + 1], frame[at + 2]];
            if bits & (0x80 >> dx) != 0 {
                pixel == white
            } else {
                pixel == [0, 0, 0]
            }
        })
    })
}

/// The bottom-most message-window cell showing the cursor glyph.
fn find_cursor(surface: &[u8], mask: &[u8; 8]) -> Option<(u8, u8)> {
    if mask.iter().all(|bits| *bits == 0) {
        return None;
    }
    (MESSAGE_WINDOW_TOP..=MESSAGE_WINDOW_BOTTOM)
        .rev()
        .find_map(|row| {
            (MESSAGE_WINDOW_LEFT..=MESSAGE_WINDOW_RIGHT)
                .find(|&column| cell_shows(surface, column, row, mask))
                .map(|column| (column, row))
        })
}

/// Whether the faithful frame shows every glyph of `row` where the layout
/// placed it (a row of spaces shows trivially).
fn row_shown(frame: &[u8], row: &MessageWindowRow, inputs: CrawlInputs<'_>) -> bool {
    (0..row.glyphs.len()).all(|at| {
        let column = usize::from(row.column) + at;
        column <= usize::from(MESSAGE_WINDOW_RIGHT)
            && cell_shows(frame, column as u8, row.row, &row_mask(row, at, inputs))
    })
}

/// The live row's end cap, as the faithful frame paints it on the message
/// window's black: band blue and rule white, the rest transparent.
pub fn cap_sprite(ibm: &FixedCellFont) -> CapSprite {
    let mut frame = vec![0u8; FRAME_W * FRAME_H * 4];
    let row = MESSAGE_WINDOW_TOP;
    u5_runtime::paint_message_line_cap(
        &mut frame,
        FRAME_W,
        FRAME_H,
        ibm,
        row,
        u5_runtime::ChromePalette::EGA,
    );
    let mut cap = [[None; 8]; 8];
    let (x0, y0) = (
        usize::from(MESSAGE_WINDOW_LEFT) * CH_CELL_SIDE,
        usize::from(row) * CH_CELL_SIDE,
    );
    for (dy, pixels) in cap.iter_mut().enumerate() {
        for (dx, pixel) in pixels.iter_mut().enumerate() {
            let at = ((y0 + dy) * FRAME_W + x0 + dx) * 4;
            if frame[at..at + 3] != [0, 0, 0] {
                *pixel = Some(crate::ega_palette_index_from_rgba(&frame[at..at + 4]));
            }
        }
    }
    cap
}

struct Built {
    rows: CrawlRows,
    unshown: usize,
}

/// Steps 1-4 of a sync: the placed rows, their ages and classes, checked
/// against the frame, as crawl lines.
fn build_rows(
    state: &PlayState,
    faithful: &[u8],
    inputs: CrawlInputs<'_>,
    ages: &mut CrawlAges,
    typed: &TypedLog,
) -> Built {
    let mut placed = placed_rows(state, inputs);

    // The transcript the window projects, as the faithful render projects
    // it, for the echo flags and the ages.
    let display = display_state(state);
    let keep =
        |text: &str| (!text.trim().is_empty()).then(|| crate::visual_display_line(&display, text));
    let entries = display.displayed_message_entries();
    let outputs = render_outputs(entries, keep);
    let unflushed = display.displayed_unflushed_message().and_then(keep);
    ages.sync(entries, state.turn, state.active_conversation.is_some());
    let echo = match tagged_outputs(entries, &outputs, unflushed.as_deref()) {
        Some((lines, typed_lines)) => {
            echo_row_flags(&placed.rows, &lines, &typed_lines, placed.live_row_placed)
        }
        None => placed.rows.iter().map(|row| row.prefixed).collect(),
    };
    let row_ages: Vec<u64> = entry_log(entries, &outputs, unflushed.as_deref())
        .and_then(|(lines, sources)| {
            let matched = row_lines(&placed.rows, &lines, placed.live_row_placed)?;
            Some(
                matched
                    .iter()
                    .map(|line| {
                        line.and_then(|line| sources[line])
                            .map_or(0, |entry| ages.age_of(entry))
                    })
                    .collect(),
            )
        })
        .unwrap_or_else(|| vec![0; placed.rows.len()]);

    // Only what the faithful frame shows.
    let mut unshown = 0;
    let mut keep_rows = Vec::with_capacity(placed.rows.len());
    let mut keep_echo = Vec::with_capacity(placed.rows.len());
    let mut keep_ages = Vec::with_capacity(placed.rows.len());
    let last = placed.rows.len().checked_sub(1);
    let live = placed.last_row_live();
    for (index, row) in placed.rows.iter().enumerate() {
        if row_shown(faithful, row, inputs) {
            keep_rows.push(row.clone());
        } else if live && Some(index) == last {
            // The live row keeps its place (the triangle and the cursor);
            // only the text the frame does not show is dropped.
            unshown += 1;
            let mut empty = row.clone();
            empty.text.clear();
            empty.glyphs.clear();
            keep_rows.push(empty);
        } else {
            unshown += 1;
            continue;
        }
        keep_echo.push(echo.get(index).copied().unwrap_or(false));
        keep_ages.push(row_ages.get(index).copied().unwrap_or(0));
    }
    placed.rows = keep_rows;
    let cursor_mask = glyph_mask(inputs.ibm, prompt_cursor_glyph(inputs.cursor_frame));
    if let Some((column, row)) = placed.cursor
        && !cell_shows(faithful, column, row, &cursor_mask)
    {
        placed.cursor = None;
    }
    let classes = classify_rows(&placed.rows, typed, &keep_echo, live);
    let (lines, cursor) = crawl_lines(
        &placed,
        &classes,
        &keep_ages,
        typed.current(),
        |row, at| row_mask(row, at, inputs),
        (prompt_cursor_glyph(inputs.cursor_frame), cursor_mask),
    );
    Built {
        rows: CrawlRows {
            lines,
            cursor,
            cap: Some(cap_sprite(inputs.ibm)),
            revision: 0,
        },
        unshown,
    }
}

/// `sync_crawl`, in `EnhSet::Present` before the pipeline host: note the
/// input line, then rebuild the rows when the faithful frame changed.
/// Read-only on the game.
pub(crate) fn sync_crawl(
    visual: Option<Res<VisualState>>,
    images: Res<Assets<Image>>,
    mut crawl: ResMut<CrawlState>,
) {
    let Some(visual) = visual else {
        return;
    };
    crawl.observe_input(&visual.input_line);
    let Some(bytes) = images
        .get(&visual.image_handle)
        .and_then(|image| image.data.as_deref())
    else {
        return;
    };
    let inputs = CrawlInputs {
        ibm: &visual.text_font,
        runes: &visual.rune_font,
        game_dir: &visual.game_dir,
        cursor_frame: visual.prompt_cursor_frame,
        input_line: &visual.input_line,
        prompt_cursor_visible: visual.prompt_cursor_visible,
    };
    crawl.sync(&visual.state, bytes, inputs);
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
