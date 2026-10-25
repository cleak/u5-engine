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
        // `magic.md §8`: a spell's shared variant is its whole-view flash,
        // and the window holds the cast rows alone until it ends.
        if self.message_window_flash_hold.casting
            && matches!(effect, SoundEffect::SharedVariant { .. })
        {
            let flash_secs = effect
                .program(&mut audio::RumbleJitter::default())
                .duration(true)
                .as_secs_f64();
            self.hold_message_window_for_spell_flash(flash_secs);
        }
        self.sound_effect_serial = self.sound_effect_serial.wrapping_add(1);
        self.sound_effect_history
            .push((self.sound_effect_serial, effect));
        if self.sound_effect_history.len() > SOUND_EFFECT_HISTORY_CAPACITY {
            self.sound_effect_history.remove(0);
        }
    }

    /// Freeze the message window for one spell flash of `flash_secs`; see
    /// [`MessageWindowFlashHold`].
    ///
    /// The window is captured as it stands - the transcript plus whatever the
    /// slot still shows on top of it - because the flash starts before the
    /// handler has printed anything of its result. The hold is a staged pure
    /// hold, so the loop buffers keys for its length the way the original's
    /// synchronous flash does. A presentation already staging its own beats
    /// is left alone: the hold must release first to know its snapshot is
    /// stale, and nothing else may be queued ahead of it.
    pub(crate) fn hold_message_window_for_spell_flash(&mut self, flash_secs: f64) {
        let hold = &mut self.message_window_flash_hold;
        if !hold.is_active() {
            if !self.staged_narration.is_empty() {
                return;
            }
            hold.entries = Some(self.message_transcript.clone());
            hold.slot = (!self.message.is_empty() && self.message != self.message_flushed)
                .then(|| self.message.clone());
        }
        hold.pending_beats = hold.pending_beats.saturating_add(1);
        self.staged_narration.push_secs(flash_secs, String::new());
    }

    /// Count `released` staged beats against the flash hold and drop the
    /// snapshot once its last hold has run.
    pub(crate) fn release_message_window_flash_beats(&mut self, released: usize) {
        let hold = &mut self.message_window_flash_hold;
        if !hold.is_active() {
            return;
        }
        let released = u16::try_from(released).unwrap_or(u16::MAX);
        hold.pending_beats = hold.pending_beats.saturating_sub(released);
        if hold.pending_beats == 0 || self.staged_narration.is_empty() {
            hold.entries = None;
            hold.slot = None;
            hold.pending_beats = 0;
        }
    }

    /// The transcript rows the message window shows: the frozen rows while
    /// a spell flash holds it, the live transcript otherwise.
    pub fn displayed_message_entries(&self) -> &[MessageEntry] {
        self.message_window_flash_hold
            .entries
            .as_deref()
            .unwrap_or(&self.message_transcript)
    }

    /// The unflushed slot text the window draws under
    /// [`Self::displayed_message_entries`], if any.
    pub fn displayed_unflushed_message(&self) -> Option<&str> {
        if self.message_window_flash_hold.is_active() {
            return self.message_window_flash_hold.slot.as_deref();
        }
        self.message_slot_needs_flush().then_some(self.message.as_str())
    }

    /// Whether a spell flash currently freezes the message window.
    pub fn message_window_flash_hold_active(&self) -> bool {
        self.message_window_flash_hold.is_active()
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

    /// `audio.md §7.5` "Movement stings": a town or overworld directional
    /// command's foot or horse marker fires this at movement-dispatch
    /// entry, "before direction processing and destination acceptance" -
    /// success or refusal alike. Ships, skiffs, carpets and dungeon
    /// movement select nothing here.
    pub(crate) fn emit_top_down_movement_sting_if_applicable(&mut self) {
        let selects = matches!(self.area, Area::Town { .. } | Area::World { .. })
            && (self.player.transport.is_foot() || self.player.transport.is_horse());
        if selects {
            self.emit_sound_effect(SoundEffect::TopDownMovementSting);
        }
    }

    /// Run the `audio.md §8.4` shared major full-viewport flash.
    ///
    /// Eight rounds of four 58-band sweeps: 1,856 band draws and 1,856
    /// frequency changes. Every band consumes one **gameplay** PRNG draw, so
    /// this must be called at the published boundary whether or not sound is
    /// audible — muting suppresses each tone start but skips none of the
    /// advances. Shrine restoration and a recognized Word of Power share it;
    /// whether a successful Shadowlord destruction invokes it at all "is
    /// **not established**" (u5-spec#296, `RETRACTIONS.md` R523), so the shard
    /// handler does not call it.
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
        let mut state = match options.target {
            PlayTarget::Town(scene) => Self::load_town_scene(game_dir, scene, options),
            PlayTarget::Dungeon(scene) => Self::load_dungeon_scene(game_dir, scene, options),
            PlayTarget::World(plane) => Self::load_world_scene(game_dir, plane, options),
        }?;
        // The message window a load hands to the world loop is empty but
        // its cursor is at the bottom row, so the first output lands
        // there and everything after it scrolls. `text-output.md §3`:
        // clearing a window does not move its cursor, and the intro
        // screens leave it at the bottom. `cleak/u5-engine#11`; asked on
        // the spec side as `cleak/u5-spec#210`.
        state.message_window_top_offset = MESSAGE_WINDOW_LOAD_CURSOR_ROWS;
        // `animation.md §13.2`: the gating byte ships in the freeze state,
        // so the session's first world step is a bare repaint.
        state.session_start_world_step_frozen = true;
        Ok(state)
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
                // `save-load.md §5.2`: the prompt is its own published
                // literal; the inline syntax hint was the engine's.
                self.message = SAVE_PROMPT_MESSAGE.to_string();
                Ok(MoveOutcome::PromptDeclined)
            }
            Some(false) => {
                // `save-load.md §5.2` step 1: "On `N`, it prints `No`" and
                // returns. Like the `Yes` reply it lands on the still-open
                // `Save game?` line.
                if !self.complete_open_direction_echo(
                    SAVE_PROMPT_MESSAGE,
                    &format!(" {SAVE_PROMPT_NO_REPLY}"),
                ) && !self.complete_open_direction_echo(SAVE_PROMPT_LINE, SAVE_PROMPT_NO_REPLY)
                {
                    self.message = SAVE_PROMPT_NO_REPLY.to_string();
                }
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
        if let Some(pre_combat_scene) = self.saved_scene_scratch {
            save[SAVE_SAVED_SCENE_SCRATCH_OFFSET] = pre_combat_scene;
        }
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
        save[SAVE_RARE_REAGENT_HARVEST_COOKIES_OFFSET
            ..SAVE_RARE_REAGENT_HARVEST_COOKIES_OFFSET + RARE_REAGENT_HARVEST_POINT_COUNT]
            .copy_from_slice(&self.rare_reagent_harvest_days);
        // `formats/saved-gam.md` §10, record 15: the gate at `0x0241` is
        // "Not a dedicated cookie. This is the **equipment-inventory counter
        // for item id `39` (Glass Sword)** from Section 7". It is inside the
        // equipment-stock block written above, so there is deliberately no
        // second write here - one used to clobber the Glass Sword counter.
        save[SAVE_SHADOWLORD_HIDEOUTS_OFFSET..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
            .copy_from_slice(&self.shadowlord_hideouts);
        // `formats/saved-gam.md §9.1`: `0x0325` is the last-summoned
        // Shadowlord id, saved because it sits in resident memory (§10
        // `0x0322..0x0327`). Nothing clears it once a summon succeeds
        // (u5-spec#300 Q3, R585), so every later save writes that id; with
        // no summon yet in this game the new-game `0xFF` stands, and other
        // values are preserved byte-for-byte from the template (playtest
        // G23).
        match self.summoned_shadowlord.and_then(|index| u8::try_from(index).ok()) {
            Some(index) => save[SAVE_ACTIVE_SHADOWLORD_OFFSET] = index,
            None if usize::from(save[SAVE_ACTIVE_SHADOWLORD_OFFSET]) < SHADOWLORD_COUNT => {
                save[SAVE_ACTIVE_SHADOWLORD_OFFSET] = SAVE_ACTIVE_SHADOWLORD_NONE;
            }
            None => {}
        }
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
        // `formats/saved-gam.md §4`: a loaded raw marker the transport still
        // decodes from is written back as it was (playtest N25).
        let preserved_marker = self.preserved_raw_transport_marker();
        save[SAVE_TRANSPORT_MARKER_OFFSET] = if let Some(raw) = preserved_marker {
            raw
        } else if self.player.transport.is_foot() {
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
            // `formats/saved-gam.md §3.1` gender byte. The inn's Leave/Pickup
            // compaction moves whole records between roster slots, so a
            // record's gender must travel with it rather than stay at the
            // slot the template had it in (playtest E43). Only a roster that
            // came from a save carries real bytes to write.
            if !self.party_roster.is_empty() {
                save[record + SAVE_CHARACTER_GENDER_OFFSET] = roster_record.gender;
                // `formats/saved-gam.md §3.3` (u5-spec#299 Q14, R569): "Every
                // byte (`+0x18`, equipment, stay counter, marker) moves with
                // its character; no byte stays with a slot". Left to the
                // template, the defense byte and the marker stayed with the
                // slot when Leave or Pickup rotated the records.
                save[record + crate::SAVE_CHARACTER_DEFENSE_BYTE_OFFSET] = roster_record.defense;
                save[record + crate::SAVE_CHARACTER_INN_MARKER_OFFSET] = roster_record.inn_marker;
            }
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
            // aged only by *the time system* at the 28-day rollover (one is
            // added below 25; 25 or more is left unchanged, R587). Clamping again on write mutates an inherited
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
        // `formats/saved-gam.md §12`: "The final 2,220 bytes,
        // `0x07B4..0x105F`, contain the current location's live NPC family
        // ... **An active-object-only writer cannot restore a working town
        // cast.**" `RETRACTIONS.md` R341: a producer that "declines to
        // persist the band at all ... resumes a **completely empty
        // location**", because "the per-tick NPC walker skips every roster
        // slot whose type byte is zero and the type array was never
        // loaded".
        //
        // §12.3 scopes the write: "A town-family scene (`1..32` at file
        // `0x02ED`) reaches the **preserving** entry mode", the only mode
        // for which the band is a live cast. A world save (`scene 0`) or a
        // dungeon save (`scene > 32`) leaves every band byte at its
        // template value; the three dungeon-mode bytes at `0x105C..0x105E`
        // are "dungeon-mode state, not an NPC field" and the engine has no
        // live source for them in any scene.
        if scene_byte_is_town_family(scene) && matches!(self.area, Area::Town { .. }) {
            write_npc_band(&mut save, &self.npcs, self.active_objects.len())?;
        }
        // `formats/saved-gam.md §12.1`: `0x105D` is the "Dungeon facing",
        // and `dungeon-mode.md §16` lists the facing among the dungeon state
        // that survives save and load. The on-foot transport marker holds
        // only two facing values, so it cannot carry it (playtest E11).
        if let Area::Dungeon { .. } = self.area
            && let Some(code) = dungeon_facing_save_code(self.player.facing)
        {
            save[SAVE_NPC_BAND_DUNGEON_FACING_OFFSET] = code;
        }

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
        let saved_npc_band = options.saved_npc_band.clone();
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
            _ => options.saved_player_aux.unwrap_or((0, 0)),
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
            cleanup_previous_hour: options.cleanup_previous_hour,
            twelve_hour_audio_repeats: options.twelve_hour_audio_repeats,
            ambient_audio_sub_tick: 0,
            dungeon_loop_minute_charged: false,
            dungeon_loop_iteration: 0,
            dungeon_flourish_draw: None,
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
            top_down_previous_viewport: None,
            dungeon_fountain_frame: 0,
            dungeon_monster_previous_cell: None,
            pending_dungeon_surface_exit: None,
            natural_moongate_counter: options.natural_moongate_counter,
            natural_moongate_live_cells: Vec::new(),
            last_natural_moongate_transit: None,
            pending_map_viewport_dissolves: Vec::new(),
            pending_blackthorn_rescue_playbacks: Vec::new(),
            pending_combat_terrain_reveals: Vec::new(),
            pending_potion_flash: None,
            message_window_flash_hold: MessageWindowFlashHold::default(),
            session_start_world_step_frozen: false,
            pending_stonegate_trapdoor_playback: None,
            pending_town_status_provision_pass: false,
            pending_town_npc_schedule_pass: false,
            transport_walker_gate_parity: false,
            quickness_walker_gate_parity: false,
            world_walkers_ran_this_turn: false,
            under_sail_wait_cursor_poll_pending: false,
            pending_town_active_object_pass: false,
            pending_town_active_object_animate_pass: false,
            pending_world_object_walker_pass: false,
            // `formats/saved-gam.md §5.1` (spec `0170809`): the pair is
            // restored from `0x02DF`/`0x02E0` and then rewritten by the
            // scene-entry moon-strip refresh - `RETRACTIONS.md` R343: "The
            // scene-entry callers are also the mechanism by which the
            // cached glyph digits are refreshed on a **Journey Onward**".
            // In a scene outside the surface/town family the renderer
            // returns at its scene gate before it writes anything
            // (`moons.md §3`: "the renderer returns before it reaches
            // either cache write, so **nothing** is cached there"), so
            // there "the pair the save was loaded with stands untouched
            // until the party next enters a scene that shows the strip".
            //
            // §5.1 also forbids the recompute this merge replaced: the pair
            // is "gameplay state, not scratch", and "An engine that treats
            // the pair as disposable, or that recomputes the phase by a
            // different route on load, sends the party through the wrong
            // gate."
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
            party_combat_defense: options.party_combat_defense,
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
            summoned_shadowlord: options.summoned_shadowlord,
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
            pending_blackthorn_merciful_death: false,
            pending_blackthorn_execution_swell: false,
            pending_blackthorn_execution_victim: None,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            camp_cooldown: options.camp_cooldown,
            camp_month_cookie: options.camp_month_cookie,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_action_result: 0,
            combat_wound_classified_at_hit: None,
            combat_pending_divide_line: None,
            combat_pending_hazard_lines: Vec::new(),
            combat_interference_sources: options.combat_interference_sources,
            combat_active: false,
            pace_combat_presentations: false,
            harness_shortcut_keys: true,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            combat_transcript_row_open: false,
            combat_prompt_row_opened_by_banner: false,
            view_overlay_row_opened_by_result: false,
            view_overlay_idle_polls: 0,
            surface_command_row_follows_history: false,
            overworld_prompt_pair_suppressed: false,
            town_hiccup_echo_continues: false,
            stats_panel_window_inverse: false,
            saved_scene_scratch: None,
            preserved_transport_marker: options
                .loaded_transport_marker
                .map(|marker| (marker, options.transport)),
            staged_narration: Default::default(),
            pending_text_hold: None,
            pending_blackthorn_rescue: None,
            pending_blackthorn_audience_exit: false,
            pending_shrine_third_mantra: None,
            blackthorn_audience_punished: false,
            message_row_open_mid_line: false,
            pending_entry_leaves_row_open: false,
            keys_buffered_during_hold: Default::default(),
            stats_panel: Default::default(),
            stats_panel_hold: None,
            use_picker_panel: None,
            staged_stats_panels: Default::default(),
            stats_panel_refresh_requested: false,
            stats_panel_command_refresh: Default::default(),
            stats_panel_command_baseline: Default::default(),
            pending_cast_argument: None,
            cast_resources_spent: false,
            active_combat_targeting: None,
            combat_remembered_targets: [None; COMBAT_ACTOR_SLOTS],
            pending_combat_terrain_trigger_slot: None,
            pending_town_conflict: None,
            pending_town_npc_contact: None,
            pending_outdoor_reaction_slots: Vec::new(),
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
            combat_cursor_blink: false,
            combat_round_loop_prologue_ran: false,
            tile_restoration_pending: false,
            pending_driver_tile_graphics_restores: 0,
            pending_driver_tile_graphics_saves: 0,
            party_stats_panel_refresh_pending: false,
            combat_aim_marker_cell: None,
            combat_aim_marker_gate: false,
            combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_cached_direction: None,
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
            message_window_top_offset: 0,
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
            endgame_entry_redraw_pending: false,
            // Cache `LOOK2.DAT` here rather than in `load_scene`, so a
            // runtime area transition - walking through a town door -
            // gets it too. A scene that loads without the file is not an
            // error; every caller treats `None` as "no description".
            look_table: crate::load_look_table(game_dir).ok(),
            diagnostics: Vec::new(),
            diagnostics_serial: 0,
            endgame_absorption_arena: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            combat_entry_chrome: None,
            cutscene_black_viewport: false,
        blackthorn_rescue_reveals_shown: 0,
        blackthorn_rescue_spectral_actor: ActiveObject::empty(),
        blackthorn_rescue_animation_tick: None,
        blackthorn_audience_actor_variants: [0; 9],
        blackthorn_audience_composite_tick: None,
        shrine_presentation_map: None,
        shrine_approach_walk: None,
        presentation_viewport_inverted: false,
        presentation_camp_flash_inverted: false,
            active_shop: None,
            arms_transaction_completed: false,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_npc_slot: None,
            talk_entered_by_contact: false,
            pending_talk_shop_turn: false,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_party_selector: None,
            active_ready: None,
            exploration_acted_charge_owed: false,
            dungeon_acted_turn_before: None,
            active_use: None,
            active_cast: None,
            active_cast_followup: None,
            active_rest: None,
            active_jimmy: None,
            active_surface_chest: None,
            active_shrine: None,
            codex_presentation_steps: 0,
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
        if let (true, Some(band)) = (has_saved_active_objects, saved_npc_band) {
            state.restore_scheduled_npcs_from_save_band(band);
        } else if has_saved_active_objects {
            state.load_scheduled_npcs_from_existing_active_objects(&npc_slots);
        } else {
            state.load_scheduled_npcs(&npc_slots);
        }
        // `town-mode.md §13`: "a settlement that hosts a Shadowlord prints
        // one line naming which [...] That single line is the only in-town
        // notice, and a settlement hosting none prints nothing."
        // `town-mode.md §13` (u5-spec#300 Q4, R590): the Sceptre flag gates
        // a silent removal of Stonegate's Sceptre figure in the setup path.
        state.remove_stonegate_sceptre_figure_on_entry();
        if let Some((slot, index)) = state.install_shadowlord_entry_encounter() {
            // `town-mode.md §13` (u5-spec#300 Q4): no notice and no tone when
            // the capability check finds nobody able to act and none asleep.
            let line = Self::shadowlord_air_line_for_index(index)
                .filter(|_| state.shadowlord_notice_capability_gate_passes());
            if let Some(line) = line {
                if !state.message.is_empty() {
                    state.message.push('\n');
                }
                state.message.push_str(&line);
                // `town-mode.md §13` (u5-spec#300 Q4, Q17): "After the text,
                // the notice plays one software envelope".
                state.emit_sound_effect(SoundEffect::ShadowlordPresenceNotice);
            }
            // The installed actor's slot is an engine internal. It used to be
            // appended to the notice as a second sentence, which put a line
            // the original never prints into the message window.
            let title = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
            state.diagnostics.push(match slot {
                Some(slot) => {
                    format!("{title} resident; actor installed in active-object slot {slot}")
                }
                None => format!("{title} resident; actor rejected by the one-at-a-time gate"),
            });
        }
        // `moons.md §3`, caller census: the town-family floor loader
        // repaints on "Every town-family scene entry **and every in-place
        // floor change inside a location**", and "Each refresh caches the
        // two glyph bytes for the current day *before* it tests whether
        // either marker is on the visible horizon". The hour-change hook
        // is the only row that also tests the floor - "This is the only
        // caller carrying that below-surface gate (Section 2.2)." - so a
        // basement entry refreshes the pair too.
        state.refresh_cached_moon_glyphs_at_scene_entry();
        state.mode_zero_cleanup();
        state.mark_visibility_dirty();
        state.append_stonegate_entry_presentation_message();
        // `stats-panel.md §2.2`'s boot-time first paint: one of the eighty
        // direct painter calls, and the reason the panel is correct before
        // any command has run.
        state.repaint_stats_panel();
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
        // `formats/saved-gam.md §8.1`: bytes `+0x05`/`+0x07` of slot zero
        // round-trip from the file (playtest G27).
        let (player_aux1, player_aux3) = options.saved_player_aux.unwrap_or((0, 0));
        let mut active_objects = vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: level as i8,
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: player_aux1,
            aux3: player_aux3,
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
                // `active-objects.md §10` (issue #299 Q7): "loading a save
                // restores the state exactly as saved, with no
                // normalization when resuming into a dungeon ... a save
                // image carrying another transport value into a dungeon
                // scene resumes with it" (playtest N25). Every in-session
                // dungeon entry forces foot travel after this load.
                transport: options.transport,
            },
            active_objects,
            npcs: Vec::new(),
            // `save-load.md §4.2`: "The load path recomputes and normalises
            // no restored byte", and only town location setup clears the
            // previous-tile gate (`doors-and-z-transitions.md §5`). Outside
            // a town the four bytes ride along untouched (playtest N23).
            door_tracker: options.door_tracker,
            door_tracker_closed: false,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            grid,
            world_live_chunks: None,
            clock: options.clock,
            cleanup_previous_hour: options.cleanup_previous_hour,
            twelve_hour_audio_repeats: options.twelve_hour_audio_repeats,
            ambient_audio_sub_tick: 0,
            dungeon_loop_minute_charged: false,
            dungeon_loop_iteration: 0,
            dungeon_flourish_draw: None,
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
            top_down_previous_viewport: None,
            dungeon_fountain_frame: 0,
            dungeon_monster_previous_cell: None,
            pending_dungeon_surface_exit: None,
            natural_moongate_counter: options.natural_moongate_counter,
            natural_moongate_live_cells: Vec::new(),
            last_natural_moongate_transit: None,
            pending_map_viewport_dissolves: Vec::new(),
            pending_blackthorn_rescue_playbacks: Vec::new(),
            pending_combat_terrain_reveals: Vec::new(),
            pending_potion_flash: None,
            message_window_flash_hold: MessageWindowFlashHold::default(),
            session_start_world_step_frozen: false,
            pending_stonegate_trapdoor_playback: None,
            pending_town_status_provision_pass: false,
            pending_town_npc_schedule_pass: false,
            transport_walker_gate_parity: false,
            quickness_walker_gate_parity: false,
            world_walkers_ran_this_turn: false,
            under_sail_wait_cursor_poll_pending: false,
            pending_town_active_object_pass: false,
            pending_town_active_object_animate_pass: false,
            pending_world_object_walker_pass: false,
            // `formats/saved-gam.md §5.1` (spec `0170809`): the pair is
            // restored from `0x02DF`/`0x02E0` and then rewritten by the
            // scene-entry moon-strip refresh - `RETRACTIONS.md` R343: "The
            // scene-entry callers are also the mechanism by which the
            // cached glyph digits are refreshed on a **Journey Onward**".
            // In a scene outside the surface/town family the renderer
            // returns at its scene gate before it writes anything
            // (`moons.md §3`: "the renderer returns before it reaches
            // either cache write, so **nothing** is cached there"), so
            // there "the pair the save was loaded with stands untouched
            // until the party next enters a scene that shows the strip".
            //
            // §5.1 also forbids the recompute this merge replaced: the pair
            // is "gameplay state, not scratch", and "An engine that treats
            // the pair as disposable, or that recomputes the phase by a
            // different route on load, sends the party through the wrong
            // gate."
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
            party_combat_defense: options.party_combat_defense,
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
            summoned_shadowlord: options.summoned_shadowlord,
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
            pending_blackthorn_merciful_death: false,
            pending_blackthorn_execution_swell: false,
            pending_blackthorn_execution_victim: None,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            camp_cooldown: options.camp_cooldown,
            camp_month_cookie: options.camp_month_cookie,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_action_result: 0,
            combat_wound_classified_at_hit: None,
            combat_pending_divide_line: None,
            combat_pending_hazard_lines: Vec::new(),
            combat_interference_sources: options.combat_interference_sources,
            combat_active: false,
            pace_combat_presentations: false,
            harness_shortcut_keys: true,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            combat_transcript_row_open: false,
            combat_prompt_row_opened_by_banner: false,
            view_overlay_row_opened_by_result: false,
            view_overlay_idle_polls: 0,
            surface_command_row_follows_history: false,
            overworld_prompt_pair_suppressed: false,
            town_hiccup_echo_continues: false,
            stats_panel_window_inverse: false,
            saved_scene_scratch: None,
            preserved_transport_marker: options
                .loaded_transport_marker
                .map(|marker| (marker, options.transport)),
            staged_narration: Default::default(),
            pending_text_hold: None,
            pending_blackthorn_rescue: None,
            pending_blackthorn_audience_exit: false,
            pending_shrine_third_mantra: None,
            blackthorn_audience_punished: false,
            message_row_open_mid_line: false,
            pending_entry_leaves_row_open: false,
            keys_buffered_during_hold: Default::default(),
            stats_panel: Default::default(),
            stats_panel_hold: None,
            use_picker_panel: None,
            staged_stats_panels: Default::default(),
            stats_panel_refresh_requested: false,
            stats_panel_command_refresh: Default::default(),
            stats_panel_command_baseline: Default::default(),
            pending_cast_argument: None,
            cast_resources_spent: false,
            active_combat_targeting: None,
            combat_remembered_targets: [None; COMBAT_ACTOR_SLOTS],
            pending_combat_terrain_trigger_slot: None,
            pending_town_conflict: None,
            pending_town_npc_contact: None,
            pending_outdoor_reaction_slots: Vec::new(),
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
            combat_cursor_blink: false,
            combat_round_loop_prologue_ran: false,
            tile_restoration_pending: false,
            pending_driver_tile_graphics_restores: 0,
            pending_driver_tile_graphics_saves: 0,
            party_stats_panel_refresh_pending: false,
            combat_aim_marker_cell: None,
            combat_aim_marker_gate: false,
            combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_cached_direction: None,
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
            message_window_top_offset: 0,
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
            endgame_entry_redraw_pending: false,
            // Cache `LOOK2.DAT` here rather than in `load_scene`, so a
            // runtime area transition - walking through a town door -
            // gets it too. A scene that loads without the file is not an
            // error; every caller treats `None` as "no description".
            look_table: crate::load_look_table(game_dir).ok(),
            diagnostics: Vec::new(),
            diagnostics_serial: 0,
            endgame_absorption_arena: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            combat_entry_chrome: None,
            cutscene_black_viewport: false,
        blackthorn_rescue_reveals_shown: 0,
        blackthorn_rescue_spectral_actor: ActiveObject::empty(),
        blackthorn_rescue_animation_tick: None,
        blackthorn_audience_actor_variants: [0; 9],
        blackthorn_audience_composite_tick: None,
        shrine_presentation_map: None,
        shrine_approach_walk: None,
        presentation_viewport_inverted: false,
        presentation_camp_flash_inverted: false,
            active_shop: None,
            arms_transaction_completed: false,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_npc_slot: None,
            talk_entered_by_contact: false,
            pending_talk_shop_turn: false,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_party_selector: None,
            active_ready: None,
            exploration_acted_charge_owed: false,
            dungeon_acted_turn_before: None,
            active_use: None,
            active_cast: None,
            active_cast_followup: None,
            active_rest: None,
            active_jimmy: None,
            active_surface_chest: None,
            active_shrine: None,
            codex_presentation_steps: 0,
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
        // `stats-panel.md §2.2`'s boot-time first paint: one of the eighty
        // direct painter calls, and the reason the panel is correct before
        // any command has run.
        state.repaint_stats_panel();
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
        // Underworld there is no canonical fresh-start spawn; (134, 2) is an
        // open grass cell of the correctly de-chunked map, which neither the
        // old (1, 1) seed nor (100, 100) is - they only looked open while
        // `UNDER.DAT` was being read as though it were already row-major.
        let default_start = match plane {
            WorldPlane::Britannia => (62, 124),
            WorldPlane::Underworld => (134, 2),
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
        // `formats/saved-gam.md §8.1`: slot zero "writes nothing to +0x05,
        // +0x06 or +0x07. Those three round-trip from the file", so a loaded
        // party off a ship keeps the saved pair rather than zeros
        // (playtest G27). Aboard, the ship transport decoded the same bytes.
        let (player_aux1, player_aux3) = match transport {
            TransportState::Ship { hull, skiffs, .. } => (hull, skiffs),
            _ => options.saved_player_aux.unwrap_or((0, 0)),
        };
        let mut active_objects = vec![ActiveObject {
            type_byte: marker,
            tile: marker,
            x,
            y,
            z: plane.save_floor(),
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: player_aux1,
            aux3: player_aux3,
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
            // `save-load.md §4.2`: "The load path recomputes and normalises
            // no restored byte", and only town location setup clears the
            // previous-tile gate (`doors-and-z-transitions.md §5`). Outside
            // a town the four bytes ride along untouched (playtest N23).
            door_tracker: options.door_tracker,
            door_tracker_closed: false,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability,
            grid,
            world_live_chunks,
            clock: options.clock,
            cleanup_previous_hour: options.cleanup_previous_hour,
            twelve_hour_audio_repeats: options.twelve_hour_audio_repeats,
            ambient_audio_sub_tick: 0,
            dungeon_loop_minute_charged: false,
            dungeon_loop_iteration: 0,
            dungeon_flourish_draw: None,
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
            top_down_previous_viewport: None,
            dungeon_fountain_frame: 0,
            dungeon_monster_previous_cell: None,
            pending_dungeon_surface_exit: None,
            natural_moongate_counter: options.natural_moongate_counter,
            natural_moongate_live_cells: Vec::new(),
            last_natural_moongate_transit: None,
            pending_map_viewport_dissolves: Vec::new(),
            pending_blackthorn_rescue_playbacks: Vec::new(),
            pending_combat_terrain_reveals: Vec::new(),
            pending_potion_flash: None,
            message_window_flash_hold: MessageWindowFlashHold::default(),
            session_start_world_step_frozen: false,
            pending_stonegate_trapdoor_playback: None,
            pending_town_status_provision_pass: false,
            pending_town_npc_schedule_pass: false,
            transport_walker_gate_parity: false,
            quickness_walker_gate_parity: false,
            world_walkers_ran_this_turn: false,
            under_sail_wait_cursor_poll_pending: false,
            pending_town_active_object_pass: false,
            pending_town_active_object_animate_pass: false,
            pending_world_object_walker_pass: false,
            // `formats/saved-gam.md §5.1` (spec `0170809`): the pair is
            // restored from `0x02DF`/`0x02E0` and then rewritten by the
            // scene-entry moon-strip refresh - `RETRACTIONS.md` R343: "The
            // scene-entry callers are also the mechanism by which the
            // cached glyph digits are refreshed on a **Journey Onward**".
            // In a scene outside the surface/town family the renderer
            // returns at its scene gate before it writes anything
            // (`moons.md §3`: "the renderer returns before it reaches
            // either cache write, so **nothing** is cached there"), so
            // there "the pair the save was loaded with stands untouched
            // until the party next enters a scene that shows the strip".
            //
            // §5.1 also forbids the recompute this merge replaced: the pair
            // is "gameplay state, not scratch", and "An engine that treats
            // the pair as disposable, or that recomputes the phase by a
            // different route on load, sends the party through the wrong
            // gate."
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
            party_combat_defense: options.party_combat_defense,
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
            summoned_shadowlord: options.summoned_shadowlord,
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
            pending_blackthorn_merciful_death: false,
            pending_blackthorn_execution_swell: false,
            pending_blackthorn_execution_victim: None,
            active_effect_tag: options.active_effect_tag,
            active_effect_counter: options.active_effect_counter,
            fortunes_of_war: options.fortunes_of_war,
            camp_cooldown: options.camp_cooldown,
            camp_month_cookie: options.camp_month_cookie,
            active_player: options.active_player,
            combat_round_counter: options.combat_round_counter,
            combat_action_result: 0,
            combat_wound_classified_at_hit: None,
            combat_pending_divide_line: None,
            combat_pending_hazard_lines: Vec::new(),
            combat_interference_sources: options.combat_interference_sources,
            combat_active: false,
            pace_combat_presentations: false,
            harness_shortcut_keys: true,
            combat_frame_snapshot: None,
            pending_combat_actor_slot: None,
            combat_transcript_row_open: false,
            combat_prompt_row_opened_by_banner: false,
            view_overlay_row_opened_by_result: false,
            view_overlay_idle_polls: 0,
            surface_command_row_follows_history: false,
            overworld_prompt_pair_suppressed: false,
            town_hiccup_echo_continues: false,
            stats_panel_window_inverse: false,
            saved_scene_scratch: None,
            preserved_transport_marker: options
                .loaded_transport_marker
                .map(|marker| (marker, options.transport)),
            staged_narration: Default::default(),
            pending_text_hold: None,
            pending_blackthorn_rescue: None,
            pending_blackthorn_audience_exit: false,
            pending_shrine_third_mantra: None,
            blackthorn_audience_punished: false,
            message_row_open_mid_line: false,
            pending_entry_leaves_row_open: false,
            keys_buffered_during_hold: Default::default(),
            stats_panel: Default::default(),
            stats_panel_hold: None,
            use_picker_panel: None,
            staged_stats_panels: Default::default(),
            stats_panel_refresh_requested: false,
            stats_panel_command_refresh: Default::default(),
            stats_panel_command_baseline: Default::default(),
            pending_cast_argument: None,
            cast_resources_spent: false,
            active_combat_targeting: None,
            combat_remembered_targets: [None; COMBAT_ACTOR_SLOTS],
            pending_combat_terrain_trigger_slot: None,
            pending_town_conflict: None,
            pending_town_npc_contact: None,
            pending_outdoor_reaction_slots: Vec::new(),
            next_combat_actor_slot: 0,
            combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
            combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
            combat_cursor_blink: false,
            combat_round_loop_prologue_ran: false,
            tile_restoration_pending: false,
            pending_driver_tile_graphics_restores: 0,
            pending_driver_tile_graphics_saves: 0,
            party_stats_panel_refresh_pending: false,
            combat_aim_marker_cell: None,
            combat_aim_marker_gate: false,
            combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
            combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            sail_cadence: 0,
            sail_cached_direction: None,
            pending_vehicle_save,
            turn: 0,
            // As above for the coordinate half. The wind half is a duplicate:
            // `text-output.md §10.7` puts the prevailing wind in the
            // viewport's bottom ribbon, which `gameplay_chrome` already
            // draws from the same state, so printing it here would show the
            // player the banner twice.
            message: String::new(),
            message_transcript: Vec::new(),
            message_window_top_offset: 0,
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
            endgame_entry_redraw_pending: false,
            // Cache `LOOK2.DAT` here rather than in `load_scene`, so a
            // runtime area transition - walking through a town door -
            // gets it too. A scene that loads without the file is not an
            // error; every caller treats `None` as "no description".
            look_table: crate::load_look_table(game_dir).ok(),
            diagnostics: Vec::new(),
            diagnostics_serial: 0,
            endgame_absorption_arena: None,
            active_blackthorn: None,
            blackthorn_audience_map: None,
            combat_entry_chrome: None,
            cutscene_black_viewport: false,
        blackthorn_rescue_reveals_shown: 0,
        blackthorn_rescue_spectral_actor: ActiveObject::empty(),
        blackthorn_rescue_animation_tick: None,
        blackthorn_audience_actor_variants: [0; 9],
        blackthorn_audience_composite_tick: None,
        shrine_presentation_map: None,
        shrine_approach_walk: None,
        presentation_viewport_inverted: false,
        presentation_camp_flash_inverted: false,
            active_shop: None,
            arms_transaction_completed: false,
            common_word_dictionary: None,
            active_conversation: None,
            active_conversation_npc_slot: None,
            talk_entered_by_contact: false,
            pending_talk_shop_turn: false,
            active_conversation_join_candidate: None,
            active_z_stats: None,
            active_party_selector: None,
            active_ready: None,
            exploration_acted_charge_owed: false,
            dungeon_acted_turn_before: None,
            active_use: None,
            active_cast: None,
            active_cast_followup: None,
            active_rest: None,
            active_jimmy: None,
            active_surface_chest: None,
            active_shrine: None,
            codex_presentation_steps: 0,
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
        // `moons.md §3`, caller census: the overworld entry pass repaints
        // on "Every entry into the overworld framer, whether that is a
        // fresh scene entry or an in-place return", and "Each refresh
        // caches the two glyph bytes for the current day *before* it
        // tests whether either marker is on the visible horizon". The
        // hour-change hook is the only row that also tests the floor -
        // "This is the only caller carrying that below-surface gate
        // (Section 2.2)." - so an Underworld entry refreshes the pair too.
        state.refresh_cached_moon_glyphs_at_scene_entry();
        state.mode_zero_cleanup();
        state.mark_visibility_dirty();
        // `stats-panel.md §2.2`'s boot-time first paint: one of the eighty
        // direct painter calls, and the reason the panel is correct before
        // any command has run.
        state.repaint_stats_panel();
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
        // `audio.md §7.5`: the movement sting fires "at movement-dispatch
        // entry, before direction processing and destination acceptance" -
        // ahead of even the diagonal refusal below, since that refusal is
        // itself part of "direction processing".
        self.emit_top_down_movement_sting_if_applicable();
        // `movement.md §2`: "No mode steps diagonally." World and town
        // movement consume the four cardinal directions only, so a diagonal
        // is refused before the facing is written. The dungeon step keeps its
        // own refusal below so its message stays mode-local.
        if !direction.is_cardinal() && !matches!(self.area, Area::Dungeon { .. }) {
            return Ok(MoveOutcome::Blocked);
        }
        // `text-output.md §10.2` (R526): a direction taken under hoisted sail,
        // keyed or synthesised, clears the overworld prompt flag; a facing
        // change or an under-sail refusal sets it again below.
        self.note_overworld_direction_taken();
        // `vehicles.md §6` (u5-spec#297): a frigate key that changes the
        // facing turns the ship in place and does not move it, in either sail
        // state. Read the facing before this step rewrites it.
        let previous_frigate_facing = self.overworld_frigate_facing();
        self.player.facing = direction;
        let previous_avatar_tile = self.player.transport.avatar_tile();
        self.player.transport = self.player.transport.with_facing(direction);
        if self.player.transport.avatar_tile() != previous_avatar_tile {
            self.sync_player_object();
            self.mark_visibility_dirty();
        }
        if previous_frigate_facing.is_some_and(|facing| facing != direction) {
            return Ok(self.turn_overworld_frigate_in_place(direction));
        }
        let (dx, dy) = direction.delta();
        let nx = self.player.x as isize + dx;
        let ny = self.player.y as isize + dy;
        let Area::Town { scene, floor } = self.area else {
            return self.step_non_town(direction, nx, ny, game_dir);
        };

        if !(0..32).contains(&nx) || !(0..32).contains(&ny) {
            // `town-mode.md §15`, "Where the substitution comes from, and who
            // else takes it" (issue #188 question 3): the substituted byte is
            // "the loaded floor's `(31,31)` cell", and it "differs per floor"
            // - there is no constant off-map tile (`RETRACTIONS.md` R366).
            // The value this reads is therefore right for every shipped exit.
            //
            // The *shape* is not: "the movement handler calls no accessor. It
            // reads the cell already sitting in the party-centred viewport,
            // and the substituted byte is there only because the *grid
            // producer* fetched it through the same accessor when it filled
            // that viewport", with the qualification that "the movement
            // sample sees the substituted byte verbatim only where nothing
            // overwrote that grid cell after the producer filled it". This
            // engine samples the loaded floor page directly, which is the
            // accessor's own value rather than the grid's; the two agree in
            // every ordinary town frame, and the difference is recorded here
            // rather than restructured, because nothing in the shipped
            // content can distinguish them.
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
        // `vehicles.md §4`: B-Board "probes the current position", with a
        // town-only `Nay!` availability check, and `shops.md §8.10` places "a
        // boardable horse object for the party" on an adjacent town cell. A
        // vehicle the current transport could board is therefore a cell the
        // party can step onto (`movement.md §7`), as on the overworld
        // (playtest E22); anything else on the cell still blocks (playtest
        // H01). A schedule NPC's own sprite is not such an object even when
        // it wears the horse frames (`npc-roster.md`: "used for stable and
        // paddock actors and for Smith at Iolo's Hut"), so a linked slot
        // keeps blocking as every other NPC does.
        let onto_boardable_vehicle = self
            .boardable_vehicle_slot_at(nx, ny)
            .is_some_and(|candidate| {
                !candidate.blocked_by_occupant
                    && self.player.transport.can_board(candidate.transport)
                    && !self
                        .npcs
                        .iter()
                        .any(|npc| npc.active_object == Some(candidate.slot))
            });
        if !onto_boardable_vehicle && self.blocking_object_at(nx, ny).is_some() {
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
            // `audio.md §7.5`: a mounted horse's "normal successful in-map
            // step" plays "a second sting after the position changes";
            // ordinary foot movement, exits and stair transitions do not.
            if self.player.transport.is_horse() {
                self.emit_sound_effect(SoundEffect::TopDownMovementSting);
            }
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
    ) -> io::Result<MoveOutcome> {
        let entries = effective_world_location_entries(game_dir)?;
        let matches: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| entry.target == PlayTarget::Town(scene))
            .collect();
        // `catalogs/gazetteer.md` §5.1 (`RETRACTIONS.md` R474): "No per-save
        // return-coordinate datum exists... every legal scene byte has a
        // populated row, so the branch is unreachable and any player-visible
        // message on it has no original counterpart" - the engine's own
        // `missing clean return-coordinate metadata` fallback is exactly that
        // withdrawn branch, and the published/merged table
        // (`effective_world_location_entries`) guarantees this `expect` never
        // fires: all forty rows are present and distinct, and a sidecar can
        // only replace a published row, never remove its coverage.
        let entry = *matches.first().unwrap_or_else(|| {
            panic!(
                "catalogs/gazetteer.md §5.1: {} has no return-coordinate row, \
                 which the published table guarantees cannot happen",
                PlayTarget::Town(scene).key()
            )
        });
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
        // carries then falls before `Exit to`. The row state is set here
        // explicitly rather than inferred (playtest E03).
        self.open_message_row();
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
        // The line closes its own row; see the Stonegate arm of
        // `apply_town_underfoot_effects`, where this was measured.
        self.emit_town_underfoot_line(Self::TOWN_TRAPDOOR_MESSAGE);
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
