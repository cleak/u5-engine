//! Tests for `u5_tui::cli`.
//!
//! Moved from `u5-runtime` when CLI parsing was lifted into the TUI crate.

use std::fs;
use std::path::PathBuf;

use u5_runtime::test_fixtures::*;
use u5_runtime::*;
use u5_tui::*;

// ---- moved test bodies ----

// from chunk_02
#[test]
fn cli_parser_accepts_play_scene_floor_and_start() {
    let args = parse_cli_args([
        "--play",
        "--scene",
        "DWELLING:1",
        "--floor",
        "1",
        "--at",
        "5,11",
        "--time",
        "18:30",
        "--wind",
        "west",
        "--climbing-gear",
        "0x02",
        "--raster-diagnostics",
        "--raster-depth",
        "cga",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(args.play);
    assert!(args.raster_diagnostics);
    assert_eq!(args.raster_depth, TileGraphicsDepth::Cga4);
    assert_eq!(args.play_script, None);
    assert_eq!(
        args.play_options.target,
        PlayTarget::Town(Scene::new(10).unwrap())
    );
    assert_eq!(args.play_options.floor, 1);
    assert_eq!(args.play_options.start, Some((5, 11)));
    assert_eq!(args.play_options.clock, GameClock::new(18, 30).unwrap());
    assert_eq!(args.play_options.wind, WindState::West);
    assert_eq!(args.play_options.climbing_gear, 2);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

// from chunk_02
#[test]
fn cli_parser_accepts_pending_vehicle_acquisition() {
    let args = parse_cli_args([
        "--play",
        "--scene",
        "BRITANNIA",
        "--pending-vehicle",
        "frigate:10,20,3",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert_eq!(
        args.play_options.pending_vehicle,
        Some(PendingVehicleAcquisition::Frigate {
            x: 10,
            y: 20,
            skiffs: 3
        })
    );
    assert!(parse_pending_vehicle_arg("skiff:1,2").is_ok());
    assert!(parse_pending_vehicle_arg("balloon:1,2").is_err());
}

// from chunk_02
#[test]
fn cli_parser_accepts_world_start_coordinates() {
    let args = parse_cli_args([
        "--play",
        "--scene",
        "UNDERWORLD",
        "--debug-enter",
        "CASTLE:0",
        "--at",
        "200,201",
        "--transport",
        "balloon",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert_eq!(
        args.play_options.target,
        PlayTarget::World(WorldPlane::Underworld)
    );
    assert_eq!(
        args.play_options.debug_enter,
        Some(PlayTarget::Town(Scene::new(17).unwrap()))
    );
    assert_eq!(args.play_options.start, Some((200, 201)));
    assert_eq!(
        args.play_options.transport,
        TransportState::Balloon {
            type_byte: FIRST_PLAYABLE_BALLOON_TILE,
            tile: FIRST_PLAYABLE_BALLOON_TILE,
        }
    );
}

// from chunk_02
#[test]
fn cli_parser_can_seed_play_from_init_gam_without_chargen() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(13, 0, 15, 15)).unwrap();
    fs::write(dir.join("INIT.OOL"), vec![0; OOL_PLANE_LEN]).unwrap();

    let args = parse_cli_args([
        "--play",
        "--from-init",
        "--climbing-gear",
        "3",
        dir.to_str().unwrap(),
    ])
    .unwrap();

    assert_eq!(
        args.play_options.target,
        PlayTarget::Town(Scene::new(13).unwrap())
    );
    assert_eq!(args.play_options.floor, 0);
    assert_eq!(args.play_options.start, Some((15, 15)));
    assert_eq!(args.play_options.clock, GameClock::new(8, 35).unwrap());
    assert_eq!(args.play_options.climbing_gear, 3);
    assert!(args.play_options.initial_britannia_overlay.is_some());
    let _ = fs::remove_dir_all(dir);
}

// from chunk_02
#[test]
fn cli_wind_override_applies_after_save_load_and_preserves_raw_non_calm_byte() {
    let dir = debug_game_dir();
    let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
    save[SAVE_AVATAR_NAME_OFFSET] = b'A';
    save[SAVE_WIND_OFFSET] = 0x7a;
    fs::write(dir.join("SAVED.GAM"), save).unwrap();
    fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();

    let args = parse_cli_args([
        "--play",
        "--from-save",
        "--wind",
        "east",
        dir.to_str().unwrap(),
    ])
    .unwrap();
    assert_eq!(args.play_options.wind, WindState::East);
    assert_eq!(args.play_options.wind_save_byte, 0x7a);

    let calm = parse_cli_args([
        "--play",
        "--from-save",
        "--wind",
        "calm",
        dir.to_str().unwrap(),
    ])
    .unwrap();
    assert_eq!(calm.play_options.wind, WindState::Calm);
    assert_eq!(calm.play_options.wind_save_byte, 0);

    let _ = fs::remove_dir_all(dir);
}

// from chunk_02
#[test]
fn from_init_world_bootstrap_uses_init_ool_surface_overlay() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 10, 20)).unwrap();
    let init_object = ActiveObject {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
        x: 7,
        y: 8,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    fs::write(dir.join("INIT.OOL"), ool_plane_with_object(1, init_object)).unwrap();
    let stale_object = ActiveObject {
        type_byte: 170,
        tile: 170,
        x: 9,
        y: 10,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    let mut saved_ool = vec![0; SAVED_OOL_LEN];
    write_ool_object(&mut saved_ool[..OOL_PLANE_LEN], 1, stale_object);
    fs::write(dir.join("SAVED.OOL"), saved_ool).unwrap();
    fs::write(dir.join("BRIT.OOL"), ool_plane_with_object(1, stale_object)).unwrap();

    let args = parse_cli_args(["--play", "--from-init", dir.to_str().unwrap()]).unwrap();
    assert_eq!(
        args.play_options.target,
        PlayTarget::World(WorldPlane::Britannia)
    );
    let init_overlay = args
        .play_options
        .initial_britannia_overlay
        .as_ref()
        .unwrap();
    assert_eq!(init_overlay[0], init_object);

    let state =
        PlayState::load_world_scene(&dir, WorldPlane::Britannia, args.play_options).unwrap();

    assert_eq!(state.active_objects[1], init_object);
    assert_ne!(state.active_objects[1], stale_object);
    let _ = fs::remove_dir_all(dir);
}

// from chunk_02
#[test]
fn cli_parser_rejects_save_and_init_seed_conflict() {
    assert!(
        parse_cli_args(["--play", "--from-save", "--from-init", r"C:\Games\U5-Clean",])
            .is_err()
    );
}

// from chunk_02
#[test]
fn cli_parser_rejects_bad_raster_depth() {
    assert!(parse_cli_args(["--play", "--raster-depth", "hercules"]).is_err());
}

// from chunk_02
#[test]
fn split_play_script_trims_and_drops_blank_commands() {
    assert_eq!(
        split_play_script(" d ; empty ; ; C1IL ; q "),
        vec!["d", "empty", "C1IL", "q"]
    );
}

// from chunk_02
#[test]
fn cli_parser_accepts_play_script_and_implies_play_mode() {
    let args = parse_cli_args([
        "--play-script",
        "d;empty;.;q",
        "--raster-diagnostics",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(args.play);
    assert!(args.raster_diagnostics);
    assert_eq!(
        args.play_script,
        Some(vec![
            "d".to_string(),
            "empty".to_string(),
            ".".to_string(),
            "q".to_string()
        ])
    );
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

// from chunk_02
#[test]
fn cli_parser_rejects_missing_or_duplicate_play_script() {
    assert!(parse_cli_args(["--play-script"]).is_err());
    assert!(parse_cli_args(["--play-script", "d", "--play-script", "q"]).is_err());
}

// from chunk_02
#[test]
fn cli_parser_recognizes_help_long_flag() {
    let args = parse_cli_args(["--help"]).unwrap();
    assert!(args.help);
    assert!(!args.play);
    assert!(args.play_script.is_none());
}

// from chunk_02
#[test]
fn cli_parser_recognizes_help_short_flag() {
    let args = parse_cli_args(["-h"]).unwrap();
    assert!(args.help);
}

// from chunk_02
#[test]
fn cli_parser_help_short_circuits_save_init_conflict() {
    // --help bypasses validation that would otherwise reject this combo,
    // so users can still get usage even with bad flags.
    let args = parse_cli_args(["--help", "--from-save", "--from-init"]).unwrap();
    assert!(args.help);
}

// from chunk_02
#[test]
fn cli_usage_lists_documented_smoke_commands() {
    assert!(CLI_USAGE.contains("--play"));
    assert!(CLI_USAGE.contains("--play-script"));
    assert!(CLI_USAGE.contains("--scene"));
    assert!(CLI_USAGE.contains("--floor"));
}

