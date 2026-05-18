use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

impl PlayState {
    pub fn open_dungeon_chest(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        verb: &str,
    ) -> MoveOutcome {
        let trap_note = if self.dungeon_chest_trap_detail(level, x, y, tile) == "no trap" {
            None
        } else {
            let target_slot = self.shared_trap_default_target_slot();
            Some(self.apply_shared_trap_effect_to_slot(target_slot))
        };
        self.grid[idx] = 0x70 | (tile & 0x0f);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = match trap_note {
            Some(trap) => format!(
                "{verb} dungeon chest at ({x}, {y}) on {} level {level}; {trap}, marked visit-local open chest.",
                scene.key()
            ),
            None => format!(
                "{verb} dungeon chest at ({x}, {y}) on {} level {level}; marked visit-local open chest.",
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
        let class_byte = self
            .party
            .iter()
            .find(|member| member.living())
            .map(|member| member.class_byte)
            .unwrap_or_default();
        let threshold = Self::dungeon_chest_pick_threshold(level, class_byte);
        let roll = self.dungeon_chest_trap_roll(level, x, y, tile, 0, 30);
        let detail = if roll > threshold && Self::is_plain_closed_dungeon_chest(tile) {
            "no trap"
        } else {
            let tier = if roll <= threshold {
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

    pub fn dungeon_chest_pick_threshold(level: u8, class_byte: u8) -> u8 {
        level
            .wrapping_mul(2)
            .wrapping_sub(class_byte)
            .wrapping_add(30)
            / 2
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

    pub fn generate_dungeon_chest_content(
        &mut self,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> String {
        let gate_upper = 4u16 * u16::from(level) + 4;
        let mut parts = Vec::new();

        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 0, 2, gate_upper) {
            let amount = self.dungeon_chest_roll(level, x, y, tile, 0, 1, 31);
            self.food = self.food.saturating_add(u16::from(amount));
            parts.push(format!("{amount} food"));
        }
        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 1, 4, gate_upper) {
            let amount = self.dungeon_chest_gold_roll(level, x, y, tile);
            self.gold = self.gold.saturating_add(u16::from(amount));
            parts.push(format!("{amount} gold"));
        }
        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 2, 5, gate_upper) {
            let amount = self.dungeon_chest_roll(level, x, y, tile, 2, 1, 3);
            self.keys = self.keys.saturating_add(amount);
            parts.push(format!("{amount} keys"));
        }
        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 3, 10, gate_upper) {
            let amount = self.dungeon_chest_roll(level, x, y, tile, 3, 1, 3);
            self.gems = self.gems.saturating_add(amount);
            parts.push(format!("{amount} gems"));
        }
        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 4, 20, gate_upper) {
            let amount = self.dungeon_chest_roll(level, x, y, tile, 4, 1, 3);
            self.torches = self.torches.saturating_add(amount);
            parts.push(format!("{amount} torches"));
        }
        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 5, 25, gate_upper) {
            let subtype = self.dungeon_chest_zero_based_roll(level, x, y, tile, 5, 1, POTION_COUNT);
            self.potion_stock[subtype] = self.potion_stock[subtype].saturating_add(1).min(99);
            parts.push(format!("1 {} potion", potion_label(subtype)));
        }
        if self.dungeon_chest_gate_succeeds(level, x, y, tile, 6, 25, gate_upper) {
            let subtype = self.dungeon_chest_zero_based_roll(level, x, y, tile, 6, 1, SCROLL_COUNT);
            self.scroll_stock[subtype] = self.scroll_stock[subtype].saturating_add(1).min(99);
            parts.push(format!("1 {} scroll", scroll_label(subtype)));
        }

        if parts.is_empty() {
            "generated chest grants nothing".to_string()
        } else {
            format!("generated chest grants {}", parts.join(", "))
        }
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

    pub fn dungeon_chest_gold_roll(&self, level: u8, x: usize, y: usize, tile: u8) -> u8 {
        let upper = 8u16 * u16::from(level);
        if upper == 0 {
            let _ = self.dungeon_chest_roll_seed(level, x, y, tile, 1, 1);
            return 0;
        }
        self.dungeon_chest_roll(level, x, y, tile, 1, 1, upper)
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
        self.player.transport = transport;
        self.timing_status = TimingStatusTag::for_transport(transport);
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
        self.timing_status = TimingStatusTag::Normal;
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
    }

    pub fn free_active_object_slot(&mut self, slot: usize) {
        if slot == 0 {
            return;
        }
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.free();
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
        self.free_active_object_slot(object_slot);
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
        if let Some(slot) = self
            .active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| object.is_empty().then_some(slot))
        {
            self.active_objects[slot] = object;
            return Some(slot);
        }
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.push(object);
            return Some(self.active_objects.len() - 1);
        }
        None
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
        let landing = self.vehicle_exit_landing(game_dir)?;

        // doors-and-z-transitions.md §11 / vehicles.md §5: a furled-ship exit
        // without nearby foot landing falls back to launching a carried skiff.
        // The ship hull stays parked at the original cell with one fewer
        // skiff aboard, and the party becomes the launched skiff in place.
        if landing.is_none() {
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
                    };
                    self.timing_status = TimingStatusTag::for_transport(self.player.transport);
                    self.sail_cadence = 0;
                    self.sail_stall_pending = false;
                    self.sync_player_object();
                    self.mark_visibility_dirty();
                    self.advance_turn();
                    self.message = "Launched a skiff from the ship.".to_string();
                    return Ok(MoveOutcome::ExitedVehicle);
                }
            }
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let (x, y) = landing.expect("landing checked above");
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
        let tile = match self.area {
            Area::Town { .. } => self.grid[self.player.y * 32 + self.player.x],
            Area::World { .. } => self.grid[world_cell_index(self.player.x, self.player.y)],
            Area::Dungeon { .. } => return false,
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
        if matches!(self.player.transport, TransportState::Ship { .. }) {
            return self.toggle_sails();
        }

        let Some(word) = word else {
            self.message = yell_prompt_message();
            return MoveOutcome::PromptDeclined;
        };
        let word = Self::normalize_yell_word(word);
        if word.is_empty() {
            self.message = YELL_NOTHING_SAID_MESSAGE.to_string();
            return MoveOutcome::PromptDeclined;
        }

        self.advance_turn();
        if let Some(dungeon) = Self::word_of_power_dungeon(&word) {
            let context = match self.area {
                Area::Dungeon { .. } => "No matching Word-of-Power seal is present.",
                _ => "This is not a dungeon Word-of-Power seal.",
            };
            self.message = format!("Yelled {word}, the Word of Power for {dungeon}. {context}");
            return MoveOutcome::Used;
        }
        if let Some(index) = Self::shadowlord_name_index(&word) {
            let shadowlord = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
            self.message = if let Some(slot) = self.place_shadowlord_name_encounter(index) {
                format!(
                    "Yelled {word}, the name of {shadowlord}. {shadowlord} appears in active-object slot {slot}."
                )
            } else if self.shadowlord_alive(index) {
                format!("Yelled {word}, the name of {shadowlord}. No Shadowlord answers here.")
            } else {
                format!("Yelled {word}, the name of {shadowlord}. {shadowlord} is vanquished.")
            };
            return MoveOutcome::Used;
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
        (index < SHADOWLORD_COUNT).then_some(SHADOWLORD_OBJECT_TILE_BASE + index as u8)
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
            aux1: index as u8,
            aux3: self.current_shadowlord_hideout_id().unwrap_or(0),
        })
    }

    pub fn shadowlord_name_encounter_index(object: ActiveObject) -> Option<usize> {
        let index = object.aux1 as usize;
        let tile = Self::shadowlord_object_tile_for_index(index)?;
        (!object.is_empty() && object.type_byte == tile && object.tile == tile).then_some(index)
    }

    pub fn shadowlord_name_encounter_present(&self, index: usize) -> bool {
        let Some(floor) = self.current_floor() else {
            return false;
        };
        self.active_objects.iter().copied().any(|object| {
            object.z == floor && Self::shadowlord_name_encounter_index(object) == Some(index)
        })
    }

    pub fn place_shadowlord_name_encounter(&mut self, index: usize) -> Option<usize> {
        let current = self.current_shadowlord_hideout_id()?;
        if self.shadowlord_hideouts.get(index).copied() != Some(current) {
            return None;
        }
        let z = self.current_floor()?;
        let (x, y) = self.shadowlord_name_encounter_position()?;
        let object = self.shadowlord_name_encounter_object(index, x, y, z)?;
        let slot = self.allocate_active_object_slot(object)?;
        self.mark_visibility_dirty();
        Some(slot)
    }

    pub fn install_shadowlord_entry_encounter(&mut self) -> Option<(usize, usize)> {
        let current = self.current_shadowlord_hideout_id()?;
        for index in 0..SHADOWLORD_COUNT {
            if self.shadowlord_hideouts.get(index).copied() != Some(current)
                || self.shadowlord_name_encounter_present(index)
            {
                continue;
            }
            let slot = self.place_shadowlord_name_encounter(index)?;
            return Some((slot, index));
        }
        None
    }

    pub fn shadowlord_name_encounter_position(&self) -> Option<(usize, usize)> {
        [
            self.player.facing,
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
            Direction::NorthEast,
            Direction::SouthEast,
            Direction::SouthWest,
            Direction::NorthWest,
        ]
        .into_iter()
        .find_map(|direction| {
            let (x, y) = self.adjacent_position(direction)?;
            matches!(self.player_can_land_on_foot(None, x, y), Ok(true)).then_some((x, y))
        })
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

    pub fn reroll_shadowlord_hideouts_excluding(&mut self, current: Option<u8>) -> usize {
        let previous = self.shadowlord_hideouts;
        let mut assigned = [0u8; SHADOWLORD_COUNT];
        let mut assigned_len = 0usize;
        let mut rerolled = 0usize;

        for slot in 0..SHADOWLORD_COUNT {
            if !Self::shadowlord_slot_is_living(previous[slot]) {
                continue;
            }

            let mut selected = None;
            for attempt in 0..16 {
                let candidate = Self::shadowlord_hideout_candidate_from_seed(
                    self.shadowlord_hideout_roll_seed(slot, attempt),
                );
                if current == Some(candidate) || assigned[..assigned_len].contains(&candidate) {
                    continue;
                }
                selected = Some(candidate);
                break;
            }

            if selected.is_none() {
                selected = (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX).find(|candidate| {
                    current != Some(*candidate) && !assigned[..assigned_len].contains(candidate)
                });
            }

            if let Some(candidate) = selected {
                self.shadowlord_hideouts[slot] = candidate;
                assigned[assigned_len] = candidate;
                assigned_len += 1;
                rerolled += 1;
            }
        }

        rerolled
    }

    pub fn shadowlord_hideout_candidate_from_seed(seed: u8) -> u8 {
        SHADOWLORD_HIDEOUT_MIN + (seed % (SHADOWLORD_HIDEOUT_MAX - SHADOWLORD_HIDEOUT_MIN + 1))
    }

    pub fn shadowlord_hideout_roll_seed(&self, slot: usize, attempt: u8) -> u8 {
        (self.turn as u8).wrapping_mul(37)
            ^ self.clock.year as u8
            ^ self.clock.month.wrapping_mul(3)
            ^ self.clock.day.wrapping_mul(5)
            ^ self.clock.hour.wrapping_mul(7)
            ^ self.clock.minute.wrapping_mul(11)
            ^ (self.player.x as u8).wrapping_mul(13)
            ^ (self.player.y as u8).wrapping_mul(17)
            ^ (slot as u8).wrapping_mul(19)
            ^ attempt.wrapping_mul(23)
    }

    pub fn terrain_encounter_note(
        &self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        object: ActiveObject,
    ) -> io::Result<String> {
        let Some(arena) = outdoor_combat_arena_index_for_object(object) else {
            return Ok(format!(
                "no terrain-combat arena selected for active-object type 0x{:02X} tile 0x{:02X}",
                object.type_byte, object.tile
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
        let setup = terrain_combat_setup_from_record(plane, object, record)?;
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
            if combat_class_stats_for_sprite_byte(object.tile)
                .or_else(|| combat_class_stats_for_sprite_byte(object.type_byte))
                .is_some()
            {
                let note = self.enter_dungeon_active_monster_combat(level, object)?;
                self.message = format!(
                    "Attacked dungeon monster tile {} at ({x}, {y}) on {} level {level}; {note}.",
                    object.tile,
                    scene.key()
                );
                return Ok(MoveOutcome::Used);
            }
            self.message = format!(
                "Attacked dungeon monster tile {} at ({x}, {y}) on {} level {level}; dungeon combat resolution is pending.",
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
                if let Some((npc_index, npc_slot, object_slot)) =
                    self.town_attack_target_at(floor, x, y)
                {
                    self.free_active_object_slot(object_slot);
                    self.npcs.remove(npc_index);
                    self.mark_removed_town_npc_once(scene, floor, npc_slot);
                    let (fortified, fleeing) =
                        self.town_alarm_sweep(scene, floor, Some(npc_slot));
                    self.mark_visibility_dirty();
                    self.message = format!(
                        "Attacked NPC slot {npc_slot} at ({x}, {y}) to the {}; target removed from {} floor {floor}; alarm raised ({fortified} fortified, {fleeing} fleeing).",
                        direction.name(),
                        scene.key()
                    );
                    return Ok(MoveOutcome::Used);
                }
                self.message = format!(
                    "Attacked object tile {} at ({x}, {y}) to the {}; no attackable town NPC.",
                    object.tile,
                    direction.name()
                );
                return Ok(MoveOutcome::Blocked);
            }
            self.message = format!(
                "Attacked object tile {} at ({x}, {y}) to the {} in slot {object_slot}; combat resolution is pending.",
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
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| self.object_occupies(object, x, y).then_some((slot, object)))
    }

    pub fn town_attack_target_at(
        &self,
        floor: i8,
        x: usize,
        y: usize,
    ) -> Option<(usize, usize, usize)> {
        if floor < 0 {
            return None;
        }
        let floor = floor as u8;
        self.npcs.iter().enumerate().find_map(|(index, npc)| {
            if npc.is_player_phantom() || npc.x != x || npc.y != y || npc.z != floor {
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
            Some((index, npc.slot, object_slot))
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
        let Some(entries) = load_town_fire_source_entries(game_dir)? else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(source) = entries
            .iter()
            .find(|entry| {
                let entry = **entry;
                entry.scene == scene
                    && entry.floor == floor
                    && town_fire_source_is_adjacent(entry, self.player.x, self.player.y)
                    && town_fire_source_tile_matches(entry, self.grid[entry.y * 32 + entry.x])
            })
            .copied()
        else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.tick_door_tracker();
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
                self.grid[y * 32 + x] = 16;
                self.record_open_town_door(scene, floor, x, y);
                self.forget_revealed_town_secret_door(scene, floor, x, y);
                self.door_tracker = None;
                self.mark_visibility_dirty();
                self.advance_turn_without_door_tick();
                self.message = format!(
                    "BOOOM! Town fire source at ({}, {}) fired {} and destroyed door tile {} at ({x}, {y}).",
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
            if (96..=103).contains(&tile) {
                return TownFireTarget::Door { x, y, tile };
            }
            if surface_tile_blocks_sight(tile) {
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
        if !direction.is_cardinal()
            || direction == self.player.facing
            || direction.opposite_cardinal() == Some(self.player.facing)
        {
            self.message = "Fire broadsides only!".to_string();
            return MoveOutcome::Blocked;
        }

        let hit = self
            .ship_broadside_target_slot(direction)
            .map(|slot| (slot, self.active_objects[slot]));
        let mut hit_report = None;
        if let Some((slot, object)) = hit {
            let damage = self.ship_broadside_damage_roll(direction, slot, object);
            let remaining = object.aux1.wrapping_sub(damage);
            if remaining & 0x80 != 0 {
                self.free_active_object_slot(slot);
                hit_report = Some(format!(
                    "BOOOM! Ship broadside hit object tile {} at ({}, {}) for {damage} durability damage; target destroyed.",
                    object.tile, object.x, object.y
                ));
            } else if let Some(target) = self.active_objects.get_mut(slot) {
                target.aux1 = remaining;
                hit_report = Some(format!(
                    "BOOOM! Ship broadside hit object tile {} at ({}, {}) for {damage} durability damage; durability now {remaining}.",
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
        for distance in 1..=3 {
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

    pub fn ship_broadside_damage_roll(
        &self,
        direction: Direction,
        slot: usize,
        object: ActiveObject,
    ) -> u8 {
        SHIP_BROADSIDE_DAMAGE_MIN
            + (self.ship_broadside_damage_seed(direction, slot, object) % SHIP_BROADSIDE_DAMAGE_MAX)
    }

    pub fn ship_broadside_damage_seed(
        &self,
        direction: Direction,
        slot: usize,
        object: ActiveObject,
    ) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (self.player.x as u8).wrapping_mul(7)
            ^ (self.player.y as u8).wrapping_mul(11)
            ^ (direction as u8).wrapping_mul(13)
            ^ (slot as u8).wrapping_mul(17)
            ^ object.type_byte.wrapping_mul(19)
            ^ object.tile.wrapping_mul(23)
            ^ object.aux1.wrapping_mul(29)
    }

    pub fn decrease_moral_standing(&mut self, amount: u8) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self.moral_standing.saturating_sub(amount);
        before - self.moral_standing
    }

    pub fn push_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => self.push_town_facing(game_dir, scene, floor),
            Area::Dungeon { .. } => {
                self.message = "What?".to_string();
                Ok(MoveOutcome::Blocked)
            }
            Area::World { .. } => {
                self.message = "Nothing to push here.".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn push_town_facing(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        let direction = self.player.facing;
        if !direction.is_cardinal() {
            self.message = "Push requires a cardinal facing direction.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
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
        if self.blocking_object_at(tx, ty).is_some() {
            self.message = format!("Cannot push occupied tile at ({tx}, {ty}).");
            return Ok(MoveOutcome::Blocked);
        }

        let target_idx = ty * 32 + tx;
        let target_tile = self.grid[target_idx];
        let Some(entries) = load_town_pushable_entries(game_dir)? else {
            self.message = "Nothing to push there.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let pushable = entries
            .iter()
            .any(|entry| town_pushable_matches(*entry, scene, floor, tx, ty, target_tile));
        if !pushable {
            self.message = "Nothing to push there.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        if self.blocking_object_at(px, py).is_some() {
            self.advance_turn();
            self.message = format!("Push blocked by actor at ({px}, {py}).");
            return Ok(MoveOutcome::Blocked);
        }
        let dest_idx = py * 32 + px;
        let dest_tile = self.grid[dest_idx];
        if !self.tile_walkable(dest_tile) {
            self.advance_turn();
            self.message = format!("Push blocked by {} at ({px}, {py}).", tile_class(dest_tile));
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[target_idx] = dest_tile;
        self.grid[dest_idx] = target_tile;
        self.forget_open_town_door(scene, floor, tx, ty);
        self.forget_open_town_door(scene, floor, px, py);
        self.forget_revealed_town_secret_door(scene, floor, tx, ty);
        self.forget_revealed_town_secret_door(scene, floor, px, py);
        if self.door_tracker.is_some_and(|tracker| {
            (tracker.x == tx && tracker.y == ty) || (tracker.x == px && tracker.y == py)
        }) {
            self.door_tracker = None;
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Pushed tile {target_tile} {} from ({tx}, {ty}) to ({px}, {py}).",
            direction.name()
        );
        Ok(MoveOutcome::Pushed)
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
            self.queue_current_moongate_prompt();
        }
        Ok(MoveOutcome::Passed)
    }

    pub fn queue_current_moongate_prompt(&mut self) -> bool {
        let Area::World { plane } = self.area else {
            return false;
        };
        let Some(entry) = self.moongate_at(plane, self.player.x, self.player.y) else {
            return false;
        };
        self.pending_moongate = Some(entry);
        if self.message.is_empty() {
            self.message = "Moongate! Enter? (Y/N).".to_string();
        } else {
            self.message.push_str(" Moongate! Enter? (Y/N).");
        }
        true
    }

    pub fn apply_top_down_post_turn_effects_after_turn(
        &mut self,
        turn_before: u64,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.turn == turn_before {
            return Ok(None);
        }
        match self.area {
            Area::World { .. } => {
                self.apply_world_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Town { .. } => {
                self.apply_town_post_turn_effects_after_turn(turn_before, game_dir)
            }
            Area::Dungeon { .. } => {
                self.apply_dungeon_post_turn_effects_after_turn(turn_before, game_dir)
            }
        }
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
        if self.turn == turn_before || self.pending_moongate.is_some() {
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
        if let Some(transition) = self.apply_world_whirlpool_engagement(game_dir, plane)? {
            let transition_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
            return Ok(Some(MoveOutcome::Transition(transition)));
        }
        if let Some(outcome) = self.apply_world_active_object_engagement(game_dir, plane)? {
            let engagement_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                engagement_message
            } else {
                format!("{pre_effect_message} {engagement_message}")
            };
            return Ok(Some(outcome));
        }
        self.append_world_damage_tile_message(Some(game_dir), plane)?;
        if let Some(slot) = self.apply_world_encounter_probe(game_dir, plane)? {
            self.message
                .push_str(&format!(" Wandering encounter spawned in slot {slot}."));
        }
        self.queue_current_moongate_prompt();
        Ok(None)
    }

    pub fn apply_world_whirlpool_engagement(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<AreaTransition>> {
        // active-objects.md §8: no-op when the party marker is the ordinary
        // on-foot avatar.
        if self.player.transport.is_foot() {
            return Ok(None);
        }
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let mut whirlpool_found = false;
        for (dx, dy) in [(0isize, -1isize), (0, 1), (-1, 0), (1, 0)] {
            let x = (px + dx).rem_euclid(WORLD_SIDE as isize) as usize;
            let y = (py + dy).rem_euclid(WORLD_SIDE as isize) as usize;
            if let Some(object) = self.world_object_at(x, y) {
                if is_whirlpool_object(*object) {
                    whirlpool_found = true;
                    break;
                }
            }
        }
        if !whirlpool_found {
            return Ok(None);
        }
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
            self.wind.status_message()
        );
        Ok(Some(AreaTransition::ChangedWorldPlane {
            from: plane,
            to: WorldPlane::Underworld,
        }))
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
        if let Some(outcome) = self.apply_town_npc_contact_event(scene, floor)? {
            return Ok(Some(outcome));
        }
        let tile = self.grid[self.player.y * 32 + self.player.x];
        if let Some(entry) =
            self.town_exit_tile_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        {
            let pre_effect_message = self.message.clone();
            let outcome = self.resolve_town_exit_tile_after_turn(game_dir, scene, floor, entry)?;
            let transition_message = self.message.clone();
            self.message = if pre_effect_message.is_empty() {
                transition_message
            } else {
                format!("{pre_effect_message} {transition_message}")
            };
            return Ok(Some(outcome));
        }

        let Some(entry) =
            self.town_trap_door_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        else {
            return Ok(None);
        };

        let pre_effect_message = self.message.clone();
        let outcome = self.apply_town_trap_door_transition(game_dir, scene, entry, false)?;
        let transition_message = self.message.clone();
        self.message = if pre_effect_message.is_empty() {
            transition_message
        } else {
            format!("{pre_effect_message} {transition_message}")
        };
        Ok(Some(outcome))
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
        if behavior.raises_guard_event() {
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
            let (fortified, fleeing) = self.town_alarm_sweep(scene, floor, Some(npc_slot));
            self.set_town_npc_alarm_state(scene, floor, npc_slot, TownNpcAlarmState::Fortified);
            self.message = if self.message.is_empty() {
                format!(
                    "Hostile NPC slot {npc_slot} (type {type_byte}) attacks; alarm raised ({fortified} fortified, {fleeing} fleeing)."
                )
            } else {
                format!(
                    "{} Hostile NPC slot {npc_slot} (type {type_byte}) attacks; alarm raised ({fortified} fortified, {fleeing} fleeing).",
                    self.message
                )
            };
            return Ok(Some(MoveOutcome::Used));
        }
        Ok(None)
    }

    pub fn town_adjacent_event_npc(
        &self,
        scene: Scene,
        floor: i8,
    ) -> Option<(usize, u8, NpcAiBehavior)> {
        let floor_u8 = floor as u8;
        self.npcs.iter().find_map(|npc| {
            if npc.is_player_phantom()
                || npc.z != floor_u8
                || npc.x.abs_diff(self.player.x) + npc.y.abs_diff(self.player.y) != 1
                || self.town_npc_alarm_state(scene, floor, npc.slot)
                    == Some(TownNpcAlarmState::Pacified)
            {
                return None;
            }
            let wp = waypoint_for_hour(&npc.schedule, self.clock.hour);
            let alarm_state = self.town_npc_alarm_state(scene, floor, npc.slot);
            let behavior = if alarm_state == Some(TownNpcAlarmState::Fortified) {
                if town_contact_type_guard_like(npc.type_byte) {
                    NpcAiBehavior::GuardOrBlock
                } else {
                    NpcAiBehavior::ApproachAndAttack
                }
            } else {
                npc_ai_behavior(npc.schedule[NPC_SCHEDULE_AI_OFFSET + wp])?
            };
            (behavior.raises_attack_event() || behavior.raises_guard_event())
                .then_some((npc.slot, npc.type_byte, behavior))
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
                    self.message =
                        "Blackthorn audience capture would begin from the captive scene.".to_string();
                    return Ok(Some(MoveOutcome::Used));
                }
                self.apply_town_arrest_surrender(game_dir)
            }
            'n' => {
                self.pending_town_arrest = None;
                let scene = Scene::new(prompt.scene_byte)?;
                let (fortified, fleeing) =
                    self.town_alarm_sweep(scene, prompt.floor, Some(prompt.npc_slot));
                self.message = format!(
                    "Refused surrender; alarm raised ({fortified} fortified, {fleeing} fleeing)."
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
        self.grid = load_town_runtime_floor(game_dir, scene, floor, self.clock.hour)?;
        self.area = Area::Town { scene, floor };
        self.player.x = TOWN_ARREST_JAIL_X as usize;
        self.player.y = TOWN_ARREST_JAIL_Y as usize;
        self.player.transport = TransportState::Foot;
        self.pending_moongate = None;
        self.clear_town_floor_reload_door_state();
        self.town_npc_alarm_states
            .retain(|marker| marker.scene_byte == scene.byte && marker.floor == floor);
        let tlk = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let npc_slots = parse_npc_block(game_dir, scene, &tlk)?;
        self.load_scheduled_npcs(&npc_slots);
        self.attach_player_phantom_npc();
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
        Ok(Some(MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))))
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
            let field_report = self.apply_dungeon_field_effect(field);
            let field_message = format!("Triggered {}; {field_report}.", field.label());
            self.message = if self.message.is_empty() {
                field_message
            } else {
                format!("{} {field_message}", self.message)
            };
            return Ok(None);
        }
        if self.dungeon_wind_tile_extinguishes_torch(Some(game_dir), scene, level, x, y, tile)? {
            self.torch_counter = 0;
            self.mark_visibility_dirty();
            let wind_message = "A breeze blows out the torch.".to_string();
            self.message = if self.message.is_empty() {
                wind_message
            } else {
                format!("{} {wind_message}", self.message)
            };
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
            if combat_class_stats_for_sprite_byte(object.tile)
                .or_else(|| combat_class_stats_for_sprite_byte(object.type_byte))
                .is_some()
            {
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
                "Dungeon monster tile {} approaches from the {} on {} level {level}; dungeon combat resolution is pending.",
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
        let Area::Dungeon { .. } = self.area else {
            return None;
        };
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find(|(_, object)| self.object_occupies(*object, object.x, object.y))
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
        hours: Option<u8>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => self.hole_up_town_command(game_dir, hours, scene, floor),
            Area::World { .. } | Area::Dungeon { .. } => {
                self.rest_with_watch(hours, Some(game_dir))
            }
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
            self.message =
                "Hole up- how many hours? Use H plus a number in this harness.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if !(1..=9).contains(&hours) {
            self.message = "Rest hours must be in 1..9.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let Some(entries) = load_town_rest_bed_entries(game_dir)? else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if !self.town_rest_bed_still_accepts(&entries, scene, floor) {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.mark_town_rest_sleepers();
        if !self.advance_town_rest_initial_schedule_burst(&entries, scene, floor) {
            let woke = self.wake_town_rest_sleepers();
            self.message = format!(
                "Rest interrupted; thrown out of the inn bed; woke {woke} asleep member(s)."
            );
            return Ok(MoveOutcome::Blocked);
        }
        if !self.advance_town_rest_until_target_hour(hours, &entries, scene, floor) {
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
        entries: &[TownRestBedEntry],
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
        entries: &[TownRestBedEntry],
        scene: Scene,
        floor: i8,
    ) -> bool {
        for _ in 0..TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS {
            self.advance_turn_with_minutes(0);
            if !self.town_rest_bed_still_accepts(entries, scene, floor) {
                return false;
            }
        }
        true
    }

    pub fn town_rest_bed_still_accepts(
        &self,
        entries: &[TownRestBedEntry],
        scene: Scene,
        floor: i8,
    ) -> bool {
        let tile = self.grid[self.player.y * 32 + self.player.x];
        entries.iter().any(|entry| {
            town_rest_bed_matches(*entry, scene, floor, self.player.x, self.player.y, tile)
        })
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

fn town_contact_type_guard_like(type_byte: u8) -> bool {
    matches!(type_byte, 0x70..=0x7f)
}
