use std::env;
use std::io;

use u5_runtime::run_report;
use u5_tui::{
    CLI_USAGE, CliArgs, parse_cli_args, run_create_character_command,
    run_interactive_create_character, run_intro_menu_loop, run_play_loop, run_route_smoke,
    run_save_frame, run_save_frame_suite,
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
    if let Some(out) = args.save_frame.as_deref() {
        return run_save_frame(
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
    if args.route_smoke {
        return run_route_smoke(&args.game_dir, args.raster_depth);
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
        "--intro --visual requires building with --features visual (e.g. \
         `cargo run --features visual -- --intro --visual <GAME_DIR>`).",
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
