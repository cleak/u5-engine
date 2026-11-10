//! Step 7, the crawl (design 2.2.4): where the rows go on each named
//! layout, the six-row cap, the colours by age and class, the outline and
//! its clip, the end cap merged into the border, the crawl off, and the
//! core never covered.

use u5_runtime::enhancements::PixelAspect;

use super::*;
use crate::display::crawl::layer::{CapSprite, CrawlCell, CrawlLine, CrawlRows, LineKind};
use crate::display::grow::compose::{RULE_WHITE, core_rect};

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

/// A line of `cells` glyphs, each one pixel of ink at its centre row 3,
/// column 3, unless `full` (every pixel ink).
fn line(kind: LineKind, age: u64, cells: usize, typed: bool, full: bool) -> CrawlLine {
    let mask = if full {
        [0xff; 8]
    } else {
        [0, 0, 0, 0x10, 0, 0, 0, 0]
    };
    CrawlLine {
        kind,
        age,
        cells: (0..cells)
            .map(|cell| CrawlCell {
                cell: cell as u8,
                byte: b'x',
                mask,
                typed,
            })
            .collect(),
        frame_row: 11,
    }
}

fn rows(lines: Vec<CrawlLine>) -> CrawlRows {
    CrawlRows {
        lines,
        ..CrawlRows::default()
    }
}

fn blank(c: &Canvas) -> Vec<u8> {
    vec![0x20; c.w as usize * c.h as usize * 4]
}

fn at(rgba: &[u8], c: &Canvas, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * c.w + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2]]
}

fn paint(rgba: &mut [u8], c: &Canvas, rows: &CrawlRows, crawl: bool, typed: bool) -> Vec<u32> {
    paint_crawl(
        rgba,
        c,
        CrawlPaint {
            rows,
            crawl,
            typed_colour: typed,
        },
    )
}

#[test]
fn the_rows_end_at_the_bottom_of_the_map_window_six_at_most() {
    for ((ww, wh), bottom) in [
        ((2560, 1440), 224),
        ((3440, 1440), 224),
        ((1920, 1080), 200),
    ] {
        let c = canvas(ww, wh);
        assert_eq!(crawl_bottom_y(&c), bottom, "{ww}x{wh}");
        let mut lines: Vec<CrawlLine> = (0..8)
            .map(|_| line(LineKind::Text, 0, 3, false, false))
            .collect();
        lines.push(line(LineKind::Live, 0, 0, false, false));
        let mut rgba = blank(&c);
        let ys = paint(&mut rgba, &c, &rows(lines), true, true);
        assert_eq!(ys, (0..6).map(|i| bottom - 40 + 8 * i).collect::<Vec<_>>());
        // The first glyph's ink, at x 16 + 3 on the top row.
        assert_eq!(at(&rgba, &c, 19, bottom - 40 + 3), EGA_PALETTE_RGB[15]);
    }
}

#[test]
fn colours_follow_the_age_and_the_class() {
    let c = canvas(2560, 1440);
    let mut lines = Vec::new();
    for age in [0, 1, 2, 3, 4, 5] {
        lines.push(line(LineKind::Text, age, 1, false, false));
        lines.push(line(LineKind::Text, age, 1, true, false));
    }
    // Aged-out rows are left out and the rest close up: 10 rows remain,
    // the newest six shown (ages 2, 2, 3, 3, 4, 4).
    let walk = rows(lines);
    let shown = shown_lines(&walk, true);
    assert_eq!(
        shown.iter().map(|l| l.age).collect::<Vec<_>>(),
        [2, 2, 3, 3, 4, 4]
    );
    let mut on = blank(&c);
    let ys = paint(&mut on, &c, &walk, true, true);
    let colour = |rgba: &[u8], i: usize| at(rgba, &c, 19, ys[i] + 3);
    let pal = |i: u8| EGA_PALETTE_RGB[usize::from(i)];
    assert_eq!(
        (0..6).map(|i| colour(&on, i)).collect::<Vec<_>>(),
        [pal(7), pal(3), pal(8), pal(8), pal(8), pal(8)]
    );
    // Typed colour off: typed text takes the game colours.
    let mut off = blank(&c);
    let ys_off = paint(&mut off, &c, &walk, true, false);
    assert_eq!(ys, ys_off);
    assert_eq!(colour(&off, 1), pal(7));
    // Fresh rows: white and bright cyan.
    let fresh = rows(vec![
        line(LineKind::Text, 0, 1, false, false),
        line(LineKind::Text, 0, 1, true, false),
        line(LineKind::Prompt, 9, 1, false, false),
    ]);
    let mut rgba = blank(&c);
    let ys = paint(&mut rgba, &c, &fresh, true, true);
    let got: Vec<[u8; 3]> = ys.iter().map(|y| at(&rgba, &c, 19, y + 3)).collect();
    assert_eq!(got, [pal(15), pal(11), pal(15)]);
}

#[test]
fn every_glyph_pixel_gets_a_black_outline_clipped_to_the_map_window() {
    let c = canvas(2560, 1440);
    let mut rgba = blank(&c);
    let ys = paint(
        &mut rgba,
        &c,
        &rows(vec![line(LineKind::Text, 0, 1, false, false)]),
        true,
        true,
    );
    let y = ys[0] + 3;
    let black = [0, 0, 0];
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let (x, yy) = ((19 + dx) as u32, (y as i32 + dy) as u32);
            let want = if (dx, dy) == (0, 0) {
                EGA_PALETTE_RGB[15]
            } else {
                black
            };
            assert_eq!(at(&rgba, &c, x, yy), want, "({dx}, {dy})");
        }
    }
    // Nothing beyond the outline.
    assert_eq!(at(&rgba, &c, 21, y), [0x20; 3]);
    assert_eq!(at(&rgba, &c, 19, y + 2), [0x20; 3]);
    // A full cell on the bottom row: its outline stops at the map window,
    // leaving the bottom rule (y 232) and the band below alone.
    let mut rgba = blank(&c);
    paint(
        &mut rgba,
        &c,
        &rows(vec![line(LineKind::Text, 0, 1, false, true)]),
        true,
        true,
    );
    assert_eq!(at(&rgba, &c, 16, 231), EGA_PALETTE_RGB[15]);
    assert_eq!(at(&rgba, &c, 15, 231), black);
    assert_eq!(at(&rgba, &c, 16, 232), [0x20; 3]);
}

#[test]
fn the_live_row_carries_the_end_cap_merged_into_the_border() {
    let c = canvas(2560, 1440);
    let mut cap: CapSprite = [[None; 8]; 8];
    // Row 0: white at column 0 (the rule's own colour), row 1: blue at
    // columns 0-1 and white at 2-3.
    cap[0][0] = Some(RULE_WHITE);
    cap[1][0] = Some(BAND_BLUE);
    cap[1][1] = Some(BAND_BLUE);
    cap[1][2] = Some(RULE_WHITE);
    cap[1][3] = Some(RULE_WHITE);
    let mut rows = rows(vec![line(LineKind::Live, 0, 0, false, false)]);
    rows.cap = Some(cap);
    rows.cursor = Some(CrawlCell {
        cell: 0,
        byte: 5,
        mask: [0, 0, 0, 0x10, 0, 0, 0, 0],
        typed: false,
    });
    let mut rgba = blank(&c);
    let ys = paint(&mut rgba, &c, &rows, false, true);
    let y = ys[0];
    assert_eq!(y, 224);
    let pal = |i: u8| EGA_PALETTE_RGB[usize::from(i)];
    assert_eq!(at(&rgba, &c, 8, y), pal(15));
    assert_eq!(
        at(&rgba, &c, 7, y),
        [0x20; 3],
        "the rule stays where the cap is white"
    );
    assert_eq!(
        at(&rgba, &c, 7, y + 1),
        pal(1),
        "the border's blue flows in"
    );
    assert_eq!(at(&rgba, &c, 8, y + 1), pal(1));
    assert_eq!(at(&rgba, &c, 10, y + 1), pal(15));
    // The cursor, white with its outline, in the first text cell.
    assert_eq!(at(&rgba, &c, 19, y + 3), pal(15));
    assert_eq!(at(&rgba, &c, 18, y + 3), [0, 0, 0]);
}

#[test]
fn with_the_crawl_off_only_the_live_row_is_drawn() {
    let c = canvas(2560, 1440);
    let rows = rows(vec![
        line(LineKind::Text, 0, 3, false, false),
        line(LineKind::Prompt, 0, 3, false, false),
        line(LineKind::Live, 0, 1, false, false),
    ]);
    let shown = shown_lines(&rows, false);
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].kind, LineKind::Live);
    let mut rgba = blank(&c);
    assert_eq!(paint(&mut rgba, &c, &rows, false, true), [224]);
}

#[test]
fn the_crawl_never_covers_the_core() {
    // The widest crawl: six full rows of sixteen cells and the cursor past
    // the last, on every named layout and the panned narrow canvases.
    let mut lines: Vec<CrawlLine> = (0..5)
        .map(|_| line(LineKind::Text, 0, 16, false, true))
        .collect();
    lines.push(line(LineKind::Live, 0, 15, true, true));
    let mut rows = rows(lines);
    rows.cursor = Some(CrawlCell {
        cell: 15,
        byte: 5,
        mask: [0xff; 8],
        typed: false,
    });
    for (ww, wh) in [
        (2560, 1440),
        (3440, 1440),
        (1920, 1080),
        (1920, 1200),
        (1920, 1440),
        (1600, 1200),
    ] {
        let c = canvas(ww, wh);
        let mut rgba = blank(&c);
        paint(&mut rgba, &c, &rows, true, true);
        let (cx, cy, side) = core_rect(&c);
        for y in cy..cy + side {
            for x in cx..cx + side {
                assert_eq!(
                    at(&rgba, &c, x as u32, y as u32),
                    [0x20; 3],
                    "{ww}x{wh}: ({x}, {y})"
                );
            }
        }
    }
}
