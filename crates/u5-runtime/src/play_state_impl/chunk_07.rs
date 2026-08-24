use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    pub fn combat_sjog_actor_direction(
        &mut self,
        actor_slot: usize,
        branch: CombatCommandBranch,
        direction: Direction,
    ) -> MoveOutcome {
        match branch {
            CombatCommandBranch::Get => self.get_combat_actor_direction(actor_slot, direction),
            CombatCommandBranch::Jimmy => self.jimmy_combat_actor_direction(actor_slot, direction),
            CombatCommandBranch::Open => self.open_combat_actor_direction(actor_slot, direction),
            CombatCommandBranch::Search => {
                self.search_combat_actor_direction(actor_slot, direction)
            }
            _ => {
                self.message = "What?".to_string();
                MoveOutcome::Blocked
            }
        }
    }

    pub fn get_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((actor, x, y)) =
            self.combat_sjog_target_coordinate(actor_slot, direction, "Nothing to get there.")
        else {
            return MoveOutcome::Blocked;
        };

        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
        {
            self.message = "Nothing to get there.".to_string();
            return MoveOutcome::Blocked;
        }
        let Some((object_slot, object)) =
            self.combat_loose_object_slot_at(x, y, actor.active_object_slot as usize)
        else {
            self.message = "Nothing to get here.".to_string();
            return MoveOutcome::Blocked;
        };

        self.free_active_object_slot(object_slot);
        self.mark_visibility_dirty();
        self.message = format!("Got combat object tile {} at ({x}, {y}).", object.tile);
        MoveOutcome::Got
    }

    pub fn search_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((actor, x, y)) =
            self.combat_sjog_target_coordinate(actor_slot, direction, "Nothing to search there.")
        else {
            return MoveOutcome::Blocked;
        };

        if let Some((_, object)) =
            self.combat_loose_object_slot_at(x, y, actor.active_object_slot as usize)
        {
            self.message = format!("Found combat object tile {} at ({x}, {y}).", object.tile);
            return MoveOutcome::Searched;
        }

        let tile = self.combat_terrain[y][x];
        self.message = format!("Searched combat tile {tile} at ({x}, {y}).");
        MoveOutcome::Searched
    }

    pub fn open_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((_, x, y)) =
            self.combat_sjog_target_coordinate(actor_slot, direction, "Nothing to open there.")
        else {
            return MoveOutcome::Blocked;
        };
        let tile = self.combat_terrain[y][x];
        if tile == TOWN_OPEN_ALREADY_OPEN_TILE {
            self.message = "It's open!".to_string();
            return MoveOutcome::DoorOpened;
        }
        if jimmy_locked_door_rewrite(tile).is_some() || jimmy_magic_locked_door(tile) {
            self.message = "Locked!".to_string();
            return MoveOutcome::Blocked;
        }
        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
        {
            self.message = "Nothing to open there.".to_string();
            return MoveOutcome::Blocked;
        }
        if !openable_town_door(tile) {
            self.message = "Nothing to open here.".to_string();
            return MoveOutcome::Blocked;
        }

        self.combat_terrain[y][x] = TOWN_DOOR_CLEARED_TILE;
        self.mark_visibility_dirty();
        self.message = "Opened!".to_string();
        MoveOutcome::DoorOpened
    }

    pub fn jimmy_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some((_, x, y)) = self.combat_sjog_target_coordinate(actor_slot, direction, "No lock!")
        else {
            return MoveOutcome::Blocked;
        };
        if self.keys == 0 {
            self.message = "No keys!".to_string();
            return MoveOutcome::Blocked;
        }
        let tile = self.combat_terrain[y][x];
        if jimmy_magic_locked_door(tile) {
            self.keys = self.keys.saturating_sub(1);
            self.message = "Key broke!".to_string();
            return MoveOutcome::LockTried;
        }
        if jimmy_restraint_tile(tile) {
            if !self.jimmy_lock_pick_succeeds(actor_slot) {
                self.keys = self.keys.saturating_sub(1);
                self.message = "Key broke!".to_string();
                return MoveOutcome::LockTried;
            }
            self.combat_terrain[y][x] = TOWN_DOOR_CLEARED_TILE;
            self.mark_visibility_dirty();
            self.message = "Unlocked".to_string();
            return MoveOutcome::LockTried;
        }
        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
        {
            self.message = "No lock!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(unlocked_tile) = Self::visible_jimmy_unlock_tile(tile) else {
            self.message = "No lock!".to_string();
            return MoveOutcome::Blocked;
        };
        if !self.jimmy_lock_pick_succeeds(actor_slot) {
            self.keys = self.keys.saturating_sub(1);
            self.message = "Key broke!".to_string();
            return MoveOutcome::LockTried;
        }

        self.combat_terrain[y][x] = unlocked_tile;
        self.mark_visibility_dirty();
        self.message = "Unlocked!".to_string();
        MoveOutcome::LockTried
    }

    pub fn combat_sjog_target_coordinate(
        &mut self,
        actor_slot: usize,
        direction: Direction,
        out_of_bounds_message: &str,
    ) -> Option<(CombatActorDescriptor, usize, usize)> {
        let Some(actor) = self.live_combat_party_actor(actor_slot) else {
            self.message = "No active combatant.".to_string();
            return None;
        };
        if !direction.is_cardinal() {
            self.message = "Direction?".to_string();
            return None;
        }
        let (dx, dy) = direction.delta();
        let x = actor.x as isize + dx;
        let y = actor.y as isize + dy;
        if !combat_arena_coordinate_in_bounds(x as i16, y as i16) {
            self.message = out_of_bounds_message.to_string();
            return None;
        }
        Some((actor, x as usize, y as usize))
    }

    pub fn klimb_combat_actor_vertical(
        &mut self,
        actor_slot: usize,
        intent: ClimbIntent,
    ) -> MoveOutcome {
        let Some(actor) = self.live_combat_party_actor(actor_slot) else {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        };
        let tile = self.combat_terrain[actor.y as usize][actor.x as usize];
        if !combat_klimb_tile_accepts_vertical(tile, intent) {
            self.message = "Klimb-What?".to_string();
            return MoveOutcome::Blocked;
        }

        let label = match intent {
            ClimbIntent::Up => "up",
            ClimbIntent::Down => "down",
        };
        self.message = format!("Klimbed {label} from combat.");
        // Successful vertical Klimb restores the suspended frame immediately,
        // so its committed-action maintenance must run while the acting
        // descriptor is still present.
        let _ = self.apply_combat_absorbable_field_contact_for_actor_position(actor_slot);
        let _ = self.apply_combat_post_dispatch_contact_for_actor_position(actor_slot);
        let _ = self.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        let _ = self.age_active_effect();
        let exit = match self.combat_round_loop_control(true, false) {
            CombatRoundLoopControl::Exit(exit) => exit,
            CombatRoundLoopControl::ContinueActorWalk | CombatRoundLoopControl::StartNextRound => {
                CombatRoundLoopExit::LeaveCombat
            }
        };
        self.apply_combat_round_loop_exit(exit);
        MoveOutcome::Moved
    }

    pub fn klimb_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        let Some(actor) = self.live_combat_party_actor(actor_slot) else {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Klimb-What?".to_string();
            return MoveOutcome::Blocked;
        }

        let (dx, dy) = direction.delta();
        let x = actor.x as isize + dx;
        let y = actor.y as isize + dy;
        if !combat_arena_coordinate_in_bounds(x as i16, y as i16) {
            self.message = "Klimb-What?".to_string();
            return MoveOutcome::Blocked;
        }
        let x = x as usize;
        let y = y as usize;
        if self
            .combat_actor_slot_at(x as u8, y as u8, actor_slot)
            .is_some()
            || !is_probe_walkable(self.combat_terrain[y][x])
        {
            self.message = "Klimb-What?".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(commit) = commit_combat_actor_linked_position(
            &mut self.combat_actors[actor_slot],
            &mut self.active_objects,
            x as u8,
            y as u8,
        ) else {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        };
        self.mark_visibility_dirty();
        self.message = format!(
            "Klimbed {} to ({}, {}).",
            direction.name(),
            commit.actor_position_after.0,
            commit.actor_position_after.1
        );
        MoveOutcome::Moved
    }

    fn live_combat_party_actor(&self, actor_slot: usize) -> Option<CombatActorDescriptor> {
        if !self.combat_active || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return None;
        }
        self.combat_actors
            .get(actor_slot)
            .copied()
            .filter(|actor| combat_actor_is_active_not_dead(*actor))
    }

    pub fn open_dungeon_chest(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        // `traps.md §2.1`/§4: the dungeon chest site uses the same shared
        // acting-member selection, minus the combat override - the Open
        // dispatcher never routes a combat-class scene here. The selection
        // runs before the trap test, so a party with nobody able aborts
        // before the trap can fire.
        let acting_member = self.dungeon_container_acting_member();
        let target_slot = match acting_member {
            ActingMemberSelection::Selected(slot) => slot,
            ActingMemberSelection::Prompt => {
                self.active_surface_chest = Some(SurfaceChestSession::new_dungeon(
                    scene, level, x, y, idx, tile,
                ));
                self.message = self.render_active_surface_chest();
                return MoveOutcome::Observed;
            }
            ActingMemberSelection::NoneAble => {
                self.message = "No party members are available.".to_string();
                return MoveOutcome::Blocked;
            }
        };
        self.finish_open_dungeon_chest(scene, level, x, y, idx, tile, target_slot)
    }

    pub(crate) fn finish_open_dungeon_chest(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        target_slot: usize,
    ) -> MoveOutcome {
        if self.grid.get(idx).copied() != Some(tile) || tile >> 4 != 0x4 {
            self.message = "Nothing to open!".to_string();
            return MoveOutcome::Blocked;
        }
        let trap_note = if self.dungeon_chest_trap_detail(level, x, y, tile) == "no trap" {
            None
        } else {
            Some(self.apply_shared_trap_effect_to_slot(target_slot))
        };
        self.grid[idx] = dungeon_open_chest_rewrite(tile);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = match trap_note {
            Some(trap) => format!(
                "Opened dungeon chest at ({x}, {y}) on {} level {level}; {trap}, marked visit-local open chest.",
                scene.key()
            ),
            None => format!(
                "Opened dungeon chest at ({x}, {y}) on {} level {level}; marked visit-local open chest.",
                scene.key()
            ),
        };
        MoveOutcome::ContainerOpened
    }

    pub fn search_dungeon_chest(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> MoveOutcome {
        let detail = self.dungeon_chest_trap_detail(level, x, y, tile);
        self.advance_turn();
        self.message = format!(
            "Searched dungeon chest at ({x}, {y}) on {} level {level}; {detail}.",
            scene.key()
        );
        MoveOutcome::Searched
    }

    pub fn dungeon_chest_trap_detail(
        &self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> &'static str {
        let trap_detection_stat = self
            .party
            .iter()
            .find(|member| member.living())
            .map(|member| member.class_byte)
            .unwrap_or_default();
        let threshold = Self::dungeon_chest_pick_threshold(level, trap_detection_stat);
        let roll = self.dungeon_chest_trap_roll(level, x, y, tile, 0, 30);
        let detail = if u16::from(roll) > threshold && Self::is_plain_closed_dungeon_chest(tile) {
            "no trap"
        } else {
            let tier = if u16::from(roll) <= threshold {
                self.dungeon_chest_trap_roll(level, x, y, tile, 1, 8)
            } else {
                level
            };
            match tier {
                0..=3 => "simple trap",
                7..=u8::MAX => "complex trap",
                _ => "trap",
            }
        };
        detail
    }

    pub fn dungeon_chest_pick_threshold(level: u8, dexterity: u8) -> u16 {
        dungeon_chest_jimmy_threshold(level, dexterity)
    }

    pub fn is_plain_closed_dungeon_chest(tile: u8) -> bool {
        tile == 0x40
    }

    pub fn dungeon_chest_trap_roll(
        &self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        stage: u8,
        upper: u8,
    ) -> u8 {
        1 + (self.dungeon_chest_roll_seed(level, x, y, tile, 7, stage) % upper)
    }

    pub fn consume_dungeon_chest(
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
        let content_tile = if tile >> 4 == 0x7 {
            0x40 | (tile & 0x0f)
        } else {
            tile
        };
        let note = self
            .apply_dungeon_chest_content(entries, scene, level, x, y, content_tile)
            .unwrap_or_else(|| self.generate_dungeon_chest_content(level, x, y, content_tile));
        self.consume_dungeon_chest_with_note(scene, level, x, y, idx, tile, verb, &note)
    }

    pub fn apply_dungeon_chest_content(
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
                && entry.expected_cell.map_or(true, |expected| {
                    expected == tile || (tile >> 4 == 0x7 && expected == (0x40 | (tile & 0x0f)))
                })
        })?;
        let mut parts = Vec::new();
        for grant in &entry.grants {
            self.apply_object_pickup(grant.kind, grant.amount);
            parts.push(format!("{} {}", grant.amount, grant.kind.label()));
        }
        Some(format!("authored chest grants {}", parts.join(", ")))
    }

    /// `containers.md §6` dungeon-chest reward generator. Iterates the
    /// seven published reward rows in order, driven by
    /// [`DUNGEON_CHEST_ROWS`] rather than by re-typed literals, so the
    /// gate thresholds and the row order have one source of truth.
    /// Each row rolls uniform in `1..=(4 * dungeon_depth + 4)`
    /// ([`dungeon_chest_row_gate_max`]) and is awarded when its
    /// threshold is at or below that roll
    /// ([`dungeon_chest_row_awarded`]); multiple rows can succeed for
    /// one chest.
    pub fn generate_dungeon_chest_content(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> String {
        let gate_upper = u16::from(dungeon_chest_row_gate_max(level));
        let mut parts = Vec::new();

        for (index, row) in DUNGEON_CHEST_ROWS.iter().enumerate() {
            let row_index = index as u8;
            let gate_roll = self.dungeon_chest_roll(level, x, y, tile, row_index, 0, gate_upper);
            if !dungeon_chest_row_awarded(*row, gate_roll) {
                continue;
            }
            match row.reward {
                DungeonChestReward::Food => {
                    let amount = self.dungeon_chest_roll(
                        level,
                        x,
                        y,
                        tile,
                        row_index,
                        1,
                        u16::from(DUNGEON_CHEST_FOOD_MAX),
                    );
                    self.apply_object_pickup(ObjectPickupKind::Food, amount);
                    parts.push(format!("{amount} food"));
                }
                DungeonChestReward::Gold => {
                    let amount = self.dungeon_chest_gold_roll(level, x, y, tile);
                    self.apply_object_pickup(ObjectPickupKind::Gold, amount);
                    parts.push(format!("{amount} gold"));
                }
                DungeonChestReward::Keys => {
                    let amount = self.dungeon_chest_small_roll(level, x, y, tile, row_index);
                    self.apply_object_pickup(ObjectPickupKind::Keys, amount);
                    parts.push(format!("{amount} keys"));
                }
                DungeonChestReward::Gems => {
                    let amount = self.dungeon_chest_small_roll(level, x, y, tile, row_index);
                    self.apply_object_pickup(ObjectPickupKind::Gems, amount);
                    parts.push(format!("{amount} gems"));
                }
                DungeonChestReward::Torches => {
                    let amount = self.dungeon_chest_small_roll(level, x, y, tile, row_index);
                    self.apply_object_pickup(ObjectPickupKind::Torches, amount);
                    parts.push(format!("{amount} torches"));
                }
                DungeonChestReward::Potion => {
                    let subtype = self.dungeon_chest_zero_based_roll(
                        level,
                        x,
                        y,
                        tile,
                        row_index,
                        1,
                        POTION_COUNT,
                    );
                    self.apply_object_pickup(ObjectPickupKind::Potion(subtype), 1);
                    parts.push(format!("1 {} potion", potion_label(subtype)));
                }
                DungeonChestReward::Scroll => {
                    let subtype = self.dungeon_chest_zero_based_roll(
                        level,
                        x,
                        y,
                        tile,
                        row_index,
                        1,
                        SCROLL_COUNT,
                    );
                    self.apply_object_pickup(ObjectPickupKind::Scroll(subtype), 1);
                    parts.push(format!("1 {} scroll", scroll_label(subtype)));
                }
            }
        }

        if parts.is_empty() {
            "generated chest grants nothing".to_string()
        } else {
            format!("generated chest grants {}", parts.join(", "))
        }
    }

    /// `containers.md §6`: the keys, gems, and torches rows all roll
    /// `1..3` for their quantity.
    pub fn dungeon_chest_small_roll(&self, level: u8, x: usize, y: usize, tile: u8, row: u8) -> u8 {
        self.dungeon_chest_roll(
            level,
            x,
            y,
            tile,
            row,
            1,
            u16::from(DUNGEON_CHEST_SMALL_MAX),
        )
    }

    pub fn dungeon_chest_gate_succeeds(
        &self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        row: u8,
        threshold: u8,
        upper: u16,
    ) -> bool {
        threshold <= self.dungeon_chest_roll(level, x, y, tile, row, 0, upper)
    }

    /// `containers.md §6` gold row. The row passes lower endpoint `1`
    /// and upper endpoint `8 * dungeon_depth`
    /// ([`dungeon_chest_gold_upper`]) to the shared one-based random
    /// helper. At `dungeon_depth == 0` that is an invalid `1..0`
    /// range; the published behaviour is to consume one PRNG advance
    /// and reach the zero-width edge rather than clamp the bound to
    /// one, so [`dungeon_chest_gold_is_zero_width`] gates that branch.
    pub fn dungeon_chest_gold_roll(&self, level: u8, x: usize, y: usize, tile: u8) -> u8 {
        const GOLD_ROW: u8 = 1;
        if dungeon_chest_gold_is_zero_width(level) {
            let _ = self.dungeon_chest_roll_seed(level, x, y, tile, GOLD_ROW, 1);
            return 0;
        }
        self.dungeon_chest_roll(
            level,
            x,
            y,
            tile,
            GOLD_ROW,
            1,
            u16::from(dungeon_chest_gold_upper(level)),
        )
    }

    pub fn dungeon_chest_roll(
        &self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        row: u8,
        stage: u8,
        upper: u16,
    ) -> u8 {
        if upper == 0 {
            return 0;
        }
        1 + (u16::from(self.dungeon_chest_roll_seed(level, x, y, tile, row, stage)) % upper) as u8
    }

    pub fn dungeon_chest_zero_based_roll(
        &self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        row: u8,
        stage: u8,
        upper: usize,
    ) -> usize {
        usize::from(self.dungeon_chest_roll_seed(level, x, y, tile, row, stage)) % upper
    }

    pub fn dungeon_chest_roll_seed(
        &self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        row: u8,
        stage: u8,
    ) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ level.wrapping_mul(17)
            ^ (x as u8).wrapping_mul(19)
            ^ (y as u8).wrapping_mul(23)
            ^ tile.wrapping_mul(29)
            ^ row.wrapping_mul(31)
            ^ stage.wrapping_mul(37)
    }

    pub fn consume_dungeon_chest_with_note(
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
        self.grid[idx] = tile & 0x08;
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

    pub fn board_vehicle(&mut self) -> MoveOutcome {
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(candidate) = self.boardable_vehicle_slot() else {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        };
        let transport = candidate.transport;
        let starting_transport_marker = self.player.transport.save_marker();
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
        if matches!(transport, TransportState::Ship { .. })
            && ship_boarding_stows_carpet(starting_transport_marker)
        {
            let slot = &mut self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX];
            *slot = slot.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
        }
        self.player.transport = transport;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        let mut message = format!("Boarded {}.", transport.kind_name());
        transport.append_ship_auxiliary_warnings(&mut message);
        self.message = message;
        MoveOutcome::Boarded
    }

    pub fn force_foot_transport(&mut self) {
        self.player.transport = TransportState::Foot;
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
    }

    pub fn free_active_object_slot(&mut self, slot: usize) {
        if slot == 0 {
            return;
        }
        let dungeon_monster =
            matches!(self.area, Area::Dungeon { .. }) && slot == DUNGEON_ACTIVE_MONSTER_SLOT;
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.free();
            if dungeon_monster {
                object.tile = 0;
                object.aux1 = DUNGEON_MONSTER_INACTIVE_DEP1;
            }
        }
    }

    pub fn clear_consumed_active_object_slot(&mut self, slot: usize) {
        if slot == 0 {
            return;
        }
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.clear_record_prefix();
        }
    }

    pub fn clear_non_player_active_objects(&mut self) {
        self.sync_player_object();
        for object in self.active_objects.iter_mut().skip(1) {
            *object = ActiveObject::empty();
        }
    }

    pub fn clear_moonstone_pickups(&mut self, slot_index: usize) -> bool {
        let mut removed = false;
        for object in self.active_objects.iter_mut().skip(1) {
            if object.moonstone_slot_index() == Some(slot_index) {
                object.free();
                removed = true;
            }
        }
        removed
    }

    pub fn moonstone_pickup_exists(&self, slot_index: usize) -> bool {
        self.active_objects
            .iter()
            .any(|object| object.moonstone_slot_index() == Some(slot_index))
    }

    pub fn moonstone_pickup_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
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

    pub fn get_moonstone_pickup_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        let (object_slot, slot_index) = self.moonstone_pickup_at(x, y)?;
        self.clear_consumed_active_object_slot(object_slot);
        self.moonstone_slots[slot_index] = MoonstoneGateSlot::invalid();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Recovered Moonstone phase {}; Gate Travel slot cleared.",
            slot_index + 1
        );
        Some(MoveOutcome::Got)
    }

    pub fn allocate_active_object_slot(&mut self, object: ActiveObject) -> Option<usize> {
        if self.active_objects.is_empty() {
            return None;
        }
        // `encounters.md §9` / `active-objects.md §4`: the ordinary acquisition
        // path searches only slots 1..=23. Slot 0 is the player and slots
        // 24..=31 are reserved for setup paths outside the allocator, so
        // handing those out here over-fills the table past the published
        // density cap.
        if let Some(slot) = self
            .active_objects
            .iter()
            .enumerate()
            .skip(1)
            .take(ACTIVE_OBJECT_ACQUISITION_LAST_SLOT)
            .find_map(|(slot, object)| object.is_empty().then_some(slot))
        {
            self.active_objects[slot] = object;
            return Some(slot);
        }
        if self.active_objects.len() <= ACTIVE_OBJECT_ACQUISITION_LAST_SLOT {
            self.active_objects.push(object);
            return Some(self.active_objects.len() - 1);
        }
        // `active-objects.md §4`: taking an empty ordinary slot "is not the
        // whole contract: if the ordinary range is full, acquisition can evict
        // a lower-priority object." Run the deterministic ten-phase cascade
        // rather than failing the acquisition.
        let slot = self.active_object_eviction_victim()?;
        self.free_active_object_slot(slot);
        self.active_objects[slot] = object;
        self.mark_visibility_dirty();
        Some(slot)
    }

    /// `active-objects.md §4` eviction cascade. Runs at *acquisition* time --
    /// the spec places eviction in the allocator ("acquisition can evict a
    /// lower-priority object"), and §8.1 confirms the split: "Eviction is
    /// demand-driven - it runs when acquisition needs a slot and the range is
    /// full - and chooses a victim by class priority. Pruning is time-driven,
    /// runs every overworld turn regardless of pressure, and chooses by
    /// position alone." The per-turn sweep is the separate §8.1 prune pass
    /// ([`Self::prune_far_overworld_objects`]); the two must not be collapsed.
    /// `encounters.md §4` corroborates the trigger: on a successful spawn "the
    /// spawner acquires or evicts an active-object slot".
    ///
    /// Phases run in published order 1..=10, and within each phase the
    /// ordinary acquisition range is scanned lowest-index-up, matching the
    /// allocator's own scan discipline. Only slots whose
    /// [`active_object_slot_role`] is `OrdinaryAcquisition` are candidates, so
    /// slot 0 (the player) and the reserved slots 24..=31 can never be taken.
    /// Type byte [`ACTIVE_OBJECT_PROTECTED_TYPE_BYTE`] (`0xB5`) is the only
    /// universally protected byte-0 value and is rejected by every phase,
    /// last-resort phase 10 included.
    ///
    /// Phase 1 (the empty-slot phase) is included for completeness; the caller
    /// has already exhausted it.
    pub fn active_object_eviction_victim(&self) -> Option<usize> {
        let mut phase = ACTIVE_OBJECT_EVICTION_PHASE_FIRST;
        while phase <= ACTIVE_OBJECT_EVICTION_PHASE_LAST {
            let phase_needs_off_screen = active_object_eviction_phase_is_off_screen(phase);
            for (slot, candidate) in self.active_objects.iter().enumerate() {
                if !matches!(
                    active_object_slot_role(slot),
                    Some(ActiveObjectSlotRole::OrdinaryAcquisition)
                ) {
                    continue;
                }
                if !active_object_eviction_byte_accepted(candidate.type_byte, phase) {
                    continue;
                }
                if phase_needs_off_screen && !self.active_object_off_screen(*candidate) {
                    continue;
                }
                return Some(slot);
            }
            phase += 1;
        }
        None
    }

    /// `active-objects.md §4` off-screen gate for eviction phases 2..=5. It
    /// passes the current player globals and candidate record X/Y into the
    /// exact wrapped-byte predicate; candidate floor is deliberately ignored.
    pub fn active_object_off_screen(&self, object: ActiveObject) -> bool {
        active_object_eviction_off_screen(
            object.x as u8,
            object.y as u8,
            self.player.x as u8,
            self.player.y as u8,
        )
    }

    #[cfg(test)]
    pub fn exit_vehicle(&mut self) -> MoveOutcome {
        self.exit_vehicle_with_game_dir(None)
            .expect("vehicle exit without sidecar metadata cannot fail on file I/O")
    }

    pub fn exit_vehicle_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
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
        let exit_position = self.vehicle_exit_current_position_if_accepted(game_dir)?;

        // doors-and-z-transitions.md §11 / vehicles.md §5: a furled-ship exit
        // without nearby foot landing falls back to launching a carried skiff.
        // The ship hull stays parked at the original cell with one fewer
        // skiff aboard, and the party becomes the launched skiff in place.
        if exit_position.is_none() {
            if let TransportState::Ship {
                type_byte,
                tile,
                sails_hoisted: false,
                hull,
                skiffs,
            } = transport
            {
                if skiffs > 0 {
                    let parked_with_one_less_skiff = ActiveObject {
                        type_byte,
                        tile,
                        x: old_x,
                        y: old_y,
                        z,
                        phase: STEADY_PHASE,
                        aux1: hull,
                        aux3: skiffs - 1,
                    };
                    if self
                        .allocate_active_object_slot(parked_with_one_less_skiff)
                        .is_none()
                    {
                        self.message = "No active-object slot for vehicle.".to_string();
                        return Ok(MoveOutcome::Blocked);
                    }
                    self.player.transport = TransportState::Skiff {
                        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
                        tile: FIRST_PLAYABLE_SKIFF_TILE,
                    }
                    .with_facing(self.player.facing);
                    self.sail_cadence = 0;
                    self.sail_stall_pending = false;
                    self.sync_player_object();
                    self.mark_visibility_dirty();
                    self.advance_turn();
                    self.message = "Launched a skiff from the ship.".to_string();
                    return Ok(MoveOutcome::ExitedVehicle);
                }
                if self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] > 0 {
                    let parked_ship = ActiveObject {
                        type_byte,
                        tile,
                        x: old_x,
                        y: old_y,
                        z,
                        phase: STEADY_PHASE,
                        aux1: hull,
                        aux3: skiffs,
                    };
                    if self.allocate_active_object_slot(parked_ship).is_none() {
                        self.message = "No active-object slot for vehicle.".to_string();
                        return Ok(MoveOutcome::Blocked);
                    }
                    self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] =
                        self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX].saturating_sub(1);
                    self.player.transport = TransportState::Carpet {
                        type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
                        tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
                    };
                    self.sail_cadence = 0;
                    self.sail_stall_pending = false;
                    self.sync_player_object();
                    self.mark_visibility_dirty();
                    self.advance_turn();
                    self.message = "Redeployed stowed magic carpet from the ship.".to_string();
                    return Ok(MoveOutcome::ExitedVehicle);
                }
                // `vehicles.md §5` / `doors-and-z-transitions.md §11`: once
                // the furled-ship branch has established that no nearby
                // landing, carried skiff, or stowed carpet is available,
                // this is the no-skiffs refusal.
                self.message = SHIP_NO_SKIFFS_WARNING.to_string();
                return Ok(MoveOutcome::Blocked);
            }
            // Every non-ship family retains its location-specific refusal.
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let (x, y) = exit_position.expect("vehicle exit acceptance checked above");
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

    pub fn vehicle_can_park_at_current_cell(&self) -> bool {
        if !self.player.transport.is_balloon() {
            return true;
        }
        let Some(tile) = self.current_surface_tile() else {
            return false;
        };
        !is_mountain_tile(tile) && !is_wall_or_closed_door_tile(tile)
    }

    pub fn toggle_sails(&mut self) -> MoveOutcome {
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
            YELL_SAILS_HOISTED_MESSAGE.to_string()
        } else {
            YELL_SAILS_FURLED_MESSAGE.to_string()
        };
        MoveOutcome::SailToggled
    }

    pub fn yell_command(&mut self, word: Option<&str>) -> MoveOutcome {
        let scene_byte = self.current_scene_byte();
        let aboard_frigate = matches!(self.player.transport, TransportState::Ship { .. });
        if yell_routes_to_ship_sails(scene_byte, aboard_frigate) {
            return self.toggle_sails();
        }

        let Some(word) = word else {
            self.message = yell_prompt_message();
            return MoveOutcome::PromptDeclined;
        };
        let word = Self::normalize_yell_word(word);
        if word.is_empty() {
            self.message = YELL_NOTHING_SAID_MESSAGE.to_string();
            self.advance_turn();
            return MoveOutcome::Used;
        }

        self.advance_turn();
        match yell_input_context(scene_byte) {
            YellInputContext::WordOfPower => {
                if let Some((word_index, seal)) = word_of_power_seal_prefix_match(&word) {
                    let outcome = self.open_word_of_power_seal(word_index, seal);
                    let utterance = format!(
                        "Yelled {word}, the Word of Power for {}. A word of power is uttered. {}",
                        seal.dungeon,
                        Self::word_of_power_presentation_message()
                    );
                    if let WordOfPowerTargetOutcome::RuinedShrine { x, y } = outcome {
                        self.active_shrine_restoration =
                            Some(ShrineRestorationSession::new(word_index, x, y, utterance));
                        self.message = self.render_active_shrine_restoration();
                        return MoveOutcome::Used;
                    }
                    let context = match outcome {
                        WordOfPowerTargetOutcome::EntranceToggled { open: true, .. } => {
                            " The seal opens."
                        }
                        WordOfPowerTargetOutcome::EntranceToggled { open: false, .. } => {
                            " The entrance collapses shut."
                        }
                        WordOfPowerTargetOutcome::RuinedShrine { .. } => unreachable!(),
                        WordOfPowerTargetOutcome::NoQualifyingNeighbor
                        | WordOfPowerTargetOutcome::WrongCoordinate { .. } => " Nothing happens.",
                    };
                    self.message = format!("{utterance}{context}");
                    return MoveOutcome::Used;
                }
            }
            YellInputContext::ShadowlordName => {
                if let Some(index) = Self::shadowlord_name_index(&word) {
                    let shadowlord =
                        Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
                    self.message = if let Some(slot) = self.place_shadowlord_name_encounter(index) {
                        format!(
                            "Yelled {word}, the name of {shadowlord}. {shadowlord} appears in active-object slot {slot}."
                        )
                    } else {
                        format!("Yelled {word}. Nothing happens.")
                    };
                    return MoveOutcome::Used;
                }
            }
            YellInputContext::NoEffect => {}
        }

        self.message = format!("Yelled {word}. Nothing happens.");
        MoveOutcome::Used
    }

    pub fn normalize_yell_word(value: &str) -> String {
        value
            .trim()
            .chars()
            .take(30)
            .flat_map(char::to_uppercase)
            .collect()
    }

    pub fn word_of_power_dungeon(word: &str) -> Option<&'static str> {
        match word {
            "FALLAX" => Some("Deceit"),
            "VILIS" => Some("Despise"),
            "INOPIA" => Some("Destard"),
            "MALUM" => Some("Wrong"),
            "AVIDUS" => Some("Covetous"),
            "INFAMA" => Some("Shame"),
            "IGNAVUS" => Some("Hythloth"),
            "VERAMOCOR" => Some("Doom"),
            _ => None,
        }
    }

    pub const fn word_of_power_presentation_message() -> &'static str {
        "A low rumble and full-viewport flash answer the word."
    }

    pub fn render_active_shrine_restoration(&self) -> String {
        self.active_shrine_restoration
            .as_ref()
            .map(|session| format!("{}{}", session.transcript, session.buffer))
            .unwrap_or_default()
    }

    pub fn step_active_shrine_restoration(
        &mut self,
        key: char,
        suffix: &str,
    ) -> Option<MoveOutcome> {
        let Some(mut session) = self.active_shrine_restoration.take() else {
            return None;
        };
        if key == '\u{1b}' {
            session.buffer.clear();
            self.message = format!("{}{}", session.transcript, session.buffer);
            self.active_shrine_restoration = Some(session);
            return None;
        }
        if matches!(key, '\u{8}' | '\u{7f}') {
            session.buffer.pop();
            self.message = format!("{}{}", session.transcript, session.buffer);
            self.active_shrine_restoration = Some(session);
            return None;
        }

        let mut response = String::new();
        if !matches!(key, '\r' | '\n') && !key.is_control() {
            response.push(key);
        }
        response.push_str(suffix);
        session.buffer.extend(
            response
                .chars()
                .filter(|ch| ch.is_ascii() && !ch.is_control())
                .take(SHRINE_RESTORATION_INPUT_MAX_LEN.saturating_sub(session.buffer.len())),
        );
        if response.is_empty() && !matches!(key, '\r' | '\n') {
            self.message = format!("{}{}", session.transcript, session.buffer);
            self.active_shrine_restoration = Some(session);
            return None;
        }

        let virtue = ShrineVirtue::from_index(session.word_index)
            .expect("Word-of-Power index is a standard virtue index");
        let required = if session.response_index == 0 {
            virtue.name()
        } else {
            virtue.mantra()
        };
        let matches = !session.buffer.is_empty()
            && session
                .buffer
                .to_ascii_lowercase()
                .contains(&required.to_ascii_lowercase());
        session.all_responses_match &= matches;
        session.transcript.push_str(&session.buffer);
        session.buffer.clear();

        if session.response_index < 3 {
            session.response_index += 1;
            session
                .transcript
                .push_str(SHRINE_RESTORATION_MANTRA_PROMPT);
            self.message = session.transcript.clone();
            self.active_shrine_restoration = Some(session);
            return None;
        }

        let coordinate_matches = session.word_index != ShrineVirtue::Spirituality.index()
            && WORLD_SHRINE_COORDINATES.get(session.word_index).copied()
                == Some((session.target_x, session.target_y));
        let target_index = world_cell_index(session.target_x, session.target_y);
        let target_still_ruined =
            self.grid.get(target_index).copied() == Some(WORLD_RUINED_SHRINE_TILE);
        if session.all_responses_match && coordinate_matches && target_still_ruined {
            self.shrine_ruin_flags[session.word_index] &= !SAVE_QUEST_TILE_FLAG_HIGH_BIT;
            self.grid[target_index] = WORLD_SHRINE_TILE;
            session
                .transcript
                .push_str(SHRINE_RESTORATION_SUCCESS_BANNER);
            session
                .transcript
                .push_str(Self::word_of_power_presentation_message());
            let _ = self.refresh_world_live_chunks_for_current_area();
            self.mark_visibility_dirty();
        } else {
            session.transcript.push('\n');
        }
        self.message = session.transcript;
        Some(MoveOutcome::Used)
    }

    pub fn open_word_of_power_seal(
        &mut self,
        word_index: usize,
        seal: WordOfPowerSeal,
    ) -> WordOfPowerTargetOutcome {
        if !matches!(self.area, Area::World { .. }) {
            return WordOfPowerTargetOutcome::NoQualifyingNeighbor;
        }
        let adjacent = [
            (self.player.x.wrapping_sub(1) % WORLD_SIDE, self.player.y),
            (self.player.x, (self.player.y + 1) % WORLD_SIDE),
            ((self.player.x + 1) % WORLD_SIDE, self.player.y),
            (self.player.x, self.player.y.wrapping_sub(1) % WORLD_SIDE),
        ];
        let Some((x, y, tile)) = adjacent.into_iter().find_map(|(x, y)| {
            let tile = self.world_live_tile_at(x, y);
            matches!(
                tile,
                value if value == seal.unsealed_tile
                    || value == WORD_OF_POWER_SEALED_TILE
                    || value == WORLD_RUINED_SHRINE_TILE
            )
            .then_some((x, y, tile))
        }) else {
            return WordOfPowerTargetOutcome::NoQualifyingNeighbor;
        };
        if tile == WORLD_RUINED_SHRINE_TILE {
            return WordOfPowerTargetOutcome::RuinedShrine { x, y };
        }
        if (x, y) != (seal.x, seal.y) {
            return WordOfPowerTargetOutcome::WrongCoordinate { x, y };
        }
        let open = tile == WORD_OF_POWER_SEALED_TILE;
        self.grid[world_cell_index(x, y)] = if open {
            seal.unsealed_tile
        } else {
            WORD_OF_POWER_SEALED_TILE
        };
        self.word_of_power_seal_flags[word_index] ^= SAVE_QUEST_TILE_FLAG_HIGH_BIT;
        let _ = self.refresh_world_live_chunks_for_current_area();
        self.mark_visibility_dirty();
        WordOfPowerTargetOutcome::EntranceToggled { x, y, open }
    }

    pub fn shadowlord_name(word: &str) -> Option<&'static str> {
        Self::shadowlord_name_index(word).and_then(Self::shadowlord_title_for_index)
    }

    pub fn shadowlord_name_index(word: &str) -> Option<usize> {
        match word {
            "FAULINEI" => Some(SHADOWLORD_FALSEHOOD_INDEX),
            "ASTAROTH" => Some(SHADOWLORD_HATRED_INDEX),
            "NOSFENTOR" => Some(SHADOWLORD_COWARDICE_INDEX),
            _ => None,
        }
    }

    pub fn shadowlord_title_for_index(index: usize) -> Option<&'static str> {
        match index {
            SHADOWLORD_FALSEHOOD_INDEX => Some("Falsehood"),
            SHADOWLORD_HATRED_INDEX => Some("Hatred"),
            SHADOWLORD_COWARDICE_INDEX => Some("Cowardice"),
            _ => None,
        }
    }

    pub fn shadowlord_object_tile_for_index(index: usize) -> Option<u8> {
        (index < SHADOWLORD_COUNT).then_some(SHADOWLORD_ACTOR_TILE)
    }

    pub fn shadowlord_name_encounter_object(
        &self,
        index: usize,
        x: usize,
        y: usize,
        z: i8,
    ) -> Option<ActiveObject> {
        let tile = Self::shadowlord_object_tile_for_index(index)?;
        let dx = x as i8 - self.player.x as i8;
        let dy = y as i8 - self.player.y as i8;
        Some(ActiveObject {
            type_byte: tile,
            tile,
            x,
            y,
            z,
            phase: active_object_phase_toward_player(dx, dy),
            aux1: 0,
            aux3: 0,
        })
    }

    pub fn is_shadowlord_actor(object: ActiveObject) -> bool {
        !object.is_empty() && object.type_byte == SHADOWLORD_ACTOR_TILE
    }

    pub fn shadowlord_name_encounter_present(&self, index: usize) -> bool {
        if self.summoned_shadowlord != Some(index) {
            return false;
        }
        let Some(floor) = self.current_floor() else {
            return false;
        };
        self.active_objects
            .iter()
            .copied()
            .skip(1)
            .any(|object| object.z == floor && Self::is_shadowlord_actor(object))
    }

    /// `commands.md §11` / `town-mode.md §13` shared one-at-a-time
    /// predicate. Shadowlord identity is stored separately; any live
    /// `0xFC` actor blocks another summon or resident installation.
    pub fn shadowlord_actor_present(&self) -> bool {
        self.active_objects
            .iter()
            .skip(1)
            .any(|object| !object.is_empty() && object.type_byte == SHADOWLORD_ACTOR_TILE)
    }

    pub fn matching_shadowlord_name_encounter_north(&self, index: usize) -> bool {
        let Some(floor) = self.current_floor() else {
            return false;
        };
        let Some(y) = self.player.y.checked_sub(1) else {
            return false;
        };
        let x = self.player.x;
        self.active_objects.iter().copied().skip(1).any(|object| {
            object.z == floor
                && object.x == x
                && object.y == y
                && self.summoned_shadowlord == Some(index)
                && Self::is_shadowlord_actor(object)
        })
    }

    pub fn place_shadowlord_name_encounter(&mut self, index: usize) -> Option<usize> {
        let Area::Town { scene, .. } = self.area else {
            return None;
        };
        if !matches!(
            scene.byte,
            SCENE_THE_LYCAEUM | SCENE_EMPATH_ABBEY | SCENE_SERPENTS_HOLD
        ) || !self.shadowlord_alive(index)
            || self.player.y < 2
            || self.shadowlord_actor_present()
        {
            return None;
        }
        let z = self.current_floor()?;
        let (x, y) = (self.player.x, self.player.y - 2);
        let object = self.shadowlord_name_encounter_object(index, x, y, z)?;
        let slot = self.allocate_highest_empty_active_object_slot(object)?;
        self.summoned_shadowlord = Some(index);
        self.mark_visibility_dirty();
        Some(slot)
    }

    /// Highest-empty active-object discipline used by name/Yell summons.
    /// Slot zero remains the player; a full table has no empty slot.
    pub fn allocate_highest_empty_active_object_slot(
        &mut self,
        object: ActiveObject,
    ) -> Option<usize> {
        self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let slot = (1..OOL_SLOTS)
            .rev()
            .find(|slot| self.active_objects[*slot].is_empty())?;
        // An off-floor NPC may retain a link to a descriptor that is empty on
        // the current floor. Acquisition transfers ownership of that record;
        // detach the stale link so the next schedule pass cannot reclaim and
        // overwrite the summoned actor.
        for npc in &mut self.npcs {
            if npc.active_object == Some(slot) {
                npc.active_object = None;
            }
        }
        self.active_objects[slot] = object;
        Some(slot)
    }

    /// `town-mode.md §13` entry-time resident selector and actor install.
    /// `Some((None, index))` records a host whose actor was rejected by the
    /// shared one-at-a-time gate; `Some((Some(slot), index))` installed it.
    pub fn install_shadowlord_entry_encounter(&mut self) -> Option<(Option<usize>, usize)> {
        self.resident_shadowlord = None;
        if self.player.y == SHADOWLORD_TOWN_ENTRY_SKIP_Y {
            return None;
        }
        let Area::Town { scene, floor } = self.area else {
            return None;
        };
        let shadowlord_index = self
            .shadowlord_hideouts
            .iter()
            .position(|hideout| *hideout == scene.byte)?;
        self.resident_shadowlord = Some(shadowlord_index);
        let _ = self.apply_resident_shadowlord_blight_with_seed(host_clock_prng_seed_now());
        if self.shadowlord_actor_present() {
            return Some((None, shadowlord_index));
        }

        let active_slot = self.insert_resident_shadowlord_npc(scene, floor)?;
        let _ = self.apply_resident_shadowlord_npc_sweep(shadowlord_index);
        self.mark_visibility_dirty();
        Some((active_slot, shadowlord_index))
    }

    fn insert_resident_shadowlord_npc(&mut self, scene: Scene, floor: i8) -> Option<Option<usize>> {
        let y = shadowlord_town_install_row(scene.byte)?;
        let npc_slot = (1..OOL_SLOTS)
            .rev()
            .find(|slot| !self.npcs.iter().any(|npc| npc.slot == *slot))
            .unwrap_or(OOL_SLOTS - 1);
        if let Some(existing_index) = self.npcs.iter().position(|npc| npc.slot == npc_slot) {
            if let Some(active_slot) = self.npcs[existing_index].active_object {
                self.free_active_object_slot(active_slot);
            }
            self.npcs.remove(existing_index);
        }
        self.npcs.push(RuntimeNpc::from_resident_shadowlord(
            npc_slot,
            SHADOWLORD_TOWN_INSTALL_X,
            y,
            self.clock.hour,
        ));
        self.npcs.sort_by_key(|npc| npc.slot);

        let active_slot = if floor == 0 {
            let npc_index = self
                .npcs
                .iter()
                .position(|npc| npc.slot == npc_slot)
                .expect("resident Shadowlord descriptor was just inserted");
            self.sync_npc_active_object(npc_index, 0);
            self.npcs[npc_index].active_object
        } else {
            None
        };
        Some(active_slot)
    }

    /// `town-mode.md §3` resident-only deterministic terrain walk followed
    /// by the published fresh host-clock state replacement.
    pub fn apply_resident_shadowlord_blight_with_seed(
        &mut self,
        trailing_host_seed: u16,
    ) -> Option<usize> {
        self.resident_shadowlord?;
        let rewritten = apply_shadowlord_blight(
            &mut self.grid,
            self.clock.day,
            &mut self.prng_state,
            trailing_host_seed,
        );
        self.mark_visibility_dirty();
        Some(rewritten)
    }

    /// `town-mode.md §13`: consume one coin draw for every roster index,
    /// then apply the host-specific destructive schedule/dialogue sweep.
    pub fn apply_resident_shadowlord_npc_sweep(
        &mut self,
        shadowlord_index: usize,
    ) -> (usize, usize) {
        if !matches!(
            shadowlord_index,
            SHADOWLORD_HATRED_INDEX | SHADOWLORD_COWARDICE_INDEX
        ) {
            return (0, 0);
        }
        let draws = std::array::from_fn(|_| self.random_range_u8(0, 1));
        self.apply_resident_shadowlord_npc_sweep_with_draws(shadowlord_index, &draws)
    }

    /// Deterministic form of the resident sweep. The fixed-slot-4 type test is
    /// intentionally reproduced for compatibility with the published defect.
    pub fn apply_resident_shadowlord_npc_sweep_with_draws(
        &mut self,
        shadowlord_index: usize,
        draws: &[u8; NPC_SLOTS_PER_SUB_MAP],
    ) -> (usize, usize) {
        let fixed_type_passes = self
            .npcs
            .iter()
            .find(|npc| npc.slot == 4)
            .is_some_and(|npc| {
                (TOWN_NPC_ORDINARY_TYPE_FIRST..=TOWN_NPC_ORDINARY_TYPE_LAST)
                    .contains(&npc.type_byte)
            });
        if !fixed_type_passes {
            return (0, 0);
        }

        let mut pursued = 0;
        let mut cowering = 0;
        for roster_index in 0..NPC_SLOTS_PER_SUB_MAP {
            if draws[roster_index] != 0 {
                continue;
            }
            let Some(index) = self.npcs.iter().position(|npc| npc.slot == roster_index) else {
                continue;
            };
            if !self.npcs[index].has_nonzero_schedule_time_boundary() {
                continue;
            }
            match shadowlord_index {
                SHADOWLORD_HATRED_INDEX => {
                    self.npcs[index].force_town_pursuit();
                    self.npcs[index].dialog_id = TOWN_NPC_BRUSHOFF_DIALOG_ID;
                    self.record_town_npc_mutation(index);
                    pursued += 1;
                }
                SHADOWLORD_COWARDICE_INDEX => {
                    let _ = self.npcs[index].force_town_flight();
                    self.npcs[index].dialog_id = TOWN_NPC_COWERING_DIALOG_ID;
                    self.record_town_npc_mutation(index);
                    cowering += 1;
                }
                _ => {}
            }
        }
        (pursued, cowering)
    }

    /// Restore the resident Shadowlord's logical high-index NPC descriptor
    /// after an in-town floor reload rebuilt the ordinary roster from disk.
    pub fn restore_resident_shadowlord_after_floor_reload(&mut self) -> Option<usize> {
        let Area::Town { scene, floor } = self.area else {
            return None;
        };
        self.resident_shadowlord = None;
        if self.player.y == SHADOWLORD_TOWN_ENTRY_SKIP_Y {
            return None;
        }
        self.resident_shadowlord = self
            .shadowlord_hideouts
            .iter()
            .position(|hideout| *hideout == scene.byte);
        self.resident_shadowlord?;
        let _ = self.apply_resident_shadowlord_blight_with_seed(host_clock_prng_seed_now());
        self.insert_resident_shadowlord_npc(scene, floor).flatten()
    }

    pub fn shadowlord_slot_is_living(value: u8) -> bool {
        (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX).contains(&value)
    }

    pub fn shadowlord_slot_is_vanquished(value: u8) -> bool {
        value == SHADOWLORD_VANQUISHED
    }

    pub fn shadowlord_alive(&self, index: usize) -> bool {
        self.shadowlord_hideouts
            .get(index)
            .copied()
            .is_some_and(Self::shadowlord_slot_is_living)
    }

    pub fn shadowlord_vanquished(&self, index: usize) -> bool {
        self.shadowlord_hideouts
            .get(index)
            .copied()
            .is_some_and(Self::shadowlord_slot_is_vanquished)
    }

    pub fn all_shadowlords_vanquished(&self) -> bool {
        self.shadowlord_hideouts
            .iter()
            .copied()
            .all(Self::shadowlord_slot_is_vanquished)
    }

    pub fn vanquish_shadowlord(&mut self, index: usize) -> bool {
        if !self.shadowlord_alive(index) {
            return false;
        }
        self.shadowlord_hideouts[index] = SHADOWLORD_VANQUISHED;
        if let Some(slot) = shadowlord_stonegate_npc_slot(index) {
            let scene = Scene::new(STONEGATE_SCENE_BYTE)
                .expect("Stonegate scene byte is a valid town scene");
            self.mark_removed_town_npc_once(scene, slot);
        }
        true
    }

    pub fn stonegate_entry_presentation_message(&self) -> Option<String> {
        let Area::Town { scene, .. } = self.area else {
            return None;
        };
        if scene.byte != STONEGATE_SCENE_BYTE {
            return None;
        }

        let mut notes = Vec::new();
        if self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] != 0 {
            notes.push("Sceptre prelude".to_string());
        }
        for index in 0..SHADOWLORD_COUNT {
            if self.shadowlord_alive(index) {
                let shadowlord = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
                notes.push(format!("air of {shadowlord}"));
            }
        }
        (!notes.is_empty()).then(|| format!("Stonegate entry: {}.", notes.join("; ")))
    }

    pub fn append_stonegate_entry_presentation_message(&mut self) {
        let Some(note) = self.stonegate_entry_presentation_message() else {
            return;
        };
        if !self.message.is_empty() {
            self.message.push('\n');
        }
        self.message.push_str(&note);
    }

    pub fn current_shadowlord_hideout_id(&self) -> Option<u8> {
        let Area::Town { scene, .. } = self.area else {
            return None;
        };
        (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX)
            .contains(&scene.byte)
            .then_some(scene.byte)
    }

    pub fn reroll_shadowlord_hideouts(&mut self) -> usize {
        self.reroll_shadowlord_hideouts_excluding(self.current_shadowlord_hideout_id())
    }

    pub fn reroll_shadowlord_hideouts_excluding(&mut self, _current: Option<u8>) -> usize {
        let previous = self.shadowlord_hideouts;
        let mut rerolled = 0usize;

        for slot in 0..SHADOWLORD_COUNT {
            if !Self::shadowlord_slot_is_living(previous[slot]) {
                continue;
            }

            self.shadowlord_hideouts[slot] =
                self.random_range_u8(SHADOWLORD_HIDEOUT_MIN, SHADOWLORD_HIDEOUT_MAX);
            rerolled += 1;
        }

        rerolled
    }

    pub fn terrain_encounter_note(
        &self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        object: ActiveObject,
    ) -> io::Result<String> {
        let hostile_terrain = self.grid[world_cell_index(object.x, object.y)];
        let aboard_ship = matches!(self.player.transport, TransportState::Ship { .. });
        let Some(arena) = outdoor_combat_arena_index(
            object.type_byte,
            hostile_terrain,
            aboard_ship,
            SCENE_OVERWORLD,
        ) else {
            return Ok(format!(
                "no terrain-combat class for active-object type 0x{:02X}",
                object.type_byte
            ));
        };
        let variant = if plane == WorldPlane::Underworld || object.z < 0 {
            " underworld variant"
        } else {
            ""
        };
        let base_class = terrain_combat_base_class(object);
        let base_class_note = base_class
            .map(|stats| format!(", base class {} ({})", stats.name, stats.class))
            .unwrap_or_default();
        let Some(game_dir) = game_dir else {
            return Ok(format!(
                "selected BRIT.CBT arena {arena}{variant}{base_class_note}"
            ));
        };
        if !game_dir.join(BRIT_CBT_FILE).exists() {
            return Ok(format!(
                "selected BRIT.CBT arena {arena}{variant}{base_class_note}"
            ));
        }
        let bank = load_brit_cbt(game_dir)?;
        let record = bank.record(arena).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("BRIT.CBT has no arena record {arena}"),
            )
        })?;
        let setup = terrain_combat_setup_from_record_at_arena(plane, object, arena, record)?;
        let terrain_origin = setup.terrain[0][0];
        let first_placement =
            setup
                .placement_slots
                .first()
                .copied()
                .unwrap_or(CombatPlacementSlot {
                    slot: 0,
                    x: 0,
                    y: 0,
                });
        Ok(format!(
            "loaded BRIT.CBT arena {arena}{variant}{base_class_note} (terrain[0,0]=0x{terrain_origin:02X}, first placement ({}, {}))",
            first_placement.x, first_placement.y
        ))
    }

    pub fn attack_command(&mut self, direction: Option<Direction>) -> MoveOutcome {
        self.attack_command_with_game_dir(direction, None)
            .expect("attack without a game dir cannot load optional arena resources")
    }

    pub fn attack_command_with_game_dir(
        &mut self,
        direction: Option<Direction>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        if let Area::Dungeon { scene, level } = self.area {
            let Some((x, y)) = self.dungeon_forward_target() else {
                self.message = "Dungeon attack requires a cardinal facing direction.".to_string();
                return Ok(MoveOutcome::Blocked);
            };
            let Some((slot, object)) = self.dungeon_active_monster_at(x, y) else {
                self.message = format!(
                    "Attacked forward at ({x}, {y}) on {} level {level}; no target.",
                    scene.key()
                );
                return Ok(MoveOutcome::Blocked);
            };
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
            self.advance_turn();
            if combat_class_stats(object.aux1).is_some() {
                let note = self.enter_dungeon_active_monster_combat(level, object)?;
                self.message = format!(
                    "Attacked dungeon monster tile {} at ({x}, {y}) on {} level {level}; {note}.",
                    object.tile,
                    scene.key()
                );
                return Ok(MoveOutcome::Used);
            }
            self.message = format!(
                "Attacked dungeon object tile {} at ({x}, {y}) on {} level {level}; no published combat class.",
                object.tile,
                scene.key()
            );
            return Ok(MoveOutcome::Used);
        }
        let Some(direction) = direction else {
            self.message = "Attack where? Use A<direction>.".to_string();
            return Ok(MoveOutcome::PromptDeclined);
        };
        let Some((x, y)) = self.adjacent_target(direction) else {
            self.advance_turn();
            self.message = format!("Attacked {}; no target.", direction.name());
            return Ok(MoveOutcome::Blocked);
        };
        if let Some((object_slot, object)) = self
            .object_slot_at_current_floor(x, y)
            .map(|(slot, object)| (slot, *object))
        {
            if let Area::World { plane } = self.area {
                if let Some(game_dir) = game_dir {
                    if game_dir.join(BRIT_CBT_FILE).exists()
                        && !is_whirlpool_object(object)
                        && terrain_combat_base_class(object).is_some()
                    {
                        self.advance_turn();
                        let note = self.enter_terrain_combat_from_world_object(
                            game_dir,
                            plane,
                            object_slot,
                            object,
                        )?;
                        self.message = format!(
                            "Attacked object tile {} at ({x}, {y}) to the {} in slot {object_slot}; {note}.",
                            object.tile,
                            direction.name()
                        );
                        return Ok(MoveOutcome::Used);
                    }
                }
                self.advance_turn();
                let note = self.terrain_encounter_note(game_dir, plane, object)?;
                self.message = format!(
                    "Attacked object tile {} at ({x}, {y}) to the {} in slot {object_slot}; {note}.",
                    object.tile,
                    direction.name()
                );
                return Ok(MoveOutcome::Used);
            }
            self.advance_turn();
            if let Area::Town { scene, floor } = self.area {
                if let Some((npc_index, npc_slot, object_slot, type_byte)) =
                    self.town_attack_target_at(floor, x, y)
                {
                    match town_npc_attack_resolution(type_byte) {
                        TownNpcAttackResolution::DeathMask => {
                            self.free_active_object_slot(object_slot);
                            self.npcs.remove(npc_index);
                            self.mark_removed_town_npc_once(scene, npc_slot);
                            let (pursued, fled) =
                                self.town_alarm_sweep(scene, floor, Some(npc_slot));
                            self.mark_visibility_dirty();
                            self.message = format!(
                                "Attacked NPC slot {npc_slot} type 0x{type_byte:02X} at ({x}, {y}) to the {}; target removed from {} floor {floor}; alarm raised ({pursued} pursuing, {fled} fleeing).",
                                direction.name(),
                                scene.key()
                            );
                            return Ok(MoveOutcome::Used);
                        }
                        TownNpcAttackResolution::AlarmOnly => {
                            let (pursued, fled) =
                                self.town_alarm_sweep(scene, floor, Some(npc_slot));
                            self.message = format!(
                                "Attacked NPC slot {npc_slot} type 0x{type_byte:02X} at ({x}, {y}) to the {}; alarm raised ({pursued} pursuing, {fled} fleeing).",
                                direction.name()
                            );
                            return Ok(MoveOutcome::Used);
                        }
                        TownNpcAttackResolution::Refused => {
                            self.message = format!(
                                "Attacked NPC slot {npc_slot} type 0x{type_byte:02X} at ({x}, {y}) to the {}; no attackable town NPC.",
                                direction.name()
                            );
                            return Ok(MoveOutcome::Blocked);
                        }
                    }
                }
                self.message = format!(
                    "Attacked object tile {} at ({x}, {y}) to the {}; no attackable town NPC.",
                    object.tile,
                    direction.name()
                );
                return Ok(MoveOutcome::Blocked);
            }
            self.message = format!(
                "Attacked object tile {} at ({x}, {y}) to the {} in slot {object_slot}; no published combat class.",
                object.tile,
                direction.name()
            );
            return Ok(MoveOutcome::Used);
        }

        self.advance_turn();
        self.message = format!("Attacked {} at ({x}, {y}); no target.", direction.name());
        Ok(MoveOutcome::Blocked)
    }

    pub fn dungeon_forward_target(&self) -> Option<(usize, usize)> {
        let Area::Dungeon { .. } = self.area else {
            return None;
        };
        let direction = if self.player.facing.is_cardinal() {
            self.player.facing
        } else {
            return None;
        };
        let (dx, dy) = direction.delta();
        let x = (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
        let y = (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
        Some((x, y))
    }

    pub fn dungeon_active_monster_at(&self, x: usize, y: usize) -> Option<(usize, ActiveObject)> {
        let Area::Dungeon { level, .. } = self.area else {
            return None;
        };
        let object = self
            .active_objects
            .get(DUNGEON_ACTIVE_MONSTER_SLOT)
            .copied()?;
        (dungeon_monster_record_active(object)
            && object.z == level as i8
            && object.x == x
            && object.y == y)
            .then_some((DUNGEON_ACTIVE_MONSTER_SLOT, object))
    }

    pub fn town_attack_target_at(
        &self,
        floor: i8,
        x: usize,
        y: usize,
    ) -> Option<(usize, usize, usize, u8)> {
        if floor < 0 {
            return None;
        }
        let floor = floor as u8;
        self.npcs.iter().enumerate().find_map(|(index, npc)| {
            if npc.x != x || npc.y != y || npc.z != floor {
                return None;
            }
            let object_slot = npc.active_object.or_else(|| {
                self.active_objects
                    .iter()
                    .copied()
                    .enumerate()
                    .skip(1)
                    .find_map(|(slot, object)| {
                        active_object_matches_runtime_npc(object, npc, floor).then_some(slot)
                    })
            })?;
            Some((index, npc.slot, object_slot, npc.type_byte))
        })
    }

    pub fn adjacent_target(&self, direction: Direction) -> Option<(usize, usize)> {
        let (dx, dy) = direction.delta();
        match self.area {
            Area::World { .. } => {
                let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                Some((x, y))
            }
            Area::Town { .. } => {
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if (0..32).contains(&x) && (0..32).contains(&y) {
                    Some((x as usize, y as usize))
                } else {
                    None
                }
            }
            Area::Dungeon { .. } => {
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if (0..DUNGEON_SIDE as isize).contains(&x)
                    && (0..DUNGEON_SIDE as isize).contains(&y)
                {
                    Some((x as usize, y as usize))
                } else {
                    None
                }
            }
        }
    }

    pub fn fire_command(
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

    pub fn fire_town_source(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let entries = load_town_fire_source_entries(game_dir)?;
        self.tick_door_tracker();
        let Some(source) = self.town_fire_source(scene, floor, entries.as_deref()) else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        let target = self.town_fire_target(source);
        match target {
            TownFireTarget::Object { slot, object } => {
                self.free_active_object_slot(slot);
                self.mark_visibility_dirty();
                let moral_delta = self.decrease_moral_standing(TOWN_CANNON_HIT_KARMA_DEBIT);
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} and hit object tile {} at ({}, {}); target removed and moral standing decreased by {moral_delta}.",
                    source.x,
                    source.y,
                    source.direction.name(),
                    object.tile,
                    object.x,
                    object.y
                );
            }
            TownFireTarget::Door { x, y, tile } => {
                self.grid[y * 32 + x] = TOWN_DOOR_CLEARED_TILE;
                self.record_open_town_door(scene, floor, x, y);
                self.forget_revealed_town_secret_door(scene, floor, x, y);
                self.door_tracker = None;
                self.mark_visibility_dirty();
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Door destroyed! Town fire source at ({}, {}) fired {} and destroyed door tile {} at ({x}, {y}).",
                    source.x,
                    source.y,
                    source.direction.name(),
                    tile
                );
            }
            TownFireTarget::Wall { x, y, tile } => {
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} and hit blocking tile {} at ({x}, {y}).",
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

    pub fn town_fire_source(
        &self,
        scene: Scene,
        floor: i8,
        sidecar_entries: Option<&[TownFireSourceEntry]>,
    ) -> Option<TownFireSourceEntry> {
        if let Some(entry) = sidecar_entries.and_then(|entries| {
            entries
                .iter()
                .find(|entry| {
                    let entry = **entry;
                    entry.scene == scene
                        && entry.floor == floor
                        && town_fire_source_is_adjacent(entry, self.player.x, self.player.y)
                        && town_fire_source_tile_matches(entry, self.grid[entry.y * 32 + entry.x])
                })
                .copied()
        }) {
            return Some(entry);
        }

        self.adjacent_town_cannon_fire_source(scene, floor)
    }

    pub fn adjacent_town_cannon_fire_source(
        &self,
        scene: Scene,
        floor: i8,
    ) -> Option<TownFireSourceEntry> {
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    continue;
                }
                let x = x as usize;
                let y = y as usize;
                let tile = self.grid[y * 32 + x];
                let Some(direction) = town_cannon_tile_fire_direction(tile) else {
                    continue;
                };
                return Some(TownFireSourceEntry {
                    scene,
                    floor,
                    x,
                    y,
                    direction,
                    expected_tile: Some(tile),
                });
            }
        }
        None
    }

    pub fn town_fire_target(&self, source: TownFireSourceEntry) -> TownFireTarget {
        let (dx, dy) = source.direction.delta();
        for distance in 1..=(TOWN_CANNON_RANGE_CELLS as isize) {
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
            if town_command_door_tile(tile) {
                return TownFireTarget::Door { x, y, tile };
            }
            if surface_tile_blocks_projectile(tile) {
                return TownFireTarget::Wall { x, y, tile };
            }
        }
        TownFireTarget::None
    }

    pub fn fire_ship_broadside(&mut self, direction: Option<Direction>) -> MoveOutcome {
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
        let Some(ship_facing) = self.player.facing.cardinal_facing_index() else {
            self.message = "Fire broadsides only!".to_string();
            return MoveOutcome::Blocked;
        };
        let Some(fire_direction) = direction.cardinal_facing_index() else {
            self.message = "Fire broadsides only!".to_string();
            return MoveOutcome::Blocked;
        };
        if !ship_broadside_direction_accepted(ship_facing, fire_direction) {
            self.message = "Fire broadsides only!".to_string();
            return MoveOutcome::Blocked;
        }

        let hit = self
            .ship_broadside_target_slot(direction)
            .map(|slot| (slot, self.active_objects[slot]));
        let mut hit_report = None;
        if let Some((slot, object)) = hit {
            let damage = self.ship_broadside_damage_roll();
            if let Some(remaining) = ship_broadside_apply_damage(object.aux1, damage) {
                if let Some(target) = self.active_objects.get_mut(slot) {
                    target.aux1 = remaining;
                    hit_report = Some(format!(
                        "BOOOM! Ship broadside hit object tile {} at ({}, {}) for {damage} durability damage; durability now {remaining}.",
                        object.tile, object.x, object.y
                    ));
                }
            } else {
                self.free_active_object_slot(slot);
                hit_report = Some(format!(
                    "BOOOM! Ship broadside hit object tile {} at ({}, {}) for {damage} durability damage; target destroyed.",
                    object.tile, object.x, object.y
                ));
            }
            self.mark_visibility_dirty();
        }
        self.advance_turn();
        self.message = if let Some(report) = hit_report {
            report
        } else {
            format!(
                "BOOOM! Ship broadside fired {} with no target in range.",
                direction.name()
            )
        };
        MoveOutcome::Fired
    }

    pub fn ship_broadside_target_slot(&self, direction: Direction) -> Option<usize> {
        let (dx, dy) = direction.delta();
        for distance in 1..=SHIP_BROADSIDE_RANGE_CELLS as isize {
            let x = (self.player.x as isize + dx * distance).rem_euclid(WORLD_SIDE as isize);
            let y = (self.player.y as isize + dy * distance).rem_euclid(WORLD_SIDE as isize);
            let x = x as usize;
            let y = y as usize;
            if let Some((slot, _)) =
                self.active_objects
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, object)| {
                        self.object_occupies(**object, x, y) && !is_whirlpool_object(**object)
                    })
            {
                return Some(slot);
            }
        }
        None
    }

    pub fn ship_broadside_damage_roll(&mut self) -> u8 {
        self.random_range_u8(SHIP_BROADSIDE_DAMAGE_MIN, SHIP_BROADSIDE_DAMAGE_MAX)
    }

    pub fn decrease_moral_standing(&mut self, amount: u8) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self.moral_standing.saturating_sub(amount);
        before - self.moral_standing
    }

    pub fn push_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.push_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn push_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => {
                self.push_town_direction(game_dir, scene, floor, direction)
            }
            Area::Dungeon { .. } => {
                self.message = "What?".to_string();
                Ok(MoveOutcome::Blocked)
            }
            Area::World { .. } => Ok(self.push_world_direction(direction)),
        }
    }

    pub fn push_world_direction(&mut self, direction: Direction) -> MoveOutcome {
        if !direction.is_cardinal() {
            self.message = "Push requires a cardinal facing direction.".to_string();
            return MoveOutcome::Blocked;
        }
        let (tx, ty) = self.world_push_coordinate(self.player.x, self.player.y, direction, 1);
        let (px, py) = self.world_push_coordinate(self.player.x, self.player.y, direction, 2);
        let target_idx = world_cell_index(tx, ty);
        let target_tile = self.grid[target_idx];

        if let Some((slot, object)) = self.object_slot_at_current_floor(tx, ty) {
            return self.push_world_dynamic_object(direction, tx, ty, px, py, slot, *object);
        }

        let Some(family) = pushable_tile_family(target_tile) else {
            self.message = "Nothing to push there.".to_string();
            return MoveOutcome::Blocked;
        };

        self.push_world_static_family(direction, tx, ty, px, py, family)
    }

    pub fn push_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        if !self.combat_active || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        }
        if !direction.is_cardinal() {
            self.message = "Push requires a cardinal facing direction.".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        };
        if !combat_actor_is_active_not_dead(actor) {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        }

        let (dx, dy) = direction.delta();
        let sx = actor.x as isize + dx;
        let sy = actor.y as isize + dy;
        let dx2 = sx + dx;
        let dy2 = sy + dy;
        if !combat_arena_coordinate_in_bounds(sx as i16, sy as i16)
            || !combat_arena_coordinate_in_bounds(dx2 as i16, dy2 as i16)
        {
            self.message = "Nothing to push there.".to_string();
            return MoveOutcome::Blocked;
        }
        let sx = sx as usize;
        let sy = sy as usize;
        let dx2 = dx2 as usize;
        let dy2 = dy2 as usize;

        if let Some(blocking_slot) = self.combat_actor_slot_at(sx as u8, sy as u8, actor_slot) {
            self.message = format!("Push blocked by combatant in slot {blocking_slot}.");
            return MoveOutcome::Blocked;
        }

        if let Some((object_slot, object)) =
            self.combat_loose_object_slot_at(sx, sy, actor.active_object_slot as usize)
        {
            return self.push_combat_dynamic_object(
                actor_slot,
                direction,
                sx,
                sy,
                dx2,
                dy2,
                object_slot,
                object,
            );
        }

        let source_tile = self.combat_terrain[sy][sx];
        let Some(family) = pushable_tile_family(source_tile) else {
            self.message = "Nothing to push there.".to_string();
            return MoveOutcome::Blocked;
        };

        self.push_combat_static_family(actor_slot, direction, sx, sy, dx2, dy2, family)
    }

    fn push_combat_static_family(
        &mut self,
        actor_slot: usize,
        direction: Direction,
        sx: usize,
        sy: usize,
        dx2: usize,
        dy2: usize,
        family: PushableTileFamily,
    ) -> MoveOutcome {
        let actor = self.combat_actors[actor_slot];
        let source_tile = self.combat_terrain[sy][sx];
        let stamp = family.floor_stamp();
        if self.combat_cell_clear_for_push(dx2, dy2) && self.combat_terrain[dy2][dx2] == stamp {
            self.combat_terrain[sy][sx] = stamp;
            self.combat_terrain[dy2][dx2] = pushable_oriented_tile(source_tile, direction);
            self.finish_combat_push(actor_slot, sx, sy);
            self.message = format!(
                "Pushed combat tile {source_tile} {} from ({sx}, {sy}) to ({dx2}, {dy2}).",
                direction.name()
            );
            return MoveOutcome::Pushed;
        }

        if self.combat_terrain[actor.y as usize][actor.x as usize] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            self.combat_terrain[actor.y as usize][actor.x as usize] =
                pushable_oriented_tile(source_tile, pull_direction);
            self.combat_terrain[sy][sx] = stamp;
            self.finish_combat_push(actor_slot, sx, sy);
            self.message = format!(
                "Pulled combat tile {source_tile} {} from ({sx}, {sy}) to ({}, {}).",
                direction.name(),
                actor.x,
                actor.y
            );
            return MoveOutcome::Pushed;
        }

        self.message = "Push blocked; it won't budge.".to_string();
        MoveOutcome::Blocked
    }

    fn push_combat_dynamic_object(
        &mut self,
        actor_slot: usize,
        direction: Direction,
        sx: usize,
        sy: usize,
        dx2: usize,
        dy2: usize,
        object_slot: usize,
        object: ActiveObject,
    ) -> MoveOutcome {
        if !self.combat_cell_clear_for_push(dx2, dy2)
            || !is_probe_walkable(self.combat_terrain[dy2][dx2])
        {
            self.message = format!("Push blocked by combat arena cell ({dx2}, {dy2}).");
            return MoveOutcome::Blocked;
        }

        if let Some(moved) = self.active_objects.get_mut(object_slot) {
            moved.x = dx2;
            moved.y = dy2;
            moved.tile = pushable_oriented_tile(moved.tile, direction);
            moved.type_byte = pushable_oriented_tile(moved.type_byte, direction);
        }
        self.finish_combat_push(actor_slot, sx, sy);
        self.message = format!(
            "Pushed combat object tile {} {} from ({sx}, {sy}) to ({dx2}, {dy2}).",
            object.tile,
            direction.name()
        );
        MoveOutcome::Pushed
    }

    fn finish_combat_push(&mut self, actor_slot: usize, x: usize, y: usize) {
        let _ = commit_combat_actor_linked_position(
            &mut self.combat_actors[actor_slot],
            &mut self.active_objects,
            x as u8,
            y as u8,
        );
        self.mark_visibility_dirty();
    }

    pub fn combat_actor_slot_at(&self, x: u8, y: u8, except_slot: usize) -> Option<usize> {
        self.combat_actors
            .iter()
            .copied()
            .enumerate()
            .find_map(|(slot, actor)| {
                (slot != except_slot && combat_actor_occupies_arena_cell(actor, x, y))
                    .then_some(slot)
            })
    }

    pub fn combat_loose_object_slot_at(
        &self,
        x: usize,
        y: usize,
        actor_object_slot: usize,
    ) -> Option<(usize, ActiveObject)> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .find_map(|(slot, object)| {
                if slot == actor_object_slot || object.is_empty() || object.x != x || object.y != y
                {
                    return None;
                }
                let linked_to_live_actor = self.combat_actors.iter().copied().any(|actor| {
                    combat_actor_is_active_not_dead(actor)
                        && actor.active_object_slot as usize == slot
                });
                (!linked_to_live_actor).then_some((slot, object))
            })
    }

    fn combat_cell_clear_for_push(&self, x: usize, y: usize) -> bool {
        self.combat_actor_slot_at(x as u8, y as u8, usize::MAX)
            .is_none()
            && self
                .active_objects
                .iter()
                .copied()
                .all(|object| object.is_empty() || object.x != x || object.y != y)
    }

    fn world_push_coordinate(
        &self,
        x: usize,
        y: usize,
        direction: Direction,
        distance: isize,
    ) -> (usize, usize) {
        let (dx, dy) = direction.delta();
        (
            (x as isize + dx * distance).rem_euclid(WORLD_SIDE as isize) as usize,
            (y as isize + dy * distance).rem_euclid(WORLD_SIDE as isize) as usize,
        )
    }

    fn push_world_static_family(
        &mut self,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
        family: PushableTileFamily,
    ) -> MoveOutcome {
        let target_idx = world_cell_index(tx, ty);
        let dest_idx = world_cell_index(px, py);
        let player_idx = world_cell_index(self.player.x, self.player.y);
        let target_tile = self.grid[target_idx];
        let stamp = family.floor_stamp();
        if self.blocking_object_at(px, py).is_none() && self.grid[dest_idx] == stamp {
            self.grid[target_idx] = stamp;
            self.grid[dest_idx] = pushable_oriented_tile(target_tile, direction);
            self.finish_world_push(tx, ty);
            self.message = format!(
                "Pushed tile {target_tile} {} from ({tx}, {ty}) to ({px}, {py}).",
                direction.name()
            );
            return MoveOutcome::Pushed;
        }

        if self.grid[player_idx] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            let old_player_x = self.player.x;
            let old_player_y = self.player.y;
            self.grid[player_idx] = pushable_oriented_tile(target_tile, pull_direction);
            self.grid[target_idx] = stamp;
            self.finish_world_push(tx, ty);
            self.message = format!(
                "Pulled tile {target_tile} {} from ({tx}, {ty}) to ({old_player_x}, {old_player_y}).",
                direction.name()
            );
            return MoveOutcome::Pushed;
        }

        self.advance_turn();
        self.message = "Push blocked; it won't budge.".to_string();
        MoveOutcome::Blocked
    }

    fn push_world_dynamic_object(
        &mut self,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
        slot: usize,
        object: ActiveObject,
    ) -> MoveOutcome {
        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn_without_door_tick();
            self.message = format!("Push blocked by actor at ({px}, {py}).");
            return MoveOutcome::Blocked;
        }
        let dest_idx = world_cell_index(px, py);
        if !self.tile_walkable(self.grid[dest_idx]) {
            self.advance_turn();
            self.message = format!(
                "Push blocked by {} at ({px}, {py}).",
                tile_class(self.grid[dest_idx])
            );
            return MoveOutcome::Blocked;
        }

        if let Some(moved) = self.active_objects.get_mut(slot) {
            moved.x = px;
            moved.y = py;
            moved.tile = pushable_oriented_tile(moved.tile, direction);
            moved.type_byte = pushable_oriented_tile(moved.type_byte, direction);
        }
        self.finish_world_push(tx, ty);
        self.message = format!(
            "Pushed object tile {} {} from ({tx}, {ty}) to ({px}, {py}).",
            object.tile,
            direction.name()
        );
        MoveOutcome::Pushed
    }

    fn finish_world_push(&mut self, tx: usize, ty: usize) {
        self.player.x = tx;
        self.player.y = ty;
        self.sync_player_object();
        let _ = self.refresh_world_live_chunks_for_current_area();
        self.mark_visibility_dirty();
        self.advance_turn();
    }

    pub fn push_town_facing(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        self.push_town_direction(game_dir, scene, floor, self.player.facing)
    }

    pub fn push_town_direction(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        if !direction.is_cardinal() {
            self.message = "Push requires a cardinal facing direction.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        self.tick_door_tracker();
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
        let target_idx = ty * 32 + tx;
        let target_tile = self.grid[target_idx];
        let entries = load_town_pushable_entries(game_dir)?;
        let sidecar_pushable = entries.as_ref().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| town_pushable_matches(*entry, scene, floor, tx, ty, target_tile))
        });

        if let Some((slot, object)) = self.object_slot_at_current_floor(tx, ty) {
            return Ok(self.push_town_dynamic_object(direction, tx, ty, px, py, slot, *object));
        }

        let Some(family) = pushable_tile_family(target_tile) else {
            if sidecar_pushable {
                return Ok(self.push_town_sidecar_tile(scene, floor, direction, tx, ty, px, py));
            }
            self.message = "Nothing to push there.".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        Ok(self.push_town_static_family(scene, floor, direction, tx, ty, px, py, family))
    }

    fn push_town_sidecar_tile(
        &mut self,
        scene: Scene,
        floor: i8,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
    ) -> MoveOutcome {
        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn();
            self.message = format!("Push blocked by actor at ({px}, {py}).");
            return MoveOutcome::Blocked;
        }
        let target_idx = ty * 32 + tx;
        let dest_idx = py * 32 + px;
        let target_tile = self.grid[target_idx];
        let dest_tile = self.grid[dest_idx];
        if !self.tile_walkable(dest_tile) {
            self.advance_turn_without_door_tick();
            self.message = format!("Push blocked by {} at ({px}, {py}).", tile_class(dest_tile));
            return MoveOutcome::Blocked;
        }

        self.grid[target_idx] = dest_tile;
        self.grid[dest_idx] = target_tile;
        self.finish_town_push(scene, floor, tx, ty, px, py);
        self.message = format!(
            "Pushed tile {target_tile} {} from ({tx}, {ty}) to ({px}, {py}).",
            direction.name()
        );
        MoveOutcome::Pushed
    }

    fn push_town_static_family(
        &mut self,
        scene: Scene,
        floor: i8,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
        family: PushableTileFamily,
    ) -> MoveOutcome {
        let target_idx = ty * 32 + tx;
        let dest_idx = py * 32 + px;
        let player_idx = self.player.y * 32 + self.player.x;
        let target_tile = self.grid[target_idx];
        let stamp = family.floor_stamp();
        if self.blocking_object_at(px, py).is_none() && self.grid[dest_idx] == stamp {
            self.grid[target_idx] = stamp;
            self.grid[dest_idx] = pushable_oriented_tile(target_tile, direction);
            self.finish_town_push(scene, floor, tx, ty, px, py);
            self.message = format!(
                "Pushed tile {target_tile} {} from ({tx}, {ty}) to ({px}, {py}).",
                direction.name()
            );
            return MoveOutcome::Pushed;
        }

        if self.grid[player_idx] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            let old_player_x = self.player.x;
            let old_player_y = self.player.y;
            self.grid[player_idx] = pushable_oriented_tile(target_tile, pull_direction);
            self.grid[target_idx] = stamp;
            self.finish_town_push(scene, floor, tx, ty, old_player_x, old_player_y);
            self.message = format!(
                "Pulled tile {target_tile} {} from ({tx}, {ty}) to ({}, {}).",
                direction.name(),
                player_idx % 32,
                player_idx / 32
            );
            return MoveOutcome::Pushed;
        }

        self.advance_turn_without_door_tick();
        self.message = "Push blocked; it won't budge.".to_string();
        MoveOutcome::Blocked
    }

    fn push_town_dynamic_object(
        &mut self,
        direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
        slot: usize,
        object: ActiveObject,
    ) -> MoveOutcome {
        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn_without_door_tick();
            self.message = format!("Push blocked by actor at ({px}, {py}).");
            return MoveOutcome::Blocked;
        }
        let dest_idx = py * 32 + px;
        if !self.tile_walkable(self.grid[dest_idx]) {
            self.advance_turn_without_door_tick();
            self.message = format!(
                "Push blocked by {} at ({px}, {py}).",
                tile_class(self.grid[dest_idx])
            );
            return MoveOutcome::Blocked;
        }

        if let Some(moved) = self.active_objects.get_mut(slot) {
            moved.x = px;
            moved.y = py;
            moved.tile = pushable_oriented_tile(moved.tile, direction);
            moved.type_byte = pushable_oriented_tile(moved.type_byte, direction);
        }
        self.player.x = tx;
        self.player.y = ty;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn_without_door_tick();
        self.message = format!(
            "Pushed object tile {} {} from ({tx}, {ty}) to ({px}, {py}).",
            object.tile,
            direction.name()
        );
        MoveOutcome::Pushed
    }

    fn finish_town_push(
        &mut self,
        scene: Scene,
        floor: i8,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
    ) {
        self.forget_open_town_door(scene, floor, tx, ty);
        self.forget_open_town_door(scene, floor, px, py);
        self.forget_revealed_town_secret_door(scene, floor, tx, ty);
        self.forget_revealed_town_secret_door(scene, floor, px, py);
        if self.door_tracker.is_some_and(|tracker| {
            (tracker.x == tx && tracker.y == ty) || (tracker.x == px && tracker.y == py)
        }) {
            self.door_tracker = None;
        }
        self.player.x = tx;
        self.player.y = ty;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn_without_door_tick();
    }

    #[cfg(test)]
    pub fn pass_turn(&mut self) -> MoveOutcome {
        self.pass_turn_with_game_dir(None)
            .expect("sidecar-free pass cannot fail")
    }

    pub fn pass_turn_with_game_dir(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
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
            self.append_pending_hourly_status_message();
        }
        Ok(MoveOutcome::Passed)
    }

    pub fn apply_top_down_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        let outcome = match self.area {
            Area::World { .. } => {
                self.apply_world_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Town { .. } => {
                self.apply_town_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Dungeon { .. } => {
                self.apply_dungeon_post_turn_effects_after_turn(turn_before, game_dir)
            }
        }?;
        if outcome.is_none() {
            self.append_pending_hourly_status_message();
        }
        Ok(outcome)
    }

    pub fn apply_post_turn_effects_after_outcome(
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

    pub fn apply_world_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
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
        // active-objects.md §8: adjacent whirlpool engagement is a
        // plane-transition effect when the party is not on foot.
        let object_epilogue_runs = self.world_object_epilogue_runs_for_turn(turn_before);
        if object_epilogue_runs {
            if let Some(outcome) = self.apply_pending_outdoor_reactions(game_dir, plane)? {
                if outcome.is_transition() {
                    let transition_message = self.message.clone();
                    self.message = if pre_effect_message.is_empty() {
                        transition_message
                    } else {
                        format!("{pre_effect_message} {transition_message}")
                    };
                } else if self.combat_active {
                    let engagement_message = self.message.clone();
                    self.message = if pre_effect_message.is_empty() {
                        engagement_message
                    } else {
                        format!("{pre_effect_message} {engagement_message}")
                    };
                } else {
                    // Immediate reaction lines were emitted to the transcript
                    // as they occurred; preserve the command result in the
                    // compatibility message slot.
                    self.message = pre_effect_message;
                }
                return Ok(Some(outcome));
            }
        }
        self.apply_fixed_narrative_gate_branch(plane);
        self.append_world_damage_tile_message(Some(game_dir), plane)?;
        self.append_world_status_tile_message(plane);
        if object_epilogue_runs {
            if let Some(slot) = self.apply_world_encounter_probe(game_dir, plane)? {
                self.message
                    .push_str(&format!(" Wandering encounter spawned in slot {slot}."));
            }
        }
        Ok(None)
    }

    /// Resolve the high-to-low reaction list staged by the active-object
    /// walker. A terrain combat pauses this list in place; production input
    /// dispatch resumes it immediately after the combat frame returns.
    pub fn apply_pending_outdoor_reactions(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<MoveOutcome>> {
        let mut reacted = false;
        while !self.pending_outdoor_reaction_slots.is_empty() {
            let slot = self.pending_outdoor_reaction_slots.remove(0);
            let Some(object) = self.active_objects.get(slot).copied() else {
                continue;
            };
            if object.is_empty() || object.z != plane.save_floor() {
                continue;
            }

            if self.outdoor_active_object_is_adjacent(slot) {
                if is_whirlpool_object(object) {
                    let outcome =
                        self.apply_world_whirlpool_slot_engagement(game_dir, plane, slot)?;
                    reacted = true;
                    if outcome.is_transition() {
                        self.pending_outdoor_reaction_slots.clear();
                        return Ok(Some(outcome));
                    }
                    continue;
                }
                if outdoor_sand_trap_class(object.type_byte) {
                    self.apply_world_sand_trap_slot_engagement(slot);
                    reacted = true;
                    continue;
                }
                if let Some(outcome) = self
                    .apply_world_generic_adjacent_slot_engagement(game_dir, plane, slot, object)?
                {
                    reacted = true;
                    if self.combat_active {
                        return Ok(Some(outcome));
                    }
                }
                continue;
            }

            if self
                .outdoor_first_phase_ranged_attack_detail(slot)
                .is_some()
            {
                reacted = true;
            }
        }
        Ok(reacted.then_some(MoveOutcome::Used))
    }

    pub fn apply_world_whirlpool_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(whirlpool_slot) = (1..self.active_objects.len()).rev().find(|slot| {
            self.outdoor_active_object_is_adjacent(*slot)
                && is_whirlpool_object(self.active_objects[*slot])
        }) else {
            return Ok(None);
        };
        self.apply_world_whirlpool_slot_engagement(game_dir, plane, whirlpool_slot)
            .map(Some)
    }

    fn apply_world_whirlpool_slot_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        whirlpool_slot: usize,
    ) -> io::Result<MoveOutcome> {
        // `overworld.md §8` correction: both marker arms reach the shared
        // §6.2.4 impact payload. On foot this is the whole-party damage pass;
        // it skips only the transition and does not clear the whirlpool slot.
        if self.player.transport.is_foot() {
            self.apply_outdoor_impact();
            return Ok(MoveOutcome::Used);
        }

        // Apply impact before changing planes so absorption sees the restored
        // original transport marker. A frigate therefore takes its hull roll
        // (and can sink) immediately before the durable transition.
        self.free_active_object_slot(whirlpool_slot);
        self.apply_outdoor_impact();
        let entry = WorldPlaneTransitionEntry {
            from_plane: plane,
            x: self.player.x,
            y: self.player.y,
            to_plane: WorldPlane::Underworld,
            to_x: 34,
            to_y: 18,
            expected_tile: None,
        };
        self.apply_world_plane_transition(game_dir, entry)?;
        self.message = format!(
            "Whirlpool! Sucked into the underworld at ({}, {}). {}",
            entry.to_x,
            entry.to_y,
            self.wind_status_message()
        );
        Ok(MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: plane,
            to: WorldPlane::Underworld,
        }))
    }

    pub fn apply_world_sand_trap_engagement(&mut self, plane: WorldPlane) -> Option<MoveOutcome> {
        let sand_trap_slot = (1..self.active_objects.len()).rev().find(|slot| {
            let object = self.active_objects[*slot];
            if object.z != plane.save_floor() || !outdoor_sand_trap_class(object.type_byte) {
                return false;
            }
            let (dx, dy) = wrapped_deltas_to_player(
                object.x as u8,
                object.y as u8,
                self.player.x as u8,
                self.player.y as u8,
            );
            outdoor_offsets_are_orthogonally_adjacent(dx, dy)
        })?;

        // `active-objects.md §8`: the sand-trap adjacency arm reaches the
        // shared impact payload directly and silently. It is not combat and
        // does not clear the active-object slot.
        self.apply_world_sand_trap_slot_engagement(sand_trap_slot);
        Some(MoveOutcome::Used)
    }

    fn apply_world_sand_trap_slot_engagement(&mut self, sand_trap_slot: usize) {
        let _ = sand_trap_slot;
        self.apply_outdoor_impact();
    }

    pub fn apply_town_post_turn_effects_after_turn(
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
        // `town-mode.md §7`: the town underfoot handler starts by giving
        // every sleeping active member an independent 1-in-16 wake roll.
        // This intentionally precedes trapdoor damage and every other tile
        // effect, including on the automatic no-input sleep pass.
        for member in &mut self.party {
            if member.status == b'S'
                && u5_prng_range_u16(&mut self.prng_state, 0, u16::from(TOWN_SLEEP_WAKE_ROLL_MAX))
                    == 0
            {
                member.status = b'G';
            }
        }
        let tile = self.grid[self.player.y * 32 + self.player.x];
        if !matches!(self.player.transport, TransportState::Carpet { .. }) {
            if let Some(entry) =
                self.town_trap_door_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
            {
                let pre_effect_message = self.message.clone();
                self.apply_town_trapdoor_party_damage();
                if scene.byte == STONEGATE_SCENE_BYTE {
                    self.apply_stonegate_trapdoor_script(floor);
                    self.message = if pre_effect_message.is_empty() {
                        "A TRAPDOOR!".to_string()
                    } else {
                        format!("{pre_effect_message} A TRAPDOOR!")
                    };
                    return Ok(Some(MoveOutcome::Used));
                }
                let outcome =
                    self.apply_town_trap_door_transition(game_dir, scene, entry, false)?;
                let transition_message = self.message.clone();
                self.message = if pre_effect_message.is_empty() {
                    transition_message
                } else {
                    format!("{pre_effect_message} {transition_message}")
                };
                return Ok(Some(outcome));
            }
        }
        self.apply_town_npc_contact_event(scene, floor)
    }

    pub fn apply_town_trapdoor_party_damage(&mut self) {
        let slots = self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
        for slot in 0..slots {
            if self.party[slot].status == CharacterStatus::Dead.save_byte() {
                continue;
            }
            let damage = self.random_range_u8(1, TRAP_BOMB_DAMAGE_MAX);
            self.apply_shared_party_damage(slot, damage);
        }
    }

    /// Stonegate's trapdoor exception after the shared mass-damage pass.
    ///
    /// Public issue `cleak/u5-spec#123`, resolved at clean-spec commit
    /// `4d03a662`, confirms that this is a same-scene scripted defeat rather
    /// than a floor transition or a durable rescue flag. The presentation is
    /// recorded separately; the durable mutations here match the cutscene's
    /// live-grid, active-object, and party writes.
    pub fn apply_stonegate_trapdoor_script(&mut self, floor: i8) {
        self.pending_stonegate_trapdoor_playback =
            Some(StonegateTrapdoorPlayback::complete(self.party.len()));

        self.grid.fill(STONEGATE_TRAPDOOR_GRID_TILE);
        self.mark_visibility_dirty();

        self.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];

        for member in &mut self.party {
            member.hp = 0;
            member.status = CharacterStatus::Dead.save_byte();
        }

        // The status/provision tail follows the deaths. If this action crossed
        // an hour boundary, its normal pass was deferred by `advance_turn` so
        // it observes zero provision eaters and no regenerating/poisoned live
        // members. The same tail clears an in-party active-member selector.
        if self
            .active_player
            .is_some_and(|slot| slot < self.party.len())
        {
            self.active_player = None;
        }
        if std::mem::take(&mut self.pending_stonegate_status_provision_pass) {
            self.apply_hourly_status_provision_pass();
        }

        // The normal coordinate-only record-zero tail follows the all-zero
        // script boundary. The zero-type animal pass cannot move it; the NPC
        // schedule then retains its ordinary opportunity to write records.
        self.active_objects[0].x = self.player.x;
        self.active_objects[0].y = self.player.y;
        self.active_objects[0].z = floor;
        if std::mem::take(&mut self.pending_stonegate_object_epilogue) {
            self.advance_active_objects();
            self.advance_npc_schedules();
        }
    }

    pub fn town_poison_gas_at(
        &self,
        _game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownPoisonGasEntry>> {
        let transport_marker = self.player.transport.save_marker();
        if town_poison_gas_live_tile_matches(tile, transport_marker) {
            return Ok(Some(TownPoisonGasEntry {
                scene,
                floor,
                x,
                y,
                expected_tile: Some(tile),
            }));
        }
        Ok(None)
    }

    pub fn append_town_poison_gas_message(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<()> {
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let Some(entry) =
            self.town_poison_gas_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        else {
            return Ok(());
        };
        let report = self.apply_town_poison_gas(entry);
        self.message.push_str(&format!(" {report}."));
        Ok(())
    }

    pub fn apply_town_poison_gas(&mut self, _entry: TownPoisonGasEntry) -> String {
        let mut checked = 0usize;
        let mut poisoned = Vec::new();
        for (member_index, member) in self.party.iter_mut().enumerate() {
            if member.status == b'D' || member.status == b'P' {
                continue;
            }
            checked += 1;
            let roll = u5_prng_range_u16(&mut self.prng_state, 0, TOWN_GAS_DOORWAY_RANGE_MAX);
            if roll > u16::from(member.climb_stat) {
                member.status = b'P';
                poisoned.push((member_index, member.slot));
            }
        }
        if poisoned.is_empty() {
            format!("poison gas doorway checked {checked} eligible member(s); no poison")
        } else {
            poisoned
                .iter()
                .map(|(member_index, slot)| {
                    let name = self
                        .party_names
                        .get(*member_index)
                        .and_then(|name| party_name_to_string(name))
                        .unwrap_or_else(|| format!("Party member {slot}"));
                    format!("{name} is poisoned!")
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    pub fn apply_town_npc_contact_event(
        &mut self,
        scene: Scene,
        floor: i8,
    ) -> io::Result<Option<MoveOutcome>> {
        if floor < 0 {
            return Ok(None);
        }
        let Some((npc_slot, type_byte, behavior)) = self.town_adjacent_event_npc(scene, floor)
        else {
            return Ok(None);
        };
        if let Some(index) = self.npcs.iter().position(|npc| npc.slot == npc_slot)
            && self.npcs[index].dialog_id == TOWN_NPC_BRUSHOFF_DIALOG_ID
        {
            let _ = self.npcs[index].force_town_flight();
            self.record_town_npc_mutation(index);
            self.message = TOWN_NPC_BRUSHOFF_RESPONSE.to_string();
            return Ok(Some(MoveOutcome::Used));
        }
        if behavior.raises_guard_event() {
            if let Some((x, y)) = self.npcs.iter().find_map(|npc| {
                (npc.slot == npc_slot && npc.dialog_id == BLACKTHORN_GUARD_DEMAND_DIALOG_ID)
                    .then_some((npc.x, npc.y))
            }) {
                return Ok(Some(self.begin_blackthorn_guard_demand(x, y, false)));
            }
            self.pending_town_arrest = Some(TownArrestPrompt {
                scene_byte: scene.byte,
                floor,
                npc_slot,
            });
            self.message = if self.message.is_empty() {
                format!("Guard NPC slot {npc_slot} catches the party. Surrender? (Y/N).")
            } else {
                format!(
                    "{} Guard NPC slot {npc_slot} catches the party. Surrender? (Y/N).",
                    self.message
                )
            };
            return Ok(Some(MoveOutcome::Used));
        }
        if behavior.raises_attack_event() {
            let (pursued, fled) = self.town_alarm_sweep(scene, floor, Some(npc_slot));
            self.message = if self.message.is_empty() {
                format!(
                    "Hostile NPC slot {npc_slot} (type {type_byte}) attacks; alarm raised ({pursued} pursuing, {fled} fleeing)."
                )
            } else {
                format!(
                    "{} Hostile NPC slot {npc_slot} (type {type_byte}) attacks; alarm raised ({pursued} pursuing, {fled} fleeing).",
                    self.message
                )
            };
            return Ok(Some(MoveOutcome::Used));
        }
        Ok(None)
    }

    pub fn town_adjacent_event_npc(
        &self,
        _scene: Scene,
        floor: i8,
    ) -> Option<(usize, u8, NpcAiBehavior)> {
        let floor_u8 = floor as u8;
        self.npcs.iter().find_map(|npc| {
            if npc.z != floor_u8
                || npc.x.abs_diff(self.player.x) + npc.y.abs_diff(self.player.y) != 1
            {
                return None;
            }
            let wp = waypoint_for_hour(&npc.schedule, self.clock.hour);
            let behavior = npc_ai_behavior(npc.schedule[NPC_SCHEDULE_AI_OFFSET + wp])?;
            // `doors-and-z-transitions.md §3.1`: released prisoners retain
            // mode 5 pursuit for the current visit, but clearing their live
            // dialogue/awareness byte suppresses mode 5's adjacent attack
            // event. Other attacking AI families are not dialogue-gated.
            let raises_attack = behavior.raises_attack_event()
                && !(behavior == NpcAiBehavior::ReservedEngage
                    && npc.dialog_id == NPC_DIALOG_ID_NONE);
            (raises_attack || behavior.raises_guard_event()).then_some((
                npc.slot,
                npc.type_byte,
                behavior,
            ))
        })
    }

    pub fn resolve_town_arrest_prompt(
        &mut self,
        key: char,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(prompt) = self.pending_town_arrest else {
            return Ok(None);
        };
        match key.to_ascii_lowercase() {
            'y' => {
                self.pending_town_arrest = None;
                if prompt.scene_byte == BLACKTHORN_CAPTIVE_CELL_SCENE {
                    return self.begin_blackthorn_audience_capture(game_dir);
                }
                self.apply_town_arrest_surrender(game_dir)
            }
            'n' => {
                self.pending_town_arrest = None;
                let scene = Scene::new(prompt.scene_byte)?;
                let (pursued, fled) =
                    self.town_alarm_sweep(scene, prompt.floor, Some(prompt.npc_slot));
                self.message = format!(
                    "Refused surrender; alarm raised ({pursued} pursuing, {fled} fleeing)."
                );
                Ok(Some(MoveOutcome::Used))
            }
            _ => {
                self.message = "Surrender? (Y/N).".to_string();
                Ok(Some(MoveOutcome::PromptDeclined))
            }
        }
    }

    pub fn apply_town_arrest_surrender(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let scene = Scene::new(TOWN_ARREST_JAIL_SCENE)?;
        let floor = TOWN_ARREST_JAIL_FLOOR as i8;
        let (grid, beacon_sources) =
            load_town_runtime_floor_with_beacon_sources(game_dir, scene, floor, self.clock.hour)?;
        self.grid = grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town { scene, floor };
        self.player.x = TOWN_ARREST_JAIL_X as usize;
        self.player.y = TOWN_ARREST_JAIL_Y as usize;
        self.player.transport = TransportState::Foot;
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        let mut cleanup_ticks = 0;
        while !town_arrest_release_loop_done(self.clock.hour) && cleanup_ticks < 96 {
            self.advance_turn_with_minutes_and_door_tick(
                TOWN_ARREST_CLEANUP_INCREMENT_MINUTES,
                false,
            );
            cleanup_ticks += 1;
        }
        self.message = format!(
            "Surrendered to the guards; jailed in {} at ({}, {}) until {:02}:00.",
            scene.key(),
            self.player.x,
            self.player.y,
            self.clock.hour
        );
        Ok(Some(MoveOutcome::Transition(
            AreaTransition::EnteredLocation(scene),
        )))
    }

    pub fn begin_blackthorn_audience_capture(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        self.pending_town_arrest = None;
        self.blackthorn_audience_map = None;
        self.clear_non_player_active_objects();
        self.mark_visibility_dirty();

        let Some(target_slot) = self.next_blackthorn_challenge_target_slot() else {
            let outcome = self.apply_blackthorn_captive_cell_handoff(
                game_dir,
                "Blackthorn audience found no eligible party member.",
            )?;
            return Ok(Some(outcome));
        };

        let mut challenge = crate::blackthorn_session::BlackthornChallenge::new();
        let prompt = match challenge.begin() {
            crate::blackthorn_session::BlackthornChallengeOutcome::PromptPresented {
                prompt,
                ..
            } => prompt,
            _ => "Virtue",
        };
        let opening = self
            .blackthorn_audience_opening_text(game_dir)?
            .unwrap_or_else(|| {
                "the party is overcome and dragged before Lord Blackthorn".to_string()
            });
        self.blackthorn_audience_map =
            load_miscmaps_cutscene_map(game_dir, BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD)?;
        self.install_blackthorn_audience_actors();
        let approach =
            self.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::AudienceThroneApproach);
        let rise = self.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::BlackthornRises);
        self.active_blackthorn = Some(challenge);
        self.message = format!(
            "Blackthorn audience: {opening}. Opening cutscene pause {}, output {} byte(s). Party slot {} is challenged for {prompt}.",
            approach.pause_ticks,
            rise.output_bytes.len(),
            target_slot + 1,
        );
        Ok(Some(MoveOutcome::Used))
    }

    pub fn blackthorn_audience_opening_text(&self, game_dir: &Path) -> io::Result<Option<String>> {
        let Some(messages) = load_misc_messages(game_dir)? else {
            return Ok(None);
        };
        Ok(messages
            .blackthorn_audience()
            .iter()
            .map(|record| record.trim())
            .find(|record| !record.is_empty())
            .map(str::to_string))
    }

    pub fn install_blackthorn_audience_actors(&mut self) {
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            if placement.actor == BlackthornCutsceneActor::SecondPartyMember
                && self.party.len() <= BLACKTHORN_FAILURE_VICTIM_SLOT
            {
                continue;
            }
            let slot = placement.actor.slot_index() as usize;
            self.active_objects[slot] = ActiveObject {
                type_byte: placement.type_byte,
                tile: placement.tile,
                x: placement.x,
                y: placement.y,
                z: 0,
                phase: STEADY_PHASE,
                aux1: placement.actor.slot_index(),
                aux3: BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER,
            };
        }
    }

    pub fn blackthorn_cutscene_vm_from_audience_state(&self) -> BlackthornCutsceneVm {
        let tile_buffer = self
            .blackthorn_audience_map
            .as_ref()
            .map(|map| map.tiles.clone())
            .unwrap_or_else(|| vec![0; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS]);
        let mut vm = BlackthornCutsceneVm::new(tile_buffer);
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            let slot = placement.actor.slot_index() as usize;
            let Some(object) = self.active_objects.get(slot).copied() else {
                continue;
            };
            if object.is_empty() || object.aux3 != BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER {
                continue;
            }
            vm.set_actor(
                placement.actor,
                BlackthornCutsceneActorState {
                    x: object.x,
                    y: object.y,
                    visible: true,
                },
            );
        }
        vm
    }

    pub fn apply_blackthorn_cutscene_vm_to_audience_state(&mut self, vm: &BlackthornCutsceneVm) {
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
            let slot = placement.actor.slot_index() as usize;
            if let Some(state) = vm.actor(placement.actor) {
                self.active_objects[slot] = ActiveObject {
                    type_byte: placement.type_byte,
                    tile: placement.tile,
                    x: state.x,
                    y: state.y,
                    z: 0,
                    phase: STEADY_PHASE,
                    aux1: placement.actor.slot_index(),
                    aux3: BLACKTHORN_CUTSCENE_AUX3_ROLE_MARKER,
                };
            } else {
                self.active_objects[slot] = ActiveObject::empty();
            }
        }
        match self.blackthorn_audience_map.as_mut() {
            Some(map) => map.tiles = vm.tile_buffer.clone(),
            None => {
                self.blackthorn_audience_map = Some(MiscmapsCutsceneMap {
                    record_index: BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD,
                    tiles: vm.tile_buffer.clone(),
                });
            }
        }
        self.mark_visibility_dirty();
    }

    pub fn run_blackthorn_cutscene_beat(
        &mut self,
        beat: BlackthornCutsceneBeat,
    ) -> BlackthornCutsceneVm {
        let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
        vm.run(blackthorn_cutscene_beat_commands(beat));
        self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
        vm
    }

    pub fn submit_blackthorn_audience_answer(
        &mut self,
        typed: &str,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let answer = blackthorn_challenge_limited_input(typed);
        if answer.is_empty() {
            self.message = self.blackthorn_current_prompt_message();
            return Ok(MoveOutcome::PromptDeclined);
        }

        let Some(mut challenge) = self.active_blackthorn.take() else {
            self.message = "No Blackthorn audience is active.".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        match challenge.submit(&answer) {
            crate::blackthorn_session::BlackthornChallengeOutcome::Correct { ordinal } => {
                let handled_slot = self.mark_blackthorn_current_target_handled();
                let vm = self
                    .run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::PerQuestionIntermission);
                if self.next_blackthorn_challenge_target_slot().is_none() {
                    self.run_blackthorn_cutscene_beat(
                        BlackthornCutsceneBeat::ConditionalThroneCleanup,
                    );
                    return self.apply_blackthorn_captive_cell_handoff(
                        game_dir,
                        &format!(
                            "Answered Blackthorn's prompt {} correctly; party slot {} is handled; cutscene pause {}.",
                            ordinal + 1,
                            handled_slot.map(|slot| slot + 1).unwrap_or(0),
                            vm.pause_ticks
                        ),
                    );
                }
                self.active_blackthorn = Some(challenge);
                self.message = format!(
                    "Answered Blackthorn's prompt {} correctly; party slot {} is handled; cutscene pause {}. {}",
                    ordinal + 1,
                    handled_slot.map(|slot| slot + 1).unwrap_or(0),
                    vm.pause_ticks,
                    self.blackthorn_current_prompt_message()
                );
                Ok(MoveOutcome::Used)
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::Survived => {
                let handled_slot = self.mark_blackthorn_current_target_handled();
                let vm = self
                    .run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::ConditionalThroneCleanup);
                self.apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    &format!(
                        "Survived Blackthorn's challenge; party slot {} is handled; cutscene screen cleared {}.",
                        handled_slot.map(|slot| slot + 1).unwrap_or(0),
                        vm.screen_cleared
                    ),
                )
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::Wrong { ordinal, expected } => {
                let victim = self.mark_blackthorn_failure_victim_handled();
                let vm = self
                    .run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::FailedChallengeReaction);
                self.apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    &format!(
                        "Failed Blackthorn's prompt {}; expected {expected}; party slot {} is punished; cutscene output {} byte(s).",
                        ordinal + 1,
                        victim.map(|slot| slot + 1).unwrap_or(0),
                        vm.output_bytes.len()
                    ),
                )
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::PromptPresented {
                prompt,
                ..
            } => {
                self.active_blackthorn = Some(challenge);
                self.message = format!("Blackthorn asks for {prompt}.");
                Ok(MoveOutcome::PromptDeclined)
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::AlreadyPunished => self
                .apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    "Blackthorn's punishment has already resolved.",
                ),
            crate::blackthorn_session::BlackthornChallengeOutcome::AlreadySurvived => self
                .apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    "Blackthorn's challenge has already resolved.",
                ),
            crate::blackthorn_session::BlackthornChallengeOutcome::AlreadyAborted => self
                .apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    "Blackthorn's challenge was aborted.",
                ),
        }
    }

    pub fn blackthorn_current_prompt_message(&self) -> String {
        let Some(challenge) = self.active_blackthorn.as_ref() else {
            return "Blackthorn audience is not active.".to_string();
        };
        if let Some((_, prompt)) = challenge.current_prompt() {
            let target = self
                .next_blackthorn_challenge_target_slot()
                .map(|slot| slot + 1)
                .unwrap_or(0);
            format!("Blackthorn asks party slot {target} for {prompt}.")
        } else {
            "Blackthorn waits.".to_string()
        }
    }

    pub fn next_blackthorn_challenge_target_slot(&self) -> Option<usize> {
        self.party.iter().enumerate().find_map(|(index, member)| {
            let slot = member.slot;
            (member.living() && !self.blackthorn_story.is_party_slot_jailed(slot)).then_some(index)
        })
    }

    pub fn mark_blackthorn_current_target_handled(&mut self) -> Option<usize> {
        let index = self.next_blackthorn_challenge_target_slot()?;
        let slot = self.party[index].slot;
        self.blackthorn_story.mark_party_slot_jailed(slot);
        Some(index)
    }

    pub fn mark_blackthorn_failure_victim_handled(&mut self) -> Option<usize> {
        let index = if self.party.len() > BLACKTHORN_FAILURE_VICTIM_SLOT {
            BLACKTHORN_FAILURE_VICTIM_SLOT
        } else {
            self.next_blackthorn_challenge_target_slot()?
        };
        let slot = self.party[index].slot;
        self.blackthorn_story.mark_party_slot_jailed(slot);
        Some(index)
    }

    pub fn apply_blackthorn_captive_cell_handoff(
        &mut self,
        game_dir: &Path,
        prefix: &str,
    ) -> io::Result<MoveOutcome> {
        let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE)?;
        let floor = 0i8;
        let (grid, beacon_sources) =
            load_town_runtime_floor_with_beacon_sources(game_dir, scene, floor, self.clock.hour)?;
        self.grid = grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town { scene, floor };
        self.player.x = BLACKTHORN_CAPTIVE_CELL_X as usize;
        self.player.y = BLACKTHORN_CAPTIVE_CELL_Y as usize;
        self.player.transport = TransportState::Foot;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.blackthorn_audience_map = None;
        if self.blackthorn_story.captive_cell_counter == 0 {
            self.blackthorn_story.captive_cell_counter = 1;
        }
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.message = format!(
            "{prefix} Returned to Blackthorn's captive cell in {} at ({}, {}).",
            scene.key(),
            self.player.x,
            self.player.y
        );
        Ok(MoveOutcome::Transition(AreaTransition::EnteredLocation(
            scene,
        )))
    }

    pub fn apply_blackthorn_rescue_refuge(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let verdict = blackthorn_rescue_verdict_record(self.moral_standing);
        let verdict_message = self.blackthorn_rescue_verdict_message(game_dir, verdict as usize)?;
        let previous_standing = self.moral_standing;

        // `blackthorn.md §7` (public spec b34ae69): after the
        // unending-darkness line, the first blocking call publishes a hidden
        // viewport containing colour zero only. It precedes scratch-state
        // clearing, tableau construction, and every durable handoff write.
        self.run_map_viewport_dissolve(MapViewportDissolveSource::BlackthornRescueBlack);

        for member in &mut self.party {
            member.status = b'G';
            member.hp = member.max_hp.max(1);
        }

        // The verdict and party restoration precede the second call. Its
        // hidden viewport is black except for the on-foot party tile at the
        // centre cell; the castle view is not this dissolve's source.
        self.run_map_viewport_dissolve(MapViewportDissolveSource::BlackthornRescuePartyOnBlack {
            cell: BLACKTHORN_RESCUE_PARTY_CELL,
        });

        // All handoff state follows completion of that second blocking call.
        self.moral_standing = blackthorn_rescue_post_print_standing(self.moral_standing);
        self.clear_active_effect_slot();
        self.torch_counter = 0;
        self.light_spell_counter = 0;
        self.blackthorn_story.clear_jailed_party_slots();
        let scene = Scene::new(BLACKTHORN_RESCUE_HANDOFF_SCENE)?;
        let floor = 0i8;
        let (grid, beacon_sources) =
            load_town_runtime_floor_with_beacon_sources(game_dir, scene, floor, self.clock.hour)?;
        self.grid = grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town { scene, floor };
        self.player.x = BLACKTHORN_RESCUE_HANDOFF_X as usize;
        self.player.y = BLACKTHORN_RESCUE_HANDOFF_Y as usize;
        self.force_foot_transport();
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.blackthorn_audience_map = None;
        if self.blackthorn_story.captive_cell_counter == 0 {
            self.blackthorn_story.captive_cell_counter = 1;
        }
        self.blackthorn_story.capture_context = BLACKTHORN_CAPTURE_CONTEXT_NONE;
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.message = format!(
            "Blackthorn rescue/refuge: {verdict_message}; standing {previous_standing}->{}, restored party, handed off to {} at ({}, {}).",
            self.moral_standing,
            scene.key(),
            self.player.x,
            self.player.y
        );
        Ok(MoveOutcome::Transition(AreaTransition::EnteredLocation(
            scene,
        )))
    }

    pub fn blackthorn_rescue_verdict_message(
        &self,
        game_dir: &Path,
        record_index: usize,
    ) -> io::Result<String> {
        let Some(records) = load_karma_records(game_dir)? else {
            return Ok(format!("verdict record {record_index} unavailable"));
        };
        Ok(records
            .get(record_index)
            .map(|record| format!("verdict record {record_index}: {record}"))
            .unwrap_or_else(|| format!("verdict record {record_index} unavailable")))
    }

    pub fn apply_dungeon_post_turn_effects_after_turn(
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
            let field_report = self.apply_dungeon_field_effect_at(level, x, y, tile, field);
            let field_message = format!("Triggered {}; {field_report}.", field.label());
            self.message = if self.message.is_empty() {
                field_message
            } else {
                format!("{} {field_message}", self.message)
            };
            return Ok(None);
        }
        if let Some(outcome) = self.apply_dungeon_active_monster_step(scene, level)? {
            return Ok(Some(outcome));
        }
        Ok(None)
    }

    pub fn apply_dungeon_active_monster_step(
        &mut self,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some((slot, object)) = self.dungeon_active_monster() else {
            return Ok(None);
        };
        let Some(step) = self.dungeon_active_monster_step(slot, object, level) else {
            return Ok(None);
        };
        if step.contact {
            let direction = self
                .dungeon_direction_from_player_to(object.x, object.y)
                .unwrap_or(self.player.facing);
            self.player.facing = direction;
            self.free_active_object_slot(slot);
            self.mark_visibility_dirty();
            if combat_class_stats(object.aux1).is_some() {
                let note = self.enter_dungeon_active_monster_combat(level, object)?;
                let contact_message = format!(
                    "Dungeon monster tile {} approaches from the {} on {} level {level}; {note}.",
                    object.tile,
                    direction.name(),
                    scene.key()
                );
                self.message = if self.message.is_empty() {
                    contact_message
                } else {
                    format!("{} {contact_message}", self.message)
                };
                return Ok(Some(MoveOutcome::Used));
            }
            let contact_message = format!(
                "Dungeon object tile {} approaches from the {} on {} level {level}; no published combat class.",
                object.tile,
                direction.name(),
                scene.key()
            );
            self.message = if self.message.is_empty() {
                contact_message
            } else {
                format!("{} {contact_message}", self.message)
            };
            return Ok(Some(MoveOutcome::Used));
        }

        self.active_objects[slot].x = step.x;
        self.active_objects[slot].y = step.y;
        self.active_objects[slot].phase =
            active_object_phase_from_direction(step.direction, object.phase & 0x0f);
        self.mark_visibility_dirty();
        let move_message = format!(
            "Dungeon monster tile {} moved {} to ({}, {}) on {} level {level}.",
            object.tile,
            step.direction.name(),
            step.x,
            step.y,
            scene.key()
        );
        self.message = if self.message.is_empty() {
            move_message
        } else {
            format!("{} {move_message}", self.message)
        };
        Ok(None)
    }

    pub fn dungeon_active_monster_step(
        &self,
        slot: usize,
        object: ActiveObject,
        level: u8,
    ) -> Option<DungeonActiveMonsterStep> {
        if object.phase & 0x0f == STEADY_PHASE {
            return None;
        }
        let current_distance =
            dungeon_wrapped_manhattan_distance(object.x, object.y, self.player.x, self.player.y);
        let directions = dungeon_monster_step_directions(self.dungeon_monster_step_seed(slot));
        for direction in directions {
            let (dx, dy) = direction.delta();
            let nx = (object.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
            let ny = (object.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
            if (nx, ny) == (self.player.x, self.player.y) {
                return Some(DungeonActiveMonsterStep {
                    direction,
                    x: nx,
                    y: ny,
                    contact: true,
                });
            }
            if self
                .active_objects
                .iter()
                .enumerate()
                .any(|(other_slot, other)| {
                    other_slot != slot && !other.is_player() && self.object_occupies(*other, nx, ny)
                })
            {
                continue;
            }
            let tile = self.dungeon_cell(level, nx, ny);
            if !dungeon_active_monster_can_step_on(tile) {
                continue;
            }
            let distance = dungeon_wrapped_manhattan_distance(nx, ny, self.player.x, self.player.y);
            if distance < current_distance {
                return Some(DungeonActiveMonsterStep {
                    direction,
                    x: nx,
                    y: ny,
                    contact: false,
                });
            }
        }
        None
    }

    pub fn dungeon_active_monster(&self) -> Option<(usize, ActiveObject)> {
        let Area::Dungeon { level, .. } = self.area else {
            return None;
        };
        let object = self
            .active_objects
            .get(DUNGEON_ACTIVE_MONSTER_SLOT)
            .copied()?;
        (dungeon_monster_record_active(object) && object.z == level as i8)
            .then_some((DUNGEON_ACTIVE_MONSTER_SLOT, object))
    }

    pub fn dungeon_monster_step_seed(&self, slot: usize) -> u8 {
        self.turn as u8 ^ self.player.x as u8 ^ ((self.player.y as u8) << 2) ^ ((slot as u8) << 4)
    }

    pub fn dungeon_direction_from_player_to(&self, x: usize, y: usize) -> Option<Direction> {
        let west = (self.player.x + DUNGEON_SIDE - x) % DUNGEON_SIDE;
        let east = (x + DUNGEON_SIDE - self.player.x) % DUNGEON_SIDE;
        let north = (self.player.y + DUNGEON_SIDE - y) % DUNGEON_SIDE;
        let south = (y + DUNGEON_SIDE - self.player.y) % DUNGEON_SIDE;
        if east == 1 && north == 0 {
            Some(Direction::East)
        } else if west == 1 && north == 0 {
            Some(Direction::West)
        } else if south == 1 && east == 0 {
            Some(Direction::South)
        } else if north == 1 && east == 0 {
            Some(Direction::North)
        } else {
            None
        }
    }

    pub fn hole_up_command(
        &mut self,
        game_dir: &Path,
        request: impl Into<InlineRestRequest>,
    ) -> io::Result<MoveOutcome> {
        self.clear_active_effect_slot();
        let request = request.into();
        match self.area {
            Area::Town { scene, floor } => {
                self.hole_up_town_command(game_dir, request.hours, scene, floor)
            }
            Area::World { .. } | Area::Dungeon { .. } => self.rest_with_watch(request, game_dir),
        }
    }

    pub fn hole_up_town_command(
        &mut self,
        game_dir: &Path,
        hours: Option<u8>,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let Some(hours) = hours else {
            return Ok(self.start_rest_prompt());
        };
        if !(1..=9).contains(&hours) {
            self.message = "Rest hours must be in 1..9.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let entries = load_town_rest_bed_entries(game_dir)?;
        if !self.town_rest_bed_still_accepts(entries.as_deref(), scene, floor) {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.mark_town_rest_sleepers();
        if !self.advance_town_rest_initial_schedule_burst(entries.as_deref(), scene, floor) {
            let woke = self.wake_town_rest_sleepers();
            self.message = format!(
                "Rest interrupted; thrown out of the inn bed; woke {woke} asleep member(s)."
            );
            return Ok(MoveOutcome::Blocked);
        }
        if !self.advance_town_rest_until_target_hour(hours, entries.as_deref(), scene, floor) {
            let woke = self.wake_town_rest_sleepers();
            self.message = format!(
                "Rest interrupted; thrown out of the inn bed; woke {woke} asleep member(s)."
            );
            return Ok(MoveOutcome::Blocked);
        }
        let mut recovered_hp = 0;
        let mut recovered_mana = 0;
        for _ in 0..hours {
            let (hp, mana) = self.apply_rest_recovery_tick();
            recovered_hp += hp;
            recovered_mana += mana;
        }
        let woke = self.wake_town_rest_sleepers();
        self.message = format!(
            "Rested {hours} hour{} at the inn bed; recovered {recovered_hp} HP and {recovered_mana} MP; woke {woke} asleep member(s).",
            if hours == 1 { "" } else { "s" }
        );
        Ok(MoveOutcome::Rested)
    }

    pub fn town_rest_target_hour(current_hour: u8, duration_digit: u8) -> u8 {
        let target = current_hour.saturating_add(duration_digit);
        if target > TOWN_REST_HOUR_WRAP_SUBTRAHEND {
            target - TOWN_REST_HOUR_WRAP_SUBTRAHEND
        } else {
            target
        }
    }

    pub fn advance_town_rest_until_target_hour(
        &mut self,
        duration_digit: u8,
        entries: Option<&[TownRestBedEntry]>,
        scene: Scene,
        floor: i8,
    ) -> bool {
        let target_hour = Self::town_rest_target_hour(self.clock.hour, duration_digit);
        let mut ticks: u16 = 0;
        while self.clock.hour != target_hour && ticks < TOWN_REST_TICK_BUDGET {
            self.advance_turn_with_minutes(TOWN_REST_MINUTES_PER_TICK);
            ticks += 1;
            if !self.town_rest_bed_still_accepts(entries, scene, floor) {
                return false;
            }
        }
        self.clock.hour == target_hour
    }

    pub fn advance_town_rest_initial_schedule_burst(
        &mut self,
        entries: Option<&[TownRestBedEntry]>,
        scene: Scene,
        floor: i8,
    ) -> bool {
        for _ in 0..TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS {
            // `karma.md §4.1`: these zero-minute schedule-only setup
            // ticks are not one of the ten-minute rest ticks that age the
            // conversation-payment cooldown.
            self.advance_turn_with_minutes_policy(0, true, true, true, false);
            if !self.town_rest_bed_still_accepts(entries, scene, floor) {
                return false;
            }
        }
        true
    }

    pub fn town_rest_bed_still_accepts(
        &self,
        entries: Option<&[TownRestBedEntry]>,
        scene: Scene,
        floor: i8,
    ) -> bool {
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let native_inn_bed =
            crate::shop_session::inn_for_scene(scene.byte).is_some() && is_town_rest_bed_tile(tile);
        native_inn_bed
            || entries.unwrap_or(&[]).iter().any(|entry| {
                town_rest_bed_matches(*entry, scene, floor, self.player.x, self.player.y, tile)
            })
    }
}

pub const fn shadowlord_stonegate_npc_slot(index: usize) -> Option<usize> {
    match index {
        SHADOWLORD_FALSEHOOD_INDEX => Some(SHADOWLORD_FALSEHOOD_STONEGATE_NPC_SLOT),
        SHADOWLORD_HATRED_INDEX => Some(SHADOWLORD_HATRED_STONEGATE_NPC_SLOT),
        SHADOWLORD_COWARDICE_INDEX => Some(SHADOWLORD_COWARDICE_STONEGATE_NPC_SLOT),
        _ => None,
    }
}

fn combat_klimb_tile_accepts_vertical(tile: u8, intent: ClimbIntent) -> bool {
    match intent {
        ClimbIntent::Up | ClimbIntent::Down => matches!(tile, 0x50..=0x57),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonActiveMonsterStep {
    pub direction: Direction,
    pub x: usize,
    pub y: usize,
    pub contact: bool,
}

pub fn dungeon_active_monster_can_step_on(tile: u8) -> bool {
    !matches!(tile >> 4, 0x06 | 0x0b..=0x0f)
        && !matches!(dungeon_field_effect(tile), Some(DungeonFieldEffect::Sleep))
}

pub fn dungeon_wrapped_manhattan_distance(ax: usize, ay: usize, bx: usize, by: usize) -> usize {
    dungeon_wrapped_axis_distance(ax, bx) + dungeon_wrapped_axis_distance(ay, by)
}

pub fn dungeon_wrapped_axis_distance(a: usize, b: usize) -> usize {
    let forward = (a + DUNGEON_SIDE - b) % DUNGEON_SIDE;
    let backward = (b + DUNGEON_SIDE - a) % DUNGEON_SIDE;
    forward.min(backward)
}

pub fn dungeon_monster_step_directions(seed: u8) -> [Direction; 4] {
    const ORDERS: [[Direction; 4]; 4] = [
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ],
        [
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::North,
        ],
        [
            Direction::South,
            Direction::West,
            Direction::North,
            Direction::East,
        ],
        [
            Direction::West,
            Direction::North,
            Direction::East,
            Direction::South,
        ],
    ];
    ORDERS[(seed as usize) & 0x03]
}

pub fn potion_label(index: usize) -> &'static str {
    const LABELS: [&str; POTION_COUNT] = [
        "blue", "yellow", "red", "green", "orange", "purple", "black", "white",
    ];
    LABELS.get(index).copied().unwrap_or("unknown")
}

pub fn scroll_label(index: usize) -> &'static str {
    SCROLL_SPELL_LABELS.get(index).copied().unwrap_or("unknown")
}
