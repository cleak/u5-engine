//! Test-only recorder for the brightness step on every suite state (Phase B
//! M2-03, design 9.3.3 and 9.5 P1).
//!
//! While recording is on, the headless play-frame compositor hands each
//! finished frame and its state here, as for the grow-left recorder. For
//! each frame the recorder runs the `[brightness]` step's region and scale
//! on copies and checks:
//!
//! - scope `map`: with a map kind (world, town, combat, dungeon) exactly
//!   the viewport, pixels 8..=183, is scaled and every other pixel is the
//!   frame's own; with no map kind (the text panel, the shrine, the
//!   cinematics, a view overlay, the endgame) the copy is the frame;
//! - scope `frame`: every pixel is scaled;
//! - (D-M2.5) on the top-down view, the grow-left canvas at 21:9 with the
//!   column closed and open: the region is inside the map window, the
//!   open column is never in it, the dimmed core is the dimmed 320x200
//!   viewport byte for byte, and every pixel outside the region (the
//!   ribbon, tabs, rules and the column) keeps its exact colour.
//!
//! The frame itself is only ever read. Nothing here is compiled outside
//! tests, and the recorder is off unless a test turns it on.

use std::cell::RefCell;

use bevy::math::UVec2;
use u5_runtime::enhancements::{BrightnessLut, BrightnessScope, DisplayLight, PixelAspect};
use u5_runtime::{FixedCellFont, MapViewKind, PeekLightPolicy, PlayState, TileAtlas};

use crate::display::grow::column::{COLUMN_RULE_X, ColumnView};
use crate::display::grow::{Canvas, ComposeOptions, bands, compose, compose_for_state_mapped};
use crate::display::light::{BrightnessStep, FRAME, VIEWPORT, brightness_region};
use crate::enh::present::{MapRegion, PresentedFrame};

/// The percent checked: the suggested level.
const PERCENT: u8 = 70;

#[derive(Default, Debug)]
pub(crate) struct Tally {
    /// Frames handed to the recorder.
    pub frames: usize,
    /// Frames per kind: world, town, combat, dungeon, none.
    pub kinds: [usize; 5],
    /// Frames whose grow-left canvas was checked.
    pub canvases: usize,
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
    RECORDER.with(|cell| cell.borrow_mut().take().unwrap_or_default())
}

fn recording() -> bool {
    RECORDER.with(|cell| cell.borrow().is_some())
}

fn step(scope: BrightnessScope) -> BrightnessStep {
    BrightnessStep::of(&DisplayLight {
        brightness: PERCENT,
        scope,
        ..DisplayLight::default()
    })
    .expect("a step below 100")
}

/// The pixels of `after` that are not `before` scaled inside `region` and
/// `before` outside it, as a count.
fn wrong_pixels(
    before: &[u8],
    after: &[u8],
    size: UVec2,
    region: Option<&MapRegion>,
    lut: &BrightnessLut,
) -> usize {
    let mut wrong = 0;
    for y in 0..size.y {
        for x in 0..size.x {
            let at = ((y * size.x + x) * 4) as usize;
            let old = &before[at..at + 4];
            let expected = if region.is_some_and(|region| region.contains(x, y)) {
                [
                    lut.apply(old[0]),
                    lut.apply(old[1]),
                    lut.apply(old[2]),
                    old[3],
                ]
            } else {
                [old[0], old[1], old[2], old[3]]
            };
            if after[at..at + 4] != expected {
                wrong += 1;
            }
        }
    }
    wrong
}

fn kind_index(kind: Option<MapViewKind>) -> usize {
    match kind {
        Some(MapViewKind::World) => 0,
        Some(MapViewKind::Town) => 1,
        Some(MapViewKind::Combat) => 2,
        Some(MapViewKind::Dungeon) => 3,
        None => 4,
    }
}

/// The grow-left checks (D-M2.5) for one column state.
#[allow(clippy::too_many_arguments)]
fn check_canvas(
    state: &PlayState,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    frame: &[u8],
    dimmed_frame: &[u8],
    open: bool,
    found: &mut Vec<String>,
) -> bool {
    let canvas = Canvas::for_window(3440, 1440, PixelAspect::Dos).expect("the 21:9 canvas");
    let snapshot = bands::band_snapshot(state);
    let options = ComposeOptions {
        light: PeekLightPolicy::Scaled,
        bands: Some(compose::Bands {
            snapshot: &snapshot,
            font,
            hp_colours: true,
        }),
        crawl: None,
        column: ColumnView {
            open,
            ..ColumnView::default()
        },
        centre_party: true,
        composite: None,
    };
    let Some((rgba, region)) = compose_for_state_mapped(frame, state, atlas, &canvas, options)
    else {
        return false;
    };
    let context = if open { "open" } else { "closed" };
    let size = UVec2::new(canvas.w, canvas.h);
    let lut = BrightnessLut::for_percent(PERCENT);
    let mut copy = PresentedFrame {
        size,
        rgba: rgba.clone(),
        smooth: false,
        map_region: Some(region.clone()),
    };
    step(BrightnessScope::Map).dim(state.light_view_kind(), &mut copy);
    let wrong = wrong_pixels(&rgba, &copy.rgba, size, Some(&region), &lut);
    if wrong > 0 {
        found.push(format!(
            "{context}: {wrong} canvas pixels off the region's rule"
        ));
    }
    let (x0, y0, x1, y1) = canvas.map_window();
    let column_shown = open || crate::display::grow::column::page_mode_active(state);
    let mut stray = 0;
    for y in 0..size.y {
        for x in 0..size.x {
            let in_window = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
            let in_column = column_shown && x <= COLUMN_RULE_X;
            if region.contains(x, y) && (!in_window || in_column) {
                stray += 1;
            }
        }
    }
    if stray > 0 {
        found.push(format!("{context}: {stray} region pixels off the map"));
    }
    // The dimmed core is the dimmed viewport (on the canvas panned for the
    // shown column).
    let view = ColumnView {
        open: column_shown,
        ..ColumnView::default()
    };
    let (cx, cy, side) = compose::core_rect(&canvas.for_column(view, true));
    let w = canvas.w as usize;
    let core_ok = (0..side).all(|row| {
        let at = ((cy + row) * w + cx) * 4;
        let from = ((8 + row) * FRAME.x as usize + 8) * 4;
        copy.rgba[at..at + side * 4] == dimmed_frame[from..from + side * 4]
    });
    if !core_ok {
        found.push(format!(
            "{context}: the dimmed core is not the dimmed viewport"
        ));
    }
    true
}

/// Check one finished frame of `state`.
pub(crate) fn note(state: &PlayState, atlas: &TileAtlas, font: &FixedCellFont, frame: &[u8]) {
    if !recording() {
        return;
    }
    let mut found = Vec::new();
    let kind = state.light_view_kind();
    let lut = BrightnessLut::for_percent(PERCENT);
    let mut map = PresentedFrame {
        size: FRAME,
        rgba: frame.to_vec(),
        ..PresentedFrame::default()
    };
    step(BrightnessScope::Map).dim(kind, &mut map);
    let region = brightness_region(BrightnessScope::Map, kind, FRAME, None);
    if region.as_ref().map(|region| region.rects.as_slice()) != kind.map(|_| &[VIEWPORT][..]) {
        found.push(format!("{kind:?}: the map region is {region:?}"));
    }
    let wrong = wrong_pixels(frame, &map.rgba, FRAME, region.as_ref(), &lut);
    if wrong > 0 {
        found.push(format!(
            "{kind:?} map: {wrong} pixels off the region's rule"
        ));
    }
    let mut whole = PresentedFrame {
        size: FRAME,
        rgba: frame.to_vec(),
        ..PresentedFrame::default()
    };
    step(BrightnessScope::Frame).dim(kind, &mut whole);
    let all = MapRegion::rect(FRAME, (0, 0, FRAME.x - 1, FRAME.y - 1));
    let wrong = wrong_pixels(frame, &whole.rgba, FRAME, Some(&all), &lut);
    if wrong > 0 {
        found.push(format!("{kind:?} frame: {wrong} pixels not scaled"));
    }
    let mut canvases = false;
    if state.top_down_world_view_active() {
        for open in [false, true] {
            canvases |= check_canvas(state, atlas, font, frame, &map.rgba, open, &mut found);
        }
        if !canvases {
            found.push("the top-down view gave no canvas".to_string());
        }
    }
    RECORDER.with(|cell| {
        if let Some(tally) = cell.borrow_mut().as_mut() {
            tally.frames += 1;
            tally.kinds[kind_index(kind)] += 1;
            if canvases {
                tally.canvases += 1;
            }
            tally.mismatches.extend(
                found
                    .into_iter()
                    .map(|line| format!("frame {}: {line}", tally.frames)),
            );
        }
    });
}
