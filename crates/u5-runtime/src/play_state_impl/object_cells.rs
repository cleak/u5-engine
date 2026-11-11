//! Which map the viewport shows, and the object-cell peek (Phase B
//! design 9.3.5, 9.3.7).
//!
//! A child module of `chunk_09`, as the ring peek is, so it can read the
//! top-down buffers without making them public. It holds [`MapViewKind`]
//! and [`PlayState::map_view_kind`], the one predicate the grow-left
//! layout (2.2.8), the brightness step (9.3.3) and the sprite composite
//! (9.3.8) share, and [`PlayState::peek_object_cells`], which tells the
//! composite step which viewport cells the faithful render filled with an
//! object and what the cell would show without it.
//!
//! Everything here takes `&self` and draws nothing from the RNG. A source
//! scan in the tests keeps it that way: this file may call no method that
//! takes the state mutably.

use crate::enhancements::sprite_mask::{SpriteMasks, composite_eligible};
use crate::*;

use super::ring_peek::terrain_tile_animates;

/// The kind of map the 11x11 viewport shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapViewKind {
    /// The top-down world (either plane).
    World,
    /// The top-down town, castle, keep or village.
    Town,
    /// A combat arena.
    Combat,
    /// The first-person dungeon view.
    Dungeon,
}

impl MapViewKind {
    /// The plain top-down view, world or town (2.2.8).
    pub fn is_top_down(self) -> bool {
        matches!(self, Self::World | Self::Town)
    }
}

impl PlayState {
    /// Which map the viewport shows, if any. Read-only; takes `&self` and
    /// draws nothing from the RNG.
    ///
    /// It mirrors the branch order of the shell's base-frame render: the
    /// pending Blackthorn rescue, then the audience, then the shrine
    /// presentation, all `None`; then
    /// [`PlayState::render_top_down_base_frame`], which draws combat first
    /// ([`MapViewKind::Combat`]), then the dungeon when there is no
    /// top-down render area ([`MapViewKind::Dungeon`]), otherwise the world
    /// or a town. It is also `None` while a view overlay (the gem view and
    /// its kin), the endgame or a visibility sweep is present, since those
    /// keep the faithful frame.
    pub fn map_view_kind(&self) -> Option<MapViewKind> {
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
            return None;
        }
        if self.blackthorn_audience_map.is_some() || self.shrine_presentation_map.is_some() {
            return None;
        }
        if self.active_view_overlay.is_some()
            || self.endgame.is_some()
            || self.visibility_sweep.is_some()
        {
            return None;
        }
        // `render_top_down_viewport`: combat first, then the dungeon when
        // there is no top-down render area.
        if self.combat_active {
            return Some(MapViewKind::Combat);
        }
        Some(match self.top_down_render_area() {
            None => MapViewKind::Dungeon,
            Some(TopDownRenderArea::World(_)) => MapViewKind::World,
            Some(TopDownRenderArea::Town) => MapViewKind::Town,
        })
    }
}

/// One viewport cell the faithful render filled with an object's tile
/// (design 9.3.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectCell {
    /// Viewport column, 0..11.
    pub col: u8,
    /// Viewport row, 0..11.
    pub row: u8,
    /// Atlas index the faithful renderer drew: actor byte + 256.
    pub drawn: u16,
    /// Terrain byte under the object (9.3.5), after the animated-tile
    /// frame table.
    pub under: u8,
    /// 16x16 palette indices of `under`, through the tile animator, the
    /// live moon-gate phase and, in a dungeon-room arena, the plane swap.
    pub under_pixels: [u8; TILE_ATLAS_TILE_PIXELS],
}

/// The object cells of one faithful frame (design 9.3.7).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectCellPeek {
    /// Row-major over the viewport.
    pub cells: Vec<ObjectCell>,
    /// True if any `under` moves with the animation clocks alone (water,
    /// lava, a fire fixture, an animated family member or a live moon
    /// gate), as the ring peek's `animated` is set. The shell peeks again
    /// on an animation tick while this is set.
    pub animated_under: bool,
}

impl PlayState {
    /// The object cells of the frame the faithful render just drew.
    /// Read-only: takes `&self` and draws nothing from the RNG.
    ///
    /// Returns `None` unless [`PlayState::map_view_kind`] is
    /// [`MapViewKind::World`], [`MapViewKind::Town`] or
    /// [`MapViewKind::Combat`].
    ///
    /// - *World and town.* The 11x11 active cells of the visibility grid
    ///   the render filled: a use-companion cell is an object cell, `drawn`
    ///   is the actor tile of its terrain-band byte (the draw-nothing byte
    ///   is skipped), and `under` is the live map tile at the cell's world
    ///   coordinate with the world wrap and the town off-floor
    ///   substitution, through the animated-tile frame table.
    /// - *Combat.* Each arena cell with a drawn sprite; `under` is the
    ///   arena terrain through the frame table. The cell under the aim
    ///   marker is left out while the marker is drawn, since the marker's
    ///   black strokes would read as background (D-M2.1).
    ///
    /// A cell is left out when [`composite_eligible`] is false for it.
    /// Call it after the faithful render, whose buffers it reads; before
    /// they were ever filled the peek has no cells.
    pub fn peek_object_cells(
        &self,
        atlas: &TileAtlas,
        masks: &SpriteMasks,
    ) -> Option<ObjectCellPeek> {
        match self.map_view_kind()? {
            MapViewKind::World | MapViewKind::Town => {
                let area = self.top_down_render_area()?;
                Some(self.peek_top_down_object_cells(area, atlas, masks))
            }
            MapViewKind::Combat => Some(self.peek_combat_object_cells(atlas, masks)),
            MapViewKind::Dungeon => None,
        }
    }

    fn peek_top_down_object_cells(
        &self,
        area: TopDownRenderArea,
        atlas: &TileAtlas,
        masks: &SpriteMasks,
    ) -> ObjectCellPeek {
        let mut peek = ObjectCellPeek::default();
        if !self.visibility_buffers_ready {
            return peek;
        }
        let wrap_world = matches!(area, TopDownRenderArea::World(_));
        let reach = VIEWPORT_PLAYER_ROW as isize;
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                let (Some(grid_index), Some(band_index)) = (
                    visibility_grid_active_index(row, col),
                    terrain_band_active_index(row, col),
                ) else {
                    continue;
                };
                if self.visibility_grid[grid_index] != VISIBILITY_USE_COMPANION {
                    continue;
                }
                let byte = self.terrain_band[band_index];
                // The render skips a cell that composes to the hidden byte.
                if byte == VISIBILITY_HIDDEN {
                    continue;
                }
                let world_x = self.player.x as isize + col as isize - reach;
                let world_y = self.player.y as isize + row as isize - reach;
                // The render draws a live gate's phase frame for a cell whose
                // composed byte is the gate id, whatever the object.
                if byte == NATURAL_MOONGATE_LIVE_TILE
                    && self.live_natural_moongate_terrain_at(area, world_x, world_y)
                    && moongate_phase_draw(self.natural_moongate_counter)
                        != MoongatePhaseDraw::WholeGate
                {
                    continue;
                }
                let drawn = actor_tile_for_byte(byte);
                let under = self
                    .surface_viewport_tile(world_x, world_y, wrap_world)
                    .map(|tile| self.animation.resolve_static_tile(tile));
                if !composite_eligible(masks, drawn, under) {
                    continue;
                }
                let (Some(drawn), Some(under)) = (drawn, under) else {
                    continue;
                };
                let live_gate = under == NATURAL_MOONGATE_LIVE_TILE
                    && self.live_natural_moongate_terrain_at(area, world_x, world_y);
                let Some(under_pixels) =
                    self.object_cell_under_pixels(atlas, under, live_gate, false)
                else {
                    continue;
                };
                peek.animated_under |= self.peek_cell_animates(
                    area,
                    under,
                    u16::from(under),
                    col as isize - reach,
                    row as isize - reach,
                );
                peek.cells.push(ObjectCell {
                    col: col as u8,
                    row: row as u8,
                    drawn: drawn as u16,
                    under,
                    under_pixels,
                });
            }
        }
        peek
    }

    fn peek_combat_object_cells(&self, atlas: &TileAtlas, masks: &SpriteMasks) -> ObjectCellPeek {
        let mut peek = ObjectCellPeek::default();
        // The arena offset of the combat render, for the 11x11 viewport.
        let offset = (VIEWPORT_SIDE as isize - COMBAT_ARENA_SIDE as isize) / 2;
        let marker = self.combat_overlay_draw_cells().secondary_marker_cell;
        let swap = self.dungeon_room_arena_plane_swap_active();
        for arena_y in 0..COMBAT_ARENA_SIDE {
            for arena_x in 0..COMBAT_ARENA_SIDE {
                if marker == Some((arena_x as u8, arena_y as u8)) {
                    continue;
                }
                let Some(drawn) = self.combat_render_sprite_at(arena_x, arena_y) else {
                    continue;
                };
                let under = self
                    .animation
                    .resolve_static_tile(self.combat_terrain[arena_y][arena_x]);
                if !composite_eligible(masks, Some(drawn), Some(under)) {
                    continue;
                }
                let col = arena_x as isize + offset;
                let row = arena_y as isize + offset;
                if !(0..VIEWPORT_SIDE as isize).contains(&col)
                    || !(0..VIEWPORT_SIDE as isize).contains(&row)
                {
                    continue;
                }
                let swapped = swap && dungeon_room_plane_swap_applies(under);
                let Some(under_pixels) =
                    self.object_cell_under_pixels(atlas, under, false, swapped)
                else {
                    continue;
                };
                peek.animated_under |= terrain_tile_animates(under);
                peek.cells.push(ObjectCell {
                    col: col as u8,
                    row: row as u8,
                    drawn: drawn as u16,
                    under,
                    under_pixels,
                });
            }
        }
        peek
    }

    /// `under` drawn as the faithful render draws a terrain cell, into a
    /// one-cell scratch viewport: the live gate's phase frame, else the
    /// ordinary tile path with the water, lava and fire animators, then the
    /// plane swap when `swapped`. `None` if the atlas lacks a tile.
    pub(super) fn object_cell_under_pixels(
        &self,
        atlas: &TileAtlas,
        under: u8,
        live_gate: bool,
        swapped: bool,
    ) -> Option<[u8; TILE_ATLAS_TILE_PIXELS]> {
        let mut scratch = TileViewport {
            depth: atlas.depth,
            cells_wide: 1,
            cells_high: 1,
            width: TILE_ATLAS_SIDE,
            height: TILE_ATLAS_SIDE,
            pixels: vec![0; TILE_ATLAS_TILE_PIXELS],
        };
        let gate_drawn = live_gate
            && self
                .blit_natural_moongate_phase_cell(&mut scratch, atlas, 0, 0)
                .ok()?;
        if !gate_drawn {
            blit_terrain_tile_to_viewport(
                &mut scratch,
                atlas,
                usize::from(under),
                0,
                0,
                self.water_scroll,
                &self.fire_flicker,
            )
            .ok()?;
        }
        if swapped {
            swap_viewport_cell_red_green(&mut scratch, 0, 0);
        }
        scratch.pixels.try_into().ok()
    }
}
