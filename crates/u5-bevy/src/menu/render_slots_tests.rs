//! The slot pages' pixels (plan M4-11 render check): each page and dialog
//! the shell draws is, byte for byte, the M4-01 review sheet's screen of
//! the same content, at 4:3 and on every grow-left canvas, the column shut
//! and open; and with the longest names and a list longer than the box,
//! every pixel is still the kit's on the cell grid.

use std::sync::Arc;

use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, parse_ch_font};

use super::frame::*;
use super::geometry::{CellRect, MenuPicture, menu_picture};
use super::host::{SlotId, SlotInfo};
use super::pages::{Dialog, NameFor, ShellSnapshot, SlotEntry};
use super::render_slots::{dialog_page, load_page, save_page};
use super::sheet_screens::{Inputs, all_screens};
use super::text as t;
use crate::display::grow::Canvas;
use crate::display::grow::column::ColumnView;
use u5_runtime::enhancements::PixelAspect;

/// Every glyph a distinct pattern, the two caps the faithful triangles.
fn font() -> u5_runtime::FixedCellFont {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    let right: [u8; 8] = [0x80, 0xe0, 0xf8, 0xfc, 0xfc, 0xf8, 0xe0, 0x80];
    for (row, bits) in right.iter().enumerate() {
        bytes[2 * 8 + row] = *bits;
        bytes[8 + row] = bits.reverse_bits();
    }
    for code in 0x05..0x80usize {
        if code == 0x20 {
            continue;
        }
        for row in 0..7 {
            bytes[code * 8 + row] = ((code * 37 + row * 11) & 0xfe) as u8 | 0x40;
        }
    }
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

/// The 320x200 frame and every distinct grow-left canvas.
fn pictures() -> Vec<MenuPicture> {
    let mut out = vec![MenuPicture::FRAME];
    let mut seen: Vec<(u32, u32)> = Vec::new();
    for aspect in [PixelAspect::Dos, PixelAspect::Square] {
        for h in [720, 960, 1080, 1200, 1440, 2160] {
            for w in (960..=3840).step_by(80) {
                if let Some(c) = Canvas::for_window(w, h, aspect)
                    && !seen.contains(&(c.w, c.h))
                {
                    seen.push((c.w, c.h));
                    out.push(menu_picture(Some(&c)));
                }
            }
        }
    }
    out
}

fn columns() -> [ColumnView; 2] {
    [
        ColumnView::default(),
        ColumnView {
            open: true,
            ..ColumnView::default()
        },
    ]
}

fn thumb() -> Vec<u8> {
    (0..THUMB_W * THUMB_H)
        .flat_map(|i| [(i % 251) as u8, (i % 13) as u8 * 19, 77, 0xff])
        .collect()
}

const AVATAR: &str = "Avatar";

/// The sheet's sample slots (`sheet_screens::SLOTS`) as the host lists
/// them: the quick slot, two of the player's, a damaged one and an import.
fn sheet_slots(thumb: &[u8]) -> ShellSnapshot {
    let slot = |id: &str, name: &str, place: &str, day: u16| SlotEntry {
        info: SlotInfo {
            id: SlotId(id.into()),
            name: name.into(),
            quick: id == "quick",
            avatar: AVATAR.into(),
            place: place.into(),
            clock: [139, 4, day, 9, 15],
            turn: 18234,
            ..SlotInfo::default()
        },
        thumb: Some(thumb.to_vec()),
    };
    let mut morning = slot("s0002", "Morning walk", "a dwelling", 2);
    morning.info.modified = true;
    let mut old = slot("s0003", "Old game", "", 1);
    old.info.damaged = Some("SAVED.GAM is missing".into());
    old.thumb = None;
    let mut import = slot("s0004", "From DOS", "Britannia", 29);
    import.info.imported = true;
    import.info.clock[1] = 3;
    import.thumb = None;
    ShellSnapshot {
        intro_shell: true,
        slots: true,
        save_available: true,
        quicksave: true,
        slot_list: Arc::new(vec![
            slot("quick", "quick", "Britannia", 7),
            slot("s0001", "Second try", "Britannia", 6),
            morning,
            old,
            import,
        ]),
        ..ShellSnapshot::default()
    }
}

/// The page with the sheet's backdrop: every clear pixel dimmed.
fn dimmed(mut img: MenuImage) -> MenuImage {
    dim_backdrop(&mut img);
    img
}

/// The shell's Save, Load and dialog pages equal the sheet's screens.
#[test]
fn the_slot_pages_are_the_sheet_s_screens() {
    let font = font();
    let thumb = thumb();
    let snap = sheet_slots(&thumb);
    for picture in pictures() {
        for column in columns() {
            let inputs = Inputs {
                font: &font,
                picture,
                column,
                thumb: &thumb,
                avatar: AVATAR,
            };
            let sheet = all_screens(&inputs);
            let screen = |name: &str| {
                &sheet
                    .iter()
                    .find(|screen| screen.name == name)
                    .unwrap_or_else(|| panic!("the sheet has {name}"))
                    .image
            };
            let at = format!("{}x{} {column:?}", picture.w, picture.h);
            let save = save_page(&font, &picture, column, &snap, 0);
            assert!(dimmed(save.image) == *screen("save"), "save at {at}");
            let load = load_page(&font, &picture, column, &snap, 0);
            assert!(dimmed(load.image) == *screen("load"), "load at {at}");
            let damaged = load_page(&font, &picture, column, &snap, 3);
            assert!(
                dimmed(damaged.image) == *screen("load-damaged"),
                "load, damaged slot, at {at}"
            );
            let dialog = |dialog, cursor, field: &str, frame| {
                dimmed(
                    dialog_page(&font, &picture, column, &snap, dialog, cursor, field, frame).image,
                )
            };
            assert!(
                dialog(Dialog::Name(NameFor::New), 0, "Morning walk", 3) == *screen("save-name"),
                "the name field at {at}"
            );
            assert!(
                dialog(Dialog::Overwrite(1), 1, "", 0) == *screen("save-overwrite"),
                "write over at {at}"
            );
            assert!(
                dialog(Dialog::Delete(3), 1, "", 0) == *screen("load-delete"),
                "delete at {at}"
            );
        }
    }
}

/// Opaque pixels are EGA colours (or, with `thumbs`, a thumbnail's
/// picture); the rest is clear (the backdrop's). Every panel lies inside
/// the picture on whole cells.
fn assert_kit_pixels(img: &MenuImage, panel: CellRect, thumbs: bool, at: &str) {
    let (x0, y0, x1, y1) = panel.pixels();
    assert!(x1 < img.w && y1 < img.h, "{at}: the panel is inside");
    for y in 0..img.h {
        for x in 0..img.w {
            let px = img.at(x, y);
            let inside = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
            if !inside {
                assert_eq!(px, CLEAR, "{at}: ({x}, {y}) outside the panel");
                continue;
            }
            assert!(
                px == CLEAR || ega_index(px).is_some() || (thumbs && px[3] == 0xff),
                "{at}: ({x}, {y}) {px:?}"
            );
        }
    }
}

/// The longest names a player can type and more slots than the list
/// shows: names are cut with "...", the list scrolls with its marks on the
/// bands, and nothing leaves the kit's grid.
#[test]
fn long_names_and_long_lists_stay_in_their_boxes() {
    let font = font();
    let thumb = thumb();
    let long = "W".repeat(t::SLOT_NAME_MAX);
    let slots: Vec<SlotEntry> = (0..40)
        .map(|i| SlotEntry {
            info: SlotInfo {
                id: SlotId(format!("s{i:04}")),
                name: long.clone(),
                avatar: "Wwwwwwww".into(),
                place: "Castle British".into(),
                clock: [139, 12, 31, 23, 59],
                turn: 9_999_999,
                modified: i % 3 == 0,
                ..SlotInfo::default()
            },
            thumb: Some(thumb.clone()),
        })
        .collect();
    let snap = ShellSnapshot {
        intro_shell: true,
        slots: true,
        quicksave: true,
        slot_list: Arc::new(slots),
        ..ShellSnapshot::default()
    };
    for picture in pictures() {
        for column in columns() {
            let at = format!("{}x{} {column:?}", picture.w, picture.h);
            for cursor in [0, 17, 39] {
                let save = save_page(&font, &picture, column, &snap, cursor);
                assert_kit_pixels(&save.image, save.panel, false, &at);
                let load = load_page(&font, &picture, column, &snap, cursor);
                assert_kit_pixels(&load.image, load.panel, true, &at);
            }
            for dialog in [
                Dialog::Name(NameFor::New),
                Dialog::Name(NameFor::Rename(0)),
                Dialog::Overwrite(0),
                Dialog::Delete(0),
                Dialog::Loading(0),
            ] {
                for cursor in [0, 1] {
                    let drawn =
                        dialog_page(&font, &picture, column, &snap, dialog, cursor, &long, 2);
                    assert_kit_pixels(&drawn.image, drawn.panel, false, &at);
                    // A dialog's panel is whole and inside the picture.
                    assert!(drawn.panel.right() < picture.cols());
                    assert!(drawn.panel.bottom() < picture.rows());
                }
            }
        }
    }
    // The long name is cut to its room, ending in "...".
    assert_eq!(t::cut(&long, 10), "WWWWWWW...");
}

/// The name field's line never reaches the box's margin: a full field and
/// its cursor end two cells before the right rule.
#[test]
fn a_full_name_field_keeps_its_margin() {
    let w = super::geometry::DIALOG_CELLS - 2;
    let last = 2 + t::SLOT_NAME_MAX as u32;
    assert!(
        last < w - 1,
        "the cursor's cell {last} is inside the margin"
    );
}
