//! Test-only recorder for the sprite composite on every suite state (Phase B
//! M2-06, design 9.3.8 and 9.5 P5).
//!
//! While recording is on, the headless play-frame compositor hands each
//! finished frame and its state here, as for the brightness recorder. For
//! each frame the recorder peeks the object cells with masks built from the
//! frame's atlas, runs the `[composite]` step's pixel rule on copies (halo
//! on and off) and checks:
//!
//! - a pixel changes only inside a peeked cell, where the tile's mask bit
//!   is set and the frame's pixel is exactly palette 0, and then it becomes
//!   the palette colour of the terrain under the object;
//! - with the halo, every changed pixel would also change without it (the
//!   halo mask is inside the background mask);
//! - no cell drawn with a listed tile (the keep-the-box and dark-body
//!   lists, the combat fields) or a near-empty tile is peeked;
//! - while a camp or potion flash inverts the view, nothing changes;
//! - with no map kind (no peek), nothing changes;
//! - on the top-down view, the grow-left canvas made from the composited
//!   frame carries the composited viewport as its core, byte for byte.
//!
//! The frame itself is only ever read. Nothing here is compiled outside
//! tests, and the recorder is off unless a test turns it on.

use std::cell::RefCell;

use u5_runtime::enhancements::{
    COMBAT_FIELDS, DARK_BODY, KEEP_THE_BOX, NEAR_EMPTY_PIXELS, PixelAspect, SpriteMasks, mask_bit,
};
use u5_runtime::{PeekLightPolicy, PlayState, TILE_ATLAS_SIDE, TileAtlas};

use crate::display::composite::{FRAME, VIEWPORT_ORIGIN, composite_cells};
use crate::display::grow::column::ColumnView;
use crate::display::grow::{
    Canvas, ComposeOptions, compose, compose_for_state_mapped, flash_active,
};

#[derive(Default, Debug)]
pub(crate) struct Tally {
    /// Frames handed to the recorder.
    pub frames: usize,
    /// Frames the peek answered (world, town or combat).
    pub peeked: usize,
    /// Cells composited, and pixels changed with and without the halo.
    pub cells: usize,
    pub changed_halo: usize,
    pub changed_flood: usize,
    /// Peeked frames with a flash.
    pub flashed: usize,
    /// Peeks with animated terrain under an object.
    pub animated: usize,
    /// Top-down frames whose grow-left core was checked.
    pub canvases: usize,
    pub mismatches: Vec<String>,
}

#[derive(Default)]
struct Recorder {
    tally: Tally,
    /// The masks of the last atlas seen, by its pixel buffer's address.
    masks: Option<(usize, SpriteMasks)>,
}

thread_local! {
    static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// Start recording on this thread, dropping anything recorded before.
pub(crate) fn start() {
    RECORDER.with(|cell| *cell.borrow_mut() = Some(Recorder::default()));
}

/// Stop recording and return the tally.
pub(crate) fn finish() -> Tally {
    RECORDER.with(|cell| {
        cell.borrow_mut()
            .take()
            .map(|recorder| recorder.tally)
            .unwrap_or_default()
    })
}

/// The pixels the rule allows to change on `frame`, as `(index, rgb)`:
/// inside a peeked cell, set in the mask and exactly palette 0.
fn allowed(
    frame: &[u8],
    palette: &[[u8; 3]],
    masks: &SpriteMasks,
    peek: &u5_runtime::ObjectCellPeek,
    halo: bool,
) -> Vec<(usize, [u8; 3])> {
    let mut out = Vec::new();
    for cell in &peek.cells {
        let mask = masks.background(usize::from(cell.drawn), halo);
        for y in 0..TILE_ATLAS_SIDE {
            for x in 0..TILE_ATLAS_SIDE {
                let px = VIEWPORT_ORIGIN + TILE_ATLAS_SIDE * usize::from(cell.col) + x;
                let py = VIEWPORT_ORIGIN + TILE_ATLAS_SIDE * usize::from(cell.row) + y;
                let at = (py * FRAME.x as usize + px) * 4;
                if mask_bit(mask, x, y) && frame[at..at + 3] == palette[0] {
                    let index = usize::from(cell.under_pixels[y * TILE_ATLAS_SIDE + x]);
                    out.push((at, palette[index % palette.len()]));
                }
            }
        }
    }
    out
}

/// The pixels `after` changed from `before` that the rule does not allow,
/// as a count.
fn wrong_pixels(before: &[u8], after: &[u8], allowed: &[(usize, [u8; 3])]) -> usize {
    let mut expected = before.to_vec();
    for (at, rgb) in allowed {
        expected[*at..*at + 3].copy_from_slice(rgb);
    }
    expected
        .chunks(4)
        .zip(after.chunks(4))
        .filter(|(a, b)| a != b)
        .count()
}

/// Check one finished frame of `state`.
pub(crate) fn note(state: &PlayState, atlas: &TileAtlas, frame: &[u8]) {
    RECORDER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(recorder) = borrow.as_mut() else {
            return;
        };
        let key = atlas.pixels.as_ptr() as usize;
        if recorder.masks.as_ref().is_none_or(|(seen, _)| *seen != key) {
            recorder.masks = Some((key, SpriteMasks::from_atlas(atlas)));
        }
        let masks = &recorder.masks.as_ref().expect("built above").1;
        let tally = &mut recorder.tally;
        tally.frames += 1;
        let at_frame = tally.frames;
        let mut found = Vec::new();
        let palette = crate::visual_palette(atlas.depth);
        let Some(peek) = state.peek_object_cells(atlas, masks) else {
            return;
        };
        if frame.len() != (FRAME.x * FRAME.y * 4) as usize {
            tally
                .mismatches
                .push(format!("frame {at_frame}: not a 320x200 frame"));
            return;
        }
        tally.peeked += 1;
        tally.cells += peek.cells.len();
        tally.animated += usize::from(peek.animated_under);
        for object in &peek.cells {
            let drawn = usize::from(object.drawn);
            if KEEP_THE_BOX.contains(&drawn)
                || DARK_BODY.contains(&drawn)
                || COMBAT_FIELDS.contains(&drawn)
                || masks.kept_pixels(drawn) < NEAR_EMPTY_PIXELS
            {
                found.push(format!("{object:?}: a boxed tile was peeked"));
            }
        }
        let flashed = flash_active(state);
        tally.flashed += usize::from(flashed);
        let mut composited = Vec::new();
        for halo in [true, false] {
            let mut copy = frame.to_vec();
            let changed = composite_cells(&mut copy, palette, masks, &peek, halo);
            let rule = allowed(frame, palette, masks, &peek, halo);
            let wrong = wrong_pixels(frame, &copy, &rule);
            if wrong > 0 {
                found.push(format!("halo {halo}: {wrong} pixels off the rule"));
            }
            if flashed && changed > 0 {
                found.push(format!(
                    "halo {halo}: {changed} pixels changed under a flash"
                ));
            }
            if halo {
                tally.changed_halo += changed;
                let flood = allowed(frame, palette, masks, &peek, false);
                if rule
                    .iter()
                    .any(|(at, _)| !flood.iter().any(|(f, _)| f == at))
                {
                    found.push("a halo pixel outside the background".to_string());
                }
                composited = copy;
            } else {
                tally.changed_flood += changed;
            }
        }
        if state.top_down_world_view_active() && !peek.cells.is_empty() {
            let canvas = Canvas::for_window(3440, 1440, PixelAspect::Dos).expect("the canvas");
            let options = ComposeOptions {
                light: PeekLightPolicy::Scaled,
                bands: None,
                crawl: None,
                column: ColumnView::default(),
                centre_party: true,
            };
            match compose_for_state_mapped(&composited, state, atlas, &canvas, options) {
                Some((rgba, _)) => {
                    tally.canvases += 1;
                    // The canvas pans for a shown column (page mode).
                    let view = ColumnView {
                        open: crate::display::grow::column::page_mode_active(state),
                        ..ColumnView::default()
                    };
                    let (cx, cy, side) = compose::core_rect(&canvas.for_column(view, true));
                    let w = canvas.w as usize;
                    let core_ok = (0..side).all(|row| {
                        let at = ((cy + row) * w + cx) * 4;
                        let from =
                            ((VIEWPORT_ORIGIN + row) * FRAME.x as usize + VIEWPORT_ORIGIN) * 4;
                        rgba[at..at + side * 4] == composited[from..from + side * 4]
                    });
                    if !core_ok {
                        found.push("the canvas core is not the composited viewport".to_string());
                    }
                }
                None => found.push("the top-down view gave no canvas".to_string()),
            }
        }
        tally.mismatches.extend(
            found
                .into_iter()
                .map(|line| format!("frame {at_frame}: {line}")),
        );
    });
}
