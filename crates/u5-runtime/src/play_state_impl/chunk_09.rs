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
        if self.combat_active {
            return Ok(None);
        }
        let Some(entries) = load_world_encounter_entries(game_dir)? else {
            return Ok(self.apply_native_world_encounter_probe(plane));
        };
        self.apply_world_encounter_sidecar_probe(&entries, game_dir, plane)
    }

    pub fn apply_world_encounter_sidecar_probe(
        &mut self,
        entries: &[WorldEncounterEntry],
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<usize>> {
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        let Some(entry) = entries
            .iter()
            .find(|entry| entry.plane == plane && entry.tile == tile)
            .copied()
        else {
            return Ok(None);
        };
        if !random_encounter_probe_fires(entry.threshold, self.world_encounter_roll(entry)) {
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

    pub fn apply_native_world_encounter_probe(&mut self, plane: WorldPlane) -> Option<usize> {
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        let threshold =
            random_encounter_threshold(plane == WorldPlane::Underworld, tile, self.clock.hour);
        let roll = self.native_world_encounter_roll_1_to_30(0);
        if !random_encounter_probe_fires(threshold, roll) {
            return None;
        }
        self.spawn_native_world_encounter(plane)
    }

    pub fn spawn_native_world_encounter(&mut self, plane: WorldPlane) -> Option<usize> {
        let scroll_base = world_scroll_base(self.player.x, self.player.y);
        for attempt in 0..ENCOUNTER_SPAWNER_RETRY_LIMIT {
            let x = (scroll_base.0
                + usize::from(self.native_world_encounter_seed(attempt, 0x21))
                    % OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE)
                % WORLD_SIDE;
            let y = (scroll_base.1
                + usize::from(self.native_world_encounter_seed(attempt, 0x43))
                    % OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE)
                % WORLD_SIDE;
            if !encounter_spawner_separation_ok(
                x as u8,
                y as u8,
                self.player.x as u8,
                self.player.y as u8,
            ) || self.world_object_at(x, y).is_some()
            {
                continue;
            }

            let tile = self.grid[world_cell_index(x, y)];
            let Some(type_byte) = self.native_world_encounter_type(plane, tile, attempt) else {
                continue;
            };
            if sea_creature_spawn_seeds_aux(type_byte) && (tile & 0xf0) == 0x60 {
                continue;
            }

            let dx = wrapped_world_axis_delta(self.player.x, x);
            let dy = wrapped_world_axis_delta(self.player.y, y);
            let object = ActiveObject {
                type_byte,
                tile: type_byte,
                x,
                y,
                z: plane.save_floor(),
                phase: active_object_phase_toward_player(dx, dy),
                aux1: if sea_creature_spawn_seeds_aux(type_byte) {
                    SEA_CREATURE_SPAWN_AUX_SEED
                } else {
                    0
                },
                aux3: 0,
            };
            let slot = self.allocate_active_object_slot(object)?;
            self.mark_visibility_dirty();
            return Some(slot);
        }
        None
    }

    pub fn native_world_encounter_type(
        &self,
        plane: WorldPlane,
        tile: u8,
        attempt: u8,
    ) -> Option<u8> {
        let underworld = plane == WorldPlane::Underworld;
        match spawn_terrain_branch(tile, underworld) {
            SpawnTerrainBranch::SurfaceTile1WhirlpoolOrAquatic => {
                if self.native_world_encounter_mod(attempt, 0x61, SPAWN_WHIRLPOOL_DENOMINATOR) == 0
                {
                    Some(0xEC)
                } else {
                    self.native_world_encounter_bucket_pick(&SURFACE_AQUATIC_BUCKET, attempt, 0x62)
                }
            }
            SpawnTerrainBranch::SeaSerpentAdjacency => {
                (self.native_world_encounter_mod(attempt, 0x63, SPAWN_SEA_SERPENT_DENOMINATOR)
                    == 0)
                    .then_some(0xE0)
            }
            SpawnTerrainBranch::UnderworldTile4RotWorm => Some(0xF8),
            SpawnTerrainBranch::HardReject | SpawnTerrainBranch::HighTileReject => None,
            SpawnTerrainBranch::LowTileAllowance => {
                if self.native_world_encounter_mod(
                    attempt,
                    0x64,
                    SPAWN_LOW_TILE_ALLOWANCE_DENOMINATOR,
                ) != 0
                {
                    return None;
                }
                if underworld {
                    self.native_world_encounter_bucket_pick(
                        &UNDERWORLD_AQUATIC_BUCKET,
                        attempt,
                        0x65,
                    )
                } else {
                    self.native_world_encounter_bucket_pick(&SURFACE_AQUATIC_BUCKET, attempt, 0x65)
                }
            }
            SpawnTerrainBranch::LandBucket => {
                if underworld {
                    self.native_world_encounter_bucket_pick(&UNDERWORLD_LAND_BUCKET, attempt, 0x66)
                } else {
                    self.native_world_encounter_bucket_pick(&SURFACE_LAND_BUCKET, attempt, 0x66)
                }
            }
        }
    }

    pub fn native_world_encounter_bucket_pick(
        &self,
        bucket: &[(u8, u8)],
        attempt: u8,
        salt: u8,
    ) -> Option<u8> {
        pick_random_spawn_bucket(bucket, self.native_world_encounter_seed(attempt, salt))
    }

    pub fn native_world_encounter_roll_1_to_30(&self, salt: u8) -> u8 {
        1 + self.native_world_encounter_seed(0, salt) % RANDOM_ENCOUNTER_DIE
    }

    pub fn native_world_encounter_mod(&self, attempt: u8, salt: u8, modulus: u8) -> u8 {
        if modulus == 0 {
            0
        } else {
            self.native_world_encounter_seed(attempt, salt) % modulus
        }
    }

    pub fn native_world_encounter_seed(&self, attempt: u8, salt: u8) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (self.player.x as u8).wrapping_mul(7)
            ^ (self.player.y as u8).wrapping_mul(11)
            ^ attempt.wrapping_mul(17)
            ^ salt
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
        1 + (seed % RANDOM_ENCOUNTER_DIE)
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
        1 + (self.world_plane_fall_damage_seed(member_index, entry) % WORLD_PLANE_FALL_DAMAGE_MAX)
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
        // One visibility check per cell, not two. The terrain tile is
        // painted first; the sprite (avatar / NPC / monster / moongate)
        // blits opaquely over it per the visibility-spec active-object
        // compositor.
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let world_x = px + cell_x as isize - r;
                let world_y = py + cell_y as isize - r;
                let Some((terrain, sprite)) =
                    self.top_down_render_cell(area, px, py, world_x, world_y, radius)
                else {
                    continue;
                };
                blit_tile_to_viewport(&mut viewport, atlas, terrain, cell_x, cell_y)?;
                if let Some(sprite) = sprite {
                    let sprite_id: usize = if sprite == PLAYER_TILE {
                        // PLAYER_TILE is a sentinel; 0xFC is "a bellows"
                        // in the lower-half tile space. Resolve to the
                        // real upper-half avatar sprite before blitting.
                        PLAYER_SPRITE_TILE
                    } else {
                        sprite as usize
                    };
                    blit_tile_id_to_viewport(&mut viewport, atlas, sprite_id, cell_x, cell_y)?;
                }
            }
        }
        Ok(Some(viewport))
    }

    /// Single-pass visibility + terrain + sprite lookup for one cell.
    /// Returns `None` if the cell is occluded or off-map. The Option in
    /// the second tuple slot is the sprite to composite on top of the
    /// terrain (player avatar, active object, or moongate frame).
    pub fn top_down_render_cell(
        &self,
        area: TopDownRenderArea,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        radius: usize,
    ) -> Option<(u8, Option<u8>)> {
        match area {
            TopDownRenderArea::Town => {
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    return None;
                }
                let visible_radius = self.surface_visibility_radius(radius);
                if !self.town_cell_visible(px, py, x, y, visible_radius) {
                    return None;
                }
                let xu = x as usize;
                let yu = y as usize;
                let terrain = self.animation.resolve_static_tile(self.grid[yu * 32 + xu]);
                let sprite = if x == px && y == py {
                    Some(PLAYER_TILE)
                } else {
                    self.object_at_current_floor(xu, yu)
                        .map(|object| object.tile)
                };
                Some((terrain, sprite))
            }
            TopDownRenderArea::World(plane) => {
                let visible_radius = self.world_visibility_radius(radius);
                if !self.world_cell_visible(px, py, x, y, visible_radius) {
                    return None;
                }
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                let terrain = self
                    .animation
                    .resolve_static_tile(self.grid[world_cell_index(wx, wy)]);
                let sprite = if x == px && y == py {
                    Some(PLAYER_TILE)
                } else if let Some(object) = self.world_object_at(wx, wy) {
                    Some(object.tile)
                } else if self.visible_moongate_at(plane, wx, wy) {
                    Some(self.animation.resolve_moongate_tile())
                } else {
                    None
                };
                Some((terrain, sprite))
            }
        }
    }

    pub fn top_down_render_area(&self) -> Option<TopDownRenderArea> {
        match self.area {
            Area::Town { .. } => Some(TopDownRenderArea::Town),
            Area::World { plane } => Some(TopDownRenderArea::World(plane)),
            Area::Dungeon { .. } => None,
        }
    }

    pub fn viewport_has_animated_tiles(&self, radius: usize) -> bool {
        let Some(area) = self.top_down_render_area() else {
            return false;
        };
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = radius as isize;
        for y in py - r..=py + r {
            for x in px - r..=px + r {
                let tile = match area {
                    TopDownRenderArea::Town => {
                        if !(0..32).contains(&x) || !(0..32).contains(&y) {
                            continue;
                        }
                        self.grid[y as usize * 32 + x as usize]
                    }
                    TopDownRenderArea::World(_) => {
                        let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                        self.grid[world_cell_index(wx, wy)]
                    }
                };
                if static_tile_animation_family(tile).is_some() {
                    return true;
                }
            }
        }
        false
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

    pub fn advance_turn_with_minutes_and_active_objects(
        &mut self,
        minutes: u8,
        advance_active_objects: bool,
    ) {
        self.advance_turn_with_minutes_and_door_tick_and_active_objects(
            minutes,
            true,
            advance_active_objects,
        );
    }

    pub fn advance_turn_without_door_tick(&mut self) {
        let minutes = self.turn_minute_increment();
        self.advance_turn_with_minutes_and_door_tick(minutes, false);
    }

    pub fn advance_turn_with_minutes_and_door_tick(&mut self, minutes: u8, tick_doors: bool) {
        self.advance_turn_with_minutes_and_door_tick_and_active_objects(minutes, tick_doors, true);
    }

    pub fn advance_turn_with_minutes_and_door_tick_and_active_objects(
        &mut self,
        minutes: u8,
        tick_doors: bool,
        advance_active_objects: bool,
    ) {
        let negate_time_active = self.negate_time_active();
        let effective_minutes = if negate_time_active {
            0
        } else {
            self.timing_status.effective_minutes(minutes)
        };
        self.turn += 1;
        let previous_day = self.clock.day;
        let previous_hour = self.clock.hour;
        let previous_moongates = self.visible_moongate_cells();
        self.clock.advance_minutes(effective_minutes);
        if self.clock.day != previous_day {
            self.reroll_shadowlord_hideouts();
        }
        if previous_day == 28 && self.clock.day == 1 {
            self.fortunes_of_war = 0;
            age_stay_counters_month(&mut self.party_stay_counters);
            age_inn_registry_month(&mut self.inn_registry);
        }
        self.decay_light_counters(effective_minutes);
        if matches!(self.area, Area::Town { .. })
            && self.clock.hour != previous_hour
            && matches!(self.clock.hour, 5 | 20)
        {
            apply_dawn_dusk_substitution(&mut self.grid);
            self.mark_visibility_dirty();
        }
        self.recompute_daylight();
        self.refresh_natural_moongates();
        if self.visible_moongate_cells() != previous_moongates {
            self.mark_visibility_dirty();
        }
        self.sync_player_object();
        if self.time_stop_counter != 0 {
            self.time_stop_counter = self.time_stop_counter.saturating_sub(1);
        } else if !negate_time_active {
            self.advance_npc_schedules();
            if advance_active_objects {
                self.advance_active_objects();
            }
        }
        self.age_active_effect();
        if tick_doors {
            self.tick_door_tracker();
        }
        self.advance_animation_clock();
    }

    pub fn mode_zero_cleanup(&mut self) {
        self.recompute_daylight();
        self.refresh_natural_moongates();
    }

    pub fn refresh_natural_moongates(&mut self) -> bool {
        if self.natural_moongate_night_window() {
            self.natural_moongate_counter = self
                .natural_moongate_counter
                .saturating_add(1)
                .min(NATURAL_MOONGATE_COUNTER_MAX);
        } else {
            self.natural_moongate_counter = self.natural_moongate_counter.saturating_sub(1);
        }

        let Some(indices) = self.natural_moongate_slot_indices_for_current_scene() else {
            return false;
        };
        let present = self.natural_moongate_counter != 0;
        let mut changed = false;

        for idx in 0..self.grid.len() {
            let eligible = indices.contains(&idx);
            let target = if eligible && present {
                NATURAL_MOONGATE_TERRAIN_TILE
            } else if (eligible && !present)
                || (!eligible && self.grid[idx] == NATURAL_MOONGATE_TERRAIN_TILE)
            {
                NATURAL_MOONGATE_RESTORED_TERRAIN_TILE
            } else {
                self.grid[idx]
            };
            if self.grid[idx] != target {
                self.grid[idx] = target;
                changed = true;
            }
        }

        if changed {
            self.mark_visibility_dirty();
            self.recompute_daylight();
        }
        changed
    }

    pub fn natural_moongate_night_window(&self) -> bool {
        matches!(self.clock.hour, 20..=23 | 0..=4)
    }

    pub fn natural_moongate_slot_indices_for_current_scene(&self) -> Option<Vec<usize>> {
        match self.area {
            Area::World { plane } => Some(
                self.moonstone_slots
                    .iter()
                    .copied()
                    .filter(|slot| slot.scene == 0 && WorldPlane::from_save_z(slot.z) == plane)
                    .map(|slot| world_cell_index(slot.x as usize, slot.y as usize))
                    .collect(),
            ),
            Area::Town { scene, floor } => Some(
                self.moonstone_slots
                    .iter()
                    .copied()
                    .filter(|slot| {
                        slot.scene == scene.byte
                            && slot.z as i8 == floor
                            && (slot.x as usize) < 32
                            && (slot.y as usize) < 32
                    })
                    .map(|slot| slot.y as usize * 32 + slot.x as usize)
                    .collect(),
            ),
            Area::Dungeon { .. } => None,
        }
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
        let (underworld, depth_z) = match self.area {
            Area::Dungeon { level, .. } => (false, level.saturating_add(1)),
            Area::World {
                plane: WorldPlane::Underworld,
            } => (true, 0),
            _ => (false, 0),
        };
        daylight_base_value(self.clock.hour, self.clock.minute, underworld, depth_z)
    }

    pub fn advance_visual_tick(&mut self) {
        self.sync_player_object();
        if self.time_stop_counter == 0
            && !self.negate_time_active()
            && !matches!(self.area, Area::Dungeon { .. })
        {
            self.animate_active_objects();
        }
        self.advance_animation_clock();
    }

    pub fn decay_light_counters(&mut self, units: u8) {
        self.torch_counter = self.torch_counter.saturating_sub(units);
        self.light_spell_counter = self.light_spell_counter.saturating_sub(units);
    }

    pub fn age_active_effect(&mut self) -> ActiveEffectAgeOutcome {
        let outcome = age_active_effect_state(self.active_effect_tag, self.active_effect_counter);
        self.active_effect_tag = outcome.tag;
        self.active_effect_counter = outcome.counter;
        if outcome.expired {
            self.mark_visibility_dirty();
        }
        outcome
    }

    pub fn negate_time_active(&self) -> bool {
        self.active_effect_tag == Some(NEGATE_TIME_ACTIVE_EFFECT_TAG)
            && self.active_effect_counter != 0
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
        if matches!(self.area, Area::World { .. }) {
            self.advance_outdoor_active_objects();
        } else {
            self.animate_active_objects();
        }
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

    pub fn advance_outdoor_active_objects(&mut self) {
        let mut last_vacated = None;
        for slot in (1..self.active_objects.len()).rev() {
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
                && self.try_wander_active_object_with_last_vacated(slot, &mut last_vacated);
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
        let mut last_vacated = None;
        self.try_wander_active_object_with_last_vacated(slot, &mut last_vacated)
    }

    pub fn try_wander_active_object_with_last_vacated(
        &mut self,
        slot: usize,
        last_vacated: &mut Option<(usize, usize)>,
    ) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let object = self.active_objects[slot];
        if object.z != plane.save_floor() || !is_outdoor_active_object_walker(object) {
            return false;
        }

        if let Some(direction) = self.outdoor_directed_step_direction(slot, object) {
            let (dx, dy) = direction.delta();
            if self.try_step_outdoor_active_object(slot, object, dx, dy, direction, last_vacated) {
                return true;
            }
        }

        let Some(direction) = direction_from_active_object_phase(object.phase)
            .filter(|direction| direction.is_cardinal())
        else {
            return false;
        };
        let (dx, dy) = direction.delta();
        self.try_step_outdoor_active_object(slot, object, dx, dy, direction, last_vacated)
    }

    pub fn outdoor_directed_step_direction(
        &self,
        slot: usize,
        object: ActiveObject,
    ) -> Option<Direction> {
        let (dx, dy) = directed_step_offsets(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        );
        let candidates = match axis_first_choice(self.outdoor_active_object_step_seed(slot, 0)) {
            Axis::X => [(dx, 0), (0, dy)],
            Axis::Y => [(0, dy), (dx, 0)],
        };
        candidates
            .into_iter()
            .filter(|(sx, sy)| *sx != 0 || *sy != 0)
            .find_map(|(sx, sy)| cardinal_direction_from_delta(sx, sy))
    }

    pub fn try_step_outdoor_active_object(
        &mut self,
        slot: usize,
        object: ActiveObject,
        dx: isize,
        dy: isize,
        direction: Direction,
        last_vacated: &mut Option<(usize, usize)>,
    ) -> bool {
        let nx = (object.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = (object.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if (nx, ny) == (self.player.x, self.player.y) || *last_vacated == Some((nx, ny)) {
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
        if !outdoor_active_object_step_accepts_tile(
            object.type_byte,
            tile,
            self.passability.as_ref(),
        ) {
            return false;
        }
        if !type_bypasses_terrain_chance_gate(object.type_byte) {
            if let Some(denominator) = terrain_chance_gate_denominator(tile) {
                if self.outdoor_active_object_step_seed(slot, tile) % denominator != 0 {
                    return false;
                }
            }
        }

        *last_vacated = Some((object.x, object.y));
        if outdoor_step_clears_on_destination(tile) {
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
            return true;
        }
        if sea_creature_spawn_seeds_aux(object.type_byte) {
            let facing = match direction {
                Direction::North => 0x2c,
                Direction::East => 0x2d,
                Direction::South => 0x2e,
                Direction::West => 0x2f,
                _ => object.type_byte,
            };
            self.active_objects[slot].type_byte = facing;
            self.active_objects[slot].tile = facing;
        }
        self.active_objects[slot].phase =
            active_object_phase_from_direction(direction, object.phase & 0x0f);
        self.active_objects[slot].x = nx;
        self.active_objects[slot].y = ny;
        self.mark_visibility_dirty();
        true
    }

    pub fn outdoor_active_object_step_seed(&self, slot: usize, salt: u8) -> u8 {
        self.turn as u8
            ^ self.clock.hour
            ^ self.clock.minute
            ^ (slot as u8).wrapping_mul(17)
            ^ (self.player.x as u8).wrapping_mul(3)
            ^ (self.player.y as u8).wrapping_mul(5)
            ^ salt
    }

    pub fn apply_world_active_object_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.combat_active {
            return Ok(None);
        }
        for direction in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            let (dx, dy) = direction.delta();
            let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
            let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
            let Some((object_slot, object)) = self
                .world_object_slot_at(x, y)
                .map(|(slot, object)| (slot, *object))
            else {
                continue;
            };
            if is_whirlpool_object(object) {
                continue;
            }
            if game_dir.join(BRIT_CBT_FILE).exists()
                && outdoor_combat_arena_index_for_object(object).is_some()
                && terrain_combat_base_class(object).is_some()
            {
                let note =
                    self.enter_terrain_combat_from_world_object(game_dir, plane, object_slot, object)?;
                self.message = format!(
                    "World object tile {} engaged from the {}; {note}.",
                    object.tile,
                    direction.name()
                );
                return Ok(Some(MoveOutcome::Used));
            }
        }
        Ok(None)
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

pub fn wrapped_world_axis_delta(from: usize, to: usize) -> i8 {
    let forward = (to + WORLD_SIDE - from) % WORLD_SIDE;
    if forward <= i8::MAX as usize {
        forward as i8
    } else {
        -((WORLD_SIDE - forward).min(i8::MAX as usize) as i8)
    }
}

pub fn cardinal_direction_from_delta(dx: i8, dy: i8) -> Option<Direction> {
    match (dx, dy) {
        (0, -1) => Some(Direction::North),
        (1, 0) => Some(Direction::East),
        (0, 1) => Some(Direction::South),
        (-1, 0) => Some(Direction::West),
        _ => None,
    }
}

pub fn is_outdoor_active_object_walker(object: ActiveObject) -> bool {
    is_outdoor_active_object_walker_byte(object.type_byte)
        || is_outdoor_active_object_walker_byte(object.tile)
}

pub const fn is_outdoor_active_object_walker_byte(byte: u8) -> bool {
    matches!(byte, 0x2c..=0x2f | 0x80..=0xff)
}

pub fn outdoor_active_object_step_accepts_tile(
    class_byte: u8,
    tile: u8,
    passability: Option<&TilePassability>,
) -> bool {
    if outdoor_active_object_class_immobile(class_byte) {
        return false;
    }
    if let Some(single_tile) = outdoor_active_object_single_tile_query(class_byte) {
        return tile == single_tile;
    }
    match class_byte {
        0x2c..=0x2f => water_creature_terrain_accepts(tile),
        0x80..=0x8f | 0x9c..=0x9f | 0xfc..=0xff => {
            tile <= 0x03 || (0x60..=0x6f).contains(&tile)
        }
        0x94..=0x97 | 0xb0..=0xb3 | 0xd8..=0xdf | 0xf0..=0xf3 => {
            (is_base_tile_passable(tile, passability) || is_water_tile(tile) || is_lava_tile(tile))
                && !is_mountain_tile(tile)
                && !is_wall_or_closed_door_tile(tile)
        }
        _ => {
            is_base_tile_passable(tile, passability)
                && !movement_chair_force_reject_applies(class_byte, tile)
        }
    }
}
