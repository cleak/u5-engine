//! Print the message window's *inputs*, not its pixels.
//!
//! A window that renders two rows too high is a layout question with
//! three possible causes: the log holds the wrong lines, the clear
//! cursor's top offset is wrong, or the live-row reservation is. A
//! screenshot cannot tell those apart, and `--save-screen` plus a glyph
//! decode answers only "what came out". This replays a `--play-script`
//! command list and prints what `layout_message_window_*` is handed -
//! every log line with its kind, the top offset, the open prompt and
//! the live row - beside the rows the layout produces.

use std::path::Path;
use u5_runtime::*;
use u5_tui::{replay_play_script_commands, split_play_script};

fn keep(text: &str) -> Option<String> {
    (!text.trim().is_empty()).then(|| text.to_string())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: message_window_probe <PROFILE_DIR> [SCRIPT]");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    if let Some(script) = args.next() {
        let commands = split_play_script(&script);
        replay_play_script_commands(&mut state, dir, &commands, |_, _, _| Ok(())).expect("replay");
    }

    let mut log = message_log_from_entries(state.message_entries(), keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state
        .message_slot_needs_flush()
        .then(|| keep(&state.message))
        .flatten()
    {
        log.push_output(&text);
    }
    let open_prompt = state.open_prompt_line();
    let spell_echo = state.typed_prompt_echo();
    let live_row = if state.message_window_live_row_suppressed() {
        None
    } else {
        Some(spell_echo.as_deref().unwrap_or(""))
    };
    let live_row_kind = if spell_echo.is_some() {
        LiveRowKind::Continuation
    } else {
        LiveRowKind::CommandRow
    };

    println!("top_offset {}", log.top_offset());
    println!("open_prompt {open_prompt:?}");
    println!("live_row {live_row:?} kind {live_row_kind:?}");
    println!(
        "slot {:?} needs_flush {}",
        state.message,
        state.message_slot_needs_flush()
    );
    println!("log ({} lines):", log.lines().len());
    for (index, line) in log.lines().iter().enumerate() {
        println!("  {index:>2} {:?} {:?}", line.kind, line.text);
    }
    let layout = layout_message_window_with_continuation(
        &log,
        live_row,
        open_prompt.as_deref(),
        combat_prompt_row_follows_history(&state),
        live_row_kind,
    );
    println!("rows:");
    for row in &layout.rows {
        println!("  row {:>2} {:?}", row.row, row.text);
    }
    println!("inline cursor {:?}", layout.inline_cursor);
}
