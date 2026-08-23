use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Copy, Debug)]
struct PreparedTopDownCell {
    terrain: u8,
    grid: u8,
}

impl PreparedTopDownCell {
    fn tile(self) -> u8 {
        match visibility_marker(self.grid) {
            VisibilityMarker::UseCompanion
            | VisibilityMarker::ClearVisible
            | VisibilityMarker::DimPeriphery => self.terrain,
            VisibilityMarker::Hidden | VisibilityMarker::AlreadyRendered => VISIBILITY_HIDDEN,
            VisibilityMarker::DirectTile(tile) => tile,
        }
    }
}

impl PlayState {
    pub fn apply_world_damage_tile(&mut self, entry: WorldDamageTileEntry) -> String {
        let mut checked = 0;
        let mut reports = Vec::new();
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            checked += 1;
            let damage = self.world_damage_tile_roll();
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

    pub fn world_damage_tile_roll(&mut self) -> u8 {
        self.random_range_u8(1, 8)
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
        if let Some(slot) = self.apply_world_encounter_sidecar_probe(&entries, game_dir, plane)? {
            return Ok(Some(slot));
        }
        if self.world_encounter_sidecar_matches_underfoot(&entries, plane) {
            return Ok(None);
        }
        Ok(self.apply_native_world_encounter_probe(plane))
    }

    pub fn world_encounter_sidecar_matches_underfoot(
        &self,
        entries: &[WorldEncounterEntry],
        plane: WorldPlane,
    ) -> bool {
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        entries
            .iter()
            .any(|entry| entry.plane == plane && entry.tile == tile)
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
                + usize::from(
                    self.random_range_u8(0, (OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE - 1) as u8),
                ))
                % WORLD_SIDE;
            let y = (scroll_base.1
                + usize::from(
                    self.random_range_u8(0, (OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE - 1) as u8),
                ))
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
        &mut self,
        plane: WorldPlane,
        tile: u8,
        _attempt: u8,
    ) -> Option<u8> {
        let underworld = plane == WorldPlane::Underworld;
        match spawn_terrain_branch(tile, underworld) {
            SpawnTerrainBranch::SurfaceTile1WhirlpoolOrAquatic => {
                if self.native_world_encounter_mod(0, 0x61, SPAWN_WHIRLPOOL_DENOMINATOR) == 0 {
                    Some(0xEC)
                } else {
                    self.native_world_encounter_bucket_pick(&SURFACE_AQUATIC_BUCKET, 0, 0x62)
                }
            }
            SpawnTerrainBranch::SeaSerpentAdjacency => {
                (self.native_world_encounter_mod(0, 0x63, SPAWN_SEA_SERPENT_DENOMINATOR) == 0)
                    .then_some(0xE0)
            }
            SpawnTerrainBranch::UnderworldTile4RotWorm => Some(0xF8),
            SpawnTerrainBranch::HardReject | SpawnTerrainBranch::HighTileReject => None,
            SpawnTerrainBranch::LowTileAllowance => {
                if self.native_world_encounter_mod(0, 0x64, SPAWN_LOW_TILE_ALLOWANCE_DENOMINATOR)
                    != 0
                {
                    return None;
                }
                if underworld {
                    self.native_world_encounter_bucket_pick(&UNDERWORLD_AQUATIC_BUCKET, 0, 0x65)
                } else {
                    self.native_world_encounter_bucket_pick(&SURFACE_AQUATIC_BUCKET, 0, 0x65)
                }
            }
            SpawnTerrainBranch::LandBucket => {
                if underworld {
                    self.native_world_encounter_bucket_pick(&UNDERWORLD_LAND_BUCKET, 0, 0x66)
                } else {
                    self.native_world_encounter_bucket_pick(&SURFACE_LAND_BUCKET, 0, 0x66)
                }
            }
        }
    }

    pub fn native_world_encounter_bucket_pick(
        &mut self,
        bucket: &[(u8, u8)],
        _attempt: u8,
        _salt: u8,
    ) -> Option<u8> {
        pick_random_spawn_bucket(bucket, self.random_range_u8(0, u8::MAX))
    }

    pub fn native_world_encounter_roll_1_to_30(&mut self, _salt: u8) -> u8 {
        self.random_range_u8(1, RANDOM_ENCOUNTER_DIE)
    }

    pub fn native_world_encounter_mod(&mut self, _attempt: u8, _salt: u8, modulus: u8) -> u8 {
        self.random_mod_u8(modulus)
    }

    pub fn world_encounter_roll(&mut self, _entry: WorldEncounterEntry) -> u8 {
        self.random_range_u8(1, RANDOM_ENCOUNTER_DIE)
    }

    pub fn apply_world_plane_transition(
        &mut self,
        game_dir: &Path,
        entry: WorldPlaneTransitionEntry,
    ) -> io::Result<()> {
        let is_surface_chasm_fall = entry.from_plane == WorldPlane::Britannia
            && entry.to_plane == WorldPlane::Underworld
            && entry.x == usize::from(SURFACE_CHASM_X)
            && entry.y == usize::from(SURFACE_CHASM_Y);
        let preserve_transport = is_surface_chasm_fall;
        let pre_fall_transport = self.player.transport;
        let fall_damage_report = self.apply_world_plane_fall_damage(entry);
        self.cache_current_world_overlay();
        self.area = Area::World {
            plane: entry.to_plane,
        };
        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        if preserve_transport {
            self.player.transport = pre_fall_transport;
        } else {
            self.force_foot_transport();
        }
        self.grid = load_world_map(game_dir, entry.to_plane)?;
        self.rebuild_world_live_chunks_from_grid(entry.to_plane)?;
        self.natural_moongate_live_cells.clear();
        self.npcs.clear();
        self.replace_world_active_objects(game_dir, entry.to_plane, entry.to_x, entry.to_y)?;
        self.sync_player_object();
        self.clear_open_town_door_state();
        self.return_world = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
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
                self.wind_status_message()
            ),
            (WorldPlane::Underworld, WorldPlane::Britannia) => format!(
                "Ascended from the underworld to Britannia at ({}, {}). {}.",
                entry.to_x,
                entry.to_y,
                self.wind_status_message()
            ),
            _ => format!(
                "Changed world plane from {} to {} at ({}, {}). {}.",
                entry.from_plane.key(),
                entry.to_plane.key(),
                entry.to_x,
                entry.to_y,
                self.wind_status_message()
            ),
        };
        Ok(())
    }

    pub fn apply_world_plane_fall_damage(
        &mut self,
        entry: WorldPlaneTransitionEntry,
    ) -> Option<String> {
        if entry.from_plane != WorldPlane::Britannia
            || entry.to_plane != WorldPlane::Underworld
            || entry.x != usize::from(SURFACE_CHASM_X)
            || entry.y != usize::from(SURFACE_CHASM_Y)
        {
            return None;
        }

        let mut checked = 0;
        let mut reports = Vec::new();
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            checked += 1;
            let roll = self.world_plane_fall_save_roll();
            if self.party[index].climb_stat > roll {
                continue;
            }
            let slot = self.party[index].slot;
            let applied = self.party[index].apply_damage(1);
            reports.push(format!(
                "party slot {slot} failed Dex roll {roll} and took {applied} HP ({} HP left)",
                self.party[index].hp
            ));
        }

        if reports.is_empty() {
            Some(format!(
                "fall damage skipped for {checked} living member(s)"
            ))
        } else {
            Some(format!("fall damage: {}", reports.join("; ")))
        }
    }

    pub fn world_plane_fall_save_roll(&mut self) -> u8 {
        self.random_range_u8(0, WORLD_PLANE_FALL_SAVE_ROLL_MAX)
    }

    pub fn render_text_frame(&mut self, radius: usize) -> String {
        if let Some(overlay) = self.active_view_overlay.as_ref() {
            return format!("{}:\n{}", overlay.title, overlay.text_map);
        }
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
        if let Some(viewport) = self.render_active_view_overlay(atlas.depth) {
            return Ok(Some(viewport));
        }
        self.render_top_down_base_frame(radius, atlas)
    }

    pub fn render_top_down_base_frame(
        &mut self,
        radius: usize,
        atlas: &TileAtlas,
    ) -> io::Result<Option<TileViewport>> {
        if !self.combat_active && self.endgame.is_none() {
            self.sync_player_object();
        }
        let viewport = self.render_top_down_viewport(radius, atlas)?;
        if viewport.is_some() {
            self.visibility_dirty = false;
            self.advance_presentation_frame();
        }
        Ok(viewport)
    }

    pub fn render_top_down_viewport(
        &mut self,
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

        if self.combat_active {
            self.render_combat_viewport(&mut viewport, atlas)?;
            return Ok(Some(viewport));
        }

        let Some(area) = self.top_down_render_area() else {
            return self
                .render_dungeon_viewport(radius, atlas.depth, atlas.dungeon_billboards.as_ref())
                .map(Some);
        };
        let _ = isize::try_from(radius).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "viewport radius is too large")
        })?;
        let prepared = if radius == VIEWPORT_PLAYER_ROW {
            self.refresh_top_down_visibility_buffers(area, radius);
            self.prepared_top_down_grid_from_visibility_buffers()
        } else {
            self.prepare_top_down_render_grid(area, radius)
        };
        let r = radius as isize;
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let Some(cell) = prepared[cell_y * cells + cell_x] else {
                    continue;
                };
                let tile = cell.tile();
                if tile == VISIBILITY_HIDDEN {
                    continue;
                }
                // `overworld.md §9.1` (spec HEAD c00bf63): a live moon-gate
                // cell is not a plain tile. Resolve it through the shared
                // sixteen-step gate-presence counter before the ordinary
                // tile path gets it.
                if tile == NATURAL_MOONGATE_LIVE_TILE
                    && self.live_natural_moongate_terrain_at(
                        area,
                        self.player.x as isize + cell_x as isize - r,
                        self.player.y as isize + cell_y as isize - r,
                    )
                    && self.blit_natural_moongate_phase_cell(
                        &mut viewport,
                        atlas,
                        cell_x,
                        cell_y,
                    )?
                {
                    continue;
                }
                let tile_id = if tile == PLAYER_TILE {
                    // PLAYER_TILE is a sentinel; 0xFC is "a bellows" in the
                    // lower-half tile space. Resolve to the real upper-half
                    // avatar sprite before blitting.
                    PLAYER_SPRITE_TILE
                } else {
                    tile as usize
                };
                blit_tile_id_to_viewport(&mut viewport, atlas, tile_id, cell_x, cell_y)?;
            }
        }
        self.draw_white_potion_sweep_overlay(area, radius, &prepared, &mut viewport);
        Ok(Some(viewport))
    }

    /// `overworld.md §9.1` (spec HEAD c00bf63): is this cell's **live
    /// terrain** the moon-gate byte `0xDC`?
    ///
    /// The phase model special-cases live terrain, so a `0xDC` a renderer
    /// overlay merely painted over some other terrain is not a gate cell
    /// and keeps the ordinary tile path.
    fn live_natural_moongate_terrain_at(
        &self,
        area: TopDownRenderArea,
        world_x: isize,
        world_y: isize,
    ) -> bool {
        let idx = match area {
            TopDownRenderArea::World(_) => {
                let wx = world_x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = world_y.rem_euclid(WORLD_SIDE as isize) as usize;
                world_cell_index(wx, wy)
            }
            TopDownRenderArea::Town { .. } => {
                if !(0..32).contains(&world_x) || !(0..32).contains(&world_y) {
                    return false;
                }
                world_y as usize * 32 + world_x as usize
            }
        };
        self.grid.get(idx).copied() == Some(NATURAL_MOONGATE_LIVE_TILE)
    }

    /// `overworld.md §9.1` (spec HEAD c00bf63): the ground half of a
    /// composed gate frame. Ordinary play uses grass, terrain `5`, the
    /// same tile the daytime pass restores; the endgame scene substitutes
    /// its throne-room floor `0x44`, which is why the endgame gate rises
    /// out of flagstones rather than turf.
    pub fn natural_moongate_phase_ground_tile(&self) -> u8 {
        moongate_phase_ground_tile(self.endgame.is_some())
    }

    /// `overworld.md §9.1` (spec HEAD c00bf63): draw one live moon-gate
    /// cell at the **global** gate-presence phase.
    ///
    /// Returns `false` when the phase is sixteen, i.e. when the whole
    /// moon-gate tile goes through the ordinary tile path and this cell
    /// needs no composition. Every visible gate reads the same counter,
    /// so gates in one view rise and sink in lockstep.
    fn blit_natural_moongate_phase_cell(
        &self,
        viewport: &mut TileViewport,
        atlas: &TileAtlas,
        cell_x: usize,
        cell_y: usize,
    ) -> io::Result<bool> {
        let rows = match moongate_phase_draw(self.natural_moongate_counter) {
            MoongatePhaseDraw::WholeGate => return Ok(false),
            // Phase zero shows zero gate rows, which is the ground plate:
            // the same frame the composition produces at `rows == 0`.
            MoongatePhaseDraw::Ground => 0,
            MoongatePhaseDraw::Composed { rows } => rows,
        };
        let ground_tile = self.natural_moongate_phase_ground_tile() as usize;
        let gate_tile = moongate_phase_gate_tile() as usize;
        let missing = |tile: usize| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("tile atlas is missing tile {tile}"),
            )
        };
        let ground = atlas
            .tile_pixels(ground_tile)
            .ok_or_else(|| missing(ground_tile))?
            .to_vec();
        let gate = atlas
            .tile_pixels(gate_tile)
            .ok_or_else(|| missing(gate_tile))?
            .to_vec();
        // The composed frame lives in the dedicated scratch tile `0x116`,
        // saved and restored around every composition so its shipped
        // artwork - which `overworld.md §9.2` reuses as the
        // party-vanishing sprite - survives.
        let mut scratch = atlas
            .tile_pixels(MOONGATE_PHASE_SCRATCH_TILE)
            .ok_or_else(|| missing(MOONGATE_PHASE_SCRATCH_TILE))?
            .to_vec();
        with_moongate_phase_scratch_tile(&mut scratch, &ground, &gate, rows, |composed| {
            blit_tile_pixels_to_viewport(viewport, composed, cell_x, cell_y)
        })??;
        Ok(true)
    }

    pub fn render_combat_viewport(
        &self,
        viewport: &mut TileViewport,
        atlas: &TileAtlas,
    ) -> io::Result<()> {
        let x_offset = (viewport.cells_wide as isize - COMBAT_ARENA_SIDE as isize) / 2;
        let y_offset = (viewport.cells_high as isize - COMBAT_ARENA_SIDE as isize) / 2;
        for cell_y in 0..viewport.cells_high {
            for cell_x in 0..viewport.cells_wide {
                let arena_x = cell_x as isize - x_offset;
                let arena_y = cell_y as isize - y_offset;
                if !(0..COMBAT_ARENA_SIDE as isize).contains(&arena_x)
                    || !(0..COMBAT_ARENA_SIDE as isize).contains(&arena_y)
                {
                    continue;
                }
                let arena_x = arena_x as usize;
                let arena_y = arena_y as usize;
                let terrain = self
                    .animation
                    .resolve_static_tile(self.combat_terrain[arena_y][arena_x]);
                blit_tile_to_viewport(viewport, atlas, terrain, cell_x, cell_y)?;
                if let Some(sprite) = self.combat_render_sprite_at(arena_x, arena_y) {
                    blit_tile_id_to_viewport(viewport, atlas, sprite, cell_x, cell_y)?;
                }
                if let Some(kind) = self.combat_potion_presentation_at(arena_x, arena_y) {
                    draw_combat_potion_presentation_cell(viewport, cell_x, cell_y, kind);
                }
                if self.combat_secondary_marker_cell() == Some((arena_x as u8, arena_y as u8)) {
                    draw_combat_secondary_marker_cell(viewport, cell_x, cell_y);
                }
                if self.combat_cursor_blink
                    && self.combat_cursor_actor_cell() == Some((arena_x as u8, arena_y as u8))
                {
                    draw_combat_cursor_marker_cell(viewport, cell_x, cell_y);
                }
            }
        }
        Ok(())
    }

    fn draw_white_potion_sweep_overlay(
        &self,
        area: TopDownRenderArea,
        radius: usize,
        prepared: &[Option<PreparedTopDownCell>],
        viewport: &mut TileViewport,
    ) {
        let Some(sweep) = self.white_potion_sweep else {
            return;
        };
        if sweep.frames_remaining == 0 {
            return;
        }
        let cells = radius.saturating_mul(2).saturating_add(1);
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = radius as isize;
        let radius_sq = u32::from(sweep.radius).saturating_mul(u32::from(sweep.radius));
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                if prepared
                    .get(cell_y * cells + cell_x)
                    .and_then(|cell| *cell)
                    .is_none()
                {
                    continue;
                }
                let world_x = px + cell_x as isize - r;
                let world_y = py + cell_y as isize - r;
                let (dx, dy) = match area {
                    TopDownRenderArea::Town => (
                        world_x - sweep.center_x as isize,
                        world_y - sweep.center_y as isize,
                    ),
                    TopDownRenderArea::World(_) => {
                        let wx = world_x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = world_y.rem_euclid(WORLD_SIDE as isize) as usize;
                        (
                            isize::from(wrapped_world_axis_delta(sweep.center_x, wx)),
                            isize::from(wrapped_world_axis_delta(sweep.center_y, wy)),
                        )
                    }
                };
                let dx = dx.unsigned_abs() as u32;
                let dy = dy.unsigned_abs() as u32;
                let distance_sq = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));
                if distance_sq <= radius_sq {
                    draw_white_potion_sweep_cell(viewport, cell_x, cell_y);
                }
            }
        }
    }

    fn combat_potion_presentation_at(
        &self,
        x: usize,
        y: usize,
    ) -> Option<CombatPotionPresentationKind> {
        let presentation = self.combat_potion_presentation?;
        let object = self.active_objects.get(presentation.active_object_slot)?;
        if object.is_empty() || object.x != x || object.y != y {
            return None;
        }
        Some(presentation.kind)
    }

    fn combat_secondary_marker_cell(&self) -> Option<(u8, u8)> {
        self.combat_secondary_marker.and_then(|(x, y)| {
            (usize::from(x) < COMBAT_ARENA_SIDE && usize::from(y) < COMBAT_ARENA_SIDE)
                .then_some((x, y))
        })
    }

    pub fn combat_render_sprite_at(&self, x: usize, y: usize) -> Option<usize> {
        self.active_objects
            .iter()
            .enumerate()
            .find_map(|(slot, object)| {
                if object.is_empty() || object.x != x || object.y != y {
                    return None;
                }
                let linked_actor = self.combat_actors.iter().copied().find(|actor| {
                    !actor.is_empty() && usize::from(actor.active_object_slot) == slot
                });
                if linked_actor.is_some_and(CombatActorDescriptor::is_hidden_or_unrevealed) {
                    return None;
                }
                Some(if object.tile == PLAYER_TILE {
                    PLAYER_SPRITE_TILE
                } else {
                    object.tile as usize
                })
            })
    }

    pub fn render_dungeon_viewport(
        &self,
        radius: usize,
        depth: TileGraphicsDepth,
        billboards: Option<&DungeonBillboardBanks>,
    ) -> io::Result<TileViewport> {
        // First-person corridor for the current dungeon view, drawn from
        // the flavour's billboard bank per `dungeon-mode.md` sections
        // 6.1-6.5. An earlier comment here promised parity with a "DOS
        // sparse point-table"; that reading of the renderer is withdrawn
        // - the corridor is bitmap blits, not a synthesised point cloud.
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
            depth,
            cells_wide: cells,
            cells_high: cells,
            width,
            height,
            pixels: vec![0; pixel_count],
        };

        let Area::Dungeon { level, .. } = self.area else {
            return Ok(viewport);
        };
        if !self.has_personal_light() {
            return Ok(viewport);
        }

        self.draw_dungeon_corridor(level, &mut viewport, billboards);
        Ok(viewport)
    }

    /// `dungeon-mode.md` sections 6.1-6.5: paint the corridor from the
    /// flavour's billboard bank.
    ///
    /// Two sweeps. The forward sweep runs band 0 (the party's own cell)
    /// through band 3, running the forward test at each band and then
    /// painting the two side cells, stopping at the first band whose
    /// forward test reports blocked. Every image is drawn twice, once
    /// at `96 - hw[b]` and once mirrored at `192 - x_left - width`, so
    /// the two halves of a forward billboard meet exactly on the centre
    /// line. There is no projection arithmetic and no depth buffer.
    ///
    /// Object sprites, fountain water and energy-field strobes - the
    /// backward sweep of section 6.5 and sections 6.6-6.8 - are not
    /// drawn yet: they come from a separate art file this engine does
    /// not load. Nothing invented stands in for them.
    fn draw_dungeon_corridor(
        &self,
        level: u8,
        viewport: &mut TileViewport,
        billboards: Option<&DungeonBillboardBanks>,
    ) {
        let Area::Dungeon { scene, .. } = self.area else {
            return;
        };
        let Some(banks) = billboards else {
            return;
        };
        let bank = banks.bank(scene.presentation_flavour());

        let (fdx, fdy) = self.player.facing.delta();
        let Some(left_facing) = self.player.facing.turn_left_cardinal() else {
            return;
        };
        let Some(right_facing) = self.player.facing.turn_right_cardinal() else {
            return;
        };
        let (ldx, ldy) = left_facing.delta();
        let (rdx, rdy) = right_facing.delta();

        let mut point_blank = false;
        for band in 0..DUNGEON_BANDS {
            let step = band as isize;
            let ahead_dx = fdx * step;
            let ahead_dy = fdy * step;
            let ahead = self.dungeon_renderer_offset_cell(level, ahead_dx, ahead_dy);
            let outcome = dungeon_forward_outcome(ahead, band);
            if outcome.point_blank {
                point_blank = true;
            }

            if let Some(role) = outcome.blocker {
                self.draw_dungeon_billboard(viewport, bank, role, band);
            }

            // A point-blank door suppresses the band-0 side cells so the
            // frame is not boxed in.
            if !(band == 0 && point_blank) {
                let left_cell =
                    self.dungeon_renderer_offset_cell(level, ahead_dx + ldx, ahead_dy + ldy);
                let right_cell =
                    self.dungeon_renderer_offset_cell(level, ahead_dx + rdx, ahead_dy + rdy);
                self.draw_dungeon_billboard(viewport, bank, dungeon_side_role(left_cell), band);
                self.draw_dungeon_billboard(viewport, bank, dungeon_side_role(right_cell), band);
            }

            if !outcome.see_through {
                break;
            }
        }
    }

    /// Blit one billboard and its mirrored copy into the viewport.
    ///
    /// The viewport is the 176x176 tile window, which the frame places
    /// at screen `(8, 8)`; the published placements are in screen
    /// pixels, so both axes shift by that origin.
    fn draw_dungeon_billboard(
        &self,
        viewport: &mut TileViewport,
        bank: &DungeonBillboardBank,
        role: DungeonBillboardRole,
        band: usize,
    ) {
        let Some(slot) = role.slot(band) else {
            return;
        };
        let Some(Some(image)) = bank.images.get(slot) else {
            return;
        };
        let left_x = dungeon_billboard_left_x(band);
        let width = image.width as i32;
        let right_x = dungeon_billboard_right_x(left_x, width);
        blit_dungeon_billboard(viewport, image, left_x, false);
        blit_dungeon_billboard(viewport, image, right_x, true);
    }

    fn dungeon_renderer_offset_cell(&self, level: u8, dx: isize, dy: isize) -> u8 {
        let x = dungeon_floor_wrap_coord(self.player.x as i16 + dx as i16) as usize;
        let y = dungeon_floor_wrap_coord(self.player.y as i16 + dy as i16) as usize;
        dungeon_renderer_cell_byte(self.dungeon_cell(level, x, y))
    }

    pub fn refresh_top_down_visibility_buffers(&mut self, area: TopDownRenderArea, radius: usize) {
        if radius != VIEWPORT_PLAYER_ROW {
            return;
        }
        let scene_byte = self.current_scene_byte();
        let path = if self.visibility_buffers_ready {
            world_tick_path(scene_byte, self.visibility_dirty)
        } else {
            WorldTickPath::ProducerFullRebuild
        };
        match path {
            WorldTickPath::CombatBlatCopy => self.copy_combat_terrain_to_visibility_buffers(),
            WorldTickPath::ProducerFullRebuild => {
                self.rebuild_top_down_visibility_buffers(area, radius);
                self.visibility_dirty = false;
                self.visibility_buffers_ready = true;
            }
            WorldTickPath::LazyRefill => {
                self.lazy_refill_top_down_visibility_buffers(area, radius);
            }
        }
    }

    fn reset_visibility_buffer_active_cells(&mut self) {
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                let grid_index = visibility_grid_active_index(row, col).unwrap();
                let terrain_index = terrain_band_active_index(row, col).unwrap();
                self.visibility_grid[grid_index] = VISIBILITY_HIDDEN;
                self.terrain_band[terrain_index] = 0;
            }
        }
    }

    fn rebuild_top_down_visibility_buffers(&mut self, area: TopDownRenderArea, radius: usize) {
        self.reset_visibility_buffer_active_cells();
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = radius as isize;
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                let world_x = px + col as isize - r;
                let world_y = py + row as isize - r;
                let Some((terrain, _)) =
                    self.top_down_render_cell_base(area, px, py, world_x, world_y, radius)
                else {
                    continue;
                };
                let grid_index = visibility_grid_active_index(row, col).unwrap();
                let terrain_index = terrain_band_active_index(row, col).unwrap();
                self.terrain_band[terrain_index] = terrain;
                self.visibility_grid[grid_index] = visibility_marker_for_viewport_cell(col, row);
            }
        }
        self.composite_active_objects_into_visibility_buffers(area, radius);
    }

    fn lazy_refill_top_down_visibility_buffers(&mut self, area: TopDownRenderArea, radius: usize) {
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = radius as isize;
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                let grid_index = visibility_grid_active_index(row, col).unwrap();
                if !visibility_cheap_path_needs_refill(self.visibility_grid[grid_index]) {
                    continue;
                }
                let world_x = px + col as isize - r;
                let world_y = py + row as isize - r;
                let Some((terrain, _)) =
                    self.top_down_render_cell_base(area, px, py, world_x, world_y, radius)
                else {
                    continue;
                };
                let terrain_index = terrain_band_active_index(row, col).unwrap();
                self.terrain_band[terrain_index] = terrain;
            }
        }
        self.composite_active_objects_into_visibility_buffers(area, radius);
    }

    fn copy_combat_terrain_to_visibility_buffers(&mut self) {
        self.reset_visibility_buffer_active_cells();
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                let grid_index = visibility_grid_active_index(row, col).unwrap();
                let terrain_index = terrain_band_active_index(row, col).unwrap();
                self.terrain_band[terrain_index] = self
                    .animation
                    .resolve_static_tile(self.combat_terrain[row][col]);
                self.visibility_grid[grid_index] = visibility_marker_for_viewport_cell(col, row);
            }
        }
        self.visibility_dirty = false;
        self.visibility_buffers_ready = true;
    }

    fn prepared_top_down_grid_from_visibility_buffers(&self) -> Vec<Option<PreparedTopDownCell>> {
        let mut prepared = vec![None; VIEWPORT_SIDE * VIEWPORT_SIDE];
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                let grid = self.visibility_grid[visibility_grid_active_index(row, col).unwrap()];
                if grid == VISIBILITY_HIDDEN {
                    continue;
                }
                let terrain = self.terrain_band[terrain_band_active_index(row, col).unwrap()];
                prepared[row * VIEWPORT_SIDE + col] = Some(PreparedTopDownCell { terrain, grid });
            }
        }
        prepared
    }

    fn composite_active_objects_into_visibility_buffers(
        &mut self,
        area: TopDownRenderArea,
        radius: usize,
    ) {
        for slot in (1..self.active_objects.len()).rev() {
            let object = self.active_objects[slot];
            self.composite_top_down_object_into_visibility_buffers(area, radius, object);
        }
        if let Some(z) = self.current_floor() {
            let player = ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: self.player.x,
                y: self.player.y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
            self.composite_top_down_object_into_visibility_buffers(area, radius, player);
        }
    }

    fn composite_top_down_object_into_visibility_buffers(
        &mut self,
        area: TopDownRenderArea,
        radius: usize,
        object: ActiveObject,
    ) {
        if object.is_empty() || object.is_player_phantom() {
            return;
        }
        let Some((col, row)) = self.top_down_object_viewport_cell(area, radius, object) else {
            return;
        };
        if col >= VIEWPORT_SIDE || row >= VIEWPORT_SIDE {
            return;
        }
        let grid_index = visibility_grid_active_index(row, col).unwrap();
        let terrain_index = terrain_band_active_index(row, col).unwrap();
        let current_grid_byte = self.visibility_grid[grid_index];
        let current_terrain = self.terrain_band[terrain_index];
        let previous_row_terrain = (row > 0).then(|| {
            let index = terrain_band_active_index(row - 1, col).unwrap();
            self.terrain_band[index]
        });
        let next_row_terrain = (row + 1 < VIEWPORT_SIDE).then(|| {
            let index = terrain_band_active_index(row + 1, col).unwrap();
            self.terrain_band[index]
        });
        let variant = self.active_object_render_variant(col, row, object);
        match active_object_composite(
            object.type_byte,
            object.tile,
            current_grid_byte,
            current_terrain,
            previous_row_terrain,
            next_row_terrain,
            row,
            variant,
        ) {
            ActiveObjectCompositeResult::Suppress => {}
            ActiveObjectCompositeResult::Companion(tile) => {
                self.terrain_band[terrain_index] = tile;
                self.visibility_grid[grid_index] = VISIBILITY_USE_COMPANION;
            }
            ActiveObjectCompositeResult::Direct(tile) => {
                self.visibility_grid[grid_index] = tile;
            }
            ActiveObjectCompositeResult::PreviousRowDirectAndCompanion {
                previous_marker,
                tile,
            } => {
                if row > 0 {
                    let previous_grid_index = visibility_grid_active_index(row - 1, col).unwrap();
                    self.visibility_grid[previous_grid_index] = previous_marker;
                }
                self.terrain_band[terrain_index] = tile;
                self.visibility_grid[grid_index] = VISIBILITY_USE_COMPANION;
            }
        }
    }

    fn prepare_top_down_render_grid(
        &self,
        area: TopDownRenderArea,
        radius: usize,
    ) -> Vec<Option<PreparedTopDownCell>> {
        let cells = radius.saturating_mul(2).saturating_add(1);
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = radius as isize;
        let mut prepared = vec![None; cells.saturating_mul(cells)];

        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let world_x = px + cell_x as isize - r;
                let world_y = py + cell_y as isize - r;
                let Some((terrain, _)) =
                    self.top_down_render_cell_base(area, px, py, world_x, world_y, radius)
                else {
                    continue;
                };
                prepared[cell_y * cells + cell_x] = Some(PreparedTopDownCell {
                    terrain,
                    grid: terrain,
                });
            }
        }

        for slot in (1..self.active_objects.len()).rev() {
            let object = self.active_objects[slot];
            self.composite_top_down_object(area, radius, object, &mut prepared);
        }
        if let Some(z) = self.current_floor() {
            let player = ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: self.player.x,
                y: self.player.y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
            self.composite_top_down_object(area, radius, player, &mut prepared);
        }

        prepared
    }

    fn top_down_render_cell_base(
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
                if self.surface_visibility_pitch_dark() {
                    return None;
                }
                let light_threshold = self.surface_visibility_light_threshold();
                if !self.town_cell_visible_with_light_threshold(
                    px,
                    py,
                    x,
                    y,
                    radius,
                    light_threshold,
                ) {
                    return None;
                }
                let xu = x as usize;
                let yu = y as usize;
                let terrain = self.animation.resolve_static_tile(self.grid[yu * 32 + xu]);
                Some((terrain, None))
            }
            TopDownRenderArea::World(_) => {
                if self.world_visibility_pitch_dark() {
                    return None;
                }
                let light_threshold = self.world_visibility_light_threshold();
                if !self.world_cell_visible_with_light_threshold(
                    px,
                    py,
                    x,
                    y,
                    radius,
                    light_threshold,
                ) {
                    return None;
                }
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                let terrain = self
                    .animation
                    .resolve_static_tile(self.world_live_tile_at(wx, wy));
                Some((terrain, None))
            }
        }
    }

    fn composite_top_down_object(
        &self,
        area: TopDownRenderArea,
        radius: usize,
        object: ActiveObject,
        prepared: &mut [Option<PreparedTopDownCell>],
    ) {
        if object.is_empty() || object.is_player_phantom() {
            return;
        }
        let Some((cell_x, cell_y)) = self.top_down_object_viewport_cell(area, radius, object)
        else {
            return;
        };
        let cells = radius.saturating_mul(2).saturating_add(1);
        let index = cell_y * cells + cell_x;
        let Some(cell) = prepared.get(index).and_then(|cell| *cell) else {
            return;
        };
        let previous_row_terrain = (cell_y > 0)
            .then(|| prepared[index - cells])
            .flatten()
            .map(|cell| cell.terrain);
        let next_row_terrain = (cell_y + 1 < cells)
            .then(|| prepared[index + cells])
            .flatten()
            .map(|cell| cell.terrain);
        let variant = self.active_object_render_variant(cell_x, cell_y, object);
        match active_object_composite(
            object.type_byte,
            object.tile,
            cell.grid,
            cell.terrain,
            previous_row_terrain,
            next_row_terrain,
            cell_y,
            variant,
        ) {
            ActiveObjectCompositeResult::Suppress => {}
            ActiveObjectCompositeResult::Companion(tile) => {
                prepared[index] = Some(PreparedTopDownCell {
                    terrain: tile,
                    grid: VISIBILITY_USE_COMPANION,
                });
            }
            ActiveObjectCompositeResult::Direct(tile) => {
                prepared[index] = Some(PreparedTopDownCell {
                    terrain: cell.terrain,
                    grid: tile,
                });
            }
            ActiveObjectCompositeResult::PreviousRowDirectAndCompanion {
                previous_marker,
                tile,
            } => {
                if cell_y > 0 {
                    if let Some(previous) = prepared[index - cells].as_mut() {
                        previous.grid = previous_marker;
                    }
                }
                prepared[index] = Some(PreparedTopDownCell {
                    terrain: tile,
                    grid: VISIBILITY_USE_COMPANION,
                });
            }
        }
    }

    fn top_down_object_viewport_cell(
        &self,
        area: TopDownRenderArea,
        radius: usize,
        object: ActiveObject,
    ) -> Option<(usize, usize)> {
        if object.z != self.current_floor()? {
            return None;
        }
        let r = radius as isize;
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
        if !(-r..=r).contains(&dx) || !(-r..=r).contains(&dy) {
            return None;
        }
        Some(((dx + r) as usize, (dy + r) as usize))
    }

    fn active_object_render_variant(
        &self,
        cell_x: usize,
        cell_y: usize,
        object: ActiveObject,
    ) -> u8 {
        let active_character_is_tinker = self
            .active_player
            .and_then(|index| self.party.get(index))
            .is_some_and(|member| member.class_byte == b'T');
        let selector = (self.turn as u8)
            ^ self.animation.frame
            ^ (cell_x as u8).wrapping_mul(3)
            ^ (cell_y as u8).wrapping_mul(5)
            ^ object.type_byte
            ^ object.tile;
        active_object_compositor_variant(active_character_is_tinker, selector)
    }

    /// Visibility + composited terrain/sprite lookup for one cell.
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
        let (terrain, _) = self.top_down_render_cell_base(area, px, py, x, y, radius)?;
        let r = radius as isize;
        let cell_x = usize::try_from(x - px + r).ok()?;
        let cell_y = usize::try_from(y - py + r).ok()?;
        let cells = radius.saturating_mul(2).saturating_add(1);
        if cell_x >= cells || cell_y >= cells {
            return Some((terrain, None));
        }
        let prepared = self.prepare_top_down_render_grid(area, radius);
        let cell = prepared[cell_y * cells + cell_x]?;
        let tile = cell.tile();
        let sprite = (tile != terrain).then_some(tile);
        Some((terrain, sprite))
    }

    pub fn top_down_render_area(&self) -> Option<TopDownRenderArea> {
        match self.area {
            Area::Town { .. } => Some(TopDownRenderArea::Town),
            Area::World { plane } => Some(TopDownRenderArea::World(plane)),
            Area::Dungeon { .. } => None,
        }
    }

    pub fn viewport_has_animated_tiles(&self, radius: usize) -> bool {
        if self.visibility_dirty
            || self.white_potion_sweep.is_some()
            || self
                .combat_potion_presentation
                .is_some_and(|presentation| presentation.kind == CombatPotionPresentationKind::Poof)
        {
            return true;
        }
        if self.combat_active {
            return self
                .combat_terrain
                .iter()
                .flatten()
                .copied()
                .any(|tile| static_tile_animation_family(tile).is_some());
        }

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
                        self.world_live_tile_at(wx, wy)
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
                let cells = radius.saturating_mul(2).saturating_add(1);
                let prepared = self.prepare_top_down_render_grid(TopDownRenderArea::Town, radius);
                for y in 0..cells {
                    for x in 0..cells {
                        let Some(cell) = prepared[y * cells + x] else {
                            out.push(' ');
                            continue;
                        };
                        let tile = cell.tile();
                        if tile == PLAYER_TILE {
                            out.push('@');
                        } else {
                            out.push(render_glyph(tile));
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
                    dungeon_display_level(level),
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
                let cells = radius.saturating_mul(2).saturating_add(1);
                let prepared =
                    self.prepare_top_down_render_grid(TopDownRenderArea::World(plane), radius);
                for y in 0..cells {
                    for x in 0..cells {
                        let Some(cell) = prepared[y * cells + x] else {
                            out.push(' ');
                            continue;
                        };
                        let tile = cell.tile();
                        if tile == PLAYER_TILE {
                            out.push('@');
                        } else {
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

    /// `visibility.md §5` + `lighting.md §3`: the cached ambient-light
    /// byte *is* the squared-distance threshold handed to the visibility
    /// carve. It is not a linear cell radius that the carve then squares.
    ///
    /// The spec is double-voiced here: §3 calls the byte the player's
    /// "current effective sight radius", while §5 settles the external
    /// contract as "the caller-provided light value is a squared-distance
    /// threshold: cells whose squared distance from the centre is less
    /// than or equal to that value are inside the main light radius"
    /// (wording question raised alongside `cleak/u5-spec#79`). Black-box
    /// observation of the original decides it: at noon on Britannia all
    /// four corners of the 11x11 viewport are lit, and a corner sits at
    /// squared distance 50 — exactly [`FULL_DAYLIGHT`]. Read as a linear
    /// radius, 50 would be absurd; read as a threshold it lands on the
    /// corner exactly.
    ///
    /// `lighting.md §3` keeps ambient on 2..=50 (dawn/dusk levels
    /// `2, 5, 10, 20, 34, 49`; torch floor 18, spell floor 10), so the
    /// same byte read as a threshold gives the full viewport at 50, a
    /// torch disc reaching squared distance 18 at night, and the bare 3x3
    /// neighbourhood at full darkness. Values above [`FULL_DAYLIGHT`] are
    /// the skip-recompute sentinels; clamp them so a stale sentinel cannot
    /// widen the disc past daylight.
    pub fn surface_visibility_light_threshold(&self) -> u32 {
        u32::from(self.ambient_light.min(FULL_DAYLIGHT))
    }

    /// `visibility.md §3`/`§4`: light radius zero is the pitch-dark
    /// branch. The producer "skips both the visibility carve helper and
    /// the full-fill path" and "the grid stays fully obscured" — the
    /// player sees nothing at all, not even the cell underfoot. Routed
    /// through [`light_radius_branch`] so the three signed cases stay in
    /// one place.
    pub fn surface_visibility_pitch_dark(&self) -> bool {
        matches!(
            light_radius_branch(self.ambient_light),
            LightRadiusBranch::PitchDark
        )
    }

    /// `lighting.md §3`: the overworld special-underfoot override forces
    /// ambient light to zero, i.e. the pitch-dark branch.
    pub fn world_visibility_light_threshold(&self) -> u32 {
        if self.world_underfoot_blackout_active() {
            return 0;
        }

        self.surface_visibility_light_threshold()
    }

    pub fn world_visibility_pitch_dark(&self) -> bool {
        self.world_underfoot_blackout_active() || self.surface_visibility_pitch_dark()
    }

    pub fn world_underfoot_blackout_active(&self) -> bool {
        if !matches!(self.area, Area::World { .. }) {
            return false;
        }
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        overworld_underfoot_forces_dark(tile, self.timing_status.save_byte())
    }

    pub fn refresh_world_underfoot_blackout_latch(&mut self) -> bool {
        if self.world_underfoot_blackout_active() {
            if !self.world_underfoot_blackout_latched || self.ambient_light != 0 {
                self.mark_visibility_dirty();
            }
            self.world_underfoot_blackout_latched = true;
            self.ambient_light = 0;
            return true;
        }

        if self.world_underfoot_blackout_latched {
            self.world_underfoot_blackout_latched = false;
            self.mode_zero_cleanup();
        }
        false
    }

    /// Convenience wrapper: carve the whole `visible_radius` disc, i.e.
    /// use the squared radius as the `visibility.md §5` light threshold.
    pub fn town_cell_visible(
        &self,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        visible_radius: usize,
    ) -> bool {
        let threshold = (visible_radius as u32).saturating_mul(visible_radius as u32);
        self.town_cell_visible_with_light_threshold(px, py, x, y, visible_radius, threshold)
    }

    pub fn town_cell_visible_with_light_threshold(
        &self,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        view_radius: usize,
        light_threshold: u32,
    ) -> bool {
        let Some(index) = visibility_view_index(px, py, x, y, view_radius) else {
            return false;
        };
        self.surface_visibility_carve_with_light_threshold(
            px,
            py,
            view_radius,
            light_threshold,
            false,
        )[index]
    }

    /// `visibility.md §6`: sight blocking is its own tile classifier, not
    /// the movement/projectile one. Uses the spec propagation-blocker set.
    pub fn town_cell_blocks_sight(&self, x: usize, y: usize) -> bool {
        self.sight_blocking_object_at_current_floor(x, y).is_some()
            || tile_blocks_sight_propagation(self.grid[y * 32 + x])
    }

    /// Convenience wrapper: carve the whole `visible_radius` disc, i.e.
    /// use the squared radius as the `visibility.md §5` light threshold.
    pub fn world_cell_visible(
        &self,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        visible_radius: usize,
    ) -> bool {
        let threshold = (visible_radius as u32).saturating_mul(visible_radius as u32);
        self.world_cell_visible_with_light_threshold(px, py, x, y, visible_radius, threshold)
    }

    pub fn world_cell_visible_with_light_threshold(
        &self,
        px: isize,
        py: isize,
        x: isize,
        y: isize,
        view_radius: usize,
        light_threshold: u32,
    ) -> bool {
        let Some(index) = visibility_view_index(px, py, x, y, view_radius) else {
            return false;
        };
        self.surface_visibility_carve_with_light_threshold(
            px,
            py,
            view_radius,
            light_threshold,
            true,
        )[index]
    }

    pub fn world_cell_blocks_sight(&self, x: usize, y: usize) -> bool {
        self.sight_blocking_object_at_current_floor(x, y).is_some()
            || world_surface_tile_blocks_sight(self.world_live_tile_at(x, y))
    }

    pub fn surface_visibility_carve(
        &self,
        px: isize,
        py: isize,
        radius: usize,
        wrap_world: bool,
    ) -> Vec<bool> {
        let threshold = (radius as u32).saturating_mul(radius as u32);
        self.surface_visibility_carve_with_light_threshold(px, py, radius, threshold, wrap_world)
    }

    /// `visibility.md §5`: `light_threshold` is the squared-distance
    /// threshold, taken as supplied. Do not square it here — see
    /// [`Self::surface_visibility_light_threshold`].
    pub fn surface_visibility_carve_with_light_threshold(
        &self,
        px: isize,
        py: isize,
        view_radius: usize,
        light_threshold: u32,
        wrap_world: bool,
    ) -> Vec<bool> {
        let side = view_radius.saturating_mul(2).saturating_add(1);
        let cell_count = side.saturating_mul(side);
        let mut visible = vec![false; cell_count];
        if cell_count == 0 {
            return visible;
        }

        let mask = self.surface_local_light_mask(px, py, wrap_world);
        let (mask_origin_x, mask_origin_y) = surface_local_light_mask_origin(px, py, wrap_world);
        let lit = |x: isize, y: isize| {
            Self::surface_local_light_mask_is_lit(
                &mask,
                mask_origin_x,
                mask_origin_y,
                x,
                y,
                wrap_world,
            )
        };

        self.surface_centre_out_carve(
            px,
            py,
            wrap_world,
            |x, y| visibility_view_index(px, py, x, y, view_radius),
            |x, y| visibility_squared_distance(px, py, x, y),
            |candidate| {
                let propagates =
                    surface_tile_propagates_visibility(candidate.tile, candidate.squared_distance);

                // Inside the threshold: painted unconditionally. Opacity
                // governs propagation *past* a cell, never visibility of
                // the cell itself (`visibility.md §3`/`§5`).
                if visibility_in_radius(candidate.squared_distance, light_threshold) {
                    return SurfaceCarveVerdict {
                        paint: true,
                        expand: propagates,
                    };
                }

                // Beyond the threshold the influence mask decides, with
                // two different rules (`visibility.md §5`, published
                // through `cleak/u5-spec#83`). Cells out here are NOT
                // automatically dark; without this an engine blacks out
                // lit rooms and lamp-lit streets at night.
                if propagates {
                    // Sight-transparent: shown if its own mask coverage is
                    // nonzero, and enqueued *either way*, so the flood can
                    // cross unlit ground and reach lit ground further out.
                    SurfaceCarveVerdict {
                        paint: lit(candidate.x, candidate.y),
                        expand: true,
                    }
                } else {
                    // Sight-blocking: shown only if the cell the carve
                    // arrived from was visible and both it and the
                    // candidate have mask coverage. Never expands.
                    SurfaceCarveVerdict {
                        paint: candidate.parent_carved
                            && lit(candidate.parent_x, candidate.parent_y)
                            && lit(candidate.x, candidate.y),
                        expand: false,
                    }
                }
            },
            &mut visible,
        );

        visible
    }

    /// `visibility.md §5`: the shared centre-out neighbour carve. Seeds a
    /// work queue with the centre cell, marks it, then repeatedly pops a
    /// coordinate and examines its eight neighbours in the fixed ring
    /// order (west, southwest, south, southeast, east, northeast, north,
    /// northwest). Candidates that are out of the caller's window, already
    /// considered, or refused by `classify` are dropped; the rest are
    /// painted and/or enqueued exactly as `classify` directs.
    ///
    /// The same helper serves the player's viewport carve (§5) and the
    /// local-light mask's per-source carve (§12) — the spec says the mask
    /// "runs the same centre-out visibility carve ... using the source as
    /// the centre and a fixed source radius", and §5 explicitly forbids
    /// implementing either as a line or shadow caster.
    ///
    /// `index_of` maps a world coordinate to a slot in `carved` (`None`
    /// for out-of-window), `squared_distance` supplies the centre-relative
    /// squared distance used by both the range tests and the
    /// adjacent-only propagation rule, and `classify` returns the paint
    /// and expand decisions. Painting and expansion are *independent*:
    /// the player carve paints a lit blocker without expanding through
    /// it, and enqueues an unlit sight-transparent cell without painting
    /// it.
    fn surface_centre_out_carve<FIndex, FDist, FClassify>(
        &self,
        center_x: isize,
        center_y: isize,
        wrap_world: bool,
        index_of: FIndex,
        squared_distance: FDist,
        classify: FClassify,
        carved: &mut [bool],
    ) where
        FIndex: Fn(isize, isize) -> Option<usize>,
        FDist: Fn(isize, isize) -> u32,
        FClassify: Fn(SurfaceCarveCandidate) -> SurfaceCarveVerdict,
    {
        let Some(center) = index_of(center_x, center_y) else {
            return;
        };
        if center >= carved.len() {
            return;
        }
        // `lighting.md §7.1`: the centre is seeded into the visible set
        // unconditionally, before any distance comparison.
        carved[center] = true;

        let mut considered = vec![false; carved.len()];
        considered[center] = true;
        let mut queue = std::collections::VecDeque::from([(center_x, center_y)]);

        while let Some((cx, cy)) = queue.pop_front() {
            let parent_carved = index_of(cx, cy).is_some_and(|index| carved[index]);
            for (dx, dy) in VISIBILITY_CARVE_NEIGHBOR_ORDER {
                let x = cx + isize::from(dx);
                let y = cy + isize::from(dy);
                let Some(index) = index_of(x, y) else {
                    continue;
                };
                if considered[index] {
                    continue;
                }
                considered[index] = true;

                let Some(tile) = self.surface_visibility_tile(x, y, wrap_world) else {
                    continue;
                };
                let verdict = classify(SurfaceCarveCandidate {
                    x,
                    y,
                    squared_distance: squared_distance(x, y),
                    tile,
                    parent_x: cx,
                    parent_y: cy,
                    parent_carved,
                });

                if verdict.paint {
                    carved[index] = true;
                }
                if verdict.expand {
                    queue.push_back((x, y));
                }
            }
        }
    }

    fn surface_local_light_mask(&self, px: isize, py: isize, wrap_world: bool) -> Vec<bool> {
        let mut mask = vec![false; TOWN_GRID_BYTES];
        let (origin_x, origin_y) = surface_local_light_mask_origin(px, py, wrap_world);

        for local_y in 0..TOWN_GRID_SIDE {
            for local_x in 0..TOWN_GRID_SIDE {
                let x = origin_x + local_x as isize;
                let y = origin_y + local_y as isize;
                if self
                    .surface_visibility_tile(x, y, wrap_world)
                    .is_some_and(is_local_light_source_tile)
                {
                    self.carve_surface_local_light_source(
                        x, y, origin_x, origin_y, wrap_world, &mut mask,
                    );
                }
            }
        }

        let Some(floor) = self.current_floor() else {
            return mask;
        };
        for object in &self.active_objects {
            if object.is_empty() || object.z != floor || !is_local_light_source_tile(object.tile) {
                continue;
            }
            let x = object.x as isize;
            let y = object.y as isize;
            if surface_local_light_mask_index(origin_x, origin_y, x, y, wrap_world).is_some() {
                self.carve_surface_local_light_source(
                    x, y, origin_x, origin_y, wrap_world, &mut mask,
                );
            }
        }

        // `visibility.md §12.4`: "local-light refresh first, beacon stamps
        // second, visibility carve third". The rotating beacon of `§12.6` is
        // the mask's one other non-combat writer.
        self.stamp_light_beacon(&mut mask, origin_x, origin_y, wrap_world);

        mask
    }

    /// `visibility.md §12`/`§12.2`: one local-light source's contribution
    /// to the 32x32 mask, per the contract re-verified in
    /// `cleak/u5-spec#42`.
    ///
    /// The refresh pass runs "the same queue-based centre-out neighbour
    /// carve the ordinary producer uses, seeded at the source", so this
    /// reuses [`Self::surface_centre_out_carve`]; §5 rules out line
    /// casting for both carves, and light therefore reaches around an
    /// L-shaped wall when an eight-neighbour path is open.
    ///
    /// Range is the squared-distance disc
    /// [`LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD`], *not* a Chebyshev square
    /// — see that constant. Blockers inside the disc are themselves lit
    /// and only stop the expansion past them, which
    /// [`Self::surface_centre_out_carve`] already does. Outside the disc
    /// the local-light flood stops dead: an out-of-range candidate is
    /// neither painted nor expanded (unlike the producer's carve, which
    /// keeps expanding through dark space and consults this mask). The
    /// per-source visited set is local to the helper, so overlapping
    /// sources union into the shared mask without shadowing each other.
    ///
    /// KNOWN GAP (`cleak/u5-spec#42`, cadence): the original rebuilds this
    /// mask on exactly three triggers — the Moonstone live-gate terrain
    /// refresh, combat entry, and combat exit — and the mask *persists*
    /// between them while the producer keeps reading it. This engine
    /// instead rebuilds it on demand during each carve, which is always
    /// at least as fresh but will not reproduce a stale-mask frame. Making
    /// that exact needs mask caching plus the three invalidation hooks.
    fn carve_surface_local_light_source(
        &self,
        source_x: isize,
        source_y: isize,
        origin_x: isize,
        origin_y: isize,
        wrap_world: bool,
        mask: &mut [bool],
    ) {
        self.surface_centre_out_carve(
            source_x,
            source_y,
            wrap_world,
            |x, y| surface_local_light_mask_index(origin_x, origin_y, x, y, wrap_world),
            |x, y| surface_local_light_squared_distance(source_x, source_y, x, y, wrap_world),
            |candidate| {
                let inside = candidate.squared_distance <= LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD;
                SurfaceCarveVerdict {
                    paint: inside,
                    expand: inside
                        && surface_tile_propagates_visibility(
                            candidate.tile,
                            candidate.squared_distance,
                        ),
                }
            },
            mask,
        );
    }

    fn surface_local_light_mask_is_lit(
        mask: &[bool],
        origin_x: isize,
        origin_y: isize,
        x: isize,
        y: isize,
        wrap_world: bool,
    ) -> bool {
        surface_local_light_mask_index(origin_x, origin_y, x, y, wrap_world)
            .and_then(|index| mask.get(index))
            .copied()
            .unwrap_or(false)
    }

    fn surface_visibility_tile(&self, x: isize, y: isize, wrap_world: bool) -> Option<u8> {
        if wrap_world {
            let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
            let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
            Some(self.world_live_tile_at(wx, wy))
        } else if (0..32).contains(&x) && (0..32).contains(&y) {
            Some(self.grid[y as usize * 32 + x as usize])
        } else {
            None
        }
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
        let turn_before = self.turn;
        let effective_minutes = if negate_time_active {
            0
        } else {
            self.timing_status.effective_minutes(minutes)
        };
        self.turn += 1;
        let previous_day = self.clock.day;
        let previous_hour = self.clock.hour;
        self.clock.advance_minutes(effective_minutes);
        if self.clock.day != previous_day {
            self.reroll_shadowlord_hideouts();
        }
        if previous_day == 28 && self.clock.day == 1 {
            self.fortunes_of_war = 0;
            age_stay_counters_month(&mut self.party_stay_counters);
            age_inn_registry_month(&mut self.inn_registry);
        }
        if self.clock.hour != previous_hour {
            self.refresh_cached_moon_glyphs();
            self.apply_hourly_status_provision_pass();
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
        // `visibility.md §12.6`: the beacon's cone advances one sixteenth of a
        // revolution per world turn, gated on the ambient value the line above
        // just recomputed.
        self.advance_light_beacon();
        self.refresh_natural_moongates();
        self.sync_player_object();
        if self.time_stop_counter != 0 {
            self.time_stop_counter = self.time_stop_counter.saturating_sub(1);
        } else if !negate_time_active && !self.combat_active {
            self.advance_npc_schedules();
            let world_object_epilogue_runs = !matches!(self.area, Area::World { .. })
                || self.timing_status.world_object_epilogue_runs(turn_before);
            if advance_active_objects && world_object_epilogue_runs {
                self.advance_active_objects();
            }
        }
        self.age_active_effect();
        if tick_doors && !self.combat_active {
            self.tick_door_tracker();
        }
        self.advance_animation_clock();
    }

    pub fn advance_presentation_frame(&mut self) {
        let mut needs_redraw = false;
        if let Some(mut sweep) = self.white_potion_sweep {
            if sweep.frames_remaining <= 1 {
                self.white_potion_sweep = None;
            } else {
                sweep.frames_remaining -= 1;
                self.white_potion_sweep = Some(sweep);
            }
            needs_redraw = true;
        }
        if let Some(mut presentation) = self.combat_potion_presentation {
            if presentation.kind != CombatPotionPresentationKind::Sleep
                && presentation.frames_remaining != u8::MAX
            {
                if presentation.frames_remaining <= 1 {
                    self.combat_potion_presentation = None;
                } else {
                    presentation.frames_remaining -= 1;
                    self.combat_potion_presentation = Some(presentation);
                }
                needs_redraw = true;
            }
        }
        if needs_redraw {
            self.mark_visibility_dirty();
        }
    }

    pub fn hourly_provision_consumer_count(&self) -> u16 {
        self.party
            .iter()
            .filter(|member| !matches!(member.status, b'D' | b'A' | b'S'))
            .count() as u16
    }

    pub fn apply_hourly_status_provision_pass(&mut self) -> u16 {
        let consumers = self.hourly_provision_consumer_count();
        self.apply_hourly_poison_tick();
        if self.food == 0 {
            self.pending_hourly_status_message = self.apply_hourly_starvation_tick();
        } else if is_provision_decrement_hour(self.clock.hour) {
            self.food = self.food.saturating_sub(consumers);
        }
        self.apply_hourly_ring_regeneration_tick();
        consumers
    }

    pub fn apply_hourly_ring_regeneration_tick(&mut self) -> u16 {
        if self.combat_active {
            return 0;
        }
        let mut healed = 0;
        if self.party_equipment.len() < self.party.len() {
            self.party_equipment
                .resize(self.party.len(), [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
        }
        for index in 0..self.party.len() {
            let member = &mut self.party[index];
            if member.status == b'D'
                || !member.living()
                || self.party_equipment[index][EQUIP_SLOT_RING]
                    != EQUIPMENT_ID_RING_REGENERATION as u8
            {
                continue;
            }
            if u5_prng_range_u16(&mut self.prng_state, 0, 7) == 0 {
                healed += member.heal_by(1);
            }
        }
        healed
    }

    pub fn apply_hourly_poison_tick(&mut self) -> u16 {
        let mut damaged = 0;
        for member in &mut self.party {
            if member.status != b'P' || !member.living() {
                continue;
            }
            member.apply_damage(FIRST_PLAYABLE_HOURLY_POISON_DAMAGE);
            damaged += 1;
        }
        damaged
    }

    pub fn apply_hourly_starvation_tick(&mut self) -> Option<String> {
        let mut reports = Vec::new();
        // `cleak/u5-spec#50`: per-slot starvation damage is the PRNG
        // roll `prng_range(1, 8)`. Roll independently for each
        // eligible slot in iteration order so the corrected spec's
        // "independent per slot" rule holds.
        for member in &mut self.party {
            if !member.living() {
                continue;
            }
            let slot = member.slot;
            let roll = u5_prng_range_u16(
                &mut self.prng_state,
                HOURLY_STARVATION_DAMAGE_MIN,
                HOURLY_STARVATION_DAMAGE_MAX,
            ) as u8;
            let applied = member.apply_damage(roll);
            reports.push(format!(
                "party slot {slot} took {applied} HP ({} HP left)",
                member.hp
            ));
        }

        if reports.is_empty() {
            Some("Starving! starvation damage skipped for 0 living member(s)".to_string())
        } else {
            Some(format!(
                "Starving! starvation damage: {}",
                reports.join("; ")
            ))
        }
    }

    pub fn append_pending_hourly_status_message(&mut self) {
        let Some(report) = self.pending_hourly_status_message.take() else {
            return;
        };
        if self.message.is_empty() {
            self.message = report;
        } else {
            self.message.push(' ');
            self.message.push_str(&report);
        }
    }

    pub fn mode_zero_cleanup(&mut self) {
        self.recompute_daylight();
        self.refresh_natural_moongates_for_current_counter();
    }

    pub fn refresh_natural_moongates(&mut self) -> bool {
        self.natural_moongate_counter =
            natural_moongate_advance_counter(self.natural_moongate_counter, self.clock.hour);
        self.refresh_natural_moongates_for_current_counter()
    }

    pub fn refresh_natural_moongates_for_current_counter(&mut self) -> bool {
        let Some(indices) = self.natural_moongate_slot_indices_for_current_scene() else {
            self.natural_moongate_live_cells.clear();
            return false;
        };
        let present = self.natural_moongate_counter != 0;
        let previous_live_cells = std::mem::take(&mut self.natural_moongate_live_cells);
        let mut changed = false;

        for idx in previous_live_cells {
            if !indices.contains(&idx) {
                if let Some(tile) = self.grid.get_mut(idx) {
                    if *tile == NATURAL_MOONGATE_LIVE_TILE {
                        *tile = NATURAL_MOONGATE_UNDERLYING_TILE;
                        changed = true;
                    }
                }
            }
        }

        for idx in indices {
            let target = if present {
                NATURAL_MOONGATE_LIVE_TILE
            } else {
                NATURAL_MOONGATE_UNDERLYING_TILE
            };
            if let Some(tile) = self.grid.get_mut(idx) {
                if *tile != target {
                    *tile = target;
                    changed = true;
                }
            }
            if present && !self.natural_moongate_live_cells.contains(&idx) {
                self.natural_moongate_live_cells.push(idx);
            }
        }

        if changed {
            self.mark_visibility_dirty();
            self.recompute_daylight();
        }
        changed
    }

    pub fn natural_moongate_chunk_window(&self) -> Option<(u8, u8, u8, u8)> {
        match self.area {
            Area::World { .. } => {
                let (x, y) = world_scroll_base(self.player.x, self.player.y);
                Some((
                    x as u8,
                    y as u8,
                    OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE as u8,
                    OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE as u8,
                ))
            }
            Area::Town { .. } => None,
            Area::Dungeon { .. } => None,
        }
    }

    pub fn natural_moongate_index_for_slot(&self, slot: MoonstoneGateSlot) -> Option<usize> {
        if !slot.is_valid() {
            return None;
        }
        match self.area {
            Area::World { plane } => {
                let window = self.natural_moongate_chunk_window();
                if natural_moongate_slot_eligible(
                    slot.scene,
                    slot.z,
                    slot.x,
                    slot.y,
                    0,
                    plane.save_floor() as u8,
                    window,
                ) {
                    Some(world_cell_index(slot.x as usize, slot.y as usize))
                } else {
                    None
                }
            }
            Area::Town { scene, floor } => {
                if natural_moongate_slot_eligible(
                    slot.scene,
                    slot.z,
                    slot.x,
                    slot.y,
                    scene.byte,
                    floor as u8,
                    None,
                ) && (slot.x as usize) < 32
                    && (slot.y as usize) < 32
                {
                    Some(slot.y as usize * 32 + slot.x as usize)
                } else {
                    None
                }
            }
            Area::Dungeon { .. } => None,
        }
    }

    pub fn restore_tracked_natural_moongates(&mut self) -> bool {
        let previous_live_cells = std::mem::take(&mut self.natural_moongate_live_cells);
        let mut changed = false;
        for idx in previous_live_cells {
            if let Some(tile) = self.grid.get_mut(idx) {
                if *tile == NATURAL_MOONGATE_LIVE_TILE {
                    *tile = NATURAL_MOONGATE_UNDERLYING_TILE;
                    changed = true;
                }
            }
        }
        if changed {
            self.mark_visibility_dirty();
            self.recompute_daylight();
        }
        changed
    }

    pub fn natural_moongate_night_window(&self) -> bool {
        matches!(
            natural_moongate_counter_step(self.clock.hour),
            NaturalMoongateCounterStep::Increase
        )
    }

    pub fn natural_moongate_slot_indices_for_current_scene(&self) -> Option<Vec<usize>> {
        match self.area {
            Area::World { .. } | Area::Town { .. } => {
                let mut indices = Vec::new();
                for slot in self.moonstone_slots.iter().copied() {
                    if let Some(idx) = self.natural_moongate_index_for_slot(slot) {
                        if !indices.contains(&idx) {
                            indices.push(idx);
                        }
                    }
                }
                Some(indices)
            }
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

    /// `active-objects.md §8.1`: "The overworld per-turn epilogue runs two
    /// passes over the table: the animate pass described above, and then a
    /// **separate prune pass**."
    ///
    /// The two passes are kept visibly separate here because they are separate
    /// mechanisms. The animate pass is per-mode. The prune pass is
    /// time-driven, positional, overworld-only, and runs unconditionally after
    /// the animate pass -- "Pruning is not animation, is not on the render
    /// tick, and is not driven by the animator." The dungeon loop runs
    /// neither. Neither pass returns a report: §8.1 forbids a pruning event
    /// that other systems observe.
    pub fn advance_active_objects(&mut self) {
        match self.area {
            Area::Dungeon { .. } => return,
            Area::World { .. } => self.advance_outdoor_active_objects(),
            Area::Town { .. } => {
                self.animate_active_objects();
                self.advance_town_free_roaming_active_objects();
            }
        }
        self.prune_far_overworld_objects();
    }

    /// `animation.md §6` global tile-animation step.
    ///
    /// It deliberately does **not** touch the natural-moongate
    /// gate-presence counter. Per `overworld.md §9.1` (spec HEAD
    /// c00bf63) that counter "is not a member of the global
    /// tile-animation families in `systems/animation.md` Section 6. It
    /// is not advanced by the animation tick, it has no frame selector,
    /// and skipping a rendered frame does not advance it." Only the
    /// once-per-turn refresh in `refresh_natural_moongates` moves it.
    pub fn advance_animation_clock(&mut self) {
        self.animation.tick_static_tiles();
    }

    pub fn animate_active_objects(&mut self) {
        for slot in 1..self.active_objects.len() {
            if self.active_objects[slot].is_empty()
                || self.active_objects[slot].is_player()
                || self.active_objects[slot].is_player_phantom()
                || (matches!(self.area, Area::Town { .. })
                    && town_free_roaming_object_eligible(self.active_objects[slot]))
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
                && !matches!(self.area, Area::Town { .. })
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
            // `active-objects.md §8`: the walker's **first phase** "handles
            // immediate hostile reactions", and only "[i]f none of those
            // immediate reactions fires" does "the cleanup phase decide
            // ordinary movement". A slot claimed by the first phase
            // therefore takes no wind drift and no directed/random step
            // this turn.
            //
            // Two of §8's four immediate reactions already run in
            // production, from the world post-turn epilogue rather than
            // from here: orthogonally adjacent hostile engagement
            // ([`Self::apply_world_active_object_engagement`]) and adjacent
            // whirlpool plane transition
            // ([`Self::apply_world_whirlpool_engagement`]). They are not
            // duplicated here. What this adds is the ranged half — the
            // breath attack and the broadside — which had no
            // implementation at all.
            let claimed_by_first_phase = self.outdoor_first_phase_ranged_attack(slot);
            let ship_wind =
                if !claimed_by_first_phase && (self.active_objects[slot].phase & 0x0f) == 0 {
                    self.try_drift_active_ship(slot, tick)
                } else {
                    ActiveShipWind::None
                };
            let ship_wind_changed = !matches!(ship_wind, ActiveShipWind::None);
            let wandered = !claimed_by_first_phase
                && !ship_wind_changed
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

    /// `active-objects.md §8` outdoor per-turn walker, first phase —
    /// **ranged half**. Returns `true` when the slot's immediate hostile
    /// reaction fired, which suppresses this turn's cleanup movement.
    ///
    /// §8 lists four immediate reactions. In walker order they are adjacent
    /// hostile engagement, the Sea Serpent / Dragon breath attack, adjacent
    /// whirlpool plane transition, and the ship-like water-creature /
    /// pirate broadside. `overworld.md §6` repeats the relative order of
    /// the ones it names: "adjacent engagement, Sea Serpent/Dragon
    /// first-frame near-range effects, aligned water-creature attack, and
    /// cleanup movement".
    ///
    /// Engagement and whirlpool are already implemented, on the world
    /// post-turn epilogue path; this method deliberately does not
    /// re-implement them. It covers the two ranged reactions, which share
    /// one routine per `overworld.md §6.2` — see
    /// [`crate::outdoor_ranged_attack`] for that contract.
    ///
    /// Every window here is measured on **wrapped** deltas. §8's own
    /// proximity helper "first computes wrapped absolute distance to the
    /// player", and the overworld is a 256-cell torus: raw subtraction
    /// would read a creature one cell across the map seam as ~255 cells
    /// away and silently disable both attacks near the seam.
    pub fn outdoor_first_phase_ranged_attack(&mut self, slot: usize) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let object = self.active_objects[slot];
        if object.z != plane.save_floor() || !is_outdoor_active_object_walker(object) {
            return false;
        }
        let (dx, dy) = wrapped_deltas_to_player(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        );

        // §8 bullet 2, and `overworld.md §6.2`'s first table row: "Sea
        // Serpent and Dragon first-frame hostile classes within three cells
        // of the player on **both** axes roll a one-in-eight trigger, and
        // on success loose a breath attack".
        //
        // Only the Dragon first frame is wired. `encounters.md`'s payload
        // table names `0xDC..0xDF` the "Dragon sprite run" and says of it
        // that "the first frame also participates in a special outdoor
        // near-range pull/effect path", which identifies `0xDC`
        // unambiguously. The Sea Serpent half is withheld: the same table
        // carries two candidate families, `0x88..0x8B` "Sea Serpent sprite
        // run" and `0xE0..0xE3` "Outdoor sea-serpent adjacency family", and
        // no spec text picks between them for this trigger. Guessing would
        // decide which creature breathes on the party.
        if object.type_byte == OUTDOOR_BREATH_ATTACKER_DRAGON_FIRST_FRAME
            && outdoor_breath_attack_in_range(dx, dy)
        {
            if outdoor_serpent_dragon_triggers(self.outdoor_serpent_dragon_breath_roll(slot)) {
                self.resolve_outdoor_ranged_attack(
                    object,
                    OutdoorRangedAttackFigure::SparkCloud,
                    None,
                );
                return true;
            }
        }

        // §8 bullet 4: "Ship-like water-creature and pirate frames aligned
        // with the player on the same row or column within three cells fire
        // a broadside: they print the boom message and then resolve the
        // same traced-line ranged attack as the breath attack above. The
        // generic 'attacked' message belongs to the adjacent-engagement
        // path, not to this one."
        if outdoor_broadside_attacker_class(object.type_byte)
            && outdoor_water_creature_attack_aligned(dx, dy)
        {
            self.resolve_outdoor_ranged_attack(
                object,
                OutdoorRangedAttackFigure::SolidBurst,
                Some(OUTDOOR_BROADSIDE_BOOM_MESSAGE),
            );
            return true;
        }

        false
    }

    /// `active-objects.md §8` one-in-eight breath-attack gate roll, reduced
    /// into `0..=7` by [`OUTDOOR_SERPENT_DRAGON_TRIGGER_DENOMINATOR`].
    /// Named so the gate can be exercised directly rather than inferred
    /// from whether a breath attack happened to fire.
    pub fn outdoor_serpent_dragon_breath_roll(&self, slot: usize) -> u8 {
        self.outdoor_active_object_step_seed(slot, OUTDOOR_SERPENT_DRAGON_BREATH_SALT)
            % OUTDOOR_SERPENT_DRAGON_TRIGGER_DENOMINATOR
    }

    /// `overworld.md §6.2` shared ranged-attack resolution, creature-to-
    /// party direction. Announces (the broadside's boom message precedes
    /// the shot; the breath attack has no announcement), traces the line,
    /// and applies the outcome.
    ///
    /// §6.2 on the announcement asymmetry: the breath row's announcement
    /// column is "None", the broadside row's is "A boom message before the
    /// shot", and "[t]he firing sound is played by the caller before the
    /// flight begins, not per cell during it".
    fn resolve_outdoor_ranged_attack(
        &mut self,
        attacker: ActiveObject,
        figure: OutdoorRangedAttackFigure,
        announcement: Option<&str>,
    ) {
        if let Some(announcement) = announcement {
            if self.message.is_empty() {
                self.message = announcement.to_string();
            } else {
                self.message.push(' ');
                self.message.push_str(announcement);
            }
        }

        let grid = &self.grid;
        let outcome = trace_outdoor_ranged_attack(
            (attacker.x as u8, attacker.y as u8),
            (self.player.x as u8, self.player.y as u8),
            |x, y| surface_tile_blocks_projectile(grid[world_cell_index(x as usize, y as usize)]),
        );

        match outcome {
            // §6.2: "If an obstruction is met first, the shot stops there
            // and nothing further happens."
            OutdoorRangedAttackOutcome::Obstructed { .. } => {}
            // §6.2: "If the line reaches the party with no intervening
            // blocker, the attack connects: the world tick runs and damage
            // is applied to the party at its map coordinates." The amount
            // is unspecified everywhere; the seam refuses rather than
            // inventing one.
            OutdoorRangedAttackOutcome::Connects => {
                require_outdoor_ranged_attack_damage(figure);
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

    pub fn advance_town_free_roaming_active_objects(&mut self) {
        for slot in 0..self.active_objects.len().min(OOL_SLOTS) {
            self.try_wander_town_active_object(slot);
        }
    }

    pub fn try_wander_town_active_object(&mut self, slot: usize) -> bool {
        let Area::Town { floor, .. } = self.area else {
            return false;
        };
        let object = self.active_objects[slot];
        if object.z != floor || !town_free_roaming_object_eligible(object) {
            return false;
        }

        if self.random_mod_u8(2) != 0 {
            return false;
        }
        if !self.town_free_roaming_pen_open(object.x, object.y) {
            return false;
        }

        let axis = self.random_mod_u8(2);
        let sign = self.random_mod_u8(2);
        let direction = town_free_roaming_direction(axis, sign);
        let (dx, dy) = direction.delta();
        let facing_byte = town_free_roaming_facing_byte(direction, object.type_byte);
        self.try_step_town_active_object(slot, object, dx, dy, facing_byte)
    }

    pub fn town_free_roaming_pen_open(&self, x: usize, y: usize) -> bool {
        for (dx, dy) in [(0isize, -1isize), (1, 0), (0, 1), (-1, 0)] {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0 || ny < 0 || nx >= TOWN_GRID_SIDE as isize || ny >= TOWN_GRID_SIDE as isize {
                return false;
            }
            let tile = self.grid[ny as usize * TOWN_GRID_SIDE + nx as usize];
            if town_free_roaming_pen_tile_blocks(tile) {
                return false;
            }
        }
        true
    }

    pub fn try_step_town_active_object(
        &mut self,
        slot: usize,
        object: ActiveObject,
        dx: isize,
        dy: isize,
        facing_byte: u8,
    ) -> bool {
        let nx = object.x as isize + dx;
        let ny = object.y as isize + dy;
        if nx < 0 || ny < 0 || nx >= TOWN_GRID_SIDE as isize || ny >= TOWN_GRID_SIDE as isize {
            return false;
        }
        let nx = nx as usize;
        let ny = ny as usize;
        if (nx, ny) == (self.player.x, self.player.y)
            || self
                .active_objects
                .iter()
                .enumerate()
                .any(|(other_slot, other)| {
                    other_slot != slot && self.object_occupies(*other, nx, ny)
                })
        {
            return false;
        }
        let tile = self.grid[ny * TOWN_GRID_SIDE + nx];
        if !town_active_object_step_accepts_tile(tile) {
            return false;
        }

        self.active_objects[slot].type_byte = facing_byte;
        self.active_objects[slot].tile = facing_byte;
        self.active_objects[slot].x = nx;
        self.active_objects[slot].y = ny;
        self.mark_visibility_dirty();
        true
    }

    pub fn town_active_object_slot_is_npc_link(&self, slot: usize) -> bool {
        self.npcs.iter().any(|npc| npc.active_object == Some(slot))
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
                let note = self.enter_terrain_combat_from_world_object(
                    game_dir,
                    plane,
                    object_slot,
                    object,
                )?;
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

    pub fn world_object_epilogue_runs_for_turn(&self, turn_before: u64) -> bool {
        self.timing_status.world_object_epilogue_runs(turn_before)
    }

    /// `active-objects.md §8.1` overworld off-screen prune pass. Invoked by
    /// the overworld per-turn epilogue "and by nothing else": not by the
    /// animator, not by the renderer, not by mode entry, not by the combat
    /// backup/restore path. Town, dungeon and combat loops do not run it.
    ///
    /// Time-driven and positional. It is *not* the §4 eviction cascade, which
    /// is demand-driven and chooses by class priority
    /// ([`Self::active_object_eviction_victim`]); §8.1 states the two "must
    /// not be collapsed" and warns that a shared distance constant across them
    /// is a sign they have been conflated.
    ///
    /// The position test is [`active_object_should_prune`] — a square window
    /// measured from the **scroll base** in unsigned eight-bit arithmetic, not
    /// a radius from the player. Releasing uses the ordinary §4 one-byte
    /// slot-freeing rule, so freed slots are immediately available to
    /// allocation.
    ///
    /// `0xB5` protection deliberately does **not** apply here: §4 scopes that
    /// byte as "the only universally protected byte-0 value **in this
    /// allocator**", and this sweep frees by position rather than by eviction
    /// priority.
    ///
    /// Spec gap: §8.1 requires that only slots of a "prunable kind" be
    /// considered but does not enumerate the classes. The pre-existing
    /// classification here — anything that is not empty and not vehicle-like —
    /// is kept unchanged rather than invented afresh; see the report on
    /// `active-objects.md §8.1` for the open question.
    ///
    /// The pass returns nothing: §8.1 forbids building a pruning event other
    /// systems can observe. The visibility-dirty mark is internal redraw
    /// bookkeeping, not a result.
    pub fn prune_far_overworld_objects(&mut self) {
        if !matches!(self.area, Area::World { .. }) {
            return;
        }
        let (scroll_base_x, scroll_base_y) = world_scroll_base(self.player.x, self.player.y);
        let scroll_base_x = scroll_base_x as u8;
        let scroll_base_y = scroll_base_y as u8;
        let mut pruned = false;
        // `active-objects.md §8.1`: "The pass walks the slots above zero
        // only." Starting at 1 is what keeps the player un-prunable.
        for slot in 1..self.active_objects.len() {
            let object = self.active_objects[slot];
            // `active-objects.md §8.1`: "A slot whose type byte does not
            // classify as a prunable kind is skipped **before** the position
            // test runs, so an out-of-window slot of an unclassified kind
            // survives." The `||` chain is ordered so classification precedes
            // `active_object_should_prune`.
            if object.is_empty()
                || is_vehicle_object_tile(object.type_byte)
                || is_vehicle_object_tile(object.tile)
            {
                continue;
            }
            if !active_object_should_prune(
                object.x as u8,
                object.y as u8,
                scroll_base_x,
                scroll_base_y,
            ) {
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

fn put_viewport_pixel(viewport: &mut TileViewport, x: i32, y: i32, colour: u8) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x >= viewport.width || y >= viewport.height {
        return;
    }
    viewport.pixels[y * viewport.width + x] = colour % viewport.depth.pixel_limit();
}

fn presentation_palette_index(depth: TileGraphicsDepth, ega_index: u8) -> u8 {
    match depth {
        TileGraphicsDepth::Ega16 => ega_index,
        TileGraphicsDepth::Cga4 => match ega_index {
            11 => 1,
            13 => 2,
            0 => 0,
            _ => 3,
        },
    }
}

fn draw_white_potion_sweep_cell(viewport: &mut TileViewport, cell_x: usize, cell_y: usize) {
    let colour = presentation_palette_index(viewport.depth, 15);
    draw_presentation_cross(viewport, cell_x, cell_y, colour);
    draw_presentation_cell_corners(viewport, cell_x, cell_y, colour);
}

fn draw_combat_potion_presentation_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    kind: CombatPotionPresentationKind,
) {
    let ega_colour = match kind {
        CombatPotionPresentationKind::Sleep => 11,
        CombatPotionPresentationKind::Poof => 13,
    };
    let colour = presentation_palette_index(viewport.depth, ega_colour);
    match kind {
        CombatPotionPresentationKind::Sleep => {
            draw_presentation_sleep_mark(viewport, cell_x, cell_y, colour)
        }
        CombatPotionPresentationKind::Poof => {
            draw_presentation_star(viewport, cell_x, cell_y, colour)
        }
    }
}

fn draw_combat_cursor_marker_cell(viewport: &mut TileViewport, cell_x: usize, cell_y: usize) {
    let colour = presentation_palette_index(viewport.depth, 14);
    draw_presentation_cell_corners(viewport, cell_x, cell_y, colour);
}

fn draw_combat_secondary_marker_cell(viewport: &mut TileViewport, cell_x: usize, cell_y: usize) {
    let colour = presentation_palette_index(viewport.depth, 11);
    draw_presentation_cross(viewport, cell_x, cell_y, colour);
}

fn draw_presentation_cross(viewport: &mut TileViewport, cell_x: usize, cell_y: usize, colour: u8) {
    let left = (cell_x * TILE_ATLAS_SIDE) as i32;
    let top = (cell_y * TILE_ATLAS_SIDE) as i32;
    let mid_x = left + (TILE_ATLAS_SIDE / 2) as i32;
    let mid_y = top + (TILE_ATLAS_SIDE / 2) as i32;
    draw_line(
        viewport,
        left + 2,
        mid_y,
        left + TILE_ATLAS_SIDE as i32 - 3,
        mid_y,
        colour,
    );
    draw_line(
        viewport,
        mid_x,
        top + 2,
        mid_x,
        top + TILE_ATLAS_SIDE as i32 - 3,
        colour,
    );
}

fn draw_presentation_cell_corners(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    colour: u8,
) {
    let left = (cell_x * TILE_ATLAS_SIDE) as i32;
    let top = (cell_y * TILE_ATLAS_SIDE) as i32;
    let right = left + TILE_ATLAS_SIDE as i32 - 1;
    let bottom = top + TILE_ATLAS_SIDE as i32 - 1;
    for offset in 0..3 {
        put_viewport_pixel(viewport, left + offset, top, colour);
        put_viewport_pixel(viewport, left, top + offset, colour);
        put_viewport_pixel(viewport, right - offset, top, colour);
        put_viewport_pixel(viewport, right, top + offset, colour);
        put_viewport_pixel(viewport, left + offset, bottom, colour);
        put_viewport_pixel(viewport, left, bottom - offset, colour);
        put_viewport_pixel(viewport, right - offset, bottom, colour);
        put_viewport_pixel(viewport, right, bottom - offset, colour);
    }
}

fn draw_presentation_star(viewport: &mut TileViewport, cell_x: usize, cell_y: usize, colour: u8) {
    draw_presentation_cross(viewport, cell_x, cell_y, colour);
    let left = (cell_x * TILE_ATLAS_SIDE) as i32;
    let top = (cell_y * TILE_ATLAS_SIDE) as i32;
    draw_line(
        viewport,
        left + 3,
        top + 3,
        left + TILE_ATLAS_SIDE as i32 - 4,
        top + TILE_ATLAS_SIDE as i32 - 4,
        colour,
    );
    draw_line(
        viewport,
        left + TILE_ATLAS_SIDE as i32 - 4,
        top + 3,
        left + 3,
        top + TILE_ATLAS_SIDE as i32 - 4,
        colour,
    );
}

fn draw_presentation_sleep_mark(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    colour: u8,
) {
    let left = (cell_x * TILE_ATLAS_SIDE) as i32;
    let top = (cell_y * TILE_ATLAS_SIDE) as i32;
    draw_line(viewport, left + 4, top + 4, left + 11, top + 4, colour);
    draw_line(viewport, left + 11, top + 4, left + 4, top + 11, colour);
    draw_line(viewport, left + 4, top + 11, left + 11, top + 11, colour);
}

fn draw_line(viewport: &mut TileViewport, x0: i32, y0: i32, x1: i32, y1: i32, colour: u8) {
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        put_viewport_pixel(viewport, x0, y0, colour);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

pub fn visibility_view_index(
    px: isize,
    py: isize,
    x: isize,
    y: isize,
    radius: usize,
) -> Option<usize> {
    let r = isize::try_from(radius).ok()?;
    let col = x - px + r;
    let row = y - py + r;
    let side = r.checked_mul(2)?.checked_add(1)?;
    if !(0..side).contains(&col) || !(0..side).contains(&row) {
        return None;
    }
    Some(row as usize * side as usize + col as usize)
}

pub fn visibility_marker_for_viewport_cell(col: usize, row: usize) -> u8 {
    if fog_refine_inside_clear_core(col as u8, row as u8) {
        VISIBILITY_CLEAR
    } else {
        VISIBILITY_DIM_PERIPHERY
    }
}

pub fn visibility_squared_distance(px: isize, py: isize, x: isize, y: isize) -> u32 {
    let dx = (x - px).unsigned_abs() as u32;
    let dy = (y - py).unsigned_abs() as u32;
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn surface_local_light_mask_origin(px: isize, py: isize, wrap_world: bool) -> (isize, isize) {
    if wrap_world {
        let half = (LOCAL_LIGHT_MASK_SIDE / 2) as isize;
        (px - half, py - half)
    } else {
        (0, 0)
    }
}

pub(crate) fn surface_local_light_mask_index(
    origin_x: isize,
    origin_y: isize,
    x: isize,
    y: isize,
    wrap_world: bool,
) -> Option<usize> {
    let (col, row) = if wrap_world {
        (
            (x - origin_x).rem_euclid(WORLD_SIDE as isize),
            (y - origin_y).rem_euclid(WORLD_SIDE as isize),
        )
    } else {
        (x - origin_x, y - origin_y)
    };
    if !(0..LOCAL_LIGHT_MASK_SIDE as isize).contains(&col)
        || !(0..LOCAL_LIGHT_MASK_SIDE as isize).contains(&row)
    {
        return None;
    }
    Some(row as usize * LOCAL_LIGHT_MASK_SIDE + col as usize)
}

fn surface_local_light_squared_distance(
    source_x: isize,
    source_y: isize,
    x: isize,
    y: isize,
    wrap_world: bool,
) -> u32 {
    if !wrap_world {
        return visibility_squared_distance(source_x, source_y, x, y);
    }
    let sx = source_x.rem_euclid(WORLD_SIDE as isize) as usize;
    let sy = source_y.rem_euclid(WORLD_SIDE as isize) as usize;
    let tx = x.rem_euclid(WORLD_SIDE as isize) as usize;
    let ty = y.rem_euclid(WORLD_SIDE as isize) as usize;
    let dx = i32::from(wrapped_world_axis_delta(sx, tx)).unsigned_abs();
    let dy = i32::from(wrapped_world_axis_delta(sy, ty)).unsigned_abs();
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

/// One candidate cell offered to a [`PlayState::surface_centre_out_carve`]
/// classifier: the cell itself, its centre-relative squared distance, its
/// live tile byte, and the cell the carve arrived from together with
/// whether that parent is currently painted. `visibility.md §5` needs the
/// parent's state for the sight-blocking influence rule.
struct SurfaceCarveCandidate {
    x: isize,
    y: isize,
    squared_distance: u32,
    tile: u8,
    parent_x: isize,
    parent_y: isize,
    parent_carved: bool,
}

/// A classifier's decision for one candidate. `paint` and `expand` are
/// independent: `visibility.md §5` paints a lit sight-blocker without
/// expanding through it, and expands through an unlit sight-transparent
/// cell without painting it.
struct SurfaceCarveVerdict {
    paint: bool,
    expand: bool,
}

/// `visibility.md §5`/`§6`: the one sight-propagation classifier, used
/// by every 2D scene family. Towns, dwellings, castles and keeps run the
/// *same* carve as the overworld — §11 says the town branch uses "the
/// same producer pipeline" — so there is no extra indoor opacity gate
/// here. An earlier town-only gate ANDed
/// [`surface_tile_blocks_projectile`] onto this classifier, which made
/// the interior brick floor `0x44` opaque and collapsed every indoor
/// scene to the player's own 3x3 neighbourhood.
pub fn surface_tile_propagates_visibility(tile: u8, squared_distance: u32) -> bool {
    if tile_blocks_sight_propagation(tile) {
        false
    } else if tile_propagates_sight_only_when_adjacent(tile) {
        squared_distance == 1
    } else {
        true
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

pub fn town_active_object_step_accepts_tile(tile: u8) -> bool {
    tile_class_dispatcher_accepts(tile, 0x10)
}

pub const fn town_free_roaming_object_eligible(object: ActiveObject) -> bool {
    (object.type_byte & 0xfe) == 0x10
}

pub const fn town_free_roaming_pen_tile_blocks(tile: u8) -> bool {
    matches!(tile, 0xa2 | 0x43)
}

pub const fn town_free_roaming_direction(axis: u8, sign: u8) -> Direction {
    match (axis & 1, sign & 1) {
        (0, 0) => Direction::North,
        (0, _) => Direction::South,
        (_, 0) => Direction::West,
        (_, _) => Direction::East,
    }
}

pub const fn town_free_roaming_facing_byte(direction: Direction, current: u8) -> u8 {
    match direction {
        Direction::East => 0x10,
        Direction::West => 0x11,
        Direction::North | Direction::South => current,
        Direction::NorthWest
        | Direction::NorthEast
        | Direction::SouthWest
        | Direction::SouthEast => current,
    }
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
        0x80..=0x8f | 0x9c..=0x9f | 0xfc..=0xff => tile <= 0x03 || (0x60..=0x6f).contains(&tile),
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
