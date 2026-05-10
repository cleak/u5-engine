    fn world_state(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
        PlayState {
            area: Area::World {
                plane: WorldPlane::Underworld,
            },
            player: Player {
                x,
                y,
                facing: Direction::South,
                transport: TransportState::Foot,
            },
            active_objects: vec![ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x,
                y,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }],
            npcs: Vec::new(),
            door_tracker: None,
            opened_town_doors: Vec::new(),
            revealed_town_secret_doors: Vec::new(),
            passability: None,
            moongates: Vec::new(),
            grid,
            clock: GameClock::default(),
            animation: AnimationClock::default(),
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
            ambient_light: 0,
            visibility_dirty: false,
            wind: WindState::default(),
            wind_save_byte: 0,
            timing_status: TimingStatusTag::default(),
            time_stop_counter: 0,
            active_effect_tag: None,
            active_effect_counter: 0,
            sail_cadence: 0,
            sail_stall_pending: false,
            turn: 0,
            message: String::new(),
            debug_enter: None,
            return_world: None,
            world_overlays: WorldOverlayCache::default(),
            save_template_source: SaveTemplateSource::PreferSavedGame,
            typeahead_buffer_enabled: false,
            pending_moongate: None,
        }
    }

    fn mount_horse(state: &mut PlayState) {
        state.player.transport = TransportState::Horse {
            type_byte: 160,
            tile: 160,
        };
        state.sync_player_object();
    }

    fn mount_balloon(state: &mut PlayState) {
        state.player.transport = TransportState::Balloon {
            type_byte: FIRST_PLAYABLE_BALLOON_TILE,
            tile: FIRST_PLAYABLE_BALLOON_TILE,
        };
        state.sync_player_object();
    }

    fn britannia_state(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
        let mut state = world_state(grid, x, y);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        state
    }

    fn debug_game_dir() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("u5-engine-test-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("CASTLE.DAT"), vec![16; 1024]).unwrap();
        fs::write(dir.join("CASTLE.NPC"), vec![0; 576]).unwrap();
        fs::write(dir.join("CASTLE.TLK"), [1, 0, 0, 0]).unwrap();
        fs::write(dir.join("DUNGEON.DAT"), vec![0; DUNGEON_DAT_LEN]).unwrap();
        fs::write(dir.join("UNDER.DAT"), vec![5; UNDER_DAT_LEN]).unwrap();
        write_britannia_world_files(&dir, 5);
        dir
    }

    fn saved_game_seed_bytes(scene: u8, z: u8, x: u8, y: u8) -> Vec<u8> {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_SCENE_OFFSET] = scene;
        bytes[SAVE_Z_OFFSET] = z;
        bytes[SAVE_X_OFFSET] = x;
        bytes[SAVE_Y_OFFSET] = y;
        write_u16_at(&mut bytes, SAVE_FOOD_STOCK_OFFSET, DEFAULT_FOOD_STOCK);
        write_u16_at(&mut bytes, SAVE_GOLD_STOCK_OFFSET, DEFAULT_GOLD_STOCK);
        bytes[SAVE_KEY_STOCK_OFFSET] = DEFAULT_KEY_STOCK;
        bytes[SAVE_GEM_STOCK_OFFSET] = DEFAULT_GEM_STOCK;
        bytes[SAVE_TORCH_STOCK_OFFSET] = DEFAULT_TORCH_STOCK;
        encode_reagent_stock(&mut bytes, DEFAULT_REAGENTS);
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());
        bytes
    }

    fn write_saved_clock(bytes: &mut [u8], clock: GameClock) {
        write_u16_at(bytes, SAVE_YEAR_OFFSET, clock.year);
        bytes[SAVE_MONTH_OFFSET] = clock.month;
        bytes[SAVE_DAY_OFFSET] = clock.day;
        bytes[SAVE_HOUR_OFFSET] = clock.hour;
        bytes[SAVE_MINUTE_OFFSET] = clock.minute;
        bytes[SAVE_AMPM_DISPLAY_OFFSET] = clock.display_hour();
    }

    fn ool_plane_with_object(slot: usize, object: ActiveObject) -> Vec<u8> {
        let mut bytes = vec![0; OOL_PLANE_LEN];
        write_ool_object(&mut bytes, slot, object);
        bytes
    }

    fn write_ool_object(bytes: &mut [u8], slot: usize, object: ActiveObject) {
        assert!(slot < OOL_SLOTS);
        let offset = slot * OOL_RECORD_LEN;
        bytes[offset] = object.type_byte;
        bytes[offset + 1] = object.tile;
        bytes[offset + 2] = object.x as u8;
        bytes[offset + 3] = object.y as u8;
        bytes[offset + 4] = if object.z < 0 { 0xff } else { object.z as u8 };
        bytes[offset + 5] = object.aux1;
        bytes[offset + 6] = object.phase;
        bytes[offset + 7] = object.aux3;
    }

    #[test]
    fn scene_partition_maps_castle_zero() {
        let scene = Scene::new(0x11).unwrap();
        assert_eq!(scene.family, Family::Castle);
        assert_eq!(scene.block, 0);
        assert_eq!(scene.key(), "CASTLE:0");
    }

    #[test]
    fn scene_key_parser_accepts_public_keys_and_scene_bytes() {
        assert_eq!(
            Scene::from_key("CASTLE:0").unwrap(),
            Scene::new(17).unwrap()
        );
        assert_eq!(Scene::from_key("town:2").unwrap(), Scene::new(3).unwrap());
        assert_eq!(Scene::from_key("0x20").unwrap(), Scene::new(32).unwrap());
        assert!(Scene::from_key("CASTLE:8").is_err());
        assert!(Scene::from_key("DUNGEON:0").is_err());
    }

    #[test]
    fn play_target_parser_accepts_town_dungeon_and_world_keys() {
        assert_eq!(
            PlayTarget::from_key("CASTLE:0").unwrap(),
            PlayTarget::Town(Scene::new(17).unwrap())
        );
        assert_eq!(
            PlayTarget::from_key("DUNGEON:0").unwrap(),
            PlayTarget::Dungeon(DungeonScene::new(33).unwrap())
        );
        assert_eq!(
            PlayTarget::from_key("0x21").unwrap(),
            PlayTarget::Dungeon(DungeonScene::new(33).unwrap())
        );
        assert_eq!(
            PlayTarget::from_key("UNDERWORLD").unwrap(),
            PlayTarget::World(WorldPlane::Underworld)
        );
        assert_eq!(
            PlayTarget::from_key("0").unwrap(),
            PlayTarget::World(WorldPlane::Britannia)
        );
        assert!(PlayTarget::from_key("DUNGEON:8").is_err());
    }

    #[test]
    fn wind_status_message_uses_public_labels() {
        assert_eq!(WindState::Calm.status_message(), "Calm Winds");
        assert_eq!(WindState::North.status_message(), "North Winds");
        assert_eq!(WindState::South.status_message(), "South Winds");
        assert_eq!(WindState::East.status_message(), "East Winds");
        assert_eq!(WindState::West.status_message(), "West Winds");
    }

    #[test]
    fn rel_hur_wind_change_uses_first_playable_cycle() {
        let mut wind = WindState::Calm;
        let mut cycle = Vec::new();
        for _ in 0..5 {
            wind = wind.rel_hur_next();
            cycle.push(wind);
        }

        assert_eq!(
            cycle,
            vec![
                WindState::North,
                WindState::South,
                WindState::East,
                WindState::West,
                WindState::Calm
            ]
        );
    }

    #[test]
    fn look2_parser_resolves_tile_descriptions_and_ignores_dos_eof() {
        let mut bytes = look2_bytes(&[(5, "grass"), (192, "villager")]);
        bytes.push(0x1a);

        let table = parse_look2_dat(&bytes).unwrap();

        assert_eq!(table.description(5), Some("grass"));
        assert_eq!(table.description(192), Some("villager"));
        assert!(table.is_sentinel(table.description(0).unwrap()));
        assert!(table.is_sentinel(table.description(6).unwrap()));
    }

    #[test]
    fn look2_parser_rejects_offsets_outside_string_pool() {
        let mut bytes = look2_bytes(&[]);
        bytes[10..12].copy_from_slice(&(LOOK2_TABLE_LEN as u16 - 1).to_le_bytes());

        assert!(parse_look2_dat(&bytes).is_err());
    }

    #[test]
    fn look2_parser_rejects_unterminated_strings() {
        let mut bytes = look2_bytes(&[(5, "grass")]);
        bytes.pop();

        assert!(parse_look2_dat(&bytes).is_err());
    }

    #[test]
    fn direction_keys_leave_command_letters_for_dispatch() {
        assert_eq!(Direction::from_play_key('w'), Some(Direction::North));
        assert_eq!(Direction::from_play_key('d'), Some(Direction::East));
        assert_eq!(Direction::from_play_key('u'), Some(Direction::NorthEast));
        assert_eq!(Direction::from_play_key('n'), Some(Direction::SouthEast));
        assert_eq!(Direction::from_play_key('l'), None);
        assert_eq!(Direction::from_play_key('k'), None);
        assert_eq!(Direction::from_play_key('h'), None);
        assert_eq!(Direction::from_play_key('j'), None);
    }

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

    #[test]
    fn from_init_town_bootstrap_caches_init_ool_for_surface_return() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(17, 0, 15, 15)).unwrap();
        let init_object = ActiveObject {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            x: 11,
            y: 12,
            z: 0,
            phase: STEADY_PHASE,
            aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
            aux3: 1,
        };
        fs::write(dir.join("INIT.OOL"), ool_plane_with_object(1, init_object)).unwrap();

        let options = load_play_options_from_init(&dir).unwrap();
        let state = PlayState::load_town_scene(&dir, scene, options).unwrap();
        let cached_overlay = state.world_overlays.get(WorldPlane::Britannia).unwrap();

        assert_eq!(cached_overlay[0], init_object);
        assert!(
            state
                .npcs
                .iter()
                .any(|npc| npc.is_player_phantom() && (npc.x, npc.y) == (15, 15))
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_init_rejects_bad_init_ool_size() {
        let dir = debug_game_dir();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0, 10, 20)).unwrap();
        fs::write(dir.join("INIT.OOL"), vec![0; OOL_PLANE_LEN - 1]).unwrap();

        let err = load_play_options_from_init(&dir).err().unwrap();

        assert!(err.to_string().contains("INIT.OOL"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_init_save_uses_init_gam_template_even_when_saved_exists() {
        let dir = debug_game_dir();
        let mut init_template = saved_game_seed_bytes(0, 0, 10, 20);
        init_template[SAVE_AVATAR_NAME_OFFSET] = b'I';
        fs::write(dir.join("INIT.GAM"), init_template).unwrap();
        fs::write(dir.join("INIT.OOL"), vec![0; OOL_PLANE_LEN]).unwrap();
        let mut stale_saved = saved_game_seed_bytes(0, 0, 99, 99);
        stale_saved[SAVE_AVATAR_NAME_OFFSET] = b'S';
        fs::write(dir.join("SAVED.GAM"), stale_saved).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();

        let options = load_play_options_from_init(&dir).unwrap();
        assert_eq!(options.save_template_source, SaveTemplateSource::InitGame);
        let mut state = PlayState::load_world_scene(&dir, WorldPlane::Britannia, options).unwrap();

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_AVATAR_NAME_OFFSET], b'I');
        assert_eq!(saved[SAVE_X_OFFSET], 10);
        assert_eq!(saved[SAVE_Y_OFFSET], 20);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cli_parser_rejects_save_and_init_seed_conflict() {
        assert!(
            parse_cli_args(["--play", "--from-save", "--from-init", r"C:\Games\U5-Clean",])
                .is_err()
        );
    }

    #[test]
    fn cli_parser_rejects_bad_raster_depth() {
        assert!(parse_cli_args(["--play", "--raster-depth", "hercules"]).is_err());
    }

    #[test]
    fn split_play_script_trims_and_drops_blank_commands() {
        assert_eq!(
            split_play_script(" d ; empty ; ; C1IL ; q "),
            vec!["d", "empty", "C1IL", "q"]
        );
    }

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
    fn cli_parser_rejects_missing_or_duplicate_play_script() {
        assert!(parse_cli_args(["--play-script"]).is_err());
        assert!(parse_cli_args(["--play-script", "d", "--play-script", "q"]).is_err());
    }

    #[test]
    fn cli_parser_recognizes_help_long_flag() {
        let args = parse_cli_args(["--help"]).unwrap();
        assert!(args.help);
        assert!(!args.play);
        assert!(args.play_script.is_none());
    }

    #[test]
    fn cli_parser_recognizes_help_short_flag() {
        let args = parse_cli_args(["-h"]).unwrap();
        assert!(args.help);
    }

    #[test]
    fn cli_parser_help_short_circuits_save_init_conflict() {
        // --help bypasses validation that would otherwise reject this combo,
        // so users can still get usage even with bad flags.
        let args = parse_cli_args(["--help", "--from-save", "--from-init"]).unwrap();
        assert!(args.help);
    }

    #[test]
    fn cli_usage_lists_documented_smoke_commands() {
        assert!(CLI_USAGE.contains("--play"));
        assert!(CLI_USAGE.contains("--play-script"));
        assert!(CLI_USAGE.contains("--scene"));
        assert!(CLI_USAGE.contains("--floor"));
    }

    #[test]
    fn from_save_refreshes_saved_ool_plane_mirrors() {
        let dir = debug_game_dir();
        let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), save).unwrap();
        let mut saved_ool = vec![0; SAVED_OOL_LEN];
        for byte in &mut saved_ool[..OOL_PLANE_LEN] {
            *byte = 0x11;
        }
        for byte in &mut saved_ool[OOL_PLANE_LEN..] {
            *byte = 0x22;
        }
        fs::write(dir.join("SAVED.OOL"), &saved_ool).unwrap();
        fs::write(dir.join("BRIT.OOL"), vec![0x99; OOL_PLANE_LEN]).unwrap();
        fs::write(dir.join("UNDER.OOL"), vec![0x88; OOL_PLANE_LEN]).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();

        assert_eq!(options.save_template_source, SaveTemplateSource::SavedGame);
        assert_eq!(
            fs::read(dir.join("BRIT.OOL")).unwrap(),
            saved_ool[..OOL_PLANE_LEN].to_vec()
        );
        assert_eq!(
            fs::read(dir.join("UNDER.OOL")).unwrap(),
            saved_ool[OOL_PLANE_LEN..].to_vec()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_save_rejects_bad_saved_ool_without_mirror_writes() {
        let dir = debug_game_dir();
        let mut save = saved_game_seed_bytes(0, 0, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), save).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN - 1]).unwrap();
        let old_brit = vec![0x33; OOL_PLANE_LEN];
        let old_under = vec![0x44; OOL_PLANE_LEN];
        fs::write(dir.join("BRIT.OOL"), &old_brit).unwrap();
        fs::write(dir.join("UNDER.OOL"), &old_under).unwrap();

        let err = load_play_options_from_save(&dir).err().unwrap();

        assert!(err.to_string().contains("SAVED.OOL"));
        assert_eq!(fs::read(dir.join("BRIT.OOL")).unwrap(), old_brit);
        assert_eq!(fs::read(dir.join("UNDER.OOL")).unwrap(), old_under);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_save_town_load_preserves_embedded_active_object_table() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let object = ActiveObject {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
            x: 4,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0x33,
            aux3: 0x44,
        };
        let mut save = saved_game_seed_bytes(scene.byte, 0, 15, 15);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        write_ool_object(
            &mut save[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
            1,
            object,
        );
        fs::write(dir.join("SAVED.GAM"), save).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();
        let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();

        assert_eq!(state.active_objects[1], object);
        assert!(state.active_objects.iter().any(|object| {
            object.type_byte == PLAYER_NPC_SENTINEL_TYPE
                && (object.x, object.y, object.z) == (15, 15, 0)
        }));

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        let saved_objects = decode_saved_active_objects(&saved).unwrap();
        assert_eq!(saved_objects[0], object);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_save_dungeon_load_preserves_embedded_ambient_active_object_table() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        let object = ActiveObject {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            x: 12,
            y: 21,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
            aux3: 2,
        };
        let mut save = saved_game_seed_bytes(scene.byte, 0, 1, 1);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        write_ool_object(
            &mut save[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
            1,
            object,
        );
        fs::write(dir.join("SAVED.GAM"), save).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();
        let state = PlayState::load_dungeon_scene(&dir, scene, options).unwrap();

        assert_eq!(state.active_objects[1], object);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_prompts_or_cancels_without_writing() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(0, 0xff, 10, 20);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), template).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);

        assert_eq!(
            state.save_game_command(&dir, None).unwrap(),
            MoveOutcome::PromptDeclined
        );
        assert_eq!(state.message, "Save game? Use QY to save or QN to cancel.");
        assert!(!dir.join("SAVED.OOL").exists());

        assert_eq!(
            state.save_game_command(&dir, Some(false)).unwrap(),
            MoveOutcome::PromptDeclined
        );
        assert_eq!(state.message, "No.");
        assert!(!dir.join("SAVED.OOL").exists());
        assert_eq!(state.turn, 0);
        let _ = fs::remove_dir_all(dir);
    }

