//! Terminal-mode shell for the Ultima V runtime.
//!
//! Owns CLI argument parsing, the interactive play loop, terminal rendering,
//! and the play-script command harness. Game logic lives in `u5-runtime`.

pub mod audio_suite;
pub mod cli;
pub mod enhancement_settings;
pub mod equivalence;
pub mod intro_loop;
pub mod play_loop;
pub mod route_smoke;
pub mod runtime_game_dir;
pub mod save_frame;
pub mod setup;
pub mod visual_manifest;

pub use audio_suite::{AudioSuiteCase, audio_suite_cases, run_audio_suite};
pub use cli::{
    CLI_USAGE, CliArgs, CliCommand, CreateCharacterCommand, chargen_typed_name, cli_usage,
    parse_chargen_gender_arg, parse_chargen_winners_arg, parse_cli_args, parse_cli_command,
    parse_pending_vehicle_arg, parse_start_arg, parse_time_arg, parse_transport_arg,
    run_create_character_command, run_interactive_create_character, split_play_script,
};
#[cfg(feature = "visual")]
pub use enhancement_settings::visual_launch_config;
pub use enhancement_settings::{
    PARITY_LOCK_ENV, PARITY_LOCK_LINE, SettingsFile, launch_enhancements, run_show_settings,
};
pub use equivalence::{
    FeatureHooks, MatrixRow, Mismatch, ReplayHooks, RunRecord, TerminalFeature, TokenEvent,
    TraceStep, check_equivalence, compare_runs, parse_matrix, plan_configurations,
    run_equivalence_suite, run_trace, terminal_features,
};
pub use intro_loop::run_intro_menu_loop;
pub use play_loop::{
    ENHANCEMENT_TOKEN_PREFIX, FreeTextLineInput, NoScriptTokenHooks, ScriptTokenHooks,
    ScriptTokenIntercept, TerminalSaveWatch, ansi_function_key, ansi_navigation_key,
    complete_headless_blocking_presentations, free_text_line_input, handle_empty_play_input,
    handle_play_script_command, is_enhancement_token, is_music_toggle_token,
    is_simple_typeahead_key, is_typeahead_toggle_token, key_flushed_typeahead,
    play_input_key_and_suffix, play_input_typeahead_chars, play_script_command_label,
    play_script_idle_tick_count, play_script_state_line, play_state_accepts_typeahead,
    play_state_runs_input_wait_pass, play_state_whole_line_reader, print_play_frame,
    print_play_script_snapshot, raster_diagnostic_line, raster_frame_kind,
    replay_play_script_commands, replay_play_script_commands_hooked, run_interactive_play_loop,
    run_interactive_play_loop_watched, run_play_loop, run_play_loop_with, run_play_script_commands,
    run_play_script_commands_watched, run_play_script_step, run_terminal_input_wait_pass,
    unclassified_escape_sequence,
};
pub use route_smoke::{
    NoRouteTrace, RouteSmokeCase, RouteSmokeExpectation, RouteSmokeFrameReport, RouteSmokeReport,
    RouteTraceObserver, route_smoke_cases, run_route_smoke, run_route_smoke_case,
    run_route_smoke_case_traced, write_route_smoke_manifest,
};
pub use runtime_game_dir::{RUNTIME_DIR_ENV, prepare_writable_game_dir};
pub use save_frame::{
    SavedFrameReport, compose_gameplay_screen, run_save_frame, run_save_frame_suite,
    run_save_screen, save_frame_suite,
};
#[cfg(feature = "visual")]
pub use setup::{
    DesktopWizardHost, report_gui_error, run_default_launch, run_gui_main, save_wizard_settings,
    wizard_settings,
};
pub use setup::{
    SETTINGS_WINDOW_FLAG, SETUP_WINDOW_FLAG, WizardMode, describe_failure, expand_path,
    no_data_message, open_settings_page, resolve_game_data, run_settings_window_child,
    run_setup_window_child, run_terminal_wizard,
};
pub use u5_host_desktop::{ReadyGameData, WizardNeed};
pub use visual_manifest::{ManifestCompareReport, compare_manifest_files, compare_manifest_text};
