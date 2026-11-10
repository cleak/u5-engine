//! The grow-left layout's canvas (Phase B design 2.2.2, 2.2.11, 2.3.7;
//! build plan M1-08).
//!
//! The grow-left screen is drawn on a canvas of its own, sized from the
//! window by one rule (2.2.11):
//!
//! - **Rows.** 240 at the canvas pixel aspect (1.2 with
//!   `pixel_aspect = dos`), except on a window whose height is a multiple
//!   of 1080, which gets 216 rows at 1.25 so that its 4x5 (or 8x10) scale
//!   is exact. With `pixel_aspect = square` the aspect is 1.
//! - **Width.** `W = 16 * floor(H * aspect * window_w / window_h / 16)`,
//!   worked in whole numbers. The named layouts come out exact: 512x240 at
//!   2560x1440 (5x6), 688x240 at 3440x1440 (5x6) and 480x216 at 1920x1080
//!   (4x5).
//! - **The party** sits exactly mid-canvas horizontally (16:9: x 248..263)
//!   and mid-map-window vertically (y 116..131 on 240 rows, 104..119 on
//!   216). The map window is the canvas minus a 16-row top band, an 8-row
//!   bottom band and 8 columns of border each side.
//! - **Narrow canvases** (D2.21). The open column must never cover the
//!   faithful core (frame x 8..183, copied 176 pixels square round the
//!   party), and the column's right rule is at x 151. Under 480 wide the
//!   map is panned right by the fewest whole tiles that put the core at
//!   x 152 or beyond, in both states. Under 384 wide (narrower than 4:3)
//!   there is no grow-left canvas and the classic frame is shown, as it is
//!   on a window smaller than the frame at scale 1 (320x240).
//!
//! The canvas is plain geometry: nothing here reads the play state, and
//! nothing is drawn yet (the compositor is M1-09).

use u5_runtime::enhancements::PixelAspect;

use super::scale::Aspect;

/// Canvas rows on an ordinary window.
pub const CANVAS_ROWS: u32 = 240;
/// Canvas rows on a window whose height is a multiple of 1080.
pub const CANVAS_ROWS_1080: u32 = 216;
/// The 1080-line pixel aspect, 1.25 (4x5).
pub const ASPECT_1080: Aspect = Aspect {
    height: 5,
    width: 4,
};
/// Canvas widths are whole tiles.
pub const TILE: u32 = 16;
/// The top band: ribbon and sky tab.
pub const TOP_BAND: u32 = 16;
/// The bottom band: the wind tab's rule row.
pub const BOTTOM_BAND: u32 = 8;
/// Border columns each side (7 blue and the white rule).
pub const SIDE_BORDER: u32 = 8;
/// The faithful core's side (frame x 8..183, the 11x11 viewport).
pub const CORE_SIDE: u32 = 176;
/// The party tile's offset inside the core (the viewport's centre tile).
pub const CORE_PARTY_OFFSET: u32 = 80;
/// The first map column the open column leaves visible (its right rule is
/// at x 151).
pub const COLUMN_CLEAR_X: u32 = 152;
/// Canvases narrower than this pan the map (D2.21).
pub const NARROW_BELOW: u32 = 480;
/// Canvases narrower than this use the classic frame (D2.21).
pub const CLASSIC_BELOW: u32 = 384;
/// The smallest window with a canvas: the frame at scale 1 (2.2.11).
pub const MIN_WINDOW: (u32, u32) = (320, 240);

/// The grow-left canvas for one window size (2.3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Canvas {
    /// 512, 688, 480, ...: always whole tiles.
    pub w: u32,
    /// 240 or 216.
    pub h: u32,
    /// Left edge of the party tile, after `pan_tiles`.
    pub party_x: u32,
    /// Top edge of the party tile.
    pub party_y: u32,
    /// Whole tiles the map is panned right on a narrow canvas, in both
    /// states. The optional `centre_party` pan of the open column (2.2.7)
    /// is not part of the canvas; the column adds it.
    pub pan_tiles: u8,
    /// One canvas pixel's height over its width: 1.2, 1.25 on the
    /// 216-row canvas, or 1 with square pixels.
    pub aspect: Aspect,
}

impl Canvas {
    /// The canvas for a window of `window_w` by `window_h` physical
    /// pixels, or `None` where the classic frame is shown instead: a
    /// window below 320x240, or a canvas under 384 wide.
    pub fn for_window(window_w: u32, window_h: u32, pixel_aspect: PixelAspect) -> Option<Self> {
        if window_w < MIN_WINDOW.0 || window_h < MIN_WINDOW.1 {
            return None;
        }
        let line_1080 = window_h.is_multiple_of(1080);
        let h = if line_1080 {
            CANVAS_ROWS_1080
        } else {
            CANVAS_ROWS
        };
        let aspect = match (pixel_aspect, line_1080) {
            (PixelAspect::Square, _) => Aspect::SQUARE,
            (PixelAspect::Dos, true) => ASPECT_1080,
            (PixelAspect::Dos, false) => Aspect::DOS,
        };
        // W = 16 * floor(H * (aspect.height / aspect.width) * ww / wh / 16),
        // in whole numbers so the named layouts are exact.
        let numerator = u64::from(h) * u64::from(aspect.height) * u64::from(window_w);
        let denominator = u64::from(aspect.width) * u64::from(window_h) * u64::from(TILE);
        let tiles = numerator / denominator;
        let w = u32::try_from(tiles * u64::from(TILE)).ok()?;
        if w < CLASSIC_BELOW {
            return None;
        }
        let centred_x = (w - TILE) / 2;
        // The core starts CORE_PARTY_OFFSET left of the party tile; pan by
        // the fewest whole tiles that keep it clear of the open column.
        let clear_party_x = COLUMN_CLEAR_X + CORE_PARTY_OFFSET;
        let pan_tiles = clear_party_x.saturating_sub(centred_x).div_ceil(TILE);
        let map_h = h - TOP_BAND - BOTTOM_BAND;
        Some(Self {
            w,
            h,
            party_x: centred_x + pan_tiles * TILE,
            party_y: TOP_BAND + (map_h - TILE) / 2,
            pan_tiles: pan_tiles as u8,
            aspect,
        })
    }

    /// The black map window inside the white rules, as
    /// `(x0, y0, x1, y1)` inclusive.
    pub fn map_window(&self) -> (u32, u32, u32, u32) {
        (
            SIDE_BORDER,
            TOP_BAND,
            self.w - SIDE_BORDER - 1,
            self.h - BOTTOM_BAND - 1,
        )
    }

    /// Top-left of the faithful core (the 176x176 copy of frame x and y
    /// 8..183), centred on the party tile.
    pub fn core_origin(&self) -> (u32, u32) {
        (
            self.party_x - CORE_PARTY_OFFSET,
            self.party_y - CORE_PARTY_OFFSET,
        )
    }

    /// Whether the party tile is exactly mid-canvas horizontally and
    /// mid-map-window vertically (every canvas without a pan).
    pub fn party_centred(&self) -> bool {
        let (x0, y0, x1, y1) = self.map_window();
        2 * self.party_x + TILE == self.w
            && 2 * self.party_y + TILE == y0 + y1 + 1
            && 2 * self.party_x + TILE == x0 + x1 + 1
    }
}

#[cfg(test)]
mod tests;
