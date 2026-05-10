    #[test]
    fn z_stats_reports_runtime_state_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);
        state.food = 123;
        state.gold = 987;
        state.keys = 7;
        state.gems = 3;
        state.torches = 5;
        state.climbing_gear = 1;
        state.torch_counter = 12;
        state.light_spell_counter = 34;
        state.ambient_light = 56;
        state.time_stop_counter = 2;
        state.wind = WindState::East;
        state.timing_status = TimingStatusTag::NoMinuteLight;
        state.spell_charges[IN_LOR_SPELL_INDEX] = 2;
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.reagents = [1, 2, 3, 4, 5, 6, 7, 8];
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: 10,
                mana: 4,
                hp: 10,
                max_hp: 20,
                level: 2,
            },
            PartyMember {
                slot: 2,
                status: b'P',
                climb_stat: 30,
                mana: 5,
                hp: 6,
                max_hp: 30,
                level: 3,
            },
        ];

        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert!(
            state
                .message
                .contains("Z-stats: CASTLE:0 floor 0 at (5, 5)")
        );
        assert!(state.message.contains("facing South"));
        assert!(state.message.contains("date Y139 M4 D5 12:00"));
        assert!(state.message.contains("transport foot"));
        assert!(state.message.contains("East Winds"));
        assert!(state.message.contains("timing no-minute-light"));
        assert!(
            state
                .message
                .contains("light torch=12 spell=34 ambient=56 time-stop=2")
        );
        assert!(state.message.contains(
            "inventory food=123 gold=987 keys=7 gems=3 torches=5 climbing=1 reagents=36"
        ));
        assert!(state.message.contains("spells IL=2, PRV=1"));
        assert!(
            state.message.contains(
                "party P1:slot0 good HP 10/20 MP 4 L2; P2:slot2 poisoned HP 6/30 MP 5 L3"
            )
        );
    }

    #[test]
    fn z_stats_reports_empty_spell_stock() {
        let mut state = test_state(open_grid(), 1, 1);
        state.spell_charges = [0; SPELL_COUNT];

        assert_eq!(state.z_stats(), MoveOutcome::Observed);

        assert_eq!(state.turn, 0);
        assert!(state.message.contains("spells none"));
        assert!(state.message.contains("party P1:slot0 good"));
    }

    #[test]
    fn dungeon_z_stats_reports_level_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);

        assert!(state.handle_dungeon_key('Z', Path::new("")).unwrap());

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert!(
            state
                .message
                .contains("Z-stats: DUNGEON:0 level 3 at (1, 1)")
        );
        assert!(state.message.contains("transport foot"));
        assert!(state.message.contains("spells none"));
    }

    #[test]
    fn new_order_swaps_runtime_party_positions_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                status: b'P',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
            PartyMember {
                slot: 2,
                status: b'S',
                climb_stat: 30,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 3,
            },
        ];

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "13", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state
                .party
                .iter()
                .map(|member| member.slot)
                .collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "New order: party slots 1 and 3 swapped.");
    }

    #[test]
    fn new_order_refusals_preserve_party_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
        ];
        let original = state.party.clone();

        assert_eq!(state.new_order_from_suffix(""), MoveOutcome::PromptDeclined);
        assert_eq!(
            state.message,
            "New order? Use N12 to swap party slots 1 and 2."
        );
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.new_order_from_suffix("11"),
            MoveOutcome::PromptDeclined
        );
        assert_eq!(state.message, "Party slots are already in that order.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);

        assert_eq!(state.new_order_from_suffix("13"), MoveOutcome::Blocked);
        assert_eq!(state.message, "Party has 2 members.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn new_order_changes_caster_position_for_inline_cast() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
        ];
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        state.ambient_light = FULL_DARKNESS;

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "12", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.party[0].slot, 1);
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[0].slot, 1);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[1].slot, 0);
        assert_eq!(state.party[1].mana, 0);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn cast_restore_spells_clear_status_and_heal_hp_after_resource_gates() {
        let mut cure = dungeon_state(open_dungeon_record(), 0, 1, 1);
        cure.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
        ];
        cure.spell_charges[CURE_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut cure, 'C', "1AN2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(cure.party[1].status, b'G');
        assert_eq!(cure.spell_charges[CURE_SPELL_INDEX], 0);
        assert_eq!(cure.party[0].mana, 2);
        assert_eq!(cure.turn, 1);
        assert_eq!(cure.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(cure.message, "Cured party member 2.");

        let mut awaken = dungeon_state(open_dungeon_record(), 0, 1, 1);
        awaken.party = cure.party.clone();
        awaken.party[0].mana = 3;
        awaken.party[1].status = b'S';
        awaken.spell_charges[AWAKEN_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut awaken, 'C', "1AZ2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(awaken.party[1].status, b'G');
        assert_eq!(awaken.spell_charges[AWAKEN_SPELL_INDEX], 0);
        assert_eq!(awaken.party[0].mana, 2);
        assert_eq!(awaken.turn, 1);
        assert_eq!(awaken.message, "Awakened party member 2.");

        let mut heal = dungeon_state(open_dungeon_record(), 0, 1, 1);
        heal.party = cure.party;
        heal.party[0].mana = 3;
        heal.party[1].hp = 8;
        heal.party[1].max_hp = 15;
        heal.spell_charges[HEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut heal, 'C', "1M2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(heal.party[1].hp, 15);
        assert_eq!(heal.spell_charges[HEAL_SPELL_INDEX], 0);
        assert_eq!(heal.party[0].mana, 2);
        assert_eq!(heal.turn, 1);
        assert_eq!(heal.message, "Healed party member 2 for 7 HP (15/15).");

        let mut great_heal = dungeon_state(open_dungeon_record(), 0, 1, 1);
        great_heal.party = heal.party.clone();
        great_heal.party[0].mana = GREAT_HEAL_COST;
        great_heal.party[0].level = GREAT_HEAL_COST;
        great_heal.party[1].hp = 4;
        great_heal.party[1].max_hp = 22;
        great_heal.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut great_heal, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(great_heal.party[1].hp, 22);
        assert_eq!(great_heal.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(great_heal.party[0].mana, 0);
        assert_eq!(great_heal.turn, 1);
        assert_eq!(
            great_heal.message,
            "Great healed party member 2 for 18 HP (22/22)."
        );

        let mut resurrect = dungeon_state(open_dungeon_record(), 0, 1, 1);
        resurrect.party = great_heal.party.clone();
        resurrect.party[0].mana = RESURRECT_COST;
        resurrect.party[0].level = RESURRECT_COST;
        resurrect.party[1].status = b'D';
        resurrect.party[1].hp = 0;
        resurrect.party[1].max_hp = 19;
        resurrect.spell_charges[RESURRECT_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut resurrect, 'C', "1CIM2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(resurrect.party[1].status, b'G');
        assert_eq!(resurrect.party[1].hp, 19);
        assert_eq!(resurrect.spell_charges[RESURRECT_SPELL_INDEX], 0);
        assert_eq!(resurrect.party[0].mana, 0);
        assert_eq!(resurrect.turn, 1);
        assert_eq!(resurrect.message, "Resurrected party member 2 (19/19).");
    }

    #[test]
    fn cast_restore_target_prompt_precedes_resource_consumption() {
        let mut missing_target = dungeon_state(open_dungeon_record(), 0, 1, 1);
        missing_target.spell_charges[CURE_SPELL_INDEX] = 1;
        missing_target.party[0].mana = CURE_COST;
        missing_target.party[0].level = CURE_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_target, 'C', "1AN", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_target.spell_charges[CURE_SPELL_INDEX], 1);
        assert_eq!(missing_target.party[0].mana, CURE_COST);
        assert_eq!(missing_target.turn, 0);
        assert_eq!(
            missing_target.message,
            "Whom? Use C1AN2 to cure party member 2."
        );

        let mut invalid_target = dungeon_state(open_dungeon_record(), 0, 1, 1);
        invalid_target.spell_charges[CURE_SPELL_INDEX] = 1;
        invalid_target.party[0].mana = CURE_COST;
        invalid_target.party[0].level = CURE_COST;

        assert_eq!(
            handle_play_key_input(&mut invalid_target, 'C', "1AN2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(invalid_target.spell_charges[CURE_SPELL_INDEX], 1);
        assert_eq!(invalid_target.party[0].mana, CURE_COST);
        assert_eq!(invalid_target.turn, 0);
        assert_eq!(invalid_target.message, "Party has 1 member.");
    }

    #[test]
    fn cast_restore_non_applicable_target_consumes_cast_and_fails() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party[0].status = b'G';
        state.spell_charges[CURE_SPELL_INDEX] = 1;
        state.party[0].mana = CURE_COST;
        state.party[0].level = CURE_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AN1", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.spell_charges[CURE_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Failed!");

        let mut great_heal_dead = dungeon_state(open_dungeon_record(), 0, 1, 1);
        great_heal_dead.party = vec![
            PartyMember {
                slot: 0,
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: GREAT_HEAL_COST,
                hp: 10,
                max_hp: 20,
                level: GREAT_HEAL_COST,
            },
            PartyMember {
                slot: 1,
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 20,
                level: 1,
            },
        ];
        great_heal_dead.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut great_heal_dead, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(great_heal_dead.party[1].status, b'D');
        assert_eq!(great_heal_dead.party[1].hp, 0);
        assert_eq!(great_heal_dead.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(great_heal_dead.party[0].mana, 0);
        assert_eq!(great_heal_dead.turn, 1);
        assert_eq!(great_heal_dead.message, "Failed!");

        let mut resurrect_living = dungeon_state(open_dungeon_record(), 0, 1, 1);
        resurrect_living.party[0].status = b'G';
        resurrect_living.party[0].hp = 10;
        resurrect_living.spell_charges[RESURRECT_SPELL_INDEX] = 1;
        resurrect_living.party[0].mana = RESURRECT_COST;
        resurrect_living.party[0].level = RESURRECT_COST;

        assert_eq!(
            handle_play_key_input(&mut resurrect_living, 'C', "1CIM1", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(resurrect_living.party[0].status, b'G');
        assert_eq!(resurrect_living.party[0].hp, 10);
        assert_eq!(resurrect_living.spell_charges[RESURRECT_SPELL_INDEX], 0);
        assert_eq!(resurrect_living.party[0].mana, 0);
        assert_eq!(resurrect_living.turn, 1);
        assert_eq!(resurrect_living.message, "Failed!");
    }

    #[test]
    fn cast_vas_lor_sets_great_light_counter() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[VAS_LOR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 3;
        state.ambient_light = FULL_DARKNESS;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1VL", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[VAS_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.light_spell_counter, VAS_LOR_LIGHT_DURATION);
        assert_eq!(state.ambient_light, LIGHT_SPELL_FLOOR);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn cast_an_sanct_safely_opens_dungeon_chest_underfoot() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.spell_charges[OPEN_SPELL_INDEX] = 1;
        state.party[0].mana = OPEN_SPELL_COST;
        state.party[0].level = OPEN_SPELL_COST;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Safely opened dungeon chest at (1, 1) on DUNGEON:0 level 0; trap generator bypassed by An Sanct, marked visit-local passage."
        );
    }

    #[test]
    fn cast_an_sanct_applies_clean_sidecar_chest_grants() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x4b GOLD 12 TORCHES 1\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.spell_charges[OPEN_SPELL_INDEX] = 1;
        state.party[0].mana = OPEN_SPELL_COST;
        state.party[0].level = OPEN_SPELL_COST;
        let starting_gold = state.gold;
        let starting_torches = state.torches;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AS", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.gold, starting_gold + 12);
        assert_eq!(state.torches, starting_torches + 1);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("trap generator bypassed by An Sanct")
        );
        assert!(
            state
                .message
                .contains("authored chest grants 12 gold, 1 torches")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_an_sanct_failures_preserve_public_resource_ordering() {
        let mut no_charge = dungeon_state(open_dungeon_record(), 0, 1, 1);
        no_charge.party[0].mana = OPEN_SPELL_COST;
        no_charge.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut no_charge, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(no_charge.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(no_charge.party[0].mana, OPEN_SPELL_COST);
        assert_eq!(no_charge.turn, 0);
        assert_eq!(no_charge.message, "None mixed!");

        let mut no_modeled_target = test_state(open_grid(), 1, 1);
        no_modeled_target.spell_charges[OPEN_SPELL_INDEX] = 1;
        no_modeled_target.party[0].mana = OPEN_SPELL_COST;
        no_modeled_target.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut no_modeled_target, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(no_modeled_target.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(no_modeled_target.party[0].mana, 0);
        assert_eq!(no_modeled_target.turn, 1);
        assert_eq!(no_modeled_target.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(no_modeled_target.message, "Failed!");

        let mut wrong_underfoot = dungeon_state(open_dungeon_record(), 0, 1, 1);
        wrong_underfoot.spell_charges[OPEN_SPELL_INDEX] = 1;
        wrong_underfoot.party[0].mana = OPEN_SPELL_COST;
        wrong_underfoot.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut wrong_underfoot, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(wrong_underfoot.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(wrong_underfoot.party[0].mana, 0);
        assert_eq!(wrong_underfoot.turn, 1);
        assert_eq!(wrong_underfoot.message, "Failed!");
    }

    #[test]
    fn cast_blink_uses_clean_sidecar_target_and_leaves_intervening_lock() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BLINK_TARGET_TABLE_FILE),
            "CASTLE:0 0 1 1 E 3 1 16 16\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 97;
        let mut state = test_state(grid, 1, 1);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (3, 1));
        assert_eq!(state.grid[32 + 2], 97);
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Blinked East to (3, 1) in CASTLE:0 floor 0.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_blink_rejects_foot_damaging_sidecar_destination() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BLINK_TARGET_TABLE_FILE),
            "BRITANNIA 0 1 1 E 2 1 5 5\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 2 1 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Failed!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_blink_rejections_preserve_public_resource_ordering() {
        let dir = debug_game_dir();
        let mut missing_direction = test_state(open_grid(), 1, 1);
        missing_direction.spell_charges[BLINK_SPELL_INDEX] = 1;
        missing_direction.party[0].mana = BLINK_COST;
        missing_direction.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_direction, 'C', "1IP", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_direction.spell_charges[BLINK_SPELL_INDEX], 1);
        assert_eq!(missing_direction.party[0].mana, BLINK_COST);
        assert_eq!(missing_direction.turn, 0);
        assert_eq!(missing_direction.message, "Direction? Use C1IP6.");

        let mut missing_row = test_state(open_grid(), 1, 1);
        missing_row.spell_charges[BLINK_SPELL_INDEX] = 1;
        missing_row.party[0].mana = BLINK_COST;
        missing_row.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_row, 'C', "1IP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_row.spell_charges[BLINK_SPELL_INDEX], 1);
        assert_eq!(missing_row.party[0].mana, BLINK_COST);
        assert_eq!(missing_row.turn, 0);
        assert_eq!(missing_row.message, "No Blink target.");

        fs::write(
            dir.join(BLINK_TARGET_TABLE_FILE),
            "CASTLE:0 0 1 1 E 2 1 16 16\n",
        )
        .unwrap();
        let mut blocked_destination = test_state(open_grid(), 1, 1);
        blocked_destination.spell_charges[BLINK_SPELL_INDEX] = 1;
        blocked_destination.party[0].mana = BLINK_COST;
        blocked_destination.party[0].level = BLINK_COST;
        blocked_destination.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut blocked_destination, 'C', "1IP6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            (blocked_destination.player.x, blocked_destination.player.y),
            (1, 1)
        );
        assert_eq!(blocked_destination.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(blocked_destination.party[0].mana, 0);
        assert_eq!(blocked_destination.turn, 1);
        assert_eq!(blocked_destination.message, "Failed!");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_uus_por_moves_up_one_dungeon_level_without_ladder() {
        let scene = DungeonScene::new(33).unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
        state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1UP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 2 });
        assert_eq!(state.active_objects[0].z, 2);
        assert_eq!(state.spell_charges[UUS_POR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Up! Changed to DUNGEON:0 (Deceit) level 2.");
    }

    #[test]
    fn cast_des_por_moves_down_one_dungeon_level_without_ladder() {
        let scene = DungeonScene::new(33).unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
        state.spell_charges[DES_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1DP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 4 });
        assert_eq!(state.active_objects[0].z, 4);
        assert_eq!(state.spell_charges[DES_POR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.message,
            "Down! Changed to DUNGEON:0 (Deceit) level 4."
        );
    }

    #[test]
    fn cast_dungeon_field_spells_place_public_field_bytes() {
        for (suffix, spell_index, cost, starting_tile, expected_tile, expected_message) in [
            (
                "1FGI6",
                FIRE_FIELD_SPELL_INDEX,
                FIELD_SPELL_COST,
                0x00,
                0x82,
                "Fire field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
            (
                "1GIN6",
                POISON_FIELD_SPELL_INDEX,
                FIELD_SPELL_COST,
                0x00,
                0x81,
                "Poison field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
            (
                "1GIZ6",
                SLEEP_FIELD_SPELL_INDEX,
                FIELD_SPELL_COST,
                0x08,
                0x88,
                "Sleep field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
            (
                "1GIS6",
                ENERGY_FIELD_SPELL_INDEX,
                ENERGY_FIELD_COST,
                0x08,
                0x8b,
                "Energy field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
        ] {
            let mut grid = open_dungeon_record();
            grid[dungeon_cell_index(0, 2, 1)] = starting_tile;
            let mut state = dungeon_state(grid, 0, 1, 1);
            state.spell_charges[spell_index] = 1;
            state.party[0].mana = cost;
            state.party[0].level = cost;
            state.visibility_dirty = false;

            assert_eq!(
                handle_play_key_input(&mut state, 'C', suffix, Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );

            assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], expected_tile);
            assert_eq!(state.spell_charges[spell_index], 0);
            assert_eq!(state.party[0].mana, 0);
            assert_eq!(state.turn, 1);
            assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
            assert!(state.visibility_dirty);
            assert_eq!(state.message, expected_message);
        }
    }

