//! The slot pages on screen: Save, Load and their dialogs (Phase B design
//! 3.2.2, 3.2.3; build plan M4-11), drawn with the M4-01 frame kit exactly
//! as the review sheet lays them out (`sheet_screens`):
//!
//! - **Save** is a narrow page: its rows, then a four-row footer box with
//!   the reminder that a slot save is a real `Q` save. A slot's row is its
//!   name, cut to fit, and its game date ending at the last text cell.
//! - **Load** is a wide page split in two: the list on the left (the name;
//!   "Damaged" greyed, or the brown "C" of a slot saved with cheats, ending
//!   at the list's last text cell), and on the right the selected slot's
//!   thumbnail in its ruled box, then the Avatar, the place, the date and
//!   time and the turn. A footer box names the keys.
//! - **Dialogs** are the kit's dialog panel, centred: the name field with
//!   the faithful input cap and the game's own turning cursor, the two
//!   questions with their buttons, and "Loading...".
//!
//! Lists scroll as the Settings pages' do. Every line sits on the cell grid
//! of its box; nothing is drawn on a rule.

use u5_runtime::{FixedCellFont, prompt_cursor_glyph};

use super::frame::*;
use super::geometry::{CellRect, MenuPicture, PageShape, panel_rect};
use super::pages::{
    Dialog, Leave, NameFor, SaveChoice, ShellSnapshot, SlotEntry, quick_slot, save_choices,
};
use super::render::{Drawn, draw_help, draw_lines};
use super::text as t;
use crate::display::grow::column::ColumnView;

/// Rows of the Save page's footer box: its help and a blank row above the
/// footer tab, as the sheet has it.
pub const SAVE_FOOTER_ROWS: u32 = 4;
/// Rows of the Load page's footer box: the keys line and a blank row.
pub const LOAD_FOOTER_ROWS: u32 = 2;
/// The Load page's split: this many cells in from the panel's left edge,
/// leaving the list 19 cells and the details 18.
pub const LOAD_SPLIT: u32 = 20;

/// A Save page row: the name cut to leave a blank and the date at the
/// end, the date in the help ink.
fn save_line(name: &str, date: Option<&str>, w: u32) -> Line {
    let Some(date) = date else {
        return Line::new(w).text(1, &t::cut(name, (w - 2) as usize), Ink::Plain);
    };
    let room = (w - 2 - 1 - date.len() as u32) as usize;
    Line::new(w)
        .text(1, &t::cut(name, room), Ink::Plain)
        .right(w - 2, date, Ink::Help)
}

/// The Save page (3.2.2) with its row `cursor` selected.
pub fn save_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    snap: &ShellSnapshot,
    cursor: usize,
) -> Drawn {
    let rect = panel_rect(PageShape::Narrow, picture, column);
    let divider = rect.bottom() - SAVE_FOOTER_ROWS - 1;
    let panel = Panel::new(rect).divider(divider);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    let choices = save_choices(snap);
    let lines: Vec<Line> = choices
        .iter()
        .map(|choice| match *choice {
            SaveChoice::New => Line::new(list.w).text(1, t::NEW_SLOT, Ink::Plain),
            SaveChoice::Quick => {
                let date = quick_slot(snap).map(|slot| t::slot_date(slot.info.clock));
                save_line(t::QUICK_SLOT, date.as_deref(), list.w)
            }
            SaveChoice::Existing(at) => {
                let slot = &snap.slot_list[usize::from(at)];
                let date = t::slot_date(slot.info.clock);
                save_line(&slot.info.name, Some(&date), list.w)
            }
        })
        .collect();
    let cursor = cursor.min(lines.len().saturating_sub(1));
    draw_lines(
        &mut img,
        font,
        list,
        rect.row,
        divider,
        &lines,
        Some(cursor),
    );
    draw_help(&mut img, font, footer, t::SAVE_FOOTER);
    draw_title_tab(&mut img, font, rect, t::SAVE_TITLE);
    draw_footer_tab(&mut img, font, rect, t::KEY_BACK);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// A Load page row: the name, and at the end "Damaged" (the whole row
/// greyed) or the brown cheats mark.
fn load_line(slot: &SlotEntry, w: u32) -> Line {
    let mark = if slot.info.damaged.is_some() {
        Some((t::DAMAGED, Ink::Plain))
    } else if slot.info.modified {
        Some((t::CHEATS_MARK, Ink::Accent))
    } else {
        None
    };
    let room = match mark {
        Some((mark, _)) => (w - 2 - 1 - mark.len() as u32) as usize,
        None => (w - 2) as usize,
    };
    let mut line = Line::new(w).text(1, &t::cut(&slot.info.name, room), Ink::Plain);
    if let Some((mark, ink)) = mark {
        line = line.right(w - 2, mark, ink);
    }
    if slot.info.damaged.is_some() {
        line.greyed()
    } else {
        line
    }
}

/// The lines under a slot's thumbnail (3.2.3), each with its ink: the
/// Avatar, the place in published words (D-M4.8), the game's date and time
/// and the turn (not for an import), and the cheats mark; only "Damaged"
/// for a damaged slot. Nothing else: no quest progress (1.4.7).
pub fn detail_lines(slot: &SlotEntry) -> Vec<(String, Ink)> {
    let mut lines: Vec<(String, Ink)> = Vec::new();
    if slot.info.damaged.is_some() {
        lines.push((t::DAMAGED.to_string(), Ink::Grey));
        return lines;
    }
    lines.push((slot.info.avatar.clone(), Ink::Plain));
    lines.push((slot.info.place.clone(), Ink::Help));
    lines.push((t::slot_when(slot.info.clock), Ink::Help));
    if !slot.info.imported {
        lines.push((t::fill(t::TURN, &[&slot.info.turn.to_string()]), Ink::Help));
    }
    if slot.info.modified {
        lines.push((t::SAVED_WITH_CHEATS.to_string(), Ink::Accent));
    }
    lines
}

/// The details of `slot` in the box `bx` (3.2.3): the thumbnail box
/// centred under the box's margin row, then a blank row, then the lines.
/// No quest progress is ever shown.
fn draw_details(img: &mut MenuImage, font: &FixedCellFont, bx: CellRect, slot: &SlotEntry) {
    let damaged = slot.info.damaged.is_some();
    let thumb = match (&slot.thumb, slot.info.imported) {
        _ if damaged => Thumb::Empty,
        (Some(rgba), _) => Thumb::Picture(rgba),
        (None, true) => Thumb::Imported,
        (None, false) => Thumb::Empty,
    };
    let thumb_col = bx.col + (bx.w - THUMB_CELLS.0) / 2;
    draw_thumbnail_box(img, font, thumb_col, bx.row + 1, thumb);
    let first = bx.row + 1 + THUMB_CELLS.1 + 1;
    let room = (bx.w - 2) as usize;
    let lines = detail_lines(slot);
    for (i, (text, ink)) in lines.iter().enumerate() {
        let row = first + i as u32;
        if row > bx.bottom() {
            break;
        }
        let line = Line::new(bx.w).text(1, &t::cut(text, room), *ink);
        draw_row(img, font, bx, row, &line, false);
    }
}

/// The Load page (3.2.3) with slot `cursor` selected.
pub fn load_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    snap: &ShellSnapshot,
    cursor: usize,
) -> Drawn {
    let rect = panel_rect(PageShape::Wide, picture, column);
    let divider = rect.bottom() - LOAD_FOOTER_ROWS - 1;
    let split = rect.col + LOAD_SPLIT;
    let panel = Panel::new(rect)
        .divider(divider)
        .split(split, rect.row, divider);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    let boxes = panel.boxes();
    let (list, details, footer) = (boxes[0], boxes[1], boxes[2]);
    let slots = &snap.slot_list;
    if slots.is_empty() {
        let line = Line::new(list.w).text(1, t::NO_SLOTS, Ink::Help);
        draw_row(&mut img, font, list, list.row + 1, &line, false);
    } else {
        let cursor = cursor.min(slots.len() - 1);
        let lines: Vec<Line> = slots.iter().map(|slot| load_line(slot, list.w)).collect();
        draw_lines(
            &mut img,
            font,
            list,
            rect.row,
            divider,
            &lines,
            Some(cursor),
        );
        draw_details(&mut img, font, details, &slots[cursor]);
    }
    let keys = Line::new(footer.w).text(
        (footer.w - t::LOAD_KEYS.len() as u32) / 2,
        t::LOAD_KEYS,
        Ink::Help,
    );
    draw_row(&mut img, font, footer, footer.row, &keys, false);
    // The title over the list, clear of the split's junction.
    let over_list = CellRect::new(rect.col, rect.row, split - rect.col, 1);
    draw_title_tab(&mut img, font, over_list, t::LOAD_TITLE);
    draw_footer_tab(&mut img, font, rect, t::KEY_BACK);
    Drawn {
        image: img,
        panel: rect,
    }
}

/// A dialog's panel of `lines` interior rows, drawn, with its box.
fn dialog_panel(picture: &MenuPicture, column: ColumnView, lines: u32) -> (MenuImage, Panel) {
    let rect = panel_rect(PageShape::Dialog { lines }, picture, column);
    let panel = Panel::new(rect);
    let mut img = MenuImage::for_picture(picture);
    draw_panel(&mut img, &panel);
    (img, panel)
}

/// Dialog buttons centred on `row` of `bx`: each label with a blank either
/// side, the `selected` one in inverse video, three cells between them.
pub fn draw_buttons(
    img: &mut MenuImage,
    font: &FixedCellFont,
    bx: CellRect,
    row: u32,
    labels: &[&str],
    selected: usize,
) {
    let total: u32 =
        labels.iter().map(|l| l.len() as u32 + 2).sum::<u32>() + 3 * (labels.len() as u32 - 1);
    let mut col = (bx.w - total) / 2;
    let mut line = Line::new(bx.w);
    let mut spans = Vec::new();
    for label in labels {
        line = line.text(col + 1, label, Ink::Plain);
        spans.push((col, label.len() as u32 + 2));
        col += label.len() as u32 + 5;
    }
    draw_row(img, font, bx, row, &line, false);
    if let Some(&(at, len)) = spans.get(selected) {
        let mut bar = Line::new(len);
        bar.cells
            .copy_from_slice(&line.cells[at as usize..(at + len) as usize]);
        draw_line(img, font, bx.col + at, row, &bar, true);
    }
}

/// A question with two buttons (3.2.2, 3.2.3): its lines from the box's
/// second row, a blank row, the buttons, and a margin row.
fn question(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    text: &[String],
    buttons: [&str; 2],
    selected: usize,
) -> Drawn {
    let lines = text.len() as u32 + 4;
    let (mut img, panel) = dialog_panel(picture, column, lines);
    let bx = panel.boxes()[0];
    for (i, line) in text.iter().enumerate() {
        let l = Line::new(bx.w).text(1, line, Ink::Plain);
        draw_row(&mut img, font, bx, bx.row + 1 + i as u32, &l, false);
    }
    draw_buttons(
        &mut img,
        font,
        bx,
        bx.row + 2 + text.len() as u32,
        &buttons,
        selected,
    );
    Drawn {
        image: img,
        panel: panel.rect,
    }
}

/// The name field (3.2.2): the prompt, then the input line with the
/// faithful input cap, the typed `field` and the game's turning cursor
/// (`cursor_frame` is the game's own prompt cursor's frame).
fn name_dialog(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    purpose: NameFor,
    field: &str,
    cursor_frame: u64,
) -> Drawn {
    let (mut img, panel) = dialog_panel(picture, column, 5);
    let bx = panel.boxes()[0];
    let (prompt, title) = match purpose {
        NameFor::New => (t::NAME_PROMPT, t::SAVE_TITLE),
        NameFor::Rename(_) => (t::RENAME_PROMPT, t::LOAD_TITLE),
    };
    let line = Line::new(bx.w).text(1, prompt, Ink::Plain);
    draw_row(&mut img, font, bx, bx.row + 1, &line, false);
    let input = Line::new(bx.w).text(2, field, Ink::Plain).glyph(
        2 + field.len() as u32,
        prompt_cursor_glyph(cursor_frame),
        Ink::Plain,
    );
    draw_row(&mut img, font, bx, bx.row + 3, &input, false);
    draw_input_cap(&mut img, font, bx.col + 1, bx.row + 3);
    draw_title_tab(&mut img, font, panel.rect, title);
    Drawn {
        image: img,
        panel: panel.rect,
    }
}

/// "Loading..." centred in a three-row dialog.
fn loading_dialog(font: &FixedCellFont, picture: &MenuPicture, column: ColumnView) -> Drawn {
    let (mut img, panel) = dialog_panel(picture, column, 3);
    let bx = panel.boxes()[0];
    let at = (bx.w - t::LOADING.len() as u32) / 2;
    let line = Line::new(bx.w).text(at, t::LOADING, Ink::Plain);
    draw_row(&mut img, font, bx, bx.row + 1, &line, false);
    Drawn {
        image: img,
        panel: panel.rect,
    }
}

/// The name of the slot at `at`, for a dialog's question.
fn slot_name(snap: &ShellSnapshot, at: u16) -> &str {
    snap.slot_list
        .get(usize::from(at))
        .map_or("", |slot| slot.info.name.as_str())
}

/// The dialog `dialog` with button `cursor` chosen.
#[allow(clippy::too_many_arguments)]
pub fn dialog_page(
    font: &FixedCellFont,
    picture: &MenuPicture,
    column: ColumnView,
    snap: &ShellSnapshot,
    dialog: Dialog,
    cursor: usize,
    field: &str,
    cursor_frame: u64,
) -> Drawn {
    match dialog {
        Dialog::Name(purpose) => name_dialog(font, picture, column, purpose, field, cursor_frame),
        Dialog::Overwrite(at) => {
            let text = t::wrap(
                &t::fill(t::OVERWRITE, &[slot_name(snap, at)]),
                t::DIALOG_TEXT,
            );
            question(font, picture, column, &text, [t::YES, t::NO], cursor)
        }
        Dialog::Delete(at) => {
            let mut text = t::wrap(&t::fill(t::DELETE, &[slot_name(snap, at)]), t::DIALOG_TEXT);
            text.extend(t::wrap(t::DELETE_WARNING, t::DIALOG_TEXT));
            question(
                font,
                picture,
                column,
                &text,
                [t::DELETE_BUTTON, t::CANCEL],
                cursor,
            )
        }
        Dialog::Loading(_) => loading_dialog(font, picture, column),
        Dialog::Unsaved(what) => {
            let (ask, buttons) = match what {
                Leave::Load(at) => (
                    t::fill(t::LOAD_ASK, &[slot_name(snap, at)]),
                    [t::LOAD_BUTTON, t::CANCEL],
                ),
                Leave::Title => (t::QUIT_TO_TITLE_ASK.to_string(), [t::YES, t::NO]),
                Leave::Desktop => (t::QUIT_TO_DESKTOP_ASK.to_string(), [t::YES, t::NO]),
            };
            let mut text = t::wrap(&ask, t::DIALOG_TEXT);
            if let Some(unsaved) = snap.unsaved {
                text.extend(t::wrap(&t::unsaved_line(unsaved), t::DIALOG_TEXT));
            }
            question(font, picture, column, &text, buttons, cursor)
        }
    }
}
