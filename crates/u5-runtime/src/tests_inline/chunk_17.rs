    #[test]
    fn z_stats_opens_browser_stats_page_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'B',
                status: b'G',
                climb_stat: 11,
                mana: 4,
                hp: 10,
                max_hp: 20,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'P',
                climb_stat: 13,
                mana: 5,
                hp: 6,
                max_hp: 30,
                level: 3,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"MARIA\0\0\0\0"];
        state.party_strengths = vec![12, 14];
        state.party_intelligence = vec![16, 18];
        state.party_experience = vec![1234, 5678];

        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Stats)
        );
        assert!(state.message.contains("Z-stats: Stats page"));
        assert!(state.message.contains("party member 1 of 2"));
        assert!(state.message.contains("Name: AVATAR"));
        assert!(state.message.contains("Class: Bard"));
        assert!(state.message.contains("Status: good"));
        assert!(state.message.contains("STR 12 DEX 11 INT 16"));
        assert!(state.message.contains("HP 10/20 MP 4 XP 1234"));
    }

    #[test]
    fn z_stats_equipment_page_skips_empty_slots_and_falls_back() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 11,
                max_hp: 21,
                level: 1,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_equipment = default_party_equipment(2);
        state.party_equipment[0][EQUIP_SLOT_HELM] = 1;
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_ID_BOW as u8;

        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));

        assert_eq!(state.turn, 0);
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Equipment)
        );
        assert!(state.message.contains("helm: "));
        assert!(state.message.contains(equipment_name(EQUIPMENT_ID_BOW)));
        assert!(!state.message.contains("offhand:"));

        assert!(state.step_active_z_stats('2', ""));
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Equipment))
        );
        assert!(state.message.contains("Nothing equipped."));
    }

    #[test]
    fn z_stats_inventory_pages_skip_zero_rows() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [3, 0, 0, 0, 0, 0, 0, 0];
        state.spell_charges = [0; SPELL_COUNT];
        state.spell_charges[IN_LOR_SPELL_INDEX] = 2;
        state.keys = 0;
        state.gems = 2;
        state.torches = 0;
        state.climbing_gear = 1;
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
        state.scroll_stock[SCROLL_LIGHT_INDEX] = 1;
        state.potion_stock[POTION_BLUE_INDEX] = 4;
        state.equipment_stock[EQUIPMENT_ID_BOW] = 2;

        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));
        assert!(state.step_active_z_stats('>', ""));

        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Reagents)
        );
        assert!(state.message.contains("Sulfur Ash: 3"));
        assert!(!state.message.contains("Ginseng:"));

        assert!(state.step_active_z_stats('>', ""));
        assert!(state.message.contains("IL Light: 2"));
        assert!(!state.message.contains("GP Magic Missile"));

        assert!(state.step_active_z_stats('>', ""));
        assert!(state.message.contains("Gems: 2"));
        assert!(state.message.contains("Grapple: 1"));
        assert!(state.message.contains("Sextant: 1"));
        assert!(state.message.contains("Scroll LV: 1"));
        assert!(state.message.contains("Blue Potion: 4"));
        assert!(!state.message.contains("Keys:"));

        assert!(state.step_active_z_stats('>', ""));
        assert!(state.message.contains(&format!(
            "{}: 2",
            equipment_name(EQUIPMENT_ID_BOW)
        )));
        assert!(!state.message.contains(equipment_name(EQUIPMENT_ID_CROSSBOW)));
    }

    #[test]
    fn z_stats_navigation_rejects_out_of_range_party_and_exits() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 11,
            max_hp: 21,
            level: 1,
        });
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_equipment = default_party_equipment(2);

        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));
        assert!(state.step_active_z_stats('2', ""));
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Equipment))
        );

        assert!(state.step_active_z_stats('6', ""));
        assert!(state.message.contains("Party has 2 members."));
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Equipment))
        );

        assert!(state.step_active_z_stats(' ', ""));
        assert!(state.active_z_stats.is_none());
        assert_eq!(state.message, "Z-stats closed.");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn combat_z_stats_binds_pending_party_actor() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'M',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 2,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"MARIA\0\0\0\0"];
        state.combat_active = true;
        state.pending_combat_actor_slot = Some(1);

        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.turn, 0);
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Stats))
        );
        assert!(state.message.contains("Name: MARIA"));
    }

    #[test]
    fn shadowlord_midnight_reroll_skips_vanquished_and_keeps_living_distinct() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.clock = GameClock::with_date(139, 4, 5, 23, 59).unwrap();
        state.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, 2];

        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.day, 6);
        assert_eq!(
            state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX],
            SHADOWLORD_VANQUISHED
        );
        assert!(PlayState::shadowlord_slot_is_living(
            state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX]
        ));
        assert!(PlayState::shadowlord_slot_is_living(
            state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX]
        ));
        assert_ne!(
            state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX],
            state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX]
        );
    }

    #[test]
    fn shadowlord_reroll_rejects_current_hideout_id() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.shadowlord_hideouts = [1, 2, SHADOWLORD_VANQUISHED];

        assert_eq!(state.reroll_shadowlord_hideouts_excluding(Some(3)), 2);

        assert_ne!(state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX], 3);
        assert_ne!(state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX], 3);
        assert_ne!(
            state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX],
            state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX]
        );
        assert_eq!(
            state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX],
            SHADOWLORD_VANQUISHED
        );
    }

    #[test]
    fn shadowlord_current_hideout_id_uses_virtue_town_scene_ids_only() {
        let mut state = test_state(open_grid(), 1, 1);

        state.area = Area::Town {
            scene: Scene::new(1).unwrap(),
            floor: 0,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), Some(1));

        state.area = Area::Town {
            scene: Scene::new(8).unwrap(),
            floor: 0,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), Some(8));

        state.area = Area::Town {
            scene: Scene::new(9).unwrap(),
            floor: 0,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), None);

        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), None);
    }

    #[test]
    fn shadowlord_midnight_reroll_excludes_current_virtue_town() {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(1).unwrap(),
            floor: 0,
        };
        state.clock = GameClock::with_date(139, 4, 5, 23, 59).unwrap();
        state.shadowlord_hideouts = [1, 2, SHADOWLORD_VANQUISHED];

        state.advance_turn_with_minutes(1);

        assert_ne!(state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX], 1);
        assert_ne!(state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX], 1);
        assert_ne!(
            state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX],
            state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX]
        );
        assert_eq!(
            state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX],
            SHADOWLORD_VANQUISHED
        );
    }

    #[test]
    fn new_order_swaps_nonleader_party_positions_and_consumes_turn() {
        // commands.md §6: a successful nonzero-slot swap exchanges the whole
        // roster records and marks the turn as consumed.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'P',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'S',
                climb_stat: 30,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 3,
            },
        ];

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "23", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state
                .party
                .iter()
                .map(|member| member.slot)
                .collect::<Vec<_>>(),
            vec![0, 2, 1]
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "New order: party slots 2 and 3 swapped.");
    }

    #[test]
    fn new_order_refuses_swaps_involving_leader_slot_one() {
        // commands.md §6: if either selected slot is slot zero (one-based
        // slot 1), the command refuses without consuming a turn — the leader
        // must remain first.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 3,
            },
        ];
        let original = state.party.clone();

        assert_eq!(state.new_order_from_suffix("12"), MoveOutcome::Blocked);
        assert_eq!(state.message, "The leader must remain first.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);

        assert_eq!(state.new_order_from_suffix("31"), MoveOutcome::Blocked);
        assert_eq!(state.message, "The leader must remain first.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn new_order_same_nonzero_slot_consumes_turn_as_noop() {
        // commands.md §6: picking the same nonzero slot twice is accepted as
        // a behavioural no-op, but the turn is still consumed.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
        ];
        let original = state.party.clone();

        assert_eq!(state.new_order_from_suffix("22"), MoveOutcome::Used);
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "New order: party slot 2 unchanged.");
    }

    #[test]
    fn new_order_refusals_preserve_party_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
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

        assert_eq!(state.new_order_from_suffix("11"), MoveOutcome::Blocked);
        assert_eq!(state.message, "The leader must remain first.");
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
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
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

        // commands.md §6: swapping nonleader slots 2 and 3 consumes one turn.
        assert_eq!(
            handle_play_key_input(&mut state, 'N', "23", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.party[1].slot, 2);
        assert_eq!(state.turn, 1);

        // Casting from one-based slot 2 should now spend the mana that was
        // originally on slot 2 but moved to runtime position 1.
        assert_eq!(
            handle_play_key_input(&mut state, 'C', "2LI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[1].slot, 2);
        assert_eq!(state.party[1].mana, 0);
        assert_eq!(state.party[2].slot, 1);
        assert_eq!(state.party[2].mana, 0);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn new_order_swaps_strength_and_equipment_sidecars() {
        // commands.md §6: New Order swaps the whole roster records (names,
        // stats, equipment, counters) and consumes a turn. Use slots 2 and 3
        // because slot 0 is the leader, which the spec refuses to move.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 5,
                mana: 0,
                hp: 9,
                max_hp: 19,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"DUPRE\0\0\0\0"];
        state.party_strengths = vec![10, 12, 34];
        state.party_equipment = vec![
            [
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
            ],
            [
                1,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
            ],
            [
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                16,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
            ],
        ];

        assert_eq!(state.new_order_from_suffix("23"), MoveOutcome::Used);

        assert_eq!(
            state.party_names,
            vec![*b"AVATAR\0\0\0", *b"DUPRE\0\0\0\0", *b"IOLO\0\0\0\0\0"]
        );
        assert_eq!(state.party_strengths, vec![10, 34, 12]);
        assert_eq!(state.party_equipment[1][EQUIP_SLOT_WEAPON], 16);
        assert_eq!(state.party_equipment[2][EQUIP_SLOT_HELM], 1);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn ready_equipment_equips_and_unequips_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![50];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[EQUIPMENT_ID_ARROWS] = 5;
        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'R', "1/26", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.party_equipment[0][EQUIP_SLOT_WEAPON],
            EQUIPMENT_ID_BOW as u8
        );
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 0);
        assert_eq!(state.turn, 0);

        assert_eq!(state.ready_equipment_from_suffix("1/26"), MoveOutcome::Used);

        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 1);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn ready_equipment_unequipping_invisibility_ring_clears_combat_hidden_flag() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_INVISIBILITY as u8;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            4,
        ]);
        state.active_objects[0].type_byte = 0x5c;
        state.active_objects[0].tile = 0x5d;
        assert!(apply_combat_linked_invisibility(
            &mut state.combat_actors[0],
            &mut state.active_objects,
        )
        .unwrap()
        .changed());
        state.visibility_dirty = false;

        assert_eq!(
            state.ready_equipment_from_suffix("1/42"),
            MoveOutcome::Used
        );

        assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_RING_INVISIBILITY], 1);
        assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.active_objects[0].tile, 0x5c);
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Unequipped Ring of Invisibility from party member 1; stock is 1."
        );
    }

    #[test]
    fn ready_equipment_enforces_stock_ammunition_and_strength_gates() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![5];
        state.party_equipment = default_party_equipment(1);

        assert_eq!(
            state.ready_equipment_from_suffix("1/16"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "No carried Dagger to ready.");

        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix("1/26"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "No arrows for that weapon.");

        state.equipment_stock[EQUIPMENT_ID_ARROWS] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix("1/26"),
            MoveOutcome::Blocked
        );
        assert!(state.message.contains("not strong enough"));
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 1);
    }

    #[test]
    fn ready_equipment_respects_hand_and_slot_occupancy() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![60];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[31] = 1;
        state.equipment_stock[4] = 1;
        state.equipment_stock[16] = 1;

        assert_eq!(state.ready_equipment_from_suffix("1/31"), MoveOutcome::Used);
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], 31);

        assert_eq!(
            state.ready_equipment_from_suffix("1/4"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Weapon hand holds a two-handed item.");

        assert_eq!(
            state.ready_equipment_from_suffix("1/16"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Remove current weapon first.");
    }

    #[test]
    fn cast_reveal_clears_combat_hidden_flags_and_marks_redraw() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = REVEAL_COST;
        state.party[0].level = REVEAL_COST;
        state.spell_charges[REVEAL_SPELL_INDEX] = 1;
        state.combat_actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            32,
            7,
            0,
            4,
            5,
        ]);
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1QW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[REVEAL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(!state.combat_actors[7].is_hidden_or_unrevealed());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Revealed 1 combat actor(s).");
    }

    #[test]
    fn cast_reveal_requires_combat_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].mana = REVEAL_COST;
        state.party[0].level = REVEAL_COST;
        state.spell_charges[REVEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1QW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[REVEAL_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, REVEAL_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_invisibility_marks_current_combat_actor_hidden() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = INVISIBILITY_COST;
        state.party[0].level = INVISIBILITY_COST;
        state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.active_objects[0].type_byte = 0x5c;
        state.active_objects[0].tile = 0x5c;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.active_objects[0].tile, COMBAT_HIDDEN_ACTIVE_OBJECT_TILE);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Invisibility!");
    }

    #[test]
    fn cast_invisibility_requires_combat_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].mana = INVISIBILITY_COST;
        state.party[0].level = INVISIBILITY_COST;
        state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, INVISIBILITY_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_cause_fear_forces_hostile_combat_actors_to_critical_hp() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = CAUSE_FEAR_COST;
        state.party[0].level = CAUSE_FEAR_COST;
        state.spell_charges[CAUSE_FEAR_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            50,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            6,
            0,
            6,
            5,
        ]);
        state.combat_actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_GIANT_RAT,
            7,
            0,
            7,
            5,
        ]);
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            25,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            COMBAT_CLASS_PYTHON,
            8,
            0,
            8,
            5,
        ]);

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1CIQ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[CAUSE_FEAR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.combat_actors[0].hp_or_wound, 20);
        assert_eq!(
            state.combat_actors[6].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().max_hp)
        );
        assert_eq!(state.combat_actors[7].hp_or_wound, 20);
        assert_eq!(
            state.combat_actors[8].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
        );
        assert_eq!(state.message, "Cause Fear affected 2 combat actor(s).");
    }

    #[test]
    fn cast_cause_fear_requires_combat_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].mana = CAUSE_FEAR_COST;
        state.party[0].level = CAUSE_FEAR_COST;
        state.spell_charges[CAUSE_FEAR_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1CIQ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[CAUSE_FEAR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, CAUSE_FEAR_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_cause_fear_skips_charmed_monsters_as_same_faction() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = CAUSE_FEAR_COST;
        state.party[0].level = CAUSE_FEAR_COST;
        state.spell_charges[CAUSE_FEAR_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            50,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
            COMBAT_CLASS_DAEMON,
            6,
            0,
            6,
            5,
        ]);
        state.combat_actors[7] = CombatActorDescriptor::from_row([
            50,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_PYTHON,
            7,
            0,
            7,
            5,
        ]);

        assert_eq!(state.cast_cause_fear(0), MoveOutcome::Cast);

        assert_eq!(state.combat_actors[6].hp_or_wound, 50);
        assert_eq!(
            state.combat_actors[7].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
        );
        assert_eq!(state.message, "Cause Fear affected 1 combat actor(s).");
    }

    #[test]
    fn cast_restore_spells_clear_status_and_heal_hp_after_resource_gates() {
        let mut cure = dungeon_state(open_dungeon_record(), 0, 1, 1);
        cure.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
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
        awaken.party = vec![
            cure.party[0].clone(),
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
        ];
        awaken.party[0].mana = 3;
        awaken.spell_charges[AWAKEN_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut awaken, 'C', "1AZ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(awaken.party[1].status, b'G');
        assert_eq!(awaken.party[2].status, b'S');
        assert_eq!(awaken.spell_charges[AWAKEN_SPELL_INDEX], 0);
        assert_eq!(awaken.party[0].mana, 2);
        assert_eq!(awaken.turn, 1);
        assert_eq!(awaken.message, "Awakened party member 2.");

        let mut heal = dungeon_state(open_dungeon_record(), 0, 1, 1);
        heal.party = cure.party;
        heal.party[0].mana = 3;
        heal.party[1].hp = 8;
        heal.party[1].max_hp = 25;
        heal.spell_charges[HEAL_SPELL_INDEX] = 1;
        let expected_heal = heal.heal_spell_amount(0, 1);

        assert_eq!(
            handle_play_key_input(&mut heal, 'C', "1M2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(expected_heal, 11);
        assert_eq!(heal.party[1].hp, 8 + expected_heal);
        assert_eq!(heal.spell_charges[HEAL_SPELL_INDEX], 0);
        assert_eq!(heal.party[0].mana, 2);
        assert_eq!(heal.turn, 1);
        assert_eq!(heal.message, "Healed party member 2 for 11 HP (19/25).");

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
        resurrect.party[1].class_byte = b'B';
        resurrect.party[1].hp = 0;
        resurrect.party[1].max_hp = 19;
        resurrect.party[1].mana = 0;
        resurrect.party[1].level = 1;
        resurrect.party_experience = vec![0, 350];
        resurrect.party_intelligence = vec![30, 13];
        resurrect.moral_standing = 99;
        resurrect.spell_charges[RESURRECT_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut resurrect, 'C', "1CIM2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(resurrect.party[1].status, b'G');
        assert_eq!(resurrect.party[1].hp, 1);
        assert_eq!(resurrect.party[1].mana, 6);
        assert_eq!(resurrect.party[1].level, 3);
        assert_eq!(resurrect.party[1].max_hp, 90);
        assert_eq!(resurrect.party_experience[1], 350);
        assert_eq!(resurrect.spell_charges[RESURRECT_SPELL_INDEX], 0);
        assert_eq!(resurrect.party[0].mana, 0);
        assert_eq!(resurrect.turn, 1);
        assert_eq!(resurrect.message, "Resurrected party member 2 (1/90).");
    }

    #[test]
    fn cast_great_heal_refuses_during_dungeon_combat_active_substate() {
        // magic.md §8: Great Heal fails during the dungeon combat-active
        // substate, even on living non-Dead targets. The cast still spends
        // resources because the gate runs after charge/MP/level.
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: GREAT_HEAL_COST,
                hp: 10,
                max_hp: 20,
                level: GREAT_HEAL_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 4,
                max_hp: 22,
                level: 1,
            },
        ];
        state.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
        state.combat_active = true;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[1].hp, 4);
        assert_eq!(state.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(state.message, "Failed!");
    }

    #[test]
    fn heal_amount_helper_matches_public_roll_range() {
        assert_eq!(heal_spell_amount_from_raw_roll(0), 1);
        assert_eq!(heal_spell_amount_from_raw_roll(1), 1);
        assert_eq!(heal_spell_amount_from_raw_roll(60), 30);

        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let mut seen = [false; 31];
        for turn in 0..=60 {
            state.turn = turn;
            let amount = state.heal_spell_amount(0, 0);
            assert!((1..=30).contains(&amount));
            seen[amount as usize] = true;
        }
        assert!(seen[1]);
        assert!(seen[30]);
    }

    #[test]
    fn resurrect_rescales_experience_when_moral_standing_is_below_threshold() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: RESURRECT_COST,
                hp: 10,
                max_hp: 20,
                level: RESURRECT_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'M',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 30,
                level: 1,
            },
        ];
        state.party_experience = vec![0, 300];
        state.party_intelligence = vec![30, 21];
        state.moral_standing = 75;
        state.spell_charges[RESURRECT_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1CIM2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party_experience[1], 400);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[1].hp, 1);
        assert_eq!(state.party[1].mana, 21);
        assert_eq!(state.party[1].level, 4);
        assert_eq!(state.party[1].max_hp, 120);
        assert_eq!(state.message, "Resurrected party member 2 (1/120).");
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
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: GREAT_HEAL_COST,
                hp: 10,
                max_hp: 20,
                level: GREAT_HEAL_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
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
            "Safely opened dungeon chest at (1, 1) on DUNGEON:0 level 0; trap generator bypassed by An Sanct, marked visit-local open chest."
        );
    }

    #[test]
    fn cast_an_sanct_opens_without_applying_clean_sidecar_chest_grants() {
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
        assert_eq!(state.gold, starting_gold);
        assert_eq!(state.torches, starting_torches);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("trap generator bypassed by An Sanct")
        );
        assert!(!state.message.contains("authored chest grants"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_an_sanct_opens_ordinary_surface_doors_without_open_tracker() {
        let mut town_grid = open_grid();
        town_grid[1 * 32 + 2] = 0x97;
        let mut town = test_state(town_grid, 1, 1);
        town.spell_charges[OPEN_SPELL_INDEX] = 1;
        town.party[0].mana = OPEN_SPELL_COST;
        town.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut town, 'C', "1AS6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(town.grid[1 * 32 + 2], 0xb8);
        assert_eq!(town.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(town.party[0].mana, 0);
        assert_eq!(town.turn, 1);
        assert_eq!(town.clock, GameClock::new(12, 1).unwrap());
        assert!(town.visibility_dirty);
        assert_eq!(town.door_tracker, None);
        let Area::Town { scene, floor } = town.area else {
            unreachable!("test state should be a town");
        };
        assert!(!town.is_recorded_open_town_door(scene, floor, 2, 1));
        assert_eq!(town.message, "Opened!");

        let mut world_grid = open_world_grid();
        world_grid[world_cell_index(4, 5)] = 0x98;
        let mut world = britannia_state(world_grid, 5, 5);
        world.spell_charges[OPEN_SPELL_INDEX] = 1;
        world.party[0].mana = OPEN_SPELL_COST;
        world.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut world, 'C', "1AS4", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(world.grid[world_cell_index(4, 5)], 0xba);
        assert_eq!(world.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(world.party[0].mana, 0);
        assert_eq!(world.turn, 1);
        assert_eq!(world.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(world.door_tracker, None);
        assert_eq!(world.message, "Opened!");
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
            handle_play_key_input(&mut no_modeled_target, 'C', "1AS6", Path::new("")).unwrap(),
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

