impl PlayState {
    fn rest_with_watch(
        &mut self,
        hours: Option<u8>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let Some(hours) = hours else {
            self.message = "Rest- how many hours? Use H plus a number in this harness.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if !(1..=24).contains(&hours) {
            self.message = "Rest hours must be in 1..24.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let asleep_at_start = self
            .party
            .iter()
            .filter(|member| member.status == b'S')
            .map(|member| member.slot)
            .collect::<Vec<_>>();
        let mut recovered_hp = 0;
        let mut recovered_mana = 0;
        let mut world_damage_ticks = 0;
        let mut last_world_damage = None;
        for _ in 0..hours {
            for _ in 0..REST_WATCH_TICKS_PER_HOUR {
                self.advance_turn_with_minutes(REST_WATCH_MINUTES_PER_TICK);
                let (hp, mana) = self.apply_rest_recovery_tick();
                recovered_hp += hp;
                recovered_mana += mana;
                if let (Some(game_dir), Area::World { plane }) = (game_dir, self.area) {
                    if let Some(report) = self.apply_world_underfoot_damage(game_dir, plane)? {
                        world_damage_ticks += 1;
                        last_world_damage = Some(report);
                    }
                }
            }
        }
        let woke = self.wake_initial_rest_sleepers(&asleep_at_start);
        self.message = format!(
            "Party rested {hours} hour{}; recovered {recovered_hp} HP and {recovered_mana} MP; woke {woke} asleep member(s); ambush checks are out of scope.",
            if hours == 1 { "" } else { "s" },
        );
        if let Some(report) = last_world_damage {
            self.message.push_str(&format!(
                " Underfoot world damage triggered {world_damage_ticks} tick(s); last {report}."
            ));
        }
        Ok(MoveOutcome::Rested)
    }

    fn apply_rest_recovery_tick(&mut self) -> (u16, u16) {
        let mut recovered_hp = 0;
        let mut recovered_mana = 0;
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            let hp_recovery = self.rest_hp_recovery_roll(index);
            recovered_hp += self.party[index].heal_by(u16::from(hp_recovery));
            let mana_recovery = self.rest_mana_recovery_roll(index);
            recovered_mana += u16::from(self.party[index].recover_mana_by(mana_recovery));
        }
        (recovered_hp, recovered_mana)
    }

    fn rest_hp_recovery_roll(&self, member_index: usize) -> u8 {
        1 + (self.rest_hp_recovery_seed(member_index) % 4)
    }

    fn rest_mana_recovery_roll(&self, member_index: usize) -> u8 {
        1 + ((self.rest_hp_recovery_seed(member_index) ^ 0x5a) % 2)
    }

    fn rest_hp_recovery_seed(&self, member_index: usize) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (self.player.x as u8).wrapping_mul(7)
            ^ (self.player.y as u8).wrapping_mul(11)
            ^ (member_index as u8).wrapping_mul(13)
    }

    fn wake_initial_rest_sleepers(&mut self, asleep_at_start: &[u8]) -> usize {
        let mut woke = 0;
        for member in &mut self.party {
            if member.status == b'S' && member.hp > 0 && asleep_at_start.contains(&member.slot) {
                member.status = b'G';
                woke += 1;
            }
        }
        woke
    }

    fn idle_tick(&mut self) -> MoveOutcome {
        self.advance_visual_tick();
        self.message = "Idle animation tick.".to_string();
        MoveOutcome::IdleTick
    }

    fn ignite_torch(&mut self) -> MoveOutcome {
        if self.torches == 0 {
            self.message = "No torches!".to_string();
            return MoveOutcome::Blocked;
        }

        self.torches = self.torches.saturating_sub(1);
        self.advance_turn();
        match self.area {
            Area::Dungeon { .. } => {
                let added = self.dungeon_torch_duration_roll();
                self.torch_counter = self.torch_counter.saturating_add(added);
                self.recompute_daylight();
                self.message = format!(
                    "Ignited a torch; dungeon light counter is {} and {} torch(es) remain.",
                    self.torch_counter, self.torches
                );
            }
            _ => {
                self.torch_counter = SURFACE_TORCH_DURATION;
                self.recompute_daylight();
                self.message = format!(
                    "Ignited a torch; light counter is {} and {} torch(es) remain.",
                    self.torch_counter, self.torches
                );
            }
        }
        MoveOutcome::Ignited
    }

    fn klimb_command(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => {
                let tile = self.grid[self.player.y * 32 + self.player.x];
                let choices = self.connected_town_climb_choices(
                    game_dir,
                    scene,
                    floor,
                    self.player.x,
                    self.player.y,
                    tile,
                )?;
                match choices.as_slice() {
                    [] => {
                        self.message = "Not climbable!".to_string();
                        Ok(MoveOutcome::Blocked)
                    }
                    [intent] => self.climb(game_dir, *intent),
                    _ => {
                        self.message =
                            "Two-way climb: use < or > to choose a climb direction.".to_string();
                        Ok(MoveOutcome::Blocked)
                    }
                }
            }
            Area::Dungeon { level, .. } => {
                match self.dungeon_cell(level, self.player.x, self.player.y) >> 4 {
                    0x1 => self.climb(game_dir, ClimbIntent::Up),
                    0x2 => self.climb(game_dir, ClimbIntent::Down),
                    0x3 => {
                        self.message =
                            "Two-way ladder: use < or > to choose a climb direction.".to_string();
                        Ok(MoveOutcome::Blocked)
                    }
                    _ => {
                        self.message = "Not climbable!".to_string();
                        Ok(MoveOutcome::Blocked)
                    }
                }
            }
            Area::World { plane } => self.climb_outdoors(game_dir, plane),
        }
    }

    fn connected_town_climb_choices(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Vec<ClimbIntent>> {
        if let Some(entry) = self.town_stair_at(game_dir, scene, floor, x, y, tile)? {
            return self.available_town_climb_choices(game_dir, scene, floor, entry.kind.intents());
        }
        if !(80..=87).contains(&tile) {
            return Ok(Vec::new());
        }
        self.available_town_climb_choices(
            game_dir,
            scene,
            floor,
            &[ClimbIntent::Up, ClimbIntent::Down],
        )
    }

    fn available_town_climb_choices(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        candidates: &[ClimbIntent],
    ) -> io::Result<Vec<ClimbIntent>> {
        let mut choices = Vec::new();
        for intent in candidates {
            let delta = town_climb_delta(*intent);
            match load_floor(game_dir, scene, floor.saturating_add(delta)) {
                Ok(_) => choices.push(*intent),
                Err(err) if err.kind() == io::ErrorKind::InvalidInput => {}
                Err(err) => return Err(err),
            }
        }
        Ok(choices)
    }

    fn climb(&mut self, game_dir: &Path, intent: ClimbIntent) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            return self.climb_dungeon(game_dir, intent);
        };
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let delta = if let Some(entry) =
            self.town_stair_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        {
            if !entry.kind.allows(intent) {
                self.message = "Not climbable!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            town_climb_delta(intent)
        } else if let Some(delta) = stair_delta(tile, intent) {
            delta
        } else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let next_floor = floor.saturating_add(delta);
        let next_grid = match load_town_runtime_floor(game_dir, scene, next_floor, self.clock.hour)
        {
            Ok(grid) => grid,
            Err(err) if err.kind() == io::ErrorKind::InvalidInput => {
                self.message = "No connected floor in this slice.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            Err(err) => return Err(err),
        };
        self.grid = next_grid;
        self.area = Area::Town {
            scene,
            floor: next_floor,
        };
        self.clear_town_floor_reload_door_state();
        self.restore_revealed_town_secret_doors_for_floor(game_dir, scene, next_floor)?;
        self.relink_npc_objects();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Changed to {} floor {}.", scene.key(), next_floor);
        Ok(MoveOutcome::Transition(AreaTransition::ChangedFloor {
            scene,
            floor: next_floor,
        }))
    }

    fn climb_dungeon(&mut self, game_dir: &Path, intent: ClimbIntent) -> io::Result<MoveOutcome> {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let tile = self.dungeon_cell(level, self.player.x, self.player.y);
        let Some(delta) = dungeon_ladder_delta(tile, intent) else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let next_level = level as i8 + delta;
        if next_level < 0 {
            if self.restore_return_world() {
                self.advance_turn();
                self.message = format!(
                    "Exited {} ({}) to overworld debug return point.",
                    scene.key(),
                    scene.name()
                );
            } else if self.restore_world_for_target(game_dir, PlayTarget::Dungeon(scene))? {
                self.advance_turn();
                self.message = format!(
                    "Exited {} ({}) to world-location table return point.",
                    scene.key(),
                    scene.name()
                );
            } else {
                return Ok(self.block_missing_dungeon_return(
                    scene,
                    level,
                    format!("Exited {} ({})", scene.key(), scene.name()),
                ));
            }
            self.mark_visibility_dirty();
            return Ok(MoveOutcome::Transition(AreaTransition::ExitedDungeon(
                scene,
            )));
        }
        if next_level > 7 {
            if let Some(entry) = self.dungeon_deeper_transition_at(
                game_dir,
                scene,
                level,
                self.player.x,
                self.player.y,
            )? {
                self.advance_turn();
                self.apply_dungeon_deeper_transition(game_dir, entry)?;
                return Ok(MoveOutcome::Transition(
                    AreaTransition::ExitedDungeonToWorldPlane {
                        scene,
                        plane: entry.to_plane,
                    },
                ));
            }
            self.message = "No connected deeper level in this slice.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let next_level = next_level as u8;
        self.area = Area::Dungeon {
            scene,
            level: next_level,
        };
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Changed to {} ({}) level {next_level}.",
            scene.key(),
            scene.name()
        );
        Ok(MoveOutcome::Transition(
            AreaTransition::ChangedDungeonLevel {
                scene,
                level: next_level,
            },
        ))
    }

    fn dungeon_deeper_transition_at(
        &self,
        game_dir: &Path,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
    ) -> io::Result<Option<DungeonDeeperTransitionEntry>> {
        Ok(
            load_dungeon_deeper_transition_entries(game_dir)?.and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| {
                        entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
                    })
                    .copied()
            }),
        )
    }

    fn apply_dungeon_deeper_transition(
        &mut self,
        game_dir: &Path,
        entry: DungeonDeeperTransitionEntry,
    ) -> io::Result<()> {
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
        self.message = format!(
            "Descended from {} ({}) through a scripted deeper transition to {} at ({}, {}). {}.",
            entry.scene.key(),
            entry.scene.name(),
            entry.to_plane.key(),
            entry.to_x,
            entry.to_y,
            self.wind.status_message()
        );
        Ok(())
    }

    fn enter_current_location(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let Area::World { plane } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        if let Some(entry) = self.moongate_at(plane, self.player.x, self.player.y) {
            return self.enter_moongate(game_dir, plane, entry);
        }

        if let Some(entries) = load_world_location_entries(game_dir)? {
            let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
            if let Some(entry) = entries.iter().find(|entry| {
                entry.plane == plane
                    && entry.x == self.player.x
                    && entry.y == self.player.y
                    && match entry.expected_tile {
                        Some(expected) => expected == tile,
                        None => true,
                    }
            }) {
                return self.enter_world_target(
                    game_dir,
                    plane,
                    entry.target,
                    entry.town_entry_y,
                    false,
                );
            }
            self.message = format!(
                "No entry in {WORLD_LOCATION_TABLE_FILE} for {} at ({}, {}).",
                plane.key(),
                self.player.x,
                self.player.y
            );
            return Ok(MoveOutcome::Blocked);
        }

        let Some(target) = self.debug_enter else {
            self.message = format!(
                "No clean-room entrance coordinate table is available for {} at ({}, {}).",
                plane.key(),
                self.player.x,
                self.player.y
            );
            return Ok(MoveOutcome::Blocked);
        };
        self.enter_world_target(game_dir, plane, target, None, true)
    }

    fn enter_moongate(
        &mut self,
        game_dir: &Path,
        from_plane: WorldPlane,
        entry: MoongateEntry,
    ) -> io::Result<MoveOutcome> {
        self.advance_turn();
        self.apply_moongate(game_dir, from_plane, entry)
    }

    fn apply_moongate(
        &mut self,
        game_dir: &Path,
        from_plane: WorldPlane,
        entry: MoongateEntry,
    ) -> io::Result<MoveOutcome> {
        self.pending_moongate = None;
        if entry.is_single_ended() {
            self.message = "Entered moongate, but it has no destination.".to_string();
            return Ok(MoveOutcome::Observed);
        }
        let to_plane = entry.destination_plane;
        if to_plane != from_plane {
            self.cache_current_world_overlay();
            self.area = Area::World { plane: to_plane };
            self.force_foot_transport();
            self.grid = load_world_map(game_dir, to_plane)?;
            self.npcs.clear();
            self.replace_world_active_objects(
                game_dir,
                to_plane,
                entry.destination_x,
                entry.destination_y,
            )?;
            self.clear_open_town_door_state();
            self.return_world = None;
            self.pending_moongate = None;
        }
        self.player.x = entry.destination_x;
        self.player.y = entry.destination_y;
        self.sync_player_object();
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        self.message = format!(
            "Entered moongate to {} at ({}, {}). {}.",
            to_plane.key(),
            entry.destination_x,
            entry.destination_y,
            self.wind.status_message()
        );
        Ok(MoveOutcome::Transition(
            AreaTransition::MoongateTeleported {
                from: from_plane,
                to: to_plane,
            },
        ))
    }

    fn resolve_moongate_prompt(
        &mut self,
        key: char,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entry) = self.pending_moongate else {
            return Ok(None);
        };
        match key {
            'y' | 'Y' => {
                let Area::World { plane } = self.area else {
                    self.pending_moongate = None;
                    self.message = "Moongate prompt cancelled outside the overworld.".to_string();
                    return Ok(Some(MoveOutcome::Blocked));
                };
                self.apply_moongate(game_dir, plane, entry).map(Some)
            }
            'n' | 'N' => {
                self.pending_moongate = None;
                self.message = "Moongate ignored.".to_string();
                Ok(Some(MoveOutcome::PromptDeclined))
            }
            _ => {
                self.message = "Enter moongate? (Y/N).".to_string();
                Ok(Some(MoveOutcome::Blocked))
            }
        }
    }

    fn enter_world_target(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        target: PlayTarget,
        town_entry_y: Option<usize>,
        debug: bool,
    ) -> io::Result<MoveOutcome> {
        self.cache_current_world_overlay();
        let return_world = WorldReturn {
            plane,
            x: self.player.x,
            y: self.player.y,
            transport: self.player.transport,
            timing_status: self.timing_status,
            sail_cadence: self.sail_cadence,
            sail_stall_pending: self.sail_stall_pending,
            grid: self.grid.clone(),
            active_objects: self.active_objects.clone(),
        };
        let mut options = PlayOptions {
            target,
            floor: 0,
            start: None,
            clock: self.clock,
            food: self.food,
            gold: self.gold,
            keys: self.keys,
            gems: self.gems,
            climbing_gear: self.climbing_gear,
            party: self.party.clone(),
            spell_charges: self.spell_charges,
            reagents: self.reagents,
            moonstone_slots: self.moonstone_slots,
            shrine_ordained_mask: self.shrine_ordained_mask,
            shrine_codex_mask: self.shrine_codex_mask,
            shrine_standing: self.shrine_standing,
            avatar_stats: self.avatar_stats,
            torches: self.torches,
            torch_counter: self.torch_counter,
            light_spell_counter: self.light_spell_counter,
            wind: self.wind,
            wind_save_byte: self.wind_save_byte,
            timing_status: TimingStatusTag::Normal,
            time_stop_counter: self.time_stop_counter,
            active_effect_tag: self.active_effect_tag,
            active_effect_counter: self.active_effect_counter,
            transport: TransportState::Foot,
            pending_vehicle: None,
            initial_britannia_overlay: None,
            debug_enter: self.debug_enter,
            saved_active_objects: None,
            save_template_source: self.save_template_source,
        };
        let mut next = match target {
            PlayTarget::Town(scene) => {
                options.start = town_entry_y
                    .map(|entry_y| Ok(Some((15, entry_y))))
                    .unwrap_or_else(|| {
                        load_location_entry_y(game_dir, scene)
                            .map(|entry_y| entry_y.map(|y| (15, y)))
                    })?;
                Self::load_town_scene(game_dir, scene, options)?
            }
            PlayTarget::Dungeon(scene) => {
                options.floor = match plane {
                    WorldPlane::Britannia => 0,
                    WorldPlane::Underworld if scene.record == 7 => 0,
                    WorldPlane::Underworld => 7,
                };
                options.start = match plane {
                    WorldPlane::Britannia => Some((1, 1)),
                    WorldPlane::Underworld if scene.record == 7 => Some((1, 1)),
                    WorldPlane::Underworld => Some((7, 7)),
                };
                let mut dungeon = Self::load_dungeon_scene(game_dir, scene, options)?;
                if matches!(plane, WorldPlane::Underworld) && scene.record != 7 {
                    dungeon.player.facing = Direction::West;
                }
                dungeon
            }
            PlayTarget::World(_) => {
                self.message = "World enter target must be a town or dungeon scene.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        };
        // Interior play starts on foot; the return snapshot owns outside transport.
        next.force_foot_transport();
        next.sync_player_object();
        next.turn = self.turn;
        next.return_world = Some(return_world);
        next.world_overlays = self.world_overlays.clone();
        next.message = match target {
            PlayTarget::Town(scene) if debug => {
                format!("Debug-entered {} from {}.", scene.key(), plane.key())
            }
            PlayTarget::Town(scene) => format!("Entered {} from {}.", scene.key(), plane.key()),
            PlayTarget::Dungeon(scene) if debug => {
                format!(
                    "Debug-entered {} ({}) from {}.",
                    scene.key(),
                    scene.name(),
                    plane.key()
                )
            }
            PlayTarget::Dungeon(scene) => {
                format!(
                    "Entered {} ({}) from {}.",
                    scene.key(),
                    scene.name(),
                    plane.key()
                )
            }
            PlayTarget::World(_) => unreachable!(),
        };
        *self = next;
        Ok(match target {
            PlayTarget::Town(scene) => {
                MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
            }
            PlayTarget::Dungeon(scene) => {
                MoveOutcome::Transition(AreaTransition::EnteredDungeon(scene))
            }
            PlayTarget::World(_) => unreachable!(),
        })
    }

    fn restore_return_world(&mut self) -> bool {
        let Some(return_world) = self.return_world.take() else {
            return false;
        };
        let plane = return_world.plane;
        self.area = Area::World { plane };
        self.player.x = return_world.x;
        self.player.y = return_world.y;
        self.player.transport = return_world.transport;
        self.timing_status = return_world.timing_status;
        self.sail_cadence = return_world.sail_cadence;
        self.sail_stall_pending = return_world.sail_stall_pending;
        self.grid = return_world.grid;
        self.npcs.clear();
        self.active_objects = return_world.active_objects;
        self.sync_player_object();
        self.cache_current_world_overlay();
        self.clear_open_town_door_state();
        self.pending_moongate = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        true
    }

    fn restore_world_for_target(
        &mut self,
        game_dir: &Path,
        target: PlayTarget,
    ) -> io::Result<bool> {
        let Some(entries) = load_world_location_entries(game_dir)? else {
            return Ok(false);
        };
        let matches: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| entry.target == target)
            .collect();
        let Some(entry) = matches.first().copied() else {
            return Ok(false);
        };
        if matches.len() > 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} has multiple return rows for {}",
                    target.key()
                ),
            ));
        }

        self.area = Area::World { plane: entry.plane };
        self.player.x = entry.x;
        self.player.y = entry.y;
        self.force_foot_transport();
        self.grid = load_world_map(game_dir, entry.plane)?;
        self.npcs.clear();
        self.replace_world_active_objects(game_dir, entry.plane, entry.x, entry.y)?;
        self.clear_open_town_door_state();
        self.pending_moongate = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        Ok(true)
    }

    fn moongate_at(&self, plane: WorldPlane, x: usize, y: usize) -> Option<MoongateEntry> {
        if plane != WorldPlane::Britannia {
            return None;
        }
        if !self.moongates_visible_by_light() {
            return None;
        }
        self.moongates.iter().copied().find(|entry| {
            entry.x == x
                && entry.y == y
                && entry.is_active_at(self.clock.hour)
                && self.moongate_origin_tile_matches(*entry)
        })
    }

    fn moongates_visible_by_light(&self) -> bool {
        self.ambient_light >= FULL_DAYLIGHT
    }

    fn moongate_origin_tile_matches(&self, entry: MoongateEntry) -> bool {
        entry.matches_origin_tile(self.grid[world_cell_index(entry.x, entry.y)])
    }

    fn visible_moongate_at(&self, plane: WorldPlane, x: usize, y: usize) -> bool {
        if plane != WorldPlane::Britannia || !self.moongates_visible_by_light() {
            return false;
        }

        self.moongates.iter().any(|entry| {
            entry.is_active_at(self.clock.hour)
                && self.moongate_origin_tile_matches(*entry)
                && ((entry.x == x && entry.y == y)
                    || (!entry.is_single_ended()
                        && entry.destination_plane == WorldPlane::Britannia
                        && entry.destination_x == x
                        && entry.destination_y == y))
        })
    }

    fn visible_moongate_cells(&self) -> Vec<(usize, usize)> {
        if !matches!(
            self.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        ) || !self.moongates_visible_by_light()
        {
            return Vec::new();
        }

        let mut cells = Vec::new();
        for entry in self.moongates.iter().filter(|entry| {
            entry.is_active_at(self.clock.hour) && self.moongate_origin_tile_matches(**entry)
        }) {
            let origin = (entry.x, entry.y);
            if !cells.contains(&origin) {
                cells.push(origin);
            }
            if !entry.is_single_ended() && entry.destination_plane == WorldPlane::Britannia {
                let destination = (entry.destination_x, entry.destination_y);
                if !cells.contains(&destination) {
                    cells.push(destination);
                }
            }
        }
        cells
    }

    fn world_plane_transition_at(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<Option<WorldPlaneTransitionEntry>> {
        let tile = self.grid[world_cell_index(x, y)];
        Ok(
            load_world_plane_transition_entries(game_dir)?.and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| {
                        entry.from_plane == plane
                            && entry.x == x
                            && entry.y == y
                            && match entry.expected_tile {
                                Some(expected) => expected == tile,
                                None => true,
                            }
                    })
                    .copied()
            }),
        )
    }

    fn apply_world_underfoot_plane_transition(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<AreaTransition>> {
        let Some(entry) =
            self.world_plane_transition_at(game_dir, plane, self.player.x, self.player.y)?
        else {
            return Ok(None);
        };
        let to_plane = entry.to_plane;
        self.apply_world_plane_transition(game_dir, entry)?;
        Ok(Some(AreaTransition::ChangedWorldPlane {
            from: plane,
            to: to_plane,
        }))
    }

    fn world_waterfall_at(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<WorldWaterfallEntry>> {
        Ok(load_world_waterfall_entries(game_dir)?.and_then(|entries| {
            entries
                .iter()
                .find(|entry| world_waterfall_matches(**entry, plane, x, y, tile))
                .copied()
        }))
    }

    fn world_damage_tile_at(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<WorldDamageTileEntry>> {
        let Some(entries) = load_world_damage_tile_entries(game_dir)? else {
            return Ok(None);
        };
        Ok(world_damage_tile_entry_at(&entries, plane, x, y, tile))
    }

    fn append_world_damage_tile_message(
        &mut self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
    ) -> io::Result<()> {
        let Some(game_dir) = game_dir else {
            return Ok(());
        };
        if let Some(report) = self.apply_world_underfoot_damage(game_dir, plane)? {
            self.message.push_str(&format!(" {report}."));
        }
        Ok(())
    }

    fn apply_world_underfoot_damage(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<String>> {
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        let Some(entry) =
            self.world_damage_tile_at(game_dir, plane, self.player.x, self.player.y, tile)?
        else {
            return Ok(None);
        };
        if entry.effect.damages_transport(self.player.transport) {
            Ok(Some(self.apply_world_damage_tile(entry)))
        } else {
            Ok(None)
        }
    }

}
