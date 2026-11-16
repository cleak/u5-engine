//! The frame kit's pixel invariants (build plan M4-01): the cell grid, the
//! corners, the junctions, the tabs, the selection bar, the EGA colours and
//! the sweep law. A synthetic block font stands in for IBM.CH; the faithful
//! pieces are drawn by the engine's own painters and compared pixel for
//! pixel.

use u5_runtime::enhancements::PixelAspect;
use u5_runtime::{
    CH_FONT_LEN, CHROME_CORNER_PROFILE, ChromeFonts, ChromeGap, ChromePalette,
    GameplayChromeContent, IBM_CH_FILE, paint_gameplay_frame_chrome, parse_ch_font,
    wind_banner_text,
};

use super::frame::*;
use super::geometry::*;
use super::sheet_screens::{Inputs, all_screens};
use super::text as t;
use crate::display::effects::{step_of, sweep_t};
use crate::display::grow::Canvas;
use crate::display::grow::bands::tab_centre_wanted;
use crate::display::grow::column::ColumnView;

/// A block font: the two cap triangles, and for every printable code a
/// distinct pattern in rows 0-6 (space blank).
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

fn shut() -> ColumnView {
    ColumnView::default()
}

fn open() -> ColumnView {
    ColumnView {
        open: true,
        ..ColumnView::default()
    }
}

/// Every distinct canvas over a spread of windows, both pixel aspects.
fn canvases() -> Vec<Canvas> {
    let mut out: Vec<Canvas> = Vec::new();
    for aspect in [PixelAspect::Dos, PixelAspect::Square] {
        for h in [720, 960, 1080, 1200, 1440, 2160] {
            for w in (960..=3840).step_by(40) {
                if let Some(c) = Canvas::for_window(w, h, aspect)
                    && !out.iter().any(|o| o.w == c.w && o.h == c.h)
                {
                    out.push(c);
                }
            }
        }
    }
    for (w, h) in [(512, 240), (688, 240), (480, 216), (688, 216)] {
        assert!(
            out.iter().any(|c| c.w == w && c.h == h),
            "the named canvas {w}x{h} is covered"
        );
    }
    out
}

fn pictures() -> Vec<MenuPicture> {
    let mut out = vec![menu_picture(None)];
    out.extend(canvases().iter().map(|c| menu_picture(Some(c))));
    out
}

const SHAPES: [PageShape; 6] = [
    PageShape::Narrow,
    PageShape::Wide,
    PageShape::Title,
    PageShape::Dialog { lines: 5 },
    PageShape::Dialog { lines: 7 },
    PageShape::Dialog { lines: 3 },
];

fn index(img: &MenuImage, x: u32, y: u32) -> Option<u8> {
    ega_index(img.at(x, y))
}

#[test]
fn panels_sit_on_the_cell_grid_of_every_picture() {
    for picture in pictures() {
        assert_eq!(picture.w % CELL, 0);
        assert_eq!(picture.h % CELL, 0);
        for shape in SHAPES {
            for column in [shut(), open()] {
                let rect = panel_rect(shape, &picture, column);
                let (x0, y0, x1, y1) = rect.pixels();
                assert!(
                    x1 < picture.w && y1 < picture.h,
                    "{shape:?} inside {picture:?}"
                );
                assert_eq!((x0 % CELL, y0 % CELL), (0, 0));
                assert_eq!(((x1 + 1) % CELL, (y1 + 1) % CELL), (0, 0));
                assert_eq!(rect.w, shape.width());
                let Some(canvas) = picture.canvas else {
                    continue;
                };
                if matches!(shape, PageShape::Narrow | PageShape::Wide) {
                    // The top rule is the canvas's tab-row rule (y 15) and
                    // the bottom rule the bottom band's (H - 8).
                    assert_eq!(rect.row * CELL + 7, 15);
                    assert_eq!(rect.bottom() * CELL, canvas.h - 8);
                }
                // Centred over the visible map, as the tabs are, when it
                // fits there; otherwise over the whole map window.
                let centre = (rect.col * CELL * 2 + rect.w * CELL) / 2;
                let first = if column.shown() { 152 } else { 8 };
                let fits = first + rect.w * CELL <= canvas.w - 8;
                let wanted = if fits {
                    tab_centre_wanted(&canvas, column.shown())
                } else {
                    canvas.w / 2
                };
                assert!(
                    centre.abs_diff(wanted) <= CELL / 2,
                    "{shape:?} on {}x{} {column:?}: centre {centre}, tabs {wanted}",
                    canvas.w,
                    canvas.h
                );
                if fits {
                    assert!(x0 >= first && x1 < canvas.w - 8, "inside the visible map");
                }
            }
        }
    }
}

#[test]
fn frame_pages_lie_on_the_frame_s_own_rings() {
    let picture = MenuPicture::FRAME;
    assert_eq!(
        panel_rect(PageShape::Narrow, &picture, shut()),
        CellRect::new(0, 0, 24, 24),
        "the map view's ring: rules at x and y 7 and 184"
    );
    assert_eq!(
        panel_rect(PageShape::Wide, &picture, shut()),
        CellRect::new(0, 0, 40, 24),
        "the frame's ring: rules at x 7 and 312, y 7 and 184"
    );
    assert_eq!(
        panel_rect(PageShape::Title, &picture, shut()),
        CellRect::new(0, 15, 40, 10),
        "the title menu's frame"
    );
    let mut img = MenuImage::for_picture(&picture);
    draw_panel(&mut img, &Panel::new(CellRect::new(0, 0, 24, 24)));
    for (x, y) in [(7, 7), (184, 7), (7, 184), (184, 184), (100, 7), (7, 100)] {
        assert_eq!(index(&img, x, y), Some(RULE), "rule at ({x}, {y})");
    }
    assert_eq!(index(&img, 8, 8), Some(PAPER));
    assert_eq!(index(&img, 6, 100), Some(BAND));
    assert_eq!(index(&img, 185, 100), Some(BAND));
}

#[test]
fn the_title_panel_is_the_title_menu_frame() {
    let mut buffer = crate::IntroDisplayBuffer::new(320, 200);
    crate::draw_visual_intro_menu_window_chrome(&mut buffer);
    let rect = panel_rect(PageShape::Title, &MenuPicture::FRAME, shut());
    let mut img = MenuImage::for_picture(&MenuPicture::FRAME);
    draw_panel(&mut img, &Panel::new(rect));
    for y in 120..200u32 {
        let start = u5_runtime::intro_menu_frame_border_start_column(y as u16).unwrap() as u32;
        for x in 0..320u32 {
            let faithful = buffer.pixels[(y * 320 + x) as usize];
            let mine = img.at(x, y);
            if x < start || x > 319 - start {
                assert_eq!(mine, CLEAR, "carved corner at ({x}, {y})");
            } else {
                assert_eq!(ega_index(mine), Some(faithful), "({x}, {y})");
            }
        }
    }
}

#[test]
fn corners_follow_the_profile() {
    let rect = CellRect::new(3, 2, 12, 9);
    let mut img = MenuImage::new(160, 120);
    draw_panel(&mut img, &Panel::new(rect));
    let (x0, y0, x1, y1) = rect.pixels();
    for r in 0..8u32 {
        let first = CHROME_CORNER_PROFILE.get(r as usize).copied().unwrap_or(0) as u32;
        for d in 0..8u32 {
            for (x, y) in [
                (x0 + d, y0 + r),
                (x1 - d, y0 + r),
                (x0 + d, y1 - r),
                (x1 - d, y1 - r),
            ] {
                let drawn = img.at(x, y) != CLEAR;
                assert_eq!(drawn, d >= first, "corner pixel ({x}, {y})");
            }
        }
    }
}

/// The faithful gameplay chrome with a stats label and a wind label.
fn faithful_chrome(font: &u5_runtime::FixedCellFont, stats_label: Option<&str>) -> Vec<u8> {
    let mut rgba = vec![0u8; 320 * 200 * 4];
    let content = GameplayChromeContent {
        bottom: ChromeGap::Label(wind_banner_text(Some("Calm"))),
        stats_label: stats_label.map(str::to_string),
        ..GameplayChromeContent::default()
    };
    paint_gameplay_frame_chrome(
        &mut rgba,
        320,
        200,
        &content,
        ChromeFonts {
            ibm: font,
            runes: font,
        },
        ChromePalette::EGA,
    );
    rgba
}

fn faithful_index(rgba: &[u8], x: u32, y: u32) -> u8 {
    let i = ((y * 320 + x) * 4) as usize;
    crate::ega_palette_index_from_rgba(&rgba[i..i + 4])
}

/// The stats panel's divider rows 7 and 10 are bands across the panel; a
/// panel with dividers at those rows is the faithful chrome there, pixel
/// for pixel, junctions included.
#[test]
fn dividers_meet_the_ring_as_the_stats_panel_does() {
    let font = font();
    let faithful = faithful_chrome(&font, None);
    // Tight, the kit's divider is the stats panel's pixel for pixel; by
    // default each rule stands a pixel off its box (below).
    let panel = Panel::new(CellRect::new(23, 0, 17, 25))
        .divider(7)
        .divider(10)
        .tight();
    let mut img = MenuImage::new(320, 200);
    draw_panel(&mut img, &panel);
    for y in 0..87u32 {
        for x in 185..320u32 {
            if y < 8 && !(24 * 8..39 * 8).contains(&x) {
                // The top-right corner is a glyph in the faithful paint; the
                // panel's own top-left corner is rounded where the faithful
                // band runs on.
                continue;
            }
            assert_eq!(
                index(&img, x, y),
                Some(faithful_index(&faithful, x, y)),
                "({x}, {y})"
            );
        }
    }
}

/// By default a divider's rules stand a pixel off the boxes beside it
/// (the footer's help never touches the rule over it): black, rule, four
/// rows of blue, rule, black, and the junctions with the ring still merge
/// with no rule pixel doubled. The ring's rules stay on the frame's own.
#[test]
fn a_divider_s_rules_stand_a_pixel_off_its_boxes() {
    let panel = Panel::new(CellRect::new(0, 0, 40, 24)).divider(10);
    let mut img = MenuImage::new(320, 200);
    draw_panel(&mut img, &panel);
    let column: Vec<Option<u8>> = (80..88).map(|y| index(&img, 100, y)).collect();
    assert_eq!(
        column,
        [
            Some(PAPER),
            Some(RULE),
            Some(BAND),
            Some(BAND),
            Some(BAND),
            Some(BAND),
            Some(RULE),
            Some(PAPER)
        ]
    );
    // At the side ring the band runs into the ring's blue and the side
    // rule turns the corner at the divider's rule.
    assert_eq!(index(&img, 7, 80), Some(RULE));
    assert_eq!(index(&img, 7, 81), Some(RULE));
    assert_eq!(index(&img, 7, 83), Some(BAND));
    assert_eq!(index(&img, 7, 86), Some(RULE));
    assert_eq!(index(&img, 7, 87), Some(RULE));
    assert_eq!(index(&img, 8, 80), Some(PAPER));
    assert_eq!(index(&img, 8, 81), Some(RULE));
    // The ring is the frame's: rules at y 7 and 184.
    assert_eq!(index(&img, 100, 7), Some(RULE));
    assert_eq!(index(&img, 100, 8), Some(PAPER));
    assert_eq!(index(&img, 100, 184), Some(RULE));
}

/// The middle band between the map view and the stats panel is a split:
/// where it meets the top band the rules turn the corner and the blues
/// merge, as the faithful frame has it.
#[test]
fn splits_meet_the_ring_as_the_middle_band_does() {
    let font = font();
    let faithful = faithful_chrome(&font, None);
    let panel = Panel::new(CellRect::new(0, 0, 40, 24)).split(23, 0, 23);
    let mut img = MenuImage::new(320, 200);
    draw_panel(&mut img, &panel);
    for y in 0..56u32 {
        for x in 8 * 8..36 * 8u32 {
            assert_eq!(
                index(&img, x, y),
                Some(faithful_index(&faithful, x, y)),
                "({x}, {y})"
            );
        }
    }
}

/// No rule pixel is doubled or missing: a band pixel is white exactly when
/// it touches a box (diagonals included), every rule is on a cell's first
/// or last pixel (a divider's a pixel in, by default), and two rules are
/// never adjacent. Each panel is checked tight and by default.
#[test]
fn no_rule_pixel_is_doubled_or_missing() {
    let panels = [
        Panel::new(CellRect::new(0, 0, 40, 24)),
        Panel::new(CellRect::new(0, 0, 40, 24))
            .divider(3)
            .divider(19),
        Panel::new(CellRect::new(2, 1, 30, 20))
            .divider(16)
            .split(20, 1, 16),
        Panel::new(CellRect::new(0, 0, 40, 24))
            .divider(10)
            .divider(19)
            .split(12, 0, 10)
            .split(30, 10, 19),
    ];
    let panels = panels.iter().flat_map(|p| [p.clone(), p.clone().tight()]);
    for panel in &panels.collect::<Vec<_>>() {
        let mut img = MenuImage::new(320, 200);
        draw_panel(&mut img, panel);
        let (x0, y0, x1, y1) = panel.rect.pixels();
        let boxes = panel.boxes();
        let cell_box = |col: u32, row: u32| boxes.iter().any(|b| b.contains(col, row));
        let is_box = |x: i64, y: i64| {
            if x < 0 || y < 0 {
                return false;
            }
            let (col, row, line) = (x as u32 / CELL, y as u32 / CELL, y as u32 % CELL);
            cell_box(col, row)
                || (panel.breathe
                    && panel.dividers.contains(&row)
                    && ((line == CELL - 1 && cell_box(col, row + 1))
                        || (line == 0 && cell_box(col, row - 1))))
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let pixel = img.at(x, y);
                if pixel == CLEAR {
                    continue;
                }
                let (xi, yi) = (i64::from(x), i64::from(y));
                if is_box(xi, yi) {
                    assert_eq!(ega_index(pixel), Some(PAPER));
                    continue;
                }
                let touches = (-1..=1).any(|dy| (-1..=1).any(|dx| is_box(xi + dx, yi + dy)));
                let want = if touches { RULE } else { BAND };
                assert_eq!(ega_index(pixel), Some(want), "({x}, {y}) of {panel:?}");
                if want == RULE {
                    let divider = panel.dividers.contains(&(y / 8));
                    let edge = if panel.breathe && divider {
                        matches!(y % 8, 1 | 6)
                    } else {
                        matches!(y % 8, 0 | 7)
                    };
                    assert!(
                        matches!(x % 8, 0 | 7) || edge,
                        "rule off the cell edge at ({x}, {y})"
                    );
                }
            }
        }
        // Never two parallel rules side by side: no 2x3 or 3x2 block of
        // rule pixels.
        let white = |x: u32, y: u32| index(&img, x, y) == Some(RULE);
        for y in y0 + 1..y1 {
            for x in x0 + 1..x1 {
                let tall = (0..2).all(|dx| (0..3).all(|dy| white(x - 1 + dx, y - 1 + dy)));
                let wide = (0..3).all(|dx| (0..2).all(|dy| white(x - 1 + dx, y - 1 + dy)));
                assert!(!tall && !wide, "doubled rule at ({x}, {y}) of {panel:?}");
            }
        }
    }
}

/// A tab on a ring row is the faithful gap: the caps and text sit on the
/// rule exactly as `>Select:<` on the stats panel's top rule and
/// `>Calm  Winds<` on the map view's bottom rule do.
#[test]
fn tabs_sit_on_the_rule_as_the_faithful_tabs_do() {
    let font = font();
    let faithful = faithful_chrome(&font, Some("Select:"));
    // The stats label: caps at 27 and 35 on row 0.
    let mut img = MenuImage::new(320, 200);
    draw_panel(
        &mut img,
        &Panel::new(CellRect::new(23, 0, 17, 25)).divider(7),
    );
    let last = draw_label_tab(&mut img, &font, 27, 0, "Select:", INK);
    assert_eq!(last, 35);
    for y in 0..9u32 {
        for x in 24 * 8..38 * 8u32 {
            assert_eq!(
                index(&img, x, y),
                Some(faithful_index(&faithful, x, y)),
                "({x}, {y})"
            );
        }
    }
    // The wind banner: caps at 6 and 18 on row 23.
    let mut img = MenuImage::new(320, 200);
    draw_panel(&mut img, &Panel::new(CellRect::new(0, 0, 24, 24)));
    let wind = wind_banner_text(Some("Calm"));
    let last = draw_label_tab(&mut img, &font, 6, 23, &wind, INK);
    assert_eq!(last, 18);
    for y in 182..192u32 {
        for x in 8..23 * 8u32 {
            assert_eq!(
                index(&img, x, y),
                Some(faithful_index(&faithful, x, y)),
                "({x}, {y})"
            );
        }
    }
}

/// The tab pieces never touch a corner cell, and the title and footer tabs
/// of every page fit their ring.
#[test]
fn tabs_stay_clear_of_the_corners() {
    let narrow = CellRect::new(0, 0, NARROW_CELLS, 24);
    let wide = CellRect::new(0, 0, WIDE_CELLS, 24);
    for (rect, tabs) in [(narrow, t::NARROW_TABS), (wide, t::WIDE_TABS)] {
        for tab in tabs {
            let first = centred_tab_col(rect, tab.len());
            let last = first + tab_cells(tab.len()) - 1;
            assert!(first > rect.col + 1 && last + 1 < rect.right(), "{tab:?}");
        }
    }
    let strip = tab_strip_cells(&t::SETTINGS_TABS);
    assert!(
        strip <= WIDE_CELLS - 4,
        "the Settings tab strip: {strip} cells"
    );
}

#[test]
fn the_selection_bar_spans_the_interior_and_inverts_the_row() {
    let font = font();
    let panel = Panel::new(CellRect::new(0, 0, 24, 24)).divider(20);
    let bx = panel.boxes()[0];
    let line = Line::new(bx.w).text(1, t::SAVE, Ink::Plain).check(18, true);
    let mut plain = MenuImage::new(320, 200);
    draw_panel(&mut plain, &panel);
    let mut barred = plain.clone();
    draw_row(&mut plain, &font, bx, 5, &line, false);
    draw_selection_bar(&mut barred, &font, bx, 5, &line);
    let (bx0, _, bx1, _) = bx.pixels();
    let (y0, y1) = (5 * CELL, 5 * CELL + 7);
    for y in 0..200 {
        for x in 0..320 {
            let inside = (bx0..=bx1).contains(&x) && (y0..=y1).contains(&y);
            if inside {
                let p = index(&plain, x, y).unwrap();
                let b = index(&barred, x, y).unwrap();
                assert!(
                    (p, b) == (INK, PAPER) || (p, b) == (PAPER, RULE),
                    "({x}, {y}): {p} vs {b}"
                );
            } else {
                assert_eq!(
                    plain.at(x, y),
                    barred.at(x, y),
                    "outside the bar at ({x}, {y})"
                );
            }
        }
    }
    // The bar touches the rules on both sides and nothing past them.
    assert_eq!(bx0 - 1, 7);
    assert_eq!(index(&barred, bx0 - 1, y0), Some(RULE));
    assert_eq!(index(&barred, bx1 + 1, y0), Some(RULE));
    assert_eq!(index(&barred, bx0 - 2, y0), Some(BAND));
}

/// Every label of the wording table fits the row it is drawn in.
#[test]
fn every_label_fits_its_page() {
    for label in t::NARROW_LABELS {
        assert!(label.len() <= t::NARROW_TEXT, "{label:?}");
    }
    for label in t::SETTINGS_ROW_LABELS {
        assert!(label.len() <= t::SETTINGS_LABEL, "{label:?}");
    }
    const {
        assert!(
            t::SETTINGS_LABEL + 1 + 15 <= t::WIDE_TEXT,
            "label, gap, value and tag"
        )
    };
    for tab in t::NARROW_TABS {
        assert!(tab.len() <= t::TAB_TEXT_NARROW, "{tab:?}");
    }
    for tab in t::WIDE_TABS {
        assert!(tab.len() <= t::TAB_TEXT_WIDE, "{tab:?}");
    }
    assert!(t::LOAD_KEYS.len() <= t::WIDE_TEXT);
    let longest = "W".repeat(t::SLOT_NAME_MAX);
    for template in t::DIALOG_TEMPLATES {
        let text = t::fill(template, &[&longest, "99999"]);
        for line in t::wrap(&text, t::DIALOG_TEXT) {
            assert!(line.len() <= t::DIALOG_TEXT, "{line:?}");
        }
        assert!(t::wrap(&text, t::DIALOG_TEXT).len() <= 3, "{template:?}");
    }
    let title = t::continue_label(&longest, "139/04/06");
    assert_eq!(
        title.len(),
        t::WIDE_TEXT,
        "the title layer's Continue: {title:?}"
    );
    assert_eq!(
        t::continue_label("Morning", "139/04/06"),
        "Continue  (Morning, 139/04/06)"
    );
    assert_eq!(t::cut("abcdefgh", 6), "abc...");
}

#[test]
fn wording_helpers() {
    assert_eq!(t::fill(t::SAVED_TO, &["Morning"]), "Saved to \"Morning\".");
    assert_eq!(t::fill("{} and {}", &["a"]), "a and ");
    assert_eq!(
        t::wrap("Thou hast not saved for many turns.", 12),
        ["Thou hast", "not saved", "for many", "turns."]
    );
    assert_eq!(t::wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
    assert!(t::wrap("", 4).is_empty());
}

fn inputs<'a>(
    font: &'a u5_runtime::FixedCellFont,
    picture: MenuPicture,
    column: ColumnView,
    thumb: &'a [u8],
) -> Inputs<'a> {
    Inputs {
        font,
        picture,
        column,
        thumb,
        avatar: "Avatar",
    }
}

/// Every screen of 3.2 on every picture: opaque pixels are EGA colours
/// (the thumbnail's picture aside), the rest is the backdrop or clear, no
/// text row crosses a rule, and every row's text stays in its box.
#[test]
fn every_screen_is_ega_on_the_grid() {
    let font = font();
    let thumb: Vec<u8> = (0..THUMB_W * THUMB_H)
        .flat_map(|i| [(i % 251) as u8, (i % 13) as u8 * 19, 77, 0xff])
        .collect();
    let named = [
        (512, 240),
        (688, 240),
        (480, 216),
        (688, 216),
        (384, 240),
        (432, 216),
    ];
    let mut list = vec![MenuPicture::FRAME];
    for (w, h) in named {
        if let Some(c) = canvases().into_iter().find(|c| (c.w, c.h) == (w, h)) {
            list.push(menu_picture(Some(&c)));
        }
    }
    assert!(list.len() >= 5, "the named canvases");
    for picture in list {
        for column in [shut(), open()] {
            let inputs = inputs(&font, picture, column, &thumb);
            for screen in all_screens(&inputs) {
                let img = &screen.image;
                assert_eq!((img.w, img.h), (picture.w, picture.h));
                let thumb_boxes: Vec<(u32, u32, u32, u32)> = screen
                    .panel
                    .iter()
                    .flat_map(|p| p.boxes())
                    .map(|b| b.pixels())
                    .collect();
                for y in 0..img.h {
                    for x in 0..img.w {
                        let pixel = img.at(x, y);
                        if pixel[3] == 0xff {
                            if ega_index(pixel).is_none() {
                                // Only the thumbnail's picture may be other
                                // colours, and only inside a box.
                                assert!(
                                    screen.name.starts_with("load")
                                        && thumb_boxes.iter().any(|&(a, b, c, d)| {
                                            (a..=c).contains(&x) && (b..=d).contains(&y)
                                        }),
                                    "{}: ({x}, {y}) {pixel:?}",
                                    screen.name
                                );
                            }
                        } else if screen.panel.is_some() {
                            assert_eq!(pixel, DIM, "{}: ({x}, {y})", screen.name);
                        } else {
                            assert_eq!(pixel, CLEAR, "{}: ({x}, {y})", screen.name);
                        }
                    }
                }
                if let Some(panel) = &screen.panel {
                    let boxes = panel.boxes();
                    for (bx, row) in &screen.rows {
                        assert!(boxes.contains(bx), "{}: a row in a box", screen.name);
                        assert!(
                            bx.contains(bx.col, *row),
                            "{}: row {row} in its box",
                            screen.name
                        );
                    }
                    // No tab covers a split's junctions.
                    for split in &panel.splits {
                        let (x, top, bottom) =
                            (split.col * CELL, split.top * CELL + 7, split.bottom * CELL);
                        for y in [top, bottom] {
                            assert_eq!(
                                index(img, x, y),
                                Some(RULE),
                                "{}: junction at ({x}, {y})",
                                screen.name
                            );
                            assert_eq!(
                                index(img, x + 7, y),
                                Some(RULE),
                                "{}: junction",
                                screen.name
                            );
                            for dx in 1..7 {
                                assert_eq!(
                                    index(img, x + dx, y),
                                    Some(BAND),
                                    "{}: junction",
                                    screen.name
                                );
                            }
                        }
                    }
                    // Outside the panel the backdrop only.
                    let (x0, y0, x1, y1) = panel.rect.pixels();
                    for y in 0..img.h {
                        for x in 0..img.w {
                            if !((x0..=x1).contains(&x) && (y0..=y1).contains(&y)) {
                                assert_eq!(img.at(x, y), DIM, "{}: ({x}, {y})", screen.name);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The sweep is the M1 law over the panel's rectangle: a pixel shows the
/// panel from step `step_of(sweep_t(x, y, top, bottom))`, the backdrop
/// before it; the whole menu with no step. It rises: no pixel arrives
/// before a pixel four rows below it in the same column.
#[test]
fn the_sweep_follows_the_m1_law() {
    let font = font();
    let thumb = vec![0u8; (THUMB_W * THUMB_H * 4) as usize];
    let canvas = Canvas::for_window(2560, 1440, PixelAspect::Dos).unwrap();
    for picture in [MenuPicture::FRAME, menu_picture(Some(&canvas))] {
        let inputs = inputs(&font, picture, shut(), &thumb);
        let screen = all_screens(&inputs).into_iter().next().unwrap();
        let rect = screen.panel.as_ref().unwrap().rect;
        let (x0, y0, x1, y1) = rect.pixels();
        assert_eq!(sweep_image(&screen.image, rect, None), screen.image);
        for k in 1..8u8 {
            let frame = sweep_image(&screen.image, rect, Some(k));
            for y in 0..picture.h {
                for x in 0..picture.w {
                    let inside = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
                    let want = if inside && step_of(sweep_t(x, y, y0, y1 + 1)) > k {
                        DIM
                    } else {
                        screen.image.at(x, y)
                    };
                    assert_eq!(frame.at(x, y), want, "step {k} at ({x}, {y})");
                }
            }
        }
        for y in y0..=y1 - 4 {
            for x in x0..=x1 {
                assert!(
                    panel_step(rect, x, y) >= panel_step(rect, x, y + 4),
                    "rising at ({x}, {y})"
                );
            }
        }
        let bottom = (x0..=x1).flat_map(|x| (y1 - 3..=y1).map(move |y| (x, y)));
        assert_eq!(
            bottom.map(|(x, y)| panel_step(rect, x, y)).min(),
            Some(1),
            "the bottom arrives first"
        );
        assert_eq!(
            (x0..=x1).map(|x| panel_step(rect, x, y0)).max(),
            Some(8),
            "the top arrives last"
        );
    }
    assert_eq!(MENU_SWEEP_STEP.as_millis() * 8, 200);
}

#[test]
fn the_backdrop_is_black_at_0_6() {
    let mut img = MenuImage::new(16, 16);
    img.set(3, 3, ega(BAND));
    dim_backdrop(&mut img);
    assert_eq!(img.at(0, 0), [0, 0, 0, 153]);
    assert_eq!(img.at(3, 3), ega(BAND));
    let picture = vec![200u8; 16 * 16 * 4];
    let out = composite_over(&picture, &img);
    assert_eq!(
        &out[0..4],
        &[80, 80, 80, 0xff],
        "0.4 of the game shows through"
    );
    assert_eq!(&out[(3 * 16 + 3) * 4..(3 * 16 + 3) * 4 + 4], &ega(BAND));
}

#[test]
fn the_thumbnail_is_the_frame_box_filtered_in_a_ruled_box() {
    let font = font();
    let mut frame = vec![0u8; 320 * 200 * 4];
    for (i, px) in frame.chunks_mut(4).enumerate() {
        let (x, y) = (i % 320, i / 320);
        px.copy_from_slice(&[
            (x / 4 * 3) as u8,
            (y / 4 * 5) as u8,
            if (x + y) % 2 == 0 { 255 } else { 0 },
            255,
        ]);
    }
    let thumb = thumbnail_from_frame(&frame);
    assert_eq!(thumb.len(), (THUMB_W * THUMB_H * 4) as usize);
    assert_eq!(&thumb[0..4], &[0, 0, 128, 255]);
    let mut img = MenuImage::new(320, 200);
    draw_thumbnail_box(&mut img, &font, 10, 4, Thumb::Picture(&thumb));
    let (x0, y0, x1, y1) = CellRect::new(10, 4, THUMB_CELLS.0, THUMB_CELLS.1).pixels();
    for x in x0 - 1..=x1 + 1 {
        assert_eq!(index(&img, x, y0 - 1), Some(RULE));
        assert_eq!(index(&img, x, y1 + 1), Some(RULE));
    }
    for y in y0..=y1 {
        assert_eq!(index(&img, x0 - 1, y), Some(RULE));
        assert_eq!(index(&img, x1 + 1, y), Some(RULE));
    }
    assert_eq!((x0 - 1) % 8, 7);
    assert_eq!((x1 + 1) % 8, 0);
    for y in [y0, y0 + THUMB_PAD - 1, y1 - THUMB_PAD + 1, y1] {
        assert_eq!(index(&img, x0 + 5, y), Some(PAPER), "padding row {y}");
    }
    assert_eq!(img.at(x0, y0 + THUMB_PAD), [0, 0, 128, 255]);
}
