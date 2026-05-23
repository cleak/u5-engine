    #[test]
    fn stair_delta_uses_request_direction_for_public_stair_family() {
        assert_eq!(stair_delta(80, ClimbIntent::Up), Some(1));
        assert_eq!(stair_delta(80, ClimbIntent::Down), Some(-1));
        assert_eq!(stair_delta(81, ClimbIntent::Down), Some(-1));
        assert_eq!(stair_delta(81, ClimbIntent::Up), Some(1));
        assert_eq!(stair_delta(16, ClimbIntent::Up), None);
    }

    #[test]
    fn town_walk_on_stair_delta_uses_facing_selector() {
        assert_eq!(town_walk_on_stair_delta(0xc5, Direction::East), Some(1));
        assert_eq!(town_walk_on_stair_delta(0xc5, Direction::West), Some(-1));
        assert_eq!(town_walk_on_stair_delta(0xc5, Direction::North), None);
        assert_eq!(
            town_walk_on_stair_delta(0xc5, Direction::NorthEast),
            None
        );
        assert_eq!(town_walk_on_stair_delta(0x80, Direction::East), None);
    }

    fn synthetic_combat_arena_record() -> Vec<u8> {
        let mut record = vec![0u8; COMBAT_ARENA_RECORD_LEN];
        for row in 0..COMBAT_ARENA_SIDE {
            let row_start = row * COMBAT_ARENA_ROW_STRIDE;
            for x in 0..COMBAT_ARENA_SIDE {
                record[row_start + x] = (row as u8) * 16 + x as u8;
            }
            for column in COMBAT_ARENA_METADATA_START..COMBAT_ARENA_ROW_STRIDE {
                record[row_start + column] = 0x80 | row as u8;
            }
        }
        for index in 0..6 {
            record[3 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = 0xa0 + index as u8;
            record[3 * COMBAT_ARENA_ROW_STRIDE + 17 + index] = 0xb0 + index as u8;
        }
        for index in 0..16 {
            record[6 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = index as u8;
            record[7 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = 15 - index as u8;
            record[5 * COMBAT_ARENA_ROW_STRIDE + 11 + index] = 0x30 + index as u8;
        }
        record
    }

    #[test]
    fn combat_arena_record_preserves_rows_terrain_and_metadata_slices() {
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();

        assert_eq!(record.terrain(0, 0), Some(0));
        assert_eq!(record.terrain(10, 10), Some(0xaa));
        assert_eq!(record.terrain(11, 0), None);
        assert_eq!(record.metadata(0, 11), Some(0x80));
        assert_eq!(record.metadata(0, 10), None);
        assert_eq!(record.row(3).unwrap()[31], 0x83);
        assert_eq!(record.outdoor_setup_table_a(), [0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5]);
        assert_eq!(record.outdoor_setup_table_b(), [0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5]);
        assert_eq!(record.outdoor_placement_x()[15], 15);
        assert_eq!(record.outdoor_placement_y()[15], 0);
        assert_eq!(record.dungeon_room_sources()[0], 0x30);
        assert_eq!(record.dungeon_room_sources()[15], 0x3f);
        assert_eq!(record.dungeon_room_setup_sources().len(), 16);
        assert_eq!(record.terrain_grid()[10][10], 0xaa);
    }

    #[test]
    fn dungeon_active_monster_combat_uses_public_ambush_floor_arena() {
        let mut state = test_state(open_grid(), 1, 1);
        let object = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 2,
            y: 1,
            z: 3,
            phase: STEADY_PHASE,
            aux1: 0x12,
            aux3: 0x34,
        };

        let note = state
            .enter_dungeon_active_monster_combat(3, object)
            .unwrap();

        assert!(note.contains("active monster"));
        assert!(state.combat_active);
        assert!(state.combat_terrain.iter().all(|row| row
            .iter()
            .all(|tile| *tile == DUNGEON_AMBUSH_ARENA_FLOOR_TILE)));
        assert_eq!(
            (
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x,
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y
            ),
            (6, 5)
        );
        assert!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_empty());
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS],
            ActiveObject {
                x: 6,
                y: 5,
                ..object
            }
        );
    }

    #[test]
    fn dungeon_room_setup_sources_classify_sources_with_high_bit_mask() {
        let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
        let source_base = DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN;
        bytes[source_base] = 0x00;
        bytes[source_base + 1] = 0x3c;
        bytes[source_base + 2] = 0x44;
        bytes[source_base + 3] = 0x80;
        bytes[source_base + 4] = 0xc4;
        let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();

        let sources = record.dungeon_room_setup_sources();

        assert_eq!(
            sources,
            vec![
                DungeonRoomSetupSource {
                    slot: 1,
                    source: 0x3c,
                    kind: DungeonRoomSetupSourceKind::AbsorbableField,
                },
                DungeonRoomSetupSource {
                    slot: 2,
                    source: 0x44,
                    kind: DungeonRoomSetupSourceKind::OrdinaryCombatant,
                },
                DungeonRoomSetupSource {
                    slot: 3,
                    source: 0x80,
                    kind: DungeonRoomSetupSourceKind::SpecialPlacement,
                },
                DungeonRoomSetupSource {
                    slot: 4,
                    source: 0xc4,
                    kind: DungeonRoomSetupSourceKind::OrdinaryCombatant,
                },
            ]
        );
        assert_eq!(dungeon_room_source_sprite(0x44), Some(0xc4));
        assert_eq!(dungeon_room_source_sprite(0xc4), Some(0xc4));
        assert_eq!(dungeon_room_source_sprite(0x80), None);
        assert!(dungeon_room_absorbable_field_family(0x3c));
        assert!(dungeon_room_absorbable_field_family(0x3f));
        assert!(!dungeon_room_absorbable_field_family(0x38));
        assert!(!dungeon_room_absorbable_field_family(0x40));
    }

    #[test]
    fn dungeon_room_combat_setup_copies_terrain_and_scanned_sources() {
        let mut bytes = synthetic_combat_arena_record();
        let source_base = DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN;
        for offset in 0..DUNGEON_ROOM_SOURCE_COUNT {
            bytes[source_base + offset] = 0x00;
        }
        bytes[source_base] = 0x00;
        bytes[source_base + 1] = 0x3c;
        bytes[source_base + 2] = 0x44;
        let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();

        let setup = dungeon_room_combat_setup_from_record(111, &record);

        assert_eq!(setup.arena_index, 111);
        assert_eq!(setup.terrain[10][10], 0xaa);
        assert_eq!(
            setup.setup_sources,
            vec![
                DungeonRoomSetupSource {
                    slot: 1,
                    source: 0x3c,
                    kind: DungeonRoomSetupSourceKind::AbsorbableField,
                },
                DungeonRoomSetupSource {
                    slot: 2,
                    source: 0x44,
                    kind: DungeonRoomSetupSourceKind::OrdinaryCombatant,
                },
            ]
        );

        let instance = dungeon_room_combat_instance_from_setup(&setup, 7);
        assert_eq!(instance.active_objects[7].type_byte, DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE);
        assert_eq!(instance.active_objects[7].tile, DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE);
        assert!(instance.actors[7].is_empty());
        assert_eq!(instance.active_objects[6].tile, 0xc4);
        assert!(!instance.actors[6].is_empty());
    }

    #[test]
    fn dungeon_room_combat_setup_compacts_monsters_and_places_party_after_them() {
        let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
        let source_base = DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN;
        bytes[source_base] = 0x02;
        bytes[source_base + 1] = 0xc4;
        bytes[source_base + 2] = 0x44;
        bytes[source_base + 3] = DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE;
        for index in 0..CBT_PLACEMENT_SLOT_COUNT {
            bytes[CBT_PLACEMENT_X_ROW * COMBAT_ARENA_ROW_STRIDE
                + COMBAT_ARENA_METADATA_START
                + index] = (index + 1) as u8;
            bytes[CBT_PLACEMENT_Y_ROW * COMBAT_ARENA_ROW_STRIDE
                + COMBAT_ARENA_METADATA_START
                + index] = (index + 2) as u8;
        }
        let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
        let setup = dungeon_room_combat_setup_from_record(3, &record);

        let mut instance = dungeon_room_combat_instance_from_setup(&setup, 4);

        assert_eq!(instance.requested_count, 4);
        assert_eq!(instance.placed_count, 2);
        assert_eq!(instance.unplaced_count, 2);
        assert_eq!((instance.active_objects[6].tile, instance.active_objects[6].x), (0xc4, 1));
        assert_eq!((instance.active_objects[7].tile, instance.active_objects[7].x), (0xc4, 2));
        assert_eq!(
            (instance.active_objects[8].tile, instance.active_objects[8].x),
            (DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE, 4)
        );
        assert!(!instance.actors[6].is_empty());
        assert!(!instance.actors[7].is_empty());
        assert!(instance.actors[8].is_empty());

        let state = test_state(open_grid(), 1, 1);
        state.populate_combat_party_at_placement_slots(
            &mut instance.active_objects,
            &mut instance.actors,
            4,
            &setup.placement_slots,
            usize::from(instance.placed_count),
        );

        assert_eq!((instance.active_objects[0].x, instance.active_objects[0].y), (3, 4));
        assert_eq!((instance.actors[0].x, instance.actors[0].y), (3, 4));
    }

    #[test]
    fn arms_price_formulas_use_equipment_base_price_and_speaker_intelligence() {
        assert_eq!(equipment_base_price(3), Some(150));
        assert_eq!(equipment_base_price(8), Some(0));
        assert_eq!(equipment_base_price(47), Some(0));
        assert_eq!(equipment_base_price(EQUIPMENT_COUNT), None);

        let buy = quote_arms_purchase(13, 20).unwrap();
        assert_eq!(buy.base_price, 300);
        assert_eq!(buy.total_price, 420);

        let expert_buy = quote_arms_purchase(13, AVATAR_STAT_MAX).unwrap();
        assert_eq!(expert_buy.total_price, 330);

        let sale = quote_arms_sale(13, 20).unwrap();
        assert_eq!(sale.base_price, 300);
        assert_eq!(sale.offer, 181);

        assert_eq!(
            quote_arms_purchase(15, 20),
            Err(ArmsPurchaseError::NotPurchasable)
        );
        assert_eq!(
            quote_arms_sale(EQUIPMENT_ID_ARROWS, 20),
            Err(ArmsSaleError::NotSellable)
        );
    }

    #[test]
    fn arms_purchase_debits_gold_and_increments_equipment_counter() {
        let mut gold = 420;
        let mut stock = 2;

        let bought = apply_arms_purchase(&mut gold, &mut stock, 13, 20).unwrap();

        assert_eq!(bought.quote.total_price, 420);
        assert_eq!(bought.gold_before, 420);
        assert_eq!(bought.gold_after, 0);
        assert_eq!(bought.stock_before, 2);
        assert_eq!(bought.stock_after, 3);
        assert_eq!((gold, stock), (0, 3));

        let mut state = world_state(open_world_grid(), 10, 10);
        state.gold = 330;
        state.avatar_stats.intelligence = AVATAR_STAT_MAX;

        let state_bought = state.buy_arms_item(13, 0).unwrap();

        assert_eq!(state_bought.quote.total_price, 330);
        assert_eq!(state.gold, 0);
        assert_eq!(state.equipment_stock[13], 1);
    }

    #[test]
    fn arms_purchase_refusals_preserve_gold_and_stock() {
        let mut gold = 419;
        let mut stock = 2;

        assert_eq!(
            apply_arms_purchase(&mut gold, &mut stock, 13, 20),
            Err(ArmsPurchaseError::InsufficientGold {
                available: 419,
                required: 420,
            })
        );
        assert_eq!((gold, stock), (419, 2));

        gold = 1000;
        stock = EQUIPMENT_STOCK_CAP;
        assert_eq!(
            apply_arms_purchase(&mut gold, &mut stock, 13, 20),
            Err(ArmsPurchaseError::StockCap {
                current: EQUIPMENT_STOCK_CAP,
                cap: EQUIPMENT_STOCK_CAP,
            })
        );
        assert_eq!((gold, stock), (1000, EQUIPMENT_STOCK_CAP));

        assert_eq!(
            apply_arms_purchase(&mut gold, &mut stock, EQUIPMENT_COUNT, 20),
            Err(ArmsPurchaseError::InvalidItem)
        );
    }

    #[test]
    fn arms_sale_credits_gold_with_cap_and_decrements_stock() {
        let mut gold = 9900;
        let mut stock = 2;

        let sold = apply_arms_sale(&mut gold, &mut stock, 13, 20).unwrap();

        assert_eq!(sold.quote.offer, 181);
        assert_eq!(sold.gold_before, 9900);
        assert_eq!(sold.gold_after, SHOP_GOLD_CAP);
        assert_eq!(sold.stock_before, 2);
        assert_eq!(sold.stock_after, 1);
        assert_eq!((gold, stock), (SHOP_GOLD_CAP, 1));

        let mut state = world_state(open_world_grid(), 10, 10);
        state.gold = 10;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 10,
            max_hp: 20,
            level: 1,
        });
        state.party_intelligence = vec![AVATAR_STAT_MAX, 20];
        state.equipment_stock[13] = 1;

        let state_sold = state.sell_arms_item(13, 1).unwrap();

        assert_eq!(state_sold.quote.offer, 181);
        assert_eq!(state.gold, 191);
        assert_eq!(state.equipment_stock[13], 0);
    }

    #[test]
    fn arms_sale_refusals_preserve_gold_and_stock() {
        let mut gold = 10;
        let mut stock = 0;

        assert_eq!(
            apply_arms_sale(&mut gold, &mut stock, 13, 20),
            Err(ArmsSaleError::NoStock)
        );
        assert_eq!((gold, stock), (10, 0));

        stock = 2;
        assert_eq!(
            apply_arms_sale(&mut gold, &mut stock, EQUIPMENT_ID_QUARRELS, 20),
            Err(ArmsSaleError::NotSellable)
        );
        assert_eq!((gold, stock), (10, 2));

        assert_eq!(
            apply_arms_sale(&mut gold, &mut stock, EQUIPMENT_COUNT, 20),
            Err(ArmsSaleError::InvalidItem)
        );
        assert_eq!((gold, stock), (10, 2));
    }

    #[test]
    fn shop_surcharge_maps_roll_seed_to_one_through_sixty_four() {
        assert_eq!(shop_surcharge_from_roll_seed(0), 1);
        assert_eq!(shop_surcharge_from_roll_seed(63), 64);
        assert_eq!(shop_surcharge_from_roll_seed(64), 1);
        assert_eq!(shop_surcharge_from_roll_seed(255), 64);
    }

    #[test]
    fn shop_surcharge_runs_only_for_zero_shared_sentinel_and_floors_gold() {
        let mut gold = 100;

        let applied = apply_shop_surcharge(&mut gold, 0, 9);

        assert_eq!(
            applied,
            ShopSurchargeOutcome {
                sentinel: 0,
                surcharge: 10,
                gold_before: 100,
                gold_after: 90,
                applied: true,
            }
        );
        assert_eq!(gold, 90);

        let suppressed = apply_shop_surcharge(&mut gold, 2, 63);

        assert_eq!(
            suppressed,
            ShopSurchargeOutcome {
                sentinel: 2,
                surcharge: 64,
                gold_before: 90,
                gold_after: 90,
                applied: false,
            }
        );
        assert_eq!(gold, 90);

        gold = 5;
        let floored = apply_shop_surcharge(&mut gold, 0, 63);

        assert_eq!(floored.surcharge, 64);
        assert_eq!(floored.gold_before, 5);
        assert_eq!(floored.gold_after, 0);
        assert!(floored.applied);
        assert_eq!(gold, 0);
    }

    #[test]
    fn guild_shop_prices_match_public_rows() {
        assert_eq!(guild_unit_price(GuildShop::TheDen, GuildCommodity::Keys), 190);
        assert_eq!(guild_unit_price(GuildShop::TheDen, GuildCommodity::Gems), 255);
        assert_eq!(
            guild_unit_price(GuildShop::TheDen, GuildCommodity::Torches),
            12
        );
        assert_eq!(
            guild_unit_price(GuildShop::TheGuild, GuildCommodity::Keys),
            160
        );
        assert_eq!(
            guild_unit_price(GuildShop::TheGuild, GuildCommodity::Gems),
            200
        );
        assert_eq!(
            guild_unit_price(GuildShop::TheGuild, GuildCommodity::Torches),
            11
        );
        assert_eq!(
            guild_unit_price(GuildShop::TheNemesis, GuildCommodity::Keys),
            185
        );
        assert_eq!(
            guild_unit_price(GuildShop::TheNemesis, GuildCommodity::Gems),
            225
        );
        assert_eq!(
            guild_unit_price(GuildShop::TheNemesis, GuildCommodity::Torches),
            25
        );
    }

    #[test]
    fn guild_purchase_debits_gold_and_caps_stock_without_partial_mutation() {
        let mut gold = 400;
        let mut gems = 97;

        let bought =
            apply_guild_purchase(&mut gold, &mut gems, GuildShop::TheGuild, GuildCommodity::Gems, 2)
                .unwrap();

        assert_eq!(bought.quote.total_price, 400);
        assert_eq!(bought.gold_before, 400);
        assert_eq!(bought.gold_after, 0);
        assert_eq!(bought.stock_before, 97);
        assert_eq!(bought.stock_after, 99);
        assert_eq!(gold, 0);
        assert_eq!(gems, SHOP_COMMODITY_STOCK_CAP);

        assert_eq!(
            apply_guild_purchase(&mut gold, &mut gems, GuildShop::TheGuild, GuildCommodity::Gems, 1),
            Err(GuildPurchaseError::StockCap {
                current: 99,
                requested: 1,
                cap: SHOP_COMMODITY_STOCK_CAP,
            })
        );
        assert_eq!(gold, 0);
        assert_eq!(gems, 99);

        let mut keys = 0;
        assert_eq!(
            apply_guild_purchase(
                &mut gold,
                &mut keys,
                GuildShop::TheGuild,
                GuildCommodity::Keys,
                1,
            ),
            Err(GuildPurchaseError::InsufficientGold {
                available: 0,
                required: 160,
            })
        );
        assert_eq!(keys, 0);
    }

    #[test]
    fn play_state_guild_purchase_routes_to_selected_counter() {
        let mut state = world_state(open_world_grid(), 10, 10);
        state.gold = 75;
        state.torches = 4;

        let bought = state
            .buy_guild_commodity(GuildShop::TheNemesis, GuildCommodity::Torches, 3)
            .unwrap();

        assert_eq!(bought.quote.unit_price, 25);
        assert_eq!(state.gold, 0);
        assert_eq!(state.torches, 7);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK);
        assert_eq!(state.gems, DEFAULT_GEM_STOCK);
    }

    #[test]
    fn shipwright_prices_match_public_rows() {
        assert_eq!(
            shipwright_price(Shipwright::IslandShipwrights, ShipwrightPurchaseKind::Frigate),
            600
        );
        assert_eq!(
            shipwright_price(Shipwright::IslandShipwrights, ShipwrightPurchaseKind::Skiff),
            200
        );
        assert_eq!(
            shipwright_price(Shipwright::TheCrowsNest, ShipwrightPurchaseKind::Frigate),
            753
        );
        assert_eq!(
            shipwright_price(Shipwright::TheCrowsNest, ShipwrightPurchaseKind::Skiff),
            175
        );
        assert_eq!(
            shipwright_price(Shipwright::TheOakenOar, ShipwrightPurchaseKind::Frigate),
            650
        );
        assert_eq!(
            shipwright_price(Shipwright::TheOakenOar, ShipwrightPurchaseKind::Skiff),
            125
        );
        assert_eq!(
            shipwright_price(Shipwright::TheRustyBucket, ShipwrightPurchaseKind::Frigate),
            700
        );
        assert_eq!(
            shipwright_price(Shipwright::TheRustyBucket, ShipwrightPurchaseKind::Skiff),
            100
        );
    }

    #[test]
    fn shipwright_purchase_queues_delivery_or_adds_skiff_cargo() {
        let mut gold = 800;
        let mut pending = None;

        let frigate = apply_shipwright_purchase(
            &mut gold,
            &mut pending,
            Shipwright::IslandShipwrights,
            ShipwrightPurchaseKind::Frigate,
            12,
            21,
        )
        .unwrap();

        assert_eq!(frigate.status, ShipwrightPurchaseStatus::QueuedFrigate);
        assert_eq!(gold, 200);
        assert_eq!(
            pending,
            Some(PendingVehicleAcquisition::Frigate {
                x: 12,
                y: 21,
                skiffs: 2,
            })
        );

        let skiff = apply_shipwright_purchase(
            &mut gold,
            &mut pending,
            Shipwright::TheRustyBucket,
            ShipwrightPurchaseKind::Skiff,
            99,
            99,
        )
        .unwrap();

        assert_eq!(
            skiff.status,
            ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate
        );
        assert_eq!(gold, 100);
        assert_eq!(
            pending,
            Some(PendingVehicleAcquisition::Frigate {
                x: 12,
                y: 21,
                skiffs: 3,
            })
        );
    }

    #[test]
    fn shipwright_purchase_refusals_preserve_gold_and_pending_delivery() {
        let mut gold = 99;
        let mut pending = Some(PendingVehicleAcquisition::Skiff { x: 1, y: 2 });

        let extra_skiff = apply_shipwright_purchase(
            &mut gold,
            &mut pending,
            Shipwright::TheRustyBucket,
            ShipwrightPurchaseKind::Skiff,
            3,
            4,
        )
        .unwrap();

        assert_eq!(
            extra_skiff.status,
            ShipwrightPurchaseStatus::ExistingDeliveryRefusal
        );
        assert_eq!(gold, 99);
        assert_eq!(pending, Some(PendingVehicleAcquisition::Skiff { x: 1, y: 2 }));

        assert_eq!(
            apply_shipwright_purchase(
                &mut gold,
                &mut pending,
                Shipwright::IslandShipwrights,
                ShipwrightPurchaseKind::Frigate,
                3,
                4,
            ),
            Err(ShipwrightPurchaseError::InsufficientGold {
                available: 99,
                required: 600,
            })
        );
        assert_eq!(gold, 99);
        assert_eq!(pending, Some(PendingVehicleAcquisition::Skiff { x: 1, y: 2 }));
    }

    #[test]
    fn play_state_shipwright_purchase_restores_pending_delivery_to_world() {
        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 700;
        state.return_world = Some(WorldReturn {
            plane: WorldPlane::Britannia,
            x: 10,
            y: 20,
            transport: TransportState::Foot,
            timing_status: TimingStatusTag::Normal,
            sail_cadence: 0,
            sail_stall_pending: false,
            grid: open_world_grid(),
            active_objects: vec![ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 10,
                y: 20,
                z: WorldPlane::Britannia.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }],
            pending_vehicle: None,
        });

        let bought = state
            .buy_shipwright_vehicle(
                Shipwright::TheRustyBucket,
                ShipwrightPurchaseKind::Frigate,
                12,
                21,
            )
            .unwrap();

        assert_eq!(bought.status, ShipwrightPurchaseStatus::QueuedFrigate);
        assert_eq!(state.gold, 0);
        assert!(state.restore_return_world());
        assert_eq!(state.area, Area::World { plane: WorldPlane::Britannia });
        assert_eq!(state.active_objects.len(), 2);
        assert_eq!(state.active_objects[1].type_byte, SHIP_PARKED_FIRST);
        assert_eq!(state.active_objects[1].tile, FIRST_PLAYABLE_FRIGATE_TILE);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (12, 21));
        assert_eq!(state.active_objects[1].aux1, FIRST_PLAYABLE_FULL_SHIP_HULL);
        assert_eq!(state.active_objects[1].aux3, 2);
    }

    #[test]
    fn stable_horse_prices_match_public_rows() {
        assert_eq!(stable_horse_price(Stable::HorseAndRider), 100);
        assert_eq!(stable_horse_price(Stable::TheStablehouse), 130);
        assert_eq!(stable_horse_price(Stable::WishingWellHorses), 160);
    }

    #[test]
    fn horse_purchase_debits_gold_and_places_boardable_horse_object() {
        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 130;

        let bought = state.buy_horse(Stable::TheStablehouse, 9, 8).unwrap();

        assert_eq!(bought.quote.price, 130);
        assert_eq!(bought.gold_before, 130);
        assert_eq!(bought.gold_after, 0);
        assert_eq!(state.gold, 0);
        assert_eq!(bought.active_object_slot, 1);
        assert_eq!(
            bought.horse,
            ActiveObject {
                type_byte: HORSE_PARKED_FIRST,
                tile: FIRST_PLAYABLE_HORSE_TILE,
                x: 9,
                y: 8,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert_eq!(state.active_objects[1], bought.horse);
        assert!(matches!(
            state.boardable_vehicle_slot_at(9, 8).map(|candidate| candidate.transport),
            Some(TransportState::Horse {
                type_byte: HORSE_MOUNTED_FIRST,
                tile: FIRST_PLAYABLE_HORSE_TILE,
            })
        ));
    }

    #[test]
    fn horse_purchase_refusals_preserve_gold_and_objects() {
        let mut poor = test_state(open_grid(), 3, 4);
        poor.gold = 99;

        assert_eq!(
            poor.buy_horse(Stable::HorseAndRider, 9, 8),
            Err(HorsePurchaseError::InsufficientGold {
                available: 99,
                required: 100,
            })
        );
        assert_eq!(poor.gold, 99);
        assert_eq!(poor.active_objects.len(), 1);

        let mut full = test_state(open_grid(), 3, 4);
        full.gold = 500;
        full.active_objects = (0..OOL_SLOTS)
            .map(|slot| ActiveObject {
                type_byte: 1,
                tile: 1,
                x: slot,
                y: 0,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            })
            .collect();

        assert_eq!(
            full.buy_horse(Stable::HorseAndRider, 9, 8),
            Err(HorsePurchaseError::NoActiveObjectSlot)
        );
        assert_eq!(full.gold, 500);
        assert!(!full
            .active_objects
            .iter()
            .any(|object| object.type_byte == HORSE_PARKED_FIRST));
    }

    #[test]
    fn healer_treatment_fees_match_public_rows() {
        assert_eq!(
            healer_treatment_fee(Healer::TheHealersMission, HealerTreatment::Cure),
            HealerTreatmentFee::Bypass
        );
        assert_eq!(
            healer_treatment_fee(Healer::TheHealersMission, HealerTreatment::Heal),
            HealerTreatmentFee::Bypass
        );
        assert_eq!(
            healer_treatment_fee(Healer::TheHealersMission, HealerTreatment::Resurrect),
            HealerTreatmentFee::Price(200)
        );
        assert_eq!(
            healer_treatment_fee(Healer::WoundsOfHonour, HealerTreatment::Cure),
            HealerTreatmentFee::Price(25)
        );
        assert_eq!(
            healer_treatment_fee(Healer::TheSpiritHealers, HealerTreatment::Heal),
            HealerTreatmentFee::Price(45)
        );
        assert_eq!(
            healer_treatment_fee(Healer::HealersSanctum, HealerTreatment::Resurrect),
            HealerTreatmentFee::Price(237)
        );
        assert_eq!(
            healer_treatment_fee(Healer::Sanctuary, HealerTreatment::Heal),
            HealerTreatmentFee::Price(55)
        );
        assert_eq!(
            healer_treatment_fee(Healer::TheShieldOfTruth, HealerTreatment::Cure),
            HealerTreatmentFee::Price(15)
        );
        assert_eq!(
            healer_treatment_fee(Healer::TheEmpath, HealerTreatment::Resurrect),
            HealerTreatmentFee::Price(262)
        );
    }

    #[test]
    fn healer_mission_cure_and_heal_bypass_gold_path() {
        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 0;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'P',
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
                max_hp: 25,
                level: 1,
            },
        ];

        let cured = state
            .buy_healer_treatment(Healer::TheHealersMission, HealerTreatment::Cure, 0)
            .unwrap();

        assert_eq!(cured.quote.fee, HealerTreatmentFee::Bypass);
        assert_eq!(cured.gold_before, 0);
        assert_eq!(cured.gold_after, 0);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 10);

        let healed = state
            .buy_healer_treatment(Healer::TheHealersMission, HealerTreatment::Heal, 1)
            .unwrap();

        assert_eq!(healed.quote.fee, HealerTreatmentFee::Bypass);
        assert_eq!(state.gold, 0);
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.party[1].hp, 25);
    }

    #[test]
    fn paid_healer_heal_and_resurrect_debit_gold_and_apply_treatment() {
        let mut heal = test_state(open_grid(), 3, 4);
        heal.gold = 60;
        heal.party[0].status = b'P';
        heal.party[0].hp = 5;
        heal.party[0].max_hp = 22;

        let healed = heal
            .buy_healer_treatment(Healer::TheShieldOfTruth, HealerTreatment::Heal, 0)
            .unwrap();

        assert_eq!(healed.gold_before, 60);
        assert_eq!(healed.gold_after, 0);
        assert_eq!(healed.hp_before, 5);
        assert_eq!(healed.hp_after, 22);
        assert_eq!(heal.party[0].status, b'P');

        let mut resurrect = test_state(open_grid(), 3, 4);
        resurrect.gold = 225;
        resurrect.party = vec![
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
                class_byte: b'B',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 19,
                level: 1,
            },
        ];
        resurrect.party_experience = vec![0, 350];
        resurrect.party_intelligence = vec![30, 13];
        resurrect.moral_standing = 99;

        let raised = resurrect
            .buy_healer_treatment(Healer::TheSpiritHealers, HealerTreatment::Resurrect, 1)
            .unwrap();

        assert_eq!(raised.gold_before, 225);
        assert_eq!(raised.gold_after, 0);
        assert_eq!(raised.max_hp_after, 90);
        assert_eq!(raised.hp_after, 90);
        assert_eq!(resurrect.party[1].status, b'G');
        assert_eq!(resurrect.party[1].mana, 6);
        assert_eq!(resurrect.party[1].level, 3);
        assert_eq!(resurrect.party_experience[1], 350);
    }

    #[test]
    fn healer_refusals_preserve_gold_status_and_hp() {
        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 39;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;

        assert_eq!(
            state.buy_healer_treatment(Healer::WoundsOfHonour, HealerTreatment::Cure, 0),
            Err(HealerTreatmentError::Untreatable)
        );
        assert_eq!(state.gold, 39);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 20);

        state.party[0].hp = 10;
        assert_eq!(
            state.buy_healer_treatment(Healer::WoundsOfHonour, HealerTreatment::Heal, 0),
            Err(HealerTreatmentError::InsufficientGold {
                available: 39,
                required: 40,
            })
        );
        assert_eq!(state.gold, 39);
        assert_eq!(state.party[0].hp, 10);

        assert_eq!(
            state.buy_healer_treatment(Healer::WoundsOfHonour, HealerTreatment::Heal, 1),
            Err(HealerTreatmentError::InvalidTarget {
                party_len: 1,
                requested: 1,
            })
        );
    }

    #[test]
    fn inn_rate_rows_and_rest_payment_use_supplied_adjusted_rate() {
        assert_eq!(inn_base_room_rate(Inn::TheWayfarerInn), 2);
        assert_eq!(inn_minimum_gold(Inn::TheWayfarerInn), 3);
        assert_eq!(inn_base_room_rate(Inn::TheWarriorsStead), 3);
        assert_eq!(inn_minimum_gold(Inn::TheWarriorsStead), 4);
        assert_eq!(inn_base_room_rate(Inn::TheHauntingInn), 2);
        assert_eq!(inn_minimum_gold(Inn::TheHauntingInn), 3);
        assert_eq!(inn_base_room_rate(Inn::HotelBrittany), 3);
        assert_eq!(inn_minimum_gold(Inn::HotelBrittany), 2);
        assert_eq!(inn_base_room_rate(Inn::TheSmugglersInn), 2);
        assert_eq!(inn_minimum_gold(Inn::TheSmugglersInn), 2);
        assert_eq!(inn_base_room_rate(Inn::TheKingsRansomInn), 3);
        assert_eq!(inn_minimum_gold(Inn::TheKingsRansomInn), 2);

        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: 9,
            mana: 4,
            hp: 12,
            max_hp: 24,
            level: 2,
        });
        state.gold = 20;

        let paid = state.pay_inn_rest(Inn::HotelBrittany, 7).unwrap();

        assert_eq!(paid.quote.party_size, 2);
        assert_eq!(paid.quote.base_room_rate, 7);
        assert_eq!(paid.quote.total_price, 14);
        assert_eq!(paid.gold_before, 20);
        assert_eq!(paid.gold_after, 6);
        assert_eq!(state.gold, 6);
    }

    #[test]
    fn paid_inn_rest_night_recovery_restores_by_class_and_kills_poisoned() {
        assert_eq!(inn_rest_hp_target(b'A', 30), 30);
        assert_eq!(inn_rest_hp_target(b'M', 30), 30);
        assert_eq!(inn_rest_hp_target(b'F', 30), 30);
        assert_eq!(inn_rest_hp_target(b'B', 31), 31);
        assert_eq!(inn_rest_mana_target(b'A', 24), Some(24));
        assert_eq!(inn_rest_mana_target(b'M', 24), Some(24));
        assert_eq!(inn_rest_mana_target(b'B', 24), Some(12));
        assert_eq!(inn_rest_mana_target(b'F', 24), None);

        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 30,
                level: 5,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'G',
                climb_stat: 7,
                mana: 1,
                hp: 10,
                max_hp: 31,
                level: 3,
            },
            PartyMember {
                slot: 2,
                class_byte: b'F',
                status: b'G',
                climb_stat: 4,
                mana: 3,
                hp: 4,
                max_hp: 24,
                level: 2,
            },
            PartyMember {
                slot: 3,
                class_byte: b'M',
                status: b'D',
                climb_stat: 4,
                mana: 0,
                hp: 0,
                max_hp: 24,
                level: 2,
            },
            PartyMember {
                slot: 4,
                class_byte: b'D',
                status: b'P',
                climb_stat: 4,
                mana: 7,
                hp: 6,
                max_hp: 28,
                level: 2,
            },
        ];
        state.party_intelligence = vec![24, 25, 30, 31, 32];

        let (hp, mana, cured) = state.apply_inn_rest_night_recovery();

        assert_eq!((hp, mana, cured), (61, 34, 1));
        assert_eq!(state.party[0].hp, 30);
        assert_eq!(state.party[0].mana, 24);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[1].hp, 31);
        assert_eq!(state.party[1].mana, 12);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[2].hp, 24);
        assert_eq!(state.party[2].mana, 3);
        assert_eq!(state.party[2].status, b'G');
        assert_eq!(state.party[3].hp, 0);
        assert_eq!(state.party[3].mana, 0);
        assert_eq!(state.party[3].status, b'D');
        assert_eq!(state.party[4].hp, 0);
        assert_eq!(state.party[4].mana, 7);
        assert_eq!(state.party[4].status, b'D');
    }

    #[test]
    fn inn_rest_refusals_preserve_gold() {
        let mut gold = 1;
        assert_eq!(
            apply_inn_rest_payment(&mut gold, Inn::TheWayfarerInn, 1, 2),
            Err(InnError::BelowMinimumGold {
                available: 1,
                minimum: 3,
            })
        );
        assert_eq!(gold, 1);

        gold = 5;
        assert_eq!(
            apply_inn_rest_payment(&mut gold, Inn::TheWayfarerInn, 3, 2),
            Err(InnError::InsufficientGold {
                available: 5,
                required: 6,
            })
        );
        assert_eq!(gold, 5);

        assert_eq!(
            quote_inn_rest(Inn::TheWayfarerInn, 0, 2),
            Err(InnError::EmptyParty)
        );
    }

    #[test]
    fn inn_leave_moves_companion_to_registry_and_compacts_party() {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 50;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 8,
                hp: 30,
                max_hp: 30,
                level: 5,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'P',
                climb_stat: 7,
                mana: 3,
                hp: 12,
                max_hp: 28,
                level: 3,
            },
        ];
        state.party_stay_counters = vec![8, 9];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_strengths = vec![30, 17];
        state.party_intelligence = vec![30, 19];
        state.party_experience = vec![0, 700];
        state.party_equipment = vec![
            [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
            [1, 2, 3, 4, 5, 6],
        ];

        let left = state.leave_inn_companion(0x11, 1, 15).unwrap();

        assert_eq!(left.registry_index, 0);
        assert_eq!(left.deposit, 15);
        assert_eq!(state.gold, 35);
        assert_eq!(state.party.len(), 1);
        assert_eq!(state.party[0].slot, 0);
        assert_eq!(state.party_stay_counters, vec![8]);
        assert_eq!(state.inn_registry.len(), 1);
        assert_eq!(state.inn_registry[0].scene_marker, 0x11);
        assert_eq!(state.inn_registry[0].name, *b"IOLO\0\0\0\0\0");
        assert_eq!(state.party_names, vec![*b"AVATAR\0\0\0"]);
        assert_eq!(state.inn_registry[0].member.status, b'P');
        assert_eq!(state.inn_registry[0].strength, 17);
        assert_eq!(state.inn_registry[0].intelligence, 19);
        assert_eq!(state.inn_registry[0].experience, 700);
        assert_eq!(state.inn_registry[0].equipment, [1, 2, 3, 4, 5, 6]);
        assert_eq!(state.inn_registry[0].stay_counter, 0);
    }

    #[test]
    fn inn_pickup_bills_zero_stay_as_one_and_poisoned_guest_returns_dead() {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 50;
        state.inn_registry.push(InnGuestRecord {
            scene_marker: 0x11,
            name: *b"IOLO\0\0\0\0\0",
            member: PartyMember {
                slot: 4,
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
            stay_counter: 0,
        });

        let picked = state.pickup_inn_guest(0x11, 0, 9).unwrap();

        assert_eq!(picked.billable_stay_units, 1);
        assert_eq!(picked.bill, 9);
        assert!(picked.returned_dead_from_poison);
        assert_eq!(state.gold, 41);
        assert!(state.inn_registry.is_empty());
        assert_eq!(state.party.len(), 2);
        assert_eq!(state.party[1].slot, 1);
        assert_eq!(state.party[1].status, b'D');
        assert_eq!(state.party_names[1], *b"IOLO\0\0\0\0\0");
        assert_eq!(state.party[1].hp, 0);
        assert_eq!(state.party_stay_counters[1], 0);
        assert_eq!(state.party_strengths[1], 17);
        assert_eq!(state.party_intelligence[1], 19);
        assert_eq!(state.party_experience[1], 700);
        assert_eq!(state.party_equipment[1], [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn inn_registry_filters_by_scene_and_refusals_preserve_state() {
        let first = InnGuestRecord {
            scene_marker: 0x11,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            member: default_party()[0],
            strength: 30,
            intelligence: 30,
            experience: 0,
            equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
            stay_counter: 2,
        };
        let second = InnGuestRecord {
            scene_marker: 0x12,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            stay_counter: 30,
            ..first
        };
        let mut registry = vec![first, second];
        assert_eq!(inn_guest_indices_for_scene(&registry, 0x11), vec![0]);
        assert_eq!(inn_guest_indices_for_scene(&registry, 0x12), vec![1]);
        assert_eq!(inn_billable_stay_units(0), 1);
        assert_eq!(inn_billable_stay_units(7), 7);
        assert_eq!(inn_billable_stay_units(30), INN_STAY_COUNTER_CAP);

        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 5;
        state.inn_registry = registry.clone();
        assert_eq!(
            state.pickup_inn_guest(0x11, 1, 2),
            Err(InnError::GuestNotAtInn {
                scene_marker: 0x12,
                requested_scene: 0x11,
            })
        );
        assert_eq!(state.gold, 5);
        assert_eq!(state.inn_registry, registry);

        state.party.push(default_party()[0]);
        state.party.push(default_party()[0]);
        state.party.push(default_party()[0]);
        state.party.push(default_party()[0]);
        state.party.push(default_party()[0]);
        assert_eq!(
            state.pickup_inn_guest(0x11, 0, 2),
            Err(InnError::PartyFull)
        );
        assert_eq!(state.inn_registry, registry);

        registry.clear();
    }

    #[test]
    fn inn_registry_month_aging_increments_lodged_guests_to_cap() {
        let base = InnGuestRecord {
            scene_marker: 0x11,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            member: default_party()[0],
            strength: 30,
            intelligence: 30,
            experience: 0,
            equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
            stay_counter: 0,
        };
        let mut registry = vec![
            base,
            InnGuestRecord {
                stay_counter: 24,
                ..base
            },
            InnGuestRecord {
                stay_counter: INN_STAY_COUNTER_CAP,
                ..base
            },
        ];

        assert_eq!(age_inn_registry_month(&mut registry), 2);
        assert_eq!(registry[0].stay_counter, 1);
        assert_eq!(registry[1].stay_counter, INN_STAY_COUNTER_CAP);
        assert_eq!(registry[2].stay_counter, INN_STAY_COUNTER_CAP);
    }

    #[test]
    fn inn_registry_ages_once_on_twenty_eight_day_month_rollover() {
        let base = InnGuestRecord {
            scene_marker: 0x11,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            member: default_party()[0],
            strength: 30,
            intelligence: 30,
            experience: 0,
            equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
            stay_counter: 3,
        };
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::with_date(500, 2, 28, 23, 58).unwrap();
        state.fortunes_of_war = 7;
        state.party_stay_counters = vec![24];
        state.inn_registry = vec![base];

        state.advance_turn_with_minutes(1);
        assert_eq!(state.clock, GameClock::with_date(500, 2, 28, 23, 59).unwrap());
        assert_eq!(state.party_stay_counters, vec![24]);
        assert_eq!(state.inn_registry[0].stay_counter, 3);
        assert_eq!(state.fortunes_of_war, 7);

        state.advance_turn_with_minutes(1);
        assert_eq!(state.clock, GameClock::with_date(500, 3, 1, 0, 0).unwrap());
        assert_eq!(state.party_stay_counters, vec![INN_STAY_COUNTER_CAP]);
        assert_eq!(state.inn_registry[0].stay_counter, 4);
        assert_eq!(state.fortunes_of_war, 0);

        state.advance_turn_with_minutes(1);
        assert_eq!(state.clock, GameClock::with_date(500, 3, 1, 0, 1).unwrap());
        assert_eq!(state.party_stay_counters, vec![INN_STAY_COUNTER_CAP]);
        assert_eq!(state.inn_registry[0].stay_counter, 4);
    }

    #[test]
    fn herbalist_reagent_prices_and_compact_menu_match_public_rows() {
        assert_eq!(
            herbalist_reagent_price(Herbalist::TheHerbalist, Reagent::SulfurAsh),
            None
        );
        assert_eq!(
            herbalist_reagent_price(Herbalist::TheHerbalist, Reagent::Ginseng),
            Some(20)
        );
        assert_eq!(
            herbalist_reagent_price(Herbalist::HealersHerbs, Reagent::SpiderSilk),
            Some(8)
        );
        assert_eq!(
            herbalist_reagent_price(Herbalist::TheAlchemist, Reagent::BlackPearl),
            Some(18)
        );
        assert_eq!(
            herbalist_reagent_price(Herbalist::Mysticism, Reagent::Mandrake),
            Some(15)
        );
        assert_eq!(
            herbalist_reagent_price(Herbalist::TheSharperMage, Reagent::BloodMoss),
            Some(50)
        );
        assert_eq!(
            herbalist_reagent_price(Herbalist::TheSharperMage, Reagent::BlackPearl),
            None
        );

        let menu = herbalist_menu_entries(Herbalist::TheHerbalist);
        assert_eq!(
            menu,
            vec![
                ReagentMenuEntry {
                    letter: 'A',
                    reagent: Reagent::Ginseng,
                    unit_price: 20,
                },
                ReagentMenuEntry {
                    letter: 'B',
                    reagent: Reagent::Garlic,
                    unit_price: 18,
                },
                ReagentMenuEntry {
                    letter: 'C',
                    reagent: Reagent::SpiderSilk,
                    unit_price: 12,
                },
                ReagentMenuEntry {
                    letter: 'D',
                    reagent: Reagent::Nightshade,
                    unit_price: 12,
                },
                ReagentMenuEntry {
                    letter: 'E',
                    reagent: Reagent::Mandrake,
                    unit_price: 13,
                },
            ]
        );
    }

    #[test]
    fn reagent_purchase_debits_gold_and_routes_to_reagent_counter() {
        let mut gold = 90;
        let mut blood_moss = 4;

        let bought = apply_reagent_purchase(
            &mut gold,
            &mut blood_moss,
            Herbalist::TheAlchemist,
            Reagent::BloodMoss,
            3,
        )
        .unwrap();

        assert_eq!(bought.quote.unit_price, 30);
        assert_eq!(bought.quote.total_price, 90);
        assert_eq!(bought.gold_before, 90);
        assert_eq!(bought.gold_after, 0);
        assert_eq!(bought.stock_before, 4);
        assert_eq!(bought.stock_after, 7);
        assert_eq!(gold, 0);
        assert_eq!(blood_moss, 7);

        let mut state = world_state(open_world_grid(), 10, 10);
        state.gold = 24;
        state.reagents = [0; REAGENT_COUNT];

        let silk = state
            .buy_reagent(Herbalist::Mysticism, Reagent::SpiderSilk, 4)
            .unwrap();

        assert_eq!(silk.quote.total_price, 24);
        assert_eq!(state.gold, 0);
        assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 4);
        assert_eq!(state.reagents[REAGENT_BLOOD_MOSS], 0);
    }

    #[test]
    fn reagent_purchase_refusals_preserve_gold_and_stock() {
        let mut gold = 50;
        let mut sulfur_ash = 0;

        assert_eq!(
            apply_reagent_purchase(
                &mut gold,
                &mut sulfur_ash,
                Herbalist::TheHerbalist,
                Reagent::SulfurAsh,
                1,
            ),
            Err(ReagentPurchaseError::NotStocked)
        );
        assert_eq!(gold, 50);
        assert_eq!(sulfur_ash, 0);

        assert_eq!(
            apply_reagent_purchase(
                &mut gold,
                &mut sulfur_ash,
                Herbalist::HealersHerbs,
                Reagent::SulfurAsh,
                0,
            ),
            Err(ReagentPurchaseError::ZeroQuantity)
        );
        assert_eq!(gold, 50);
        assert_eq!(sulfur_ash, 0);

        let mut nightshade = 98;
        assert_eq!(
            apply_reagent_purchase(
                &mut gold,
                &mut nightshade,
                Herbalist::Mysticism,
                Reagent::Nightshade,
                2,
            ),
            Err(ReagentPurchaseError::StockCap {
                current: 98,
                requested: 2,
                cap: SHOP_COMMODITY_STOCK_CAP,
            })
        );
        assert_eq!(nightshade, 98);

        assert_eq!(
            apply_reagent_purchase(
                &mut gold,
                &mut sulfur_ash,
                Herbalist::HealersHerbs,
                Reagent::SulfurAsh,
                5,
            ),
            Err(ReagentPurchaseError::InsufficientGold {
                available: 50,
                required: 60,
            })
        );
        assert_eq!(gold, 50);
        assert_eq!(sulfur_ash, 0);
    }

    #[test]
    fn tavern_price_tables_match_public_rows() {
        assert_eq!(tavern_provision_unit_price(Tavern::TheHonestMeal), 10);
        assert_eq!(tavern_provision_unit_price(Tavern::TheWayfarerTavern), 15);
        assert_eq!(tavern_provision_unit_price(Tavern::TheSwordAndKeg), 20);
        assert_eq!(
            tavern_provision_unit_price(Tavern::TheSlaughteredLamb),
            25
        );
        assert_eq!(tavern_provision_unit_price(Tavern::TheHumblePalate), 30);
        assert_eq!(tavern_provision_unit_price(Tavern::TheBlueBoarTavern), 25);
        assert_eq!(tavern_provision_unit_price(Tavern::TheCatsLair), 20);
        assert_eq!(tavern_provision_unit_price(Tavern::TheFallenVirgin), 25);
        assert_eq!(tavern_provision_unit_price(Tavern::TheFolleyTap), 30);

        let humble = quote_tavern_round_drink(Tavern::TheHumblePalate, 3).unwrap();
        assert_eq!(humble.menu_letter, 'A');
        assert_eq!(humble.unit_price, 2);
        assert_eq!(humble.total_price, 6);

        let lamb = quote_tavern_round_drink(Tavern::TheSlaughteredLamb, 2).unwrap();
        assert_eq!(lamb.menu_letter, 'A');
        assert_eq!(lamb.unit_price, 3);
        assert_eq!(lamb.total_price, 6);

        assert_eq!(blue_boar_drink_price(BlueBoarDrinkChoice::A), 18);
        assert_eq!(blue_boar_drink_price(BlueBoarDrinkChoice::B), 192);
        assert_eq!(blue_boar_drink_price(BlueBoarDrinkChoice::C), 79);
        assert_eq!(blue_boar_drink_price(BlueBoarDrinkChoice::D), 30);
        assert_eq!(blue_boar_drink_price(BlueBoarDrinkChoice::E), 275);
        assert_eq!(blue_boar_drink_price(BlueBoarDrinkChoice::F), 98);
    }

    #[test]
    fn tavern_round_drink_debits_once_per_living_party_member() {
        let mut state = test_state(open_grid(), 3, 4);
        state.gold = 30;
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
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 20,
                level: 1,
            },
        ];

        let drank = state.buy_tavern_round_drink(Tavern::TheSwordAndKeg).unwrap();

        assert_eq!(drank.total_price, 10);
        assert_eq!(drank.gold_before, 30);
        assert_eq!(drank.gold_after, 20);
        assert_eq!(state.gold, 20);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.party[2].status, b'D');

        assert_eq!(
            quote_tavern_round_drink(Tavern::TheSwordAndKeg, 0),
            Err(TavernDrinkError::NoLivingParty)
        );
    }

    #[test]
    fn blue_boar_fixed_drinks_debit_exact_choice_price() {
        let mut gold = 98;

        let drank = apply_blue_boar_drink(&mut gold, BlueBoarDrinkChoice::F).unwrap();

        assert_eq!(drank.total_price, 98);
        assert_eq!(drank.gold_before, 98);
        assert_eq!(drank.gold_after, 0);
        assert_eq!(gold, 0);

        assert_eq!(
            apply_blue_boar_drink(&mut gold, BlueBoarDrinkChoice::A),
            Err(TavernDrinkError::InsufficientGold {
                available: 0,
                required: 18,
            })
        );
    }

    #[test]
    fn provision_purchase_can_partially_complete_until_gold_or_capacity_runs_out() {
        let mut gold = 65;
        let mut food = 10;

        let bought =
            apply_provision_purchase(&mut gold, &mut food, Tavern::TheWayfarerTavern, 10).unwrap();

        assert_eq!(bought.quote.unit_price, 15);
        assert_eq!(bought.requested_quantity, 10);
        assert_eq!(bought.purchased_quantity, 4);
        assert_eq!(bought.total_price, 60);
        assert_eq!(bought.gold_before, 65);
        assert_eq!(bought.gold_after, 5);
        assert_eq!(bought.food_before, 10);
        assert_eq!(bought.food_after, 14);
        assert_eq!(gold, 5);
        assert_eq!(food, 14);

        let mut state = world_state(open_world_grid(), 10, 10);
        state.gold = 1000;
        state.food = SHOP_FOOD_STOCK_CAP - 2;

        let capped = state
            .buy_provisions(Tavern::TheHonestMeal, 5)
            .expect("food capacity permits two units");

        assert_eq!(capped.purchased_quantity, 2);
        assert_eq!(capped.total_price, 20);
        assert_eq!(state.food, SHOP_FOOD_STOCK_CAP);
        assert_eq!(state.gold, 980);
    }

    #[test]
    fn provision_purchase_refusals_preserve_gold_and_food() {
        let mut gold = 9;
        let mut food = 12;

        assert_eq!(
            apply_provision_purchase(&mut gold, &mut food, Tavern::TheHonestMeal, 0),
            Err(ProvisionPurchaseError::ZeroQuantity)
        );
        assert_eq!((gold, food), (9, 12));

        assert_eq!(
            apply_provision_purchase(&mut gold, &mut food, Tavern::TheHonestMeal, 1),
            Err(ProvisionPurchaseError::InsufficientGold {
                available: 9,
                required_per_unit: 10,
            })
        );
        assert_eq!((gold, food), (9, 12));

        gold = 100;
        food = SHOP_FOOD_STOCK_CAP;
        assert_eq!(
            apply_provision_purchase(&mut gold, &mut food, Tavern::TheHonestMeal, 1),
            Err(ProvisionPurchaseError::NoNeed)
        );
        assert_eq!((gold, food), (100, SHOP_FOOD_STOCK_CAP));
    }

    #[test]
    fn sage_topic_matching_uses_cap_case_and_strict_boundary() {
        let hone = find_sage_topic(&SAGE_RUMOUR_TABLE, "  HONE map").unwrap();
        assert_eq!(hone.entry.subject, "Malik");
        assert_eq!(hone.input_len, 8);

        assert!(sage_topic_matches_input("hone", "hone"));
        assert!(sage_topic_matches_input("hone", "hone clue"));
        assert!(!sage_topic_matches_input("hone", "honesty"));
        assert_eq!(
            find_sage_topic(&SAGE_RUMOUR_TABLE, "honesty"),
            Err(SageRumourError::NoTopicMatch)
        );
        let lighthouse = find_sage_topic(&SAGE_RUMOUR_TABLE, "unde lore beyond cap").unwrap();
        assert_eq!(lighthouse.entry.subject, "Jotham");
        assert_eq!(lighthouse.input_len, SAGE_TOPIC_INPUT_LIMIT);
        assert_eq!(
            find_sage_topic(&SAGE_RUMOUR_TABLE, "1234567890123456"),
            Err(SageRumourError::NoTopicMatch)
        );
        assert_eq!(
            find_sage_topic(&SAGE_RUMOUR_TABLE, " "),
            Err(SageRumourError::EmptyInput)
        );
    }

    #[test]
    fn sage_rumour_lookup_renders_paid_table_substitutions() {
        let mut state = world_state(open_world_grid(), 10, 10);
        state.gold = 20;

        let bought = state
            .consult_sage_rumour(
                &SAGE_RUMOUR_TABLE,
                "HONE",
                SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
            )
            .unwrap();

        assert_eq!(state.gold, 20);
        assert_eq!(bought.rendered, "Seek ye Malik in Moonglow!");
        assert_eq!(bought.quote.entry.fee, 50);
    }

    #[test]
    fn sage_rumour_refusals_preserve_gold() {
        let gold = 24;

        assert_eq!(
            apply_sage_rumour_lookup(
                &SAGE_RUMOUR_TABLE,
                "1234567890123456",
                SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
            ),
            Err(SageRumourError::NoTopicMatch)
        );
        assert_eq!(gold, 24);

        assert_eq!(
            apply_sage_rumour_lookup(
                &SAGE_RUMOUR_TABLE,
                "valor",
                SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
            ),
            Err(SageRumourError::NoTopicMatch)
        );
        assert_eq!(gold, 24);
    }

    #[test]
    fn combat_class_stats_expose_published_monster_rows() {
        let dragon = combat_class_stats(39).unwrap();
        assert_eq!(dragon.name, "Dragon");
        assert_eq!(dragon.raw_row(), [30, 25, 25, 10, 30, 99, 2, 30]);
        assert_eq!(dragon.reward_unit(), 25);
        assert_eq!(dragon.mass_charm_threshold(), 25);

        let guard = combat_class_stats(12).unwrap();
        assert_eq!(guard.raw_row(), [22, 30, 10, 6, 30, 99, 8, 5]);
        assert_eq!(guard.mass_charm_threshold(), 10);

        let reserved = combat_class_stats(42).unwrap();
        assert_eq!(reserved.raw_row(), [0; 8]);
        assert_eq!(combat_class_stats(11), None);
        assert_eq!(combat_class_stats(COMBAT_CLASS_COUNT as u8), None);
    }

    #[test]
    fn combat_ranged_effect_stats_expose_published_side_rows() {
        let mage = combat_ranged_effect_stats(0).unwrap();
        assert_eq!(mage.name, "Mage");
        assert_eq!(mage.range_effect_selector, 7);
        assert_eq!(mage.payload, 4);
        assert!(mage.scene_resistance);
        assert!(!mage.cast_like_branch);
        assert!(!mage.pre_gate_bypass);

        let gremlin = combat_ranged_effect_stats(25).unwrap();
        assert_eq!(gremlin.range_effect_selector, 1);
        assert!(gremlin.cast_like_branch);

        let mimic = combat_ranged_effect_stats(26).unwrap();
        assert_eq!(mimic.range_effect_selector, 2);
        assert_eq!(mimic.payload, 5);
        assert!(mimic.pre_gate_bypass);

        let dragon = combat_ranged_effect_stats(39).unwrap();
        assert_eq!(dragon.range_effect_selector, 9);
        assert_eq!(dragon.payload, 3);
        assert!(!dragon.scene_resistance);

        assert_eq!(combat_ranged_effect_stats(11), None);
        assert_eq!(combat_ranged_effect_stats(48), None);
    }

    #[test]
    fn combat_class_traits_expose_published_behavior_rows() {
        let mage = combat_class_traits(0).unwrap();
        assert!(mage.turnable_attack);
        assert!(!mage.physical_immune);

        let wanderer = combat_class_traits(13).unwrap();
        assert!(wanderer.physical_immune);
        assert!(wanderer.blink);
        assert!(wanderer.teleport_capable);
        assert!(wanderer.turnable_attack);
        assert!(wanderer.vanish_branch);

        let ghost = combat_class_traits(23).unwrap();
        assert!(ghost.physical_half);
        assert!(ghost.blink);
        assert!(!ghost.physical_immune);

        let gazer = combat_class_traits(28).unwrap();
        assert!(gazer.special_death);
        assert!(gazer.possess);
        assert!(gazer.turnable_attack);

        let dragon = combat_class_traits(39).unwrap();
        assert!(dragon.summon_daemon);
        assert!(!dragon.turnable_attack);

        let reserved = combat_class_traits(42).unwrap();
        assert_eq!(reserved.name, "Reserved gap");
        assert_eq!(reserved, traits_without_identity(42, "Reserved gap"));

        assert_eq!(combat_class_traits(11), None);
        assert_eq!(combat_class_traits(48), None);
    }

    fn traits_without_identity(class: u8, name: &'static str) -> CombatClassTraits {
        CombatClassTraits {
            class,
            name,
            ..CombatClassTraits::empty()
        }
    }

    #[test]
    fn combat_class_traits_map_from_sprite_and_modify_physical_damage() {
        let skeleton = combat_class_traits_for_sprite_byte(0xc4).unwrap();
        assert_eq!(skeleton.class, 33);
        assert!(skeleton.physical_half);

        let spider = combat_class_traits_for_sprite_byte(0x9a).unwrap();
        assert_eq!(spider.class, 22);
        assert!(spider.poison_status_attack);

        assert_eq!(combat_class_traits_for_sprite_byte(0xe8), None);

        assert_eq!(resolve_physical_damage_for_class(33, 9, false), 4);
        assert_eq!(resolve_physical_damage_for_class(13, 9, false), 0);
        assert_eq!(resolve_physical_damage_for_class(32, 9, false), 9);
        assert_eq!(resolve_physical_damage_for_class(33, 9, true), 9);
        assert_eq!(resolve_physical_damage_for_class(99, 9, false), 9);
    }

    #[test]
    fn amulet_turning_catalog_row_matches_readied_slot() {
        assert_eq!(EQUIPMENT_ID_AMULET_TURNING, 45);
        assert_eq!(EQUIPMENT_NAMES[EQUIPMENT_ID_AMULET_TURNING], "Amulet/Turning");
        assert_eq!(
            EQUIPMENT_CLASS_TAGS[EQUIPMENT_ID_AMULET_TURNING],
            EQUIPMENT_TAG_AMULET
        );

        let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        equipment[EQUIP_SLOT_RING] = EQUIPMENT_ID_AMULET_TURNING as u8;
        assert!(!is_amulet_turning_readied(&equipment));

        equipment[EQUIP_SLOT_RING] = EQUIPMENT_EMPTY;
        equipment[EQUIP_SLOT_AMULET] = EQUIPMENT_ID_AMULET_TURNING as u8;
        assert!(is_amulet_turning_readied(&equipment));
    }

    #[test]
    fn amulet_turning_scatter_requires_all_documented_preconditions() {
        assert!(resolve_amulet_turning_scatter(true, true, true, 127));
        assert!(!resolve_amulet_turning_scatter(true, true, true, 128));
        assert!(!resolve_amulet_turning_scatter(false, true, true, 0));
        assert!(!resolve_amulet_turning_scatter(true, false, true, 0));
        assert!(!resolve_amulet_turning_scatter(true, true, false, 0));

        assert!(resolve_amulet_turning_scatter_for_class(28, true, true, 0).unwrap());
        assert!(!resolve_amulet_turning_scatter_for_class(39, true, true, 0).unwrap());
        assert_eq!(
            resolve_amulet_turning_scatter_for_class(11, true, true, 0),
            None
        );
    }

    #[test]
    fn amulet_turning_party_target_wrapper_uses_living_status_and_equipment() {
        let target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };
        let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        equipment[EQUIP_SLOT_AMULET] = EQUIPMENT_ID_AMULET_TURNING as u8;

        assert!(resolve_amulet_turning_scatter_for_party_target(28, target, &equipment, 1).unwrap());
        assert!(!resolve_amulet_turning_scatter_for_party_target(
            28,
            PartyMember {
                hp: 0,
                status: b'D',
                ..target
            },
            &equipment,
            1
        )
        .unwrap());
    }

    #[test]
    fn combat_ai_attack_route_uses_range_cap_and_adjacent_melee_boundary() {
        assert_eq!(resolve_combat_ai_attack_route(11, 1), None);

        assert_eq!(resolve_combat_ai_attack_route(28, 6), Some(CombatAiAttackRoute::OutOfRange));
        assert_eq!(resolve_combat_ai_attack_route(28, 1), Some(CombatAiAttackRoute::Melee));
        assert_eq!(
            resolve_combat_ai_attack_route(28, 2),
            Some(CombatAiAttackRoute::RangedEffect {
                range_effect_selector: 5,
                payload: 6,
                scene_resistance: true,
                cast_like_branch: false,
                pre_gate_bypass: false,
            })
        );
        assert_eq!(
            resolve_combat_ai_attack_route(25, 2),
            Some(CombatAiAttackRoute::OutOfRange)
        );
        assert_eq!(
            resolve_combat_ai_attack_route(25, 1),
            Some(CombatAiAttackRoute::Melee)
        );
    }

    #[test]
    fn combat_ai_special_hook_uses_published_class_traits_and_gates() {
        assert_eq!(
            resolve_combat_ai_special_hook(11, true, 0, 0, true),
            None
        );
        assert_eq!(
            resolve_combat_ai_special_hook(28, true, 1, 1, false),
            Some(CombatAiSpecialHook::Possess)
        );
        assert_eq!(
            resolve_combat_ai_special_hook(28, false, 0, 0, true),
            None
        );
        assert_eq!(
            resolve_combat_ai_special_hook(23, false, 0, 0, true),
            Some(CombatAiSpecialHook::Blink)
        );
        assert_eq!(
            resolve_combat_ai_special_hook(23, false, 1, 0, true),
            None
        );
        assert_eq!(
            resolve_combat_ai_special_hook(39, false, 0, 8, true),
            Some(CombatAiSpecialHook::SummonDaemon)
        );
        assert_eq!(
            resolve_combat_ai_special_hook(39, false, 0, 9, true),
            None
        );
        assert_eq!(
            resolve_combat_ai_special_hook(39, false, 0, 8, false),
            None
        );
        assert_eq!(
            resolve_combat_ai_special_hook(32, true, 0, 0, true),
            None
        );
    }

    #[test]
    fn combat_ai_special_hook_preserves_documented_branch_order() {
        let mut traits = CombatClassTraits {
            class: 250,
            name: "Synthetic",
            ..CombatClassTraits::empty()
        };
        traits.possess = true;
        traits.blink = true;
        traits.summon_daemon = true;

        assert!(combat_ai_special_one_in_eight_gate(0));
        assert!(combat_ai_special_one_in_eight_gate(8));
        assert!(!combat_ai_special_one_in_eight_gate(1));

        assert_eq!(
            resolve_combat_ai_special_hook_for_traits(traits, true, 0, 0, true),
            Some(CombatAiSpecialHook::Possess)
        );
        assert_eq!(
            resolve_combat_ai_special_hook_for_traits(traits, false, 0, 0, true),
            Some(CombatAiSpecialHook::Blink)
        );
        assert_eq!(
            resolve_combat_ai_special_hook_for_traits(traits, false, 1, 0, true),
            Some(CombatAiSpecialHook::SummonDaemon)
        );
        assert_eq!(
            resolve_combat_ai_special_hook_for_traits(traits, false, 1, 0, false),
            None
        );
    }

    fn possess_member(status: u8, hp: u16) -> PartyMember {
        PartyMember {
            slot: 0,
            class_byte: 1,
            status,
            climb_stat: 0,
            mana: 0,
            hp,
            max_hp: 20,
            level: 1,
        }
    }

    fn possess_candidate(
        descriptor: CombatActorDescriptor,
        member: Option<PartyMember>,
    ) -> CombatPossessCandidateView {
        combat_possess_candidate_view(descriptor, member, false, false)
    }

    #[test]
    fn combat_possess_candidate_gate_accepts_only_live_visible_idle_party_slots() {
        let live = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        let hidden = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            0,
            0,
            0,
            4,
            5,
        ]);
        let dead = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            0,
            0,
            0,
            4,
            5,
        ]);
        let passive = CombatActorDescriptor::from_row([20, 1, 0, 0, 0, 0, 4, 5]);

        assert!(combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(live, Some(possess_member(b'G', 10))),
            None
        ));
        assert!(combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(live, Some(possess_member(b'P', 10))),
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(live, Some(possess_member(b'S', 10))),
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(live, Some(possess_member(b'D', 0))),
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(hidden, Some(possess_member(b'G', 10))),
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(dead, Some(possess_member(b'G', 10))),
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(passive, Some(possess_member(b'G', 10))),
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            CombatPossessCandidateView {
                suppressed: true,
                ..possess_candidate(live, Some(possess_member(b'G', 10)))
            },
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            CombatPossessCandidateView {
                invisible_or_unrevealed: true,
                ..possess_candidate(live, Some(possess_member(b'G', 10)))
            },
            None
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            0,
            possess_candidate(live, Some(possess_member(b'G', 10))),
            Some(0)
        ));
        assert!(!combat_possess_candidate_reaches_resistance(
            COMBAT_PARTY_ACTOR_SLOTS,
            possess_candidate(live, Some(possess_member(b'G', 10))),
            None
        ));
    }

    #[test]
    fn combat_possess_random_slot_must_itself_reach_resistance() {
        let live = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        let mut candidates = [possess_candidate(CombatActorDescriptor::empty(), None); COMBAT_ACTOR_SLOTS];
        candidates[3] = possess_candidate(live, Some(possess_member(b'G', 10)));
        candidates[4] = possess_candidate(live, Some(possess_member(b'S', 10)));

        assert_eq!(
            resolve_combat_possess_candidate_slot(&candidates, 3, None),
            Some(3)
        );
        assert_eq!(
            resolve_combat_possess_candidate_slot(&candidates, 4, None),
            None
        );
        assert_eq!(
            resolve_combat_possess_candidate_slot(&candidates, COMBAT_ACTOR_SLOTS, None),
            None
        );
    }

    #[test]
    fn combat_possess_resistance_outcome_tracks_landing_side_effects() {
        assert_eq!(
            resolve_combat_possess_resistance_outcome(1, COMBAT_CLASS_DAEMON, Some(1), true),
            CombatPossessResistanceOutcome::Blocked
        );
        assert_eq!(
            resolve_combat_possess_resistance_outcome(1, COMBAT_CLASS_DAEMON, Some(1), false),
            CombatPossessResistanceOutcome::Landed {
                cleared_active_player: true,
                daemon_clears_self: true,
            }
        );
        assert_eq!(
            resolve_combat_possess_resistance_outcome(1, 28, Some(0), false),
            CombatPossessResistanceOutcome::Landed {
                cleared_active_player: false,
                daemon_clears_self: false,
            }
        );
    }

    #[test]
    fn poison_status_attack_poisons_living_good_party_member_without_damage() {
        let mut target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };

        let outcome =
            resolve_poison_status_attack_for_party_target(22, &mut target, true, 9).unwrap();

        assert_eq!(
            outcome,
            CombatPoisonStatusAttackOutcome::PoisonedPartyMember {
                status_before: b'G',
                status_after: b'P',
            }
        );
        assert_eq!(target.status, b'P');
        assert_eq!(target.hp, 12);
    }

    #[test]
    fn poison_status_attack_preserves_gate_and_fallback_damage_boundaries() {
        let mut good_target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };
        assert_eq!(
            resolve_poison_status_attack_for_party_target(22, &mut good_target, false, 9).unwrap(),
            CombatPoisonStatusAttackOutcome::GateRejected
        );
        assert_eq!(good_target.status, b'G');

        let mut poisoned_target = PartyMember {
            status: b'P',
            ..good_target
        };
        assert_eq!(
            resolve_poison_status_attack_for_party_target(22, &mut poisoned_target, true, 9)
                .unwrap(),
            CombatPoisonStatusAttackOutcome::FallbackDamage { raw_damage: 9 }
        );
        assert_eq!(poisoned_target.status, b'P');
        assert_eq!(poisoned_target.hp, 12);

        let mut non_poison_attacker_target = good_target;
        assert_eq!(
            resolve_poison_status_attack_for_party_target(
                32,
                &mut non_poison_attacker_target,
                true,
                9,
            )
            .unwrap(),
            CombatPoisonStatusAttackOutcome::NotPoisonStatusClass
        );
        assert_eq!(
            resolve_poison_status_attack_for_party_target(
                11,
                &mut non_poison_attacker_target,
                true,
                9,
            ),
            None
        );
    }

    #[test]
    fn combat_field_kind_maps_spell_bytes_and_acceptance_gate() {
        assert_eq!(
            CombatArenaFieldKind::from_kind_byte(COMBAT_FIELD_KIND_POISON),
            Some(CombatArenaFieldKind::Poison)
        );
        assert_eq!(
            CombatArenaFieldKind::from_kind_byte(COMBAT_FIELD_KIND_SLEEP),
            Some(CombatArenaFieldKind::Sleep)
        );
        assert_eq!(
            CombatArenaFieldKind::from_kind_byte(COMBAT_FIELD_KIND_FIRE),
            Some(CombatArenaFieldKind::Fire)
        );
        assert_eq!(
            CombatArenaFieldKind::from_kind_byte(COMBAT_FIELD_KIND_ENERGY),
            Some(CombatArenaFieldKind::Energy)
        );
        assert_eq!(CombatArenaFieldKind::from_kind_byte(0x32), None);
        assert_eq!(CombatArenaFieldKind::Poison.kind_byte(), 0x33);

        assert!(resolve_combat_field_placement_acceptance(
            CombatArenaFieldKind::Poison,
            false
        ));
        assert!(!resolve_combat_field_placement_acceptance(
            CombatArenaFieldKind::Fire,
            false
        ));
        assert!(resolve_combat_field_placement_acceptance(
            CombatArenaFieldKind::Sleep,
            true
        ));
    }

    fn seed_for_first_mod_roll(modulus: u8, expected: u8) -> u16 {
        for seed in 0..=u16::MAX {
            let mut prng = seed;
            if u5_prng_range_u16(&mut prng, 0, u16::from(modulus - 1)) as u8 == expected {
                return seed;
            }
        }
        panic!("no deterministic PRNG seed found for requested mod roll");
    }

    #[test]
    fn combat_field_placement_callback_uses_poison_immediate_and_one_in_eight_gate() {
        let mut state = world_state(open_world_grid(), 10, 20);
        assert_eq!(COMBAT_ARENA_FIELD_RANDOM_GATE_DENOMINATOR, 8);

        state.prng_state = seed_for_first_mod_roll(COMBAT_ARENA_FIELD_RANDOM_GATE_DENOMINATOR, 7);
        assert!(state.combat_arena_field_placement_callback_accepts(
            0,
            COMBAT_PARTY_ACTOR_SLOTS,
            POISON_FIELD_SPELL_INDEX
        ));

        state.prng_state = seed_for_first_mod_roll(COMBAT_ARENA_FIELD_RANDOM_GATE_DENOMINATOR, 0);
        assert!(state.combat_arena_field_placement_callback_accepts(
            0,
            COMBAT_PARTY_ACTOR_SLOTS,
            FIRE_FIELD_SPELL_INDEX
        ));

        state.prng_state = seed_for_first_mod_roll(COMBAT_ARENA_FIELD_RANDOM_GATE_DENOMINATOR, 7);
        assert!(!state.combat_arena_field_placement_callback_accepts(
            0,
            COMBAT_PARTY_ACTOR_SLOTS,
            ENERGY_FIELD_SPELL_INDEX
        ));
    }

    #[test]
    fn combat_field_contact_skips_current_actor_and_poison_linked_monster_tiles() {
        let mut target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };

        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Poison,
                2,
                2,
                0x40,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::SkippedCurrentActor
        );
        assert_eq!(target.status, b'G');

        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Poison,
                1,
                2,
                0x80,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass
        );
        assert_eq!(target.status, b'G');
    }

    #[test]
    fn combat_field_contact_applies_party_poison_sleep_and_damage_inputs() {
        let mut target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };

        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Poison,
                1,
                2,
                0x40,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::PoisonedPartyMember {
                status_before: b'G',
                status_after: b'P',
            }
        );
        assert_eq!(target.status, b'P');
        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Poison,
                1,
                2,
                0x40,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage: 20 }
        );

        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Sleep,
                1,
                2,
                0x40,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::SleptPartyMember {
                status_before: b'P',
                status_after: b'S',
            }
        );
        assert_eq!(target.status, b'S');

        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Fire,
                1,
                2,
                0x40,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::FireDamage { raw_damage: 21 }
        );
        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Energy,
                1,
                2,
                0x40,
                &mut target,
                19,
                20,
            ),
            CombatArenaFieldContactOutcome::EnergyDamage { raw_damage: 0 }
        );
    }

    #[test]
    fn combat_field_contact_handles_dead_party_and_non_party_status_edges() {
        let mut dead_target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'D',
            climb_stat: 0,
            mana: 0,
            hp: 0,
            max_hp: 20,
            level: 1,
        };

        assert_eq!(
            resolve_combat_arena_field_contact_for_party_target(
                CombatArenaFieldKind::Sleep,
                1,
                2,
                0x40,
                &mut dead_target,
                0,
                0,
            ),
            CombatArenaFieldContactOutcome::SleepSkippedDeadParty
        );
        assert_eq!(dead_target.status, b'D');

        assert_eq!(
            resolve_combat_arena_field_contact_for_non_party_target(
                CombatArenaFieldKind::Sleep,
                1,
                2,
                0x40,
                0,
                0,
            ),
            CombatArenaFieldContactOutcome::SleepDisabledNonParty
        );
        assert_eq!(
            resolve_combat_arena_field_contact_for_non_party_target(
                CombatArenaFieldKind::Poison,
                1,
                2,
                0x40,
                20,
                0,
            ),
            CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage: 1 }
        );
        assert_eq!(
            resolve_combat_arena_field_contact_for_non_party_target(
                CombatArenaFieldKind::Poison,
                1,
                2,
                0x80,
                20,
                0,
            ),
            CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass
        );
    }

    #[test]
    fn combat_sleep_status_helper_sleeps_living_party_members_only() {
        let mut target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'P',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };

        assert_eq!(
            apply_combat_sleep_to_party_target(&mut target),
            CombatPartySleepOutcome::SleptPartyMember {
                status_before: b'P',
                status_after: b'S',
            }
        );
        assert_eq!(target.status, b'S');

        let mut dead = PartyMember {
            status: b'D',
            hp: 0,
            ..target
        };
        assert_eq!(
            apply_combat_sleep_to_party_target(&mut dead),
            CombatPartySleepOutcome::SkippedDeadParty
        );
        assert_eq!(dead.status, b'D');

        let mut zero_hp = PartyMember {
            status: b'G',
            hp: 0,
            ..target
        };
        assert_eq!(
            apply_combat_sleep_to_party_target(&mut zero_hp),
            CombatPartySleepOutcome::SkippedDeadParty
        );
        assert_eq!(zero_hp.status, b'G');
    }

    #[test]
    fn combat_poison_status_helper_poisons_good_living_party_else_damage() {
        let mut target = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };

        assert_eq!(
            apply_combat_poison_to_party_target(&mut target, 20),
            CombatPartyPoisonOutcome::PoisonedPartyMember {
                status_before: b'G',
                status_after: b'P',
            }
        );
        assert_eq!(target.status, b'P');
        assert_eq!(target.hp, 12);

        assert_eq!(
            apply_combat_poison_to_party_target(&mut target, 20),
            CombatPartyPoisonOutcome::FallbackDamage { raw_damage: 1 }
        );
        assert_eq!(target.status, b'P');
        assert_eq!(target.hp, 12);

        let mut zero_hp = PartyMember {
            status: b'G',
            hp: 0,
            ..target
        };
        assert_eq!(
            apply_combat_poison_to_party_target(&mut zero_hp, 19),
            CombatPartyPoisonOutcome::FallbackDamage { raw_damage: 20 }
        );
        assert_eq!(zero_hp.status, b'G');
    }

    #[test]
    fn post_combat_active_party_slot_restores_only_conscious_survivors() {
        let party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'S',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: 1,
                status: b'D',
                climb_stat: 0,
                mana: 0,
                hp: 0,
                max_hp: 20,
                level: 1,
            },
        ];

        assert_eq!(
            resolve_post_combat_active_party_slot(Some(0), &party),
            Some(0)
        );
        assert_eq!(resolve_post_combat_active_party_slot(Some(1), &party), None);
        assert_eq!(resolve_post_combat_active_party_slot(Some(2), &party), None);
        assert_eq!(resolve_post_combat_active_party_slot(Some(3), &party), None);
        assert_eq!(resolve_post_combat_active_party_slot(None, &party), None);
    }

    #[test]
    fn combat_class_for_sprite_byte_maps_published_sprite_runs() {
        assert_eq!(combat_class_for_sprite_byte(0x80), Some(16));
        assert_eq!(combat_class_for_sprite_byte(0x83), Some(16));
        assert_eq!(combat_class_for_sprite_byte(0xc0), Some(32));
        assert_eq!(
            combat_class_stats_for_sprite_byte(0xf8).map(|stats| stats.name),
            Some("Rot Worm")
        );
        assert_eq!(combat_class_for_sprite_byte(0xe8), None);
        assert_eq!(combat_class_for_sprite_byte(0xef), None);
        assert_eq!(combat_class_for_sprite_byte(0x2c), None);
    }

    #[test]
    fn combat_actor_descriptor_preserves_published_row_order() {
        let descriptor = CombatActorDescriptor::from_row([10, 20, 0x80, 32, 7, 4, 5, 6]);

        assert_eq!(descriptor.hp_or_wound, 10);
        assert_eq!(descriptor.base_step, 20);
        assert_eq!(descriptor.flags, 0x80);
        assert_eq!(descriptor.owner_target_class, 32);
        assert_eq!(descriptor.active_object_slot, 7);
        assert_eq!(descriptor.phase_counter, 4);
        assert_eq!(descriptor.x, 5);
        assert_eq!(descriptor.y, 6);
        assert_eq!(descriptor.raw_row(), [10, 20, 0x80, 32, 7, 4, 5, 6]);
        assert_eq!(COMBAT_ACTOR_SLOTS, 32);
        assert_eq!(COMBAT_PARTY_ACTOR_SLOTS, 6);
        assert_eq!(COMBAT_ACTOR_RECORD_LEN, 8);
        assert_eq!(COMBAT_ACTOR_FLAG_CONTROLLED, 0x01);
        assert_eq!(COMBAT_ACTOR_FLAG_TEAM_TOGGLE, COMBAT_ACTOR_FLAG_CONTROLLED);
        assert_eq!(COMBAT_ACTOR_FLAG_FLEEING, 0x02);
        assert_eq!(COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED, 0x04);
        assert_eq!(COMBAT_ACTOR_FLAG_STATUS_DISABLED, 0x08);
        assert_eq!(COMBAT_SLEEP_DURATION_SLOTS, 0x40);
        assert!(COMBAT_SLEEP_DISABLED_DURATION_DEFAULT > 0);
    }

    #[test]
    fn combat_actor_monster_placement_uses_class_hp_speed_and_linkage() {
        let stats = combat_class_stats(39).unwrap();
        let descriptor = CombatActorDescriptor::for_monster_placement(
            stats,
            12,
            3,
            4,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            9,
        );

        assert_eq!(descriptor.hp_or_wound, stats.max_hp);
        assert_eq!(descriptor.base_step, stats.speed_seed);
        assert_eq!(descriptor.owner_target_class, stats.class);
        assert_eq!(descriptor.active_object_slot, 12);
        assert_eq!(descriptor.phase_counter, 9);
        assert_eq!((descriptor.x, descriptor.y), (3, 4));
        assert!(descriptor.has_field_lookup_selectable_bit());
    }

    #[test]
    fn combat_actor_field_lookup_filter_matches_published_bits() {
        let live =
            CombatActorDescriptor::from_row([1, 2, COMBAT_ACTOR_FLAG_SELECTABLE_40, 3, 4, 5, 6, 7]);
        assert!(live.eligible_for_field_coordinate_lookup(0xc0));

        let dead = CombatActorDescriptor::from_row([
            1,
            2,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            3,
            4,
            5,
            6,
            7,
        ]);
        assert!(!dead.eligible_for_field_coordinate_lookup(0xc0));

        let hidden = CombatActorDescriptor::from_row([
            1,
            2,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            3,
            4,
            5,
            6,
            7,
        ]);
        assert!(!hidden.eligible_for_field_coordinate_lookup(0xc0));

        let unselectable = CombatActorDescriptor::from_row([1, 2, 0, 3, 4, 5, 6, 7]);
        assert!(!unselectable.eligible_for_field_coordinate_lookup(0xc0));
        assert!(!live.eligible_for_field_coordinate_lookup(
            COMBAT_FIELD_REJECTED_ACTIVE_OBJECT_TILE
        ));
    }

    #[test]
    fn combat_actor_field_coordinate_lookup_scans_slots_and_linked_objects() {
        let mut descriptors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        descriptors[0] =
            CombatActorDescriptor::from_row([1, 2, COMBAT_ACTOR_FLAG_MARKED_DEAD, 3, 0, 5, 4, 4]);
        descriptors[1] = CombatActorDescriptor::from_row([
            1,
            2,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            3,
            1,
            5,
            4,
            4,
        ]);
        descriptors[2] = CombatActorDescriptor::from_row([
            1,
            2,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            3,
            2,
            5,
            4,
            4,
        ]);
        descriptors[3] = CombatActorDescriptor::from_row([
            1,
            2,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            3,
            3,
            5,
            4,
            4,
        ]);
        descriptors[4] = CombatActorDescriptor::from_row([
            1,
            2,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            3,
            9,
            5,
            4,
            4,
        ]);

        let mut active_objects = vec![ActiveObject::empty(); 4];
        active_objects[0].tile = 0xc0;
        active_objects[1].tile = 0xc0;
        active_objects[2].tile = COMBAT_FIELD_REJECTED_ACTIVE_OBJECT_TILE;
        active_objects[3].tile = 0xc0;

        assert_eq!(
            find_combat_actor_at_field_coordinate(&descriptors, &active_objects, 4, 4),
            Some(3)
        );
        assert_eq!(
            find_combat_actor_at_field_coordinate_skipping(
                &descriptors,
                &active_objects,
                4,
                4,
                Some(3)
            ),
            None
        );
        assert_eq!(
            find_combat_actor_at_field_coordinate(&descriptors, &active_objects, 5, 4),
            None
        );
    }

    #[test]
    fn combat_actor_clear_and_mark_dead_mutate_only_descriptor_state() {
        let mut descriptor =
            CombatActorDescriptor::from_row([10, 20, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 7, 4, 5, 6]);

        descriptor.mark_dead();
        assert!(descriptor.is_marked_dead());
        assert_eq!(descriptor.owner_target_class, 32);

        descriptor.clear();
        assert_eq!(descriptor, CombatActorDescriptor::empty());
        assert!(descriptor.is_empty());
    }

    #[test]
    fn combat_invisibility_marks_live_actor_hidden_only_once() {
        let mut actor =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 7, 0, 4, 5]);

        assert!(apply_combat_invisibility(&mut actor));
        assert!(actor.is_hidden_or_unrevealed());
        assert_eq!(
            actor.flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED
        );

        assert!(!apply_combat_invisibility(&mut actor));
        assert_eq!(
            actor.flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED
        );
    }

    #[test]
    fn combat_invisibility_ignores_empty_and_dead_actor_rows() {
        let mut empty = CombatActorDescriptor::empty();
        assert!(!apply_combat_invisibility(&mut empty));
        assert_eq!(empty, CombatActorDescriptor::empty());

        let mut dead =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 32, 7, 0, 4, 5]);
        assert!(!apply_combat_invisibility(&mut dead));
        assert_eq!(dead.flags, COMBAT_ACTOR_FLAG_MARKED_DEAD);
    }

    #[test]
    fn combat_invisibility_can_be_cleared_from_actor_row() {
        let mut actor = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            0,
            0,
            0,
            4,
            4,
        ]);

        assert!(clear_combat_invisibility(&mut actor));
        assert!(!actor.is_hidden_or_unrevealed());
        assert!(!clear_combat_invisibility(&mut actor));
    }

    #[test]
    fn linked_combat_invisibility_updates_actor_flag_and_visual_tile() {
        let mut actor =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 1, 0, 4, 5]);
        let mut active_objects = vec![ActiveObject::empty(); 2];
        active_objects[1] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc2,
            x: 4,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let hidden =
            apply_combat_linked_invisibility(&mut actor, &mut active_objects).unwrap();

        assert_eq!(hidden.visibility, CombatLinkedVisibility::Hidden);
        assert!(hidden.changed());
        assert_eq!(hidden.visual_tile_before, Some(0xc2));
        assert_eq!(
            hidden.visual_tile_after,
            Some(COMBAT_HIDDEN_ACTIVE_OBJECT_TILE)
        );
        assert!(actor.is_hidden_or_unrevealed());
        assert_eq!(active_objects[1].tile, COMBAT_HIDDEN_ACTIVE_OBJECT_TILE);

        let visible =
            clear_combat_linked_invisibility(&mut actor, &mut active_objects).unwrap();

        assert_eq!(visible.visibility, CombatLinkedVisibility::Visible);
        assert!(visible.changed());
        assert_eq!(
            visible.visual_tile_before,
            Some(COMBAT_HIDDEN_ACTIVE_OBJECT_TILE)
        );
        assert_eq!(visible.visual_tile_after, Some(0xc0));
        assert!(!actor.is_hidden_or_unrevealed());
        assert_eq!(active_objects[1].tile, 0xc0);
    }

    #[test]
    fn combat_blink_phase_toggles_same_linked_visibility_state() {
        let mut actor =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 23, 1, 0, 4, 5]);
        let mut active_objects = vec![ActiveObject::empty(); 2];
        active_objects[1] = ActiveObject {
            type_byte: 0x9c,
            tile: 0x9d,
            x: 4,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let disappeared = toggle_combat_blink_phase(&mut actor, &mut active_objects).unwrap();
        assert_eq!(disappeared.visibility, CombatLinkedVisibility::Hidden);
        assert!(actor.is_hidden_or_unrevealed());
        assert_eq!(active_objects[1].tile, COMBAT_HIDDEN_ACTIVE_OBJECT_TILE);

        let returned = toggle_combat_blink_phase(&mut actor, &mut active_objects).unwrap();
        assert_eq!(returned.visibility, CombatLinkedVisibility::Visible);
        assert!(!actor.is_hidden_or_unrevealed());
        assert_eq!(active_objects[1].tile, 0x9c);
    }

    #[test]
    fn combat_ai_blink_special_toggles_linked_visibility_and_marks_dirty() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[actor_slot] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            23,
            actor_slot as u8,
            0,
            4,
            5,
        ]);
        state.active_objects[actor_slot] = ActiveObject {
            type_byte: 0x9c,
            tile: 0x9d,
            x: 4,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.visibility_dirty = false;

        let application = state.apply_combat_ai_blink_special(actor_slot).unwrap();

        assert!(matches!(
            application,
            CombatAiSpecialApplication::Blink {
                actor_slot: slot,
                visibility: CombatLinkedVisibilityOutcome {
                    visibility: CombatLinkedVisibility::Hidden,
                    ..
                },
            } if slot == actor_slot
        ));
        assert!(state.combat_actors[actor_slot].is_hidden_or_unrevealed());
        assert_eq!(
            state.active_objects[actor_slot].tile,
            COMBAT_HIDDEN_ACTIVE_OBJECT_TILE
        );
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Monster vanishes.");
    }

    #[test]
    fn combat_reveal_clears_live_hidden_actor_flags_only() {
        let mut actors = [
            CombatActorDescriptor::from_row([
                10,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
                32,
                7,
                0,
                4,
                5,
            ]),
            CombatActorDescriptor::empty(),
            CombatActorDescriptor::from_row([
                10,
                1,
                COMBAT_ACTOR_FLAG_MARKED_DEAD | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
                32,
                7,
                0,
                4,
                5,
            ]),
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 7, 0, 4, 5]),
        ];

        assert_eq!(apply_combat_reveal(&mut actors), 1);
        assert_eq!(actors[0].flags, COMBAT_ACTOR_FLAG_SELECTABLE_80);
        assert_eq!(actors[1], CombatActorDescriptor::empty());
        assert!(actors[2].is_hidden_or_unrevealed());
        assert_eq!(actors[3].flags, COMBAT_ACTOR_FLAG_SELECTABLE_40);
    }

    #[test]
    fn combat_actor_range_uses_truncated_euclidean_arena_distance() {
        assert_eq!(combat_arena_range(0, 0, 0, 0), 0);
        assert_eq!(combat_arena_range(0, 0, 3, 4), 5);
        assert_eq!(combat_arena_range(4, 4, 1, 0), 5);
        assert_eq!(combat_arena_range(0, 0, 2, 2), 2);
        assert_eq!(combat_arena_range(10, 10, 0, 0), 14);

        let first = CombatActorDescriptor::from_row([1, 2, 3, 4, 5, 6, 1, 1]);
        let second = CombatActorDescriptor::from_row([1, 2, 3, 4, 5, 6, 6, 4]);
        assert_eq!(first.range_to(second), 5);
        assert_eq!(second.range_to(first), 5);
    }

    #[test]
    fn combat_hit_roll_uses_strict_public_score_comparison() {
        assert_eq!(combat_to_hit_score(30, 10), 25);
        assert!(resolve_combat_hit(30, 10, 24));
        assert!(!resolve_combat_hit(30, 10, 25));

        assert_eq!(combat_to_hit_score(0, 99), -34);
        assert!(!resolve_combat_hit(0, 99, 0));

        assert_eq!(combat_to_hit_score(255, 0), 142);
        assert!(resolve_combat_hit(255, 0, 141));
        assert!(!resolve_combat_hit(255, 0, 142));
    }

    #[test]
    fn combat_step_destination_uses_published_direction_codes_and_arena_bounds() {
        assert_eq!(combat_direction_code_for_direction(Direction::West), Some(1));
        assert_eq!(combat_direction_code_for_direction(Direction::East), Some(2));
        assert_eq!(combat_direction_code_for_direction(Direction::North), Some(3));
        assert_eq!(combat_direction_code_for_direction(Direction::South), Some(4));
        assert_eq!(
            combat_direction_code_for_direction(Direction::NorthWest),
            None
        );
        assert_eq!(
            combat_direction_code_for_direction(Direction::SouthEast),
            None
        );

        assert_eq!(
            combat_direction_code_step(1),
            CombatStepVector { dx: -1, dy: 0 }
        );
        assert_eq!(
            combat_direction_code_step(2),
            CombatStepVector { dx: 1, dy: 0 }
        );
        assert_eq!(
            combat_direction_code_step(3),
            CombatStepVector { dx: 0, dy: -1 }
        );
        assert_eq!(
            combat_direction_code_step(4),
            CombatStepVector { dx: 0, dy: 1 }
        );
        assert_eq!(
            combat_direction_code_step(0),
            CombatStepVector { dx: 0, dy: 0 }
        );
        assert_eq!(
            combat_direction_code_step(99),
            CombatStepVector { dx: 0, dy: 0 }
        );

        assert_eq!(
            resolve_combat_step_destination(5, 5, 1),
            CombatStepDestination {
                dx: -1,
                dy: 0,
                x: 4,
                y: 5,
                in_bounds: true,
            }
        );
        assert_eq!(
            resolve_combat_step_destination(0, 0, 1),
            CombatStepDestination {
                dx: -1,
                dy: 0,
                x: -1,
                y: 0,
                in_bounds: false,
            }
        );
        assert_eq!(
            resolve_combat_step_destination(10, 10, 4),
            CombatStepDestination {
                dx: 0,
                dy: 1,
                x: 10,
                y: 11,
                in_bounds: false,
            }
        );
        assert!(combat_arena_coordinate_in_bounds(10, 10));
        assert!(!combat_arena_coordinate_in_bounds(11, 10));
    }

    #[test]
    fn combat_step_or_attack_inner_pass_classifies_move_attack_and_blocks() {
        let mut candidates =
            [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];
        candidates[6] = combat_target_view(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                COMBAT_CLASS_GIANT_RAT,
                6,
                0,
                6,
                5,
            ]),
            COMBAT_TARGET_GROUP_MONSTER,
        );
        candidates[7] = combat_target_view(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
                7,
                0,
                5,
                4,
            ]),
            COMBAT_TARGET_GROUP_PARTY,
        );

        assert!(combat_target_groups_are_hostile(
            COMBAT_TARGET_GROUP_PARTY,
            COMBAT_TARGET_GROUP_MONSTER
        ));
        assert!(!combat_target_groups_are_hostile(
            COMBAT_TARGET_GROUP_PARTY,
            COMBAT_TARGET_GROUP_PARTY
        ));
        assert_eq!(
            resolve_combat_step_or_attack_inner_pass(
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                resolve_combat_step_destination(5, 5, 2),
                true,
            ),
            CombatStepOrAttackOutcome::Attack { target_slot: 6 }
        );
        assert_eq!(
            resolve_combat_step_or_attack_inner_pass(
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                resolve_combat_step_destination(5, 5, 3),
                true,
            ),
            CombatStepOrAttackOutcome::BlockedActor { target_slot: 7 }
        );
        assert_eq!(
            resolve_combat_step_or_attack_inner_pass(
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                resolve_combat_step_destination(5, 5, 4),
                true,
            ),
            CombatStepOrAttackOutcome::Move { x: 5, y: 6 }
        );
        assert_eq!(
            resolve_combat_step_or_attack_inner_pass(
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                resolve_combat_step_destination(5, 5, 1),
                false,
            ),
            CombatStepOrAttackOutcome::BlockedWall
        );
        assert_eq!(
            resolve_combat_step_or_attack_inner_pass(
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                resolve_combat_step_destination(0, 0, 1),
                true,
            ),
            CombatStepOrAttackOutcome::OutOfArena { x: -1, y: 0 }
        );
    }

    #[test]
    fn combat_step_or_attack_inner_pass_ignores_suppressed_hidden_or_dead_occupants() {
        let mut candidates =
            [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];
        let live = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_BAT,
            6,
            0,
            6,
            5,
        ]);
        candidates[6] = CombatTargetCandidateView {
            suppressed: true,
            ..combat_target_view(live, COMBAT_TARGET_GROUP_MONSTER)
        };
        candidates[7] = CombatTargetCandidateView {
            invisible_or_unrevealed: true,
            ..combat_target_view(live, COMBAT_TARGET_GROUP_MONSTER)
        };
        candidates[8] = combat_target_view(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
                COMBAT_CLASS_PYTHON,
                8,
                0,
                6,
                5,
            ]),
            COMBAT_TARGET_GROUP_MONSTER,
        );

        assert!(!combat_step_or_attack_occupant_is_active(candidates[6]));
        assert!(!combat_step_or_attack_occupant_is_active(candidates[7]));
        assert!(!combat_step_or_attack_occupant_is_active(candidates[8]));
        assert_eq!(
            resolve_combat_step_or_attack_inner_pass(
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                resolve_combat_step_destination(5, 5, 2),
                true,
            ),
            CombatStepOrAttackOutcome::Move { x: 6, y: 5 }
        );
    }

    #[test]
    fn combat_step_or_attack_primitive_commits_only_empty_walkable_movement() {
        let mut actor = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            2,
            0,
            5,
            5,
        ]);
        let mut active_objects = vec![ActiveObject::empty(); 4];
        active_objects[2] = ActiveObject {
            type_byte: 0x5c,
            tile: 0x5c,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let candidates = [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];

        let outcome = resolve_combat_step_or_attack_primitive(
            &mut actor,
            &mut active_objects,
            &candidates,
            0,
            COMBAT_TARGET_GROUP_PARTY,
            2,
            true,
        );

        assert!(outcome.committed_movement());
        assert_eq!(
            outcome,
            CombatStepOrAttackPrimitiveOutcome::Moved {
                commit: CombatLinkedPositionCommitOutcome {
                    active_object_slot: 2,
                    actor_position_before: (5, 5),
                    actor_position_after: (6, 5),
                    active_object_position_before: Some((5, 5)),
                    active_object_position_after: Some((6, 5)),
                },
            }
        );
        assert_eq!((actor.x, actor.y), (6, 5));
        assert_eq!((active_objects[2].x, active_objects[2].y), (6, 5));
    }

    #[test]
    fn combat_step_or_attack_primitive_reports_attack_block_and_escape_without_committing() {
        let actor_template = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            0,
            5,
            5,
        ]);
        let mut candidates =
            [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];
        candidates[6] = combat_target_view(
            CombatActorDescriptor::from_row([
                20,
                7,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                COMBAT_CLASS_GIANT_RAT,
                6,
                0,
                6,
                5,
            ]),
            COMBAT_TARGET_GROUP_MONSTER,
        );
        candidates[2] = combat_target_view(
            CombatActorDescriptor::from_row([
                20,
                7,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
                2,
                0,
                5,
                4,
            ]),
            COMBAT_TARGET_GROUP_PARTY,
        );

        let mut objects = vec![ActiveObject::empty(); 2];
        objects[1].x = 5;
        objects[1].y = 5;

        let mut attack_actor = actor_template;
        assert_eq!(
            resolve_combat_step_or_attack_primitive(
                &mut attack_actor,
                &mut objects,
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                2,
                false,
            ),
            CombatStepOrAttackPrimitiveOutcome::Attack { target_slot: 6 }
        );
        assert_eq!((attack_actor.x, attack_actor.y), (5, 5));
        assert_eq!((objects[1].x, objects[1].y), (5, 5));

        let mut blocked_actor = actor_template;
        let blocked = resolve_combat_step_or_attack_primitive(
            &mut blocked_actor,
            &mut objects,
            &candidates,
            0,
            COMBAT_TARGET_GROUP_PARTY,
            3,
            true,
        );
        assert_eq!(
            blocked,
            CombatStepOrAttackPrimitiveOutcome::BlockedActor { target_slot: 2 }
        );
        assert!(blocked.blocked());
        assert_eq!((blocked_actor.x, blocked_actor.y), (5, 5));

        let mut wall_actor = actor_template;
        assert_eq!(
            resolve_combat_step_or_attack_primitive(
                &mut wall_actor,
                &mut objects,
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                1,
                false,
            ),
            CombatStepOrAttackPrimitiveOutcome::BlockedWall
        );
        assert_eq!((wall_actor.x, wall_actor.y), (5, 5));

        let mut edge_actor = CombatActorDescriptor {
            x: 0,
            y: 0,
            ..actor_template
        };
        assert_eq!(
            resolve_combat_step_or_attack_primitive(
                &mut edge_actor,
                &mut objects,
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                1,
                true,
            ),
            CombatStepOrAttackPrimitiveOutcome::OutOfArena { x: -1, y: 0 }
        );
        assert_eq!((edge_actor.x, edge_actor.y), (0, 0));

        let mut inactive = CombatActorDescriptor {
            flags: COMBAT_ACTOR_FLAG_MARKED_DEAD,
            ..actor_template
        };
        assert_eq!(
            resolve_combat_step_or_attack_primitive(
                &mut inactive,
                &mut objects,
                &candidates,
                0,
                COMBAT_TARGET_GROUP_PARTY,
                2,
                true,
            ),
            CombatStepOrAttackPrimitiveOutcome::InactiveActor
        );
    }

    #[test]
    fn combat_ai_synthesized_command_key_uses_attack_or_cardinal_direction_bytes() {
        assert_eq!(combat_direction_code_ai_command_key(1), Some('W'));
        assert_eq!(combat_direction_code_ai_command_key(2), Some('E'));
        assert_eq!(combat_direction_code_ai_command_key(3), Some('N'));
        assert_eq!(combat_direction_code_ai_command_key(4), Some('S'));
        assert_eq!(combat_direction_code_ai_command_key(0), None);
        assert_eq!(combat_direction_code_ai_command_key(5), None);

        assert_eq!(
            resolve_combat_ai_synthesized_command_key(Some(1), Some(2)),
            Some(COMBAT_AI_ATTACK_COMMAND_KEY)
        );
        assert_eq!(
            resolve_combat_ai_synthesized_command_key(Some(0), Some(2)),
            Some('E')
        );
        assert_eq!(
            resolve_combat_ai_synthesized_command_key(Some(2), Some(3)),
            Some('N')
        );
        assert_eq!(
            resolve_combat_ai_synthesized_command_key(None, Some(4)),
            Some('S')
        );
        assert_eq!(
            resolve_combat_ai_synthesized_command_key(Some(3), None),
            None
        );
        assert_eq!(
            resolve_combat_ai_synthesized_command_key(Some(1), None),
            Some('A')
        );
    }

    #[test]
    fn combat_out_of_arena_leave_resolves_refusals_direction_lock_and_presentation() {
        assert!(combat_direction_code_is_cardinal(1));
        assert!(combat_direction_code_is_cardinal(4));
        assert!(!combat_direction_code_is_cardinal(0));
        assert!(!combat_direction_code_is_cardinal(5));

        assert_eq!(
            resolve_combat_out_of_arena_leave(true, 1, false, false, None, true),
            CombatOutOfArenaLeaveOutcome::InArena
        );
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 0, false, false, None, true),
            CombatOutOfArenaLeaveOutcome::NotCardinalMove
        );
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 1, true, false, None, true),
            CombatOutOfArenaLeaveOutcome::RefusedShipStyle
        );
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 2, false, true, Some(1), true),
            CombatOutOfArenaLeaveOutcome::RefusedConstrainedDirection {
                required_direction_code: 1,
                attempted_direction_code: 2,
            }
        );

        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 1, false, true, None, true),
            CombatOutOfArenaLeaveOutcome::Accepted {
                direction_code: 1,
                presentation: CombatOutOfArenaLeavePresentation::EscapeWithFoes,
                established_direction_code: Some(1),
            }
        );
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 1, false, true, Some(1), false),
            CombatOutOfArenaLeaveOutcome::Accepted {
                direction_code: 1,
                presentation: CombatOutOfArenaLeavePresentation::OrdinaryCleanup,
                established_direction_code: Some(1),
            }
        );
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 4, false, false, None, false),
            CombatOutOfArenaLeaveOutcome::Accepted {
                direction_code: 4,
                presentation: CombatOutOfArenaLeavePresentation::OrdinaryCleanup,
                established_direction_code: None,
            }
        );
    }

    #[test]
    fn spell_damage_rolls_wrap_on_published_ranges() {
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::MagicMissile, 0),
            1
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::MagicMissile, 15),
            16
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::MagicMissile, 16),
            1
        );

        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Fireball, 0),
            1
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Fireball, 29),
            30
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Fireball, 30),
            1
        );

        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Tremor, 0),
            1
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Tremor, 19),
            20
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Tremor, 20),
            1
        );

        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::FlameWind, 29),
            30
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::FlameWind, 30),
            1
        );
    }

    #[test]
    fn equipment_attack_tables_match_public_item_catalog_rows() {
        assert_eq!(equipment_attack_max(3), Some(4));
        assert_eq!(equipment_attack_max(16), Some(6));
        assert_eq!(equipment_attack_max(34), Some(30));
        assert_eq!(equipment_attack_max(35), Some(99));
        assert_eq!(equipment_attack_max(39), Some(99));
        assert_eq!(equipment_attack_max(40), Some(1));
        assert_eq!(equipment_attack_max(42), Some(0));
        assert_eq!(equipment_attack_max(EQUIPMENT_COUNT), None);

        assert_eq!(equipment_weapon_range_cap(16), Some(3));
        assert_eq!(equipment_weapon_range_cap(17), Some(4));
        assert_eq!(equipment_weapon_range_cap(26), Some(7));
        assert_eq!(equipment_weapon_range_cap(36), Some(15));
        assert_eq!(equipment_weapon_range_cap(38), Some(15));
        assert_eq!(equipment_weapon_range_cap(40), Some(0));
        assert_eq!(equipment_weapon_range_cap(EQUIPMENT_COUNT), None);

        assert_eq!(equipment_weapon_effect_code(17), Some(7));
        assert_eq!(equipment_weapon_effect_code(19), Some(3));
        assert_eq!(equipment_weapon_effect_code(22), Some(2));
        assert_eq!(equipment_weapon_effect_code(38), Some(2));
        assert_eq!(equipment_weapon_effect_code(36), Some(0));
        assert_eq!(equipment_weapon_effect_code(EQUIPMENT_COUNT), None);
    }

    #[test]
    fn weapon_damage_route_resolves_none_fixed_roll_and_special_rows() {
        assert_eq!(
            resolve_combat_weapon_raw_damage(0, 200),
            CombatWeaponDamageRoute::NoOrdinaryDamage
        );
        assert_eq!(
            resolve_combat_weapon_raw_damage(1, 200),
            CombatWeaponDamageRoute::Damage { raw_damage: 1 }
        );
        assert_eq!(
            resolve_combat_weapon_raw_damage(6, 0),
            CombatWeaponDamageRoute::Damage { raw_damage: 1 }
        );
        assert_eq!(
            resolve_combat_weapon_raw_damage(6, 5),
            CombatWeaponDamageRoute::Damage { raw_damage: 6 }
        );
        assert_eq!(
            resolve_combat_weapon_raw_damage(6, 6),
            CombatWeaponDamageRoute::Damage { raw_damage: 1 }
        );
        assert_eq!(
            resolve_combat_weapon_raw_damage(99, 0),
            CombatWeaponDamageRoute::Special
        );

        assert_eq!(
            resolve_combat_equipment_weapon_raw_damage(16, 5),
            Some(CombatWeaponDamageRoute::Damage { raw_damage: 6 })
        );
        assert_eq!(
            resolve_combat_equipment_weapon_raw_damage(35, 0),
            Some(CombatWeaponDamageRoute::Special)
        );
        assert_eq!(
            resolve_combat_equipment_weapon_raw_damage(EQUIPMENT_COUNT, 0),
            None
        );
    }

    #[test]
    fn weapon_attack_resolver_applies_melee_and_ranged_range_gates() {
        assert_eq!(
            resolve_combat_weapon_attack_range_route(1, 0, 7),
            Some(CombatWeaponAttackRangeRoute::Melee)
        );
        assert_eq!(
            resolve_combat_weapon_attack_range_route(4, 4, 7),
            Some(CombatWeaponAttackRangeRoute::Ranged { effect_code: 7 })
        );
        assert_eq!(resolve_combat_weapon_attack_range_route(5, 4, 7), None);
        assert_eq!(resolve_combat_weapon_attack_range_route(2, 0, 7), None);

        assert_eq!(
            resolve_combat_equipment_weapon_attack(18, 1, 30, 10, 24, 7, None),
            Some(CombatWeaponAttackResolution::Hit {
                route: CombatWeaponAttackRangeRoute::Melee,
                raw_damage: 8,
            })
        );
        assert_eq!(
            resolve_combat_equipment_weapon_attack(17, 4, 30, 10, 24, 5, None),
            Some(CombatWeaponAttackResolution::Hit {
                route: CombatWeaponAttackRangeRoute::Ranged { effect_code: 7 },
                raw_damage: 6,
            })
        );
        assert_eq!(
            resolve_combat_equipment_weapon_attack(17, 5, 30, 10, 24, 5, None),
            Some(CombatWeaponAttackResolution::OutOfRange {
                target_range: 5,
                range_cap: 4,
            })
        );
        assert_eq!(
            resolve_combat_equipment_weapon_attack(18, 2, 30, 10, 24, 7, None),
            Some(CombatWeaponAttackResolution::OutOfRange {
                target_range: 2,
                range_cap: 0,
            })
        );
    }

    #[test]
    fn weapon_attack_resolver_tracks_miss_forced_hit_and_non_damage_routes() {
        assert_eq!(
            resolve_combat_weapon_attack(6, 1, 0, 0, 30, 10, 25, 5, None),
            CombatWeaponAttackResolution::Miss {
                route: CombatWeaponAttackRangeRoute::Melee,
                hit_score: 25,
            }
        );
        assert_eq!(
            resolve_combat_weapon_attack(6, 1, 0, 0, 30, 10, 25, 5, Some(true)),
            CombatWeaponAttackResolution::Hit {
                route: CombatWeaponAttackRangeRoute::Melee,
                raw_damage: 6,
            }
        );
        assert_eq!(
            resolve_combat_weapon_attack(6, 1, 0, 0, 30, 10, 0, 5, Some(false)),
            CombatWeaponAttackResolution::Miss {
                route: CombatWeaponAttackRangeRoute::Melee,
                hit_score: 25,
            }
        );
        assert_eq!(
            resolve_combat_weapon_attack(0, 1, 0, 0, 30, 10, 0, 5, None),
            CombatWeaponAttackResolution::NoOrdinaryDamage {
                route: CombatWeaponAttackRangeRoute::Melee,
            }
        );
        assert_eq!(
            resolve_combat_weapon_attack(99, 1, 0, 0, 30, 10, 0, 5, None),
            CombatWeaponAttackResolution::Special {
                route: CombatWeaponAttackRangeRoute::Melee,
            }
        );
        assert_eq!(
            resolve_combat_equipment_weapon_attack(EQUIPMENT_COUNT, 1, 30, 10, 0, 5, None),
            None
        );
    }

    #[test]
    fn spell_damage_defense_subtraction_preserves_kill_sentinel() {
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::Kill, 42),
            COMBAT_INSTANT_KILL_DAMAGE
        );
        assert_eq!(
            resolve_combat_spell_raw_damage(CombatSpellDamageKind::DeathWind, 42),
            COMBAT_INSTANT_KILL_DAMAGE
        );
        assert_eq!(
            resolve_spell_damage_after_defense(COMBAT_INSTANT_KILL_DAMAGE, 255),
            COMBAT_INSTANT_KILL_DAMAGE
        );

        assert_eq!(resolve_spell_damage_after_defense(10, 3), 7);
        assert_eq!(resolve_spell_damage_after_defense(3, 5), -2);
    }

    #[test]
    fn active_target_spell_damage_applies_defense_only_to_projectile_wrappers() {
        assert_eq!(
            resolve_active_target_spell_damage(CombatSpellDamageKind::MagicMissile, 15, 3),
            Some(13)
        );
        assert_eq!(
            resolve_active_target_spell_damage(CombatSpellDamageKind::Fireball, 29, 7),
            Some(23)
        );
        assert_eq!(
            resolve_active_target_spell_damage(CombatSpellDamageKind::MagicMissile, 0, 5),
            Some(-4)
        );
        assert_eq!(
            resolve_active_target_spell_damage(CombatSpellDamageKind::Kill, 42, 255),
            Some(COMBAT_INSTANT_KILL_DAMAGE)
        );
        assert_eq!(
            resolve_active_target_spell_damage(CombatSpellDamageKind::Tremor, 19, 0),
            None
        );
        assert_eq!(
            resolve_active_target_spell_damage(CombatSpellDamageKind::FlameWind, 29, 0),
            None
        );
    }

    #[test]
    fn directed_spell_actor_scan_is_table_ordered_and_non_factional() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([20, 1, 0, 0, 0, 0, 3, 3]);
        actors[4] = CombatActorDescriptor::from_row([20, 1, 0, 32, 0, 0, 5, 5]);
        actors[5] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            33,
            0,
            0,
            5,
            5,
        ]);
        actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            34,
            0,
            0,
            5,
            5,
        ]);
        actors[9] = CombatActorDescriptor::from_row([20, 1, 0, 12, 0, 0, 5, 5]);

        assert!(directed_spell_actor_is_eligible(actors[0]));
        assert!(!directed_spell_actor_is_eligible(actors[5]));
        assert!(!directed_spell_actor_is_eligible(actors[7]));

        let slots = collect_directed_spell_actor_slots(&actors, &[(5, 5), (5, 5), (3, 3)]);

        assert_eq!(slots, vec![0, 4, 9]);
    }

    #[test]
    fn combat_xit_cleanup_requires_no_active_not_dead_non_party_actors() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        actors[6] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            32,
            0,
            0,
            5,
            5,
        ]);

        assert!(combat_actor_is_active_not_dead(actors[6]));
        assert!(combat_has_active_not_dead_non_party_actor(&actors));
        assert!(!resolve_combat_xit_cleanup_allowed(&actors));

        actors[6].mark_dead();
        assert!(!combat_has_active_not_dead_non_party_actor(&actors));
        assert!(resolve_combat_xit_cleanup_allowed(&actors));

        actors[7] = CombatActorDescriptor::from_row([10, 1, 0, 32, 0, 0, 5, 6]);
        assert!(!combat_actor_is_active_not_dead(actors[7]));
        assert!(resolve_combat_xit_cleanup_allowed(&actors));
    }

    #[test]
    fn combat_victory_requires_no_active_not_dead_non_party_actors() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);

        assert!(resolve_combat_victory(&actors));

        actors[6] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            32,
            0,
            0,
            5,
            5,
        ]);
        assert!(!resolve_combat_victory(&actors));

        actors[6].mark_dead();
        assert!(resolve_combat_victory(&actors));

        actors[7] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            32,
            0,
            0,
            5,
            6,
        ]);
        assert!(resolve_combat_victory(&actors));
    }

    #[test]
    fn combat_defeat_requires_no_party_slot_that_can_continue() {
        let mut party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'S',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: 1,
                status: b'D',
                climb_stat: 0,
                mana: 0,
                hp: 0,
                max_hp: 20,
                level: 1,
            },
        ];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            1,
            0,
            0,
            4,
            3,
        ]);

        assert!(combat_party_slot_can_continue(0, &actors, &party));
        assert!(!combat_party_slot_can_continue(1, &actors, &party));
        assert!(!combat_party_slot_can_continue(2, &actors, &party));
        assert!(!combat_party_slot_can_continue(
            COMBAT_PARTY_ACTOR_SLOTS,
            &actors,
            &party
        ));
        assert!(!resolve_combat_defeat(&party, &actors));

        actors[0].clear();
        assert!(!combat_party_slot_can_continue(0, &actors, &party));
        assert!(resolve_combat_defeat(&party, &actors));

        party[0].status = b'G';
        party[0].hp = 12;
        actors[0] = CombatActorDescriptor::from_row([20, 1, 0, 0, 0, 0, 3, 3]);
        assert!(resolve_combat_defeat(&party, &actors));
    }

    #[test]
    fn combat_round_loop_control_maps_exit_flags_and_slot_exhaustion() {
        assert_eq!(COMBAT_ROUND_RESULT_DEFEAT, 0);
        assert_eq!(COMBAT_ROUND_RESULT_SUCCESS, 1);

        let defeat = resolve_combat_round_loop_control(true, false, false);
        assert_eq!(defeat, CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat));
        assert_eq!(defeat.result_code(), Some(COMBAT_ROUND_RESULT_DEFEAT));

        let leave = resolve_combat_round_loop_control(false, true, false);
        assert_eq!(
            leave,
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        );
        assert_eq!(leave.result_code(), Some(COMBAT_ROUND_RESULT_SUCCESS));

        assert_eq!(
            resolve_combat_round_loop_control(false, false, true),
            CombatRoundLoopControl::StartNextRound
        );
        assert_eq!(
            resolve_combat_round_loop_control(false, false, false),
            CombatRoundLoopControl::ContinueActorWalk
        );

        let both = resolve_combat_round_loop_control(true, true, true);
        assert_eq!(both, CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat));
        assert_eq!(both.result_code(), Some(COMBAT_ROUND_RESULT_DEFEAT));
    }

    #[test]
    fn combat_round_loop_control_state_wrapper_reads_current_party_and_actor_table() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            8,
            0,
            5,
            5,
        ]);

        assert_eq!(
            state.combat_round_loop_control(false, false),
            CombatRoundLoopControl::ContinueActorWalk
        );
        assert_eq!(
            state.combat_round_loop_control(false, true),
            CombatRoundLoopControl::StartNextRound
        );
        assert_eq!(
            state.combat_round_loop_control(true, true),
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        );

        state.combat_actors[8].clear();
        assert_eq!(
            state.combat_round_loop_control(false, false),
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        );

        state.combat_actors[0].clear();
        assert_eq!(
            state.combat_round_loop_control(true, true),
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
        );
    }

    #[test]
    fn combat_step_or_attack_state_wrapper_commits_movement_and_marks_visibility_dirty() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.visibility_dirty = false;
        state.active_objects[0] = ActiveObject {
            type_byte: 0x5c,
            tile: 0x5c,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);

        let outcome = state.apply_combat_step_or_attack_primitive(
            0,
            COMBAT_TARGET_GROUP_PARTY,
            2,
            true,
        );

        assert!(outcome.committed_movement());
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (6, 5));
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_ambush_reveal_records_consume_trigger_and_stamp_targets() {
        let mut records = [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT];
        records[2] = Some(CombatAmbushRevealRecord::new(6, 5, 0x34, 1, 2, 10, 10));
        let mut terrain = [[0u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

        let application =
            apply_combat_ambush_reveal_records(&mut records, &mut terrain, 6, 5).unwrap();

        assert_eq!(
            application,
            CombatAmbushRevealApplication {
                slot: 2,
                trigger_x: 6,
                trigger_y: 5,
                reveal_tile: 0x34,
                stamped_cells: 2,
            }
        );
        assert_eq!(records[2], None);
        assert_eq!(terrain[2][1], 0x34);
        assert_eq!(terrain[10][10], 0x34);
        assert_eq!(
            apply_combat_ambush_reveal_records(&mut records, &mut terrain, 6, 5),
            None
        );
    }

    #[test]
    fn combat_ambush_reveal_records_treat_out_of_range_targets_as_unused() {
        let mut records = [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT];
        records[0] = Some(CombatAmbushRevealRecord::new(
            3,
            4,
            0x51,
            COMBAT_ARENA_SIDE as u8,
            2,
            8,
            COMBAT_ARENA_SIDE as u8,
        ));
        let mut terrain = [[0u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

        let application =
            apply_combat_ambush_reveal_records(&mut records, &mut terrain, 3, 4).unwrap();

        assert_eq!(application.stamped_cells, 0);
        assert_eq!(records[0], None);
        assert!(terrain.iter().flatten().all(|tile| *tile == 0));
    }

    #[test]
    fn combat_post_step_ambush_reveal_fires_after_committed_movement() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.visibility_dirty = false;
        state.active_objects[0] = ActiveObject {
            type_byte: 0x5c,
            tile: 0x5c,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_ambush_reveals[0] = Some(CombatAmbushRevealRecord::new(
            6,
            5,
            0x44,
            3,
            4,
            COMBAT_ARENA_SIDE as u8,
            COMBAT_ARENA_SIDE as u8,
        ));

        let outcome = state.apply_combat_step_or_attack_primitive(
            0,
            COMBAT_TARGET_GROUP_PARTY,
            2,
            true,
        );

        assert!(outcome.committed_movement());
        assert_eq!(state.combat_ambush_reveals[0], None);
        assert_eq!(state.combat_terrain[4][3], 0x44);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_ambush_reveal_does_not_fire_on_attack_or_blocked_step() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.visibility_dirty = false;
        state.active_objects[0].x = 5;
        state.active_objects[0].y = 5;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            6,
            0,
            6,
            5,
        ]);
        let reveal = CombatAmbushRevealRecord::new(6, 5, 0x44, 3, 4, 4, 4);
        state.combat_ambush_reveals[0] = Some(reveal);

        assert_eq!(
            state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 2, true),
            CombatStepOrAttackPrimitiveOutcome::Attack { target_slot: 6 }
        );
        assert_eq!(state.combat_ambush_reveals[0], Some(reveal));
        assert_eq!(state.combat_terrain[4][3], DEFAULT_COMBAT_ARENA_TERRAIN[4][3]);
        assert!(!state.visibility_dirty);

        state.combat_actors[6].clear();
        assert_eq!(
            state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 1, false),
            CombatStepOrAttackPrimitiveOutcome::BlockedWall
        );
        assert_eq!(state.combat_ambush_reveals[0], Some(reveal));
        assert_eq!(state.combat_terrain[4][3], DEFAULT_COMBAT_ARENA_TERRAIN[4][3]);
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn combat_frame_entry_clears_or_installs_ambush_reveal_records() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_ambush_reveals[0] = Some(CombatAmbushRevealRecord::new(1, 1, 0x44, 2, 2, 3, 3));
        let active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];

        state
            .enter_combat_frame(active_objects.clone(), actors)
            .expect("ordinary combat frame should enter");
        assert!(state.combat_ambush_reveals.iter().all(Option::is_none));
        let snapshot = state.combat_frame_snapshot.take().unwrap();
        state.restore_combat_frame(snapshot);

        let mut reveals = [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT];
        reveals[7] = Some(CombatAmbushRevealRecord::new(4, 4, 0x55, 5, 5, 6, 6));
        state
            .enter_combat_frame_with_terrain_and_reveals(
                active_objects,
                actors,
                DEFAULT_COMBAT_ARENA_TERRAIN,
                reveals,
            )
            .expect("ambush combat frame should enter");
        assert_eq!(state.combat_ambush_reveals, reveals);
        let snapshot = state.combat_frame_snapshot.take().unwrap();
        state.restore_combat_frame(snapshot);
        assert!(state.combat_ambush_reveals.iter().all(Option::is_none));
    }

    #[test]
    fn combat_post_step_absorbable_field_contact_sets_armed_result_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.visibility_dirty = false;
        state.active_player = Some(0);
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[0] = ActiveObject {
            type_byte: 0x5c,
            tile: 0x5c,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[7] = ActiveObject {
            type_byte: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
            tile: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
            x: 6,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_frame_snapshot = Some(CombatFrameSnapshot {
            area: Area::Dungeon {
                scene: DungeonScene::new(DUNGEON_DOOM_SCENE_BYTE).unwrap(),
                level: DOOM_FINAL_ROOM_LEVEL,
            },
            player: state.player,
            active_objects: state.active_objects.clone(),
            active_player: state.active_player,
            combat_terrain: state.combat_terrain,
            dungeon_room_clear_on_success: None,
            enter_endgame_after_successful_combat: false,
            endgame_messages: Some(EndgameMessages {
                records: vec!["Welcome back".to_string()],
            }),
        });

        let outcome = state.apply_combat_step_or_attack_primitive(
            0,
            COMBAT_TARGET_GROUP_PARTY,
            COMBAT_DIRECTION_EAST,
            true,
        );

        assert!(outcome.committed_movement());
        assert_eq!(state.active_player, None);
        assert_eq!(state.message, "Absorbed!");
        assert!(state.visibility_dirty);
        assert_eq!(
            state
                .combat_frame_snapshot
                .as_ref()
                .map(|snapshot| snapshot.enter_endgame_after_successful_combat),
            Some(true)
        );
    }

    #[test]
    fn combat_absorbable_field_contact_without_armed_snapshot_does_not_set_result_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_player = Some(0);
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[7] = ActiveObject {
            type_byte: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
            tile: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_frame_snapshot = Some(CombatFrameSnapshot {
            area: Area::World {
                plane: WorldPlane::Britannia,
            },
            player: state.player,
            active_objects: state.active_objects.clone(),
            active_player: state.active_player,
            combat_terrain: state.combat_terrain,
            dungeon_room_clear_on_success: None,
            enter_endgame_after_successful_combat: false,
            endgame_messages: None,
        });

        let application = state
            .apply_combat_absorbable_field_contact_for_actor_position(0)
            .unwrap();

        assert!(!application.armed_endgame_result);
        assert_eq!(state.active_player, None);
        assert_eq!(
            state
                .combat_frame_snapshot
                .as_ref()
                .map(|snapshot| snapshot.enter_endgame_after_successful_combat),
            Some(false)
        );
    }

    #[test]
    fn combat_step_or_attack_state_wrapper_reports_non_movement_without_dirtying_visibility() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.visibility_dirty = false;
        state.active_objects[0].x = 5;
        state.active_objects[0].y = 5;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            6,
            0,
            6,
            5,
        ]);

        assert_eq!(
            state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 2, true),
            CombatStepOrAttackPrimitiveOutcome::Attack { target_slot: 6 }
        );
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));
        assert!(!state.visibility_dirty);

        assert_eq!(
            state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 1, false),
            CombatStepOrAttackPrimitiveOutcome::BlockedWall
        );
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));
        assert!(!state.visibility_dirty);

        assert_eq!(
            state.apply_combat_step_or_attack_primitive(
                COMBAT_ACTOR_SLOTS,
                COMBAT_TARGET_GROUP_PARTY,
                2,
                true,
            ),
            CombatStepOrAttackPrimitiveOutcome::InactiveActor
        );
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn combat_round_counter_wraps_at_ten_and_only_then_advances_time() {
        assert_eq!(COMBAT_ROUND_COUNTER_WRAP, 10);
        assert_eq!(COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES, 1);

        assert_eq!(
            resolve_combat_round_counter_tick(0),
            CombatRoundCounterTick {
                counter: 1,
                wrapped: false,
                redraw_tiles: false,
                advance_time_minutes: 0,
            }
        );
        assert_eq!(
            resolve_combat_round_counter_tick(8),
            CombatRoundCounterTick {
                counter: 9,
                wrapped: false,
                redraw_tiles: false,
                advance_time_minutes: 0,
            }
        );
        assert_eq!(
            resolve_combat_round_counter_tick(9),
            CombatRoundCounterTick {
                counter: 0,
                wrapped: true,
                redraw_tiles: true,
                advance_time_minutes: 1,
            }
        );
    }

    #[test]
    fn combat_actor_phase_tick_decrements_waiting_slots_and_refreshes_ready_slots() {
        let mut waiting = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            6,
            3,
            5,
            5,
        ]);
        assert_eq!(
            tick_combat_actor_phase_counter(&mut waiting, 30),
            CombatActorPhaseTick::Waiting {
                counter_before: 3,
                counter_after: 2,
            }
        );
        assert_eq!(waiting.phase_counter, 2);

        let mut one_before_ready = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            6,
            1,
            5,
            5,
        ]);
        let one_tick = tick_combat_actor_phase_counter(&mut one_before_ready, 30);
        assert_eq!(
            one_tick,
            CombatActorPhaseTick::Ready {
                counter_before: 1,
                refreshed_counter: 23,
            }
        );
        assert!(one_tick.actor_should_dispatch());
        assert_eq!(one_before_ready.phase_counter, 23);

        let mut already_zero = CombatActorDescriptor::from_row([
            20,
            12,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_PYTHON,
            7,
            0,
            4,
            4,
        ]);
        assert_eq!(
            tick_combat_actor_phase_counter(&mut already_zero, 30),
            CombatActorPhaseTick::Ready {
                counter_before: 0,
                refreshed_counter: 18,
            }
        );
        assert_eq!(already_zero.phase_counter, 18);
    }

    #[test]
    fn combat_actor_phase_tick_skips_inactive_slots_and_saturates_fast_refreshes() {
        let mut dead = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_GIANT_RAT,
            6,
            1,
            5,
            5,
        ]);
        assert_eq!(
            tick_combat_actor_phase_counter(&mut dead, 30),
            CombatActorPhaseTick::Inactive
        );
        assert_eq!(dead.phase_counter, 1);

        let mut very_fast = CombatActorDescriptor::from_row([
            20,
            40,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            8,
            0,
            6,
            6,
        ]);
        assert_eq!(resolve_combat_phase_refresh_counter(40, 30), 0);
        assert_eq!(
            tick_combat_actor_phase_counter(&mut very_fast, 30),
            CombatActorPhaseTick::Ready {
                counter_before: 0,
                refreshed_counter: 0,
            }
        );
        assert_eq!(very_fast.phase_counter, 0);
    }

    #[test]
    fn combat_actor_phase_state_wrapper_waiting_slots_do_not_advance_round_counter() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            6,
            3,
            5,
            5,
        ]);
        state.combat_round_counter = 4;
        state.visibility_dirty = false;

        assert_eq!(
            state.tick_combat_actor_phase_counter(0, 30),
            Some(CombatActorPhaseTick::Waiting {
                counter_before: 3,
                counter_after: 2,
            })
        );
        assert_eq!(state.combat_actors[0].phase_counter, 2);
        assert_eq!(state.combat_round_counter, 4);
        assert!(!state.visibility_dirty);
        assert_eq!(state.tick_combat_actor_phase_counter(COMBAT_ACTOR_SLOTS, 30), None);
    }

    #[test]
    fn combat_actor_phase_state_wrapper_ready_slots_advance_round_counter_and_wrap_redraw() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            7,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            6,
            1,
            5,
            5,
        ]);
        state.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 1;
        state.visibility_dirty = false;

        assert_eq!(
            state.tick_combat_actor_phase_counter(0, 30),
            Some(CombatActorPhaseTick::Ready {
                counter_before: 1,
                refreshed_counter: 23,
            })
        );
        assert_eq!(state.combat_actors[0].phase_counter, 23);
        assert_eq!(state.combat_round_counter, 0);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_pass_and_quit_commands_have_specified_control_flow() {
        assert_eq!(
            resolve_combat_pass_command(),
            CombatPassCommandOutcome {
                moves: false,
                attacks: false,
                ends_turn: true,
            }
        );

        let quit = resolve_combat_quit_command();
        assert_eq!(quit, CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat));
        assert_eq!(quit.result_code(), Some(COMBAT_ROUND_RESULT_DEFEAT));
    }

    #[test]
    fn combat_ctrl_s_and_escape_are_no_turn_top_level_controls() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);

        assert!(state.music_enabled);
        assert_eq!(
            handle_play_key_input(&mut state, PLAY_MUSIC_TOGGLE_KEY, "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(!state.music_enabled);
        assert_eq!(state.turn, 0);
        assert_eq!(state.pending_combat_actor_slot, None);
        assert_eq!(state.message, "Music Off.");
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (10, 10));

        assert_eq!(
            handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert_eq!(state.pending_combat_actor_slot, None);
        assert_eq!(state.message, "Aborted.");
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (10, 10));
    }

    #[test]
    fn combat_active_player_digit_selection_maps_only_published_keys() {
        assert_eq!(
            resolve_combat_active_player_digit('0'),
            CombatActivePlayerSelectionOutcome::Clear
        );
        assert_eq!(
            resolve_combat_active_player_digit('1'),
            CombatActivePlayerSelectionOutcome::SelectPartySlot(0)
        );
        assert_eq!(
            resolve_combat_active_player_digit('6'),
            CombatActivePlayerSelectionOutcome::SelectPartySlot(5)
        );
        assert_eq!(
            resolve_combat_active_player_digit('7'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
        assert_eq!(
            resolve_combat_active_player_digit('A'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
    }

    #[test]
    fn combat_active_player_digit_state_wrapper_updates_only_valid_party_slots() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let mut second = state.party[0];
        second.slot = 1;
        state.party.push(second);

        assert_eq!(
            state.apply_combat_active_player_digit('2'),
            CombatActivePlayerSelectionOutcome::SelectPartySlot(1)
        );
        assert_eq!(state.active_player, Some(1));

        assert_eq!(
            state.apply_combat_active_player_digit('6'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
        assert_eq!(state.active_player, Some(1));
        assert_eq!(
            state.apply_combat_active_player_digit('A'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
        assert_eq!(state.active_player, Some(1));

        assert_eq!(
            state.apply_combat_active_player_digit('0'),
            CombatActivePlayerSelectionOutcome::Clear
        );
        assert_eq!(state.active_player, None);
    }

    #[test]
    fn post_combat_active_player_restore_clears_dead_asleep_or_missing_slot_only() {
        let mut party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 10,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 10,
                level: 1,
            },
        ];

        assert_eq!(resolve_post_combat_active_player_restore(None, &party), None);
        assert_eq!(
            resolve_post_combat_active_player_restore(Some(0), &party),
            Some(0)
        );
        assert_eq!(
            resolve_post_combat_active_player_restore(Some(1), &party),
            Some(1)
        );
        assert_eq!(resolve_post_combat_active_player_restore(Some(2), &party), None);

        party[0].status = b'D';
        party[1].status = b'S';
        assert_eq!(resolve_post_combat_active_player_restore(Some(0), &party), None);
        assert_eq!(resolve_post_combat_active_player_restore(Some(1), &party), None);
    }

    #[test]
    fn active_player_slot_codec_preserves_valid_saved_slots_only() {
        assert_eq!(decode_active_player_slot(0xff, 6), None);
        assert_eq!(decode_active_player_slot(0, 6), Some(0));
        assert_eq!(decode_active_player_slot(5, 6), Some(5));
        assert_eq!(decode_active_player_slot(6, 6), None);
        assert_eq!(decode_active_player_slot(1, 1), None);
        assert_eq!(encode_active_player_slot(None), 0xff);
        assert_eq!(encode_active_player_slot(Some(2)), 2);
        assert_eq!(encode_active_player_slot(Some(6)), 0xff);
    }

    #[test]
    fn combat_frame_restores_world_table_player_and_surviving_active_player() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let mut second = state.party[0];
        second.slot = 1;
        state.party.push(second);
        state.active_player = Some(1);
        state.combat_terrain[0][0] = 0x77;
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 10,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 1,
            aux3: 2,
        };
        state.active_objects[15] = ActiveObject {
            type_byte: 0xa0,
            tile: 0xa1,
            x: 33,
            y: 44,
            z: WorldPlane::Underworld.save_floor(),
            phase: 7,
            aux1: 8,
            aux3: 9,
        };
        let original_player = state.player;
        let original_objects = state.active_objects.clone();

        let mut combat_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        combat_objects[6] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 5,
            y: 6,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let mut combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        combat_actors[6] = CombatActorDescriptor::for_monster_placement(
            combat_class_stats_for_sprite_byte(0xc0).unwrap(),
            6,
            5,
            6,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        let mut combat_terrain = DEFAULT_COMBAT_ARENA_TERRAIN;
        combat_terrain[5][5] = 0x0c;
        combat_terrain[6][4] = 0x04;
        combat_terrain[6][5] = 0x04;

        let snapshot = state
            .enter_combat_frame_with_terrain(combat_objects.clone(), combat_actors, combat_terrain)
            .unwrap();
        assert!(state.combat_active);
        assert_eq!(state.combat_frame_snapshot, Some(snapshot.clone()));
        assert_eq!(state.active_objects, combat_objects);
        assert_eq!(state.combat_actors[6], combat_actors[6]);
        assert_eq!(state.combat_terrain, combat_terrain);
        let legal = state.combat_legal_cell_mask();
        assert!(!legal[5][5]);
        assert!(legal[6][4]);
        assert!(!legal[6][5]);

        state.player.x = 3;
        state.player.y = 4;
        state.party[1].status = b'G';
        state.restore_combat_frame(snapshot);

        assert!(!state.combat_active);
        assert_eq!(state.combat_frame_snapshot, None);
        assert_eq!(state.player, original_player);
        assert_eq!(state.active_objects, original_objects);
        assert_eq!(state.active_player, Some(1));
        assert_eq!(
            state.combat_actors,
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]
        );
        assert_eq!(state.combat_terrain[0][0], 0x77);
        assert!(state.visibility_dirty);
    }

    fn viewport_palette_at_cell(viewport: &TileViewport, cell_x: usize, cell_y: usize) -> u8 {
        viewport.pixels[cell_y * TILE_ATLAS_SIDE * viewport.width + cell_x * TILE_ATLAS_SIDE]
    }

    #[test]
    fn combat_raster_renders_arena_terrain_and_visible_actor_sprites() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[0][0] = 0x0c;
        state.combat_terrain[5][5] = 0x05;
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[6] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[12] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 7,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[13] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 8,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            0,
            6,
            0,
            6,
            5,
        ]);
        state.combat_actors[9] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            0,
            12,
            0,
            7,
            5,
        ]);
        state.combat_actors[10] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            13,
            0,
            8,
            5,
        ]);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let viewport = state.render_top_down_frame(5, &atlas).unwrap().unwrap();

        assert_eq!(viewport.cells_wide, COMBAT_ARENA_SIDE);
        assert_eq!(viewport.cells_high, COMBAT_ARENA_SIDE);
        assert_eq!(
            viewport_palette_at_cell(&viewport, 0, 0),
            0x0c % atlas.depth.pixel_limit()
        );
        assert_eq!(
            viewport_palette_at_cell(&viewport, 5, 5),
            (PLAYER_SPRITE_TILE as u8) % atlas.depth.pixel_limit()
        );
        assert_eq!(
            viewport_palette_at_cell(&viewport, 6, 5),
            0x04 % atlas.depth.pixel_limit(),
            "hidden combat actor must not overwrite terrain"
        );
        assert_eq!(
            viewport_palette_at_cell(&viewport, 7, 5),
            0x04 % atlas.depth.pixel_limit(),
            "hidden combat actor must be found through active_object_slot, not slot index"
        );
        assert_eq!(
            viewport_palette_at_cell(&viewport, 8, 5),
            0xc0 % atlas.depth.pixel_limit(),
            "visible combat actor with a non-parallel active object slot still renders"
        );
    }

    #[test]
    fn combat_viewport_reports_visible_animated_terrain() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        assert!(!state.viewport_has_animated_tiles(5));

        state.combat_terrain[3][4] = 0x01;
        assert!(state.viewport_has_animated_tiles(5));
    }

    #[test]
    fn combat_frame_restore_clears_dead_or_asleep_saved_active_player() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let mut second = state.party[0];
        second.slot = 1;
        state.party.push(second);
        state.active_player = Some(1);
        let snapshot = state
            .enter_combat_frame(
                vec![ActiveObject::empty(); OOL_SLOTS],
                [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            )
            .unwrap();

        state.party[1].status = b'S';
        state.restore_combat_frame(snapshot);

        assert_eq!(state.active_player, None);
    }

    #[test]
    fn combat_round_loop_exit_restores_stored_frame_snapshot() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let original_player = state.player;
        let original_objects = state.active_objects.clone();
        let mut combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state
            .enter_combat_frame(vec![ActiveObject::empty(); OOL_SLOTS], combat_actors)
            .unwrap();
        state.player.x = 1;
        state.active_objects[0] = ActiveObject {
            type_byte: 0x99,
            tile: 0x99,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };

        let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);

        assert_eq!(
            application,
            CombatRoundLoopExitApplication {
                exit: CombatRoundLoopExit::LeaveCombat,
                result_code: COMBAT_ROUND_RESULT_SUCCESS,
                restored_snapshot: true,
            }
        );
        assert!(!state.combat_active);
        assert_eq!(state.combat_frame_snapshot, None);
        assert_eq!(state.player, original_player);
        assert_eq!(state.active_objects, original_objects);
        assert_eq!(
            state.combat_actors,
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]
        );
    }

    #[test]
    fn terrain_combat_round_exit_reconciles_original_trigger_slot_after_restore() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.active_objects[2] = ActiveObject {
            type_byte: 0x44,
            tile: 0xc0,
            x: 11,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x07,
            aux1: 0x55,
            aux3: 0xaa,
        };
        state
            .enter_combat_frame(
                vec![ActiveObject::empty(); OOL_SLOTS],
                [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            )
            .unwrap();
        state.pending_combat_terrain_trigger_slot = Some(2);

        let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);

        assert_eq!(application.result_code, COMBAT_ROUND_RESULT_SUCCESS);
        assert!(!state.combat_active);
        assert_eq!(state.pending_combat_terrain_trigger_slot, None);
        assert_eq!(
            state.active_objects[2],
            ActiveObject {
                type_byte: 0,
                tile: 0,
                x: 0,
                y: 0,
                z: 0,
                phase: 0x07,
                aux1: 0x55,
                aux3: 0xaa,
            }
        );
    }

    #[test]
    fn terrain_combat_victory_rewrites_water_trigger_slot_after_restore() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.active_objects[2] = ActiveObject {
            type_byte: 0x2c,
            tile: 0x2f,
            x: 11,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x07,
            aux1: 0x55,
            aux3: 0xaa,
        };
        state
            .enter_combat_frame(
                vec![ActiveObject::empty(); OOL_SLOTS],
                [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
            )
            .unwrap();
        state.pending_combat_terrain_trigger_slot = Some(2);

        let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);

        assert_eq!(application.result_code, COMBAT_ROUND_RESULT_SUCCESS);
        assert_eq!(
            state.active_objects[2],
            ActiveObject {
                type_byte: 0x24,
                tile: 0x27,
                x: 11,
                y: 20,
                z: WorldPlane::Underworld.save_floor(),
                phase: 0x07,
                aux1: WATER_CREATURE_BODY_AUX1,
                aux3: WATER_CREATURE_BODY_AUX3,
            }
        );
    }

    #[test]
    fn terrain_combat_escape_with_living_foes_clears_water_trigger_slot() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.active_objects[2] = ActiveObject {
            type_byte: 0x2f,
            tile: 0x2d,
            x: 11,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x07,
            aux1: 0x55,
            aux3: 0xaa,
        };
        let mut combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            32,
            0,
            0,
            5,
            5,
        ]);
        state
            .enter_combat_frame(vec![ActiveObject::empty(); OOL_SLOTS], combat_actors)
            .unwrap();
        state.pending_combat_terrain_trigger_slot = Some(2);

        let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);

        assert_eq!(application.result_code, COMBAT_ROUND_RESULT_SUCCESS);
        assert_eq!(
            state.active_objects[2],
            ActiveObject {
                type_byte: 0,
                tile: 0,
                x: 0,
                y: 0,
                z: 0,
                phase: 0x07,
                aux1: 0x55,
                aux3: 0xaa,
            }
        );
    }

    #[test]
    fn combat_round_loop_exit_without_snapshot_clears_combat_state() {
        let mut state = combat_ai_turn_state(8, 5);
        state.pending_combat_actor_slot = Some(0);
        state.next_combat_actor_slot = 7;

        let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::Defeat);

        assert_eq!(
            application,
            CombatRoundLoopExitApplication {
                exit: CombatRoundLoopExit::Defeat,
                result_code: COMBAT_ROUND_RESULT_DEFEAT,
                restored_snapshot: false,
            }
        );
        assert!(!state.combat_active);
        assert_eq!(state.pending_combat_actor_slot, None);
        assert_eq!(state.next_combat_actor_slot, 0);
        assert_eq!(
            state.combat_actors,
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_exit_body_retrieval_state_requires_success_with_no_living_foes() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        assert!(combat_exit_requests_body_retrieval_reconcile(
            CombatRoundLoopExit::LeaveCombat,
            &actors
        ));
        assert!(!combat_exit_requests_body_retrieval_reconcile(
            CombatRoundLoopExit::Defeat,
            &actors
        ));

        actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            32,
            0,
            0,
            5,
            5,
        ]);
        assert!(!combat_exit_requests_body_retrieval_reconcile(
            CombatRoundLoopExit::LeaveCombat,
            &actors
        ));
    }

    #[test]
    fn post_combat_terrain_trigger_reconciler_clears_bytes_zero_through_four_only() {
        let mut objects = vec![ActiveObject::empty(); OOL_SLOTS];
        objects[4] = ActiveObject {
            type_byte: 0x44,
            tile: 0xc0,
            x: 88,
            y: 99,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x0a,
            aux1: 0x55,
            aux3: 0xaa,
        };

        let outcome = reconcile_post_combat_terrain_trigger_slot(&mut objects, 4, false);

        assert_eq!(outcome, PostCombatTriggerReconcile::Cleared);
        assert_eq!(
            objects[4],
            ActiveObject {
                type_byte: 0,
                tile: 0,
                x: 0,
                y: 0,
                z: 0,
                phase: 0x0a,
                aux1: 0x55,
                aux3: 0xaa,
            }
        );
    }

    #[test]
    fn post_combat_terrain_trigger_reconciler_rewrites_body_family_when_exit_state_requests_it() {
        let mut objects = vec![ActiveObject::empty(); OOL_SLOTS];
        objects[9] = ActiveObject {
            type_byte: 0x2f,
            tile: 0x2d,
            x: 17,
            y: 18,
            z: 0,
            phase: 0x07,
            aux1: 0x11,
            aux3: 0x22,
        };

        let outcome = reconcile_post_combat_terrain_trigger_slot(&mut objects, 9, true);

        assert_eq!(outcome, PostCombatTriggerReconcile::BodyRetrieval);
        assert_eq!(
            objects[9],
            ActiveObject {
                type_byte: 0x27,
                tile: 0x25,
                x: 17,
                y: 18,
                z: 0,
                phase: 0x07,
                aux1: 0x63,
                aux3: 0x02,
            }
        );
    }

    #[test]
    fn post_combat_terrain_trigger_reconciler_marks_state_dirty_from_play_state() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.active_objects[3] = ActiveObject {
            type_byte: 0x44,
            tile: 0xc0,
            x: 11,
            y: 12,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x09,
            aux1: 0x01,
            aux3: 0x02,
        };
        state.visibility_dirty = false;

        let outcome = state.reconcile_post_combat_terrain_trigger_slot(3, false);

        assert_eq!(outcome, PostCombatTriggerReconcile::Cleared);
        assert!(state.visibility_dirty);
        assert_eq!(
            state.reconcile_post_combat_terrain_trigger_slot(OOL_SLOTS + 1, false),
            PostCombatTriggerReconcile::MissingSlot
        );
    }

    #[test]
    fn combat_command_branch_classifier_matches_published_dispatch_map() {
        assert_eq!(resolve_combat_command_branch('A'), CombatCommandBranch::Attack);
        assert_eq!(resolve_combat_command_branch('a'), CombatCommandBranch::Attack);
        assert_eq!(
            resolve_combat_command_branch('B'),
            CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Board)
        );
        assert_eq!(resolve_combat_command_branch('C'), CombatCommandBranch::CastSpell);
        assert_eq!(
            resolve_combat_command_branch('D'),
            CombatCommandBranch::DWhatRefusal
        );
        assert_eq!(resolve_combat_command_branch('G'), CombatCommandBranch::Get);
        assert_eq!(resolve_combat_command_branch('J'), CombatCommandBranch::Jimmy);
        assert_eq!(resolve_combat_command_branch('K'), CombatCommandBranch::Klimb);
        assert_eq!(
            resolve_combat_command_branch('M'),
            CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Mix)
        );
        assert_eq!(resolve_combat_command_branch('O'), CombatCommandBranch::Open);
        assert_eq!(resolve_combat_command_branch('P'), CombatCommandBranch::Push);
        assert_eq!(
            resolve_combat_command_branch('Q'),
            CombatCommandBranch::QuitDefeat
        );
        assert_eq!(resolve_combat_command_branch('R'), CombatCommandBranch::Ready);
        assert_eq!(resolve_combat_command_branch('S'), CombatCommandBranch::Search);
        assert_eq!(
            resolve_combat_command_branch('U'),
            CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::UseItem)
        );
        assert_eq!(
            resolve_combat_command_branch('W'),
            CombatCommandBranch::WWhatRefusal
        );
        assert_eq!(
            resolve_combat_command_branch('X'),
            CombatCommandBranch::XitCleanup
        );
        assert_eq!(resolve_combat_command_branch('Y'), CombatCommandBranch::Yell);
        assert_eq!(resolve_combat_command_branch('Z'), CombatCommandBranch::ZStats);
        assert_eq!(resolve_combat_command_branch(' '), CombatCommandBranch::Pass);
        assert_eq!(
            resolve_combat_command_branch('\u{1b}'),
            CombatCommandBranch::AbortPrompt
        );
        assert_eq!(
            resolve_combat_command_branch('\u{13}'),
            CombatCommandBranch::ToggleMusic
        );
        assert_eq!(
            resolve_combat_command_branch('7'),
            CombatCommandBranch::Invalid
        );
    }

    #[test]
    fn combat_command_live_actor_gate_applies_only_to_specified_branches() {
        for branch in [
            CombatCommandBranch::Get,
            CombatCommandBranch::Jimmy,
            CombatCommandBranch::Open,
            CombatCommandBranch::Ready,
            CombatCommandBranch::Search,
        ] {
            assert!(combat_command_branch_requires_live_active_actor(branch));
        }

        for branch in [
            CombatCommandBranch::Attack,
            CombatCommandBranch::CastSpell,
            CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::UseItem),
            CombatCommandBranch::DWhatRefusal,
            CombatCommandBranch::Klimb,
            CombatCommandBranch::Push,
            CombatCommandBranch::QuitDefeat,
            CombatCommandBranch::WWhatRefusal,
            CombatCommandBranch::XitCleanup,
            CombatCommandBranch::Yell,
            CombatCommandBranch::ZStats,
            CombatCommandBranch::Pass,
            CombatCommandBranch::AbortPrompt,
            CombatCommandBranch::ToggleMusic,
            CombatCommandBranch::Invalid,
        ] {
            assert!(!combat_command_branch_requires_live_active_actor(branch));
        }
    }

    #[test]
    fn combat_command_live_actor_gate_rejects_missing_empty_and_dead_actors() {
        let live = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            32,
            7,
            0,
            4,
            5,
        ]);
        let dead = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            32,
            7,
            0,
            4,
            5,
        ]);
        let unselectable = CombatActorDescriptor::from_row([10, 1, 0, 32, 7, 0, 4, 5]);

        assert_eq!(
            resolve_combat_command_live_actor_gate(CombatCommandBranch::Get, Some(live)),
            CombatCommandLiveActorGate::Accepted
        );
        assert_eq!(
            resolve_combat_command_live_actor_gate(CombatCommandBranch::Get, Some(dead)),
            CombatCommandLiveActorGate::RejectedDeadOrMissing
        );
        assert_eq!(
            resolve_combat_command_live_actor_gate(CombatCommandBranch::Get, Some(unselectable)),
            CombatCommandLiveActorGate::RejectedDeadOrMissing
        );
        assert_eq!(
            resolve_combat_command_live_actor_gate(CombatCommandBranch::Get, None),
            CombatCommandLiveActorGate::RejectedDeadOrMissing
        );

        assert_eq!(
            resolve_combat_command_live_actor_gate(CombatCommandBranch::Push, None),
            CombatCommandLiveActorGate::NotRequired
        );
        assert_eq!(
            resolve_combat_command_live_actor_gate(CombatCommandBranch::XitCleanup, Some(dead)),
            CombatCommandLiveActorGate::NotRequired
        );
    }

    #[test]
    fn combat_command_branch_labels_include_only_exactly_published_strings() {
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::DWhatRefusal),
            Some("D-What?")
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Get),
            Some("Get-")
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Jimmy),
            Some("Jimmy-")
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Open),
            Some("Open-")
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Push),
            Some("Push-")
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Search),
            Some("Search-")
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::WWhatRefusal),
            Some("W-What?")
        );

        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Ready),
            None
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::Yell),
            None
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::QuitDefeat),
            None
        );
        assert_eq!(
            combat_command_branch_published_label(CombatCommandBranch::SceneMessageAbort(
                CombatSceneAbortVerb::UseItem
            )),
            None
        );

        assert_eq!(
            combat_scene_abort_verb_prefix(CombatSceneAbortVerb::UseItem),
            "Use"
        );
        assert_eq!(
            combat_scene_abort_verb_prefix(CombatSceneAbortVerb::HoleUp),
            "Hole up"
        );
    }

    #[test]
    fn combat_command_branch_named_multistage_matches_published_list() {
        for branch in [
            CombatCommandBranch::Attack,
            CombatCommandBranch::CastSpell,
            CombatCommandBranch::Get,
            CombatCommandBranch::Jimmy,
            CombatCommandBranch::Klimb,
            CombatCommandBranch::Open,
            CombatCommandBranch::Ready,
            CombatCommandBranch::Search,
            CombatCommandBranch::Yell,
        ] {
            assert!(combat_command_branch_is_named_multistage(branch));
        }

        for branch in [
            CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::UseItem),
            CombatCommandBranch::DWhatRefusal,
            CombatCommandBranch::Push,
            CombatCommandBranch::QuitDefeat,
            CombatCommandBranch::WWhatRefusal,
            CombatCommandBranch::XitCleanup,
            CombatCommandBranch::ZStats,
            CombatCommandBranch::Pass,
            CombatCommandBranch::AbortPrompt,
            CombatCommandBranch::ToggleMusic,
            CombatCommandBranch::Invalid,
        ] {
            assert!(!combat_command_branch_is_named_multistage(branch));
        }
    }

    #[test]
    fn combat_yell_uses_only_prompt_nothing_said_or_no_effect_paths() {
        assert_eq!(
            resolve_combat_yell_command(None),
            CombatYellCommandOutcome::PromptForInput
        );
        assert_eq!(
            resolve_combat_yell_command(Some("")),
            CombatYellCommandOutcome::NothingSaid
        );
        assert_eq!(
            resolve_combat_yell_command(Some("   ")),
            CombatYellCommandOutcome::NothingSaid
        );

        assert_eq!(
            resolve_combat_yell_command(Some("hello")),
            CombatYellCommandOutcome::NoEffect
        );
        assert_eq!(
            resolve_combat_yell_command(Some("FALLAX")),
            CombatYellCommandOutcome::NoEffect
        );
        assert_eq!(
            resolve_combat_yell_command(Some("FAULINEI")),
            CombatYellCommandOutcome::NoEffect
        );
    }

    #[test]
    fn combat_cast_interference_requires_live_visible_awake_adjacent_target_without_negate_time() {
        let caster =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 0, 0, 5, 5]);
        let adjacent =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 2, 1, 0, 6, 5]);
        let far =
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 2, 1, 0, 7, 5]);
        let hidden = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            2,
            1,
            0,
            6,
            5,
        ]);
        let dead = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            2,
            1,
            0,
            6,
            5,
        ]);
        let unselectable = CombatActorDescriptor::from_row([10, 1, 0, 2, 1, 0, 6, 5]);

        assert!(combat_cast_interference_target_is_live_visible(adjacent));
        assert!(!combat_cast_interference_target_is_live_visible(hidden));
        assert!(!combat_cast_interference_target_is_live_visible(dead));
        assert!(!combat_cast_interference_target_is_live_visible(unselectable));

        assert_eq!(
            resolve_combat_cast_interference(caster, Some(adjacent), true, false),
            CombatCastInterferenceOutcome::Interfered
        );
        assert_eq!(
            resolve_combat_cast_interference(caster, Some(adjacent), false, false),
            CombatCastInterferenceOutcome::ContinueToSpellDispatcher
        );
        assert_eq!(
            resolve_combat_cast_interference(caster, Some(adjacent), true, true),
            CombatCastInterferenceOutcome::ContinueToSpellDispatcher
        );
        assert_eq!(
            resolve_combat_cast_interference(caster, Some(far), true, false),
            CombatCastInterferenceOutcome::ContinueToSpellDispatcher
        );
        assert_eq!(
            resolve_combat_cast_interference(caster, Some(hidden), true, false),
            CombatCastInterferenceOutcome::ContinueToSpellDispatcher
        );
        assert_eq!(
            resolve_combat_cast_interference(caster, None, true, false),
            CombatCastInterferenceOutcome::ContinueToSpellDispatcher
        );
    }

    #[test]
    fn directed_spell_damage_semantics_match_wind_family() {
        assert_eq!(
            resolve_directed_spell_raw_damage(CombatDirectedSpellEffect::Sleep, 29),
            None
        );
        assert_eq!(
            resolve_directed_spell_raw_damage(CombatDirectedSpellEffect::PoisonWind, 29),
            None
        );
        assert_eq!(
            resolve_directed_spell_raw_damage(CombatDirectedSpellEffect::DeathWind, 29),
            Some(COMBAT_INSTANT_KILL_DAMAGE)
        );
        assert_eq!(
            resolve_directed_spell_raw_damage(CombatDirectedSpellEffect::FlameWind, 29),
            Some(30)
        );
        assert_eq!(
            resolve_directed_spell_raw_damage(CombatDirectedSpellEffect::FlameWind, 30),
            Some(1)
        );

        assert!(!directed_spell_damage_credits_caster(
            CombatDirectedSpellEffect::Sleep
        ));
        assert!(!directed_spell_damage_credits_caster(
            CombatDirectedSpellEffect::PoisonWind
        ));
        assert!(directed_spell_damage_credits_caster(
            CombatDirectedSpellEffect::DeathWind
        ));
        assert!(directed_spell_damage_credits_caster(
            CombatDirectedSpellEffect::FlameWind
        ));
    }

    #[test]
    fn directed_sleep_uses_shared_target_walk_cells() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            32,
            COMBAT_PARTY_ACTOR_SLOTS as u8,
            0,
            8,
            5,
        ]);

        let sleep_cells = state
            .directed_combat_spell_target_cells(
                0,
                COMBAT_PARTY_ACTOR_SLOTS,
                CombatDirectedSpellEffect::Sleep,
            )
            .unwrap();
        let poison_cells = state
            .directed_combat_spell_target_cells(
                0,
                COMBAT_PARTY_ACTOR_SLOTS,
                CombatDirectedSpellEffect::PoisonWind,
            )
            .unwrap();

        assert_eq!(sleep_cells, poison_cells);
        assert!(sleep_cells.len() > 1);
        assert!(sleep_cells.contains(&(8, 5)));
    }

    #[test]
    fn combat_spell_handler_family_maps_published_combat_spell_ids() {
        let family = |code: &str| resolve_combat_spell_handler_family(spell_index_from_code(code).unwrap());

        assert_eq!(
            family("GP"),
            Some(CombatSpellHandlerFamily::ActiveTargetAttack(
                CombatSpellDamageKind::MagicMissile
            ))
        );
        assert_eq!(
            family("FV"),
            Some(CombatSpellHandlerFamily::ActiveTargetAttack(
                CombatSpellDamageKind::Fireball
            ))
        );
        assert_eq!(
            family("CX"),
            Some(CombatSpellHandlerFamily::ActiveTargetAttack(
                CombatSpellDamageKind::Kill
            ))
        );

        assert_eq!(
            family("FGI"),
            Some(CombatSpellHandlerFamily::FieldPlacement(
                CombatArenaFieldKind::Fire
            ))
        );
        assert_eq!(
            family("GIN"),
            Some(CombatSpellHandlerFamily::FieldPlacement(
                CombatArenaFieldKind::Poison
            ))
        );
        assert_eq!(
            family("GIZ"),
            Some(CombatSpellHandlerFamily::FieldPlacement(
                CombatArenaFieldKind::Sleep
            ))
        );
        assert_eq!(
            family("GIS"),
            Some(CombatSpellHandlerFamily::FieldPlacement(
                CombatArenaFieldKind::Energy
            ))
        );
        assert_eq!(family("AG"), Some(CombatSpellHandlerFamily::FieldRemoval));

        assert_eq!(
            family("IZ"),
            Some(CombatSpellHandlerFamily::DirectedTargetWalk(
                CombatDirectedSpellEffect::Sleep
            ))
        );
        assert_eq!(
            family("HIN"),
            Some(CombatSpellHandlerFamily::DirectedTargetWalk(
                CombatDirectedSpellEffect::PoisonWind
            ))
        );
        assert_eq!(
            family("CGIV"),
            Some(CombatSpellHandlerFamily::DirectedTargetWalk(
                CombatDirectedSpellEffect::DeathWind
            ))
        );
        assert_eq!(
            family("FHI"),
            Some(CombatSpellHandlerFamily::DirectedTargetWalk(
                CombatDirectedSpellEffect::FlameWind
            ))
        );
        assert_eq!(family("IPVY"), Some(CombatSpellHandlerFamily::TableWideTremor));

        assert_eq!(
            family("IS"),
            Some(CombatSpellHandlerFamily::ActiveEffect {
                tag: PROTECTION_ACTIVE_EFFECT_TAG,
                duration: PROTECTION_ACTIVE_EFFECT_DURATION,
            })
        );
        assert_eq!(
            family("RT"),
            Some(CombatSpellHandlerFamily::ActiveEffect {
                tag: QUICKNESS_ACTIVE_EFFECT_TAG,
                duration: QUICKNESS_ACTIVE_EFFECT_DURATION,
            })
        );
        assert_eq!(
            family("AQW"),
            Some(CombatSpellHandlerFamily::ActiveEffect {
                tag: MASS_CHARM_ACTIVE_EFFECT_TAG,
                duration: MASS_CHARM_ACTIVE_EFFECT_DURATION,
            })
        );
        assert_eq!(
            family("AI"),
            Some(CombatSpellHandlerFamily::ActiveEffect {
                tag: NEGATE_MAGIC_ACTIVE_EFFECT_TAG,
                duration: NEGATE_MAGIC_ACTIVE_EFFECT_DURATION,
            })
        );

        assert_eq!(family("KX"), Some(CombatSpellHandlerFamily::ConjureAnimal));
        assert_eq!(family("BIX"), Some(CombatSpellHandlerFamily::Swarm));
        assert_eq!(family("CKX"), Some(CombatSpellHandlerFamily::SummonDaemon));
        assert_eq!(
            family("AEX"),
            Some(CombatSpellHandlerFamily::CreaturePromptTargeter(
                CombatCreaturePromptSpellEffect::Charm
            ))
        );
        assert_eq!(
            family("BRX"),
            Some(CombatSpellHandlerFamily::CreaturePromptTargeter(
                CombatCreaturePromptSpellEffect::Polymorph
            ))
        );
        assert_eq!(
            family("IQX"),
            Some(CombatSpellHandlerFamily::CreaturePromptTargeter(
                CombatCreaturePromptSpellEffect::Clone
            ))
        );
        assert_eq!(
            family("LS"),
            Some(CombatSpellHandlerFamily::ActiveCasterInvisibility)
        );
        assert_eq!(family("CIQ"), Some(CombatSpellHandlerFamily::TableWideFear));

        assert_eq!(family("IL"), None);
        assert_eq!(resolve_combat_spell_handler_family(SPELL_COUNT), None);
    }

    #[test]
    fn tremor_scan_uses_table_order_gate_and_no_faction_filter() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([20, 1, 0, 0, 0, 0, 3, 3]);
        actors[4] = CombatActorDescriptor::from_row([20, 1, 0, 32, 0, 0, 5, 5]);
        actors[5] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            33,
            0,
            0,
            6,
            6,
        ]);
        actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            34,
            0,
            0,
            7,
            7,
        ]);
        actors[9] = CombatActorDescriptor::from_row([20, 1, 0, 12, 0, 0, 8, 8]);
        let mut gate_accepts = [false; COMBAT_ACTOR_SLOTS];
        gate_accepts[0] = true;
        gate_accepts[4] = false;
        gate_accepts[5] = true;
        gate_accepts[7] = true;
        gate_accepts[9] = true;

        assert!(tremor_spell_actor_is_damageable(actors[0]));
        assert!(!tremor_spell_actor_is_damageable(actors[5]));
        assert!(!tremor_spell_actor_is_damageable(actors[7]));

        let slots = collect_tremor_spell_actor_slots(&actors, &gate_accepts);

        assert_eq!(slots, vec![0, 9]);
    }

    #[test]
    fn tremor_damage_and_spell_reward_credit_are_capped() {
        assert_eq!(resolve_tremor_spell_raw_damage(0), 1);
        assert_eq!(resolve_tremor_spell_raw_damage(19), 20);
        assert_eq!(resolve_tremor_spell_raw_damage(20), 1);

        assert_eq!(apply_combat_spell_experience_reward(10, 25), 35);
        assert_eq!(
            apply_combat_spell_experience_reward(COMBAT_EXPERIENCE_CAP - 1, 25),
            COMBAT_EXPERIENCE_CAP
        );
        assert_eq!(
            apply_combat_spell_experience_reward(u16::MAX, 25),
            COMBAT_EXPERIENCE_CAP
        );
    }

    #[test]
    fn cause_fear_scan_skips_dead_same_faction_and_protected_targets() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([20, 1, 0, 0, 0, 0, 3, 3]);
        actors[4] = CombatActorDescriptor::from_row([20, 1, 0, 32, 0, 0, 5, 5]);
        actors[5] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            33,
            0,
            0,
            6,
            6,
        ]);
        actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            34,
            0,
            0,
            7,
            7,
        ]);
        actors[9] = CombatActorDescriptor::from_row([20, 1, 0, 12, 0, 0, 8, 8]);
        let mut groups = [0u8; COMBAT_ACTOR_SLOTS];
        groups[0] = 1;
        groups[4] = 2;
        groups[5] = 2;
        groups[7] = 2;
        groups[9] = 2;
        let mut protected_or_immune = [false; COMBAT_ACTOR_SLOTS];
        protected_or_immune[9] = true;

        assert!(cause_fear_actor_is_live(actors[0]));
        assert!(!cause_fear_actor_is_live(actors[5]));

        let slots = collect_cause_fear_actor_slots(&actors, &groups, 1, &protected_or_immune);

        assert_eq!(slots, vec![4, 7]);
    }

    #[test]
    fn cause_fear_forced_hp_feeds_under_quarter_morale_bucket() {
        for max_hp in [0, 1, 2, 5, 10, 20, 99] {
            let current_hp = cause_fear_forced_current_hp(max_hp);
            let morale = resolve_combat_wound_morale(current_hp, max_hp, 255);
            assert_eq!(morale.bucket, CombatWoundScoreBucket::UnderOneQuarter);
            assert!(morale.fleeing);
        }
    }

    #[test]
    fn cause_fear_critical_hp_setup_mutates_accepted_live_actor_slots_only() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[2] = CombatActorDescriptor::from_row([20, 1, 0, COMBAT_CLASS_DAEMON, 0, 0, 3, 3]);
        actors[4] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            COMBAT_CLASS_PYTHON,
            0,
            0,
            4,
            4,
        ]);
        actors[6] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_GIANT_RAT,
            0,
            0,
            5,
            5,
        ]);
        actors[8] = CombatActorDescriptor::from_row([20, 1, 0, 99, 0, 0, 6, 6]);

        assert_eq!(
            apply_cause_fear_critical_hp_setup(&mut actors, &[2, 4, 6, 8, 99]),
            2
        );
        assert_eq!(
            actors[2].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().max_hp)
        );
        assert_eq!(
            actors[4].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
        );
        assert_eq!(
            actors[4].flags,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED | COMBAT_ACTOR_FLAG_FLEEING
        );
        assert_eq!(actors[6].hp_or_wound, 20);
        assert_eq!(actors[8].hp_or_wound, 20);
    }

    #[test]
    fn conjure_spell_class_selector_uses_fifteen_weighted_outcomes() {
        for selector in 0..6 {
            assert_eq!(
                resolve_conjure_spell_class(selector),
                COMBAT_CLASS_GIANT_RAT
            );
        }
        for selector in 6..11 {
            assert_eq!(
                resolve_conjure_spell_class(selector),
                COMBAT_CLASS_GIANT_SPIDER
            );
        }
        for selector in 11..14 {
            assert_eq!(resolve_conjure_spell_class(selector), COMBAT_CLASS_BAT);
        }
        assert_eq!(resolve_conjure_spell_class(14), COMBAT_CLASS_PYTHON);
        assert_eq!(resolve_conjure_spell_class(15), COMBAT_CLASS_GIANT_RAT);
        assert_eq!(resolve_conjure_spell_class(29), COMBAT_CLASS_PYTHON);
    }

    #[test]
    fn summon_spell_descriptors_use_published_class_rows_and_coordinates() {
        let conjured = resolve_conjure_spell_descriptor(
            10,
            4,
            5,
            6,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            3,
        )
        .unwrap();
        let spider_stats = combat_class_stats(COMBAT_CLASS_GIANT_SPIDER).unwrap();
        assert_eq!(conjured.owner_target_class, COMBAT_CLASS_GIANT_SPIDER);
        assert_eq!(conjured.hp_or_wound, spider_stats.max_hp);
        assert_eq!(conjured.base_step, spider_stats.speed_seed);
        assert_eq!(conjured.active_object_slot, 4);
        assert_eq!((conjured.x, conjured.y), (5, 6));
        assert_eq!(conjured.flags, COMBAT_ACTOR_FLAG_SELECTABLE_80);
        assert_eq!(conjured.phase_counter, 3);

        let swarm = resolve_swarm_spell_descriptor(5, 7, 8, COMBAT_ACTOR_FLAG_SELECTABLE_40, 2)
            .unwrap();
        let swarm_stats = combat_class_stats(COMBAT_CLASS_INSECT_SWARM).unwrap();
        assert_eq!(swarm.owner_target_class, COMBAT_CLASS_INSECT_SWARM);
        assert_eq!(swarm.hp_or_wound, swarm_stats.max_hp);
        assert_eq!((swarm.x, swarm.y), (7, 8));

        let daemon =
            resolve_summon_daemon_spell_descriptor(6, 9, 10, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1)
                .unwrap();
        let daemon_stats = combat_class_stats(COMBAT_CLASS_DAEMON).unwrap();
        assert_eq!(daemon.owner_target_class, COMBAT_CLASS_DAEMON);
        assert_eq!(daemon.hp_or_wound, daemon_stats.max_hp);
        assert_eq!(daemon.base_step, daemon_stats.speed_seed);
        assert_eq!((daemon.x, daemon.y), (9, 10));
    }

    #[test]
    fn generic_summoned_descriptor_rejects_unknown_combat_class() {
        assert_eq!(
            resolve_summoned_combat_actor_descriptor(
                99,
                4,
                5,
                6,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                3,
            ),
            None
        );
    }

    #[test]
    fn summoned_active_object_records_use_class_sprite_base_and_coordinates() {
        assert_eq!(combat_class_sprite_base(COMBAT_CLASS_GIANT_RAT), Some(0x90));
        assert_eq!(combat_class_sprite_base(COMBAT_CLASS_DAEMON), Some(0xd8));
        assert_eq!(combat_class_sprite_base(44), Some(0xf0));
        assert_eq!(combat_class_sprite_base(42), None);

        let object = summoned_active_object_record(COMBAT_CLASS_DAEMON, 7, 8, -1).unwrap();
        assert_eq!(object.type_byte, 0xd8);
        assert_eq!(object.tile, 0xd8);
        assert_eq!((object.x, object.y, object.z), (7, 8, -1));
        assert_eq!(object.phase, STEADY_PHASE);
        assert_eq!(summoned_active_object_record(42, 7, 8, 0), None);
    }

    #[test]
    fn combat_neighbor_candidate_coordinates_rotate_around_center_and_clip_edges() {
        assert_eq!(
            combat_neighbor_candidate_coordinates(5, 5, 2),
            vec![(6, 4), (4, 5), (6, 5), (4, 6), (5, 6), (6, 6), (4, 4), (5, 4)]
        );
        assert_eq!(
            combat_neighbor_candidate_coordinates(0, 0, 0),
            vec![(1, 0), (0, 1), (1, 1)]
        );
    }

    #[test]
    fn combat_ring_candidate_coordinates_use_fixed_north_clockwise_order() {
        assert_eq!(
            combat_ring_candidate_coordinates(5, 5),
            vec![(5, 4), (6, 4), (6, 5), (6, 6), (5, 6), (4, 6), (4, 5), (4, 4)]
        );
        assert_eq!(
            combat_ring_candidate_coordinates(0, 0),
            vec![(1, 0), (1, 1), (0, 1)]
        );
    }

    #[test]
    fn combat_summon_application_allocates_actor_and_object_on_legal_neighbor() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[4][6] = 0x04;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.visibility_dirty = false;

        let application = state
            .apply_combat_summon_class_around_slot(COMBAT_CLASS_GIANT_SPIDER, 0, 10)
            .unwrap();

        assert_eq!(application.actor_slot, COMBAT_PARTY_ACTOR_SLOTS);
        assert_eq!(application.active_object_slot, COMBAT_PARTY_ACTOR_SLOTS);
        assert_eq!((application.x, application.y), (6, 4));
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS],
            resolve_summoned_combat_actor_descriptor(
                COMBAT_CLASS_GIANT_SPIDER,
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                6,
                4,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS],
            summoned_active_object_record(COMBAT_CLASS_GIANT_SPIDER, 6, 4, -1).unwrap()
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn creature_prompt_target_gate_rejects_empty_disabled_controlled_same_faction_and_protected() {
        let live_target = CombatActorDescriptor::from_row([20, 1, 0, 32, 0, 0, 5, 5]);
        assert!(creature_prompt_target_is_eligible(live_target, 2, 1, false));
        assert!(!creature_prompt_target_is_eligible(
            CombatActorDescriptor::empty(),
            2,
            1,
            false,
        ));
        assert!(!creature_prompt_target_is_eligible(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_MARKED_DEAD,
                32,
                0,
                0,
                5,
                5,
            ]),
            2,
            1,
            false,
        ));
        assert!(!creature_prompt_target_is_eligible(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
                32,
                0,
                0,
                5,
                5,
            ]),
            2,
            1,
            false,
        ));
        assert!(!creature_prompt_target_is_eligible(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_STATUS_DISABLED,
                32,
                0,
                0,
                5,
                5,
            ]),
            2,
            1,
            false,
        ));
        assert!(!creature_prompt_target_is_eligible(
            CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_CONTROLLED,
                32,
                0,
                0,
                5,
                5,
            ]),
            2,
            1,
            false,
        ));
        assert!(!creature_prompt_target_is_eligible(live_target, 1, 1, false));
        assert!(!creature_prompt_target_is_eligible(live_target, 2, 1, true));
    }

    #[test]
    fn charm_toggle_flips_low_team_flag_and_target_group() {
        let mut actor = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            7,
            0,
            5,
            5,
        ]);

        assert!(!actor.team_toggled());
        assert_eq!(
            resolve_combat_target_group_for_actor(actor, COMBAT_PARTY_ACTOR_SLOTS, None),
            COMBAT_TARGET_GROUP_MONSTER
        );
        assert_eq!(
            toggle_combat_charm_allegiance(&mut actor),
            Some((
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
            ))
        );
        assert!(actor.team_toggled());
        assert_eq!(
            resolve_combat_target_group_for_actor(actor, COMBAT_PARTY_ACTOR_SLOTS, None),
            COMBAT_TARGET_GROUP_PARTY
        );
        assert_eq!(
            toggle_combat_charm_allegiance(&mut actor),
            Some((
                COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
            ))
        );
        assert!(!actor.team_toggled());
    }

    #[test]
    fn charm_toggle_rejects_empty_and_dead_actor_records() {
        let mut empty = CombatActorDescriptor::empty();
        assert_eq!(toggle_combat_charm_allegiance(&mut empty), None);

        let mut dead = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_DAEMON,
            7,
            0,
            5,
            5,
        ]);
        assert_eq!(toggle_combat_charm_allegiance(&mut dead), None);
        assert_eq!(
            dead.flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD
        );
    }

    #[test]
    fn charm_application_toggles_target_actor_flag() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            7,
            0,
            5,
            5,
        ]);

        assert_eq!(
            state.apply_combat_charm_allegiance(target_slot),
            Some(CombatCharmApplication {
                target_slot,
                flags_before: COMBAT_ACTOR_FLAG_SELECTABLE_80,
                flags_after: COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
            })
        );
        assert_eq!(
            state.combat_actors[target_slot].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE
        );
    }

    #[test]
    fn polymorph_replaces_target_with_giant_rat_at_same_coordinates() {
        let target = CombatActorDescriptor::from_row([
            33,
            22,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            39,
            7,
            3,
            5,
            6,
        ]);

        let rat = resolve_polymorph_giant_rat_descriptor(target).unwrap();
        let rat_stats = combat_class_stats(20).unwrap();

        assert_eq!(rat.owner_target_class, 20);
        assert_eq!(rat.hp_or_wound, rat_stats.max_hp);
        assert_eq!(rat.base_step, rat_stats.speed_seed);
        assert_eq!(rat.active_object_slot, target.active_object_slot);
        assert_eq!((rat.x, rat.y), (target.x, target.y));
        assert_eq!(rat.flags, target.flags);
        assert_eq!(rat.phase_counter, target.phase_counter);
    }

    #[test]
    fn polymorph_updates_linked_active_object_to_giant_rat_sprite_base() {
        let target_object = ActiveObject {
            type_byte: 0xdc,
            tile: 0xde,
            x: 5,
            y: 6,
            z: 2,
            phase: 0x21,
            aux1: 3,
            aux3: 4,
        };

        let rat_object = polymorph_giant_rat_active_object(target_object, 7, 8);

        assert_eq!(rat_object.type_byte, COMBAT_CLASS_GIANT_RAT_SPRITE_BASE);
        assert_eq!(rat_object.tile, COMBAT_CLASS_GIANT_RAT_SPRITE_BASE);
        assert_eq!((rat_object.x, rat_object.y), (7, 8));
        assert_eq!(rat_object.z, target_object.z);
        assert_eq!(rat_object.phase, target_object.phase);
        assert_eq!(rat_object.aux1, target_object.aux1);
        assert_eq!(rat_object.aux3, target_object.aux3);
    }

    #[test]
    fn clone_spell_allocation_requires_free_actor_and_active_object_slots() {
        let mut actors = [CombatActorDescriptor::from_row([1, 1, 0, 32, 1, 0, 1, 1]);
            COMBAT_ACTOR_SLOTS];
        actors[COMBAT_PARTY_ACTOR_SLOTS + 1] = CombatActorDescriptor::empty();
        let mut active_objects = vec![
            ActiveObject {
                type_byte: 0x80,
                tile: 0x80,
                x: 1,
                y: 1,
                z: 0,
                phase: 0,
                aux1: 0,
                aux3: 0,
            };
            8
        ];
        active_objects.resize(COMBAT_PARTY_ACTOR_SLOTS + 3, active_objects[0]);
        active_objects[COMBAT_PARTY_ACTOR_SLOTS + 2] = ActiveObject::empty();

        assert_eq!(
            resolve_clone_spell_allocation(&actors, &active_objects),
            Some(CombatCloneAllocation {
                actor_slot: COMBAT_PARTY_ACTOR_SLOTS + 1,
                active_object_slot: COMBAT_PARTY_ACTOR_SLOTS + 2,
            })
        );

        actors[COMBAT_PARTY_ACTOR_SLOTS + 1] =
            CombatActorDescriptor::from_row([1, 1, 0, 32, 1, 0, 1, 1]);
        assert_eq!(resolve_clone_spell_allocation(&actors, &active_objects), None);

        actors[COMBAT_PARTY_ACTOR_SLOTS + 1] = CombatActorDescriptor::empty();
        active_objects[COMBAT_PARTY_ACTOR_SLOTS + 2] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 1,
            y: 1,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        assert_eq!(resolve_clone_spell_allocation(&actors, &active_objects), None);
    }

    #[test]
    fn clone_spell_copies_records_relinks_actor_and_moves_to_accepted_cell() {
        let target_actor =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 39, 7, 2, 5, 5]);
        let target_object = ActiveObject {
            type_byte: 0x91,
            tile: 0x91,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x21,
            aux1: 3,
            aux3: 4,
        };

        let cloned_actor = clone_combat_actor_descriptor(target_actor, 12, 8, 9);
        let cloned_object = clone_active_object_record(target_object, 8, 9);

        assert_eq!(cloned_actor.active_object_slot, 12);
        assert_eq!((cloned_actor.x, cloned_actor.y), (8, 9));
        assert_eq!(cloned_actor.owner_target_class, target_actor.owner_target_class);
        assert_eq!(cloned_actor.flags, target_actor.flags);
        assert_eq!(cloned_actor.hp_or_wound, target_actor.hp_or_wound);

        assert_eq!((cloned_object.x, cloned_object.y), (8, 9));
        assert_eq!(cloned_object.type_byte, target_object.type_byte);
        assert_eq!(cloned_object.tile, target_object.tile);
        assert_eq!(cloned_object.phase, target_object.phase);
        assert_eq!(cloned_object.aux1, target_object.aux1);
        assert_eq!(cloned_object.aux3, target_object.aux3);
    }

    #[test]
    fn clone_placement_coordinate_uses_first_legal_candidate() {
        let mut legal = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        legal[8][7] = true;
        legal[2][1] = true;

        assert_eq!(
            resolve_combat_clone_placement_coordinate(&legal, &[(0, 0), (7, 8), (1, 2)]),
            Some((7, 8))
        );
        assert_eq!(
            resolve_combat_clone_placement_coordinate(&legal, &[(9, 9), (10, 10)]),
            None
        );
        assert_eq!(
            resolve_combat_clone_placement_coordinate(&legal, &[(11, 0), (0, 11)]),
            None
        );
    }

    #[test]
    fn clone_candidate_coordinates_cover_arena_from_seeded_start() {
        let candidates = combat_clone_candidate_coordinates(120);

        assert_eq!(candidates.len(), COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE);
        assert_eq!(candidates[0], (10, 10));
        assert_eq!(candidates[1], (0, 0));
        assert_eq!(candidates[120], (9, 10));
    }

    #[test]
    fn clone_application_allocates_paired_slots_and_marks_visibility_dirty() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let target_actor =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 39, 7, 2, 5, 5]);
        state.combat_actors[target_slot] = target_actor;
        state.active_objects[7] = ActiveObject {
            type_byte: 0xdc,
            tile: 0xdd,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x21,
            aux1: 3,
            aux3: 4,
        };
        state.visibility_dirty = false;
        let mut legal = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        legal[8][9] = true;

        let expected_actor_slot = target_slot + 1;
        let expected_object_slot = COMBAT_PARTY_ACTOR_SLOTS;
        assert_eq!(
            state.apply_combat_clone_with_legal_mask(target_slot, &legal, &[(1, 1), (9, 8)]),
            Some(CombatCloneApplication {
                target_slot,
                actor_slot: expected_actor_slot,
                active_object_slot: expected_object_slot,
                x: 9,
                y: 8,
                actor: clone_combat_actor_descriptor(target_actor, expected_object_slot as u8, 9, 8),
                active_object: clone_active_object_record(state.active_objects[7], 9, 8),
            })
        );

        assert_eq!(
            state.combat_actors[expected_actor_slot],
            clone_combat_actor_descriptor(target_actor, expected_object_slot as u8, 9, 8)
        );
        assert_eq!(
            state.active_objects[expected_object_slot],
            clone_active_object_record(state.active_objects[7], 9, 8)
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn clone_application_writes_no_partial_record_without_capacity_or_placement() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let target_actor =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 39, 7, 2, 5, 5]);
        state.combat_actors = [CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            32,
            1,
            0,
            1,
            1,
        ]); COMBAT_ACTOR_SLOTS];
        state.combat_actors[target_slot] = target_actor;
        state.active_objects = vec![
            ActiveObject {
                type_byte: 0xc0,
                tile: 0xc0,
                x: 1,
                y: 1,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
            OOL_SLOTS
        ];
        state.active_objects[7] = ActiveObject {
            type_byte: 0xdc,
            tile: 0xdd,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x21,
            aux1: 3,
            aux3: 4,
        };
        let actors_before = state.combat_actors;
        let objects_before = state.active_objects.clone();
        let legal = [[true; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

        assert_eq!(
            state.apply_combat_clone_with_legal_mask(target_slot, &legal, &[(9, 8)]),
            None
        );
        assert_eq!(state.combat_actors, actors_before);
        assert_eq!(state.active_objects, objects_before);

        state.combat_actors[target_slot + 1] = CombatActorDescriptor::empty();
        let actors_before = state.combat_actors;
        let objects_before = state.active_objects.clone();
        let blocked = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

        assert_eq!(
            state.apply_combat_clone_with_legal_mask(target_slot, &blocked, &[(9, 8)]),
            None
        );
        assert_eq!(state.combat_actors, actors_before);
        assert_eq!(state.active_objects, objects_before);
    }

    #[test]
    fn combat_cast_clone_routes_resources_and_places_copy_on_legal_cell() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[1][8] = 0x04;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 7,
            hp: 30,
            max_hp: 30,
            level: 7,
        }];
        state.prng_state = 0;
        let spell_index = spell_index_from_code("IQX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let dragon_stats = combat_class_stats(39).unwrap();
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            dragon_stats,
            target_slot as u8,
            5,
            6,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            2,
        );
        state.active_objects[target_slot] = ActiveObject {
            type_byte: 0xdc,
            tile: 0xdc,
            x: 5,
            y: 6,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x33,
            aux3: 0x44,
        };
        state.visibility_dirty = false;
        let mut expected_prng = state.prng_state;
        let _placement_seed = u5_prng_range_u16(
            &mut expected_prng,
            0,
            (COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE - 1) as u16,
        );

        assert_eq!(
            state
                .cast_spell_from_suffix("1IQX7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        let cloned_actor_slot = target_slot + 1;
        let cloned_object_slot = target_slot + 1;
        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.message, "Clone!");
        assert_eq!(
            state.combat_actors[cloned_actor_slot],
            clone_combat_actor_descriptor(
                state.combat_actors[target_slot],
                cloned_object_slot as u8,
                8,
                1,
            )
        );
        assert_eq!(
            state.active_objects[cloned_object_slot],
            clone_active_object_record(state.active_objects[target_slot], 8, 1)
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_cast_clone_rejects_same_faction_target_before_resources() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 7,
                hp: 30,
                max_hp: 30,
                level: 7,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 30,
                max_hp: 30,
                level: 1,
            },
        ];
        let spell_index = spell_index_from_code("IQX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            0,
            4,
            3,
        ]);

        assert_eq!(
            state
                .cast_spell_from_suffix("1IQX2", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[spell_index], 1);
        assert_eq!(state.party[0].mana, 7);
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.message,
            "Target? Use C1IQX7 to target a hostile creature."
        );
    }

    #[test]
    fn combat_cast_charm_routes_resources_and_toggles_target_allegiance() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 6,
            hp: 30,
            max_hp: 30,
            level: 6,
        }];
        let spell_index = spell_index_from_code("AEX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            target_slot as u8,
            0,
            5,
            5,
        ]);

        assert_eq!(
            state
                .cast_spell_from_suffix("1AEX7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Charm!");
        assert_eq!(
            state.combat_actors[target_slot].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE
        );
        assert_eq!(
            resolve_combat_target_group_for_actor(state.combat_actors[target_slot], target_slot, None),
            COMBAT_TARGET_GROUP_PARTY
        );
    }

    #[test]
    fn combat_cast_charm_rejects_already_allied_target_before_resources() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 6,
            hp: 30,
            max_hp: 30,
            level: 6,
        }];
        let spell_index = spell_index_from_code("AEX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
            COMBAT_CLASS_DAEMON,
            target_slot as u8,
            0,
            5,
            5,
        ]);

        assert_eq!(
            state
                .cast_spell_from_suffix("1AEX7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[spell_index], 1);
        assert_eq!(state.party[0].mana, 6);
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.message,
            "Target? Use C1AEX7 to target a hostile creature."
        );
    }

    #[test]
    fn combat_cast_conjure_routes_resources_and_places_weighted_animal() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[0][7] = 0x04;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 2,
            hp: 30,
            max_hp: 30,
            level: 2,
        }];
        state.prng_state = 0;
        let spell_index = spell_index_from_code("KX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let mut expected_prng = state.prng_state;
        let _class_selector = u5_prng_range_u16(
            &mut expected_prng,
            0,
            u16::from(CONJURE_ANIMAL_OUTCOME_COUNT - 1),
        );
        let conjure_x = u5_prng_range_u16(&mut expected_prng, 0, 10) as u8;
        let conjure_y = u5_prng_range_u16(&mut expected_prng, 0, 10) as u8;

        assert_eq!(
            state
                .cast_spell_from_suffix("1KX", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.message, "Success!");
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS],
            resolve_summoned_combat_actor_descriptor(
                COMBAT_CLASS_GIANT_RAT,
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                conjure_x,
                conjure_y,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS],
            summoned_active_object_record(COMBAT_CLASS_GIANT_RAT, 7, 0, 0).unwrap()
        );
    }

    #[test]
    fn combat_cast_swarm_routes_resources_and_places_up_to_eight_swarms() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[4][5] = 0x04;
        state.combat_terrain[4][6] = 0x04;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 5,
            hp: 30,
            max_hp: 30,
            level: 5,
        }];
        state.prng_state = 0;
        let spell_index = spell_index_from_code("BIX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let expected_prng = state.prng_state;

        assert_eq!(
            state
                .cast_spell_from_suffix("1BIX", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.message, "Success!");
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS],
            resolve_swarm_spell_descriptor(
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                5,
                4,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1],
            resolve_swarm_spell_descriptor(
                (COMBAT_PARTY_ACTOR_SLOTS + 1) as u8,
                6,
                4,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS],
            summoned_active_object_record(COMBAT_CLASS_INSECT_SWARM, 5, 4, 0).unwrap()
        );
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 1],
            summoned_active_object_record(COMBAT_CLASS_INSECT_SWARM, 6, 4, 0).unwrap()
        );
    }

    #[test]
    fn combat_cast_summon_daemon_routes_resources_and_places_daemon() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[5][4] = 0x04;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 8,
            hp: 30,
            max_hp: 30,
            level: 8,
        }];
        state.prng_state = 0;
        let spell_index = spell_index_from_code("CKX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let expected_prng = state.prng_state;

        assert_eq!(
            state
                .cast_spell_from_suffix("1CKX", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.message, "Summon Daemon!");
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS],
            resolve_summoned_combat_actor_descriptor(
                COMBAT_CLASS_DAEMON,
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                4,
                5,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.active_objects[COMBAT_PARTY_ACTOR_SLOTS],
            summoned_active_object_record(COMBAT_CLASS_DAEMON, 4, 5, 0).unwrap()
        );
    }

    #[test]
    fn combat_ai_summon_daemon_special_places_daemon_without_spell_resources() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[4][6] = 0x04;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[actor_slot] = CombatActorDescriptor::from_row([
            99,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DRAGON,
            actor_slot as u8,
            0,
            5,
            5,
        ]);
        state.active_objects[actor_slot] = ActiveObject {
            type_byte: 0xdc,
            tile: 0xdc,
            x: 5,
            y: 5,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.visibility_dirty = false;

        let application = state
            .apply_combat_ai_summon_daemon_special_with_candidates(actor_slot, &[(4, 4), (6, 4)])
            .unwrap();

        let summoned_actor_slot = actor_slot + 1;
        let summoned_object_slot = actor_slot + 1;
        assert_eq!(
            application,
            CombatAiSpecialApplication::SummonDaemon {
                actor_slot,
                summon: CombatSummonApplication {
                    class: COMBAT_CLASS_DAEMON,
                    actor_slot: summoned_actor_slot,
                    active_object_slot: summoned_object_slot,
                    x: 6,
                    y: 4,
                    actor: resolve_summoned_combat_actor_descriptor(
                        COMBAT_CLASS_DAEMON,
                        summoned_object_slot as u8,
                        6,
                        4,
                        COMBAT_SUMMONED_ACTOR_FLAGS,
                        0,
                    )
                    .unwrap(),
                    active_object: summoned_active_object_record(COMBAT_CLASS_DAEMON, 6, 4, -1)
                        .unwrap(),
                },
            }
        );
        assert_eq!(
            state.combat_actors[summoned_actor_slot],
            resolve_summoned_combat_actor_descriptor(
                COMBAT_CLASS_DAEMON,
                summoned_object_slot as u8,
                6,
                4,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.active_objects[summoned_object_slot],
            summoned_active_object_record(COMBAT_CLASS_DAEMON, 6, 4, -1).unwrap()
        );
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Monster summons daemon.");
    }

    #[test]
    fn combat_ai_summon_daemon_special_rejects_non_summoner_class() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[actor_slot] = CombatActorDescriptor::from_row([
            75,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            actor_slot as u8,
            0,
            5,
            5,
        ]);
        state.active_objects[actor_slot] = ActiveObject {
            type_byte: 0xd8,
            tile: 0xd8,
            x: 5,
            y: 5,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        assert_eq!(
            state.apply_combat_ai_summon_daemon_special_with_candidates(actor_slot, &[(6, 5)]),
            None
        );
    }

    #[test]
    fn mass_charm_group_remap_uses_strict_threshold() {
        assert_eq!(resolve_mass_charm_target_group(3, 25, 25), 3);
        assert_eq!(resolve_mass_charm_target_group(3, 25, 26), 0);
        assert_eq!(resolve_mass_charm_target_group(3, 255, 255), 3);
    }

    #[test]
    fn combat_target_group_helper_uses_party_monster_and_saduj_name_rules() {
        assert_eq!(
            resolve_combat_target_group(0, Some(b"Avatar"), false),
            COMBAT_TARGET_GROUP_PARTY
        );
        assert_eq!(
            resolve_combat_target_group(6, None, false),
            COMBAT_TARGET_GROUP_MONSTER
        );
        assert_eq!(
            resolve_combat_target_group(0, Some(b"Saduj"), false),
            COMBAT_TARGET_GROUP_MONSTER
        );
        assert_eq!(
            resolve_combat_target_group(0, Some(b"SADUJ"), false),
            COMBAT_TARGET_GROUP_PARTY
        );
        assert_eq!(
            resolve_combat_target_group(COMBAT_ACTOR_SLOTS, None, false),
            COMBAT_TARGET_GROUP_NEUTRAL
        );
    }

    #[test]
    fn combat_target_group_helper_applies_team_toggle_without_overriding_saduj_rule() {
        assert_eq!(
            resolve_combat_target_group(0, Some(b"Avatar"), true),
            COMBAT_TARGET_GROUP_MONSTER
        );
        assert_eq!(
            resolve_combat_target_group(6, None, true),
            COMBAT_TARGET_GROUP_PARTY
        );
        assert_eq!(
            resolve_combat_target_group(0, Some(b"Saduj"), true),
            COMBAT_TARGET_GROUP_MONSTER
        );
    }

    #[test]
    fn combat_target_candidate_view_helper_packages_group_and_visibility_inputs() {
        let descriptor = CombatActorDescriptor::from_row([10, 1, 0, 0, 4, 0, 3, 2]);

        let view = combat_target_candidate_view_from_descriptor(
            descriptor,
            0,
            Some(b"Avatar"),
            true,
            false,
        );

        assert_eq!(view.descriptor, descriptor);
        assert_eq!(view.group, COMBAT_TARGET_GROUP_PARTY);
        assert!(view.suppressed);
        assert!(!view.invisible_or_unrevealed);
    }

    #[test]
    fn active_effect_aging_keeps_zero_and_255_inert() {
        assert_eq!(
            age_active_effect_state(Some(PROTECTION_ACTIVE_EFFECT_TAG), 0),
            ActiveEffectAgeOutcome {
                tag: None,
                counter: 0,
                expired: false,
            }
        );
        assert_eq!(
            age_active_effect_state(Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG), u8::MAX),
            ActiveEffectAgeOutcome {
                tag: Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG),
                counter: u8::MAX,
                expired: false,
            }
        );
        assert_eq!(
            age_active_effect_state(Some(QUICKNESS_ACTIVE_EFFECT_TAG), 2),
            ActiveEffectAgeOutcome {
                tag: Some(QUICKNESS_ACTIVE_EFFECT_TAG),
                counter: 1,
                expired: false,
            }
        );
        assert_eq!(
            age_active_effect_state(Some(QUICKNESS_ACTIVE_EFFECT_TAG), 1),
            ActiveEffectAgeOutcome {
                tag: None,
                counter: 0,
                expired: true,
            }
        );
    }

    #[test]
    fn active_effect_state_wrapper_marks_redraw_only_on_expiry() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 2;
        state.visibility_dirty = false;

        assert_eq!(
            state.age_active_effect(),
            ActiveEffectAgeOutcome {
                tag: Some(QUICKNESS_ACTIVE_EFFECT_TAG),
                counter: 1,
                expired: false,
            }
        );
        assert!(!state.visibility_dirty);

        assert_eq!(
            state.age_active_effect(),
            ActiveEffectAgeOutcome {
                tag: None,
                counter: 0,
                expired: true,
            }
        );
        assert!(state.visibility_dirty);

        state.visibility_dirty = false;
        state.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = u8::MAX;
        assert_eq!(
            state.age_active_effect(),
            ActiveEffectAgeOutcome {
                tag: Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG),
                counter: u8::MAX,
                expired: false,
            }
        );
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn active_effect_consumers_use_tag_and_nonzero_counter() {
        assert_eq!(
            resolve_protection_defense_bonus(7, Some(PROTECTION_ACTIVE_EFFECT_TAG), 20),
            10
        );
        assert_eq!(
            resolve_protection_defense_bonus(254, Some(PROTECTION_ACTIVE_EFFECT_TAG), 20),
            255
        );
        assert_eq!(
            resolve_protection_defense_bonus(7, Some(QUICKNESS_ACTIVE_EFFECT_TAG), 20),
            7
        );

        assert!(resolve_quickness_dispatch_consumed(
            Some(QUICKNESS_ACTIVE_EFFECT_TAG),
            30,
            0
        ));
        assert!(!resolve_quickness_dispatch_consumed(
            Some(QUICKNESS_ACTIVE_EFFECT_TAG),
            30,
            1
        ));
        assert!(!resolve_quickness_dispatch_consumed(
            Some(QUICKNESS_ACTIVE_EFFECT_TAG),
            0,
            0
        ));

        assert!(resolve_negate_magic_absorbs_combat_cast(
            Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG),
            10
        ));
        assert!(!resolve_negate_magic_absorbs_combat_cast(
            Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG),
            0
        ));
        assert!(!resolve_negate_magic_absorbs_combat_cast(
            Some(MASS_CHARM_ACTIVE_EFFECT_TAG),
            20
        ));
    }

    #[test]
    fn combat_ai_step_vector_uses_axis_sign_and_flee_inversion() {
        assert_eq!(
            combat_ai_step_vector(4, 4, 7, 1, false),
            CombatStepVector { dx: 1, dy: -1 }
        );
        assert_eq!(
            combat_ai_step_vector(4, 4, 7, 1, true),
            CombatStepVector { dx: -1, dy: 1 }
        );
        assert_eq!(
            combat_ai_step_vector(4, 4, 4, 8, false),
            CombatStepVector { dx: 0, dy: 1 }
        );
    }

    fn combat_target_view(
        descriptor: CombatActorDescriptor,
        group: u8,
    ) -> CombatTargetCandidateView {
        CombatTargetCandidateView {
            descriptor,
            group,
            suppressed: false,
            invisible_or_unrevealed: false,
        }
    }

    #[test]
    fn combat_ai_target_picker_scans_backwards_and_keeps_low_slot_ties() {
        let mut actors = [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];
        actors[10] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 32, 0, 0, 5, 5]),
            2,
        );
        actors[3] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 4, 5]),
            1,
        );
        actors[7] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 33, 0, 0, 6, 5]),
            1,
        );
        actors[20] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 34, 0, 0, 8, 8]),
            1,
        );

        let pick = find_combat_ai_target(&actors, 10, 2, false);

        assert_eq!(pick.slot, Some(3));
        assert!(pick.first_five_party_slot_survived);
    }

    #[test]
    fn combat_ai_target_picker_applies_group_suppression_and_visibility_filters() {
        let mut actors = [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];
        actors[10] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 32, 0, 0, 5, 5]),
            2,
        );
        actors[1] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 0, 0, 0, 4, 5]),
            1,
        );
        actors[2] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 4, 5]),
            2,
        );
        actors[3] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 4, 5]),
            2,
        );
        actors[4] = CombatTargetCandidateView {
            suppressed: true,
            ..combat_target_view(
                CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 4, 5]),
                1,
            )
        };
        actors[6] = CombatTargetCandidateView {
            invisible_or_unrevealed: true,
            ..combat_target_view(
                CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 4, 5]),
                1,
            )
        };

        let rejected = find_combat_ai_target(&actors, 10, 2, false);
        assert_eq!(rejected.slot, None);
        assert!(!rejected.first_five_party_slot_survived);

        let bypassed = find_combat_ai_target(&actors, 10, 2, true);
        assert_eq!(bypassed.slot, Some(4));
        assert!(bypassed.first_five_party_slot_survived);
    }

    #[test]
    fn combat_ai_target_picker_allows_status_disabled_targets() {
        let mut actors = [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];
        actors[10] = combat_target_view(
            CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 10, 0, 5, 5]),
            2,
        );
        actors[0] = combat_target_view(
            CombatActorDescriptor::from_row([
                10,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_STATUS_DISABLED,
                0,
                0,
                0,
                4,
                5,
            ]),
            1,
        );

        let pick = find_combat_ai_target(&actors, 10, 2, false);
        assert_eq!(pick.slot, Some(0));
        assert!(pick.first_five_party_slot_survived);
    }

    fn combat_ai_turn_state(monster_x: u8, monster_y: u8) -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[0] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        state.active_objects[8] = ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: usize::from(monster_x),
            y: usize::from(monster_y),
            ..ActiveObject::empty()
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            8,
            0,
            monster_x,
            monster_y,
        ]);
        state
    }

    #[test]
    fn combat_ai_turn_moves_toward_out_of_range_target_and_updates_linked_object() {
        let mut state = combat_ai_turn_state(8, 5);

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[4, 1, 3, 2],
                None,
            )
            .unwrap();

        assert_eq!(application.actor_slot, 8);
        assert_eq!(application.special, None);
        assert!(!application.possess_hook_handled);
        assert_eq!(application.acting_group, COMBAT_TARGET_GROUP_MONSTER);
        assert_eq!(
            application.target,
            CombatAiTargetResolution::ChosenActor {
                slot: 0,
                x: 5,
                y: 5,
            }
        );
        assert_eq!(
            application.step_vector,
            Some(CombatStepVector { dx: -1, dy: 0 })
        );
        assert_eq!(application.attack_route, Some(CombatAiAttackRoute::OutOfRange));
        assert_eq!(application.monster_attack, None);
        assert_eq!(
            application.movement,
            Some(CombatAiMovementOutcome::Step {
                direction_code: 1,
                x: 7,
                y: 5,
            })
        );
        assert_eq!(application.command_key, Some('W'));
        assert_eq!(application.movement_commit.unwrap().active_object_slot, 8);
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
        assert_eq!((state.active_objects[8].x, state.active_objects[8].y), (7, 5));
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_ai_turn_uses_wound_morale_to_flee_from_target() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].hp_or_wound = 2;

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[4, 1, 3, 2],
                None,
            )
            .unwrap();

        assert_eq!(
            application.step_vector,
            Some(CombatStepVector { dx: 1, dy: 0 })
        );
        assert_eq!(
            application.movement,
            Some(CombatAiMovementOutcome::Step {
                direction_code: 2,
                x: 9,
                y: 5,
            })
        );
        assert!(state.combat_actors[8].is_fleeing());
    }

    #[test]
    fn combat_ai_turn_applies_saduj_name_group_to_target_scan() {
        let mut state = combat_ai_turn_state(8, 5);
        state.party_names[0] = *b"ABCDj\0\0\0\0";

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[4, 1, 3, 2],
                None,
            )
            .unwrap();

        assert_eq!(
            application.target,
            CombatAiTargetResolution::CenterFallback {
                x: COMBAT_ARENA_CENTER_COORDINATE,
                y: COMBAT_ARENA_CENTER_COORDINATE,
                critical_hp_flee_slots: vec![8],
            }
        );
    }

    #[test]
    fn combat_ai_turn_doom_context_bypasses_suppressed_phase_targets() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
        state.combat_frame_snapshot = Some(CombatFrameSnapshot {
            area: Area::Dungeon {
                scene: DungeonScene {
                    byte: DUNGEON_DOOM_SCENE_BYTE,
                    record: DOOM_DUNGEON_RECORD,
                },
                level: 0,
            },
            player: state.player,
            active_objects: state.active_objects.clone(),
            active_player: state.active_player,
            combat_terrain: state.combat_terrain,
            dungeon_room_clear_on_success: None,
            enter_endgame_after_successful_combat: false,
            endgame_messages: None,
        });

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[4, 1, 3, 2],
                None,
            )
            .unwrap();

        assert_eq!(
            application.target,
            CombatAiTargetResolution::ChosenActor {
                slot: 0,
                x: 5,
                y: 5,
            }
        );
    }

    #[test]
    fn combat_ai_summon_daemon_prefers_current_step_direction() {
        let mut state = combat_ai_turn_state(8, 5);
        let stats = combat_class_stats(COMBAT_CLASS_DRAGON).unwrap();
        state.combat_actors[8] = CombatActorDescriptor::for_monster_placement(
            stats,
            8,
            8,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        state.active_objects[8].type_byte = 0xdc;
        state.active_objects[8].tile = 0xdc;

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                8,
                &[(9, 5), (7, 5)],
                None,
                0,
                false,
                None,
                true,
                &[4, 1, 3, 2],
                None,
            )
            .unwrap();

        let Some(CombatAiSpecialApplication::SummonDaemon { summon, .. }) = application.special
        else {
            panic!("dragon should summon a daemon");
        };
        assert_eq!((summon.x, summon.y), (7, 5));
    }

    #[test]
    fn combat_ai_turn_synthesizes_attack_for_adjacent_target_without_moving() {
        let mut state = combat_ai_turn_state(6, 5);

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[1, 2, 3, 4],
                None,
            )
            .unwrap();

        assert_eq!(
            application.target,
            CombatAiTargetResolution::ChosenActor {
                slot: 0,
                x: 5,
                y: 5,
            }
        );
        assert_eq!(application.attack_route, Some(CombatAiAttackRoute::Melee));
        assert_eq!(application.monster_attack, None);
        assert_eq!(application.command_key, Some(COMBAT_AI_ATTACK_COMMAND_KEY));
        assert_eq!(application.movement, None);
        assert_eq!(application.movement_commit, None);
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (6, 5));
        assert_eq!((state.active_objects[8].x, state.active_objects[8].y), (6, 5));
    }

    #[test]
    fn combat_ai_turn_applies_monster_attack_when_attack_inputs_are_supplied() {
        let mut state = combat_ai_turn_state(6, 5);
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[1, 2, 3, 4],
                Some(CombatMonsterAttackInputs {
                    party_defender_rating: 0,
                    hit_roll: 0,
                    damage_roll: 0,
                    forced_hit: Some(true),
                    ..CombatMonsterAttackInputs::default()
                }),
            )
            .unwrap();

        assert_eq!(application.attack_route, Some(CombatAiAttackRoute::Melee));
        let monster_attack = application.monster_attack.unwrap();
        assert_eq!(monster_attack.attacker_slot, 8);
        assert_eq!(monster_attack.target_slot, 0);
        assert!(matches!(
            monster_attack.resolution,
            Some(CombatWeaponAttackResolution::Hit {
                route: CombatWeaponAttackRangeRoute::Melee,
                ..
            })
        ));
        assert!(matches!(
            monster_attack.damage_application,
            Some(CombatWeaponDamageApplication::Party { target_slot: 0, .. })
        ));
        assert!(state.party[0].hp < 20);
        assert_eq!(application.command_key, Some(COMBAT_AI_ATTACK_COMMAND_KEY));
        assert_eq!(application.movement, None);
        assert_eq!(application.movement_commit, None);
    }

    #[test]
    fn combat_ai_possess_special_mutates_control_state_and_daemon_clears_self() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = COMBAT_CLASS_DAEMON;

        let application = state
            .apply_combat_ai_possess_special_with_inputs(8, 0, false)
            .unwrap();

        assert_eq!(
            application,
            CombatAiSpecialApplication::Possess {
                actor_slot: 8,
                target_slot: 0,
                outcome: CombatPossessResistanceOutcome::Landed {
                    cleared_active_player: false,
                    daemon_clears_self: true,
                },
                target_flags_before: COMBAT_ACTOR_FLAG_SELECTABLE_80,
                target_flags_after: COMBAT_ACTOR_FLAG_SELECTABLE_80
                    | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
            }
        );
        assert_eq!(
            state.combat_actors[0].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE
        );
        assert_eq!(state.active_player, None);
        assert!(state.combat_actors[8].is_empty());
        assert!(state.active_objects[8].is_empty());
        assert_eq!(state.message, "Monster possessed party member 1.");
    }

    #[test]
    fn combat_ai_possess_special_resistance_blocks_without_mutation() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = 28;

        let application = state
            .apply_combat_ai_possess_special_with_inputs(8, 0, true)
            .unwrap();

        assert_eq!(
            application,
            CombatAiSpecialApplication::Possess {
                actor_slot: 8,
                target_slot: 0,
                outcome: CombatPossessResistanceOutcome::Blocked,
                target_flags_before: COMBAT_ACTOR_FLAG_SELECTABLE_80,
                target_flags_after: COMBAT_ACTOR_FLAG_SELECTABLE_80,
            }
        );
        assert_eq!(state.combat_actors[0].flags, COMBAT_ACTOR_FLAG_SELECTABLE_80);
        assert!(!state.combat_actors[8].is_empty());
        assert_eq!(state.message, "Possession resisted.");
    }

    #[test]
    fn combat_ai_turn_applies_possess_hook_before_target_synthesis() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = 28;

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                true,
                0,
                false,
                0,
                0,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[1, 2, 3, 4],
                None,
            )
            .unwrap();

        assert!(application.possess_hook_handled);
        assert!(matches!(
            application.special,
            Some(CombatAiSpecialApplication::Possess {
                target_slot: 0,
                outcome: CombatPossessResistanceOutcome::Landed {
                    daemon_clears_self: false,
                    ..
                },
                ..
            })
        ));
        assert_eq!(
            application.target,
            CombatAiTargetResolution::CenterFallback {
                x: COMBAT_ARENA_CENTER_COORDINATE,
                y: COMBAT_ARENA_CENTER_COORDINATE,
                critical_hp_flee_slots: vec![8],
            }
        );
        assert_eq!(
            state.combat_actors[0].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE
        );
    }

    #[test]
    fn combat_round_production_path_can_drive_possess_special() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = 28;
        state.combat_actors[8].phase_counter = 1;
        state.next_combat_actor_slot = 8;
        state.prng_state = 0x00f0;

        let application = state.ensure_pending_combat_player_turn().unwrap();

        assert!(matches!(
            application.stop_reason,
            CombatRoundWalkStopReason::AwaitingPlayer | CombatRoundWalkStopReason::EndOfRound
        ));
        assert_eq!(
            state.combat_actors[0].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE
        );
        assert_eq!(state.message, "Monster possessed party member 1.");
    }

    #[test]
    fn combat_round_walk_production_path_applies_monster_attack_damage() {
        let mut state = combat_ai_turn_state(6, 5);
        state.combat_actors[8].phase_counter = 1;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;

        let application = state.apply_combat_round_walk_from_slot(8, 30, false);

        assert_eq!(application.stop_reason, CombatRoundWalkStopReason::EndOfRound);
        let monster_attack = application
            .applications
            .iter()
            .find_map(|entry| match entry {
                CombatActorSlotDispatchApplication::Slot {
                    slot: 8,
                    action:
                        CombatActorDispatchAction::MonsterAi {
                            ai_turn: Some(ai_turn),
                        },
                    ..
                } => ai_turn.monster_attack,
                _ => None,
            })
            .expect("production round walker should provide monster attack inputs");
        assert_eq!(monster_attack.attacker_slot, 8);
        assert_eq!(monster_attack.target_slot, 0);
        assert_eq!(
            monster_attack.poison_status_outcome,
            Some(CombatPoisonStatusAttackOutcome::PoisonedPartyMember {
                status_before: b'G',
                status_after: b'P',
            })
        );
        assert_eq!(monster_attack.resolution, None);
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 20);
    }

    #[test]
    fn combat_round_walk_production_path_applies_amulet_turning_scatter() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = 28;
        state.combat_actors[8].phase_counter = 1;
        state.turn = 0;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_AMULET] = EQUIPMENT_ID_AMULET_TURNING as u8;

        let application = state.apply_combat_round_walk_from_slot(8, 30, false);

        assert_eq!(application.stop_reason, CombatRoundWalkStopReason::EndOfRound);
        let monster_attack = application
            .applications
            .iter()
            .find_map(|entry| match entry {
                CombatActorSlotDispatchApplication::Slot {
                    slot: 8,
                    action:
                        CombatActorDispatchAction::MonsterAi {
                            ai_turn: Some(ai_turn),
                        },
                    ..
                } => ai_turn.monster_attack,
                _ => None,
            })
            .expect("turnable ranged attack should still resolve through monster attack");
        assert_eq!(
            monster_attack.resolution,
            Some(CombatWeaponAttackResolution::Miss {
                route: CombatWeaponAttackRangeRoute::Ranged { effect_code: 6 },
                hit_score: 0,
            })
        );
        assert_eq!(state.party[0].hp, 20);
    }

    #[test]
    fn combat_round_walk_amulet_turning_scatter_can_hit_adjacent_impact_actor() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = 28;
        state.combat_actors[8].phase_counter = 1;
        state.prng_state = 0x0003;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        });
        state.party_equipment = default_party_equipment(2);
        state.party_equipment[0][EQUIP_SLOT_AMULET] = EQUIPMENT_ID_AMULET_TURNING as u8;
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            0,
            4,
            4,
        ]);

        let application = state.apply_combat_round_walk_from_slot(8, 30, false);
        let monster_attack = application
            .applications
            .iter()
            .find_map(|entry| match entry {
                CombatActorSlotDispatchApplication::Slot {
                    slot: 8,
                    action:
                        CombatActorDispatchAction::MonsterAi {
                            ai_turn: Some(ai_turn),
                        },
                    ..
                } => ai_turn.monster_attack,
                _ => None,
            })
            .expect("turnable ranged attack should resolve at the scattered impact cell");

        assert_eq!(monster_attack.target_slot, 1);
        assert_eq!(
            monster_attack.resolution,
            Some(CombatWeaponAttackResolution::Hit {
                route: CombatWeaponAttackRangeRoute::Ranged { effect_code: 6 },
                raw_damage: 1,
            })
        );
        assert_eq!(state.party[0].hp, 20);
        assert_eq!(state.party[1].hp, 19);
    }

    fn combat_player_command_state(monster_x: u8, monster_y: u8) -> PlayState {
        let mut state = combat_ai_turn_state(monster_x, monster_y);
        state.combat_actors[8].phase_counter = 0;
        state
    }

    fn advance_expected_giant_rat_ai_input_prng(expected_prng: &mut u16) {
        let _ = u5_prng_range_u16(expected_prng, 0, 1);
        for _ in 0..4 {
            let _ = u5_prng_range_u16(expected_prng, 1, 4);
        }
        let _ = u5_prng_range_u16(expected_prng, 0, u16::from(u8::MAX));
        let _ = u5_prng_range_u16(expected_prng, 0, u16::from(u8::MAX));
        let _ = u5_prng_range_u16(expected_prng, 0, 1);
        let _ = u5_prng_range_u16(expected_prng, 0, 19);
        let _ = u5_prng_range_u16(expected_prng, 0, 7);
    }

    #[test]
    fn combat_push_static_tile_uses_actor_anchor_and_arena_terrain() {
        let mut state = combat_player_command_state(10, 10);
        state.visibility_dirty = false;
        state.combat_terrain[5][6] = 0x90;
        state.combat_terrain[5][7] = PUSHABLE_GENERIC_FLOOR_STAMP;

        let outcome = state.push_combat_actor_direction(0, Direction::East);

        assert_eq!(outcome, MoveOutcome::Pushed);
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (6, 5));
        assert_eq!(state.combat_terrain[5][6], PUSHABLE_GENERIC_FLOOR_STAMP);
        assert_eq!(state.combat_terrain[5][7], 0x91);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Pushed combat tile 144 East"));
    }

    #[test]
    fn combat_push_dynamic_object_moves_loose_object_only_in_arena() {
        let mut state = combat_player_command_state(10, 10);
        state.visibility_dirty = false;
        state.active_objects[1] = ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 6,
            y: 5,
            ..ActiveObject::empty()
        };

        let outcome = state.push_combat_actor_direction(0, Direction::East);

        assert_eq!(outcome, MoveOutcome::Pushed);
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (6, 5));
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (7, 5));
        assert_eq!(state.active_objects[1].tile, 0x91);
        assert_eq!(state.active_objects[1].type_byte, 0x91);
        assert_eq!(state.combat_terrain[5][6], 0x04);
        assert_eq!(state.combat_terrain[5][7], 0x04);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Pushed combat object tile 144 East"));
    }

    #[test]
    fn combat_input_dispatch_push_prompt_keeps_actor_until_direction() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.visibility_dirty = false;
        state.combat_terrain[5][6] = 0x90;
        state.combat_terrain[5][7] = PUSHABLE_GENERIC_FLOOR_STAMP;

        assert_eq!(
            handle_play_key_input(&mut state, 'P', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "Push-");
        assert!(matches!(
            state.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::CombatPush { actor_slot: 0 })
        ));
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_direction_prompt.is_none());
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!(state.combat_terrain[5][6], PUSHABLE_GENERIC_FLOOR_STAMP);
        assert_eq!(state.combat_terrain[5][7], 0x91);
        assert!(state.message.contains("Pushed combat tile 144 East"));
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_input_dispatch_push_prompt_cancel_restores_pending_actor() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);

        assert_eq!(
            handle_play_key_input(&mut state, 'P', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.pending_combat_actor_slot, None);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_direction_prompt.is_none());
        assert_eq!(state.pending_combat_actor_slot, Some(0));
        assert_eq!(state.message, DIRECTION_PROMPT_LABEL_PASS);
    }

    #[test]
    fn combat_input_dispatch_inline_push_suffix_pushes_immediately() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.combat_terrain[5][6] = 0x90;
        state.combat_terrain[5][7] = PUSHABLE_GENERIC_FLOOR_STAMP;

        assert_eq!(
            handle_play_key_input(&mut state, 'P', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_direction_prompt.is_none());
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!(state.combat_terrain[5][6], PUSHABLE_GENERIC_FLOOR_STAMP);
        assert_eq!(state.combat_terrain[5][7], 0x91);
        assert!(state.message.contains("Pushed combat tile 144 East"));
    }

    #[test]
    fn combat_klimb_cardinal_suffix_moves_actor_inside_arena() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.visibility_dirty = false;
        state.combat_terrain[4][5] = 0x04;

        assert_eq!(
            handle_play_key_input(&mut state, 'K', "8", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 4));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (5, 4));
        assert_eq!(
            state.message,
            "Klimbed North to (5, 4).\nGiant Rat moved to (9, 10)."
        );
        assert!(state.visibility_dirty);
        assert!(state.combat_active);
    }

    #[test]
    fn combat_klimb_vertical_suffix_exits_from_ladder_tile() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.combat_terrain[5][5] = 0x50;

        assert_eq!(
            handle_play_key_input(&mut state, 'K', "<", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Klimbed up from combat.");
        assert!(!state.combat_active);
        assert_eq!(state.pending_combat_actor_slot, None);
    }

    #[test]
    fn combat_klimb_prompt_accepts_vertical_and_refusal_keeps_actor_pending() {
        let game_dir = std::path::Path::new(".");
        let mut prompted = combat_player_command_state(10, 10);
        prompted.combat_terrain[5][5] = 0x50;

        assert_eq!(
            handle_play_key_input(&mut prompted, 'K', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(prompted.message, "Klimb-");
        assert!(matches!(
            prompted.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::CombatKlimb { actor_slot: 0 })
        ));

        assert_eq!(
            handle_play_key_input(&mut prompted, '>', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(prompted.message, "Klimbed down from combat.");
        assert!(!prompted.combat_active);

        let mut refused = combat_player_command_state(10, 10);
        assert_eq!(
            handle_play_key_input(&mut refused, 'K', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_key_input(&mut refused, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(refused.pending_combat_actor_slot, Some(0));
        assert_eq!(refused.message, DIRECTION_PROMPT_LABEL_PASS);
    }

    #[test]
    fn combat_sjog_get_removes_loose_combat_object() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.active_objects[1] = ActiveObject {
            type_byte: 0x50,
            tile: 0x50,
            x: 6,
            y: 5,
            ..ActiveObject::empty()
        };
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'G', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_objects[1].is_empty());
        assert!(state.visibility_dirty);
        assert!(state.message.starts_with("Got combat object tile 80 at (6, 5)."));
        assert!(state.message.contains("Giant Rat"));
    }

    #[test]
    fn combat_sjog_open_and_jimmy_mutate_combat_terrain() {
        let game_dir = std::path::Path::new(".");
        let mut open_state = combat_player_command_state(10, 10);
        open_state.combat_terrain[5][6] = 97;
        open_state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut open_state, 'O', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(open_state.combat_terrain[5][6], 16);
        assert!(open_state.visibility_dirty);
        assert!(open_state
            .message
            .starts_with("Opened combat tile 97 at (6, 5)."));

        let mut jimmy_state = combat_player_command_state(10, 10);
        jimmy_state.keys = 1;
        jimmy_state.party[0].class_byte = 255;
        jimmy_state.combat_terrain[5][6] = 99;
        jimmy_state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut jimmy_state, 'J', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(jimmy_state.combat_terrain[5][6], 98);
        assert_eq!(jimmy_state.keys, 1);
        assert!(jimmy_state.visibility_dirty);
        assert!(jimmy_state
            .message
            .starts_with("Unlocked combat tile 99 at (6, 5)."));
    }

    #[test]
    fn combat_sjog_search_observes_without_removing_object() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.active_objects[1] = ActiveObject {
            type_byte: 0x51,
            tile: 0x51,
            x: 6,
            y: 5,
            ..ActiveObject::empty()
        };

        assert_eq!(
            handle_play_key_input(&mut state, 'S', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(!state.active_objects[1].is_empty());
        assert!(state
            .message
            .starts_with("Found combat object tile 81 at (6, 5)."));
    }

    #[test]
    fn combat_sjog_prompt_cancel_restores_pending_actor() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);

        assert_eq!(
            handle_play_key_input(&mut state, 'G', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "Get-");
        assert!(matches!(
            state.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::CombatSjog {
                actor_slot: 0,
                branch: CombatCommandBranch::Get,
            })
        ));

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.pending_combat_actor_slot, Some(0));
        assert_eq!(state.message, DIRECTION_PROMPT_LABEL_PASS);
    }

    #[test]
    fn combat_player_command_quickness_can_consume_dispatch_before_input() {
        let mut state = combat_player_command_state(8, 5);
        assert_eq!(state.combat_quickness_dispatch_roll(0), 1);
        assert_eq!(state.prng_state, 0);

        state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 3;
        let mut expected_prng = state.prng_state;
        let quickness_roll = u5_prng_range_u16(&mut expected_prng, 0, 1) as u8;
        assert_eq!(state.combat_quickness_dispatch_roll(0), quickness_roll);
        assert_eq!(state.prng_state, expected_prng);

        let application = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('Q'), 0)
            .unwrap();

        assert_eq!(
            application,
            CombatPlayerCommandApplication {
                actor_slot: 0,
                input: CombatPlayerCommandInput::Key('Q'),
                action: CombatPlayerCommandAction::QuicknessSkipped,
                weapon_attack: None,
                ring_pass: None,
                control_after: CombatRoundLoopControl::ContinueActorWalk,
            }
        );
        assert!(state.combat_active);
        assert!(state.party[0].living());

        let quit = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('Q'), 1)
            .unwrap();
        assert_eq!(quit.action, CombatPlayerCommandAction::QuitDefeat);
        assert_eq!(
            quit.control_after,
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
        );
    }

    #[test]
    fn combat_player_command_routes_direction_and_attack_prompt_through_step_primitive() {
        let mut move_state = combat_player_command_state(8, 5);
        move_state.visibility_dirty = false;

        let moved = move_state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(2), 1)
            .unwrap();

        assert_eq!(
            moved.action,
            CombatPlayerCommandAction::StepOrAttack {
                prompted_attack: false,
                direction_code: 2,
                outcome: CombatStepOrAttackPrimitiveOutcome::Moved {
                    commit: CombatLinkedPositionCommitOutcome {
                        active_object_slot: 0,
                        actor_position_before: (5, 5),
                        actor_position_after: (6, 5),
                        active_object_position_before: Some((5, 5)),
                        active_object_position_after: Some((6, 5)),
                    },
                },
            }
        );
        assert_eq!((move_state.combat_actors[0].x, move_state.combat_actors[0].y), (6, 5));
        assert_eq!((move_state.active_objects[0].x, move_state.active_objects[0].y), (6, 5));
        assert!(move_state.visibility_dirty);

        let mut attack_state = combat_player_command_state(6, 5);
        attack_state.visibility_dirty = false;
        let attacked = attack_state
            .apply_combat_player_command_with_inputs(
                0,
                CombatPlayerCommandInput::AttackDirection(2),
                1,
            )
            .unwrap();

        assert_eq!(
            attacked.action,
            CombatPlayerCommandAction::StepOrAttack {
                prompted_attack: true,
                direction_code: 2,
                outcome: CombatStepOrAttackPrimitiveOutcome::Attack { target_slot: 8 },
            }
        );
        assert_eq!(
            (attack_state.combat_actors[0].x, attack_state.combat_actors[0].y),
            (5, 5)
        );
        assert!(!attack_state.visibility_dirty);
    }

    #[test]
    fn combat_player_command_out_of_arena_direction_exits_combat() {
        let mut state = combat_player_command_state(8, 5);
        state.combat_actors[0].x = 10;
        state.active_objects[0].x = 10;

        let application = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(2), 1)
            .unwrap();

        assert_eq!(
            application.action,
            CombatPlayerCommandAction::StepOrAttack {
                prompted_attack: false,
                direction_code: 2,
                outcome: CombatStepOrAttackPrimitiveOutcome::OutOfArena { x: 11, y: 5 },
            }
        );
        assert_eq!(
            application.control_after,
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        );
    }

    #[test]
    fn combat_player_command_attack_applies_readied_weapon_damage() {
        let mut state = combat_player_command_state(6, 5);
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
        state.party_strengths = vec![30];
        state.party_experience = vec![0];
        let hp_before = state.combat_actors[8].hp_or_wound;

        let application = state
            .apply_combat_player_command_with_attack_inputs(
                0,
                CombatPlayerCommandInput::AttackDirection(2),
                1,
                CombatPlayerWeaponAttackInputs {
                    damage_roll: 0,
                    forced_hit: Some(true),
                    ..CombatPlayerWeaponAttackInputs::default()
                },
            )
            .unwrap();

        assert_eq!(
            application.action,
            CombatPlayerCommandAction::StepOrAttack {
                prompted_attack: true,
                direction_code: 2,
                outcome: CombatStepOrAttackPrimitiveOutcome::Attack { target_slot: 8 },
            }
        );
        assert!(matches!(
            application.weapon_attack,
            Some(CombatWeaponAttackApplication {
                resolution: CombatWeaponAttackResolution::Hit {
                    route: CombatWeaponAttackRangeRoute::Melee,
                    raw_damage: 1,
                },
                damage_application: Some(CombatWeaponDamageApplication::Monster {
                    target_slot: 8,
                    ..
                }),
            })
        ));
        assert_eq!(state.combat_actors[8].hp_or_wound, hp_before - 1);
        assert_eq!(state.party_experience[0], 1);
    }

    #[test]
    fn combat_player_command_attack_exits_when_last_foe_dies() {
        let mut state = combat_player_command_state(6, 5);
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
        state.party_strengths = vec![30];
        state.combat_actors[8].hp_or_wound = 1;

        let application = state
            .apply_combat_player_command_with_attack_inputs(
                0,
                CombatPlayerCommandInput::AttackDirection(2),
                1,
                CombatPlayerWeaponAttackInputs {
                    damage_roll: 0,
                    forced_hit: Some(true),
                    ..CombatPlayerWeaponAttackInputs::default()
                },
            )
            .unwrap();

        assert!(state.combat_actors[8].is_marked_dead());
        assert_eq!(
            application.control_after,
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        );
        assert!(application.weapon_attack.is_some());
    }

    #[test]
    fn combat_player_command_runs_visible_magic_ring_pass() {
        let mut state = combat_player_command_state(8, 5);
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
        state.party[0].hp = state.party[0].hp.saturating_sub(3);
        let hp_before = state.party[0].hp;
        state.prng_state = 0x0030;
        let mut expected_prng = state.prng_state;
        let regeneration_roll = u5_prng_range_u16(&mut expected_prng, 0, 7);
        let vanish_roll = u5_prng_range_u16(&mut expected_prng, 0, 15);
        assert_eq!(regeneration_roll, 0);
        assert_ne!(vanish_roll, 0);

        let application = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(' '), 1)
            .unwrap();

        assert_eq!(
            application.ring_pass,
            Some(CombatMagicRingPassOutcome {
                invisibility_applied: false,
                regeneration_applied: 1,
                vanished_ring: None,
            })
        );
        assert_eq!(state.party[0].hp, hp_before + 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(
            state.party_equipment[0][EQUIP_SLOT_RING],
            EQUIPMENT_ID_RING_REGENERATION as u8
        );
    }

    #[test]
    fn combat_player_command_handles_digits_pass_branches_and_xit_cleanup() {
        let mut state = combat_player_command_state(8, 5);

        let selected = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('1'), 1)
            .unwrap();
        assert_eq!(
            selected.action,
            CombatPlayerCommandAction::ActivePlayerSelection(
                CombatActivePlayerSelectionOutcome::SelectPartySlot(0)
            )
        );
        assert_eq!(state.active_player, Some(0));

        let pass = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(' '), 1)
            .unwrap();
        assert_eq!(
            pass.action,
            CombatPlayerCommandAction::Pass(CombatPassCommandOutcome {
                moves: false,
                attacks: false,
                ends_turn: true,
            })
        );

        let get = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('G'), 1)
            .unwrap();
        assert_eq!(
            get.action,
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::Get,
                live_actor_gate: CombatCommandLiveActorGate::Accepted,
            }
        );

        let blocked_xit = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('X'), 1)
            .unwrap();
        assert_eq!(
            blocked_xit.action,
            CombatPlayerCommandAction::XitCleanup { allowed: false }
        );
        assert_eq!(blocked_xit.control_after, CombatRoundLoopControl::ContinueActorWalk);

        state.combat_actors[8].mark_dead();
        let allowed_xit = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('X'), 1)
            .unwrap();
        assert_eq!(
            allowed_xit.action,
            CombatPlayerCommandAction::XitCleanup { allowed: true }
        );
        assert_eq!(
            allowed_xit.control_after,
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        );

        let invalid_direction = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(5), 1)
            .unwrap();
        assert_eq!(
            invalid_direction.action,
            CombatPlayerCommandAction::InvalidDirection { direction_code: 5 }
        );
    }

    #[test]
    fn combat_input_dispatch_routes_play_keys_to_combat_parser() {
        let game_dir = std::path::Path::new(".");
        let mut move_state = combat_player_command_state(8, 5);
        move_state.active_player = Some(0);

        assert_eq!(
            handle_play_key_input(&mut move_state, 'd', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!((move_state.combat_actors[0].x, move_state.combat_actors[0].y), (6, 5));
        assert_eq!(
            move_state.message,
            "Moved to (6, 5).\nGiant Rat moved to (7, 5)."
        );
        assert_eq!(move_state.pending_combat_actor_slot, Some(0));

        let mut attack_state = combat_player_command_state(6, 5);
        attack_state.active_player = Some(0);
        assert_eq!(
            handle_play_key_input(&mut attack_state, 'A', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            (attack_state.combat_actors[0].x, attack_state.combat_actors[0].y),
            (5, 5)
        );
        assert_eq!(
            attack_state.message,
            "Attack: no readied weapon.\nGiant Rat poisoned party member 1."
        );
        assert_eq!(attack_state.pending_combat_actor_slot, Some(0));

        let mut quit_state = combat_player_command_state(8, 5);
        assert_eq!(
            handle_play_key_input(&mut quit_state, 'q', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(quit_state.message, "Combat abandoned.");
        assert!(!quit_state.combat_active);
        assert_eq!(quit_state.pending_combat_actor_slot, None);
    }

    #[test]
    fn combat_input_dispatch_reports_weapon_hit_damage_and_xp() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(6, 5);
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
        state.party_strengths = vec![255];
        state.party_experience = vec![0];
        let mut expected_prng = state.prng_state;
        let _hit_roll = u5_prng_range_u16(&mut expected_prng, 0, u16::from(u8::MAX));
        let damage_roll = u5_prng_range_u16(&mut expected_prng, 0, u16::from(u8::MAX)) as u8;
        let expected_damage =
            combat_spell_damage_roll(damage_roll, equipment_attack_max(16).unwrap()) as u8;
        advance_expected_giant_rat_ai_input_prng(&mut expected_prng);

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.message,
            format!(
                "Hit Giant Rat for {expected_damage} damage with melee. Gained {expected_damage} XP.\nGiant Rat poisoned party member 1."
            )
        );
        assert_eq!(state.combat_actors[8].hp_or_wound, 10 - expected_damage);
        assert_eq!(state.party_experience[0], u16::from(expected_damage));
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.pending_combat_actor_slot, Some(0));
    }

    #[test]
    fn combat_input_dispatch_reports_weapon_kill_and_exits_combat() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(6, 5);
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
        state.party_strengths = vec![255];
        state.party_experience = vec![0];
        state.combat_actors[8].hp_or_wound = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.message,
            "Hit Giant Rat for 1 damage with melee. Giant Rat is defeated. Gained 3 XP."
        );
        assert_eq!(state.party_experience[0], 3);
        assert!(!state.combat_active);
        assert_eq!(state.pending_combat_actor_slot, None);
    }

    #[test]
    fn combat_input_dispatch_appends_monster_attack_round_summary() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(6, 5);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.message,
            "Pass.\nGiant Rat poisoned party member 1."
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.pending_combat_actor_slot, Some(0));
    }

    #[test]
    fn combat_input_dispatch_scene_abort_branches_use_visible_refusals_without_item_effects() {
        let game_dir = std::path::Path::new(".");
        let mut use_state = combat_player_command_state(8, 5);
        use_state.potion_stock[POTION_BLUE_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut use_state, 'U', "blue1", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(use_state.potion_stock[POTION_BLUE_INDEX], 1);
        assert!(use_state.active_use.is_none());
        assert_eq!(
            use_state.message,
            "Use-Not here!\nGiant Rat moved to (7, 5)."
        );
        assert!(use_state.combat_active);
    }

    #[test]
    fn combat_input_dispatch_applies_round_walker_defeat_exit() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(6, 5);
        state.combat_actors[8].owner_target_class = 33;
        state.party[0].status = b'G';
        state.party[0].hp = 1;
        state.party[0].max_hp = 20;
        state.prng_state = 0x0070;

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.message.starts_with("Pass.\nSkeleton "));
        assert!(!state.combat_active);
        assert_eq!(state.pending_combat_actor_slot, None);
        assert_eq!(state.combat_actors, [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]);
    }

    #[test]
    fn combat_input_dispatch_appends_monster_movement_round_summary() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Pass.\nGiant Rat moved to (7, 5).");
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
        assert_eq!(state.pending_combat_actor_slot, Some(0));
    }

    #[test]
    fn combat_input_dispatch_exit_control_restores_stored_frame_snapshot() {
        let game_dir = std::path::Path::new(".");
        let mut state = world_state(open_world_grid(), 10, 20);
        let original_player = state.player;
        let original_objects = state.active_objects.clone();
        let mut combat_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        combat_objects[0] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        combat_objects[8] = ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 8,
            y: 5,
            ..ActiveObject::empty()
        };
        let mut combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        combat_actors[8] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            8,
            0,
            8,
            5,
        ]);
        state.enter_combat_frame(combat_objects, combat_actors).unwrap();
        state.player.x = 99;

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Combat abandoned.");
        assert!(!state.combat_active);
        assert_eq!(state.combat_frame_snapshot, None);
        assert_eq!(state.player, original_player);
        assert_eq!(state.active_objects, original_objects);
        assert_eq!(
            state.combat_actors,
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]
        );
    }

    #[test]
    fn combat_input_dispatch_out_of_arena_move_restores_stored_frame_snapshot() {
        let game_dir = std::path::Path::new(".");
        let mut state = world_state(open_world_grid(), 10, 20);
        let original_player = state.player;
        let original_objects = state.active_objects.clone();
        let mut combat_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        combat_objects[0] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 10,
            y: 5,
            ..ActiveObject::empty()
        };
        let mut combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            10,
            5,
        ]);
        state.enter_combat_frame(combat_objects, combat_actors).unwrap();
        state.pending_combat_actor_slot = Some(0);
        state.player.x = 99;

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Leaving combat at (11, 5).");
        assert!(!state.combat_active);
        assert_eq!(state.combat_frame_snapshot, None);
        assert_eq!(state.player, original_player);
        assert_eq!(state.active_objects, original_objects);
        assert_eq!(
            state.combat_actors,
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]
        );
    }

    #[test]
    fn combat_input_dispatch_z_stats_binds_pending_actor_without_ending_turn() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.pending_combat_actor_slot = Some(0);
        state.next_combat_actor_slot = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.pending_combat_actor_slot, Some(0));
        assert_eq!(state.active_z_stats.as_ref().unwrap().selected_party_index, 0);
        assert!(state.message.starts_with("Z-stats: Stats page"));
    }

    #[test]
    fn combat_input_dispatch_z_stats_refuses_missing_or_disabled_actor() {
        let game_dir = std::path::Path::new(".");
        let mut missing = combat_player_command_state(8, 5);
        missing.combat_actors[0] = CombatActorDescriptor::empty();
        missing.pending_combat_actor_slot = Some(0);

        assert_eq!(
            handle_play_key_input(&mut missing, 'Z', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(missing.active_z_stats.is_none());
        assert_eq!(missing.pending_combat_actor_slot, None);
        assert_eq!(missing.message, "No active combatant.");

        let mut disabled = combat_player_command_state(8, 5);
        disabled.combat_actors[0].set_status_disabled();
        disabled.combat_actors[1] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            1,
            4,
            5,
        ]);
        disabled.pending_combat_actor_slot = Some(0);
        disabled.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        disabled.active_effect_counter = 3;
        disabled.prng_state = 0x1234;

        assert_eq!(
            handle_play_key_input(&mut disabled, 'Z', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(disabled.active_z_stats.is_none());
        assert_eq!(disabled.pending_combat_actor_slot, None);
        assert_eq!(disabled.message, "No active combatant.");
        assert_eq!(disabled.prng_state, 0x1234);

        let mut cast = combat_player_command_state(8, 5);
        cast.combat_actors[0].set_status_disabled();
        cast.combat_actors[1] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            1,
            4,
            5,
        ]);
        cast.pending_combat_actor_slot = Some(0);
        cast.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        cast.active_effect_counter = 3;
        cast.prng_state = 0x2345;

        assert_eq!(
            handle_play_key_input(&mut cast, 'C', "1IMX6", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(cast.pending_combat_actor_slot, None);
        assert_eq!(cast.message, "No active combatant.");
        assert_eq!(cast.prng_state, 0x2345);
    }

    #[test]
    fn combat_input_dispatch_ready_binds_pending_actor_to_picker() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.pending_combat_actor_slot = Some(0);
        state.next_combat_actor_slot = 1;
        state.equipment_stock[16] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'R', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.pending_combat_actor_slot, Some(0));
        assert_eq!(state.active_ready.as_ref().unwrap().selected_party_index, Some(0));
        assert!(state.message.starts_with("Ready: party member 1."));
    }

    #[test]
    fn combat_input_dispatch_yell_word_uses_combat_no_effect_route() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.pending_combat_actor_slot = Some(0);
        state.next_combat_actor_slot = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "FALLAX", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.message,
            "Yelled FALLAX. Nothing happens.\nGiant Rat moved to (7, 5)."
        );
        assert!(!state.message.contains("Word of Power"));
        assert!(state.active_ready.is_none());
        assert!(state.active_z_stats.is_none());
    }

    #[test]
    fn combat_input_dispatch_yell_prompt_keeps_same_actor_pending() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.pending_combat_actor_slot = Some(0);
        state.next_combat_actor_slot = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Yell what? Use Y<word>.");
        assert_eq!(state.pending_combat_actor_slot, Some(0));
    }

    #[test]
    fn combat_input_dispatch_uses_pending_round_walker_actor_slot() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.combat_actors[0].phase_counter = 3;
        state.active_objects[1] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 4,
            y: 5,
            ..ActiveObject::empty()
        };
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            1,
            4,
            5,
        ]);

        let walk = state.ensure_pending_combat_player_turn().unwrap();
        assert_eq!(walk.stop_reason, CombatRoundWalkStopReason::AwaitingPlayer);
        assert_eq!(state.pending_combat_actor_slot, Some(1));
        assert_eq!(state.next_combat_actor_slot, 2);

        assert_eq!(
            handle_play_key_input(&mut state, 's', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.combat_actors[1].x, state.combat_actors[1].y), (4, 6));
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (4, 6));
        assert_ne!((state.combat_actors[0].x, state.combat_actors[0].y), (4, 6));
        assert_eq!(
            state.message,
            "Moved to (4, 6).\nGiant Rat moved to (7, 5)."
        );
    }

    #[test]
    fn combat_input_dispatch_attack_prompt_keeps_pending_actor_for_direction() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "Attack-");
        assert_eq!(state.pending_combat_actor_slot, Some(0));

        assert_eq!(
            handle_play_key_input(&mut state, 'd', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!(
            state.message,
            "Moved to (6, 5).\nGiant Rat moved to (7, 5)."
        );
    }

    #[test]
    fn combat_input_dispatch_quickness_roll_can_consume_ready_actor() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.turn = 0;
        state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 3;

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Quickness!\nGiant Rat moved to (7, 5).");
        assert!(state.combat_active);
    }

    #[test]
    fn combat_input_dispatch_reports_magic_ring_vanish_message() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_INVISIBILITY as u8;
        state.prng_state = 0x0070;
        let mut expected_prng = state.prng_state;
        let vanish_roll = u5_prng_range_u16(&mut expected_prng, 0, 15);
        assert_eq!(vanish_roll, 0);
        advance_expected_giant_rat_ai_input_prng(&mut expected_prng);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.message,
            "Ring of Invisibility vanished.\nGiant Rat moved to (7, 5)."
        );
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
        assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.prng_state, expected_prng);
    }

    #[test]
    fn combat_input_dispatch_cast_uses_pending_actor_as_caster() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: INVISIBILITY_COST,
            hp: 20,
            max_hp: 20,
            level: INVISIBILITY_COST,
        });
        state.party[0].mana = 0;
        state.party[0].level = INVISIBILITY_COST;
        state.party_experience.push(0);
        state.party_strengths.push(20);
        state.party_intelligence.push(20);
        state.party_equipment = default_party_equipment(2);
        state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;
        state.active_objects[1] = ActiveObject {
            type_byte: 0x81,
            tile: 0x81,
            x: 4,
            y: 5,
            ..ActiveObject::empty()
        };
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            0,
            4,
            5,
        ]);
        state.pending_combat_actor_slot = Some(1);
        state.next_combat_actor_slot = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LS", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[1].mana, 0);
        assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
        assert!(state.combat_actors[1].is_hidden_or_unrevealed());
        assert_eq!(state.message, "Invisibility!\nGiant Rat moved to (7, 5).");
    }

    #[test]
    fn combat_input_dispatch_quickness_can_consume_cast_before_resources() {
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(8, 5);
        state.party[0].mana = INVISIBILITY_COST;
        state.party[0].level = INVISIBILITY_COST;
        state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;
        state.turn = 0;
        state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 3;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LS", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.message, "Quickness!\nGiant Rat moved to (7, 5).");
        assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, INVISIBILITY_COST);
        assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
        assert!(state.combat_active);
    }

    #[test]
    fn combat_actor_slot_dispatch_waits_when_phase_counter_is_not_ready() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].phase_counter = 3;
        state.combat_round_counter = 4;

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(
            application,
            CombatActorSlotDispatchApplication::Slot {
                slot: 8,
                phase_tick: Some(CombatActorPhaseTick::Waiting {
                    counter_before: 3,
                    counter_after: 2,
                }),
                action: CombatActorDispatchAction::Waiting,
                control_after: CombatRoundLoopControl::ContinueActorWalk,
            }
        );
        assert_eq!(state.combat_actors[8].phase_counter, 2);
        assert_eq!(state.combat_round_counter, 4);
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));
    }

    #[test]
    fn combat_actor_slot_dispatch_skips_actor_standing_on_blocked_arena_cell() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].phase_counter = 1;
        state.combat_round_counter = 4;
        state.combat_terrain[5][8] = 0;

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(
            application,
            CombatActorSlotDispatchApplication::Slot {
                slot: 8,
                phase_tick: Some(CombatActorPhaseTick::Inactive),
                action: CombatActorDispatchAction::Inactive,
                control_after: CombatRoundLoopControl::ContinueActorWalk,
            }
        );
        assert_eq!(state.combat_actors[8].phase_counter, 1);
        assert_eq!(state.combat_round_counter, 4);
    }

    #[test]
    fn combat_actor_slot_dispatch_runs_ready_monster_ai_after_phase_refresh() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].phase_counter = 1;
        state.combat_round_counter = 4;

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        let CombatActorSlotDispatchApplication::Slot {
            slot,
            phase_tick,
            action,
            control_after,
        } = application
        else {
            panic!("ready monster slot should dispatch");
        };
        assert_eq!(slot, 8);
        assert_eq!(
            phase_tick,
            Some(CombatActorPhaseTick::Ready {
                counter_before: 1,
                refreshed_counter: 29,
            })
        );
        let CombatActorDispatchAction::MonsterAi {
            ai_turn: Some(ai_turn),
        } = action
        else {
            panic!("ready monster slot should run AI");
        };
        assert_eq!(
            ai_turn.movement,
            Some(CombatAiMovementOutcome::Step {
                direction_code: 1,
                x: 7,
                y: 5,
            })
        );
        assert_eq!(ai_turn.monster_attack, None);
        assert_eq!(ai_turn.command_key, Some('W'));
        assert_eq!(control_after, CombatRoundLoopControl::ContinueActorWalk);
        assert_eq!(state.combat_actors[8].phase_counter, 29);
        assert_eq!(state.combat_round_counter, 5);
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    }

    #[test]
    fn combat_actor_slot_dispatch_routes_controlled_non_party_actor_to_player_path() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        state.combat_actors[8].phase_counter = 1;

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[(7, 5)],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(
            application,
            CombatActorSlotDispatchApplication::Slot {
                slot: 8,
                phase_tick: Some(CombatActorPhaseTick::Ready {
                    counter_before: 1,
                    refreshed_counter: 29,
                }),
                action: CombatActorDispatchAction::PlayerReady,
                control_after: CombatRoundLoopControl::ContinueActorWalk,
            }
        );
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));
    }

    #[test]
    fn combat_actor_slot_dispatch_applies_slot_matched_monster_attack_inputs() {
        let mut state = combat_ai_turn_state(6, 5);
        state.combat_actors[8].phase_counter = 1;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[
                (
                    7,
                    CombatMonsterAttackInputs {
                        party_defender_rating: 99,
                        forced_hit: Some(false),
                        ..CombatMonsterAttackInputs::default()
                    },
                ),
                (
                    8,
                    CombatMonsterAttackInputs {
                        party_defender_rating: 0,
                        damage_roll: 0,
                        forced_hit: Some(true),
                        ..CombatMonsterAttackInputs::default()
                    },
                ),
            ],
        );

        let CombatActorSlotDispatchApplication::Slot {
            action:
                CombatActorDispatchAction::MonsterAi {
                    ai_turn: Some(ai_turn),
                },
            ..
        } = application
        else {
            panic!("ready monster slot should run AI");
        };

        assert_eq!(ai_turn.attack_route, Some(CombatAiAttackRoute::Melee));
        assert!(matches!(
            ai_turn.monster_attack,
            Some(CombatMonsterAttackApplication {
                attacker_slot: 8,
                target_slot: 0,
                resolution: Some(CombatWeaponAttackResolution::Hit { .. }),
                ..
            })
        ));
        assert!(state.party[0].hp < 20);
    }

    #[test]
    fn combat_actor_slot_dispatch_reports_end_of_round_for_exhausted_slots() {
        let mut state = combat_ai_turn_state(8, 5);

        assert_eq!(
            state.apply_combat_actor_slot_dispatch_with_inputs(
                COMBAT_ACTOR_SLOTS,
                30,
                false,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[1, 2, 3, 4],
                &[],
            ),
            CombatActorSlotDispatchApplication::EndOfRound {
                control: CombatRoundLoopControl::StartNextRound,
            }
        );
    }

    #[test]
    fn combat_actor_slot_dispatch_sweeps_dead_party_before_phase_tick() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].phase_counter = 1;
        state.party[0].status = b'D';
        state.party[0].hp = 0;

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            0,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(
            application,
            CombatActorSlotDispatchApplication::Slot {
                slot: 0,
                phase_tick: None,
                action: CombatActorDispatchAction::PartyDeathSweep,
                control_after: CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat),
            }
        );
        assert!(state.combat_actors[0].is_marked_dead());
        assert_eq!(state.combat_actors[0].phase_counter, 1);
    }

    #[test]
    fn combat_round_walk_stops_when_player_slot_is_ready_for_input() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].phase_counter = 1;

        let application = state.apply_combat_round_walk_from_slot_with_inputs(
            0,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(application.start_slot, 0);
        assert_eq!(application.next_slot, 1);
        assert_eq!(
            application.stop_reason,
            CombatRoundWalkStopReason::AwaitingPlayer
        );
        assert_eq!(application.applications.len(), 1);
        assert!(matches!(
            application.applications[0],
            CombatActorSlotDispatchApplication::Slot {
                slot: 0,
                action: CombatActorDispatchAction::PlayerReady,
                ..
            }
        ));
    }

    #[test]
    fn combat_round_walk_continues_through_monster_ai_to_end_of_round() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].phase_counter = 1;

        let application = state.apply_combat_round_walk_from_slot_with_inputs(
            1,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(application.start_slot, 1);
        assert_eq!(application.next_slot, COMBAT_ACTOR_SLOTS);
        assert_eq!(application.stop_reason, CombatRoundWalkStopReason::EndOfRound);
        assert!(matches!(
            application.applications.last(),
            Some(CombatActorSlotDispatchApplication::EndOfRound {
                control: CombatRoundLoopControl::StartNextRound,
            })
        ));
        let monster_ai = application
            .applications
            .iter()
            .find_map(|entry| match entry {
                CombatActorSlotDispatchApplication::Slot {
                    slot: 8,
                    action:
                        CombatActorDispatchAction::MonsterAi {
                            ai_turn: Some(ai_turn),
                        },
                    ..
                } => Some(ai_turn),
                _ => None,
            })
            .expect("slot 8 should run monster AI during the table walk");
        assert_eq!(monster_ai.command_key, Some('W'));
        assert_eq!(monster_ai.monster_attack, None);
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    }

    #[test]
    fn combat_round_walk_decrements_sleep_durations_and_wakes_expired_actor() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].set_status_disabled();
        state.combat_sleep_durations[8] = 1;

        let application = state.apply_combat_round_walk_from_slot_with_inputs(
            1,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(application.stop_reason, CombatRoundWalkStopReason::EndOfRound);
        assert_eq!(state.combat_sleep_durations[8], 0);
        assert!(!state.combat_actors[8].is_status_disabled());
    }

    #[test]
    fn combat_round_walk_carries_monster_attack_inputs_through_dispatch() {
        let mut state = combat_ai_turn_state(6, 5);
        state.combat_actors[8].phase_counter = 1;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;

        let application = state.apply_combat_round_walk_from_slot_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[(
                8,
                CombatMonsterAttackInputs {
                    party_defender_rating: 0,
                    damage_roll: 0,
                    forced_hit: Some(true),
                    ..CombatMonsterAttackInputs::default()
                },
            )],
        );

        assert_eq!(application.stop_reason, CombatRoundWalkStopReason::EndOfRound);
        let monster_ai = application
            .applications
            .iter()
            .find_map(|entry| match entry {
                CombatActorSlotDispatchApplication::Slot {
                    slot: 8,
                    action:
                        CombatActorDispatchAction::MonsterAi {
                            ai_turn: Some(ai_turn),
                        },
                    ..
                } => Some(ai_turn),
                _ => None,
            })
            .expect("slot 8 should run monster AI during the table walk");
        assert!(monster_ai.monster_attack.is_some());
        assert!(state.party[0].hp < 20);
    }

    #[test]
    fn combat_round_walk_stops_on_exit_control_after_death_sweep() {
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].phase_counter = 1;
        state.party[0].status = b'D';
        state.party[0].hp = 0;

        let application = state.apply_combat_round_walk_from_slot_with_inputs(
            0,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(application.next_slot, 1);
        assert_eq!(application.stop_reason, CombatRoundWalkStopReason::Exit);
        assert_eq!(application.applications.len(), 1);
        assert!(matches!(
            application.applications[0],
            CombatActorSlotDispatchApplication::Slot {
                action: CombatActorDispatchAction::PartyDeathSweep,
                control_after: CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat),
                ..
            }
        ));
    }

    #[test]
    fn combat_ai_target_resolution_prefers_scan_target_then_cleanup_fallback() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[2] = CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 7]);
        actors[8] =
            CombatActorDescriptor::from_row([20, 1, 0, COMBAT_CLASS_DAEMON, 0, 0, 8, 8]);

        assert_eq!(
            resolve_combat_ai_target_after_scan(
                &mut actors,
                CombatTargetPick {
                    slot: Some(2),
                    first_five_party_slot_survived: false,
                },
                Some((9, 9)),
            ),
            CombatAiTargetResolution::ChosenActor {
                slot: 2,
                x: 4,
                y: 7,
            }
        );
        assert_eq!(actors[8].hp_or_wound, 20);

        assert_eq!(
            resolve_combat_ai_target_after_scan(
                &mut actors,
                CombatTargetPick {
                    slot: None,
                    first_five_party_slot_survived: false,
                },
                Some((6, 3)),
            ),
            CombatAiTargetResolution::CleanupFallback { x: 6, y: 3 }
        );
        assert_eq!(actors[8].hp_or_wound, 20);
    }

    #[test]
    fn combat_ai_center_fallback_marks_live_monster_side_slots_backwards() {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[4] =
            CombatActorDescriptor::from_row([25, 1, 0, COMBAT_CLASS_DAEMON, 0, 0, 1, 1]);
        actors[6] =
            CombatActorDescriptor::from_row([25, 1, 0, COMBAT_CLASS_DAEMON, 0, 0, 2, 2]);
        actors[9] = CombatActorDescriptor::from_row([
            25,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_PYTHON,
            0,
            0,
            3,
            3,
        ]);
        actors[10] =
            CombatActorDescriptor::from_row([10, 1, 0, COMBAT_CLASS_GIANT_RAT, 0, 0, 4, 4]);
        actors[12] = CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 5, 5]);
        actors[31] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            COMBAT_CLASS_PYTHON,
            0,
            0,
            6,
            6,
        ]);

        assert_eq!(
            resolve_combat_ai_target_after_scan(
                &mut actors,
                CombatTargetPick {
                    slot: None,
                    first_five_party_slot_survived: false,
                },
                None,
            ),
            CombatAiTargetResolution::CenterFallback {
                x: COMBAT_ARENA_CENTER_COORDINATE,
                y: COMBAT_ARENA_CENTER_COORDINATE,
                critical_hp_flee_slots: vec![31, 10, 6],
            }
        );
        assert_eq!(combat_ai_center_fallback_target(), (5, 5));
        assert_eq!(actors[4].hp_or_wound, 25);
        assert_eq!(
            actors[6].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().max_hp)
        );
        assert_eq!(actors[9].hp_or_wound, 25);
        assert_eq!(
            actors[10].hp_or_wound,
            cause_fear_forced_current_hp(
                combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp
            )
        );
        assert_eq!(actors[12].hp_or_wound, 10);
        assert_eq!(
            actors[31].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
        );

        assert_eq!(
            resolve_combat_ai_target_after_scan(
                &mut actors,
                CombatTargetPick {
                    slot: None,
                    first_five_party_slot_survived: true,
                },
                None,
            ),
            CombatAiTargetResolution::NoUsableTarget
        );
    }

    fn combat_ai_legal_grid() -> [[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] {
        [[true; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    }

    #[test]
    fn combat_ai_movement_accepts_legal_teleport_before_ordinary_step() {
        let mut legal = combat_ai_legal_grid();
        legal[9][9] = true;

        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                5,
                5,
                CombatStepVector { dx: 1, dy: 0 },
                true,
                Some((9, 9)),
                true,
                &[1, 2, 3, 4],
            ),
            CombatAiMovementOutcome::Teleport { x: 9, y: 9 }
        );

        legal[9][9] = false;
        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                5,
                5,
                CombatStepVector { dx: 1, dy: 0 },
                true,
                Some((9, 9)),
                true,
                &[1, 2, 3, 4],
            ),
            CombatAiMovementOutcome::Step {
                direction_code: 2,
                x: 6,
                y: 5,
            }
        );
    }

    #[test]
    fn combat_ai_legal_cell_mask_layers_actor_occupancy_over_terrain_passability() {
        let mut terrain = [[0x01; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        terrain[1][1] = 0xff;
        terrain[9][9] = 0x01;
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            2,
            2,
        ]);
        actors[6] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            COMBAT_CLASS_DAEMON,
            0,
            0,
            3,
            3,
        ]);
        actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_PYTHON,
            0,
            0,
            4,
            4,
        ]);
        actors[8] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            0,
            0,
            99,
            99,
        ]);

        assert!(combat_actor_occupies_arena_cell(actors[0], 2, 2));
        assert!(!combat_actor_occupies_arena_cell(actors[7], 4, 4));

        let legal = build_combat_ai_legal_cell_mask(&terrain, &actors, |tile| tile != 0xff);

        assert!(legal[0][0]);
        assert!(!legal[1][1]);
        assert!(!legal[2][2]);
        assert!(!legal[3][3]);
        assert!(legal[4][4]);
        assert!(legal[9][9]);
    }

    #[test]
    fn combat_ai_movement_uses_axis_priority_then_random_cardinal_fallback() {
        let mut legal = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        legal[4][5] = true;
        legal[5][6] = true;
        assert_eq!(combat_direction_code_for_step(-1, 0), Some(1));
        assert_eq!(combat_direction_code_for_step(1, 1), None);

        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                5,
                5,
                CombatStepVector { dx: 1, dy: -1 },
                false,
                Some((4, 4)),
                true,
                &[4, 1, 3, 2],
            ),
            CombatAiMovementOutcome::Step {
                direction_code: 2,
                x: 6,
                y: 5,
            }
        );

        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                5,
                5,
                CombatStepVector { dx: 1, dy: -1 },
                false,
                Some((4, 4)),
                false,
                &[4, 1, 3, 2],
            ),
            CombatAiMovementOutcome::Step {
                direction_code: 3,
                x: 5,
                y: 4,
            }
        );

        legal[4][5] = false;
        legal[5][6] = false;
        legal[6][5] = true;
        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                5,
                5,
                CombatStepVector { dx: 1, dy: -1 },
                false,
                None,
                true,
                &[2, 5, 4, 1, 3],
            ),
            CombatAiMovementOutcome::Step {
                direction_code: 4,
                x: 5,
                y: 6,
            }
        );
    }

    #[test]
    fn combat_ai_movement_commit_updates_actor_and_linked_active_object_position() {
        let mut actor = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_GIANT_RAT,
            4,
            0,
            2,
            3,
        ]);
        let mut active_objects = vec![ActiveObject::empty(); 8];
        active_objects[4] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 2,
            y: 3,
            z: 0,
            phase: 1,
            aux1: 2,
            aux3: 3,
        };

        let outcome = commit_combat_ai_movement_outcome(
            &mut actor,
            &mut active_objects,
            CombatAiMovementOutcome::Step {
                direction_code: 2,
                x: 6,
                y: 3,
            },
        )
        .unwrap();

        assert_eq!(
            outcome,
            CombatLinkedPositionCommitOutcome {
                active_object_slot: 4,
                actor_position_before: (2, 3),
                actor_position_after: (6, 3),
                active_object_position_before: Some((2, 3)),
                active_object_position_after: Some((6, 3)),
            }
        );
        assert_eq!((actor.x, actor.y), (6, 3));
        assert_eq!((active_objects[4].x, active_objects[4].y), (6, 3));
        assert_eq!(active_objects[4].tile, 0xc0);
    }

    #[test]
    fn combat_ai_movement_commit_reports_missing_linked_active_object_slot() {
        let mut actor = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_BAT,
            9,
            0,
            4,
            4,
        ]);
        let mut active_objects = vec![ActiveObject::empty(); 2];

        let outcome = commit_combat_ai_movement_outcome(
            &mut actor,
            &mut active_objects,
            CombatAiMovementOutcome::Teleport { x: 8, y: 1 },
        )
        .unwrap();

        assert_eq!(outcome.active_object_slot, 9);
        assert_eq!(outcome.actor_position_before, (4, 4));
        assert_eq!(outcome.actor_position_after, (8, 1));
        assert_eq!(outcome.active_object_position_before, None);
        assert_eq!(outcome.active_object_position_after, None);
        assert_eq!((actor.x, actor.y), (8, 1));
    }

    #[test]
    fn combat_ai_movement_commit_skips_blocked_or_inactive_actors() {
        let mut actor = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_PYTHON,
            1,
            0,
            5,
            5,
        ]);
        let mut active_objects = vec![ActiveObject::empty(); 2];
        active_objects[1].x = 5;
        active_objects[1].y = 5;

        assert_eq!(
            commit_combat_ai_movement_outcome(
                &mut actor,
                &mut active_objects,
                CombatAiMovementOutcome::Blocked { surrounded: true },
            ),
            None
        );
        assert_eq!((actor.x, actor.y), (5, 5));
        assert_eq!((active_objects[1].x, active_objects[1].y), (5, 5));

        actor.flags |= COMBAT_ACTOR_FLAG_MARKED_DEAD;
        assert_eq!(
            commit_combat_actor_linked_position(&mut actor, &mut active_objects, 6, 5),
            None
        );
        assert_eq!((actor.x, actor.y), (5, 5));
        assert_eq!((active_objects[1].x, active_objects[1].y), (5, 5));
    }

    #[test]
    fn combat_ai_movement_reports_surrounded_blocked_cells() {
        let mut legal = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        legal[0][0] = true;
        legal[0][1] = true;
        assert!(combat_ai_legal_cell(&legal, 0, 0));
        assert!(!combat_ai_legal_cell(&legal, -1, 0));
        assert!(!combat_ai_cardinal_neighbors_surrounded(&legal, 0, 0));

        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                0,
                0,
                CombatStepVector { dx: -1, dy: -1 },
                false,
                None,
                true,
                &[1, 3],
            ),
            CombatAiMovementOutcome::Blocked { surrounded: false }
        );

        legal[0][1] = false;
        assert!(combat_ai_cardinal_neighbors_surrounded(&legal, 0, 0));
        assert_eq!(
            resolve_combat_ai_movement(
                &legal,
                0,
                0,
                CombatStepVector { dx: -1, dy: -1 },
                false,
                None,
                true,
                &[1, 3],
            ),
            CombatAiMovementOutcome::Blocked { surrounded: true }
        );
    }

    #[test]
    fn combat_wound_morale_uses_quarter_buckets_and_documented_roll_rate() {
        assert_eq!(
            resolve_combat_wound_morale(24, 99, 0),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::UnderOneQuarter,
                fleeing: true,
            }
        );
        assert_eq!(
            resolve_combat_wound_morale(25, 99, 251),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::OneQuarterToUnderHalf,
                fleeing: true,
            }
        );
        assert_eq!(
            resolve_combat_wound_morale(49, 99, 252),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::OneQuarterToUnderHalf,
                fleeing: false,
            }
        );
        assert_eq!(
            resolve_combat_wound_morale(50, 99, 255),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::HalfToUnderThreeQuarters,
                fleeing: false,
            }
        );
        assert_eq!(
            resolve_combat_wound_morale(75, 99, 255),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::ThreeQuartersOrMore,
                fleeing: false,
            }
        );
    }

    #[test]
    fn combat_wound_morale_can_resolve_class_max_hp() {
        assert_eq!(
            resolve_combat_wound_morale_for_class(4, 32, 0).unwrap(),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::OneQuarterToUnderHalf,
                fleeing: true,
            }
        );
        assert_eq!(
            resolve_combat_wound_morale_for_class(4, 32, 252).unwrap(),
            CombatWoundMorale {
                bucket: CombatWoundScoreBucket::OneQuarterToUnderHalf,
                fleeing: false,
            }
        );
        assert_eq!(resolve_combat_wound_morale_for_class(1, 11, 0), None);
    }

    #[test]
    fn combat_party_damage_clamps_miss_and_uses_saturating_hp_counter() {
        let mut member = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };

        let miss = apply_combat_party_damage(&mut member, -1);
        assert!(miss.missed);
        assert!(!miss.instant_kill);
        assert!(!miss.killed);
        assert_eq!(miss.applied_damage, 0);
        assert_eq!(miss.status_before, b'G');
        assert_eq!(miss.status_after, b'G');
        assert_eq!(member.hp, 12);
        assert_eq!(member.status, b'G');

        let hit = apply_combat_party_damage(&mut member, 5);
        assert!(!hit.missed);
        assert_eq!(hit.applied_damage, 5);
        assert!(!hit.killed);
        assert_eq!(member.hp, 7);
        assert_eq!(member.status, b'G');

        let death = apply_combat_party_damage(&mut member, 30);
        assert_eq!(death.applied_damage, 7);
        assert!(death.killed);
        assert_eq!(death.status_after, b'D');
        assert_eq!(member.hp, 0);
        assert_eq!(member.status, b'D');
    }

    #[test]
    fn combat_party_damage_instant_kill_forces_death_status() {
        let mut member = PartyMember {
            slot: 1,
            class_byte: 1,
            status: b'S',
            climb_stat: 0,
            mana: 0,
            hp: 300,
            max_hp: 400,
            level: 1,
        };

        let kill = apply_combat_party_damage(&mut member, COMBAT_INSTANT_KILL_DAMAGE);

        assert!(kill.instant_kill);
        assert!(kill.killed);
        assert_eq!(kill.applied_damage, 300);
        assert_eq!(kill.status_before, b'S');
        assert_eq!(kill.status_after, b'D');
        assert_eq!(member.hp, 0);
        assert_eq!(member.status, b'D');
    }

    #[test]
    fn combat_party_damage_state_wrapper_clears_only_dead_active_player() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
        ];
        state.active_player = Some(1);
        state.active_objects.resize(2, ActiveObject::empty());
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4]);
        state.combat_actors[1] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 5, 4]);
        state.active_objects[0].type_byte = PLAYER_TILE;
        state.active_objects[0].tile = PLAYER_TILE;
        state.active_objects[1].type_byte = PLAYER_TILE;
        state.active_objects[1].tile = PLAYER_TILE;

        let nonlethal = state.apply_combat_party_damage_to_slot(1, 5).unwrap();
        assert!(!nonlethal.killed);
        assert_eq!(state.party[1].hp, 7);
        assert_eq!(state.active_player, Some(1));
        assert_eq!(state.active_objects[1].tile, PLAYER_TILE);

        let inactive_death = state.apply_combat_party_damage_to_slot(0, 20).unwrap();
        assert!(inactive_death.killed);
        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.active_player, Some(1));
        assert_eq!(state.active_objects[0].tile, COMBAT_PARTY_CORPSE_TILE);

        let active_death = state.apply_combat_party_damage_to_slot(1, 20).unwrap();
        assert!(active_death.killed);
        assert_eq!(state.party[1].status, b'D');
        assert_eq!(state.active_player, None);
        assert_eq!(state.active_objects[1].tile, COMBAT_PARTY_CORPSE_TILE);
        assert_eq!(state.apply_combat_party_damage_to_slot(7, 1), None);
    }

    #[test]
    fn combat_party_attacker_experience_credit_requires_living_party_slot_and_caps() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let living = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };
        state.party = vec![
            living,
            PartyMember {
                slot: 1,
                status: b'D',
                hp: 0,
                ..living
            },
        ];
        state.party_experience = Vec::new();

        assert_eq!(apply_combat_experience_reward(10, 25), 35);
        assert_eq!(
            state.credit_combat_party_attacker_experience(0, 25),
            Some(25)
        );
        assert_eq!(state.party_experience, vec![25, 0]);
        state.party_experience[0] = COMBAT_EXPERIENCE_CAP - 1;
        assert_eq!(
            state.credit_combat_party_attacker_experience(0, 25),
            Some(COMBAT_EXPERIENCE_CAP)
        );
        assert_eq!(state.credit_combat_party_attacker_experience(1, 25), None);
        assert_eq!(state.party_experience[1], 0);
        assert_eq!(state.credit_combat_party_attacker_experience(7, 25), None);
    }

    #[test]
    fn combat_weapon_damage_application_routes_party_targets_through_party_damage() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
        ];
        state.active_player = Some(1);
        state.party_experience = vec![10, 20];

        assert_eq!(
            state.apply_combat_weapon_damage_to_target(Some(0), 1, 12, false),
            Some(CombatWeaponDamageApplication::Party {
                target_slot: 1,
                damage: CombatPartyDamageOutcome {
                    raw_damage: 12,
                    applied_damage: 12,
                    missed: false,
                    instant_kill: false,
                    killed: true,
                    status_before: b'G',
                    status_after: b'D',
                },
            })
        );
        assert_eq!(state.party[1].hp, 0);
        assert_eq!(state.active_player, None);
        assert_eq!(state.party_experience, vec![10, 20]);
        assert_eq!(
            state.apply_combat_weapon_damage_to_target(Some(0), 5, 1, false),
            None
        );
    }

    #[test]
    fn combat_weapon_damage_application_routes_monster_targets_and_credits_party_attacker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let living = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };
        state.party = vec![
            living,
            PartyMember {
                slot: 1,
                status: b'D',
                hp: 0,
                ..living
            },
        ];
        state.party_experience = vec![10, 20];
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

        assert_eq!(
            state.apply_combat_weapon_damage_to_target(Some(0), target_slot, 4, false),
            Some(CombatWeaponDamageApplication::Monster {
                target_slot,
                damage: CombatMonsterDamageOutcome {
                    class: 32,
                    raw_damage: 4,
                    applied_damage: 4,
                    missed: false,
                    instant_kill: false,
                    killed: false,
                    return_value: 4,
                    death_path: None,
                },
                credited_experience: Some(14),
            })
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, 6);
        assert_eq!(state.party_experience, vec![14, 20]);

        assert_eq!(
            state.apply_combat_weapon_damage_to_target(Some(1), target_slot, 2, false),
            Some(CombatWeaponDamageApplication::Monster {
                target_slot,
                damage: CombatMonsterDamageOutcome {
                    class: 32,
                    raw_damage: 2,
                    applied_damage: 2,
                    missed: false,
                    instant_kill: false,
                    killed: false,
                    return_value: 2,
                    death_path: None,
                },
                credited_experience: None,
            })
        );
        assert_eq!(state.party_experience, vec![14, 20]);

        assert_eq!(
            state.apply_combat_weapon_damage_to_target(
                Some(0),
                target_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                false
            ),
            Some(CombatWeaponDamageApplication::Monster {
                target_slot,
                damage: CombatMonsterDamageOutcome {
                    class: 32,
                    raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                    applied_damage: 4,
                    missed: false,
                    instant_kill: true,
                    killed: true,
                    return_value: stats.reward_unit(),
                    death_path: Some(CombatMonsterDeathPath::DefaultDropCheck),
                },
                credited_experience: Some(14 + u16::from(stats.reward_unit())),
            })
        );
        assert!(state.combat_actors[target_slot].is_marked_dead());
        assert_eq!(
            state.party_experience[0],
            14 + u16::from(stats.reward_unit())
        );
        assert_eq!(
            state.apply_combat_weapon_damage_to_target(Some(0), COMBAT_ACTOR_SLOTS, 1, false),
            None
        );
    }

    fn seed_for_default_death_gates(
        drop_cap: u8,
        first_accepts: bool,
        second_accepts: bool,
    ) -> u16 {
        for seed in 0..=u16::MAX {
            let mut prng = seed;
            let first = u5_prng_range_u16(
                &mut prng,
                0,
                u16::from(COMBAT_DEFAULT_DEATH_DROP_ROLL_MAX),
            ) as u8;
            let second = u5_prng_range_u16(
                &mut prng,
                0,
                u16::from(COMBAT_DEFAULT_DEATH_DROP_ROLL_MAX),
            ) as u8;
            if combat_default_death_drop_gate_accepts(drop_cap, first) == first_accepts
                && combat_default_death_drop_gate_accepts(drop_cap, second) == second_accepts
            {
                return seed;
            }
        }
        panic!("no deterministic PRNG seed found for requested default-death gates");
    }

    fn place_death_side_effect_monster(
        state: &mut PlayState,
        class: u8,
        actor_slot: usize,
        active_object_slot: usize,
    ) -> CombatClassStats {
        let stats = combat_class_stats(class).unwrap();
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());
        state.combat_actors[actor_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            active_object_slot as u8,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        state.active_objects[active_object_slot] = ActiveObject {
            type_byte: COMBAT_DEFAULT_DEATH_DROP_TILE + class,
            tile: COMBAT_DEFAULT_DEATH_DROP_TILE + class,
            x: 4,
            y: 5,
            z: 0,
            phase: 7,
            aux1: 0x55,
            aux3: 0,
        };
        stats
    }

    #[test]
    fn combat_monster_default_death_materializes_drop_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 9;
        let stats = place_death_side_effect_monster(&mut state, 32, actor_slot, active_object_slot);
        state.prng_state = seed_for_default_death_gates(stats.default_drop_cap, true, false);

        let application = state
            .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();

        assert!(matches!(
            application,
            CombatWeaponDamageApplication::Monster { damage, .. }
                if damage.death_path == Some(CombatMonsterDeathPath::DefaultDropCheck)
        ));
        assert!(state.combat_actors[actor_slot].is_marked_dead());
        assert_eq!(
            state.active_objects[active_object_slot].type_byte,
            COMBAT_DEFAULT_DEATH_DROP_TILE
        );
        assert_eq!(
            state.active_objects[active_object_slot].tile,
            COMBAT_DEFAULT_DEATH_DROP_TILE
        );
        assert_eq!(
            state.active_objects[active_object_slot].aux1,
            stats.default_drop_cap
        );
        assert_eq!(state.active_objects[active_object_slot].phase, STEADY_PHASE);
    }

    #[test]
    fn combat_monster_default_death_materializes_no_drop_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 10;
        let stats = place_death_side_effect_monster(&mut state, 32, actor_slot, active_object_slot);
        state.prng_state = seed_for_default_death_gates(stats.default_drop_cap, false, true);

        state
            .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();

        assert!(state.combat_actors[actor_slot].is_marked_dead());
        assert_eq!(
            state.active_objects[active_object_slot].type_byte,
            COMBAT_DEFAULT_DEATH_NO_DROP_TILE
        );
        assert_eq!(
            state.active_objects[active_object_slot].tile,
            COMBAT_DEFAULT_DEATH_NO_DROP_TILE
        );
        assert_eq!(state.active_objects[active_object_slot].aux1, 0);
    }

    #[test]
    fn combat_monster_vanish_death_clears_actor_and_updates_visual_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 11;
        place_death_side_effect_monster(&mut state, 13, actor_slot, active_object_slot);

        let application = state
            .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();

        assert!(matches!(
            application,
            CombatWeaponDamageApplication::Monster { damage, .. }
                if damage.death_path == Some(CombatMonsterDeathPath::Vanish)
        ));
        assert!(state.combat_actors[actor_slot].is_empty());
        assert_eq!(
            state.active_objects[active_object_slot].type_byte,
            COMBAT_VANISH_DEATH_MARKER_TILE
        );
        assert_eq!(
            state.active_objects[active_object_slot].tile,
            COMBAT_VANISH_DEATH_MARKER_TILE
        );
        assert_eq!(state.active_objects[active_object_slot].aux1, 0);
        assert_eq!(state.active_objects[active_object_slot].phase, STEADY_PHASE);
    }

    #[test]
    fn combat_monster_gazer_death_writes_eye_burst_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 12;
        place_death_side_effect_monster(&mut state, 28, actor_slot, active_object_slot);

        let application = state
            .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();

        assert!(matches!(
            application,
            CombatWeaponDamageApplication::Monster { damage, .. }
                if damage.death_path == Some(CombatMonsterDeathPath::SpecialTileTransition)
        ));
        assert!(state.combat_actors[actor_slot].is_marked_dead());
        assert_eq!(
            state.active_objects[active_object_slot].type_byte,
            COMBAT_GAZER_DEATH_MARKER_TILE
        );
        assert_eq!(
            state.active_objects[active_object_slot].tile,
            COMBAT_GAZER_DEATH_MARKER_TILE
        );
        assert_eq!(state.active_objects[active_object_slot].aux1, 0);
        assert_eq!(state.active_objects[active_object_slot].phase, STEADY_PHASE);
    }

    #[test]
    fn combat_monster_gargoyle_death_leaves_lava_then_default_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 13;
        let stats = place_death_side_effect_monster(&mut state, 30, actor_slot, active_object_slot);
        state.prng_state = seed_for_default_death_gates(stats.default_drop_cap, false, false);

        let application = state
            .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();

        assert!(matches!(
            application,
            CombatWeaponDamageApplication::Monster { damage, .. }
                if damage.death_path == Some(CombatMonsterDeathPath::SpecialTileTransition)
        ));
        assert!(state.combat_actors[actor_slot].is_marked_dead());
        assert_eq!(state.combat_terrain[5][4], COMBAT_GARGOYLE_DEATH_TERRAIN_TILE);
        assert_eq!(
            state.active_objects[active_object_slot].type_byte,
            COMBAT_DEFAULT_DEATH_DROP_TILE
        );
        assert_eq!(
            state.active_objects[active_object_slot].tile,
            COMBAT_DEFAULT_DEATH_DROP_TILE
        );
        assert_eq!(state.active_objects[active_object_slot].aux1, 0);
        assert_eq!(state.active_objects[active_object_slot].phase, STEADY_PHASE);
    }

    #[test]
    fn combat_weapon_attack_application_uses_actor_range_and_applies_hit_damage() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.party_experience = vec![10];
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 8, 4, 0, 0);

        assert_eq!(
            state.resolve_and_apply_combat_equipment_weapon_attack(
                17,
                0,
                target_slot,
                30,
                10,
                0,
                5,
                None,
                false,
            ),
            Some(CombatWeaponAttackApplication {
                resolution: CombatWeaponAttackResolution::Hit {
                    route: CombatWeaponAttackRangeRoute::Ranged { effect_code: 7 },
                    raw_damage: 6,
                },
                damage_application: Some(CombatWeaponDamageApplication::Monster {
                    target_slot,
                    damage: CombatMonsterDamageOutcome {
                        class: 32,
                        raw_damage: 6,
                        applied_damage: 6,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        return_value: 6,
                        death_path: None,
                    },
                    credited_experience: Some(16),
                }),
            })
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, 4);
        assert_eq!(state.party_experience[0], 16);
    }

    #[test]
    fn combat_weapon_attack_application_leaves_state_unchanged_for_no_hit_routes() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 8, 4, 0, 0);

        assert_eq!(
            state.resolve_and_apply_combat_equipment_weapon_attack(
                16,
                0,
                target_slot,
                30,
                10,
                0,
                5,
                None,
                false,
            ),
            Some(CombatWeaponAttackApplication {
                resolution: CombatWeaponAttackResolution::OutOfRange {
                    target_range: 4,
                    range_cap: 3,
                },
                damage_application: None,
            })
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);

        state.combat_actors[target_slot].x = 5;
        assert_eq!(
            state.resolve_and_apply_combat_equipment_weapon_attack(
                16,
                0,
                target_slot,
                30,
                10,
                25,
                5,
                None,
                false,
            ),
            Some(CombatWeaponAttackApplication {
                resolution: CombatWeaponAttackResolution::Miss {
                    route: CombatWeaponAttackRangeRoute::Melee,
                    hit_score: 25,
                },
                damage_application: None,
            })
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
        assert_eq!(
            state.resolve_and_apply_combat_equipment_weapon_attack(
                EQUIPMENT_COUNT,
                0,
                target_slot,
                30,
                10,
                0,
                5,
                None,
                false,
            ),
            None
        );
        assert_eq!(
            state.resolve_and_apply_combat_equipment_weapon_attack(
                16,
                0,
                COMBAT_ACTOR_SLOTS,
                30,
                10,
                0,
                5,
                None,
                false,
            ),
            None
        );
    }

    fn combat_monster_attack_state(attacker_class: u8, attacker_x: u8, attacker_y: u8) -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            attacker_class,
            8,
            0,
            attacker_x,
            attacker_y,
        ]);
        state
    }

    #[test]
    fn combat_monster_attack_applies_poison_status_before_ordinary_melee_damage() {
        let mut state = combat_monster_attack_state(COMBAT_CLASS_GIANT_SPIDER, 6, 5);

        let application = state
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 0, 7, true, 8, Some(true))
            .unwrap();

        assert_eq!(
            application,
            CombatMonsterAttackApplication {
                attacker_slot: 8,
                target_slot: 0,
                poison_status_outcome: Some(
                    CombatPoisonStatusAttackOutcome::PoisonedPartyMember {
                        status_before: b'G',
                        status_after: b'P',
                    }
                ),
                resolution: None,
                damage_application: None,
            }
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 12);
    }

    #[test]
    fn combat_monster_attack_poison_branch_falls_back_to_damage_for_non_good_party() {
        let mut state = combat_monster_attack_state(COMBAT_CLASS_GIANT_SPIDER, 6, 5);
        state.party[0].status = b'P';

        let application = state
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 0, 7, true, 8, Some(true))
            .unwrap();

        assert_eq!(
            application,
            CombatMonsterAttackApplication {
                attacker_slot: 8,
                target_slot: 0,
                poison_status_outcome: Some(CombatPoisonStatusAttackOutcome::FallbackDamage {
                    raw_damage: 9
                }),
                resolution: None,
                damage_application: Some(CombatWeaponDamageApplication::Party {
                    target_slot: 0,
                    damage: CombatPartyDamageOutcome {
                        raw_damage: 9,
                        applied_damage: 9,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        status_before: b'P',
                        status_after: b'P',
                    },
                }),
            }
        );
        assert_eq!(state.party[0].hp, 3);
        assert_eq!(state.party[0].status, b'P');
    }

    #[test]
    fn combat_monster_attack_gate_rejection_uses_ordinary_melee_hit_resolution() {
        let mut state = combat_monster_attack_state(COMBAT_CLASS_GIANT_SPIDER, 6, 5);

        let application = state
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 255, 1, false, 8, Some(true))
            .unwrap();

        assert_eq!(
            application,
            CombatMonsterAttackApplication {
                attacker_slot: 8,
                target_slot: 0,
                poison_status_outcome: Some(CombatPoisonStatusAttackOutcome::GateRejected),
                resolution: Some(CombatWeaponAttackResolution::Hit {
                    route: CombatWeaponAttackRangeRoute::Melee,
                    raw_damage: 2,
                }),
                damage_application: Some(CombatWeaponDamageApplication::Party {
                    target_slot: 0,
                    damage: CombatPartyDamageOutcome {
                        raw_damage: 2,
                        applied_damage: 2,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        status_before: b'G',
                        status_after: b'G',
                    },
                }),
            }
        );
        assert_eq!(state.party[0].hp, 10);
        assert_eq!(state.party[0].status, b'G');
    }

    #[test]
    fn combat_monster_attack_uses_ranged_effect_route_for_in_range_non_adjacent_targets() {
        let mut state = combat_monster_attack_state(COMBAT_CLASS_DRAGON, 8, 5);

        let application = state
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 255, 4, true, 8, Some(true))
            .unwrap();

        assert_eq!(
            application,
            CombatMonsterAttackApplication {
                attacker_slot: 8,
                target_slot: 0,
                poison_status_outcome: None,
                resolution: Some(CombatWeaponAttackResolution::Hit {
                    route: CombatWeaponAttackRangeRoute::Ranged { effect_code: 3 },
                    raw_damage: 5,
                }),
                damage_application: Some(CombatWeaponDamageApplication::Party {
                    target_slot: 0,
                    damage: CombatPartyDamageOutcome {
                        raw_damage: 5,
                        applied_damage: 5,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        status_before: b'G',
                        status_after: b'G',
                    },
                }),
            }
        );
        assert_eq!(state.party[0].hp, 7);
    }

    #[test]
    fn active_target_spell_damage_application_applies_defense_and_credits_caster() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.party_experience = vec![10];
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);

        assert_eq!(
            state.apply_active_target_combat_spell_damage(
                Some(0),
                target_slot,
                CombatSpellDamageKind::MagicMissile,
                15,
                11,
            ),
            Some(CombatActiveTargetSpellDamageApplication {
                kind: CombatSpellDamageKind::MagicMissile,
                raw_damage: 5,
                damage_application: CombatWeaponDamageApplication::Monster {
                    target_slot,
                    damage: CombatMonsterDamageOutcome {
                        class: 32,
                        raw_damage: 5,
                        applied_damage: 5,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        return_value: 5,
                        death_path: None,
                    },
                    credited_experience: Some(15),
                },
            })
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, 5);
        assert_eq!(state.party_experience[0], 15);
    }

    #[test]
    fn combat_cast_active_target_spell_routes_resources_damage_and_xp() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 1,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.party_experience = vec![10];
        state.prng_state = 0x1234;
        let spell_index = spell_index_from_code("GP").unwrap();
        state.spell_charges[spell_index] = 1;
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

        let mut expected_prng = state.prng_state;
        let raw_roll = u5_prng_range_u16(
            &mut expected_prng,
            0,
            u16::from(COMBAT_MAGIC_MISSILE_DAMAGE_ROLL_MAX - 1),
        ) as u8;
        let defense_roll =
            u5_prng_range_u16(&mut expected_prng, 0, u16::from(stats.defense)) as u8;
        let expected_damage = resolve_active_target_spell_damage(
            CombatSpellDamageKind::MagicMissile,
            raw_roll,
            defense_roll,
        )
        .unwrap();

        assert_eq!(
            state
                .cast_spell_from_suffix("1GP7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.message, "Magic Missile!");
        assert_eq!(
            i16::from(stats.max_hp) - i16::from(state.combat_actors[target_slot].hp_or_wound),
            expected_damage.max(0)
        );
        assert_eq!(
            state.party_experience[0],
            10 + u16::try_from(expected_damage.max(0)).unwrap()
        );
    }

    #[test]
    fn combat_cast_active_target_spell_gates_target_and_negate_magic_before_resources() {
        let mut missing_target = world_state(open_world_grid(), 10, 20);
        missing_target.combat_active = true;
        let spell_index = spell_index_from_code("GP").unwrap();
        missing_target.spell_charges[spell_index] = 1;
        missing_target.party[0].mana = 1;
        missing_target.party[0].level = 1;

        assert_eq!(
            missing_target
                .cast_spell_from_suffix("1GP7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(missing_target.spell_charges[spell_index], 1);
        assert_eq!(missing_target.party[0].mana, 1);
        assert_eq!(missing_target.turn, 0);
        assert_eq!(
            missing_target.message,
            "Target? Use C1GP7 to target a live combat slot."
        );

        let mut absorbed = world_state(open_world_grid(), 10, 20);
        absorbed.combat_active = true;
        absorbed.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
        absorbed.spell_charges[spell_index] = 1;
        absorbed.party[0].mana = 1;
        absorbed.party[0].level = 1;

        assert_eq!(
            absorbed
                .cast_spell_from_suffix("1GP7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(absorbed.spell_charges[spell_index], 1);
        assert_eq!(absorbed.party[0].mana, 1);
        assert_eq!(absorbed.turn, 0);
        assert_eq!(absorbed.message, "Magic absorbed!");
    }

    #[test]
    fn active_combat_cast_target_followup_collects_one_and_two_digit_slots() {
        let spell_index = spell_index_from_code("GP").unwrap();
        let stats = combat_class_stats(32).unwrap();

        let mut single = world_state(open_world_grid(), 10, 20);
        single.combat_active = true;
        single.party[0].mana = 1;
        single.party[0].level = 1;
        single.spell_charges[spell_index] = 1;
        single.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5]);
        let single_target = COMBAT_PARTY_ACTOR_SLOTS;
        single.combat_actors[single_target] = CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

        assert_eq!(
            single.start_combat_cast_spell_prompt(0, false),
            MoveOutcome::Observed
        );
        assert!(single
            .step_active_cast('G', "P", std::path::Path::new(""))
            .unwrap()
            .is_none());
        assert!(single
            .step_active_cast(' ', "", std::path::Path::new(""))
            .unwrap()
            .is_none());
        assert!(single.active_cast_followup.is_some());
        assert!(single.message.contains("Target?"));
        assert_eq!(single.spell_charges[spell_index], 1);
        assert_eq!(single.party[0].mana, 1);
        assert_eq!(single.turn, 0);

        let single_result = single
            .step_active_cast_followup('7', "", std::path::Path::new(""))
            .unwrap()
            .expect("slot 7 should finish the combat spell");
        assert_eq!(single_result.0, MoveOutcome::Cast);
        assert_eq!(single.spell_charges[spell_index], 0);
        assert_eq!(single.party[0].mana, 0);
        assert_eq!(single.turn, 1);
        assert_eq!(single.message, "Magic Missile!");

        let mut double = world_state(open_world_grid(), 10, 20);
        double.combat_active = true;
        double.party[0].mana = 1;
        double.party[0].level = 1;
        double.spell_charges[spell_index] = 1;
        double.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5]);
        let double_target = 9;
        double.combat_actors[double_target] = CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

        assert_eq!(
            double.start_combat_cast_spell_prompt(0, false),
            MoveOutcome::Observed
        );
        assert!(double
            .step_active_cast('G', "P", std::path::Path::new(""))
            .unwrap()
            .is_none());
        assert!(double
            .step_active_cast(' ', "", std::path::Path::new(""))
            .unwrap()
            .is_none());
        assert!(
            double
                .step_active_cast_followup('1', "", std::path::Path::new(""))
                .unwrap()
                .is_none()
        );
        assert!(double.active_cast_followup.is_some());
        assert!(double.message.contains("1_"));
        assert_eq!(double.spell_charges[spell_index], 1);
        assert_eq!(double.party[0].mana, 1);
        assert_eq!(double.turn, 0);

        let double_result = double
            .step_active_cast_followup('0', "", std::path::Path::new(""))
            .unwrap()
            .expect("slot 10 should finish the combat spell");
        assert_eq!(double_result.0, MoveOutcome::Cast);
        assert_eq!(double.spell_charges[spell_index], 0);
        assert_eq!(double.party[0].mana, 0);
        assert_eq!(double.turn, 1);
        assert_eq!(double.message, "Magic Missile!");
    }

    #[test]
    fn combat_cast_repel_undead_routes_resources_and_dispels_undead_classes() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party[0].mana = REPEL_UNDEAD_COST;
        state.party[0].level = REPEL_UNDEAD_COST;
        state.party_experience = vec![10];
        state.spell_charges[REPEL_UNDEAD_SPELL_INDEX] = 1;

        let ghost = combat_class_stats(23).unwrap();
        let skeleton = combat_class_stats(33).unwrap();
        let orc = combat_class_stats(32).unwrap();
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] =
            CombatActorDescriptor::for_monster_placement(
                ghost,
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                4,
                5,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
            );
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1] =
            CombatActorDescriptor::for_monster_placement(
                skeleton,
                (COMBAT_PARTY_ACTOR_SLOTS + 1) as u8,
                5,
                5,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
            );
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2] =
            CombatActorDescriptor::for_monster_placement(
                orc,
                (COMBAT_PARTY_ACTOR_SLOTS + 2) as u8,
                6,
                5,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
            );

        assert_eq!(
            state
                .cast_spell_from_suffix("1ACX", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[REPEL_UNDEAD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_marked_dead());
        assert!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_marked_dead());
        assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2].is_marked_dead());
        assert_eq!(
            state.party_experience[0],
            10 + u16::from(ghost.reward_unit()) + u16::from(skeleton.reward_unit())
        );
        assert_eq!(state.message, "Repel Undead! 2 undead repelled.");
    }

    #[test]
    fn combat_cast_directed_sleep_and_poison_wind_mutate_party_targets() {
        let mut sleep = world_state(open_world_grid(), 10, 20);
        sleep.combat_active = true;
        sleep.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: SLEEP_COST,
                hp: 12,
                max_hp: 20,
                level: SLEEP_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
        ];
        sleep.spell_charges[SLEEP_SPELL_INDEX] = 1;
        sleep.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        sleep.combat_actors[1] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 6, 5]);

        assert_eq!(
            sleep
                .cast_spell_from_suffix("1IZ2", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(sleep.spell_charges[SLEEP_SPELL_INDEX], 0);
        assert_eq!(sleep.party[0].mana, 0);
        assert_eq!(sleep.party[1].status, b'S');
        assert_eq!(sleep.message, "Sleep!");

        let mut poison = world_state(open_world_grid(), 10, 20);
        poison.combat_active = true;
        poison.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: POISON_WIND_COST,
                hp: 12,
                max_hp: 20,
                level: POISON_WIND_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
        ];
        poison.spell_charges[POISON_WIND_SPELL_INDEX] = 1;
        poison.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        poison.combat_actors[2] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 2, 0, 6, 5]);

        assert_eq!(
            poison
                .cast_spell_from_suffix("1HIN3", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(poison.spell_charges[POISON_WIND_SPELL_INDEX], 0);
        assert_eq!(poison.party[0].mana, 0);
        assert_eq!(poison.party[2].status, b'P');
        assert_eq!(poison.message, "Poison wind!");
    }

    #[test]
    fn combat_cast_directed_damage_winds_route_damage_and_friendly_fire() {
        let mut death = world_state(open_world_grid(), 10, 20);
        death.combat_active = true;
        death.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: DEATH_WIND_COST,
                hp: 12,
                max_hp: 20,
                level: DEATH_WIND_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 12,
                max_hp: 20,
                level: 1,
            },
        ];
        death.party_experience = vec![10, 20];
        death.spell_charges[DEATH_WIND_SPELL_INDEX] = 1;
        death.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        death.combat_actors[1] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 6, 5]);
        let stats = combat_class_stats(32).unwrap();
        death.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] =
            CombatActorDescriptor::for_monster_placement(
                stats,
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                7,
                5,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
            );

        assert_eq!(
            death
                .cast_spell_from_suffix("1CGIV7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(death.spell_charges[DEATH_WIND_SPELL_INDEX], 0);
        assert_eq!(death.party[0].status, b'G');
        assert_eq!(death.party[1].status, b'D');
        assert!(death.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_marked_dead());
        assert_eq!(
            death.party_experience[0],
            10 + u16::from(stats.reward_unit())
        );
        assert_eq!(death.message, "Death wind!");

        let mut flame = world_state(open_world_grid(), 10, 20);
        flame.combat_active = true;
        flame.party[0].mana = FLAME_WIND_COST;
        flame.party[0].level = FLAME_WIND_COST;
        flame.party_experience = vec![10];
        flame.spell_charges[FLAME_WIND_SPELL_INDEX] = 1;
        flame.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        flame.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] =
            CombatActorDescriptor::for_monster_placement(
                stats,
                COMBAT_PARTY_ACTOR_SLOTS as u8,
                6,
                5,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                0,
            );

        assert_eq!(
            flame
                .cast_spell_from_suffix("1FHI7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(flame.spell_charges[FLAME_WIND_SPELL_INDEX], 0);
        assert!(flame.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].hp_or_wound < stats.max_hp);
        assert_eq!(flame.message, "Flame wind!");
    }

    #[test]
    fn active_target_spell_damage_application_preserves_kill_and_miss_routes() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.party_experience = vec![10];
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);

        assert_eq!(
            state.apply_active_target_combat_spell_damage(
                Some(0),
                target_slot,
                CombatSpellDamageKind::MagicMissile,
                0,
                5,
            ),
            Some(CombatActiveTargetSpellDamageApplication {
                kind: CombatSpellDamageKind::MagicMissile,
                raw_damage: -4,
                damage_application: CombatWeaponDamageApplication::Monster {
                    target_slot,
                    damage: CombatMonsterDamageOutcome {
                        class: 32,
                        raw_damage: -4,
                        applied_damage: 0,
                        missed: true,
                        instant_kill: false,
                        killed: false,
                        return_value: 0,
                        death_path: None,
                    },
                    credited_experience: None,
                },
            })
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
        assert_eq!(state.party_experience[0], 10);

        assert_eq!(
            state.apply_active_target_combat_spell_damage(
                Some(0),
                target_slot,
                CombatSpellDamageKind::Kill,
                0,
                255,
            ),
            Some(CombatActiveTargetSpellDamageApplication {
                kind: CombatSpellDamageKind::Kill,
                raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                damage_application: CombatWeaponDamageApplication::Monster {
                    target_slot,
                    damage: CombatMonsterDamageOutcome {
                        class: 32,
                        raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                        applied_damage: stats.max_hp,
                        missed: false,
                        instant_kill: true,
                        killed: true,
                        return_value: stats.reward_unit(),
                        death_path: Some(CombatMonsterDeathPath::DefaultDropCheck),
                    },
                    credited_experience: Some(10 + u16::from(stats.reward_unit())),
                },
            })
        );
        assert!(state.combat_actors[target_slot].is_marked_dead());
        assert_eq!(
            state.apply_active_target_combat_spell_damage(
                Some(0),
                target_slot,
                CombatSpellDamageKind::Tremor,
                0,
                0,
            ),
            None
        );
    }

    #[test]
    fn tremor_spell_damage_application_scans_table_order_and_credits_caster() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.party_experience = vec![10];
        state.active_player = Some(0);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 3, 3]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);
        state.combat_actors[target_slot + 1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            33,
            8,
            0,
            5,
            5,
        ]);
        let mut gate_accepts = [false; COMBAT_ACTOR_SLOTS];
        gate_accepts[0] = true;
        gate_accepts[target_slot] = true;
        gate_accepts[target_slot + 1] = true;

        assert_eq!(
            state.apply_tremor_combat_spell_damage(Some(0), &gate_accepts, &[2, 4]),
            Some(CombatTremorSpellDamageApplication {
                applications: vec![
                    CombatTremorSpellSlotDamageApplication {
                        target_slot: 0,
                        raw_damage: 3,
                        damage_application: CombatWeaponDamageApplication::Party {
                            target_slot: 0,
                            damage: CombatPartyDamageOutcome {
                                raw_damage: 3,
                                applied_damage: 3,
                                missed: false,
                                instant_kill: false,
                                killed: false,
                                status_before: b'G',
                                status_after: b'G',
                            },
                        },
                    },
                    CombatTremorSpellSlotDamageApplication {
                        target_slot,
                        raw_damage: 5,
                        damage_application: CombatWeaponDamageApplication::Monster {
                            target_slot,
                            damage: CombatMonsterDamageOutcome {
                                class: 32,
                                raw_damage: 5,
                                applied_damage: 5,
                                missed: false,
                                instant_kill: false,
                                killed: false,
                                return_value: 5,
                                death_path: None,
                            },
                            credited_experience: Some(15),
                        },
                    },
                ],
            })
        );
        assert_eq!(state.party[0].hp, 9);
        assert_eq!(state.active_player, Some(0));
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, 5);
        assert_eq!(state.party_experience[0], 15);
    }

    #[test]
    fn combat_cast_tremor_routes_resources_table_damage_and_xp() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 6,
            hp: 30,
            max_hp: 30,
            level: 6,
        }];
        state.party_experience = vec![10];
        state.prng_state = 0x1234;
        let spell_index = spell_index_from_code("IPVY").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([1, 1, 0, 0, 0, 0, 3, 3]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

        let mut expected_prng = state.prng_state;
        let party_roll = u5_prng_range_u16(
            &mut expected_prng,
            0,
            u16::from(COMBAT_TREMOR_DAMAGE_ROLL_MAX - 1),
        ) as u8;
        let monster_roll = u5_prng_range_u16(
            &mut expected_prng,
            0,
            u16::from(COMBAT_TREMOR_DAMAGE_ROLL_MAX - 1),
        ) as u8;
        let party_damage = resolve_tremor_spell_raw_damage(party_roll);
        let monster_damage = resolve_tremor_spell_raw_damage(monster_roll);
        let monster_reward = if monster_damage as u8 >= stats.max_hp {
            stats.reward_unit()
        } else {
            monster_damage as u8
        };

        assert_eq!(
            state
                .cast_spell_from_suffix("1IPVY", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.message, "Tremor!");
        assert_eq!(state.party[0].hp, 30 - party_damage as u16);
        assert_eq!(
            state.combat_actors[target_slot].hp_or_wound,
            stats.max_hp.saturating_sub(monster_damage as u8)
        );
        assert_eq!(
            state.party_experience[0],
            10 + u16::from(monster_reward)
        );
    }

    #[test]
    fn combat_cast_polymorph_routes_resources_and_replaces_hostile_creature() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 6,
            hp: 30,
            max_hp: 30,
            level: 6,
        }];
        let spell_index = spell_index_from_code("BRX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let dragon_stats = combat_class_stats(39).unwrap();
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            dragon_stats,
            target_slot as u8,
            5,
            6,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            2,
        );
        state.active_objects[target_slot] = ActiveObject {
            type_byte: 0xdc,
            tile: 0xdc,
            x: 5,
            y: 6,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x33,
            aux3: 0x44,
        };
        state.visibility_dirty = false;

        assert_eq!(
            state
                .cast_spell_from_suffix("1BRX7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        let rat_stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap();
        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Polymorph!");
        assert_eq!(
            state.combat_actors[target_slot].owner_target_class,
            COMBAT_CLASS_GIANT_RAT
        );
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, rat_stats.max_hp);
        assert_eq!(
            state.combat_actors[target_slot].active_object_slot,
            target_slot as u8
        );
        assert_eq!((state.combat_actors[target_slot].x, state.combat_actors[target_slot].y), (5, 6));
        assert_eq!(state.active_objects[target_slot].type_byte, COMBAT_CLASS_GIANT_RAT_SPRITE_BASE);
        assert_eq!(state.active_objects[target_slot].tile, COMBAT_CLASS_GIANT_RAT_SPRITE_BASE);
        assert_eq!((state.active_objects[target_slot].x, state.active_objects[target_slot].y), (5, 6));
        assert_eq!(state.active_objects[target_slot].aux1, 0x33);
        assert_eq!(state.active_objects[target_slot].aux3, 0x44);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_cast_polymorph_rejects_same_faction_target_before_resources() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 6,
                hp: 30,
                max_hp: 30,
                level: 6,
            },
            PartyMember {
                slot: 1,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 30,
                max_hp: 30,
                level: 1,
            },
        ];
        let spell_index = spell_index_from_code("BRX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            0,
            4,
            3,
        ]);

        assert_eq!(
            state
                .cast_spell_from_suffix("1BRX2", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[spell_index], 1);
        assert_eq!(state.party[0].mana, 6);
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.message,
            "Target? Use C1BRX7 to target a hostile creature."
        );
    }

    #[test]
    fn combat_cast_field_spell_places_arena_marker_after_coordinate_lookup() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 3,
            hp: 30,
            max_hp: 30,
            level: 3,
        }];
        let spell_index = spell_index_from_code("GIN").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let stats = combat_class_stats(32).unwrap();
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            target_slot as u8,
            4,
            3,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        state.active_objects[target_slot] = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 4,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        assert_eq!(
            state
                .cast_spell_from_suffix("1GIN6", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Poison field placed.");
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: COMBAT_FIELD_KIND_POISON,
                tile: COMBAT_FIELD_KIND_POISON,
                x: 4,
                y: 3,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_cast_field_spell_places_marker_without_immediate_contact() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 3,
            hp: 30,
            max_hp: 30,
            level: 3,
        }];
        state.party_experience = vec![123];
        let spell_index = spell_index_from_code("FGI").unwrap();
        state.spell_charges[spell_index] = 1;
        state.prng_state = seed_for_first_mod_roll(COMBAT_ARENA_FIELD_RANDOM_GATE_DENOMINATOR, 0);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let stats = combat_class_stats(32).unwrap();
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            target_slot as u8,
            4,
            3,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        state.active_objects[target_slot] = ActiveObject {
            type_byte: 0x70,
            tile: 0x70,
            x: 4,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        assert_eq!(
            state
                .cast_spell_from_suffix("1FGI6", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Fire field placed.");
        assert_eq!(state.party_experience, vec![123]);
        assert!(!state.combat_actors[target_slot].is_marked_dead());
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
        assert_eq!(state.active_objects[target_slot].tile, 0x70);
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: COMBAT_FIELD_KIND_FIRE,
                tile: COMBAT_FIELD_KIND_FIRE,
                x: 4,
                y: 3,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
    }

    #[test]
    fn combat_cast_fire_field_random_gate_failure_consumes_cast_without_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 3,
            hp: 30,
            max_hp: 30,
            level: 3,
        }];
        let spell_index = spell_index_from_code("FGI").unwrap();
        state.spell_charges[spell_index] = 1;
        state.prng_state = seed_for_first_mod_roll(COMBAT_ARENA_FIELD_RANDOM_GATE_DENOMINATOR, 7);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let stats = combat_class_stats(32).unwrap();
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            target_slot as u8,
            4,
            3,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        state.active_objects[target_slot] = ActiveObject {
            type_byte: 0x70,
            tile: 0x70,
            x: 4,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        assert_eq!(
            state
                .cast_spell_from_suffix("1FGI6", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Failed!");
        assert!(!state.combat_actors[target_slot].is_marked_dead());
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
        assert_eq!(state.active_objects[target_slot].tile, 0x70);
        assert!(state.active_objects[1..target_slot]
            .iter()
            .all(|object| object.is_empty()));
    }

    #[test]
    fn combat_cast_field_spell_requires_target_lookup_and_keeps_marker_table_unchanged() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 3,
            hp: 30,
            max_hp: 30,
            level: 3,
        }];
        let spell_index = spell_index_from_code("GIN").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        assert_eq!(
            state
                .cast_spell_from_suffix("1GIN6", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Failed!");
        assert!(state.active_objects[1..].iter().all(|object| object.is_empty()));
    }

    #[test]
    fn combat_cast_dispel_field_removes_matching_arena_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 4,
            hp: 30,
            max_hp: 30,
            level: 4,
        }];
        state.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[1] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_FIRE,
            tile: COMBAT_FIELD_KIND_FIRE,
            x: 4,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x55,
            aux3: 0x66,
        };
        state.visibility_dirty = false;

        assert_eq!(
            state
                .cast_spell_from_suffix("1AG6", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.spell_charges[DISPEL_FIELD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Dispelled Fire field.");
        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.active_objects[1].tile, COMBAT_FIELD_KIND_FIRE);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_cast_dispel_field_without_marker_consumes_cast_and_fails() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 4,
            hp: 30,
            max_hp: 30,
            level: 4,
        }];
        state.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        assert_eq!(
            state
                .cast_spell_from_suffix("1AG6", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[DISPEL_FIELD_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Failed!");
        assert!(state.active_objects[1..].iter().all(|object| object.is_empty()));
    }

    #[test]
    fn tremor_spell_damage_application_requires_roll_for_each_accepted_target() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party[0].hp = 12;
        state.party_experience = vec![10];
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 3, 3]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);
        let mut gate_accepts = [false; COMBAT_ACTOR_SLOTS];
        gate_accepts[0] = true;
        gate_accepts[target_slot] = true;

        assert_eq!(
            state.apply_tremor_combat_spell_damage(Some(0), &gate_accepts, &[2]),
            None
        );
        assert_eq!(state.party[0].hp, 12);
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
        assert_eq!(state.party_experience[0], 10);
    }

    #[test]
    fn directed_spell_damage_application_applies_death_wind_in_table_order() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let living = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };
        state.party = vec![
            living,
            PartyMember {
                slot: 1,
                ..living
            },
        ];
        state.party_experience = vec![10, 20];
        state.active_player = Some(1);
        state.combat_actors[1] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 1, 0, 4, 4]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 5, 5, 0, 0);

        assert_eq!(
            state.apply_directed_combat_spell_damage(
                Some(0),
                CombatDirectedSpellEffect::DeathWind,
                &[(4, 4), (5, 5)],
                &[],
            ),
            Some(CombatDirectedSpellDamageApplication {
                effect: CombatDirectedSpellEffect::DeathWind,
                applications: vec![
                    CombatDirectedSpellSlotDamageApplication {
                        target_slot: 1,
                        raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                        damage_application: CombatWeaponDamageApplication::Party {
                            target_slot: 1,
                            damage: CombatPartyDamageOutcome {
                                raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                                applied_damage: 12,
                                missed: false,
                                instant_kill: true,
                                killed: true,
                                status_before: b'G',
                                status_after: b'D',
                            },
                        },
                    },
                    CombatDirectedSpellSlotDamageApplication {
                        target_slot,
                        raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                        damage_application: CombatWeaponDamageApplication::Monster {
                            target_slot,
                            damage: CombatMonsterDamageOutcome {
                                class: 32,
                                raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                                applied_damage: stats.max_hp,
                                missed: false,
                                instant_kill: true,
                                killed: true,
                                return_value: stats.reward_unit(),
                                death_path: Some(CombatMonsterDeathPath::DefaultDropCheck),
                            },
                            credited_experience: Some(10 + u16::from(stats.reward_unit())),
                        },
                    },
                ],
            })
        );
        assert_eq!(state.active_player, None);
        assert_eq!(
            state.party_experience[0],
            10 + u16::from(stats.reward_unit())
        );
    }

    #[test]
    fn directed_spell_damage_application_handles_flame_wind_rolls_and_non_damage_effects() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party_experience = vec![10];
        let stats = combat_class_stats(32).unwrap();
        let first_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let second_slot = first_slot + 1;
        state.combat_actors[first_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);
        state.combat_actors[second_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 8, 5, 5, 0, 0);

        assert_eq!(
            state.apply_directed_combat_spell_damage(
                Some(0),
                CombatDirectedSpellEffect::FlameWind,
                &[(4, 5), (5, 5)],
                &[4],
            ),
            None
        );
        assert_eq!(state.combat_actors[first_slot].hp_or_wound, stats.max_hp);
        assert_eq!(state.combat_actors[second_slot].hp_or_wound, stats.max_hp);

        assert_eq!(
            state.apply_directed_combat_spell_damage(
                Some(0),
                CombatDirectedSpellEffect::PoisonWind,
                &[(4, 5)],
                &[4],
            ),
            None
        );

        assert_eq!(
            state.apply_directed_combat_spell_damage(
                Some(0),
                CombatDirectedSpellEffect::FlameWind,
                &[(4, 5), (4, 5), (5, 5)],
                &[4, 9],
            ),
            Some(CombatDirectedSpellDamageApplication {
                effect: CombatDirectedSpellEffect::FlameWind,
                applications: vec![
                    CombatDirectedSpellSlotDamageApplication {
                        target_slot: first_slot,
                        raw_damage: 5,
                        damage_application: CombatWeaponDamageApplication::Monster {
                            target_slot: first_slot,
                            damage: CombatMonsterDamageOutcome {
                                class: 32,
                                raw_damage: 5,
                                applied_damage: 5,
                                missed: false,
                                instant_kill: false,
                                killed: false,
                                return_value: 5,
                                death_path: None,
                            },
                            credited_experience: Some(15),
                        },
                    },
                    CombatDirectedSpellSlotDamageApplication {
                        target_slot: second_slot,
                        raw_damage: 10,
                        damage_application: CombatWeaponDamageApplication::Monster {
                            target_slot: second_slot,
                            damage: CombatMonsterDamageOutcome {
                                class: 32,
                                raw_damage: 10,
                                applied_damage: 10,
                                missed: false,
                                instant_kill: false,
                                killed: true,
                                return_value: stats.reward_unit(),
                                death_path: Some(CombatMonsterDeathPath::DefaultDropCheck),
                            },
                            credited_experience: Some(15 + u16::from(stats.reward_unit())),
                        },
                    },
                ],
            })
        );
        assert_eq!(state.combat_actors[first_slot].hp_or_wound, 5);
        assert!(state.combat_actors[second_slot].is_marked_dead());
    }

    #[test]
    fn directed_spell_status_application_applies_sleep_to_party_and_reports_non_party() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 5, 5, 0, 0);
        state.combat_actors[target_slot + 1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            33,
            8,
            0,
            6,
            6,
        ]);

        assert_eq!(
            state.apply_directed_combat_spell_status(
                CombatDirectedSpellEffect::Sleep,
                &[(4, 4), (5, 5), (6, 6)],
                &[],
                &[],
            ),
            Some(CombatDirectedSpellStatusApplication {
                effect: CombatDirectedSpellEffect::Sleep,
                applications: vec![
                    CombatDirectedSpellSlotStatusApplication::PartySleep {
                        target_slot: 0,
                        outcome: CombatPartySleepOutcome::SleptPartyMember {
                            status_before: b'G',
                            status_after: b'S',
                        },
                    },
                    CombatDirectedSpellSlotStatusApplication::NonPartySleepDisabled { target_slot },
                ],
            })
        );
        assert_eq!(state.party[0].status, b'S');
        assert!(state.combat_actors[target_slot].is_status_disabled());
        assert_eq!(
            state.combat_sleep_durations[target_slot],
            COMBAT_SLEEP_DISABLED_DURATION_DEFAULT
        );
        assert_eq!(
            state.apply_directed_combat_spell_status(
                CombatDirectedSpellEffect::DeathWind,
                &[(4, 4)],
                &[],
                &[],
            ),
            None
        );
    }

    #[test]
    fn directed_spell_status_application_applies_poison_wind_gate_status_and_fallback_damage() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let living = PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        };
        state.party = vec![
            living,
            PartyMember {
                slot: 1,
                status: b'P',
                ..living
            },
        ];
        state.party_experience = vec![10, 20];
        state.active_player = Some(1);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
        state.combat_actors[1] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 1, 0, 5, 4]);
        let stats = combat_class_stats(32).unwrap();
        let first_monster = COMBAT_PARTY_ACTOR_SLOTS;
        let second_monster = first_monster + 1;
        state.combat_actors[first_monster] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 5, 5, 0, 0);
        state.combat_actors[second_monster] =
            CombatActorDescriptor::for_monster_placement(stats, 8, 6, 5, 0, 0);

        assert_eq!(
            state.apply_directed_combat_spell_status(
                CombatDirectedSpellEffect::PoisonWind,
                &[(4, 4), (5, 4), (5, 5), (6, 5)],
                &[true, true, true, false],
                &[0, 4, 9, 19],
            ),
            Some(CombatDirectedSpellStatusApplication {
                effect: CombatDirectedSpellEffect::PoisonWind,
                applications: vec![
                    CombatDirectedSpellSlotStatusApplication::PartyPoison {
                        target_slot: 0,
                        outcome: CombatPartyPoisonOutcome::PoisonedPartyMember {
                            status_before: b'G',
                            status_after: b'P',
                        },
                        fallback_damage_application: None,
                    },
                    CombatDirectedSpellSlotStatusApplication::PartyPoison {
                        target_slot: 1,
                        outcome: CombatPartyPoisonOutcome::FallbackDamage { raw_damage: 5 },
                        fallback_damage_application: Some(
                            CombatWeaponDamageApplication::Party {
                                target_slot: 1,
                                damage: CombatPartyDamageOutcome {
                                    raw_damage: 5,
                                    applied_damage: 5,
                                    missed: false,
                                    instant_kill: false,
                                    killed: false,
                                    status_before: b'P',
                                    status_after: b'P',
                                },
                            },
                        ),
                    },
                    CombatDirectedSpellSlotStatusApplication::NonPartyPoisonFallbackDamage {
                        target_slot: first_monster,
                        raw_damage: 10,
                        damage_application: CombatWeaponDamageApplication::Monster {
                            target_slot: first_monster,
                            damage: CombatMonsterDamageOutcome {
                                class: 32,
                                raw_damage: 10,
                                applied_damage: 10,
                                missed: false,
                                instant_kill: false,
                                killed: true,
                                return_value: stats.reward_unit(),
                                death_path: Some(CombatMonsterDeathPath::DefaultDropCheck),
                            },
                            credited_experience: None,
                        },
                    },
                    CombatDirectedSpellSlotStatusApplication::PoisonGateRejected {
                        target_slot: second_monster,
                    },
                ],
            })
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[1].hp, 7);
        assert_eq!(state.active_player, Some(1));
        assert!(state.combat_actors[first_monster].is_marked_dead());
        assert_eq!(state.combat_actors[second_monster].hp_or_wound, stats.max_hp);
        assert_eq!(state.party_experience, vec![10, 20]);
    }

    #[test]
    fn directed_spell_status_application_requires_poison_inputs_before_mutation() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party[0].status = b'G';
        state.party[0].hp = 12;
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
        let stats = combat_class_stats(32).unwrap();
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 5, 5, 0, 0);

        assert_eq!(
            state.apply_directed_combat_spell_status(
                CombatDirectedSpellEffect::PoisonWind,
                &[(4, 4), (5, 5)],
                &[true],
                &[0, 4],
            ),
            None
        );
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);

        assert_eq!(
            state.apply_directed_combat_spell_status(
                CombatDirectedSpellEffect::PoisonWind,
                &[(4, 4), (5, 5)],
                &[true, true],
                &[0],
            ),
            None
        );
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
    }

    #[test]
    fn arena_field_contact_application_applies_party_status_and_no_xp_fallback_damage() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 12,
            max_hp: 20,
            level: 1,
        }];
        state.party_experience = vec![10];
        state.active_player = Some(0);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
        state.active_objects[0].tile = 0x10;

        assert_eq!(
            state.apply_combat_arena_field_contact(CombatArenaFieldKind::Poison, 7, 0, 4, 0, 0),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Poison,
                target_slot: 0,
                contact_outcome: CombatArenaFieldContactOutcome::PoisonedPartyMember {
                    status_before: b'G',
                    status_after: b'P',
                },
                damage_application: None,
            })
        );
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 12);

        assert_eq!(
            state.apply_combat_arena_field_contact(CombatArenaFieldKind::Poison, 7, 0, 4, 0, 0),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Poison,
                target_slot: 0,
                contact_outcome: CombatArenaFieldContactOutcome::PoisonFallbackDamage {
                    raw_damage: 5,
                },
                damage_application: Some(CombatWeaponDamageApplication::Party {
                    target_slot: 0,
                    damage: CombatPartyDamageOutcome {
                        raw_damage: 5,
                        applied_damage: 5,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        status_before: b'P',
                        status_after: b'P',
                    },
                }),
            })
        );
        assert_eq!(state.party[0].hp, 7);
        assert_eq!(state.party_experience, vec![10]);

        assert_eq!(
            state.apply_combat_arena_field_contact(CombatArenaFieldKind::Fire, 7, 0, 20, 20, 5),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Fire,
                target_slot: 0,
                contact_outcome: CombatArenaFieldContactOutcome::FireDamage {
                    raw_damage: 21,
                },
                damage_application: Some(CombatWeaponDamageApplication::Party {
                    target_slot: 0,
                    damage: CombatPartyDamageOutcome {
                        raw_damage: 16,
                        applied_damage: 7,
                        missed: false,
                        instant_kill: false,
                        killed: true,
                        status_before: b'P',
                        status_after: b'D',
                    },
                }),
            })
        );
        assert_eq!(state.active_player, None);
    }

    #[test]
    fn arena_field_contact_application_handles_non_party_damage_skip_and_sleep_report() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party_experience = vec![10];
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        let stats = combat_class_stats(32).unwrap();
        let first_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let second_slot = first_slot + 1;
        state.combat_actors[first_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);
        state.combat_actors[second_slot] =
            CombatActorDescriptor::for_monster_placement(stats, 8, 5, 5, 0, 0);
        state.active_objects[7].tile = 0x70;
        state.active_objects[8].tile = 0x90;

        assert_eq!(
            state.apply_combat_arena_field_contact(
                CombatArenaFieldKind::Poison,
                0,
                second_slot,
                19,
                0,
                0,
            ),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Poison,
                target_slot: second_slot,
                contact_outcome: CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass,
                damage_application: None,
            })
        );
        assert_eq!(state.combat_actors[second_slot].hp_or_wound, stats.max_hp);

        assert_eq!(
            state.apply_combat_arena_field_contact(
                CombatArenaFieldKind::Sleep,
                0,
                second_slot,
                0,
                0,
                0,
            ),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Sleep,
                target_slot: second_slot,
                contact_outcome: CombatArenaFieldContactOutcome::SleepDisabledNonParty,
                damage_application: None,
            })
        );
        assert_eq!(state.combat_actors[first_slot].hp_or_wound, stats.max_hp);
        assert!(state.combat_actors[second_slot].is_status_disabled());
        assert_eq!(
            state.combat_sleep_durations[second_slot],
            COMBAT_SLEEP_DISABLED_DURATION_DEFAULT
        );

        assert_eq!(
            state.apply_combat_arena_field_contact(
                CombatArenaFieldKind::Fire,
                0,
                first_slot,
                0,
                20,
                5,
            ),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Fire,
                target_slot: first_slot,
                contact_outcome: CombatArenaFieldContactOutcome::FireDamage {
                    raw_damage: 21,
                },
                damage_application: Some(CombatWeaponDamageApplication::Monster {
                    target_slot: first_slot,
                    damage: CombatMonsterDamageOutcome {
                        class: 32,
                        raw_damage: 16,
                        applied_damage: 10,
                        missed: false,
                        instant_kill: false,
                        killed: true,
                        return_value: stats.reward_unit(),
                        death_path: Some(CombatMonsterDeathPath::DefaultDropCheck),
                    },
                    credited_experience: None,
                }),
            })
        );
        assert!(state.combat_actors[first_slot].is_marked_dead());
        assert_eq!(state.party_experience, vec![10]);
    }

    #[test]
    fn arena_field_contact_application_skips_current_actor_and_applies_energy_zero_damage() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
        state.active_objects[0].tile = 0x10;

        assert_eq!(
            state.apply_combat_arena_field_contact(CombatArenaFieldKind::Fire, 0, 0, 0, 20, 0),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Fire,
                target_slot: 0,
                contact_outcome: CombatArenaFieldContactOutcome::SkippedCurrentActor,
                damage_application: None,
            })
        );
        assert_eq!(state.party[0].hp, 60);

        assert_eq!(
            state.apply_combat_arena_field_contact(CombatArenaFieldKind::Energy, 7, 0, 0, 0, 0),
            Some(CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Energy,
                target_slot: 0,
                contact_outcome: CombatArenaFieldContactOutcome::EnergyDamage { raw_damage: 0 },
                damage_application: Some(CombatWeaponDamageApplication::Party {
                    target_slot: 0,
                    damage: CombatPartyDamageOutcome {
                        raw_damage: 0,
                        applied_damage: 0,
                        missed: false,
                        instant_kill: false,
                        killed: false,
                        status_before: b'G',
                        status_after: b'G',
                    },
                }),
            })
        );
        assert_eq!(state.party[0].hp, 60);
    }

    #[test]
    fn combat_step_post_field_contact_applies_without_consuming_marker() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        }];
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 3,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[1] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_FIRE,
            tile: COMBAT_FIELD_KIND_FIRE,
            x: 4,
            y: 3,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let mut expected_prng = state.prng_state;
        let _poison_roll = u5_prng_range_u16(&mut expected_prng, 0, 19);
        let fire_roll = u5_prng_range_u16(&mut expected_prng, 0, 20) as u8;
        let defense_roll =
            u5_prng_range_u16(&mut expected_prng, 0, CHARACTER_DEFENSE_FACTORY_SEED.into())
                as u8;
        let expected_damage = resolve_spell_damage_after_defense(
            combat_field_fire_raw_damage(fire_roll) as i16,
            defense_roll,
        )
        .max(0) as u16;

        let outcome = state.apply_combat_step_or_attack_primitive(0, 1, COMBAT_DIRECTION_EAST, true);

        assert!(outcome.committed_movement());
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (4, 3));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (4, 3));
        assert_eq!(state.party[0].hp, 20 - expected_damage);
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: COMBAT_FIELD_KIND_FIRE,
                tile: COMBAT_FIELD_KIND_FIRE,
                x: 4,
                y: 3,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            }
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn combat_round_counter_state_wrapper_updates_byte_and_marks_wrap_redraw() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 2;
        state.visibility_dirty = false;
        let clock_before = state.clock;

        let ordinary = state.advance_combat_round_counter();
        assert_eq!(ordinary.counter, COMBAT_ROUND_COUNTER_WRAP - 1);
        assert!(!ordinary.wrapped);
        assert_eq!(ordinary.advance_time_minutes, 0);
        assert_eq!(state.combat_round_counter, COMBAT_ROUND_COUNTER_WRAP - 1);
        assert!(!state.visibility_dirty);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, clock_before);

        state.combat_active = true;
        state.clock = GameClock::new(1, 59).unwrap();
        let active_objects_before = state.active_objects.clone();
        let wrapped = state.advance_combat_round_counter();
        assert_eq!(wrapped.counter, 0);
        assert!(wrapped.wrapped);
        assert_eq!(
            wrapped.advance_time_minutes,
            COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES
        );
        assert_eq!(state.combat_round_counter, 0);
        assert!(state.visibility_dirty);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(2, 0).unwrap());
        assert_eq!(state.active_objects, active_objects_before);
    }

    #[test]
    fn combat_post_round_maintenance_sweeps_effects_and_visual_markers_without_field_lifetime() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.active_player = Some(0);
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            6,
        ]);
        state.combat_magic_effects =
            [[COMBAT_POST_ROUND_NO_EFFECT_SENTINEL; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_magic_effects[0][0] = COMBAT_FIELD_KIND_POISON;
        state.combat_terrain[1][0] = COMBAT_POST_ROUND_MAGIC_TIMER_TILE;
        state.combat_magic_effect_timer = COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX - 1;
        state.combat_terrain[2][0] = 0x04;
        state.combat_secondary_marker = Some((3, 4));
        state.active_objects.push(ActiveObject {
            type_byte: COMBAT_FIELD_KIND_FIRE,
            tile: COMBAT_FIELD_KIND_FIRE,
            x: 7,
            y: 7,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let active_objects_before = state.active_objects.clone();
        let party_before = state.party.clone();

        let report = state.apply_combat_post_round_maintenance();

        assert_eq!(
            report.cell_dispatches[0],
            CombatPostRoundCellDispatch {
                x: 0,
                y: 0,
                kind: CombatPostRoundCellDispatchKind::MagicEffectByte {
                    effect: COMBAT_FIELD_KIND_POISON,
                },
            }
        );
        assert_eq!(
            report.cell_dispatches[1],
            CombatPostRoundCellDispatch {
                x: 0,
                y: 1,
                kind: CombatPostRoundCellDispatchKind::MagicTimerTick {
                    before: COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX - 1,
                    after: COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX,
                },
            }
        );
        assert_eq!(
            report.cell_dispatches[2],
            CombatPostRoundCellDispatch {
                x: 0,
                y: 2,
                kind: CombatPostRoundCellDispatchKind::TerrainEffectByte { terrain: 0x04 },
            }
        );
        assert_eq!(report.cell_dispatches.len(), 3);
        assert!(report.cursor_blink_visible);
        assert_eq!(report.cursor_draw_cell, Some((5, 6)));
        assert_eq!(report.secondary_marker_cell, Some((3, 4)));
        assert_eq!(
            state.combat_magic_effect_timer,
            COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX
        );
        assert_eq!(state.active_objects, active_objects_before);
        assert_eq!(state.party, party_before);
    }

    #[test]
    fn combat_magic_ring_pass_applies_invisibility_and_vanish_clears_it() {
        let mut state = world_state(open_world_grid(), 10, 20);
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
        state.active_objects[0].tile = 0x5c;
        state.visibility_dirty = false;

        assert_eq!(
            state.apply_combat_magic_ring_pass_to_slot(0, 7, 1),
            Some(CombatMagicRingPassOutcome {
                invisibility_applied: true,
                regeneration_applied: 0,
                vanished_ring: None,
            })
        );
        assert!(state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.active_objects[0].tile, COMBAT_HIDDEN_ACTIVE_OBJECT_TILE);
        assert!(state.visibility_dirty);

        state.visibility_dirty = false;
        assert_eq!(
            state.apply_combat_magic_ring_pass_to_slot(0, 7, 0),
            Some(CombatMagicRingPassOutcome {
                invisibility_applied: false,
                regeneration_applied: 0,
                vanished_ring: Some(EQUIPMENT_ID_RING_INVISIBILITY as u8),
            })
        );
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
        assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.active_objects[0].tile, 0x5c);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Ring of Invisibility vanished.");
    }

    #[test]
    fn combat_magic_ring_pass_regenerates_living_wearers_and_can_vanish() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party[0].hp = 8;
        state.party[0].max_hp = 10;
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
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

        assert_eq!(
            state.apply_combat_magic_ring_pass_to_slot(0, 0, 1),
            Some(CombatMagicRingPassOutcome {
                invisibility_applied: false,
                regeneration_applied: 1,
                vanished_ring: None,
            })
        );
        assert_eq!(state.party[0].hp, 9);

        assert_eq!(
            state.apply_combat_magic_ring_pass_to_slot(0, 7, 0),
            Some(CombatMagicRingPassOutcome {
                invisibility_applied: false,
                regeneration_applied: 0,
                vanished_ring: Some(EQUIPMENT_ID_RING_REGENERATION as u8),
            })
        );
        assert_eq!(state.party[0].hp, 9);
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
        assert_eq!(state.message, "Ring of Regeneration vanished.");
    }

    #[test]
    fn combat_xit_cleanup_state_wrapper_requires_no_active_living_foes() {
        let mut state = world_state(open_world_grid(), 10, 20);
        assert!(state.combat_xit_cleanup_allowed());

        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            20,
            0,
            0,
            5,
            5,
        ]);
        assert!(!state.combat_xit_cleanup_allowed());

        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].mark_dead();
        assert!(state.combat_xit_cleanup_allowed());
    }

    #[test]
    fn default_monster_death_marker_keeps_drop_cap_and_special_bit_separate() {
        assert_eq!(COMBAT_DEFAULT_DEATH_DROP_ROLL_MAX, 99);
        assert_eq!(COMBAT_PARTY_CORPSE_TILE, 0x1e);
        assert_eq!(COMBAT_DEFAULT_DEATH_DROP_TILE, 0x01);
        assert_eq!(COMBAT_DEFAULT_DEATH_NO_DROP_TILE, 0x01);
        assert_eq!(COMBAT_VANISH_DEATH_MARKER_TILE, 0x16);
        assert_eq!(COMBAT_GAZER_DEATH_MARKER_TILE, 0x1f);
        assert_eq!(COMBAT_GARGOYLE_DEATH_TERRAIN_TILE, 0x4c);
        assert!(combat_default_death_drop_gate_accepts(11, 10));
        assert!(!combat_default_death_drop_gate_accepts(11, 11));
        assert_eq!(
            resolve_default_monster_death_marker(0x1e, false, true),
            CombatDefaultDeathMarker::NoDrop
        );
        assert_eq!(
            resolve_default_monster_death_marker(0x1e, true, false),
            CombatDefaultDeathMarker::Drop { loot_byte: 0x1e }
        );
        assert_eq!(
            resolve_default_monster_death_marker(0x1e, true, true),
            CombatDefaultDeathMarker::Drop { loot_byte: 0x9e }
        );
    }

    #[test]
    fn default_monster_death_marker_can_resolve_class_drop_cap() {
        assert_eq!(
            resolve_default_monster_death_marker_for_class(39, true, false).unwrap(),
            CombatDefaultDeathMarker::Drop { loot_byte: 30 }
        );
        assert_eq!(
            resolve_default_monster_death_marker_for_class(39, true, true).unwrap(),
            CombatDefaultDeathMarker::Drop { loot_byte: 0x80 | 30 }
        );
        assert_eq!(resolve_default_monster_death_marker_for_class(11, true, false), None);
    }

    #[test]
    fn combat_split_placement_requires_split_trait_damage_and_survival() {
        let descriptors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        assert_eq!(
            resolve_combat_split_placement(24, 1, false, &descriptors, &[7]),
            Some(CombatSplitPlacement { slot: 7, class: 24 })
        );
        assert_eq!(
            resolve_combat_split_placement(32, 1, false, &descriptors, &[7]),
            None
        );
        assert_eq!(
            resolve_combat_split_placement(24, 0, false, &descriptors, &[7]),
            None
        );
        assert_eq!(
            resolve_combat_split_placement(24, 1, true, &descriptors, &[7]),
            None
        );
    }

    #[test]
    fn combat_split_placement_checks_up_to_eight_candidate_slots() {
        let mut descriptors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        descriptors[3] = CombatActorDescriptor::from_row([10, 1, 0, 24, 0, 0, 4, 4]);
        descriptors[4] = CombatActorDescriptor::from_row([10, 1, 0, 24, 0, 0, 5, 4]);
        descriptors[5] = CombatActorDescriptor::from_row([10, 1, 0, 24, 0, 0, 6, 4]);

        assert_eq!(
            resolve_combat_split_placement(
                24,
                1,
                false,
                &descriptors,
                &[99, 3, 4, 5, 6, 7, 8, 9],
            ),
            Some(CombatSplitPlacement { slot: 6, class: 24 })
        );
        assert_eq!(
            resolve_combat_split_placement(
                24,
                1,
                false,
                &descriptors,
                &[99, 3, 4, 5, 99, 99, 99, 99, 6],
            ),
            None
        );
    }

    #[test]
    fn combat_actor_monster_damage_clamps_miss_and_subtracts_without_underflow() {
        let stats = combat_class_stats(32).unwrap();
        let mut descriptor =
            CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);

        let miss = descriptor.apply_monster_damage(-1, false).unwrap();
        assert!(miss.missed);
        assert!(!miss.killed);
        assert_eq!(miss.applied_damage, 0);
        assert_eq!(miss.return_value, 0);
        assert_eq!(descriptor.hp_or_wound, 10);

        let hit = descriptor.apply_monster_damage(4, false).unwrap();
        assert!(!hit.missed);
        assert!(!hit.killed);
        assert_eq!(hit.applied_damage, 4);
        assert_eq!(hit.return_value, 4);
        assert_eq!(descriptor.hp_or_wound, 6);

        let kill = descriptor.apply_monster_damage(99, false).unwrap();
        assert!(kill.instant_kill);
        assert!(kill.killed);
        assert_eq!(kill.applied_damage, 6);
        assert_eq!(kill.return_value, stats.reward_unit());
        assert_eq!(
            kill.death_path,
            Some(CombatMonsterDeathPath::DefaultDropCheck)
        );
        assert_eq!(descriptor.hp_or_wound, 0);
        assert!(descriptor.is_marked_dead());
    }

    #[test]
    fn combat_actor_monster_damage_applies_physical_traits_and_death_paths() {
        let skeleton = combat_class_stats(33).unwrap();
        let mut physical_half =
            CombatActorDescriptor::for_monster_placement(skeleton, 7, 4, 5, 0, 0);
        let half = physical_half.apply_monster_damage(9, false).unwrap();
        assert_eq!(half.applied_damage, 4);
        assert_eq!(physical_half.hp_or_wound, 16);

        let mut magical_hit =
            CombatActorDescriptor::for_monster_placement(skeleton, 7, 4, 5, 0, 0);
        let magic = magical_hit.apply_monster_damage(9, true).unwrap();
        assert_eq!(magic.applied_damage, 9);
        assert_eq!(magical_hit.hp_or_wound, 11);

        let wanderer = combat_class_stats(13).unwrap();
        let mut immune = CombatActorDescriptor::for_monster_placement(wanderer, 7, 4, 5, 0, 0);
        let immune_hit = immune.apply_monster_damage(40, false).unwrap();
        assert_eq!(immune_hit.applied_damage, 0);
        assert!(!immune_hit.killed);
        assert_eq!(immune.hp_or_wound, 99);

        let vanish = immune
            .apply_monster_damage(COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();
        assert!(vanish.killed);
        assert_eq!(vanish.death_path, Some(CombatMonsterDeathPath::Vanish));

        let gazer = combat_class_stats(28).unwrap();
        let mut special = CombatActorDescriptor::for_monster_placement(gazer, 7, 4, 5, 0, 0);
        let special_kill = special.apply_monster_damage(20, false).unwrap();
        assert!(special_kill.killed);
        assert_eq!(
            special_kill.death_path,
            Some(CombatMonsterDeathPath::SpecialTileTransition)
        );
    }

    #[test]
    fn combat_spawn_count_respects_exact_sentinels_fortunes_reroll_and_cap() {
        assert_eq!(resolve_combat_spawn_count(1, 7, Some(2)), 1);
        assert_eq!(resolve_combat_spawn_count(8, 7, Some(2)), 8);
        assert_eq!(resolve_combat_spawn_count(16, 7, Some(2)), 16);

        assert_eq!(resolve_combat_spawn_count(10, 7, None), 8);
        assert_eq!(resolve_combat_spawn_count(10, 7, Some(2)), 3);
        assert_eq!(resolve_combat_spawn_count(30, 29, None), 26);
        assert_eq!(resolve_combat_spawn_count(0, 29, None), 0);
    }

    #[test]
    fn terrain_combat_setup_count_consumes_fortunes_flag_and_town_override() {
        assert_eq!(resolve_terrain_combat_setup_count(10, 0, 7, 2, false), 8);
        assert_eq!(resolve_terrain_combat_setup_count(10, 1, 7, 2, false), 3);
        assert_eq!(
            resolve_terrain_combat_setup_count(10, 0xff, 7, 2, false),
            3
        );
        assert_eq!(resolve_terrain_combat_setup_count(10, 0xff, 7, 2, true), 1);
        assert_eq!(resolve_terrain_combat_setup_count(16, 0xff, 7, 2, false), 16);

        let mut state = world_state(open_world_grid(), 10, 20);
        state.fortunes_of_war = 0;
        assert_eq!(state.resolve_terrain_combat_setup_count(10, 7, 2, false), 8);
        state.fortunes_of_war = 1;
        assert_eq!(state.resolve_terrain_combat_setup_count(10, 7, 2, false), 3);
    }

    #[test]
    fn terrain_combat_setup_count_rolls_from_resident_prng() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0x1234;
        let mut expected_prng = state.prng_state;
        let expected_count = u5_prng_range_u16(&mut expected_prng, 1, 10) as u8;

        assert_eq!(
            state.roll_terrain_combat_setup_count(10, false),
            expected_count
        );
        assert_eq!(state.prng_state, expected_prng);

        state.fortunes_of_war = 1;
        let mut expected_prng = state.prng_state;
        let _first_roll = u5_prng_range_u16(&mut expected_prng, 1, 10);
        let expected_count = u5_prng_range_u16(&mut expected_prng, 1, 10) as u8;

        assert_eq!(
            state.roll_terrain_combat_setup_count(10, false),
            expected_count
        );
        assert_eq!(state.prng_state, expected_prng);

        let unchanged_prng = state.prng_state;
        assert_eq!(state.roll_terrain_combat_setup_count(16, false), 16);
        assert_eq!(state.roll_terrain_combat_setup_count(10, true), 1);
        assert_eq!(state.prng_state, unchanged_prng);
    }

    #[test]
    fn terrain_combat_public_issue_3_replacement_rows_are_encoded() {
        assert_eq!(
            TERRAIN_COMBAT_REPLACEMENT_TILES_RAW,
            [
                0x21, 0x01, 0x01, 0x03, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0a, 0x04,
                0x0c, 0x0d, 0x0e, 0x0f
            ]
        );
        assert_eq!(terrain_combat_raw_replacement_tile_for_arena(16), None);
    }

    #[test]
    fn terrain_combat_replacement_rolls_only_for_eligible_followers() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0x1234;
        let mut expected_prng = state.prng_state;
        let expected_first =
            u5_prng_range_u16(&mut expected_prng, 0, u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1))
                as u8;
        let expected_second =
            u5_prng_range_u16(&mut expected_prng, 0, u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1))
                as u8;

        let rolls = state.terrain_combat_replacement_roll_seeds(8, Some(0xa4));

        assert_eq!(rolls.len(), 8);
        assert_eq!(rolls[0], 1);
        assert_eq!(rolls[1], expected_first);
        assert_eq!(rolls[2], expected_second);
        assert!(rolls[3..].iter().all(|roll| *roll == 1));
        assert_eq!(state.prng_state, expected_prng);

        let unchanged_prng = state.prng_state;
        assert_eq!(
            state.terrain_combat_replacement_roll_seeds(8, None),
            vec![1; 8]
        );
        assert_eq!(state.prng_state, unchanged_prng);
    }

    #[test]
    fn terrain_combat_tile_replacement_only_applies_to_eligible_followers() {
        assert_eq!(terrain_combat_replacement_threshold(1), 1);
        assert_eq!(terrain_combat_replacement_threshold(8), 3);
        assert_eq!(terrain_combat_replacement_threshold(16), 5);

        assert_eq!(
            terrain_combat_tile_for_spawn_index(0, 8, 0xc0, Some(0xa4), 0),
            0xc0
        );
        assert_eq!(
            terrain_combat_tile_for_spawn_index(1, 8, 0xc0, Some(0xa4), 0),
            0xa4
        );
        assert_eq!(
            terrain_combat_tile_for_spawn_index(2, 8, 0xc0, Some(0xa4), 9),
            0xa4
        );
        assert_eq!(
            terrain_combat_tile_for_spawn_index(3, 8, 0xc0, Some(0xa4), 0),
            0xc0
        );
        assert_eq!(
            terrain_combat_tile_for_spawn_index(1, 8, 0xc0, Some(0xa4), 1),
            0xc0
        );
        assert_eq!(
            terrain_combat_tile_for_spawn_index(1, 8, 0xc0, None, 0),
            0xc0
        );
    }

    #[test]
    fn terrain_combat_setup_from_record_copies_record_slices_and_base_class() {
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let trigger = ActiveObject {
            type_byte: 0x50,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: -1,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };

        let setup =
            terrain_combat_setup_from_record(WorldPlane::Britannia, trigger, &record).unwrap();

        assert_eq!(setup.arena_index, 4);
        assert!(setup.underworld_variant);
        assert_eq!(setup.terrain[10][10], 0xaa);
        assert_eq!(setup.setup_table_a, [0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5]);
        assert_eq!(setup.setup_table_b, [0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5]);
        assert_eq!(setup.base_tile, 0xc0);
        assert_eq!(setup.placement_slots.len(), 16);
        assert_eq!(
            setup.placement_slots.first(),
            Some(&CombatPlacementSlot {
                slot: 0,
                x: 0,
                y: 15,
            })
        );
        assert_eq!(
            setup.placement_slots.last(),
            Some(&CombatPlacementSlot {
                slot: 15,
                x: 15,
                y: 0,
            })
        );
        let base_class = setup.base_class.unwrap();
        assert_eq!(base_class.class, 32);
        assert_eq!(base_class.name, "Orc");
    }

    #[test]
    fn terrain_combat_instance_places_monsters_and_parallel_descriptors() {
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let trigger = ActiveObject {
            type_byte: 0x50,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        let setup =
            terrain_combat_setup_from_record(WorldPlane::Britannia, trigger, &record).unwrap();

        let instance =
            terrain_combat_instance_from_setup(&setup, 8, Some(0xa4), &[0, 0, 1]).unwrap();

        assert_eq!(instance.requested_count, 8);
        assert_eq!(instance.placed_count, 8);
        assert_eq!(instance.unplaced_count, 0);
        assert!(instance.active_objects[..COMBAT_PARTY_ACTOR_SLOTS]
            .iter()
            .all(|object| object.is_empty()));
        assert_eq!(instance.active_objects[6].tile, 0xc0);
        assert_eq!(
            (instance.active_objects[6].x, instance.active_objects[6].y),
            (0, 15)
        );
        assert_eq!(instance.active_objects[6].z, WorldPlane::Britannia.save_floor());
        assert_eq!(instance.actors[6].owner_target_class, 32);
        assert_eq!(instance.actors[6].active_object_slot, 6);
        assert_eq!((instance.actors[6].x, instance.actors[6].y), (0, 15));
        assert!(combat_actor_is_active_not_dead(instance.actors[6]));

        assert_eq!(instance.active_objects[7].tile, 0xa4);
        assert_eq!(instance.actors[7].owner_target_class, 25);
        assert_eq!((instance.actors[7].x, instance.actors[7].y), (1, 14));
        assert_eq!(instance.active_objects[8].tile, 0xc0);
        assert_eq!(instance.actors[8].owner_target_class, 32);
    }

    #[test]
    fn terrain_combat_party_uses_record_slots_after_monsters() {
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let trigger = ActiveObject {
            type_byte: 0x50,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        let setup =
            terrain_combat_setup_from_record(WorldPlane::Britannia, trigger, &record).unwrap();
        let mut instance = terrain_combat_instance_from_setup(&setup, 8, None, &[]).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party[0].class_byte = COMBAT_CLASS_GIANT_RAT;

        state.populate_combat_party_at_placement_slots(
            &mut instance.active_objects,
            &mut instance.actors,
            0,
            &setup.placement_slots,
            usize::from(instance.placed_count),
        );

        assert_eq!(
            (instance.active_objects[0].x, instance.active_objects[0].y),
            (8, 7)
        );
        assert_eq!((instance.actors[0].x, instance.actors[0].y), (8, 7));
        assert_eq!(
            instance.actors[0].base_step,
            combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().speed_seed
        );
    }

    #[test]
    fn terrain_combat_instance_reports_unplaced_count_when_placement_slots_end() {
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let trigger = ActiveObject {
            type_byte: 0x50,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        let setup =
            terrain_combat_setup_from_record(WorldPlane::Britannia, trigger, &record).unwrap();

        let instance = terrain_combat_instance_from_setup(&setup, 26, None, &[]).unwrap();

        assert_eq!(instance.placed_count, 16);
        assert_eq!(instance.unplaced_count, 10);
        assert_eq!(instance.active_objects[6].z, WorldPlane::Underworld.save_floor());
        assert_eq!(instance.active_objects[21].tile, 0xc0);
        assert_eq!(
            (instance.active_objects[21].x, instance.active_objects[21].y),
            (15, 0)
        );
        assert!(instance.active_objects[22].is_empty());
        assert!(instance.actors[21].has_field_lookup_selectable_bit());
        assert!(instance.actors[22].is_empty());
    }

    #[test]
    fn terrain_combat_setup_rejects_objects_without_arena_selection() {
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let trigger = ActiveObject {
            type_byte: 0x10,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };

        let err = terrain_combat_setup_from_record(WorldPlane::Britannia, trigger, &record)
            .expect_err("object has no outdoor arena selector");

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn combat_arena_bank_validates_expected_record_counts() {
        let record = synthetic_combat_arena_record();
        let mut brit = Vec::new();
        for _ in 0..BRIT_CBT_RECORDS {
            brit.extend_from_slice(&record);
        }

        let bank = parse_combat_arena_bank(BRIT_CBT_FILE, &brit, BRIT_CBT_RECORDS).unwrap();

        assert_eq!(bank.resource_name, BRIT_CBT_FILE);
        assert_eq!(bank.records.len(), 16);
        assert!(bank.record(15).is_some());
        assert!(bank.record(16).is_none());
        assert!(parse_combat_arena_bank(BRIT_CBT_FILE, &brit[..brit.len() - 1], BRIT_CBT_RECORDS)
            .is_err());
        assert!(CombatArenaRecord::from_record_bytes(&record[..COMBAT_ARENA_RECORD_LEN - 1])
            .is_err());
    }

    /// Regression: without a save and without `--at`, the world fallback
    /// used to pick the first walkable cell in linear scan order, which
    /// landed on a single-tile island where movement was impossible.
    /// The current rule requires >=5 walkable cells in the 3x3 neighbourhood
    /// so the player can actually explore.
    #[test]
    fn first_world_walkable_skips_isolated_walkable_cells() {
        // Build a world that's all water (sentinel) except a single grass
        // tile near the origin and a 3x3 island of grass further on.
        const GRASS: u8 = 5;
        const WATER: u8 = 1;
        let mut grid = vec![WATER; WORLD_CELLS];

        // Lone island at (10, 0): walkable but no walkable neighbours.
        grid[world_cell_index(10, 0)] = GRASS;

        // 3x3 island centred on (20, 20).
        for dy in 0..3 {
            for dx in 0..3 {
                grid[world_cell_index(20 + dx, 20 + dy)] = GRASS;
            }
        }

        let picked = first_world_walkable_for_transport(
            &grid,
            WorldPlane::Britannia,
            None,
            TransportState::Foot,
            &[],
        )
        .expect("should find the 3x3 island");

        // The 3x3 island's centre (21, 21) has all 8 walkable neighbours;
        // the corners have 3. The earliest-in-scan cell that satisfies the
        // >=5-neighbours rule is the top edge (20, 20) or (21, 20).
        let (x, y) = picked;
        assert!(
            (20..=22).contains(&x) && (20..=22).contains(&y),
            "expected to land on the 3x3 island, got ({x}, {y})"
        );
        assert_ne!(picked, (10, 0), "must not pick the isolated 1x1 island");
    }

    /// LOOK2.DAT cross-check: tile ids 0x0a..=0x0f are six DISTINCT terrain
    /// types -- not all "mountains". Per the canonical labels:
    ///   0x0a tropical forest  (dense forest, blocks sight, IMPASSABLE)
    ///   0x0b foothills        (rolling hills, doesn't block, WALKABLE)
    ///   0x0c mountains        (true mountain, blocks sight, IMPASSABLE)
    ///   0x0d high peaks       (true mountain, blocks sight, IMPASSABLE)
    ///   0x0e foothills        (rolling hills, doesn't block, WALKABLE)
    ///   0x0f foothills        (rolling hills, doesn't block, WALKABLE)
    /// Per u5-spec/catalogs/tile-catalog.md Section 5: mountains are
    /// impassable for everything except the balloon. Foothills are not
    /// mountains -- they are hills, walkable on foot.
    #[test]
    fn foothills_are_walkable_per_look2_dat() {
        assert!(
            is_probe_walkable(0x0b),
            "0x0b 'foothills' must be walkable on foot"
        );
        assert!(
            is_probe_walkable(0x0e),
            "0x0e 'foothills' must be walkable on foot"
        );
        assert!(
            is_probe_walkable(0x0f),
            "0x0f 'foothills' must be walkable on foot"
        );
    }

    #[test]
    fn true_mountains_are_impassable_per_spec() {
        // Tropical forest 0x0a is walkable but blocks sight (see
        // dense_forest_is_walkable_but_blocks_sight). Only mountains
        // and high peaks block on-foot movement.
        assert!(
            !is_probe_walkable(0x0c),
            "0x0c 'mountains' must be impassable on foot"
        );
        assert!(
            !is_probe_walkable(0x0d),
            "0x0d 'high peaks' must be impassable on foot"
        );
    }

    /// Per u5-spec/systems/visibility.md Section 6: forest interior /
    /// mountains block sight; open ground including paths, grass, water,
    /// and HILLS does not. The "see over the mountain from a hill"
    /// mechanic does not exist -- but hills themselves are transparent.
    #[test]
    fn foothills_do_not_block_sight_on_overworld() {
        assert!(
            !world_surface_tile_blocks_sight(0x0b),
            "0x0b foothills must be see-through on the overworld"
        );
        assert!(
            !world_surface_tile_blocks_sight(0x0e),
            "0x0e foothills must be see-through on the overworld"
        );
        assert!(
            !world_surface_tile_blocks_sight(0x0f),
            "0x0f foothills must be see-through on the overworld"
        );
    }

    #[test]
    fn mountains_peaks_and_dense_forest_block_sight_on_overworld() {
        assert!(
            world_surface_tile_blocks_sight(0x0a),
            "0x0a 'tropical forest' (dense) must block sight"
        );
        assert!(
            world_surface_tile_blocks_sight(0x0c),
            "0x0c 'mountains' must block sight"
        );
        assert!(
            world_surface_tile_blocks_sight(0x0d),
            "0x0d 'high peaks' must block sight"
        );
    }

    /// Per u5-spec/systems/animation.md Section 6 the water animator
    /// uses a shared frame selector: every water cell shows the same
    /// frame at the same tick, cycling through the family.
    #[test]
    fn water_animation_cycles_three_frames_shared_selector() {
        let mut clock = AnimationClock::default();
        for tick in 0..9 {
            let resolved = clock.resolve_static_tile(0x01);
            let expected = 0x01 + (tick % 3);
            assert_eq!(
                resolved, expected,
                "tick {tick}: water-family base 0x01 must show frame 0x{expected:02x}"
            );
            clock.tick_static_tiles();
        }
    }

    /// Per u5-spec the water animator runs as part of the per-turn
    /// epilogue. After enough ticks the displayed tile of any single
    /// water cell must cycle through every frame in its family.
    #[test]
    fn water_cells_visit_every_frame_across_ticks() {
        let mut clock = AnimationClock::default();
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..12 {
            seen.insert(clock.resolve_static_tile(0x02));
            clock.tick_static_tiles();
        }
        assert!(
            seen.contains(&0x01) && seen.contains(&0x02) && seen.contains(&0x03),
            "water cell stored as 0x02 must visit 0x01, 0x02, and 0x03 across the cycle, got {seen:?}"
        );
    }

    /// Per u5-spec/systems/animation.md Section 6: "A map cell continues
    /// to mean 'water'; the renderer resolves that semantic tile through
    /// the current water-frame selector at draw time. This keeps the map
    /// stable and makes one frame-counter update affect every visible
    /// cell in the same family."
    /// I.e. the animation is a SHARED FRAME SELECTOR -- at any given
    /// tick, every water-family cell displays the same frame, regardless
    /// of what its stored id is.
    #[test]
    fn water_animation_is_shared_frame_selector() {
        for frame in 0..6u8 {
            let clock = AnimationClock {
                frame,
                moongate_frame: 0,
            };
            let a = clock.resolve_static_tile(0x01);
            let b = clock.resolve_static_tile(0x02);
            let c = clock.resolve_static_tile(0x03);
            assert_eq!(
                a, b,
                "water cells 0x01 and 0x02 must show the same frame at tick {frame}"
            );
            assert_eq!(
                b, c,
                "water cells 0x02 and 0x03 must show the same frame at tick {frame}"
            );
        }
    }

    /// Per actual Ultima V gameplay: swamp tiles are walkable on foot
    /// (you take poison damage stepping through). 0x04 is "swamp" per
    /// LOOK2.DAT. The visual sprite at 0x04 (green dots over blue) is a
    /// distinct terrain type from water; it must NOT participate in the
    /// water animation cycle and must NOT block on-foot movement.
    #[test]
    fn swamp_is_walkable_and_static() {
        assert!(
            is_probe_walkable(0x04),
            "0x04 'swamp' must be walkable on foot"
        );
        for frame in 0..6u8 {
            let clock = AnimationClock {
                frame,
                moongate_frame: 0,
            };
            assert_eq!(
                clock.resolve_static_tile(0x04),
                0x04,
                "swamp must stay 0x04 across all animation frames"
            );
        }
    }

    /// Tropical forest (0x0a) is dense forest interior. Per the
    /// visibility spec Section 6 it blocks sight; per actual U5
    /// gameplay the player CAN walk into a forest -- dense forest just
    /// limits visibility to one cell out before the interior wraps.
    #[test]
    fn dense_forest_is_walkable_but_blocks_sight() {
        assert!(
            is_probe_walkable(0x0a),
            "0x0a 'tropical forest' must be walkable on foot"
        );
        assert!(
            world_surface_tile_blocks_sight(0x0a),
            "0x0a 'tropical forest' must block line of sight"
        );
    }
