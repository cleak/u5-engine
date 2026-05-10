pub fn play_script_idle_tick_count(command: &str) -> io::Result<Option<usize>> {
    let command = command.trim();
    let lower = command.to_ascii_lowercase();
    if matches!(lower.as_str(), "idle" | "tick" | "ticks") {
        return Ok(Some(1));
    }
    let Some(value) = lower
        .strip_prefix("idle:")
        .or_else(|| lower.strip_prefix("tick:"))
    else {
        return Ok(None);
    };
    let count = value.parse::<usize>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("script idle command `{command}` has invalid tick count: {err}"),
        )
    })?;
    if count == 0 || count > PLAY_SCRIPT_MAX_IDLE_TICKS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "script idle command `{command}` tick count must be 1..{PLAY_SCRIPT_MAX_IDLE_TICKS}"
            ),
        ));
    }
    Ok(Some(count))
}

pub fn split_play_script(script: &str) -> Vec<String> {
    script
        .split(';')
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn play_script_command_label(command: &str) -> String {
    if command.is_empty() {
        return "empty".to_string();
    }
    if let Some(function_key) = ansi_function_key(command.trim()) {
        return format!("F{function_key}");
    }
    let mut label = String::new();
    for ch in command.chars() {
        if ch.is_control() {
            label.push_str(&format!("\\x{:02x}", ch as u32));
        } else {
            label.push(ch);
        }
    }
    label
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
    let mut args = args.into_iter().map(Into::into);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--play" => play = true,
            "--visual" => visual = true,
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

SMOKE COMMANDS:
    cargo run -- C:\\Games\\U5-Clean
    cargo run -- --play C:\\Games\\U5-Clean
    cargo run -- --play-script \"z;q\" C:\\Games\\U5-Clean
    cargo run -- --play --scene DUNGEON:0 --floor 0 C:\\Games\\U5-Clean
    cargo run --features visual -- --visual --scene BRITANNIA C:\\Games\\U5-Clean
    cargo run --features visual -- --visual --scene CASTLE:0 --floor 0 C:\\Games\\U5-Clean
";

pub fn load_play_options_from_save(game_dir: &Path) -> io::Result<PlayOptions> {
    let mut options = load_play_options_from_save_file(game_dir, "SAVED.GAM", "--from-save", true)?;
    refresh_saved_ool_mirrors_for_load(game_dir)?;
    options.save_template_source = SaveTemplateSource::SavedGame;
    Ok(options)
}

pub fn load_play_options_from_init(game_dir: &Path) -> io::Result<PlayOptions> {
    let mut options = load_play_options_from_save_file(game_dir, "INIT.GAM", "--from-init", false)?;
    options.initial_britannia_overlay = Some(load_init_overlay_objects(game_dir)?);
    options.save_template_source = SaveTemplateSource::InitGame;
    Ok(options)
}

pub fn load_save_image_template(game_dir: &Path, source: SaveTemplateSource) -> io::Result<Vec<u8>> {
    match source {
        SaveTemplateSource::SavedGame => {
            read_save_image_file(&game_dir.join("SAVED.GAM"), "SAVED.GAM")
        }
        SaveTemplateSource::InitGame => {
            read_save_image_file(&game_dir.join("INIT.GAM"), "INIT.GAM")
        }
        SaveTemplateSource::PreferSavedGame => {
            let saved = game_dir.join("SAVED.GAM");
            if saved.exists() {
                return read_save_image_file(&saved, "SAVED.GAM");
            }
            let init = game_dir.join("INIT.GAM");
            if init.exists() {
                return read_save_image_file(&init, "INIT.GAM");
            }
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "saving requires an existing SAVED.GAM or INIT.GAM template",
            ))
        }
    }
}

pub fn read_save_image_file(path: &Path, file_name: &str) -> io::Result<Vec<u8>> {
    let bytes = read(path)?;
    if bytes.len() != SAVED_GAM_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{file_name} must be {SAVED_GAM_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

pub fn load_play_options_from_save_file(
    game_dir: &Path,
    file_name: &str,
    option_name: &str,
    include_active_objects: bool,
) -> io::Result<PlayOptions> {
    let bytes = read(&game_dir.join(file_name))?;
    play_options_from_save_bytes_named(&bytes, file_name, option_name, include_active_objects)
}

#[cfg(test)]
pub fn play_options_from_save_bytes(bytes: &[u8]) -> io::Result<PlayOptions> {
    play_options_from_save_bytes_named(bytes, "SAVED.GAM", "--from-save", true)
}

pub fn play_options_from_save_bytes_named(
    bytes: &[u8],
    file_name: &str,
    option_name: &str,
    include_active_objects: bool,
) -> io::Result<PlayOptions> {
    if bytes.len() != SAVED_GAM_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{file_name} must be {SAVED_GAM_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    let _avatar_name_present = saved_game_has_avatar_name(bytes);
    let scene_byte = bytes[SAVE_SCENE_OFFSET];
    if scene_byte > 40 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{option_name} currently supports overworld, town-family, or stock dungeon scenes only; scene is {scene_byte}"
            ),
        ));
    }
    let z = bytes[SAVE_Z_OFFSET];
    let x = bytes[SAVE_X_OFFSET] as usize;
    let y = bytes[SAVE_Y_OFFSET] as usize;
    let (target, floor) = if scene_byte == 0 {
        if x >= WORLD_SIDE || y >= WORLD_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved world position must be inside 0..255, got ({x}, {y})"),
            ));
        }
        let plane = WorldPlane::from_save_z(z);
        (PlayTarget::World(plane), plane.save_floor())
    } else if scene_byte <= 32 {
        if x >= 32 || y >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved town position must be inside 0..31, got ({x}, {y})"),
            ));
        }
        (PlayTarget::Town(Scene::new(scene_byte)?), z as i8)
    } else {
        if z > 7 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved dungeon level must be inside 0..7, got {z}"),
            ));
        }
        if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("saved dungeon position must be inside 0..7, got ({x}, {y})"),
            ));
        }
        (PlayTarget::Dungeon(DungeonScene::new(scene_byte)?), z as i8)
    };

    let mut spell_charges = [0; SPELL_COUNT];
    spell_charges.copy_from_slice(
        &bytes[SAVE_SPELL_CHARGES_OFFSET..SAVE_SPELL_CHARGES_OFFSET + SPELL_COUNT],
    );
    let moonstone_slots = decode_moonstone_gate_slots(bytes);
    let reagents = decode_reagent_stock(bytes);
    let avatar_stats = decode_avatar_stats(bytes);

    Ok(PlayOptions {
        target,
        floor,
        start: Some((x, y)),
        clock: GameClock::with_date(
            u16_at(bytes, SAVE_YEAR_OFFSET),
            bytes[SAVE_MONTH_OFFSET],
            bytes[SAVE_DAY_OFFSET],
            bytes[SAVE_HOUR_OFFSET],
            bytes[SAVE_MINUTE_OFFSET],
        )?,
        food: u16_at(bytes, SAVE_FOOD_STOCK_OFFSET),
        gold: u16_at(bytes, SAVE_GOLD_STOCK_OFFSET),
        keys: bytes[SAVE_KEY_STOCK_OFFSET],
        gems: bytes[SAVE_GEM_STOCK_OFFSET],
        climbing_gear: DEFAULT_CLIMBING_GEAR,
        party: decode_save_party(bytes),
        spell_charges,
        reagents,
        moonstone_slots,
        shrine_ordained_mask: bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET],
        shrine_codex_mask: bytes[SAVE_SHRINE_CODEX_MASK_OFFSET],
        shrine_standing: [0; VIRTUE_COUNT],
        avatar_stats,
        torches: bytes[SAVE_TORCH_STOCK_OFFSET],
        torch_counter: bytes[SAVE_TORCH_COUNTER_OFFSET],
        light_spell_counter: bytes[SAVE_LIGHT_SPELL_COUNTER_OFFSET],
        wind: WindState::from_save_byte(bytes[SAVE_WIND_OFFSET]),
        wind_save_byte: bytes[SAVE_WIND_OFFSET],
        timing_status: TimingStatusTag::from_save_byte(bytes[SAVE_TIMING_STATUS_TAG_OFFSET]),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        transport: transport_from_save_marker(bytes[SAVE_TRANSPORT_MARKER_OFFSET]),
        pending_vehicle: None,
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: if include_active_objects {
            Some(decode_saved_active_objects(bytes)?)
        } else {
            None
        },
        save_template_source: SaveTemplateSource::PreferSavedGame,
    })
}

pub fn decode_reagent_stock(bytes: &[u8]) -> [u8; REAGENT_COUNT] {
    let mut reagents = [0; REAGENT_COUNT];
    for (save_index, recipe_index) in REAGENT_SAVE_ORDER.iter().copied().enumerate() {
        reagents[recipe_index] = bytes[SAVE_REAGENTS_OFFSET + save_index];
    }
    reagents
}

pub fn encode_reagent_stock(bytes: &mut [u8], reagents: [u8; REAGENT_COUNT]) {
    for (save_index, recipe_index) in REAGENT_SAVE_ORDER.iter().copied().enumerate() {
        bytes[SAVE_REAGENTS_OFFSET + save_index] = reagents[recipe_index];
    }
}

pub fn decode_avatar_stats(bytes: &[u8]) -> AvatarStats {
    let avatar_record = SAVE_ROSTER_OFFSET;
    AvatarStats {
        strength: bytes[avatar_record + SAVE_CHARACTER_STR_OFFSET],
        dexterity: bytes[avatar_record + SAVE_CHARACTER_DEX_OFFSET],
        intelligence: bytes[avatar_record + SAVE_CHARACTER_INT_OFFSET],
    }
}

pub fn decode_save_party(bytes: &[u8]) -> Vec<PartyMember> {
    let party_size = bytes[SAVE_PARTY_SIZE_OFFSET] as usize;
    if !(1..=6).contains(&party_size) {
        return default_party();
    }

    (0..party_size)
        .map(|slot| {
            let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
            let status = bytes[record + SAVE_CHARACTER_STATUS_OFFSET];
            let climb_stat = bytes[record + SAVE_CHARACTER_DEX_OFFSET];
            let mana = bytes[record + SAVE_CHARACTER_MANA_OFFSET];
            let hp = u16_at(bytes, record + SAVE_CHARACTER_HP_OFFSET);
            let max_hp = u16_at(bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET);
            let level = bytes[record + SAVE_CHARACTER_LEVEL_OFFSET];
            PartyMember {
                slot: slot as u8,
                status,
                climb_stat,
                mana,
                hp,
                max_hp,
                level,
            }
        })
        .collect()
}

pub fn decode_moonstone_gate_slots(bytes: &[u8]) -> [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT] {
    let mut slots = [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT];
    for (slot_index, slot) in slots.iter_mut().enumerate() {
        *slot = MoonstoneGateSlot {
            x: bytes[SAVE_MOONSTONE_X_OFFSET + slot_index],
            y: bytes[SAVE_MOONSTONE_Y_OFFSET + slot_index],
            scene: bytes[SAVE_MOONSTONE_SCENE_OFFSET + slot_index],
            z: bytes[SAVE_MOONSTONE_Z_OFFSET + slot_index],
        };
    }
    slots
}

pub fn saved_game_has_avatar_name(bytes: &[u8]) -> bool {
    bytes
        .get(SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + SAVE_AVATAR_NAME_LEN)
        .is_some_and(|name| name.iter().any(|byte| *byte != 0))
}

pub fn parse_u8_literal(value: &str) -> io::Result<u8> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16)
    } else {
        trimmed.parse::<u8>()
    };
    parsed.map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid byte literal `{value}`: {err}"),
        )
    })
}

pub fn parse_i8_literal(value: &str) -> io::Result<i8> {
    value.trim().parse::<i8>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid signed byte literal `{value}`: {err}"),
        )
    })
}

pub fn parse_cardinal_direction(value: &str) -> io::Result<Direction> {
    match value.trim().to_ascii_lowercase().as_str() {
        "north" | "n" => Ok(Direction::North),
        "east" | "e" => Ok(Direction::East),
        "south" | "s" => Ok(Direction::South),
        "west" | "w" => Ok(Direction::West),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("direction must be north, east, south, or west, got `{value}`"),
        )),
    }
}

pub fn parse_inline_hours(value: &str) -> Option<u8> {
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u8>().ok()
    }
}

pub fn moonstone_phase_from_inline_number(value: u8) -> Option<usize> {
    (1..=MOONSTONE_SLOT_COUNT as u8)
        .contains(&value)
        .then_some(value as usize - 1)
}

pub fn parse_inline_use_request(value: &str) -> Option<UseItemRequest> {
    let token = value.chars().find(|ch| !ch.is_whitespace())?;
    match token.to_ascii_uppercase() {
        'T' | 'I' => Some(UseItemRequest::Torch),
        'G' | 'V' => Some(UseItemRequest::Gem),
        'K' | 'J' => Some(UseItemRequest::Key),
        '1'..='8' => token
            .to_digit(10)
            .and_then(|digit| moonstone_phase_from_inline_number(digit as u8))
            .map(UseItemRequest::Moonstone),
        _ => Some(UseItemRequest::Invalid),
    }
}

pub fn parse_inline_cardinal_direction(value: &str) -> Option<Direction> {
    value.chars().rev().find_map(|ch| match ch {
        '8' => Some(Direction::North),
        '6' => Some(Direction::East),
        '2' => Some(Direction::South),
        '4' => Some(Direction::West),
        _ => None,
    })
}

pub fn parse_inline_yes_no(value: &str) -> Option<bool> {
    value.chars().find_map(|ch| match ch.to_ascii_lowercase() {
        'y' => Some(true),
        'n' => Some(false),
        _ => None,
    })
}

pub fn parse_inline_party_index(value: &str) -> Option<usize> {
    value
        .chars()
        .find_map(|ch| ch.to_digit(10))
        .and_then(|digit| usize::try_from(digit).ok())
        .map(|digit| digit.saturating_sub(1))
}

pub fn parse_inline_target_party_index(value: &str) -> Option<usize> {
    value
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .nth(1)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| digit.checked_sub(1))
}

pub fn parse_inline_party_swap(value: &str) -> Option<(usize, usize)> {
    let mut digits = value.chars().filter_map(|ch| ch.to_digit(10));
    let first = digits.next()?;
    let second = digits.next()?;
    if first == 0 || second == 0 {
        return None;
    }
    Some(((first - 1) as usize, (second - 1) as usize))
}

pub fn parse_inline_gate_phase_index(value: &str) -> Option<usize> {
    value
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .nth(1)
        .and_then(|digit| usize::try_from(digit).ok())
        .and_then(|digit| {
            (1..=MOONSTONE_SLOT_COUNT)
                .contains(&digit)
                .then_some(digit - 1)
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineMixRequest {
    pub spell_index: Option<usize>,
    pub reagent_mask: u8,
    pub amount: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineShrineRequest {
    pub mantra: String,
    pub offering: Option<u8>,
}

pub fn parse_inline_mix_request(value: &str) -> io::Result<Option<InlineMixRequest>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parts: Vec<_> = trimmed
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Mix syntax is M<spell>/<reagent-mask>/<quantity>, for example MIL/0x80/1.",
        ));
    }
    let spell_code = inline_spell_code(parts[0]);
    if spell_code.is_empty() {
        return Ok(None);
    }
    let reagent_mask = parse_u8_literal(parts[1]).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("invalid mix reagent mask `{}`: {err}", parts[1]),
        )
    })?;
    let amount = parse_u8_literal(parts[2]).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("invalid mix quantity `{}`: {err}", parts[2]),
        )
    })?;
    Ok(Some(InlineMixRequest {
        spell_index: spell_index_from_code(&spell_code),
        reagent_mask,
        amount,
    }))
}

pub fn inline_mix_candidate(value: &str) -> bool {
    let parts: Vec<_> = value
        .trim()
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    parts.len() == 3 && !inline_spell_code(parts[0]).is_empty()
}

pub fn parse_inline_shrine_request(value: &str) -> io::Result<Option<InlineShrineRequest>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parts: Vec<_> = trimmed
        .split(|ch| matches!(ch, '/' | ':' | ','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Ok(None);
    }
    if parts.len() > 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Shrine syntax is M<mantra> or M<mantra>/<offering-digit>.",
        ));
    }
    let mantra = parts[0].to_string();
    if mantra.is_empty() {
        return Ok(None);
    }
    let offering = if let Some(offering) = parts.get(1) {
        if offering.len() != 1 || !offering.as_bytes()[0].is_ascii_digit() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Shrine offering must be a digit 0 through 9.",
            ));
        }
        Some(offering.as_bytes()[0] - b'0')
    } else {
        None
    };
    Ok(Some(InlineShrineRequest { mantra, offering }))
}

pub fn mix_prompt_message() -> String {
    "Mix what? Use M<spell>/<reagent-mask>/<quantity>, for example MIL/0x80/1.".to_string()
}

pub fn shrine_prompt_message(virtue: ShrineVirtue) -> String {
    format!(
        "Meditate at the Shrine of {}? Use M{} or M{}/<offering-digit>.",
        virtue.name(),
        virtue.mantra(),
        virtue.mantra()
    )
}

pub fn cast_prompt_message() -> String {
    "Cast what? Use C1IL/C1AZ2/C1AN2/C1M2/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS/C1LV/C1HR/C1IP6/C1PU/C1DP/C1AG6/C1AEP/C1EIP/C1IQW/C1AWY/C1PRV2/C1AT."
        .to_string()
}

pub fn use_prompt_message() -> String {
    "Use what? Use UT for torch, UG for gem, UK for key, or U1 through U8 for Moonstone phase."
        .to_string()
}

pub fn new_order_prompt_message() -> String {
    "New order? Use N12 to swap party slots 1 and 2.".to_string()
}

pub fn inline_spell_code(value: &str) -> String {
    let mut letters: Vec<_> = value
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_uppercase())
        .collect();
    letters.sort_unstable();
    letters.into_iter().collect()
}

pub fn spell_index_from_code(code: &str) -> Option<usize> {
    SPELL_CODES.iter().position(|known| *known == code)
}

pub fn spell_scene_bit_for_area(area: Area) -> u8 {
    match area {
        Area::World { .. } => SPELL_SCENE_OVERWORLD,
        Area::Town { .. } => SPELL_SCENE_INDOOR,
        Area::Dungeon { .. } => SPELL_SCENE_DUNGEON,
    }
}

pub fn spell_allowed_in_area(spell_index: usize, area: Area) -> bool {
    SPELL_SCENE_MASKS[spell_index] & spell_scene_bit_for_area(area) != 0
}

pub fn selected_reagent_indices(mask: u8) -> Vec<usize> {
    REAGENT_MASKS
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, bit)| (mask & bit != 0).then_some(index))
        .collect()
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

