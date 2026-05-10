impl PlayState {
    pub fn z_stats(&mut self) -> MoveOutcome {
        self.message = self.z_stats_message();
        MoveOutcome::Observed
    }

    pub fn z_stats_message(&self) -> String {
        let area = self.area_status_label();
        let reagents_total: u16 = self.reagents.iter().map(|count| *count as u16).sum();
        let spells = self.spell_stock_summary();
        let party = self.party_status_summary();
        let effect = self.active_effect_status();
        format!(
            "Z-stats: {area} at ({}, {}), facing {}, date Y{} M{} D{} {:02}:{:02}, turn {}; transport {}; wind {}; typeahead {}; timing {}; light torch={} spell={} ambient={} time-stop={} effect={}; inventory food={} gold={} keys={} gems={} torches={} climbing={} reagents={}; spells {}; party {}.",
            self.player.x,
            self.player.y,
            self.player.facing.name(),
            self.clock.year,
            self.clock.month,
            self.clock.day,
            self.clock.hour,
            self.clock.minute,
            self.turn,
            self.player.transport.status_label(),
            self.wind.status_message(),
            self.typeahead_status_label(),
            self.timing_status.status_label(),
            self.torch_counter,
            self.light_spell_counter,
            self.ambient_light,
            self.time_stop_counter,
            effect,
            self.food,
            self.gold,
            self.keys,
            self.gems,
            self.torches,
            self.climbing_gear,
            reagents_total,
            spells,
            party
        )
    }

    pub fn active_effect_status(&self) -> String {
        match (self.active_effect_tag, self.active_effect_counter) {
            (Some(tag), counter) if counter > 0 => {
                format!("{}/{}", char::from(tag), counter)
            }
            _ => "none".to_string(),
        }
    }

    pub fn toggle_typeahead_buffer(&mut self) {
        self.typeahead_buffer_enabled = !self.typeahead_buffer_enabled;
        self.message = if self.typeahead_buffer_enabled {
            "Buffer On."
        } else {
            "Buffer Off."
        }
        .to_string();
    }

    pub fn exit_to_dos_prompt(&mut self, confirm: Option<bool>) -> PlayInputDisposition {
        match confirm {
            None => {
                self.message = "Exit to DOS? Use QY to exit or QN to cancel.".to_string();
                PlayInputDisposition::Continue
            }
            Some(false) => {
                self.message = "No.".to_string();
                PlayInputDisposition::Continue
            }
            Some(true) => {
                self.message = "Yes. Exiting to DOS.".to_string();
                PlayInputDisposition::Quit
            }
        }
    }

    pub fn typeahead_status_label(&self) -> &'static str {
        if self.typeahead_buffer_enabled {
            "on"
        } else {
            "off"
        }
    }

    pub fn area_status_label(&self) -> String {
        match self.area {
            Area::Town { scene, floor } => format!("{} floor {floor}", scene.key()),
            Area::Dungeon { scene, level } => format!("{} level {level}", scene.key()),
            Area::World { plane } => plane.key().to_string(),
        }
    }

    pub fn spell_stock_summary(&self) -> String {
        let stock = self
            .spell_charges
            .iter()
            .enumerate()
            .filter(|(_, charges)| **charges > 0)
            .map(|(index, charges)| format!("{}={charges}", SPELL_CODES[index]))
            .collect::<Vec<_>>();
        if stock.is_empty() {
            "none".to_string()
        } else {
            stock.join(", ")
        }
    }

    pub fn party_status_summary(&self) -> String {
        let party = self
            .party
            .iter()
            .enumerate()
            .map(|(index, member)| {
                format!(
                    "P{}:slot{} {} HP {}/{} MP {} L{}",
                    index + 1,
                    member.slot,
                    party_status_name(member.status),
                    member.hp,
                    member.max_hp,
                    member.mana,
                    member.level
                )
            })
            .collect::<Vec<_>>();
        if party.is_empty() {
            "none".to_string()
        } else {
            party.join("; ")
        }
    }

    pub fn new_order_from_suffix(&mut self, suffix: &str) -> MoveOutcome {
        let Some((first, second)) = parse_inline_party_swap(suffix) else {
            self.message = new_order_prompt_message();
            return MoveOutcome::PromptDeclined;
        };
        if first == second {
            self.message = "Party slots are already in that order.".to_string();
            return MoveOutcome::PromptDeclined;
        }
        let party_len = self.party.len();
        if first >= party_len || second >= party_len {
            self.message = format!(
                "Party has {} member{}.",
                party_len,
                if party_len == 1 { "" } else { "s" }
            );
            return MoveOutcome::Blocked;
        }

        self.party.swap(first, second);
        self.message = format!(
            "New order: party slots {} and {} swapped.",
            first + 1,
            second + 1
        );
        MoveOutcome::Used
    }

    pub fn cast_light_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        duration: u8,
    ) -> MoveOutcome {
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        self.advance_turn();
        self.light_spell_counter = duration;
        self.recompute_daylight();
        self.message = "Light!".to_string();
        MoveOutcome::Cast
    }

    pub fn cast_active_effect_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        tag: u8,
        duration: u8,
        label: &str,
    ) -> MoveOutcome {
        if !spell_allowed_in_area(spell_index, self.area) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        self.advance_turn();
        self.active_effect_tag = Some(tag);
        self.active_effect_counter = duration;
        self.message = format!("{label}!");
        MoveOutcome::Cast
    }

    pub fn cast_awaken(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, AWAKEN_SPELL_INDEX, AWAKEN_COST)
        {
            return outcome;
        }

        if self.party[target_index].status != b'S' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        self.party[target_index].status = b'G';
        self.advance_turn();
        self.message = format!("Awakened party member {}.", target_index + 1);
        MoveOutcome::Cast
    }

    pub fn cast_cure(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, CURE_SPELL_INDEX, CURE_COST)
        {
            return outcome;
        }

        if self.party[target_index].status != b'P' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        self.party[target_index].status = b'G';
        self.advance_turn();
        self.message = format!("Cured party member {}.", target_index + 1);
        MoveOutcome::Cast
    }

    pub fn cast_heal(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, HEAL_SPELL_INDEX, HEAL_COST)
        {
            return outcome;
        }

        if !self.party[target_index].living() {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let healed = self.party[target_index].heal_by(HEAL_AMOUNT);
        let hp = self.party[target_index].hp;
        let max_hp = self.party[target_index].max_hp;
        self.advance_turn();
        self.message = format!(
            "Healed party member {} for {healed} HP ({hp}/{max_hp}).",
            target_index + 1
        );
        MoveOutcome::Cast
    }

    pub fn cast_great_heal(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, GREAT_HEAL_SPELL_INDEX, GREAT_HEAL_COST)
        {
            return outcome;
        }

        if !self.party[target_index].living() {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let before = self.party[target_index].hp;
        let (_, hp) = self.party[target_index].heal_to_max();
        let healed = hp.saturating_sub(before);
        let max_hp = self.party[target_index].max_hp;
        self.advance_turn();
        self.message = format!(
            "Great healed party member {} for {healed} HP ({hp}/{max_hp}).",
            target_index + 1
        );
        MoveOutcome::Cast
    }

    pub fn cast_resurrect(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, RESURRECT_SPELL_INDEX, RESURRECT_COST)
        {
            return outcome;
        }

        if self.party[target_index].status != b'D' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        self.party[target_index].status = b'G';
        let (_, hp) = self.party[target_index].heal_to_max();
        let max_hp = self.party[target_index].max_hp;
        self.advance_turn();
        self.message = format!(
            "Resurrected party member {} ({hp}/{max_hp}).",
            target_index + 1
        );
        MoveOutcome::Cast
    }

    pub fn cast_locate(&mut self, caster_index: usize) -> MoveOutcome {
        let Area::World { plane } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, IN_WIS_SPELL_INDEX, IN_WIS_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.message = format!(
            "Locate: {} at ({}, {}), facing {}, wind {}, time {:02}:{:02}.",
            plane.key(),
            self.player.x,
            self.player.y,
            self.player.facing.name(),
            self.wind.status_message(),
            self.clock.hour,
            self.clock.minute
        );
        MoveOutcome::Observed
    }

    pub fn cast_peer(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, PEER_SPELL_INDEX, PEER_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.message = self.peer_view_message();
        MoveOutcome::Observed
    }

    pub fn cast_x_ray(&mut self, caster_index: usize) -> MoveOutcome {
        if !spell_allowed_in_area(X_RAY_SPELL_INDEX, self.area) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, X_RAY_SPELL_INDEX, X_RAY_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.message = self.x_ray_view_message();
        MoveOutcome::Observed
    }

    pub fn cast_create_food(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, CREATE_FOOD_SPELL_INDEX, CREATE_FOOD_COST)
        {
            return outcome;
        }

        let before = self.food;
        self.food = self.food.saturating_add(CREATE_FOOD_AMOUNT);
        let created = self.food.saturating_sub(before);
        self.advance_turn();
        self.message = format!("Created {created} food; stock is {}.", self.food);
        MoveOutcome::Cast
    }

    pub fn peer_view_message(&self) -> String {
        match self.area {
            Area::Dungeon { scene, level } => format!(
                "Peer view of {} ({}) level {} (spell; centered flood map, exact glyph/floodability edge cases out of scope):\n{}",
                scene.key(),
                scene.name(),
                level,
                self.dungeon_vision_map(level)
            ),
            Area::Town { scene, floor } => format!(
                "Peer view of {} floor {} (spell; full-fill 11x11 map):\n{}",
                scene.key(),
                floor,
                self.surface_gem_map(5)
            ),
            Area::World { plane } => format!(
                "Peer view of {} at ({}, {}) (spell; full-fill 11x11 map):\n{}",
                plane.key(),
                self.player.x,
                self.player.y,
                self.surface_gem_map(5)
            ),
        }
    }

    pub fn x_ray_view_message(&self) -> String {
        match self.area {
            Area::Town { scene, floor } => format!(
                "X-Ray view of {} floor {} (spell; first-playable full-fill 11x11 map):\n{}",
                scene.key(),
                floor,
                self.surface_gem_map(5)
            ),
            Area::World { plane } => format!(
                "X-Ray view of {} at ({}, {}) (spell; first-playable full-fill 11x11 map):\n{}",
                plane.key(),
                self.player.x,
                self.player.y,
                self.surface_gem_map(5)
            ),
            Area::Dungeon { .. } => "Not here!".to_string(),
        }
    }

    pub fn cast_open_spell(&mut self, caster_index: usize, game_dir: &Path) -> io::Result<MoveOutcome> {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, OPEN_SPELL_INDEX, OPEN_SPELL_COST)
        {
            return Ok(outcome);
        }

        let Area::Dungeon { scene, level } = self.area else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        if tile >> 4 != 0x4 {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let chest_entries = load_dungeon_chest_content_entries(game_dir)?;
        let note = self
            .apply_dungeon_chest_content(
                chest_entries.as_deref(),
                scene,
                level,
                self.player.x,
                self.player.y,
                tile,
            )
            .map(|grant_note| format!("trap generator bypassed by An Sanct; {grant_note}"))
            .unwrap_or_else(|| "trap generator bypassed by An Sanct".to_string());

        Ok(self.consume_dungeon_chest_with_note(
            scene,
            level,
            self.player.x,
            self.player.y,
            idx,
            tile,
            "Safely opened",
            &note,
        ))
    }

    pub fn cast_dungeon_level_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        delta: i8,
        label: &str,
    ) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, spell_index, DUNGEON_LEVEL_SPELL_COST)
        {
            return outcome;
        }

        let next_level = level as i8 + delta;
        if !(0..=7).contains(&next_level) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let next_level = next_level as u8;
        self.area = Area::Dungeon {
            scene,
            level: next_level,
        };
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "{label}! Changed to {} ({}) level {next_level}.",
            scene.key(),
            scene.name()
        );
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel {
            scene,
            level: next_level,
        })
    }

    pub fn cast_dungeon_field_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        direction: Option<Direction>,
        base_field: u8,
        marker_field: u8,
        label: &str,
    ) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        let Some(direction) = direction else {
            self.message = "Direction? Use C1FGI6/C1GIN6/C1GIZ6/C1GIS6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Field placement requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&tx) || !(0..DUNGEON_SIDE as isize).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let idx = dungeon_cell_index(level, tx as usize, ty as usize);
        self.grid[idx] = match self.grid[idx] {
            0x00 => base_field,
            0x08 => marker_field,
            _ => {
                self.advance_turn();
                self.message = "Failed!".to_string();
                return MoveOutcome::Blocked;
            }
        };
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "{label} placed {} at ({}, {}) on {} level {level}.",
            direction.name(),
            tx,
            ty,
            scene.key()
        );
        MoveOutcome::Cast
    }

    pub fn cast_dispel_field(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
    ) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        let Some(direction) = direction else {
            self.message = "Direction? Use C1AG6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Dispel Field requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, DISPEL_FIELD_SPELL_INDEX, DISPEL_FIELD_COST)
        {
            return outcome;
        }

        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&tx) || !(0..DUNGEON_SIDE as isize).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let idx = dungeon_cell_index(level, tx as usize, ty as usize);
        let cell = self.grid[idx];
        let Some(field) = dungeon_field_effect(cell) else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        };
        self.grid[idx] = cell & 0x08;
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Dispelled {} at ({}, {}) on {} level {level}.",
            field.label(),
            tx,
            ty,
            scene.key()
        );
        MoveOutcome::Cast
    }

    pub fn cast_time_stop(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, TIME_STOP_SPELL_INDEX, TIME_STOP_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.time_stop_counter = TIME_STOP_DURATION;
        self.message = format!("Time stopped for {TIME_STOP_DURATION} turns.");
        MoveOutcome::Cast
    }

    pub fn cast_blink(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Some(direction) = direction else {
            self.message = "Direction? Use C1IP6.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(entry) = self.blink_target_at(game_dir, direction)? else {
            self.message = "No Blink target.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, BLINK_SPELL_INDEX, BLINK_COST)
        {
            return Ok(outcome);
        }

        if !self.blink_source_matches(entry) || !self.blink_destination_legal(game_dir, entry)? {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Blinked {} to ({}, {}) in {}.",
            direction.name(),
            entry.to_x,
            entry.to_y,
            self.current_area_label()
        );
        Ok(MoveOutcome::Cast)
    }

    pub fn blink_target_at(
        &self,
        game_dir: &Path,
        direction: Direction,
    ) -> io::Result<Option<BlinkTargetEntry>> {
        let (target, floor, x, y) = self.current_blink_context();
        Ok(load_blink_target_entries(game_dir)?.and_then(|entries| {
            entries.into_iter().find(|entry| {
                entry.target == target
                    && entry.floor == floor
                    && entry.from_x == x
                    && entry.from_y == y
                    && entry.direction == direction
            })
        }))
    }

    pub fn current_blink_context(&self) -> (PlayTarget, i8, usize, usize) {
        match self.area {
            Area::World { plane } => (
                PlayTarget::World(plane),
                plane.save_floor(),
                self.player.x,
                self.player.y,
            ),
            Area::Town { scene, floor } => {
                (PlayTarget::Town(scene), floor, self.player.x, self.player.y)
            }
            Area::Dungeon { scene, level } => (
                PlayTarget::Dungeon(scene),
                level as i8,
                self.player.x,
                self.player.y,
            ),
        }
    }

    pub fn current_area_label(&self) -> String {
        match self.area {
            Area::World { plane } => plane.key().to_string(),
            Area::Town { scene, floor } => format!("{} floor {floor}", scene.key()),
            Area::Dungeon { scene, level } => format!("{} level {level}", scene.key()),
        }
    }

    pub fn blink_source_matches(&self, entry: BlinkTargetEntry) -> bool {
        entry.expected_from_tile.map_or(true, |expected| {
            expected == self.current_area_tile(entry.from_x, entry.from_y)
        })
    }

    pub fn blink_destination_legal(
        &self,
        game_dir: &Path,
        entry: BlinkTargetEntry,
    ) -> io::Result<bool> {
        if entry.expected_to_tile.map_or(false, |expected| {
            expected != self.current_area_tile(entry.to_x, entry.to_y)
        }) {
            return Ok(false);
        }
        match self.area {
            Area::World { .. } | Area::Town { .. } => {
                self.player_can_land_on_foot(Some(game_dir), entry.to_x, entry.to_y)
            }
            Area::Dungeon { scene, level } => {
                let cell = self.dungeon_cell(level, entry.to_x, entry.to_y);
                if self.dungeon_closed_door_at(
                    Some(game_dir),
                    scene,
                    level,
                    entry.to_x,
                    entry.to_y,
                    cell,
                )? {
                    return Ok(false);
                }
                Ok(is_dungeon_walkable(cell)
                    || self.dungeon_open_door_at(
                        Some(game_dir),
                        scene,
                        level,
                        entry.to_x,
                        entry.to_y,
                        cell,
                    )?)
            }
        }
    }

    pub fn current_area_tile(&self, x: usize, y: usize) -> u8 {
        match self.area {
            Area::World { .. } => self.grid[world_cell_index(x, y)],
            Area::Town { .. } => self.grid[y * 32 + x],
            Area::Dungeon { level, .. } => self.dungeon_cell(level, x, y),
        }
    }

    pub fn cast_magic_lock(&mut self, caster_index: usize, game_dir: &Path) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, MAGIC_LOCK_SPELL_INDEX, MAGIC_LOCK_COST)
        {
            return Ok(outcome);
        }

        let (dx, dy) = self.player.facing.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        let Some(entry) = self.town_magic_lock_target_at(game_dir, scene, floor, tx, ty, tile)?
        else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.grid[idx] = entry.locked_tile;
        self.forget_open_town_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = "Magic lock!".to_string();
        Ok(MoveOutcome::Cast)
    }

    pub fn cast_unlock_magic(
        &mut self,
        caster_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, UNLOCK_MAGIC_SPELL_INDEX, UNLOCK_MAGIC_COST)
        {
            return Ok(outcome);
        }

        let (dx, dy) = self.player.facing.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        let Some(entry) = self.town_lock_at(Some(game_dir), scene, floor, tx, ty, tile)? else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if entry.kind != TownLockKind::Magic {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[idx] = entry.unlocked_tile;
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = "Unlocked!".to_string();
        Ok(MoveOutcome::Cast)
    }

}
