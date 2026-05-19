use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    pub fn load_scene(game_dir: &Path, options: PlayOptions) -> io::Result<Self> {
        match options.target {
            PlayTarget::Town(scene) => Self::load_town_scene(game_dir, scene, options),
            PlayTarget::Dungeon(scene) => Self::load_dungeon_scene(game_dir, scene, options),
            PlayTarget::World(plane) => Self::load_world_scene(game_dir, plane, options),
        }
    }

    pub fn mark_visibility_dirty(&mut self) {
        self.visibility_dirty = true;
    }

    pub fn current_world_overlay_objects(&self) -> Vec<ActiveObject> {
        let mut objects = vec![ActiveObject::empty(); OOL_SLOTS - 1];
        for (index, object) in self
            .active_objects
            .iter()
            .skip(1)
            .take(OOL_SLOTS - 1)
            .copied()
            .enumerate()
        {
            objects[index] = object;
        }
        objects
    }

    pub fn cache_current_world_overlay(&mut self) {
        let Area::World { plane } = self.area else {
            return;
        };
        self.world_overlays
            .set(plane, self.current_world_overlay_objects());
    }

    pub fn save_game_command(
        &mut self,
        game_dir: &Path,
        confirm: Option<bool>,
    ) -> io::Result<MoveOutcome> {
        match confirm {
            None => {
                self.message = "Save game? Use QY to save or QN to cancel.".to_string();
                Ok(MoveOutcome::PromptDeclined)
            }
            Some(false) => {
                self.message = "No.".to_string();
                Ok(MoveOutcome::PromptDeclined)
            }
            Some(true) => {
                self.write_save_files(game_dir)?;
                self.message = "Yes. Saving... Done.".to_string();
                Ok(MoveOutcome::Saved)
            }
        }
    }

    pub fn write_save_files(&mut self, game_dir: &Path) -> io::Result<()> {
        let (scene, z, x, y) = self.current_save_location().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "save game is only available in active play modes",
            )
        })?;
        self.sync_player_object();
        self.cache_current_world_overlay();

        let mut save = load_save_image_template(game_dir, self.save_template_source)?;
        save[SAVE_SCENE_OFFSET] = scene;
        save[SAVE_Z_OFFSET] = z;
        save[SAVE_X_OFFSET] = x;
        save[SAVE_Y_OFFSET] = y;
        write_u16_at(&mut save, SAVE_YEAR_OFFSET, self.clock.year);
        save[SAVE_MONTH_OFFSET] = self.clock.month;
        save[SAVE_DAY_OFFSET] = self.clock.day;
        save[SAVE_HOUR_OFFSET] = self.clock.hour;
        save[SAVE_MINUTE_OFFSET] = self.clock.minute;
        save[SAVE_AMPM_DISPLAY_OFFSET] = self.clock.display_hour();
        write_u16_at(&mut save, SAVE_FOOD_STOCK_OFFSET, self.food);
        write_u16_at(&mut save, SAVE_GOLD_STOCK_OFFSET, self.gold);
        save[SAVE_KEY_STOCK_OFFSET] = self.keys;
        save[SAVE_GEM_STOCK_OFFSET] = self.gems;
        save[SAVE_TORCH_STOCK_OFFSET] = self.torches;
        save[SAVE_GRAPPLE_OFFSET] = self.climbing_gear;
        save[SAVE_SPECIAL_ITEM_OFFSET..SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_COUNT]
            .copy_from_slice(&self.special_items);
        save[SAVE_EQUIPMENT_STOCK_OFFSET..SAVE_EQUIPMENT_STOCK_OFFSET + EQUIPMENT_COUNT]
            .copy_from_slice(&self.equipment_stock);
        save[SAVE_SPELL_CHARGES_OFFSET..SAVE_SPELL_CHARGES_OFFSET + SPELL_COUNT]
            .copy_from_slice(&self.spell_charges);
        save[SAVE_SCROLL_STOCK_OFFSET..SAVE_SCROLL_STOCK_OFFSET + SCROLL_COUNT]
            .copy_from_slice(&self.scroll_stock);
        save[SAVE_POTION_STOCK_OFFSET..SAVE_POTION_STOCK_OFFSET + POTION_COUNT]
            .copy_from_slice(&self.potion_stock);
        encode_reagent_stock(&mut save, self.reagents);
        for (slot_index, slot) in self.moonstone_slots.iter().copied().enumerate() {
            save[SAVE_MOONSTONE_X_OFFSET + slot_index] = slot.x;
            save[SAVE_MOONSTONE_Y_OFFSET + slot_index] = slot.y;
            save[SAVE_MOONSTONE_SCENE_OFFSET + slot_index] = slot.scene;
            save[SAVE_MOONSTONE_Z_OFFSET + slot_index] = slot.z;
        }
        save[SAVE_LIGHT_SPELL_COUNTER_OFFSET] = self.light_spell_counter;
        save[SAVE_TORCH_COUNTER_OFFSET] = self.torch_counter;
        save[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = self.shrine_ordained_mask;
        save[SAVE_SHRINE_CODEX_MASK_OFFSET] = self.shrine_codex_mask;
        save[SAVE_MORAL_STANDING_OFFSET] = self.moral_standing;
        save[SAVE_TIMING_STATUS_TAG_OFFSET] = self.timing_status.save_byte();
        save[SAVE_FORTUNES_OF_WAR_OFFSET] = self.fortunes_of_war;
        save[SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET
            ..SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET + SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN]
            .copy_from_slice(&self.dungeon_room_clear_bitmap);
        if matches!(self.area, Area::Dungeon { .. })
            && self.grid.len() == SAVE_DUNGEON_WORKING_BUFFER_LEN
        {
            save[SAVE_DUNGEON_WORKING_BUFFER_OFFSET
                ..SAVE_DUNGEON_WORKING_BUFFER_OFFSET + SAVE_DUNGEON_WORKING_BUFFER_LEN]
                .copy_from_slice(&self.grid);
        }
        save[SAVE_ACTIVE_PLAYER_OFFSET] = encode_active_player_slot(self.active_player);
        save[SAVE_COMBAT_ROUND_COUNTER_OFFSET] = self.combat_round_counter;
        save[SAVE_TRANSPORT_MARKER_OFFSET] = if self.player.transport.is_foot() {
            self.player
                .transport
                .save_marker_with_facing(self.player.facing)
        } else {
            self.player.transport.save_marker()
        };
        save[SAVE_WIND_OFFSET] = self.wind_save_byte;
        let roster = self.synced_party_roster();
        save[SAVE_PARTY_SIZE_OFFSET] = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize) as u8;
        for (party_index, roster_record) in roster.iter().take(SAVE_ROSTER_SLOT_COUNT).enumerate() {
            let member = roster_record.member;
            let record = SAVE_ROSTER_OFFSET + party_index * SAVE_CHARACTER_RECORD_LEN;
            if record + SAVE_CHARACTER_MAX_HP_OFFSET + 1 >= save.len() {
                continue;
            }
            save[record..record + SAVE_CHARACTER_NAME_LEN].copy_from_slice(&roster_record.name);
            save[record + SAVE_CHARACTER_STR_OFFSET] = if party_index == 0 {
                self.avatar_stats.strength
            } else {
                roster_record.strength
            };
            save[record + SAVE_CHARACTER_DEX_OFFSET] = if party_index == 0 {
                self.avatar_stats.dexterity
            } else {
                member.climb_stat
            };
            save[record + SAVE_CHARACTER_INT_OFFSET] = if party_index == 0 {
                self.avatar_stats.intelligence
            } else {
                roster_record.intelligence
            };
            save[record + SAVE_CHARACTER_CLASS_OFFSET] = member.class_byte;
            save[record + SAVE_CHARACTER_STATUS_OFFSET] = member.status;
            save[record + SAVE_CHARACTER_MANA_OFFSET] = member.mana;
            write_u16_at(&mut save, record + SAVE_CHARACTER_HP_OFFSET, member.hp);
            write_u16_at(
                &mut save,
                record + SAVE_CHARACTER_MAX_HP_OFFSET,
                member.max_hp,
            );
            write_u16_at(
                &mut save,
                record + SAVE_CHARACTER_EXPERIENCE_OFFSET,
                roster_record.experience,
            );
            save[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET] =
                roster_record.stay_counter.min(INN_STAY_COUNTER_CAP);
            save[record + SAVE_CHARACTER_LEVEL_OFFSET] = member.level;
            let start = record + SAVE_CHARACTER_EQUIPMENT_OFFSET;
            save[start..start + EQUIPMENT_SLOT_COUNT].copy_from_slice(&roster_record.equipment);
        }
        encode_inn_registry(&mut save, &self.inn_registry);
        let active_table = encode_active_object_table(&self.active_objects)?;
        save[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN]
            .copy_from_slice(&active_table);

        let saved_ool = self.encode_saved_ool(game_dir)?;
        write_saved_ool_mirrors_for_save(game_dir, &saved_ool, 0)?;
        fs::write(game_dir.join(SAVED_GAM_FILENAME), save)?;
        fs::write(game_dir.join(SAVED_OOL_FILENAME), saved_ool)?;
        Ok(())
    }

    pub fn current_save_location(&self) -> Option<(u8, u8, u8, u8)> {
        let x = u8::try_from(self.player.x).ok()?;
        let y = u8::try_from(self.player.y).ok()?;
        match self.area {
            Area::Town { scene, floor } => Some((scene.byte, floor as u8, x, y)),
            Area::Dungeon { scene, level } => Some((scene.byte, level, x, y)),
            Area::World { plane } => {
                let z = match plane {
                    WorldPlane::Britannia => 0,
                    WorldPlane::Underworld => 0xff,
                };
                Some((0, z, x, y))
            }
        }
    }

    pub fn encode_saved_ool(&self, game_dir: &Path) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(SAVED_OOL_LEN);
        for plane in [WorldPlane::Britannia, WorldPlane::Underworld] {
            let objects = self.save_overlay_objects_for_plane(game_dir, plane)?;
            bytes.extend(encode_ool_plane_objects(&objects)?);
        }
        Ok(bytes)
    }

    pub fn save_overlay_objects_for_plane(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Vec<ActiveObject>> {
        if let Some(objects) = self.world_overlays.get(plane) {
            Ok(objects)
        } else {
            load_world_overlay_mirror_objects(game_dir, plane)
        }
    }

    pub fn load_world_overlay_for_plane(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Vec<ActiveObject>> {
        if let Some(objects) = self.world_overlays.get(plane) {
            Ok(objects)
        } else {
            load_world_overlay_objects(game_dir, plane)
        }
    }

    pub fn replace_world_active_objects(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<()> {
        let overlay = self.load_world_overlay_for_plane(game_dir, plane)?;
        self.active_objects.clear();
        self.active_objects.push(ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: plane.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        self.active_objects.extend(overlay);
        self.cache_current_world_overlay();
        Ok(())
    }

    pub fn load_town_scene(
        game_dir: &Path,
        scene: Scene,
        options: PlayOptions,
    ) -> io::Result<Self> {
        let mut grid = load_floor(game_dir, scene, options.floor)?;
        let passability = load_tile_passability(game_dir)?;
        let moongates = load_moongate_entries(game_dir)?.unwrap_or_default();
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        let markers = harvest_location_markers(&grid);
        normalize_town_runtime_floor(&mut grid, options.clock.hour);
        let table_start = if options.floor == 0 {
            load_location_entry_y(game_dir, scene)?
                .map(|entry_y| (LOCATION_DEFAULT_ENTRY_X, entry_y))
        } else {
            None
        };
        let saved_active_objects = options.saved_active_objects.clone();
        let has_saved_active_objects = saved_active_objects.is_some();
        let (x, y) = match options.start.or(table_start) {
            Some(pos) => {
                validate_start(&grid, pos, passability.as_ref())?;
                pos
            }
            None => markers
                .spawn_markers
                .first()
                .copied()
                .or_else(|| first_walkable(&grid, passability.as_ref()))
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "no playable start cell")
                })?,
        };

        let world_overlays = initial_world_overlay_cache(&options);
        let mut active_objects = vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: options.floor,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }];
        if let Some(objects) = saved_active_objects {
            active_objects.extend(objects);
        }
        let mut state = Self {
            area: Area::Town {
                scene,
                floor: options.floor,
            },
            player: Player {
                x,
                y,
                facing: options.facing.unwrap_or(Direction::South),
                transport: TransportState::Foot,
            },
            active_objects,
            npcs: Vec::new(),
            door_tracker: None,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            moongates,
            grid,
            clock: options.clock,
            animation: AnimationClock::default(),
            natural_moongate_counter: 0,
            natural_moongate_live_cells: Vec::new(),
            cached_moon_glyph_slots: [None, None],
            food: options.food,
            gold: options.gold,
            keys: options.keys,
            gems: options.gems,
            climbing_gear: options.climbing_gear,
            special_items: options.special_items,
            party: options.party,
            party_names: options.party_names,
            party_experience: options.party_experience,
            party_stay_counters: options.party_stay_counters,
            party_strengths: options.party_strengths,
            party_intelligence: options.party_intelligence,
            party_equipment: options.party_equipment,
            party_roster: options.party_roster,
            equipment_stock: options.equipment_stock,
            spell_charges: options.spell_charges,
            scroll_stock: options.scroll_stock,
            potion_stock: options.potion_stock,
            reagents: options.reagents,
            rare_reagent_harvest_days: options.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: options.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: options.fixed_hidden_treasure_daily_day,
            dungeon_room_clear_bitmap: options.dungeon_room_clear_bitmap,
            moonstone_slots: options.moonstone_slots,
            shadowlord_hideouts: options.shadowlord_hideouts,
            shrine_ordained_mask: options.shrine_ordained_mask,
            shrine_codex_mask: options.shrine_codex_mask,
            shrine_standing: options.shrine_standing,
            moral_standing: options.moral_standing,
            avatar_stats: options.avatar_stats,
            torches: options.torches,
            torch_counter: options.torch_counter,
            light_spell_counter: options.light_spell_counter,
            ambient_light: 0,
            visibility_dirty: false,
            wind: options.wind,
            wind_save_byte: options.wind_save_byte,
            timing_status: options.timing_status,
            time_stop_counter: options.time_stop_counter,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_active: false,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            pending_combat_terrain_trigger_slot: None,
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_stall_pending: false,
            turn: 0,
            message: format!("Entered {} at ({x}, {y}).", scene.key()),
            debug_enter: options.debug_enter,
            return_world: None,
            world_overlays,
            save_template_source: options.save_template_source,
            typeahead_buffer_enabled: false,
            music_enabled: true,
            pending_moongate: None,
            pending_town_arrest: None,
            endgame: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            blackthorn_jailed_party_slots: Vec::new(),
            active_shop: None,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_ready: None,
            active_use: None,
            active_cast: None,
            active_cast_followup: None,
            active_rest: None,
            active_jimmy: None,
            active_surface_chest: None,
            active_shrine: None,
            active_mix: None,
            active_new_order: None,
            active_yell: None,
            active_view_overlay: None,
            white_potion_sweep: None,
            combat_potion_presentation: None,
            active_direction_prompt: None,
            active_yes_no_prompt: None,
            pickpocketed_npcs: Vec::new(),
            removed_town_npcs: Vec::new(),
            town_npc_alarm_states: Vec::new(),
            talk_branch_flags: HashMap::new(),
            conversation_resource_signals: [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
            conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
            conversation_signal_bank_a: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
            conversation_signal_bank_b: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
            inn_registry: options.inn_registry,
        };
        if has_saved_active_objects {
            state.load_scheduled_npcs_from_existing_active_objects(&npc_slots);
        } else {
            state.load_scheduled_npcs(&npc_slots);
        }
        state.attach_player_phantom_npc();
        if !has_saved_active_objects {
            if let Some((slot, index)) = state.install_shadowlord_entry_encounter() {
                let shadowlord = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
                if !state.message.is_empty() {
                    state.message.push('\n');
                }
                state.message.push_str(&format!(
                    "Shadowlord entry: {shadowlord} appears in active-object slot {slot}."
                ));
            }
        }
        state.mode_zero_cleanup();
        state.mark_visibility_dirty();
        state.append_stonegate_entry_presentation_message();
        Ok(state)
    }

    pub fn load_dungeon_scene(
        game_dir: &Path,
        scene: DungeonScene,
        options: PlayOptions,
    ) -> io::Result<Self> {
        if !(0..=7).contains(&options.floor) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "dungeon levels are limited to 0..7 by the public spec, got {}",
                    options.floor
                ),
            ));
        }
        let grid = if let Some(buffer) = options.saved_dungeon_working_buffer.clone() {
            if buffer.len() != SAVE_DUNGEON_WORKING_BUFFER_LEN {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "saved dungeon working buffer must be {SAVE_DUNGEON_WORKING_BUFFER_LEN} bytes, got {}",
                        buffer.len()
                    ),
                ));
            }
            buffer
        } else {
            let mut grid = load_dungeon_record(game_dir, scene)?;
            apply_dungeon_room_clear_bitmap(&mut grid, scene, &options.dungeon_room_clear_bitmap);
            grid
        };
        let passability = load_tile_passability(game_dir)?;
        let moongates = load_moongate_entries(game_dir)?.unwrap_or_default();
        let level = options.floor as u8;
        let default_start = (1, 1);
        let saved_active_objects = options.saved_active_objects.clone();
        let (x, y) = match options.start {
            Some(pos) => {
                validate_dungeon_start(&grid, scene, level, pos)?;
                pos
            }
            None => {
                if validate_dungeon_start(&grid, scene, level, default_start).is_ok() {
                    default_start
                } else {
                    first_dungeon_walkable(&grid, level).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "no playable dungeon start cell")
                    })?
                }
            }
        };

        let world_overlays = initial_world_overlay_cache(&options);
        let mut active_objects = vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: level as i8,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }];
        if let Some(objects) = saved_active_objects {
            active_objects.extend(objects);
        }
        let mut state = Self {
            area: Area::Dungeon { scene, level },
            player: Player {
                x,
                y,
                facing: options.facing.unwrap_or(Direction::East),
                transport: TransportState::Foot,
            },
            active_objects,
            npcs: Vec::new(),
            door_tracker: None,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            moongates,
            grid,
            clock: options.clock,
            animation: AnimationClock::default(),
            natural_moongate_counter: 0,
            natural_moongate_live_cells: Vec::new(),
            cached_moon_glyph_slots: [None, None],
            food: options.food,
            gold: options.gold,
            keys: options.keys,
            gems: options.gems,
            climbing_gear: options.climbing_gear,
            special_items: options.special_items,
            party: options.party,
            party_names: options.party_names,
            party_experience: options.party_experience,
            party_stay_counters: options.party_stay_counters,
            party_strengths: options.party_strengths,
            party_intelligence: options.party_intelligence,
            party_equipment: options.party_equipment,
            party_roster: options.party_roster,
            equipment_stock: options.equipment_stock,
            spell_charges: options.spell_charges,
            scroll_stock: options.scroll_stock,
            potion_stock: options.potion_stock,
            reagents: options.reagents,
            rare_reagent_harvest_days: options.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: options.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: options.fixed_hidden_treasure_daily_day,
            dungeon_room_clear_bitmap: options.dungeon_room_clear_bitmap,
            moonstone_slots: options.moonstone_slots,
            shadowlord_hideouts: options.shadowlord_hideouts,
            shrine_ordained_mask: options.shrine_ordained_mask,
            shrine_codex_mask: options.shrine_codex_mask,
            shrine_standing: options.shrine_standing,
            moral_standing: options.moral_standing,
            avatar_stats: options.avatar_stats,
            torches: options.torches,
            torch_counter: options.torch_counter,
            light_spell_counter: options.light_spell_counter,
            ambient_light: 0,
            visibility_dirty: false,
            wind: options.wind,
            wind_save_byte: options.wind_save_byte,
            timing_status: options.timing_status,
            time_stop_counter: options.time_stop_counter,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_active: false,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            pending_combat_terrain_trigger_slot: None,
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_stall_pending: false,
            turn: 0,
            message: format!(
                "Entered {} ({}) level {level} at ({x}, {y}).",
                scene.key(),
                scene.name()
            ),
            debug_enter: options.debug_enter,
            return_world: None,
            world_overlays,
            save_template_source: options.save_template_source,
            typeahead_buffer_enabled: false,
            music_enabled: true,
            pending_moongate: None,
            pending_town_arrest: None,
            endgame: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            blackthorn_jailed_party_slots: Vec::new(),
            active_shop: None,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_ready: None,
            active_use: None,
            active_cast: None,
            active_cast_followup: None,
            active_rest: None,
            active_jimmy: None,
            active_surface_chest: None,
            active_shrine: None,
            active_mix: None,
            active_new_order: None,
            active_yell: None,
            active_view_overlay: None,
            white_potion_sweep: None,
            combat_potion_presentation: None,
            active_direction_prompt: None,
            active_yes_no_prompt: None,
            pickpocketed_npcs: Vec::new(),
            removed_town_npcs: Vec::new(),
            town_npc_alarm_states: Vec::new(),
            talk_branch_flags: HashMap::new(),
            conversation_resource_signals: [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
            conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
            conversation_signal_bank_a: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
            conversation_signal_bank_b: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
            inn_registry: options.inn_registry,
        };
        state.mode_zero_cleanup();
        state.mark_visibility_dirty();
        Ok(state)
    }

    pub fn load_world_scene(
        game_dir: &Path,
        plane: WorldPlane,
        options: PlayOptions,
    ) -> io::Result<Self> {
        let grid = load_world_map(game_dir, plane)?;
        let passability = load_tile_passability(game_dir)?;
        let moongates = load_moongate_entries(game_dir)?.unwrap_or_default();
        let damage_tiles = load_world_damage_tile_entries(game_dir)?.unwrap_or_default();
        // Canonical Ultima V starting position: Iolo's Hut on the surface
        // (Britannia), at the cell south of the dwelling entrance. For the
        // Underworld there is no canonical fresh-start spawn, so we keep
        // the safer (1,1) seed and fall back to a search if that is blocked.
        let default_start = match plane {
            WorldPlane::Britannia => (62, 124),
            WorldPlane::Underworld => (1, 1),
        };
        let (x, y) = match options.start {
            Some(pos) => {
                validate_world_start_for_transport(
                    &grid,
                    pos,
                    plane,
                    passability.as_ref(),
                    options.transport,
                    &damage_tiles,
                )?;
                pos
            }
            None => {
                if world_start_safe_for_transport(
                    &grid,
                    default_start,
                    plane,
                    passability.as_ref(),
                    options.transport,
                    &damage_tiles,
                ) {
                    default_start
                } else {
                    first_world_walkable_for_transport(
                        &grid,
                        plane,
                        passability.as_ref(),
                        options.transport,
                        &damage_tiles,
                    )
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "no playable world start cell")
                    })?
                }
            }
        };

        let transport = options.transport;
        let world_overlays = initial_world_overlay_cache(&options);
        let mut active_objects = vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: plane.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }];
        match options.saved_active_objects {
            Some(saved_objects) => active_objects.extend(saved_objects),
            None => {
                if plane == WorldPlane::Britannia {
                    if let Some(objects) = options.initial_britannia_overlay.clone() {
                        active_objects.extend(objects);
                    } else {
                        active_objects.extend(load_world_overlay_objects(game_dir, plane)?);
                    }
                } else {
                    active_objects.extend(load_world_overlay_objects(game_dir, plane)?);
                }
            }
        }
        if let Some(pending) = options.pending_vehicle {
            place_pending_vehicle_acquisition(&mut active_objects, plane, pending)?;
        }

        let mut state = Self {
            area: Area::World { plane },
            player: Player {
                x,
                y,
                facing: options.facing.unwrap_or(Direction::South),
                transport,
            },
            active_objects,
            npcs: Vec::new(),
            door_tracker: None,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            moongates,
            grid,
            clock: options.clock,
            animation: AnimationClock::default(),
            natural_moongate_counter: 0,
            natural_moongate_live_cells: Vec::new(),
            cached_moon_glyph_slots: [None, None],
            food: options.food,
            gold: options.gold,
            keys: options.keys,
            gems: options.gems,
            climbing_gear: options.climbing_gear,
            special_items: options.special_items,
            party: options.party,
            party_names: options.party_names,
            party_experience: options.party_experience,
            party_stay_counters: options.party_stay_counters,
            party_strengths: options.party_strengths,
            party_intelligence: options.party_intelligence,
            party_equipment: options.party_equipment,
            party_roster: options.party_roster,
            equipment_stock: options.equipment_stock,
            spell_charges: options.spell_charges,
            scroll_stock: options.scroll_stock,
            potion_stock: options.potion_stock,
            reagents: options.reagents,
            rare_reagent_harvest_days: options.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: options.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: options.fixed_hidden_treasure_daily_day,
            dungeon_room_clear_bitmap: options.dungeon_room_clear_bitmap,
            moonstone_slots: options.moonstone_slots,
            shadowlord_hideouts: options.shadowlord_hideouts,
            shrine_ordained_mask: options.shrine_ordained_mask,
            shrine_codex_mask: options.shrine_codex_mask,
            shrine_standing: options.shrine_standing,
            moral_standing: options.moral_standing,
            avatar_stats: options.avatar_stats,
            torches: options.torches,
            torch_counter: options.torch_counter,
            light_spell_counter: options.light_spell_counter,
            ambient_light: 0,
            visibility_dirty: false,
            wind: options.wind,
            wind_save_byte: options.wind_save_byte,
            timing_status: options.timing_status,
            time_stop_counter: options.time_stop_counter,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_active: false,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            pending_combat_terrain_trigger_slot: None,
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_stall_pending: false,
            turn: 0,
            message: format!(
                "Entered {} at ({x}, {y}). {}.",
                plane.key(),
                options.wind.status_message()
            ),
            debug_enter: options.debug_enter,
            return_world: None,
            world_overlays,
            save_template_source: options.save_template_source,
            typeahead_buffer_enabled: false,
            music_enabled: true,
            pending_moongate: None,
            pending_town_arrest: None,
            endgame: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            blackthorn_jailed_party_slots: Vec::new(),
            active_shop: None,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_ready: None,
            active_use: None,
            active_cast: None,
            active_cast_followup: None,
            active_rest: None,
            active_jimmy: None,
            active_surface_chest: None,
            active_shrine: None,
            active_mix: None,
            active_new_order: None,
            active_yell: None,
            active_view_overlay: None,
            white_potion_sweep: None,
            combat_potion_presentation: None,
            active_direction_prompt: None,
            active_yes_no_prompt: None,
            pickpocketed_npcs: Vec::new(),
            removed_town_npcs: Vec::new(),
            town_npc_alarm_states: Vec::new(),
            talk_branch_flags: HashMap::new(),
            conversation_resource_signals: [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
            conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
            conversation_signal_bank_a: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
            conversation_signal_bank_b: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
            inn_registry: options.inn_registry,
        };
        state.sync_player_object();
        state.cache_current_world_overlay();
        state.mode_zero_cleanup();
        state.mark_visibility_dirty();
        Ok(state)
    }

    #[cfg(test)]
    pub fn step(&mut self, direction: Direction) -> MoveOutcome {
        self.step_with_game_dir(direction, None)
            .expect("step without a game dir cannot perform file-backed transitions")
    }

    pub fn step_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.player.facing = direction;
        let previous_avatar_tile = self.player.transport.avatar_tile();
        self.player.transport = self.player.transport.with_facing(direction);
        if self.player.transport.avatar_tile() != previous_avatar_tile {
            self.sync_player_object();
            self.mark_visibility_dirty();
        }
        let (dx, dy) = direction.delta();
        let nx = self.player.x as isize + dx;
        let ny = self.player.y as isize + dy;
        let Area::Town { scene, floor } = self.area else {
            return self.step_non_town(direction, nx, ny, game_dir);
        };

        if !(0..32).contains(&nx) || !(0..32).contains(&ny) {
            self.advance_turn();
            if self.restore_return_world() {
                self.message = format!("Exited {} to overworld debug return point.", scene.key());
                self.mark_visibility_dirty();
                return Ok(MoveOutcome::Transition(AreaTransition::ExitedLocation(
                    scene,
                )));
            } else if let Some(game_dir) = game_dir {
                if self.restore_world_for_target(game_dir, PlayTarget::Town(scene))? {
                    self.message = format!(
                        "Exited {} to world-location table return point.",
                        scene.key()
                    );
                    self.mark_visibility_dirty();
                    return Ok(MoveOutcome::Transition(AreaTransition::ExitedLocation(
                        scene,
                    )));
                }
            }
            return Ok(self.block_missing_town_return(
                scene,
                floor,
                format!("Exited {}", scene.key()),
            ));
        }

        let nx = nx as usize;
        let ny = ny as usize;
        if self.blocking_object_at(nx, ny).is_some() {
            self.message = format!("Blocked by actor at ({nx}, {ny}).");
            return Ok(MoveOutcome::Blocked);
        }
        let tile = self.grid[ny * 32 + nx];
        if let Some(game_dir) = game_dir {
            if let Some(entry) = self.town_exit_tile_at(game_dir, scene, floor, nx, ny, tile)? {
                self.player.x = nx;
                self.player.y = ny;
                self.sync_player_object();
                self.mark_visibility_dirty();
                return self.resolve_town_exit_tile(game_dir, scene, floor, entry);
            }
        }
        if let Some(game_dir) = game_dir {
            if let Some(delta) = town_walk_on_stair_delta(tile, direction) {
                self.player.x = nx;
                self.player.y = ny;
                self.sync_player_object();
                self.mark_visibility_dirty();
                return self.change_town_floor(game_dir, scene, floor.saturating_add(delta));
            }
        }
        if let Some(game_dir) = game_dir {
            if self
                .town_stair_at(game_dir, scene, floor, nx, ny, tile)?
                .is_some()
                || (self.tile_walkable(tile) && (80..=87).contains(&tile))
            {
                return self.step_town_stair(game_dir, scene, floor, nx, ny, tile);
            }
        }
        if let Some(game_dir) = game_dir {
            if let Some(entry) = self.town_trap_door_at(game_dir, scene, floor, nx, ny, tile)? {
                self.player.x = nx;
                self.player.y = ny;
                self.sync_player_object();
                self.mark_visibility_dirty();
                return self.apply_town_trap_door(game_dir, scene, entry);
            }
        }
        if self.tile_walkable(tile) {
            self.player.x = nx;
            self.player.y = ny;
            self.sync_player_object();
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = format!("Moved to ({nx}, {ny}).");
            Ok(MoveOutcome::Moved)
        } else {
            self.message = format!("Blocked by {} at ({nx}, {ny}).", tile_class(tile));
            Ok(MoveOutcome::Blocked)
        }
    }

    pub fn town_exit_tile_at(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownExitTileEntry>> {
        if let Some(entry) = load_town_exit_tile_entries(game_dir)?.and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| town_exit_tile_matches(*entry, scene, floor, x, y, tile))
        }) {
            return Ok(Some(entry));
        }
        Ok(
            (tile == TOWN_EXIT_THRESHOLD_TILE).then_some(TownExitTileEntry {
                scene,
                floor,
                x,
                y,
                expected_tile: Some(TOWN_EXIT_THRESHOLD_TILE),
            }),
        )
    }

    pub fn town_lock_at(
        &self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownLockEntry>> {
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        Ok(load_town_lock_entries(game_dir)?.and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| town_lock_matches(*entry, scene, floor, x, y, tile))
        }))
    }

    pub fn town_magic_lock_target_at(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownLockEntry>> {
        Ok(load_town_lock_entries(game_dir)?.and_then(|entries| {
            entries.into_iter().find(|entry| {
                entry.kind == TownLockKind::Magic
                    && entry.scene == scene
                    && entry.floor == floor
                    && entry.x == x
                    && entry.y == y
                    && entry.unlocked_tile == tile
            })
        }))
    }

    pub fn town_stair_at(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownStairEntry>> {
        Ok(load_town_stair_entries(game_dir)?.and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| town_stair_matches(*entry, scene, floor, x, y, tile))
        }))
    }

    pub fn resolve_town_exit_tile(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        entry: TownExitTileEntry,
    ) -> io::Result<MoveOutcome> {
        self.resolve_town_exit_tile_transition(game_dir, scene, floor, entry, true)
    }

    pub fn resolve_town_exit_tile_after_turn(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        entry: TownExitTileEntry,
    ) -> io::Result<MoveOutcome> {
        self.resolve_town_exit_tile_transition(game_dir, scene, floor, entry, false)
    }

    pub fn resolve_town_exit_tile_transition(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        entry: TownExitTileEntry,
        advance_turn: bool,
    ) -> io::Result<MoveOutcome> {
        if advance_turn {
            self.advance_turn();
        }
        if self.restore_return_world() {
            self.message = format!(
                "Stepped onto town exit tile at ({}, {}) in {}; returned to overworld debug return point.",
                entry.x,
                entry.y,
                scene.key()
            );
            self.mark_visibility_dirty();
            return Ok(MoveOutcome::Transition(AreaTransition::ExitedLocation(
                scene,
            )));
        } else if self.restore_world_for_target(game_dir, PlayTarget::Town(scene))? {
            self.message = format!(
                "Stepped onto town exit tile at ({}, {}) in {}; returned to world-location table point.",
                entry.x,
                entry.y,
                scene.key()
            );
            self.mark_visibility_dirty();
            return Ok(MoveOutcome::Transition(AreaTransition::ExitedLocation(
                scene,
            )));
        }
        Ok(self.block_missing_town_return(
            scene,
            floor,
            format!(
                "Stepped onto town exit tile at ({}, {}) in {}",
                entry.x,
                entry.y,
                scene.key()
            ),
        ))
    }

    pub fn block_missing_town_return(
        &mut self,
        scene: Scene,
        floor: i8,
        event: String,
    ) -> MoveOutcome {
        self.area = Area::Town { scene, floor };
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.message =
            format!("{event}; missing clean return-coordinate metadata, stayed in location.");
        MoveOutcome::Blocked
    }

    pub fn town_trap_door_at(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<TownTrapDoorEntry>> {
        Ok(load_town_trap_door_entries(game_dir)?.and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| town_trap_door_matches(*entry, scene, floor, x, y, tile))
        }))
    }

    pub fn apply_town_trap_door(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        entry: TownTrapDoorEntry,
    ) -> io::Result<MoveOutcome> {
        self.apply_town_trap_door_transition(game_dir, scene, entry, true)
    }

    pub fn apply_town_trap_door_transition(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        entry: TownTrapDoorEntry,
        advance_turn: bool,
    ) -> io::Result<MoveOutcome> {
        self.grid = load_town_runtime_floor(game_dir, scene, entry.to_floor, self.clock.hour)?;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town {
            scene,
            floor: entry.to_floor,
        };
        self.clear_town_floor_reload_door_state();
        self.restore_revealed_town_secret_doors_for_floor(game_dir, scene, entry.to_floor)?;
        self.relink_npc_objects();
        self.mark_visibility_dirty();
        if advance_turn {
            self.advance_turn();
        }
        self.message = format!(
            "Fell through trap door at ({}, {}) to {} floor {}.",
            entry.x,
            entry.y,
            scene.key(),
            entry.to_floor
        );
        Ok(MoveOutcome::Transition(AreaTransition::ChangedFloor {
            scene,
            floor: entry.to_floor,
        }))
    }

    pub fn step_town_stair(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        nx: usize,
        ny: usize,
        tile: u8,
    ) -> io::Result<MoveOutcome> {
        let choices = self.connected_town_climb_choices(game_dir, scene, floor, nx, ny, tile)?;
        match choices.as_slice() {
            [] => {
                self.message = "Not climbable!".to_string();
                Ok(MoveOutcome::Blocked)
            }
            [intent] => {
                self.player.x = nx;
                self.player.y = ny;
                self.sync_player_object();
                self.mark_visibility_dirty();
                self.climb(game_dir, *intent)
            }
            _ => {
                self.player.x = nx;
                self.player.y = ny;
                self.sync_player_object();
                self.mark_visibility_dirty();
                Ok(self.start_klimb_direction_prompt())
            }
        }
    }
}
