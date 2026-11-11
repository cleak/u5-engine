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
//! could not see from where it stands. The active objects of the current
//! floor are then composited onto the lit ring cells with the faithful
//! compositor, always at variant 0, so the peek never draws from the shared
//! gameplay stream. [`PlayState::render_peek_ring`] rasterises a ring the
//! way the faithful viewport blit does; with sprite compositing on,
//! [`PlayState::render_peek_ring_composited`] draws the ring's object
//! cells as the core's composite does (design 9.4.4, P5b), from the
//! `under` each stamped cell carries.

use std::collections::VecDeque;
use std::io;

use crate::enhancements::sprite_mask::{SpriteMasks, TileMask, composite_eligible, mask_bit};

use crate::{
    ActiveObject, ActiveObjectCompositeResult, FULL_DAYLIGHT, NATURAL_MOONGATE_LIVE_TILE,
    PlayState, TILE_ATLAS_SIDE, TILE_ATLAS_TILE_PIXELS, TileAtlas, TileViewport, TopDownRenderArea,
    VISIBILITY_CARVE_NEIGHBOR_ORDER, VISIBILITY_CLEAR, VISIBILITY_DIM_PERIPHERY, VISIBILITY_HIDDEN,
    VISIBILITY_USE_COMPANION, blit_terrain_tile_to_viewport, composite_active_object_slot,
    fire_fixture_spec, fire_pass_refreshes_field_tile, static_tile_animation_family,
    visibility_grid_active_index, visibility_in_radius, water_composite_mask,
    water_pass_rotates_tile,
};

use super::{PreparedTopDownCell, surface_tile_propagates_visibility, wrapped_world_axis_delta};

/// Whether terrain id `terrain` moves with the animation clocks alone: a
/// `§6` static-tile family, the water and lava rotation or a fire fixture.
/// The live moon gate is a property of the cell, not the id, and is tested
/// by the caller.
pub(super) fn terrain_tile_animates(terrain: u8) -> bool {
    static_tile_animation_family(terrain).is_some()
        || water_pass_rotates_tile(terrain)
        || water_composite_mask(terrain).is_some()
        || fire_fixture_spec(terrain).is_some()
}

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
    /// The 512-tile atlas id the faithful blit draws for this cell, as
    /// `PreparedTopDownCell::tile_id()`: a terrain id, or an actor stamp
    /// in the actor half (byte + 256), which `tile` alone cannot tell
    /// apart. `None` exactly when the cell is hidden.
    pub atlas: Option<u16>,
    /// For a cell the object pass stamped with an actor tile (a
    /// use-companion cell, as the core's object cells are): the terrain the
    /// cell shows when it is empty, the live map tile through the
    /// animated-tile frame table (design 9.4.4, P5b). `None` for terrain,
    /// direct-tile and hidden cells. Only the composited raster reads it.
    pub under: Option<u8>,
}

impl PeekCell {
    /// The cell for anything out of sight, carrying the same hidden byte
    /// a hidden core cell composes to.
    pub const HIDDEN: PeekCell = PeekCell {
        tile: VISIBILITY_HIDDEN,
        shade: PeekShade::Hidden,
        atlas: None,
        under: None,
    };

    /// A lit cell showing the terrain id `tile` with nothing over it.
    pub const fn lit_terrain(tile: u8) -> PeekCell {
        PeekCell {
            tile,
            shade: PeekShade::Lit,
            atlas: Some(tile as u16),
            under: None,
        }
    }
}

/// The cells around the faithful core, for one window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopDownPeekRing {
    pub window: PeekWindow,
    /// Row-major over the whole window; `None` inside the 11x11
    /// core, which the shell copies from the faithful frame.
    pub cells: Vec<Option<PeekCell>>,
    /// True if any cell holds an animated static tile: a lit cell whose
    /// picture moves with the animation clocks alone (a `§6` static-tile
    /// family, the water and lava rotation, a fire fixture, a field tile or
    /// a live moon gate). The shell peeks again on an animation tick while
    /// this is set, since the faithful frame need not change then.
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
    /// This is the fixed-screen test of design 2.2.8: exactly the
    /// [`MapViewKind::World`] and [`MapViewKind::Town`] answers of
    /// [`PlayState::map_view_kind`], which mirrors the shell's base-frame
    /// render (design 9.3.7). Combat, the dungeon, the cutscene screens, a
    /// view overlay, the endgame and a visibility sweep are all false.
    pub fn top_down_world_view_active(&self) -> bool {
        // One predicate (design 9.3.7): the world and town kinds of
        // [`PlayState::map_view_kind`].
        self.map_view_kind()
            .is_some_and(super::object_cells::MapViewKind::is_top_down)
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
        let mut prepared = self.flood_peek_ring(area, light, window);
        let mut unders = vec![None; prepared.len()];
        self.composite_peek_ring_objects(area, window, &mut prepared, &mut unders);
        for (index, cell) in prepared.into_iter().enumerate() {
            let Some(cell) = cell else {
                continue;
            };
            let (tile, atlas) = (cell.tile(), cell.tile_id());
            // A lit cell always has an atlas id: the compositor below
            // never keeps a stamp that would leave it without one.
            let Some(atlas) = atlas.and_then(|id| u16::try_from(id).ok()) else {
                continue;
            };
            let (dx, dy) = window.offset_of(index)?;
            ring.animated |= self.peek_cell_animates(area, tile, atlas, dx, dy);
            let under = unders[index];
            if let Some(under) = under {
                // The composited raster draws the terrain under the actor.
                ring.animated |= self.peek_cell_animates(area, under, u16::from(under), dx, dy);
            }
            ring.cells[index] = Some(PeekCell {
                tile,
                shade: PeekShade::Lit,
                atlas: Some(atlas),
                under,
            });
        }
        Some(ring)
    }

    /// The second-stage flood of design 2.3.6 over the ring cells of
    /// `window`.
    ///
    /// It is the faithful centre-out carve
    /// ([`PlayState::surface_visibility_carve_with_light_threshold`])
    /// continued past the core: the same neighbour order, the same
    /// propagation classifier, the same terrain read with world wrap and
    /// the town off-floor substitution, the same light mask with the
    /// beacon stamp, and the same three-way verdict. Only the "in range"
    /// test changes with the light policy.
    ///
    /// Returns the window row-major, with each painted ring cell as the
    /// buffer path prepares a lit cell (its live terrain in the band under
    /// the clear marker) and `None` everywhere else, the core included.
    fn flood_peek_ring(
        &self,
        area: TopDownRenderArea,
        light: PeekLightPolicy,
        window: PeekWindow,
    ) -> Vec<Option<PreparedTopDownCell>> {
        let mut prepared = vec![None; window.cell_count()];
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
            return prepared;
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
                    prepared[index] = Some(PreparedTopDownCell {
                        terrain,
                        grid: VISIBILITY_CLEAR,
                    });
                }
                if expand {
                    queue.push_back((x, y));
                }
            }
        }
        prepared
    }

    /// The object pass of design 2.3.6 over the painted ring cells.
    ///
    /// Every active object of the current floor is placed by the wrapped
    /// world delta or the town delta, as
    /// [`PlayState::top_down_object_viewport_cell`] places it, generalised
    /// to the window, and composited with [`composite_active_object_slot`]
    /// over the live map read [`PlayState::top_down_live_terrain_at`], in
    /// the faithful slot order (thirty-one down to one, so a lower slot
    /// paints over a higher one). Slot zero, the party, is always in the
    /// core and is left to the faithful frame.
    ///
    /// The variant is always 0, as the query-path compositor
    /// ([`PlayState::composite_top_down_object`]) takes it: the peek never
    /// calls [`PlayState::draw_active_object_composite_variant`], which
    /// draws from the shared gameplay stream (`visibility.md §8.1`), so a
    /// seated actor in the ring shows the first entry of its range.
    ///
    /// Objects show only on painted ring cells, and a stamp never writes
    /// into the core (the previous-row marker of the trapped-soul row is
    /// dropped when the row above is a core or an unpainted cell). A stamp
    /// that would leave the cell with no atlas tile (the draw-nothing actor
    /// byte, which the faithful blit answers with the previous frame's
    /// pixels) is not applied, so the ring shows what is under it.
    ///
    /// `unders` (one entry per window cell, all `None` on entry) receives
    /// the rule of the object-cell peek (design 9.3.5, 9.4.4): a cell the
    /// pass leaves as a use-companion stamp gets the terrain the flood
    /// painted there, which is the tile the cell shows when it is empty;
    /// any other stamp, or a previous-row marker written over a stamped
    /// cell, leaves it `None`.
    fn composite_peek_ring_objects(
        &self,
        area: TopDownRenderArea,
        window: PeekWindow,
        prepared: &mut [Option<PreparedTopDownCell>],
        unders: &mut [Option<u8>],
    ) {
        let Some(floor) = self.current_floor() else {
            return;
        };
        // The flood paints every lit ring cell with its live terrain under
        // the clear marker: the picture of the cell with nothing on it.
        let empty: Vec<Option<u8>> = prepared
            .iter()
            .map(|cell| cell.map(|cell| cell.terrain))
            .collect();
        let columns = window.columns();
        for slot in (1..self.active_objects.len()).rev() {
            let object = self.active_objects[slot];
            if object.is_empty() || object.z != floor {
                continue;
            }
            let Some((dx, dy)) = self.peek_object_offset(area, object) else {
                continue;
            };
            if PeekWindow::is_core(dx, dy) {
                continue;
            }
            let Some(index) = window.index_of(dx, dy) else {
                continue;
            };
            let Some(cell) = prepared[index] else {
                continue;
            };
            let current_grid_byte = match cell.grid {
                VISIBILITY_CLEAR | VISIBILITY_DIM_PERIPHERY => cell.terrain,
                byte => byte,
            };
            let object_x = object.x as isize;
            let object_y = object.y as isize;
            let current_terrain = self
                .top_down_live_terrain_at(area, object_x, object_y)
                .unwrap_or(cell.terrain);
            let previous_row_terrain = self.top_down_live_terrain_at(area, object_x, object_y - 1);
            let next_row_terrain = self.top_down_live_terrain_at(area, object_x, object_y + 1);
            let row = index / columns;
            let previous_index = (row > 0)
                .then(|| index - columns)
                .filter(|_| !PeekWindow::is_core(dx, dy - 1));
            let stamped = match composite_active_object_slot(
                false,
                object.type_byte,
                object.tile,
                current_grid_byte,
                current_terrain,
                previous_row_terrain,
                next_row_terrain,
                row,
                0,
            ) {
                ActiveObjectCompositeResult::Suppress => continue,
                ActiveObjectCompositeResult::Companion(tile) => PreparedTopDownCell {
                    terrain: tile,
                    grid: VISIBILITY_USE_COMPANION,
                },
                ActiveObjectCompositeResult::Direct(tile) => PreparedTopDownCell {
                    terrain: cell.terrain,
                    grid: tile,
                },
                ActiveObjectCompositeResult::PreviousRowDirectAndCompanion {
                    previous_marker,
                    tile,
                } => {
                    let marked = previous_index
                        .and_then(|previous| prepared[previous].map(|cell| (previous, cell)))
                        .map(|(previous, cell)| {
                            (
                                previous,
                                PreparedTopDownCell {
                                    terrain: cell.terrain,
                                    grid: previous_marker,
                                },
                            )
                        });
                    if let Some((previous, marked)) = marked
                        && marked.tile_id().is_some()
                    {
                        prepared[previous] = Some(marked);
                        unders[previous] = None;
                    }
                    PreparedTopDownCell {
                        terrain: tile,
                        grid: VISIBILITY_USE_COMPANION,
                    }
                }
            };
            if stamped.tile_id().is_some() && stamped.tile() != VISIBILITY_HIDDEN {
                prepared[index] = Some(stamped);
                unders[index] = (stamped.grid == VISIBILITY_USE_COMPANION)
                    .then_some(empty[index])
                    .flatten();
            }
        }
    }

    /// An object's offset from the party: the town delta, or the wrapped
    /// world delta on the overworld (as the faithful projection takes it).
    fn peek_object_offset(
        &self,
        area: TopDownRenderArea,
        object: ActiveObject,
    ) -> Option<(isize, isize)> {
        let (dx, dy) = match area {
            TopDownRenderArea::Town => (
                object.x as isize - self.player.x as isize,
                object.y as isize - self.player.y as isize,
            ),
            TopDownRenderArea::World(_) => (
                wrapped_world_axis_delta(self.player.x, object.x) as isize,
                wrapped_world_axis_delta(self.player.y, object.y) as isize,
            ),
        };
        let reach = isize::from(PEEK_MAX_REACH);
        ((-reach..=reach).contains(&dx) && (-reach..=reach).contains(&dy)).then_some((dx, dy))
    }

    /// Whether a lit ring cell's picture moves with the animation clocks
    /// alone, with no change to the game state.
    pub(super) fn peek_cell_animates(
        &self,
        area: TopDownRenderArea,
        tile: u8,
        atlas: u16,
        dx: isize,
        dy: isize,
    ) -> bool {
        let atlas = usize::from(atlas);
        if let Ok(terrain) = u8::try_from(atlas) {
            terrain_tile_animates(terrain)
                || (tile == NATURAL_MOONGATE_LIVE_TILE
                    && self.live_natural_moongate_terrain_at(
                        area,
                        self.player.x as isize + dx,
                        self.player.y as isize + dy,
                    ))
        } else {
            fire_pass_refreshes_field_tile(atlas)
        }
    }

    /// Rasterise `ring` as the faithful viewport blit draws its cells: one
    /// 16-pixel atlas tile per lit ring cell, through the water, lava and
    /// fire animators and the live moon-gate phase, at the state's current
    /// clocks. Core and hidden cells stay at palette index 0 (black); the
    /// shell copies the core from the faithful frame.
    ///
    /// Read-only, like the peek. Call it with the ring the peek just
    /// returned for this state.
    pub fn render_peek_ring(
        &self,
        ring: &TopDownPeekRing,
        atlas: &TileAtlas,
    ) -> io::Result<TileViewport> {
        self.render_peek_ring_composited(ring, atlas, None)
    }

    /// [`PlayState::render_peek_ring`] with sprite compositing (design
    /// 9.4.4, P5b). With `composite` = `Some((masks, halo))`, each lit cell
    /// with an `under` that [`composite_eligible`] accepts for its atlas
    /// tile is drawn as the core's composite draws it: the actor tile as
    /// the faithful blit draws it, then every pixel of the tile's
    /// background mask (less the 1-px halo with `halo`) that is still
    /// palette 0 takes the pixel of `under`, drawn as the faithful render
    /// draws a terrain cell (the animators and the live moon-gate phase).
    /// The mask is black in the actor tile by construction, so this is
    /// `under` and then the actor's unmasked pixels. A live moon-gate
    /// phase cell is drawn as before and never composited, as in the core.
    /// With `None` the raster is exactly [`PlayState::render_peek_ring`]'s.
    pub fn render_peek_ring_composited(
        &self,
        ring: &TopDownPeekRing,
        atlas: &TileAtlas,
        composite: Option<(&SpriteMasks, bool)>,
    ) -> io::Result<TileViewport> {
        let window = ring.window;
        if ring.cells.len() != window.cell_count() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "peek ring cell count does not match its window",
            ));
        }
        let width = window.columns() * TILE_ATLAS_SIDE;
        let height = window.rows() * TILE_ATLAS_SIDE;
        let mut viewport = TileViewport {
            depth: atlas.depth,
            cells_wide: window.columns(),
            cells_high: window.rows(),
            width,
            height,
            pixels: vec![0; width * height],
        };
        let area = self.top_down_render_area();
        for (index, cell) in ring.cells.iter().enumerate() {
            let Some(PeekCell {
                tile,
                shade: PeekShade::Lit,
                atlas: Some(atlas_id),
                under,
            }) = *cell
            else {
                continue;
            };
            let (dx, dy) = window
                .offset_of(index)
                .expect("the index is inside the window");
            let cell_x = index % window.columns();
            let cell_y = index / window.columns();
            if tile == NATURAL_MOONGATE_LIVE_TILE
                && let Some(area) = area
                && self.live_natural_moongate_terrain_at(
                    area,
                    self.player.x as isize + dx,
                    self.player.y as isize + dy,
                )
                && self.blit_natural_moongate_phase_cell(&mut viewport, atlas, cell_x, cell_y)?
            {
                continue;
            }
            blit_terrain_tile_to_viewport(
                &mut viewport,
                atlas,
                usize::from(atlas_id),
                cell_x,
                cell_y,
                self.water_scroll,
                &self.fire_flicker,
            )?;
            if let (Some((masks, halo)), Some(under)) = (composite, under)
                && composite_eligible(masks, Some(usize::from(atlas_id)), Some(under))
            {
                let live_gate = under == NATURAL_MOONGATE_LIVE_TILE
                    && area.is_some_and(|area| {
                        self.live_natural_moongate_terrain_at(
                            area,
                            self.player.x as isize + dx,
                            self.player.y as isize + dy,
                        )
                    });
                if let Some(under_pixels) =
                    self.object_cell_under_pixels(atlas, under, live_gate, false)
                {
                    composite_viewport_cell(
                        &mut viewport,
                        cell_x,
                        cell_y,
                        masks.background(usize::from(atlas_id), halo),
                        &under_pixels,
                    );
                }
            }
        }
        Ok(viewport)
    }
}

/// Writes `under` into cell `(cell_x, cell_y)` of `viewport` wherever
/// `mask` is set and the pixel is still palette 0, the core composite's
/// rule (design 9.3.8) on a palette raster.
fn composite_viewport_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    mask: &TileMask,
    under: &[u8; TILE_ATLAS_TILE_PIXELS],
) {
    let x0 = cell_x * TILE_ATLAS_SIDE;
    let y0 = cell_y * TILE_ATLAS_SIDE;
    for y in 0..TILE_ATLAS_SIDE {
        for x in 0..TILE_ATLAS_SIDE {
            if !mask_bit(mask, x, y) {
                continue;
            }
            let at = (y0 + y) * viewport.width + x0 + x;
            if let Some(pixel) = viewport.pixels.get_mut(at)
                && *pixel == 0
            {
                *pixel = under[y * TILE_ATLAS_SIDE + x];
            }
        }
    }
}
