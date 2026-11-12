//! The gem map's canvas and its outdoor layout (Phase B design 10.2.4;
//! build plan M3-10, D-M3.3).
//!
//! **The canvas** is the foundation's logical canvas for the window:
//! 320 x 200 at 4:3, 512 x 240 at 16:9, 688 x 240 at 21:9 (and the
//! 1080-line 480 x 216 or 688 x 216). The foundation's own sizing
//! (`Canvas::for_window`, in the Bevy frontend) has no canvas below
//! 384 wide, where the classic frame shows; the gem map then uses the
//! classic 320 x 200. Below a 320 x 200 window there is no gem map and the
//! 1.0.1 view shows (10.7). [`gem_canvas`] takes the foundation's answer as
//! plain numbers, so this crate needs no frontend type (D-M3.3).
//!
//! **Outdoors** every cell is 4 px ([`OUTDOOR_CELL_PX`]), so the canvas
//! holds `W / 4` by `H / 4` cells. The party cell's centre is the canvas
//! centre exactly: its top-left is `(W / 2 - 2, H / 2 - 2)`. The cells
//! around it are laid out from there, so the outermost columns and rows
//! are cut by the canvas edge (2 px of them show at the even sizes).
//!
//! **In towns** ([`TownLayout`], 10.2.5, M3-12) the whole 32 x 32 floor is
//! centred at the largest whole cell size that fits: 7 px at 240 lines,
//! 6 px at 200 or 216. The land beside it and the strips above and below
//! fill the rest of the canvas at the same size.

use super::seen::GemWindow;

/// The gem map's canvas when the foundation has none (D-M3.3).
pub const GEM_CLASSIC_CANVAS: GemCanvas = GemCanvas { w: 320, h: 200 };

/// Pixels per cell side outdoors (10.2.4, D10.1).
pub const OUTDOOR_CELL_PX: u32 = 4;

/// The logical canvas the gem map fills, edge to edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GemCanvas {
    pub w: u32,
    pub h: u32,
}

impl GemCanvas {
    /// The canvas's pixel count.
    pub fn pixels(self) -> usize {
        self.w as usize * self.h as usize
    }
}

/// The gem map's canvas (D-M3.3): `foundation`, the foundation's canvas
/// for this window as `(w, h)`, when it has one; else the classic
/// 320 x 200 when the window (`(w, h)` in physical pixels) holds it; else
/// `None`, and the 1.0.1 view shows.
pub fn gem_canvas(foundation: Option<(u32, u32)>, window: (u32, u32)) -> Option<GemCanvas> {
    if window.0 < GEM_CLASSIC_CANVAS.w || window.1 < GEM_CLASSIC_CANVAS.h {
        return None;
    }
    Some(match foundation {
        Some((w, h)) if w > 0 && h > 0 => GemCanvas { w, h },
        _ => GEM_CLASSIC_CANVAS,
    })
}

/// Where the outdoor cells fall on a canvas (10.2.4). Offsets `dx`, `dy`
/// are cells east and south of the party (negative: west, north).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OutdoorLayout {
    pub canvas: GemCanvas,
    /// Pixels per cell side, [`OUTDOOR_CELL_PX`].
    pub cell: u32,
    /// The canvas position of the party cell's top-left corner.
    pub party_px: (i32, i32),
    /// The first and last cell offsets the canvas touches, inclusive.
    pub dx: (i32, i32),
    pub dy: (i32, i32),
}

impl OutdoorLayout {
    /// The outdoor layout of `canvas`.
    pub fn new(canvas: GemCanvas) -> Self {
        let cell = OUTDOOR_CELL_PX as i32;
        let party_px = (
            canvas.w as i32 / 2 - cell / 2,
            canvas.h as i32 / 2 - cell / 2,
        );
        // The first offset whose cell still reaches pixel 0, and the last
        // whose cell starts before the far edge.
        let span = |party: i32, side: i32| {
            (
                -(party + cell - 1).div_euclid(cell),
                (side - 1 - party).div_euclid(cell),
            )
        };
        Self {
            canvas,
            cell: OUTDOOR_CELL_PX,
            party_px,
            dx: span(party_px.0, canvas.w as i32),
            dy: span(party_px.1, canvas.h as i32),
        }
    }

    /// The nominal cell count, `W / 4` by `H / 4` (10.2.4).
    pub fn cells(&self) -> (u32, u32) {
        (self.canvas.w / self.cell, self.canvas.h / self.cell)
    }

    /// The columns and rows the canvas touches, partial ones included.
    pub fn drawn(&self) -> (u32, u32) {
        (
            (self.dx.1 - self.dx.0 + 1) as u32,
            (self.dy.1 - self.dy.0 + 1) as u32,
        )
    }

    /// The canvas rectangle of the cell at offset `(dx, dy)`, as
    /// `(x, y, w, h)` before clipping.
    pub fn cell_rect(&self, dx: i32, dy: i32) -> (i32, i32, u32, u32) {
        let cell = self.cell as i32;
        (
            self.party_px.0 + dx * cell,
            self.party_px.1 + dy * cell,
            self.cell,
            self.cell,
        )
    }

    /// The party cell's centre in canvas pixels, doubled so a half pixel
    /// stays exact; equal to `(W, H)` when it is the canvas centre.
    pub fn party_centre_x2(&self) -> (i32, i32) {
        let cell = self.cell as i32;
        (2 * self.party_px.0 + cell, 2 * self.party_px.1 + cell)
    }

    /// The window the record's seen-cell observer takes for this layout
    /// (10.3.3): every cell the canvas touches.
    pub fn gem_window(&self) -> GemWindow {
        GemWindow {
            half_cols: self.dx.0.unsigned_abs().max(self.dx.1.unsigned_abs()) as u16,
            half_rows: self.dy.0.unsigned_abs().max(self.dy.1.unsigned_abs()) as u16,
        }
    }
}

/// Cells on a side of a town floor (`town-mode.md §3`).
pub const TOWN_FLOOR_CELLS: i32 = 32;

/// The floor row the gazetteer entrance row sits beside (10.2.5, the
/// approved mockup): floor row `r` is beside world row
/// `entrance_y - 16 + r`, so the canvas's 36 rows at 240 lines are the side
/// window's 36 rows.
pub const TOWN_ENTRANCE_FLOOR_ROW: i32 = 16;

/// Where a town's cells fall on a canvas (10.2.5). Cells are addressed by
/// floor column and row: `0..32` is the floor, negative columns are the
/// land west of the entrance, columns from 32 the land east of it, and the
/// rows above 0 and from 32 on are the strips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TownLayout {
    pub canvas: GemCanvas,
    /// Pixels per cell side: the largest whole size at which the 32 x 32
    /// floor fits the canvas (7 at 240 lines, 6 at 200 or 216).
    pub cell: u32,
    /// The canvas position of floor cell (0, 0)'s top-left: the floor is
    /// centred.
    pub floor_px: (i32, i32),
    /// The first and last floor columns and rows the canvas touches,
    /// inclusive, partial ones included.
    pub cols: (i32, i32),
    pub rows: (i32, i32),
}

impl TownLayout {
    /// The town layout of `canvas`, or `None` when the floor cannot fit at
    /// one pixel a cell.
    pub fn new(canvas: GemCanvas) -> Option<Self> {
        let floor = TOWN_FLOOR_CELLS as u32;
        let cell = (canvas.w / floor).min(canvas.h / floor).min(16);
        if cell == 0 {
            return None;
        }
        let side = floor * cell;
        let floor_px = (
            ((canvas.w - side) / 2) as i32,
            ((canvas.h - side) / 2) as i32,
        );
        let c = cell as i32;
        let span = |origin: i32, extent: i32| {
            (
                -(origin + c - 1).div_euclid(c),
                (extent - 1 - origin).div_euclid(c),
            )
        };
        Some(Self {
            canvas,
            cell,
            floor_px,
            cols: span(floor_px.0, canvas.w as i32),
            rows: span(floor_px.1, canvas.h as i32),
        })
    }

    /// The canvas rectangle of floor cell `(fx, fy)`, as `(x, y, w, h)`
    /// before clipping.
    pub fn cell_rect(&self, fx: i32, fy: i32) -> (i32, i32, u32, u32) {
        let c = self.cell as i32;
        (
            self.floor_px.0 + fx * c,
            self.floor_px.1 + fy * c,
            self.cell,
            self.cell,
        )
    }

    /// The columns the canvas touches west and east of the floor, partial
    /// ones included (21 and 21 at 512 x 240, 11 and 11 at 320 x 200).
    pub fn drawn_sides(&self) -> (u32, u32) {
        (
            (-self.cols.0).max(0) as u32,
            (self.cols.1 - (TOWN_FLOOR_CELLS - 1)).max(0) as u32,
        )
    }

    /// The side widths in cells as the approved mockup reports them: the
    /// west side with its partial column, the east side as the whole cells
    /// left of `W / cell` (21 and 20 at 512 x 240, 11 and 10 at 320 x 200).
    pub fn side_cells(&self) -> (u32, u32) {
        let west = self.drawn_sides().0;
        let whole = self.canvas.w / self.cell;
        (west, whole.saturating_sub(TOWN_FLOOR_CELLS as u32 + west))
    }

    /// The offset from the entrance column of floor column `fx`'s land:
    /// `fx` on the west side (-1 touches the floor), `fx - 31` on the east
    /// side (32 is one east of the entrance), `None` on the floor's own
    /// columns, which cut the land open at the entrance column.
    pub fn entrance_dx(fx: i32) -> Option<i32> {
        if fx < 0 {
            Some(fx)
        } else if fx >= TOWN_FLOOR_CELLS {
            Some(fx - (TOWN_FLOOR_CELLS - 1))
        } else {
            None
        }
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
