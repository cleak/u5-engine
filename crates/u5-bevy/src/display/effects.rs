//! The dither effects (Phase B design 2.2.6, 2.2.7, 2.3.10; build plan
//! M1-15): the docked column's open and close effect, compose step 9.
//!
//! With `display.dither` on, Ctrl+P does not switch the column in one
//! frame. The effect is nine frames: the start, seven steps of 50 ms on the
//! shell's frame clock, and the end. Each step blends three images the
//! compositor draws from the same faithful frame: the collapsed canvas, the
//! open canvas and the neutral canvas (the collapsed one with no tabs and no
//! moving ribbon members, [`super::grow::compose::BandMode::Neutral`]).
//!
//! - **The sweep.** Over the column's box (x 7..152, from the top rule to
//!   the bottom rule) each pixel shows the open image from the first step
//!   whose time its [`sweep_t`] is below: a rise from the crawl to the
//!   roster with a 4x4 ordered-dither edge.
//! - **The text.** The crawl's glyphs and their outlines ([`TextMask`]) are
//!   kept out of the sweep. They show the collapsed image's colours until
//!   the first step whose time is past every text pixel ([`swap_step`]),
//!   and the open image's from then on: one step, and the text never moves.
//! - **The bands** (the top and bottom bands outside the box: the tabs and
//!   the 21:9 ribbon's third column). Steps 1-3 take a third of the old
//!   tabs away each, to the neutral band; step 3 shows no tab; steps 4-6
//!   add a third of the new ones each. One mask, [`tab_t`], depends only on
//!   the row within the band, so the sky and wind tabs change on the same
//!   pixels of the same steps and always share a centre.
//! - **The rest of the canvas** shows the open image from step 4: the
//!   `centre_party` pan happens under the sweep. Without it the map is the
//!   same in both images.
//!
//! Closing runs the same steps backwards. Pressing Ctrl+P again during the
//! effect reverses it from the step it is on. The effect is drawn only: no
//! key is held, nothing feeds the game, and the step clock is the shell's
//! own frame time. With `display.dither` off the column switches in one
//! frame, exactly as before this package.
//!
//! The numbers are the approved mockup's (`make_c04f.py`), evaluated in
//! double precision as its script evaluates them, so every pixel lands on
//! the same step.

use std::time::Duration;

use bevy::math::UVec2;

use super::grow::column::{COLUMN_X0, ColumnView};
use super::grow::crawl_paint::{CrawlPaint, paint_crawl_painted};
use super::grow::{BOTTOM_BAND, Canvas, TOP_BAND};
use crate::enh::present::MapRegion;

/// The 4x4 ordered-dither matrix.
pub const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// The sweep's divisions: step `k` of 1..=7 shows the open image where
/// `sweep_t < k / 8`; 0 is the collapsed image and 8 the open one.
pub const SWEEP_STEPS: u8 = 8;

/// One step of the effect on the shell's frame clock.
pub const STEP: Duration = Duration::from_millis(50);

/// A step falls due this much early, so that a frame clock of whole
/// sixtieths (16.666 ms) shows each step for three frames.
pub const STEP_SLACK: Duration = Duration::from_micros(500);

/// The step from which the rest of the canvas (and the `centre_party` pan)
/// shows the open image, and from which the new tabs dither in.
pub const PAN_STEP: u8 = 4;

/// The last step of the tabs' dither out (all old tabs gone).
pub const TABS_OUT_LAST: u8 = 3;

/// The last step of the tabs' dither in.
pub const TABS_IN_LAST: u8 = 6;

/// The first column the sweep covers: the screen's left rule.
pub const SWEEP_X0: u32 = COLUMN_X0;

/// One past the last column the sweep covers (x 7..152).
pub const SWEEP_X1: u32 = 153;

/// The sweep's rows on `canvas`: the top rule to the bottom rule,
/// `(top, bottom)` with `bottom` exclusive (15, 233 on 240 rows).
pub fn sweep_rows(canvas: &Canvas) -> (u32, u32) {
    (TOP_BAND - 1, canvas.h - BOTTOM_BAND + 1)
}

fn bayer(x: u32, y: u32) -> f64 {
    f64::from(BAYER4[(y % 4) as usize][(x % 4) as usize])
}

/// Sweep arrival time in 0..1 for a column pixel (2.3.10): `0.72 *
/// (bottom - 1 - y) / (bottom - top) + 0.28 * BAYER4[y % 4][x % 4] / 16`,
/// with `bottom` exclusive, in the mockup script's order of operations.
pub fn sweep_t(x: u32, y: u32, top: u32, bottom: u32) -> f64 {
    let span = f64::from(bottom - top);
    0.72 * f64::from(bottom - 1 - y) / span + 0.28 * bayer(x, y) / 16.0
}

/// Tab dither order (2.3.10): `0.6 * BAYER4[y % 4][x % 4] / 16 + 0.4 *
/// (7 - y % 8) / 8`. It depends only on `y % 8` and the Bayer cell, so the
/// top and bottom tabs get the same mask.
pub fn tab_t(x: u32, y: u32) -> f64 {
    0.6 * bayer(x, y) / 16.0 + 0.4 * f64::from(7 - y % 8) / 8.0
}

/// The first step (1..=7) whose time `t` is below, or 8 when none is.
pub fn step_of(t: f64) -> u8 {
    (1..SWEEP_STEPS)
        .find(|k| t < f64::from(*k) / f64::from(SWEEP_STEPS))
        .unwrap_or(SWEEP_STEPS)
}

/// The step from which a column pixel shows the open image.
pub fn sweep_step(x: u32, y: u32, top: u32, bottom: u32) -> u8 {
    step_of(sweep_t(x, y, top, bottom))
}

/// The share of the old tabs gone at steps 1-3, and of the new tabs in at
/// steps 4-6: a third, two thirds, all.
fn tab_share(k: u8) -> f64 {
    match k {
        1 | 4 => 1.0 / 3.0,
        2 | 5 => 2.0 / 3.0,
        _ => 1.0,
    }
}

/// Which image a band pixel shows at step `k` (1..=7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandPick {
    Closed,
    Neutral,
    Opened,
}

/// The band pixel `(x, y)` at step `k`: steps 1-3 keep the old (collapsed)
/// pixel until its [`tab_t`] falls under the share gone, showing the neutral
/// band after; steps 4-6 show the new (open) pixel once its `tab_t` falls
/// under the share in; step 7 shows the open image.
pub fn band_pick(x: u32, y: u32, k: u8) -> BandPick {
    let t = tab_t(x, y);
    match k {
        1..=TABS_OUT_LAST if t < tab_share(k) => BandPick::Neutral,
        1..=TABS_OUT_LAST => BandPick::Closed,
        4..=TABS_IN_LAST if t < tab_share(k) => BandPick::Opened,
        4..=TABS_IN_LAST => BandPick::Neutral,
        _ => BandPick::Opened,
    }
}

/// Where the crawl's text is on the collapsed image: its glyphs and their
/// outlines (2.3.10), a canvas-sized mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextMask {
    pub w: u32,
    pub h: u32,
    bits: Vec<bool>,
}

impl TextMask {
    pub fn empty(canvas: &Canvas) -> Self {
        Self {
            w: canvas.w,
            h: canvas.h,
            bits: vec![false; (canvas.w * canvas.h) as usize],
        }
    }

    /// The crawl `paint` draws on the collapsed `canvas`: every glyph pixel
    /// (the cursor's included, the end cap's not) and its eight neighbours
    /// clipped to the map window, as the outline is.
    pub fn of_crawl(canvas: &Canvas, paint: CrawlPaint<'_>) -> Self {
        let mut scratch = vec![0; (canvas.w * canvas.h * 4) as usize];
        let painted = paint_crawl_painted(&mut scratch, canvas, paint);
        let mut mask = Self::empty(canvas);
        let (cx0, cy0, cx1, cy1) = canvas.map_window();
        for ink in &painted.ink {
            mask.set(ink.x, ink.y);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (x, y) = (i64::from(ink.x) + dx, i64::from(ink.y) + dy);
                    if x >= i64::from(cx0)
                        && x <= i64::from(cx1)
                        && y >= i64::from(cy0)
                        && y <= i64::from(cy1)
                    {
                        mask.set(x as u32, y as u32);
                    }
                }
            }
        }
        mask
    }

    /// Mark `(x, y)` as text.
    pub fn set(&mut self, x: u32, y: u32) {
        if x < self.w && y < self.h {
            self.bits[(y * self.w + x) as usize] = true;
        }
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x < self.w && y < self.h && self.bits[(y * self.w + x) as usize]
    }

    /// The rows holding text, `(first, last + 1)`, or `None` when empty.
    pub fn rows(&self) -> Option<(u32, u32)> {
        let w = self.w as usize;
        let mut rows = self
            .bits
            .chunks(w)
            .enumerate()
            .filter(|(_, row)| row.iter().any(|bit| *bit))
            .map(|(y, _)| y as u32);
        let first = rows.next()?;
        let last = rows.next_back().unwrap_or(first);
        Some((first, last + 1))
    }
}

/// The step on which the text takes the open image's colours: the first
/// step whose time is past every pixel of the text's rows (every Bayer
/// column of them, x 8..11), or 8 when there is no such step. 4 for the
/// mockup's six rows at 16:9.
pub fn swap_step(mask: &TextMask, canvas: &Canvas) -> u8 {
    let Some((y0, y1)) = mask.rows() else {
        return SWEEP_STEPS;
    };
    let (top, bottom) = sweep_rows(canvas);
    let latest = (y0..y1)
        .flat_map(|y| (8..12).map(move |x| (x, y)))
        .filter(|(_, y)| (top..bottom).contains(y))
        .map(|(x, y)| sweep_t(x, y, top, bottom))
        .fold(f64::NEG_INFINITY, f64::max);
    step_of(latest)
}

/// The three images one effect step blends, all `canvas`-sized RGBA.
#[derive(Clone, Copy)]
pub struct EffectImages<'a> {
    pub closed: &'a [u8],
    pub opened: &'a [u8],
    pub neutral: &'a [u8],
}

/// Which of the three images a canvas pixel shows at one step of the
/// effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Closed,
    Opened,
    Neutral,
}

/// The per-pixel choice of step `k` (1..=7): shared by the picture
/// ([`blend`]) and its map region ([`blend_regions`]), so the two always
/// agree.
struct Picker<'a> {
    k: u8,
    top: u32,
    bottom: u32,
    h: u32,
    text_open: bool,
    text: &'a TextMask,
}

impl<'a> Picker<'a> {
    fn new(canvas: &Canvas, k: u8, text: &'a TextMask) -> Self {
        let (top, bottom) = sweep_rows(canvas);
        Self {
            k,
            top,
            bottom,
            h: canvas.h,
            text_open: k >= swap_step(text, canvas),
            text,
        }
    }

    fn at(&self, x: u32, y: u32) -> Pick {
        let open = |open: bool| if open { Pick::Opened } else { Pick::Closed };
        let in_band = y < TOP_BAND || y >= self.h - BOTTOM_BAND;
        let in_sweep = (SWEEP_X0..SWEEP_X1).contains(&x) && (self.top..self.bottom).contains(&y);
        if in_sweep {
            if self.text.contains(x, y) {
                open(self.text_open)
            } else {
                open(sweep_step(x, y, self.top, self.bottom) <= self.k)
            }
        } else if in_band {
            match band_pick(x, y, self.k) {
                BandPick::Closed => Pick::Closed,
                BandPick::Opened => Pick::Opened,
                BandPick::Neutral => Pick::Neutral,
            }
        } else if self.text.contains(x, y) {
            open(self.text_open)
        } else {
            open(self.k >= PAN_STEP)
        }
    }
}

/// Compose step 9 (2.3.7, 2.3.10): step `k` (1..=7) of the opening
/// sequence. Closing shows the same steps from 7 down to 1.
pub fn blend(images: EffectImages<'_>, canvas: &Canvas, k: u8, text: &TextMask) -> Vec<u8> {
    let (w, h) = (canvas.w, canvas.h);
    let picker = Picker::new(canvas, k, text);
    let mut out = images.closed.to_vec();
    for y in 0..h {
        for x in 0..w {
            let source = match picker.at(x, y) {
                Pick::Closed => continue,
                Pick::Opened => images.opened,
                Pick::Neutral => images.neutral,
            };
            let at = ((y * w + x) * 4) as usize;
            out[at..at + 4].copy_from_slice(&source[at..at + 4]);
        }
    }
    out
}

/// The map region of [`blend`]'s picture (build plan D-M2.5): each pixel
/// is map where the image it shows has map there. `regions` are the
/// closed, opened and neutral images' regions, in that order.
pub fn blend_regions(
    regions: [&MapRegion; 3],
    canvas: &Canvas,
    k: u8,
    text: &TextMask,
) -> MapRegion {
    let picker = Picker::new(canvas, k, text);
    let mut region = MapRegion::rect(UVec2::new(canvas.w, canvas.h), canvas.map_window());
    let (x0, y0, x1, y1) = canvas.map_window();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let source = match picker.at(x, y) {
                Pick::Closed => regions[0],
                Pick::Opened => regions[1],
                Pick::Neutral => regions[2],
            };
            if !source.contains(x, y) {
                region.exclude(x, y);
            }
        }
    }
    region
}

/// Ctrl+P: flip the player's column state; with `dither` on, start the
/// effect toward the new state, or reverse a running one from its step.
pub fn toggle(view: &mut ColumnView, dither: bool) {
    view.open = !view.open;
    view.step = if dither {
        Some(
            view.step
                .unwrap_or(if view.open { 1 } else { SWEEP_STEPS - 1 }),
        )
    } else {
        None
    };
}

/// One step of a running effect: toward 8 when opening, toward 0 when
/// closing; at either end the effect is over.
pub fn advance(view: &mut ColumnView) {
    let Some(k) = view.step else {
        return;
    };
    let next = if view.open {
        k + 1
    } else {
        k.saturating_sub(1)
    };
    view.step = (1..SWEEP_STEPS).contains(&next).then_some(next);
}

/// The effect's clock: `dt` of shell frame time passed. Steps whenever 50
/// ms have built up (carrying the rest), stops the effect when `dither` went
/// off, and holds no time while no effect runs.
pub fn tick(view: &mut ColumnView, elapsed: &mut Duration, dt: Duration, dither: bool) {
    if view.step.is_none() || !dither {
        view.step = None;
        *elapsed = Duration::ZERO;
        return;
    }
    *elapsed += dt;
    while view.step.is_some() && *elapsed + STEP_SLACK >= STEP {
        *elapsed = elapsed.saturating_sub(STEP);
        advance(view);
    }
    if view.step.is_none() {
        *elapsed = Duration::ZERO;
    }
}

#[cfg(test)]
#[path = "effects_tests.rs"]
mod tests;
