use std::env;
use std::fs;
use std::io;
use std::path::Path;

use u5_runtime::{audit_location_dat_files, location_audit_report_text, run_report};
use u5_tui::{
    CLI_USAGE, CliArgs, compare_manifest_files, parse_cli_args, run_create_character_command,
    run_interactive_create_character, run_intro_menu_loop, run_play_loop, run_route_smoke,
    run_save_frame, run_save_frame_suite, run_save_screen,
};

fn main() -> io::Result<()> {
    let args = parse_cli_args(env::args().skip(1))?;
    if args.help {
        print!("{CLI_USAGE}");
        return Ok(());
    }
    if args.intro && args.visual {
        return run_visual_intro(args);
    }
    if args.intro {
        return run_intro_menu_loop(&args.game_dir, args.raster_diagnostics, args.raster_depth);
    }
    if let Some(command) = args.create_character.as_ref() {
        let avatar = run_create_character_command(&args.game_dir, command)?;
        let name = String::from_utf8_lossy(
            &avatar.name[..avatar
                .name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(avatar.name.len())],
        );
        println!("Created character {name}. Choose Journey Onward to load the new save.");
        return Ok(());
    }
    if args.create_character_interactive {
        run_interactive_create_character(&args.game_dir)?;
        return Ok(());
    }
    if let Some(out) = args.location_audit.as_deref() {
        return run_location_audit(&args.game_dir, out);
    }
    if let Some((baseline, candidate)) = args.compare_frame_manifests.as_ref() {
        let report = compare_manifest_files(baseline, candidate)?;
        println!("{}", report.summary());
        if report.is_clean() {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame manifest comparison failed",
        ));
    }
    if let Some(out) = args.save_frame.as_deref() {
        return run_save_frame(
            &args.game_dir,
            args.play_options,
            args.raster_depth,
            args.play_script,
            out,
        );
    }
    if let Some(out) = args.save_screen.as_deref() {
        return run_save_screen(
            &args.game_dir,
            args.play_options,
            args.raster_depth,
            args.play_script,
            out,
        );
    }
    if let Some(out_dir) = args.save_frame_suite.as_deref() {
        return run_save_frame_suite(&args.game_dir, args.raster_depth, out_dir);
    }
    if let Some(out_dir) = args.visual_frame_suite.as_deref() {
        return run_visual_frame_suite(&args, out_dir);
    }
    if let Some(out_dir) = args.visual_route_suite.as_deref() {
        return run_visual_route_suite(&args, out_dir);
    }
    if args.route_smoke {
        return run_route_smoke(
            &args.game_dir,
            args.raster_depth,
            args.route_smoke_manifest.as_deref(),
        );
    }
    if args.visual {
        return run_visual(args);
    }
    if args.play {
        return run_play_loop(
            &args.game_dir,
            args.play_options,
            args.raster_diagnostics,
            args.raster_depth,
            args.play_script,
        );
    }
    run_report(&args.game_dir)
}

fn run_location_audit(game_dir: &Path, out: &Path) -> io::Result<()> {
    let report = audit_location_dat_files(game_dir)?;
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, location_audit_report_text(&report))?;
    println!("Saved location audit: {}", out.display());
    Ok(())
}

#[cfg(feature = "visual")]
fn run_visual(args: CliArgs) -> io::Result<()> {
    u5_bevy::run_visual_loop(&args.game_dir, args.play_options, args.raster_depth)
}

#[cfg(feature = "visual")]
fn run_visual_intro(args: CliArgs) -> io::Result<()> {
    u5_bevy::run_visual_intro_loop(&args.game_dir, args.raster_depth)
}

#[cfg(feature = "visual")]
fn run_visual_frame_suite(args: &CliArgs, out_dir: &std::path::Path) -> io::Result<()> {
    u5_bevy::run_visual_frame_suite(&args.game_dir, args.raster_depth, out_dir)
}

#[cfg(feature = "visual")]
fn run_visual_route_suite(args: &CliArgs, out_dir: &std::path::Path) -> io::Result<()> {
    u5_bevy::run_visual_route_suite(&args.game_dir, args.raster_depth, out_dir)
}

#[cfg(not(feature = "visual"))]
fn run_visual(_args: CliArgs) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "--visual requires building with --features visual (e.g. \
         `cargo run --features visual -- --visual <GAME_DIR>`).",
    ))
}

#[cfg(not(feature = "visual"))]
fn run_visual_intro(_args: CliArgs) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "--visual-playable / --intro --visual requires building with --features visual (e.g. \
         `cargo run --features visual -- --visual-playable <GAME_DIR>`).",
    ))
}

#[cfg(not(feature = "visual"))]
fn run_visual_frame_suite(_args: &CliArgs, _out_dir: &std::path::Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "--visual-frame-suite requires building with --features visual (e.g. \
         `cargo run --features visual -- --visual-frame-suite <DIR> <GAME_DIR>`).",
    ))
}

#[cfg(not(feature = "visual"))]
fn run_visual_route_suite(_args: &CliArgs, _out_dir: &std::path::Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "--visual-route-suite requires building with --features visual (e.g. \
         `cargo run --features visual -- --visual-route-suite <DIR> <GAME_DIR>`).",
    ))
}
