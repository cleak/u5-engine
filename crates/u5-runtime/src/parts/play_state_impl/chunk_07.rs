impl PlayState {
    fn consume_dungeon_chest(
        &mut self,
        entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        verb: &str,
    ) -> MoveOutcome {
        let note = self
            .apply_dungeon_chest_content(entries, scene, level, x, y, tile)
            .unwrap_or_else(|| "content/trap generator is out of scope".to_string());
        self.consume_dungeon_chest_with_note(scene, level, x, y, idx, tile, verb, &note)
    }

    fn apply_dungeon_chest_content(
        &mut self,
        entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> Option<String> {
        let entry = entries?.iter().find(|entry| {
            entry.scene == scene
                && entry.level == level
                && entry.x == x
                && entry.y == y
                && entry
                    .expected_cell
                    .map_or(true, |expected| expected == tile)
        })?;
        let mut parts = Vec::new();
        for grant in &entry.grants {
            self.apply_object_pickup(grant.kind, grant.amount);
            parts.push(format!("{} {}", grant.amount, grant.kind.label()));
        }
        Some(format!("authored chest grants {}", parts.join(", ")))
    }

    fn consume_dungeon_chest_with_note(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        verb: &str,
        note: &str,
    ) -> MoveOutcome {
        self.grid[idx] = 0x70 | (tile & 0x0f);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "{verb} dungeon chest at ({}, {}) on {} level {level}; {note}, marked visit-local passage.",
            x,
            y,
            scene.key()
        );
        MoveOutcome::ContainerOpened
    }

    fn board_vehicle(&mut self) -> MoveOutcome {
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(candidate) = self.boardable_vehicle_slot() else {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        };
        let transport = candidate.transport;
        if !self.player.transport.can_board(transport) {
            self.message = "On foot.".to_string();
            return MoveOutcome::Blocked;
        }
        if matches!(self.area, Area::Town { .. })
            && transport.is_horse()
            && candidate.blocked_by_occupant
        {
            self.message = "Nay!".to_string();
            return MoveOutcome::Blocked;
        }

        self.free_active_object_slot(candidate.slot);
        self.player.transport = transport;
        self.timing_status = TimingStatusTag::for_transport(transport);
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        let mut message = format!("Boarded {}.", transport.kind_name());
        transport.append_ship_auxiliary_warnings(&mut message);
        self.message = message;
        MoveOutcome::Boarded
    }

    fn force_foot_transport(&mut self) {
        self.player.transport = TransportState::Foot;
        self.timing_status = TimingStatusTag::Normal;
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
    }

    fn free_active_object_slot(&mut self, slot: usize) {
        if slot == 0 {
            return;
        }
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.free();
        }
    }

    fn clear_non_player_active_objects(&mut self) {
        self.sync_player_object();
        for object in self.active_objects.iter_mut().skip(1) {
            *object = ActiveObject::empty();
        }
    }

    fn clear_moonstone_pickups(&mut self, slot_index: usize) -> bool {
        let mut removed = false;
        for object in self.active_objects.iter_mut().skip(1) {
            if object.moonstone_slot_index() == Some(slot_index) {
                object.free();
                removed = true;
            }
        }
        removed
    }

    fn moonstone_pickup_exists(&self, slot_index: usize) -> bool {
        self.active_objects
            .iter()
            .any(|object| object.moonstone_slot_index() == Some(slot_index))
    }

    fn moonstone_pickup_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(object_slot, object)| {
                if self.object_occupies(*object, x, y) {
                    object
                        .moonstone_slot_index()
                        .map(|slot_index| (object_slot, slot_index))
                } else {
                    None
                }
            })
    }

    fn get_moonstone_pickup_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        let (object_slot, slot_index) = self.moonstone_pickup_at(x, y)?;
        self.free_active_object_slot(object_slot);
        self.moonstone_slots[slot_index] = MoonstoneGateSlot::invalid();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Recovered Moonstone phase {}; Gate Travel slot cleared.",
            slot_index + 1
        );
        Some(MoveOutcome::Got)
    }

    fn allocate_active_object_slot(&mut self, object: ActiveObject) -> Option<usize> {
        if self.active_objects.is_empty() {
            return None;
        }
        if let Some(slot) = self
            .active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| object.is_empty().then_some(slot))
        {
            self.active_objects[slot] = object;
            return Some(slot);
        }
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.push(object);
            return Some(self.active_objects.len() - 1);
        }
        None
    }

    #[cfg(test)]
    fn exit_vehicle(&mut self) -> MoveOutcome {
        self.exit_vehicle_with_game_dir(None)
            .expect("vehicle exit without sidecar metadata cannot fail on file I/O")
    }

    fn exit_vehicle_with_game_dir(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let transport = self.player.transport;
        if transport.is_foot() {
            self.message = "On foot!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if matches!(
            transport,
            TransportState::Ship {
                sails_hoisted: true,
                ..
            }
        ) {
            self.message = "Cannot exit while sails are hoisted.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let Some(z) = self.current_floor() else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let old_x = self.player.x;
        let old_y = self.player.y;
        if !self.vehicle_can_park_at_current_cell() {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let Some((x, y)) = self.vehicle_exit_landing(game_dir)? else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(parked) = transport.parked_object(old_x, old_y, z) else {
            self.message = "Nothing to exit.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if self.allocate_active_object_slot(parked).is_none() {
            self.message = "No active-object slot for vehicle.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.player.x = x;
        self.player.y = y;
        self.force_foot_transport();
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        let mut message = format!("{}!", transport.kind_name());
        transport.append_ship_auxiliary_warnings(&mut message);
        self.message = message;
        Ok(MoveOutcome::ExitedVehicle)
    }

    fn vehicle_can_park_at_current_cell(&self) -> bool {
        if !self.player.transport.is_balloon() {
            return true;
        }
        let tile = match self.area {
            Area::Town { .. } => self.grid[self.player.y * 32 + self.player.x],
            Area::World { .. } => self.grid[world_cell_index(self.player.x, self.player.y)],
            Area::Dungeon { .. } => return false,
        };
        !is_mountain_tile(tile) && !is_wall_or_closed_door_tile(tile)
    }

    fn toggle_sails(&mut self) -> MoveOutcome {
        let TransportState::Ship {
            type_byte,
            tile,
            sails_hoisted,
            hull,
            skiffs,
        } = self.player.transport
        else {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        };
        let next = !sails_hoisted;
        self.player.transport = TransportState::Ship {
            type_byte,
            tile,
            sails_hoisted: next,
            hull,
            skiffs,
        };
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
        self.advance_turn();
        self.message = if next {
            "Sails hoisted.".to_string()
        } else {
            "Sails furled.".to_string()
        };
        MoveOutcome::SailToggled
    }

    fn fire_command(
        &mut self,
        direction: Option<Direction>,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::World { .. } => Ok(self.fire_ship_broadside(direction)),
            Area::Town { scene, floor } => self.fire_town_source(game_dir, scene, floor),
            Area::Dungeon { .. } => {
                self.message = "What?".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    fn fire_town_source(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let Some(entries) = load_town_fire_source_entries(game_dir)? else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(source) = entries
            .iter()
            .find(|entry| {
                let entry = **entry;
                entry.scene == scene
                    && entry.floor == floor
                    && town_fire_source_is_adjacent(entry, self.player.x, self.player.y)
                    && town_fire_source_tile_matches(entry, self.grid[entry.y * 32 + entry.x])
            })
            .copied()
        else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.tick_door_tracker();
        let target = self.town_fire_target(source);
        match target {
            TownFireTarget::Object { slot, object } => {
                self.free_active_object_slot(slot);
                self.mark_visibility_dirty();
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} and hit object tile {} at ({}, {}); first-playable target removed while durability tables remain out of scope.",
                    source.x,
                    source.y,
                    source.direction.name(),
                    object.tile,
                    object.x,
                    object.y
                );
            }
            TownFireTarget::Door { x, y, tile } => {
                self.grid[y * 32 + x] = 16;
                self.record_open_town_door(scene, floor, x, y);
                self.forget_revealed_town_secret_door(scene, floor, x, y);
                self.door_tracker = None;
                self.mark_visibility_dirty();
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} and destroyed door tile {} at ({x}, {y}).",
                    source.x,
                    source.y,
                    source.direction.name(),
                    tile
                );
            }
            TownFireTarget::Wall { x, y, tile } => {
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} and hit blocking tile {} at ({x}, {y}); wall durability is out of scope.",
                    source.x,
                    source.y,
                    source.direction.name(),
                    tile
                );
            }
            TownFireTarget::None => {
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} with no target in range.",
                    source.x,
                    source.y,
                    source.direction.name()
                );
            }
        }
        Ok(MoveOutcome::Fired)
    }

    fn town_fire_target(&self, source: TownFireSourceEntry) -> TownFireTarget {
        let (dx, dy) = source.direction.delta();
        for distance in 1..=3 {
            let x = source.x as isize + dx * distance;
            let y = source.y as isize + dy * distance;
            if !(0..32).contains(&x) || !(0..32).contains(&y) {
                break;
            }
            let x = x as usize;
            let y = y as usize;
            if let Some((slot, object)) = self
                .active_objects
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, object)| self.object_occupies(**object, x, y))
                .map(|(slot, object)| (slot, *object))
            {
                return TownFireTarget::Object { slot, object };
            }

            let tile = self.grid[y * 32 + x];
            if (96..=103).contains(&tile) {
                return TownFireTarget::Door { x, y, tile };
            }
            if surface_tile_blocks_sight(tile) {
                return TownFireTarget::Wall { x, y, tile };
            }
        }
        TownFireTarget::None
    }

    fn fire_ship_broadside(&mut self, direction: Option<Direction>) -> MoveOutcome {
        if !matches!(self.area, Area::World { .. }) {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        }
        if !matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(direction) = direction else {
            self.message =
                "Fire- which direction? Use F plus a direction in this harness.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal()
            || direction == self.player.facing
            || direction.opposite_cardinal() == Some(self.player.facing)
        {
            self.message = "Fire broadsides only!".to_string();
            return MoveOutcome::Blocked;
        }

        let hit = self
            .ship_broadside_target_slot(direction)
            .map(|slot| (slot, self.active_objects[slot]));
        if let Some((slot, _)) = hit {
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
        }
        self.advance_turn();
        self.message = if let Some((_, object)) = hit {
            format!(
                "BOOOM! Ship broadside hit object tile {} at ({}, {}); first-playable target removed while durability tables remain out of scope.",
                object.tile, object.x, object.y
            )
        } else {
            format!(
                "BOOOM! Ship broadside fired {} with no target in range.",
                direction.name()
            )
        };
        MoveOutcome::Fired
    }

    fn ship_broadside_target_slot(&self, direction: Direction) -> Option<usize> {
        let (dx, dy) = direction.delta();
        for distance in 1..=3 {
            let x = (self.player.x as isize + dx * distance).rem_euclid(WORLD_SIDE as isize);
            let y = (self.player.y as isize + dy * distance).rem_euclid(WORLD_SIDE as isize);
            let x = x as usize;
            let y = y as usize;
            if let Some((slot, _)) = self
                .active_objects
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, object)| self.object_occupies(**object, x, y))
            {
                return Some(slot);
            }
        }
        None
    }

    fn push_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => self.push_town_facing(game_dir, scene, floor),
            Area::Dungeon { .. } => {
                self.message = "What?".to_string();
                Ok(MoveOutcome::Blocked)
            }
            Area::World { .. } => {
                self.message = "Nothing to push here.".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    fn push_town_facing(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let direction = self.player.facing;
        if !direction.is_cardinal() {
            self.message = "Push requires a cardinal facing direction.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        let px = tx + dx;
        let py = ty + dy;
        if !(0..32).contains(&tx)
            || !(0..32).contains(&ty)
            || !(0..32).contains(&px)
            || !(0..32).contains(&py)
        {
            self.message = "Nothing to push there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let px = px as usize;
        let py = py as usize;
        if self.blocking_object_at(tx, ty).is_some() {
            self.message = format!("Cannot push occupied tile at ({tx}, {ty}).");
            return Ok(MoveOutcome::Blocked);
        }

        let target_idx = ty * 32 + tx;
        let target_tile = self.grid[target_idx];
        let Some(entries) = load_town_pushable_entries(game_dir)? else {
            self.message = "Nothing to push there.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let pushable = entries
            .iter()
            .any(|entry| town_pushable_matches(*entry, scene, floor, tx, ty, target_tile));
        if !pushable {
            self.message = "Nothing to push there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn();
            self.message = format!("Push blocked by actor at ({px}, {py}).");
            return Ok(MoveOutcome::Blocked);
        }
        let dest_idx = py * 32 + px;
        let dest_tile = self.grid[dest_idx];
        if !self.tile_walkable(dest_tile) {
            self.advance_turn();
            self.message = format!("Push blocked by {} at ({px}, {py}).", tile_class(dest_tile));
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[target_idx] = dest_tile;
        self.grid[dest_idx] = target_tile;
        self.forget_open_town_door(scene, floor, tx, ty);
        self.forget_open_town_door(scene, floor, px, py);
        self.forget_revealed_town_secret_door(scene, floor, tx, ty);
        self.forget_revealed_town_secret_door(scene, floor, px, py);
        if self.door_tracker.is_some_and(|tracker| {
            (tracker.x == tx && tracker.y == ty) || (tracker.x == px && tracker.y == py)
        }) {
            self.door_tracker = None;
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Pushed tile {target_tile} {} from ({tx}, {ty}) to ({px}, {py}).",
            direction.name()
        );
        Ok(MoveOutcome::Pushed)
    }

    #[cfg(test)]
    fn pass_turn(&mut self) -> MoveOutcome {
        self.pass_turn_with_game_dir(None)
            .expect("sidecar-free pass cannot fail")
    }

    fn pass_turn_with_game_dir(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        let turn_before = self.turn;
        self.advance_turn();
        if self.sail_stall_pending {
            self.sail_stall_pending = false;
            self.message = "Ship remains stalled by the wind.".to_string();
        } else {
            self.message = "Passed.".to_string();
        }
        if let Some(game_dir) = game_dir {
            if let Some(outcome) =
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?
            {
                return Ok(outcome);
            }
        } else {
            self.queue_current_moongate_prompt();
        }
        Ok(MoveOutcome::Passed)
    }

    fn queue_current_moongate_prompt(&mut self) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let Some(entry) = self.moongate_at(plane, self.player.x, self.player.y) else {
            return false;
        };
        self.pending_moongate = Some(entry);
        if self.message.is_empty() {
            self.message = "Moongate! Enter? (Y/N).".to_string();
        } else {
            self.message.push_str(" Moongate! Enter? (Y/N).");
        }
        true
    }

    fn apply_top_down_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        match self.area {
            Area::World { .. } => {
                self.apply_world_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Town { .. } => {
                self.apply_town_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Dungeon { .. } => {
                self.apply_dungeon_post_turn_effects_after_turn(turn_before, game_dir)
            }
        }
    }

    fn apply_post_turn_effects_after_outcome(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
        outcome: MoveOutcome,
    ) -> io::Result<Option<MoveOutcome>> {
        if outcome.is_transition() {
            Ok(None)
        } else {
            self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)
        }
    }

    fn apply_world_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before || self.pending_moongate.is_some() {
            return Ok(None);
        }
        let Area::World { plane } = self.area else {
            return Ok(None);
        };

        let pre_effect_message = self.message.clone();
        if let Some(transition) = self.apply_world_underfoot_plane_transition(game_dir, plane)? {
            let transition_message = self.message.clone();
            self.message = format!("{pre_effect_message} {transition_message}");
            return Ok(Some(MoveOutcome::Transition(transition)));
        }
        self.append_world_damage_tile_message(Some(game_dir), plane)?;
        if let Some(slot) = self.apply_world_encounter_probe(game_dir, plane)? {
            self.message
                .push_str(&format!(" Wandering encounter spawned in slot {slot}."));
        }
        self.queue_current_moongate_prompt();
        Ok(None)
    }

    fn apply_town_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        let Area::Town { scene, floor } = self.area else {
            return Ok(None);
        };
        let tile = self.grid[self.player.y * 32 + self.player.x];
        if let Some(entry) =
            self.town_exit_tile_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        {
            let pre_effect_message = self.message.clone();
            let outcome = self.resolve_town_exit_tile_after_turn(game_dir, scene, floor, entry)?;
            let transition_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
            return Ok(Some(outcome));
        }

        let Some(entry) =
            self.town_trap_door_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        else {
            return Ok(None);
        };

        let pre_effect_message = self.message.clone();
        let outcome = self.apply_town_trap_door_transition(game_dir, scene, entry, false)?;
        let transition_message = self.message.clone();
        self.message = if pre_effect_message.is_empty() {
            transition_message
        } else {
            format!("{pre_effect_message} {transition_message}")
        };
        Ok(Some(outcome))
    }

    fn apply_dungeon_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        let Area::Dungeon { scene, level } = self.area else {
            return Ok(None);
        };
        let x = self.player.x;
        let y = self.player.y;
        let tile = self.dungeon_cell(level, x, y);
        if let Some(entry) = self.dungeon_teleport_at(Some(game_dir), scene, level, x, y, tile)? {
            let pre_effect_message = self.message.clone();
            let outcome = self.apply_dungeon_teleport_after_turn(scene, entry);
            let transition_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
            return Ok(Some(outcome));
        }
        if self.dungeon_exit_tile_at(Some(game_dir), scene, level, x, y, tile)? {
            let pre_effect_message = self.message.clone();
            let outcome = self.resolve_dungeon_exit_tile_after_turn(game_dir, scene, level)?;
            let transition_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
            return Ok(Some(outcome));
        }
        if is_dungeon_fall_trap(tile) {
            let pre_effect_message = self.message.clone();
            let outcome = self.resolve_dungeon_fall_trap_transition(
                scene,
                level,
                x,
                y,
                Some(game_dir),
                false,
            )?;
            let transition_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
            return Ok(Some(outcome));
        }
        if is_dungeon_bomb_trap(tile) {
            self.grid[dungeon_cell_index(level, x, y)] |= 0x08;
            self.mark_visibility_dirty();
            let trap_message = format!(
                "Triggered bomb trap at ({x}, {y}) on {} level {level}.",
                scene.key()
            );
            self.message = if self.message.is_empty() {
                trap_message
            } else {
                format!("{} {trap_message}", self.message)
            };
            return Ok(None);
        }
        if let Some(field) = dungeon_field_effect(tile) {
            let field_report = self.apply_dungeon_field_effect(field);
            let field_message = format!("Triggered {}; {field_report}.", field.label());
            self.message = if self.message.is_empty() {
                field_message
            } else {
                format!("{} {field_message}", self.message)
            };
            return Ok(None);
        }
        if self.dungeon_wind_tile_extinguishes_torch(Some(game_dir), scene, level, x, y, tile)? {
            self.torch_counter = 0;
            self.mark_visibility_dirty();
            let wind_message = "A breeze blows out the torch.".to_string();
            self.message = if self.message.is_empty() {
                wind_message
            } else {
                format!("{} {wind_message}", self.message)
            };
        }
        Ok(None)
    }

    fn hole_up_command(&mut self, game_dir: &Path, hours: Option<u8>) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => self.hole_up_town_command(game_dir, hours, scene, floor),
            Area::World { .. } | Area::Dungeon { .. } => {
                self.rest_with_watch(hours, Some(game_dir))
            }
        }
    }

    fn hole_up_town_command(
        &mut self,
        game_dir: &Path,
        hours: Option<u8>,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let Some(hours) = hours else {
            self.message =
                "Hole up- how many hours? Use H plus a number in this harness.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if !(1..=24).contains(&hours) {
            self.message = "Rest hours must be in 1..24.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let Some(entries) = load_town_rest_bed_entries(game_dir)? else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let allowed = entries.iter().any(|entry| {
            town_rest_bed_matches(*entry, scene, floor, self.player.x, self.player.y, tile)
        });
        if !allowed {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let mut recovered_hp = 0;
        let mut recovered_mana = 0;
        for _ in 0..hours {
            self.advance_turn_with_minutes(60);
            let (hp, mana) = self.apply_rest_recovery_tick();
            recovered_hp += hp;
            recovered_mana += mana;
        }
        self.message = format!(
            "Rested {hours} hour{} at the inn bed; recovered {recovered_hp} HP and {recovered_mana} MP; encounter interruption is out of scope.",
            if hours == 1 { "" } else { "s" }
        );
        Ok(MoveOutcome::Rested)
    }

}
