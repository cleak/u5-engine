//! The gem map's shown set: the cells it draws and the tile drawn at each
//! (Phase B design 10.2.3; build plan M3-10, D-M3.5).
//!
//! **Outdoors** the shown set is every cell of the canvas window around the
//! party ([`OutdoorLayout`]), wrapping at 256 on both axes, on whichever
//! plane the party is on (Britannia or the Underworld). Each cell's tile is
//! `gem_sample_tile(x, y, terrain_only = true)`: the live terrain after the
//! static-tile animation, never a tile taken from an active object, so no
//! person, monster or hidden thing reaches the map (D-M3.5). The live
//! moongate stays, because it is written into the world grid, not drawn as
//! an object. The party's own ship or mount shows through the party marker.
//!
//! The set is computed once when the view opens and kept in
//! [`GemViewState`] (10.3.4 step 1); the render only reads it. Every read
//! here is `&PlayState`.
//!
//! Towns (M3-12) and dungeons (M3-13) add their own arms; until then their
//! shown set is empty and nothing is drawn.

use u5_runtime::{Area, PlayState, ViewOverlayKind, ViewOverlayMode, WORLD_SIDE, WorldPlane};

use super::layout::{GemCanvas, OutdoorLayout};

/// The outdoor shown set (10.2.3, 10.2.4): one drawn tile per canvas cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutdoorShown {
    pub plane: WorldPlane,
    /// The party's world cell, the window's centre.
    pub party: (u8, u8),
    pub layout: OutdoorLayout,
    /// The drawn tiles, row-major over the layout's `dx` and `dy` spans.
    tiles: Vec<u8>,
}

impl OutdoorShown {
    /// The shown set for the party's current window, or `None` off the
    /// world map.
    pub fn sample(state: &PlayState, layout: OutdoorLayout) -> Option<Self> {
        let Area::World { plane } = state.area else {
            return None;
        };
        let (cols, rows) = layout.drawn();
        let mut tiles = Vec::with_capacity(cols as usize * rows as usize);
        let (px, py) = (state.player.x as isize, state.player.y as isize);
        for dy in layout.dy.0..=layout.dy.1 {
            for dx in layout.dx.0..=layout.dx.1 {
                tiles.push(state.gem_sample_tile(px + dx as isize, py + dy as isize, true));
            }
        }
        Some(Self {
            plane,
            party: (wrap(px), wrap(py)),
            layout,
            tiles,
        })
    }

    /// The drawn tile at offset `(dx, dy)` from the party, or `None`
    /// outside the window.
    pub fn tile(&self, dx: i32, dy: i32) -> Option<u8> {
        let (cols, _) = self.layout.drawn();
        if !(self.layout.dx.0..=self.layout.dx.1).contains(&dx)
            || !(self.layout.dy.0..=self.layout.dy.1).contains(&dy)
        {
            return None;
        }
        let col = (dx - self.layout.dx.0) as usize;
        let row = (dy - self.layout.dy.0) as usize;
        self.tiles.get(row * cols as usize + col).copied()
    }

    /// The world cell at offset `(dx, dy)`, wrapped.
    pub fn world_cell(&self, dx: i32, dy: i32) -> (u8, u8) {
        (
            wrap(self.party.0 as isize + dx as isize),
            wrap(self.party.1 as isize + dy as isize),
        )
    }

    /// Every shown cell as `(dx, dy, tile)`, row-major.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32, u8)> + '_ {
        let (cols, _) = self.layout.drawn();
        let (x0, y0) = (self.layout.dx.0, self.layout.dy.0);
        self.tiles.iter().enumerate().map(move |(index, &tile)| {
            let index = index as i32;
            (x0 + index % cols as i32, y0 + index / cols as i32, tile)
        })
    }

    /// Whether world cell `(x, y)` is in the shown set.
    pub fn contains(&self, x: u8, y: u8) -> bool {
        let offset = |cell: u8, party: u8, span: (i32, i32)| {
            let d = i32::from(cell.wrapping_sub(party));
            // Try the offset both ways round the 256-cell wrap.
            [d, d - WORLD_SIDE as i32]
                .into_iter()
                .any(|d| (span.0..=span.1).contains(&d))
        };
        offset(x, self.party.0, self.layout.dx) && offset(y, self.party.1, self.layout.dy)
    }
}

fn wrap(value: isize) -> u8 {
    value.rem_euclid(WORLD_SIDE as isize) as u8
}

/// The shown set of one open view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GemShown {
    /// Nothing to draw: no gem view, or a view this milestone does not
    /// draw yet (towns, dungeons).
    Empty,
    Outdoor(OutdoorShown),
}

/// One open gem view's state (10.3.2): the canvas, the cached shown set
/// and the inputs fixed when it opened. The shell builds it on open and on
/// resize; [`render_gem_map`](super::raster::render_gem_map) only reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GemViewState {
    pub canvas: GemCanvas,
    /// `display.map_brightness` when the view opened, for
    /// `gem_map.brightness_link` (10.2.10). 100 means off.
    pub display_map_brightness: u8,
    /// The dungeon level page, 0 for the current level (M3-15).
    pub page: i8,
    /// The sweep step to draw, when a sweep runs (M3-11).
    pub sweep_step: Option<u8>,
    pub shown: GemShown,
}

impl GemViewState {
    /// The state for the view now open on `state`, at `canvas`. The shown
    /// set is empty unless a gem view (not Peer, X-Ray or the sky) is up.
    pub fn open(state: &PlayState, canvas: GemCanvas, display_map_brightness: u8) -> Self {
        let shown = if gem_view_open(state) {
            match state.area {
                Area::World { .. } => OutdoorShown::sample(state, OutdoorLayout::new(canvas))
                    .map_or(GemShown::Empty, GemShown::Outdoor),
                Area::Town { .. } | Area::Dungeon { .. } => GemShown::Empty,
            }
        } else {
            GemShown::Empty
        };
        Self {
            canvas,
            display_map_brightness,
            page: 0,
            sweep_step: None,
            shown,
        }
    }
}

/// Whether the game's own view overlay is a gem view (10.3.2): the
/// overlay is up in [`ViewOverlayMode::GemView`] and is not the sky.
pub fn gem_view_open(state: &PlayState) -> bool {
    state.active_view_overlay.as_ref().is_some_and(|overlay| {
        overlay.mode == ViewOverlayMode::GemView && !matches!(overlay.kind, ViewOverlayKind::Sky(_))
    })
}

#[cfg(test)]
#[path = "shown_tests.rs"]
mod tests;
