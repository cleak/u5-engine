use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

impl PlayState {
    pub fn climb_outdoors(&mut self, game_dir: &Path, plane: WorldPlane) -> io::Result<MoveOutcome> {
        if self.climbing_gear == 0 {
            self.message = "With what?".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if !self.player.transport.is_foot() {
            self.message = "On foot!".to_string();
            return Ok(MoveOutcome::Blocked);
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
            let roll = self.outdoor_climb_stat_roll(index);
            if self.party[index].climb_stat < roll {
                let damage = self.outdoor_climb_damage_roll(index);
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

    pub fn outdoor_climb_stat_roll(&self, member_index: usize) -> u8 {
        1 + (self.outdoor_climb_roll_seed(member_index) % 30)
    }

    pub fn outdoor_climb_damage_roll(&self, member_index: usize) -> u8 {
        1 + (self.outdoor_climb_roll_seed(member_index).wrapping_add(17) % 5)
    }

    pub fn outdoor_climb_roll_seed(&self, member_index: usize) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (self.player.x as u8).wrapping_mul(7)
            ^ (self.player.y as u8).wrapping_mul(11)
            ^ (member_index as u8).wrapping_mul(13)
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
            self.advance_turn();
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

        self.advance_turn();
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

    pub fn open_facing_with_game_dir(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        let (scene, floor) = match self.area {
            Area::Town { scene, floor } => (scene, floor),
            Area::Dungeon { scene, level } => {
                return self.open_dungeon_underfoot(game_dir, scene, level);
            }
            Area::World { .. } => {
                self.message = "Nothing to open here.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        };
        self.tick_door_tracker();
        let (dx, dy) = self.player.facing.delta();
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
        let chest_entries = game_dir
            .map(load_dungeon_chest_content_entries)
            .transpose()?
            .flatten();
        match tile >> 4 {
            0x4 => Ok(self.consume_dungeon_chest(
                chest_entries.as_deref(),
                scene,
                level,
                self.player.x,
                self.player.y,
                idx,
                tile,
                "Opened",
            )),
            0xF => {
                let Some(entry) = self.dungeon_door_entry_at(
                    game_dir,
                    scene,
                    level,
                    self.player.x,
                    self.player.y,
                )?
                else {
                    self.message =
                        "Dungeon heavy-door and room-trigger subtypes are still open in the public spec for this slice."
                            .to_string();
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

    #[cfg(test)]
    pub fn jimmy_facing(&mut self) -> MoveOutcome {
        self.jimmy_facing_with_game_dir(None)
            .expect("jimmy without a game dir cannot load sidecar metadata")
    }

    pub fn jimmy_facing_with_game_dir(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        if self.keys == 0 {
            self.message = "No keys!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        match self.area {
            Area::Town { scene, floor } => self.jimmy_town_facing(game_dir, scene, floor),
            Area::Dungeon { scene, level } => self.jimmy_dungeon_underfoot(game_dir, scene, level),
            Area::World { .. } => {
                self.message = "No lock!".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn jimmy_town_facing(
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
        let tx = tx as usize;
        let ty = ty as usize;
        if let Some(object) = self.blocking_object_at(tx, ty).copied() {
            self.advance_turn();
            self.message = format!(
                "Jimmy checked NPC/object tile {} at ({tx}, {ty}); pickpocket rewards are out of scope in this slice.",
                object.tile
            );
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
                self.message = "Magic lock!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            self.grid[idx] = entry.unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if (96..=103).contains(&tile) {
            self.advance_turn();
            self.message = format!(
                "Jimmy checked door tile {tile} at ({tx}, {ty}); lock-state table and pick roll are out of scope in this slice."
            );
            return Ok(MoveOutcome::LockTried);
        }
        self.message = "No lock!".to_string();
        Ok(MoveOutcome::Blocked)
    }

    pub fn jimmy_dungeon_underfoot(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        Ok(match tile >> 4 {
            0x4 => {
                self.advance_turn();
                self.message = format!(
                    "Jimmy checked dungeon chest at ({}, {}) on {} level {level}; key break and content rolls are out of scope in this slice.",
                    self.player.x,
                    self.player.y,
                    scene.key()
                );
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
                    self.advance_turn();
                    self.message = format!(
                        "Jimmy checked dungeon door at ({}, {}) on {} level {level}; lock-state low-nibble and pick roll are out of scope in this slice.",
                        self.player.x,
                        self.player.y,
                        scene.key()
                    );
                    return Ok(MoveOutcome::LockTried);
                };
                if entry.open_cell == tile {
                    self.advance_turn();
                    self.message = "It's open!".to_string();
                    return Ok(MoveOutcome::LockTried);
                }
                if !dungeon_closed_door_matches(entry, tile) {
                    self.message =
                        "Dungeon door sidecar did not match the current cell byte.".to_string();
                    return Ok(MoveOutcome::Blocked);
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
            0x4 => self.consume_dungeon_chest(
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

        self.free_active_object_slot(slot);
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

    pub fn apply_object_pickup(&mut self, kind: ObjectPickupKind, amount: u8) {
        match kind {
            ObjectPickupKind::Food => self.food = self.food.saturating_add(u16::from(amount)),
            ObjectPickupKind::Gold => self.gold = self.gold.saturating_add(u16::from(amount)),
            ObjectPickupKind::Keys => self.keys = self.keys.saturating_add(amount),
            ObjectPickupKind::Gems => self.gems = self.gems.saturating_add(amount),
            ObjectPickupKind::Torches => self.torches = self.torches.saturating_add(amount),
        }
    }

    pub fn get_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        match self.area {
            Area::World { plane } => self.get_world_facing(game_dir, plane),
            Area::Town { scene, floor } => self.get_town_facing(game_dir, scene, floor),
            Area::Dungeon { scene, level } => {
                self.get_dungeon_underfoot_with_game_dir(Some(game_dir), scene, level)
            }
        }
    }

    pub fn get_world_facing(&mut self, game_dir: &Path, plane: WorldPlane) -> io::Result<MoveOutcome> {
        let (dx, dy) = self.player.facing.delta();
        let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if let Some(outcome) = self.get_moonstone_pickup_at(tx, ty) {
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
            self.apply_object_pickup(grant.kind, grant.amount);
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
        let (dx, dy) = self.player.facing.delta();
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
        if let Some(outcome) =
            self.get_object_pickup_at(game_dir, PlayTarget::Town(scene), floor, tx, ty)?
        {
            return Ok(outcome);
        }
        if self.blocking_object_at(tx, ty).is_some() {
            self.message = "Nothing to get there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
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
            self.apply_object_pickup(grant.kind, grant.amount);
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

    pub fn search_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let entries = load_secret_door_entries(game_dir)?.unwrap_or_default();
        let chest_entries = load_dungeon_chest_content_entries(game_dir)?;
        Ok(self.search_facing_secret(&entries, chest_entries.as_deref()))
    }

    pub fn search_facing_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
    ) -> MoveOutcome {
        match self.area {
            Area::Town { scene, floor } => self.search_town_secret(entries, scene, floor),
            Area::Dungeon { scene, level } => {
                self.search_dungeon_secret(entries, chest_entries, scene, level)
            }
            Area::World { plane } => self.search_world_moonstone(plane),
        }
    }

    pub fn search_world_moonstone(&mut self, plane: WorldPlane) -> MoveOutcome {
        let (dx, dy) = self.player.facing.delta();
        let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        self.search_moonstone_at(
            tx,
            ty,
            |slot| moonstone_slot_matches_world(slot, plane, tx, ty),
            "Nothing to search here.",
        )
    }

    pub fn search_town_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        scene: Scene,
        floor: i8,
    ) -> MoveOutcome {
        let (dx, dy) = self.player.facing.delta();
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
            return self.search_moonstone_at(
                tx,
                ty,
                |slot| moonstone_slot_matches_town(slot, scene, floor, tx, ty),
                "No secret door found.",
            );
        };
        if !(24..=63).contains(&tile) {
            return self.search_moonstone_at(
                tx,
                ty,
                |slot| moonstone_slot_matches_town(slot, scene, floor, tx, ty),
                "No secret door found.",
            );
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
            self.message = miss_message.to_string();
            return MoveOutcome::Blocked;
        };

        if self.moonstone_pickup_exists(slot_index) {
            self.message = format!(
                "Moonstone phase {} is already surfaced as a strange rock.",
                slot_index + 1
            );
            return MoveOutcome::Blocked;
        }

        let Some(z) = self.current_floor() else {
            self.message = miss_message.to_string();
            return MoveOutcome::Blocked;
        };
        let pickup = ActiveObject::moonstone_pickup(slot_index, x, y, z);
        if self.allocate_active_object_slot(pickup).is_none() {
            self.message = "No active-object slot for Moonstone pickup.".to_string();
            return MoveOutcome::Blocked;
        }

        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Found a strange rock for Moonstone phase {}.",
            slot_index + 1
        );
        MoveOutcome::Searched
    }

    pub fn search_dungeon_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
    ) -> MoveOutcome {
        let (dx, dy) = self.player.facing.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&tx) || !(0..DUNGEON_SIDE as isize).contains(&ty) {
            self.message = "Nothing to search there.".to_string();
            return MoveOutcome::Blocked;
        }
        let tx = tx as usize;
        let ty = ty as usize;
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
            0x4 => self.consume_dungeon_chest(
                chest_entries,
                scene,
                level,
                tx,
                ty,
                idx,
                tile,
                "Searched",
            ),
            _ if is_dungeon_bomb_trap(tile) => {
                self.grid[idx] = 0x6a;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = format!(
                    "Cleared dungeon bomb trap at ({tx}, {ty}) on {} level {level}.",
                    scene.key()
                );
                MoveOutcome::Searched
            }
            _ => self.search_dungeon_feature(scene, level, tx, ty, tile),
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
