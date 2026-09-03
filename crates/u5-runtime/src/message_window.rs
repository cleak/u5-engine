//! The gameplay message/command window: the right-hand column below
//! the stats boxes, which echoes each command, prints its output, and
//! carries the live input line on its own bottom row.
//!
//! # Provenance
//!
//! `systems/text-output.md §10` publishes the standing window-2 rectangle,
//! bottom-anchored cursor, command-echo cadence, line-prefix composite,
//! scrolling behavior, and live input line. `cleak/u5-spec#79` is closed.
//!
//! Each echoed
//! command occupies one row prefixed by the ribbon end-cap sprite in
//! column 24 with text from column 25; each pure output line is
//! unprefixed from column 24; and one blank row follows each command
//! turn. The live input line is the last row of the same window — the
//! original has no separate bottom-left prompt window.

use crate::gameplay_chrome::{
    MESSAGE_WINDOW_BOTTOM, MESSAGE_WINDOW_LEFT, MESSAGE_WINDOW_RIGHT, MESSAGE_WINDOW_TOP,
};

/// Cells available to an unprefixed output line, columns 24..=38.
/// `text-output.md §4` defines a window's printable width as
/// `bottom_right_x - top_left_x`, excluding the trailing column, so
/// the window's right edge at column 39 is never printed into.
pub const MESSAGE_WINDOW_WIDTH: usize = (MESSAGE_WINDOW_RIGHT - MESSAGE_WINDOW_LEFT) as usize;
/// Cells available to a prefixed line, whose text starts at column 25.
pub const MESSAGE_WINDOW_PREFIXED_WIDTH: usize = MESSAGE_WINDOW_WIDTH - 1;
/// Rows in the window (11..=23), the last of which is the live input.
pub const MESSAGE_WINDOW_ROWS: usize = (MESSAGE_WINDOW_BOTTOM - MESSAGE_WINDOW_TOP) as usize + 1;
/// Rows available to scrolled-back history, i.e. every row but the
/// live input line.
pub const MESSAGE_WINDOW_HISTORY_ROWS: usize = MESSAGE_WINDOW_ROWS - 1;

/// How one logged line is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageLineKind {
    /// Echoed command: end-cap prefix in column 24, text from 25.
    Command,
    /// Handler output: unprefixed, from column 24.
    Output,
    /// The blank row that closes a command turn.
    Blank,
}

impl MessageLineKind {
    /// Whether this line carries the ribbon end-cap prefix sprite.
    pub const fn prefixed(self) -> bool {
        matches!(self, Self::Command)
    }

    /// Text cells available to this line.
    pub const fn width(self) -> usize {
        if self.prefixed() {
            MESSAGE_WINDOW_PREFIXED_WIDTH
        } else {
            MESSAGE_WINDOW_WIDTH
        }
    }
}

/// One already-wrapped row of the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageLogLine {
    /// Row text, never wider than `kind.width()`.
    pub text: String,
    /// Font-preserving cells aligned with `text`.
    pub glyphs: Vec<crate::TlkRenderedGlyph>,
    /// How the row is drawn.
    pub kind: MessageLineKind,
    pub centered: bool,
}

/// A scrolling log of command echoes and handler output.
///
/// The log holds wrapped rows, so the window is a plain tail view. It
/// is deliberately capped: only the rows that can still scroll into
/// view are retained.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameplayMessageLog {
    lines: Vec<MessageLogLine>,
}

impl GameplayMessageLog {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rows currently held, oldest first.
    pub fn lines(&self) -> &[MessageLogLine] {
        &self.lines
    }

    /// Whether anything has been logged.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Drop every logged row.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Append an echoed command line.
    pub fn push_command(&mut self, text: &str) {
        self.push_wrapped_plain(text, MessageLineKind::Command);
    }

    /// Append one or more handler-output lines.
    pub fn push_output(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        self.push_wrapped_plain(text, MessageLineKind::Output);
    }

    pub fn push_tlk_output(&mut self, text: &str, glyphs: &[crate::TlkRenderedGlyph]) {
        if text.trim().is_empty() {
            return;
        }
        self.push_wrapped_glyphs(glyphs, MessageLineKind::Output);
    }

    /// Close a command turn with the single blank row the original
    /// leaves between turns. Consecutive blanks collapse.
    pub fn end_turn(&mut self) {
        if self.lines.is_empty() {
            return;
        }
        if matches!(
            self.lines.last().map(|line| line.kind),
            Some(MessageLineKind::Blank)
        ) {
            return;
        }
        self.lines.push(MessageLogLine {
            text: String::new(),
            glyphs: Vec::new(),
            kind: MessageLineKind::Blank,
            centered: false,
        });
        self.trim();
    }

    fn push_wrapped_plain(&mut self, text: &str, kind: MessageLineKind) {
        let glyphs = crate::ordinary_glyphs_from_engine_text(text);
        self.push_wrapped_glyphs(&glyphs, kind);
    }

    fn push_wrapped_glyphs(&mut self, glyphs: &[crate::TlkRenderedGlyph], kind: MessageLineKind) {
        for glyphs in wrap_rendered_to_width(glyphs, kind.width()) {
            let text = glyphs.iter().map(|glyph| char::from(glyph.byte)).collect();
            self.lines.push(MessageLogLine {
                text,
                glyphs,
                kind,
                centered: false,
            });
        }
        self.trim();
    }

    fn trim(&mut self) {
        if self.lines.len() > MESSAGE_WINDOW_HISTORY_ROWS {
            let excess = self.lines.len() - MESSAGE_WINDOW_HISTORY_ROWS;
            self.lines.drain(0..excess);
        }
    }
}

/// One placed row of the window: an absolute screen row plus the text
/// and whether it takes the end-cap prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageWindowRow {
    /// Absolute screen row, 11..=23.
    pub row: u8,
    /// Absolute column the text starts at: 24, or 25 when prefixed.
    pub column: u8,
    /// Row text.
    pub text: String,
    /// Font-preserving cells aligned with `text`.
    pub glyphs: Vec<crate::TlkRenderedGlyph>,
    /// Whether the ribbon end-cap sprite is drawn in column 24.
    pub prefixed: bool,
}

/// The window's placed rows for one frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageWindowLayout {
    /// Rows to draw, top to bottom.
    pub rows: Vec<MessageWindowRow>,
}

impl MessageWindowLayout {
    /// Screen rows that take the end-cap prefix sprite.
    pub fn prefixed_rows(&self) -> Vec<u8> {
        self.rows
            .iter()
            .filter(|row| row.prefixed)
            .map(|row| row.row)
            .collect()
    }
}

/// Project a `PlayState` message transcript into the window's log.
///
/// `text-output.md §11`: the transcript is the record of what was
/// printed, so the window is drawn from it rather than from the
/// single-line slot — a turn that produced an announcement *and* a
/// command result has both entries here, and both are drawn.
///
/// `render` decides how each line reaches the window: `None` drops it
/// (blank filler the window derives for itself), `Some(text)` supplies
/// the text to draw, which lets a caller substitute names into it.
///
/// The blank row between turns is derived rather than stored, per
/// `text-output.md §10.4`: it is the next command cycle's leading line
/// feed landing on a row the previous turn already closed, so one blank
/// precedes every command echo but the first.
pub fn message_log_from_entries<'a>(
    entries: impl IntoIterator<Item = &'a crate::MessageEntry>,
    mut render: impl FnMut(&str) -> Option<String>,
) -> GameplayMessageLog {
    let mut log = GameplayMessageLog::new();
    for entry in entries {
        // An explicit blank row carries no text, so it is placed before
        // the renderer is consulted: a caller that drops empty lines - the
        // Bevy shell's own `keep` filter does - must not be able to erase
        // a row the producer asked for. Such a row is output the original
        // produces, not padding this window adds: `text-output.md` section
        // 10.4 derives one from the leading line feed of the next print
        // ("the next cycle's leading line feed advances again - producing
        // exactly one blank row after each completed command turn"), and
        // section 10.3 lists echoes that emit one deliberately ("two
        // newlines | Complete, plus one deliberate extra blank row").
        if entry.explicit_blank {
            log.lines.push(MessageLogLine {
                text: String::new(),
                glyphs: Vec::new(),
                kind: MessageLineKind::Blank,
                centered: false,
            });
            log.trim();
            continue;
        }
        let Some(text) = render(&entry.text) else {
            continue;
        };
        if entry.is_command_echo {
            log.end_turn();
            log.push_command(&text);
        } else if entry.centered && text == entry.text {
            let glyphs = entry.glyphs.clone();
            log.lines.push(MessageLogLine {
                text,
                glyphs,
                kind: MessageLineKind::Output,
                centered: true,
            });
            log.trim();
        } else if text == entry.text {
            log.push_tlk_output(&text, &entry.glyphs);
        } else {
            log.push_output(&text);
        }
    }
    log
}

/// Place a log — and optionally the live input line — into the window.
///
/// History is bottom-anchored just above the live input row, so the
/// most recent output always sits directly above the prompt. When
/// `live_input` is `None` the history uses the bottom row too.
pub fn layout_message_window(
    log: &GameplayMessageLog,
    live_input: Option<&str>,
) -> MessageWindowLayout {
    let mut rows = Vec::new();
    let history_rows = match live_input {
        Some(_) => MESSAGE_WINDOW_HISTORY_ROWS,
        None => MESSAGE_WINDOW_ROWS,
    };
    let lines = log.lines();
    let start = lines.len().saturating_sub(history_rows);
    let placed = &lines[start..];
    let first_row = MESSAGE_WINDOW_TOP as usize + (history_rows - placed.len());
    for (offset, line) in placed.iter().enumerate() {
        if matches!(line.kind, MessageLineKind::Blank) {
            continue;
        }
        let prefixed = line.kind.prefixed();
        rows.push(MessageWindowRow {
            row: (first_row + offset) as u8,
            column: if line.centered {
                MESSAGE_WINDOW_LEFT
                    + crate::text_window_centred_start_column(
                        16,
                        line.glyphs.len().min(u8::MAX as usize) as u8,
                    )
            } else {
                MESSAGE_WINDOW_LEFT + u8::from(prefixed)
            },
            text: line.text.clone(),
            glyphs: line.glyphs.clone(),
            prefixed,
        });
    }
    if let Some(live) = live_input {
        let text: String = live.chars().take(MESSAGE_WINDOW_PREFIXED_WIDTH).collect();
        let glyphs = text
            .bytes()
            .map(crate::TlkRenderedGlyph::ordinary)
            .collect();
        rows.push(MessageWindowRow {
            row: MESSAGE_WINDOW_BOTTOM,
            column: MESSAGE_WINDOW_LEFT + 1,
            text,
            glyphs,
            prefixed: true,
        });
    }
    MessageWindowLayout { rows }
}

fn wrap_rendered_to_width(
    glyphs: &[crate::TlkRenderedGlyph],
    width: usize,
) -> Vec<Vec<crate::TlkRenderedGlyph>> {
    let mut rows = Vec::new();
    if width == 0 {
        return rows;
    }
    let mut buffer = Vec::new();
    let mut last_break = None;
    for glyph in glyphs.iter().copied() {
        match glyph.byte {
            b' ' => {
                buffer.push(glyph);
                last_break = Some(buffer.len() - 1);
            }
            b'\n' | b'\r' => {
                trim_trailing_spaces(&mut buffer);
                rows.push(std::mem::take(&mut buffer));
                last_break = None;
            }
            _ => {
                if !buffer.is_empty() && buffer.len() + 1 > width {
                    if let Some(break_at) = last_break {
                        let mut surplus = buffer.split_off(break_at);
                        trim_trailing_spaces(&mut buffer);
                        rows.push(std::mem::take(&mut buffer));
                        while surplus.first().is_some_and(|glyph| glyph.byte == b' ') {
                            surplus.remove(0);
                        }
                        buffer = surplus;
                    } else {
                        rows.push(std::mem::take(&mut buffer));
                    }
                    last_break = buffer.iter().rposition(|glyph| glyph.byte == b' ');
                }
                buffer.push(glyph);
            }
        }
    }
    if !buffer.is_empty() {
        trim_trailing_spaces(&mut buffer);
        rows.push(buffer);
    }
    rows
}

fn trim_trailing_spaces(glyphs: &mut Vec<crate::TlkRenderedGlyph>) {
    while glyphs.last().is_some_and(|glyph| glyph.byte == b' ') {
        glyphs.pop();
    }
}

#[cfg(test)]
mod font_tests {
    use super::*;
    use crate::{TlkGlyphFont, TlkRenderedGlyph};

    #[test]
    fn styled_wrap_preserves_runic_cells_across_a_word_break() {
        let mut glyphs: Vec<_> = b"ab "
            .iter()
            .copied()
            .map(TlkRenderedGlyph::ordinary)
            .collect();
        glyphs.extend(b"cdef".iter().copied().map(TlkRenderedGlyph::runic));

        let rows = wrap_rendered_to_width(&glyphs, 4);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].iter().map(|glyph| glyph.byte).collect::<Vec<_>>(),
            b"ab"
        );
        assert_eq!(
            rows[1].iter().map(|glyph| glyph.byte).collect::<Vec<_>>(),
            b"cdef"
        );
        assert!(
            rows[1]
                .iter()
                .all(|glyph| glyph.font == TlkGlyphFont::Runic)
        );
    }
}
