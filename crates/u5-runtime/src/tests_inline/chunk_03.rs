    #[test]
    fn save_game_command_writes_supported_saved_gam_and_saved_ool_fields() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(17, 0, 15, 15);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        template[SAVE_WIND_OFFSET] = 9;
        fs::write(dir.join("SAVED.GAM"), template).unwrap();
        let britannia_object = ActiveObject {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
            x: 3,
            y: 4,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let mut existing_ool = vec![0; SAVED_OOL_LEN];
        write_ool_object(&mut existing_ool[..OOL_PLANE_LEN], 1, britannia_object);
        fs::write(dir.join("SAVED.OOL"), existing_ool).unwrap();
        let underworld_object = ActiveObject {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            x: 12,
            y: 21,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
            aux3: 2,
        };
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(140, 13, 28, 18, 45).unwrap();
        state.food = 1234;
        state.gold = 9876;
        state.keys = 7;
        state.gems = 3;
        state.torches = 5;
        state.torch_counter = 44;
        state.light_spell_counter = 22;
        state.wind = WindState::North;
        state.wind_save_byte = 9;
        state.player.transport = TransportState::Skiff {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
        };
        state.timing_status = TimingStatusTag::HalfTime;
        state.spell_charges[REL_HUR_SPELL_INDEX] = 4;
        state.reagents = [9, 8, 7, 6, 5, 4, 3, 2];
        state.moonstone_slots[2] = MoonstoneGateSlot {
            scene: 0,
            x: 77,
            y: 88,
            z: 0,
        };
        state.shrine_ordained_mask = 0b0000_1010;
        state.shrine_codex_mask = 0b0100_0001;
        state.avatar_stats = AvatarStats {
            strength: 23,
            dexterity: 24,
            intelligence: 25,
        };
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'P',
                climb_stat: 18,
                mana: 5,
                hp: 33,
                max_hp: 66,
                level: 3,
            },
            PartyMember {
                slot: 1,
                status: b'S',
                climb_stat: 7,
                mana: 6,
                hp: 44,
                max_hp: 88,
                level: 4,
            },
        ];
        state.active_objects.push(underworld_object);

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved.len(), SAVED_GAM_LEN);
        assert_eq!(saved[SAVE_SCENE_OFFSET], 0);
        assert_eq!(saved[SAVE_Z_OFFSET], 0xff);
        assert_eq!(saved[SAVE_X_OFFSET], 10);
        assert_eq!(saved[SAVE_Y_OFFSET], 20);
        assert_eq!(u16_at(&saved, SAVE_YEAR_OFFSET), 140);
        assert_eq!(saved[SAVE_MONTH_OFFSET], 13);
        assert_eq!(saved[SAVE_DAY_OFFSET], 28);
        assert_eq!(saved[SAVE_HOUR_OFFSET], 18);
        assert_eq!(saved[SAVE_MINUTE_OFFSET], 45);
        assert_eq!(saved[SAVE_AMPM_DISPLAY_OFFSET], 6);
        assert_eq!(u16_at(&saved, SAVE_FOOD_STOCK_OFFSET), 1234);
        assert_eq!(u16_at(&saved, SAVE_GOLD_STOCK_OFFSET), 9876);
        assert_eq!(saved[SAVE_KEY_STOCK_OFFSET], 7);
        assert_eq!(saved[SAVE_GEM_STOCK_OFFSET], 3);
        assert_eq!(saved[SAVE_TORCH_STOCK_OFFSET], 5);
        assert_eq!(saved[SAVE_SPELL_CHARGES_OFFSET + REL_HUR_SPELL_INDEX], 4);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET], 4);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 1], 5);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 2], 7);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 3], 8);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 4], 2);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 5], 3);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 6], 6);
        assert_eq!(saved[SAVE_REAGENTS_OFFSET + 7], 9);
        assert_eq!(saved[SAVE_MOONSTONE_X_OFFSET + 2], 77);
        assert_eq!(saved[SAVE_MOONSTONE_Y_OFFSET + 2], 88);
        assert_eq!(saved[SAVE_MOONSTONE_SCENE_OFFSET + 2], 0);
        assert_eq!(saved[SAVE_MOONSTONE_Z_OFFSET + 2], 0);
        assert_eq!(saved[SAVE_LIGHT_SPELL_COUNTER_OFFSET], 22);
        assert_eq!(saved[SAVE_TORCH_COUNTER_OFFSET], 44);
        assert_eq!(saved[SAVE_SHRINE_ORDAINED_MASK_OFFSET], 0b0000_1010);
        assert_eq!(saved[SAVE_SHRINE_CODEX_MASK_OFFSET], 0b0100_0001);
        assert_eq!(saved[SAVE_TIMING_STATUS_TAG_OFFSET], b'Q');
        assert_eq!(saved[SAVE_ACTIVE_PLAYER_OFFSET], 0xff);
        assert_eq!(
            saved[SAVE_TRANSPORT_MARKER_OFFSET],
            FIRST_PLAYABLE_SKIFF_TILE
        );
        assert_eq!(saved[SAVE_WIND_OFFSET], 9);
        assert_eq!(saved[SAVE_PARTY_SIZE_OFFSET], 2);
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_STATUS_OFFSET],
            b'P'
        );
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_STR_OFFSET], 23);
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_DEX_OFFSET], 24);
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_INT_OFFSET], 25);
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_MANA_OFFSET], 5);
        assert_eq!(
            u16_at(&saved, SAVE_ROSTER_OFFSET + SAVE_CHARACTER_HP_OFFSET),
            33
        );
        assert_eq!(
            u16_at(&saved, SAVE_ROSTER_OFFSET + SAVE_CHARACTER_MAX_HP_OFFSET),
            66
        );
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_LEVEL_OFFSET], 3);
        let second = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        assert_eq!(saved[second + SAVE_CHARACTER_STATUS_OFFSET], b'S');
        assert_eq!(saved[second + SAVE_CHARACTER_MANA_OFFSET], 6);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_HP_OFFSET), 44);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_MAX_HP_OFFSET), 88);
        assert_eq!(saved[second + SAVE_CHARACTER_LEVEL_OFFSET], 4);
        assert_eq!(saved[SAVE_ACTIVE_OBJECTS_OFFSET], PLAYER_TILE);
        assert_eq!(
            saved[SAVE_ACTIVE_OBJECTS_OFFSET + 1],
            FIRST_PLAYABLE_SKIFF_TILE
        );
        let object_slot = SAVE_ACTIVE_OBJECTS_OFFSET + OOL_RECORD_LEN;
        assert_eq!(saved[object_slot], FIRST_PLAYABLE_FRIGATE_TILE);
        assert_eq!(saved[object_slot + 2], 12);
        assert_eq!(saved[object_slot + 3], 21);

        let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
        assert_eq!(saved_ool.len(), SAVED_OOL_LEN);
        let britannia_overlay = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
        let underworld_overlay = decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..]).unwrap();
        assert_eq!(britannia_overlay[0], britannia_object);
        assert_eq!(underworld_overlay[0], underworld_object);
        assert!(state.message.contains("Done."));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_preserves_unmapped_wind_save_byte() {
        let dir = debug_game_dir();
        let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        save[SAVE_WIND_OFFSET] = 0x7a;
        fs::write(dir.join("SAVED.GAM"), save).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.wind, WindState::Calm);
        assert_eq!(options.wind_save_byte, 0x7a);

        let mut state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_WIND_OFFSET], 0x7a);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn top_down_q_yes_routes_to_save_and_q_no_cancels() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(17, 0, 5, 5);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), template).unwrap();
        let mut state = test_state(open_grid(), 5, 5);

        assert!(
            state
                .handle_top_down_key_with_inline('Q', &dir, None, None, Some(false), None)
                .unwrap()
        );
        assert_eq!(state.message, "No.");
        assert!(!dir.join("SAVED.OOL").exists());

        assert!(
            state
                .handle_top_down_key_with_inline('Q', &dir, None, None, Some(true), None)
                .unwrap()
        );
        assert_eq!(state.message, "Yes. Saving... Done.");
        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_SCENE_OFFSET], 17);
        assert_eq!(saved[SAVE_X_OFFSET], 5);
        assert_eq!(saved[SAVE_Y_OFFSET], 5);
        assert!(dir.join("SAVED.OOL").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_play_options_read_public_location_tuple() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 13;
        bytes[SAVE_Z_OFFSET] = 0;
        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_Y_OFFSET] = 15;
        let clock = GameClock::with_date(141, 6, 7, 8, 35).unwrap();
        write_saved_clock(&mut bytes, clock);

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.target, PlayTarget::Town(Scene::new(13).unwrap()));
        assert_eq!(options.floor, 0);
        assert_eq!(options.start, Some((15, 15)));
        assert_eq!(options.clock, clock);
        assert_eq!(options.wind, WindState::Calm);
        assert_eq!(options.keys, 0);
        assert_eq!(options.climbing_gear, DEFAULT_CLIMBING_GEAR);
        assert_eq!(options.party, default_party());
    }

    #[test]
    fn save_play_options_read_party_climb_profile_from_roster() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 13;
        bytes[SAVE_Z_OFFSET] = 0;
        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_Y_OFFSET] = 15;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());
        bytes[SAVE_PARTY_SIZE_OFFSET] = 2;
        bytes[SAVE_SPELL_CHARGES_OFFSET + REL_HUR_SPELL_INDEX] = 3;
        bytes[SAVE_MOONSTONE_X_OFFSET + 1] = 22;
        bytes[SAVE_MOONSTONE_Y_OFFSET + 1] = 23;
        bytes[SAVE_MOONSTONE_SCENE_OFFSET + 1] = 0;
        bytes[SAVE_MOONSTONE_Z_OFFSET + 1] = 0xff;
        bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = 0b0010_0010;
        bytes[SAVE_SHRINE_CODEX_MASK_OFFSET] = 0b1000_0001;
        let first = SAVE_ROSTER_OFFSET;
        bytes[first + SAVE_CHARACTER_STATUS_OFFSET] = b'G';
        bytes[first + SAVE_CHARACTER_STR_OFFSET] = 11;
        bytes[first + SAVE_CHARACTER_DEX_OFFSET] = 18;
        bytes[first + SAVE_CHARACTER_INT_OFFSET] = 19;
        bytes[first + SAVE_CHARACTER_MANA_OFFSET] = 12;
        bytes[first + SAVE_CHARACTER_HP_OFFSET] = 44;
        bytes[first + SAVE_CHARACTER_HP_OFFSET + 1] = 1;
        bytes[first + SAVE_CHARACTER_MAX_HP_OFFSET] = 194;
        bytes[first + SAVE_CHARACTER_MAX_HP_OFFSET + 1] = 1;
        bytes[first + SAVE_CHARACTER_LEVEL_OFFSET] = 5;
        let second = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        bytes[second + SAVE_CHARACTER_STATUS_OFFSET] = b'D';
        bytes[second + SAVE_CHARACTER_DEX_OFFSET] = 7;
        bytes[second + SAVE_CHARACTER_MANA_OFFSET] = 2;
        bytes[second + SAVE_CHARACTER_HP_OFFSET] = 0;
        bytes[second + SAVE_CHARACTER_HP_OFFSET + 1] = 0;
        bytes[second + SAVE_CHARACTER_MAX_HP_OFFSET] = 120;
        bytes[second + SAVE_CHARACTER_LEVEL_OFFSET] = 1;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.spell_charges[REL_HUR_SPELL_INDEX], 3);
        assert_eq!(options.shrine_ordained_mask, 0b0010_0010);
        assert_eq!(options.shrine_codex_mask, 0b1000_0001);
        assert_eq!(
            options.avatar_stats,
            AvatarStats {
                strength: 11,
                dexterity: 18,
                intelligence: 19,
            }
        );
        assert_eq!(
            options.moonstone_slots[1],
            MoonstoneGateSlot {
                scene: 0,
                x: 22,
                y: 23,
                z: 0xff,
            }
        );
        assert_eq!(
            options.party,
            vec![
                PartyMember {
                    slot: 0,
                    status: b'G',
                    climb_stat: 18,
                    mana: 12,
                    hp: 300,
                    max_hp: 450,
                    level: 5,
                },
                PartyMember {
                    slot: 1,
                    status: b'D',
                    climb_stat: 7,
                    mana: 2,
                    hp: 0,
                    max_hp: 120,
                    level: 1,
                },
            ]
        );
    }

    #[test]
    fn save_play_options_read_signed_town_floor_tuple() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 13;
        bytes[SAVE_Z_OFFSET] = 0xff;
        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_Y_OFFSET] = 15;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.target, PlayTarget::Town(Scene::new(13).unwrap()));
        assert_eq!(options.floor, -1);
        assert_eq!(options.start, Some((15, 15)));
        assert_eq!(options.clock, GameClock::new(8, 35).unwrap());
    }

    #[test]
    fn save_play_options_read_public_dungeon_tuple() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        write_u16_at(&mut bytes, SAVE_FOOD_STOCK_OFFSET, 321);
        write_u16_at(&mut bytes, SAVE_GOLD_STOCK_OFFSET, 4321);
        bytes[SAVE_KEY_STOCK_OFFSET] = 5;
        bytes[SAVE_GEM_STOCK_OFFSET] = 2;
        bytes[SAVE_TORCH_STOCK_OFFSET] = 3;
        bytes[SAVE_SCENE_OFFSET] = 33;
        bytes[SAVE_Z_OFFSET] = 3;
        bytes[SAVE_X_OFFSET] = 7;
        bytes[SAVE_Y_OFFSET] = 6;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());
        bytes[SAVE_LIGHT_SPELL_COUNTER_OFFSET] = 21;
        bytes[SAVE_TORCH_COUNTER_OFFSET] = 34;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(
            options.target,
            PlayTarget::Dungeon(DungeonScene::new(33).unwrap())
        );
        assert_eq!(options.floor, 3);
        assert_eq!(options.start, Some((7, 6)));
        assert_eq!(options.clock, GameClock::new(8, 35).unwrap());
        assert_eq!(options.food, 321);
        assert_eq!(options.gold, 4321);
        assert_eq!(options.keys, 5);
        assert_eq!(options.gems, 2);
        assert_eq!(options.torches, 3);
        assert_eq!(options.light_spell_counter, 21);
        assert_eq!(options.torch_counter, 34);
    }

    #[test]
    fn save_play_options_read_public_overworld_tuple() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 0;
        bytes[SAVE_Z_OFFSET] = 0xff;
        bytes[SAVE_X_OFFSET] = 200;
        bytes[SAVE_Y_OFFSET] = 201;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
        assert_eq!(options.floor, -1);
        assert_eq!(options.start, Some((200, 201)));
        assert_eq!(options.clock, GameClock::new(8, 35).unwrap());
    }

    #[test]
    fn save_play_options_reads_reagents_in_recipe_order() {
        let mut bytes = saved_game_seed_bytes(17, 0, 15, 15);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_REAGENTS_OFFSET] = 4;
        bytes[SAVE_REAGENTS_OFFSET + 1] = 6;
        bytes[SAVE_REAGENTS_OFFSET + 2] = 7;
        bytes[SAVE_REAGENTS_OFFSET + 3] = 6;
        bytes[SAVE_REAGENTS_OFFSET + 4] = 1;
        bytes[SAVE_REAGENTS_OFFSET + 5] = 3;
        bytes[SAVE_REAGENTS_OFFSET + 6] = 2;
        bytes[SAVE_REAGENTS_OFFSET + 7] = 5;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.reagents, [5, 6, 7, 2, 6, 4, 3, 1]);
    }

    #[test]
    fn save_play_options_reads_recognized_transport_marker_family() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 0;
        bytes[SAVE_Z_OFFSET] = 0xff;
        bytes[SAVE_X_OFFSET] = 200;
        bytes[SAVE_Y_OFFSET] = 201;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());
        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = 168;

        let ship = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(
            ship.transport,
            TransportState::Ship {
                type_byte: 168,
                tile: 168,
                sails_hoisted: false,
                hull: 0,
                skiffs: 0,
            }
        );

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = 184;
        let carpet = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(
            carpet.transport,
            TransportState::Carpet {
                type_byte: 184,
                tile: 184,
            }
        );

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = 191;
        let unknown = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(unknown.transport, TransportState::Foot);
    }

    #[test]
    fn save_play_options_reads_timing_status_tag() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 0;
        bytes[SAVE_Z_OFFSET] = 0xff;
        bytes[SAVE_X_OFFSET] = 200;
        bytes[SAVE_Y_OFFSET] = 201;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        bytes[SAVE_TIMING_STATUS_TAG_OFFSET] = b'Q';
        let half_time = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(half_time.timing_status, TimingStatusTag::HalfTime);

        bytes[SAVE_TIMING_STATUS_TAG_OFFSET] = b'T';
        let no_minute_light = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(
            no_minute_light.timing_status,
            TimingStatusTag::NoMinuteLight
        );

        bytes[SAVE_TIMING_STATUS_TAG_OFFSET] = b'X';
        let unknown = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(unknown.timing_status, TimingStatusTag::Normal);
    }

    #[test]
    fn save_play_options_carries_embedded_active_objects() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 0;
        bytes[SAVE_Z_OFFSET] = 0xff;
        bytes[SAVE_X_OFFSET] = 200;
        bytes[SAVE_Y_OFFSET] = 201;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());
        let slot = SAVE_ACTIVE_OBJECTS_OFFSET + OOL_RECORD_LEN;
        bytes[slot] = 168;
        bytes[slot + 1] = 169;
        bytes[slot + 2] = 12;
        bytes[slot + 3] = 34;
        bytes[slot + 4] = 0xff;
        bytes[slot + 5] = 77;
        bytes[slot + 6] = 0x22;
        bytes[slot + 7] = 2;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        let saved_objects = options.saved_active_objects.unwrap();
        assert_eq!(saved_objects.len(), OOL_SLOTS - 1);
        assert_eq!(
            saved_objects[0],
            ActiveObject {
                type_byte: 168,
                tile: 169,
                x: 12,
                y: 34,
                z: -1,
                phase: 0x22,
                aux1: 77,
                aux3: 2,
            }
        );
        assert!(saved_objects.iter().skip(1).all(|object| object.is_empty()));
    }

    #[test]
    fn save_play_options_reject_unsupported_scene_ranges() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 41;
        bytes[SAVE_Z_OFFSET] = 0;
        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_Y_OFFSET] = 15;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        assert!(play_options_from_save_bytes(&bytes).is_err());
    }

    #[test]
    fn save_play_options_validate_size_time_floor_and_position() {
        assert!(play_options_from_save_bytes(&vec![0; SAVED_GAM_LEN - 1]).is_err());

        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 13;
        bytes[SAVE_Z_OFFSET] = 0;
        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_Y_OFFSET] = 15;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        bytes[SAVE_X_OFFSET] = 32;
        assert!(play_options_from_save_bytes(&bytes).is_err());

        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_MONTH_OFFSET] = 0;
        assert!(play_options_from_save_bytes(&bytes).is_err());

        bytes[SAVE_MONTH_OFFSET] = PLAY_START_MONTH;
        bytes[SAVE_DAY_OFFSET] = 29;
        assert!(play_options_from_save_bytes(&bytes).is_err());

        bytes[SAVE_DAY_OFFSET] = PLAY_START_DAY;
        bytes[SAVE_HOUR_OFFSET] = 24;
        assert!(play_options_from_save_bytes(&bytes).is_err());

        bytes[SAVE_SCENE_OFFSET] = 33;
        bytes[SAVE_Z_OFFSET] = 8;
        bytes[SAVE_X_OFFSET] = 7;
        bytes[SAVE_HOUR_OFFSET] = 8;
        assert!(play_options_from_save_bytes(&bytes).is_err());

        bytes[SAVE_Z_OFFSET] = 7;
        bytes[SAVE_X_OFFSET] = 8;
        assert!(play_options_from_save_bytes(&bytes).is_err());
    }

    #[test]
    fn saved_game_avatar_name_helper_uses_public_name_field() {
        let mut bytes = vec![0; SAVED_GAM_LEN];

        assert!(!saved_game_has_avatar_name(&bytes));
        bytes[SAVE_AVATAR_NAME_OFFSET + 4] = b'A';
        assert!(saved_game_has_avatar_name(&bytes));
    }

    #[test]
    fn game_clock_advances_full_britannian_date() {
        let mut clock = GameClock::with_date(139, 4, 5, 23, 59).unwrap();
        clock.advance_minutes(1);
        assert_eq!(clock, GameClock::with_date(139, 4, 6, 0, 0).unwrap());

        let mut month = GameClock::with_date(139, 4, 28, 23, 59).unwrap();
        month.advance_minutes(1);
        assert_eq!(month, GameClock::with_date(139, 5, 1, 0, 0).unwrap());

        let mut year = GameClock::with_date(139, 13, 28, 23, 59).unwrap();
        year.advance_minutes(1);
        assert_eq!(year, GameClock::with_date(140, 1, 1, 0, 0).unwrap());

        let mut max_year = GameClock::with_date(u16::MAX, 13, 28, 23, 59).unwrap();
        max_year.advance_minutes(1);
        assert_eq!(
            max_year,
            GameClock::with_date(u16::MAX, 1, 1, 0, 0).unwrap()
        );
    }

    #[test]
    fn validate_start_rejects_blocked_tiles() {
        let mut grid = open_grid();
        grid[3 * 32 + 4] = 24;

        assert!(validate_start(&grid, (4, 3), None).is_err());
        assert!(validate_start(&grid, (5, 3), None).is_ok());
    }

    #[test]
    fn tile_passability_uses_msb_first_bits() {
        let mut bytes = [0; TILE_PASSABILITY_LEN];
        bytes[0] = 0b1000_0001;
        bytes[1] = 0b1000_0000;
        let passability = TilePassability::from_bytes(&bytes).unwrap();

        assert!(passability.is_passable(0));
        assert!(!passability.is_passable(1));
        assert!(passability.is_passable(7));
        assert!(passability.is_passable(8));
    }

    #[test]
    fn tile_passability_validates_exact_size() {
        assert!(TilePassability::from_bytes(&[0; TILE_PASSABILITY_LEN - 1]).is_err());
        assert!(TilePassability::from_bytes(&[0; TILE_PASSABILITY_LEN + 1]).is_err());
    }

    #[test]
    fn optional_passability_bitmap_can_allow_class_blocked_tile() {
        let mut grid = open_grid();
        grid[3 * 32 + 4] = 24;
        let passability = passability_with_tiles(&[24]);

        assert!(validate_start(&grid, (4, 3), Some(&passability)).is_ok());
    }

    #[test]
    fn dawn_dusk_substitution_toggles_archway_pair() {
        let mut grid = open_grid();
        grid[2 * 32 + 4] = 0x87;
        grid[3 * 32 + 4] = 0x44;

        apply_dawn_dusk_substitution(&mut grid);
        assert_eq!(grid[2 * 32 + 4], 0x87);
        assert_eq!(grid[3 * 32 + 4], 0x99);

        apply_dawn_dusk_substitution(&mut grid);
        assert_eq!(grid[3 * 32 + 4], 0x44);
    }

    #[test]
    fn town_entry_applies_night_gate_substitution() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut grid = open_grid();
        grid[2 * 32 + 4] = 0x87;
        grid[3 * 32 + 4] = 0x44;
        fs::write(dir.join("CASTLE.DAT"), grid).unwrap();
        let options = PlayOptions {
            target: PlayTarget::Town(scene),
            floor: 0,
            start: Some((0, 0)),
            clock: GameClock::new(20, 0).unwrap(),
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
        };

        let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

        assert_eq!(state.grid[3 * 32 + 4], 0x99);
        assert_eq!(state.ambient_light, FULL_DARKNESS);
        assert!(state.visibility_dirty);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_turn_toggles_gate_on_dawn_and_dusk_boundaries() {
        let mut grid = open_grid();
        grid[2 * 32 + 4] = 0x87;
        grid[3 * 32 + 4] = 0x44;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(19, 59).unwrap();

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.clock, GameClock::new(20, 0).unwrap());
        assert_eq!(state.grid[3 * 32 + 4], 0x99);

        state.clock = GameClock::new(4, 59).unwrap();
        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.clock, GameClock::new(5, 0).unwrap());
        assert_eq!(state.grid[3 * 32 + 4], 0x44);
    }

    #[test]
    fn underworld_decoder_accepts_dense_public_map_shape() {
        let bytes = vec![5; UNDER_DAT_LEN];

        let grid = decode_world_map_bytes(WorldPlane::Underworld, &bytes).unwrap();

        assert_eq!(grid.len(), WORLD_CELLS);
        assert_eq!(grid[world_cell_index(255, 255)], 5);
        assert!(
            decode_world_map_bytes(WorldPlane::Underworld, &bytes[..UNDER_DAT_LEN - 1]).is_err()
        );
    }


    #[test]
    fn britannia_chunk_index_finder_uses_public_shape() {
        let table = synthetic_britannia_chunk_index();
        let mut data = vec![42; 32];
        data.extend_from_slice(&table);
        data.extend_from_slice(&[42; 32]);

        let found = find_britannia_chunk_index(&data).unwrap();

        assert_eq!(found, table);

        let mut ambiguous = Vec::new();
        ambiguous.extend_from_slice(&table);
        ambiguous.push(0);
        ambiguous.extend_from_slice(&table);
        assert!(find_britannia_chunk_index(&ambiguous).is_err());
    }

    #[test]
    fn britannia_decoder_materializes_sparse_chunks_and_water() {
        let table = synthetic_britannia_chunk_index();
        let mut bytes = vec![0; BRIT_DAT_LEN];
        for chunk in 0..BRIT_STORED_CHUNKS {
            bytes[chunk * CHUNK_BYTES..chunk * CHUNK_BYTES + CHUNK_BYTES].fill(chunk as u8);
        }

        let grid = decode_britannia_map_bytes(&bytes, &table).unwrap();

        assert_eq!(grid[world_cell_index(0, 0)], 0);
        assert_eq!(
            grid[world_cell_index(12 * CHUNK_SIDE, 12 * CHUNK_SIDE)],
            204
        );
        assert_eq!(
            grid[world_cell_index(13 * CHUNK_SIDE, 12 * CHUNK_SIDE)],
            BRIT_DEEP_WATER_TILE
        );
        assert!(decode_britannia_map_bytes(&bytes[..BRIT_DAT_LEN - 1], &table).is_err());
    }

    #[test]
    fn location_markers_use_column_major_loader_order() {
        let mut grid = open_grid();
        grid[5 * 32] = 0x2a;
        grid[32 + 1] = 0x2a;
        grid[2 * 32 + 4] = 0x48;
        grid[1] = 0x49;

        let markers = harvest_location_markers(&grid);

        assert_eq!(markers.spawn_markers, vec![(0, 5), (1, 1)]);
        assert_eq!(markers.npc_markers, vec![(1, 0), (4, 2)]);
    }

    #[test]
    fn scrub_location_entry_markers_removes_spawn_and_npc_bytes_only() {
        let mut grid = open_grid();
        grid[5 * 32] = 0x2a;
        grid[2 * 32 + 4] = 0x48;
        grid[3 * 32 + 5] = 0x49;
        grid[4 * 32 + 6] = 0xc8;

        scrub_location_entry_markers(&mut grid);

        assert_eq!(grid[5 * 32], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(grid[2 * 32 + 4], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(grid[3 * 32 + 5], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(grid[4 * 32 + 6], 0xc8);
        assert!(harvest_location_markers(&grid).spawn_markers.is_empty());
        assert!(harvest_location_markers(&grid).npc_markers.is_empty());
    }

    #[test]
    fn load_town_scene_scrubs_harvested_entry_markers_from_runtime_grid() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut grid = open_grid();
        grid[1 * 32 + 2] = 0x2a;
        grid[2 * 32 + 4] = 0x48;
        grid[3 * 32 + 5] = 0x49;
        fs::write(dir.join("CASTLE.DAT"), grid).unwrap();

        let state = PlayState::load_town_scene(&dir, scene, PlayOptions::default()).unwrap();

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.grid[1 * 32 + 2], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[2 * 32 + 4], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[3 * 32 + 5], LOCATION_MARKER_CLEANUP_TILE);
        assert!(
            harvest_location_markers(&state.grid)
                .spawn_markers
                .is_empty()
        );
        assert!(harvest_location_markers(&state.grid).npc_markers.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn movement_accepts_walkable_tiles_and_ticks_animation() {
        let mut state = test_state(open_grid(), 1, 1);
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(
            state.active_objects[0],
            ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 2,
                y: 1,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.animation.frame, 1);
        assert!(state.visibility_dirty);
    }

