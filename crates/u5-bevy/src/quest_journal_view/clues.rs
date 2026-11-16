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
//!
//! **Text (M6-04).** With `clue_text` not `off`, Enter on a clue opens its
//! text: the rows as the window drew them, 16 cells wide in the game's own
//! fixed-cell fonts (runic spans in `RUNES.CH`), or the replay line.
//!
//! **Notes (M6-04).** With `journal.notes`, `N` writes a note on the
//! selected clue or person, and the Notes tab lists the save's own note and
//! every other. The text is typed into the overlay's field from
//! [`OverlayKey`](crate::enh::input::OverlayKey) events, never the game's.
//!
//! **Evidence links (M6-07).** With `journal.quest_tracker`, `Q` on a clue
//! opens the pin picker: the cards the tier shows (and the shrines and
//! passwords, which a pin is how a card comes to exist below tier 3), each
//! with the Knowledge steps it names; Enter pins or unpins (LINKS kinds 0
//! and 1). Enter on a card's step opens this tab with the Quest filter,
//! which keeps the clues pinned there and, from tier 3, the ones the spec
//! links; Esc clears it.

use bevy::prelude::KeyCode;
use u5_runtime::enhancements::ClueText;
use u5_runtime::enhancements::clues::{
    ClueLog, EntryFlags, NOTE_MAX_LEN, REPLAY_LINE, TextBody, TextRow,
};
use u5_runtime::quest_journal::clues::{
    ClueFilter, ClueQuery, ClueRow, ClueView, CluesTab, CluesView, GroupKey, MAX_PANEL_PINS,
    MAX_SEARCH_LEN, NOTE_PREFIX, RowKind, RowMark, unsaved_line,
};
use u5_runtime::quest_journal::text::{PIN_LINKED, PIN_TO_QUEST, QUEST_FILTER};
use u5_runtime::quest_journal::trackers::{LinkTarget, PinTarget};

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
pub const KEY_HELP_ENTRY_NOTES: &str = "S star  H hide  P pin  N note";
pub const KEY_HELP_NOTES: &str = "N write  / find  Esc close";
pub const KEY_HELP_NOTE: &str = "Enter keep  Esc cancel";
pub const KEY_HELP_READING: &str = "\u{18}\u{19} scroll  Esc back";
pub const KEY_HELP_READING_NOTES: &str = "\u{18}\u{19} scroll  N note  Esc back";
/// With `journal.quest_tracker` (M6-07).
pub const KEY_HELP_ENTRY_QUEST: &str = "S star  H hide  P pin  Q quest";
pub const KEY_HELP_ENTRY_QUEST_NOTES: &str = "S star  P pin  Q quest  N note";
pub const KEY_HELP_PICKER: &str = "Enter pin  Esc back";
pub const KEY_HELP_QUEST_EMPTY: &str = "Esc all clues";

/// The text pane: rows of the clue's text, 16 cells, centred in the body
/// below the two summary rows.
pub const TEXT_PANE_FIRST_ROW: u8 = super::ROWS_FIRST_ROW + 1;
pub const TEXT_PANE_ROWS: usize = (BODY_LAST_ROW - TEXT_PANE_FIRST_ROW + 1) as usize;
pub const TEXT_PANE_COLUMN: u8 = TEXT_COLUMN + ((TEXT_CELLS - 16) / 2) as u8;

/// A note being written: what it is on, and the text so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteEdit {
    pub owner_kind: u8,
    pub owner: u16,
    pub text: String,
}

/// The Clues tab's Quest filter: the card or step, and its label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestFilter {
    pub target: LinkTarget,
    pub label: String,
}

/// The pin picker over one clue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picker {
    pub entry: usize,
    pub cursor: usize,
    pub scroll: usize,
    /// The targets, derived by the page's system from the tracker view.
    pub rows: Vec<PinTarget>,
}

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
    /// `journal.clue_text` in force: Enter opens a clue's text unless off.
    pub text: ClueText,
    /// `journal.notes` is on: `N` writes notes.
    pub notes: bool,
    /// The clue whose text is open, and how far it is scrolled.
    pub reading: Option<usize>,
    pub reading_scroll: usize,
    /// The open clue's text, derived by the page's system.
    pub body: Option<TextBody>,
    /// The note being written; keys go to it.
    pub note: Option<NoteEdit>,
    /// `journal.quest_tracker` is on: `Q` pins a clue to a quest.
    pub quests: bool,
    /// The Quest filter (Enter on a card's step), and the entries it
    /// keeps, derived by the page's system.
    pub quest: Option<QuestFilter>,
    pub only: Vec<usize>,
    /// The pin picker; keys go to it.
    pub picking: Option<Picker>,
}

impl ClueUi {
    pub fn query(&self, tab: CluesTab) -> ClueQuery<'_> {
        ClueQuery {
            tab,
            view: self.view,
            filter: self.filter,
            search: &self.search,
            open: &self.open,
            only: self.quest.as_ref().map(|_| self.only.as_slice()),
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
        self.reading = None;
        self.reading_scroll = 0;
        self.body = None;
        self.note = None;
        self.quest = None;
        self.only.clear();
        self.picking = None;
    }

    /// The page closed: the search goes (never saved), and so does an
    /// open text or a note not kept.
    pub fn close(&mut self) {
        self.search.clear();
        self.searching = false;
        self.reading = None;
        self.body = None;
        self.note = None;
        self.quest = None;
        self.picking = None;
    }

    /// Whether Esc belongs to the tab (a search, a note, an open text)
    /// rather than closing the page.
    pub fn takes_escape(&self) -> bool {
        self.searching
            || self.note.is_some()
            || self.reading.is_some()
            || self.quest.is_some()
            || self.picking.is_some()
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
        if self.note.is_some() {
            return self.note_key(key, shift, log);
        }
        if self.reading.is_some() {
            return self.reading_key(key, log);
        }
        if self.picking.is_some() {
            return self.picker_key(key, log);
        }
        match key {
            KeyCode::Escape if self.quest.is_some() => {
                self.quest = None;
                self.cursor = 0;
                self.scroll = 0;
            }
            KeyCode::KeyQ if self.quests && tab != CluesTab::Notes => {
                if let Some(entry) = self.selected().and_then(ClueRow::entry) {
                    self.picking = Some(Picker {
                        entry,
                        ..Picker::default()
                    });
                }
            }
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
            KeyCode::KeyN if self.notes => self.start_note(log.as_deref()),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => self.open_selected(log),
            KeyCode::KeyS => return self.flag_selected(log, EntryFlags::STAR),
            KeyCode::KeyH => return self.flag_selected(log, EntryFlags::HIDDEN),
            KeyCode::KeyP => return self.flag_selected(log, EntryFlags::PANEL),
            _ => return KeyOutcome::Unused,
        }
        KeyOutcome::Used
    }

    /// The pin picker: Up and Down pick, Enter pins or unpins, Esc goes
    /// back to the clues.
    fn picker_key(&mut self, key: KeyCode, log: Option<&mut ClueLog>) -> KeyOutcome {
        let Some(picker) = self.picking.as_mut() else {
            return KeyOutcome::Unused;
        };
        let rows = picker.rows.len();
        let mut step = |by: isize| {
            picker.cursor = picker
                .cursor
                .saturating_add_signed(by)
                .min(rows.saturating_sub(1));
            if picker.cursor < picker.scroll {
                picker.scroll = picker.cursor;
            } else if picker.cursor >= picker.scroll + CLUE_VISIBLE_ROWS {
                picker.scroll = picker.cursor + 1 - CLUE_VISIBLE_ROWS;
            }
        };
        match key {
            KeyCode::ArrowUp => step(-1),
            KeyCode::ArrowDown => step(1),
            KeyCode::PageUp => step(-(CLUE_VISIBLE_ROWS as isize)),
            KeyCode::PageDown => step(CLUE_VISIBLE_ROWS as isize),
            KeyCode::Home => step(isize::MIN / 2),
            KeyCode::End => step(isize::MAX / 2),
            KeyCode::Escape | KeyCode::Backspace => self.picking = None,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let (Some(row), Some(log), Ok(entry)) = (
                    picker.rows.get(picker.cursor),
                    log,
                    u16::try_from(picker.entry),
                ) {
                    let link = row.target.link(entry);
                    if row.linked {
                        log.remove_link(link);
                    } else {
                        log.add_link(link);
                    }
                }
            }
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

    /// `N`: a note on the selected row's clue, person or (on the Notes
    /// tab) owner, starting from the note there.
    fn start_note(&mut self, log: Option<&ClueLog>) {
        let owner = match self.reading {
            Some(index) => u16::try_from(index)
                .ok()
                .map(|index| (u5_runtime::enhancements::clues::note_owner::CLUE, index)),
            None => self.selected().and_then(ClueRow::note_owner),
        };
        let Some((owner_kind, owner)) = owner else {
            return;
        };
        let text = log
            .and_then(|log| log.note_for(owner_kind, owner))
            .map(|note| note.text.clone())
            .unwrap_or_default();
        self.note = Some(NoteEdit {
            owner_kind,
            owner,
            text,
        });
    }

    fn note_key(&mut self, key: KeyCode, shift: bool, log: Option<&mut ClueLog>) -> KeyOutcome {
        let Some(edit) = self.note.as_mut() else {
            return KeyOutcome::Unused;
        };
        match key {
            KeyCode::Escape => self.note = None,
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let (Some(edit), Some(log)) = (self.note.take(), log) {
                    log.set_note(edit.owner_kind, edit.owner, &edit.text);
                }
            }
            KeyCode::Backspace => {
                edit.text.pop();
            }
            key => match note_char(key, shift) {
                Some(ch) if edit.text.len() < NOTE_MAX_LEN => edit.text.push(ch),
                Some(_) => {}
                None => return KeyOutcome::Unused,
            },
        }
        KeyOutcome::Used
    }

    /// Keys while a clue's text is open: scroll, a note, back.
    fn reading_key(&mut self, key: KeyCode, log: Option<&mut ClueLog>) -> KeyOutcome {
        let rows = match &self.body {
            Some(TextBody::Rows(rows)) => rows.len(),
            _ => 0,
        };
        let last = rows.saturating_sub(TEXT_PANE_ROWS);
        match key {
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Backspace => {
                self.reading = None;
                self.reading_scroll = 0;
                self.body = None;
            }
            KeyCode::ArrowUp => self.reading_scroll = self.reading_scroll.saturating_sub(1),
            KeyCode::ArrowDown => self.reading_scroll = (self.reading_scroll + 1).min(last),
            KeyCode::PageUp => {
                self.reading_scroll = self.reading_scroll.saturating_sub(TEXT_PANE_ROWS);
            }
            KeyCode::PageDown => {
                self.reading_scroll = (self.reading_scroll + TEXT_PANE_ROWS).min(last);
            }
            KeyCode::KeyN if self.notes => self.start_note(log.as_deref()),
            _ => return KeyOutcome::Unused,
        }
        KeyOutcome::Used
    }

    /// Enter: a group opens or closes; opening marks its entries read. An
    /// entry is marked read, and its text opens unless `clue_text` is off.
    /// On the Notes tab a row's note is written.
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
                if self.text != ClueText::Off {
                    self.reading = Some(index);
                    self.reading_scroll = 0;
                    self.body = None;
                }
            }
            RowKind::Noted { .. } if self.notes => self.start_note(log.as_deref()),
            RowKind::Person(_) | RowKind::Note | RowKind::Noted { .. } => {}
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

/// The characters a note takes: letters (shift for capitals), digits, the
/// space and a little punctuation, all in IBM.CH's ASCII range.
pub fn note_char(key: KeyCode, shift: bool) -> Option<char> {
    let punctuation = match (key, shift) {
        (KeyCode::Period, false) => Some('.'),
        (KeyCode::Comma, false) => Some(','),
        (KeyCode::Minus, false) => Some('-'),
        (KeyCode::Quote, false) => Some('\''),
        (KeyCode::Slash, true) => Some('?'),
        (KeyCode::Digit1, true) => Some('!'),
        (KeyCode::Semicolon, true) => Some(':'),
        (KeyCode::Semicolon, false) => Some(';'),
        _ => None,
    };
    punctuation.or_else(|| {
        search_char(key).map(|ch| {
            if ch.is_ascii_alphabetic() && !shift {
                ch.to_ascii_lowercase()
            } else {
                ch
            }
        })
    })
}

/// The key help of a clue tab.
pub fn clue_key_help(ui: &ClueUi, tab: CluesTab) -> &'static str {
    if ui.searching {
        return KEY_HELP_SEARCH;
    }
    if ui.note.is_some() {
        return KEY_HELP_NOTE;
    }
    if ui.reading.is_some() {
        return if ui.notes {
            KEY_HELP_READING_NOTES
        } else {
            KEY_HELP_READING
        };
    }
    if ui.picking.is_some() {
        return KEY_HELP_PICKER;
    }
    if tab == CluesTab::Notes {
        return KEY_HELP_NOTES;
    }
    match ui.selected().map(|row| &row.kind) {
        Some(RowKind::Entry(_)) if ui.quests && ui.notes => KEY_HELP_ENTRY_QUEST_NOTES,
        Some(RowKind::Entry(_)) if ui.quests => KEY_HELP_ENTRY_QUEST,
        Some(RowKind::Entry(_)) if ui.notes => KEY_HELP_ENTRY_NOTES,
        Some(RowKind::Entry(_)) => KEY_HELP_ENTRY,
        Some(RowKind::Group { .. }) | Some(RowKind::Person(_)) if tab == CluesTab::Clues => {
            KEY_HELP_GROUP
        }
        Some(RowKind::Group { .. }) | Some(RowKind::Person(_)) => KEY_HELP_TOPICS_GROUP,
        _ if ui.quest.is_some() => KEY_HELP_QUEST_EMPTY,
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
    if ui.reading.is_some() {
        return reading_cells(ui);
    }
    if let Some(picker) = &ui.picking {
        return picker_cells(ui, picker);
    }
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
    } else if tab == CluesTab::Notes {
        // Notes have no filter or view: the row stays empty.
    } else if let Some(quest) = ui.quest.as_ref().filter(|_| tab == CluesTab::Clues) {
        // The Quest filter replaces the filter and the view.
        cells.push(Cell::new(TEXT_COLUMN, HEADER_ROW, QUEST_FILTER, BODY));
        let column = TEXT_COLUMN + QUEST_FILTER.len() as u8 + 1;
        cells.push(Cell::new(
            column,
            HEADER_ROW,
            cut(&quest.label, usize::from(LAST_TEXT_COLUMN + 1 - column)),
            WHITE,
        ));
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
    if let Some(edit) = &ui.note {
        cells.extend(note_field_cells(edit));
    } else if let Some(row) = ui.selected() {
        // A note takes the last detail row; the rest runs on over the first.
        let (detail, note) = match row.detail.last() {
            Some(last) if last.starts_with(NOTE_PREFIX) && row.detail.len() > 1 => {
                (&row.detail[..row.detail.len() - 1], Some(last))
            }
            _ => (row.detail.as_slice(), None),
        };
        let lines: Vec<String> = match (detail, note) {
            (_, Some(note)) => {
                let mut lines = wrap(&detail.join("  "), TEXT_CELLS, 1);
                lines.truncate(1);
                lines.push(cut(note, TEXT_CELLS));
                lines
            }
            (detail, None) => match detail {
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
            },
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

/// The pin picker: its title over the clue's topic, then the targets,
/// a step indented under its card, `pinned` on the right of a target the
/// clue is pinned to already.
fn picker_cells(ui: &ClueUi, picker: &Picker) -> Vec<Cell> {
    let mut cells = vec![Cell::new(TEXT_COLUMN, HEADER_ROW, PIN_TO_QUEST, WHITE)];
    if let Some(row) = ui
        .rows()
        .iter()
        .find(|row| row.entry() == Some(picker.entry))
    {
        cells.push(Cell::new(
            TEXT_COLUMN,
            SECTION_ROW,
            cut(&row.label, TEXT_CELLS - 3),
            BODY,
        ));
    }
    if picker.rows.len() > CLUE_VISIBLE_ROWS {
        let more_above = picker.scroll > 0;
        let more_below = picker.scroll + CLUE_VISIBLE_ROWS < picker.rows.len();
        for (column, glyph, more) in [
            (LAST_TEXT_COLUMN - 1, GLYPH_UP, more_above),
            (LAST_TEXT_COLUMN, GLYPH_DOWN, more_below),
        ] {
            let ink = if more { BROWN } else { DARK_GREY };
            cells.push(Cell::new(column, SECTION_ROW, glyph, ink));
        }
    }
    let mark_column = LAST_TEXT_COLUMN + 1 - PIN_LINKED.len() as u8;
    for (offset, row) in picker
        .rows
        .iter()
        .enumerate()
        .skip(picker.scroll)
        .take(CLUE_VISIBLE_ROWS)
    {
        let y = CLUE_ROWS_FIRST_ROW + (offset - picker.scroll) as u8;
        let selected = offset == picker.cursor;
        if selected {
            cells.push(Cell::new(TEXT_COLUMN, y, " ".repeat(TEXT_CELLS), WHITE).on(Paper::Bar));
        }
        let column = TEXT_COLUMN + 2 * row.depth;
        let room = usize::from(mark_column - 1 - column);
        let ink = if selected { WHITE } else { BODY };
        cells.push(Cell::new(column, y, cut(row.label.as_str(), room), ink));
        if row.linked {
            cells.push(Cell::new(mark_column, y, PIN_LINKED, BROWN));
        }
    }
    cells
}

/// The note being written, over the two detail rows: `Note: ` in brown,
/// the text in white and the brown cursor after it.
fn note_field_cells(edit: &NoteEdit) -> Vec<Cell> {
    let mut cells = vec![Cell::new(TEXT_COLUMN, DETAIL_FIRST_ROW, NOTE_PREFIX, BROWN)];
    let first = TEXT_CELLS - NOTE_PREFIX.len();
    let text: Vec<char> = edit.text.chars().collect();
    let (head, tail) = text.split_at(text.len().min(first));
    let head: String = head.iter().collect();
    let tail: String = tail.iter().take(TEXT_CELLS).collect();
    let column = TEXT_COLUMN + NOTE_PREFIX.len() as u8;
    cells.push(Cell::new(column, DETAIL_FIRST_ROW, head.clone(), WHITE));
    let (cursor_column, cursor_row) = if tail.is_empty() && head.chars().count() < first {
        (column + head.chars().count() as u8, DETAIL_FIRST_ROW)
    } else {
        cells.push(Cell::new(
            TEXT_COLUMN,
            DETAIL_FIRST_ROW + 1,
            tail.clone(),
            WHITE,
        ));
        (
            TEXT_COLUMN + tail.chars().count() as u8,
            DETAIL_FIRST_ROW + 1,
        )
    };
    if cursor_column <= LAST_TEXT_COLUMN {
        cells.push(Cell::new(cursor_column, cursor_row, "_", BROWN));
    }
    cells
}

/// A clue's text: the summary and when on the header and section rows,
/// the rows 16 cells wide centred in the body (runic cells in `RUNES.CH`),
/// scroll marks, and the note on the detail rows.
fn reading_cells(ui: &ClueUi) -> Vec<Cell> {
    let mut cells = Vec::new();
    let row = ui
        .rows()
        .iter()
        .find(|row| row.entry().is_some() && row.entry() == ui.reading);
    if let Some(row) = row {
        // The header row: when, how often, the marks; clear of the scroll
        // marks.
        if let Some(facts) = row.detail.get(1) {
            let line = format!("{}  {}", row.right, facts);
            cells.push(Cell::new(
                TEXT_COLUMN,
                HEADER_ROW,
                cut(line.trim(), TEXT_CELLS - 3),
                BODY,
            ));
        }
        // The summary on the two rows above the text.
        if let Some(summary) = row.detail.first() {
            for (index, line) in wrap(summary, TEXT_CELLS, 2).into_iter().enumerate() {
                cells.push(Cell::new(
                    TEXT_COLUMN,
                    SECTION_ROW + index as u8,
                    line,
                    WHITE,
                ));
            }
        }
    }
    match &ui.body {
        Some(TextBody::Rows(rows)) => {
            if rows.len() > TEXT_PANE_ROWS {
                let more_above = ui.reading_scroll > 0;
                let more_below = ui.reading_scroll + TEXT_PANE_ROWS < rows.len();
                for (column, glyph, more) in [
                    (LAST_TEXT_COLUMN - 1, GLYPH_UP, more_above),
                    (LAST_TEXT_COLUMN, GLYPH_DOWN, more_below),
                ] {
                    let ink = if more { BROWN } else { DARK_GREY };
                    cells.push(Cell::new(column, HEADER_ROW, glyph, ink));
                }
            }
            for (offset, text_row) in rows
                .iter()
                .skip(ui.reading_scroll)
                .take(TEXT_PANE_ROWS)
                .enumerate()
            {
                cells.extend(text_row_cells(
                    text_row,
                    TEXT_PANE_COLUMN,
                    TEXT_PANE_FIRST_ROW + offset as u8,
                ));
            }
        }
        Some(TextBody::Replay) => {
            cells.push(Cell::new(
                TEXT_COLUMN,
                TEXT_PANE_FIRST_ROW,
                cut(REPLAY_LINE, TEXT_CELLS),
                BODY,
            ));
        }
        Some(TextBody::Off) | None => {}
    }
    if let Some(edit) = &ui.note {
        cells.extend(note_field_cells(edit));
    } else if let Some(note) =
        row.and_then(|row| row.detail.iter().find(|line| line.starts_with(NOTE_PREFIX)))
    {
        for (index, line) in wrap(note, TEXT_CELLS, 2).into_iter().enumerate() {
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

/// One row of a clue's text: runs of one font each, in the window's light
/// grey. Bytes are the window's own cell codes.
fn text_row_cells(row: &TextRow, column: u8, y: u8) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut run = String::new();
    let mut run_start = 0usize;
    let mut run_runic = false;
    for (index, byte) in row.bytes.iter().take(16).enumerate() {
        let runic = row.runic & (1 << index) != 0;
        if !run.is_empty() && runic != run_runic {
            cells.push(text_run(column + run_start as u8, y, &run, run_runic));
            run.clear();
        }
        if run.is_empty() {
            run_start = index;
            run_runic = runic;
        }
        run.push(char::from(*byte & 0x7f));
    }
    if !run.is_empty() {
        cells.push(text_run(column + run_start as u8, y, &run, run_runic));
    }
    cells
}

fn text_run(column: u8, y: u8, text: &str, runic: bool) -> Cell {
    let mut cell = Cell::new(column, y, text, BODY);
    cell.runic = runic;
    cell
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
    if let RowKind::Noted { .. } = row.kind {
        let ink = if selected { WHITE } else { BODY };
        cells.push(Cell::new(TEXT_COLUMN, y, cut(&row.label, TEXT_CELLS), ink));
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
