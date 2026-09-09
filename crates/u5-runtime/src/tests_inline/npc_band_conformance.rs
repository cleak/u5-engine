// `formats/saved-gam.md §12` - the saved NPC family band at
// `0x07B4..0x105F`. §12: "An active-object-only writer cannot restore a
// working town cast."

fn npc_band_template(scene: u8) -> Vec<u8> {
    saved_game_seed_bytes(scene, 0, 15, 15)
}

fn npc_band_game_dir(template: Vec<u8>) -> std::path::PathBuf {
    let dir = debug_game_dir();
    fs::write(dir.join("SAVED.GAM"), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    dir
}

fn band_test_npc(slot: usize, type_byte: u8, x: usize, y: usize) -> RuntimeNpc {
    let mut schedule = [0u8; NPC_SCHEDULE_RECORD_LEN];
    for wp in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
        schedule[NPC_SCHEDULE_AI_OFFSET + wp] = 1;
        schedule[NPC_SCHEDULE_X_OFFSET + wp] = (x + wp) as u8;
        schedule[NPC_SCHEDULE_Y_OFFSET + wp] = (y + wp) as u8;
        schedule[NPC_SCHEDULE_Z_OFFSET + wp] = 0;
    }
    for boundary in 0..NPC_SCHEDULE_TIME_BOUNDARY_COUNT {
        schedule[NPC_SCHEDULE_TIME_OFFSET + boundary] = (6 * boundary) as u8;
    }
    RuntimeNpc {
        slot,
        type_byte,
        dialog_id: (slot + 1) as u8,
        schedule,
        state: NPC_STATE_IDLE,
        x,
        y,
        z: 0,
        cached_wp: 1,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    }
}

/// `formats/saved-gam.md §12.1` gives the complete band map. Pin every
/// published file offset so a refactor of the anchored constant chain
/// cannot silently slide a sub-table.
#[test]
fn npc_band_sub_tables_sit_at_their_published_file_offsets() {
    assert_eq!(SAVE_NPC_BAND_OFFSET, 0x07B4);
    assert_eq!(SAVE_NPC_BAND_OPAQUE_HEAD_OFFSET, 0x07B4);
    assert_eq!(SAVE_NPC_BAND_SCHEDULES_OFFSET, 0x07B8);
    assert_eq!(SAVE_NPC_BAND_RUNTIME_OFFSET, 0x09B8);
    assert_eq!(SAVE_NPC_BAND_ROUTES_OFFSET, 0x0BB8);
    assert_eq!(SAVE_NPC_BAND_ROUTE_CURSORS_OFFSET, 0x0FB8);
    assert_eq!(SAVE_NPC_BAND_TYPES_OFFSET, 0x0FF8);
    assert_eq!(SAVE_NPC_BAND_EVENT_KIND_OFFSET, 0x1018);
    assert_eq!(SAVE_NPC_BAND_EVENT_NPC_INDEX_OFFSET, 0x1019);
    assert_eq!(SAVE_NPC_BAND_OPAQUE_TAIL_OFFSET, 0x101A);
    assert_eq!(SAVE_NPC_BAND_STUCK_COUNTERS_OFFSET, 0x101C);
    assert_eq!(SAVE_NPC_BAND_DUNGEON_ARRIVAL_SELECTOR_OFFSET, 0x105C);
    assert_eq!(SAVE_NPC_BAND_DUNGEON_FACING_OFFSET, 0x105D);
    assert_eq!(SAVE_NPC_BAND_DUNGEON_VIEW_STATE_OFFSET, 0x105E);
    assert_eq!(SAVE_NPC_BAND_SHIPWRIGHT_CLASS_OFFSET, 0x105F);
    assert_eq!(SAVE_NPC_BAND_SHIPWRIGHT_CLASS_OFFSET, SAVE_PENDING_VEHICLE_CLASS_OFFSET);
    // §12: "The final 2,220 bytes, `0x07B4..0x105F`". §12.1: "The six NPC
    // tables occupy 2,208 bytes in total ... Six opaque bytes, the
    // two-byte event pair and four final mode-state bytes account for the
    // remaining twelve, giving exactly 2,220 bytes."
    assert_eq!(SAVE_NPC_BAND_LEN, 2_220);
    assert_eq!(SAVE_NPC_BAND_LEN, SAVE_RESERVED_TAIL_LEN);
    let six_tables = SAVE_NPC_BAND_SCHEDULES_LEN
        + SAVE_NPC_BAND_RUNTIME_LEN
        + SAVE_NPC_BAND_ROUTES_LEN
        + SAVE_NPC_BAND_ROUTE_CURSORS_LEN
        + SAVE_NPC_BAND_TYPES_LEN
        + SAVE_NPC_BAND_STUCK_COUNTERS_LEN;
    assert_eq!(six_tables, 2_208);
    assert_eq!(SAVE_NPC_BAND_OFFSET + SAVE_NPC_BAND_LEN, SAVED_GAM_LEN);
}

/// `formats/saved-gam.md §12.2`: "the second [byte of a run] is a
/// direction: `1` east, `2` north, `3` west, `4` south". The engine's own
/// BFS codes (`npc-schedules.md §8.2`) are a different permutation, so the
/// writer has to translate.
#[test]
fn saved_route_directions_are_not_the_engine_bfs_codes() {
    assert_eq!(
        saved_route_direction_from_engine(NPC_PATH_DIR_EAST),
        Some(SAVE_NPC_BAND_ROUTE_DIR_EAST)
    );
    assert_eq!(
        saved_route_direction_from_engine(NPC_PATH_DIR_NORTH),
        Some(SAVE_NPC_BAND_ROUTE_DIR_NORTH)
    );
    assert_eq!(
        saved_route_direction_from_engine(NPC_PATH_DIR_WEST),
        Some(SAVE_NPC_BAND_ROUTE_DIR_WEST)
    );
    assert_eq!(
        saved_route_direction_from_engine(NPC_PATH_DIR_SOUTH),
        Some(SAVE_NPC_BAND_ROUTE_DIR_SOUTH)
    );
    // The two encodings genuinely differ; an engine that wrote its own
    // codes would send east where the original walks west.
    assert_ne!(NPC_PATH_DIR_EAST, SAVE_NPC_BAND_ROUTE_DIR_EAST);
    assert_ne!(NPC_PATH_DIR_NORTH, SAVE_NPC_BAND_ROUTE_DIR_NORTH);
    for code in [
        NPC_PATH_DIR_WEST,
        NPC_PATH_DIR_SOUTH,
        NPC_PATH_DIR_EAST,
        NPC_PATH_DIR_NORTH,
    ] {
        let saved = saved_route_direction_from_engine(code).unwrap();
        assert_eq!(engine_route_direction_from_saved(saved), Some(code));
    }
    assert_eq!(saved_route_direction_from_engine(0), None);
    assert_eq!(saved_route_direction_from_engine(5), None);
}

/// `formats/saved-gam.md §12.2`: "A route buffer holds up to sixteen
/// two-byte runs. The first byte of a run is its remaining step count."
#[test]
fn route_runs_are_run_length_pairs_and_an_empty_queue_is_the_inactive_cursor() {
    let (buffer, cursor) = encode_route_buffer(&[]).unwrap();
    assert_eq!(cursor, SAVE_NPC_BAND_ROUTE_CURSOR_INACTIVE);
    assert_eq!(buffer[0], 0, "no queued step means a zero count");
    assert!(decode_route_buffer(&buffer, cursor).is_empty());

    let queue = vec![
        NPC_PATH_DIR_EAST,
        NPC_PATH_DIR_EAST,
        NPC_PATH_DIR_EAST,
        NPC_PATH_DIR_NORTH,
        NPC_PATH_DIR_WEST,
        NPC_PATH_DIR_WEST,
    ];
    let (buffer, cursor) = encode_route_buffer(&queue).unwrap();
    assert_eq!(cursor, 0);
    assert_eq!(buffer[0], 3);
    assert_eq!(buffer[1], SAVE_NPC_BAND_ROUTE_DIR_EAST);
    assert_eq!(buffer[2], 1);
    assert_eq!(buffer[3], SAVE_NPC_BAND_ROUTE_DIR_NORTH);
    assert_eq!(buffer[4], 2);
    assert_eq!(buffer[5], SAVE_NPC_BAND_ROUTE_DIR_WEST);
    // "Reaching the end of the buffer or a zero next count makes the
    // cursor inactive."
    assert_eq!(buffer[6], 0);
    assert_eq!(decode_route_buffer(&buffer, cursor), queue);
}

/// `formats/saved-gam.md §12.2`: "An accepted queued step decrements the
/// run's count ... Preserve partially consumed counts, the cursor, and the
/// whole buffer." The engine holds a flat queue plus a read index, so a
/// partially consumed queue must save as the *remaining* count.
#[test]
fn a_partially_consumed_queue_saves_its_remaining_count() {
    let mut npc = band_test_npc(4, 0x54, 6, 7);
    npc.move_queue = vec![
        NPC_PATH_DIR_SOUTH,
        NPC_PATH_DIR_SOUTH,
        NPC_PATH_DIR_SOUTH,
        NPC_PATH_DIR_SOUTH,
        NPC_PATH_DIR_EAST,
    ];
    npc.move_queue_pos = 3;
    let mut save = npc_band_template(0x11);
    write_npc_band(&mut save, std::slice::from_ref(&npc), 1).unwrap();

    let route = band_route_offset(4);
    assert_eq!(save[route], 1, "one south step is still queued, not four");
    assert_eq!(save[route + 1], SAVE_NPC_BAND_ROUTE_DIR_SOUTH);
    assert_eq!(save[route + 2], 1);
    assert_eq!(save[route + 3], SAVE_NPC_BAND_ROUTE_DIR_EAST);
    assert_eq!(
        u16::from_le_bytes([
            save[band_route_cursor_offset(4)],
            save[band_route_cursor_offset(4) + 1],
        ]),
        0
    );

    let restored = read_npc_band(&save);
    assert_eq!(restored.len(), 1);
    assert_eq!(
        restored[0].move_queue,
        vec![NPC_PATH_DIR_SOUTH, NPC_PATH_DIR_EAST],
        "the same two steps remain to walk"
    );
    assert_eq!(restored[0].move_queue_pos, 0);
}

/// `formats/saved-gam.md §12.1`: the runtime record's "eight words",
/// including "a basement floor byte `0xFF` is stored as word `0x00FF`",
/// and §12.1 row 10's stuck counters.
#[test]
fn the_band_round_trips_positions_routes_and_stuck_counters() {
    let mut merchant = band_test_npc(3, 0x54, 12, 9);
    merchant.state = 3;
    merchant.dialog_id = 0x07;
    merchant.cached_wp = 2;
    merchant.stuck_counter = 200;
    merchant.move_queue = vec![NPC_PATH_DIR_NORTH, NPC_PATH_DIR_NORTH, NPC_PATH_DIR_WEST];
    merchant.active_object = Some(2);

    let mut basement_guard = band_test_npc(9, 0x70, 4, 5);
    // §12.1: "Initialization zero-extends the source byte: a basement
    // floor byte `0xFF` is stored as word `0x00FF`."
    basement_guard.z = 0xFF;
    basement_guard.stuck_counter = 3;
    basement_guard.cached_wp = 0;

    let npcs = vec![merchant.clone(), basement_guard.clone()];
    let mut save = npc_band_template(0x11);
    write_npc_band(&mut save, &npcs, 8).unwrap();

    let runtime = band_runtime_offset(9) + SAVE_NPC_BAND_RUNTIME_Z_OFFSET;
    assert_eq!(
        u16::from_le_bytes([save[runtime], save[runtime + 1]]),
        0x00FF,
        "the basement floor byte zero-extends, it does not sign-extend"
    );
    assert_eq!(save[band_type_offset(3)], 0x54);
    assert_eq!(save[band_type_offset(9)], 0x70);
    assert_eq!(
        u16::from_le_bytes([
            save[band_stuck_counter_offset(3)],
            save[band_stuck_counter_offset(3) + 1],
        ]),
        200
    );
    // §12.1 schedules row: "The saved copy is the **live** schedule."
    assert_eq!(
        &save[band_schedule_offset(3)..band_schedule_offset(3) + NPC_SCHEDULE_RECORD_LEN],
        &merchant.schedule[..]
    );

    let restored = read_npc_band(&save);
    assert_eq!(restored, npcs);
}

/// `formats/saved-gam.md §12.1`: "slot zero is reserved ... Preserve slot
/// zero and inactive slots when round-tripping."
#[test]
fn slot_zero_and_inactive_slots_survive_the_write_byte_for_byte() {
    let mut template = npc_band_template(0x11);
    // Junk in slot zero and in an inactive slot 17: the type byte stays
    // zero, so both are "inactive" and neither may be touched.
    for (index, offset) in [
        band_schedule_offset(0),
        band_runtime_offset(0),
        band_route_offset(0),
        band_route_cursor_offset(0),
        band_stuck_counter_offset(0),
        band_schedule_offset(17),
        band_runtime_offset(17),
        band_route_offset(17),
        band_route_cursor_offset(17),
        band_stuck_counter_offset(17),
    ]
    .into_iter()
    .enumerate()
    {
        template[offset] = 0xA0 + index as u8;
        template[offset + 1] = 0xB0 + index as u8;
    }
    // The opaque bytes, the event pair and the dungeon-mode bytes have no
    // live engine source and must pass through untouched.
    template[SAVE_NPC_BAND_OPAQUE_HEAD_OFFSET..SAVE_NPC_BAND_OPAQUE_HEAD_OFFSET + 4]
        .copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);
    template[SAVE_NPC_BAND_EVENT_KIND_OFFSET] = 0x74;
    template[SAVE_NPC_BAND_EVENT_NPC_INDEX_OFFSET] = 6;
    template[SAVE_NPC_BAND_OPAQUE_TAIL_OFFSET] = 0x55;
    template[SAVE_NPC_BAND_OPAQUE_TAIL_OFFSET + 1] = 0x66;
    template[SAVE_NPC_BAND_DUNGEON_ARRIVAL_SELECTOR_OFFSET] = 0x77;
    template[SAVE_NPC_BAND_DUNGEON_FACING_OFFSET] = 0x88;
    template[SAVE_NPC_BAND_DUNGEON_VIEW_STATE_OFFSET] = 0x99;

    let mut save = template.clone();
    write_npc_band(&mut save, &[band_test_npc(3, 0x54, 12, 9)], 4).unwrap();

    for slot in [0usize, 17] {
        for (offset, len) in [
            (band_schedule_offset(slot), SAVE_NPC_BAND_SCHEDULE_RECORD_LEN),
            (band_runtime_offset(slot), SAVE_NPC_BAND_RUNTIME_RECORD_LEN),
            (band_route_offset(slot), SAVE_NPC_BAND_ROUTE_BUFFER_LEN),
            (band_route_cursor_offset(slot), 2),
            (band_type_offset(slot), 1),
            (band_stuck_counter_offset(slot), 2),
        ] {
            assert_eq!(
                &save[offset..offset + len],
                &template[offset..offset + len],
                "slot {slot} sub-table at {offset:#x} must round-trip untouched"
            );
        }
    }
    assert_eq!(
        &save[SAVE_NPC_BAND_OPAQUE_HEAD_OFFSET..SAVE_NPC_BAND_OPAQUE_HEAD_OFFSET + 4],
        &[0x11, 0x22, 0x33, 0x44]
    );
    assert_eq!(save[SAVE_NPC_BAND_EVENT_KIND_OFFSET], 0x74);
    assert_eq!(save[SAVE_NPC_BAND_EVENT_NPC_INDEX_OFFSET], 6);
    assert_eq!(save[SAVE_NPC_BAND_OPAQUE_TAIL_OFFSET], 0x55);
    assert_eq!(save[SAVE_NPC_BAND_OPAQUE_TAIL_OFFSET + 1], 0x66);
    assert_eq!(save[SAVE_NPC_BAND_DUNGEON_ARRIVAL_SELECTOR_OFFSET], 0x77);
    assert_eq!(save[SAVE_NPC_BAND_DUNGEON_FACING_OFFSET], 0x88);
    assert_eq!(save[SAVE_NPC_BAND_DUNGEON_VIEW_STATE_OFFSET], 0x99);
}

/// `formats/saved-gam.md §12.3`: the band is "**per location, not per
/// floor**", so a slot the template marks occupied but this location's
/// live cast does not hold belongs to a previously saved location. Leaving
/// its type byte set would resurrect that cast here, because "the per-tick
/// NPC walker skips every roster slot whose type byte is zero" - and would
/// not skip this one. `formats/npc.md §6` licenses leaving the record
/// residue: "The corresponding schedule record may still hold non-zero
/// bytes (residue from authoring or from save-game state), but the engine
/// does not read them."
#[test]
fn a_stale_occupied_slot_from_another_location_is_retired() {
    let mut template = npc_band_template(0x11);
    template[band_type_offset(11)] = 0x90;
    template[band_route_offset(11)] = 4;
    template[band_route_offset(11) + 1] = SAVE_NPC_BAND_ROUTE_DIR_EAST;
    template[band_route_cursor_offset(11)] = 0;
    template[band_stuck_counter_offset(11)] = 9;
    template[band_runtime_offset(11) + SAVE_NPC_BAND_RUNTIME_STATE_OFFSET] = 3;

    let mut save = template;
    write_npc_band(&mut save, &[band_test_npc(3, 0x54, 12, 9)], 4).unwrap();

    assert_eq!(save[band_type_offset(11)], SAVE_NPC_BAND_TYPE_EMPTY);
    // §12.2 fresh-slot shape: "clears only the first route byte, sets the
    // cursor to `0xFFFF` and the stuck counter to zero".
    assert_eq!(save[band_route_offset(11)], 0);
    assert_eq!(
        u16::from_le_bytes([
            save[band_route_cursor_offset(11)],
            save[band_route_cursor_offset(11) + 1],
        ]),
        SAVE_NPC_BAND_ROUTE_CURSOR_INACTIVE
    );
    assert_eq!(
        u16::from_le_bytes([
            save[band_stuck_counter_offset(11)],
            save[band_stuck_counter_offset(11) + 1],
        ]),
        0
    );
    assert!(read_npc_band(&save).iter().all(|npc| npc.slot != 11));
}

/// End-to-end: a town save carries the whole live cast, and §12.3's
/// outside-band join holds - "The linked-object words refer to
/// `0x06B4..0x07B3`, whose records must agree with the saved NPC positions
/// and links."
#[test]
fn a_town_save_writes_the_live_cast_and_keeps_the_linked_object_join() {
    let dir = npc_band_game_dir(npc_band_template(0x11));
    let mut state = test_state(open_grid(), 15, 15);
    state.npcs = vec![
        band_test_npc(3, 0x54, 12, 9),
        band_test_npc(5, 0x70, 20, 4),
    ];
    state.npcs[0].stuck_counter = 2;
    state.npcs[0].move_queue = vec![NPC_PATH_DIR_WEST, NPC_PATH_DIR_WEST, NPC_PATH_DIR_NORTH];
    state.relink_npc_objects();
    assert!(state.npcs.iter().all(|npc| npc.active_object.is_some()));

    state.write_save_files(&dir).unwrap();
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();

    let restored = read_npc_band(&saved);
    assert_eq!(restored.len(), 2);
    for (npc, source) in restored.iter().zip(state.npcs.iter()) {
        assert_eq!(npc.slot, source.slot);
        assert_eq!(npc.type_byte, source.type_byte);
        assert_eq!(npc.dialog_id, source.dialog_id);
        assert_eq!(npc.schedule, source.schedule);
        assert_eq!((npc.x, npc.y, npc.z), (source.x, source.y, source.z));
        assert_eq!(npc.stuck_counter, source.stuck_counter);
        assert_eq!(npc.move_queue, source.move_queue);
        assert_eq!(npc.active_object, source.active_object);
        // §12.3: the linked record must agree with the saved position.
        let record = SAVE_ACTIVE_OBJECTS_OFFSET + npc.active_object.unwrap() * OOL_RECORD_LEN;
        assert_ne!(saved[record], 0, "a linked record may not be an empty slot");
        assert_eq!(usize::from(saved[record + 2]), npc.x);
        assert_eq!(usize::from(saved[record + 3]), npc.y);
        assert_eq!(saved[record + 4], npc.z);
        // "It is an object-table index, not the NPC's roster index."
        assert_ne!(npc.active_object, Some(0), "slot zero is the player");
    }
    let _ = fs::remove_dir_all(dir);
}

/// `formats/saved-gam.md §12.3`: only "A town-family scene (`1..32` at
/// file `0x02ED`)" reaches the preserving mode the band describes. A world
/// save must leave every band byte at its template value.
#[test]
fn a_world_save_leaves_the_npc_band_untouched() {
    assert!(scene_byte_is_town_family(1));
    assert!(scene_byte_is_town_family(32));
    assert!(!scene_byte_is_town_family(0), "scene 0 is the overworld");
    assert!(!scene_byte_is_town_family(33), "scenes above 32 are dungeons");

    let mut template = npc_band_template(0);
    template[SAVE_Z_OFFSET] = 0xff;
    for slot in 0..SAVE_NPC_BAND_SLOT_COUNT {
        template[band_type_offset(slot)] = 0x40 + slot as u8;
        template[band_route_offset(slot)] = 7;
        template[band_stuck_counter_offset(slot)] = 5;
    }
    let dir = npc_band_game_dir(template.clone());

    let mut state = world_state(open_world_grid(), 10, 20);
    state.npcs = vec![band_test_npc(3, 0x54, 12, 9)];
    state.write_save_files(&dir).unwrap();
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();

    // `0x105F` is the shipwright byte, which §9.3 owns and the writer sets
    // in every scene; the rest of the band is untouched.
    assert_eq!(
        &saved[SAVE_NPC_BAND_OFFSET..SAVE_NPC_BAND_SHIPWRIGHT_CLASS_OFFSET],
        &template[SAVE_NPC_BAND_OFFSET..SAVE_NPC_BAND_SHIPWRIGHT_CLASS_OFFSET],
        "a world save must not rewrite the town NPC band"
    );
    let _ = fs::remove_dir_all(dir);
}
