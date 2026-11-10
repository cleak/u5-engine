//! Test-only recorder for the message crawl (Phase B M1-12, design 2.2.4,
//! 2.2.9 and 2.3.8).
//!
//! While recording is on, the play-frame compositor hands over the message
//! rows it painted ([`painted`]) and then the finished frame with what it
//! was drawn with ([`note`]). The recorder runs the crawl's own sync on the
//! same state and checks:
//!
//! - the rows the crawl reads (`placed_rows`, its own call of the integrated
//!   surface on a clone) are the rows the frame painted;
//! - every row is on the frame where the layout placed it, so the crawl
//!   left none out as unshown, and every crawl character is in the frame's
//!   placed rows, in order (the no-reveal rule);
//! - a prompt row is drawn only above the live `:` row;
//! - the crawl, drawn on every named layout and a panned narrow one, never
//!   touches the faithful core;
//! - (M1-13) with the column open, every older row of its message window is
//!   text the transcript once held (an entry's own text or the text the
//!   window renders for it), and the open window, like the crawl, never
//!   touches the core.
//!
//! Nothing here is compiled outside tests, and the recorder is off unless a
//! test turns it on.

use std::cell::RefCell;
use std::path::Path;

use u5_runtime::{FixedCellFont, MessageWindowRow, PlayState};

use crate::display::crawl::layer::LineKind;
use crate::display::crawl::sync::{CrawlInputs, CrawlState, placed_rows};
use crate::display::grow::Canvas;
use crate::display::grow::column::{ColumnText, paint_open_window};
use crate::display::grow::compose::core_rect;
use crate::display::grow::crawl_paint::{CrawlPaint, paint_crawl};
use crate::grow_agreement::WINDOWS;

#[derive(Default, Debug)]
pub(crate) struct Tally {
    /// Frames checked.
    pub frames: usize,
    /// Frames whose crawl drew a committed row.
    pub with_text: usize,
    /// Frames whose crawl had a live row.
    pub with_live: usize,
    /// Frames whose crawl drew a prompt row.
    pub with_prompt: usize,
    /// Frames whose open window showed older rows.
    pub with_older: usize,
    /// Older rows checked against the transcript.
    pub older_rows: usize,
    /// Syncs whose older rows could not be lined up with the placed rows.
    pub unaligned: usize,
    pub mismatches: Vec<String>,
}

#[derive(Default)]
struct Recorder {
    tally: Tally,
    painted: Option<Vec<MessageWindowRow>>,
    crawl: CrawlState,
    /// Every transcript entry's text seen so far, as stored and as the
    /// window renders it.
    held: std::collections::HashSet<String>,
}

thread_local! {
    static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// Start recording on this thread, dropping anything recorded before.
pub(crate) fn start() {
    RECORDER.with(|cell| *cell.borrow_mut() = Some(Recorder::default()));
}

/// Stop recording and return the tally.
pub(crate) fn finish() -> Tally {
    RECORDER.with(|cell| {
        cell.borrow_mut()
            .take()
            .map(|recorder| recorder.tally)
            .unwrap_or_default()
    })
}

/// The rows the frame being composed painted.
pub(crate) fn painted(rows: &[MessageWindowRow]) {
    RECORDER.with(|cell| {
        if let Some(recorder) = cell.borrow_mut().as_mut() {
            recorder.painted = Some(rows.to_vec());
        }
    });
}

/// What [`note`] is told about the frame besides the state.
pub(crate) struct Drawn<'a> {
    pub ibm: &'a FixedCellFont,
    pub runes: &'a FixedCellFont,
    pub game_dir: &'a Path,
    pub cursor_frame: u64,
    pub input_line: &'a str,
    pub prompt_cursor_visible: bool,
}

/// Check one finished frame of `state`.
pub(crate) fn note(state: &PlayState, drawn: Drawn<'_>, frame: &[u8]) {
    RECORDER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(recorder) = borrow.as_mut() else {
            return;
        };
        let Some(painted) = recorder.painted.take() else {
            return;
        };
        let inputs = CrawlInputs {
            ibm: drawn.ibm,
            runes: drawn.runes,
            game_dir: drawn.game_dir,
            cursor_frame: drawn.cursor_frame,
            input_line: drawn.input_line,
            prompt_cursor_visible: drawn.prompt_cursor_visible,
        };
        let mut found = Vec::new();
        let placed = placed_rows(state, inputs);
        if placed.rows != painted {
            found.push(format!(
                "the crawl reads {:?}, the frame painted {:?}",
                texts(&placed.rows),
                texts(&painted)
            ));
        }
        let unshown = recorder.crawl.unshown_rows();
        let mut display = state.clone();
        if !display.message.is_empty() {
            display.message = crate::visual_display_message(&display);
        }
        for entry in state
            .message_entries()
            .iter()
            .chain(state.displayed_message_entries())
        {
            recorder.held.insert(entry.text.clone());
            recorder
                .held
                .insert(crate::visual_display_line(&display, &entry.text));
        }
        recorder.crawl.set_history_wanted(true);
        recorder.crawl.observe_input(drawn.input_line);
        recorder.crawl.sync(state, frame, inputs);
        if recorder.crawl.unshown_rows() != unshown {
            found.push(format!(
                "rows the frame does not show: {:?}",
                texts(&placed.rows)
            ));
        }
        let rows = recorder.crawl.rows();
        // Every crawl character is in the painted rows, in order.
        let mut painted_texts = painted.iter().map(|row| row.text.as_str());
        for line in &rows.lines {
            let text = line.text();
            if line.cells.is_empty() {
                continue;
            }
            if !painted_texts.any(|painted| painted.contains(text.trim_end())) {
                found.push(format!(
                    "the crawl line {text:?} is not in the painted rows"
                ));
            }
        }
        for (index, line) in rows.lines.iter().enumerate() {
            if line.kind == LineKind::Prompt
                && rows.lines.get(index + 1).map(|below| below.kind) != Some(LineKind::Live)
            {
                found.push(format!(
                    "the prompt row {:?} is not above the live row",
                    line.text()
                ));
            }
        }
        // The open window's older rows: only text the transcript held.
        for line in &rows.older {
            let text = line.text();
            let text = text.trim();
            if text.is_empty() {
                found.push("an empty older row".to_string());
                continue;
            }
            if !recorder.held.iter().any(|held| held.contains(text)) {
                found.push(format!(
                    "the older row {text:?} is no text the transcript held"
                ));
            }
        }
        // The crawl never covers the core.
        for (ww, wh) in WINDOWS.iter().copied().chain([(1920, 1440)]) {
            let canvas = Canvas::for_window(ww, wh, u5_runtime::enhancements::PixelAspect::Dos)
                .expect("a canvas");
            let mut rgba = vec![0x40; canvas.w as usize * canvas.h as usize * 4];
            let before = rgba.clone();
            paint_crawl(
                &mut rgba,
                &canvas,
                CrawlPaint {
                    rows: &rows,
                    crawl: true,
                    typed_colour: true,
                },
            );
            let (cx, cy, side) = core_rect(&canvas);
            let w = canvas.w as usize;
            let covered = (cy..cy + side).any(|y| {
                let at = (y * w + cx) * 4;
                rgba[at..at + side * 4] != before[at..at + side * 4]
            });
            if covered {
                found.push(format!("{ww}x{wh}: the crawl covers the core"));
            }
            let mut open = before.clone();
            paint_open_window(
                &mut open,
                &canvas,
                ColumnText {
                    rows: &rows,
                    typed_colour: true,
                },
            );
            let covered = (cy..cy + side).any(|y| {
                let at = (y * w + cx) * 4;
                open[at..at + side * 4] != before[at..at + side * 4]
            });
            if covered {
                found.push(format!("{ww}x{wh}: the open window covers the core"));
            }
        }
        let tally = &mut recorder.tally;
        tally.frames += 1;
        if rows.lines.iter().any(|line| line.kind == LineKind::Text) {
            tally.with_text += 1;
        }
        if rows.live().is_some() {
            tally.with_live += 1;
        }
        if rows.lines.iter().any(|line| line.kind == LineKind::Prompt) {
            tally.with_prompt += 1;
        }
        if !rows.older.is_empty() {
            tally.with_older += 1;
        }
        tally.older_rows += rows.older.len();
        tally.unaligned = recorder.crawl.unaligned_syncs();
        tally.mismatches.extend(found);
    });
}

fn texts(rows: &[MessageWindowRow]) -> Vec<&str> {
    rows.iter().map(|row| row.text.as_str()).collect()
}
