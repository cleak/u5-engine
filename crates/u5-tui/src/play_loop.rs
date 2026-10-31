//! Terminal play loop, rendering, input parsing, and script harness.
//!
//! Moved out of `u5-runtime` -- these helpers are TUI-only. Game logic
//! and the cross-shell input dispatcher (`handle_play_key_input`,
//! `PlayInputDisposition`) stay in `u5-runtime`.

use std::collections::VecDeque;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Duration;

use u5_runtime::stats_panel::transcribe_cell_frame_for_plain_text;
use u5_runtime::{
    Area, Direction, ExplorationTurnGateOutcome, INPUT_CODE_EAST, INPUT_CODE_NORTH,
    INPUT_CODE_NORTHEAST, INPUT_CODE_NORTHWEST, INPUT_CODE_SOUTH, INPUT_CODE_SOUTHEAST,
    INPUT_CODE_SOUTHWEST, INPUT_CODE_WEST, PLAY_EXIT_TO_DOS_KEY, PLAY_IGNORED_INPUT_KEY,
    PLAY_MUSIC_TOGGLE_KEY, PLAY_SCRIPT_MAX_COMBAT_STALL_PASSES, PLAY_SCRIPT_MAX_IDLE_TICKS,
    PLAY_TYPEAHEAD_TOGGLE_KEY, PlayInputDisposition, PlayOptions, PlayState, SoundEffect,
    TileAtlas, TileGraphicsDepth, combat_awaits_automatic_turns, handle_play_key_input, hash_bytes,
    hash_palette_indices, input_function_key_code, load_tile_atlas,
    run_potion_flash_soundless_timing,
};

pub fn run_play_loop(
    game_dir: &Path,
    options: PlayOptions,
    raster_diagnostics: bool,
    raster_depth: TileGraphicsDepth,
    play_script: Option<Vec<String>>,
) -> io::Result<()> {
    let intro_target = options.target;
    let intro_floor = options.floor;
    let mut state = PlayState::load_scene(game_dir, options)?;
    let tile_atlas = if raster_diagnostics {
        Some(load_tile_atlas(game_dir, raster_depth)?)
    } else {
        None
    };
    println!("Ultima V playable harness");
    println!(
        "Scene {} floor/level {}. Town/world move: arrow keys, Home/Page/End, or legacy lowercase wasd/yubn (a plain digit is Set Active Player, not a direction). Dungeon: W/S forward/back, A/D turn. Attack: A prompts or A+dir. Enter: e. Open: O prompts in town or o+dir. Get/Search: G/S prompt or +dir. Push: P prompts in town or p+dir. Hole up: h+hours. Look: l+dir; fountain and wishing-well prompts continue after Look. View: v. Use: U opens picker or UT/UG/UK/U1-U8. Stats: Z. Ignite: i. Talk: T prompts or TKEYWORD. Climb: k/< />. Board/Xit/Yell sails: B/X/x/Y. Fire: F prompts on ships or f+dir. Cast: C opens spell-name and follow-up prompts, or inline C1IL/C1AZ2/C1AN2/C1M2/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS/C1LV/C1HR/C1IP6/C1IQW/C1AWY/C1PU/C1DP/C1AG6/C1GIN6/C1GIS6/C1AEP/C1EIP/C1PRV2/C1AT. Mix: M opens mixer or MIL/0x80/1. Order: N opens prompt or N12. Yell: Y opens prompt or YWORD. Top-down save: Q prompts or QY/QN. Dungeon exit prompt: Q prompts or QY/QN. Buffer/typeahead: buffer. Combat music toggle: music. Idle animation: . Optional startup wind/Grapple/transport/raster diagnostics: --wind, --grapple, --climbing-gear, --transport, --raster-diagnostics, --raster-depth ega|tandy. Pass: Space/Enter. Harness quit: q.",
        intro_target.key(),
        intro_floor
    );
    if let Some(commands) = play_script {
        println!("Script mode: {} command(s).", commands.len());
        return run_play_script_commands(&mut state, game_dir, &commands, tile_atlas.as_ref());
    }
    run_interactive_play_loop(
        &mut state,
        game_dir,
        tile_atlas.as_ref(),
        &mut io::stdin().lock(),
    )
}

/// Whether the terminal loop runs the input helper's idle wait before it
/// reads the next key: the same test the visual shell's idle pump applies
/// (`u5-bevy`'s `visual_idle_tick`), so a modal surface that owns the key
/// wait - a picker, a line reader, the party selector - does not take the
/// overworld helper's pass.
pub fn play_state_runs_input_wait_pass(state: &PlayState) -> bool {
    play_state_accepts_typeahead(state) && state.active_party_selector.is_none()
}

/// `overworld.md §8` (`RETRACTIONS.md` R320) and `timing.md §8.2`: the
/// overworld input helper does work of its own "from the top of the input
/// helper, before that turn's key is read" - a waterfall directly south of
/// the party takes the falls chain there, with no key pressed - and "performs
/// one world step when it is entered".
///
/// The visual shell pumps that pass every frame. The terminal shell blocks on
/// a line read and has no timer pump, so it runs the pass exactly once per
/// input-helper entry: once before each key it reads. That is every point at
/// which the original's helper is entered; the further timer-paced passes of
/// a long wait are pacing this harness does not reproduce.
///
/// Under sail the pass is the first half of the auto-advance (`timing.md
/// §8.2`); a swallowed same-heading key completes it through
/// [`PlayState::run_under_sail_advance_body`]. A key that is *not* swallowed
/// "ends the loop and becomes that turn's command", so the next entry starts
/// a fresh pass rather than resuming the half the command interrupted.
///
/// Returns whether a pass ran.
pub fn run_terminal_input_wait_pass(state: &mut PlayState, game_dir: &Path) -> io::Result<bool> {
    if !play_state_runs_input_wait_pass(state) {
        return Ok(false);
    }
    state.under_sail_wait_cursor_poll_pending = false;
    state.idle_wait_pass(Some(game_dir))?;
    Ok(true)
}

/// The interactive terminal loop, reading one line per key from
/// `input_reader` until it is exhausted or the player quits.
pub fn run_interactive_play_loop<R: BufRead>(
    state: &mut PlayState,
    game_dir: &Path,
    tile_atlas: Option<&TileAtlas>,
    input_reader: &mut R,
) -> io::Result<()> {
    let mut input = String::new();
    let mut queued_input = VecDeque::new();
    loop {
        // The terminal shell has no animation pump, so a staged presentation
        // would never advance here. Print the rest of it at once rather than
        // stall: this shell is a harness, not a pacing reference.
        state.flush_staged_narration();
        state.flush_text_hold();
        // `blackthorn.md §7`'s rescue cinematic still has to reach its
        // handoff, and this shell has no pump to walk it there.
        state.run_blackthorn_rescue_to_handoff(game_dir)?;
        state.run_blackthorn_audience_exit_to_handoff(game_dir)?;
        resolve_shrine_third_mantra_without_pump(state, game_dir)?;
        // `endgame.md §7` steps 4-6 read no key and this shell has no pump,
        // so the box hand-over is run to record `3` here; a key typed during
        // it waits in the buffer and is replayed below.
        state.finish_endgame_victory_staging();
        // Anything typed while a hold owned the loop is replayed now.
        while u5_runtime::input_dispatch::drain_buffered_hold_key(state, game_dir)?.is_some() {}
        if play_state_accepts_typeahead(state) {
            match state.apply_exploration_turn_gate(game_dir)? {
                ExplorationTurnGateOutcome::Ready { .. } => {}
                ExplorationTurnGateOutcome::Slept { .. } => {
                    print_play_frame(state, tile_atlas)?;
                    // Host-side pacing only: the clean spec requires a brief
                    // pause but does not publish a duration for this shell.
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                ExplorationTurnGateOutcome::Rescued { .. } => {
                    print_play_frame(state, tile_atlas)?;
                    continue;
                }
            }
        }
        // The input helper's own pass, before the key is read; see
        // [`run_terminal_input_wait_pass`].
        run_terminal_input_wait_pass(state, game_dir)?;
        print_play_frame(state, tile_atlas)?;
        if !play_state_honours_typeahead_queue(state) {
            queued_input.clear();
        }
        let (key, suffix) = if let Some(key) = queued_input.pop_front() {
            (key, String::new())
        } else {
            print!("> ");
            io::stdout().flush()?;
            input.clear();
            if input_reader.read_line(&mut input)? == 0 {
                break;
            }
            if play_state_whole_line_reader(state) {
                match free_text_line_input(&input) {
                    FreeTextLineInput::Submit(key, suffix) => (key, suffix),
                    FreeTextLineInput::Discard => continue,
                    FreeTextLineInput::Key => match play_input_key_and_suffix(&input) {
                        Some((key, suffix)) => (key, suffix),
                        None => {
                            handle_empty_play_input(state, game_dir)?;
                            continue;
                        }
                    },
                }
            } else if play_state_typeahead_setting_in_force(state) {
                if let Some(keys) = play_input_typeahead_chars(&input) {
                    let mut keys = keys.into_iter();
                    let key = keys.next().expect("typeahead input is non-empty");
                    queued_input.extend(keys);
                    (key, String::new())
                } else if let Some((key, suffix)) = play_input_key_and_suffix(&input) {
                    (key, suffix)
                } else {
                    handle_empty_play_input(state, game_dir)?;
                    continue;
                }
            } else if let Some((key, suffix)) = play_input_key_and_suffix(&input) {
                (key, suffix)
            } else {
                handle_empty_play_input(state, game_dir)?;
                continue;
            }
        };
        let effects_before = state.sound_effect_serial;
        if handle_play_key_input(state, key, &suffix, game_dir)? == PlayInputDisposition::Quit {
            break;
        }
        if key_flushed_typeahead(state, effects_before) {
            queued_input.clear();
        }
    }
    Ok(())
}

/// `audio.md §7.4`: a refused overworld or town step "Prints `Blocked!`,
/// beeps, flushes keyboard type-ahead", whatever the Buffer setting - the
/// setting governs only the per-read flush of `input.md §6`. The beep marks
/// the refusal arms that flush, the test the visual shell applies: the silent
/// whirlpool refusal and the under-sail arms take neither, and combat's own
/// blocked-step beep publishes no flush. `dungeon-mode.md §14`'s contact
/// sequence flushes too ("4. Flush type-ahead"), before the arena opens, so
/// its request is honoured with combat already live (playtest P19-05).
pub fn key_flushed_typeahead(state: &PlayState, effects_before: u64) -> bool {
    state.typeahead_flush_requested
        || (!state.combat_active
            && state
                .sound_effects_after(effects_before)
                .iter()
                .any(|effect| matches!(effect, SoundEffect::BlockedStep)))
}

/// Walk a pending third-mantra resolution to its end in a shell with no
/// presentation pump.
///
/// `karma.md §7.2`: nothing between the Return that ends the third mantra and
/// the next command row waits for a key - the turn-in arm's attribute lines
/// print "after the whole sound" and its exit pacing follows. The shells flush
/// staged narration *before* this resolution runs, so what the resolution
/// itself stages has to be flushed here too; it used to wait for the next
/// command, leaving `Strength +1` and the prompt off the frame (playtest H41).
fn resolve_shrine_third_mantra_without_pump(
    state: &mut PlayState,
    game_dir: &Path,
) -> io::Result<()> {
    if state.pending_shrine_third_mantra.is_none() {
        return Ok(());
    }
    state.run_shrine_third_mantra_resolution_to_completion(game_dir)?;
    state.flush_staged_narration();
    Ok(())
}

pub fn run_play_script_commands(
    state: &mut PlayState,
    game_dir: &Path,
    commands: &[String],
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    run_play_script_load_entry_reaction(state, game_dir)?;
    print_play_script_snapshot(state, tile_atlas)?;
    for (index, command) in commands.iter().enumerate() {
        println!(
            "script[{}]: {}",
            index + 1,
            play_script_command_label(command)
        );
        let disposition = handle_play_script_command(state, command, game_dir)?;
        // No pump in script mode either; see the interactive loop above.
        state.flush_staged_narration();
        state.flush_text_hold();
        // The cinematics and the shrine's third-mantra resolution need
        // walking to their end here too, exactly as
        // [`replay_play_script_commands`] does: the exploration gate stays
        // closed while one is pending, so without this every command after a
        // third shrine mantra was a no-op (playtest E58).
        state.run_blackthorn_rescue_to_handoff(game_dir)?;
        state.run_blackthorn_audience_exit_to_handoff(game_dir)?;
        resolve_shrine_third_mantra_without_pump(state, game_dir)?;
        // The script shells run no exploration turn gate, so the acted turn an
        // E-Enter shrine visit owes once its exit pacing has drained is paid
        // here, as the gate pays it (`commands.md §3`, playtest S14).
        u5_runtime::input_dispatch::settle_exploration_acted_charge(state, game_dir)?;
        drain_play_script_prompt_head_panel_refresh(state);
        print_play_script_snapshot(state, tile_atlas)?;
        if disposition == PlayInputDisposition::Quit {
            break;
        }
    }
    Ok(())
}

/// `dungeon-mode.md §4` Initialisation: a save resumed on a dungeon room
/// cell runs the room-entry helper before the first command is read, and
/// `§5` lists resuming a dungeon save among the dungeon-mode entries. The
/// interactive shells reach it through the exploration gate before they draw
/// the waiting frame; a script runner records its first frame straight after
/// the load and met the gate only when a command arrived, so an empty script
/// showed the corridor prompt instead of the room (playtest P24-03).
fn run_play_script_load_entry_reaction(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if play_state_accepts_typeahead(state) {
        u5_runtime::run_dungeon_entry_underfoot_reaction(state, game_dir)?;
    }
    Ok(())
}

/// `main-loop.md §6` step 1 / `stats-panel.md §2.3`: in the overworld and
/// town a pending stats-panel refresh request is drained "at the head of a
/// command prompt ... before the prompt newline and the key read". The
/// interactive shells reach that point through the exploration gate before
/// they draw the waiting frame; a script runner records its frame as soon as
/// the command returns, and only met the gate when the *next* command
/// arrived, so a Heal's deferred repaint showed one command late (playtest
/// G10). Only the drain is run here: the gate's other work (the sleeping
/// arm's turn, the dungeon entry reaction) still belongs to the next
/// command's gate. Combat drains at its own prompts.
fn drain_play_script_prompt_head_panel_refresh(state: &mut PlayState) {
    if play_state_accepts_typeahead(state)
        && !state.combat_active
        && state.pending_blackthorn_rescue.is_none()
    {
        state.drain_stats_panel_refresh();
    }
}

pub fn replay_play_script_commands<F>(
    state: &mut PlayState,
    game_dir: &Path,
    commands: &[String],
    mut after_command: F,
) -> io::Result<()>
where
    F: FnMut(&mut PlayState, usize, &str) -> io::Result<()>,
{
    run_play_script_load_entry_reaction(state, game_dir)?;
    for (index, command) in commands.iter().enumerate() {
        let disposition = handle_play_script_command(state, command, game_dir)?;
        state.flush_staged_narration();
        state.flush_text_hold();
        // The cinematics need walking to their handoff here as well as in the
        // idle gate. The gate only runs while `play_state_accepts_typeahead`,
        // which is false for the whole of a Blackthorn audience, so a script
        // that finished an audience never reached the exit handoff and ended
        // in the throne room instead of the destination scene -
        // `blackthorn-audience-correct` and `-wrong`.
        state.run_blackthorn_rescue_to_handoff(game_dir)?;
        state.run_blackthorn_audience_exit_to_handoff(game_dir)?;
        resolve_shrine_third_mantra_without_pump(state, game_dir)?;
        // The script shells run no exploration turn gate, so the acted turn an
        // E-Enter shrine visit owes once its exit pacing has drained is paid
        // here, as the gate pays it (`commands.md §3`, playtest S14).
        u5_runtime::input_dispatch::settle_exploration_acted_charge(state, game_dir)?;
        drain_play_script_prompt_head_panel_refresh(state);
        after_command(state, index, command)?;
        if disposition == PlayInputDisposition::Quit {
            break;
        }
    }
    Ok(())
}

pub fn print_play_frame(state: &mut PlayState, tile_atlas: Option<&TileAtlas>) -> io::Result<()> {
    write_play_frame(state, tile_atlas, &mut io::stdout())
}

/// Terminal body of [`print_play_frame`], writing to an arbitrary sink so
/// the emitted bytes can be inspected.
///
/// `render_text_window_frame` transcribes the emitted CELL SURFACE, whose
/// active-player marker is the fixed-cell font's glyph code `0x1A`
/// (`stats-panel.md §4`, party-row column 33). A terminal has no glyph
/// table, so that byte must be transcribed to the plain-text stand-in
/// before it is printed; see
/// [`transcribe_cell_frame_for_plain_text`].
pub fn write_play_frame<W: Write>(
    state: &mut PlayState,
    tile_atlas: Option<&TileAtlas>,
    out: &mut W,
) -> io::Result<()> {
    complete_headless_blocking_presentations(state, tile_atlas)?;
    writeln!(out)?;
    writeln!(out, "{}", state.render_text_frame(5))?;
    writeln!(
        out,
        "{}",
        transcribe_cell_frame_for_plain_text(&render_play_text_frame(state))
    )?;
    if let Some(atlas) = tile_atlas {
        writeln!(out, "{}", raster_diagnostic_line(state, 5, atlas)?)?;
    }
    // The dissolve is a completed blocking driver call. This frontend has no
    // intermediate pixel page, so printing the resulting state acknowledges
    // the pending completion records rather than retaining them forever.
    let _ = state.take_pending_map_viewport_dissolves();
    let _ = state.take_pending_blackthorn_rescue_playbacks();
    let _ = state.take_pending_stonegate_trapdoor_playback();
    Ok(())
}

/// The text frame a terminal or play-script step prints.
///
/// `render_text_window_frame` paints the message window from the message
/// slot alone, so a frame showed only whichever text last landed there: a
/// conversation step lost its keyword echo and the reply before the next
/// `Your interest?` (`conversation.md §6`, playtest P24-11), and a Pass
/// repeated the previous command's `Blocked!` instead of its own `Pass`
/// echo (`commands.md §8.1`, playtest P24-12). The message window rows are
/// replaced here with the layout `--save-screen` composes from the
/// transcript, the open prompt and the live row. A full-screen endgame page
/// owns every cell and keeps its own rows.
pub fn render_play_text_frame(state: &mut PlayState) -> String {
    let frame = state.render_text_window_frame(None);
    if u5_runtime::endgame_narrative_frame::endgame_narrative_full_screen_text_rows(state).is_some()
        || u5_runtime::endgame_certificate_frame::endgame_certificate_full_screen_text_rows(state)
            .is_some()
    {
        return frame;
    }
    let layout = crate::save_frame::save_screen_message_window_layout(state);
    overlay_message_window_rows(&frame, &layout)
}

/// Clear the message window's cells in a 40-column text frame and write the
/// layout's rows into them. A prefixed row's column-24 ribbon end cap is a
/// sprite with no text cell, so it stays blank here.
pub fn overlay_message_window_rows(
    frame: &str,
    layout: &u5_runtime::MessageWindowLayout,
) -> String {
    use u5_runtime::{
        MESSAGE_WINDOW_BOTTOM, MESSAGE_WINDOW_LEFT, MESSAGE_WINDOW_RIGHT, MESSAGE_WINDOW_TOP,
    };
    let mut rows: Vec<Vec<char>> = frame.split('\n').map(|row| row.chars().collect()).collect();
    let left = usize::from(MESSAGE_WINDOW_LEFT);
    let right = usize::from(MESSAGE_WINDOW_RIGHT);
    for y in usize::from(MESSAGE_WINDOW_TOP)..=usize::from(MESSAGE_WINDOW_BOTTOM) {
        let Some(row) = rows.get_mut(y) else { continue };
        for cell in row.iter_mut().take(right + 1).skip(left) {
            *cell = ' ';
        }
    }
    for placed in &layout.rows {
        let Some(row) = rows.get_mut(usize::from(placed.row)) else {
            continue;
        };
        for (offset, glyph) in placed.glyphs.iter().enumerate() {
            let x = usize::from(placed.column) + offset;
            if x > right {
                break;
            }
            if let Some(cell) = row.get_mut(x) {
                *cell = char::from(glyph.byte);
            }
        }
    }
    rows.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn print_play_script_snapshot(
    state: &mut PlayState,
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    write_play_script_snapshot(state, tile_atlas, &mut io::stdout())
}

/// Terminal body of [`print_play_script_snapshot`]; same cell-surface to
/// plain-text transcription as [`write_play_frame`].
pub fn write_play_script_snapshot<W: Write>(
    state: &mut PlayState,
    tile_atlas: Option<&TileAtlas>,
    out: &mut W,
) -> io::Result<()> {
    complete_headless_blocking_presentations(state, tile_atlas)?;
    writeln!(out, "{}", play_script_state_line(state))?;
    writeln!(
        out,
        "{}",
        transcribe_cell_frame_for_plain_text(&render_play_text_frame(state))
    )?;
    if let Some(atlas) = tile_atlas {
        writeln!(out, "{}", raster_diagnostic_line(state, 5, atlas)?)?;
    }
    let _ = state.take_pending_map_viewport_dissolves();
    let _ = state.take_pending_blackthorn_rescue_playbacks();
    let _ = state.take_pending_stonegate_trapdoor_playback();
    Ok(())
}

/// Complete synchronous presentation work before a terminal or headless
/// frontend accepts another command. With an atlas, the visibility sweep's
/// twenty frames run through the normal map compositor; text-only mode still
/// advances the same presentation state even though it has no persistent
/// pixel page to display.
pub fn complete_headless_blocking_presentations(
    state: &mut PlayState,
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    if let Some(playback) = state.take_pending_potion_flash() {
        let _ = run_potion_flash_soundless_timing(playback);
    }

    while state.visibility_sweep.is_some() {
        let rendered = if let Some(atlas) = tile_atlas {
            state.render_top_down_base_frame(5, atlas)?.is_some()
        } else {
            false
        };
        if !rendered {
            state.advance_presentation_frame();
        }
    }
    Ok(())
}

pub fn raster_diagnostic_line(
    state: &mut PlayState,
    radius: usize,
    atlas: &TileAtlas,
) -> io::Result<String> {
    let Some(viewport) = state.render_top_down_frame(radius, atlas)? else {
        return Ok("Raster viewport: unavailable for this mode.".to_string());
    };
    Ok(format!(
        "Raster {}: {}x{} px, {}x{} cells, {}, hash {:016x}.",
        raster_frame_kind(state),
        viewport.width,
        viewport.height,
        viewport.cells_wide,
        viewport.cells_high,
        viewport.depth.label(),
        hash_palette_indices(&viewport.pixels)
    ))
}

pub fn raster_frame_kind(state: &PlayState) -> &'static str {
    if state.active_view_overlay.is_some() {
        "view overlay"
    } else if state.endgame.is_some() {
        "endgame tableau"
    } else if state.combat_active {
        "combat viewport"
    } else if matches!(state.area, Area::Dungeon { .. }) {
        "dungeon first-person viewport"
    } else {
        "tile viewport"
    }
}

pub fn play_script_state_line(state: &PlayState) -> String {
    format!(
        "State: {} at ({}, {}), facing {}, turn {}, date Y{} M{} D{} {:02}:{:02}, transport {}, wind {}, typeahead {}, music {}, message-bytes {} hash {:016x}.",
        state.current_area_label(),
        state.player.x,
        state.player.y,
        state.player.facing.name(),
        state.turn,
        state.clock.year,
        state.clock.month,
        state.clock.day,
        state.clock.hour,
        state.clock.minute,
        state.player.transport.status_label(),
        state.wind.status_message(),
        state.typeahead_status_label(),
        state.music_status_label(),
        state.message.len(),
        hash_bytes(state.message.as_bytes())
    )
}

pub fn play_input_key_and_suffix(input: &str) -> Option<(char, String)> {
    let input = input.trim_end_matches(|ch| ch == '\r' || ch == '\n');
    if is_typeahead_toggle_token(input) {
        return Some((PLAY_TYPEAHEAD_TOGGLE_KEY, String::new()));
    }
    if is_music_toggle_token(input) {
        return Some((PLAY_MUSIC_TOGGLE_KEY, String::new()));
    }
    if is_return_token(input) {
        return Some(('\r', String::new()));
    }
    if is_program_exit_token(input) {
        return Some((PLAY_EXIT_TO_DOS_KEY, String::new()));
    }
    if let Some(key) = ansi_navigation_key(input) {
        return Some((key, String::new()));
    }
    if let Some(key) = ansi_function_key(input).and_then(input_function_key_code) {
        return Some((char::from(key), String::new()));
    }
    if unclassified_escape_sequence(input) {
        return Some((PLAY_IGNORED_INPUT_KEY, "escape".to_string()));
    }
    let mut chars = input.chars();
    chars.next().map(|key| (key, chars.collect()))
}

pub fn play_input_typeahead_chars(input: &str) -> Option<Vec<char>> {
    let input = input.trim_end_matches(|ch| ch == '\r' || ch == '\n');
    if input.is_empty()
        || is_typeahead_toggle_token(input)
        || is_music_toggle_token(input)
        || ansi_navigation_key(input).is_some()
        || ansi_function_key(input).is_some()
        || unclassified_escape_sequence(input)
    {
        return None;
    }
    let chars = input.chars().collect::<Vec<_>>();
    if chars.len() > 1 && chars.iter().all(|key| is_simple_typeahead_key(*key)) {
        Some(chars)
    } else {
        None
    }
}

/// The prompts whose session takes one typed token as the whole line the
/// free-text reader returns on Enter: the conversation keyword and ASK-WHO
/// prompts and the Yell prompt. Their handlers concatenate `key` and
/// `suffix` into the submitted line and leave the line editing to the shell
/// (`handle_active_conversation_key_input`: "The shell clears its own line
/// buffer").
pub fn play_state_whole_line_reader(state: &PlayState) -> bool {
    state.active_conversation.is_some() || state.active_yell.is_some()
}

/// How the shell hands one input token to a whole-line reader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FreeTextLineInput {
    /// A typed line ended by Enter, as the session's `key` and `suffix`.
    Submit(char, String),
    /// Only keys the reader discards were typed; no Enter was pressed, so
    /// the prompt keeps waiting.
    Discard,
    /// Not typed text: Return, an empty line, or a bare Escape, which the
    /// session handles as a key.
    Key,
}

/// `input.md §8`, the free-text reader's byte rules applied to one token.
///
/// The token is a typed line: "A printable ASCII character - appended to the
/// buffer", "A backspace - pops the most recent character", Escape "clears
/// the whole current line but does **not** submit or terminate it", and
/// "Any other byte (function key, direction code, etc.) - discarded". Enter
/// "terminates the prompt, returning the accumulated string" - every
/// character of the word, never one line per character (playtest G01, G21).
///
/// A token that names a single non-text key - an arrow or function-key
/// sequence, the type-ahead, music or Exit-to-DOS chords - is a key the
/// reader discards (`§5`: a typed Control character reaches no prompt), so
/// the prompt keeps waiting.
pub fn free_text_line_input(input: &str) -> FreeTextLineInput {
    let input = input.trim_end_matches(['\r', '\n']);
    if input.is_empty() || input == "\x1b" || is_return_token(input) {
        return FreeTextLineInput::Key;
    }
    if is_typeahead_toggle_token(input)
        || is_music_toggle_token(input)
        || is_program_exit_token(input)
        || ansi_navigation_key(input).is_some()
        || ansi_function_key(input).is_some()
        || unclassified_escape_sequence(input)
    {
        return FreeTextLineInput::Discard;
    }
    let mut line = String::new();
    let mut typed_text = false;
    for ch in input.chars() {
        match ch {
            '\u{08}' => {
                line.pop();
            }
            '\u{1b}' => line.clear(),
            ' '..='~' => {
                typed_text = true;
                line.push(ch);
            }
            _ => {}
        }
    }
    if !typed_text {
        return FreeTextLineInput::Discard;
    }
    let mut chars = line.chars();
    match chars.next() {
        Some(key) => FreeTextLineInput::Submit(key, chars.collect()),
        // Everything typed was erased; Enter "then submits whatever remains,
        // including an empty buffer".
        None => FreeTextLineInput::Submit('\r', String::new()),
    }
}

pub fn is_simple_typeahead_key(key: char) -> bool {
    Direction::from_play_key(key).is_some() || matches!(key, '.' | ' ')
}

pub fn is_typeahead_toggle_token(input: &str) -> bool {
    matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "buffer" | "typeahead" | "typeahead-buffer" | "toggle-buffer"
    )
}

pub fn is_music_toggle_token(input: &str) -> bool {
    matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "music" | "sound" | "ctrl-s" | "control-s"
    )
}

/// `commands.md` Section 9's Control + `E` row. A real terminal delivers the
/// chord as the raw control byte, which already dispatches, so this token only
/// gives scripts and the keyless harness the same reach.
/// A scripted Return.
///
/// The reader trims a trailing carriage return from every token, so a bare
/// `\r` in a `--play-script` reaches the dispatcher as the empty token and
/// is taken for a pass. Prompt flows - the party-member picker, the
/// equipment picker, a free-text line - are driven by Return, so scripting
/// them needs a name for it.
pub fn is_return_token(input: &str) -> bool {
    matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "return" | "enter"
    )
}

pub fn is_program_exit_token(input: &str) -> bool {
    matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "ctrl-e" | "control-e" | "exit-to-dos"
    )
}

pub fn ansi_navigation_key(input: &str) -> Option<char> {
    ansi_navigation_input_byte(input).map(char::from)
}

pub fn ansi_navigation_input_byte(input: &str) -> Option<u8> {
    match input {
        "\x1b[A" | "\x1bOA" => Some(INPUT_CODE_NORTH),
        "\x1b[B" | "\x1bOB" => Some(INPUT_CODE_SOUTH),
        "\x1b[D" | "\x1bOD" => Some(INPUT_CODE_WEST),
        "\x1b[C" | "\x1bOC" => Some(INPUT_CODE_EAST),
        "\x1b[H" | "\x1bOH" | "\x1b[1~" | "\x1b[7~" => Some(INPUT_CODE_NORTHWEST),
        "\x1b[5~" => Some(INPUT_CODE_NORTHEAST),
        "\x1b[F" | "\x1bOF" | "\x1b[4~" | "\x1b[8~" => Some(INPUT_CODE_SOUTHWEST),
        "\x1b[6~" => Some(INPUT_CODE_SOUTHEAST),
        _ => None,
    }
}

pub fn ansi_function_key(input: &str) -> Option<u8> {
    match input {
        "\x1bOP" | "\x1b[11~" | "\x1b[[A" => Some(1),
        "\x1bOQ" | "\x1b[12~" | "\x1b[[B" => Some(2),
        "\x1bOR" | "\x1b[13~" | "\x1b[[C" => Some(3),
        "\x1bOS" | "\x1b[14~" | "\x1b[[D" => Some(4),
        "\x1b[15~" | "\x1b[[E" => Some(5),
        "\x1b[17~" => Some(6),
        "\x1b[18~" => Some(7),
        "\x1b[19~" => Some(8),
        "\x1b[20~" => Some(9),
        "\x1b[21~" => Some(10),
        _ => None,
    }
}

pub fn unclassified_escape_sequence(input: &str) -> bool {
    input.starts_with('\x1b')
        && input.chars().nth(1).is_some()
        && ansi_navigation_key(input).is_none()
        && ansi_function_key(input).is_none()
}

pub fn handle_empty_play_input(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if !play_state_accepts_typeahead(state) {
        let _ = handle_play_key_input(state, '\n', "", game_dir)?;
    } else if state.combat_active
        // Playtest N05: inside a room fight the party already stands on the
        // room cell; re-resolving the trigger re-ran the room entry and
        // erased the encounter. `combat.md §8`: Space passes the actor.
        || state
            .resolve_current_dungeon_room_trigger(Some(game_dir))?
            .is_none()
    {
        // `commands.md §8.1`: "There is no distinct Pass input: Space is the
        // key and `Pass` is its echo." Dispatching the key rather than calling
        // the handler directly is what puts that echo on the transcript; the
        // pass prints no result of its own, so without the echo the turn left
        // no trace in the message window at all.
        let _ = handle_play_key_input(state, ' ', "", game_dir)?;
    }
    Ok(())
}

pub fn handle_play_script_command(
    state: &mut PlayState,
    command: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    settle_play_script_exploration_gate(state, game_dir)?;
    run_play_script_input_helper_entry(state);
    let command = command.trim();
    if is_typeahead_toggle_token(command) {
        return handle_play_key_input(state, PLAY_TYPEAHEAD_TOGGLE_KEY, "", game_dir);
    }
    if is_music_toggle_token(command) {
        return handle_play_key_input(state, PLAY_MUSIC_TOGGLE_KEY, "", game_dir);
    }
    if is_return_token(command) {
        return handle_play_key_input(state, '\r', "", game_dir);
    }
    if is_program_exit_token(command) {
        return handle_play_key_input(state, PLAY_EXIT_TO_DOS_KEY, "", game_dir);
    }
    // Endgame tableau movement and the gate presentation are driven by
    // rendered frames, not keypresses. Scripted acceptance routes use this
    // command to pump exactly one owed presentation frame.
    if command.eq_ignore_ascii_case("endgame:frame") {
        if !state.advance_endgame_display_frame() {
            return Err(io::Error::other(
                "play script `endgame:frame` found no pending endgame presentation frame",
            ));
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(command.to_ascii_lowercase().as_str(), "empty" | "pass") {
        handle_empty_play_input(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    // One pass of the input helper's idle wait with no key pressed
    // (`timing.md §8.2`), the pass the visual shell pumps every frame. Work
    // that runs "from the top of the input helper, before that turn's key is
    // read" - the directly-south arm of the `overworld.md §8` falls chain -
    // happens here and nowhere a key reaches.
    if command.eq_ignore_ascii_case("input-wait") {
        state.idle_wait_pass(Some(game_dir))?;
        return Ok(PlayInputDisposition::Continue);
    }
    if let Some(count) = play_script_idle_tick_count(command)? {
        // An open prompt is still the shared wait, which steps the world on
        // every no-key pass (`input.md §2`, u5-spec#303 Q3); a `.` key would
        // instead be read by the prompt as a key.
        // The shrine and Codex reads and the endgame's ten shared-wait sites
        // are among them (u5-spec#304 Q4), each pass stepping the world.
        if state.prompt_waits_in_shared_wait() {
            for _ in 0..count {
                state.prompt_wait_pass();
            }
            return Ok(PlayInputDisposition::Continue);
        }
        // `endgame.md §8.3` step 8 (u5-spec#304 Q4, R629): a final-narrative
        // page wait is a tight loop on the raw key check, with "no cursor
        // blink, no world step and no one-tick pacing". A pass with no key
        // there does nothing at all - and a `.` would be read as the page key.
        if state.endgame_raw_key_page_wait_open() {
            return Ok(PlayInputDisposition::Continue);
        }
        // Each `.` is a no-key pass inside the command wait, not a command
        // returning to the input helper, so it owes no new entry world step:
        // the helper takes one step per prompt (`overworld.md §5` item 5,
        // `town-mode.md §7` step 1), however long the wait (playtest P16-21).
        let entry_owed = state.overworld_input_helper_entry_owed;
        for _ in 0..count {
            if handle_play_key_input(state, '.', "", game_dir)? == PlayInputDisposition::Quit {
                return Ok(PlayInputDisposition::Quit);
            }
        }
        if state.harness_shortcut_keys {
            state.overworld_input_helper_entry_owed = entry_owed;
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if play_state_whole_line_reader(state) {
        match free_text_line_input(command) {
            FreeTextLineInput::Submit(key, suffix) => {
                return handle_play_key_input(state, key, &suffix, game_dir);
            }
            FreeTextLineInput::Discard => return Ok(PlayInputDisposition::Continue),
            FreeTextLineInput::Key => {}
        }
    } else if play_state_typeahead_setting_in_force(state) {
        if let Some(keys) = play_input_typeahead_chars(command) {
            for key in keys {
                if !play_state_honours_typeahead_queue(state) {
                    break;
                }
                settle_play_script_exploration_gate(state, game_dir)?;
                run_play_script_input_helper_entry(state);
                let effects_before = state.sound_effect_serial;
                if handle_play_key_input(state, key, "", game_dir)? == PlayInputDisposition::Quit {
                    return Ok(PlayInputDisposition::Quit);
                }
                if key_flushed_typeahead(state, effects_before) {
                    break;
                }
            }
            return Ok(PlayInputDisposition::Continue);
        }
    }
    if let Some(code) = play_script_direction_prompt_digit(state, command) {
        return handle_play_key_input(state, char::from(code), "", game_dir);
    }
    let Some((key, suffix)) = play_input_key_and_suffix(command) else {
        handle_empty_play_input(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    };
    handle_play_key_input(state, key, &suffix, game_dir)
}

/// A play script answers an open `Direction-` prompt with a compass digit,
/// which stands for the keypad key with NumLock on: `input.md §5` translates
/// that one into a direction code before any prompt sees it, while "Plain
/// unmodified top-row digits remain ordinary text input" and no longer
/// answer the prompt (R589). The script step delivers the translated code,
/// as an arrow key typed at the terminal does. Digits anywhere else (a
/// member picker, say) stay digits.
fn play_script_direction_prompt_digit(state: &PlayState, command: &str) -> Option<u8> {
    if state.active_direction_prompt.is_none() {
        return None;
    }
    match command.as_bytes() {
        [digit @ b'1'..=b'9'] => u5_runtime::input_keypad_digit_direction_code(digit - b'0'),
        _ => None,
    }
}

/// `overworld.md §5` item 5 (u5-spec#305 Q5): "Input-helper entry: **one
/// world tick, unconditionally**, at every prompt, the session's first prompt
/// included." A script command is a key read at that prompt, so the step the
/// last key (or the load) left owed runs before it, as the interactive loop's
/// [`run_terminal_input_wait_pass`] and the visual shell's entry step do.
/// Without it a script never rolled the wind, never took the session's first
/// full step and never set the loaded world-tick gate byte back to one
/// (playtest P13-03).
///
/// A prompt still open means the command that owed the step has not
/// returned, so the step is dropped, as the visual shell drops it; the key
/// that finishes the command owes it again.
fn run_play_script_input_helper_entry(state: &mut PlayState) {
    if !state.overworld_input_helper_entry_owed {
        return;
    }
    if !play_state_accepts_typeahead(state) {
        state.overworld_input_helper_entry_owed = false;
        return;
    }
    let _ = state.run_overworld_input_helper_entry_step();
}

fn settle_play_script_exploration_gate(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if !play_state_accepts_typeahead(state) {
        return Ok(());
    }
    let mut combat_stalls = 0;
    for _ in 0..PLAY_SCRIPT_MAX_IDLE_TICKS {
        // The same three lines the interactive loop above runs before its own
        // gate call, for the same reason: a script has no animation pump, so
        // a staged presentation never advances and the rescue cinematic never
        // reaches its handoff. Without them the gate sees
        // `pending_blackthorn_rescue` still set on every one of its 1024
        // passes and reports the party as never becoming command-ready -
        // which is what `blackthorn-rescue-refuge`,
        // `britannia-defeat-persists-ool-before-rescue` and
        // `stonegate-trapdoor-rescue` were failing with.
        state.flush_staged_narration();
        state.flush_text_hold();
        state.run_blackthorn_rescue_to_handoff(game_dir)?;
        state.run_blackthorn_audience_exit_to_handoff(game_dir)?;
        resolve_shrine_third_mantra_without_pump(state, game_dir)?;
        match state.apply_exploration_turn_gate(game_dir)? {
            ExplorationTurnGateOutcome::Ready { .. } => return Ok(()),
            // Playtest P17-14: in combat each gate pass drives the round
            // walker to its own guard, so a pass that still leaves the fight
            // owing automatic turns found no keyboard turn to stop at. A
            // second one in a row ends the script instead of re-driving the
            // walker up to 1024 times with nothing on screen.
            ExplorationTurnGateOutcome::Slept { .. }
                if state.combat_active && combat_awaits_automatic_turns(state) =>
            {
                combat_stalls += 1;
                if combat_stalls >= PLAY_SCRIPT_MAX_COMBAT_STALL_PASSES {
                    return Err(io::Error::other(format!(
                        "combat reached no keyboard turn after {combat_stalls} automatic passes"
                    )));
                }
            }
            ExplorationTurnGateOutcome::Slept { .. }
            | ExplorationTurnGateOutcome::Rescued { .. } => combat_stalls = 0,
        }
    }
    Err(io::Error::other(format!(
        "party capability did not become command-ready after {PLAY_SCRIPT_MAX_IDLE_TICKS} automatic passes"
    )))
}

pub fn play_state_accepts_typeahead(state: &PlayState) -> bool {
    state.pending_town_arrest.is_none()
        && state.endgame.is_none()
        && state.active_blackthorn.is_none()
        && state.active_shop.is_none()
        && state.active_conversation.is_none()
        && state.active_z_stats.is_none()
        && state.active_ready.is_none()
        && state.active_use.is_none()
        && state.active_cast.is_none()
        && state.active_cast_followup.is_none()
        && state.active_rest.is_none()
        && state.active_jimmy.is_none()
        && state.active_surface_chest.is_none()
        && state.active_shrine.is_none()
        && state.pending_shrine_third_mantra.is_none()
        && state.active_mix.is_none()
        && state.active_new_order.is_none()
        && state.active_yell.is_none()
        && state.active_shrine_restoration.is_none()
        && state.active_wishing_well.is_none()
        && state.active_direction_prompt.is_none()
        && state.active_yes_no_prompt.is_none()
}

/// `systems/input.md §6`, third writer of the type-ahead setting:
/// "the free-text line reader saves the setting, forces type-ahead on
/// for the duration of a typed line, and restores it afterwards. That
/// last one means typing a name, a keyword, or a word of power always
/// honours the queue regardless of the player's choice."
///
/// `§8` step 2 states the same thing from the prompt's side: "Disable
/// the buffer flush. The flush gate (Section 6) is cleared so the
/// player can type ahead. The prompt restores it on exit."
///
/// The membership test is the session's accumulating line buffer: each
/// of these sessions holds a `String` it appends typed characters to,
/// which is what makes it the free-text line reader rather than one of
/// §8's single-character prompts (Y/N, a digit, a target-slot letter),
/// which run the loop exactly once and are not covered.
pub fn play_state_free_text_line_reader(state: &PlayState) -> bool {
    state.active_conversation.is_some()
        || state.active_yell.is_some()
        || state.active_shrine_restoration.is_some()
        || state.active_cast.is_some()
        || state.active_cast_followup.is_some()
        || state.active_mix.is_some()
}

/// `systems/input.md §6`: whether a queued keystroke survives to the
/// next read. The non-text modals still flush, but a free-text line
/// reader forces the honour-the-queue state for the duration of the
/// typed line, so the queue must not be cleared under one.
///
/// Save/restore of the underlying setting is implicit here: nothing in
/// this path writes `typeahead_buffer_enabled`, so the player's own
/// choice is exactly what is in force again once the session closes.
pub fn play_state_honours_typeahead_queue(state: &PlayState) -> bool {
    play_state_accepts_typeahead(state) || play_state_free_text_line_reader(state)
}

/// `systems/input.md §6`: the effective type-ahead setting for the
/// current state — the player's toggle, forced on while a free-text
/// line reader is open.
pub fn play_state_typeahead_setting_in_force(state: &PlayState) -> bool {
    if play_state_free_text_line_reader(state) {
        return true;
    }
    state.typeahead_buffer_enabled && play_state_accepts_typeahead(state)
}

pub fn play_script_idle_tick_count(command: &str) -> io::Result<Option<usize>> {
    let command = command.trim();
    let lower = command.to_ascii_lowercase();
    if matches!(lower.as_str(), "idle" | "tick" | "ticks") {
        return Ok(Some(1));
    }
    let Some(value) = lower
        .strip_prefix("idle:")
        .or_else(|| lower.strip_prefix("tick:"))
    else {
        return Ok(None);
    };
    let count = value.parse::<usize>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("script idle command `{command}` has invalid tick count: {err}"),
        )
    })?;
    if count == 0 || count > PLAY_SCRIPT_MAX_IDLE_TICKS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "script idle command `{command}` tick count must be 1..{PLAY_SCRIPT_MAX_IDLE_TICKS}"
            ),
        ));
    }
    Ok(Some(count))
}

pub fn play_script_command_label(command: &str) -> String {
    if command.is_empty() {
        return "empty".to_string();
    }
    if let Some(function_key) = ansi_function_key(command.trim()) {
        return format!("F{function_key}");
    }
    let mut label = String::new();
    for ch in command.chars() {
        if ch.is_control() {
            label.push_str(&format!("\\x{:02x}", ch as u32));
        } else {
            label.push(ch);
        }
    }
    label
}
