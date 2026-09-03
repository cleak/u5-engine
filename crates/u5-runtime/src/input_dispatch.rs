//! The shell-agnostic input dispatcher: takes a key + suffix, mutates PlayState, returns whether to keep going. Used by both u5-tui (terminal) and u5-bevy (window).

use std::io;
use std::path::Path;

use crate::shop_runtime::HealerService;
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayInputDisposition {
    Continue,
    Quit,
}

/// `commands.md §5` + `text-output.md §2`: dispatch one key and make sure
/// whatever it printed reaches the scrolling message transcript. The
/// world/town/dungeon dispatchers open their own verb echo; anything that
/// resolves before them (or outside the command surface entirely) is
/// recorded here as a plain continuation line.
pub fn handle_play_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let mut result = handle_play_key_input_inner(state, key, suffix, game_dir);
    // A generic adjacent terrain combat suspends the high-to-low outdoor
    // reaction walk. As soon as the combat frame returns, continue the
    // remaining lower slots before accepting another world command.
    // `town-mode.md §14`: drain the town NPC-conflict chain's exit -
    // "On exit the town chain clears the NPC slot, reloads the town map,
    // and re-runs the Shadowlord install pass of Section 13".
    if result.is_ok() && !state.combat_active && state.pending_town_conflict.is_some() {
        if let Err(error) = state.drain_pending_town_conflict(game_dir) {
            result = Err(error);
        }
    }
    if result.is_ok() && !state.combat_active && !state.pending_outdoor_reaction_slots.is_empty() {
        if let Area::World { plane } = state.area {
            if let Err(error) = state.apply_pending_outdoor_reactions(game_dir, plane) {
                result = Err(error);
            }
        } else {
            state.pending_outdoor_reaction_slots.clear();
        }
    }
    state.commit_command_echo();
    // `text-output.md §11`: whatever is still only in the slot is a line
    // the original would already have printed, so record it before the
    // next key can overwrite it.
    state.flush_message_slot();
    result
}

fn handle_play_key_input_inner(
    state: &mut PlayState,
    mut key: char,
    mut suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if let Some(byte) = input_byte_from_char(key) {
        if matches!(input_byte_class(byte), InputByteClass::FunctionKey) {
            state.message = "Function key ignored.".to_string();
            return Ok(PlayInputDisposition::Continue);
        }
    }
    if state.endgame.is_some() {
        return handle_endgame_key_input(state, key, suffix, game_dir);
    }
    if state.active_blackthorn.is_some() {
        return handle_active_blackthorn_key_input(state, key, suffix, game_dir);
    }
    if state.active_yes_no_prompt.is_some() {
        return handle_active_yes_no_prompt_key_input(state, key, suffix, game_dir);
    }
    if state.active_direction_prompt.is_some() {
        return handle_active_direction_prompt_key_input(state, key, suffix, game_dir);
    }
    if state.active_cast_followup.is_some() {
        return handle_active_cast_followup_key_input(state, key, suffix, game_dir);
    }
    if state.active_cast.is_some() {
        return handle_active_cast_key_input(state, key, suffix, game_dir);
    }
    if state.active_rest.is_some() {
        return handle_active_rest_key_input(state, key, suffix, game_dir);
    }
    if state.active_jimmy.is_some() {
        return handle_active_jimmy_key_input(state, key, suffix, game_dir);
    }
    if state.active_surface_chest.is_some() {
        return handle_active_surface_chest_key_input(state, key, suffix, game_dir);
    }
    if state.active_shrine.is_some() {
        return handle_active_shrine_key_input(state, key, suffix, game_dir);
    }
    if state.active_shrine_restoration.is_some() {
        return handle_active_shrine_restoration_key_input(state, key, suffix, game_dir);
    }
    if state.active_mix.is_some() {
        return Ok(handle_active_mix_key_input(state, key, suffix));
    }
    if state.active_new_order.is_some() {
        return Ok(handle_active_new_order_key_input(state, key, suffix));
    }
    if state.active_wishing_well.is_some() {
        return handle_active_wishing_well_key_input(state, key, suffix, game_dir);
    }
    if state.active_yell.is_some() {
        return handle_active_yell_key_input(state, key, suffix, game_dir);
    }
    if state.active_view_overlay.is_some() {
        state.clear_active_view_overlay();
        return Ok(PlayInputDisposition::Continue);
    }
    if state.active_ready.is_some() {
        return Ok(handle_active_ready_key_input(state, key, suffix));
    }
    if state.active_use.is_some() {
        return handle_active_use_key_input(state, key, suffix, game_dir);
    }
    if state.active_party_selector.is_some() {
        state.step_active_party_selector(key, suffix);
        return Ok(PlayInputDisposition::Continue);
    }
    if state.active_z_stats.is_some() {
        return Ok(handle_active_z_stats_key_input(state, key, suffix));
    }
    if state.active_shop.is_some() {
        return Ok(handle_active_shop_key_input(state, key, suffix, game_dir));
    }
    if state.active_conversation.is_some() {
        return Ok(handle_active_conversation_key_input(state, key, suffix));
    }
    if state
        .resolve_blackthorn_guard_demand_input(key, suffix)
        .is_some()
    {
        return Ok(PlayInputDisposition::Continue);
    }
    if state.resolve_town_arrest_prompt(key, game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if state.resolve_natural_moongate_entry(game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    // `systems/shops.md` tavern drunkenness: every top-level town command
    // performs a fresh even-odds gate. Active prompts/sessions above consume
    // their own keys before this point and therefore are not commands.
    if let Area::Town { scene, floor } = state.area {
        if state.town_drunkenness_counter != 0 && state.random_range_u8(0, 1) == 1 {
            state.town_alarm_sweep(scene, floor, None);
            state.town_drunkenness_counter -= 1;
            state.emit_message_line("Hic!\n");
            let replacement = state.random_range_u8(0, 3);
            key = char::from(INPUT_CODE_CARDINAL_FIRST + replacement);
            suffix = "";
        }
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
    if key == PLAY_MUSIC_TOGGLE_KEY {
        state.toggle_music();
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active
        && combat_has_dispatchable_player_actor(state)
        && (state.pending_combat_actor_slot.is_some() || combat_has_active_non_party_actor(state))
        && key == 'C'
        && !suffix.is_empty()
    {
        return handle_combat_cast_key_input(state, suffix, game_dir);
    }
    if key == 'C' && !suffix.is_empty() {
        state.begin_command_echo_for(Command::Cast);
        let turn_before = state.turn;
        let outcome = state.cast_spell_from_suffix(suffix, game_dir)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active {
        return Ok(handle_combat_key_input(state, key, suffix));
    }
    if key == 'Z' {
        state.begin_command_echo_for(Command::ZStats);
        state.z_stats_command();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'M' && !suffix.is_empty() {
        state.begin_command_echo_for(Command::Mix);
        if state
            .read_codex_urn_at_current_position(game_dir)?
            .is_none()
            && state
                .meditate_shrine_from_suffix(suffix, game_dir)?
                .is_none()
        {
            state.mix_reagents_from_suffix(suffix);
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'N' && !suffix.is_empty() {
        state.begin_command_echo_for(Command::NewOrder);
        state.new_order_from_suffix(suffix);
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'R' && !suffix.is_empty() {
        state.begin_command_echo_for(Command::Ready);
        let turn_before = state.turn;
        let outcome = state.ready_equipment_from_suffix(suffix);
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    // Inline harnesses may still supply Jimmy's party member as a suffix.
    // Interactive input falls through to the shared adjacent-direction
    // prompt first, matching the published command sequence.
    if matches!(key, 'J' | 'j')
        && (!suffix.is_empty() || matches!(state.area, Area::Dungeon { .. }))
    {
        state.begin_command_echo_for(Command::Jimmy);
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
        state.begin_command_echo_for(Command::Yell);
        let turn_before = state.turn;
        let outcome = state.yell_command(non_empty_yell_word(suffix));
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'q' {
        return Ok(PlayInputDisposition::Quit);
    }
    // `main-loop.md §4`: the event-driven engine fuses the outer mode
    // loop with one inner-loop input iteration. Every non-modal input is
    // routed from the resident scene byte here; a transition replaces the
    // active `PlayState`, and the next input dispatches through the new
    // scene class. The historical exit-pending flag collapses into this
    // call boundary, as §14 permits, and the single-directory runtime's
    // between-mode disk-prompt presentation pass is a no-op.
    let active_route = scene_route(state.current_scene_byte());
    if active_route == SceneRoute::Dungeon && key == 'Q' {
        state.begin_command_echo_for(Command::Quit);
        if let Some(confirm) = parse_inline_yes_no(suffix) {
            return Ok(state.exit_to_dos_prompt(Some(confirm)));
        }
        state.start_exit_to_dos_prompt();
        return Ok(PlayInputDisposition::Continue);
    }
    let inline_direction = suffix.chars().find_map(Direction::from_play_key);
    let inline_rest = parse_inline_rest_request(suffix);
    let inline_drink = parse_inline_yes_no(suffix);
    let inline_party_index = parse_inline_party_index(suffix);
    let inline_use_request = parse_inline_use_request(suffix);
    let inline_look_focus = suffix.chars().find_map(dungeon_look_focus_from_key);
    let inline_talk_keyword = non_empty_talk_keyword(suffix);
    if active_route == SceneRoute::Dungeon
        && state.handle_dungeon_key_with_inline(
            key,
            game_dir,
            inline_rest,
            inline_drink,
            inline_party_index,
            inline_use_request,
            inline_look_focus,
        )?
    {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'T' | 't') && inline_talk_keyword.is_some() {
        let turn_before = state.turn;
        let outcome = state.talk_facing_with_game_dir_and_keyword(game_dir, inline_talk_keyword)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    // `town-mode.md §13` + `commands.md §3`: in a town-family scene the digit
    // keys `0`..`9` reach one handler with two behaviours. Seated at the
    // harpsichord it consumes the key and reports the town-only status `3` —
    // re-prompt immediately, with no turn, no clock advance, no NPC schedule
    // tick, and no redraw. Anywhere else, and on any other floor, it forwards
    // the digit to the ordinary dispatcher below and returns its result.
    if active_route == SceneRoute::TownFamily
        && let Some(digit) = key.to_digit(10).and_then(|digit| u8::try_from(digit).ok())
        && state.play_harpsichord_digit(digit)
    {
        return Ok(PlayInputDisposition::Continue);
    }
    match active_route {
        SceneRoute::Overworld | SceneRoute::TownFamily => {
            if state.handle_top_down_key_with_inline(
                key,
                game_dir,
                inline_direction,
                inline_rest,
                inline_drink,
                inline_use_request,
            )? {
                return Ok(PlayInputDisposition::Continue);
            }
        }
        SceneRoute::Dungeon => {
            state.advance_visual_tick();
            state.message = "Zzzzzz...".to_string();
            return Ok(PlayInputDisposition::Continue);
        }
        SceneRoute::IntroOrPreview | SceneRoute::CombatTemporary => {
            // Intro values are consumed before gameplay and combat returns
            // through its framer, so neither can reach this non-modal path.
            debug_assert!(false, "non-playable scene reached world input dispatch");
        }
    }
    state.emit_command_echo_line(UNRECOGNISED_COMMAND_MESSAGE);
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_z_stats_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let combat_actor = state
        .combat_active
        .then_some(state.pending_combat_actor_slot)
        .flatten();
    state.step_active_z_stats(key, suffix);
    if state.active_z_stats.is_none() {
        finish_combat_modal_actor_action(state, combat_actor);
    }
    PlayInputDisposition::Continue
}

fn handle_active_ready_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let combat_actor = state
        .combat_active
        .then_some(state.pending_combat_actor_slot)
        .flatten();
    state.step_active_ready(key, suffix);
    if state.active_ready.is_none() {
        finish_combat_modal_actor_action(state, combat_actor);
    }
    PlayInputDisposition::Continue
}

fn handle_active_cast_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let pending_combat = state.active_cast.as_ref().and_then(|session| {
        session
            .combat_actor_slot
            .map(|slot| (slot, session.combat_had_foe))
    });
    let turn_before = state.turn;
    let Some((outcome, combat)) = state.step_active_cast(key, suffix, game_dir)? else {
        // `combat.md §8`: accepting C commits the actor's action unless the
        // caster itself is rejected as dead. Escape or a blank spell-name
        // response closes the modal without a spell outcome, so detect that
        // closed session here and still run the combat action tail. Starting
        // a spell-specific follow-up is not a close and retains the actor.
        if state.active_cast.is_none()
            && state.active_cast_followup.is_none()
            && let Some((actor_slot, had_foe)) = pending_combat
        {
            finish_combat_cast_actor_action(state, actor_slot, had_foe);
        }
        return Ok(PlayInputDisposition::Continue);
    };
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    if let Some((actor_slot, had_foe)) = combat {
        finish_combat_cast_actor_action(state, actor_slot, had_foe);
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_cast_followup_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let pending_combat = state.active_cast_followup.as_ref().and_then(|session| {
        session
            .combat_actor_slot
            .map(|slot| (slot, session.combat_had_foe))
    });
    let turn_before = state.turn;
    let Some((outcome, combat)) = state.step_active_cast_followup(key, suffix, game_dir)? else {
        // A canceled combat target/cursor prompt is still the completion of
        // the already accepted C action. Some field spells have also spent
        // charge and mana before this prompt, as required by `magic.md §8`.
        if state.active_cast_followup.is_none()
            && let Some((actor_slot, had_foe)) = pending_combat
        {
            finish_combat_cast_actor_action(state, actor_slot, had_foe);
        }
        return Ok(PlayInputDisposition::Continue);
    };
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    if let Some((actor_slot, had_foe)) = combat {
        finish_combat_cast_actor_action(state, actor_slot, had_foe);
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_mix_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let _ = state.step_active_mix(key, suffix);
    PlayInputDisposition::Continue
}

fn handle_active_rest_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_rest(key, suffix, game_dir)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_jimmy_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_jimmy(key, suffix, game_dir)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_surface_chest_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_surface_chest(key, suffix)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_shrine_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_shrine(key, suffix, game_dir)? {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_shrine_restoration_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_shrine_restoration(key, suffix) {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_new_order_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let _ = state.step_active_new_order(key, suffix);
    PlayInputDisposition::Continue
}

fn handle_active_yell_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if state.combat_active {
        let Some(actor_slot) = state.pending_combat_actor_slot else {
            state.active_yell = None;
            return Ok(PlayInputDisposition::Continue);
        };
        let Some(mut session) = state.active_yell.take() else {
            return Ok(PlayInputDisposition::Continue);
        };
        let mut line = String::new();
        if !matches!(key, '\r' | '\n' | '\u{1b}') {
            line.push(key);
        }
        line.push_str(suffix);
        session.buffer.push_str(&line);
        state.message = if key == '\u{1b}' || session.buffer.trim().is_empty() {
            YELL_NOTHING_SAID_MESSAGE.to_string()
        } else {
            let word = PlayState::normalize_yell_word(&session.buffer);
            format!("Yelled {word}. Nothing happens.")
        };
        state.pending_combat_actor_slot = None;
        let _ = apply_combat_committed_action_maintenance(state, actor_slot);
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
        return Ok(PlayInputDisposition::Continue);
    }

    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_yell(key, suffix) {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_wishing_well_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_wishing_well(key, suffix) {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_direction_prompt_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    let combat_push_actor =
        state
            .active_direction_prompt
            .as_ref()
            .and_then(|session| match session.kind {
                DirectionPromptKind::CombatKlimb { actor_slot } => Some(actor_slot),
                DirectionPromptKind::CombatPush { actor_slot } => Some(actor_slot),
                DirectionPromptKind::CombatSjog { actor_slot, .. } => Some(actor_slot),
                _ => None,
            });
    let combat_klimb = state
        .active_direction_prompt
        .as_ref()
        .is_some_and(|session| matches!(session.kind, DirectionPromptKind::CombatKlimb { .. }));
    if let Some(outcome) = state.step_active_direction_prompt(key, suffix, game_dir)? {
        if let Some(actor_slot) = combat_push_actor {
            if combat_klimb && matches!(outcome, MoveOutcome::Blocked) {
                // `combat.md §8`: blocked Klimb is one of the parser's
                // exhaustive free re-prompt cases.
                state.pending_combat_actor_slot = Some(actor_slot);
            } else if state.combat_active {
                // Push and the live-actor-gated SJOG verbs commit once their
                // letter has been accepted, including a canceled shared
                // direction prompt. Klimb cancellation is likewise distinct
                // from its explicitly free blocked-result branch.
                let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                advance_combat_round_after_actor_and_append_message(state, actor_slot);
            }
        } else {
            state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        }
    }
    Ok(PlayInputDisposition::Continue)
}

fn handle_active_yes_no_prompt_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    Ok(state
        .step_active_yes_no_prompt(key, suffix, game_dir)?
        .unwrap_or(PlayInputDisposition::Continue))
}

fn handle_active_use_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let combat_actor = state
        .combat_active
        .then_some(state.pending_combat_actor_slot)
        .flatten();
    state.step_active_use(key, suffix, game_dir)?;
    if state.active_use.is_none() {
        finish_combat_modal_actor_action(state, combat_actor);
    }
    Ok(PlayInputDisposition::Continue)
}

/// `combat.md §8`: Ready, Z-stats, and Use return through their shared modal
/// handlers before the combat parser ends the acting combatant's action. Keep
/// the actor bound while the modal is open, then resume the round walker once.
fn finish_combat_modal_actor_action(state: &mut PlayState, actor_slot: Option<usize>) {
    let Some(actor_slot) = actor_slot else {
        return;
    };
    if state.pending_combat_actor_slot != Some(actor_slot) {
        return;
    }
    state.pending_combat_actor_slot = None;
    if state.combat_active {
        let _ = apply_combat_committed_action_maintenance(state, actor_slot);
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }
}

fn handle_active_shop_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> PlayInputDisposition {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;

    let Some(mut session) = state.active_shop.take() else {
        return PlayInputDisposition::Continue;
    };
    let ctx = ShopTransactionContext {
        party_gold: state.gold,
        speaker_intelligence: active_speaker_intelligence(state),
        world_hour: state.clock.hour,
        party_size: state.party.len(),
        living_party_members: state
            .party
            .iter()
            .filter(|member| member.living())
            .count()
            .min(u8::MAX as usize) as u8,
    };
    let key_byte = key as u8;
    let inline_digit = suffix
        .chars()
        .find(|c| c.is_ascii_digit())
        .and_then(|c| c.to_digit(10).map(|d| d as u8))
        .or_else(|| key.to_digit(10).map(|d| d as u8));
    let inline_quantity = parse_active_shop_inline_quantity(key, suffix);
    let yes = matches!(key_byte, b'Y' | b'y') || suffix.chars().any(|c| matches!(c, 'Y' | 'y'));
    let no = matches!(key_byte, b'N' | b'n') || suffix.chars().any(|c| matches!(c, 'N' | 'n'));
    let mut replacement_session: Option<ActiveShopSession> = None;

    let message = match &mut session {
        ActiveShopSession::Arms(s) => handle_arms_shop_key_input(
            state,
            s,
            None,
            ctx,
            key_byte,
            inline_digit,
            yes,
            no,
            game_dir,
        ),
        ActiveShopSession::ArmsLocal(s, shop) => {
            let table = shop.stock_table();
            handle_arms_shop_key_input(
                state,
                s,
                Some(table),
                ctx,
                key_byte,
                inline_digit,
                yes,
                no,
                game_dir,
            )
        }
        ActiveShopSession::ArmsStocked(s, stock_table) => handle_arms_shop_key_input(
            state,
            s,
            Some(*stock_table),
            ctx,
            key_byte,
            inline_digit,
            yes,
            no,
            game_dir,
        ),
        ActiveShopSession::Healer(s, healer) => match (*s, yes, no, inline_digit) {
            (HealerShopState::Greeting, _, true, _) => {
                *s = HealerShopState::Exited;
                "Farewell.".to_string()
            }
            (HealerShopState::Greeting, true, _, _) => {
                *s = HealerShopState::PickService;
                "Cure (C), Heal (H), or Resurrect (R)?".to_string()
            }
            (HealerShopState::Greeting, _, _, _)
                if matches!(key_byte, b'H' | b'h' | b'Y' | b'y') =>
            {
                *s = HealerShopState::PickService;
                "Cure (C), Heal (H), or Resurrect (R)?".to_string()
            }
            (HealerShopState::Greeting, _, _, _) => {
                *s = HealerShopState::Exited;
                "Farewell.".to_string()
            }
            (HealerShopState::PickService, _, _, _) => match healer_service_action(key_byte) {
                HealerServiceAction::Treatment(treatment) => {
                    let service = healer_service_for_treatment(treatment);
                    let cost = match healer_treatment_fee(*healer, treatment) {
                        HealerTreatmentFee::Bypass => 0,
                        HealerTreatmentFee::Price(cost) => cost,
                    };
                    *s = HealerShopState::PickPartyMember { service, cost };
                    format!("Who needs {}? (1-6)", treatment.display_name())
                }
                HealerServiceAction::Exit => {
                    *s = HealerShopState::Exited;
                    "Farewell.".to_string()
                }
                HealerServiceAction::Discard => "Cure (C), Heal (H), or Resurrect (R)?".to_string(),
            },
            (HealerShopState::PickPartyMember { service, .. }, _, true, _) => {
                *s = HealerShopState::PickService;
                let treatment = healer_treatment_for_service(service);
                format!("Cancelled {}.", treatment.display_name())
            }
            (HealerShopState::PickPartyMember { service, .. }, _, _, Some(d)) if d >= 1 => {
                let target_index = usize::from(d - 1);
                let treatment = healer_treatment_for_service(service);
                let Some(member) = state.party.get(target_index).copied() else {
                    return {
                        state.active_shop = Some(session);
                        state.message = "I do not understand.".to_string();
                        PlayInputDisposition::Continue
                    };
                };
                if !active_healer_target_accepts(treatment, member) {
                    *s = HealerShopState::PickService;
                    "That treatment is not needed.".to_string()
                } else {
                    match healer_treatment_fee(*healer, treatment) {
                        HealerTreatmentFee::Bypass => {
                            let message = match state.buy_healer_treatment(
                                *healer,
                                treatment,
                                target_index,
                            ) {
                                Ok(outcome) => format_healer_treatment_outcome(outcome),
                                Err(err) => format_healer_treatment_error(err),
                            };
                            *s = HealerShopState::PickService;
                            message
                        }
                        HealerTreatmentFee::Price(cost) => {
                            *s = HealerShopState::Confirm {
                                service,
                                slot: d - 1,
                                cost,
                            };
                            format!("{} costs {cost} gold. (Y/N)", treatment.display_name())
                        }
                    }
                }
            }
            (
                HealerShopState::Confirm {
                    service,
                    slot,
                    cost,
                    ..
                },
                true,
                _,
                _,
            ) => {
                let treatment = healer_treatment_for_service(service);
                let message =
                    match state.buy_healer_treatment(*healer, treatment, usize::from(slot)) {
                        Ok(outcome) => {
                            let surcharge =
                                if matches!(outcome.quote.fee, HealerTreatmentFee::Price(_)) {
                                    apply_active_shop_surcharge(state)
                                } else {
                                    None
                                };
                            append_active_shop_surcharge(
                                format_healer_treatment_outcome(outcome),
                                surcharge,
                            )
                        }
                        Err(crate::shops::HealerTreatmentError::InsufficientGold { .. }) => {
                            format!("Thou lackest the {cost} gold.")
                        }
                        Err(err) => format_healer_treatment_error(err),
                    };
                *s = HealerShopState::PickService;
                message
            }
            (HealerShopState::Confirm { service, .. }, _, true, _) => {
                *s = HealerShopState::PickService;
                let treatment = healer_treatment_for_service(service);
                format!("Declined {}.", treatment.display_name())
            }
            (HealerShopState::Exited, _, _, _) => "Farewell.".to_string(),
            _ => "I do not understand.".to_string(),
        },
        ActiveShopSession::Innkeeper(s) => {
            let scene_marker = active_inn_scene_marker(state);
            match (*s, yes, no, inline_digit) {
                (InnkeeperState::Greeting { inn }, _, _, _) => match inn_main_action(key_byte) {
                    InnMainAction::Rest => {
                        let base_room_rate = inn_base_room_rate(inn);
                        let total_price = quote_inn_rest_for_speaker(
                            inn,
                            state.party.len(),
                            ctx.speaker_intelligence,
                        )
                        .map(|quote| quote.total_price)
                        .unwrap_or(0);
                        *s = InnkeeperState::ConfirmRest {
                            inn,
                            base_room_rate,
                            total_price,
                        };
                        format!(
                            "{} room and board costs {total_price} gold. (Y/N)",
                            inn.display_name()
                        )
                    }
                    InnMainAction::LeaveCompanion => {
                        let deposit =
                            inn_leave_companion_deposit_for_speaker(inn, ctx.speaker_intelligence);
                        *s = InnkeeperState::PickLeaveCompanion { inn, deposit };
                        format!("Leave which companion? Deposit is {deposit} gold. (1-6)")
                    }
                    InnMainAction::PickUpCompanion => {
                        let base_room_rate = inn_base_room_rate(inn);
                        let guests = inn_guest_indices_for_scene(&state.inn_registry, scene_marker);
                        if guests.is_empty() {
                            *s = InnkeeperState::Greeting { inn };
                            "No one here is from thy party!".to_string()
                        } else if guests.len() == 1 {
                            let registry_index = guests[0];
                            let bill = state
                                .inn_registry
                                .get(registry_index)
                                .map(|guest| {
                                    inn_pickup_bill_for_speaker(
                                        inn,
                                        guest.stay_counter,
                                        ctx.speaker_intelligence,
                                    )
                                })
                                .unwrap_or(0);
                            *s = InnkeeperState::ConfirmPickUpCompanion {
                                inn,
                                registry_index,
                                base_lodging_charge: base_room_rate,
                                bill,
                            };
                            format!("Pickup bill is {bill} gold. (Y/N)")
                        } else {
                            let mut guest_indices = [0usize; INN_REGISTRY_CAP];
                            for (slot, registry_index) in
                                guests.iter().copied().take(INN_REGISTRY_CAP).enumerate()
                            {
                                guest_indices[slot] = registry_index;
                            }
                            let guest_count = guests.len().min(INN_REGISTRY_CAP) as u8;
                            *s = InnkeeperState::PickUpCompanion {
                                inn,
                                guest_indices,
                                guest_count,
                                base_lodging_charge: base_room_rate,
                            };
                            format!(
                                "Guest register has {guest_count} companion(s). Pick 1-{guest_count}."
                            )
                        }
                    }
                    InnMainAction::Exit => {
                        *s = InnkeeperState::Exited;
                        "Farewell.".to_string()
                    }
                    InnMainAction::Discard => {
                        "Rest (R), Leave (L), Pick up (P), or Space.".to_string()
                    }
                },
                (
                    InnkeeperState::ConfirmRest {
                        inn, total_price, ..
                    },
                    true,
                    _,
                    _,
                ) => {
                    let result = state.pay_inn_rest_total(inn, total_price);
                    *s = InnkeeperState::Greeting { inn };
                    match result {
                        Ok(outcome) => {
                            let message = apply_paid_inn_rest(state, outcome.quote.total_price);
                            let surcharge = apply_active_shop_surcharge(state);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Err(err) => format_inn_error(err),
                    }
                }
                (InnkeeperState::ConfirmRest { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::PickLeaveCompanion { inn, deposit: _ }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::PickLeaveCompanion { inn, deposit }, _, _, Some(d)) if d >= 1 => {
                    let party_index = usize::from(d - 1);
                    *s = InnkeeperState::ConfirmLeaveCompanion {
                        inn,
                        party_index,
                        deposit,
                    };
                    format!("Leave party member {d} for {deposit} gold? (Y/N)")
                }
                (
                    InnkeeperState::ConfirmLeaveCompanion {
                        inn,
                        party_index,
                        deposit,
                    },
                    true,
                    _,
                    _,
                ) => {
                    let result = state.leave_inn_companion(scene_marker, party_index, deposit);
                    *s = InnkeeperState::Greeting { inn };
                    match result {
                        Ok(outcome) => {
                            let message = format!(
                                "Left companion {} at the inn for {} gold.",
                                outcome.party_index + 1,
                                outcome.deposit
                            );
                            let surcharge = apply_active_shop_surcharge(state);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Err(err) => format_inn_error(err),
                    }
                }
                (InnkeeperState::ConfirmLeaveCompanion { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (
                    InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        guest_count,
                        base_lodging_charge,
                    },
                    _,
                    true,
                    _,
                ) => {
                    let _ = (guest_indices, guest_count, base_lodging_charge);
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (
                    InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        guest_count,
                        base_lodging_charge,
                    },
                    _,
                    _,
                    Some(d),
                ) if d >= 1 && d <= guest_count => {
                    let registry_index = guest_indices[usize::from(d - 1)];
                    let bill = state
                        .inn_registry
                        .get(registry_index)
                        .map(|guest| {
                            let _ = base_lodging_charge;
                            inn_pickup_bill_for_speaker(
                                inn,
                                guest.stay_counter,
                                ctx.speaker_intelligence,
                            )
                        })
                        .unwrap_or(0);
                    *s = InnkeeperState::ConfirmPickUpCompanion {
                        inn,
                        registry_index,
                        base_lodging_charge,
                        bill,
                    };
                    format!("Pickup bill is {bill} gold. (Y/N)")
                }
                (
                    InnkeeperState::ConfirmPickUpCompanion {
                        inn,
                        registry_index,
                        base_lodging_charge: _,
                        bill,
                        ..
                    },
                    true,
                    _,
                    _,
                ) => {
                    let result =
                        state.pickup_inn_guest_with_bill(scene_marker, registry_index, bill);
                    *s = InnkeeperState::Greeting { inn };
                    match result {
                        Ok(outcome) if outcome.returned_dead_from_poison => {
                            let message = format!(
                                "Picked up companion {} for {} gold. Thy friend has died, by the way.",
                                outcome.party_index + 1,
                                outcome.bill
                            );
                            let surcharge = apply_active_shop_surcharge(state);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Ok(outcome) => {
                            let message = format!(
                                "Picked up companion {} for {} gold.",
                                outcome.party_index + 1,
                                outcome.bill
                            );
                            let surcharge = apply_active_shop_surcharge(state);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Err(err) => format_inn_error(err),
                    }
                }
                (InnkeeperState::ConfirmPickUpCompanion { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::Greeting { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::Exited, _, _, _) => "Farewell.".to_string(),
                _ => "I do not understand.".to_string(),
            }
        }
        ActiveShopSession::Tavern(s) => {
            let mut food = state.food;
            let outcome = match (*s, yes, no, inline_digit) {
                (TavernState::Greeting { .. }, _, _, _) => step_tavern(
                    s,
                    TavernInput::Key(key_byte),
                    ctx,
                    &mut state.gold,
                    &mut food,
                ),
                (
                    TavernState::Menu {
                        tavern,
                        continuation_ready,
                    }
                    | TavernState::PostListWait {
                        tavern,
                        continuation_ready,
                    },
                    _,
                    _,
                    _,
                ) if state.tavern_secondary_drink_count == 3
                    && key_byte.to_ascii_uppercase()
                        == tavern_menu_letters(tavern).secondary as u8 =>
                {
                    *s = TavernState::ConfirmEnoughDrink {
                        tavern,
                        continuation_ready,
                    };
                    TavernOutcome::ConfirmEnoughDrink
                }
                (
                    TavernState::ConfirmEnoughDrink {
                        tavern,
                        continuation_ready: _,
                    },
                    true,
                    _,
                    _,
                ) => {
                    *s = TavernState::AnythingElse {
                        tavern,
                        continuation_ready: true,
                    };
                    TavernOutcome::DeclinedEnoughDrink
                }
                (
                    TavernState::ConfirmEnoughDrink {
                        tavern,
                        continuation_ready,
                    },
                    _,
                    true,
                    _,
                ) => {
                    // The consequence is committed before the requested
                    // fourth purchase performs any affordability check.
                    state.town_drunkenness_counter = 25;
                    state.moral_standing = state.moral_standing.saturating_sub(1);
                    *s = TavernState::Menu {
                        tavern,
                        continuation_ready,
                    };
                    step_tavern(
                        s,
                        TavernInput::Key(tavern_menu_letters(tavern).secondary as u8),
                        ctx,
                        &mut state.gold,
                        &mut food,
                    )
                }
                (
                    TavernState::Menu { .. }
                    | TavernState::PostListWait { .. }
                    | TavernState::BlueBoarDrinkList { .. }
                    | TavernState::AnythingElse { .. },
                    _,
                    _,
                    _,
                ) => step_tavern(
                    s,
                    TavernInput::Key(key_byte),
                    ctx,
                    &mut state.gold,
                    &mut food,
                ),
                (TavernState::PickProvisionQuantity { .. }, _, true, _) => {
                    *s = TavernState::Exited;
                    TavernOutcome::Exited
                }
                (TavernState::PickProvisionQuantity { .. }, _, _, _) => {
                    if let Some(quantity) = inline_quantity {
                        step_tavern(
                            s,
                            TavernInput::Quantity(quantity),
                            ctx,
                            &mut state.gold,
                            &mut food,
                        )
                    } else {
                        TavernOutcome::InvalidInput
                    }
                }
                _ => TavernOutcome::InvalidInput,
            };
            state.food = food;
            if matches!(outcome, TavernOutcome::RoundDrinkServed { .. }) {
                state.rewrite_tavern_round_table_setting();
            }
            if matches!(
                outcome,
                TavernOutcome::SecondaryTavernSelected { .. }
                    | TavernOutcome::BlueBoarDrinkServed { .. }
            ) {
                state.tavern_secondary_drink_count =
                    state.tavern_secondary_drink_count.saturating_add(1);
            }
            let surcharge_applies = match outcome {
                TavernOutcome::RoundDrinkServed { .. }
                | TavernOutcome::SecondaryTavernSelected { .. }
                | TavernOutcome::BlueBoarDrinkServed { .. } => true,
                TavernOutcome::ProvisionsPurchased {
                    paid: 1..,
                    completion,
                    ..
                } => completion.surcharge_applies(),
                _ => false,
            };
            let surcharge = if surcharge_applies {
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            if matches!(outcome, TavernOutcome::EnteredSagePrompt) {
                replacement_session = Some(ActiveShopSession::Sage(SageState::default()));
            }
            let provision_quote_record_id =
                matches!(outcome, TavernOutcome::PickProvisionQuantity { .. }).then(|| {
                    usize::from(state.random_range_u8(
                        TAVERN_PROVISION_QUOTE_RECORD_FIRST as u8,
                        TAVERN_PROVISION_QUOTE_RECORD_LAST as u8,
                    ))
                });
            let no_sale_record_id = matches!(
                outcome,
                TavernOutcome::DeclinedContinuation | TavernOutcome::NoSaleExit
            )
            .then(|| {
                usize::from(state.random_range_u8(
                    TAVERN_NO_SALE_RECORD_FIRST as u8,
                    TAVERN_NO_SALE_RECORD_LAST as u8,
                ))
            });
            append_active_shop_surcharge(
                format_tavern_outcome_with_shoppe(
                    outcome,
                    provision_quote_record_id,
                    no_sale_record_id,
                    game_dir,
                ),
                surcharge,
            )
        }
        ActiveShopSession::Sage(s) => {
            let outcome = match *s {
                SageState::Prompt { .. } => {
                    let line = active_shop_text_line(key, suffix);
                    step_sage(s, SageInput::Keyword(&line), &mut state.gold)
                }
                SageState::Confirm { quote, .. } if yes && state.gold >= quote.entry.fee => {
                    let record_id = usize::from(state.random_range_u8(
                        SAGE_RUMOUR_SUCCESS_RECORD_FIRST as u8,
                        SAGE_RUMOUR_SUCCESS_RECORD_LAST as u8,
                    ));
                    step_sage(
                        s,
                        SageInput::Confirm {
                            accepted: true,
                            record_id,
                        },
                        &mut state.gold,
                    )
                }
                SageState::Confirm { .. } if yes => step_sage(
                    s,
                    SageInput::Confirm {
                        accepted: true,
                        record_id: SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
                    },
                    &mut state.gold,
                ),
                SageState::Confirm { .. } if no || key_byte == b' ' => step_sage(
                    s,
                    SageInput::Confirm {
                        accepted: false,
                        record_id: SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
                    },
                    &mut state.gold,
                ),
                SageState::Confirm { quote, .. } => SageOutcome::QuotedFee { quote },
                SageState::Exited => SageOutcome::Exited,
            };
            let paid = matches!(outcome, SageOutcome::RumourFound { .. });
            let message = format_sage_outcome_with_shoppe(outcome, game_dir);
            let surcharge = if paid {
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            append_active_shop_surcharge(message, surcharge)
        }
        ActiveShopSession::Reagent(s) => {
            let mut stock = state.reagents;
            let outcome = match (*s, inline_digit) {
                (ReagentShopState::Greeting { .. } | ReagentShopState::PickReagent { .. }, _) => {
                    step_reagent_shop(
                        s,
                        ReagentShopInput::Key(key_byte),
                        &mut state.gold,
                        &mut stock,
                    )
                }
                (ReagentShopState::PickQuantity { .. }, _) => {
                    if let Some(q) = inline_quantity.and_then(|q| u8::try_from(q).ok()) {
                        step_reagent_shop(
                            s,
                            ReagentShopInput::Quantity(q),
                            &mut state.gold,
                            &mut stock,
                        )
                    } else {
                        ReagentShopOutcome::InvalidInput
                    }
                }
                _ => ReagentShopOutcome::InvalidInput,
            };
            state.reagents = stock;
            format_reagent_outcome(outcome)
        }
        ActiveShopSession::HorseTrader(s) => {
            let outcome = match (*s, yes, no) {
                (HorseTraderState::Greeting { .. }, _, _) => step_horse_trader(
                    s,
                    HorseTraderInput::Key {
                        key: key_byte,
                        speaker_intelligence: ctx.speaker_intelligence,
                    },
                    &mut state.gold,
                ),
                (HorseTraderState::ConfirmPurchase { stable, price }, true, _) => {
                    if state.gold < price {
                        *s = HorseTraderState::Greeting { stable };
                        HorseTraderOutcome::RefusedShortFunds { price }
                    } else if let Some((x, y)) = horse_sale_position(state) {
                        match state.buy_horse_for_price(stable, price, x, y) {
                            Ok(_) => {
                                *s = HorseTraderState::Exited;
                                HorseTraderOutcome::Purchased { price }
                            }
                            Err(_) => {
                                *s = HorseTraderState::Greeting { stable };
                                HorseTraderOutcome::InvalidInput
                            }
                        }
                    } else {
                        *s = HorseTraderState::Greeting { stable };
                        HorseTraderOutcome::RefusedNoMarker { price }
                    }
                }
                (HorseTraderState::ConfirmPurchase { .. }, _, true) => step_horse_trader(
                    s,
                    HorseTraderInput::Confirm {
                        accepted: false,
                        can_place_horse: false,
                    },
                    &mut state.gold,
                ),
                _ => HorseTraderOutcome::InvalidInput,
            };
            let surcharge = if matches!(outcome, HorseTraderOutcome::Purchased { .. }) {
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            append_active_shop_surcharge(format_horse_trader_outcome(outcome), surcharge)
        }
        ActiveShopSession::ShipBroker(s) => {
            let outcome = if let Some(return_world) = state.return_world.as_mut() {
                match (*s, yes, no) {
                    (ShipBrokerState::Greeting { .. }, _, _) => step_ship_broker(
                        s,
                        ShipBrokerInput::Key(key_byte),
                        &mut state.gold,
                        &mut return_world.pending_vehicle,
                    ),
                    (ShipBrokerState::ConfirmPurchase { .. }, true, _) => step_ship_broker(
                        s,
                        ShipBrokerInput::Confirm(true),
                        &mut state.gold,
                        &mut return_world.pending_vehicle,
                    ),
                    (ShipBrokerState::ConfirmPurchase { .. }, _, true) => step_ship_broker(
                        s,
                        ShipBrokerInput::Confirm(false),
                        &mut state.gold,
                        &mut return_world.pending_vehicle,
                    ),
                    _ => ShipBrokerOutcome::InvalidInput,
                }
            } else {
                ShipBrokerOutcome::InvalidInput
            };
            if let ShipBrokerOutcome::PurchaseApplied { outcome: purchase } = outcome {
                state.sync_pending_vehicle_purchase_state(purchase);
            }
            let surcharge = if matches!(
                &outcome,
                ShipBrokerOutcome::PurchaseApplied { outcome }
                    if matches!(
                        outcome.status,
                        crate::shops::ShipwrightPurchaseStatus::QueuedFrigate
                            | crate::shops::ShipwrightPurchaseStatus::QueuedSkiff
                            | crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate
                    )
            ) {
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            append_active_shop_surcharge(format_ship_broker_outcome(outcome), surcharge)
        }
        ActiveShopSession::Guild(s) => {
            let mut gems = state.gems;
            let mut keys = state.keys;
            let mut torches = state.torches;
            let outcome = match (*s, inline_digit) {
                (GuildShopState::Greeting { .. } | GuildShopState::PickItem { .. }, _) => {
                    step_guild_shop(
                        s,
                        GuildShopInput::Key(key_byte),
                        &mut state.gold,
                        &mut gems,
                        &mut keys,
                        &mut torches,
                    )
                }
                (GuildShopState::PickQuantity { .. }, _) => {
                    if let Some(q) = inline_quantity.and_then(|q| u8::try_from(q).ok()) {
                        step_guild_shop(
                            s,
                            GuildShopInput::Quantity(q),
                            &mut state.gold,
                            &mut gems,
                            &mut keys,
                            &mut torches,
                        )
                    } else {
                        GuildShopOutcome::InvalidInput
                    }
                }
                _ => GuildShopOutcome::InvalidInput,
            };
            state.gems = gems;
            state.keys = keys;
            state.torches = torches;
            format_guild_outcome(outcome)
        }
    };
    state.message = message;

    if let Some(next_session) = replacement_session {
        state.active_shop = Some(next_session);
    } else if !session.is_exited() {
        state.active_shop = Some(session);
    }
    PlayInputDisposition::Continue
}

fn horse_sale_position(state: &PlayState) -> Option<(usize, usize)> {
    [
        Direction::South,
        Direction::North,
        Direction::East,
        Direction::West,
    ]
    .into_iter()
    .filter_map(|direction| state.adjacent_position(direction))
    .find(|(x, y)| {
        matches!(state.current_area_tile(*x, *y), 0x44 | 0x45 | 0x05)
            && state.object_at_current_floor(*x, *y).is_none()
            && state.npc_at_current_floor(*x, *y).is_none()
    })
}

fn active_speaker_intelligence(state: &PlayState) -> u8 {
    let slot = state.active_player.unwrap_or(0);
    state
        .party_intelligence
        .get(slot)
        .copied()
        .or_else(|| state.party_intelligence.first().copied())
        .unwrap_or(0)
}

fn active_shop_text_line(key: char, suffix: &str) -> String {
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    line
}

fn parse_active_shop_inline_quantity(key: char, suffix: &str) -> Option<u16> {
    let digits: String = std::iter::once(key)
        .chain(suffix.chars())
        .filter(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn active_inn_scene_marker(state: &PlayState) -> u8 {
    match state.area {
        Area::Town { scene, .. } => scene.byte,
        _ => 0,
    }
}

/// `systems/shops.md §8.0` shop-trigger table: `.NPC` dialogue byte `0x81`
/// is the "Weaponsmith / armourer" row, and the same byte keys the arms
/// column of the resident vendor-name table.
const SHOP_DIALOG_ID_ARMS: u8 = 0x81;

/// `systems/shops.md §8.0`: "Two resident name tables are indexed by the same
/// row: the shop's display name ... and the vendor's name, which fills the `$`
/// substitution and the `says <shopkeeper>.` / `yells <shopkeeper>.`
/// attribution tails. ... the shopkeeper an implementation names in shop text
/// is a property of the location, not of the NPC the player happened to talk
/// to." So the arms tails of `§8.1` read the arms row of that table by the
/// live town scene byte, and never the shop's display label.
///
/// Returns `None` outside a town and for any scene the arms table does not
/// list; the render sites then print the resident line unattributed rather
/// than inventing a name.
fn active_arms_shopkeeper_name(state: &PlayState) -> Option<&'static str> {
    let scene = match state.area {
        Area::Town { scene, .. } => scene.byte,
        _ => return None,
    };
    arms_shopkeeper_name_for_scene(scene)
}

/// The arms row of the `systems/shops.md §8.0` vendor-name table, reached
/// through the single implementation of that table in
/// `play_state_impl::chunk_04::shop_vendor_name_for_scene`. Copying the nine
/// published rows here instead would give the table a second source of truth,
/// which the table's own doc comment warns against.
fn arms_shopkeeper_name_for_scene(scene_byte: u8) -> Option<&'static str> {
    crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_ARMS, scene_byte)
}

fn active_shop_surcharge_sentinel(state: &PlayState) -> u8 {
    state.shared_town_conversation_sentinel()
}

fn apply_active_shop_surcharge(state: &mut PlayState) -> Option<ShopSurchargeOutcome> {
    let sentinel = active_shop_surcharge_sentinel(state);
    let roll_seed = if sentinel == SHOP_SURCHARGE_SENTINEL_ENABLES {
        state.random_range_u8(1, SHOP_SURCHARGE_GOLD_MAX as u8) - 1
    } else {
        0
    };
    let outcome = apply_shop_surcharge(&mut state.gold, sentinel, roll_seed);
    outcome.applied.then_some(outcome)
}

fn append_active_shop_surcharge(
    mut message: String,
    surcharge: Option<ShopSurchargeOutcome>,
) -> String {
    if let Some(outcome) = surcharge {
        message.push_str(&format!(" Surcharge {} gold.", outcome.surcharge));
    }
    message
}

fn apply_paid_inn_rest(state: &mut PlayState, cost: u16) -> String {
    const INN_REST_HOURS: u8 = 8;
    state.mark_town_rest_sleepers();
    for _ in 0..INN_REST_HOURS {
        state.advance_turn_with_minutes(MINUTES_PER_HOUR);
    }
    let woke = state.wake_town_rest_sleepers();
    let (recovered_hp, recovered_mana, cured) = state.apply_inn_rest_night_recovery();
    format!(
        "Rested {INN_REST_HOURS} hours at the inn for {cost} gold; recovered {recovered_hp} HP and {recovered_mana} MP; cured {cured} poisoned member(s); woke {woke} asleep member(s)."
    )
}

fn handle_arms_shop_key_input(
    state: &mut PlayState,
    shop_state: &mut crate::shop_runtime::ArmsShopState,
    stock_table: Option<crate::shops::ArmsStockTable>,
    ctx: crate::shop_runtime::ShopTransactionContext,
    key_byte: u8,
    inline_digit: Option<u8>,
    yes: bool,
    no: bool,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::{
        ArmsSellBrowserCommand, ArmsShopInput, ArmsShopOutcome, ArmsShopState, step_arms_shop,
    };

    let mut prices = [0u16; crate::EQUIPMENT_COUNT];
    prices.copy_from_slice(&crate::EQUIPMENT_BASE_PRICES);
    let mut stock = state.equipment_stock;
    let prior_state = *shop_state;
    let outcome = match (prior_state, yes, no, inline_digit) {
        (ArmsShopState::Greeting, _, _, _) => step_arms_shop(
            shop_state,
            ArmsShopInput::Key(key_byte),
            ctx,
            &mut state.gold,
            &mut stock,
            &prices,
        ),
        (ArmsShopState::BuyPickItem, _, _, _) if matches!(key_byte, b' ' | 0x1b) => {
            *shop_state = ArmsShopState::Exited;
            ArmsShopOutcome::Exited
        }
        (ArmsShopState::BuyPickItem, _, _, _) => {
            if let Some(table) = stock_table {
                step_arms_shop(
                    shop_state,
                    ArmsShopInput::StockLetter {
                        letter: key_byte,
                        table,
                    },
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                )
            } else if let Some(d) = inline_digit {
                step_arms_shop(
                    shop_state,
                    ArmsShopInput::Item(d),
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                )
            } else {
                ArmsShopOutcome::InvalidInput
            }
        }
        (ArmsShopState::SellPickItem(_), _, _, _) => {
            let command = match key_byte {
                b'\r' | b'\n' | b' ' => Some(ArmsSellBrowserCommand::Select),
                0x1b => Some(ArmsSellBrowserCommand::Exit),
                crate::INPUT_CODE_WEST | crate::INPUT_CODE_NORTH => {
                    Some(ArmsSellBrowserCommand::Previous)
                }
                crate::INPUT_CODE_EAST | crate::INPUT_CODE_SOUTH => {
                    Some(ArmsSellBrowserCommand::Next)
                }
                crate::INPUT_CODE_NORTHWEST => Some(ArmsSellBrowserCommand::First),
                crate::INPUT_CODE_SOUTHWEST => Some(ArmsSellBrowserCommand::Last),
                crate::INPUT_CODE_NORTHEAST => Some(ArmsSellBrowserCommand::PageUp),
                crate::INPUT_CODE_SOUTHEAST => Some(ArmsSellBrowserCommand::PageDown),
                _ => None,
            };
            command.map_or(ArmsShopOutcome::InvalidInput, |command| {
                step_arms_shop(
                    shop_state,
                    ArmsShopInput::SellBrowser(command),
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                )
            })
        }
        (ArmsShopState::BuyConfirm { .. } | ArmsShopState::SellConfirm { .. }, true, _, _) => {
            step_arms_shop(
                shop_state,
                ArmsShopInput::Confirm(true),
                ctx,
                &mut state.gold,
                &mut stock,
                &prices,
            )
        }
        (ArmsShopState::BuyConfirm { .. } | ArmsShopState::SellConfirm { .. }, _, true, _) => {
            step_arms_shop(
                shop_state,
                ArmsShopInput::Confirm(false),
                ctx,
                &mut state.gold,
                &mut stock,
                &prices,
            )
        }
        _ => ArmsShopOutcome::InvalidInput,
    };
    state.equipment_stock = stock;
    let surcharge = if matches!(outcome, ArmsShopOutcome::Bought { .. }) {
        apply_active_shop_surcharge(state)
    } else {
        None
    };
    let was_invalid_stock_pick = matches!(outcome, ArmsShopOutcome::InvalidInput)
        && matches!(prior_state, ArmsShopState::BuyPickItem)
        && stock_table.is_some();
    let confirmation_prompt_roll = matches!(outcome, ArmsShopOutcome::QuotedBuyPrice { .. })
        .then(|| state.random_range_u8(0, 3));
    let no_credit_roll = matches!(outcome, ArmsShopOutcome::BuyRefusedShortFunds { .. })
        .then(|| state.random_range_u8(0, 3));
    let speech = ArmsShopSpeech {
        shopkeeper: active_arms_shopkeeper_name(state),
        speaker_is_female: active_speaker_is_female(state),
    };
    let message = match (outcome, stock_table) {
        // `systems/shops.md §8.1`: "The list is preceded by a heading line and
        // one of four resident 'what we have' call lines chosen with a uniform
        // `0..3` draw." The draw is made here, where the list is first
        // rendered, and nowhere else — see `arms_stock_call_for_roll`.
        (ArmsShopOutcome::EnteredBuy, Some(table)) => {
            let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
            format!("{call}\n{}", format_arms_stock_buy_menu(table))
        }
        (ArmsShopOutcome::InvalidInput, Some(table)) if was_invalid_stock_pick => {
            format_arms_stock_buy_menu(table)
        }
        (ArmsShopOutcome::InvalidInput, _)
            if matches!(
                prior_state,
                ArmsShopState::SellPickItem(_) | ArmsShopState::SellConfirm { .. }
            ) =>
        {
            state.message.clone()
        }
        (ArmsShopOutcome::InvalidInput, _) => match prior_state {
            ArmsShopState::BuyConfirm {
                item,
                quoted_price,
                quote_record_id,
            } => format_arms_outcome_with_rolls(
                ArmsShopOutcome::QuotedBuyPrice {
                    item,
                    price: quoted_price,
                    quote_record_id,
                },
                game_dir,
                None,
                None,
                speech,
            ),
            _ => format_arms_outcome(ArmsShopOutcome::InvalidInput, game_dir),
        },
        (ArmsShopOutcome::EnteredSell, _) => arms_sell_entry_prompt(state.random_range_u8(0, 3)),
        (ArmsShopOutcome::SellBrowserMoved, _) => state.message.clone(),
        (ArmsShopOutcome::OfferedSellPrice { item, offer }, _) => {
            let record_id = crate::shops::SHOPPE_RECORDS_ARMS_SELL_FIRST
                + usize::from(state.random_range_u8(0, 7));
            let quote = render_shoppe_record_for_arms_quote(game_dir, record_id, item, offer);
            format!("{quote}\nDeal? (Y/N)")
        }
        (ArmsShopOutcome::SellRefusedZeroPrice { .. }, _)
            if matches!(shop_state, ArmsShopState::SellPickItem(_)) =>
        {
            format!(
                "I cannot buy that.\n{}",
                arms_sell_continuation_prompt(state.random_range_u8(0, 3))
            )
        }
        (ArmsShopOutcome::Declined, _) if matches!(shop_state, ArmsShopState::SellPickItem(_)) => {
            format!(
                "No\n{}",
                arms_sell_continuation_prompt(state.random_range_u8(0, 3))
            )
        }
        (
            ArmsShopOutcome::Sold {
                browser_continues: true,
                ..
            },
            _,
        ) => format!(
            "Sold!\n{}",
            arms_sell_continuation_prompt(state.random_range_u8(0, 3))
        ),
        (
            ArmsShopOutcome::Sold {
                browser_continues: false,
                ..
            },
            _,
        ) => format!("Sold!\n{}", arms_sell_goodbye(state.random_range_u8(0, 3))),
        (ArmsShopOutcome::Exited, _) if matches!(prior_state, ArmsShopState::SellPickItem(_)) => {
            arms_sell_goodbye(state.random_range_u8(0, 3))
        }
        (outcome, _) => format_arms_outcome_with_rolls(
            outcome,
            game_dir,
            confirmation_prompt_roll,
            no_credit_roll,
            speech,
        ),
    };
    append_active_shop_surcharge(message, surcharge)
}

fn arms_sell_entry_prompt(roll: u8) -> String {
    [
        "Which item wouldst thou like to sell?",
        "What dost thou wish to sell?",
        "Show me what ye got...",
        "What dost thou have for me to buy?",
    ][usize::from(roll) % 4]
        .to_string()
}

fn arms_sell_continuation_prompt(roll: u8) -> String {
    [
        "What else can ye offer me?",
        "What else hath ye to sell?",
        "What else doth thou wish to sell?",
        "What other arms wilt thou sell?",
    ][usize::from(roll) % 4]
        .to_string()
}

fn arms_sell_goodbye(roll: u8) -> String {
    [
        "Good-bye...",
        "Mayhap another time...",
        "Godspeed...",
        "Fare thee well...",
    ][usize::from(roll) % 4]
        .to_string()
}

/// `systems/shops.md §8.1` ("The list is preceded by a heading line and one of
/// four resident 'what we have' call lines chosen with a uniform `0..3` draw",
/// with the draw table immediately below it) and the `§8.A` resident-literal
/// row "Arms stock-call pool (verbatim)". Printed once above the arms buy
/// stock list.
///
/// `§8.A` also states that a plain ignored-key wait "does not re-render the
/// visible quote or menu, and does not consume a random bark draw", and `§8.1`
/// states that invalid buy selectors print no refusal line — so the draw is
/// made only where the list is first rendered, never on an invalid stock
/// letter.
const fn arms_stock_call_for_roll(roll: u8) -> &'static str {
    match roll & 0x03 {
        0 => "What may I show thee?",
        1 => "Which wouldst thou like to see?",
        2 => "What is thine interest?",
        _ => "Which would ye see?",
    }
}

/// `systems/shops.md §8.1`: "It then prints the post-item prompt
/// `Anything else,` followed by `milady?` when the speaking member's gender
/// field is the female value and `sir?` otherwise, or `then?` when no
/// transaction has completed in this visit." Also the `§8.A` resident-literal
/// row "Arms successful sale tail", and the `§8.A` wording policy paragraph
/// which lists the "anything else" tail and its gendered suffixes among the
/// literals published verbatim.
fn arms_post_item_prompt(speaker_is_female: bool, transaction_completed: bool) -> String {
    let suffix = if !transaction_completed {
        "then?"
    } else if speaker_is_female {
        "milady?"
    } else {
        "sir?"
    };
    format!("Anything else, {suffix}")
}

/// `systems/shops.md §8.1`: the arms "anything else" tail is addressed by the
/// *speaking* party member's gender field — `formats/saved-gam.md §3.1` record
/// offset `0x09`, value `0x0B` male / `0x0C` female, see
/// [`crate::SAVE_GENDER_FEMALE_BYTE`].
///
/// The speaker is the same party member the five stat-sensitive price paths
/// use: `systems/shops.md §2` says every Talk shop arm receives one caller
/// context word that "member-sensitive price paths use ... as the speaking
/// party member's roster slot", so this resolves the same slot as
/// [`active_speaker_intelligence`]. `systems/shops.md §8.A` contrasts this
/// tail with the shipwright's — "the arms tail, by contrast, selects
/// correctly" — so the feminine form must be reachable here.
///
/// The record is resolved by member identity, not by bare slot index: the
/// gender byte has no parallel active-party vector, and the inn's
/// leave/pick-up helpers and New Order reshuffle the active party without
/// reshuffling `party_roster`. See `party_roster_record_for_active_slot`.
/// A slot with no roster record falls back to the leader, and an absent
/// roster takes the spec's explicit "otherwise" branch.
fn active_speaker_is_female(state: &PlayState) -> bool {
    let slot = state.active_player.unwrap_or(0);
    crate::party::party_roster_record_for_active_slot(
        &state.party_roster,
        slot,
        state.party_names.get(slot),
    )
    .is_some_and(PartyRosterRecord::is_female)
}

fn format_arms_stock_buy_menu(table: crate::shops::ArmsStockTable) -> String {
    if table.is_empty() {
        return "We have nothing for sale.".to_string();
    }
    let mut entries = Vec::new();
    for index in 0..table.len() {
        let item = table.item_ids[index] as usize;
        let letter = (b'a' + index as u8) as char;
        entries.push(format!("{letter}) {}", equipment_name(item)));
    }
    format!("We have: {}.", entries.join(", "))
}

fn format_inn_error(err: InnError) -> String {
    match err {
        InnError::EmptyParty => "No one is here to lodge.".to_string(),
        InnError::PartyTooSmallToLeave => "Thou must keep at least one companion.".to_string(),
        InnError::PartyFull => "Thy party is already full.".to_string(),
        InnError::InvalidPartyIndex { .. } => "That companion is not in thy party.".to_string(),
        InnError::InvalidGuestIndex { .. } | InnError::GuestNotAtInn { .. } => {
            "No one here is from thy party!".to_string()
        }
        InnError::RegistryFull => "The inn has no more room for guests.".to_string(),
        InnError::BelowMinimumGold { minimum, .. } => {
            format!("Thou needest at least {minimum} gold to lodge here.")
        }
        InnError::InsufficientGold { required, .. } => {
            format!("Thou lackest the {required} gold.")
        }
    }
}

fn format_arms_outcome(outcome: crate::shop_runtime::ArmsShopOutcome, game_dir: &Path) -> String {
    format_arms_outcome_with_rolls(outcome, game_dir, None, None, ArmsShopSpeech::default())
}

/// Render-time context for the arms buy path's resident literals: the
/// shopkeeper name that fills the `<shopkeeper>` slot of the
/// `yells <shopkeeper>.` / `says <shopkeeper>.` attribution tails of
/// `systems/shops.md §8.1` and `§8.A`, and the speaking member's gender for
/// the post-item "anything else" tail.
#[derive(Clone, Copy, Debug, Default)]
struct ArmsShopSpeech {
    /// `systems/shops.md §8.0`: the shopkeeper's name is a property of the
    /// *location*, not of the NPC the player talked to. `None` when the live
    /// scene is not one of the nine published arms rows, in which case there
    /// is no published name to attribute the line to and the bare line is
    /// printed rather than an invented name.
    shopkeeper: Option<&'static str>,
    speaker_is_female: bool,
}

impl ArmsShopSpeech {
    /// `systems/shops.md §8.1` / `§8.A`: wrap a resident arms line in an
    /// attribution tail. With no published shopkeeper name for the live
    /// scene the line is printed unattributed.
    fn attribute(self, line: &str, verb: &str) -> String {
        match self.shopkeeper {
            Some(shopkeeper) => format!("{line}\n{verb} {shopkeeper}."),
            None => line.to_string(),
        }
    }
}

fn format_arms_outcome_with_rolls(
    outcome: crate::shop_runtime::ArmsShopOutcome,
    game_dir: &Path,
    confirmation_prompt_roll: Option<u8>,
    no_credit_roll: Option<u8>,
    speech: ArmsShopSpeech,
) -> String {
    use crate::shop_runtime::ArmsShopOutcome::*;
    match outcome {
        EnteredBuy => "Buy: pick an item number.".to_string(),
        EnteredSell => "Sell: pick an item number.".to_string(),
        SellRefusedEmpty => "Thou hast nothing to sell.".to_string(),
        SellBrowserMoved => String::new(),
        Exited => "Farewell.".to_string(),
        QuotedBuyPrice {
            item,
            price,
            quote_record_id,
        } => {
            let quote = render_shoppe_record_for_arms_quote(game_dir, quote_record_id, item, price);
            format!(
                "{quote}\n{}",
                confirmation_prompt_roll
                    .map(crate::shops::arms_buy_confirmation_prompt_for_roll)
                    .unwrap_or_else(|| crate::shops::arms_buy_confirmation_prompt(item))
            )
        }
        OfferedSellPrice { item, offer } => {
            format!("I will pay {offer} gold for item {item}. (Y/N)")
        }
        SellRefusedZeroPrice { .. } => "I cannot buy that.".to_string(),
        SellRefusedAmmunition { .. } => "I buy no used ammunition.".to_string(),
        // `systems/shops.md §8.1`: a successful purchase "prints the fixed
        // success line `Sold!`", and "It then prints the post-item prompt
        // `Anything else,`" with the gendered suffix. The purchase that just
        // completed *is* a completed transaction this visit, so the neutral
        // `then?` form belongs to any future render site that runs the tail
        // before a sale, not to this one.
        Bought { .. } => format!(
            "Sold!\n{}",
            arms_post_item_prompt(speech.speaker_is_female, true)
        ),
        Sold { item, received, .. } => format!("Sold item {item} for {received} gold."),
        Declined => "As you wish.".to_string(),
        // `systems/shops.md §8.1` / `§8.A`: the drawn no-credit bark is
        // "wrapped in the shopkeeper-attribution tail `yells <shopkeeper>.`".
        BuyRefusedShortFunds { item, .. } => {
            let bark = no_credit_roll
                .map(crate::shops::arms_no_credit_bark_for_roll)
                .unwrap_or_else(|| crate::shops::arms_no_credit_bark(item));
            match speech.shopkeeper {
                Some(shopkeeper) => {
                    crate::shops::arms_no_credit_bark_with_attribution(bark, shopkeeper)
                }
                None => bark.to_string(),
            }
        }
        SellRefusedNoStock { item } => format!("Thou hast no item {item} to sell."),
        // `systems/shops.md §8.1`: "it prints the fixed refusal `Thou canst
        // not carry any more!` followed by the shopkeeper-attribution tail
        // `says <shopkeeper>.`" (`§8.A` row "Arms carry-cap refusal
        // (verbatim)" repeats both halves).
        BuyRefusedCapHit { .. } => speech.attribute("Thou canst not carry any more!", "says"),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn render_shoppe_record_for_arms_quote(
    game_dir: &Path,
    record_id: usize,
    item: u8,
    price: u16,
) -> String {
    let placeholders = crate::shoppe_bark::ShoppeBarkContext {
        gold: price,
        item_name: equipment_name(item as usize),
        ..Default::default()
    };
    crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
        .and_then(|renderer| {
            renderer
                .render_record(record_id, &placeholders)
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
        })
        .unwrap_or_else(|_| format!("{} costs {price} gold.", equipment_name(item as usize)))
}

fn healer_treatment_for_service(service: HealerService) -> HealerTreatment {
    match service {
        HealerService::Cure => HealerTreatment::Cure,
        HealerService::Heal => HealerTreatment::Heal,
        HealerService::Resurrect => HealerTreatment::Resurrect,
    }
}

fn healer_service_for_treatment(treatment: HealerTreatment) -> HealerService {
    match treatment {
        HealerTreatment::Cure => HealerService::Cure,
        HealerTreatment::Heal => HealerService::Heal,
        HealerTreatment::Resurrect => HealerService::Resurrect,
    }
}

fn active_healer_target_accepts(treatment: HealerTreatment, member: PartyMember) -> bool {
    match treatment {
        HealerTreatment::Cure => member.status == b'P',
        HealerTreatment::Heal => member.living() && member.hp < member.max_hp,
        HealerTreatment::Resurrect => member.status == b'D',
    }
}

fn format_healer_treatment_outcome(outcome: HealerTreatmentOutcome) -> String {
    let slot = outcome.target_index + 1;
    match outcome.quote.treatment {
        HealerTreatment::Cure => format!("Cured party member {slot}."),
        HealerTreatment::Heal => format!(
            "Healed party member {slot} to {}/{}.",
            outcome.hp_after, outcome.max_hp_after
        ),
        HealerTreatment::Resurrect => format!(
            "Resurrected party member {slot} ({}/{}).",
            outcome.hp_after, outcome.max_hp_after
        ),
    }
}

fn format_healer_treatment_error(error: crate::shops::HealerTreatmentError) -> String {
    match error {
        crate::shops::HealerTreatmentError::InsufficientGold { required, .. } => {
            format!("Thou lackest the {required} gold.")
        }
        crate::shops::HealerTreatmentError::InvalidTarget { .. } => {
            "I do not understand.".to_string()
        }
        crate::shops::HealerTreatmentError::Untreatable => {
            "That treatment is not needed.".to_string()
        }
    }
}

fn format_tavern_outcome(outcome: crate::shop_runtime::TavernOutcome) -> String {
    use crate::shop_runtime::TavernOutcome::*;
    match outcome {
        EnteredMenu {
            tavern,
            round_letter,
            secondary_letter,
            provisions_letter,
            lore_letter,
        } => {
            let provisions = provisions_letter
                .map(|letter| format!(", provisions ({letter})"))
                .unwrap_or_default();
            format!(
                "{}: drink round ({round_letter}), tavern ({secondary_letter}){provisions}, lore ({lore_letter}), or Space.",
                tavern.display_name()
            )
        }
        EnteredSagePrompt => "Of what wouldst thou hear my lore?".to_string(),
        RoundDrinkServed { tavern, cost } => {
            format!(
                "{} served a round for {cost} gold. Anything else? (Y/N)",
                tavern.display_name()
            )
        }
        SecondaryTavernSelected {
            tavern,
            letter,
            cost,
        } => {
            format!(
                "{} served {letter} for {cost} gold. Anything else? (Y/N)",
                tavern.display_name()
            )
        }
        PickBlueBoarDrink => "Choose Blue Boar drink A-F.".to_string(),
        ConfirmEnoughDrink => "Had enough? (Y/N)".to_string(),
        DeclinedEnoughDrink => "Anything else? (Y/N)".to_string(),
        BlueBoarDrinkServed { choice, cost } => {
            format!(
                "Blue Boar drink {:?} served for {cost} gold. Anything else? (Y/N)",
                choice
            )
        }
        PickProvisionQuantity { tavern, unit_price } => format!(
            "{} provisions cost {unit_price} gold each. Quantity?",
            tavern.display_name()
        ),
        ProvisionsPurchased {
            tavern,
            requested_quantity,
            purchased_quantity,
            paid,
            food_added,
            completion: _,
        } => format!(
            "{} sold {purchased_quantity}/{requested_quantity} provision packs for {paid} gold; food +{food_added}. Anything else? (Y/N)",
            tavern.display_name()
        ),
        CharityProvisions {
            tavern,
            food_added,
            record_id: _,
        } => format!(
            "{} offered table scraps; food +{food_added}. Farewell.",
            tavern.display_name()
        ),
        Continued {
            tavern,
            follow_up_record_id,
        } => format!(
            "Yes\n{} continues with SHOPPE.DAT record {follow_up_record_id}.",
            tavern.display_name()
        ),
        DeclinedContinuation => "No\nFarewell.".to_string(),
        NoSaleExit => "Farewell.".to_string(),
        IgnoredInput => String::new(),
        Declined => "Hrumph.\n\nAnything else for thee?".to_string(),
        RefusedShortFunds { .. } => TAVERN_AFFORDABILITY_REFUSAL_BARK.to_string(),
        RefusedNoLivingParty => "No one can drink right now.".to_string(),
        RefusedNoNeed => "Thou needest no provisions.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_tavern_outcome_with_shoppe(
    outcome: crate::shop_runtime::TavernOutcome,
    provision_quote_record_id: Option<usize>,
    no_sale_record_id: Option<usize>,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::TavernOutcome::*;

    let renderer = crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir).ok();
    let rendered = renderer.as_ref().and_then(|renderer| match outcome {
        EnteredMenu { tavern, .. } => renderer
            .render_record(
                tavern_menu_record_id(tavern),
                &crate::shoppe_bark::ShoppeBarkContext {
                    shop_name: tavern.display_name(),
                    ..Default::default()
                },
            )
            .ok(),
        PickProvisionQuantity { tavern, unit_price } => {
            provision_quote_record_id.and_then(|record_id| {
                renderer
                    .render_record(
                        record_id,
                        &crate::shoppe_bark::ShoppeBarkContext {
                            gold: unit_price,
                            shop_name: tavern.display_name(),
                            ..Default::default()
                        },
                    )
                    .ok()
            })
        }
        Continued {
            tavern,
            follow_up_record_id,
        } => renderer
            .render_record(
                follow_up_record_id,
                &crate::shoppe_bark::ShoppeBarkContext {
                    shop_name: tavern.display_name(),
                    ..Default::default()
                },
            )
            .ok()
            .map(|rendered| format!("Yes\n{rendered}")),
        DeclinedContinuation | NoSaleExit => no_sale_record_id.and_then(|record_id| {
            renderer
                .render_record(record_id, &crate::shoppe_bark::ShoppeBarkContext::default())
                .ok()
                .map(|rendered| {
                    if matches!(outcome, DeclinedContinuation) {
                        format!("No\n{rendered}")
                    } else {
                        rendered
                    }
                })
        }),
        _ => None,
    });
    rendered
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| format_tavern_outcome(outcome))
}

fn format_sage_outcome(outcome: crate::shop_runtime::SageOutcome) -> String {
    use crate::shop_runtime::SageOutcome::*;
    match outcome {
        QuotedFee { quote } => format!("That will cost {} gold. Pay? (Y/N)", quote.entry.fee),
        RumourFound { rendered, .. } => rendered,
        Declined => "Farewell.".to_string(),
        RefusedShortFunds { .. } => TAVERN_AFFORDABILITY_REFUSAL_BARK.to_string(),
        InputTooLong { limit, .. } => format!("Ask in {limit} characters or fewer."),
        NoTopicMatch => "That, I cannot help thee with.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_sage_outcome_with_shoppe(
    outcome: crate::shop_runtime::SageOutcome,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::SageOutcome::*;

    let renderer = crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir).ok();
    match outcome {
        QuotedFee { quote } => renderer
            .as_ref()
            .and_then(|renderer| {
                renderer
                    .render_sage_fee_quote_record(quote.entry.fee, None)
                    .ok()
            })
            .unwrap_or_else(|| format_sage_outcome(QuotedFee { quote })),
        RefusedShortFunds {
            required,
            available,
        } => renderer
            .as_ref()
            .and_then(|renderer| renderer.render_sage_short_funds_record(None).ok())
            .unwrap_or_else(|| {
                format_sage_outcome(RefusedShortFunds {
                    required,
                    available,
                })
            }),
        RumourFound { outcome, rendered } => renderer
            .as_ref()
            .and_then(|renderer| {
                renderer
                    .render_sage_rumour_record(
                        outcome.record_id,
                        outcome.quote.entry.subject,
                        outcome.quote.entry.destination,
                        None,
                    )
                    .ok()
            })
            .unwrap_or(rendered),
        outcome => format_sage_outcome(outcome),
    }
}

fn format_reagent_outcome(outcome: crate::shop_runtime::ReagentShopOutcome) -> String {
    use crate::shop_runtime::ReagentShopOutcome::*;
    match outcome {
        EnteredMenu { herbalist } => {
            format!(
                "{} offers reagents A-E, or Space.",
                herbalist.display_name()
            )
        }
        QuotedUnit {
            herbalist,
            reagent,
            unit_price,
        } => format!(
            "{} sells {} for {unit_price} gold each. Quantity?",
            herbalist.display_name(),
            reagent.display_name()
        ),
        Bought {
            herbalist,
            reagent,
            quantity,
            paid,
        } => format!(
            "{} sold {quantity} {} for {paid} gold.",
            herbalist.display_name(),
            reagent.display_name()
        ),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        RefusedStockCap { cap, .. } => format!("Thou canst carry only {cap}."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_horse_trader_outcome(outcome: crate::shop_runtime::HorseTraderOutcome) -> String {
    use crate::shop_runtime::HorseTraderOutcome::*;
    match outcome {
        QuotedPrice { price } => format!("A fine steed costs {price} gold. (Y/N)"),
        Purchased { price } => format!("Sold for {price} gold. Thy horse awaits outside."),
        RefusedShortFunds { price } => format!("Thou lackest the {price} gold."),
        RefusedNoMarker { .. } => "There is no room for a horse here.".to_string(),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_ship_broker_outcome(outcome: crate::shop_runtime::ShipBrokerOutcome) -> String {
    use crate::shop_runtime::ShipBrokerOutcome::*;
    match outcome {
        QuotedPurchase { quote } => {
            let item = match quote.kind {
                crate::shops::ShipwrightPurchaseKind::Frigate => "frigate",
                crate::shops::ShipwrightPurchaseKind::Skiff => "skiff",
            };
            format!("A {item} costs {} gold. (Y/N)", quote.price)
        }
        PurchaseApplied { outcome } => match outcome.status {
            crate::shops::ShipwrightPurchaseStatus::QueuedFrigate => {
                format!(
                    "Frigate purchased for {} gold. Delivery is queued.",
                    outcome.quote.price
                )
            }
            crate::shops::ShipwrightPurchaseStatus::QueuedSkiff => {
                format!(
                    "Skiff purchased for {} gold. Delivery is queued.",
                    outcome.quote.price
                )
            }
            crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate => format!(
                "Skiff purchased for {} gold and added to the pending frigate.",
                outcome.quote.price
            ),
            crate::shops::ShipwrightPurchaseStatus::ExistingDeliveryRefusal => {
                "No dock space is available for another delivery.".to_string()
            }
        },
        RefusedShortFunds { required, .. } => format!("Thou lackest the {required} gold."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn format_guild_outcome(outcome: crate::shop_runtime::GuildShopOutcome) -> String {
    use crate::shop_runtime::GuildShopOutcome::*;
    match outcome {
        EnteredMenu { shop } => format!(
            "{}: Keys (A), Gems (B), Torches (C), or Space.",
            shop.display_name()
        ),
        QuotedUnit {
            shop,
            commodity,
            unit_price,
        } => format!(
            "{} sells {} for {unit_price} gold each. Quantity?",
            shop.display_name(),
            commodity.display_name()
        ),
        Bought {
            shop,
            commodity,
            quantity,
            paid,
        } => format!(
            "{} sold {quantity} {} for {paid} gold.",
            shop.display_name(),
            commodity.display_name()
        ),
        RefusedShortFunds { cost } => format!("Thou lackest the {cost} gold."),
        RefusedStockCap { cap, .. } => format!("Thou canst carry only {cap}."),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        InvalidInput => "I do not understand.".to_string(),
    }
}

fn handle_active_conversation_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    // The conversation loop accepts free-text keyword lines. When the
    // outer dispatcher hands us a single key plus an optional suffix
    // we treat the concatenation as the typed line. Pressing a bare
    // Enter (key `\r` or `\n`) submits an empty line, which closes
    // the conversation via the Bye shortcut.
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    let line = line.trim().to_string();
    let (text, ended) = state.submit_active_conversation_keyword(&line);
    if !ended {
        if let Some(session) = state.active_conversation.as_ref() {
            let prompt = session.prompt_message();
            state.message = if prompt.is_empty() {
                text
            } else if text.is_empty() {
                prompt.to_string()
            } else if text.ends_with('\n') {
                format!("{text}{prompt}")
            } else {
                format!("{text}\n{prompt}")
            };
        }
    }
    PlayInputDisposition::Continue
}

fn handle_active_blackthorn_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    state.submit_blackthorn_audience_answer(&line, game_dir)?;
    Ok(PlayInputDisposition::Continue)
}

fn handle_endgame_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    state.ensure_endgame_messages_loaded(game_dir)?;
    let answer = parse_inline_yes_no(suffix).or_else(|| match key {
        'Y' | 'y' => Some(true),
        'N' | 'n' => Some(false),
        _ => None,
    });
    if let Some(answer) = answer {
        state.resolve_endgame_confirmation_from_game_dir(answer, game_dir)?;
    } else if state
        .endgame
        .as_ref()
        .is_some_and(EndgameState::is_terminal)
    {
        state.resolve_endgame_confirmation(false);
    } else {
        state.message = "Endgame confirmation requires Y or N.".to_string();
    }
    Ok(PlayInputDisposition::Continue)
}

/// `magic.md §8`: "Nothing routes a summoned creature through the
/// player command parser, and the player never gets to move it."
/// The dispatch decision is `combat.md §6.1a`'s slot-to-group helper,
/// not the controlled bit read directly - a party-side actor carrying
/// that bit (Sword of Chaos, possession, Charm) goes to the automatic
/// driver, and a monster-side actor never gets a keystroke prompt.
fn combat_actor_accepts_player_input(slot: usize, actor: CombatActorDescriptor) -> bool {
    combat_actor_is_active_not_dead(actor) && combat_slot_takes_player_command_path(slot, actor)
}

fn combat_has_dispatchable_player_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .enumerate()
        .take(COMBAT_ACTOR_SLOTS)
        .map(|(slot, actor)| (slot, *actor))
        .any(|(slot, actor)| combat_actor_accepts_player_input(slot, actor))
}

fn combat_has_active_non_party_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .skip(COMBAT_PARTY_ACTOR_SLOTS)
        .copied()
        .any(combat_actor_is_active_not_dead)
}

fn combat_pending_player_actor_is_active(state: &PlayState, actor_slot: usize) -> bool {
    state
        .combat_actors
        .get(actor_slot)
        .copied()
        .is_some_and(|actor| combat_actor_accepts_player_input(actor_slot, actor))
}

fn handle_combat_cast_key_input(
    state: &mut PlayState,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message.clear();
        return Ok(PlayInputDisposition::Continue);
    };
    if !combat_pending_player_actor_is_active(state, actor_slot) {
        state.message.clear();
        return Ok(PlayInputDisposition::Continue);
    }

    // `combat.md §8`: the player's cast path reads only the Negate Magic tag.
    // The single Quickness gate lives at the head of the automatic actor
    // driver, not here.
    let had_foe = combat_has_active_non_party_actor(state);
    let turn_before = state.turn;
    let cast_suffix = combat_cast_suffix_for_actor(suffix, actor_slot);
    let outcome = state.cast_spell_from_suffix(&cast_suffix, game_dir)?;
    state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    finish_combat_cast_actor_action(state, actor_slot, had_foe);

    Ok(PlayInputDisposition::Continue)
}

/// Finish an accepted combat C-Cast action, including cancel/no-spell paths.
/// The dead-caster refusal never reaches this helper and remains a free
/// re-prompt under `combat.md §8`.
fn finish_combat_cast_actor_action(state: &mut PlayState, actor_slot: usize, had_foe: bool) {
    state.pending_combat_actor_slot = None;
    let cast_message = state.message.clone();
    let ring_pass = apply_combat_committed_action_maintenance(state, actor_slot);
    state.message = combat_magic_ring_pass_message(ring_pass).unwrap_or(cast_message);

    if state.combat_active
        && matches!(
            state.combat_round_loop_control(false, false),
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
        )
    {
        state.apply_combat_round_loop_exit(CombatRoundLoopExit::Defeat);
    } else if state.combat_active && had_foe && !combat_has_active_non_party_actor(state) {
        // `combat.md §7`: "If party actors remain and foes do not, it
        // prints `VICTORY!` once and continues" (`RETRACTIONS.md` R289).
        if state.announce_combat_victory_if_needed() {
            state
                .message
                .push_str(crate::combat_frame::COMBAT_VICTORY_LINE);
        }
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    } else if state.combat_active {
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }
}

fn handle_combat_key_input(state: &mut PlayState, key: char, suffix: &str) -> PlayInputDisposition {
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message.clear();
        return PlayInputDisposition::Continue;
    };
    if !combat_pending_player_actor_is_active(state, actor_slot) {
        state.message.clear();
        return PlayInputDisposition::Continue;
    }
    let input = combat_player_command_input_from_key_suffix(key, suffix);
    // `combat.md §8.1`: the banner was already emitted into the transcript
    // when this turn opened - "before any key is read" - so this keystroke's
    // own transcript starts with the command output and never reprints it.
    // The free re-prompt below reinstates the pending slot without reopening
    // the turn, which is what keeps that path on the short form.
    let Some(application) = state.apply_combat_player_command_with_inputs(actor_slot, input) else {
        state.message.clear();
        return PlayInputDisposition::Continue;
    };
    state.message = combat_player_command_application_message(state, &application);
    if handle_combat_multistage_command(state, actor_slot, &application.action, suffix) {
        return PlayInputDisposition::Continue;
    }
    if application.reprompt {
        state.pending_combat_actor_slot = Some(actor_slot);
        return PlayInputDisposition::Continue;
    }
    if let CombatRoundLoopControl::Exit(exit) = application.control_after {
        let edge_defeat_message = (exit == CombatRoundLoopExit::Defeat
            && application.out_of_arena_leave.is_some_and(|edge| {
                matches!(edge.outcome, CombatOutOfArenaLeaveOutcome::Accepted { .. })
            }))
        .then(|| state.message.clone());
        state.apply_combat_round_loop_exit(exit);
        if let Some(edge_message) = edge_defeat_message {
            state.message = format!("{edge_message}\nBATTLE IS LOST!");
        }
    } else if matches!(
        application.action,
        CombatPlayerCommandAction::PromptForAttackDirection
    ) {
        state.pending_combat_actor_slot = Some(actor_slot);
    } else if application.control_after.result_code().is_none() {
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }
    PlayInputDisposition::Continue
}

fn handle_combat_multistage_command(
    state: &mut PlayState,
    actor_slot: usize,
    action: &CombatPlayerCommandAction,
    suffix: &str,
) -> bool {
    let CombatPlayerCommandAction::Branch {
        branch,
        live_actor_gate,
    } = action
    else {
        return false;
    };
    if matches!(
        live_actor_gate,
        CombatCommandLiveActorGate::RejectedDeadOrMissing
    ) {
        state.message.clear();
        return false;
    }

    match branch {
        CombatCommandBranch::Ready => {
            state.start_combat_ready_equipment(actor_slot);
            state.pending_combat_actor_slot = Some(actor_slot);
            true
        }
        CombatCommandBranch::UseItem => {
            // `combat.md §8` / `inventory.md §7`: combat U is Shape A, not
            // the withdrawn scene-refusal branch. It prints the normal verb
            // echo, passes the live-actor gate above, and enters the same item
            // picker used by world modes.
            state.begin_command_echo_for(Command::Use);
            state.start_use_item();
            if state.active_use.is_some() {
                state.pending_combat_actor_slot = Some(actor_slot);
            } else {
                state.pending_combat_actor_slot = None;
                let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                advance_combat_round_after_actor_and_append_message(state, actor_slot);
            }
            true
        }
        CombatCommandBranch::CastSpell => {
            if let Some(source_slot) = state.combat_cast_interference_source_for_slot(actor_slot) {
                state.message = format!(
                    "\n{} interferes!",
                    combat_interference_actor_name(state, source_slot)
                );
                state.pending_combat_actor_slot = Some(actor_slot);
                return true;
            }
            state.start_combat_cast_spell_prompt(
                actor_slot,
                combat_has_active_non_party_actor(state),
            );
            state.pending_combat_actor_slot = Some(actor_slot);
            true
        }
        CombatCommandBranch::ZStats => {
            state.z_stats_for_party(actor_slot);
            state.pending_combat_actor_slot = Some(actor_slot);
            true
        }
        CombatCommandBranch::Yell => {
            match combat_yell_word_from_suffix(suffix) {
                CombatInlineYell::Prompt => {
                    state.active_yell = Some(YellSession::new());
                    state.message = yell_prompt_message();
                    state.pending_combat_actor_slot = Some(actor_slot);
                }
                CombatInlineYell::Empty => {
                    state.message = YELL_NOTHING_SAID_MESSAGE.to_string();
                    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
                CombatInlineYell::Word(word) => {
                    let word = PlayState::normalize_yell_word(word);
                    state.message = format!("Yelled {word}. Nothing happens.");
                    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            }
            true
        }
        CombatCommandBranch::Push => {
            // `commands.md §8`: combat reaches the same pre-prompt door
            // cleanup and keeps the resident `Push-` echo open for either a
            // direction or Space/Pass.
            state.tick_door_tracker();
            state.begin_command_echo_for(Command::Push);
            if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                state.push_combat_actor_direction_after_cleanup(actor_slot, direction);
                state.prepend_push_direction_result(direction);
                let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                advance_combat_round_after_actor_and_append_message(state, actor_slot);
            } else {
                state.active_direction_prompt = Some(DirectionPromptSession::new(
                    DirectionPromptKind::CombatPush { actor_slot },
                ));
                state.message = state.render_active_direction_prompt();
            }
            true
        }
        CombatCommandBranch::Klimb => {
            if let Some(intent) = suffix.chars().find_map(combat_klimb_vertical_intent) {
                let outcome = state.klimb_combat_actor_vertical(actor_slot, intent);
                if matches!(outcome, MoveOutcome::Blocked) {
                    state.pending_combat_actor_slot = Some(actor_slot);
                } else if state.combat_active {
                    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                let outcome = state.klimb_combat_actor_direction(actor_slot, direction);
                if matches!(outcome, MoveOutcome::Blocked) {
                    state.pending_combat_actor_slot = Some(actor_slot);
                } else if state.combat_active {
                    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else {
                state.active_direction_prompt = Some(DirectionPromptSession::new(
                    DirectionPromptKind::CombatKlimb { actor_slot },
                ));
                state.message = state.render_active_direction_prompt();
            }
            true
        }
        CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Open
        | CombatCommandBranch::Search => {
            if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                state.combat_sjog_actor_direction(actor_slot, *branch, direction);
                if state.combat_active {
                    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else {
                state.active_direction_prompt = Some(DirectionPromptSession::new(
                    DirectionPromptKind::CombatSjog {
                        actor_slot,
                        branch: *branch,
                    },
                ));
                state.message = state.render_active_direction_prompt();
            }
            true
        }
        _ => false,
    }
}

fn combat_klimb_vertical_intent(ch: char) -> Option<ClimbIntent> {
    match ch {
        '<' => Some(ClimbIntent::Up),
        '>' => Some(ClimbIntent::Down),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CombatInlineYell<'a> {
    Prompt,
    Empty,
    Word(&'a str),
}

fn combat_yell_word_from_suffix(suffix: &str) -> CombatInlineYell<'_> {
    if suffix.is_empty() {
        return CombatInlineYell::Prompt;
    }
    match non_empty_yell_word(suffix) {
        Some(word) => CombatInlineYell::Word(word),
        None => CombatInlineYell::Empty,
    }
}

fn combat_magic_ring_pass_message(pass: Option<CombatMagicRingPassOutcome>) -> Option<String> {
    pass.and_then(|ring_pass| ring_pass.vanished_ring)
        .map(|ring| format!("{} vanished.", equipment_name(ring as usize)))
}

/// `combat.md §8`: every committed non-digit player action checks the Doom
/// companion band, runs common terrain/marker contact, performs visible-ring
/// maintenance, and then ages the active timed effect. Multi-stage commands
/// call this only when their final input commits the action.
fn apply_combat_committed_action_maintenance(
    state: &mut PlayState,
    actor_slot: usize,
) -> Option<CombatMagicRingPassOutcome> {
    let _ = state.apply_combat_absorbable_field_contact_for_actor_position(actor_slot);
    let _ = state.apply_combat_post_dispatch_contact_for_actor_position(actor_slot);
    state.clear_combat_interference_for_completed_action(actor_slot);
    let ring_pass = state.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
    let _ = state.age_active_effect();
    ring_pass
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
    if key.is_ascii_uppercase() {
        return CombatPlayerCommandInput::Key(key);
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
        // `combat.md §8.1`/`§8.2`: what `A` adds on top of the turn banner
        // is `Attack-` and, "immediately before the cursor opens", `Aim! `.
        CombatPlayerCommandAction::PromptForAttackDirection => {
            format!("{COMBAT_ATTACK_LABEL}{COMBAT_ATTACK_AIM_PROMPT}")
        }
        // Every production step-or-attack transcript comes from
        // `combat_step_or_attack_application_message`, which prints the
        // `combat.md §3` lines (the direction word, then `Blocked!`,
        // `Stay with ship!`, `Escape!` or `Leave!`). This arm held a
        // parallel set that
        // named arena coordinates and combatant slot numbers; nothing
        // reached it, and no published line looks like that.
        CombatPlayerCommandAction::StepOrAttack { .. } => String::new(),
        CombatPlayerCommandAction::InvalidDirection { .. } => "What?".to_string(),
        CombatPlayerCommandAction::EscapeCleanup { application } => match application.decision {
            CombatEscapeCleanupDecision::RefusedNotHere => "Escape-Not here!\n".to_string(),
            CombatEscapeCleanupDecision::RefusedNotYet => "Escape-Not yet!\n".to_string(),
            CombatEscapeCleanupDecision::Accepted => "Escape!".to_string(),
        },
        CombatPlayerCommandAction::Branch { branch, .. } => combat_command_branch_message(*branch),
    }
}

fn combat_command_branch_message(branch: CombatCommandBranch) -> String {
    if let Some(label) = combat_command_branch_published_label(branch) {
        return label.to_string();
    }

    match branch {
        CombatCommandBranch::SceneMessageAbort(verb) => match combat_scene_abort_tail(verb) {
            CombatSceneAbortTail::What => {
                format!("{} what?", combat_scene_abort_verb_prefix(verb))
            }
            CombatSceneAbortTail::NotHere => {
                format!("{}-Not here", combat_scene_abort_verb_prefix(verb))
            }
            CombatSceneAbortTail::FunnyNoResponse => {
                format!(
                    "{}-Funny, no response!",
                    combat_scene_abort_verb_prefix(verb)
                )
            }
        },
        CombatCommandBranch::Klimb => "Klimb-What?".to_string(),
        CombatCommandBranch::ToggleMusic => "Music toggled.".to_string(),
        CombatCommandBranch::Invalid => "What?".to_string(),
        CombatCommandBranch::Attack
        | CombatCommandBranch::CastSpell
        | CombatCommandBranch::Ready
        | CombatCommandBranch::UseItem
        | CombatCommandBranch::EscapeCleanup
        | CombatCommandBranch::Yell
        | CombatCommandBranch::ZStats
        | CombatCommandBranch::Pass
        | CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Open
        | CombatCommandBranch::Push
        | CombatCommandBranch::Search
        | CombatCommandBranch::DWhatRefusal
        | CombatCommandBranch::WWhatRefusal => format!("{branch:?}."),
    }
}

fn combat_player_command_application_message(
    state: &PlayState,
    application: &CombatPlayerCommandApplication,
) -> String {
    let message = match application.action {
        CombatPlayerCommandAction::StepOrAttack {
            direction_code,
            outcome,
            ..
        } => combat_step_or_attack_application_message(
            state,
            direction_code,
            outcome,
            application.out_of_arena_leave,
            application.weapon_attack,
        ),
        _ => combat_magic_ring_pass_message(application.ring_pass)
            .unwrap_or_else(|| combat_player_command_message(&application.action)),
    };
    if application.victory_announced {
        // `combat.md §7`/`§14`: once the post-action side recount finds no
        // hostile left, the round loop prints the resident `VICTORY!`
        // string through the ordinary string printer - one leading and one
        // trailing newline, one-shot - and then keeps running
        // (`RETRACTIONS.md` R289).
        let mut message = message;
        message.push_str(crate::combat_frame::COMBAT_VICTORY_LINE);
        return message;
    }
    message
}

fn combat_direction_code_name(direction_code: u8) -> &'static str {
    match direction_code {
        COMBAT_DIRECTION_WEST => "West",
        COMBAT_DIRECTION_EAST => "East",
        COMBAT_DIRECTION_NORTH => "North",
        COMBAT_DIRECTION_SOUTH => "South",
        _ => "What?",
    }
}

fn combat_step_or_attack_application_message(
    state: &PlayState,
    direction_code: u8,
    outcome: CombatStepOrAttackPrimitiveOutcome,
    edge: Option<CombatOutOfArenaLeaveApplication>,
    weapon_attack: Option<CombatWeaponAttackApplication>,
) -> String {
    let direction = combat_direction_code_name(direction_code);
    match outcome {
        CombatStepOrAttackPrimitiveOutcome::BlockedActor { .. }
        | CombatStepOrAttackPrimitiveOutcome::BlockedWall => {
            format!("{direction}\nBlocked!\n")
        }
        CombatStepOrAttackPrimitiveOutcome::OutOfArena { .. } => match edge.map(|e| e.outcome) {
            Some(CombatOutOfArenaLeaveOutcome::RefusedShipStyle) => {
                format!("{direction}\n\nStay with ship!\n")
            }
            Some(CombatOutOfArenaLeaveOutcome::RefusedConstrainedDirection { .. }) => {
                format!("{direction}\n\nAll must use the same exit!\n")
            }
            Some(CombatOutOfArenaLeaveOutcome::Accepted {
                presentation: CombatOutOfArenaLeavePresentation::EscapeWithFoes,
                ..
            }) => format!("{direction}\nEscape!\n"),
            Some(CombatOutOfArenaLeaveOutcome::Accepted {
                presentation: CombatOutOfArenaLeavePresentation::OrdinaryCleanup,
                ..
            }) => format!("{direction}\nLeave!\n"),
            _ => format!("{direction}\n"),
        },
        CombatStepOrAttackPrimitiveOutcome::Attack { target_slot } => {
            let result = weapon_attack
                .and_then(|attack| combat_weapon_attack_result_message(state, target_slot, attack));
            result
                .map(|result| format!("{direction}\n{result}\n"))
                .unwrap_or_else(|| format!("{direction}\n"))
        }
        CombatStepOrAttackPrimitiveOutcome::Moved { .. } => format!("{direction}\n"),
        CombatStepOrAttackPrimitiveOutcome::InactiveActor => String::new(),
    }
}

/// `combat.md §11.1` "The census": one attack outcome's printed result
/// line, for either side of the arena. Two rules from that section govern
/// every string here - "**Every result line names the target, never the
/// attacker.** No combat result line anywhere in the game is
/// attacker-named", and the two sides "share the to-hit roll, the impact
/// presentation, the damage roller and the result narrator". Internal
/// slots, coordinates, rolls and raw damage never belong in this string.
pub(crate) fn combat_weapon_attack_result_message(
    state: &PlayState,
    target_slot: usize,
    attack: CombatWeaponAttackApplication,
) -> Option<String> {
    combat_attack_result_message(state, target_slot, None, attack)
}

fn combat_attack_result_message(
    state: &PlayState,
    target_slot: usize,
    attacker_slot: Option<usize>,
    attack: CombatWeaponAttackApplication,
) -> Option<String> {
    let target_name = combat_actor_display_name(state, target_slot);
    match attack.resolution {
        // `combat.md §11.1` census, "To-hit fails | **party melee** |
        // `<target> missed!`, following the newline already printed before
        // the roll". The line is target-named: "`Bat missed!` is a real
        // original-game line, and it reads *the Bat was missed*: it is
        // printed by a party member's failed swing **at** a Bat".
        //
        // The party ranged and thrown arm shares this line, and §11.1
        // makes it conditional there: it prints "only when the resolver
        // reports nobody **and** the originally aimed cell held a real
        // actor". This engine models no scatter on that arm, so every
        // failed party ranged roll is that case.
        CombatWeaponAttackResolution::Miss { .. } => Some(format!("{target_name} missed!")),
        CombatWeaponAttackResolution::Hit { .. } => match attack.damage_application {
            Some(application) => {
                combat_landed_damage_result_line(state, target_slot, attacker_slot, application)
            }
            None => Some(format!("{target_name} hit!")),
        },
        // `combat.md §11.1` census: "Glass Sword swing | party melee |
        // `Thy sword hath shattered!`, printed **inside** the damage roll,
        // so it lands between the hit newline and the result line". The
        // ordinary result line then follows - for the sentinel that is
        // "Target dies | both | `<target> killed!`" - so the shatter line
        // is a prefix, not a replacement.
        CombatWeaponAttackResolution::Special { shattered, .. } => {
            let mut lines = Vec::new();
            if shattered {
                lines.push(COMBAT_GLASS_SWORD_SHATTER_LINE.to_string());
            }
            if let Some(result) = attack.damage_application.and_then(|application| {
                combat_landed_damage_result_line(state, target_slot, attacker_slot, application)
            }) {
                lines.push(result);
            }
            (!lines.is_empty()).then(|| lines.join("\n"))
        }
        CombatWeaponAttackResolution::OutOfRange { .. }
        | CombatWeaponAttackResolution::NoOrdinaryDamage { .. } => None,
    }
}

/// `combat.md §11.1` census rows for a landed swing, in the order the
/// section's "Order, stated once" list gives them: the graze arm first
/// (it "suppresses every later result line"), then the death arm, then
/// the ordinary hit.
fn combat_landed_damage_result_line(
    state: &PlayState,
    target_slot: usize,
    attacker_slot: Option<usize>,
    application: CombatWeaponDamageApplication,
) -> Option<String> {
    match application {
        // `combat.md §11.1`: "Damage zero or negative | both | `<target>
        // grazed!` **and nothing else** - the kill, sleep, hit and wound
        // lines are all suppressed". `RETRACTIONS.md` R352 withdrew the
        // former miss reading of this arm.
        CombatWeaponDamageApplication::Party { damage, .. } if damage.grazed => Some(format!(
            "{} grazed!",
            combat_actor_display_name(state, target_slot)
        )),
        CombatWeaponDamageApplication::Party { damage, .. } => {
            let target_name = combat_actor_display_name(state, target_slot);
            if damage.killed {
                // "Target dies | both | `<target> killed!`"
                return Some(format!("{target_name} killed!"));
            }
            // `combat.md §11.1`: "**A party member who takes a solid
            // landed hit always reads the flat `<target> hit!`** - or
            // `<target> dragged under!` when the attacker is a Corpser."
            // The grading "never applies to a **party** target".
            if attacker_slot.is_some_and(|slot| {
                state
                    .combat_actors
                    .get(slot)
                    .is_some_and(|actor| actor.owner_target_class == COMBAT_CLASS_CORPSER)
            }) {
                return Some(format!("{target_name} dragged under!"));
            }
            Some(format!("{target_name} hit!"))
        }
        CombatWeaponDamageApplication::Monster { damage, .. } => {
            let class_name = combat_class_stats(damage.class)
                .map(|stats| stats.name.to_string())
                .unwrap_or_else(|| combat_actor_display_name(state, target_slot));
            if damage.grazed {
                return Some(format!("{class_name} grazed!"));
            }
            if damage.killed {
                // `combat.md §11.1`: "Monster dies, vanish class | party
                // attacker | `<monster> vanishes!` ... printed inside the
                // damage handler, which then suppresses the kill line."
                // The vanish line itself is emitted by the death path in
                // `combat_frame.rs`.
                if damage.death_path == Some(CombatMonsterDeathPath::Vanish) {
                    return None;
                }
                return Some(format!("{class_name} killed!"));
            }
            // `combat.md §11.1` "The graded wound lines are monster-target
            // only": the result line "is graded by the target's remaining
            // HP against its class maximum, using the same four-bucket
            // wound score the flee classifier of Section 9 computes".
            let actor = state.combat_actors.get(target_slot)?;
            let max_hp = combat_class_stats(damage.class)?.max_hp;
            let wound_line = match combat_wound_score_bucket(actor.hp_or_wound, max_hp) {
                CombatWoundScoreBucket::ThreeQuartersOrMore => "barely wounded",
                CombatWoundScoreBucket::HalfToUnderThreeQuarters => "lightly wounded",
                CombatWoundScoreBucket::OneQuarterToUnderHalf => "heavily wounded",
                CombatWoundScoreBucket::UnderOneQuarter => "critical",
            };
            Some(format!("{class_name} {wound_line}!"))
        }
    }
}

fn combat_actor_display_name(state: &PlayState, slot: usize) -> String {
    if slot < COMBAT_PARTY_ACTOR_SLOTS {
        return state
            .combat_roster_slot_for_actor_slot(slot)
            .and_then(|roster_slot| state.party_names.get(roster_slot))
            .and_then(|name| party_name_to_string(name))
            .unwrap_or_else(|| format!("Party member {}", slot + 1));
    }
    let class = state
        .combat_actors
        .get(slot)
        .map(|actor| actor.owner_target_class)
        .unwrap_or_default();
    combat_class_stats(class)
        .map(|stats| stats.name.to_string())
        .unwrap_or_else(|| "Combatant".to_string())
}

/// `combat.md §11.1` for a self-acting hostile's turn. The two sides
/// "join *below* the announcement layer, which is why an ordinary hostile
/// monster prints no banner, no `Attack-`, no `Aim! ` and, on a melee
/// miss, no line at all" (`RETRACTIONS.md` R353).
pub(crate) fn combat_monster_attack_result_message(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
) -> Option<String> {
    let target_name = combat_actor_display_name(state, attack.target_slot);
    if matches!(
        attack.poison_status_outcome,
        Some(CombatPoisonStatusAttackOutcome::PoisonedPartyMember { .. })
    ) {
        return Some(format!("{target_name} is poisoned!"));
    }
    if matches!(
        attack.resolution,
        Some(CombatWeaponAttackResolution::Miss { .. })
    ) {
        // `combat.md §11.1` census: "To-hit fails | **monster melee** |
        // **nothing at all**", and the section text - "an ordinary hostile
        // monster's melee miss prints nothing and sounds nothing - no
        // newline, no name, no line, no tone". §11.1's controlled-monster
        // carve-out (a reduced banner and `<target> missed!`) does not
        // apply here: a monster carrying the controlled bit is toggled
        // into the party group and dispatched to `PlayerReady`, so it
        // narrates through the party-side helper above instead of this
        // one.
        //
        // The ranged carve-out - a failed monster ranged roll scatters and
        // "the **full hit chain runs against that actor**" - is not
        // modelled; this engine resolves a failed ranged roll as silent,
        // which §11.1 lists as one of its three genuinely silent cases.
        return None;
    }
    combat_attack_result_message(
        state,
        attack.target_slot,
        Some(attack.attacker_slot),
        CombatWeaponAttackApplication {
            resolution: attack.resolution?,
            damage_application: attack.damage_application,
        },
    )
}

fn append_combat_result_line(message: &mut String, line: &str) {
    if !message.is_empty() && !message.ends_with('\n') {
        message.push('\n');
    }
    message.push_str(line);
    message.push('\n');
}

fn append_combat_round_walk_messages(
    state: &mut PlayState,
    application: &CombatRoundWalkApplication,
) {
    let lines = application
        .applications
        .iter()
        .filter_map(|entry| match entry {
            CombatActorSlotDispatchApplication::Slot {
                action:
                    CombatActorDispatchAction::MonsterAi {
                        ai_turn: Some(ai_turn),
                    },
                ..
            } => ai_turn
                .monster_attack
                .and_then(|attack| combat_monster_attack_result_message(state, attack)),
            _ => None,
        })
        .collect::<Vec<_>>();
    for line in lines {
        append_combat_result_line(&mut state.message, &line);
    }
}

fn drive_combat_round_walk_and_append_message(state: &mut PlayState) {
    if !state.combat_active || state.pending_combat_actor_slot.is_some() {
        return;
    }

    for _ in 0..COMBAT_ROUND_WALK_DRAIN_LIMIT {
        let start_slot = state.next_combat_actor_slot.min(COMBAT_ACTOR_SLOTS);
        let application = if state.pace_combat_presentations {
            state.apply_combat_round_walk_from_slot_paced(
                start_slot,
                COMBAT_PHASE_REFRESH_CONSTANT,
                false,
            )
        } else {
            state.apply_combat_round_walk_from_slot(
                start_slot,
                COMBAT_PHASE_REFRESH_CONSTANT,
                false,
            )
        };
        append_combat_round_walk_messages(state, &application);
        state.next_combat_actor_slot = match application.stop_reason {
            CombatRoundWalkStopReason::EndOfRound => 0,
            CombatRoundWalkStopReason::AwaitingPlayer
            | CombatRoundWalkStopReason::AutomaticAction
            | CombatRoundWalkStopReason::Exit => application.next_slot,
        };
        if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
            // `combat.md §8.1`: opening a keyboard-driven turn prints the
            // turn banner, before any key is read.
            state.open_pending_combat_player_turn(ready_player_slot_from_input_round_walk(
                &application,
            ));
        }

        let should_stop = !matches!(
            application.stop_reason,
            CombatRoundWalkStopReason::EndOfRound
        ) || state.pending_combat_actor_slot.is_some();
        let exit = combat_round_walk_exit(&application);
        if let Some(exit) = exit {
            state.apply_combat_round_loop_exit(exit);
            break;
        }
        if should_stop {
            break;
        }
    }
}

/// Advance one paced automatic combat presentation. Graphical frontends call
/// this only while combat is active and no party actor is awaiting input.
pub fn advance_paced_combat_presentation(state: &mut PlayState) {
    if state.pace_combat_presentations {
        drive_combat_round_walk_and_append_message(state);
    }
}

fn advance_combat_round_after_actor_and_append_message(state: &mut PlayState, actor_slot: usize) {
    // Every call site reaches this boundary only after the actor has committed
    // its dispatched action. Keep the victim-local interference clear here as
    // a backstop for modal branches that can close without their normal tail.
    state.clear_combat_interference_for_completed_action(actor_slot);
    state.next_combat_actor_slot = actor_slot.saturating_add(1).min(COMBAT_ACTOR_SLOTS);
    if !state.pace_combat_presentations {
        drive_combat_round_walk_and_append_message(state);
    }
}

fn ready_player_slot_from_input_round_walk(
    application: &CombatRoundWalkApplication,
) -> Option<usize> {
    application.applications.iter().rev().find_map(|entry| {
        let CombatActorSlotDispatchApplication::Slot { slot, action, .. } = entry else {
            return None;
        };
        if matches!(action, CombatActorDispatchAction::PlayerReady) {
            Some(*slot)
        } else {
            None
        }
    })
}

fn combat_round_walk_exit(application: &CombatRoundWalkApplication) -> Option<CombatRoundLoopExit> {
    application
        .applications
        .iter()
        .rev()
        .find_map(|entry| match entry {
            CombatActorSlotDispatchApplication::EndOfRound { control }
            | CombatActorSlotDispatchApplication::Slot {
                control_after: control,
                ..
            } => match control {
                CombatRoundLoopControl::Exit(exit) => Some(*exit),
                CombatRoundLoopControl::ContinueActorWalk
                | CombatRoundLoopControl::StartNextRound => None,
            },
        })
}

fn combat_interference_actor_name(state: &PlayState, slot: usize) -> String {
    if slot < COMBAT_PARTY_ACTOR_SLOTS {
        return state
            .party_names
            .get(slot)
            .and_then(|name| party_name_to_string(name))
            .unwrap_or_else(|| format!("Party member {}", slot + 1));
    }
    let class = state
        .combat_actors
        .get(slot)
        .map(|actor| actor.owner_target_class)
        .unwrap_or_default();
    combat_class_stats(class)
        .map(|stats| stats.name.to_string())
        .unwrap_or_else(|| format!("Combatant {slot}"))
}

/// `commands.md §5.2` verb-echo table, last row: "any unmapped key" ->
/// `What?` plus a newline, and "the same text answers a key that is
/// recognised but meaningless in the current mode".
/// `text-output.md §10.3` repeats it: "An unrecognised command key
/// prints `What?` followed by a newline and **consumes no turn**."
///
/// The engine used to print an internal diagnostic naming the raw input
/// code here, which leaked a hex byte to the player and has no
/// counterpart in the original.
pub const UNRECOGNISED_COMMAND_MESSAGE: &str = "What?";

/// The raw input byte behind a dispatch key, when the key is one.
const fn input_byte_from_char(key: char) -> Option<u8> {
    let scalar = key as u32;
    if scalar <= u8::MAX as u32 {
        Some(scalar as u8)
    } else {
        None
    }
}

/// `systems/shops.md §8.1` / `§8.A` — the resident literals of the arms buy
/// path: the stock-call pool printed above the stock list, the two
/// shopkeeper-attribution tails, and the post-item "anything else" tail.
#[cfg(test)]
mod arms_shop_resident_literal_tests {
    use super::*;
    use crate::shop_runtime::ArmsShopOutcome;

    fn game_dir() -> &'static Path {
        Path::new("this-path-does-not-exist-so-no-SHOPPE.DAT-is-read")
    }

    /// `systems/shops.md §8.1` draw table under "The list is preceded by a
    /// heading line and one of four resident 'what we have' call lines chosen
    /// with a uniform `0..3` draw", published verbatim again in the `§8.A`
    /// row "Arms stock-call pool (verbatim)".
    #[test]
    fn arms_stock_call_pool_is_the_published_verbatim_four() {
        assert_eq!(arms_stock_call_for_roll(0), "What may I show thee?");
        assert_eq!(
            arms_stock_call_for_roll(1),
            "Which wouldst thou like to see?"
        );
        assert_eq!(arms_stock_call_for_roll(2), "What is thine interest?");
        assert_eq!(arms_stock_call_for_roll(3), "Which would ye see?");
    }

    /// The draw is uniform over `0..3`, so the pool wraps rather than
    /// panicking if a wider roll ever reaches it.
    #[test]
    fn arms_stock_call_pool_wraps_past_three() {
        for roll in 0u8..=u8::MAX {
            assert_eq!(
                arms_stock_call_for_roll(roll),
                arms_stock_call_for_roll(roll % 4)
            );
        }
    }

    /// Put a stocked arms shop in front of the player, already at its
    /// greeting, so the `B` key drives the real buy-entry render arm.
    fn stocked_arms_state() -> PlayState {
        use crate::shop_runtime::ArmsShopState;
        use crate::shop_session::ActiveShopSession;
        use crate::shops::ArmsStockTable;

        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 1, 1);
        state.gold = 1000;
        state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::Greeting,
            ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
        ));
        state
    }

    /// `systems/shops.md §8.1`: the call line is *printed once above the
    /// stock list*, chosen "with a uniform `0..3` draw".
    ///
    /// This drives the production render arm through `handle_play_key_input`
    /// instead of re-assembling the two halves in the test, so deleting the
    /// call line from that arm fails here. It also pins the draw to the live
    /// PRNG: the rendered call must be the one the shop's own next draw
    /// selects, not an arbitrary member of the pool.
    #[test]
    fn arms_buy_entry_prints_the_drawn_stock_call_above_the_list() {
        let mut state = stocked_arms_state();

        // Take the draw the buy-entry arm is about to make from a clone, so
        // the assertion knows which of the four lines is the correct one.
        let expected_call = arms_stock_call_for_roll(state.clone().random_range_u8(0, 3));

        handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();

        let mut lines = state.message.lines();
        assert_eq!(
            lines.next(),
            Some(expected_call),
            "the buy list must lead with the drawn call line: {:?}",
            state.message
        );
        assert!(
            lines
                .next()
                .is_some_and(|line| line.starts_with("We have:")),
            "the stock list must follow the call line: {:?}",
            state.message
        );
        assert!(state.message.contains("a) Short Sword"));
    }

    /// `systems/shops.md §8.1`: "Invalid buy selectors ... do not print a
    /// refusal line. The buy menu simply keeps waiting for a valid letter,
    /// Space, or Escape." `§8.A` adds that plain ignored-key waits "do not
    /// re-render the visible quote or menu, and do not consume a random bark
    /// draw" — so this redraw arm must not take a fresh call-line draw.
    ///
    /// The PRNG word is the observable: a re-drawn call line would advance it.
    #[test]
    fn arms_invalid_buy_letter_redraw_consumes_no_random_draw() {
        let mut state = stocked_arms_state();
        handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();

        let prng_after_entry = state.prng_state;
        let call_after_entry = state.message.lines().next().unwrap().to_string();

        // `d` is past the three-entry stock table, so it is an invalid buy
        // selector rather than a purchase.
        handle_play_key_input(&mut state, 'd', "", Path::new("")).unwrap();

        assert_eq!(
            state.prng_state, prng_after_entry,
            "the invalid-selector redraw must not consume a random draw"
        );
        assert!(
            !state.message.contains(&call_after_entry),
            "the redraw must not re-print the call line: {:?}",
            state.message
        );
        assert!(
            state.message.starts_with("We have:"),
            "the redraw re-renders the bare stock list: {:?}",
            state.message
        );
    }

    /// `systems/shops.md §8.1` / `§8.A`: the drawn no-credit bark is wrapped
    /// in the shopkeeper-attribution tail `yells <shopkeeper>.`
    #[test]
    fn arms_no_credit_bark_render_carries_the_yells_attribution_tail() {
        let speech = ArmsShopSpeech {
            shopkeeper: Some("Gwenneth"),
            speaker_is_female: false,
        };
        let rendered = format_arms_outcome_with_rolls(
            ArmsShopOutcome::BuyRefusedShortFunds {
                item: 16,
                quoted_price: 500,
            },
            game_dir(),
            None,
            Some(2),
            speech,
        );
        assert_eq!(rendered, "OUT, SLIME!\nyells Gwenneth.");
    }

    /// `systems/shops.md §8.1` / `§8.A` row "Arms carry-cap refusal
    /// (verbatim)": the fixed refusal is followed by the attribution tail
    /// `says <shopkeeper>.`
    #[test]
    fn arms_carry_cap_refusal_render_carries_the_says_attribution_tail() {
        let speech = ArmsShopSpeech {
            shopkeeper: Some("Kitiara"),
            speaker_is_female: false,
        };
        let rendered = format_arms_outcome_with_rolls(
            ArmsShopOutcome::BuyRefusedCapHit { item: 16 },
            game_dir(),
            None,
            None,
            speech,
        );
        assert_eq!(rendered, "Thou canst not carry any more!\nsays Kitiara.");
    }

    /// The two tails use different verbs; neither may borrow the other's.
    #[test]
    fn the_two_arms_attribution_tails_use_their_own_verbs() {
        let speech = ArmsShopSpeech {
            shopkeeper: Some("Max"),
            speaker_is_female: false,
        };
        assert_eq!(speech.attribute("Line.", "says"), "Line.\nsays Max.");
        assert_eq!(speech.attribute("Line.", "yells"), "Line.\nyells Max.");
    }

    /// With no published shopkeeper name for the live scene the resident line
    /// is printed unattributed rather than with an invented name.
    #[test]
    fn arms_attribution_tails_are_omitted_when_no_shopkeeper_is_published() {
        let speech = ArmsShopSpeech::default();
        assert_eq!(speech.shopkeeper, None);
        assert_eq!(
            format_arms_outcome_with_rolls(
                ArmsShopOutcome::BuyRefusedCapHit { item: 16 },
                game_dir(),
                None,
                None,
                speech,
            ),
            "Thou canst not carry any more!"
        );
        assert_eq!(
            format_arms_outcome_with_rolls(
                ArmsShopOutcome::BuyRefusedShortFunds {
                    item: 16,
                    quoted_price: 500,
                },
                game_dir(),
                None,
                Some(3),
                speech,
            ),
            "BEAT IT!"
        );
    }

    /// `systems/shops.md §8.1`: `Anything else,` closed by `milady?` for the
    /// female gender value, `sir?` otherwise, `then?` when no transaction has
    /// completed in this visit.
    #[test]
    fn arms_post_item_prompt_has_the_three_published_forms() {
        assert_eq!(arms_post_item_prompt(true, true), "Anything else, milady?");
        assert_eq!(arms_post_item_prompt(false, true), "Anything else, sir?");
        assert_eq!(arms_post_item_prompt(false, false), "Anything else, then?");
        assert_eq!(arms_post_item_prompt(true, false), "Anything else, then?");
    }

    /// `systems/shops.md §8.1`: a successful purchase "prints the fixed
    /// success line `Sold!`" and "then prints the post-item prompt".
    #[test]
    fn arms_successful_purchase_render_appends_the_post_item_prompt() {
        let rendered = format_arms_outcome_with_rolls(
            ArmsShopOutcome::Bought { item: 16, paid: 20 },
            game_dir(),
            None,
            None,
            ArmsShopSpeech::default(),
        );
        assert_eq!(rendered, "Sold!\nAnything else, sir?");
    }

    /// `systems/shops.md §8.0`: the shopkeeper filling the attribution tails
    /// is "a property of the location", read from the arms row of the vendor
    /// name table by the live scene byte. This pins that the render sites are
    /// wired to the real table rather than to a stub.
    #[test]
    fn arms_shopkeeper_name_comes_from_the_published_arms_vendor_row() {
        assert_eq!(arms_shopkeeper_name_for_scene(2), Some("Gwenneth"));
        assert_eq!(arms_shopkeeper_name_for_scene(24), Some("Kitiara"));
        assert_eq!(arms_shopkeeper_name_for_scene(32), Some("Thol"));
        // Scene `1` carries a tavern row but no arms row, so the arms lookup
        // must not fall through to another shop kind's vendor.
        assert_eq!(arms_shopkeeper_name_for_scene(1), None);
    }

    /// `systems/shops.md §8.1`: the post-item prompt prints `milady?` "when
    /// the speaking member's gender field is the female value". The gender
    /// field is `formats/saved-gam.md §3.1` record offset `0x09`, female
    /// value `0x0C`. `§8.A` states the arms tail "selects correctly" — in
    /// contrast to the shipwright's unreachable feminine form — so a female
    /// speaker must reach the feminine branch here.
    #[test]
    fn a_female_roster_speaker_reaches_the_arms_milady_tail() {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 1, 1);
        state.party_roster[0].gender = SAVE_GENDER_FEMALE_BYTE;
        state.active_player = Some(0);
        assert!(active_speaker_is_female(&state));

        let rendered = format_arms_outcome_with_rolls(
            ArmsShopOutcome::Bought { item: 16, paid: 20 },
            game_dir(),
            None,
            None,
            ArmsShopSpeech {
                shopkeeper: None,
                speaker_is_female: active_speaker_is_female(&state),
            },
        );
        assert_eq!(
            rendered,
            "Sold!
Anything else, milady?"
        );
    }

    /// The same wiring must still take the spec's "otherwise" branch for the
    /// male value `0x0B` (`formats/saved-gam.md §3.1`), so the change is a
    /// selection and not a blanket flip.
    #[test]
    fn a_male_roster_speaker_keeps_the_arms_sir_tail() {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 1, 1);
        state.party_roster[0].gender = SAVE_GENDER_MALE_BYTE;
        state.active_player = Some(0);
        assert!(!active_speaker_is_female(&state));

        let rendered = format_arms_outcome_with_rolls(
            ArmsShopOutcome::Bought { item: 16, paid: 20 },
            game_dir(),
            None,
            None,
            ArmsShopSpeech {
                shopkeeper: None,
                speaker_is_female: active_speaker_is_female(&state),
            },
        );
        assert_eq!(
            rendered,
            "Sold!
Anything else, sir?"
        );
    }

    /// `systems/shops.md §2`: the caller-context word is "the speaking party
    /// member's roster slot", so the tail follows the active member rather
    /// than always reading the leader.
    #[test]
    fn the_arms_tail_follows_the_active_speaker_slot_not_the_leader() {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 1, 1);
        let mut second = state.party_roster[0].clone();
        second.member.slot = 1;
        second.gender = SAVE_GENDER_FEMALE_BYTE;
        state.party_roster.push(second);
        state.party_roster[0].gender = SAVE_GENDER_MALE_BYTE;

        state.active_player = Some(0);
        assert!(!active_speaker_is_female(&state));
        state.active_player = Some(1);
        assert!(active_speaker_is_female(&state));
    }

    /// The wired lookup reaches the render site: a short-funds refusal in an
    /// arms scene carries that scene's published shopkeeper.
    #[test]
    fn arms_no_credit_bark_uses_the_scene_shopkeeper_end_to_end() {
        let speech = ArmsShopSpeech {
            shopkeeper: arms_shopkeeper_name_for_scene(17),
            speaker_is_female: false,
        };
        let rendered = format_arms_outcome_with_rolls(
            ArmsShopOutcome::BuyRefusedShortFunds {
                item: 16,
                quoted_price: 500,
            },
            game_dir(),
            None,
            Some(0),
            speech,
        );
        assert_eq!(rendered, "Can't pay?! Out with ye, orc-face!\nyells Max.");
    }
}
