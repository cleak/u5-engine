//! Terminal play loop, rendering, input parsing, and script harness.
//!
//! Moved out of `u5-runtime` -- these helpers are TUI-only. Game logic
//! and the cross-shell input dispatcher (`handle_play_key_input`,
//! `PlayInputDisposition`) stay in `u5-runtime`.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::path::Path;

use u5_runtime::{
    Direction, PLAY_IGNORED_INPUT_KEY, PLAY_MUSIC_TOGGLE_KEY, PLAY_SCRIPT_MAX_IDLE_TICKS,
    PLAY_TYPEAHEAD_TOGGLE_KEY, PlayInputDisposition, PlayOptions, PlayState, TileAtlas,
    TileGraphicsDepth, handle_play_key_input, hash_bytes, hash_palette_indices, load_tile_atlas,
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
    println!("Ultima V first-playable slice");
    println!(
        "Scene {} floor/level {}. Town/world move: numpad 1-9 or lowercase wasd/yubn. Dungeon: W/S forward/back, A/D turn. Attack: A prompts or A+dir. Enter: e. Open: O prompts in town or o+dir. Get/Search: G/S prompt or +dir. Push: P prompts in town or p+dir. Hole up: h+hours. Look: l; fountain drink lY/lN or l2Y. View: v. Use: U opens picker or UT/UG/UK/U1-U8. Stats: Z. Ignite: i. Talk: T prompts or TKEYWORD. Climb: k/< />. Board/Xit/Yell sails: B/X/x/Y. Fire: F prompts on ships or f+dir. Cast: C opens spell-name and follow-up prompts, or inline C1IL/C1AZ2/C1AN2/C1M2/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS/C1LV/C1HR/C1IP6/C1IQW/C1AWY/C1PU/C1DP/C1AG6/C1GIN6/C1GIS6/C1AEP/C1EIP/C1PRV2/C1AT. Mix: M opens mixer or MIL/0x80/1. Order: N opens prompt or N12. Yell: Y opens prompt or YWORD. Top-down save: Q prompts or QY/QN. Dungeon exit prompt: Q prompts or QY/QN. Buffer/typeahead: buffer. Combat music toggle: music. Idle animation: . Optional startup wind/Grapple/transport/raster diagnostics: --wind, --grapple, --climbing-gear, --transport, --raster-diagnostics, --raster-depth ega|cga. Pass: Space/Enter. Harness quit: q.",
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
        print_play_frame(&mut state, tile_atlas.as_ref())?;
        let (key, suffix) = if let Some(key) = queued_input.pop_front() {
            (key, String::new())
        } else {
            print!("> ");
            io::stdout().flush()?;
            input.clear();
            if io::stdin().read_line(&mut input)? == 0 {
                break;
            }
            if state.typeahead_buffer_enabled {
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

pub fn print_play_frame(state: &mut PlayState, tile_atlas: Option<&TileAtlas>) -> io::Result<()> {
    println!();
    println!("{}", state.render_text_frame(5));
    println!("{}", state.render_stats_panel_frame());
    if let Some(atlas) = tile_atlas {
        println!("{}", raster_diagnostic_line(state, 5, atlas)?);
    }
    Ok(())
}

pub fn print_play_script_snapshot(
    state: &mut PlayState,
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    println!("{}", play_script_state_line(state));
    println!("{}", state.render_stats_panel_frame());
    if let Some(atlas) = tile_atlas {
        println!("{}", raster_diagnostic_line(state, 5, atlas)?);
    }
    Ok(())
}

pub fn raster_diagnostic_line(
    state: &mut PlayState,
    radius: usize,
    atlas: &TileAtlas,
) -> io::Result<String> {
    let Some(viewport) = state.render_top_down_frame(radius, atlas)? else {
        return Ok("Raster viewport: unavailable for dungeon mode.".to_string());
    };
    Ok(format!(
        "Raster viewport: {}x{} px, {}x{} cells, {}, hash {:016x}.",
        viewport.width,
        viewport.height,
        viewport.cells_wide,
        viewport.cells_high,
        viewport.depth.label(),
        hash_palette_indices(&viewport.pixels)
    ))
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
    if ansi_function_key(input).is_some() {
        return Some((PLAY_IGNORED_INPUT_KEY, "function".to_string()));
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
    match input {
        "\x1b[A" | "\x1bOA" => Some('8'),
        "\x1b[B" | "\x1bOB" => Some('2'),
        "\x1b[D" | "\x1bOD" => Some('4'),
        "\x1b[C" | "\x1bOC" => Some('6'),
        "\x1b[H" | "\x1bOH" | "\x1b[1~" | "\x1b[7~" => Some('7'),
        "\x1b[5~" => Some('9'),
        "\x1b[F" | "\x1bOF" | "\x1b[4~" | "\x1b[8~" => Some('1'),
        "\x1b[6~" => Some('3'),
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
    if state.pending_moongate.is_some() {
        state.resolve_moongate_prompt('\n', game_dir)?;
    } else if state
        .resolve_current_dungeon_room_trigger(Some(game_dir))?
        .is_none()
    {
        state.pass_turn_with_game_dir(Some(game_dir))?;
    }
    Ok(())
}

pub fn handle_play_script_command(
    state: &mut PlayState,
    command: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let command = command.trim();
    if is_typeahead_toggle_token(command) {
        return handle_play_key_input(state, PLAY_TYPEAHEAD_TOGGLE_KEY, "", game_dir);
    }
    if is_music_toggle_token(command) {
        return handle_play_key_input(state, PLAY_MUSIC_TOGGLE_KEY, "", game_dir);
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
    if state.typeahead_buffer_enabled {
        if let Some(keys) = play_input_typeahead_chars(command) {
            for key in keys {
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
