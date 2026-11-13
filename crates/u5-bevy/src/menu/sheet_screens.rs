//! Every screen of chapter 3.2, drawn through the frame kit with sample
//! content, for the pixel tests ([`super::frame_tests`]) and the review
//! sheet ([`super::sheet_tests`]). Test-only: the real pages are later
//! packages'. The sample content names no place, person or item of the game
//! the player could not have seen: slot names are a player's own words, the
//! places are "Britannia" and the plain words of D-M4.8.

use u5_runtime::{FixedCellFont, prompt_cursor_glyph};

use super::frame::*;
use super::geometry::{CellRect, MenuPicture, PageShape, panel_rect};
use super::text as t;
use crate::display::grow::column::ColumnView;

/// One drawn screen: its menu image (backdrop included) and its panel.
pub struct Screen {
    pub name: &'static str,
    pub image: MenuImage,
    /// The panel, or `None` for a toast (no panel, no backdrop).
    pub panel: Option<Panel>,
    /// The cell rows that hold text, per box, for the "no glyph crosses a
    /// rule" check: `(box, row)`.
    pub rows: Vec<(CellRect, u32)>,
}

/// What a screen is drawn over, for the thumbnail.
pub struct Inputs<'a> {
    pub font: &'a FixedCellFont,
    pub picture: MenuPicture,
    pub column: ColumnView,
    /// The 80x50 thumbnail of the sample slot.
    pub thumb: &'a [u8],
    /// The Avatar's name for the slot details.
    pub avatar: &'a str,
}

struct Draw<'a> {
    font: &'a FixedCellFont,
    img: MenuImage,
    panel: Panel,
    rows: Vec<(CellRect, u32)>,
}

impl<'a> Draw<'a> {
    fn new(inputs: &Inputs<'a>, panel: Panel) -> Self {
        let mut img = MenuImage::for_picture(&inputs.picture);
        draw_panel(&mut img, &panel);
        Self {
            font: inputs.font,
            img,
            panel,
            rows: Vec::new(),
        }
    }

    fn row(&mut self, bx: CellRect, row: u32, line: Line, selected: bool) {
        draw_row(&mut self.img, self.font, bx, row, &line, selected);
        self.rows.push((bx, row));
    }

    /// A plain text row with a margin cell.
    fn text(&mut self, bx: CellRect, row: u32, text: &str, ink: Ink) {
        self.row(bx, row, Line::new(bx.w).text(1, text, ink), false);
    }

    /// Help text wrapped into the footer box, "Spoiler:" in brown.
    fn help(&mut self, bx: CellRect, text: &str) {
        for (i, line) in t::wrap(text, (bx.w - 2) as usize).iter().enumerate() {
            let mut l = Line::new(bx.w).text(1, line, Ink::Help);
            if let Some(at) = line.find(t::SPOILER) {
                l = l.text(1 + at as u32, t::SPOILER, Ink::Accent);
            }
            self.row(bx, bx.row + i as u32, l, false);
        }
    }

    /// Dialog buttons centred on `row`: each label with a blank either side,
    /// the selected one in inverse video, three cells between them.
    fn buttons(&mut self, bx: CellRect, row: u32, labels: &[&str], selected: usize) {
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
        self.row(bx, row, line.clone(), false);
        let (at, len) = spans[selected];
        let mut bar = Line::new(len);
        bar.cells
            .copy_from_slice(&line.cells[at as usize..(at + len) as usize]);
        draw_line(&mut self.img, self.font, bx.col + at, row, &bar, true);
    }

    fn finish(mut self, name: &'static str) -> Screen {
        dim_backdrop(&mut self.img);
        Screen {
            name,
            image: self.img,
            panel: Some(self.panel),
            rows: self.rows,
        }
    }
}

/// The top menu: `items`, the selected one, and whether Save is greyed.
fn top_menu(
    inputs: &Inputs<'_>,
    name: &'static str,
    items: &[&str],
    selected: usize,
    save_greyed: bool,
) -> Screen {
    let rect = panel_rect(PageShape::Narrow, &inputs.picture, inputs.column);
    let divider = rect.bottom() - 3 - 1;
    let mut d = Draw::new(inputs, Panel::new(rect).divider(divider));
    let boxes = d.panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    for (i, item) in items.iter().enumerate() {
        let mut line = Line::new(list.w).text(1, item, Ink::Plain);
        if save_greyed && *item == t::SAVE {
            line = line.greyed();
        }
        d.row(list, list.row + 1 + i as u32, line, i == selected);
    }
    if save_greyed && items[selected] == t::SAVE {
        d.help(footer, t::SAVE_UNAVAILABLE);
    } else {
        d.text(
            footer,
            footer.row,
            &t::fill(t::LAST_SAVE_TURNS, &["12"]),
            Ink::Help,
        );
        d.text(footer, footer.row + 1, t::WAITING, Ink::Help);
    }
    draw_title_tab(&mut d.img, inputs.font, rect, t::MENU_TITLE);
    draw_footer_tab(&mut d.img, inputs.font, rect, t::KEY_CLOSE);
    d.finish(name)
}

const SLOTS: [(&str, &str, &str); 5] = [
    ("quick", "Britannia", "139/04/07"),
    ("Second try", "Britannia", "139/04/06"),
    ("Morning walk", "a dwelling", "139/04/02"),
    ("Old game", "", ""),
    ("From DOS", "Britannia", "139/03/29"),
];

fn save_page(inputs: &Inputs<'_>) -> Screen {
    let rect = panel_rect(PageShape::Narrow, &inputs.picture, inputs.column);
    let divider = rect.bottom() - FOOTER_ROWS - 1;
    let mut d = Draw::new(inputs, Panel::new(rect).divider(divider));
    let boxes = d.panel.boxes();
    let (list, footer) = (boxes[0], boxes[1]);
    d.row(
        list,
        list.row + 1,
        Line::new(list.w).text(1, t::NEW_SLOT, Ink::Plain),
        true,
    );
    // The damaged slot is not offered for saving over.
    for (i, (name, _, date)) in SLOTS.iter().filter(|slot| !slot.2.is_empty()).enumerate() {
        let room = (list.w - 2 - 1 - date.len() as u32) as usize;
        let line = Line::new(list.w)
            .text(1, &t::cut(name, room), Ink::Plain)
            .right(list.w - 2, date, Ink::Help);
        d.row(list, list.row + 2 + i as u32, line, false);
    }
    d.help(footer, t::SAVE_FOOTER);
    draw_title_tab(&mut d.img, inputs.font, rect, t::SAVE_TITLE);
    draw_footer_tab(&mut d.img, inputs.font, rect, t::KEY_BACK);
    d.finish("save")
}

fn dialog<'a>(inputs: &Inputs<'a>, lines: u32) -> Draw<'a> {
    let rect = panel_rect(PageShape::Dialog { lines }, &inputs.picture, inputs.column);
    Draw::new(inputs, Panel::new(rect))
}

fn name_dialog(inputs: &Inputs<'_>) -> Screen {
    let mut d = dialog(inputs, 5);
    let bx = d.panel.boxes()[0];
    d.text(bx, bx.row + 1, t::NAME_PROMPT, Ink::Plain);
    // The input line: the faithful cap, the typed name, the barber-pole
    // cursor, as the message window's input line.
    let typed = "Morning walk";
    let line = Line::new(bx.w).text(2, typed, Ink::Plain).glyph(
        2 + typed.len() as u32,
        prompt_cursor_glyph(3),
        Ink::Plain,
    );
    d.row(bx, bx.row + 3, line, false);
    draw_input_cap(&mut d.img, inputs.font, bx.col + 1, bx.row + 3);
    draw_title_tab(&mut d.img, inputs.font, d.panel.rect, t::SAVE_TITLE);
    d.finish("save-name")
}

fn overwrite_dialog(inputs: &Inputs<'_>) -> Screen {
    let mut d = dialog(inputs, 5);
    let bx = d.panel.boxes()[0];
    d.text(
        bx,
        bx.row + 1,
        &t::fill(t::OVERWRITE, &["Second try"]),
        Ink::Plain,
    );
    d.buttons(bx, bx.row + 3, &[t::YES, t::NO], 1);
    d.finish("save-overwrite")
}

fn unsaved_dialog(inputs: &Inputs<'_>) -> Screen {
    let mut d = dialog(inputs, 7);
    let bx = d.panel.boxes()[0];
    d.text(
        bx,
        bx.row + 1,
        &t::fill(t::LOAD_ASK, &["Second try"]),
        Ink::Plain,
    );
    let warning = t::fill(t::UNSAVED_TURNS, &["37"]);
    for (i, line) in t::wrap(&warning, t::DIALOG_TEXT).iter().enumerate() {
        d.text(bx, bx.row + 2 + i as u32, line, Ink::Plain);
    }
    d.buttons(bx, bx.row + 5, &[t::LOAD_BUTTON, t::CANCEL], 0);
    d.finish("load-unsaved")
}

fn delete_dialog(inputs: &Inputs<'_>) -> Screen {
    let mut d = dialog(inputs, 6);
    let bx = d.panel.boxes()[0];
    d.text(
        bx,
        bx.row + 1,
        &t::fill(t::DELETE, &["Old game"]),
        Ink::Plain,
    );
    d.text(bx, bx.row + 2, t::DELETE_WARNING, Ink::Plain);
    d.buttons(bx, bx.row + 4, &[t::DELETE_BUTTON, t::CANCEL], 1);
    d.finish("load-delete")
}

/// The Load page with slot `selected` (0 quick, 3 damaged, 4 imported).
fn load_page(inputs: &Inputs<'_>, name: &'static str, selected: usize) -> Screen {
    let rect = panel_rect(PageShape::Wide, &inputs.picture, inputs.column);
    let divider = rect.bottom() - 2 - 1;
    let split = rect.col + 20;
    let panel = Panel::new(rect)
        .divider(divider)
        .split(split, rect.row, divider);
    let mut d = Draw::new(inputs, panel);
    let boxes = d.panel.boxes();
    let (list, details, footer) = (boxes[0], boxes[1], boxes[2]);
    for (i, (slot, _, _)) in SLOTS.iter().enumerate() {
        let mut line = Line::new(list.w).text(1, slot, Ink::Plain);
        if i == 3 {
            line = line.right(list.w - 2, t::DAMAGED, Ink::Plain).greyed();
        }
        if i == 2 {
            line = line.right(list.w - 2, t::CHEATS_MARK, Ink::Accent);
        }
        d.row(list, list.row + 1 + i as u32, line, i == selected);
    }
    let thumb_col = details.col + (details.w - THUMB_CELLS.0) / 2;
    let thumb = match selected {
        3 => Thumb::Empty,
        4 => Thumb::Imported,
        _ => Thumb::Picture(inputs.thumb),
    };
    draw_thumbnail_box(&mut d.img, inputs.font, thumb_col, details.row + 1, thumb);
    let first = details.row + 1 + THUMB_CELLS.1 + 1;
    let (_, place, date) = SLOTS[selected];
    if selected == 3 {
        d.text(details, first, t::DAMAGED, Ink::Grey);
    } else {
        let turn = t::fill(t::TURN, &["18234"]);
        let when = format!("{date} 09:15");
        let lines: [(&str, Ink); 4] = [
            (inputs.avatar, Ink::Plain),
            (place, Ink::Help),
            (&when, Ink::Help),
            (&turn, Ink::Help),
        ];
        for (i, (text, ink)) in lines.iter().enumerate() {
            d.text(details, first + i as u32, text, *ink);
        }
    }
    let keys = Line::new(footer.w).text(
        (footer.w - t::LOAD_KEYS.len() as u32) / 2,
        t::LOAD_KEYS,
        Ink::Help,
    );
    d.row(footer, footer.row, keys, false);
    // The title over the list, clear of the split's junction.
    let over_list = CellRect::new(rect.col, rect.row, split - rect.col, 1);
    draw_title_tab(&mut d.img, inputs.font, over_list, t::LOAD_TITLE);
    draw_footer_tab(&mut d.img, inputs.font, rect, t::KEY_BACK);
    d.finish(name)
}

/// A Settings row's line: label (indented), then a value at the value
/// column, then an apply tag.
#[derive(Clone, Copy)]
enum Value {
    None,
    Check(bool),
    Slider(u8),
    Text(&'static str),
}

struct SettingRow {
    label: &'static str,
    indent: u32,
    value: Value,
    tag: Option<&'static str>,
    greyed: bool,
    header: bool,
}

/// Rows of a footer box: the text and one blank row above the footer tab.
const FOOTER_ROWS: u32 = 4;

const fn row(label: &'static str, value: Value) -> SettingRow {
    SettingRow {
        label,
        indent: 0,
        value,
        tag: None,
        greyed: false,
        header: false,
    }
}

const fn header(label: &'static str) -> SettingRow {
    SettingRow {
        header: true,
        ..row(label, Value::None)
    }
}

const VALUE_COL: u32 = 1 + t::SETTINGS_LABEL as u32 + 1;
const SLIDER_INNER: u32 = 8;

fn setting_line(w: u32, r: &SettingRow) -> Line {
    if r.header {
        return Line::new(w).text(1, r.label, Ink::Help);
    }
    let mut line = Line::new(w).text(1 + r.indent, r.label, Ink::Plain);
    if let Some(at) = r.label.find(t::SPOILER) {
        line = line.text(1 + r.indent + at as u32, t::SPOILER, Ink::Accent);
    }
    if let Some(at) = r.label.find("Cheats") {
        line = line.text(1 + r.indent + at as u32, "Cheats", Ink::Accent);
    }
    line = match r.value {
        Value::None => line,
        Value::Check(on) => line.check(VALUE_COL, on),
        Value::Slider(p) => {
            line.slider(VALUE_COL, SLIDER_INNER, p)
                .right(w - 2, &format!("{p}%"), Ink::Help)
        }
        Value::Text(text) => line.text(VALUE_COL, text, Ink::Plain),
    };
    if let Some(tag) = r.tag {
        line = line.right(w - 2, tag, Ink::Help);
    }
    if r.greyed { line.greyed() } else { line }
}

fn settings_page(inputs: &Inputs<'_>, name: &'static str, tab: usize) -> Screen {
    let rect = panel_rect(PageShape::Wide, &inputs.picture, inputs.column);
    let lower = rect.bottom() - FOOTER_ROWS - 1;
    let enh = tab == 3;
    let upper = rect.row + 3;
    let panel = if enh {
        Panel::new(rect).divider(upper).divider(lower)
    } else {
        Panel::new(rect).divider(lower)
    };
    let mut d = Draw::new(inputs, panel);
    let boxes = d.panel.boxes();
    let (list, footer) = (boxes[boxes.len() - 2], boxes[boxes.len() - 1]);
    let tagged = |mut r: SettingRow, tag| {
        r.tag = Some(tag);
        r
    };
    let indented = |mut r: SettingRow| {
        r.indent += 2;
        r
    };
    let grouped = |mut r: SettingRow| {
        r.indent += 1;
        r
    };
    let (rows, selected, help): (Vec<SettingRow>, usize, &str) = match tab {
        0 => (
            vec![
                row(t::VOLUME, Value::Slider(70)),
                row(t::MUTE, Value::Check(false)),
                row(t::MUTE_ON_FOCUS_LOSS, Value::Check(true)),
            ],
            0,
            t::AUDIO_HELP,
        ),
        1 => (
            vec![
                row("Wait for vsync", Value::Check(true)),
                row("Scale mode", Value::Text("fit")),
                row("Fullscreen key (F11)", Value::Check(false)),
                tagged(row("Wider world view", Value::Check(true)), t::NEXT_LAUNCH),
                indented(row("Messages over map", Value::Check(true))),
                indented(row("Typed text in cyan", Value::Check(true))),
                indented(row("Health colours", Value::Check(true))),
                indented(row("Fade effects", Value::Check(true))),
                indented(row("Keep party centred", Value::Check(false))),
                row("Brightness", Value::Slider(100)),
                row("Sprites over terrain", Value::Check(true)),
            ],
            3,
            "Shows more of the world beside the original view. Ctrl+P opens the side column.",
        ),
        2 => (
            vec![
                row(t::SOUND, Value::Text(t::ON)),
                row(t::BUFFER, Value::Text(t::OFF)),
                row(t::START_SOUND_OFF, Value::Check(false)),
                row(t::START_BUFFER_ON, Value::Check(false)),
                row(t::CONFIRM_UNSAVED, Value::Check(true)),
                SettingRow {
                    greyed: true,
                    ..row(t::PACING, Value::Text(t::RESERVED))
                },
            ],
            0,
            "Choosing this closes the menu and types Ctrl-S; the game prints its own line.",
        ),
        _ => (
            vec![
                header("Display"),
                grouped(indented(row("Sprites outlined", Value::Check(true)))),
                header("Maps"),
                grouped(tagged(
                    row("Enhanced gem map", Value::Check(true)),
                    t::NEXT_PROMPT,
                )),
                grouped(indented(row("Spoiler: show all", Value::Check(false)))),
                header("Menu and saves"),
                grouped(tagged(
                    row("Menu overlay (F12)", Value::Check(true)),
                    t::NEXT_LAUNCH,
                )),
                grouped(tagged(
                    row("Title menu (F12)", Value::Check(true)),
                    t::NEXT_LAUNCH,
                )),
                grouped(tagged(
                    row("Named save slots", Value::Check(true)),
                    t::NEXT_PROMPT,
                )),
                grouped(indented(row("Quick slot", Value::Check(true)))),
                header("Audio"),
                grouped(row(t::MUTE_ON_FOCUS_LOSS, Value::Check(true))),
                header("Combat"),
                grouped(row("Fight for me", Value::Check(false))),
                header(t::CHEATS_GROUP),
            ],
            4,
            "Spoiler: the gem map shows the whole land, places thou hast not found too.",
        ),
    };
    // A scrolled list on the Enhancements tab: start one row in, and as
    // many as the box holds below its margin row.
    let first = usize::from(enh);
    let visible = (list.h - 1) as usize;
    for (i, r) in rows.iter().skip(first).take(visible).enumerate() {
        let index = i + first;
        let mut line = setting_line(list.w, r);
        if index == rows.len() - 1 && enh {
            // The collapsed cheats group: "+ Cheats (...)", the word brown.
            line = Line::new(list.w)
                .text(1, "+", Ink::Plain)
                .text(3, t::CHEATS_GROUP, Ink::Plain)
                .text(3, "Cheats", Ink::Accent);
        }
        d.row(list, list.row + 1 + i as u32, line, index == selected);
    }
    if enh {
        let preset = boxes[0];
        let line = Line::new(preset.w)
            .text(1, t::PRESET, Ink::Plain)
            .text(10, "Classic", Ink::Help)
            .text(19, "Recommended", Ink::Help);
        d.row(preset, preset.row, line, false);
        let line =
            Line::new(preset.w)
                .text(10, "Everything", Ink::Help)
                .text(22, t::CUSTOM, Ink::Plain);
        d.row(preset, preset.row + 1, line, false);
        draw_scroll_mark(&mut d.img, inputs.font, list.right() - 1, upper, Scroll::Up);
        if rows.len() - first > visible {
            draw_scroll_mark(
                &mut d.img,
                inputs.font,
                list.right() - 1,
                lower,
                Scroll::Down,
            );
        }
    }
    d.help(footer, help);
    draw_tab_strip(&mut d.img, inputs.font, rect, &t::SETTINGS_TABS, tab);
    draw_footer_tab(&mut d.img, inputs.font, rect, t::KEY_BACK);
    d.finish(name)
}

fn title_layer(inputs: &Inputs<'_>, name: &'static str, slots: bool) -> Screen {
    let rect = panel_rect(PageShape::Title, &inputs.picture, inputs.column);
    let mut d = Draw::new(inputs, Panel::new(rect));
    let bx = d.panel.boxes()[0];
    let cont = t::continue_label("Second try", "139/04/06");
    let items: Vec<&str> = if slots {
        vec![&cont, t::LOAD, t::SETTINGS, t::QUIT]
    } else {
        vec![t::CONTINUE, t::SETTINGS, t::QUIT]
    };
    // Centred labels, the selected one inverted with a blank either side,
    // as the title menu draws its own.
    for (i, item) in items.iter().enumerate() {
        let at = (bx.w - item.len() as u32) / 2;
        let row = bx.row + 1 + i as u32;
        let line = Line::new(bx.w).text(at, item, Ink::Plain);
        d.row(bx, row, line.clone(), false);
        if i == 0 {
            let len = item.len() as u32 + 2;
            let mut bar = Line::new(len);
            bar.cells
                .copy_from_slice(&line.cells[(at - 1) as usize..(at - 1 + len) as usize]);
            draw_line(&mut d.img, inputs.font, bx.col + at - 1, row, &bar, true);
        }
    }
    draw_title_tab(&mut d.img, inputs.font, rect, t::TITLE_LAYER_TITLE);
    draw_footer_tab(&mut d.img, inputs.font, rect, t::KEY_CLOSE);
    d.finish(name)
}

/// A toast as the foundation draws it today: the strip of light grey on
/// black over the picture's top row, centred.
fn toast(inputs: &Inputs<'_>, name: &'static str, text: &str) -> Screen {
    let (w, h, strip) = crate::enh::toast::toast_strip_rgba(inputs.font, text);
    let mut img = MenuImage::for_picture(&inputs.picture);
    let x0 = (img.w.saturating_sub(w)) / 2;
    for y in 0..h {
        for x in 0..w.min(img.w) {
            let i = ((y * w + x) * 4) as usize;
            img.set(
                x0 + x,
                y,
                [strip[i], strip[i + 1], strip[i + 2], strip[i + 3]],
            );
        }
    }
    Screen {
        name,
        image: img,
        panel: None,
        rows: Vec::new(),
    }
}

/// A toast proposed as a tab on the picture's top band (row 0 on the
/// frame, the tab row on a canvas), centred like the sky tab.
fn toast_tab(inputs: &Inputs<'_>, name: &'static str, text: &str) -> Screen {
    let mut img = MenuImage::for_picture(&inputs.picture);
    let row = if inputs.picture.canvas.is_some() {
        1
    } else {
        0
    };
    let col = (inputs.picture.cols() - tab_cells(text.len())) / 2;
    draw_label_tab(&mut img, inputs.font, col, row, text, INK);
    Screen {
        name,
        image: img,
        panel: None,
        rows: Vec::new(),
    }
}

/// Every screen of 3.2 on `inputs.picture`.
pub fn all_screens(inputs: &Inputs<'_>) -> Vec<Screen> {
    let full = [
        t::RESUME,
        t::SAVE,
        t::LOAD,
        t::JOURNAL,
        t::MAP,
        t::SETTINGS,
        t::CHEATS,
        t::QUIT_TO_TITLE,
        t::QUIT_TO_DESKTOP,
    ];
    let saved = t::fill(t::SAVED_TO, &["Second try"]);
    let failed = t::fill(t::SLOT_WRITE_FAILED, &["Morning walk", "disk full"]);
    vec![
        top_menu(inputs, "top", &full, 0, false),
        top_menu(
            inputs,
            "top-visual",
            &[t::RESUME, t::SETTINGS, t::QUIT_TO_DESKTOP],
            1,
            false,
        ),
        top_menu(inputs, "top-save-greyed", &full, 1, true),
        save_page(inputs),
        name_dialog(inputs),
        overwrite_dialog(inputs),
        load_page(inputs, "load", 0),
        load_page(inputs, "load-damaged", 3),
        load_page(inputs, "load-imported", 4),
        delete_dialog(inputs),
        unsaved_dialog(inputs),
        settings_page(inputs, "settings-audio", 0),
        settings_page(inputs, "settings-display", 1),
        settings_page(inputs, "settings-game", 2),
        settings_page(inputs, "settings-enhancements", 3),
        title_layer(inputs, "title-slots", true),
        title_layer(inputs, "title-no-slots", false),
        toast(inputs, "toast-saved", &saved),
        toast(inputs, "toast-prompt", t::OPENS_AT_PROMPT),
        toast(inputs, "toast-failed", &failed),
        toast_tab(inputs, "toast-tab-saved", &saved),
    ]
}
