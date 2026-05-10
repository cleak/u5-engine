impl PlayState {
    pub fn tick_door_tracker(&mut self) {
        let Some(mut tracker) = self.door_tracker else {
            return;
        };
        tracker.turns_remaining = tracker.turns_remaining.saturating_sub(1);
        if tracker.turns_remaining == 0 {
            self.grid[tracker.y * 32 + tracker.x] = tracker.previous_tile;
            if let Area::Town { scene, floor } = self.area {
                self.forget_open_town_door(scene, floor, tracker.x, tracker.y);
            }
            self.door_tracker = None;
            self.mark_visibility_dirty();
        } else {
            self.door_tracker = Some(tracker);
        }
    }

    pub fn sync_player_object(&mut self) {
        let z = match self.area {
            Area::Town { floor, .. } => floor,
            Area::Dungeon { level, .. } => level as i8,
            Area::World { plane } => plane.save_floor(),
        };
        let player_object = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: self.player.transport.avatar_tile(),
            x: self.player.x,
            y: self.player.y,
            z,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        if self.active_objects.is_empty() {
            self.active_objects.push(player_object);
            return;
        }

        if self.active_objects[0].is_player() {
            self.active_objects[0].x = player_object.x;
            self.active_objects[0].y = player_object.y;
            self.active_objects[0].z = player_object.z;
            self.active_objects[0].tile = player_object.tile;
        } else {
            self.active_objects[0] = player_object;
        }
        for object in self.active_objects.iter_mut().skip(1) {
            if object.is_player() {
                object.free();
            }
        }
    }

    pub fn current_floor(&self) -> Option<i8> {
        match self.area {
            Area::Town { floor, .. } => Some(floor),
            Area::Dungeon { level, .. } => Some(level as i8),
            Area::World { plane } => Some(plane.save_floor()),
        }
    }

    pub fn boardable_vehicle_slot(&self) -> Option<BoardVehicleCandidate> {
        let positions = self.board_probe_positions();
        positions
            .into_iter()
            .find_map(|(x, y)| self.boardable_vehicle_slot_at(x, y))
    }

    pub fn boardable_vehicle_slot_at(&self, x: usize, y: usize) -> Option<BoardVehicleCandidate> {
        let mut candidate = None;
        let mut blocked_by_occupant = false;
        for (slot, object) in self.active_objects.iter().enumerate().skip(1) {
            if !self.object_occupies(*object, x, y) {
                continue;
            }
            if let Some(transport) = transport_from_vehicle_object(
                object.type_byte,
                object.tile,
                object.aux1,
                object.aux3,
            ) {
                candidate.get_or_insert(BoardVehicleCandidate {
                    slot,
                    transport,
                    blocked_by_occupant: false,
                });
            } else {
                blocked_by_occupant = true;
            }
        }
        candidate.map(|mut candidate| {
            candidate.blocked_by_occupant = blocked_by_occupant;
            candidate
        })
    }

    pub fn board_probe_positions(&self) -> Vec<(usize, usize)> {
        let mut out = vec![(self.player.x, self.player.y)];
        let (dx, dy) = self.player.facing.delta();
        match self.area {
            Area::World { .. } => {
                out.push((
                    (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize,
                    (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize,
                ));
            }
            Area::Town { .. } => {
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if (0..32).contains(&x) && (0..32).contains(&y) {
                    out.push((x as usize, y as usize));
                }
            }
            Area::Dungeon { .. } => {}
        }
        out
    }

    pub fn vehicle_exit_landing(&self, game_dir: Option<&Path>) -> io::Result<Option<(usize, usize)>> {
        for direction in [
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::North,
            Direction::SouthEast,
            Direction::SouthWest,
            Direction::NorthEast,
            Direction::NorthWest,
        ] {
            let Some((x, y)) = self.adjacent_position(direction) else {
                continue;
            };
            if self.player_can_land_on_foot(game_dir, x, y)? {
                return Ok(Some((x, y)));
            }
        }
        Ok(None)
    }

    pub fn adjacent_position(&self, direction: Direction) -> Option<(usize, usize)> {
        let (dx, dy) = direction.delta();
        match self.area {
            Area::World { .. } => Some((
                (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize,
                (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize,
            )),
            Area::Town { .. } => {
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                ((0..32).contains(&x) && (0..32).contains(&y)).then_some((x as usize, y as usize))
            }
            Area::Dungeon { .. } => None,
        }
    }

    pub fn player_can_land_on_foot(
        &self,
        game_dir: Option<&Path>,
        x: usize,
        y: usize,
    ) -> io::Result<bool> {
        if self.object_at_current_floor(x, y).is_some() {
            return Ok(false);
        }
        match self.area {
            Area::Town { scene, floor } => {
                let tile = self.grid[y * 32 + x];
                if (80..=87).contains(&tile) {
                    return Ok(false);
                }
                if let Some(game_dir) = game_dir {
                    if self
                        .town_exit_tile_at(game_dir, scene, floor, x, y, tile)?
                        .is_some()
                    {
                        return Ok(false);
                    }
                    if self
                        .town_trap_door_at(game_dir, scene, floor, x, y, tile)?
                        .is_some()
                    {
                        return Ok(false);
                    }
                    if self
                        .town_stair_at(game_dir, scene, floor, x, y, tile)?
                        .is_some()
                    {
                        return Ok(false);
                    }
                }
                Ok(is_tile_walkable_for_transport(
                    tile,
                    self.passability.as_ref(),
                    TransportState::Foot,
                ))
            }
            Area::World { plane } => {
                let tile = self.grid[world_cell_index(x, y)];
                if self.moongate_at(plane, x, y).is_some() {
                    return Ok(false);
                }
                if let Some(game_dir) = game_dir {
                    if self
                        .world_plane_transition_at(game_dir, plane, x, y)?
                        .is_some()
                    {
                        return Ok(false);
                    }
                    if self
                        .world_waterfall_at(game_dir, plane, x, y, tile)?
                        .is_some()
                    {
                        return Ok(false);
                    }
                    if let Some(entry) = self.world_damage_tile_at(game_dir, plane, x, y, tile)? {
                        if !entry.effect.allows_transport(TransportState::Foot)
                            || entry.effect.damages_transport(TransportState::Foot)
                        {
                            return Ok(false);
                        }
                    }
                }
                Ok(is_tile_walkable_for_transport(
                    tile,
                    self.passability.as_ref(),
                    TransportState::Foot,
                ))
            }
            Area::Dungeon { .. } => Ok(false),
        }
    }

    pub fn object_at_current_floor(&self, x: usize, y: usize) -> Option<&ActiveObject> {
        self.active_objects
            .iter()
            .skip(1)
            .find(|object| self.object_occupies(**object, x, y))
    }

    pub fn object_occupies(&self, object: ActiveObject, x: usize, y: usize) -> bool {
        !object.is_empty()
            && !object.is_player_phantom()
            && self
                .current_floor()
                .map(|floor| object.x == x && object.y == y && object.z == floor)
                .unwrap_or(false)
    }

    pub fn dungeon_cell(&self, level: u8, x: usize, y: usize) -> u8 {
        self.grid[dungeon_cell_index(level, x, y)]
    }

    pub fn blocking_object_at(&self, x: usize, y: usize) -> Option<&ActiveObject> {
        self.object_at_current_floor(x, y)
    }

    pub fn sight_blocking_object_at_current_floor(&self, x: usize, y: usize) -> Option<&ActiveObject> {
        self.active_objects.iter().skip(1).find(|object| {
            self.object_occupies(**object, x, y) && surface_tile_blocks_sight(object.tile)
        })
    }

    pub fn npc_at_current_floor(&self, x: usize, y: usize) -> Option<&RuntimeNpc> {
        let floor = self.current_floor()?;
        if floor < 0 {
            return None;
        }
        let floor = floor as u8;
        self.npcs
            .iter()
            .find(|npc| npc.x == x && npc.y == y && npc.z == floor)
    }

    pub fn world_object_at(&self, x: usize, y: usize) -> Option<&ActiveObject> {
        if !matches!(self.area, Area::World { .. }) {
            return None;
        }
        self.object_at_current_floor(x, y)
    }

    pub fn load_scheduled_npcs(&mut self, slots: &[NpcSlot]) {
        self.npcs = slots
            .iter()
            .skip(1)
            .filter(|slot| slot.type_byte != 0)
            .map(|slot| RuntimeNpc::from_slot(slot, self.clock.hour))
            .collect();
        self.relink_npc_objects();
    }

    pub fn load_scheduled_npcs_from_existing_active_objects(&mut self, slots: &[NpcSlot]) {
        self.npcs = slots
            .iter()
            .skip(1)
            .filter(|slot| slot.type_byte != 0)
            .map(|slot| RuntimeNpc::from_slot(slot, self.clock.hour))
            .collect();
        self.link_npcs_to_existing_active_objects();
    }

    pub fn attach_player_phantom_npc(&mut self) {
        let Area::Town { floor, .. } = self.area else {
            return;
        };
        if floor < 0 {
            return;
        }
        let floor = floor as u8;
        if let Some(index) = self.npcs.iter().position(|npc| npc.is_player_phantom()) {
            self.npcs[index].sync_player_phantom_floor(floor, self.clock.hour);
            self.sync_npc_active_object(index, floor);
            return;
        }
        self.npcs.push(RuntimeNpc::from_player_phantom(
            self.player.x,
            self.player.y,
            floor,
            self.clock.hour,
        ));
        let index = self.npcs.len() - 1;
        if let Some(slot) = self.match_existing_npc_active_object(index, floor, &[]) {
            self.npcs[index].active_object = Some(slot);
        }
        self.sync_npc_active_object(index, floor);
    }

    pub fn link_npcs_to_existing_active_objects(&mut self) {
        let Area::Town { floor, .. } = self.area else {
            return;
        };
        if floor < 0 {
            return;
        }
        let floor = floor as u8;
        let mut claimed = vec![false; self.active_objects.len()];
        for index in 0..self.npcs.len() {
            if let Some(slot) = self.match_existing_npc_active_object(index, floor, &claimed) {
                self.npcs[index].active_object = Some(slot);
                claimed[slot] = true;
            }
        }
    }

    pub fn match_existing_npc_active_object(
        &self,
        npc_index: usize,
        floor: u8,
        claimed: &[bool],
    ) -> Option<usize> {
        let npc = self.npcs.get(npc_index)?;
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                if claimed.get(slot).copied().unwrap_or(false) {
                    return None;
                }
                active_object_matches_runtime_npc(object, npc, floor).then_some(slot)
            })
    }

    pub fn sync_npc_active_object(&mut self, index: usize, floor: u8) -> bool {
        if self.npcs[index].is_player_phantom() {
            let (x, y, z, active_object) = {
                let npc = &self.npcs[index];
                (npc.x, npc.y, npc.z, npc.active_object)
            };
            let should_link = x < 32 && y < 32 && z == floor;
            if let Some(slot) = active_object {
                if !should_link {
                    self.free_active_object_slot(slot);
                    self.npcs[index].active_object = None;
                    return true;
                }
                let object = player_phantom_active_object(x, y, z);
                if let Some(active_object) = self.active_objects.get_mut(slot) {
                    *active_object = object;
                } else if let Some(slot) = self.allocate_active_object_slot(object) {
                    self.npcs[index].active_object = Some(slot);
                } else {
                    self.npcs[index].active_object = None;
                }
                return true;
            }
            if should_link {
                let object = player_phantom_active_object(x, y, z);
                self.npcs[index].active_object = self.allocate_active_object_slot(object);
                return self.npcs[index].active_object.is_some();
            }
            return false;
        }
        let (x, y, z, type_byte, active_object) = {
            let npc = &self.npcs[index];
            (npc.x, npc.y, npc.z, npc.type_byte, npc.active_object)
        };
        let should_render =
            x < 32 && y < 32 && z == floor && (x, y) != (self.player.x, self.player.y);
        if let Some(slot) = active_object {
            if !should_render {
                self.free_active_object_slot(slot);
                self.npcs[index].active_object = None;
                return true;
            }
            let object = npc_active_object(type_byte, x, y, z);
            if let Some(active_object) = self.active_objects.get_mut(slot) {
                *active_object = object;
            } else if let Some(slot) = self.allocate_active_object_slot(object) {
                self.npcs[index].active_object = Some(slot);
            } else {
                self.npcs[index].active_object = None;
            }
            return true;
        }
        if should_render {
            let object = npc_active_object(type_byte, x, y, z);
            self.npcs[index].active_object = self.allocate_active_object_slot(object);
            return self.npcs[index].active_object.is_some();
        }
        false
    }

    pub fn relink_npc_objects(&mut self) {
        let Area::Town { floor, .. } = self.area else {
            return;
        };
        let floor = floor as u8;
        self.clear_non_player_active_objects();
        for index in 0..self.npcs.len() {
            self.npcs[index].active_object = None;
            if self.npcs[index].is_player_phantom() {
                self.npcs[index].sync_player_phantom_floor(floor, self.clock.hour);
                self.sync_npc_active_object(index, floor);
                continue;
            }
            let npc = &self.npcs[index];
            if npc.x >= 32
                || npc.y >= 32
                || npc.z != floor
                || (npc.x, npc.y) == (self.player.x, self.player.y)
            {
                continue;
            }
            let object = npc_active_object(npc.type_byte, npc.x, npc.y, npc.z);
            self.npcs[index].active_object = self.allocate_active_object_slot(object);
        }
    }

    pub fn advance_npc_schedules(&mut self) {
        let Area::Town { floor, .. } = self.area else {
            return;
        };
        let floor = floor as u8;
        let mut moved = false;
        for index in 0..self.npcs.len() {
            if self.npcs[index].is_player_phantom() {
                self.npcs[index].sync_player_phantom_floor(floor, self.clock.hour);
                self.sync_npc_active_object(index, floor);
                continue;
            }
            let wp = waypoint_for_hour(&self.npcs[index].schedule, self.clock.hour);
            let (tx, ty, tz) = self.npcs[index].waypoint_position(wp);
            if (self.npcs[index].x, self.npcs[index].y, self.npcs[index].z) == (tx, ty, tz) {
                self.npcs[index].cached_wp = wp;
                continue;
            }
            if self.npcs[index].z != floor || tz != floor {
                self.npcs[index].x = tx;
                self.npcs[index].y = ty;
                self.npcs[index].z = tz;
                self.npcs[index].cached_wp = wp;
                moved |= self.sync_npc_active_object(index, floor);
                continue;
            }
            let start = (self.npcs[index].x, self.npcs[index].y);
            let direct_step = step_toward(start, (tx, ty))
                .filter(|(nx, ny)| self.npc_can_step(index, *nx, *ny, floor));
            let Some((nx, ny)) =
                direct_step.or_else(|| self.npc_path_step(index, start, (tx, ty), floor))
            else {
                continue;
            };
            if !self.npc_can_step(index, nx, ny, floor) {
                continue;
            }
            self.npcs[index].x = nx;
            self.npcs[index].y = ny;
            moved = true;
            if (nx, ny) == (tx, ty) {
                self.npcs[index].cached_wp = wp;
            }
            self.sync_npc_active_object(index, floor);
        }
        if moved {
            self.mark_visibility_dirty();
        }
    }

    pub fn npc_path_step(
        &self,
        npc_index: usize,
        start: (usize, usize),
        target: (usize, usize),
        floor: u8,
    ) -> Option<(usize, usize)> {
        if start == target {
            return None;
        }
        let mut prev = vec![None::<(usize, usize)>; 1024];
        let mut seen = vec![false; 1024];
        let mut q = VecDeque::new();
        q.push_back(start);
        seen[start.1 * 32 + start.0] = true;
        while let Some((x, y)) = q.pop_front() {
            for (nx, ny) in neighbors(x, y) {
                let idx = ny * 32 + nx;
                if seen[idx] || !self.npc_can_step(npc_index, nx, ny, floor) {
                    continue;
                }
                seen[idx] = true;
                prev[idx] = Some((x, y));
                if (nx, ny) == target {
                    let mut first = (nx, ny);
                    while let Some(parent) = prev[first.1 * 32 + first.0] {
                        if parent == start {
                            return Some(first);
                        }
                        first = parent;
                    }
                    return None;
                }
                if q.len() < NPC_PATH_QUEUE_LIMIT {
                    q.push_back((nx, ny));
                }
            }
        }
        None
    }

}
