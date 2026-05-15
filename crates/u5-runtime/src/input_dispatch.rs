//! The shell-agnostic input dispatcher: takes a key + suffix, mutates PlayState, returns whether to keep going. Used by both u5-tui (terminal) and u5-bevy (window).

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayInputDisposition {
    Continue,
    Quit,
}

pub fn handle_play_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if state.endgame.is_some() {
        return Ok(handle_endgame_key_input(state, key, suffix));
    }
    if state.resolve_moongate_prompt(key, game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if state.resolve_natural_moongate_entry(game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_IGNORED_INPUT_KEY {
        state.message = match suffix {
            "function" => "Function key ignored.",
            _ => "Input ignored.",
        }
        .to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_TYPEAHEAD_TOGGLE_KEY {
        state.toggle_typeahead_buffer();
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active
        && combat_has_dispatchable_party_actor(state)
        && (state.pending_combat_actor_slot.is_some() || combat_has_active_non_party_actor(state))
        && key == 'C'
        && !suffix.is_empty()
    {
        return handle_combat_cast_key_input(state, suffix, game_dir);
    }
    if key == 'C' && !suffix.is_empty() {
        let turn_before = state.turn;
        let outcome = state.cast_spell_from_suffix(suffix, game_dir)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'M' && !suffix.is_empty() {
        if inline_mix_candidate(suffix) {
            state.mix_reagents_from_suffix(suffix);
        } else if state
            .meditate_shrine_from_suffix(suffix, game_dir)?
            .is_none()
        {
            state.mix_reagents_from_suffix(suffix);
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'N' && !suffix.is_empty() {
        state.new_order_from_suffix(suffix);
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'R' && !suffix.is_empty() {
        let turn_before = state.turn;
        let outcome = state.ready_equipment_from_suffix(suffix);
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'J' | 'j') {
        let turn_before = state.turn;
        let member_index = if suffix.is_empty() {
            None
        } else {
            parse_inline_party_index(suffix)
        };
        let outcome = state.jimmy_facing_with_game_dir_and_member(Some(game_dir), member_index)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'Y' | 'y') && !suffix.trim().is_empty() {
        let turn_before = state.turn;
        let outcome = state.yell_command(non_empty_yell_word(suffix));
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active {
        return Ok(handle_combat_key_input(state, key, suffix));
    }
    if key == 'q' {
        return Ok(PlayInputDisposition::Quit);
    }
    if matches!(state.area, Area::Dungeon { .. }) && key == 'Q' {
        return Ok(state.exit_to_dos_prompt(parse_inline_yes_no(suffix)));
    }
    let inline_direction = suffix.chars().find_map(Direction::from_play_key);
    let inline_hours = parse_inline_hours(suffix);
    let inline_drink = parse_inline_yes_no(suffix);
    let inline_party_index = parse_inline_party_index(suffix);
    let inline_use_request = parse_inline_use_request(suffix);
    let inline_talk_keyword = non_empty_talk_keyword(suffix);
    if state.handle_dungeon_key_with_inline(
        key,
        game_dir,
        inline_hours,
        inline_drink,
        inline_party_index,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'T' | 't') && inline_talk_keyword.is_some() {
        let turn_before = state.turn;
        let outcome = state.talk_facing_with_game_dir_and_keyword(game_dir, inline_talk_keyword)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.handle_top_down_key_with_inline(
        key,
        game_dir,
        inline_direction,
        inline_hours,
        inline_drink,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(state.area, Area::Dungeon { .. }) {
        state.advance_visual_tick();
        state.message = "Zzzzzz...".to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    state.message = format!("Unhandled command `{key}`.");
    Ok(PlayInputDisposition::Continue)
}

fn handle_endgame_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let answer = parse_inline_yes_no(suffix).or_else(|| match key {
        'Y' | 'y' => Some(true),
        'N' | 'n' => Some(false),
        _ => None,
    });
    if let Some(answer) = answer {
        state.resolve_endgame_confirmation(answer);
    } else if state
        .endgame
        .as_ref()
        .is_some_and(EndgameState::is_terminal)
    {
        state.resolve_endgame_confirmation(false);
    } else {
        state.message = "Endgame confirmation requires Y or N.".to_string();
    }
    PlayInputDisposition::Continue
}

fn combat_has_dispatchable_party_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .take(COMBAT_PARTY_ACTOR_SLOTS)
        .copied()
        .any(combat_actor_is_active_not_dead)
}

fn combat_has_active_non_party_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .skip(COMBAT_PARTY_ACTOR_SLOTS)
        .copied()
        .any(combat_actor_is_active_not_dead)
}

fn handle_combat_cast_key_input(
    state: &mut PlayState,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message = "No active combatant.".to_string();
        return Ok(PlayInputDisposition::Continue);
    };

    let quickness_roll = state.combat_quickness_dispatch_roll(actor_slot);
    if resolve_quickness_dispatch_consumed(
        state.active_effect_tag,
        state.active_effect_counter,
        quickness_roll,
    ) {
        let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        state.message =
            combat_magic_ring_pass_message(ring_pass).unwrap_or_else(|| "Quickness!".to_string());
        state.ensure_pending_combat_player_turn();
        return Ok(PlayInputDisposition::Continue);
    }

    let had_foe = combat_has_active_non_party_actor(state);
    let turn_before = state.turn;
    let cast_suffix = combat_cast_suffix_for_actor(suffix, actor_slot);
    let outcome = state.cast_spell_from_suffix(&cast_suffix, game_dir)?;
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    let cast_message = state.message.clone();
    let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
    state.message = combat_magic_ring_pass_message(ring_pass).unwrap_or(cast_message);

    if state.combat_active && had_foe && !combat_has_active_non_party_actor(state) {
        state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
    } else if state.combat_active {
        state.ensure_pending_combat_player_turn();
    }

    Ok(PlayInputDisposition::Continue)
}

fn handle_combat_key_input(state: &mut PlayState, key: char, suffix: &str) -> PlayInputDisposition {
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message = "No active combatant.".to_string();
        return PlayInputDisposition::Continue;
    };
    let input = combat_player_command_input_from_key_suffix(key, suffix);
    let quickness_roll = state.combat_quickness_dispatch_roll(actor_slot);
    let Some(application) =
        state.apply_combat_player_command_with_inputs(actor_slot, input, quickness_roll)
    else {
        state.message = "No active combatant.".to_string();
        return PlayInputDisposition::Continue;
    };
    state.message = combat_magic_ring_pass_message(application.ring_pass)
        .unwrap_or_else(|| combat_player_command_message(&application.action));
    if let CombatRoundLoopControl::Exit(exit) = application.control_after {
        state.apply_combat_round_loop_exit(exit);
    } else if matches!(
        application.action,
        CombatPlayerCommandAction::PromptForAttackDirection
    ) {
        state.pending_combat_actor_slot = Some(actor_slot);
    } else if application.control_after.result_code().is_none() {
        state.ensure_pending_combat_player_turn();
    }
    PlayInputDisposition::Continue
}

fn combat_magic_ring_pass_message(pass: Option<CombatMagicRingPassOutcome>) -> Option<String> {
    pass.and_then(|ring_pass| ring_pass.vanished_ring)
        .map(|ring| format!("{} vanished.", equipment_name(ring as usize)))
}

fn combat_cast_suffix_for_actor(suffix: &str, actor_slot: usize) -> String {
    let caster_digit = char::from_digit((actor_slot + 1) as u32, 10).unwrap_or('1');
    let Some((index, first_non_space)) = suffix
        .char_indices()
        .find(|(_, ch)| !ch.is_ascii_whitespace())
    else {
        return caster_digit.to_string();
    };

    if first_non_space.is_ascii_digit() {
        let next_index = index + first_non_space.len_utf8();
        let mut rewritten = String::with_capacity(suffix.len());
        rewritten.push_str(&suffix[..index]);
        rewritten.push(caster_digit);
        rewritten.push_str(&suffix[next_index..]);
        rewritten
    } else {
        let mut rewritten = String::with_capacity(suffix.len() + 1);
        rewritten.push(caster_digit);
        rewritten.push_str(suffix);
        rewritten
    }
}

fn combat_player_command_input_from_key_suffix(
    key: char,
    suffix: &str,
) -> CombatPlayerCommandInput {
    if key.eq_ignore_ascii_case(&'A') {
        if let Some(direction_code) = suffix
            .chars()
            .find_map(Direction::from_play_key)
            .and_then(combat_direction_code_for_direction)
        {
            return CombatPlayerCommandInput::AttackDirection(direction_code);
        }
        return CombatPlayerCommandInput::Key('A');
    }
    if let Some(direction_code) =
        Direction::from_play_key(key).and_then(combat_direction_code_for_direction)
    {
        return CombatPlayerCommandInput::Direction(direction_code);
    }
    CombatPlayerCommandInput::Key(key.to_ascii_uppercase())
}

fn combat_player_command_message(action: &CombatPlayerCommandAction) -> String {
    match action {
        CombatPlayerCommandAction::QuicknessSkipped => "Quickness!".to_string(),
        CombatPlayerCommandAction::ActivePlayerSelection(_) => {
            "Active player selected.".to_string()
        }
        CombatPlayerCommandAction::Pass(_) => "Pass.".to_string(),
        CombatPlayerCommandAction::PromptForAttackDirection => "Attack-".to_string(),
        CombatPlayerCommandAction::StepOrAttack { outcome, .. } => match outcome {
            CombatStepOrAttackPrimitiveOutcome::InactiveActor => "No active combatant.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::OutOfArena { .. } => "Leaving combat.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::Moved { .. } => "Moved.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::Attack { .. } => "Attack.".to_string(),
            CombatStepOrAttackPrimitiveOutcome::BlockedActor { .. }
            | CombatStepOrAttackPrimitiveOutcome::BlockedWall => "Blocked.".to_string(),
        },
        CombatPlayerCommandAction::InvalidDirection { .. } => "Direction?".to_string(),
        CombatPlayerCommandAction::QuitDefeat => "Combat abandoned.".to_string(),
        CombatPlayerCommandAction::XitCleanup { allowed: true } => "Exit combat.".to_string(),
        CombatPlayerCommandAction::XitCleanup { allowed: false } => "Foes remain.".to_string(),
        CombatPlayerCommandAction::Branch { branch, .. } => format!("{branch:?}."),
    }
}
