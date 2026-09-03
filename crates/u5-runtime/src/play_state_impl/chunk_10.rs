use std::collections::VecDeque;
use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    pub fn tick_door_tracker(&mut self) {
        let Some(mut tracker) = self.door_tracker else {
            return;
        };
        // A loaded town clears only the previous-tile/active byte. Preserve
        // the other three save bytes exactly; they are inert and must not
        // restore a tile or count down during this visit.
        if tracker.previous_tile == 0 {
            return;
        }
        // `doors-and-z-transitions.md §5`: "Each turn that consumes a turn
        // decrements the countdown; when it hits zero the engine writes the
        // previous-tile byte back to the saved cell and the door silently
        // re-closes."
        //
        // Runtime observation, spec silent on what happens to the block
        // afterwards: the DOS build does not clear it. A session that opened
        // one door and then took fourteen turns saved `0x03A9..0x03AC` as
        // `B8 0F 13 F6` — previous tile, X and Y all still present and the
        // countdown at `4 - 14 = -10` — so the counter keeps running past
        // zero and only the one crossing closes the door. Zeroing the block
        // on close was this engine's only divergence in those four bytes.
        tracker.turns_remaining = tracker.turns_remaining.wrapping_sub(1);
        if tracker.turns_remaining == 0 && !self.door_tracker_closed {
            self.grid[tracker.y * 32 + tracker.x] = tracker.previous_tile;
            if let Area::Town { scene, floor } = self.area {
                self.forget_open_town_door(scene, floor, tracker.x, tracker.y);
            }
            self.door_tracker_closed = true;
            self.mark_visibility_dirty();
        }
        self.door_tracker = Some(tracker);
    }

    pub fn sync_player_object(&mut self) {
        // `active-objects.md §5`: the compositor refreshes bytes 0..4 of
        // slot zero from the world-state globals "every **world**
        // frame". Combat is not a world frame: §9 has the framer back the
        // whole table up and the party seating write "arena coordinates
        // over the low records, record zero included", so during combat
        // record zero is the seated first party member - "record zero is
        // overwritten because it is the first record the party seating
        // allocates, not because combat reserves a player slot" - and the
        // framer restores the world value on exit. Refreshing it here
        // while combat is live overwrote that member's arena tile and
        // coordinates with the world avatar marker, which erased the
        // first party member from the arena on the first turn after
        // entry (play-test defect 18).
        if self.combat_active {
            return;
        }
        let z = match self.area {
            Area::Town { floor, .. } => floor,
            Area::Dungeon { level, .. } => level as i8,
            Area::World { plane } => plane.save_floor(),
        };
        let (aux1, aux3) = match self.player.transport {
            TransportState::Ship { hull, skiffs, .. } => (hull, skiffs),
            _ => (0, 0),
        };
        let marker = self.player.transport.save_marker();
        let player_object = ActiveObject {
            type_byte: marker,
            tile: marker,
            x: self.player.x,
            y: self.player.y,
            z,
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1,
            aux3,
        };
        if self.active_objects.is_empty() {
            self.active_objects.push(player_object);
            return;
        }

        // `active-objects.md §5`: index zero, not a magic type byte,
        // identifies the player. Refresh the five compositor-owned bytes
        // while retaining the record's phase field. Transport-specific
        // auxiliary bytes follow the live transport state during ordinary
        // play; the town-exit mirror path performs its narrower byte-0..4
        // synchronization directly so reloaded auxiliary bytes survive.
        self.active_objects[0].type_byte = player_object.type_byte;
        self.active_objects[0].tile = player_object.tile;
        self.active_objects[0].x = player_object.x;
        self.active_objects[0].y = player_object.y;
        self.active_objects[0].z = player_object.z;
        self.active_objects[0].aux1 = player_object.aux1;
        self.active_objects[0].aux3 = player_object.aux3;
        // `overworld.md §8.1`: the falls chain hides the marker across its
        // damage pass and the whirlpool replaces it with the whirlpool
        // sprite, both restoring it before the state commit. Re-apply the
        // override last so a world tick taken *inside* one of those
        // presentations cannot repaint the ordinary marker over it.
        if let Some(tile) = self.party_marker_tile_override {
            self.active_objects[0].tile = tile;
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
            // `vehicles.md`: the skiff's extra X-Xit rejection is the bridge
            // tile pair underfoot, not deep water. No water tile is rejected -
            // deep water, water and shoals (0x01..=0x03) are exactly what a
            // skiff sits on, so rejecting deep water blocked ordinary shoreline
            // landings. The nearby-support half of the rule was correct.
            TransportState::Skiff { .. } => {
                nearby_support
                    && !matches!(
                        self.current_surface_tile(),
                        Some(SKIFF_XIT_REJECTED_BRIDGE_FIRST..=SKIFF_XIT_REJECTED_BRIDGE_LAST)
                    )
            }
            TransportState::Ship {
                sails_hoisted: false,
                ..
            } => nearby_support,
            // `vehicles.md §11`: balloon is "catalog assets only"; the
            // X-Xit landing rule this arm carried is one of the three
            // things §11 names explicitly -- "Do not invent boarding,
            // landing, or wind-driven balloon movement" -- so it is
            // deleted rather than given to another family. §5's X-Xit
            // acceptances for horse, carpet, skiff and furled ship are
            // unchanged.
            TransportState::Foot
            | TransportState::SpriteSuppressed
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
                if is_town_stair_tile(tile) || town_klimb_underfoot_intent(tile).is_some() {
                    return Ok(false);
                }
                if let Some(game_dir) = game_dir {
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
                if let Some(game_dir) = game_dir {
                    if self
                        .world_plane_transition_at(game_dir, plane, x, y)?
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
            self.object_occupies(**object, x, y) && tile_blocks_sight_propagation(object.tile)
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

    /// `conversation.md §2` Talk status-tile filter feed
    /// (`cleak/u5-spec#44`): live tile byte the Talk command compares
    /// against the published sleeping/praying status-tile constants.
    /// Resolves the candidate NPC at `(x, y)` on the current floor,
    /// then reads its linked active-object frame byte through the
    /// shared [`active_object_frame_tile`] helper. Returns `None` when
    /// no NPC is at the cell.
    pub fn npc_live_tile_at(&self, x: usize, y: usize) -> Option<u8> {
        let npc = self.npc_at_current_floor(x, y)?;
        if let Some(slot) = npc.active_object {
            if let Some(object) = self.active_objects.get(slot) {
                return Some(
                    active_object_frame_tile(object.type_byte, object.phase).unwrap_or(object.tile),
                );
            }
        }
        // No linked active-object slot: fall back to the NPC's
        // type-byte mapped walking-pose tile via the same frame
        // helper, which is the renderer's idle pose for that NPC class.
        Some(active_object_frame_tile(npc.type_byte, 0).unwrap_or(npc.type_byte))
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
        let removed = self.removed_town_npc_mask_for_current_scene();
        self.npcs = effective_npc_slots(slots)
            .filter(|slot| {
                // town-mode.md §4: "a set bit means 'this slot is
                // permanently gone from this location; do not place it'.
                // It is read once per slot on entry." The sprite-class
                // filter governs the write path only; the read path
                // honours whatever bit the mask carries, including the
                // two hard-wired bypass writes.
                slot.type_byte != 0
                    && !1u32
                        .checked_shl(slot.slot as u32)
                        .is_some_and(|bit| removed & bit != 0)
            })
            .map(|slot| RuntimeNpc::from_slot(slot, self.clock.hour))
            .collect();
        self.apply_persisted_town_npc_mutations();
        self.relink_npc_objects();
    }

    pub fn load_scheduled_npcs_from_existing_active_objects(&mut self, slots: &[NpcSlot]) {
        let removed = self.removed_town_npc_mask_for_current_scene();
        self.npcs = effective_npc_slots(slots)
            .filter(|slot| {
                // town-mode.md §4: "a set bit means 'this slot is
                // permanently gone from this location; do not place it'.
                // It is read once per slot on entry." The sprite-class
                // filter governs the write path only; the read path
                // honours whatever bit the mask carries, including the
                // two hard-wired bypass writes.
                slot.type_byte != 0
                    && !1u32
                        .checked_shl(slot.slot as u32)
                        .is_some_and(|bit| removed & bit != 0)
            })
            .map(|slot| RuntimeNpc::from_slot(slot, self.clock.hour))
            .collect();
        self.apply_persisted_town_npc_mutations();
        self.link_npcs_to_existing_active_objects();
    }

    fn apply_persisted_town_npc_mutations(&mut self) {
        let Area::Town { scene, .. } = self.area else {
            return;
        };
        for mutation in self
            .town_npc_mutations
            .iter()
            .copied()
            .filter(|mutation| mutation.scene_byte == scene.byte)
        {
            if let Some(npc) = self
                .npcs
                .iter_mut()
                .find(|npc| npc.slot == mutation.npc_slot)
            {
                mutation.apply_to(npc);
            }
        }
    }

    pub(crate) fn record_town_npc_mutation(&mut self, npc_index: usize) {
        let Area::Town { scene, .. } = self.area else {
            return;
        };
        let Some(npc) = self.npcs.get(npc_index) else {
            return;
        };
        upsert_town_npc_mutation(
            &mut self.town_npc_mutations,
            TownNpcMutation::from_runtime(scene, npc),
        );
    }

    pub fn removed_town_npc_mask_for_current_scene(&self) -> u32 {
        let Area::Town { scene, .. } = self.area else {
            return 0;
        };
        self.removed_town_npc_flags
            .get(&scene.byte)
            .copied()
            .unwrap_or(0)
    }

    pub fn mark_removed_town_npc_once(&mut self, scene: Scene, slot: usize) -> bool {
        let Some(bit) = 1u32.checked_shl(slot as u32) else {
            return false;
        };
        let mask = self.removed_town_npc_flags.entry(scene.byte).or_insert(0);
        let was_clear = *mask & bit == 0;
        *mask |= bit;
        was_clear
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

    /// `npc-schedules.md §12`, "**The clear-and-re-place pass.**"
    ///
    /// "Three callers run the same pass that clears every non-party
    /// active-object record and then re-places each live scheduled NPC at the
    /// position its schedule gives for the *current* hour: entering the
    /// location (the initialisation described in Section 4), the H-Hole-Up
    /// hours command when it finishes its rested hours, and an inn's
    /// rest-for-the-night, which ends at 06:00 and re-places the roster for
    /// that hour before returning control (`systems/shops.md` Section 8.4).
    /// After any of the three, an NPC that was mid-route is standing on its
    /// scheduled position rather than where it had walked to, and its queued
    /// route is gone."
    ///
    /// Town entry reaches the same result through
    /// [`Self::load_scheduled_npcs`], which builds every runtime record from
    /// the entry hour and then relinks; this is the pass for the two rest
    /// callers, which keep their existing records.
    pub fn clear_and_replace_scheduled_npcs(&mut self) {
        if !matches!(self.area, Area::Town { .. }) {
            return;
        }
        let hour = self.clock.hour;
        for index in 0..self.npcs.len() {
            let waypoint = waypoint_for_hour(&self.npcs[index].schedule, hour);
            let (x, y, z) = self.npcs[index].waypoint_position(waypoint);
            self.npcs[index].x = x;
            self.npcs[index].y = y;
            self.npcs[index].z = z;
            // Writes the cache, returns the NPC to idle and discards any
            // queued route - the three things §6 says both arrival paths do
            // together.
            self.npcs[index].set_settled_at_waypoint(waypoint);
        }
        self.relink_npc_objects();
        self.mark_visibility_dirty();
    }

    pub fn relink_npc_objects(&mut self) {
        let Area::Town { scene, floor } = self.area else {
            return;
        };
        let floor = floor as u8;
        self.clear_non_player_active_objects();
        for index in 0..self.npcs.len() {
            self.npcs[index].active_object = None;
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

    pub fn town_alarm_sweep(
        &mut self,
        _scene: Scene,
        _floor: i8,
        _trigger_slot: Option<usize>,
    ) -> (usize, usize) {
        let non_special_count = self
            .npcs
            .iter()
            .filter(|npc| {
                !matches!(
                    npc.type_byte,
                    SHADOWLORD_ACTOR_TILE | TOWN_NPC_ALARM_LICH_TYPE | TOWN_NPC_ALARM_GUARD_TYPE
                )
            })
            .count();
        let draws = (0..non_special_count)
            .map(|_| self.random_range_u8(0, u8::MAX))
            .collect::<Vec<_>>();
        self.town_alarm_sweep_with_draws(&draws)
    }

    pub fn town_alarm_sweep_with_draws(&mut self, draws: &[u8]) -> (usize, usize) {
        let mut pursued = 0;
        let mut fled = 0;
        let mut draws = draws.iter().copied();
        for index in 0..self.npcs.len() {
            let type_byte = self.npcs[index].type_byte;
            if matches!(
                type_byte,
                SHADOWLORD_ACTOR_TILE | TOWN_NPC_ALARM_LICH_TYPE | TOWN_NPC_ALARM_GUARD_TYPE
            ) {
                self.npcs[index].force_town_pursuit();
                self.record_town_npc_mutation(index);
                pursued += 1;
            } else {
                let roll = draws
                    .next()
                    .expect("one alarm draw is required for every non-special occupied actor");
                if roll <= 127 && self.npcs[index].force_town_flight() {
                    self.record_town_npc_mutation(index);
                    fled += 1;
                }
            }
        }
        (pursued, fled)
    }

    pub fn advance_npc_schedules(&mut self) {
        let Area::Town { floor, .. } = self.area else {
            return;
        };
        let floor = floor as u8;
        let mut moved = false;
        // npc-schedules.md §7: "At most one NPC per tick may start a fresh
        // search. The walker latches a 'someone already moved' flag on the
        // first slot that enters a search arm, and every later slot in the
        // same tick that would have searched is skipped until the next
        // tick. Queue replay is not affected by the latch."
        let mut searched = false;
        for index in 0..self.npcs.len() {
            let wp = waypoint_for_hour(&self.npcs[index].schedule, self.clock.hour);
            let (tx, ty, tz) = self.npcs[index].waypoint_position(wp);
            // `npc-schedules.md §5` step 5: the boundary trigger is called
            // only on idle NPCs ("If state is 'idle' (state <= 1) ..."); a
            // slot already in a movement state goes straight to the §7 state
            // machine and never reaches the §9 AI dispatch.
            let settled = self.npcs[index].state <= NPC_STATE_IDLE;
            let at_boundary = npc_schedule_hour_at_boundary(
                self.npcs[index].schedule_time_boundaries(),
                self.clock.hour,
            );
            // `npc-schedules.md §6`, "What a boundary hour does to a
            // wandering NPC": "The **direct** route to the AI dispatch of
            // Section 9 is taken only when the trigger reports 'no action' -
            // that is, only while the current hour matches none of the NPC's
            // own four `time` bytes. ... For the whole of an hour that *does*
            // equal one of them, a settled NPC is routed into the
            // route/state machine of Section 7 instead, so a wander-AI NPC
            // normally stands still for that entire hour, whether or not its
            // waypoint actually changed."
            //
            // Of the two published escapes this fork is the first: "**The
            // tick's heavy-work budget was already spent.** The walker allows
            // one expensive replan/route operation per pass. If a
            // lower-numbered NPC already consumed it, a diverted NPC takes
            // the cheap arm instead, which re-checks the floor and then
            // enters the AI dispatch after all - using its **cached**
            // waypoint index rather than the freshly resolved one. Such an
            // NPC wanders normally on that turn." The second escape - "the
            // waypoint actually changed at the boundary" - needs no code
            // here: that NPC leaves the idle state below and route-walks
            // under §7. "An engine that models the boundary hour as a hard
            // freeze will be right most of the time and will diverge in a
            // crowded location, which is exactly where the difference is
            // visible."
            let ai_waypoint = if !settled {
                None
            } else if !at_boundary {
                Some(wp)
            } else if searched {
                Some(self.npcs[index].cached_wp)
            } else {
                None
            };
            if let Some(ai_wp) = ai_waypoint {
                let (ai_tx, ai_ty, _) = self.npcs[index].waypoint_position(ai_wp);
                let raw_ai = self.npcs[index].schedule[NPC_SCHEDULE_AI_OFFSET + ai_wp];
                let behavior = npc_ai_behavior(raw_ai);
                if behavior == Some(NpcAiBehavior::Retreating)
                    && self.npcs[index].z == floor
                    && self.town_npc_player_distance(index) <= TOWN_NPC_CHASE_RADIUS
                {
                    if let Some((nx, ny)) = self.town_npc_flee_step(index, floor) {
                        self.npcs[index].x = nx;
                        self.npcs[index].y = ny;
                        moved = true;
                        self.sync_npc_active_object(index, floor);
                    }
                    continue;
                }
                if self.npcs[index].z == floor {
                    if let Some(behavior) = behavior {
                        // `npc-schedules.md §9` value `4`, corrected by
                        // **R317**: "While the player is four or more tiles
                        // from the *waypoint*, the NPC takes the ordinary
                        // bounded wander step with the **same constant cap of
                        // three** that value `1` uses; when the player is
                        // closer than that, it enters the engagement path".
                        // "The distance this mode measures is used only to
                        // choose between wandering and engaging and is then
                        // discarded; the wander radius is a constant,
                        // identical to value `1`, and never narrows as the
                        // player closes."
                        let approach_wanders = behavior == NpcAiBehavior::ApproachAndAttack
                            && self.town_player_distance_to(ai_tx, ai_ty)
                                >= TOWN_NPC_APPROACH_ENGAGE_WAYPOINT_DISTANCE;
                        if approach_wanders {
                            if let Some((nx, ny)) = self.town_npc_wander_step(
                                index,
                                floor,
                                ai_tx,
                                ai_ty,
                                TOWN_NPC_BOUNDED_WANDER_CAP,
                            ) {
                                self.npcs[index].x = nx;
                                self.npcs[index].y = ny;
                                moved = true;
                                self.sync_npc_active_object(index, floor);
                                continue;
                            }
                        } else if behavior.raises_attack_event() || behavior.raises_guard_event() {
                            let unconditional_chase = matches!(
                                behavior,
                                NpcAiBehavior::ReservedEngage | NpcAiBehavior::RandomChase
                            );
                            if !self.town_npc_adjacent_to_player(index)
                                && (unconditional_chase
                                    || self.town_npc_player_distance(index)
                                        <= TOWN_NPC_CHASE_RADIUS)
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
                            // §9 value `2`: "the same one-attempt-per-turn
                            // coin and the same single direction draw, with
                            // the waypoint-radius test switched off entirely
                            // (cap zero)."
                            let cap = if matches!(behavior, NpcAiBehavior::BoundedWander) {
                                TOWN_NPC_BOUNDED_WANDER_CAP
                            } else {
                                TOWN_NPC_UNBOUNDED_WANDER_CAP
                            };
                            if let Some((nx, ny)) =
                                self.town_npc_wander_step(index, floor, ai_tx, ai_ty, cap)
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
            }
            if (self.npcs[index].x, self.npcs[index].y, self.npcs[index].z) == (tx, ty, tz) {
                self.npcs[index].set_settled_at_waypoint(wp);
                continue;
            }

            if self.npcs[index].state <= NPC_STATE_IDLE {
                // `npc-schedules.md §6` "Boundary equality": the trigger
                // "compares the current hour byte against each of the NPC's
                // four `time` bytes. If none match exactly, the trigger
                // returns 'no action' - the NPC stays idle."
                if !at_boundary {
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
                    match self.advance_npc_replay_queue_step(index, wp, tx, ty, tz, floor) {
                        NpcScheduleStepOutcome::Moved => moved = true,
                        NpcScheduleStepOutcome::Stalled => {}
                        // npc-schedules.md §7: the queue-drain re-entry into
                        // state 6/7 "also ends the tick... every slot after
                        // the one that triggered it is skipped for that tick".
                        // It is the only path in the walker that leaves the
                        // per-slot loop early.
                        NpcScheduleStepOutcome::EndTick => break,
                    }
                }
                NPC_STATE_INPLANE_MOVE => {
                    match self.advance_npc_in_plane_schedule_step(
                        index,
                        wp,
                        tx,
                        ty,
                        tz,
                        floor,
                        &mut searched,
                    ) {
                        NpcScheduleStepOutcome::Moved => moved = true,
                        NpcScheduleStepOutcome::Stalled => {}
                        NpcScheduleStepOutcome::EndTick => break,
                    }
                }
                NPC_STATE_DESCEND_TOWARD_TARGET
                | NPC_STATE_ASCEND_TOWARD_TARGET
                | NPC_STATE_CLIMB_UP_OFF_FLOOR
                | NPC_STATE_CLIMB_DOWN_OFF_FLOOR => {
                    if self.advance_npc_floor_transition_step(
                        index,
                        wp,
                        tx,
                        ty,
                        tz,
                        floor,
                        &mut searched,
                    ) {
                        moved = true;
                    }
                }
                // npc-schedules.md §7: state 8 is *not* a parked state - the
                // walker resolves it immediately with the ungated placement,
                // and "the same ungated placement is what happens if any
                // unexpected state value reaches the floor-transition arm".
                _ => {
                    if self.place_npc_at_waypoint_ungated(index, wp, tx, ty, tz, floor) {
                        moved = true;
                    }
                }
            }
        }
        if moved {
            self.mark_visibility_dirty();
        }
    }

    /// `npc-schedules.md §7` state 2. The cardinal probe is not a search;
    /// only the flood fill is, so `searched` gates the route call alone.
    pub fn advance_npc_in_plane_schedule_step(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
        searched: &mut bool,
    ) -> NpcScheduleStepOutcome {
        let start = (self.npcs[npc_index].x, self.npcs[npc_index].y);
        let target = (target_x, target_y);
        let direct_step = step_toward(start, target).filter(|(nx, ny)| {
            self.npc_can_step_toward(npc_index, *nx, *ny, floor, target_x, target_y)
        });
        if let Some((nx, ny)) = direct_step {
            return NpcScheduleStepOutcome::from_moved(self.commit_npc_schedule_position(
                npc_index, waypoint, target_x, target_y, target_z, floor, nx, ny, target_z,
            ));
        }
        if *searched {
            // `npc-schedules.md §9.1`, "The same stepper is reached with the
            // cap switched off": one of the two route-recovery entries is
            // "the state-2/3 arm whose queue is exhausted and whose replan
            // either failed or **was not attempted**". This is the
            // not-attempted arm - §7's per-tick latch, "at most one NPC per
            // tick may start a fresh search", has already spent the tick's
            // one search.
            return self.npc_route_recovery_wander(npc_index, floor, target_x, target_y);
        }
        // `npc-schedules.md §9.1`: "when the NPC's stuck counter is non-zero
        // the walker only re-plans on a **one-in-three** draw and otherwise
        // ages the counter for that turn." The draw is taken before the
        // search latch is spent, so a stalled NPC does not deny the tick's
        // one search to a later slot on the turns it does not replan.
        if self.npcs[npc_index].stuck_counter != 0 && !self.town_npc_stuck_replan_draw_passes() {
            self.npcs[npc_index].note_failed_progress();
            return NpcScheduleStepOutcome::Stalled;
        }
        *searched = true;
        let Some(route) = self.npc_path_route(npc_index, start, target, floor) else {
            // The other published route-recovery entry: the replan failed.
            return self.npc_route_recovery_wander(npc_index, floor, target_x, target_y);
        };
        self.npcs[npc_index].set_move_queue(route);
        self.advance_npc_replay_queue_step(npc_index, waypoint, target_x, target_y, target_z, floor)
    }

    /// `npc-schedules.md §9.1`, "**The same stepper is reached with the cap
    /// switched off.**"
    ///
    /// "Two route-recovery paths in the state machine call the identical
    /// wander step with cap zero: the queued-route replay when its next
    /// queued step is refused, and the state-2/3 arm whose queue is exhausted
    /// and whose replan either failed or was not attempted because a queue
    /// pointer was still live. Both are still behind the same one-in-two
    /// coin, so an NPC that is recovering from a blocked route also moves on
    /// about half its turns, but without the waypoint-radius limit."
    ///
    /// The recovery step is the wander stepper itself, so it pays the coin
    /// and the direction draw and spends the turn on a stage-3/4 rejection
    /// exactly as the AI-dispatch entry does. It commits the cell the same
    /// way the AI dispatch does - position plus sprite sync - and leaves the
    /// state byte, the move queue and the cached waypoint alone, because none
    /// of §7's route bookkeeping applies to a step the route did not choose.
    /// The failed route step is still counted against the stuck counter
    /// (§5 step 7, "if the NPC fails to make progress, the stuck counter is
    /// bumped"); recovering sideways is not progress along the route.
    fn npc_route_recovery_wander(
        &mut self,
        npc_index: usize,
        floor: u8,
        target_x: usize,
        target_y: usize,
    ) -> NpcScheduleStepOutcome {
        self.npcs[npc_index].note_failed_progress();
        let Some((nx, ny)) = self.town_npc_wander_step(
            npc_index,
            floor,
            target_x,
            target_y,
            TOWN_NPC_UNBOUNDED_WANDER_CAP,
        ) else {
            return NpcScheduleStepOutcome::Stalled;
        };
        self.npcs[npc_index].x = nx;
        self.npcs[npc_index].y = ny;
        self.sync_npc_active_object(npc_index, floor);
        NpcScheduleStepOutcome::Moved
    }

    /// `npc-schedules.md §9.1`: "when the NPC's stuck counter is non-zero the
    /// walker only re-plans on a **one-in-three** draw and otherwise ages the
    /// counter for that turn."
    ///
    /// Only the rate is published. The draw form here is the same range
    /// primitive the wander gate uses, over `0..2`, accepted on one value.
    fn town_npc_stuck_replan_draw_passes(&mut self) -> bool {
        self.random_range_u8(0, TOWN_NPC_STUCK_REPLAN_DRAW_MAX) == 0
    }

    /// `npc-schedules.md §7` state 8 and the unexpected-state default:
    /// "the walker resolves it immediately by writing the active
    /// waypoint's `(x, y, z)` straight into the NPC's runtime position,
    /// caching the waypoint, deactivating the move queue and returning
    /// the state to idle." No gate and no search; the world-mutation
    /// primitive still gets its chance to free or allocate a sprite.
    pub fn place_npc_at_waypoint_ungated(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
    ) -> bool {
        self.npcs[npc_index].x = target_x;
        self.npcs[npc_index].y = target_y;
        self.npcs[npc_index].z = target_z;
        self.npcs[npc_index].set_settled_at_waypoint(waypoint);
        // §7: when neither end is on the displayed floor "no sprite is
        // allocated and nothing is visible", so only an actual sprite-layer
        // change counts as movement for the pass's repaint flag.
        self.sync_npc_active_object(npc_index, floor)
    }

    /// `npc-schedules.md §7` state 3. Queue replay is exempt from the
    /// per-tick search latch; it performs no search of its own.
    pub fn advance_npc_replay_queue_step(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
    ) -> NpcScheduleStepOutcome {
        let start = (self.npcs[npc_index].x, self.npcs[npc_index].y);
        let Some(code) = self.npcs[npc_index].peek_move_queue_direction() else {
            // npc-schedules.md §7: "when a queued route drains while the NPC
            // is still in state 3, the walker re-reads the active waypoint
            // and re-enters state 6 or 7 according to whether that
            // waypoint's floor is above or below the displayed floor" - and
            // that transition also ends the tick.
            if target_z != floor {
                self.npcs[npc_index].state =
                    schedule_floor_state(self.npcs[npc_index].z, target_z, floor);
                self.npcs[npc_index].reset_move_queue();
                return NpcScheduleStepOutcome::EndTick;
            }
            self.npcs[npc_index].set_idle();
            return NpcScheduleStepOutcome::Stalled;
        };
        // `npc-schedules.md §9.1`: the first of the two cap-zero
        // route-recovery entries is "the queued-route replay **when its next
        // queued step is refused**". Both refusal arms below are that case -
        // an unusable direction code and a cell the §10 checks reject.
        let Some((nx, ny)) = npc_step_from_direction_code(start, code) else {
            return self.npc_route_recovery_wander(npc_index, floor, target_x, target_y);
        };
        if !self.npc_can_step_toward(npc_index, nx, ny, floor, target_x, target_y) {
            return self.npc_route_recovery_wander(npc_index, floor, target_x, target_y);
        }
        self.npcs[npc_index].advance_move_queue_direction();
        let moved = self.commit_npc_schedule_position(
            npc_index, waypoint, target_x, target_y, target_z, floor, nx, ny, target_z,
        );
        if (nx, ny) != (target_x, target_y) && !self.npcs[npc_index].move_queue.is_empty() {
            self.npcs[npc_index].state = NPC_STATE_REPLAY_QUEUE;
        }
        NpcScheduleStepOutcome::from_moved(moved)
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

    /// `npc-schedules.md §9.1` stage 1, the per-turn wander gate: "A fair
    /// coin - **one in two**, not one in eight. The engine draws a uniform
    /// byte and tests a single bit of it. On the losing half the NPC does
    /// nothing at all this turn; there is no second look and no accumulated
    /// credit. *Established.* The draw is the byte-wide form of the range
    /// primitive in `systems/prng.md`, whose width divides the generator's
    /// magnitude range evenly, so the draw is bias-free and the single bit is
    /// an exact half."
    ///
    /// Which of the eight bits is tested, and which half of it loses, are not
    /// published; the low bit is used here. Only the rate is contractual.
    pub fn town_npc_wander_gate_passes(&mut self) -> bool {
        self.random_range_u8(0, u8::MAX) & 1 == 0
    }

    /// `npc-schedules.md §9.1` stage 2: "A single cardinal direction is
    /// drawn: a uniform value over the sixty-five integers `0..64`, folded to
    /// its low two bits and mapped to east, north, west, south in that order.
    /// Because sixty-five values do not fold evenly into four, the mapping is
    /// very slightly biased: east is drawn about `26.2%` of the time and each
    /// of the other three about `24.6%`. An implementation that draws a clean
    /// uniform `0..3` will be indistinguishable in play but not in a recorded
    /// roll sequence."
    pub fn town_npc_wander_direction(&mut self) -> Direction {
        match self.random_range_u8(0, TOWN_NPC_WANDER_DIRECTION_DRAW_MAX) & 0b11 {
            0 => Direction::East,
            1 => Direction::North,
            2 => Direction::West,
            _ => Direction::South,
        }
    }

    /// `npc-schedules.md §9.1`, the complete per-turn wander contract for AI
    /// values `1`, `2` and the far-from-player arm of value `4`.
    ///
    /// "An NPC that reaches the wander step gets **exactly one attempt per
    /// player turn**, and that attempt has four stages, each of which can end
    /// the turn with no movement": the one-in-two coin, one direction draw,
    /// the waypoint-radius test ("skipped entirely when the cap is zero" -
    /// the candidate cell's "**Manhattan distance to the active waypoint's
    /// stored `(x, y)`** is compared against the cap, and a distance
    /// **strictly greater** than the cap rejects the step"), and the ordinary
    /// per-step cell check of §10.
    ///
    /// "**A rejection at stage 3 or 4 is a spent turn.** The NPC does not try
    /// a second direction, does not re-roll, and does not fall back to a
    /// different rule. One coin, one direction, one candidate, one turn." The
    /// earlier implementation swept all four cardinals in a turn-derived
    /// rotation with no coin at all, which is why this is a rewrite rather
    /// than an edit.
    ///
    /// "Note that the rule caps where the NPC may *stand*, not how far it may
    /// travel: an NPC sitting exactly on its waypoint may step to any of the
    /// four neighbours, and only a candidate four cells away is refused at
    /// cap three." The radius is therefore Manhattan on the candidate, not a
    /// per-axis test on the committed cell.
    ///
    /// A step that lands on the waypoint commits but does **not** refresh the
    /// cached-waypoint field (§6); the caller leaves that field alone.
    pub fn town_npc_wander_step(
        &mut self,
        npc_index: usize,
        floor: u8,
        waypoint_x: usize,
        waypoint_y: usize,
        cap: usize,
    ) -> Option<(usize, usize)> {
        if !self.town_npc_wander_gate_passes() {
            return None;
        }
        let direction = self.town_npc_wander_direction();
        let (dx, dy) = direction.delta();
        let candidate_x = self.npcs[npc_index].x as isize + dx;
        let candidate_y = self.npcs[npc_index].y as isize + dy;
        if cap != 0 {
            let distance = candidate_x.abs_diff(waypoint_x as isize)
                + candidate_y.abs_diff(waypoint_y as isize);
            if distance > cap {
                return None;
            }
        }
        self.town_npc_step_in_direction_toward(npc_index, floor, direction, waypoint_x, waypoint_y)
    }

    /// Manhattan distance from the party to an arbitrary town cell. Used by
    /// the `npc-schedules.md §9` value-`4` fork, which measures the player
    /// against the *waypoint* rather than against the NPC.
    pub fn town_player_distance_to(&self, x: usize, y: usize) -> usize {
        self.player.x.abs_diff(x) + self.player.y.abs_diff(y)
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

    /// `npc-schedules.md §7` states 4/5/6/7. Both halves of the pass run
    /// a search, so both are gated by the per-tick search latch; the
    /// on-floor gate-accept fast path performs no search and stays
    /// unlatched.
    pub fn advance_npc_floor_transition_step(
        &mut self,
        npc_index: usize,
        waypoint: usize,
        target_x: usize,
        target_y: usize,
        target_z: u8,
        floor: u8,
        searched: &mut bool,
    ) -> bool {
        let npc_z = self.npcs[npc_index].z;
        if npc_z == floor {
            if target_z == floor {
                return false;
            }
            // npc-schedules.md §8.5: states 6/7 hunt the link that points
            // toward the waypoint's floor, the one that is not displayed.
            let marker = npc_floor_link_marker_toward(floor, target_z);
            let current = (self.npcs[npc_index].x, self.npcs[npc_index].y);
            if npc_floor_link_gate_accepts(self.grid[current.1 * 32 + current.0], marker) {
                // npc-schedules.md §8.5, states 6/7: "When the gate accepts,
                // the walker writes the NPC's position directly to the active
                // waypoint's own `(x, y, z)`, caches the waypoint, deactivates
                // the move queue and returns the state to idle; the NPC leaves
                // the displayed floor and its sprite is released." There is no
                // paired link cell on the destination floor and no one-floor-
                // at-a-time climb: "an on-floor NPC lands on its schedule
                // waypoint's own coordinates, wherever they are."
                return self.place_npc_at_waypoint_ungated(
                    npc_index, waypoint, target_x, target_y, target_z, floor,
                );
            }
            if *searched {
                self.npcs[npc_index].note_failed_progress();
                return false;
            }
            *searched = true;
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
            // npc-schedules.md §8.5: for states 4/5 the floor that is not
            // displayed is the NPC's own, so the marker points toward it —
            // `0xC8` for an NPC above, `0xC9` for one below.
            let marker = npc_floor_link_marker_toward(floor, npc_z);
            if *searched {
                self.npcs[npc_index].note_failed_progress();
                return false;
            }
            *searched = true;
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

        // npc-schedules.md §7: neither end is on the displayed floor, so
        // this is the state-8 case - the ungated placement, not a stall.
        self.place_npc_at_waypoint_ungated(npc_index, waypoint, target_x, target_y, target_z, floor)
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
                if npc_floor_link_arrival_accepts(self.grid[idx], marker) {
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
        // npc-schedules.md §8.5: the arrival/route cells accept the
        // state's own link marker or the visible stairway family; every
        // other cell falls back to the §10 obstacle test.
        let tile = self.grid[y * 32 + x];
        if !npc_floor_link_arrival_accepts(tile, marker) && npc_path_tile_obstacle(tile) {
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
                row.iter().enumerate().filter_map(move |(x, tile)| {
                    npc_floor_link_arrival_accepts(*tile, marker).then_some((x, y))
                })
            })
            .collect()
    }
}

const TOWN_NPC_CHASE_RADIUS: usize = 8;
/// `npc-schedules.md §9` value `1`: "an accepted step must leave the NPC
/// within Manhattan distance **three** of the active waypoint", and value `4`
/// (R317) "uses the **same constant cap of three**".
const TOWN_NPC_BOUNDED_WANDER_CAP: usize = 3;
/// `npc-schedules.md §9` value `2`: "the waypoint-radius test switched off
/// entirely (cap zero)".
const TOWN_NPC_UNBOUNDED_WANDER_CAP: usize = 0;
/// `npc-schedules.md §9.1`: the stuck-counter replan draw is "a
/// **one-in-three** draw", so the range primitive runs over `0..2`.
const TOWN_NPC_STUCK_REPLAN_DRAW_MAX: u8 = 2;
/// `npc-schedules.md §9.1` stage 2: the direction draw is "a uniform value
/// over the sixty-five integers `0..64`".
const TOWN_NPC_WANDER_DIRECTION_DRAW_MAX: u8 = 64;
/// `npc-schedules.md §9` value `4` (R317): the fork between the wander arm
/// and the engagement path is "while the player is four or more tiles from
/// the *waypoint*".
const TOWN_NPC_APPROACH_ENGAGE_WAYPOINT_DISTANCE: usize = 4;
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
