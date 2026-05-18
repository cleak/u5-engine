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
    fn wind_save_byte_maps_public_persistent_values() {
        assert_eq!(WindState::from_save_byte(0), WindState::Calm);
        assert_eq!(WindState::from_save_byte(1), WindState::North);
        assert_eq!(WindState::from_save_byte(2), WindState::South);
        assert_eq!(WindState::from_save_byte(3), WindState::East);
        assert_eq!(WindState::from_save_byte(4), WindState::West);
        assert_eq!(WindState::from_save_byte(0x7a), WindState::Calm);

        assert_eq!(WindState::Calm.save_byte(), 0);
        assert_eq!(WindState::North.save_byte(), 1);
        assert_eq!(WindState::South.save_byte(), 2);
        assert_eq!(WindState::East.save_byte(), 3);
        assert_eq!(WindState::West.save_byte(), 4);
    }

    #[test]
    fn prng_state_advance_matches_public_formula_sequence() {
        let mut state = 0;

        state = u5_prng_advance_state(state);
        assert_eq!(state, 0x8012);
        state = u5_prng_advance_state(state);
        assert_eq!(state, 0xd014);
        state = u5_prng_advance_state(state);
        assert_eq!(state, 0x1e14);
        state = u5_prng_advance_state(state);
        assert_eq!(state, 0x0454);
        state = u5_prng_advance_state(state);
        assert_eq!(state, 0x00ac);
    }

    #[test]
    fn prng_range_consumes_state_and_applies_inclusive_modulo() {
        let mut prng = U5Prng::new(0);

        assert_eq!(prng.next_range_u16(1, 30), 19);
        assert_eq!(prng.state(), 0x8012);
        assert_eq!(prng.next_range_u16(1, 30), 11);
        assert_eq!(prng.state(), 0xd014);
        assert_eq!(prng.next_range_u8(10, 15), 12);
        assert_eq!(prng.state(), 0x1e14);
    }

    #[test]
    fn prng_range_helper_updates_external_state_word() {
        let mut state = 0x1234;

        assert_eq!(u5_prng_range_u16(&mut state, 4, 7), 4);
        assert_eq!(state, 0x06d8);
    }

    #[test]
    fn prng_zero_width_range_panics_after_state_advance() {
        let mut prng = U5Prng::new(0);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            prng.next_range_u16(0, 0xffff);
        }));

        assert!(result.is_err());
        assert_eq!(prng.state(), 0x8012);
    }

    #[test]
    fn rel_hur_wind_change_uses_prompt_direction_mapping() {
        assert_eq!(
            WindState::rel_hur_target(Direction::North),
            Some(WindState::West)
        );
        assert_eq!(
            WindState::rel_hur_target(Direction::East),
            Some(WindState::East)
        );
        assert_eq!(
            WindState::rel_hur_target(Direction::South),
            Some(WindState::South)
        );
        assert_eq!(
            WindState::rel_hur_target(Direction::West),
            Some(WindState::North)
        );
        assert_eq!(WindState::rel_hur_target(Direction::NorthEast), None);
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
    fn chargen_pair_records_follow_public_question_table() {
        assert_eq!(
            chargen_question_record_for_pair(ShrineVirtue::Honesty, ShrineVirtue::Compassion)
                .unwrap(),
            2
        );
        assert_eq!(
            chargen_question_record_for_pair(ShrineVirtue::Humility, ShrineVirtue::Honesty)
                .unwrap(),
            8
        );
        assert_eq!(
            chargen_question_record_for_pair(ShrineVirtue::Justice, ShrineVirtue::Sacrifice)
                .unwrap(),
            20
        );
        assert_eq!(
            chargen_question_record_for_pair(ShrineVirtue::Spirituality, ShrineVirtue::Humility)
                .unwrap(),
            29
        );
        assert_eq!(
            chargen_question_record_for_pair(ShrineVirtue::Valor, ShrineVirtue::Valor),
            Err(ChargenError::SameVirtuePair)
        );
    }

    #[test]
    fn chargen_winner_stats_apply_virtue_deltas_and_strength_floor() {
        let stats = chargen_stats_from_winners(&[
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
        ]);

        assert_eq!(
            stats,
            ChargenStats {
                strength: 20,
                dexterity: 5,
                intelligence: 5,
            }
        );
    }

    #[test]
    fn chargen_application_customizes_only_avatar_identity_and_stats() {
        let mut save = saved_game_seed_bytes(0, 0, 10, 20);
        let record = SAVE_ROSTER_OFFSET;
        save[record + SAVE_CHARACTER_NAME_LEN - 1] = 0x7f;
        save[record + SAVE_CHARACTER_CLASS_OFFSET] = b'A';
        save[record + SAVE_CHARACTER_STATUS_OFFSET] = b'G';
        write_u16_at(&mut save, record + SAVE_CHARACTER_HP_OFFSET, 60);
        write_u16_at(&mut save, record + SAVE_CHARACTER_EXPERIENCE_OFFSET, 150);
        save[record + SAVE_CHARACTER_LEVEL_OFFSET] = 2;
        let equipment_offset = record + SAVE_CHARACTER_EQUIPMENT_OFFSET;
        save[equipment_offset] = 0x42;
        let stats = ChargenStats {
            strength: 20,
            dexterity: 7,
            intelligence: 9,
        };

        let avatar = apply_chargen_to_save(&mut save, b"DUPRE", false, stats).unwrap();

        assert_eq!(&save[record..record + SAVE_CHARACTER_NAME_LEN - 1], b"DUPRE\0\0\0");
        assert_eq!(save[record + SAVE_CHARACTER_NAME_LEN - 1], 0x7f);
        assert_eq!(save[record + SAVE_CHARACTER_GENDER_OFFSET], SAVE_GENDER_FEMALE_BYTE);
        assert_eq!(save[record + SAVE_CHARACTER_CLASS_OFFSET], b'A');
        assert_eq!(save[record + SAVE_CHARACTER_STATUS_OFFSET], b'G');
        assert_eq!(save[record + SAVE_CHARACTER_STR_OFFSET], 20);
        assert_eq!(save[record + SAVE_CHARACTER_DEX_OFFSET], 7);
        assert_eq!(save[record + SAVE_CHARACTER_INT_OFFSET], 9);
        assert_eq!(save[record + SAVE_CHARACTER_MANA_OFFSET], 9);
        assert_eq!(u16_at(&save, record + SAVE_CHARACTER_HP_OFFSET), 60);
        assert_eq!(u16_at(&save, record + SAVE_CHARACTER_EXPERIENCE_OFFSET), 150);
        assert_eq!(save[record + SAVE_CHARACTER_LEVEL_OFFSET], 2);
        assert_eq!(save[equipment_offset], 0x42);
        assert_eq!(
            avatar,
            ChargenAvatar {
                name: [b'D', b'U', b'P', b'R', b'E', 0, 0, 0, 0x7f],
                male: false,
                stats,
            }
        );
    }

    #[test]
    fn chargen_commit_writes_saved_files_from_init_seeds() {
        let dir = debug_game_dir();
        let mut init_gam = saved_game_seed_bytes(13, 0, 15, 15);
        init_gam[SAVE_TORCH_STOCK_OFFSET] = 4;
        init_gam[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_CLASS_OFFSET] = b'A';
        fs::write(dir.join("INIT.GAM"), &init_gam).unwrap();
        let init_ool: Vec<u8> = (0..OOL_PLANE_LEN).map(|index| 255 - index as u8).collect();
        fs::write(dir.join("INIT.OOL"), &init_ool).unwrap();
        fs::write(dir.join("SAVED.GAM"), vec![0xcc; SAVED_GAM_LEN]).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0xdd; SAVED_OOL_LEN]).unwrap();
        let stats = ChargenStats {
            strength: 20,
            dexterity: 6,
            intelligence: 8,
        };

        let avatar = commit_chargen_save(&dir, b"AVATAR", true, stats).unwrap();

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
        assert_eq!(
            &saved[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + SAVE_CHARACTER_NAME_LEN],
            b"AVATAR\0\0\0"
        );
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_GENDER_OFFSET],
            SAVE_GENDER_MALE_BYTE
        );
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_CLASS_OFFSET], b'A');
        assert_eq!(saved[SAVE_TORCH_STOCK_OFFSET], 4);
        assert_eq!(&saved_ool[..OOL_PLANE_LEN], vec![0; OOL_PLANE_LEN]);
        assert_eq!(&saved_ool[OOL_PLANE_LEN..], init_ool.as_slice());
        assert_eq!(avatar.male, true);
        assert_eq!(avatar.stats, stats);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn chargen_blank_name_does_not_rewrite_saved_files() {
        let dir = debug_game_dir();
        fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(13, 0, 15, 15)).unwrap();
        fs::write(dir.join("INIT.OOL"), vec![0x11; OOL_PLANE_LEN]).unwrap();
        let old_saved = vec![0x22; SAVED_GAM_LEN];
        let old_saved_ool = vec![0x33; SAVED_OOL_LEN];
        fs::write(dir.join("SAVED.GAM"), &old_saved).unwrap();
        fs::write(dir.join("SAVED.OOL"), &old_saved_ool).unwrap();

        let err = commit_chargen_save(
            &dir,
            b"",
            true,
            ChargenStats {
                strength: 20,
                dexterity: 0,
                intelligence: 0,
            },
        )
        .err()
        .unwrap();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read(dir.join("SAVED.GAM")).unwrap(), old_saved);
        assert_eq!(fs::read(dir.join("SAVED.OOL")).unwrap(), old_saved_ool);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn u4_transfer_parses_party_sav_player_zero_and_virtue_gate() {
        let mut bytes = vec![0; U4_PARTY_SAV_REQUIRED_LEN];
        let record = U4_PARTY_SAV_PLAYER0_OFFSET;
        bytes[record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET..record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET + 16]
            .copy_from_slice(b"AVATAR\0\0\0\0\0\0\0\0\0\0");
        bytes[record + U4_PARTY_SAV_CHARACTER_SEX_OFFSET] = U4_PARTY_SAV_MALE_BYTE;
        bytes[record + U4_PARTY_SAV_CHARACTER_CLASS_OFFSET] = 6;
        bytes[record + U4_PARTY_SAV_CHARACTER_XP_OFFSET..record + U4_PARTY_SAV_CHARACTER_XP_OFFSET + 2]
            .copy_from_slice(&4321u16.to_le_bytes());
        bytes[record + U4_PARTY_SAV_CHARACTER_STR_OFFSET..record + U4_PARTY_SAV_CHARACTER_STR_OFFSET + 2]
            .copy_from_slice(&29u16.to_le_bytes());
        bytes[record + U4_PARTY_SAV_CHARACTER_DEX_OFFSET..record + U4_PARTY_SAV_CHARACTER_DEX_OFFSET + 2]
            .copy_from_slice(&30u16.to_le_bytes());
        bytes[record + U4_PARTY_SAV_CHARACTER_INT_OFFSET..record + U4_PARTY_SAV_CHARACTER_INT_OFFSET + 2]
            .copy_from_slice(&9u16.to_le_bytes());
        bytes[U4_PARTY_SAV_FOOD_OFFSET..U4_PARTY_SAV_FOOD_OFFSET + 4]
            .copy_from_slice(&9999u32.to_le_bytes());
        bytes[U4_PARTY_SAV_GOLD_OFFSET..U4_PARTY_SAV_GOLD_OFFSET + 2]
            .copy_from_slice(&9999u16.to_le_bytes());
        bytes[U4_PARTY_SAV_GEMS_OFFSET..U4_PARTY_SAV_GEMS_OFFSET + 2]
            .copy_from_slice(&12u16.to_le_bytes());
        bytes[U4_PARTY_SAV_KARMA_OFFSET + 4..U4_PARTY_SAV_KARMA_OFFSET + 6]
            .copy_from_slice(&1u16.to_le_bytes());

        let source = parse_u4_transfer_source_from_party_sav(&bytes).unwrap();

        assert_eq!(source.name, b"AVATAR\0\0\0\0\0\0\0\0\0\0");
        assert!(source.male);
        assert_eq!(source.class_index, 6);
        assert_eq!(source.strength, 29);
        assert_eq!(source.dexterity, 30);
        assert_eq!(source.intelligence, 9);
        assert_eq!(source.experience, 4321);
    }

    #[test]
    fn u4_transfer_party_sav_parser_rejects_empty_virtues_and_bad_fields() {
        let mut bytes = vec![0; U4_PARTY_SAV_REQUIRED_LEN];
        let record = U4_PARTY_SAV_PLAYER0_OFFSET;
        bytes[record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET..record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET + 16]
            .copy_from_slice(b"AVATAR\0\0\0\0\0\0\0\0\0\0");
        bytes[record + U4_PARTY_SAV_CHARACTER_CLASS_OFFSET] = 0;

        bytes[U4_PARTY_SAV_GOLD_OFFSET..U4_PARTY_SAV_GOLD_OFFSET + 2]
            .copy_from_slice(&10000u16.to_le_bytes());
        assert_eq!(
            parse_u4_transfer_source_from_party_sav(&bytes).err(),
            Some(U4TransferError::SourceCounterOutOfRange {
                field: "gold",
                value: 10000,
                max: 9999,
            })
        );
        bytes[U4_PARTY_SAV_GOLD_OFFSET..U4_PARTY_SAV_GOLD_OFFSET + 2]
            .copy_from_slice(&0u16.to_le_bytes());

        assert_eq!(
            parse_u4_transfer_source_from_party_sav(&bytes).err(),
            Some(U4TransferError::NoTransferableData)
        );

        bytes[U4_PARTY_SAV_KARMA_OFFSET..U4_PARTY_SAV_KARMA_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        bytes[record + U4_PARTY_SAV_CHARACTER_CLASS_OFFSET] = 8;
        assert_eq!(
            parse_u4_transfer_source_from_party_sav(&bytes).err(),
            Some(U4TransferError::InvalidClassIndex(8))
        );

        bytes[record + U4_PARTY_SAV_CHARACTER_CLASS_OFFSET] = 0;
        bytes[record + U4_PARTY_SAV_CHARACTER_NAME_OFFSET] = 0x1f;
        assert_eq!(
            parse_u4_transfer_source_from_party_sav(&bytes).err(),
            Some(U4TransferError::InvalidNameByte(0x1f))
        );
    }

    #[test]
    fn u4_transfer_application_maps_avatar_fields_and_preserves_seed_bytes() {
        let mut save = saved_game_seed_bytes(0, 0, 10, 20);
        let equipment_offset = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_EQUIPMENT_OFFSET;
        save[equipment_offset] = 0x7d;
        save[SAVE_TORCH_STOCK_OFFSET] = 6;
        let source = U4TransferSource {
            name: b"IOLO12345".to_vec(),
            male: true,
            class_index: 1,
            strength: 8,
            dexterity: 29,
            intelligence: 30,
            experience: 3500,
        };

        let avatar = apply_u4_transfer_to_save(&mut save, &source, None).unwrap();

        let record = SAVE_ROSTER_OFFSET;
        assert_eq!(&save[record..record + SAVE_CHARACTER_NAME_LEN], b"IOLO1234\0");
        assert_eq!(save[record + SAVE_CHARACTER_GENDER_OFFSET], U5_TRANSFER_MALE_BYTE);
        assert_eq!(save[record + SAVE_CHARACTER_CLASS_OFFSET], b'B');
        assert_eq!(save[record + SAVE_CHARACTER_STATUS_OFFSET], b'G');
        assert_eq!(save[record + SAVE_CHARACTER_STR_OFFSET], 20);
        assert_eq!(save[record + SAVE_CHARACTER_DEX_OFFSET], 20);
        assert_eq!(save[record + SAVE_CHARACTER_INT_OFFSET], 20);
        assert_eq!(save[record + SAVE_CHARACTER_MANA_OFFSET], 20);
        assert_eq!(u16_at(&save, record + SAVE_CHARACTER_HP_OFFSET), 90);
        assert_eq!(u16_at(&save, record + SAVE_CHARACTER_MAX_HP_OFFSET), 90);
        assert_eq!(u16_at(&save, record + SAVE_CHARACTER_EXPERIENCE_OFFSET), 350);
        assert_eq!(save[record + SAVE_CHARACTER_LEVEL_OFFSET], 3);
        assert_eq!(save[equipment_offset], 0x7d);
        assert_eq!(save[SAVE_TORCH_STOCK_OFFSET], 6);
        assert_eq!(
            avatar,
            U4TransferAvatar {
                name: *b"IOLO1234\0",
                male: true,
                class_byte: b'B',
                strength: 20,
                dexterity: 20,
                intelligence: 20,
                experience: 350,
                level: 3,
                hp: 90,
            }
        );
    }

    #[test]
    fn u4_transfer_commit_writes_saved_game_from_brit_seed_and_saved_ool_from_brit_ool() {
        let dir = debug_game_dir();
        let mut brit_gam = saved_game_seed_bytes(0, 0, 22, 33);
        brit_gam[SAVE_TORCH_STOCK_OFFSET] = 5;
        fs::write(dir.join("BRIT.GAM"), &brit_gam).unwrap();
        let brit_ool: Vec<u8> = (0..OOL_PLANE_LEN).map(|index| index as u8).collect();
        fs::write(dir.join("BRIT.OOL"), &brit_ool).unwrap();
        let under_ool = vec![0x44; OOL_PLANE_LEN];
        fs::write(dir.join("UNDER.OOL"), &under_ool).unwrap();
        fs::write(dir.join("SAVED.GAM"), vec![0xcc; SAVED_GAM_LEN]).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0xdd; SAVED_OOL_LEN]).unwrap();
        let source = U4TransferSource {
            name: b"MARIA".to_vec(),
            male: false,
            class_index: 0,
            strength: 30,
            dexterity: 10,
            intelligence: 11,
            experience: 990,
        };

        let avatar = commit_u4_transfer_save(&dir, &source, None).unwrap();

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
        assert_eq!(&saved[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + SAVE_CHARACTER_NAME_LEN], b"MARIA\0\0\0\0");
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_GENDER_OFFSET],
            U5_TRANSFER_FEMALE_BYTE
        );
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_CLASS_OFFSET], b'M');
        assert_eq!(saved[SAVE_TORCH_STOCK_OFFSET], 5);
        assert_eq!(&saved_ool[..OOL_PLANE_LEN], vec![0; OOL_PLANE_LEN]);
        assert_eq!(&saved_ool[OOL_PLANE_LEN..], brit_ool.as_slice());
        assert_eq!(fs::read(dir.join("BRIT.OOL")).unwrap(), brit_ool);
        assert_eq!(fs::read(dir.join("UNDER.OOL")).unwrap(), under_ool);
        assert_eq!(avatar.class_byte, b'M');
        assert_eq!(avatar.male, false);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn u4_transfer_invalid_source_does_not_rewrite_saved_files() {
        let dir = debug_game_dir();
        fs::write(dir.join("BRIT.GAM"), saved_game_seed_bytes(0, 0, 10, 20)).unwrap();
        fs::write(dir.join("BRIT.OOL"), vec![0x11; OOL_PLANE_LEN]).unwrap();
        let old_saved = vec![0x22; SAVED_GAM_LEN];
        let old_saved_ool = vec![0x33; SAVED_OOL_LEN];
        fs::write(dir.join("SAVED.GAM"), &old_saved).unwrap();
        fs::write(dir.join("SAVED.OOL"), &old_saved_ool).unwrap();
        let source = U4TransferSource {
            name: b"AVATAR".to_vec(),
            male: true,
            class_index: 8,
            strength: 20,
            dexterity: 20,
            intelligence: 20,
            experience: 0,
        };

        let err = commit_u4_transfer_save(&dir, &source, None).err().unwrap();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read(dir.join("SAVED.GAM")).unwrap(), old_saved);
        assert_eq!(fs::read(dir.join("SAVED.OOL")).unwrap(), old_saved_ool);
        let _ = fs::remove_dir_all(dir);
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

