use std::collections::VecDeque;
use std::io;
use std::path::Path;

use crate::*;

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
        let (aux1, aux3) = match self.player.transport {
            TransportState::Ship { hull, skiffs, .. } => (hull, skiffs),
            _ => (0, 0),
        };
        let player_object = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: self.player.transport.avatar_tile(),
            x: self.player.x,
            y: self.player.y,
            z,
            phase: STEADY_PHASE,
            aux1,
            aux3,
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
            self.active_objects[0].aux1 = player_object.aux1;
            self.active_objects[0].aux3 = player_object.aux3;
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

    pub fn vehicle_exit_has_nearby_support(&self, game_dir: Option<&Path>) -> io::Result<bool> {
        for direction in [
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::North,
        ] {
            let Some((x, y)) = self.adjacent_position(direction) else {
                continue;
            };
            if self.vehicle_exit_support_at(game_dir, x, y)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn vehicle_exit_current_position_if_accepted(
        &self,
        game_dir: Option<&Path>,
    ) -> io::Result<Option<(usize, usize)>> {
        let nearby_support = self.vehicle_exit_has_nearby_support(game_dir)?;
        let current = (self.player.x, self.player.y);
        let accepted = match self.player.transport {
            TransportState::Horse { .. } => true,
            TransportState::Carpet { .. } => {
                nearby_support || self.player_can_land_on_foot(game_dir, current.0, current.1)?
            }
            TransportState::Skiff { .. } => {
                nearby_support && self.current_surface_tile() != Some(BRIT_DEEP_WATER_TILE)
            }
            TransportState::Ship {
                sails_hoisted: false,
                ..
            } => nearby_support,
            TransportState::Balloon { .. } => {
                nearby_support || self.player_can_land_on_foot(game_dir, current.0, current.1)?
            }
            TransportState::Foot
            | TransportState::Ship {
                sails_hoisted: true,
                ..
            } => false,
        };
        Ok(accepted.then_some(current))
    }

    pub fn vehicle_exit_support_at(
        &self,
        game_dir: Option<&Path>,
        x: usize,
        y: usize,
    ) -> io::Result<bool> {
        if self.vehicle_exit_object_support_at(x, y) {
            return Ok(true);
        }
        self.player_can_land_on_foot(game_dir, x, y)
    }

    pub fn vehicle_exit_object_support_at(&self, x: usize, y: usize) -> bool {
        self.active_objects
            .iter()
            .copied()
            .skip(1)
            .any(|object| self.object_occupies(object, x, y) && vehicle_exit_object_support(object))
    }

    pub fn current_surface_tile(&self) -> Option<u8> {
        match self.area {
            Area::Town { .. } => Some(self.grid[self.player.y * 32 + self.player.x]),
            Area::World { .. } => Some(self.grid[world_cell_index(self.player.x, self.player.y)]),
            Area::Dungeon { .. } => None,
        }
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

    pub fn object_slot_at_current_floor(
        &self,
        x: usize,
        y: usize,
    ) -> Option<(usize, &ActiveObject)> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, object)| self.object_occupies(**object, x, y))
    }

    pub fn object_at_current_floor(&self, x: usize, y: usize) -> Option<&ActiveObject> {
        self.object_slot_at_current_floor(x, y)
            .map(|(_, object)| object)
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

    pub fn sight_blocking_object_at_current_floor(
        &self,
        x: usize,
        y: usize,
    ) -> Option<&ActiveObject> {
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
        self.world_object_slot_at(x, y).map(|(_, object)| object)
    }

    pub fn world_object_slot_at(&self, x: usize, y: usize) -> Option<(usize, &ActiveObject)> {
        if !matches!(self.area, Area::World { .. }) {
            return None;
        }
        self.object_slot_at_current_floor(x, y)
    }

    pub fn load_scheduled_npcs(&mut self, slots: &[NpcSlot]) {
        let removed = self.removed_town_npc_markers_for_current_scene();
        self.npcs = slots
            .iter()
            .skip(1)
            .filter(|slot| {
                slot.type_byte != 0
                    && !(town_npc_activation_mask_eligible(slot.type_byte)
                        && removed.contains(&slot.slot))
            })
            .map(|slot| RuntimeNpc::from_slot(slot, self.clock.hour))
            .collect();
        self.relink_npc_objects();
    }

    pub fn load_scheduled_npcs_from_existing_active_objects(&mut self, slots: &[NpcSlot]) {
        let removed = self.removed_town_npc_markers_for_current_scene();
        self.npcs = slots
            .iter()
            .skip(1)
            .filter(|slot| {
                slot.type_byte != 0
                    && !(town_npc_activation_mask_eligible(slot.type_byte)
                        && removed.contains(&slot.slot))
            })
            .map(|slot| RuntimeNpc::from_slot(slot, self.clock.hour))
            .collect();
        self.link_npcs_to_existing_active_objects();
    }

    pub fn removed_town_npc_markers_for_current_scene(&self) -> Vec<usize> {
        let Area::Town { scene, floor } = self.area else {
            return Vec::new();
        };
        self.removed_town_npcs
            .iter()
            .filter_map(|(entry_scene, entry_floor, slot)| {
                (*entry_scene == scene.byte && *entry_floor == floor).then_some(*slot)
            })
            .collect()
    }

    pub fn mark_removed_town_npc_once(&mut self, scene: Scene, floor: i8, slot: usize) -> bool {
        let marker = (scene.byte, floor, slot);
        if self.removed_town_npcs.contains(&marker) {
            return false;
        }
        self.removed_town_npcs.push(marker);
        true
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
        let scene_byte = match self.area {
            Area::Town { scene, .. } => scene.byte,
            _ => 0,
        };
        let (x, y, z, type_byte, npc_slot, active_object) = {
            let npc = &self.npcs[index];
            (
                npc.x,
                npc.y,
                npc.z,
                npc.type_byte,
                npc.slot,
                npc.active_object,
            )
        };
        let should_render =
            x < 32 && y < 32 && z == floor && (x, y) != (self.player.x, self.player.y);
        if let Some(slot) = active_object {
            if !should_render {
                self.free_active_object_slot(slot);
                self.npcs[index].active_object = None;
                return true;
            }
            let mut object = npc_active_object(type_byte, x, y, z);
            if npc_hidden_sprite_slot(scene_byte, npc_slot) {
                object.tile = NPC_HIDDEN_SPRITE_TILE;
            }
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
            let mut object = npc_active_object(type_byte, x, y, z);
            if npc_hidden_sprite_slot(scene_byte, npc_slot) {
                object.tile = NPC_HIDDEN_SPRITE_TILE;
            }
            self.npcs[index].active_object = self.allocate_active_object_slot(object);
            return self.npcs[index].active_object.is_some();
        }
        false
    }

    pub fn relink_npc_objects(&mut self) {
        let Area::Town { scene, floor } = self.area else {
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
            let mut object = npc_active_object(npc.type_byte, npc.x, npc.y, npc.z);
            if npc_hidden_sprite_slot(scene.byte, npc.slot) {
                object.tile = NPC_HIDDEN_SPRITE_TILE;
            }
            self.npcs[index].active_object = self.allocate_active_object_slot(object);
        }
    }

    pub fn town_npc_alarm_state(
        &self,
        scene: Scene,
        floor: i8,
        npc_slot: usize,
    ) -> Option<TownNpcAlarmState> {
        self.town_npc_alarm_states
            .iter()
            .find(|marker| {
                marker.scene_byte == scene.byte
                    && marker.floor == floor
                    && marker.npc_slot == npc_slot
            })
            .map(|marker| marker.state)
    }

    pub fn set_town_npc_alarm_state(
        &mut self,
        scene: Scene,
        floor: i8,
        npc_slot: usize,
        state: TownNpcAlarmState,
    ) {
        if let Some(marker) = self.town_npc_alarm_states.iter_mut().find(|marker| {
            marker.scene_byte == scene.byte && marker.floor == floor && marker.npc_slot == npc_slot
        }) {
            marker.state = state;
            return;
        }
        self.town_npc_alarm_states.push(TownNpcAlarmMarker {
            scene_byte: scene.byte,
            floor,
            npc_slot,
            state,
        });
    }

    pub fn town_alarm_sweep(
        &mut self,
        scene: Scene,
        floor: i8,
        trigger_slot: Option<usize>,
    ) -> (usize, usize) {
        let mut fortified = 0;
        let mut fleeing = 0;
        let npc_slots: Vec<(usize, u8, bool)> = self
            .npcs
            .iter()
            .filter(|npc| npc.z as i8 == floor)
            .map(|npc| (npc.slot, npc.type_byte, npc.is_player_phantom()))
            .collect();
        for (slot, type_byte, player_phantom) in npc_slots {
            let state = if player_phantom
                || trigger_slot == Some(slot)
                || town_npc_type_fortifies_on_alarm(type_byte)
                || !town_alarm_rolls_flee(scene.byte, floor, slot, type_byte, self.turn)
            {
                fortified += 1;
                TownNpcAlarmState::Fortified
            } else {
                fleeing += 1;
                TownNpcAlarmState::Fleeing
            };
            self.set_town_npc_alarm_state(scene, floor, slot, state);
        }
        (fortified, fleeing)
    }

    pub fn advance_npc_schedules(&mut self) {
        let Area::Town { scene, floor } = self.area else {
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
            let alarm_state = self.town_npc_alarm_state(scene, floor as i8, self.npcs[index].slot);
            if alarm_state == Some(TownNpcAlarmState::Pacified) {
                continue;
            }
            if alarm_state == Some(TownNpcAlarmState::Fleeing) && self.npcs[index].z == floor {
                if let Some((nx, ny)) = self.town_npc_flee_step(index, floor) {
                    self.npcs[index].x = nx;
                    self.npcs[index].y = ny;
                    moved = true;
                    self.sync_npc_active_object(index, floor);
                }
                continue;
            }
            let raw_ai = self.npcs[index].schedule[NPC_SCHEDULE_AI_OFFSET + wp];
            let behavior = if alarm_state == Some(TownNpcAlarmState::Fortified) {
                if town_npc_type_guard_like(self.npcs[index].type_byte) {
                    Some(NpcAiBehavior::GuardOrBlock)
                } else {
                    Some(NpcAiBehavior::ApproachAndAttack)
                }
            } else {
                npc_ai_behavior(raw_ai)
            };
            if self.npcs[index].z == floor {
                if let Some(behavior) = behavior {
                    if behavior.raises_attack_event()
                        || behavior.raises_guard_event()
                        || matches!(behavior, NpcAiBehavior::FollowAtDistance)
                    {
                        if !self.town_npc_adjacent_to_player(index)
                            && self.town_npc_player_distance(index) <= TOWN_NPC_CHASE_RADIUS
                        {
                            if let Some((nx, ny)) = self.town_npc_chase_step(index, floor) {
                                self.npcs[index].x = nx;
                                self.npcs[index].y = ny;
                                moved = true;
                                self.sync_npc_active_object(index, floor);
                                continue;
                            }
                        }
                    } else if behavior.is_wander() {
                        let bounded = matches!(behavior, NpcAiBehavior::BoundedWander);
                        if let Some((nx, ny)) =
                            self.town_npc_wander_step(index, floor, tx, ty, bounded)
                        {
                            self.npcs[index].x = nx;
                            self.npcs[index].y = ny;
                            moved = true;
                            self.sync_npc_active_object(index, floor);
                            continue;
                        }
                    }
                }
            }
            if (self.npcs[index].x, self.npcs[index].y, self.npcs[index].z) == (tx, ty, tz) {
                self.npcs[index].set_settled_at_waypoint(wp);
                continue;
            }

            if self.npcs[index].state <= NPC_STATE_IDLE {
                if !npc_schedule_hour_at_boundary(
                    self.npcs[index].schedule_time_boundaries(),
                    self.clock.hour,
                ) {
                    continue;
                }
                if self.npcs[index].cached_wp == wp {
                    self.npcs[index].set_idle();
                    continue;
                }
                self.npcs[index].state = schedule_floor_state(self.npcs[index].z, tz, floor);
            }

            match self.npcs[index].state {
                NPC_STATE_REPLAY_QUEUE => {
                    if self.advance_npc_replay_queue_step(index, wp, tx, ty, tz, floor) {
                        moved = true;
                    }
                }
                NPC_STATE_INPLANE_MOVE => {
                    if self.advance_npc_in_plane_schedule_step(index, wp, tx, ty, tz, floor) {
                        moved = true;
                    }
                }
                NPC_STATE_DESCEND_TOWARD_TARGET
                | NPC_STATE_ASCEND_TOWARD_TARGET
                | NPC_STATE_CLIMB_UP_OFF_FLOOR
                | NPC_STATE_CLIMB_DOWN_OFF_FLOOR => {
                    if self.advance_npc_floor_transition_step(index, wp, tx, ty, tz, floor) {
                        moved = true;
                    }
                }
                NPC_STATE_PARKED_OFF_FLOOR => {
                    self.npcs[index].note_failed_progress();
                }
                _ => {}
            }
        }
        if moved {
            self.mark_visibility_dirty();
        }
    }

    pub fn advance_npc_in_plane_schedule_step(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
    ) -> bool {
        let start = (self.npcs[npc_index].x, self.npcs[npc_index].y);
        let target = (target_x, target_y);
        let direct_step = step_toward(start, target).filter(|(nx, ny)| {
            self.npc_can_step_toward(npc_index, *nx, *ny, floor, target_x, target_y)
        });
        if let Some((nx, ny)) = direct_step {
            return self.commit_npc_schedule_position(
                npc_index, waypoint, target_x, target_y, target_z, floor, nx, ny, target_z,
            );
        }
        let Some(route) = self.npc_path_route(npc_index, start, target, floor) else {
            self.npcs[npc_index].note_failed_progress();
            return false;
        };
        self.npcs[npc_index].set_move_queue(route);
        self.advance_npc_replay_queue_step(npc_index, waypoint, target_x, target_y, target_z, floor)
    }

    pub fn advance_npc_replay_queue_step(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
    ) -> bool {
        let start = (self.npcs[npc_index].x, self.npcs[npc_index].y);
        let Some(code) = self.npcs[npc_index].peek_move_queue_direction() else {
            self.npcs[npc_index].set_idle();
            return false;
        };
        let Some((nx, ny)) = npc_step_from_direction_code(start, code) else {
            self.npcs[npc_index].note_failed_progress();
            return false;
        };
        if !self.npc_can_step_toward(npc_index, nx, ny, floor, target_x, target_y) {
            self.npcs[npc_index].note_failed_progress();
            return false;
        }
        self.npcs[npc_index].advance_move_queue_direction();
        let moved = self.commit_npc_schedule_position(
            npc_index, waypoint, target_x, target_y, target_z, floor, nx, ny, target_z,
        );
        if (nx, ny) != (target_x, target_y) && !self.npcs[npc_index].move_queue.is_empty() {
            self.npcs[npc_index].state = NPC_STATE_REPLAY_QUEUE;
        }
        moved
    }

    pub fn commit_npc_schedule_position(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
        x: usize,
        y: usize,
        z: u8,
    ) -> bool {
        let old = (
            self.npcs[npc_index].x,
            self.npcs[npc_index].y,
            self.npcs[npc_index].z,
        );
        self.npcs[npc_index].x = x;
        self.npcs[npc_index].y = y;
        self.npcs[npc_index].z = z;
        let linked_changed = self.sync_npc_active_object(npc_index, floor);
        if (x, y, z) == (target_x, target_y, target_z) {
            self.npcs[npc_index].set_settled_at_waypoint(waypoint);
        } else {
            self.npcs[npc_index].state = schedule_floor_state(z, target_z, floor);
            self.npcs[npc_index].stuck_counter = 0;
        }
        old != (x, y, z) || linked_changed
    }

    pub fn town_npc_adjacent_to_player(&self, npc_index: usize) -> bool {
        self.town_npc_player_distance(npc_index) == 1
    }

    pub fn town_npc_player_distance(&self, npc_index: usize) -> usize {
        let npc = &self.npcs[npc_index];
        npc.x.abs_diff(self.player.x) + npc.y.abs_diff(self.player.y)
    }

    pub fn town_npc_chase_step(&self, npc_index: usize, floor: u8) -> Option<(usize, usize)> {
        let npc = &self.npcs[npc_index];
        let dx = self.player.x as isize - npc.x as isize;
        let dy = self.player.y as isize - npc.y as isize;
        let primary = if dx.abs() >= dy.abs() {
            cardinal_direction_from_sign(dx.signum(), 0)
        } else {
            cardinal_direction_from_sign(0, dy.signum())
        };
        let secondary = if dx.abs() >= dy.abs() {
            cardinal_direction_from_sign(0, dy.signum())
        } else {
            cardinal_direction_from_sign(dx.signum(), 0)
        };
        [primary, secondary]
            .into_iter()
            .flatten()
            .find_map(|direction| {
                self.town_npc_step_in_direction_toward(
                    npc_index,
                    floor,
                    direction,
                    self.player.x,
                    self.player.y,
                )
            })
    }

    pub fn town_npc_flee_step(&self, npc_index: usize, floor: u8) -> Option<(usize, usize)> {
        let mut best = None;
        let mut best_distance = self.town_npc_player_distance(npc_index);
        for direction in TOWN_NPC_CARDINAL_DIRECTIONS {
            if let Some((nx, ny)) = self.town_npc_step_in_direction_toward(
                npc_index,
                floor,
                direction,
                self.player.x,
                self.player.y,
            ) {
                let distance = nx.abs_diff(self.player.x) + ny.abs_diff(self.player.y);
                if distance > best_distance {
                    best_distance = distance;
                    best = Some((nx, ny));
                }
            }
        }
        best
    }

    pub fn town_npc_wander_step(
        &self,
        npc_index: usize,
        floor: u8,
        waypoint_x: usize,
        waypoint_y: usize,
        bounded: bool,
    ) -> Option<(usize, usize)> {
        let start =
            ((self.turn as usize) + self.npcs[npc_index].slot) % TOWN_NPC_CARDINAL_DIRECTIONS.len();
        for offset in 0..TOWN_NPC_CARDINAL_DIRECTIONS.len() {
            let direction =
                TOWN_NPC_CARDINAL_DIRECTIONS[(start + offset) % TOWN_NPC_CARDINAL_DIRECTIONS.len()];
            let Some((nx, ny)) = self.town_npc_step_in_direction_toward(
                npc_index, floor, direction, waypoint_x, waypoint_y,
            ) else {
                continue;
            };
            if bounded
                && (nx.abs_diff(waypoint_x) > TOWN_NPC_BOUNDED_WANDER_RADIUS
                    || ny.abs_diff(waypoint_y) > TOWN_NPC_BOUNDED_WANDER_RADIUS)
            {
                continue;
            }
            return Some((nx, ny));
        }
        None
    }

    pub fn town_npc_step_in_direction(
        &self,
        npc_index: usize,
        floor: u8,
        direction: Direction,
    ) -> Option<(usize, usize)> {
        let (dx, dy) = direction.delta();
        let npc = &self.npcs[npc_index];
        let nx = npc.x as isize + dx;
        let ny = npc.y as isize + dy;
        if !(0..32).contains(&nx) || !(0..32).contains(&ny) {
            return None;
        }
        let nx = nx as usize;
        let ny = ny as usize;
        self.npc_can_step(npc_index, nx, ny, floor)
            .then_some((nx, ny))
    }

    pub fn town_npc_step_in_direction_toward(
        &self,
        npc_index: usize,
        floor: u8,
        direction: Direction,
        destination_x: usize,
        destination_y: usize,
    ) -> Option<(usize, usize)> {
        let (dx, dy) = direction.delta();
        let npc = &self.npcs[npc_index];
        let nx = npc.x as isize + dx;
        let ny = npc.y as isize + dy;
        if !(0..32).contains(&nx) || !(0..32).contains(&ny) {
            return None;
        }
        let nx = nx as usize;
        let ny = ny as usize;
        self.npc_can_step_toward(npc_index, nx, ny, floor, destination_x, destination_y)
            .then_some((nx, ny))
    }

    pub fn npc_path_step(
        &self,
        npc_index: usize,
        start: (usize, usize),
        target: (usize, usize),
        floor: u8,
    ) -> Option<(usize, usize)> {
        let code = self
            .npc_path_route(npc_index, start, target, floor)?
            .into_iter()
            .next()?;
        npc_step_from_direction_code(start, code)
    }

    pub fn npc_path_route(
        &self,
        npc_index: usize,
        start: (usize, usize),
        target: (usize, usize),
        floor: u8,
    ) -> Option<Vec<u8>> {
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
                if seen[idx]
                    || !self.npc_can_step_toward(npc_index, nx, ny, floor, target.0, target.1)
                {
                    continue;
                }
                seen[idx] = true;
                prev[idx] = Some((x, y));
                if (nx, ny) == target {
                    let mut cells = vec![(nx, ny)];
                    let mut current = (nx, ny);
                    while let Some(parent) = prev[current.1 * 32 + current.0] {
                        if parent == start {
                            break;
                        }
                        cells.push(parent);
                        current = parent;
                    }
                    cells.reverse();
                    let mut route = Vec::with_capacity(cells.len());
                    let mut from = start;
                    for to in cells {
                        route.push(npc_direction_code_between(from, to)?);
                        from = to;
                    }
                    return (!route.is_empty()).then_some(route);
                }
                if q.len() < NPC_PATH_QUEUE_LIMIT {
                    q.push_back((nx, ny));
                }
            }
        }
        None
    }

    pub fn advance_npc_floor_transition_step(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
    ) -> bool {
        let npc_z = self.npcs[npc_index].z;
        if npc_z == floor {
            if target_z == floor {
                return false;
            }
            let marker = npc_floor_link_marker_for_delta(npc_z, target_z);
            let current = (self.npcs[npc_index].x, self.npcs[npc_index].y);
            if self.grid[current.1 * 32 + current.0] == marker {
                let next_z = next_floor_toward(npc_z, target_z);
                self.npcs[npc_index].z = next_z;
                if (self.npcs[npc_index].x, self.npcs[npc_index].y, next_z)
                    == (target_x, target_y, target_z)
                {
                    self.npcs[npc_index].set_settled_at_waypoint(waypoint);
                } else {
                    self.npcs[npc_index].state = schedule_floor_state(next_z, target_z, floor);
                    self.npcs[npc_index].stuck_counter = 0;
                }
                return self.sync_npc_active_object(npc_index, floor);
            }
            if let Some((nx, ny)) =
                self.npc_path_step_to_floor_link(npc_index, marker, target_x, target_y, floor)
            {
                self.npcs[npc_index].x = nx;
                self.npcs[npc_index].y = ny;
                self.sync_npc_active_object(npc_index, floor);
                self.npcs[npc_index].state = schedule_floor_state(npc_z, target_z, floor);
                self.npcs[npc_index].stuck_counter = 0;
                return true;
            }
            self.npcs[npc_index].note_failed_progress();
            return false;
        }

        if target_z == floor {
            let marker = npc_floor_link_marker_for_delta(npc_z, target_z);
            let Some((x, y)) = self.nearest_npc_floor_link_to(marker, target_x, target_y) else {
                self.npcs[npc_index].note_failed_progress();
                return false;
            };
            self.npcs[npc_index].x = x;
            self.npcs[npc_index].y = y;
            self.npcs[npc_index].z = floor;
            if (x, y) == (target_x, target_y) {
                self.npcs[npc_index].set_settled_at_waypoint(waypoint);
            } else {
                self.npcs[npc_index].state = schedule_floor_state(floor, target_z, floor);
                self.npcs[npc_index].stuck_counter = 0;
            }
            return self.sync_npc_active_object(npc_index, floor);
        }

        if (self.npcs[npc_index].x, self.npcs[npc_index].y, npc_z) == (target_x, target_y, target_z)
        {
            self.npcs[npc_index].set_settled_at_waypoint(waypoint);
        } else {
            self.npcs[npc_index].note_failed_progress();
        }
        false
    }

    pub fn npc_path_step_to_floor_link(
        &self,
        npc_index: usize,
        marker: u8,
        destination_x: usize,
        destination_y: usize,
        floor: u8,
    ) -> Option<(usize, usize)> {
        let start = (self.npcs[npc_index].x, self.npcs[npc_index].y);
        let code = self
            .npc_path_route_to_floor_link_marker(
                npc_index,
                start,
                marker,
                (destination_x, destination_y),
                floor,
            )?
            .into_iter()
            .next()?;
        npc_step_from_direction_code(start, code)
    }

    pub fn npc_path_route_to_floor_link_marker(
        &self,
        npc_index: usize,
        start: (usize, usize),
        marker: u8,
        destination: (usize, usize),
        floor: u8,
    ) -> Option<Vec<u8>> {
        if start.0 >= 32 || start.1 >= 32 {
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
                if seen[idx]
                    || !self.npc_can_step_toward_floor_link_marker(
                        npc_index,
                        nx,
                        ny,
                        marker,
                        destination,
                        floor,
                    )
                {
                    continue;
                }
                seen[idx] = true;
                prev[idx] = Some((x, y));
                if self.grid[idx] == marker {
                    let mut cells = vec![(nx, ny)];
                    let mut current = (nx, ny);
                    while let Some(parent) = prev[current.1 * 32 + current.0] {
                        if parent == start {
                            break;
                        }
                        cells.push(parent);
                        current = parent;
                    }
                    cells.reverse();
                    let mut route = Vec::with_capacity(cells.len());
                    let mut from = start;
                    for to in cells {
                        route.push(npc_direction_code_between(from, to)?);
                        from = to;
                    }
                    return (!route.is_empty()).then_some(route);
                }
                if q.len() < NPC_PATH_QUEUE_LIMIT {
                    q.push_back((nx, ny));
                }
            }
        }
        None
    }

    pub fn npc_can_step_toward_floor_link_marker(
        &self,
        npc_index: usize,
        x: usize,
        y: usize,
        marker: u8,
        destination: (usize, usize),
        floor: u8,
    ) -> bool {
        if x >= 32 || y >= 32 {
            return false;
        }
        let tile = self.grid[y * 32 + x];
        if tile != marker && !npc_path_tile_open(tile) {
            return false;
        }

        if (x, y) == (self.player.x, self.player.y) {
            return false;
        }

        let own_active_object = self.npcs[npc_index].active_object;
        !self
            .active_objects
            .iter()
            .enumerate()
            .any(|(slot, object)| {
                Some(slot) != own_active_object
                    && !object.is_empty()
                    && object.x == x
                    && object.y == y
                    && object.z == floor as i8
                    && npc_dynamic_obstacle_blocks(
                        object.x as i32,
                        object.y as i32,
                        destination.0 as i32,
                        destination.1 as i32,
                    )
            })
    }

    pub fn nearest_npc_floor_link_to(
        &self,
        marker: u8,
        target_x: usize,
        target_y: usize,
    ) -> Option<(usize, usize)> {
        self.floor_link_marker_coordinates(marker)
            .into_iter()
            .min_by_key(|(x, y)| x.abs_diff(target_x) + y.abs_diff(target_y))
    }

    pub fn floor_link_marker_coordinates(&self, marker: u8) -> Vec<(usize, usize)> {
        self.grid
            .chunks_exact(32)
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, tile)| (*tile == marker).then_some((x, y)))
            })
            .collect()
    }
}

const TOWN_NPC_CHASE_RADIUS: usize = 8;
const TOWN_NPC_BOUNDED_WANDER_RADIUS: usize = 2;
const TOWN_NPC_CARDINAL_DIRECTIONS: [Direction; 4] = [
    Direction::West,
    Direction::South,
    Direction::East,
    Direction::North,
];

fn cardinal_direction_from_sign(dx: isize, dy: isize) -> Option<Direction> {
    match (dx, dy) {
        (-1, 0) => Some(Direction::West),
        (1, 0) => Some(Direction::East),
        (0, -1) => Some(Direction::North),
        (0, 1) => Some(Direction::South),
        _ => None,
    }
}

fn npc_floor_link_marker_for_delta(from_z: u8, to_z: u8) -> u8 {
    if to_z < from_z {
        NPC_FLOOR_LINK_TILE_C8
    } else {
        NPC_FLOOR_LINK_TILE_C9
    }
}

fn next_floor_toward(from_z: u8, to_z: u8) -> u8 {
    if to_z < from_z {
        from_z.saturating_sub(1)
    } else if to_z > from_z {
        from_z.saturating_add(1)
    } else {
        from_z
    }
}

fn npc_direction_code_between(from: (usize, usize), to: (usize, usize)) -> Option<u8> {
    match (
        to.0 as isize - from.0 as isize,
        to.1 as isize - from.1 as isize,
    ) {
        (-1, 0) => Some(NPC_PATH_DIR_WEST),
        (0, 1) => Some(NPC_PATH_DIR_SOUTH),
        (1, 0) => Some(NPC_PATH_DIR_EAST),
        (0, -1) => Some(NPC_PATH_DIR_NORTH),
        _ => None,
    }
}

fn npc_step_from_direction_code(start: (usize, usize), code: u8) -> Option<(usize, usize)> {
    let (dx, dy) = npc_path_direction_offset(code);
    let nx = start.0 as isize + dx as isize;
    let ny = start.1 as isize + dy as isize;
    ((0..32).contains(&nx) && (0..32).contains(&ny)).then_some((nx as usize, ny as usize))
}

fn town_npc_type_fortifies_on_alarm(type_byte: u8) -> bool {
    town_npc_type_guard_like(type_byte) || matches!(type_byte, PLAYER_NPC_SENTINEL_TYPE | 0x00)
}

fn town_alarm_rolls_flee(scene_byte: u8, floor: i8, slot: usize, type_byte: u8, turn: u64) -> bool {
    let seed = u64::from(scene_byte)
        ^ ((floor as i64 as u64) << 8)
        ^ ((slot as u64) << 16)
        ^ ((type_byte as u64) << 24)
        ^ turn.rotate_left(7);
    seed.count_ones() % 2 == 0
}
