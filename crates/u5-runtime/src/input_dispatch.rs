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
    run_with_command_epilogue(state, game_dir, |state| {
        handle_play_key_input_inner(state, key, suffix, game_dir)
    })
}

/// `combat.md §8.1` and `§14`, for the turns combat takes **without** a key.
///
/// The round walker reads a key only for a keyboard-driven combatant that
/// can act: a sleeping one gets its banner, its asleep arm's wake draw and
/// `Zzzzz...` "and the turn ends with no key read", and "a wipe that leaves
/// a sleeping member instead simply passes turns until someone wakes or
/// dies". `main-loop.md §6`: "combat has no such [party-capability] check of
/// its own", so the exploration gate's world sleep pass must never stand in
/// for those turns (playtest F24).
///
/// One call drives the walker exactly as a committed action's tail does, up
/// to the next prompted combatant or the end of the fight, and then runs the
/// same post-command hooks a key would - a fight that ends here still drains
/// its town tail and outdoor reactions.
pub fn advance_combat_without_input(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if !combat_awaits_automatic_turns(state) {
        return Ok(());
    }
    run_with_command_epilogue(state, game_dir, |state| {
        drive_combat_round_walk_and_append_message(state);
        Ok(PlayInputDisposition::Continue)
    })
    .map(|_| ())
}

/// Combat is running and no combatant is waiting at the prompt, so the
/// walker owes turns that read no key.
pub(crate) fn combat_awaits_automatic_turns(state: &PlayState) -> bool {
    state.combat_active
        && state.pending_combat_actor_slot.is_none()
        && state.active_combat_targeting.is_none()
}

fn run_with_command_epilogue(
    state: &mut PlayState,
    game_dir: &Path,
    command: impl FnOnce(&mut PlayState) -> io::Result<PlayInputDisposition>,
) -> io::Result<PlayInputDisposition> {
    // `stats-panel.md §2.2`: every site that changes a displayed number
    // either repaints the panel itself or raises the deferred request, and
    // which one is a property of the site. Immediate is the default because
    // the census makes it the majority; a handler that is one of the
    // exceptions sets its own mode while it runs.
    state.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Immediate;
    state.stats_panel_command_baseline = crate::stats_panel::stats_panel_snapshot_of(state);
    let mut combat_was_active = state.combat_active;
    let mut result = command(state);
    if result.is_ok() {
        if let Err(error) = settle_dungeon_acted_charge(state, game_dir) {
            result = Err(error);
        }
    }
    // `combat.md §8.1`: the turn banner is "emitted at the start of every
    // keyboard-driven combatant's turn, *before any key is read*". A
    // committed combat action already drives the walker to the next prompt
    // before it returns; the command that *opens* a fight did not, so the
    // first banner only appeared together with the next key's echo
    // (playtest F35). Drive it here, once the entry framing has printed.
    // The paced (graphical) shell walks the same turns from its own pump.
    if result.is_ok()
        && !combat_was_active
        && combat_awaits_automatic_turns(state)
        && !state.pace_combat_presentations
    {
        combat_was_active = true;
        drive_combat_round_walk_and_append_message(state);
    }
    // A generic adjacent terrain combat suspends the high-to-low outdoor
    // reaction walk. As soon as the combat frame returns, continue the
    // remaining lower slots before accepting another world command.
    // `dungeon-mode.md §14.1` (u5-spec#298 Q1): a post-combat code 5 or 6
    // past the level stack takes the `§13.2` surface exit.
    if result.is_ok() && !state.combat_active && state.pending_dungeon_surface_exit.is_some() {
        if let Err(error) = state.drain_pending_dungeon_surface_exit(game_dir) {
            result = Err(error);
        }
    }
    // `town-mode.md §14`: drain the town NPC-conflict chain's exit -
    // "On exit the town chain clears the NPC slot, reloads the town map,
    // and re-runs the Shadowlord install pass of Section 13".
    let town_attack_epilogue_owed = state
        .pending_town_conflict
        .is_some_and(|pending| pending.owes_turn_epilogue);
    let mut town_conflict_drained = false;
    if result.is_ok() && !state.combat_active && state.pending_town_conflict.is_some() {
        match state.drain_pending_town_conflict(game_dir) {
            Ok(drained) => town_conflict_drained = drained,
            Err(error) => result = Err(error),
        }
    }
    // `town-mode.md §7` step 4: the town turn's walker and schedule tail runs
    // after the command handler returns. It is drained only on the key that
    // ended a fight, so a town command that leaves the tail deliberately owed
    // is untouched. A town A-Attack's whole epilogue - minute, underfoot,
    // walkers, schedule, post-action cleanup - runs here once, after the
    // fight and the conflict chain's reload (issue #298 Q40).
    if result.is_ok() && combat_was_active && !state.combat_active {
        state.drain_town_tail_after_combat();
    }
    if result.is_ok() && town_attack_epilogue_owed && town_conflict_drained {
        if let Err(error) = state.run_town_attack_epilogue_after_combat(game_dir) {
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
    // `combat.md §8.1`: the turn banner is printed "before any key is
    // read". A command that *started* a fight - a step onto a dungeon room
    // trigger, a camp ambush, a Doom-mouth ambush - left the round walker
    // un-run, so the banner only appeared once the next key arrived and was
    // followed straight away by that key's echo (playtest F09). It runs
    // after the slot flush so the banner lands under the entry narration.
    // Only a command that framed the fight owes it: inside combat the
    // handlers open the next turn themselves, and an actor whose sub-prompt
    // (a spell name, an Attack aim) is still open has no pending slot either.
    if result.is_ok() && !combat_was_active {
        open_owed_combat_player_turn(state);
    }
    if result.is_ok() {
        if let Err(error) = run_overworld_loop_head_hooks(state, game_dir) {
            result = Err(error);
        }
    }
    if result.is_ok() {
        if let Err(error) = run_dungeon_entry_underfoot_reaction(state, game_dir) {
            result = Err(error);
        }
    }
    // The command's own panel mechanism, applied where the command ends.
    //
    // Not where the *change* happened, which is where §2.2's immediate sites
    // paint. That distinction is only observable to a capture taken between
    // the change and the end of the command - which means inside a hold, and
    // the holds that matter are named in the section. Those get their own
    // explicit `repaint_stats_panel()` at the right step rather than relying
    // on this.
    // A command that changed none of the panel's numbers touches the panel
    // at all: the sites in `stats-panel.md §2.2`'s census are sites that
    // *changed* something. Without this test every command would refresh,
    // and the wishing well's debit - which its own arm declines to paint -
    // would appear on the panel at the very next command anyway. The census'
    // "whole panel" immediate sites (`§2.2.1`: U-Use after the pick, Stats
    // browser exit, R Ready end, dungeon post-action redraw, combat entry
    // and exit) repaint unconditionally at their own step instead, which is
    // §2.4's "any inline repaint" that does reveal a stale debit.
    let panel_changed =
        crate::stats_panel::stats_panel_snapshot_of(state) != state.stats_panel_command_baseline;
    match state.stats_panel_command_refresh {
        crate::stats_panel::PanelRefreshMode::Always => state.repaint_stats_panel(),
        crate::stats_panel::PanelRefreshMode::Immediate if panel_changed => {
            state.repaint_stats_panel()
        }
        crate::stats_panel::PanelRefreshMode::Deferred if panel_changed => {
            state.request_stats_panel_refresh()
        }
        _ => {}
    }
    result
}

/// `commands.md §3` / `dungeon-mode.md §4` step 6: once a dungeon letter
/// that reports the default "acted" has closed its last prompt, charge the
/// turn its handler did not take and run the dungeon post-action pass - the
/// 1-in-16 wake roll and the active monster's step - before the next key is
/// read (playtest J08). The dungeon loop advanced the minute at its head
/// ([`PlayState::advance_dungeon_loop_minute`]), so the charge adds no
/// second minute. A fight or a scene change ends the obligation.
fn settle_dungeon_acted_charge(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    let Some(turn_before) = state.dungeon_acted_turn_before else {
        return Ok(());
    };
    if !state.combat_active && !next_key_is_a_command(state) {
        return Ok(());
    }
    state.dungeon_acted_turn_before = None;
    if state.combat_active
        || !matches!(state.area, Area::Dungeon { .. })
        || state.turn != turn_before
    {
        return Ok(());
    }
    state.advance_turn();
    state.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
    Ok(())
}

/// Whether the next key the shell reads is an ordinary command key: no
/// prompt, session, hold or cinematic owns it. Mirrors the gates that
/// [`handle_play_key_input_inner`] tests before its command dispatch.
fn next_key_is_a_command(state: &PlayState) -> bool {
    state.pending_text_hold.is_none()
        && state.endgame.is_none()
        && state.pending_blackthorn_rescue.is_none()
        && !state.cinematic_hold_owns_input()
        && state.active_blackthorn.is_none()
        && state.codex_presentation_steps == 0
        && state.active_yes_no_prompt.is_none()
        && state.active_direction_prompt.is_none()
        && state.active_cast_followup.is_none()
        && state.active_cast.is_none()
        && state.active_rest.is_none()
        && state.active_jimmy.is_none()
        && state.active_surface_chest.is_none()
        && state.active_shrine.is_none()
        && state.active_shrine_restoration.is_none()
        && state.active_mix.is_none()
        && state.active_new_order.is_none()
        && state.active_wishing_well.is_none()
        && state.active_yell.is_none()
        && state.active_view_overlay.is_none()
        && state.active_ready.is_none()
        && state.active_use.is_none()
        && state.active_party_selector.is_none()
        && state.active_z_stats.is_none()
        && state.active_shop.is_none()
        && state.active_conversation.is_none()
        && state.active_blackthorn_guard_demand.is_none()
}

/// `combat.md §8.1`: "**Short re-prompt, banner not reprinted:** an
/// unrecognised key (`What?`), the two toggles Ctrl-S and Ctrl-B, and a
/// refused step or refused climb" - "The handler emits one newline,
/// refreshes the prompt window ... and reads another key", so the next
/// echo keeps `text-output.md §10.4`'s derived blank row, as the arena's own
/// `What?` already did. The toggles and a function key are answered before
/// the arena's parser sees them, so they mark the short form here (playtest
/// H24).
fn mark_combat_short_reprompt(state: &mut PlayState) {
    if state.combat_active {
        state.combat_prompt_row_opened_by_banner = false;
    }
}

/// Whether a function key arrives at the command prompt rather than at an
/// open prompt: [`next_key_is_a_command`], and neither the town arrest
/// prompt nor the combat target cursor is waiting on the key.
fn function_key_reaches_the_command_dispatcher(state: &PlayState) -> bool {
    next_key_is_a_command(state)
        && state.pending_town_arrest.is_none()
        && state.active_combat_targeting.is_none()
}

/// Whether an open prompt reads a typed line through the free-text reader
/// (`input.md §8`) and takes each key it is handed as text: the conversation
/// keyword and ASK-WHO prompts, Yell, the Blackthorn answer row, the wishing
/// well's wish and the sage's topic. The shrine answers filter their own keys.
fn free_text_line_reader_open(state: &PlayState) -> bool {
    state.active_conversation.is_some()
        || state.active_yell.is_some()
        || state.blackthorn_prompt_echo().is_some()
        || state
            .active_wishing_well
            .as_ref()
            .is_some_and(|session| session.coin_accepted)
        || matches!(
            state.active_shop.as_ref(),
            Some(crate::shop_session::ActiveShopSession::Sage(
                crate::shop_runtime::SageState::Prompt { .. }
            ))
        )
}

/// The overworld loop's head, between one command and the next key read.
///
/// `overworld.md §9.2`: "The overworld command loop has a live-gate entry
/// hook that runs before normal input dispatch, on every iteration", and on
/// `0xDC` it runs the transit "before the party is relocated and before any
/// key is read". A step onto a gate therefore transits at once; running the
/// hook only when the next key arrived made the transit wait for, and
/// swallow, that key (playtest E25).
pub fn run_overworld_loop_head_hooks(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if !matches!(state.area, Area::World { .. })
        || state.combat_active
        || !next_key_is_a_command(state)
    {
        return Ok(());
    }
    // The loop comes straight back to the hook after a transit, still
    // without reading a key, so an arrival on another live gate transits
    // again. Each transit restores its own gate cell, so this ends; the
    // bound only guards a fixture that never clears one.
    for _ in 0..MOONSTONE_SLOT_COUNT {
        if state.resolve_natural_moongate_entry(game_dir)?.is_none() {
            break;
        }
        if !matches!(state.area, Area::World { .. }) || !next_key_is_a_command(state) {
            break;
        }
    }
    Ok(())
}

/// `dungeon-mode.md §4`, Initialisation and the underfoot reaction: dungeon
/// mode reads the underfoot tile when it is entered and, "if the cached high
/// nibble is room-helper state (`0xA`) or room trigger (`0xF`), run[s] the
/// room-entry helper" - before the inner loop reads a key. `§5`: these
/// effects "fire before the player can act".
///
/// The engine only resolved a room cell at the head of the *next* key's
/// dispatch, so a surface entry onto Shame's or Doom's `0xF0` entry cell, or
/// a save loaded onto one, waited at an ordinary prompt; the next key was
/// echoed, did nothing, and only then did `Entering room...` print (playtest
/// F08). This runs where the original's initialisation would: after the
/// command that brought the party into dungeon mode, and from the
/// exploration gate before the first key after a load.
///
/// Returns whether a room was entered.
pub fn run_dungeon_entry_underfoot_reaction(
    state: &mut PlayState,
    game_dir: &Path,
) -> io::Result<bool> {
    if !matches!(state.area, Area::Dungeon { .. })
        || state.combat_active
        || !next_key_is_a_command(state)
    {
        return Ok(false);
    }
    if state
        .resolve_current_dungeon_room_trigger(Some(game_dir))?
        .is_none()
    {
        return Ok(false);
    }
    state.flush_message_slot();
    open_owed_combat_player_turn(state);
    Ok(true)
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
    // `conversation.md §7.3`: a key that arrives during a `0x83` pause "is
    // consumed and ends the pause at once", and `0x8F` blocks for "exactly
    // one command keystroke (any key; its value is discarded)". Whatever the
    // key is, it belongs to the hold, not to a command.
    if state.consume_text_hold_key() {
        return Ok(PlayInputDisposition::Continue);
    }
    if let Some(byte) = input_byte_from_char(key) {
        // `input.md §5`: nothing consumes the function-key block - "no mode
        // loop, no letter dispatcher, no prompt, and no panel". The `What?`
        // below is the command dispatcher's answer to an unassigned key, so
        // it belongs to the command prompt alone. Under an open prompt the
        // key goes to that prompt, whose own rule for a key it does not
        // accept applies - discarded and read again (`§8`, `§10`), or the
        // `N` an Exit-to-DOS answer prints (`commands.md §9`). Answering
        // `What?` there drew a command marker over a prompt that was still
        // live, and the next `Y` quit the game (playtest G13).
        if matches!(input_byte_class(byte), InputByteClass::FunctionKey)
            && function_key_reaches_the_command_dispatcher(state)
        {
            // Measured 2026-09-07: a function key is just an unassigned
            // key, and answers the resident `What?` like any other.
            //
            // "Like any other" includes *how* the row is drawn.
            // `commands.md §5.2` lists "any unmapped key | `What?\n`" in the
            // verb-echo table, so the row carries the command end cap and the
            // blank above it. Writing the slot directly made it an ordinary
            // result line instead. Measured 2026-09-11 (`stray-keys/fkey`),
            // where the original draws `?What?` and this engine drew a bare
            // `What?` with no separating row.
            state.emit_command_echo_line(unassigned_refusal_echo(0));
            mark_combat_short_reprompt(state);
            return Ok(PlayInputDisposition::Continue);
        }
        // `input.md §8`: the free-text reader discards "Any other byte
        // (function key, direction code, etc.)". These readers build their
        // line from whatever key they are handed, so the code would
        // otherwise be taken as typed text - a yelled or wished-for word, a
        // conversation keyword, a Blackthorn answer.
        if matches!(input_byte_class(byte), InputByteClass::FunctionKey)
            && free_text_line_reader_open(state)
        {
            return Ok(PlayInputDisposition::Continue);
        }
    }
    if state.endgame.is_some() {
        return handle_endgame_key_input(state, key, suffix, game_dir);
    }
    // `blackthorn.md §7` step 19, the rescue cinematic's only blocking key
    // read: "any key satisfies it, no typed character is echoed, and the
    // returned key is discarded rather than interpreted as a command".
    if state.blackthorn_rescue_awaiting_acknowledgement() {
        let _ = state.step_blackthorn_rescue(game_dir, true)?;
        // It is an ordinary read (it "uses the ordinary command reader"), so
        // `input.md §6` applies: with type-ahead off "the queue is emptied on
        // every read". Keys typed during the narration must not run as
        // commands after the hand-off (playtest N21).
        if !state.typeahead_buffer_enabled {
            state.keys_buffered_during_hold.clear();
        }
        return Ok(PlayInputDisposition::Continue);
    }
    // Every other cinematic phase is a hold that polls no keyboard. What is
    // typed during one is not lost - see
    // [`PlayState::keys_buffered_during_hold`].
    if state.cinematic_hold_owns_input() {
        state
            .keys_buffered_during_hold
            .push_back((key, suffix.to_string()));
        return Ok(PlayInputDisposition::Continue);
    }
    if state.active_blackthorn.is_some() {
        return handle_active_blackthorn_key_input(state, key, suffix, game_dir);
    }
    // `karma.md §8.2`: the Codex reader owns the keyboard until it has
    // stepped through its records, one record per key. Measured 2026-09-09
    // (`qa/paired/codex-enter.tsv`): with nothing ordained the fourth key
    // after E-Enter is an ordinary command again.
    if state.step_codex_presentation(game_dir)? {
        return Ok(PlayInputDisposition::Continue);
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
        return handle_active_mix_key_input(state, key, suffix, game_dir);
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
        let disposition = handle_active_ready_key_input(state, key, suffix);
        settle_exploration_acted_charge(state, game_dir)?;
        return Ok(disposition);
    }
    if state.active_use.is_some() {
        return handle_active_use_key_input(state, key, suffix, game_dir);
    }
    if state.active_party_selector.is_some() {
        let search_turn_before = state
            .active_party_selector
            .is_some_and(|session| matches!(session.target, PartySelectorTarget::Search { .. }))
            .then_some(state.turn);
        let cast_turn_before = state
            .active_party_selector
            .is_some_and(|session| matches!(session.target, PartySelectorTarget::Cast))
            .then_some(state.turn);
        state.step_active_party_selector_with_game_dir(key, suffix, game_dir);
        // `commands.md §3`/§7: a Search that reached the acting-member
        // picker reports "acted" whether the pick searched or cancelled.
        if let Some(turn_before) = search_turn_before {
            if state.active_party_selector.is_none() {
                state.charge_tile_command_acted_exit(turn_before);
                state.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
            }
        }
        // `commands.md §3` / `magic.md §5` step 1: `Player: None!` at Cast's
        // acting-member picker is a C path, so it reports acted too
        // (playtest S03, S13). A pick opens the spell prompt instead, and
        // the cast charges when that closes.
        if let Some(turn_before) = cast_turn_before
            && state.active_party_selector.is_none()
            && state.active_cast.is_none()
            && !state.combat_active
            && state.turn == turn_before
        {
            state.charge_cast_refusal_turn();
            state.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
        }
        settle_exploration_acted_charge(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.active_z_stats.is_some() {
        let disposition = handle_active_z_stats_key_input(state, key, suffix);
        settle_exploration_acted_charge(state, game_dir)?;
        return Ok(disposition);
    }
    if state.active_shop.is_some() {
        let disposition = handle_active_shop_key_input(state, key, suffix, game_dir);
        // `town-mode.md §7` step 4 / `shops.md §2`: the explicit Talk that
        // opened this shop "receives normal town clock/scheduler processing"
        // once the shop arm returns (playtest J16).
        if state.active_shop.is_none() && std::mem::take(&mut state.pending_talk_shop_turn) {
            let turn_before = state.turn;
            state.advance_turn();
            state.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
        }
        return Ok(disposition);
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
    // `overworld.md §9.2`: the live-gate hook runs "before normal input
    // dispatch, on every iteration", and "before any key is read". The
    // ordinary place this engine runs it is the end of the previous command
    // ([`run_overworld_loop_head_hooks`]); this catches a party that reached
    // the command loop on a live gate any other way (a load, or a closed
    // session). The transit consumes no key: once it has run, the key is
    // dispatched from the top as the next iteration's command (playtest E25).
    // Swallowing that key also broke `North` and `Q`/`Y` after a Gate Travel
    // onto a live gate cell (playtest E42).
    if state.resolve_natural_moongate_entry(game_dir)?.is_some() {
        // The key that brought this loop iteration here is still the turn's
        // command, so it is dispatched from the post-transit state - into the
        // meditation prompt when the hook opened one. Returning here instead
        // dropped every other command while the party stood on a re-rising
        // gate (playtest E61). The transit rewrote the cell to terrain `5`,
        // so this re-entry cannot run the same transit again.
        return handle_play_key_input_inner(state, key, suffix, game_dir);
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
    // `input.md §2` step 3: an extended key that does not classify "is
    // returned as 'no key'", and the engine "treats them as 'no key' and
    // continues polling". Nothing is printed, no turn passes, and the
    // message window is left exactly as it was - `Input ignored.` and
    // `Function key ignored.` stood here, and no spec text carries either
    // (playtest M5). It is no command either, so it takes no drunken
    // even-odds gate below.
    if key == PLAY_IGNORED_INPUT_KEY {
        return Ok(PlayInputDisposition::Continue);
    }
    // `systems/shops.md` tavern drunkenness: every top-level town command
    // performs a fresh even-odds gate. Active prompts/sessions above consume
    // their own keys before this point and therefore are not commands.
    if let Area::Town { scene, floor } = state.area {
        if state.town_drunkenness_counter != 0 && state.random_range_u8(0, 1) == 1 {
            state.town_alarm_sweep(scene, floor, None);
            state.town_drunkenness_counter -= 1;
            // `town-mode.md §7`/§7.1: drunkenness is a pre-dispatch stage
            // after the command read, so `Hic!` lands on that command's
            // prompt row - after its feed and marker - and its line break
            // leaves the substituted step's echo on the next row with no
            // prompt pair of its own (playtest J18).
            state.flush_message_slot();
            state.emit_command_echo_line("Hic!");
            state.message_row_open_mid_line = false;
            state.town_hiccup_echo_continues = true;
            let replacement = state.random_range_u8(0, 3);
            key = char::from(INPUT_CODE_CARDINAL_FIRST + replacement);
            suffix = "";
        }
    }
    // `dungeon-mode.md §15`: the dungeon's minute advance "sits at the head
    // of each iteration ... so a command the dispatcher reports as \"no
    // action\" (a digit solo-select, a refused Push, the typeahead toggle)
    // still costs a minute underground", and `§4` step 3 dispatches the
    // four shared Control bindings from that same iteration. The bindings
    // still take no *turn* (`commands.md §9`); only the clock moves
    // (playtest E17).
    if matches!(
        key,
        PLAY_TYPEAHEAD_TOGGLE_KEY
            | PLAY_MUSIC_TOGGLE_KEY
            | PLAY_MORAL_STANDING_KEY
            | PLAY_VERSION_BANNER_KEY
            | PLAY_EXIT_TO_DOS_KEY
    ) && matches!(state.area, Area::Dungeon { .. })
        && !state.combat_active
    {
        state.advance_dungeon_loop_minute();
    }
    if key == PLAY_TYPEAHEAD_TOGGLE_KEY {
        state.toggle_typeahead_buffer();
        mark_combat_short_reprompt(state);
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_MUSIC_TOGGLE_KEY {
        state.toggle_music();
        mark_combat_short_reprompt(state);
        return Ok(PlayInputDisposition::Continue);
    }
    // `commands.md §9`: "Combat replaces the scheme entirely with its own
    // parser", and `combat.md §8` lists the arena's control codes as Escape,
    // Ctrl-S, Ctrl-B, Space, digits and directions - "Anything else - the
    // stock `What?` refusal and a free re-prompt". Ctrl-K, Ctrl-V and Ctrl-E
    // are therefore exploration-only; in the arena they fall through to the
    // combat parser (playtest K15).
    if matches!(
        key,
        PLAY_MORAL_STANDING_KEY | PLAY_VERSION_BANNER_KEY | PLAY_EXIT_TO_DOS_KEY
    ) && state.combat_active
    {
        return Ok(handle_combat_key_input(state, key, suffix));
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
        // An Uus Por / Des Por level change runs the climb's post-action
        // pass; every other outcome falls through to the generic hook.
        state.apply_post_turn_effects_after_dungeon_level_change(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.combat_active {
        return Ok(handle_combat_key_input(state, key, suffix));
    }
    if key == 'Z' {
        // `dungeon-mode.md §15`: the dungeon's minute "sits at the head of
        // each iteration ... [and] does **not** gate that call on whether the
        // command consumed a turn". `Z` is read by this shared intercept
        // rather than the dungeon handler, so it runs that loop head itself;
        // otherwise a `Z` after a no-action command found the previous
        // iteration's charge still standing and spent no minute (p10 R12).
        if matches!(state.area, Area::Dungeon { .. }) {
            state.advance_dungeon_loop_minute();
        }
        state.begin_command_echo_for(Command::ZStats);
        state.z_stats_command();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'M' && !suffix.is_empty() {
        state.begin_command_echo_for(Command::Mix);
        // `karma.md §7`: "`M` always selects Mix Reagents, including at a
        // shrine" (`RETRACTIONS.md` R411); meditation is reached only through
        // E-Enter. §8 (R451): the Codex reader is likewise E-Enter on `0x11`.
        let turn_before = state.turn;
        let outcome = state.mix_reagents_from_suffix(suffix);
        state.charge_mix_command_acted_exit(turn_before);
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
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
            // `dungeon-mode.md §4` step 3: a byte the dungeon handler does
            // not own goes to the resident dispatcher and "reports whatever
            // the dispatcher decides". For unknown input the dispatcher
            // prints the stock `What?\n` (`commands.md §5.2`, "any unmapped
            // key") and reports "no action" (`commands.md §3`), so this
            // falls through to the same refusal the top-down scenes print.
            // The sleep line belongs to the party-capability check, never
            // to a keystroke.
            state.advance_visual_tick();
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
        // `stats-panel.md §2.2.1`: "Stats browser exit" is an immediate
        // whole-panel site - it repaints from live state whether or not
        // anything changed, which also corrects a stale panel.
        state.repaint_stats_panel();
        finish_combat_modal_actor_action(state, combat_actor);
    }
    PlayInputDisposition::Continue
}

/// `inventory.md §5` / `commands.md §4`: once R-Ready's or Z-stats' last
/// prompt has closed, pay the owed exploration charge and run the
/// post-action pass that follows it (playtest F23).
pub fn settle_exploration_acted_charge(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if let Some(turn_before) = state.settle_exploration_acted_charge() {
        // The exchange's closing reply (`Item: Done`, `Status: Done`,
        // `Player: None!`) is already committed when the charge is paid.
        // The world pass joins whatever the slot holds onto an engagement's
        // own text (`pre_effect_message`), so a reply left there came back
        // after the banner as `Item: Done *** CONFLICT ***`. Take it out of
        // the slot for the pass, as the dungeon climb word is, and put it
        // back only when the pass printed nothing.
        state.flush_message_slot();
        let reply = (state.message == state.message_flushed).then(|| {
            (
                std::mem::take(&mut state.message),
                std::mem::take(&mut state.message_flushed),
            )
        });
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, MoveOutcome::Used)?;
        if let Some((message, flushed)) = reply
            && state.message.is_empty()
        {
            state.message = message;
            state.message_flushed = flushed;
        }
    }
    Ok(())
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
        // `stats-panel.md §2.2.1`: "R Ready at its end" repaints the whole
        // panel inline.
        state.repaint_stats_panel();
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
    if combat.is_none() {
        // `dungeon-mode.md §13.1`: an Uus Por / Des Por level change is a
        // climbing transition, so the post-action pass reads the landing
        // cell before the next key, as after K-Klimb (playtest E08).
        state.apply_post_turn_effects_after_dungeon_level_change(turn_before, game_dir, outcome)?;
    } else {
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
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
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let turn_before = state.turn;
    if let Some(outcome) = state.step_active_mix(key, suffix) {
        // `magic.md §6` (corrected 2026-10-25, u5-spec#302 Q1, R621): M-Mix
        // "reports \"acted\" on every path, and the mode loop runs its
        // ordinary per-turn epilogue exactly once after the command returns"
        // - success, a wrong mix, `None!`, Escape at the reagent list, a zero
        // quantity and `Nothing to mix!` alike (playtest S01).
        state.charge_mix_command_acted_exit(turn_before);
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
    }
    Ok(PlayInputDisposition::Continue)
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
        // `commands.md §7`: Jimmy reports "acted" after every exit.
        state.charge_tile_command_acted_exit(turn_before);
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
        // `commands.md §3`/§7: the G/S/O/J letter that opened this picker
        // reports "acted" on every exit.
        state.charge_tile_command_acted_exit(turn_before);
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
    // The E-Enter shrine arm's acted turn, once the session and its exit
    // pacing have finished (`commands.md §3`, playtest S14); while the exit
    // presentation is still staged, the exploration turn gate pays it.
    settle_exploration_acted_charge(state, game_dir)?;
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
        // `input.md §8`: Escape in the free-text reader clears the line but
        // neither submits nor ends the prompt - the same rule the
        // exploration Yell prompt follows (`step_active_yell`).
        if key == '\u{1b}' && suffix.is_empty() {
            return Ok(PlayInputDisposition::Continue);
        }
        let Some(mut session) = state.active_yell.take() else {
            return Ok(PlayInputDisposition::Continue);
        };
        let mut line = String::new();
        if !matches!(key, '\r' | '\n') {
            line.push(key);
        }
        line.push_str(suffix);
        session.buffer.push_str(&line);
        // `commands.md §5` transcript: the typed word is echoed on the
        // prompt's own colon row - `Yell what?` / `:WORD` - and the result
        // follows, the no-effect line under its own blank row
        // (`[blank]` / `No effect!`), the nothing-said one directly beneath.
        let word = PlayState::normalize_yell_word(&session.buffer);
        state.commit_typed_free_text_line(":", &word);
        state.message = if word.is_empty() {
            YELL_NOTHING_SAID_MESSAGE.to_string()
        } else {
            // `combat.md §8`, the `Y` row: nonempty combat Yell "reaches the
            // handler's no-effect path", whose literal is the shared one.
            format!("\n{YELL_NO_EFFECT_MESSAGE}")
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
    let dungeon_klimb = matches!(state.area, Area::Dungeon { .. })
        && state
            .active_direction_prompt
            .as_ref()
            .is_some_and(|session| matches!(session.kind, DirectionPromptKind::Klimb));
    let tile_command = state.active_direction_prompt.as_ref().is_some_and(|session| {
        matches!(
            session.kind,
            DirectionPromptKind::Get
                | DirectionPromptKind::Jimmy
                | DirectionPromptKind::Open
                | DirectionPromptKind::Search
        )
    });
    if let Some(outcome) = state.step_active_direction_prompt(key, suffix, game_dir)? {
        if tile_command {
            // `commands.md §3`/§7: G/S/O/J report "acted" on every exit.
            state.charge_tile_command_acted_exit(turn_before);
        }
        if let Some(actor_slot) = combat_push_actor {
            if combat_klimb && matches!(outcome, MoveOutcome::Blocked) {
                // `combat.md §8`: blocked Klimb is one of the parser's
                // exhaustive free re-prompt cases. `§8.4` (u5-spec#299 Q13):
                // it takes "the short re-prompt of Section 8.1: one newline,
                // so a blank row follows `What?`, then a prompt-window refresh
                // and another key read, with no banner" - the next echo keeps
                // `text-output.md §10.4`'s derived blank.
                state.pending_combat_actor_slot = Some(actor_slot);
                state.combat_prompt_row_opened_by_banner = false;
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
        } else if dungeon_klimb {
            // `dungeon-mode.md §13.1`: the up-or-down answer is a climbing
            // arm, so the post-action pass reads the landing cell.
            state.apply_post_turn_effects_after_dungeon_level_change(turn_before, game_dir, outcome)?;
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
    // `input.md §8`: the sage's `You respond:` topic line is the shared
    // free-text reader, where "Escape clears the whole current line but does
    // **not** submit or terminate it" (`text-output.md §10.6`). The shell
    // clears its own buffer; a bare Escape reaching here is never a typed
    // topic (playtest G15).
    if key == '\u{1b}'
        && suffix.is_empty()
        && matches!(session, ActiveShopSession::Sage(SageState::Prompt { .. }))
    {
        state.active_shop = Some(session);
        return PlayInputDisposition::Continue;
    }
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
    let yes = matches!(key_byte, b'Y' | b'y') || suffix.chars().any(|c| matches!(c, 'Y' | 'y'));
    let no = matches!(key_byte, b'N' | b'n') || suffix.chars().any(|c| matches!(c, 'N' | 'n'));
    let mut replacement_session: Option<ActiveShopSession> = None;
    // Row state a tavern or sage message leaves for the next key's echo,
    // stated once that message is on the transcript. Each arm states it from
    // the outcome it printed (see [`tavern_outcome_prompt_row`]); it is never
    // read back from the message text (u5-engine#46 residual).
    let mut prompt_row_open: Option<bool> = None;
    // Measured 2026-09-07/08 at all eight overlays: the answer to the entry
    // question echoes as a word - `Yes` or `No` - onto the row the greeting
    // left the cursor on. A Paws capture shows it appended to the greeting's
    // own last row (`thee?" Yes`); other captures show it a row lower, which
    // is the same rule when the greeting happens to fill its last row.
    // `shops.md §8.B`, the arms entry: its answer keys are not `Y`/`N`.
    // "Buy echoes `Buy\n\n"`, Sell echoes `Sell\n\n"`, and Space echoes `No`
    // without a line feed before the ordinary nothing-bought closing bark."
    // Measured 2026-09-09 (`qa/paired/shop-arms-sell-flow.tsv`, beat
    // `browser`): the original shows `or Sell?" Sell` before the sell prompt,
    // where this engine printed no echo at all and so left the two windows
    // structurally different for the rest of the visit.
    let arms_entry_echo = matches!(
        session,
        ActiveShopSession::Arms(ArmsShopState::Greeting)
            | ActiveShopSession::ArmsLocal(ArmsShopState::Greeting, _)
            | ActiveShopSession::ArmsStocked(ArmsShopState::Greeting, _)
    )
    .then(|| match key_byte {
        b'B' | b'b' => Some(("Buy", true)),
        b'S' | b's' => Some(("Sell", true)),
        b' ' => Some(("No", false)),
        _ => None,
    })
    .flatten();
    // The arms entry answers through `arms_entry_echo` above, and `shops.md`
    // §8.B has it "ignore other keys without reprinting": `Y`/`N` there echo
    // nothing. Before this they echoed `Yes`/`No` onto the greeting row.
    let arms_entry = matches!(
        session,
        ActiveShopSession::Arms(_)
            | ActiveShopSession::ArmsLocal(..)
            | ActiveShopSession::ArmsStocked(..)
    );
    let entry_answer_echo = (session.awaiting_entry_answer() && !arms_entry).then(|| {
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
                // `shops.md §8.B`: "The healer entry accepts Y/N only and
                // ignores Space"; every No "exits through the appropriate
                // closing-bark flow".
                (HealerShopState::Greeting, _, true, _) => {
                    *s = HealerShopState::Exited;
                    shop_closing_bark(state, SHOP_DIALOG_ID_HEALER, healer_name, game_dir)
                }
                (HealerShopState::Greeting, true, _, _) => {
                    *s = HealerShopState::PickService;
                    healer_service_menu(healer_name)
                }
                // Other entry keys re-poll without output.
                (HealerShopState::Greeting, _, _, _) => String::new(),
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
                        state.emit_message_line_continuing_row(healer_service_echo(treatment));
                        // No blank of its own: §8.C's `\n\n"Who needs my aid?" `
                        // and every result row bring their own `\n\n`, whose
                        // first feed closes this echo's row. Pushing a blank
                        // here as well gave three blank rows above the
                        // question (playtest F47).
                        if state.party.len() == 1 {
                            *s = HealerShopState::PickPartyMember {
                                service,
                                cost,
                                highlight: 0,
                            };
                            // The automatic selection commits the one row the
                            // selector can indicate.
                            return {
                                state.active_shop = Some(session);
                                handle_active_shop_key_input(state, '\r', "", game_dir)
                            };
                        }
                        *s = HealerShopState::PickPartyMember {
                            service,
                            cost,
                            highlight: 0,
                        };
                        // §8.C: "A sole party member is selected automatically;
                        // otherwise print `\n\n"Who needs my aid?" ` and use
                        // the party selector." The question's closing quote
                        // and space leave its row open for the cancel's
                        // `No one`; the selector state is not one of the
                        // inline prompts the generic rule below states, so
                        // this caller states it.
                        prompt_row_open = Some(true);
                        HEALER_WHO_NEEDS_AID.to_string()
                    }
                    // §8.C: "Space/Return echo `Nothing` and leave through the
                    // ordinary farewell."
                    HealerServiceAction::Exit => {
                        state.emit_message_line_continuing_row("Nothing");
                        *s = HealerShopState::Exited;
                        shop_closing_bark(state, SHOP_DIALOG_ID_HEALER, healer_name, game_dir)
                    }
                    // §8.C: "Other keys silently wait without reprinting.
                    // Earlier invalid-key menu-redraw claims are retracted
                    // (R419)."
                    HealerServiceAction::Discard => String::new(),
                },
                // `inventory.md §4.3`, the shared selector §8.C's healer uses:
                // "Only Return or Space commits the indicated row, Escape
                // cancels". `N` is not a cancel key there (playtest E48).
                (HealerShopState::PickPartyMember { .. }, _, _, _) if key_byte == 0x1b => {
                    *s = HealerShopState::Continuation;
                    // §8.C: "Cancellation adds `No one`, with no trailing line
                    // feed", then the continuation question. "Adds" is onto
                    // the row `Who needs my aid?" ` left open, as every
                    // other shop echo continues its question's row; the
                    // continuation's own first feed then closes that row.
                    state.emit_message_line_continuing_row("No one");
                    HEALER_CONTINUATION.to_string()
                }
                // "A digit moves the indicator; it does not commit ... the
                // four direction keys move the indicator"; other keys leave
                // the message window untouched.
                (
                    HealerShopState::PickPartyMember {
                        service,
                        cost,
                        highlight,
                    },
                    _,
                    _,
                    digit,
                ) if !matches!(key_byte, b'\r' | b'\n' | b' ') => {
                    let highlight = usize::from(highlight);
                    let moved = shop_party_selector_move(state, highlight, key, digit);
                    *s = HealerShopState::PickPartyMember {
                        service,
                        cost,
                        highlight: moved as u8,
                    };
                    state.message.clone()
                }
                (
                    HealerShopState::PickPartyMember {
                        service, highlight, ..
                    },
                    _,
                    _,
                    _,
                ) => {
                    let d = highlight + 1;
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
                        // §8.3: "An untreatable selection prints the shared
                        // healer refusal and asks the continuation Y/N
                        // question".
                        *s = HealerShopState::Continuation;
                        healer_no_need_refusal(healer_name)
                    } else {
                        match healer_treatment_fee(*healer, treatment) {
                            HealerTreatmentFee::Bypass => {
                                let message = match state.buy_healer_treatment(
                                    *healer,
                                    treatment,
                                    target_index,
                                ) {
                                    // §8.C "Minoc free Cure/Heal |
                                    // `\n\n"Receive now the Light!"`,
                                    // followed by treatment effects".
                                    Ok(_) => format!(
                                        "{HEALER_RECEIVE_THE_LIGHT}{HEALER_CONTINUATION}"
                                    ),
                                    Err(err) => format_healer_treatment_error(err, healer_name),
                                };
                                *s = HealerShopState::Continuation;
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
                    // The fee prompt `pay?" ` leaves its row open mid-line.
                    state.message_row_open_mid_line = true;
                    state.emit_message_line_continuing_row("Yes");
                    let treatment = healer_treatment_for_service(service);
                    match state.buy_healer_treatment(*healer, treatment, usize::from(slot)) {
                        // §8.3: the paid branch "deducts gold, applies the
                        // treatment, refreshes the visible character status,
                        // and asks the continuation Y/N question".
                        Ok(outcome) => {
                            state.arms_transaction_completed = true;
                            // §8.C (u5-spec#298 Q19): on the low-fee charity
                            // "the post-payment surcharge step does not run".
                            let surcharge = if matches!(
                                outcome.quote.fee,
                                HealerTreatmentFee::Price(_)
                            ) && !outcome.charity
                            {
                                    apply_active_shop_surcharge(state)
                                } else {
                                    None
                                };
                            *s = HealerShopState::Continuation;
                            // §8.C's continuation keeps both of its feeds
                            // here: the first closes `pay?" Yes`, the second
                            // is the blank row. Playtest F47: dropping one ran
                            // `"Is there any other way` straight under it.
                            shop_message_ignoring_surcharge(HEALER_CONTINUATION.to_string(), surcharge)
                        }
                        // §8.C "Ordinary short funds | Record `173`, then the
                        // continuation question below, exactly as after a
                        // declined fee; no gold, status or HP changes"
                        // (u5-spec#298 Q19: the visit continues).
                        Err(crate::shops::HealerTreatmentError::InsufficientGold {
                            ..
                        }) => {
                            *s = HealerShopState::Continuation;
                            let record = render_shared_shoppe_record(
                                game_dir,
                                HEALER_SHORT_FUNDS_RECORD,
                                cost,
                                healer_name,
                                healer.display_name(),
                            );
                            format!("{record}{HEALER_CONTINUATION}")
                        }
                        Err(err) => {
                            *s = HealerShopState::Continuation;
                            format_healer_treatment_error(err, healer_name)
                        }
                    }
                }
                (HealerShopState::Confirm { service, .. }, _, true, _) => {
                    *s = HealerShopState::Continuation;
                    let _ = service;
                    // §8.C: the fee prompt `Wilt thou\npay?" ` echoes a bare
                    // `No` onto its own row, exactly as `Yes` does, and the
                    // visit continues on its own question. Returning the echo
                    // inside the message put it on a row of its own
                    // (play-test P22). The prompt row is open mid-line.
                    state.message_row_open_mid_line = true;
                    state.emit_message_line_continuing_row("No");
                    HEALER_CONTINUATION.to_string()
                }
                // §8.C: "The continuation accepts Y/N, with the same entry
                // echoes as Section 8.B. Yes repeats the service question; No
                // ends the visit."
                (HealerShopState::Continuation, true, _, _) => {
                    state.emit_message_line_continuing_row("Yes");
                    state.push_explicit_blank_message_entry();
                    *s = HealerShopState::PickService;
                    healer_service_menu(healer_name)
                }
                (HealerShopState::Continuation, _, true, _) => {
                    state.emit_message_line_continuing_row("No");
                    *s = HealerShopState::Exited;
                    shop_closing_bark(state, SHOP_DIALOG_ID_HEALER, healer_name, game_dir)
                }
                (HealerShopState::Exited, _, _, _) => String::new(),
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
            let honorific = tavern_honorific(speaker_is_female);
            let attribution = |verb: &str| {
                innkeeper_name
                    .map(|name| format!("\n{verb} {name}."))
                    .unwrap_or_default()
            };
            let service_question = || match innkeeper_name {
                Some(name) => format!("{name} asks,\n{INN_SERVICE_QUESTION}"),
                None => INN_SERVICE_QUESTION.to_string(),
            };
            let who_will_stay = || match innkeeper_name {
                Some(name) => format!("{name} asks,\n{INN_WHO_WILL_STAY_PROMPT}"),
                None => INN_WHO_WILL_STAY_PROMPT.to_string(),
            };
            match (*s, yes, no, inline_digit) {
                // `shops.md §8`'s entry table, row `0x88`: the greeting takes
                // `Y`, `N` or Space; Yes opens the service question and every
                // other key leaves the greeting visible.
                (InnkeeperState::Greeting { inn }, true, _, _) => {
                    *s = InnkeeperState::ServiceMenu { inn };
                    service_question()
                }
                // `shops.md §8.B`: "Space means No where accepted. Every No
                // response echoes `No` without a line feed, then exits through
                // the appropriate closing-bark flow." Measured 2026-09-11
                // (`shop-inn-after-entry/after-space`), where the original
                // answers `No` and then `"Well, we can't help thee, then."`
                // over `says Donya.`
                //
                // The bark rides §8's shared closing envelope, which opens
                // `\n\n"`: the first feed closes the `:No` row, the second is
                // the blank under it. Measured 2026-10-16 (same beat): the
                // original reads `:No`, a blank row, then the quoted bark;
                // this engine put `No` on a row of its own under the `:` and
                // the bark directly beneath it.
                (InnkeeperState::Greeting { .. }, _, true, _) => {
                    *s = InnkeeperState::Exited;
                    inn_entry_closing_bark(state, innkeeper_name, game_dir)
                }
                (InnkeeperState::Greeting { .. }, _, _, _) if key_byte == b' ' => {
                    // §8.B: the answer appends to the greeting's continuation
                    // row, which is open whatever the slot's text ended on.
                    state.flush_message_slot();
                    state.open_message_row();
                    state.emit_message_line_continuing_row("No");
                    *s = InnkeeperState::Exited;
                    inn_entry_closing_bark(state, innkeeper_name, game_dir)
                }
                // "Return, Escape and service letters such as R at the inn do
                // not select a service at this initial prompt: unaccepted
                // normalized keys wait silently".
                (InnkeeperState::Greeting { .. }, _, _, _) => String::new(),
                (InnkeeperState::ServiceMenu { inn }, _, _, _) => match inn_main_action(key_byte) {
                    InnMainAction::Rest => {
                        // §8.C: "L and R echo those uppercase letters".
                        state.emit_message_line_continuing_row("R");
                        if crate::shops::inn_lodging_full(inn, &state.inn_registry, scene_marker)
                        {
                            *s = InnkeeperState::Continuation { inn };
                            return_inn_capacity_refusal(honorific)
                        } else {
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
                            // §8.C "Rest offer | Opening quote, room record,
                            // then `\nWilt thou take\nit?" `. Records by inn
                            // row are `186, 187, 188, 188, 189, 190`."
                            //
                            // The capacity check's `\n\n` comes first: one
                            // feed closes the `night?" R` row, the other is
                            // the blank (playtest F48).
                            format!(
                                "\n\n\"{}{INN_TAKE_IT_PROMPT}",
                                render_shared_shoppe_record(
                                    game_dir,
                                    inn_room_record(inn),
                                    total_price,
                                    innkeeper_name,
                                    inn.display_name(),
                                )
                            )
                        }
                    }
                    InnMainAction::LeaveCompanion => {
                        state.emit_message_line_continuing_row("L");
                        if crate::shops::inn_lodging_full(inn, &state.inn_registry, scene_marker)
                        {
                            *s = InnkeeperState::Continuation { inn };
                            return_inn_capacity_refusal(honorific)
                        } else if ctx.party_size <= 1 {
                            // §8.C "Leave with only Avatar travelling | After
                            // the lodging-capacity check, test party size
                            // ... Record `191`; visit ends without an added
                            // attribution or ordinary farewell". Measured
                            // 2026-09-08 at Hotel Brittany
                            // (`qa/paired/nb-inn-refusals.tsv`).
                            *s = InnkeeperState::Exited;
                            let record =
                                render_shared_shoppe_record(
                                game_dir,
                                INN_SOLO_LEAVE_RECORD,
                                0,
                                innkeeper_name,
                                inn.display_name(),
                            );
                            // Without `SHOPPE.DAT` the record renders
                            // empty and nothing is said: §8.C names only the
                            // record, and the engine must not carry a copy
                            // of its text (playtest Q16).
                            format!("\n\n{record}")
                        } else {
                            let deposit = inn_leave_companion_deposit_for_speaker(
                                inn,
                                ctx.speaker_intelligence,
                            );
                            *s = InnkeeperState::PickLeaveCompanion {
                                inn,
                                deposit,
                                highlight: 0,
                            };
                            // **Measured** 2026-09-08 at Hotel Brittany with a
                            // party of three (`qa/paired/nb-inn-companion.tsv`):
                            // the innkeeper asks in the message window while
                            // the *panel* becomes a framed `Select:` picker
                            // over the party rows. No deposit is quoted here.
                            // The capacity check's `\n\n`, then the question.
                            format!("\n\n{}", who_will_stay())
                        }
                    }
                    // §8.C: "P has no separate echo."
                    InnMainAction::PickUpCompanion => {
                        let guests = inn_guest_indices_for_scene(&state.inn_registry, scene_marker);
                        if state.party.len() >= crate::INN_PARTY_CAP {
                            // "Pickup with six travelling members |
                            // `\n\nOne must first be left behind!\n\n`; this
                            // check precedes the no-guests check".
                            *s = InnkeeperState::Continuation { inn };
                            format!("\n\nOne must first be left behind!\n\n{INN_ANYTHING_MORE_PROMPT}")
                        } else if guests.is_empty() {
                            // "Pickup with no guest at this inn |
                            // Token-expanded `\n\n"No one here is from thy
                            // party!"\nsays $.\n\n`".
                            *s = InnkeeperState::Continuation { inn };
                            format!(
                                "\n\n\"No one here is from thy party!\"{}\n\n{INN_ANYTHING_MORE_PROMPT}",
                                attribution("says")
                            )
                        } else if guests.len() == 1 {
                            // "Single-guest Pickup selects automatically.
                            // After guest selection, the bill and
                            // affordability check proceed without a Y/N
                            // confirmation." Measured 2026-09-08 at Hotel
                            // Brittany (`qa/paired/nb-inn-pickup.tsv`).
                            inn_pickup_selected_guest(
                                state,
                                s,
                                inn,
                                scene_marker,
                                guests[0],
                                ctx.speaker_intelligence,
                                innkeeper_name,
                                game_dir,
                            )
                        } else {
                            // `shops.md §8.4` (u5-spec#299 Q14): "The Guest
                            // Register and its selection steps scan records 1
                            // through 15", so the register lists the guests
                            // in roster-record order.
                            let mut guests = guests;
                            guests.sort_by_key(|index| {
                                state.inn_registry.get(*index).map(|guest| guest.registry_slot)
                            });
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
                                base_lodging_charge: inn_base_room_rate(inn),
                                highlight: 0,
                            };
                            // "Multiple-guest selection | `\n\n"Who will\ncheck
                            // out?" `"; the framed register is the side panel.
                            format!("\n\n{INN_WHO_WILL_CHECK_OUT_PROMPT}")
                        }
                    }
                    // `shops.md §8.A` "Shared closing envelope": the ordinary
                    // farewell, drawn from the no-sale or purchase-completed
                    // row for this visit.
                    InnMainAction::Exit => {
                        *s = InnkeeperState::Exited;
                        shop_closing_bark_after_open_prompt(
                            state,
                            SHOP_DIALOG_ID_INN,
                            innkeeper_name,
                            game_dir,
                        )
                    }
                    // `shops.md §8.B` "Inn actions": keys outside `L`/`P`/`R`
                    // and Space "silently wait". Measured 2026-09-11
                    // (`nb-inn/no` and `/esc`).
                    InnMainAction::Discard => String::new(),
                },
                (
                    InnkeeperState::ConfirmRest {
                        inn, total_price, ..
                    },
                    true,
                    _,
                    _,
                ) => {
                    state.emit_message_line_continuing_row("Yes");
                    let poisoned: Vec<usize> = state
                        .party
                        .iter()
                        .enumerate()
                        .filter(|(_, member)| member.status == b'P')
                        .map(|(index, _)| index)
                        .collect();
                    let result = state.pay_inn_rest_total(inn, total_price);
                    // §8.C: a paid Rest ends the visit "without the inn
                    // continuation question or shared farewell"; the
                    // Highwaymen refusal ends it too.
                    *s = InnkeeperState::Exited;
                    match result {
                        Ok(outcome) => {
                            state.arms_transaction_completed = true;
                            // "Paid Rest | `"Have a pleasant\nnight, `,
                            // honorific, token-expanded `!"\nsays $.\n\n`, then
                            // the rest sequence."
                            // The offer's `\n\n` after the bare echo: one
                            // feed closes `it?" Yes`, one is the blank.
                            let mut message = format!(
                                "\n\n\"Have a pleasant\nnight, {honorific}!\"{}\n\n",
                                attribution("says")
                            );
                            message.push_str(&apply_paid_inn_rest(
                                state,
                                inn,
                                outcome.quote.total_price,
                            ));
                            // "A poisoned member's recovery death adds `\n`,
                            // the member name, and ` has\npassed away.\n`."
                            // That `\n` follows `Morning!\n`, whose own feed
                            // the slot leaves to the next row, so the name
                            // takes two here: one closes the `Morning!` row and
                            // one is the blank above the name. The closing
                            // `\n` is left to the next row as `Morning!`'s is
                            // (playtest H47).
                            for index in poisoned {
                                if state.party.get(index).is_some_and(|m| m.status == b'D') {
                                    let name = state
                                        .party_names
                                        .get(index)
                                        .and_then(|name| crate::party::party_name_to_string(name))
                                        .unwrap_or_default();
                                    message.push_str(&format!("\n\n{name} has\npassed away."));
                                }
                            }
                            let surcharge = apply_active_shop_surcharge(state);
                            shop_message_ignoring_surcharge(message, surcharge)
                        }
                        // "Accepted Rest, short funds | `"Highwaymen!\nCheap,
                        // at that!\nOUT!" `, then token-expanded `screams\n$.\n`".
                        Err(err) => format!(
                            "\n\n{}{}",
                            format_inn_error(err, speaker_is_female),
                            innkeeper_name
                                .map(|name| format!("screams\n{name}."))
                                .unwrap_or_default()
                        ),
                    }
                }
                // "Declined Rest offer | After the bare `No` and two line
                // feeds, end the visit through the ordinary farewell for the
                // visit's existing outcome ... it does not return to the
                // service question."
                //
                // The two feeds are read as the envelope's own `\n\n"`, as
                // the arms exit measures them (`shop-arms-menus/exit`): `No`,
                // one blank row, the quoted bark. The engine used to print one
                // feed of its own before an envelope-less bark.
                (InnkeeperState::ConfirmRest { .. }, _, true, _) => {
                    state.emit_message_line_continuing_row("No");
                    *s = InnkeeperState::Exited;
                    shop_closing_bark(state, SHOP_DIALOG_ID_INN, innkeeper_name, game_dir)
                }
                // `shops.md §8.C` "Leave-companion selection": Escape cancels
                // and prints `Nobody\n\n`, then reaches the continuation.
                (InnkeeperState::PickLeaveCompanion { inn, .. }, _, _, _) if key_byte == 0x1b => {
                    state.emit_message_line_continuing_row("Nobody");
                    *s = InnkeeperState::Continuation { inn };
                    format!("\n\n{INN_ANYTHING_MORE_PROMPT}")
                }
                // "A valid party digit moves the highlight; Return or Space
                // commits the indicated member", which starts on Avatar. "The
                // direction keys move within the travelling roster's bounds.
                // `Y`, `P`, `0` and out-of-party digits leave the picker
                // waiting", and `inventory.md §4.3` leaves the message window
                // untouched meanwhile. A digit used to commit at once and
                // Down blanked the window (playtest E49).
                (
                    InnkeeperState::PickLeaveCompanion {
                        inn,
                        deposit,
                        highlight,
                    },
                    _,
                    _,
                    digit,
                ) if !matches!(key_byte, b' ' | b'\r' | b'\n') => {
                    *s = InnkeeperState::PickLeaveCompanion {
                        inn,
                        deposit,
                        highlight: shop_party_selector_move(state, highlight, key, digit),
                    };
                    state.message.clone()
                }
                (
                    InnkeeperState::PickLeaveCompanion {
                        inn,
                        deposit,
                        highlight,
                    },
                    _,
                    _,
                    _,
                ) => {
                    let party_index = highlight;
                    if party_index == 0 {
                        // "Leave selects Avatar | `\n\nThy friend`, plural `s`
                        // only when more than two members travel, then ` will
                        // not leave thee!\n\n`; repeat the target question".
                        let plural = if state.party.len() > 2 { "s" } else { "" };
                        format!(
                            "\n\nThy friend{plural} will not leave thee!\n\n{}",
                            who_will_stay()
                        )
                    } else if state.party[party_index].status == b'D' {
                        // "Leave selects a dead companion | After the
                        // selection's `\n\n`, record `192`; visit ends without
                        // ordinary farewell".
                        *s = InnkeeperState::Exited;
                        format!(
                            "\n\n{}",
                            render_shared_shoppe_record(
                                game_dir,
                                INN_DEAD_COMPANION_RECORD,
                                0,
                                innkeeper_name,
                                inn.display_name(),
                            )
                        )
                    } else {
                        *s = InnkeeperState::ConfirmLeaveCompanion {
                            inn,
                            party_index,
                            deposit,
                        };
                        // "Leave rate | After `\n\n`, `"The rate for\nour most
                        // comfortable room will be `, token-expanded `% gold
                        // per month, due at check-out.`, then `\nWilt thou
                        // take\nit?" `."
                        // Playtest F48: with one feed the rate ran straight
                        // under `stay?" `.
                        format!(
                            "\n\n\"The rate for\nour most comfortable room will be {deposit} gold per month, due at check-out.{INN_TAKE_IT_PROMPT}"
                        )
                    }
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
                    state.emit_message_line_continuing_row("Yes");
                    let result = state.leave_inn_companion(scene_marker, party_index, deposit);
                    *s = InnkeeperState::Continuation { inn };
                    match result {
                        // "Leave accepted | Token-expanded `"I thank
                        // thee."\nsays $.\n\n`", then the inn's continuation.
                        Ok(_) => {
                            state.arms_transaction_completed = true;
                            // "Leave rate | ... bare echo, then `\n\n`".
                            format!(
                                "\n\n{INN_THANKS_LINE}{}\n\n{INN_ANYTHING_MORE_PROMPT}",
                                attribution("says")
                            )
                        }
                        Err(err) => format!(
                            "\n\n{}{INN_ANYTHING_MORE_PROMPT}",
                            format_inn_error(err, speaker_is_female)
                        ),
                    }
                }
                (InnkeeperState::ConfirmLeaveCompanion { inn, .. }, _, true, _) => {
                    state.emit_message_line_continuing_row("No");
                    *s = InnkeeperState::Continuation { inn };
                    format!("\n\n{INN_ANYTHING_MORE_PROMPT}")
                }
                // "Cancel guest selection | Escape prints `No one\n\n`;
                // Space/Return instead select the highlighted guest".
                (InnkeeperState::PickUpCompanion { inn, .. }, _, _, _) if key_byte == 0x1b => {
                    state.emit_message_line_continuing_row("No one");
                    *s = InnkeeperState::Continuation { inn };
                    format!("\n\n{INN_ANYTHING_MORE_PROMPT}")
                }
                // `shops.md §8.C` "Guest Register keys" (u5-spec#299 Q15):
                // "Return or Space commits the highlighted guest", which
                // starts on the topmost listed guest. "Digits never move the
                // highlight and never commit"; a digit used to commit that
                // guest at once.
                (
                    InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        highlight,
                        ..
                    },
                    _,
                    _,
                    _,
                ) if matches!(key_byte, b' ' | b'\r') => inn_pickup_selected_guest(
                    state,
                    s,
                    inn,
                    scene_marker,
                    guest_indices[usize::from(highlight)],
                    ctx.speaker_intelligence,
                    innkeeper_name,
                    game_dir,
                ),
                // "West or North moves it up one listed guest and East or
                // South moves it down one. There is no wrap: at either end
                // the key does nothing." "Every other key is ignored without
                // a redraw: the digits `0`-`9`, letters, the four diagonal
                // codes and line feed."
                (
                    InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        guest_count,
                        base_lodging_charge,
                        highlight,
                    },
                    _,
                    _,
                    _,
                ) => {
                    let last = guest_count.saturating_sub(1);
                    let moved = match crate::z_stats::party_selector_direction_step(key) {
                        Some(true) => highlight.saturating_add(1).min(last),
                        Some(false) => highlight.saturating_sub(1),
                        None => highlight,
                    };
                    *s = InnkeeperState::PickUpCompanion {
                        inn,
                        guest_indices,
                        guest_count,
                        base_lodging_charge,
                        highlight: moved,
                    };
                    state.message.clone()
                }
                // §8.C "Inn actions": "After a continuing action, print
                // `"Is there\nanything more\nI can do for\nthee?" `; Y echoes
                // `Yes` and repeats the service question, N or Space echoes
                // `No` and exits. Other keys wait."
                (InnkeeperState::Continuation { inn }, true, _, _) => {
                    state.emit_message_line_continuing_row("Yes");
                    *s = InnkeeperState::ServiceMenu { inn };
                    // §8.B's `Yes` then `\n\n$ asks,`: the echo row closes
                    // and one blank follows, as at entry.
                    format!("\n\n{}", service_question())
                }
                (InnkeeperState::Continuation { .. }, _, _, _) if no || key_byte == b' ' => {
                    state.emit_message_line_continuing_row("No");
                    *s = InnkeeperState::Exited;
                    shop_closing_bark(state, SHOP_DIALOG_ID_INN, innkeeper_name, game_dir)
                }
                (InnkeeperState::Exited, _, _, _) => String::new(),
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
            // `shops.md §8.B`: "Space means No where accepted. Every No
            // response echoes `No` without a line feed"; Y/N take the shared
            // entry echo, so Space's `No` continues the greeting's open row
            // here (playtest H28).
            if matches!(*s, TavernState::Greeting { .. }) && key_byte == b' ' {
                state.flush_message_slot();
                state.open_message_row();
                state.emit_message_line_continuing_row("No");
            }
            // `shops.md` §8.C's result table opens the bill "After the
            // action-letter echo", and the tavern's list and follow-up records
            // both end in a closing quote and a space - that space is the cell
            // the accepted letter echoes into. Measured 2026-09-10
            // (`paws-sage/lore`): the original's row reads `Rations?" C`,
            // where this engine printed the row with nothing after the quote.
            let tavern_letter_echo = matches!(
                *s,
                TavernState::Menu { .. } | TavernState::PostListWait { .. }
            )
            .then(|| tavern_accepted_menu_letter(*s, key_byte))
            .flatten();
            // `shops.md §8.C`, "Blue Boar wine selection accepted":
            // "Uppercase selected letter, then `\n\n"Ah, a fine\nchoice, `".
            // The list's `Thy choice?" ` leaves its row open for it, the same
            // cell the menu letter echoes into. The engine echoed nothing.
            let blue_boar_choice_list = matches!(*s, TavernState::BlueBoarDrinkList { .. });
            // `shops.md` §8.C, "Provision offer": the quantity prompt ends
            // `thou like?" `, a closing quote and a space, and
            // `text-output.md` §10.6 keeps the cursor on that space while the
            // digits are typed - the same open row the letter echo above
            // continues. Measured 2026-09-19 (`paws-tavern/zero`): the
            // original reads `thou like?" 0`, this engine `thou like?" `
            // with no digit at all. `cleak/u5-engine#41`.
            //
            // `shops.md §8.5`, "Which reader takes the number": the shared
            // two-digit reader, with "Return the only exit with an empty
            // entry submitting as zero" and "every other key discarded in
            // silence". The engine committed the first digit it saw, so a
            // two-digit quantity was impossible and a bare Return did nothing
            // (playtest E47). Keys edit the echo on the prompt's own row; a
            // line delivered whole (a script token such as `12`, or the
            // visual shell's line editor) submits once its keys are read, as
            // the mixer's quantity does.
            let mut quantity_entry_pending = false;
            let mut submitted_quantity = None;
            if let TavernState::PickProvisionQuantity {
                tavern,
                unit_price,
                continuation_ready,
                mut entry,
            } = *s
            {
                let mut submit = false;
                for ch in std::iter::once(key).chain(suffix.chars()) {
                    if entry.apply(ch as u8) {
                        submit = true;
                        break;
                    }
                }
                submit |= !suffix.is_empty() && !entry.as_str().is_empty();
                let row = format!("{TAVERN_PROVISION_QUANTITY_PROMPT_ROW}{}", entry.as_str());
                state.flush_message_slot();
                state.rewrite_open_prompt_row(TAVERN_PROVISION_QUANTITY_PROMPT_ROW, &row);
                *s = TavernState::PickProvisionQuantity {
                    tavern,
                    unit_price,
                    continuation_ready,
                    entry,
                };
                if submit {
                    submitted_quantity = Some(entry.quantity());
                } else {
                    quantity_entry_pending = true;
                }
            }
            // `shops.md §8.C`, "Enough-drink answer": "N prints `No!` and
            // proceeds with the purchase". Set by that arm so the echo can
            // land on the question's open row before the purchase text.
            let mut enough_drink_refused = false;
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
                    enough_drink_refused = true;
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
                (TavernState::PickProvisionQuantity { .. }, _, _, _) => {
                    if let Some(quantity) = submitted_quantity {
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
            // `shops.md §8.A`: a visit that completed a purchase closes on the
            // purchase-completed row rather than the nothing-bought one.
            if matches!(
                outcome,
                TavernOutcome::RoundDrinkServed { .. }
                    | TavernOutcome::SecondaryTavernSelected { .. }
                    | TavernOutcome::BlueBoarDrinkServed { .. }
                    | TavernOutcome::ProvisionsPurchased { paid: 1.., .. }
            ) {
                state.arms_transaction_completed = true;
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
                // `shops.md §8.A`: nothing bought draws `61..64`, a completed
                // purchase in this visit `65..68`.
                let (first, last) = tavern_closing_bark_records(state.arms_transaction_completed);
                usize::from(state.random_range_u8(first as u8, last as u8))
            });
            if let Some(letter) = tavern_letter_echo {
                if !matches!(outcome, TavernOutcome::InvalidInput) {
                    state.emit_message_line_continuing_row(letter.to_string());
                }
            }
            if blue_boar_choice_list
                && matches!(
                    outcome,
                    TavernOutcome::BlueBoarDrinkServed { .. }
                        | TavernOutcome::BlueBoarRefusedShortFunds { .. }
                )
            {
                state.emit_message_line_continuing_row(
                    (key_byte.to_ascii_uppercase() as char).to_string(),
                );
            }
            // `shops.md §8.C`: "N prints `No` and runs the ordinary quoted
            // farewell with its `says $.\n` attribution" - no feeds of its
            // own, so `No` continues the continuation question's open row
            // the same way the sibling `Y` does below.
            if matches!(outcome, TavernOutcome::DeclinedContinuation) {
                state.emit_message_line_continuing_row("No");
            }
            // `shops.md §8.C`, "Tavern and sage continuation": "Y prints
            // `Yes\n\n"`, the applicable follow-up record, a closing quote
            // and a space." That `Yes` continues the row the previous
            // record's closing quote and space left open - the same
            // placement §8.B gives the entry answer - and the two feeds are
            // the blank row beneath it.
            if matches!(outcome, TavernOutcome::Continued { .. }) {
                state.emit_message_line_continuing_row("Yes");
                state.push_explicit_blank_message_entry();
            }
            // `shops.md §8.C`, "Enough-drink answer": the question ends
            // `to drink?" ` with no line feed, so both answers continue that
            // row - "Y prints `Yes\n\n` and returns to continuation. N prints
            // `No!` and proceeds with the purchase". The engine put `Yes` on
            // a row of its own and printed no `No!` at all (playtest K24,
            // K25).
            if matches!(outcome, TavernOutcome::DeclinedEnoughDrink) {
                state.emit_message_line_continuing_row("Yes");
                state.push_explicit_blank_message_entry();
            }
            if enough_drink_refused {
                state.emit_message_line_continuing_row("No!");
            }
            let tavern_vendor_name = match state.area {
                Area::Town { scene, .. } => crate::play_state_impl::shop_vendor_name_for_scene(
                    SHOP_DIALOG_ID_TAVERN,
                    scene.byte,
                ),
                _ => None,
            };
            let tavern_vendor_named = tavern_vendor_name.is_some();
            let tavern_message = shop_message_ignoring_surcharge(
                format_tavern_outcome_with_shoppe(
                    outcome,
                    provision_quote_record_id,
                    no_sale_record_id,
                    tavern_vendor_name,
                    active_speaker_is_female(state),
                    // `shops.md` §4: the bill's count "is the number of
                    // nondead party members".
                    state
                        .party
                        .iter()
                        .filter(|member| member.status != crate::PARTY_STATUS_DEAD)
                        .count(),
                    game_dir,
                ),
                surcharge,
            );
            // The provision quantity's echo row is one transition a later key
            // continues (the next digit while the reader is still taking
            // keys; `quantity_entry_pending` below states it open). The
            // row-open flag otherwise
            // goes unmanaged here, same as every other direct `message =
            // ...` assignment in the engine - but this one specific
            // transition needs it accurate, so state it explicitly rather
            // than inferring it for every tavern outcome's text, which
            // regressed the mixer's own unrelated row accounting when tried
            // generically (`cleak/u5-engine#41`).
            //
            // The continuation question is the other transition a later key
            // echoes onto: `shops.md §8.C` "Y prints `Yes`" on the row
            // `Anything else for thee?" ` left open. Since the
            // continuing-row emitter started honouring this flag
            // (`6493e0d9`), a continuation reached by any outcome other than
            // a quantity left it closed, and `Yes` dropped to a row of its
            // own - measured 2026-10-16 (`paws-sage`, beats `again` and
            // `rumour`): the original reads `for thee?" Yes`.
            prompt_row_open = tavern_outcome_prompt_row(&outcome, tavern_vendor_named);
            if quantity_entry_pending {
                // Still reading: the slot holds the rewritten echo row the
                // transcript already has, and the cursor stays on it.
                prompt_row_open = Some(true);
                state.message.clone()
            } else {
                tavern_message
            }
        }
        ActiveShopSession::Sage(s) => {
            let mut sage_confirm_accepted = false;
            let outcome = match *s {
                SageState::Prompt { .. } => {
                    // `shops.md §8.8`: "Input accepts up to fifteen
                    // characters." A byte past the cap is never accepted, so
                    // it is neither echoed nor matched (playtest G16).
                    let line = active_shop_text_line(key, suffix)
                        .chars()
                        .take(crate::shops::SAGE_TOPIC_INPUT_LIMIT)
                        .collect::<String>();
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
                    sage_confirm_accepted = true;
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
                SageState::Confirm { .. } if yes => {
                    sage_confirm_accepted = true;
                    step_sage(
                        s,
                        SageInput::Confirm {
                            accepted: true,
                            record_id: SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
                        },
                        &mut state.gold,
                    )
                }
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
            let asks_fee = matches!(outcome, SageOutcome::QuotedFee { .. });
            if paid {
                // `shops.md §8.A`: paid advice is a completed purchase.
                state.arms_transaction_completed = true;
            }
            // `shops.md §8.C` sage result table: an empty topic and a
            // declined fee both return to the tavern continuation, with the
            // continuation retained (R436); only short funds ends the visit.
            let declined = matches!(outcome, SageOutcome::Declined);
            let returns_to_continuation =
                paid || declined || matches!(outcome, SageOutcome::EmptyTopic);
            if declined {
                state.emit_message_line_continuing_row("No");
                state.push_explicit_blank_message_entry();
            }
            // `shops.md §8.A`'s shared Y/N prompt primitive: the accepted
            // answer echoes onto the row the question's own trailing space
            // left open, and the two feeds that follow it are the blank row
            // beneath. The sage's fee confirmation ends `Fair 'nuff?" `, so
            // the `Yes` belongs on that row - the same placement the tavern's
            // continuation and menu letter already use.
            //
            // Measured 2026-09-11 (`paws-sage/rumour`): the original reads
            // `Fair 'nuff?" Yes` then a blank row; this engine printed no
            // echo at all and ran two rows short from there.
            if sage_confirm_accepted {
                state.emit_message_line_continuing_row("Yes");
                state.push_explicit_blank_message_entry();
            }
            let mut message = match outcome {
                // `shops.md §8.C` sage result table: "Unknown topic |
                // `"That, I cannot help thee with.\n\n`, then repeat the topic
                // question beginning with `Of what wouldst`". The reply opens
                // the quote the repeated question closes. The engine printed
                // the sentence bare and never asked again (playtest E55).
                SageOutcome::NoTopicMatch => format!(
                    "\"That, I cannot help thee with.\n\n{}",
                    sage_topic_question(speaker_is_female)
                ),
                outcome => format_sage_outcome_with_shoppe(
                    outcome,
                    area_scene_byte.and_then(|scene| {
                        crate::play_state_impl::shop_vendor_name_for_scene(
                            SHOP_DIALOG_ID_TAVERN,
                            scene,
                        )
                    }),
                    speaker_is_female,
                    game_dir,
                ),
            };
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
            }
            let mut appended_continuation = false;
            if returns_to_continuation
                && let Some(tavern) =
                    area_scene_byte.and_then(crate::shop_session::tavern_for_scene)
            {
                appended_continuation = true;
                if paid {
                    message.push_str("\n\n");
                }
                message.push_str(TAVERN_ANYTHING_ELSE_PROMPT);
                replacement_session = Some(ActiveShopSession::Tavern(
                    crate::shop_runtime::TavernState::AnythingElse {
                        tavern,
                        continuation_ready: true,
                    },
                ));
            }
            let surcharge = if paid {
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            // The fee question `Fair 'nuff?" ` and the tavern continuation
            // it returns to both leave their row open for the `Yes`/`No`
            // echo; see the tavern arm above. Every other sage outcome either
            // ends the visit or hands the row to the typed-topic reader, and
            // states nothing.
            prompt_row_open = (asks_fee || appended_continuation).then_some(true);
            shop_message_ignoring_surcharge(message, surcharge)
        }
        ActiveShopSession::Reagent(s) => {
            let confirming = matches!(*s, ReagentShopState::ConfirmBundle { .. });
            let mut stock = state.reagents;
            let outcome = step_reagent_shop(
                s,
                ReagentShopInput::Key(key_byte),
                &mut state.gold,
                &mut stock,
                ctx.speaker_intelligence,
            );
            state.reagents = stock;
            // `shops.md §8.A`: the offer's answer echoes a bare `Yes`/`No`.
            match outcome {
                ReagentShopOutcome::Bought { .. } | ReagentShopOutcome::RefusedShortFunds { .. }
                    if confirming =>
                {
                    state.emit_message_line_continuing_row("Yes");
                }
                ReagentShopOutcome::Declined { .. } => {
                    state.emit_message_line_continuing_row("No")
                }
                // `shops.md §8.B`: "Space means No where accepted. Every No
                // response echoes `No`"; Y/N take the shared entry echo.
                // The greeting's continuation row is open whatever the slot's
                // text ended on, as at the inn and the shipwright (playtest
                // H29).
                ReagentShopOutcome::DeclinedEntry if key_byte == b' ' => {
                    state.flush_message_slot();
                    state.open_message_row();
                    state.emit_message_line_continuing_row("No")
                }
                _ => {}
            }
            if matches!(outcome, ReagentShopOutcome::Bought { .. }) {
                state.arms_transaction_completed = true;
            }
            let vendor_name = area_scene_byte.and_then(|scene| {
                crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_HERBALIST, scene)
            });
            let message = match outcome {
                ReagentShopOutcome::DeclinedEntry => {
                    shop_closing_bark(state, SHOP_DIALOG_ID_HERBALIST, vendor_name, game_dir)
                }
                ReagentShopOutcome::LeftMenu => shop_closing_bark_after_open_prompt(
                    state,
                    SHOP_DIALOG_ID_HERBALIST,
                    vendor_name,
                    game_dir,
                ),
                outcome => format_reagent_outcome(outcome, vendor_name, game_dir),
            };
            message
        }
        ActiveShopSession::HorseTrader(s) => {
            let vendor = area_scene_byte.and_then(|scene| {
                crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_STABLE, scene)
            });
            let prior = *s;
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
                        *s = HorseTraderState::Exited;
                        HorseTraderOutcome::RefusedShortFunds { price }
                    } else if let Some((x, y)) = horse_sale_position(state) {
                        match state.buy_horse_for_price(stable, price, x, y) {
                            Ok(_) => {
                                *s = HorseTraderState::Exited;
                                HorseTraderOutcome::Purchased { price }
                            }
                            Err(_) => {
                                *s = HorseTraderState::Exited;
                                HorseTraderOutcome::RefusedNoMarker { price }
                            }
                        }
                    } else {
                        *s = HorseTraderState::Exited;
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
            let confirming = matches!(prior, HorseTraderState::ConfirmPurchase { .. });
            // `shops.md §8.C` "Horse traders": "Its Y/N-only confirmation
            // echoes `No` on decline or `Yes!` on acceptance, without a
            // trailing line feed." At entry the shared echo covers `Y`/`N`;
            // Space "means No" and echoes it here.
            match outcome {
                HorseTraderOutcome::Declined if confirming => {
                    state.emit_message_line_continuing_row("No");
                }
                // The greeting's continuation row is open whatever the slot's
                // text ended on (playtest H29).
                HorseTraderOutcome::Declined if key_byte == b' ' => {
                    state.flush_message_slot();
                    state.open_message_row();
                    state.emit_message_line_continuing_row("No");
                }
                HorseTraderOutcome::Purchased { .. }
                | HorseTraderOutcome::RefusedShortFunds { .. }
                | HorseTraderOutcome::RefusedNoMarker { .. }
                    if confirming =>
                {
                    state.emit_message_line_continuing_row("Yes!");
                }
                _ => {}
            }
            let surcharge = if matches!(outcome, HorseTraderOutcome::Purchased { .. }) {
                state.arms_transaction_completed = true;
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            let message = match outcome {
                // `shops.md §8.B`: `Yes\n\n"`, then record `104`, then
                // `\n\nDeal?" `, with the adjusted price through `%`.
                HorseTraderOutcome::QuotedPrice { price } => {
                    let body = render_shared_shoppe_record(
                        game_dir,
                        HORSE_TRADER_OFFER_RECORD,
                        price,
                        vendor,
                        "",
                    );
                    if body.is_empty() {
                        format_horse_trader_outcome(outcome, vendor)
                    } else {
                        format!("\"{body}{HORSE_TRADER_DEAL_PROMPT}")
                    }
                }
                // `shops.md §8.C`: "Successful placement uses the sale
                // farewell; entry/offer decline uses no-sale." Measured
                // 2026-09-08 at North Britanny's stable at noon
                // (`qa/paired/nb-stable-open.tsv`): `"Fare thee well!"` over
                // `says Theoan.` with no success line of its own.
                HorseTraderOutcome::Purchased { .. } | HorseTraderOutcome::Declined => {
                    let bark = shop_closing_bark(state, SHOP_DIALOG_ID_STABLE, vendor, game_dir);
                    if bark.is_empty() {
                        format_horse_trader_outcome(outcome, vendor)
                    } else {
                        bark
                    }
                }
                outcome => format_horse_trader_outcome(outcome, vendor),
            };
            shop_message_ignoring_surcharge(message, surcharge)
        }
        ActiveShopSession::ShipBroker(s) => {
            // Read before the mutable borrows below; `shops.md §6.1` adjusts
            // the shipwright's base rows by the speaker's Intelligence.
            let speaker_intelligence = active_speaker_intelligence(state);
            // `shops.md §8.B`: record `119` "carries its own leading spacing
            // and prompt", and it ends on a trailing space, so the accepted
            // `F`/`S` echoes onto that open row - the same placement the
            // tavern's menu letter uses. Measured 2026-09-12
            // (`bd-shipwright/frigate`), where the original reads
            // `like to see?" F`.
            let shipwright_letter_echo = matches!(*s, ShipBrokerState::Menu { .. })
                .then(|| match key_byte {
                    b'F' | b'f' => Some('F'),
                    b'S' | b's' => Some('S'),
                    _ => None,
                })
                .flatten();
            // The pending delivery is scratch for the step function; the
            // durable copy is `pending_vehicle_save`, which
            // `sync_pending_vehicle_purchase_state` writes below and which the
            // town-exit and dungeon-return paths both read.
            //
            // This branch used to be gated on `return_world` being present so
            // it could borrow that record's copy - but `return_world` is only
            // populated when entering a *dungeon*, and every shipwright is in
            // a town. The live game therefore never satisfied the gate: `Y` at
            // the greeting fell through to `InvalidInput` and printed nothing,
            // so the Frigate/Skiff menu was unreachable. Measured 2026-09-10
            // (`bd-shipwright/yes`, both sides on the same greeting record).
            // The end-to-end test set `return_world` by hand, which is why it
            // passed throughout.
            let mut pending = state
                .return_world
                .as_ref()
                .map(|world| world.pending_vehicle)
                .unwrap_or_else(|| state.pending_vehicle_save.acquisition());
            let prior = *s;
            let outcome = match (*s, yes, no) {
                (ShipBrokerState::Greeting { .. } | ShipBrokerState::Menu { .. }, _, _) => {
                    step_ship_broker(
                        s,
                        ShipBrokerInput::Key(key_byte),
                        &mut state.gold,
                        &mut pending,
                        speaker_intelligence,
                    )
                }
                (
                    ShipBrokerState::ConfirmPurchase { .. }
                    | ShipBrokerState::ConfirmDockSpace { .. }
                    | ShipBrokerState::Continuation { .. },
                    true,
                    _,
                ) => step_ship_broker(
                    s,
                    ShipBrokerInput::Confirm(true),
                    &mut state.gold,
                    &mut pending,
                    speaker_intelligence,
                ),
                (
                    ShipBrokerState::ConfirmPurchase { .. }
                    | ShipBrokerState::ConfirmDockSpace { .. }
                    | ShipBrokerState::Continuation { .. },
                    _,
                    true,
                ) => step_ship_broker(
                    s,
                    ShipBrokerInput::Confirm(false),
                    &mut state.gold,
                    &mut pending,
                    speaker_intelligence,
                ),
                _ => ShipBrokerOutcome::InvalidInput,
            };
            if let Some(world) = state.return_world.as_mut() {
                world.pending_vehicle = pending;
            }
            if let ShipBrokerOutcome::PurchaseApplied { outcome: purchase } = outcome {
                state.sync_pending_vehicle_purchase_state(purchase);
            }
            let purchase_completed = matches!(
                &outcome,
                ShipBrokerOutcome::PurchaseApplied { outcome }
                    if matches!(
                        outcome.status,
                        crate::shops::ShipwrightPurchaseStatus::QueuedFrigate
                            | crate::shops::ShipwrightPurchaseStatus::QueuedSkiff
                            | crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate
                    )
            );
            let surcharge = if purchase_completed {
                state.arms_transaction_completed = true;
                apply_active_shop_surcharge(state)
            } else {
                None
            };
            let vendor_name = area_scene_byte.and_then(|scene| {
                crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_SHIPWRIGHT, scene)
            });
            let record = |id: usize, state: &PlayState| {
                render_shipwright_record(game_dir, id, vendor_name, state)
            };
            // `§8.B`: the Frigate/Skiff menu is record `119`, "with no added
            // resident menu question; the record carries its own leading
            // spacing and prompt".
            let menu_record = record(crate::shops::SHOPPE_RECORD_SHIPWRIGHT_MENU, state);
            // `§8.C`'s shipwright result table, "Ordinary offer": "Record
            // `117` for Frigate or `118` for Skiff, then record `126`". The
            // engine used to print its own `A frigate costs 700 gold.`
            // sentence here, on the grounds that the record ids were
            // unpublished; they are published now. Measured 2026-09-12
            // (`bd-shipwright/frigate`), where the original renders the
            // vessel's own body and its price differs from the engine's.
            let offer_record = match &outcome {
                ShipBrokerOutcome::QuotedPurchase { quote } => {
                    let body = match quote.kind {
                        crate::shops::ShipwrightPurchaseKind::Frigate => {
                            crate::shops::SHOPPE_RECORD_SHIPWRIGHT_FRIGATE_OFFER
                        }
                        crate::shops::ShipwrightPurchaseKind::Skiff => {
                            crate::shops::SHOPPE_RECORD_SHIPWRIGHT_SKIFF_OFFER
                        }
                    };
                    render_shipwright_offer(game_dir, body, vendor_name, quote.price, state)
                }
                ShipBrokerOutcome::QuotedDockSpace => {
                    record(crate::shops::SHOPPE_RECORD_SHIPWRIGHT_DOCK_SPACE, state)
                }
                _ => None,
            };
            if let Some(letter) = shipwright_letter_echo {
                // `§8.C` "Shipwrights": "F and S echo with `\n\n`" - the
                // letter closes the menu's open prompt row and one blank row
                // follows. The blank leaves the cursor at column 0, so the
                // row state is set explicitly rather than inferred.
                state.emit_message_line_continuing_row(letter.to_string());
                state.push_explicit_blank_message_entry();
                state.close_message_row();
            }
            // `§8.C` "Shipwrights": the ordinary offer's answer echoes `No` or
            // `Yes\n\n`; the dock-space and continuation answers echo a bare
            // `Yes` or `No`. At entry the shared echo covers `Y`/`N` and Space
            // "means No".
            match prior {
                ShipBrokerState::ConfirmPurchase { .. } if yes => {
                    state.emit_message_line_continuing_row("Yes");
                    state.push_explicit_blank_message_entry();
                }
                ShipBrokerState::ConfirmDockSpace { .. } | ShipBrokerState::Continuation { .. }
                    if yes =>
                {
                    state.emit_message_line_continuing_row("Yes");
                }
                ShipBrokerState::ConfirmPurchase { .. }
                | ShipBrokerState::ConfirmDockSpace { .. }
                | ShipBrokerState::Continuation { .. }
                    if no =>
                {
                    state.emit_message_line_continuing_row("No");
                }
                ShipBrokerState::Greeting { .. } if key_byte == b' ' => {
                    // The greeting's `" ` row is open; see the entry echo below.
                    state.flush_message_slot();
                    state.open_message_row();
                    state.emit_message_line_continuing_row("No");
                }
                _ => {}
            }
            // `§8.C` "Shipwrights": "Invalid service keys redraw record
            // `119`". The redraw is the same text the menu already printed,
            // so leaving it in the slot was indistinguishable from no write
            // and the flush dropped it (playtest F50). Emit it outright; the
            // record's own leading feeds close the open `see?" ` row and
            // leave one blank above the second copy.
            let menu_redraw = matches!(prior, ShipBrokerState::Menu { .. })
                && matches!(outcome, ShipBrokerOutcome::EnteredMenu);
            let message = match outcome {
                // `§8.A`/`§8.C`: "Ordinary ship endings use the shared
                // farewell", nothing-bought or purchase-completed by the visit.
                //
                // The bark follows the `No` echo on the row the question left
                // open, so it opens with two feeds: the close and the blank.
                // Measured 2026-10-16 (`bd-shipwright-20261016-215905`, beat
                // `no`, and every stock capture back to 2026-09-18): the
                // original reads `take it?" No`, a blank row, then the bark;
                // this engine had the `No` on its own row and no blank.
                ShipBrokerOutcome::Declined => {
                    shop_closing_bark(state, SHOP_DIALOG_ID_SHIPWRIGHT, vendor_name, game_dir)
                }
                // `§8.C` "Affordability failure | Record `122`, then
                // token-expanded `yells $.\n`; suppress ordinary farewell".
                ShipBrokerOutcome::RefusedShortFunds { .. } => {
                    let mut text = record(crate::shops::SHOPPE_RECORD_SHIPWRIGHT_SHORT_FUNDS, state)
                        .unwrap_or_default();
                    if let Some(name) = vendor_name {
                        text.push_str(&format!("\nyells {name}.\n"));
                    }
                    text
                }
                // `§8.C`: records `123`/`120`/`121` for the three statuses,
                // then "Record `124`, honorific, then `?" `". The shipped
                // honorific test never matches, so it is always `sir`.
                ShipBrokerOutcome::PurchaseApplied { outcome: purchase } => {
                    let status_record = match purchase.status {
                        crate::shops::ShipwrightPurchaseStatus::QueuedFrigate
                        | crate::shops::ShipwrightPurchaseStatus::QueuedSkiff => {
                            crate::shops::SHOPPE_RECORD_SHIPWRIGHT_DELIVERY_QUEUED
                        }
                        crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate => {
                            crate::shops::SHOPPE_RECORD_SHIPWRIGHT_SKIFF_STOWED
                        }
                        crate::shops::ShipwrightPurchaseStatus::ExistingDeliveryRefusal => {
                            crate::shops::SHOPPE_RECORD_SHIPWRIGHT_ALREADY_SKIFF
                        }
                    };
                    format!(
                        "{}{}{SHIPWRIGHT_CONTINUATION_TAIL}",
                        record(status_record, state).unwrap_or_default(),
                        record(crate::shops::SHOPPE_RECORD_SHIPWRIGHT_CONTINUATION, state)
                            .unwrap_or_default(),
                    )
                }
                outcome => format_ship_broker_outcome(outcome, menu_record, offer_record),
            };
            if menu_redraw && !message.is_empty() {
                state.emit_message_line(message.clone());
            }
            shop_message_ignoring_surcharge(message, surcharge)
        }
        ActiveShopSession::Guild(s) => {
            let mut gems = state.gems;
            let mut keys = state.keys;
            let mut torches = state.torches;
            let confirming = matches!(*s, GuildShopState::ConfirmBundle { .. });
            let outcome = step_guild_shop(
                s,
                GuildShopInput::Key(key_byte),
                &mut state.gold,
                &mut gems,
                &mut keys,
                &mut torches,
                ctx.speaker_intelligence,
            );
            state.gems = gems;
            state.keys = keys;
            state.torches = torches;
            // `shops.md §8.C`: "A/B/C echo the corresponding lowercase
            // letter"; the offer's answer echoes a bare `Yes` or `No`.
            match outcome {
                GuildShopOutcome::Offered { .. } => {
                    state.emit_message_line_continuing_row(
                        (key_byte.to_ascii_lowercase() as char).to_string(),
                    );
                }
                GuildShopOutcome::Bought { .. } | GuildShopOutcome::RefusedShortFunds { .. }
                    if confirming =>
                {
                    state.emit_message_line_continuing_row("Yes");
                }
                GuildShopOutcome::Declined => state.emit_message_line_continuing_row("No"),
                // `shops.md §8.B`: "Space means No where accepted. Every No
                // response echoes `No`"; Y/N take the shared entry echo.
                // The greeting's continuation row is open whatever the slot's
                // text ended on (playtest H29).
                GuildShopOutcome::DeclinedEntry if key_byte == b' ' => {
                    state.flush_message_slot();
                    state.open_message_row();
                    state.emit_message_line_continuing_row("No")
                }
                _ => {}
            }
            if matches!(outcome, GuildShopOutcome::Bought { .. }) {
                state.arms_transaction_completed = true;
            }
            let vendor_name = area_scene_byte.and_then(|scene| {
                crate::play_state_impl::shop_vendor_name_for_scene(SHOP_DIALOG_ID_GUILD, scene)
            });
            let message = match outcome {
                GuildShopOutcome::DeclinedEntry => {
                    shop_closing_bark(state, SHOP_DIALOG_ID_GUILD, vendor_name, game_dir)
                }
                GuildShopOutcome::LeftMenu => shop_closing_bark_after_open_prompt(
                    state,
                    SHOP_DIALOG_ID_GUILD,
                    vendor_name,
                    game_dir,
                ),
                outcome => format_guild_outcome(
                    outcome,
                    vendor_name,
                    speaker_is_female,
                    game_dir,
                ),
            };
            // `§8.C` D: record `164` prints straight after `Thy concern?" `,
            // on the row that prompt left open (the prompt waits with its row
            // open - `keeps_cursor_inline`), so it is emitted continuing that
            // row rather than left in the slot, whose flush starts a fresh
            // row. The emitter states the row the menu leaves open.
            // Without a readable record there is nothing to continue the row with,
            // and the redrawn menu takes its ordinary fresh row.
            if matches!(outcome, GuildShopOutcome::Described { .. })
                && message.len() > guild_stock_menu().len()
            {
                state.open_message_row();
                state.emit_message_line_continuing_row(message.clone());
            }
            message
        }
    };
    if let Some((echo, opens_speech)) = arms_entry_echo {
        state.emit_message_line_continuing_row(echo);
        if opens_speech {
            // `Buy\n\n"` / `Sell\n\n"`: the echo closes its row and a blank
            // follows before the quoted speech the outcome carries. Space's
            // `No` takes no line feed, so the closing bark continues its row.
            state.push_explicit_blank_message_entry();
        }
    }
    if let Some(Some(echo)) = entry_answer_echo {
        // `shops.md §8.B`: "Where a colon is emitted, the accepted answer
        // appends directly after it, producing `:Yes` or `:No`; do not insert
        // an intervening space. A greeting ending in columns 1 through 11
        // instead receives the single-space continuation shown above." The
        // greeting already placed that space or that colon, so the echo is
        // the bare word either way; the engine used to supply a space of its
        // own and so double-spaced the tavern's row and put the healer's
        // answer on a row with no colon.
        //
        // Every continuation the greeting can end on - `\n:`, one space,
        // `\n\n:` - leaves the cursor on that row, so the row is stated open
        // here rather than inferred from text. Since the continuing-row
        // emitter started honouring the flag (`6493e0d9`) the greeting's
        // producer blanks left it closed, and `Yes` dropped to a row of its
        // own - measured 2026-10-16 (`paws-tavern`, beat `menu`): the
        // original reads `afternoon?" Yes`.
        //
        // The greeting's tail - `" ` or the colon - leaves its row open, so
        // the slot that printed it is flushed and the row stated open here
        // rather than read from whatever the last `emit_*` call left. Measured
        // 2026-10-16 (`bd-shipwright-20261016-215905`, beat `yes`): the
        // original reads `vessels?" Yes`; this engine put the `Yes` on a row
        // of its own from the 2026-10-13 runs on.
        state.flush_message_slot();
        state.open_message_row();
        state.emit_message_line_continuing_row(echo);
        // `§8.B`'s per-kind table gives the Yes branches `Yes\n\n` before
        // their service text - except the shipwright, which prints "`Yes`,
        // then record `119` ... the record carries its own leading spacing".
        // Measured 2026-10-16 (`bd-shipwright-20261016-215905`, beat `yes`, and
        // every engine capture back to 2026-09-18): the original has one blank
        // row between `Yes` and the menu; this engine had three, its own blank
        // plus the record's two feeds spent against a closed row.
        let record_owns_spacing = matches!(session, ActiveShopSession::ShipBroker(_));
        if echo == "Yes" && !record_owns_spacing {
            state.push_explicit_blank_message_entry();
        }
    }
    state.message = message;
    // The flag describes the row the message *ends* on, so it is stated
    // after the message is on the transcript: flushing a message with
    // embedded blank rows rewrites it on the way (each producer blank closes
    // the row), and a flag set before the flush was lost there.
    if let Some(open) = prompt_row_open {
        state.flush_message_slot();
        if open {
            state.open_message_row();
        } else {
            state.close_message_row();
        }
    }
    // The arms overlay's waits all sit on a row their own prompt left open:
    // the entry question, the stock call, the buy quote, the sell browser's
    // question and its `Deal?" ` all end in a closing quote and one space
    // (`shops.md` §8.B, §8.1), and the next key's echo - `Sell`, the stock
    // letter, `No`, `Yes` - "appends after the space" on that row. The slot
    // is flushed verbatim and never says where it left the cursor, so this
    // caller states it: flush now, then mark the row open.
    //
    // Measured 2026-10-16 (`shop-arms-sell-flow-20261016-223816`, beats
    // `browser`, `declined` and `sold`; `shop-arms-buy-confirm-20261016-214848`
    // beat `quote`): the original reads `or Sell?" Sell`, `Deal?" No`,
    // `Deal?" Yes` and `interest?" a` on one row, where this engine had put
    // every echo on a row of its own since the 2026-10-14 runs, because the
    // flag still held whatever the last `emit_*` call had left.
    let arms_row_left_open = !session.is_exited()
        && matches!(
            session,
            ActiveShopSession::Arms(_)
                | ActiveShopSession::ArmsLocal(..)
                | ActiveShopSession::ArmsStocked(..)
                | ActiveShopSession::ShipBroker(_)
        )
        && session.keeps_cursor_inline()
        && !state.message.is_empty();
    if arms_row_left_open {
        state.flush_message_slot();
        state.open_message_row();
    }
    // Every other kind's quoted key prompts (`shops.md §8.B`/`§8.C`, the
    // states [`ActiveShopSession::keeps_cursor_inline`] lists) wait the same
    // way, and their `Yes`/`No`/letter echoes continue the prompt's row. The
    // flush is verbatim and states nothing, so this caller states it from
    // the state the session now waits in: each of those states waits on a
    // question that ends on a closing quote and one space, so its row is
    // open - unless the tavern/sage arm already stated the row itself.
    else if prompt_row_open.is_none()
        && !session.is_exited()
        && session.keeps_cursor_inline()
        && !state.message.is_empty()
    {
        state.flush_message_slot();
        state.open_message_row();
    }

    if let Some(next_session) = replacement_session {
        state.active_shop = Some(next_session);
    } else if !session.is_exited() {
        state.active_shop = Some(session);
    }
    PlayInputDisposition::Continue
}

/// `inventory.md §4.3`'s shared member selector, for the shop callers that
/// keep its highlight in their own state (inn Leave, the healer): "`1`
/// through `6`, bounded by the party size, reposition the inverted row
/// exactly as the direction keys do and leave the prompt open". Any other
/// key leaves the highlight where it is.
fn shop_party_selector_move(
    state: &PlayState,
    highlight: usize,
    key: char,
    digit: Option<u8>,
) -> usize {
    let last = state.party.len().saturating_sub(1);
    if let Some(forward) = crate::z_stats::party_selector_direction_step(key) {
        return if forward {
            highlight.saturating_add(1).min(last)
        } else {
            highlight.saturating_sub(1)
        };
    }
    match digit {
        Some(d) if d >= 1 && usize::from(d) <= state.party.len() => usize::from(d - 1),
        _ => highlight,
    }
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

pub(crate) fn active_speaker_intelligence(state: &PlayState) -> u8 {
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
///
/// `shops.md §8.B`: `"What is the nature of thy need?" ` - the closing quote
/// and one space are the cell the service echo lands in. Playtest F47: the
/// engine stopped at the quote and read `need?"Curing`.
const HEALER_SERVICE_QUESTION: &str = "\"What is the nature of thy need?\" ";

/// `shops.md §8.C`'s healer result table: "Selected member is untreatable |
/// Token-expanded `\n\n"Thou hast no need of this art!"\nsays $.`", then
/// separately "Continuation after treatment, refusal, or member cancellation |
/// `\n\n"Is there any other way in which I may\n`, then `aid thee?" `".
fn healer_no_need_refusal(name: Option<&'static str>) -> String {
    // The leading `\n\n` is the published text's: the selector's question
    // (or the service echo, for a sole member) leaves its row open, so the
    // first feed closes it and the second is the one blank row. Playtest F47.
    let refusal = "\n\n\"Thou hast no need of this art!\"";
    match name {
        Some(name) => format!("{refusal}\nsays {name}.{HEALER_CONTINUATION}"),
        None => format!("{refusal}{HEALER_CONTINUATION}"),
    }
}

/// §8.C: the continuation the visit resumes on. Its own leading line feeds
/// and its trailing space are the published text's.
const HEALER_CONTINUATION: &str = "\n\n\"Is there any other way in which I may\naid thee?\" ";

/// §8.C "Minoc free Cure/Heal | `\n\n"Receive now the Light!"`".
const HEALER_RECEIVE_THE_LIGHT: &str = "\n\n\"Receive now the Light!\"";

/// §8.C "Ordinary short funds | Record `173`".
const HEALER_SHORT_FUNDS_RECORD: usize = 173;

/// Render one `SHOPPE.DAT` record with `%` set to `gold`, or nothing when the
/// asset is unavailable.
fn render_shared_shoppe_record(
    game_dir: &Path,
    record_id: usize,
    gold: u16,
    vendor_name: Option<&str>,
    shop_name: &str,
) -> String {
    crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
        .ok()
        .and_then(|renderer| {
            renderer
                .render_record(
                    record_id,
                    &crate::shoppe_bark::ShoppeBarkContext {
                        gold,
                        vendor_name: vendor_name.unwrap_or_default(),
                        shop_name,
                        ..Default::default()
                    },
                )
                .ok()
        })
        .unwrap_or_default()
}

/// §8.C's three paid introductions. The fee row completes each of them.
fn healer_paid_introduction(treatment: crate::shops::HealerTreatment) -> &'static str {
    match treatment {
        // Each opens with §8.C's own `\n\n`: one feed closes the open
        // selector (or echo) row, the other is the blank above the reply.
        // Playtest F47: the reply started on the very next row.
        crate::shops::HealerTreatment::Cure => "\n\n\"I can cure thy poisoned body ",
        crate::shops::HealerTreatment::Heal => "\n\n\"I can heal thee ",
        crate::shops::HealerTreatment::Resurrect => {
            "\n\n\"I can raise this unfortunate person from the dead "
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
        // §8.C: "C/H/R echo `Curing`, `Healing`, or `Resurrect`". Playtest
        // F45: the engine invented `Resurrecting`.
        crate::shops::HealerTreatment::Resurrect => "Resurrect",
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
const SHOP_DIALOG_ID_HERBALIST: u8 = 0x85;
const SHOP_DIALOG_ID_GUILD: u8 = 0x86;
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

/// `shops.md §6.2`: the Falsehood surcharge "is a gold-side effect only; it
/// does not change shop stock, item quantities, or the quoted record text."
/// The player sees and confirms the displayed price and then simply has less
/// gold, which is what makes Faulinei's presence something to deduce rather
/// than something the window announces.
///
/// So the outcome is deliberately dropped here rather than rendered. The
/// parameter stays because every paid branch must still *run* the gate at its
/// §6.2 point, and threading the result to this one place keeps that visible.
fn shop_message_ignoring_surcharge(
    message: String,
    _surcharge: Option<ShopSurchargeOutcome>,
) -> String {
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
    // `shops.md §8.C`, "Sleep / morning": `Zzzzzz....\n\n` - four dots -
    // "followed at morning by `Morning!\n`". The hours, the fee and the
    // recovery totals are engine bookkeeping, not the original's text.
    state.diagnostics.push(format!(
        "paid inn rest {hours}h for {cost} gold; recovered {recovered_hp} HP and \
         {recovered_mana} MP; cured {cured} poisoned member(s); woke {woke} asleep member(s)"
    ));
    format!(
        "{}\n\n{}",
        crate::commands::PAID_INN_REST_SLEEP_LINE,
        crate::commands::PAID_INN_REST_MORNING_LINE
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
    // `shops.md §8.1`: after the carry-cap refusal the shop "waits for one
    // key, and returns to the buy list"; "A purchase, decline, or carry-cap
    // refusal prints the post-item prompt and then the whole listing with a
    // new stock-call draw; the next valid item letter starts another quote
    // without another `B`." The waited key is consumed (playtest E46).
    if matches!(prior_state, ArmsShopState::BuyCapRefusalWait) {
        *shop_state = ArmsShopState::BuyPickItem;
        let Some(table) = stock_table else {
            return String::new();
        };
        let post = arms_post_item_prompt(
            active_speaker_is_female(state),
            state.arms_transaction_completed,
        );
        let listing = format_arms_stock_buy_menu(table, None);
        let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
        return format!("{post}\n\n{listing}\n{call}");
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
                let outcome = step_arms_shop(
                    shop_state,
                    ArmsShopInput::StockLetter {
                        letter: key_byte,
                        table,
                    },
                    ctx,
                    &mut state.gold,
                    &mut stock,
                    &prices,
                );
                // The accepted stock letter echoes onto the row the stock-call
                // question left open, exactly as `shops.md §8.B`'s entry answer
                // does - the question's tail is `" `, a closing quote and one
                // space, so the cursor is sitting there. Measured 2026-09-09
                // (`qa/paired/shop-arms-menus.tsv`, beat `buyquote`): the
                // original shows `see?" a`, a blank row, then the quote.
                // §8.1: "Invalid listing keys keep the existing text and
                // consume no new draw", so only an accepted letter echoes.
                if matches!(outcome, ArmsShopOutcome::QuotedBuyPrice { .. }) {
                    state.emit_message_line_continuing_row(
                        (key_byte as char).to_ascii_lowercase().to_string(),
                    );
                    state.push_explicit_blank_message_entry();
                }
                outcome
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
            emit_arms_confirm_echo(state, prior_state, "Yes");
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
            emit_arms_confirm_echo(state, prior_state, "No");
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
        (ArmsShopOutcome::InvalidInput, Some(_)) if was_invalid_stock_pick => {
            // `§8.A`'s "Arms buy stock list" row: invalid stock letters "leave
            // the stock list visible and keep waiting; they do not redraw the
            // list or consume a random draw", and `§8.1` repeats it - "Invalid
            // listing keys keep the existing text and consume no new draw."
            // This arm re-rendered the listing, which appended a second copy
            // under the first. Measured 2026-09-10 (`shop-arms-menus/sell`,
            // where the scripted `S` is not a stock letter): the original's
            // window is unchanged from the previous beat while this engine had
            // grown a duplicate list. Keeping the existing text is the same
            // thing the sell-side sibling arm below already does.
            state.message.clone()
        }
        (ArmsShopOutcome::InvalidInput, _)
            if matches!(
                prior_state,
                ArmsShopState::SellPickItem(_) | ArmsShopState::SellConfirm { .. }
            ) =>
        {
            state.message.clone()
        }
        // `shops.md §8.A`, "Arms item quote and confirmation": "Only `Y` and
        // `N` advance. Other keys leave the quote/prompt visible and do not
        // redraw or consume a random draw." Re-rendering the quote here
        // swapped the drawn confirmation prompt for the undrawn fallback
        // variant (playtest E54).
        (ArmsShopOutcome::InvalidInput, _)
            if matches!(prior_state, ArmsShopState::BuyConfirm { .. }) =>
        {
            state.message.clone()
        }
        (ArmsShopOutcome::InvalidInput, _) => {
            format_arms_outcome(ArmsShopOutcome::InvalidInput, game_dir)
        }
        (ArmsShopOutcome::EnteredSell, _) => arms_sell_entry_prompt(state.random_range_u8(0, 3)),
        (ArmsShopOutcome::SellBrowserMoved, _) => state.message.clone(),
        (ArmsShopOutcome::OfferedSellPrice { item, offer }, _) => {
            let record_id = crate::shops::SHOPPE_RECORDS_ARMS_SELL_FIRST
                + usize::from(state.random_range_u8(0, 7));
            let quote = render_shoppe_record_for_arms_quote(game_dir, record_id, item, offer);
            // Measured 2026-09-07 (`qa/paired/shop-arms-sell.tsv`): the offer
            // record, a blank row, then `Deal?"` - the closing quote of the
            // shopkeeper's speech, and no `(Y/N)`.
            //
            // The closing quote is followed by one space, which is the cell
            // the accepted `Yes`/`No` echoes into - the same tail shape §8.1
            // gives the stock-call question and §8.B the entry greeting.
            // Measured 2026-09-10 (`shop-arms-sell-flow/sold`): the original
            // reads `Deal?" Yes`, this engine read `Deal?"Yes`.
            //
            // `shops.md` §8.1: "An ordinary offer starts with `\n\n\"`" - the
            // first feed closes the row the browser question left open, the
            // second leaves the blank, and the quote opens the speech that
            // `Deal?" ` closes. Measured 2026-10-16
            // (`shop-arms-sell-flow-20261016-223816`, beat `offer`): the
            // original has the blank row and the opening quote; this engine
            // had neither.
            format!("\n\n\"{quote}\n\nDeal?\" ")
        }
        (ArmsShopOutcome::SellRefusedZeroPrice { .. }, _)
            if matches!(shop_state, ArmsShopState::SellPickItem(_)) =>
        {
            // **Measured** 2026-09-07 (`qa/paired/shop-arms-sell-zero-price.tsv`):
            // quoted and attributed with `says` - unlike the ammunition
            // refusal's `growls` - and the browser keeps going with its
            // continuation prompt.
            // The refusal and the continuation prompt each follow two line
            // feeds, the same spacing the sale and the decline take. Measured
            // 2026-09-10 (`shop-arms-sell-zero-price/ammo` and `/after`): the
            // original has a blank row before the refusal and another between
            // its attribution and the prompt; this engine had neither, so the
            // browser transcript ran two rows short.
            // §8.1: `\n\n"That, I cannot buy from thee."\nsays $.` after the
            // browser question, whose row is still open.
            format!(
                "\n\n{}\n\n{}",
                speech.attribute("\"That, I cannot buy from thee.\"", "says"),
                arms_sell_continuation_prompt(state.random_range_u8(0, 3))
            )
        }
        // Measured: a completed purchase and a declined quote both leave the
        // Buy listing on screen with a freshly drawn call line under it, and
        // the next letter buys again without a second `B`.
        (ArmsShopOutcome::Bought { .. }, Some(table)) => {
            state.arms_transaction_completed = true;
            let post = arms_post_item_prompt(speech.speaker_is_female, true);
            let listing = format_arms_stock_buy_menu(table, None);
            let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
            format!("Sold!\n{post}\n\n{listing}\n{call}")
        }
        // `shops.md §8.1`: "A decline or carry-cap refusal uses this same
        // prompt, retaining whether an earlier purchase completed." This arm
        // printed the redrawn listing alone, so a declined quote lost the
        // prompt entirely. Measured 2026-09-10: `shop-arms-menus/buydecline`
        // has the original on `then?` with nothing bought yet, and
        // `shop-arms-buy-confirm/declined` on `sir?` after a purchase - the
        // two suffixes the retained flag selects between.
        (ArmsShopOutcome::Declined, Some(table))
            if matches!(prior_state, ArmsShopState::BuyConfirm { .. }) =>
        {
            let post =
                arms_post_item_prompt(speech.speaker_is_female, state.arms_transaction_completed);
            let listing = format_arms_stock_buy_menu(table, None);
            let call = arms_stock_call_for_roll(state.random_range_u8(0, 3));
            // `shops.md §8.A`, "Arms buy decline echo": "`N` prints `No`
            // followed by a blank line". The `No` continues the prompt's
            // row; the first feed closes it, the second is the blank.
            format!("\n\n{post}\n\n{listing}\n{call}")
        }
        (ArmsShopOutcome::Declined, _) if matches!(shop_state, ArmsShopState::SellPickItem(_)) => {
            // The `No` is echoed onto the open `Deal?" ` row above, not
            // printed here on a row of its own, and the continuation prompt
            // follows two line feeds. Measured 2026-09-10
            // (`shop-arms-sell-flow/declined`): the original reads
            // `Deal?" No`, a blank row, then the prompt.
            //
            // `shops.md` §8.1: "The continuation supplies its own leading two
            // line feeds after either the decline echo" - the first closes the
            // `Deal?" No` row, the second is the blank.
            format!(
                "\n\n{}",
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
            // Measured 2026-09-10 (`shop-arms-sell-flow/sold`): the original
            // reads `Deal?" Yes`, a blank row, `"Done!"` over its
            // attribution, another blank row, then the continuation prompt.
            // `shops.md` §8.1: `Yes\n\n"Done!"\nsays $.` - the `Yes` echo
            // leaves its row open, so two feeds give the blank above `Done!`.
            "\n\n{}\n\n{}",
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
            // The sale that empties the browser takes the same spacing, with
            // the local goodbye where the continuation prompt would be. Not
            // separately measured.
            // `shops.md` §8.1: the goodbye's `says $.` follows only "If any
            // equipment remains ... When the final carried item has been
            // sold, the quoted goodbye still prints but this attribution is
            // omitted." Playtest F44: the engine attributed it anyway.
            "\n\n{}\n\n{}",
            speech.attribute("\"Done!\"", "says"),
            arms_sell_goodbye(state.random_range_u8(0, 3))
        ),
        (ArmsShopOutcome::Exited, _) if matches!(prior_state, ArmsShopState::SellPickItem(_)) => {
            // Two line feeds before the local goodbye as well. Measured
            // 2026-09-10 (`shop-arms-sell-zero-price/after`).
            // §8.1: "An ordinary browser-local goodbye prints `\n\n\"`" - the
            // browser question's row is open, so both feeds are needed.
            format!(
                "\n\n{}",
                speech.attribute(&arms_sell_goodbye(state.random_range_u8(0, 3)), "says")
            )
        }
        // Measured 2026-09-07 (`qa/paired/shop-arms-buy.tsv`): a visit that
        // ends without a sale closes on a *rendered* shared-band record, not
        // a resident goodbye - `"Thanks for nothing!" says Gwenneth` and
        // `"Harrumph!" says Gwenneth` are records 0 and 2, found with the
        // `shoppe_find` probe. `shops.md` §8.A's uniform `0..3` draw picks
        // between them, so the draw is the record id for this shop's row.
        (ArmsShopOutcome::Exited | ArmsShopOutcome::Declined, _) => {
            let roll = state.random_range_u8(0, 3);
            arms_closing_bark_record(state.arms_transaction_completed, roll)
                .and_then(|record| render_shared_shoppe_flourish(game_dir, record))
                // The farewell follows *two* line feeds, not one: the first
                // closes the row the stock-call question left its cursor on,
                // the second leaves a blank row. `shops.md` §8.C states the
                // rule for the inn's ordinary farewell - "After the bare `No`
                // and two line feeds, end the visit through the ordinary
                // farewell" - and the arms exit measures the same. Measured
                // 2026-09-10 (`shop-arms-menus/exit`,
                // `shop-arms-buy-confirm/exit`): the original has a blank row
                // between the call line and the bark, and this engine put the
                // bark straight onto the next row.
                // The row the bark follows is open (the stock call's `" `, or
                // the entry `No` echo), so the first feed closes it and the
                // second is the blank.
                .map(|flourish| format!("\n\n{}", speech.attribute(&flourish, "says")))
                .unwrap_or_default()
        }
        // `shops.md §8.1`: short funds "exits the whole arms buy flow ...
        // the next thing the player sees is the farewell" - the bark, then
        // the ordinary closing envelope the listing's Space exit prints, with
        // its own draw after the bark's. Playtest E45: the visit stayed open
        // on the Buy/Sell prompt.
        (refusal @ ArmsShopOutcome::BuyRefusedShortFunds { .. }, _) => {
            let bark = format_arms_outcome_with_rolls(
                refusal,
                game_dir,
                confirmation_prompt_roll,
                no_credit_roll,
                speech,
            );
            let roll = state.random_range_u8(0, 3);
            let farewell = arms_closing_bark_record(state.arms_transaction_completed, roll)
                .and_then(|record| render_shared_shoppe_flourish(game_dir, record))
                .map(|flourish| format!("\n\n{}", speech.attribute(&flourish, "says")))
                .unwrap_or_default();
            format!("{bark}{farewell}")
        }
        (outcome, _) => format_arms_outcome_with_rolls(
            outcome,
            game_dir,
            confirmation_prompt_roll,
            no_credit_roll,
            speech,
        ),
    };
    shop_message_ignoring_surcharge(message, surcharge)
}

/// `shops.md §8.1` / `§8.A` "Arms carry-cap refusal (verbatim)".
const ARMS_CARRY_CAP_REFUSAL: &str = "\"Thou canst not carry any more!\"";

/// **Measured** 2026-09-07 (`qa/paired/shop-arms-menus.tsv`,
/// `shop-arms-sell.tsv`): the sell-entry prompt is *quoted* -
/// `"Which item wouldst thou like to sell?"` and
/// `"What dost thou wish to sell?"` both came back with an opening and a
/// closing double quote. The other two lines of the pool did not draw in
/// either capture and take the same shape.
/// `shops.md` §8.1: nonempty Sell entry "inherits its opening double quote
/// from `Sell\n\n"` in Section 8.B and appends `" ` after the selected initial
/// question" - a closing quote **and one space**. The wrap trims that space
/// from the drawn row and records it, and `text-output.md` §10.6 puts the
/// cursor in the cell after it. Without it the cursor sat on the quote.
/// Measured 2026-09-18 (`shop-arms-sell-flow`, beats `browser` and `down`,
/// both reading `cursor`): the original's cursor is one cell further right.
fn arms_sell_entry_prompt(roll: u8) -> String {
    [
        "\"Which item wouldst thou like to sell?\" ",
        "\"What dost thou wish to sell?\" ",
        "\"Show me what ye got...\" ",
        "\"What dost thou have for me to buy?\" ",
    ][usize::from(roll) % 4]
        .to_string()
}

/// **Measured** 2026-09-07 (`qa/paired/shop-arms-sell-flow.tsv`): quoted, like
/// the entry pool - `"What else hath ye to sell?"` drew twice in one visit.
fn arms_sell_continuation_prompt(roll: u8) -> String {
    // §8.1: "A continuation prints `\n\n"`, its selected continuation
    // question, and `" `" - the same closing quote and space as the entry.
    [
        "\"What else can ye offer me?\" ",
        "\"What else hath ye to sell?\" ",
        "\"What else doth thou wish to sell?\" ",
        "\"What other arms wilt thou sell?\" ",
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
/// `shops.md §8.1`: the stock-call line "is followed immediately by a closing
/// double quote and one space".
///
/// That space is not decoration: it is the cell the accepted item letter
/// echoes into, and `text-output.md §10.6` leaves the cursor on it while the
/// list waits. Measured (`qa/paired/shop-arms-menus.tsv`, beat `buyquote`):
/// the original reads `see?" a`, and this engine read `interest?"a` because
/// the literals stopped at the quote.
const fn arms_stock_call_for_roll(roll: u8) -> &'static str {
    match roll & 0x03 {
        0 => "What may I show thee?\" ",
        1 => "Which wouldst thou like to see?\" ",
        2 => "What is thine interest?\" ",
        _ => "Which would ye see?\" ",
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

/// `shops.md §8.1`: the buy-list display name "is the canonical equipment name
/// from `catalogs/item-list.md`, *except* that a canonical name of thirteen
/// characters or more is replaced by that item's shorter panel label, so the
/// entry fits the shop's list column". `catalogs/item-list.md §5.1.2` owns the
/// label row. The engine printed the canonical name unconditionally, so
/// `Ring of Regeneration` wrapped and `Amulet/Turning` broke mid-word.
fn arms_buy_list_name(item_id: usize) -> &'static str {
    let canonical = equipment_name(item_id);
    if canonical.len() >= 13 {
        crate::EQUIPMENT_SHORT_LABELS
            .get(item_id)
            .copied()
            .unwrap_or(canonical)
    } else {
        canonical
    }
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
        rows.push_str(&format!("{letter}...{}\n", arms_buy_list_name(item)));
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
        // Leave's Avatar and dead-companion refusals are decided by the
        // picker before the registry write; these are invariant guards.
        // audit: not a player-facing line
        InnError::AvatarCannotLeave | InnError::DeadCompanion => String::new(),
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
            // Measured 2026-09-09 (`qa/paired/shop-arms-menus.tsv`, beat
            // `buyquote`): the description and its confirmation prompt are one
            // quoted speech - an opening `"` before the description, a blank
            // row, the prompt, and the closing `" ` that keeps the cursor on
            // that row. This is the same shape `shops.md §8.B` gives the entry
            // answer, whose Buy echo is `Buy\n\n"`, and the same closing the
            // sell offer's `Deal?"` already used.
            format!(
                "\"{quote}\n\n{}\" ",
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
        // The `§8.A` verbatim row quotes the refusal:
        // `"Thou canst not carry any more!"` (playtest E46). The post-item
        // prompt and listing follow only after the one-key wait.
        BuyRefusedCapHit { .. } => speech.attribute(ARMS_CARRY_CAP_REFUSAL, "says"),
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
        .map(|text| quote_shoppe_flourish(&text))
}

/// Wrap a rendered shared bark in its speech quotes.
///
/// The `SHOPPE.DAT` bark records carry their own closing quote and no opening
/// one, so the caller supplies the opening quote only. Measured 2026-09-10
/// against `shop-arms-menus/exit`: the original drew the `Come back...` record
/// and printed one closing quote, while this engine drew `Harrumph!` and
/// printed `"Harrumph!""`. Appending is guarded rather than dropped outright so
/// a record that lacks the trailing quote still closes.
/// The `SHOPPE.DAT` record the arms shop closes a visit on.
///
/// `shops.md` §8.A lists the closing bark as *two* rows, not one - "Shared
/// closing bark, nothing bought" and "Shared closing bark, purchase completed"
/// - each "one of four `SHOPPE.DAT` records from the current shop-kind" row
/// under a uniform `0..3` draw. The arms rows are the two halves of the shared
/// `0..7` band, which is what the engine had collapsed: it drew `0..3`
/// whichever way the visit went, so a paid-up customer got sent off with
/// `Harrumph!`.
///
/// Measured 2026-09-10. `shop-arms-buy-confirm` buys a Dagger and leaves, and
/// the original closed it on record `7`; the scenarios that buy nothing and
/// leave have only ever drawn `0..3`. The bands read that way too - `0..3` are
/// the dismissals and `4..7` the warm send-offs - which is the sentiment split
/// the two outcome rows describe.
/// Echo the accepted `Y`/`N` onto the row the sell quote's `Deal?" ` tail, or
/// the buy quote's confirmation prompt, left the cursor on.
///
/// `shops.md` §8.A's "Shared `Y`/`N` prompt primitive" row: the prompt "loops
/// until uppercase `Y` or `N`; `Y` echoes the resident `Yes` literal and `N`
/// echoes the resident `No` literal". Measured 2026-09-10
/// (`shop-arms-sell-flow/declined` and `/sold`): the original reads
/// `Deal?" No` and `Deal?" Yes` on one row. This engine printed the `No` on a
/// row of its own and omitted the `Yes` entirely, so the sell transcript ran a
/// row short of the original from the first quote onward.
fn emit_arms_confirm_echo(
    state: &mut PlayState,
    prior_state: crate::shop_runtime::ArmsShopState,
    echo: &str,
) {
    // `shops.md §8.1`: "`N` echoes `No` ... `Y` echoes `Yes` and first
    // checks whether the carried equipment counter ... is already `99`" -
    // the buy confirmation echoes on its prompt's open row the same as the
    // sell `Deal?" ` does. The buy side printed nothing (playtest K07, K26).
    if matches!(
        prior_state,
        crate::shop_runtime::ArmsShopState::SellConfirm { .. }
            | crate::shop_runtime::ArmsShopState::BuyConfirm { .. }
    ) {
        state.emit_message_line_continuing_row(echo.to_string());
    }
}

fn arms_closing_bark_record(transaction_completed: bool, roll: u8) -> Option<usize> {
    // The row bases are the ones already tabulated for every shop kind, so
    // this reads the arms row out of that table rather than repeating its
    // numbers. The purchase-completed row is the `Farewell` row, whose arms
    // base is `4` - the value the record-7 measurement implies.
    let kind = if transaction_completed {
        crate::shoppe_records::SharedShopBarkKind::Farewell
    } else {
        crate::shoppe_records::SharedShopBarkKind::InitialGreeting
    };
    crate::shoppe_records::shared_shop_bark_record(
        crate::shoppe_records::SHOP_DIALOG_ID_ARMS,
        kind,
        roll & 0x03,
    )
}

fn quote_shoppe_flourish(text: &str) -> String {
    let text = text.trim();
    if text.ends_with('"') {
        format!("\"{text}")
    } else {
        format!("\"{text}\"")
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

/// `shops.md §8.C` sage topic question, from `Of what wouldst` on: the
/// caller supplies whatever precedes it (the opening quote on entry, the
/// unknown-topic reply on a repeat).
fn sage_topic_question(speaker_is_female: bool) -> String {
    format!(
        "Of what wouldst thou hear my lore, {}?\"\n\n{SAGE_RESPOND_PROMPT}",
        tavern_honorific(speaker_is_female)
    )
}
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
const BLUE_BOAR_CHOICE_PROMPT: &str = "Thy choice?\" ";
/// The tavern's own follow-up question, measured with the zero-quantity
/// dismissal and reused when the sage hands control back.
///
/// `shops.md §8.C` publishes it exactly: after a branch returns to
/// continuation the tavern "refresh[es] the stats panel, retain[s] the message
/// window's text and cursor, and print[s] `"Anything else\nfor thee?" `" - one
/// explicit line feed inside it and a trailing space, which is what leaves the
/// cursor on the answer's row.
/// `shops.md §8.C`, "Provision offer": the resident quantity question that
/// follows the drawn quote record, carrying the offer's closing quote and the
/// trailing space the typed number echoes into.
const TAVERN_PROVISION_QUANTITY_PROMPT: &str = "\n\nHow many wouldst\nthou like?\" ";
/// The row [`TAVERN_PROVISION_QUANTITY_PROMPT`] leaves open for the typed
/// number's echo.
const TAVERN_PROVISION_QUANTITY_PROMPT_ROW: &str = "thou like?\" ";

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
/// `shops.md §8.C` "Take-it confirmation wording": the inn's Rest and Leave
/// offers share the resident tail `\nWilt thou take\nit?" `.
const INN_TAKE_IT_PROMPT: &str = "\nWilt thou take\nit?\" ";
/// `shops.md §8.C` "Multiple-guest selection".
const INN_WHO_WILL_CHECK_OUT_PROMPT: &str = "\"Who will\ncheck out?\" ";
/// `shops.md §8.C` inn records: solo Leave `191`, dead companion `192`, and
/// Pickup short funds `193`.
const INN_SOLO_LEAVE_RECORD: usize = 191;
const INN_DEAD_COMPANION_RECORD: usize = 192;
const INN_PICKUP_SHORT_FUNDS_RECORD: usize = 193;

/// `shops.md §8.C` "Inn room-resource lookup": "Records by inn row are
/// `186, 187, 188, 188, 189, 190`."
const fn inn_room_record(inn: crate::shops::Inn) -> usize {
    use crate::shops::Inn;
    match inn {
        Inn::TheWayfarerInn => 186,
        Inn::TheWarriorsStead => 187,
        Inn::TheHauntingInn | Inn::HotelBrittany => 188,
        Inn::TheSmugglersInn => 189,
        Inn::TheKingsRansomInn => 190,
    }
}

/// `shops.md §8.C` "Capacity check before Rest or Leave | First `\n\n`. If
/// full: `"I am sorry,\n`, honorific, then `, but we\nhave no room\n
/// available."\n\n`", after which the visit continues on the inn's own
/// question.
fn return_inn_capacity_refusal(honorific: &str) -> String {
    format!(
        "\n\n\"I am sorry,\n{honorific}, but we\nhave no room\navailable.\"\n\n{INN_ANYTHING_MORE_PROMPT}"
    )
}

/// `shops.md §8.C` Pickup after guest selection: "the bill and affordability
/// check proceed without a Y/N confirmation. If affordable, payment and the
/// guest's return occur before the next input wait, which is the inn
/// continuation question."
#[allow(clippy::too_many_arguments)]
fn inn_pickup_selected_guest(
    state: &mut PlayState,
    shop_state: &mut crate::shop_runtime::InnkeeperState,
    inn: crate::shops::Inn,
    scene_marker: u8,
    registry_index: usize,
    speaker_intelligence: u8,
    innkeeper_name: Option<&'static str>,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::InnkeeperState;
    let bill = state
        .inn_registry
        .get(registry_index)
        .map(|guest| {
            crate::shops::inn_pickup_bill_for_speaker(inn, guest.stay_counter, speaker_intelligence)
        })
        .unwrap_or(0);
    // "Pickup bill | Token-expanded `\n\n"That will be % gold,
    // please."\n\n"`".
    let bill_line = format!("\n\n\"That will be {bill} gold, please.\"\n\n\"");
    let attribution = innkeeper_name
        .map(|name| format!("says {name}.\n\n"))
        .unwrap_or_else(|| "\n".to_string());
    match state.pickup_inn_guest_with_bill(scene_marker, registry_index, bill) {
        Ok(outcome) => {
            state.arms_transaction_completed = true;
            *shop_state = InnkeeperState::Continuation { inn };
            // "Paid healthy pickup | `I hope thou hast found thy stay
            // enjoyable,"\n`, then token-expanded `says $.\n\n`"; "Paid
            // poisoned pickup | `Thy friend has died, by the way."\n`, then the
            // same attribution".
            let body = if outcome.returned_dead_from_poison {
                "Thy friend has died, by the way.\"\n"
            } else {
                "I hope thou hast found thy stay enjoyable,\"\n"
            };
            let message = format!("{bill_line}{body}{attribution}{INN_ANYTHING_MORE_PROMPT}");
            let surcharge = apply_active_shop_surcharge(state);
            shop_message_ignoring_surcharge(message, surcharge)
        }
        // "Pickup short funds | Record `193`, following the bill's opening
        // quote; visit ends without ordinary farewell".
        Err(InnError::InsufficientGold { .. }) => {
            *shop_state = InnkeeperState::Exited;
            format!(
                "{bill_line}{}",
                render_shared_shoppe_record(
                    game_dir,
                    INN_PICKUP_SHORT_FUNDS_RECORD,
                    bill,
                    innkeeper_name,
                    inn.display_name(),
                )
            )
        }
        Err(err) => {
            *shop_state = InnkeeperState::Continuation { inn };
            format!(
                "{}{INN_ANYTHING_MORE_PROMPT}",
                format_inn_error(err, active_speaker_is_female(state))
            )
        }
    }
}
/// **Measured**: the question the leave-companion branch asks.
const INN_WHO_WILL_STAY_PROMPT: &str = "\"Who will\nstay?\" ";
/// **Measured**: what the innkeeper says once a companion is lodged, and the
/// question the visit continues on.
const INN_THANKS_LINE: &str = "\"I thank thee.\"";
const INN_ANYTHING_MORE_PROMPT: &str = "\"Is there\nanything more\nI can do for\nthee?\" ";


/// `shops.md` §4's "Tavern count-word exception": the bill's count "is the
/// number of nondead party members: two through six produce English number
/// words; zero or one produce nothing. The surrounding spaces remain, so a
/// solo party sees `for the  of ye,` with two spaces".
fn tavern_bill_count_word(living_party: usize) -> &'static str {
    match living_party {
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        _ => "",
    }
}

/// `shops.md` §8.C's tavern result table: the ordinary round/meal and
/// secondary-drink bill is `"That will be `, the decimal price, ` gold for the
/// `, the count word, ` of ye,\n`, the honorific, `.` - and a paid purchase
/// then adds `\nEnjoy!"`. §4 stresses that this "is resident text, not a
/// resource record" and "has no `SHOPPE.DAT` record ordinal".
fn tavern_bill_line(price: u16, living_party: usize, speaker_is_female: bool) -> String {
    format!(
        "{}\nEnjoy!\"",
        tavern_bill_prefix(price, living_party, speaker_is_female)
    )
}

/// The bill up to its honorific, before the paid `\nEnjoy!"` or the
/// unaffordable CAN'T PAY envelope.
fn tavern_bill_prefix(price: u16, living_party: usize, speaker_is_female: bool) -> String {
    // The leading feeds are the blank row the original leaves between the
    // accepted letter's echo and the bill. Measured 2026-09-11
    // (`paws-sage/ale`): the original reads `thy travels?" A`, a blank row,
    // then `"That will be 1`; this engine ran the bill straight onto the row
    // below the echo. The lore branch already had this blank.
    //
    // Both of §8.C's feeds ("After the action-letter echo and `\n\n`"):
    // the echo now lands on the menu's open row, so the first feed closes
    // that row and the second is the blank. With one feed the bill sat
    // directly under `thy travels?" A` (probe of `paws-sage`, beat `ale`,
    // after `d2de5563`).
    format!(
        "\n\n\"That will be {price} gold for the {} of ye,\n{}.",
        tavern_bill_count_word(living_party),
        tavern_honorific(speaker_is_female)
    )
}

/// `shops.md §8.C` tavern result table, "Cannot afford that bill":
/// `"\n\n"CAN'T PAY?\nBeat it!"\nyells `, vendor name, `.\n`; the visit
/// ends without the shared farewell. The attribution row is dropped when the
/// vendor name is unknown rather than invented.
fn tavern_cant_pay_envelope(vendor_name: Option<&'static str>) -> String {
    let mut text = String::from("\"\n\n\"CAN'T PAY?\nBeat it!\"");
    if let Some(name) = vendor_name {
        text.push_str(&format!("\nyells {name}.\n"));
    }
    text
}

/// `shops.md §8.C`, "Blue Boar wine selection accepted": the gendered
/// fine-choice line that precedes either `Enjoy!` or the CAN'T PAY envelope.
///
/// "Uppercase selected letter, then `\n\n"Ah, a fine\nchoice, `, honorific
/// and `.`": the letter echoes onto the list's open row, so the first feed
/// closes that row and the second is the blank beneath it.
fn blue_boar_fine_choice_prefix(speaker_is_female: bool) -> String {
    format!(
        "\n\n\"Ah, a fine\nchoice, {}.",
        tavern_honorific(speaker_is_female)
    )
}

fn format_tavern_outcome(
    outcome: crate::shop_runtime::TavernOutcome,
    speaker_is_female: bool,
    living_party: usize,
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
        // `shops.md` §8.C's sage result table: "Topic prompt | Echo the
        // state's lore letter, then `\n\n"Of what wouldst\nthou hear my\nlore,
        // `, honorific, `?"\n\nYou respond:\n`". The letter now echoes onto
        // the follow-up record's open row, and the two line feeds after it
        // leave the blank row the original shows above the question.
        // Measured 2026-09-10 (`paws-sage/lore`).
        //
        // Both feeds are printed: the lore letter lands on the open row, so
        // the first closes it and the second is the blank (`d2de5563` made
        // the echo land there; with one feed the question sat directly under
        // `Rations?" C`).
        EnteredSagePrompt => format!("\n\n\"{}", sage_topic_question(speaker_is_female)),
        RoundDrinkServed { tavern, cost } => {
            // **Measured** 2026-09-08 at The Cat's Lair
            // (`qa/paired/paws-sage.tsv`): a served purchase is followed by
            // the tavern's own question, `"Anything else for thee?"`, not by
            // an `Anything else? (Y/N)` of the engine's invention.
            //
            // The bill above it used to be the engine's own sentence, on the
            // grounds that its record id was unpublished. `shops.md` §8.C now
            // publishes it as *resident* text with no record at all, so the
            // invented `<tavern> served a round for N gold.` is replaced by
            // the published assembly. Measured 2026-09-10 (`paws-sage/ale`).
            let _ = tavern;
            format!(
                "{}\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                tavern_bill_line(cost, living_party, speaker_is_female)
            )
        }
        SecondaryTavernSelected {
            tavern,
            letter,
            cost,
        } => {
            // Same published bill as the round; the accepted letter has
            // already echoed onto the list's open row.
            let _ = (tavern, letter);
            format!(
                "{}\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                tavern_bill_line(cost, living_party, speaker_is_female)
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
            // `shops.md §8.C`: "The Blue Boar list begins `"Our wine
            // list,\n`, honorific, `.\n\n`, then" the rows. The heading and
            // the quote `Thy choice?" ` closes were missing (playtest E52).
            format!(
                "\"Our wine list,\n{}.\n\n{rows}\n{BLUE_BOAR_CHOICE_PROMPT}",
                tavern_honorific(speaker_is_female)
            )
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
        // The `Yes` itself is echoed onto the question's row by the
        // dispatcher.
        DeclinedEnoughDrink => TAVERN_ANYTHING_ELSE_PROMPT.to_string(),
        // Measured: the served drink draws one gendered line, and the price
        // is not repeated back.
        // `shops.md §8.C`: the accepted choice's line, then for a paid
        // purchase "`\nEnjoy!"`, then `\n\n`" and the continuation.
        BlueBoarDrinkServed { choice, cost } => {
            let _ = (choice, cost);
            format!(
                "{}\nEnjoy!\"\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                blue_boar_fine_choice_prefix(speaker_is_female)
            )
        }
        BlueBoarRefusedShortFunds { .. } => format!(
            "{}{}",
            blue_boar_fine_choice_prefix(speaker_is_female),
            tavern_cant_pay_envelope(None)
        ),
        // Without `SHOPPE.DAT` record `89` only the continuation remains.
        BlueBoarListCancelled => format!("\n\n{TAVERN_ANYTHING_ELSE_PROMPT}"),
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
        DeclinedContinuation => "No".to_string(),
        NoSaleExit => String::new(),
        IgnoredInput => String::new(),
        // **Measured** 2026-09-07 at The Cat's Lair in Paws
        // (`qa/paired/paws-tavern.tsv`): both lines are quoted, and they sit
        // on consecutive rows with no blank between them.
        Declined => format!("\n\n\"Hrumph.\"\n{TAVERN_ANYTHING_ELSE_PROMPT}"),
        RefusedShortFunds { cost } => format!(
            "{}{}",
            tavern_bill_prefix(cost, living_party, speaker_is_female),
            tavern_cant_pay_envelope(None)
        ),
        // `shops.md §8.C` "No packs affordable, at least three food servings":
        // `\n\n"Thou hast\nneither gold nor\nneed! Out!"\nyells `, vendor name,
        // `.\n`; ends visit. The vendor-named form is the SHOPPE-backed
        // formatter's; this fallback prints the bark without the attribution
        // row rather than inventing a name.
        RefusedNoNeed => TAVERN_NO_GOLD_NO_NEED_BARK.to_string(),
        // `shops.md §8.A`: "There is no universal `Farewell.` or `As you
        // wish.` line." A shop that closes on a bark renders it through the
        // shared envelope; this arm is the silent outcome, which "omits this
        // entire sequence and its random draw".
        Exited => String::new(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

/// The accepted tavern menu letter this key answers, if any.
///
/// `shops.md` §8.C: the four letters are the tavern's own round, secondary,
/// provisions and lore keys; anything else "silently wait[s]". The echo is the
/// uppercase form, printed onto the row the list or follow-up record left open
/// after its closing quote and space.
fn tavern_accepted_menu_letter(
    state: crate::shop_runtime::TavernState,
    key_byte: u8,
) -> Option<char> {
    use crate::shop_runtime::TavernState;
    let tavern = match state {
        TavernState::Menu { tavern, .. } | TavernState::PostListWait { tavern, .. } => tavern,
        _ => return None,
    };
    let letters = tavern_menu_letters(tavern);
    let upper = key_byte.to_ascii_uppercase() as char;
    let accepted = [
        Some(letters.round),
        Some(letters.secondary),
        letters.provisions,
        Some(letters.lore),
    ];
    accepted
        .into_iter()
        .flatten()
        .any(|letter| letter.to_ascii_uppercase() == upper)
        .then_some(upper)
}



/// The row a tavern outcome's message leaves for the next key's echo, stated
/// from the outcome itself - never inferred from the text (the project rule
/// for `message_row_open_mid_line`; u5-engine#46 residual).
///
/// `shops.md §8.A`'s shared Y/N primitive and §8.C's tavern prompts end on a
/// closing quote and a space, the cursor waiting on that row for the answer
/// `emit_message_line_continuing_row` echoes there: every outcome that leaves
/// the tavern waiting on its menu, wine list, enough-drink question,
/// provision offer or `"Anything else\nfor thee?" ` continuation is `open`.
/// The vendor-attributed refusals end on `yells $.\n` (§8.C), a closed row.
/// The sage hand-off waits on the typed-topic reader, and the remaining
/// outcomes print nothing or end the visit on a record whose tail is the
/// record's own; those state nothing (`None`).
pub(crate) fn tavern_outcome_prompt_row(
    outcome: &crate::shop_runtime::TavernOutcome,
    vendor_named: bool,
) -> Option<bool> {
    use crate::shop_runtime::TavernOutcome::*;
    match outcome {
        EnteredMenu { .. }
        | RoundDrinkServed { .. }
        | SecondaryTavernSelected { .. }
        | PickBlueBoarDrink
        | ConfirmEnoughDrink
        | DeclinedEnoughDrink
        | BlueBoarDrinkServed { .. }
        | BlueBoarListCancelled
        | PickProvisionQuantity { .. }
        | ProvisionsPurchased { .. }
        | Continued { .. }
        | Declined => Some(true),
        RefusedShortFunds { .. } | BlueBoarRefusedShortFunds { .. } | RefusedNoNeed
            if vendor_named =>
        {
            Some(false)
        }
        RefusedShortFunds { .. }
        | BlueBoarRefusedShortFunds { .. }
        | RefusedNoNeed
        | EnteredSagePrompt
        | CharityProvisions { .. }
        | DeclinedContinuation
        | NoSaleExit
        | IgnoredInput
        | Exited
        | InvalidInput => None,
    }
}

fn format_tavern_outcome_with_shoppe(
    outcome: crate::shop_runtime::TavernOutcome,
    provision_quote_record_id: Option<usize>,
    no_sale_record_id: Option<usize>,
    tavern_vendor_name: Option<&'static str>,
    speaker_is_female: bool,
    living_party: usize,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::TavernOutcome::*;

    // `shops.md §8.C` tavern result table: the CAN'T PAY envelope carries
    // the vendor attribution, and the visit ends there.
    match outcome {
        RefusedShortFunds { cost } => {
            return format!(
                "{}{}",
                tavern_bill_prefix(cost, living_party, speaker_is_female),
                tavern_cant_pay_envelope(tavern_vendor_name)
            );
        }
        BlueBoarRefusedShortFunds { .. } => {
            return format!(
                "{}{}",
                blue_boar_fine_choice_prefix(speaker_is_female),
                tavern_cant_pay_envelope(tavern_vendor_name)
            );
        }
        // `shops.md §8.C`'s outcome table: a fully served quantity (or one
        // that only stopped because the 9999 food ceiling was hit) prints
        // `\n\n` and no sentence of its own. `§8.5`: "After any recognized
        // branch that does not force an immediate exit, the tavern ... prints
        // `"Anything else\nfor thee?" `", and `§8.C` names "a completed or
        // partial purchase" among them. Both arms are resident text, so they
        // no longer wait on `SHOPPE.DAT`, and the continuation question they
        // left out is printed (playtest E47).
        ProvisionsPurchased {
            completion: crate::shops::ProvisionPurchaseCompletion::Completed,
            ..
        } => return format!("\n\n{TAVERN_ANYTHING_ELSE_PROMPT}"),
        // `shops.md §8.C`, "Some requested packs afforded": gold ran out
        // after at least one pack was served. `\n\n"Thou canst\nafford only
        // `, the actual packs served, `!"\n\n` - resident text with no
        // `SHOPPE.DAT` record, found unread by `qa/tools/unwired_literals.py`
        // as `TAVERN_PARTIAL_AFFORD_PREFIX` (u5-engine#43).
        ProvisionsPurchased {
            completion: crate::shops::ProvisionPurchaseCompletion::GoldExhausted,
            purchased_quantity,
            ..
        } => {
            return format!(
                "\n\n\"{}{purchased_quantity}!\"\n\n{TAVERN_ANYTHING_ELSE_PROMPT}",
                crate::shops::TAVERN_PARTIAL_AFFORD_PREFIX
            );
        }
        // `shops.md §8.C`: "Fourth secondary drink attempt ... `\n\n"I beg
        // thy\npardon, `, honorific, `,"\nsays `, vendor name, `.\n"But
        // haven't\nye had enough\nto drink?" `". The vendor-less fallback
        // below drops the attribution row; with the name known it belongs
        // here (playtest E53).
        ConfirmEnoughDrink => {
            if let Some(name) = tavern_vendor_name {
                return format!(
                    "\n\n\"I beg thy\npardon, {},\"\nsays {name}.\n\"But haven't\nye had enough\nto drink?\" ",
                    tavern_honorific(speaker_is_female)
                );
            }
        }
        _ => {}
    }

    let renderer = crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir).ok();
    let rendered = renderer.as_ref().and_then(|renderer| match outcome {
        // `shops.md §8.C`, "Blue Boar wine list cancelled": "Space prints
        // `\n\n`, then record `89`, then returns to continuation".
        BlueBoarListCancelled => renderer
            .render_record(
                crate::shops::BLUE_BOAR_WINE_LIST_CANCEL_RECORD_ID,
                &crate::shoppe_bark::ShoppeBarkContext::default(),
            )
            .ok()
            .map(|record| {
                let feed = if record.ends_with('\n') { "" } else { "\n" };
                format!("\n\n{record}{feed}{TAVERN_ANYTHING_ELSE_PROMPT}")
            }),
        // `shops.md §8`'s entry table, the Tavern row: "`Yes\n\n"`, then the
        // state menu record, then a separately emitted closing quote and
        // space." The engine printed `Yes`, two feeds and the bare record -
        // no opening quote, no closing quote, and no trailing space for the
        // accepted letter to echo into. This is the same shape the
        // `Continued` arm below already uses for the follow-up record.
        //
        // Measured 2026-09-11 (`paws-sage/menu` and `/ale`): the original's
        // rows read `"What'll it` and `thy travels?" A`, this engine's read
        // `What'll it be...` and `thy travels?A`. The missing quotes rewrap
        // the record, which is what put the whole capture two rows out.
        EnteredMenu { tavern, .. } => renderer
            .render_record(
                tavern_menu_record_id(tavern),
                &crate::shoppe_bark::ShoppeBarkContext {
                    shop_name: tavern.display_name(),
                    ..Default::default()
                },
            )
            .ok()
            .map(|rendered| format!("\"{rendered}\" ")),
        // `shops.md §8.C`, "Provision offer": "Opening quote, one uniform
        // record from `77..83`, then `\n\nHow many wouldst\nthou like?" `;
        // typed number allows two digits." The engine rendered the quote
        // record bare - no opening quote, and no resident quantity prompt at
        // all, so the branch asked for a number without showing a question.
        // Measured 2026-09-11 (`paws-tavern/rations`).
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
                    // The leading feed is the blank row the original leaves
                    // between the accepted letter's echo and the offer, the
                    // same one the round/secondary bill takes. Measured
                    // 2026-09-11 (`paws-tavern/rations`).
                    // Two feeds since the echo lands on the menu's open
                    // row: one closes it, one is the blank (see
                    // `tavern_bill_prefix`).
                    .map(|quote| format!("\n\n\"{quote}{TAVERN_PROVISION_QUANTITY_PROMPT}"))
            })
        }
        // `shops.md §8.5`: the charitable outcome "adds `1` to the food
        // counter and renders `SHOPPE.DAT` ordinal `90`, a table-scraps
        // brush-off". The record id travels on the outcome; the engine used
        // to drop it and print an invented sentence naming the tavern.
        //
        // `shops.md §8.C`: "No packs affordable, fewer than three food
        // servings | `\n\n`, then record `90`", and the record's `$` is the
        // vendor (`§4.1`). Both were missing - the brush-off ran onto the
        // quantity row and closed on `orders .` (playtest E50).
        CharityProvisions {
            tavern, record_id, ..
        } => renderer
            .render_record(
                record_id,
                &crate::shoppe_bark::ShoppeBarkContext {
                    shop_name: tavern.display_name(),
                    vendor_name: tavern_vendor_name.unwrap_or(""),
                    ..Default::default()
                },
            )
            .ok()
            .map(|record| format!("\n\n{record}")),
        // `shops.md §8.C`, "No packs affordable, at least three food
        // servings": `\n\n"Thou hast\nneither gold nor\nneed! Out!"\nyells `,
        // vendor name, `.\n`; ends visit. The no-SHOPPE fallback prints the
        // bare bark without this attribution row (u5-engine#43).
        RefusedNoNeed => tavern_vendor_name
            .map(|name| format!("{TAVERN_NO_GOLD_NO_NEED_BARK}\nyells {name}.\n")),
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
            // `shops.md` §8.C, "Tavern and sage continuation": "Y prints
            // `Yes\n\n\"`, the applicable follow-up record, a closing quote
            // and a space." The engine printed `Yes`, one line feed and the
            // bare record - no opening quote, no closing quote, and no
            // trailing space for the next accepted letter to echo into.
            // Measured 2026-09-10 (`paws-sage/lore`): the original's row
            // reads `Rations?" C`, this engine's read `Rations`.
            // The `Yes` is echoed onto the question's own open row by the
            // caller, the same way the accepted menu letter is; §8.C's
            // `Yes\n\n` is that echo plus the blank row, not a fresh row of
            // its own. Measured 2026-09-11 (`paws-sage/again`): the
            // original reads `for thee?" Yes`, this engine put `Yes` alone
            // on the next row and ran one row ahead from there.
            .map(|rendered| format!("\"{rendered}\" ")),
        // Measured: the closing bark is attributed to the tavern's own vendor
        // - `"What's wrong with ye? Can't hold thy liquor?"` over
        // `says Dr. Cat.` - using the §8.0 vendor-name row the engine already
        // keeps for the arms shops.
        //
        // `No` is echoed onto the continuation question's own open row by
        // the caller, the same way `Yes` is - it is not part of this
        // record's own text (`cleak/u5-engine#41`).
        //
        // `shops.md §8.C`: N "runs the ordinary quoted farewell", and §8's
        // shared closing envelope is "`\n\n\"`, the selected record, a line
        // feed only when the resulting local cursor column is nonzero, then
        // token-expanded `says $.\n`. The record owns its closing quote."
        // The engine printed the record bare under the `No` row. Measured
        // 2026-10-16 (`paws-tavern`, beat `no`): the original reads
        // `for thee?" No`, a blank row, then `"What's wrong / with ye? ...`.
        DeclinedContinuation | NoSaleExit => no_sale_record_id.and_then(|record_id| {
            renderer
                .render_record(record_id, &crate::shoppe_bark::ShoppeBarkContext::default())
                .ok()
                .map(|rendered| match tavern_vendor_name {
                    Some(name) => format!("\n\n\"{rendered}\nsays {name}."),
                    None => format!("\n\n\"{rendered}"),
                })
        }),
        _ => None,
    });
    rendered
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| format_tavern_outcome(outcome, speaker_is_female, living_party))
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
        Declined => String::new(),
        RefusedShortFunds { .. } => TAVERN_AFFORDABILITY_REFUSAL_BARK.to_string(),
        InputTooLong { limit, .. } => format!("Ask in {limit} characters or fewer."),
        NoTopicMatch => "That, I cannot help thee with.".to_string(),
        // The tavern continuation question is appended by the caller.
        EmptyTopic => String::new(),
        // `shops.md §8.A`: "There is no universal `Farewell.` or `As you
        // wish.` line." A shop that closes on a bark renders it through the
        // shared envelope; this arm is the silent outcome, which "omits this
        // entire sequence and its random draw".
        Exited => String::new(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

fn format_sage_outcome_with_shoppe(
    outcome: crate::shop_runtime::SageOutcome,
    vendor_name: Option<&'static str>,
    speaker_is_female: bool,
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
        // `shops.md §8.C` sage result table: "Short funds | `"Sorry, `,
        // honorific, record `91`; ends visit", the record's `$` naming the
        // tavern vendor (`§4.1`). The engine printed the record alone with an
        // empty vendor - `,` / ... / `says .` (playtest E50).
        RefusedShortFunds {
            required,
            available,
        } => renderer
            .as_ref()
            .and_then(|renderer| {
                renderer
                    .render_sage_short_funds_record(vendor_name.unwrap_or(""), None)
                    .ok()
            })
            .map(|record| format!("\"Sorry, {}{record}", tavern_honorific(speaker_is_female)))
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

/// `shops.md §8.C` reagent records: offers `139..146` in the canonical
/// eight-reagent order, and short funds `147`.
const REAGENT_OFFER_RECORD_FIRST: usize = 139;
const REAGENT_SHORT_FUNDS_RECORD: usize = 147;

/// The stock rows and the closing question: "Each stock row is its
/// uppercase compact letter, `...`, the short reagent name, and `\n`. After
/// the list print `\nThy interest?" `."
fn reagent_stock_menu(herbalist: crate::shops::Herbalist) -> String {
    let mut rows = String::new();
    for entry in crate::shops::herbalist_menu_entries(herbalist) {
        rows.push_str(&format!(
            "{}...{}\n",
            entry.letter,
            entry.reagent.display_name()
        ));
    }
    format!("{rows}\n{REAGENT_MENU_QUESTION}")
}

fn format_reagent_outcome(
    outcome: crate::shop_runtime::ReagentShopOutcome,
    vendor_name: Option<&'static str>,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::ReagentShopOutcome::*;
    let render = |record_id: usize, gold: u16, quantity: u16| {
        crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
            .ok()
            .and_then(|renderer| {
                renderer
                    .render_record(
                        record_id,
                        &crate::shoppe_bark::ShoppeBarkContext {
                            gold,
                            quantity,
                            vendor_name: vendor_name.unwrap_or_default(),
                            ..Default::default()
                        },
                    )
                    .ok()
            })
            .unwrap_or_default()
    };
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
        EnteredMenu { herbalist } => {
            format!("{REAGENT_MENU_HEADING}\n\n{}", reagent_stock_menu(herbalist))
        }
        // `shops.md §8.C` "Offer": `\n\n"`, then record `139..146` in the
        // canonical eight-reagent order, then ` Is this thy need?" `.
        Offered {
            herbalist,
            reagent,
            price,
        } => {
            let ordinal = crate::shops::REAGENT_VENDOR_ORDER
                .iter()
                .position(|candidate| *candidate == reagent)
                .unwrap_or_default();
            // `shops.md §4.1`: `^` is "a count" filled "mid-loop by the shop
            // arm as the player picks items"; for a reagent offer that is the
            // §6 fixed bundle quantity the quoted price buys. Leaving it at
            // zero rendered `costs 22 gp for 0 roots`.
            let quantity = crate::shops::herbalist_reagent_bundle_quantity(herbalist, reagent)
                .map(u16::from)
                .unwrap_or_default();
            reagent_offer_text(&render(REAGENT_OFFER_RECORD_FIRST + ordinal, price, quantity))
        }
        // "Already carrying 99 | `\n\n"Thou canst not carry any more!"\n\n`;
        // consume one command key, then redraw stock".
        // The refusal follows `Thy interest?" `'s open row, so the first
        // feed of each pair closes a row and the second is the blank; one
        // feed a side ran it straight under the prompt and the redrawn
        // stock straight under it (playtest H46).
        RefusedAtCap { .. } => "\n\n\"Thou canst not carry any more!\"\n\n".to_string(),
        RedrawStock { herbalist } => reagent_stock_menu(herbalist),
        // "Paid | Token-expanded `\n"I thank thee!"\nsays $.\n`, then
        // `"Anything else?\n\n`, then stock".
        Bought { herbalist, .. } => {
            let attribution = vendor_name
                .map(|name| format!("\nsays {name}."))
                .unwrap_or_default();
            // `Yes\n` then `\n"I thank thee!"`: the echo row closes and one
            // blank follows (playtest F46).
            format!(
                "\n\n\"I thank thee!\"{attribution}\n\"Anything else?\n\n{}",
                reagent_stock_menu(herbalist)
            )
        }
        // "Short funds | Record `147`; visit ends without ordinary farewell".
        // The engine's `Yes` echo leaves its `\n` to the next text (the
        // paid arm's leading `\n\n` above), so that feed is supplied here
        // and the record's own leading feed still spends its blank row
        // (playtest N02).
        RefusedShortFunds { cost } => format!("\n{}", render(REAGENT_SHORT_FUNDS_RECORD, cost, 0)),
        // "Decline | `No\n\n"What else?\n\n`, then stock".
        Declined { herbalist } => format!(
            "\n\n\"What else?\n\n{}",
            reagent_stock_menu(herbalist)
        ),
        // Closing barks are rendered by the caller through the shared
        // envelope; these are the silent arms.
        DeclinedEntry | LeftMenu | Exited => String::new(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

/// `shops.md §8.C` reagent "Offer": `\n\n"`, then the offer record, then
/// ` Is this thy need?" `. "If the resulting local column exceeds 12, append
/// `\n:`." The question's row is open, so the two leading feeds close it and
/// leave one blank row (playtest F46).
///
/// The column is the window-local cursor column after the question's
/// trailing space: the cells the wrapped final row holds through the closing
/// quote, plus that space ([`crate::wrapped_final_row_columns`] trims a
/// trailing space, so it is added back here).
pub(crate) fn reagent_offer_text(record: &str) -> String {
    let question = format!("\n\n\"{record}{REAGENT_OFFER_QUESTION}");
    let column = crate::wrapped_final_row_columns(&question) + 1;
    if column > REAGENT_OFFER_COLON_COLUMN {
        format!("{question} {REAGENT_OFFER_COLON_CONTINUATION}")
    } else {
        format!("{question} ")
    }
}

/// The offer question through its closing quote; [`reagent_offer_text`]
/// adds the space after it.
const REAGENT_OFFER_QUESTION: &str = " Is this thy need?\"";
/// `shops.md §8.C`: the column the offer's cursor must exceed for the colon.
const REAGENT_OFFER_COLON_COLUMN: usize = 12;
/// `shops.md §8.C`: what a past-12 offer appends.
const REAGENT_OFFER_COLON_CONTINUATION: &str = "\n:";

const REAGENT_MENU_HEADING: &str = "\"Fine! We sell:";
/// §8.C: `\nThy interest?" ` - the closing quote and one space. Playtest
/// F46: without the space the cursor abutted the quote.
const REAGENT_MENU_QUESTION: &str = "Thy interest?\" ";

/// `shops.md §8.6`, the horse trader's one placement refusal.
const HORSE_TRADER_CLOSED_REFUSAL: &str = "The stables are closed.\n";
/// `shops.md §8.B` "Horse and ship resource lookup": the stable / horse
/// offer body is record `104`, followed by the resident `\n\nDeal?" `.
const HORSE_TRADER_OFFER_RECORD: usize = 104;
const HORSE_TRADER_DEAL_PROMPT: &str = "\n\nDeal?\" ";

/// `shops.md §8.10`: on entry the flow takes the first free active-object
/// slot and probes the four cardinal placement cells; "If all four probes
/// fail, or if no free object slot was found, the shop prints the refusal
/// line and exits without a purchase". True when the stables are closed.
pub(crate) fn horse_trader_stables_closed(state: &PlayState) -> bool {
    let has_free_slot = !state.active_objects.is_empty()
        && (state.active_objects.len() <= crate::ACTIVE_OBJECT_ACQUISITION_LAST_SLOT
            || state
                .active_objects
                .iter()
                .skip(1)
                .take(crate::ACTIVE_OBJECT_ACQUISITION_LAST_SLOT)
                .any(|object| object.is_empty()));
    !has_free_slot || horse_sale_position(state).is_none()
}

/// The closed refusal as the entry prints it, before any greeting.
pub(crate) const HORSE_TRADER_STABLES_CLOSED_LINE: &str = HORSE_TRADER_CLOSED_REFUSAL;

fn format_horse_trader_outcome(
    outcome: crate::shop_runtime::HorseTraderOutcome,
    vendor_name: Option<&'static str>,
) -> String {
    use crate::shop_runtime::HorseTraderOutcome::*;
    match outcome {
        // Fallbacks for a run without `SHOPPE.DAT` only; the asset supplies
        // record `104` and the closing barks.
        QuotedPrice { price } => format!("\"A fine steed costs {price} gold.{HORSE_TRADER_DEAL_PROMPT}"),
        Purchased { .. } | Declined => String::new(),
        // `shops.md §8.C` "Horse traders": "Short funds then print
        // `\n\n"Thou couldst not afford to ` followed by token-expanded
        // `feed it!"\nyells $.\n`, and suppress the ordinary farewell."
        RefusedShortFunds { .. } => format!(
            "\n\n\"Thou couldst not afford to feed it!\"{}",
            vendor_name
                .map(|name| format!("\nyells {name}.\n"))
                .unwrap_or_default()
        ),
        // `shops.md §8.C`: "No free active-object slot and no suitable
        // adjacent placement cell both print `The stables are closed.\n`
        // before any greeting, then exit." The entry probe normally catches
        // it; this covers a cell lost between entry and confirmation.
        RefusedNoMarker { .. } => HORSE_TRADER_CLOSED_REFUSAL.to_string(),
        Exited => String::new(),
        // `shops.md §8.6`: the purchase loop is "Y/N driven" and other keys
        // wait; no refusal prints.
        InvalidInput => String::new(),
    }
}

/// The confirmation prompt a quoted hull ends on, quoted like the arms
/// browser's `Deal?"`.
const SHIPWRIGHT_CONFIRM_PROMPT: &str = "Wilt thou take it?\"";

/// `shops.md §8.C`: the shipwright's ordinary offer is the vessel body record
/// followed by the shared confirmation record `126`, which "owns the
/// confirmation's text and spacing".
/// `shops.md §8.C` "Purchase continuation": after record `124`, the
/// honorific - always `sir` in the shipped build - then `?" `.
const SHIPWRIGHT_CONTINUATION_TAIL: &str = "sir?\" ";

/// Render one deterministic shipwright record (`shops.md §8.7`), or `None`
/// when `SHOPPE.DAT` is unavailable or the record is empty.
fn render_shipwright_record(
    game_dir: &Path,
    record_id: usize,
    vendor_name: Option<&'static str>,
    state: &PlayState,
) -> Option<String> {
    crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
        .ok()?
        .render_record(
            record_id,
            &crate::shoppe_bark::ShoppeBarkContext {
                vendor_name: vendor_name.unwrap_or_default(),
                hour: state.clock.hour,
                ..Default::default()
            },
        )
        .ok()
        .filter(|text| !text.trim().is_empty())
}

fn render_shipwright_offer(
    game_dir: &Path,
    body_record: usize,
    vendor_name: Option<&'static str>,
    price: u16,
    state: &PlayState,
) -> Option<String> {
    let renderer = crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir).ok()?;
    let context = crate::shoppe_bark::ShoppeBarkContext {
        vendor_name: vendor_name.unwrap_or_default(),
        hour: state.clock.hour,
        gold: price,
        ..Default::default()
    };
    let body = renderer.render_record(body_record, &context).ok()?;
    let confirm = renderer
        .render_record(crate::shops::SHOPPE_RECORD_SHIPWRIGHT_CONFIRM, &context)
        .ok()?;
    (!body.trim().is_empty()).then(|| format!("{body}{confirm}"))
}

fn format_ship_broker_outcome(
    outcome: crate::shop_runtime::ShipBrokerOutcome,
    menu_record: Option<String>,
    offer_record: Option<String>,
) -> String {
    use crate::shop_runtime::ShipBrokerOutcome::*;
    match outcome {
        // `shops.md §8.B`, the shipwright's row: "`Yes`, then record `119`
        // with no added resident menu question; the record carries its own
        // leading spacing and prompt." The asset supplies the wording; a run
        // without `SHOPPE.DAT` shows nothing rather than a transcription of
        // the record (playtest Q16).
        EnteredMenu => menu_record.unwrap_or_default(),
        QuotedPurchase { quote } => offer_record.unwrap_or_else(|| {
            // Fallback for a run without the asset: the engine's own
            // sentence, kept only so a headless harness still has text.
            let item = match quote.kind {
                crate::shops::ShipwrightPurchaseKind::Frigate => "frigate",
                crate::shops::ShipwrightPurchaseKind::Skiff => "skiff",
            };
            format!(
                "A {item} costs {} gold.\n\n{SHIPWRIGHT_CONFIRM_PROMPT}",
                quote.price
            )
        }),
        // `§8.7`: the record owns the dock-space discussion; "there is no
        // additional resident no-dockspace refusal".
        QuotedDockSpace => offer_record.unwrap_or_default(),
        // Rendered by the caller from records `120..124` and `122`.
        PurchaseApplied { .. } | RefusedShortFunds { .. } | Declined => String::new(),
        // `shops.md §8.A`: "There is no universal `Farewell.` or `As you
        // wish.` line." A shop that closes on a bark renders it through the
        // shared envelope; this arm is the silent outcome, which "omits this
        // entire sequence and its random draw".
        Exited => String::new(),
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
/// §8.C: the menu ends `Thy concern?" ` and the offer `\n\nInterested?" ` -
/// each with the space the next echo lands after. Playtest F46 read
/// `Thy concern?"a` and `Interested?"No`.
const GUILD_MENU_QUESTION: &str = "Thy concern?\" ";
/// The prompt a quoted commodity ends on.
const GUILD_QUOTE_PROMPT: &str = "Interested?\" ";
/// The heading the list takes when a declined quote redraws it.
const GUILD_MENU_AGAIN_HEADING: &str = "\"What else, then?";
const GUILD_MENU_ROW_WIDTH: usize = 14;

pub(crate) fn guild_menu_rows() -> String {
    let mut rows = String::new();
    for (letter, commodity) in [
        ('a', crate::shops::GuildCommodity::Keys),
        ('b', crate::shops::GuildCommodity::Gems),
        ('c', crate::shops::GuildCommodity::Torches),
    ] {
        // Capitalised, as measured and as §8.C prints them (playtest E56).
        let name = commodity.menu_label();
        let dots = GUILD_MENU_ROW_WIDTH.saturating_sub(1 + name.len());
        rows.push_str(&format!("{letter}{}{name}\n", ".".repeat(dots)));
    }
    rows
}

/// `shops.md §8.C` guild records: offers `160`/`161`/`162` for Keys, Gems
/// and Torches, short funds `163`, and the `D` description `164`.
const GUILD_OFFER_RECORD_KEYS: usize = 160;
const GUILD_SHORT_FUNDS_RECORD: usize = 163;
const GUILD_DESCRIBE_RECORD: usize = 164;

fn guild_offer_record(commodity: crate::shops::GuildCommodity) -> usize {
    use crate::shops::GuildCommodity;
    GUILD_OFFER_RECORD_KEYS
        + match commodity {
            GuildCommodity::Keys => 0,
            GuildCommodity::Gems => 1,
            GuildCommodity::Torches => 2,
        }
}

/// `shops.md §8.C`: "D renders record `164`, then redraws the stock menu."
/// Nothing else: there is no echo of the key and no spacing of the
/// engine's own between the two. The record carries both - it opens with
/// the text that lands after `Thy concern?" ` on the prompt's open row and
/// ends with the feeds that leave one blank row above the menu - so adding
/// a `\n\n` here left three blank rows (rc3 residual, CASTLE:5 NPC 0x86).
pub(crate) fn guild_described_text(record: &str) -> String {
    format!("{record}{}", guild_stock_menu())
}

fn guild_stock_menu() -> String {
    format!("{}\n{GUILD_MENU_QUESTION}", guild_menu_rows())
}

fn format_guild_outcome(
    outcome: crate::shop_runtime::GuildShopOutcome,
    vendor_name: Option<&'static str>,
    speaker_is_female: bool,
    game_dir: &Path,
) -> String {
    use crate::shop_runtime::GuildShopOutcome::*;
    let render = |record_id: usize, shop: Option<crate::shops::GuildShop>, gold: u16| {
        crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
            .ok()
            .and_then(|renderer| {
                renderer
                    .render_record(
                        record_id,
                        &crate::shoppe_bark::ShoppeBarkContext {
                            gold,
                            vendor_name: vendor_name.unwrap_or_default(),
                            shop_name: shop.map(|shop| shop.display_name()).unwrap_or_default(),
                            ..Default::default()
                        },
                    )
                    .ok()
            })
            .unwrap_or_default()
    };
    match outcome {
        // `shops.md §8.B`: `Yes\n\n"We sell:\n\n`, then the stock list.
        EnteredMenu { .. } => format!("{GUILD_MENU_HEADING}\n\n{}", guild_stock_menu()),
        // `shops.md §8.C` "Offer": `\n\n"`, then record `160`/`161`/`162`,
        // then `\n\nInterested?" `.
        Offered {
            shop,
            commodity,
            price,
        } => format!(
            // After the letter echo on the open `Thy concern?" ` row: one
            // feed closes it, one is the blank (playtest F46).
            "\n\n\"{}\n\n{GUILD_QUOTE_PROMPT}",
            render(guild_offer_record(commodity), Some(shop), price)
        ),
        // "D renders record `164`, then redraws the stock menu."
        Described { shop } => {
            guild_described_text(&render(GUILD_DESCRIBE_RECORD, Some(shop), 0))
        }
        // "Paid | Token-expanded `\n"Sold!"\nsays $.\n\n"What else, \n`, then
        // `m'lady` for a female speaking member or `m'lord` otherwise, then
        // `?\n\n`, then stock menu".
        Bought { .. } => {
            let attribution = vendor_name
                .map(|name| format!("\nsays {name}."))
                .unwrap_or_default();
            format!(
                "\n\n\"Sold!\"{attribution}\n\n\"What else, \n{}?\n\n{}",
                if speaker_is_female { "m'lady" } else { "m'lord" },
                guild_stock_menu()
            )
        }
        // "Short funds | Record `163`; visit ends without ordinary farewell".
        // As the reagent arm: the `Yes\n` echo's feed comes first, so the
        // record's own leading feed spends the blank row (playtest N02).
        RefusedShortFunds { cost } => format!("\n{}", render(GUILD_SHORT_FUNDS_RECORD, None, cost)),
        // "Decline | `No\n\n"What else, then?\n\n`, then stock menu".
        Declined => format!("\n\n{GUILD_MENU_AGAIN_HEADING}\n\n{}", guild_stock_menu()),
        // Closing barks are rendered by the caller through the shared
        // envelope; these are the silent arms.
        DeclinedEntry | LeftMenu | Exited => String::new(),
        // `shops.md §8.B`/`§8.C`: an unrecognised key waits; there is no
        // I-do-not-understand line anywhere in the shop family.
        InvalidInput => String::new(),
    }
}

/// `shops.md §8.A` shared closing-bark table, tavern row: the nothing-bought
/// records `61..64`, or `65..68` once a purchase completed in this visit.
pub(crate) const fn tavern_closing_bark_records(purchase_completed: bool) -> (usize, usize) {
    if purchase_completed {
        (
            crate::shops::TAVERN_PURCHASE_COMPLETED_RECORD_FIRST,
            crate::shops::TAVERN_PURCHASE_COMPLETED_RECORD_LAST,
        )
    } else {
        (TAVERN_NO_SALE_RECORD_FIRST, TAVERN_NO_SALE_RECORD_LAST)
    }
}

/// The inn's entry `No`/Space exit: the ordinary farewell, whose envelope
/// already carries its leading `\n\n` - this used to add a second pair.
fn inn_entry_closing_bark(
    state: &mut PlayState,
    innkeeper_name: Option<&'static str>,
    game_dir: &Path,
) -> String {
    shop_closing_bark(state, SHOP_DIALOG_ID_INN, innkeeper_name, game_dir)
}

/// [`shop_closing_bark`] for an exit key that echoes nothing: the bark
/// follows the service prompt's own `" ` on a row that prompt left open, so
/// the row is stated open here rather than read from whatever the last
/// emission left (the envelope's first feed closes it, the second is the
/// blank).
fn shop_closing_bark_after_open_prompt(
    state: &mut PlayState,
    dialog_id: u8,
    vendor_name: Option<&'static str>,
    game_dir: &Path,
) -> String {
    state.flush_message_slot();
    state.open_message_row();
    shop_closing_bark(state, dialog_id, vendor_name, game_dir)
}

/// `shops.md §8` intro and `§8.C`'s "Shared closing envelope": an ordinary
/// visit ending draws one record from the shop kind's nothing-bought row, or
/// from its purchase-completed row when a purchase completed in this visit,
/// and wraps it as `\n\n"`, the record, then `says $.\n`. Silent outcomes
/// do not call this and so take no draw.
///
/// "Both ordinary farewell pools ... use the same sequence: `\n\n\"`, the
/// selected record ...", for every shop kind. Only the shipwright's and the
/// arms shop's callers used to supply the `\n\n`, so the guild, herbalist,
/// healer, inn and stable printed their farewell directly under the row it
/// followed; the envelope now lives here, once. The arms shop renders its own
/// bands through the same envelope in its own outcome formatter.
fn shop_closing_bark(
    state: &mut PlayState,
    dialog_id: u8,
    vendor_name: Option<&'static str>,
    game_dir: &Path,
) -> String {
    let kind = if state.arms_transaction_completed {
        crate::shoppe_records::SharedShopBarkKind::Farewell
    } else {
        crate::shoppe_records::SharedShopBarkKind::InitialGreeting
    };
    let roll = state.random_range_u8(0, 3);
    crate::shoppe_records::shared_shop_bark_record(dialog_id, kind, roll)
        .and_then(|record_id| render_shared_shoppe_flourish(game_dir, record_id))
        .map(|line| match vendor_name {
            Some(name) => format!("\n\n{line}\nsays {name}."),
            None => format!("\n\n{line}"),
        })
        .unwrap_or_default()
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
    // `conversation.md §7.6`: "a line containing only a space is *not*
    // empty", and `input.md §8` accepts space as printable input, so only
    // the Enter that ends the line is dropped; the typed bytes are kept.
    //
    // `input.md §8`: in the free-text reader (keyword and ASK-WHO prompts,
    // `conversation.md`) "Escape clears the whole current line but does
    // **not** submit or terminate it". The shell clears its own line
    // buffer; a bare Escape reaching here is never a typed keyword.
    if key == '\u{1b}'
        && suffix.is_empty()
        && state
            .active_conversation
            .as_ref()
            .is_some_and(|session| !session.prompt_message().is_empty())
    {
        return PlayInputDisposition::Continue;
    }
    let mut line = String::new();
    if !matches!(key, '\r' | '\n') {
        line.push(key);
    }
    line.push_str(suffix);
    let line = line.trim_end_matches(['\r', '\n']).to_string();
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
                // only owes the prompt that follows the reply - and when the
                // reply is held behind a pause, only once the hold is over.
                state.set_message_after_text_hold(prompt);
            }
        }
    }
    PlayInputDisposition::Continue
}

/// Replay one key that was typed while a cinematic hold owned the loop.
///
/// Called by the shell once the hold is over. One per call, so a buffered
/// burst is dispatched at the same one-command-per-frame cadence a player's
/// own typing would be.
///
/// The hold itself discards nothing (`karma.md §12`), but the read that takes
/// the key is an ordinary one: `input.md §6`, with type-ahead off "the queue
/// is emptied on every read" - "drop all pending keystrokes after consuming
/// one". Replaying the whole burst let four taps during the third shrine
/// mantra answer both altar pages and then run as commands (playtest H13).
pub fn drain_buffered_hold_key(
    state: &mut PlayState,
    game_dir: &Path,
) -> io::Result<Option<PlayInputDisposition>> {
    if !state.buffered_hold_key_ready() {
        return Ok(None);
    }
    let Some((key, suffix)) = state.keys_buffered_during_hold.pop_front() else {
        return Ok(None);
    };
    let honours_queue = buffered_key_read_honours_queue(state);
    let disposition = handle_play_key_input(state, key, &suffix, game_dir)?;
    if !honours_queue {
        state.keys_buffered_during_hold.clear();
    }
    Ok(Some(disposition))
}

/// `input.md §6`: whether the read about to take a buffered key leaves the
/// rest queued - the player's Buffer On, or a free-text line reader, which
/// "saves the setting, forces type-ahead on for the duration of a typed line,
/// and restores it afterwards". The shrine's virtue and mantra answers and
/// the spell and mixer name entries are typed lines too.
fn buffered_key_read_honours_queue(state: &PlayState) -> bool {
    state.typeahead_buffer_enabled
        || free_text_line_reader_open(state)
        || state.active_shrine_restoration.is_some()
        || state.active_cast.is_some()
        || state.active_cast_followup.is_some()
        || state.active_mix.is_some()
        || state.active_shrine.as_ref().is_some_and(|session| {
            matches!(
                session.phase,
                crate::ShrinePhase::Virtue | crate::ShrinePhase::Mantra
            )
        })
}

fn handle_active_blackthorn_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    // `input.md §8`: the audience answer is read by the shared free-text
    // reader, where Escape "clears the whole current line but does **not**
    // submit or terminate it". A bare Escape at the answer prompt is never
    // an answer, and one inside a typed line discards what came before it
    // (playtest H39). The acknowledgement holds still take any key.
    let answering = state
        .active_blackthorn
        .as_ref()
        .is_some_and(|challenge| challenge.current_prompt().is_some());
    if answering && key == '\u{1b}' && suffix.is_empty() {
        return Ok(PlayInputDisposition::Continue);
    }
    let mut line = String::new();
    if !matches!(key, '\r' | '\n' | ' ') {
        line.push(key);
    }
    line.push_str(suffix);
    if answering && let Some(cleared) = line.rfind('\u{1b}') {
        line = line[cleared + 1..].to_string();
    }
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
    // `endgame.md §7` steps 4-6 - slot 0's walk up and back, the box, then
    // record `3` - read no key. A key that arrives while they run stays in
    // the type-ahead buffer until they finish, and is then read by whatever
    // follows (`drain_buffered_hold_key`). It used to finish the hand-over at
    // once and land on record `3`.
    if state.endgame_victory_staging_pending() {
        state
            .keys_buffered_during_hold
            .push_back((key, suffix.to_string()));
        return Ok(PlayInputDisposition::Continue);
    }
    // `endgame.md §4`/`§5`: the setup pass and its walk-in read no key, so
    // a key that arrives while they still owe frames waits for the first
    // read that follows - the greeting's any-key wait (`§5.1`). Settle the
    // entry presentation first, which prints the greeting and opens that
    // wait, and let this key satisfy it. It used to reach the yes/no read
    // directly: record 1 never printed and the answer echo was glued to the
    // greeting row (playtest K09). The holds those frames stage print
    // nothing and precede the greeting, so they are spent here too, with the
    // greeting kept in the slot it was composed into: left queued, they were
    // emitted after the question and took the slot that the answer echo
    // reads its `You reply: ` row from.
    if state
        .endgame
        .as_ref()
        .is_some_and(|endgame| !endgame.is_terminal())
        && state.finish_endgame_entry_presentation() > 0
    {
        let greeting = std::mem::take(&mut state.message);
        let greeting_flushed = std::mem::take(&mut state.message_flushed);
        state.flush_staged_narration();
        state.message = greeting;
        state.message_flushed = greeting_flushed;
    }
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
        // `combat.md §8.4` (u5-spec#299 Q19): "Cast's own copy of the test
        // prints `Cast...` and its newline, then `Can't!` on the next row",
        // and the refusal is `Can't!\n` - its feed closes the row, so the
        // full re-prompt's leading newline leaves a blank row before the
        // banner. This printed `Cast...Can't!` on one row with no blank.
        state.emit_combat_command_echo_line(&format!(
            "{COMBAT_CAST_COMMAND_LABEL}\n{COMBAT_PARTY_SIDE_REFUSAL}\n"
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
pub(crate) fn finish_combat_cast_actor_action(
    state: &mut PlayState,
    actor_slot: usize,
    had_foe: bool,
) {
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
        //
        // `§14`: "The stored string has one leading and one trailing
        // newline", and the kill line it follows is a completed result
        // (`magic.md §7`: the results "carry trailing feeds"). Appending it
        // to the slot let its leading feed close the result's row instead,
        // so `Merchant killed!` ran straight into `VICTORY!` (playtest F38).
        // Close the result first, then print it as the weapon path does.
        close_combat_cast_result_row(state);
        if state.announce_combat_victory_if_needed() {
            state.emit_combat_print(crate::combat_frame::COMBAT_VICTORY_LINE);
        }
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    } else if state.combat_active {
        close_combat_cast_result_row(state);
        advance_combat_round_after_actor_and_append_message(state, actor_slot);
    }
}

/// `magic.md §7`: the shared dispatcher's result literals (`Success!`,
/// `Not here!`, `None mixed!`, `Failed!`, `Absorbed!` ...) "carry trailing
/// feeds", and "Combat's subsequent actor banner has its own leading feed,
/// which leaves a blank row after a completed result" (`combat.md §8.4`
/// says the same for `Not here!` / `None mixed!`).
///
/// Those results reach the transcript through the message slot, not as arena
/// prints, so `combat_transcript_row_open` still described the `Cast...`
/// echo row the command opened and the banner's leading feed was spent
/// closing it. Flush the result here and record, for this caller, that its
/// row is closed. A cast whose output was all arena prints leaves the slot
/// already flushed, and then the flags those prints recorded stand
/// (playtest E40).
fn close_combat_cast_result_row(state: &mut PlayState) {
    if state.flush_message_slot() {
        state.combat_transcript_row_open = false;
        state.message_row_open_mid_line = false;
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
        // Two leading feeds, not one.
        //
        // `combat.md §11.1`'s census gives the party melee swing "**a
        // newline, unconditionally, before the roll**", and the miss and hit
        // rows both say the result line follows "the newline already printed
        // before the roll". That accounts for one. The other closes the
        // `Aim! ` row, which the confirmed cursor leaves open.
        //
        // The comment that stood here said the opposite - that the pre-roll
        // newline *was* the one closing the `Aim! ` row, "so it closes that
        // row instead of blanking one". Measured 2026-09-13
        // (`qa/paired/combat-damage-narration.tsv`, every swing of fourteen):
        // the original reads
        //
        // ```text
        //  Attack-Aim!
        //
        // Merchant barely
        // wounded!
        // ```
        //
        // with a blank row between them, on the miss rows and on all four
        // wound grades alike, and this engine ran the result straight under
        // the prompt every time.
        //
        // The `Nothing!` routes of `§8.2` keep their single feed and are
        // right to: no roll happens on them, so `§11.1`'s pre-roll newline
        // is never emitted, and the original shows no blank there. That
        // asymmetry is what makes this two rather than a wrapping artifact.
        state.emit_combat_print(&format!("\n\n{line}\n"));
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
        // `apply_combat_round_loop_exit` writes `COMBAT_DEFEAT_LINE` into
        // `message` for the ordinary defeat exit, and that reaches the
        // transcript. On the edge-exit path the line is printed explicitly
        // instead, so that it lands on the row below the edge echo - and with
        // both producers live the original's single `BATTLE IS LOST!` came
        // out twice, with a blank between (`cleak/u5-engine#35`).
        //
        // Clearing `message` after the call does not help: by then it has
        // already been flushed. The assignment has to not happen, so the edge
        // path takes the variant that leaves the line to its caller.
        //
        // Established with the probe rather than by reading: tagging the
        // explicit print showed the first copy comes from the assignment and
        // the second from the print, in that order.
        if edge_defeat {
            state.apply_combat_round_loop_exit_without_result_line(exit);
            // The edge-exit echo is already on the transcript, so the defeat
            // line is its own print on the row below it.
            state.emit_combat_print("\nBATTLE IS LOST!");
            // The row is NOT left open for the world prompt. `combat.md §14`
            // says the stored string "has no trailing newline", and playtest
            // F41 read that as "the prompt follows directly" - but the stock
            // capture shows a blank row between `BATTLE IS LOST!` and the
            // world prompt (`qa/paired/combat-escape-ordinary.tsv`, edge,
            // stepped-out and after beats; matched in rc2, broke when F41
            // marked this row open). Whatever the framer prints on the way
            // back to the world closes it, so the ordinary
            // `text-output.md §10.4` blank applies.
        } else {
            state.apply_combat_round_loop_exit(exit);
            // `combat.md §14`: the accepted Escape key "appends the character
            // `!`, producing `Escape!`; this handler appends no newline after
            // it" - the echo row stays open for the next prompt's feed to
            // close (playtest F41).
            if matches!(
                application.action,
                CombatPlayerCommandAction::EscapeCleanup { application }
                    if application.decision == CombatEscapeCleanupDecision::Accepted
            ) && !state.combat_active
            {
                state.leave_last_transcript_row_open();
            }
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
            // `commands.md §5.2` gives R the echo `Ready...`, and the arena
            // uses the ordinary verb echoes - combat U right below this emits
            // its own the same way. This branch emitted none, so the arena's
            // Ready went straight to its result with no echo row at all.
            // Measured 2026-09-12 (`qa/paired/combat-ready-armour.tsv`, beat
            // `ready-open`): the original shows ` Ready...` above
            // `Thou art empty-handed!`.
            state.begin_command_echo_for(Command::Ready);
            state.start_combat_ready_equipment(actor_slot);
            if state.active_ready.is_none() {
                // An empty pack answers `Thou art empty-handed!` and opens no
                // picker, so this is not a multistage command: the caller
                // advances the round, which is what reprints the banner the
                // original shows under the refusal. `combat.md §8.1`'s
                // no-banner short form is the *free re-prompt* case, and this
                // arm charges its turn.
                return false;
            }
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
            // `combat.md §10`: "when Negate Magic's `N` tag is active, the
            // cast is absorbed before the shared spell dispatcher or
            // spell-selection prompt runs. The exact command output is
            // `Cast...\nAbsorbed!\n`", followed by the manual absorption
            // envelope. `magic.md §7`: "There is no spell-selection prompt";
            // the attempt "completes the current combat action while
            // spending no premixed charge or MP". The interference gate above
            // takes precedence. This arm used to open `Spell name:` first and
            // absorb only after the exchange (playtest E35).
            if resolve_negate_magic_absorbs_combat_cast(
                state.active_effect_tag,
                state.active_effect_counter,
            ) {
                let had_foe = combat_has_active_non_party_actor(state);
                state.message = format!("{SCENE_ABSORBED_CAST_MESSAGE}\n");
                state.emit_sound_effect(SoundEffect::MagicAbsorbed);
                finish_combat_cast_actor_action(state, actor_slot, had_foe);
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
                    // `combat.md §8`, the `Y` row: "Prints the combat Yell
                    // label and dispatches to the shared Yell handler", and
                    // `commands.md §11` has combat's scene `0xFF` ask "the
                    // ordinary `Yell what?` question". The arena opened the
                    // reader with nothing on screen - no label, no question,
                    // no colon row (playtest F27) - so it is the world
                    // prompt's own echo here.
                    state.begin_command_echo_for(Command::Yell);
                    state.active_yell = Some(YellSession::new());
                    if state.complete_open_direction_echo("Yell ", YELL_QUESTION) {
                        state.message = state.render_active_yell();
                    } else {
                        state.message = format!("Yell {YELL_QUESTION}\n:");
                    }
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
            if let Some(direction) = suffix
                .chars()
                .find_map(Direction::from_play_key)
                .filter(|direction| direction.is_cardinal())
            {
                // `combat.md §8`, the `P` row: "Prints the `Push-` label" -
                // the surface label even in a dungeon-room arena, whose
                // `Push` refusal form never runs here (playtest H27).
                if let Some(echo) = command_echo(Command::Push, CommandEchoMode::Surface) {
                    state.begin_command_echo(echo);
                }
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
                // That echo is the label's only print: `combat.md §8`'s `P`
                // row prints `Push-` once, and the world dispatcher's verb
                // echo opened here as well stacked a second `Push-` (or the
                // dungeon `Push`) row above it (playtest H23/H27).
                let echo = state.render_active_direction_prompt();
                state.message.clear();
                state.emit_combat_command_echo_line(&echo);
            }
            true
        }
        CombatCommandBranch::Klimb => {
            // `combat.md §8` (u5-spec#298 Q1): "K prints `Klimb-` and resolves
            // its current-cell ladder cases first" - a single ladder (or a
            // room grate) under the acting member climbs at once, with no key
            // read; only the two-way ambush centre waits, on `U/D-`.
            let current_cell = suffix
                .is_empty()
                .then(|| state.combat_klimb_current_cell(actor_slot))
                .flatten();
            if let Some(intent) = match current_cell {
                Some(CombatKlimbCurrentCell::Up) => Some(ClimbIntent::Up),
                Some(CombatKlimbCurrentCell::Down) => Some(ClimbIntent::Down),
                _ => None,
            } {
                state.message.clear();
                state.emit_combat_command_echo_line(DUNGEON_KLIMB_PROMPT_ONE);
                let outcome = state.klimb_combat_actor_vertical(actor_slot, intent);
                if matches!(outcome, MoveOutcome::Blocked) {
                    // `combat.md §8.4` "Turn cost" (u5-spec#299 Q13): a
                    // refused Up/Down departure is free and "the parser takes
                    // the short re-prompt with no banner".
                    state.pending_combat_actor_slot = Some(actor_slot);
                    state.combat_prompt_row_opened_by_banner = false;
                } else if state.combat_active {
                    let _ = apply_combat_committed_action_maintenance(state, actor_slot);
                    advance_combat_round_after_actor_and_append_message(state, actor_slot);
                }
            } else if let Some(intent) = suffix.chars().find_map(combat_klimb_vertical_intent) {
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
                // The inline form of the adjacent-climb prompt prints what the
                // interactive one does (`combat.md §8.4`, u5-spec#299 Q13):
                // the `Klimb-` row completed by the direction name, then any
                // `What?` on its own row and the short re-prompt.
                state.message.clear();
                state.emit_combat_command_echo_line(&format!(
                    "{DUNGEON_KLIMB_PROMPT_ONE}{}",
                    direction.name()
                ));
                let outcome = state.klimb_combat_actor_direction(actor_slot, direction);
                if matches!(outcome, MoveOutcome::Blocked) {
                    state.pending_combat_actor_slot = Some(actor_slot);
                    state.combat_prompt_row_opened_by_banner = false;
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
    // `combat.md §8.3`: a typed digit becomes a direction only through the
    // shared reader's numpad flag, which remaps all eight of `1`-`4` and
    // `6`-`9` at once; with the flag clear "typed digits keep their ASCII
    // meaning and reach the active-player-selection handler". A plain digit
    // here is the flag-clear case - the frontend delivers a remapped keypad
    // digit as a direction input code, exactly as the world loops'
    // `Set Active Plr:` path assumes. Playtest E36: `2`/`4`/`6`/`8` stepped
    // the actor while `1`/`3`/`5` selected a member, which is neither mode.
    if key.is_ascii_digit() {
        return CombatPlayerCommandInput::Key(key);
    }
    if let Some(direction_code) =
        Direction::from_play_key(key).and_then(combat_direction_code_for_direction)
    {
        return CombatPlayerCommandInput::Direction(direction_code);
    }
    CombatPlayerCommandInput::Key(key.to_ascii_uppercase())
}

fn combat_player_command_message(
    state: &PlayState,
    action: &CombatPlayerCommandAction,
) -> String {
    match action {
        CombatPlayerCommandAction::QuicknessSkipped => "Quickness!".to_string(),
        // `combat.md §8.4`: the digit answers with `Set active plr:\n` and
        // then `None!`, the selected actor's name, or `Invalid!`. This
        // engine answered `Active player selected.` for all three, which
        // is published nowhere and says nothing about which member is now
        // active.
        CombatPlayerCommandAction::ActivePlayerSelection(outcome) => {
            let answer = match outcome {
                CombatActivePlayerSelectionOutcome::Clear => {
                    COMBAT_SET_ACTIVE_PLAYER_NONE.to_string()
                }
                // §8.4's "selected actor name" is the same name the rest
                // of combat prints for a party-side slot, which is what
                // `combat_actor_display_name` serves.
                CombatActivePlayerSelectionOutcome::SelectPartySlot(slot) => state
                    .party_names
                    .get(*slot)
                    .and_then(|name| crate::party_name_to_string(name))
                    .unwrap_or_else(|| COMBAT_SET_ACTIVE_PLAYER_INVALID.to_string()),
                CombatActivePlayerSelectionOutcome::Invalid => {
                    COMBAT_SET_ACTIVE_PLAYER_INVALID.to_string()
                }
            };
            format!("{COMBAT_SET_ACTIVE_PLAYER_LABEL}{answer}\n")
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
            // `R` and `U` are Shape A letters too, and their verb labels are
            // the shared world echoes `commands.md §5.2` publishes,
            // `Ready...\n\n` and `Use_item\n\n` - the same echoes the
            // party-side arms below print before their delegates. Without
            // them a controlled monster's `R`/`U` printed a bare `Can't!`
            // (playtest E38).
            //
            // `combat.md §8.4` (u5-spec#299 Q19): the refusal is `Can't!\n`
            // in every row - R gives `Ready...`, a blank row, `Can't!`, a
            // blank row, then the reprinted banner. Its feed closes the
            // refusal's row, so the full re-prompt's leading newline leaves
            // that blank row; without it the banner ran straight under
            // `Can't!`. Cast prints `Cast...` "and its newline".
            let label = match branch {
                CombatCommandBranch::CastSpell => Some("Cast...\n"),
                CombatCommandBranch::Ready => Some("Ready...\n\n"),
                CombatCommandBranch::UseItem => Some("Use item\n\n"),
                _ => combat_command_branch_published_label(*branch),
            };
            format!("{}{COMBAT_PARTY_SIDE_REFUSAL}\n", label.unwrap_or(""))
        }
        CombatPlayerCommandAction::Branch { branch, .. } => combat_command_branch_message(*branch),
    }
}

fn combat_command_branch_message(branch: CombatCommandBranch) -> String {
    if let Some(label) = combat_command_branch_published_label(branch) {
        // The two `What?` refusals close their own row, exactly as the
        // scene-abort literals below do: `combat.md §8.1`'s banner then leaves
        // the blank row the original shows between the refusal and the next
        // `Avatar, armed with bare hands:`. Measured 2026-09-12
        // (`qa/paired/combat-refusal-audio.tsv`, beat `silent`): the stock
        // game has a blank row under ` D-What?` and this engine ran the banner
        // straight on. The direction-prompt prefixes that share this label
        // table must not gain the feed, so it is added here rather than to the
        // label.
        if matches!(
            branch,
            CombatCommandBranch::DWhatRefusal | CombatCommandBranch::WWhatRefusal
        ) {
            return format!("{label}\n");
        }
        return label.to_string();
    }

    match branch {
        CombatCommandBranch::SceneMessageAbort(verb) => match combat_scene_abort_tail(verb) {
            // `combat.md §8`, the X-it row: "`X-it what?\n`, followed by the
            // refusal tones and a full retry" - the literal carries its own
            // line feed, which closes its row so `§8.1`'s banner leaves the
            // blank the original shows between the two. Measured 2026-09-09
            // (`qa/paired/combat-commands.tsv`, `xit` beat).
            CombatSceneAbortTail::What => {
                format!("{} what?\n", combat_scene_abort_verb_prefix(verb))
            }
            CombatSceneAbortTail::NotHere => {
                // The trailing feed closes the refusal's row, which is what
                // lets the next actor banner's own leading feed land as a
                // blank row rather than merely closing this one -
                // `combat.md §8.4`: "Combat's next full actor banner
                // contributes its own leading feed, making the blank row
                // after the result." The sibling `What` arm above already
                // carries it.
                //
                // Measured 2026-09-11 (`combat-refusal-audio/refused`): the
                // original reads `Look-Not here`, a blank row, then the
                // banner; this engine ran the banner onto the row beneath
                // the refusal and sat one row short for the rest of the
                // capture.
                format!("{}-Not here\n", combat_scene_abort_verb_prefix(verb))
            }
            CombatSceneAbortTail::FunnyNoResponse => {
                // With its trailing newline, exactly as the `-Not here`
                // tail above carries one: `combat.md §8`'s rule is that a
                // result's "trailing newline closes the last result row.
                // The next full actor banner begins with its own newline,
                // **leaving a blank row after the result**." Measured
                // 2026-09-19 (`qa/paired/combat-refusal-letters.tsv`, beat
                // `tv`): the original reads `Talk-Funny, no` / `response!`
                // / blank / `Avatar, armed`, and without the feed this
                // engine ran the banner onto the row beneath the refusal.
                format!(
                    "{}-Funny, no response!\n",
                    combat_scene_abort_verb_prefix(verb)
                )
            }
        },
        CombatCommandBranch::Klimb => DUNGEON_KLIMB_WHAT_REFUSAL.trim_end().to_string(),
        // The music toggle is a harness control, not a game command.
        CombatCommandBranch::ToggleMusic => String::new(),
        CombatCommandBranch::Invalid => "What?".to_string(),
        // Ready has already written its own echo and, on an empty pack, its
        // refusal; the round advance that follows must add nothing. Without
        // this it fell to the debug-formatted arm below and printed a stray
        // ` Ready.` row under the refusal
        // (`qa/paired/combat-ready-armour.tsv`, beat `ready-open`).
        CombatCommandBranch::Ready => String::new(),
        CombatCommandBranch::Attack
        | CombatCommandBranch::CastSpell
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
        | CombatCommandBranch::WWhatRefusal => {
            // Every branch here has already written whatever it prints - its
            // own echo, its own refusal, or nothing at all - so the round
            // advance that follows adds no line.
            //
            // This used to be `format!("{branch:?}.")`, which puts a Rust enum
            // name in the message window for any branch that reaches it. It
            // was invisible only because nothing did until the Ready arm above
            // started letting the round advance, at which point the original's
            // `Thou art empty-handed!` gained a ` Ready.` row underneath
            // (`qa/paired/combat-ready-armour.tsv`, beat `ready-open`). A
            // debug format is not a fallback the player should ever be able to
            // see, so it is gone rather than special-cased further.
            String::new()
        }
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
            .unwrap_or_else(|| combat_player_command_message(state, &application.action)),
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
    state: &mut PlayState,
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
    state: &mut PlayState,
    target_slot: usize,
    attack: CombatWeaponAttackApplication,
) -> Option<String> {
    combat_attack_result_message(state, target_slot, None, attack)
}

fn combat_attack_result_message(
    state: &mut PlayState,
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
    state: &mut PlayState,
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
    // `combat.md §11.1`: a split class's `<monster> divides!` is printed
    // "inside the damage handler, and the ordinary result line **still**
    // prints after it" - after the damage roll's shatter line, before the
    // result line (playtest H20).
    let inside_damage_roll = match (inside_damage_roll, state.combat_pending_divide_line.take()) {
        (Some(inside), Some(divide)) => Some(format!("{inside}\n{divide}")),
        (inside, divide) => inside.or(divide),
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
    state: &mut PlayState,
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
            // `combat.md §11.1` census: "After any narrated hit on a
            // **party** target | both | *no text* | full stats-panel redraw
            // and a visibility pass - skipped on the graze arm and on the
            // vanish arm" ("Order, stated once", step 6). Immediate, so it
            // does not wait on the running command's own panel mechanism:
            // stock captures show the hit points drop right under the line
            // (`combat-damage-narration` 20261016-220421, 20261022-122324).
            // A vanish-gated result never reaches this producer's caller's
            // print, but the vanish arm only fells monsters.
            state.repaint_stats_panel();
            state.mark_visibility_dirty();
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
            //
            // "Because this is the same classifier, a score of 1 - and a
            // score of 2 on the morale draw described in Section 9 - also
            // raises the target's fleeing bit as a side effect of narrating
            // the hit": the classifier runs here, morale draw included.
            //
            // The single direction-keyed swing runs it at hit time, ahead of
            // the committed-action tail, and leaves a marker so this later
            // transcript assembly does not take the morale draw twice.
            let max_hp = combat_class_stats(damage.class)?.max_hp;
            if state.combat_wound_classified_at_hit.take() != Some(target_slot) {
                let _ = state.apply_combat_wound_score_classifier(target_slot);
            }
            let actor = state.combat_actors.get(target_slot)?;
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
    state: &mut PlayState,
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
    state: &mut PlayState,
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
    state: &mut PlayState,
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
                slot,
                action:
                    CombatActorDispatchAction::MonsterAi {
                        ai_turn: Some(ai_turn),
                    },
                ..
            } => Some((
                *slot,
                ai_turn.monster_attack?,
                CombatMonsterAttackNarration::SelfActingHostile,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    // `combat.md §6.3`: the narrator gate already ran inside each attack's
    // own dispatch, while the shared result field still described that
    // slot; this loop only prints what the gate let through.
    for (slot, attack, narration) in attacks {
        if let Some(line) = combat_monster_attack_narrated_result_message(state, attack, narration)
        {
            // `combat.md §11.1`, the order stated once: step 3 is "a newline",
            // step 5 the result line - which carries its own trailing newline.
            state.emit_combat_print(&format!("\n{line}\n"));
        }
        // The attacker's standing-hazard line follows its attack's result.
        state.flush_combat_pending_hazard_lines(Some(slot));
    }
    state.flush_combat_pending_hazard_lines(None);
}

fn drive_combat_round_walk_and_append_message(state: &mut PlayState) {
    if !state.combat_active || state.pending_combat_actor_slot.is_some() {
        return;
    }

    // `combat.md §7` restarts the table walk whenever slot 32 is reached
    // with actors still present, so the walk runs until a keyboard-driven
    // actor needs a key. The drain limit only guards against a walk that
    // makes no progress; it has to count walks in which nothing acted, not
    // every walk. A dragged-under victim's `ARGH!` turns (`§8.1`) and the
    // foes' turns around them can outlast thirty-seven tables, and the
    // walker then stopped mid-fight with no prompt open: the rest of the
    // round - a last foe's `escapes!` and `VICTORY!` - only printed once
    // the next key arrived (playtest Q02).
    let mut idle_walks = 0;
    for _ in 0..COMBAT_ROUND_WALK_PROGRESS_LIMIT {
        if idle_walks >= COMBAT_ROUND_WALK_DRAIN_LIMIT {
            break;
        }
        let start_slot = state.next_combat_actor_slot.min(COMBAT_ACTOR_SLOTS);
        let application = if state.pace_combat_presentations {
            state.apply_combat_round_walk_from_slot_paced(start_slot, COMBAT_PHASE_REFRESH_CONSTANT)
        } else {
            state.apply_combat_round_walk_from_slot(start_slot, COMBAT_PHASE_REFRESH_CONSTANT)
        };
        if combat_round_walk_dispatched_an_action(&application) {
            idle_walks = 0;
        } else {
            idle_walks += 1;
        }
        append_combat_round_walk_messages(state, &application);
        state.next_combat_actor_slot = match application.stop_reason {
            CombatRoundWalkStopReason::EndOfRound => 0,
            CombatRoundWalkStopReason::AwaitingPlayer
            | CombatRoundWalkStopReason::AutomaticAction
            | CombatRoundWalkStopReason::Exit => application.next_slot,
        };
        if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
            // `stats-panel.md §2.3`'s fourth consumer, drained once per
            // acting character at the round walker's `AwaitingPlayer`
            // boundary - see the sibling call in `combat_frame.rs`.
            state.drain_stats_panel_refresh();
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

/// Upper bound on the walks one drive runs while actors keep acting - an
/// engine-only non-termination guard far above any fight's length between
/// two keyboard turns.
const COMBAT_ROUND_WALK_PROGRESS_LIMIT: usize = 4096;

/// Whether any slot in `application` took a turn: anything but an empty
/// slot, a counting-down phase counter or an active-player skip.
fn combat_round_walk_dispatched_an_action(application: &CombatRoundWalkApplication) -> bool {
    application.applications.iter().any(|entry| {
        matches!(
            entry,
            CombatActorSlotDispatchApplication::Slot { action, .. }
                if !matches!(
                    action,
                    CombatActorDispatchAction::Inactive
                        | CombatActorDispatchAction::Waiting
                        | CombatActorDispatchAction::ActivePlayerSentinelSkipped
                )
        )
    })
}

/// `combat.md §8.1` / `dungeon-mode.md §8.1` (issue #210): once a fight is
/// framed, the round walker runs to the first keyboard-driven actor and its
/// turn banner is on the screen before the first combat key is read - the
/// verified rows for a fall into a room are "`Entering room...`, combat's
/// blank row, a word-wrapped banner ... and the prompt".
///
/// A no-op unless combat is active with no actor pending. The paced
/// frontends walk the round on their own presentation pump instead
/// ([`advance_paced_combat_presentation`]), one automatic action per frame.
pub fn open_owed_combat_player_turn(state: &mut PlayState) {
    if state.combat_active
        && state.pending_combat_actor_slot.is_none()
        && state.active_combat_targeting.is_none()
        && !state.pace_combat_presentations
    {
        drive_combat_round_walk_and_append_message(state);
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
    // `combat.md §11.1`: the standing hazard's result line follows the
    // actor's own output, which every caller has printed by now.
    state.flush_combat_pending_hazard_lines(None);
    state.next_combat_actor_slot = actor_slot.saturating_add(1).min(COMBAT_ACTOR_SLOTS);
    if !state.pace_combat_presentations {
        // A staged beat the action queued - the Shard's `\n\nNo effect!\n`
        // behind its swell (`inventory.md §7.1`) - is still part of this
        // actor's output, and the next actor's banner (`combat.md §8.1`)
        // follows it. The paced shell holds the walk while the narration
        // owns the loop; an unpaced walk runs now, so print the beat first
        // (playtest F33). A spell flash's holds are not such beats: they
        // print nothing and already hold the window display over whatever
        // the walk prints (`magic.md §8`), so they are left to run out.
        if state.staged_narration_active() && !state.message_window_flash_hold_active() {
            state.flush_staged_narration();
        }
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
    ///
    /// And with the space after it. `§8.1` says the question "is followed
    /// immediately by a closing double quote **and one space**", and that
    /// space is the cell the accepted item letter echoes into - measured
    /// 2026-09-10 on the same scenario's `buyquote` beat, where the original
    /// reads `see?" a` and this engine read `interest?"a`.
    #[test]
    fn arms_stock_call_pool_is_the_published_four_with_their_closing_quote() {
        assert_eq!(arms_stock_call_for_roll(0), "What may I show thee?\" ");
        assert_eq!(
            arms_stock_call_for_roll(1),
            "Which wouldst thou like to see?\" "
        );
        assert_eq!(arms_stock_call_for_roll(2), "What is thine interest?\" ");
        assert_eq!(arms_stock_call_for_roll(3), "Which would ye see?\" ");
    }

    /// The draw is uniform over `0..3`, so the pool wraps rather than
    /// panicking if a wider roll ever reaches it.
    /// `shops.md §8.B`: the shipwright's menu is record `119`, which the
    /// asset supplies. Without `SHOPPE.DAT` the engine shows nothing rather
    /// than a transcription of the record (playtest Q16).
    #[test]
    fn shipwright_menu_without_shoppe_dat_has_no_transcribed_fallback() {
        assert_eq!(
            format_ship_broker_outcome(
                crate::shop_runtime::ShipBrokerOutcome::EnteredMenu,
                None,
                None
            ),
            ""
        );
    }

    #[test]
    fn arms_stock_call_pool_wraps_past_three() {
        for roll in 0u8..=u8::MAX {
            assert_eq!(
                arms_stock_call_for_roll(roll),
                arms_stock_call_for_roll(roll % 4)
            );
        }
    }

    /// `shops.md §8.1` / `catalogs/item-list.md §5.1.2`: canonical names of
    /// thirteen or more characters swap to their short label in the buy list,
    /// and shorter names keep the canonical spelling. Playtest F43: the
    /// Shattered Shield list wrapped `Ring of Regeneration` and broke
    /// `Amulet/Turning` mid-word.
    #[test]
    fn arms_buy_list_swaps_long_names_for_their_short_labels() {
        let rows = format_arms_stock_buy_menu(
            crate::shops::ArmsStockTable::new([7, 32, 36, 27, 31, 44, 45, 0], 7),
            None,
        );
        assert_eq!(
            rows,
            "a...Magic Shield\nb...2H Axe\nc...Magic Bow\nd...Arrows\n\
             e...2H Hammer\nf...Regen Ring\ng...Am/Turning\n"
        );
        let swapped: Vec<usize> = (0..crate::EQUIPMENT_COUNT)
            .filter(|&id| arms_buy_list_name(id) != equipment_name(id))
            .collect();
        assert_eq!(swapped, vec![6, 10, 15, 35, 40, 42, 43, 44, 45, 46]);
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

    /// `systems/shops.md §8.A`, the "Arms buy stock list" row: invalid stock
    /// letters "leave the stock list visible and keep waiting; they do not
    /// redraw the list or consume a random draw". `§8.1` repeats it: "Invalid
    /// listing keys keep the existing text and consume no new draw."
    ///
    /// This engine re-rendered the stock rows, which appended a second copy of
    /// the list under the first. Measured 2026-09-10 on
    /// `shop-arms-menus/sell`, where the scripted `S` is not a stock letter:
    /// the original's window is byte-identical to the previous beat's.
    ///
    /// The window text and the PRNG word are both observables here - a redraw
    /// would change the first, a fresh call-line draw the second.
    #[test]
    fn arms_invalid_buy_letter_leaves_the_window_and_the_prng_untouched() {
        let mut state = stocked_arms_state();
        handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();

        let prng_after_entry = state.prng_state;
        let message_after_entry = state.message.clone();

        // `d` is past the three-entry stock table, so it is an invalid buy
        // selector rather than a purchase.
        handle_play_key_input(&mut state, 'd', "", Path::new("")).unwrap();

        assert_eq!(
            state.prng_state, prng_after_entry,
            "an invalid selector must not consume a random draw"
        );
        assert_eq!(
            state.message, message_after_entry,
            "an invalid selector must leave the existing text exactly as it was"
        );
    }

    /// `systems/shops.md §8.A` lists the closing bark as two rows, one for a
    /// visit that bought nothing and one for a visit that completed a
    /// purchase, each a uniform `0..3` draw into its own row. Measured
    /// 2026-09-10: `shop-arms-buy-confirm` buys and leaves, and the original
    /// closed it on record `7`, which a `0..3` draw into a row based at `0`
    /// cannot reach.
    #[test]
    fn the_arms_closing_bark_row_follows_the_visits_outcome() {
        for roll in 0..4 {
            assert_eq!(
                arms_closing_bark_record(false, roll),
                Some(usize::from(roll))
            );
            assert_eq!(
                arms_closing_bark_record(true, roll),
                Some(usize::from(roll) + 4)
            );
        }
        // Every record the draw can reach is inside the shared 0..7 band.
        for completed in [false, true] {
            for roll in 0..4 {
                assert!(arms_closing_bark_record(completed, roll).unwrap() < 8);
            }
        }
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
        assert_eq!(rendered, "\"Thou canst not carry any more!\"\nsays Kitiara.");
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
            "\"Thou canst not carry any more!\""
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

    /// `systems/shops.md §8.1`: "A decline or carry-cap refusal uses this
    /// same prompt, retaining whether an earlier purchase completed."
    /// Measured 2026-09-10: `shop-arms-menus/buydecline` shows the original on
    /// `then?` with nothing bought this visit, and
    /// `shop-arms-buy-confirm/declined` on `sir?` after a purchase. This
    /// engine printed the redrawn listing with no prompt at all, which left
    /// the two windows a row apart for the rest of the visit.
    #[test]
    fn arms_declined_quote_keeps_the_post_item_prompt_and_its_retained_suffix() {
        for (completed, suffix) in [(false, "then?"), (true, "sir?")] {
            let mut state = stocked_arms_state();
            state.arms_transaction_completed = completed;
            // `B` opens the buy listing, `a` quotes the first stock row, and
            // `N` declines it.
            for key in ['B', 'a', 'N'] {
                handle_active_shop_key_input(&mut state, key, "", game_dir());
            }
            let rendered = state.message.clone();
            // `shops.md §8.A`: the `No` echo is followed by a blank line,
            // so two feeds lead the prompt (playtest K07).
            assert!(
                rendered.starts_with(&format!("\n\n\"Anything else, {suffix}")),
                "declined quote with completed={completed} rendered {rendered:?}"
            );
        }
    }

    /// The shared `SHOPPE.DAT` bark records carry their own closing quote and
    /// no opening one, so the renderer supplies the opening quote only.
    /// Measured 2026-09-10 on `shop-arms-menus/exit`: the original printed one
    /// closing quote while this engine printed `"Harrumph!""`.
    #[test]
    fn shared_shoppe_flourish_does_not_double_the_records_closing_quote() {
        assert_eq!(quote_shoppe_flourish("Harrumph!\""), "\"Harrumph!\"");
        assert_eq!(
            quote_shoppe_flourish("Come back when you're ready to buy something!\""),
            "\"Come back when you're ready to buy something!\""
        );
        // A record without the trailing quote still gets one, so the guard
        // cannot leave a line unclosed.
        assert_eq!(quote_shoppe_flourish("Harrumph!"), "\"Harrumph!\"");
        // The rendered record is trimmed before it is quoted.
        assert_eq!(quote_shoppe_flourish("  Harrumph!\" "), "\"Harrumph!\"");
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
