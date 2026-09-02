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
        cleanup_previous_hour: 0,
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
        party_roster: default_party_roster(1),
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
        removed_town_npc_flags: HashMap::new(),
        talk_branch_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        toll_progress: 0,
        natural_moongate_counter: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        wind: WindState::default(),
        wind_save_byte: 0,
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        camp_cooldown: 0,
        camp_month_cookie: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        transport,
        facing: None,
        door_tracker: None,
        pending_vehicle: None,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        inn_registry: Vec::new(),
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: Some(Vec::new()),
        town_npc_mutations: Vec::new(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
    };

    let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options.clone()).unwrap();

    assert_eq!(state.player.transport, transport);
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_SHIP_FURLED_FIRST
    );
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
    assert_eq!(
        state.pending_vehicle_save,
        PendingVehicleSaveState {
            x: 12,
            y: 21,
            class_byte: 0,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_load_consumes_pending_skiff_by_appending_slot() {
    let dir = debug_game_dir();
    let options = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        floor: -1,
        start: Some((10, 20)),
        pending_vehicle: Some(PendingVehicleAcquisition::Skiff {
            x: 12,
            y: 21,
            aux3: 0x15,
        }),
        pending_vehicle_save: PendingVehicleSaveState {
            x: 12,
            y: 21,
            class_byte: 0x55,
        },
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
            aux3: 0x15,
        }
    );
    assert_eq!(
        state.pending_vehicle_save,
        PendingVehicleSaveState {
            x: 12,
            y: 21,
            class_byte: 0,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn pending_vehicle_packed_classes_decode_without_normalization() {
    assert_eq!(
        PendingVehicleSaveState {
            x: 9,
            y: 7,
            class_byte: 0x3f,
        }
        .acquisition(),
        None
    );
    assert_eq!(
        PendingVehicleSaveState {
            x: 9,
            y: 7,
            class_byte: 0x7f,
        }
        .acquisition(),
        Some(PendingVehicleAcquisition::Skiff {
            x: 9,
            y: 7,
            aux3: 0x3f,
        })
    );
    let packed_frigate = PendingVehicleSaveState {
        x: 9,
        y: 7,
        class_byte: 0xc2,
    };
    let frigate = packed_frigate.acquisition().unwrap();
    assert_eq!(
        frigate,
        PendingVehicleAcquisition::Frigate {
            x: 9,
            y: 7,
            skiffs: 0x42,
        }
    );
    assert_eq!(
        PendingVehicleSaveState::from_acquisition(frigate),
        packed_frigate
    );
    assert_eq!(frigate.active_object(0).aux3, 2);
}

#[test]
fn world_load_rejects_pending_vehicle_when_object_table_is_full() {
    let dir = debug_game_dir();
    let options = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        floor: -1,
        start: Some((10, 20)),
        pending_vehicle: Some(PendingVehicleAcquisition::Skiff {
            x: 12,
            y: 21,
            aux3: 0,
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
fn world_enter_reports_no_matching_coordinate_after_consuming_world_action() {
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x14;
    let mut state = world_state(grid, 10, 20);

    assert_eq!(
        state.enter_current_location(Path::new("")).unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "towne\nWhat town?");
    assert_eq!(state.turn, 1);
    assert_eq!((state.clock.hour, state.clock.minute), (12, 2));
}

#[test]
fn published_world_location_table_matches_gazetteer_return_rows() {
    let entries = published_world_location_entries();

    assert_eq!(entries.len(), 40);
    assert!(entries.iter().any(|entry| {
        entry.target == PlayTarget::Town(Scene::new(1).unwrap())
            && entry.plane == WorldPlane::Britannia
            && (entry.x, entry.y) == (232, 135)
            && entry.town_entry_y.is_none()
            && entry.expected_tile == Some(0x14)
            && entry.narration_class == Some(WorldEntryNarrationClass::Towne)
    }));
    assert!(entries.iter().any(|entry| {
        entry.target == PlayTarget::Town(Scene::new(32).unwrap())
            && entry.plane == WorldPlane::Britannia
            && (entry.x, entry.y) == (146, 241)
    }));
    assert!(entries.iter().any(|entry| {
        entry.target == PlayTarget::Dungeon(DungeonScene::new(33).unwrap())
            && entry.plane == WorldPlane::Britannia
            && (entry.x, entry.y) == (240, 73)
    }));
    assert!(entries.iter().any(|entry| {
        entry.target == PlayTarget::Dungeon(DungeonScene::new(40).unwrap())
            && entry.plane == WorldPlane::Underworld
            && (entry.x, entry.y) == (128, 128)
    }));
}

fn write_all_location_family_fixtures(dir: &Path) {
    let mut open_pages = Vec::with_capacity(LOCATION_DAT_FILE_LEN);
    for _ in 0..LOCATION_DAT_BLOCKS_PER_FILE * LOCATION_DAT_FLOOR_PAGES_PER_BLOCK {
        open_pages.extend(open_grid());
    }
    for (dat, npc, tlk) in [
        ("TOWNE.DAT", TOWNE_NPC_FILENAME, TOWNE_TLK_FILENAME),
        ("DWELLING.DAT", DWELLING_NPC_FILENAME, DWELLING_TLK_FILENAME),
        ("CASTLE.DAT", CASTLE_NPC_FILENAME, CASTLE_TLK_FILENAME),
        ("KEEP.DAT", KEEP_NPC_FILENAME, KEEP_TLK_FILENAME),
    ] {
        fs::write(dir.join(dat), &open_pages).unwrap();
        fs::write(dir.join(npc), vec![0; NPC_FILE_LEN]).unwrap();
        fs::write(dir.join(tlk), [0, 0]).unwrap();
    }
}

#[test]
fn world_enter_all_published_locations_without_sidecar() {
    let dir = debug_game_dir();
    write_all_location_family_fixtures(&dir);

    for entry in published_world_location_entries() {
        let mut grid = open_world_grid();
        grid[world_cell_index(entry.x, entry.y)] = entry.expected_tile.unwrap();
        let mut state = world_state(grid, entry.x, entry.y);
        state.area = Area::World { plane: entry.plane };
        state.active_objects[0].z = entry.plane.save_floor();
        if matches!(entry.target, PlayTarget::Dungeon(scene) if scene.record == 7) {
            state.shadowlord_hideouts = [SHADOWLORD_VANQUISHED; SHADOWLORD_COUNT];
        }

        state.begin_command_echo_for(Command::Enter);
        let outcome = state.enter_current_location(&dir).unwrap();

        let transcript = state.message_entries();
        assert_eq!(
            transcript.first().map(|line| line.text.as_str()),
            Some(format!("Enter {}", entry.narration_class.unwrap().text()).as_str()),
            "{} used the wrong Enter continuation",
            entry.target.key()
        );
        assert!(transcript.first().unwrap().is_command_echo);
        match (entry.proper_name, entry.name_column) {
            (Some(name), Some(column)) => {
                let name_entry = transcript
                    .iter()
                    .find(|line| line.text == name)
                    .expect("named entry must publish its proper-name line");
                assert!(name_entry.centered);
                assert!(transcript.iter().any(|line| line.explicit_blank));
                let log = message_log_from_entries(transcript, |text| Some(text.to_string()));
                let layout = layout_message_window(&log, Some(""));
                let row = layout
                    .rows
                    .iter()
                    .find(|row| row.text == name)
                    .expect("proper name must reach the message-window layout");
                assert_eq!(row.column, MESSAGE_WINDOW_LEFT + column);
            }
            (None, None) => {
                assert!(!transcript.iter().any(|line| line.centered));
            }
            _ => panic!("published proper-name metadata must be complete"),
        }

        match entry.target {
            PlayTarget::Town(scene) => {
                assert_eq!(
                    outcome,
                    MoveOutcome::Transition(AreaTransition::EnteredLocation(scene)),
                    "published location row for {} at {} ({}, {}) did not enter town",
                    scene.key(),
                    entry.plane.key(),
                    entry.x,
                    entry.y
                );
                assert_eq!(state.area, Area::Town { scene, floor: 0 });
                assert_eq!(
                    (state.player.x, state.player.y),
                    (LOCATION_DEFAULT_ENTRY_X, LOCATION_DEFAULT_ENTRY_Y),
                    "{} did not use the fixed public #94 entry cell",
                    scene.key()
                );
                assert_eq!(
                    (state.active_objects[0].x, state.active_objects[0].y),
                    (LOCATION_DEFAULT_ENTRY_X, LOCATION_DEFAULT_ENTRY_Y)
                );
                assert!(state.return_world.is_none());
                assert!(!state.message.contains("Entered "));
            }
            PlayTarget::Dungeon(scene) => {
                assert_eq!(
                    outcome,
                    MoveOutcome::Transition(AreaTransition::EnteredDungeon(scene)),
                    "published location row for {} at {} ({}, {}) did not enter dungeon",
                    scene.key(),
                    entry.plane.key(),
                    entry.x,
                    entry.y
                );
                let expected_level = if entry.plane == WorldPlane::Underworld && scene.record != 7 {
                    7
                } else {
                    0
                };
                assert_eq!(
                    state.area,
                    Area::Dungeon {
                        scene,
                        level: expected_level
                    }
                );
                assert_eq!(
                    state
                        .return_world
                        .as_ref()
                        .map(|ret| (ret.plane, ret.x, ret.y)),
                    Some((entry.plane, entry.x, entry.y))
                );
                assert!(!state.message.contains("Entered "));
            }
            PlayTarget::World(_) => unreachable!("published table excludes world targets"),
        }
    }

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn restore_world_for_all_published_location_targets_without_sidecar() {
    let dir = debug_game_dir();

    for entry in published_world_location_entries() {
        let mut state = world_state(open_world_grid(), 0, 0);

        assert!(
            state.restore_world_for_target(&dir, entry.target).unwrap(),
            "published return row for {} did not restore",
            entry.target.key()
        );

        assert_eq!(state.area, Area::World { plane: entry.plane });
        assert_eq!((state.player.x, state.player.y), (entry.x, entry.y));
        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!(state.active_objects[0].z, entry.plane.save_floor());
        assert!(state.return_world.is_none());
    }

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_uses_published_location_table_without_sidecar() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(86, 107)] = 0x3E;
    let mut state = britannia_state(grid, 86, 107);

    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert!(state.return_world.is_none());
    assert_eq!(state.message, "the Castle of Lord British!\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_existing_table_without_matching_coordinate_consumes_world_action() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA\t11\t20\tCASTLE:0\t7\t0x15\tCASTLE\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x15;
    let mut state = britannia_state(grid, 10, 20);
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
    assert_eq!(state.turn, 1);
    assert_eq!((state.clock.hour, state.clock.minute), (12, 2));
    assert_eq!(state.message, "castle\nWhat town?");
    assert_eq!(
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Enter castle", "What town?"]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_sidecar_without_narration_class_is_no_action() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA\t10\t20\tCASTLE:0\t7\t24\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x15;
    let mut state = britannia_state(grid, 10, 20);

    state.begin_command_echo_for(Command::Enter);
    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    state.commit_command_echo();

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (10, 20));
    assert_eq!(state.turn, 0);
    assert_eq!((state.clock.hour, state.clock.minute), (12, 0));
    assert_eq!(state.message, "What?");
    assert_eq!(state.message_entries()[0].text, "Enter What?");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_uses_clean_location_table_for_town() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA\t10\t20\tCASTLE:0\t7\t0x15\tCASTLE\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x15;
    let mut state = britannia_state(grid, 10, 20);
    let transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 3,
        skiffs: 1,
    };
    state.player.transport = transport;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
    state.sail_cadence = 1;
    state.sail_stall_pending = true;
    state.sync_player_object();

    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert_eq!(state.player.transport, transport);
    assert_eq!(
        state.active_effect_timing_status(),
        TimingStatusTag::HalfTime
    );
    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!(state.active_objects[0].tile, transport.save_marker());
    assert!(state.return_world.is_none());
    assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
    assert_eq!(state.message, "castle\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_live_tile_selects_noun_within_helper_not_authored_stock_tile() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 CASTLE:0 7 0x15 CASTLE\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x10;
    let mut state = britannia_state(grid, 10, 20);

    state.begin_command_echo_for(Command::Enter);
    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(Scene::new(17).unwrap()))
    );

    assert_eq!(state.message_entries()[0].text, "Enter hut");
    assert_eq!(state.message, "hut\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_opposite_helper_uses_its_own_no_match_refusal() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 CASTLE:0 7 0x15 CASTLE\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x17;
    let mut state = britannia_state(grid, 10, 20);

    assert!(
        state
            .handle_top_down_key_with_inline('E', &dir, None, None, None, None)
            .unwrap()
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "mine\nWhat dungeon?");
    assert_eq!(
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Enter mine", "What dungeon?"]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_narration_and_current_plane_ool_precede_scene_load_failure() {
    let dir = debug_game_dir();
    fs::remove_file(dir.join("CASTLE.DAT")).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 CASTLE:0 7 0x15 CASTLE\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x15;
    let mut state = britannia_state(grid, 10, 20);
    state.active_objects.push(ActiveObject {
        type_byte: 0xA8,
        tile: 0xA8,
        x: 9,
        y: 20,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.begin_command_echo_for(Command::Enter);

    assert!(state.enter_current_location(&dir).is_err());

    assert_eq!(state.message_entries()[0].text, "Enter castle");
    let britannia =
        decode_full_ool_plane_table(&fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap()).unwrap();
    assert_eq!(
        (britannia[1].type_byte, britannia[1].x, britannia[1].y),
        (0xA8, 9, 20)
    );
    assert!(!dir.join(UNDER_OOL_FILENAME).is_file());
    assert!(matches!(state.area, Area::World { .. }));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_ignores_retired_town_entry_y_column() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 CASTLE:0 7 0x15 CASTLE\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x15;
    let mut state = britannia_state(grid, 10, 20);

    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert_eq!((state.player.x, state.player.y), (15, 30));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (15, 30)
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_stonegate_keeps_exact_entry_narration_without_diagnostics() {
    let dir = debug_game_dir();
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    fs::write(dir.join("KEEP.DAT"), location_pages()).unwrap();
    fs::write(dir.join("KEEP.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
    fs::write(dir.join("KEEP.TLK"), [0, 0]).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 KEEP:4 7 0x12 KEEP\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x12;
    let mut state = britannia_state(grid, 10, 20);
    state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];

    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    assert_eq!(state.message, "keep\n");
    assert!(!state.message.contains("Stonegate entry:"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_uses_clean_location_table_for_dungeon_seed() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA,10,20,DUNGEON:0,0x18,DUNGEON\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x18;
    let mut state = world_state(grid, 10, 20);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    let transport = TransportState::Foot;
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
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
    assert_eq!(
        state.active_effect_timing_status(),
        TimingStatusTag::NoMinuteLight
    );
    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!(state.active_objects[0].tile, PLAYER_TILE);
    assert_eq!(
        state.return_world.as_ref().map(|ret| (
            ret.transport,
            ret.sail_cadence,
            ret.sail_stall_pending
        )),
        Some((transport, 1, true))
    );
    assert_eq!(state.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
    assert_eq!(state.message, "dungeon\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_enter_dungeon_transport_refusal_is_exact_and_acted() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 DUNGEON:0 0x18 DUNGEON\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x18;
    let mut state = britannia_state(grid, 10, 20);
    state.player.transport = TransportState::Carpet {
        type_byte: 176,
        tile: 176,
    };
    state.sync_player_object();

    assert!(
        state
            .handle_top_down_key_with_inline('E', &dir, None, None, None, None)
            .unwrap()
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "dungeon\nOn foot!");
    assert_eq!(
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Enter dungeon", "On foot!"]
    );
    assert!(matches!(state.area, Area::World { .. }));
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
        "UNDERWORLD 10 20 DUNGEON:0 0x18 DUNGEON\nUNDERWORLD 12 34 DUNGEON:7 0x16 CAVE\n",
    )
    .unwrap();
    let mut non_doom_grid = open_world_grid();
    non_doom_grid[world_cell_index(10, 20)] = 0x18;
    let mut non_doom_state = world_state(non_doom_grid, 10, 20);

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

    let mut doom_grid = open_world_grid();
    doom_grid[world_cell_index(12, 34)] = 0x16;
    let mut sealed_doom_state = world_state(doom_grid.clone(), 12, 34);
    sealed_doom_state.begin_command_echo_for(Command::Enter);

    assert_eq!(
        sealed_doom_state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    sealed_doom_state.commit_command_echo();

    assert_eq!(
        sealed_doom_state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert!(sealed_doom_state.return_world.is_none());
    assert_eq!(sealed_doom_state.message, "cave\nAttacked at entrance!");
    assert_eq!(
        sealed_doom_state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Enter cave", "Attacked at entrance!"]
    );
    assert!(
        sealed_doom_state
            .active_objects
            .iter()
            .skip(1)
            .any(|object| !object.is_empty()),
        "sealed Doom entry must place its outdoor ambush object"
    );

    let mut doom_state = world_state(doom_grid, 12, 34);
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
        "BRITANNIA 10 20 CASTLE:0 7 0x18 CASTLE\nUNDERWORLD 12 34 DUNGEON:1 0x24 CAVE\n",
    )
    .unwrap();

    assert_eq!(entries[0].target, PlayTarget::Town(Scene::new(17).unwrap()));
    assert_eq!(entries[0].town_entry_y, Some(7));
    assert_eq!(entries[0].expected_tile, Some(0x18));
    assert_eq!(
        entries[0].narration_class,
        Some(WorldEntryNarrationClass::Castle)
    );
    assert_eq!(
        entries[1].target,
        PlayTarget::Dungeon(DungeonScene::new(34).unwrap())
    );
    assert_eq!(entries[1].town_entry_y, None);
    assert_eq!(entries[1].expected_tile, Some(0x24));
    assert_eq!(
        entries[1].narration_class,
        Some(WorldEntryNarrationClass::Cave)
    );
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
fn world_location_table_accepts_underworld_town_rows_for_ararat_extensions() {
    assert!(parse_world_location_entries("UNDERWORLD 10 20 CASTLE:0\n").is_ok());
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
fn retired_location_entry_y_table_does_not_override_fixed_entry() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    fs::write(dir.join(LOCATION_ENTRY_Y_TABLE_FILE), "CASTLE:0 7\n").unwrap();
    let options = PlayOptions {
        cleanup_previous_hour: 0,
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
        party_roster: default_party_roster(1),
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
        removed_town_npc_flags: HashMap::new(),
        talk_branch_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        toll_progress: 0,
        natural_moongate_counter: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        wind: WindState::default(),
        wind_save_byte: 0,
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        camp_cooldown: 0,
        camp_month_cookie: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        transport: TransportState::Foot,
        facing: None,
        door_tracker: None,
        pending_vehicle: None,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        inn_registry: Vec::new(),
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: None,
        town_npc_mutations: Vec::new(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
    };

    let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

    assert_eq!((state.player.x, state.player.y), (15, 30));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (15, 30)
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
    fs::write(dir.join("KEEP.TLK"), [0, 0]).unwrap();
    let mut options = PlayOptions::default();
    options.target = PlayTarget::Town(scene);
    options.start = Some((1, 1));
    options.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    options.shadowlord_hideouts = [SHADOWLORD_VANQUISHED, 2, SHADOWLORD_VANQUISHED];

    let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

    // The entry itself prints nothing (no published scene-entry narration),
    // so the presentation notes are the whole of the slot.
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
    fs::write(dir.join("TOWNE.TLK"), [0, 0]).unwrap();
    let mut options = PlayOptions::default();
    options.target = PlayTarget::Town(scene);
    options.start = Some((5, 5));
    options.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    options.saved_active_objects = None;

    let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

    assert!(state.message.contains("air of Falsehood"));
    assert_eq!(state.resident_shadowlord, Some(SHADOWLORD_FALSEHOOD_INDEX));
    let resident = state
        .npcs
        .iter()
        .find(|npc| npc.type_byte == SHADOWLORD_ACTOR_TILE)
        .unwrap();
    assert_eq!(resident.slot, 31);
    let object = state
        .active_objects
        .iter()
        .copied()
        .skip(1)
        .find(|object| PlayState::is_shadowlord_actor(*object))
        .unwrap();
    assert_eq!(
        object,
        ActiveObject {
            type_byte: SHADOWLORD_OBJECT_TILE_BASE,
            tile: SHADOWLORD_OBJECT_TILE_BASE,
            x: SHADOWLORD_TOWN_INSTALL_X,
            y: SHADOWLORD_TOWN_INSTALL_ROWS[0],
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_entry_preserving_reentry_records_host_without_duplicate_shadowlord_actor() {
    let dir = debug_game_dir();
    let scene = Scene::new(1).unwrap();
    fs::write(dir.join("TOWNE.DAT"), open_grid()).unwrap();
    fs::write(dir.join("TOWNE.NPC"), vec![0; (scene.block + 1) * 576]).unwrap();
    fs::write(dir.join("TOWNE.TLK"), [0, 0]).unwrap();
    let existing = ActiveObject {
        type_byte: SHADOWLORD_OBJECT_TILE_BASE,
        tile: SHADOWLORD_OBJECT_TILE_BASE,
        x: SHADOWLORD_TOWN_INSTALL_X,
        y: SHADOWLORD_TOWN_INSTALL_ROWS[0],
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    let mut options = PlayOptions::default();
    options.target = PlayTarget::Town(scene);
    options.start = Some((5, 5));
    options.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    options.saved_active_objects = Some(vec![existing]);

    let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

    assert!(state.message.contains("air of Falsehood"));
    assert_eq!(state.resident_shadowlord, Some(SHADOWLORD_FALSEHOOD_INDEX));
    assert_eq!(
        state
            .active_objects
            .iter()
            .copied()
            .skip(1)
            .filter(|object| PlayState::is_shadowlord_actor(*object))
            .count(),
        1
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_entry_row_four_skips_shadowlord_host_and_install() {
    let mut state = test_state(open_grid(), 5, SHADOWLORD_TOWN_ENTRY_SKIP_Y);
    state.area = Area::Town {
        scene: Scene::new(SCENE_MOONGLOW).unwrap(),
        floor: 0,
    };
    state.shadowlord_hideouts = [SCENE_MOONGLOW, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];

    assert_eq!(state.install_shadowlord_entry_encounter(), None);
    assert_eq!(state.resident_shadowlord, None);
    assert!(
        state
            .active_objects
            .iter()
            .copied()
            .skip(1)
            .all(|object| !PlayState::is_shadowlord_actor(object))
    );
    assert!(
        state
            .npcs
            .iter()
            .all(|npc| npc.type_byte != SHADOWLORD_ACTOR_TILE)
    );
}

#[test]
fn resident_shadowlord_uses_highest_free_npc_index_and_ordinary_actor_slot() {
    let mut state = test_state(open_grid(), 5, 5);
    state.area = Area::Town {
        scene: Scene::new(SCENE_BRITAIN).unwrap(),
        floor: 0,
    };
    state.shadowlord_hideouts = [SCENE_BRITAIN, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    for slot in (1..OOL_SLOTS).filter(|slot| *slot != 30) {
        state.npcs.push(RuntimeNpc::from_slot(
            &NpcSlot {
                slot,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0; NPC_SCHEDULE_RECORD_LEN],
                name: None,
            },
            state.clock.hour,
        ));
    }

    assert_eq!(
        state.install_shadowlord_entry_encounter(),
        Some((Some(1), SHADOWLORD_FALSEHOOD_INDEX))
    );
    let resident = state
        .npcs
        .iter()
        .find(|npc| npc.type_byte == SHADOWLORD_ACTOR_TILE)
        .unwrap();
    assert_eq!(resident.slot, 30);
    assert_eq!(resident.active_object, Some(1));
    assert_eq!(
        (resident.x, resident.y, resident.z),
        (
            SHADOWLORD_TOWN_INSTALL_X,
            SHADOWLORD_TOWN_INSTALL_ROWS[1],
            0
        )
    );
}

#[test]
fn resident_shadowlord_records_host_but_rejects_second_actor() {
    let mut state = test_state(open_grid(), 5, 5);
    state.area = Area::Town {
        scene: Scene::new(SCENE_JHELOM).unwrap(),
        floor: 0,
    };
    state.shadowlord_hideouts = [SCENE_JHELOM, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    state.active_objects.push(ActiveObject {
        type_byte: SHADOWLORD_ACTOR_TILE,
        tile: SHADOWLORD_ACTOR_TILE,
        x: 7,
        y: 7,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state.install_shadowlord_entry_encounter(),
        Some((None, SHADOWLORD_FALSEHOOD_INDEX))
    );
    assert_eq!(state.resident_shadowlord, Some(SHADOWLORD_FALSEHOOD_INDEX));
    assert!(
        state
            .npcs
            .iter()
            .all(|npc| npc.type_byte != SHADOWLORD_ACTOR_TILE)
    );
    assert_eq!(
        state
            .active_objects
            .iter()
            .copied()
            .skip(1)
            .filter(|object| PlayState::is_shadowlord_actor(*object))
            .count(),
        1
    );
}

#[test]
fn resident_shadowlord_overwrites_npc_index_31_when_roster_is_full() {
    let mut state = test_state(open_grid(), 5, 5);
    state.area = Area::Town {
        scene: Scene::new(SCENE_MINOC).unwrap(),
        floor: 0,
    };
    state.shadowlord_hideouts = [SCENE_MINOC, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    for slot in 1..OOL_SLOTS {
        state.npcs.push(RuntimeNpc::from_slot(
            &NpcSlot {
                slot,
                type_byte: 1,
                dialog_id: 0,
                schedule: [0; NPC_SCHEDULE_RECORD_LEN],
                name: None,
            },
            state.clock.hour,
        ));
    }

    assert_eq!(
        state.install_shadowlord_entry_encounter(),
        Some((Some(1), SHADOWLORD_FALSEHOOD_INDEX))
    );
    assert_eq!(state.npcs.len(), OOL_SLOTS - 1);
    let slot_31 = state.npcs.iter().find(|npc| npc.slot == 31).unwrap();
    assert_eq!(slot_31.type_byte, SHADOWLORD_ACTOR_TILE);
    assert_eq!(slot_31.active_object, Some(1));
    assert_eq!(
        (slot_31.x, slot_31.y, slot_31.z),
        (
            SHADOWLORD_TOWN_INSTALL_X,
            SHADOWLORD_TOWN_INSTALL_ROWS[4],
            0
        )
    );
}

#[test]
fn resident_hatred_sweep_reproduces_fixed_slot_four_defect() {
    let mut state = test_state(open_grid(), 5, 5);
    let scheduled = |slot, type_byte| NpcSlot {
        slot,
        type_byte,
        dialog_id: 0x20 + slot as u8,
        schedule: [1, 2, 3, 5, 6, 7, 5, 6, 7, 0, 0, 0, 6, 12, 18, 22],
        name: None,
    };
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        scheduled(1, 0x20),
        scheduled(2, 0x80),
        scheduled(4, TOWN_NPC_ORDINARY_TYPE_FIRST),
    ]);
    let mut draws = [1; NPC_SLOTS_PER_SUB_MAP];
    draws[1] = 0;
    draws[2] = 0;

    assert_eq!(
        state.apply_resident_shadowlord_npc_sweep_with_draws(SHADOWLORD_HATRED_INDEX, &draws),
        (2, 0)
    );
    let low = state.npcs.iter().find(|npc| npc.slot == 1).unwrap();
    assert_eq!(&low.schedule[..3], &[6, 6, 6]);
    assert_eq!(&low.schedule[12..16], &[0, 0, 0, 0]);
    assert_eq!(low.dialog_id, TOWN_NPC_BRUSHOFF_DIALOG_ID);
    let high = state.npcs.iter().find(|npc| npc.slot == 2).unwrap();
    assert_eq!(&high.schedule[..3], &[7, 7, 7]);
    assert_eq!(high.dialog_id, TOWN_NPC_BRUSHOFF_DIALOG_ID);

    let mut rejected = test_state(open_grid(), 5, 5);
    rejected.load_scheduled_npcs(&[
        scheduled(1, 0x20),
        scheduled(4, TOWN_NPC_ORDINARY_TYPE_FIRST - 1),
    ]);
    let before = rejected.npcs.clone();
    assert_eq!(
        rejected.apply_resident_shadowlord_npc_sweep_with_draws(
            SHADOWLORD_HATRED_INDEX,
            &[0; NPC_SLOTS_PER_SUB_MAP]
        ),
        (0, 0)
    );
    assert_eq!(rejected.npcs, before);
}

#[test]
fn resident_cowardice_writes_cowering_dialogue_even_when_flight_rejects() {
    let mut state = test_state(open_grid(), 5, 5);
    let scheduled = |slot, type_byte| NpcSlot {
        slot,
        type_byte,
        dialog_id: 0x30 + slot as u8,
        schedule: [1, 2, 3, 5, 6, 7, 5, 6, 7, 0, 0, 0, 6, 12, 18, 22],
        name: None,
    };
    state.load_scheduled_npcs(&[
        scheduled(1, 0x20),
        scheduled(2, TOWN_NPC_ORDINARY_TYPE_LAST),
        scheduled(4, TOWN_NPC_ORDINARY_TYPE_FIRST),
    ]);
    let mut draws = [1; NPC_SLOTS_PER_SUB_MAP];
    draws[1] = 0;
    draws[2] = 0;

    assert_eq!(
        state.apply_resident_shadowlord_npc_sweep_with_draws(SHADOWLORD_COWARDICE_INDEX, &draws),
        (0, 2)
    );
    let rejected = state.npcs.iter().find(|npc| npc.slot == 1).unwrap();
    assert_eq!(&rejected.schedule[..3], &[1, 2, 3]);
    assert_eq!(rejected.dialog_id, TOWN_NPC_COWERING_DIALOG_ID);
    let fled = state.npcs.iter().find(|npc| npc.slot == 2).unwrap();
    assert_eq!(&fled.schedule[..3], &[3, 3, 3]);
    assert_eq!(&fled.schedule[12..16], &[6, 12, 18, 22]);
    assert_eq!(fled.dialog_id, TOWN_NPC_COWERING_DIALOG_ID);
}

#[test]
fn resident_sweep_consumes_all_thirty_two_draws_before_other_tests() {
    let mut state = test_state(open_grid(), 5, 5);
    state.load_scheduled_npcs(&[NpcSlot {
        slot: 4,
        type_byte: TOWN_NPC_ORDINARY_TYPE_FIRST - 1,
        dialog_id: 0,
        schedule: [0; 16],
        name: None,
    }]);
    state.prng_state = 0x1234;
    let mut expected_state = state.prng_state;
    for _ in 0..NPC_SLOTS_PER_SUB_MAP {
        let _ = u5_prng_range_u16(&mut expected_state, 0, 1);
    }

    assert_eq!(
        state.apply_resident_shadowlord_npc_sweep(SHADOWLORD_HATRED_INDEX),
        (0, 0)
    );
    assert_eq!(state.prng_state, expected_state);
    let before_falsehood = state.prng_state;
    assert_eq!(
        state.apply_resident_shadowlord_npc_sweep(SHADOWLORD_FALSEHOOD_INDEX),
        (0, 0)
    );
    assert_eq!(state.prng_state, before_falsehood);
}

#[test]
fn resident_shadowlord_rows_cover_all_eight_towns() {
    for (scene, expected_row) in
        (SCENE_MOONGLOW..=SCENE_NEW_MAGINCIA).zip(SHADOWLORD_TOWN_INSTALL_ROWS)
    {
        assert_eq!(shadowlord_town_install_row(scene), Some(expected_row));
    }
    assert_eq!(shadowlord_town_install_row(SCENE_EAST_BRITANNY), None);
}

#[test]
fn shadowlord_blight_is_day_seeded_and_replaces_state_with_host_seed() {
    let mut grid = vec![
        SHADOWLORD_BLIGHT_STANDING_CROP_TILE,
        0x01,
        SHADOWLORD_BLIGHT_FRUIT_TREE_TILE,
        SHADOWLORD_BLIGHT_STANDING_CROP_TILE,
        SHADOWLORD_BLIGHT_FRUIT_TREE_TILE,
    ];
    let mut expected = grid.clone();
    let mut expected_state = 17u16;
    let mut expected_rewritten = 0;
    for cell in &mut expected {
        let replacement = match *cell {
            SHADOWLORD_BLIGHT_STANDING_CROP_TILE => SHADOWLORD_BLIGHT_PLOWED_PATCH_TILE,
            SHADOWLORD_BLIGHT_FRUIT_TREE_TILE => SHADOWLORD_BLIGHT_HOLLOW_STUMP_TILE,
            _ => continue,
        };
        if u5_prng_range_u16(&mut expected_state, 0, SHADOWLORD_BLIGHT_ROLL_HIGH) != 0 {
            *cell = replacement;
            expected_rewritten += 1;
        }
    }
    let mut actual_state = 0xFFFF;

    assert_eq!(
        apply_shadowlord_blight(&mut grid, 17, &mut actual_state, 0x093D),
        expected_rewritten
    );
    assert_eq!(grid, expected);
    assert_eq!(actual_state, 0x093D);
}

#[test]
fn rejected_resident_install_still_runs_blight_first() {
    let mut state = test_state(
        vec![SHADOWLORD_BLIGHT_STANDING_CROP_TILE; TOWN_GRID_BYTES],
        5,
        5,
    );
    state.area = Area::Town {
        scene: Scene::new(SCENE_TRINSIC).unwrap(),
        floor: 0,
    };
    state.clock.day = 11;
    state.shadowlord_hideouts = [SCENE_TRINSIC, SHADOWLORD_VANQUISHED, SHADOWLORD_VANQUISHED];
    state.active_objects.push(ActiveObject {
        type_byte: SHADOWLORD_ACTOR_TILE,
        tile: SHADOWLORD_ACTOR_TILE,
        x: 1,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    let mut expected = vec![SHADOWLORD_BLIGHT_STANDING_CROP_TILE; TOWN_GRID_BYTES];
    let mut ignored_state = 0;
    apply_shadowlord_blight(&mut expected, 11, &mut ignored_state, 0);

    assert_eq!(
        state.install_shadowlord_entry_encounter(),
        Some((None, SHADOWLORD_FALSEHOOD_INDEX))
    );
    assert_eq!(state.grid, expected);
    assert!(state.visibility_dirty);
    assert!(state.prng_state <= PRNG_HOST_CLOCK_MASK);
}

#[test]
fn town_climb_uses_clean_location_floor_table_for_basement_page() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
    fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
    let mut grid = open_grid();
    grid[0] = TOWN_KLIMB_DESCEND_TILE;
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
    grid[0] = TOWN_KLIMB_ASCEND_TILE;
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
    grid[0] = TOWN_KLIMB_ASCEND_TILE;
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
            // `formats/npc.md` section 6 row `1`: the default-person
            // sentinel keeps its roster type byte and draws the forced
            // person tile. The withdrawn clamp made both fields `192`.
            type_byte: NPC_TYPE_DEFAULT_HUMAN_SPRITE,
            tile: NPC_DEFAULT_PERSON_SPRITE_TILE,
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
