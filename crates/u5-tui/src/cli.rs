//! CLI argument parsing, `CliArgs`, and the `--help` text.
//!
//! Lifted out of `u5-runtime`. The runtime owns parsing of inline
//! command suffixes typed at the play prompt; this module owns the
//! command-line surface.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use u5_runtime::{
    CHARGEN_QUESTION_COUNT, ChargenAvatar, ChargenSession, ChargenSessionStep, ChargenStats,
    DEFAULT_GAME_DIR, FIRST_PLAYABLE_BALLOON_TILE, FIRST_PLAYABLE_FRIGATE_TILE,
    FIRST_PLAYABLE_FULL_SHIP_HULL, FIRST_PLAYABLE_SKIFF_TILE, GameClock, PendingVehicleAcquisition,
    PlayOptions, PlayTarget, ShrineVirtue, TileGraphicsDepth, TimingStatusTag, TransportState,
    WindState, chargen_stats_from_winners, commit_chargen_save, load_play_options_from_init,
    load_play_options_from_save, load_question_records, parse_u8_literal, run_chargen_tournament,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateCharacterCommand {
    pub name: Vec<u8>,
    pub male: bool,
    pub winners: Vec<ShrineVirtue>,
    pub stats: ChargenStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliArgs {
    pub intro: bool,
    pub play: bool,
    pub visual: bool,
    pub raster_diagnostics: bool,
    pub raster_depth: TileGraphicsDepth,
    pub route_smoke: bool,
    pub play_script: Option<Vec<String>>,
    pub game_dir: PathBuf,
    pub play_options: PlayOptions,
    pub help: bool,
    /// If set, render the current viewport after the play-script runs and
    /// write it to this path as a PNG. Bypasses the interactive play loop
    /// and the Bevy harness; useful for verifying movement without a desktop.
    pub save_frame: Option<PathBuf>,
    /// If set, write representative headless PNG frames and a sanitized
    /// manifest into the supplied directory.
    pub save_frame_suite: Option<PathBuf>,
    pub create_character: Option<CreateCharacterCommand>,
    pub create_character_interactive: bool,
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
    let mut intro = false;
    let mut visual = false;
    let mut raster_diagnostics = false;
    let mut raster_depth = TileGraphicsDepth::Ega16;
    let mut route_smoke = false;
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
    let mut save_frame_suite: Option<PathBuf> = None;
    let mut create_character_name: Option<Vec<u8>> = None;
    let mut create_character_male: Option<bool> = None;
    let mut create_character_winners: Option<Vec<ShrineVirtue>> = None;
    let mut create_character_interactive = false;
    let mut args = args.into_iter().map(Into::into);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--intro" => intro = true,
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
            "--save-frame-suite" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--save-frame-suite requires an output directory",
                    )
                })?;
                save_frame_suite = Some(PathBuf::from(value));
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
            "--route-smoke" => route_smoke = true,
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
            "--create-character" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--create-character requires a name",
                    )
                })?;
                if create_character_name.replace(value.into_bytes()).is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--create-character may only be supplied once",
                    ));
                }
            }
            "--create-character-interactive" => create_character_interactive = true,
            "--gender" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--gender requires male|female")
                })?;
                create_character_male = Some(parse_chargen_gender_arg(&value)?);
            }
            "--chargen-winners" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--chargen-winners requires seven comma-separated virtues",
                    )
                })?;
                create_character_winners = Some(parse_chargen_winners_arg(&value)?);
            }
            "--chargen-answers" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--chargen-answers requires a seven-character A/B string",
                    )
                })?;
                let answers = parse_chargen_answers_arg(&value)?;
                let rng = chargen_default_rng_pool();
                let outcome = run_chargen_tournament(&rng, &answers).map_err(|err| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("chargen tournament failed: {err:?}"),
                    )
                })?;
                let winners: Vec<ShrineVirtue> =
                    outcome.questions.iter().map(|q| q.winner).collect();
                create_character_winners = Some(winners);
            }
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
            "--climbing-gear" | "--grapple" => {
                let option = arg.as_str();
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{option} requires a byte value"),
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
            intro: false,
            visual: false,
            raster_diagnostics: false,
            raster_depth,
            route_smoke: false,
            play_script: None,
            game_dir,
            play_options: PlayOptions::default(),
            help: true,
            save_frame: None,
            save_frame_suite: None,
            create_character: None,
            create_character_interactive: false,
        });
    }
    if from_save && from_init {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--from-save and --from-init are mutually exclusive",
        ));
    }
    if route_smoke
        && (play
            || visual
            || save_frame.is_some()
            || save_frame_suite.is_some()
            || from_save
            || from_init
            || options != PlayOptions::default()
            || wind_override.is_some()
            || climbing_gear_override.is_some()
            || pending_vehicle_override.is_some()
            || transport_override.is_some())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--route-smoke runs its own scripted scenes; it cannot be combined with play, visual, save-frame, from-save, from-init, scene, start, or gameplay overrides",
        ));
    }
    if save_frame_suite.is_some()
        && (play
            || visual
            || save_frame.is_some()
            || route_smoke
            || from_save
            || from_init
            || play_script.is_some()
            || options != PlayOptions::default()
            || wind_override.is_some()
            || climbing_gear_override.is_some()
            || pending_vehicle_override.is_some()
            || transport_override.is_some())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--save-frame-suite runs its own scenes; it cannot be combined with play, visual, route-smoke, save-frame, from-save, from-init, play-script, scene, start, or gameplay overrides",
        ));
    }
    if intro
        && (play
            || save_frame.is_some()
            || save_frame_suite.is_some()
            || route_smoke
            || from_save
            || from_init)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--intro owns the title/menu flow; it cannot be combined with play, route-smoke, save-frame, save-frame-suite, from-save, or from-init",
        ));
    }
    if create_character_interactive
        && (intro
            || play
            || visual
            || save_frame.is_some()
            || save_frame_suite.is_some()
            || route_smoke
            || from_save
            || from_init)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--create-character-interactive writes a save and returns to the intro/menu state; it cannot be combined with intro, play, visual, route-smoke, save-frame, save-frame-suite, from-save, or from-init",
        ));
    }
    if create_character_interactive && create_character_name.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--create-character and --create-character-interactive are mutually exclusive",
        ));
    }

    let create_character = if let Some(name) = create_character_name {
        if intro
            || play
            || visual
            || save_frame.is_some()
            || save_frame_suite.is_some()
            || route_smoke
            || from_save
            || from_init
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--create-character writes a save and returns to the intro/menu state; it cannot be combined with intro, play, visual, route-smoke, save-frame, save-frame-suite, from-save, or from-init",
            ));
        }
        let male = create_character_male.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--create-character requires --gender male|female",
            )
        })?;
        let winners = create_character_winners.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--create-character requires --chargen-winners with seven virtues",
            )
        })?;
        let stats = chargen_stats_from_winners(&winners);
        Some(CreateCharacterCommand {
            name,
            male,
            winners,
            stats,
        })
    } else {
        if create_character_male.is_some() || create_character_winners.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--gender and --chargen-winners require --create-character",
            ));
        }
        None
    };
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
        intro,
        play,
        visual,
        raster_diagnostics,
        raster_depth,
        route_smoke,
        play_script,
        game_dir,
        play_options: options,
        help: false,
        save_frame,
        save_frame_suite,
        create_character,
        create_character_interactive,
    })
}

pub const CLI_USAGE: &str = "\
Ultima V clean-room verification and playable harness.

USAGE:
    cargo run -- [OPTIONS] [GAME_DIR]

GAME_DIR defaults to the local clean asset path. With no flags, runs the
Lord British throne-room verification report.

OPTIONS:
    -h, --help                Print this usage and exit.
        --intro               Launch the terminal title/menu flow.
        --play                Launch the terminal playable harness.
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
        --create-character <N>
                              Create SAVED.GAM/SAVED.OOL from INIT seeds and return.
        --create-character-interactive
                              Run the interactive name/gender/questionnaire flow.
        --gender <G>          male|female for --create-character.
        --chargen-winners <V> Seven comma-separated winning virtues for chargen.
        --raster-diagnostics  Emit per-frame raster diagnostics.
        --route-smoke         Run route-level scripted smoke cases and exit.
        --raster-depth <D>    ega|cga (default ega).
        --visual              Launch the Bevy visual harness.
                              Requires building with `--features visual`.
                              Combine with --intro for the Bevy intro/menu shell.
        --save-frame <PATH>   Render the current viewport (after running
                              --play-script if given) to a PNG and exit.
                              Useful for verifying movement without a desktop.
        --save-frame-suite <DIR>
                              Write representative headless PNG frames plus a
                              sanitized manifest into DIR and exit.

SMOKE COMMANDS:
    cargo run -- C:\\Games\\U5-Clean
    cargo run -- --play C:\\Games\\U5-Clean
    cargo run -- --play-script \"z;q\" C:\\Games\\U5-Clean
    cargo run -- --route-smoke C:\\Games\\U5-Clean
    cargo run -- --save-frame-suite target\\frame-suite C:\\Games\\U5-Clean
    cargo run -- --play --scene DUNGEON:0 --floor 0 C:\\Games\\U5-Clean
    cargo run -- --create-character Avatar --gender male --chargen-winners Honesty,Compassion,Valor,Justice,Sacrifice,Honor,Spirituality C:\\Games\\U5-Clean
    cargo run --features visual -- --visual --scene BRITANNIA C:\\Games\\U5-Clean
    cargo run --features visual -- --visual --scene CASTLE:0 --floor 0 C:\\Games\\U5-Clean
    cargo run --features visual -- --intro --visual C:\\Games\\U5-Clean
";

pub fn parse_chargen_answers_arg(value: &str) -> io::Result<Vec<bool>> {
    let trimmed = value.trim();
    if trimmed.len() != 7 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "--chargen-answers requires exactly seven characters, got {}",
                trimmed.len()
            ),
        ));
    }
    let mut answers = Vec::with_capacity(7);
    for ch in trimmed.chars() {
        match ch.to_ascii_uppercase() {
            'A' => answers.push(true),
            'B' => answers.push(false),
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("--chargen-answers must be A or B, got `{other}`"),
                ));
            }
        }
    }
    Ok(answers)
}

/// Deterministic 64-byte RNG pool fed to the tournament's
/// rejection-sampled virtue picker. Using a fixed pool keeps the CLI
/// command reproducible across invocations.
pub fn chargen_default_rng_pool() -> [u8; 64] {
    let mut bytes = [0u8; 64];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = i as u8;
    }
    bytes
}

pub fn run_create_character_command(
    game_dir: &Path,
    command: &CreateCharacterCommand,
) -> io::Result<ChargenAvatar> {
    commit_chargen_save(game_dir, &command.name, command.male, command.stats)
}

pub fn run_interactive_create_character(game_dir: &Path) -> io::Result<Option<ChargenAvatar>> {
    let records = load_question_records(game_dir)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "QUESTION.DAT is required for interactive character creation",
        )
    })?;
    let mut session = ChargenSession::new(records.records, chargen_interactive_rng_pool())?;
    let mut input = String::new();

    println!("Create New Character");
    loop {
        match session.current_step() {
            ChargenSessionStep::PromptName => {
                print!("By what name shalt thou be known? ");
                io::stdout().flush()?;
                input.clear();
                io::stdin().read_line(&mut input)?;
                match session.submit_name(&input) {
                    ChargenSessionStep::Aborted => {
                        println!("Character creation aborted; returning to the intro menu.");
                        return Ok(None);
                    }
                    ChargenSessionStep::PromptGender => {}
                    _ => println!("Use a nonblank printable ASCII name up to eight characters."),
                }
            }
            ChargenSessionStep::PromptGender => {
                print!("Art thou Male or Female? ");
                io::stdout().flush()?;
                input.clear();
                io::stdin().read_line(&mut input)?;
                if let Some(byte) = input.bytes().next() {
                    if !matches!(
                        session.submit_gender_key(byte),
                        ChargenSessionStep::PresentIntro { .. }
                    ) {
                        println!("Press M or F.");
                    }
                }
            }
            ChargenSessionStep::PresentIntro { text, .. } => {
                println!();
                println!("{text}");
                prompt_continue()?;
                session.advance_intro();
            }
            ChargenSessionStep::PresentQuestion(question) => {
                println!();
                println!(
                    "Question {} of {} (round {})",
                    question.question_index + 1,
                    CHARGEN_QUESTION_COUNT,
                    question.round
                );
                println!("{}", question.text);
                println!(
                    "A: {}    B: {}",
                    question.option_a.name(),
                    question.option_b.name()
                );
                loop {
                    print!("Choose A or B: ");
                    io::stdout().flush()?;
                    input.clear();
                    io::stdin().read_line(&mut input)?;
                    let Some(byte) = input.bytes().next() else {
                        continue;
                    };
                    if !matches!(session.submit_answer_key(byte), ChargenSessionStep::Ignored) {
                        break;
                    }
                }
            }
            ChargenSessionStep::Completed(result) => {
                let avatar = commit_chargen_save(
                    game_dir,
                    &result.entered_name,
                    result.male,
                    result.tournament.stats,
                )?;
                println!("Created character. Choose Journey Onward to load the new save.");
                return Ok(Some(avatar));
            }
            ChargenSessionStep::Aborted => return Ok(None),
            ChargenSessionStep::Ignored => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "character creation reached an invalid state",
                ));
            }
        }
    }
}

fn prompt_continue() -> io::Result<()> {
    print!("Press Enter to continue.");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(())
}

pub fn chargen_interactive_rng_pool() -> Vec<u8> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x5eed_1234);
    let mut state = nanos ^ 0xa5a5_1f2e_3d4c_5b6a;
    let mut bytes = Vec::with_capacity(128);
    for _ in 0..128 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        bytes.push((state >> 32) as u8);
    }
    bytes
}

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

pub fn parse_chargen_gender_arg(value: &str) -> io::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "m" | "male" => Ok(true),
        "f" | "female" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown gender `{value}`; expected male|female"),
        )),
    }
}

pub fn parse_chargen_winners_arg(value: &str) -> io::Result<Vec<ShrineVirtue>> {
    let winners: Vec<_> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            ShrineVirtue::from_key(part).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown chargen virtue `{part}`"),
                )
            })
        })
        .collect::<io::Result<_>>()?;
    if winners.len() != 7 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "--chargen-winners requires exactly seven virtues, got {}",
                winners.len()
            ),
        ));
    }
    Ok(winners)
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
