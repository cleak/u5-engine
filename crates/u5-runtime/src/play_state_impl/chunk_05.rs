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
        self.step_dungeon_with_back_step_gate(direction, nx, ny, scene, level, game_dir, false)
    }

    pub fn step_dungeon_back(
        &mut self,
        direction: Direction,
        nx: isize,
        ny: isize,
        scene: DungeonScene,
        level: u8,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.step_dungeon_with_back_step_gate(direction, nx, ny, scene, level, game_dir, true)
    }

    fn step_dungeon_with_back_step_gate(
        &mut self,
        direction: Direction,
        nx: isize,
        ny: isize,
        scene: DungeonScene,
        level: u8,
        game_dir: Option<&Path>,
        back_step: bool,
    ) -> io::Result<MoveOutcome> {
        if !direction.is_cardinal() {
            self.message =
                "Dungeon debug movement uses cardinal steps only in this slice.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let origin_x = self.player.x;
        let origin_y = self.player.y;

        let nx = nx.rem_euclid(DUNGEON_SIDE as isize) as usize;
        let ny = ny.rem_euclid(DUNGEON_SIDE as isize) as usize;
        if self.dungeon_active_monster_at(nx, ny).is_some() {
            self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let tile = self.dungeon_cell(level, nx, ny);
        if back_step && dungeon_back_step_rejected(tile) {
            self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }
        // `dungeon-mode.md` Section 8.1, "Electric contact": it "prints
        // `Ouch!` then `Electric field!` **before** the destination-class
        // test, so those two lines precede any `Blocked!` the same step later
        // produces". The screen flash, rumble, one-cell push-back and
        // whole-party damage follow.
        if matches!(
            dungeon_field_effect(tile),
            Some(DungeonFieldEffect::Electric)
        ) {
            self.emit_message_line(DUNGEON_ELECTRIC_OUCH_LINE);
            self.emit_message_line(DUNGEON_ELECTRIC_FIELD_LINE);
            // Section 8: "electric contact first commits the requested
            // adjacent field cell for the flash presentation, then reverses
            // that exact one-cell displacement" back to the just-vacated
            // origin, with no obstruction test on the reversal.
            self.player.x = nx;
            self.player.y = ny;
            self.sync_player_object();
            self.mark_visibility_dirty();
            self.player.x = origin_x;
            self.player.y = origin_y;
            self.sync_player_object();
            self.mark_visibility_dirty();
            let _ = self.apply_dungeon_field_effect_at(
                level,
                nx,
                ny,
                tile,
                DungeonFieldEffect::Electric,
            );
            self.advance_turn();
            // Section 8.1: the two lines "precede any `Blocked!` the same
            // step later produces", so the destination-class test still runs
            // after them rather than being short-circuited. On shipped data
            // the field class `0x8` is always walkable, so this arm is a
            // published-ordering guard rather than a reachable path.
            if !is_dungeon_walkable(tile) {
                self.emit_message_line(MOVEMENT_BLOCKED_REFUSAL);
                return Ok(MoveOutcome::Blocked);
            }
            self.message = DUNGEON_ELECTRIC_FIELD_LINE.to_string();
            return Ok(MoveOutcome::Moved);
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
        if !is_dungeon_walkable(tile) {
            self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
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
            // `dungeon-mode.md` Section 8.1, bomb trap `0x62`/`0x6A`.
            self.emit_message_line(DUNGEON_BOMB_TRAP_LINE);
            self.message = DUNGEON_KABOOM_LINE.to_string();
            return Ok(MoveOutcome::Moved);
        }
        if let Some(field) = dungeon_field_effect(tile) {
            // Section 8.1: "Both field lines print **before** their per-member
            // rolls, so the line appears even when nobody is affected."
            let line = dungeon_field_consequence_line(field);
            if let Some(line) = line {
                self.emit_message_line(line);
            }
            let _ = self.apply_dungeon_field_effect_at(level, nx, ny, tile, field);
            self.advance_turn();
            // "Any other underfoot byte: nothing."
            self.message = line.unwrap_or_default().to_string();
            return Ok(MoveOutcome::Moved);
        }
        if is_dungeon_room_helper_state(tile) {
            return self.resolve_dungeon_room_trigger(game_dir, scene, level, nx, ny, tile);
        }
        self.advance_turn();
        // As above: the dungeon loop's own `Advance` echo (`commands.md
        // §5.2`) is the whole of an ordinary accepted step.
        self.message = String::new();
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
        let changed_level = entry.level != entry.to_level;
        self.area = Area::Dungeon {
            scene,
            level: entry.to_level,
        };
        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        self.sync_player_object();
        if changed_level {
            self.setup_dungeon_active_monster_fresh();
        }
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
                dungeon_display_level(entry.level),
                dungeon_display_level(entry.to_level),
                entry.to_x,
                entry.to_y
            )
        } else {
            format!(
                "Triggered scripted dungeon teleport at ({}, {}) on {} level {}; changed to level {} at ({}, {}).",
                entry.x,
                entry.y,
                scene.key(),
                dungeon_display_level(entry.level),
                dungeon_display_level(entry.to_level),
                entry.to_x,
                entry.to_y
            )
        };
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel {
            scene,
            level: entry.to_level,
        })
    }

    /// `systems/dungeon-mode.md` § 8 "Special cells in detail", Energy fields.
    ///
    /// Implemented here: a sleep field "rewrite[s] the live field cell to keep
    /// only its visit-marker bit" and marks the presentation dirty, so it is a
    /// one-shot contact hazard for the current visit, while a poison field
    /// "do[es] not rewrite the field cell, so standing on or re-entering the
    /// same poison field can trigger it again".
    ///
    /// Electric movement reversal is owned by the movement caller because it
    /// must restore the exact pre-attempt coordinate before this helper rolls
    /// damage. This helper owns the independently rolled `1..8` damage pass.
    pub fn apply_dungeon_field_effect_at(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
        field: DungeonFieldEffect,
    ) -> String {
        let report = self.apply_dungeon_field_effect(field);
        if field == DungeonFieldEffect::Sleep {
            self.grid[dungeon_cell_index(level, x, y)] = tile & DUNGEON_VISIT_MARKER_BIT;
            self.mark_visibility_dirty();
        }
        report
    }

    /// `systems/dungeon-mode.md` § 8 "Special cells in detail", Energy fields.
    ///
    /// Sleep and poison roll independently for each non-Dead active member in
    /// slot order. The inclusive `1..30` roll applies status on
    /// `roll >= current Dexterity`; equality therefore fails the save, while an
    /// unclamped Dexterity above 30 always saves.
    pub fn apply_dungeon_field_effect(&mut self, field: DungeonFieldEffect) -> String {
        if let Some(status) = field.status() {
            let mut affected = 0;
            for index in 0..self.party.len().min(SAVE_PARTY_SIZE_MAX as usize) {
                if self.party[index].status == b'D' {
                    continue;
                }
                let roll = self.random_range_u8(
                    DUNGEON_FIELD_STATUS_ROLL_LOW,
                    DUNGEON_FIELD_STATUS_ROLL_HIGH,
                );
                if dungeon_field_status_applies(self.party[index].climb_stat, roll) {
                    self.party[index].status = status;
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
            for index in 0..self.party.len().min(SAVE_PARTY_SIZE_MAX as usize) {
                if self.party[index].status == b'D' {
                    continue;
                }
                let damage = self.dungeon_field_damage_roll();
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

    pub fn dungeon_field_damage_roll(&mut self) -> u8 {
        self.random_range_u8(1, 8)
    }

    pub fn dungeon_fountain_damage_roll(&mut self) -> u8 {
        self.random_range_u8(0, 7)
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

    pub fn resolve_current_dungeon_room_trigger(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<Option<MoveOutcome>> {
        let Area::Dungeon { scene, level } = self.area else {
            return Ok(None);
        };
        let tile = self.dungeon_cell(level, self.player.x, self.player.y);
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
            return self.enter_endgame_from_game_dir(game_dir);
        }
        let marked_helper = !helper_state && !doom_final_room;
        if marked_helper {
            self.grid[dungeon_cell_index(level, x, y)] = 0xa0 | slot;
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        if dungeon_cbt_available {
            let game_dir = game_dir.expect("availability checked from game_dir");
            let entry_seed = dungeon_room_entry_seed_for_direction(self.player.facing);
            let _combat_note = self.enter_dungeon_room_combat(
                game_dir,
                scene,
                level,
                slot,
                arena,
                entry_seed,
                !helper_state,
                doom_final_room,
            )?;
            self.message = DUNGEON_ROOM_ENTRY_NARRATION.to_string();
            return Ok(MoveOutcome::Moved);
        }
        // Keep the asset/arena validation side effect even though the original
        // user-facing narration contains none of the clean harness diagnostics.
        let _arena_note = self.dungeon_room_arena_note(game_dir, arena)?;
        self.message = DUNGEON_ROOM_ENTRY_NARRATION.to_string();
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

    pub fn clear_town_visit_state(&mut self) {
        self.clear_open_town_door_state();
        self.active_blackthorn_guard_demand = None;
        self.town_drunkenness_counter = 0;
        self.tavern_secondary_drink_count = 0;
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
                TOWN_DOOR_CLEARED_TILE
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
            // `dungeon-mode.md` Section 8.1: the three-line pit group is
            // printed **once per descent step**, in this order, with the
            // level change and view repaint between `Falling...` and the
            // splat. "A two-deep fall chain prints the three-line pit group
            // twice."
            self.emit_message_line(DUNGEON_PIT_TRAP_LINE);
            self.emit_message_line(DUNGEON_FALLING_LINE);
            let Some(next_level) = level.checked_add(1).filter(|next| *next <= 7) else {
                return self.resolve_dungeon_fall_off_bottom(
                    scene,
                    level,
                    x,
                    y,
                    game_dir,
                    advance_turn,
                );
            };

            level = next_level;
            self.area = Area::Dungeon { scene, level };
            self.sync_player_object();
            self.setup_dungeon_active_monster_fresh();
            let destination = dungeon_cell_index(level, x, y);
            if self.grid[destination] < 0x90 {
                self.grid[destination] |= 0x08;
            }
            // "then the level change and view repaint, then
            // `      ...splat!` - **six leading spaces**".
            self.emit_message_line(DUNGEON_SPLAT_LINE);
        }

        self.area = Area::Dungeon { scene, level };
        self.sync_player_object();
        self.mark_visibility_dirty();
        if advance_turn {
            self.advance_turn();
        }
        // Section 8.1: "the arrival on the lower level narrates nothing of its
        // own beyond `      ...splat!`". The per-drop group above is the whole
        // of the chain's narration; the level/coordinate summary this used to
        // print has no counterpart in the original.
        let _ = drops;
        self.message = DUNGEON_SPLAT_LINE.to_string();
        Ok(MoveOutcome::Transition(
            AreaTransition::ChangedDungeonLevel { scene, level },
        ))
    }

    pub fn resolve_dungeon_fall_off_bottom(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        game_dir: Option<&Path>,
        advance_turn: bool,
    ) -> io::Result<MoveOutcome> {
        if advance_turn {
            self.advance_turn();
        }
        // `dungeon-mode.md` §13.2: the automatic fall-trap chain is the
        // defensive off-bottom exception. It keeps the trap column instead
        // of consulting the exterior-coordinate table, but an off-bottom
        // handoff still resumes on the Underworld map.
        let plane = WorldPlane::Underworld;
        if let Some(game_dir) = game_dir {
            self.restore_world_at(game_dir, plane, x, y)?;
            self.message = format!(
                "Fell out of {} ({}) past level {}; cleared dungeon scene at trap-chain coordinate ({x}, {y}) on {:?}.",
                scene.key(),
                scene.name(),
                level + 1,
                plane
            );
            return Ok(MoveOutcome::Transition(
                AreaTransition::ExitedDungeonToWorldPlane { scene, plane },
            ));
        }

        // The no-I/O test/debug route can reuse a cached Underworld snapshot.
        // A cached Britannia image cannot stand in for the required plane.
        if self
            .return_world
            .as_ref()
            .is_some_and(|return_world| return_world.plane == plane)
        {
            let return_world = self
                .return_world
                .take()
                .expect("matching cached return world was just observed");
            self.area = Area::World { plane };
            self.player.x = x;
            self.player.y = y;
            self.player.transport = TransportState::Foot;
            self.sail_cadence = 0;
            self.sail_stall_pending = false;
            self.grid = return_world.grid;
            self.natural_moongate_live_cells.clear();
            self.npcs.clear();
            self.active_objects = return_world.active_objects;
            self.sync_player_object();
            self.cache_current_world_overlay();
            self.clear_open_town_door_state();
            self.pending_town_arrest = None;
            self.active_blackthorn = None;
            self.mode_zero_cleanup();
            self.mark_visibility_dirty();
            self.message = format!(
                "Fell out of {} ({}) past level {}; cleared dungeon scene at trap-chain coordinate ({x}, {y}) on {:?}.",
                scene.key(),
                scene.name(),
                level + 1,
                plane
            );
            return Ok(MoveOutcome::Transition(
                AreaTransition::ExitedDungeonToWorldPlane { scene, plane },
            ));
        }

        Ok(self.block_missing_dungeon_return(
            scene,
            level,
            format!("Fell out of {} ({})", scene.key(), scene.name()),
        ))
    }

    pub fn step_world(
        &mut self,
        direction: Direction,
        nx: isize,
        ny: isize,
        plane: WorldPlane,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        // `movement.md §2`: overworld movement consumes the four cardinal
        // directions only — "No mode steps diagonally."
        if !direction.is_cardinal() {
            return Ok(MoveOutcome::Blocked);
        }
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        // `vehicles.md §11` "Balloon boundary": "Do not invent boarding,
        // landing, or wind-driven balloon movement." The wind-driven
        // balloon drift step that used to run here is removed with the
        // family; wind still gates the hoisted frigate below, which is the
        // only wind-cadenced movement §3 publishes.
        if let Some(outcome) = self.resolve_sailed_ship_wind_gate(direction) {
            return Ok(outcome);
        }
        let ship_under_sail = self.player.transport.is_ship_under_sail();
        let underfoot_blackout_latched = self.refresh_world_underfoot_blackout_latch();

        let nx = nx.rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = ny.rem_euclid(WORLD_SIDE as isize) as usize;
        let tile = self.grid[world_cell_index(nx, ny)];
        // `overworld.md §6.2.5`: exact pier terrain is a refused coordinate
        // step that silently docks the ship in place. It precedes occupancy
        // and passability rejection and consumes neither random stream.
        if ship_under_sail && tile == OVERWORLD_PIER_TILE {
            self.player.transport = self.player.transport.with_ship_sails_furled();
            self.sail_cadence = 0;
            self.sail_stall_pending = false;
            self.sync_player_object();
            self.mark_visibility_dirty();
            self.message = "Docked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let transition = if let Some(game_dir) = game_dir {
            self.world_plane_transition_at(game_dir, plane, nx, ny)?
        } else {
            None
        };
        let damage_tile = if transition.is_none() {
            if let Some(game_dir) = game_dir {
                self.world_damage_tile_at(game_dir, plane, nx, ny, tile)?
            } else {
                intrinsic_world_damage_tile_entry(plane, nx, ny, tile)
            }
        } else {
            None
        };
        if transition.is_none() {
            if let Some(entry) = damage_tile {
                if !entry.effect.allows_transport(self.player.transport) {
                    if ship_under_sail {
                        let _ = self.apply_sailing_collision(tile);
                        return Ok(MoveOutcome::Blocked);
                    }
                    self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
                    // `audio.md §7.4`: terrain impassable for the current
                    // transport is one of the two ways the overworld step is
                    // refused. No blocking object is involved, so the
                    // whirlpool arm cannot apply.
                    self.emit_overworld_blocked_step(false);
                    return Ok(MoveOutcome::Blocked);
                }
            } else if !self.tile_walkable(tile) && !(ship_under_sail && tile == OVERWORLD_PIER_TILE)
            {
                if ship_under_sail {
                    let _ = self.apply_sailing_collision(tile);
                    return Ok(MoveOutcome::Blocked);
                }
                self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
                // `audio.md §7.4`: the other overworld refusal, terrain
                // impassable for the current transport.
                self.emit_overworld_blocked_step(false);
                return Ok(MoveOutcome::Blocked);
            }
        }
        if let Some((object_slot, object)) = self
            .world_object_slot_at(nx, ny)
            .map(|(slot, object)| (slot, *object))
        {
            if ship_under_sail {
                let _ = self.apply_sailing_collision(tile);
                return Ok(MoveOutcome::Blocked);
            }
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
            let _note = self.terrain_encounter_note(game_dir, plane, object)?;
            // `audio.md §7.4`: refusal by a blocking object. Aboard a vehicle
            // a whirlpool-class blocker "returns completely silently, with no
            // message at all", so it is the one arm here that neither prints
            // nor beeps; every other arm prints the shared refusal.
            let whirlpool = is_whirlpool_object(object);
            let silent_whirlpool = whirlpool && !self.player.transport.is_foot();
            self.message = if silent_whirlpool {
                String::new()
            } else {
                MOVEMENT_BLOCKED_REFUSAL.to_string()
            };
            self.emit_overworld_blocked_step(whirlpool);
            return Ok(MoveOutcome::Blocked);
        }

        if underfoot_blackout_latched {
            self.advance_turn();
            self.message = format!(
                "Movement held by special underfoot tile at ({}, {}).",
                self.player.x, self.player.y
            );
            return Ok(MoveOutcome::Used);
        }

        let final_x = nx;
        let final_y = ny;
        let final_tile = tile;

        self.player.x = final_x;
        self.player.y = final_y;
        self.rebuild_world_live_chunks_from_grid(plane)?;
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
        // `text-output.md §10.2`/§10.3: an accepted step is complete at its
        // own direction echo - the mode loop has already drawn the prompt
        // marker and the direction word, and `commands.md §8.1` states of the
        // movement family that it "never prints tile ids, coordinates,
        // active-object slot numbers, terrain-class names". Only the
        // consequences below (damage, status, hourly report) produce a
        // result line.
        let _ = (final_tile, direction);
        self.message = String::new();
        self.apply_fixed_narrative_gate_branch(plane);
        self.append_world_damage_tile_message(game_dir, plane)?;
        self.append_world_status_tile_message(plane);
        self.append_pending_hourly_status_message();
        Ok(MoveOutcome::Moved)
    }
}
