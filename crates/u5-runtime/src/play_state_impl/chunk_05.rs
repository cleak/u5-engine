use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    pub fn step_dungeon(
        &mut self,
        direction: Direction,
        nx: isize,
        ny: isize,
        scene: DungeonScene,
        level: u8,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        if !direction.is_cardinal() {
            self.message =
                "Dungeon debug movement uses cardinal steps only in this slice.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let nx = nx.rem_euclid(DUNGEON_SIDE as isize) as usize;
        let ny = ny.rem_euclid(DUNGEON_SIDE as isize) as usize;
        if self.dungeon_active_monster_at(nx, ny).is_some() {
            self.message = "Blocked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let tile = self.dungeon_cell(level, nx, ny);
        if self.dungeon_open_door_at(game_dir, scene, level, nx, ny, tile)? {
            self.player.x = nx;
            self.player.y = ny;
            self.sync_player_object();
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = format!(
                "Moved {} to ({nx}, {ny}) on {} level {level}; passed through open dungeon door.",
                direction.name(),
                scene.key()
            );
            return Ok(MoveOutcome::Moved);
        }
        if self.dungeon_closed_door_at(game_dir, scene, level, nx, ny, tile)? {
            self.message = "Blocked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if is_dungeon_room_trigger(tile) {
            self.player.x = nx;
            self.player.y = ny;
            self.sync_player_object();
            self.mark_visibility_dirty();
            return self.resolve_dungeon_room_trigger(game_dir, scene, level, nx, ny, tile);
        }
        if let Some(entry) = self.dungeon_teleport_at(game_dir, scene, level, nx, ny, tile)? {
            return Ok(self.apply_dungeon_teleport(scene, entry, direction));
        }
        if self.dungeon_exit_tile_at(game_dir, scene, level, nx, ny, tile)? {
            self.player.x = nx;
            self.player.y = ny;
            self.sync_player_object();
            self.mark_visibility_dirty();
            return self.resolve_dungeon_exit_tile(game_dir, scene, level);
        }
        if !is_dungeon_walkable(tile) {
            self.message = "Blocked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.player.x = nx;
        self.player.y = ny;
        self.sync_player_object();
        self.mark_visibility_dirty();
        if is_dungeon_fall_trap(tile) {
            return self.resolve_dungeon_fall_trap(scene, level, nx, ny, game_dir);
        }
        if is_dungeon_bomb_trap(tile) {
            self.grid[dungeon_cell_index(level, nx, ny)] |= 0x08;
            self.advance_turn();
            self.message = format!(
                "Moved {} to ({nx}, {ny}) on {} level {level}; triggered bomb trap.",
                direction.name(),
                scene.key()
            );
            return Ok(MoveOutcome::Moved);
        }
        if let Some(field) = dungeon_field_effect(tile) {
            let field_report = self.apply_dungeon_field_effect(field);
            self.advance_turn();
            self.message = format!(
                "Moved {} to ({nx}, {ny}) on {} level {level}; triggered {}; {field_report}.",
                direction.name(),
                scene.key(),
                field.label()
            );
            return Ok(MoveOutcome::Moved);
        }
        if is_dungeon_room_helper_state(tile) {
            return self.resolve_dungeon_room_trigger(game_dir, scene, level, nx, ny, tile);
        }
        if self.dungeon_wind_tile_extinguishes_torch(game_dir, scene, level, nx, ny, tile)? {
            self.torch_counter = 0;
            self.advance_turn();
            self.message = format!(
                "Moved {} to ({nx}, {ny}) on {} level {level}; a breeze blows out the torch.",
                direction.name(),
                scene.key()
            );
            return Ok(MoveOutcome::Moved);
        }
        self.advance_turn();
        self.message = format!(
            "Moved {} to ({nx}, {ny}) on {} level {level}; underfoot {}.",
            direction.name(),
            scene.key(),
            dungeon_cell_class(tile)
        );
        Ok(MoveOutcome::Moved)
    }

    pub fn dungeon_teleport_at(
        &self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        cell: u8,
    ) -> io::Result<Option<DungeonTeleportEntry>> {
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        Ok(
            load_dungeon_teleport_entries(game_dir)?.and_then(|entries| {
                entries
                    .into_iter()
                    .find(|entry| dungeon_teleport_matches(*entry, scene, level, x, y, cell))
            }),
        )
    }

    pub fn apply_dungeon_teleport(
        &mut self,
        scene: DungeonScene,
        entry: DungeonTeleportEntry,
        direction: Direction,
    ) -> MoveOutcome {
        self.apply_dungeon_teleport_transition(scene, entry, Some(direction), true)
    }

    pub fn apply_dungeon_teleport_after_turn(
        &mut self,
        scene: DungeonScene,
        entry: DungeonTeleportEntry,
    ) -> MoveOutcome {
        self.apply_dungeon_teleport_transition(scene, entry, None, false)
    }

    pub fn apply_dungeon_teleport_transition(
        &mut self,
        scene: DungeonScene,
        entry: DungeonTeleportEntry,
        direction: Option<Direction>,
        advance_turn: bool,
    ) -> MoveOutcome {
        self.area = Area::Dungeon {
            scene,
            level: entry.to_level,
        };
        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        self.sync_player_object();
        self.mark_visibility_dirty();
        if advance_turn {
            self.advance_turn();
        }
        self.message = if let Some(direction) = direction {
            format!(
                "Moved {} onto scripted dungeon teleport at ({}, {}) on {} level {}; changed to level {} at ({}, {}).",
                direction.name(),
                entry.x,
                entry.y,
                scene.key(),
                entry.level,
                entry.to_level,
                entry.to_x,
                entry.to_y
            )
        } else {
            format!(
                "Triggered scripted dungeon teleport at ({}, {}) on {} level {}; changed to level {} at ({}, {}).",
                entry.x,
                entry.y,
                scene.key(),
                entry.level,
                entry.to_level,
                entry.to_x,
                entry.to_y
            )
        };
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel {
            scene,
            level: entry.to_level,
        })
    }

    pub fn dungeon_wind_tile_extinguishes_torch(
        &self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        cell: u8,
    ) -> io::Result<bool> {
        let Some(game_dir) = game_dir else {
            return Ok(false);
        };
        if self.torch_counter == 0 {
            return Ok(false);
        }
        Ok(load_dungeon_wind_tile_entries(game_dir)?
            .unwrap_or_default()
            .into_iter()
            .any(|entry| dungeon_wind_tile_matches(entry, scene, level, x, y, cell)))
    }

    pub fn dungeon_exit_tile_at(
        &self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        cell: u8,
    ) -> io::Result<bool> {
        let Some(game_dir) = game_dir else {
            return Ok(false);
        };
        Ok(load_dungeon_exit_tile_entries(game_dir)?
            .unwrap_or_default()
            .into_iter()
            .any(|entry| dungeon_exit_tile_matches(entry, scene, level, x, y, cell)))
    }

    pub fn apply_dungeon_field_effect(&mut self, field: DungeonFieldEffect) -> String {
        if let Some(status) = field.status() {
            let mut affected = 0;
            for member in &mut self.party {
                if member.living() {
                    member.status = status;
                    affected += 1;
                }
            }
            return format!(
                "set {} living member(s) to {}",
                affected,
                party_status_name(status)
            );
        }

        if field.is_damage_field() {
            let mut reports = Vec::new();
            for index in 0..self.party.len() {
                if !self.party[index].living() {
                    continue;
                }
                let damage = self.dungeon_field_damage_roll(index, field);
                let slot = self.party[index].slot;
                let applied = self.party[index].apply_damage(damage);
                reports.push(format!(
                    "slot {slot} took {applied} HP ({} HP left)",
                    self.party[index].hp
                ));
            }
            if reports.is_empty() {
                return "damage skipped for 0 living member(s)".to_string();
            }
            return reports.join("; ");
        }

        "generic energy field has no contact effect".to_string()
    }

    pub fn dungeon_field_damage_roll(&self, member_index: usize, field: DungeonFieldEffect) -> u8 {
        1 + (self.dungeon_field_damage_seed(member_index, field) % 8)
    }

    pub fn dungeon_field_damage_seed(&self, member_index: usize, field: DungeonFieldEffect) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(7)
            ^ self.clock.minute.wrapping_mul(11)
            ^ (self.player.x as u8).wrapping_mul(13)
            ^ (self.player.y as u8).wrapping_mul(17)
            ^ (member_index as u8).wrapping_mul(23)
            ^ field.damage_seed_bias()
    }

    pub fn dungeon_fountain_damage_roll(&self, member_index: usize, tile: u8) -> u8 {
        self.dungeon_fountain_damage_seed(member_index, tile) % 8
    }

    pub fn dungeon_fountain_damage_seed(&self, member_index: usize, tile: u8) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(5)
            ^ self.clock.minute.wrapping_mul(9)
            ^ (self.player.x as u8).wrapping_mul(13)
            ^ (self.player.y as u8).wrapping_mul(17)
            ^ (self.player.facing as u8).wrapping_mul(19)
            ^ (member_index as u8).wrapping_mul(23)
            ^ (tile & 0x0f).wrapping_mul(29)
    }

    pub fn resolve_dungeon_exit_tile(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        self.resolve_dungeon_exit_tile_transition(
            game_dir,
            scene,
            level,
            true,
            "Stepped onto dungeon exit tile",
        )
    }

    pub fn resolve_dungeon_exit_tile_after_turn(
        &mut self,
        game_dir: &Path,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        self.resolve_dungeon_exit_tile_transition(
            Some(game_dir),
            scene,
            level,
            false,
            "Triggered dungeon exit tile",
        )
    }

    pub fn resolve_dungeon_exit_tile_transition(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        advance_turn: bool,
        event: &str,
    ) -> io::Result<MoveOutcome> {
        if advance_turn {
            self.advance_turn();
        }
        let event = format!("{event} in {} ({})", scene.key(), scene.name());
        if self.restore_return_world() {
            self.message = format!("{event}; returned to overworld debug return point.");
            self.mark_visibility_dirty();
            return Ok(MoveOutcome::Transition(AreaTransition::ExitedDungeon(
                scene,
            )));
        } else if let Some(game_dir) = game_dir {
            if self.restore_world_for_target(game_dir, PlayTarget::Dungeon(scene))? {
                self.message = format!("{event}; returned to world-location table point.");
                self.mark_visibility_dirty();
                return Ok(MoveOutcome::Transition(AreaTransition::ExitedDungeon(
                    scene,
                )));
            }
        }
        Ok(self.block_missing_dungeon_return(scene, level, event))
    }

    pub fn block_missing_dungeon_return(
        &mut self,
        scene: DungeonScene,
        level: u8,
        event: String,
    ) -> MoveOutcome {
        self.area = Area::Dungeon { scene, level };
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.message =
            format!("{event}; missing clean return-coordinate metadata, stayed in dungeon.");
        MoveOutcome::Blocked
    }

    pub fn dungeon_door_entry_at(
        &self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
    ) -> io::Result<Option<DungeonDoorEntry>> {
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        Ok(load_dungeon_door_entries(game_dir)?.and_then(|entries| {
            entries.into_iter().find(|entry| {
                entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
            })
        }))
    }

    pub fn dungeon_closed_door_at(
        &self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        cell: u8,
    ) -> io::Result<bool> {
        Ok(self
            .dungeon_door_entry_at(game_dir, scene, level, x, y)?
            .is_some_and(|entry| dungeon_closed_door_matches(entry, cell)))
    }

    pub fn dungeon_open_door_at(
        &self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        cell: u8,
    ) -> io::Result<bool> {
        Ok(self
            .dungeon_door_entry_at(game_dir, scene, level, x, y)?
            .is_some_and(|entry| entry.open_cell == cell))
    }

    pub fn resolve_current_dungeon_room_trigger(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<Option<MoveOutcome>> {
        let Area::Dungeon { scene, level } = self.area else {
            return Ok(None);
        };
        let tile = self.dungeon_cell(level, self.player.x, self.player.y);
        if is_dungeon_room_trigger(tile)
            && self
                .dungeon_door_entry_at(game_dir, scene, level, self.player.x, self.player.y)?
                .is_some_and(|entry| {
                    entry.open_cell == tile || dungeon_closed_door_matches(entry, tile)
                })
        {
            return Ok(None);
        }
        Ok(
            if is_dungeon_room_trigger(tile) || is_dungeon_room_helper_state(tile) {
                Some(self.resolve_dungeon_room_trigger(
                    game_dir,
                    scene,
                    level,
                    self.player.x,
                    self.player.y,
                    tile,
                )?)
            } else {
                None
            },
        )
    }

    pub fn resolve_dungeon_room_trigger(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<MoveOutcome> {
        let slot = dungeon_room_slot(tile);
        let helper_state = is_dungeon_room_helper_state(tile);
        let doom_final_room = scene.record == DOOM_DUNGEON_RECORD
            && level == DOOM_FINAL_ROOM_LEVEL
            && x == DOOM_FINAL_ROOM_X
            && y == DOOM_FINAL_ROOM_Y
            && slot == DOOM_FINAL_ROOM_SLOT;
        let arena = dungeon_room_arena_index(scene, tile);
        let dungeon_cbt_available = game_dir.is_some_and(|dir| dir.join(DUNGEON_CBT_FILE).exists());
        if doom_final_room && !dungeon_cbt_available {
            return Ok(self.enter_endgame());
        }
        let marked_helper = !helper_state && !doom_final_room;
        if marked_helper {
            self.grid[dungeon_cell_index(level, x, y)] = 0xa0 | slot;
            set_dungeon_room_clear_bit(&mut self.dungeon_room_clear_bitmap, scene, slot);
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        let state_note = if doom_final_room {
            "kept final room trigger state"
        } else if marked_helper {
            "marked visit-local room-helper state"
        } else {
            "kept visit-local room-helper state"
        };
        let trigger_kind = if helper_state {
            "room-helper state"
        } else {
            "room trigger"
        };
        if dungeon_cbt_available {
            let game_dir = game_dir.expect("availability checked from game_dir");
            let combat_note = self.enter_dungeon_room_combat(game_dir, scene, level, arena)?;
            self.message = format!(
                "Entered dungeon {trigger_kind} slot {slot} at ({x}, {y}) on {} level {level}; {combat_note}; {state_note}.",
                scene.key()
            );
            return Ok(MoveOutcome::Moved);
        }
        let arena_note = self.dungeon_room_arena_note(game_dir, arena)?;
        self.message = format!(
            "Entered dungeon {trigger_kind} slot {slot} at ({x}, {y}) on {} level {level}; {arena_note}; {state_note}.",
            scene.key()
        );
        Ok(MoveOutcome::Moved)
    }

    pub fn dungeon_room_arena_note(
        &self,
        game_dir: Option<&Path>,
        arena: usize,
    ) -> io::Result<String> {
        let Some(game_dir) = game_dir else {
            return Ok(format!("selected DUNGEON.CBT arena {arena}"));
        };
        if !game_dir.join(DUNGEON_CBT_FILE).exists() {
            return Ok(format!("selected DUNGEON.CBT arena {arena}"));
        }
        let bank = load_dungeon_cbt(game_dir)?;
        let record = bank.record(arena).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("DUNGEON.CBT has no arena record {arena}"),
            )
        })?;
        let setup = dungeon_room_combat_setup_from_record(arena, record);
        let room_sources = record.dungeon_room_sources();
        let source_count = setup.setup_sources.len();
        let absorbable_count = setup
            .setup_sources
            .iter()
            .filter(|source| source.kind == DungeonRoomSetupSourceKind::AbsorbableField)
            .count();
        let first_source = room_sources[0];
        let terrain_origin = setup.terrain[0][0];
        let absorbable_note = if absorbable_count > 0 {
            format!(", {absorbable_count} absorbable-field marker(s)")
        } else {
            String::new()
        };
        Ok(format!(
            "loaded DUNGEON.CBT arena {arena} (terrain[0,0]=0x{terrain_origin:02X}, {source_count} room source marker(s){absorbable_note}, first source 0x{first_source:02X})"
        ))
    }

    pub fn tile_walkable(&self, tile: u8) -> bool {
        is_tile_walkable_for_transport(tile, self.passability.as_ref(), self.player.transport)
    }

    pub fn opened_town_door_key(
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
    ) -> (u8, i8, usize, usize) {
        (scene.byte, floor, x, y)
    }

    pub fn is_recorded_open_town_door(&self, scene: Scene, floor: i8, x: usize, y: usize) -> bool {
        let key = Self::opened_town_door_key(scene, floor, x, y);
        self.opened_town_doors.contains(&key)
    }

    pub fn record_open_town_door(&mut self, scene: Scene, floor: i8, x: usize, y: usize) {
        let key = Self::opened_town_door_key(scene, floor, x, y);
        if !self.opened_town_doors.contains(&key) {
            self.opened_town_doors.push(key);
        }
    }

    pub fn forget_open_town_door(&mut self, scene: Scene, floor: i8, x: usize, y: usize) {
        let key = Self::opened_town_door_key(scene, floor, x, y);
        self.opened_town_doors.retain(|entry| *entry != key);
    }

    pub fn is_revealed_town_secret_door(
        &self,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
    ) -> bool {
        let key = Self::opened_town_door_key(scene, floor, x, y);
        self.revealed_town_secret_doors.contains(&key)
    }

    pub fn record_revealed_town_secret_door(
        &mut self,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
    ) {
        let key = Self::opened_town_door_key(scene, floor, x, y);
        if !self.revealed_town_secret_doors.contains(&key) {
            self.revealed_town_secret_doors.push(key);
        }
    }

    pub fn forget_revealed_town_secret_door(
        &mut self,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
    ) {
        let key = Self::opened_town_door_key(scene, floor, x, y);
        self.revealed_town_secret_doors
            .retain(|entry| *entry != key);
    }

    pub fn clear_open_town_door_state(&mut self) {
        self.door_tracker = None;
        self.opened_town_doors.clear();
        self.revealed_town_secret_doors.clear();
    }

    pub fn clear_town_floor_reload_door_state(&mut self) {
        self.door_tracker = None;
        let revealed = self.revealed_town_secret_doors.clone();
        self.opened_town_doors
            .retain(|entry| revealed.contains(entry));
    }

    pub fn restore_revealed_town_secret_doors_for_floor(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<()> {
        let Some(entries) = load_secret_door_entries(game_dir)? else {
            return Ok(());
        };
        for entry in entries {
            let SecretDoorEntry::Town {
                scene: entry_scene,
                floor: entry_floor,
                x,
                y,
                reveal_tile,
                expected_tile: _,
            } = entry
            else {
                continue;
            };
            if entry_scene != scene
                || entry_floor != floor
                || !self.is_revealed_town_secret_door(scene, floor, x, y)
            {
                continue;
            }
            let idx = y * 32 + x;
            self.grid[idx] = if self.is_recorded_open_town_door(scene, floor, x, y) {
                16
            } else {
                reveal_tile
            };
        }
        Ok(())
    }

    pub fn resolve_dungeon_fall_trap(
        &mut self,
        scene: DungeonScene,
        start_level: u8,
        x: usize,
        y: usize,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.resolve_dungeon_fall_trap_transition(scene, start_level, x, y, game_dir, true)
    }

    pub fn resolve_dungeon_fall_trap_transition(
        &mut self,
        scene: DungeonScene,
        start_level: u8,
        x: usize,
        y: usize,
        game_dir: Option<&Path>,
        advance_turn: bool,
    ) -> io::Result<MoveOutcome> {
        let mut level = start_level;
        let mut drops = 0u8;
        loop {
            let current = dungeon_cell_index(level, x, y);
            if !is_dungeon_fall_trap(self.grid[current]) {
                break;
            }
            self.grid[current] &= !0x08;
            self.mark_visibility_dirty();
            drops += 1;
            let Some(next_level) = level.checked_add(1).filter(|next| *next <= 7) else {
                if advance_turn {
                    self.advance_turn();
                }
                if self.restore_return_world() {
                    self.message = format!(
                        "Fell out of {} ({}) to overworld debug return point.",
                        scene.key(),
                        scene.name()
                    );
                } else if let Some(game_dir) = game_dir {
                    if self.restore_world_for_target(game_dir, PlayTarget::Dungeon(scene))? {
                        self.message = format!(
                            "Fell out of {} ({}) to world-location table return point.",
                            scene.key(),
                            scene.name()
                        );
                    } else {
                        return Ok(self.block_missing_dungeon_return(
                            scene,
                            level,
                            format!("Fell out of {} ({})", scene.key(), scene.name()),
                        ));
                    }
                } else {
                    return Ok(self.block_missing_dungeon_return(
                        scene,
                        level,
                        format!("Fell out of {} ({})", scene.key(), scene.name()),
                    ));
                }
                self.mark_visibility_dirty();
                return Ok(MoveOutcome::Transition(AreaTransition::ExitedDungeon(
                    scene,
                )));
            };

            level = next_level;
            let destination = dungeon_cell_index(level, x, y);
            if self.grid[destination] < 0x90 {
                self.grid[destination] |= 0x08;
            }
        }

        self.area = Area::Dungeon { scene, level };
        self.sync_player_object();
        self.mark_visibility_dirty();
        if advance_turn {
            self.advance_turn();
        }
        self.message = format!(
            "Fell {drops} level(s) through pit trap to {} ({}) level {level}.",
            scene.key(),
            scene.name()
        );
        Ok(MoveOutcome::Transition(
            AreaTransition::ChangedDungeonLevel { scene, level },
        ))
    }

    pub fn step_world(
        &mut self,
        mut direction: Direction,
        mut nx: isize,
        mut ny: isize,
        plane: WorldPlane,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.pending_moongate = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        if let Some(outcome) = self.resolve_balloon_wind_step(&mut direction, &mut nx, &mut ny) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.resolve_sailed_ship_wind_gate(direction) {
            return Ok(outcome);
        }

        let nx = nx.rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = ny.rem_euclid(WORLD_SIDE as isize) as usize;
        let tile = self.grid[world_cell_index(nx, ny)];
        let moongate = self.moongate_at(plane, nx, ny);
        let mut transition = if let Some(game_dir) = game_dir {
            self.world_plane_transition_at(game_dir, plane, nx, ny)?
        } else {
            None
        };
        let damage_tile = if transition.is_none() && moongate.is_none() {
            if let Some(game_dir) = game_dir {
                self.world_damage_tile_at(game_dir, plane, nx, ny, tile)?
            } else {
                None
            }
        } else {
            None
        };
        let first_waterfall =
            if transition.is_none() && moongate.is_none() && !self.player.transport.is_balloon() {
                if let Some(game_dir) = game_dir {
                    self.world_waterfall_at(game_dir, plane, nx, ny, tile)?
                } else {
                    None
                }
            } else {
                None
            };
        if transition.is_none() && moongate.is_none() {
            if let Some(entry) = damage_tile {
                if !entry.effect.allows_transport(self.player.transport) {
                    self.message = format!("Blocked by {} at ({nx}, {ny}).", entry.effect.label());
                    return Ok(MoveOutcome::Blocked);
                }
            } else if first_waterfall.is_none() && !self.tile_walkable(tile) {
                self.message = format!("Blocked by {} at ({nx}, {ny}).", tile_class(tile));
                return Ok(MoveOutcome::Blocked);
            }
        }
        if let Some((object_slot, object)) = self
            .world_object_slot_at(nx, ny)
            .map(|(slot, object)| (slot, *object))
        {
            if let Some(game_dir) = game_dir {
                if game_dir.join(BRIT_CBT_FILE).exists()
                    && outdoor_combat_arena_index_for_object(object).is_some()
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
                        "Moved into world object tile {} at ({nx}, {ny}) in slot {object_slot}; {note}.",
                        object.tile
                    );
                    return Ok(MoveOutcome::Used);
                }
            }
            let note = self.terrain_encounter_note(game_dir, plane, object)?;
            self.message = format!(
                "Blocked by world object tile {} at ({nx}, {ny}) in slot {object_slot}; {note}.",
                object.tile
            );
            return Ok(MoveOutcome::Blocked);
        }

        let mut final_x = nx;
        let mut final_y = ny;
        let mut final_tile = tile;
        let mut final_moongate = moongate;
        let mut stride_cells = 1;
        if transition.is_none()
            && moongate.is_none()
            && first_waterfall.is_none()
            && self.player.transport.is_horse()
            && is_horse_fast_stride_tile(tile)
        {
            let (dx, dy) = direction.delta();
            let sx = (nx as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
            let sy = (ny as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
            let second_tile = self.grid[world_cell_index(sx, sy)];
            let second_moongate = self.moongate_at(plane, sx, sy);
            let second_transition = if let Some(game_dir) = game_dir {
                self.world_plane_transition_at(game_dir, plane, sx, sy)?
            } else {
                None
            };
            let second_damage_tile = if second_transition.is_none() && second_moongate.is_none() {
                if let Some(game_dir) = game_dir {
                    self.world_damage_tile_at(game_dir, plane, sx, sy, second_tile)?
                } else {
                    None
                }
            } else {
                None
            };
            let second_waterfall = if second_transition.is_none()
                && second_moongate.is_none()
                && second_damage_tile.is_none()
            {
                if let Some(game_dir) = game_dir {
                    self.world_waterfall_at(game_dir, plane, sx, sy, second_tile)?
                } else {
                    None
                }
            } else {
                None
            };
            if self.world_object_at(sx, sy).is_none()
                && (second_transition.is_some()
                    || second_moongate.is_some()
                    || second_waterfall.is_some()
                    || (second_damage_tile.is_none()
                        && is_horse_fast_stride_tile(second_tile)
                        && self.tile_walkable(second_tile)))
            {
                final_x = sx;
                final_y = sy;
                final_tile = second_tile;
                final_moongate = second_moongate;
                stride_cells = 2;
                transition = second_transition;
            }
        }

        self.player.x = final_x;
        self.player.y = final_y;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        if let Some(game_dir) = game_dir {
            if let Some(entry) = transition {
                self.apply_world_plane_transition(game_dir, entry)?;
                return Ok(MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                    from: plane,
                    to: entry.to_plane,
                }));
            }
        }
        let verb = if stride_cells == 2 { "Rode" } else { "Moved" };
        let waterfall = if final_moongate.is_none() && !self.player.transport.is_balloon() {
            if let Some(game_dir) = game_dir {
                self.world_waterfall_at(game_dir, plane, final_x, final_y, final_tile)?
            } else {
                None
            }
        } else {
            None
        };
        if let Some(entry) = waterfall {
            let game_dir = game_dir.expect("waterfall entries require a game directory");
            let sweep_prefix = |swept_steps: u8, x: usize, y: usize| {
                format!(
                    "{verb} {} to ({final_x}, {final_y}) on {}; waterfall swept party {swept_steps} step(s) {} to ({}, {}).",
                    direction.name(),
                    plane.key(),
                    entry.direction.name(),
                    x,
                    y
                )
            };
            match self.apply_world_waterfall_sweep(game_dir, plane, entry)? {
                WorldWaterfallSweep::Settled { steps } => {
                    self.message = sweep_prefix(steps, self.player.x, self.player.y);
                    self.append_world_damage_tile_message(Some(game_dir), plane)?;
                }
                WorldWaterfallSweep::PlaneTransition {
                    steps,
                    entry: transition,
                } => {
                    let to_plane = transition.to_plane;
                    let swept_x = self.player.x;
                    let swept_y = self.player.y;
                    self.apply_world_plane_transition(game_dir, transition)?;
                    let transition_message = self.message.clone();
                    self.message = format!(
                        "{} {}",
                        sweep_prefix(steps, swept_x, swept_y),
                        transition_message
                    );
                    return Ok(MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                        from: plane,
                        to: to_plane,
                    }));
                }
                WorldWaterfallSweep::Moongate { steps, entry } => {
                    self.pending_moongate = Some(entry);
                    self.message = format!(
                        "{} moongate! Enter? (Y/N).",
                        sweep_prefix(steps, self.player.x, self.player.y)
                    );
                }
            }
        } else if let Some(entry) = final_moongate {
            self.pending_moongate = Some(entry);
            self.message = format!(
                "{verb} {} to ({final_x}, {final_y}) on {}; moongate! Enter? (Y/N).",
                direction.name(),
                plane.key()
            );
        } else {
            self.message = format!(
                "{verb} {} to ({final_x}, {final_y}) on {}; underfoot {}.",
                direction.name(),
                plane.key(),
                tile_class(final_tile)
            );
            self.apply_fixed_narrative_gate_branch(plane);
            self.append_world_damage_tile_message(game_dir, plane)?;
        }
        Ok(MoveOutcome::Moved)
    }
}
