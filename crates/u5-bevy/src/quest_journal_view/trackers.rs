//! Chapter 8's tracker cards on the Quests tab (Phase B design 8.2.7,
//! 8.3.9, 8.4.4; build plan M6-06).
//!
//! **Where.** With `journal.quest_tracker` on, the cards take the place of
//! chapter 5's Virtues section: the section switcher reads `Quests` there
//! and the other sections stay. With `journal.page` off the Quests tab is
//! the cards alone, with no switcher arrows. With `quest_tracker` off
//! [`super::QuestJournalUi`] has no [`TrackerUi`] and the page is chapter
//! 5's, unchanged.
//!
//! **The list.** One row per card: the quest, its state in a fixed 10-cell
//! box (`[Codex read]`) or a plain value (`Shards held  2`), and the short
//! next-step cell; the selected card's next-step lines on the detail rows.
//! With `journal.page` on, only the cards chapter 5's other sections do
//! not already show are listed: the shrines and the passwords.
//!
//! **A card.** Enter opens it: its steps with their ticks, its notes, the
//! tier 4 answer cell and the clue line ("Pinned by you: N", or "Clue log
//! off"). The answer is masked as chapter 5's `[hidden]` and is revealed
//! with R, then R again to confirm (8.2.7). A reveal lasts for the session
//! only: it is kept here, never saved. `journal.mantras = never` blanks
//! the mantra cells.

use u5_runtime::enhancements::{MantraDisplay, SpoilerTier};
use u5_runtime::quest_journal::trackers::{
    QuestKind, StepId, Tick, TrackerActivity, TrackerCard, TrackerView,
};
use u5_runtime::quest_journal::{StatusMark, text};

use bevy::prelude::KeyCode;

use super::{
    BODY, BROWN, Cell, DARK_GREY, DETAIL_FIRST_ROW, GLYPH_DOWN, GLYPH_LEFT, GLYPH_RIGHT, GLYPH_UP,
    HEADER_ROW, HEADING_ROW, LAST_TEXT_COLUMN, PageView, Paper, ROWS_FIRST_ROW, SECTION_ROW,
    TEXT_CELLS, TEXT_COLUMN, VISIBLE_ROWS, WHITE, answer_cell, ega, wrap,
};

/// The list's columns: the quest, the state box and the next cell.
pub const CARD_COLUMN: u8 = TEXT_COLUMN;
pub const STATE_COLUMN: u8 = 15;
pub const NEXT_COLUMN: u8 = 28;
/// The state box's inside: as wide as `Codex read`, the longest state.
pub const STATE_CELLS: usize = 10;

/// The key help in the list and on a card.
pub const KEY_HELP_LIST: &str = "\u{18}\u{19} row  \u{1b}\u{1a} section  Enter card";
pub const KEY_HELP_LIST_ALONE: &str = "\u{18}\u{19} row  Enter card  Esc close";
pub const KEY_HELP_EMPTY: &str = "Esc close";
pub const KEY_HELP_CARD: &str = "Esc back";
pub const KEY_HELP_REVEAL: &str = "R reveal  Esc back";
pub const KEY_HELP_CONFIRM: &str = "R again to reveal  Esc back";

/// The tracker's page state. Present exactly while `quest_tracker` is on.
#[derive(Debug, Default)]
pub struct TrackerUi {
    /// The view shown, derived while the page is open.
    pub view: Option<TrackerView>,
    pub cursor: usize,
    pub scroll: usize,
    /// A card is open.
    pub detail: bool,
    /// R was pressed once on a masked answer.
    pub arming: bool,
    /// The answers revealed this session (8.2.7). Never saved.
    pub revealed: Vec<StepId>,
    /// When each quest's save evidence changed, this session.
    pub activity: TrackerActivity,
    /// Chapter 5's page is on: only the shrine and password cards are
    /// listed, between its other sections.
    pub with_page: bool,
}

/// What a key did on the tracker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackerKey {
    Used,
    /// Not the tracker's: chapter 5's page takes it (its section keys).
    Unused,
}

impl TrackerUi {
    pub fn new(with_page: bool) -> Self {
        Self {
            with_page,
            ..Self::default()
        }
    }

    /// The cards listed: every card, or with chapter 5's page the ones
    /// its other sections do not show.
    pub fn cards(&self) -> Vec<&TrackerCard> {
        self.view.as_ref().map_or_else(Vec::new, |view| {
            view.cards
                .iter()
                .filter(|card| {
                    !self.with_page
                        || matches!(card.quest.kind, QuestKind::Shrine | QuestKind::Password)
                })
                .collect()
        })
    }

    pub fn selected(&self) -> Option<&TrackerCard> {
        self.cards().get(self.cursor).copied()
    }

    /// Takes a newly derived view: the cursor stays on the same card.
    pub fn take_view(&mut self, view: TrackerView) {
        let quest = self.selected().map(|card| card.quest);
        self.view = Some(view);
        if let Some(at) =
            quest.and_then(|quest| self.cards().iter().position(|card| card.quest == quest))
        {
            self.cursor = at;
        }
        self.clamp();
    }

    fn clamp(&mut self) {
        let rows = self.cards().len();
        if rows == 0 {
            self.detail = false;
        }
        self.cursor = self.cursor.min(rows.saturating_sub(1));
        self.scroll = self.scroll.min(rows.saturating_sub(VISIBLE_ROWS));
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + VISIBLE_ROWS {
            self.scroll = self.cursor + 1 - VISIBLE_ROWS;
        }
    }

    fn step(&mut self, step: isize) {
        self.cursor = self.cursor.saturating_add_signed(step);
        self.clamp();
    }

    /// The selected card's answer cell, unless `mantras = never` blanks it.
    pub fn answer(&self, mantras: MantraDisplay) -> Option<(StepId, &'static str, &'static str)> {
        let answer = self.selected()?.answer?;
        if answer.label == text::ANSWER_MANTRA && mantras == MantraDisplay::Never {
            return None;
        }
        Some((answer.step, answer.label, answer.secret))
    }

    /// Whether the open card has an answer still masked.
    pub fn maskable(&self, mantras: MantraDisplay) -> bool {
        self.detail
            && self
                .answer(mantras)
                .is_some_and(|(step, _, _)| !self.revealed.contains(&step))
    }

    /// Whether Esc goes back to the list rather than closing the page.
    pub fn takes_escape(&self) -> bool {
        self.detail
    }

    /// One key on the tracker.
    pub fn key(&mut self, key: KeyCode, mantras: MantraDisplay) -> TrackerKey {
        let arming = std::mem::take(&mut self.arming);
        match key {
            KeyCode::ArrowUp if !self.detail => self.step(-1),
            KeyCode::ArrowDown if !self.detail => self.step(1),
            KeyCode::PageUp if !self.detail => self.step(-(VISIBLE_ROWS as isize)),
            KeyCode::PageDown if !self.detail => self.step(VISIBLE_ROWS as isize),
            KeyCode::Home if !self.detail => self.step(isize::MIN / 2),
            KeyCode::End if !self.detail => self.step(isize::MAX / 2),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                self.detail = !self.detail && self.selected().is_some();
            }
            KeyCode::Escape | KeyCode::Backspace if self.detail => self.detail = false,
            KeyCode::KeyR if self.maskable(mantras) => {
                if arming {
                    if let Some((step, _, _)) = self.answer(mantras) {
                        self.revealed.push(step);
                    }
                } else {
                    self.arming = true;
                }
            }
            // The section keys are chapter 5's while its page is on; on a
            // card or with the cards alone they do nothing.
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Tab
                if self.with_page && !self.detail =>
            {
                return TrackerKey::Unused;
            }
            _ => {}
        }
        TrackerKey::Used
    }

    /// The page closed: back to the list. Reveals stay for the session.
    pub fn close(&mut self) {
        self.detail = false;
        self.arming = false;
    }
}

/// The key help for the tracker.
pub fn tracker_key_help(ui: &TrackerUi, mantras: MantraDisplay) -> &'static str {
    if ui.detail {
        if ui.arming {
            KEY_HELP_CONFIRM
        } else if ui.maskable(mantras) {
            KEY_HELP_REVEAL
        } else {
            KEY_HELP_CARD
        }
    } else if ui.cards().is_empty() {
        KEY_HELP_EMPTY
    } else if ui.with_page {
        KEY_HELP_LIST
    } else {
        KEY_HELP_LIST_ALONE
    }
}

/// The ink of a state: body grey, as chapter 5's values, and a ruined
/// shrine in red as chapter 5 shows it (5.2.6). The word is always there.
fn state_ink(mark: StatusMark) -> super::Rgb {
    if mark == StatusMark::ALERT {
        ega(mark.colour)
    } else {
        BODY
    }
}

const fn tick_ink(tick: Tick) -> super::Rgb {
    match tick {
        Tick::Done => ega(StatusMark::DONE.colour),
        Tick::Hollow => ega(StatusMark::CODEX_READ.colour),
        Tick::Unknown => DARK_GREY,
        Tick::Open => BODY,
    }
}

/// `state` in the box's 10 cells, centred as in the 8.2.7 sketch's `?`
/// and left-aligned otherwise.
fn boxed(state: &str) -> String {
    let state: String = state.chars().take(STATE_CELLS).collect();
    if state == text::UNKNOWN_STATE {
        format!("{:^width$}", state, width = STATE_CELLS)
    } else {
        format!("{:<width$}", state, width = STATE_CELLS)
    }
}

/// The cells of the tracker section, below the title band and above the
/// key help band.
pub fn tracker_cells(view: &PageView<'_>, ui: &TrackerUi) -> Vec<Cell> {
    let mut cells = Vec::new();
    if let Some(model) = view.model {
        cells.push(Cell::new(
            TEXT_COLUMN,
            HEADER_ROW,
            model.header.as_str(),
            BROWN,
        ));
    }
    let cards = ui.cards();
    match ui.selected() {
        Some(card) if ui.detail => card_cells(view, ui, card, &mut cells),
        _ => list_cells(ui, &cards, &mut cells),
    }
    cells
}

fn switcher(ui: &TrackerUi, title: &str, cells: &mut Vec<Cell>) {
    if ui.with_page {
        cells.push(Cell::new(TEXT_COLUMN, SECTION_ROW, GLYPH_LEFT, BROWN));
        cells.push(Cell::new(TEXT_COLUMN + 2, SECTION_ROW, title, WHITE));
        cells.push(Cell::new(
            TEXT_COLUMN + 3 + title.chars().count() as u8,
            SECTION_ROW,
            GLYPH_RIGHT,
            BROWN,
        ));
    } else {
        cells.push(Cell::new(TEXT_COLUMN, SECTION_ROW, title, WHITE));
    }
}

fn list_cells(ui: &TrackerUi, cards: &[&TrackerCard], cells: &mut Vec<Cell>) {
    switcher(ui, text::TRACKER_SECTION, cells);
    if cards.is_empty() {
        cells.push(Cell::new(
            TEXT_COLUMN,
            ROWS_FIRST_ROW,
            text::NO_CARDS,
            DARK_GREY,
        ));
        return;
    }
    if cards.len() > VISIBLE_ROWS {
        let more_above = ui.scroll > 0;
        let more_below = ui.scroll + VISIBLE_ROWS < cards.len();
        for (column, glyph, more) in [
            (LAST_TEXT_COLUMN - 1, GLYPH_UP, more_above),
            (LAST_TEXT_COLUMN, GLYPH_DOWN, more_below),
        ] {
            let ink = if more { BROWN } else { DARK_GREY };
            cells.push(Cell::new(column, SECTION_ROW, glyph, ink));
        }
    }
    for (column, head) in [
        (CARD_COLUMN, text::HEAD_CARD),
        (STATE_COLUMN, text::HEAD_STATE),
        (NEXT_COLUMN, text::HEAD_NEXT),
    ] {
        cells.push(Cell::new(column, HEADING_ROW, head, WHITE));
    }
    for (offset, card) in cards.iter().enumerate().skip(ui.scroll).take(VISIBLE_ROWS) {
        let y = ROWS_FIRST_ROW + (offset - ui.scroll) as u8;
        let selected = offset == ui.cursor;
        if selected {
            cells.push(Cell::new(TEXT_COLUMN, y, " ".repeat(TEXT_CELLS), WHITE).on(Paper::Bar));
        }
        let ink = if selected { WHITE } else { BODY };
        let title: String = card
            .title
            .as_str()
            .chars()
            .take(usize::from(STATE_COLUMN - CARD_COLUMN - 1))
            .collect();
        cells.push(Cell::new(CARD_COLUMN, y, title, ink));
        if card.boxed {
            cells.push(Cell::new(STATE_COLUMN, y, "[", BODY));
            cells.push(Cell::new(
                STATE_COLUMN + 1,
                y,
                boxed(card.state.as_str()),
                state_ink(card.mark),
            ));
            cells.push(Cell::new(
                STATE_COLUMN + 1 + STATE_CELLS as u8,
                y,
                "]",
                BODY,
            ));
        } else if !card.state.is_empty() {
            let room = usize::from(LAST_TEXT_COLUMN - STATE_COLUMN) + 1;
            let state: String = card.state.as_str().chars().take(room).collect();
            cells.push(Cell::new(STATE_COLUMN, y, state, BODY));
        }
        if !card.cell.is_empty() {
            cells.push(Cell::new(NEXT_COLUMN, y, card.cell.as_str(), BODY));
        }
    }
    if let Some(card) = ui.selected() {
        let lines: Vec<&str> = if card.next.is_empty() {
            card.notes.iter().map(|note| note.as_str()).collect()
        } else {
            card.next.iter().map(|line| line.as_str()).collect()
        };
        let joined = lines.join(if card.next.is_empty() { ", " } else { " " });
        for (index, line) in wrap(&joined, TEXT_CELLS, 2).into_iter().enumerate() {
            cells.push(Cell::new(
                TEXT_COLUMN,
                DETAIL_FIRST_ROW + index as u8,
                line,
                BODY,
            ));
        }
    }
}

fn card_cells(view: &PageView<'_>, ui: &TrackerUi, card: &TrackerCard, cells: &mut Vec<Cell>) {
    cells.push(Cell::new(
        TEXT_COLUMN,
        SECTION_ROW,
        card.title.as_str(),
        WHITE,
    ));
    if card.boxed {
        cells.push(Cell::new(STATE_COLUMN, SECTION_ROW, "[", BODY));
        cells.push(Cell::new(
            STATE_COLUMN + 1,
            SECTION_ROW,
            boxed(card.state.as_str()),
            state_ink(card.mark),
        ));
        cells.push(Cell::new(
            STATE_COLUMN + 1 + STATE_CELLS as u8,
            SECTION_ROW,
            "]",
            BODY,
        ));
    } else if !card.state.is_empty() {
        cells.push(Cell::new(
            STATE_COLUMN,
            SECTION_ROW,
            card.state.as_str(),
            BODY,
        ));
    }
    let mut y = ROWS_FIRST_ROW;
    let last = ROWS_FIRST_ROW + VISIBLE_ROWS as u8 - 1;
    let push = |cells: &mut Vec<Cell>, y: &mut u8, new: Vec<Cell>| {
        if *y <= last {
            cells.extend(new);
            *y += 1;
        }
    };
    for step in &card.steps {
        let label: String = step.label.as_str().chars().take(TEXT_CELLS - 4).collect();
        let row = y;
        push(
            cells,
            &mut y,
            vec![
                Cell::new(TEXT_COLUMN, row, step.tick.glyph(), tick_ink(step.tick)),
                Cell::new(TEXT_COLUMN + 4, row, label, BODY),
            ],
        );
    }
    for note in &card.notes {
        let row = y;
        push(
            cells,
            &mut y,
            vec![Cell::new(TEXT_COLUMN + 1, row, note.as_str(), BODY)],
        );
    }
    if let Some((step, label, secret)) = ui.answer(view.mantras) {
        y += 1;
        let label = format!("{label}: ");
        let column = TEXT_COLUMN + label.chars().count() as u8;
        let row = y;
        push(
            cells,
            &mut y,
            vec![
                Cell::new(TEXT_COLUMN, row, label, BODY),
                answer_cell(column, row, secret, ui.revealed.contains(&step)),
            ],
        );
    }
    let log_off = ui.view.as_ref().is_some_and(|view| view.clue_log_off);
    let clue_lines: Vec<String> = if log_off {
        vec![text::CLUE_LOG_OFF.to_string()]
    } else {
        let mut lines = vec![text::pinned_by_you(card.pinned)];
        if view.tier() >= SpoilerTier::Hints && card.heard > 0 {
            lines.push(text::heard_count(card.heard));
        }
        lines
    };
    y += 1;
    for line in clue_lines {
        let row = y;
        push(
            cells,
            &mut y,
            vec![Cell::new(TEXT_COLUMN, row, line, DARK_GREY)],
        );
    }
    let joined = card
        .next
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    for (index, line) in wrap(&joined, TEXT_CELLS, 2).into_iter().enumerate() {
        cells.push(Cell::new(
            TEXT_COLUMN,
            DETAIL_FIRST_ROW + index as u8,
            line,
            BODY,
        ));
    }
}
