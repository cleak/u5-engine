use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

impl PlayState {
    pub fn cast_rel_hur(&mut self, caster_index: usize) -> MoveOutcome {
        if !matches!(self.area, Area::World { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, REL_HUR_SPELL_INDEX, REL_HUR_COST)
        {
            return outcome;
        }

        let previous = self.wind;
        self.wind = self.wind.rel_hur_next();
        if self.wind == WindState::Calm {
            self.wind_save_byte = 0;
        }
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
        self.advance_turn();
        self.message = format!(
            "Wind change! {} -> {}.",
            previous.status_message(),
            self.wind.status_message()
        );
        MoveOutcome::Cast
    }

    pub fn cast_gate_travel(
        &mut self,
        caster_index: usize,
        slot_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        if matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = "Cannot Gate Travel shipboard.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, GATE_TRAVEL_SPELL_INDEX, GATE_TRAVEL_COST)
        {
            return Ok(outcome);
        }

        let phase = slot_index + 1;
        let slot = self.moonstone_slots[slot_index];
        self.advance_turn();
        match gate_travel_destination(slot) {
            GateTravelDestination::Ready {
                target,
                floor,
                start,
            } => {
                self.apply_gate_travel(game_dir, phase, target, floor, start)?;
                Ok(MoveOutcome::Transition(AreaTransition::GateTraveled {
                    target,
                }))
            }
            GateTravelDestination::Empty => {
                self.message = format!("Gate Travel phase {phase} is not set.");
                Ok(MoveOutcome::Blocked)
            }
            GateTravelDestination::Invalid(reason) => {
                self.message = format!("Gate Travel phase {phase} is invalid: {reason}.");
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn use_item_command(
        &mut self,
        request: Option<UseItemRequest>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        Ok(match request {
            Some(UseItemRequest::Torch) => self.ignite_torch(),
            Some(UseItemRequest::Gem) => self.view_gem(),
            Some(UseItemRequest::Key) => self.jimmy_facing_with_game_dir(game_dir)?,
            Some(UseItemRequest::Moonstone(slot_index)) => {
                self.use_moonstone_phase(Some(slot_index))
            }
            Some(UseItemRequest::Invalid) | None => {
                self.message = use_prompt_message();
                MoveOutcome::Blocked
            }
        })
    }

    pub fn use_moonstone_phase(&mut self, slot_index: Option<usize>) -> MoveOutcome {
        let Some(slot_index) = slot_index else {
            self.message = use_prompt_message();
            return MoveOutcome::Blocked;
        };
        let Some((scene, z, tile, label)) = self.current_moonstone_bury_context() else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        if !moonstone_bury_tile_allowed(tile) {
            self.message = format!("Cannot bury Moonstone on tile {tile}.");
            return MoveOutcome::Blocked;
        }

        let removed_pickup = self.clear_moonstone_pickups(slot_index);
        self.moonstone_slots[slot_index] = MoonstoneGateSlot {
            scene,
            x: self.player.x as u8,
            y: self.player.y as u8,
            z,
        };
        if removed_pickup {
            self.mark_visibility_dirty();
        }
        self.advance_turn();
        self.message = format!(
            "Buried Moonstone phase {} at {label} ({}, {}).",
            slot_index + 1,
            self.player.x,
            self.player.y
        );
        MoveOutcome::Used
    }

    pub fn current_moonstone_bury_context(&self) -> Option<(u8, u8, u8, String)> {
        match self.area {
            Area::World { plane } => Some((
                0,
                plane.save_floor() as u8,
                self.grid[world_cell_index(self.player.x, self.player.y)],
                plane.key().to_string(),
            )),
            Area::Town { scene, floor } => Some((
                scene.byte,
                floor as u8,
                self.grid[self.player.y * 32 + self.player.x],
                scene.key(),
            )),
            Area::Dungeon { .. } => None,
        }
    }

    pub fn cast_spell_resource_gate(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
    ) -> Option<MoveOutcome> {
        let Some(caster) = self.party.get(caster_index).copied() else {
            self.message = "Nobody can cast!".to_string();
            return Some(MoveOutcome::Blocked);
        };
        if !caster.conscious() {
            self.message = "Nobody can cast!".to_string();
            return Some(MoveOutcome::Blocked);
        }
        if self.spell_charges[spell_index] == 0 {
            self.message = "None mixed!".to_string();
            return Some(MoveOutcome::Blocked);
        }

        self.spell_charges[spell_index] = self.spell_charges[spell_index].saturating_sub(1);
        if self.party[caster_index].mana < mana_cost {
            self.message = "M.P. too low!".to_string();
            self.advance_turn();
            return Some(MoveOutcome::Blocked);
        }
        self.party[caster_index].mana -= mana_cost;
        if self.party[caster_index].level < mana_cost {
            self.message = "M.P. too low!".to_string();
            self.advance_turn();
            return Some(MoveOutcome::Blocked);
        }

        None
    }

    pub fn apply_gate_travel(
        &mut self,
        game_dir: &Path,
        phase: usize,
        target: PlayTarget,
        floor: i8,
        start: (usize, usize),
    ) -> io::Result<()> {
        self.cache_current_world_overlay();
        let previous_turn = self.turn;
        let mut options = PlayOptions {
            target,
            floor,
            start: Some(start),
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
            initial_britannia_overlay: self.world_overlays.get(WorldPlane::Britannia),
            debug_enter: self.debug_enter,
            saved_active_objects: None,
            save_template_source: self.save_template_source,
        };
        if let PlayTarget::World(plane) = target {
            options.saved_active_objects = self.world_overlays.get(plane);
        }

        let mut next = Self::load_scene(game_dir, options)?;
        next.turn = previous_turn;
        next.world_overlays = self.world_overlays.clone();
        if matches!(target, PlayTarget::World(_)) {
            next.cache_current_world_overlay();
        }
        next.force_foot_transport();
        next.sync_player_object();
        next.pending_moongate = None;
        next.message = format!(
            "Gate Travel phase {phase} -> {} at ({}, {}).",
            target.key(),
            start.0,
            start.1
        );
        *self = next;
        Ok(())
    }

    pub fn turn_dungeon(&mut self, clockwise: bool) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Turn is only meaningful in dungeon mode.".to_string();
            return MoveOutcome::Blocked;
        };
        let next = if clockwise {
            self.player.facing.turn_right_cardinal()
        } else {
            self.player.facing.turn_left_cardinal()
        };
        let Some(next) = next else {
            self.message = "Dungeon turn requires a cardinal facing direction.".to_string();
            return MoveOutcome::Blocked;
        };
        self.player.facing = next;
        self.advance_turn();
        self.message = format!(
            "Turned to face {} on {} ({}) level {level}.",
            next.name(),
            scene.key(),
            scene.name()
        );
        MoveOutcome::Moved
    }

    pub fn look_dungeon(&mut self) -> MoveOutcome {
        self.look_dungeon_with_drink(None, None)
    }

    pub fn look_dungeon_with_drink(
        &mut self,
        drink: Option<bool>,
        party_index: Option<usize>,
    ) -> MoveOutcome {
        let Area::Dungeon { level, .. } = self.area else {
            self.message = "Look is only implemented for dungeon mode in this slice.".to_string();
            return MoveOutcome::Blocked;
        };
        if !self.has_personal_light() {
            self.message = "You see: darkness.".to_string();
            return MoveOutcome::Observed;
        }
        let (dx, dy) = self.player.facing.delta();
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            self.message = "You see: the dungeon boundary.".to_string();
            return MoveOutcome::Observed;
        }

        let tile = self.dungeon_cell(level, x as usize, y as usize);
        let description = dungeon_look_description(tile);
        if (tile >> 4) == 0x5 {
            self.message = match drink {
                None => {
                    "You see: a fountain. Will you drink? (use lY/lN, or l2Y for party member 2)."
                        .to_string()
                }
                Some(false) => "You see: a fountain. Will you drink? No.".to_string(),
                Some(true) => {
                    let member_index = party_index.unwrap_or(0);
                    match self.apply_dungeon_fountain_effect(member_index, tile) {
                        Some(report) => {
                            format!("You see: a fountain. Will you drink? Yes. {report}")
                        }
                        None => format!(
                            "You see: a fountain. Will you drink? Yes, but party member {} is unavailable.",
                            member_index + 1
                        ),
                    }
                }
            };
            return if drink == Some(false) {
                MoveOutcome::PromptDeclined
            } else {
                MoveOutcome::Observed
            };
        }

        self.message = format!("You see: {description}.");
        MoveOutcome::Observed
    }

    pub fn apply_dungeon_fountain_effect(&mut self, member_index: usize, tile: u8) -> Option<String> {
        let subtype = tile & 0x0f;
        match subtype {
            0 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let before = member.status;
                member.status = b'G';
                Some(format!(
                    "Cured! slot {slot} status {} -> good",
                    party_status_name(before)
                ))
            }
            1 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let (before, after) = member.heal_to_max();
                Some(format!("Healed! slot {slot} HP {before}->{after}"))
            }
            2 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                member.status = b'P';
                Some(format!("Poisoned! slot {slot} is poisoned"))
            }
            _ => {
                let damage = self.dungeon_fountain_damage_roll(member_index, tile);
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let applied = member.apply_damage(damage);
                Some(format!(
                    "Bad taste. slot {slot} took {applied} HP ({} HP left)",
                    member.hp
                ))
            }
        }
    }

    pub fn view_gem(&mut self) -> MoveOutcome {
        if self.gems == 0 {
            self.message = "No gems!".to_string();
            return MoveOutcome::Blocked;
        }

        match self.area {
            Area::Dungeon { scene, level } => {
                self.gems = self.gems.saturating_sub(1);
                self.message = format!(
                    "Dungeon view of {} ({}) level {} ({} gem(s) remain; centered flood map, exact glyph/floodability edge cases out of scope):\n{}",
                    scene.key(),
                    scene.name(),
                    level,
                    self.gems,
                    self.dungeon_vision_map(level)
                );
                MoveOutcome::Observed
            }
            Area::Town { scene, floor } => {
                self.gems = self.gems.saturating_sub(1);
                self.message = format!(
                    "Gem view of {} floor {} ({} gem(s) remain; full-fill 11x11 map):\n{}",
                    scene.key(),
                    floor,
                    self.gems,
                    self.surface_gem_map(5)
                );
                MoveOutcome::Observed
            }
            Area::World { plane } => {
                self.gems = self.gems.saturating_sub(1);
                self.message = format!(
                    "Gem view of {} at ({}, {}) ({} gem(s) remain; full-fill 11x11 map):\n{}",
                    plane.key(),
                    self.player.x,
                    self.player.y,
                    self.gems,
                    self.surface_gem_map(5)
                );
                MoveOutcome::Observed
            }
        }
    }

    pub fn dungeon_vision_map(&self, level: u8) -> String {
        let radius = DUNGEON_GEM_VIEW_RADIUS;
        let side = (radius * 2 + 1) as usize;
        let center = radius as usize;
        let mut visible = vec![false; side * side];
        let mut queue = VecDeque::new();

        let center_index = center * side + center;
        visible[center_index] = true;
        queue.push_back((0isize, 0isize));

        while let Some((sx, sy)) = queue.pop_front() {
            let world_x = (self.player.x as isize + sx).rem_euclid(DUNGEON_SIDE as isize) as usize;
            let world_y = (self.player.y as isize + sy).rem_euclid(DUNGEON_SIDE as isize) as usize;
            if (sx != 0 || sy != 0)
                && !is_dungeon_walkable(self.dungeon_cell(level, world_x, world_y))
            {
                continue;
            }

            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let next_x = sx + dx;
                    let next_y = sy + dy;
                    if next_x < -radius || next_x > radius || next_y < -radius || next_y > radius {
                        continue;
                    }
                    let scratch_x = (next_x + radius) as usize;
                    let scratch_y = (next_y + radius) as usize;
                    let index = scratch_y * side + scratch_x;
                    if !visible[index] {
                        visible[index] = true;
                        queue.push_back((next_x, next_y));
                    }
                }
            }
        }

        let mut out = String::new();
        for scratch_y in 0..side {
            for scratch_x in 0..side {
                let index = scratch_y * side + scratch_x;
                if scratch_x == center && scratch_y == center {
                    out.push('@');
                } else if visible[index] {
                    let dx = scratch_x as isize - radius;
                    let dy = scratch_y as isize - radius;
                    let world_x =
                        (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
                    let world_y =
                        (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
                    out.push(render_dungeon_glyph(
                        self.dungeon_cell(level, world_x, world_y),
                    ));
                } else {
                    out.push(' ');
                }
            }
            out.push('\n');
        }
        out
    }

    pub fn dungeon_forward_view(&self, level: u8) -> String {
        let mut out = String::from("First-person dungeon view:\n");
        out.push_str(&format!(
            "0: here {}\n",
            self.describe_dungeon_offset(level, 0, 0)
        ));

        let Some(left) = self.player.facing.turn_left_cardinal() else {
            out.push_str("view requires a cardinal facing direction\n");
            return out;
        };
        let Some(right) = self.player.facing.turn_right_cardinal() else {
            out.push_str("view requires a cardinal facing direction\n");
            return out;
        };

        let (fdx, fdy) = self.player.facing.delta();
        let (ldx, ldy) = left.delta();
        let (rdx, rdy) = right.delta();
        let mut obscured = false;
        for band in 1..=DUNGEON_VIEW_DEPTH {
            if obscured {
                out.push_str(&format!("{band}: obscured by front wall\n"));
                continue;
            }

            let band = band as isize;
            let ahead_dx = fdx * band;
            let ahead_dy = fdy * band;
            out.push_str(&format!(
                "{band}: ahead {}; left {}; right {}\n",
                self.describe_dungeon_offset(level, ahead_dx, ahead_dy),
                self.describe_dungeon_offset(level, ahead_dx + ldx, ahead_dy + ldy),
                self.describe_dungeon_offset(level, ahead_dx + rdx, ahead_dy + rdy)
            ));
            obscured = self.dungeon_offset_blocks_view(level, ahead_dx, ahead_dy);
        }

        out
    }

    pub fn describe_dungeon_offset(&self, level: u8, dx: isize, dy: isize) -> String {
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            "the dungeon boundary".to_string()
        } else {
            dungeon_look_description(self.dungeon_cell(level, x as usize, y as usize)).to_string()
        }
    }

    pub fn dungeon_offset_blocks_view(&self, level: u8, dx: isize, dy: isize) -> bool {
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            return true;
        }

        !is_dungeon_walkable(self.dungeon_cell(level, x as usize, y as usize))
    }

    pub fn surface_gem_map(&self, radius: usize) -> String {
        let mut out = String::new();
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let r = radius as isize;
        match self.area {
            Area::Town { .. } => {
                for y in py - r..=py + r {
                    for x in px - r..=px + r {
                        if x == px && y == py {
                            out.push('@');
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
            Area::World { plane } => {
                for y in py - r..=py + r {
                    for x in px - r..=px + r {
                        let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                        if wx == self.player.x && wy == self.player.y {
                            out.push('@');
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
            Area::Dungeon { .. } => {}
        }
        out
    }

    #[cfg(test)]
    pub fn look_facing(&mut self) -> MoveOutcome {
        self.look_facing_with_table(None)
    }

    pub fn look_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let look_table = load_look_table(game_dir)?;
        self.look_facing_with_resources(Some(&look_table), Some(game_dir))
    }

    #[cfg(test)]
    pub fn look_facing_with_table(&mut self, look_table: Option<&LookTable>) -> MoveOutcome {
        self.look_facing_with_resources(look_table, None)
            .expect("look without a game dir cannot perform file-backed look context")
    }

    pub fn look_facing_with_resources(
        &mut self,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Dungeon { .. } => Ok(self.look_dungeon()),
            Area::Town { .. } => {
                let (dx, dy) = self.player.facing.delta();
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    self.message = "You see: the location boundary.".to_string();
                    return Ok(MoveOutcome::Observed);
                }
                let x = x as usize;
                let y = y as usize;
                if let Some(object) = self.blocking_object_at(x, y) {
                    self.message = if look_table.is_some() {
                        format!(
                            "You see: {} at ({x}, {y}).",
                            self.look_description(object.tile, look_table)
                        )
                    } else {
                        format!("You see: an actor tile {} at ({x}, {y}).", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                let tile = self.grid[y * 32 + x];
                self.message = format!(
                    "You see: {} at ({x}, {y}).",
                    self.look_description(tile, look_table)
                );
                Ok(MoveOutcome::Observed)
            }
            Area::World { plane } => {
                let (dx, dy) = self.player.facing.delta();
                let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                if let Some(object) = self.world_object_at(x, y) {
                    self.message = if look_table.is_some() {
                        format!(
                            "You see: {} at ({x}, {y}).",
                            self.look_description(object.tile, look_table)
                        )
                    } else {
                        format!("You see: an object tile {} at ({x}, {y}).", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                let tile = self.grid[world_cell_index(x, y)];
                let description =
                    self.look_description_for_world_tile(tile, look_table, game_dir, plane, x, y)?;
                self.message =
                    format!("You see: {} at ({x}, {y}) on {}.", description, plane.key());
                Ok(MoveOutcome::Observed)
            }
        }
    }

    pub fn look_description(&self, tile: u8, look_table: Option<&LookTable>) -> String {
        let base = look_table
            .and_then(|table| {
                table.description(tile as usize).filter(|description| {
                    !description.is_empty() && !table.is_sentinel(description)
                })
            })
            .map(str::to_string)
            .unwrap_or_else(|| tile_class(tile).to_string());

        if matches!(tile, 0xfa | 0xfb) {
            format!(
                "{base} ({}:{:02} {})",
                self.clock.display_hour(),
                self.clock.minute,
                self.clock.am_pm_suffix()
            )
        } else {
            base
        }
    }

    pub fn look_description_for_world_tile(
        &self,
        tile: u8,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<String> {
        let base = self.look_description(tile, look_table);
        if tile != 0xdf {
            return Ok(base);
        }
        let Some(name) = self.world_dungeon_name_at(game_dir, plane, x, y, tile)? else {
            return Ok(base);
        };
        Ok(format!("{base} ({name})"))
    }

    pub fn world_dungeon_name_at(
        &self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<&'static str>> {
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        Ok(load_world_location_entries(game_dir)?.and_then(|entries| {
            entries.into_iter().find_map(|entry| {
                if entry.plane == plane
                    && entry.x == x
                    && entry.y == y
                    && entry
                        .expected_tile
                        .map_or(true, |expected| expected == tile)
                {
                    match entry.target {
                        PlayTarget::Dungeon(scene) => Some(scene.name()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
        }))
    }

    pub fn talk_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.talk_facing_with_game_dir_and_keyword(game_dir, None)
    }

    pub fn talk_facing_with_game_dir_and_keyword(
        &mut self,
        game_dir: &Path,
        keyword: Option<&str>,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, .. } = self.area else {
            self.message = "Funny, no response!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let dialogue = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        Ok(self.talk_facing_with_dialogue_and_keyword(&dialogue, keyword))
    }

    pub fn facing_talk_target(&self) -> Option<(u8, usize, usize)> {
        let (dx, dy) = self.player.facing.delta();
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return None;
        }

        let x = x as usize;
        let y = y as usize;
        if let Some(npc) = self.npc_at_current_floor(x, y) {
            return Some((npc.dialog_id, x, y));
        }
        if !is_talk_through_tile(self.grid[y * 32 + x]) {
            return None;
        }

        let x = x as isize + dx;
        let y = y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return None;
        }
        let x = x as usize;
        let y = y as usize;
        self.npc_at_current_floor(x, y)
            .map(|npc| (npc.dialog_id, x, y))
    }

    #[cfg(test)]
    pub fn talk_facing_with_dialogue(&mut self, dialogue: &HashMap<u16, Vec<String>>) -> MoveOutcome {
        self.talk_facing_with_dialogue_and_keyword(dialogue, None)
    }

    pub fn talk_facing_with_dialogue_and_keyword(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::Town { .. }) {
            self.message = "Funny, no response!".to_string();
            return MoveOutcome::Blocked;
        }

        let Some((dialog_id, x, y)) = self.facing_talk_target() else {
            self.message = "Nobody's here!".to_string();
            return MoveOutcome::Blocked;
        };

        if (0x81..=0x88).contains(&dialog_id) {
            self.advance_turn();
            self.message = format!(
                "Talk reached shop trigger 0x{dialog_id:02X} at ({x}, {y}); shop flow is out of scope."
            );
            return MoveOutcome::Talked;
        }
        if dialog_id <= 1 {
            self.message = "They give thee a funny look.".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene.");
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope.");
            return MoveOutcome::Blocked;
        }

        let name = fields
            .first()
            .filter(|name| !name.is_empty())
            .map(String::as_str)
            .unwrap_or("someone");
        let description = fields
            .get(1)
            .filter(|description| !description.is_empty())
            .map(String::as_str)
            .unwrap_or("no description");
        let greeting = fields
            .get(2)
            .filter(|greeting| !greeting.is_empty())
            .map(String::as_str)
            .unwrap_or("...");

        self.advance_turn();
        if let Some(keyword) = keyword.and_then(non_empty_talk_keyword) {
            if fields.len() < 5 {
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope.");
                return MoveOutcome::Talked;
            }
            let response = talk_keyword_response(fields, keyword)
                .filter(|response| !response.is_empty())
                .unwrap_or("I cannot help thee with that.");
            self.message = format!("Talked to {name}: {response}");
        } else {
            self.message =
                format!("Talked to {name}: {description}. {greeting} (keyword loop out of scope).");
        }
        MoveOutcome::Talked
    }

}
