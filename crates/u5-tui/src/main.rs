use std::env;
use std::io;

use u5_runtime::run_report;
use u5_tui::{CLI_USAGE, CliArgs, parse_cli_args, run_play_loop};

fn main() -> io::Result<()> {
    let args = parse_cli_args(env::args().skip(1))?;
    if args.help {
        print!("{CLI_USAGE}");
        return Ok(());
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

#[cfg(not(feature = "visual"))]
fn run_visual(_args: CliArgs) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "--visual requires building with --features visual (e.g. \
         `cargo run --features visual -- --visual <GAME_DIR>`).",
    ))
}
