use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
        self.handle_dungeon_key_with_inline(key, game_dir, None, None, None, None)
    }

    pub fn handle_dungeon_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_rest: impl Into<InlineRestRequest>,
        inline_drink: Option<bool>,
        inline_party_index: Option<usize>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        let inline_rest = inline_rest.into();
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
        if key == 'A' {
            handled!(self.attack_command_with_game_dir(None, Some(game_dir))?);
        }
        match key.to_ascii_lowercase() {
            '8' | 'w' => {
                handled!(self.step_with_game_dir(self.player.facing, Some(game_dir))?);
            }
            '2' | 's' => {
                let facing = self.player.facing;
                let outcome = if let Some(direction) = facing.opposite_cardinal() {
                    let outcome = self.step_with_game_dir(direction, Some(game_dir))?;
                    if matches!(self.area, Area::Dungeon { .. }) {
                        self.player.facing = facing;
                    }
                    outcome
                } else {
                    self.message =
                        "Dungeon back-step requires a cardinal facing direction.".to_string();
                    MoveOutcome::Blocked
                };
                handled!(outcome);
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
                let tile = self.dungeon_cell(level, self.player.x, self.player.y);
                let outcome = if tile == 0x60 {
                    self.climb(game_dir, ClimbIntent::Up)?
                } else {
                    match tile >> 4 {
                        0x1 => self.climb(game_dir, ClimbIntent::Up)?,
                        0x2 => self.climb(game_dir, ClimbIntent::Down)?,
                        0x3 => {
                            self.message =
                                "Two-way ladder: use < or > to choose a climb direction."
                                    .to_string();
                            MoveOutcome::Blocked
                        }
                        _ => {
                            self.message = "Not climbable!".to_string();
                            MoveOutcome::Blocked
                        }
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
                handled!(self.start_ready_equipment());
            }
            'u' => {
                let outcome = if inline_use_request.is_some() {
                    self.use_item_command(inline_use_request, Some(game_dir))?
                } else {
                    self.start_use_item()
                };
                handled!(outcome);
            }
            'y' => {
                handled!(self.yell_command(None));
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

    pub fn handle_top_down_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_direction: Option<Direction>,
        inline_rest: impl Into<InlineRestRequest>,
        inline_yes_no: Option<bool>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        let inline_rest = inline_rest.into();
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
                    handled!(self.attack_command_with_game_dir(inline_direction, Some(game_dir))?);
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
                    handled!(self.look_facing_with_game_dir(game_dir)?);
                }
                'M' => {
                    if let Some(outcome) = self.read_codex_urn_at_current_position(game_dir)? {
                        handled!(outcome);
                    } else {
                        self.message = self
                            .shrine_prompt_at_current_position(game_dir)?
                            .unwrap_or_else(mix_prompt_message);
                        handled!();
                    }
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
                    handled!(self.start_ready_equipment());
                }
                'S' => {
                    handled!(self.search_facing_with_game_dir(game_dir)?);
                }
                'T' => {
                    handled!(self.talk_facing_with_game_dir(game_dir)?);
                }
                'U' => {
                    let outcome = if inline_use_request.is_some() {
                        self.use_item_command(inline_use_request, Some(game_dir))?
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
                    handled!(self.yell_command(None));
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
            'h' => self.hole_up_command(game_dir, inline_rest)?,
            'f' => self.fire_command(inline_direction, game_dir)?,
            'p' => self.push_facing_with_game_dir(game_dir)?,
            'g' => self.get_facing_with_game_dir(game_dir)?,
            't' => self.talk_facing_with_game_dir(game_dir)?,
            'j' => self.jimmy_facing_with_game_dir(Some(game_dir))?,
            'k' => self.klimb_command(game_dir)?,
            'x' => self.exit_vehicle_with_game_dir(Some(game_dir))?,
            'm' => {
                if let Some(outcome) = self.read_codex_urn_at_current_position(game_dir)? {
                    outcome
                } else {
                    self.message = self
                        .shrine_prompt_at_current_position(game_dir)?
                        .unwrap_or_else(mix_prompt_message);
                    MoveOutcome::Observed
                }
            }
            'z' => self.z_stats(),
            'r' => self.start_ready_equipment(),
            'u' => {
                if inline_use_request.is_some() {
                    self.use_item_command(inline_use_request, Some(game_dir))?
                } else {
                    self.start_use_item()
                }
            }
            'y' => self.yell_command(None),
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
            && self.current_scene_absorbs_casts()
        {
            self.message = "Absorbed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if spell_index.is_some()
            && parse_inline_party_index(suffix).is_some()
            && self.combat_active
            && self.active_effect_tag == Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG)
        {
            self.message = "Magic absorbed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if let (Some(spell_index), Some(caster_index)) =
            (spell_index, parse_inline_party_index(suffix))
        {
            if spell_index != BLINK_SPELL_INDEX
                && self.party.get(caster_index).is_some()
                && !self.spell_allowed_in_current_cast_context(spell_index)
            {
                self.message = "Not here!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
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
                    self.message = "Who casts? Use C1AS6 for party slot 1.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_open_spell(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
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
                Ok(self.cast_vanish(caster_index, parse_inline_cardinal_direction(suffix)))
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
                Ok(self.cast_dungeon_level_spell(caster_index, DES_POR_SPELL_INDEX, 1, "Down"))
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
                        parse_inline_cardinal_direction(suffix),
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
                        parse_inline_cardinal_direction(suffix),
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
                        parse_inline_cardinal_direction(suffix),
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
                        parse_inline_cardinal_direction(suffix),
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
                self.cast_blink(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
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
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1IZ7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::Sleep,
                    target_slot,
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
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1HIN7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::PoisonWind,
                    target_slot,
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
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1CGIV7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::DeathWind,
                    target_slot,
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
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.message = "Target? Use C1FHI7 to target combat slot 7.".to_string();
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::FlameWind,
                    target_slot,
                ))
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
        let target_slot = self.shared_trap_default_target_slot();
        let trap = self.apply_shared_trap_effect_to_slot(target_slot);
        format!("{base}\n{trap}")
    }

    pub fn shared_trap_default_target_slot(&self) -> usize {
        self.party
            .iter()
            .position(|member| matches!(member.status, b'G' | b'P'))
            .or(self.active_player)
            .unwrap_or(0)
    }

    pub fn apply_shared_trap_effect_to_slot(&mut self, triggering_slot: usize) -> String {
        match self.shared_trap_effect_id(triggering_slot) {
            0 => self.apply_acid_trap_effect(triggering_slot),
            1 => self.apply_poison_trap_effect(triggering_slot),
            2 => self.apply_bomb_trap_effect(triggering_slot),
            _ => self.apply_gas_trap_effect(),
        }
    }

    pub fn shared_trap_effect_id(&self, triggering_slot: usize) -> u8 {
        let seed = self.shared_trap_seed(triggering_slot, 0);
        shared_trap_effect_id_from_index(seed, self.combat_active)
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

    pub fn apply_poison_trap_effect(&mut self, triggering_slot: usize) -> String {
        if self.revive_dead_party_member_as_poisoned(triggering_slot) {
            format!(
                "Poison trap revived party member {} as poisoned.",
                triggering_slot + 1
            )
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

    pub fn apply_gas_trap_effect(&mut self) -> String {
        let mut revived = 0usize;
        for slot in 0..self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS) {
            if self.revive_dead_party_member_as_poisoned(slot) {
                revived += 1;
            }
        }
        format!("Gas trap revived {revived} dead party member(s) as poisoned.")
    }

    pub fn revive_dead_party_member_as_poisoned(&mut self, slot: usize) -> bool {
        let Some(member) = self.party.get_mut(slot) else {
            return false;
        };
        if member.status == b'D' {
            member.status = b'P';
            true
        } else {
            false
        }
    }

    pub fn shrine_prompt_at_current_position(&self, game_dir: &Path) -> io::Result<Option<String>> {
        Ok(self
            .current_shrine_entry(game_dir)?
            .map(|entry| shrine_prompt_message(entry.virtue)))
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
                let gained = self.add_shrine_standing(
                    entry.virtue,
                    ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE,
                );
                // karma.md §3-4: shared moral-standing selector +3 on Codex
                // turn-in, with Humility receiving an additional +3.
                let moral_gained =
                    self.add_moral_standing(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE);
                let mut stat_notes = self.apply_shrine_stat_reward(entry.virtue);
                if entry.virtue == ShrineVirtue::Humility {
                    let humility_gain = self.add_shrine_standing(
                        entry.virtue,
                        ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE,
                    );
                    if humility_gain > 0 {
                        stat_notes.push(format!("standing +{humility_gain}"));
                    }
                    let humility_moral =
                        self.add_moral_standing(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE);
                    if humility_moral > 0 {
                        stat_notes.push(format!("moral +{humility_moral}"));
                    }
                }
                let stat_note = if stat_notes.is_empty() {
                    "no stat reward".to_string()
                } else {
                    stat_notes.join(", ")
                };
                self.message = format!(
                    "Completed the Shrine of {}; standing +{} to {}; moral +{} to {}; {}.",
                    entry.virtue.name(),
                    gained,
                    self.shrine_standing[entry.virtue.index()],
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
                let gained = self.add_shrine_standing(entry.virtue, offering);
                // karma.md §3-4: completed-shrine gold offering adds the
                // offered digit to the shared moral-standing selector.
                let moral_gained = self.add_moral_standing(offering);
                self.message = format!(
                    "Offered {cost} gold at the Shrine of {}; standing +{} to {}; moral +{} to {}.",
                    entry.virtue.name(),
                    gained,
                    self.shrine_standing[entry.virtue.index()],
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

    pub fn add_shrine_standing(&mut self, virtue: ShrineVirtue, amount: u8) -> u8 {
        let standing = &mut self.shrine_standing[virtue.index()];
        let before = *standing;
        *standing = (*standing).saturating_add(amount).min(SHRINE_STANDING_MAX);
        (*standing).saturating_sub(before)
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
