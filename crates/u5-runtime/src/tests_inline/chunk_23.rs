#[test]
fn stair_delta_uses_exact_directional_town_klimb_links() {
    assert_eq!(stair_delta(0xc8, ClimbIntent::Up), Some(1));
    assert_eq!(stair_delta(0xc8, ClimbIntent::Down), None);
    assert_eq!(stair_delta(0xc9, ClimbIntent::Down), Some(-1));
    assert_eq!(stair_delta(0xc9, ClimbIntent::Up), None);
    assert_eq!(stair_delta(0x86, ClimbIntent::Down), Some(-1));
    assert_eq!(stair_delta(0x50, ClimbIntent::Up), None);
    assert_eq!(stair_delta(0xc4, ClimbIntent::Up), None);
    assert_eq!(stair_delta(0x8c, ClimbIntent::Down), None);
}

#[test]
fn town_klimb_target_classifiers_are_exhaustive() {
    for tile in 0u8..=u8::MAX {
        assert_eq!(
            town_klimb_underfoot_intent(tile),
            match tile {
                0xc8 => Some(ClimbIntent::Up),
                0xc9 | 0x86 => Some(ClimbIntent::Down),
                _ => None,
            }
        );
        assert_eq!(
            town_klimb_over_target(tile),
            matches!(tile, 0x4c | 0xca | 0xcb)
        );
        assert_eq!(is_town_trapdoor_live_tile(tile), tile == 0x8c);
    }
}

#[test]
fn town_walk_on_stair_delta_uses_facing_selector() {
    assert_eq!(town_walk_on_stair_delta(0xc5, Direction::East), Some(1));
    assert_eq!(town_walk_on_stair_delta(0xc5, Direction::West), Some(-1));
    assert_eq!(town_walk_on_stair_delta(0xc5, Direction::North), None);
    assert_eq!(town_walk_on_stair_delta(0xc5, Direction::NorthEast), None);
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
    let bytes = synthetic_combat_arena_record();
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();

    assert_eq!(record.terrain(0, 0), Some(0));
    assert_eq!(record.terrain(10, 10), Some(0xaa));
    assert_eq!(record.terrain(11, 0), None);
    assert_eq!(record.metadata(0, 11), Some(0x80));
    assert_eq!(record.metadata(0, 10), None);
    assert_eq!(record.row(3).unwrap()[31], 0x83);
    assert_eq!(
        record.outdoor_setup_table_a(),
        [0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5]
    );
    assert_eq!(
        record.outdoor_setup_table_b(),
        [0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5]
    );
    assert_eq!(record.outdoor_placement_x()[15], 15);
    assert_eq!(record.outdoor_placement_y()[15], 0);
    assert_eq!(record.dungeon_room_sources()[0], 0x30);
    assert_eq!(record.dungeon_room_sources()[15], 0x3f);
    assert_eq!(record.dungeon_room_source_x()[15], 15);
    assert_eq!(record.dungeon_room_source_y()[15], 0);
    assert_eq!(dungeon_room_party_position_row(3), 1);
    assert_eq!(dungeon_room_party_position_row(1), 2);
    assert_eq!(dungeon_room_party_position_row(0), 3);
    assert_eq!(dungeon_room_party_position_row(5), 3);
    assert_eq!(dungeon_room_party_position_row(2), 4);
    assert_eq!(
        record.dungeon_room_party_positions_for_seed(0)[5],
        (0xa5, 0xb5)
    );
    assert_eq!(record.dungeon_room_setup_sources().len(), 16);
    assert_eq!(record.terrain_grid()[10][10], 0xaa);
    assert_eq!(record.record_bytes().as_slice(), bytes.as_slice());
}

#[test]
/// `dungeon-mode.md §14.1`: the wandering-monster launch synthesises
/// the arena and its metadata band rather than duelling one monster
/// at a fixed cell. Monsters enter on the side the party faces and
/// the party is seated on the row that facing selects, behind it.
fn dungeon_active_monster_combat_synthesises_published_ambush_arena() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::North;
    let object = ActiveObject {
        type_byte: 0,
        tile: 0,
        x: 2,
        y: 1,
        z: 3,
        phase: STEADY_PHASE,
        aux1: 20,
        aux3: DUNGEON_MONSTER_UPPER_DEP3,
    };

    let note = state
        .enter_dungeon_active_monster_combat(3, object)
        .unwrap();

    assert!(note.contains("active monster"));
    assert!(state.combat_active);
    assert!(state.combat_terrain.iter().all(|row| {
        row.iter()
            .all(|tile| *tile == DUNGEON_AMBUSH_ARENA_FLOOR_TILE)
    }));

    // Giant Rat's spawn-count stat byte is ten and is not one of the
    // exact-count sentinels, so the launch rolls `[1, 10]`.
    let stats = combat_class_stats(20).unwrap();
    assert_eq!(stats.default_spawn_count, 10);
    let placed = (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
        .filter(|slot| !state.combat_actors[*slot].is_empty())
        .count();
    assert!(placed >= 1 && placed <= usize::from(stats.default_spawn_count));

    // `combat.md §5`: descriptors six and up, but renderer records
    // "continue from the first record left free by the seated
    // party", so read each monster's record through its link byte.
    // The stock roster is one live member, so the first monster's
    // record is one and the run is contiguous from there.
    assert_eq!(
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].active_object_slot,
        1
    );
    for slot in COMBAT_PARTY_ACTOR_SLOTS..COMBAT_PARTY_ACTOR_SLOTS + placed {
        let actor = state.combat_actors[slot];
        let record = usize::from(actor.active_object_slot);
        assert_eq!(actor.owner_target_class, 20);
        assert_eq!(
            state.active_objects[record].tile,
            combat_class_sprite_byte(20)
        );
        assert!(
            DUNGEON_AMBUSH_SOURCE_X_NORTH
                .iter()
                .zip(DUNGEON_AMBUSH_SOURCE_Y_NORTH.iter())
                .any(|(x, y)| *x == actor.x && *y == actor.y),
            "monster at ({}, {}) is not on a published facing-north source cell",
            actor.x,
            actor.y
        );
    }

    if !state.combat_actors[0].is_empty() {
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 6));
    }
}

#[test]
fn dungeon_room_setup_sources_classify_sources_with_high_bit_mask() {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    bytes[source_base] = 0x00;
    bytes[source_base + 1] = 0x3c;
    bytes[source_base + 2] = 0x44;
    bytes[source_base + 3] = 0xb4;
    bytes[source_base + 4] = 0xc4;
    bytes[source_base + 5] = 0xe8;
    bytes[source_base + 6] = 0xec;
    bytes[source_base + 7] = 0xef;
    let source_x_base =
        DUNGEON_ROOM_SOURCE_X_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    let source_y_base =
        DUNGEON_ROOM_SOURCE_Y_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    for offset in 0..DUNGEON_ROOM_SOURCE_COUNT {
        bytes[source_x_base + offset] = (offset + 10) as u8;
        bytes[source_y_base + offset] = (offset + 20) as u8;
    }
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();

    let sources = record.dungeon_room_setup_sources();

    assert_eq!(
        sources,
        vec![
            DungeonRoomSetupSource {
                slot: 1,
                source: 0x3c,
                x: 11,
                y: 21,
                kind: DungeonRoomSetupSourceKind::AbsorbableField,
            },
            DungeonRoomSetupSource {
                slot: 2,
                source: 0x44,
                x: 12,
                y: 22,
                kind: DungeonRoomSetupSourceKind::OrdinaryCombatant {
                    setup_class: 1,
                    palette_selector: None,
                },
            },
            DungeonRoomSetupSource {
                slot: 3,
                source: 0xb4,
                x: 13,
                y: 23,
                kind: DungeonRoomSetupSourceKind::SpecialPlacement(
                    DungeonRoomSpecialPlacement::from_setup_id(0xb4),
                ),
            },
            DungeonRoomSetupSource {
                slot: 4,
                source: 0xc4,
                x: 14,
                y: 24,
                kind: DungeonRoomSetupSourceKind::OrdinaryCombatant {
                    setup_class: 33,
                    palette_selector: None,
                },
            },
            DungeonRoomSetupSource {
                slot: 5,
                source: 0xe8,
                x: 15,
                y: 25,
                kind: DungeonRoomSetupSourceKind::SpecialPlacement(
                    DungeonRoomSpecialPlacement::from_setup_id(0xe8),
                ),
            },
            DungeonRoomSetupSource {
                slot: 6,
                source: 0xec,
                x: 16,
                y: 26,
                kind: DungeonRoomSetupSourceKind::OrdinaryCombatant {
                    setup_class: 43,
                    palette_selector: Some(0),
                },
            },
            DungeonRoomSetupSource {
                slot: 7,
                source: 0xef,
                x: 17,
                y: 27,
                kind: DungeonRoomSetupSourceKind::OrdinaryCombatant {
                    setup_class: 43,
                    palette_selector: Some(3),
                },
            },
        ]
    );
    assert_eq!(dungeon_room_ordinary_setup_class(0x44), Some(1));
    assert_eq!(dungeon_room_ordinary_setup_class(0xc4), Some(33));
    assert_eq!(dungeon_room_ordinary_setup_class(0xb4), None);
    // `formats/cbt.md §5`: "The `0xEC..0xEF` family is not excluded
    // by this test and therefore takes this path."
    assert_eq!(dungeon_room_ordinary_setup_class(0xec), Some(43));
    assert_eq!(dungeon_room_ordinary_setup_class(0xef), Some(43));
    assert_eq!(dungeon_room_ordinary_setup_class(0xe8), None);
    assert_eq!(dungeon_room_vermin_palette_selector(0xec), Some(0));
    assert_eq!(dungeon_room_vermin_palette_selector(0xef), Some(3));
    assert_eq!(dungeon_room_vermin_palette_selector(0xe8), None);
    assert_eq!(
        dungeon_room_special_post_write(1),
        DungeonRoomSpecialPostWrite::LevelTimesThreePlusSeven
    );
    assert_eq!(
        dungeon_room_special_post_write(2),
        DungeonRoomSpecialPostWrite::LevelScaledRandom
    );
    assert_eq!(
        dungeon_room_special_post_write(15),
        DungeonRoomSpecialPostWrite::RandomRange { low: 1, high: 7 }
    );
    assert_eq!(
        dungeon_room_special_post_write(16),
        DungeonRoomSpecialPostWrite::None
    );
    assert_eq!(dungeon_room_source_sprite(0x44), Some(0x44));
    assert_eq!(dungeon_room_source_sprite(0xc4), Some(0xc4));
    assert_eq!(dungeon_room_source_sprite(0xb4), None);
    assert_eq!(dungeon_room_source_sprite(0xe8), None);
    assert_eq!(dungeon_room_source_sprite(0xec), None);
    assert_eq!(dungeon_room_source_sprite(0xef), None);
    assert!(dungeon_room_absorbable_field_family(0x3c));
    assert!(dungeon_room_absorbable_field_family(0x3f));
    assert!(!dungeon_room_absorbable_field_family(0x38));
    assert!(!dungeon_room_absorbable_field_family(0x40));
}

#[test]
fn dungeon_room_combat_setup_copies_terrain_and_scanned_sources() {
    let mut bytes = synthetic_combat_arena_record();
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
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
                x: 1,
                y: 14,
                kind: DungeonRoomSetupSourceKind::AbsorbableField,
            },
            DungeonRoomSetupSource {
                slot: 2,
                source: 0x44,
                x: 2,
                y: 13,
                kind: DungeonRoomSetupSourceKind::OrdinaryCombatant {
                    setup_class: 1,
                    palette_selector: None,
                },
            },
        ]
    );

    let instance = dungeon_room_combat_instance_from_setup(&setup, 7);
    assert_eq!(
        instance.active_objects[6].type_byte,
        DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE
    );
    assert_eq!(
        instance.active_objects[6].tile,
        DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE
    );
    assert!(!instance.actors[6].is_empty());
    assert_eq!(instance.active_objects[7].tile, 0x44);
    assert_eq!(
        (instance.active_objects[7].x, instance.active_objects[7].y),
        (2, 13)
    );
    assert!(instance.actors[7].is_empty());
}

#[test]
fn dungeon_room_combat_setup_uses_source_coordinates_and_facing_party_row() {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    bytes[source_base] = 0x02;
    bytes[source_base + 1] = 0xc4;
    bytes[source_base + 2] = 0x44;
    bytes[source_base + 3] = DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE;
    for index in 0..CBT_PLACEMENT_SLOT_COUNT {
        bytes[DUNGEON_ROOM_SOURCE_X_ROW * COMBAT_ARENA_ROW_STRIDE
            + COMBAT_ARENA_METADATA_START
            + index] = (index + 1) as u8;
        bytes[DUNGEON_ROOM_SOURCE_Y_ROW * COMBAT_ARENA_ROW_STRIDE
            + COMBAT_ARENA_METADATA_START
            + index] = (index + 20) as u8;
    }
    for slot in 0..COMBAT_PARTY_ACTOR_SLOTS {
        bytes[2 * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_PARTY_COLUMN_X + slot] = (30 + slot) as u8;
        bytes[2 * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_PARTY_COLUMN_Y + slot] = (40 + slot) as u8;
    }
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
    let setup = dungeon_room_combat_setup_from_record_for_entry(3, &record, 1, true);

    let mut instance = dungeon_room_combat_instance_from_setup(&setup, 4);

    assert_eq!(instance.requested_count, 4);
    assert_eq!(instance.placed_count, 4);
    assert_eq!(instance.unplaced_count, 0);
    assert_eq!(
        (
            instance.active_objects[6].tile,
            instance.active_objects[6].x,
            instance.active_objects[6].y
        ),
        (0x02, 1, 20)
    );
    assert!(!instance.actors[6].is_empty());
    assert_eq!(
        (
            instance.active_objects[7].tile,
            instance.active_objects[7].x,
            instance.active_objects[7].y
        ),
        (0xc4, 2, 21)
    );
    assert_eq!(
        (
            instance.active_objects[8].tile,
            instance.active_objects[8].x,
            instance.active_objects[8].y
        ),
        (0x44, 3, 22)
    );
    assert!(!instance.actors[7].is_empty());
    assert!(instance.actors[8].is_empty());
    assert_eq!(
        (
            instance.active_objects[9].tile,
            instance.active_objects[9].x,
            instance.active_objects[9].y
        ),
        (DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE, 4, 23)
    );
    assert!(instance.actors[9].is_empty());

    let mut state = test_state(open_grid(), 1, 1);
    let mut second = state.party[0];
    second.slot = 1;
    second.class_byte = b'F';
    state.party.push(second);
    state.populate_dungeon_room_combat_party(
        &mut instance.active_objects,
        &mut instance.actors,
        4,
        &setup.party_positions,
    );

    assert_eq!(
        (instance.active_objects[0].x, instance.active_objects[0].y),
        (30, 40)
    );
    assert_eq!((instance.actors[0].x, instance.actors[0].y), (30, 40));
    assert_eq!(instance.actors[0].owner_target_class, 0);
    assert_eq!(instance.actors[0].active_object_slot, 0);
    assert_eq!(instance.actors[1].owner_target_class, 1);
    assert_eq!(instance.actors[1].active_object_slot, 1);
}

#[test]
/// `formats/cbt.md §5` + `dungeon-mode.md §14`: "**The random-special
/// family `0xEC..0xEF` is an ordinary placement, not a marker.**"
/// It allocates both a descriptor and an active-object record, and
/// its tile follows the substituted class.
fn dungeon_room_vermin_family_spawns_ordinary_actors_from_the_palette() {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    bytes[source_base] = DUNGEON_ROOM_SPECIAL_SOURCE_EC;
    bytes[source_base + 1] = 0xef;
    bytes[source_base + 2] = 0xc4;
    for index in 0..DUNGEON_ROOM_SOURCE_COUNT {
        bytes[DUNGEON_ROOM_SOURCE_X_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + index] = (index + 1) as u8;
        bytes[DUNGEON_ROOM_SOURCE_Y_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + index] = (index + 2) as u8;
    }
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
    let setup = dungeon_room_combat_setup_from_record(0, &record);

    let mut expected_prng = 0;
    let expected_random_ids = dungeon_room_random_special_setup_ids(true, &mut expected_prng);
    let instance = dungeon_room_combat_instance_from_setup(&setup, 0);

    assert_eq!(instance.requested_count, 3);
    assert_eq!(instance.placed_count, 3);
    // Source `0xEC` takes palette id 0, `0xEF` takes palette id 3,
    // and each is placed on the ordinary path with a full combat
    // descriptor and the tile derived from the substituted class.
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile,
        combat_class_sprite_byte(expected_random_ids[0])
    );
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 1].tile,
        combat_class_sprite_byte(expected_random_ids[3])
    );
    assert!(!instance.actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty());
    assert_eq!(
        instance.actors[COMBAT_PARTY_ACTOR_SLOTS].owner_target_class,
        expected_random_ids[0]
    );
    assert!(!instance.actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_empty());
    assert_eq!(
        instance.actors[COMBAT_PARTY_ACTOR_SLOTS + 1].owner_target_class,
        expected_random_ids[3]
    );
    // Ordinary placements store the substituted class's starting HP.
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS].aux1,
        combat_class_stats(expected_random_ids[0]).unwrap().max_hp
    );
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 2].tile,
        0xc4
    );
    assert!(!instance.actors[COMBAT_PARTY_ACTOR_SLOTS + 2].is_empty());
}

#[test]
fn dungeon_room_special_setup_ids_keep_public_post_write_shape() {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    bytes[source_base] = 0x01;
    bytes[source_base + 1] = 0x02;
    bytes[source_base + 2] = 0x0f;
    bytes[source_base + 3] = 0x10;
    for index in 0..DUNGEON_ROOM_SOURCE_COUNT {
        bytes[DUNGEON_ROOM_SOURCE_X_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + index] = index as u8;
        bytes[DUNGEON_ROOM_SOURCE_Y_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + index] = (COMBAT_ARENA_SIDE - 1 - index.min(COMBAT_ARENA_SIDE - 1)) as u8;
    }
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
    let setup = dungeon_room_combat_setup_from_record(0, &record);

    assert_eq!(
        setup.setup_sources[0].kind,
        DungeonRoomSetupSourceKind::SpecialPlacement(DungeonRoomSpecialPlacement {
            setup_id: 1,
            post_write: DungeonRoomSpecialPostWrite::LevelTimesThreePlusSeven,
        })
    );
    assert_eq!(
        setup.setup_sources[1].kind,
        DungeonRoomSetupSourceKind::SpecialPlacement(DungeonRoomSpecialPlacement {
            setup_id: 2,
            post_write: DungeonRoomSpecialPostWrite::LevelScaledRandom,
        })
    );
    assert_eq!(
        setup.setup_sources[2].kind,
        DungeonRoomSetupSourceKind::SpecialPlacement(DungeonRoomSpecialPlacement {
            setup_id: 15,
            post_write: DungeonRoomSpecialPostWrite::RandomRange { low: 1, high: 7 },
        })
    );
    assert_eq!(
        setup.setup_sources[3].kind,
        DungeonRoomSetupSourceKind::SpecialPlacement(DungeonRoomSpecialPlacement {
            setup_id: 16,
            post_write: DungeonRoomSpecialPostWrite::None,
        })
    );

    let mut expected_prng = 0;
    let _expected_random_ids = dungeon_room_random_special_setup_ids(true, &mut expected_prng);
    let expected_id2_aux = u5_prng_range_u16(&mut expected_prng, 1, 30) as u8;
    let expected_id15_aux = u5_prng_range_u16(&mut expected_prng, 1, 7) as u8;
    let mut actual_prng = 0;
    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 2, &mut actual_prng);

    assert_eq!(instance.requested_count, 4);
    assert_eq!(instance.placed_count, 4);
    for (offset, expected_tile) in [0x01, 0x02, 0x0f, 0x10].into_iter().enumerate() {
        let slot = COMBAT_PARTY_ACTOR_SLOTS + offset;
        assert_eq!(instance.active_objects[slot].tile, expected_tile);
        assert!(instance.actors[slot].is_empty());
    }
    assert_eq!(instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS].aux1, 13);
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 1].aux1,
        expected_id2_aux
    );
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 2].aux1,
        expected_id15_aux
    );
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 3].aux1,
        16
    );
    assert_eq!(actual_prng, expected_prng);
}

#[test]
fn dungeon_room_helper_setup_skips_source_scan_but_keeps_party_row() {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    let source_base =
        DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
    bytes[source_base] = 0xc4;
    for slot in 0..COMBAT_PARTY_ACTOR_SLOTS {
        bytes[3 * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_PARTY_COLUMN_X + slot] = (50 + slot) as u8;
        bytes[3 * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_PARTY_COLUMN_Y + slot] = (60 + slot) as u8;
    }
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();

    let setup = dungeon_room_combat_setup_from_record_for_entry(4, &record, 0, false);

    assert!(setup.setup_sources.is_empty());
    assert_eq!(setup.party_positions[0], (50, 60));
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
    assert_eq!(
        guild_unit_price(GuildShop::TheDen, GuildCommodity::Keys),
        190
    );
    assert_eq!(
        guild_unit_price(GuildShop::TheDen, GuildCommodity::Gems),
        255
    );
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

    let bought = apply_guild_purchase(
        &mut gold,
        &mut gems,
        GuildShop::TheGuild,
        GuildCommodity::Gems,
        2,
    )
    .unwrap();

    assert_eq!(bought.quote.total_price, 400);
    assert_eq!(bought.gold_before, 400);
    assert_eq!(bought.gold_after, 0);
    assert_eq!(bought.stock_before, 97);
    assert_eq!(bought.stock_after, 99);
    assert_eq!(gold, 0);
    assert_eq!(gems, SHOP_COMMODITY_STOCK_CAP);

    assert_eq!(
        apply_guild_purchase(
            &mut gold,
            &mut gems,
            GuildShop::TheGuild,
            GuildCommodity::Gems,
            1
        ),
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
        shipwright_price(
            Shipwright::IslandShipwrights,
            ShipwrightPurchaseKind::Frigate
        ),
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
fn shipwright_delivery_coordinates_are_the_published_table_rows() {
    // `shops.md §8.7`: the delivery cell is table data held beside the
    // price rows, never the scene's exterior entrance/exit cell.
    assert_eq!(
        shipwright_delivery_coordinate(Shipwright::IslandShipwrights),
        (39, 221)
    );
    assert_eq!(
        shipwright_delivery_coordinate(Shipwright::TheCrowsNest),
        (151, 21)
    );
    assert_eq!(
        shipwright_delivery_coordinate(Shipwright::TheOakenOar),
        (79, 109)
    );
    assert_eq!(
        shipwright_delivery_coordinate(Shipwright::TheRustyBucket),
        (138, 159)
    );
}

#[test]
fn arms_no_credit_bark_pool_is_the_published_verbatim_four() {
    // `shops.md §8.1` draw table / `§8.A` resident-literal table.
    assert_eq!(
        crate::shops::arms_no_credit_bark_for_roll(0),
        "Can't pay?! Out with ye, orc-face!"
    );
    assert_eq!(
        crate::shops::arms_no_credit_bark_for_roll(1),
        "What be ye trying to pull? OUT!"
    );
    assert_eq!(crate::shops::arms_no_credit_bark_for_roll(2), "OUT, SLIME!");
    assert_eq!(crate::shops::arms_no_credit_bark_for_roll(3), "BEAT IT!");
    // The draw is a two-bit selector, so higher rolls wrap.
    assert_eq!(
        crate::shops::arms_no_credit_bark_for_roll(4),
        crate::shops::arms_no_credit_bark_for_roll(0)
    );
}

#[test]
fn arms_no_credit_bark_carries_the_yells_attribution_tail() {
    assert_eq!(
        crate::shops::arms_no_credit_bark_with_attribution(
            crate::shops::arms_no_credit_bark_for_roll(2),
            "the smith",
        ),
        "OUT, SLIME!\nyells the smith."
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
fn queued_frigate_skiff_purchase_increments_the_whole_packed_class_byte() {
    let mut gold = 500;
    let mut pending = Some(PendingVehicleAcquisition::Frigate {
        x: 12,
        y: 21,
        skiffs: 0x3f,
    });

    apply_shipwright_purchase(
        &mut gold,
        &mut pending,
        Shipwright::TheRustyBucket,
        ShipwrightPurchaseKind::Skiff,
        99,
        99,
    )
    .unwrap();

    let pending = pending.unwrap();
    assert_eq!(
        PendingVehicleSaveState::from_acquisition(pending).class_byte,
        0xc0
    );
    assert_eq!(pending.active_object(0).aux3, 0);

    let mut pending = Some(PendingVehicleAcquisition::Frigate {
        x: 12,
        y: 21,
        skiffs: 0x7f,
    });
    apply_shipwright_purchase(
        &mut gold,
        &mut pending,
        Shipwright::TheRustyBucket,
        ShipwrightPurchaseKind::Skiff,
        99,
        99,
    )
    .unwrap();
    assert_eq!(pending, None);
}

#[test]
fn shipwright_purchase_refusals_preserve_gold_and_pending_delivery() {
    let mut gold = 99;
    let mut pending = Some(PendingVehicleAcquisition::Skiff {
        x: 1,
        y: 2,
        aux3: 0,
    });

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
    assert_eq!(
        pending,
        Some(PendingVehicleAcquisition::Skiff {
            x: 1,
            y: 2,
            aux3: 0,
        })
    );

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
    assert_eq!(
        pending,
        Some(PendingVehicleAcquisition::Skiff {
            x: 1,
            y: 2,
            aux3: 0,
        })
    );
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
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(state.active_objects[1].type_byte, SHIP_PARKED_FIRST);
    assert_eq!(state.active_objects[1].tile, FIRST_PLAYABLE_FRIGATE_TILE);
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (12, 21)
    );
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
        state
            .boardable_vehicle_slot_at(9, 8)
            .map(|candidate| candidate.transport),
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

    // active-objects.md §4: only 0xB5 is universally protected and
    // rejected by every eviction phase, last resort included. A table
    // packed with anything else is now recoverable by the cascade, so
    // 0xB5 is what a genuine "no slot" case looks like.
    let mut full = test_state(open_grid(), 3, 4);
    full.gold = 500;
    full.active_objects = (0..OOL_SLOTS)
        .map(|slot| ActiveObject {
            type_byte: ACTIVE_OBJECT_PROTECTED_TYPE_BYTE,
            tile: 1,
            x: slot,
            y: 0,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        })
        .collect();

    // Every ordinary slot holds the protected byte, so no phase --
    // last resort included -- has a candidate. This is the only shape
    // of genuine refusal left now that the cascade is wired; the
    // evicting case is `horse_purchase_evicts_a_low_priority_slot_when_the_range_is_full`.
    assert_eq!(
        full.buy_horse(Stable::HorseAndRider, 9, 8),
        Err(HorsePurchaseError::NoActiveObjectSlot)
    );
    assert_eq!(full.gold, 500);
    assert!(
        full.active_objects
            .iter()
            .all(|object| object.type_byte == ACTIVE_OBJECT_PROTECTED_TYPE_BYTE)
    );

    // `0xB5` is the only universally protected byte-0 value, so a
    // table made entirely of it is the single genuine no-slot case.
    let mut protected = test_state(open_grid(), 3, 4);
    protected.gold = 500;
    protected.active_objects = (0..OOL_SLOTS)
        .map(|slot| ActiveObject {
            type_byte: ACTIVE_OBJECT_PROTECTED_TYPE_BYTE,
            tile: ACTIVE_OBJECT_PROTECTED_TYPE_BYTE,
            x: slot,
            y: 0,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        })
        .collect();

    assert_eq!(
        protected.buy_horse(Stable::HorseAndRider, 9, 8),
        Err(HorsePurchaseError::NoActiveObjectSlot)
    );
    assert_eq!(protected.gold, 500);
    assert!(
        !protected
            .active_objects
            .iter()
            .any(|object| object.type_byte == HORSE_PARKED_FIRST)
    );
}

#[test]
fn horse_purchase_evicts_a_low_priority_slot_when_the_range_is_full() {
    // active-objects.md §4: "if the ordinary range is full,
    // acquisition can evict a lower-priority object." Before this was
    // wired, a full table silently failed the purchase.
    let mut state = test_state(open_grid(), 3, 4);
    state.gold = 500;
    state.active_objects = (0..OOL_SLOTS)
        .map(|slot| ActiveObject {
            type_byte: 0x05,
            tile: 0x05,
            x: slot,
            y: 0,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        })
        .collect();

    let bought = state
        .buy_horse(Stable::HorseAndRider, 9, 8)
        .expect("full ordinary range must evict rather than refuse");
    assert_eq!(state.gold, 400);
    // Phase 2 is scenery 0x01..=0x0F **off-screen only**, so it takes
    // the lowest ordinary slot that is outside the player's square
    // on-screen window. The fixture puts slot N at (N, 0) with the
    // player at (3, 4): |N - 3| first exceeds the five-cell half-window
    // at N = 9, so slot 9 is the victim and slots 1..=8 -- on-screen
    // and therefore ineligible in phase 2 -- survive. That the
    // off-screen gate, not merely the lowest index, picks the victim is
    // the point of this assertion.
    assert_eq!(bought.active_object_slot, 9);
    assert_eq!(state.active_objects[9].type_byte, HORSE_PARKED_FIRST);
    for slot in ACTIVE_OBJECT_ORDINARY_FIRST..9 {
        assert_eq!(
            state.active_objects[slot].type_byte, 0x05,
            "on-screen slot {slot} is not eligible for off-screen phase 2"
        );
    }
    // active-objects.md §4: slot 0 is the player and the reserved
    // band 24..=31 sits outside the allocator; neither is ever a
    // victim.
    assert_eq!(
        state.active_objects[ACTIVE_OBJECT_PLAYER_SLOT].type_byte,
        0x05
    );
    for slot in ACTIVE_OBJECT_RESERVED_FIRST..=ACTIVE_OBJECT_RESERVED_LAST {
        assert_eq!(state.active_objects[slot].type_byte, 0x05);
    }
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
    state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT], [1, 2, 3, 4, 5, 6]];

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
    assert_eq!(state.pickup_inn_guest(0x11, 0, 2), Err(InnError::PartyFull));
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
    assert_eq!(
        state.clock,
        GameClock::with_date(500, 2, 28, 23, 59).unwrap()
    );
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
    assert_eq!(tavern_provision_unit_price(Tavern::TheSlaughteredLamb), 25);
    assert_eq!(tavern_provision_unit_price(Tavern::TheHumblePalate), 30);
    assert_eq!(tavern_provision_unit_price(Tavern::TheBlueBoarTavern), 25);
    assert_eq!(tavern_provision_unit_price(Tavern::TheCatsLair), 20);
    assert_eq!(tavern_provision_unit_price(Tavern::TheFallenVirgin), 25);
    assert_eq!(tavern_provision_unit_price(Tavern::TheFolleyTap), 30);

    let humble = quote_tavern_round_drink(Tavern::TheHumblePalate, 3).unwrap();
    assert_eq!(humble.menu_letter, 'F');
    assert_eq!(humble.unit_price, 2);
    assert_eq!(humble.total_price, 6);

    let lamb = quote_tavern_round_drink(Tavern::TheSlaughteredLamb, 2).unwrap();
    assert_eq!(lamb.menu_letter, 'B');
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

    let drank = state
        .buy_tavern_round_drink(Tavern::TheSwordAndKeg)
        .unwrap();

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
fn provision_purchase_adds_twenty_five_per_pack_and_stops_when_gold_runs_out() {
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
    assert_eq!(bought.food_after, 110);
    assert_eq!(
        bought.completion,
        ProvisionPurchaseCompletion::GoldExhausted
    );
    assert_eq!(gold, 5);
    assert_eq!(food, 110);

    let mut state = world_state(open_world_grid(), 10, 10);
    state.gold = 1000;
    state.food = SHOP_FOOD_STOCK_CAP - 2;

    let capped = state
        .buy_provisions(Tavern::TheHonestMeal, 5)
        .expect("one pack reaches the food ceiling");

    assert_eq!(capped.purchased_quantity, 1);
    assert_eq!(capped.total_price, 10);
    assert_eq!(capped.completion, ProvisionPurchaseCompletion::Completed);
    assert_eq!(state.food, SHOP_FOOD_STOCK_CAP);
    assert_eq!(state.gold, 990);
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
        Err(ProvisionPurchaseError::NoNeed)
    );
    assert_eq!((gold, food), (9, 12));

    food = 2;
    let charity = apply_provision_purchase(&mut gold, &mut food, Tavern::TheHonestMeal, 1).unwrap();
    assert_eq!(charity.purchased_quantity, 0);
    assert_eq!(charity.total_price, 0);
    assert_eq!(charity.food_after, 3);
    assert_eq!(charity.completion, ProvisionPurchaseCompletion::Charity);
    assert_eq!((gold, food), (9, 3));
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
        .consult_sage_rumour(&SAGE_RUMOUR_TABLE, "HONE", SAGE_RUMOUR_SUCCESS_RECORD_FIRST)
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
    const EXPECTED_ROWS: &[(u8, &str, [u8; 8])] = &[
        (0, "Mage", [10, 15, 20, 0, 15, 10, 3, 20]),
        (1, "Bard", [15, 20, 10, 4, 12, 15, 9, 10]),
        (2, "Fighter", [20, 15, 10, 8, 15, 20, 6, 15]),
        (3, "Avatar", [25, 25, 25, 7, 30, 20, 1, 25]),
        (4, "Villager", [12, 12, 12, 0, 6, 8, 1, 10]),
        (5, "Merchant", [12, 12, 18, 0, 6, 8, 1, 10]),
        (6, "Jester", [12, 18, 12, 0, 6, 8, 1, 10]),
        (7, "Bard (second row)", [12, 16, 14, 0, 6, 8, 1, 10]),
        (8, "Pirate", [12, 12, 12, 0, 0, 5, 1, 0]),
        (9, "Unnamed reserved", [12, 12, 12, 0, 0, 5, 1, 0]),
        (10, "Child", [8, 8, 8, 0, 0, 5, 1, 0]),
        (11, "Beggar", [8, 8, 8, 0, 0, 5, 1, 0]),
        (12, "Guard", [22, 30, 10, 6, 30, 99, 8, 5]),
        (13, "Wanderer", [30, 30, 30, 30, 99, 99, 1, 0]),
        (14, "Blackthorn", [30, 30, 30, 30, 30, 99, 1, 0]),
        (15, "Lord British", [30, 30, 30, 30, 99, 99, 1, 0]),
        (16, "Sea Horse", [17, 20, 20, 2, 10, 30, 3, 0]),
        (17, "Squid", [24, 20, 8, 0, 20, 50, 2, 0]),
        (18, "Sea Serpent", [17, 17, 8, 2, 30, 70, 1, 0]),
        (19, "Shark", [20, 17, 5, 0, 8, 22, 10, 0]),
        (20, "Giant Rat", [5, 20, 5, 0, 6, 10, 10, 5]),
        (21, "Bat", [5, 30, 5, 0, 6, 5, 16, 0]),
        (22, "Giant Spider", [10, 10, 5, 0, 8, 10, 4, 5]),
        (23, "Ghost", [1, 20, 10, 0, 12, 20, 6, 0]),
        (24, "Slime", [6, 6, 2, 0, 4, 10, 16, 0]),
        (25, "Gremlin", [10, 21, 10, 2, 4, 10, 13, 12]),
        (26, "Mimic", [20, 30, 12, 3, 15, 30, 1, 20]),
        (27, "Reaper", [20, 25, 12, 4, 20, 40, 3, 25]),
        (28, "Gazer", [8, 10, 25, 0, 10, 20, 4, 0]),
        (29, "Crawler", [17, 15, 12, 0, 15, 35, 4, 0]),
        (30, "Gargoyle", [20, 10, 5, 15, 20, 40, 1, 0]),
        (31, "Insect Swarm", [1, 30, 1, 0, 4, 5, 10, 0]),
        (32, "Orc", [15, 13, 10, 2, 12, 10, 10, 11]),
        (33, "Skeleton", [10, 20, 5, 0, 12, 20, 8, 13]),
        (34, "Python", [5, 18, 8, 1, 8, 10, 4, 0]),
        (35, "Ettin", [20, 15, 12, 3, 15, 30, 6, 17]),
        (36, "Headless", [19, 12, 8, 2, 12, 20, 8, 12]),
        (37, "Wisp", [8, 30, 20, 0, 20, 40, 4, 0]),
        (38, "Daemon", [25, 25, 25, 5, 20, 75, 4, 0]),
        (39, "Dragon", [30, 25, 25, 10, 30, 99, 2, 30]),
        (40, "Sand Trap", [25, 25, 5, 10, 30, 80, 1, 25]),
        (41, "Troll", [18, 17, 9, 4, 15, 15, 4, 15]),
        (42, "Reserved gap", [0; 8]),
        (43, "Reserved gap", [0; 8]),
        (44, "Mongbat", [10, 30, 15, 4, 20, 20, 16, 5]),
        (45, "Corpser", [17, 10, 8, 0, 15, 40, 4, 0]),
        (46, "Rot Worm", [5, 17, 6, 0, 6, 5, 10, 0]),
        (47, "Shadow Lord", [25, 30, 30, 10, 30, 99, 1, 0]),
    ];

    for &(class, name, row) in EXPECTED_ROWS {
        let stats = combat_class_stats(class).unwrap();
        assert_eq!(stats.name, name, "class {class} name");
        assert_eq!(stats.raw_row(), row, "class {class} stat row");
        assert_eq!(
            stats.reward_unit(),
            row[5] / 4 + 1,
            "class {class} reward unit"
        );
        assert_eq!(
            stats.mass_charm_threshold(),
            row[2],
            "class {class} charm threshold"
        );
    }

    assert_eq!(combat_class_stats(COMBAT_CLASS_COUNT as u8), None);
}

#[test]
fn combat_ranged_effect_stats_expose_published_side_rows() {
    const EXPECTED_ROWS: &[(u8, &str, u8, u8, bool, bool, bool)] = &[
        (0, "Mage", 7, 4, true, false, false),
        (12, "Guard", 15, 2, false, false, false),
        (13, "Wanderer", 9, 4, true, false, false),
        (14, "Blackthorn", 9, 3, true, false, false),
        (15, "Lord British", 9, 4, true, false, false),
        (16, "Sea Horse", 5, 4, true, false, false),
        (17, "Squid", 7, 4, false, false, false),
        (18, "Sea Serpent", 9, 3, false, false, false),
        (19, "Shark", 1, 0, false, false, false),
        (20, "Giant Rat", 1, 0, false, false, false),
        (21, "Bat", 1, 0, false, false, false),
        (22, "Giant Spider", 1, 0, false, false, false),
        (23, "Ghost", 1, 0, false, false, false),
        (24, "Slime", 1, 0, false, false, false),
        (25, "Gremlin", 1, 0, false, true, false),
        (26, "Mimic", 2, 5, false, false, true),
        (27, "Reaper", 9, 4, true, false, false),
        (28, "Gazer", 5, 6, true, false, false),
        (29, "Crawler", 1, 0, false, false, false),
        (30, "Gargoyle", 9, 7, false, false, false),
        (31, "Insect Swarm", 1, 0, false, false, false),
        (32, "Orc", 1, 0, false, false, false),
        (33, "Skeleton", 1, 0, false, false, false),
        (34, "Python", 3, 5, false, false, false),
        (35, "Ettin", 5, 7, false, false, false),
        (36, "Headless", 1, 0, false, false, false),
        (37, "Wisp", 1, 0, false, false, false),
        (38, "Daemon", 9, 3, true, false, false),
        (39, "Dragon", 9, 3, false, false, false),
        (40, "Sand Trap", 1, 0, false, false, false),
        (41, "Troll", 5, 2, false, false, false),
        (42, "Reserved gap", 1, 0, false, false, false),
        (43, "Reserved gap", 1, 0, false, false, false),
        (44, "Mongbat", 1, 0, false, false, false),
        (45, "Corpser", 1, 0, false, false, false),
        (46, "Rot Worm", 1, 0, false, false, false),
        (47, "Shadow Lord", 9, 3, true, false, false),
    ];

    for &(
        class,
        name,
        range_effect_selector,
        payload,
        scene_resistance,
        cast_like_branch,
        pre_gate_bypass,
    ) in EXPECTED_ROWS
    {
        let stats = combat_ranged_effect_stats(class).unwrap();
        assert_eq!(stats.name, name, "class {class} name");
        assert_eq!(
            stats.range_effect_selector, range_effect_selector,
            "class {class} range/effect selector"
        );
        assert_eq!(stats.payload, payload, "class {class} payload");
        assert_eq!(
            stats.scene_resistance, scene_resistance,
            "class {class} scene resistance"
        );
        assert_eq!(
            stats.cast_like_branch, cast_like_branch,
            "class {class} cast-like branch"
        );
        assert_eq!(
            stats.pre_gate_bypass, pre_gate_bypass,
            "class {class} pre-gate bypass"
        );
    }

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

#[test]
fn monster_ability_hook_is_bounded_to_three_bits_in_fixed_order() {
    assert_eq!(first_monster_ability(0), None);
    assert_eq!(
        first_monster_ability(MONSTER_ABILITY_POSSESS),
        Some(MonsterAbility::Possess)
    );
    assert_eq!(
        first_monster_ability(MONSTER_ABILITY_BLINK),
        Some(MonsterAbility::Blink)
    );
    assert_eq!(
        first_monster_ability(MONSTER_ABILITY_SUMMON_DAEMON),
        Some(MonsterAbility::SummonDaemon)
    );
    assert_eq!(
        first_monster_ability(MONSTER_ABILITY_POSSESS | MONSTER_ABILITY_BLINK),
        Some(MonsterAbility::Possess)
    );
    assert_eq!(
        first_monster_ability(MONSTER_ABILITY_BLINK | MONSTER_ABILITY_SUMMON_DAEMON),
        Some(MonsterAbility::Blink)
    );
    assert_eq!(
        first_monster_ability(
            MONSTER_ABILITY_POSSESS | MONSTER_ABILITY_BLINK | MONSTER_ABILITY_SUMMON_DAEMON
        ),
        Some(MonsterAbility::Possess)
    );
    assert_eq!(first_monster_ability(0xffff & !0x0c40), None);
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
    assert_eq!(
        EQUIPMENT_NAMES[EQUIPMENT_ID_AMULET_TURNING],
        "Amulet/Turning"
    );
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
    assert!(
        !resolve_amulet_turning_scatter_for_party_target(
            28,
            PartyMember {
                hp: 0,
                status: b'D',
                ..target
            },
            &equipment,
            1
        )
        .unwrap()
    );
}

#[test]
fn combat_ai_attack_route_uses_range_cap_and_adjacent_melee_boundary() {
    assert_eq!(resolve_combat_ai_attack_route(11, 1), None);

    assert_eq!(
        resolve_combat_ai_attack_route(28, 6),
        Some(CombatAiAttackRoute::OutOfRange)
    );
    assert_eq!(
        resolve_combat_ai_attack_route(28, 1),
        Some(CombatAiAttackRoute::Melee)
    );
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
    assert_eq!(resolve_combat_ai_special_hook(11, true, 0, 0, true), None);
    assert_eq!(
        resolve_combat_ai_special_hook(28, true, 1, 1, false),
        Some(CombatAiSpecialHook::Possess)
    );
    assert_eq!(resolve_combat_ai_special_hook(28, false, 0, 0, true), None);
    assert_eq!(
        resolve_combat_ai_special_hook(23, false, 0, 0, true),
        Some(CombatAiSpecialHook::Blink)
    );
    assert_eq!(resolve_combat_ai_special_hook(23, false, 32, 0, true), None);
    assert_eq!(
        resolve_combat_ai_special_hook(39, false, 0, 31, true),
        Some(CombatAiSpecialHook::SummonDaemon)
    );
    assert_eq!(resolve_combat_ai_special_hook(39, false, 0, 32, true), None);
    assert_eq!(
        resolve_combat_ai_special_hook(39, false, 0, 31, false),
        None
    );
    assert_eq!(resolve_combat_ai_special_hook(32, true, 0, 0, true), None);
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
    assert!(combat_ai_special_one_in_eight_gate(31));
    assert!(!combat_ai_special_one_in_eight_gate(32));
    assert!(!combat_ai_special_one_in_eight_gate(u8::MAX));

    assert_eq!(
        resolve_combat_ai_special_hook_for_traits(traits, true, 0, 0, true),
        Some(CombatAiSpecialHook::Possess)
    );
    assert_eq!(
        resolve_combat_ai_special_hook_for_traits(traits, false, 0, 0, true),
        Some(CombatAiSpecialHook::Blink)
    );
    assert_eq!(
        resolve_combat_ai_special_hook_for_traits(traits, false, 32, 31, true),
        Some(CombatAiSpecialHook::SummonDaemon)
    );
    assert_eq!(
        resolve_combat_ai_special_hook_for_traits(traits, false, 32, 31, false),
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
    let live =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5]);
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
    let live =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5]);
    let mut candidates =
        [possess_candidate(CombatActorDescriptor::empty(), None); COMBAT_ACTOR_SLOTS];
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

    let outcome = resolve_poison_status_attack_for_party_target(22, &mut target, true, 9).unwrap();

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
        resolve_poison_status_attack_for_party_target(22, &mut poisoned_target, true, 9).unwrap(),
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
        resolve_poison_status_attack_for_party_target(11, &mut non_poison_attacker_target, true, 9,),
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
    assert!(resolve_combat_field_placement_acceptance(
        CombatArenaFieldKind::Fire,
        false
    ));
    assert!(resolve_combat_field_placement_acceptance(
        CombatArenaFieldKind::Sleep,
        true
    ));
}

#[test]
fn combat_field_placement_callback_accepts_all_arena_field_kinds() {
    let mut state = world_state(open_world_grid(), 10, 20);

    assert!(state.combat_arena_field_placement_callback_accepts(
        0,
        COMBAT_PARTY_ACTOR_SLOTS,
        POISON_FIELD_SPELL_INDEX
    ));
    assert!(state.combat_arena_field_placement_callback_accepts(
        0,
        COMBAT_PARTY_ACTOR_SLOTS,
        FIRE_FIELD_SPELL_INDEX
    ));
    assert!(state.combat_arena_field_placement_callback_accepts(
        0,
        COMBAT_PARTY_ACTOR_SLOTS,
        ENERGY_FIELD_SPELL_INDEX
    ));
}

#[test]
fn combat_field_cursor_start_prefers_valid_hint_else_caster_cell() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    state.combat_secondary_marker = Some((7, 5));
    assert_eq!(state.combat_field_cursor_start(0), Some((7, 5)));

    state.combat_secondary_marker = Some((99, 99));
    assert_eq!(state.combat_field_cursor_start(0), Some((5, 5)));

    state.combat_actors[0].x = 0;
    state.combat_actors[0].y = 0;
    state.combat_secondary_marker = Some((10, 10));
    assert_eq!(state.combat_field_cursor_start(0), Some((0, 0)));
}

#[test]
fn active_combat_field_followup_cancels_after_spending_without_marker() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.party[0].mana = FIELD_SPELL_COST;
    state.party[0].level = FIELD_SPELL_COST;
    state.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);

    assert_eq!(
        state.start_combat_cast_spell_prompt(0, false),
        MoveOutcome::Observed
    );
    assert!(
        state
            .step_active_cast('F', "GI", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(
        state
            .step_active_cast(' ', "", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(state.active_cast_followup.is_some());
    assert_eq!(state.spell_charges[FIRE_FIELD_SPELL_INDEX], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.turn, 0);

    assert!(
        state
            .step_active_cast_followup('\u{1b}', "", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(state.active_cast_followup.is_none());
    assert_eq!(state.message, "None!");
    assert_eq!(state.turn, 0);
    assert!(
        state
            .active_objects
            .iter()
            .all(|object| object.type_byte != COMBAT_FIELD_KIND_FIRE)
    );
}

#[test]
fn active_combat_field_followup_ignores_out_of_bounds_cursor_move() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.party[0].mana = FIELD_SPELL_COST;
    state.party[0].level = FIELD_SPELL_COST;
    state.spell_charges[POISON_FIELD_SPELL_INDEX] = 1;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 0, 0]);

    assert_eq!(
        state.start_combat_cast_spell_prompt(0, false),
        MoveOutcome::Observed
    );
    assert!(
        state
            .step_active_cast('G', "IN", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(
        state
            .step_active_cast(' ', "", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert_eq!(state.message.lines().next(), Some("Target? (0, 0)"));

    assert!(
        state
            .step_active_cast_followup('4', "", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert_eq!(state.message.lines().next(), Some("Target? (0, 0)"));
    let result = state
        .step_active_cast_followup(' ', "", std::path::Path::new(""))
        .unwrap()
        .expect("confirming the unchanged cursor should cast the field");

    assert_eq!(result.0, MoveOutcome::Cast);
    assert_eq!(state.turn, 1);
    let marker = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == COMBAT_FIELD_KIND_POISON)
        .expect("poison field marker should be placed");
    assert_eq!((marker.x, marker.y), (0, 0));
}

#[test]
fn combat_field_contact_targets_current_actor_and_skips_poison_linked_monster_tiles() {
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
            0x40,
            &mut target,
            19,
            20,
        ),
        Some(CombatArenaFieldContactOutcome::PoisonedPartyMember {
            status_before: b'G',
            status_after: b'P',
        })
    );
    assert_eq!(target.status, b'P');
    target.status = b'G';

    assert_eq!(
        resolve_combat_arena_field_contact_for_party_target(
            CombatArenaFieldKind::Poison,
            0x80,
            &mut target,
            19,
            20,
        ),
        Some(CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass)
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
            0x40,
            &mut target,
            19,
            20,
        ),
        Some(CombatArenaFieldContactOutcome::PoisonedPartyMember {
            status_before: b'G',
            status_after: b'P',
        })
    );
    assert_eq!(target.status, b'P');
    assert_eq!(
        resolve_combat_arena_field_contact_for_party_target(
            CombatArenaFieldKind::Poison,
            0x40,
            &mut target,
            19,
            20,
        ),
        Some(CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage: 19 })
    );

    assert_eq!(
        resolve_combat_arena_field_contact_for_party_target(
            CombatArenaFieldKind::Sleep,
            0x40,
            &mut target,
            19,
            20,
        ),
        Some(CombatArenaFieldContactOutcome::SleptPartyMember {
            status_before: b'P',
            status_after: b'S',
        })
    );
    assert_eq!(target.status, b'S');

    assert_eq!(
        resolve_combat_arena_field_contact_for_party_target(
            CombatArenaFieldKind::Fire,
            0x40,
            &mut target,
            19,
            20,
        ),
        Some(CombatArenaFieldContactOutcome::FireDamage { raw_damage: 9 })
    );
    assert_eq!(
        resolve_combat_arena_field_contact_for_party_target(
            CombatArenaFieldKind::Energy,
            0x40,
            &mut target,
            19,
            20,
        ),
        None
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
            0x40,
            &mut dead_target,
            0,
            0,
        ),
        Some(CombatArenaFieldContactOutcome::SleepSkippedDeadParty)
    );
    assert_eq!(dead_target.status, b'D');

    assert_eq!(
        resolve_combat_arena_field_contact_for_non_party_target(
            CombatArenaFieldKind::Sleep,
            0x40,
            0,
            0,
        ),
        Some(CombatArenaFieldContactOutcome::SleepDisabledNonParty)
    );
    assert_eq!(
        resolve_combat_arena_field_contact_for_non_party_target(
            CombatArenaFieldKind::Poison,
            0x40,
            20,
            0,
        ),
        Some(CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage: 20 })
    );
    assert_eq!(
        resolve_combat_arena_field_contact_for_non_party_target(
            CombatArenaFieldKind::Poison,
            0x80,
            20,
            0,
        ),
        Some(CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass)
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
    assert_eq!(combat_class_for_sprite_byte(0x70), Some(COMBAT_CLASS_GUARD));
    assert_eq!(combat_class_for_sprite_byte(0x73), Some(COMBAT_CLASS_GUARD));
    assert_eq!(
        combat_class_for_sprite_byte(0x74),
        Some(COMBAT_CLASS_WANDERER)
    );
    assert_eq!(
        combat_class_for_sprite_byte(0x77),
        Some(COMBAT_CLASS_WANDERER)
    );
    assert_eq!(
        combat_class_for_sprite_byte(0x78),
        Some(COMBAT_CLASS_BLACKTHORN)
    );
    assert_eq!(
        combat_class_for_sprite_byte(0x7b),
        Some(COMBAT_CLASS_BLACKTHORN)
    );
    assert_eq!(
        combat_class_for_sprite_byte(0x7c),
        Some(COMBAT_CLASS_LORD_BRITISH)
    );
    assert_eq!(
        combat_class_for_sprite_byte(0x7f),
        Some(COMBAT_CLASS_LORD_BRITISH)
    );
    assert_eq!(combat_class_for_sprite_byte(0x80), Some(16));
    assert_eq!(combat_class_for_sprite_byte(0x83), Some(16));
    assert_eq!(combat_class_for_sprite_byte(0xc0), Some(32));
    assert_eq!(
        combat_class_stats_for_sprite_byte(0x70).map(|stats| stats.name),
        Some("Guard")
    );
    assert_eq!(
        combat_class_traits_for_sprite_byte(0x7c).map(|traits| traits.physical_immune),
        Some(true)
    );
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
    assert_eq!(COMBAT_SLEEP_WAKE_ROLL_LOW, 0);
    assert_eq!(COMBAT_SLEEP_WAKE_ROLL_HIGH, 16);
    assert_eq!(COMBAT_SLEEP_WAKE_SUCCESS_ROLL, 16);
}

#[test]
fn combat_actor_monster_placement_uses_class_hp_speed_and_linkage() {
    let stats = combat_class_stats(39).unwrap();
    let descriptor = CombatActorDescriptor::for_monster_placement(
        stats,
        12,
        3,
        4,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
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
    assert!(!live.eligible_for_field_coordinate_lookup(COMBAT_FIELD_REJECTED_ACTIVE_OBJECT_TILE));
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
    descriptors[2] =
        CombatActorDescriptor::from_row([1, 2, COMBAT_ACTOR_FLAG_SELECTABLE_40, 3, 2, 5, 4, 4]);
    descriptors[3] =
        CombatActorDescriptor::from_row([1, 2, COMBAT_ACTOR_FLAG_SELECTABLE_80, 3, 3, 5, 4, 4]);
    descriptors[4] =
        CombatActorDescriptor::from_row([1, 2, COMBAT_ACTOR_FLAG_SELECTABLE_80, 3, 9, 5, 4, 4]);

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

    let hidden = apply_combat_linked_invisibility(&mut actor, &mut active_objects).unwrap();

    assert_eq!(hidden.visibility, CombatLinkedVisibility::Hidden);
    assert!(hidden.changed());
    assert_eq!(hidden.visual_tile_before, Some(0xc2));
    assert_eq!(
        hidden.visual_tile_after,
        Some(COMBAT_HIDDEN_ACTIVE_OBJECT_TILE)
    );
    assert!(actor.is_hidden_or_unrevealed());
    assert_eq!(active_objects[1].tile, COMBAT_HIDDEN_ACTIVE_OBJECT_TILE);

    let visible = clear_combat_linked_invisibility(&mut actor, &mut active_objects).unwrap();

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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    assert_eq!(
        combat_direction_code_for_direction(Direction::West),
        Some(1)
    );
    assert_eq!(
        combat_direction_code_for_direction(Direction::East),
        Some(2)
    );
    assert_eq!(
        combat_direction_code_for_direction(Direction::North),
        Some(3)
    );
    assert_eq!(
        combat_direction_code_for_direction(Direction::South),
        Some(4)
    );
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
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 7, 0, 5, 4]),
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
    let mut actor =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 2, 0, 5, 5]);
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
    let actor_template =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 5, 5]);
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
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 2, 0, 5, 4]),
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
            established_direction_code: Some(4),
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
    actors[0] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
    actors[4] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 0, 0, 5, 5,
    ]);
    actors[5] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 33, 0, 0, 5, 5]);
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
    actors[8] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_STATUS_DISABLED, 35, 0, 0, 5, 5]);
    actors[9] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 12, 0, 0, 5, 5,
    ]);

    assert!(directed_spell_actor_is_eligible(actors[0]));
    assert!(!directed_spell_actor_is_eligible(actors[5]));
    assert!(!directed_spell_actor_is_eligible(actors[7]));
    assert!(!directed_spell_actor_is_eligible(actors[8]));

    let slots = collect_directed_spell_actor_slots(&actors, &[(5, 5), (5, 5), (3, 3)]);

    assert_eq!(slots, vec![0, 4, 9]);
}

#[test]
fn combat_escape_cleanup_uses_party_side_bits_mode_and_announcement() {
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
    actors[6] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 0, 0, 5, 5]);

    assert!(combat_actor_is_active_not_dead(actors[6]));
    assert!(combat_has_active_not_dead_non_party_actor(&actors));
    assert_eq!(
        resolve_combat_escape_cleanup(&actors, false, false),
        CombatEscapeCleanupDecision::RefusedNotYet
    );
    assert_eq!(
        resolve_combat_escape_cleanup(&actors, true, false),
        CombatEscapeCleanupDecision::RefusedNotHere
    );
    assert_eq!(
        resolve_combat_escape_cleanup(&actors, false, true),
        CombatEscapeCleanupDecision::Accepted
    );

    actors[0].mark_dead();
    assert_eq!(
        resolve_combat_escape_cleanup(&actors, false, false),
        CombatEscapeCleanupDecision::Accepted
    );

    actors[7] = CombatActorDescriptor::from_row([10, 1, 0, 32, 0, 0, 5, 6]);
    assert!(!combat_actor_is_active_not_dead(actors[7]));
    assert_eq!(
        resolve_combat_escape_cleanup(&actors, true, false),
        CombatEscapeCleanupDecision::Accepted
    );
}

#[test]
fn combat_victory_requires_no_active_not_dead_non_party_actors() {
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);

    assert!(resolve_combat_victory(&actors));

    actors[6] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 0, 0, 5, 5]);
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
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
    actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 0, 0, 4, 3]);

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
    assert_eq!(COMBAT_ROUND_RESULT_SUCCESS, 0);
    assert_eq!(COMBAT_ROUND_RESULT_DEFEAT, 1);

    let defeat = resolve_combat_round_loop_control(true, false, false);
    assert_eq!(
        defeat,
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
    );
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
    assert_eq!(
        both,
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
    );
    assert_eq!(both.result_code(), Some(COMBAT_ROUND_RESULT_DEFEAT));
}

#[test]
fn combat_round_loop_control_state_wrapper_reads_current_party_and_actor_table() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
    state.combat_actors[8] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
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
        CombatRoundLoopControl::ContinueActorWalk
    );

    state.combat_actors[0].clear();
    assert_eq!(
        state.combat_round_loop_control(true, true),
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let outcome =
        state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 2, true);

    assert!(outcome.committed_movement());
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (6, 5)
    );
    assert!(state.visibility_dirty);
}

#[test]
fn combat_ambush_reveal_records_consume_trigger_and_stamp_targets() {
    let mut records = [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT];
    records[2] = Some(CombatAmbushRevealRecord::new(6, 5, 0x34, 1, 2, 10, 10));
    let mut terrain = [[0u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];

    let application = apply_combat_ambush_reveal_records(&mut records, &mut terrain, 6, 5).unwrap();

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

    let application = apply_combat_ambush_reveal_records(&mut records, &mut terrain, 3, 4).unwrap();

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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.combat_ambush_reveals[0] = Some(CombatAmbushRevealRecord::new(
        6,
        5,
        0x44,
        3,
        4,
        COMBAT_ARENA_SIDE as u8,
        COMBAT_ARENA_SIDE as u8,
    ));

    let outcome =
        state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 2, true);

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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    assert_eq!(
        state.combat_terrain[4][3],
        DEFAULT_COMBAT_ARENA_TERRAIN[4][3]
    );
    assert!(!state.visibility_dirty);

    state.combat_actors[6].clear();
    assert_eq!(
        state.apply_combat_step_or_attack_primitive(0, COMBAT_TARGET_GROUP_PARTY, 1, false),
        CombatStepOrAttackPrimitiveOutcome::BlockedWall
    );
    assert_eq!(state.combat_ambush_reveals[0], Some(reveal));
    assert_eq!(
        state.combat_terrain[4][3],
        DEFAULT_COMBAT_ARENA_TERRAIN[4][3]
    );
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
fn combat_frame_entry_and_exit_preserve_save_backed_interference_sources() {
    let mut state = world_state(open_world_grid(), 10, 20);
    let mut expected = [COMBAT_INTERFERENCE_NO_SOURCE; COMBAT_ACTOR_SLOTS];
    expected[0] = 8;
    expected[9] = 31;
    state.combat_interference_sources = expected;

    let snapshot = state
        .enter_combat_frame(
            vec![ActiveObject::empty(); OOL_SLOTS],
            [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        )
        .expect("combat frame should enter");
    assert_eq!(state.combat_interference_sources, expected);

    state.restore_combat_frame(snapshot);
    assert_eq!(state.combat_interference_sources, expected);
}

#[test]
fn combat_post_dispatch_absorbable_field_contact_sets_armed_result_marker() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.visibility_dirty = false;
    state.active_player = Some(0);
    state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    state.active_objects[0] = ActiveObject {
        type_byte: 0x5c,
        tile: 0x5c,
        x: 6,
        y: 2,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.active_objects[7] = ActiveObject {
        type_byte: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
        tile: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
        x: 6,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 6, 2]);
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
        endgame_tableau_map: None,
        encounter_mode_high_bit: false,
        suppress_controlled_faint_sleep_tick: false,
        exit_announced: false,
        established_exit_direction_code: None,
    });

    let application = state
        .apply_combat_player_command_with_attack_inputs(
            0,
            CombatPlayerCommandInput::Key(' '),
            CombatPlayerWeaponAttackInputs::default(),
        )
        .expect("pass should complete the player dispatch");

    assert!(matches!(
        application.action,
        CombatPlayerCommandAction::Pass(_)
    ));
    assert!(matches!(
        application.absorbable_contact,
        Some(CombatAbsorbableFieldApplication {
            companion_band_index,
            marker_byte: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
            x: 6,
            y: 2,
            armed_endgame_result: true,
            ..
        }) if companion_band_index == terrain_band_active_index(1, 6).unwrap()
    ));
    assert_eq!(application.post_dispatch_contact, None);
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
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 2]);
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
        endgame_tableau_map: None,
        encounter_mode_high_bit: false,
        suppress_controlled_faint_sleep_tick: false,
        exit_announced: false,
        established_exit_direction_code: None,
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 7, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    assert_eq!(
        state.tick_combat_actor_phase_counter(COMBAT_ACTOR_SLOTS, 30),
        None
    );
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
fn combat_pass_command_has_specified_control_flow() {
    assert_eq!(
        resolve_combat_pass_command(),
        CombatPassCommandOutcome {
            moves: false,
            attacks: false,
            ends_turn: true,
        }
    );
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
    assert_eq!(
        (state.combat_actors[8].x, state.combat_actors[8].y),
        (10, 10)
    );

    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.turn, 0);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert_eq!(state.message, "Escape-Not yet!\n");
    assert_eq!(
        (state.combat_actors[8].x, state.combat_actors[8].y),
        (10, 10)
    );

    state.combat_actors[8].mark_dead();
    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert_eq!(state.message, "Escape-Not yet!\n");
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

    assert_eq!(
        resolve_post_combat_active_player_restore(None, &party),
        None
    );
    assert_eq!(
        resolve_post_combat_active_player_restore(Some(0), &party),
        Some(0)
    );
    assert_eq!(
        resolve_post_combat_active_player_restore(Some(1), &party),
        Some(1)
    );
    assert_eq!(
        resolve_post_combat_active_player_restore(Some(2), &party),
        None
    );

    party[0].status = b'D';
    party[1].status = b'S';
    assert_eq!(
        resolve_post_combat_active_player_restore(Some(0), &party),
        None
    );
    assert_eq!(
        resolve_post_combat_active_player_restore(Some(1), &party),
        None
    );
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
    state.combat_actors[10] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 13, 0, 8, 5]);
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

    // `animation.md §6` (spec HEAD `c00bf63`): water `0x01` is not an
    // animated family. `0xD4` is the waterfall family's first id.
    state.combat_terrain[3][4] = 0x01;
    assert!(
        !state.viewport_has_animated_tiles(5),
        "water must not count as animated terrain"
    );

    state.combat_terrain[3][4] = 0xD4;
    assert!(state.viewport_has_animated_tiles(5));
}

/// `animation.md §7`: "after advancing phases and tile selectors, the
/// engine explicitly gives the display layer a chance to make the
/// result visible", and `§10` requires an implementation to "present
/// the frame only after both the per-slot pass and the global tile
/// selector pass have completed".
///
/// This engine composes the viewport once and caches it (`main-loop.md
/// §9`: the producer only re-runs while the visibility-dirty flag is
/// set), and the composition stores the *resolved* family frame. The
/// tile-selector pass therefore has to invalidate that composition, or
/// the presented frame never observes it and a waterfall stays frozen
/// for the whole life of the process.
#[test]
fn static_tile_animation_tick_restages_the_cached_viewport_composition() {
    let mut grid = open_grid();
    // `animation.md §6`: `0xD4..0xD7` waterfall, "advanced every tick.
    // Ungated."
    grid[18 * TOWN_GRID_SIDE + 10] = 0xD4;
    let mut state = test_state(grid, 10, 20);
    let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

    let first = state
        .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
        .unwrap()
        .unwrap();
    assert!(!state.visibility_dirty, "a render consumes the dirty flag");

    state.advance_animation_clock();
    let second = state
        .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
        .unwrap()
        .unwrap();

    assert_ne!(
        first.pixels, second.pixels,
        "the waterfall family advances every tick, so the next presented              frame must show the new selector rather than the cached one"
    );
}

/// `animation.md §6`: the pendulum family `0x80..0x83` is "inside the
/// bit-0 gate, so it changes at half rate". A tick whose pass skipped
/// the only animated family on screen cannot change the picture, so it
/// must leave `main-loop.md §9`'s lazy-refill branch in charge.
#[test]
fn static_tile_animation_tick_only_restages_families_the_pass_advanced() {
    let mut grid = open_grid();
    grid[18 * TOWN_GRID_SIDE + 10] = 0x80;
    let mut state = test_state(grid, 10, 20);
    state.animation = AnimationClock::at_static_tile_phase(0);
    state.visibility_dirty = false;

    // Phase 0 has bit 0 clear: waterfall and fountain advance, the
    // pendulum does not.
    state.advance_animation_clock();
    assert!(
        !state.visibility_dirty,
        "a tick that cannot move the pendulum must not force a rebuild"
    );

    // Phase 1 has bit 0 set, so the pendulum's selector moves.
    state.advance_animation_clock();
    assert!(
        state.visibility_dirty,
        "the half-rate tick that does move the pendulum must restage the frame"
    );
}

/// The gated view of the same question, without a render in the loop.
#[test]
fn viewport_animated_tile_scan_honours_the_pass_gates() {
    let mut grid = open_grid();
    grid[18 * TOWN_GRID_SIDE + 10] = 0x80;
    let state = test_state(grid, 10, 20);

    assert!(state.viewport_has_animated_tiles(VIEWPORT_PLAYER_ROW));
    assert!(state.viewport_has_animated_tiles_advanced_by(
        VIEWPORT_PLAYER_ROW,
        static_tile_animation_pass(1)
    ));
    assert!(!state.viewport_has_animated_tiles_advanced_by(
        VIEWPORT_PLAYER_ROW,
        static_tile_animation_pass(0)
    ));
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
    combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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

    let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);

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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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

    let application = state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);

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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    combat_actors[COMBAT_PARTY_ACTOR_SLOTS] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 0, 0, 5, 5]);
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
        CombatRoundLoopExit::Victory,
        &actors
    ));
    assert!(!combat_exit_requests_body_retrieval_reconcile(
        CombatRoundLoopExit::LeaveCombat,
        &actors
    ));
    assert!(!combat_exit_requests_body_retrieval_reconcile(
        CombatRoundLoopExit::Defeat,
        &actors
    ));

    actors[COMBAT_PARTY_ACTOR_SLOTS] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 0, 0, 5, 5]);
    assert!(!combat_exit_requests_body_retrieval_reconcile(
        CombatRoundLoopExit::Victory,
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    assert_eq!(
        resolve_combat_command_branch('A'),
        CombatCommandBranch::Attack
    );
    assert_eq!(
        resolve_combat_command_branch('a'),
        CombatCommandBranch::Attack
    );
    assert_eq!(
        resolve_combat_command_branch('B'),
        CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Board)
    );
    assert_eq!(
        resolve_combat_command_branch('C'),
        CombatCommandBranch::CastSpell
    );
    assert_eq!(
        resolve_combat_command_branch('D'),
        CombatCommandBranch::DWhatRefusal
    );
    assert_eq!(resolve_combat_command_branch('G'), CombatCommandBranch::Get);
    assert_eq!(
        resolve_combat_command_branch('J'),
        CombatCommandBranch::Jimmy
    );
    assert_eq!(
        resolve_combat_command_branch('K'),
        CombatCommandBranch::Klimb
    );
    assert_eq!(
        resolve_combat_command_branch('M'),
        CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Mix)
    );
    assert_eq!(
        resolve_combat_command_branch('O'),
        CombatCommandBranch::Open
    );
    assert_eq!(
        resolve_combat_command_branch('P'),
        CombatCommandBranch::Push
    );
    assert_eq!(
        resolve_combat_command_branch('Q'),
        CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Quit)
    );
    assert_eq!(
        resolve_combat_command_branch('R'),
        CombatCommandBranch::Ready
    );
    assert_eq!(
        resolve_combat_command_branch('S'),
        CombatCommandBranch::Search
    );
    assert_eq!(
        resolve_combat_command_branch('U'),
        CombatCommandBranch::UseItem
    );
    assert_eq!(
        resolve_combat_command_branch('W'),
        CombatCommandBranch::WWhatRefusal
    );
    assert_eq!(
        resolve_combat_command_branch('X'),
        CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Xit)
    );
    assert_eq!(
        resolve_combat_command_branch('Y'),
        CombatCommandBranch::Yell
    );
    assert_eq!(
        resolve_combat_command_branch('Z'),
        CombatCommandBranch::ZStats
    );
    assert_eq!(
        resolve_combat_command_branch(' '),
        CombatCommandBranch::Pass
    );
    assert_eq!(
        resolve_combat_command_branch('\u{1b}'),
        CombatCommandBranch::EscapeCleanup
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
        CombatCommandBranch::UseItem,
    ] {
        assert!(combat_command_branch_requires_live_active_actor(branch));
    }

    for branch in [
        CombatCommandBranch::Attack,
        CombatCommandBranch::CastSpell,
        CombatCommandBranch::DWhatRefusal,
        CombatCommandBranch::Klimb,
        CombatCommandBranch::Push,
        CombatCommandBranch::WWhatRefusal,
        CombatCommandBranch::EscapeCleanup,
        CombatCommandBranch::Yell,
        CombatCommandBranch::ZStats,
        CombatCommandBranch::Pass,
        CombatCommandBranch::ToggleMusic,
        CombatCommandBranch::Invalid,
    ] {
        assert!(!combat_command_branch_requires_live_active_actor(branch));
    }
}

#[test]
fn combat_command_live_actor_gate_rejects_missing_empty_and_dead_actors() {
    let live =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 7, 0, 4, 5]);
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
        resolve_combat_command_live_actor_gate(CombatCommandBranch::EscapeCleanup, Some(dead)),
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
        combat_command_branch_published_label(CombatCommandBranch::EscapeCleanup),
        None
    );
    assert_eq!(
        combat_command_branch_published_label(CombatCommandBranch::UseItem),
        None
    );

    assert_eq!(
        combat_scene_abort_verb_prefix(CombatSceneAbortVerb::HoleUp),
        "Hole up"
    );
    assert_eq!(
        combat_scene_abort_tail(CombatSceneAbortVerb::Board),
        CombatSceneAbortTail::What
    );
    assert_eq!(
        combat_scene_abort_tail(CombatSceneAbortVerb::Quit),
        CombatSceneAbortTail::NotHere
    );
    assert_eq!(
        combat_scene_abort_tail(CombatSceneAbortVerb::Talk),
        CombatSceneAbortTail::FunnyNoResponse
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
        CombatCommandBranch::UseItem,
        CombatCommandBranch::Yell,
    ] {
        assert!(combat_command_branch_is_named_multistage(branch));
    }

    for branch in [
        CombatCommandBranch::DWhatRefusal,
        CombatCommandBranch::Push,
        CombatCommandBranch::WWhatRefusal,
        CombatCommandBranch::EscapeCleanup,
        CombatCommandBranch::ZStats,
        CombatCommandBranch::Pass,
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
    let sleeping = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_STATUS_DISABLED,
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
    assert!(!combat_cast_interference_target_is_live_visible(sleeping));
    assert!(!combat_cast_interference_target_is_live_visible(dead));
    assert!(!combat_cast_interference_target_is_live_visible(
        unselectable
    ));

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
        resolve_combat_cast_interference(caster, Some(sleeping), true, false),
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
fn directed_sleep_uses_shared_cardinal_wind_cone_cells() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let sleep_cells = state
        .directed_combat_spell_target_cells(0, Direction::West, CombatDirectedSpellEffect::Sleep)
        .unwrap();
    let poison_cells = state
        .directed_combat_spell_target_cells(
            0,
            Direction::West,
            CombatDirectedSpellEffect::PoisonWind,
        )
        .unwrap();

    assert_eq!(sleep_cells, poison_cells);
    assert_eq!(sleep_cells.len(), 35);
    assert_eq!(&sleep_cells[0..3], &[(4, 4), (4, 5), (4, 6)]);
    assert!(sleep_cells.contains(&(0, 0)));
    assert!(sleep_cells.contains(&(0, 10)));
    assert!(!sleep_cells.contains(&(5, 5)));
}

#[test]
fn directed_wind_cone_geometry_matches_cardinal_bands_and_cap() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let cases = [
        (Direction::West, [(4, 4), (4, 5), (4, 6)], (0, 0), (0, 10)),
        (Direction::East, [(6, 4), (6, 5), (6, 6)], (10, 0), (10, 10)),
        (Direction::North, [(4, 4), (5, 4), (6, 4)], (0, 0), (10, 0)),
        (
            Direction::South,
            [(4, 6), (5, 6), (6, 6)],
            (0, 10),
            (10, 10),
        ),
    ];

    for (direction, first_band, edge_a, edge_b) in cases {
        let cells = state
            .directed_combat_spell_target_cells(0, direction, CombatDirectedSpellEffect::FlameWind)
            .unwrap();
        assert_eq!(cells.len(), 35, "{direction:?}");
        assert_eq!(&cells[0..3], &first_band, "{direction:?}");
        assert!(cells.contains(&edge_a), "{direction:?}");
        assert!(cells.contains(&edge_b), "{direction:?}");
        assert!(!cells.contains(&(5, 5)), "{direction:?}");
    }

    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 0, 5]);
    let capped = state
        .directed_combat_spell_target_cells(
            0,
            Direction::East,
            CombatDirectedSpellEffect::DeathWind,
        )
        .unwrap();
    assert_eq!(capped.len(), DIRECTED_WIND_MAX_CELLS);
    assert_eq!(&capped[0..3], &[(1, 4), (1, 5), (1, 6)]);
    assert_eq!(capped.last().copied(), Some((8, 5)));
    let unique: std::collections::HashSet<_> = capped.iter().copied().collect();
    assert_eq!(unique.len(), capped.len());
}

#[test]
fn combat_spell_handler_family_maps_published_combat_spell_ids() {
    let family =
        |code: &str| resolve_combat_spell_handler_family(spell_index_from_code(code).unwrap());

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
        Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::Sleep
        ))
    );
    assert_eq!(
        family("HIN"),
        Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::PoisonWind
        ))
    );
    assert_eq!(
        family("CGIV"),
        Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::DeathWind
        ))
    );
    assert_eq!(
        family("FHI"),
        Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::FlameWind
        ))
    );
    assert_eq!(
        family("IPVY"),
        Some(CombatSpellHandlerFamily::TableWideTremor)
    );

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
    actors[0] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
    actors[4] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 0, 0, 5, 5,
    ]);
    actors[5] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 33, 0, 0, 6, 6]);
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
    actors[9] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 12, 0, 0, 8, 8,
    ]);
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
fn cause_fear_scan_uses_monster_side_not_faction_and_skips_protected_targets() {
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
    actors[4] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 0, 0, 5, 5,
    ]);
    actors[5] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 33, 0, 0, 6, 6]);
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
    actors[9] = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 12, 0, 0, 8, 8,
    ]);
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

    assert_eq!(slots, vec![7]);
}

#[test]
fn cause_fear_and_repel_force_exactly_one_hp() {
    for max_hp in [0, 1, 2, 5, 10, 20, 99] {
        assert_eq!(cause_fear_forced_current_hp(max_hp), 1);
    }
}

#[test]
fn shared_combat_resistance_formula_pins_skew_score_and_equality() {
    assert_eq!(combat_skewed_roll_1_to_30(0), 1);
    assert_eq!(combat_skewed_roll_1_to_30(3), 1);
    assert_eq!(combat_skewed_roll_1_to_30(4), 2);
    assert_eq!(combat_skewed_roll_1_to_30(59), 29);
    assert_eq!(combat_skewed_roll_1_to_30(60), 30);

    assert_eq!(combat_resistance_score(255, 0), -112);
    assert_eq!(combat_resistance_score(0, 255), 142);
    assert_eq!(combat_resistance_score(10, 20), 20);
    assert!(combat_resistance_blocks_from_raw_roll(10, 20, 38));
    assert!(!combat_resistance_blocks_from_raw_roll(10, 20, 40));
    assert!(!combat_resistance_blocks_from_raw_roll(255, 0, 0));
    assert!(combat_resistance_blocks_from_raw_roll(0, 255, 60));

    assert!(!combat_target_weight_gate_accepts_from_raw_roll(20, 38));
    assert!(combat_target_weight_gate_accepts_from_raw_roll(20, 40));
}

#[test]
fn shared_combat_resistance_uses_owner_intelligence_and_class_endurance_once() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party_intelligence = vec![0, 37];
    state.combat_actors[0] = CombatActorDescriptor::from_row([20, 9, 0, 1, 0, 0, 4, 4]);
    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::for_monster_placement(
        combat_class_stats(COMBAT_CLASS_DAEMON).unwrap(),
        COMBAT_PARTY_ACTOR_SLOTS as u8,
        5,
        5,
        0,
        0,
    );

    assert_eq!(state.combat_actor_resistance_rating(0), Some(37));
    assert_eq!(
        state.combat_actor_resistance_rating(COMBAT_PARTY_ACTOR_SLOTS),
        Some(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().endurance)
    );

    state.prng_state = 0x456;
    let mut expected_prng = state.prng_state;
    let raw = u5_prng_range_u16(&mut expected_prng, 0, 60) as u8;
    let expected = combat_resistance_blocks_from_raw_roll(
        combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().endurance,
        37,
        raw,
    );
    assert_eq!(
        state.combat_ai_possess_resistance_blocks(COMBAT_PARTY_ACTOR_SLOTS, 0),
        expected
    );
    assert_eq!(state.prng_state, expected_prng);
}

#[test]
fn target_combat_weight_applies_only_the_published_forced_one_cases() {
    let ordinary = CombatActorDescriptor::from_row([20, 9, 0, 32, 0, 0, 4, 4]);
    let disabled =
        CombatActorDescriptor::from_row([20, 9, COMBAT_ACTOR_FLAG_STATUS_DISABLED, 32, 0, 0, 4, 4]);
    let mimic = CombatActorDescriptor::from_row([20, 9, 0, COMBAT_CLASS_MIMIC, 0, 0, 4, 4]);

    assert_eq!(combat_actor_weight(0, ordinary, true), 9);
    assert_eq!(
        combat_actor_weight(COMBAT_PARTY_ACTOR_SLOTS, ordinary, false),
        9
    );
    assert_eq!(
        combat_actor_weight(COMBAT_PARTY_ACTOR_SLOTS, ordinary, true),
        1
    );
    assert_eq!(combat_actor_weight(0, disabled, false), 1);
    assert_eq!(
        combat_actor_weight(COMBAT_PARTY_ACTOR_SLOTS, mimic, false),
        1
    );
}

#[test]
fn cause_fear_critical_hp_setup_mutates_accepted_live_actor_slots_only() {
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[2] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        COMBAT_CLASS_DAEMON,
        0,
        0,
        3,
        3,
    ]);
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
fn conjure_spell_class_selector_uses_sixteen_weighted_outcomes() {
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
    assert_eq!(resolve_conjure_spell_class(15), COMBAT_CLASS_PYTHON);
    assert_eq!(resolve_conjure_spell_class(16), COMBAT_CLASS_GIANT_RAT);
    assert_eq!(resolve_conjure_spell_class(31), COMBAT_CLASS_PYTHON);
}

#[test]
fn summon_spell_descriptors_use_published_class_rows_and_coordinates() {
    let conjured =
        resolve_conjure_spell_descriptor(10, 4, 5, 6, COMBAT_ACTOR_FLAG_SELECTABLE_80, 3).unwrap();
    let spider_stats = combat_class_stats(COMBAT_CLASS_GIANT_SPIDER).unwrap();
    assert_eq!(conjured.owner_target_class, COMBAT_CLASS_GIANT_SPIDER);
    assert_eq!(conjured.hp_or_wound, spider_stats.max_hp);
    assert_eq!(conjured.base_step, spider_stats.speed_seed);
    assert_eq!(conjured.active_object_slot, 4);
    assert_eq!((conjured.x, conjured.y), (5, 6));
    assert_eq!(conjured.flags, COMBAT_ACTOR_FLAG_SELECTABLE_80);
    assert_eq!(conjured.phase_counter, 3);

    let swarm =
        resolve_swarm_spell_descriptor(5, 7, 8, COMBAT_ACTOR_FLAG_SELECTABLE_40, 2).unwrap();
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
        resolve_summoned_combat_actor_descriptor(99, 4, 5, 6, COMBAT_ACTOR_FLAG_SELECTABLE_80, 3,),
        None
    );
}

#[test]
fn summoned_active_object_records_use_class_sprite_base_and_coordinates() {
    assert_eq!(
        combat_class_sprite_base(COMBAT_CLASS_GUARD),
        Some(COMBAT_CLASS_GUARD_SPRITE_BASE)
    );
    assert_eq!(combat_class_sprite_base(COMBAT_CLASS_WANDERER), Some(0x74));
    assert_eq!(
        combat_class_sprite_base(COMBAT_CLASS_BLACKTHORN),
        Some(0x78)
    );
    assert_eq!(
        combat_class_sprite_base(COMBAT_CLASS_LORD_BRITISH),
        Some(0x7c)
    );
    assert_eq!(combat_class_sprite_base(COMBAT_CLASS_GIANT_RAT), Some(0x90));
    assert_eq!(combat_class_sprite_base(COMBAT_CLASS_DAEMON), Some(0xd8));
    assert_eq!(combat_class_sprite_base(44), Some(0xf0));
    assert_eq!(
        combat_class_sprite_base(COMBAT_CLASS_SHADOW_LORD),
        Some(0xfc)
    );
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
        vec![
            (6, 4),
            (4, 5),
            (6, 5),
            (4, 6),
            (5, 6),
            (6, 6),
            (4, 4),
            (5, 4)
        ]
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
        vec![
            (5, 4),
            (6, 4),
            (6, 5),
            (6, 6),
            (5, 6),
            (4, 6),
            (4, 5),
            (4, 4)
        ]
    );
    assert_eq!(
        combat_ring_candidate_coordinates(0, 0),
        vec![(1, 0), (1, 1), (0, 1)]
    );
    assert_eq!(
        combat_ring_candidate_coordinates_around(-1, 5),
        vec![(0, 4), (0, 5), (0, 6)]
    );
    assert_eq!(
        combat_direction_target_coordinate(5, 5, Direction::East),
        Some((6, 5))
    );
}

#[test]
fn combat_summon_application_allocates_actor_and_object_on_legal_neighbor() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.combat_terrain[4][6] = 0x04;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    let live_target = CombatActorDescriptor::from_row([
        20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 0, 0, 5, 5,
    ]);
    assert!(creature_prompt_target_is_eligible(live_target, 2, 1, false));
    assert!(!creature_prompt_target_is_eligible(
        CombatActorDescriptor::empty(),
        2,
        1,
        false,
    ));
    assert!(!creature_prompt_target_is_eligible(
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 32, 0, 0, 5, 5,]),
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
        CombatActorDescriptor::from_row(
            [20, 1, COMBAT_ACTOR_FLAG_STATUS_DISABLED, 32, 0, 0, 5, 5,]
        ),
        2,
        1,
        false,
    ));
    assert!(!creature_prompt_target_is_eligible(
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_CONTROLLED, 32, 0, 0, 5, 5,]),
        2,
        1,
        false,
    ));
    assert!(!creature_prompt_target_is_eligible(
        live_target,
        1,
        1,
        false
    ));
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
    let target =
        CombatActorDescriptor::from_row([33, 22, COMBAT_ACTOR_FLAG_SELECTABLE_80, 39, 7, 3, 5, 6]);

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
    let mut actors = [CombatActorDescriptor::from_row([
        1,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        32,
        1,
        0,
        1,
        1,
    ]); COMBAT_ACTOR_SLOTS];
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
        CombatActorDescriptor::from_row([
            1,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            32,
            1,
            0,
            1,
            1,
        ]);
    assert_eq!(
        resolve_clone_spell_allocation(&actors, &active_objects),
        None
    );

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
    assert_eq!(
        resolve_clone_spell_allocation(&actors, &active_objects),
        None
    );
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
    assert_eq!(
        cloned_actor.owner_target_class,
        target_actor.owner_target_class
    );
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let target_actor =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 39, 7, 2, 5, 5]);
    state.combat_actors =
        [CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 1, 0, 1, 1]);
            COMBAT_ACTOR_SLOTS];
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 4, 3]);

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
    state.party_intelligence[0] = u8::MAX;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
    // `catalogs/spell-list.md` id 34 + `combat.md §6.1a`: Charm prints its
    // own `<name> charmed!` line and suppresses the dispatcher's
    // success/failure epilogue, so the generic `Charm!` never appears.
    assert_eq!(state.message, "Daemon charmed!");
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
fn combat_cast_charm_rejects_an_unmarked_same_group_target_before_resources() {
    // `magic.md §8`: Charm re-targets an actor that already carries the
    // controlled/charmed marker, because "a second successful Charm on the
    // same actor clears it". The refusal this pins is the *other* case —
    // an unmarked target already in the caster's own group.
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
    // A party-side slot with the marker clear groups with the caster, so
    // the creature prompt refuses it before the resource gate.
    let target_slot = 1usize;
    state.combat_actors[target_slot] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        target_slot as u8,
        target_slot as u8,
        0,
        5,
        5,
    ]);
    assert_eq!(
        state.combat_target_group_for_slot(target_slot),
        state.combat_target_group_for_slot(0)
    );

    assert_eq!(
        state
            .cast_spell_from_suffix("1AEX2", std::path::Path::new(""))
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
    state.combat_terrain = [[0x05; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    let (conjure_x, conjure_y) = loop {
        let x = u5_prng_range_u16(&mut expected_prng, 0, 15) as u8;
        let y = u5_prng_range_u16(&mut expected_prng, 0, 15) as u8;
        if usize::from(x) < COMBAT_ARENA_SIDE
            && usize::from(y) < COMBAT_ARENA_SIDE
            && (x, y) != (5, 5)
        {
            break (x, y);
        }
    };

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
        summoned_active_object_record(
            COMBAT_CLASS_GIANT_RAT,
            usize::from(conjure_x),
            usize::from(conjure_y),
            0,
        )
        .unwrap()
    );
}

#[test]
fn combat_cast_swarm_routes_resources_and_places_four_swarms_on_one_probe() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_terrain = [[0x05; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    let (swarm_x, swarm_y) = loop {
        let x = u5_prng_range_u16(&mut expected_prng, 0, 15) as u8;
        let y = u5_prng_range_u16(&mut expected_prng, 0, 15) as u8;
        if usize::from(x) < COMBAT_ARENA_SIDE
            && usize::from(y) < COMBAT_ARENA_SIDE
            && (x, y) != (5, 5)
        {
            break (x, y);
        }
    };
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
    for offset in 0..4 {
        let slot = COMBAT_PARTY_ACTOR_SLOTS + offset;
        assert_eq!(
            state.combat_actors[slot],
            resolve_swarm_spell_descriptor(
                slot as u8,
                swarm_x,
                swarm_y,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
        assert_eq!(
            state.active_objects[slot],
            summoned_active_object_record(
                COMBAT_CLASS_INSECT_SWARM,
                usize::from(swarm_x),
                usize::from(swarm_y),
                0,
            )
            .unwrap()
        );
    }
}

#[test]
fn combat_swarm_rejects_an_off_arena_probe_then_places_four_at_one_cell() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    let mut legal_cells = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    legal_cells[4][4] = true;
    let seed = (0..=u16::MAX)
        .find(|candidate| {
            let mut prng = *candidate;
            let first_x = u5_prng_range_u16(&mut prng, 0, 15);
            let first_y = u5_prng_range_u16(&mut prng, 0, 15);
            let second_x = u5_prng_range_u16(&mut prng, 0, 15);
            let second_y = u5_prng_range_u16(&mut prng, 0, 15);
            (first_x >= COMBAT_ARENA_SIDE as u16 || first_y >= COMBAT_ARENA_SIDE as u16)
                && (second_x, second_y) == (4, 4)
        })
        .unwrap();
    state.prng_state = seed;
    let mut expected_prng = seed;
    for _ in 0..4 {
        let _ = u5_prng_range_u16(&mut expected_prng, 0, 15);
    }

    let applied = state.apply_combat_swarm_with_random_attempts(0, &legal_cells);

    assert_eq!(applied.len(), 4);
    assert!(
        applied
            .iter()
            .all(|application| (application.x, application.y) == (4, 4))
    );
    assert_eq!(state.prng_state, expected_prng);
    for offset in 0..4 {
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + offset],
            resolve_swarm_spell_descriptor(
                (COMBAT_PARTY_ACTOR_SLOTS + offset) as u8,
                4,
                4,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                0,
            )
            .unwrap()
        );
    }
}

#[test]
fn combat_cast_summon_daemon_routes_resources_and_places_daemon() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.party_intelligence = vec![31];
    state.prng_state = 0;
    let spell_index = spell_index_from_code("CKX").unwrap();
    state.spell_charges[spell_index] = 1;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    let (expected_x, expected_y) = loop {
        let x = u5_prng_range_u16(&mut expected_prng, 0, 15) as u8;
        let y = u5_prng_range_u16(&mut expected_prng, 0, 15) as u8;
        if usize::from(x) < COMBAT_ARENA_SIDE
            && usize::from(y) < COMBAT_ARENA_SIDE
            && (x, y) != (5, 5)
        {
            break (x, y);
        }
    };
    let _self_check_raw_roll = u5_prng_range_u16(&mut expected_prng, 0, 60);

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
            expected_x,
            expected_y,
            COMBAT_SUMMONED_ACTOR_FLAGS,
            0,
        )
        .unwrap()
    );
    assert_eq!(
        state.active_objects[COMBAT_PARTY_ACTOR_SLOTS],
        summoned_active_object_record(
            COMBAT_CLASS_DAEMON,
            usize::from(expected_x),
            usize::from(expected_y),
            0
        )
        .unwrap()
    );
}

#[test]
fn summon_self_check_threshold_uses_party_intelligence_or_monster_endurance() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party_intelligence = vec![31];
    state.combat_actors[0] = CombatActorDescriptor::from_row([30, 1, 0, 0, 0, 0, 5, 5]);
    let monster_slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[monster_slot] = CombatActorDescriptor::for_monster_placement(
        combat_class_stats(COMBAT_CLASS_DAEMON).unwrap(),
        monster_slot as u8,
        6,
        5,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        0,
    );

    assert_eq!(state.combat_summon_daemon_self_check_threshold(0), 31);
    assert_eq!(
        state.combat_summon_daemon_self_check_threshold(monster_slot),
        combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().endurance
    );
}

#[test]
fn combat_cast_summon_daemon_does_not_require_direction_and_can_oops() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.party_intelligence = vec![1];
    let spell_index = spell_index_from_code("CKX").unwrap();
    state.spell_charges[spell_index] = 1;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    assert_eq!(
        state
            .cast_spell_from_suffix("1CKX", std::path::Path::new(""))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "Oops...");
    assert_eq!(state.spell_charges[spell_index], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.turn, 1);
    assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty());
    assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_controlled());
}

#[test]
fn combat_ai_summon_daemon_special_places_daemon_without_spell_resources() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.combat_terrain[4][6] = 0x04;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
                    COMBAT_ACTOR_FLAG_SELECTABLE_80,
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
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
fn passive_classes_keep_physical_occupancy_but_leave_targeting_and_side_counts() {
    let classes = [7u8, 8, 9, 10];
    let expected_flags = [
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_ACTOR_FLAG_MARKED_DEAD,
        COMBAT_ACTOR_FLAG_MARKED_DEAD,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
    ];
    let expected_groups = [
        COMBAT_TARGET_GROUP_MONSTER,
        COMBAT_TARGET_GROUP_PARTY,
        COMBAT_TARGET_GROUP_PARTY,
        COMBAT_TARGET_GROUP_MONSTER,
    ];
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    let mut candidates =
        [combat_target_view(CombatActorDescriptor::empty(), 0); COMBAT_ACTOR_SLOTS];

    for (offset, class) in classes.into_iter().enumerate() {
        let slot = COMBAT_PARTY_ACTOR_SLOTS + offset;
        let descriptor = CombatActorDescriptor::from_row([
            20,
            1,
            combat_monster_placement_flags(class),
            class,
            slot as u8,
            0,
            6 + offset as u8,
            5,
        ]);
        actors[slot] = descriptor;
        candidates[slot] = combat_target_view(
            descriptor,
            resolve_combat_target_group_for_actor(descriptor, slot, None),
        );
        assert_eq!(descriptor.flags, expected_flags[offset]);
        assert_eq!(candidates[slot].group, expected_groups[offset]);
        assert!(combat_actor_occupies_arena_cell(
            descriptor,
            descriptor.x,
            descriptor.y
        ));
    }

    assert_eq!(
        actors
            .iter()
            .copied()
            .filter(|actor| combat_actor_is_present_not_dead(*actor))
            .count(),
        2,
        "only classes 7 and 10 participate in the side count"
    );
    assert!(matches!(
        resolve_combat_step_or_attack_inner_pass(
            &candidates,
            0,
            COMBAT_TARGET_GROUP_PARTY,
            resolve_combat_step_destination(5, 5, 2),
            true,
        ),
        CombatStepOrAttackOutcome::Attack { target_slot: 6 }
    ));
    assert_eq!(
        resolve_combat_step_or_attack_inner_pass(
            &candidates,
            0,
            COMBAT_TARGET_GROUP_PARTY,
            resolve_combat_step_destination(6, 5, 2),
            true,
        ),
        CombatStepOrAttackOutcome::BlockedActor { target_slot: 7 }
    );
}

#[test]
fn mass_charm_uses_linked_party_dexterity_for_reachable_controlled_party_actor() {
    let mut state = combat_ai_turn_state(8, 5);
    state.active_effect_tag = Some(MASS_CHARM_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = MASS_CHARM_ACTIVE_EFFECT_DURATION;
    state.party[0].climb_stat = 10;
    state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;

    let at_threshold = state
        .clone()
        .apply_combat_ai_turn_with_inputs(
            0,
            false,
            0,
            false,
            32,
            32,
            &[],
            None,
            10,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();
    let above_threshold = state
        .apply_combat_ai_turn_with_inputs(
            0,
            false,
            0,
            false,
            32,
            32,
            &[],
            None,
            11,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();

    assert_eq!(at_threshold.acting_group, COMBAT_TARGET_GROUP_MONSTER);
    assert_eq!(above_threshold.acting_group, COMBAT_TARGET_GROUP_PARTY);
}

/// `combat.md §9`: the hard-wired hostile roster template is keyed to
/// the roster record - "the last record of the shipped sixteen-record
/// roster". "The player's own character is exempt by construction,
/// and no name the player can enter changes any actor's team."
#[test]
fn combat_target_group_helper_keys_the_traitor_override_to_the_roster_record() {
    assert_eq!(
        resolve_combat_target_group(0, 0, false),
        COMBAT_TARGET_GROUP_PARTY
    );
    assert_eq!(
        resolve_combat_target_group(6, 20, false),
        COMBAT_TARGET_GROUP_MONSTER
    );
    assert_eq!(
        resolve_combat_target_group(0, TRAITOR_ROSTER_RECORD, false),
        COMBAT_TARGET_GROUP_MONSTER
    );
    assert_eq!(
        resolve_combat_target_group(1, 1, false),
        COMBAT_TARGET_GROUP_PARTY
    );
    assert_eq!(
        resolve_combat_target_group(COMBAT_ACTOR_SLOTS, 0, false),
        COMBAT_TARGET_GROUP_NEUTRAL
    );
    assert!(roster_record_is_shipped_traitor(TRAITOR_ROSTER_RECORD));
    assert!(!roster_record_is_shipped_traitor(0));
}

/// Regression for the withdrawn name-keyed rule: an Avatar whose
/// fifth name byte is a lowercase `j` ("Marijke") sits in roster
/// record zero and must stay in the party group.
#[test]
fn combat_target_group_never_reads_a_player_entered_name() {
    let avatar =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    assert_eq!(
        resolve_combat_target_group_for_actor(avatar, 0, Some(b"Marijke")),
        COMBAT_TARGET_GROUP_PARTY
    );
    assert_eq!(
        resolve_combat_target_group_for_actor(avatar, 0, Some(b"Saduj")),
        COMBAT_TARGET_GROUP_PARTY
    );

    let traitor = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        TRAITOR_ROSTER_RECORD,
        1,
        0,
        5,
        5,
    ]);

    assert_eq!(
        resolve_combat_target_group_for_actor(traitor, 1, None),
        COMBAT_TARGET_GROUP_MONSTER
    );
}

#[test]
fn combat_target_group_helper_applies_team_toggle_without_overriding_traitor_rule() {
    assert_eq!(
        resolve_combat_target_group(0, 0, true),
        COMBAT_TARGET_GROUP_MONSTER
    );
    assert_eq!(
        resolve_combat_target_group(6, 20, true),
        COMBAT_TARGET_GROUP_PARTY
    );
    assert_eq!(
        resolve_combat_target_group(0, TRAITOR_ROSTER_RECORD, true),
        COMBAT_TARGET_GROUP_MONSTER
    );
}

#[test]
fn combat_target_candidate_view_helper_packages_group_and_visibility_inputs() {
    let descriptor = CombatActorDescriptor::from_row([10, 1, 0, 0, 4, 0, 3, 2]);

    let view =
        combat_target_candidate_view_from_descriptor(descriptor, 0, Some(b"Avatar"), true, false);

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
fn protection_active_effect_does_not_modify_live_party_spell_defense() {
    let mut state = world_state(open_world_grid(), 10, 20);

    assert_eq!(
        state.combat_spell_target_defense_value(0),
        CHARACTER_DEFENSE_FACTORY_SEED
    );

    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = PROTECTION_ACTIVE_EFFECT_DURATION;
    assert_eq!(
        state.combat_spell_target_defense_value(0),
        CHARACTER_DEFENSE_FACTORY_SEED
    );
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

fn combat_target_view(descriptor: CombatActorDescriptor, group: u8) -> CombatTargetCandidateView {
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
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 0, 0, 5, 5,
        ]),
        2,
    );
    actors[3] = combat_target_view(
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5,
        ]),
        1,
    );
    actors[7] = combat_target_view(
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 33, 0, 0, 6, 5,
        ]),
        1,
    );
    actors[20] = combat_target_view(
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 34, 0, 0, 8, 8,
        ]),
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
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 32, 0, 0, 5, 5,
        ]),
        2,
    );
    actors[1] = combat_target_view(
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_MARKED_DEAD, 0, 0, 0, 4, 5]),
        1,
    );
    actors[2] = combat_target_view(
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5,
        ]),
        2,
    );
    actors[3] = combat_target_view(
        CombatActorDescriptor::from_row([
            10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5,
        ]),
        2,
    );
    actors[4] = CombatTargetCandidateView {
        suppressed: true,
        ..combat_target_view(
            CombatActorDescriptor::from_row([
                10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5,
            ]),
            1,
        )
    };
    actors[6] = CombatTargetCandidateView {
        invisible_or_unrevealed: true,
        ..combat_target_view(
            CombatActorDescriptor::from_row([
                10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5,
            ]),
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.combat_actors[8] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
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
    assert_eq!(
        application.attack_route,
        Some(CombatAiAttackRoute::OutOfRange)
    );
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
    assert_eq!(
        (state.active_objects[8].x, state.active_objects[8].y),
        (7, 5)
    );
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
/// `combat.md §9`: the hard-wired hostile roster template is keyed to
/// the roster record. "The player's own character is exempt by
/// construction, and no name the player can enter changes any
/// actor's team."
fn combat_ai_turn_applies_traitor_roster_record_group_to_target_scan() {
    // A player-entered name whose fifth byte is a lowercase 'j' must
    // leave the Avatar a valid party-side target.
    let mut named = combat_ai_turn_state(8, 5);
    named.party_names[0] = *b"ABCDj\0\0\0\0";
    named.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_SELECTABLE_40;

    let named_application = named
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
        named_application.target,
        CombatAiTargetResolution::ChosenActor {
            slot: 0,
            x: COMBAT_ARENA_CENTER_COORDINATE,
            y: COMBAT_ARENA_CENTER_COORDINATE,
        }
    );

    // The traitor's roster record does flip the group: with the only
    // party-side candidate hostile the scan takes the centre
    // fallback and marks the monster-side slots fleeing.
    let mut traitor = combat_ai_turn_state(8, 5);
    traitor.combat_actors[0].owner_target_class = TRAITOR_ROSTER_RECORD;
    traitor.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_SELECTABLE_40;

    let traitor_application = traitor
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
        traitor_application.target,
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
        endgame_tableau_map: None,
        encounter_mode_high_bit: false,
        suppress_controlled_faint_sleep_tick: false,
        exit_announced: false,
        established_exit_direction_code: None,
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
fn combat_ai_summon_daemon_uses_only_the_single_injected_probe() {
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
            32,
            31,
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

    let Some(CombatAiSpecialApplication::SummonDaemon { summon, .. }) = application.special else {
        panic!("dragon should summon a daemon");
    };
    assert_eq!((summon.x, summon.y), (9, 5));
    assert_eq!(application.target, CombatAiTargetResolution::NoUsableTarget);
    assert_eq!(application.movement, None);
}

#[test]
fn combat_ai_failed_summon_probe_continues_ordinary_action() {
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

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            32,
            31,
            &[(15, 15)],
            None,
            0,
            false,
            None,
            true,
            &[4, 1, 3, 2],
            None,
        )
        .unwrap();

    assert_eq!(application.special, None);
    assert_eq!(
        application.target,
        CombatAiTargetResolution::ChosenActor {
            slot: 0,
            x: 5,
            y: 5,
        }
    );
    assert!(application.command_key.is_some() || application.movement.is_some());
}

#[test]
fn combat_ai_successful_blink_consumes_the_actor_turn() {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].owner_target_class = 23;

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            31,
            31,
            &[(6, 5)],
            None,
            0,
            false,
            None,
            true,
            &[4, 1, 3, 2],
            None,
        )
        .unwrap();

    assert!(matches!(
        application.special,
        Some(CombatAiSpecialApplication::Blink { .. })
    ));
    assert_eq!(application.target, CombatAiTargetResolution::NoUsableTarget);
    assert_eq!(application.step_vector, None);
    assert_eq!(application.attack_route, None);
    assert_eq!(application.movement, None);
}

#[test]
fn production_combat_ai_summon_draws_gate_then_fresh_x_then_y_and_stops() {
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

    let (seed, expected_prng, expected_coordinate) =
        (0..=0x0fff_u16)
            .find_map(|seed| {
                let mut expected = state.clone();
                expected.prng_state = seed;
                let gate = expected.random_range_u8(0, u8::MAX);
                let x = expected.random_range_u8(0, 15);
                let y = expected.random_range_u8(0, 15);
                (gate <= 31 && x <= 10 && y <= 10 && (x, y) != (5, 5) && (x, y) != (8, 5))
                    .then_some((seed, expected.prng_state, (x, y)))
            })
            .expect("the 12-bit PRNG must expose an accepted legal summon probe");
    state.prng_state = seed;

    let application = state.apply_combat_ai_turn(8).unwrap();

    let Some(CombatAiSpecialApplication::SummonDaemon { summon, .. }) = application.special else {
        panic!("accepted production summon probe should place one daemon");
    };
    assert_eq!((summon.x, summon.y), expected_coordinate);
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(application.target, CombatAiTargetResolution::NoUsableTarget);
    assert_eq!(application.movement, None);
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
    assert_eq!(
        (state.active_objects[8].x, state.active_objects[8].y),
        (6, 5)
    );
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
fn negate_magic_skips_enemy_special_hook_but_preserves_ordinary_melee() {
    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].owner_target_class = 28;
    state.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 20;
    state.party[0].status = b'G';
    state.party[0].hp = 20;
    state.party[0].max_hp = 20;

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
            Some(CombatMonsterAttackInputs {
                party_defender_rating: 0,
                forced_hit: Some(true),
                ..CombatMonsterAttackInputs::default()
            }),
        )
        .unwrap();

    assert_eq!(application.special, None);
    assert!(!application.possess_hook_handled);
    assert_eq!(application.attack_route, Some(CombatAiAttackRoute::Melee));
    assert!(application.monster_attack.is_some());
    assert!(state.party[0].hp < 20);
    assert_eq!(
        state.combat_actors[0].flags,
        COMBAT_ACTOR_FLAG_SELECTABLE_80
    );
}

#[test]
fn crown_bypasses_enemy_teleport_arm_and_continues_ordinary_step() {
    let mut state = combat_ai_turn_state(10, 10);
    state.combat_actors[8].owner_target_class = 13;
    state.combat_actors[0].x = 0;
    state.combat_actors[0].y = 0;
    state.active_objects[0].x = 0;
    state.active_objects[0].y = 0;
    state.active_effect_tag = Some(CROWN_LB_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = u8::MAX;

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            Some((9, 9)),
            true,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();

    assert_eq!(
        application.attack_route,
        Some(CombatAiAttackRoute::OutOfRange)
    );
    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::Step {
            direction_code: 1,
            x: 9,
            y: 10,
        })
    );
    assert_eq!(
        (state.combat_actors[8].x, state.combat_actors[8].y),
        (9, 10)
    );
    assert_eq!(
        (state.active_objects[8].x, state.active_objects[8].y),
        (9, 10)
    );
}

#[test]
fn negate_magic_silently_consumes_only_scene_resistant_ranged_effects() {
    let attack_inputs = CombatMonsterAttackInputs {
        party_defender_rating: 0,
        forced_hit: Some(true),
        ..CombatMonsterAttackInputs::default()
    };
    let mut resistant = combat_ai_turn_state(8, 5);
    resistant.combat_actors[8].owner_target_class = 28;
    resistant.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
    resistant.active_effect_counter = 20;
    resistant.party[0].status = b'G';
    resistant.party[0].hp = 20;
    resistant.party[0].max_hp = 20;

    let blocked = resistant
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            Some(attack_inputs),
        )
        .unwrap();

    assert!(matches!(
        blocked.attack_route,
        Some(CombatAiAttackRoute::RangedEffect {
            scene_resistance: true,
            ..
        })
    ));
    assert_eq!(blocked.command_key, Some(COMBAT_AI_ATTACK_COMMAND_KEY));
    assert_eq!(blocked.monster_attack, None);
    assert_eq!(blocked.movement, None);
    assert_eq!(resistant.party[0].hp, 20);

    let mut ordinary = combat_ai_turn_state(8, 5);
    ordinary.combat_actors[8].owner_target_class = COMBAT_CLASS_DRAGON;
    ordinary.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
    ordinary.active_effect_counter = 20;
    ordinary.party[0].status = b'G';
    ordinary.party[0].hp = 20;
    ordinary.party[0].max_hp = 20;

    let landed = ordinary
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            Some(attack_inputs),
        )
        .unwrap();

    assert!(matches!(
        landed.attack_route,
        Some(CombatAiAttackRoute::RangedEffect {
            scene_resistance: false,
            ..
        })
    ));
    assert!(landed.monster_attack.is_some());
    assert!(ordinary.party[0].hp < 20);
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
            target_flags_after: COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
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
    assert_eq!(
        state.combat_actors[0].flags,
        COMBAT_ACTOR_FLAG_SELECTABLE_80
    );
    assert!(!state.combat_actors[8].is_empty());
    assert_eq!(state.message, "Possession resisted.");
}

#[test]
fn combat_ai_turn_applies_possess_hook_before_target_synthesis() {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].owner_target_class = 28;
    state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_SELECTABLE_40;

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
    assert_eq!(application.target, CombatAiTargetResolution::NoUsableTarget);
    assert_eq!(application.step_vector, None);
    assert_eq!(application.attack_route, None);
    assert_eq!(application.movement, None);
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

    assert_eq!(
        application.stop_reason,
        CombatRoundWalkStopReason::EndOfRound
    );
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

    assert_eq!(
        application.stop_reason,
        CombatRoundWalkStopReason::EndOfRound
    );
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
        // `combat.md §5`: the owner/target/class byte is "the
        // character's roster slot index", so the second seated
        // member's descriptor names roster slot one.
        1,
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
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (6, 5)
    );
    assert_eq!(state.combat_terrain[5][6], PUSHABLE_GENERIC_FLOOR_STAMP);
    assert_eq!(state.combat_terrain[5][7], 0x91);
    assert!(state.visibility_dirty);
    assert_eq!(state.message, "East\nPushed!");
}

#[test]
fn combat_push_dynamic_object_is_an_emphatic_refusal_and_never_moves() {
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

    assert_eq!(outcome, MoveOutcome::Blocked);
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (5, 5)
    );
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (6, 5)
    );
    assert_eq!(state.active_objects[1].tile, 0x90);
    assert_eq!(state.active_objects[1].type_byte, 0x90);
    assert_eq!(state.combat_terrain[5][6], 0x04);
    assert_eq!(state.combat_terrain[5][7], 0x04);
    assert!(!state.visibility_dirty);
    assert_eq!(state.message, "East\nWon't budge!");
}

#[test]
fn combat_push_reveal_marker_preempts_source_test_without_a_result_line() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.combat_ambush_reveals[0] = Some(CombatAmbushRevealRecord::new(6, 5, 0x44, 2, 2, 3, 3));

    handle_play_key_input(&mut state, 'P', "6", game_dir).unwrap();

    assert_eq!(state.combat_ambush_reveals[0], None);
    assert_eq!(state.combat_terrain[2][2], 0x44);
    assert_eq!(state.combat_terrain[3][3], 0x44);
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));
    assert_eq!(state.message_entries()[0].text, "Push-East");
    assert!(state.message_entries().iter().all(|entry| {
        !matches!(
            entry.text.as_str(),
            PUSHED_SUCCESS | PULLED_SUCCESS | PUSH_WONT_BUDGE_EMPHATIC | PUSH_WONT_BUDGE_SHORT
        )
    }));
}

#[test]
fn combat_input_dispatch_push_prompt_keeps_actor_until_direction() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.visibility_dirty = false;
    state.combat_terrain[5][6] = 0x90;
    state.combat_terrain[5][7] = PUSHABLE_GENERIC_FLOOR_STAMP;
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

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
    assert_eq!(state.active_effect_counter, 3);

    assert_eq!(
        handle_play_key_input(&mut state, '6', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_direction_prompt.is_none());
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
    assert_eq!(state.combat_terrain[5][6], PUSHABLE_GENERIC_FLOOR_STAMP);
    assert_eq!(state.combat_terrain[5][7], 0x91);
    assert!(state.message.starts_with("Pushed!"));
    assert_eq!(state.message_entries()[0].text, "Push-East");
    assert_eq!(state.message_entries()[1].text, "Pushed!");
    assert!(state.visibility_dirty);
    assert_eq!(state.active_effect_counter, 2);
}

#[test]
fn combat_input_dispatch_push_prompt_cancel_commits_actor_action() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'P', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_direction_prompt.is_some());
    assert_eq!(state.pending_combat_actor_slot, None);

    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_direction_prompt.is_some());
    assert_eq!(state.active_effect_counter, 3);
    assert_eq!(state.message_entries()[0].text, "Push-");

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_direction_prompt.is_none());
    assert_eq!(state.active_effect_counter, 2);
    assert_eq!(
        (state.combat_actors[8].x, state.combat_actors[8].y),
        (9, 10)
    );
    assert_eq!(
        state.message,
        format!("Push-{DIRECTION_PROMPT_LABEL_PASS}")
    );
}

#[test]
fn combat_input_dispatch_inline_push_suffix_pushes_immediately() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.combat_terrain[5][6] = 0x90;
    state.combat_terrain[5][7] = PUSHABLE_GENERIC_FLOOR_STAMP;
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'P', "6", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_direction_prompt.is_none());
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
    assert_eq!(state.combat_terrain[5][6], PUSHABLE_GENERIC_FLOOR_STAMP);
    assert_eq!(state.combat_terrain[5][7], 0x91);
    assert!(state.message.starts_with("East\nPushed!"));
    assert_eq!(state.message_entries()[0].text, "Push-East");
    assert_eq!(state.message_entries()[1].text, "Pushed!");
    assert_eq!(state.active_effect_counter, 2);
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
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (5, 4)
    );
    assert_eq!(state.message, "Klimbed North to (5, 4).");
    assert!(state.visibility_dirty);
    assert!(state.combat_active);
}

#[test]
fn combat_klimb_vertical_suffix_exits_from_ladder_tile() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.combat_terrain[5][5] = 0x50;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'K', "<", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Klimbed up from combat.");
    assert_eq!(state.active_effect_counter, 2);
    assert!(!state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, None);
}

#[test]
fn combat_klimb_prompt_cancel_commits_but_blocked_direction_reprompts() {
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

    let mut cancelled = combat_player_command_state(10, 10);
    cancelled.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    cancelled.active_effect_counter = 3;
    cancelled.combat_interference_sources[0] = 8;
    assert_eq!(
        handle_play_key_input(&mut cancelled, 'K', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        handle_play_key_input(&mut cancelled, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(cancelled.active_effect_counter, 2);
    assert_eq!(
        cancelled.combat_interference_sources[0],
        COMBAT_INTERFERENCE_NO_SOURCE
    );
    assert_eq!(
        (cancelled.combat_actors[8].x, cancelled.combat_actors[8].y),
        (9, 10)
    );
    assert_eq!(cancelled.message, DIRECTION_PROMPT_LABEL_PASS);

    let mut blocked = combat_player_command_state(10, 10);
    blocked.combat_terrain[4][5] = 0x0c;
    blocked.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    blocked.active_effect_counter = 3;
    blocked.combat_interference_sources[0] = 8;
    assert_eq!(
        handle_play_key_input(&mut blocked, 'K', "8", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(blocked.message, "Klimb-What?");
    assert_eq!(blocked.pending_combat_actor_slot, Some(0));
    assert_eq!(
        (blocked.combat_actors[8].x, blocked.combat_actors[8].y),
        (10, 10)
    );
    assert_eq!(blocked.active_effect_counter, 3);
    assert_eq!(blocked.combat_interference_sources[0], 8);
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
    assert!(
        state
            .message
            .starts_with("Got combat object tile 80 at (6, 5).")
    );
    assert!(!state.message.contains("Giant Rat"));
}

#[test]
fn combat_sjog_open_and_jimmy_mutate_combat_terrain() {
    let game_dir = std::path::Path::new(".");
    let mut open_state = combat_player_command_state(10, 10);
    open_state.combat_terrain[5][6] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
    open_state.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut open_state, 'O', "6", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(open_state.combat_terrain[5][6], TOWN_DOOR_CLEARED_TILE);
    assert!(open_state.visibility_dirty);
    assert!(open_state.message.starts_with("Opened!"));

    let mut jimmy_state = combat_player_command_state(10, 10);
    jimmy_state.keys = 1;
    jimmy_state.party[0].climb_stat = 30;
    jimmy_state.combat_terrain[5][6] = TOWN_DOOR_PLAIN_LOCKED_TILE;
    jimmy_state.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut jimmy_state, 'J', "6", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        jimmy_state.combat_terrain[5][6],
        TOWN_DOOR_PLAIN_UNLOCKED_TILE
    );
    assert_eq!(jimmy_state.keys, 1);
    assert!(jimmy_state.visibility_dirty);
    assert!(jimmy_state.message.starts_with("Unlocked!"));
}

#[test]
fn combat_jimmy_restraints_use_flat_dexterity_roll_and_clear_on_success() {
    let mut success = combat_player_command_state(10, 10);
    success.keys = 1;
    success.party[0].climb_stat = 30;
    success.combat_terrain[5][6] = JIMMY_STOCKS_TILE;
    success.visibility_dirty = false;

    assert_eq!(
        success.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(success.combat_terrain[5][6], TOWN_DOOR_CLEARED_TILE);
    assert_eq!(success.keys, 1);
    assert!(success.visibility_dirty);
    assert_eq!(success.message, "Unlocked");

    let mut failure = combat_player_command_state(10, 10);
    failure.keys = 1;
    failure.party[0].climb_stat = 0;
    failure.combat_terrain[5][6] = JIMMY_MANACLES_TILE;
    failure.visibility_dirty = false;

    assert_eq!(
        failure.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(failure.combat_terrain[5][6], JIMMY_MANACLES_TILE);
    assert_eq!(failure.keys, 0);
    assert!(!failure.visibility_dirty);
    assert_eq!(failure.message, "Key broke!");
}

#[test]
fn combat_jimmy_magic_lock_breaks_key_without_dexterity_roll() {
    let mut state = combat_player_command_state(10, 10);
    state.keys = 1;
    state.party[0].climb_stat = 30;
    state.combat_terrain[5][6] = TOWN_DOOR_MAGIC_PLAIN_TILE;
    state.visibility_dirty = false;
    let prng_before = state.prng_state;

    assert_eq!(
        state.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(state.combat_terrain[5][6], TOWN_DOOR_MAGIC_PLAIN_TILE);
    assert_eq!(state.keys, 0);
    assert_eq!(state.prng_state, prng_before);
    assert!(!state.visibility_dirty);
    assert_eq!(state.message, "Key broke!");
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
    assert!(
        state
            .message
            .starts_with("Found combat object tile 81 at (6, 5).")
    );
}

#[test]
fn combat_sjog_prompt_cancel_commits_actor_action() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

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

    assert_eq!(state.active_effect_counter, 2);
    assert_eq!(
        (state.combat_actors[8].x, state.combat_actors[8].y),
        (9, 10)
    );
    assert_eq!(state.message, DIRECTION_PROMPT_LABEL_PASS);
}

#[test]
fn combat_player_command_ignores_quickness_entirely() {
    // `combat.md` section 8: the player's command handler reads only the
    // Negate Magic tag. Quickness must never consume a player dispatch -
    // it makes hostiles act about half as often, it does not turn the
    // player's own turn into a coin flip. The single gate lives at the
    // head of the automatic actor driver.
    let mut state = combat_player_command_state(8, 5);
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    let prng_before = state.prng_state;
    let hp_before = state.party[0].hp;

    let quit = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('Q'))
        .unwrap();

    assert_eq!(
        quit.action,
        CombatPlayerCommandAction::Branch {
            branch: CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Quit),
            live_actor_gate: CombatCommandLiveActorGate::NotRequired,
        }
    );
    assert!(quit.reprompt);
    assert_eq!(quit.ring_pass, None);
    assert_eq!(quit.active_effect_age, None);
    assert_eq!(state.active_effect_counter, 3);
    assert_eq!(state.prng_state, prng_before);
    assert_eq!(state.party[0].hp, hp_before);
    assert_eq!(
        quit.control_after,
        CombatRoundLoopControl::ContinueActorWalk
    );
}

#[test]
fn combat_player_command_routes_direction_and_attack_prompt_through_step_primitive() {
    let mut move_state = combat_player_command_state(8, 5);
    move_state.visibility_dirty = false;

    let moved = move_state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(2))
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
    assert_eq!(
        (move_state.combat_actors[0].x, move_state.combat_actors[0].y),
        (6, 5)
    );
    assert_eq!(
        (
            move_state.active_objects[0].x,
            move_state.active_objects[0].y
        ),
        (6, 5)
    );
    assert!(move_state.visibility_dirty);

    let mut attack_state = combat_player_command_state(6, 5);
    attack_state.combat_terrain[5][5] = 0x05;
    attack_state.visibility_dirty = false;
    attack_state.party_equipment = default_party_equipment(1);
    attack_state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    attack_state.party[0].hp = attack_state.party[0].hp.saturating_sub(1);
    attack_state.prng_state = 0x0030;
    let prng_before_prompt = attack_state.prng_state;
    let prompted = attack_state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('A'))
        .unwrap();
    assert_eq!(
        prompted.action,
        CombatPlayerCommandAction::PromptForAttackDirection
    );
    assert_eq!(prompted.ring_pass, None);
    assert_eq!(attack_state.prng_state, prng_before_prompt);

    let attacked = attack_state
        .apply_combat_player_command_with_attack_inputs(
            0,
            CombatPlayerCommandInput::AttackDirection(2),
            CombatPlayerWeaponAttackInputs::default(),
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
        attacked.ring_pass,
        Some(CombatMagicRingPassOutcome {
            invisibility_applied: false,
            regeneration_applied: 1,
            vanished_ring: None,
        })
    );
    assert_eq!(
        (
            attack_state.combat_actors[0].x,
            attack_state.combat_actors[0].y
        ),
        (5, 5)
    );
    assert!(!attack_state.visibility_dirty);
}

#[test]
fn combat_player_command_out_of_arena_direction_releases_only_the_actor() {
    let mut state = combat_player_command_state(8, 5);
    state.combat_actors[0].x = 10;
    state.combat_actors[0].owner_target_class = 0x2a;
    state.active_objects[0].x = 10;

    let application = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(2))
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
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
    );
    assert_eq!(
        application.out_of_arena_leave,
        Some(CombatOutOfArenaLeaveApplication {
            outcome: CombatOutOfArenaLeaveOutcome::Accepted {
                direction_code: 2,
                presentation: CombatOutOfArenaLeavePresentation::EscapeWithFoes,
                established_direction_code: Some(2),
            },
            cleared_descriptor: true,
            cleared_active_object: true,
            world_ticks: 1,
        })
    );
    assert!(state.combat_actors[0].is_empty());
    assert_eq!(state.combat_actors[0].owner_target_class, 0x2a);
    assert!(state.active_objects[0].is_empty());
    assert!(!state.combat_actors[8].is_empty());
    assert!(!state.active_objects[8].is_empty());
    assert!(state.visibility_dirty);
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
            damage_application: Some(CombatWeaponDamageApplication::Monster { target_slot: 8, .. }),
        })
    ));
    assert_eq!(state.combat_actors[8].hp_or_wound, hp_before - 1);
    assert_eq!(state.party_experience[0], 1);
}

#[test]
fn combat_player_command_attack_announces_victory_and_continues_cleanup() {
    let mut state = combat_player_command_state(6, 5);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
    state.party_strengths = vec![30];
    state.combat_actors[8].hp_or_wound = 1;

    let application = state
        .apply_combat_player_command_with_attack_inputs(
            0,
            CombatPlayerCommandInput::AttackDirection(2),
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
        CombatRoundLoopControl::ContinueActorWalk
    );
    assert!(application.victory_announced);
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
    assert_eq!(regeneration_roll, 0);

    let application = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(' '))
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
fn combat_player_command_regeneration_tail_checks_every_living_wearer() {
    let mut state = combat_player_command_state(8, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: 1,
        status: b'G',
        climb_stat: 0,
        mana: 0,
        hp: 17,
        max_hp: 20,
        level: 1,
    });
    state.party[0].hp = state.party[0].hp.saturating_sub(3);
    state.party_equipment = default_party_equipment(2);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    state.party_equipment[1][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    state.combat_actors[1] = state.combat_actors[0];
    state.combat_actors[1].owner_target_class = 1;
    state.combat_actors[1].x = 4;
    let hp_before = [state.party[0].hp, state.party[1].hp];
    state.prng_state = 0x0070;
    let mut expected_prng = state.prng_state;
    assert_eq!(u5_prng_range_u16(&mut expected_prng, 0, 7), 0);
    assert_eq!(u5_prng_range_u16(&mut expected_prng, 0, 7), 0);

    let application = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(' '))
        .unwrap();

    assert_eq!(
        application.ring_pass,
        Some(CombatMagicRingPassOutcome {
            invisibility_applied: false,
            regeneration_applied: 2,
            vanished_ring: None,
        })
    );
    assert_eq!(state.party[0].hp, hp_before[0] + 1);
    assert_eq!(state.party[1].hp, hp_before[1] + 1);
    assert_eq!(state.prng_state, expected_prng);
}

#[test]
fn combat_player_command_handles_digits_pass_branches_and_escape_cleanup() {
    let mut state = combat_player_command_state(8, 5);
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    let selected = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('1'))
        .unwrap();
    assert_eq!(
        selected.action,
        CombatPlayerCommandAction::ActivePlayerSelection(
            CombatActivePlayerSelectionOutcome::SelectPartySlot(0)
        )
    );
    assert_eq!(state.active_player, Some(0));
    assert_eq!(selected.active_effect_age, None);
    assert_eq!(state.active_effect_counter, 3);

    let pass = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(' '))
        .unwrap();
    assert_eq!(
        pass.action,
        CombatPlayerCommandAction::Pass(CombatPassCommandOutcome {
            moves: false,
            attacks: false,
            ends_turn: true,
        })
    );
    assert_eq!(
        pass.active_effect_age,
        Some(ActiveEffectAgeOutcome {
            tag: Some(QUICKNESS_ACTIVE_EFFECT_TAG),
            counter: 2,
            expired: false,
        })
    );
    assert_eq!(state.active_effect_counter, 2);

    let get = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('G'))
        .unwrap();
    assert_eq!(
        get.action,
        CombatPlayerCommandAction::Branch {
            branch: CombatCommandBranch::Get,
            live_actor_gate: CombatCommandLiveActorGate::Accepted,
        }
    );

    let refused_xit = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('X'))
        .unwrap();
    assert_eq!(
        refused_xit.action,
        CombatPlayerCommandAction::Branch {
            branch: CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Xit),
            live_actor_gate: CombatCommandLiveActorGate::NotRequired,
        }
    );
    assert!(refused_xit.reprompt);
    assert_eq!(
        refused_xit.control_after,
        CombatRoundLoopControl::ContinueActorWalk
    );

    let blocked_escape = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('\u{1b}'))
        .unwrap();
    assert_eq!(
        blocked_escape.action,
        CombatPlayerCommandAction::EscapeCleanup {
            application: CombatEscapeCleanupApplication::refused(
                CombatEscapeCleanupDecision::RefusedNotYet
            )
        }
    );
    assert!(blocked_escape.reprompt);
    assert_eq!(
        blocked_escape.control_after,
        CombatRoundLoopControl::ContinueActorWalk
    );

    let invalid_direction = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(5))
        .unwrap();
    assert_eq!(
        invalid_direction.action,
        CombatPlayerCommandAction::InvalidDirection { direction_code: 5 }
    );
    assert!(invalid_direction.reprompt);

    state.combat_actors[8].flags = COMBAT_ACTOR_FLAG_SELECTABLE_40;
    state.combat_frame_snapshot = Some(CombatFrameSnapshot {
        area: state.area,
        player: state.player,
        active_objects: state.active_objects.clone(),
        active_player: state.active_player,
        combat_terrain: state.combat_terrain,
        dungeon_room_clear_on_success: None,
        enter_endgame_after_successful_combat: false,
        endgame_messages: None,
        endgame_tableau_map: None,
        encounter_mode_high_bit: false,
        suppress_controlled_faint_sleep_tick: false,
        exit_announced: true,
        established_exit_direction_code: None,
    });
    let allowed_escape = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('\u{1b}'))
        .unwrap();
    let CombatPlayerCommandAction::EscapeCleanup { application } = allowed_escape.action else {
        panic!("Escape should run cleanup");
    };
    assert_eq!(application.decision, CombatEscapeCleanupDecision::Accepted);
    assert_eq!(application.cleared_descriptor_slots, 2);
    assert_eq!(application.cleared_active_object_slots, 2);
    assert_eq!(application.world_ticks, 4);
    assert!(application.rising_glissando);
    assert!(application.stats_panel_dirty);
    assert!(!allowed_escape.reprompt);
    assert_eq!(
        allowed_escape.control_after,
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
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
    assert_eq!(
        (move_state.combat_actors[0].x, move_state.combat_actors[0].y),
        (6, 5)
    );
    assert_eq!(move_state.message, "East\n");
    assert_eq!(move_state.pending_combat_actor_slot, Some(0));

    let mut blocked_state = combat_player_command_state(8, 5);
    blocked_state.active_player = Some(0);
    blocked_state.combat_terrain[4][5] = 0x0c;
    assert_eq!(
        handle_play_key_input(&mut blocked_state, 'w', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(blocked_state.message, "North\nBlocked!\n");
    assert_eq!(blocked_state.pending_combat_actor_slot, Some(0));
    assert_eq!(
        (
            blocked_state.combat_actors[8].x,
            blocked_state.combat_actors[8].y
        ),
        (8, 5)
    );

    let mut attack_state = combat_player_command_state(6, 5);
    attack_state.active_player = Some(0);
    assert_eq!(
        handle_play_key_input(&mut attack_state, 'A', "6", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        (
            attack_state.combat_actors[0].x,
            attack_state.combat_actors[0].y
        ),
        (5, 5)
    );
    assert_eq!(attack_state.message, "East\nAvatar is poisoned!\n");
    assert_eq!(attack_state.pending_combat_actor_slot, Some(0));

    let mut quit_state = combat_player_command_state(8, 5);
    assert_eq!(
        handle_play_key_input(&mut quit_state, 'q', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(quit_state.message, "Quit-Not here");
    assert!(quit_state.combat_active);
    assert_eq!(quit_state.pending_combat_actor_slot, Some(0));
    assert_eq!(
        (quit_state.combat_actors[8].x, quit_state.combat_actors[8].y),
        (8, 5)
    );

    for (key, expected) in [
        ('X', "X-it what?"),
        ('B', "Board what?"),
        ('E', "Enter-Not here"),
        ('T', "Talk-Funny, no response!"),
    ] {
        assert_eq!(
            handle_play_key_input(&mut quit_state, key, "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(quit_state.message, expected);
        assert_eq!(quit_state.pending_combat_actor_slot, Some(0));
        assert_eq!(
            (quit_state.combat_actors[8].x, quit_state.combat_actors[8].y),
            (8, 5)
        );
    }
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
        "East\nGiant Rat lightly wounded!\nAvatar is poisoned!\n"
    );
    assert_eq!(state.combat_actors[8].hp_or_wound, 10 - expected_damage);
    assert_eq!(state.party_experience[0], u16::from(expected_damage));
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

#[test]
fn combat_input_dispatch_reports_weapon_kill_and_keeps_victory_cleanup_live() {
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

    assert_eq!(state.message, "East\nGiant Rat killed!\n");
    assert_eq!(state.party_experience[0], 3);
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

#[test]
fn combat_input_dispatch_reports_only_original_style_monster_attack_result() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(6, 5);

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Pass.\nAvatar is poisoned!\n");
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

#[test]
fn paced_combat_presents_one_automatic_action_before_accepting_more_input() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(6, 5);
    state.pace_combat_presentations = true;
    let hp_before = state.party[0].hp;

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.message, "Pass.");
    assert_eq!(state.party[0].hp, hp_before);
    assert_eq!(state.pending_combat_actor_slot, None);

    advance_paced_combat_presentation(&mut state);
    assert_eq!(state.message, "Pass.\nAvatar is poisoned!\n");
    assert_eq!(state.pending_combat_actor_slot, None);

    advance_paced_combat_presentation(&mut state);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

#[test]
fn combat_input_dispatch_use_opens_shared_picker_and_ends_action_on_completion() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;

    assert_eq!(
        handle_play_key_input(&mut state, 'U', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_use.is_some());
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert!(state.message.contains("Pocket Watch"));
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_use.is_none());
    assert_eq!(state.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX], 1);
    assert!(state.message.starts_with("Pocket Watch:"));
    assert!(!state.message.contains("Giant Rat moved"));
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert!(state.combat_active);
}

#[test]
fn combat_input_dispatch_use_without_items_ends_action_after_refusal() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.special_items.fill(0);
    state.scroll_stock.fill(0);
    state.potion_stock.fill(0);
    state.area = Area::Dungeon {
        scene: DungeonScene::new(DUNGEON_DOOM_SCENE_BYTE).unwrap(),
        level: 0,
    };

    assert_eq!(
        handle_play_key_input(&mut state, 'U', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_use.is_none());
    assert!(state.message.starts_with("No usable items."));
    assert!(!state.message.contains("Giant Rat moved"));
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    assert_eq!(state.pending_combat_actor_slot, Some(0));
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

    assert_eq!(state.message, "\nBATTLE IS LOST!");
    assert!(!state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, None);
    assert_eq!(
        state.combat_actors,
        [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS]
    );
}

#[test]
fn combat_input_dispatch_keeps_internal_monster_movement_out_of_transcript() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Pass.");
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

#[test]
fn combat_input_dispatch_escape_cleanup_restores_stored_frame_snapshot() {
    let game_dir = std::path::Path::new(".");
    let mut state = world_state(open_world_grid(), 10, 20);
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[2] = ActiveObject {
        type_byte: 0x2c,
        tile: 0x2f,
        x: 11,
        y: 20,
        phase: 0x07,
        aux1: 0x55,
        aux3: 0xaa,
        ..ActiveObject::empty()
    };
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
    combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
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
    state
        .enter_combat_frame(combat_objects, combat_actors)
        .unwrap();
    state.pending_combat_terrain_trigger_slot = Some(2);
    state.player.x = 99;
    state.combat_actors[8].flags = COMBAT_ACTOR_FLAG_SELECTABLE_40;
    state.combat_frame_snapshot.as_mut().unwrap().exit_announced = true;
    state.pending_combat_actor_slot = Some(0);

    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Escape!");
    assert!(!state.combat_active);
    assert_eq!(state.combat_frame_snapshot, None);
    assert_eq!(state.player, original_player);
    let mut expected_objects = original_objects;
    assert_eq!(
        reconcile_post_combat_terrain_trigger_slot(&mut expected_objects, 2, true),
        PostCombatTriggerReconcile::BodyRetrieval
    );
    assert_eq!(state.active_objects, expected_objects);
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
    combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 10, 5]);
    state
        .enter_combat_frame(combat_objects, combat_actors)
        .unwrap();
    state.pending_combat_actor_slot = Some(0);
    state.player.x = 99;

    assert_eq!(
        handle_play_key_input(&mut state, '6', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "East\nLeave!\n");
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
fn combat_input_dispatch_z_stats_ends_actor_action_when_modal_closes() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.pending_combat_actor_slot = Some(0);
    state.next_combat_actor_slot = 1;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'Z', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert_eq!(
        state.active_z_stats.as_ref().unwrap().selected_party_index,
        0
    );
    assert!(state.message.starts_with("Z-stats: Stats page"));
    assert_eq!(state.active_effect_counter, 3);

    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_z_stats.is_none());
    assert_eq!(state.active_effect_counter, 2);
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));
    assert_eq!(state.pending_combat_actor_slot, Some(0));
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
    assert_eq!(missing.message, "");

    let mut disabled = combat_player_command_state(8, 5);
    disabled.combat_actors[0].set_status_disabled();
    disabled.combat_actors[1] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 1, 4, 5]);
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
    assert_eq!(disabled.message, "");
    assert_eq!(disabled.prng_state, 0x1234);

    let mut cast = combat_player_command_state(8, 5);
    cast.combat_actors[0].set_status_disabled();
    cast.combat_actors[1] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 1, 4, 5]);
    cast.pending_combat_actor_slot = Some(0);
    cast.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    cast.active_effect_counter = 3;
    cast.prng_state = 0x2345;

    assert_eq!(
        handle_play_key_input(&mut cast, 'C', "1IMX6", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(cast.pending_combat_actor_slot, None);
    assert_eq!(cast.message, "");
    assert_eq!(cast.prng_state, 0x2345);
}

#[test]
fn combat_input_dispatch_refuses_controlled_non_party_actor_turn() {
    // magic.md §8: "All three place their creature through the
    // ordinary monster placement path, so the new actor keeps the
    // monster-side class byte and monster AI drives its turns
    // exactly as it drives any other monster. Nothing routes a
    // summoned creature through the player command parser, and the
    // player never gets to move it."
    //
    // combat.md §6.1a: the round walker picks the keystroke path
    // through the slot-to-group helper, and only "the group
    // ordinarily occupied by seated party members" reaches it. The
    // controlled bit does move this monster into the party's group
    // for the same-faction filter, which is why the withdrawn
    // reading looked plausible.
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_TEAM_TOGGLE;
    state.pending_combat_actor_slot = Some(8);
    state.next_combat_actor_slot = 9;
    state.visibility_dirty = false;

    assert_eq!(
        handle_play_key_input(&mut state, '6', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "");
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));
    assert_eq!(
        (state.active_objects[8].x, state.active_objects[8].y),
        (8, 5)
    );
}

#[test]
fn combat_input_dispatch_ready_ends_actor_action_when_picker_closes() {
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
    assert_eq!(
        state.active_ready.as_ref().unwrap().selected_party_index,
        Some(0)
    );
    assert!(state.message.starts_with("Ready: party member 1."));

    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_ready.is_none());
    assert!(!state.message.contains("Giant Rat moved"));
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    assert_eq!(state.pending_combat_actor_slot, Some(0));
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

    assert_eq!(state.message, "Yelled FALLAX. Nothing happens.");
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
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'Y', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Yell what? Use Y<word>.");
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert!(state.active_yell.is_some());
    assert_eq!(state.active_effect_counter, 3);

    assert_eq!(
        handle_play_key_input(&mut state, 'F', "ALLAX", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_yell.is_none());
    assert_eq!(state.message, "Yelled FALLAX. Nothing happens.");
    assert_eq!(state.active_effect_counter, 2);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

#[test]
fn combat_input_dispatch_empty_yell_commits_the_pending_actor_action() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.pending_combat_actor_slot = Some(0);
    state.next_combat_actor_slot = 1;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'Y', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_yell.is_some());
    assert_eq!(state.pending_combat_actor_slot, Some(0));

    assert_eq!(
        handle_play_key_input(&mut state, '\r', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_yell.is_none());
    assert_eq!(state.message, YELL_NOTHING_SAID_MESSAGE);
    assert_eq!(state.active_effect_counter, 2);
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
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 1, 4, 5]);

    let walk = state.ensure_pending_combat_player_turn().unwrap();
    assert_eq!(walk.stop_reason, CombatRoundWalkStopReason::AwaitingPlayer);
    assert_eq!(state.pending_combat_actor_slot, Some(1));
    assert_eq!(state.next_combat_actor_slot, 2);

    assert_eq!(
        handle_play_key_input(&mut state, 's', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.combat_actors[1].x, state.combat_actors[1].y), (4, 6));
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (4, 6)
    );
    assert_ne!((state.combat_actors[0].x, state.combat_actors[0].y), (4, 6));
    assert_eq!(state.message, "South\n");
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
    assert_eq!(state.message, "East\n");
}

#[test]
fn combat_input_dispatch_quickness_never_consumes_the_ready_player() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.turn = 0;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'q', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Quit-Not here");
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));
}

#[test]
fn combat_input_dispatch_action_tail_does_not_run_encounter_ring_vanishal() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    // Keep the next round walk focused on the action-tail contract. Once the
    // ring hides the party member, a live monster correctly has no eligible
    // target and enters its separately tested no-target AI path.
    state.combat_actors[0].base_step = 29;
    state.combat_actors[8].phase_counter = 29;
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_INVISIBILITY as u8;
    state.prng_state = 0x0070;
    let expected_prng = state.prng_state;

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "Pass.");
    assert_eq!(
        state.party_equipment[0][EQUIP_SLOT_RING],
        EQUIPMENT_ID_RING_INVISIBILITY as u8
    );
    assert!(state.combat_actors[0].is_hidden_or_unrevealed());
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
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 4, 5]);
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
    assert_eq!(state.message, "Invisibility!");
}

#[test]
fn combat_input_dispatch_interference_reprompts_before_spell_prompt_without_clearing() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(6, 5);
    state.combat_interference_sources[0] = 8;
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'C', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "\nGiant Rat interferes!");
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert!(state.active_cast.is_none());
    assert!(state.active_cast_followup.is_none());
    assert_eq!(state.combat_interference_sources[0], 8);
    assert_eq!(state.active_effect_counter, 3);
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (6, 5));
}

#[test]
fn combat_input_dispatch_negate_time_suppresses_interference_without_clearing_source() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(6, 5);
    state.combat_interference_sources[0] = 8;
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    assert_eq!(
        handle_play_key_input(&mut state, 'C', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.active_cast.is_some());
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert_eq!(state.combat_interference_sources[0], 8);
    assert_eq!(state.active_effect_counter, 3);
}

#[test]
fn combat_cast_interference_revalidates_hostility_visibility_status_range_and_negate_time() {
    let mut state = combat_player_command_state(6, 5);
    state.combat_interference_sources[0] = 8;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), Some(8));

    state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), None);
    state.combat_actors[8].flags &= !COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;

    state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), None);
    state.combat_actors[8].flags &= !COMBAT_ACTOR_FLAG_STATUS_DISABLED;

    state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), None);
    state.combat_actors[8].flags &= !COMBAT_ACTOR_FLAG_CONTROLLED;

    state.combat_actors[8].x = 8;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), None);
    state.combat_actors[8].x = 6;

    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), None);
    assert_eq!(state.combat_interference_sources[0], 8);

    state.active_effect_counter = 0;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), Some(8));
    state.combat_interference_sources[0] = COMBAT_INTERFERENCE_NO_SOURCE;
    assert_eq!(state.combat_cast_interference_source_for_slot(0), None);
}

#[test]
fn combat_completed_player_action_clears_only_its_victim_source() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.combat_interference_sources[0] = 8;
    state.combat_interference_sources[1] = 9;

    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        state.combat_interference_sources[0],
        COMBAT_INTERFERENCE_NO_SOURCE
    );
    assert_eq!(state.combat_interference_sources[1], 9);
}

#[test]
fn combat_input_dispatch_blank_or_escape_cast_prompt_commits_actor_action() {
    let game_dir = std::path::Path::new(".");

    for cancel_key in ['\u{1b}', '\r'] {
        let mut state = combat_player_command_state(8, 5);
        state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 3;
        state.combat_interference_sources[0] = 8;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_cast.is_some());
        assert_eq!(state.active_effect_counter, 3);
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (8, 5));

        assert_eq!(
            handle_play_key_input(&mut state, cancel_key, "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_cast.is_none());
        assert_eq!(state.active_effect_counter, 2);
        assert_eq!(
            state.combat_interference_sources[0],
            COMBAT_INTERFERENCE_NO_SOURCE
        );
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
        assert_eq!(state.message, "None!");
    }
}

#[test]
fn combat_input_dispatch_cancelled_field_cursor_commits_spent_cast_action() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.party[0].mana = FIELD_SPELL_COST;
    state.party[0].level = FIELD_SPELL_COST;
    state.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    handle_play_key_input(&mut state, 'C', "", game_dir).unwrap();
    handle_play_key_input(&mut state, 'F', "GI", game_dir).unwrap();
    handle_play_key_input(&mut state, ' ', "", game_dir).unwrap();

    assert!(state.active_cast_followup.is_some());
    assert_eq!(state.spell_charges[FIRE_FIELD_SPELL_INDEX], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.active_effect_counter, 3);

    handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap();

    assert!(state.active_cast_followup.is_none());
    assert_eq!(state.active_effect_counter, 2);
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (7, 5));
    assert!(
        state
            .active_objects
            .iter()
            .all(|object| object.type_byte != COMBAT_FIELD_KIND_FIRE)
    );
    assert_eq!(state.message, "None!");
}

#[test]
fn combat_input_dispatch_quickness_does_not_consume_a_player_cast() {
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

    assert_eq!(state.message, "Invisibility!");
    assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 0);
    assert_eq!(state.party[0].mana, 0);
    assert!(state.combat_actors[0].is_hidden_or_unrevealed());
    assert!(state.combat_active);
}

#[test]
fn combat_actor_slot_dispatch_quickness_consumes_an_automatic_actor() {
    // `combat.md` section 8: the engine's single Quickness gate sits at
    // the head of the automatic actor driver, so a self-acting slot
    // forfeits a dispatch when the roll comes up zero. The party slots and
    // any controlled actor never reach it.
    let mut state = combat_ai_turn_state(8, 5);
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;
    let position_before = (state.combat_actors[8].x, state.combat_actors[8].y);

    let mut skipped = 0usize;
    let mut acted = 0usize;
    for _ in 0..16 {
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
        match application {
            CombatActorSlotDispatchApplication::Slot { action, .. } => match action {
                CombatActorDispatchAction::QuicknessSkipped => skipped += 1,
                CombatActorDispatchAction::MonsterAi { .. } => acted += 1,
                _ => {}
            },
            CombatActorSlotDispatchApplication::EndOfRound { .. } => {}
        }
    }
    assert!(
        skipped > 0,
        "a live Quickness effect must consume dispatches"
    );
    let _ = (acted, position_before);

    // With no Quickness tag the same slot always takes its AI turn.
    let mut state = combat_ai_turn_state(8, 5);
    for _ in 0..8 {
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
        if let CombatActorSlotDispatchApplication::Slot { action, .. } = application {
            assert!(
                !matches!(action, CombatActorDispatchAction::QuicknessSkipped),
                "no Quickness tag means no Quickness gate"
            );
        }
    }
}

#[test]
fn combat_negate_time_skips_self_acting_actors_but_still_prompts_the_party() {
    // `magic.md` runtime tag `T`: "In combat the automatic actor
    // driver returns immediately, so every self-acting actor's turn
    // is skipped outright while the tag lasts; the party is still
    // prompted normally." Unlike Quickness there is no roll - the
    // skip holds for every dispatch while the tag is live.
    let mut state = combat_ai_turn_state(8, 5);
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 3;

    let mut skipped = 0usize;
    for _ in 0..16 {
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
        if let CombatActorSlotDispatchApplication::Slot { action, .. } = application {
            match action {
                CombatActorDispatchAction::NegateTimeSkipped => skipped += 1,
                // The per-actor phase counter sits upstream of the
                // driver, so a not-yet-ready slot still reports
                // `Waiting` rather than reaching the gate at all.
                CombatActorDispatchAction::Waiting => {}
                other => panic!("Negate Time must skip a self-acting dispatch, got {other:?}"),
            }
        }
    }
    assert!(
        skipped > 0,
        "a live Negate Time tag must skip self-acting dispatches"
    );

    // The party is unaffected: slot 0 is still handed the ordinary
    // ready dispatch, so the player is prompted as normal. This is
    // the Quickness bug's shape and must not recur here.
    let mut party = combat_ai_turn_state(8, 5);
    party.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    party.active_effect_counter = 3;
    let application = party.apply_combat_actor_slot_dispatch_with_inputs(
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
    if let CombatActorSlotDispatchApplication::Slot { action, .. } = application {
        assert_eq!(
            action,
            CombatActorDispatchAction::PlayerReady,
            "Negate Time must not gate the party's own dispatch"
        );
    } else {
        panic!("party slot should produce a slot dispatch");
    }

    // With no tag the same automatic slot takes its AI turn.
    let mut clear = combat_ai_turn_state(8, 5);
    for _ in 0..8 {
        let application = clear.apply_combat_actor_slot_dispatch_with_inputs(
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
        if let CombatActorSlotDispatchApplication::Slot { action, .. } = application {
            assert!(
                !matches!(action, CombatActorDispatchAction::NegateTimeSkipped),
                "no Negate Time tag means no Negate Time gate"
            );
        }
    }
}

#[test]
fn production_pass_one_gates_precede_all_monster_special_prng_draws() {
    let mut quick = combat_ai_turn_state(8, 5);
    quick.combat_actors[8].owner_target_class = COMBAT_CLASS_DRAGON;
    quick.combat_actors[8].phase_counter = 1;
    quick.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    quick.active_effect_counter = 3;
    let (seed, expected_prng) = (0..=0x0fff_u16)
        .find_map(|seed| {
            let mut expected = quick.clone();
            expected.prng_state = seed;
            (expected.combat_quickness_dispatch_roll(8) == 0).then_some((seed, expected.prng_state))
        })
        .unwrap();
    quick.prng_state = seed;

    let application = quick.apply_combat_actor_slot_dispatch(8, 30, false);

    assert!(matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::QuicknessSkipped,
            ..
        }
    ));
    assert_eq!(quick.prng_state, expected_prng);

    let mut negate = combat_ai_turn_state(8, 5);
    negate.combat_actors[8].owner_target_class = COMBAT_CLASS_DRAGON;
    negate.combat_actors[8].phase_counter = 1;
    negate.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    negate.active_effect_counter = 3;
    negate.prng_state = 0x0357;

    let application = negate.apply_combat_actor_slot_dispatch(8, 30, false);

    assert!(matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::NegateTimeSkipped,
            ..
        }
    ));
    assert_eq!(negate.prng_state, 0x0357);
}

#[test]
fn combat_actor_slot_dispatch_waits_when_phase_counter_is_not_ready() {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].phase_counter = 3;
    state.combat_round_counter = 4;
    state.combat_interference_sources[8] = 7;

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
    assert_eq!(state.combat_interference_sources[8], 7);
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
    state.combat_interference_sources[8] = 7;

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
        state.combat_interference_sources[8],
        COMBAT_INTERFERENCE_NO_SOURCE
    );
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
        action: CombatActorDispatchAction::MonsterAi {
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
    assert_eq!(
        application.stop_reason,
        CombatRoundWalkStopReason::EndOfRound
    );
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
fn combat_round_walk_spends_disabled_actor_turn_on_wake_check() {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].set_status_disabled();
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut prng = *seed;
            u5_prng_range_u16(&mut prng, 0, 16) == 16
        })
        .unwrap();

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
        &[],
    );

    assert_eq!(
        application.stop_reason,
        CombatRoundWalkStopReason::EndOfRound
    );
    assert!(!state.combat_actors[8].is_status_disabled());
    assert!(application.applications.iter().any(|entry| matches!(
        entry,
        CombatActorSlotDispatchApplication::Slot {
            slot: 8,
            action: CombatActorDispatchAction::StatusDisabledWake {
                wake: CombatSleepWakeApplication {
                    slot: 8,
                    roll: 16,
                    woke: true,
                },
            },
            ..
        }
    )));
}

#[test]
fn combat_disabled_actor_failed_wake_roll_does_not_run_ai() {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].set_status_disabled();
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut prng = *seed;
            u5_prng_range_u16(&mut prng, 0, 16) != 16
        })
        .unwrap();

    let application = state.apply_combat_actor_slot_dispatch(8, 30, false);

    assert!(state.combat_actors[8].is_status_disabled());
    assert!(matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            slot: 8,
            action: CombatActorDispatchAction::StatusDisabledWake {
                wake: CombatSleepWakeApplication { woke: false, .. },
            },
            ..
        }
    ));
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

    assert_eq!(
        application.stop_reason,
        CombatRoundWalkStopReason::EndOfRound
    );
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
    actors[2] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 7,
    ]);
    actors[8] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        COMBAT_CLASS_DAEMON,
        0,
        0,
        8,
        8,
    ]);

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
    actors[4] = CombatActorDescriptor::from_row([
        25,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_CLASS_DAEMON,
        0,
        0,
        1,
        1,
    ]);
    actors[5] = CombatActorDescriptor::from_row([
        25,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_CLASS_DAEMON,
        0,
        0,
        2,
        2,
    ]);
    actors[6] = CombatActorDescriptor::from_row([25, 1, 0, COMBAT_CLASS_DAEMON, 0, 0, 2, 2]);
    actors[9] = CombatActorDescriptor::from_row([
        25,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
        COMBAT_CLASS_PYTHON,
        0,
        0,
        3,
        3,
    ]);
    actors[10] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_CLASS_GIANT_RAT,
        0,
        0,
        4,
        4,
    ]);
    actors[12] = CombatActorDescriptor::from_row([10, 1, 0, 0, 0, 0, 5, 5]);
    actors[25] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_CLASS_PYTHON,
        0,
        0,
        6,
        6,
    ]);
    actors[26] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_CLASS_PYTHON,
        0,
        0,
        6,
        6,
    ]);
    actors[31] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
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
            critical_hp_flee_slots: vec![25, 10, 5],
        }
    );
    assert_eq!(combat_ai_center_fallback_target(), (5, 5));
    assert_eq!(actors[4].hp_or_wound, 25);
    assert!(!actors[4].is_fleeing());
    assert_eq!(
        actors[5].hp_or_wound,
        cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().max_hp)
    );
    assert_eq!(actors[5].phase_counter, COMBAT_NO_TARGET_FLEE_STEP_QUEUE);
    assert!(actors[5].is_fleeing());
    assert_eq!(actors[6].hp_or_wound, 25);
    assert!(!actors[6].is_fleeing());
    assert_eq!(actors[9].hp_or_wound, 25);
    assert_eq!(
        actors[10].hp_or_wound,
        cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().max_hp)
    );
    assert_eq!(actors[10].phase_counter, COMBAT_NO_TARGET_FLEE_STEP_QUEUE);
    assert!(actors[10].is_fleeing());
    assert_eq!(actors[12].hp_or_wound, 10);
    assert_eq!(
        actors[25].hp_or_wound,
        cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
    );
    assert_eq!(actors[25].phase_counter, COMBAT_NO_TARGET_FLEE_STEP_QUEUE);
    assert!(actors[25].is_fleeing());
    assert_eq!(actors[26].hp_or_wound, 20);
    assert!(!actors[26].is_fleeing());
    assert_eq!(actors[31].hp_or_wound, 20);
    assert!(!actors[31].is_fleeing());

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
    actors[0] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 2, 2]);
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
    actors[9] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_STATUS_DISABLED,
        COMBAT_CLASS_GIANT_RAT,
        0,
        0,
        9,
        9,
    ]);

    assert!(combat_actor_occupies_arena_cell(actors[0], 2, 2));
    assert!(combat_actor_occupies_arena_cell(actors[9], 9, 9));
    assert!(!combat_actor_occupies_arena_cell(actors[7], 4, 4));

    let legal = build_combat_ai_legal_cell_mask(&terrain, &actors, |tile| tile != 0xff);

    assert!(legal[0][0]);
    assert!(!legal[1][1]);
    assert!(!legal[2][2]);
    assert!(!legal[3][3]);
    assert!(legal[4][4]);
    assert!(!legal[9][9]);
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
    assert_eq!(resolve_combat_wound_morale_for_class(1, 48, 0), None);
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
    state.active_objects[0].tile = 0x10;
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
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
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

fn seed_for_default_death_gates(drop_cap: u8, first_accepts: bool, second_accepts: bool) -> u16 {
    // `combat.md §6.3` "Both rolls use the same helper": both drop gates
    // draw the shared near-uniform `1..30` value, and only the second
    // gate is strict.
    for seed in 0..=u16::MAX {
        let mut prng = seed;
        let first = combat_skewed_roll_1_to_30(u5_prng_range_u16(&mut prng, 0, 60) as u8);
        let second = combat_skewed_roll_1_to_30(u5_prng_range_u16(&mut prng, 0, 60) as u8);
        if combat_default_death_drop_gate_accepts_inclusive(drop_cap, first) == first_accepts
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
    state.combat_terrain[5][4] = 0x04;
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
fn combat_monster_vanish_death_reveals_terrain_then_releases_records() {
    let mut state = world_state(open_world_grid(), 10, 20);
    let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let active_object_slot = 11;
    place_death_side_effect_monster(&mut state, 13, actor_slot, active_object_slot);
    state.combat_terrain[5][4] = 0x42;
    state.active_objects[active_object_slot].aux3 = 0xa5;

    let application = state
        .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
        .unwrap();

    assert!(matches!(
        application,
        CombatWeaponDamageApplication::Monster { damage, .. }
            if damage.death_path == Some(CombatMonsterDeathPath::Vanish)
    ));
    assert!(state.combat_actors[actor_slot].is_free_for_allocation());
    assert_eq!(state.combat_actors[actor_slot].owner_target_class, 13);
    assert_eq!(
        state.active_objects[active_object_slot].type_byte,
        0
    );
    assert_eq!(state.active_objects[active_object_slot].tile, 0);
    assert_eq!(state.active_objects[active_object_slot].aux1, 0);
    assert_eq!(state.active_objects[active_object_slot].phase, STEADY_PHASE);
    assert_eq!(state.active_objects[active_object_slot].aux3, 0xa5);
    assert_eq!(state.combat_action_result, COMBAT_ACTION_RESULT_VANISH_NARRATED);
    assert_eq!(state.pending_combat_terrain_reveals.len(), 1);
    let reveal = &state.pending_combat_terrain_reveals[0];
    assert_eq!(reveal.actor_slot, actor_slot);
    assert_eq!(reveal.arena_cell, (4, 5));
    assert_eq!(reveal.terrain_tile, 0x42);
    assert_eq!(reveal.pixel_order.len(), 256);
    assert_eq!(reveal.world_tick_after_operations.len(), 31);
    assert_eq!(state.message, "Wanderer vanishes!");
}

#[test]
fn combat_monster_vanish_faint_tail_disarms_and_sleeps_first_controlled_party_actor() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.active_player = Some(0);
    state.party_equipment[0][3] = EQUIPMENT_SWORD_OF_CHAOS as u8;
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        20,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED,
        0,
        0,
        0,
        5,
        5,
    ]);
    state.active_objects[0] = ActiveObject {
        type_byte: PLAYER_TILE,
        tile: PLAYER_TILE,
        x: 5,
        y: 5,
        phase: STEADY_PHASE,
        ..ActiveObject::empty()
    };
    let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let active_object_slot = 11;
    place_death_side_effect_monster(&mut state, 13, actor_slot, active_object_slot);

    let mut dead = state.clone();
    dead.party[0].status = b'D';
    let dead_sound_before = dead.sound_effect_serial;
    dead.apply_combat_weapon_damage_to_target(
        None,
        actor_slot,
        COMBAT_INSTANT_KILL_DAMAGE,
        true,
    )
    .unwrap();
    assert_eq!(dead.party[0].status, b'D');
    assert_eq!(dead.party_equipment[0][3], EQUIPMENT_EMPTY);
    assert_eq!(
        dead.sound_effects_after(dead_sound_before).last(),
        Some(&SoundEffect::ControlledPartyFaint)
    );
    assert_eq!(
        dead.combat_actors[0].flags,
        COMBAT_ACTOR_FLAG_SELECTABLE_80
    );
    assert_eq!(
        dead.combat_action_result,
        COMBAT_ACTION_RESULT_VANISH_NARRATED
    );

    let mut suppressed = state.clone();
    suppressed.combat_frame_snapshot = Some(CombatFrameSnapshot {
        area: suppressed.area,
        player: suppressed.player,
        active_objects: suppressed.active_objects.clone(),
        active_player: suppressed.active_player,
        combat_terrain: suppressed.combat_terrain,
        dungeon_room_clear_on_success: None,
        enter_endgame_after_successful_combat: false,
        endgame_messages: None,
        endgame_tableau_map: None,
        encounter_mode_high_bit: false,
        suppress_controlled_faint_sleep_tick: true,
        exit_announced: false,
        established_exit_direction_code: None,
    });
    let suppressed_animation_before = suppressed.animation.frame;
    suppressed
        .apply_combat_weapon_damage_to_target(
            None,
            actor_slot,
            COMBAT_INSTANT_KILL_DAMAGE,
            true,
        )
        .unwrap();
    assert_eq!(
        suppressed.animation.frame,
        suppressed_animation_before.wrapping_add(COMBAT_TERRAIN_REVEAL_WORLD_TICKS)
            % STATIC_TILE_ANIMATION_PERIOD_TICKS
    );

    let animation_before = state.animation.frame;
    let sound_before = state.sound_effect_serial;

    state
        .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
        .unwrap();

    assert_eq!(state.party[0].status, b'S');
    assert_eq!(state.party_equipment[0][3], EQUIPMENT_EMPTY);
    assert_eq!(
        state.combat_actors[0].flags,
        COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_STATUS_DISABLED
    );
    assert_eq!(state.active_objects[0].tile, COMBAT_POTION_SLEEP_DISPLAY_TILE);
    assert_eq!(state.active_player, None);
    assert_eq!(state.combat_action_result, COMBAT_ACTION_RESULT_SLEEP);
    assert_eq!(state.message, "Avatar passes out!");
    assert_eq!(
        state.sound_effects_after(sound_before).last(),
        Some(&SoundEffect::ControlledPartyFaint)
    );
    assert_eq!(
        state.animation.frame,
        animation_before
            .wrapping_add(COMBAT_TERRAIN_REVEAL_WORLD_TICKS)
            .wrapping_add(1)
            % STATIC_TILE_ANIMATION_PERIOD_TICKS
    );
    assert!(state
        .message_entries()
        .iter()
        .any(|entry| entry.text == "Wanderer vanishes!"));
}

#[test]
fn combat_monster_default_death_rejecting_terrain_releases_without_drop_rolls() {
    let mut state = world_state(open_world_grid(), 10, 20);
    let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let active_object_slot = 10;
    let stats = place_death_side_effect_monster(&mut state, 32, actor_slot, active_object_slot);
    state.combat_terrain[5][4] = 0x02;
    state.active_objects[active_object_slot].aux3 = 0xa5;
    let prng_before = state.prng_state;

    state
        .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
        .unwrap();

    assert!(state.combat_actors[actor_slot].is_free_for_allocation());
    assert_eq!(state.combat_actors[actor_slot].owner_target_class, stats.class);
    assert_eq!(
        state.active_objects[active_object_slot],
        ActiveObject {
            phase: 7,
            aux3: 0xa5,
            ..ActiveObject::empty()
        }
    );
    assert_eq!(state.prng_state, prng_before);
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
fn combat_monster_gargoyle_death_stamps_lava_and_releases_slot_without_marker() {
    // `combat.md §6.3` Gargoyle row + "Gargoyle does not fall through to
    // the ordinary path": the branch writes `0x4C` into the arena terrain
    // cell under the actor, writes **no** tile byte into the active-object
    // record, runs no drop rolls, and releases the slot. The earlier
    // engine reading — lava plus a verbatim copy of the default
    // drop-check, including its two PRNG draws — is what this pins out.
    let mut state = world_state(open_world_grid(), 10, 20);
    let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let active_object_slot = 13;
    place_death_side_effect_monster(&mut state, 30, actor_slot, active_object_slot);
    state.active_objects[active_object_slot].aux3 = 0xa5;
    let marker_before = state.active_objects[active_object_slot];
    let prng_before = state.prng_state;

    let application = state
        .apply_combat_weapon_damage_to_target(None, actor_slot, COMBAT_INSTANT_KILL_DAMAGE, true)
        .unwrap();

    assert!(matches!(
        application,
        CombatWeaponDamageApplication::Monster { damage, .. }
            if damage.death_path == Some(CombatMonsterDeathPath::SpecialTileTransition)
    ));
    assert_eq!(
        state.combat_terrain[5][4],
        COMBAT_GARGOYLE_DEATH_TERRAIN_TILE
    );
    assert!(
        state.combat_actors[actor_slot].is_free_for_allocation(),
        "the Gargoyle branch releases its descriptor slot"
    );
    assert_eq!(
        state.active_objects[active_object_slot],
        ActiveObject {
            phase: marker_before.phase,
            aux3: marker_before.aux3,
            ..ActiveObject::empty()
        },
        "the Gargoyle branch clears bytes 0..5 and preserves bytes 6..7"
    );
    assert_eq!(
        state.prng_state, prng_before,
        "the Gargoyle branch consumes no drop rolls"
    );
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
            poison_status_outcome: Some(CombatPoisonStatusAttackOutcome::PoisonedPartyMember {
                status_before: b'G',
                status_after: b'P',
            }),
            resolution: None,
            damage_application: None,
        }
    );
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[0].hp, 12);
    assert_eq!(state.combat_interference_sources[0], 8);
}

#[test]
fn combat_monster_adjacent_miss_overwrites_interference_but_controlled_attack_does_not() {
    let mut automatic = combat_monster_attack_state(COMBAT_CLASS_GIANT_RAT, 6, 5);
    automatic.combat_interference_sources[0] = 7;

    let missed = automatic
        .resolve_and_apply_combat_monster_attack(8, 0, 7, 255, 1, false, 8, Some(false))
        .unwrap();
    assert!(matches!(
        missed.resolution,
        Some(CombatWeaponAttackResolution::Miss { .. })
    ));
    assert_eq!(automatic.combat_interference_sources[0], 8);

    let mut controlled = combat_monster_attack_state(COMBAT_CLASS_GIANT_RAT, 6, 5);
    controlled.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
    controlled.combat_interference_sources[0] = 7;
    assert!(
        controlled
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 255, 1, false, 8, Some(false))
            .is_some()
    );
    assert_eq!(controlled.combat_interference_sources[0], 7);
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
    state.combat_interference_sources[0] = 7;

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
    assert_eq!(state.combat_interference_sources[0], 7);
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
    let defense_roll = u5_prng_range_u16(&mut expected_prng, 0, u16::from(stats.defense)) as u8;
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
    absorbed.active_effect_counter = NEGATE_MAGIC_ACTIVE_EFFECT_DURATION;
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

    absorbed.active_effect_counter = 0;
    assert_eq!(
        absorbed
            .cast_spell_from_suffix("1GP7", std::path::Path::new(""))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(absorbed.spell_charges[spell_index], 1);
    assert_eq!(absorbed.party[0].mana, 1);
    assert_eq!(absorbed.turn, 0);
    assert_eq!(
        absorbed.message,
        "Target? Use C1GP7 to target a live combat slot."
    );
}

#[test]
fn kill_rejects_protected_special_classes_after_resources_without_randomness() {
    assert!(combat_class_is_protected_special(COMBAT_CLASS_BLACKTHORN));
    assert!(combat_class_is_protected_special(COMBAT_CLASS_LORD_BRITISH));
    assert!(combat_class_is_protected_special(COMBAT_CLASS_SHADOW_LORD));
    assert!(!combat_class_is_protected_special(COMBAT_CLASS_DAEMON));

    let spell_index = spell_index_from_code("CX").unwrap();
    let mana_cost = (spell_index / 6 + 1) as u8;
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    for class in [
        COMBAT_CLASS_BLACKTHORN,
        COMBAT_CLASS_LORD_BRITISH,
        COMBAT_CLASS_SHADOW_LORD,
    ] {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party[0].mana = mana_cost;
        state.party[0].level = mana_cost;
        state.spell_charges[spell_index] = 1;
        state.prng_state = 0x1234;
        state.combat_actors[target_slot] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            class,
            0,
            0,
            5,
            5,
        ]);
        let target_before = state.combat_actors[target_slot];
        let prng_before = state.prng_state;

        assert_eq!(
            state.cast_active_target_combat_spell(
                0,
                spell_index,
                CombatSpellDamageKind::Kill,
                target_slot,
            ),
            MoveOutcome::Blocked
        );

        assert_eq!(state.spell_charges[spell_index], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.prng_state, prng_before);
        assert_eq!(state.combat_actors[target_slot], target_before);
        assert_eq!(state.message, "Failed!");
    }
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
    assert!(
        single
            .step_active_cast('G', "P", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(
        single
            .step_active_cast(' ', "", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
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
    assert!(
        double
            .step_active_cast('G', "P", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(
        double
            .step_active_cast(' ', "", std::path::Path::new(""))
            .unwrap()
            .is_none()
    );
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
fn combat_cast_repel_undead_routes_resources_and_forces_undead_to_flee() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.party_intelligence[0] = u8::MAX;
    state.party[0].mana = REPEL_UNDEAD_COST;
    state.party[0].level = REPEL_UNDEAD_COST;
    state.party_experience = vec![10];
    state.spell_charges[REPEL_UNDEAD_SPELL_INDEX] = 1;

    let ghost = combat_class_stats(23).unwrap();
    let skeleton = combat_class_stats(33).unwrap();
    let orc = combat_class_stats(32).unwrap();
    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::for_monster_placement(
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
    assert_eq!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].hp_or_wound, 1);
    assert!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_fleeing());
    assert_eq!(
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].hp_or_wound,
        1
    );
    assert!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_fleeing());
    assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_marked_dead());
    assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_marked_dead());
    assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2].is_marked_dead());
    assert_eq!(
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2].hp_or_wound,
        orc.max_hp
    );
    assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2].is_fleeing());
    assert_eq!(state.party_experience[0], 10);
    assert_eq!(state.message, "Repel Undead! 2 undead repelled.");
}

#[test]
fn combat_cast_directed_sleep_and_poison_wind_mutate_party_targets() {
    let mut sleep = world_state(open_world_grid(), 10, 20);
    sleep.combat_active = true;
    sleep.party_intelligence = vec![u8::MAX, 0];
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
        CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 6, 5]);

    assert_eq!(
        sleep
            .cast_spell_from_suffix("1IZ6", std::path::Path::new(""))
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
            .cast_spell_from_suffix("1HIN6", std::path::Path::new(""))
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
    death.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    death.combat_active = true;
    death.party_intelligence = vec![u8::MAX, 0];
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
        CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 6, 5]);
    let stats = combat_class_stats(32).unwrap();
    death.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::for_monster_placement(
        stats,
        COMBAT_PARTY_ACTOR_SLOTS as u8,
        7,
        5,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        0,
    );

    assert_eq!(
        death
            .cast_spell_from_suffix("1CGIV6", std::path::Path::new(""))
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
    flame.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::for_monster_placement(
        stats,
        COMBAT_PARTY_ACTOR_SLOTS as u8,
        6,
        5,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        0,
    );

    assert_eq!(
        flame
            .cast_spell_from_suffix("1FHI6", std::path::Path::new(""))
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
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
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
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
    let stats = combat_class_stats(32).unwrap();
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[target_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
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
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        1, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
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
    state.combat_actors[target_slot].base_step = 1;

    let initial_prng = state.prng_state;

    assert_eq!(
        state
            .cast_spell_from_suffix("1IPVY", std::path::Path::new(""))
            .unwrap(),
        MoveOutcome::Cast
    );

    assert_eq!(state.spell_charges[spell_index], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.turn, 1);
    assert_ne!(state.prng_state, initial_prng);
    assert_eq!(state.message, "Tremor!");
    assert!(state.party[0].hp < 30);
    assert!(state.combat_actors[target_slot].hp_or_wound < stats.max_hp);
    assert!(state.party_experience[0] > 10);
}

#[test]
fn combat_cast_polymorph_routes_resources_and_replaces_hostile_creature() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
    assert_eq!(
        state.combat_actors[target_slot].hp_or_wound,
        rat_stats.max_hp
    );
    assert_eq!(
        state.combat_actors[target_slot].active_object_slot,
        target_slot as u8
    );
    assert_eq!(
        (
            state.combat_actors[target_slot].x,
            state.combat_actors[target_slot].y
        ),
        (5, 6)
    );
    assert_eq!(
        state.active_objects[target_slot].type_byte,
        COMBAT_CLASS_GIANT_RAT_SPRITE_BASE
    );
    assert_eq!(
        state.active_objects[target_slot].tile,
        COMBAT_CLASS_GIANT_RAT_SPRITE_BASE
    );
    assert_eq!(
        (
            state.active_objects[target_slot].x,
            state.active_objects[target_slot].y
        ),
        (5, 6)
    );
    assert_eq!(state.active_objects[target_slot].aux1, 0x33);
    assert_eq!(state.active_objects[target_slot].aux3, 0x44);
    assert!(state.visibility_dirty);
}

#[test]
fn combat_cast_polymorph_rejects_same_faction_target_before_resources() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 4, 3]);

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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
            .cast_spell_from_suffix("1GIN4,3", std::path::Path::new(""))
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
            .cast_spell_from_suffix("1FGI4,3", std::path::Path::new(""))
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
fn combat_cast_fire_field_places_marker_without_random_gate() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
            .cast_spell_from_suffix("1FGI4,3", std::path::Path::new(""))
            .unwrap(),
        MoveOutcome::Cast
    );

    assert_eq!(state.spell_charges[spell_index], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "Fire field placed.");
    assert!(!state.combat_actors[target_slot].is_marked_dead());
    assert_eq!(state.combat_actors[target_slot].hp_or_wound, stats.max_hp);
    assert_eq!(state.active_objects[target_slot].tile, 0x70);
    assert_eq!(state.active_objects[1].type_byte, COMBAT_FIELD_KIND_FIRE);
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (4, 3)
    );
}

#[test]
fn combat_cast_field_spell_places_marker_without_actor_contact_target() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
            .cast_spell_from_suffix("1GIN4,3", std::path::Path::new(""))
            .unwrap(),
        MoveOutcome::Cast
    );

    assert_eq!(state.spell_charges[spell_index], 0);
    assert_eq!(state.party[0].mana, 0);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "Poison field placed.");
    assert_eq!(state.active_objects[1].type_byte, COMBAT_FIELD_KIND_POISON);
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        (4, 3)
    );
}

#[test]
fn combat_cast_dispel_field_removes_matching_arena_marker() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
    assert!(
        state.active_objects[1..]
            .iter()
            .all(|object| object.is_empty())
    );
}

#[test]
fn tremor_spell_damage_application_requires_roll_for_each_accepted_target() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party[0].hp = 12;
    state.party_experience = vec![10];
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
    let stats = combat_class_stats(32).unwrap();
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[target_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
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
    state.party = vec![living, PartyMember { slot: 1, ..living }];
    state.party_experience = vec![10, 20];
    state.active_player = Some(1);
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3,
    ]);
    state.combat_actors[1] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 4, 4,
    ]);
    let stats = combat_class_stats(32).unwrap();
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[target_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

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
fn directed_spell_damage_skips_disabled_targets_in_cone() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.party_experience = vec![10];
    let stats = combat_class_stats(32).unwrap();
    let disabled_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let enabled_slot = disabled_slot + 1;
    state.combat_actors[disabled_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
    state.combat_actors[disabled_slot].flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    state.combat_actors[enabled_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            8,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

    assert_eq!(
        state.apply_directed_combat_spell_damage(
            Some(0),
            CombatDirectedSpellEffect::DeathWind,
            &[(4, 5), (5, 5)],
            &[],
        ),
        Some(CombatDirectedSpellDamageApplication {
            effect: CombatDirectedSpellEffect::DeathWind,
            applications: vec![CombatDirectedSpellSlotDamageApplication {
                target_slot: enabled_slot,
                raw_damage: COMBAT_INSTANT_KILL_DAMAGE,
                damage_application: CombatWeaponDamageApplication::Monster {
                    target_slot: enabled_slot,
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
            }],
        })
    );
    assert_eq!(state.combat_actors[disabled_slot].hp_or_wound, stats.max_hp);
    assert!(state.combat_actors[enabled_slot].is_marked_dead());
}

#[test]
fn directed_spell_damage_application_handles_flame_wind_rolls_and_non_damage_effects() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.party_experience = vec![10];
    let stats = combat_class_stats(32).unwrap();
    let first_slot = COMBAT_PARTY_ACTOR_SLOTS;
    let second_slot = first_slot + 1;
    state.combat_actors[first_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
    state.combat_actors[second_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            8,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

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
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4,
    ]);
    let stats = combat_class_stats(32).unwrap();
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[target_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
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
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
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
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4,
    ]);
    state.combat_actors[1] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 5, 4,
    ]);
    let stats = combat_class_stats(32).unwrap();
    let first_monster = COMBAT_PARTY_ACTOR_SLOTS;
    let second_monster = first_monster + 1;
    state.combat_actors[first_monster] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
    state.combat_actors[second_monster] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            8,
            6,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

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
                    fallback_damage_application: Some(CombatWeaponDamageApplication::Party {
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
                    },),
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
    assert_eq!(
        state.combat_actors[second_monster].hp_or_wound,
        stats.max_hp
    );
    assert_eq!(state.party_experience, vec![10, 20]);
}

#[test]
fn directed_spell_status_application_requires_poison_inputs_before_mutation() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party[0].status = b'G';
    state.party[0].hp = 12;
    state.combat_actors[0] = CombatActorDescriptor::from_row([
        12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4,
    ]);
    let stats = combat_class_stats(32).unwrap();
    let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
    state.combat_actors[target_slot] =
        CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );

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
    state.combat_actors[0] = CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
    state.active_objects[0].tile = 0x10;

    assert_eq!(
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Poison, 0, 4, 0),
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
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Poison, 0, 4, 0),
        Some(CombatArenaFieldContactApplication {
            field: CombatArenaFieldKind::Poison,
            target_slot: 0,
            contact_outcome: CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage: 4 },
            damage_application: Some(CombatWeaponDamageApplication::Party {
                target_slot: 0,
                damage: CombatPartyDamageOutcome {
                    raw_damage: 4,
                    applied_damage: 4,
                    missed: false,
                    instant_kill: false,
                    killed: false,
                    status_before: b'P',
                    status_after: b'P',
                },
            }),
        })
    );
    assert_eq!(state.party[0].hp, 8);
    assert_eq!(state.party_experience, vec![10]);

    assert_eq!(
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Fire, 0, 0, 10),
        Some(CombatArenaFieldContactApplication {
            field: CombatArenaFieldKind::Fire,
            target_slot: 0,
            contact_outcome: CombatArenaFieldContactOutcome::FireDamage { raw_damage: 10 },
            damage_application: Some(CombatWeaponDamageApplication::Party {
                target_slot: 0,
                damage: CombatPartyDamageOutcome {
                    raw_damage: 10,
                    applied_damage: 8,
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
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.party_experience = vec![10];
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
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
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Poison, second_slot, 19, 0,),
        Some(CombatArenaFieldContactApplication {
            field: CombatArenaFieldKind::Poison,
            target_slot: second_slot,
            contact_outcome: CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass,
            damage_application: None,
        })
    );
    assert_eq!(state.combat_actors[second_slot].hp_or_wound, stats.max_hp);

    assert_eq!(
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Sleep, second_slot, 0, 0,),
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
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Fire, first_slot, 0, 10,),
        Some(CombatArenaFieldContactApplication {
            field: CombatArenaFieldKind::Fire,
            target_slot: first_slot,
            contact_outcome: CombatArenaFieldContactOutcome::FireDamage { raw_damage: 10 },
            damage_application: Some(CombatWeaponDamageApplication::Monster {
                target_slot: first_slot,
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
            }),
        })
    );
    assert!(state.combat_actors[first_slot].is_marked_dead());
    assert_eq!(state.party_experience, vec![10]);
}

#[test]
fn arena_field_contact_application_targets_current_actor_and_ignores_energy() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_actors[0] = CombatActorDescriptor::from_row([12, 1, 0, 0, 0, 0, 4, 4]);
    state.active_objects[0].tile = 0x10;

    assert_eq!(
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Fire, 0, 0, 10),
        Some(CombatArenaFieldContactApplication {
            field: CombatArenaFieldKind::Fire,
            target_slot: 0,
            contact_outcome: CombatArenaFieldContactOutcome::FireDamage { raw_damage: 10 },
            damage_application: Some(CombatWeaponDamageApplication::Party {
                target_slot: 0,
                damage: CombatPartyDamageOutcome {
                    raw_damage: 10,
                    applied_damage: 10,
                    missed: false,
                    instant_kill: false,
                    killed: false,
                    status_before: b'G',
                    status_after: b'G',
                },
            }),
        })
    );
    assert_eq!(state.party[0].hp, 50);

    assert_eq!(
        state.apply_combat_arena_field_contact(CombatArenaFieldKind::Energy, 0, 0, 0),
        None
    );
    assert_eq!(state.party[0].hp, 50);
}

#[test]
fn arena_field_contact_scan_skips_linked_renderer_and_uses_first_separate_marker() {
    let mut state = combat_player_command_state(10, 10);
    state.combat_actors[0].x = 4;
    state.combat_actors[0].y = 4;
    state.active_objects[0] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_FIRE,
        tile: PLAYER_TILE,
        x: 4,
        y: 4,
        ..ActiveObject::empty()
    };
    state.active_objects[1] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_SLEEP,
        tile: COMBAT_FIELD_KIND_SLEEP,
        x: 4,
        y: 4,
        ..ActiveObject::empty()
    };
    state.active_objects[2] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_POISON,
        tile: COMBAT_FIELD_KIND_POISON,
        x: 4,
        y: 4,
        ..ActiveObject::empty()
    };
    let prng_before = state.prng_state;

    assert!(matches!(
        state.apply_combat_arena_field_contact_for_actor_position(0),
        Some(CombatArenaFieldContactApplication {
            field: CombatArenaFieldKind::Sleep,
            target_slot: 0,
            contact_outcome: CombatArenaFieldContactOutcome::SleptPartyMember {
                status_before: b'G',
                status_after: b'S',
            },
            damage_application: None,
        })
    ));
    assert_eq!(state.party[0].status, b'S');
    assert_eq!(state.prng_state, prng_before);
    assert_eq!(state.active_objects[1].type_byte, COMBAT_FIELD_KIND_SLEEP);
    assert_eq!(state.active_objects[2].type_byte, COMBAT_FIELD_KIND_POISON);
}

#[test]
fn combat_arena_terrain_contact_kind_recognizes_only_exact_published_bytes() {
    for tile in 0u8..=u8::MAX {
        let expected = match tile {
            COMBAT_CONTACT_TERRAIN_SWAMP => Some(CombatArenaFieldKind::Poison),
            COMBAT_CONTACT_TERRAIN_MOLTEN_LAVA | COMBAT_CONTACT_TERRAIN_FIREPLACE => {
                Some(CombatArenaFieldKind::Fire)
            }
            _ => None,
        };
        assert_eq!(
            combat_arena_terrain_contact_kind(tile),
            expected,
            "tile {tile:#04x}"
        );
    }
}

#[test]
fn terrain_poison_preempts_markers_even_when_poison_is_status_or_tile_gated() {
    let mut state = combat_player_command_state(10, 10);
    state.combat_actors[0].x = 4;
    state.combat_actors[0].y = 4;
    state.active_objects[0].x = 4;
    state.active_objects[0].y = 4;
    state.active_objects[0].tile = 0x10;
    state.combat_terrain[4][4] = COMBAT_CONTACT_TERRAIN_SWAMP;
    state.active_objects[1] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_FIRE,
        tile: COMBAT_FIELD_KIND_FIRE,
        x: 4,
        y: 4,
        ..ActiveObject::empty()
    };
    state.party[0].status = b'G';
    let prng_before_good = state.prng_state;

    assert!(matches!(
        state.apply_combat_post_dispatch_contact_for_actor_position(0),
        Some(CombatPostDispatchContactApplication {
            source: CombatPostDispatchContactSource::ArenaTerrain {
                tile: COMBAT_CONTACT_TERRAIN_SWAMP,
            },
            field_contact: CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Poison,
                contact_outcome: CombatArenaFieldContactOutcome::PoisonedPartyMember { .. },
                ..
            },
        })
    ));
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.prng_state, prng_before_good);

    state.active_objects[0].tile = 0x80;
    let hp_before_rejection = state.party[0].hp;
    let prng_before_rejection = state.prng_state;
    assert!(matches!(
        state.apply_combat_post_dispatch_contact_for_actor_position(0),
        Some(CombatPostDispatchContactApplication {
            source: CombatPostDispatchContactSource::ArenaTerrain {
                tile: COMBAT_CONTACT_TERRAIN_SWAMP,
            },
            field_contact: CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Poison,
                contact_outcome: CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass,
                ..
            },
        })
    ));
    assert_eq!(state.party[0].hp, hp_before_rejection);
    assert_eq!(state.prng_state, prng_before_rejection);
    assert_eq!(state.active_objects[1].type_byte, COMBAT_FIELD_KIND_FIRE);
}

#[test]
fn lava_and_fireplace_terrain_use_one_inclusive_raw_fire_damage_draw() {
    for tile in [
        COMBAT_CONTACT_TERRAIN_MOLTEN_LAVA,
        COMBAT_CONTACT_TERRAIN_FIREPLACE,
    ] {
        let mut state = combat_player_command_state(10, 10);
        state.combat_actors[0].x = 4;
        state.combat_actors[0].y = 4;
        state.active_objects[0].x = 4;
        state.active_objects[0].y = 4;
        state.active_objects[0].tile = 0x10;
        state.combat_terrain[4][4] = tile;
        state.active_objects[1] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_SLEEP,
            tile: COMBAT_FIELD_KIND_SLEEP,
            x: 4,
            y: 4,
            ..ActiveObject::empty()
        };
        let hp_before = state.party[0].hp;
        let mut expected_prng = state.prng_state;
        let expected_raw = u5_prng_range_u16(&mut expected_prng, 0, 10) as u8;

        assert!(matches!(
            state.apply_combat_post_dispatch_contact_for_actor_position(0),
            Some(CombatPostDispatchContactApplication {
                source: CombatPostDispatchContactSource::ArenaTerrain {
                    tile: selected_tile,
                },
                field_contact: CombatArenaFieldContactApplication {
                    field: CombatArenaFieldKind::Fire,
                    contact_outcome: CombatArenaFieldContactOutcome::FireDamage { raw_damage },
                    ..
                },
            }) if selected_tile == tile && raw_damage == expected_raw
        ));
        assert_eq!(state.prng_state, expected_prng);
        assert_eq!(
            state.party[0].hp,
            hp_before.saturating_sub(expected_raw as u16)
        );
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.active_objects[1].type_byte, COMBAT_FIELD_KIND_SLEEP);
    }
}

#[test]
fn doom_absorption_skips_digit_selection_and_automatic_dispatch() {
    let mut player = combat_player_command_state(10, 10);
    player.combat_actors[0].x = 5;
    player.combat_actors[0].y = 2;
    player.active_objects[0].x = 5;
    player.active_objects[0].y = 2;
    player.active_objects[20] = ActiveObject {
        type_byte: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
        tile: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
        x: 5,
        y: 1,
        ..ActiveObject::empty()
    };
    player.active_player = Some(0);

    let selection = player
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('1'))
        .expect("digit selection should dispatch");
    assert!(matches!(
        selection.action,
        CombatPlayerCommandAction::ActivePlayerSelection(_)
    ));
    assert_eq!(selection.absorbable_contact, None);
    assert_eq!(selection.post_dispatch_contact, None);
    assert_eq!(player.active_player, Some(0));

    let mut automatic = combat_ai_turn_state(8, 2);
    automatic.combat_actors[8].phase_counter = 1;
    automatic.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    automatic.active_effect_counter = 3;
    automatic.active_objects[20] = ActiveObject {
        type_byte: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
        tile: DUNGEON_ROOM_ABSORBABLE_FIELD_SOURCE,
        x: 8,
        y: 1,
        ..ActiveObject::empty()
    };
    automatic.active_player = Some(0);
    let dispatch = automatic.apply_combat_actor_slot_dispatch_with_inputs(
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
    assert!(matches!(
        dispatch,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::NegateTimeSkipped,
            ..
        }
    ));
    assert_eq!(automatic.active_player, Some(0));
    assert_ne!(automatic.message, "Absorbed!");
}

#[test]
fn arena_field_contact_consumes_only_branch_specific_inclusive_damage_draws() {
    let mut state = combat_player_command_state(10, 10);
    state.combat_actors[0].x = 4;
    state.combat_actors[0].y = 4;
    state.active_objects[0].x = 4;
    state.active_objects[0].y = 4;
    state.active_objects[0].tile = 0x10;
    state.active_objects[1] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_POISON,
        tile: COMBAT_FIELD_KIND_POISON,
        x: 4,
        y: 4,
        ..ActiveObject::empty()
    };
    state.party[0].status = b'G';

    state.prng_state = 0x1234;
    let prng_before_good_poison = state.prng_state;
    assert!(matches!(
        state.apply_combat_arena_field_contact_for_actor_position(0),
        Some(CombatArenaFieldContactApplication {
            contact_outcome: CombatArenaFieldContactOutcome::PoisonedPartyMember { .. },
            ..
        })
    ));
    assert_eq!(state.prng_state, prng_before_good_poison);

    state.party[0].status = b'P';
    state.active_objects[0].tile = 0x80;
    let prng_before_linked_rejection = state.prng_state;
    assert!(matches!(
        state.apply_combat_arena_field_contact_for_actor_position(0),
        Some(CombatArenaFieldContactApplication {
            contact_outcome: CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass,
            ..
        })
    ));
    assert_eq!(state.prng_state, prng_before_linked_rejection);

    state.active_objects[0].tile = 0x10;
    let hp_before_poison = state.party[0].hp;
    let mut expected_prng = state.prng_state;
    let expected_poison_damage = u5_prng_range_u16(&mut expected_prng, 0, 20) as u8;
    assert!(matches!(
        state.apply_combat_arena_field_contact_for_actor_position(0),
        Some(CombatArenaFieldContactApplication {
            contact_outcome:
                CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage },
            ..
        }) if raw_damage == expected_poison_damage
    ));
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(
        state.party[0].hp,
        hp_before_poison.saturating_sub(expected_poison_damage as u16)
    );

    state.active_objects[1].type_byte = COMBAT_FIELD_KIND_FIRE;
    state.active_objects[1].tile = COMBAT_FIELD_KIND_FIRE;
    let hp_before_fire = state.party[0].hp;
    let mut expected_prng = state.prng_state;
    let expected_fire_damage = u5_prng_range_u16(&mut expected_prng, 0, 10) as u8;
    assert!(matches!(
        state.apply_combat_arena_field_contact_for_actor_position(0),
        Some(CombatArenaFieldContactApplication {
            contact_outcome: CombatArenaFieldContactOutcome::FireDamage { raw_damage },
            ..
        }) if raw_damage == expected_fire_damage
    ));
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(
        state.party[0].hp,
        hp_before_fire.saturating_sub(expected_fire_damage as u16)
    );
}

#[test]
fn energy_field_blocks_player_and_ai_without_running_contact_on_reprompt() {
    let mut state = combat_player_command_state(10, 10);
    state.active_objects[1] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_FIRE,
        tile: COMBAT_FIELD_KIND_FIRE,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    };
    state.active_objects[2] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_ENERGY,
        tile: COMBAT_FIELD_KIND_ENERGY,
        x: 6,
        y: 5,
        ..ActiveObject::empty()
    };
    let hp_before = state.party[0].hp;
    let prng_before = state.prng_state;

    assert_eq!(
        state.combat_destination_walkable_for_direction(0, COMBAT_DIRECTION_EAST),
        Some(false)
    );
    assert!(!state.combat_legal_cell_mask()[5][6]);
    let application = state
        .apply_combat_player_command_with_attack_inputs(
            0,
            CombatPlayerCommandInput::Direction(COMBAT_DIRECTION_EAST),
            CombatPlayerWeaponAttackInputs::default(),
        )
        .expect("blocked direction should return a re-prompt application");

    assert!(matches!(
        application.action,
        CombatPlayerCommandAction::StepOrAttack {
            outcome: CombatStepOrAttackPrimitiveOutcome::BlockedWall,
            ..
        }
    ));
    assert!(application.reprompt);
    assert_eq!(application.post_dispatch_contact, None);
    assert_eq!(state.party[0].hp, hp_before);
    assert_eq!(state.prng_state, prng_before);
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));
}

#[test]
fn automatic_no_action_and_status_disabled_dispatches_run_field_contact() {
    let mut skipped = combat_ai_turn_state(8, 5);
    skipped.combat_terrain[5][8] = 0x05;
    skipped.combat_actors[8].phase_counter = 1;
    skipped.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    skipped.active_effect_counter = 3;
    skipped.active_objects[1] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_SLEEP,
        tile: COMBAT_FIELD_KIND_SLEEP,
        x: 8,
        y: 5,
        ..ActiveObject::empty()
    };
    let prng_before_skip = skipped.prng_state;

    let application = skipped.apply_combat_actor_slot_dispatch_with_inputs(
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
    assert!(matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::NegateTimeSkipped,
            ..
        }
    ));
    assert!(skipped.combat_actors[8].is_status_disabled());
    assert_eq!(skipped.prng_state, prng_before_skip);

    let mut disabled = combat_ai_turn_state(8, 5);
    disabled.combat_terrain[5][8] = 0x05;
    disabled.combat_actors[8].phase_counter = 1;
    disabled.combat_actors[8].hp_or_wound = 50;
    disabled.combat_actors[8].set_status_disabled();
    disabled.active_objects[1] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_FIRE,
        tile: COMBAT_FIELD_KIND_FIRE,
        x: 8,
        y: 5,
        ..ActiveObject::empty()
    };
    disabled.prng_state = 0x4321;
    let hp_before = disabled.combat_actors[8].hp_or_wound;
    let mut expected_prng = disabled.prng_state;
    let _wake_roll = u5_prng_range_u16(
        &mut expected_prng,
        u16::from(COMBAT_SLEEP_WAKE_ROLL_LOW),
        u16::from(COMBAT_SLEEP_WAKE_ROLL_HIGH),
    );
    let expected_fire_damage = u5_prng_range_u16(&mut expected_prng, 0, 10) as u8;

    let application = disabled.apply_combat_actor_slot_dispatch_with_inputs(
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
    assert!(matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::StatusDisabledWake { .. },
            ..
        }
    ));
    assert_eq!(disabled.prng_state, expected_prng);
    assert_eq!(
        disabled.combat_actors[8].hp_or_wound,
        hp_before.saturating_sub(expected_fire_damage)
    );
}

#[test]
fn combat_step_defers_field_contact_until_completed_dispatch_without_consuming_marker() {
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
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
    let prng_before_step = state.prng_state;

    let outcome = state.apply_combat_step_or_attack_primitive(0, 1, COMBAT_DIRECTION_EAST, true);

    assert!(outcome.committed_movement());
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (4, 3));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (4, 3)
    );
    assert_eq!(state.party[0].hp, 20);
    assert_eq!(state.prng_state, prng_before_step);
    assert_eq!(state.party[0].status, b'G');

    let mut expected_prng = state.prng_state;
    let expected_raw = u5_prng_range_u16(&mut expected_prng, 0, 10) as u8;
    let application = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(' '))
        .expect("pass should complete the actor dispatch");
    assert!(matches!(
        application.post_dispatch_contact,
        Some(CombatPostDispatchContactApplication {
            source: CombatPostDispatchContactSource::PlacedMarker {
                active_object_slot: 1,
            },
            field_contact: CombatArenaFieldContactApplication {
                field: CombatArenaFieldKind::Fire,
                target_slot: 0,
                contact_outcome: CombatArenaFieldContactOutcome::FireDamage { raw_damage },
                ..
            }
        }) if raw_damage == expected_raw
    ));
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(state.party[0].hp, 20u16.saturating_sub(expected_raw as u16));
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
fn combat_cursor_blink_tick_reports_cursor_and_secondary_marker_cells() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.active_player = Some(0);
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 6]);
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

    let report = state.apply_combat_cursor_blink_tick();

    assert!(report.cursor_blink_visible);
    assert_eq!(report.cursor_draw_cell, Some((5, 6)));
    assert_eq!(report.secondary_marker_cell, Some((3, 4)));
    assert_eq!(state.active_objects, active_objects_before);
    assert_eq!(state.party, party_before);

    state.combat_cursor_blink = false;
    state.combat_secondary_marker = Some((99, 99));
    let unclipped_report = state.apply_combat_cursor_blink_tick();
    assert_eq!(unclipped_report.cursor_draw_cell, Some((5, 6)));
    assert_eq!(unclipped_report.secondary_marker_cell, Some((99, 99)));

    state.combat_cursor_blink = false;
    state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_TEAM_TOGGLE;
    let non_player_report = state.apply_combat_cursor_blink_tick();
    assert!(non_player_report.cursor_blink_visible);
    assert_eq!(non_player_report.cursor_draw_cell, None);
    assert_eq!(non_player_report.secondary_marker_cell, None);
}

#[test]
fn the_idle_redraw_tick_owns_the_combat_cursor_blink() {
    // `combat.md §7`: the shared tile-painting pass is "run by the idle
    // redraw tick in *every* mode", and its combat-band tail "toggles a
    // blink flag each pass". The frontends pump that tick, so the flag has
    // to move there rather than at a round boundary - otherwise the box the
    // original blinks stands solid for a whole round.
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;

    assert!(!state.combat_cursor_blink);
    state.advance_visual_tick();
    assert!(state.combat_cursor_blink);
    state.advance_visual_tick();
    assert!(!state.combat_cursor_blink);

    // Outside a fight the tail does not run at all.
    let mut exploring = world_state(open_world_grid(), 10, 20);
    exploring.advance_visual_tick();
    assert!(!exploring.combat_cursor_blink);
}

#[test]
fn combat_entry_magic_ring_pass_applies_invisibility_and_vanish_clears_it() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_INVISIBILITY as u8;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4]);
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
    assert_eq!(
        state.active_objects[0].tile,
        COMBAT_HIDDEN_ACTIVE_OBJECT_TILE
    );
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
    assert_eq!(state.message, "A ring has vanished!");
}

#[test]
fn combat_entry_magic_ring_pass_regenerates_living_wearers_and_can_vanish() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party[0].hp = 8;
    state.party[0].max_hp = 10;
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4]);

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
    assert_eq!(state.message, "A ring has vanished!");
}

#[test]
fn combat_frame_entry_runs_vanish_before_invisibility_seating_hook() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_INVISIBILITY as u8;
    state.prng_state = 0x0070;
    let mut expected_prng = state.prng_state;
    assert_eq!(u5_prng_range_u16(&mut expected_prng, 0, 15), 0);
    let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    active_objects[0] = ActiveObject {
        type_byte: PLAYER_TILE,
        tile: PLAYER_TILE,
        x: 4,
        y: 4,
        ..ActiveObject::empty()
    };
    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 4]);

    state.enter_combat_frame(active_objects, actors).unwrap();

    assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
    assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
    assert_eq!(state.active_objects[0].tile, PLAYER_TILE);
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(state.message, "A ring has vanished!");
}

#[test]
fn combat_escape_cleanup_state_wrapper_uses_party_side_descriptor() {
    let mut state = world_state(open_world_grid(), 10, 20);
    assert_eq!(
        state.combat_escape_cleanup_decision(),
        CombatEscapeCleanupDecision::Accepted
    );

    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 20, 0, 0, 5, 5]);
    assert_eq!(
        state.combat_escape_cleanup_decision(),
        CombatEscapeCleanupDecision::RefusedNotYet
    );

    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].mark_dead();
    assert_eq!(
        state.combat_escape_cleanup_decision(),
        CombatEscapeCleanupDecision::Accepted
    );
}

#[test]
fn default_monster_death_marker_keeps_drop_cap_and_special_bit_separate() {
    assert_eq!(COMBAT_DEFAULT_DEATH_DROP_ROLL_MAX, 99);
    assert_eq!(COMBAT_PARTY_CORPSE_TILE, 0x1e);
    // `combat.md §6.3`: the accepted drop leaves a chest (`0x01`);
    // the rejected drop leaves a moldy corpse (`0x1F`). Two distinct
    // tile ids, not one.
    assert_eq!(COMBAT_DEFAULT_DEATH_DROP_TILE, 0x01);
    assert_eq!(COMBAT_DEFAULT_DEATH_NO_DROP_TILE, 0x1F);
    assert_ne!(
        COMBAT_DEFAULT_DEATH_DROP_TILE,
        COMBAT_DEFAULT_DEATH_NO_DROP_TILE
    );
    assert_eq!(COMBAT_VANISH_DEATH_MARKER_TILE, 0x16);
    assert_eq!(COMBAT_GAZER_DEATH_MARKER_TILE, 0x1f);
    assert_eq!(COMBAT_GARGOYLE_DEATH_TERRAIN_TILE, 0x4c);
    // `combat.md §6.3`: the first roll accepts when it is <= the drop cap;
    // only the special-drop bit's second roll is strictly below it.
    assert!(combat_default_death_drop_gate_accepts_inclusive(11, 11));
    assert!(!combat_default_death_drop_gate_accepts_inclusive(11, 12));
    assert!(combat_default_death_drop_gate_accepts(11, 10));
    assert!(!combat_default_death_drop_gate_accepts(11, 11));
    // The shared `1..30` helper never returns zero, so a zero drop cap
    // can never take the accepted branch under either gate.
    assert!(!combat_default_death_drop_gate_accepts_inclusive(0, 1));
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
        CombatDefaultDeathMarker::Drop {
            loot_byte: 0x80 | 30
        }
    );
    assert_eq!(
        resolve_default_monster_death_marker_for_class(48, true, false),
        None
    );
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
    descriptors[3] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        24,
        0,
        0,
        4,
        4,
    ]);
    descriptors[4] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        24,
        0,
        0,
        5,
        4,
    ]);
    descriptors[5] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        24,
        0,
        0,
        6,
        4,
    ]);

    assert_eq!(
        resolve_combat_split_placement(24, 1, false, &descriptors, &[99, 3, 4, 5, 6, 7, 8, 9],),
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
    let mut descriptor = CombatActorDescriptor::for_monster_placement(stats, 7, 4, 5, 0, 0);

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
    let mut physical_half = CombatActorDescriptor::for_monster_placement(skeleton, 7, 4, 5, 0, 0);
    let half = physical_half.apply_monster_damage(9, false).unwrap();
    assert_eq!(half.applied_damage, 4);
    assert_eq!(physical_half.hp_or_wound, 16);

    let mut magical_hit = CombatActorDescriptor::for_monster_placement(skeleton, 7, 4, 5, 0, 0);
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
    // `encounters.md §5`: the second roll draws over the FIRST
    // ROLL'S RESULT (8), not over the class maximum, so the damper
    // "can only *lower* the count. It is a damper, not a doubler."
    assert_eq!(resolve_combat_spawn_count(10, 7, Some(2)), 3);
    assert_eq!(resolve_combat_spawn_count(10, 0, Some(9)), 1);
    assert_eq!(resolve_combat_spawn_count(30, 29, None), 26);
    assert_eq!(resolve_combat_spawn_count(0, 29, None), 0);
    for first_seed in 0..=u8::MAX {
        let undamped = resolve_combat_spawn_count(13, first_seed, None);
        for second_seed in 0..=u8::MAX {
            let damped = resolve_combat_spawn_count(13, first_seed, Some(second_seed));
            assert!(
                damped <= undamped,
                "damper raised the count from {undamped} to {damped}"
            );
            assert!(damped >= 1);
        }
    }
}

#[test]
fn terrain_combat_setup_count_consumes_fortunes_flag_and_town_override() {
    assert_eq!(resolve_terrain_combat_setup_count(10, 0, 7, 2, false), 8);
    assert_eq!(resolve_terrain_combat_setup_count(10, 1, 7, 2, false), 3);
    assert_eq!(resolve_terrain_combat_setup_count(10, 0xff, 7, 2, false), 3);
    assert_eq!(resolve_terrain_combat_setup_count(10, 0xff, 7, 2, true), 1);
    assert_eq!(
        resolve_terrain_combat_setup_count(16, 0xff, 7, 2, false),
        16
    );

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
    // `encounters.md §5`: the damper's second draw is bounded by the
    // first roll's result, so it can only lower the count.
    let first_roll = u5_prng_range_u16(&mut expected_prng, 1, 10) as u8;
    let expected_count = u5_prng_range_u16(&mut expected_prng, 1, u16::from(first_roll)) as u8;

    assert!(expected_count <= first_roll);
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

/// `combat.md §5`: "A town-style single-attacker override applies
/// before the lookup: if the pre-combat scene was a town, dwelling,
/// castle, or keep, the party is on the surface, and the base class
/// is not 12 (Guard), the count is forced to one."
#[test]
fn terrain_combat_town_style_override_matches_combat_md_section_five() {
    assert!(!scene_is_town_dwelling_castle_or_keep(SCENE_OVERWORLD));
    assert!(scene_is_town_dwelling_castle_or_keep(
        SCENE_TOWN_FAMILY_FIRST
    ));
    assert!(scene_is_town_dwelling_castle_or_keep(
        SCENE_TOWN_FAMILY_LAST
    ));
    assert!(!scene_is_town_dwelling_castle_or_keep(
        SCENE_TOWN_FAMILY_LAST + 1
    ));

    // An ordinary townsperson yields a single attacker...
    assert_eq!(resolve_terrain_combat_setup_count(13, 0, 7, 2, true), 1);
    // ...while a Guard falls through to the row's sentinel eight.
    let guard = combat_class_stats(COMBAT_CLASS_GUARD).unwrap();
    assert_eq!(guard.default_spawn_count, 8);
    assert_eq!(
        resolve_terrain_combat_setup_count(guard.default_spawn_count, 0, 7, 2, false),
        8
    );
}

/// `combat.md §5`: "A short combat banner ("CONFLICT") is printed at
/// the start of setup, before any monsters are placed."
#[test]
fn combat_banner_is_the_published_one_word_string() {
    assert_eq!(COMBAT_BANNER, "CONFLICT");
}

/// `magic.md §8`, Crown of Lord British: "Shares the enemy-cast gate
/// with `N`: while the Crown occupies the slot it acts as a permanent
/// Negate Magic aura."
#[test]
fn negate_magic_aura_accepts_negate_magic_and_the_crown() {
    assert!(negate_magic_aura_active(
        Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG),
        10
    ));
    assert!(negate_magic_aura_active(
        Some(CROWN_LB_ACTIVE_EFFECT_TAG),
        0xff
    ));
    assert!(!negate_magic_aura_active(
        Some(CROWN_LB_ACTIVE_EFFECT_TAG),
        0
    ));
    assert!(!negate_magic_aura_active(
        Some(NEGATE_TIME_ACTIVE_EFFECT_TAG),
        10
    ));
    assert!(!negate_magic_aura_active(None, 10));

    // §8 gives the Crown only the enemy-cast gate; the party-side
    // combat C-Cast absorption stays keyed to `N` alone.
    assert!(!resolve_negate_magic_absorbs_combat_cast(
        Some(CROWN_LB_ACTIVE_EFFECT_TAG),
        0xff
    ));
    assert!(resolve_negate_magic_absorbs_combat_cast(
        Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG),
        10
    ));
}

/// `formats/cbt.md §5` + `dungeon-mode.md §14`: a `0xEC..0xEF`
/// source "is placed on the ordinary path with a full combat actor
/// and the tile derived from the substituted class, exactly like any
/// other ordinary source. It receives no auxiliary-byte post-write
/// because that post-write is gated on the special path."
#[test]
fn dungeon_room_vermin_family_places_real_combatants() {
    let setup = DungeonRoomCombatSetup {
        arena_index: 0,
        terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        placement_slots: Vec::new(),
        party_positions: [(0, 0); COMBAT_PARTY_ACTOR_SLOTS],
        setup_sources: vec![DungeonRoomSetupSource::new(0, 0xec, 4, 5).unwrap()],
        scan_sources: true,
    };
    let mut expected_prng = 0x1234u16;
    let palette_ids = dungeon_room_random_special_setup_ids(true, &mut expected_prng);
    let mut prng = 0x1234u16;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 3, &mut prng);

    let class = palette_ids[0];
    assert!(DUNGEON_ROOM_RANDOM_SPECIAL_SETUP_PALETTE.contains(&class));
    let slot = COMBAT_PARTY_ACTOR_SLOTS;
    assert_eq!(instance.placed_count, 1);
    assert_eq!(
        instance.active_objects[slot].tile,
        combat_class_sprite_byte(class)
    );
    assert_eq!(
        instance.active_objects[slot].type_byte,
        combat_class_sprite_byte(class)
    );
    assert_eq!(
        (
            instance.active_objects[slot].x,
            instance.active_objects[slot].y
        ),
        (4, 5)
    );
    assert_eq!(
        instance.active_objects[slot].aux1,
        combat_class_stats(class).unwrap().max_hp
    );
    assert!(!instance.actors[slot].is_empty());
    assert_eq!(instance.actors[slot].owner_target_class, class);
}

/// `dungeon-mode.md §14.1` step 3: the wandering-monster launch
/// synthesises the arena's metadata band instead of loading a
/// `DUNGEON.CBT` record.
#[test]
fn dungeon_ambush_arena_synthesises_published_metadata_band() {
    let mut permutation = [0u8; DUNGEON_ROOM_SOURCE_COUNT];
    for (index, slot) in permutation.iter_mut().enumerate() {
        *slot = index as u8;
    }
    permutation.swap(0, 9);

    let record = CombatArenaRecord::synthesise_dungeon_ambush(
        DUNGEON_AMBUSH_ARENA_FLOOR_TILE,
        0,
        COMBAT_CLASS_BAT,
        3,
        permutation,
    );

    assert_eq!(record.terrain_grid(), DUNGEON_AMBUSH_ARENA_TERRAIN);

    // "facing north picks row three, east row two, south row four,
    // west row one" - the row that facing selects seats the party
    // behind its facing.
    assert_eq!(record.dungeon_room_party_positions_for_seed(0)[0], (5, 6));
    assert_eq!(record.dungeon_room_party_positions_for_seed(1)[0], (4, 5));
    assert_eq!(record.dungeon_room_party_positions_for_seed(2)[0], (5, 4));
    assert_eq!(record.dungeon_room_party_positions_for_seed(3)[0], (6, 5));

    assert_eq!(
        record.dungeon_room_source_x(),
        DUNGEON_AMBUSH_SOURCE_X_NORTH
    );
    assert_eq!(
        record.dungeon_room_source_y(),
        DUNGEON_AMBUSH_SOURCE_Y_NORTH
    );

    // `count` copies of `class * 4 + 0x40` in the first `count`
    // permuted slots, and nothing anywhere else.
    let source_byte = COMBAT_CLASS_BAT * 4 + DUNGEON_ROOM_ORDINARY_SOURCE_FIRST;
    let sources = record.dungeon_room_sources();
    assert_eq!(sources[9], source_byte);
    assert_eq!(sources[1], source_byte);
    assert_eq!(sources[2], source_byte);
    assert_eq!(
        sources.iter().filter(|byte| **byte == source_byte).count(),
        3
    );
    assert_eq!(sources.iter().filter(|byte| **byte != 0).count(), 3);
    assert_eq!(
        dungeon_room_ordinary_setup_class(source_byte),
        Some(COMBAT_CLASS_BAT)
    );
}

/// `dungeon-mode.md §14.1`: the live ambush shuffles all sixteen source
/// indexes with sixteen independent full-range swaps. Dormant terrain combat
/// uses the same range but stops after source index fourteen.
#[test]
fn dungeon_combat_source_shuffles_match_published_draw_order() {
    let mut live = combat_ai_turn_state(8, 5);
    live.prng_state = 0;
    assert_eq!(
        live.dungeon_ambush_source_permutation(),
        [15, 4, 1, 0, 14, 7, 6, 3, 5, 8, 10, 11, 12, 9, 13, 2]
    );
    assert_eq!(live.prng_state, 0x01c0);

    let mut dormant = combat_ai_turn_state(8, 5);
    dormant.prng_state = 0;
    assert_eq!(
        dormant.dormant_terrain_combat_source_permutation(),
        [2, 4, 1, 0, 14, 7, 6, 3, 5, 8, 10, 11, 12, 9, 13, 15]
    );
    assert_eq!(dormant.prng_state, 0x0cf4);
}

/// `dungeon-mode.md §14.1`: "facing south swaps the east pair;
/// facing west swaps the north pair. A facing value outside zero
/// through three leaves both rows untouched."
#[test]
fn dungeon_ambush_source_rows_swap_by_facing() {
    assert_eq!(
        dungeon_ambush_source_rows(0),
        Some((DUNGEON_AMBUSH_SOURCE_X_NORTH, DUNGEON_AMBUSH_SOURCE_Y_NORTH))
    );
    assert_eq!(
        dungeon_ambush_source_rows(1),
        Some((DUNGEON_AMBUSH_SOURCE_X_EAST, DUNGEON_AMBUSH_SOURCE_Y_EAST))
    );
    assert_eq!(
        dungeon_ambush_source_rows(2),
        Some((DUNGEON_AMBUSH_SOURCE_Y_EAST, DUNGEON_AMBUSH_SOURCE_X_EAST))
    );
    assert_eq!(
        dungeon_ambush_source_rows(3),
        Some((DUNGEON_AMBUSH_SOURCE_Y_NORTH, DUNGEON_AMBUSH_SOURCE_X_NORTH))
    );
    assert_eq!(dungeon_ambush_source_rows(4), None);

    let untouched = CombatArenaRecord::synthesise_dungeon_ambush(
        DUNGEON_AMBUSH_ARENA_FLOOR_TILE,
        4,
        COMBAT_CLASS_BAT,
        1,
        [0; DUNGEON_ROOM_SOURCE_COUNT],
    );
    assert_eq!(
        untouched.dungeon_room_source_x(),
        [0; DUNGEON_ROOM_SOURCE_COUNT]
    );
    assert_eq!(
        untouched.dungeon_room_source_y(),
        [0; DUNGEON_ROOM_SOURCE_COUNT]
    );

    // The entry-seed map the ambush party rows are built around is
    // the shared dungeon-room one.
    assert_eq!(dungeon_room_entry_seed_for_direction(Direction::North), 0);
    assert_eq!(dungeon_room_entry_seed_for_direction(Direction::East), 1);
    assert_eq!(dungeon_room_entry_seed_for_direction(Direction::South), 2);
    assert_eq!(dungeon_room_entry_seed_for_direction(Direction::West), 3);
    assert_eq!(dungeon_room_party_position_row(0), 3);
    assert_eq!(dungeon_room_party_position_row(1), 2);
    assert_eq!(dungeon_room_party_position_row(2), 4);
    assert_eq!(dungeon_room_party_position_row(3), 1);
}

/// `catalogs/monster-bestiary.md §2.1`: the companion table is
/// forty-eight entries indexed by class id whose values are class
/// ids. Nothing about the substitution is keyed to the arena.
#[test]
fn combat_class_companion_table_matches_monster_bestiary_section_two_one() {
    assert_eq!(
        COMBAT_CLASS_COMPANION,
        [
            33, 1, 1, 3, 4, 4, 4, 4, 4, 4, 10, 4, 12, 13, 14, 15, 17, 16, 17, 19, 33, 21, 20, 33,
            24, 26, 35, 21, 21, 24, 30, 24, 41, 0, 22, 36, 35, 23, 39, 39, 40, 20, 42, 43, 44, 45,
            20, 38
        ]
    );
    assert_eq!(COMBAT_CLASS_COMPANION.len(), COMBAT_CLASS_COUNT);
    // The three worked examples §5 of `combat.md` names.
    assert_eq!(combat_class_companion(32), Some(41));
    assert_eq!(combat_class_companion(23), Some(33));
    assert_eq!(combat_class_companion(38), Some(39));
    assert_eq!(combat_class_companion(COMBAT_CLASS_COUNT as u8), None);
    let self_companions = COMBAT_CLASS_COMPANION
        .iter()
        .enumerate()
        .filter(|(class, companion)| *class as u8 == **companion)
        .count();
    assert_eq!(self_companions, 18);
}

#[test]
fn terrain_combat_replacement_rolls_only_for_eligible_followers() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.prng_state = 0x1234;
    let mut expected_prng = state.prng_state;
    let expected_first = u5_prng_range_u16(
        &mut expected_prng,
        0,
        u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1),
    ) as u8;
    let expected_second = u5_prng_range_u16(
        &mut expected_prng,
        0,
        u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1),
    ) as u8;

    let rolls = state.terrain_combat_replacement_roll_seeds(8, Some(41));

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

    // `combat.md §5`: the substituted value is the base class's
    // COMPANION CLASS - Orc (32) mixes in Troll (41).
    assert_eq!(
        terrain_combat_class_for_spawn_index(0, 8, 32, Some(41), 0),
        32
    );
    assert_eq!(
        terrain_combat_class_for_spawn_index(1, 8, 32, Some(41), 0),
        41
    );
    assert_eq!(
        terrain_combat_class_for_spawn_index(2, 8, 32, Some(41), 9),
        41
    );
    assert_eq!(
        terrain_combat_class_for_spawn_index(3, 8, 32, Some(41), 0),
        32
    );
    assert_eq!(
        terrain_combat_class_for_spawn_index(1, 8, 32, Some(41), 1),
        32
    );
    assert_eq!(terrain_combat_class_for_spawn_index(1, 8, 32, None, 0), 32);
}

/// `combat.md §5`: the companion substitution is indexed by the base
/// class id and applies identically in every arena.
#[test]
fn terrain_combat_companion_substitution_applies_per_class_not_per_arena() {
    for base_class in 0..COMBAT_CLASS_COUNT as u8 {
        let companion = combat_class_companion(base_class).unwrap();

        assert_eq!(
            terrain_combat_class_for_spawn_index(0, 16, base_class, Some(companion), 0),
            base_class,
            "class {base_class} first spawn must keep the base class"
        );
        for spawn_index in 1..terrain_combat_replacement_threshold(16) {
            assert_eq!(
                terrain_combat_class_for_spawn_index(
                    spawn_index,
                    16,
                    base_class,
                    Some(companion),
                    0,
                ),
                companion,
                "class {base_class} eligible follower spawn {spawn_index} must accept the companion roll"
            );
            assert_eq!(
                terrain_combat_class_for_spawn_index(
                    spawn_index,
                    16,
                    base_class,
                    Some(companion),
                    1,
                ),
                base_class,
                "class {base_class} eligible follower spawn {spawn_index} must reject a nonzero roll"
            );
        }
        assert_eq!(
            terrain_combat_class_for_spawn_index(5, 16, base_class, Some(companion), 0),
            base_class,
            "class {base_class} late spawn must not roll for the companion"
        );
    }
}

#[test]
fn terrain_combat_setup_from_record_copies_record_slices_and_base_class() {
    let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
    let trigger = ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 10,
        y: 20,
        z: -1,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };

    let setup =
        terrain_combat_setup_from_record_at_arena(WorldPlane::Britannia, trigger, 4, &record)
            .unwrap();

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
        type_byte: 0xc0,
        tile: 0xc0,
        x: 10,
        y: 20,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    let setup =
        terrain_combat_setup_from_record_at_arena(WorldPlane::Britannia, trigger, 4, &record)
            .unwrap();

    let instance = terrain_combat_instance_from_setup(&setup, 8, Some(41), &[0, 0, 1], &[]).unwrap();

    assert_eq!(instance.requested_count, 8);
    assert_eq!(instance.placed_count, 8);
    assert_eq!(instance.unplaced_count, 0);
    assert!(
        instance.active_objects[..COMBAT_PARTY_ACTOR_SLOTS]
            .iter()
            .all(|object| object.is_empty())
    );
    assert_eq!(instance.active_objects[6].tile, 0xc0);
    assert_eq!(
        (instance.active_objects[6].x, instance.active_objects[6].y),
        (0, 15)
    );
    assert_eq!(
        instance.active_objects[6].z,
        WorldPlane::Britannia.save_floor()
    );
    assert_eq!(instance.actors[6].owner_target_class, 32);
    assert_eq!(instance.actors[6].active_object_slot, 6);
    assert_eq!((instance.actors[6].x, instance.actors[6].y), (0, 15));
    assert!(combat_actor_is_active_not_dead(instance.actors[6]));

    // `combat.md §5`: "A spawned actor's renderer-facing tile is then
    // derived from whichever class was chosen" - Troll (41) is
    // `41 * 4 + 0x40 = 0xE4`, and the descriptor carries class 41,
    // so tile and descriptor class can no longer disagree.
    assert_eq!(instance.active_objects[7].tile, 0xe4);
    assert_eq!(instance.active_objects[7].type_byte, 0xe4);
    assert_eq!(instance.actors[7].owner_target_class, 41);
    assert_eq!((instance.actors[7].x, instance.actors[7].y), (1, 14));
    assert_eq!(instance.active_objects[8].tile, 0xc0);
    assert_eq!(instance.actors[8].owner_target_class, 32);
}

/// `combat.md §5`: "Because seating happens first and reads its own
/// coordinate table, party seats never depend on the monster count
/// and never consume a placement slot." The coordinates come from
/// the arena record's six party entry X and Y slices
/// (`formats/cbt.md §5`, row 3 columns 11-16 and 17-22).
#[test]
fn terrain_combat_party_seats_come_from_arena_entry_coordinates() {
    let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
    let trigger = ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 10,
        y: 20,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    let setup =
        terrain_combat_setup_from_record_at_arena(WorldPlane::Britannia, trigger, 4, &record)
            .unwrap();
    let positions = terrain_combat_party_entry_positions(&setup);
    assert_eq!(
        positions[0],
        (setup.setup_table_a[0], setup.setup_table_b[0])
    );
    assert_eq!(
        positions[5],
        (setup.setup_table_a[5], setup.setup_table_b[5])
    );

    let mut state = world_state(open_world_grid(), 10, 20);
    state.party[0].class_byte = b'A';
    // `combat.md §5` party descriptor seeding: base-step is the
    // character's dexterity, phase counter thirty-six minus it.
    state.party[0].climb_stat = 22;

    // The seats must be identical no matter how many monsters the
    // count roll produced.
    for requested_count in [1u8, 8, 16] {
        let mut instance =
            terrain_combat_instance_from_setup(&setup, requested_count, None, &[], &[]).unwrap();
        state.populate_combat_party_with_positions(
            &mut instance.active_objects,
            &mut instance.actors,
            0,
            &positions,
        );

        assert_eq!(
            (instance.active_objects[0].x, instance.active_objects[0].y),
            (usize::from(positions[0].0), usize::from(positions[0].1))
        );
        assert_eq!((instance.actors[0].x, instance.actors[0].y), positions[0]);
        assert_eq!(instance.actors[0].owner_target_class, 0);
        assert_eq!(instance.actors[0].active_object_slot, 0);
        assert_eq!(instance.actors[0].base_step, state.party[0].dexterity());
        assert_eq!(
            instance.actors[0].phase_counter,
            COMBAT_PLACEMENT_PHASE_BASE - state.party[0].dexterity()
        );
    }
}

#[test]
fn terrain_combat_instance_reports_unplaced_count_when_placement_slots_end() {
    let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
    let trigger = ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 10,
        y: 20,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    let setup =
        terrain_combat_setup_from_record_at_arena(WorldPlane::Britannia, trigger, 4, &record)
            .unwrap();

    let instance = terrain_combat_instance_from_setup(&setup, 26, None, &[], &[]).unwrap();

    assert_eq!(instance.placed_count, 16);
    assert_eq!(instance.unplaced_count, 10);
    assert_eq!(
        instance.active_objects[6].z,
        WorldPlane::Underworld.save_floor()
    );
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

    let err = terrain_combat_setup_from_record_at_arena(WorldPlane::Britannia, trigger, 2, &record)
        .expect_err("object has no outdoor combat class");

    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn terrain_combat_class_selector_uses_active_object_type_not_tile() {
    let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
    let invalid_type_with_arena_tile = ActiveObject {
        type_byte: 0x10,
        tile: 0x50,
        x: 10,
        y: 20,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };

    assert_eq!(
        outdoor_combat_class_id(invalid_type_with_arena_tile.type_byte),
        None
    );
    let err = terrain_combat_setup_from_record_at_arena(
        WorldPlane::Britannia,
        invalid_type_with_arena_tile,
        2,
        &record,
    )
    .expect_err("tile/frame byte must not select the combat class");
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

    let pirate = ActiveObject {
        type_byte: 0x2f,
        tile: 0x10,
        x: 10,
        y: 20,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    assert_eq!(outdoor_combat_class_id(pirate.type_byte), Some(1));
    assert_eq!(
        outdoor_combat_arena_index(pirate.type_byte, 5, false, 0),
        Some(12)
    );
}

#[test]
fn terrain_combat_local_brit_cbt_records_drive_all_outdoor_arenas_when_present() {
    let game_dir = std::path::Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join(BRIT_CBT_FILE).exists() {
        return;
    }

    let bank = load_brit_cbt(game_dir).unwrap();
    assert_eq!(bank.records.len(), BRIT_CBT_RECORDS);

    for arena_index in 0..BRIT_CBT_RECORDS {
        let record = bank.record(arena_index).unwrap();
        let trigger = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: WorldPlane::Britannia.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        let setup = terrain_combat_setup_from_record_at_arena(
            WorldPlane::Britannia,
            trigger,
            arena_index,
            record,
        )
        .unwrap();

        assert_eq!(setup.arena_index, arena_index);
        assert!(!setup.underworld_variant);
        assert_eq!(setup.terrain, record.terrain_grid());
        assert_eq!(setup.setup_table_a, record.outdoor_setup_table_a());
        assert_eq!(setup.setup_table_b, record.outdoor_setup_table_b());
        assert_eq!(setup.placement_slots.len(), CBT_PLACEMENT_SLOT_COUNT);
        assert_eq!(setup.base_class.map(|class| class.class), Some(32));

        for (slot, placement) in setup.placement_slots.iter().enumerate() {
            assert_eq!(placement.slot, slot);
            assert!(
                usize::from(placement.x) < COMBAT_ARENA_SIDE
                    && usize::from(placement.y) < COMBAT_ARENA_SIDE,
                "arena {arena_index} placement slot {slot} is outside the visible 11x11 arena: ({}, {})",
                placement.x,
                placement.y
            );
        }

        let instance = terrain_combat_instance_from_setup(&setup, 16, None, &[], &[]).unwrap();
        assert_eq!(instance.requested_count, 16);
        assert_eq!(instance.placed_count, 16);
        assert_eq!(instance.unplaced_count, 0);

        for spawn_index in 0..16 {
            let actor_slot = COMBAT_PARTY_ACTOR_SLOTS + spawn_index;
            let placement = setup.placement_slots[spawn_index];
            let object = instance.active_objects[actor_slot];
            let actor = instance.actors[actor_slot];

            assert_eq!(object.tile, 0xc0, "arena {arena_index} spawn {spawn_index}");
            assert_eq!(object.type_byte, 0xc0);
            assert_eq!(object.x, usize::from(placement.x));
            assert_eq!(object.y, usize::from(placement.y));
            assert_eq!(object.z, WorldPlane::Britannia.save_floor());
            assert_eq!(actor.owner_target_class, 32);
            assert_eq!(actor.active_object_slot, actor_slot as u8);
            assert_eq!((actor.x, actor.y), (placement.x, placement.y));
            assert!(combat_actor_is_active_not_dead(actor));
        }
    }
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
    assert!(
        parse_combat_arena_bank(BRIT_CBT_FILE, &brit[..brit.len() - 1], BRIT_CBT_RECORDS).is_err()
    );
    assert!(CombatArenaRecord::from_record_bytes(&record[..COMBAT_ARENA_RECORD_LEN - 1]).is_err());
}

#[test]
fn combat_arena_local_clean_cbt_banks_decode_when_present() {
    let game_dir = std::path::Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join(BRIT_CBT_FILE).exists() || !game_dir.join(DUNGEON_CBT_FILE).exists() {
        return;
    }

    let brit = load_brit_cbt(game_dir).unwrap();
    let dungeon = load_dungeon_cbt(game_dir).unwrap();

    assert_eq!(brit.resource_name, BRIT_CBT_FILE);
    assert_eq!(brit.records.len(), BRIT_CBT_RECORDS);
    assert_eq!(dungeon.resource_name, DUNGEON_CBT_FILE);
    assert_eq!(dungeon.records.len(), DUNGEON_CBT_RECORDS);
    assert!(brit.record(BRIT_CBT_RECORDS).is_none());
    assert!(dungeon.record(DUNGEON_CBT_RECORDS).is_none());

    for bank in [&brit, &dungeon] {
        for record in &bank.records {
            assert_eq!(record.record_bytes().len(), COMBAT_ARENA_RECORD_LEN);
            assert_eq!(record.terrain_grid().len(), COMBAT_ARENA_SIDE);
            assert!(record.row(COMBAT_ARENA_SIDE).is_none());
            assert!(record.metadata(0, COMBAT_ARENA_METADATA_START).is_some());
        }
    }
}

#[test]
fn dungeon_cbt_local_clean_room_source_census_when_present() {
    let game_dir = std::path::Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join(DUNGEON_CBT_FILE).exists() {
        return;
    }

    let dungeon = load_dungeon_cbt(game_dir).unwrap();
    let mut ordinary = 0usize;
    let mut absorbable = 0usize;
    let mut special = 0usize;
    let mut random_special = 0usize;

    for record in &dungeon.records {
        for source in record.dungeon_room_setup_sources() {
            assert!(
                source.x < COMBAT_ARENA_SIDE as u8 && source.y < COMBAT_ARENA_SIDE as u8,
                "DUNGEON.CBT source {} has out-of-arena coordinate ({},{})",
                source.source,
                source.x,
                source.y
            );
            match source.kind {
                DungeonRoomSetupSourceKind::OrdinaryCombatant {
                    setup_class,
                    palette_selector,
                } => {
                    ordinary += 1;
                    assert_eq!(
                        dungeon_room_ordinary_setup_class(source.source),
                        Some(setup_class)
                    );
                    match palette_selector {
                        // `formats/cbt.md §5`: the vermin family is an
                        // ordinary placement whose class is
                        // substituted, so it has no source-derived
                        // sprite of its own.
                        Some(selector) => {
                            random_special += 1;
                            assert_eq!(selector, source.source & 0x03);
                            assert_eq!(setup_class, 43);
                            assert_eq!(dungeon_room_source_sprite(source.source), None);
                        }
                        None => {
                            assert!(dungeon_room_source_sprite(source.source).is_some());
                        }
                    }
                }
                DungeonRoomSetupSourceKind::AbsorbableField => {
                    absorbable += 1;
                    assert!(dungeon_room_absorbable_field_family(source.source));
                    assert_eq!(dungeon_room_source_sprite(source.source), None);
                }
                DungeonRoomSetupSourceKind::SpecialPlacement(special_placement) => {
                    special += 1;
                    assert_eq!(
                        special_placement,
                        DungeonRoomSpecialPlacement::from_setup_id(source.source)
                    );
                    assert_eq!(dungeon_room_source_sprite(source.source), None);
                    assert_ne!(
                        source.source & DUNGEON_ROOM_SPECIAL_SOURCE_MASK,
                        DUNGEON_ROOM_SPECIAL_SOURCE_EC
                    );
                }
            }
        }
    }

    assert!(ordinary > 0);
    assert!(special > 0);
    assert!(absorbable > 0);
    assert!(random_special > 0);
    assert!(ordinary + absorbable + special > DUNGEON_CBT_RECORDS);
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
/// Per u5-spec/catalogs/tile-catalog.md Section 5: "Mountain tiles
/// are rejected by the named foot, horse, and carpet movement
/// queries. Balloon art has no promoted live transport predicate in
/// the analyzed baseline" -- so mountains are impassable for every
/// transport family, with no balloon exception. Foothills are not
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

/// Swamp tiles are walkable on foot (you take poison damage stepping
/// through). `0x04` is "swamp" per the shipped description table.
///
/// The three `water_animation_*` tests that stood here — asserting a
/// three-frame water cycle behind one shared family-wide selector —
/// are deleted. `animation.md §6` (spec HEAD `c00bf63`) withdraws
/// both claims: "**no water, lava, brazier or torch tile animates
/// through this pass at all**", and each id owns its own selector
/// rather than sharing one. `static_tile_animation_*` in chunk 29
/// covers the replacement contract.
#[test]
fn swamp_is_walkable_and_static() {
    assert!(
        is_probe_walkable(0x04),
        "0x04 'swamp' must be walkable on foot"
    );
    for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
        let clock = AnimationClock::at_static_tile_phase(phase);
        assert_eq!(
            clock.resolve_static_tile(0x04),
            0x04,
            "swamp must stay 0x04 across all animation phases"
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

#[test]
fn town_drunkenness_rechecks_each_command_and_scrambles_only_on_even_gate() {
    fn seed_for_gate(expected: u16) -> u16 {
        (0..=u16::MAX)
            .find(|seed| {
                let mut state = *seed;
                u5_prng_range_u16(&mut state, 0, 1) == expected
            })
            .expect("the two-outcome PRNG must produce both values")
    }

    let mut sober_roll = test_state(open_grid(), 15, 15);
    sober_roll.town_drunkenness_counter = 25;
    sober_roll.prng_state = seed_for_gate(0);
    assert_eq!(
        handle_play_key_input(&mut sober_roll, 'q', "", Path::new("")).unwrap(),
        PlayInputDisposition::Quit
    );
    assert_eq!(sober_roll.town_drunkenness_counter, 25);
    assert!(
        !sober_roll
            .message_entries()
            .iter()
            .any(|entry| entry.text == "Hic!")
    );

    let mut scrambled = test_state(open_grid(), 15, 15);
    scrambled.town_drunkenness_counter = 25;
    scrambled.prng_state = seed_for_gate(1);
    assert_eq!(
        handle_play_key_input(&mut scrambled, 'q', "ignored", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(scrambled.town_drunkenness_counter, 24);
    assert_ne!((scrambled.player.x, scrambled.player.y), (15, 15));
    assert!(
        scrambled
            .message_entries()
            .iter()
            .any(|entry| entry.text == "Hic!")
    );
}

#[test]
fn fourth_blue_boar_drink_warning_commits_before_affordability() {
    let mut state = test_state(open_grid(), 15, 15);
    state.active_shop = Some(crate::shop_session::ActiveShopSession::Tavern(
        crate::shop_runtime::TavernState::Menu {
            tavern: Tavern::TheBlueBoarTavern,
            continuation_ready: true,
        },
    ));
    state.tavern_secondary_drink_count = 3;
    state.moral_standing = 5;
    state.gold = 0;

    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    assert_eq!(state.message, "Had enough? (Y/N)");
    assert_eq!(state.town_drunkenness_counter, 0);
    assert_eq!(state.moral_standing, 5);

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert_eq!(state.tavern_secondary_drink_count, 3);
    assert_eq!(state.town_drunkenness_counter, 0);
    assert!(matches!(
        state.active_shop,
        Some(crate::shop_session::ActiveShopSession::Tavern(
            crate::shop_runtime::TavernState::AnythingElse { .. }
        ))
    ));

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    assert_eq!(state.town_drunkenness_counter, 25);
    assert_eq!(state.moral_standing, 4);
    assert!(matches!(
        state.active_shop,
        Some(crate::shop_session::ActiveShopSession::Tavern(
            crate::shop_runtime::TavernState::BlueBoarDrinkList { .. }
        ))
    ));

    handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
    assert_eq!(state.tavern_secondary_drink_count, 3);
    assert_eq!(state.town_drunkenness_counter, 25);
    assert_eq!(state.moral_standing, 4);
}

#[test]
fn non_blue_secondary_drink_charges_per_non_dead_member_and_short_funds_exit() {
    let ctx = crate::shop_runtime::ShopTransactionContext {
        party_gold: 10,
        speaker_intelligence: u8::MAX,
        world_hour: 12,
        party_size: 3,
        living_party_members: 3,
    };
    let mut state = crate::shop_runtime::TavernState::Menu {
        tavern: Tavern::TheWayfarerTavern,
        continuation_ready: false,
    };
    let mut gold = 10;
    let mut food = 50;
    assert_eq!(
        crate::shop_runtime::step_tavern(
            &mut state,
            crate::shop_runtime::TavernInput::Key(b'A'),
            ctx,
            &mut gold,
            &mut food,
        ),
        crate::shop_runtime::TavernOutcome::SecondaryTavernSelected {
            tavern: Tavern::TheWayfarerTavern,
            letter: 'A',
            cost: 3,
        }
    );
    assert_eq!(gold, 7);
    assert_eq!(food, 50);

    let mut refused = crate::shop_runtime::TavernState::Menu {
        tavern: Tavern::TheWayfarerTavern,
        continuation_ready: true,
    };
    gold = 2;
    assert_eq!(
        crate::shop_runtime::step_tavern(
            &mut refused,
            crate::shop_runtime::TavernInput::Key(b'A'),
            ctx,
            &mut gold,
            &mut food,
        ),
        crate::shop_runtime::TavernOutcome::RefusedShortFunds { cost: 3 }
    );
    assert_eq!(gold, 2);
    assert_eq!(refused, crate::shop_runtime::TavernState::Exited);
}

#[test]
fn inert_loaded_door_tracker_never_ticks_or_restores_a_tile() {
    let mut state = test_state(open_grid(), 3, 4);
    state.grid[9 * 32 + 7] = 0x44;
    state.door_tracker = Some(DoorTracker {
        previous_tile: 0,
        x: 7,
        y: 9,
        turns_remaining: 3,
    });

    state.tick_door_tracker();

    assert_eq!(state.grid[9 * 32 + 7], 0x44);
    assert_eq!(
        state.door_tracker,
        Some(DoorTracker {
            previous_tile: 0,
            x: 7,
            y: 9,
            turns_remaining: 3,
        })
    );
}

#[test]
fn tavern_round_table_setting_is_north_first_with_southeast_edge_fallback() {
    let mut north_first = test_state(open_grid(), 10, 10);
    north_first.grid[9 * 32 + 10] = TAVERN_BARE_TABLE_SETTING_TILE;
    north_first.grid[11 * 32 + 10] = TAVERN_BARE_TABLE_SETTING_TILE;
    north_first.visibility_dirty = false;
    assert!(north_first.rewrite_tavern_round_table_setting());
    assert_eq!(
        north_first.grid[9 * 32 + 10],
        TAVERN_NORTH_FOOD_SETTING_TILE
    );
    assert_eq!(
        north_first.grid[11 * 32 + 10],
        TAVERN_BARE_TABLE_SETTING_TILE
    );
    assert!(north_first.visibility_dirty);

    let mut south = test_state(open_grid(), 10, 10);
    south.grid[11 * 32 + 10] = TAVERN_BARE_TABLE_SETTING_TILE;
    assert!(south.rewrite_tavern_round_table_setting());
    assert_eq!(south.grid[11 * 32 + 10], TAVERN_SOUTH_FOOD_SETTING_TILE);

    let mut top_edge = test_state(open_grid(), 10, 0);
    top_edge.grid[TOWN_GRID_BYTES - 1] = TAVERN_BARE_TABLE_SETTING_TILE;
    top_edge.grid[32 + 10] = TAVERN_BARE_TABLE_SETTING_TILE;
    assert!(top_edge.rewrite_tavern_round_table_setting());
    assert_eq!(
        top_edge.grid[TOWN_GRID_BYTES - 1],
        TAVERN_NORTH_FOOD_SETTING_TILE
    );
    assert_eq!(top_edge.grid[32 + 10], TAVERN_BARE_TABLE_SETTING_TILE);

    let mut bottom_edge = test_state(open_grid(), 10, 31);
    bottom_edge.grid[TOWN_GRID_BYTES - 1] = TAVERN_BARE_TABLE_SETTING_TILE;
    assert!(bottom_edge.rewrite_tavern_round_table_setting());
    assert_eq!(
        bottom_edge.grid[TOWN_GRID_BYTES - 1],
        TAVERN_SOUTH_FOOD_SETTING_TILE
    );
}

/// `animation.md §13.1`: "**Negate Time freezes all of it.** While that timed
/// effect is active, the world tick forces the gating byte into a skip state
/// on *every* call, and the spell-effect sweep carries the same test. For the
/// effect's full duration nothing advances: no water rotation, no fire
/// flicker, no fountain, no banner, no clock or bellows, no object animation
/// ..."
#[test]
fn negate_time_freezes_every_tile_animation_clock() {
    let mut grid = open_grid();
    // A waterfall cell, one of the `animation.md §6` families, so a tick that
    // did run would show up in the viewport scan as well as in the counters.
    grid[18 * TOWN_GRID_SIDE + 10] = 0xD4;
    let mut state = test_state(grid, 10, 20);
    state.animation = AnimationClock::at_static_tile_phase(3);
    state.water_scroll = WaterScrollClock::at_phase(7);
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.visibility_dirty = false;

    for _ in 0..40 {
        state.advance_animation_clock();
    }

    assert_eq!(state.animation.frame, 3, "no fountain, banner or clock step");
    assert_eq!(state.water_scroll.phase, 7, "no water rotation");
    assert!(
        !state.visibility_dirty,
        "a frozen pass cannot change the picture, so it must not restage it"
    );

    // `magic.md §8`: the shared `T` tag is what gates it. Clear the tag and
    // the same call advances again, so the freeze is the effect rather than a
    // dead code path.
    state.active_effect_tag = None;
    state.active_effect_counter = 0;
    state.advance_animation_clock();
    assert_eq!(state.animation.frame, 4);
    assert_eq!(state.water_scroll.phase, 8);
}

/// `visibility.md §8`: "unless the Negate Time timed effect is active, select
/// a uniform random entry from the four-value range; while it is active, the
/// selector short-circuits and returns the first entry for every actor."
/// `§8.3`: "The appearance freezes on variant 0 for the whole duration of
/// Negate Time."
#[test]
fn active_object_variant_short_circuits_to_zero_under_negate_time() {
    let mut state = test_state(open_grid(), 10, 20);
    let actor = ActiveObject {
        type_byte: 0x90,
        tile: 0x90,
        x: 10,
        y: 19,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };

    // Sweep the turn counter: without the short-circuit the draw visits more
    // than one entry, so the freeze below is doing real work.
    let mut unfrozen = std::collections::BTreeSet::new();
    for turn in 0..64u64 {
        state.turn = turn;
        unfrozen.insert(state.active_object_render_variant(5, 4, actor));
    }
    assert!(
        unfrozen.len() > 1,
        "the unfrozen selector must not be constant"
    );

    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    for turn in 0..64u64 {
        state.turn = turn;
        assert_eq!(
            state.active_object_render_variant(5, 4, actor),
            0,
            "Negate Time returns the first entry for every actor"
        );
    }
}

/// `timing.md §8.2`: "The shared wait tests the current scene value and
/// performs no world step for values `0x21` through `0x7F` **inclusive**;
/// both the bound and its inclusiveness are exact. ... Implement the gate as
/// a numeric range test on the scene value, **not** as an 'is this dungeon
/// mode' test: the band is a strict superset of the dungeon scenes, and the
/// intro, character-creation and Return-to-View animation states (`0x40`,
/// `0x41`, `0x42`) also lie inside it."
#[test]
fn idle_world_step_suppression_is_the_published_scene_value_band() {
    assert_eq!(IDLE_WORLD_STEP_SUPPRESSED_FIRST_SCENE, 0x21);
    assert_eq!(IDLE_WORLD_STEP_SUPPRESSED_LAST_SCENE, 0x7F);
    for scene in 0u8..=u8::MAX {
        assert_eq!(
            idle_world_step_suppressed_for_scene(scene),
            (0x21..=0x7F).contains(&scene),
            "scene {scene:#04x}"
        );
    }

    // The band is a strict superset of the eight first-person dungeon scenes,
    // which is exactly why an `Area::Dungeon` match cannot stand in for it.
    for scene in SCENE_DUNGEON_NAMED_FIRST..=SCENE_DUNGEON_NAMED_LAST {
        assert!(idle_world_step_suppressed_for_scene(scene));
    }
    for scene in SCENE_INTRO_FIRST..=SCENE_INTRO_LAST {
        assert!(
            idle_world_step_suppressed_for_scene(scene),
            "the intro/chargen/Return-to-View states lie inside the band"
        );
    }
    assert!(!idle_world_step_suppressed_for_scene(SCENE_OVERWORLD));
    for scene in SCENE_TOWN_FAMILY_FIRST..=SCENE_TOWN_FAMILY_LAST {
        assert!(!idle_world_step_suppressed_for_scene(scene));
    }
    assert!(
        !idle_world_step_suppressed_for_scene(SCENE_COMBAT_TEMPORARY),
        "combat sets scene value 0xFF and does run the world step"
    );
}

/// `animation.md §9`: the driver-side animation layer "is **not** reset ...
/// its state lives in the asset buffer for the whole program run", and `§12.1`
/// adds that it "survives scene changes, save loads, and everything else
/// short of reloading the asset". So an area constructor must inherit the
/// running phases rather than start a fresh clock.
#[test]
fn area_constructors_inherit_the_animation_asset_buffer() {
    let dir = debug_game_dir();
    let carried = AnimationAssetBuffer {
        animation: AnimationClock::at_static_tile_phase(5),
        water_scroll: WaterScrollClock::at_phase(9),
    };

    let world = PlayState::load_world_scene(
        &dir,
        WorldPlane::Underworld,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            floor: -1,
            start: Some((10, 20)),
            animation_asset_buffer: carried,
            ..PlayOptions::default()
        },
    )
    .unwrap();
    assert_eq!(world.animation, carried.animation);
    assert_eq!(world.water_scroll, carried.water_scroll);
    assert_eq!(world.animation_asset_buffer(), carried);

    let town_scene = Scene::new(0x11).unwrap();
    let town = PlayState::load_town_scene(
        &dir,
        town_scene,
        PlayOptions {
            target: PlayTarget::Town(town_scene),
            floor: 0,
            start: Some((1, 1)),
            animation_asset_buffer: carried,
            ..PlayOptions::default()
        },
    )
    .unwrap();
    assert_eq!(town.animation_asset_buffer(), carried);

    let dungeon_scene = DungeonScene::new(33).unwrap();
    let dungeon = PlayState::load_dungeon_scene(
        &dir,
        dungeon_scene,
        PlayOptions {
            target: PlayTarget::Dungeon(dungeon_scene),
            floor: 0,
            start: Some((1, 1)),
            animation_asset_buffer: carried,
            ..PlayOptions::default()
        },
    )
    .unwrap();
    assert_eq!(dungeon.animation_asset_buffer(), carried);
}

/// `animation.md §6.1`: "**It is initialised to the identity map at
/// startup** ... The shipped phase counter starts at zero." A program that is
/// only now booting is the one place the buffer legitimately starts fresh.
#[test]
fn animation_asset_buffer_boot_value_is_phase_zero() {
    assert_eq!(AnimationAssetBuffer::AT_BOOT.animation.frame, 0);
    assert_eq!(AnimationAssetBuffer::AT_BOOT.water_scroll.phase, 0);
    assert_eq!(
        AnimationAssetBuffer::default(),
        AnimationAssetBuffer::AT_BOOT
    );
    assert_eq!(
        PlayOptions::default().animation_asset_buffer,
        AnimationAssetBuffer::AT_BOOT
    );
}
