use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    /// `town-mode.md §10`: the fireplace id of the town burning family.
    pub const TOWN_BURNING_FIREPLACE_TILE: u8 = 0xbc;

    /// `town-mode.md §10`: the molten-lava id of the town burning family. It
    /// is the same tile the Stonegate script fills the whole live grid with,
    /// which is why that scene's survivors keep burning afterwards.
    pub const TOWN_BURNING_LAVA_TILE: u8 = 0x8f;

    /// `town-mode.md §10`: the stored line printed by the burning family.
    pub const TOWN_BURNING_MESSAGE: &'static str = "Burning!";

    /// `blackthorn.md §4`/`§8`: "Moral standing | Durable; debited five per
    /// correct interrogation answer". The subtraction is clamped and floored
    /// at zero.
    pub const BLACKTHORN_CORRECT_ANSWER_STANDING_DEBIT: u8 = 5;

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
        if tile == TOWN_OPEN_TOO_HEAVY_TILE {
            self.message = "Too heavy!".to_string();
            return MoveOutcome::Blocked;
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
            self.message = "Key broke!".to_string();
            // `audio.md §8.1`: failure only — print the break line, play
            // the 40-update action snap, then decrement the key count.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return MoveOutcome::LockTried;
        }
        if jimmy_restraint_tile(tile) {
            if !self.jimmy_lock_pick_succeeds(actor_slot) {
                self.message = "Key broke!".to_string();
                // `audio.md §8.1`: failure only — print the break line,
                // play the 40-update action snap, then decrement the key
                // count.
                self.emit_sound_effect(SoundEffect::ActionSnap);
                self.keys = self.keys.saturating_sub(1);
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
            self.message = "Key broke!".to_string();
            // `audio.md §8.1`: failure only — print the break line, play
            // the 40-update action snap, then decrement the key count.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
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
            self.message.clear();
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
            self.message.clear();
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
            self.message.clear();
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
            self.message.clear();
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

    /// `dungeon-mode.md §8` chest Search and §8.1 "Search outcomes": the
    /// detection roll's tier selects exactly one of the four published
    /// trap-tier lines, "**none of which carries a terminal period**". They
    /// are the one outcome line that follows the unconditional `You find:`
    /// preamble, so this arm prints no narration of its own.
    pub fn search_dungeon_chest(
        &mut self,
        _scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> MoveOutcome {
        let detail = self.dungeon_chest_trap_detail(level, x, y, tile);
        self.advance_turn();
        self.message = dungeon_chest_search_trap_line(detail).to_string();
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
        self.sail_cached_direction = None;
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
                    self.sail_cached_direction = None;
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
                    self.sail_cached_direction = None;
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
        // `vehicles.md` "Ship Sails": furling makes the ship "manually
        // handled" and "wind-driven drift should not advance the ship while
        // furled", so the toggle drops the cache the auto-advance route
        // reads. Hoisting starts with none, since the cache is written by a
        // movement command.
        self.sail_cached_direction = None;
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
                    // `audio.md §8.4`: the recognized-Word effect occurs before
                    // the location-specific success test, so a known Word
                    // spoken at the wrong place is still audible and visible.
                    self.emit_major_flash();
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
            // `audio.md §8.4`: a successful ruined-shrine restoration invokes
            // the shared full-viewport flash again at its own success
            // boundary — a second invocation after the recognized-Word flash.
            self.emit_major_flash();
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

    /// `time.md §7`: "the daily walker skips any slot whose high bit is set".
    /// The skip test is the high bit, not the living `1..8` range, because a
    /// slot holding `0` means "not yet placed" — neither in a town nor
    /// vanquished — and "the reroll walker rewrites it on the first day
    /// rollover". Only a vanquished `0xFF` slot is sticky.
    pub fn shadowlord_slot_is_rerollable(value: u8) -> bool {
        value & SAVE_QUEST_TILE_FLAG_HIGH_BIT == 0
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

    /// `time.md §7`: "For each slot whose high bit is clear, the midnight
    /// pass draws a candidate id uniformly from `1..8` inclusive and rejects
    /// it when either of these holds, then draws again: the candidate equals
    /// the party's current scene byte, or the candidate equals the value
    /// currently stored in **any** of the three slots, including the slot
    /// being rerolled and any slot already rewritten earlier in the same
    /// pass."
    ///
    /// The rejection set is therefore read live from `shadowlord_hideouts`,
    /// never from a pre-pass snapshot: that is what makes "a living
    /// Shadowlord never stays in the same town two days running, and no two
    /// living Shadowlords share a town" true. Vanquished `0xFF` slots never
    /// collide with a `1..8` candidate, so they do not constrain the draw,
    /// and with three slots plus one party scene at most four of the eight
    /// ids are ever excluded, so the redraw loop always terminates.
    pub fn reroll_shadowlord_hideouts_excluding(&mut self, current: Option<u8>) -> usize {
        let mut rerolled = 0usize;

        for slot in 0..SHADOWLORD_COUNT {
            if !Self::shadowlord_slot_is_rerollable(self.shadowlord_hideouts[slot]) {
                continue;
            }

            loop {
                let candidate =
                    self.random_range_u8(SHADOWLORD_HIDEOUT_MIN, SHADOWLORD_HIDEOUT_MAX);
                if Some(candidate) == current {
                    continue;
                }
                if self.shadowlord_hideouts.contains(&candidate) {
                    continue;
                }
                self.shadowlord_hideouts[slot] = candidate;
                break;
            }
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
                        let _setup_report = self.enter_terrain_combat_from_world_object(
                            game_dir,
                            plane,
                            object_slot,
                            object,
                        )?;
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
                    // `town-mode.md §14`: "The town overlay has a live
                    // NPC-conflict chain, entered both from A-Attack and
                    // from post-action cleanup, that hands the target
                    // NPC's linked active-object slot to the same
                    // terrain-combat entry the overworld uses, so a town
                    // fight is an ordinary arena fight: ordinary town
                    // ground resolves to the cobble arena, and the
                    // scene-keyed town-style override forces the monster
                    // count to one unless the target's class is Guard
                    // (whose stat row carries the sentinel count eight)."
                    // The withdrawn reading kept A-Attack inside town
                    // mode and never called the combat framer.
                    if town_npc_attack_enters_conflict(type_byte)
                        && !matches!(
                            town_npc_attack_resolution(type_byte),
                            TownNpcAttackResolution::Refused
                        )
                    {
                        // `combat.md §5`: the encounter's base combat
                        // class "is derived from the creature's own
                        // sprite byte". A town actor's renderer record
                        // carries the drawn person tile rather than its
                        // roster sprite class, so the conflict chain
                        // hands the entry a trigger stamped with the
                        // roster type byte - that is what makes a guard
                        // resolve to class 12 and skip the town-style
                        // single-attacker override.
                        // `combat.md §5`: the placed lead monster "keeps
                        // the triggering object's own tile byte", so the
                        // trigger carries the roster sprite tag in both
                        // sprite bytes rather than the town renderer's
                        // generic person tile.
                        let trigger = ActiveObject {
                            type_byte,
                            tile: type_byte,
                            ..object
                        };
                        if let Some(game_dir) = game_dir {
                            if game_dir.join(BRIT_CBT_FILE).exists()
                                && terrain_combat_base_class(trigger).is_some()
                            {
                                // §14 keeps the alarm sweep on the
                                // A-Attack routing; the death flow's
                                // slot clear, floor reload and
                                // Shadowlord re-install move to the
                                // arena exit.
                                let (pursued, fled) =
                                    self.town_alarm_sweep(scene, floor, Some(npc_slot));
                                self.pending_town_conflict = Some(PendingTownConflict {
                                    scene_byte: scene.byte,
                                    floor,
                                    npc_slot,
                                    type_byte,
                                    awaiting_floor_reload: false,
                                });
                                let hostile_terrain = self.grid[y * TOWN_GRID_SIDE + x];
                                let _setup_report = self
                                    .enter_terrain_combat_from_object_in_scene_with_terrain(
                                        game_dir,
                                        WorldPlane::Britannia,
                                        object_slot,
                                        trigger,
                                        scene.byte,
                                        hostile_terrain,
                                    )?;
                                let _ = (pursued, fled);
                                return Ok(MoveOutcome::Used);
                            }
                        }
                    }
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

    /// `town-mode.md §14`: the first half of the NPC-conflict chain's
    /// exit - "On exit the town chain clears the NPC slot". The
    /// removal-mask policy of §4 still decides whether the cleared slot
    /// is recorded as permanently gone: "killing a townsperson or a
    /// named character records that slot as permanently removed in the
    /// per-scene removal mask (Section 4), while killing a guard or a
    /// creature records nothing and that slot is placed again on the
    /// next entry."
    pub fn clear_town_conflict_npc_slot(&mut self, scene: Scene, npc_slot: usize, type_byte: u8) {
        if let Some(npc_index) = self.npcs.iter().position(|npc| npc.slot == npc_slot) {
            if let Some(object_slot) = self.npcs[npc_index].active_object {
                self.free_active_object_slot(object_slot);
            }
            self.npcs.remove(npc_index);
        }
        if town_npc_removal_recorded(type_byte) {
            self.mark_removed_town_npc_once(scene, npc_slot);
        }
        self.mark_visibility_dirty();
    }

    /// `town-mode.md §14`: the rest of the NPC-conflict chain's exit -
    /// the chain "reloads the town map, and re-runs the Shadowlord
    /// install pass of Section 13 (which, in a hideout town whose
    /// Shadowlord is still standing in the active-object table, is
    /// rejected by the one-at-a-time check and does nothing)". Both
    /// halves need a game directory, so they are drained at the input
    /// boundary once the arena frame has been restored.
    pub fn drain_pending_town_conflict(&mut self, game_dir: &Path) -> io::Result<bool> {
        if self.combat_active {
            return Ok(false);
        }
        let Some(pending) = self.pending_town_conflict else {
            return Ok(false);
        };
        if !pending.awaiting_floor_reload {
            return Ok(false);
        }
        self.pending_town_conflict = None;
        let Area::Town { scene, floor } = self.area else {
            return Ok(false);
        };
        if scene.byte != pending.scene_byte {
            return Ok(false);
        }
        self.reload_town_floor(game_dir, scene, floor)?;
        // `encounters.md §7`: the chain "does not re-place the player:
        // the player's position comes from the world-state globals
        // throughout."
        self.install_shadowlord_entry_encounter();
        Ok(true)
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
                self.door_tracker_closed = false;
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
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = PUSH_NOT_HERE_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }
        self.tick_door_tracker();
        let outcome = self.push_direction_after_cleanup_with_game_dir(direction, game_dir)?;
        self.prepend_push_direction_result(direction);
        Ok(outcome)
    }

    pub(crate) fn push_direction_after_cleanup_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => {
                self.push_town_direction_after_cleanup(game_dir, scene, floor, direction)
            }
            Area::Dungeon { .. } => unreachable!("dungeon Push bypasses direction handling"),
            Area::World { .. } => Ok(self.push_world_direction(direction)),
        }
    }

    pub(crate) fn prepend_push_direction_result(&mut self, direction: Direction) {
        let result = std::mem::take(&mut self.message);
        self.message = if result.is_empty() {
            direction.name().to_string()
        } else {
            format!("{}\n{result}", direction.name())
        };
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

        if self.object_slot_at_current_floor(tx, ty).is_some() {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }

        let Some(family) = pushable_tile_family(target_tile) else {
            // `commands.md §3`: P-Push does not forward the handler's
            // refusal status. Outside dungeons the resident dispatcher keeps
            // its default acted result, so even a source miss consumes the
            // ordinary world action.
            self.advance_turn();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        };

        self.push_world_static_family(direction, tx, ty, px, py, family)
    }

    pub fn push_combat_actor_direction(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        self.tick_door_tracker();
        let outcome = self.push_combat_actor_direction_after_cleanup(actor_slot, direction);
        self.prepend_push_direction_result(direction);
        outcome
    }

    pub(crate) fn push_combat_actor_direction_after_cleanup(
        &mut self,
        actor_slot: usize,
        direction: Direction,
    ) -> MoveOutcome {
        if !self.combat_active || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if !direction.is_cardinal() {
            self.message = "Push requires a cardinal facing direction.".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if !combat_actor_is_active_not_dead(actor) {
            self.message.clear();
            return MoveOutcome::Blocked;
        }

        let (dx, dy) = direction.delta();
        let sx = actor.x as isize + dx;
        let sy = actor.y as isize + dy;
        let dx2 = sx + dx;
        let dy2 = sy + dy;
        if apply_combat_ambush_reveal_records(
            &mut self.combat_ambush_reveals,
            &mut self.combat_terrain,
            sx as u8,
            sy as u8,
        )
        .is_some()
        {
            // `commands.md §8.2`: a pre-placed ambush/camp reveal marker
            // consumes the action before the ordinary source predicate and
            // has no result continuation after the direction echo.
            self.mark_visibility_dirty();
            self.message.clear();
            return MoveOutcome::Pushed;
        }
        if !combat_arena_coordinate_in_bounds(sx as i16, sy as i16)
            || !combat_arena_coordinate_in_bounds(dx2 as i16, dy2 as i16)
        {
            // The finite clean arena has no exposed backing bytes. Its
            // default off-grid sample is zero/non-pushable; tests that model
            // another backing byte must provide an in-range fixture cell.
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }
        let sx = sx as usize;
        let sy = sy as usize;
        let dx2 = dx2 as usize;
        let dy2 = dy2 as usize;

        if self
            .combat_actor_slot_at(sx as u8, sy as u8, actor_slot)
            .is_some()
        {
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }

        if self
            .combat_loose_object_slot_at(sx, sy, actor.active_object_slot as usize)
            .is_some()
        {
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return MoveOutcome::Blocked;
        }

        let source_tile = self.combat_terrain[sy][sx];
        let Some(family) = pushable_tile_family(source_tile) else {
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
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
            self.message = PUSHED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        if self.combat_terrain[actor.y as usize][actor.x as usize] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            self.combat_terrain[actor.y as usize][actor.x as usize] =
                pushable_oriented_tile(source_tile, pull_direction);
            self.combat_terrain[sy][sx] = stamp;
            self.finish_combat_push(actor_slot, sx, sy);
            self.message = PULLED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        self.message = PUSH_WONT_BUDGE_SHORT.to_string();
        MoveOutcome::Blocked
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
            self.message = PUSHED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        if self.grid[player_idx] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            self.grid[player_idx] = pushable_oriented_tile(target_tile, pull_direction);
            self.grid[target_idx] = stamp;
            self.finish_world_push(tx, ty);
            self.message = PULLED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        self.advance_turn();
        self.message = PUSH_WONT_BUDGE_SHORT.to_string();
        MoveOutcome::Blocked
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
        self.tick_door_tracker();
        let outcome = self.push_town_direction_after_cleanup(game_dir, scene, floor, direction)?;
        self.prepend_push_direction_result(direction);
        Ok(outcome)
    }

    pub(crate) fn push_town_direction_after_cleanup(
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
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        let px = tx + dx;
        let py = ty + dy;
        // `commands.md §8.2`: an out-of-grid location tile read aliases
        // the southeast cell, while the true coordinate cannot match an
        // ordinary active-object record.
        let source_in_bounds = (0..32).contains(&tx) && (0..32).contains(&ty);
        let far_in_bounds = (0..32).contains(&px) && (0..32).contains(&py);
        let (tx, ty) = if source_in_bounds {
            (tx as usize, ty as usize)
        } else {
            (31, 31)
        };
        let (px, py) = if far_in_bounds {
            (px as usize, py as usize)
        } else {
            (31, 31)
        };
        let target_idx = ty * 32 + tx;
        let target_tile = self.grid[target_idx];
        let entries = load_town_pushable_entries(game_dir)?;
        let sidecar_pushable = entries.as_ref().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| town_pushable_matches(*entry, scene, floor, tx, ty, target_tile))
        });

        if source_in_bounds && self.object_slot_at_current_floor(tx, ty).is_some() {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let Some(family) = pushable_tile_family(target_tile) else {
            if sidecar_pushable {
                return Ok(self.push_town_sidecar_tile(scene, floor, direction, tx, ty, px, py));
            }
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_EMPHATIC.to_string();
            return Ok(MoveOutcome::Blocked);
        };

        Ok(self.push_town_static_family(scene, floor, direction, tx, ty, px, py, family))
    }

    fn push_town_sidecar_tile(
        &mut self,
        scene: Scene,
        floor: i8,
        _direction: Direction,
        tx: usize,
        ty: usize,
        px: usize,
        py: usize,
    ) -> MoveOutcome {
        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_SHORT.to_string();
            return MoveOutcome::Blocked;
        }
        let target_idx = ty * 32 + tx;
        let dest_idx = py * 32 + px;
        let target_tile = self.grid[target_idx];
        let dest_tile = self.grid[dest_idx];
        if !self.tile_walkable(dest_tile) {
            self.advance_turn_without_door_tick();
            self.message = PUSH_WONT_BUDGE_SHORT.to_string();
            return MoveOutcome::Blocked;
        }

        self.grid[target_idx] = dest_tile;
        self.grid[dest_idx] = target_tile;
        self.finish_town_push(scene, floor, tx, ty, px, py);
        self.message = PUSHED_SUCCESS.to_string();
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
            self.message = PUSHED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        if self.grid[player_idx] == stamp {
            let pull_direction = direction.opposite_cardinal().unwrap_or(direction);
            let old_player_x = self.player.x;
            let old_player_y = self.player.y;
            self.grid[player_idx] = pushable_oriented_tile(target_tile, pull_direction);
            self.grid[target_idx] = stamp;
            self.finish_town_push(scene, floor, tx, ty, old_player_x, old_player_y);
            self.message = PULLED_SUCCESS.to_string();
            return MoveOutcome::Pushed;
        }

        self.advance_turn_without_door_tick();
        self.message = PUSH_WONT_BUDGE_SHORT.to_string();
        MoveOutcome::Blocked
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
            self.door_tracker_closed = false;
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
        // `weather.md §5.1`, clear three: "The Pass command | Only while the
        // outdoor scene is current and the cache is non-zero; it prints the
        // stalled-sailing line first, then clears." The condition is the
        // cache, not the engine's own stall flag: a Pass taken on a released
        // pass still had a heading cached and still clears it.
        if matches!(self.area, Area::World { .. }) && self.sail_cached_direction.is_some() {
            self.sail_cached_direction = None;
            self.message = "Ship remains stalled by the wind.".to_string();
        } else {
            // `commands.md §8.1` row B: `Pass` completes its own echo and
            // "no result line follows"; `text-output.md §10.3` lists the
            // `Pass` echo as complete in itself. The observed original
            // prints the echo and nothing beneath it, so the slot is left
            // empty for whatever the turn epilogue produces.
            self.message = String::new();
        }
        if let Some(game_dir) = game_dir {
            if let Some(outcome) =
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?
            {
                return Ok(outcome);
            }
        } else {
            // Test-only/sidecar-free callers have no town post-action I/O
            // path to enter. They still finish the deferred town tail.
            self.apply_pending_town_status_provision_pass();
            self.apply_pending_town_object_epilogue();
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
        let mut nonterminal_outcome = None;
        // `overworld.md` Section 8, precedence list item 3: the falls chain
        // is reached from the post-action pass when a waterfall is underfoot,
        // and from the top of the input helper when one stands directly
        // south. Both arms run the same handler, so this one site covers
        // both trigger cells (`RETRACTIONS.md` R320). It runs ahead of the
        // sidecar-driven plane transitions because the chain owns its own
        // plane write.
        if let Some(transition) = self.apply_world_falls_chain(game_dir, plane)? {
            return Ok(Some(MoveOutcome::Transition(transition)));
        }
        if let Some(transition) = self.apply_world_underfoot_plane_transition(game_dir, plane)? {
            let transition_message = self.message.clone();
            // The command that consumed the turn may have printed nothing
            // (`Pass`, an accepted step), so the separator only belongs here
            // when there is something to separate - as the sibling branches
            // below already do.
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
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
                    nonterminal_outcome = Some(outcome);
                }
                if outcome.is_transition() || self.combat_active {
                    return Ok(Some(outcome));
                }
            }
        }
        // `overworld.md §6.2.5`: rough seas follows committed action work and
        // cleanup, but precedes the remaining encounter epilogue. A reaction
        // that already changed mode returned above; an ordinary reaction does
        // not suppress this terrain/transport check.
        let _ = self.apply_rough_seas_if_eligible();
        self.apply_fixed_narrative_gate_branch(plane);
        self.append_world_damage_tile_message(Some(game_dir), plane)?;
        self.append_world_status_tile_message(plane);
        if object_epilogue_runs {
            if let Some(slot) = self.apply_world_encounter_probe(game_dir, plane)? {
                self.message
                    .push_str(&format!(" Wandering encounter spawned in slot {slot}."));
            }
        }
        Ok(nonterminal_outcome)
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
        // `overworld.md` Section 8.1, whirlpool swallow, in print order.
        // Step 1: the banner, with its leading line feed - one blank row.
        // "There is **no advance warning line**: the swallow banner ... is the
        // first and only text."
        self.emit_message_line(OVERWORLD_WHIRLPOOL_BANNER);
        // Step 2: the slot is cleared and the party marker is replaced by the
        // whirlpool sprite.
        let whirlpool_tile = self.active_objects[whirlpool_slot].tile;
        self.free_active_object_slot(whirlpool_slot);
        self.party_marker_tile_override = Some(whirlpool_tile);
        self.sync_player_object();
        self.mark_visibility_dirty();
        // Step 3: "One world tick, so the player sees a whirlpool where the
        // party was."
        self.advance_visual_tick();
        // Step 4: the long descending sweep.
        // `audio.md §8.9`, second row: "The whirlpool object is cleared,
        // `WHIRLPOOL!` prints, the party sprite is **replaced by the whirlpool
        // sprite**, and the viewport repaints ... **then** the long descent -
        // then the sprite is restored, the shared impact payload runs, and the
        // party is teleported".
        //
        // So the sweep sits after the slot is freed and strictly before the
        // impact payload and the transition: "The state commit - the teleport -
        // happens strictly after it." The on-foot arm above returned already,
        // which is the `§9` silence boundary "whirlpool engagement while the
        // party is on foot, which plays no long descent".
        self.emit_sound_effect(SoundEffect::LongDescent);
        // Step 5: "The party marker is restored", before the impact payload,
        // which is why a frigate's hull roll can still sink the ship in the
        // instant before the teleport.
        self.party_marker_tile_override = None;
        self.sync_player_object();
        self.mark_visibility_dirty();
        // Step 6: the Section 6.2.4 impact payload, "which may add
        // `Ship sunk!` and then either `Abandon ship!` or `DROWNING!!!`".
        self.apply_outdoor_impact();
        // Step 7: "The plane write and the move to `(34, 18)` ... This runs
        // **unconditionally** after step 6, including after the drowning
        // arm." It prints nothing of its own: the banner above is the first
        // and only text on the path, so the coordinate narration this used to
        // emit is removed.
        let entry = WorldPlaneTransitionEntry {
            from_plane: plane,
            x: self.player.x,
            y: self.player.y,
            to_plane: WorldPlane::Underworld,
            to_x: WHIRLPOOL_UNDERWORLD_EMERGENCE_X,
            to_y: WHIRLPOOL_UNDERWORLD_EMERGENCE_Y,
            expected_tile: None,
            // Section 8.1 publishes the *order* - marker restored, payload,
            // then the plane write - but says nothing about the durable
            // marker after the teleport, so the engine's existing foot reset
            // stands. Only the falls chain is published as preserving it.
            preserves_transport: false,
        };
        self.apply_world_plane_transition(game_dir, entry)?;
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
        // `active-objects.md §9`: "Combat suspends the world by swapping
        // the active-object table to a backup region and overwriting the
        // live table with combat actors ... The calling mode loop sees
        // combat as a function call that returns with the table and
        // globals exactly as they were". The §7 per-turn epilogue -
        // underfoot effects and the NPC schedule processor - therefore
        // resumes only once the framer has restored the world table. This
        // arm became reachable with the §14 NPC-conflict chain, which
        // enters an arena while the area is still a town.
        if self.combat_active {
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
                // `town-mode.md §10`: "If an effect moves the party to a
                // different floor, the handler re-reads the tile under the
                // party's new position and applies that tile's effect too."
                let landed_tile = self.grid[self.player.y * 32 + self.player.x];
                if Self::is_town_burning_live_tile(landed_tile) {
                    self.apply_town_burning_underfoot_effect();
                }
                let Area::Town {
                    scene: landed_scene,
                    floor: landed_floor,
                } = self.area
                else {
                    unreachable!("town trapdoor transition must remain in town mode");
                };
                self.append_town_poison_gas_message(game_dir, landed_scene, landed_floor)?;
                self.apply_pending_town_status_provision_pass();
                self.apply_pending_town_object_epilogue();
                let transition_message = self.message.clone();
                self.message = if pre_effect_message.is_empty() {
                    transition_message
                } else {
                    format!("{pre_effect_message} {transition_message}")
                };
                return Ok(Some(outcome));
            }
        }
        // `town-mode.md §10` burning family. This arm runs after the
        // trapdoor arm and before the trailing party pass, and it is not
        // shadowed by the trapdoor: the trapdoor tile is `0x8C` while the
        // burning ids are `0xBC`/`0x8F`. It has no transport gate, so it
        // fires from a carpet too, and it re-fires every consumed turn the
        // party spends standing on the tile.
        if Self::is_town_burning_live_tile(tile) {
            self.apply_town_burning_underfoot_effect();
        }
        // `town-mode.md §17` "Underfoot-effect cadence is fixed": "The
        // underfoot handler is a per-turn post-action pass, not a step-commit
        // hook. Any earlier statement that the poison-gas effect 'fires from
        // the step path' is retracted: it fires once per turn-consuming action
        // while the party occupies the tile, including turns spent passing in
        // place, and it fires after that turn's clock advance."
        //
        // `town-mode.md §10`: "If an effect moves the party to a different
        // floor, the handler re-reads the tile under the party's new position
        // and applies that tile's effect too", so the gas arm re-reads the
        // live tile rather than reusing the `tile` sampled above the trapdoor
        // arm.
        self.append_town_poison_gas_message(game_dir, scene, floor)?;
        self.apply_pending_town_status_provision_pass();
        self.apply_pending_town_object_epilogue();
        self.apply_town_npc_contact_event(scene, floor)
    }

    /// Finish the shared pass deliberately deferred by an ordinary town
    /// turn's clock advance. `town-mode.md §10` makes this the last act of the
    /// underfoot handler, after waking, trapdoor/burning, and poison-gas work.
    pub(crate) fn apply_pending_town_status_provision_pass(&mut self) {
        if std::mem::take(&mut self.pending_town_status_provision_pass) {
            self.apply_hourly_status_provision_pass();
        }
    }

    /// Finish the scheduler/object half of the ordinary town epilogue after
    /// underfoot effects and the shared status/provision pass.
    ///
    /// `town-mode.md §7` step 4, as corrected by **R328**: the turn's
    /// movement work runs "in a fixed order: three effect gates, the **town
    /// object walker** that moves the location's loose horse-family objects,
    /// and finally the NPC schedule processor with the current hour byte".
    /// The three gates were applied by the clock routine, which is what left
    /// these two flags set; this is the walker half of that order.
    ///
    /// The explicit-T arrest discriminator is the fourth skip, and it is the
    /// only one of the four that, in `town-mode.md §7` step 4's words,
    /// "skips the schedule processor *only*, and it is tested after the
    /// object walker has already made its pass". `npc-schedules.md §5` draws
    /// the consequence: "That is why a result-two turn can still move a loose
    /// horse-family object while no scheduled NPC moves." Both walkers
    /// raise the visibility-dirty flag from inside themselves, so §7 step 5's
    /// "the test has to cover **both** walkers, not just the schedule
    /// processor" holds on a result-two turn too.
    ///
    /// *Corrected (R328).* This used to call the schedule processor first and
    /// the object pass afterwards, which left a repaint conditioned on the
    /// processor's report blind to an object the walker had moved.
    pub(crate) fn apply_pending_town_object_epilogue(&mut self) {
        let run_npc_schedule = std::mem::take(&mut self.pending_town_npc_schedule_pass);
        let run_active_objects = std::mem::take(&mut self.pending_town_active_object_pass);
        // `npc-schedules.md §5` gates "**both** town walkers", which the
        // animator is not one of (`npc-schedules.md §12`; `RETRACTIONS.md`
        // R316), so the animate half carries its own flag and is not skipped
        // by a turn the transport or Quickness parity gated out.
        let run_animator = std::mem::take(&mut self.pending_town_active_object_animate_pass);
        if run_animator {
            self.animate_active_objects();
        }
        if run_active_objects {
            self.advance_active_object_walkers();
        }
        if run_npc_schedule && self.pending_town_arrest.is_none() {
            self.advance_npc_schedules();
        }
    }

    /// `town-mode.md §10`: the live town tiles in the burning family — the
    /// fireplace `0xBC` and molten lava `0x8F` of
    /// `catalogs/tile-catalog.md §6`. The earlier "rune/lever family" label
    /// for this bullet is withdrawn; both ids are damage tiles.
    pub const fn is_town_burning_live_tile(tile: u8) -> bool {
        matches!(
            tile,
            Self::TOWN_BURNING_FIREPLACE_TILE | Self::TOWN_BURNING_LAVA_TILE
        )
    }

    /// `town-mode.md §10`: "Rebuild the view, print the stored line
    /// `Burning!`, then apply the same independently rolled `1..8` mass
    /// damage to every non-Dead slot."
    pub fn apply_town_burning_underfoot_effect(&mut self) {
        self.mark_visibility_dirty();
        let line = Self::TOWN_BURNING_MESSAGE;
        self.message = if self.message.is_empty() {
            line.to_string()
        } else {
            format!("{} {line}", self.message)
        };
        self.apply_town_burning_party_damage();
    }

    /// The burning family's mass damage is the same independently rolled
    /// `1..8` pass the trapdoor arm runs (`town-mode.md §10`).
    pub fn apply_town_burning_party_damage(&mut self) {
        let slots = self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
        for slot in 0..slots {
            if self.party[slot].status == CharacterStatus::Dead.save_byte() {
                continue;
            }
            let damage = self.random_range_u8(1, TRAP_BOMB_DAMAGE_MAX);
            self.apply_shared_party_damage(slot, damage);
        }
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

        // `town-mode.md §7.1` fixes the order of this script precisely, and
        // `audio.md §8.2` places the descent inside it. Step 1 is the direct
        // black viewport fill, carried by the playback record above. Step 2 is
        // the speaker sweep — every integer frequency from 1000 down through
        // 251 Hz, 750 tones. Only then does step 3 rewrite the live grid.
        // Emitting after the grid rewrite would put the sweep one published
        // step late.
        self.emit_sound_effect(SoundEffect::StonegateDescent);

        self.grid.fill(STONEGATE_TRAPDOOR_GRID_TILE);
        self.mark_visibility_dirty();

        self.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];

        for slot in 0..self.party.len() {
            self.party[slot].hp = 0;
            self.party[slot].status = CharacterStatus::Dead.save_byte();
            // `audio.md §8.2`: the descent stops, then one 75-update
            // 100..500 Hz rumble per party member, as that member is killed
            // and the stats panel is repainted.
            self.emit_sound_effect(SoundEffect::StonegateMemberDeath);
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
        self.apply_pending_town_status_provision_pass();

        // The normal coordinate-only record-zero tail follows the all-zero
        // script boundary. The zero-type animal pass cannot move it; the NPC
        // schedule then retains its ordinary opportunity to write records.
        self.active_objects[0].x = self.player.x;
        self.active_objects[0].y = self.player.y;
        self.active_objects[0].z = floor;
        self.apply_pending_town_object_epilogue();
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

    /// `town-mode.md §10` poison-gas terrain, driven at the `§17` cadence:
    /// "The underfoot handler is a per-turn post-action pass, not a
    /// step-commit hook ... it fires once per turn-consuming action while the
    /// party occupies the tile, including turns spent passing in place, and it
    /// fires after that turn's clock advance." The only caller is
    /// [`PlayState::apply_town_post_turn_effects_after_turn`]; the retracted
    /// step-commit call site in the town step path is gone.
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
        if self.message.is_empty() {
            self.message = format!("{report}.");
        } else {
            self.message.push_str(&format!(" {report}."));
        }
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
        // `moons.md §3` (issue #190): "The town arrest jail relocation
        // refreshes. The relocation arm - the one that moves the party to
        // the Yew jail, runs the clock forward to 08:00 in repeated
        // twenty-minute cleanup calls (`systems/town-mode.md` Section 14)
        // and sets the floor byte to the entry floor - ends by running the
        // town entry pass, so it reaches the floor loader and repaints
        // unconditionally; because it clears the floor byte first, that
        // repaint takes the **normal** arm rather than the erase arm."
        //
        // The entry pass runs at the *end*, so the refresh is here rather
        // than beside the floor load above: the twenty-minute burst can
        // cross a day rollover, and the pair the pass caches is the one for
        // the day the party wakes on.
        self.refresh_cached_moon_glyphs_at_scene_entry();
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

        // `moons.md §3` (issue #190, `RETRACTIONS.md` R375): "The
        // Blackthorn audience cutscene repaints once, not twice. The
        // audience routine contains two repaint calls. The first, on
        // entry, always paints, and always takes the erase arm: the party
        // level is forced below surface immediately before it and the
        // scene is still Blackthorn's castle."
        //
        // This is that first call. R375 is the reason there is no matching
        // call in the routine's tail on the ordinary cutscene path: "on the
        // ordinary cutscene path the scene byte has been forced out of the
        // renderer's range earlier in the routine and is not restored until
        // after that second call, so the renderer returns at its first gate
        // having painted nothing and cached nothing ... An implementation
        // that wires both of the routine's own calls paints an extra
        // erase-arm frame the original never paints." The real
        // post-cutscene refresh comes from the caller instead - the town
        // entry pass, whose floor loader repaints (`reload_town_floor`).
        //
        // The below-surface level force is presentation-only here: this
        // engine paints no strip during the cutscene, and which arm the
        // painter would take does not change the cache write, which
        // precedes both erase-arm tests (§3).
        self.refresh_cached_moon_glyphs_at_scene_entry();

        // `blackthorn.md §3` step 2: "Select which shrine the interrogation
        // will demand a mantra for: scan the eight shrine ruin flags in
        // shrine order and take the first whose flag is *exactly* clear —
        // never ruined and never restored. If every flag is non-zero the
        // whole audience is abandoned." The §3 withdrawal is explicit that
        // this eight-slot scan selects a *shrine*, not a party member, and
        // that there is no per-member Blackthorn jail flag.
        let Some(shrine_index) = self.blackthorn_selected_shrine() else {
            // `moons.md §3` (R375): the routine's *second* repaint "paints
            // only on the routine's early-exit path". This is one of the
            // two early exits, so it is wired here and nowhere on the
            // ordinary cutscene path.
            self.refresh_cached_moon_glyphs_at_scene_entry();
            let outcome = self.apply_blackthorn_captive_cell_handoff(
                game_dir,
                "Blackthorn audience found no un-ruined shrine to interrogate.",
            )?;
            return Ok(Some(outcome));
        };

        // `blackthorn.md §3`: the setup "counts the active party members
        // that are still eligible for the challenge". Eligibility is
        // liveness only - the §3 withdrawal is explicit that "**There is
        // no per-member jail flag.**"
        if self.blackthorn_eligible_party_member_count() == 0 {
            // `moons.md §3` (R375): the routine's second repaint, on the
            // other early-exit path. See the shrine-scan exit above.
            self.refresh_cached_moon_glyphs_at_scene_entry();
            let outcome = self.apply_blackthorn_captive_cell_handoff(
                game_dir,
                "Blackthorn audience found no eligible party member.",
            )?;
            return Ok(Some(outcome));
        }

        let mut challenge =
            crate::blackthorn_session::BlackthornChallenge::for_shrine(shrine_index as u8);
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
        let release = self.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::GuardReleaseRoute);
        self.active_blackthorn = Some(challenge);
        // `blackthorn.md §4`: "The loop asks about ONE shrine, up to four
        // times." The withdrawn reading made the loop's party-slot
        // argument semantic - "it names which companion is at risk" - so
        // the prompt names the shrine's virtue and nothing else.
        self.message = format!(
            "Blackthorn audience: {opening}. Opening cutscene advanced {} world ticks; the guard release advanced {}. Blackthorn demands the mantra of {prompt}.",
            approach.world_ticks, release.world_ticks,
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
            let actor_byte = if placement.actor == BlackthornCutsceneActor::SecondPartyMember {
                self.party
                    .get(BLACKTHORN_FAILURE_VICTIM_SLOT)
                    .map(|member| combat_party_actor_byte(member.class_byte))
                    .filter(|byte| *byte != 0)
                    .unwrap_or(crate::PLAYER_TILE)
            } else {
                placement.tile
            };
            self.active_objects[slot] = ActiveObject {
                type_byte: actor_byte,
                tile: actor_byte,
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
                    actor_byte: object.tile,
                },
            );
        }
        vm.visible_actors = vm.actors;
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
                    type_byte: state.actor_byte,
                    tile: state.actor_byte,
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
        // `audio.md §8.6` / cleak/u5-spec#177: every explicit stinger-pause
        // repetition and every movement step while per-step pauses are enabled
        // runs the live two-part sting. The VM owns the separate two-tick
        // cinematic pause; the speaker program owns only its two rumble halves
        // and calibrated silent gap.
        for _ in 0..vm.stinger_count {
            self.emit_sound_effect(SoundEffect::BlackthornMovementStinger);
        }
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

        // `blackthorn.md §4`: "The shrine index is fixed before the loop
        // starts and never changes inside it", so the consequence arms read
        // the session's index rather than re-scanning the ruin flags — a
        // re-scan would drift onto the next shrine the moment this one is
        // ruined.
        let shrine_index = usize::from(challenge.shrine_index);

        match challenge.submit(&answer) {
            crate::blackthorn_session::BlackthornChallengeOutcome::Correct { ordinal } => {
                // `blackthorn.md §4`: the correct-answer consequences are the
                // shrine ruin flag and the clamped five-point standing debit.
                self.apply_blackthorn_correct_answer_consequences(shrine_index);
                let standing = self.moral_standing;
                let fate = self.apply_blackthorn_correct_answer_companion_fate();
                let vm = self
                    .run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::PerQuestionIntermission);
                // A correct answer resolves the interrogation; the
                // withdrawn reading ended it only once the jail scan ran
                // dry, i.e. once every party slot had been flagged.
                self.run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::ConditionalThroneCleanup);
                self.apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    &format!(
                        "Answered Blackthorn's prompt {} correctly; shrine {} is ruined; standing {standing}; {fate}; cutscene advanced {} world ticks.",
                        ordinal + 1,
                        shrine_index + 1,
                        vm.world_ticks
                    ),
                )
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::Survived => {
                // `blackthorn.md §4`: a correct answer resolves the
                // interrogation, and it is the answer — not surviving the
                // ladder — that ruins the shrine and debits five points of
                // moral standing.
                self.apply_blackthorn_correct_answer_consequences(shrine_index);
                let standing = self.moral_standing;
                let fate = self.apply_blackthorn_correct_answer_companion_fate();
                let vm = self
                    .run_blackthorn_cutscene_beat(BlackthornCutsceneBeat::ConditionalThroneCleanup);
                self.apply_blackthorn_captive_cell_handoff(
                    game_dir,
                    &format!(
                        "Survived Blackthorn's challenge; shrine {} is ruined; standing {standing}; {fate}; cutscene advanced {} world ticks without clearing the screen.",
                        shrine_index + 1,
                        vm.world_ticks
                    ),
                )
            }
            crate::blackthorn_session::BlackthornChallengeOutcome::Wrong { ordinal, expected } => {
                // `blackthorn.md §4`: "**A wrong answer, when few companions
                // remain, ends the interrogation** with a mocking line about
                // lying and a threat of the dungeon. **A wrong answer
                // otherwise escalates.** The first wrong answer produces a
                // threat naming the companion at risk. Later wrong answers
                // stamp a tile into the cutscene map, and the fourth wrong
                // answer **kills** the named companion with the pendulum-blade
                // narration."
                //
                // §5 scopes the punishing side to "a branch that can punish a
                // companion" and names the victim as "the second living party
                // member (the first living companion behind the Avatar)", so
                // the ending branch is the one where no such companion exists.
                // §4 does not quantify "few" beyond that, so nothing narrower
                // is assumed here.
                let victim = self
                    .blackthorn_failure_victim_index()
                    .filter(|index| *index != 0);
                let Some(victim) = victim else {
                    return self.apply_blackthorn_captive_cell_handoff(
                        game_dir,
                        &format!(
                            "Failed Blackthorn's prompt {}; expected {expected}; too few companions remain, so the interrogation ends with a threat of the dungeon.",
                            ordinal + 1,
                        ),
                    );
                };
                match challenge.wrong_escalation() {
                    // First wrong answer: a threat only. No tile is stamped
                    // yet, and the loop re-asks with the next wording.
                    Some(crate::blackthorn_session::BlackthornWrongEscalation::Threat) => {
                        let prompt = challenge
                            .current_prompt()
                            .map(|(_, prompt)| prompt)
                            .unwrap_or("Virtue");
                        self.active_blackthorn = Some(challenge);
                        self.message = format!(
                            "Failed Blackthorn's prompt {}; expected {expected}; Blackthorn threatens party slot {}. He demands the mantra of {prompt} again.",
                            ordinal + 1,
                            victim + 1,
                        );
                        Ok(MoveOutcome::PromptDeclined)
                    }
                    // Second and third wrong answers: "Later wrong answers
                    // stamp a tile into the cutscene map". They do not run
                    // §5's execution beat or clear the victim actor; that
                    // happens only on the fourth answer. The two published
                    // punishment props are introduced in prompt order.
                    Some(crate::blackthorn_session::BlackthornWrongEscalation::TileStamp) => {
                        let (x, y, tile) = if ordinal == 1 {
                            (5, 7, BLACKTHORN_PENDULUM_TILE)
                        } else {
                            (5, 9, BLACKTHORN_HOURGLASS_TILE)
                        };
                        let mut vm = self.blackthorn_cutscene_vm_from_audience_state();
                        vm.run(&[
                            BlackthornCutsceneCommand::WriteTerrain { x, y, tile },
                            BlackthornCutsceneCommand::ExplicitRedraw,
                        ]);
                        self.apply_blackthorn_cutscene_vm_to_audience_state(&vm);
                        let prompt = challenge
                            .current_prompt()
                            .map(|(_, prompt)| prompt)
                            .unwrap_or("Virtue");
                        self.active_blackthorn = Some(challenge);
                        self.message = format!(
                            "Failed Blackthorn's prompt {}; expected {expected}; the punishment tableau changes over party slot {}; cutscene advanced {} world tick. He demands the mantra of {prompt} again.",
                            ordinal + 1,
                            victim + 1,
                            vm.world_ticks,
                        );
                        Ok(MoveOutcome::PromptDeclined)
                    }
                    // Fourth wrong answer: the §5 execution.
                    _ => {
                        let vm = self.run_blackthorn_cutscene_beat(
                            BlackthornCutsceneBeat::FailedChallengeReaction,
                        );
                        let report = self
                            .execute_blackthorn_companion(victim)
                            .unwrap_or_else(|| "no companion remains to punish".to_string());
                        self.apply_blackthorn_captive_cell_handoff(
                            game_dir,
                            &format!(
                                "Failed Blackthorn's prompt {}; expected {expected}; {report} by the pendulum blade; cutscene advanced {} world ticks.",
                                ordinal + 1,
                                vm.world_ticks
                            ),
                        )
                    }
                }
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
            // `blackthorn.md §4` withdrawal: "the loop's party-slot
            // argument is semantic: it names which companion is at risk"
            // is withdrawn. "The loop asks about ONE shrine, up to four
            // times", so the prompt names only that shrine's virtue.
            format!("Blackthorn asks for the mantra of {prompt}.")
        } else {
            "Blackthorn waits.".to_string()
        }
    }

    /// `blackthorn.md §3` step 2: "scan the eight shrine ruin flags in
    /// shrine order and take the first whose flag is *exactly* clear — never
    /// ruined and never restored."
    ///
    /// "Exactly clear" is a whole-byte test, not a high-bit test: the
    /// restoration path at the Word-of-Power arm only clears the ruin bit, so
    /// a restored shrine keeps a non-zero flag byte and is skipped here.
    pub fn blackthorn_selected_shrine(&self) -> Option<usize> {
        (0..SAVE_SHRINE_RUIN_FLAG_COUNT).find(|index| self.shrine_ruin_flags[*index] == 0)
    }

    /// `blackthorn.md §4`: "A correct answer ruins that shrine and costs five
    /// points of moral standing. The shrine's durable ruin flag is set, so
    /// the shrine thereafter renders and behaves as a ruined shrine until the
    /// player restores it by meditating there. The moral-standing debit is a
    /// clamped subtraction of five, floored at zero."
    ///
    /// The §4 withdrawal reverses the earlier "the challenge does not
    /// directly adjust numeric karma" reading: the interrogation *does* debit
    /// moral standing. The ruin bit written here is the same
    /// `SAVE_QUEST_TILE_FLAG_HIGH_BIT` the overworld quest-tile pass reads to
    /// render a ruined shrine and the shrine-restoration arm clears.
    pub fn apply_blackthorn_correct_answer_consequences(&mut self, shrine_index: usize) -> bool {
        let Some(flag) = self.shrine_ruin_flags.get_mut(shrine_index) else {
            return false;
        };
        *flag |= SAVE_QUEST_TILE_FLAG_HIGH_BIT;
        self.moral_standing = self
            .moral_standing
            .saturating_sub(Self::BLACKTHORN_CORRECT_ANSWER_STANDING_DEBIT);
        true
    }

    /// `blackthorn.md §3`: the audience setup "counts the active party
    /// members that are still eligible for the challenge". The withdrawn
    /// reading made this an eight-slot scan for "the first party slot
    /// ... whose per-member Blackthorn-jail flag is still clear";
    /// §3 answers that in full: "**There is no per-member jail flag.**"
    /// Eligibility is liveness.
    pub fn blackthorn_eligible_party_member_count(&self) -> usize {
        self.party.iter().filter(|member| member.living()).count()
    }

    /// `blackthorn.md §5`: "The victim is the second living party member
    /// (the first living companion behind the Avatar)". Nothing is
    /// flagged on the victim's record - the earlier
    /// `mark_blackthorn_failure_victim_handled` wrote a durable jail bit,
    /// which §8 withdraws.
    pub fn blackthorn_failure_victim_index(&self) -> Option<usize> {
        self.party
            .iter()
            .enumerate()
            .filter(|(_, member)| member.living())
            .map(|(index, _)| index)
            .nth(BLACKTHORN_FAILURE_VICTIM_SLOT)
            .or_else(|| self.party.iter().position(|member| member.living()))
    }

    /// `blackthorn.md §4`: "**A correct answer also decides a
    /// companion's fate.** If more than one companion is still alive,
    /// Blackthorn thanks the player for their honesty and **kills** one
    /// companion as 'a merciful death'. If only one remains, he spares
    /// the player instead."
    ///
    /// §5 gives the execution's shape: the routine "erases that
    /// companion's on-screen actor; lifts their roster record out of the
    /// party, compacts the remaining records up, and decrements the
    /// party count; parks the lifted record in the last roster slot with
    /// a **whereabouts value that matches no location**." §8 lists the
    /// result as "Durable and irreversible". The withdrawn reading
    /// substituted a per-member jail flag for the death, so the roster
    /// never shrank.
    pub fn apply_blackthorn_correct_answer_companion_fate(&mut self) -> String {
        let living_companions = self
            .party
            .iter()
            .skip(1)
            .filter(|member| member.living())
            .count();
        if living_companions <= 1 {
            return "only one companion remains, so Blackthorn spares the player".to_string();
        }
        let Some(index) = self.blackthorn_failure_victim_index() else {
            return "no companion remains to take the merciful death".to_string();
        };
        if index == 0 {
            return "only the Avatar remains, so Blackthorn spares the player".to_string();
        }
        // `blackthorn.md §5`: "The same execution runs on the *correct*-answer
        // branch whenever more than one companion is alive, under a different
        // message - Blackthorn thanking the player for their honesty and
        // granting the companion 'a merciful death'." Both branches therefore
        // share one execution routine.
        match self.execute_blackthorn_companion(index) {
            Some(report) => format!("{report} as a merciful death"),
            None => "no companion remains to take the merciful death".to_string(),
        }
    }

    /// `blackthorn.md §5`: "parks the lifted record in the last roster slot
    /// with a **whereabouts value that matches no location**. That
    /// whereabouts field is the same one the innkeeper uses when a companion
    /// is left at an inn; the value written here matches no inn and no scene,
    /// so no inn can ever retrieve them".
    ///
    /// The spec pins the *property* of that byte - it matches no inn and no
    /// scene - and leaves the exact value with the data, so the engine reuses
    /// the no-scene sentinel it already has: [`Scene::new`] rejects `0xFF`,
    /// and no inn's scene marker can equal it, so
    /// [`crate::inn_guest_indices_for_scene`] can never select the parked
    /// record.
    pub const BLACKTHORN_EXECUTED_WHEREABOUTS: u8 = 0xff;

    /// `blackthorn.md §5`: "**The punishment is an execution, and it is
    /// durable.** ... the routine:
    ///
    /// - erases that companion's on-screen actor;
    /// - lifts their roster record out of the party, compacts the remaining
    ///   records up, and decrements the party count;
    /// - parks the lifted record in the last roster slot with a
    ///   **whereabouts value that matches no location**."
    ///
    /// §8 records the result as "Durable and irreversible", and §5 adds that
    /// "the refuge/rescue sequence does not restore them either. **The
    /// companion is dead and gone, and the effect survives saving and
    /// reloading.**" This is the shared routine behind both the §5 failure
    /// reaction and the §4 correct-answer "merciful death".
    ///
    /// Returns a description of what was executed, or `None` when `index`
    /// names no companion behind the Avatar.
    pub fn execute_blackthorn_companion(&mut self, index: usize) -> Option<String> {
        if index == 0 || index >= self.party.len() {
            return None;
        }

        // "erases that companion's on-screen actor" - the audience cinematic
        // holds the victim in the `SecondPartyMember` actor slot of
        // `blackthorn.md §6`.
        let actor_slot = BlackthornCutsceneActor::SecondPartyMember.slot_index() as usize;
        if let Some(object) = self.active_objects.get_mut(actor_slot) {
            *object = ActiveObject::empty();
        }

        // "lifts their roster record out of the party, compacts the remaining
        // records up, and decrements the party count".
        let mut roster = self.synced_party_roster();
        let lifted = if index < roster.len() {
            Some(roster.remove(index))
        } else {
            None
        };
        let removed = self.party.remove(index);
        if index < self.party_names.len() {
            self.party_names.remove(index);
        }
        if index < self.party_experience.len() {
            self.party_experience.remove(index);
        }
        if index < self.party_stay_counters.len() {
            self.party_stay_counters.remove(index);
        }
        if index < self.party_strengths.len() {
            self.party_strengths.remove(index);
        }
        if index < self.party_intelligence.len() {
            self.party_intelligence.remove(index);
        }
        if index < self.party_equipment.len() {
            self.party_equipment.remove(index);
        }
        for (slot, member) in self.party.iter_mut().enumerate() {
            member.slot = slot as u8;
        }
        match self.active_player {
            Some(active) if active == index => self.active_player = None,
            Some(active) if active > index => self.active_player = Some(active - 1),
            _ => {}
        }

        // "parks the lifted record in the last roster slot with a whereabouts
        // value that matches no location". `formats/saved-gam.md` puts the
        // roster at sixteen fixed slots, so the park target is the sixteenth;
        // a shorter modelled roster parks at its own last slot.
        if let Some(record) = lifted {
            let park_slot = (SAVE_ROSTER_SLOT_COUNT - 1).min(roster.len());
            roster.insert(park_slot, record.clone());
            // The whereabouts byte lives in the shifted inn-guest view that
            // overlaps the roster (`formats/saved-gam.md`: the registry starts
            // one record-length minus one past the roster, so guest slot `k`'s
            // leading marker is the tail byte of roster record `k`). Writing
            // an unmatchable marker there is what makes the record
            // unretrievable by any inn while still surviving save/reload.
            if self.inn_registry.len() < INN_REGISTRY_CAP {
                let registry_slot =
                    crate::free_inn_registry_slot(&self.inn_registry).unwrap_or_default();
                self.inn_registry.push(InnGuestRecord {
                    registry_slot,
                    scene_marker: Self::BLACKTHORN_EXECUTED_WHEREABOUTS,
                    name: record.name,
                    member: record.member,
                    strength: record.strength,
                    intelligence: record.intelligence,
                    experience: record.experience,
                    equipment: record.equipment,
                    stay_counter: record.stay_counter,
                });
            }
        }
        self.party_roster = roster;
        self.mark_visibility_dirty();

        Some(format!(
            "roster slot {} is executed; party count {}",
            removed.slot + 1,
            self.party.len()
        ))
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
        // `blackthorn.md §8`: "Carried-key count zeroed by the audience
        // cleanup | Durable inventory effect of the capture". The byte an
        // earlier revision of that table called a Blackthorn conversation
        // signal "is the party's ordinary carried-key counter ... and the
        // audience's cleanup simply zeroes it, so the party leaves the
        // capture without its keys". This handoff is that cleanup, so the
        // debit fires on every exit path out of the interrogation.
        self.keys = 0;
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

        // `blackthorn.md §7` (public spec b34ae69): after the
        // unending-darkness line, the first blocking call publishes a hidden
        // viewport containing colour zero only. It precedes scratch-state
        // clearing, tableau construction, and every durable handoff write.
        self.run_map_viewport_dissolve(MapViewportDissolveSource::BlackthornRescueBlack);

        // `blackthorn.md §7.1`: the temporary refuge tableau is presented
        // between the two dissolves. Its three direct cell reveals all use
        // the shared 256-pixel LFSR order and 31 checkpoints. The thunder
        // beat invokes the shared major flash twice and therefore consumes
        // 3,712 gameplay-PRNG draws even when sound is muted.
        self.pending_blackthorn_rescue_playbacks
            .push(blackthorn_rescue_playback());
        // `audio.md §8.6.2`: after the refuge tableau first redraws the party
        // actor, run six independent envelope programs back-to-back. No visual
        // operation or intentional hold occurs between rows.
        self.emit_sound_effect(SoundEffect::BlackthornRescueEnvelopes);
        self.emit_major_flash();
        self.emit_major_flash();

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
        // `blackthorn.md §8.1` (R284): there is no captive-cell counter.
        // Rescue instead initializes the existing Food word only when the
        // complete little-endian value is zero; every nonzero value survives.
        if self.food == 0 {
            self.food = 63;
        }
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
        self.clear_town_floor_reload_door_state();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        let _ = self.restore_resident_shadowlord_after_floor_reload();
        self.sync_player_object();
        self.mark_visibility_dirty();
        // The message window belongs to the original verdict record. Scene,
        // coordinate, standing and restoration details are runtime state, not
        // player-facing prose from the game data.
        self.message = verdict_message;
        Ok(MoveOutcome::Transition(AreaTransition::EnteredLocation(
            scene,
        )))
    }

    pub fn take_pending_blackthorn_rescue_playbacks(&mut self) -> Vec<BlackthornRescuePlayback> {
        std::mem::take(&mut self.pending_blackthorn_rescue_playbacks)
    }

    pub fn blackthorn_rescue_verdict_message(
        &self,
        game_dir: &Path,
        record_index: usize,
    ) -> io::Result<String> {
        let Some(records) = load_karma_records(game_dir)? else {
            return Ok(String::new());
        };
        Ok(records.get(record_index).cloned().unwrap_or_default())
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
            // `dungeon-mode.md` Section 8.1: the two bomb lines, and nothing
            // else. The post-action pass is a producer that can run alongside
            // the command handler, so both lines go straight to the transcript
            // rather than being folded into the compatibility slot.
            self.emit_message_line(DUNGEON_BOMB_TRAP_LINE);
            self.emit_message_line(DUNGEON_KABOOM_LINE);
            return Ok(None);
        }
        if let Some(field) = dungeon_field_effect(tile) {
            // Section 8.1: the field line prints before the per-member rolls.
            if let Some(line) = dungeon_field_consequence_line(field) {
                self.emit_message_line(line);
            }
            let _ = self.apply_dungeon_field_effect_at(level, x, y, tile, field);
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
                    other_slot != slot
                        && other_slot != ACTIVE_OBJECT_PLAYER_SLOT
                        && self.object_occupies(*other, nx, ny)
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
        // `npc-schedules.md §12`: the H-Hole-Up hours command is the second
        // of the clear-and-re-place pass's three callers, and it runs "when
        // it finishes its rested hours". An interrupted rest returns above
        // and never reaches it.
        self.clear_and_replace_scheduled_npcs();
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
            // `moons.md §3` (issue #190), the second of the two `H` (Hole
            // up) arms: the census row for "`H` (Hole up), town-bed rest
            // loop" is "On the loop's ten-minute steps", so the repaint is
            // per step rather than once at the end. The strip's cell
            // positions move with the hour and the glyph pair with the day,
            // and a nine-hour rest crosses both.
            self.refresh_cached_moon_glyphs_at_scene_entry();
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
