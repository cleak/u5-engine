//! Test-only recorder for the grow-left compositor's crop-back identity
//! (Phase B M1-09, design 2.3.7 and 2.6).
//!
//! The canvas core must be the faithful viewport byte for byte, on every
//! frame the shell composes. While recording is on, the headless play-frame
//! compositor hands each finished frame (after the flash and the dissolve
//! drain, exactly the bytes an upload site writes) and its state here; the
//! recorder composes the canvas for the named layouts and a panned narrow
//! one and checks:
//!
//! - a fixed screen (the peek's predicate false) gives no canvas, so the
//!   faithful frame passes through;
//! - otherwise the core equals frame x and y 8..183, so a flash inverts it
//!   once (the frame's own inversion) and never twice;
//! - the canvas's top-left, top-right and bottom-left corners carry the
//!   frame's own rounded-corner staircase (the 6x6 corner block), the one
//!   measured off the original frame;
//! - during a flash every ring pixel is the palette XOR of the unflashed
//!   ring pixel;
//! - (M1-10) the sky and wind tabs on the canvas are the frame's own tab
//!   pixels byte for byte, and the two share one centre;
//! - (M1-10, the no-reveal rule) the ribbon shows no more members than the
//!   party has, and every character it draws is in the faithful panel row
//!   of the same member, or in the counters or date row, of this frame;
//! - (M1-14) while a page is up the column is shown with the column closed
//!   (the tabs at the open centre), its panel is the frame's own right panel
//!   (frame x 192..311, y 8..80) byte for byte, and its message window is
//!   the frame's own message window byte for byte.
//!
//! Nothing here is compiled outside tests, and the recorder is off unless a
//! test turns it on.

use std::cell::RefCell;

use u5_runtime::{FixedCellFont, PeekLightPolicy, PlayState, TileAtlas};

use crate::VISUAL_PLAY_FRAME_WIDTH;
use crate::display::grow::bands::{self, BandSnapshot, ribbon_layout};
use crate::display::grow::column::{
    COUNTERS_CROP, COUNTERS_Y, CROP_X, ColumnView, FrameRect, MESSAGE_CROP, PAGE_PANEL, PANEL_DROP,
    ROSTER_CROP, ROSTER_Y, message_crop_origin, page_mode_active,
};
use crate::display::grow::{Canvas, ComposeOptions, compose, compose_for_state, flash_active};

/// The windows checked: the three named layouts, a 21:9 at 2560x1080 and
/// a panned 16:10.
pub(crate) const WINDOWS: [(u32, u32); 5] = [
    (2560, 1440),
    (3440, 1440),
    (1920, 1080),
    (2560, 1080),
    (1920, 1200),
];

#[derive(Default, Debug)]
pub(crate) struct Tally {
    /// Frames handed to the recorder.
    pub frames: usize,
    /// Frames that showed the top-down view and got a canvas.
    pub composed: usize,
    /// Frames passed through (fixed screens).
    pub passed: usize,
    /// Composed frames with the flash on.
    pub flashed: usize,
    /// Composed frames with a page up (M1-14).
    pub paged: usize,
    pub mismatches: Vec<String>,
}

thread_local! {
    static RECORDER: RefCell<Option<Tally>> = const { RefCell::new(None) };
}

/// Start recording on this thread, dropping anything recorded before.
pub(crate) fn start() {
    RECORDER.with(|cell| *cell.borrow_mut() = Some(Tally::default()));
}

/// Stop recording and return the tally.
pub(crate) fn finish() -> Tally {
    let tally = RECORDER.with(|cell| cell.borrow_mut().take().unwrap_or_default());
    eprintln!("grow-left compose: {} frames with a page up", tally.paged);
    tally
}

pub(crate) fn recording() -> bool {
    RECORDER.with(|cell| cell.borrow().is_some())
}

const FRAME_W: usize = VISUAL_PLAY_FRAME_WIDTH as usize;

fn pixel(rgba: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let at = (y * width + x) * 4;
    [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
}

fn options<'a>(
    snapshot: &'a BandSnapshot,
    font: &'a FixedCellFont,
    open: bool,
) -> ComposeOptions<'a> {
    ComposeOptions {
        light: PeekLightPolicy::Scaled,
        bands: Some(compose::Bands {
            snapshot,
            font,
            hp_colours: true,
        }),
        crawl: None,
        column: ColumnView {
            open,
            ..ColumnView::default()
        },
        centre_party: false,
    }
}

/// The ribbon draws nothing the faithful panel of this frame does not show.
fn check_ribbon(state: &PlayState, snapshot: &BandSnapshot, found: &mut Vec<String>) {
    let panel = state.displayed_stats_panel();
    let ribbon = &snapshot.ribbon;
    if ribbon.members.len() > state.party.len() {
        found.push(format!(
            "the ribbon shows {} members of a party of {}",
            ribbon.members.len(),
            state.party.len()
        ));
    }
    for (index, member) in ribbon.members.iter().enumerate() {
        let row = panel.rows.get(index).map(String::as_str).unwrap_or("");
        let name_ok = row.starts_with(&member.name);
        let hp_ok = row.contains(&member.hp);
        let status_ok = member.status == panel.statuses.get(index).copied().flatten();
        if !(name_ok && hp_ok && status_ok) {
            found.push(format!(
                "ribbon member {index} {member:?} is not panel row {row:?}"
            ));
        }
    }
    for (text, row) in [
        (&ribbon.counters, panel.counter_row.as_str()),
        (&ribbon.date, panel.date_row.as_str()),
    ] {
        if text.split(' ').any(|word| !row.contains(word)) {
            found.push(format!("ribbon text {text:?} is not panel row {row:?}"));
        }
    }
}

/// The tabs on `rgba` are the frame's tabs, at one shared centre.
fn check_tabs(
    rgba: &[u8],
    canvas: &Canvas,
    frame: &[u8],
    snapshot: &BandSnapshot,
    open: bool,
    context: &str,
    found: &mut Vec<String>,
) {
    let w = canvas.w as usize;
    let sky = snapshot.gaps.top_rect();
    let wind = snapshot.gaps.bottom_rect();
    let width = sky.map_or(112, |rect| rect.width() as u32);
    let centre = bands::tab_centre(canvas, open, width, &snapshot.ribbon);
    let mut centres = Vec::new();
    for (rect, y0) in [(sky, 8usize), (wind, canvas.h as usize - 8)] {
        let Some(rect) = rect else {
            continue;
        };
        let x0 = bands::tab_left(rect, centre) as usize;
        centres.push(2 * x0 + rect.width());
        let same = (0..8).all(|row| {
            let at = ((y0 + row) * w + x0) * 4;
            let from = ((rect.y0 + row) * FRAME_W + rect.x0) * 4;
            rgba[at..at + rect.width() * 4] == frame[from..from + rect.width() * 4]
        });
        if !same {
            found.push(format!("{context}: a tab is not the frame's own pixels"));
        }
    }
    if centres.windows(2).any(|pair| pair[0] != pair[1]) {
        found.push(format!(
            "{context}: the tabs do not share a centre: {centres:?}"
        ));
    }
    // Text never sits under the sky tab.
    if let Some(rect) = sky {
        let x0 = bands::tab_left(rect, centre);
        let x1 = x0 + rect.width() as u32;
        let layout = ribbon_layout(canvas, open);
        let member_w = layout.member_cells() as u32 * 8;
        let left = bands::counters_left(canvas, &snapshot.ribbon);
        let clear = (0..snapshot.ribbon.members.len())
            .map(|index| layout.place(index).0)
            .all(|x| x + member_w <= x0 || x >= x1)
            && left >= x1;
        if !clear {
            found.push(format!("{context}: text under the sky tab"));
        }
    }
}

/// (M1-13) With the column open: its roster and food/gold/date windows are
/// the frame's own windows byte for byte, the core is still the faithful
/// viewport (the column never covers it), and the tabs are the frame's own
/// pixels at the open centre.
fn check_column(
    frame: &[u8],
    state: &PlayState,
    atlas: &TileAtlas,
    canvas: &Canvas,
    options: ComposeOptions<'_>,
    context: &str,
    found: &mut Vec<String>,
) {
    let Some(rgba) = compose_for_state(frame, state, atlas, canvas, options) else {
        found.push(format!("{context}: no canvas"));
        return;
    };
    let w = canvas.w as usize;
    let crop_is_frame = |rect: FrameRect, y0: u32| {
        (0..rect.h as usize).all(|row| {
            let at = ((y0 as usize + row) * w + CROP_X as usize) * 4;
            let from = ((rect.y as usize + row) * FRAME_W + rect.x as usize) * 4;
            let width = rect.w as usize * 4;
            rgba[at..at + width] == frame[from..from + width]
        })
    };
    if !crop_is_frame(ROSTER_CROP, ROSTER_Y.0) {
        found.push(format!(
            "{context}: the roster is not the frame's own window"
        ));
    }
    if !crop_is_frame(COUNTERS_CROP, COUNTERS_Y.0) {
        found.push(format!(
            "{context}: food, gold and the date are not the frame's own window"
        ));
    }
    if page_mode_active(state) {
        if !crop_is_frame(PAGE_PANEL, PAGE_PANEL.y + PANEL_DROP) {
            found.push(format!(
                "{context}: the page's panel is not the frame's own panel"
            ));
        }
        if !crop_is_frame(MESSAGE_CROP, message_crop_origin(canvas).1) {
            found.push(format!(
                "{context}: the page's message window is not the frame's own window"
            ));
        }
    }
    let (cx, cy, side) = compose::core_rect(canvas);
    let core_ok = (0..side).all(|row| {
        let at = ((cy + row) * w + cx) * 4;
        let from = ((8 + row) * FRAME_W + 8) * 4;
        rgba[at..at + side * 4] == frame[from..from + side * 4]
    });
    if !core_ok {
        found.push(format!("{context}: the column covers the core"));
    }
    let snapshot = options.bands.expect("the bands").snapshot;
    check_tabs(&rgba, canvas, frame, snapshot, true, context, found);
}

/// Check one finished frame of `state`.
pub(crate) fn note(state: &PlayState, atlas: &TileAtlas, font: &FixedCellFont, frame: &[u8]) {
    if !recording() {
        return;
    }
    let mut found = Vec::new();
    let snapshot = bands::band_snapshot(state);
    if state.top_down_world_view_active() {
        check_ribbon(state, &snapshot, &mut found);
    }
    let mut tally_composed = false;
    let mut flashed = false;
    let mut pages_seen = false;
    for (ww, wh) in WINDOWS {
        let canvas = Canvas::for_window(ww, wh, u5_runtime::enhancements::PixelAspect::Dos)
            .expect("a canvas for every window checked");
        let composed = compose_for_state(
            frame,
            state,
            atlas,
            &canvas,
            options(&snapshot, font, false),
        );
        let Some(rgba) = composed else {
            if state.top_down_world_view_active() {
                found.push(format!("{ww}x{wh}: the top-down view gave no canvas"));
            }
            continue;
        };
        if !state.top_down_world_view_active() {
            found.push(format!("{ww}x{wh}: a fixed screen gave a canvas"));
        }
        tally_composed = true;
        let w = canvas.w as usize;
        let h = canvas.h as usize;
        assert_eq!(rgba.len(), w * h * 4);
        // Crop-back identity.
        let (cx, cy, side) = compose::core_rect(&canvas);
        let mut core_ok = true;
        for row in 0..side {
            let at = ((cy + row) * w + cx) * 4;
            let from = ((8 + row) * FRAME_W + 8) * 4;
            if rgba[at..at + side * 4] != frame[from..from + side * 4] {
                core_ok = false;
                break;
            }
        }
        if !core_ok {
            found.push(format!("{ww}x{wh}: the core is not the faithful viewport"));
        }
        // A page shows the column with the player's column closed.
        let page = page_mode_active(state);
        if page {
            pages_seen = true;
        }
        check_tabs(
            &rgba,
            &canvas,
            frame,
            &snapshot,
            page,
            &format!("{ww}x{wh}"),
            &mut found,
        );
        check_column(
            frame,
            state,
            atlas,
            &canvas,
            options(&snapshot, font, true),
            &format!("{ww}x{wh} open"),
            &mut found,
        );
        // The corners: the frame's 6x6 staircase blocks, top-left,
        // top-right and bottom-left of the frame's two blue frames (the
        // frame's bottom-left corner sits at row 191, above the bottom
        // text rows).
        let frame_bottom = 191;
        for (name, fx, fy, kx, ky) in [
            ("top-left", 0, 0, 0, 0),
            ("top-right", FRAME_W - 6, 0, w - 6, 0),
            ("bottom-left", 0, frame_bottom - 5, 0, h - 6),
        ] {
            let same = (0..6).all(|dy| {
                (0..6).all(|dx| {
                    pixel(frame, FRAME_W, fx + dx, fy + dy) == pixel(&rgba, w, kx + dx, ky + dy)
                })
            });
            if !same {
                found.push(format!(
                    "{ww}x{wh}: the {name} corner differs from the frame"
                ));
            }
        }
        // The flash: the ring is the unflashed ring, inverted.
        if flash_active(state) {
            flashed = true;
            let mut calm = state.clone();
            calm.presentation_camp_flash_inverted = false;
            calm.message_window_flash_hold = Default::default();
            if let Some(plain) = compose_for_state(
                frame,
                &calm,
                atlas,
                &canvas,
                options(&snapshot, font, false),
            ) {
                let mask = u5_runtime::potion_flash_playback(0)
                    .expect("the shared flash")
                    .palette_xor_mask;
                let (mx0, my0, mx1, my1) = canvas.map_window();
                let bad = (my0 as usize..=my1 as usize).any(|y| {
                    (mx0 as usize..=mx1 as usize).any(|x| {
                        if (cy..cy + side).contains(&y) && (cx..cx + side).contains(&x) {
                            return false;
                        }
                        let at = (y * w + x) * 4;
                        let plain = crate::ega_palette_index_from_rgba(&plain[at..at + 4]);
                        let lit = crate::ega_palette_index_from_rgba(&rgba[at..at + 4]);
                        plain ^ mask != lit
                    })
                });
                if bad {
                    found.push(format!("{ww}x{wh}: the ring is not inverted once"));
                }
            }
        }
    }
    RECORDER.with(|cell| {
        if let Some(tally) = cell.borrow_mut().as_mut() {
            tally.frames += 1;
            if tally_composed {
                tally.composed += 1;
            } else {
                tally.passed += 1;
            }
            if flashed {
                tally.flashed += 1;
            }
            if pages_seen {
                tally.paged += 1;
            }
            tally.mismatches.extend(found);
        }
    });
}
