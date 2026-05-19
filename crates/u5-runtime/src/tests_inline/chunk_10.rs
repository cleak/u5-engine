    #[test]
    fn world_load_from_save_applies_transport_marker_to_player_slot() {
        let dir = debug_game_dir();
        fs::write(dir.join("UNDER.DAT"), vec![1; UNDER_DAT_LEN]).unwrap();
        let transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        };
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            clock: GameClock::default(),
            food: DEFAULT_FOOD_STOCK,
            gold: DEFAULT_GOLD_STOCK,
            keys: DEFAULT_KEY_STOCK,
            gems: DEFAULT_GEM_STOCK,
            climbing_gear: DEFAULT_CLIMBING_GEAR,
            special_items: [0; SPECIAL_ITEM_COUNT],
            party: default_party(),
            party_names: default_party_names(1),
            party_experience: default_party_experience(1),
            party_stay_counters: default_party_stay_counters(1),
            party_strengths: default_party_strengths(1),
            party_intelligence: default_party_intelligence(1),
            party_equipment: default_party_equipment(1),
            equipment_stock: [0; EQUIPMENT_COUNT],
            spell_charges: [0; SPELL_COUNT],
            scroll_stock: [0; SCROLL_COUNT],
            potion_stock: [0; POTION_COUNT],
            reagents: DEFAULT_REAGENTS,
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            saved_dungeon_working_buffer: None,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            shrine_standing: [0; VIRTUE_COUNT],
            moral_standing: 0,
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
            fortunes_of_war: 0,
            active_player: None,
            combat_round_counter: 0,
            transport,
            pending_vehicle: None,
            inn_registry: Vec::new(),
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: Some(Vec::new()),
            save_template_source: SaveTemplateSource::PreferSavedGame,
        };

        let state =
            PlayState::load_world_scene(&dir, WorldPlane::Underworld, options.clone()).unwrap();

        assert_eq!(state.player.transport, transport);
        assert_eq!(state.active_objects[0].tile, 168);
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.grid[world_cell_index(10, 20)], 1);

        let mut foot_options = options;
        foot_options.transport = TransportState::Foot;
        assert!(PlayState::load_world_scene(&dir, WorldPlane::Underworld, foot_options).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_consumes_pending_frigate_into_first_free_object_slot() {
        let dir = debug_game_dir();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            pending_vehicle: Some(PendingVehicleAcquisition::Frigate {
                x: 12,
                y: 21,
                skiffs: 3,
            }),
            saved_active_objects: Some(vec![
                ActiveObject {
                    type_byte: 170,
                    tile: 170,
                    x: 4,
                    y: 5,
                    z: -1,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                },
                ActiveObject::empty(),
            ]),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!(
            state.active_objects[2],
            ActiveObject {
                type_byte: SHIP_PARKED_FIRST,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                x: 12,
                y: 21,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
                aux3: 3,
            }
        );
        assert_eq!(state.active_objects.len(), 3);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_consumes_pending_skiff_by_appending_slot() {
        let dir = debug_game_dir();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            pending_vehicle: Some(PendingVehicleAcquisition::Skiff { x: 12, y: 21 }),
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: SKIFF_PARKED_FIRST,
                tile: FIRST_PLAYABLE_SKIFF_TILE,
                x: 12,
                y: 21,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_rejects_pending_vehicle_when_object_table_is_full() {
        let dir = debug_game_dir();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            pending_vehicle: Some(PendingVehicleAcquisition::Skiff { x: 12, y: 21 }),
            saved_active_objects: Some(vec![
                ActiveObject {
                    type_byte: 170,
                    tile: 170,
                    x: 4,
                    y: 5,
                    z: -1,
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                };
                OOL_SLOTS - 1
            ]),
            ..PlayOptions::default()
        };

        let err = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options)
            .err()
            .unwrap();

        assert!(err.to_string().contains("pending vehicle acquisition"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_rejects_clean_lava_sidecar_start_for_foot() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 10 20 LAVA 5\n",
        )
        .unwrap();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            transport: TransportState::Foot,
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let err = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options)
            .err()
            .unwrap();

        assert!(err.to_string().contains("blocked by lava"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_allows_clean_lava_sidecar_start_for_carpet() {
        let dir = debug_game_dir();
        let mut under = vec![5; UNDER_DAT_LEN];
        under[world_cell_index(10, 20)] = 14;
        fs::write(dir.join("UNDER.DAT"), under).unwrap();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 10 20 LAVA 14\n",
        )
        .unwrap();
        let transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            transport,
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.player.transport, transport);
        assert_eq!(state.grid[world_cell_index(10, 20)], 14);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_allows_clean_drowning_sidecar_start_for_foot() {
        let dir = debug_game_dir();
        let mut under = vec![5; UNDER_DAT_LEN];
        under[world_cell_index(10, 20)] = 1;
        fs::write(dir.join("UNDER.DAT"), under).unwrap();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 10 20 DROWNING 1\n",
        )
        .unwrap();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            transport: TransportState::Foot,
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.grid[world_cell_index(10, 20)], 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_fallback_skips_clean_lava_sidecar_for_foot() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 1 1 LAVA 5\nUNDERWORLD 0 0 LAVA 5\n",
        )
        .unwrap();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: None,
            transport: TransportState::Foot,
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!((state.player.x, state.player.y), (1, 0));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_fallback_skips_foot_damaging_sidecar_for_foot() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 1 1 DROWNING 5\nUNDERWORLD 0 0 DROWNING 5\n",
        )
        .unwrap();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: None,
            transport: TransportState::Foot,
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.player.transport, TransportState::Foot);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_load_fallback_skips_transport_damaging_sidecar_for_carpet() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "UNDERWORLD 1 1 LAVA 5\nUNDERWORLD 0 0 LAVA 5\n",
        )
        .unwrap();
        let transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: None,
            transport,
            saved_active_objects: Some(Vec::new()),
            ..PlayOptions::default()
        };

        let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!((state.player.x, state.player.y), (1, 0));
        assert_eq!(state.player.transport, transport);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_render_composites_overlay_objects() {
        let mut state = world_state(open_world_grid(), 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 2,
            y: 1,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let view = state.render_text_view(1);

        assert!(view.contains("@v"));
    }

    #[test]
    fn world_render_prefers_lower_active_object_slot_at_same_cell() {
        let mut state = world_state(open_world_grid(), 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 2,
            y: 1,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let view = state.render_text_view(1);

        assert!(view.contains("@n"));
        assert!(!view.contains("@v"));
    }

    #[test]
    fn world_enter_reports_missing_clean_coordinate_table() {
        let mut state = world_state(open_world_grid(), 10, 20);

        assert_eq!(
            state.enter_current_location(Path::new("")).unwrap(),
            MoveOutcome::Blocked
        );

        assert!(
            state
                .message
                .contains("No clean-room entrance coordinate table")
        );
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn world_enter_existing_table_without_matching_coordinate_is_not_a_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA\t11\t20\tCASTLE:0\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.debug_enter = Some(PlayTarget::Town(Scene::new(17).unwrap()));

        assert_eq!(
            state
                .handle_top_down_key_with_inline('E', &dir, None, None, None, None)
                .unwrap(),
            true
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("No entry in world_locations.tsv"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_enter_location_tile_guard_mismatch_is_not_a_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA\t10\t20\tCASTLE:0\t7\t24\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("No entry in world_locations.tsv"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_enter_uses_clean_location_table_for_town() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA\t10\t20\tCASTLE:0\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        let transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: true,
            hull: 3,
            skiffs: 1,
        };
        state.player.transport = transport;
        state.timing_status = TimingStatusTag::HalfTime;
        state.sail_cadence = 1;
        state.sail_stall_pending = true;
        state.sync_player_object();

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.active_objects[0].tile, PLAYER_TILE);
        assert_eq!(
            state.return_world.as_ref().map(|ret| (ret.x, ret.y)),
            Some((10, 20))
        );
        assert_eq!(
            state.return_world.as_ref().map(|ret| (
                ret.transport,
                ret.timing_status,
                ret.sail_cadence,
                ret.sail_stall_pending
            )),
            Some((transport, TimingStatusTag::HalfTime, 1, true))
        );
        assert!(state.message.contains("Entered CASTLE:0 from BRITANNIA"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_enter_uses_optional_town_entry_y_column() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0 7\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
        );

        assert_eq!(state.area, Area::Town { scene, floor: 0 });
        assert_eq!((state.player.x, state.player.y), (15, 7));
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (15, 7)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_enter_stonegate_preserves_entry_message_and_presentation_notes() {
        let dir = debug_game_dir();
        let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
        fs::write(dir.join("KEEP.DAT"), location_pages()).unwrap();
        fs::write(dir.join("KEEP.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
        fs::write(dir.join("KEEP.TLK"), [1, 0, 0, 0]).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 KEEP:4 7\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
        state.shadowlord_hideouts = [
            1,
            SHADOWLORD_VANQUISHED,
            SHADOWLORD_VANQUISHED,
        ];

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
        );

        assert!(state.message.contains("Entered KEEP:4 from BRITANNIA"));
        assert!(state.message.contains("Sceptre prelude"));
        assert!(state.message.contains("air of Falsehood"));
        assert!(!state.message.contains("air of Hatred"));
        assert!(!state.message.contains("air of Cowardice"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_enter_uses_clean_location_table_for_dungeon_seed() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA,10,20,DUNGEON:0\n",
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();
        let transport = TransportState::Carpet {
            type_byte: 176,
            tile: 176,
        };
        state.player.transport = transport;
        state.timing_status = TimingStatusTag::NoMinuteLight;
        state.sail_cadence = 1;
        state.sail_stall_pending = true;
        state.sync_player_object();

        assert_eq!(
            state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredDungeon(scene))
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.timing_status, TimingStatusTag::Normal);
        assert_eq!(state.sail_cadence, 0);
        assert!(!state.sail_stall_pending);
        assert_eq!(state.active_objects[0].tile, PLAYER_TILE);
        assert_eq!(
            state.return_world.as_ref().map(|ret| (
                ret.transport,
                ret.timing_status,
                ret.sail_cadence,
                ret.sail_stall_pending
            )),
            Some((transport, TimingStatusTag::NoMinuteLight, 1, true))
        );
        assert!(state.message.contains("Entered DUNGEON:0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_enter_underworld_dungeon_seed_respects_doom_exception() {
        let dir = debug_game_dir();
        let non_doom = DungeonScene::new(33).unwrap();
        let doom = DungeonScene::new(40).unwrap();
        fs::write(dir.join("DUNGEON.DAT"), vec![0; DUNGEON_DAT_LEN]).unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 10 20 DUNGEON:0\nUNDERWORLD 12 34 DUNGEON:7\n",
        )
        .unwrap();
        let mut non_doom_state = world_state(open_world_grid(), 10, 20);

        assert_eq!(
            non_doom_state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredDungeon(non_doom))
        );

        assert_eq!(
            non_doom_state.area,
            Area::Dungeon {
                scene: non_doom,
                level: 7
            }
        );
        assert_eq!((non_doom_state.player.x, non_doom_state.player.y), (7, 7));
        assert_eq!(non_doom_state.player.facing, Direction::West);
        assert_eq!(non_doom_state.active_objects[0].z, 7);
        assert_eq!(
            non_doom_state.return_world.as_ref().map(|ret| ret.plane),
            Some(WorldPlane::Underworld)
        );

        let mut sealed_doom_state = world_state(open_world_grid(), 12, 34);

        assert_eq!(
            sealed_doom_state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(
            sealed_doom_state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert!(sealed_doom_state.return_world.is_none());
        assert_eq!(
            sealed_doom_state.message,
            "Doom is sealed until all Shadowlords are vanquished."
        );

        let mut doom_state = world_state(open_world_grid(), 12, 34);
        doom_state.shadowlord_hideouts = [SHADOWLORD_VANQUISHED; SHADOWLORD_COUNT];

        assert_eq!(
            doom_state.enter_current_location(&dir).unwrap(),
            MoveOutcome::Transition(AreaTransition::EnteredDungeon(doom))
        );

        assert_eq!(
            doom_state.area,
            Area::Dungeon {
                scene: doom,
                level: 0
            }
        );
        assert_eq!((doom_state.player.x, doom_state.player.y), (1, 1));
        assert_eq!(doom_state.player.facing, Direction::East);
        assert_eq!(doom_state.active_objects[0].z, 0);
        assert_eq!(
            doom_state.return_world.as_ref().map(|ret| ret.plane),
            Some(WorldPlane::Underworld)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_world_location_entries_accepts_optional_tile_guards() {
        let entries = parse_world_location_entries(
            "BRITANNIA 10 20 CASTLE:0 7 0x18\nUNDERWORLD 12 34 DUNGEON:1 0x24\n",
        )
        .unwrap();

        assert_eq!(entries[0].target, PlayTarget::Town(Scene::new(17).unwrap()));
        assert_eq!(entries[0].town_entry_y, Some(7));
        assert_eq!(entries[0].expected_tile, Some(0x18));
        assert_eq!(
            entries[1].target,
            PlayTarget::Dungeon(DungeonScene::new(34).unwrap())
        );
        assert_eq!(entries[1].town_entry_y, None);
        assert_eq!(entries[1].expected_tile, Some(0x24));
    }

    #[test]
    fn world_location_table_rejects_duplicate_coordinate_rows() {
        let text = "\
BRITANNIA 10 20 CASTLE:0
BRITANNIA 10 20 DUNGEON:0
";

        assert!(parse_world_location_entries(text).is_err());
    }

    #[test]
    fn world_location_table_rejects_duplicate_target_rows() {
        let text = "\
BRITANNIA 10 20 CASTLE:0
BRITANNIA 11 21 CASTLE:0
";

        assert!(parse_world_location_entries(text).is_err());
        assert!(
            parse_world_location_entries("BRITANNIA 10 20 DUNGEON:0\nUNDERWORLD 11 21 DUNGEON:0\n")
                .is_err()
        );
    }

    #[test]
    fn world_location_table_rejects_entry_y_for_dungeon_rows() {
        assert!(parse_world_location_entries("BRITANNIA 10 20 DUNGEON:0 7 8\n").is_err());
    }

    #[test]
    fn world_location_table_rejects_underworld_town_rows() {
        assert!(parse_world_location_entries("UNDERWORLD 10 20 CASTLE:0\n").is_err());
        assert!(parse_world_location_entries("UNDERWORLD 10 20 DUNGEON:0\n").is_ok());
    }

    #[test]
    fn location_floor_table_maps_signed_floors_to_clean_base_page() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();

        assert_eq!(load_floor(&dir, scene, -1).unwrap()[0], 4);
        assert_eq!(load_floor(&dir, scene, 0).unwrap()[0], 5);
        assert_eq!(load_floor(&dir, scene, 1).unwrap()[0], 6);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn location_floor_table_rejects_duplicate_scene_rows() {
        let text = "\
CASTLE:0 5
CASTLE:0 6
";

        assert!(parse_location_floor_entries(text).is_err());
    }

    #[test]
    fn location_entry_y_table_seeds_direct_town_start() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join(LOCATION_ENTRY_Y_TABLE_FILE), "CASTLE:0 7\n").unwrap();
        let options = PlayOptions {
            target: PlayTarget::Town(scene),
            floor: 0,
            start: None,
            clock: GameClock::default(),
            food: DEFAULT_FOOD_STOCK,
            gold: DEFAULT_GOLD_STOCK,
            keys: DEFAULT_KEY_STOCK,
            gems: DEFAULT_GEM_STOCK,
            climbing_gear: DEFAULT_CLIMBING_GEAR,
            special_items: [0; SPECIAL_ITEM_COUNT],
            party: default_party(),
            party_names: default_party_names(1),
            party_experience: default_party_experience(1),
            party_stay_counters: default_party_stay_counters(1),
            party_strengths: default_party_strengths(1),
            party_intelligence: default_party_intelligence(1),
            party_equipment: default_party_equipment(1),
            equipment_stock: [0; EQUIPMENT_COUNT],
            spell_charges: [0; SPELL_COUNT],
            scroll_stock: [0; SCROLL_COUNT],
            potion_stock: [0; POTION_COUNT],
            reagents: DEFAULT_REAGENTS,
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            saved_dungeon_working_buffer: None,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            shrine_standing: [0; VIRTUE_COUNT],
            moral_standing: 0,
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
            fortunes_of_war: 0,
            active_player: None,
            combat_round_counter: 0,
            transport: TransportState::Foot,
            pending_vehicle: None,
            inn_registry: Vec::new(),
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: None,
            save_template_source: SaveTemplateSource::PreferSavedGame,
        };

        let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

        assert_eq!((state.player.x, state.player.y), (15, 7));
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (15, 7)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn location_entry_y_table_rejects_duplicate_scene_rows() {
        let text = "\
CASTLE:0 7
CASTLE:0 8
";

        assert!(parse_location_entry_y_entries(text).is_err());
    }

    #[test]
    fn stonegate_load_appends_entry_presentation_notes() {
        let dir = debug_game_dir();
        let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
        fs::write(dir.join("KEEP.DAT"), location_pages()).unwrap();
        fs::write(dir.join("KEEP.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
        fs::write(dir.join("KEEP.TLK"), [1, 0, 0, 0]).unwrap();
        let mut options = PlayOptions::default();
        options.target = PlayTarget::Town(scene);
        options.start = Some((1, 1));
        options.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
        options.shadowlord_hideouts = [
            SHADOWLORD_VANQUISHED,
            2,
            SHADOWLORD_VANQUISHED,
        ];

        let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

        assert!(state.message.contains("Entered KEEP:4"));
        assert!(state.message.contains("Sceptre prelude"));
        assert!(state.message.contains("air of Hatred"));
        assert!(!state.message.contains("air of Falsehood"));
        assert!(!state.message.contains("air of Cowardice"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_entry_installs_living_shadowlord_for_matching_fresh_scene() {
        let dir = debug_game_dir();
        let scene = Scene::new(1).unwrap();
        fs::write(dir.join("TOWNE.DAT"), open_grid()).unwrap();
        fs::write(dir.join("TOWNE.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
        fs::write(dir.join("TOWNE.TLK"), [1, 0, 0, 0]).unwrap();
        let mut options = PlayOptions::default();
        options.target = PlayTarget::Town(scene);
        options.start = Some((5, 5));
        options.shadowlord_hideouts = [
            1,
            SHADOWLORD_VANQUISHED,
            SHADOWLORD_VANQUISHED,
        ];
        options.saved_active_objects = None;

        let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

        assert!(state.message.contains("Shadowlord entry: Falsehood appears"));
        let object = state
            .active_objects
            .iter()
            .copied()
            .find(|object| {
                PlayState::shadowlord_name_encounter_index(*object)
                    == Some(SHADOWLORD_FALSEHOOD_INDEX)
            })
            .unwrap();
        assert_eq!(
            object,
            ActiveObject {
                type_byte: SHADOWLORD_OBJECT_TILE_BASE,
                tile: SHADOWLORD_OBJECT_TILE_BASE,
                x: 5,
                y: 6,
                z: 0,
                phase: active_object_phase_from_direction(Direction::North, 0),
                aux1: SHADOWLORD_FALSEHOOD_INDEX as u8,
                aux3: 1,
            }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_entry_preserving_reentry_does_not_duplicate_shadowlord_object() {
        let dir = debug_game_dir();
        let scene = Scene::new(1).unwrap();
        fs::write(dir.join("TOWNE.DAT"), open_grid()).unwrap();
        fs::write(dir.join("TOWNE.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
        fs::write(dir.join("TOWNE.TLK"), [1, 0, 0, 0]).unwrap();
        let existing = ActiveObject {
            type_byte: SHADOWLORD_OBJECT_TILE_BASE,
            tile: SHADOWLORD_OBJECT_TILE_BASE,
            x: 5,
            y: 6,
            z: 0,
            phase: STEADY_PHASE,
            aux1: SHADOWLORD_FALSEHOOD_INDEX as u8,
            aux3: 1,
        };
        let mut options = PlayOptions::default();
        options.target = PlayTarget::Town(scene);
        options.start = Some((5, 5));
        options.shadowlord_hideouts = [
            1,
            SHADOWLORD_VANQUISHED,
            SHADOWLORD_VANQUISHED,
        ];
        options.saved_active_objects = Some(vec![existing]);

        let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

        assert!(!state.message.contains("Shadowlord entry"));
        assert_eq!(
            state
                .active_objects
                .iter()
                .copied()
                .filter(|object| {
                    PlayState::shadowlord_name_encounter_index(*object)
                        == Some(SHADOWLORD_FALSEHOOD_INDEX)
                })
                .count(),
            1
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_climb_uses_clean_location_floor_table_for_basement_page() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);

        assert_eq!(
            state.climb(&dir, ClimbIntent::Down).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: -1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: -1 });
        assert_eq!(state.grid[0], 4);
        assert_eq!(state.active_objects[0].z, -1);
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_climb_clears_active_objects_without_compacting_slots() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 2,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject::empty());

        assert_eq!(
            state.climb(&dir, ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.active_objects.len(), 3);
        assert_eq!(state.active_objects[0].z, 1);
        assert!(state.active_objects[1].is_empty());
        assert!(state.active_objects[2].is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_climb_relinks_npcs_for_reloaded_floor() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        let mut grid = open_grid();
        grid[0] = 80;
        let mut state = test_state(grid, 0, 0);
        let slots = vec![
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0, 0, 0, 4, 4, 4, 5, 5, 5, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
            NpcSlot {
                slot: 2,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0, 0, 0, 6, 6, 6, 7, 7, 7, 1, 1, 1, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(state.npcs[0].active_object, Some(1));
        assert_eq!(state.npcs[1].active_object, None);

        assert_eq!(
            state.climb(&dir, ClimbIntent::Up).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: 1 })
        );

        assert_eq!(state.area, Area::Town { scene, floor: 1 });
        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.npcs[0].active_object, None);
        let slot = state.npcs[1]
            .active_object
            .expect("floor 1 NPC should link");
        assert_eq!(
            state.active_objects[slot],
            ActiveObject {
                type_byte: 192,
                tile: 192,
                x: 6,
                y: 7,
                z: 1,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert_eq!(state.turn, 1);
        let _ = fs::remove_dir_all(dir);
    }

