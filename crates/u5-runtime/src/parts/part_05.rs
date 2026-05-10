#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayOptions {
    pub target: PlayTarget,
    pub floor: i8,
    pub start: Option<(usize, usize)>,
    pub clock: GameClock,
    pub food: u16,
    pub gold: u16,
    pub keys: u8,
    pub gems: u8,
    pub climbing_gear: u8,
    pub party: Vec<PartyMember>,
    pub spell_charges: [u8; SPELL_COUNT],
    pub reagents: [u8; REAGENT_COUNT],
    pub moonstone_slots: [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT],
    pub shrine_ordained_mask: u8,
    pub shrine_codex_mask: u8,
    pub shrine_standing: [u8; VIRTUE_COUNT],
    pub avatar_stats: AvatarStats,
    pub torches: u8,
    pub torch_counter: u8,
    pub light_spell_counter: u8,
    pub wind: WindState,
    pub wind_save_byte: u8,
    pub timing_status: TimingStatusTag,
    pub time_stop_counter: u8,
    pub active_effect_tag: Option<u8>,
    pub active_effect_counter: u8,
    pub transport: TransportState,
    pub pending_vehicle: Option<PendingVehicleAcquisition>,
    pub initial_britannia_overlay: Option<Vec<ActiveObject>>,
    pub debug_enter: Option<PlayTarget>,
    pub saved_active_objects: Option<Vec<ActiveObject>>,
    pub save_template_source: SaveTemplateSource,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            target: PlayTarget::Town(
                Scene::new(0x11).expect("default Lord British castle scene is valid"),
            ),
            floor: 0,
            start: None,
            clock: GameClock::default(),
            food: DEFAULT_FOOD_STOCK,
            gold: DEFAULT_GOLD_STOCK,
            keys: DEFAULT_KEY_STOCK,
            gems: DEFAULT_GEM_STOCK,
            climbing_gear: DEFAULT_CLIMBING_GEAR,
            party: default_party(),
            spell_charges: [0; SPELL_COUNT],
            reagents: DEFAULT_REAGENTS,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            shrine_standing: [0; VIRTUE_COUNT],
            avatar_stats: AvatarStats::default(),
            torches: DEFAULT_TORCH_STOCK,
            torch_counter: 0,
            light_spell_counter: 0,
            wind: WindState::default(),
            wind_save_byte: 0,
            timing_status: TimingStatusTag::default(),
            time_stop_counter: 0,
            active_effect_tag: None,
            active_effect_counter: 0,
            transport: TransportState::Foot,
            pending_vehicle: None,
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: None,
            save_template_source: SaveTemplateSource::PreferSavedGame,
        }
    }
}

pub fn initial_world_overlay_cache(options: &PlayOptions) -> WorldOverlayCache {
    let mut overlays = WorldOverlayCache::default();
    if let Some(objects) = options.initial_britannia_overlay.clone() {
        overlays.set(WorldPlane::Britannia, objects);
    }
    overlays
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateTravelDestination {
    Ready {
        target: PlayTarget,
        floor: i8,
        start: (usize, usize),
    },
    Empty,
    Invalid(String),
}

pub fn gate_travel_destination(slot: MoonstoneGateSlot) -> GateTravelDestination {
    if !slot.is_valid() {
        return GateTravelDestination::Empty;
    }

    let x = slot.x as usize;
    let y = slot.y as usize;
    match slot.scene {
        0 => {
            let plane = WorldPlane::from_save_z(slot.z);
            GateTravelDestination::Ready {
                target: PlayTarget::World(plane),
                floor: plane.save_floor(),
                start: (x, y),
            }
        }
        1..=32 => {
            if x >= 32 || y >= 32 {
                return GateTravelDestination::Invalid(format!(
                    "town position must be inside 0..31, got ({x}, {y})"
                ));
            }
            match Scene::new(slot.scene) {
                Ok(scene) => GateTravelDestination::Ready {
                    target: PlayTarget::Town(scene),
                    floor: slot.z as i8,
                    start: (x, y),
                },
                Err(err) => GateTravelDestination::Invalid(err.to_string()),
            }
        }
        33..=40 => {
            if slot.z > 7 {
                return GateTravelDestination::Invalid(format!(
                    "dungeon level must be inside 0..7, got {}",
                    slot.z
                ));
            }
            if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
                return GateTravelDestination::Invalid(format!(
                    "dungeon position must be inside 0..7, got ({x}, {y})"
                ));
            }
            match DungeonScene::new(slot.scene) {
                Ok(scene) => GateTravelDestination::Ready {
                    target: PlayTarget::Dungeon(scene),
                    floor: slot.z as i8,
                    start: (x, y),
                },
                Err(err) => GateTravelDestination::Invalid(err.to_string()),
            }
        }
        scene => GateTravelDestination::Invalid(format!("unsupported scene {scene}")),
    }
}

pub fn moonstone_slot_matches_world(
    slot: MoonstoneGateSlot,
    plane: WorldPlane,
    x: usize,
    y: usize,
) -> bool {
    slot.scene == 0
        && WorldPlane::from_save_z(slot.z) == plane
        && slot.x as usize == x
        && slot.y as usize == y
}

pub fn moonstone_slot_matches_town(
    slot: MoonstoneGateSlot,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
) -> bool {
    slot.scene == scene.byte
        && slot.z as i8 == floor
        && slot.x as usize == x
        && slot.y as usize == y
}

pub fn moonstone_bury_tile_allowed(tile: u8) -> bool {
    matches!(tile, 4..=10 | 44 | 45)
}

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveOutcome {
    Moved,
    Blocked,
    Boarded,
    ExitedVehicle,
    SailToggled,
    SailStalled,
    Fired,
    Pushed,
    Rested,
    Talked,
    Ignited,
    Cast,
    DoorOpened,
    ContainerOpened,
    Got,
    Used,
    LockTried,
    Observed,
    Searched,
    IdleTick,
    PromptDeclined,
    Passed,
    Saved,
    Transition(AreaTransition),
}

impl MoveOutcome {
    fn is_transition(self) -> bool {
        matches!(self, Self::Transition(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseItemRequest {
    Torch,
    Gem,
    Key,
    Moonstone(usize),
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaTransition {
    ChangedFloor {
        scene: Scene,
        floor: i8,
    },
    ChangedDungeonLevel {
        scene: DungeonScene,
        level: u8,
    },
    ChangedWorldPlane {
        from: WorldPlane,
        to: WorldPlane,
    },
    MoongateTeleported {
        from: WorldPlane,
        to: WorldPlane,
    },
    GateTraveled {
        target: PlayTarget,
    },
    EnteredLocation(Scene),
    EnteredDungeon(DungeonScene),
    ExitedLocation(Scene),
    ExitedDungeon(DungeonScene),
    ExitedDungeonToWorldPlane {
        scene: DungeonScene,
        plane: WorldPlane,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClimbIntent {
    Up,
    Down,
}

#[derive(Debug)]
pub struct NpcSlot {
    pub slot: usize,
    pub type_byte: u8,
    pub dialog_id: u8,
    pub schedule: [u8; 16],
    pub name: Option<String>,
}

#[derive(Debug)]
pub struct MapStats {
    pub scene: Scene,
    pub floor: usize,
    pub npc_markers: Vec<(usize, usize)>,
    pub spawn_markers: Vec<(usize, usize)>,
    pub door_count: usize,
    pub stair_count: usize,
    pub render_hash: u64,
    pub class_histogram: HashMap<&'static str, usize>,
}

pub fn run() -> io::Result<()> {
    let args = parse_cli_args(env::args().skip(1))?;
    if args.help {
        print!("{CLI_USAGE}");
        return Ok(());
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

pub fn run_report(game_dir: &Path) -> io::Result<()> {
    let mut report = String::new();
    report.push_str("# Lord British throne-room verification slice\n\n");
    report.push_str(&format!("Game data: `{}`\n\n", game_dir.display()));
    report.push_str("This executable is a parity harness. It reads original data at runtime but does not embed or emit raw map, dialogue, or asset dumps.\n\n");

    let lb_candidate = Scene::new(0x11)?; // CASTLE:0 by public scene partition.
    let fifth_castle = Scene::new(0x15)?; // CASTLE:4, the disputed public wording.
    let decomp_special = Scene::new(0x1d)?; // Scene identified by private TOWN note.

    let castle_tlk = parse_tlk(&game_dir.join("CASTLE.TLK"))?;
    let keep_tlk = parse_tlk(&game_dir.join("KEEP.TLK"))?;

    let lb_slots = parse_npc_block(&game_dir, lb_candidate, &castle_tlk)?;
    let fifth_slots = parse_npc_block(&game_dir, fifth_castle, &castle_tlk)?;
    let special_slots = parse_npc_block(&game_dir, decomp_special, &keep_tlk)?;

    let lb_names = names(&lb_slots);
    let fifth_names = names(&fifth_slots);
    let special_names = names(&special_slots);

    let lb_has_castle_staff = contains_all(
        &lb_names,
        &[
            "Alistair", "Stephen", "Treanna", "Margaret", "Desiree", "Saduj",
        ],
    );
    let fifth_has_castle_staff = contains_any(&fifth_names, &["Alistair", "Stephen", "Saduj"]);
    let special_is_keep = decomp_special.family == Family::Keep;

    report.push_str("## Scene binding checks\n\n");
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        lb_candidate.byte,
        lb_candidate.key()
    ));
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        fifth_castle.byte,
        fifth_castle.key()
    ));
    report.push_str(&format!(
        "- Scene `0x{:02X}` resolves by public partition to `{}`.\n",
        decomp_special.byte,
        decomp_special.key()
    ));
    report.push_str(&format!(
        "- `CASTLE:0` contains Lord-British-castle staff markers: {}.\n",
        pass_fail(lb_has_castle_staff)
    ));
    report.push_str(&format!(
        "- `CASTLE:4` contains those staff markers: {}.\n",
        pass_fail(fifth_has_castle_staff)
    ));
    report.push_str(&format!(
        "- Private-note special scene `0x1D` maps to keep family under the public partition: {}.\n\n",
        pass_fail(special_is_keep)
    ));

    report.push_str("Representative roster names, limited to avoid dialogue or roster dumps:\n\n");
    report.push_str(&format!(
        "- `{}`: {}\n",
        lb_candidate.key(),
        sample_names(&lb_names)
    ));
    report.push_str(&format!(
        "- `{}`: {}\n",
        fifth_castle.key(),
        sample_names(&fifth_names)
    ));
    report.push_str(&format!(
        "- `{}`: {}\n\n",
        decomp_special.key(),
        sample_names(&special_names)
    ));

    let floor0 = load_floor(&game_dir, lb_candidate, 0)?;
    let floor1 = load_floor(&game_dir, lb_candidate, 1)?;
    let stats0 = analyze_map(lb_candidate, 0, &floor0);
    let stats1 = analyze_map(lb_candidate, 1, &floor1);

    report.push_str("## Map/render checks\n\n");
    append_map_stats(&mut report, &stats0);
    append_map_stats(&mut report, &stats1);

    let start = harvest_location_markers(&floor0)
        .spawn_markers
        .first()
        .copied()
        .or_else(|| first_walkable(&floor0, None))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no walkable floor-0 start"))?;
    let target = stats0
        .npc_markers
        .first()
        .copied()
        .filter(|target| *target != start)
        .or_else(|| first_distinct_walkable(&floor0, start))
        .or_else(|| first_walkable(&floor0, None))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no floor-0 target"))?;
    let path = find_path(&floor0, start, target);

    report.push_str("## Movement/pathfinding checks\n\n");
    report.push_str(&format!(
        "- Movement probe start: ({}, {}), target: ({}, {}).\n",
        start.0, start.1, target.0, target.1
    ));
    match &path {
        Some(steps) => {
            report.push_str(&format!(
                "- Class-derived pathfinding found a path of {} steps: PASS.\n",
                steps.len().saturating_sub(1)
            ));
            let legal = steps.windows(2).all(|w| {
                manhattan(w[0], w[1]) == 1 && is_probe_walkable(floor0[w[1].1 * 32 + w[1].0])
            });
            report.push_str(&format!(
                "- Simulated step-by-step movement over the path: {}.\n",
                pass_fail(legal)
            ));
        }
        None => {
            report.push_str("- Class-derived pathfinding found no path. This is a WARNING, not a hard failure, because exact passability bitmap placement is still open in the public specs.\n");
        }
    }
    report.push_str(&format!(
        "- Door-family tiles detected on tested floors: {}.\n\n",
        stats0.door_count + stats1.door_count
    ));
    match door_probe(&floor1) {
        Some((pos, opened_walkable)) => {
            report.push_str(&format!(
                "- Door interaction smoke probe at ({}, {}) rewrote a door-family cell and produced a walkable result: {}.\n\n",
                pos.0,
                pos.1,
                pass_fail(opened_walkable)
            ));
        }
        None => report.push_str(
            "- Door interaction smoke probe: WARNING, no door-family tile found on floor 1.\n\n",
        ),
    }

    report.push_str("## Schedule/conversation checks\n\n");
    let occupied = lb_slots.iter().filter(|s| s.type_byte != 0).count();
    let named = lb_slots.iter().filter(|s| s.name.is_some()).count();
    report.push_str(&format!(
        "- Occupied `CASTLE:0` roster slots: {occupied}.\n"
    ));
    report.push_str(&format!(
        "- Occupied slots with resolved TLK display names: {named}.\n"
    ));
    report.push_str("- Noon waypoint sample:\n");
    for slot in lb_slots.iter().filter(|s| s.type_byte != 0).take(6) {
        let wp = waypoint_for_hour(&slot.schedule, 12);
        let name = slot.name.as_deref().unwrap_or("(unnamed)");
        report.push_str(&format!(
            "  - slot {} dlg {} `{}` -> waypoint {} at ({}, {}, {}).\n",
            slot.slot,
            slot.dialog_id,
            name,
            wp,
            slot.schedule[3 + wp],
            slot.schedule[6 + wp],
            slot.schedule[9 + wp] as i8
        ));
    }
    if let Some(slot) = lb_slots
        .iter()
        .find(|slot| slot.type_byte != 0 && slot.dialog_id > 1 && slot.name.is_some())
    {
        let fields = castle_tlk
            .get(&(slot.dialog_id as u16))
            .map(|fields| fields.len())
            .unwrap_or(0);
        let keywords = fields.saturating_sub(5) / 2;
        report.push_str(&format!(
            "- Conversation envelope probe: slot {} dlg {} has the five leading TLK fields and {} keyword pairs: {}.\n",
            slot.slot,
            slot.dialog_id,
            keywords,
            pass_fail(fields >= 5)
        ));
    } else {
        report.push_str(
            "- Conversation envelope probe: FAIL, no named dialogue-bearing slot found.\n",
        );
    }
    report.push_str("\n");

    report.push_str("## Findings\n\n");
    report.push_str("- The slice runs end-to-end for file loading, scene partitioning, roster/TLK joins, map analysis, render hashing, schedule sampling, and pathfinding smoke checks.\n");
    report.push_str("- `CASTLE:0`, not the fifth castle slot, is the strongest data-backed public binding for Lord British's castle in this slice.\n");
    report.push_str("- The private TOWN note's `0x1D` special-case label conflicts with the public scene partition and should be treated as an unresolved private-analysis issue until rechecked.\n");
    report.push_str("- The aggregate report keeps its class-derived smoke path; runtime play can also consume an optional clean-room passability bitmap.\n");

    if !lb_has_castle_staff || !special_is_keep {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "required scene binding check failed",
        ));
    }

    fs::create_dir_all("reports")?;
    fs::write(REPORT_PATH, &report)?;
    print!("{report}");
    println!("\nReport written to {REPORT_PATH}");
    Ok(())
}

pub fn run_play_loop(
    game_dir: &Path,
    options: PlayOptions,
    raster_diagnostics: bool,
    raster_depth: TileGraphicsDepth,
    play_script: Option<Vec<String>>,
) -> io::Result<()> {
    let intro_target = options.target;
    let intro_floor = options.floor;
    let mut state = PlayState::load_scene(game_dir, options)?;
    let tile_atlas = if raster_diagnostics {
        Some(load_tile_atlas(game_dir, raster_depth)?)
    } else {
        None
    };
    println!("Ultima V first-playable slice");
    println!(
        "Scene {} floor/level {}. Town/world move: numpad 1-9 or lowercase wasd/yubn. Dungeon: W/S forward/back, A/D turn. Enter: e. Open: o. Push: p. Hole up: h+hours. Look: l; fountain drink lY/lN or l2Y. View: v. Use: UT/UG/UK/U1-U8. Stats: Z. Ignite: i. Talk: t or TKEYWORD. Climb: k/< />. Board/Xit/Yell sails: B/X/x/Y. Fire: f or f+dir. Cast: C1IL/C1AZ2/C1AN2/C1M2/C1MV2/C1CIM2/C1IS/C1RT/C1AI/C1IW/C1IMX/C1AS/C1LV/C1HR/C1IP6/C1IQW/C1AWY/C1PU/C1DP/C1AG6/C1FGI6/C1GIN6/C1GIS6/C1AEP/C1EIP/C1PRV2/C1AT. Mix: MIL/0x80/1. Order: N12. Top-down save: QY/QN. Dungeon exit prompt: QY/QN. Buffer/typeahead: buffer. Idle animation: . Optional startup wind/gear/transport/raster diagnostics: --wind, --climbing-gear, --transport, --raster-diagnostics, --raster-depth ega|cga. Pass: Space/Enter. Harness quit: q.",
        intro_target.key(),
        intro_floor
    );
    if let Some(commands) = play_script {
        println!("Script mode: {} command(s).", commands.len());
        return run_play_script_commands(&mut state, game_dir, &commands, tile_atlas.as_ref());
    }
    let mut input = String::new();
    let mut queued_input = VecDeque::new();
    loop {
        print_play_frame(&mut state, tile_atlas.as_ref())?;
        let (key, suffix) = if let Some(key) = queued_input.pop_front() {
            (key, String::new())
        } else {
            print!("> ");
            io::stdout().flush()?;
            input.clear();
            if io::stdin().read_line(&mut input)? == 0 {
                break;
            }
            if state.typeahead_buffer_enabled {
                if let Some(keys) = play_input_typeahead_chars(&input) {
                    let mut keys = keys.into_iter();
                    let key = keys.next().expect("typeahead input is non-empty");
                    queued_input.extend(keys);
                    (key, String::new())
                } else if let Some((key, suffix)) = play_input_key_and_suffix(&input) {
                    (key, suffix)
                } else {
                    handle_empty_play_input(&mut state, game_dir)?;
                    continue;
                }
            } else if let Some((key, suffix)) = play_input_key_and_suffix(&input) {
                (key, suffix)
            } else {
                handle_empty_play_input(&mut state, game_dir)?;
                continue;
            }
        };
        if handle_play_key_input(&mut state, key, &suffix, game_dir)? == PlayInputDisposition::Quit
        {
            break;
        }
    }
    Ok(())
}

pub fn run_play_script_commands(
    state: &mut PlayState,
    game_dir: &Path,
    commands: &[String],
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    print_play_script_snapshot(state, tile_atlas)?;
    for (index, command) in commands.iter().enumerate() {
        println!(
            "script[{}]: {}",
            index + 1,
            play_script_command_label(command)
        );
        let disposition = handle_play_script_command(state, command, game_dir)?;
        print_play_script_snapshot(state, tile_atlas)?;
        if disposition == PlayInputDisposition::Quit {
            break;
        }
    }
    Ok(())
}

pub fn print_play_frame(state: &mut PlayState, tile_atlas: Option<&TileAtlas>) -> io::Result<()> {
    println!();
    println!("{}", state.render_text_frame(5));
    if let Some(atlas) = tile_atlas {
        println!("{}", raster_diagnostic_line(state, 5, atlas)?);
    }
    Ok(())
}

pub fn print_play_script_snapshot(
    state: &mut PlayState,
    tile_atlas: Option<&TileAtlas>,
) -> io::Result<()> {
    println!("{}", play_script_state_line(state));
    if let Some(atlas) = tile_atlas {
        println!("{}", raster_diagnostic_line(state, 5, atlas)?);
    }
    Ok(())
}

pub fn raster_diagnostic_line(
    state: &mut PlayState,
    radius: usize,
    atlas: &TileAtlas,
) -> io::Result<String> {
    let Some(viewport) = state.render_top_down_frame(radius, atlas)? else {
        return Ok("Raster viewport: unavailable for dungeon mode.".to_string());
    };
    Ok(format!(
        "Raster viewport: {}x{} px, {}x{} cells, {}, hash {:016x}.",
        viewport.width,
        viewport.height,
        viewport.cells_wide,
        viewport.cells_high,
        viewport.depth.label(),
        hash_palette_indices(&viewport.pixels)
    ))
}

pub fn play_script_state_line(state: &PlayState) -> String {
    format!(
        "State: {} at ({}, {}), facing {}, turn {}, date Y{} M{} D{} {:02}:{:02}, transport {}, wind {}, typeahead {}, message-bytes {} hash {:016x}.",
        state.current_area_label(),
        state.player.x,
        state.player.y,
        state.player.facing.name(),
        state.turn,
        state.clock.year,
        state.clock.month,
        state.clock.day,
        state.clock.hour,
        state.clock.minute,
        state.player.transport.status_label(),
        state.wind.status_message(),
        state.typeahead_status_label(),
        state.message.len(),
        hash_bytes(state.message.as_bytes())
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayInputDisposition {
    Continue,
    Quit,
}

pub fn handle_play_key_input(
    state: &mut PlayState,
    key: char,
    suffix: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    if state.resolve_moongate_prompt(key, game_dir)?.is_some() {
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_IGNORED_INPUT_KEY {
        state.message = match suffix {
            "function" => "Function key ignored.",
            _ => "Input ignored.",
        }
        .to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == PLAY_TYPEAHEAD_TOGGLE_KEY {
        state.toggle_typeahead_buffer();
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'q' {
        return Ok(PlayInputDisposition::Quit);
    }
    if matches!(state.area, Area::Dungeon { .. }) && key == 'Q' {
        return Ok(state.exit_to_dos_prompt(parse_inline_yes_no(suffix)));
    }
    if key == 'C' && !suffix.is_empty() {
        let turn_before = state.turn;
        let outcome = state.cast_spell_from_suffix(suffix, game_dir)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'M' && !suffix.is_empty() {
        if inline_mix_candidate(suffix) {
            state.mix_reagents_from_suffix(suffix);
        } else if state
            .meditate_shrine_from_suffix(suffix, game_dir)?
            .is_none()
        {
            state.mix_reagents_from_suffix(suffix);
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if key == 'N' && !suffix.is_empty() {
        state.new_order_from_suffix(suffix);
        return Ok(PlayInputDisposition::Continue);
    }
    let inline_direction = suffix.chars().find_map(Direction::from_play_key);
    let inline_hours = parse_inline_hours(suffix);
    let inline_drink = parse_inline_yes_no(suffix);
    let inline_party_index = parse_inline_party_index(suffix);
    let inline_use_request = parse_inline_use_request(suffix);
    let inline_talk_keyword = non_empty_talk_keyword(suffix);
    if state.handle_dungeon_key_with_inline(
        key,
        game_dir,
        inline_hours,
        inline_drink,
        inline_party_index,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(key, 'T' | 't') && inline_talk_keyword.is_some() {
        let turn_before = state.turn;
        let outcome = state.talk_facing_with_game_dir_and_keyword(game_dir, inline_talk_keyword)?;
        state.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if state.handle_top_down_key_with_inline(
        key,
        game_dir,
        inline_direction,
        inline_hours,
        inline_drink,
        inline_use_request,
    )? {
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(state.area, Area::Dungeon { .. }) {
        state.advance_visual_tick();
        state.message = "Zzzzzz...".to_string();
        return Ok(PlayInputDisposition::Continue);
    }
    state.message = format!("Unhandled command `{key}`.");
    Ok(PlayInputDisposition::Continue)
}

pub fn play_input_key_and_suffix(input: &str) -> Option<(char, String)> {
    let input = input.trim_end_matches(|ch| ch == '\r' || ch == '\n');
    if is_typeahead_toggle_token(input) {
        return Some((PLAY_TYPEAHEAD_TOGGLE_KEY, String::new()));
    }
    if let Some(key) = ansi_navigation_key(input) {
        return Some((key, String::new()));
    }
    if ansi_function_key(input).is_some() {
        return Some((PLAY_IGNORED_INPUT_KEY, "function".to_string()));
    }
    if unclassified_escape_sequence(input) {
        return Some((PLAY_IGNORED_INPUT_KEY, "escape".to_string()));
    }
    let mut chars = input.chars();
    chars.next().map(|key| (key, chars.collect()))
}

pub fn play_input_typeahead_chars(input: &str) -> Option<Vec<char>> {
    let input = input.trim_end_matches(|ch| ch == '\r' || ch == '\n');
    if input.is_empty()
        || is_typeahead_toggle_token(input)
        || ansi_navigation_key(input).is_some()
        || ansi_function_key(input).is_some()
        || unclassified_escape_sequence(input)
    {
        return None;
    }
    let chars = input.chars().collect::<Vec<_>>();
    if chars.len() > 1 && chars.iter().all(|key| is_simple_typeahead_key(*key)) {
        Some(chars)
    } else {
        None
    }
}

pub fn is_simple_typeahead_key(key: char) -> bool {
    Direction::from_play_key(key).is_some() || matches!(key, '.' | ' ')
}

pub fn is_typeahead_toggle_token(input: &str) -> bool {
    matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "buffer" | "typeahead" | "typeahead-buffer" | "toggle-buffer"
    )
}

pub fn ansi_navigation_key(input: &str) -> Option<char> {
    match input {
        "\x1b[A" | "\x1bOA" => Some('8'),
        "\x1b[B" | "\x1bOB" => Some('2'),
        "\x1b[D" | "\x1bOD" => Some('4'),
        "\x1b[C" | "\x1bOC" => Some('6'),
        "\x1b[H" | "\x1bOH" | "\x1b[1~" | "\x1b[7~" => Some('7'),
        "\x1b[5~" => Some('9'),
        "\x1b[F" | "\x1bOF" | "\x1b[4~" | "\x1b[8~" => Some('1'),
        "\x1b[6~" => Some('3'),
        _ => None,
    }
}

pub fn ansi_function_key(input: &str) -> Option<u8> {
    match input {
        "\x1bOP" | "\x1b[11~" | "\x1b[[A" => Some(1),
        "\x1bOQ" | "\x1b[12~" | "\x1b[[B" => Some(2),
        "\x1bOR" | "\x1b[13~" | "\x1b[[C" => Some(3),
        "\x1bOS" | "\x1b[14~" | "\x1b[[D" => Some(4),
        "\x1b[15~" | "\x1b[[E" => Some(5),
        "\x1b[17~" => Some(6),
        "\x1b[18~" => Some(7),
        "\x1b[19~" => Some(8),
        "\x1b[20~" => Some(9),
        "\x1b[21~" => Some(10),
        _ => None,
    }
}

pub fn unclassified_escape_sequence(input: &str) -> bool {
    input.starts_with('\x1b')
        && input.chars().nth(1).is_some()
        && ansi_navigation_key(input).is_none()
        && ansi_function_key(input).is_none()
}

pub fn handle_empty_play_input(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    if state.pending_moongate.is_some() {
        state.resolve_moongate_prompt('\n', game_dir)?;
    } else if state
        .resolve_current_dungeon_room_trigger(Some(game_dir))?
        .is_none()
    {
        state.pass_turn_with_game_dir(Some(game_dir))?;
    }
    Ok(())
}

pub fn handle_play_script_command(
    state: &mut PlayState,
    command: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let command = command.trim();
    if is_typeahead_toggle_token(command) {
        return handle_play_key_input(state, PLAY_TYPEAHEAD_TOGGLE_KEY, "", game_dir);
    }
    if matches!(command.to_ascii_lowercase().as_str(), "empty" | "pass") {
        handle_empty_play_input(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if let Some(count) = play_script_idle_tick_count(command)? {
        for _ in 0..count {
            if handle_play_key_input(state, '.', "", game_dir)? == PlayInputDisposition::Quit {
                return Ok(PlayInputDisposition::Quit);
            }
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if state.typeahead_buffer_enabled {
        if let Some(keys) = play_input_typeahead_chars(command) {
            for key in keys {
                if handle_play_key_input(state, key, "", game_dir)? == PlayInputDisposition::Quit {
                    return Ok(PlayInputDisposition::Quit);
                }
            }
            return Ok(PlayInputDisposition::Continue);
        }
    }
    let Some((key, suffix)) = play_input_key_and_suffix(command) else {
        handle_empty_play_input(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    };
    handle_play_key_input(state, key, &suffix, game_dir)
}

