//! The menu on screen (Phase B design 3.3.4 "Rendering", 3.4.4 "Chapter
//! 2"; build plan M4-05, D-M4.1, D-M4.10).
//!
//! - [`render_page`] draws the open page's rows with the M4-01 frame kit
//!   ([`super::frame`]) into a picture the size of the one the frame sprite
//!   shows: the 320x200 frame on 4:3 and on fixed screens, the grow-left
//!   canvas otherwise ([`super::geometry::menu_picture`]). Panel pixels are
//!   opaque; everything else is clear.
//! - `menu_render` (`EnhSet::Present`) redraws that picture only when the
//!   page, its rows, the picture or the column's place changes, and
//!   recomposes the node's image only when the sweep step changes.
//! - `place_overlay` (`EnhSet::Present`, after the render) puts the menu's
//!   `ImageNode` ([`MenuNode`]) exactly over the frame sprite's rectangle,
//!   with layout rounding off, so one menu pixel is one game pixel at every
//!   scale, and a full-window backdrop node ([`MenuBackdrop`], the kit's 0.6 dim as
//!   the GPU blends it) under it, which dims the frame, the bars, the side fill and the
//!   side panels. The toasts (`i32::MAX - 1`) stay above both. A gem view
//!   is never the idle prompt, so the gem map is never up with the menu and
//!   the two share its layers ([`MENU_NODE_Z`], [`MENU_BACKDROP_Z`]). The
//!   menu is never a `Sprite`, so `fit_display_to_window` never sees it.
//!
//! The picture over the backdrop is the kit's dimmed picture over the
//! frame (`frame::composite_over` of `frame::dim_backdrop`): a clear pixel
//! shows the backdrop's dim, an opaque one covers it. The backdrop's
//! opacity ([`BACKDROP_ALPHA`]) is chosen for the GPU's linear-light blend,
//! so the real dim looks as the sheet's does.
//!
//! **The sweep** (D-M4.10) is the M1 dither law over the panel's rectangle
//! only ([`super::frame::panel_step`]), rising from step 1 on open and
//! falling from step 7 on close, a step every [`MENU_SWEEP_STEP`]. A close
//! that interrupts an open (or the reverse) turns back from the step it is
//! on. Keys are never held for it: a key during the sweep changes the model
//! that frame, the picture is redrawn and the sweep goes on. Page changes
//! are instant. A close because the game left the prompt
//! ([`CloseReason::NotIdle`]) skips it: the menu goes that frame.

use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::LayoutConfig;
use u5_runtime::FixedCellFont;

use super::frame::Scroll;
use super::frame::*;
use super::geometry::{CellRect, MenuPicture, PageShape, menu_picture, panel_rect};
use super::pages::{
    Dialog, Page, PageState, Row, RowAction, RowKind, SettingsTab, ShellSnapshot, Shown, SlotList,
    page_rows, settle,
};
use super::render_slots;
use super::text as t;
use super::title::TitleFont;
use super::{MenuState, SnapshotInputs};
use crate::display::Display;
use crate::display::effects::{STEP_SLACK, SWEEP_STEPS};
use crate::display::grow::column::{ColumnState, ColumnView, page_mode_active};
use crate::display::grow::{Canvas, grow_canvas};
use crate::enh::gem_map::view::{GEM_BEFORE_Z, GEM_NODE_Z};
use crate::enh::overlay::{CloseReason, OverlayChanged, OverlayId};
use crate::enh::present::PresentHost;
use crate::enh::side::FrameImageSize;
use crate::{VisualIntroState, VisualState};
use u5_runtime::enhancements::PRESETS;

/// The menu node's layer: the gem map's, above the frame sprite, the side
/// fill and the side panels, below the toasts.
pub const MENU_NODE_Z: i32 = GEM_NODE_Z;
/// The backdrop's layer, directly under the menu node.
pub const MENU_BACKDROP_Z: i32 = GEM_BEFORE_Z;

/// The backdrop's opacity on the GPU. The kit's dim (and the M4-01 sheet)
/// is black at 0.6 blended on the sRGB values ([`composite_over`]); the
/// GPU blends UI nodes in linear light, where 0.6 dims far less (white to
/// 170, the band's blue to 112, against 102 and 68 on the sheet). Black at
/// this opacity in linear light lands within 4 of the sheet's dim on every
/// EGA level (pinned by `tests::the_gpu_backdrop_matches_the_sheet_s_dim`).
pub const BACKDROP_ALPHA: f32 = 0.858;

/// The backdrop's colour: black at [`BACKDROP_ALPHA`].
pub fn backdrop_colour() -> Color {
    Color::srgba(0.0, 0.0, 0.0, BACKDROP_ALPHA)
}

/// Rows of the top menu's footer box.
const TOP_FOOTER_ROWS: u32 = 3;
/// Rows of a Settings page's footer box.
const SETTINGS_FOOTER_ROWS: u32 = 4;
/// The cell a Settings row's value starts at: after the margin, the label
/// column and one blank.
pub const VALUE_COL: u32 = 1 + t::SETTINGS_LABEL as u32 + 1;

/// One page drawn: the picture (clear outside the panel) and the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub image: MenuImage,
    pub panel: CellRect,
}

/// The first row shown of a list of `len` rows with room for `visible`,
/// so that row `cursor` is in sight: the list scrolls only once the cursor
/// passes the last visible row.
pub fn first_shown(cursor: usize, len: usize, visible: usize) -> usize {
    if visible == 0 || len <= visible {
        return 0;
    }
    cursor
        .min(len - 1)
        .saturating_sub(visible - 1)
        .min(len - visible)
}

/// The rows `bx` shows of `rows`, between its top and bottom margin rows,
/// from `first`; their lines from `line`, the selected one (if it is in
/// this list) as the selection bar, with the scroll marks on the bands
/// above (`upper`) and below (`lower`). The margin rows keep the bar off
/// the rules above and below the list: it never merges with them.
#[allow(clippy::too_many_arguments)]
fn draw_list(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    upper: u32,
    lower: u32,
    rows: &[Row],
    cursor: Option<usize>,
    line: impl Fn(&Row, u32) -> Line,
) {
    let lines: Vec<Line> = rows.iter().map(|row| line(row, bx.w)).collect();
    draw_lines(img, font, bx, upper, lower, &lines, cursor);
}

/// `lines` (each `bx.w` cells) in the list box `bx`, as [`draw_list`]
/// draws a page's rows: between the box's margin rows, scrolled so line
/// `cursor` is in sight and drawn as the selection bar, with the scroll
/// marks on the bands above (`upper`) and below (`lower`).
pub(super) fn draw_lines(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    upper: u32,
    lower: u32,
    lines: &[Line],
    cursor: Option<usize>,
) {
    let visible = list_rows(bx);
    let first = first_shown(cursor.unwrap_or(0), lines.len(), visible);
    for (i, line) in lines.iter().enumerate().skip(first).take(visible) {
        let at = bx.row + 1 + (i - first) as u32;
        draw_row(img, font, bx, at, line, Some(i) == cursor);
    }
    if first > 0 {
        draw_scroll_mark(img, font, bx.right() - 1, upper, Scroll::Up);
    }
    if first + visible < lines.len() {
        draw_scroll_mark(img, font, bx.right() - 1, lower, Scroll::Down);
    }
}

/// The rows a list box shows: all but its two margin rows.
pub fn list_rows(bx: CellRect) -> usize {
    bx.h.saturating_sub(2) as usize
}

/// `lines` (already wrapped) in the footer box `bx` in the help ink,
/// "Spoiler:" in brown; lines past the box are dropped.
fn draw_help_lines(img: &mut MenuImage, font: &FixedCellFont, bx: CellRect, lines: &[String]) {
    for (i, line) in lines.iter().take(bx.h as usize).enumerate() {
        let mut l = Line::new(bx.w).text(1, line, Ink::Help);
        if let Some(at) = line.find(t::SPOILER) {
            l = l.text(1 + at as u32, t::SPOILER, Ink::Accent);
        }
        draw_row(img, font, bx, bx.row + i as u32, &l, false);
    }
}

/// `text` wrapped into the footer box `bx` in the help ink, "Spoiler:" in
/// brown; lines past the box are dropped.
pub(super) fn draw_help(img: &mut MenuImage, font: &FixedCellFont, bx: CellRect, text: &str) {
    draw_help_lines(img, font, bx, &t::wrap(text, (bx.w - 2) as usize));
}

/// A top-menu row: its label after the margin cell, grey when disabled.
pub fn top_line(row: &Row, w: u32) -> Line {
    let line = Line::new(w).text(1, row.label, Ink::Plain);
    if row.enabled { line } else { line.greyed() }
}

/// Cells inside a slider's brackets (3.2.7: `[#######---]`).
pub const SLIDER_INNER: u32 = 8;

/// The cells a row's value takes from [`VALUE_COL`].
fn value_cells(row: &Row, w: u32) -> u32 {
    match row.kind {
        RowKind::Check { .. } => 3,
        RowKind::Value(Shown::Word(word)) => word.len() as u32,
        // The slider, and its percent ending at the row's last text cell.
        RowKind::Value(Shown::Percent { .. }) => w - 1 - VALUE_COL,
        _ => 0,
    }
}

/// Whether a row's apply tag fits after its value, with a blank between,
/// ending at the row's last text cell; a tag that does not fit is said in
/// the footer instead.
pub fn tag_fits(row: &Row, w: u32) -> bool {
    let Some(tag) = row.tag else {
        return true;
    };
    let start = w - 1 - tag.len() as u32;
    let used = value_cells(row, w);
    VALUE_COL + used < start
}

/// The label cells a row has: from its indent to one blank before the
/// value column.
pub fn label_room(row: &Row) -> usize {
    (VALUE_COL - 2).saturating_sub(u32::from(row.indent)) as usize
}

/// A Settings row (3.2.7): a group heading in light grey; otherwise its
/// label after the margin cell and its indent ("Spoiler:" in brown), its
/// value at [`VALUE_COL`] (a check box, a word, or a slider with its
/// percent), and its apply tag ending at the last text cell when it fits.
/// The collapsed Cheats group is "+" and its heading, "Cheats" in brown.
/// A disabled row is grey.
pub fn setting_line(row: &Row, w: u32) -> Line {
    if row.kind == RowKind::Header {
        return Line::new(w).text(1, row.label, Ink::Help);
    }
    if row.label == t::CHEATS_GROUP {
        return Line::new(w)
            .text(1, "+", Ink::Plain)
            .text(3, t::CHEATS_GROUP, Ink::Plain)
            .text(3, "Cheats", Ink::Accent);
    }
    let at = 1 + u32::from(row.indent);
    let label = t::cut(row.label, label_room(row));
    let mut line = Line::new(w).text(at, &label, Ink::Plain);
    if label.starts_with(t::SPOILER) {
        line = line.text(at, t::SPOILER, Ink::Accent);
    }
    line = match row.kind {
        RowKind::Check { on } => line.check(VALUE_COL, on),
        RowKind::Value(Shown::Word(word)) => line.text(VALUE_COL, word, Ink::Plain),
        RowKind::Value(Shown::Percent { value, fill }) => line
            .slider(VALUE_COL, SLIDER_INNER, fill)
            .right(w - 2, &format!("{value}%"), Ink::Help),
        _ => line,
    };
    if let Some(tag) = row.tag
        && tag_fits(row, w)
    {
        line = line.right(w - 2, tag, Ink::Help);
    }
    if row.enabled { line } else { line.greyed() }
}

/// The footer's lines for the selected `row` of a Settings tab, wrapped to
/// `width`, at most `rows` of them: the reason a greyed row is greyed, or
/// the row's help ("Spoiler:" first on a spoiler row); then, on their own
/// lines, the apply point when the row had no room for its tag, and the
/// command-line mark. Help that does not fit gives way to those.
pub fn setting_help(
    row: Option<&Row>,
    fallback: Option<&str>,
    width: usize,
    rows: usize,
) -> Vec<String> {
    let Some(row) = row else {
        return fallback
            .map(|text| t::wrap(text, width))
            .unwrap_or_default();
    };
    let mut notes = Vec::new();
    if !tag_fits(row, width as u32 + 2) {
        notes.extend(row.tag.map(|tag| {
            if tag == t::NEXT_PROMPT {
                t::NOTE_NEXT_PROMPT
            } else {
                t::NOTE_NEXT_LAUNCH
            }
        }));
    }
    if row.by_cli {
        notes.push(t::SET_BY_COMMAND_LINE);
    }
    let text = row.reason.or(row.help).or(fallback);
    let mut lines = text.map(|text| t::wrap(text, width)).unwrap_or_default();
    let notes: Vec<String> = notes.iter().flat_map(|note| t::wrap(note, width)).collect();
    lines.truncate(rows.saturating_sub(notes.len()));
    lines.extend(notes);
    lines
}

/// The help the footer shows for the selected row: its reason when it is
/// greyed.
fn selected_reason(rows: &[Row], cursor: usize) -> Option<&'static str> {
    rows.get(cursor).and_then(|row| row.reason)
}

/// The top menu (3.2.1): the narrow panel, its rows, a footer box and the
/// "Menu" and "F12: close" tabs.
fn top_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    rows: &[Row],
    cursor: usize,
    turns_since_save: Option<u64>,
) -> Drawn {
    let rect = panel_rect(PageShape::Narrow, picture, column);
    let divider = rect.bottom() - TOP_FOOTER_ROWS - 1;
    let panel = Panel::new(rect).divider(divider);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    draw_list(
        &mut img,
        font,
        list,
        rect.row,
        divider,
        rows,
        Some(cursor),
        top_line,
    );
    match selected_reason(rows, cursor) {
        Some(reason) => draw_help(&mut img, font, footer, reason),
        // The last-save line on the footer's first row, once the session
        // has one (3.3.5 "Unsaved progress"), and the world's line on the
        // second, as the sheet has them.
        None => {
            if let Some(line) = last_save_line(turns_since_save) {
                let last = Line::new(footer.w).text(1, &line, Ink::Help);
                draw_row(&mut img, font, footer, footer.row, &last, false);
            }
            let waits = Line::new(footer.w).text(1, t::WAITING, Ink::Help);
            draw_row(&mut img, font, footer, footer.row + 1, &waits, false);
        }
    }
    draw_title_tab(&mut img, font, rect, t::MENU_TITLE);
    draw_footer_tab(&mut img, font, rect, t::KEY_CLOSE);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// The label the title layer shows for `row`: Continue names the slot it
/// loads and the game date it was saved on (3.2.4), the others their own.
pub fn title_label(row: &Row, snap: &ShellSnapshot) -> String {
    match row.action {
        RowAction::Continue(Some(at)) => match snap.slot_list.get(usize::from(at)) {
            Some(slot) => t::continue_label(&slot.info.name, &t::slot_date(slot.info.clock)),
            None => row.label.to_string(),
        },
        _ => row.label.to_string(),
    }
}

/// The title layer (3.2.4): the title menu's own frame and place, its rows
/// centred as the title menu centres its own, the selected one in the
/// title menu's tight highlight (the label and a blank either side), the
/// "Ultima V" and "F12: close" tabs.
fn title_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    rows: &[Row],
    cursor: usize,
    snap: &ShellSnapshot,
) -> Drawn {
    let rect = panel_rect(PageShape::Title, picture, column);
    let panel = Panel::new(rect);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let bx = panel.boxes()[0];
    for (i, row) in rows.iter().enumerate().take(list_rows(bx)) {
        let label = t::cut(&title_label(row, snap), (bx.w - 2) as usize);
        let len = label.len() as u32;
        let at = (bx.w - len) / 2;
        let at_row = bx.row + 1 + i as u32;
        let line = Line::new(bx.w).text(at, &label, Ink::Plain);
        let line = if row.enabled { line } else { line.greyed() };
        draw_row(&mut img, font, bx, at_row, &line, false);
        if i == cursor {
            let bar_len = len + 2;
            let mut bar = Line::new(bar_len);
            bar.cells
                .copy_from_slice(&line.cells[(at - 1) as usize..(at - 1 + bar_len) as usize]);
            draw_line(&mut img, font, bx.col + at - 1, at_row, &bar, true);
        }
    }
    draw_title_tab(&mut img, font, rect, t::TITLE_LAYER_TITLE);
    draw_footer_tab(&mut img, font, rect, t::KEY_CLOSE);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// The top menu's last-save line for `turns` since the last save or load
/// this session: `None` before the first.
pub fn last_save_line(turns: Option<u64>) -> Option<String> {
    Some(match turns? {
        0 => t::LAST_SAVE_NOW.to_string(),
        1 => t::LAST_SAVE_ONE.to_string(),
        n => t::fill(t::LAST_SAVE_TURNS, &[&n.min(t::LAST_SAVE_MAX).to_string()]),
    })
}

/// The first cell of the first preset on the preset row: after "Preset:"
/// and two blanks, as the sheet sets it.
pub const PRESET_COL: u32 = 1 + t::PRESET.len() as u32 + 2;

/// Where the preset row puts each of `words` (the presets, then "Custom"):
/// `(line, cell)`, one after another from [`PRESET_COL`] with two blanks
/// between, on to the next line at [`PRESET_COL`] when a word would pass
/// the last text cell of a `w`-cell row.
pub fn preset_places(words: &[&str], w: u32) -> Vec<(u32, u32)> {
    let mut places = Vec::with_capacity(words.len());
    let (mut line, mut col) = (0, PRESET_COL);
    for word in words {
        if col + word.len() as u32 > w - 1 && col > PRESET_COL {
            line += 1;
            col = PRESET_COL;
        }
        places.push((line, col));
        col += word.len() as u32 + 2;
    }
    places
}

/// The preset row in its box (3.2.7): "Preset:", the presets and "Custom".
/// The one in force is white, the others light grey, and "Custom" dark grey
/// while a preset is in force (it cannot be chosen). With the cursor on
/// the row the one in force is drawn in inverse video with a blank either
/// side, as a dialog's button.
fn draw_preset_row(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    labels: &[&str],
    current: Option<u8>,
    selected: bool,
) {
    let mut words: Vec<&str> = labels.to_vec();
    words.push(t::CUSTOM);
    let places = preset_places(&words, bx.w);
    let custom = words.len() - 1;
    let chosen = current.map_or(custom, usize::from);
    // The two text lines between the box's margin rows.
    let mut lines: Vec<Line> = (0..PRESET_LINES).map(|_| Line::new(bx.w)).collect();
    lines[0] = lines[0].clone().text(1, t::PRESET, Ink::Plain);
    for (i, (word, (line, col))) in words.iter().zip(&places).enumerate() {
        let ink = if i == chosen {
            Ink::Plain
        } else if i == custom {
            Ink::Grey
        } else {
            Ink::Help
        };
        if let Some(l) = lines.get_mut(*line as usize) {
            *l = l.clone().text(*col, word, ink);
        }
    }
    for (i, line) in lines.iter().enumerate() {
        draw_row(img, font, bx, bx.row + 1 + i as u32, line, false);
    }
    if selected && let Some((line, col)) = places.get(chosen).copied() {
        let len = words[chosen].len() as u32 + 2;
        let Some(row) = lines.get(line as usize) else {
            return;
        };
        let mut bar = Line::new(len);
        bar.cells
            .copy_from_slice(&row.cells[(col - 1) as usize..(col - 1 + len) as usize]);
        draw_line(img, font, bx.col + col - 1, bx.row + 1 + line, &bar, true);
    }
}

/// Text lines of the Extras tab's preset row.
pub const PRESET_LINES: u32 = 2;
/// Rows of the Extras tab's preset box: its two lines between a margin row
/// above and below, so the bar on a preset never merges with a rule.
pub const PRESET_ROWS: u32 = PRESET_LINES + 2;

/// A Settings page (3.2.7): the wide panel with the tab strip, its rows, a
/// footer box for the selected row's help and the "Esc: back" tab. The
/// Extras tab puts its preset row in a box of its own above the list.
fn settings_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    tab: usize,
    rows: &[Row],
    cursor: usize,
    presets: &[&str],
) -> Drawn {
    let rect = panel_rect(PageShape::Wide, picture, column);
    let lower = rect.bottom() - SETTINGS_FOOTER_ROWS - 1;
    let preset = match rows.first() {
        Some(Row {
            kind: RowKind::Preset { current, .. },
            ..
        }) => Some(*current),
        _ => None,
    };
    let upper = rect.row + 1 + PRESET_ROWS;
    let panel = match preset {
        Some(_) => Panel::new(rect).divider(upper).divider(lower),
        None => Panel::new(rect).divider(lower),
    };
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, footer) = (boxes[boxes.len() - 2], boxes[boxes.len() - 1]);
    let (listed, list_cursor, top) = match preset {
        Some(current) => {
            draw_preset_row(&mut img, font, boxes[0], presets, current, cursor == 0);
            (&rows[1..], cursor.checked_sub(1), upper)
        }
        None => (rows, Some(cursor), rect.row),
    };
    if listed.is_empty() {
        let line = Line::new(list.w).text(1, t::NOTHING_HERE, Ink::Help);
        draw_row(&mut img, font, list, list.row + 1, &line, false);
    }
    draw_list(
        &mut img,
        font,
        list,
        top,
        lower,
        listed,
        list_cursor,
        setting_line,
    );
    // The Audio tab's own help shows under a row with none (3.2.7).
    let fallback = (tab == SettingsTab::Audio.index()).then_some(t::AUDIO_HELP);
    let help = setting_help(
        rows.get(cursor),
        fallback,
        (footer.w - 2) as usize,
        footer.h as usize,
    );
    draw_help_lines(&mut img, font, footer, &help);
    draw_tab_strip(&mut img, font, rect, &t::SETTINGS_TABS, tab);
    draw_footer_tab(&mut img, font, rect, t::KEY_BACK);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// The page `state` of the menu for the shell in `snap`, drawn on
/// `picture` with the docked column in `column`'s state.
pub fn render_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    state: PageState,
    snap: &ShellSnapshot,
) -> Drawn {
    render_page_with(font, picture, column, state, snap, "", 0)
}

/// [`render_page`] with the name field holding `field` and its cursor at
/// the game's prompt-cursor frame `cursor_frame`.
pub fn render_page_with(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    state: PageState,
    snap: &ShellSnapshot,
    field: &str,
    cursor_frame: u64,
) -> Drawn {
    let rows = page_rows(state.page, snap);
    let cursor = settle(&rows, state.cursor);
    match state.page {
        Page::Top => top_page(font, picture, column, &rows, cursor, snap.turns_since_save),
        Page::Title => title_page(font, picture, column, &rows, cursor, snap),
        Page::Settings(tab) => {
            let presets: Vec<&str> = PRESETS.iter().map(|preset| preset.label()).collect();
            settings_page(font, picture, column, tab.index(), &rows, cursor, &presets)
        }
        Page::Save => render_slots::save_page(font, picture, column, snap, state.cursor),
        Page::Load => render_slots::load_page(font, picture, column, snap, state.cursor),
        Page::Dialog(dialog) => render_slots::dialog_page(
            font,
            picture,
            column,
            snap,
            dialog,
            state.cursor,
            field,
            cursor_frame,
        ),
    }
}

/// The sweep's state (D-M4.10): where it is going and the step it shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuSweep {
    /// True while the menu is (or is becoming) shown.
    pub open: bool,
    /// The step drawn, 1..=7, while a sweep runs; `None` when it is over:
    /// the whole menu while `open`, nothing otherwise.
    pub step: Option<u8>,
    elapsed: Duration,
}

impl MenuSweep {
    /// The menu opened: in from step 1, or back from the step a close is on.
    pub fn start_open(&mut self) {
        self.open = true;
        self.step = Some(self.step.unwrap_or(1));
        self.elapsed = Duration::ZERO;
    }

    /// The menu closed: out from step 7, or back from the step an open is
    /// on.
    pub fn start_close(&mut self) {
        self.open = false;
        self.step = Some(self.step.unwrap_or(SWEEP_STEPS - 1));
        self.elapsed = Duration::ZERO;
    }

    /// Gone at once.
    pub fn stop(&mut self) {
        *self = Self::default();
    }

    /// Whether anything of the menu shows.
    pub fn visible(&self) -> bool {
        self.open || self.step.is_some()
    }

    /// `dt` of shell frame time passed: a step every [`MENU_SWEEP_STEP`],
    /// towards 8 (the whole menu) when opening and 0 (none) when closing.
    pub fn tick(&mut self, dt: Duration) {
        if self.step.is_none() {
            self.elapsed = Duration::ZERO;
            return;
        }
        self.elapsed += dt;
        while let Some(k) = self.step
            && self.elapsed + STEP_SLACK >= MENU_SWEEP_STEP
        {
            self.elapsed = self.elapsed.saturating_sub(MENU_SWEEP_STEP);
            let next = if self.open {
                k + 1
            } else {
                k.saturating_sub(1)
            };
            self.step = (1..SWEEP_STEPS).contains(&next).then_some(next);
        }
        if self.step.is_none() {
            self.elapsed = Duration::ZERO;
        }
    }
}

/// What a drawn page was drawn from; a new one redraws it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DrawKey {
    state: PageState,
    rows: Vec<Row>,
    picture: MenuPicture,
    column_shown: bool,
    /// What the slot pages and the top menu's footer show besides rows.
    slots: SlotList,
    quicksave: bool,
    turns_since_save: Option<u64>,
    field: String,
    /// The name field's cursor glyph, while the name field shows.
    cursor_glyph: Option<u8>,
}

/// A sweep the title layer asks for. The layer opens and closes outside
/// the overlay model (D-M4.3), so it tells the renderer itself, as an
/// [`OverlayChanged`] tells it for the menu in play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleSweep {
    /// The layer opened: sweep in.
    Open,
    /// The layer closed by a key: sweep out.
    Close,
    /// The layer closed because the game moved on (a load, the idle
    /// preview): gone that frame.
    Stop,
}

/// What the menu shows and how far its sweep is. Exists while the menu
/// plugin is in the app.
#[derive(Resource, Debug, Default)]
pub struct MenuView {
    pub sweep: MenuSweep,
    /// The title layer's sweep asked for this frame.
    pub(crate) title_sweep: Option<TitleSweep>,
    /// The page last drawn, kept through the close sweep.
    drawn: Option<(DrawKey, Drawn)>,
    /// The node's picture and the draw and step it was composed from.
    image: Option<Handle<Image>>,
    composed: Option<(u64, Option<u8>)>,
    /// How many times a page was drawn, and the node's picture composed.
    pub renders: u64,
    pub composes: u64,
}

impl MenuView {
    /// The node's picture, while the menu shows.
    pub fn image(&self) -> Option<&Handle<Image>> {
        self.image.as_ref().filter(|_| self.sweep.visible())
    }

    /// The page last drawn.
    pub fn drawn(&self) -> Option<&Drawn> {
        self.drawn.as_ref().map(|(_, drawn)| drawn)
    }

    /// The picture's size, while the menu shows.
    pub fn picture_size(&self) -> Option<UVec2> {
        self.drawn()
            .filter(|_| self.sweep.visible())
            .map(|drawn| UVec2::new(drawn.image.w, drawn.image.h))
    }
}

/// The shell state the picture's size and the panel's place come from.
#[derive(SystemParam)]
pub(crate) struct PictureInputs<'w, 's> {
    visual: Option<Res<'w, VisualState>>,
    display: Option<Res<'w, Display>>,
    frame_size: Option<Res<'w, FrameImageSize>>,
    column: Option<Res<'w, ColumnState>>,
    windows: Query<'w, 's, &'static Window>,
}

impl PictureInputs<'_, '_> {
    /// The grow-left canvas the frame sprite shows this frame, if any: the
    /// one the frame fit placed (`FrameImageSize`).
    fn canvas(&self) -> Option<Canvas> {
        let size = self.frame_size.as_deref()?.0?;
        let display = self.display.as_deref()?;
        let window = self.windows.single().ok()?.physical_size();
        let state = self.visual.as_deref().map(|visual| &visual.state);
        let canvas = grow_canvas(state, window, display.0.pixel_aspect)?;
        (UVec2::new(canvas.w, canvas.h) == size).then_some(canvas)
    }

    /// The picture and the column's state.
    pub(crate) fn picture(&self) -> (MenuPicture, ColumnView) {
        let mut column = self
            .column
            .as_deref()
            .map(ColumnState::view)
            .unwrap_or_default();
        if let Some(visual) = self.visual.as_deref() {
            column.page_mode = page_mode_active(&visual.state);
        }
        (menu_picture(self.canvas().as_ref()), column)
    }
}

/// What the title layer's picture is drawn from before play.
#[derive(SystemParam)]
pub(crate) struct TitleInputs<'w> {
    font: Option<Res<'w, TitleFont>>,
    intro: Option<Res<'w, VisualIntroState>>,
}

fn rgba_image(w: u32, h: u32, rgba: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Writes `img` into the image `handle` names when it has the same size,
/// else into a new image; returns the handle in use.
fn upload(
    images: &mut Assets<Image>,
    handle: Option<Handle<Image>>,
    img: &MenuImage,
) -> Handle<Image> {
    let size = UVec2::new(img.w, img.h);
    if let Some(handle) = handle {
        if let Some(image) = images.get_mut(&handle)
            && image.size() == size
        {
            image.data = Some(img.rgba.clone());
            return handle;
        }
        images.remove(&handle);
    }
    images.add(rgba_image(img.w, img.h, img.rgba.clone()))
}

/// Starts and steps the sweep, redraws the page when what it shows
/// changed and recomposes the node's picture when the draw or the step
/// changed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn menu_render(
    mut changed: EventReader<OverlayChanged>,
    state: Res<MenuState>,
    pages: SnapshotInputs,
    inputs: PictureInputs,
    title: TitleInputs,
    time: Res<Time>,
    mut view: ResMut<MenuView>,
    mut images: ResMut<Assets<Image>>,
) {
    let view = &mut *view;
    let mut started = false;
    for change in changed.read() {
        match *change {
            OverlayChanged::Opened(OverlayId::Menu) => {
                view.sweep.start_open();
                started = true;
            }
            OverlayChanged::Closed {
                id: OverlayId::Menu,
                why,
            } => {
                if why == CloseReason::NotIdle {
                    view.sweep.stop();
                } else {
                    view.sweep.start_close();
                }
                started = true;
            }
            _ => {}
        }
    }
    if let Some(change) = view.title_sweep.take() {
        match change {
            TitleSweep::Open => view.sweep.start_open(),
            TitleSweep::Close => view.sweep.start_close(),
            TitleSweep::Stop => view.sweep.stop(),
        }
        started = true;
    }
    if !started {
        view.sweep.tick(time.delta());
    }
    // The game's font and the prompt cursor's frame in play; at the title
    // the layer's own copy of the font and the title menu's cursor passes.
    let source = match (inputs.visual.as_deref(), title.font.as_deref()) {
        (Some(visual), _) => Some((&visual.text_font, visual.prompt_cursor_frame)),
        (None, Some(font)) => Some((
            &font.0,
            title
                .intro
                .as_deref()
                .map_or(0, |intro| intro.menu_cursor_pass),
        )),
        (None, None) => None,
    };
    if let (Some(model), Some((font, cursor_frame))) = (state.model(), source) {
        let snap = pages.snapshot();
        let (picture, column) = inputs.picture();
        let current = model.current();
        let naming = matches!(current.page, Page::Dialog(Dialog::Name(_)));
        let key = DrawKey {
            state: current,
            rows: page_rows(current.page, &snap),
            picture,
            column_shown: column.shown(),
            slots: snap.slot_list.clone(),
            quicksave: snap.quicksave,
            turns_since_save: snap.turns_since_save,
            field: model.field().to_string(),
            cursor_glyph: naming.then(|| u5_runtime::prompt_cursor_glyph(cursor_frame)),
        };
        if view.drawn.as_ref().map(|(drawn_from, _)| drawn_from) != Some(&key) {
            let drawn = render_page_with(
                font,
                &picture,
                column,
                current,
                &snap,
                model.field(),
                cursor_frame,
            );
            view.drawn = Some((key, drawn));
            view.renders += 1;
        }
    }
    if !view.sweep.visible() || view.drawn.is_none() {
        view.sweep.stop();
        view.drawn = None;
        view.composed = None;
        if let Some(handle) = view.image.take() {
            images.remove(&handle);
        }
        return;
    }
    let composed = (view.renders, view.sweep.step);
    if view.composed == Some(composed) && view.image.is_some() {
        return;
    }
    let Some((_, drawn)) = view.drawn.as_ref() else {
        return;
    };
    let picture = sweep_image_over(&drawn.image, drawn.panel, view.sweep.step, CLEAR);
    let handle = view.image.take();
    view.image = Some(upload(&mut images, handle, &picture));
    view.composed = Some(composed);
    view.composes += 1;
}

/// The menu's picture over the frame sprite.
#[derive(Component, Debug)]
pub struct MenuNode;

/// The full-window dim under it.
#[derive(Component, Debug)]
pub struct MenuBackdrop;

/// The frame sprite's rectangle in window coordinates (logical pixels):
/// the 2D camera puts the world origin at the window centre, y up.
pub fn frame_rect(window: &Window, sprite: &Sprite, transform: &Transform) -> Option<Rect> {
    let size = sprite.custom_size? * transform.scale.truncate();
    let centre = Vec2::new(
        window.resolution.width() / 2.0 + transform.translation.x,
        window.resolution.height() / 2.0 - transform.translation.y,
    );
    Some(Rect::from_center_size(centre, size))
}

fn set_rect(node: &mut Node, rect: Rect) {
    let wanted = [
        Val::Px(rect.min.x),
        Val::Px(rect.min.y),
        Val::Px(rect.width()),
        Val::Px(rect.height()),
    ];
    for (field, value) in [
        &mut node.left,
        &mut node.top,
        &mut node.width,
        &mut node.height,
    ]
    .into_iter()
    .zip(wanted)
    {
        if *field != value {
            *field = value;
        }
    }
}

fn absolute(rect: Rect) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    set_rect(&mut node, rect);
    node
}

/// The full window: the backdrop's node.
fn full_window() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

/// Run condition: the menu shows, or one of its nodes is left to remove.
pub(crate) fn place_due(
    view: Res<MenuView>,
    nodes: Query<(), Or<(With<MenuNode>, With<MenuBackdrop>)>>,
) -> bool {
    view.image().is_some() || !nodes.is_empty()
}

/// Spawns, places and removes the menu node and the backdrop. The frame
/// sprite is the one showing the game (the presented or the faithful
/// image); the node takes its rectangle exactly.
#[allow(clippy::type_complexity)]
pub(crate) fn place_overlay(
    mut commands: Commands,
    view: Res<MenuView>,
    visual: Option<Res<VisualState>>,
    intro: Option<Res<VisualIntroState>>,
    present: Option<Res<PresentHost>>,
    windows: Query<&Window>,
    sprites: Query<(&Sprite, &Transform)>,
    mut nodes: Query<(Entity, &mut Node, &mut ImageNode), With<MenuNode>>,
    backdrops: Query<Entity, With<MenuBackdrop>>,
) {
    let rect = windows.single().ok().and_then(|window| {
        let presented = present.as_deref().and_then(PresentHost::presented);
        let faithful = visual.as_deref().map(|visual| &visual.image_handle);
        // Before play the frame sprite shows the intro's picture.
        let title = intro
            .as_deref()
            .and_then(|intro| intro.image_handle.as_ref());
        let frame = sprites
            .iter()
            .find(|(sprite, _)| {
                let image = Some(&sprite.image);
                image == presented || image == faithful || image == title
            })
            .or_else(|| sprites.iter().next());
        let (sprite, transform) = frame?;
        frame_rect(window, sprite, transform)
    });
    let (Some(image), Some(rect)) = (view.image().cloned(), rect) else {
        for (entity, ..) in &nodes {
            commands.entity(entity).despawn();
        }
        for entity in &backdrops {
            commands.entity(entity).despawn();
        }
        return;
    };
    let mut placed = false;
    for (entity, mut node, mut image_node) in &mut nodes {
        if placed {
            commands.entity(entity).despawn();
            continue;
        }
        set_rect(&mut node, rect);
        if image_node.image != image {
            image_node.image = image.clone();
        }
        placed = true;
    }
    if !placed {
        commands.spawn((
            Name::new("enh menu"),
            MenuNode,
            absolute(rect),
            LayoutConfig {
                use_rounding: false,
            },
            ImageNode::new(image),
            GlobalZIndex(MENU_NODE_Z),
        ));
    }
    let mut backdrop = false;
    for entity in &backdrops {
        if backdrop {
            commands.entity(entity).despawn();
        }
        backdrop = true;
    }
    if !backdrop {
        commands.spawn((
            Name::new("enh menu backdrop"),
            MenuBackdrop,
            full_window(),
            BackgroundColor(backdrop_colour()),
            GlobalZIndex(MENU_BACKDROP_Z),
        ));
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
