use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    pub fn climb_outdoors(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<MoveOutcome> {
        match overworld_klimb_entry_gate(self.climbing_gear != 0, self.player.transport.is_foot()) {
            OverworldKlimbEntryGate::NoGrapple => {
                self.message = "With what?".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            OverworldKlimbEntryGate::NotOnFoot => {
                self.message = "On foot!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            OverworldKlimbEntryGate::Proceed => {}
        }

        let direction = self.player.facing;
        let (dx, dy) = direction.delta();
        let nx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        let tile = self.grid[world_cell_index(nx, ny)];

        if let Some(object) = self.world_object_at(nx, ny) {
            self.message = format!(
                "Impassable! World object tile {} blocks outdoor climb at ({nx}, {ny}).",
                object.tile
            );
            return Ok(MoveOutcome::Blocked);
        }
        if let Some(entry) = self.world_damage_tile_at(game_dir, plane, nx, ny, tile)? {
            if !entry.effect.allows_transport(self.player.transport) {
                self.message = "Impassable!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        }
        if !is_outdoor_climbable_tile(tile) {
            self.message = if is_tile_walkable_for_transport(
                tile,
                self.passability.as_ref(),
                TransportState::Foot,
            ) {
                "Not climbable!".to_string()
            } else {
                "Impassable!".to_string()
            };
            return Ok(MoveOutcome::Blocked);
        }

        let (checked, falls) = self.apply_outdoor_climb_fall_checks();
        self.player.x = nx;
        self.player.y = ny;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        let fall_report = if falls.is_empty() {
            format!("fall checks passed for {checked} living member(s)")
        } else {
            falls.join("; ")
        };
        self.message = format!(
            "Climbed {} to ({nx}, {ny}) on {}; {fall_report}.",
            direction.name(),
            plane.key()
        );
        if let Some(entry) = self.world_plane_transition_at(game_dir, plane, nx, ny)? {
            let to_plane = entry.to_plane;
            let climb_message = self.message.clone();
            self.apply_world_plane_transition(game_dir, entry)?;
            let transition_message = self.message.clone();
            self.message = format!("{climb_message} {transition_message}");
            return Ok(MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: plane,
                to: to_plane,
            }));
        }
        Ok(MoveOutcome::Moved)
    }

    pub fn apply_outdoor_climb_fall_checks(&mut self) -> (usize, Vec<String>) {
        let mut checked = 0;
        let mut falls = Vec::new();
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            checked += 1;
            let roll = self.outdoor_climb_stat_roll();
            if outdoor_klimb_member_falls(self.party[index].climb_stat, roll) {
                let damage = self.outdoor_climb_damage_roll();
                let slot = self.party[index].slot;
                let applied = self.party[index].apply_damage(damage);
                falls.push(format!(
                    "Fell! slot {slot} took {applied} HP ({} HP left)",
                    self.party[index].hp
                ));
            }
        }
        (checked, falls)
    }

    pub fn outdoor_climb_stat_roll(&mut self) -> u8 {
        self.random_range_u8(1, 30)
    }

    pub fn outdoor_climb_damage_roll(&mut self) -> u8 {
        self.random_range_u8(1, 5)
    }

    pub fn resolve_sailed_ship_wind_gate(&mut self, direction: Direction) -> Option<MoveOutcome> {
        if !self.player.transport.is_ship_under_sail() {
            return None;
        }
        if !direction.is_cardinal() {
            self.message = "Sails need a cardinal heading.".to_string();
            return Some(MoveOutcome::Blocked);
        }

        let Some(wind_direction) = self.wind.direction() else {
            self.sail_cadence = 0;
            self.sail_stall_pending = true;
            self.advance_sailing_wait_turn();
            self.message = "Sails hang slack in calm wind.".to_string();
            return Some(MoveOutcome::SailStalled);
        };

        if direction == wind_direction {
            self.sail_cadence = 0;
            self.sail_stall_pending = false;
            return None;
        }
        if direction.opposite_cardinal() == Some(wind_direction) {
            self.sail_cadence = self.sail_cadence.wrapping_add(1) & 1;
            if self.sail_cadence == 0 {
                self.sail_stall_pending = false;
                return None;
            }
        } else {
            self.sail_cadence = 0;
        }

        self.advance_sailing_wait_turn();
        self.sail_stall_pending = true;
        self.message = format!(
            "Sails stalled by {} wind while heading {}.",
            self.wind.name(),
            direction.name()
        );
        Some(MoveOutcome::SailStalled)
    }

    pub fn resolve_balloon_wind_step(
        &mut self,
        direction: &mut Direction,
        nx: &mut isize,
        ny: &mut isize,
    ) -> Option<MoveOutcome> {
        if !self.player.transport.is_balloon() {
            return None;
        }

        let Some(wind_direction) = self.wind.direction() else {
            self.advance_turn();
            self.message = "Balloon hangs motionless in calm wind.".to_string();
            return Some(MoveOutcome::SailStalled);
        };

        let (dx, dy) = wind_direction.delta();
        *direction = wind_direction;
        *nx = self.player.x as isize + dx;
        *ny = self.player.y as isize + dy;
        self.player.facing = wind_direction;
        None
    }

    #[cfg(test)]
    pub fn open_facing(&mut self) -> MoveOutcome {
        self.open_facing_with_game_dir(None)
            .expect("open without a game dir cannot load sidecar metadata")
    }

    pub fn open_facing_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.open_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn open_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let (scene, floor) = match self.area {
            Area::Town { scene, floor } => (scene, floor),
            Area::Dungeon { scene, level } => {
                return self.open_dungeon_underfoot(game_dir, scene, level);
            }
            Area::World { .. } => {
                let (dx, dy) = direction.delta();
                let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                if self.surface_object_chest_slot_at(tx, ty).is_some() {
                    return Ok(self.start_surface_object_chest_prompt(
                        tx,
                        ty,
                        SurfaceChestVerb::Open,
                    ));
                }
                self.message = "Nothing to open here.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        };
        self.tick_door_tracker();
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = "Nothing to open there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if tile == 16 && self.is_recorded_open_town_door(scene, floor, tx, ty) {
            self.advance_turn_without_door_tick();
            self.message = "It's open!".to_string();
            return Ok(MoveOutcome::DoorOpened);
        }
        let revealed_secret_door =
            (96..=103).contains(&tile) && self.is_revealed_town_secret_door(scene, floor, tx, ty);
        if !revealed_secret_door
            && self
                .town_lock_at(game_dir, scene, floor, tx, ty, tile)?
                .is_some()
        {
            self.message = "Locked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if self.surface_object_chest_slot_at(tx, ty).is_some() {
            return Ok(self.start_surface_object_chest_prompt(tx, ty, SurfaceChestVerb::Open));
        }
        if !(96..=103).contains(&tile) {
            self.message = "Nothing to open!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[idx] = 16;
        self.record_open_town_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn_without_door_tick();
        if !revealed_secret_door {
            self.door_tracker = Some(DoorTracker {
                previous_tile: tile,
                x: tx,
                y: ty,
                turns_remaining: 4,
            });
        }
        self.message = "Opened!".to_string();
        Ok(MoveOutcome::DoorOpened)
    }

    pub fn open_dungeon_underfoot(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        match tile >> 4 {
            0x4 => Ok(self.open_dungeon_chest(
                scene,
                level,
                self.player.x,
                self.player.y,
                idx,
                tile,
                "Opened",
            )),
            0x7 => {
                self.advance_turn();
                self.message = "It's open!".to_string();
                Ok(MoveOutcome::ContainerOpened)
            }
            0xF => {
                let Some(entry) = self.dungeon_door_entry_at(
                    game_dir,
                    scene,
                    level,
                    self.player.x,
                    self.player.y,
                )?
                else {
                    self.message = "Nothing to open here.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if entry.open_cell == tile {
                    self.advance_turn();
                    self.message = "It's open!".to_string();
                    return Ok(MoveOutcome::DoorOpened);
                }
                if !dungeon_closed_door_matches(entry, tile) {
                    self.message =
                        "Dungeon door sidecar did not match the current cell byte.".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                self.grid[idx] = entry.open_cell;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = "Opened!".to_string();
                Ok(MoveOutcome::DoorOpened)
            }
            _ => {
                self.message = "Nothing to open here.".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn start_jimmy_party_prompt(&mut self, direction: Direction) -> MoveOutcome {
        self.active_jimmy = Some(JimmySession::new(direction));
        self.message = self.render_active_jimmy();
        MoveOutcome::Observed
    }

    pub fn render_active_jimmy(&self) -> String {
        let last = self.party.len().min(6);
        format!("Who picks? _\nChoose party member 1-{last}; Space/Esc cancels.")
    }

    pub fn step_active_jimmy(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(session) = self.active_jimmy.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                self.message = "None!".to_string();
                return Ok(Some(MoveOutcome::PromptDeclined));
            }
            let Some(digit) = ch
                .to_digit(10)
                .and_then(|digit| usize::try_from(digit).ok())
            else {
                continue;
            };
            if !(1..=self.party.len().min(6)).contains(&digit) {
                continue;
            }
            let outcome = self.jimmy_direction_with_game_dir_and_member(
                session.direction,
                Some(game_dir),
                Some(digit - 1),
            )?;
            return Ok(Some(outcome));
        }
        self.active_jimmy = Some(session);
        self.message = self.render_active_jimmy();
        Ok(None)
    }

    pub fn start_surface_object_chest_prompt(
        &mut self,
        x: usize,
        y: usize,
        verb: SurfaceChestVerb,
    ) -> MoveOutcome {
        self.active_surface_chest = Some(SurfaceChestSession::new(x, y, verb));
        self.message = self.render_active_surface_chest();
        MoveOutcome::Observed
    }

    pub fn render_active_surface_chest(&self) -> String {
        let last = self.party.len().min(6);
        format!("Who opens? _\nChoose party member 1-{last}; Space/Esc cancels.")
    }

    pub fn step_active_surface_chest(
        &mut self,
        key: char,
        suffix: &str,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(session) = self.active_surface_chest.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                self.message = "None!".to_string();
                return Ok(Some(MoveOutcome::PromptDeclined));
            }
            let Some(digit) = ch
                .to_digit(10)
                .and_then(|digit| usize::try_from(digit).ok())
            else {
                continue;
            };
            if !(1..=self.party.len().min(6)).contains(&digit) {
                continue;
            }
            let member_index = digit - 1;
            if !self.surface_chest_member_available(member_index) {
                self.message = party_member_unavailable_message(member_index);
                return Ok(Some(MoveOutcome::PromptDeclined));
            }
            let outcome = self
                .consume_surface_object_chest_at(
                    session.x,
                    session.y,
                    Some(member_index),
                    session.verb.label(),
                )
                .unwrap_or_else(|| {
                    self.message = "Nothing to open!".to_string();
                    MoveOutcome::Blocked
                });
            return Ok(Some(outcome));
        }
        self.active_surface_chest = Some(session);
        self.message = self.render_active_surface_chest();
        Ok(None)
    }

    pub fn surface_chest_member_available(&self, member_index: usize) -> bool {
        self.party
            .get(member_index)
            .is_some_and(|member| member.conscious())
    }

    #[cfg(test)]
    pub fn jimmy_facing(&mut self) -> MoveOutcome {
        self.jimmy_facing_with_game_dir_and_member(None, Some(0))
            .expect("jimmy without a game dir cannot load sidecar metadata")
    }

    pub fn jimmy_facing_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_facing_with_game_dir_and_member(game_dir, Some(0))
    }

    pub fn use_skull_key(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        if self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] == 0 {
            self.message = "No Skull Keys!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        match self.area {
            Area::Town { scene, floor } => self.use_skull_key_town_facing(game_dir, scene, floor),
            Area::Dungeon { .. } => {
                self.message = "Not here!".to_string();
                Ok(MoveOutcome::Blocked)
            }
            Area::World { .. } => {
                self.message = "No lock!".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn use_skull_key_town_facing(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = self.player.facing.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] =
            self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX].saturating_sub(1);
        let tx = tx as usize;
        let ty = ty as usize;
        if self.blocking_object_at(tx, ty).is_some() {
            self.advance_turn();
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::LockTried);
        }

        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if (96..=103).contains(&tile) && self.is_revealed_town_secret_door(scene, floor, tx, ty) {
            self.advance_turn();
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if let Some(entry) = self.town_lock_at(game_dir, scene, floor, tx, ty, tile)? {
            if entry.kind == TownLockKind::Magic {
                self.advance_turn();
                self.message = "Magic lock!".to_string();
                return Ok(MoveOutcome::LockTried);
            }
            self.grid[idx] = entry.unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if let Some(unlocked_tile) = Self::visible_jimmy_unlock_tile(tile) {
            self.grid[idx] = unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }

        self.advance_turn();
        self.message = "No lock!".to_string();
        Ok(MoveOutcome::LockTried)
    }

    pub fn jimmy_facing_with_game_dir_and_member(
        &mut self,
        game_dir: Option<&Path>,
        member_index: Option<usize>,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_direction_with_game_dir_and_member(self.player.facing, game_dir, member_index)
    }

    pub fn jimmy_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_direction_with_game_dir_and_member(direction, game_dir, Some(0))
    }

    pub fn jimmy_direction_with_game_dir_and_member(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
        member_index: Option<usize>,
    ) -> io::Result<MoveOutcome> {
        if let Area::Dungeon { scene, level } = self.area {
            let Some(member_index) = member_index else {
                return Ok(self.start_jimmy_party_prompt(direction));
            };
            let Some(member_index) = self.resolve_jimmy_member_index(Some(member_index)) else {
                return Ok(MoveOutcome::PromptDeclined);
            };
            return self.jimmy_dungeon_underfoot(game_dir, scene, level, member_index);
        }
        if self.keys == 0 {
            self.message = "No keys!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let Some(member_index) = member_index else {
            return Ok(self.start_jimmy_party_prompt(direction));
        };
        let Some(member_index) = self.resolve_jimmy_member_index(Some(member_index)) else {
            return Ok(MoveOutcome::PromptDeclined);
        };
        match self.area {
            Area::Town { scene, floor } => {
                self.jimmy_town_direction(game_dir, scene, floor, member_index, direction)
            }
            Area::World { .. } => {
                self.message = "No lock!".to_string();
                Ok(MoveOutcome::Blocked)
            }
            Area::Dungeon { .. } => unreachable!("dungeon Jimmy returns before key preflight"),
        }
    }

    pub fn resolve_jimmy_member_index(&mut self, member_index: Option<usize>) -> Option<usize> {
        let Some(member_index) = member_index else {
            self.message = "Who picks? Use J<party>.".to_string();
            return None;
        };
        if !self
            .party
            .get(member_index)
            .is_some_and(|member| member.conscious())
        {
            self.message = party_member_unavailable_message(member_index);
            return None;
        }
        Some(member_index)
    }

    pub fn jimmy_town_facing(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        member_index: usize,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_town_direction(game_dir, scene, floor, member_index, self.player.facing)
    }

    pub fn jimmy_town_direction(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        member_index: usize,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        if let Some(outcome) = self.jimmy_surface_object_chest_at(tx, ty, member_index) {
            return Ok(outcome);
        }
        if self.blocking_object_at(tx, ty).is_some() {
            return self.jimmy_town_pickpocket(game_dir, scene, floor, tx, ty, member_index);
        }
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if (96..=103).contains(&tile) && self.is_revealed_town_secret_door(scene, floor, tx, ty) {
            self.advance_turn();
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if let Some(entry) = self.town_lock_at(game_dir, scene, floor, tx, ty, tile)? {
            if entry.kind == TownLockKind::Magic {
                self.message = "Magic lock!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            if !self.jimmy_lock_pick_succeeds(member_index) {
                self.keys = self.keys.saturating_sub(1);
                self.advance_turn();
                self.message = "Key broke!".to_string();
                return Ok(MoveOutcome::LockTried);
            }
            self.grid[idx] = entry.unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if let Some(unlocked_tile) = Self::visible_jimmy_unlock_tile(tile) {
            if !self.jimmy_lock_pick_succeeds(member_index) {
                self.keys = self.keys.saturating_sub(1);
                self.advance_turn();
                self.message = "Key broke!".to_string();
                return Ok(MoveOutcome::LockTried);
            }
            self.grid[idx] = unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        self.message = "No lock!".to_string();
        Ok(MoveOutcome::Blocked)
    }

    fn jimmy_town_pickpocket(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        tx: usize,
        ty: usize,
        member_index: usize,
    ) -> io::Result<MoveOutcome> {
        let Some((slot, dialog_id)) = self.npc_pickpocket_target(floor, tx, ty) else {
            self.message = "No one is there!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if !self.jimmy_lock_pick_succeeds(member_index) {
            self.keys = self.keys.saturating_sub(1);
            self.advance_turn();
            self.message = "Key broke!".to_string();
            return Ok(MoveOutcome::LockTried);
        }

        if self.mark_npc_pickpocketed_once(scene, floor, slot) {
            self.add_moral_standing(2);
        }
        self.advance_turn();
        self.message = self.npc_pickpocket_thanks_line(game_dir, scene, dialog_id)?;
        Ok(MoveOutcome::LockTried)
    }

    fn npc_pickpocket_target(&self, floor: i8, tx: usize, ty: usize) -> Option<(usize, u8)> {
        if floor < 0 {
            return None;
        }
        let floor = floor as u8;
        self.npcs
            .iter()
            .find(|npc| !npc.is_player_phantom() && npc.x == tx && npc.y == ty && npc.z == floor)
            .map(|npc| (npc.slot, npc.dialog_id))
    }

    fn mark_npc_pickpocketed_once(&mut self, scene: Scene, floor: i8, slot: usize) -> bool {
        let marker = (scene.byte, floor, slot);
        if self.pickpocketed_npcs.contains(&marker) {
            return false;
        }
        self.pickpocketed_npcs.push(marker);
        true
    }

    pub fn surface_object_chest_slot_at(
        &self,
        x: usize,
        y: usize,
    ) -> Option<(usize, ActiveObject)> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                (self.object_occupies(object, x, y)
                    && Self::surface_object_chest_stat(object).is_some())
                .then_some((slot, object))
            })
    }

    pub fn surface_object_chest_stat(object: ActiveObject) -> Option<u8> {
        let is_furniture_chest = (TILE_FURNITURE_FIRST..=TILE_FURNITURE_LAST)
            .contains(&object.type_byte)
            || (TILE_FURNITURE_FIRST..=TILE_FURNITURE_LAST).contains(&object.tile);
        (is_furniture_chest && object.aux1 != 0).then_some(object.aux1)
    }

    pub fn jimmy_surface_object_chest_at(
        &mut self,
        x: usize,
        y: usize,
        member_index: usize,
    ) -> Option<MoveOutcome> {
        let (slot, object) = self.surface_object_chest_slot_at(x, y)?;
        let stat = Self::surface_object_chest_stat(object)?;
        let Some(threshold) =
            object_chest_jimmy_threshold(stat, self.party[member_index].class_byte)
        else {
            self.advance_turn();
            self.message = "Key broke!".to_string();
            return Some(MoveOutcome::LockTried);
        };
        let roll = self.surface_object_chest_jimmy_roll();
        if object_chest_jimmy_succeeds(threshold, roll) {
            self.keys = self.keys.saturating_sub(1);
            self.advance_turn();
            self.message = "Unlocked!".to_string();
        } else {
            if let Some(object) = self.active_objects.get_mut(slot) {
                object.aux1 &= 0x7f;
            }
            self.advance_turn();
            self.message = "Key broke!".to_string();
        }
        Some(MoveOutcome::LockTried)
    }

    pub fn surface_object_chest_jimmy_roll(&mut self) -> u8 {
        self.random_range_u8(JIMMY_OBJECT_DIE_LOW, JIMMY_OBJECT_DIE_HIGH)
    }

    pub fn consume_surface_object_chest_at(
        &mut self,
        x: usize,
        y: usize,
        member_index: Option<usize>,
        verb: &str,
    ) -> Option<MoveOutcome> {
        let (slot, object) = self.surface_object_chest_slot_at(x, y)?;
        let stat = Self::surface_object_chest_stat(object)?;
        let chest_class = stat & 0x7f;
        let trap_note = (stat & 0x80 != 0).then(|| {
            let target = member_index.unwrap_or_else(|| self.shared_trap_default_target_slot());
            self.apply_shared_trap_effect_to_slot(target)
        });
        let content_note = self.generate_surface_object_chest_content(slot, x, y, chest_class);
        self.clear_consumed_active_object_slot(slot);
        self.rewrite_surface_object_chest_cell(x, y);
        if matches!(self.area, Area::Town { .. }) {
            self.moral_standing = town_chest_open_standing(self.moral_standing);
        } else if matches!(self.area, Area::World { .. }) {
            self.cache_current_world_overlay();
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = match trap_note {
            Some(trap) => {
                format!("{verb} object chest at ({x}, {y}); {trap}; {content_note}.")
            }
            None => format!("{verb} object chest at ({x}, {y}); {content_note}."),
        };
        Some(MoveOutcome::ContainerOpened)
    }

    pub fn rewrite_surface_object_chest_cell(&mut self, x: usize, y: usize) {
        match self.area {
            Area::Town { .. } => {
                if x < 32 && y < 32 {
                    self.grid[y * 32 + x] = LOCATION_MARKER_CLEANUP_TILE;
                }
            }
            Area::World { .. } => {
                if x < WORLD_SIDE && y < WORLD_SIDE {
                    self.grid[world_cell_index(x, y)] = LOCATION_MARKER_CLEANUP_TILE;
                }
            }
            Area::Dungeon { .. } => {}
        }
    }

    pub fn generate_surface_object_chest_content(
        &mut self,
        slot: usize,
        x: usize,
        y: usize,
        chest_class: u8,
    ) -> String {
        let mut parts = Vec::new();
        for (row, threshold) in CHEST_PRIMARY_POOL_THRESHOLDS.iter().copied().enumerate() {
            let roll = self.surface_object_chest_roll(slot, x, y, chest_class, row, 0, 30);
            if !chest_primary_pool_row_succeeds(chest_class, threshold, roll) {
                continue;
            }
            match row {
                0 => {
                    let amount = self.surface_object_chest_roll(slot, x, y, chest_class, row, 1, 2);
                    self.apply_object_pickup(ObjectPickupKind::Food, amount);
                    parts.push(format!("{amount} food"));
                }
                1 => {
                    let amount = self.surface_object_chest_roll(slot, x, y, chest_class, row, 1, 2);
                    self.apply_object_pickup(ObjectPickupKind::Torches, amount);
                    parts.push(format!("{amount} torches"));
                }
                2 => {
                    let amount = self.surface_object_chest_roll(slot, x, y, chest_class, row, 1, 2);
                    self.apply_object_pickup(ObjectPickupKind::Gems, amount);
                    parts.push(format!("{amount} gems"));
                }
                3 => {
                    let amount = self.surface_object_chest_roll(slot, x, y, chest_class, row, 1, 2);
                    self.apply_object_pickup(ObjectPickupKind::Keys, amount);
                    parts.push(format!("{amount} keys"));
                }
                4 => {
                    let subtype = self.surface_object_chest_zero_based_roll(
                        slot,
                        x,
                        y,
                        chest_class,
                        row,
                        1,
                        8,
                    );
                    let kind = ObjectPickupKind::Scroll(subtype);
                    self.apply_object_pickup(kind, 1);
                    parts.push(format!("1 {}", kind.label()));
                }
                5 => {
                    let subtype = self.surface_object_chest_zero_based_roll(
                        slot,
                        x,
                        y,
                        chest_class,
                        row,
                        1,
                        8,
                    );
                    self.apply_object_pickup(ObjectPickupKind::Potion(subtype), 1);
                    parts.push(format!("1 {} potion", potion_label(subtype)));
                }
                6 => {
                    let upper = chest_class.saturating_mul(3);
                    if upper != 0 {
                        let amount =
                            self.surface_object_chest_roll(slot, x, y, chest_class, row, 1, upper);
                        self.apply_object_pickup(ObjectPickupKind::Gold, amount);
                        parts.push(format!("{amount} gold"));
                    }
                }
                7 => {
                    let marker = self.surface_object_chest_roll(
                        slot,
                        x,
                        y,
                        chest_class,
                        row,
                        1,
                        chest_class,
                    );
                    parts.push(format!("marker {marker}"));
                }
                _ => {}
            }
        }

        let attempts = chest_secondary_pool_attempts(chest_class);
        for attempt in 0..attempts {
            let row = self.surface_object_chest_zero_based_roll(
                slot,
                x,
                y,
                chest_class,
                usize::from(attempt),
                2,
                CHEST_SECONDARY_POOL_ROW_COUNT,
            );
            let Some(threshold) = chest_secondary_pool_threshold(row) else {
                continue;
            };
            let roll = self.surface_object_chest_roll(
                slot,
                x,
                y,
                chest_class,
                usize::from(attempt),
                3,
                30,
            );
            if chest_primary_pool_row_succeeds(chest_class, threshold, roll) {
                self.apply_object_pickup(ObjectPickupKind::Equipment(row), 1);
                parts.push(format!("1 {}", equipment_name(row)));
            }
        }

        if parts.is_empty() {
            "chest was empty".to_string()
        } else {
            format!("chest grants {}", parts.join(", "))
        }
    }

    pub fn surface_object_chest_roll(
        &self,
        slot: usize,
        x: usize,
        y: usize,
        chest_class: u8,
        row: usize,
        stage: u8,
        upper: u8,
    ) -> u8 {
        if upper == 0 {
            0
        } else {
            1 + (self.surface_object_chest_seed(slot, x, y, chest_class, row, stage) % upper)
        }
    }

    pub fn surface_object_chest_zero_based_roll(
        &self,
        slot: usize,
        x: usize,
        y: usize,
        chest_class: u8,
        row: usize,
        stage: u8,
        upper: usize,
    ) -> usize {
        if upper == 0 {
            0
        } else {
            usize::from(self.surface_object_chest_seed(slot, x, y, chest_class, row, stage)) % upper
        }
    }

    pub fn surface_object_chest_seed(
        &self,
        slot: usize,
        x: usize,
        y: usize,
        chest_class: u8,
        row: usize,
        stage: u8,
    ) -> u8 {
        (self.turn as u8)
            ^ (slot as u8).wrapping_mul(7)
            ^ (x as u8).wrapping_mul(11)
            ^ (y as u8).wrapping_mul(13)
            ^ chest_class.wrapping_mul(17)
            ^ (row as u8).wrapping_mul(19)
            ^ stage.wrapping_mul(23)
    }

    pub fn add_moral_standing(&mut self, amount: u8) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self
            .moral_standing
            .saturating_add(amount)
            .min(MORAL_STANDING_MAX);
        self.moral_standing - before
    }

    fn npc_pickpocket_thanks_line(
        &self,
        game_dir: Option<&Path>,
        scene: Scene,
        dialog_id: u8,
    ) -> io::Result<String> {
        if dialog_id <= 1 {
            return Ok("Thanks!".to_string());
        }
        let Some(game_dir) = game_dir else {
            return Ok("Thanks!".to_string());
        };
        let dialogue_path = game_dir.join(format!("{}.TLK", scene.family.stem()));
        if !dialogue_path.exists() {
            return Ok("Thanks!".to_string());
        }
        let dialogue = parse_tlk(&dialogue_path)?;
        Ok(dialogue
            .get(&(dialog_id as u16))
            .and_then(|fields| fields.get(4))
            .filter(|line| !line.is_empty())
            .cloned()
            .unwrap_or_else(|| "Thanks!".to_string()))
    }

    pub fn jimmy_dungeon_underfoot(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        member_index: usize,
    ) -> io::Result<MoveOutcome> {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        Ok(match tile >> 4 {
            0x4 => {
                if self.keys == 0 {
                    self.message = "No keys!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                if Self::is_plain_closed_dungeon_chest(tile) {
                    self.keys = self.keys.saturating_sub(1);
                    self.advance_turn();
                    self.message = "Key broke!".to_string();
                    return Ok(MoveOutcome::LockTried);
                }

                let class_byte = self
                    .party
                    .get(member_index)
                    .map(|member| member.class_byte)
                    .unwrap_or_default();
                let threshold = Self::dungeon_chest_pick_threshold(level, class_byte);
                let roll = self.random_range_u8(JIMMY_OBJECT_DIE_LOW, JIMMY_OBJECT_DIE_HIGH);
                if roll > threshold {
                    self.keys = self.keys.saturating_sub(1);
                    self.advance_turn();
                    self.message = "Key broke!".to_string();
                    return Ok(MoveOutcome::LockTried);
                }

                self.grid[idx] = 0x70 | (tile & 0x0f);
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = "Unlocked!".to_string();
                MoveOutcome::LockTried
            }
            0x7 => {
                self.advance_turn();
                self.message = "It's open!".to_string();
                MoveOutcome::LockTried
            }
            0xF => {
                let Some(entry) = self.dungeon_door_entry_at(
                    game_dir,
                    scene,
                    level,
                    self.player.x,
                    self.player.y,
                )?
                else {
                    self.message = "No lock!".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if entry.open_cell == tile {
                    self.advance_turn();
                    self.message = "It's open!".to_string();
                    return Ok(MoveOutcome::LockTried);
                }
                if self.keys == 0 {
                    self.message = "No keys!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                if !dungeon_closed_door_matches(entry, tile) {
                    self.message =
                        "Dungeon door sidecar did not match the current cell byte.".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                if !self.jimmy_lock_pick_succeeds(member_index) {
                    self.keys = self.keys.saturating_sub(1);
                    self.advance_turn();
                    self.message = "Key broke!".to_string();
                    return Ok(MoveOutcome::LockTried);
                }
                self.grid[idx] = entry.open_cell;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = "Unlocked!".to_string();
                MoveOutcome::LockTried
            }
            _ => {
                self.message = "No lock!".to_string();
                MoveOutcome::Blocked
            }
        })
    }

    pub fn jimmy_lock_pick_succeeds(&mut self, member_index: usize) -> bool {
        let class_byte = self
            .party
            .get(member_index)
            .map(|member| member.class_byte)
            .unwrap_or_default();
        let roll = self.jimmy_lock_pick_roll();
        jimmy_door_succeeds(class_byte, roll)
    }

    pub fn jimmy_lock_pick_roll(&mut self) -> u8 {
        self.random_range_u8(JIMMY_DOOR_DIE_LOW, JIMMY_DOOR_DIE_HIGH)
    }

    pub fn random_range_u8(&mut self, low: u8, high: u8) -> u8 {
        u5_prng_range_u16(&mut self.prng_state, u16::from(low), u16::from(high)) as u8
    }

    pub fn random_mod_u8(&mut self, modulus: u8) -> u8 {
        if modulus == 0 {
            0
        } else {
            self.random_range_u8(0, modulus - 1)
        }
    }

    pub fn visible_jimmy_unlock_tile(tile: u8) -> Option<u8> {
        if (97..=103).contains(&tile) && tile % 2 == 1 {
            Some(tile - 1)
        } else {
            None
        }
    }

    #[cfg(test)]
    pub fn get_dungeon_underfoot(&mut self, scene: DungeonScene, level: u8) -> MoveOutcome {
        self.get_dungeon_underfoot_with_contents(None, scene, level)
    }

    pub fn get_dungeon_underfoot_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        let chest_entries = game_dir
            .map(load_dungeon_chest_content_entries)
            .transpose()?
            .flatten();
        Ok(self.get_dungeon_underfoot_with_contents(chest_entries.as_deref(), scene, level))
    }

    pub fn get_dungeon_underfoot_with_contents(
        &mut self,
        chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
    ) -> MoveOutcome {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        match tile >> 4 {
            0x4 => {
                self.message = "Must open it first.".to_string();
                MoveOutcome::Blocked
            }
            0x7 => self.consume_dungeon_chest(
                chest_entries,
                scene,
                level,
                self.player.x,
                self.player.y,
                idx,
                tile,
                "Got",
            ),
            0xF => {
                self.message = "Must open it first.".to_string();
                MoveOutcome::Blocked
            }
            _ => {
                self.message = "Nothing to get here.".to_string();
                MoveOutcome::Blocked
            }
        }
    }

    pub fn get_object_pickup_at(
        &mut self,
        game_dir: &Path,
        target: PlayTarget,
        floor: i8,
        x: usize,
        y: usize,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entries) = load_object_pickup_entries(game_dir)? else {
            return Ok(None);
        };
        let hit = self
            .active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                if !self.object_occupies(object, x, y) {
                    return None;
                }
                let entry = entries
                    .iter()
                    .copied()
                    .find(|entry| object_pickup_matches(*entry, target, floor, x, y, object))?;
                Some((slot, object.tile, entry))
            });
        let Some((slot, tile, entry)) = hit else {
            return Ok(None);
        };

        self.clear_consumed_active_object_slot(slot);
        self.apply_object_pickup(entry.kind, entry.amount);
        self.cache_current_world_overlay();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Got {} {} from active-object tile {tile} at ({x}, {y}) in {} floor {}.",
            entry.amount,
            entry.kind.label(),
            target.key(),
            floor
        );
        Ok(Some(MoveOutcome::Got))
    }

    pub fn search_object_pickup_at(
        &mut self,
        entries: Option<&[ObjectPickupEntry]>,
        target: PlayTarget,
        floor: i8,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        let entries = entries?;
        let hit = self
            .active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                if !self.object_occupies(object, x, y) {
                    return None;
                }
                let entry = entries
                    .iter()
                    .copied()
                    .find(|entry| object_pickup_matches(*entry, target, floor, x, y, object))?;
                Some((slot, object.tile, entry))
            });
        let (slot, tile, entry) = hit?;

        self.clear_consumed_active_object_slot(slot);
        self.apply_object_pickup(entry.kind, entry.amount);
        self.cache_current_world_overlay();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Found {} {} from active-object tile {tile} at ({x}, {y}) in {} floor {}.",
            entry.amount,
            entry.kind.label(),
            target.key(),
            floor
        );
        Some(MoveOutcome::Searched)
    }

    pub fn search_surface_object_trap_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        let (slot, object) = self.surface_object_chest_slot_at(x, y)?;
        let stat = Self::surface_object_chest_stat(object)?;
        let member_index = self
            .party
            .iter()
            .position(|member| member.living())
            .unwrap_or_default();
        let member_trap_detection = self
            .party
            .get(member_index)
            .map(|member| member.class_byte)
            .unwrap_or_default();
        let trappable = stat & 0x80 != 0;
        let difficulty = stat & 0x7f;
        let threshold =
            search_trap_detection_threshold(trappable, difficulty, member_trap_detection);
        let roll = self.surface_object_search_trap_roll(slot, x, y, stat, member_index);
        let detection_bit = roll >= threshold;
        let visibility = search_trap_visibility(trappable, difficulty, detection_bit);

        self.advance_turn();
        self.message = format!(
            "Searched active-object tile {} at ({x}, {y}); {}.",
            object.tile,
            surface_search_trap_visibility_label(visibility)
        );
        Some(MoveOutcome::Searched)
    }

    pub fn surface_object_search_trap_roll(
        &self,
        slot: usize,
        x: usize,
        y: usize,
        stat: u8,
        member_index: usize,
    ) -> u8 {
        1 + (self.surface_object_chest_seed(slot, x, y, stat, member_index, 2)
            % JIMMY_OBJECT_DIE_HIGH)
    }

    pub fn get_native_object_pickup_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        let (slot, object) = self
            .active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find(|(_, object)| {
                self.object_occupies(*object, x, y) && gettable_object_visual(object.tile)
            })?;

        let Some(grant) = native_object_pickup_grant(object) else {
            self.message = match inventory_add_class(object.type_byte) {
                InventoryAddClass::MustOpenFirst => "Must open it first.".to_string(),
                _ => "Nothing to get here.".to_string(),
            };
            return Some(MoveOutcome::Blocked);
        };

        self.clear_consumed_active_object_slot(slot);
        self.apply_object_pickup(grant.kind, grant.amount);
        self.cache_current_world_overlay();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Got {} {} from active-object tile {} at ({x}, {y}).",
            grant.amount,
            grant.kind.label(),
            object.tile
        );
        Some(MoveOutcome::Got)
    }

    pub fn apply_object_pickup(&mut self, kind: ObjectPickupKind, amount: u8) {
        match kind {
            ObjectPickupKind::Food => {
                self.food = self
                    .food
                    .saturating_add(u16::from(amount))
                    .min(PARTY_FOOD_CAP)
            }
            ObjectPickupKind::Gold => {
                self.gold = self
                    .gold
                    .saturating_add(u16::from(amount))
                    .min(PARTY_GOLD_CAP)
            }
            ObjectPickupKind::Keys => {
                self.keys = self.keys.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP)
            }
            ObjectPickupKind::Gems => {
                self.gems = self.gems.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP)
            }
            ObjectPickupKind::Torches => {
                self.torches = self
                    .torches
                    .saturating_add(amount)
                    .min(PARTY_BYTE_STOCK_CAP)
            }
            ObjectPickupKind::SkullKeys => {
                let slot = &mut self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX];
                *slot = slot.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
            }
            ObjectPickupKind::Potion(index) => {
                if let Some(stock) = self.potion_stock.get_mut(index) {
                    *stock = stock.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
                }
            }
            ObjectPickupKind::Scroll(index) => {
                if let Some(stock) = self.scroll_stock.get_mut(index) {
                    *stock = stock.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
                }
            }
            ObjectPickupKind::Equipment(index) => {
                if let Some(stock) = self.equipment_stock.get_mut(index) {
                    let units = inventory_add_equipment_units(index).saturating_mul(amount);
                    *stock = stock.saturating_add(units).min(PARTY_BYTE_STOCK_CAP);
                }
            }
            ObjectPickupKind::Moonstone(index) => {
                if index < MOONSTONE_SLOT_COUNT {
                    self.moonstone_slots[index] = MoonstoneGateSlot::invalid();
                }
            }
            ObjectPickupKind::MagicCarpet => {
                let slot = &mut self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX];
                *slot = slot.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
            }
            ObjectPickupKind::HmsCapePlans => {
                self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = self.special_items
                    [SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX]
                    .max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::SandalwoodBox => {
                self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] =
                    self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::CrownOfLordBritish => {
                self.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] =
                    self.special_items[SPECIAL_ITEM_CROWN_LB_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::SceptreOfLordBritish => {
                self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] =
                    self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::AmuletOfLordBritish => {
                self.special_items[SPECIAL_ITEM_AMULET_LB_INDEX] =
                    self.special_items[SPECIAL_ITEM_AMULET_LB_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::ShadowlordShard(index) => {
                if index < SHADOWLORD_COUNT {
                    self.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + index] =
                        SPECIAL_ITEM_OWNED_VALUE;
                }
            }
        }
    }

    pub fn apply_get_tile_grant(&mut self, grant: ObjectPickupGrant) {
        self.apply_object_pickup(grant.kind, grant.amount);
        if matches!(grant.kind, ObjectPickupKind::Food) {
            self.debit_crop_or_table_food_moral();
        }
    }

    pub fn debit_crop_or_table_food_moral(&mut self) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self
            .moral_standing
            .saturating_sub(KARMA_CROP_OR_TABLE_FOOD_DEBIT);
        before - self.moral_standing
    }

    pub fn get_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.get_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn get_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::World { plane } => self.get_world_direction(game_dir, plane, direction),
            Area::Town { scene, floor } => {
                self.get_town_direction(game_dir, scene, floor, direction)
            }
            Area::Dungeon { scene, level } => {
                self.get_dungeon_underfoot_with_game_dir(Some(game_dir), scene, level)
            }
        }
    }

    pub fn get_world_facing(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<MoveOutcome> {
        self.get_world_direction(game_dir, plane, self.player.facing)
    }

    pub fn get_world_direction(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if let Some(outcome) = self.get_moonstone_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_fixed_hidden_treasure_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_object_pickup_at(
            game_dir,
            PlayTarget::World(plane),
            plane.save_floor(),
            tx,
            ty,
        )? {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_native_object_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if self.surface_object_chest_slot_at(tx, ty).is_some() {
            return Ok(self.start_surface_object_chest_prompt(tx, ty, SurfaceChestVerb::Get));
        }
        if self.world_object_at(tx, ty).is_some() {
            self.message = "Nothing to get there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let idx = world_cell_index(tx, ty);
        let tile = self.grid[idx];
        let Some(entries) = load_world_get_tile_entries(game_dir)? else {
            self.message = "Nothing to get here.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(entry) = entries
            .iter()
            .find(|entry| world_get_tile_matches(**entry, plane, tx, ty, tile))
            .copied()
        else {
            self.message = "Nothing to get here.".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.grid[idx] = entry.replacement_tile;
        if let Some(grant) = entry.grant {
            self.apply_get_tile_grant(grant);
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = tile_get_message(
            format!("Got world tile {tile} at ({tx}, {ty}) on {}", plane.key()),
            entry.replacement_tile,
            entry.grant,
        );
        Ok(MoveOutcome::Got)
    }

    pub fn get_town_facing(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        self.get_town_direction(game_dir, scene, floor, self.player.facing)
    }

    pub fn get_town_direction(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = "Nothing to get there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        if let Some(outcome) = self.get_moonstone_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_fixed_hidden_treasure_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) =
            self.get_object_pickup_at(game_dir, PlayTarget::Town(scene), floor, tx, ty)?
        {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_native_object_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if self.surface_object_chest_slot_at(tx, ty).is_some() {
            return Ok(self.start_surface_object_chest_prompt(tx, ty, SurfaceChestVerb::Get));
        }
        if self.blocking_object_at(tx, ty).is_some() {
            self.message = "Nothing to get there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if let Some(outcome) = self.get_town_table_food(idx, tx, ty, tile, dx, dy) {
            return Ok(outcome);
        }
        let Some(entries) = load_town_get_tile_entries(game_dir)? else {
            self.message = "Nothing to get here.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(entry) = entries
            .iter()
            .find(|entry| town_get_tile_matches(**entry, scene, floor, tx, ty, tile))
            .copied()
        else {
            self.message = "Nothing to get here.".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.grid[idx] = entry.replacement_tile;
        if let Some(grant) = entry.grant {
            self.apply_get_tile_grant(grant);
        }
        if self
            .door_tracker
            .is_some_and(|tracker| tracker.x == tx && tracker.y == ty)
        {
            self.door_tracker = None;
        }
        self.forget_open_town_door(scene, floor, tx, ty);
        self.forget_revealed_town_secret_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = tile_get_message(
            format!("Got tile {tile} at ({tx}, {ty})"),
            entry.replacement_tile,
            entry.grant,
        );
        Ok(MoveOutcome::Got)
    }

    pub fn get_town_table_food(
        &mut self,
        idx: usize,
        x: usize,
        y: usize,
        tile: u8,
        dx: isize,
        dy: isize,
    ) -> Option<MoveOutcome> {
        let replacement = match (tile, dx, dy) {
            (0x9b, 0, -1) => 0x95,
            (0x9c, 0, -1) => 0x9a,
            (0x9c, 0, 1) => 0x9b,
            (0x9b | 0x9c, _, _) => {
                self.message = "The plate cannot be reached.".to_string();
                return Some(MoveOutcome::Blocked);
            }
            _ => return None,
        };

        self.grid[idx] = replacement;
        self.food = self.food.saturating_add(1).min(PARTY_FOOD_CAP);
        self.debit_crop_or_table_food_moral();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Ate food from table tile 0x{tile:02X} at ({x}, {y}); replaced with tile 0x{replacement:02X}; added 1 food."
        );
        Some(MoveOutcome::Got)
    }

    pub fn search_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.search_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn search_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let entries = load_secret_door_entries(game_dir)?.unwrap_or_default();
        let chest_entries = load_dungeon_chest_content_entries(game_dir)?;
        let object_pickup_entries = load_object_pickup_entries(game_dir)?;
        Ok(self.search_direction_secret_with_object_pickups(
            direction,
            &entries,
            chest_entries.as_deref(),
            object_pickup_entries.as_deref(),
        ))
    }

    pub fn search_facing_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
    ) -> MoveOutcome {
        self.search_direction_secret(self.player.facing, entries, chest_entries)
    }

    pub fn search_direction_secret(
        &mut self,
        direction: Direction,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
    ) -> MoveOutcome {
        self.search_direction_secret_with_object_pickups(direction, entries, chest_entries, None)
    }

    pub fn search_direction_secret_with_object_pickups(
        &mut self,
        direction: Direction,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
        object_pickup_entries: Option<&[ObjectPickupEntry]>,
    ) -> MoveOutcome {
        match self.area {
            Area::Town { scene, floor } => self.search_town_secret_direction_with_object_pickups(
                entries,
                scene,
                floor,
                direction,
                object_pickup_entries,
            ),
            Area::Dungeon { scene, level } => {
                self.search_dungeon_secret(entries, chest_entries, scene, level)
            }
            Area::World { plane } => self.search_world_moonstone_direction_with_object_pickups(
                plane,
                direction,
                object_pickup_entries,
            ),
        }
    }

    pub fn search_world_moonstone(&mut self, plane: WorldPlane) -> MoveOutcome {
        self.search_world_moonstone_direction(plane, self.player.facing)
    }

    pub fn search_world_moonstone_direction(
        &mut self,
        plane: WorldPlane,
        direction: Direction,
    ) -> MoveOutcome {
        self.search_world_moonstone_direction_with_object_pickups(plane, direction, None)
    }

    pub fn search_world_moonstone_direction_with_object_pickups(
        &mut self,
        plane: WorldPlane,
        direction: Direction,
        object_pickup_entries: Option<&[ObjectPickupEntry]>,
    ) -> MoveOutcome {
        let (dx, dy) = direction.delta();
        let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if let Some(outcome) = self.search_object_pickup_at(
            object_pickup_entries,
            PlayTarget::World(plane),
            plane.save_floor(),
            tx,
            ty,
        ) {
            return outcome;
        }
        if let Some(outcome) = self.search_active_object_treasure_marker_at(tx, ty) {
            return outcome;
        }
        if let Some(outcome) = self.search_surface_object_trap_at(tx, ty) {
            return outcome;
        }
        let target_tile = self.grid.get(ty * WORLD_SIDE + tx).copied();
        let skip_moonstone_scan = target_tile == Some(0xdc);
        if !skip_moonstone_scan {
            if let Some(outcome) = self.search_moonstone_pickup_at(tx, ty, |slot| {
                moonstone_slot_matches_world(slot, plane, tx, ty)
            }) {
                return outcome;
            }
        }
        if let Some(outcome) = self.search_rare_reagent_at(plane, tx, ty) {
            return outcome;
        }
        if let Some(outcome) = self.search_fixed_hidden_treasure_at(
            HiddenTreasureTarget::World(plane),
            plane.save_floor(),
            tx,
            ty,
        ) {
            return outcome;
        }
        if skip_moonstone_scan {
            self.message =
                "Searched a generic find marker; no Moonstone scan was attempted.".to_string();
            return MoveOutcome::Blocked;
        }
        self.message = "Nothing to search here.".to_string();
        MoveOutcome::Blocked
    }

    pub fn search_town_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        scene: Scene,
        floor: i8,
    ) -> MoveOutcome {
        self.search_town_secret_direction(entries, scene, floor, self.player.facing)
    }

    pub fn search_town_secret_direction(
        &mut self,
        entries: &[SecretDoorEntry],
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> MoveOutcome {
        self.search_town_secret_direction_with_object_pickups(
            entries, scene, floor, direction, None,
        )
    }

    pub fn search_town_secret_direction_with_object_pickups(
        &mut self,
        entries: &[SecretDoorEntry],
        scene: Scene,
        floor: i8,
        direction: Direction,
        object_pickup_entries: Option<&[ObjectPickupEntry]>,
    ) -> MoveOutcome {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = "Nothing to search there.".to_string();
            return MoveOutcome::Blocked;
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if let Some(outcome) = self.search_object_pickup_at(
            object_pickup_entries,
            PlayTarget::Town(scene),
            floor,
            tx,
            ty,
        ) {
            return outcome;
        }
        if let Some(outcome) = self.search_active_object_treasure_marker_at(tx, ty) {
            return outcome;
        }
        if let Some(outcome) = self.search_surface_object_trap_at(tx, ty) {
            return outcome;
        }
        let reveal_tile = entries.iter().find_map(|entry| match *entry {
            SecretDoorEntry::Town {
                scene: entry_scene,
                floor: entry_floor,
                x,
                y,
                reveal_tile,
                expected_tile,
            } if entry_scene == scene
                && entry_floor == floor
                && x == tx
                && y == ty
                && expected_tile.map_or(true, |expected| expected == tile) =>
            {
                Some(reveal_tile)
            }
            _ => None,
        });
        let Some(reveal_tile) = reveal_tile else {
            if tile == 0xdc {
                if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                    HiddenTreasureTarget::Town(scene.byte),
                    floor,
                    tx,
                    ty,
                ) {
                    return outcome;
                }
                self.message =
                    "Searched a generic find marker; no Moonstone scan was attempted.".to_string();
                return MoveOutcome::Blocked;
            }
            let miss_message =
                town_search_live_tile_miss_message(tile).unwrap_or("No secret door found.");
            if let Some(outcome) = self.search_moonstone_pickup_at(tx, ty, |slot| {
                moonstone_slot_matches_town(slot, scene, floor, tx, ty)
            }) {
                return outcome;
            }
            if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                HiddenTreasureTarget::Town(scene.byte),
                floor,
                tx,
                ty,
            ) {
                return outcome;
            }
            self.message = miss_message.to_string();
            return MoveOutcome::Blocked;
        };
        if !(24..=63).contains(&tile) {
            if tile == 0xdc {
                if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                    HiddenTreasureTarget::Town(scene.byte),
                    floor,
                    tx,
                    ty,
                ) {
                    return outcome;
                }
                self.message =
                    "Searched a generic find marker; no Moonstone scan was attempted.".to_string();
                return MoveOutcome::Blocked;
            }
            let miss_message =
                town_search_live_tile_miss_message(tile).unwrap_or("No secret door found.");
            if let Some(outcome) = self.search_moonstone_pickup_at(tx, ty, |slot| {
                moonstone_slot_matches_town(slot, scene, floor, tx, ty)
            }) {
                return outcome;
            }
            if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                HiddenTreasureTarget::Town(scene.byte),
                floor,
                tx,
                ty,
            ) {
                return outcome;
            }
            self.message = miss_message.to_string();
            return MoveOutcome::Blocked;
        }

        self.grid[idx] = reveal_tile;
        self.forget_open_town_door(scene, floor, tx, ty);
        self.record_revealed_town_secret_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Revealed secret door at ({tx}, {ty}).");
        MoveOutcome::Searched
    }

    pub fn search_moonstone_at<F>(
        &mut self,
        x: usize,
        y: usize,
        matches_slot: F,
        miss_message: &str,
    ) -> MoveOutcome
    where
        F: Fn(MoonstoneGateSlot) -> bool,
    {
        if let Some(outcome) = self.search_moonstone_pickup_at(x, y, matches_slot) {
            return outcome;
        }

        self.message = miss_message.to_string();
        MoveOutcome::Blocked
    }

    pub fn search_moonstone_pickup_at<F>(
        &mut self,
        x: usize,
        y: usize,
        matches_slot: F,
    ) -> Option<MoveOutcome>
    where
        F: Fn(MoonstoneGateSlot) -> bool,
    {
        let Some(slot_index) = self
            .moonstone_slots
            .iter()
            .copied()
            .enumerate()
            .rev()
            .find_map(|(slot_index, slot)| {
                (slot.is_valid() && matches_slot(slot)).then_some(slot_index)
            })
        else {
            return None;
        };

        if self.moonstone_pickup_exists(slot_index) {
            self.message = format!(
                "Moonstone phase {} is already surfaced as a strange rock.",
                slot_index + 1
            );
            return Some(MoveOutcome::Blocked);
        }

        let Some(z) = self.current_floor() else {
            return None;
        };
        let pickup = ActiveObject::moonstone_pickup(slot_index, x, y, z);
        if self.allocate_active_object_slot(pickup).is_none() {
            self.message = "No active-object slot for Moonstone pickup.".to_string();
            return Some(MoveOutcome::Blocked);
        }

        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Found a strange rock for Moonstone phase {}.",
            slot_index + 1
        );
        Some(MoveOutcome::Searched)
    }

    pub fn search_rare_reagent_at(
        &mut self,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        if plane != WorldPlane::Britannia || self.clock.hour != 0 {
            return None;
        }
        let point = RARE_REAGENT_HARVEST_POINTS
            .iter()
            .find(|point| point.x == x && point.y == y)?;
        if self.rare_reagent_harvest_days[point.index] == self.clock.day {
            return None;
        }

        let amount = self.rare_reagent_harvest_amount(point.index);
        self.reagents[point.reagent_index] = self.reagents[point.reagent_index]
            .saturating_add(amount)
            .min(99);
        self.rare_reagent_harvest_days[point.index] = self.clock.day;
        self.advance_turn();
        self.message = format!("Found {amount} sprigs of {}.", point.label);
        Some(MoveOutcome::Searched)
    }

    pub fn rare_reagent_harvest_amount(&mut self, _point_index: usize) -> u8 {
        self.random_range_u8(
            RARE_REAGENT_HARVEST_QUANTITY_MIN,
            RARE_REAGENT_HARVEST_QUANTITY_MAX,
        )
    }

    pub fn search_fixed_hidden_treasure_at(
        &mut self,
        target: HiddenTreasureTarget,
        floor: i8,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        let mut matching_entries = FIXED_HIDDEN_TREASURES.iter().copied().filter(|entry| {
            entry.target == target && entry.floor == floor && entry.x == x && entry.y == y
        });
        let Some(entry) = matching_entries.find(|entry| {
            if self.fixed_hidden_treasure_pickup_exists(entry.record) {
                true
            } else {
                self.fixed_hidden_treasure_rule_allows(*entry, x, y)
            }
        }) else {
            return None;
        };
        if self.fixed_hidden_treasure_pickup_exists(entry.record) {
            self.message = format!("{} is already surfaced here.", entry.pickup.label());
            return Some(MoveOutcome::Blocked);
        }
        let pickup = ActiveObject::fixed_hidden_treasure_pickup(entry.record, x, y, floor);
        if self.allocate_active_object_slot(pickup).is_none() {
            self.message = "No active-object slot for hidden treasure pickup.".to_string();
            return Some(MoveOutcome::Blocked);
        }
        self.mark_fixed_hidden_treasure_found(entry);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Found {}.", entry.pickup.label());
        Some(MoveOutcome::Searched)
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_len() -> usize {
        FIXED_HIDDEN_TREASURES.len()
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_records_are_sequential() -> bool {
        FIXED_HIDDEN_TREASURES
            .iter()
            .enumerate()
            .all(|(record, entry)| entry.record == record)
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_fingerprint() -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for entry in FIXED_HIDDEN_TREASURES {
            for value in [
                entry.record as u64,
                fixed_hidden_treasure_target_code(entry.target),
                fixed_hidden_treasure_floor_code(entry.floor),
                entry.x as u64,
                entry.y as u64,
                fixed_hidden_treasure_pickup_code(entry.pickup),
                entry.state as u64,
                fixed_hidden_treasure_rule_code(entry.rule),
            ] {
                hash ^= value;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        hash
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_pickup_counts() -> [usize; HIDDEN_TREASURE_PICKUP_CLASS_COUNT]
    {
        let mut counts = [0usize; HIDDEN_TREASURE_PICKUP_CLASS_COUNT];
        for entry in FIXED_HIDDEN_TREASURES {
            counts[fixed_hidden_treasure_pickup_code(entry.pickup) as usize] += 1;
        }
        counts
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_rule_counts() -> [usize; HIDDEN_TREASURE_RULE_COUNT] {
        let mut counts = [0usize; HIDDEN_TREASURE_RULE_COUNT];
        for entry in FIXED_HIDDEN_TREASURES {
            counts[fixed_hidden_treasure_rule_code(entry.rule) as usize] += 1;
        }
        counts
    }

    pub fn fixed_hidden_treasure_rule_allows(
        &self,
        entry: FixedHiddenTreasureEntry,
        x: usize,
        y: usize,
    ) -> bool {
        match entry.rule {
            HiddenTreasureRule::OneShot => !self.fixed_hidden_treasure_found(entry.record),
            HiddenTreasureRule::KeyNpcGated => {
                self.keys != 0
                    && !self.fixed_hidden_treasure_found(entry.record)
                    && !self.fixed_hidden_treasure_target_occupied(x, y)
            }
            HiddenTreasureRule::Daily => self.fixed_hidden_treasure_daily_day != self.clock.day,
            HiddenTreasureRule::SingleUseNpcGated => {
                !self.fixed_hidden_treasure_found(entry.record)
                    && !self.fixed_hidden_treasure_target_occupied(x, y)
            }
        }
    }

    pub fn mark_fixed_hidden_treasure_found(&mut self, entry: FixedHiddenTreasureEntry) {
        match entry.rule {
            HiddenTreasureRule::Daily => self.fixed_hidden_treasure_daily_day = self.clock.day,
            _ => self.set_fixed_hidden_treasure_found(entry.record),
        }
    }

    pub fn fixed_hidden_treasure_found(&self, record: usize) -> bool {
        let byte = record / 8;
        let bit = record % 8;
        self.fixed_hidden_treasure_found
            .get(byte)
            .is_some_and(|value| value & (1 << bit) != 0)
    }

    pub fn set_fixed_hidden_treasure_found(&mut self, record: usize) {
        let byte = record / 8;
        let bit = record % 8;
        if let Some(value) = self.fixed_hidden_treasure_found.get_mut(byte) {
            *value |= 1 << bit;
        }
    }

    pub fn fixed_hidden_treasure_target_occupied(&self, x: usize, y: usize) -> bool {
        self.active_objects.iter().skip(1).any(|object| {
            self.object_occupies(*object, x, y)
                && !object.is_empty()
                && object.fixed_hidden_treasure_record().is_none()
                && object.moonstone_slot_index().is_none()
        })
    }

    pub fn fixed_hidden_treasure_pickup_exists(&self, record: usize) -> bool {
        self.active_objects
            .iter()
            .any(|object| object.fixed_hidden_treasure_record() == Some(record))
    }

    pub fn fixed_hidden_treasure_pickup_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(object_slot, object)| {
                if self.object_occupies(*object, x, y) {
                    object
                        .fixed_hidden_treasure_record()
                        .map(|record| (object_slot, record))
                } else {
                    None
                }
            })
    }

    pub fn active_object_treasure_marker_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .rev()
            .find_map(|(object_slot, object)| {
                if !self.object_occupies(object, x, y)
                    || object.type_byte != FIXED_HIDDEN_TREASURE_OBJECT_TILE
                {
                    return None;
                }
                object
                    .fixed_hidden_treasure_record()
                    .or_else(|| {
                        let record = object.aux1 as usize;
                        (record < FIXED_HIDDEN_TREASURE_COUNT).then_some(record)
                    })
                    .map(|record| (object_slot, record))
            })
    }

    pub fn search_active_object_treasure_marker_at(
        &mut self,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        let (object_slot, record) = self.active_object_treasure_marker_at(x, y)?;
        let Some(entry) = FIXED_HIDDEN_TREASURES
            .iter()
            .find(|entry| entry.record == record)
            .copied()
        else {
            self.message = "Unknown active-object treasure marker.".to_string();
            return Some(MoveOutcome::Blocked);
        };

        self.clear_consumed_active_object_slot(object_slot);
        let grant = self.apply_fixed_hidden_treasure_pickup(entry.pickup, entry.state);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Found {}{}.", entry.pickup.label(), grant);
        Some(MoveOutcome::Searched)
    }

    pub fn get_fixed_hidden_treasure_pickup_at(
        &mut self,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        let (object_slot, record) = self.fixed_hidden_treasure_pickup_at(x, y)?;
        let Some(entry) = FIXED_HIDDEN_TREASURES
            .iter()
            .find(|entry| entry.record == record)
            .copied()
        else {
            self.message = "Unknown hidden treasure pickup.".to_string();
            return Some(MoveOutcome::Blocked);
        };
        self.clear_consumed_active_object_slot(object_slot);
        let grant = self.apply_fixed_hidden_treasure_pickup(entry.pickup, entry.state);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Got {}{}.", entry.pickup.label(), grant);
        Some(MoveOutcome::Got)
    }

    pub fn apply_fixed_hidden_treasure_pickup(
        &mut self,
        pickup: HiddenTreasurePickup,
        state: u8,
    ) -> String {
        match pickup {
            HiddenTreasurePickup::Food => {
                self.food = self
                    .food
                    .saturating_add(u16::from(state))
                    .min(PARTY_FOOD_CAP);
                format!("; added {state} food")
            }
            HiddenTreasurePickup::SackOfGold => {
                self.gold = self
                    .gold
                    .saturating_add(u16::from(state))
                    .min(SHOP_GOLD_CAP);
                format!("; added {state} gold")
            }
            HiddenTreasurePickup::RingOfKeys => {
                self.keys = self.keys.saturating_add(state).min(99);
                format!("; added {state} keys")
            }
            HiddenTreasurePickup::Gem => {
                self.gems = self.gems.saturating_add(state).min(99);
                format!("; added {state} gems")
            }
            HiddenTreasurePickup::Torches => {
                self.torches = self.torches.saturating_add(state).min(99);
                format!("; added {state} torches")
            }
            HiddenTreasurePickup::Potion => {
                let subtype = (state as usize) & 7;
                self.potion_stock[subtype] = self.potion_stock[subtype].saturating_add(1).min(99);
                format!("; added 1 {} potion", potion_label(subtype))
            }
            HiddenTreasurePickup::Scroll => {
                let subtype = (state as usize) & 7;
                self.scroll_stock[subtype] = self.scroll_stock[subtype].saturating_add(1).min(99);
                format!("; added 1 scroll subtype {subtype}")
            }
            HiddenTreasurePickup::Armour
            | HiddenTreasurePickup::Weapon
            | HiddenTreasurePickup::Ring
            | HiddenTreasurePickup::Amulet => {
                let item = state as usize;
                if item < EQUIPMENT_COUNT {
                    let amount = inventory_add_equipment_units(item);
                    self.equipment_stock[item] =
                        self.equipment_stock[item].saturating_add(amount).min(99);
                    format!("; added equipment id {item}")
                } else {
                    "; no compatible inventory slot".to_string()
                }
            }
            HiddenTreasurePickup::MoldyCorpse | HiddenTreasurePickup::RottingBody => {
                "; found no usable inventory".to_string()
            }
        }
    }

    pub fn search_dungeon_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        _chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
    ) -> MoveOutcome {
        self.search_dungeon_secret_focus(
            entries,
            _chest_entries,
            scene,
            level,
            DungeonLookFocus::Ahead,
        )
    }

    pub fn search_dungeon_focus_with_game_dir(
        &mut self,
        focus: DungeonLookFocus,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Search is only implemented for dungeon mode in this slice.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let entries = load_secret_door_entries(game_dir)?.unwrap_or_default();
        let chest_entries = load_dungeon_chest_content_entries(game_dir)?;
        Ok(self.search_dungeon_secret_focus(
            &entries,
            chest_entries.as_deref(),
            scene,
            level,
            focus,
        ))
    }

    pub fn search_dungeon_secret_focus(
        &mut self,
        entries: &[SecretDoorEntry],
        _chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
        focus: DungeonLookFocus,
    ) -> MoveOutcome {
        if !self.has_personal_light() {
            self.message = "You see: darkness.".to_string();
            return MoveOutcome::Blocked;
        }
        let (tx, ty) = self.dungeon_look_focus_coord(focus);
        let idx = dungeon_cell_index(level, tx, ty);
        let tile = self.grid[idx];
        if let Some(reveal_cell) = entries.iter().find_map(|entry| match *entry {
            SecretDoorEntry::Dungeon {
                scene: entry_scene,
                level: entry_level,
                x,
                y,
                reveal_cell,
                expected_cell,
            } if entry_scene == scene
                && entry_level == level
                && x == tx
                && y == ty
                && expected_cell.map_or(true, |expected| expected == tile) =>
            {
                Some(reveal_cell)
            }
            _ => None,
        }) {
            self.grid[idx] = reveal_cell;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = format!(
                "Revealed dungeon secret door at ({tx}, {ty}) on {} level {level}.",
                scene.key()
            );
            return MoveOutcome::Searched;
        }

        match tile >> 4 {
            0x4 => self.search_dungeon_chest(scene, level, tx, ty, tile),
            _ if tile == 0x60 => self.search_dungeon_pit(scene, level, tx, ty),
            _ if dungeon_search_secret_pit_reveal(tile, level).is_some() => {
                self.search_dungeon_secret_pit(scene, level, tx, ty, idx, tile)
            }
            _ if is_dungeon_bomb_trap(tile) => {
                self.search_dungeon_bomb_trap(scene, level, tx, ty, idx, tile)
            }
            _ if dungeon_search_wall_rewrite(tile).is_some() => {
                self.search_dungeon_wall_rewrite(scene, level, tx, ty, idx, tile)
            }
            _ => self.search_dungeon_feature(scene, level, tx, ty, tile),
        }
    }

    pub fn search_dungeon_pit(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
    ) -> MoveOutcome {
        self.advance_turn();
        self.message = format!(
            "Searched dungeon pit at ({x}, {y}) on {} level {level}; nothing found.",
            scene.key()
        );
        MoveOutcome::Searched
    }

    pub fn search_dungeon_secret_pit(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        let Some(reveal) = dungeon_search_secret_pit_reveal(tile, level) else {
            return self.search_dungeon_feature(scene, level, x, y, tile);
        };
        self.grid[idx] = 0x60;
        if matches!(
            reveal,
            DungeonSearchSecretPitReveal::RewriteAndStampLevelBelow
        ) {
            let below_idx = dungeon_cell_index(level + 1, x, y);
            self.grid[below_idx] |= DUNGEON_RUNTIME_VARIANT_BIT;
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Searched dungeon pit at ({x}, {y}) on {} level {level}; found a secret door.",
            scene.key()
        );
        MoveOutcome::Searched
    }

    pub fn search_dungeon_bomb_trap(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        let class_byte = self
            .party
            .iter()
            .find(|member| member.living())
            .map(|member| member.class_byte)
            .unwrap_or_default();
        let threshold = Self::dungeon_chest_pick_threshold(level, class_byte);
        let roll = self.dungeon_chest_trap_roll(level, x, y, tile, 0, 30);
        match dungeon_bomb_search_outcome(threshold, roll) {
            DungeonBombSearchOutcome::NothingOnPit => {
                self.advance_turn();
                self.message = format!(
                    "Searched dungeon pit at ({x}, {y}) on {} level {level}; nothing found.",
                    scene.key()
                );
            }
            DungeonBombSearchOutcome::SpringBomb => {
                self.grid[idx] = 0x00;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = format!(
                    "Searched dungeon bomb trap at ({x}, {y}) on {} level {level}; sprung the bomb.",
                    scene.key()
                );
            }
        }
        MoveOutcome::Searched
    }

    pub fn search_dungeon_wall_rewrite(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        match dungeon_search_wall_rewrite(tile) {
            Some(DungeonSearchWallRewrite::NarrateOnly) => {
                self.search_dungeon_feature(scene, level, x, y, tile)
            }
            Some(DungeonSearchWallRewrite::ToFlavourFind(reveal_cell)) => {
                self.grid[idx] = reveal_cell;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = format!(
                    "Searched dungeon wall at ({x}, {y}) on {} level {level}; found a hidden passage.",
                    scene.key()
                );
                MoveOutcome::Searched
            }
            Some(DungeonSearchWallRewrite::ToHiddenWallReveal(reveal_cell)) => {
                self.grid[idx] = reveal_cell;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = format!(
                    "Searched dungeon wall at ({x}, {y}) on {} level {level}; revealed a hidden wall.",
                    scene.key()
                );
                MoveOutcome::Searched
            }
            None => self.search_dungeon_feature(scene, level, x, y, tile),
        }
    }

    pub fn search_dungeon_feature(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> MoveOutcome {
        let description = dungeon_search_description(tile);
        self.advance_turn();
        self.message = format!(
            "Searched dungeon cell at ({x}, {y}) on {} level {level}; found {description}.",
            scene.key()
        );
        MoveOutcome::Searched
    }
}

fn native_object_pickup_grant(object: ActiveObject) -> Option<ObjectPickupGrant> {
    let subtype = object.aux1;
    let grant = match inventory_add_class(object.type_byte) {
        InventoryAddClass::MustOpenFirst | InventoryAddClass::NothingToGet => return None,
        InventoryAddClass::Gold => ObjectPickupGrant {
            kind: ObjectPickupKind::Gold,
            amount: subtype,
        },
        InventoryAddClass::Potion => ObjectPickupGrant {
            kind: ObjectPickupKind::Potion((subtype & SCROLL_GRANT_LABEL_MASK) as usize),
            amount: 1,
        },
        InventoryAddClass::ScrollOrPlans => {
            if subtype & !SCROLL_GRANT_LABEL_MASK != 0 {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::HmsCapePlans,
                    amount: 1,
                }
            } else {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::Scroll(scroll_grant_label_id(subtype) as usize),
                    amount: 1,
                }
            }
        }
        InventoryAddClass::Equipment => ObjectPickupGrant {
            kind: ObjectPickupKind::Equipment(subtype as usize),
            amount: 1,
        },
        InventoryAddClass::Key => {
            if subtype & 0x80 != 0 {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::SkullKeys,
                    amount: subtype & 0x7f,
                }
            } else {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::Keys,
                    amount: subtype,
                }
            }
        }
        InventoryAddClass::Gem => ObjectPickupGrant {
            kind: ObjectPickupKind::Gems,
            amount: subtype,
        },
        InventoryAddClass::Torch => ObjectPickupGrant {
            kind: ObjectPickupKind::Torches,
            amount: subtype,
        },
        InventoryAddClass::SandalwoodBox => ObjectPickupGrant {
            kind: ObjectPickupKind::SandalwoodBox,
            amount: 1,
        },
        InventoryAddClass::Food => ObjectPickupGrant {
            kind: ObjectPickupKind::Food,
            amount: subtype,
        },
        InventoryAddClass::Moonstone => ObjectPickupGrant {
            kind: ObjectPickupKind::Moonstone(subtype as usize),
            amount: 1,
        },
        InventoryAddClass::MagicCarpet => ObjectPickupGrant {
            kind: ObjectPickupKind::MagicCarpet,
            amount: 1,
        },
        InventoryAddClass::ShadowlordShard => ObjectPickupGrant {
            kind: ObjectPickupKind::ShadowlordShard(subtype as usize),
            amount: 1,
        },
        InventoryAddClass::CrownOfLordBritish => ObjectPickupGrant {
            kind: ObjectPickupKind::CrownOfLordBritish,
            amount: 1,
        },
        InventoryAddClass::SceptreOfLordBritish => ObjectPickupGrant {
            kind: ObjectPickupKind::SceptreOfLordBritish,
            amount: 1,
        },
        InventoryAddClass::AmuletOfLordBritish => ObjectPickupGrant {
            kind: ObjectPickupKind::AmuletOfLordBritish,
            amount: 1,
        },
    };
    Some(grant)
}

fn surface_search_trap_visibility_label(visibility: SearchTrapVisibility) -> &'static str {
    match visibility {
        SearchTrapVisibility::NoTrap => "no trap",
        SearchTrapVisibility::SimpleTrap => "simple trap",
        SearchTrapVisibility::ComplexTrap => "complex trap",
        SearchTrapVisibility::GenericTrap => "trap",
    }
}

#[derive(Clone, Copy)]
pub struct RareReagentHarvestPoint {
    pub index: usize,
    pub x: usize,
    pub y: usize,
    pub reagent_index: usize,
    pub label: &'static str,
}

pub const RARE_REAGENT_HARVEST_POINTS: [RareReagentHarvestPoint; RARE_REAGENT_HARVEST_POINT_COUNT] = [
    RareReagentHarvestPoint {
        index: 0,
        x: 182,
        y: 54,
        reagent_index: REAGENT_MANDRAKE,
        label: "Mandrake Root",
    },
    RareReagentHarvestPoint {
        index: 1,
        x: 97,
        y: 165,
        reagent_index: REAGENT_MANDRAKE,
        label: "Mandrake Root",
    },
    RareReagentHarvestPoint {
        index: 2,
        x: 44,
        y: 137,
        reagent_index: REAGENT_NIGHTSHADE,
        label: "Nightshade",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasureTarget {
    World(WorldPlane),
    Town(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasurePickup {
    Armour,
    Weapon,
    Scroll,
    Potion,
    Gem,
    Food,
    Torches,
    Ring,
    Amulet,
    RingOfKeys,
    SackOfGold,
    MoldyCorpse,
    RottingBody,
}

#[cfg(test)]
const HIDDEN_TREASURE_PICKUP_CLASS_COUNT: usize = 13;

#[cfg(test)]
fn fixed_hidden_treasure_pickup_code(pickup: HiddenTreasurePickup) -> u64 {
    match pickup {
        HiddenTreasurePickup::Armour => 0,
        HiddenTreasurePickup::Weapon => 1,
        HiddenTreasurePickup::Scroll => 2,
        HiddenTreasurePickup::Potion => 3,
        HiddenTreasurePickup::Gem => 4,
        HiddenTreasurePickup::Food => 5,
        HiddenTreasurePickup::Torches => 6,
        HiddenTreasurePickup::Ring => 7,
        HiddenTreasurePickup::Amulet => 8,
        HiddenTreasurePickup::RingOfKeys => 9,
        HiddenTreasurePickup::SackOfGold => 10,
        HiddenTreasurePickup::MoldyCorpse => 11,
        HiddenTreasurePickup::RottingBody => 12,
    }
}

impl HiddenTreasurePickup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Armour => "armour",
            Self::Weapon => "weapon",
            Self::Scroll => "scroll",
            Self::Potion => "potion",
            Self::Gem => "gem",
            Self::Food => "food",
            Self::Torches => "torches",
            Self::Ring => "ring",
            Self::Amulet => "amulet",
            Self::RingOfKeys => "ring of keys",
            Self::SackOfGold => "sack of gold",
            Self::MoldyCorpse => "moldy corpse",
            Self::RottingBody => "rotting body",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasureRule {
    OneShot,
    KeyNpcGated,
    Daily,
    SingleUseNpcGated,
}

#[cfg(test)]
const HIDDEN_TREASURE_RULE_COUNT: usize = 4;

#[cfg(test)]
fn fixed_hidden_treasure_rule_code(rule: HiddenTreasureRule) -> u64 {
    match rule {
        HiddenTreasureRule::OneShot => 0,
        HiddenTreasureRule::KeyNpcGated => 1,
        HiddenTreasureRule::Daily => 2,
        HiddenTreasureRule::SingleUseNpcGated => 3,
    }
}

#[cfg(test)]
fn fixed_hidden_treasure_target_code(target: HiddenTreasureTarget) -> u64 {
    match target {
        HiddenTreasureTarget::World(WorldPlane::Britannia) => 0,
        HiddenTreasureTarget::World(WorldPlane::Underworld) => 255,
        HiddenTreasureTarget::Town(scene) => 1000 + u64::from(scene),
    }
}

#[cfg(test)]
fn fixed_hidden_treasure_floor_code(floor: i8) -> u64 {
    if floor == -1 { 255 } else { floor as u8 as u64 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedHiddenTreasureEntry {
    pub record: usize,
    pub target: HiddenTreasureTarget,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub pickup: HiddenTreasurePickup,
    pub state: u8,
    pub rule: HiddenTreasureRule,
}

const fn ht(
    record: usize,
    target: HiddenTreasureTarget,
    floor: i8,
    x: usize,
    y: usize,
    pickup: HiddenTreasurePickup,
    state: u8,
    rule: HiddenTreasureRule,
) -> FixedHiddenTreasureEntry {
    FixedHiddenTreasureEntry {
        record,
        target,
        floor,
        x,
        y,
        pickup,
        state,
        rule,
    }
}

pub const FIXED_HIDDEN_TREASURES: &[FixedHiddenTreasureEntry] = &[
    ht(
        0,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        1,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        2,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        3,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        4,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        5,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        6,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        7,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        8,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        9,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        10,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        11,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        12,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        15,
        HiddenTreasurePickup::Scroll,
        255,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        13,
        HiddenTreasureTarget::Town(18),
        -1,
        6,
        8,
        HiddenTreasurePickup::RingOfKeys,
        9,
        HiddenTreasureRule::KeyNpcGated,
    ),
    ht(
        14,
        HiddenTreasureTarget::Town(5),
        0,
        2,
        2,
        HiddenTreasurePickup::RingOfKeys,
        133,
        HiddenTreasureRule::Daily,
    ),
    ht(
        15,
        HiddenTreasureTarget::World(WorldPlane::Britannia),
        0,
        80,
        64,
        HiddenTreasurePickup::Weapon,
        39,
        HiddenTreasureRule::SingleUseNpcGated,
    ),
    ht(
        16,
        HiddenTreasureTarget::Town(18),
        1,
        6,
        7,
        HiddenTreasurePickup::Weapon,
        35,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        17,
        HiddenTreasureTarget::Town(18),
        1,
        6,
        23,
        HiddenTreasurePickup::Weapon,
        40,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        18,
        HiddenTreasureTarget::Town(1),
        0,
        5,
        8,
        HiddenTreasurePickup::Gem,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        19,
        HiddenTreasureTarget::Town(1),
        0,
        6,
        25,
        HiddenTreasurePickup::Armour,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        20,
        HiddenTreasureTarget::Town(1),
        0,
        8,
        25,
        HiddenTreasurePickup::Armour,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        21,
        HiddenTreasureTarget::Town(1),
        0,
        5,
        23,
        HiddenTreasurePickup::Potion,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        22,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        13,
        HiddenTreasurePickup::Potion,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        23,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        14,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        24,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        16,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        25,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        17,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        26,
        HiddenTreasureTarget::Town(1),
        1,
        19,
        24,
        HiddenTreasurePickup::Food,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        27,
        HiddenTreasureTarget::Town(1),
        0,
        3,
        27,
        HiddenTreasurePickup::Torches,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        28,
        HiddenTreasureTarget::Town(1),
        0,
        29,
        27,
        HiddenTreasurePickup::Ring,
        42,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        29,
        HiddenTreasureTarget::Town(2),
        0,
        1,
        2,
        HiddenTreasurePickup::Food,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        30,
        HiddenTreasureTarget::Town(2),
        0,
        26,
        6,
        HiddenTreasurePickup::Weapon,
        20,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        31,
        HiddenTreasureTarget::Town(2),
        1,
        6,
        24,
        HiddenTreasurePickup::Ring,
        43,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        32,
        HiddenTreasureTarget::Town(3),
        0,
        16,
        21,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        33,
        HiddenTreasureTarget::Town(3),
        0,
        10,
        20,
        HiddenTreasurePickup::Food,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        34,
        HiddenTreasureTarget::Town(3),
        0,
        1,
        29,
        HiddenTreasurePickup::Food,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        35,
        HiddenTreasureTarget::Town(3),
        0,
        23,
        30,
        HiddenTreasurePickup::Weapon,
        38,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        36,
        HiddenTreasureTarget::Town(3),
        0,
        29,
        1,
        HiddenTreasurePickup::Torches,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        37,
        HiddenTreasureTarget::Town(4),
        0,
        11,
        29,
        HiddenTreasurePickup::Weapon,
        18,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        38,
        HiddenTreasureTarget::Town(4),
        0,
        26,
        22,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        39,
        HiddenTreasureTarget::Town(4),
        0,
        26,
        1,
        HiddenTreasurePickup::Scroll,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        40,
        HiddenTreasureTarget::Town(4),
        0,
        2,
        13,
        HiddenTreasurePickup::MoldyCorpse,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        41,
        HiddenTreasureTarget::Town(4),
        0,
        2,
        14,
        HiddenTreasurePickup::MoldyCorpse,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        42,
        HiddenTreasureTarget::Town(4),
        0,
        4,
        14,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        43,
        HiddenTreasureTarget::Town(4),
        0,
        3,
        16,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        44,
        HiddenTreasureTarget::Town(4),
        0,
        2,
        18,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        45,
        HiddenTreasureTarget::Town(4),
        0,
        3,
        16,
        HiddenTreasurePickup::Weapon,
        21,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        46,
        HiddenTreasureTarget::Town(4),
        -1,
        22,
        27,
        HiddenTreasurePickup::Weapon,
        37,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        47,
        HiddenTreasureTarget::Town(4),
        -1,
        22,
        20,
        HiddenTreasurePickup::RingOfKeys,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        48,
        HiddenTreasureTarget::Town(5),
        0,
        8,
        27,
        HiddenTreasurePickup::SackOfGold,
        99,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        49,
        HiddenTreasureTarget::Town(5),
        1,
        11,
        13,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        50,
        HiddenTreasureTarget::Town(5),
        1,
        11,
        12,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        51,
        HiddenTreasureTarget::Town(5),
        1,
        21,
        8,
        HiddenTreasurePickup::Potion,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        52,
        HiddenTreasureTarget::Town(5),
        1,
        23,
        8,
        HiddenTreasurePickup::Potion,
        2,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        53,
        HiddenTreasureTarget::Town(5),
        1,
        23,
        7,
        HiddenTreasurePickup::Scroll,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        54,
        HiddenTreasureTarget::Town(6),
        1,
        6,
        24,
        HiddenTreasurePickup::Gem,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        55,
        HiddenTreasureTarget::Town(7),
        0,
        2,
        5,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        56,
        HiddenTreasureTarget::Town(7),
        0,
        7,
        6,
        HiddenTreasurePickup::Potion,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        57,
        HiddenTreasureTarget::Town(8),
        1,
        18,
        21,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        58,
        HiddenTreasureTarget::Town(8),
        1,
        21,
        25,
        HiddenTreasurePickup::RingOfKeys,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        59,
        HiddenTreasureTarget::Town(9),
        0,
        12,
        10,
        HiddenTreasurePickup::Torches,
        9,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        60,
        HiddenTreasureTarget::Town(11),
        0,
        15,
        21,
        HiddenTreasurePickup::Potion,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        61,
        HiddenTreasureTarget::Town(11),
        0,
        9,
        14,
        HiddenTreasurePickup::Gem,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        62,
        HiddenTreasureTarget::Town(11),
        0,
        12,
        16,
        HiddenTreasurePickup::SackOfGold,
        50,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        63,
        HiddenTreasureTarget::Town(13),
        0,
        2,
        24,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        64,
        HiddenTreasureTarget::Town(14),
        0,
        12,
        16,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        65,
        HiddenTreasureTarget::Town(14),
        0,
        16,
        14,
        HiddenTreasurePickup::Amulet,
        45,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        66,
        HiddenTreasureTarget::Town(14),
        0,
        12,
        17,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        67,
        HiddenTreasureTarget::Town(14),
        0,
        12,
        14,
        HiddenTreasurePickup::Potion,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        68,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        20,
        HiddenTreasurePickup::Armour,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        69,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        21,
        HiddenTreasurePickup::Armour,
        11,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        70,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        22,
        HiddenTreasurePickup::Armour,
        9,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        71,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        23,
        HiddenTreasurePickup::Armour,
        12,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        72,
        HiddenTreasureTarget::Town(17),
        1,
        13,
        21,
        HiddenTreasurePickup::Weapon,
        30,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        73,
        HiddenTreasureTarget::Town(17),
        -1,
        18,
        7,
        HiddenTreasurePickup::RingOfKeys,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        74,
        HiddenTreasureTarget::Town(17),
        -1,
        23,
        20,
        HiddenTreasurePickup::Ring,
        44,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        75,
        HiddenTreasureTarget::Town(18),
        1,
        18,
        17,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        76,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        13,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        77,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        14,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        78,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        16,
        HiddenTreasurePickup::Scroll,
        2,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        79,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        17,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        80,
        HiddenTreasureTarget::Town(18),
        2,
        7,
        19,
        HiddenTreasurePickup::Ring,
        43,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        81,
        HiddenTreasureTarget::Town(19),
        0,
        2,
        3,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        82,
        HiddenTreasureTarget::Town(19),
        0,
        7,
        5,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        83,
        HiddenTreasureTarget::Town(19),
        0,
        7,
        7,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        84,
        HiddenTreasureTarget::Town(19),
        0,
        2,
        7,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        85,
        HiddenTreasureTarget::Town(19),
        0,
        7,
        5,
        HiddenTreasurePickup::Scroll,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        86,
        HiddenTreasureTarget::Town(19),
        0,
        2,
        3,
        HiddenTreasurePickup::Ring,
        44,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        87,
        HiddenTreasureTarget::Town(20),
        0,
        25,
        18,
        HiddenTreasurePickup::Gem,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        88,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        13,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        89,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        14,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        90,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        16,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        91,
        HiddenTreasureTarget::Town(22),
        0,
        13,
        13,
        HiddenTreasurePickup::Ring,
        42,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        92,
        HiddenTreasureTarget::Town(22),
        0,
        12,
        3,
        HiddenTreasurePickup::RingOfKeys,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        93,
        HiddenTreasureTarget::Town(23),
        0,
        1,
        15,
        HiddenTreasurePickup::Amulet,
        47,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        94,
        HiddenTreasureTarget::Town(24),
        0,
        22,
        19,
        HiddenTreasurePickup::RingOfKeys,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        95,
        HiddenTreasureTarget::Town(24),
        0,
        22,
        19,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        96,
        HiddenTreasureTarget::Town(25),
        0,
        16,
        25,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        97,
        HiddenTreasureTarget::Town(26),
        0,
        4,
        11,
        HiddenTreasurePickup::Amulet,
        46,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        98,
        HiddenTreasureTarget::Town(27),
        0,
        17,
        11,
        HiddenTreasurePickup::Potion,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        99,
        HiddenTreasureTarget::Town(27),
        0,
        17,
        10,
        HiddenTreasurePickup::Potion,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        100,
        HiddenTreasureTarget::Town(28),
        0,
        22,
        19,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        101,
        HiddenTreasureTarget::Town(30),
        2,
        9,
        23,
        HiddenTreasurePickup::Ring,
        42,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        102,
        HiddenTreasureTarget::Town(30),
        2,
        7,
        23,
        HiddenTreasurePickup::Amulet,
        46,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        103,
        HiddenTreasureTarget::Town(30),
        2,
        7,
        20,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        104,
        HiddenTreasureTarget::Town(30),
        1,
        19,
        22,
        HiddenTreasurePickup::Weapon,
        18,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        105,
        HiddenTreasureTarget::Town(30),
        1,
        17,
        22,
        HiddenTreasurePickup::Amulet,
        46,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        106,
        HiddenTreasureTarget::Town(31),
        0,
        3,
        6,
        HiddenTreasurePickup::Potion,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        107,
        HiddenTreasureTarget::Town(31),
        0,
        7,
        19,
        HiddenTreasurePickup::Food,
        20,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        108,
        HiddenTreasureTarget::Town(32),
        1,
        21,
        8,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        109,
        HiddenTreasureTarget::Town(1),
        1,
        24,
        19,
        HiddenTreasurePickup::Food,
        16,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        110,
        HiddenTreasureTarget::Town(1),
        1,
        24,
        20,
        HiddenTreasurePickup::Food,
        13,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        111,
        HiddenTreasureTarget::Town(17),
        2,
        12,
        12,
        HiddenTreasurePickup::Potion,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        112,
        HiddenTreasureTarget::Town(17),
        2,
        12,
        12,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
];

pub fn town_search_live_tile_miss_message(tile: u8) -> Option<&'static str> {
    match tile {
        0x2b => Some("Searched stump; nothing found."),
        0x4f => Some("Searched wall; nothing found."),
        0x5a => Some("Searched shelf; nothing found."),
        0x5c | 0x5d => Some("Searched bookshelf; nothing found."),
        0xa1 => Some("Searched well; nothing found."),
        0xa5 => Some("Searched desk; nothing found."),
        0xa6 => Some("Searched barrel; nothing found."),
        0xa8 => Some("Searched vanity; nothing found."),
        0xab | 0xac => Some("Searched under bed; nothing found."),
        0xad => Some("Searched dresser; nothing found."),
        0xaf => Some("Searched trunk; nothing found."),
        0xb2 => Some("Searched brazier; nothing found."),
        0xbc => Some("Searched fireplace; nothing found."),
        _ => None,
    }
}
