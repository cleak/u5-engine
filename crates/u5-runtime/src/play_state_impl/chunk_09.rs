use std::io;
use std::path::Path;

use crate::rest_camp::camp_cooldown_after_hour_rollover;
use crate::*;

#[derive(Clone, Copy, Debug)]
struct PreparedTopDownCell {
    terrain: u8,
    grid: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutdoorActiveObjectStepAttempt {
    CandidateBlocked,
    ChanceRefused,
    Committed,
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

    /// Resolve the split terrain/actor storage domains into the 512-tile
    /// atlas. `active-objects.md §12` makes a zero/use-companion grid cell
    /// select `companion + 256`; direct and ordinary grid bytes stay in the
    /// terrain half. Actor byte `0x16` is transparent.
    fn tile_id(self) -> Option<usize> {
        match visibility_marker(self.grid) {
            VisibilityMarker::UseCompanion => actor_tile_for_byte(self.terrain),
            VisibilityMarker::ClearVisible | VisibilityMarker::DimPeriphery => {
                Some(usize::from(self.terrain))
            }
            VisibilityMarker::Hidden | VisibilityMarker::AlreadyRendered => None,
            VisibilityMarker::DirectTile(tile) => Some(usize::from(tile)),
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
            // `encounters.md §4`: "Terrain tile 7 (parched desert) |
            // **One-in-four** chance of the **Sand Trap** sprite run
            // `0xE0..0xE3`; a failed special roll rejects the candidate."
            // The sprite this writes was always right; the branch name and
            // the denominator were not. Sea Serpents reach the overworld
            // only through the water buckets, whose first frame is `0x88`.
            SpawnTerrainBranch::SandTrapParchedDesert => {
                (self.native_world_encounter_mod(0, 0x63, SPAWN_SAND_TRAP_DENOMINATOR) == 0)
                    .then_some(OUTDOOR_SAND_TRAP_SPRITE_RUN_FIRST)
            }
            SpawnTerrainBranch::UnderworldTile4RotWorm => Some(0xF8),
            SpawnTerrainBranch::HardReject | SpawnTerrainBranch::HighTileReject => None,
            // `encounters.md §4`: the low-tile allowance die is "a draw over
            // the closed interval `[0, 64]`, inclusive, accepted when the
            // result is below sixteen — **sixteen outcomes in sixty-five**".
            // Not a one-in-N gate, so it does not go through
            // `native_world_encounter_mod`.
            SpawnTerrainBranch::LowTileAllowance => {
                let allowance = self.random_range_u8(0, SPAWN_LOW_TILE_ALLOWANCE_DRAW_HIGH);
                if !spawn_low_tile_allowance_accepts(allowance) {
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
        // `overworld.md` Section 8, forced-movement table: the falls handler
        // saves the vehicle marker across the damage block and restores it
        // **before** the coordinate test, "so the plane swap runs with the
        // original transport in place; the falls handler does not force the
        // durable post-transition transport marker to foot". Section 8.1 says
        // nothing about the whirlpool's *durable* marker after the teleport,
        // so that arm keeps the ordinary reset and only the falls chain sets
        // this flag.
        let preserve_transport = entry.preserves_transport;
        let pre_transition_transport = self.player.transport;
        self.cache_current_world_overlay();
        self.area = Area::World {
            plane: entry.to_plane,
        };
        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        if preserve_transport {
            self.player.transport = pre_transition_transport;
        } else {
            self.force_foot_transport();
        }
        self.grid = load_world_map(game_dir, entry.to_plane)?;
        apply_world_quest_tile_substitutions(
            &mut self.grid,
            &self.word_of_power_seal_flags,
            &self.shrine_ruin_flags,
        );
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
        // `overworld.md` Section 8.1: "Overworld re-initialisation does not
        // redraw the chrome on either transition", and neither chain's plane
        // write carries narration of its own - the falls chain's transition
        // line is printed by the chain before this call, and the whirlpool
        // prints nothing here at all. The coordinate narration this used to
        // emit had no counterpart in the original and is removed.
        Ok(())
    }

    /// `overworld.md` Section 8 "Falls (chasm)" / Section 8.1: does a
    /// waterfall-family tile stand in the cell immediately south of the
    /// party, or under it?
    ///
    /// `RETRACTIONS.md` R320 withdrew the coordinate trigger this engine used
    /// to key on: `(54, 138)` is the *landing* cell that gates the plane
    /// flip, not a brink, so keying the chain on it fired the chain at none
    /// of its real trigger sites. Britannia has exactly three brink cells -
    /// `(46, 90)`, `(100, 96)` and `(54, 136)` - and the Underworld 116, and
    /// the chain runs at every one of them on **both** planes.
    pub fn world_falls_trigger_tile(&self) -> Option<u8> {
        if !matches!(self.area, Area::World { .. }) {
            return None;
        }
        let underfoot = self.grid[world_cell_index(self.player.x, self.player.y)];
        if is_waterfall_tile(underfoot) {
            return Some(underfoot);
        }
        let south_y = (self.player.y + 1) % WORLD_SIDE;
        let south = self.grid[world_cell_index(self.player.x, south_y)];
        is_waterfall_tile(south).then_some(south)
    }

    /// `overworld.md` Section 8.1 "Exact result lines: the falls chain", in
    /// print order. The chain is unconditional on both planes; only the
    /// landing coordinate decides whether the plane is also written.
    ///
    /// 1. `F-A-L-L-S!!!` + line feed
    /// 2. Two forced one-cell steps south, with one world tick between them
    /// 3. The 300-update descending sweep of `audio.md` Section 10.6
    /// 4. The party marker is hidden; one world tick
    /// 5. Per living member, the Dexterity check - **no text**
    /// 6. Two world ticks; the party marker is restored
    /// 7. `Falling into underworld!!`, only on Britannia `(54, 138)`
    pub fn apply_world_falls_chain(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<AreaTransition>> {
        if self.world_falls_trigger_tile().is_none() {
            return Ok(None);
        }

        // Step 1. There is no leading blank row: the earlier of the handler's
        // two entry points runs before the loop's gated newline, and every
        // overworld command echo already ends with a line feed.
        self.emit_message_line(OVERWORLD_FALLS_BANNER);

        // Step 2. "pushes the party two cells south, unconditionally, on both
        // planes, with one world tick between the two steps."
        for step in 0..OVERWORLD_FALLS_FORCED_STEPS_SOUTH {
            if step > 0 {
                self.advance_visual_tick();
            }
            self.player.y = (self.player.y + 1) % WORLD_SIDE;
            self.sync_player_object();
            self.mark_visibility_dirty();
        }

        // Step 3.
        self.emit_sound_effect(SoundEffect::SurfaceFallsDescent);

        // Step 4.
        self.party_marker_tile_override = Some(TRANSPORT_MARKER_SPRITE_SUPPRESSED);
        self.sync_player_object();
        self.advance_visual_tick();

        // Step 5. `RETRACTIONS.md` R321: the shared skewed `1..30` roll, and
        // damage lands when Dexterity is less than **or equal to** it. "There
        // is no per-member narration anywhere in the chain. An implementation
        // must not invent one: the fall's per-member feedback is graphical
        // and audible only" - which is what the shared damage helper emits.
        self.apply_world_falls_damage_pass();

        // Step 6.
        self.advance_visual_tick();
        self.advance_visual_tick();
        self.party_marker_tile_override = None;
        self.sync_player_object();
        self.mark_visibility_dirty();

        // Step 7. The gate never tests the plane (Section 8.1), but no
        // shipped underworld brink can reach column 54, so this is
        // unreachable from the Underworld in stock data.
        if !is_surface_chasm_cell(self.player.x as u8, self.player.y as u8) {
            return Ok(None);
        }
        self.emit_message_line(OVERWORLD_FALLS_UNDERWORLD_NARRATION);
        let entry = WorldPlaneTransitionEntry {
            from_plane: plane,
            x: self.player.x,
            y: self.player.y,
            to_plane: WorldPlane::Underworld,
            to_x: self.player.x,
            to_y: self.player.y,
            expected_tile: None,
            preserves_transport: true,
        };
        self.apply_world_plane_transition(game_dir, entry)?;
        Ok(Some(AreaTransition::ChangedWorldPlane {
            from: plane,
            to: WorldPlane::Underworld,
        }))
    }

    /// `overworld.md` Section 8, "Surface chasm/falls" row: "Each non-dead
    /// party member is checked once during the fall presentation."
    pub fn apply_world_falls_damage_pass(&mut self) {
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            let roll = self.world_plane_fall_save_roll();
            if !world_plane_fall_member_takes_damage(self.party[index].climb_stat, roll) {
                continue;
            }
            // "That helper is the same per-member application specified in
            // Section 6.2.4, and it emits a stats-row flash and a short
            // rumble but **no text**."
            let _ = self.apply_shared_party_damage(index, WORLD_PLANE_FALL_DAMAGE);
        }
    }

    /// `combat.md` Section 9.1 shared skewed roll, as `RETRACTIONS.md` R321
    /// requires: a uniform `0..60` draw halved with truncation, zero promoted
    /// to one. It must **not** be shared with outdoor K-Klimb, which draws a
    /// flat `1..30` directly and gates strictly.
    pub fn world_plane_fall_save_roll(&mut self) -> u8 {
        let raw = self.random_range_u8(
            WORLD_PLANE_FALL_SAVE_RAW_ROLL_LOW,
            WORLD_PLANE_FALL_SAVE_RAW_ROLL_HIGH,
        );
        combat_skewed_roll_1_to_30(raw)
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
                .render_dungeon_viewport(
                    radius,
                    atlas.depth,
                    atlas.dungeon_billboards.as_ref(),
                    atlas.dungeon_sprites.as_ref(),
                )
                .map(Some);
        };
        let _ = isize::try_from(radius).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "viewport radius is too large")
        })?;
        let prepared = if radius == VIEWPORT_PLAYER_ROW {
            if let Some(sweep) = self.visibility_sweep {
                self.prepare_visibility_sweep_render_grid(area, radius, sweep)
            } else {
                self.refresh_top_down_visibility_buffers(area, radius);
                self.prepared_top_down_grid_from_visibility_buffers()
            }
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
                let Some(tile_id) = cell.tile_id() else {
                    continue;
                };
                // `animation.md §12.2/§12.3`: the display driver rotates
                // water and lava, and composites the river, coast and shore
                // ids from the rotated shoals tile. `§12.4` flickers the fire
                // fixtures off the same driver pass. A cell
                // whose composed tile is a sprite takes the ordinary path,
                // exactly as it does for the `§6` families.
                blit_terrain_tile_to_viewport(
                    &mut viewport,
                    atlas,
                    tile_id,
                    cell_x,
                    cell_y,
                    self.water_scroll,
                    &self.fire_flicker,
                )?;
            }
        }
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
                blit_terrain_tile_to_viewport(
                    viewport,
                    atlas,
                    usize::from(terrain),
                    cell_x,
                    cell_y,
                    self.water_scroll,
                    &self.fire_flicker,
                )?;
                if let Some(sprite) = self.combat_render_sprite_at(arena_x, arena_y) {
                    // `animation.md §12.4`: a combat field-effect tile is one
                    // of the four the driver re-randomises every step, so the
                    // arena's sprite pass goes through the actor-half helper.
                    blit_actor_tile_to_viewport(
                        viewport,
                        atlas,
                        sprite,
                        cell_x,
                        cell_y,
                        &self.fire_flicker,
                    )?;
                }
            }
        }

        // `combat.md §7`: repaint the complete base viewport first. On a lit,
        // eligible player pass, draw the cursor and only then the secondary
        // marker so the latter wins at overlapping pixels. The secondary
        // coordinate has no arena-range precheck; ordinary pixel clipping is
        // the entire bounds policy.
        if self.combat_cursor_blink {
            if let Some((cursor_x, cursor_y)) = self.combat_cursor_actor_cell() {
                draw_combat_cursor_marker_cell(
                    viewport,
                    isize::from(cursor_x) + x_offset,
                    isize::from(cursor_y) + y_offset,
                );
                if let Some((marker_x, marker_y)) = self.combat_secondary_marker {
                    draw_combat_secondary_marker_cell(
                        viewport,
                        isize::from(marker_x) + x_offset,
                        isize::from(marker_y) + y_offset,
                    );
                }
            }
        }
        Ok(())
    }

    /// `catalogs/item-list.md §7.2`: the shared spell/potion visibility sweep
    /// runs the producer once — in its no-line-of-sight mode, so the field is
    /// all 121 cells — then "the completed grid stays unchanged for all twenty
    /// frames". Terrain, ordinary objects and animation are freshly
    /// composited through that frozen field; the sweep "draws no colour,
    /// circle, mask, line, or cell overlay" of its own.
    fn prepare_visibility_sweep_render_grid(
        &self,
        area: TopDownRenderArea,
        radius: usize,
        sweep: VisibilitySweep,
    ) -> Vec<Option<PreparedTopDownCell>> {
        let cells = radius.saturating_mul(2).saturating_add(1);
        let px = sweep.center_x as isize;
        let py = sweep.center_y as isize;
        let r = radius as isize;
        let mut prepared = vec![None; cells.saturating_mul(cells)];
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let index = cell_y * cells + cell_x;
                if !sweep.visible_cells.get(index).copied().unwrap_or(false) {
                    continue;
                }
                let world_x = px + cell_x as isize - r;
                let world_y = py + cell_y as isize - r;
                let terrain = match area {
                    TopDownRenderArea::Town => {
                        if !(0..32).contains(&world_x) || !(0..32).contains(&world_y) {
                            continue;
                        }
                        self.grid[world_y as usize * 32 + world_x as usize]
                    }
                    TopDownRenderArea::World(_) => self.world_live_tile_at(
                        world_x.rem_euclid(WORLD_SIDE as isize) as usize,
                        world_y.rem_euclid(WORLD_SIDE as isize) as usize,
                    ),
                };
                let terrain = self.animation.resolve_static_tile(terrain);
                prepared[index] = Some(PreparedTopDownCell {
                    terrain,
                    grid: terrain,
                });
            }
        }
        for slot in (1..self.active_objects.len()).rev() {
            self.composite_top_down_object(
                area,
                radius,
                self.active_objects[slot],
                false,
                &mut prepared,
            );
        }
        if let Some(z) = self.current_floor() {
            self.composite_top_down_object(
                area,
                radius,
                ActiveObject {
                    type_byte: self.player.transport.save_marker(),
                    tile: self.player.transport.save_marker(),
                    x: sweep.center_x,
                    y: sweep.center_y,
                    z,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                },
                true,
                &mut prepared,
            );
        }
        prepared
    }

    pub fn combat_render_actor_byte_at(&self, x: usize, y: usize) -> Option<u8> {
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
                Some(object.tile)
            })
    }

    pub fn combat_render_sprite_at(&self, x: usize, y: usize) -> Option<usize> {
        self.combat_render_actor_byte_at(x, y)
            .and_then(actor_tile_for_byte)
    }

    pub fn render_dungeon_viewport(
        &mut self,
        radius: usize,
        depth: TileGraphicsDepth,
        billboards: Option<&DungeonBillboardBanks>,
        sprites: Option<&DungeonSpriteBanks>,
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

        self.draw_dungeon_corridor(level, &mut viewport, billboards, sprites);
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
    fn draw_dungeon_corridor(
        &mut self,
        level: u8,
        viewport: &mut TileViewport,
        billboards: Option<&DungeonBillboardBanks>,
        sprites: Option<&DungeonSpriteBanks>,
    ) {
        let Area::Dungeon { scene, .. } = self.area else {
            return;
        };
        let bank = billboards.map(|banks| banks.bank(scene.presentation_flavour()));

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
        let mut last_visible_band = 0;
        let mut decorations = Vec::new();
        for band in 0..DUNGEON_BANDS {
            last_visible_band = band;
            let step = band as isize;
            let ahead_dx = fdx * step;
            let ahead_dy = fdy * step;
            let ahead = self.dungeon_renderer_offset_cell(level, ahead_dx, ahead_dy);
            let outcome = dungeon_forward_outcome(ahead, band);
            if outcome.point_blank {
                point_blank = true;
            }

            if let Some(role) = outcome.blocker {
                if let Some(bank) = bank {
                    self.draw_dungeon_billboard(viewport, bank, role, band);
                }
                if role == DungeonBillboardRole::ForwardFlavourWall
                    && scene.presentation_flavour() == DungeonPresentationFlavour::Normal
                {
                    let x =
                        dungeon_floor_wrap_coord(self.player.x as i16 + ahead_dx as i16) as usize;
                    let y =
                        dungeon_floor_wrap_coord(self.player.y as i16 + ahead_dy as i16) as usize;
                    decorations.push((x, y, band, DungeonDecorationPlacement::Forward));
                }
            }

            // A point-blank door suppresses the band-0 side cells so the
            // frame is not boxed in.
            if !(band == 0 && point_blank) {
                let left_cell =
                    self.dungeon_renderer_offset_cell(level, ahead_dx + ldx, ahead_dy + ldy);
                let right_cell =
                    self.dungeon_renderer_offset_cell(level, ahead_dx + rdx, ahead_dy + rdy);
                let left_role = dungeon_side_role(left_cell);
                let right_role = dungeon_side_role(right_cell);
                if let Some(bank) = bank {
                    self.draw_dungeon_billboard(viewport, bank, left_role, band);
                    self.draw_dungeon_billboard(viewport, bank, right_role, band);
                }
                if scene.presentation_flavour() == DungeonPresentationFlavour::Normal {
                    if left_role == DungeonBillboardRole::SideFlavourWall {
                        let x = dungeon_floor_wrap_coord(
                            self.player.x as i16 + (ahead_dx + ldx) as i16,
                        ) as usize;
                        let y = dungeon_floor_wrap_coord(
                            self.player.y as i16 + (ahead_dy + ldy) as i16,
                        ) as usize;
                        decorations.push((x, y, band, DungeonDecorationPlacement::SideLeft));
                    }
                    if right_role == DungeonBillboardRole::SideFlavourWall {
                        let x = dungeon_floor_wrap_coord(
                            self.player.x as i16 + (ahead_dx + rdx) as i16,
                        ) as usize;
                        let y = dungeon_floor_wrap_coord(
                            self.player.y as i16 + (ahead_dy + rdy) as i16,
                        ) as usize;
                        decorations.push((x, y, band, DungeonDecorationPlacement::SideRight));
                    }
                }
            }

            if !outcome.see_through {
                break;
            }
        }

        for (x, y, band, placement) in decorations.into_iter().rev() {
            self.draw_dungeon_wall_decoration(level, x, y, band, placement, viewport);
        }

        if let Some(sprites) = sprites {
            for band in (0..=last_visible_band).rev() {
                let step = band as isize;
                let x =
                    dungeon_floor_wrap_coord(self.player.x as i16 + (fdx * step) as i16) as usize;
                let y =
                    dungeon_floor_wrap_coord(self.player.y as i16 + (fdy * step) as i16) as usize;
                self.draw_dungeon_cell_contents(level, x, y, band, viewport, sprites);
            }
            self.dungeon_fountain_frame = (self.dungeon_fountain_frame + 1) % 3;
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

    /// `dungeon-mode.md §§6.6-6.9`: the far-to-near object/field pass.
    fn draw_dungeon_cell_contents(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        band: usize,
        viewport: &mut TileViewport,
        banks: &DungeonSpriteBanks,
    ) {
        let raw = self.dungeon_cell(level, x, y);
        let cell = dungeon_renderer_cell_byte(raw);
        let (rising, floor) = dungeon_object_family_slots(cell);
        if let Some(objects) = banks.objects() {
            if let Some(base) = rising {
                self.draw_dungeon_object_sprite(viewport, objects, base + band, band, true);
            }
            if let Some(base) = floor {
                self.draw_dungeon_object_sprite(viewport, objects, base + band, band, false);
            }
        }

        if cell >> 4 == 0x5 {
            let pen = if band == 0 { 9 } else { 1 };
            for &(px, py) in dungeon_fountain_points(band, self.dungeon_fountain_frame as usize) {
                Self::plot_dungeon_screen_pixel(viewport, px, py, pen);
                Self::plot_dungeon_screen_pixel(viewport, 190 - px, py, pen);
            }
        }

        if let Some(spec) = dungeon_field_paint_spec(cell, band) {
            for _ in 0..spec.strokes {
                let px = u5_prng_range_u16(
                    &mut self.prng_state,
                    spec.minimum,
                    spec.maximum - spec.endpoint_delta,
                ) as i32;
                let py = u5_prng_range_u16(&mut self.prng_state, spec.minimum, spec.maximum) as i32;
                for offset in 0..=i32::from(spec.endpoint_delta) {
                    Self::plot_dungeon_screen_pixel(viewport, px + offset, py, spec.pen);
                }
            }
        }

        self.draw_dungeon_active_monster(level, x, y, band, viewport, banks);

        // The raw 0x08 bit is a separate static rising-pit overlay after
        // ordinary object and field painting. It is not a visit marker.
        if cell >> 4 < 9 && raw & 0x08 != 0 {
            if let Some(objects) = banks.objects() {
                self.draw_dungeon_object_sprite(viewport, objects, 8 + band, band, true);
            }
        }
    }

    /// `dungeon-mode.md §4.1`: replace the one resident wandering
    /// dungeon-monster record. Family, placement attempts, and the
    /// Spider/Slime upper-placement check all consume the shared PRNG.
    pub fn setup_dungeon_active_monster_fresh(&mut self) -> bool {
        let Area::Dungeon { level, .. } = self.area else {
            return false;
        };
        let family = self.random_range_u8(0, 7);
        let family_index = usize::from(family);
        let mut object = ActiveObject {
            type_byte: family,
            tile: family,
            x: 0,
            y: 0,
            z: level as i8,
            phase: DUNGEON_MONSTER_INITIAL_STATES[family_index],
            aux1: DUNGEON_MONSTER_COMBAT_CLASSES[family_index],
            aux3: DUNGEON_MONSTER_FLOOR_DEP3,
        };
        let mut placed = false;
        for _ in 0..DUNGEON_ACTIVE_OBJECT_PLACEMENT_ATTEMPTS {
            let x = usize::from(self.random_range_u8(0, (DUNGEON_SIDE - 1) as u8));
            let y = usize::from(self.random_range_u8(0, (DUNGEON_SIDE - 1) as u8));
            if (x, y) == (self.player.x, self.player.y)
                || !dungeon_active_object_spawn_accepts(self.dungeon_cell(level, x, y))
            {
                continue;
            }
            object.x = x;
            object.y = y;
            placed = true;
            break;
        }
        if placed && matches!(family, 2 | 4) && self.random_range_u8(0, 99) > 48 {
            object.aux3 = DUNGEON_MONSTER_UPPER_DEP3;
        }
        if !placed {
            object.type_byte = 0;
            object.tile = 0;
            object.x = 0;
            object.y = 0;
            object.aux1 = DUNGEON_MONSTER_INACTIVE_DEP1;
        }

        if self.active_objects.len() <= DUNGEON_ACTIVE_MONSTER_SLOT {
            self.active_objects
                .resize(DUNGEON_ACTIVE_MONSTER_SLOT + 1, ActiveObject::empty());
        }
        self.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT] = object;
        self.mark_visibility_dirty();
        placed
    }

    fn draw_dungeon_active_monster(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        band: usize,
        viewport: &mut TileViewport,
        banks: &DungeonSpriteBanks,
    ) {
        if band == 0 {
            return;
        }
        let slot = DUNGEON_ACTIVE_MONSTER_SLOT;
        let Some(object) = self.active_objects.get(slot).copied().filter(|object| {
            dungeon_monster_record_active(*object)
                && object.z == level as i8
                && object.x == x
                && object.y == y
        }) else {
            return;
        };
        let family = usize::from(object.type_byte);
        if family >= DUNGEON_MONSTER_INITIAL_STATES.len() {
            return;
        }
        let Some(sheet) = banks.monster(family) else {
            return;
        };

        let (left_pose, right_pose) = if self.negate_time_active() {
            self.active_objects[slot].phase = DUNGEON_MONSTER_INITIAL_STATES[family];
            dungeon_monster_negate_poses(family)
        } else {
            let left = u5_prng_range_u16(&mut self.prng_state, 0, 100) < 50;
            let right = u5_prng_range_u16(&mut self.prng_state, 0, 100) < 50;
            let (new_state, left_pose, right_pose) =
                dungeon_monster_pose_step(object.phase, left, right);
            self.active_objects[slot].phase = new_state;
            (left_pose, right_pose)
        };

        let depth = band - 1;
        let y = if object.aux3 == DUNGEON_MONSTER_UPPER_DEP3 {
            DUNGEON_MONSTER_UPPER_Y[depth]
        } else {
            DUNGEON_MONSTER_NORMAL_Y[depth]
        };
        let Some(Some(left_sprite)) = sheet.sprites.get(left_pose * 3 + depth) else {
            return;
        };
        let Some(Some(right_sprite)) = sheet.sprites.get(right_pose * 3 + depth) else {
            return;
        };
        blit_dungeon_sprite(
            viewport,
            left_sprite,
            DUNGEON_MONSTER_LEFT_X[depth],
            y,
            false,
        );
        blit_dungeon_sprite(viewport, right_sprite, DUNGEON_VANISHING_X, y, true);
    }

    fn draw_dungeon_wall_decoration(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        band: usize,
        placement: DungeonDecorationPlacement,
        viewport: &mut TileViewport,
    ) {
        let index = dungeon_cell_index(level, x, y);
        let Some(cell) = self.grid.get(index).copied() else {
            return;
        };
        if cell >> 4 != 0x0c {
            return;
        }
        let stage = cell & 0x07;
        if stage == 5 {
            // `audio.md §8.5`: the droplet plays its depth-dependent glissando
            // only when it reaches landing stage 5. All four depth bands are
            // emitted, including the far band, whose published span is
            // negative: it produces no tone update but still performs the
            // final speaker stop.
            self.emit_sound_effect(SoundEffect::DungeonWallDrip { band: band as u8 });
            self.grid[index] = cell & 0xf8;
            return;
        }
        let Some((center_x, center_y)) = dungeon_decoration_position(placement, band, stage) else {
            return;
        };
        let pen = if stage == 4 { 11 } else { 1 };
        for offset in -1..=1 {
            Self::plot_dungeon_screen_pixel(viewport, center_x + offset, center_y, pen);
            Self::plot_dungeon_screen_pixel(viewport, center_x, center_y + offset, pen);
        }
        if stage < 4 {
            Self::plot_dungeon_screen_pixel(viewport, center_x, center_y, 9);
        }

        let advance = if stage == 0 {
            u5_prng_range_u16(&mut self.prng_state, 0, 64) <= 3
        } else {
            true
        };
        if advance {
            self.grid[index] = (cell & 0xf8) | (stage + 1);
        }
    }

    fn draw_dungeon_object_sprite(
        &self,
        viewport: &mut TileViewport,
        sheet: &GraphicSpriteSheet,
        slot: usize,
        band: usize,
        rising: bool,
    ) {
        let Some(Some(sprite)) = sheet.sprites.get(slot) else {
            return;
        };
        let y = if rising {
            95 - sprite.image.height as i32
        } else if slot / DUNGEON_BANDS == 0 || slot / DUNGEON_BANDS == 1 {
            96
        } else {
            DUNGEON_OBJECT_FLOOR_Y[band] - sprite.image.height as i32
        };
        blit_dungeon_sprite(viewport, sprite, DUNGEON_OBJECT_LEFT_X[band], y, false);
        blit_dungeon_sprite(viewport, sprite, DUNGEON_VANISHING_X, y, true);
    }

    fn plot_dungeon_screen_pixel(
        viewport: &mut TileViewport,
        screen_x: i32,
        screen_y: i32,
        pen: u8,
    ) {
        let x = screen_x - VIEWPORT_ORIGIN_X as i32;
        let y = screen_y - VIEWPORT_ORIGIN_Y as i32;
        if x < 0 || y < 0 || x >= viewport.width as i32 || y >= viewport.height as i32 {
            return;
        }
        viewport.pixels[y as usize * viewport.width + x as usize] =
            pen % viewport.depth.pixel_limit();
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
            self.composite_top_down_object_into_visibility_buffers(area, radius, object, false);
        }
        if let Some(z) = self.current_floor() {
            let marker = self.player.transport.save_marker();
            let player = ActiveObject {
                type_byte: marker,
                tile: marker,
                x: self.player.x,
                y: self.player.y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
            self.composite_top_down_object_into_visibility_buffers(area, radius, player, true);
        }
    }

    fn composite_top_down_object_into_visibility_buffers(
        &mut self,
        area: TopDownRenderArea,
        radius: usize,
        object: ActiveObject,
        player_slot: bool,
    ) {
        if object.is_empty() {
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
        // `visibility.md §8`: "Terrain-aware stamps use the live
        // world/combat tile at the object's coordinate, plus one
        // neighbouring row for a few edge shapes." The read is against the map
        // at the object's own world coordinate - not against the eleven-by-
        // eleven companion terrain band, which has no row outside the viewport
        // and which this pass overwrites as it walks slots thirty-one down to
        // zero. See [`Self::top_down_live_terrain_at`].
        let object_x = object.x as isize;
        let object_y = object.y as isize;
        let current_terrain = self
            .top_down_live_terrain_at(area, object_x, object_y)
            .unwrap_or(self.terrain_band[terrain_index]);
        let previous_row_terrain = self.top_down_live_terrain_at(area, object_x, object_y - 1);
        let next_row_terrain = self.top_down_live_terrain_at(area, object_x, object_y + 1);
        // `visibility.md §8.1`, normative: "For an actor whose composite
        // lands on one of the five selecting rows of the Section 8 table, the
        // variant is drawn **once per composite pass** - not once per
        // placement - and **it is never cached anywhere.** For every other
        // composited actor, **no draw is taken at all.**" The skips above have
        // already run, and `§8.1` is explicit that "every skip is evaluated
        // before the compositor is invoked, so a skipped actor costs no draw
        // at all. An implementation must not draw speculatively for an actor
        // it is about to skip".
        let variant = if composite_active_object_slot_draws_variant(
            object.type_byte,
            object.tile,
            current_grid_byte,
            current_terrain,
            previous_row_terrain,
            next_row_terrain,
        ) {
            self.draw_active_object_composite_variant()
        } else {
            0
        };
        match composite_active_object_slot(
            player_slot,
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
            self.composite_top_down_object(area, radius, object, false, &mut prepared);
        }
        if let Some(z) = self.current_floor() {
            let marker = self.player.transport.save_marker();
            let player = ActiveObject {
                type_byte: marker,
                tile: marker,
                x: self.player.x,
                y: self.player.y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
            self.composite_top_down_object(area, radius, player, true, &mut prepared);
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
                let terrain = self.top_down_live_terrain_at(area, x, y)?;
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
                let terrain = self.top_down_live_terrain_at(area, x, y)?;
                Some((terrain, None))
            }
        }
    }

    /// `visibility.md §8`: the live map tile at one world coordinate, with no
    /// lighting or visibility gate and with no composited sprite over it.
    ///
    /// The terrain-aware rows of the `§8` compositor table are defined against
    /// this read, not against the companion terrain band: "Terrain-aware
    /// stamps use the live world/combat tile at the object's coordinate, plus
    /// one neighbouring row for a few edge shapes." The band is not a
    /// substitute for it on either half of that sentence. It spans only the
    /// eleven-by-eleven viewport, so the neighbouring row of an object on an
    /// edge row is simply not in it; and every `Companion` stamp the pass has
    /// already made has *overwritten* the band cell it wrote, so an actor
    /// composited onto a laden-table cell erases the neighbour id a later slot
    /// would read. Either way `§8.1`'s draw count is the casualty - "an engine
    /// that draws from the shared stream on any other row advances the single
    /// global generator when the original does not, and its stream position
    /// diverges permanently" - and so is the stamped tile.
    ///
    /// This is the same accessor [`Self::top_down_render_cell_base`] uses once
    /// its visibility gates have passed, so for a lit, un-composited viewport
    /// cell the two agree byte for byte.
    fn top_down_live_terrain_at(&self, area: TopDownRenderArea, x: isize, y: isize) -> Option<u8> {
        let tile = match area {
            // `town-mode.md §15`: cells beyond the 32-by-32 floor take
            // the loaded floor's southeast-corner terrain, not a hole.
            TopDownRenderArea::Town => self.surface_viewport_tile(x, y, false)?,
            TopDownRenderArea::World(_) => {
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                self.world_live_tile_at(wx, wy)
            }
        };
        Some(self.animation.resolve_static_tile(tile))
    }

    fn composite_top_down_object(
        &self,
        area: TopDownRenderArea,
        radius: usize,
        object: ActiveObject,
        player_slot: bool,
        prepared: &mut [Option<PreparedTopDownCell>],
    ) {
        if object.is_empty() {
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
        // The same live-map read the buffer pass takes - `visibility.md §8`:
        // "Terrain-aware stamps use the live world/combat tile at the
        // object's coordinate, plus one neighbouring row for a few edge
        // shapes." Reading `prepared` instead would lose the neighbouring row
        // of an object on an edge row and would see earlier slots' stamps
        // rather than the map.
        let object_x = object.x as isize;
        let object_y = object.y as isize;
        let current_terrain = self
            .top_down_live_terrain_at(area, object_x, object_y)
            .unwrap_or(cell.terrain);
        let previous_row_terrain = self.top_down_live_terrain_at(area, object_x, object_y - 1);
        let next_row_terrain = self.top_down_live_terrain_at(area, object_x, object_y + 1);
        // This helper is `&self`: it answers *queries* about the composed
        // grid (the text view, the outdoor ranged line-of-sight trace, and
        // non-viewport radii), not the redraw's composite pass. `§8.1`
        // charges one draw per selecting actor "per composite pass", so a
        // query must take none - drawing here would advance the single global
        // gameplay stream on a frame the original never composites.
        // The canonical pass is
        // [`Self::composite_top_down_object_into_visibility_buffers`].
        //
        // Recorded consequence: a caller that renders *only* through this
        // helper - the text view, the visibility-sweep repaint, any radius
        // that is not the viewport's - shows the first entry of a selecting
        // row permanently, while the buffer path re-rolls it every pass. The
        // no-draw side is what `§8.1` charges; which of the two a non-viewport
        // presentation should show is not published, and is a spec question.
        let variant = 0;
        match composite_active_object_slot(
            player_slot,
            object.type_byte,
            object.tile,
            cell.grid,
            current_terrain,
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

    /// `visibility.md §8`'s shared variant selector for a composited
    /// active object: "unless the Negate Time timed effect is active, select
    /// a uniform random entry from the four-value range; while it is active,
    /// the selector short-circuits and returns the first entry for every
    /// actor".
    ///
    /// **This is a fresh draw from the shared gameplay stream on every
    /// composite pass, and it is never cached.** `§8.1` is normative: the
    /// selector "is a helper that takes no arguments and stores nothing of
    /// its own: it returns a value in `0..3` which the caller consumes
    /// immediately. Its one side effect is the generator advance inside the
    /// shared draw itself, which is unconditional whenever the selector is
    /// entered on its random arm." And: "**There is no cache to find.** ...
    /// there is no per-actor variant field, no scratch table keyed by actor,
    /// and no frame counter anywhere in the path."
    ///
    /// The caller must therefore have already established that this actor
    /// lands on one of the five selecting rows - see
    /// [`composite_active_object_slot_draws_variant`] - because `§8.1` charges
    /// "**zero draws for everything else**" and an engine that draws on any
    /// other row "advances the single global generator when the original does
    /// not, and its stream position diverges permanently".
    ///
    /// The short-circuit input is the **shared timed-effect register byte**,
    /// not a boolean and not a character class. `§8` retracts the "active
    /// character's class letter is Tinker" reading in full, and its producer
    /// census (`RETRACTIONS.md` R333) finds "**one site that installs a
    /// different timed-effect code into the same byte** - this is a shared
    /// timed-effect register that other effects also write, not a Negate Time
    /// flag". [`Self::negate_time_active`] reads that whole byte and compares
    /// it against the Negate Time code, so an install of some other effect's
    /// code leaves the selector on its random arm. The code has two
    /// producers: the spell at ten turns and the scroll at twenty.
    ///
    /// The short-circuit arm takes **no** draw: `§8.1` makes the generator
    /// advance conditional on the selector being "entered on its random arm".
    ///
    /// *Retracted in this engine:* until `cleak/u5-spec#182` was answered
    /// (spec commit `210aa41`, `RETRACTIONS.md` R329) this was a
    /// deterministic hash of the turn counter, the cell and the actor's
    /// bytes, re-rolled once per consumed turn. That was adopted because a
    /// clean-side capture saw a seated Avatar never change tile; R329
    /// explains the capture - the seat used was a fall-through, and "a seated
    /// actor that never changes tile is the expected result for the majority
    /// of seats, not a defect". A named-cell recapture then timed four
    /// qualifying seats at 0.695, 0.709, 0.742 and 0.753 transitions per tick
    /// and three fall-throughs at 0.000, which is the fresh per-pass draw.
    pub fn draw_active_object_composite_variant(&mut self) -> u8 {
        if self.negate_time_active() {
            // `§8.2`: "The composite still runs while Negate Time is active;
            // it just draws variant 0 every time."
            return active_object_compositor_variant(true, 0);
        }
        let selector = self.random_range_u8(
            ACTIVE_OBJECT_VARIANT_RANGE_LOW,
            ACTIVE_OBJECT_VARIANT_RANGE_HIGH,
        );
        active_object_compositor_variant(false, selector)
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
        if self.visibility_dirty || self.visibility_sweep.is_some() {
            return true;
        }
        self.viewport_has_animated_tiles_advanced_by(radius, StaticTileAnimationPass::ALL)
    }

    /// Narrower form of [`Self::viewport_has_animated_tiles`]: is any
    /// `animation.md §6` family that **this tick's pass advanced** visible
    /// in the viewport?
    ///
    /// The gated families only move on some ticks (the pendulum and the
    /// flag at half rate, the clock/bellows pair at quarter rate), so a
    /// tick whose pass skipped them cannot change the composed frame and
    /// must not force a rebuild of it.
    ///
    /// Unlike [`Self::viewport_has_animated_tiles`] this is a pure question
    /// about the map and the pass. It deliberately does **not** report the
    /// already-dirty field or a live sweep repaint as "animated": those two
    /// say the frame is being redrawn anyway, which is the opposite of a
    /// reason to raise the flag. The spell/potion visibility sweep in
    /// particular "does not itself dirty the visibility grid"
    /// (`catalogs/item-list.md §7.2`).
    pub fn viewport_has_animated_tiles_advanced_by(
        &self,
        radius: usize,
        pass: StaticTileAnimationPass,
    ) -> bool {
        let advances = |tile: u8| {
            static_tile_animation_family(tile).is_some_and(|family| pass.advances(family))
        };
        if self.combat_active {
            return self.combat_terrain.iter().flatten().copied().any(&advances);
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
                if advances(tile) {
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
        overworld_underfoot_forces_dark(tile, self.active_effect_tag.unwrap_or(0))
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

    /// `visibility.md §4` Stage 2 — the producer's branch on the light
    /// argument, which is **signed**. This is the entry point every caller
    /// that can supply a sentinel must use; the carve below is only the
    /// positive arm.
    ///
    /// | light | producer behaviour |
    /// |---|---|
    /// | positive | run the visibility carve with the value as the inclusive squared-distance threshold |
    /// | zero | total blackout — the carve is skipped outright and the grid stays fully obscured, "the player's own cell included" |
    /// | negative | full-fill — "every cell is populated from the world map directly, without any visibility carve, distance gate or blocker test" |
    ///
    /// **R327.** `visibility.md §3` previously called the negative branch
    /// "structurally unreachable in the shipped 2D pipeline". That is
    /// withdrawn: the redraw orchestrator still zero-extends the unsigned
    /// lighting byte and can never present a negative value, but it is not the
    /// producer's only caller — the spell/potion visibility sweep passes the
    /// negative sentinel deliberately, and that branch is "the whole mechanism
    /// of the White potion and the X-Ray spell". Implement it as live gameplay
    /// behaviour, not as compatibility scaffolding.
    pub fn surface_visibility_produce(
        &self,
        px: isize,
        py: isize,
        view_radius: usize,
        light: i32,
        wrap_world: bool,
    ) -> Vec<bool> {
        let side = view_radius.saturating_mul(2).saturating_add(1);
        let cell_count = side.saturating_mul(side);
        if light < 0 {
            // `catalogs/item-list.md §7.2`: "no distance test, no propagation
            // frontier, and no blocker rule on this branch: a wall does not
            // stop the reveal, and a cell in the far corner is revealed
            // exactly as readily as the party's own." The window bounds are
            // the whole contract, so every cell is in.
            return vec![true; cell_count];
        }
        if light == 0 {
            return vec![false; cell_count];
        }
        self.surface_visibility_carve_with_light_threshold(
            px,
            py,
            view_radius,
            light as u32,
            wrap_world,
        )
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

        // `visibility.md §12.4`: the disc-shaped source mask persists
        // between its three refresh triggers. The beacon is the one other
        // writer, stamped after that cached refresh and before this carve.
        let mut mask = self.local_light_mask.to_vec();
        let (mask_origin_x, mask_origin_y) = surface_local_light_mask_origin(px, py, wrap_world);
        self.stamp_light_beacon(&mut mask, mask_origin_x, mask_origin_y, wrap_world);
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

                // `town-mode.md §15`: the carve runs over the constructed
                // viewport, so off-floor town cells carry the substituted
                // southeast-corner terrain rather than dropping out.
                let Some(tile) = self.surface_viewport_tile(x, y, wrap_world) else {
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

    /// `visibility.md §12.4`: rebuild the persistent local-light resource.
    /// Callers are restricted to the published Moonstone live-gate refresh,
    /// combat entry, and combat exit trigger points.
    pub(crate) fn rebuild_surface_local_light_mask(&mut self) {
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let wrap_world = matches!(self.area, Area::World { .. });
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

        if let Some(floor) = self.current_floor() {
            for object in &self.active_objects {
                if object.is_empty()
                    || object.z != floor
                    || !is_local_light_source_tile(object.tile)
                {
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
        }
        if self.local_light_mask.as_slice() != mask.as_slice() {
            self.local_light_mask.copy_from_slice(&mask);
            self.mark_visibility_dirty();
        }
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
            self.grid.get(y as usize * 32 + x as usize).copied()
        } else {
            None
        }
    }

    /// `town-mode.md §15`: "movement reads the adjacent cell already
    /// present in the party-centred viewport. When that cell represents any
    /// coordinate outside a town floor, **viewport construction substitutes
    /// the loaded floor's southeast-corner cell `(31,31)`**."
    ///
    /// The substitution belongs to viewport construction, so it applies to
    /// the terrain the viewport paints and to the visibility carve that
    /// runs over that viewport - not to the 32-by-32 map window itself.
    /// [`Self::surface_visibility_tile`] therefore stays the raw map
    /// accessor and keeps returning `None` off-grid for the local-light
    /// scan of `visibility.md §12.1`, which explicitly "scan[s] the active
    /// thirty-two by thirty-two map window".
    ///
    /// OPEN SPEC QUESTION (`turn-clock-wind-report.md`, question 3): the two
    /// published sentences about an off-floor town read disagree.
    /// `town-mode.md §15` gives the `(31,31)` substitution above;
    /// `visibility.md §3` says the world-tile getter's "Out-of-range queries
    /// to the location/dungeon-explore buffer return a sentinel byte address
    /// (a fixed location whose contents act as a 'you walked off the map'
    /// tile)". Applying the §15 reading to the carve as well as to the
    /// painted terrain makes off-floor cells sight-propagating open ground.
    /// The original capture shows those rows lit, so the painted half is
    /// confirmed; if §3's sentinel is a distinct read, the carve should use
    /// it instead.
    ///
    /// Without this a party standing within five cells of a town edge sees
    /// the out-of-floor rows painted black; the original paints the corner
    /// terrain there (Britain's is grass).
    fn surface_viewport_tile(&self, x: isize, y: isize, wrap_world: bool) -> Option<u8> {
        if let Some(tile) = self.surface_visibility_tile(x, y, wrap_world) {
            return Some(tile);
        }
        if wrap_world || !matches!(self.area, Area::Town { .. }) {
            return None;
        }
        self.grid.get(TOWN_VIEWPORT_OFF_GRID_SAMPLE_INDEX).copied()
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
        self.advance_turn_with_minutes_policy(
            minutes,
            tick_doors,
            advance_active_objects,
            true,
            true,
        );
    }

    /// `rest-and-camp.md §5`: wilderness camp advances the ordinary clock and
    /// world epilogue without entering the hourly poison/provision/starvation
    /// pass. Ring regeneration is invoked once for this five-minute camp step.
    pub fn advance_wilderness_camp_tick(&mut self, minutes: u8) {
        self.advance_turn_with_minutes_policy(minutes, true, true, false, true);
        self.apply_hourly_ring_regeneration_tick();
    }

    /// `dungeon-mode.md §15`: "Dungeon turns advance the world clock at the
    /// indoor rate: one minute per loop iteration. The loop calls the same
    /// world-clock advance routine that town turns use, but — unlike the town
    /// and overworld loops — it does **not** gate that call on whether the
    /// command consumed a turn. The single call site sits at the head of each
    /// iteration, ahead of the render-and-poll step and the command dispatch,
    /// so a command the dispatcher reports as \"no action\" (a digit
    /// solo-select, a refused Push, the typeahead toggle) still costs a minute
    /// underground. Only the dungeon post-action pass is gated on the status
    /// word."
    ///
    /// This is the clock half of a turn: the calendar cascade, the hour
    /// bundle, the light-counter decay and the daylight recompute run, but the
    /// action counter is not bumped and no post-action pass is entered. The
    /// iteration's minute is flagged so the dispatched command's own
    /// `advance_turn` does not charge it a second time.
    pub fn advance_dungeon_loop_minute(&mut self) {
        let minutes = self.turn_minute_increment();
        let effective_minutes = if self.negate_time_active() {
            0
        } else {
            TimingStatusTag::from_save_byte(self.active_effect_tag.unwrap_or(0))
                .effective_minutes(minutes)
        };
        self.dungeon_loop_minute_charged = true;
        let previous_day = self.clock.day;
        let previous_hour = self.clock.hour;
        // `time.md §2`: "A pre-cascade snapshot of the hour, taken at the
        // start of every cleanup pass." `formats/saved-gam.md §5` persists it
        // at `0x02DA`.
        self.cleanup_previous_hour = previous_hour;
        self.clock.advance_minutes(effective_minutes);
        if self.clock.day != previous_day {
            self.reroll_shadowlord_hideouts();
        }
        if previous_day == 28 && self.clock.day == 1 {
            self.rare_reagent_harvest_days
                .fill(RARE_REAGENT_HARVEST_UNSEEN_DAY);
            self.fixed_hidden_treasure_daily_day = FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY;
            self.fortunes_of_war = 0;
            age_stay_counters_month(&mut self.party_stay_counters);
            age_inn_registry_month(&mut self.inn_registry);
        }
        if self.clock.hour != previous_hour {
            self.refresh_cached_moon_glyphs();
            self.camp_cooldown = camp_cooldown_after_hour_rollover(self.camp_cooldown);
        }
        self.decay_light_counters(effective_minutes);
        self.recompute_daylight();
    }

    pub(crate) fn advance_turn_with_minutes_policy(
        &mut self,
        minutes: u8,
        tick_doors: bool,
        advance_active_objects: bool,
        apply_hourly_party_status: bool,
        age_payment_cooldown: bool,
    ) {
        // `dungeon-mode.md §15`: the dungeon loop already charged this
        // iteration's minute at its head, so a turn-consuming dungeon command
        // spends the action but not the ordinary mode increment a second
        // time.
        //
        // The suppression covers only that mode increment. `dungeon-mode.md
        // §11` step 2 has the dungeon rest wrapper "elapse the accepted
        // duration by calling the world-clock advance routine repeatedly",
        // and `time.md §10` says a wait/rest handler "calls the cleanup
        // directly with whatever increment it wants" — those are a second,
        // genuine call site, so a handler-supplied increment keeps its full
        // value rather than being swallowed by the loop-head minute. The flag
        // is consumed on the next advance either way, so it cannot leak past
        // one turn.
        let minutes = if self.dungeon_loop_minute_charged {
            self.dungeon_loop_minute_charged = false;
            if matches!(self.area, Area::Dungeon { .. }) && minutes == self.turn_minute_increment()
            {
                0
            } else {
                minutes
            }
        } else {
            minutes
        };
        let defer_stonegate_epilogue = self.stonegate_trapdoor_underfoot_is_armed();
        // Production dispatch drains this at the end of the same town turn.
        // If a low-level caller began another turn without running that tail,
        // finish the earlier pass before changing the clock again rather than
        // silently dropping it.
        if std::mem::take(&mut self.pending_town_status_provision_pass) {
            self.apply_hourly_status_provision_pass();
        }
        self.apply_pending_town_object_epilogue();
        let negate_time_active = self.negate_time_active();
        let turn_before = self.turn;
        let effective_minutes = if negate_time_active {
            0
        } else {
            TimingStatusTag::from_save_byte(self.active_effect_tag.unwrap_or(0))
                .effective_minutes(minutes)
        };
        self.turn += 1;
        // `karma.md §4.1`: this saved byte is a turn-aged cooldown, not a
        // payment count. Combat owns a separate actor-turn loop. Town-bed
        // setup ticks opt out below; each real ten-minute rest tick enters
        // through the ordinary wrapper and ages once.
        if age_payment_cooldown && !self.combat_active {
            self.toll_progress = self.toll_progress.saturating_add(1);
        }
        let previous_day = self.clock.day;
        let previous_hour = self.clock.hour;
        // `time.md §2`: "A pre-cascade snapshot of the hour, taken at the
        // start of every cleanup pass." `formats/saved-gam.md §5` persists it
        // at `0x02DA`.
        self.cleanup_previous_hour = previous_hour;
        self.clock.advance_minutes(effective_minutes);
        if self.clock.day != previous_day {
            self.reroll_shadowlord_hideouts();
        }
        if previous_day == 28 && self.clock.day == 1 {
            // `time.md §8` long-period flag clears. "The traced set is
            // exactly: the three rare-reagent harvest cooldown cookies … the
            // cycling fixed hidden-treasure record's daily cooldown cookie
            // (record 14) … the early-game encounter-size damper." Zero is
            // written deliberately: "Zeroing matters because zero matches no
            // calendar day (days run `1..28`), so every once-per-day gate
            // that compares against one of these cookies is guaranteed open
            // on the first day of a new month."
            self.rare_reagent_harvest_days
                .fill(RARE_REAGENT_HARVEST_UNSEEN_DAY);
            self.fixed_hidden_treasure_daily_day = FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY;
            self.fortunes_of_war = 0;
            age_stay_counters_month(&mut self.party_stay_counters);
            age_inn_registry_month(&mut self.inn_registry);
        }
        if self.clock.hour != previous_hour {
            self.refresh_cached_moon_glyphs();
            // `rest-and-camp.md §5`: the camp cooldown counter is
            // "reduced by one, floored at zero, at every hour rollover".
            self.camp_cooldown = camp_cooldown_after_hour_rollover(self.camp_cooldown);
        }
        // `time.md §5`: the party status/provision pass runs "once per
        // turn-consuming action in overworld mode, town mode, and dungeon
        // mode", not once per hour. "Only the food and starvation branch is
        // gated on an hour change; everything else in the pass runs on every
        // invocation." The pass owns its own previous-hour snapshot, so it is
        // invoked here unconditionally and decides the hour gate itself.
        if apply_hourly_party_status {
            // `town-mode.md §7`/§10 puts this pass at the *end* of the
            // underfoot handler. Ordinary town actions are one-minute turns;
            // longer town-rest/arrest cleanup increments are direct time-loop
            // calls and retain their existing immediate pass.
            let ordinary_town_turn =
                matches!(self.area, Area::Town { .. }) && minutes == MINUTES_PER_INDOOR_TURN;
            if ordinary_town_turn || defer_stonegate_epilogue {
                self.pending_town_status_provision_pass = true;
            } else {
                self.apply_hourly_status_provision_pass();
            }
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
            let world_object_epilogue_runs = !matches!(self.area, Area::World { .. })
                || TimingStatusTag::from_save_byte(self.active_effect_tag.unwrap_or(0))
                    .world_object_epilogue_runs(turn_before);
            let ordinary_town_turn =
                matches!(self.area, Area::Town { .. }) && minutes == MINUTES_PER_INDOOR_TURN;
            if ordinary_town_turn || defer_stonegate_epilogue {
                self.pending_town_npc_schedule_pass = true;
                self.pending_town_active_object_pass =
                    advance_active_objects && world_object_epilogue_runs;
            } else {
                self.advance_npc_schedules();
                if advance_active_objects && world_object_epilogue_runs {
                    self.advance_active_objects();
                }
            }
        }
        self.age_active_effect();
        if tick_doors && !self.combat_active {
            self.tick_door_tracker();
        }
        self.advance_animation_clock();
        // `text-output.md §11`: the per-turn epilogue is the one phase
        // that reliably runs before the command handler writes its
        // result, so anything it printed is recorded here rather than
        // left in the slot for the handler to overwrite.
        self.flush_message_slot();
    }

    fn stonegate_trapdoor_underfoot_is_armed(&self) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        scene.byte == STONEGATE_SCENE_BYTE
            && !matches!(self.player.transport, TransportState::Carpet { .. })
            && self
                .grid
                .get(self.player.y * TOWN_GRID_SIDE + self.player.x)
                .copied()
                .is_some_and(is_town_trapdoor_live_tile)
    }

    /// One frame of a blocking presentation.
    ///
    /// `animation.md §13.5` claim 1: "No blocking presentation runs the town
    /// NPC schedule processor, the town object walker that moves loose
    /// horse-family objects, or the outdoor per-turn creature walker ...
    /// **Exceptions: none.**" What it *does* pay for is presentation state —
    /// `catalogs/item-list.md §7.2`: "That per-frame animator step advances
    /// sprite appearance only and **moves no actor**, so running it inside the
    /// sweep is faithful; what the sweep must not do is walk NPCs or
    /// creatures, and the original does not." Neither the loop nor the final
    /// idle redraw spends a turn or calls the gameplay clock, and Negate Time
    /// suppresses both the object and the tile animation step.
    pub fn advance_presentation_frame(&mut self) {
        if let Some(mut sweep) = self.visibility_sweep {
            if !self.negate_time_active() {
                self.animate_active_objects();
                self.advance_animation_clock();
            }
            if sweep.frames_remaining <= 1 {
                self.visibility_sweep = None;
            } else {
                sweep.frames_remaining -= 1;
                self.visibility_sweep = Some(sweep);
            }
        }
    }

    pub fn hourly_provision_consumer_count(&self) -> u16 {
        self.party
            .iter()
            .filter(|member| !matches!(member.status, b'D' | b'A' | b'S'))
            .count() as u16
    }

    /// `time.md §5` party status/provision pass, entered once per
    /// turn-consuming action. The pass "keeps its own previous-hour snapshot
    /// and compares it with the current hour", and updates that snapshot
    /// after the branch "so the branch cannot repeat until the clock crosses
    /// another hour".
    pub fn apply_hourly_status_provision_pass(&mut self) -> u16 {
        let hour_changed = self.clock.hour != self.status_pass_previous_hour;
        let consumers = self.apply_status_provision_pass(hour_changed);
        self.status_pass_previous_hour = self.clock.hour;
        consumers
    }

    /// `time.md §5`: "Only the food and starvation branch is gated on an
    /// hour change; everything else in the pass runs on every invocation."
    ///
    /// - *Unconditional part, every invocation*: the party walk (Dead-selector
    ///   clear plus the 1 HP-per-Poisoned-member tick) and the
    ///   provision-consumer count.
    /// - *Hour-gated part*: starvation when the provision counter is already
    ///   zero, otherwise the 06:00/12:00/18:00 provision decrement.
    /// - *Trailing part, every invocation*: the Ring of Regeneration 1-in-8
    ///   roll.
    pub fn apply_status_provision_pass(&mut self, hour_changed: bool) -> u16 {
        let consumers = self.hourly_provision_consumer_count();
        self.apply_hourly_poison_tick();
        if hour_changed {
            if self.food == 0 {
                self.pending_hourly_status_message = self.apply_hourly_starvation_tick();
            } else if is_provision_decrement_hour(self.clock.hour) {
                self.food = self.food.saturating_sub(consumers);
            }
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

    /// `time.md §5` unconditional part of the status/provision pass: "The
    /// pass walks the active party in slot order" and, per member, clears the
    /// active-member selector when that member is Dead, skips Dead and
    /// Sleeping members entirely, and takes "exactly 1 current hit point"
    /// from a member whose status is exactly Poisoned. "This is per member per
    /// turn, independently, not a shared roll and not an hourly effect."
    ///
    /// Poison damage that reaches zero hit points goes through the shared
    /// party-damage path, which marks the member Dead and "clears the
    /// active-member selector if it pointed at that member".
    pub fn apply_hourly_poison_tick(&mut self) -> u16 {
        let mut damaged = 0;
        for index in 0..self.party.len() {
            if self.party[index].status == b'D' {
                if self.active_player == Some(index) {
                    self.active_player = None;
                }
                continue;
            }
            if self.party[index].status != b'P' || !self.party[index].living() {
                continue;
            }
            self.party[index].apply_damage(FIRST_PLAYABLE_HOURLY_POISON_DAMAGE);
            if self.party[index].hp == 0 && self.active_player == Some(index) {
                self.active_player = None;
            }
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

    /// Append one result sentence to the slot, separated by a space only
    /// when the slot already holds something.
    ///
    /// An accepted step now leaves the slot empty (`text-output.md §10.2`),
    /// so the consequence lines that used to continue the move narration
    /// must not open with a stray space.
    pub(crate) fn append_result_sentence(&mut self, sentence: &str) {
        if self.message.is_empty() {
            self.message = sentence.to_string();
        } else {
            self.message.push(' ');
            self.message.push_str(sentence);
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
        // `visibility.md §12.4`: the Moonstone live-gate terrain refresh is
        // one of exactly three local-light rebuild triggers, and the rebuild
        // follows the terrain rewrite stage. This helper is also the mode-zero
        // initial refresh, establishing the first cached mask.
        let Some(indices) = self.natural_moongate_slot_indices_for_current_scene() else {
            self.natural_moongate_live_cells.clear();
            self.rebuild_surface_local_light_mask();
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
        self.rebuild_surface_local_light_mask();
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

    /// `time.md §6` / `lighting.md §3` stage one. The plane/floor forced-dark
    /// test is on "the party's Z value, read as an unsigned byte": any Z with
    /// its high bit set pins the ambient value at full darkness for every
    /// hour, which selects "the **Underworld plane** on the outdoor map … and
    /// a **below-entry floor** inside a town-family location".
    ///
    /// "It does **not** select ordinary dungeon levels: a dungeon level index
    /// counts upward from zero at the top of the stack, so it never sets the
    /// high bit, and the ambient value computed while the party is inside a
    /// dungeon is simply whatever the clock produces." Earlier wording placing
    /// "any dungeon depth" inside the forced-dark scope is withdrawn, so no
    /// depth is fed to the helper here.
    pub fn base_daylight(&self) -> u8 {
        // `Area::Town`'s floor is an `i8`, so a below-entry (basement) floor
        // is exactly the Z byte whose high bit is set when read unsigned.
        let underworld = match self.area {
            Area::World {
                plane: WorldPlane::Underworld,
            } => true,
            Area::Town { floor, .. } => floor < 0,
            _ => false,
        };
        let depth_z = 0u8;
        // `lighting.md §3` "Scope of the forced-dark tests": there are
        // exactly two, and the second "pins scene twenty-five (Ararat)
        // to 2 at every hour, independently of Z". The scene byte is
        // therefore part of the base-daylight input.
        let scene_byte = match self.area {
            Area::Town { scene, .. } => scene.byte,
            _ => SCENE_OVERWORLD,
        };
        daylight_base_value_for_scene(
            self.clock.hour,
            self.clock.minute,
            underworld,
            depth_z,
            scene_byte,
        )
    }

    /// One world step (`animation.md §13.2`).
    ///
    /// Returns the wind state the step's wind check installed, if it fired,
    /// so a shell that narrates the idle tick can report it. Every other
    /// caller ignores it.
    pub fn advance_visual_tick(&mut self) -> Option<WindState> {
        self.advance_visual_tick_inner(true)
    }

    /// A bare repaint tick: the redraw half of a world step, without the
    /// per-pass wind check and so without its gameplay-PRNG draw.
    ///
    /// `animation.md §13` separates the two: "The viewport rebuild and
    /// redraw run whenever the master redraw gate is set, regardless of the
    /// second gate", while §13.2's wind check is one of the things "one
    /// world step advances" behind both gates. Callers that the spec
    /// describes as repainting rather than stepping the world take this
    /// entry point, so their published gameplay-draw counts stay exact -
    /// see [`Self::apply_rough_seas_if_eligible`] and `overworld.md §6.2.5`.
    pub fn advance_world_repaint_tick(&mut self) {
        let _ = self.advance_visual_tick_inner(false);
    }

    fn advance_visual_tick_inner(&mut self, run_wind_check: bool) -> Option<WindState> {
        // A visibility-sweep repaint advances this same presentation state
        // from `advance_presentation_frame`. Suppress the ordinary frontend
        // tick so one displayed sweep frame cannot advance objects or tiles
        // twice.
        if self.visibility_sweep.is_some() {
            return None;
        }
        // `timing.md §8.2`: "The shared wait tests the current scene value and
        // performs no world step for values `0x21` through `0x7F`
        // **inclusive**". "First-person dungeon scenes occupy `0x21..0x28` and
        // therefore get no idle world step - they run their own loop instead,
        // which uses the same cursor-poll helper and so inherits the same
        // one-tick pacing and four-frame cursor, but whose per-pass work is a
        // first-person re-render and a rumble step, with no viewport rebuild,
        // no sprite animation, no wind check and no moongate or beacon work."
        //
        // The gate is the published **numeric range test on the scene value**,
        // not an "is this dungeon mode" test. The engine's other scene values -
        // overworld `0`, towns `1..=32`, combat `0xFF` - all fall outside the
        // band and step the world as before; combat is called out explicitly
        // as doing so.
        if idle_world_step_suppressed_for_scene(self.current_scene_byte()) {
            return None;
        }
        // Combat and endgame own temporary active-object tables. Their
        // presentation ticks must not recreate slot zero from the saved world
        // player after an actor release (the top-down renderer observes the
        // same ownership boundary).
        if !self.combat_active && self.endgame.is_none() {
            self.sync_player_object();
        }
        // `timing.md §8.2`: the idle wait performs no world step for scene
        // values `0x21..=0x7F` inclusive, and the gate must be that numeric
        // range test "**not** ... an 'is this dungeon mode' test: the band
        // is a strict superset of the dungeon scenes, and the intro,
        // character-creation and Return-to-View animation states (`0x40`,
        // `0x41`, `0x42`) also lie inside it". An `Area::Dungeon` match
        // cannot express those three, so the scene value is tested instead.
        // (This engine's `current_scene_byte()` cannot itself return
        // `0x40..=0x42` today - those states are not modelled as scene
        // values here - so at this call site the band is numerically the
        // same set as the dungeon family. The published shape is what is
        // being followed, and the predicate is reusable where those states
        // do get modelled.)
        // Combat reports `0xFF` and is outside the band, which is what
        // §8.2 requires ("Combat sets scene value `0xFF` and does run the
        // world step").
        //
        // NOT YET CONFORMANT, and out of scope here: §8.2 says the pass
        // performs *no world step* in the band, and `animation.md §13.2`
        // counts "the tail pair of Sections 6 and 12" among what one world
        // step advances - yet `advance_animation_clock()` below still runs
        // unconditionally, in the band as everywhere else. This gate covers
        // only the sprite/AI half. Behaviour there is unchanged from before
        // this change; closing it is the audit's separate `CONTRA (partial)`
        // row.
        if self.time_stop_counter == 0
            && !self.negate_time_active()
            && !idle_world_step_suppressed_for_scene(self.current_scene_byte())
        {
            // Presentation only: see [`Self::animate_active_objects`], which
            // per `active-objects.md §8` (R316) writes nothing but the
            // displayed tile and the packed phase/facing byte. A visual tick
            // must not move an actor, because the observed original does not
            // move one while the player is idle (R315).
            self.animate_active_objects();
        }
        // `animation.md §13.2`: one world step advances "one wind-change
        // check" alongside the object phases, and `§13.1` freezes it with
        // the rest of the step while Negate Time runs ("no wind check").
        // Unlike the object pass above it is not scene-gated -
        // `weather.md §2`: "The store happens before any scene test, so
        // the state is always updated; only the banner repaint is
        // conditional."
        let wind_change =
            if run_wind_check && self.time_stop_counter == 0 && !self.negate_time_active() {
                self.idle_wind_drift()
            } else {
                None
            };
        self.advance_animation_clock();
        // `combat.md §7`: the shared tile-painting pass - "run by the idle
        // redraw tick in *every* mode" - has a combat-band tail that "toggles
        // a blink flag each pass and, on the lit pass, draws the player cursor
        // box". This is that tick, so the toggle belongs here. Without it the
        // flag only moved at a round boundary, and the box the original blinks
        // sat solid for a whole round. The helper is inert outside combat.
        let _ = self.apply_combat_cursor_blink_tick();
        wind_change
    }

    // `timing.md §8.2` also publishes the under-sail wait pass's cost - "an
    // **under-sail auto-advance pass costs two ticks and one world step and
    // never enters the command wait at all**". That sentence is about the
    // *overworld command-wait helper*, and it is deliberately NOT implemented
    // here. `advance_visual_tick` is the shared "advance one world tick"
    // primitive: combat entry preserves `Area::World` and the hoisted-sail
    // transport, and the `.` pass-turn command routes through it too, so a
    // half-rate gate at this level would silently halve the combat
    // presentation beats and the pass-turn command as well. Implementing only
    // the cost half without the auto-advance would also leave a party sitting
    // still with sails hoisted running the whole world at half rate, which is
    // neither the original's behaviour nor anything published. If it is ever
    // implemented it belongs at the idle pump (`u5-bevy::visual_idle_tick`),
    // not here.

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

    /// Timing is a projection of the shared active-effect slot, never an
    /// independently mutable state value.
    pub const fn active_effect_timing_status(&self) -> TimingStatusTag {
        TimingStatusTag::from_save_byte(match self.active_effect_tag {
            Some(tag) if self.active_effect_counter != 0 => tag,
            _ => 0,
        })
    }

    pub fn clear_active_effect_slot(&mut self) {
        let changed = self.active_effect_tag.is_some() || self.active_effect_counter != 0;
        self.active_effect_tag = None;
        self.active_effect_counter = 0;
        if changed {
            self.mark_visibility_dirty();
        }
    }

    /// `animation.md §9`/`§12`: snapshot the driver-side animation layer so
    /// a rebuilt [`PlayState`] can carry it forward. Its state "lives in the
    /// asset buffer for the whole program run" and is not reset by an area
    /// entry, a scene change or a save load.
    pub const fn animation_asset_buffer(&self) -> AnimationAssetBuffer {
        AnimationAssetBuffer {
            animation: self.animation,
            water_scroll: self.water_scroll,
        }
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
        // `animation.md §13.1` "Two freezes an implementation must
        // reproduce": "**Negate Time freezes all of it.** While that timed
        // effect is active, the world tick forces the gating byte into a
        // skip state on *every* call, and the spell-effect sweep carries the
        // same test. For the effect's full duration nothing advances: no
        // water rotation, no fire flicker, no fountain, no banner, no clock
        // or bellows, no object animation, no AI roll, no wind check, no
        // moongate refresh, no beacon step, and no shrine/lava ambience
        // tick. ... An engine that keeps animating during Negate Time is
        // visibly wrong."
        //
        // `magic.md §8` names the same tag: "**`T` Negate Time.** The
        // per-turn cleanup skips the entire time advance ... and the
        // overworld epilogue returns before animating anything."
        //
        // The test lives inside the step rather than at its call sites
        // precisely because the spec says the gating byte is forced on
        // *every* call: both the idle world tick and the White-potion
        // spell-effect sweep reach the §6 pass through here, and §13.1
        // names both. Scripted scene animation that drives
        // `AnimationClock::tick_static_tiles` directly (the endgame and the
        // moongate transit) is not the world tick and is left alone.
        //
        // The composite pass is deliberately *not* suppressed with it:
        // `visibility.md §8.2` says "The composite still runs while Negate
        // Time is active; it just draws variant 0 every time."
        if self.negate_time_active() {
            return;
        }
        // `animation.md §6`: the pass that is about to run is the one for
        // the counter's *current* value; the counter is incremented once
        // at the end, "whichever path was taken".
        let pass = static_tile_animation_pass(self.animation.frame);
        self.animation.tick_static_tiles();
        // `cleak/u5-spec#179`: the display driver's water animator advances
        // one step per world tick through sixteen phases, on one global
        // counter shared by its rotation and composite stages. It rides the same
        // tick as the `§6` pass but is not part of it — no water tile is
        // a member of any published family (`RETRACTIONS.md` R148), and
        // this counter has neither the family gates nor their period.
        // Advancing it costs no repaint decision of its own: the water
        // pixels are rolled inside the blit, so the cached tile-id
        // buffers `main-loop.md §9` guards stay valid.
        self.water_scroll.tick();
        // `animation.md §12.4`: the same driver pass's third stage. "First
        // the driver refreshes four actor-half 'field' tiles ... with fresh
        // pseudo-random pixel bits from a generator the driver owns ... Then
        // it uses one of the refreshed tiles as a noise source and, for each
        // fire fixture, over the whole 16x16 tile: `fixture ^= (noise AND
        // mask)`." Like the water stages it has no gate of its own (`§12.1`),
        // so it rides this tick exactly.
        self.fire_flicker.tick();
        // `animation.md §7`/`§10`: "after advancing phases and tile
        // selectors, the engine explicitly gives the display layer a
        // chance to make the result visible", and an implementation must
        // "present the frame only after both the per-slot pass and the
        // global tile selector pass have completed".
        //
        // This engine's viewport composition caches the *resolved* family
        // frame in the terrain band (`§6`: "one selector update affect
        // every visible cell in the same family"), and `main-loop.md §9`
        // only re-runs the producer when the visibility-dirty flag is set.
        // Without this the composed frame keeps whatever selector value it
        // was built with, so a waterfall, fountain, pendulum, flag or
        // clock stays frozen for the whole life of the process even though
        // the shared counter is advancing underneath it. The per-slot pass
        // already marks the composition dirty when an object's frame tile
        // changes; the global tile pass owes the same notification.
        //
        // The flag is only raised when this tick's pass actually advanced
        // a family that is on screen, so `main-loop.md §9`'s lazy-refill
        // branch still runs on every tick that cannot change the picture.
        if self.viewport_has_animated_tiles_advanced_by(VIEWPORT_PLAYER_ROW, pass) {
            self.mark_visibility_dirty();
        }
    }

    /// The per-slot animator pass (`active-objects.md §8`,
    /// `animation.md §5`). For each non-empty slot it advances the animation
    /// phase and, at phase zero, may reroll the stored facing and the
    /// displayed frame.
    ///
    /// **It cannot move anything.** "The animator's complete set of record
    /// writes is the displayed-tile byte and the packed phase/facing byte; it
    /// never writes a slot's column or row."
    ///
    /// **R316.** The withdrawn text had the phase-zero roll "turn or step them
    /// one cell" for "hostile creature classes that wander on the overworld
    /// (or in towns past their schedule)". Both halves are gone: there is no
    /// animator movement path at all, so there is nothing for a collision rule
    /// to refuse. Ambient creature movement belongs to the outdoor per-turn
    /// walker [`Self::advance_outdoor_active_objects`], and a town's loose
    /// horse-family roamers to the separate town object walker
    /// [`Self::advance_town_free_roaming_active_objects`] — "town roamers,
    /// town schedule NPCs and outdoor creatures are three separate movement
    /// systems that happen to share this table".
    ///
    /// Because the pass moves nothing, it is also what a blocking
    /// presentation is allowed to pump: `animation.md §13.5` claim 1 —
    /// "no blocking presentation runs the town NPC schedule processor, the
    /// town object walker ..., or the outdoor per-turn creature walker.
    /// **Exceptions: none.**"
    pub fn animate_active_objects(&mut self) {
        for slot in 1..self.active_objects.len() {
            if self.active_objects[slot].is_empty()
                || (matches!(self.area, Area::Town { .. })
                    && town_free_roaming_object_eligible(self.active_objects[slot]))
            {
                continue;
            }
            let tick = self.active_objects[slot].tick_phase();
            if matches!(tick, PhaseTick::Countdown | PhaseTick::DecisionPoint) {
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
        self.pending_outdoor_reaction_slots.clear();
        // `active-objects.md §8` correction: this is a running total for the
        // whole high-to-low walk, not a per-slot flag. Once any reaction has
        // fired, movement dispatch stays disabled for every remaining slot.
        let mut reaction_count = 0usize;
        for slot in (1..self.active_objects.len()).rev() {
            if self.active_objects[slot].is_empty() {
                continue;
            }
            let phase_before_tick = self.active_objects[slot].phase;
            let tick = self.active_objects[slot].tick_phase();
            // Orthogonal adjacency is consumed before any ranged class test.
            // Its I/O-bearing combat/transition effect is completed by the
            // post-turn handler, but it claims the walker slot here so neither
            // it nor any lower slot can move out from under that handler.
            let claimed_by_first_phase = self.outdoor_first_phase_reaction_fires(slot);
            if claimed_by_first_phase {
                self.pending_outdoor_reaction_slots.push(slot);
            }
            reaction_count += usize::from(claimed_by_first_phase);
            let movement_allowed = reaction_count == 0;
            let ship_wind =
                if movement_allowed && self.slot_is_wind_driven_ship(slot, phase_before_tick) {
                    // `weather.md §7`: see `animate_active_objects` — the cadence
                    // counter lives in the slot and is not the shared countdown.
                    self.active_objects[slot].phase = merge_active_ship_cadence_phase(
                        self.active_objects[slot].phase,
                        phase_before_tick,
                    );
                    self.try_drift_active_ship(slot)
                } else {
                    ActiveShipWind::None
                };
            let ship_wind_changed = !matches!(ship_wind, ActiveShipWind::None);
            let wandered = movement_allowed
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
    /// §8 lists the immediate reactions in walker order: adjacent hostile
    /// engagement, adjacent whirlpool, adjacent sand trap, then the two
    /// ranged reactions. "Orthogonal adjacency is tested **before** any
    /// class test, so an adjacent creature engages rather than firing."
    /// The adjacency arms are already implemented on the world post-turn
    /// epilogue path; this method deliberately does not re-implement them.
    /// It covers the two ranged reactions, which share one routine per
    /// `overworld.md §6.2` — see [`crate::outdoor_ranged_attack`].
    ///
    /// Every window here is measured on **wrapped** deltas. §8's own
    /// proximity helper "first computes wrapped absolute distance to the
    /// player", and the overworld is a 256-cell torus: raw subtraction
    /// would read a creature one cell across the map seam as ~255 cells
    /// away and silently disable both attacks near the seam.
    pub fn outdoor_first_phase_ranged_attack(&mut self, slot: usize) -> bool {
        self.outdoor_first_phase_ranged_attack_detail(slot)
            .is_some()
    }

    /// Pure first-phase claim test used by the I/O-free walker. Reaction
    /// effects are staged until the post-turn pass, which can load combat
    /// resources and can pause/resume the remaining lower slots around a
    /// terrain-combat frame.
    pub fn outdoor_first_phase_reaction_fires(&self, slot: usize) -> bool {
        if self.outdoor_active_object_is_adjacent(slot) {
            return true;
        }
        let Area::World { plane } = self.area else {
            return false;
        };
        let Some(object) = self.active_objects.get(slot).copied() else {
            return false;
        };
        if object.z != plane.save_floor() || !is_outdoor_active_object_walker(object) {
            return false;
        }
        let (dx, dy) = wrapped_deltas_to_player(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        );
        match outdoor_ranged_attacker_figure(object.type_byte, dx, dy) {
            Some(OutdoorRangedAttackFigure::SparkCloud) => {
                outdoor_serpent_dragon_triggers(self.outdoor_serpent_dragon_breath_roll(slot))
            }
            Some(OutdoorRangedAttackFigure::SolidBurst) => true,
            None => false,
        }
    }

    /// [`Self::outdoor_first_phase_ranged_attack`], reporting what the
    /// shot did rather than only that it fired.
    pub fn outdoor_first_phase_ranged_attack_detail(
        &mut self,
        slot: usize,
    ) -> Option<OutdoorRangedAttackReport> {
        let Area::World { plane } = self.area else {
            return None;
        };
        let object = self.active_objects[slot];
        if object.z != plane.save_floor() || !is_outdoor_active_object_walker(object) {
            return None;
        }
        let (dx, dy) = wrapped_deltas_to_player(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        );
        // §8 / overworld.md §6.2.1: adjacency is tested before the class
        // test, so even a ranged family takes the adjacent-engagement arm at
        // an effective distance of one.
        if outdoor_offsets_are_orthogonally_adjacent(dx, dy) {
            return None;
        }

        // `overworld.md §6.2.1` is the closed recognition table for both
        // outdoor attackers. The breath row is exact equality against the
        // Sea Serpent and Dragon *first* frames — "Sibling animation
        // frames `0x89..0x8B` and `0xDD..0xDF` never enter the breath
        // branch" — while the broadside row is a masked family test on
        // `0x2C..0x2F`. "Do not generalise either rule to the other."
        let figure = outdoor_ranged_attacker_figure(object.type_byte, dx, dy)?;

        let announcement = match figure {
            // §6.2.1's announcement column: "None" for the breath, "A boom
            // message before the shot" for the broadside.
            OutdoorRangedAttackFigure::SparkCloud => {
                // "One-in-eight each turn ... the breath asks for the
                // closed interval `[0, 7]` and fires on one of those eight
                // outcomes." The broadside has no such gate: it "fires
                // whenever the geometry holds".
                if !outdoor_serpent_dragon_triggers(self.outdoor_serpent_dragon_breath_roll(slot)) {
                    return None;
                }
                None
            }
            OutdoorRangedAttackFigure::SolidBurst => Some(OUTDOOR_BROADSIDE_BOOM_MESSAGE),
        };

        Some(self.resolve_outdoor_ranged_attack(dx, dy, figure, announcement))
    }

    pub fn outdoor_active_object_is_adjacent(&self, slot: usize) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let Some(object) = self.active_objects.get(slot).copied() else {
            return false;
        };
        if object.z != plane.save_floor() || !is_outdoor_active_object_walker(object) {
            return false;
        }
        let (dx, dy) = wrapped_deltas_to_player(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        );
        outdoor_offsets_are_orthogonally_adjacent(dx, dy)
    }

    /// `overworld.md §6.2.1` one-in-eight breath-attack gate roll, reduced
    /// into the closed interval `[0, 7]` by
    /// [`OUTDOOR_SERPENT_DRAGON_TRIGGER_DENOMINATOR`]. Named so the gate
    /// can be exercised directly rather than inferred from whether a
    /// breath attack happened to fire.
    pub fn outdoor_serpent_dragon_breath_roll(&self, slot: usize) -> u8 {
        self.outdoor_active_object_step_seed(slot, OUTDOOR_SERPENT_DRAGON_BREATH_SALT)
            % OUTDOOR_SERPENT_DRAGON_TRIGGER_DENOMINATOR
    }

    /// `overworld.md §6.2` shared ranged-attack resolution, creature-to-
    /// party direction: announce, trace, and on a clear line run the
    /// §6.2.4 payload.
    ///
    /// The trace runs in **viewport** space, as §6.2.2 specifies. It samples
    /// the primary grid after compositing: an ordinary actor stamp therefore
    /// contributes passable sentinel `0x00` instead of either its sprite or
    /// the hidden terrain. An unresolved/dark cell is likewise passable.
    fn resolve_outdoor_ranged_attack(
        &mut self,
        wrapped_dx: i32,
        wrapped_dy: i32,
        figure: OutdoorRangedAttackFigure,
        announcement: Option<&str>,
    ) -> OutdoorRangedAttackReport {
        if let Some(announcement) = announcement {
            self.push_impact_line(announcement);
        }

        let attacker_cell = outdoor_ranged_attacker_viewport_cell(wrapped_dx, wrapped_dy);
        let plane = match self.area {
            Area::World { plane } => plane,
            _ => unreachable!("outdoor ranged attacks require world mode"),
        };
        let viewport =
            self.prepare_top_down_render_grid(TopDownRenderArea::World(plane), VIEWPORT_PLAYER_ROW);
        let outcome = trace_outdoor_ranged_attack(
            attacker_cell,
            OUTDOOR_RANGED_ATTACK_PARTY_CELL,
            |column, row| {
                let tile = usize::try_from(row)
                    .ok()
                    .zip(usize::try_from(column).ok())
                    .and_then(|(row, column)| viewport.get(row * VIEWPORT_SIDE + column))
                    .and_then(|cell| cell.map(|cell| cell.grid))
                    .unwrap_or(VISIBILITY_USE_COMPANION);
                outdoor_projectile_tile_blocks(tile)
            },
        );

        let absorption = match outcome {
            // §6.2.2: "*Blocked* means the shot stops where it stopped and
            // nothing further happens — no payload, no message, no state
            // change."
            OutdoorRangedAttackOutcome::Obstructed { .. } => None,
            // §6.2.4: "On a clear line the attack connects, and the
            // payload below runs."
            OutdoorRangedAttackOutcome::Connects => Some(self.apply_outdoor_impact()),
        };

        OutdoorRangedAttackReport {
            figure,
            attacker_cell,
            outcome,
            absorption,
        }
    }

    /// `overworld.md §6.2.4` shared impact payload, both stages.
    ///
    /// This is the whole payload, and it is shared: besides the two ranged
    /// attacks it is reached "from the sand-trap adjacency reaction and
    /// from the whirlpool engagement", and the drowning rung of
    /// `vehicles.md §6` runs its second stage directly.
    ///
    /// It does **not** route through the combat damage-and-status
    /// resolver: §6.2.4 states positively that "[n]o attacker identity,
    /// sprite byte, class or sentinel participates anywhere on this path,
    /// and the path never reaches the combat damage-and-status resolver."
    /// That is why nothing here takes an attacker argument.
    pub fn apply_outdoor_impact(&mut self) -> OutdoorImpactAbsorption {
        self.outdoor_impact_presentation();
        self.apply_outdoor_impact_absorption()
    }

    /// `overworld.md §6.2.5`: a released hoisted-frigate move that a
    /// non-pier destination refuses. Narration selects only on exact shoal
    /// terrain; the collision rumble owns the audio stream and absorption
    /// owns the single gameplay draw.
    pub fn apply_sailing_collision(&mut self, destination_tile: u8) -> OutdoorImpactAbsorption {
        self.message = if destination_tile == 0x03 {
            "BREAKING UP!".to_string()
        } else {
            "COLLISION!".to_string()
        };
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
        self.mark_visibility_dirty();
        self.emit_sound_effect(SoundEffect::ShipCollisionRumble);
        self.apply_outdoor_impact_absorption()
    }

    /// `overworld.md §6.2.5`: exact deep water under a skiff or carpet.
    pub fn rough_seas_trigger_is_eligible(&self) -> bool {
        if !matches!(self.area, Area::World { .. }) {
            return false;
        }
        let terrain = self.grid[world_cell_index(self.player.x, self.player.y)];
        let marker = self.player.transport.save_marker();
        terrain == 0x01
            && (matches!(
                marker,
                TRANSPORT_MARKER_SKIFF_FIRST..=TRANSPORT_MARKER_SKIFF_LAST
            ) || CARPET_MARKER_FRAMES.contains(&marker))
    }

    /// Run the rough-seas presentation and shared non-frigate absorption.
    /// The caller has already committed exactly one ordinary outdoor action.
    pub fn apply_rough_seas_if_eligible(&mut self) -> Option<OutdoorImpactAbsorption> {
        if !self.rough_seas_trigger_is_eligible() {
            return None;
        }
        self.push_impact_line("Rough seas!");
        self.outdoor_impact_presentation();
        self.emit_sound_effect(SoundEffect::RoughSeasImpactRumble);
        // `overworld.md §6.2.5`: "Order is `Rough seas!`, impact figure at
        // the party cell, impact rumble, one world repaint tick, then
        // absorption." The same section scopes the whole sequence's cost
        // over that same order - "`N` damaged members advance the audio
        // LFSR `300 + 160N` times in all, while consuming exactly `N`
        // gameplay draws" (the `300` is the impact rumble, which precedes
        // the tick) - and its conformance vector repeats it: "consume
        // exactly `N` gameplay draws in `1..8`". So the repaint tick here
        // must cost the gameplay stream nothing, which is the repaint half
        // `animation.md §13` separates out ("The viewport rebuild and
        // redraw run whenever the master redraw gate is set, regardless of
        // the second gate"), not a full §13.2 world step with its wind
        // check.
        self.advance_world_repaint_tick();
        Some(self.apply_outdoor_impact_absorption())
    }

    /// `overworld.md §6.2.4` stage one — impact presentation. "An impact
    /// figure is drawn at the party's own map coordinates ..., a short
    /// tone plays, and the viewport is rebuilt. This stage writes no
    /// character state and no vehicle state, and prints no narration
    /// line."
    fn outdoor_impact_presentation(&mut self) {
        self.mark_visibility_dirty();
    }

    /// `overworld.md §6.2.4` stage two — impact absorption. The stage
    /// "takes no arguments and branches on exactly one thing: the party's
    /// transport marker", and the two branches differ in kind.
    pub fn apply_outdoor_impact_absorption(&mut self) -> OutdoorImpactAbsorption {
        // "**Aboard a frigate** — any marker in the hoisted or furled ship
        // families ..., meaning all four headings and both sail states,
        // eight values in total."
        if let TransportState::Ship { hull, .. } = self.player.transport {
            let roll =
                self.random_range_u8(OUTDOOR_IMPACT_HULL_ROLL_LOW, OUTDOOR_IMPACT_HULL_ROLL_HIGH);
            return match outdoor_impact_hull_outcome(roll, hull) {
                // "Roll **strictly less than** the hull: subtract the roll
                // from the hull, repaint the stats panel, and return. **No
                // party member loses hit points.**"
                OutdoorImpactHullOutcome::Absorbed { hull_after } => {
                    if let TransportState::Ship { hull, .. } = &mut self.player.transport {
                        *hull = hull_after;
                    }
                    self.sync_player_object();
                    self.mark_visibility_dirty();
                    OutdoorImpactAbsorption::HullAbsorbed { roll, hull_after }
                }
                // "Roll **greater than or equal to** the hull: the ship is
                // destroyed. The ship-sunk line prints and the
                // loss-of-ship ladder in `systems/vehicles.md` Section 6
                // runs exactly as published there."
                OutdoorImpactHullOutcome::ShipDestroyed => {
                    self.push_impact_line(SHIP_SUNK_MESSAGE);
                    let (fallback, drowning) = self.apply_ship_loss_ladder();
                    OutdoorImpactAbsorption::ShipDestroyed {
                        roll,
                        fallback,
                        drowning,
                    }
                }
            };
        }

        // "**Under every other transport marker** — foot, horse, carpet,
        // skiff, and the sprite-suppressed value — the **whole-party
        // damage pass** below runs, and the stage returns."
        OutdoorImpactAbsorption::PartyDamaged(self.apply_outdoor_impact_party_damage())
    }

    /// `overworld.md §6.2.4` whole-party damage pass. It "walks roster
    /// slots from index zero upward. For each slot index that is **below
    /// the party-size byte** and whose **status byte is not the dead
    /// marker**, it draws its own **fresh, independent** uniform integer in
    /// the **closed interval `[1, 8]`, inclusive on both ends**, and
    /// applies it. The pass's own hard bound is six slots, indices
    /// `0..5`."
    ///
    /// What §6.2.4 rules out, "scoped to the whole of that pass and the
    /// whole of the absorption stage, both of which were read from entry
    /// to exit": no active-player selection, no first-living selection, no
    /// single randomly chosen target, no fixed slot, and one roll per
    /// damaged member rather than one roll shared between them.
    pub fn apply_outdoor_impact_party_damage(&mut self) -> Vec<OutdoorImpactMemberDamage> {
        let slots = self.party.len().min(OUTDOOR_IMPACT_PARTY_PASS_SLOT_BOUND);
        let mut applied_to = Vec::new();
        for slot in 0..slots {
            if !outdoor_impact_damages_member(self.party[slot].status) {
                continue;
            }
            let roll = self.random_range_u8(
                OUTDOOR_IMPACT_MEMBER_DAMAGE_LOW,
                OUTDOOR_IMPACT_MEMBER_DAMAGE_HIGH,
            );
            applied_to.push(self.apply_shared_party_damage(slot, roll));
        }
        applied_to
    }

    /// `overworld.md §6.2.4` per-member application — "the same
    /// party-damage helper that the surface chasm/falls row of Section 8
    /// uses for its one-point fall damage".
    ///
    /// The helper flashes the member's row, subtracts from the **current**
    /// hit points word (character record `+0x10`), clamps at zero and
    /// writes the dead status letter into `+0x0B` when the result is zero
    /// or below, and "if the member that just died is the currently
    /// selected character, writes the published 'none selected' value into
    /// the active-player index byte". `formats/saved-gam.md §4` publishes
    /// that value as `0xFF`, modelled here as `None`. §6.2.4 is explicit
    /// that it "is **not** an attacker id, and nothing on this path reads
    /// it back as one".
    ///
    /// "Maximum hit points, experience, level, magic points and equipment
    /// are untouched by this helper." §6.2.5 keeps the closing stats-panel
    /// repaint an open gap, so that list is "these fields are written",
    /// not "only these fields are written".
    pub fn apply_shared_party_damage(
        &mut self,
        slot: usize,
        amount: u8,
    ) -> OutdoorImpactMemberDamage {
        // `audio.md §8.2`: "The shared damage presentation runs the
        // 160-update 100..2000 Hz rumble. Preserve the caller's own
        // damage/narration order; this is not a global sound for every HP
        // write." This helper *is* that shared presentation - it flashes the
        // member's row before subtracting from the hit-point word - so the
        // rumble is recorded ahead of the write and ahead of the death and
        // active-player bookkeeping below. The cue is deliberately not
        // attached to `PartyMember::apply_damage`, which several callers
        // reach without this presentation and which stay silent.
        self.emit_sound_effect(SoundEffect::DamageRumble);
        let applied = self.party[slot].apply_damage(amount);
        let hp_after = self.party[slot].hp;
        let died = hp_after == 0;
        if died && self.active_player == Some(slot) {
            self.active_player = None;
        }
        // `overworld.md §6.2.4` / `stats-panel.md §2.1`: the closing full
        // repaint independently clears a selected Sleeping row. Its Dead
        // safeguard is normally redundant with the endpoint write above.
        if stats_panel_active_cursor_resets(self, self.active_player) {
            self.active_player = None;
        }
        OutdoorImpactMemberDamage {
            slot,
            roll: amount,
            applied,
            hp_after,
            died,
        }
    }

    /// `vehicles.md §6` loss-of-ship ladder: "The engine walks a fixed
    /// fallback ladder and takes the first option that is available."
    ///
    /// Returns the rung taken, and — on the drowning rung — one entry per
    /// iteration of the drowning loop.
    pub fn apply_ship_loss_ladder(
        &mut self,
    ) -> (ShipLossFallback, Vec<Vec<OutdoorImpactMemberDamage>>) {
        let TransportState::Ship {
            type_byte, skiffs, ..
        } = self.player.transport
        else {
            // Only a frigate can be lost. Reaching here with any other
            // marker would mean the absorption stage's ship branch and
            // this ladder disagree about what the party is aboard.
            return (ShipLossFallback::Drown, Vec::new());
        };
        let facing = type_byte & TRANSPORT_FACING_MASK;
        let fallback =
            ship_loss_fallback(skiffs, self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX]);
        let mut drowning = Vec::new();

        match fallback {
            // "The party abandons into a skiff, keeping the ship's current
            // facing, and the marker becomes the matching skiff value."
            ShipLossFallback::Skiff => {
                self.push_impact_line(ABANDON_SHIP_MESSAGE);
                let marker = TRANSPORT_MARKER_SKIFF_FIRST + facing;
                self.player.transport = TransportState::Skiff {
                    type_byte: marker,
                    tile: transport_visual_tile_for_marker(marker)
                        .unwrap_or(FIRST_PLAYABLE_SKIFF_TILE),
                };
            }
            // "The party deploys a carried carpet, the carried-carpet
            // count is decremented, and the marker becomes one of the two
            // carpet frames (chosen at random, since the frame is
            // cosmetic)."
            ShipLossFallback::Carpet => {
                self.push_impact_line(ABANDON_SHIP_MESSAGE);
                self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] =
                    self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX].saturating_sub(1);
                let pick = self.random_mod_u8(CARPET_MARKER_FRAMES.len() as u8) as usize;
                let marker = CARPET_MARKER_FRAMES[pick % CARPET_MARKER_FRAMES.len()];
                self.player.transport = TransportState::Carpet {
                    type_byte: marker,
                    tile: transport_visual_tile_for_marker(marker)
                        .unwrap_or(FIRST_PLAYABLE_MAGIC_CARPET_TILE),
                };
            }
            // "The marker is set to the sprite-suppressed value and the
            // drowning outcome runs. This is the only way the suppressed
            // value becomes persistent state."
            ShipLossFallback::Drown => {
                self.player.transport = TransportState::SpriteSuppressed;
                // `audio.md §8.9`, first row: "`Ship sunk!` prints, the party
                // sprite is cleared, the stats panel refreshes, and the
                // viewport is rebuilt so the empty ocean is on screen -
                // **then** the long descent - then `DROWNING!!!`, then the
                // death loop."
                //
                // `Ship sunk!` was pushed by the absorption stage before it
                // called this ladder, and the sprite clear is the marker write
                // immediately above, so this is the published slot: after the
                // clear and its repaint, before the loop. The loop's own
                // per-member damage rumbles therefore trail the sweep in the
                // effect history rather than racing it.
                //
                // Only this rung sounds. On the skiff and carpet rungs "the
                // game prints `Abandon ship!`, substitutes the vehicle, and
                // plays **no** long sound", which is why the emission is inside
                // this arm and not after the match.
                self.sync_player_object();
                self.mark_visibility_dirty();
                self.emit_sound_effect(SoundEffect::LongDescent);
                self.push_impact_line(DROWNING_MESSAGE);
                drowning = self.apply_ship_loss_drowning();
            }
        }

        self.sync_player_object();
        self.mark_visibility_dirty();
        (fallback, drowning)
    }

    /// `vehicles.md §6` drowning outcome: "a loop, and it tests before it
    /// damages. While the shared living-member scan does not report 'none
    /// remaining', each iteration plays the impact presentation at the
    /// party's cell and runs the whole-party damage pass ... **once**".
    ///
    /// "A party that is already entirely dead when the ladder reaches this
    /// rung takes no damage at all, because the test comes first."
    ///
    /// The exit scan and the damage filter are deliberately different
    /// tests; see [`party_member_counts_as_living`] and
    /// [`outdoor_impact_damages_member`].
    pub fn apply_ship_loss_drowning(&mut self) -> Vec<Vec<OutdoorImpactMemberDamage>> {
        let mut iterations = Vec::new();
        while self.party_has_drowning_loop_survivor() {
            self.outdoor_impact_presentation();
            iterations.push(self.apply_outdoor_impact_party_damage());
        }
        iterations
    }

    /// `vehicles.md §6` shared living-member scan, as the drowning loop's
    /// exit test uses it.
    pub fn party_has_drowning_loop_survivor(&self) -> bool {
        self.party
            .iter()
            .take(OUTDOOR_IMPACT_PARTY_PASS_SLOT_BOUND)
            .any(|member| party_member_counts_as_living(member.status))
    }

    /// Emit one impact-path line into the message window.
    ///
    /// `text-output.md §11`: the impact lines are produced by the
    /// per-turn epilogue, which runs *before* the command handler writes
    /// its own result, so they must reach the transcript as they are
    /// produced. Assigning the slot here is what lost the broadside
    /// announcement: the handler's own result replaced it and "no test of
    /// an individual message" could show it.
    fn push_impact_line(&mut self, line: &str) {
        self.emit_message_line(line);
    }

    /// `weather.md §7`: is this slot one of the wind-driven "ship-like
    /// water-creature class" records the overworld per-slot movement
    /// dispatch applies prevailing wind to?
    ///
    /// `phase_before_tick` is the slot's phase byte as it stood before
    /// the shared animation countdown ran this turn; see
    /// [`merge_active_ship_cadence_phase`].
    pub fn slot_is_wind_driven_ship(&self, slot: usize, phase_before_tick: u8) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let Some(object) = self.active_objects.get(slot).copied() else {
            return false;
        };
        object.z == plane.save_floor()
            && is_ship_object(object)
            // `STEADY_PHASE` is this engine's "slot does not animate"
            // marker (see `ActiveObject::tick_phase`), which is what a
            // parked vehicle object carries. Weather drives *active*
            // ship-like slots; a steady slot is not one, and
            // `merge_active_ship_cadence_phase` guarantees the cadence
            // encoding never produces that value.
            && (phase_before_tick & 0x0f) != STEADY_PHASE
            && cardinal_direction_from_active_object_phase(phase_before_tick).is_some()
    }

    /// `weather.md §7` Active Ships.
    ///
    /// "Calm wind suppresses this movement. For non-calm wind, the object
    /// uses its current frame and the prevailing wind to select a cadence
    /// cap" — perpendicular frames move every turn, a frame facing the
    /// wind source moves 2 of 3 turns, a frame facing away moves 3 of 4.
    /// [`WindState::active_ship_cadence`] owns that table.
    ///
    /// "The cadence counter is stored per active-object slot. A '2 of 3'
    /// entry means the slot moves on two eligible passes, then resets and
    /// skips one. A '3 of 4' entry moves on three eligible passes, then
    /// resets and skips one. The cadence counter is persisted with the
    /// object, so it survives save and reload. 'Every turn' bypasses the
    /// counter and immediately allows the slot's movement helper to run."
    ///
    /// The counter lives in bits `2..3` of the object's phase byte —
    /// `formats/saved-gam.md §8.1` byte `+6`, the "animation phase /
    /// direction-step counter ... compositor reads it for water
    /// creatures" — whose high nibble carries the frame heading and
    /// whose bits `0..1` select the drawn frame. See
    /// [`ACTIVE_SHIP_CADENCE_PHASE_MASK`]. The callers hand this
    /// function the pre-tick phase so the shared animation countdown
    /// cannot decrement the cadence count.
    ///
    /// This replaces an ad-hoc heading-versus-wind test that stalled every
    /// perpendicular frame forever, which is the opposite of the table's
    /// "every turn" row.
    pub fn try_drift_active_ship(&mut self, slot: usize) -> ActiveShipWind {
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
        // Calm returns `None` from the cadence table as well, but keep the
        // explicit wind read so a calm slot reports `Stalled` rather than
        // "not a ship".
        let Some(cadence) = self
            .wind
            .direction()
            .and_then(|_| self.wind.active_ship_cadence(heading))
        else {
            return ActiveShipWind::Stalled;
        };

        let counter =
            (object.phase & ACTIVE_SHIP_CADENCE_PHASE_MASK) >> ACTIVE_SHIP_CADENCE_PHASE_SHIFT;
        let moves_per_cycle = cadence.0;
        if cadence != ACTIVE_SHIP_CADENCE_EVERY_TURN {
            let next = if counter >= moves_per_cycle {
                // The cycle's moves are spent: skip this pass and reset.
                0
            } else {
                counter + 1
            };
            self.active_objects[slot].phase = (object.phase & !ACTIVE_SHIP_CADENCE_PHASE_MASK)
                | (next << ACTIVE_SHIP_CADENCE_PHASE_SHIFT);
            if counter >= moves_per_cycle {
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
                other_slot != slot
                    && other_slot != ACTIVE_OBJECT_PLAYER_SLOT
                    && self.object_occupies(*other, nx, ny)
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
        // The cadence counter written above stays: `weather.md §7` counts
        // the passes the cadence allowed, and validation of the step it
        // allowed belongs to `active-objects.md`.
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

        for direction in self
            .outdoor_directed_step_directions(slot, object)
            .into_iter()
            .flatten()
        {
            let (dx, dy) = direction.delta();
            match self.try_step_outdoor_active_object_detail(
                slot,
                object,
                dx,
                dy,
                direction,
                last_vacated,
            ) {
                OutdoorActiveObjectStepAttempt::Committed => return true,
                OutdoorActiveObjectStepAttempt::ChanceRefused => return false,
                OutdoorActiveObjectStepAttempt::CandidateBlocked => {}
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
        self.outdoor_directed_step_directions(slot, object)
            .into_iter()
            .flatten()
            .next()
    }

    pub fn outdoor_directed_step_directions(
        &self,
        slot: usize,
        object: ActiveObject,
    ) -> [Option<Direction>; 2] {
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
        candidates.map(|(sx, sy)| cardinal_direction_from_delta(sx, sy))
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
        matches!(
            self.try_step_outdoor_active_object_detail(
                slot,
                object,
                dx,
                dy,
                direction,
                last_vacated,
            ),
            OutdoorActiveObjectStepAttempt::Committed
        )
    }

    fn try_step_outdoor_active_object_detail(
        &mut self,
        slot: usize,
        object: ActiveObject,
        dx: isize,
        dy: isize,
        direction: Direction,
        last_vacated: &mut Option<(usize, usize)>,
    ) -> OutdoorActiveObjectStepAttempt {
        let nx = (object.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = (object.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if (nx, ny) == (self.player.x, self.player.y) || *last_vacated == Some((nx, ny)) {
            return OutdoorActiveObjectStepAttempt::CandidateBlocked;
        }
        if self
            .active_objects
            .iter()
            .enumerate()
            .any(|(other_slot, other)| {
                other_slot != slot
                    && other_slot != ACTIVE_OBJECT_PLAYER_SLOT
                    && self.object_occupies(*other, nx, ny)
            })
        {
            return OutdoorActiveObjectStepAttempt::CandidateBlocked;
        }
        let tile = self.grid[world_cell_index(nx, ny)];
        if !outdoor_active_object_step_accepts_tile(
            object.type_byte,
            tile,
            self.passability.as_ref(),
        ) {
            return OutdoorActiveObjectStepAttempt::CandidateBlocked;
        }
        if !type_bypasses_terrain_chance_gate(object.type_byte) {
            if let Some(denominator) = terrain_chance_gate_denominator(tile) {
                if self.outdoor_active_object_step_seed(slot, tile) % denominator != 0 {
                    return OutdoorActiveObjectStepAttempt::ChanceRefused;
                }
            }
        }

        *last_vacated = Some((object.x, object.y));
        if outdoor_step_clears_on_destination(tile) {
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
            return OutdoorActiveObjectStepAttempt::Committed;
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
        OutdoorActiveObjectStepAttempt::Committed
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
        // `town-mode.md §16` eligible-object-bytes row: "Empty slots, the
        // avatar's slot-zero record, **linked NPC sprite classes**, and all
        // other object families are skipped." Twenty shipped roster slots
        // carry the horse tags `0x10`/`0x11` (`catalogs/npc-roster.md §4`:
        // "Unmounted horse frames, used for stable and paddock actors"), so
        // a linked stable horse would otherwise be driven twice per turn -
        // once by its schedule and once by this walker - and would spend
        // PRNG draws doing it. The chance gate sits below this test because
        // the same row says "No PRNG value is consumed for ineligible or
        // off-floor slots".
        if self.npcs.iter().any(|npc| npc.active_object == Some(slot)) {
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
                // `town-mode.md §16` gives the pen gate exactly two live
                // blocker bytes. Map bounds are tested only after the two
                // direction draws select a destination, so an edge object can
                // still choose an inward step and an outward choice consumes
                // the same draws before failing.
                continue;
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
        let mut reacted = false;
        for object_slot in (1..self.active_objects.len()).rev() {
            if !self.outdoor_active_object_is_adjacent(object_slot) {
                continue;
            }
            let object = self.active_objects[object_slot];
            if is_whirlpool_object(object) || outdoor_sand_trap_class(object.type_byte) {
                continue;
            }
            if let Some(outcome) = self.apply_world_generic_adjacent_slot_engagement(
                game_dir,
                plane,
                object_slot,
                object,
            )? {
                reacted = true;
                if self.combat_active {
                    return Ok(Some(outcome));
                }
            }
        }
        Ok(reacted.then_some(MoveOutcome::Used))
    }

    pub(crate) fn apply_world_generic_adjacent_slot_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        object_slot: usize,
        object: ActiveObject,
    ) -> io::Result<Option<MoveOutcome>> {
        if terrain_combat_base_class(object).is_none() {
            return Ok(None);
        }

        // The generic arm rebuilds first and emits its fixed line before
        // either the shared impact payload or terrain combat.
        self.mark_visibility_dirty();
        self.push_impact_line("Attacked!");
        let party_terrain = self.grid[world_cell_index(self.player.x, self.player.y)];
        if generic_adjacent_hostile_uses_impact(party_terrain, self.player.transport.save_marker())
        {
            self.apply_outdoor_impact();
            return Ok(Some(MoveOutcome::Used));
        }

        let _setup_report =
            self.enter_terrain_combat_from_world_object(game_dir, plane, object_slot, object)?;
        Ok(Some(MoveOutcome::Used))
    }

    pub fn world_object_epilogue_runs_for_turn(&self, turn_before: u64) -> bool {
        TimingStatusTag::from_save_byte(self.active_effect_tag.unwrap_or(0))
            .world_object_epilogue_runs(turn_before)
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
    /// a radius from the player. Releasing uses §8.1's shared record-writer
    /// behavior: encoded fields 0..=5 are cleared while phase/DEP3 survive.
    /// This is intentionally not the ordinary §4 one-byte free.
    ///
    /// Classification is the exact byte-0 range table in
    /// [`active_object_type_is_prunable`]. Byte 1 (`tile`) does not
    /// participate even when it no longer mirrors byte 0. In particular,
    /// parked vehicles and pickups survive, `0x2C..=0x2F` is prunable, and
    /// the entire `0xB4..=0xB7` band (including protected `0xB5`) survives.
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
            if !active_object_type_is_prunable(object.type_byte) {
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
            if let Some(object) = self.active_objects.get_mut(slot) {
                object.clear_record_prefix();
            }
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

fn draw_combat_cursor_marker_cell(viewport: &mut TileViewport, cell_x: isize, cell_y: isize) {
    let colour = presentation_palette_index(viewport.depth, 15);
    let left = (cell_x * TILE_ATLAS_SIDE as isize) as i32;
    let top = (cell_y * TILE_ATLAS_SIDE as isize) as i32;
    for offset in [0, 1, 14, 15] {
        draw_line(
            viewport,
            left,
            top + offset,
            left + 15,
            top + offset,
            colour,
        );
        draw_line(
            viewport,
            left + offset,
            top,
            left + offset,
            top + 15,
            colour,
        );
    }
}

fn draw_combat_secondary_marker_cell(viewport: &mut TileViewport, cell_x: isize, cell_y: isize) {
    let white = presentation_palette_index(viewport.depth, 15);
    let black = presentation_palette_index(viewport.depth, 0);
    let left = (cell_x * TILE_ATLAS_SIDE as isize) as i32;
    let top = (cell_y * TILE_ATLAS_SIDE as isize) as i32;

    // Upper white group.
    draw_line(viewport, left + 2, top + 6, left + 6, top + 6, white);
    draw_line(viewport, left + 6, top + 2, left + 6, top + 6, white);
    // Upper black group: narrow left, wide left, narrow right, wide right.
    draw_line(viewport, left + 2, top + 5, left + 5, top + 5, black);
    draw_line(viewport, left + 5, top + 2, left + 5, top + 5, black);
    draw_line(viewport, left + 2, top + 7, left + 6, top + 7, black);
    draw_line(viewport, left + 7, top + 2, left + 7, top + 6, black);
    draw_line(viewport, left + 10, top + 5, left + 13, top + 5, black);
    draw_line(viewport, left + 10, top + 2, left + 10, top + 5, black);
    draw_line(viewport, left + 9, top + 7, left + 13, top + 7, black);
    draw_line(viewport, left + 8, top + 2, left + 8, top + 6, black);

    // Lower white group.
    draw_line(viewport, left + 2, top + 9, left + 6, top + 9, white);
    draw_line(viewport, left + 6, top + 9, left + 6, top + 13, white);
    // Lower black group: narrow left, wide left, narrow right, wide right.
    draw_line(viewport, left + 2, top + 10, left + 5, top + 10, black);
    draw_line(viewport, left + 5, top + 10, left + 5, top + 13, black);
    draw_line(viewport, left + 2, top + 8, left + 6, top + 8, black);
    draw_line(viewport, left + 7, top + 9, left + 7, top + 13, black);
    draw_line(viewport, left + 10, top + 10, left + 13, top + 10, black);
    draw_line(viewport, left + 10, top + 10, left + 10, top + 13, black);
    draw_line(viewport, left + 9, top + 8, left + 13, top + 8, black);
    draw_line(viewport, left + 8, top + 9, left + 8, top + 13, black);
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

/// `systems/weather.md §7`: "The cadence counter is stored per
/// active-object slot ... and is persisted with the object."
///
/// This engine keeps that counter in bits `2..3` of the slot's phase
/// byte ([`ACTIVE_SHIP_CADENCE_PHASE_MASK`]), which the shared
/// animation countdown in [`ActiveObject::tick_phase`] would otherwise
/// decrement as part of the low nibble. Rejoin the two: the frame
/// selector and heading come from the post-tick byte, the cadence count
/// from the pre-tick byte.
///
/// The one forbidden result is a low nibble equal to [`STEADY_PHASE`],
/// which is the "slot does not animate" marker a parked vehicle
/// carries; clearing the two cosmetic frame bits keeps the encoding out
/// of it.
pub const fn merge_active_ship_cadence_phase(post_tick: u8, pre_tick: u8) -> u8 {
    let merged =
        (post_tick & !ACTIVE_SHIP_CADENCE_PHASE_MASK) | (pre_tick & ACTIVE_SHIP_CADENCE_PHASE_MASK);
    if (merged & 0x0f) == STEADY_PHASE {
        merged & !0x03
    } else {
        merged
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

#[cfg(test)]
mod time_cascade_and_daylight_spec_tests {
    use crate::test_fixtures::{
        dungeon_state, open_dungeon_record, open_grid, open_world_grid, test_state, world_state,
    };
    use crate::*;

    /// `time.md §5`: "A member whose status is exactly Poisoned loses
    /// **exactly 1 current hit point** … This is per member per turn,
    /// independently, not a shared roll and not an hourly effect."
    #[test]
    fn a_poisoned_member_loses_one_hit_point_on_every_turn_not_every_hour() {
        let mut state = test_state(open_grid(), 10, 10);
        state.party[0].status = b'P';
        state.party[0].hp = DEFAULT_PARTY_HP;
        state.food = 100;
        let hour_before = state.clock.hour;

        for _ in 0..3 {
            state.advance_turn();
            // Town's shared pass is the trailing act of its underfoot
            // handler, not part of the clock advance itself.
            state.apply_pending_town_status_provision_pass();
        }

        assert_eq!(
            state.clock.hour, hour_before,
            "the three turns must stay inside one hour for this to test the cadence"
        );
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP - 3);
    }

    /// `time.md §5`: "Only the food and starvation branch is gated on an hour
    /// change" - so provisions must not be spent by ordinary turns inside the
    /// same hour.
    #[test]
    fn provisions_are_not_spent_by_turns_inside_the_same_hour() {
        let mut state = test_state(open_grid(), 10, 10);
        state.clock.hour = 12;
        state.clock.minute = 10;
        state.status_pass_previous_hour = 12;
        state.food = 100;

        for _ in 0..5 {
            state.advance_turn();
            state.apply_pending_town_status_provision_pass();
        }

        assert_eq!(state.clock.hour, 12);
        assert_eq!(state.food, 100);
    }

    /// `time.md §5`: "If the member is Dead and is also the currently selected
    /// active member, the active-member selector is cleared to its
    /// no-selection sentinel."
    #[test]
    fn the_status_pass_clears_the_selector_when_it_names_a_dead_member() {
        let mut state = test_state(open_grid(), 10, 10);
        state.party[0].status = b'D';
        state.party[0].hp = 0;
        state.active_player = Some(0);
        state.food = 100;

        state.advance_turn();
        state.apply_pending_town_status_provision_pass();

        assert_eq!(state.active_player, None);
    }

    /// `time.md §8`: the month rollover zeroes "the three rare-reagent harvest
    /// cooldown cookies", "the cycling fixed hidden-treasure record's daily
    /// cooldown cookie (record 14)" and "the early-game encounter-size
    /// damper". "Zeroing matters because zero matches no calendar day."
    #[test]
    fn the_month_rollover_clears_the_long_period_cookies() {
        let mut state = test_state(open_grid(), 10, 10);
        state.clock.day = 28;
        state.clock.hour = 23;
        state.clock.minute = 59;
        state.rare_reagent_harvest_days = [7; RARE_REAGENT_HARVEST_POINT_COUNT];
        state.fixed_hidden_treasure_daily_day = 7;
        state.fortunes_of_war = 3;
        state.food = 100;

        state.advance_turn();

        assert_eq!(state.clock.day, 1, "the turn must cross the 28 -> 1 wrap");
        assert_eq!(
            state.rare_reagent_harvest_days,
            [RARE_REAGENT_HARVEST_UNSEEN_DAY; RARE_REAGENT_HARVEST_POINT_COUNT]
        );
        assert_eq!(
            state.fixed_hidden_treasure_daily_day,
            FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY
        );
        assert_eq!(state.fortunes_of_war, 0);
    }

    /// `time.md §8`: the clears fire only on the `28 -> 1` wrap, "never at
    /// ordinary midnight".
    #[test]
    fn ordinary_midnight_leaves_the_long_period_cookies_alone() {
        let mut state = test_state(open_grid(), 10, 10);
        state.clock.day = 12;
        state.clock.hour = 23;
        state.clock.minute = 59;
        state.rare_reagent_harvest_days = [7; RARE_REAGENT_HARVEST_POINT_COUNT];
        state.fixed_hidden_treasure_daily_day = 7;
        state.food = 100;

        state.advance_turn();

        assert_eq!(state.clock.day, 13);
        assert_eq!(state.rare_reagent_harvest_days[0], 7);
        assert_eq!(state.fixed_hidden_treasure_daily_day, 7);
    }

    /// `time.md §6` / `lighting.md §3`: "any Z with its high bit set … pins
    /// the ambient value at 2 for every hour", which selects "a **below-entry
    /// floor** inside a town-family location".
    #[test]
    fn a_town_basement_floor_is_fully_dark_at_midday() {
        let mut basement = test_state(open_grid(), 10, 10);
        let Area::Town { scene, .. } = basement.area else {
            unreachable!("test_state builds a town area");
        };
        basement.area = Area::Town { scene, floor: -1 };
        basement.clock.hour = 12;
        basement.clock.minute = 0;

        assert_eq!(basement.base_daylight(), FULL_DARKNESS);

        let mut entry_floor = test_state(open_grid(), 10, 10);
        entry_floor.clock.hour = 12;
        entry_floor.clock.minute = 0;
        assert_eq!(entry_floor.base_daylight(), FULL_DAYLIGHT);
    }

    /// `lighting.md §3`: the forced-dark test "does **not** select ordinary
    /// dungeon levels: a dungeon level index counts upward from zero at the
    /// top of the stack, so it never sets the high bit, and the ambient value
    /// computed while the party is inside a dungeon is simply whatever the
    /// clock produces." Earlier "any dungeon depth" wording is withdrawn.
    #[test]
    fn ordinary_dungeon_levels_are_not_forced_dark() {
        for level in [0u8, 1, 5] {
            let mut state = dungeon_state(open_dungeon_record(), level, 1, 1);
            state.clock.hour = 12;
            state.clock.minute = 0;
            assert_eq!(
                state.base_daylight(),
                FULL_DAYLIGHT,
                "dungeon level {level} must take the clock's value"
            );

            state.clock.hour = 2;
            assert_eq!(state.base_daylight(), FULL_DARKNESS);
        }
    }

    /// `time.md §6`: the Underworld plane keeps its high-bit forced dark.
    #[test]
    fn the_underworld_plane_stays_fully_dark_at_midday() {
        let mut state = world_state(open_world_grid(), 40, 40);
        state.clock.hour = 12;
        state.clock.minute = 0;
        assert_eq!(state.base_daylight(), FULL_DARKNESS);
    }
}
