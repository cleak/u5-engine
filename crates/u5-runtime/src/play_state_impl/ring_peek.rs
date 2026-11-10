//! The ring peek: a read-only look at the top-down cells around the
//! faithful 11x11 window, for the grow-left layout (Phase B design 2.3.6).
//!
//! Declared as a child module of `chunk_09`, so the peek can use that
//! chunk's private top-down helpers with no visibility change.
//!
//! This file holds the peek's types, the fixed-screen predicate
//! [`PlayState::top_down_world_view_active`] (design 2.2.8), the
//! [`PlayState::peek_top_down_ring`] entry point and its second-stage
//! sight flood with the light test (design 2.3.6, 2.2.9). The flood is
//! seeded from the faithful visibility buffer the render just filled, so
//! the core is never recomputed and the ring never shows a cell the party
//! could not see from where it stands. Objects arrive with the next
//! ring-peek package; until then a lit ring cell carries its plain
//! terrain. Nothing calls the peek yet.

use std::collections::VecDeque;

use crate::{
    FULL_DAYLIGHT, PlayState, TopDownRenderArea, VISIBILITY_CARVE_NEIGHBOR_ORDER,
    VISIBILITY_HIDDEN, visibility_grid_active_index, visibility_in_radius,
};

use super::surface_tile_propagates_visibility;

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
    /// `None`; the shell copies them from the faithful frame. A ring cell
    /// is [`PeekShade::Lit`] only when the second-stage flood reaches and
    /// paints it (design 2.3.6), and is [`PeekCell::HIDDEN`] otherwise.
    ///
    /// Call it after the faithful render, which leaves the visibility
    /// buffer the flood is seeded from. Before the buffer has ever been
    /// filled the ring is all hidden.
    pub fn peek_top_down_ring(
        &self,
        window: PeekWindow,
        light: PeekLightPolicy,
    ) -> Option<TopDownPeekRing> {
        if !window.is_valid() || !self.top_down_world_view_active() {
            return None;
        }
        let area = self.top_down_render_area()?;
        let mut ring = TopDownPeekRing::all_hidden(window);
        self.flood_peek_ring(area, light, &mut ring);
        Some(ring)
    }

    /// The second-stage flood of design 2.3.6 over the ring cells of
    /// `ring.window`, painting what it reaches into `ring.cells`.
    ///
    /// It is the faithful centre-out carve
    /// ([`PlayState::surface_visibility_carve_with_light_threshold`])
    /// continued past the core: the same neighbour order, the same
    /// propagation classifier, the same terrain read with world wrap and
    /// the town off-floor substitution, the same light mask with the
    /// beacon stamp, and the same three-way verdict. Only the "in range"
    /// test changes with the light policy.
    fn flood_peek_ring(
        &self,
        area: TopDownRenderArea,
        light: PeekLightPolicy,
        ring: &mut TopDownPeekRing,
    ) {
        let window = ring.window;
        let wrap_world = matches!(area, TopDownRenderArea::World(_));
        let (pitch_dark, threshold) = match area {
            TopDownRenderArea::Town => (
                self.surface_visibility_pitch_dark(),
                self.surface_visibility_light_threshold(),
            ),
            TopDownRenderArea::World(_) => (
                self.world_visibility_pitch_dark(),
                self.world_visibility_light_threshold(),
            ),
        };
        // Pitch dark (and the overworld underfoot blackout, which is pitch
        // dark by another road) leaves the faithful grid fully obscured,
        // so nothing in the ring can be seen either. A buffer that was
        // never filled has no seeds to trust.
        if pitch_dark || !self.visibility_buffers_ready {
            return;
        }
        let in_range = |squared_distance: u32| match light {
            // Full daylight reaches the whole window, as it reaches the
            // whole 11x11 screen; below it the true squared distance, so
            // dusk, night and a torch stay radii in world cells.
            PeekLightPolicy::Scaled if threshold >= u32::from(FULL_DAYLIGHT) => true,
            PeekLightPolicy::Scaled | PeekLightPolicy::Faithful => {
                visibility_in_radius(squared_distance, threshold)
            }
        };

        let px = self.player.x as isize;
        let py = self.player.y as isize;

        // The light mask exactly as the carve builds it: the persistent
        // disc-shaped sources, then the beacon stamp.
        let mut mask = self.local_light_mask.to_vec();
        let (mask_origin_x, mask_origin_y) =
            super::surface_local_light_mask_origin(px, py, wrap_world);
        self.stamp_light_beacon(&mut mask, mask_origin_x, mask_origin_y, wrap_world);
        let lit = |dx: isize, dy: isize| {
            Self::surface_local_light_mask_is_lit(
                &mask,
                mask_origin_x,
                mask_origin_y,
                px + dx,
                py + dy,
                wrap_world,
            )
        };
        let squared = |dx: isize, dy: isize| {
            let (ax, ay) = (dx.unsigned_abs() as u32, dy.unsigned_abs() as u32);
            ax.saturating_mul(ax).saturating_add(ay.saturating_mul(ay))
        };

        let mut considered = vec![false; window.cell_count()];
        let mut painted = vec![false; window.cell_count()];
        let mut queue: VecDeque<(isize, isize)> = VecDeque::new();

        // 1. Seeds: the visible core-edge cells whose terrain propagates
        // sight, read from the faithful buffer in row-major order. A
        // visible blocker at the edge is painted in the core but never
        // expands, as in the faithful carve.
        let reach = isize::from(PEEK_CORE_REACH);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                if dx.abs() != reach && dy.abs() != reach {
                    continue;
                }
                let row = (dy + reach) as usize;
                let col = (dx + reach) as usize;
                let Some(grid_index) = visibility_grid_active_index(row, col) else {
                    continue;
                };
                if self.visibility_grid[grid_index] == VISIBILITY_HIDDEN {
                    continue;
                }
                let Some(tile) = self.surface_viewport_tile(px + dx, py + dy, wrap_world) else {
                    continue;
                };
                if surface_tile_propagates_visibility(tile, squared(dx, dy)) {
                    queue.push_back((dx, dy));
                }
            }
        }

        // 2-4. The flood over the ring only.
        while let Some((cx, cy)) = queue.pop_front() {
            // Seeds are visible core cells; a ring cell carries its paint.
            let parent_painted = PeekWindow::is_core(cx, cy)
                || window.index_of(cx, cy).is_some_and(|index| painted[index]);
            for (ndx, ndy) in VISIBILITY_CARVE_NEIGHBOR_ORDER {
                let x = cx + isize::from(ndx);
                let y = cy + isize::from(ndy);
                // A flood never re-enters the core, nor leaves the window.
                if PeekWindow::is_core(x, y) {
                    continue;
                }
                let Some(index) = window.index_of(x, y) else {
                    continue;
                };
                if considered[index] {
                    continue;
                }
                considered[index] = true;

                let Some(tile) = self.surface_viewport_tile(px + x, py + y, wrap_world) else {
                    continue;
                };
                let squared_distance = squared(x, y);
                let propagates = surface_tile_propagates_visibility(tile, squared_distance);
                let (paint, expand) = if in_range(squared_distance) {
                    (true, propagates)
                } else if propagates {
                    (lit(x, y), true)
                } else {
                    (parent_painted && lit(cx, cy) && lit(x, y), false)
                };

                if paint {
                    let Some(terrain) = self.top_down_live_terrain_at(area, px + x, py + y) else {
                        continue;
                    };
                    painted[index] = true;
                    ring.cells[index] = Some(PeekCell {
                        tile: terrain,
                        shade: PeekShade::Lit,
                    });
                }
                if expand {
                    queue.push_back((x, y));
                }
            }
        }
    }
}
