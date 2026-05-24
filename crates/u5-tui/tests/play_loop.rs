//! Tests for `u5_tui::play_loop`.
//!
//! Moved from `u5-runtime` when the play loop and its helpers were
//! lifted into the TUI crate. Tests that exercise both runtime and
//! TUI symbols live here because the TUI symbols are no longer in
//! `u5-runtime`'s scope.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use u5_runtime::shop_runtime::ReagentShopState;
use u5_runtime::shop_session::ActiveShopSession;
use u5_runtime::test_fixtures::*;
use u5_runtime::*;
use u5_tui::*;

// ---- moved test bodies ----

// from chunk_14
#[test]
fn raster_diagnostic_line_reports_hash_without_pixels() {
    let mut state = test_state(open_grid(), 1, 1);
    state.visibility_dirty = true;
    let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
    let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();
    let expected_hash = hash_palette_indices(&viewport.pixels);
    state.visibility_dirty = true;

    let line = raster_diagnostic_line(&mut state, 1, &atlas).unwrap();

    assert!(line.contains("Raster tile viewport"));
    assert!(line.contains("48x48 px"));
    assert!(line.contains("3x3 cells"));
    assert!(line.contains("EGA tile atlas"));
    assert!(line.contains(&format!("{expected_hash:016x}")));
    assert!(!state.visibility_dirty);
}

// from chunk_14
#[test]
fn raster_diagnostic_line_reports_selected_cga_depth() {
    let mut state = test_state(open_grid(), 1, 1);
    let atlas = synthetic_tile_atlas(TileGraphicsDepth::Cga4);

    let line = raster_diagnostic_line(&mut state, 1, &atlas).unwrap();

    assert!(line.contains("CGA tile atlas"));
}

#[test]
fn raster_diagnostic_line_reports_dungeon_raster_and_clears_dirty() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.torch_counter = 9;
    state.visibility_dirty = true;
    let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
    let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();
    let expected_hash = hash_palette_indices(&viewport.pixels);
    state.visibility_dirty = true;

    let line = raster_diagnostic_line(&mut state, 1, &atlas).unwrap();

    assert!(line.contains("Raster dungeon first-person viewport"));
    assert!(line.contains("48x48 px"));
    assert!(line.contains("3x3 cells"));
    assert!(line.contains("EGA tile atlas"));
    assert!(line.contains(&format!("{expected_hash:016x}")));
    assert!(!state.visibility_dirty);
}

#[test]
fn raster_frame_kind_names_dungeon_combat_and_tile_modes() {
    let town = test_state(open_grid(), 1, 1);
    assert_eq!(raster_frame_kind(&town), "tile viewport");

    let dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    assert_eq!(raster_frame_kind(&dungeon), "dungeon first-person viewport");

    let mut combat = test_state(open_grid(), 1, 1);
    combat.combat_active = true;
    assert_eq!(raster_frame_kind(&combat), "combat viewport");

    let mut overlay = test_state(open_grid(), 1, 1);
    overlay.gems = 1;
    overlay.view_gem();
    assert_eq!(raster_frame_kind(&overlay), "view overlay");
}

// from chunk_16
#[test]
fn play_input_preserves_space_command_and_suffix() {
    assert_eq!(play_input_key_and_suffix("\n"), None);
    assert_eq!(play_input_key_and_suffix("\r\n"), None);
    assert_eq!(play_input_key_and_suffix(" \n"), Some((' ', String::new())));
    assert_eq!(
        play_input_key_and_suffix("f4\r\n"),
        Some(('f', "4".to_string()))
    );
    assert_eq!(
        play_input_key_and_suffix(" f\n"),
        Some((' ', "f".to_string()))
    );
}

// from chunk_18
#[test]
fn play_input_translates_common_ansi_navigation_sequences() {
    assert_eq!(
        play_input_key_and_suffix("\x1b[A\n"),
        Some((char::from(INPUT_CODE_NORTH), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[B\r\n"),
        Some((char::from(INPUT_CODE_SOUTH), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[D\n"),
        Some((char::from(INPUT_CODE_WEST), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[C\n"),
        Some((char::from(INPUT_CODE_EAST), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[H\n"),
        Some((char::from(INPUT_CODE_NORTHWEST), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[5~\n"),
        Some((char::from(INPUT_CODE_NORTHEAST), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[F\n"),
        Some((char::from(INPUT_CODE_SOUTHWEST), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[6~\n"),
        Some((char::from(INPUT_CODE_SOUTHEAST), String::new()))
    );
}

// from chunk_18
#[test]
fn translated_ansi_navigation_routes_through_play_movement() {
    let (key, suffix) = play_input_key_and_suffix("\x1b[A\n").unwrap();
    let mut town = test_state(open_grid(), 5, 5);

    assert_eq!(
        handle_play_key_input(&mut town, key, &suffix, Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((town.player.x, town.player.y), (5, 4));
    assert_eq!(town.turn, 1);

    let (key, suffix) = play_input_key_and_suffix("\x1b[A\n").unwrap();
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

    assert_eq!(
        handle_play_key_input(&mut dungeon, key, &suffix, Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((dungeon.player.x, dungeon.player.y), (2, 1));
    assert_eq!(dungeon.player.facing, Direction::East);
    assert_eq!(dungeon.turn, 1);
}

// from chunk_18
#[test]
fn typeahead_buffer_toggle_is_no_turn_and_visible_in_status() {
    let mut state = test_state(open_grid(), 1, 1);

    assert_eq!(
        handle_play_script_command(&mut state, "buffer", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.typeahead_buffer_enabled);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Buffer On.");
    assert!(play_script_state_line(&state).contains("typeahead on"));
    assert!(state.z_stats_message().contains("typeahead on"));

    assert_eq!(
        handle_play_script_command(&mut state, "typeahead", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(!state.typeahead_buffer_enabled);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Buffer Off.");
}

// from chunk_18
#[test]
fn music_toggle_token_maps_to_ctrl_s_and_is_visible_in_status() {
    let mut state = test_state(open_grid(), 1, 1);

    assert_eq!(
        play_input_key_and_suffix("music\n"),
        Some((PLAY_MUSIC_TOGGLE_KEY, String::new()))
    );
    assert_eq!(play_input_typeahead_chars("music\n"), None);
    assert_eq!(
        handle_play_script_command(&mut state, "music", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(!state.music_enabled);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Music Off.");
    assert!(play_script_state_line(&state).contains("music off"));
    assert!(state.z_stats_message().contains("music off"));

    assert_eq!(
        handle_play_script_command(&mut state, "ctrl-s", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.music_enabled);
    assert_eq!(state.message, "Music On.");
}

// from chunk_18
#[test]
fn typeahead_input_parser_only_splits_simple_keys() {
    assert_eq!(
        play_input_typeahead_chars("dd.\n"),
        Some(vec!['d', 'd', '.'])
    );
    assert_eq!(
        play_input_typeahead_chars("d d\n"),
        Some(vec!['d', ' ', 'd'])
    );
    assert_eq!(play_input_typeahead_chars("d"), None);
    assert_eq!(play_input_typeahead_chars("TJOB"), None);
    assert_eq!(play_input_typeahead_chars("C1IL"), None);
    assert_eq!(
        play_input_key_and_suffix("buffer\n"),
        Some((PLAY_TYPEAHEAD_TOGGLE_KEY, String::new()))
    );
}

#[test]
fn typeahead_buffer_pauses_during_modal_prompts() {
    let mut state = world_state(open_world_grid(), 4, 5);
    assert!(play_state_accepts_typeahead(&state));

    state.typeahead_buffer_enabled = true;
    state.pending_moongate = Some(MoongateEntry {
        x: 4,
        y: 5,
        destination_plane: WorldPlane::Britannia,
        destination_x: 6,
        destination_y: 7,
        active_hours: None,
        expected_tile: None,
    });

    assert_eq!(play_input_typeahead_chars("12"), Some(vec!['1', '2']));
    assert!(!play_state_accepts_typeahead(&state));
    assert_eq!(
        handle_play_script_command(&mut state, "12", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.turn, 0);
    assert_eq!((state.player.x, state.player.y), (4, 5));
    assert!(state.pending_moongate.is_some());
    assert_eq!(state.message, "Enter moongate? (Y/N).");
}

#[test]
fn typeahead_buffer_preserves_multi_digit_shop_quantity_lines() {
    let mut state = test_state(open_grid(), 1, 1);
    state.typeahead_buffer_enabled = true;
    state.gold = 100;
    state.reagents = [0; REAGENT_COUNT];
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
        Herbalist::Mysticism,
    )));

    assert_eq!(
        handle_play_script_command(&mut state, "A", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(matches!(
        state.active_shop.as_ref(),
        Some(ActiveShopSession::Reagent(
            ReagentShopState::PickQuantity { .. }
        ))
    ));
    assert!(!play_state_accepts_typeahead(&state));
    assert_eq!(play_input_typeahead_chars("12"), Some(vec!['1', '2']));

    assert_eq!(
        handle_play_script_command(&mut state, "12", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 12);
    assert_eq!(state.gold, 28);
    assert!(state.message.contains("72 gold"));
}

// from chunk_18
#[test]
fn ansi_function_keys_are_ignored_before_command_dispatch() {
    assert_eq!(ansi_function_key("\x1bOP"), Some(1));
    assert_eq!(ansi_function_key("\x1b[21~"), Some(10));
    assert_eq!(
        play_input_key_and_suffix("\x1bOP\n"),
        Some((char::from(INPUT_CODE_F1), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[21~\n"),
        Some((char::from(INPUT_CODE_F10), String::new()))
    );
    assert_eq!(
        play_input_key_and_suffix("\x1b[A\n"),
        Some((char::from(INPUT_CODE_NORTH), String::new()))
    );
}

// from chunk_18
#[test]
fn function_key_input_is_no_turn_and_no_idle_tick() {
    let (key, suffix) = play_input_key_and_suffix("\x1bOP\n").unwrap();
    let mut town = test_state(open_grid(), 5, 5);

    assert_eq!(
        handle_play_key_input(&mut town, key, &suffix, Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((town.player.x, town.player.y), (5, 5));
    assert_eq!(town.turn, 0);
    assert_eq!(town.animation.frame, 0);
    assert_eq!(town.message, "Function key ignored.");

    let (key, suffix) = play_input_key_and_suffix("\x1b[21~\n").unwrap();
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

    assert_eq!(
        handle_play_key_input(&mut dungeon, key, &suffix, Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((dungeon.player.x, dungeon.player.y), (1, 1));
    assert_eq!(dungeon.turn, 0);
    assert_eq!(dungeon.animation.frame, 0);
    assert_eq!(dungeon.message, "Function key ignored.");
}

// from chunk_18
#[test]
fn unclassified_escape_sequences_are_ignored_without_swallowing_escape_key() {
    assert_eq!(
        play_input_key_and_suffix("\x1b[99~\n"),
        Some((PLAY_IGNORED_INPUT_KEY, "escape".to_string()))
    );
    assert_eq!(play_input_typeahead_chars("\x1b[99~\n"), None);
    assert_eq!(
        play_input_key_and_suffix("\x1b\n"),
        Some(('\x1b', String::new()))
    );
}

// from chunk_19
#[test]
fn unclassified_escape_input_is_no_turn_and_no_idle_tick() {
    let (key, suffix) = play_input_key_and_suffix("\x1b[99~\n").unwrap();
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);

    assert_eq!(
        handle_play_key_input(&mut dungeon, key, &suffix, Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((dungeon.player.x, dungeon.player.y), (1, 1));
    assert_eq!(dungeon.turn, 0);
    assert_eq!(dungeon.animation.frame, 0);
    assert_eq!(dungeon.message, "Input ignored.");
}

// from chunk_19
#[test]
fn play_script_typeahead_replays_simple_movement_queue() {
    let mut state = test_state(open_grid(), 1, 1);

    assert_eq!(
        handle_play_script_command(&mut state, "buffer", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        handle_play_script_command(&mut state, "dd.", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (3, 1));
    assert_eq!(state.turn, 2);
    assert!(state.typeahead_buffer_enabled);
    assert_eq!(state.message, "Idle animation tick.");
}

// from chunk_19
#[test]
fn play_script_command_label_sanitizes_control_sequences() {
    assert_eq!(play_script_command_label(""), "empty");
    assert_eq!(play_script_command_label("\x1bOP"), "F1");
    assert_eq!(play_script_command_label("\x1b[21~"), "F10");
    assert_eq!(play_script_command_label("d\x01."), "d\\x01.");
}

// from chunk_19
#[test]
fn pending_prompt_consumes_typeahead_toggle_without_changing_buffer_state() {
    let mut prompted = world_state(open_world_grid(), 4, 5);
    prompted.pending_moongate = Some(MoongateEntry {
        x: 4,
        y: 5,
        destination_plane: WorldPlane::Britannia,
        destination_x: 6,
        destination_y: 7,
        active_hours: None,
        expected_tile: None,
    });

    assert_eq!(
        handle_play_script_command(&mut prompted, "buffer", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(!prompted.typeahead_buffer_enabled);
    assert_eq!(prompted.turn, 0);
    assert_eq!(prompted.animation.frame, 0);
    assert!(prompted.pending_moongate.is_some());
    assert_eq!(prompted.message, "Enter moongate? (Y/N).");
}

// from chunk_19
#[test]
fn empty_play_input_repeats_pending_moongate_prompt_without_turn() {
    let mut prompted = world_state(open_world_grid(), 4, 5);
    prompted.pending_moongate = Some(MoongateEntry {
        x: 4,
        y: 5,
        destination_plane: WorldPlane::Britannia,
        destination_x: 6,
        destination_y: 7,
        active_hours: None,
        expected_tile: None,
    });

    handle_empty_play_input(&mut prompted, Path::new("")).unwrap();

    assert_eq!(prompted.turn, 0);
    assert_eq!((prompted.player.x, prompted.player.y), (4, 5));
    assert!(prompted.pending_moongate.is_some());
    assert_eq!(prompted.message, "Enter moongate? (Y/N).");

    let mut unprompted = test_state(open_grid(), 1, 1);

    handle_empty_play_input(&mut unprompted, Path::new("")).unwrap();

    assert_eq!(unprompted.turn, 1);
    assert_eq!(unprompted.message, "Passed.");
}

#[test]
fn empty_play_input_submits_modal_prompt_enter_without_passing_turn() {
    let mut prompted = test_state(open_grid(), 1, 1);
    assert_eq!(prompted.start_yell_prompt(), MoveOutcome::Observed);
    assert!(prompted.active_yell.is_some());
    assert!(!play_state_accepts_typeahead(&prompted));

    handle_empty_play_input(&mut prompted, Path::new("")).unwrap();

    assert_eq!(prompted.turn, 0);
    assert!(prompted.active_yell.is_none());
    assert_eq!(prompted.message, YELL_NOTHING_SAID_MESSAGE);
}

// from chunk_19
#[test]
fn empty_play_input_resolves_dungeon_room_trigger_before_pass() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0xf3;
    let mut state = dungeon_state(grid, 0, 1, 1);

    handle_empty_play_input(&mut state, Path::new("")).unwrap();

    assert_eq!(state.turn, 1);
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa3);
    assert!(
        state
            .message
            .contains("Entered dungeon room trigger slot 3")
    );
    assert!(!state.message.contains("Passed"));
}

// from chunk_19
#[test]
fn play_script_command_routes_movement_pass_idle_and_quit() {
    let mut state = test_state(open_grid(), 1, 1);

    assert_eq!(
        handle_play_script_command(&mut state, "d", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);

    assert_eq!(
        handle_play_script_command(&mut state, "empty", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.turn, 2);
    assert_eq!(state.message, "Passed.");

    assert_eq!(
        handle_play_script_command(&mut state, "pass", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.turn, 3);

    assert_eq!(
        handle_play_script_command(&mut state, ".", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.turn, 3);
    assert_eq!(state.message, "Idle animation tick.");

    assert_eq!(
        handle_play_script_command(&mut state, "q", Path::new("")).unwrap(),
        PlayInputDisposition::Quit
    );
    assert_eq!(state.turn, 3);
}

#[test]
fn shared_script_replay_stops_after_quit_before_later_commands() {
    let mut state = test_state(open_grid(), 1, 1);
    let commands = vec!["q".to_string(), "d".to_string()];
    let mut observed = Vec::new();

    replay_play_script_commands(
        &mut state,
        Path::new(""),
        &commands,
        |state, index, command| {
            observed.push((index, command.to_string(), state.player.x, state.turn));
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(observed, vec![(0, "q".to_string(), 1, 0)]);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.turn, 0);
}

#[test]
fn every_published_spell_code_reaches_a_modeled_dispatch_branch() {
    for (index, code) in SPELL_CODES.iter().enumerate() {
        let mut state = if SPELL_SCENE_MASKS[index] & SPELL_SCENE_COMBAT != 0 {
            let mut state = world_state(open_world_grid(), 5, 5);
            state.combat_active = true;
            state
        } else if SPELL_SCENE_MASKS[index] & SPELL_SCENE_OVERWORLD != 0 {
            world_state(open_world_grid(), 5, 5)
        } else if SPELL_SCENE_MASKS[index] & SPELL_SCENE_INDOOR != 0 {
            test_state(open_grid(), 5, 5)
        } else {
            dungeon_state(open_dungeon_record(), 0, 1, 1)
        };

        state.cast_spell_from_suffix(code, Path::new("")).unwrap();

        assert_ne!(
            state.message, "No effect!",
            "{code} should have an explicit dispatcher branch"
        );
    }
}

// from chunk_19
#[test]
fn play_script_idle_count_replays_no_turn_visual_ticks() {
    let mut state = test_state(open_grid(), 1, 1);

    assert_eq!(
        handle_play_script_command(&mut state, "idle:3", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.turn, 0);
    assert_eq!(state.clock, GameClock::default());
    assert_eq!(state.animation.frame, 3);
    assert_eq!(state.message, "Idle animation tick.");

    assert_eq!(
        handle_play_script_command(&mut state, "tick", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    // Animation counter is now mod 12 (LCM of supported cycle lengths)
    // so it just increments to 4. The visible water cycle modulo 3 still
    // wraps cleanly on top of this.
    assert_eq!(state.animation.frame, 4);
    assert_eq!(state.turn, 0);
}

// from chunk_19
#[test]
fn play_script_idle_count_respects_pending_prompt_freeze() {
    let mut prompted = world_state(open_world_grid(), 4, 5);
    prompted.pending_moongate = Some(MoongateEntry {
        x: 4,
        y: 5,
        destination_plane: WorldPlane::Britannia,
        destination_x: 6,
        destination_y: 7,
        active_hours: None,
        expected_tile: None,
    });

    assert_eq!(
        handle_play_script_command(&mut prompted, "idle:2", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(prompted.turn, 0);
    assert_eq!(prompted.animation.frame, 0);
    assert!(prompted.pending_moongate.is_some());
    assert_eq!(prompted.message, "Enter moongate? (Y/N).");
}

// from chunk_19
#[test]
fn play_script_idle_count_rejects_bad_counts() {
    assert!(play_script_idle_tick_count("idle:0").is_err());
    assert!(play_script_idle_tick_count("idle:nope").is_err());
    assert!(play_script_idle_tick_count("idle:1025").is_err());
    assert_eq!(play_script_idle_tick_count("idle").unwrap(), Some(1));
    assert_eq!(play_script_idle_tick_count("tick:4").unwrap(), Some(4));
    assert_eq!(play_script_idle_tick_count("d").unwrap(), None);
}

// from chunk_19
#[test]
fn play_script_state_line_hashes_message_without_printing_it() {
    let mut state = test_state(open_grid(), 5, 6);
    state.message = "Talked to Ada: I mend gear".to_string();

    let line = play_script_state_line(&state);

    assert!(line.contains("State: CASTLE:0 floor 0 at (5, 6)"));
    assert!(line.contains("message-bytes 26 hash"));
    assert!(!line.contains("Ada"));
    assert!(!line.contains("mend gear"));
}

// from chunk_19
#[test]
fn play_script_local_clean_smoke_runs_default_scene_when_present() {
    let game_dir = Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join("CASTLE.DAT").exists() || !game_dir.join(TILES_EGA_FILE).exists() {
        return;
    }

    let mut state = PlayState::load_scene(game_dir, PlayOptions::default()).unwrap();
    let atlas = load_tile_atlas(game_dir, TileGraphicsDepth::Ega16).unwrap();
    let initial_message = state.message.clone();
    let initial_line = play_script_state_line(&state);

    assert!(initial_line.contains("State: CASTLE:0 floor 0"));
    assert!(initial_line.contains("message-bytes"));
    assert!(!initial_message.is_empty());
    assert!(!initial_line.contains(&initial_message));

    let raster_line = raster_diagnostic_line(&mut state, 5, &atlas).unwrap();
    assert!(raster_line.contains("EGA tile atlas"));
    assert!(raster_line.contains("hash "));

    assert_eq!(
        handle_play_script_command(&mut state, "empty", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.turn, 1);

    assert_eq!(
        handle_play_script_command(&mut state, ".", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.turn, 1);

    assert_eq!(
        handle_play_script_command(&mut state, "q", game_dir).unwrap(),
        PlayInputDisposition::Quit
    );
}

#[test]
fn route_smoke_cases_cover_representative_modes() {
    let cases = route_smoke_cases();

    assert_eq!(cases.len(), 493);
    assert!(cases.iter().any(|case| matches!(
        case.expected,
        RouteSmokeExpectation::World(WorldPlane::Britannia)
    )));
    assert!(cases.iter().any(|case| matches!(
        case.expected,
        RouteSmokeExpectation::World(WorldPlane::Underworld)
    )));
    assert!(
        cases
            .iter()
            .any(|case| matches!(case.expected, RouteSmokeExpectation::Town(_)))
    );
    assert!(
        cases
            .iter()
            .any(|case| matches!(case.expected, RouteSmokeExpectation::Dungeon(_)))
    );
    assert!(
        cases
            .iter()
            .any(|case| matches!(case.expected, RouteSmokeExpectation::Endgame(_)))
    );
    assert!(cases.iter().any(|case| case.name == "debug-enter-castle"));
    assert!(
        cases
            .iter()
            .any(|case| case.name == "debug-enter-castle-return-world")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "debug-enter-castle-from-underworld")
    );
    assert!(cases.iter().any(|case| case.name == "debug-enter-dungeon"));
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-save-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-view-overlay")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-spyglass-chunk-map")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-utility-use-items")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-create-food-cast")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-blink-east-ray")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-locate-cast")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-rel-hur-east")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-light-open-spell-route")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-restore-spell-suite")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-active-effect-spell-suite")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "ship-hms-cape-plans-use")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-chasm-fall-to-underworld")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-whirlpool-forced-underworld")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-fixed-narrative-gate-open-south-step")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-fixed-narrative-gate-ordained-block")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-hole-up-rest")
    );
    assert!(cases.iter().any(|case| case.name == "castle-save-refusal"));
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-dispatcher-refusals")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-dispatcher-refusals")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-party-overlay-routes")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-wooden-box-use")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "endgame-missing-box-confirmation")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "endgame-missing-box-terminal-jitter")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "endgame-box-victory-confirmation")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "endgame-box-full-victory-cinematic")
    );
    for name in [
        "castle-town-attack-death-mask-npc",
        "castle-town-attack-guard-alarm",
        "castle-town-hostile-adjacent-alarm",
        "castle-town-guard-arrest-refusal",
        "castle-town-guard-arrest-surrender-yew",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    assert!(
        cases
            .iter()
            .any(|case| case.name == "virtue-town-shadowlord-entry")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "virtue-town-shadowlord-yell")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "lycaeum-shard-falsehood-vanquish")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "empath-shard-hatred-vanquish")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "serpents-hold-shard-cowardice-vanquish")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "blackthorn-fixed-hidden-zero-key-search")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "stonegate-shadowlord-entry")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "britannia-word-of-power-seal-opens")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "underworld-doom-word-of-power-seal-opens")
    );
    assert!(cases.iter().any(|case| case.name == "castle-view-overlay"));
    assert!(cases.iter().any(|case| case.name == "castle-peer-overlay"));
    assert!(cases.iter().any(|case| case.name == "castle-x-ray-overlay"));
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-surface-fountain-look")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "buccaneers-den-wishing-well-horse")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "buccaneers-den-wishing-well-ferrari-grants-horse")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-death-vision-look")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "yew-wanted-poster-look")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-talk-status-sleeping-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-talk-status-praying-refusal")
    );
    for name in [
        "talk-towne-reserved-name",
        "talk-towne-08-reserved-thank",
        "talk-dwelling-reserved-job",
        "talk-dwelling-16-ordinary-no-match",
        "talk-castle-reserved-thank",
        "talk-castle-24-reserved-work",
        "talk-keep-ordinary-no-match",
        "talk-keep-32-reserved-bye",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    for name in [
        "shrine-native-honesty-meditation",
        "shrine-native-humility-meditation",
        "codex-urn-honesty-read",
        "shrine-honesty-codex-turn-in",
        "shrine-compassion-completed-offering",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-light-decay-route")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "castle-hourly-ring-regeneration-pass")
    );
    for name in [
        "reload-boarded-horse-pass",
        "reload-gate-travel-underworld-pass",
        "reload-chasm-underworld-pass",
        "reload-underworld-fixed-hidden-stack-search-get-search",
        "reload-minoc-fixed-hidden-daily-search-get-repeat",
        "reload-horse-trader-horse-and-rider-buy-pass",
        "reload-ship-xit-skiff-pass",
        "reload-dungeon-ladder-down-up-route",
        "reload-dungeon-surface-exit-return-world",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    assert!(
        cases
            .iter()
            .any(|case| case.name == "shop-inn-rest-accept-public-rate")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "shop-horse-trader-horse-and-rider-buy")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "shop-horse-trader-stablehouse-buy")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "shop-horse-trader-wishing-well-buy")
    );
    for name in [
        "shop-shipwright-island-frigate-buy",
        "shop-shipwright-crows-nest-skiff-buy",
        "shop-shipwright-oaken-oar-frigate-buy",
        "shop-shipwright-rusty-bucket-skiff-buy",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    assert!(
        cases
            .iter()
            .any(|case| case.name == "shop-sage-topic-paid-success-route")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "shop-sage-topic-short-funds-route")
    );
    for name in [
        "shop-tavern-honest-meal-lore-route",
        "shop-tavern-wayfarer-lore-route",
        "shop-tavern-sword-and-keg-lore-route",
        "shop-tavern-slaughtered-lamb-lore-route",
        "shop-tavern-humble-palate-lore-route",
        "shop-tavern-blue-boar-lore-route",
        "shop-tavern-cats-lair-lore-route",
        "shop-tavern-fallen-virgin-lore-route",
        "shop-tavern-folley-tap-lore-route",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-turn-and-blocked-step")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-level-up-down-spells")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-field-cycle-spells")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-open-chest-spell")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-attack-direction-route")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-active-monster-attack-ambush")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-active-monster-contact-ambush")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-search-focus-route")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-ladder-down-up-route")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-surface-exit-return-world")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-sjog-underfoot-routes")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-refusal-letter-routes")
    );
    assert!(cases.iter().any(|case| case.name == "dungeon-exit-confirm"));
    assert!(cases.iter().any(|case| case.name == "dungeon-view-overlay"));
    assert!(cases.iter().any(|case| case.name == "dungeon-hole-up-rest"));
    assert!(
        cases
            .iter()
            .any(|case| case.name == "underworld-pass-and-idle")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "ship-xit-launches-skiff")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "ship-hoist-and-sail-east")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-room-combat-trigger")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "terrain-combat-party-entry")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "dungeon-room-party-entry")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-view-label-only")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-look-label-only")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-pass-round")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-select-player-clear")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-select-player-one")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-select-player-six")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-escape-abort")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-music-toggle")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-direct-step-east")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-use-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-d-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-w-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-board-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-enter-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-fire-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-hole-up-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-ignite-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-mix-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-new-order-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-talk-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-attack-direction")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-cast-refusal")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "combat-directed-sleep-cone")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "combat-directed-poison-wind-cone")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "combat-directed-death-wind-cone")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "combat-directed-flame-wind-cone")
    );
    for name in [
        "combat-directed-sleep-cone-north",
        "combat-directed-sleep-cone-east",
        "combat-directed-sleep-cone-south",
        "combat-directed-sleep-cone-west",
        "combat-directed-poison-wind-cone-north",
        "combat-directed-poison-wind-cone-east",
        "combat-directed-poison-wind-cone-south",
        "combat-directed-poison-wind-cone-west",
        "combat-directed-death-wind-cone-north",
        "combat-directed-death-wind-cone-east",
        "combat-directed-death-wind-cone-south",
        "combat-directed-death-wind-cone-west",
        "combat-directed-flame-wind-cone-north",
        "combat-directed-flame-wind-cone-east",
        "combat-directed-flame-wind-cone-south",
        "combat-directed-flame-wind-cone-west",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    for name in [
        "combat-field-fire-marker-placement",
        "combat-field-poison-marker-placement",
        "combat-field-sleep-marker-placement",
        "combat-field-energy-marker-placement",
        "combat-magic-missile-target",
        "combat-fireball-target",
        "combat-reveal-hidden-target",
        "combat-invisibility-caster",
        "combat-cause-fear-target",
        "combat-mass-charm-effect",
        "combat-tremor-targets",
        "combat-repel-undead-targets",
        "combat-charm-target",
        "combat-polymorph-target",
        "combat-clone-target",
        "combat-conjure-animal",
        "combat-swarm-summon",
        "combat-summon-daemon-ring",
        "combat-kill-gazer-eye-burst",
        "combat-kill-gargoyle-lava-marker",
        "combat-kill-shadowlord-vanish-marker",
        "terrain-combat-party-entry",
        "dungeon-room-party-entry",
    ] {
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-get-direction")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-jimmy-direction")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-open-direction")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-push-direction")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-klimb-direction")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-ready-prompt")
    );
    assert!(cases.iter().any(|case| case.name == "doom-combat-z-stats"));
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-yell-word")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-xit-foes-remain")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-quit-defeat")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "terrain-combat-xit-no-foes-clean-exit")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "terrain-combat-out-of-arena-leave")
    );
    assert!(
        cases
            .iter()
            .any(|case| case.name == "doom-combat-search-prompt")
    );
    for row in 1..=published_world_location_entries().len() {
        let name = format!("stock-location-enter-{row:02}");
        assert!(cases.iter().any(|case| case.name == name), "{name}");
    }
}

#[test]
fn route_smoke_local_clean_cases_run_when_present() {
    let game_dir = Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join("CASTLE.DAT").exists() || !game_dir.join(TILES_EGA_FILE).exists() {
        return;
    }

    let atlas = load_tile_atlas(game_dir, TileGraphicsDepth::Ega16).unwrap();
    for case in route_smoke_cases() {
        let report = run_route_smoke_case(game_dir, &atlas, &case).unwrap();
        assert_eq!(report.name, case.name);
        assert_eq!(report.commands_run, case.script.len());
        assert!(report.final_state_line.contains("State:"));
        assert!(report.final_raster_line.contains(case.expected_frame_kind));
        assert!(report.final_raster_line.contains(" hash "));
        assert!(report.final_width > 0);
        assert!(report.final_height > 0);
        assert!(report.final_hash != 0);
    }
}

#[test]
fn route_smoke_manifest_is_sanitized_and_comparable() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "u5-route-smoke-manifest-test-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("manifest.txt");
    let reports = vec![RouteSmokeReport {
        name: "world\tmove".to_string(),
        commands_run: 2,
        final_state_line: "State:\tBRITANNIA\nhash 1234".to_string(),
        final_raster_line: "Raster tile viewport: 48x48 px, hash aaaaaaaaaaaaaaaa.".to_string(),
        final_frame_kind: "tile viewport".to_string(),
        final_width: 48,
        final_height: 48,
        final_hash: 0xaaaaaaaaaaaaaaaa,
        final_nonblack_pixels: 42,
    }];

    write_route_smoke_manifest(&path, &reports).unwrap();
    let manifest = fs::read_to_string(&path).unwrap();
    let _ = fs::remove_dir_all(&dir);

    assert!(manifest.contains("coverage\ttotal-routes\t1"));
    assert!(manifest.contains("route-world move\t48x48\ttile viewport"));
    assert!(manifest.contains("hash aaaaaaaaaaaaaaaa"));
    assert!(manifest.contains("nonblack 42"));
    assert!(!manifest.contains("world\tmove"));
    assert!(
        compare_manifest_text(&manifest, &manifest)
            .unwrap()
            .is_clean()
    );
}

#[test]
fn route_smoke_scripts_cover_every_published_spell_code() {
    let cases = route_smoke_cases();
    let mut covered = [false; SPELL_COUNT];

    for command in cases.iter().flat_map(|case| case.script.iter().copied()) {
        let Some(suffix) = command.strip_prefix('C') else {
            continue;
        };
        let code = u5_runtime::inline_parsers::inline_spell_code(suffix);
        if let Some(index) = spell_index_from_code(&code) {
            covered[index] = true;
        }
    }

    for (index, code) in SPELL_CODES.iter().enumerate() {
        assert!(covered[index], "route smoke scripts should cover {code}");
    }
}
