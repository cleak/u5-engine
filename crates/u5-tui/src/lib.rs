//! Terminal-mode shell for the Ultima V runtime.
//!
//! Owns CLI argument parsing, the interactive play loop, terminal rendering,
//! and the play-script command harness. Game logic lives in `u5-runtime`.

pub mod play_loop;

pub use play_loop::{
    ansi_function_key, ansi_navigation_key, handle_empty_play_input, handle_play_script_command,
    is_simple_typeahead_key, is_typeahead_toggle_token, play_input_key_and_suffix,
    play_input_typeahead_chars, play_script_command_label, play_script_idle_tick_count,
    play_script_state_line, print_play_frame, print_play_script_snapshot, raster_diagnostic_line,
    run_play_loop, run_play_script_commands, unclassified_escape_sequence,
};
