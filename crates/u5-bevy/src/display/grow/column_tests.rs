//! Step 8, the docked column (design 2.2.2, 2.2.5, 2.2.7): the chrome and
//! its junctions pixel by pixel on the 240- and 216-row canvases, the crops
//! byte for byte, the open window's rows on the crawl's own pixels in their
//! fresh colours, the window's height and clip, and the core never covered.

use bevy::input::ButtonState;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, HostCapabilities, LockState, PixelAspect,
    ResolveInput, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, PlayState, TileGraphicsDepth, parse_ch_font};

use super::*;
use crate::display::crawl::layer::{CapSprite, CrawlCell, LineKind};
use crate::display::crawl::sync::CrawlState;
use crate::display::grow::compose::{core_rect, paint_chrome};
use crate::display::grow::crawl_paint::{Ink, cell_colour, shown_lines};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline, check_shell_equivalence};
use crate::{key_code_to_input_byte, key_code_to_line_input_byte};

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

fn index_at(rgba: &[u8], c: &Canvas, x: u32, y: u32) -> u8 {
    let i = ((y * c.w + x) * 4) as usize;
    crate::ega_palette_index_from_rgba(&rgba[i..i + 4])
}

/// A canvas with the chrome and a map window full of EGA 2 (green), so the
/// column's every pixel is told apart from the map.
fn chrome(c: &Canvas) -> Vec<u8> {
    let mut rgba = vec![0; c.w as usize * c.h as usize * 4];
    paint_chrome(&mut rgba, c);
    let (x0, y0, x1, y1) = c.map_window();
    let green = EGA_PALETTE_RGB[2];
    for y in y0..=y1 {
        for x in x0..=x1 {
            let i = ((y * c.w + x) * 4) as usize;
            rgba[i..i + 3].copy_from_slice(&green);
        }
    }
    rgba
}

/// The column's pixel for `(x, y)` on a canvas whose bottom rule is on
/// `rule` (232 on 240 rows, 208 on 216), written from the mockup's
/// construction (make_c04f.py `column`: the blue rect, the rule at x 151,
/// the three windows, the two junction runs) rather than from the code:
/// `b` blue, `W` white, `.` black, `?` outside the column.
fn expected(x: u32, y: u32, rule: u32) -> char {
    if !(15..=rule).contains(&y) || !(7..=151).contains(&x) {
        return '?';
    }
    if x == 151 {
        return 'W';
    }
    if (145..=150).contains(&x) {
        return 'b';
    }
    for (y0, y1, bottom) in [(16, 63, true), (72, 87, true), (96, rule - 1, false)] {
        let last = if bottom { y1 + 1 } else { y1 };
        if (y0 - 1..=last).contains(&y) {
            if (8..=143).contains(&x) && (y0..=y1).contains(&y) {
                return '.';
            }
            return 'W';
        }
    }
    if y == rule {
        // The screen's own bottom rule.
        return 'W';
    }
    'b'
}

fn glyph(index: u8) -> char {
    match index {
        1 => 'b',
        15 => 'W',
        0 => '.',
        _ => '?',
    }
}

#[test]
fn the_chrome_and_its_junctions_follow_the_mockup_pixel_for_pixel() {
    for ((ww, wh), rule) in [
        ((2560, 1440), 232),
        ((3440, 1440), 232),
        ((1920, 1080), 208),
        ((1920, 1200), 232),
    ] {
        let c = canvas(ww, wh);
        let mut rgba = chrome(&c);
        let before = rgba.clone();
        paint_column_chrome(&mut rgba, &c);
        for y in 0..c.h {
            for x in 0..c.w {
                let want = expected(x, y, rule);
                if want == '?' {
                    // Outside the column: untouched.
                    let i = ((y * c.w + x) * 4) as usize;
                    assert_eq!(rgba[i..i + 4], before[i..i + 4], "{ww}x{wh} ({x}, {y})");
                } else {
                    let got = glyph(index_at(&rgba, &c, x, y));
                    assert_eq!(got, want, "{ww}x{wh} ({x}, {y})");
                }
            }
        }
        // The band's blue runs into the outer border between the windows,
        // and through the top and bottom rules beside the map.
        for y in (65..=70).chain(89..=94) {
            assert_eq!(index_at(&rgba, &c, 6, y), 1);
            assert_eq!(index_at(&rgba, &c, 7, y), 1, "{ww}x{wh} y {y}");
        }
        for y in [15, rule] {
            for x in 145..=150 {
                assert_eq!(index_at(&rgba, &c, x, y), 1, "{ww}x{wh} ({x}, {y})");
            }
            assert_eq!(index_at(&rgba, &c, 144, y), 15);
            assert_eq!(index_at(&rgba, &c, 151, y), 15);
            assert_eq!(index_at(&rgba, &c, 152, y), 15);
        }
    }
}

/// A faithful frame whose every pixel differs from its neighbours.
fn faithful_pattern() -> Vec<u8> {
    (0..320 * 200)
        .flat_map(|i: usize| {
            let x = i % 320;
            let y = i / 320;
            [(x * 7) as u8, (y * 13) as u8, (x ^ y) as u8, 0xff]
        })
        .collect()
}

#[test]
fn the_roster_and_counters_are_the_frames_own_windows() {
    assert_eq!(
        (ROSTER_CROP.x, ROSTER_CROP.y, ROSTER_CROP.w, ROSTER_CROP.h),
        (192, 8, 120, 48)
    );
    assert_eq!(
        (
            COUNTERS_CROP.x,
            COUNTERS_CROP.y,
            COUNTERS_CROP.w,
            COUNTERS_CROP.h
        ),
        (192, 64, 120, 16)
    );
    let faithful = faithful_pattern();
    for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080)] {
        let c = canvas(ww, wh);
        let mut rgba = chrome(&c);
        paint_column(&mut rgba, &c, &faithful, None, false);
        for (rect, y0) in [(ROSTER_CROP, ROSTER_Y.0), (COUNTERS_CROP, COUNTERS_Y.0)] {
            for row in 0..rect.h {
                let at = (((y0 + row) * c.w + CROP_X) * 4) as usize;
                let from = (((rect.y + row) * 320 + rect.x) * 4) as usize;
                let width = rect.w as usize * 4;
                assert_eq!(rgba[at..at + width], faithful[from..from + width]);
            }
        }
        // Placed 8 pixels inside each window, 8 black columns each side.
        for y in (16..=63).chain(72..=87) {
            for x in (8..16).chain(136..144) {
                assert_eq!(index_at(&rgba, &c, x, y), 0, "{ww}x{wh} ({x}, {y})");
            }
        }
    }
}

#[test]
fn the_open_window_is_seventeen_rows_or_fourteen() {
    assert_eq!(open_window_rows(&canvas(2560, 1440)), 17);
    assert_eq!(open_window_rows(&canvas(3440, 1440)), 17);
    assert_eq!(open_window_rows(&canvas(1920, 1080)), 14);
    assert_eq!(message_window_y(&canvas(2560, 1440)), (96, 231));
    assert_eq!(message_window_y(&canvas(1920, 1080)), (96, 207));
    assert_eq!(
        open_window_rows(&canvas(2560, 1440)),
        crate::display::crawl::OPEN_WINDOW_ROWS_MAX
    );
}

#[test]
fn the_column_never_covers_the_core() {
    let faithful = faithful_pattern();
    for (ww, wh) in [
        (2560, 1440),
        (3440, 1440),
        (1920, 1080),
        (1920, 1200),
        (1920, 1440),
        (1600, 1200),
    ] {
        for page_mode in [false, true] {
            let c = canvas(ww, wh);
            let mut rgba = chrome(&c);
            let before = rgba.clone();
            paint_column(
                &mut rgba,
                &c,
                &faithful,
                Some(ColumnText {
                    rows: &walk_rows(),
                    typed_colour: true,
                }),
                page_mode,
            );
            let (cx, _, _) = core_rect(&c);
            assert!(cx as u32 > COLUMN_RULE_X, "{ww}x{wh}");
            // Nothing right of the column's rule is touched.
            for y in 0..c.h {
                for x in COLUMN_RULE_X + 1..c.w {
                    let i = ((y * c.w + x) * 4) as usize;
                    assert_eq!(rgba[i..i + 4], before[i..i + 4], "{ww}x{wh} ({x}, {y})");
                }
            }
        }
    }
}

fn line(kind: LineKind, age: u64, text: &str, typed: bool) -> CrawlLine {
    CrawlLine {
        kind,
        age,
        cells: text
            .bytes()
            .enumerate()
            .map(|(cell, byte)| CrawlCell {
                cell: cell as u8,
                byte,
                // Two ink pixels per cell.
                mask: [0, 0, 0, 0x10, 0x08, 0, 0, 0],
                typed,
            })
            .collect(),
        frame_row: 11,
    }
}

fn cap() -> CapSprite {
    let mut cap = [[None; 8]; 8];
    for (row, pixels) in cap.iter_mut().enumerate() {
        for (col, pixel) in pixels.iter_mut().enumerate() {
            if col <= row / 2 {
                *pixel = Some(if col == row / 2 { 15 } else { 1 });
            }
        }
    }
    cap
}

/// The mockup's walk (make_c04f.py `SCRIPT_B` at `NOW_B`, one row aged
/// out), its live row, and twenty older rows.
fn walk_rows() -> CrawlRows {
    CrawlRows {
        lines: vec![
            line(LineKind::Text, 9, "North", true),
            line(LineKind::Text, 4, "East", true),
            line(LineKind::Text, 2, "East", true),
            line(LineKind::Text, 2, "Slow progress!", false),
            line(LineKind::Text, 0, "North", true),
            line(LineKind::Live, 0, "", false),
        ],
        cursor: Some(CrawlCell {
            cell: 0,
            byte: 5,
            mask: [0xff; 8],
            typed: false,
        }),
        cap: Some(cap()),
        older: (0..20)
            .map(|i| line(LineKind::Text, 0, &format!("older {i}"), i % 2 == 0))
            .collect(),
        revision: 1,
    }
}

#[test]
fn the_crawl_keeps_its_pixels_when_the_column_opens() {
    for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080)] {
        let c = canvas(ww, wh);
        let rows = walk_rows();
        let mut closed = chrome(&c);
        let crawl = shown_lines(&rows, true);
        let closed_painted = paint_lines(&mut closed, &c, &crawl, &rows, c.map_window(), |l, t| {
            cell_colour(l, t, true)
        });
        let mut open = chrome(&c);
        paint_column_chrome(&mut open, &c);
        let open_painted = paint_open_window(
            &mut open,
            &c,
            ColumnText {
                rows: &rows,
                typed_colour: true,
            },
        );
        // The crawl's lines are the open window's bottom lines, on the
        // same y.
        let n = closed_painted.ys.len();
        let ys = &open_painted.ys;
        assert_eq!(ys[ys.len() - n..], closed_painted.ys[..], "{ww}x{wh}");
        // Every ink pixel of the crawl is ink in the open window at the
        // same place, in its class's fresh colour; nothing else is drawn on
        // the crawl's rows but the aged-out row's ink.
        let first_crawl_y = closed_painted.ys[0];
        for ink in &closed_painted.ink {
            let line = crawl[((ink.y - first_crawl_y) / 8) as usize];
            let typed = line.cells.first().is_some_and(|cell| cell.typed);
            let want = if line.kind == LineKind::Live {
                15
            } else {
                fresh_colour(typed, true)
            };
            assert_eq!(index_at(&open, &c, ink.x, ink.y), want, "{ww}x{wh} {ink:?}");
            assert!(open_painted.ink.contains(&Ink {
                colour: want,
                ..*ink
            }));
        }
        let on_crawl_rows = open_painted
            .ink
            .iter()
            .filter(|ink| ink.y >= first_crawl_y)
            .count();
        assert_eq!(on_crawl_rows, closed_painted.ink.len(), "{ww}x{wh}");
        // The aged-out "North" is above the crawl, fresh cyan.
        assert_eq!(ys.len(), open_window_rows(&c));
        assert_eq!(ys[0], MESSAGE_Y0);
        let aged_y = ys[ys.len() - n - 1];
        assert!(
            open_painted
                .ink
                .iter()
                .any(|ink| ink.y == aged_y + 3 && ink.colour == 11)
        );
        // The live row keeps its cap, merged into the border.
        let live_y = *ys.last().unwrap();
        for (row, pixels) in cap().iter().enumerate() {
            for (col, pixel) in pixels.iter().enumerate() {
                if let Some(index) = pixel {
                    let (x, y) = (8 + col as u32, live_y + row as u32);
                    assert_eq!(index_at(&open, &c, x, y), *index);
                    assert_eq!(index_at(&closed, &c, x, y), *index);
                }
            }
            let x7 = if pixels[0] == Some(1) { 1 } else { 15 };
            assert_eq!(index_at(&open, &c, 7, live_y + row as u32), x7);
        }
    }
}

#[test]
fn the_open_window_shows_aged_rows_and_older_ones_in_fresh_colours() {
    let rows = walk_rows();
    let c = canvas(2560, 1440);
    let lines = open_lines(&rows, open_window_rows(&c));
    assert_eq!(lines.len(), 17);
    let texts: Vec<String> = lines.iter().map(|line| line.text()).collect();
    // The newest older rows, then every placed line, the aged-out first
    // "North" (age 9) included.
    assert_eq!(texts[16], "");
    assert_eq!(texts[11], "North");
    assert_eq!(texts[10], "older 19");
    assert_eq!(texts[0], "older 9");
    assert_eq!(fresh_colour(true, true), 11);
    assert_eq!(fresh_colour(false, true), 15);
    assert_eq!(fresh_colour(true, false), 15);
    // On the 216-row canvas, fourteen.
    assert_eq!(
        open_lines(&rows, open_window_rows(&canvas(1920, 1080))).len(),
        14
    );
    // With the typed colour off, typed rows are game white.
    let mut rgba = chrome(&c);
    paint_column_chrome(&mut rgba, &c);
    let painted = paint_open_window(
        &mut rgba,
        &c,
        ColumnText {
            rows: &rows,
            typed_colour: false,
        },
    );
    assert!(painted.ink.iter().all(|ink| ink.colour == 15));
}

#[test]
fn the_open_windows_outline_stays_inside_its_rules() {
    let c = canvas(2560, 1440);
    let full = CrawlLine {
        kind: LineKind::Text,
        age: 0,
        cells: (0..16)
            .map(|cell| CrawlCell {
                cell,
                byte: b'#',
                mask: [0xff; 8],
                typed: false,
            })
            .collect(),
        frame_row: 11,
    };
    let rows = CrawlRows {
        lines: vec![full; 17],
        ..CrawlRows::default()
    };
    let mut rgba = chrome(&c);
    paint_column_chrome(&mut rgba, &c);
    let before = rgba.clone();
    paint_open_window(
        &mut rgba,
        &c,
        ColumnText {
            rows: &rows,
            typed_colour: true,
        },
    );
    for y in 95..=232 {
        for x in [7, 144, 145, 151] {
            let i = ((y * c.w + x) * 4) as usize;
            assert_eq!(rgba[i..i + 4], before[i..i + 4], "({x}, {y})");
        }
    }
    for x in 7..=151 {
        for y in [95, 232] {
            let i = ((y * c.w + x) * 4) as usize;
            assert_eq!(rgba[i..i + 4], before[i..i + 4], "({x}, {y})");
        }
    }
}

#[test]
fn the_column_starts_open_only_with_the_crawl_off() {
    assert!(!ColumnView::at_launch(true).open);
    assert!(ColumnView::at_launch(false).open);
    let state = ColumnState::new(ColumnView::at_launch(true));
    let shared = state.clone();
    state.toggle();
    assert!(shared.view().open);
    shared.toggle();
    assert!(!state.view().open);
    state.set_open(true);
    assert_eq!(
        shared.view(),
        ColumnView {
            open: true,
            step: None,
            page_mode: false
        }
    );
}

// ---------------------------------------------------------------- the shell

fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &[DISPLAY_P1, DISPLAY_P2, DISPLAY_P8],
        ),
        ..VisualLaunchConfig::classic()
    }
}

fn grow_on() -> VisualLaunchConfig {
    launch(&[CliSetting::Enable("display.grow_left".into())])
}

/// A font whose glyphs all have ink and differ.
fn scene() -> Scene {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    for code in 0..CH_FONT_LEN / 8 {
        let c = code as u8;
        let rows = [c | 1, !c, c | 0x80, c, !c | 1, c ^ 0x55, c | 0x10, 0x81];
        bytes[code * 8..code * 8 + 8].copy_from_slice(&rows);
    }
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    Scene {
        game_dir: ".".into(),
        atlas: synthetic_tile_atlas(TileGraphicsDepth::Ega16),
        text_font: font.clone(),
        rune_font: font,
    }
}

fn spawn_window(app: &mut App) {
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(2560.0, 1440.0),
        ..default()
    });
}

fn shell(launch: &VisualLaunchConfig, state: PlayState) -> Shell {
    let mut shell = Shell::new(state, &scene(), launch, |_| {});
    spawn_window(&mut shell.app);
    shell
}

fn talking() -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    crate::seed_visual_key_route_conversation(&mut state);
    state
}

/// Ctrl+P at `frame`: Ctrl held around a tap of P, all the feature's.
fn ctrl_p(timeline: &mut Timeline, frame: usize) {
    timeline
        .event(
            frame,
            KeyCode::ControlLeft,
            ButtonState::Pressed,
            false,
            Owner::Feature,
        )
        .event(
            frame,
            KeyCode::KeyP,
            ButtonState::Pressed,
            false,
            Owner::Feature,
        )
        .event(
            frame + 1,
            KeyCode::KeyP,
            ButtonState::Released,
            false,
            Owner::Feature,
        )
        .event(
            frame + 1,
            KeyCode::ControlLeft,
            ButtonState::Released,
            false,
            Owner::Feature,
        );
}

fn presented(shell: &mut Shell) -> (UVec2, Vec<u8>) {
    let world = shell.app.world_mut();
    let mut sprites = world.query::<&Sprite>();
    let handle = sprites.single(world).unwrap().image.clone();
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    (image.size(), image.data.clone().unwrap())
}

fn faithful(shell: &Shell) -> Vec<u8> {
    let handle = shell.visual().image_handle.clone();
    shell
        .app
        .world()
        .resource::<Assets<Image>>()
        .get(&handle)
        .and_then(|image| image.data.clone())
        .unwrap()
}

fn open(shell: &Shell) -> bool {
    shell.app.world().resource::<ColumnState>().view().open
}

#[test]
fn the_column_exists_only_with_grow_left() {
    let mut off = shell(&launch(&[]), test_state(open_grid(), 5, 5));
    off.step(&[]);
    assert!(!off.app.world().contains_resource::<ColumnState>());
    let mut on = shell(&grow_on(), test_state(open_grid(), 5, 5));
    on.step(&[]);
    assert!(!open(&on));
    // With the crawl off the column starts open.
    let mut quiet = shell(
        &launch(&[
            CliSetting::Enable("display.grow_left".into()),
            CliSetting::Disable("display.crawl".into()),
        ]),
        test_state(open_grid(), 5, 5),
    );
    quiet.step(&[]);
    assert!(open(&quiet));
}

#[test]
fn ctrl_p_opens_and_closes_the_presented_column() {
    // In one frame: `display.dither` off (the effect is M1-15's tests).
    let one_frame = launch(&[
        CliSetting::Enable("display.grow_left".into()),
        CliSetting::Disable("display.dither".into()),
    ]);
    let mut shell = shell(&one_frame, test_state(open_grid(), 5, 5));
    let mut timeline = Timeline::new(60);
    timeline.game_tap(3, KeyCode::ArrowUp);
    ctrl_p(&mut timeline, 20);
    ctrl_p(&mut timeline, 40);
    let mut seen = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let (size, rgba) = presented(&mut shell);
        assert_eq!(size, UVec2::new(512, 240));
        let c = canvas(2560, 1440);
        let column_rule = index_at(&rgba, &c, COLUMN_RULE_X, 120);
        let between = index_at(&rgba, &c, 7, 67);
        seen.push((open(&shell), column_rule, between));
        if open(&shell) {
            // The roster and counters are the faithful frame's own.
            let frame_rgba = faithful(&shell);
            for (rect, y0) in [(ROSTER_CROP, ROSTER_Y.0), (COUNTERS_CROP, COUNTERS_Y.0)] {
                for row in 0..rect.h {
                    let at = (((y0 + row) * c.w + CROP_X) * 4) as usize;
                    let from = (((rect.y + row) * 320 + rect.x) * 4) as usize;
                    let width = rect.w as usize * 4;
                    assert_eq!(rgba[at..at + width], frame_rgba[from..from + width]);
                }
            }
        }
    }
    // Closed before the first Ctrl+P, open after it, closed after the
    // second: the column's rule and the band's blue in the left border
    // come and go with it, on the frame the key is pressed.
    assert!(!seen[19].0);
    assert!(seen[20].0 && seen[39].0 && !seen[40].0 && !seen[59].0);
    for (open, rule, between) in &seen {
        if *open {
            assert_eq!((*rule, *between), (15, 1));
        } else {
            assert_eq!(*between, 15);
        }
    }
    // The history: older rows only while open.
    let crawl = shell.app.world().resource::<CrawlState>();
    assert!(crawl.rows().older.is_empty());
}

#[test]
fn ctrl_p_is_no_game_byte_in_either_input_mode() {
    for shift in [false, true] {
        assert_eq!(key_code_to_input_byte(KeyCode::KeyP, shift, true), None);
        assert_eq!(
            key_code_to_line_input_byte(KeyCode::KeyP, shift, true),
            None
        );
    }
    // Walking (single keys) and in a conversation (a typed line, Ctrl+P
    // between the letters and before the submit): the game's trajectory,
    // frame and queue are those of Classic without the key.
    let mut walk = Timeline::new(90);
    walk.game_tap(4, KeyCode::ArrowUp);
    ctrl_p(&mut walk, 10);
    walk.game_tap(12, KeyCode::ArrowLeft);
    ctrl_p(&mut walk, 30);
    ctrl_p(&mut walk, 31);
    walk.game_tap(50, KeyCode::ArrowDown);
    ctrl_p(&mut walk, 70);
    let mut talk = Timeline::new(90);
    talk.game_tap(4, KeyCode::KeyJ);
    ctrl_p(&mut talk, 8);
    talk.game_tap(12, KeyCode::KeyO);
    ctrl_p(&mut talk, 16);
    talk.game_tap(20, KeyCode::KeyB);
    talk.game_tap(30, KeyCode::Enter);
    ctrl_p(&mut talk, 40);
    talk.game_tap(50, KeyCode::KeyB);
    talk.game_tap(54, KeyCode::KeyY);
    talk.game_tap(58, KeyCode::KeyE);
    ctrl_p(&mut talk, 60);
    talk.game_tap(64, KeyCode::Enter);
    for (start, timeline) in [(test_state(open_grid(), 5, 5), &walk), (talking(), &talk)] {
        for launch in [
            grow_on(),
            launch(&[
                CliSetting::Enable("display.grow_left".into()),
                CliSetting::Disable("display.crawl".into()),
            ]),
        ] {
            let mismatches =
                check_shell_equivalence(&start, &scene(), timeline, &launch, spawn_window);
            assert_eq!(mismatches, []);
        }
    }
}

#[test]
fn the_open_column_shows_older_rows_from_the_transcript() {
    let mut shell = shell(&grow_on(), test_state(open_grid(), 5, 5));
    let mut timeline = Timeline::new(400);
    let keys = [
        KeyCode::ArrowUp,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
    ];
    for (index, key) in keys.iter().cycle().take(30).enumerate() {
        timeline.game_tap(5 + index * 12, *key);
    }
    ctrl_p(&mut timeline, 200);
    let mut older_seen = 0;
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let crawl = shell.app.world().resource::<CrawlState>();
        let rows = crawl.rows();
        if frame < 200 {
            assert!(rows.older.is_empty(), "frame {frame}");
            continue;
        }
        // Thirty moves: the open window is full, its older rows are the
        // moves' echoes, all text the transcript held.
        let lines = open_lines(&rows, 17);
        if lines.len() == 17 {
            older_seen += 1;
        }
        let held: Vec<String> = shell
            .visual()
            .state
            .message_entries()
            .iter()
            .map(|entry| entry.text.clone())
            .chain(
                crawl
                    .history()
                    .committed()
                    .iter()
                    .map(|entry| entry.text.clone()),
            )
            .collect();
        for line in &rows.older {
            let text = line.text();
            assert!(
                held.iter().any(|entry| entry.contains(text.trim())),
                "frame {frame}: {text:?}"
            );
        }
        assert_eq!(crawl.unaligned_syncs(), 0, "frame {frame}");
    }
    assert!(older_seen > 0, "the open window never filled");
}
