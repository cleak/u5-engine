impl PlayState {
    fn step_non_town(
        &mut self,
        direction: Direction,
        nx: isize,
        ny: isize,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Dungeon { scene, level } => {
                self.step_dungeon(direction, nx, ny, scene, level, game_dir)
            }
            Area::World { plane } => self.step_world(direction, nx, ny, plane, game_dir),
            Area::Town { .. } => unreachable!("town step handled before non-town dispatch"),
        }
    }

    #[cfg(test)]
    fn handle_dungeon_key(&mut self, key: char, game_dir: &Path) -> io::Result<bool> {
        self.handle_dungeon_key_with_inline(key, game_dir, None, None, None, None)
    }

    fn handle_dungeon_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_hours: Option<u8>,
        inline_drink: Option<bool>,
        inline_party_index: Option<usize>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        if !matches!(self.area, Area::Dungeon { .. }) {
            return Ok(false);
        }
        if self
            .resolve_current_dungeon_room_trigger(Some(game_dir))?
            .is_some()
        {
            return Ok(true);
        }
        let turn_before = self.turn;
        macro_rules! handled {
            ($outcome:expr) => {{
                let outcome = $outcome;
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
                return Ok(true);
            }};
            () => {{
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
                return Ok(true);
            }};
        }

        if key == 'S' {
            handled!(self.search_facing_with_game_dir(game_dir)?);
        }
        match key.to_ascii_lowercase() {
            '8' | 'w' => {
                self.step_with_game_dir(self.player.facing, Some(game_dir))?;
                Ok(true)
            }
            '2' | 's' => {
                let facing = self.player.facing;
                if let Some(direction) = facing.opposite_cardinal() {
                    self.step_with_game_dir(direction, Some(game_dir))?;
                    if matches!(self.area, Area::Dungeon { .. }) {
                        self.player.facing = facing;
                    }
                } else {
                    self.message =
                        "Dungeon back-step requires a cardinal facing direction.".to_string();
                }
                Ok(true)
            }
            '4' | 'a' => {
                handled!(self.turn_dungeon(false));
            }
            '6' | 'd' => {
                handled!(self.turn_dungeon(true));
            }
            'k' => {
                let Area::Dungeon { level, .. } = self.area else {
                    unreachable!("dungeon key handler is gated to dungeon scenes");
                };
                let outcome = match self.dungeon_cell(level, self.player.x, self.player.y) >> 4 {
                    0x1 => self.climb(game_dir, ClimbIntent::Up)?,
                    0x2 => self.climb(game_dir, ClimbIntent::Down)?,
                    0x3 => {
                        self.message =
                            "Two-way ladder: use < or > to choose a climb direction.".to_string();
                        MoveOutcome::Blocked
                    }
                    _ => {
                        self.message = "Not climbable!".to_string();
                        MoveOutcome::Blocked
                    }
                };
                handled!(outcome);
            }
            '<' => {
                handled!(self.climb(game_dir, ClimbIntent::Up)?);
            }
            '>' => {
                handled!(self.climb(game_dir, ClimbIntent::Down)?);
            }
            'l' => {
                handled!(self.look_dungeon_with_drink(inline_drink, inline_party_index));
            }
            'g' => {
                let Area::Dungeon { scene, level } = self.area else {
                    unreachable!("dungeon key handler is gated to dungeon scenes");
                };
                handled!(self.get_dungeon_underfoot_with_game_dir(Some(game_dir), scene, level)?);
            }
            'i' => {
                handled!(self.ignite_torch());
            }
            'o' => {
                handled!(self.open_facing_with_game_dir(Some(game_dir))?);
            }
            'v' => {
                handled!(self.view_gem());
            }
            't' => {
                self.message = "Funny, no response!".to_string();
                handled!();
            }
            'b' | 'e' | 'x' => {
                self.message = "Not here!".to_string();
                handled!();
            }
            'f' | 'p' => {
                self.message = "What?".to_string();
                handled!();
            }
            'q' => {
                let _ = self.exit_to_dos_prompt(inline_drink);
                Ok(true)
            }
            'h' => {
                handled!(self.hole_up_command(game_dir, inline_hours)?);
            }
            ' ' => {
                self.pass_turn_with_game_dir(Some(game_dir))?;
                Ok(true)
            }
            '7' | '9' | '1' | '3' => {
                self.message =
                    "Dungeon movement supports forward, back, and turns only.".to_string();
                handled!();
            }
            'c' => {
                self.message = cast_prompt_message();
                handled!();
            }
            'm' => {
                self.message = mix_prompt_message();
                handled!();
            }
            'n' => {
                self.message = new_order_prompt_message();
                handled!();
            }
            'r' => {
                self.message = "Ready is out of scope in this slice.".to_string();
                handled!();
            }
            'u' => {
                handled!(self.use_item_command(inline_use_request, Some(game_dir))?);
            }
            'y' => {
                self.message = "Yell is out of scope in this slice.".to_string();
                handled!();
            }
            'z' => {
                handled!(self.z_stats());
            }
            'j' => {
                handled!(self.jimmy_facing_with_game_dir(Some(game_dir))?);
            }
            _ => Ok(false),
        }
    }

    fn handle_top_down_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_direction: Option<Direction>,
        inline_hours: Option<u8>,
        inline_yes_no: Option<bool>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        if matches!(self.area, Area::Dungeon { .. }) {
            return Ok(false);
        }
        let turn_before = self.turn;
        macro_rules! handled {
            ($outcome:expr) => {{
                let outcome = $outcome;
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
                return Ok(true);
            }};
            () => {{
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
                return Ok(true);
            }};
        }

        if key == ' ' {
            self.pass_turn_with_game_dir(Some(game_dir))?;
            return Ok(true);
        }

        if key.is_ascii_uppercase() {
            match key {
                'A' => {
                    self.message = "Attack is out of scope in this slice.".to_string();
                    handled!();
                }
                'B' => {
                    handled!(self.board_vehicle());
                }
                'C' => {
                    self.message = cast_prompt_message();
                    handled!();
                }
                'D' | 'W' => {
                    self.message = "What?".to_string();
                    handled!();
                }
                'E' => {
                    handled!(self.enter_current_location(game_dir)?);
                }
                'F' => {
                    handled!(self.fire_command(inline_direction, game_dir)?);
                }
                'G' => {
                    handled!(self.get_facing_with_game_dir(game_dir)?);
                }
                'H' => {
                    handled!(self.hole_up_command(game_dir, inline_hours)?);
                }
                'I' => {
                    handled!(self.ignite_torch());
                }
                'J' => {
                    handled!(self.jimmy_facing_with_game_dir(Some(game_dir))?);
                }
                'K' => {
                    handled!(self.klimb_command(game_dir)?);
                }
                'L' => {
                    handled!(self.look_facing_with_game_dir(game_dir)?);
                }
                'M' => {
                    self.message = self
                        .shrine_prompt_at_current_position(game_dir)?
                        .unwrap_or_else(mix_prompt_message);
                    handled!();
                }
                'N' => {
                    self.message = new_order_prompt_message();
                    handled!();
                }
                'O' => {
                    handled!(self.open_facing_with_game_dir(Some(game_dir))?);
                }
                'P' => {
                    handled!(self.push_facing_with_game_dir(game_dir)?);
                }
                'Q' => {
                    handled!(self.save_game_command(game_dir, inline_yes_no)?);
                }
                'R' => {
                    self.message = "Ready is out of scope in this slice.".to_string();
                    handled!();
                }
                'S' => {
                    handled!(self.search_facing_with_game_dir(game_dir)?);
                }
                'T' => {
                    handled!(self.talk_facing_with_game_dir(game_dir)?);
                }
                'U' => {
                    handled!(self.use_item_command(inline_use_request, Some(game_dir))?);
                }
                'V' => {
                    handled!(self.view_gem());
                }
                'X' => {
                    handled!(self.exit_vehicle_with_game_dir(Some(game_dir))?);
                }
                'Y' => {
                    handled!(self.toggle_sails());
                }
                'Z' => {
                    handled!(self.z_stats());
                }
                _ => {}
            }
        }

        if let Some(direction) = Direction::from_play_key(key) {
            self.step_with_game_dir(direction, Some(game_dir))?;
            return Ok(true);
        }

        let outcome = match key.to_ascii_lowercase() {
            'e' => self.enter_current_location(game_dir)?,
            'o' => self.open_facing_with_game_dir(Some(game_dir))?,
            'l' => self.look_facing_with_game_dir(game_dir)?,
            'v' => self.view_gem(),
            'i' => self.ignite_torch(),
            'h' => self.hole_up_command(game_dir, inline_hours)?,
            'f' => self.fire_command(inline_direction, game_dir)?,
            'p' => self.push_facing_with_game_dir(game_dir)?,
            'g' => self.get_facing_with_game_dir(game_dir)?,
            't' => self.talk_facing_with_game_dir(game_dir)?,
            'j' => self.jimmy_facing_with_game_dir(Some(game_dir))?,
            'k' => self.klimb_command(game_dir)?,
            'x' => self.exit_vehicle_with_game_dir(Some(game_dir))?,
            'm' => {
                self.message = self
                    .shrine_prompt_at_current_position(game_dir)?
                    .unwrap_or_else(mix_prompt_message);
                MoveOutcome::Observed
            }
            'z' => self.z_stats(),
            'r' => {
                self.message = "Ready is out of scope in this slice.".to_string();
                MoveOutcome::Blocked
            }
            '<' => self.climb(game_dir, ClimbIntent::Up)?,
            '>' => self.climb(game_dir, ClimbIntent::Down)?,
            '.' => self.idle_tick(),
            _ => return Ok(false),
        };
        self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        Ok(true)
    }

    fn cast_spell_from_suffix(&mut self, suffix: &str, game_dir: &Path) -> io::Result<MoveOutcome> {
        let spell_code = inline_spell_code(suffix);
        if spell_code.is_empty() {
            self.message = cast_prompt_message();
            return Ok(MoveOutcome::Blocked);
        }
        match spell_code.as_str() {
            "AG" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AG6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dispel_field(caster_index, parse_inline_cardinal_direction(suffix)))
            }
            "AEP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AEP for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_magic_lock(caster_index, game_dir)
            }
            "AN" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AN2 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.message = "Whom? Use C1AN2 to cure party member 2.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_cure(caster_index, target_index))
            }
            "AS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AS for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_open_spell(caster_index, game_dir)
            }
            "AT" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AT for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_time_stop(caster_index))
            }
            "AWY" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AWY for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_x_ray(caster_index))
            }
            "AZ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AZ2 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.message = "Whom? Use C1AZ2 to awaken party member 2.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_awaken(caster_index, target_index))
            }
            "DP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1DP for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dungeon_level_spell(caster_index, DES_POR_SPELL_INDEX, 1, "Down"))
            }
            "FGI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1FGI6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    FIRE_FIELD_SPELL_INDEX,
                    FIELD_SPELL_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x82,
                    0x8a,
                    "Fire field",
                ))
            }
            "GIN" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1GIN6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    POISON_FIELD_SPELL_INDEX,
                    FIELD_SPELL_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x81,
                    0x89,
                    "Poison field",
                ))
            }
            "GIZ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1GIZ6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    SLEEP_FIELD_SPELL_INDEX,
                    FIELD_SPELL_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x80,
                    0x88,
                    "Sleep field",
                ))
            }
            "GIS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1GIS6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    ENERGY_FIELD_SPELL_INDEX,
                    ENERGY_FIELD_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x83,
                    0x8b,
                    "Energy field",
                ))
            }
            "IL" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IL for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_light_spell(
                    caster_index,
                    IN_LOR_SPELL_INDEX,
                    IN_LOR_COST,
                    IN_LOR_LIGHT_DURATION,
                ))
            }
            "IS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IS for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    PROTECTION_SPELL_INDEX,
                    PROTECTION_COST,
                    PROTECTION_ACTIVE_EFFECT_TAG,
                    PROTECTION_ACTIVE_EFFECT_DURATION,
                    "Protection",
                ))
            }
            "IMX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IMX for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_create_food(caster_index))
            }
            "IP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IP6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_blink(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    game_dir,
                )
            }
            "IQW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IQW for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_peer(caster_index))
            }
            "IW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IW for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_locate(caster_index))
            }
            "HR" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1HR for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_rel_hur(caster_index))
            }
            "LV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1LV for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_light_spell(
                    caster_index,
                    VAS_LOR_SPELL_INDEX,
                    VAS_LOR_COST,
                    VAS_LOR_LIGHT_DURATION,
                ))
            }
            "M" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1M2 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.message = "Whom? Use C1M2 to heal party member 2.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_heal(caster_index, target_index))
            }
            "MV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1MV2 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.message = "Whom? Use C1MV2 to great-heal party member 2.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_great_heal(caster_index, target_index))
            }
            "EIP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1EIP for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_unlock_magic(caster_index, game_dir)
            }
            "CIM" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1CIM2 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.message = "Whom? Use C1CIM2 to resurrect party member 2.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_resurrect(caster_index, target_index))
            }
            "PU" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1PU for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dungeon_level_spell(caster_index, UUS_POR_SPELL_INDEX, -1, "Up"))
            }
            "PRV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1PRV2 for party slot 1 to phase 2.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(slot_index) = parse_inline_gate_phase_index(suffix) else {
                    self.message = "To phase? Use C1PRV1 through C1PRV8.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_gate_travel(caster_index, slot_index, game_dir)
            }
            "RT" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1RT for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    QUICKNESS_SPELL_INDEX,
                    QUICKNESS_COST,
                    QUICKNESS_ACTIVE_EFFECT_TAG,
                    QUICKNESS_ACTIVE_EFFECT_DURATION,
                    "Quickness",
                ))
            }
            "AI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AI for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    NEGATE_MAGIC_SPELL_INDEX,
                    NEGATE_MAGIC_COST,
                    NEGATE_MAGIC_ACTIVE_EFFECT_TAG,
                    NEGATE_MAGIC_ACTIVE_EFFECT_DURATION,
                    "Negate magic",
                ))
            }
            _ => {
                if let Some(spell_index) = spell_index_from_code(&spell_code) {
                    if !spell_allowed_in_area(spell_index, self.area) {
                        self.message = "Not here!".to_string();
                        return Ok(MoveOutcome::Blocked);
                    }
                }
                self.message = "No effect!".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    fn mix_reagents_from_suffix(&mut self, suffix: &str) -> MoveOutcome {
        if self.reagents.iter().all(|count| *count == 0) {
            self.message = "No reagents owned!".to_string();
            return MoveOutcome::Blocked;
        }
        let request = match parse_inline_mix_request(suffix) {
            Ok(Some(request)) => request,
            Ok(None) => {
                self.message = mix_prompt_message();
                return MoveOutcome::PromptDeclined;
            }
            Err(err) => {
                self.message = format!("{err}");
                return MoveOutcome::Blocked;
            }
        };
        if request.amount == 0 {
            self.message = "None!".to_string();
            return MoveOutcome::PromptDeclined;
        }
        if request.reagent_mask == 0 {
            self.message = "Nothing to mix!".to_string();
            return MoveOutcome::Blocked;
        }
        for index in selected_reagent_indices(request.reagent_mask) {
            if self.reagents[index] < request.amount {
                self.message = "Insufficient reagents!".to_string();
                return MoveOutcome::Blocked;
            }
        }

        for index in selected_reagent_indices(request.reagent_mask) {
            self.reagents[index] -= request.amount;
        }
        match request.spell_index {
            Some(spell_index) if request.reagent_mask == SPELL_RECIPE_MASKS[spell_index] => {
                let before = self.spell_charges[spell_index];
                self.spell_charges[spell_index] = self.spell_charges[spell_index]
                    .saturating_add(request.amount)
                    .min(99);
                let gained = self.spell_charges[spell_index].saturating_sub(before);
                self.message = format!(
                    "Mixed {} {} charge{}; stock is {}.",
                    gained,
                    SPELL_CODES[spell_index],
                    if gained == 1 { "" } else { "s" },
                    self.spell_charges[spell_index]
                );
                MoveOutcome::Cast
            }
            Some(spell_index) => {
                self.message = format!(
                    "Mixed wrong reagents for {}; no spell charges added.",
                    SPELL_CODES[spell_index]
                );
                MoveOutcome::Blocked
            }
            None => {
                self.message =
                    "Mixed wrong reagents for unknown spell; no spell charges added.".to_string();
                MoveOutcome::Blocked
            }
        }
    }

    fn shrine_prompt_at_current_position(&self, game_dir: &Path) -> io::Result<Option<String>> {
        Ok(self
            .current_shrine_entry(game_dir)?
            .map(|entry| shrine_prompt_message(entry.virtue)))
    }

    fn meditate_shrine_from_suffix(
        &mut self,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entry) = self.current_shrine_entry(game_dir)? else {
            return Ok(None);
        };
        let request = match parse_inline_shrine_request(suffix) {
            Ok(Some(request)) => request,
            Ok(None) => {
                self.message = shrine_prompt_message(entry.virtue);
                return Ok(Some(MoveOutcome::PromptDeclined));
            }
            Err(err) => {
                self.message = format!("{err}");
                return Ok(Some(MoveOutcome::Blocked));
            }
        };
        if !request.mantra.eq_ignore_ascii_case(entry.virtue.mantra()) {
            self.message = "No effect!".to_string();
            return Ok(Some(MoveOutcome::Blocked));
        }

        let bit = entry.virtue.bit();
        let ordained = self.shrine_ordained_mask & bit != 0;
        let codex = self.shrine_codex_mask & bit != 0;
        let outcome = match (ordained, codex) {
            (false, false) => {
                self.shrine_ordained_mask |= bit;
                self.message = format!(
                    "Meditated at the Shrine of {}; ordained for the Codex quest.",
                    entry.virtue.name()
                );
                MoveOutcome::Observed
            }
            (true, false) => {
                self.message = format!(
                    "Meditated at the Shrine of {}; seek the Codex.",
                    entry.virtue.name()
                );
                MoveOutcome::Observed
            }
            (true, true) => {
                self.shrine_ordained_mask &= !bit;
                let gained = self.add_shrine_standing(entry.virtue, 3);
                let mut stat_notes = self.apply_shrine_stat_reward(entry.virtue);
                if entry.virtue == ShrineVirtue::Humility {
                    let humility_gain = self.add_shrine_standing(entry.virtue, 3);
                    if humility_gain > 0 {
                        stat_notes.push(format!("standing +{humility_gain}"));
                    }
                }
                let stat_note = if stat_notes.is_empty() {
                    "no stat reward".to_string()
                } else {
                    stat_notes.join(", ")
                };
                self.message = format!(
                    "Completed the Shrine of {}; standing +{} to {}; {}.",
                    entry.virtue.name(),
                    gained,
                    self.shrine_standing[entry.virtue.index()],
                    stat_note
                );
                MoveOutcome::Observed
            }
            (false, true) => {
                let Some(offering) = request.offering else {
                    self.message = format!(
                        "Offering? Use M{}/1 through M{}/9, or M{}/0 to cancel.",
                        entry.virtue.mantra(),
                        entry.virtue.mantra(),
                        entry.virtue.mantra()
                    );
                    return Ok(Some(MoveOutcome::PromptDeclined));
                };
                if offering == 0 {
                    self.message = "No effect!".to_string();
                    return Ok(Some(MoveOutcome::PromptDeclined));
                }
                let cost = offering as u16 * 100;
                if self.gold < cost {
                    self.message = format!("Need {cost} gold for offering.");
                    return Ok(Some(MoveOutcome::Blocked));
                }
                self.gold -= cost;
                let gained = self.add_shrine_standing(entry.virtue, offering);
                self.message = format!(
                    "Offered {cost} gold at the Shrine of {}; standing +{} to {}.",
                    entry.virtue.name(),
                    gained,
                    self.shrine_standing[entry.virtue.index()]
                );
                MoveOutcome::Observed
            }
        };
        Ok(Some(outcome))
    }

    fn current_shrine_entry(&self, game_dir: &Path) -> io::Result<Option<ShrineEntry>> {
        let Area::World { plane } = self.area else {
            return Ok(None);
        };
        if plane != WorldPlane::Britannia {
            return Ok(None);
        }
        let Some(entries) = load_shrine_entries(game_dir)? else {
            return Ok(None);
        };
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        Ok(entries.into_iter().find(|entry| {
            entry.plane == plane
                && entry.x == self.player.x
                && entry.y == self.player.y
                && entry
                    .expected_tile
                    .map_or(true, |expected| expected == tile)
        }))
    }

    fn add_shrine_standing(&mut self, virtue: ShrineVirtue, amount: u8) -> u8 {
        let standing = &mut self.shrine_standing[virtue.index()];
        let before = *standing;
        *standing = (*standing).saturating_add(amount).min(SHRINE_STANDING_MAX);
        (*standing).saturating_sub(before)
    }

    fn apply_shrine_stat_reward(&mut self, virtue: ShrineVirtue) -> Vec<String> {
        let mut notes = Vec::new();
        match virtue {
            ShrineVirtue::Honesty => self.add_avatar_intelligence_reward(&mut notes),
            ShrineVirtue::Compassion => self.add_avatar_dexterity_reward(&mut notes),
            ShrineVirtue::Valor => self.add_avatar_strength_reward(&mut notes),
            ShrineVirtue::Justice => {
                self.add_avatar_dexterity_reward(&mut notes);
                self.add_avatar_intelligence_reward(&mut notes);
            }
            ShrineVirtue::Sacrifice => {
                self.add_avatar_strength_reward(&mut notes);
                self.add_avatar_dexterity_reward(&mut notes);
            }
            ShrineVirtue::Honor => {
                self.add_avatar_strength_reward(&mut notes);
                self.add_avatar_intelligence_reward(&mut notes);
            }
            ShrineVirtue::Spirituality => {
                self.add_avatar_strength_reward(&mut notes);
                self.add_avatar_dexterity_reward(&mut notes);
                self.add_avatar_intelligence_reward(&mut notes);
            }
            ShrineVirtue::Humility => {}
        }
        notes
    }

    fn add_avatar_strength_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_strength() {
            notes.push("STR +1".to_string());
        }
    }

    fn add_avatar_dexterity_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_dexterity() {
            self.sync_avatar_dexterity_to_party();
            notes.push("DEX +1".to_string());
        }
    }

    fn add_avatar_intelligence_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_intelligence() {
            notes.push("INT +1".to_string());
        }
    }

    fn sync_avatar_dexterity_to_party(&mut self) {
        if let Some(member) = self.party.iter_mut().find(|member| member.slot == 0) {
            member.climb_stat = self.avatar_stats.dexterity;
        }
    }

}
