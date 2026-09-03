use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    /// Record one published sound boundary.
    ///
    /// `audio.md §3`: the sound setting changes output, not cadence, so the
    /// runtime records every effect unconditionally and never consults the
    /// Ctrl-S boolean here. A frontend decides audibility.
    pub(crate) fn emit_sound_effect(&mut self, effect: SoundEffect) {
        self.sound_effect_serial = self.sound_effect_serial.wrapping_add(1);
        self.sound_effect_history
            .push((self.sound_effect_serial, effect));
        if self.sound_effect_history.len() > SOUND_EFFECT_HISTORY_CAPACITY {
            self.sound_effect_history.remove(0);
        }
    }

    /// `audio.md §6.1` scroll presentation: the variant is the scroll index.
    ///
    /// "A scroll supplies its **scroll index**, 0 through 7." The scroll
    /// variant disagrees with the corresponding spell's in six of the eight
    /// cases, so "a frontend must not reuse the spell's variant for the
    /// scroll" - this helper exists so no caller can.
    pub(crate) fn emit_scroll_shared_variant(&mut self, scroll_index: usize) {
        if let Some(variant) = audio::scroll_shared_variant(scroll_index) {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }
    }

    /// `audio.md §7.4` overworld blocked step.
    ///
    /// One of the exactly four sites that carry the 165 Hz / 200-unit recipe.
    /// The predicate "is not simply `step refused`": under sail nothing beeps,
    /// and a whirlpool-class blocker aboard a vehicle "returns completely
    /// silently, with no message at all". `RETRACTIONS.md` records that naming
    /// only town and combat under-scoped the cue by this mode.
    ///
    /// The `OUCH!` animated-terrain branch is **unidentified** in `§7.4`, so no
    /// caller can select it; see [`audio::overworld_blocked_step_beeps`].
    pub(crate) fn emit_overworld_blocked_step(&mut self, blocker_is_whirlpool_class: bool) {
        if audio::overworld_blocked_step_beeps(
            self.player.transport.is_ship_under_sail(),
            !self.player.transport.is_foot(),
            blocker_is_whirlpool_class,
            false,
        ) {
            self.emit_sound_effect(SoundEffect::BlockedStep);
        }
    }

    /// `audio.md §7.4` town blocked step.
    ///
    /// The second of the four sites: "Prints `Blocked!`, beeps, flushes
    /// type-ahead. Two refusal arms (object occupancy, tile-class refusal)
    /// share one tail." Town has no under-sail or whirlpool arm, so both
    /// refusal arms beep unconditionally.
    pub(crate) fn emit_town_blocked_step(&mut self) {
        self.emit_sound_effect(SoundEffect::BlockedStep);
    }

    /// Run the `audio.md §8.4` shared major full-viewport flash.
    ///
    /// Eight rounds of four 58-band sweeps: 1,856 band draws and 1,856
    /// frequency changes. Every band consumes one **gameplay** PRNG draw, so
    /// this must be called at the published boundary whether or not sound is
    /// audible — muting suppresses each tone start but skips none of the
    /// advances. Shrine restoration, a recognized Word of Power, and
    /// Shadowlord destruction all share it.
    pub(crate) fn emit_major_flash(&mut self) {
        let mut prng = U5Prng::new(self.prng_state);
        let bands = audio::draw_major_flash_bands(&mut prng);
        self.prng_state = prng.state();
        self.emit_sound_effect(SoundEffect::MajorFlash { bands });
    }

    pub fn sound_effects_after(&self, serial: u64) -> Vec<SoundEffect> {
        self.sound_effect_history
            .iter()
            .filter(|(event_serial, _)| *event_serial > serial)
            .map(|(_, effect)| effect.clone())
            .collect()
    }

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
        Self::world_overlay_objects_from_active_objects(&self.active_objects)
    }

    fn world_overlay_objects_from_active_objects(
        active_objects: &[ActiveObject],
    ) -> Vec<ActiveObject> {
        let mut objects = vec![ActiveObject::empty(); OOL_SLOTS - 1];
        for (index, object) in active_objects
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

    /// `save-load.md §5.2`: the save path writes the staged underworld
    /// mirror back unless its entry required-disk state was already the
    /// canonical Britannia disk. That session state is captured on the
    /// way in, so it is threaded rather than read at the write site.
    ///
    /// Normal gameplay resources run under canonical Britannia index 1.
    /// A mounted-directory runtime still follows the same request sequence;
    /// every role simply resolves to its one virtual fixed drive.
    pub fn save_game_command(
        &mut self,
        game_dir: &Path,
        confirm: Option<bool>,
    ) -> io::Result<MoveOutcome> {
        self.save_game_command_with_entry_required_disk(game_dir, confirm, RequiredDisk::Britannia)
    }

    pub fn save_game_command_with_entry_required_disk(
        &mut self,
        game_dir: &Path,
        confirm: Option<bool>,
        entry_required_disk: RequiredDisk,
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
                self.write_save_files_with_entry_required_disk(game_dir, entry_required_disk)?;
                // `save-load.md §5.2` steps 2 and 8: on `Y` the handler
                // "prints `Yes` followed by `Saving...`", and after the
                // write it "prints `Done.`". The reply lands on the
                // still-open `Save game?` line - the original renders
                // `Save game? Yes` / `Saving...` / `Done.` on three rows,
                // not one wrapped run.
                if !self.complete_open_direction_echo(
                    SAVE_PROMPT_MESSAGE,
                    &format!(" {SAVE_PROMPT_YES_REPLY}"),
                ) && !self.complete_open_direction_echo(SAVE_PROMPT_LINE, SAVE_PROMPT_YES_REPLY)
                {
                    self.emit_message_line(SAVE_PROMPT_YES_REPLY);
                }
                self.emit_message_line(SAVE_IN_PROGRESS_MESSAGE);
                self.emit_message_line(SAVE_DONE_MESSAGE);
                Ok(MoveOutcome::Saved)
            }
        }
    }

    pub fn write_save_files(&mut self, game_dir: &Path) -> io::Result<()> {
        self.write_save_files_with_entry_required_disk(game_dir, RequiredDisk::Britannia)
    }

    pub fn write_save_files_with_entry_required_disk(
        &mut self,
        game_dir: &Path,
        entry_required_disk: RequiredDisk,
    ) -> io::Result<()> {
        let mut disk_session = DiskPromptSession::single_directory();
        disk_session
            .request_disk(entry_required_disk.index())
            .expect("entry required-disk roles are canonical");
        self.write_save_files_with_disk_session(game_dir, &mut disk_session)
    }

    pub fn write_save_files_with_disk_session(
        &mut self,
        game_dir: &Path,
        disk_session: &mut DiskPromptSession,
    ) -> io::Result<()> {
        let entry_required_disk = disk_session.required_disk();
        let result =
            self.write_save_files_in_disk_session(game_dir, disk_session, entry_required_disk);
        disk_session
            .request_disk(entry_required_disk.index())
            .expect("captured required-disk roles are canonical");
        result
    }

    fn write_save_files_in_disk_session(
        &mut self,
        game_dir: &Path,
        disk_session: &mut DiskPromptSession,
        entry_required_disk: RequiredDisk,
    ) -> io::Result<()> {
        let (scene, z, x, y) = self.current_save_location().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "save game is only available in active play modes",
            )
        })?;
        self.sync_player_object();

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
        // `formats/saved-gam.md §5`: the byte at `0x02DA` is the per-turn
        // cleanup's pre-cascade hour snapshot, "used by the time cleanup to
        // detect hour crossings", and `time.md §2` has it "taken at the start
        // of every cleanup pass".
        save[SAVE_SAVED_HOUR_SNAPSHOT_OFFSET] = self.cleanup_previous_hour;
        // `formats/saved-gam.md §5` / `time.md §11` (spec `0170809`):
        // `0x02DE` is the "Twelve-hour hour value / audio repeat countdown",
        // "written with the twelve-hour form of the hour when the cleanup
        // finds the snapshot at `0x02DA` disagreeing with the hour at
        // `0x02D9`, then counted down toward zero by the ambient-audio
        // tick". `RETRACTIONS.md` R338 withdraws the old "12-hour display"
        // reading this engine used to justify a blind template round trip:
        // the value rule survives but the byte is live state, so it is
        // flushed from the counter the clock and the audio tick maintain.
        // The DOS build's zero on a no-turn load-and-save and on a
        // four-turn 08:59 -> 09:03 session is what the write-then-decay
        // pair produces once any idle world ticks have run.
        save[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET] = self.twelve_hour_audio_repeats;
        // `formats/saved-gam.md §5.1` (spec `0170809`): the cached Trammel
        // and Felucca moon-phase digits, "gameplay state, not scratch" -
        // "natural-moongate transit selects its destination from these two
        // cached bytes and from nothing else".
        save[SAVE_CACHED_TRAMMEL_GLYPH_OFFSET] = self.cached_moon_glyph_bytes[0];
        save[SAVE_CACHED_FELUCCA_GLYPH_OFFSET] = self.cached_moon_glyph_bytes[1];
        // `formats/saved-gam.md §10` (spec `0170809`): the ambient light
        // level, "recomputed by **every** clock call including the mode-zero
        // ... call that scene entry issues". The shipped seed's `5` "is a
        // stale sample the first clock call overwrites", so a save taken
        // after entry carries the recomputed value, not the template's.
        save[SAVE_AMBIENT_LIGHT_OFFSET] = self.ambient_light;
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
        save[SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_OFFSET
            ..SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_OFFSET + SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_LEN]
            .copy_from_slice(&self.combat_interference_sources);
        save[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = self.shrine_ordained_mask;
        save[SAVE_SHRINE_CODEX_MASK_OFFSET] = self.shrine_codex_mask;
        save[SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET
            ..SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET + SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT]
            .copy_from_slice(&self.word_of_power_seal_flags);
        save[SAVE_SHRINE_RUIN_FLAGS_OFFSET
            ..SAVE_SHRINE_RUIN_FLAGS_OFFSET + SAVE_SHRINE_RUIN_FLAG_COUNT]
            .copy_from_slice(&self.shrine_ruin_flags);
        save[SAVE_MORAL_STANDING_OFFSET] = self.moral_standing;
        save[SAVE_TOLL_PROGRESS_OFFSET] = self.toll_progress;
        // `overworld.md §9.1` (spec HEAD c00bf63): persist the shared
        // gate-presence counter so a save taken at 20:07 reloads with
        // its gates at the height they had.
        save[SAVE_NATURAL_MOONGATE_COUNTER_OFFSET] = self.natural_moongate_counter;
        save[SAVE_CAMP_COOLDOWN_OFFSET] = self.camp_cooldown;
        save[SAVE_CAMP_MONTH_COOKIE_OFFSET] = self.camp_month_cookie;
        if let Some(tracker) = self.door_tracker {
            save[SAVE_DOOR_TRACKER_PREVIOUS_TILE_OFFSET] = tracker.previous_tile;
            save[SAVE_DOOR_TRACKER_X_OFFSET] = u8::try_from(tracker.x).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("door tracker X is outside byte range: {}", tracker.x),
                )
            })?;
            save[SAVE_DOOR_TRACKER_Y_OFFSET] = u8::try_from(tracker.y).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("door tracker Y is outside byte range: {}", tracker.y),
                )
            })?;
            save[SAVE_DOOR_TRACKER_COUNTDOWN_OFFSET] = tracker.turns_remaining;
        } else {
            save[SAVE_DOOR_TRACKER_PREVIOUS_TILE_OFFSET..=SAVE_DOOR_TRACKER_COUNTDOWN_OFFSET]
                .fill(0);
        }
        let pending_vehicle_save = self
            .return_world
            .as_ref()
            .and_then(|world| world.pending_vehicle)
            .map(PendingVehicleSaveState::from_acquisition)
            .unwrap_or(self.pending_vehicle_save);
        self.pending_vehicle_save = pending_vehicle_save;
        save[SAVE_PENDING_VEHICLE_X_OFFSET] = pending_vehicle_save.x;
        save[SAVE_PENDING_VEHICLE_Y_OFFSET] = pending_vehicle_save.y;
        save[SAVE_PENDING_VEHICLE_CLASS_OFFSET] = pending_vehicle_save.class_byte;
        save[SAVE_ACTIVE_EFFECT_CODE_OFFSET] = self.active_effect_tag.unwrap_or(0);
        save[SAVE_ACTIVE_EFFECT_DURATION_OFFSET] = self.active_effect_counter;
        save[SAVE_FORTUNES_OF_WAR_OFFSET] = self.fortunes_of_war;
        save[SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET
            ..SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES]
            .copy_from_slice(&self.fixed_hidden_treasure_found);
        save[SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET] = self.fixed_hidden_treasure_daily_day;
        // `formats/saved-gam.md` §10, record 15: the gate at `0x0241` is
        // "Not a dedicated cookie. This is the **equipment-inventory counter
        // for item id `39` (Glass Sword)** from Section 7". It is inside the
        // equipment-stock block written above, so there is deliberately no
        // second write here - one used to clobber the Glass Sword counter.
        save[SAVE_SHADOWLORD_HIDEOUTS_OFFSET..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
            .copy_from_slice(&self.shadowlord_hideouts);
        // `formats/saved-gam.md §10` / `town-mode.md §5` step 6 (spec
        // `0170809`): `0x03B2` is the resident-Shadowlord selector, "a
        // **per-entry latch, not durable world state**". Town-family entry
        // stores `0xFF` unconditionally - the write "sits after the
        // entry-mode guard", so it happens on preserving re-entries too -
        // and the install helper replaces it with an index only in a
        // hosting location. "A byte-compatible producer emits `0xFF` for
        // any save taken inside a location; a save tool should preserve
        // whatever it finds", so a save taken outside one leaves the
        // template byte alone. The factory seed's `0` is "a stale,
        // semantically wrong value".
        if matches!(self.area, Area::Town { .. }) {
            save[SAVE_RESIDENT_SHADOWLORD_OFFSET] = self
                .resident_shadowlord
                .and_then(|index| u8::try_from(index).ok())
                .unwrap_or(SAVE_RESIDENT_SHADOWLORD_NONE);
        }
        encode_npc_mask_bank(
            &mut save,
            SAVE_NPC_REMOVED_MASKS_OFFSET,
            &self.removed_town_npc_flags,
        );
        encode_npc_mask_bank(
            &mut save,
            SAVE_NPC_NAME_KNOWN_MASKS_OFFSET,
            &self.talk_branch_flags,
        );
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
            // `formats/saved-gam.md` §3.1: the per-character month counter is
            // "capped at 25" by *the time system* when it increments at the
            // 28-day rollover. Clamping again on write mutates an inherited
            // byte the engine only read - the shipped seed carries `0xFF`
            // here - so the raw value round-trips and the cap stays in the
            // ageing pass that owns it.
            save[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET] = roster_record.stay_counter;
            save[record + SAVE_CHARACTER_LEVEL_OFFSET] = member.level;
            let start = record + SAVE_CHARACTER_EQUIPMENT_OFFSET;
            save[start..start + EQUIPMENT_SLOT_COUNT].copy_from_slice(&roster_record.equipment);
        }
        encode_inn_registry(&mut save, &self.inn_registry);
        let active_table = encode_active_object_table(&self.active_objects)?;
        save[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN]
            .copy_from_slice(&active_table);

        disk_session.request_operation(DiskOperationFamily::GameplayResources);
        let (saved_ool, _) = stage_saved_ool_for_save(game_dir, entry_required_disk)?;
        disk_session.request_operation(DiskOperationFamily::UltimaVSaveFiles);
        write_disk_file(&game_dir.join(SAVED_GAM_FILENAME), save)?;
        write_disk_file(&game_dir.join(SAVED_OOL_FILENAME), saved_ool)?;
        write_world_progress_state(game_dir, WorldProgressState::from_play_state(self))?;
        write_town_npc_mutations(game_dir, &self.town_npc_mutations)?;
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
        let marker = self.player.transport.save_marker();
        self.active_objects.push(ActiveObject {
            type_byte: marker,
            tile: marker,
            x,
            y,
            z: plane.save_floor(),
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: 0,
            aux3: 0,
        });
        self.active_objects.extend(overlay);
        self.place_underworld_fixed_objects(plane);
        self.cache_current_world_overlay();
        Ok(())
    }

    /// `catalogs/quest-graph.md §5`, "Where the shards are: fixed
    /// Underworld placement": the three shards and the Amulet of Lord
    /// British "are ordinary active objects placed at fixed Underworld
    /// coordinates by the outdoor setup pass that runs whenever the
    /// party is on a non-surface outdoor plane", every record "on the
    /// Underworld plane (floor byte `255`)".
    ///
    /// Both gates are required. A shard is emitted only while "the party
    /// does not carry it **and** [its Shadowlord's] slot is not
    /// vanquished", because destruction "clears exactly the carried flag
    /// this pass reads", so "an engine that implements the placement with
    /// only the carried-flag half of the gate will respawn every spent
    /// shard". The Shadowlord half is the *vanquished* test rather than
    /// the living test: `systems/time.md §7` makes slot value `0` mean
    /// "not yet placed", "neither 'in a town' nor 'vanquished'", and a
    /// newly created game holds `0` in all three slots until the first
    /// midnight pass.
    ///
    /// "The pass is a placement pass, not a respawn: once the carried
    /// flag is set the object is never emitted again."
    pub fn place_underworld_fixed_objects(&mut self, plane: WorldPlane) {
        if plane != WorldPlane::Underworld {
            return;
        }
        for placement in UNDERWORLD_FIXED_OBJECT_PLACEMENTS {
            if self
                .special_items
                .get(placement.special_item_index)
                .copied()
                .unwrap_or(0)
                != 0
            {
                continue;
            }
            if let Some(shadowlord_index) = placement.shadowlord_index {
                if self.shadowlord_vanquished(shadowlord_index) {
                    continue;
                }
            }
            let x = placement.x as usize;
            let y = placement.y as usize;
            // The overlay file may already ship the same record. The pass
            // places one object, so never stack a second copy on the cell.
            if self.active_objects.iter().any(|object| {
                object.type_byte == placement.class_byte
                    && object.aux1 == placement.subtype
                    && object.x == x
                    && object.y == y
            }) {
                continue;
            }
            let object = ActiveObject {
                type_byte: placement.class_byte,
                // `containers.md §3` puts the loose item-art band at
                // `0x80..=0xBF`, which is what `gettable_object_visual`
                // accepts, and the §8 quest class bytes sit inside it. The
                // individual art id for these four records is not
                // published, so the class byte doubles as the visual until
                // a clean tile catalog names one.
                tile: placement.class_byte,
                x,
                y,
                z: plane.save_floor(),
                phase: STEADY_PHASE,
                aux1: placement.subtype,
                aux3: 0,
            };
            if let Some(slot) = self
                .active_objects
                .iter()
                .enumerate()
                .skip(ACTIVE_OBJECT_ORDINARY_FIRST)
                .find_map(|(slot, existing)| existing.is_empty().then_some(slot))
            {
                self.active_objects[slot] = object;
            } else if self.active_objects.len() < OOL_SLOTS {
                self.active_objects.push(object);
            }
        }
    }

    pub fn load_town_scene(
        game_dir: &Path,
        scene: Scene,
        options: PlayOptions,
    ) -> io::Result<Self> {
        let mut grid = load_floor(game_dir, scene, options.floor)?;
        let passability = load_tile_passability(game_dir)?;
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        // `visibility.md §12.6`: inside a location the map setup clears both
        // beacon positions and records up to two bright-light hits. Harvested
        // from the raw floor, before the runtime normalisation pass rewrites
        // any cell.
        let beacon_sources = harvest_location_beacon_sources(&grid);
        normalize_town_runtime_floor(&mut grid, options.clock.hour);
        let default_entry = if options.floor == 0 {
            Some((LOCATION_DEFAULT_ENTRY_X, LOCATION_DEFAULT_ENTRY_Y))
        } else {
            None
        };
        let saved_active_objects = options.saved_active_objects.clone();
        let has_saved_active_objects = saved_active_objects.is_some();
        let saved_game_reload = options.save_template_source == SaveTemplateSource::SavedGame;
        let (x, y) = match options.start.or(default_entry) {
            Some(pos) => {
                if pos.0 >= 32 || pos.1 >= 32 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "start coordinate must be inside 0..31, got ({}, {})",
                            pos.0, pos.1
                        ),
                    ));
                }
                if !saved_game_reload {
                    validate_start(&grid, pos, passability.as_ref())?;
                }
                pos
            }
            // Non-entry callers loading another floor must normally supply
            // the preserved party coordinate. Keep the first-walkable path
            // only as a graphics-free harness default; overworld entry on
            // floor zero has already selected the fixed #94 coordinate.
            None => first_walkable(&grid, passability.as_ref()).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "no playable start cell")
            })?,
        };

        let world_overlays = initial_world_overlay_cache(&options);
        let transport = options.transport;
        let (player_aux1, player_aux3) = match transport {
            TransportState::Ship { hull, skiffs, .. } => (hull, skiffs),
            _ => (0, 0),
        };
        let marker = transport.save_marker();
        let mut active_objects = vec![ActiveObject {
            type_byte: marker,
            tile: marker,
            x,
            y,
            z: options.floor,
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: player_aux1,
            aux3: player_aux3,
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
                transport,
            },
            active_objects,
            npcs: Vec::new(),
            // `commands.md` Journey Onward contract: the four bytes survive
            // save decoding, but town setup clears only the active/previous-
            // tile byte before the authored location floor is installed.
            // The remaining bytes stay resident and inert for round-tripping.
            door_tracker: options.door_tracker.map(|mut tracker| {
                if saved_game_reload {
                    tracker.previous_tile = 0;
                }
                tracker
            }),
            door_tracker_closed: false,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            grid,
            world_live_chunks: None,
            clock: options.clock,
            status_pass_previous_hour: options.clock.hour,
            cleanup_previous_hour: options.cleanup_previous_hour,
            twelve_hour_audio_repeats: options.twelve_hour_audio_repeats,
            ambient_audio_sub_tick: 0,
            dungeon_loop_minute_charged: false,
            prng_state: host_clock_prng_seed_now(),
            // Neither counter restarts on area entry, but for two
            // different published reasons.
            //
            // `water_scroll` is the `animation.md §12` driver-side layer.
            // `§9`: it "is **not** reset ... its state lives in the asset
            // buffer for the whole program run", and `§12.1` adds that the
            // mutation "survives scene changes, save loads, and everything
            // else short of reloading the asset".
            //
            // `animation` is the `§6` frame-selector pass, a different
            // layer, and `§9` does license resetting it ("Rebuild or reset
            // transient animation counters during startup or mode entry as
            // needed"). It is carried anyway on `§6.1`'s own terms: the
            // selector "is transient and global. It is not part of saved
            // state, it survives map changes and reloads."
            //
            // Either way an area constructor carries the running phases in
            // rather than starting a fresh clock — see
            // [`AnimationAssetBuffer`].
            animation: options.animation_asset_buffer.animation,
            water_scroll: options.animation_asset_buffer.water_scroll,
            // GAP: `fire_flicker` is the same `§12` driver-side layer and
            // `§9` names it explicitly ("the fire fixtures keep every noise
            // pattern ever XORed into them"), so it should be carried too.
            // It is not yet, because [`AnimationAssetBuffer`] is a `Copy`
            // value and [`FireFlickerClock`] holds two 16x16 parity planes
            // per field tile. Restarting it on area entry only reseeds the
            // flicker noise, which has no gameplay meaning; the water phase
            // and the frame selector, which are visible as a cadence, are
            // carried.
            fire_flicker: FireFlickerClock::default(),
            dungeon_fountain_frame: 0,
            natural_moongate_counter: options.natural_moongate_counter,
            natural_moongate_live_cells: Vec::new(),
            last_natural_moongate_transit: None,
            pending_map_viewport_dissolves: Vec::new(),
            pending_blackthorn_rescue_playbacks: Vec::new(),
            pending_combat_terrain_reveals: Vec::new(),
            pending_potion_flash: None,
            pending_stonegate_trapdoor_playback: None,
            pending_town_status_provision_pass: false,
            pending_town_npc_schedule_pass: false,
            pending_town_active_object_pass: false,
            // `formats/saved-gam.md §5.1` (spec `0170809`): the pair is
            // restored from `0x02DF`/`0x02E0` and then rewritten by the
            // scene-entry moon-strip refresh - `RETRACTIONS.md` R343: "The
            // scene-entry callers are also the mechanism by which the
            // cached glyph digits are refreshed on a **Journey Onward**".
            // In a scene outside the surface/town family the renderer is
            // never reached at all (`moons.md §2.2`: "Nothing is drawn and
            // nothing is cached"), so there the restored bytes stand.
            cached_moon_glyph_bytes: options.cached_moon_glyph_bytes,
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
            resident_shadowlord: None,
            summoned_shadowlord: None,
            removed_town_npc_flags: options.removed_town_npc_flags,
            shrine_ordained_mask: options.shrine_ordained_mask,
            shrine_codex_mask: options.shrine_codex_mask,
            word_of_power_seal_flags: options.word_of_power_seal_flags,
            shrine_ruin_flags: options.shrine_ruin_flags,
            moral_standing: options.moral_standing,
            town_drunkenness_counter: 0,
            tavern_secondary_drink_count: 0,
            toll_progress: options.toll_progress,
            avatar_stats: options.avatar_stats,
            torches: options.torches,
            torch_counter: options.torch_counter,
            light_spell_counter: options.light_spell_counter,
            ambient_light: options.ambient_light,
            light_beacon: LightBeaconState {
                sources: beacon_sources,
                bearing: BEACON_INITIAL_BEARING,
            },
            beacon_bearing_stencils: load_beacon_bearing_stencils(game_dir)?,
            local_light_mask: [false; TOWN_GRID_BYTES],
            visibility_dirty: false,
            visibility_grid: [0; VISIBILITY_GRID_LEN],
            terrain_band: [0; TERRAIN_BAND_LEN],
            visibility_buffers_ready: false,
            world_underfoot_blackout_latched: false,
            wind: options.wind,
            wind_save_byte: options.wind_save_byte,
            time_stop_counter: options.time_stop_counter,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            camp_cooldown: options.camp_cooldown,
            camp_month_cookie: options.camp_month_cookie,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_action_result: 0,
            combat_interference_sources: options.combat_interference_sources,
            combat_active: false,
            pace_combat_presentations: false,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            pending_combat_terrain_trigger_slot: None,
            pending_town_conflict: None,
            pending_outdoor_reaction_slots: Vec::new(),
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
            combat_cursor_blink: false,
            combat_round_loop_prologue_ran: false,
            combat_secondary_marker: None,
            combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_stall_pending: false,
            pending_vehicle_save: options
                .pending_vehicle
                .map(PendingVehicleSaveState::from_acquisition)
                .unwrap_or(options.pending_vehicle_save),
            turn: 0,
            // `cleak/u5-spec#81` dropped the dungeon entry line for the same
            // reason this town one goes: no `systems/` document publishes a
            // scene-entry narration, and the line printed the party's raw map
            // coordinates and an internal scene key. `town-mode.md` has the
            // entry paint the frame and nothing else. The route harness reads
            // position from `play_script_state_line`, not from this slot.
            message: String::new(),
            message_transcript: Vec::new(),
            message_transcript_revision: 0,
            message_flushed: String::new(),
            pending_command_echo: None,
            pending_hourly_status_message: None,
            debug_enter: options.debug_enter,
            return_world: None,
            world_overlays,
            save_template_source: options.save_template_source,
            typeahead_buffer_enabled: false,
            music_enabled: true,
            sound_effect_serial: 0,
            sound_effect_history: Vec::new(),
            harpsichord_progress: 0,
            active_blackthorn_guard_demand: None,
            pending_town_arrest: None,
            endgame: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            active_shop: None,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_npc_slot: None,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_party_selector: None,
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
            active_shrine_restoration: None,
            active_wishing_well: None,
            active_view_overlay: None,
            visibility_sweep: None,
            party_marker_tile_override: None,
            active_direction_prompt: None,
            active_yes_no_prompt: None,
            town_npc_mutations: options.town_npc_mutations,
            talk_branch_flags: options.talk_branch_flags,
            conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
            inn_registry: options.inn_registry,
        };
        if has_saved_active_objects {
            state.load_scheduled_npcs_from_existing_active_objects(&npc_slots);
        } else {
            state.load_scheduled_npcs(&npc_slots);
        }
        if let Some((slot, index)) = state.install_shadowlord_entry_encounter() {
            let shadowlord = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
            if !state.message.is_empty() {
                state.message.push('\n');
            }
            state
                .message
                .push_str(&format!("An air of {shadowlord} doth surround thee."));
            if let Some(slot) = slot {
                state.message.push_str(&format!(
                    " Shadowlord actor installed in active-object slot {slot}."
                ));
            }
        }
        // `moons.md §3` (the caller list `RETRACTIONS.md` R343 put on
        // the record): the moon-phase status-strip renderer's callers are
        // "every overworld scene entry; every town-family scene entry" and
        // the hour-change hook, and "Each refresh caches the two glyph
        // bytes for the current day *before* it tests whether either
        // marker is on the visible horizon". `moons.md §2.2` adds that the
        // scene-entry callers "carry no such gate" as the hour-change
        // hook's floor test, so a basement or Underworld entry refreshes
        // the pair too.
        state.refresh_cached_moon_glyphs_at_scene_entry();
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
        let level = options.floor as u8;
        let default_start = (1, 1);
        let saved_active_objects = options.saved_active_objects.clone();
        let saved_game_reload = options.save_template_source == SaveTemplateSource::SavedGame;
        let (x, y) = match options.start {
            Some(pos) => {
                if pos.0 >= DUNGEON_SIDE || pos.1 >= DUNGEON_SIDE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "dungeon coordinate must be inside 0..7, got ({}, {})",
                            pos.0, pos.1
                        ),
                    ));
                }
                if !saved_game_reload {
                    validate_dungeon_start(&grid, scene, level, pos)?;
                }
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
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
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
            door_tracker_closed: false,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            grid,
            world_live_chunks: None,
            clock: options.clock,
            status_pass_previous_hour: options.clock.hour,
            cleanup_previous_hour: options.cleanup_previous_hour,
            twelve_hour_audio_repeats: options.twelve_hour_audio_repeats,
            ambient_audio_sub_tick: 0,
            dungeon_loop_minute_charged: false,
            prng_state: host_clock_prng_seed_now(),
            // Neither counter restarts on area entry, but for two
            // different published reasons.
            //
            // `water_scroll` is the `animation.md §12` driver-side layer.
            // `§9`: it "is **not** reset ... its state lives in the asset
            // buffer for the whole program run", and `§12.1` adds that the
            // mutation "survives scene changes, save loads, and everything
            // else short of reloading the asset".
            //
            // `animation` is the `§6` frame-selector pass, a different
            // layer, and `§9` does license resetting it ("Rebuild or reset
            // transient animation counters during startup or mode entry as
            // needed"). It is carried anyway on `§6.1`'s own terms: the
            // selector "is transient and global. It is not part of saved
            // state, it survives map changes and reloads."
            //
            // Either way an area constructor carries the running phases in
            // rather than starting a fresh clock — see
            // [`AnimationAssetBuffer`].
            animation: options.animation_asset_buffer.animation,
            water_scroll: options.animation_asset_buffer.water_scroll,
            // GAP: `fire_flicker` is the same `§12` driver-side layer and
            // `§9` names it explicitly ("the fire fixtures keep every noise
            // pattern ever XORed into them"), so it should be carried too.
            // It is not yet, because [`AnimationAssetBuffer`] is a `Copy`
            // value and [`FireFlickerClock`] holds two 16x16 parity planes
            // per field tile. Restarting it on area entry only reseeds the
            // flicker noise, which has no gameplay meaning; the water phase
            // and the frame selector, which are visible as a cadence, are
            // carried.
            fire_flicker: FireFlickerClock::default(),
            dungeon_fountain_frame: 0,
            natural_moongate_counter: options.natural_moongate_counter,
            natural_moongate_live_cells: Vec::new(),
            last_natural_moongate_transit: None,
            pending_map_viewport_dissolves: Vec::new(),
            pending_blackthorn_rescue_playbacks: Vec::new(),
            pending_combat_terrain_reveals: Vec::new(),
            pending_potion_flash: None,
            pending_stonegate_trapdoor_playback: None,
            pending_town_status_provision_pass: false,
            pending_town_npc_schedule_pass: false,
            pending_town_active_object_pass: false,
            // `formats/saved-gam.md §5.1` (spec `0170809`): the pair is
            // restored from `0x02DF`/`0x02E0` and then rewritten by the
            // scene-entry moon-strip refresh - `RETRACTIONS.md` R343: "The
            // scene-entry callers are also the mechanism by which the
            // cached glyph digits are refreshed on a **Journey Onward**".
            // In a scene outside the surface/town family the renderer is
            // never reached at all (`moons.md §2.2`: "Nothing is drawn and
            // nothing is cached"), so there the restored bytes stand.
            cached_moon_glyph_bytes: options.cached_moon_glyph_bytes,
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
            resident_shadowlord: None,
            summoned_shadowlord: None,
            removed_town_npc_flags: options.removed_town_npc_flags,
            shrine_ordained_mask: options.shrine_ordained_mask,
            shrine_codex_mask: options.shrine_codex_mask,
            word_of_power_seal_flags: options.word_of_power_seal_flags,
            shrine_ruin_flags: options.shrine_ruin_flags,
            moral_standing: options.moral_standing,
            town_drunkenness_counter: 0,
            tavern_secondary_drink_count: 0,
            toll_progress: options.toll_progress,
            avatar_stats: options.avatar_stats,
            torches: options.torches,
            torch_counter: options.torch_counter,
            light_spell_counter: options.light_spell_counter,
            ambient_light: options.ambient_light,
            light_beacon: LightBeaconState {
                sources: [None; BEACON_SOURCE_SLOTS],
                bearing: BEACON_INITIAL_BEARING,
            },
            beacon_bearing_stencils: load_beacon_bearing_stencils(game_dir)?,
            local_light_mask: [false; TOWN_GRID_BYTES],
            visibility_dirty: false,
            visibility_grid: [0; VISIBILITY_GRID_LEN],
            terrain_band: [0; TERRAIN_BAND_LEN],
            visibility_buffers_ready: false,
            world_underfoot_blackout_latched: false,
            wind: options.wind,
            wind_save_byte: options.wind_save_byte,
            time_stop_counter: options.time_stop_counter,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            camp_cooldown: options.camp_cooldown,
            camp_month_cookie: options.camp_month_cookie,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_action_result: 0,
            combat_interference_sources: options.combat_interference_sources,
            combat_active: false,
            pace_combat_presentations: false,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            pending_combat_terrain_trigger_slot: None,
            pending_town_conflict: None,
            pending_outdoor_reaction_slots: Vec::new(),
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
            combat_cursor_blink: false,
            combat_round_loop_prologue_ran: false,
            combat_secondary_marker: None,
            combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_stall_pending: false,
            pending_vehicle_save: options
                .pending_vehicle
                .map(PendingVehicleSaveState::from_acquisition)
                .unwrap_or(options.pending_vehicle_save),
            turn: 0,
            // cleak/u5-spec#81: dungeon-mode.md publishes no dungeon-entry
            // narration, so nothing player-facing is printed here. The old
            // `Entered <name> (<name>) level N at (x, y).` line exposed raw
            // coordinates and a zero-based level; no test or suite depended
            // on it, so it is dropped rather than moved behind a debug flag.
            // `PlayState::area_status_label` still reports the one-based
            // level for diagnostics.
            message: String::new(),
            message_transcript: Vec::new(),
            message_transcript_revision: 0,
            message_flushed: String::new(),
            pending_command_echo: None,
            pending_hourly_status_message: None,
            debug_enter: options.debug_enter,
            return_world: None,
            world_overlays,
            save_template_source: options.save_template_source,
            typeahead_buffer_enabled: false,
            music_enabled: true,
            sound_effect_serial: 0,
            sound_effect_history: Vec::new(),
            harpsichord_progress: 0,
            active_blackthorn_guard_demand: None,
            pending_town_arrest: None,
            endgame: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            active_shop: None,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_npc_slot: None,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_party_selector: None,
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
            active_shrine_restoration: None,
            active_wishing_well: None,
            active_view_overlay: None,
            visibility_sweep: None,
            party_marker_tile_override: None,
            active_direction_prompt: None,
            active_yes_no_prompt: None,
            town_npc_mutations: options.town_npc_mutations,
            talk_branch_flags: options.talk_branch_flags,
            conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
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
        let mut grid = load_world_map(game_dir, plane)?;
        apply_world_quest_tile_substitutions(
            &mut grid,
            &options.word_of_power_seal_flags,
            &options.shrine_ruin_flags,
        );
        let passability = load_tile_passability(game_dir)?;
        let damage_tiles = load_world_damage_tile_entries(game_dir)?.unwrap_or_default();
        // Canonical Ultima V starting position: Iolo's Hut on the surface
        // (Britannia), at the cell south of the dwelling entrance. For the
        // Underworld there is no canonical fresh-start spawn, so we keep
        // the safer (1,1) seed and fall back to a search if that is blocked.
        let default_start = match plane {
            WorldPlane::Britannia => (62, 124),
            WorldPlane::Underworld => (1, 1),
        };
        let saved_game_reload = options.save_template_source == SaveTemplateSource::SavedGame;
        let (x, y) = match options.start {
            Some(pos) => {
                if pos.0 >= WORLD_SIDE || pos.1 >= WORLD_SIDE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "world coordinate must be inside 0..255, got ({}, {})",
                            pos.0, pos.1
                        ),
                    ));
                }
                if !saved_game_reload {
                    validate_world_start_for_transport(
                        &grid,
                        pos,
                        plane,
                        passability.as_ref(),
                        options.transport,
                        &damage_tiles,
                    )?;
                }
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
        let marker = transport.save_marker();
        let mut active_objects = vec![ActiveObject {
            type_byte: marker,
            tile: marker,
            x,
            y,
            z: plane.save_floor(),
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
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
        let mut pending_vehicle_save = options
            .pending_vehicle
            .map(PendingVehicleSaveState::from_acquisition)
            .unwrap_or(options.pending_vehicle_save);
        if let Some(pending) = options.pending_vehicle {
            place_pending_vehicle_acquisition(&mut active_objects, plane, pending)?;
            pending_vehicle_save = pending_vehicle_save.clear_class();
        }
        let world_live_chunks = Some(WorldLiveChunkBuffer::from_full_grid(
            plane,
            &grid,
            x,
            y,
            |_| LiveChunkSubstitutionPolicy::NONE,
        )?);
        // `visibility.md §12.6`: the chunk loader scans each freshly loaded
        // 32x32 window for the lighthouse tile and records the first hit, or
        // the "no beacon" sentinel when the window holds none.
        let beacon_sources = match &world_live_chunks {
            Some(buffer) => {
                harvest_outdoor_beacon_sources(buffer.scroll_base, |wx, wy| buffer.tile_at(wx, wy))
            }
            None => [None; BEACON_SOURCE_SLOTS],
        };

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
            door_tracker_closed: false,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            grid,
            world_live_chunks,
            clock: options.clock,
            status_pass_previous_hour: options.clock.hour,
            cleanup_previous_hour: options.cleanup_previous_hour,
            twelve_hour_audio_repeats: options.twelve_hour_audio_repeats,
            ambient_audio_sub_tick: 0,
            dungeon_loop_minute_charged: false,
            prng_state: host_clock_prng_seed_now(),
            // Neither counter restarts on area entry, but for two
            // different published reasons.
            //
            // `water_scroll` is the `animation.md §12` driver-side layer.
            // `§9`: it "is **not** reset ... its state lives in the asset
            // buffer for the whole program run", and `§12.1` adds that the
            // mutation "survives scene changes, save loads, and everything
            // else short of reloading the asset".
            //
            // `animation` is the `§6` frame-selector pass, a different
            // layer, and `§9` does license resetting it ("Rebuild or reset
            // transient animation counters during startup or mode entry as
            // needed"). It is carried anyway on `§6.1`'s own terms: the
            // selector "is transient and global. It is not part of saved
            // state, it survives map changes and reloads."
            //
            // Either way an area constructor carries the running phases in
            // rather than starting a fresh clock — see
            // [`AnimationAssetBuffer`].
            animation: options.animation_asset_buffer.animation,
            water_scroll: options.animation_asset_buffer.water_scroll,
            // GAP: `fire_flicker` is the same `§12` driver-side layer and
            // `§9` names it explicitly ("the fire fixtures keep every noise
            // pattern ever XORed into them"), so it should be carried too.
            // It is not yet, because [`AnimationAssetBuffer`] is a `Copy`
            // value and [`FireFlickerClock`] holds two 16x16 parity planes
            // per field tile. Restarting it on area entry only reseeds the
            // flicker noise, which has no gameplay meaning; the water phase
            // and the frame selector, which are visible as a cadence, are
            // carried.
            fire_flicker: FireFlickerClock::default(),
            dungeon_fountain_frame: 0,
            natural_moongate_counter: options.natural_moongate_counter,
            natural_moongate_live_cells: Vec::new(),
            last_natural_moongate_transit: None,
            pending_map_viewport_dissolves: Vec::new(),
            pending_blackthorn_rescue_playbacks: Vec::new(),
            pending_combat_terrain_reveals: Vec::new(),
            pending_potion_flash: None,
            pending_stonegate_trapdoor_playback: None,
            pending_town_status_provision_pass: false,
            pending_town_npc_schedule_pass: false,
            pending_town_active_object_pass: false,
            // `formats/saved-gam.md §5.1` (spec `0170809`): the pair is
            // restored from `0x02DF`/`0x02E0` and then rewritten by the
            // scene-entry moon-strip refresh - `RETRACTIONS.md` R343: "The
            // scene-entry callers are also the mechanism by which the
            // cached glyph digits are refreshed on a **Journey Onward**".
            // In a scene outside the surface/town family the renderer is
            // never reached at all (`moons.md §2.2`: "Nothing is drawn and
            // nothing is cached"), so there the restored bytes stand.
            cached_moon_glyph_bytes: options.cached_moon_glyph_bytes,
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
            resident_shadowlord: None,
            summoned_shadowlord: None,
            removed_town_npc_flags: options.removed_town_npc_flags,
            shrine_ordained_mask: options.shrine_ordained_mask,
            shrine_codex_mask: options.shrine_codex_mask,
            word_of_power_seal_flags: options.word_of_power_seal_flags,
            shrine_ruin_flags: options.shrine_ruin_flags,
            moral_standing: options.moral_standing,
            town_drunkenness_counter: 0,
            tavern_secondary_drink_count: 0,
            toll_progress: options.toll_progress,
            avatar_stats: options.avatar_stats,
            torches: options.torches,
            torch_counter: options.torch_counter,
            light_spell_counter: options.light_spell_counter,
            ambient_light: options.ambient_light,
            light_beacon: LightBeaconState {
                sources: beacon_sources,
                bearing: BEACON_INITIAL_BEARING,
            },
            beacon_bearing_stencils: load_beacon_bearing_stencils(game_dir)?,
            local_light_mask: [false; TOWN_GRID_BYTES],
            visibility_dirty: false,
            visibility_grid: [0; VISIBILITY_GRID_LEN],
            terrain_band: [0; TERRAIN_BAND_LEN],
            visibility_buffers_ready: false,
            world_underfoot_blackout_latched: false,
            wind: options.wind,
            wind_save_byte: options.wind_save_byte,
            time_stop_counter: options.time_stop_counter,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            camp_cooldown: options.camp_cooldown,
            camp_month_cookie: options.camp_month_cookie,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_action_result: 0,
            combat_interference_sources: options.combat_interference_sources,
            combat_active: false,
            pace_combat_presentations: false,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            pending_combat_terrain_trigger_slot: None,
            pending_town_conflict: None,
            pending_outdoor_reaction_slots: Vec::new(),
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
            combat_cursor_blink: false,
            combat_round_loop_prologue_ran: false,
            combat_secondary_marker: None,
            combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_stall_pending: false,
            pending_vehicle_save,
            turn: 0,
            // As above for the coordinate half. The wind half is a duplicate:
            // `text-output.md §10.7` puts the prevailing wind in the
            // viewport's bottom ribbon, which `gameplay_chrome` already
            // draws from the same state, so printing it here would show the
            // player the banner twice.
            message: String::new(),
            message_transcript: Vec::new(),
            message_transcript_revision: 0,
            message_flushed: String::new(),
            pending_command_echo: None,
            pending_hourly_status_message: None,
            debug_enter: options.debug_enter,
            return_world: None,
            world_overlays,
            save_template_source: options.save_template_source,
            typeahead_buffer_enabled: false,
            music_enabled: true,
            sound_effect_serial: 0,
            sound_effect_history: Vec::new(),
            harpsichord_progress: 0,
            active_blackthorn_guard_demand: None,
            pending_town_arrest: None,
            endgame: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            active_shop: None,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_npc_slot: None,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_party_selector: None,
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
            active_shrine_restoration: None,
            active_wishing_well: None,
            active_view_overlay: None,
            visibility_sweep: None,
            party_marker_tile_override: None,
            active_direction_prompt: None,
            active_yes_no_prompt: None,
            town_npc_mutations: options.town_npc_mutations,
            talk_branch_flags: options.talk_branch_flags,
            conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
            inn_registry: options.inn_registry,
        };
        state.sync_player_object();
        state.cache_current_world_overlay();
        // `moons.md §3` (the caller list `RETRACTIONS.md` R343 put on
        // the record): the moon-phase status-strip renderer's callers are
        // "every overworld scene entry; every town-family scene entry" and
        // the hour-change hook, and "Each refresh caches the two glyph
        // bytes for the current day *before* it tests whether either
        // marker is on the visible horizon". `moons.md §2.2` adds that the
        // scene-entry callers "carry no such gate" as the hour-change
        // hook's floor test, so a basement or Underworld entry refreshes
        // the pair too.
        state.refresh_cached_moon_glyphs_at_scene_entry();
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
        // `movement.md §2`: "No mode steps diagonally." World and town
        // movement consume the four cardinal directions only, so a diagonal
        // is refused before the facing is written. The dungeon step keeps its
        // own refusal below so its message stays mode-local.
        if !direction.is_cardinal() && !matches!(self.area, Area::Dungeon { .. }) {
            return Ok(MoveOutcome::Blocked);
        }
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
            let corner_tile = self.grid[TOWN_VIEWPORT_OFF_GRID_SAMPLE_INDEX];
            if !self.tile_walkable(corner_tile) {
                self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
                // `audio.md §7.4`: the town tile-class refusal arm.
                self.emit_town_blocked_step();
                self.advance_turn();
                return Ok(MoveOutcome::Blocked);
            }
            if self.blocking_town_object_at_candidate(nx, ny).is_some() {
                self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
                // `audio.md §7.4`: the town object-occupancy refusal arm.
                self.emit_town_blocked_step();
                self.advance_turn();
                return Ok(MoveOutcome::Blocked);
            }
            return Ok(self.start_town_exit_prompt(scene, floor));
        }

        let nx = nx as usize;
        let ny = ny as usize;
        if self.blocking_object_at(nx, ny).is_some() {
            self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
            // `audio.md §7.4`: the town object-occupancy refusal arm.
            self.emit_town_blocked_step();
            // INFERENCE, not a citation. The only published town-refusal
            // turn cost is `town-mode.md §15`'s table row "Terrain rejected
            // | ... | Consumes one normal town turn: advance the clock by
            // one minute, run underfoot/post-action processing, and run one
            // NPC schedule step" - and that table is introduced by
            // "**Boundary-exit attempts** have these exact turn effects",
            // so on its face it scores only a boundary step whose terrain
            // test (§15's `(31,31)` corner sample) rejects.
            //
            // Charging this interior occupancy refusal the same turn rests
            // on two joins the spec does not state as one rule: §15 applies
            // "the ordinary transport-sensitive terrain predicate" and §7
            // says "The ordinary passability and occupancy tests run first
            // and still win, so a destination the classifier rejects prints
            // the blocked feedback instead of prompting", i.e. the same
            // wrapper and the same classifier; and the occupancy arm
            // reaches the identical blocked-feedback tail (`audio.md §7.4`:
            // the two town arms "share one tail" - that sentence is about
            // the beep and the type-ahead flush, not the clock). §7 itself
            // says nothing about turn cost. Corroborated only in aggregate
            // by the defect-13 replay. Open spec question 2 in
            // `turn-clock-wind-report.md`.
            self.advance_turn();
            return Ok(MoveOutcome::Blocked);
        }
        let tile = self.grid[ny * 32 + nx];
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
            {
                return self.step_town_stair(game_dir, scene, floor, nx, ny, tile);
            }
        }
        let trapdoor_walkable = if let Some(game_dir) = game_dir {
            self.town_trap_door_at(game_dir, scene, floor, nx, ny, tile)?
                .is_some()
        } else {
            is_town_trapdoor_live_tile(tile)
        };
        if self.tile_walkable(tile) || trapdoor_walkable {
            self.player.x = nx;
            self.player.y = ny;
            self.sync_player_object();
            self.mark_visibility_dirty();
            // `text-output.md §10.2`/§10.3: the direction echo is the whole
            // of an accepted step; there is no result line and no
            // coordinate narration (`commands.md §8.1`).
            self.message = String::new();
            // `town-mode.md §17` "Underfoot-effect cadence is fixed": "The
            // underfoot handler is a per-turn post-action pass, not a
            // step-commit hook. Any earlier statement that the poison-gas
            // effect 'fires from the step path' is retracted". The gas arm
            // now lives in `apply_town_post_turn_effects_after_turn`, after
            // this turn's clock advance.
            self.advance_turn();
            Ok(MoveOutcome::Moved)
        } else {
            self.message = MOVEMENT_BLOCKED_REFUSAL.to_string();
            // `audio.md §7.4`: the town tile-class refusal arm.
            self.emit_town_blocked_step();
            // INFERENCE, not a citation, for the same reason as the
            // occupancy arm above: `town-mode.md §15`'s "Terrain rejected |
            // ... | Consumes one normal town turn" row is the terrain arm,
            // but its table is scoped to "**Boundary-exit attempts**". This
            // site is the interior step. See the derivation above and open
            // spec question 2 in `turn-clock-wind-report.md`.
            self.advance_turn();
            Ok(MoveOutcome::Blocked)
        }
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

    pub fn blocking_town_object_at_candidate(&self, x: isize, y: isize) -> Option<&ActiveObject> {
        ((0..32).contains(&x) && (0..32).contains(&y))
            .then(|| self.blocking_object_at(x as usize, y as usize))
            .flatten()
    }

    pub fn resolve_town_boundary_exit_transition(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let entries = effective_world_location_entries(game_dir)?;
        let matches: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| entry.target == PlayTarget::Town(scene))
            .collect();
        let Some(entry) = matches.first().copied() else {
            return Ok(self.block_missing_town_return(
                scene,
                floor,
                format!("Accepted the boundary exit from {}", scene.key()),
            ));
        };
        if matches.len() > 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} has multiple return rows for {}",
                    PlayTarget::Town(scene).key()
                ),
            ));
        }

        // `town-mode.md` §15: the scene selects the destination plane;
        // neither the gazetteer row's entry plane nor a cached debug return
        // point may override it. Ararat (`0x19`) is the sole Underworld arm.
        let plane = if scene.byte == TOWN_EXIT_UNDERWORLD_SCENE {
            WorldPlane::Underworld
        } else {
            WorldPlane::Britannia
        };

        self.restore_world_from_town_mirror(game_dir, plane, entry.x, entry.y)?;
        // `doors-and-z-transitions.md` Section 12.1, accepted arm: `Yes`, a
        // blank row, `Exit to`, then the plane name on its own row - here the
        // break before the plane name **is** in the data. Ararat (`0x19`) is
        // the only town-family location on the underworld plane.
        // The prompt left the cursor after its trailing space, so `Yes` lands
        // on that same row - "to leave? Yes" - and the blank row the data
        // carries then falls before `Exit to`.
        self.emit_message_line_continuing_row(format!(
            "{TOWN_EXIT_ACCEPTED_NARRATION}{}",
            match plane {
                WorldPlane::Britannia => TOWN_EXIT_TO_BRITANNIA_NARRATION,
                WorldPlane::Underworld => TOWN_EXIT_TO_UNDERWORLD_NARRATION,
            }
        ));
        Ok(MoveOutcome::Transition(AreaTransition::ExitedLocation(
            scene,
        )))
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
        let sidecar = load_town_trap_door_entries(game_dir)?.and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| town_trap_door_matches(*entry, scene, floor, x, y, tile))
        });
        Ok(sidecar.or_else(|| {
            is_town_trapdoor_live_tile(tile).then_some(TownTrapDoorEntry {
                scene,
                floor,
                x,
                y,
                to_floor: floor.saturating_sub(1),
                expected_tile: Some(tile),
            })
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
        self.message = "A TRAPDOOR!".to_string();
        self.reload_town_floor(game_dir, scene, entry.to_floor)?;
        if advance_turn {
            self.advance_turn();
        }
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
