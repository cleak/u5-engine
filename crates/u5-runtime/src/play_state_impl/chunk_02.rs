use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    pub fn step_non_town(
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
    pub fn handle_dungeon_key(&mut self, key: char, game_dir: &Path) -> io::Result<bool> {
        self.handle_dungeon_key_with_inline(key, game_dir, None, None, None, None, None)
    }

    /// `commands.md §5`: every dungeon command echoes its resident verb
    /// prefix before the handler prompts or refuses. The echo is opened
    /// speculatively — a key this mode does not handle rolls it back —
    /// and the handler's own prompt or result is folded into it on the
    /// way out. See [`PlayState::begin_command_echo`].
    pub fn handle_dungeon_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_rest: impl Into<InlineRestRequest>,
        inline_drink: Option<bool>,
        inline_party_index: Option<usize>,
        inline_use_request: Option<UseItemRequest>,
        inline_look_focus: Option<DungeonLookFocus>,
    ) -> io::Result<bool> {
        if !matches!(self.area, Area::Dungeon { .. }) {
            return Ok(false);
        }
        if let Some(echo) = dungeon_command_echo(key) {
            self.begin_command_echo(echo);
        }
        let handled = self.handle_dungeon_key_with_inline_inner(
            key,
            game_dir,
            inline_rest,
            inline_drink,
            inline_party_index,
            inline_use_request,
            inline_look_focus,
        );
        match &handled {
            Ok(true) => {
                self.commit_command_echo();
            }
            _ => self.abort_command_echo(),
        }
        handled
    }

    fn handle_dungeon_key_with_inline_inner(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_rest: impl Into<InlineRestRequest>,
        inline_drink: Option<bool>,
        inline_party_index: Option<usize>,
        inline_use_request: Option<UseItemRequest>,
        inline_look_focus: Option<DungeonLookFocus>,
    ) -> io::Result<bool> {
        let inline_rest = inline_rest.into();
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
            let outcome = if let Some(focus) = inline_look_focus {
                self.search_dungeon_focus_with_game_dir(focus, game_dir)?
            } else {
                self.start_dungeon_search_prompt()
            };
            handled!(outcome);
        }
        if key == 'A' {
            handled!(self.attack_command_with_game_dir(None, Some(game_dir))?);
        }
        if matches!(key, 'D' | 'W') {
            self.message = unassigned_refusal_echo(key as u8).to_string();
            handled!();
        }
        if let Some(direction) = high_byte_direction_from_key(key) {
            match direction {
                Direction::North => {
                    handled!(self.step_with_game_dir(self.player.facing, Some(game_dir))?);
                }
                Direction::South => {
                    let facing = self.player.facing;
                    let outcome = if let Some(direction) = facing.opposite_cardinal() {
                        self.step_dungeon_back_with_game_dir(direction, Some(game_dir))?
                    } else {
                        self.message =
                            "Dungeon back-step requires a cardinal facing direction.".to_string();
                        MoveOutcome::Blocked
                    };
                    self.player.facing = facing;
                    handled!(outcome);
                }
                Direction::West => {
                    handled!(self.turn_dungeon(false));
                }
                Direction::East => {
                    handled!(self.turn_dungeon(true));
                }
                Direction::NorthWest
                | Direction::NorthEast
                | Direction::SouthWest
                | Direction::SouthEast => {
                    self.message =
                        "Dungeon movement supports forward, back, and turns only.".to_string();
                    handled!();
                }
            }
        }
        match key.to_ascii_lowercase() {
            '8' | 'w' | '.' | '\r' | '\n' => {
                handled!(self.step_with_game_dir(self.player.facing, Some(game_dir))?);
            }
            '2' | 's' => {
                let facing = self.player.facing;
                let outcome = if let Some(direction) = facing.opposite_cardinal() {
                    self.step_dungeon_back_with_game_dir(direction, Some(game_dir))?
                } else {
                    self.message =
                        "Dungeon back-step requires a cardinal facing direction.".to_string();
                    MoveOutcome::Blocked
                };
                self.player.facing = facing;
                handled!(outcome);
            }
            '4' | 'a' => {
                handled!(self.turn_dungeon(false));
            }
            '6' | 'd' => {
                handled!(self.turn_dungeon(true));
            }
            'k' => {
                handled!(self.klimb_command(game_dir)?);
            }
            '<' => {
                handled!(self.climb(game_dir, ClimbIntent::Up)?);
            }
            '>' => {
                handled!(self.climb(game_dir, ClimbIntent::Down)?);
            }
            'l' => {
                let outcome = if let Some(focus) = inline_look_focus {
                    self.look_dungeon_with_focus(inline_drink, inline_party_index, focus)
                } else {
                    self.start_dungeon_look_prompt(inline_party_index, inline_drink)
                };
                handled!(outcome);
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
                if inline_drink.is_some() {
                    let _ = self.exit_to_dos_prompt(inline_drink);
                } else {
                    let _ = self.start_exit_to_dos_prompt();
                }
                Ok(true)
            }
            'h' => {
                handled!(self.hole_up_command(game_dir, inline_rest)?);
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
                handled!(self.start_cast_spell_prompt());
            }
            'm' => {
                handled!(self.start_mix_reagents_prompt());
            }
            'n' => {
                handled!(self.start_new_order_prompt());
            }
            'r' => {
                handled!(self.start_ready_equipment());
            }
            'u' => {
                let outcome = if inline_use_request.is_some() {
                    let outcome = self.use_item_command(inline_use_request, Some(game_dir))?;
                    self.ensure_use_action_turn(turn_before);
                    outcome
                } else {
                    self.start_use_item()
                };
                handled!(outcome);
            }
            'y' => {
                handled!(self.start_yell_prompt());
            }
            'z' => {
                handled!(self.z_stats_command());
            }
            'j' => {
                handled!(self.jimmy_facing_with_game_dir(Some(game_dir))?);
            }
            _ => Ok(false),
        }
    }

    fn step_dungeon_back_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let Area::Dungeon { scene, level } = self.area else {
            return self.step_with_game_dir(direction, game_dir);
        };
        let (dx, dy) = direction.delta();
        let nx = self.player.x as isize + dx;
        let ny = self.player.y as isize + dy;
        self.step_dungeon_back(direction, nx, ny, scene, level, game_dir)
    }

    /// `commands.md §5`: every overworld and town-family command echoes
    /// its resident verb prefix before the handler prompts or refuses.
    /// See [`PlayState::begin_command_echo`].
    pub fn handle_top_down_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_direction: Option<Direction>,
        inline_rest: impl Into<InlineRestRequest>,
        inline_yes_no: Option<bool>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        if matches!(self.area, Area::Dungeon { .. }) {
            return Ok(false);
        }
        if let Some(echo) = top_down_command_echo(key) {
            self.begin_command_echo(echo);
        }
        let handled = self.handle_top_down_key_with_inline_inner(
            key,
            game_dir,
            inline_direction,
            inline_rest,
            inline_yes_no,
            inline_use_request,
        );
        match &handled {
            Ok(true) => {
                self.commit_command_echo();
            }
            _ => self.abort_command_echo(),
        }
        handled
    }

    fn handle_top_down_key_with_inline_inner(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_direction: Option<Direction>,
        inline_rest: impl Into<InlineRestRequest>,
        inline_yes_no: Option<bool>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        let inline_rest = inline_rest.into();
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
                    if let Some(direction) = inline_direction {
                        handled!(
                            self.attack_command_with_game_dir(Some(direction), Some(game_dir))?
                        );
                    } else {
                        handled!(self.start_attack_direction_prompt());
                    }
                }
                'B' => {
                    handled!(self.board_vehicle());
                }
                'C' => {
                    handled!(self.start_cast_spell_prompt());
                }
                'D' | 'W' => {
                    self.message = unassigned_refusal_echo(key as u8).to_string();
                    handled!();
                }
                'E' => {
                    handled!(self.enter_current_location(game_dir)?);
                }
                'F' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.fire_command(Some(direction), game_dir)?);
                    } else if matches!(
                        (self.area, self.player.transport),
                        (Area::World { .. }, TransportState::Ship { .. })
                    ) {
                        handled!(self.start_fire_direction_prompt());
                    } else {
                        handled!(self.fire_command(None, game_dir)?);
                    }
                }
                'G' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.get_direction_with_game_dir(direction, game_dir)?);
                    } else if matches!(self.area, Area::Dungeon { .. }) {
                        handled!(self.get_facing_with_game_dir(game_dir)?);
                    } else {
                        handled!(self.start_get_direction_prompt());
                    }
                }
                'H' => {
                    handled!(self.hole_up_command(game_dir, inline_rest)?);
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
                    if let Some(direction) = inline_direction {
                        handled!(self.look_direction_with_game_dir(direction, game_dir)?);
                    } else {
                        handled!(self.start_look_direction_prompt());
                    }
                }
                'M' => {
                    if let Some(outcome) = self.read_codex_urn_at_current_position(game_dir)? {
                        handled!(outcome);
                    } else if let Some(outcome) =
                        self.start_shrine_prompt_at_current_position(game_dir)?
                    {
                        handled!(outcome);
                    } else {
                        handled!(self.start_mix_reagents_prompt());
                    }
                }
                'N' => {
                    handled!(self.start_new_order_prompt());
                }
                'O' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.open_direction_with_game_dir(direction, Some(game_dir))?);
                    } else if matches!(self.area, Area::Town { .. }) {
                        handled!(self.start_open_direction_prompt());
                    } else {
                        handled!(self.open_facing_with_game_dir(Some(game_dir))?);
                    }
                }
                'P' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.push_direction_with_game_dir(direction, game_dir)?);
                    } else if matches!(self.area, Area::World { .. } | Area::Town { .. }) {
                        handled!(self.start_push_direction_prompt());
                    } else {
                        handled!(self.push_facing_with_game_dir(game_dir)?);
                    }
                }
                'Q' => {
                    if inline_yes_no.is_some() {
                        handled!(self.save_game_command(game_dir, inline_yes_no)?);
                    } else {
                        handled!(self.start_save_game_prompt());
                    }
                }
                'R' => {
                    handled!(self.start_ready_equipment());
                }
                'S' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.search_direction_with_game_dir(direction, game_dir)?);
                    } else if matches!(self.area, Area::Dungeon { .. }) {
                        handled!(self.search_facing_with_game_dir(game_dir)?);
                    } else {
                        handled!(self.start_search_direction_prompt());
                    }
                }
                'T' => {
                    if matches!(self.area, Area::Town { .. }) {
                        handled!(self.start_talk_direction_prompt());
                    } else {
                        handled!(self.talk_facing_with_game_dir(game_dir)?);
                    }
                }
                'U' => {
                    let outcome = if inline_use_request.is_some() {
                        let outcome = self.use_item_command(inline_use_request, Some(game_dir))?;
                        self.ensure_use_action_turn(turn_before);
                        outcome
                    } else {
                        self.start_use_item()
                    };
                    handled!(outcome);
                }
                'V' => {
                    handled!(self.view_gem());
                }
                'X' => {
                    handled!(self.exit_vehicle_with_game_dir(Some(game_dir))?);
                }
                'Y' => {
                    handled!(self.start_yell_prompt());
                }
                'Z' => {
                    handled!(self.z_stats_command());
                }
                _ => {}
            }
        }

        if let Some(direction) = Direction::from_play_key(key) {
            let town_mode = matches!(self.area, Area::Town { .. });
            let outcome = self.step_with_game_dir(direction, Some(game_dir))?;
            // World movement already owns its landing effects inside
            // `step_world`; town movement reaches the shared town epilogue
            // here so native underfoot reactions run after the consumed step.
            if town_mode {
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
            }
            return Ok(true);
        }

        let outcome = match key.to_ascii_lowercase() {
            'e' => self.enter_current_location(game_dir)?,
            'o' => {
                if let Some(direction) = inline_direction {
                    self.open_direction_with_game_dir(direction, Some(game_dir))?
                } else if matches!(self.area, Area::Town { .. }) {
                    self.start_open_direction_prompt()
                } else {
                    self.open_facing_with_game_dir(Some(game_dir))?
                }
            }
            'l' => {
                if let Some(direction) = inline_direction {
                    self.look_direction_with_game_dir(direction, game_dir)?
                } else {
                    self.look_facing_with_game_dir(game_dir)?
                }
            }
            'v' => self.view_gem(),
            'i' => self.ignite_torch(),
            'h' => self.hole_up_command(game_dir, inline_rest)?,
            'f' => {
                if let Some(direction) = inline_direction {
                    self.fire_command(Some(direction), game_dir)?
                } else if matches!(
                    (self.area, self.player.transport),
                    (Area::World { .. }, TransportState::Ship { .. })
                ) {
                    self.start_fire_direction_prompt()
                } else {
                    self.fire_command(None, game_dir)?
                }
            }
            'p' => {
                if let Some(direction) = inline_direction {
                    self.push_direction_with_game_dir(direction, game_dir)?
                } else if matches!(self.area, Area::World { .. } | Area::Town { .. }) {
                    self.start_push_direction_prompt()
                } else {
                    self.push_facing_with_game_dir(game_dir)?
                }
            }
            'g' => {
                if let Some(direction) = inline_direction {
                    self.get_direction_with_game_dir(direction, game_dir)?
                } else if matches!(self.area, Area::Dungeon { .. }) {
                    self.get_facing_with_game_dir(game_dir)?
                } else {
                    self.start_get_direction_prompt()
                }
            }
            't' => {
                if matches!(self.area, Area::Town { .. }) {
                    self.start_talk_direction_prompt()
                } else {
                    self.talk_facing_with_game_dir(game_dir)?
                }
            }
            'j' => self.jimmy_facing_with_game_dir(Some(game_dir))?,
            'k' => self.klimb_command(game_dir)?,
            'x' => self.exit_vehicle_with_game_dir(Some(game_dir))?,
            'm' => {
                if let Some(outcome) = self.read_codex_urn_at_current_position(game_dir)? {
                    outcome
                } else if let Some(outcome) =
                    self.start_shrine_prompt_at_current_position(game_dir)?
                {
                    outcome
                } else {
                    self.start_mix_reagents_prompt()
                }
            }
            'z' => self.z_stats_command(),
            'c' => self.start_cast_spell_prompt(),
            'n' => self.start_new_order_prompt(),
            'r' => self.start_ready_equipment(),
            'u' => {
                if inline_use_request.is_some() {
                    let outcome = self.use_item_command(inline_use_request, Some(game_dir))?;
                    self.ensure_use_action_turn(turn_before);
                    outcome
                } else {
                    self.start_use_item()
                }
            }
            'y' => self.start_yell_prompt(),
            '<' => self.climb(game_dir, ClimbIntent::Up)?,
            '>' => self.climb(game_dir, ClimbIntent::Down)?,
            '.' => self.idle_tick(),
            _ => return Ok(false),
        };
        self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        Ok(true)
    }

    pub fn cast_spell_from_suffix(
        &mut self,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let spell_code = inline_spell_code(suffix);
        if spell_code.is_empty() {
            self.message = cast_prompt_message();
            return Ok(MoveOutcome::Blocked);
        }
        let spell_index = spell_index_from_code(&spell_code);
        if spell_index.is_some()
            && parse_inline_party_index(suffix).is_some()
            && spell_index != Some(TIME_STOP_SPELL_INDEX)
            && self.current_scene_absorbs_casts()
        {
            self.message = "Absorbed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if spell_index.is_some()
            && parse_inline_party_index(suffix).is_some()
            && self.combat_active
            && resolve_negate_magic_absorbs_combat_cast(
                self.active_effect_tag,
                self.active_effect_counter,
            )
        {
            self.message = "Magic absorbed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        // `magic.md §5` step 3: the context gate runs before the handler,
        // so `Not here!` precedes any handler-specific direction/target
        // prompt and costs neither a charge nor a turn. Blink used to be
        // exempted here and re-tested inside its own handlers; that let the
        // dungeon/town "Direction?" prompt appear for a spell whose
        // published mask is `C/O` (`catalogs/spell-list.md §5`, id 17).
        if let (Some(spell_index), Some(caster_index)) =
            (spell_index, parse_inline_party_index(suffix))
            && self.party.get(caster_index).is_some()
            && !self.spell_allowed_in_current_cast_context(spell_index)
        {
            self.message = "Not here!".to_string();
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
                self.cast_magic_lock(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                    game_dir,
                )
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
                    self.message = "Who casts? Use C1AS6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_open_spell(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                    game_dir,
                )
            }
            "ACX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1ACX for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_repel_undead(caster_index))
            }
            "AT" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AT for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_time_stop(caster_index))
            }
            "AY" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AY6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_vanish(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                ))
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
                    self.message = "Who casts? Use C1AZ for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_awaken(caster_index))
            }
            "KX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1KX for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_conjure_spell(caster_index, spell_index.unwrap()))
            }
            "AEX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AEX7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Creature? Use C1AEX7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_charm_spell(caster_index, spell_index.unwrap(), target_slot))
            }
            "BRX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1BRX7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Creature? Use C1BRX7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_polymorph_spell(
                    caster_index,
                    spell_index.unwrap(),
                    target_slot,
                ))
            }
            "BIX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1BIX for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_swarm_spell(caster_index, spell_index.unwrap()))
            }
            "IQX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IQX7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Creature? Use C1IQX7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_clone_spell(caster_index, spell_index.unwrap(), target_slot))
            }
            "CX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1CX7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1CX7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_target_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatSpellDamageKind::Kill,
                    target_slot,
                ))
            }
            "DP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1DP for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_dungeon_level_spell(
                    caster_index,
                    DES_POR_SPELL_INDEX,
                    1,
                    "Down",
                    game_dir,
                )
            }
            "FV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1FV7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1FV7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_target_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatSpellDamageKind::Fireball,
                    target_slot,
                ))
            }
            "FGI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1FGI6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        FIRE_FIELD_SPELL_INDEX,
                        FIELD_SPELL_COST,
                        CombatArenaFieldKind::Fire,
                        parse_inline_combat_spell_coordinate(suffix, "FGI"),
                    ));
                }
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
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        POISON_FIELD_SPELL_INDEX,
                        FIELD_SPELL_COST,
                        CombatArenaFieldKind::Poison,
                        parse_inline_combat_spell_coordinate(suffix, "GIN"),
                    ));
                }
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
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        SLEEP_FIELD_SPELL_INDEX,
                        FIELD_SPELL_COST,
                        CombatArenaFieldKind::Sleep,
                        parse_inline_combat_spell_coordinate(suffix, "GIZ"),
                    ));
                }
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
            "GP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1GP7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1GP7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_target_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatSpellDamageKind::MagicMissile,
                    target_slot,
                ))
            }
            "GIS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1GIS6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        ENERGY_FIELD_SPELL_INDEX,
                        ENERGY_FIELD_COST,
                        CombatArenaFieldKind::Energy,
                        parse_inline_combat_spell_coordinate(suffix, "GIS"),
                    ));
                }
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
                if self.combat_active {
                    return Ok(self.cast_combat_blink_to_coordinate(
                        caster_index,
                        parse_inline_blink_combat_coordinate(suffix),
                    ));
                }
                self.cast_blink(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix)
                        && parse_inline_cardinal_direction(suffix).is_none(),
                    game_dir,
                )
            }
            "IPVY" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IPVY for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_tremor_combat_spell(caster_index, spell_index.unwrap()))
            }
            "IZ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1IZ7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.message = "Direction? Use C1IZ6 for east.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::Sleep,
                    Some(direction),
                ))
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
            "LS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1LS for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_invisibility(caster_index))
            }
            "HIN" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1HIN7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.message = "Direction? Use C1HIN6 for east.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::PoisonWind,
                    Some(direction),
                ))
            }
            "HR" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1HR for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                let direction = parse_inline_cardinal_direction(suffix);
                Ok(self.cast_rel_hur(
                    caster_index,
                    direction,
                    direction.is_none() && suffix.ends_with(' '),
                ))
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
                self.cast_unlock_magic(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                    game_dir,
                )
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
            "CKX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1CKX for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_summon_daemon_spell(caster_index, spell_index.unwrap()))
            }
            "CGIV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1CGIV7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.message = "Direction? Use C1CGIV6 for east.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::DeathWind,
                    Some(direction),
                ))
            }
            "CIQ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1CIQ for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_cause_fear(caster_index))
            }
            "FHI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1FHI7 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.message = "Direction? Use C1FHI6 for east.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::FlameWind,
                    Some(direction),
                ))
            }
            "PU" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1PU for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_dungeon_level_spell(caster_index, UUS_POR_SPELL_INDEX, -1, "Up", game_dir)
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
            "QW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1QW for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_reveal(caster_index))
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
            "AQW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.message = "Who casts? Use C1AQW for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    MASS_CHARM_SPELL_INDEX,
                    MASS_CHARM_COST,
                    MASS_CHARM_ACTIVE_EFFECT_TAG,
                    MASS_CHARM_ACTIVE_EFFECT_DURATION,
                    "Mass charm",
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
                if let Some(spell_index) = spell_index {
                    if !self.spell_allowed_in_current_cast_context(spell_index) {
                        self.message = "Not here!".to_string();
                        return Ok(MoveOutcome::Blocked);
                    }
                }
                self.message = "No effect!".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn current_scene_absorbs_casts(&self) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        if scene.byte == STONEGATE_SCENE_BYTE {
            return true;
        }
        scene.byte == LORD_BLACKTHORN_CASTLE_SCENE_BYTE
            && self.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] == 0
    }

    pub fn mix_reagents_from_suffix(&mut self, suffix: &str) -> MoveOutcome {
        if self.reagents.iter().all(|count| *count == 0) {
            self.message = MMIX_NO_REAGENTS_OWNED_MESSAGE.to_string();
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
            self.message = MMIX_EMPTY_SELECTION_MESSAGE.to_string();
            return MoveOutcome::Blocked;
        }
        for index in selected_reagent_indices(request.reagent_mask) {
            if self.reagents[index] < request.amount {
                self.message = MMIX_INSUFFICIENT_REAGENTS_MESSAGE.to_string();
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
                let base = format!(
                    "Mixed wrong reagents for {}; no spell charges added.",
                    SPELL_CODES[spell_index]
                );
                self.message = self.wrong_mix_trap_message(base);
                MoveOutcome::Blocked
            }
            None => {
                let base =
                    "Mixed wrong reagents for unknown spell; no spell charges added.".to_string();
                self.message = self.wrong_mix_trap_message(base);
                MoveOutcome::Blocked
            }
        }
    }

    pub fn wrong_mix_trap_message(&mut self, base: String) -> String {
        let target_slot = self.mixer_trap_target_slot();
        let trap = self.apply_shared_trap_effect_to_slot(target_slot);
        format!("{base}\n{trap}")
    }

    /// `traps.md §4` (M-Mix): the mixer supplies its own victim slot and
    /// does **not** use the §2.1 acting-member selection. Before calling
    /// the trap-effect resolver it refreshes its target to the **first**
    /// travelling member currently marked Good or Poisoned.
    ///
    /// When no such member exists the original leaves the target holding
    /// whatever it last held and the trap lands on that stale value;
    /// `traps.md` §4 names that undefined behaviour and tells a port to
    /// decide deliberately rather than invent a fallback. The
    /// `.or(active)/0` tail is that deliberate choice, not published
    /// behaviour. `cleak/u5-spec#89` re-confirmed the case is explicitly
    /// undefined, so the choice stays as it is.
    ///
    /// The two container call sites do **not** share this helper: §2.1
    /// publishes a real selection rule for them, implemented in
    /// [`Self::shared_acting_member_selection`].
    pub fn mixer_trap_target_slot(&self) -> usize {
        self.party
            .iter()
            .position(|member| matches!(member.status, b'G' | b'P'))
            .or(self.active_player)
            .unwrap_or(0)
    }

    /// `traps.md §2.1`: the shared acting-member selection, in priority
    /// order.
    ///
    /// 1. **During a combat-class scene**, the party slot bound to the
    ///    combatant whose turn is in progress, chosen silently: no prompt,
    ///    no status test.
    /// 2. **Otherwise, when a single active character is set**, that
    ///    character, returned directly and silently, with **no status
    ///    re-check** — so a member who has become Asleep or Charmed since
    ///    the hint was set can still be the trap victim.
    /// 3. **Otherwise**, the Good-or-Poisoned scan of
    ///    [`acting_member_scan`].
    ///
    /// The scoping matters and was nearly published wrong: "a party with
    /// no able-bodied member can never spring a container trap" holds
    /// **only** outside a combat-class scene and **only** with no active
    /// character set. Both override branches skip the status test
    /// entirely, so neither is covered by that guarantee.
    ///
    /// `allow_combat_override` selects whether branch 1 is reachable. The
    /// `O` Open dispatcher routes only a narrow band of dungeon scenes to
    /// the dungeon chest handler and every other scene — combat-class
    /// scenes included — to the surface/town handler, so per §4 the combat
    /// override can fire at the surface/town container site and can
    /// **never** fire at the dungeon chest site.
    ///
    /// §2.1 records that the original range-checks neither the combatant
    /// index nor its "is a party member" flag, and that a port which does
    /// range-check is safe against every reachable case published. This
    /// range-checks.
    pub fn shared_acting_member_selection(
        &self,
        allow_combat_override: bool,
    ) -> ActingMemberSelection {
        if allow_combat_override && self.combat_active {
            if let Some(slot) = self
                .pending_combat_actor_slot
                .filter(|slot| *slot < self.party.len() && *slot < COMBAT_PARTY_ACTOR_SLOTS)
            {
                return ActingMemberSelection::Selected(slot);
            }
        }
        if let Some(slot) = self.active_player.filter(|slot| *slot < self.party.len()) {
            return ActingMemberSelection::Selected(slot);
        }
        let statuses: Vec<u8> = self
            .party
            .iter()
            .take(COMBAT_PARTY_ACTOR_SLOTS)
            .map(|member| member.status)
            .collect();
        acting_member_scan(&statuses)
    }

    /// `traps.md §2.1`/§4: the surface/town container site's acting-member
    /// selection. The Open dispatcher sends combat-class scenes here, so
    /// the combat override is reachable at this site.
    pub fn surface_container_acting_member(&self) -> ActingMemberSelection {
        self.shared_acting_member_selection(true)
    }

    /// `traps.md §2.1`/§4: the dungeon chest site's acting-member
    /// selection. Only the narrow dungeon-scene band reaches the dungeon
    /// chest handler, and combat-class scene values sit far above that
    /// band, so the combat override can never fire here.
    pub fn dungeon_container_acting_member(&self) -> ActingMemberSelection {
        self.shared_acting_member_selection(false)
    }

    /// `traps.md §2-3`: dispatch the resolved effect family. The family
    /// classification lives in [`shared_trap_effect_family_from_index`]
    /// so the resolver and the published-predicate helpers cannot drift
    /// apart; this match is exhaustive over [`TrapEffect`] rather than
    /// falling through a catch-all arm.
    pub fn apply_shared_trap_effect_to_slot(&mut self, triggering_slot: usize) -> String {
        match self.shared_trap_effect_family(triggering_slot) {
            TrapEffect::Acid => self.apply_acid_trap_effect(triggering_slot),
            TrapEffect::Poison => self.apply_poison_trap_effect(triggering_slot),
            TrapEffect::Bomb => self.apply_bomb_trap_effect(triggering_slot),
            TrapEffect::Gas => self.apply_gas_trap_effect(),
        }
    }

    pub fn shared_trap_effect_id(&self, triggering_slot: usize) -> u8 {
        let seed = self.shared_trap_seed(triggering_slot, 0);
        shared_trap_effect_id_from_index(seed, self.combat_active)
    }

    /// `traps.md §3`: the effect family selected for this trigger, using
    /// the same combat/non-combat split as [`Self::shared_trap_effect_id`].
    pub fn shared_trap_effect_family(&self, triggering_slot: usize) -> TrapEffect {
        let seed = self.shared_trap_seed(triggering_slot, 0);
        shared_trap_effect_family_from_index(seed, self.combat_active)
    }

    pub fn shared_trap_seed(&self, triggering_slot: usize, salt: u8) -> u8 {
        (self.turn as u8)
            .wrapping_add((self.player.x as u8).wrapping_mul(3))
            .wrapping_add((self.player.y as u8).wrapping_mul(5))
            .wrapping_add((triggering_slot as u8).wrapping_mul(17))
            .wrapping_add(salt)
    }

    pub fn shared_trap_damage_roll(
        &self,
        triggering_slot: usize,
        target_slot: usize,
        max_damage: u8,
        salt: u8,
    ) -> u8 {
        let seed = self
            .shared_trap_seed(triggering_slot, salt)
            .wrapping_add((target_slot as u8).wrapping_mul(13));
        shared_trap_damage_from_index(seed, max_damage)
    }

    pub fn apply_acid_trap_effect(&mut self, triggering_slot: usize) -> String {
        let damage =
            self.shared_trap_damage_roll(triggering_slot, triggering_slot, TRAP_ACID_DAMAGE_MAX, 3);
        let Some(member) = self.party.get_mut(triggering_slot) else {
            return format!(
                "Acid trap found no party member in slot {}.",
                triggering_slot + 1
            );
        };

        let applied = member.apply_damage(damage);
        if member.hp == 0 && self.active_player == Some(triggering_slot) {
            self.active_player = None;
        }
        format!(
            "Acid trap hit party member {} for {applied} HP.",
            triggering_slot + 1
        )
    }

    /// `traps.md §3` effect id 1: apply Poisoned status to the triggering
    /// slot through the shared poison helper. A member already marked
    /// Dead is skipped and left Dead.
    pub fn apply_poison_trap_effect(&mut self, triggering_slot: usize) -> String {
        if self.apply_trap_poison_status_to_slot(triggering_slot) {
            format!("Poison trap poisoned party member {}.", triggering_slot + 1)
        } else {
            format!(
                "Poison trap had no effect on party member {}.",
                triggering_slot + 1
            )
        }
    }

    pub fn apply_bomb_trap_effect(&mut self, triggering_slot: usize) -> String {
        let mut affected = 0usize;
        let mut total_applied = 0u16;
        let limit = self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
        for target_slot in 0..limit {
            if !self.party[target_slot].living() {
                continue;
            }
            let damage = self.shared_trap_damage_roll(
                triggering_slot,
                target_slot,
                TRAP_BOMB_DAMAGE_MAX,
                11,
            );
            let applied = self.party[target_slot].apply_damage(damage);
            if applied != 0 {
                affected += 1;
                total_applied = total_applied.saturating_add(applied);
            }
            if self.party[target_slot].hp == 0 && self.active_player == Some(target_slot) {
                self.active_player = None;
            }
        }
        format!("Bomb trap dealt {total_applied} HP across {affected} party member(s).")
    }

    /// `traps.md §3` effect id 3: apply Poisoned status across the
    /// six-slot party band through the same helper as effect id 1.
    /// Empty roster positions and Dead members are skipped by the helper.
    pub fn apply_gas_trap_effect(&mut self) -> String {
        let mut poisoned = 0usize;
        for slot in 0..COMBAT_PARTY_ACTOR_SLOTS {
            if self.apply_trap_poison_status_to_slot(slot) {
                poisoned += 1;
            }
        }
        format!("Gas trap poisoned {poisoned} party member(s).")
    }

    /// `traps.md §3`: the shared trap poison-status primitive used by
    /// effect ids 1 and 3. A slot outside the current party count is
    /// ignored and a member already marked Dead is skipped and left
    /// Dead; any other in-party member is rewritten to Poisoned.
    /// Nothing else changes — no hit points, maxima, or magic points,
    /// and no relation to the resurrection spell path.
    pub fn apply_trap_poison_status_to_slot(&mut self, slot: usize) -> bool {
        let Some(member) = self.party.get_mut(slot) else {
            return false;
        };
        if !trap_poison_accepts_status_byte(member.status) {
            return false;
        }
        member.status = TRAP_POISON_STATUS_BYTE;
        true
    }

    pub fn shrine_prompt_at_current_position(&self, game_dir: &Path) -> io::Result<Option<String>> {
        Ok(self
            .current_shrine_entry(game_dir)?
            .map(|entry| shrine_prompt_message(entry.virtue)))
    }

    pub fn start_shrine_prompt_at_current_position(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entry) = self.current_shrine_entry(game_dir)? else {
            return Ok(None);
        };
        self.active_shrine = Some(ShrineSession::new(entry.virtue));
        self.message = self.render_active_shrine();
        Ok(Some(MoveOutcome::Observed))
    }

    pub fn render_active_shrine(&self) -> String {
        self.active_shrine
            .as_ref()
            .map(|session| self.render_shrine_session(session))
            .unwrap_or_else(|| "Mantra?".to_string())
    }

    fn render_shrine_session(&self, session: &ShrineSession) -> String {
        match session.phase {
            ShrinePhase::Mantra => {
                let mantra = if session.mantra_buffer.is_empty() {
                    "_".to_string()
                } else {
                    session.mantra_buffer.clone()
                };
                format!(
                    "Shrine of {} mantra? {mantra}\nType up to {SHRINE_MANTRA_INPUT_LIMIT} characters; Enter accepts; Esc cancels.",
                    session.virtue.name()
                )
            }
            ShrinePhase::Offering => {
                format!(
                    // cleak/u5-spec#81: offering prompt literal unpublished; the
                    // invented instructional line is removed.
                    "Offering at the Shrine of {}? _",
                    session.virtue.name()
                )
            }
        }
    }

    pub fn step_active_shrine(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(mut session) = self.active_shrine.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            match session.phase {
                ShrinePhase::Mantra => {
                    if ch == '\u{1b}' {
                        self.message = "None!".to_string();
                        return Ok(Some(MoveOutcome::PromptDeclined));
                    }
                    match ch {
                        '\r' | '\n' => {
                            return self.complete_active_shrine_mantra(session, game_dir);
                        }
                        '\u{8}' | '\u{7f}' => {
                            session.mantra_buffer.pop();
                        }
                        ch if !ch.is_control()
                            && session.mantra_buffer.len() < SHRINE_MANTRA_INPUT_LIMIT =>
                        {
                            session.mantra_buffer.push(ch);
                        }
                        _ => {}
                    }
                }
                ShrinePhase::Offering => {
                    if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                        self.message = "No effect!".to_string();
                        return Ok(Some(MoveOutcome::PromptDeclined));
                    }
                    let Some(digit) = ch.to_digit(10).and_then(|digit| u8::try_from(digit).ok())
                    else {
                        continue;
                    };
                    if !(1..=9).contains(&digit) {
                        continue;
                    }
                    if let Some(cost) = ShrineVirtue::shrine_offering_cost(digit) {
                        if self.gold < cost {
                            self.message = format!(
                                "Need {cost} gold for offering.\n{}",
                                self.render_shrine_session(&session)
                            );
                            self.active_shrine = Some(session);
                            return Ok(None);
                        }
                    }
                    let suffix = format!("{}/{}", session.mantra_buffer, digit);
                    return Ok(self.meditate_shrine_from_suffix(&suffix, game_dir)?);
                }
            }
        }
        self.message = self.render_shrine_session(&session);
        self.active_shrine = Some(session);
        Ok(None)
    }

    fn complete_active_shrine_mantra(
        &mut self,
        mut session: ShrineSession,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if session.mantra_buffer.is_empty() {
            self.message = "No effect!".to_string();
            return Ok(Some(MoveOutcome::Blocked));
        }
        let mantra_matches = session
            .mantra_buffer
            .eq_ignore_ascii_case(session.virtue.mantra());
        if mantra_matches {
            let bit = session.virtue.bit();
            let ordained = self.shrine_ordained_mask & bit != 0;
            let codex = self.shrine_codex_mask & bit != 0;
            if !ordained && codex {
                session.phase = ShrinePhase::Offering;
                self.message = self.render_shrine_session(&session);
                self.active_shrine = Some(session);
                return Ok(None);
            }
        }
        self.meditate_shrine_from_suffix(&session.mantra_buffer, game_dir)
    }

    pub fn meditate_shrine_from_suffix(
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
                // karma.md §3-4: shared moral-standing selector +3 on Codex
                // turn-in, with Humility receiving an additional +3.
                let mut moral_gained =
                    self.add_moral_standing(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE);
                let stat_notes = self.apply_shrine_stat_reward(entry.virtue);
                if entry.virtue == ShrineVirtue::Humility {
                    let humility_moral =
                        self.add_moral_standing(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE);
                    moral_gained = moral_gained.saturating_add(humility_moral);
                }
                let stat_note = if stat_notes.is_empty() {
                    "no stat reward".to_string()
                } else {
                    stat_notes.join(", ")
                };
                self.message = format!(
                    "Completed the Shrine of {}; moral +{} to {}; {}.",
                    entry.virtue.name(),
                    moral_gained,
                    self.moral_standing,
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
                // karma.md §3-4: completed-shrine gold offering adds the
                // offered digit to the shared moral-standing selector.
                let moral_gained = self.add_moral_standing(offering);
                self.message = format!(
                    "Offered {cost} gold at the Shrine of {}; moral +{} to {}.",
                    entry.virtue.name(),
                    moral_gained,
                    self.moral_standing
                );
                MoveOutcome::Observed
            }
        };
        Ok(Some(outcome))
    }

    pub fn read_codex_urn_at_current_position(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(_entry) = self.current_codex_urn_entry(game_dir)? else {
            return Ok(None);
        };
        self.message = match read_codex_urn(self.shrine_ordained_mask, &mut self.shrine_codex_mask)
        {
            CodexUrnReadOutcome::Completed => {
                "Codex urn: all virtue pages have already been read.".to_string()
            }
            CodexUrnReadOutcome::NoOrdained => {
                "Codex urn: no ordained virtue is ready.".to_string()
            }
            CodexUrnReadOutcome::Stamped(virtue) => {
                let status = format!("Read Codex page for {}; Codex-read bit set.", virtue.name());
                match self.codex_urn_text_for_virtue(game_dir, virtue)? {
                    Some(text) if !text.is_empty() => format!("{status} {text}"),
                    _ => status,
                }
            }
        };
        Ok(Some(MoveOutcome::Observed))
    }

    pub fn codex_urn_text_for_virtue(
        &self,
        game_dir: &Path,
        virtue: ShrineVirtue,
    ) -> io::Result<Option<String>> {
        let Some(messages) = load_misc_messages(game_dir)? else {
            return Ok(None);
        };
        Ok(messages
            .urn_codex_for_virtue_index(virtue.index())
            .map(render_miscmsg_tile_glyph_text))
    }

    pub fn current_codex_urn_entry(&self, game_dir: &Path) -> io::Result<Option<CodexUrnEntry>> {
        let Area::World { plane } = self.area else {
            return Ok(None);
        };
        let Some(entries) = load_codex_urn_entries(game_dir)? else {
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

    pub fn current_shrine_entry(&self, game_dir: &Path) -> io::Result<Option<ShrineEntry>> {
        let Area::World { plane } = self.area else {
            return Ok(None);
        };
        if plane != WorldPlane::Britannia {
            return Ok(None);
        }
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        if let Some(entries) = load_shrine_entries(game_dir)? {
            if let Some(entry) = entries.into_iter().find(|entry| {
                entry.plane == plane
                    && entry.x == self.player.x
                    && entry.y == self.player.y
                    && entry
                        .expected_tile
                        .map_or(true, |expected| expected == tile)
            }) {
                return Ok(Some(entry));
            }
        }
        Ok(
            shrine_virtue_for_altar_tile(tile).map(|virtue| ShrineEntry {
                plane,
                x: self.player.x,
                y: self.player.y,
                virtue,
                expected_tile: Some(tile),
            }),
        )
    }

    pub fn apply_shrine_stat_reward(&mut self, virtue: ShrineVirtue) -> Vec<String> {
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

    pub fn add_avatar_strength_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_strength() {
            notes.push("STR +1".to_string());
        }
    }

    pub fn add_avatar_dexterity_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_dexterity() {
            self.sync_avatar_dexterity_to_party();
            notes.push("DEX +1".to_string());
        }
    }

    pub fn add_avatar_intelligence_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_intelligence() {
            notes.push("INT +1".to_string());
        }
    }

    pub fn sync_avatar_dexterity_to_party(&mut self) {
        if let Some(member) = self.party.iter_mut().find(|member| member.slot == 0) {
            member.climb_stat = self.avatar_stats.dexterity;
        }
    }
}

pub(crate) fn high_byte_direction_from_key(key: char) -> Option<Direction> {
    let scalar = key as u32;
    if scalar > u8::MAX as u32 {
        return None;
    }
    match input_code_direction(scalar as u8)? {
        InputDirection::North => Some(Direction::North),
        InputDirection::South => Some(Direction::South),
        InputDirection::East => Some(Direction::East),
        InputDirection::West => Some(Direction::West),
        InputDirection::Northwest => Some(Direction::NorthWest),
        InputDirection::Northeast => Some(Direction::NorthEast),
        InputDirection::Southwest => Some(Direction::SouthWest),
        InputDirection::Southeast => Some(Direction::SouthEast),
    }
}
