//! Chapter 8's Clues and Topics tabs on the journal page (Phase B design
//! 8.2.3-8.2.5; build plan M6-03).
//!
//! The rows come from [`derive_clues`] (pure, in u5-runtime) over the
//! [`ClueLogResource`] and the live `.TLK` cache; this file keeps the tab's
//! own state (view, filter, search, open groups, cursor) and lays the rows
//! out on the page's 40x25 grid in the same cells, inks and bar as the
//! Quests tab.
//!
//! **Search** (`/`) types into [`ClueUi::search`] from the router's
//! [`OverlayKey`](crate::enh::input::OverlayKey) events: the game's
//! `typed_keys` never sees them, and the text is never saved (it is
//! cleared when the page closes).
//!
//! **Marks.** Opening a group marks its entries read; `S` stars, `H`
//! hides, `P` pins to the side panel (at most [`MAX_PANEL_PINS`]). The log
//! never sets a star itself.

use bevy::prelude::KeyCode;
use u5_runtime::enhancements::clues::{ClueLog, EntryFlags};
use u5_runtime::quest_journal::clues::{
    ClueFilter, ClueQuery, ClueRow, ClueView, CluesTab, CluesView, GroupKey, MAX_PANEL_PINS,
    MAX_SEARCH_LEN, RowKind, RowMark, unsaved_line,
};

use super::{
    BODY, BODY_LAST_ROW, BROWN, Cell, DARK_GREY, DETAIL_FIRST_ROW, GLYPH_DOWN, GLYPH_RIGHT,
    GLYPH_UP, HEADER_ROW, LAST_TEXT_COLUMN, Paper, SECTION_ROW, TEXT_CELLS, TEXT_COLUMN, WHITE,
    wrap,
};

/// The first list row: the Quests tab's heading row.
pub const CLUE_ROWS_FIRST_ROW: u8 = super::HEADING_ROW;
/// The rows a clue tab shows at once.
pub const CLUE_VISIBLE_ROWS: usize = (BODY_LAST_ROW - CLUE_ROWS_FIRST_ROW + 1) as usize;

/// An entry row's count ends here and its date takes the cells after.
const COUNT_LAST_COLUMN: u8 = 27;
const ENTRY_LABEL_LAST_COLUMN: u8 = 23;

/// Shown when a sixth pin is refused (8.2.5).
pub const PINS_FULL: &str = "Five clues are pinned: unpin one first.";

/// The key help of a clue tab, by what is selected.
pub const KEY_HELP_GROUP: &str = "Enter open  F filter  V view";
pub const KEY_HELP_TOPICS_GROUP: &str = "Enter open  / find  F filter";
pub const KEY_HELP_ENTRY: &str = "S star  H hide  P pin  / find";
pub const KEY_HELP_SEARCH: &str = "Enter done  Esc clear";
pub const KEY_HELP_EMPTY: &str = "/ find  F filter  Esc close";

/// The clue tabs' state. Session only; nothing here is saved.
#[derive(Clone, Debug, Default)]
pub struct ClueUi {
    pub view: ClueView,
    pub filter: ClueFilter,
    /// The search text; never saved, cleared on close.
    pub search: String,
    /// Keys go to the search field.
    pub searching: bool,
    pub open: Vec<GroupKey>,
    pub cursor: usize,
    pub scroll: usize,
    /// The rows last derived for the tab showing.
    pub last: Option<CluesView>,
}

impl ClueUi {
    pub fn query(&self, tab: CluesTab) -> ClueQuery<'_> {
        ClueQuery {
            tab,
            view: self.view,
            filter: self.filter,
            search: &self.search,
            open: &self.open,
        }
    }

    pub fn rows(&self) -> &[ClueRow] {
        self.last.as_ref().map_or(&[], |view| view.rows.as_slice())
    }

    pub fn selected(&self) -> Option<&ClueRow> {
        self.rows().get(self.cursor)
    }

    /// Takes newly derived rows, keeping the cursor on the same row.
    pub fn take_view(&mut self, view: CluesView) {
        let kind = self.selected().map(|row| row.kind.clone());
        if let Some(at) = kind.and_then(|kind| view.rows.iter().position(|row| row.kind == kind)) {
            self.cursor = at;
        }
        self.last = Some(view);
        self.clamp();
    }

    fn clamp(&mut self) {
        let rows = self.rows().len();
        self.cursor = self.cursor.min(rows.saturating_sub(1));
        self.scroll = self.scroll.min(rows.saturating_sub(CLUE_VISIBLE_ROWS));
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + CLUE_VISIBLE_ROWS {
            self.scroll = self.cursor + 1 - CLUE_VISIBLE_ROWS;
        }
    }

    fn move_cursor(&mut self, step: isize) {
        self.cursor = self.cursor.saturating_add_signed(step);
        self.clamp();
    }

    /// A tab change or a view change: back to the top.
    pub fn reset_position(&mut self) {
        self.cursor = 0;
        self.scroll = 0;
        self.last = None;
    }

    /// The page closed: the search goes (never saved).
    pub fn close(&mut self) {
        self.search.clear();
        self.searching = false;
    }

    /// One key on a clue tab. Returns `false` for a key the tab does not
    /// use. `log` is `None` while the log is off.
    pub fn key(
        &mut self,
        tab: CluesTab,
        key: KeyCode,
        shift: bool,
        log: Option<&mut ClueLog>,
    ) -> KeyOutcome {
        if self.searching {
            return self.search_key(key);
        }
        match key {
            KeyCode::ArrowUp => self.move_cursor(-1),
            KeyCode::ArrowDown => self.move_cursor(1),
            KeyCode::PageUp => self.move_cursor(-(CLUE_VISIBLE_ROWS as isize)),
            KeyCode::PageDown => self.move_cursor(CLUE_VISIBLE_ROWS as isize),
            KeyCode::Home => self.move_cursor(isize::MIN),
            KeyCode::End => self.move_cursor(isize::MAX),
            KeyCode::Slash | KeyCode::NumpadDivide => self.searching = true,
            KeyCode::KeyF => {
                self.filter = self.filter.step(if shift { -1 } else { 1 });
                self.cursor = 0;
                self.scroll = 0;
            }
            KeyCode::KeyV if tab == CluesTab::Clues => {
                self.view = self.view.other();
                self.cursor = 0;
                self.scroll = 0;
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => self.open_selected(log),
            KeyCode::KeyS => return self.flag_selected(log, EntryFlags::STAR),
            KeyCode::KeyH => return self.flag_selected(log, EntryFlags::HIDDEN),
            KeyCode::KeyP => return self.flag_selected(log, EntryFlags::PANEL),
            _ => return KeyOutcome::Unused,
        }
        KeyOutcome::Used
    }

    fn search_key(&mut self, key: KeyCode) -> KeyOutcome {
        match key {
            KeyCode::Escape => {
                self.search.clear();
                self.searching = false;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => self.searching = false,
            KeyCode::Backspace => {
                self.search.pop();
            }
            key => match search_char(key) {
                Some(ch) if self.search.chars().count() < MAX_SEARCH_LEN => self.search.push(ch),
                Some(_) => {}
                None => return KeyOutcome::Unused,
            },
        }
        self.cursor = 0;
        self.scroll = 0;
        KeyOutcome::Used
    }

    /// Enter: a group opens or closes; opening marks its entries read. An
    /// entry is marked read.
    fn open_selected(&mut self, log: Option<&mut ClueLog>) {
        let Some(row) = self.selected().cloned() else {
            return;
        };
        match row.kind {
            RowKind::Group { key, open, entries } => {
                if open {
                    self.open.retain(|k| *k != key);
                } else {
                    self.open.push(key);
                    if let Some(log) = log {
                        for index in entries {
                            log.set_flag(index, EntryFlags::READ, true);
                        }
                    }
                }
            }
            RowKind::Entry(index) => {
                if let Some(log) = log {
                    log.set_flag(index, EntryFlags::READ, true);
                }
            }
            RowKind::Person(_) | RowKind::Note => {}
        }
    }

    /// `S`, `H`, `P` on an entry: toggles the flag. A sixth pin is refused.
    fn flag_selected(&mut self, log: Option<&mut ClueLog>, flag: u8) -> KeyOutcome {
        let (Some(index), Some(log)) = (self.selected().and_then(ClueRow::entry), log) else {
            return KeyOutcome::Used;
        };
        let Some(entry) = log.entries().get(index) else {
            return KeyOutcome::Used;
        };
        let on = !entry.flags.has(flag);
        if flag == EntryFlags::PANEL && on {
            let pinned = log
                .entries()
                .iter()
                .filter(|entry| entry.flags.has(EntryFlags::PANEL))
                .count();
            if pinned >= MAX_PANEL_PINS {
                return KeyOutcome::Refused(PINS_FULL);
            }
        }
        log.set_flag(index, flag, on);
        log.set_flag(index, EntryFlags::READ, true);
        KeyOutcome::Used
    }
}

/// What a key did on a clue tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyOutcome {
    Used,
    Unused,
    /// Refused, with a toast.
    Refused(&'static str),
}

/// The characters search takes: letters (upper-case, as keywords are
/// compared), digits and the space.
fn search_char(key: KeyCode) -> Option<char> {
    use KeyCode::*;
    let letters = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    let digits = [
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    if let Some(at) = letters.iter().position(|k| *k == key) {
        return Some(char::from(b'A' + at as u8));
    }
    if let Some(at) = digits.iter().position(|k| *k == key) {
        return Some(char::from(b'0' + at as u8));
    }
    (key == Space).then_some(' ')
}

/// The key help of a clue tab.
pub fn clue_key_help(ui: &ClueUi, tab: CluesTab) -> &'static str {
    if ui.searching {
        return KEY_HELP_SEARCH;
    }
    match ui.selected().map(|row| &row.kind) {
        Some(RowKind::Entry(_)) => KEY_HELP_ENTRY,
        Some(RowKind::Group { .. }) | Some(RowKind::Person(_)) if tab == CluesTab::Clues => {
            KEY_HELP_GROUP
        }
        Some(RowKind::Group { .. }) | Some(RowKind::Person(_)) => KEY_HELP_TOPICS_GROUP,
        _ => KEY_HELP_EMPTY,
    }
}

/// `text` cut to `cells`.
fn cut(text: &str, cells: usize) -> String {
    text.chars().take(cells).collect()
}

/// `text` right-aligned to end at `last`.
fn right(text: &str, last: u8, y: u8, ink: super::Rgb) -> Cell {
    let width = text
        .chars()
        .count()
        .min(usize::from(last + 1 - TEXT_COLUMN)) as u8;
    Cell::new(last + 1 - width, y, cut(text, usize::from(width)), ink)
}

/// Every text cell of a clue tab: the search or filter line, the unsaved
/// line, the visible rows and the selected row's details.
pub fn clue_cells(ui: &ClueUi, tab: CluesTab) -> Vec<Cell> {
    let mut cells = Vec::new();
    // Row 1: the search field, or the filter and the view.
    if ui.searching || !ui.search.is_empty() {
        cells.push(Cell::new(TEXT_COLUMN, HEADER_ROW, "/", BROWN));
        let shown = cut(&ui.search, MAX_SEARCH_LEN);
        let width = shown.chars().count() as u8;
        cells.push(Cell::new(TEXT_COLUMN + 1, HEADER_ROW, shown, WHITE));
        if ui.searching {
            cells.push(Cell::new(TEXT_COLUMN + 1 + width, HEADER_ROW, "_", BROWN));
        }
        if let Some(view) = &ui.last {
            let matches = match view.matches {
                1 => "1 match".to_string(),
                n => format!("{n} matches"),
            };
            cells.push(right(&matches, LAST_TEXT_COLUMN, HEADER_ROW, BODY));
        }
    } else {
        cells.push(Cell::new(TEXT_COLUMN, HEADER_ROW, "Filter:", BODY));
        cells.push(Cell::new(
            TEXT_COLUMN + 8,
            HEADER_ROW,
            ui.filter.label(),
            WHITE,
        ));
        if tab == CluesTab::Clues {
            let label = ui.view.label();
            let width = label.chars().count() as u8;
            cells.push(Cell::new(
                LAST_TEXT_COLUMN - width - 5,
                HEADER_ROW,
                "View:",
                BODY,
            ));
            cells.push(right(label, LAST_TEXT_COLUMN, HEADER_ROW, WHITE));
        }
    }
    let Some(view) = &ui.last else {
        return cells;
    };
    // Row 3: the clues not yet saved, and the scroll marks.
    if let Some(line) = unsaved_line(view.unsaved) {
        // Clear of the scroll marks.
        cells.push(Cell::new(
            TEXT_COLUMN,
            SECTION_ROW,
            cut(&line, TEXT_CELLS - 3),
            BROWN,
        ));
    }
    if view.rows.len() > CLUE_VISIBLE_ROWS {
        let more_above = ui.scroll > 0;
        let more_below = ui.scroll + CLUE_VISIBLE_ROWS < view.rows.len();
        for (column, glyph, more) in [
            (LAST_TEXT_COLUMN - 1, GLYPH_UP, more_above),
            (LAST_TEXT_COLUMN, GLYPH_DOWN, more_below),
        ] {
            let ink = if more { BROWN } else { DARK_GREY };
            cells.push(Cell::new(column, SECTION_ROW, glyph, ink));
        }
    }
    for (offset, row) in view
        .rows
        .iter()
        .enumerate()
        .skip(ui.scroll)
        .take(CLUE_VISIBLE_ROWS)
    {
        let y = CLUE_ROWS_FIRST_ROW + (offset - ui.scroll) as u8;
        cells.extend(row_cells(row, y, offset == ui.cursor));
    }
    if let Some(row) = ui.selected() {
        let lines: Vec<String> = match row.detail.as_slice() {
            [] => Vec::new(),
            [only] => wrap(only, TEXT_CELLS, 2),
            // The body on its own row when it fits, else body and facts
            // run on over both rows.
            [first, rest @ ..] if first.chars().count() <= TEXT_CELLS => {
                let mut lines = vec![first.clone()];
                lines.extend(wrap(&rest.join("  "), TEXT_CELLS, 1));
                lines
            }
            [first, rest @ ..] => {
                let all = std::iter::once(first.as_str())
                    .chain(rest.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join("  ");
                wrap(&all, TEXT_CELLS, 2)
            }
        };
        for (index, line) in lines.into_iter().enumerate() {
            cells.push(Cell::new(
                TEXT_COLUMN,
                DETAIL_FIRST_ROW + index as u8,
                line,
                BODY,
            ));
        }
    }
    cells
}

/// One row's cells on grid row `y`.
fn row_cells(row: &ClueRow, y: u8, selected: bool) -> Vec<Cell> {
    let mut cells = Vec::new();
    if selected {
        cells.push(Cell::new(TEXT_COLUMN, y, " ".repeat(TEXT_CELLS), WHITE).on(Paper::Bar));
    }
    if row.kind == RowKind::Note {
        cells.push(Cell::new(TEXT_COLUMN, y, cut(&row.label, TEXT_CELLS), BODY));
        return cells;
    }
    let indent = 2 * row.depth;
    let ink = |normal| if selected { WHITE } else { normal };
    if let RowKind::Group { open, .. } = row.kind {
        let glyph = if open { GLYPH_DOWN } else { GLYPH_RIGHT };
        cells.push(Cell::new(TEXT_COLUMN + indent, y, glyph, BROWN));
    }
    match row.mark {
        RowMark::None => {}
        RowMark::Unread => cells.push(Cell::new(
            TEXT_COLUMN + 2 + indent,
            y,
            RowMark::Unread.glyph(),
            ink(BODY),
        )),
        RowMark::Star => cells.push(Cell::new(
            TEXT_COLUMN + 2 + indent,
            y,
            RowMark::Star.glyph(),
            BROWN,
        )),
    }
    let label_column = TEXT_COLUMN + 4 + indent;
    let is_entry = row.entry().is_some();
    let label_last = if is_entry && row.count.is_some() {
        ENTRY_LABEL_LAST_COLUMN
    } else if is_entry {
        // No count: the label may run up to the date.
        COUNT_LAST_COLUMN
    } else {
        let width = row.right.chars().count() as u8;
        LAST_TEXT_COLUMN.saturating_sub(width + 2)
    };
    let room = usize::from(label_last.saturating_sub(label_column) + 1);
    let label_ink = if matches!(row.kind, RowKind::Group { .. }) {
        WHITE
    } else {
        ink(BODY)
    };
    cells.push(Cell::new(label_column, y, cut(&row.label, room), label_ink));
    if let Some(count) = row.count {
        let count = format!("x{}", count.min(99));
        cells.push(right(&count, COUNT_LAST_COLUMN, y, ink(BODY)));
    }
    if !row.right.is_empty() {
        cells.push(right(&row.right, LAST_TEXT_COLUMN, y, ink(BODY)));
    }
    cells
}
