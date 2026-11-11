//! Page mode, narrow canvases and `centre_party` (design 2.2.5, 2.2.7,
//! 2.2.11, D2.20, D2.21; build plan M1-14): the column shown by itself
//! during a Z-stats page, a picker, an arms-sell browser and the inn's
//! register, with the original's message window in its own; the player's
//! column state back when the page goes; the core visible at 16:10 and 4:3
//! in every state; the centring pan exactly 4 tiles.

use bevy::input::ButtonState;
use bevy::window::WindowResolution;
use u5_runtime::enhancements::{
    CliSetting, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, HostCapabilities, LockState, ResolveInput,
    resolve_as_shipped,
};
use u5_runtime::shop_runtime::{ArmsSellBrowser, ArmsShopState, InnkeeperState};
use u5_runtime::shop_session::ActiveShopSession;
use u5_runtime::shops::{INN_REGISTRY_CAP, Inn};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::z_stats::UseSession;
use u5_runtime::{TileAtlas, TileGraphicsDepth};

use super::column::{
    COLUMN_RULE_X, COUNTERS_CROP, COUNTERS_Y, CROP_X, ColumnState, FrameRect, MESSAGE_CROP,
    PAGE_PANEL, PageRow, ROSTER_CROP, ROSTER_Y, WINDOW_X0, WINDOW_X1, message_crop_origin,
    message_window_y, page_mode_active, page_row_kind,
};
use super::compose::{BAND_BLUE, RULE_WHITE, core_rect};
use super::crawl_paint::CrawlPaint;
use super::*;
use crate::EGA_PALETTE_RGB;
use crate::display::crawl::layer::{CrawlCell, CrawlLine, CrawlRows, LineKind};
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline, check_shell_equivalence};

fn canvas(w: u32, h: u32) -> Canvas {
    Canvas::for_window(w, h, PixelAspect::Dos).unwrap()
}

fn atlas() -> TileAtlas {
    synthetic_tile_atlas(TileGraphicsDepth::Ega16)
}

/// A faithful frame whose every pixel differs from its neighbours.
fn faithful_pattern() -> Vec<u8> {
    (0..(320 * 200))
        .flat_map(|i: usize| {
            let [r, g, b] = EGA_PALETTE_RGB[(i * 7 + i / 320 + 3) % 16];
            [r, g, b, 0xff]
        })
        .collect()
}

fn index_at(rgba: &[u8], c: &Canvas, x: u32, y: u32) -> u8 {
    let i = ((y * c.w + x) * 4) as usize;
    crate::ega_palette_index_from_rgba(&rgba[i..i + 4])
}

/// The four kinds of page, and the plain roster.
#[derive(Clone, Copy, Debug)]
enum Page {
    None,
    ZStats,
    Picker,
    ArmsSell,
    InnRegister,
}

const PAGES: [Page; 4] = [
    Page::ZStats,
    Page::Picker,
    Page::ArmsSell,
    Page::InnRegister,
];

fn with_page(page: Page) -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    match page {
        Page::None => {}
        Page::ZStats => {
            state.z_stats_for_party(0);
        }
        Page::Picker => state.active_use = Some(UseSession::new()),
        Page::ArmsSell => {
            state.active_shop = Some(ActiveShopSession::Arms(ArmsShopState::SellPickItem(
                ArmsSellBrowser {
                    selected: 0,
                    page_start: 0,
                },
            )));
        }
        Page::InnRegister => {
            state.active_shop = Some(ActiveShopSession::Innkeeper(
                InnkeeperState::PickUpCompanion {
                    inn: Inn::TheWayfarerInn,
                    guest_indices: [0; INN_REGISTRY_CAP],
                    guest_count: 1,
                    base_lodging_charge: 0,
                    highlight: 0,
                },
            ));
        }
    }
    state
}

fn line(kind: LineKind, text: &str, typed: bool) -> CrawlLine {
    CrawlLine {
        kind,
        age: 0,
        cells: text
            .bytes()
            .enumerate()
            .map(|(cell, byte)| CrawlCell {
                cell: cell as u8,
                byte,
                mask: [0xff, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xff],
                typed,
            })
            .collect(),
        frame_row: 11,
    }
}

/// Sixteen-cell rows, as wide as the original's window, and older rows.
fn rows() -> CrawlRows {
    CrawlRows {
        lines: vec![
            line(LineKind::Text, "Slow progress!!!", false),
            line(LineKind::Text, "North", true),
            line(LineKind::Live, "WWWWWWWWWWWWWWWW", false),
        ],
        cursor: None,
        cap: None,
        older: (0..20)
            .map(|i| line(LineKind::Text, &format!("older row {i:6}"), i % 2 == 0))
            .collect(),
        revision: 1,
    }
}

fn options(rows: &CrawlRows, open: bool, centre_party: bool) -> ComposeOptions<'_> {
    ComposeOptions {
        light: PeekLightPolicy::Scaled,
        bands: None,
        crawl: Some(CrawlPaint {
            rows,
            crawl: true,
            typed_colour: true,
        }),
        column: ColumnView {
            open,
            ..ColumnView::default()
        },
        centre_party,
        composite: None,
    }
}

fn crop_is_frame(rgba: &[u8], c: &Canvas, faithful: &[u8], rect: FrameRect, at: (u32, u32)) {
    for row in 0..rect.h {
        let to = (((at.1 + row) * c.w + at.0) * 4) as usize;
        let from = (((rect.y + row) * 320 + rect.x) * 4) as usize;
        let width = rect.w as usize * 4;
        assert_eq!(
            rgba[to..to + width],
            faithful[from..from + width],
            "row {row} of {rect:?}"
        );
    }
}

/// The core on `rgba` is the faithful viewport, byte for byte.
fn core_is_faithful(rgba: &[u8], c: &Canvas, faithful: &[u8]) -> bool {
    let (cx, cy, side) = core_rect(c);
    let w = c.w as usize;
    (0..side).all(|row| {
        let at = ((cy + row) * w + cx) * 4;
        let from = ((8 + row) * 320 + 8) * 4;
        rgba[at..at + side * 4] == faithful[from..from + side * 4]
    })
}

// ------------------------------------------------------------- page mode

#[test]
fn page_mode_follows_the_originals_right_column() {
    assert!(!page_mode_active(&with_page(Page::None)));
    for page in PAGES {
        let mut state = with_page(page);
        assert!(page_mode_active(&state), "{page:?}");
        state.active_z_stats = None;
        state.active_use = None;
        state.active_shop = None;
        assert!(!page_mode_active(&state), "{page:?} gone");
    }
    // An arms shop that is not browsing is still the shop's page.
    let mut shop = with_page(Page::None);
    shop.active_shop = Some(ActiveShopSession::Arms(ArmsShopState::default()));
    assert!(page_mode_active(&shop));
}

#[test]
fn the_message_crop_is_the_originals_window_on_the_crawls_bottom_row() {
    assert_eq!(
        (
            MESSAGE_CROP.x,
            MESSAGE_CROP.y,
            MESSAGE_CROP.w,
            MESSAGE_CROP.h
        ),
        (192, 88, 128, 104)
    );
    // Bottom-aligned: the original's live row (frame y 184..191) lands on
    // the crawl's bottom row, its last column on the window's last.
    assert_eq!(message_crop_origin(&canvas(2560, 1440)), (16, 128));
    assert_eq!(message_crop_origin(&canvas(3440, 1440)), (16, 128));
    assert_eq!(message_crop_origin(&canvas(1920, 1080)), (16, 104));
    for (ww, wh) in [(2560, 1440), (1920, 1080)] {
        let c = canvas(ww, wh);
        let (x, y) = message_crop_origin(&c);
        assert_eq!(y + MESSAGE_CROP.h - 8, crawl_paint::crawl_bottom_y(&c));
        assert_eq!(x + MESSAGE_CROP.w - 1, WINDOW_X1);
        assert!(y >= message_window_y(&c).0);
    }
}

#[test]
fn a_page_shows_the_column_with_the_originals_windows() {
    let faithful = faithful_pattern();
    let rows = rows();
    for page in PAGES {
        let state = with_page(page);
        for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080), (1600, 1200)] {
            let c = canvas(ww, wh);
            let context = format!("{page:?} {ww}x{wh}");
            // The player's column is closed: the page opens it anyway.
            let rgba = compose_for_state(
                &faithful,
                &state,
                &atlas(),
                &c,
                options(&rows, false, false),
            )
            .expect("the top-down view");
            for y in 15..=c.map_window().3 + 1 {
                assert_eq!(
                    index_at(&rgba, &c, COLUMN_RULE_X, y),
                    RULE_WHITE,
                    "{context} y {y}"
                );
            }
            // The whole panel, the roster and counters windows with it.
            crop_is_frame(&rgba, &c, &faithful, PAGE_PANEL, (CROP_X, 16));
            crop_is_frame(&rgba, &c, &faithful, ROSTER_CROP, (CROP_X, ROSTER_Y.0));
            crop_is_frame(&rgba, &c, &faithful, COUNTERS_CROP, (CROP_X, COUNTERS_Y.0));
            let origin = message_crop_origin(&c);
            crop_is_frame(&rgba, &c, &faithful, MESSAGE_CROP, origin);
            // No crawl, no history: the window is black above the crop and
            // left of it.
            let (y0, y1) = message_window_y(&c);
            for y in y0..=y1 {
                for x in WINDOW_X0..=WINDOW_X1 {
                    if y < origin.1 || x < origin.0 {
                        assert_eq!(index_at(&rgba, &c, x, y), 0, "{context} ({x}, {y})");
                    }
                }
            }
            assert!(core_is_faithful(&rgba, &c, &faithful), "{context}");
            // Open by the player as well: the same picture.
            let open =
                compose_for_state(&faithful, &state, &atlas(), &c, options(&rows, true, false))
                    .unwrap();
            assert!(
                open == rgba,
                "{context}: the player's state shows through a page"
            );
        }
    }
}

fn set(frame: &mut [u8], x: u32, y: u32, index: u8) {
    let at = ((y * 320 + x) * 4) as usize;
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index)];
    frame[at..at + 4].copy_from_slice(&[r, g, b, 0xff]);
}

/// The frame's right panel drawn the way a page draws it: the window rule
/// at x 191 and 312, `rules` rows white across with a black badge gap,
/// `bands` rows blue with a white badge glyph; the rest keeps the pattern
/// (text), with black at the window's first and last columns.
fn panel_frame(rules: &[u32], bands: &[u32]) -> Vec<u8> {
    let mut frame = faithful_pattern();
    for y in 7..=80 {
        let band = bands.contains(&y);
        let rule = rules.contains(&y);
        let edge = if band { BAND_BLUE } else { RULE_WHITE };
        set(&mut frame, 191, y, edge);
        set(&mut frame, 312, y, edge);
        for x in 192..=311 {
            let badge = (241..=262).contains(&x);
            if rule {
                set(&mut frame, x, y, if badge { 0 } else { RULE_WHITE });
            } else if band {
                set(
                    &mut frame,
                    x,
                    y,
                    if badge && x % 3 == 0 {
                        RULE_WHITE
                    } else {
                        BAND_BLUE
                    },
                );
            } else if x == 192 || x == 311 {
                set(&mut frame, x, y, 0);
            }
        }
    }
    frame
}

#[test]
fn the_page_panel_follows_the_frames_own_rows() {
    let rows = rows();
    // Z-stats, the pickers and the register: one window over y 8..79. The
    // arms-sell browser: the roster window, its badge on the band, food and
    // gold.
    let merged = panel_frame(&[7, 80], &[]);
    let arms = panel_frame(&[7, 56, 63, 80], &[57, 58, 59, 60, 61, 62]);
    for (name, frame, page) in [
        ("merged", &merged, Page::ZStats),
        ("arms", &arms, Page::ArmsSell),
    ] {
        let state = with_page(page);
        for fy in 8..=80 {
            let want = if name == "arms" && (57..=62).contains(&fy) {
                PageRow::Band
            } else if fy == 80 || (name == "arms" && (fy == 56 || fy == 63)) {
                PageRow::Rule
            } else {
                PageRow::Window
            };
            assert_eq!(page_row_kind(frame, fy), want, "{name} frame row {fy}");
        }
        for (ww, wh) in [(2560, 1440), (1920, 1080)] {
            let c = canvas(ww, wh);
            let rgba = compose_for_state(frame, &state, &atlas(), &c, options(&rows, false, false))
                .unwrap();
            crop_is_frame(&rgba, &c, frame, PAGE_PANEL, (CROP_X, 16));
            for y in 16..=88 {
                let context = format!("{name} {ww}x{wh} y {y}");
                let (edge, margin) = match page_row_kind(frame, y - 8) {
                    PageRow::Window => (RULE_WHITE, 0),
                    PageRow::Rule => (RULE_WHITE, RULE_WHITE),
                    PageRow::Band => (BAND_BLUE, BAND_BLUE),
                };
                assert_eq!(index_at(&rgba, &c, 7, y), edge, "{context}");
                assert_eq!(index_at(&rgba, &c, 144, y), edge, "{context}");
                for x in (8..16).chain(136..144) {
                    assert_eq!(index_at(&rgba, &c, x, y), margin, "{context} x {x}");
                }
                // The column's own band and rule to the right never change.
                for x in 145..=150 {
                    assert_eq!(index_at(&rgba, &c, x, y), BAND_BLUE, "{context} x {x}");
                }
                assert_eq!(
                    index_at(&rgba, &c, COLUMN_RULE_X, y),
                    RULE_WHITE,
                    "{context}"
                );
            }
            // The page's window runs through where the column's band was.
            let between = index_at(&rgba, &c, 7, 67);
            assert_eq!(
                between,
                if name == "arms" {
                    BAND_BLUE
                } else {
                    RULE_WHITE
                },
                "{name}"
            );
            // The top rule is the screen's; no title is drawn over it.
            for x in 7..=144 {
                assert_eq!(
                    index_at(&rgba, &c, x, 15),
                    RULE_WHITE,
                    "{name} top rule x {x}"
                );
            }
        }
    }
}

#[test]
fn without_a_page_the_column_is_the_players() {
    let faithful = faithful_pattern();
    let rows = rows();
    let state = with_page(Page::None);
    let c = canvas(2560, 1440);
    let closed = compose_for_state(
        &faithful,
        &state,
        &atlas(),
        &c,
        options(&rows, false, false),
    )
    .unwrap();
    // Closed: the left rule is unbroken; the crawl is drawn.
    assert_eq!(index_at(&closed, &c, 7, 67), RULE_WHITE);
    let open =
        compose_for_state(&faithful, &state, &atlas(), &c, options(&rows, true, false)).unwrap();
    assert_eq!(index_at(&open, &c, 7, 67), BAND_BLUE);
    // Open without a page: the rows, not the crop. The crop's first row
    // (frame y 88) would sit at y 128; the open window's row there is a
    // text row of the history, which differs from the frame's.
    let origin = message_crop_origin(&c);
    let to = ((origin.1 * c.w + origin.0) * 4) as usize;
    let from = ((88 * 320 + 192) * 4) as usize;
    assert_ne!(open[to..to + 512], faithful[from..from + 512]);
    // A page with the column closed and the column open by the player give
    // one picture, and a page gone gives the player's picture back.
    let paged = with_page(Page::ZStats);
    let shown = compose_for_state(
        &faithful,
        &paged,
        &atlas(),
        &c,
        options(&rows, false, false),
    )
    .unwrap();
    assert_ne!(shown, closed);
}

// ------------------------------------------------- the shell: the round trip

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

fn grow_on(extra: &[&str]) -> VisualLaunchConfig {
    let mut cli = vec![CliSetting::Enable("display.grow_left".into())];
    cli.extend(extra.iter().map(|key| CliSetting::Enable((*key).into())));
    launch(&cli)
}

fn scene() -> Scene {
    let mut bytes = vec![0u8; u5_runtime::CH_FONT_LEN];
    for code in 0..u5_runtime::CH_FONT_LEN / 8 {
        let c = code as u8;
        let rows = [c | 1, !c, c | 0x80, c, !c | 1, c ^ 0x55, c | 0x10, 0x81];
        bytes[code * 8..code * 8 + 8].copy_from_slice(&rows);
    }
    let font = u5_runtime::parse_ch_font(&bytes, u5_runtime::IBM_CH_FILE).unwrap();
    Scene {
        game_dir: ".".into(),
        atlas: atlas(),
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

fn ctrl_p(timeline: &mut Timeline, frame: usize) {
    for (at, state) in [
        (frame, ButtonState::Pressed),
        (frame + 1, ButtonState::Released),
    ] {
        timeline.event(at, KeyCode::ControlLeft, state, false, Owner::Feature);
        timeline.event(at, KeyCode::KeyP, state, false, Owner::Feature);
    }
}

fn presented(shell: &mut Shell) -> (UVec2, Vec<u8>) {
    let world = shell.app.world_mut();
    let mut sprites = world.query::<&Sprite>();
    let handle = sprites.single(world).unwrap().image.clone();
    let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
    (image.size(), image.data.clone().unwrap())
}

/// Each frame: whether a page is up, whether the presented column is
/// shown, and the player's own column state.
fn run(timeline: &Timeline) -> Vec<(bool, bool, bool)> {
    let mut shell = Shell::new(with_page(Page::ZStats), &scene(), &grow_on(&[]), |_| {});
    spawn_window(&mut shell.app);
    let c = canvas(2560, 1440);
    let mut seen = Vec::new();
    for frame in 0..timeline.len() {
        shell.step(timeline.frame(frame));
        let (size, rgba) = presented(&mut shell);
        assert_eq!(size, UVec2::new(c.w, c.h));
        let page = page_mode_active(&shell.visual().state);
        // The column's band and rule beside the map (a page may run its
        // window through the band at x 7).
        let shown = [(145, 120), (150, 40)]
            .iter()
            .all(|&(x, y)| index_at(&rgba, &c, x, y) == BAND_BLUE)
            && index_at(&rgba, &c, COLUMN_RULE_X, 120) == RULE_WHITE;
        let open = shell.app.world().resource::<ColumnState>().view().open;
        seen.push((page, shown, open));
    }
    seen
}

#[test]
fn the_column_returns_to_the_players_state_when_the_page_goes() {
    // Escape leaves the Z-stats page.
    let mut plain = Timeline::new(40);
    plain.game_tap(20, KeyCode::Escape);
    let seen = run(&plain);
    assert!(
        seen[..20]
            .iter()
            .all(|&(page, shown, open)| page && shown && !open),
        "{seen:?}"
    );
    let after = seen
        .iter()
        .position(|(page, ..)| !page)
        .expect("the page goes");
    assert!(
        seen[after..]
            .iter()
            .all(|&(_, shown, open)| !shown && !open),
        "{seen:?}"
    );
    // Ctrl+P during the page flips only the player's state, which shows
    // once the page goes.
    let mut toggled = Timeline::new(40);
    ctrl_p(&mut toggled, 8);
    toggled.game_tap(20, KeyCode::Escape);
    let seen = run(&toggled);
    assert!(
        seen[..8].iter().all(|&(_, shown, open)| shown && !open),
        "{seen:?}"
    );
    assert!(
        seen[8..20]
            .iter()
            .all(|&(page, shown, open)| page && shown && open),
        "{seen:?}"
    );
    let after = seen
        .iter()
        .position(|(page, ..)| !page)
        .expect("the page goes");
    assert!(
        seen[after..].iter().all(|&(_, shown, open)| shown && open),
        "{seen:?}"
    );
}

#[test]
fn page_mode_and_the_centring_pan_leave_the_game_alone() {
    let mut timeline = Timeline::new(60);
    ctrl_p(&mut timeline, 6);
    timeline.game_tap(12, KeyCode::ArrowDown);
    timeline.game_tap(24, KeyCode::Escape);
    timeline.game_tap(36, KeyCode::ArrowUp);
    ctrl_p(&mut timeline, 44);
    for launch in [grow_on(&[]), grow_on(&["display.centre_party"])] {
        let mismatches = check_shell_equivalence(
            &with_page(Page::ZStats),
            &scene(),
            &timeline,
            &launch,
            spawn_window,
        );
        assert_eq!(mismatches, []);
    }
}

// ------------------------------------------------------- narrow canvases

#[test]
fn the_core_stays_visible_at_16_10_and_4_3() {
    let faithful = faithful_pattern();
    let rows = rows();
    // (window, canvas width, narrow pan)
    for ((ww, wh), w, pan) in [
        ((1920, 1200), 448, 1),
        ((1600, 1200), 384, 3),
        ((1680, 1050), 448, 1),
    ] {
        let c = canvas(ww, wh);
        assert_eq!((c.w, c.pan_tiles), (w, pan), "{ww}x{wh}");
        for page in [Page::None, Page::ZStats] {
            let state = with_page(page);
            for open in [false, true] {
                for centre_party in [false, true] {
                    let context = format!("{ww}x{wh} {page:?} open {open} centre {centre_party}");
                    let rgba = compose_for_state(
                        &faithful,
                        &state,
                        &atlas(),
                        &c,
                        options(&rows, open, centre_party),
                    )
                    .expect("the top-down view");
                    let view = ColumnView {
                        open,
                        page_mode: page_mode_active(&state),
                        ..ColumnView::default()
                    };
                    let drawn = c.for_column(view, centre_party);
                    assert!(core_is_faithful(&rgba, &drawn, &faithful), "{context}");
                    let (cx, _, side) = core_rect(&drawn);
                    assert!(cx as u32 > COLUMN_RULE_X, "{context}");
                    assert!(cx + side <= drawn.map_window().2 as usize + 1, "{context}");
                }
            }
        }
    }
    // Narrower than 4:3: the classic frame.
    assert!(Canvas::for_window(1280, 1024, PixelAspect::Dos).is_none());
}

#[test]
fn the_crawl_never_reaches_the_core_on_a_narrow_canvas() {
    for (ww, wh) in [(1920, 1200), (1600, 1200)] {
        let c = canvas(ww, wh);
        let mut rgba = vec![0; (c.w * c.h * 4) as usize];
        let before = rgba.clone();
        crawl_paint::paint_crawl(
            &mut rgba,
            &c,
            CrawlPaint {
                rows: &rows(),
                crawl: true,
                typed_colour: true,
            },
        );
        let (cx, cy, side) = core_rect(&c);
        let w = c.w as usize;
        for y in cy..cy + side {
            let at = (y * w + cx) * 4;
            assert_eq!(
                rgba[at..at + side * 4],
                before[at..at + side * 4],
                "{ww}x{wh} row {y}"
            );
        }
        assert_ne!(rgba, before, "{ww}x{wh}: the crawl drew nothing");
    }
}

// --------------------------------------------------------- centre_party

#[test]
fn centre_party_pans_exactly_four_tiles() {
    assert_eq!(CENTRE_PARTY_TILES, 4);
    let open = ColumnView {
        open: true,
        ..ColumnView::default()
    };
    let page = ColumnView {
        page_mode: true,
        ..ColumnView::default()
    };
    for (ww, wh) in [(2560, 1440), (3440, 1440), (1920, 1080), (2560, 1080)] {
        let c = canvas(ww, wh);
        assert_eq!(c.pan_tiles, 0);
        for shown in [open, page] {
            let panned = c.for_column(shown, true);
            assert_eq!(panned.party_x, c.party_x + 64, "{ww}x{wh}");
            assert_eq!(panned.party_y, c.party_y, "{ww}x{wh}");
            // Mid-view: the party tile's centre within half a tile of the
            // visible map's centre.
            let (left, right) = (COLUMN_RULE_X + 1, c.map_window().2 + 1);
            let visible = left + (right - left) / 2;
            assert!(visible.abs_diff(panned.party_x + 8) <= 8, "{ww}x{wh}");
            assert_eq!(c.for_column(shown, false), c, "{ww}x{wh}: off never moves");
        }
        assert_eq!(
            c.for_column(ColumnView::default(), true),
            c,
            "{ww}x{wh}: closed"
        );
    }
    // A narrow canvas's own pan counts toward the four tiles.
    for (ww, wh) in [(1920, 1200), (1600, 1200)] {
        let c = canvas(ww, wh);
        let panned = c.for_column(open, true);
        assert_eq!(panned.party_x, (c.w - 16) / 2 + 64, "{ww}x{wh}");
        assert_eq!(panned.pan_tiles, 4, "{ww}x{wh}");
    }
}

#[test]
fn centre_party_moves_the_whole_map_with_the_party() {
    let faithful = faithful_pattern();
    let rows = rows();
    let state = with_page(Page::None);
    let c = canvas(2560, 1440);
    let still =
        compose_for_state(&faithful, &state, &atlas(), &c, options(&rows, true, false)).unwrap();
    let panned =
        compose_for_state(&faithful, &state, &atlas(), &c, options(&rows, true, true)).unwrap();
    let moved = c.for_column(
        ColumnView {
            open: true,
            ..ColumnView::default()
        },
        true,
    );
    assert!(core_is_faithful(&still, &c, &faithful));
    assert!(core_is_faithful(&panned, &moved, &faithful));
    // Every map pixel right of the column's rule and clear of the right
    // edge is the unpanned pixel 64 to its left.
    let (_, my0, mx1, my1) = c.map_window();
    for y in my0..=my1 {
        for x in COLUMN_RULE_X + 1 + 64..=mx1 {
            let a = ((y * c.w + x) * 4) as usize;
            let b = ((y * c.w + x - 64) * 4) as usize;
            assert_eq!(panned[a..a + 4], still[b..b + 4], "({x}, {y})");
        }
    }
    // The column, the bands and the crawl's rows do not move.
    for y in 0..c.h {
        for x in (0..=COLUMN_RULE_X).chain(c.w - 8..c.w) {
            let a = ((y * c.w + x) * 4) as usize;
            assert_eq!(panned[a..a + 4], still[a..a + 4], "({x}, {y})");
        }
    }
}
