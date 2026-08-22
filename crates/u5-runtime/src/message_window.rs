//! The gameplay message/command window: the right-hand column below
//! the stats boxes, which echoes each command, prints its output, and
//! carries the live input line on its own bottom row.
//!
//! # Provenance
//!
//! `systems/text-output.md §9` step 3 says gameplay routines call the
//! rectangle setter to lay out "the main text area, the status panel,
//! the input prompt, and so on" but never publishes the values.
//! The rectangle and line cadence here are measured by black-box
//! observation of the original (see `gameplay_chrome` for the method);
//! the pending spec question is `cleak/u5-spec#79`.
//!
//! Observed cadence, from a capture with five prior turns: each echoed
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
    /// How the row is drawn.
    pub kind: MessageLineKind,
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
        self.push_wrapped(text, MessageLineKind::Command);
    }

    /// Append one or more handler-output lines.
    pub fn push_output(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        self.push_wrapped(text, MessageLineKind::Output);
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
            kind: MessageLineKind::Blank,
        });
        self.trim();
    }

    fn push_wrapped(&mut self, text: &str, kind: MessageLineKind) {
        for row in wrap_to_width(text, kind.width()) {
            self.lines.push(MessageLogLine { text: row, kind });
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
            column: MESSAGE_WINDOW_LEFT + u8::from(prefixed),
            text: line.text.clone(),
            prefixed,
        });
    }
    if let Some(live) = live_input {
        let text: String = live.chars().take(MESSAGE_WINDOW_PREFIXED_WIDTH).collect();
        rows.push(MessageWindowRow {
            row: MESSAGE_WINDOW_BOTTOM,
            column: MESSAGE_WINDOW_LEFT + 1,
            text,
            prefixed: true,
        });
    }
    MessageWindowLayout { rows }
}

/// Word-wrap `text` to `width` cells, hard-splitting words that cannot
/// fit. Always yields at least one row for non-empty input.
fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    if width == 0 {
        return rows;
    }
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let mut word = word;
            while word.chars().count() > width {
                if !current.is_empty() {
                    rows.push(std::mem::take(&mut current));
                }
                let split: String = word.chars().take(width).collect();
                rows.push(split);
                word = &word[char_boundary(word, width)..];
            }
            if word.is_empty() {
                continue;
            }
            let needed = word.chars().count() + usize::from(!current.is_empty());
            if current.chars().count() + needed > width {
                rows.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            rows.push(current);
        }
    }
    rows
}

fn char_boundary(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

/// True when the message is the engine's own scene-entry narration —
/// `"Entered <scene> at (x, y)."` and its level/plane variants, plus
/// the moongate arrival line.
///
/// These print the party's raw map coordinates and no `systems/`
/// document specifies them: they are harness diagnostics rather than
/// game text, so the gameplay message window leaves them out. They
/// stay in `PlayState::message` for the terminal harness and for the
/// tests that assert on scene entry.
pub fn message_is_scene_entry_narration(message: &str, x: usize, y: usize) -> bool {
    message.starts_with("Entered ") && message.contains(&format!(" at ({x}, {y})."))
}
