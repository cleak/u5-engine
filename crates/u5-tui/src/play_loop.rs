//! Terminal play loop, rendering, input parsing, and script harness.
//!
//! Moved out of `u5-runtime` -- these helpers are TUI-only. Game logic
//! and the cross-shell input dispatcher (`handle_play_key_input`,
//! `PlayInputDisposition`) stay in `u5-runtime`.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use u5_runtime::stats_panel::transcribe_cell_frame_for_plain_text;
use u5_runtime::{
    Area, Direction, ExplorationTurnGateOutcome, INPUT_CODE_EAST, INPUT_CODE_NORTH,
    INPUT_CODE_NORTHEAST, INPUT_CODE_NORTHWEST, INPUT_CODE_SOUTH, INPUT_CODE_SOUTHEAST,
    INPUT_CODE_SOUTHWEST, INPUT_CODE_WEST, PLAY_IGNORED_INPUT_KEY, PLAY_MUSIC_TOGGLE_KEY,
    PLAY_SCRIPT_MAX_IDLE_TICKS, PLAY_TYPEAHEAD_TOGGLE_KEY, PlayInputDisposition, PlayOptions,
    PlayState, TileAtlas, TileGraphicsDepth, handle_play_key_input, hash_bytes,
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
        "Scene {} floor/level {}. Town/world move: arrow keys, Home/Page/End, numpad 1-9, or legacy lowercase wasd/yubn. Dungeon: W/S forward/back, A/D turn. Attack: A prompts or A+dir. Enter: e. Open: O prompts in town or o+dir. Get/Search: G/S prompt or +dir. Push: P prompts in town or p+dir. Hole up: h+hours. Look: l+dir; fountain and wishing-well prompts continue after Look. View: v. Use: U opens picker or UT/UG/UK/U1-U8. Stats: Z. Ignite: i. Talk: T prompts or TKEYWORD. Climb: k/< />. Board/Xit/Yell sails: B/X/x/Y. Fire: F prompts on ships or f+dir. Cast: C opens spell-name and follow-up prompts, or inline C1IL/C1AZ2/C1AN2/C1M2/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS/C1LV/C1HR/C1IP6/C1IQW/C1AWY/C1PU/C1DP/C1AG6/C1GIN6/C1GIS6/C1AEP/C1EIP/C1PRV2/C1AT. Mix: M opens mixer or MIL/0x80/1. Order: N opens prompt or N12. Yell: Y opens prompt or YWORD. Top-down save: Q prompts or QY/QN. Dungeon exit prompt: Q prompts or QY/QN. Buffer/typeahead: buffer. Combat music toggle: music. Idle animation: . Optional startup wind/Grapple/transport/raster diagnostics: --wind, --grapple, --climbing-gear, --transport, --raster-diagnostics, --raster-depth ega|tandy. Pass: Space/Enter. Harness quit: q.",
        intro_target.key(),
        intro_floor
    );
    if let Some(commands) = play_script {
        println!("Script mode: {} command(s).", commands.len());
        return run_play_script_commands(&mut state, game_dir, &commands, tile_atlas.as_ref());
    }
    let mut input = String::new();
    let mut queued_input = VecDeque::new();
    loop {
        if play_state_accepts_typeahead(&state) {
            match state.apply_exploration_turn_gate(game_dir)? {
                ExplorationTurnGateOutcome::Ready { .. } => {}
                ExplorationTurnGateOutcome::Slept { .. } => {
                    print_play_frame(&mut state, tile_atlas.as_ref())?;
                    // Host-side pacing only: the clean spec requires a brief
                    // pause but does not publish a duration for this shell.
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                ExplorationTurnGateOutcome::Rescued { .. } => {
                    print_play_frame(&mut state, tile_atlas.as_ref())?;
                    continue;
                }
            }
        }
        print_play_frame(&mut state, tile_atlas.as_ref())?;
        if !play_state_honours_typeahead_queue(&state) {
            queued_input.clear();
        }
        let (key, suffix) = if let Some(key) = queued_input.pop_front() {
            (key, String::new())
        } else {
            print!("> ");
            io::stdout().flush()?;
            input.clear();
            if io::stdin().read_line(&mut input)? == 0 {
                break;
            }
            if play_state_typeahead_setting_in_force(&state) {
                if let Some(keys) = play_input_typeahead_chars(&input) {
                    let mut keys = keys.into_iter();
                    let key = keys.next().expect("typeahead input is non-empty");
                    queued_input.extend(keys);
                    (key, String::new())
                } else if let Some((key, suffix)) = play_input_key_and_suffix(&input) {
                    (key, suffix)
                } else {
                    handle_empty_play_input(&mut state, game_dir)?;
                    continue;
                }
            } else if let Some((key, suffix)) = play_input_key_and_suffix(&input) {
                (key, suffix)
            } else {
                handle_empty_play_input(&mut state, game_dir)?;
                continue;
            }
        };
        if handle_play_key_input(&mut state, key, &suffix, game_dir)? == PlayInputDisposition::Quit
        {
            break;
        }
    }
    Ok(())
}

pub fn run_play_script_commands(
    state: &mut PlayState,
    game_dir: &Path,
    commands: &[String],
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    print_play_script_snapshot(state, tile_atlas)?;
    for (index, command) in commands.iter().enumerate() {
        println!(
            "script[{}]: {}",
            index + 1,
            play_script_command_label(command)
        );
        let disposition = handle_play_script_command(state, command, game_dir)?;
        print_play_script_snapshot(state, tile_atlas)?;
        if disposition == PlayInputDisposition::Quit {
            break;
        }
    }
    Ok(())
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
    for (index, command) in commands.iter().enumerate() {
        let disposition = handle_play_script_command(state, command, game_dir)?;
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
        transcribe_cell_frame_for_plain_text(&state.render_text_window_frame(None))
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
        transcribe_cell_frame_for_plain_text(&state.render_text_window_frame(None))
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
    } else if state
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
    let command = command.trim();
    if is_typeahead_toggle_token(command) {
        return handle_play_key_input(state, PLAY_TYPEAHEAD_TOGGLE_KEY, "", game_dir);
    }
    if is_music_toggle_token(command) {
        return handle_play_key_input(state, PLAY_MUSIC_TOGGLE_KEY, "", game_dir);
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
    if let Some(count) = play_script_idle_tick_count(command)? {
        for _ in 0..count {
            if handle_play_key_input(state, '.', "", game_dir)? == PlayInputDisposition::Quit {
                return Ok(PlayInputDisposition::Quit);
            }
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if play_state_typeahead_setting_in_force(state) {
        if let Some(keys) = play_input_typeahead_chars(command) {
            for key in keys {
                if !play_state_honours_typeahead_queue(state) {
                    break;
                }
                settle_play_script_exploration_gate(state, game_dir)?;
                if handle_play_key_input(state, key, "", game_dir)? == PlayInputDisposition::Quit {
                    return Ok(PlayInputDisposition::Quit);
                }
            }
            return Ok(PlayInputDisposition::Continue);
        }
    }
    let Some((key, suffix)) = play_input_key_and_suffix(command) else {
        handle_empty_play_input(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    };
    handle_play_key_input(state, key, &suffix, game_dir)
}

fn settle_play_script_exploration_gate(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if !play_state_accepts_typeahead(state) {
        return Ok(());
    }
    for _ in 0..PLAY_SCRIPT_MAX_IDLE_TICKS {
        match state.apply_exploration_turn_gate(game_dir)? {
            ExplorationTurnGateOutcome::Ready { .. } => return Ok(()),
            ExplorationTurnGateOutcome::Slept { .. }
            | ExplorationTurnGateOutcome::Rescued { .. } => {}
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
