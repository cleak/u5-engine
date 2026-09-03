//! Tests for `u5_tui::cli`.
//!
//! Moved from `u5-runtime` when CLI parsing was lifted into the TUI crate.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(args.play);
    assert!(args.raster_diagnostics);
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
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

#[test]
fn cli_parser_accepts_grapple_alias_for_climbing_gear() {
    let args = parse_cli_args([
        "--play",
        "--scene",
        "BRITANNIA",
        "--grapple",
        "0x04",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert_eq!(args.play_options.climbing_gear, 4);
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
        "horse",
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
        TransportState::Horse {
            type_byte: 160,
            tile: 160,
        }
    );
}

/// `vehicles.md` section 11: "Balloon sprites are catalog assets only."
/// There is no balloon vehicle family, so the parser must reject the word
/// rather than seat the player in a vehicle the game does not have.
///
/// This test previously asserted the opposite - that `--transport balloon`
/// produced a `TransportState::Balloon` - which is a large part of why the
/// divergence survived as long as it did.
#[test]
fn cli_parser_rejects_the_balloon_transport() {
    let error = parse_cli_args(["--play", "--transport", "balloon", r"C:\Games\U5-Clean"])
        .expect_err("balloon is not a vehicle family");
    assert!(
        error.to_string().to_ascii_lowercase().contains("transport"),
        "the error should name the rejected argument, got {error}"
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
        parse_cli_args(["--play", "--from-save", "--from-init", r"C:\Games\U5-Clean",]).is_err()
    );
}

// from chunk_02
#[test]
fn cli_parser_rejects_bad_raster_depth() {
    assert!(parse_cli_args(["--play", "--raster-depth", "vga"]).is_err());
}

// from chunk_02
#[test]
fn cli_parser_maps_tandy_raster_depth_to_ega_equivalent_path() {
    let args = parse_cli_args(["--play", "--raster-depth", "tandy", r"C:\Games\U5-Clean"]).unwrap();

    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
}

// from chunk_02
#[test]
fn cli_parser_rejects_hercules_as_out_of_v1_scope() {
    let err = parse_cli_args(["--play", "--raster-depth", "hercules"]).unwrap_err();

    assert!(err.to_string().contains("outside the v1"));
}

#[test]
fn cli_parser_rejects_cga_as_out_of_v1_scope() {
    let err = parse_cli_args(["--play", "--raster-depth", "cga"]).unwrap_err();

    assert!(err.to_string().contains("outside the v1"));
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

#[test]
fn cli_parser_accepts_route_smoke_mode() {
    let args = parse_cli_args([
        "--route-smoke",
        "--route-smoke-manifest",
        "target/route-smoke/manifest.txt",
        "--raster-depth",
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(args.route_smoke);
    assert!(!args.play);
    assert!(!args.visual);
    assert_eq!(
        args.route_smoke_manifest,
        Some(PathBuf::from("target/route-smoke/manifest.txt"))
    );
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_route_smoke_rejects_other_play_modes_and_overrides() {
    assert!(parse_cli_args(["--route-smoke", "--play"]).is_err());
    assert!(parse_cli_args(["--route-smoke", "--visual"]).is_err());
    assert!(parse_cli_args(["--route-smoke", "--save-frame", "frame.png"]).is_err());
    assert!(parse_cli_args(["--route-smoke", "--scene", "BRITANNIA"]).is_err());
    assert!(parse_cli_args(["--route-smoke", "--wind", "north"]).is_err());
    assert!(parse_cli_args(["--route-smoke-manifest", "target/manifest.txt"]).is_err());
    assert!(
        parse_cli_args([
            "--route-smoke",
            "--route-smoke-manifest",
            "one.txt",
            "--route-smoke-manifest",
            "two.txt"
        ])
        .is_err()
    );
}

#[test]
fn cli_parser_accepts_save_frame_with_script_and_scene_options() {
    let args = parse_cli_args([
        "--save-frame",
        "screenshots/world.png",
        "--scene",
        "BRITANNIA",
        "--play-script",
        "idle:2;q",
        "--raster-depth",
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(args.play);
    assert_eq!(
        args.save_frame,
        Some(PathBuf::from("screenshots/world.png"))
    );
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
    assert_eq!(
        args.play_script,
        Some(vec!["idle:2".to_string(), "q".to_string()])
    );
    assert_eq!(
        args.play_options.target,
        PlayTarget::World(WorldPlane::Britannia)
    );
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_parser_accepts_save_frame_suite_mode() {
    let args = parse_cli_args([
        "--save-frame-suite",
        "target/frame-suite",
        "--raster-depth",
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(!args.play);
    assert_eq!(
        args.save_frame_suite,
        Some(PathBuf::from("target/frame-suite"))
    );
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_parser_accepts_visual_frame_suite_mode() {
    let args = parse_cli_args([
        "--visual-frame-suite",
        "target/visual-frame-suite",
        "--raster-depth",
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(!args.play);
    assert!(!args.visual);
    assert_eq!(
        args.visual_frame_suite,
        Some(PathBuf::from("target/visual-frame-suite"))
    );
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_parser_accepts_visual_route_suite_mode() {
    let args = parse_cli_args([
        "--visual-route-suite",
        "target/visual-route-suite",
        "--raster-depth",
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(!args.play);
    assert!(!args.visual);
    assert_eq!(
        args.visual_route_suite,
        Some(PathBuf::from("target/visual-route-suite"))
    );
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_parser_accepts_frame_manifest_compare_mode() {
    let args = parse_cli_args([
        "--compare-frame-manifests",
        "target/baseline/manifest.txt",
        "target/candidate/manifest.txt",
    ])
    .unwrap();

    assert!(!args.play);
    assert!(!args.visual);
    assert_eq!(
        args.compare_frame_manifests,
        Some((
            PathBuf::from("target/baseline/manifest.txt"),
            PathBuf::from("target/candidate/manifest.txt")
        ))
    );
}

#[test]
fn cli_parser_accepts_location_audit_mode() {
    let args = parse_cli_args([
        "--location-audit",
        "target/location-audit.txt",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(!args.play);
    assert!(!args.visual);
    assert_eq!(
        args.location_audit,
        Some(PathBuf::from("target/location-audit.txt"))
    );
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_save_frame_suite_rejects_other_play_modes_and_overrides() {
    assert!(parse_cli_args(["--save-frame-suite", "out", "--play"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--visual"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--save-frame", "frame.png"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--route-smoke"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--scene", "BRITANNIA"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--play-script", "q"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--visual-frame-suite", "vis"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite", "out", "--visual-route-suite", "vis"]).is_err());
    assert!(
        parse_cli_args([
            "--save-frame-suite",
            "out",
            "--compare-frame-manifests",
            "base",
            "candidate"
        ])
        .is_err()
    );
}

#[test]
fn cli_visual_frame_suite_rejects_other_play_modes_and_overrides() {
    assert!(parse_cli_args(["--visual-frame-suite", "out", "--play"]).is_err());
    assert!(parse_cli_args(["--visual-frame-suite", "out", "--visual"]).is_err());
    assert!(parse_cli_args(["--visual-frame-suite", "out", "--save-frame", "frame.png"]).is_err());
    assert!(
        parse_cli_args([
            "--visual-frame-suite",
            "out",
            "--save-frame-suite",
            "frames"
        ])
        .is_err()
    );
    assert!(parse_cli_args(["--visual-frame-suite", "out", "--route-smoke"]).is_err());
    assert!(parse_cli_args(["--visual-frame-suite", "out", "--scene", "BRITANNIA"]).is_err());
    assert!(parse_cli_args(["--visual-frame-suite", "out", "--play-script", "q"]).is_err());
    assert!(
        parse_cli_args([
            "--visual-frame-suite",
            "out",
            "--visual-route-suite",
            "routes"
        ])
        .is_err()
    );
    assert!(
        parse_cli_args([
            "--visual-frame-suite",
            "out",
            "--compare-frame-manifests",
            "base",
            "candidate"
        ])
        .is_err()
    );
}

#[test]
fn cli_visual_route_suite_rejects_other_play_modes_and_overrides() {
    assert!(parse_cli_args(["--visual-route-suite", "out", "--play"]).is_err());
    assert!(parse_cli_args(["--visual-route-suite", "out", "--visual"]).is_err());
    assert!(parse_cli_args(["--visual-route-suite", "out", "--save-frame", "frame.png"]).is_err());
    assert!(
        parse_cli_args([
            "--visual-route-suite",
            "out",
            "--save-frame-suite",
            "frames"
        ])
        .is_err()
    );
    assert!(parse_cli_args(["--visual-route-suite", "out", "--route-smoke"]).is_err());
    assert!(parse_cli_args(["--visual-route-suite", "out", "--scene", "BRITANNIA"]).is_err());
    assert!(parse_cli_args(["--visual-route-suite", "out", "--play-script", "q"]).is_err());
    assert!(
        parse_cli_args([
            "--visual-route-suite",
            "out",
            "--compare-frame-manifests",
            "base",
            "candidate"
        ])
        .is_err()
    );
}

#[test]
fn cli_frame_manifest_compare_rejects_other_modes_and_missing_paths() {
    assert!(parse_cli_args(["--compare-frame-manifests"]).is_err());
    assert!(parse_cli_args(["--compare-frame-manifests", "base"]).is_err());
    assert!(parse_cli_args(["--compare-frame-manifests", "base", "candidate", "--play"]).is_err());
    assert!(
        parse_cli_args([
            "--compare-frame-manifests",
            "base",
            "candidate",
            "--visual-frame-suite",
            "out"
        ])
        .is_err()
    );
}

#[test]
fn cli_location_audit_rejects_other_modes_and_missing_path() {
    assert!(parse_cli_args(["--location-audit"]).is_err());
    assert!(parse_cli_args(["--location-audit", "out", "--play"]).is_err());
    assert!(parse_cli_args(["--location-audit", "out", "--intro"]).is_err());
    assert!(parse_cli_args(["--location-audit", "out", "--visual"]).is_err());
    assert!(parse_cli_args(["--location-audit", "out", "--route-smoke"]).is_err());
    assert!(parse_cli_args(["--location-audit", "out", "--save-frame", "frame.png"]).is_err());
    assert!(parse_cli_args(["--location-audit", "out", "--scene", "BRITANNIA"]).is_err());
    assert!(
        parse_cli_args(["--location-audit", "one.txt", "--location-audit", "two.txt"]).is_err()
    );
    assert!(
        parse_cli_args([
            "--location-audit",
            "out",
            "--compare-frame-manifests",
            "base",
            "candidate"
        ])
        .is_err()
    );
}

// from chunk_02
#[test]
fn cli_parser_rejects_missing_or_duplicate_play_script() {
    assert!(parse_cli_args(["--play-script"]).is_err());
    assert!(parse_cli_args(["--play-script", "d", "--play-script", "q"]).is_err());
}

#[test]
fn cli_parser_rejects_missing_save_frame_path() {
    assert!(parse_cli_args(["--save-frame"]).is_err());
    assert!(parse_cli_args(["--save-frame-suite"]).is_err());
    assert!(parse_cli_args(["--visual-frame-suite"]).is_err());
    assert!(parse_cli_args(["--visual-route-suite"]).is_err());
    assert!(parse_cli_args(["--route-smoke", "--route-smoke-manifest"]).is_err());
    assert!(parse_cli_args(["--compare-frame-manifests"]).is_err());
    assert!(parse_cli_args(["--location-audit"]).is_err());
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

#[test]
fn cli_binary_help_prints_usage_without_assets() {
    let output = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("SMOKE COMMANDS:"));
    assert!(stdout.contains("cargo run -- --play C:\\Games\\U5-Clean"));
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn cli_parser_accepts_intro_menu_mode() {
    let args = parse_cli_args([
        "--intro",
        "--raster-diagnostics",
        "--raster-depth",
        "tandy",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(args.intro);
    assert!(!args.play);
    assert_eq!(args.raster_depth, TileGraphicsDepth::Ega16);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_parser_accepts_visual_intro_menu_mode() {
    let args = parse_cli_args(["--intro", "--visual", r"C:\Games\U5-Clean"]).unwrap();

    assert!(args.intro);
    assert!(args.visual);
    assert!(!args.play);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_parser_accepts_visual_playable_alias() {
    let args = parse_cli_args(["--visual-playable", r"C:\Games\U5-Clean"]).unwrap();

    assert!(args.intro);
    assert!(args.visual);
    assert!(!args.play);
    assert_eq!(args.game_dir, PathBuf::from(r"C:\Games\U5-Clean"));
}

#[test]
fn cli_intro_rejects_direct_play_or_save_creation_modes() {
    assert!(parse_cli_args(["--intro", "--play"]).is_err());
    assert!(parse_cli_args(["--intro", "--from-save"]).is_err());
    assert!(parse_cli_args(["--intro", "--save-frame", "frame.png"]).is_err());
    assert!(parse_cli_args(["--intro", "--create-character-interactive"]).is_err());
    assert!(
        parse_cli_args([
            "--intro",
            "--create-character",
            "Avatar",
            "--gender",
            "male",
            "--chargen-winners",
            "Honesty,Compassion,Valor,Justice,Sacrifice,Honor,Spirituality",
        ])
        .is_err()
    );
}

#[test]
fn cli_intro_ignores_invalid_menu_key_without_explanatory_text() {
    let dir = debug_game_dir();
    let mut child = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--intro")
        .arg(dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b" \nX\n").unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Intro Menu"));
    assert!(!stdout.contains("Choose J, C, T, U, A, R"));
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cli_intro_journey_onward_without_active_save_returns_to_menu() {
    let dir = debug_game_dir();
    fs::write(dir.join(SAVED_GAM_FILENAME), vec![0; SAVED_GAM_LEN]).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--intro")
        .arg(dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"J\n\nX\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Journey Onward"));
    assert!(stdout.contains("Disk read failed for SAVED.GAM"));
    assert!(stdout.contains("No active game"));
    assert!(stdout.contains("Intro Menu"));
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cli_intro_pre_flourish_fails_loudly_when_ibm_ch_is_missing() {
    // `systems/intro.md §3` step 2 requires loading IBM.CH and
    // RUNES.CH before the title flourish. If the game directory is
    // missing these files, the intro must surface a NotFound IO
    // error instead of silently skipping the load. Removing IBM.CH
    // after `debug_game_dir` populates the standard fixture
    // simulates a partial install or a deleted asset.
    let dir = debug_game_dir();
    let _ = fs::remove_file(dir.join(IBM_CH_FILE));
    let mut child = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--intro")
        .arg(dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success(), "missing IBM.CH must fail loudly");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(IBM_CH_FILE), "{stderr}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cli_intro_non_j_first_key_does_not_take_journey_onward_path() {
    // `systems/intro.md §3` step 2 step 6: the pre-flourish poll
    // takes the Journey Onward shortcut only when the queued key
    // folds to `J`. A queued ` ` (space) must fall through to the
    // menu without printing the centered banner or attempting to
    // load the save. The X that follows is an invalid menu key
    // that the menu silently ignores; the second \n exits stdin.
    let dir = debug_game_dir();
    fs::write(dir.join(SAVED_GAM_FILENAME), vec![0; SAVED_GAM_LEN]).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--intro")
        .arg(dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b" \nX\n").unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Intro Menu"),
        "menu must follow non-J first key"
    );
    assert!(
        !stdout.contains("Disk read failed for SAVED.GAM"),
        "non-J first key must not attempt the Journey Onward load: {stdout}"
    );
    assert!(
        !stdout.contains("No active game"),
        "non-J first key must not surface the empty-save message: {stdout}"
    );
    let _ = fs::remove_dir_all(dir);
}

// from chunk_02
#[test]
fn cli_parser_help_short_circuits_save_init_conflict() {
    // --help bypasses validation that would otherwise reject this combo,
    // so users can still get usage even with bad flags.
    let args = parse_cli_args(["--help", "--from-save", "--from-init"]).unwrap();
    assert!(args.help);
}

#[test]
fn cli_parser_accepts_chargen_answers_and_derives_winners() {
    let args = parse_cli_args([
        "--create-character",
        "AVATAR",
        "--gender",
        "male",
        "--chargen-answers",
        "AAAAAAA",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();
    let command = args.create_character.unwrap();
    assert_eq!(command.name, b"AVATAR");
    assert!(command.male);
    // Tournament always produces exactly seven winners.
    assert_eq!(command.winners.len(), 7);
    // STR floor (20) always applies for the questionnaire path.
    assert_eq!(command.stats.strength, 20);
}

#[test]
fn cli_parser_rejects_invalid_chargen_answer_string() {
    let err = parse_cli_args([
        "--create-character",
        "AVATAR",
        "--gender",
        "male",
        "--chargen-answers",
        "AAAQAAA",
        r"C:\Games\U5-Clean",
    ])
    .unwrap_err();
    assert!(err.to_string().contains("A or B") || err.to_string().contains("chargen-answers"));
}

#[test]
fn cli_parser_rejects_wrong_length_chargen_answer_string() {
    let err = parse_cli_args([
        "--create-character",
        "AVATAR",
        "--gender",
        "male",
        "--chargen-answers",
        "AAA",
        r"C:\Games\U5-Clean",
    ])
    .unwrap_err();
    assert!(err.to_string().contains("seven characters"));
}

#[test]
fn cli_parser_accepts_deterministic_create_character_command() {
    let args = parse_cli_args([
        "--create-character",
        "AVATAR",
        "--gender",
        "female",
        "--chargen-winners",
        "Honesty,Compassion,Valor,Justice,Sacrifice,Honor,Spirituality",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();

    assert!(!args.play);
    assert!(!args.visual);
    let command = args.create_character.unwrap();
    assert_eq!(command.name, b"AVATAR");
    assert_eq!(command.male, false);
    assert_eq!(
        command.winners,
        vec![
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
        ]
    );
    // `chargen.md 7`: "The running dexterity tally starts from the shipped
    // seed record's dexterity of `15`", so these seven winners' DEX deltas
    // (5) are added to that 15 rather than to zero.
    assert_eq!(
        command.stats,
        ChargenStats {
            strength: 20,
            dexterity: CHARGEN_SEED_DEXTERITY + 5,
            intelligence: 5,
        }
    );
}

#[test]
fn cli_create_character_command_writes_saved_files_and_returns_without_play() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(13, 0, 15, 15)).unwrap();
    fs::write(dir.join("INIT.OOL"), vec![0x77; OOL_PLANE_LEN]).unwrap();
    let args = parse_cli_args([
        "--create-character",
        "IOLO",
        "--gender",
        "male",
        "--chargen-winners",
        "Humility,Humility,Humility,Humility,Humility,Humility,Humility",
        dir.to_str().unwrap(),
    ])
    .unwrap();

    let avatar =
        run_create_character_command(&args.game_dir, args.create_character.as_ref().unwrap())
            .unwrap();

    assert!(!args.play);
    assert_eq!(avatar.name, *b"IOLO\0\0\0\0\0");
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(
        &saved[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + SAVE_CHARACTER_NAME_LEN],
        b"IOLO\0\0\0\0\0"
    );
    assert_eq!(
        saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_GENDER_OFFSET],
        SAVE_GENDER_MALE_BYTE
    );
    let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
    assert_eq!(&saved_ool[..OOL_PLANE_LEN], vec![0; OOL_PLANE_LEN]);
    assert_eq!(&saved_ool[OOL_PLANE_LEN..], vec![0x77; OOL_PLANE_LEN]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cli_binary_create_character_save_loads_through_play_script() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(17, 0, 15, 15)).unwrap();
    fs::write(dir.join("INIT.OOL"), vec![0; OOL_PLANE_LEN]).unwrap();

    let create = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .args([
            "--create-character",
            "AVATAR",
            "--gender",
            "male",
            "--chargen-answers",
            "AAAAAAA",
        ])
        .arg(dir.to_str().unwrap())
        .output()
        .unwrap();
    assert!(create.status.success());
    assert!(String::from_utf8(create.stderr).unwrap().is_empty());

    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(
        &saved[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 6],
        b"AVATAR"
    );

    let play = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .args(["--play", "--from-save", "--play-script", "q"])
        .arg(dir.to_str().unwrap())
        .output()
        .unwrap();
    assert!(play.status.success());
    let stdout = String::from_utf8(play.stdout).unwrap();
    assert!(stdout.contains("Ultima V playable harness"));
    assert!(stdout.contains("Script mode: 1 command(s)."));
    assert!(stdout.contains("State: CASTLE:0"));
    // The one tolerated stderr line is the interim spec-gap notice for the
    // missing location floor-page table (`cleak/u5-spec#80`); the synthetic
    // fixture dir ships no `location_floor_pages.tsv`.
    let play_stderr = String::from_utf8(play.stderr).unwrap();
    assert!(
        play_stderr.is_empty() || play_stderr.contains("cleak/u5-spec#80"),
        "{play_stderr}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cli_binary_character_creation_stages_a_readonly_install() {
    let source = debug_game_dir();
    let runtime_root = PathBuf::from(format!("{}-runtime", source.display()));
    fs::create_dir_all(&runtime_root).unwrap();
    fs::write(
        source.join("INIT.GAM"),
        saved_game_seed_bytes(17, 0, 15, 15),
    )
    .unwrap();
    fs::write(source.join("INIT.OOL"), vec![0; OOL_PLANE_LEN]).unwrap();
    let brit_path = source.join(BRIT_OOL_FILENAME);
    let brit_bytes = vec![0x5a; OOL_PLANE_LEN];
    fs::write(&brit_path, &brit_bytes).unwrap();
    let mut permissions = fs::metadata(&brit_path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&brit_path, permissions).unwrap();

    let create = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .args([
            "--create-character",
            "AVATAR",
            "--gender",
            "male",
            "--chargen-answers",
            "AAAAAAA",
        ])
        .arg(source.as_os_str())
        .env(RUNTIME_DIR_ENV, &runtime_root)
        .output()
        .unwrap();
    assert!(create.status.success(), "{:?}", create.stderr);
    assert_eq!(fs::read(&brit_path).unwrap(), brit_bytes);
    assert!(!source.join(SAVED_GAM_FILENAME).exists());

    let runtime = fs::read_dir(&runtime_root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_dir())
        .expect("launcher should create one writable runtime mirror");
    let saved = fs::read(runtime.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(
        &saved[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 6],
        b"AVATAR"
    );
    assert!(
        !fs::metadata(runtime.join(BRIT_OOL_FILENAME))
            .unwrap()
            .permissions()
            .readonly()
    );

    u5_runtime::test_fixtures::clear_readonly(&brit_path).unwrap();
    fs::remove_dir_all(source).unwrap();
    fs::remove_dir_all(runtime_root).unwrap();
}

#[test]
fn cli_binary_play_script_confirmed_save_round_trips_to_temp_save() {
    let dir = debug_game_dir();
    let mut save = saved_game_seed_bytes(0, 0, 10, 20);
    save[SAVE_AVATAR_NAME_OFFSET] = b'A';
    fs::write(dir.join(SAVED_GAM_FILENAME), save).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .args(["--play", "--from-save", "--play-script", "6;QY;q"])
        .arg(dir.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Script mode: 3 command(s)."));
    // `save-load.md §5.2` steps 2 and 8: the handler prints `Yes`, then
    // `Saving...`, then `Done.` as three separate lines rather than the
    // one `Yes. Saving... Done.` run the engine used to emit and the
    // fifteen-cell window then wrapped. This ASCII harness paints only
    // the newest line of the stream (`text-output.md §11`), so the last
    // of the three is what reaches stdout; the full three-line sequence
    // is pinned by `u5-runtime`'s `zstats_text` batch.
    let squished: String = stdout.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert!(squished.contains("Done."), "{stdout}");
    assert!(!squished.contains("Yes.Saving..."), "{stdout}");
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());

    let reloaded = load_play_options_from_save(&dir).unwrap();
    assert_eq!(reloaded.target, PlayTarget::World(WorldPlane::Britannia));
    assert_eq!(reloaded.start, Some((11, 20)));
    assert_eq!(reloaded.clock.minute, 37);
    let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
    assert_eq!(saved_ool.len(), SAVED_OOL_LEN);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn cli_intro_u4_transfer_refuses_terminal_preview_fallback() {
    let dir = debug_game_dir();
    // `u4-transfer.md §4` / `cleak/u5-spec#88`: the U5-side seed pair is
    // `INIT.GAM` / `INIT.OOL`. `BRIT.GAM` is withdrawn and does not ship.
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 22, 33)).unwrap();
    fs::write(dir.join("INIT.OOL"), vec![0x44; OOL_PLANE_LEN]).unwrap();
    fs::write(
        dir.join(U4_TRANSFER_U4_SOURCE_FILENAME),
        u4_transfer_party_sav_fixture(),
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--intro")
        .arg(dir.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\nT\n").unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("terminal Ultima IV transfer preview is a forbidden fallback"),
        "{stderr}"
    );
    assert!(stderr.contains("cleak/u5-spec#73"), "{stderr}");
    assert!(
        !dir.join(SAVED_GAM_FILENAME).exists(),
        "terminal fallback must not commit a save"
    );
    let _ = fs::remove_dir_all(dir);
}

/// A **synthetic** `PARTY.SAV` built from the published
/// `u4-transfer.md §5.1` layout by the engine's single fixture builder.
///
/// This used to be a second, hand-written fixture that placed the name
/// at file offset `0x001A` and the class at `0x0019` and filled nine
/// party-wide counters the transfer never reads - the same wrong layout
/// the retired parser used, so the pair agreed with each other and with
/// nothing else (`cleak/u5-spec#88`). It is replaced by
/// `synthetic_party_sav_fixture`, which writes the `§5.1` offsets the
/// one surviving parser reads.
///
/// No Ultima IV installation or genuine `PARTY.SAV` exists on this
/// machine, and none was fabricated and treated as real data.
fn u4_transfer_party_sav_fixture() -> Vec<u8> {
    synthetic_party_sav_fixture(&U4PreviewSource {
        name: "AVATAR".to_string(),
        male: true,
        class_index: 6,
        strength: 29,
        dexterity: 30,
        intelligence: 9,
        experience: 4321,
        max_hit_points: 300,
        // `§5.3`: one nonzero standing, so this source is not an
        // Avatar. All-zero standings would be accepted too - they are
        // the Avatar success condition, never a rejection.
        is_avatar: false,
    })
}

#[test]
fn cli_create_character_rejects_incomplete_or_play_combined_invocations() {
    assert!(
        parse_cli_args([
            "--create-character",
            "AVATAR",
            "--chargen-winners",
            "Honesty,Compassion,Valor,Justice,Sacrifice,Honor,Spirituality",
        ])
        .is_err()
    );
    assert!(
        parse_cli_args([
            "--play",
            "--create-character",
            "AVATAR",
            "--gender",
            "male",
            "--chargen-winners",
            "Honesty,Compassion,Valor,Justice,Sacrifice,Honor,Spirituality",
        ])
        .is_err()
    );
    assert!(parse_chargen_winners_arg("Honesty,Valor").is_err());
    assert!(parse_chargen_gender_arg("unknown").is_err());
}

// from chunk_02
#[test]
fn cli_usage_lists_documented_smoke_commands() {
    assert!(CLI_USAGE.contains("--play"));
    assert!(CLI_USAGE.contains("--play-script"));
    assert!(CLI_USAGE.contains("--route-smoke"));
    assert!(CLI_USAGE.contains("--save-frame-suite"));
    assert!(CLI_USAGE.contains("--visual-frame-suite"));
    assert!(CLI_USAGE.contains("--visual-route-suite"));
    assert!(CLI_USAGE.contains("--visual-playable"));
    assert!(CLI_USAGE.contains("--compare-frame-manifests"));
    assert!(CLI_USAGE.contains("--location-audit"));
    assert!(CLI_USAGE.contains("--scene"));
    assert!(CLI_USAGE.contains("--floor"));
    assert!(CLI_USAGE.contains("--create-character"));
}
