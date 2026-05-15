//! Terminal-mode shell for the Ultima V runtime.
//!
//! Owns CLI argument parsing, the interactive play loop, terminal rendering,
//! and the play-script command harness. Game logic lives in `u5-runtime`.

pub mod cli;
pub mod play_loop;
pub mod save_frame;

pub use cli::{
    CLI_USAGE, CliArgs, CreateCharacterCommand, parse_chargen_gender_arg,
    parse_chargen_winners_arg, parse_cli_args, parse_pending_vehicle_arg, parse_start_arg,
    parse_time_arg, parse_transport_arg, run_create_character_command, split_play_script,
};
pub use play_loop::{
    ansi_function_key, ansi_navigation_key, handle_empty_play_input, handle_play_script_command,
    is_simple_typeahead_key, is_typeahead_toggle_token, play_input_key_and_suffix,
    play_input_typeahead_chars, play_script_command_label, play_script_idle_tick_count,
    play_script_state_line, print_play_frame, print_play_script_snapshot, raster_diagnostic_line,
    run_play_loop, run_play_script_commands, unclassified_escape_sequence,
};
pub use save_frame::run_save_frame;
