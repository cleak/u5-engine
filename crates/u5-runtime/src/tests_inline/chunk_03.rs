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
        fs::write(
            dir.join("BRIT.OOL"),
            ool_plane_with_object(1, britannia_object),
        )
        .unwrap();
        fs::write(dir.join("UNDER.OOL"), vec![0x22; OOL_PLANE_LEN]).unwrap();
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
        state.climbing_gear = 1;
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
        state.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;
        state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 1;
        state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = 1;
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
        state.scroll_stock[6] = 8;
        state.potion_stock[7] = 9;
        state.reagents = [9, 8, 7, 6, 5, 4, 3, 2];
        state.moonstone_slots[2] = MoonstoneGateSlot {
            scene: 0,
            x: 77,
            y: 88,
            z: 0,
        };
        state.rare_reagent_harvest_days = [3, 4, 5];
        state.fixed_hidden_treasure_found[0] = 0x21;
        state.fixed_hidden_treasure_found[FIXED_HIDDEN_TREASURE_FOUND_BYTES - 1] = 0x80;
        state.fixed_hidden_treasure_daily_day = 6;
        state.fixed_hidden_treasure_single_use_cookie = 0x77;
        state.shadowlord_hideouts = [8, SHADOWLORD_VANQUISHED, 4];
        state.quest_progress_word = 0x120e;
        state.shrine_ordained_mask = 0b0000_1010;
        state.shrine_codex_mask = 0b0100_0001;
        state.moral_standing = 42;
        state.fortunes_of_war = 0x7e;
        state.dungeon_room_clear_bitmap[3] = 0xa5;
        state.active_player = Some(1);
        state.combat_round_counter = 7;
        state.avatar_stats = AvatarStats {
            strength: 23,
            dexterity: 24,
            intelligence: 25,
        };
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'P',
                climb_stat: 18,
                mana: 5,
                hp: 33,
                max_hp: 66,
                level: 3,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'S',
                climb_stat: 7,
                mana: 6,
                hp: 44,
                max_hp: 88,
                level: 4,
            },
        ];
        state.party_names = vec![*b"MARIA\0\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_experience = vec![350, 750];
        state.party_stay_counters = vec![4, 30];
        state.party_strengths = vec![23, 12];
        state.party_intelligence = vec![25, 13];
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
        assert_eq!(saved[SAVE_CLIMBING_GEAR_OFFSET], 1);
        assert_eq!(
            saved[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_SEXTANT_INDEX],
            1
        );
        assert_eq!(
            saved[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_SPYGLASS_INDEX],
            1
        );
        assert_eq!(
            saved[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_POCKET_WATCH_INDEX],
            1
        );
        assert_eq!(
            saved[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_BLACK_BADGE_INDEX],
            1
        );
        assert_eq!(saved[SAVE_SPELL_CHARGES_OFFSET + REL_HUR_SPELL_INDEX], 4);
        assert_eq!(saved[SAVE_SCROLL_STOCK_OFFSET + 6], 8);
        assert_eq!(saved[SAVE_POTION_STOCK_OFFSET + 7], 9);
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
        assert_eq!(saved[SAVE_MORAL_STANDING_OFFSET], 42);
        assert_eq!(saved[SAVE_FORTUNES_OF_WAR_OFFSET], 0x7e);
        assert_eq!(
            &saved[SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET
                ..SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            &state.fixed_hidden_treasure_found
        );
        assert_eq!(saved[SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET], 6);
        assert_eq!(
            saved[SAVE_FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_OFFSET],
            0x77
        );
        assert_eq!(
            &saved[SAVE_SHADOWLORD_HIDEOUTS_OFFSET
                ..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT],
            &state.shadowlord_hideouts
        );
        assert_eq!(u16_at(&saved, SAVE_QUEST_PROGRESS_WORD_OFFSET), 0x120e);
        assert_eq!(
            &saved[SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET
                ..SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET + SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            &state.dungeon_room_clear_bitmap
        );
        assert_eq!(saved[SAVE_TIMING_STATUS_TAG_OFFSET], b'Q');
        assert_eq!(saved[SAVE_ACTIVE_PLAYER_OFFSET], 1);
        assert_eq!(saved[SAVE_COMBAT_ROUND_COUNTER_OFFSET], 7);
        assert_eq!(
            saved[SAVE_TRANSPORT_MARKER_OFFSET],
            TRANSPORT_MARKER_SKIFF_FIRST
        );
        assert_eq!(saved[SAVE_WIND_OFFSET], 9);
        assert_eq!(saved[SAVE_PARTY_SIZE_OFFSET], 2);
        assert_eq!(
            &saved[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + SAVE_CHARACTER_NAME_LEN],
            b"MARIA\0\0\0\0"
        );
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_STATUS_OFFSET],
            b'P'
        );
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_CLASS_OFFSET],
            b'A'
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
        assert_eq!(
            u16_at(&saved, SAVE_ROSTER_OFFSET + SAVE_CHARACTER_EXPERIENCE_OFFSET),
            350
        );
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_STAY_COUNTER_OFFSET],
            4
        );
        let second = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        assert_eq!(&saved[second..second + SAVE_CHARACTER_NAME_LEN], b"IOLO\0\0\0\0\0");
        assert_eq!(saved[second + SAVE_CHARACTER_CLASS_OFFSET], b'B');
        assert_eq!(saved[second + SAVE_CHARACTER_STATUS_OFFSET], b'S');
        assert_eq!(saved[second + SAVE_CHARACTER_STR_OFFSET], 12);
        assert_eq!(saved[second + SAVE_CHARACTER_DEX_OFFSET], 7);
        assert_eq!(saved[second + SAVE_CHARACTER_INT_OFFSET], 13);
        assert_eq!(saved[second + SAVE_CHARACTER_MANA_OFFSET], 6);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_HP_OFFSET), 44);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_MAX_HP_OFFSET), 88);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_EXPERIENCE_OFFSET), 750);
        assert_eq!(
            saved[second + SAVE_CHARACTER_STAY_COUNTER_OFFSET],
            INN_STAY_COUNTER_CAP
        );
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
        assert_eq!(
            fs::read(dir.join("BRIT.OOL")).unwrap(),
            saved_ool[..OOL_PLANE_LEN].to_vec()
        );
        assert_eq!(
            fs::read(dir.join("UNDER.OOL")).unwrap(),
            saved_ool[OOL_PLANE_LEN..].to_vec()
        );
        let world_progress = load_world_progress_state(&dir).unwrap();
        assert_eq!(world_progress, WorldProgressState::from_play_state(&state));
        let reloaded_options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(
            WorldProgressState::from_play_options(&reloaded_options),
            world_progress
        );
        assert!(state.message.contains("Done."));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_stages_inactive_plane_from_per_plane_mirror() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(0, 0xff, 10, 20);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        let mut stale_saved_ool = vec![0; SAVED_OOL_LEN];
        stale_saved_ool[OOL_PLANE_LEN + OOL_RECORD_LEN] = 0x33;
        stale_saved_ool[OOL_PLANE_LEN + OOL_RECORD_LEN + 1] = 0x33;
        fs::write(dir.join(SAVED_OOL_FILENAME), stale_saved_ool).unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
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
        fs::write(
            dir.join(UNDER_OOL_FILENAME),
            ool_plane_with_object(1, underworld_object),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.active_objects[0].z = WorldPlane::Britannia.save_floor();

        state.save_game_command(&dir, Some(true)).unwrap();

        let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
        let underworld_overlay = decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..]).unwrap();
        assert_eq!(underworld_overlay[0], underworld_object);
        assert_eq!(
            fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(),
            saved_ool[OOL_PLANE_LEN..].to_vec()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_persists_pending_shipwright_delivery_from_town_return_world() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(17, 0, 3, 4);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
        fs::write(dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();

        let stale_cached_object = ActiveObject {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
            x: 1,
            y: 2,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let pending = PendingVehicleAcquisition::Frigate {
            x: 136,
            y: 158,
            skiffs: 3,
        };
        let expected = pending.active_object(WorldPlane::Britannia.save_floor());
        let mut state = test_state(open_grid(), 3, 4);
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Britannia,
            x: 12,
            y: 21,
            transport: TransportState::Foot,
            timing_status: TimingStatusTag::Normal,
            sail_cadence: 0,
            sail_stall_pending: false,
            grid: open_world_grid(),
            active_objects: vec![ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 12,
                y: 21,
                z: WorldPlane::Britannia.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }],
            pending_vehicle: Some(pending),
        });
        state
            .world_overlays
            .set(WorldPlane::Britannia, vec![stale_cached_object; OOL_SLOTS - 1]);

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
        let britannia_overlay = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
        assert_eq!(britannia_overlay[0], expected);
        assert!(!britannia_overlay.iter().any(|object| *object == stale_cached_object));
        assert_eq!(
            fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
            saved_ool[..OOL_PLANE_LEN].to_vec()
        );
        assert_eq!(
            load_world_overlay_objects(&dir, WorldPlane::Britannia).unwrap()[0],
            expected
        );
        assert_eq!(
            state.return_world.as_ref().and_then(|world| world.pending_vehicle),
            Some(pending)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_persists_pending_skiff_delivery_from_town_return_world() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(17, 0, 3, 4);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
        fs::write(dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();

        let pending = PendingVehicleAcquisition::Skiff { x: 85, y: 107 };
        let expected = pending.active_object(WorldPlane::Britannia.save_floor());
        let mut world_objects = vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 81,
            y: 106,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }];
        world_objects.push(ActiveObject {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            x: 82,
            y: 106,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
            aux3: 1,
        });
        let mut state = test_state(open_grid(), 3, 4);
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Britannia,
            x: 81,
            y: 106,
            transport: TransportState::Foot,
            timing_status: TimingStatusTag::Normal,
            sail_cadence: 0,
            sail_stall_pending: false,
            grid: open_world_grid(),
            active_objects: world_objects,
            pending_vehicle: Some(pending),
        });

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
        let britannia_overlay = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
        assert_eq!(
            britannia_overlay
                .iter()
                .filter(|object| object.type_byte == FIRST_PLAYABLE_FRIGATE_TILE)
                .count(),
            1
        );
        assert!(britannia_overlay.iter().any(|object| *object == expected));
        assert_eq!(
            fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
            saved_ool[..OOL_PLANE_LEN].to_vec()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_writes_foot_transport_marker_with_current_facing() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(0, 0xff, 10, 20);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
        fs::write(dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();

        let mut state = world_state(open_world_grid(), 10, 20);
        state.player.transport = TransportState::Foot;
        state.player.facing = Direction::West;

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(saved[SAVE_TRANSPORT_MARKER_OFFSET], TRANSPORT_MARKER_FOOT_LAST);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_does_not_persist_transient_potion_presentation() {
        let base_dir = debug_game_dir();
        let transient_dir = debug_game_dir();
        for dir in [&base_dir, &transient_dir] {
            let mut template = saved_game_seed_bytes(0, 0xff, 10, 20);
            template[SAVE_AVATAR_NAME_OFFSET] = b'A';
            fs::write(dir.join("SAVED.GAM"), template).unwrap();
            fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();
        }

        let mut base = world_state(open_world_grid(), 10, 20);
        assert_eq!(
            base.save_game_command(&base_dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let base_gam = fs::read(base_dir.join("SAVED.GAM")).unwrap();
        let base_ool = fs::read(base_dir.join("SAVED.OOL")).unwrap();

        let mut transient = world_state(open_world_grid(), 10, 20);
        transient.white_potion_sweep = Some(WhitePotionSweep {
            frames_remaining: 7,
            radius: 3,
            center_x: 10,
            center_y: 20,
        });
        transient.combat_potion_presentation = Some(CombatPotionPresentation {
            kind: CombatPotionPresentationKind::Poof,
            actor_slot: 0,
            active_object_slot: 0,
            frames_remaining: 1,
        });
        assert_eq!(
            transient
                .save_game_command(&transient_dir, Some(true))
                .unwrap(),
            MoveOutcome::Saved
        );

        assert_eq!(fs::read(transient_dir.join("SAVED.GAM")).unwrap(), base_gam);
        assert_eq!(fs::read(transient_dir.join("SAVED.OOL")).unwrap(), base_ool);
        let _ = fs::remove_dir_all(base_dir);
        let _ = fs::remove_dir_all(transient_dir);
    }

    #[test]
    fn load_scene_starts_with_no_transient_potion_presentation() {
        let dir = debug_game_dir();
        let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), save).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();
        let state = PlayState::load_scene(&dir, options).unwrap();

        assert_eq!(state.white_potion_sweep, None);
        assert_eq!(state.combat_potion_presentation, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_progress_state_codec_round_trips() {
        let mut progress = WorldProgressState::default();
        progress.rare_reagent_harvest_days = [7, 8, 9];
        progress.fixed_hidden_treasure_mirror_present = true;
        progress.fixed_hidden_treasure_found[0] = 0x01;
        progress.fixed_hidden_treasure_found[7] = 0xa5;
        progress.fixed_hidden_treasure_found[FIXED_HIDDEN_TREASURE_FOUND_BYTES - 1] = 0x40;
        progress.fixed_hidden_treasure_daily_day = 12;
        progress.fixed_hidden_treasure_single_use_cookie = Some(0x55);
        progress.shadowlord_mirror_present = true;
        progress.shadowlord_hideouts = [SHADOWLORD_VANQUISHED, 3, 6];

        let encoded = progress.encoded();
        assert_eq!(encoded.len(), WORLD_PROGRESS_STATE_LEN);
        assert_eq!(WorldProgressState::decode(&encoded).unwrap(), progress);
        let mut legacy = encoded.to_vec();
        legacy.extend_from_slice(&[10, 20, 30, 40, 50, 60, 70, 80]);
        assert_eq!(legacy.len(), WORLD_PROGRESS_STATE_LEN + VIRTUE_COUNT);
        assert_eq!(WorldProgressState::decode(&legacy).unwrap(), progress);
        let mut old = encoded.to_vec();
        old.remove(WORLD_PROGRESS_STATE_LEN - SHADOWLORD_COUNT - 1);
        old.truncate(WORLD_PROGRESS_STATE_LEN - 1);
        let old_decoded = WorldProgressState::decode(&old).unwrap();
        assert_eq!(old_decoded.fixed_hidden_treasure_single_use_cookie, None);
        assert_eq!(old_decoded.shadowlord_hideouts, progress.shadowlord_hideouts);
        assert!(WorldProgressState::decode(&[0; WORLD_PROGRESS_STATE_LEN]).is_err());
        assert!(WorldProgressState::decode(&encoded[..WORLD_PROGRESS_STATE_LEN - 2]).is_err());
    }

    #[test]
    fn missing_world_progress_sidecar_loads_default_state() {
        let dir = debug_game_dir();
        let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), save).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();

        assert_eq!(
            options.rare_reagent_harvest_days,
            [RARE_REAGENT_HARVEST_UNSEEN_DAY; RARE_REAGENT_HARVEST_POINT_COUNT]
        );
        assert_eq!(options.fixed_hidden_treasure_found, [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES]);
        assert_eq!(
            options.fixed_hidden_treasure_daily_day,
            FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY
        );
        assert_eq!(
            options.fixed_hidden_treasure_single_use_cookie,
            FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_CLEAR
        );
        assert_eq!(options.shadowlord_hideouts, DEFAULT_SHADOWLORD_HIDEOUTS);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_save_decodes_fixed_hidden_treasure_state() {
        let mut bytes = saved_game_seed_bytes(0, 0xff, 10, 20);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET] = 0x05;
        bytes[SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET + FIXED_HIDDEN_TREASURE_FOUND_BYTES - 1] =
            0x80;
        bytes[SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET] = 0x12;
        bytes[SAVE_FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_OFFSET] = 0x34;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.fixed_hidden_treasure_found[0], 0x05);
        assert_eq!(
            options.fixed_hidden_treasure_found[FIXED_HIDDEN_TREASURE_FOUND_BYTES - 1],
            0x80
        );
        assert_eq!(options.fixed_hidden_treasure_daily_day, 0x12);
        assert_eq!(options.fixed_hidden_treasure_single_use_cookie, 0x34);
    }

    #[test]
    fn from_save_keeps_native_hidden_treasure_state_over_sidecar_mirror() {
        let dir = debug_game_dir();
        let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        save[SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET] = 0x05;
        save[SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET] = 0x12;
        save[SAVE_FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_OFFSET] = 0x34;
        save[SAVE_SHADOWLORD_HIDEOUTS_OFFSET..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
            .copy_from_slice(&[8, SHADOWLORD_VANQUISHED, 4]);
        write_u16_at(&mut save, SAVE_QUEST_PROGRESS_WORD_OFFSET, 0x120e);
        fs::write(dir.join(SAVED_GAM_FILENAME), save).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();

        let mut progress = WorldProgressState::default();
        progress.rare_reagent_harvest_days = [7, 8, 9];
        progress.fixed_hidden_treasure_mirror_present = true;
        progress.fixed_hidden_treasure_found[0] = 0xa0;
        progress.fixed_hidden_treasure_daily_day = 0x56;
        progress.fixed_hidden_treasure_single_use_cookie = Some(0x78);
        progress.shadowlord_mirror_present = true;
        progress.shadowlord_hideouts = [1, 2, 3];
        write_world_progress_state(&dir, progress).unwrap();

        let options = load_play_options_from_save(&dir).unwrap();

        assert_eq!(options.rare_reagent_harvest_days, [7, 8, 9]);
        assert_eq!(options.fixed_hidden_treasure_found[0], 0x05);
        assert_eq!(options.fixed_hidden_treasure_daily_day, 0x12);
        assert_eq!(options.fixed_hidden_treasure_single_use_cookie, 0x34);
        assert_eq!(options.shadowlord_hideouts, [8, SHADOWLORD_VANQUISHED, 4]);
        assert_eq!(options.quest_progress_word, 0x120e);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn from_save_decodes_shadowlord_hideout_slots() {
        let mut bytes = saved_game_seed_bytes(0, 0xff, 10, 20);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SHADOWLORD_HIDEOUTS_OFFSET..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
            .copy_from_slice(&[8, SHADOWLORD_VANQUISHED, 4]);
        write_u16_at(&mut bytes, SAVE_QUEST_PROGRESS_WORD_OFFSET, 0x120e);

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.shadowlord_hideouts, [8, SHADOWLORD_VANQUISHED, 4]);
        assert_eq!(options.quest_progress_word, 0x120e);
    }

    #[test]
    fn from_save_decodes_dungeon_room_clear_bitmap() {
        let mut bytes = saved_game_seed_bytes(33, 0, 1, 1);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET + 14] = 0x40;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.dungeon_room_clear_bitmap[14], 0x40);
        assert_eq!(
            options.dungeon_room_clear_bitmap
                [SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN - 1],
            0
        );
    }

    #[test]
    fn from_save_decodes_dungeon_working_buffer_for_dungeon_scene() {
        let mut bytes = saved_game_seed_bytes(33, 0, 1, 1);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        let buffer_start = SAVE_DUNGEON_WORKING_BUFFER_OFFSET;
        let buffer_end = buffer_start + SAVE_DUNGEON_WORKING_BUFFER_LEN;
        bytes[buffer_start..buffer_end].fill(0x44);
        bytes[buffer_start + dungeon_cell_index(0, 2, 1)] = 0x68;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        let buffer = options
            .saved_dungeon_working_buffer
            .expect("dungeon saves carry the live working buffer");
        assert_eq!(buffer.len(), SAVE_DUNGEON_WORKING_BUFFER_LEN);
        assert_eq!(buffer[dungeon_cell_index(0, 2, 1)], 0x68);

        bytes[SAVE_SCENE_OFFSET] = 0;
        let world_options = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(world_options.saved_dungeon_working_buffer, None);
    }

    #[test]
    fn dungeon_save_writes_live_working_buffer() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(33, 0, 1, 1);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        template[SAVE_DUNGEON_WORKING_BUFFER_OFFSET..SAVE_DUNGEON_WORKING_BUFFER_OFFSET + SAVE_DUNGEON_WORKING_BUFFER_LEN]
            .fill(0x7f);
        fs::write(dir.join("SAVED.GAM"), template).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x68;
        grid[dungeon_cell_index(0, 3, 1)] = 0x8a;
        let mut state = dungeon_state(grid.clone(), 0, 1, 1);

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(
            &saved[SAVE_DUNGEON_WORKING_BUFFER_OFFSET
                ..SAVE_DUNGEON_WORKING_BUFFER_OFFSET + SAVE_DUNGEON_WORKING_BUFFER_LEN],
            &grid[..]
        );
        let _ = fs::remove_dir_all(dir);
    }

    fn write_save_template_and_empty_overlays(dir: &Path, scene: u8, z: u8, x: u8, y: u8) {
        let mut template = saved_game_seed_bytes(scene, z, x, y);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
        fs::write(dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
    }

    #[test]
    fn town_floor_transition_save_load_round_trips_floor_and_active_table() {
        let dir = debug_game_dir();
        write_castle_trap_door_fixture(&dir);
        write_save_template_and_empty_overlays(&dir, 17, 0, 1, 1);
        let scene = Scene::new(17).unwrap();
        let mut state = town_trap_door_origin_state();

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedFloor { scene, floor: -1 })
        );
        let floor_object = ActiveObject {
            type_byte: 0x77,
            tile: 0x77,
            x: 3,
            y: 1,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0x22,
            aux3: 0x33,
        };
        state.active_objects.push(floor_object);

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(saved[SAVE_SCENE_OFFSET], scene.byte);
        assert_eq!(saved[SAVE_Z_OFFSET], 0xff);
        assert_eq!(saved[SAVE_X_OFFSET], 1);
        assert_eq!(saved[SAVE_Y_OFFSET], 1);
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.target, PlayTarget::Town(scene));
        assert_eq!(options.floor, -1);
        assert_eq!(options.start, Some((1, 1)));
        assert_eq!(
            options.saved_active_objects.as_ref().unwrap()[0],
            floor_object
        );

        let reloaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(reloaded.area, Area::Town { scene, floor: -1 });
        assert_eq!((reloaded.player.x, reloaded.player.y), (1, 1));
        assert_eq!(reloaded.active_objects[0].z, -1);
        assert_eq!(reloaded.active_objects[1], floor_object);
        assert_eq!(reloaded.grid[32 + 1], 4);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_teleport_save_load_round_trips_level_and_working_buffer() {
        let dir = debug_game_dir();
        write_save_template_and_empty_overlays(&dir, 33, 0, 1, 1);
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 2 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 3 })
        );
        let patched_cell = dungeon_cell_index(3, 5, 5);
        state.grid[patched_cell] = 0x68;

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(saved[SAVE_SCENE_OFFSET], scene.byte);
        assert_eq!(saved[SAVE_Z_OFFSET], 3);
        assert_eq!(saved[SAVE_X_OFFSET], 4);
        assert_eq!(saved[SAVE_Y_OFFSET], 5);
        assert_eq!(
            saved[SAVE_DUNGEON_WORKING_BUFFER_OFFSET + patched_cell],
            0x68
        );
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.target, PlayTarget::Dungeon(scene));
        assert_eq!(options.floor, 3);
        assert_eq!(options.start, Some((4, 5)));
        assert_eq!(
            options.saved_dungeon_working_buffer.as_ref().unwrap()[patched_cell],
            0x68
        );

        let reloaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(reloaded.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((reloaded.player.x, reloaded.player.y), (4, 5));
        assert_eq!(reloaded.active_objects[0].z, 3);
        assert_eq!(reloaded.grid[patched_cell], 0x68);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_save_load_round_trips_returned_underworld_state() {
        let dir = debug_game_dir();
        write_save_template_and_empty_overlays(&dir, 33, 0, 1, 1);
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );
        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(saved[SAVE_SCENE_OFFSET], 0);
        assert_eq!(saved[SAVE_Z_OFFSET], 0xff);
        assert_eq!(saved[SAVE_X_OFFSET], 10);
        assert_eq!(saved[SAVE_Y_OFFSET], 20);
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
        assert_eq!(options.start, Some((10, 20)));
        assert_eq!(options.saved_dungeon_working_buffer, None);

        let reloaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(
            reloaded.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((reloaded.player.x, reloaded.player.y), (10, 20));
        assert_eq!(
            reloaded.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(reloaded.grid[world_cell_index(10, 20)], 5);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_game_command_writes_inn_registry_view() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(17, 0, 15, 15);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), template).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.inn_registry.push(InnGuestRecord {
            scene_marker: 0x12,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            member: PartyMember {
                slot: 0,
                class_byte: b'B',
                status: b'P',
                climb_stat: 7,
                mana: 3,
                hp: 12,
                max_hp: 28,
                level: 3,
            },
            strength: 17,
            intelligence: 19,
            experience: 700,
            equipment: [1, 2, 3, 4, 5, 6],
            stay_counter: 4,
        });

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        let registry = decode_inn_registry(&saved);
        assert_eq!(registry, state.inn_registry);
        assert_eq!(
            saved[SAVE_INN_REGISTRY_OFFSET + SAVE_CHARACTER_RECORD_LEN],
            0
        );
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
        assert_eq!(state.wind_status_message(), "Winds");
        assert!(state.message.ends_with("Winds."));
        assert!(state.z_stats_message().contains("wind Winds;"));

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_WIND_OFFSET], 0x7a);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_load_maps_public_wind_save_values() {
        let mut save = saved_game_seed_bytes(0, 0xff, 10, 20);
        save[SAVE_AVATAR_NAME_OFFSET] = b'A';
        save[SAVE_WIND_OFFSET] = 3;

        let options = play_options_from_save_bytes(&save).unwrap();

        assert_eq!(options.wind, WindState::East);
        assert_eq!(options.wind_save_byte, 3);
    }

    #[test]
    fn inn_registry_decodes_nonzero_scene_markers_from_shifted_save_view() {
        let mut bytes = saved_game_seed_bytes(17, 0, 5, 5);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        let record = SAVE_INN_REGISTRY_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        bytes[record] = 0x12;
        bytes[record + SAVE_CHARACTER_CLASS_OFFSET] = b'B';
        bytes[record + SAVE_CHARACTER_STATUS_OFFSET] = b'P';
        bytes[record + SAVE_CHARACTER_STR_OFFSET] = 17;
        bytes[record + SAVE_CHARACTER_DEX_OFFSET] = 7;
        bytes[record + SAVE_CHARACTER_INT_OFFSET] = 19;
        bytes[record + SAVE_CHARACTER_MANA_OFFSET] = 3;
        write_u16_at(&mut bytes, record + SAVE_CHARACTER_HP_OFFSET, 12);
        write_u16_at(&mut bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET, 28);
        write_u16_at(&mut bytes, record + SAVE_CHARACTER_EXPERIENCE_OFFSET, 700);
        bytes[record + SAVE_CHARACTER_LEVEL_OFFSET] = 3;
        bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET] = 4;
        bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
            ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT]
            .copy_from_slice(&[1, 2, 3, 4, 5, 6]);

        let registry = decode_inn_registry(&bytes);

        assert_eq!(registry.len(), 1);
        assert_eq!(registry[0].scene_marker, 0x12);
        assert_eq!(registry[0].member.slot, 1);
        assert_eq!(registry[0].member.class_byte, b'B');
        assert_eq!(registry[0].member.status, b'P');
        assert_eq!(registry[0].member.hp, 12);
        assert_eq!(registry[0].member.max_hp, 28);
        assert_eq!(registry[0].strength, 17);
        assert_eq!(registry[0].intelligence, 19);
        assert_eq!(registry[0].experience, 700);
        assert_eq!(registry[0].equipment, [1, 2, 3, 4, 5, 6]);
        assert_eq!(registry[0].stay_counter, 4);

        let options = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(options.inn_registry, registry);
    }

    #[test]
    fn inn_registry_encoding_writes_known_guest_fields_and_clears_markers() {
        let mut bytes = vec![0xff; SAVED_GAM_LEN];
        let guest = InnGuestRecord {
            scene_marker: 0x11,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            member: PartyMember {
                slot: 5,
                class_byte: b'M',
                status: b'G',
                climb_stat: 8,
                mana: 9,
                hp: 42,
                max_hp: 84,
                level: 4,
            },
            strength: 14,
            intelligence: 22,
            experience: 1234,
            equipment: [6, 5, 4, 3, 2, 1],
            stay_counter: 30,
        };

        encode_inn_registry(&mut bytes, &[guest]);

        let record = SAVE_INN_REGISTRY_OFFSET;
        assert_eq!(bytes[record], 0x11);
        assert_eq!(bytes[record + SAVE_CHARACTER_CLASS_OFFSET], b'M');
        assert_eq!(bytes[record + SAVE_CHARACTER_STATUS_OFFSET], b'G');
        assert_eq!(bytes[record + SAVE_CHARACTER_STR_OFFSET], 14);
        assert_eq!(bytes[record + SAVE_CHARACTER_DEX_OFFSET], 8);
        assert_eq!(bytes[record + SAVE_CHARACTER_INT_OFFSET], 22);
        assert_eq!(bytes[record + SAVE_CHARACTER_MANA_OFFSET], 9);
        assert_eq!(u16_at(&bytes, record + SAVE_CHARACTER_HP_OFFSET), 42);
        assert_eq!(u16_at(&bytes, record + SAVE_CHARACTER_MAX_HP_OFFSET), 84);
        assert_eq!(
            u16_at(&bytes, record + SAVE_CHARACTER_EXPERIENCE_OFFSET),
            1234
        );
        assert_eq!(bytes[record + SAVE_CHARACTER_LEVEL_OFFSET], 4);
        assert_eq!(
            bytes[record + SAVE_CHARACTER_STAY_COUNTER_OFFSET],
            INN_STAY_COUNTER_CAP
        );
        assert_eq!(
            &bytes[record + SAVE_CHARACTER_EQUIPMENT_OFFSET
                ..record + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIPMENT_SLOT_COUNT],
            &[6, 5, 4, 3, 2, 1]
        );
        assert_eq!(bytes[SAVE_INN_REGISTRY_OFFSET + SAVE_CHARACTER_RECORD_LEN], 0);
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
        bytes[SAVE_MORAL_STANDING_OFFSET] = 37;
        bytes[SAVE_GRAPPLE_OFFSET] = 1;
        bytes[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_SEXTANT_INDEX] = 1;
        bytes[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_SPYGLASS_INDEX] = 1;
        bytes[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;
        bytes[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_BLACK_BADGE_INDEX] = 1;
        bytes[SAVE_FORTUNES_OF_WAR_OFFSET] = 0x99;
        bytes[SAVE_COMBAT_ROUND_COUNTER_OFFSET] = 8;
        let clock = GameClock::with_date(141, 6, 7, 8, 35).unwrap();
        write_saved_clock(&mut bytes, clock);

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.target, PlayTarget::Town(Scene::new(13).unwrap()));
        assert_eq!(options.floor, 0);
        assert_eq!(options.start, Some((15, 15)));
        assert_eq!(options.clock, clock);
        assert_eq!(options.wind, WindState::Calm);
        assert_eq!(options.keys, 0);
        assert_eq!(options.moral_standing, 37);
        assert_eq!(SAVE_GRAPPLE_OFFSET, SAVE_CLIMBING_GEAR_OFFSET);
        assert_eq!(options.climbing_gear, 1);
        assert_eq!(options.special_items[SPECIAL_ITEM_SEXTANT_INDEX], 1);
        assert_eq!(options.special_items[SPECIAL_ITEM_SPYGLASS_INDEX], 1);
        assert_eq!(options.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX], 1);
        assert_eq!(options.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX], 1);
        assert_eq!(options.fortunes_of_war, 0x99);
        assert_eq!(options.combat_round_counter, 8);
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
        bytes[SAVE_SCROLL_STOCK_OFFSET + 6] = 4;
        bytes[SAVE_POTION_STOCK_OFFSET + 7] = 5;
        bytes[SAVE_MOONSTONE_X_OFFSET + 1] = 22;
        bytes[SAVE_MOONSTONE_Y_OFFSET + 1] = 23;
        bytes[SAVE_MOONSTONE_SCENE_OFFSET + 1] = 0;
        bytes[SAVE_MOONSTONE_Z_OFFSET + 1] = 0xff;
        bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = 0b0010_0010;
        bytes[SAVE_SHRINE_CODEX_MASK_OFFSET] = 0b1000_0001;
        let first = SAVE_ROSTER_OFFSET;
        bytes[first..first + SAVE_CHARACTER_NAME_LEN].copy_from_slice(b"MARIA\0\0\0\0");
        bytes[first + SAVE_CHARACTER_CLASS_OFFSET] = b'A';
        bytes[first + SAVE_CHARACTER_STATUS_OFFSET] = b'G';
        bytes[first + SAVE_CHARACTER_STR_OFFSET] = 11;
        bytes[first + SAVE_CHARACTER_DEX_OFFSET] = 18;
        bytes[first + SAVE_CHARACTER_INT_OFFSET] = 19;
        bytes[first + SAVE_CHARACTER_MANA_OFFSET] = 12;
        bytes[first + SAVE_CHARACTER_HP_OFFSET] = 44;
        bytes[first + SAVE_CHARACTER_HP_OFFSET + 1] = 1;
        bytes[first + SAVE_CHARACTER_MAX_HP_OFFSET] = 194;
        bytes[first + SAVE_CHARACTER_MAX_HP_OFFSET + 1] = 1;
        write_u16_at(&mut bytes, first + SAVE_CHARACTER_EXPERIENCE_OFFSET, 350);
        bytes[first + SAVE_CHARACTER_STAY_COUNTER_OFFSET] = 3;
        bytes[first + SAVE_CHARACTER_LEVEL_OFFSET] = 5;
        let second = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        bytes[second..second + SAVE_CHARACTER_NAME_LEN].copy_from_slice(b"IOLO\0\0\0\0\0");
        bytes[second + SAVE_CHARACTER_CLASS_OFFSET] = b'B';
        bytes[second + SAVE_CHARACTER_STATUS_OFFSET] = b'D';
        bytes[second + SAVE_CHARACTER_STR_OFFSET] = 9;
        bytes[second + SAVE_CHARACTER_DEX_OFFSET] = 7;
        bytes[second + SAVE_CHARACTER_INT_OFFSET] = 13;
        bytes[second + SAVE_CHARACTER_MANA_OFFSET] = 2;
        bytes[second + SAVE_CHARACTER_HP_OFFSET] = 0;
        bytes[second + SAVE_CHARACTER_HP_OFFSET + 1] = 0;
        bytes[second + SAVE_CHARACTER_MAX_HP_OFFSET] = 120;
        write_u16_at(&mut bytes, second + SAVE_CHARACTER_EXPERIENCE_OFFSET, 750);
        bytes[second + SAVE_CHARACTER_STAY_COUNTER_OFFSET] = 4;
        bytes[second + SAVE_CHARACTER_LEVEL_OFFSET] = 1;
        let third = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN * 2;
        bytes[third..third + SAVE_CHARACTER_NAME_LEN].copy_from_slice(b"GWENNO\0\0\0");
        bytes[third + SAVE_CHARACTER_CLASS_OFFSET] = b'D';
        bytes[third + SAVE_CHARACTER_STATUS_OFFSET] = b'G';
        bytes[third + SAVE_CHARACTER_STR_OFFSET] = 12;
        bytes[third + SAVE_CHARACTER_DEX_OFFSET] = 14;
        bytes[third + SAVE_CHARACTER_INT_OFFSET] = 16;
        bytes[third + SAVE_CHARACTER_MANA_OFFSET] = 5;
        bytes[third + SAVE_CHARACTER_HP_OFFSET] = 40;
        bytes[third + SAVE_CHARACTER_MAX_HP_OFFSET] = 80;
        write_u16_at(&mut bytes, third + SAVE_CHARACTER_EXPERIENCE_OFFSET, 900);
        bytes[third + SAVE_CHARACTER_STAY_COUNTER_OFFSET] = 7;
        bytes[third + SAVE_CHARACTER_LEVEL_OFFSET] = 4;

        let options = play_options_from_save_bytes(&bytes).unwrap();

        assert_eq!(options.spell_charges[REL_HUR_SPELL_INDEX], 3);
        assert_eq!(options.scroll_stock[6], 4);
        assert_eq!(options.potion_stock[7], 5);
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
        assert_eq!(options.party_experience, vec![350, 750]);
        assert_eq!(
            options.party_names,
            vec![*b"MARIA\0\0\0\0", *b"IOLO\0\0\0\0\0"]
        );
        assert_eq!(options.party_stay_counters, vec![3, 4]);
        assert_eq!(options.party_strengths, vec![11, 9]);
        assert_eq!(options.party_intelligence, vec![19, 13]);
        assert_eq!(
            options.party,
            vec![
                PartyMember {
                    slot: 0,
                    class_byte: b'A',
                    status: b'G',
                    climb_stat: 18,
                    mana: 12,
                    hp: 300,
                    max_hp: 450,
                    level: 5,
                },
                PartyMember {
                    slot: 1,
                    class_byte: b'B',
                    status: b'D',
                    climb_stat: 7,
                    mana: 2,
                    hp: 0,
                    max_hp: 120,
                    level: 1,
                },
            ]
        );
        assert_eq!(options.party_roster.len(), SAVE_ROSTER_SLOT_COUNT);
        assert_eq!(options.party_roster[2].name, *b"GWENNO\0\0\0");
        assert_eq!(options.party_roster[2].experience, 900);
        assert_eq!(options.party_roster[2].stay_counter, 7);
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
    fn save_play_options_reads_public_transport_markers() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 0;
        bytes[SAVE_Z_OFFSET] = 0xff;
        bytes[SAVE_X_OFFSET] = 200;
        bytes[SAVE_Y_OFFSET] = 201;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 1;
        let hoisted = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(hoisted.facing, Some(Direction::East));
        assert_eq!(
            hoisted.transport,
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 1,
                tile: FIRST_PLAYABLE_FRIGATE_TILE + 1,
                sails_hoisted: true,
                hull: 0,
                skiffs: 0,
            }
        );

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = TRANSPORT_MARKER_SHIP_FURLED_FIRST + 2;
        let furled = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(furled.facing, Some(Direction::South));
        assert_eq!(
            furled.transport,
            TransportState::Ship {
                type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST + 2,
                tile: FIRST_PLAYABLE_FRIGATE_TILE + 2,
                sails_hoisted: false,
                hull: 0,
                skiffs: 0,
            }
        );

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = TRANSPORT_MARKER_MAGIC_CARPET_LAST;
        let carpet = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(carpet.facing, Some(Direction::West));
        assert_eq!(
            carpet.transport,
            TransportState::Carpet {
                type_byte: TRANSPORT_MARKER_MAGIC_CARPET_LAST,
                tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE + 3,
            }
        );

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = TRANSPORT_MARKER_SKIFF_LAST;
        let skiff = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(skiff.facing, Some(Direction::West));
        assert_eq!(
            skiff.transport,
            TransportState::Skiff {
                type_byte: TRANSPORT_MARKER_SKIFF_LAST,
                tile: FIRST_PLAYABLE_SKIFF_TILE + 3,
            }
        );

        bytes[SAVE_TRANSPORT_MARKER_OFFSET] = TRANSPORT_MARKER_FOOT_LAST;
        let foot = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(foot.transport, TransportState::Foot);
        assert_eq!(foot.facing, Some(Direction::West));
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
        assert_eq!(unknown.timing_status, TimingStatusTag::Opaque(b'X'));
        assert_eq!(unknown.timing_status.save_byte(), b'X');

        bytes[SAVE_TIMING_STATUS_TAG_OFFSET] = OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG;
        let underfoot_exempt = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(
            underfoot_exempt.timing_status,
            TimingStatusTag::Opaque(OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG)
        );
        assert_eq!(
            underfoot_exempt.timing_status.save_byte(),
            OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG
        );
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
    fn save_play_options_reject_empty_saved_game_before_scene_dispatch() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_AVATAR_NAME_OFFSET + 4] = b'A';
        bytes[SAVE_SCENE_OFFSET] = 0;
        bytes[SAVE_Z_OFFSET] = 0;
        bytes[SAVE_X_OFFSET] = 15;
        bytes[SAVE_Y_OFFSET] = 15;
        write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());

        let err = play_options_from_save_bytes(&bytes).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("No active game"));
    }

    #[test]
    fn saved_game_avatar_name_helper_uses_first_public_name_byte() {
        let mut bytes = vec![0; SAVED_GAM_LEN];

        assert!(!saved_game_has_avatar_name(&bytes));
        bytes[SAVE_AVATAR_NAME_OFFSET + 4] = b'A';
        assert!(!saved_game_has_avatar_name(&bytes));
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        assert!(saved_game_has_avatar_name(&bytes));
    }

    #[test]
    fn decode_party_names_reads_active_roster_names_with_fixed_stride() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_PARTY_SIZE_OFFSET] = 2;
        bytes[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + 5].copy_from_slice(b"AVATR");
        let second = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        bytes[second..second + 5].copy_from_slice(b"Saduj");

        let names = decode_party_names(&bytes);

        assert_eq!(names.len(), 2);
        assert_eq!(&names[0][..5], b"AVATR");
        assert_eq!(&names[1][..5], b"Saduj");
        assert!(party_name_forces_monster_combat_group(&names[1]));
    }

    #[test]
    fn decode_party_names_rejects_invalid_party_size_like_party_decoder() {
        let mut bytes = vec![0; SAVED_GAM_LEN];
        bytes[SAVE_PARTY_SIZE_OFFSET] = 7;
        bytes[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + 5].copy_from_slice(b"AVATR");

        assert!(decode_party_names(&bytes).is_empty());
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
    fn turn_advance_clears_fortunes_of_war_on_month_boundary_only() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 27, 23, 59).unwrap();
        state.fortunes_of_war = 0x55;

        state.advance_turn_with_minutes(1);
        assert_eq!(state.clock, GameClock::with_date(139, 4, 28, 0, 0).unwrap());
        assert_eq!(state.fortunes_of_war, 0x55);

        state.clock = GameClock::with_date(139, 4, 28, 23, 59).unwrap();
        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock, GameClock::with_date(139, 5, 1, 0, 0).unwrap());
        assert_eq!(state.fortunes_of_war, 0);
    }

    #[test]
    fn hourly_status_pass_spends_food_at_public_decrement_hours() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, 5, 59).unwrap();
        state.food = 10;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 12,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'P',
                climb_stat: 30,
                mana: 8,
                hp: 12,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'S',
                climb_stat: 30,
                mana: 8,
                hp: 12,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 3,
                class_byte: b'D',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 4,
                class_byte: b'B',
                status: b'A',
                climb_stat: 30,
                mana: 8,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
        ];

        assert_eq!(state.hourly_provision_consumer_count(), 2);
        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.hour, 6);
        assert_eq!(state.food, 8);
        assert_eq!(state.party[0].hp, 12);
        assert_eq!(state.party[1].hp, 11);
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.party[2].hp, 12);
    }

    #[test]
    fn hourly_status_pass_only_spends_food_on_decrement_hours_and_floors() {
        let mut off_hour = world_state(open_world_grid(), 10, 20);
        off_hour.clock = GameClock::with_date(139, 4, 5, 6, 59).unwrap();
        off_hour.food = 1;

        off_hour.advance_turn_with_minutes(1);

        assert_eq!(off_hour.clock.hour, 7);
        assert_eq!(off_hour.food, 1);

        let mut noon = world_state(open_world_grid(), 10, 20);
        noon.clock = GameClock::with_date(139, 4, 5, 11, 59).unwrap();
        noon.food = 1;
        noon.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: 30,
            mana: 8,
            hp: 12,
            max_hp: 20,
            level: 8,
        });

        noon.advance_turn_with_minutes(1);

        assert_eq!(noon.clock.hour, 12);
        assert_eq!(noon.food, 0);
    }

    #[test]
    fn hourly_poison_tick_can_kill_poisoned_living_member() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, 7, 59).unwrap();
        state.food = 99;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'P',
                climb_stat: 30,
                mana: 8,
                hp: 1,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 1,
                max_hp: 20,
                level: 8,
            },
        ];

        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.hour, 8);
        assert_eq!(state.food, 99);
        assert_eq!(state.party[0].hp, 0);
        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[1].hp, 1);
        assert_eq!(state.party[1].status, b'G');
    }

    #[test]
    fn hourly_ring_regeneration_tick_uses_ring_slot_and_living_hp_cap() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 19,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 20,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
        ];
        state.party_equipment = default_party_equipment(3);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        state.party_equipment[1][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        state.party_equipment[2][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;

        state.prng_state = 0x0030;

        assert_eq!(state.apply_hourly_ring_regeneration_tick(), 1);
        assert_eq!(state.party[0].hp, 20);
        assert_eq!(state.party[0].mana, 8);
        assert_eq!(state.party[1].hp, 20);
        assert_eq!(state.party[2].hp, 0);
    }

    #[test]
    fn hourly_ring_regeneration_tick_skips_combat_and_wrong_slot() {
        let mut wrong_slot = world_state(open_world_grid(), 10, 20);
        wrong_slot.party[0].hp = 19;
        wrong_slot.party[0].max_hp = 20;
        wrong_slot.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_ID_RING_REGENERATION as u8;
        wrong_slot.prng_state = 0x0030;

        assert_eq!(wrong_slot.apply_hourly_ring_regeneration_tick(), 0);
        assert_eq!(wrong_slot.party[0].hp, 19);

        let mut combat = wrong_slot.clone();
        combat.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        combat.combat_active = true;

        assert_eq!(combat.apply_hourly_ring_regeneration_tick(), 0);
        assert_eq!(combat.party[0].hp, 19);
    }

    #[test]
    fn hourly_starvation_tick_damages_living_members_when_food_is_zero() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, 8, 59).unwrap();
        state.food = 0;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 2,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'S',
                climb_stat: 30,
                mana: 8,
                hp: 2,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
        ];

        let p0_hp_before = state.party[0].hp;
        let p1_hp_before = state.party[1].hp;
        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.hour, 9);
        assert_eq!(state.food, 0);
        // `cleak/u5-spec#50`: starvation rolls `prng_range(1, 8)` per
        // non-dead slot. With starting HP = 2, every roll either
        // partially damages (HP >= 0) or kills (HP = 0). Assert the
        // spec contract rather than a specific roll.
        assert!(state.party[0].hp <= p0_hp_before);
        // Status stays `'G'` while alive; becomes `'D'` only on
        // exact-zero HP.
        let p0_expected_status = if state.party[0].hp == 0 { b'D' } else { b'G' };
        assert_eq!(state.party[0].status, p0_expected_status);
        assert!(state.party[1].hp <= p1_hp_before);
        let p1_expected_status = if state.party[1].hp == 0 { b'D' } else { b'S' };
        assert_eq!(state.party[1].status, p1_expected_status);
        // Already-dead member must be skipped.
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].status, b'D');
        assert!(state
            .pending_hourly_status_message
            .as_deref()
            .is_some_and(|message| message.contains("Starving!")));
    }

    #[test]
    fn hourly_status_pass_applies_poison_before_seeded_starvation_rolls() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, 8, 59).unwrap();
        state.food = 0;
        state.prng_state = 0x3456;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'P',
                climb_stat: 30,
                mana: 8,
                hp: 20,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'G',
                climb_stat: 30,
                mana: 8,
                hp: 20,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'D',
                climb_stat: 30,
                mana: 8,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
        ];

        let mut expected_prng = state.prng_state;
        let poisoned_starvation =
            u5_prng_range_u16(&mut expected_prng, HOURLY_STARVATION_DAMAGE_MIN, HOURLY_STARVATION_DAMAGE_MAX)
                as u8;
        let good_starvation =
            u5_prng_range_u16(&mut expected_prng, HOURLY_STARVATION_DAMAGE_MIN, HOURLY_STARVATION_DAMAGE_MAX)
                as u8;

        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.hour, 9);
        assert_eq!(
            state.party[0].hp,
            20u16
                - u16::from(FIRST_PLAYABLE_HOURLY_POISON_DAMAGE)
                - u16::from(poisoned_starvation)
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[1].hp, 20 - u16::from(good_starvation));
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.prng_state, expected_prng);
        let report = state.pending_hourly_status_message.as_deref().unwrap();
        assert!(report.contains(&format!("party slot 0 took {poisoned_starvation} HP")));
        assert!(report.contains(&format!("party slot 1 took {good_starvation} HP")));
    }

    #[test]
    fn hourly_status_pass_applies_ring_after_poison_and_provisions() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, 5, 59).unwrap();
        state.food = 10;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'P',
            climb_stat: 30,
            mana: 8,
            hp: 20,
            max_hp: 20,
            level: 8,
        }];
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        state.prng_state = 0x0030;

        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.hour, 6);
        assert_eq!(state.food, 9);
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(
            state.party[0].hp, 20,
            "poison damage should be eligible for the same hourly ring tick"
        );
    }

    #[test]
    fn starvation_warning_appends_after_pass_turn_hour_crossing() {
        // `cleak/u5-spec#50`: starvation rolls `prng_range(1, 8)` per
        // non-dead slot. Verify the message bubbles up regardless of
        // the specific roll value.
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, 8, 58).unwrap();
        state.food = 0;
        state.party[0].hp = 10; // headroom so the roll cannot kill
        let hp_before = state.party[0].hp;

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);

        assert_eq!(state.clock.hour, 9);
        assert!(state.party[0].hp < hp_before);
        assert!((hp_before - state.party[0].hp) as u16 <= HOURLY_STARVATION_DAMAGE_MAX);
        assert!(state.message.contains("Passed."));
        assert!(state.message.contains("Starving!"));
        assert_eq!(state.message.matches("Starving!").count(), 1);
        assert!(state.pending_hourly_status_message.is_none());
    }

    #[test]
    fn validate_start_rejects_blocked_tiles() {
        let mut grid = open_grid();
        grid[3 * 32 + 4] = 0x0c;

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
    fn optional_passability_bitmap_still_controls_base_predicate() {
        let passability = passability_with_tiles(&[0x0c]);

        assert!(is_base_tile_passable(0x0c, Some(&passability)));
        assert!(!is_base_tile_passable(0x0c, None));
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
            fixed_hidden_treasure_single_use_cookie: FIXED_HIDDEN_TREASURE_SINGLE_USE_COOKIE_CLEAR,
            dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            saved_dungeon_working_buffer: None,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            quest_progress_word: DEFAULT_QUEST_PROGRESS_WORD,
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            moral_standing: 0,
            toll_progress: 0,
            natural_moongate_counter: 0,
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
            facing: None,
            pending_vehicle: None,
            inn_registry: Vec::new(),
            blackthorn_story: BlackthornStoryState::default(),
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

    /// `formats/location-dat.md §6`: the load pass "walks every cell of the
    /// freshly-read tile grid in loader order (column 0 north-to-south,
    /// then column 1, and so on)".
    #[test]
    fn location_markers_use_column_major_loader_order() {
        let mut grid = open_grid();
        grid[2 * 32 + 4] = 0x48;
        grid[1] = 0x49;

        let markers = harvest_location_markers(&grid);

        assert_eq!(markers.npc_markers, vec![(1, 0), (4, 2)]);
    }

    /// `formats/location-dat.md §6` withdrew the `0x2A` spawn-marker
    /// reading in full: "it is the night beacon's indoor light source". The
    /// scrub therefore claims only the NPC start-marker pair, and the byte
    /// the beacon harvests survives the runtime tile buffer — which is what
    /// makes `§6`'s "ordering cannot disarm it" hold on our side too.
    #[test]
    fn scrub_location_entry_markers_removes_npc_bytes_and_spares_the_bright_light() {
        let mut grid = open_grid();
        grid[5 * 32] = BEACON_BRIGHT_LIGHT_TILE;
        grid[2 * 32 + 4] = 0x48;
        grid[3 * 32 + 5] = 0x49;
        grid[4 * 32 + 6] = 0xc8;

        scrub_location_entry_markers(&mut grid);

        assert_eq!(
            grid[5 * 32], BEACON_BRIGHT_LIGHT_TILE,
            "the beacon's indoor light source is not a marker to scrub"
        );
        assert_eq!(grid[2 * 32 + 4], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(grid[3 * 32 + 5], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(grid[4 * 32 + 6], 0xc8);
        assert_eq!(
            harvest_location_beacon_sources(&grid),
            [Some((0, 5)), None],
            "and it is still harvestable after the scrub"
        );
        assert!(harvest_location_markers(&grid).npc_markers.is_empty());
    }

    #[test]
    fn load_town_scene_scrubs_npc_markers_and_keeps_the_beacon_source() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut grid = open_grid();
        grid[1 * 32 + 2] = BEACON_BRIGHT_LIGHT_TILE;
        grid[2 * 32 + 4] = 0x48;
        grid[3 * 32 + 5] = 0x49;
        fs::write(dir.join("CASTLE.DAT"), grid).unwrap();

        let state = PlayState::load_town_scene(&dir, scene, PlayOptions::default()).unwrap();

        // The entry cell is no longer taken from a `0x2A` cell. With no
        // caller start and no per-scene entry row, what is left is the
        // first walkable cell — see the KNOWN GAP at the `load_town_scene`
        // fallback.
        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.grid[1 * 32 + 2], BEACON_BRIGHT_LIGHT_TILE);
        assert_eq!(state.grid[2 * 32 + 4], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.grid[3 * 32 + 5], LOCATION_MARKER_CLEANUP_TILE);
        assert_eq!(state.light_beacon.sources, [Some((2, 1)), None]);
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
