//! Pixel compositor for the Ultima IV transfer preview screen.
//!
//! Provenance: `systems/u4-transfer.md §6.1` through `§6.6`, published
//! in answer to `cleak/u5-spec#73`. All geometry, wording and stage
//! sequencing lives in [`u5_runtime::u4_transfer_preview`]; this module
//! only paints what that module describes and owns the commit hand-off.
//!
//! Two structural properties of `§6` drive the design here:
//!
//! * **There is no double buffering, no page swap and no deferred
//!   flush anywhere on this path.** The screen is therefore modelled as
//!   one persistent surface that is drawn once and then edited in
//!   place, never as a per-frame repaint.
//! * **Every stage repeats exactly four moves and nothing else on the
//!   screen is redrawn at any point.** Each keystroke turns into a
//!   short list of [`U4PreviewEdit`]s that are applied to that
//!   surface; the panels are never rebuilt and the frames are never
//!   redrawn.
//!
//! `§6.4`'s insert-disk instruction block is unreachable dead code in
//! the shipped build and is deliberately not drawn.

use std::path::PathBuf;

use u5_runtime::gameplay_chrome::{ChromePalette, RibbonCapDirection};
use u5_runtime::intro_menu::{IntroSubflow, IntroSubflowResult};
use u5_runtime::text_wrap::{TextWindowDescriptor, text_window_centred_start_column};
#[cfg(test)]
use u5_runtime::u4_preview_panel_title_text;
use u5_runtime::{
    CH_CELL_SIDE, FixedCellFont, U4_PREVIEW_AVATAR_VERDICT_ROW, U4_PREVIEW_COMMIT_COLUMN,
    U4_PREVIEW_COMMIT_MESSAGE_WINDOW, U4_PREVIEW_COMMIT_ROW, U4_PREVIEW_FIELD_LABEL_COLUMN,
    U4_PREVIEW_FIELD_LABELS, U4_PREVIEW_FRAME_RULE_PATH, U4_PREVIEW_FULL_SCREEN_WINDOW,
    U4_PREVIEW_MESSAGE_LINE_WINDOW, U4_PREVIEW_NAME_ENTRY_MAX_CHARS,
    U4_PREVIEW_NAME_REPLACE_CURSOR, U4_PREVIEW_NAME_REPLACE_PROMPT, U4_PREVIEW_NAME_ROW,
    U4_PREVIEW_PANEL_BOTTOM_LEFT_CELL, U4_PREVIEW_PANEL_BOTTOM_RIGHT_CELL,
    U4_PREVIEW_PANEL_TITLE_FIRST_CELL, U4_PREVIEW_PANEL_TITLE_LEFT_CAP_CELL,
    U4_PREVIEW_PANEL_TITLE_RIGHT_CAP_CELL, U4_PREVIEW_PANEL_TITLE_ROW_CELLS,
    U4_PREVIEW_PANEL_TITLE_SOLID_CELLS, U4_PREVIEW_PANEL_TITLE_TEXT,
    U4_PREVIEW_RIGHT_PANEL_TITLE_BLANKED_CELL, U4_PREVIEW_VALUE_COLUMN,
    U4_TRANSFER_U4_SOURCE_FILENAME, U4PreviewAction, U4PreviewEdit, U4PreviewMessage,
    U4PreviewMessagePlacement, U4PreviewPageLine, U4PreviewPanel, U4PreviewPhase, U4PreviewSession,
    U4PreviewSource, commit_u4_transfer_save, load_ibm_ch_font, parse_u4_preview_source,
    read_disk_file, u4_preview_field_label, u4_preview_panel_bars, u4_preview_panel_rule_polyline,
    u4_preview_prompt_frame_cells,
};

use crate::{
    IntroDisplayBuffer, VisualIntroPanelOutcome, draw_intro_ribbon_cap, new_intro_display_buffer,
};

/// `display-driver.md §2` user-interface colour table, as the intro
/// already parameterises it. Slot 1 is the accent (rules, label text,
/// the stroked pass of a bracket cap); slot 2 is the panel/frame band.
const U4_PREVIEW_PALETTE: ChromePalette = ChromePalette::EGA;
/// Slot 1 — rules, title text, and the panel rule polyline.
const UI_COLOUR_SLOT_1: u8 = U4_PREVIEW_PALETTE.accent;
/// Slot 2 — the panel bars and every frame glyph.
const UI_COLOUR_SLOT_2: u8 = U4_PREVIEW_PALETTE.chrome;
/// The text windows' background.
const UI_BACKGROUND: u8 = U4_PREVIEW_PALETTE.background;
/// Ordinary text ink: the fixed-cell printer's active window colour.
const TEXT_INK: u8 = U4_PREVIEW_PALETTE.accent;

/// `§6.5`: how many cells a right-panel value write covers. Column 10
/// to the right panel's inner edge, so a shorter value blanks whatever
/// the longer one before it left behind.
const VALUE_FIELD_CELLS: u8 = 8;

/// Which whole-screen page still has to be painted before the pending
/// edits are applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingPage {
    /// `§6.3` bad-data page.
    Rejected,
    /// `§6.3` "Found" summary page.
    Found,
    /// `§6.1`/`§6.2`/`§6.5` comparison screen, left panel filled.
    Comparison,
}

/// What [`enter_u4_transfer`] produced.
pub(crate) enum U4TransferEntry {
    /// A source save was read; the preview owns the screen from here.
    Screen(Box<U4TransferScreen>),
    /// `§3`: missing or wrong media is a *retryable* condition, not a
    /// hard failure and not `§6.3`'s bad-data page. A single-directory
    /// implementation collapses the floppy prompts into an ordinary
    /// file-existence check, and the player-visible feedback comes from
    /// the resident media-error handler rather than from this screen.
    MediaUnavailable,
}

/// `§6` transfer preview: the persistent screen plus its stage machine.
#[derive(Debug)]
pub(crate) struct U4TransferScreen {
    game_dir: PathBuf,
    session: U4PreviewSession,
    /// The visible page. `§6` has no hidden surface, so this *is* the
    /// screen: painted once, then edited in place.
    surface: IntroDisplayBuffer,
    font: Option<FixedCellFont>,
    pending_page: Option<PendingPage>,
    pending_edits: Vec<U4PreviewEdit>,
}

/// `§6.4`/`§11` step 4: read the Ultima IV player disk's `PARTY.SAV`.
///
/// A file that will not read at all is media (`§3`); a file that reads
/// but fails `§5.2`'s gate lands on `§6.3`'s bad-data page.
pub(crate) fn enter_u4_transfer(game_dir: PathBuf) -> U4TransferEntry {
    let Ok(bytes) = read_disk_file(&game_dir.join(U4_TRANSFER_U4_SOURCE_FILENAME)) else {
        return U4TransferEntry::MediaUnavailable;
    };
    let session = match parse_u4_preview_source(&bytes) {
        Ok(source) => U4PreviewSession::new(source),
        Err(_rejected) => U4PreviewSession::rejected(),
    };
    U4TransferEntry::Screen(Box::new(U4TransferScreen::new(game_dir, session)))
}

/// A `§6` preview screen built from a **synthetic** source record, for
/// the visual frame suite only.
///
/// The suite renders published screens headlessly so their geometry can
/// be compared against captures of the original. There is no Ultima IV
/// install and no genuine `PARTY.SAV` anywhere in this repository, so
/// the character below is constructed from the published `§5.1` field
/// layout. It is a rendering input, never presented as captured data.
pub(crate) fn frame_suite_screen(game_dir: PathBuf) -> U4TransferScreen {
    U4TransferScreen::new(
        game_dir,
        U4PreviewSession::new(U4PreviewSource {
            name: "Avatar".to_string(),
            male: true,
            class_index: 5,
            strength: 35,
            dexterity: 20,
            intelligence: 22,
            experience: 1500,
            max_hit_points: 300,
            is_avatar: true,
        }),
    )
}

impl U4TransferScreen {
    fn new(game_dir: PathBuf, session: U4PreviewSession) -> Self {
        let pending_page = match session.phase() {
            U4PreviewPhase::Rejected => PendingPage::Rejected,
            _ => PendingPage::Found,
        };
        Self {
            game_dir,
            session,
            surface: new_intro_display_buffer(),
            font: None,
            pending_page: Some(pending_page),
            pending_edits: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_source(game_dir: PathBuf, source: U4PreviewSource) -> Self {
        Self::new(game_dir, U4PreviewSession::new(source))
    }

    #[cfg(test)]
    pub(crate) fn rejected(game_dir: PathBuf) -> Self {
        Self::new(game_dir, U4PreviewSession::rejected())
    }

    #[cfg(test)]
    pub(crate) fn session(&self) -> &U4PreviewSession {
        &self.session
    }

    #[cfg(test)]
    pub(crate) fn surface(&self) -> &IntroDisplayBuffer {
        &self.surface
    }

    fn font(&mut self) -> FixedCellFont {
        if self.font.is_none() {
            self.font = Some(
                load_ibm_ch_font(&self.game_dir)
                    .unwrap_or_else(|err| panic!("U4 transfer preview requires IBM.CH: {err}")),
            );
        }
        self.font.clone().expect("font just loaded")
    }

    /// `§6.5` move (d) is the caller's wait; this is what happens when
    /// the key finally arrives.
    pub(crate) fn step(&mut self, key: char) -> VisualIntroPanelOutcome {
        let response = self.session.key(key);
        match response.action {
            U4PreviewAction::None => {
                self.pending_edits.extend(response.edits);
                VisualIntroPanelOutcome::Stay
            }
            U4PreviewAction::BuildComparisonScreen => {
                self.pending_page = Some(PendingPage::Comparison);
                self.pending_edits.extend(response.edits);
                VisualIntroPanelOutcome::Stay
            }
            // `§6.3`: any key returns to the menu with nothing written.
            U4PreviewAction::ReturnToMenu => VisualIntroPanelOutcome::ReturnToMenu {
                subflow: IntroSubflow::UltimaIvTransfer,
                result: IntroSubflowResult::Cancelled,
                message: String::new(),
            },
            U4PreviewAction::Commit => {
                self.pending_edits.extend(response.edits);
                // `§6.6`: the commit is issued immediately after the
                // last keypress, and the notice is already on the
                // visible page by the time the write starts. This
                // renderer keeps its own surface, so present first.
                self.present();
                let source = self.session.committed_source();
                let avatar = commit_u4_transfer_save(&self.game_dir, &source, None)
                    .unwrap_or_else(|err| panic!("Ultima IV transfer save commit failed: {err}"));
                // `§6.6`/`§8`: on return the intro reloads and redraws
                // the start/menu view from scratch. Nothing underneath
                // this screen was saved, so nothing is restored.
                VisualIntroPanelOutcome::ReturnToMenu {
                    subflow: IntroSubflow::UltimaIvTransfer,
                    result: IntroSubflowResult::SaveReady,
                    message: format!(
                        "Transferred {}. Choose Journey Onward to load the new save.",
                        String::from_utf8_lossy(&avatar.name)
                            .trim_end_matches('\0')
                            .trim_end()
                    ),
                }
            }
        }
    }

    /// Flush every pending page build and edit onto the surface.
    fn present(&mut self) {
        let font = self.font();
        if let Some(page) = self.pending_page.take() {
            match page {
                PendingPage::Rejected => paint_full_screen_page(
                    &mut self.surface,
                    &font,
                    &self.session.rejected_page_lines(),
                ),
                PendingPage::Found => paint_full_screen_page(
                    &mut self.surface,
                    &font,
                    &self.session.found_page_lines(),
                ),
                PendingPage::Comparison => {
                    paint_comparison_screen(&mut self.surface, &font);
                    paint_panel_lines(
                        &mut self.surface,
                        &font,
                        U4PreviewPanel::Source,
                        &self.session.left_panel_lines(),
                    );
                }
            }
        }
        for edit in std::mem::take(&mut self.pending_edits) {
            apply_edit(&mut self.surface, &font, &edit);
        }
    }

    pub(crate) fn render(&mut self) -> Vec<u8> {
        self.present();
        self.surface.to_rgba()
    }

    /// A structural summary. `§6` forbids a text transcript standing in
    /// for the screen, so this names regions rather than reproducing
    /// what is drawn.
    pub(crate) fn summary(&self) -> String {
        let phase = match self.session.phase() {
            U4PreviewPhase::Rejected => "rejected-source page".to_string(),
            U4PreviewPhase::Found => "\"Found\" summary page".to_string(),
            U4PreviewPhase::Stage(stage) => format!("stage {stage:?} on panel row {}", stage.row()),
            U4PreviewPhase::Complete => "commit notice".to_string(),
        };
        format!(
            "Ultima IV transfer: {phase}; panels at x=0 and x=168, message line at cells 3..37 row 21."
        )
    }
}

// ---------------------------------------------------------------------------
// Painting
// ---------------------------------------------------------------------------

fn fill(buffer: &mut IntroDisplayBuffer, rect: (u16, u16, u16, u16), colour: u8) {
    let (x0, y0, x1, y1) = rect;
    buffer.clear_rect_inclusive(
        usize::from(x0),
        usize::from(y0),
        usize::from(x1),
        usize::from(y1),
        colour,
    );
}

/// Stroke an axis-aligned polyline one pixel wide.
///
/// Every segment `§6.1` publishes — the prompt frame's four-segment
/// rectangle and each panel's broken-top rule — is horizontal or
/// vertical, so no general line primitive is needed. A diagonal would
/// be an unpublished shape, hence the assert rather than a silent
/// approximation.
fn stroke_polyline(buffer: &mut IntroDisplayBuffer, points: &[(u16, u16)], colour: u8) {
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        assert!(
            x0 == x1 || y0 == y1,
            "u4-transfer.md §6.1 publishes only axis-aligned segments; ({x0}, {y0}) -> ({x1}, {y1}) is diagonal (cleak/u5-spec#73)"
        );
        fill(
            buffer,
            (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)),
            colour,
        );
    }
}

fn draw_text(
    buffer: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    text: &str,
    column: u8,
    row: u8,
    foreground: u8,
    background: u8,
) {
    for (offset, byte) in text.bytes().enumerate() {
        buffer.draw_fixed_glyph_cell(
            font,
            byte,
            usize::from(column) + offset,
            usize::from(row),
            foreground,
            background,
        );
    }
}

/// Clear a run of cells to the window background.
fn blank_cells(buffer: &mut IntroDisplayBuffer, column: u8, row: u8, cells: u8) {
    if cells == 0 {
        return;
    }
    let x0 = usize::from(column) * CH_CELL_SIDE;
    let y0 = usize::from(row) * CH_CELL_SIDE;
    buffer.clear_rect_inclusive(
        x0,
        y0,
        x0 + usize::from(cells) * CH_CELL_SIDE - 1,
        y0 + CH_CELL_SIDE - 1,
        UI_BACKGROUND,
    );
}

/// Absolute cell of a panel-relative `(column, row)`.
fn panel_cell(panel: U4PreviewPanel, column: u8, row: u8) -> (u8, u8) {
    let window = panel.window();
    (window.top_left_x + column, window.top_left_y + row)
}

/// The absolute start column of a line centred in `window`.
fn centred_column(window: TextWindowDescriptor, text: &str) -> u8 {
    let chars = u8::try_from(text.len()).unwrap_or(u8::MAX);
    window.top_left_x + text_window_centred_start_column(window.inner_width(), chars)
}

/// `§6.1`: the lower prompt frame plus its accent rectangle, and the
/// two character-information panels with their title plates.
fn paint_comparison_screen(buffer: &mut IntroDisplayBuffer, font: &FixedCellFont) {
    // `§6.1`: the path clears the whole screen with a single
    // full-screen text window, then draws the lower prompt frame,
    // before the three preview rectangles are installed.
    buffer.clear(UI_BACKGROUND);

    for cell in u4_preview_prompt_frame_cells() {
        buffer.draw_fixed_glyph_cell(
            font,
            cell.glyph,
            usize::from(cell.column),
            usize::from(cell.row),
            UI_COLOUR_SLOT_2,
            UI_BACKGROUND,
        );
    }
    stroke_polyline(buffer, &U4_PREVIEW_FRAME_RULE_PATH, UI_COLOUR_SLOT_1);

    for panel in U4PreviewPanel::BOTH {
        paint_panel(buffer, font, panel);
    }

    // `§6.1`: immediately after the second panel is drawn the path
    // selects the right panel and writes a single space over its title
    // cell 12 — the `I` of `IV`. Painting it as its own write keeps the
    // published draw order rather than quietly painting a different
    // string.
    let (column, row) = panel_cell(
        U4PreviewPanel::Result,
        U4_PREVIEW_RIGHT_PANEL_TITLE_BLANKED_CELL,
        0,
    );
    draw_text(buffer, font, " ", column, row, TEXT_INK, UI_BACKGROUND);

    // `§6.2`: each label is printed twice — once with the left panel
    // selected, once with the right — not once per display page.
    for panel in U4PreviewPanel::BOTH {
        for (row, label) in U4_PREVIEW_FIELD_LABELS {
            paint_label(buffer, font, panel, row, label, false);
        }
    }
}

fn paint_panel(buffer: &mut IntroDisplayBuffer, font: &FixedCellFont, panel: U4PreviewPanel) {
    let origin_x = panel.origin_x();
    for bar in u4_preview_panel_bars(origin_x) {
        fill(buffer, bar, UI_COLOUR_SLOT_2);
    }
    stroke_polyline(
        buffer,
        &u4_preview_panel_rule_polyline(origin_x),
        UI_COLOUR_SLOT_1,
    );

    let window = panel.window();
    let title_row = window.top_left_y;
    let last_cell = U4_PREVIEW_PANEL_TITLE_ROW_CELLS - 1;

    // Corners and the solid cells flanking the title plate.
    let corner = |cell: u8| window.top_left_x + cell;
    for (cell, glyph) in [
        (0, u5_runtime::U4_PREVIEW_FRAME_GLYPH_TOP_LEFT),
        (last_cell, u5_runtime::U4_PREVIEW_FRAME_GLYPH_TOP_RIGHT),
    ] {
        buffer.draw_fixed_glyph_cell(
            font,
            glyph,
            usize::from(corner(cell)),
            usize::from(title_row),
            UI_COLOUR_SLOT_2,
            UI_BACKGROUND,
        );
    }
    for cell in U4_PREVIEW_PANEL_TITLE_SOLID_CELLS {
        buffer.draw_fixed_glyph_cell(
            font,
            u5_runtime::U4_PREVIEW_FRAME_GLYPH_SOLID,
            usize::from(corner(cell)),
            usize::from(title_row),
            UI_COLOUR_SLOT_2,
            UI_BACKGROUND,
        );
    }

    // `§6.1`: the two title-plate caps are the build's single bracket
    // primitive — the solid triangle glyph in the panel colour plus two
    // accent rules along its hypotenuse. `display-driver.md §7` owns
    // that composite, so it is reused rather than restated.
    for (cell, direction) in [
        (
            U4_PREVIEW_PANEL_TITLE_LEFT_CAP_CELL,
            RibbonCapDirection::Right,
        ),
        (
            U4_PREVIEW_PANEL_TITLE_RIGHT_CAP_CELL,
            RibbonCapDirection::Left,
        ),
    ] {
        draw_intro_ribbon_cap(
            buffer,
            font,
            direction,
            usize::from(corner(cell)),
            usize::from(title_row),
            U4_PREVIEW_PALETTE,
        );
    }

    // Both panels are painted from the same ` Ultima IV ` string; the
    // right panel's cell 12 is blanked afterwards by the caller.
    draw_text(
        buffer,
        font,
        U4_PREVIEW_PANEL_TITLE_TEXT,
        corner(U4_PREVIEW_PANEL_TITLE_FIRST_CELL),
        title_row,
        TEXT_INK,
        UI_BACKGROUND,
    );

    for (cell, row, glyph) in [
        (
            U4_PREVIEW_PANEL_BOTTOM_LEFT_CELL.0,
            U4_PREVIEW_PANEL_BOTTOM_LEFT_CELL.1,
            u5_runtime::U4_PREVIEW_FRAME_GLYPH_BOTTOM_LEFT,
        ),
        (
            U4_PREVIEW_PANEL_BOTTOM_RIGHT_CELL.0,
            U4_PREVIEW_PANEL_BOTTOM_RIGHT_CELL.1,
            u5_runtime::U4_PREVIEW_FRAME_GLYPH_BOTTOM_RIGHT,
        ),
    ] {
        buffer.draw_fixed_glyph_cell(
            font,
            glyph,
            usize::from(corner(cell)),
            usize::from(window.top_left_y + row),
            UI_COLOUR_SLOT_2,
            UI_BACKGROUND,
        );
    }
}

/// `§6.2`/`§6.5` move (a): reprint one label, normal or inverse video.
/// Highlighting is a per-window inverse toggle applied around the
/// single label reprint; nothing else is repainted.
fn paint_label(
    buffer: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    panel: U4PreviewPanel,
    row: u8,
    label: &str,
    inverse: bool,
) {
    let (column, row) = panel_cell(panel, U4_PREVIEW_FIELD_LABEL_COLUMN, row);
    let (foreground, background) = if inverse {
        (UI_BACKGROUND, TEXT_INK)
    } else {
        (TEXT_INK, UI_BACKGROUND)
    };
    draw_text(buffer, font, label, column, row, foreground, background);
}

fn paint_panel_lines(
    buffer: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    panel: U4PreviewPanel,
    lines: &[U4PreviewPageLine],
) {
    let window = panel.window();
    for line in lines {
        let column = match line.column {
            Some(column) => window.top_left_x + column,
            None => centred_column(window, &line.text),
        };
        draw_text(
            buffer,
            font,
            &line.text,
            column,
            window.top_left_y + line.row,
            TEXT_INK,
            UI_BACKGROUND,
        );
    }
}

/// `§6.3`: both full-screen pages clear first and then print into the
/// full 40-by-25 window.
fn paint_full_screen_page(
    buffer: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    lines: &[U4PreviewPageLine],
) {
    buffer.clear(UI_BACKGROUND);
    let window = U4_PREVIEW_FULL_SCREEN_WINDOW;
    for line in lines {
        let column = match line.column {
            Some(column) => window.top_left_x + column,
            None => centred_column(window, &line.text),
        };
        draw_text(
            buffer,
            font,
            &line.text,
            column,
            window.top_left_y + line.row,
            TEXT_INK,
            UI_BACKGROUND,
        );
    }
}

/// `§6.5` move (c): the message-line window is cleared, then the
/// stage's message is printed into it.
fn paint_message(
    buffer: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    message: &U4PreviewMessage,
) {
    let window = U4_PREVIEW_MESSAGE_LINE_WINDOW;
    blank_cells(
        buffer,
        window.top_left_x,
        window.top_left_y,
        window.bottom_right_x - window.top_left_x + 1,
    );
    let (column, row) = match message.placement {
        U4PreviewMessagePlacement::Cell(x, y) => (window.top_left_x + x, window.top_left_y + y),
        U4PreviewMessagePlacement::Centred => {
            (centred_column(window, &message.text), window.top_left_y)
        }
    };
    draw_text(
        buffer,
        font,
        &message.text,
        column,
        row,
        TEXT_INK,
        UI_BACKGROUND,
    );
}

/// `§6.5`: the replacement-name field starts immediately after the
/// sixteen-character prompt and is at most eight characters wide.
fn name_field_cell() -> (u8, u8) {
    let window = U4_PREVIEW_MESSAGE_LINE_WINDOW;
    let prompt_len = u8::try_from(U4_PREVIEW_NAME_REPLACE_PROMPT.len())
        .expect("the published prompt is sixteen characters");
    (
        window.top_left_x + U4_PREVIEW_NAME_REPLACE_CURSOR.0 + prompt_len,
        window.top_left_y + U4_PREVIEW_NAME_REPLACE_CURSOR.1,
    )
}

fn apply_edit(buffer: &mut IntroDisplayBuffer, font: &FixedCellFont, edit: &U4PreviewEdit) {
    match edit {
        U4PreviewEdit::Stage(stage) => {
            // (a) In both panels, reprint the previous stage's label in
            // normal video and the new stage's label in inverse.
            for panel in U4PreviewPanel::BOTH {
                if let Some(row) = stage.previous_label_row
                    && let Some(label) = u4_preview_field_label(row)
                {
                    paint_label(buffer, font, panel, row, label, false);
                }
                if let Some(label) = u4_preview_field_label(stage.inverse_label_row) {
                    paint_label(buffer, font, panel, stage.inverse_label_row, label, true);
                }
            }
            // (b) Write the converted value into the right panel at
            // column 10 of the stage's row.
            if let Some(value) = &stage.right_value {
                paint_right_value(buffer, font, stage.stage.row(), value);
            }
            // (c) Clear the message-line window and print the message.
            paint_message(buffer, font, &stage.message);
        }
        U4PreviewEdit::RightValue { row, text } => paint_right_value(buffer, font, *row, text),
        U4PreviewEdit::NameField(entry) => {
            let (column, row) = name_field_cell();
            blank_cells(
                buffer,
                column,
                row,
                u8::try_from(U4_PREVIEW_NAME_ENTRY_MAX_CHARS).unwrap_or(u8::MAX),
            );
            draw_text(buffer, font, entry, column, row, TEXT_INK, UI_BACKGROUND);
        }
        U4PreviewEdit::AcceptedName(name) => {
            let window = U4PreviewPanel::Result.window();
            let row = window.top_left_y + U4_PREVIEW_NAME_ROW;
            blank_cells(buffer, window.top_left_x, row, window.inner_width());
            draw_text(
                buffer,
                font,
                name,
                centred_column(window, name),
                row,
                TEXT_INK,
                UI_BACKGROUND,
            );
        }
        U4PreviewEdit::Finish { verdict, notice } => {
            // `§6.6`: `Avatar` or `Non-Avatar`, centred, into right
            // panel cell (0, 15).
            let window = U4PreviewPanel::Result.window();
            draw_text(
                buffer,
                font,
                verdict,
                centred_column(window, verdict),
                window.top_left_y + U4_PREVIEW_AVATAR_VERDICT_ROW,
                TEXT_INK,
                UI_BACKGROUND,
            );
            // `§6.6`: the message-line window is widened to columns
            // 2..37, rows 21..22, cleared, and the notice printed. The
            // published string's line breaks scroll the two-row window
            // so the settled result is the notice on row 21 with its
            // leading space at column 2 and row 22 left blank.
            let commit = U4_PREVIEW_COMMIT_MESSAGE_WINDOW;
            let cells = commit.bottom_right_x - commit.top_left_x + 1;
            for row in commit.top_left_y..=commit.bottom_right_y {
                blank_cells(buffer, commit.top_left_x, row, cells);
            }
            draw_text(
                buffer,
                font,
                notice,
                U4_PREVIEW_COMMIT_COLUMN,
                U4_PREVIEW_COMMIT_ROW,
                TEXT_INK,
                UI_BACKGROUND,
            );
        }
    }
}

fn paint_right_value(buffer: &mut IntroDisplayBuffer, font: &FixedCellFont, row: u8, value: &str) {
    let (column, row) = panel_cell(U4PreviewPanel::Result, U4_PREVIEW_VALUE_COLUMN, row);
    blank_cells(buffer, column, row, VALUE_FIELD_CELLS);
    draw_text(buffer, font, value, column, row, TEXT_INK, UI_BACKGROUND);
}

/// `§6.5`: everything a stage is allowed to repaint, as absolute cell
/// rows. Used by the redraw-scope test and by nothing else.
#[cfg(test)]
pub(crate) fn stage_edit_permitted_rows(edit: &u5_runtime::U4PreviewStageEdit) -> Vec<u8> {
    let mut rows = vec![U4_PREVIEW_MESSAGE_LINE_WINDOW.top_left_y];
    for row in [Some(edit.inverse_label_row), edit.previous_label_row]
        .into_iter()
        .flatten()
    {
        rows.push(U4PreviewPanel::Source.window().top_left_y + row);
    }
    rows.push(U4PreviewPanel::Result.window().top_left_y + edit.stage.row());
    rows.sort_unstable();
    rows.dedup();
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use u5_runtime::{
        U4_PREVIEW_CLASS_NAMES, U4_PREVIEW_REJECTED_LINES, U4PreviewStage,
        synthetic_party_sav_fixture,
    };

    /// A **synthetic** `PARTY.SAV` source built from the published
    /// `§5.1` layout. There is no Ultima IV install and no genuine
    /// `PARTY.SAV` anywhere in this repository; this is a constructed
    /// test input, never presented as captured data.
    fn synthetic_source(is_avatar: bool) -> U4PreviewSource {
        U4PreviewSource {
            name: "Dupre".to_string(),
            male: true,
            class_index: 5,
            strength: 35,
            dexterity: 20,
            intelligence: 22,
            experience: 1500,
            max_hit_points: 300,
            is_avatar,
        }
    }

    /// A scratch game directory whose `IBM.CH` has 128 **pairwise
    /// distinct** glyphs.
    ///
    /// The shared fixture writes a uniform `0xFF` font, which renders
    /// every character as the same solid block and would make every
    /// text assertion below vacuously true. Row 0 of glyph `c` is `c`,
    /// so no two glyphs can compare equal.
    fn preview_game_dir() -> PathBuf {
        let dir = u5_runtime::test_fixtures::debug_game_dir();
        let mut font = vec![0u8; u5_runtime::CH_FONT_LEN];
        for code in 0..128usize {
            for row in 0..CH_CELL_SIDE {
                font[code * CH_CELL_SIDE + row] =
                    (code as u8) ^ (u8::try_from(row).unwrap().wrapping_mul(0x11));
            }
        }
        std::fs::write(dir.join(u5_runtime::IBM_CH_FILE), font).unwrap();
        dir
    }

    fn screen(is_avatar: bool) -> U4TransferScreen {
        U4TransferScreen::from_source(preview_game_dir(), synthetic_source(is_avatar))
    }

    fn cleanup(screen: U4TransferScreen) {
        let _ = std::fs::remove_dir_all(&screen.game_dir);
    }

    fn cell_row(surface: &IntroDisplayBuffer, column: u8, row: u8) -> Vec<u8> {
        let x0 = usize::from(column) * CH_CELL_SIDE;
        let y0 = usize::from(row) * CH_CELL_SIDE;
        let mut pixels = Vec::new();
        for y in y0..y0 + CH_CELL_SIDE {
            pixels.extend_from_slice(&surface.pixels[y * surface.width + x0..][..CH_CELL_SIDE]);
        }
        pixels
    }

    /// Render one text run into a scratch surface so a screen region
    /// can be compared against expected characters without reading a
    /// transcript back out of pixels.
    fn expected_cells(
        font: &FixedCellFont,
        text: &str,
        foreground: u8,
        background: u8,
    ) -> Vec<Vec<u8>> {
        let mut scratch = new_intro_display_buffer();
        scratch.clear(UI_BACKGROUND);
        draw_text(&mut scratch, font, text, 0, 0, foreground, background);
        (0..text.len())
            .map(|index| cell_row(&scratch, u8::try_from(index).unwrap(), 0))
            .collect()
    }

    fn assert_text_at(
        surface: &IntroDisplayBuffer,
        font: &FixedCellFont,
        text: &str,
        column: u8,
        row: u8,
    ) {
        assert_text_at_in(surface, font, text, column, row, TEXT_INK, UI_BACKGROUND);
    }

    fn assert_text_at_in(
        surface: &IntroDisplayBuffer,
        font: &FixedCellFont,
        text: &str,
        column: u8,
        row: u8,
        foreground: u8,
        background: u8,
    ) {
        for (index, expected) in expected_cells(font, text, foreground, background)
            .into_iter()
            .enumerate()
        {
            let column = column + u8::try_from(index).unwrap();
            assert_eq!(
                cell_row(surface, column, row),
                expected,
                "cell ({column}, {row}) of {text:?}"
            );
        }
    }

    /// Is `(x, y)` on `§6.1`'s four-segment accent rectangle?
    fn on_frame_rule(x: usize, y: usize) -> bool {
        U4_PREVIEW_FRAME_RULE_PATH.windows(2).any(|pair| {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            (usize::from(x0.min(x1))..=usize::from(x0.max(x1))).contains(&x)
                && (usize::from(y0.min(y1))..=usize::from(y0.max(y1))).contains(&y)
        })
    }

    fn advance_to_comparison(screen: &mut U4TransferScreen) -> FixedCellFont {
        screen.step(' ');
        let _ = screen.render();
        screen.font()
    }

    #[test]
    fn both_panel_titles_are_drawn_and_only_the_right_one_loses_its_i() {
        let mut screen = screen(true);
        let font = advance_to_comparison(&mut screen);
        let surface = screen.surface().clone();

        let left = U4PreviewPanel::Source.window();
        let right = U4PreviewPanel::Result.window();
        assert_text_at(
            &surface,
            &font,
            " Ultima IV ",
            left.top_left_x + U4_PREVIEW_PANEL_TITLE_FIRST_CELL,
            left.top_left_y,
        );
        assert_text_at(
            &surface,
            &font,
            " Ultima  V ",
            right.top_left_x + U4_PREVIEW_PANEL_TITLE_FIRST_CELL,
            right.top_left_y,
        );
        assert_eq!(
            u4_preview_panel_title_text(U4PreviewPanel::Result),
            " Ultima  V "
        );
        cleanup(screen);
    }

    #[test]
    fn both_panels_carry_the_eight_field_labels() {
        let mut screen = screen(true);
        let font = advance_to_comparison(&mut screen);
        let surface = screen.surface().clone();
        for panel in U4PreviewPanel::BOTH {
            for (label_row, label) in U4_PREVIEW_FIELD_LABELS {
                let (column, row) = panel_cell(panel, U4_PREVIEW_FIELD_LABEL_COLUMN, label_row);
                // `§6.5` move (a): the opening stage has already put
                // the Name row into inverse video in both panels.
                let inverse = label_row == U4PreviewStage::NameConfirm.row();
                let (foreground, background) = if inverse {
                    (UI_BACKGROUND, TEXT_INK)
                } else {
                    (TEXT_INK, UI_BACKGROUND)
                };
                assert_text_at_in(&surface, &font, label, column, row, foreground, background);
            }
        }
        cleanup(screen);
    }

    #[test]
    fn the_prompt_frame_lands_on_the_published_cells_and_rule() {
        let mut screen = screen(true);
        let font = advance_to_comparison(&mut screen);
        let surface = screen.surface().clone();
        for cell in u4_preview_prompt_frame_cells() {
            let expected = expected_cells(
                &font,
                &(cell.glyph as char).to_string(),
                UI_COLOUR_SLOT_2,
                UI_BACKGROUND,
            );
            let actual = cell_row(&surface, cell.column, cell.row);
            for (index, expected) in expected[0].iter().enumerate() {
                let x = usize::from(cell.column) * CH_CELL_SIDE + index % CH_CELL_SIDE;
                let y = usize::from(cell.row) * CH_CELL_SIDE + index / CH_CELL_SIDE;
                // The accent rectangle is stroked after the frame band
                // and legitimately overwrites the pixels it crosses.
                if on_frame_rule(x, y) {
                    continue;
                }
                assert_eq!(
                    actual[index], *expected,
                    "frame cell ({}, {}) pixel ({x}, {y})",
                    cell.column, cell.row
                );
            }
        }
        // The accent rectangle's four corners.
        let at = |x: usize, y: usize| surface.pixels[y * surface.width + x];
        assert_eq!(at(7, 159), UI_COLOUR_SLOT_1);
        assert_eq!(at(312, 159), UI_COLOUR_SLOT_1);
        assert_eq!(at(312, 184), UI_COLOUR_SLOT_1);
        assert_eq!(at(7, 184), UI_COLOUR_SLOT_1);
        cleanup(screen);
    }

    #[test]
    fn the_panel_bars_are_painted_in_ui_colour_slot_two() {
        let mut screen = screen(true);
        advance_to_comparison(&mut screen);
        let surface = screen.surface().clone();
        let at = |x: usize, y: usize| surface.pixels[y * surface.width + x];
        for origin in [0usize, 168] {
            // Sampled clear of the title row and the bottom corner
            // glyphs, which are drawn over the bars afterwards.
            assert_eq!(at(origin, 100), UI_COLOUR_SLOT_2, "left bar at {origin}");
            assert_eq!(
                at(origin + 6, 100),
                UI_COLOUR_SLOT_2,
                "left bar at {origin}"
            );
            assert_eq!(
                at(origin + 145, 100),
                UI_COLOUR_SLOT_2,
                "right bar at {origin}"
            );
            assert_eq!(
                at(origin + 50, 140),
                UI_COLOUR_SLOT_2,
                "foot bar at {origin}"
            );
            // The broken-top rule polyline runs in slot 1 down
            // `origin + 7` and `origin + 143`.
            assert_eq!(at(origin + 7, 100), UI_COLOUR_SLOT_1, "rule at {origin}");
            assert_eq!(at(origin + 143, 100), UI_COLOUR_SLOT_1, "rule at {origin}");
        }
        cleanup(screen);
    }

    #[test]
    fn a_stage_redraw_touches_only_the_four_published_moves() {
        let mut screen = screen(true);
        advance_to_comparison(&mut screen);
        let before = screen.surface().clone();

        // `Y` at the name prompt enters the Sex stage.
        screen.step('Y');
        let U4PreviewPhase::Stage(stage) = screen.session().phase() else {
            panic!("expected a stage");
        };
        assert_eq!(stage, U4PreviewStage::SexConfirm);
        let edit = u5_runtime::U4PreviewStageEdit {
            stage,
            previous_label_row: Some(U4PreviewStage::NameConfirm.row()),
            inverse_label_row: stage.row(),
            right_value: Some("Male".to_string()),
            message: screen.session().stage_message(stage),
        };
        let permitted = stage_edit_permitted_rows(&edit);
        let _ = screen.render();
        let after = screen.surface().clone();

        let mut changed_rows: Vec<u8> = Vec::new();
        for row in 0..25u8 {
            for column in 0..40u8 {
                if cell_row(&before, column, row) != cell_row(&after, column, row) {
                    changed_rows.push(row);
                    break;
                }
            }
        }
        assert_eq!(
            changed_rows, permitted,
            "a stage redraw repainted rows outside the four published moves"
        );
        cleanup(screen);
    }

    #[test]
    fn an_invalid_key_at_a_confirmation_prompt_changes_no_pixel() {
        let mut screen = screen(true);
        advance_to_comparison(&mut screen);
        let before = screen.surface().clone();
        for key in ['q', '7', '\x1b', '\r'] {
            screen.step(key);
            let _ = screen.render();
            assert_eq!(
                screen.surface(),
                &before,
                "{key:?} redrew part of the screen"
            );
        }
        cleanup(screen);
    }

    #[test]
    fn escape_never_leaves_the_transfer_once_the_screen_is_up() {
        let mut screen = screen(true);
        advance_to_comparison(&mut screen);
        assert!(matches!(screen.step('\x1b'), VisualIntroPanelOutcome::Stay));
        cleanup(screen);
    }

    #[test]
    fn a_name_edit_repaints_only_the_typed_run() {
        let mut screen = screen(true);
        advance_to_comparison(&mut screen);
        screen.step('N');
        let _ = screen.render();
        let before = screen.surface().clone();

        screen.step('Z');
        let _ = screen.render();
        let after = screen.surface().clone();

        let (field_column, field_row) = name_field_cell();
        for row in 0..25u8 {
            for column in 0..40u8 {
                let changed = cell_row(&before, column, row) != cell_row(&after, column, row);
                let inside_field = row == field_row
                    && column >= field_column
                    && column
                        < field_column + u8::try_from(U4_PREVIEW_NAME_ENTRY_MAX_CHARS).unwrap();
                assert!(
                    !changed || inside_field,
                    "cell ({column}, {row}) changed outside the typed field"
                );
            }
        }
        cleanup(screen);
    }

    #[test]
    fn the_rejected_source_page_is_the_published_text() {
        let mut screen = U4TransferScreen::rejected(preview_game_dir());
        let _ = screen.render();
        let font = screen.font();
        let surface = screen.surface().clone();
        for (index, text) in U4_PREVIEW_REJECTED_LINES.iter().enumerate() {
            if text.is_empty() {
                continue;
            }
            let row = u5_runtime::U4_PREVIEW_REJECTED_FIRST_ROW + u8::try_from(index).unwrap();
            let column = centred_column(U4_PREVIEW_FULL_SCREEN_WINDOW, text);
            assert_text_at(&surface, &font, text, column, row);
        }
        // Any key returns to the menu with nothing written.
        assert!(matches!(
            screen.step('\x1b'),
            VisualIntroPanelOutcome::ReturnToMenu { .. }
        ));
        assert!(!screen.game_dir.join("SAVED.GAM").exists());
        cleanup(screen);
    }

    #[test]
    fn the_found_page_is_drawn_before_the_comparison_screen() {
        let mut screen = screen(false);
        let _ = screen.render();
        let font = screen.font();
        let surface = screen.surface().clone();
        assert_text_at(
            &surface,
            &font,
            "Found:",
            centred_column(U4_PREVIEW_FULL_SCREEN_WINDOW, "Found:"),
            u5_runtime::U4_PREVIEW_FOUND_LABEL_ROW,
        );
        assert_text_at(&surface, &font, "STR:  35", 17, 15);
        cleanup(screen);
    }

    #[test]
    fn the_insert_disk_instructions_are_never_drawn() {
        // `§6.4`: the shipped build's block is unreachable dead code.
        // Scan only the production half of this file, so the guard's
        // own needles do not match themselves.
        let module = include_str!("u4_transfer.rs");
        let (production, _) = module
            .rsplit_once("mod tests {")
            .expect("this file ends with its test module");
        for dead in [
            "Please insert",
            "press drive letter",
            "Esc> to abort transfer",
            "Transfer Character from Ultima IV",
        ] {
            assert!(
                !production.contains(dead),
                "{dead:?} appears in the transfer compositor"
            );
        }
    }

    #[test]
    fn the_last_keypress_writes_the_save_with_the_replacement_name() {
        let mut screen = screen(true);
        std::fs::write(
            screen
                .game_dir
                .join(u5_runtime::U4_TRANSFER_U5_SEED_GAM_FILENAME),
            u5_runtime::test_fixtures::saved_game_seed_bytes(0, 0, 10, 20),
        )
        .unwrap();
        std::fs::write(
            screen
                .game_dir
                .join(u5_runtime::U4_TRANSFER_U5_SEED_OOL_FILENAME),
            vec![0x55; u5_runtime::OOL_PLANE_LEN],
        )
        .unwrap();
        advance_to_comparison(&mut screen);

        // `N` at the name prompt, a replacement, then `Y` at the sex
        // prompt and one key per informational stage.
        screen.step('N');
        for key in "Shamino".chars() {
            screen.step(key);
        }
        screen.step('\r');
        screen.step('Y');
        let mut outcome = VisualIntroPanelOutcome::Stay;
        for _ in 0..6 {
            assert!(
                !screen
                    .game_dir
                    .join(u5_runtime::SAVED_GAM_FILENAME)
                    .exists(),
                "nothing is written before the last stage's keypress"
            );
            outcome = screen.step('\r');
        }
        assert!(matches!(
            outcome,
            VisualIntroPanelOutcome::ReturnToMenu {
                result: IntroSubflowResult::SaveReady,
                ..
            }
        ));
        let saved = std::fs::read(screen.game_dir.join(u5_runtime::SAVED_GAM_FILENAME)).unwrap();
        assert!(saved[u5_runtime::SAVE_ROSTER_OFFSET..].starts_with(b"Shamino\0"));
        assert!(
            screen
                .game_dir
                .join(u5_runtime::SAVED_OOL_FILENAME)
                .exists()
        );
        cleanup(screen);
    }

    #[test]
    fn the_source_reader_accepts_a_synthetic_party_sav_fixture() {
        let source = synthetic_source(true);
        let bytes = synthetic_party_sav_fixture(&source);
        assert_eq!(parse_u4_preview_source(&bytes).unwrap(), source);
        assert_eq!(
            U4_PREVIEW_CLASS_NAMES[usize::from(source.class_index)],
            "Paladin"
        );
    }
}
