//! Terminal-mode shell for the Ultima V runtime.
//!
//! Owns CLI argument parsing, the interactive play loop, terminal rendering,
//! and the play-script command harness. Game logic lives in `u5-runtime`.

pub mod audio_suite;
pub mod cli;
pub mod intro_loop;
pub mod play_loop;
pub mod route_smoke;
pub mod runtime_game_dir;
pub mod save_frame;
pub mod visual_manifest;

pub use audio_suite::{AudioSuiteCase, audio_suite_cases, run_audio_suite};
pub use cli::{
    CLI_USAGE, CliArgs, CreateCharacterCommand, parse_chargen_gender_arg,
    parse_chargen_winners_arg, parse_cli_args, parse_pending_vehicle_arg, parse_start_arg,
    parse_time_arg, parse_transport_arg, run_create_character_command,
    run_interactive_create_character, split_play_script,
};
pub use intro_loop::run_intro_menu_loop;
pub use play_loop::{
    ansi_function_key, ansi_navigation_key, complete_headless_blocking_presentations,
    handle_empty_play_input, handle_play_script_command, is_music_toggle_token,
    is_simple_typeahead_key, is_typeahead_toggle_token, play_input_key_and_suffix,
    play_input_typeahead_chars, play_script_command_label, play_script_idle_tick_count,
    play_script_state_line, play_state_accepts_typeahead, print_play_frame,
    print_play_script_snapshot, raster_diagnostic_line, raster_frame_kind,
    replay_play_script_commands, run_play_loop, run_play_script_commands,
    unclassified_escape_sequence,
};
pub use route_smoke::{
    RouteSmokeCase, RouteSmokeExpectation, RouteSmokeFrameReport, RouteSmokeReport,
    route_smoke_cases, run_route_smoke, run_route_smoke_case, write_route_smoke_manifest,
};
pub use runtime_game_dir::{RUNTIME_DIR_ENV, prepare_writable_game_dir};
pub use save_frame::{
    SavedFrameReport, compose_gameplay_screen, run_save_frame, run_save_frame_suite,
    run_save_screen, save_frame_suite,
};
pub use visual_manifest::{ManifestCompareReport, compare_manifest_files, compare_manifest_text};
