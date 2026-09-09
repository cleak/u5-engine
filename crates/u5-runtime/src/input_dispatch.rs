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
    // `weather.md §5.1`, clear four: "After **every** outdoor command handler
    // returns, and before the next input is read, the loop clears the cache
    // unless the transport marker is still a hoisted frigate." This is that
    // loop position - the guard covers Furl, board, X-it, mounting a horse or
    // a carpet and anything else that moves the marker, and equally leaves a
    // marker-neutral Look or Ztats under sail alone.
    state.apply_outdoor_sail_cache_marker_guard();
    state.commit_command_echo();
    // `text-output.md §11`: whatever is still only in the slot is a line
    // the original would already have printed, so record it before the
    // next key can overwrite it.
    state.flush_message_slot();
    result
}

/// `timing.md §8.2` / `RETRACTIONS.md` R372: the cardinal direction a key
/// translates to, using the **translated code** path only.
///
/// The under-sail swallow tests "a key whose translated code equals the
/// cached sail heading", so it must read the same control codes the original
/// input helper compares, not this engine's additional letter aliases - those
/// collide with command verbs (`A`ttack, `S`earch, ...).
fn under_sail_translated_cardinal(key: char) -> Option<Direction> {
    let byte = input_byte_from_char(key)?;
    match input_code_direction(byte)? {
        InputDirection::North => Some(Direction::North),
        InputDirection::South => Some(Direction::South),
        InputDirection::East => Some(Direction::East),
        InputDirection::West => Some(Direction::West),
        _ => None,
    }
}

fn handle_play_key_input_inner(
    state: &mut PlayState,
    mut key: char,
    mut suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if let Some(byte) = input_byte_from_char(key) {
        if matches!(input_byte_class(byte), InputByteClass::FunctionKey) {
            // Measured 2026-09-07: a function key is just an unassigned
            // key, and answers the resident `What?` like any other.
            state.message = unassigned_refusal_echo(0).to_string();
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
        state.step_active_party_selector_with_game_dir(key, suffix, game_dir);
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
        .resolve_blackthorn_guard_demand_input(key, suffix, Some(game_dir))
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
    // `timing.md §8.2`, the under-sail pass's keyboard poll, and
    // `RETRACTIONS.md` R372: "a key whose translated code **equals the cached
    // sail heading is swallowed** - discarded, with the advance body running
    // anyway - which is why pressing the arrow you are already sailing does
    // nothing. Any other key ends the loop and becomes that turn's command."
    //
    // §8.2 calls the swallow "observable behaviour rather than a timing
    // detail", so discarding the key is only half of the clause: the advance
    // body has to run anyway or the key becomes a no-op the original never
    // has. The body is the auto-advance pass
    // (`PlayState::run_under_sail_advance_body`), and the swallowed key
    // completes the pass it was polled in - R371: such a pass "pays **one**"
    // tick instead of two, so it ends early rather than being duplicated.
    if suffix.is_empty() && state.under_sail_key_is_swallowed(under_sail_translated_cardinal(key)) {
        state.run_under_sail_advance_body(Some(game_dir))?;
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
    // `commands.md §9`: the other two shared Control bindings. Neither
    // consumes a turn, and both print one command-echo row.
    if key == PLAY_MORAL_STANDING_KEY {
        state.print_moral_standing();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_VERSION_BANNER_KEY {
        state.print_version_banner();
        return Ok(PlayInputDisposition::Continue);
    }
    // `commands.md` Section 9: Control + `E` "Prompts "Exit to DOS?"; a yes
    // answer leaves the game, anything else prints the refusal and continues",
    // and "None of the four consumes a turn in any mode". The prompt itself is
    // the shared yes/no session, so the answer arrives on the next dispatch and
    // the confirmed arm is what returns `Quit`.
    if key == PLAY_EXIT_TO_DOS_KEY {
        let _ = state.start_exit_to_dos_prompt();
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
        // `karma.md §8` (`RETRACTIONS.md` R451): the Codex reader is reached
        // by E-Enter on tile `0x11`, not by `M`.
        if state
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
    // `dungeon-mode.md` Section 10: "`Q` is the ordinary save-game route; the
    // "Exit to DOS?" prompt is a Control binding in the mode-local table, not
    // a letter." So the dungeon has no `Q` interception of its own; the letter
    // reaches the dungeon handler and takes the same save route every other
    // scene takes. Control + `E` owns the program exit.
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
            // `combat.md §8`, the `Y` row: nonempty combat Yell "reaches the
            // handler's no-effect path", whose literal is the shared one.
            let _ = PlayState::normalize_yell_word(&session.buffer);
            YELL_NO_EFFECT_MESSAGE.to_string()
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
                //
                // The prompt held the acting slot open so the paced round
                // walker could not start the next turn underneath it; the
                // action is now spent, so release it before advancing, the
                // same order the Cast/Use/Ready arms use.
                state.pending_combat_actor_slot = None;
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
    // `shops.md §8.C`: "The inn honorific below is `milady` for a female
    // speaking member, `sir` otherwise."
    let speaker_is_female = active_speaker_is_female(state);
    let area_scene_byte = match state.area {
        Area::Town { scene, .. } => Some(scene.byte),
        _ => None,
    };
    let inline_digit = suffix
        .chars()
        .find(|c| c.is_ascii_digit())
        .and_then(|c| c.to_digit(10).map(|d| d as u8))
        .or_else(|| key.to_digit(10).map(|d| d as u8));
    let inline_quantity = parse_active_shop_inline_quantity(key, suffix);
    let yes = matches!(key_byte, b'Y' | b'y') || suffix.chars().any(|c| matches!(c, 'Y' | 'y'));
    let no = matches!(key_byte, b'N' | b'n') || suffix.chars().any(|c| matches!(c, 'N' | 'n'));
    let mut replacement_session: Option<ActiveShopSession> = None;
    // Measured 2026-09-07/08 at all eight overlays: the answer to the entry
    // question echoes as a word - `Yes` or `No` - onto the row the greeting
    // left the cursor on. A Paws capture shows it appended to the greeting's
    // own last row (`thee?" Yes`); other captures show it a row lower, which
    // is the same rule when the greeting happens to fill its last row.
    let entry_answer_echo = session.awaiting_entry_answer().then(|| {
        if yes {
            Some("Yes")
        } else if no {
            Some("No")
        } else {
            None
        }
    });

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
        ActiveShopSession::Healer(s, healer) => {
            let healer_name = match area_scene_byte {
                Some(scene) => {
                    crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_HEALER, scene)
                }
                None => None,
            };
            match (*s, yes, no, inline_digit) {
                (HealerShopState::Greeting, _, true, _) => {
                    *s = HealerShopState::Exited;
                    "Farewell.".to_string()
                }
                (HealerShopState::Greeting, true, _, _) => {
                    *s = HealerShopState::PickService;
                    healer_service_menu(healer_name)
                }
                (HealerShopState::Greeting, _, _, _)
                    if matches!(key_byte, b'H' | b'h' | b'Y' | b'y') =>
                {
                    *s = HealerShopState::PickService;
                    healer_service_menu(healer_name)
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
                        // **Measured** 2026-09-09 at Cove's Sanctuary
                        // (`qa/paired/cove-healer.tsv`, beat `cure`): the
                        // accepted service letter echoes a word onto the row
                        // §8.B's question left open - `need?" Curing` - and a
                        // one-member party is then treated without a prompt
                        // of any kind: the refusal followed the echo directly.
                        //
                        // `shops.md §8` says "Each mode prompts for a party
                        // member first"; with one member there is nobody to
                        // choose between, and the engine asked anyway with an
                        // invented `Who needs Cure? (1-6)`.
                        state.emit_message_line_continuing_row(healer_service_echo(treatment));
                        state.push_explicit_blank_message_entry();
                        if state.party.len() == 1 {
                            *s = HealerShopState::PickPartyMember { service, cost };
                            return {
                                state.active_shop = Some(session);
                                handle_active_shop_key_input(state, '1', "", game_dir)
                            };
                        }
                        *s = HealerShopState::PickPartyMember { service, cost };
                        // §8.C publishes this after all: "A sole party member
                        // is selected automatically; otherwise print
                        // `\n\n"Who needs my aid?" ` and use the party
                        // selector."
                        HEALER_WHO_NEEDS_AID.to_string()
                    }
                    HealerServiceAction::Exit => {
                        *s = HealerShopState::Exited;
                        "Farewell.".to_string()
                    }
                    HealerServiceAction::Discard => healer_service_menu(healer_name),
                },
                (HealerShopState::PickPartyMember { service, .. }, _, true, _) => {
                    *s = HealerShopState::PickService;
                    let _ = service;
                    // §8.C: "Cancellation adds `No one`, with no trailing line
                    // feed", then the visit continues on its own question.
                    format!("No one{HEALER_CONTINUATION}")
                }
                (HealerShopState::PickPartyMember { service, .. }, _, _, Some(d)) if d >= 1 => {
                    let target_index = usize::from(d - 1);
                    let treatment = healer_treatment_for_service(service);
                    let Some(member) = state.party.get(target_index).copied() else {
                        return {
                            state.active_shop = Some(session);
                            // An out-of-range healer target: the shop's own
                            // picker bounds it, so this is an invariant guard.
                            state.message.clear();
                            PlayInputDisposition::Continue
                        };
                    };
                    if !active_healer_target_accepts(treatment, member) {
                        *s = HealerShopState::PickService;
                        healer_no_need_refusal(healer_name)
                    } else {
                        match healer_treatment_fee(*healer, treatment) {
                            HealerTreatmentFee::Bypass => {
                                let message = match state.buy_healer_treatment(
                                    *healer,
                                    treatment,
                                    target_index,
                                ) {
                                    Ok(outcome) => format_healer_treatment_outcome(outcome),
                                    Err(err) => format_healer_treatment_error(err, healer_name),
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
                                healer_fee_and_confirmation(treatment, cost)
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
                            Err(crate::shops::HealerTreatmentError::InsufficientGold {
                                ..
                            }) => {
                                format!("Thou lackest the {cost} gold.")
                            }
                            Err(err) => format_healer_treatment_error(err, healer_name),
                        };
                    *s = HealerShopState::PickService;
                    message
                }
                (HealerShopState::Confirm { service, .. }, _, true, _) => {
                    *s = HealerShopState::PickService;
                    let _ = service;
                    // §8.C: the fee prompt echoes a bare `No`, and the visit
                    // continues on its own question.
                    format!("No{HEALER_CONTINUATION}")
                }
                (HealerShopState::Exited, _, _, _) => "Farewell.".to_string(),
                // `shops.md §8.B`/`§8.C`: every shop kind ignores a key it does
                // not recognise - "letters outside the displayed stock count
                // are ignored without a refusal bark", "unknown menu letters
                // silently wait; there is no I-do-not-understand line",
                // "other keys wait". None of them answers.
                _ => String::new(),
            }
        }
        ActiveShopSession::Innkeeper(s) => {
            let scene_marker = active_inn_scene_marker(state);
            let innkeeper_name = area_scene_byte.and_then(|scene| {
                crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_INN, scene)
            });
            match (*s, yes, no, inline_digit) {
                // `shops.md §8`'s entry table, row `0x88`: the greeting takes
                // `Y`, `N` or Space; Yes opens the service question and every
                // other key leaves the greeting visible. `§8.B` publishes the
                // question itself, which the engine had no state for - it read
                // the entry key as a service letter instead.
                (InnkeeperState::Greeting { inn }, true, _, _) => {
                    *s = InnkeeperState::ServiceMenu { inn };
                    match innkeeper_name {
                        Some(name) => format!("{name} asks,\n{INN_SERVICE_QUESTION}"),
                        None => INN_SERVICE_QUESTION.to_string(),
                    }
                }
                (InnkeeperState::Greeting { .. }, _, true, _) => {
                    *s = InnkeeperState::Exited;
                    String::new()
                }
                (InnkeeperState::Greeting { inn }, _, _, _) => {
                    if matches!(key_byte, b' ' | b'\r' | b'\n' | 0x1b) {
                        *s = InnkeeperState::Exited;
                        String::new()
                    } else {
                        // "Other initial keys leave the existing greeting
                        // visible and re-poll", without redrawing it.
                        *s = InnkeeperState::Greeting { inn };
                        state.message.clone()
                    }
                }
                (InnkeeperState::ServiceMenu { inn }, _, _, _) => match inn_main_action(key_byte) {
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
                        // **Measured** 2026-09-08 at Hotel Brittany
                        // (`qa/paired/nb-inn-branches.tsv`): the quote ends on
                        // the shipwright's prompt, quoted and without `(Y/N)`.
                        // The room's own description is inn text this spec
                        // does not publish, so the engine's sentence stands
                        // above it for now.
                        format!(
                            "{} room and board costs {total_price} gold.\n\n{INN_CONFIRM_PROMPT}",
                            inn.display_name()
                        )
                    }
                    InnMainAction::LeaveCompanion => {
                        // **Measured** 2026-09-08 at Hotel Brittany
                        // (`qa/paired/nb-inn-refusals.tsv`): asking to leave a
                        // companion with nobody to leave draws a refusal that
                        // ends the visit, and it carries no attribution tail.
                        // The engine printed `Thou must keep at least one
                        // companion.` and stayed in the menu.
                        if ctx.party_size <= 1 {
                            *s = InnkeeperState::Exited;
                            INN_NOBODY_TO_LEAVE_REFUSAL.to_string()
                        } else {
                            let deposit = inn_leave_companion_deposit_for_speaker(
                                inn,
                                ctx.speaker_intelligence,
                            );
                            *s = InnkeeperState::PickLeaveCompanion { inn, deposit };
                            // **Measured** 2026-09-08 at Hotel Brittany with a
                            // party of three (`qa/paired/nb-inn-companion.tsv`):
                            // the innkeeper asks in the message window while
                            // the *panel* becomes a framed `Select:` picker
                            // over the party rows. No deposit is quoted here.
                            match innkeeper_name {
                                Some(name) => {
                                    format!("{name} asks,\n{INN_WHO_WILL_STAY_PROMPT}")
                                }
                                None => INN_WHO_WILL_STAY_PROMPT.to_string(),
                            }
                        }
                    }
                    InnMainAction::PickUpCompanion => {
                        let base_room_rate = inn_base_room_rate(inn);
                        let guests = inn_guest_indices_for_scene(&state.inn_registry, scene_marker);
                        if guests.is_empty() {
                            *s = InnkeeperState::ServiceMenu { inn };
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
                            // **Measured** 2026-09-08 at Hotel Brittany
                            // (`qa/paired/nb-inn-pickup.tsv`): collecting the
                            // only lodged companion charges the bill straight
                            // away and returns them - there is no `(Y/N)`
                            // confirmation and no picker. Gold fell 150 -> 117
                            // on the keypress and the panel grew back to three
                            // rows.
                            let _ = base_room_rate;
                            let result = state.pickup_inn_guest_with_bill(
                                scene_marker,
                                registry_index,
                                bill,
                            );
                            *s = InnkeeperState::ServiceMenu { inn };
                            match result {
                                Ok(outcome) => {
                                    let surcharge = apply_active_shop_surcharge(state);
                                    // The poisoned-guest death note is this
                                    // engine's own: the measured capture
                                    // collected a living companion, and a
                                    // player who is told nothing would not
                                    // learn of the death until the panel is
                                    // read. Kept, and marked, until measured.
                                    let death_note = if outcome.returned_dead_from_poison {
                                        "\nThy friend has died, by the way."
                                    } else {
                                        ""
                                    };
                                    let message = match innkeeper_name {
                                        Some(name) => format!(
                                            "\"That will be {bill} gold, please.\"{death_note}\n\n{INN_STAY_ENJOYABLE_LINE}\nsays {name}.\n\n{INN_ANYTHING_MORE_PROMPT}"
                                        ),
                                        None => format!(
                                            "\"That will be {bill} gold, please.\"{death_note}\n\n{INN_STAY_ENJOYABLE_LINE}\n\n{INN_ANYTHING_MORE_PROMPT}"
                                        ),
                                    };
                                    append_active_shop_surcharge(message, surcharge)
                                }
                                Err(err) => format_inn_error(err, speaker_is_female),
                            }
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
                    *s = InnkeeperState::ServiceMenu { inn };
                    match result {
                        Ok(outcome) => {
                            let message =
                                apply_paid_inn_rest(state, inn, outcome.quote.total_price);
                            let surcharge = apply_active_shop_surcharge(state);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Err(err) => format_inn_error(err, speaker_is_female),
                    }
                }
                (InnkeeperState::ConfirmRest { inn, .. }, _, true, _) => {
                    // Measured: a declined room ends the visit on the
                    // innkeeper's own attributed line, not a return to the
                    // branch question.
                    let _ = inn;
                    *s = InnkeeperState::Exited;
                    inn_declined_line(innkeeper_name)
                }
                (InnkeeperState::PickLeaveCompanion { inn, deposit: _ }, _, true, _) => {
                    *s = InnkeeperState::ServiceMenu { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::PickLeaveCompanion { inn, deposit }, _, _, Some(d)) if d >= 1 => {
                    let party_index = usize::from(d - 1);
                    *s = InnkeeperState::ConfirmLeaveCompanion {
                        inn,
                        party_index,
                        deposit,
                    };
                    // **Measured** 2026-09-08 (`qa/paired/nb-inn-commit.tsv`):
                    // committing the panel selection quotes the deposit as a
                    // monthly rate due at check-out, and ends on the same
                    // prompt the room quote uses.
                    format!(
                        "{} comfortable room will be {deposit} gold per month, due at check-out.\n\n{INN_CONFIRM_PROMPT}",
                        inn.display_name()
                    )
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
                    *s = InnkeeperState::ServiceMenu { inn };
                    match result {
                        Ok(outcome) => {
                            // Measured: the innkeeper thanks the party and
                            // then asks his own follow-up question; the engine
                            // reported the slot and the deposit instead.
                            let _ = outcome;
                            let message = match innkeeper_name {
                                Some(name) => format!(
                                    "{INN_THANKS_LINE}\nsays {name}.\n\n{INN_ANYTHING_MORE_PROMPT}"
                                ),
                                None => format!("{INN_THANKS_LINE}\n\n{INN_ANYTHING_MORE_PROMPT}"),
                            };
                            let surcharge = apply_active_shop_surcharge(state);
                            append_active_shop_surcharge(message, surcharge)
                        }
                        Err(err) => format_inn_error(err, speaker_is_female),
                    }
                }
                (InnkeeperState::ConfirmLeaveCompanion { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::ServiceMenu { inn };
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
                    *s = InnkeeperState::ServiceMenu { inn };
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
                    *s = InnkeeperState::ServiceMenu { inn };
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
                        Err(err) => format_inn_error(err, speaker_is_female),
                    }
                }
                (InnkeeperState::ConfirmPickUpCompanion { inn, .. }, _, true, _) => {
                    *s = InnkeeperState::ServiceMenu { inn };
                    "As you wish.".to_string()
                }
                (InnkeeperState::Exited, _, _, _) => "Farewell.".to_string(),
                // `shops.md §8.B`/`§8.C`: every shop kind ignores a key it does
                // not recognise - "letters outside the displayed stock count
                // are ignored without a refusal bark", "unknown menu letters
                // silently wait; there is no I-do-not-understand line",
                // "other keys wait". None of them answers.
                _ => String::new(),
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
                    match state.area {
                        Area::Town { scene, .. } => {
                            crate::play_state_impl::shop_vendor_name_for_scene(
                                SHOP_DIALOG_ID_TAVERN,
                                scene.byte,
                            )
                        }
                        _ => None,
                    },
                    active_speaker_is_female(state),
                    game_dir,
                ),
                surcharge,
            )
        }
        ActiveShopSession::Sage(s) => {
            let outcome = match *s {
                SageState::Prompt { .. } => {
                    let line = active_shop_text_line(key, suffix);
                    // `shops.md §8.C`, the sage result table: the topic prompt
                    // ends `You respond:\n`, and "After typed topic | `\n\n`".
                    // The typed word therefore occupies the row that prompt
                    // opened, and two line feeds follow it.
                    //
                    // The word is only ever a live row in this engine - the
                    // shell's own input buffer - so it never reached the
                    // transcript, and the sage's quote printed two rows high.
                    // Same shape as the Blackthorn answer row.
                    if !line.trim().is_empty() {
                        // The prompt ends `You respond:\n`, so the typed word
                        // opens the row below it rather than continuing that
                        // one, and it echoes upper-cased - measured at the Paws
                        // sage, which reads back `You respond:` / `SPIR` for a
                        // topic typed lower-case.
                        state.emit_message_line(line.to_uppercase());
                        state.push_explicit_blank_message_entry();
                    }
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
            let mut message = format_sage_outcome_with_shoppe(outcome, game_dir);
            if paid {
                // Measured 2026-09-08 at The Cat's Lair
                // (`qa/paired/paws-sage.tsv`): the rumour is attributed to the
                // *tavern's* vendor - the sage is reached through the tavern
                // and never speaks under a name of its own - and the visit
                // then returns to the tavern's own anything-else question
                // rather than waiting for another topic.
                let tavern_vendor = area_scene_byte.and_then(|scene| {
                    crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_TAVERN, scene)
                });
                if let Some(name) = tavern_vendor {
                    message.push_str(&format!("\nsays {name}."));
                }
                if let Some(tavern) =
                    area_scene_byte.and_then(crate::shop_session::tavern_for_scene)
                {
                    message.push_str(&format!("\n\n{TAVERN_ANYTHING_ELSE_PROMPT}"));
                    replacement_session = Some(ActiveShopSession::Tavern(
                        crate::shop_runtime::TavernState::AnythingElse {
                            tavern,
                            continuation_ready: true,
                        },
                    ));
                }
            }
            let surcharge = if paid {
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            append_active_shop_surcharge(message, surcharge)
        }
        ActiveShopSession::Reagent(s) => {
            // The declined-quote arm redraws this herbalist's list, so the
            // formatter needs to know whose shop it is even when the outcome
            // does not carry it.
            let declined_herbalist = match *s {
                ReagentShopState::Greeting { herbalist }
                | ReagentShopState::PickReagent { herbalist }
                | ReagentShopState::PickQuantity { herbalist, .. } => Some(herbalist),
                ReagentShopState::Exited => None,
            };
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
            format_reagent_outcome(outcome, declined_herbalist)
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
            // Measured 2026-09-08 at North Britanny's stable at noon
            // (`qa/paired/nb-stable-open.tsv`): the accepted purchase debits
            // the gold and ends the visit on the shop's farewell bark,
            // attributed - `"Fare thee well!"` over `says Theoan.` - with no
            // success line of its own. The engine printed
            // `Sold for 143 gold. Thy horse awaits outside.`
            let message = match outcome {
                HorseTraderOutcome::Purchased { .. } => {
                    let vendor = area_scene_byte.and_then(|scene| {
                        crate::play_state_impl::shop_vendor_name_for_scene(
                            SHOP_DIALOG_ID_STABLE,
                            scene,
                        )
                    });
                    let roll = state.random_range_u8(0, 3);
                    let record = crate::shoppe_records::shared_shop_bark_record(
                        SHOP_DIALOG_ID_STABLE,
                        crate::shoppe_records::SharedShopBarkKind::Farewell,
                        roll,
                    );
                    let flourish = record
                        .and_then(|record_id| render_shared_shoppe_flourish(game_dir, record_id));
                    match (flourish, vendor) {
                        (Some(line), Some(name)) => format!("{line}\nsays {name}."),
                        (Some(line), None) => line,
                        (None, _) => format_horse_trader_outcome(outcome),
                    }
                }
                outcome => format_horse_trader_outcome(outcome),
            };
            append_active_shop_surcharge(message, surcharge)
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
            let vendor_name = area_scene_byte.and_then(|scene| {
                crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_SHIPWRIGHT, scene)
            });
            // `§8.B`: the Frigate/Skiff menu is record `119`, "with no added
            // resident menu question; the record carries its own leading
            // spacing and prompt".
            let menu_record = crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
                .ok()
                .and_then(|renderer| {
                    renderer
                        .render_record(
                            crate::shops::SHOPPE_RECORD_SHIPWRIGHT_MENU,
                            &crate::shoppe_bark::ShoppeBarkContext {
                                vendor_name: vendor_name.unwrap_or_default(),
                                hour: state.clock.hour,
                                ..Default::default()
                            },
                        )
                        .ok()
                })
                .filter(|text| !text.trim().is_empty());
            append_active_shop_surcharge(
                format_ship_broker_outcome(outcome, vendor_name, menu_record),
                surcharge,
            )
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
    if let Some(Some(echo)) = entry_answer_echo {
        // `shops.md §8.B`: "Where a colon is emitted, the accepted answer
        // appends directly after it, producing `:Yes` or `:No`; do not insert
        // an intervening space. A greeting ending in columns 1 through 11
        // instead receives the single-space continuation shown above." The
        // greeting already placed that space or that colon, so the echo is
        // the bare word either way; the engine used to supply a space of its
        // own and so double-spaced the tavern's row and put the healer's
        // answer on a row with no colon.
        state.emit_message_line_continuing_row(echo);
        if echo == "Yes" {
            // `§8.B`'s per-kind table gives every Yes branch `Yes\n\n` before
            // its service text.
            state.push_explicit_blank_message_entry();
        }
    }
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

const SHOP_DIALOG_ID_HEALER: u8 = 0x87;

/// **Measured** 2026-09-07 at Cove's Sanctuary (`qa/paired/cove-healer.tsv`):
/// answering the greeting prints the powers line, attributed to the healer,
/// then the service question a blank row below.
///
/// ```text
/// "We have powers
/// to Cure, Heal,
/// or Resurrect."
/// says Jessica.
///
/// "What is the
/// nature of thy
/// need?"
/// ```
///
/// The engine asked `Cure (C), Heal (H), or Resurrect (R)?`, which is not a
/// line the original prints.
fn healer_service_menu(name: Option<&'static str>) -> String {
    let powers = "\"We have powers to Cure, Heal, or Resurrect.\"";
    match name {
        Some(name) => format!("{powers}\nsays {name}.\n\n{HEALER_SERVICE_QUESTION}"),
        None => format!("{powers}\n\n{HEALER_SERVICE_QUESTION}"),
    }
}

/// The question the service menu ends on, and the one a completed or refused
/// treatment returns to.
const HEALER_SERVICE_QUESTION: &str = "\"What is the nature of thy need?\"";

/// `shops.md §8.C`'s healer result table: "Selected member is untreatable |
/// Token-expanded `\n\n"Thou hast no need of this art!"\nsays $.`", then
/// separately "Continuation after treatment, refusal, or member cancellation |
/// `\n\n"Is there any other way in which I may\n`, then `aid thee?" `".
fn healer_no_need_refusal(name: Option<&'static str>) -> String {
    let refusal = "\"Thou hast no need of this art!\"";
    match name {
        Some(name) => format!("{refusal}\nsays {name}.{HEALER_CONTINUATION}"),
        None => format!("{refusal}{HEALER_CONTINUATION}"),
    }
}

/// §8.C: the continuation the visit resumes on. Its own leading line feeds
/// and its trailing space are the published text's.
const HEALER_CONTINUATION: &str = "\n\n\"Is there any other way in which I may\naid thee?\" ";

/// §8.C's three paid introductions. The fee row completes each of them.
fn healer_paid_introduction(treatment: crate::shops::HealerTreatment) -> &'static str {
    match treatment {
        crate::shops::HealerTreatment::Cure => "\"I can cure thy poisoned body ",
        crate::shops::HealerTreatment::Heal => "\"I can heal thee ",
        crate::shops::HealerTreatment::Resurrect => {
            "\"I can raise this unfortunate person from the dead "
        }
    }
}

/// §8.C: "Fee and confirmation, after an introduction | Token-expanded
/// `for % gold.\n\nWilt thou\npay?" `; accept Y/N only and echo bare `Yes`
/// or `No`". Measured at Cove's Sanctuary
/// (`qa/paired/cove-healer-services.tsv`, beat `heal`), which reads back
/// `"I can heal thee` / `for 55 gold.` / blank / `Wilt thou` / `pay?"`.
fn healer_fee_and_confirmation(treatment: crate::shops::HealerTreatment, cost: u16) -> String {
    format!(
        "{}for {cost} gold.\n\nWilt thou\npay?\" ",
        healer_paid_introduction(treatment)
    )
}

/// §8.C: "A sole party member is selected automatically; otherwise print
/// `\n\n"Who needs my aid?" ` and use the party selector."
const HEALER_WHO_NEEDS_AID: &str = "\n\n\"Who needs my aid?\" ";

/// `shops.md §8.B`, the innkeeper's row: "`Yes`, then token-expanded
/// `\n\n$ asks,\n"Art thou here\nto Pick up or\n`, then `Leave a\ncompanion,
/// or\nto Rest for the\nnight?" `". The line breaks are the resident text's
/// own.
const INN_SERVICE_QUESTION: &str =
    "\"Art thou here\nto Pick up or\nLeave a\ncompanion, or\nto Rest for the\nnight?\" ";

/// **Measured** at Cove's Sanctuary: the accepted service letter echoes a
/// word onto the open question row (`need?" Curing`). Only Cure is measured;
/// `qa/paired/cove-healer-services.tsv` drives the other two.
fn healer_service_echo(treatment: crate::shops::HealerTreatment) -> &'static str {
    match treatment {
        crate::shops::HealerTreatment::Cure => "Curing",
        crate::shops::HealerTreatment::Heal => "Healing",
        crate::shops::HealerTreatment::Resurrect => "Resurrecting",
    }
}

/// `shops.md §8.B`'s arms entry, stage 1: "Welcome | `"Good @, and welcome to
/// #!"\n`, with token expansion" - `@` the time of day, `#` the shop name.
pub(crate) fn arms_welcome_line(hour: u8, shop_name: &str) -> String {
    // `§8.0`'s Talk entry newline opens the row the welcome prints on, the
    // same way it does for the seven shared-greeting kinds.
    format!(
        "\n\"Good {}, and welcome to {shop_name}!\"\n",
        crate::shops::shoppe_time_of_day_word(hour)
    )
}

/// §8.B's two equal-probability arms greetings, and the stages around them:
/// "Attribution | `\n$ says,\n"`", the selected variant, then "Tail after the
/// selected variant | `" ` - a closing quote and one space".
pub(crate) const ARMS_GREETINGS: [&str; 2] = [
    "Hail, friend! Wouldst thou Buy or Sell?",
    "Greetings, traveller! Wish ye to Buy, or hast thou wares to Sell?",
];

pub(crate) fn arms_attribution_and_greeting(vendor_name: Option<&str>, variant_roll: u8) -> String {
    let greeting = ARMS_GREETINGS[usize::from(variant_roll & 1)];
    match vendor_name {
        Some(vendor) => format!("\n{vendor} says,\n\"{greeting}\" "),
        // The attribution carries the vendor token; with no vendor resolved
        // the quote still opens, which is what the tail closes.
        None => format!("\n\"{greeting}\" "),
    }
}

const SHOP_DIALOG_ID_TAVERN: u8 = 0x82;
const SHOP_DIALOG_ID_STABLE: u8 = 0x83;
const SHOP_DIALOG_ID_SHIPWRIGHT: u8 = 0x84;
const SHOP_DIALOG_ID_INN: u8 = 0x88;

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

/// `shops.md §8.4` inn `R` (Rest for the night).
///
/// "Three world-state effects come with it, and an engine that treats the
/// rest as a pure presentation will miss all three. The party's map position
/// is written to the inn's bed cell for the duration ... The clock is then
/// run forward in paced steps until the hour byte reads **six** ... On
/// completion the location runs the same clear-and-re-place pass that town
/// entry runs: every non-party active-object record is cleared, and every
/// scheduled NPC is re-placed at the position its schedule gives for the new
/// hour (`systems/npc-schedules.md` Section 12). That is why the town's
/// residents are in their morning positions, not their overnight ones, the
/// moment the party wakes."
///
/// `main-loop.md §9` makes the reclassification explicit: "a paced sequence
/// of *real turns* - sailing into the wind, crossing difficult terrain,
/// holing up, resting at an inn - is not a presentation at all, however much
/// it looks like one from the outside."
///
/// The fixed eight-hour advance this replaces both ended at the wrong hour
/// and left the roster wherever the walker had taken it.
fn apply_paid_inn_rest(state: &mut PlayState, inn: crate::shops::Inn, cost: u16) -> String {
    // `shops.md §8.4` (issue #190), the first of the three world-state
    // effects: "The party's map position is written to the inn's bed cell
    // for the duration, so the party is standing on the bed while the
    // sequence plays." Which cell was the open question this handler
    // carried; §8.4 now publishes the two parallel six-entry tables, so
    // the cell is looked up rather than derived - "there is no rule to
    // derive these from the map or from the shop cell; they are authored
    // data and must be carried as data".
    //
    // "**The floor byte is not written.** The rest happens on whatever
    // floor the inn menu was opened from. Do not reset the party to the
    // entry floor as part of the rest." So only X and Y move here; the
    // `Area`'s floor is left exactly as it was.
    //
    // This is on the **completed** path only. §8.4: "All three of the
    // handler's early exits - the pre-menu helper declining, an answer
    // other than `Y` at the confirmation prompt, and gold below the quoted
    // charge - return without moving the party at all." Each of those
    // returns before reaching this function.
    let (bed_x, bed_y) = inn.bed_cell();
    state.player.x = usize::from(bed_x);
    state.player.y = usize::from(bed_y);
    state.sync_player_object();
    state.mark_town_rest_sleepers();
    let hours = state.advance_inn_rest_clock_to_morning();
    let woke = state.wake_town_rest_sleepers();
    let (recovered_hp, recovered_mana, cured) = state.apply_inn_rest_night_recovery();
    state.clear_and_replace_scheduled_npcs();
    // `shops.md §8.4`: "**The party does not wake in the bed.** On the
    // completed-rest path the handler steps the party **one tile east** of
    // the bed cell before returning."
    //
    // §8.4 is explicit that it does not claim the bed cell or its eastern
    // neighbour is walkable on the shipped pages, so no walkability test
    // is applied here either: "The coordinates are the ones the rest
    // handler writes, which is what an implementation needs."
    let (wake_x, wake_y) = inn.bed_wake_cell();
    state.player.x = usize::from(wake_x);
    state.player.y = usize::from(wake_y);
    state.sync_player_object();
    state.mark_visibility_dirty();
    format!(
        "Rested {hours} hours at the inn for {cost} gold; recovered {recovered_hp} HP and {recovered_mana} MP; cured {cured} poisoned member(s); woke {woke} asleep member(s)."
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
    // `shops.md §8.B` stage 2: "Pause | Wait for one key before the next
    // text", then stage 3's attribution and the selected greeting. The
    // greeting is one of two equal-probability variants, so this is where that
    // draw is spent.
    if matches!(prior_state, ArmsShopState::Welcome) {
        *shop_state = ArmsShopState::Greeting;
        let vendor = match state.area {
            Area::Town { scene, .. } => crate::play_state_impl::shop_vendor_name_for_scene(
                crate::shoppe_records::SHOP_DIALOG_ID_ARMS,
                scene.byte,
            ),
            _ => None,
        };
        let roll = state.random_range_u8(0, 1);
        return arms_attribution_and_greeting(vendor, roll);
    }
    let outcome = match (prior_state, yes, no, inline_digit) {
        (ArmsShopState::Welcome | ArmsShopState::Greeting, _, _, _) => step_arms_shop(
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
            // Measured order: heading, the lettered stock rows, a blank row,
            // then the call line. `§8.B` fixes the two heading draws and
            // their order - affirmation first, then stock introduction - and
            // both precede `§8.1`'s call-line draw, which prints after the
            // list.
            let listing = format_arms_stock_buy_menu(
                table,
                Some((state.random_range_u8(0, 3), state.random_range_u8(0, 3))),
            );
            let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
            format!("{listing}\n{call}")
        }
        (ArmsShopOutcome::InvalidInput, Some(table)) if was_invalid_stock_pick => {
            // `§8.1`: an invalid stock letter "leave[s] the stock list visible
            // and keep[s] waiting; they do not redraw the list or consume a
            // random draw", so the redraw reuses no fresh heading draw.
            format_arms_stock_buy_menu(table, None)
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
            // Measured 2026-09-07 (`qa/paired/shop-arms-sell.tsv`): the offer
            // record, a blank row, then `Deal?"` - the closing quote of the
            // shopkeeper's speech, and no `(Y/N)`.
            format!("{quote}\n\nDeal?\"")
        }
        (ArmsShopOutcome::SellRefusedZeroPrice { .. }, _)
            if matches!(shop_state, ArmsShopState::SellPickItem(_)) =>
        {
            // **Measured** 2026-09-07 (`qa/paired/shop-arms-sell-zero-price.tsv`):
            // quoted and attributed with `says` - unlike the ammunition
            // refusal's `growls` - and the browser keeps going with its
            // continuation prompt.
            format!(
                "{}\n{}",
                speech.attribute("\"That, I cannot buy from thee.\"", "says"),
                arms_sell_continuation_prompt(state.random_range_u8(0, 3))
            )
        }
        // Measured: a completed purchase and a declined quote both leave the
        // Buy listing on screen with a freshly drawn call line under it, and
        // the next letter buys again without a second `B`.
        (ArmsShopOutcome::Bought { .. }, Some(table)) => {
            let post = arms_post_item_prompt(speech.speaker_is_female, true);
            let listing = format_arms_stock_buy_menu(table, None);
            let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
            format!("Sold!\n{post}\n\n{listing}\n{call}")
        }
        (ArmsShopOutcome::Declined, Some(table))
            if matches!(prior_state, ArmsShopState::BuyConfirm { .. }) =>
        {
            let listing = format_arms_stock_buy_menu(table, None);
            let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
            format!("{listing}\n{call}")
        }
        (ArmsShopOutcome::Declined, _) if matches!(shop_state, ArmsShopState::SellPickItem(_)) => {
            format!(
                "No\n{}",
                arms_sell_continuation_prompt(state.random_range_u8(0, 3))
            )
        }
        // Measured 2026-09-07 (`qa/paired/shop-arms-sell-flow.tsv`): a sale
        // through the browser answers `"Done!"` over `says <shopkeeper>.`,
        // not the buy path's fixed `Sold!` of `shops.md` §8.1.
        (
            ArmsShopOutcome::Sold {
                browser_continues: true,
                ..
            },
            _,
        ) => format!(
            "{}\n{}",
            speech.attribute("\"Done!\"", "says"),
            arms_sell_continuation_prompt(state.random_range_u8(0, 3))
        ),
        (
            ArmsShopOutcome::Sold {
                browser_continues: false,
                ..
            },
            _,
        ) => format!(
            "{}\n{}",
            speech.attribute("\"Done!\"", "says"),
            speech.attribute(&arms_sell_goodbye(state.random_range_u8(0, 3)), "says")
        ),
        (ArmsShopOutcome::Exited, _) if matches!(prior_state, ArmsShopState::SellPickItem(_)) => {
            speech.attribute(&arms_sell_goodbye(state.random_range_u8(0, 3)), "says")
        }
        // Measured 2026-09-07 (`qa/paired/shop-arms-buy.tsv`): a visit that
        // ends without a sale closes on a *rendered* shared-band record, not
        // a resident goodbye - `"Thanks for nothing!" says Gwenneth` and
        // `"Harrumph!" says Gwenneth` are records 0 and 2, found with the
        // `shoppe_find` probe. `shops.md` §8.A's uniform `0..3` draw picks
        // between them, so the draw is the record id for this shop's row.
        (ArmsShopOutcome::Exited | ArmsShopOutcome::Declined, _) => {
            let roll = state.random_range_u8(0, 3);
            render_shared_shoppe_flourish(game_dir, usize::from(roll))
                .map(|flourish| speech.attribute(&flourish, "says"))
                .unwrap_or_default()
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

/// **Measured** 2026-09-07 (`qa/paired/shop-arms-menus.tsv`,
/// `shop-arms-sell.tsv`): the sell-entry prompt is *quoted* -
/// `"Which item wouldst thou like to sell?"` and
/// `"What dost thou wish to sell?"` both came back with an opening and a
/// closing double quote. The other two lines of the pool did not draw in
/// either capture and take the same shape.
fn arms_sell_entry_prompt(roll: u8) -> String {
    [
        "\"Which item wouldst thou like to sell?\"",
        "\"What dost thou wish to sell?\"",
        "\"Show me what ye got...\"",
        "\"What dost thou have for me to buy?\"",
    ][usize::from(roll) % 4]
        .to_string()
}

/// **Measured** 2026-09-07 (`qa/paired/shop-arms-sell-flow.tsv`): quoted, like
/// the entry pool - `"What else hath ye to sell?"` drew twice in one visit.
fn arms_sell_continuation_prompt(roll: u8) -> String {
    [
        "\"What else can ye offer me?\"",
        "\"What else hath ye to sell?\"",
        "\"What else doth thou wish to sell?\"",
        "\"What other arms wilt thou sell?\"",
    ][usize::from(roll) % 4]
        .to_string()
}

/// **Measured** 2026-09-07: quoted, and attributed to the shopkeeper by the
/// caller - the capture leaving the browser reads `"Godspeed..."` over
/// `says Gwenneth.`.
fn arms_sell_goodbye(roll: u8) -> String {
    [
        "\"Good-bye...\"",
        "\"Mayhap another time...\"",
        "\"Godspeed...\"",
        "\"Fare thee well...\"",
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
/// **Measured** 2026-09-07: the call line closes the shopkeeper's speech that
/// the listing heading opened, so it carries a closing double quote and no
/// opening one - `Which would ye see?"`, `What is thine interest?"` and
/// `What may I show thee?"` all drew that way.
const fn arms_stock_call_for_roll(roll: u8) -> &'static str {
    match roll & 0x03 {
        0 => "What may I show thee?\"",
        1 => "Which wouldst thou like to see?\"",
        2 => "What is thine interest?\"",
        _ => "Which would ye see?\"",
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
    // Measured 2026-09-07 (`qa/paired/shop-arms-menus.tsv`): the row reads
    // `"Anything else,` - the opening quote is part of the line, and there is
    // no closing one. `shops.md` §8.1 transcribes the literal without it.
    format!("\"Anything else, {suffix}")
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

/// **Measured** 2026-09-07 (`qa/paired/shop-arms-menus.tsv`, Iolo's Bows):
/// the Buy listing opens with this two-row heading, then a blank row, then one
/// row per stock letter, then a blank row, and only then the call line of
/// `shops.md §8.1`. The engine printed the call line *first* and folded the
/// list into one `We have: a) Dagger, b) Sling, ...` sentence.
/// `shops.md §8.B`: "On Buy, the echo is followed by one uniformly selected
/// affirmation from `Very good!\n`, `Excellent!\n`, `Fine, fine!\n`,
/// `But of course!\n`, then one independently selected stock introduction from
/// `We have:`, `We stock:`, `Thou canst buy:`, `We've got:`." Two draws, in
/// that order. The engine printed the one combination
/// `qa/paired/shop-arms-menus.tsv` happened to capture as a fixed heading.
const ARMS_BUY_AFFIRMATIONS: [&str; 4] =
    ["Very good!", "Excellent!", "Fine, fine!", "But of course!"];
const ARMS_BUY_STOCK_INTRODUCTIONS: [&str; 4] =
    ["We have:", "We stock:", "Thou canst buy:", "We've got:"];

/// The opening double quote belongs to §8.B's `Buy\n\n"` echo, which this
/// engine has no separate producer for, so the composed heading carries it.
fn arms_buy_menu_heading(affirmation_roll: u8, introduction_roll: u8) -> String {
    format!(
        "\"{}\n{}",
        ARMS_BUY_AFFIRMATIONS[usize::from(affirmation_roll & 0x03)],
        ARMS_BUY_STOCK_INTRODUCTIONS[usize::from(introduction_roll & 0x03)]
    )
}

/// The stock rows read `a...Dagger` - the separator is three ASCII full stops,
/// read back by glyph index rather than guessed from the blank-cell filler.
fn format_arms_stock_buy_menu(
    table: crate::shops::ArmsStockTable,
    heading: Option<(u8, u8)>,
) -> String {
    if table.is_empty() {
        // Unmeasured: no shipped arms shop ships an empty stock table, so this
        // arm is a guard rather than a transcript, and the original has no
        // line for a case it cannot reach.
        // audit: not a player-facing line
        return String::new();
    }
    let mut rows = String::new();
    for index in 0..table.len() {
        let item = table.item_ids[index] as usize;
        let letter = (b'a' + index as u8) as char;
        rows.push_str(&format!("{letter}...{}\n", equipment_name(item)));
    }
    // `shops.md §8.B`: "These two draws occur once per accepted Buy entry.
    // Repeated item listings do not redraw either heading." The engine drew a
    // fresh affirmation and introduction after every purchase and declined
    // quote, which both reprinted the heading and spent two draws the original
    // does not.
    match heading {
        Some((affirmation_roll, introduction_roll)) => format!(
            "{}\n\n{rows}",
            arms_buy_menu_heading(affirmation_roll, introduction_roll)
        ),
        None => rows,
    }
}

/// `shops.md §8.C`'s "Inn result" table, which publishes every one of these
/// refusals. The engine's own sentences - `No one is here to lodge.`,
/// `Thou must keep at least one companion.`, `Thy party is already full.`,
/// `That companion is not in thy party.`, `The inn has no more room for
/// guests.` and the two gold lines - were all invented.
fn format_inn_error(err: InnError, speaker_is_female: bool) -> String {
    let honorific = tavern_honorific(speaker_is_female);
    match err {
        // No published row covers "the party is empty"; it is an invariant
        // guard, not a transcript, so it says nothing.
        // audit: not a player-facing line
        InnError::EmptyParty => String::new(),
        // "Leave with only Avatar travelling | ... Record `191`; visit ends
        // without an added attribution or ordinary farewell". The record is
        // the SHOPPE-backed formatter's; this fallback prints nothing rather
        // than a sentence the original does not have.
        InnError::PartyTooSmallToLeave => String::new(),
        // "Pickup with six travelling members | `\n\nOne must first be left
        // behind!\n\n`; this check precedes the no-guests check".
        InnError::PartyFull => "\n\nOne must first be left behind!\n\n".to_string(),
        // "The register only offers guests at this inn, so it has no separate
        // arbitrary-member/not-in-party refusal."
        InnError::InvalidPartyIndex { .. } => String::new(),
        // "Pickup with no guest at this inn | Token-expanded `\n\n"No one here
        // is from thy party!"\nsays $.\n\n`". The attribution belongs to the
        // SHOPPE-backed formatter.
        InnError::InvalidGuestIndex { .. } | InnError::GuestNotAtInn { .. } => {
            "\n\n\"No one here is from thy party!\"\n\n".to_string()
        }
        // "Capacity check before Rest or Leave | First `\n\n`. If full:
        // `"I am sorry,\n`, honorific, then `, but we\nhave no room\n
        // available."\n\n`."
        InnError::RegistryFull => {
            format!("\n\n\"I am sorry,\n{honorific}, but we\nhave no room\navailable.\"\n\n")
        }
        // `shops.md §8.4`: "Only Rest and Pickup test affordability; there is
        // no separate minimum-gold gate (R420)." The arm is unreachable and
        // silent until the gate itself is removed.
        // audit: not a player-facing line
        InnError::BelowMinimumGold { .. } => String::new(),
        // "Accepted Rest, short funds | `"Highwaymen!\nCheap, at that!\nOUT!" `,
        // then token-expanded `screams\n$.\n`".
        InnError::InsufficientGold { .. } => "\"Highwaymen!\nCheap, at that!\nOUT!\" ".to_string(),
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
        // `shops.md §8.B`: "Buy echoes `Buy\n\n"`, Sell echoes `Sell\n\n"`" -
        // and "There is no additional instruction line listing keys." The
        // listing itself belongs to the stateful site, which draws the stock
        // rows; these two arms are the echo alone.
        EnteredBuy => "Buy\n\n\"".to_string(),
        EnteredSell => "Sell\n\n\"".to_string(),
        // Measured: quoted, and attributed with `growls` rather than the
        // `says` the carry-cap refusal takes.
        SellRefusedEmpty => speech.attribute("\"Thou hast nothing to sell!\"", "growls"),
        SellBrowserMoved => String::new(),
        // The stateful site above owns the closing flourish, because the
        // record id is a per-render draw.
        Exited => String::new(),
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
        SellRefusedZeroPrice { .. } => {
            speech.attribute("\"That, I cannot buy from thee.\"", "says")
        }
        // **Measured** 2026-09-07 (`qa/paired/shop-arms-sell-ammunition.tsv`):
        // quoted and attributed with `growls`, the same verb the
        // empty-inventory refusal takes.
        SellRefusedAmmunition { .. } => {
            speech.attribute("\"We don't deal in used ammunition!\"", "growls")
        }
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
        Declined => String::new(),
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
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

/// **Measured** 2026-09-07 (`qa/paired/shop-arms-buy.tsv`, plus the
/// `shoppe_find` probe for the ids): a shop's closing flourish is a rendered
/// record from the shared 0-7 band, wrapped in the same attribution tail the
/// carry-cap refusal uses. Two of that band's records were seen this way -
/// 0 and 2 - which is what makes §8.A's uniform `0..3` draw read as the
/// record id itself for this shop's row. Whether another shop kind's row
/// starts elsewhere in the band is not measured (`cleak/u5-spec#238`).
fn render_shared_shoppe_flourish(game_dir: &Path, record_id: usize) -> Option<String> {
    let placeholders = crate::shoppe_bark::ShoppeBarkContext::default();
    crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
        .ok()
        .and_then(|renderer| renderer.render_record(record_id, &placeholders).ok())
        .map(|text| format!("\"{}\"", text.trim()))
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

fn format_healer_treatment_error(
    error: crate::shops::HealerTreatmentError,
    healer_name: Option<&'static str>,
) -> String {
    match error {
        crate::shops::HealerTreatmentError::InsufficientGold { required, .. } => {
            format!("Thou lackest the {required} gold.")
        }
        // `shops.md §8.C`, healers: "Other keys silently wait without
        // reprinting."
        crate::shops::HealerTreatmentError::InvalidTarget { .. } => String::new(),
        crate::shops::HealerTreatmentError::Untreatable => healer_no_need_refusal(healer_name),
    }
}

/// The typed-input prompt the sage question ends on, measured with it.
const SAGE_RESPOND_PROMPT: &str = "You respond:";
/// **Measured** 2026-09-08 at the Blue Boar: the six drinks it sells, in the
/// order its list prints them. The prices come from the published table.
const BLUE_BOAR_DRINK_ROWS: [(char, &str, crate::shops::BlueBoarDrinkChoice); 6] = [
    ('a', "Rose", crate::shops::BlueBoarDrinkChoice::A),
    ('b', "Claret", crate::shops::BlueBoarDrinkChoice::B),
    ('c', "Sauterne", crate::shops::BlueBoarDrinkChoice::C),
    ('d', "Muscatel", crate::shops::BlueBoarDrinkChoice::D),
    ('e', "Moselle", crate::shops::BlueBoarDrinkChoice::E),
    ('f', "Chablis", crate::shops::BlueBoarDrinkChoice::F),
];
const BLUE_BOAR_CHOICE_PROMPT: &str = "Thy choice?\"";
/// The tavern's own follow-up question, measured with the zero-quantity
/// dismissal and reused when the sage hands control back.
///
/// `shops.md §8.C` publishes it exactly: after a branch returns to
/// continuation the tavern "refresh[es] the stats panel, retain[s] the message
/// window's text and cursor, and print[s] `"Anything else\nfor thee?" `" - one
/// explicit line feed inside it and a trailing space, which is what leaves the
/// cursor on the answer's row.
const TAVERN_ANYTHING_ELSE_PROMPT: &str = "\"Anything else\nfor thee?\" ";

/// `shops.md §8.C`: "Tavern and sage honorifics address Avatar: `sir` for
/// male, otherwise `milady`."
const fn tavern_honorific(speaker_is_female: bool) -> &'static str {
    if speaker_is_female { "milady" } else { "sir" }
}

/// `shops.md §8.C` "No packs affordable, at least three food servings". The
/// attribution row - `yells `, vendor name, `.\n` - belongs to the
/// SHOPPE-backed formatter, which knows the name.
const TAVERN_NO_GOLD_NO_NEED_BARK: &str = "\n\n\"Thou hast\nneither gold nor\nneed! Out!\"";
/// **Measured** 2026-09-08 at Hotel Brittany: the inn's room quote and the
/// shipwright's hull quote end on the same prompt.
const INN_CONFIRM_PROMPT: &str = "Wilt thou take it?\"";
/// **Measured** 2026-09-08 at Hotel Brittany with a party of one.
/// **Measured**: the question the leave-companion branch asks.
const INN_WHO_WILL_STAY_PROMPT: &str = "\"Who will stay?\"";
/// **Measured**: what the innkeeper says once a companion is lodged, and the
/// question the visit continues on.
const INN_THANKS_LINE: &str = "\"I thank thee.\"";
const INN_ANYTHING_MORE_PROMPT: &str = "\"Is there anything more I can do for thee?\"";
/// **Measured**: the line that follows the pick-up bill, attributed. Note the
/// comma inside the quotes - the attribution completes the sentence.
const INN_STAY_ENJOYABLE_LINE: &str = "\"I hope thou hast found thy stay enjoyable,\"";
const INN_NOBODY_TO_LEAVE_REFUSAL: &str =
    "\"Lord British is missing, and all ye plan to do is SLEEP for a month or so? Not in my inn!\"";

/// **Measured**: the line a declined room draws, attributed to the innkeeper.
fn inn_declined_line(innkeeper_name: Option<&'static str>) -> String {
    let line = "\"Perhaps another time...\"";
    match innkeeper_name {
        Some(name) => format!("{line}\nsays {name}."),
        None => line.to_string(),
    }
}

fn format_tavern_outcome(
    outcome: crate::shop_runtime::TavernOutcome,
    speaker_is_female: bool,
) -> String {
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
        // **Measured** 2026-09-08 at The Cat's Lair in Paws
        // (`qa/paired/paws-sage.tsv`): the lore letter opens a quoted,
        // gendered question and a typed-input prompt on its own row.
        //
        // ```text
        // "Of what wouldst
        // thou hear my
        // lore, sir?"
        //
        // You respond:
        // ```
        EnteredSagePrompt => format!(
            "\"Of what wouldst thou hear my lore, {}?\"\n\n{SAGE_RESPOND_PROMPT}",
            if speaker_is_female { "milady" } else { "sir" }
        ),
        RoundDrinkServed { tavern, cost } => {
            // **Measured** 2026-09-08 at The Cat's Lair
            // (`qa/paired/paws-sage.tsv`): a served purchase is followed by
            // the tavern's own question, `"Anything else for thee?"`, not by
            // an `Anything else? (Y/N)` of the engine's invention. The
            // purchase line itself is a `SHOPPE.DAT` record whose id is not
            // published, so the engine's sentence still stands above it.
            format!(
                "{} served a round for {cost} gold.\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                tavern.display_name()
            )
        }
        SecondaryTavernSelected {
            tavern,
            letter,
            cost,
        } => {
            format!(
                "{} served {letter} for {cost} gold.\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                tavern.display_name()
            )
        }
        // **Measured** 2026-09-08 at the Blue Boar in West Britanny
        // (`qa/paired/wb-blueboar.tsv`): a six-row fixed-price list whose
        // rows are `letter)`, a space, the drink's name, a dot leader and the
        // price right aligned at column sixteen.
        //
        // ```text
        // a) Rose.......18
        // b) Claret....192
        // ...
        //
        // Thy choice?"
        // ```
        //
        // The prices are the published ones; the names were not in this
        // engine at all, since it printed `Choose Blue Boar drink A-F.`
        PickBlueBoarDrink => {
            let mut rows = String::new();
            for (letter, name, choice) in BLUE_BOAR_DRINK_ROWS {
                let price = crate::shops::blue_boar_drink_price(choice).to_string();
                let prefix = format!("{letter}) {name}");
                let dots = crate::message_window::MESSAGE_WINDOW_WIDTH
                    .saturating_sub(prefix.len() + price.len())
                    .max(1);
                rows.push_str(&format!("{prefix}{}{price}\n", ".".repeat(dots)));
            }
            format!("{rows}\n{BLUE_BOAR_CHOICE_PROMPT}")
        }
        // `shops.md §8.C`: "Fourth secondary drink attempt, when prior count
        // is exactly three | `\n\n"I beg thy\npardon, `, honorific, `,"\nsays `,
        // vendor name, `.\n"But haven't\nye had enough\nto drink?" `". The
        // vendor name is only available to the SHOPPE-backed formatter, which
        // overrides this arm; without it the attribution row is dropped rather
        // than invented.
        ConfirmEnoughDrink => format!(
            "\n\n\"I beg thy\npardon, {},\"\n\"But haven't\nye had enough\nto drink?\" ",
            tavern_honorific(speaker_is_female)
        ),
        // `shops.md §8.C` "Enough-drink answer": "Y prints `Yes\n\n` and
        // returns to continuation."
        DeclinedEnoughDrink => format!("Yes\n\n{TAVERN_ANYTHING_ELSE_PROMPT}"),
        // Measured: the served drink draws one gendered line, and the price
        // is not repeated back.
        BlueBoarDrinkServed { choice, cost } => {
            let _ = (choice, cost);
            format!(
                "\"Ah, a fine choice, {}. Enjoy!\"\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                if speaker_is_female { "milady" } else { "sir" }
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
        // **Measured** 2026-09-07 at The Cat's Lair in Paws
        // (`qa/paired/paws-tavern.tsv`): both lines are quoted, and they sit
        // on consecutive rows with no blank between them.
        Declined => format!("\n\n\"Hrumph.\"{TAVERN_ANYTHING_ELSE_PROMPT}"),
        RefusedShortFunds { .. } => TAVERN_AFFORDABILITY_REFUSAL_BARK.to_string(),
        // `shops.md §8.C` "No packs affordable, at least three food servings":
        // `\n\n"Thou hast\nneither gold nor\nneed! Out!"\nyells `, vendor name,
        // `.\n`; ends visit. The vendor-named form is the SHOPPE-backed
        // formatter's; this fallback prints the bark without the attribution
        // row rather than inventing a name.
        RefusedNoNeed => TAVERN_NO_GOLD_NO_NEED_BARK.to_string(),
        Exited => "Farewell.".to_string(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

fn format_tavern_outcome_with_shoppe(
    outcome: crate::shop_runtime::TavernOutcome,
    provision_quote_record_id: Option<usize>,
    no_sale_record_id: Option<usize>,
    tavern_vendor_name: Option<&'static str>,
    speaker_is_female: bool,
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
        // `shops.md §8.5`: the charitable outcome "adds `1` to the food
        // counter and renders `SHOPPE.DAT` ordinal `90`, a table-scraps
        // brush-off". The record id travels on the outcome; the engine used
        // to drop it and print an invented sentence naming the tavern.
        CharityProvisions {
            tavern, record_id, ..
        } => renderer
            .render_record(
                record_id,
                &crate::shoppe_bark::ShoppeBarkContext {
                    shop_name: tavern.display_name(),
                    ..Default::default()
                },
            )
            .ok(),
        // `shops.md §8.5`, the outcome table: a fully served quantity prints
        // a "blank-line tail" and then runs the surcharge - no sentence of
        // its own. The partial line is resident text this spec does not
        // quote, so that one still falls through to the placeholder below.
        ProvisionsPurchased {
            completion: crate::shops::ProvisionPurchaseCompletion::Completed,
            ..
        } => Some(String::from("\n")),
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
        // Measured: the closing bark is attributed to the tavern's own vendor
        // - `"What's wrong with ye? Can't hold thy liquor?"` over
        // `says Dr. Cat.` - using the §8.0 vendor-name row the engine already
        // keeps for the arms shops.
        DeclinedContinuation | NoSaleExit => no_sale_record_id.and_then(|record_id| {
            renderer
                .render_record(record_id, &crate::shoppe_bark::ShoppeBarkContext::default())
                .ok()
                .map(|rendered| {
                    let attributed = match tavern_vendor_name {
                        Some(name) => format!("{rendered}\nsays {name}."),
                        None => rendered,
                    };
                    if matches!(outcome, DeclinedContinuation) {
                        format!("No\n{attributed}")
                    } else {
                        attributed
                    }
                })
        }),
        _ => None,
    });
    rendered
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| format_tavern_outcome(outcome, speaker_is_female))
}

fn format_sage_outcome(outcome: crate::shop_runtime::SageOutcome) -> String {
    use crate::shop_runtime::SageOutcome::*;
    match outcome {
        // The quote body is `SHOPPE.DAT` record `84`; this is the no-assets
        // fallback, and `§8.C`'s resident suffix is the part that is ours to
        // print either way. The engine's own `That will cost % gold. Pay?
        // (Y/N)` sentence appears nowhere in the spec.
        QuotedFee { .. } => crate::shoppe_bark::SAGE_FEE_CONFIRMATION_SUFFIX
            .trim_start()
            .to_string(),
        RumourFound { rendered, .. } => rendered,
        Declined => "Farewell.".to_string(),
        RefusedShortFunds { .. } => TAVERN_AFFORDABILITY_REFUSAL_BARK.to_string(),
        InputTooLong { limit, .. } => format!("Ask in {limit} characters or fewer."),
        NoTopicMatch => "That, I cannot help thee with.".to_string(),
        Exited => "Farewell.".to_string(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
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

fn format_reagent_outcome(
    outcome: crate::shop_runtime::ReagentShopOutcome,
    declined_herbalist: Option<crate::shops::Herbalist>,
) -> String {
    use crate::shop_runtime::ReagentShopOutcome::*;
    match outcome {
        // **Measured** 2026-09-07 at Cove (`qa/paired/cove-herbalist.tsv`):
        // the herbalist's list has the same shape as the arms listing - a
        // heading, a blank row, one `letter...name` row per stocked reagent,
        // a blank row, then the question:
        //
        // ```text
        // "Fine! We sell:
        //
        // A...Spider Silk
        // B...Blood Moss
        // C...Black Pearl
        // D...Nightshade
        // E...Mandrake
        //
        // Thy interest?"
        // ```
        //
        // The engine printed `<herbalist> offers reagents A-E, or Space.`
        EnteredMenu { herbalist } => {
            let mut rows = String::new();
            for entry in crate::shops::herbalist_menu_entries(herbalist) {
                rows.push_str(&format!(
                    "{}...{}\n",
                    entry.letter,
                    entry.reagent.display_name()
                ));
            }
            format!("{REAGENT_MENU_HEADING}\n\n{rows}\n{REAGENT_MENU_QUESTION}")
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
        // Measured: declining a quote redraws the list and its question
        // rather than printing a dismissal of its own.
        Declined => match declined_herbalist {
            Some(herbalist) => format_reagent_outcome(EnteredMenu { herbalist }, None),
            None => String::new(),
        },
        Exited => "Farewell.".to_string(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

const REAGENT_MENU_HEADING: &str = "\"Fine! We sell:";
const REAGENT_MENU_QUESTION: &str = "Thy interest?\"";

/// `shops.md §8.6`, the horse trader's one placement refusal.
const HORSE_TRADER_CLOSED_REFUSAL: &str = "The stables are closed.\n";

fn format_horse_trader_outcome(outcome: crate::shop_runtime::HorseTraderOutcome) -> String {
    use crate::shop_runtime::HorseTraderOutcome::*;
    match outcome {
        QuotedPrice { price } => format!("A fine steed costs {price} gold. (Y/N)"),
        Purchased { price } => format!("Sold for {price} gold. Thy horse awaits outside."),
        RefusedShortFunds { price } => format!("Thou lackest the {price} gold."),
        // `shops.md §8.6`: "No free active-object slot and no suitable
        // adjacent placement cell both print `The stables are closed.\n`
        // before any greeting, then exit." One line covers both failures;
        // `There is no room for a horse here.` was invented.
        RefusedNoMarker { .. } => HORSE_TRADER_CLOSED_REFUSAL.to_string(),
        Declined => "As you wish.".to_string(),
        Exited => "Farewell.".to_string(),
        // `shops.md §8.6`: the purchase loop is "Y/N driven" and other keys
        // wait; no refusal prints.
        InvalidInput => String::new(),
    }
}

/// `shops.md §8.B`, the shipwright's row: "`Yes`, then record `119` with no
/// added resident menu question; the record carries its own leading spacing
/// and prompt." The asset supplies the wording, so the engine reads it rather
/// than transcribing it; these two literals were measured at The Rusty Bucket
/// in Buccaneer's Den (`qa/paired/bd-shipwright.tsv`) and remain only as the
/// fallback for a run with no `SHOPPE.DAT`.
const SHIPWRIGHT_STOCK_LINE: &str = "\"We sell ocean-going Frigates and small, light Skiffs.";
const SHIPWRIGHT_STOCK_QUESTION: &str = "Which would ye like to see?\"";
/// The confirmation prompt a quoted hull ends on, quoted like the arms
/// browser's `Deal?"`.
const SHIPWRIGHT_CONFIRM_PROMPT: &str = "Wilt thou take it?\"";

fn format_ship_broker_outcome(
    outcome: crate::shop_runtime::ShipBrokerOutcome,
    vendor_name: Option<&'static str>,
    menu_record: Option<String>,
) -> String {
    use crate::shop_runtime::ShipBrokerOutcome::*;
    match outcome {
        EnteredMenu { .. } => menu_record
            .unwrap_or_else(|| format!("{SHIPWRIGHT_STOCK_LINE}\n\n{SHIPWRIGHT_STOCK_QUESTION}")),
        QuotedPurchase { quote } => {
            // The quote body is a `SHOPPE.DAT` record with the price
            // substituted; until its record id is published this keeps the
            // engine's own sentence and appends the measured prompt.
            let item = match quote.kind {
                crate::shops::ShipwrightPurchaseKind::Frigate => "frigate",
                crate::shops::ShipwrightPurchaseKind::Skiff => "skiff",
            };
            format!(
                "A {item} costs {} gold.\n\n{SHIPWRIGHT_CONFIRM_PROMPT}",
                quote.price
            )
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
        // Measured: declining the hull ends the visit on the shipwright's own
        // quoted jeer, attributed like every other shop bark.
        Declined => match vendor_name {
            Some(name) => format!("\"Hmph! Landlubber!\"\nsays {name}."),
            None => "\"Hmph! Landlubber!\"".to_string(),
        },
        Exited => "Farewell.".to_string(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

/// **Measured** 2026-09-08 at The Guild in Paws (`qa/paired/paws-guild.tsv`):
///
/// ```text
/// "We sell:
///
/// a.........Keys
/// b.........Gems
/// c......Torches
///
/// Thy concern?"
/// ```
///
/// The dots are literal full stops, and unlike the arms and herbalist
/// listings - which use a fixed three-dot separator - the guild pads its
/// leader so that letter, dots and name occupy fourteen columns, right
/// aligning the names.
const GUILD_MENU_HEADING: &str = "\"We sell:";
const GUILD_MENU_QUESTION: &str = "Thy concern?\"";
/// The prompt a quoted commodity ends on.
const GUILD_QUOTE_PROMPT: &str = "Interested?\"";
/// The heading the list takes when a declined quote redraws it.
const GUILD_MENU_AGAIN_HEADING: &str = "\"What else, then?";
const GUILD_MENU_ROW_WIDTH: usize = 14;

fn guild_menu_rows() -> String {
    let mut rows = String::new();
    for (letter, commodity) in [
        ('a', crate::shops::GuildCommodity::Keys),
        ('b', crate::shops::GuildCommodity::Gems),
        ('c', crate::shops::GuildCommodity::Torches),
    ] {
        let name = commodity.display_name();
        let dots = GUILD_MENU_ROW_WIDTH.saturating_sub(1 + name.len());
        rows.push_str(&format!("{letter}{}{name}\n", ".".repeat(dots)));
    }
    rows
}

fn format_guild_outcome(outcome: crate::shop_runtime::GuildShopOutcome) -> String {
    use crate::shop_runtime::GuildShopOutcome::*;
    match outcome {
        EnteredMenu { .. } => format!(
            "{GUILD_MENU_HEADING}\n\n{}\n{GUILD_MENU_QUESTION}",
            guild_menu_rows()
        ),
        QuotedUnit {
            shop,
            commodity,
            unit_price,
        } => format!(
            "{} sells {} for {unit_price} gold each.\n\n{GUILD_QUOTE_PROMPT}",
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
        // Measured: a declined quote redraws the list under its own heading.
        Declined => format!(
            "{GUILD_MENU_AGAIN_HEADING}\n\n{}\n{GUILD_MENU_QUESTION}",
            guild_menu_rows()
        ),
        Exited => "Farewell.".to_string(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
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
    let (_text, ended) = state.submit_active_conversation_keyword(&line);
    if !ended {
        if let Some(session) = state.active_conversation.as_ref() {
            let prompt = session.prompt_message();
            if !prompt.is_empty() {
                // The reply is already on the transcript: the submit path
                // ends in `emit_tlk_message`, which pushes the rendered
                // lines *and* marks the slot flushed. Re-assigning the
                // reply text here left the slot differing from the flushed
                // value, so the safety-net flush appended it a second time
                // and every answer printed twice - `Greyson` / `Greyson`,
                // `I am an adventurer!` / `I am an adventurer!`. The slot
                // only owes the prompt that follows the reply.
                state.message = prompt;
            }
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
    // `endgame.md §5.1`'s blocking key read between pages. The greeting
    // is one page; the question that follows it is the next, and the
    // confirmation only reads once both are on screen.
    if state.advance_endgame_greeting_page() {
        return Ok(PlayInputDisposition::Continue);
    }
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
    }
    // Anything else is simply re-read. `endgame.md §5.1`: each prompt is a
    // "blocking single-key read that accepts only `Y` or `N` and re-reads
    // on anything else" - it prints nothing, and a paired capture of a
    // stray key at the prompt leaves the stock window untouched.
    Ok(PlayInputDisposition::Continue)
}

/// Which slots the player may actually type a command for. `combat.md
/// §6.1a`'s slot-to-group helper decides who reaches the player-command
/// handler - a party-side actor carrying the controlled bit (Sword of
/// Chaos, possession, Charm) resolves to group 1 and goes to the automatic
/// driver, while a monster-side actor carrying it resolves to group 0 and
/// "is dispatched to the keystroke/command path, not to the automatic
/// driver" (`§6.1a` writer 3; `magic.md`'s "the player never gets to move
/// it" is withdrawn by `RETRACTIONS.md` R354).
///
/// Past that split the handler "**prompts, and never synthesizes**. Its one
/// gate is the active-player sentinel" (`§16.1`), so every group-0 slot the
/// sentinel admits is prompted, party or monster.
///
/// *(**Corrected.** This used to AND in the descriptor's party-side bit, on
/// §16.1's withdrawn "still synthesizes an automatic action" clause -
/// `RETRACTIONS.md` R377.)*
fn combat_actor_accepts_player_input(
    state: &PlayState,
    slot: usize,
    actor: CombatActorDescriptor,
) -> bool {
    combat_actor_is_active_not_dead(actor)
        && combat_slot_prompted_by_player_command_handler(state.active_player, slot, actor)
}

fn combat_has_dispatchable_player_actor(state: &PlayState) -> bool {
    state
        .combat_actors
        .iter()
        .enumerate()
        .take(COMBAT_ACTOR_SLOTS)
        .map(|(slot, actor)| (slot, *actor))
        .any(|(slot, actor)| combat_actor_accepts_player_input(state, slot, actor))
}

/// `combat.md §16.1`: "Side counting skips empty, dead, and passive
/// descriptors, then uses the same group resolver. Group 1 counts as
/// foes and group 0 as friends. Control and the traitor identity
/// therefore affect victory detection." The raw slot-index scan this
/// replaced disagreed with the round loop's own census for exactly the
/// charmed-monster and traitor cases §16.1 publishes.
pub(crate) fn combat_has_active_non_party_actor(state: &PlayState) -> bool {
    combat_has_active_not_dead_non_party_actor(&state.combat_actors)
}

fn combat_pending_player_actor_is_active(state: &PlayState, actor_slot: usize) -> bool {
    state
        .combat_actors
        .get(actor_slot)
        .copied()
        .is_some_and(|actor| combat_actor_accepts_player_input(state, actor_slot, actor))
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

    // `combat.md §8`, the `C` row: "The branch first prints `Cast...` and
    // then applies its own copy of the shape-A **party-side** test: a monster
    // acting under player control gets `Can't!`, the banner is reprinted and
    // the prompt is re-issued at no cost." (`RETRACTIONS.md` R381 - the
    // withdrawn wording made this a dead-caster test.)
    if matches!(
        resolve_combat_command_party_side_gate(
            CombatCommandBranch::CastSpell,
            state.combat_actors.get(actor_slot).copied(),
        ),
        CombatCommandPartySideGate::RefusedMonsterSide
    ) {
        state.message.clear();
        state.emit_combat_command_echo_line(&format!(
            "{COMBAT_CAST_COMMAND_LABEL}{COMBAT_PARTY_SIDE_REFUSAL}"
        ));
        // `§8.1`: every `Can't!` refusal takes the full re-prompt, "the whole
        // banner reprinted from its leading newline".
        state.open_pending_combat_player_turn(Some(actor_slot));
        return Ok(PlayInputDisposition::Continue);
    }

    // `combat.md §8`, the `C` row: "The branch first prints `Cast...`". The
    // refusal arm above already carries the label on its own row, so the
    // accepted arm supplies it here, before the spell-name exchange. Stock
    // shows ` Cast...` and `Spell name:` on consecutive rows.
    state.message.clear();
    state.emit_combat_command_echo_line(COMBAT_CAST_COMMAND_LABEL);

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

/// `combat.md §8.2`: append one attempt-walk advance to the transcript and,
/// when no further cursor opens, spend the acting combatant's turn.
/// "The acting character's turn is consumed either way: cancelling with
/// Escape or Space does not return to the command prompt and does not give
/// the turn back."
fn finish_combat_attack_walk(
    state: &mut PlayState,
    actor_slot: usize,
    had_foe: bool,
    walk: CombatAttackWalkApplication,
) {
    // `combat.md §11.1`: a monster carrying the controlled/charmed bit prints
    // "on a failed roll `<target> missed!`" where the self-acting hostile
    // prints nothing - `§7` step 6 puts such a slot in group zero, so it "is
    // therefore driven from the player's prompt".
    // Everything else on the row is the shared narrator's.
    if let Some((_, monster_attack)) = walk.monster_attack
        && let Some(line) = combat_monster_attack_narrated_result_message(
            state,
            monster_attack,
            CombatMonsterAttackNarration::Controlled,
        )
    {
        state.emit_combat_print(&format!("\n{line}\n"));
    }
    if let Some((target_slot, attack)) = walk.attack
        && let Some(line) = combat_weapon_attack_narrated_result_message(state, target_slot, attack)
    {
        // `combat.md §11.1`: the party melee arm emitted step 3's newline
        // "before the roll", which is the same one leading this print; the
        // cursor was left mid-row by `Aim! `, so it closes that row instead
        // of blanking one.
        state.emit_combat_print(&format!("\n{line}\n"));
    }
    if !walk.text.is_empty() {
        // `combat.md §8.2`: `Attack-` and `Aim! ` are what `A` adds on top
        // of the turn banner, and they are typed onto the marker row the
        // banner opened - `text-output.md §10.2`: "echoed command lines
        // carry it". `Aim! ` carries no newline, so the cursor stays on that
        // row and the attempt's own result line follows it without a blank.
        state.emit_combat_command_echo_line(&walk.text);
    }
    if walk.cursor_open {
        // The turn is not over: `§8.2` opens one cursor per readied item and
        // the acting combatant keeps the keyboard. `combat.md §7`'s
        // `VICTORY!` belongs to the end of the dispatch, so a kill on this
        // attempt is announced when the last attempt closes - the census
        // that decides it is carried by the walk, not recomputed then.
        state.pending_combat_actor_slot = Some(actor_slot);
        return;
    }

    state.pending_combat_actor_slot = None;
    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
    if state.combat_active
        && matches!(
            state.combat_round_loop_control(false, false),
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
        )
    {
        state.apply_combat_round_loop_exit(CombatRoundLoopExit::Defeat);
        return;
    }
    if state.combat_active {
        // `combat.md §7`: "If party actors remain and foes do not, it prints
        // `VICTORY!` once and continues" (`RETRACTIONS.md` R289).
        if had_foe
            && !combat_has_active_non_party_actor(state)
            && state.announce_combat_victory_if_needed()
        {
            // `combat.md §7`: the resident `VICTORY!` string goes through
            // the ordinary string printer with "one leading and one trailing
            // newline", which is exactly one arena output print.
            state.emit_combat_print(crate::combat_frame::COMBAT_VICTORY_LINE);
        }
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }
}

/// `combat.md §8.2`: while a targeting cursor is open it owns the keyboard.
/// The cursor loop "reads another key" for every input outside its published
/// table, so no keystroke reaches the combat command parser here.
fn handle_combat_targeting_cursor_key(
    state: &mut PlayState,
    key: char,
    suffix: &str,
) -> PlayInputDisposition {
    let Some(actor_slot) = state
        .active_combat_targeting
        .as_ref()
        .map(|session| session.actor_slot)
    else {
        return PlayInputDisposition::Continue;
    };
    // `combat.md §8.2`: the cursor's own prints continue the line `A` opened
    // - `Aim! ` carries no newline - and a discarded key prints nothing at
    // all, so the standing transcript is appended to rather than replaced.
    for cursor_key in std::iter::once(key).chain(suffix.chars()) {
        // `combat.md §7`: `VICTORY!` is owed when "party actors remain and
        // foes do not". The census belongs to the whole `A` walk, not to
        // this keystroke: an earlier attempt in the same walk may already
        // have killed the last foe, and re-asking here would answer `false`
        // and lose the line.
        let Some(had_foe) = state
            .active_combat_targeting
            .as_ref()
            .map(|session| session.foes_present_at_walk_start)
        else {
            break;
        };
        if let Some(walk) = state.apply_combat_targeting_cursor_key(cursor_key) {
            finish_combat_attack_walk(state, actor_slot, had_foe, walk);
        }
    }
    PlayInputDisposition::Continue
}

fn handle_combat_key_input(state: &mut PlayState, key: char, suffix: &str) -> PlayInputDisposition {
    if state.active_combat_targeting.is_some() {
        return handle_combat_targeting_cursor_key(state, key, suffix);
    }
    state.ensure_pending_combat_player_turn();
    let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
        state.message.clear();
        return PlayInputDisposition::Continue;
    };
    if !combat_pending_player_actor_is_active(state, actor_slot) {
        // The slot the walker had pending is no longer one the handler
        // prompts - it died, or the active-player sentinel moved. `§16.1`
        // skips such a slot "without a banner, a keystroke or an action", so
        // nothing is printed. The shape-A `Can't!` refusal is **not** here:
        // that gate reads the party-side bit on a slot that *is* prompted
        // (`RETRACTIONS.md` R381), and it is applied in
        // `handle_combat_multistage_command` below.
        state.message.clear();
        // The slot is not reinstated: the walker re-selects an acting slot on
        // the next keystroke, which is `§8`'s free re-prompt.
        return PlayInputDisposition::Continue;
    }
    let input = combat_player_command_input_from_key(key);
    // `combat.md §8.1`: the banner was already emitted into the transcript
    // when this turn opened - "before any key is read" - so this keystroke's
    // own transcript starts with the command output and never reprints it.
    // The free re-prompt below reinstates the pending slot without reopening
    // the turn, which is what keeps that path on the short form.
    let Some(application) = state.apply_combat_player_command_with_inputs(actor_slot, input) else {
        state.message.clear();
        return PlayInputDisposition::Continue;
    };
    let echo = combat_player_command_application_message(state, &application);
    state.message = echo.clone();
    if handle_combat_multistage_command(state, actor_slot, &application.action, suffix) {
        return PlayInputDisposition::Continue;
    }
    // The key was read on the marker row, so what it echoes is written
    // there. `text-output.md §10.2` states that rule for the loops it
    // lists, and combat is not one of them, so the arena's end-cap
    // placement is a runtime observation rather than a §10.2 claim.
    // `combat.md §8.1` is what supplies the row's line feed - the turn
    // handler "emits the line feed itself, unconditionally, between
    // printing the banner and reading the command byte" - so the echo
    // neither opens a row of its own nor takes §10.4's derived blank.
    state.message.clear();
    if !echo.is_empty() {
        state.emit_combat_command_echo_line(&echo);
    }
    if let Some(shape) = combat_player_command_action_reprompt_shape(&application.action) {
        match shape {
            // `combat.md §8.1`: "**Short re-prompt, banner not reprinted** ...
            // The handler emits one newline, refreshes the prompt window ...
            // and reads another key", so no line feed was spent on this marker
            // row and `text-output.md §10.4`'s derived blank stands.
            CombatCommandRepromptShape::Short => {
                state.pending_combat_actor_slot = Some(actor_slot);
                state.combat_prompt_row_opened_by_banner = false;
            }
            // "**Full re-prompt, the whole banner reprinted from its leading
            // newline**" - the same producer the turn opened with
            // (`RETRACTIONS.md` R380).
            CombatCommandRepromptShape::FullBanner => {
                state.open_pending_combat_player_turn(Some(actor_slot));
            }
        }
        return PlayInputDisposition::Continue;
    }
    if application.reprompt {
        state.pending_combat_actor_slot = Some(actor_slot);
        state.combat_prompt_row_opened_by_banner = false;
        return PlayInputDisposition::Continue;
    }
    if let CombatRoundLoopControl::Exit(exit) = application.control_after {
        let edge_defeat = exit == CombatRoundLoopExit::Defeat
            && application.out_of_arena_leave.is_some_and(|edge| {
                matches!(edge.outcome, CombatOutOfArenaLeaveOutcome::Accepted { .. })
            });
        state.apply_combat_round_loop_exit(exit);
        if edge_defeat {
            // The edge-exit echo is already on the transcript, so the defeat
            // line is its own print on the row below it.
            state.emit_combat_print("\nBATTLE IS LOST!");
        }
    } else if matches!(
        application.action,
        CombatPlayerCommandAction::OpenTargetingCursor
    ) {
        // `combat.md §8.2`: accepted Attack walks the readied slots, printing
        // `Attack-` (and a per-item name line when two or three qualify) per
        // attempt and opening one cursor per attempt that survives the
        // interference abort.
        let had_foe = combat_has_active_non_party_actor(state);
        let walk = state.begin_combat_attack_walk(actor_slot, had_foe);
        finish_combat_attack_walk(state, actor_slot, had_foe, walk);
        // A scripted `A<keys>` token feeds the rest of its characters to the
        // cursor one at a time, exactly as separate keystrokes would.
        for cursor_key in suffix.chars() {
            let Some(had_foe) = state
                .active_combat_targeting
                .as_ref()
                .map(|session| session.foes_present_at_walk_start)
            else {
                break;
            };
            if let Some(walk) = state.apply_combat_targeting_cursor_key(cursor_key) {
                finish_combat_attack_walk(state, actor_slot, had_foe, walk);
            }
        }
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
        party_side_gate,
    } = action
    else {
        return false;
    };
    if matches!(
        party_side_gate,
        CombatCommandPartySideGate::RefusedMonsterSide
    ) {
        // The label and the `Can't!` are already in the echo
        // (`combat_player_command_message`); this arm only stops the verb
        // reaching its delegate. `§8.1` then takes the banner-reprinting
        // re-prompt.
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
            // echo, passes the party-side gate above, and enters the same item
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
            // `combat.md §8`, the `C` row: "The branch first prints
            // `Cast...`" - before its party-side test and before the
            // spell-name exchange (`RETRACTIONS.md` R381). The caller left
            // this branch's placeholder in the message slot; the label is
            // what actually reaches the window, on the marker row the turn
            // banner opened, so it is emitted as a command echo here rather
            // than being overwritten by the prompt below.
            state.message.clear();
            state.emit_combat_command_echo_line(COMBAT_CAST_COMMAND_LABEL);
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
            // `combat.md §8`, the `Z` row: "Inside an arena that handler tests
            // the acting slot's party side: for a **party-side** actor it opens
            // that character's own sheet silently, with no prompt; for a
            // **monster-side** actor under player control it prints `Player: `
            // and runs the ordinary roster picker, so the player must choose a
            // character." (`RETRACTIONS.md` R381 - the unqualified "selecting
            // the acting combatant's party slot instead of prompting" is
            // withdrawn for a monster-side actor.)
            if state
                .combat_actors
                .get(actor_slot)
                .is_some_and(|actor| actor.is_party_side())
            {
                state.z_stats_for_party(actor_slot);
            } else {
                state.start_party_selector(PartySelectorTarget::ZStats);
            }
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
                    // `combat.md §8`, the `Y` row: "In combat, nonempty Yell
                    // input reaches the handler's no-effect path", which is
                    // the shared handler's own literal - not a second
                    // wording that echoes the word back.
                    let _ = PlayState::normalize_yell_word(word);
                    state.message = YELL_NO_EFFECT_MESSAGE.to_string();
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
                // `combat.md §7`: a round is "a per-actor body that runs
                // zero or one times per slot", and `§8.1` emits the turn
                // banner "at the start of every keyboard-driven combatant's
                // turn, *before any key is read*". The acting slot was taken
                // out of the state when this key was read, so leaving it
                // `None` while the direction prompt waits lets the paced
                // round walker open the *next* turn - printing a second
                // banner over the open `Verb-` row, which then refuses the
                // direction word because its row is no longer last. Every
                // other multi-stage arm reinstates the slot here; these did
                // not.
                state.pending_combat_actor_slot = Some(actor_slot);
                // `commands.md §5.1`: "Every mode's turn loop opens its input
                // line with the same two steps: emit a newline into the
                // message window, then draw that one triangle", and the verb
                // echo "begins in the cell the cursor occupied, one cell right
                // of the triangle". The arena echo is such a line - the stock
                // window shows ` Get-North`, ` Klimb-Pass` and ` Attack-North`
                // all end-capped - so it has to be logged as a command echo.
                // Assigning it to the message slot flushed it as ordinary
                // output, which drew the row one column left of the original.
                let echo = state.render_active_direction_prompt();
                state.message.clear();
                state.emit_combat_command_echo_line(&echo);
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
                // `combat.md §7`: a round is "a per-actor body that runs
                // zero or one times per slot", and `§8.1` emits the turn
                // banner "at the start of every keyboard-driven combatant's
                // turn, *before any key is read*". The acting slot was taken
                // out of the state when this key was read, so leaving it
                // `None` while the direction prompt waits lets the paced
                // round walker open the *next* turn - printing a second
                // banner over the open `Verb-` row, which then refuses the
                // direction word because its row is no longer last. Every
                // other multi-stage arm reinstates the slot here; these did
                // not.
                state.pending_combat_actor_slot = Some(actor_slot);
                // `commands.md §5.1`: "Every mode's turn loop opens its input
                // line with the same two steps: emit a newline into the
                // message window, then draw that one triangle", and the verb
                // echo "begins in the cell the cursor occupied, one cell right
                // of the triangle". The arena echo is such a line - the stock
                // window shows ` Get-North`, ` Klimb-Pass` and ` Attack-North`
                // all end-capped - so it has to be logged as a command echo.
                // Assigning it to the message slot flushed it as ordinary
                // output, which drew the row one column left of the original.
                let echo = state.render_active_direction_prompt();
                state.message.clear();
                state.emit_combat_command_echo_line(&echo);
            }
            true
        }
        CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Open
        | CombatCommandBranch::Search => {
            // `combat.md §7.1`: "Jimmy first requires the party to hold at
            // least one key: with a key count of zero it prints `No keys!` and
            // returns immediately, **before the direction prompt** and before
            // any tile is examined."
            if *branch == CombatCommandBranch::Jimmy && state.keys == 0 {
                // `combat.md §8` Shape A prints the verb label first and the
                // delegate's own line after it. The label is already in the
                // slot.
                state.message.push_str(COMBAT_JIMMY_NO_KEYS_MESSAGE);
                let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                advance_combat_round_after_actor_and_append_message(state, actor_slot);
                return true;
            }
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
                // `combat.md §7`: a round is "a per-actor body that runs
                // zero or one times per slot", and `§8.1` emits the turn
                // banner "at the start of every keyboard-driven combatant's
                // turn, *before any key is read*". The acting slot was taken
                // out of the state when this key was read, so leaving it
                // `None` while the direction prompt waits lets the paced
                // round walker open the *next* turn - printing a second
                // banner over the open `Verb-` row, which then refuses the
                // direction word because its row is no longer last. Every
                // other multi-stage arm reinstates the slot here; these did
                // not.
                state.pending_combat_actor_slot = Some(actor_slot);
                // `commands.md §5.1`: "Every mode's turn loop opens its input
                // line with the same two steps: emit a newline into the
                // message window, then draw that one triangle", and the verb
                // echo "begins in the cell the cursor occupied, one cell right
                // of the triangle". The arena echo is such a line - the stock
                // window shows ` Get-North`, ` Klimb-Pass` and ` Attack-North`
                // all end-capped - so it has to be logged as a command echo.
                // Assigning it to the message slot flushed it as ordinary
                // output, which drew the row one column left of the original.
                let echo = state.render_active_direction_prompt();
                state.message.clear();
                state.emit_combat_command_echo_line(&echo);
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
    state.apply_combat_committed_action_tail(actor_slot)
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

fn combat_player_command_input_from_key(key: char) -> CombatPlayerCommandInput {
    // `combat.md §8.2`: `A` no longer folds a following direction key into a
    // one-shot attack direction. Accepting Attack "opens a second, separate
    // input read, and it is not a one-shot direction key but an
    // **interactive targeting cursor**", so any suffix after `A` is fed to
    // that cursor a key at a time by the caller.
    if key.eq_ignore_ascii_case(&'A') {
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
        // `text-output.md §10.3`, the representative-literal table:
        // "`Pass` + newline | complete". The word carries no full stop,
        // and its trailing newline closes the echo's row, which is what
        // puts a blank row under it before the round's first result line.
        CombatPlayerCommandAction::Pass(_) => "Pass\n".to_string(),
        // `combat.md §8.1`/`§8.2`: what `A` adds on top of the turn banner
        // is `Attack-` and, "immediately before the cursor opens", `Aim! `,
        // plus a per-item name line when two or three items qualify. All of
        // that is produced per attempt by the attempt walker, so this arm
        // contributes nothing of its own.
        CombatPlayerCommandAction::OpenTargetingCursor => String::new(),
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
        // `combat.md §8` Shape A: "The helper prints the verb label, then
        // requires that the acting combatant is a **party-side** descriptor.
        // An actor that is not gets the short `Can't!` refusal." `C`-Cast
        // "carries its own copy of the same party-side test and refuses the
        // same way, after printing `Cast...`" (`RETRACTIONS.md` R381).
        CombatPlayerCommandAction::Branch {
            branch,
            party_side_gate: CombatCommandPartySideGate::RefusedMonsterSide,
        } => {
            let label = match branch {
                CombatCommandBranch::CastSpell => Some(COMBAT_CAST_COMMAND_LABEL),
                _ => combat_command_branch_published_label(*branch),
            };
            format!("{}{COMBAT_PARTY_SIDE_REFUSAL}", label.unwrap_or(""))
        }
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
        CombatCommandBranch::Klimb => DUNGEON_KLIMB_WHAT_REFUSAL.trim_end().to_string(),
        // The music toggle is a harness control, not a game command.
        CombatCommandBranch::ToggleMusic => String::new(),
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
    state: &mut PlayState,
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
    state: &mut PlayState,
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
            let result = weapon_attack.and_then(|attack| {
                combat_weapon_attack_narrated_result_message(state, target_slot, attack)
            });
            result
                .map(|result| format!("{direction}\n{result}\n"))
                .unwrap_or_else(|| format!("{direction}\n"))
        }
        CombatStepOrAttackPrimitiveOutcome::Moved { .. } => format!("{direction}\n"),
        CombatStepOrAttackPrimitiveOutcome::InactiveActor => String::new(),
    }
}

/// `combat.md §6.3`: the common attack result narrator is the only
/// relevant reader of the shared action-result scratch. It clears the
/// kill-narrated bit `0x01` first; a surviving vanish bit `0x02` then
/// suppresses the generic killed/slept/hit chain and is cleared in
/// cleanup, so a vanish death never also prints `<name> killed!`.
///
/// The scope is exactly that chain. `§11.1` names the reader "a wider
/// test that halts the narrator before the kill, sleep, hit and wound
/// chain", and it makes the miss line a separate producer - "The routine
/// that prints a miss line has exactly two call sites, both inside
/// party-side attack helpers" - so a miss neither is suppressed by the
/// bit nor touches the field. Likewise `§6.3`: "If that narrator is not
/// reached, the combat walker replaces the whole field with zero before
/// the next actor dispatch", so a dispatch that produces no result line
/// at all (out of range, no ordinary damage, a special resolution)
/// leaves the field to the walker's own zeroing rather than clearing
/// `0x02` or setting `0x01` here.
///
/// The engine builds attack transcripts after the action has resolved,
/// so the gate is applied here, at the one point per attack where the
/// generic chain would emit its line.
fn combat_apply_attack_narrator_gate(
    state: &mut PlayState,
    line: Option<String>,
    narrates_kill: bool,
    reaches_generic_chain: bool,
) -> Option<String> {
    if !reaches_generic_chain {
        return line;
    }
    let gate = resolve_combat_attack_narrator_gate(state.combat_action_result, narrates_kill);
    state.combat_action_result = gate.result_after;
    if gate.run_generic_chain { line } else { None }
}

/// `combat.md §11.1`: the generic chain is the "kill, sleep, hit and
/// wound chain" - every result line [`combat_landed_damage_result_line`]
/// produces. Both the `Hit` and the `Special` resolutions end there, so
/// both are inside the gate's scope. A failed to-hit takes the separate
/// miss producer - "The routine that prints a miss line has exactly two
/// call sites, both inside party-side attack helpers" - and the
/// resolutions that print nothing never reach the narrator at all, so
/// `§6.3` leaves their field to "the combat walker[, which] replaces the
/// whole field with zero before the next actor dispatch".
///
/// The `Special` arm's own `Thy sword hath shattered!` is **not** in
/// scope: `§11.1` prints it "**inside** the damage roll, so it lands
/// between the hit newline and the result line", ahead of the narrator
/// this gate halts. [`combat_attack_result_narration`] therefore keeps it
/// separate from the result line, so a standing `0x02` can withhold the
/// result line without swallowing the shatter line with it.
pub(crate) fn combat_weapon_resolution_reaches_generic_chain(
    resolution: CombatWeaponAttackResolution,
) -> bool {
    matches!(
        resolution,
        CombatWeaponAttackResolution::Hit { .. } | CombatWeaponAttackResolution::Special { .. }
    )
}

/// The party-side result line with `combat.md §6.3`'s narrator gate
/// applied. The party melee path resolves its attack and assembles its
/// transcript inside one dispatch, so the gate runs here, at the one point
/// per attack where the generic chain would emit its line.
///
/// The gate covers the result line only. `§11.1`'s Glass Sword row puts
/// `Thy sword hath shattered!` inside the damage roll, before the
/// narrator, so it survives a standing `0x02` - and it must, because the
/// vanish branch that raised `0x02` has already printed `<monster>
/// vanishes!` and `§6.3` says the narrator then "skips the generic
/// killed/slept/hit chain, produces no message or sound". A Glass Sword
/// kill of a vanish-class monster therefore reads `<monster> vanishes!`
/// then `Thy sword hath shattered!`, and never also `<monster> killed!`.
pub(crate) fn combat_weapon_attack_narrated_result_message(
    state: &mut PlayState,
    target_slot: usize,
    attack: CombatWeaponAttackApplication,
) -> Option<String> {
    let mut narration = combat_attack_result_narration(state, target_slot, None, attack);
    let narrates_kill = combat_weapon_damage_application_killed(attack.damage_application);
    let reaches_generic_chain =
        narration.result.is_some() && narration.result_reaches_generic_chain;
    narration.result = combat_apply_attack_narrator_gate(
        state,
        narration.result.take(),
        narrates_kill,
        reaches_generic_chain,
    );
    narration.into_message()
}

/// The narrator gate for a monster-side attack already ran, inside the
/// dispatch that resolved the attack
/// ([`PlayState::apply_combat_monster_attack_narrator_gate`]). `combat.md
/// §6.3` gives the shared result field a one-dispatch lifetime - "the
/// combat walker replaces the whole field with zero before the next actor
/// dispatch" - and a round walk visits several slots before any transcript
/// is assembled, so this stage only honours the verdict recorded then.
pub(crate) fn combat_monster_attack_narrated_result_message(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
    narration: CombatMonsterAttackNarration,
) -> Option<String> {
    if attack.generic_chain_suppressed {
        return None;
    }
    match narration {
        CombatMonsterAttackNarration::SelfActingHostile => {
            combat_monster_attack_result_message(state, attack)
        }
        CombatMonsterAttackNarration::Controlled => {
            combat_controlled_monster_attack_result_message(state, attack)
        }
    }
}

/// Which of `combat.md §11.1`'s two producers narrates one resolved
/// monster-side attack. The section's scope note is explicit that the
/// choice is not made by the actor: "*party-side helper* describes the
/// routine, not the actor - Section 6.1a's controlled bit lets a monster
/// reach it and lets a party member bypass it."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CombatMonsterAttackNarration {
    /// The `§9` automatic actor driver's own turn, which "calls the shared
    /// helpers directly and passes through none of" the announcement layer.
    SelfActingHostile,
    /// A monster descriptor control moved to group 0, narrating the attempt
    /// it took at the **player's prompt** (`§16.1`, `RETRACTIONS.md` R377).
    Controlled,
}

/// `combat.md §11.1` "The census": the party-side half of one attack
/// outcome's printed result line. Internal slots, coordinates, rolls and
/// raw damage never belong in this string.
///
/// Two rules from that section govern every string here. Rule 1: "**Every
/// result line names the target, never the attacker.** ... `Bat missed!`
/// ... is printed by a party member's failed swing **at** a Bat, never by
/// the Bat's failed swing at the party. An engine that prints the
/// attacker's name in the miss line produces a transcript that is wrong on
/// every line it emits." Rule 2: the two sides "share the to-hit roll, the
/// impact presentation, the damage roller and the result narrator".
///
/// This is the producer alone, without `§6.3`'s narrator gate; the
/// dispatch prints [`combat_weapon_attack_narrated_result_message`]
/// instead. Only conformance tests that assert on the unsuppressed census
/// rows call it directly.
#[cfg(test)]
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
    combat_attack_result_narration(state, target_slot, attacker_slot, attack).into_message()
}

/// One attack outcome's printed lines, split where `combat.md §6.3`'s
/// narrator gate cuts.
///
/// `§11.1` puts two different things into one transcript: lines printed
/// "**inside** the damage roll" - the Glass Sword row's `Thy sword hath
/// shattered!` - and the result line the shared narrator emits after it.
/// `§6.3` gives the gate only the second of those: while `0x02` stands
/// the narrator "skips the generic killed/slept/hit chain, produces no
/// message or sound", and it says nothing about the damage roll's own
/// prints. Keeping the two apart is what lets a Glass Sword kill of a
/// vanish-class monster read `<monster> vanishes!` then `Thy sword hath
/// shattered!` without also printing `<monster> killed!` beside it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CombatAttackResultNarration {
    /// Printed inside the damage roll, ahead of the result narrator.
    inside_damage_roll: Option<String>,
    /// The result line, when this outcome narrates one.
    result: Option<String>,
    /// Whether [`Self::result`] is the generic killed/slept/hit/wound
    /// chain's line - the only line the gate withholds.
    result_reaches_generic_chain: bool,
}

impl CombatAttackResultNarration {
    fn into_message(self) -> Option<String> {
        match (self.inside_damage_roll, self.result) {
            (Some(inside), Some(result)) => Some(format!("{inside}\n{result}")),
            (Some(line), None) | (None, Some(line)) => Some(line),
            (None, None) => None,
        }
    }
}

fn combat_attack_result_narration(
    state: &PlayState,
    target_slot: usize,
    attacker_slot: Option<usize>,
    attack: CombatWeaponAttackApplication,
) -> CombatAttackResultNarration {
    let result_reaches_generic_chain =
        combat_weapon_resolution_reaches_generic_chain(attack.resolution);
    let target_name = combat_actor_display_name(state, target_slot);
    let (inside_damage_roll, result) = match attack.resolution {
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
        CombatWeaponAttackResolution::Miss { .. } => (None, Some(format!("{target_name} missed!"))),
        CombatWeaponAttackResolution::Hit { .. } => (
            None,
            match attack.damage_application {
                Some(application) => {
                    combat_landed_damage_result_line(state, target_slot, attacker_slot, application)
                }
                None => Some(format!("{target_name} hit!")),
            },
        ),
        // `combat.md §11.1` census: "Glass Sword swing | party melee |
        // `Thy sword hath shattered!`, printed **inside** the damage roll,
        // so it lands between the hit newline and the result line". The
        // ordinary result line then follows - for the sentinel that is
        // "Target dies | both | `<target> killed!`" - so the shatter line
        // is a prefix, not a replacement, and it sits on the damage roll's
        // side of `§6.3`'s narrator gate rather than the narrator's.
        CombatWeaponAttackResolution::Special { shattered, .. } => (
            shattered.then(|| COMBAT_GLASS_SWORD_SHATTER_LINE.to_string()),
            attack.damage_application.and_then(|application| {
                combat_landed_damage_result_line(state, target_slot, attacker_slot, application)
            }),
        ),
        CombatWeaponAttackResolution::OutOfRange { .. }
        | CombatWeaponAttackResolution::NoOrdinaryDamage { .. } => (None, None),
    };
    CombatAttackResultNarration {
        inside_damage_roll,
        result,
        result_reaches_generic_chain,
    }
}

/// `combat.md §11.1` census rows for a landed swing, in the order the
/// section's "Order, stated once" list gives them: the graze arm first
/// (it "suppresses every later result line"), then the death arm, then
/// the ordinary hit.
pub(crate) fn combat_landed_damage_result_line(
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
                // `combat_frame.rs`, and so is that suppression: `§6.3`
                // step 2 has the vanish branch "replace the shared combat
                // action-result/narration field with `0x02`", and the
                // common narrator skips the generic chain "when `0x02` is
                // still present". This producer therefore keeps writing the
                // line and [`combat_apply_attack_narrator_gate`] (party
                // side) / `PlayState::apply_combat_monster_attack_narrator_gate`
                // (monster side) withholds it, because `§6.3` publishes one
                // case where it must *not* be withheld: when the faint
                // tail's "sleep helper replaces the whole result field with
                // sleep bit `0x04`, losing `0x02`", the narrator "appends
                // the vanished target's `<name> killed!` line - not
                // `<name> slept!` - after the faint tail".
                return Some(format!("{class_name} killed!"));
            }
            // `combat.md §11.1` "The graded wound lines are monster-target
            // only": the result line "is graded by the target's remaining
            // HP against its class maximum, using the same four-bucket
            // wound score the flee classifier of Section 9 computes".
            let actor = state.combat_actors.get(target_slot)?;
            let max_hp = combat_class_stats(damage.class)?.max_hp;
            let wound_line = combat_monster_wound_line_grade(actor.hp_or_wound, max_hp);
            Some(format!("{class_name} {wound_line}!"))
        }
    }
}

/// `combat.md` 11.1, "The graded wound lines are monster-target only". The
/// four strings are published verbatim there against the same four-bucket wound
/// score the flee classifier of 9 computes:
///
/// | 1 | below one quarter | `<target> critical!` |
/// | 2 | one quarter to just under one half | `<target> heavily wounded!` |
/// | 3 | one half to just under three quarters | `<target> lightly wounded!` |
/// | 4 | three quarters or more | `<target> barely wounded!` |
///
/// `grazed!` is **not** one of them: 11.1 reserves that line for the separate
/// zero-or-negative-damage outcome, and 12 narrates it "and nothing else".
pub(crate) fn combat_monster_wound_line_grade(current_hp: u8, max_hp: u8) -> &'static str {
    match combat_wound_score_bucket(current_hp, max_hp) {
        CombatWoundScoreBucket::ThreeQuartersOrMore => "barely wounded",
        CombatWoundScoreBucket::HalfToUnderThreeQuarters => "lightly wounded",
        CombatWoundScoreBucket::OneQuarterToUnderHalf => "heavily wounded",
        CombatWoundScoreBucket::UnderOneQuarter => "critical",
    }
}

pub(crate) fn combat_actor_display_name(state: &PlayState, slot: usize) -> String {
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

/// `combat.md §11.1` census: "Party target poisoned | monster attacker |
/// `<target> is poisoned!`, printed **inside** damage resolution - after
/// the hit newline and before the result line - and the ordinary result
/// line is then suppressed". The row is keyed on the attacker being a
/// monster, not on which driver dispatched it, so both monster-side
/// producers below start here.
fn combat_monster_attack_poison_line(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
) -> Option<String> {
    matches!(
        attack.poison_status_outcome,
        Some(CombatPoisonStatusAttackOutcome::PoisonedPartyMember { .. })
    )
    .then(|| {
        format!(
            "{} is poisoned!",
            combat_actor_display_name(state, attack.target_slot)
        )
    })
}

/// The shared result narrator, entered from a monster-side attack.
/// `combat.md §11.1` rule 2: below the announcement layer both sides
/// "share the to-hit roll, the impact presentation, the damage roller and
/// the result narrator". The attacker slot travels with the call because
/// one census row reads it: "Ordinary landed hit, **party** target,
/// attacker is a **Corpser** (class 45) | monster attacker | `<target>
/// dragged under!` in place of `hit!`".
fn combat_monster_attack_shared_result_message(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
) -> Option<String> {
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

/// `combat.md §11.1` for a self-acting hostile's turn: the monster-side
/// half of the published attack-outcome census. The two sides "join
/// *below* the announcement layer, which is why an ordinary hostile
/// monster prints no banner, no `Attack-`, no `Aim! `, no `Nothing!` and,
/// on a melee miss, no line at all" (`RETRACTIONS.md` R353); below that
/// layer it shares the impact presentation, the damage roller and the
/// result narrator with the party side.
pub(crate) fn combat_monster_attack_result_message(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
) -> Option<String> {
    // `combat.md §11` (`RETRACTIONS.md` R361), the Gremlin food-theft
    // branch: on acceptance it prints "a newline, `A `, the acting
    // creature's name, and ` stole some food!` with its own trailing
    // newline - i.e. `A <monster> stole some food!` on its own row", and
    // "**Consume the attack action and return.** The branch replaces the
    // entire damage and narration chain ... no result line". The line is
    // returned bare, like every sibling line in this narrator: the caller's
    // `append_combat_result_line` is what supplies the leading newline and
    // the trailing one, so embedding them here would double both.
    // The two rejecting outcomes fall through to ordinary resolution and
    // narrate nothing of their own.
    if matches!(
        attack.food_theft,
        Some(CombatFoodTheftOutcome::Stole { .. })
    ) {
        let acting_creature_name = combat_actor_display_name(state, attack.attacker_slot);
        return Some(format!(
            "{COMBAT_FOOD_THEFT_LINE_LEAD}{acting_creature_name}{COMBAT_FOOD_THEFT_LINE_TAIL}"
        ));
    }
    // `combat.md §11.1`: "Target slept | both | `<target> slept!` | none"
    // (`RETRACTIONS.md` R359 corrects the row's former "slept or stoned").
    // The message "is not printed by the sleep routine itself: the narration
    // code it sets is what makes the shared result narrator print `<target>
    // slept!` on the caller's next narration step".
    if matches!(
        attack.sleep_effect,
        Some(CombatSleepEffectOutcome::PartyMemberSlept { .. })
            | Some(CombatSleepEffectOutcome::NonPartyDisabled)
    ) {
        return Some(format!(
            "{} slept!",
            combat_actor_display_name(state, attack.target_slot)
        ));
    }
    if let Some(line) = combat_monster_attack_poison_line(state, attack) {
        return Some(line);
    }

    // 11.1, the census row that carries the whole section: "**To-hit fails** |
    // **monster melee** | **nothing at all**", and in the prose, "**an ordinary
    // hostile monster's melee miss prints nothing and sounds nothing** - no
    // newline, no name, no line, no tone". The reason is structural: "the
    // routine that prints a miss line has exactly two call sites, both inside
    // party-side attack helpers".
    //
    // Two things this must not become. It must not print an attacker-named
    // line: rule 1 of 11.1 is that "**Every result line names the target, never
    // the attacker**", and `<attacker> missed!` "produces a transcript that is
    // wrong on every line it emits". And it must not be widened to every
    // monster attacker - 11.1's announcement table gives a monster carrying the
    // controlled/charmed bit (6.1a) "the **reduced** banner ... then one fixed
    // attempt: `Attack-`, `Aim! `, and on a failed roll `<target> missed!`",
    // because that slot is driven from the player's prompt.
    //
    // The ranged carve-out is **not** folded in here. 11.1: "A monster's failed
    // **ranged or thrown** to-hit is not unconditionally silent ... the impact
    // point is drawn from the three-by-three neighbourhood centred on the **aim
    // cell** ... and the **full hit chain runs against that actor**." This
    // engine does not model that scatter, so the miss arm stays silent on both
    // routes; the gap is a missing hit chain against a scatter victim, never a
    // miss line, so nothing here would print on either reading.
    if matches!(
        attack.resolution,
        Some(CombatWeaponAttackResolution::Miss { .. })
    ) {
        // §11.1's controlled-monster carve-out is not reached here: that
        // slot narrates through
        // [`combat_controlled_monster_attack_result_message`] instead, on
        // §11.1's own scoping - "*party-side helper* describes the routine,
        // not the actor - Section 6.1a's controlled bit lets a monster reach
        // it". The dispatch that picks between the two is `§16.1`'s, in
        // `PlayState::apply_combat_actor_slot_dispatch`.
        return None;
    }
    combat_monster_attack_shared_result_message(state, attack)
}

/// `combat.md §11.1` for a monster carrying the controlled/charmed bit.
/// Its announcement table gives that turn as the "**reduced** banner ...
/// then, **only if the player presses `A`**, one fixed attempt: `Attack-`,
/// then `Aim! ` for a class whose reach selector is one, and on a failed roll
/// `<target> missed!`". The turn is prompted, not synthesized
/// (`RETRACTIONS.md` R377).
///
/// So this arm differs from the self-acting hostile's in exactly one row -
/// the miss - and §11.1 says why the difference is a difference of
/// *producer*, not of string: "Note the scoping: *party-side helper*
/// describes the routine, not the actor - Section 6.1a's controlled bit
/// lets a monster reach it and lets a party member bypass it." Every other
/// row, the graded wound lines of "The graded wound lines are
/// monster-target only" included, is the shared narrator's, reached
/// through the same [`combat_attack_result_message`] the party's own swing
/// uses; §11.1 rule 1 then holds on every line this can emit, because that
/// routine names the target and never the attacker.
pub(crate) fn combat_controlled_monster_attack_result_message(
    state: &PlayState,
    attack: CombatMonsterAttackApplication,
) -> Option<String> {
    if let Some(line) = combat_monster_attack_poison_line(state, attack) {
        return Some(line);
    }
    combat_monster_attack_shared_result_message(state, attack)
}

pub(crate) fn append_combat_round_walk_messages(
    state: &mut PlayState,
    application: &CombatRoundWalkApplication,
) {
    let attacks = application
        .applications
        .iter()
        .filter_map(|entry| match entry {
            CombatActorSlotDispatchApplication::Slot {
                action:
                    CombatActorDispatchAction::MonsterAi {
                        ai_turn: Some(ai_turn),
                    },
                ..
            } => Some((
                ai_turn.monster_attack?,
                CombatMonsterAttackNarration::SelfActingHostile,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    // `combat.md §6.3`: the narrator gate already ran inside each attack's
    // own dispatch, while the shared result field still described that
    // slot; this loop only prints what the gate let through.
    for (attack, narration) in attacks {
        if let Some(line) = combat_monster_attack_narrated_result_message(state, attack, narration)
        {
            // `combat.md §11.1`, the order stated once: step 3 is "a newline",
            // step 5 the result line - which carries its own trailing newline.
            state.emit_combat_print(&format!("\n{line}\n"));
        }
    }
}

fn drive_combat_round_walk_and_append_message(state: &mut PlayState) {
    if !state.combat_active || state.pending_combat_actor_slot.is_some() {
        return;
    }

    for _ in 0..COMBAT_ROUND_WALK_DRAIN_LIMIT {
        let start_slot = state.next_combat_actor_slot.min(COMBAT_ACTOR_SLOTS);
        let application = if state.pace_combat_presentations {
            state.apply_combat_round_walk_from_slot_paced(start_slot, COMBAT_PHASE_REFRESH_CONSTANT)
        } else {
            state.apply_combat_round_walk_from_slot(start_slot, COMBAT_PHASE_REFRESH_CONSTANT)
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
    ///
    /// Measured 2026-09-07 (`qa/paired/shop-arms-menus.tsv`): each line ends
    /// with the closing double quote of the speech the listing heading
    /// opened, which the published transcription omits.
    #[test]
    fn arms_stock_call_pool_is_the_published_four_with_their_closing_quote() {
        assert_eq!(arms_stock_call_for_roll(0), "What may I show thee?\"");
        assert_eq!(
            arms_stock_call_for_roll(1),
            "Which wouldst thou like to see?\""
        );
        assert_eq!(arms_stock_call_for_roll(2), "What is thine interest?\"");
        assert_eq!(arms_stock_call_for_roll(3), "Which would ye see?\"");
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
    fn arms_buy_entry_prints_the_stock_list_then_the_drawn_call() {
        let mut state = stocked_arms_state();

        // Take the three draws the buy-entry arm is about to make from a
        // clone, so the assertion knows which member of each pool is correct.
        // `§8.B` orders them: affirmation, stock introduction, then `§8.1`'s
        // call line.
        let mut probe = state.clone();
        let expected_heading =
            arms_buy_menu_heading(probe.random_range_u8(0, 3), probe.random_range_u8(0, 3));
        let expected_call = arms_stock_call_for_roll(probe.random_range_u8(0, 3));

        handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();

        // Measured 2026-09-07 at Iolo's Bows (`qa/paired/shop-arms-menus.tsv`):
        // heading, blank row, one row per stock letter, blank row, call line.
        let lines: Vec<&str> = state.message.lines().collect();
        let mut expected_heading_lines = expected_heading.lines();
        assert_eq!(Some(lines[0]), expected_heading_lines.next());
        assert_eq!(Some(lines[1]), expected_heading_lines.next());
        assert!(ARMS_BUY_AFFIRMATIONS.contains(&&lines[0][1..]));
        assert!(ARMS_BUY_STOCK_INTRODUCTIONS.contains(&lines[1]));
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "a...Short Sword");
        assert_eq!(lines.last(), Some(&expected_call));
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
        let call_after_entry = state.message.lines().last().unwrap().to_string();

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
        // `§8.B`: "These two draws occur once per accepted Buy entry.
        // Repeated item listings do not redraw either heading." The engine
        // reprinted a fixed affirmation/introduction pair here.
        assert!(
            state.message.starts_with("a...Short Sword"),
            "the redraw re-renders the stock rows without the heading: {:?}",
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
        assert_eq!(
            arms_post_item_prompt(true, true),
            "\"Anything else, milady?"
        );
        assert_eq!(arms_post_item_prompt(false, true), "\"Anything else, sir?");
        assert_eq!(
            arms_post_item_prompt(false, false),
            "\"Anything else, then?"
        );
        assert_eq!(arms_post_item_prompt(true, false), "\"Anything else, then?");
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
        assert_eq!(rendered, "Sold!\n\"Anything else, sir?");
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
\"Anything else, milady?"
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
\"Anything else, sir?"
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
