//! The message crawl's row classes, the typed-line log and the history
//! buffer (Phase B design 2.2.4, 2.2.5, 2.3.8; build plan M1-11).
//!
//! Everything here is a pure function or a plain value: no Bevy system, no
//! resource registration and no read of the game's state beyond the slices
//! it is handed. The crawl (M1-12) and the docked column (M1-13) build on
//! it.
//!
//! - [`RowClass`] and [`classify_rows`] sort the rows the faithful window
//!   placed into game text, typed text, a prompt row that is drawn only
//!   while its `:` row is live, and the live row itself.
//! - [`echo_row_flags`] says exactly which placed rows a command echo
//!   wrote, wrapped continuations included. The rows alone cannot say it: a
//!   wrapped echo's continuation is an ordinary unprefixed row
//!   ([`MessageLineKind::continuation`]), the same as the handler output on
//!   the row beneath an echo. So the transcript is projected a second time
//!   with every cell masked by its entry's kind, and the placed rows are
//!   matched to the projection from the bottom up.
//! - [`TypedLog`] remembers the lines the player submitted, so a committed
//!   `:bread` row is split into its prompt and its typed tail by what the
//!   shell actually sent, not by guessing from the text.
//! - [`MessageHistory`] keeps the transcript entries the transcript has
//!   dropped, found by comparing slices rather than trusting
//!   `message_transcript_revision` (2.3.8).
//!
//! The transcript's capacity ([`MESSAGE_TRANSCRIPT_CAPACITY`]) and the
//! window's width ([`u5_runtime::MESSAGE_WINDOW_WIDTH`]) are never changed: the older
//! rows are projected with the runtime's own [`message_log_from_entries`]
//! at the window's sixteen cells.

use std::collections::VecDeque;

use u5_runtime::tlk_control_codes::TLK_SCOPED_PROMPT;
use u5_runtime::{
    MESSAGE_TRANSCRIPT_CAPACITY, MESSAGE_WINDOW_LEFT, MessageEntry, MessageLineKind,
    MessageLogLine, MessageWindowRow, TLK_ASK_WHO_PROMPT, TLK_KEYWORD_PROMPT,
    TLK_KEYWORD_PROMPT_OPEN_LINE, TlkRenderedGlyph, message_log_from_entries,
};

/// Rows of the tallest open message window: 17 on 240-row canvases
/// (y 96..231), 14 on the 216-row canvas (2.2.5).
pub const OPEN_WINDOW_ROWS_MAX: usize = 17;

/// Committed entries that can still show a row which [`MessageHistory`]
/// keeps, counted from the newest.
///
/// Each such entry projects to at least one row, so
/// [`OPEN_WINDOW_ROWS_MAX`] of them always fill the window. Twice that
/// leaves room for rows the window leaves out - a prompt row, an emptied
/// `:` row - without the buffer growing past what the window can show
/// (D2.9: session only, the rows that fit).
pub const HISTORY_KEPT_ENTRIES: usize = 2 * OPEN_WINDOW_ROWS_MAX;

/// Submitted lines [`TypedLog`] remembers, newest first.
///
/// As many as the transcript can hold entries, so a committed input row
/// still in the transcript (or in the history) can always find the line it
/// was typed as.
pub const TYPED_LOG_DEPTH: usize = MESSAGE_TRANSCRIPT_CAPACITY;

/// What a placed message row is, for its colour and for whether it is
/// drawn at all (2.2.4, 2.3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowClass {
    /// Text the game printed: white, then grey, then dark grey.
    Game,
    /// Text the player caused to be printed: a command echo and its
    /// wrapped continuations, or a whole row the player typed under a
    /// prompt.
    Typed,
    /// Game text, then typed cells from `from` (a cell index into the
    /// row's glyphs). A committed `:bread` is `Mixed { from: 1 }`; the crawl
    /// drops its leading `:`, which belongs to the prompt (2.2.4).
    Mixed { from: u8 },
    /// The first line of a keyword, scoped or ASK-WHO prompt directly above
    /// a `:` row. Drawn, in game white, only while the row below it is the
    /// live row; never kept in history.
    Prompt,
    /// The row the player is typing on: the layout's live row or the row
    /// carrying its inline cursor.
    Live,
}

/// The lines the player submitted, newest first.
///
/// A shell resource in M1-12. It snapshots `VisualState::input_line` each
/// frame through [`TypedLog::observe`], and when a non-empty line is
/// cleared it records that line as submitted. Matching a committed row's
/// tail against what the shell actually sent is exact; guessing the typed
/// part from the text alone is not (the row-open flag lesson).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypedLog {
    /// The input line as it stood at the last observation.
    current: String,
    /// Submitted lines, newest first, at most [`TYPED_LOG_DEPTH`].
    submitted: VecDeque<String>,
}

impl TypedLog {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Note the shell's input line as it stands this frame.
    ///
    /// A line that was non-empty at the last observation and is empty now
    /// was submitted (a submit is what clears the line). Backspacing to
    /// empty records the last character, which can only matter if a row
    /// then ends in exactly that text after a prompt's own text - and the
    /// row would not, since nothing was sent.
    pub fn observe(&mut self, input_line: &str) {
        if input_line.is_empty() && !self.current.is_empty() {
            let line = std::mem::take(&mut self.current);
            self.record(line);
        } else if input_line != self.current {
            self.current.clear();
            self.current.push_str(input_line);
        }
    }

    /// Record `line` as submitted.
    pub fn record(&mut self, line: impl Into<String>) {
        let line = line.into();
        if line.is_empty() {
            return;
        }
        self.submitted.push_front(line);
        self.submitted.truncate(TYPED_LOG_DEPTH);
    }

    /// The line being typed, as last observed.
    pub fn current(&self) -> &str {
        &self.current
    }

    /// Submitted lines, newest first.
    pub fn submitted(&self) -> impl Iterator<Item = &str> {
        self.submitted.iter().map(String::as_str)
    }
}

/// The rows of the open-prompt literals that sit directly above their `:`
/// input row: `Your interest?` ([`TLK_KEYWORD_PROMPT`]) and `You respond-`
/// ([`TLK_SCOPED_PROMPT`], which [`TLK_ASK_WHO_PROMPT`] also ends with).
///
/// Taken from the literals themselves, so a change to one changes this.
pub fn prompt_lines() -> Vec<&'static str> {
    let mut lines = Vec::new();
    for prompt in [TLK_KEYWORD_PROMPT, TLK_SCOPED_PROMPT, TLK_ASK_WHO_PROMPT] {
        let mut rows = prompt.rsplit('\n');
        let open = rows.next();
        debug_assert_eq!(open, Some(TLK_KEYWORD_PROMPT_OPEN_LINE));
        if let Some(line) = rows.next().filter(|line| !line.is_empty())
            && !lines.contains(&line)
        {
            lines.push(line);
        }
    }
    lines
}

/// Whether `text` is a committed or live `:` input row.
fn is_open_line(text: &str) -> bool {
    text.starts_with(TLK_KEYWORD_PROMPT_OPEN_LINE)
}

/// Whether `text`, trimmed, ends the way a prompt that waits for typed
/// input ends: `:`, `?` or `-` (`Your interest?` / `:`, `How much? `,
/// `You respond-`, `Player: `).
fn ends_like_a_prompt(text: &str) -> bool {
    text.trim_end()
        .chars()
        .last()
        .is_some_and(|last| matches!(last, ':' | '?' | '-'))
}

/// Whether row `below` sits on the screen row directly under row `above`.
fn adjacent(above: &MessageWindowRow, below: &MessageWindowRow) -> bool {
    below.row == above.row.wrapping_add(1)
}

/// The cell the submitted tail of row `index` starts at, if the row ends
/// in a line the player submitted after a prompt's own text.
///
/// `Some(0)` is a whole row the player typed under a prompt row (the
/// sage's topic, `SPIR`). The comparison ignores ASCII case, because some
/// prompts echo upper-cased (`typed_prompt_echo_uppercases`: the spell
/// name, the shrine's virtue).
fn typed_tail(rows: &[MessageWindowRow], index: usize, submitted: &TypedLog) -> Option<u8> {
    let row = &rows[index];
    let cells: Vec<u8> = row.glyphs.iter().map(|glyph| glyph.byte).collect();
    for line in submitted.submitted() {
        let line = line.as_bytes();
        if line.is_empty() || line.len() > cells.len() {
            continue;
        }
        let from = cells.len() - line.len();
        if !cells[from..].eq_ignore_ascii_case(line) {
            continue;
        }
        let head = String::from_utf8_lossy(&cells[..from]);
        if from == 0 {
            let under_a_prompt = index
                .checked_sub(1)
                .map(|above| &rows[above])
                .is_some_and(|above| adjacent(above, row) && ends_like_a_prompt(&above.text));
            if under_a_prompt {
                return Some(0);
            }
            continue;
        }
        if head.trim().is_empty() || !ends_like_a_prompt(&head) {
            continue;
        }
        return Some(from.min(usize::from(u8::MAX)) as u8);
    }
    None
}

/// Classify the rows the faithful window placed (2.3.8).
///
/// - `echo[i]` says row `i` was written by a command echo, its wrapped
///   continuations included ([`echo_row_flags`]). Missing entries count as
///   `false`.
/// - `live` says the layout placed a live row or an inline cursor, which
///   makes the last row [`RowClass::Live`].
///
/// A row equal to a prompt's first line ([`prompt_lines`]) directly above a
/// `:` row is [`RowClass::Prompt`], live or not: whether it is drawn is
/// the crawl's decision, from the class of the row below. An echo row is
/// [`RowClass::Typed`]. Any other row that ends in a submitted line
/// ([`TypedLog`]) after a prompt's own text is [`RowClass::Mixed`] (or
/// `Typed` when the whole row is the submitted line, under a prompt row).
/// Everything else is [`RowClass::Game`].
pub fn classify_rows(
    rows: &[MessageWindowRow],
    submitted: &TypedLog,
    echo: &[bool],
    live: bool,
) -> Vec<RowClass> {
    let prompts = prompt_lines();
    let last = rows.len().checked_sub(1);
    (0..rows.len())
        .map(|index| {
            let row = &rows[index];
            if live && Some(index) == last {
                return RowClass::Live;
            }
            let prompt_above_open_line = prompts.contains(&row.text.as_str())
                && rows
                    .get(index + 1)
                    .is_some_and(|below| adjacent(row, below) && is_open_line(&below.text));
            if prompt_above_open_line {
                return RowClass::Prompt;
            }
            if echo.get(index).copied().unwrap_or(false) {
                return RowClass::Typed;
            }
            match typed_tail(rows, index, submitted) {
                Some(0) => RowClass::Typed,
                Some(from) => RowClass::Mixed { from },
                None => RowClass::Game,
            }
        })
        .collect()
}

/// The cell a command echo's projection is masked with.
const TYPED_MARK: u8 = b'T';
/// The cell every other entry's projection is masked with.
const GAME_MARK: u8 = b'g';

/// `text` with every cell the wrap does not break on replaced by `mark`.
///
/// The window's wrap breaks only on spaces and line feeds, so the masked
/// text wraps exactly as the original does. Non-ASCII characters are
/// dropped, as `glyphs_from_engine_text` drops them.
fn mask_text(text: &str, mark: u8) -> String {
    text.chars()
        .filter(char::is_ascii)
        .map(|ch| match ch {
            ' ' | '\n' | '\r' => ch,
            _ => char::from(mark),
        })
        .collect()
}

/// [`mask_text`] for a glyph list, keeping each cell's font.
fn mask_glyphs(glyphs: &[TlkRenderedGlyph], mark: u8) -> Vec<TlkRenderedGlyph> {
    glyphs
        .iter()
        .map(|glyph| TlkRenderedGlyph {
            byte: match glyph.byte {
                b' ' | b'\n' | b'\r' => glyph.byte,
                _ => mark,
            },
            font: glyph.font,
        })
        .collect()
}

/// Whether [`message_log_from_entries`] hands this entry to its `render`
/// callback (the branches it takes before consulting it).
fn reaches_render(entry: &MessageEntry) -> bool {
    !entry.bare_marker
        && !entry.explicit_blank
        && !(entry.prompt_column_row && entry.text.trim().is_empty())
}

/// The window's log for `entries`, as the shell projects it, and for each
/// logged line whether a command echo wrote it.
///
/// `unflushed` is the line still being composed, already rendered, which
/// the shell appends as output (`displayed_unflushed_message`). `render` is
/// called once for each entry that reaches it, in order, exactly as
/// [`message_log_from_entries`] would call it.
///
/// `None` when the masked projection does not have the real one's shape,
/// which would mean the projection took a branch this mask does not
/// mirror; callers fall back to the rows' own `prefixed` flag.
pub fn tagged_log(
    entries: &[MessageEntry],
    unflushed: Option<&str>,
    mut render: impl FnMut(&str) -> Option<String>,
) -> Option<(Vec<MessageLogLine>, Vec<bool>)> {
    let mut real_outputs = VecDeque::new();
    let mut masked_outputs = VecDeque::new();
    let mut masked = Vec::with_capacity(entries.len());
    for entry in entries {
        let mark = if entry.is_command_echo {
            TYPED_MARK
        } else {
            GAME_MARK
        };
        let mut copy = entry.clone();
        copy.text = mask_text(&entry.text, mark);
        copy.glyphs = mask_glyphs(&entry.glyphs, mark);
        if reaches_render(entry) {
            let output = render(&entry.text);
            // Unchanged text keeps the font-preserving (or centred) branch,
            // which compares the output with the entry's own text; changed
            // text takes the plain branch. The masked copy must take the
            // same one.
            masked_outputs.push_back(output.as_ref().map(|text| {
                if *text == entry.text {
                    copy.text.clone()
                } else {
                    mask_text(text, mark)
                }
            }));
            real_outputs.push_back(output);
        }
        masked.push(copy);
    }
    let mut real_calls = 0usize;
    let mut real = message_log_from_entries(entries, |_| {
        real_calls += 1;
        real_outputs.pop_front().flatten()
    });
    let mut masked_calls = 0usize;
    let mut mask = message_log_from_entries(&masked, |_| {
        masked_calls += 1;
        masked_outputs.pop_front().flatten()
    });
    if let Some(text) = unflushed {
        real.push_output(text);
        mask.push_output(&mask_text(text, GAME_MARK));
    }
    let same_shape = real_calls == masked_calls
        && real.lines().len() == mask.lines().len()
        && real.lines().iter().zip(mask.lines()).all(|(line, masked)| {
            line.kind == masked.kind
                && line.glyphs.len() == masked.glyphs.len()
                && line
                    .glyphs
                    .iter()
                    .zip(&masked.glyphs)
                    .all(|(a, b)| (a.byte == b' ') == (b.byte == b' '))
        });
    if !same_shape {
        return None;
    }
    let typed = mask
        .lines()
        .iter()
        .map(|line| line.glyphs.iter().any(|glyph| glyph.byte == TYPED_MARK))
        .collect();
    Some((real.lines().to_vec(), typed))
}

/// For each placed row, whether a command echo wrote it, wrapped
/// continuations included (2.3.8).
///
/// `lines` and `typed` come from [`tagged_log`] over the same entries the
/// faithful frame drew. `live_row_placed` is whether the layout added its
/// own live row under the log (the shell's live row was not suppressed);
/// that row is the bottom row and no line wrote it.
///
/// The other rows are matched to the lines from the bottom up, as the
/// layout places the log's tail: blank lines are skipped (the layout places
/// no row for them), an empty line the layout dropped (a trailing marker
/// row under a live row) is skipped, and the bottom row may be an open
/// prompt's row with the typed input appended. Any other disagreement means
/// the rows did not come from these lines, and every row then falls back
/// to its own `prefixed` flag, which is right for every unwrapped echo.
pub fn echo_row_flags(
    rows: &[MessageWindowRow],
    lines: &[MessageLogLine],
    typed: &[bool],
    live_row_placed: bool,
) -> Vec<bool> {
    let fallback = || rows.iter().map(|row| row.prefixed).collect();
    if lines.len() != typed.len() {
        return fallback();
    }
    let mut flags = vec![false; rows.len()];
    let mut line_index = lines.len();
    let mut row_index = rows.len();
    if live_row_placed {
        row_index = row_index.saturating_sub(1);
    }
    let open_row = row_index;
    while row_index > 0 {
        let row = &rows[row_index - 1];
        // The next line that places a row.
        while line_index > 0
            && matches!(
                lines[line_index - 1].kind,
                MessageLineKind::Blank | MessageLineKind::ProducerBlank
            )
        {
            line_index -= 1;
        }
        let Some(line) = line_index.checked_sub(1).map(|index| &lines[index]) else {
            return fallback();
        };
        let open_prompt_row = row_index == open_row && !live_row_placed;
        if row.text == line.text || (open_prompt_row && row.text.starts_with(&line.text)) {
            flags[row_index - 1] = typed[line_index - 1];
            row_index -= 1;
            line_index -= 1;
        } else if line.text.is_empty() {
            line_index -= 1;
        } else {
            return fallback();
        }
    }
    flags
}

/// One older row for the open window (2.2.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryRow {
    pub text: String,
    /// Font-preserving cells aligned with `text`; runic text stays runic.
    pub glyphs: Vec<TlkRenderedGlyph>,
    /// Whether the original drew the end cap on it. The crawl and the
    /// open window draw no end cap on a committed row (2.2.4); kept for
    /// the column's crop checks.
    pub prefixed: bool,
    /// [`RowClass::Game`], [`RowClass::Typed`] or [`RowClass::Mixed`];
    /// never `Prompt` or `Live`.
    pub class: RowClass,
}

/// The lines `entries` project to, with their echo flags, cut into pieces
/// that each stay under the log's own row cap.
///
/// [`message_log_from_entries`] keeps only the window's thirteen rows. The
/// history needs more, so the entries are projected in runs that each
/// produce fewer lines than that, newest run first, until `want` lines
/// that place a row have been collected or the entries run out. A run
/// never starts directly after a bare marker entry, whose next output is
/// written on the marker's own row. The only thing a cut changes is the
/// derived blank between turns, which the history does not draw. A single
/// entry that wraps past the cap keeps only its last rows, which still
/// fill the window.
fn projected_lines(
    entries: &[MessageEntry],
    want: usize,
    mut render: impl FnMut(&str) -> Option<String>,
) -> Vec<(MessageLogLine, bool)> {
    const CAP: usize = u5_runtime::MESSAGE_WINDOW_ROWS;
    fn project(
        run: &[MessageEntry],
        render: &mut dyn FnMut(&str) -> Option<String>,
    ) -> (Vec<MessageLogLine>, Vec<bool>) {
        if let Some(tagged) = tagged_log(run, None, &mut *render) {
            return tagged;
        }
        let log = message_log_from_entries(run, &mut *render);
        let lines = log.lines().to_vec();
        let typed = lines.iter().map(|line| line.kind.prefixed()).collect();
        (lines, typed)
    }
    let mut runs: Vec<Vec<(MessageLogLine, bool)>> = Vec::new();
    let mut collected = 0usize;
    let mut end = entries.len();
    while end > 0 && collected < want {
        let mut start = end - 1;
        while start > 0 && project(&entries[start - 1..end], &mut render).0.len() < CAP {
            start -= 1;
        }
        while start > 0 && start + 1 < end && entries[start - 1].bare_marker {
            start += 1;
        }
        let (lines, typed) = project(&entries[start..end], &mut render);
        collected += lines
            .iter()
            .filter(|line| {
                !matches!(
                    line.kind,
                    MessageLineKind::Blank | MessageLineKind::ProducerBlank
                )
            })
            .count();
        runs.push(lines.into_iter().zip(typed).collect());
        end = start;
    }
    runs.into_iter().rev().flatten().collect()
}

/// Project `entries` to the open window's older rows, oldest first: blank
/// rows, empty rows and prompt rows left out, the newest `max_rows` kept
/// (2.2.5, 2.3.8).
///
/// `render` is the shell's per-line renderer (its `keep` filter and name
/// substitution). A prompt row in history is never live, so it is always
/// left out; a committed `:word` row keeps its class
/// ([`RowClass::Mixed`]), and the window drops its leading `:`.
pub fn project_history_rows(
    entries: &[MessageEntry],
    submitted: &TypedLog,
    max_rows: usize,
    render: impl FnMut(&str) -> Option<String>,
) -> Vec<HistoryRow> {
    if max_rows == 0 {
        return Vec::new();
    }
    // Rows the window leaves out are found only after projecting, so ask
    // for enough to cover them: at most every other row is a prompt row.
    let lines = projected_lines(entries, max_rows.saturating_mul(2), render);
    // Number every line so adjacency survives the blanks the layout skips,
    // then classify the rows the window would place.
    let mut rows = Vec::new();
    let mut echo = Vec::new();
    for (number, (line, typed)) in lines.iter().enumerate() {
        if matches!(
            line.kind,
            MessageLineKind::Blank | MessageLineKind::ProducerBlank
        ) {
            continue;
        }
        let prefixed = line.kind.prefixed();
        rows.push(MessageWindowRow {
            row: (number % 256) as u8,
            column: MESSAGE_WINDOW_LEFT + u8::from(prefixed),
            text: line.text.clone(),
            glyphs: line.glyphs.clone(),
            prefixed,
            trailing_spaces: line.trailing_spaces,
        });
        echo.push(*typed);
    }
    let classes = classify_rows(&rows, submitted, &echo, false);
    let mut out: Vec<HistoryRow> = rows
        .into_iter()
        .zip(classes)
        .filter(|(row, class)| *class != RowClass::Prompt && !row.glyphs.is_empty())
        .map(|(row, class)| HistoryRow {
            text: row.text,
            glyphs: row.glyphs,
            prefixed: row.prefixed,
            class,
        })
        .collect();
    let excess = out.len().saturating_sub(max_rows);
    out.drain(..excess);
    out
}

/// Copies of the transcript entries the transcript has dropped, for the
/// open window's older rows (2.2.5, 2.3.8).
///
/// The original's window keeps no history (a scroll loses the top row's
/// pixels for good), and the transcript holds at most
/// [`MESSAGE_TRANSCRIPT_CAPACITY`] entries. This buffer is session only
/// (D2.9): not saved, so a restart or a load begins with what the
/// transcript holds.
///
/// The design names the stored type `HistoryEntry`; it is the transcript's
/// own [`MessageEntry`], kept whole so the runtime's projection can be
/// run over it unchanged.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageHistory {
    /// Entries the transcript no longer holds, oldest first.
    committed: VecDeque<MessageEntry>,
    /// Mirror of `message_entries()` at the last sync.
    live: Vec<MessageEntry>,
}

impl MessageHistory {
    /// An empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Entries the transcript has dropped, oldest first.
    pub fn committed(&self) -> &VecDeque<MessageEntry> {
        &self.committed
    }

    /// The transcript as it stood at the last sync.
    pub fn live(&self) -> &[MessageEntry] {
        &self.live
    }

    /// Bring the history up to date with the transcript `now`.
    ///
    /// Pure; run on every frame whose faithful frame changed. The
    /// transcript can change without a push and its revision counter can
    /// go down (`abort_command_echo`, the bare marker withdrawal) or be
    /// copied across states (a scene rebuild, a location entry), so the
    /// slices are compared instead:
    ///
    /// 1. Find the fewest entries `d` dropped from the front of the last
    ///    mirror such that what remains is still at the front of `now`,
    ///    allowing the last remaining entry to differ (an in-place edit or
    ///    an open row being completed) or one remaining entry to be gone
    ///    (the newest echo taken back, a bare marker withdrawn). A front drop is
    ///    accepted only with at least one entry left exactly equal.
    /// 2. Those `d` entries have left the transcript: commit them.
    /// 3. With no such overlap (a clear, `clear_message_window`, or a
    ///    combat arena's own transcript), commit the whole mirror.
    /// 4. Mirror `now` and trim the committed entries to what the tallest
    ///    open window can show ([`HISTORY_KEPT_ENTRIES`]).
    ///
    /// Entries a withdrawal or an edit replaced are not committed: the
    /// window never kept them either.
    pub fn sync(&mut self, now: &[MessageEntry]) {
        if self.live.as_slice() == now {
            return;
        }
        let dropped = front_drop(&self.live, now).unwrap_or(self.live.len());
        self.committed.extend(self.live.drain(..dropped));
        self.live.clear();
        self.live.extend_from_slice(now);
        self.trim();
    }

    /// Drop committed entries the tallest window can no longer reach.
    fn trim(&mut self) {
        let mut showing = 0usize;
        let mut keep = 0usize;
        for entry in self.committed.iter().rev() {
            if showing == HISTORY_KEPT_ENTRIES {
                break;
            }
            keep += 1;
            if shows_a_row(entry) {
                showing += 1;
            }
        }
        let excess = self.committed.len() - keep;
        self.committed.drain(..excess);
    }

    /// The open window's older rows, oldest first, at most `max_rows`:
    /// the committed entries projected with [`project_history_rows`].
    pub fn older_rows(
        &self,
        submitted: &TypedLog,
        max_rows: usize,
        render: impl FnMut(&str) -> Option<String>,
    ) -> Vec<HistoryRow> {
        let entries: Vec<MessageEntry> = self.committed.iter().cloned().collect();
        project_history_rows(&entries, submitted, max_rows, render)
    }
}

/// Whether an entry projects to at least one row with text on it.
fn shows_a_row(entry: &MessageEntry) -> bool {
    !entry.explicit_blank && !entry.bare_marker && !entry.text.trim().is_empty()
}

/// Step 1 of [`MessageHistory::sync`]: how many entries left the front of
/// `live`, or `None` when `now` keeps none of them.
fn front_drop(live: &[MessageEntry], now: &[MessageEntry]) -> Option<usize> {
    if live.is_empty() {
        return Some(0);
    }
    (0..live.len()).find(|&dropped| {
        let rest = &live[dropped..];
        let kept = rest
            .iter()
            .zip(now)
            .take_while(|(was, is)| was == is)
            .count();
        if dropped > 0 && kept == 0 {
            return false;
        }
        // Everything kept, perhaps with more appended.
        kept == rest.len()
            // The last entry edited in place.
            || (kept + 1 == rest.len() && now.len() > kept)
            // One entry withdrawn, and what followed it still in order. The
            // runtime takes back only its newest echo (`abort_command_echo`)
            // or a bare marker row (`withdraw_overworld_bare_marker_entry`),
            // and a lone entry that vanished is a clear, not a withdrawal.
            || (rest.len() >= 2
                && (kept + 1 == rest.len() || rest[kept].bare_marker)
                && now[kept..].starts_with(&rest[kept + 1..]))
    })
}

#[cfg(test)]
#[path = "crawl_tests.rs"]
mod tests;
