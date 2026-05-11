//! CLI argument parsing, `CliArgs`, and the `--help` text.
//!
//! Lifted out of `u5-runtime`. The runtime owns parsing of inline
//! command suffixes typed at the play prompt; this module owns the
//! command-line surface.

use std::io;
use std::path::PathBuf;

use u5_runtime::{
    DEFAULT_GAME_DIR, FIRST_PLAYABLE_BALLOON_TILE, FIRST_PLAYABLE_FRIGATE_TILE,
    FIRST_PLAYABLE_FULL_SHIP_HULL, FIRST_PLAYABLE_SKIFF_TILE, GameClock,
    PendingVehicleAcquisition, PlayOptions, PlayTarget, TileGraphicsDepth, TimingStatusTag,
    TransportState, WindState, load_play_options_from_init, load_play_options_from_save,
    parse_u8_literal,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliArgs {
    pub play: bool,
    pub visual: bool,
    pub raster_diagnostics: bool,
    pub raster_depth: TileGraphicsDepth,
    pub play_script: Option<Vec<String>>,
    pub game_dir: PathBuf,
    pub play_options: PlayOptions,
    pub help: bool,
    /// If set, render the top-down viewport after the play-script runs and
    /// write it to this path as a PNG. Bypasses the interactive play loop
    /// and the Bevy harness; useful for verifying movement without a desktop.
    pub save_frame: Option<PathBuf>,
}

pub fn split_play_script(script: &str) -> Vec<String> {
    script
        .split(';')
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn parse_cli_args<I>(args: I) -> io::Result<CliArgs>
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let mut play = false;
    let mut visual = false;
    let mut raster_diagnostics = false;
    let mut raster_depth = TileGraphicsDepth::Ega16;
    let mut play_script = None;
    let mut game_dir = None;
    let mut options = PlayOptions::default();
    let mut from_save = false;
    let mut from_init = false;
    let mut wind_override = None;
    let mut climbing_gear_override = None;
    let mut pending_vehicle_override = None;
    let mut transport_override = None;
    let mut help = false;
    let mut save_frame: Option<PathBuf> = None;
    let mut args = args.into_iter().map(Into::into);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--play" => play = true,
            "--visual" => visual = true,
            "--save-frame" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--save-frame requires a PNG output path",
                    )
                })?;
                save_frame = Some(PathBuf::from(value));
                play = true;
            }
            "--play-script" => {
                play = true;
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--play-script requires semicolon-separated commands",
                    )
                })?;
                if play_script.replace(split_play_script(&value)).is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--play-script may only be supplied once",
                    ));
                }
            }
            "--raster-diagnostics" => raster_diagnostics = true,
            "--raster-depth" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--raster-depth requires ega or cga",
                    )
                })?;
                raster_depth = TileGraphicsDepth::from_key(&value)?;
            }
            "--from-save" => from_save = true,
            "--from-init" => from_init = true,
            "--scene" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--scene requires a value")
                })?;
                options.target = PlayTarget::from_key(&value)?;
            }
            "--debug-enter" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--debug-enter requires a value",
                    )
                })?;
                options.debug_enter = Some(PlayTarget::from_key(&value)?);
            }
            "--floor" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--floor requires a value")
                })?;
                options.floor = value.parse::<i8>().map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid floor `{value}`: {err}"),
                    )
                })?;
            }
            "--at" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--at requires x,y")
                })?;
                options.start = Some(parse_start_arg(&value)?);
            }
            "--time" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--time requires HH:MM")
                })?;
                options.clock = parse_time_arg(&value)?;
            }
            "--wind" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--wind requires calm|north|south|east|west",
                    )
                })?;
                wind_override = Some(WindState::from_key(&value)?);
            }
            "--climbing-gear" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--climbing-gear requires a byte value",
                    )
                })?;
                climbing_gear_override = Some(parse_u8_literal(&value)?);
            }
            "--pending-vehicle" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--pending-vehicle requires frigate:x,y[,skiffs] or skiff:x,y",
                    )
                })?;
                pending_vehicle_override = Some(parse_pending_vehicle_arg(&value)?);
            }
            "--transport" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--transport requires foot|horse|ship|skiff|carpet|balloon",
                    )
                })?;
                transport_override = Some(parse_transport_arg(&value)?);
            }
            _ if arg.starts_with("--") => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown option `{arg}`"),
                ));
            }
            _ => game_dir = Some(PathBuf::from(arg)),
        }
    }
    let game_dir = game_dir.unwrap_or_else(|| PathBuf::from(DEFAULT_GAME_DIR));
    if help {
        return Ok(CliArgs {
            play: false,
            visual: false,
            raster_diagnostics: false,
            raster_depth,
            play_script: None,
            game_dir,
            play_options: PlayOptions::default(),
            help: true,
            save_frame: None,
        });
    }
    if from_save && from_init {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--from-save and --from-init are mutually exclusive",
        ));
    }
    if from_save {
        options = load_play_options_from_save(&game_dir)?;
    } else if from_init {
        options = load_play_options_from_init(&game_dir)?;
    }
    if let Some(wind) = wind_override {
        options.wind = wind;
        if wind == WindState::Calm {
            options.wind_save_byte = 0;
        }
    }
    if let Some(climbing_gear) = climbing_gear_override {
        options.climbing_gear = climbing_gear;
    }
    if let Some(pending_vehicle) = pending_vehicle_override {
        options.pending_vehicle = Some(pending_vehicle);
    }
    if let Some(transport) = transport_override {
        options.transport = transport;
        options.timing_status = TimingStatusTag::for_transport(transport);
    }

    Ok(CliArgs {
        play,
        visual,
        raster_diagnostics,
        raster_depth,
        play_script,
        game_dir,
        play_options: options,
        help: false,
        save_frame,
    })
}

pub const CLI_USAGE: &str = "\
Ultima V verification + first-playable harness (clean-room).

USAGE:
    cargo run -- [OPTIONS] [GAME_DIR]

GAME_DIR defaults to the local clean asset path. With no flags, runs the
Lord British throne-room verification report.

OPTIONS:
    -h, --help                Print this usage and exit.
        --play                Launch the terminal first-playable harness.
        --play-script <CMDS>  Run a semicolon-separated script then exit.
                              Implies --play.
        --scene <KEY>         Start scene, e.g. CASTLE:0 or DUNGEON:0.
        --floor <N>           Start floor/level (signed).
        --at <X,Y>            Start coordinates.
        --time <HH:MM>        Start clock.
        --wind <DIR>          calm|north|south|east|west.
        --transport <KIND>    foot|horse|ship|skiff|carpet|balloon.
        --from-save           Seed play options from SAVED.GAM/SAVED.OOL.
        --from-init           Seed play options from INIT.GAM (debug).
        --raster-diagnostics  Emit per-frame raster diagnostics.
        --raster-depth <D>    ega|cga (default ega).
        --visual              Launch the Bevy top-down visual harness.
                              Requires building with `--features visual`.
        --save-frame <PATH>   Render the top-down viewport (after running
                              --play-script if given) to a PNG and exit.
                              Useful for verifying movement without a desktop.

SMOKE COMMANDS:
    cargo run -- C:\\Games\\U5-Clean
    cargo run -- --play C:\\Games\\U5-Clean
    cargo run -- --play-script \"z;q\" C:\\Games\\U5-Clean
    cargo run -- --play --scene DUNGEON:0 --floor 0 C:\\Games\\U5-Clean
    cargo run --features visual -- --visual --scene BRITANNIA C:\\Games\\U5-Clean
    cargo run --features visual -- --visual --scene CASTLE:0 --floor 0 C:\\Games\\U5-Clean
";

pub fn parse_start_arg(value: &str) -> io::Result<(usize, usize)> {
    let (x, y) = value.split_once(',').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("start coordinate must be x,y, got `{value}`"),
        )
    })?;
    let x = x.trim().parse::<usize>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid start x `{x}`: {err}"),
        )
    })?;
    let y = y.trim().parse::<usize>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid start y `{y}`: {err}"),
        )
    })?;
    Ok((x, y))
}

pub fn parse_pending_vehicle_arg(value: &str) -> io::Result<PendingVehicleAcquisition> {
    let (kind, payload) = value.split_once(':').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("pending vehicle must be frigate:x,y[,skiffs] or skiff:x,y, got `{value}`"),
        )
    })?;
    let parts: Vec<_> = payload
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    let parse_coord = |part: &str, axis: &str| -> io::Result<usize> {
        Ok(parse_u8_literal(part).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("invalid pending vehicle {axis} `{part}`: {err}"),
            )
        })? as usize)
    };
    match kind.trim().to_ascii_lowercase().as_str() {
        "frigate" => {
            if !(2..=3).contains(&parts.len()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("pending frigate must be frigate:x,y[,skiffs], got `{value}`"),
                ));
            }
            let x = parse_coord(parts[0], "x")?;
            let y = parse_coord(parts[1], "y")?;
            let skiffs = parts
                .get(2)
                .map(|skiffs| parse_u8_literal(skiffs))
                .transpose()?
                .unwrap_or(2);
            Ok(PendingVehicleAcquisition::Frigate { x, y, skiffs })
        }
        "skiff" => {
            if parts.len() != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("pending skiff must be skiff:x,y, got `{value}`"),
                ));
            }
            Ok(PendingVehicleAcquisition::Skiff {
                x: parse_coord(parts[0], "x")?,
                y: parse_coord(parts[1], "y")?,
            })
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown pending vehicle kind `{kind}`"),
        )),
    }
}


pub fn parse_transport_arg(value: &str) -> io::Result<TransportState> {
    match value.trim().to_ascii_lowercase().as_str() {
        "foot" => Ok(TransportState::Foot),
        "horse" => Ok(TransportState::Horse {
            type_byte: 160,
            tile: 160,
        }),
        "ship" | "frigate" => Ok(TransportState::Ship {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
            skiffs: 2,
        }),
        "skiff" => Ok(TransportState::Skiff {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
        }),
        "carpet" | "magic-carpet" => Ok(TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        }),
        "balloon" => Ok(TransportState::Balloon {
            type_byte: FIRST_PLAYABLE_BALLOON_TILE,
            tile: FIRST_PLAYABLE_BALLOON_TILE,
        }),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown transport `{value}`; expected foot|horse|ship|skiff|carpet|balloon"),
        )),
    }
}

pub fn parse_time_arg(value: &str) -> io::Result<GameClock> {
    let (hour, minute) = if let Some((hour, minute)) = value.split_once(':') {
        (hour, minute)
    } else {
        (value, "0")
    };
    let hour = hour.trim().parse::<u8>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid time hour `{hour}`: {err}"),
        )
    })?;
    let minute = minute.trim().parse::<u8>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid time minute `{minute}`: {err}"),
        )
    })?;
    GameClock::new(hour, minute)
}

