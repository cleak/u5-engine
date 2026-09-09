use std::env;
use std::fs;
use std::io;
use std::path::Path;

use u5_runtime::{
    Direction, DiskPromptSession, PlayOptions, PlayState, audit_location_dat_files,
    location_audit_report_text, run_report,
};
use u5_tui::{
    CLI_USAGE, CliArgs, compare_manifest_files, parse_cli_args, prepare_writable_game_dir,
    run_audio_suite, run_create_character_command, run_interactive_create_character,
    run_intro_menu_loop, run_play_loop, run_route_smoke, run_save_frame, run_save_frame_suite,
    run_save_screen,
};

fn main() -> io::Result<()> {
    let mut args = parse_cli_args(env::args().skip(1))?;
    if args.help {
        print!("{CLI_USAGE}");
        return Ok(());
    }
    if needs_writable_game_dir(&args) {
        args.game_dir = prepare_writable_game_dir(&args.game_dir)?;
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
    if args.write_seed {
        return run_write_seed(
            &args.game_dir,
            args.play_options,
            args.play_script.clone(),
            args.seed_beside,
        );
    }
    if let Some(out) = args.audio_suite.as_deref() {
        return run_audio_suite(out);
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

fn needs_writable_game_dir(args: &CliArgs) -> bool {
    args.intro
        || args.play
        || args.visual
        || args.create_character.is_some()
        || args.create_character_interactive
}

/// Persist the start state `--scene`/`--at`/`--time` describe as a save.
///
/// `formats/saved-gam.md` publishes the image the original reads, and the
/// engine already writes it for the S-Save command; this is the same writer
/// with no game attached. A paired scenario that needs the party somewhere
/// specific can then seed **both** sides from one file instead of scripting a
/// walk to it, which is what makes long walk-ups diverge: town residents move
/// on their own schedules, so sixteen steps of drift put the two sides in
/// front of different cells (`cleak/u5-engine#22`).
/// Move the party onto a walkable cell adjacent to the NPC carrying
/// `dialog_id`, facing it.
///
/// A shopkeeper walks its schedule, so replaying a scenario's walk puts the
/// party where the script aimed but not necessarily beside the keeper - which
/// is the whole reason those scenarios diverge. Placing the pair in contact
/// before the save is written gives both sides one deterministic starting
/// arrangement. This edits the seed, not gameplay: nothing in the shipped game
/// relocates the party this way.
fn place_party_beside_dialog(state: &mut PlayState, dialog_id: u8) -> io::Result<()> {
    let floor = state.current_floor().unwrap_or(0);
    let Some((nx, ny)) = state
        .npcs
        .iter()
        .filter(|npc| npc.dialog_id == dialog_id && i32::from(npc.z) == i32::from(floor as u8))
        .map(|npc| (npc.x, npc.y))
        .next()
    else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no NPC with dialog byte {dialog_id:#04x} on this floor"),
        ));
    };
    // West of the NPC first, so the scenario's Talk is `Talk-East`, which is
    // what the walk-up scripts already use.
    for (dx, dy, facing) in [
        (-1i32, 0i32, Direction::East),
        (1, 0, Direction::West),
        (0, -1, Direction::South),
        (0, 1, Direction::North),
    ] {
        let x = nx as i32 + dx;
        let y = ny as i32 + dy;
        if x < 0 || y < 0 {
            continue;
        }
        let (x, y) = (x as usize, y as usize);
        let tile = state.current_area_tile(x, y);
        if !state.tile_walkable(tile) || state.npc_at_current_floor(x, y).is_some() {
            continue;
        }
        state.player.x = x;
        state.player.y = y;
        state.player.facing = facing;
        state.sync_player_object();
        println!("Placed the party at {x},{y} facing {facing:?}, beside {nx},{ny}");
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("no walkable cell adjacent to the NPC at {nx},{ny}"),
    ))
}

fn run_write_seed(
    game_dir: &Path,
    options: PlayOptions,
    play_script: Option<Vec<String>>,
    seed_beside: Option<u8>,
) -> io::Result<()> {
    let mut state = PlayState::load_scene(game_dir, options)?;
    // A `--play-script` runs *before* the write, so a seed can be placed by
    // replaying the walk a scenario used to script. That is the point: the
    // walk is what diverges, so doing it once here and saving the result lets
    // the scenario start where it was trying to get to.
    if let Some(commands) = play_script {
        for command in &commands {
            u5_tui::handle_play_script_command(&mut state, command, game_dir)?;
        }
    }
    if let Some(dialog_id) = seed_beside {
        place_party_beside_dialog(&mut state, dialog_id)?;
    }
    let mut disk_session = DiskPromptSession::single_directory();
    state.write_save_files_with_disk_session(game_dir, &mut disk_session)?;
    let (scene, _z, x, y) = state
        .current_save_location()
        .expect("a written seed is always in an active play mode");
    println!(
        "Wrote seed save into {} (scene {scene}, at {x},{y}, {:02}:{:02})",
        game_dir.display(),
        state.clock.hour,
        state.clock.minute
    );
    Ok(())
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
