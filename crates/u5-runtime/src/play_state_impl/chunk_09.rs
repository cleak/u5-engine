use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

impl PlayState {
    pub fn apply_world_damage_tile(&mut self, entry: WorldDamageTileEntry) -> String {
        let mut checked = 0;
        let mut reports = Vec::new();
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            checked += 1;
            let damage = self.world_damage_tile_roll(index, entry);
            let slot = self.party[index].slot;
            let applied = self.party[index].apply_damage(damage);
            reports.push(format!(
                "party slot {slot} took {applied} HP ({} HP left)",
                self.party[index].hp
            ));
        }
        if reports.is_empty() {
            format!(
                "{} damage skipped for {checked} living member(s)",
                entry.effect.label()
            )
        } else {
            format!("{} damage: {}", entry.effect.label(), reports.join("; "))
        }
    }

    pub fn world_damage_tile_roll(&self, member_index: usize, entry: WorldDamageTileEntry) -> u8 {
        1 + (self.world_damage_tile_seed(member_index, entry) % 8)
    }

    pub fn world_damage_tile_seed(&self, member_index: usize, entry: WorldDamageTileEntry) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (entry.x as u8).wrapping_mul(7)
            ^ (entry.y as u8).wrapping_mul(11)
            ^ (member_index as u8).wrapping_mul(13)
    }

    pub fn apply_world_encounter_probe(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<usize>> {
        let Some(entries) = load_world_encounter_entries(game_dir)? else {
            return Ok(None);
        };
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        let Some(entry) = entries
            .iter()
            .find(|entry| entry.plane == plane && entry.tile == tile)
            .copied()
        else {
            return Ok(None);
        };
        if entry.threshold == 0 || self.world_encounter_roll(entry) > entry.threshold {
            return Ok(None);
        }

        let x = (self.player.x as isize + entry.dx as isize).rem_euclid(WORLD_SIDE as isize);
        let y = (self.player.y as isize + entry.dy as isize).rem_euclid(WORLD_SIDE as isize);
        let x = x as usize;
        let y = y as usize;
        if (x, y) == (self.player.x, self.player.y)
            || !self.player_can_land_on_foot(Some(game_dir), x, y)?
        {
            return Ok(None);
        }

        let object = ActiveObject {
            type_byte: entry.type_byte,
            tile: entry.type_byte,
            x,
            y,
            z: plane.save_floor(),
            phase: entry.phase,
            aux1: 0,
            aux3: 0,
        };
        let Some(slot) = self.allocate_active_object_slot(object) else {
            return Ok(None);
        };
        self.mark_visibility_dirty();
        Ok(Some(slot))
    }

    pub fn world_encounter_roll(&self, entry: WorldEncounterEntry) -> u8 {
        let seed = self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (self.player.x as u8).wrapping_mul(7)
            ^ (self.player.y as u8).wrapping_mul(11)
            ^ entry.tile.wrapping_mul(13)
            ^ entry.type_byte.wrapping_mul(17)
            ^ (entry.dx as u8).wrapping_mul(19)
            ^ (entry.dy as u8).wrapping_mul(23);
        1 + (seed % 30)
    }

    pub fn apply_world_waterfall_sweep(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        entry: WorldWaterfallEntry,
    ) -> io::Result<WorldWaterfallSweep> {
        let (dx, dy) = entry.direction.delta();
        let mut x = entry.x;
        let mut y = entry.y;
        let mut swept_steps = 0;
        for _ in 0..entry.steps {
            let nx = (x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
            let ny = (y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
            if self.world_object_at(nx, ny).is_some() {
                break;
            }
            let tile = self.grid[world_cell_index(nx, ny)];
            if let Some(entry) = self.world_plane_transition_at(game_dir, plane, nx, ny)? {
                x = nx;
                y = ny;
                swept_steps += 1;
                self.player.x = x;
                self.player.y = y;
                self.sync_player_object();
                self.mark_visibility_dirty();
                return Ok(WorldWaterfallSweep::PlaneTransition {
                    steps: swept_steps,
                    entry,
                });
            }
            if let Some(entry) = self.moongate_at(plane, nx, ny) {
                x = nx;
                y = ny;
                swept_steps += 1;
                self.player.x = x;
                self.player.y = y;
                self.sync_player_object();
                self.mark_visibility_dirty();
                return Ok(WorldWaterfallSweep::Moongate {
                    steps: swept_steps,
                    entry,
                });
            }
            if let Some(entry) = self.world_damage_tile_at(game_dir, plane, nx, ny, tile)? {
                if !entry.effect.allows_transport(self.player.transport) {
                    break;
                }
            } else if !self.tile_walkable(tile) {
                break;
            }
            x = nx;
            y = ny;
            swept_steps += 1;
        }
        self.player.x = x;
        self.player.y = y;
        self.sync_player_object();
        self.mark_visibility_dirty();
        Ok(WorldWaterfallSweep::Settled { steps: swept_steps })
    }

    pub fn apply_world_plane_transition(
        &mut self,
        game_dir: &Path,
        entry: WorldPlaneTransitionEntry,
    ) -> io::Result<()> {
        let fall_damage_report = self.apply_world_plane_fall_damage(entry);
        self.cache_current_world_overlay();
        self.area = Area::World {
            plane: entry.to_plane,
        };
        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        self.force_foot_transport();
        self.grid = load_world_map(game_dir, entry.to_plane)?;
        self.npcs.clear();
        self.replace_world_active_objects(game_dir, entry.to_plane, entry.to_x, entry.to_y)?;
        self.clear_open_town_door_state();
        self.return_world = None;
        self.pending_moongate = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        self.message = match (entry.from_plane, entry.to_plane) {
            (WorldPlane::Britannia, WorldPlane::Underworld) => format!(
                "F-A-L-L-S! Falling into the underworld; landed at ({}, {}){}. {}.",
                entry.to_x,
                entry.to_y,
                fall_damage_report
                    .map(|report| format!("; {report}"))
                    .unwrap_or_default(),
                self.wind.status_message()
            ),
            (WorldPlane::Underworld, WorldPlane::Britannia) => format!(
                "Ascended from the underworld to Britannia at ({}, {}). {}.",
                entry.to_x,
                entry.to_y,
                self.wind.status_message()
            ),
            _ => format!(
                "Changed world plane from {} to {} at ({}, {}). {}.",
                entry.from_plane.key(),
                entry.to_plane.key(),
                entry.to_x,
                entry.to_y,
                self.wind.status_message()
            ),
        };
        Ok(())
    }

    pub fn apply_world_plane_fall_damage(
        &mut self,
        entry: WorldPlaneTransitionEntry,
    ) -> Option<String> {
        if entry.from_plane != WorldPlane::Britannia || entry.to_plane != WorldPlane::Underworld {
            return None;
        }

        let mut checked = 0;
        let mut reports = Vec::new();
        for index in 0..self.party.len() {
            if !self.party[index].conscious() {
                continue;
            }
            checked += 1;
            let damage = self.world_plane_fall_damage_roll(index, entry);
            let slot = self.party[index].slot;
            let applied = self.party[index].apply_damage(damage);
            reports.push(format!(
                "party slot {slot} took {applied} HP ({} HP left)",
                self.party[index].hp
            ));
        }

        if reports.is_empty() {
            Some(format!(
                "fall damage skipped for {checked} conscious member(s)"
            ))
        } else {
            Some(format!("fall damage: {}", reports.join("; ")))
        }
    }

    pub fn world_plane_fall_damage_roll(
        &self,
        member_index: usize,
        entry: WorldPlaneTransitionEntry,
    ) -> u8 {
        1 + (self.world_plane_fall_damage_seed(member_index, entry) % 5)
    }

    pub fn world_plane_fall_damage_seed(
        &self,
        member_index: usize,
        entry: WorldPlaneTransitionEntry,
    ) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(5)
            ^ self.clock.minute.wrapping_mul(3)
            ^ (entry.x as u8).wrapping_mul(7)
            ^ (entry.y as u8).wrapping_mul(11)
            ^ (entry.to_x as u8).wrapping_mul(13)
            ^ (entry.to_y as u8).wrapping_mul(17)
            ^ (member_index as u8).wrapping_mul(19)
    }

    pub fn render_text_frame(&mut self, radius: usize) -> String {
        self.sync_player_object();
        let frame = self.render_text_view(radius);
        self.visibility_dirty = false;
        frame
    }

    pub fn render_top_down_frame(
        &mut self,
        radius: usize,
        atlas: &TileAtlas,
    ) -> io::Result<Option<TileViewport>> {
        self.sync_player_object();
        let viewport = self.render_top_down_viewport(radius, atlas)?;
        if viewport.is_some() {
            self.visibility_dirty = false;
        }
        Ok(viewport)
    }

    pub fn render_top_down_viewport(
        &self,
        radius: usize,
        atlas: &TileAtlas,
    ) -> io::Result<Option<TileViewport>> {
        let cells = radius
            .checked_mul(2)
            .and_then(|diameter| diameter.checked_add(1))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "viewport cell count overflows")
            })?;
        let width = cells.checked_mul(TILE_ATLAS_SIDE).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "viewport width overflows")
        })?;
        let height = width;
        let pixel_count = width.checked_mul(height).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "viewport pixel count overflows",
            )
        })?;
        let mut viewport = TileViewport {
            depth: atlas.depth,
            cells_wide: cells,
            cells_high: cells,
            width,
            height,
            pixels: vec![0; pixel_count],
        };

        let Some(area) = self.top_down_render_area() else {
            return Ok(None);
        };
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = isize::try_from(radius).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "viewport radius is too large")
        })?;
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let world_x = px + cell_x as isize - r;
                let world_y = py + cell_y as isize - r;
                // Per the visibility spec: paint terrain first (opaque), then
                // composite sprites on top (palette-0 = transparent) so the
                // avatar/NPCs/monsters/moongate frames don't blot out the
                // underlying terrain with their black backgrounds.
                let Some(terrain) =
                    self.top_down_terrain_tile(area, px, py, world_x, world_y, radius)
                else {
                    continue;
                };
                blit_tile_to_viewport(&mut viewport, atlas, terrain, cell_x, cell_y)?;
                if let Some(sprite) =
                    self.top_down_sprite_tile(area, px, py, world_x, world_y, radius)
                {
                    // The PLAYER_TILE byte (0xFC) is a sentinel for "this
                    // is the avatar slot", not a real sprite -- the lower
                    // 8-bit tile space stores "a bellows" at 0xFC. Resolve
                    // it to the actual avatar sprite in the upper-half tile
                    // space (256..=511) before blitting.
                    let sprite_id: usize = if sprite == PLAYER_TILE {
                        PLAYER_SPRITE_TILE
                    } else {
                        sprite as usize
                    };
                    composite_sprite_id_to_viewport(
                        &mut viewport,
                        atlas,
                        sprite_id,
                        cell_x,
                        cell_y,
                    )?;
                }
            }
        }
        Ok(Some(viewport))
    }

    pub fn top_down_render_area(&self) -> Option<TopDownRenderArea> {
        match self.area {
            Area::Town { .. } => Some(TopDownRenderArea::Town),
            Area::World { plane } => Some(TopDownRenderArea::World(plane)),
            Area::Dungeon { .. } => None,
        }
    }

    /// Combined terrain + sprite lookup. Kept for callers (text renderer,
    /// raster diagnostics) that want a single glyph per cell. Returns the
    /// sprite override if one would composite onto the cell, otherwise the
    /// underlying terrain tile.
    pub fn top_down_view_tile(
        &self,
        area: TopDownRenderArea,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        radius: usize,
    ) -> Option<u8> {
        self.top_down_sprite_tile(area, px, py, x, y, radius)
            .or_else(|| self.top_down_terrain_tile(area, px, py, x, y, radius))
    }

    /// Terrain-only lookup for the cell at (x, y). Returns `None` if the cell
    /// is outside the visible radius / off-map / occluded by line of sight.
    pub fn top_down_terrain_tile(
        &self,
        area: TopDownRenderArea,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        radius: usize,
    ) -> Option<u8> {
        match area {
            TopDownRenderArea::Town => {
                let visible_radius = self.surface_visibility_radius(radius);
                if !self.town_cell_visible(px, py, x, y, visible_radius)
                    || !(0..32).contains(&x)
                    || !(0..32).contains(&y)
                {
                    return None;
                }
                let x = x as usize;
                let y = y as usize;
                Some(self.animation.resolve_static_tile(self.grid[y * 32 + x]))
            }
            TopDownRenderArea::World(_plane) => {
                let visible_radius = self.world_visibility_radius(radius);
                if !self.world_cell_visible(px, py, x, y, visible_radius) {
                    return None;
                }
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                Some(
                    self.animation
                        .resolve_static_tile(self.grid[world_cell_index(wx, wy)]),
                )
            }
        }
    }

    /// Sprite override for the cell at (x, y): player avatar, active object
    /// (NPC / monster / vehicle / item), or moongate frame. Composited on
    /// top of the terrain with palette index 0 treated as transparent.
    pub fn top_down_sprite_tile(
        &self,
        area: TopDownRenderArea,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        radius: usize,
    ) -> Option<u8> {
        if x == px && y == py {
            return Some(PLAYER_TILE);
        }
        match area {
            TopDownRenderArea::Town => {
                let visible_radius = self.surface_visibility_radius(radius);
                if !self.town_cell_visible(px, py, x, y, visible_radius)
                    || !(0..32).contains(&x)
                    || !(0..32).contains(&y)
                {
                    return None;
                }
                let x = x as usize;
                let y = y as usize;
                self.object_at_current_floor(x, y).map(|object| object.tile)
            }
            TopDownRenderArea::World(plane) => {
                let visible_radius = self.world_visibility_radius(radius);
                if !self.world_cell_visible(px, py, x, y, visible_radius) {
                    return None;
                }
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                if let Some(object) = self.world_object_at(wx, wy) {
                    Some(object.tile)
                } else if self.visible_moongate_at(plane, wx, wy) {
                    Some(self.animation.resolve_moongate_tile())
                } else {
                    None
                }
            }
        }
    }

    pub fn render_text_view(&self, radius: usize) -> String {
        let mut out = String::new();
        match self.area {
            Area::Town { scene, floor } => {
                out.push_str(&format!(
                    "{} floor {} {:02}:{:02} turn {} frame {}\n",
                    scene.key(),
                    floor,
                    self.clock.hour,
                    self.clock.minute,
                    self.turn,
                    self.animation.frame
                ));
                let px = self.player.x as isize;
                let py = self.player.y as isize;
                let r = radius as isize;
                let visible_radius = self.surface_visibility_radius(radius);
                for y in py - r..=py + r {
                    for x in px - r..=px + r {
                        if x == px && y == py {
                            out.push('@');
                        } else if !self.town_cell_visible(px, py, x, y, visible_radius) {
                            out.push(' ');
                        } else if (0..32).contains(&x) && (0..32).contains(&y) {
                            if let Some(object) =
                                self.object_at_current_floor(x as usize, y as usize)
                            {
                                out.push(render_glyph(object.tile));
                            } else {
                                let tile = self.grid[y as usize * 32 + x as usize];
                                let tile = self.animation.resolve_static_tile(tile);
                                out.push(render_glyph(tile));
                            }
                        } else {
                            out.push(' ');
                        }
                    }
                    out.push('\n');
                }
            }
            Area::Dungeon { scene, level } => {
                out.push_str(&format!(
                    "{} ({}) level {} facing {} {:02}:{:02} turn {} torch {} spell {}\n",
                    scene.key(),
                    scene.name(),
                    level,
                    self.player.facing.name(),
                    self.clock.hour,
                    self.clock.minute,
                    self.turn,
                    self.torch_counter,
                    self.light_spell_counter
                ));
                if !self.has_personal_light() {
                    out.push_str("darkness\n");
                    out.push_str(&self.message);
                    return out;
                }
                out.push_str(&self.dungeon_forward_view(level));
            }
            Area::World { plane } => {
                out.push_str(&format!(
                    "{} {:02}:{:02} turn {} frame {} wind {}\n",
                    plane.key(),
                    self.clock.hour,
                    self.clock.minute,
                    self.turn,
                    self.animation.frame,
                    self.wind.name()
                ));
                let px = self.player.x as isize;
                let py = self.player.y as isize;
                let r = radius as isize;
                let visible_radius = self.world_visibility_radius(radius);
                for y in py - r..=py + r {
                    for x in px - r..=px + r {
                        let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                        if wx == self.player.x && wy == self.player.y {
                            out.push('@');
                        } else if !self.world_cell_visible(px, py, x, y, visible_radius) {
                            out.push(' ');
                        } else if let Some(object) = self.world_object_at(wx, wy) {
                            out.push(render_glyph(object.tile));
                        } else if self.visible_moongate_at(plane, wx, wy) {
                            out.push(render_glyph(self.animation.resolve_moongate_tile()));
                        } else {
                            let tile = self.grid[world_cell_index(wx, wy)];
                            let tile = self.animation.resolve_static_tile(tile);
                            out.push(render_glyph(tile));
                        }
                    }
                    out.push('\n');
                }
            }
        }
        out.push_str(&self.message);
        out
    }

    pub fn surface_visibility_radius(&self, requested: usize) -> usize {
        if self.ambient_light == 0 || self.ambient_light >= FULL_DAYLIGHT {
            return requested;
        }

        let cap = if self.ambient_light >= DAWN_DUSK_LIGHT[5] {
            requested
        } else if self.ambient_light >= DAWN_DUSK_LIGHT[4] {
            4
        } else if self.ambient_light >= DAWN_DUSK_LIGHT[3] {
            3
        } else if self.ambient_light >= DAWN_DUSK_LIGHT[2] {
            2
        } else if self.ambient_light >= DAWN_DUSK_LIGHT[1] {
            1
        } else {
            0
        };
        requested.min(cap)
    }

    pub fn world_visibility_radius(&self, requested: usize) -> usize {
        if matches!(self.area, Area::World { .. })
            && is_water_tile(self.grid[world_cell_index(self.player.x, self.player.y)])
        {
            0
        } else {
            self.surface_visibility_radius(requested)
        }
    }

    pub fn town_cell_visible(
        &self,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        visible_radius: usize,
    ) -> bool {
        if !cell_in_visibility_radius(px, py, x, y, visible_radius) {
            return false;
        }
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return false;
        }
        surface_line_unblocked(px, py, x, y, |sx, sy| {
            if !(0..32).contains(&sx) || !(0..32).contains(&sy) {
                return true;
            }
            self.town_cell_blocks_sight(sx as usize, sy as usize)
        })
    }

    pub fn town_cell_blocks_sight(&self, x: usize, y: usize) -> bool {
        self.sight_blocking_object_at_current_floor(x, y).is_some()
            || surface_tile_blocks_sight(self.grid[y * 32 + x])
    }

    pub fn world_cell_visible(
        &self,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        visible_radius: usize,
    ) -> bool {
        if !cell_in_visibility_radius(px, py, x, y, visible_radius) {
            return false;
        }
        surface_line_unblocked(px, py, x, y, |sx, sy| {
            let wx = sx.rem_euclid(WORLD_SIDE as isize) as usize;
            let wy = sy.rem_euclid(WORLD_SIDE as isize) as usize;
            self.world_cell_blocks_sight(wx, wy)
        })
    }

    pub fn world_cell_blocks_sight(&self, x: usize, y: usize) -> bool {
        self.sight_blocking_object_at_current_floor(x, y).is_some()
            || world_surface_tile_blocks_sight(self.grid[world_cell_index(x, y)])
    }

    pub fn advance_turn(&mut self) {
        let minutes = self.turn_minute_increment();
        self.advance_turn_with_minutes(minutes);
    }

    pub fn advance_turn_with_minutes(&mut self, minutes: u8) {
        self.advance_turn_with_minutes_and_door_tick(minutes, true);
    }

    pub fn advance_turn_without_door_tick(&mut self) {
        let minutes = self.turn_minute_increment();
        self.advance_turn_with_minutes_and_door_tick(minutes, false);
    }

    pub fn advance_turn_with_minutes_and_door_tick(&mut self, minutes: u8, tick_doors: bool) {
        let effective_minutes = self.timing_status.effective_minutes(minutes);
        self.turn += 1;
        let previous_hour = self.clock.hour;
        let previous_moongates = self.visible_moongate_cells();
        self.clock.advance_minutes(effective_minutes);
        self.decay_light_counters(effective_minutes);
        if matches!(self.area, Area::Town { .. })
            && self.clock.hour != previous_hour
            && matches!(self.clock.hour, 5 | 20)
        {
            apply_dawn_dusk_substitution(&mut self.grid);
            self.mark_visibility_dirty();
        }
        self.recompute_daylight();
        if self.visible_moongate_cells() != previous_moongates {
            self.mark_visibility_dirty();
        }
        self.sync_player_object();
        if self.time_stop_counter != 0 {
            self.time_stop_counter = self.time_stop_counter.saturating_sub(1);
        } else {
            self.advance_npc_schedules();
            self.advance_active_objects();
        }
        self.age_active_effect();
        if tick_doors {
            self.tick_door_tracker();
        }
        self.advance_animation_clock();
    }

    pub fn mode_zero_cleanup(&mut self) {
        self.recompute_daylight();
    }

    pub fn recompute_daylight(&mut self) {
        if self.ambient_light >= DAYLIGHT_SENTINEL_MIN {
            return;
        }

        let previous = self.ambient_light;
        let mut ambient = self.base_daylight();
        if self.torch_counter != 0 {
            ambient = ambient.max(TORCH_LIGHT_FLOOR);
        }
        if self.light_spell_counter != 0 {
            ambient = ambient.max(LIGHT_SPELL_FLOOR);
        }
        self.ambient_light = ambient;
        if self.ambient_light != previous {
            self.visibility_dirty = true;
        }
    }

    pub fn base_daylight(&self) -> u8 {
        match self.area {
            Area::Dungeon { .. }
            | Area::World {
                plane: WorldPlane::Underworld,
            } => FULL_DARKNESS,
            _ => match self.clock.hour {
                0..=4 | 20..=23 => FULL_DARKNESS,
                5 => DAWN_DUSK_LIGHT[(self.clock.minute / 10).min(5) as usize],
                19 => DAWN_DUSK_LIGHT[((59 - self.clock.minute) / 10).min(5) as usize],
                _ => FULL_DAYLIGHT,
            },
        }
    }

    pub fn advance_visual_tick(&mut self) {
        self.sync_player_object();
        if self.time_stop_counter == 0 && !matches!(self.area, Area::Dungeon { .. }) {
            self.animate_active_objects();
        }
        self.advance_animation_clock();
    }

    pub fn decay_light_counters(&mut self, units: u8) {
        self.torch_counter = self.torch_counter.saturating_sub(units);
        self.light_spell_counter = self.light_spell_counter.saturating_sub(units);
    }

    pub fn age_active_effect(&mut self) {
        if self.active_effect_counter == 0 {
            self.active_effect_tag = None;
            return;
        }
        self.active_effect_counter = self.active_effect_counter.saturating_sub(1);
        if self.active_effect_counter == 0 {
            self.active_effect_tag = None;
        }
    }

    pub fn has_personal_light(&self) -> bool {
        self.torch_counter != 0 || self.light_spell_counter != 0
    }

    pub fn dungeon_torch_duration_roll(&self) -> u8 {
        let roll = self.turn as u8
            ^ self.clock.hour
            ^ self.clock.minute
            ^ self.player.x as u8
            ^ (self.player.y as u8).wrapping_shl(1);
        DUNGEON_TORCH_DURATION_MIN + (roll & 0x0f)
    }

    pub fn turn_minute_increment(&self) -> u8 {
        match self.area {
            Area::World { .. } => 2,
            _ => 1,
        }
    }

    pub fn advance_active_objects(&mut self) {
        if matches!(self.area, Area::Dungeon { .. }) {
            return;
        }
        self.animate_active_objects();
        self.prune_far_overworld_objects();
    }

    pub fn advance_animation_clock(&mut self) {
        self.animation.tick_static_tiles();
        if !self.visible_moongate_cells().is_empty() {
            self.animation.tick_moongate();
        }
    }

    pub fn animate_active_objects(&mut self) {
        for slot in 1..self.active_objects.len() {
            if self.active_objects[slot].is_empty()
                || self.active_objects[slot].is_player()
                || self.active_objects[slot].is_player_phantom()
            {
                continue;
            }
            let tick = self.active_objects[slot].tick_phase();
            let ship_wind = if (self.active_objects[slot].phase & 0x0f) == 0 {
                self.try_drift_active_ship(slot, tick)
            } else {
                ActiveShipWind::None
            };
            let ship_wind_changed = !matches!(ship_wind, ActiveShipWind::None);
            let wandered = !ship_wind_changed
                && (self.active_objects[slot].phase & 0x0f) == 0
                && self.try_wander_active_object(slot);
            if wandered {
                self.active_objects[slot].phase = (self.active_objects[slot].phase & 0xf0) | 0x02;
            }
            if ship_wind_changed
                || wandered
                || matches!(tick, PhaseTick::Countdown | PhaseTick::DecisionPoint)
            {
                if let Some(tile) = active_object_frame_tile(
                    self.active_objects[slot].type_byte,
                    self.active_objects[slot].phase,
                ) {
                    if self.active_objects[slot].tile != tile {
                        self.active_objects[slot].tile = tile;
                        self.mark_visibility_dirty();
                    }
                }
            }
        }
    }

    pub fn try_drift_active_ship(&mut self, slot: usize, tick: PhaseTick) -> ActiveShipWind {
        let Area::World { plane } = self.area else {
            return ActiveShipWind::None;
        };
        let object = self.active_objects[slot];
        if object.z != plane.save_floor() || !is_ship_object(object) {
            return ActiveShipWind::None;
        }
        let Some(heading) = cardinal_direction_from_active_object_phase(object.phase) else {
            return ActiveShipWind::None;
        };
        let Some(wind_direction) = self.wind.direction() else {
            return ActiveShipWind::Stalled;
        };

        if heading != wind_direction {
            if heading.opposite_cardinal() == Some(wind_direction)
                && matches!(tick, PhaseTick::Countdown)
            {
                // The prior stalled phase countdown completed, so this is the
                // slow same-axis movement turn.
            } else {
                if heading.opposite_cardinal() == Some(wind_direction) {
                    self.active_objects[slot].phase = (object.phase & 0xf0) | 0x01;
                }
                return ActiveShipWind::Stalled;
            }
        }

        let (dx, dy) = heading.delta();
        let nx = (object.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = (object.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if (nx, ny) == (self.player.x, self.player.y) {
            return ActiveShipWind::Stalled;
        }
        if self
            .active_objects
            .iter()
            .enumerate()
            .any(|(other_slot, other)| {
                other_slot != slot && !other.is_player() && self.object_occupies(*other, nx, ny)
            })
        {
            return ActiveShipWind::Stalled;
        }
        let tile = self.grid[world_cell_index(nx, ny)];
        if !is_tile_walkable_for_transport(
            tile,
            self.passability.as_ref(),
            TransportState::Ship {
                type_byte: object.type_byte,
                tile: object.tile,
                sails_hoisted: true,
                hull: object.aux1,
                skiffs: object.aux3,
            },
        ) {
            return ActiveShipWind::Stalled;
        }

        self.active_objects[slot].x = nx;
        self.active_objects[slot].y = ny;
        self.active_objects[slot].phase = object.phase & 0xf0;
        self.mark_visibility_dirty();
        ActiveShipWind::Drifted
    }

    pub fn try_wander_active_object(&mut self, slot: usize) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let object = self.active_objects[slot];
        if object.z != plane.save_floor() {
            return false;
        }
        if !is_ambient_wanderer_object(object) {
            return false;
        }
        let Some(direction) = direction_from_active_object_phase(object.phase) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        let nx = (object.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = (object.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if (nx, ny) == (self.player.x, self.player.y) {
            return false;
        }
        if self
            .active_objects
            .iter()
            .enumerate()
            .any(|(other_slot, other)| {
                other_slot != slot && !other.is_player() && self.object_occupies(*other, nx, ny)
            })
        {
            return false;
        }
        let tile = self.grid[world_cell_index(nx, ny)];
        if !is_tile_walkable_for_transport(tile, self.passability.as_ref(), TransportState::Foot) {
            return false;
        }
        self.active_objects[slot].x = nx;
        self.active_objects[slot].y = ny;
        self.mark_visibility_dirty();
        true
    }

    pub fn prune_far_overworld_objects(&mut self) {
        if !matches!(self.area, Area::World { .. }) {
            return;
        }
        let scroll_base = world_scroll_base(self.player.x, self.player.y);
        let mut pruned = false;
        for slot in 1..self.active_objects.len() {
            let object = self.active_objects[slot];
            if object.is_empty()
                || is_vehicle_object_tile(object.type_byte)
                || is_vehicle_object_tile(object.tile)
                || world_scroll_neighborhood_contains(scroll_base, object.x, object.y)
            {
                continue;
            }
            self.free_active_object_slot(slot);
            pruned = true;
        }
        if pruned {
            self.mark_visibility_dirty();
        }
    }

}
