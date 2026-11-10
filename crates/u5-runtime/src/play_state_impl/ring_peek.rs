//! The ring peek: a read-only look at the top-down cells around the
//! faithful 11x11 window, for the grow-left layout (Phase B design 2.3.6).
//!
//! Declared as a child module of `chunk_09`, so the peek can use that
//! chunk's private top-down helpers with no visibility change.
//!
//! This file holds the peek's types, the fixed-screen predicate
//! [`PlayState::top_down_world_view_active`] (design 2.2.8) and the
//! [`PlayState::peek_top_down_ring`] entry point. The sight flood, the
//! light test and the objects arrive with the later ring-peek packages;
//! until then every ring cell is [`PeekShade::Hidden`], which is the
//! conservative side of the no-reveal rule (2.2.9): the ring never shows
//! more than the faithful core does. Nothing calls the peek yet.

use crate::{PlayState, VISIBILITY_HIDDEN};

/// How far daylight reaches in the wider ring (2.2.11, D2.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeekLightPolicy {
    /// Daylight reaches the whole canvas.
    #[default]
    Scaled,
    /// Daylight reaches only the original distance.
    Faithful,
}

/// Cells of the faithful core beyond the party on each side: the 11x11
/// top-down viewport is the party's cell plus five on every side.
pub const PEEK_CORE_REACH: u8 = 5;

/// The most cells the peek looks beyond the party on any one side
/// (design 2.3.6).
pub const PEEK_MAX_REACH: u8 = 40;

/// The cells the map window shows, measured from the party's cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeekWindow {
    /// Cells beyond the party on each side, partial cells
    /// included. 16:9 collapsed: 15, 15, 7, 7.
    pub west: u8,
    pub east: u8,
    pub north: u8,
    pub south: u8,
}

impl PeekWindow {
    /// A window that holds the whole core and reaches no further than
    /// [`PEEK_MAX_REACH`] on any side. The peek refuses any other.
    pub fn is_valid(self) -> bool {
        [self.west, self.east, self.north, self.south]
            .into_iter()
            .all(|reach| (PEEK_CORE_REACH..=PEEK_MAX_REACH).contains(&reach))
    }

    /// Cells across the window.
    pub fn columns(self) -> usize {
        usize::from(self.west) + 1 + usize::from(self.east)
    }

    /// Cells down the window.
    pub fn rows(self) -> usize {
        usize::from(self.north) + 1 + usize::from(self.south)
    }

    /// Cells in the whole window, core included.
    pub fn cell_count(self) -> usize {
        self.columns() * self.rows()
    }

    /// The offset from the party of the cell at row-major `index`, or
    /// `None` past the end of the window.
    pub fn offset_of(self, index: usize) -> Option<(isize, isize)> {
        if index >= self.cell_count() {
            return None;
        }
        let columns = self.columns();
        let dx = (index % columns) as isize - isize::from(self.west);
        let dy = (index / columns) as isize - isize::from(self.north);
        Some((dx, dy))
    }

    /// The row-major index of the cell at offset `(dx, dy)` from the
    /// party, or `None` outside the window.
    pub fn index_of(self, dx: isize, dy: isize) -> Option<usize> {
        let x = dx.checked_add(isize::from(self.west))?;
        let y = dy.checked_add(isize::from(self.north))?;
        let x = usize::try_from(x).ok().filter(|&x| x < self.columns())?;
        let y = usize::try_from(y).ok().filter(|&y| y < self.rows())?;
        Some(y * self.columns() + x)
    }

    /// Whether offset `(dx, dy)` is inside the faithful 11x11 core.
    pub fn is_core(dx: isize, dy: isize) -> bool {
        let reach = isize::from(PEEK_CORE_REACH);
        dx.abs() <= reach && dy.abs() <= reach
    }
}

/// How a ring cell is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeekShade {
    /// Out of sight: drawn black, as a hidden core cell is.
    Hidden,
    /// In sight: drawn from its tile.
    Lit,
}

/// One cell of the ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeekCell {
    /// Final composed tile id, as `PreparedTopDownCell::tile()`.
    pub tile: u8,
    pub shade: PeekShade,
}

impl PeekCell {
    /// The cell for anything out of sight, carrying the same hidden byte
    /// a hidden core cell composes to.
    pub const HIDDEN: PeekCell = PeekCell {
        tile: VISIBILITY_HIDDEN,
        shade: PeekShade::Hidden,
    };
}

/// The cells around the faithful core, for one window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopDownPeekRing {
    pub window: PeekWindow,
    /// Row-major over the whole window; `None` inside the 11x11
    /// core, which the shell copies from the faithful frame.
    pub cells: Vec<Option<PeekCell>>,
    /// True if any cell holds an animated static tile.
    pub animated: bool,
}

impl TopDownPeekRing {
    /// A ring with every cell out of sight.
    fn all_hidden(window: PeekWindow) -> Self {
        let cells = (0..window.cell_count())
            .map(|index| {
                let (dx, dy) = window
                    .offset_of(index)
                    .expect("the index is inside the window");
                (!PeekWindow::is_core(dx, dy)).then_some(PeekCell::HIDDEN)
            })
            .collect();
        Self {
            window,
            cells,
            animated: false,
        }
    }
}

impl PlayState {
    /// The plain top-down world or town view is what the frame shows.
    ///
    /// This is the fixed-screen test of design 2.2.8. It mirrors the
    /// branch order of the shell's base-frame render: the pending
    /// Blackthorn rescue, then the audience, then the shrine presentation,
    /// then [`PlayState::render_top_down_base_frame`], which hands combat
    /// and the dungeon to their own renderers. It is also false while a
    /// view overlay (the gem view and its kin), the endgame or a
    /// visibility sweep is present, since those keep the faithful frame.
    pub fn top_down_world_view_active(&self) -> bool {
        // The rescue's own render. During its live-grid stretch it draws
        // nothing and the ordinary map shows, so the switch to the fixed
        // screen comes on the first frame the rescue paints (2.2.8);
        // otherwise the held black field, a queued playback or the
        // running cinematic owns the viewport.
        let rescue_live_grid = self.blackthorn_rescue_active()
            && self.blackthorn_rescue_viewport
                == crate::blackthorn::BlackthornRescueViewport::LiveGrid;
        if !rescue_live_grid
            && (!self.pending_blackthorn_rescue_playbacks.is_empty()
                || self.blackthorn_rescue_active()
                || self.cutscene_black_viewport)
        {
            return false;
        }
        if self.blackthorn_audience_map.is_some() || self.shrine_presentation_map.is_some() {
            return false;
        }
        if self.active_view_overlay.is_some()
            || self.endgame.is_some()
            || self.visibility_sweep.is_some()
        {
            return false;
        }
        // `render_top_down_viewport`: combat first, then the dungeon when
        // there is no top-down render area.
        !self.combat_active && self.top_down_render_area().is_some()
    }

    /// Read-only. Takes `&self`; draws nothing from the RNG.
    ///
    /// Returns `None` whenever [`PlayState::top_down_world_view_active`]
    /// is false, and for a window that does not hold the whole core or
    /// reaches past [`PEEK_MAX_REACH`] on a side. Core cells are always
    /// `None`. Until the sight flood lands every ring cell is hidden, so
    /// the peek never shows more than the faithful core.
    pub fn peek_top_down_ring(
        &self,
        window: PeekWindow,
        light: PeekLightPolicy,
    ) -> Option<TopDownPeekRing> {
        // The light policy only changes the flood's in-range test, which
        // arrives with the sight flood.
        let _ = light;
        if !window.is_valid() || !self.top_down_world_view_active() {
            return None;
        }
        Some(TopDownPeekRing::all_hidden(window))
    }
}
