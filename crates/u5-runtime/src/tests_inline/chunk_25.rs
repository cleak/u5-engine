#[test]
fn ool_slot_class_groups_public_allocator_bands() {
    assert_eq!(ool_slot_class(0x00), OolSlotClass::Empty);
    assert_eq!(ool_slot_class(0x01), OolSlotClass::Scenery);
    assert_eq!(ool_slot_class(0x0F), OolSlotClass::Scenery);
    assert_eq!(ool_slot_class(0x10), OolSlotClass::DoorFixture);
    assert_eq!(ool_slot_class(0x11), OolSlotClass::DoorFixture);
    assert_eq!(ool_slot_class(0x12), OolSlotClass::NpcOrVehicleProtectedBand);
    assert_eq!(ool_slot_class(0x2F), OolSlotClass::NpcOrVehicleProtectedBand);
    assert_eq!(ool_slot_class(0x30), OolSlotClass::MidrangeObject);
    assert_eq!(ool_slot_class(0x7F), OolSlotClass::MidrangeObject);
    assert_eq!(ool_slot_class(0x80), OolSlotClass::DynamicActor);
    assert_eq!(ool_slot_class(0xB4), OolSlotClass::DynamicActor);
    assert_eq!(ool_slot_class(0xB5), OolSlotClass::ProtectedType);
    assert_eq!(ool_slot_class(0xFF), OolSlotClass::DynamicActor);
}

#[test]
fn ool_plane_audit_counts_roles_classes_and_empty_payloads() {
    let mut bytes = vec![0u8; OOL_PLANE_LEN];
    bytes[ACTIVE_OBJECT_FIELD_TYPE] = 0xFC;
    bytes[ACTIVE_OBJECT_FIELD_TILE] = 0xFC;
    bytes[ACTIVE_OBJECT_FIELD_Z] = OOL_NO_Z_SENTINEL;
    bytes[ACTIVE_OBJECT_FIELD_PHASE] = ANIMATION_PHASE_STEADY_NIBBLE;

    let empty_payload_slot = OOL_RECORD_LEN;
    bytes[empty_payload_slot + ACTIVE_OBJECT_FIELD_DEP1] = 0x44;

    let scenery_slot = OOL_RECORD_LEN * 2;
    bytes[scenery_slot + ACTIVE_OBJECT_FIELD_TYPE] = 0x02;
    bytes[scenery_slot + ACTIVE_OBJECT_FIELD_TILE] = 0x02;
    bytes[scenery_slot + ACTIVE_OBJECT_FIELD_PHASE] = 0x03;

    let protected_slot = OOL_RECORD_LEN * 31;
    bytes[protected_slot + ACTIVE_OBJECT_FIELD_TYPE] = ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE;
    bytes[protected_slot + ACTIVE_OBJECT_FIELD_TILE] = ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE;
    bytes[protected_slot + ACTIVE_OBJECT_FIELD_PHASE] = 0x10;

    let audit = audit_ool_plane_bytes(OolAuditSource::BritOol, &bytes).unwrap();
    assert_eq!(audit.total_slots, OOL_SLOTS);
    assert_eq!(audit.non_empty_slots, 3);
    assert_eq!(audit.empty_payload_slots, 1);
    assert_eq!(audit.empty_zeroed_slots, OOL_SLOTS - 4);
    assert_eq!(audit.z_sentinel_slots, 1);
    assert_eq!(audit.type_equals_tile_slots, 3);
    assert_eq!(audit.high_direction_nibble_slots, 1);
    assert_eq!(audit.role_counts[0], 1);
    assert_eq!(audit.role_counts[1], ACTIVE_OBJECT_ORDINARY_LAST);
    assert_eq!(audit.role_counts[2], OOL_SLOTS - ACTIVE_OBJECT_ORDINARY_LAST - 1);
    assert_eq!(audit.class_counts[OolSlotClass::Empty.index()], OOL_SLOTS - 3);
    assert_eq!(audit.class_counts[OolSlotClass::Scenery.index()], 1);
    assert_eq!(audit.class_counts[OolSlotClass::DynamicActor.index()], 1);
    assert_eq!(audit.class_counts[OolSlotClass::ProtectedType.index()], 1);
    assert_eq!(audit.phase_counts[OolPhaseClass::Steady.index()], 1);
    assert_eq!(audit.phase_counts[OolPhaseClass::Decrementing.index()], 1);
    assert_eq!(audit.phase_counts[OolPhaseClass::AiEligible.index()], OOL_SLOTS - 2);
}

#[test]
fn synthetic_ool_audit_reads_all_roles_without_raw_report_rows() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("u5-ool-audit-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let mut saved_gam = vec![0u8; SAVED_GAM_LEN];
    let mut live = vec![0u8; OOL_PLANE_LEN];
    write_active_object_record(
        &mut live,
        0,
        ActiveObject {
            type_byte: 0xFC,
            tile: 0xFC,
            x: 10,
            y: 20,
            z: OOL_NO_Z_SENTINEL as i8,
            aux1: 1,
            phase: ANIMATION_PHASE_STEADY_NIBBLE,
            aux3: 2,
        },
    )
    .unwrap();
    saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN]
        .copy_from_slice(&live);

    let mut brit = vec![0u8; OOL_PLANE_LEN];
    write_active_object_record(
        &mut brit,
        2,
        ActiveObject {
            type_byte: 0x31,
            tile: 0x31,
            x: 12,
            y: 24,
            z: OOL_NO_Z_SENTINEL as i8,
            aux1: 0,
            phase: 0x03,
            aux3: 0,
        },
    )
    .unwrap();
    let mut under = vec![0u8; OOL_PLANE_LEN];
    write_active_object_record(
        &mut under,
        31,
        ActiveObject {
            type_byte: ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE,
            tile: ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE,
            x: 42,
            y: 24,
            z: OOL_NO_Z_SENTINEL as i8,
            aux1: 0,
            phase: 0x10,
            aux3: 0,
        },
    )
    .unwrap();

    let mut saved_ool = Vec::with_capacity(SAVED_OOL_LEN);
    saved_ool.extend_from_slice(&brit);
    saved_ool.extend_from_slice(&under);

    fs::write(dir.join(SAVED_GAM_FILENAME), saved_gam).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), saved_ool).unwrap();
    fs::write(dir.join(BRIT_OOL_FILENAME), &brit).unwrap();
    fs::write(dir.join(UNDER_OOL_FILENAME), &under).unwrap();
    fs::write(dir.join(INIT_OOL_FILENAME), &brit).unwrap();

    let report = audit_ool_files(&dir).unwrap();
    assert_eq!(report.planes.len(), OOL_AUDIT_SOURCE_COUNT);
    assert_eq!(report.total_slots, OOL_AUDIT_SOURCE_COUNT * OOL_SLOTS);
    assert_eq!(report.non_empty_slots, 6);
    assert_eq!(report.empty_payload_slots, 0);
    assert!(report.saved_britannia_matches_mirror);
    assert!(report.saved_underworld_matches_mirror);
    assert!(report.init_matches_britannia_seed);
    assert_eq!(report.source_counts, [1; OOL_AUDIT_SOURCE_COUNT]);
    assert!(report.class_counts[OolSlotClass::MidrangeObject.index()] > 0);
    assert!(report.class_counts[OolSlotClass::ProtectedType.index()] > 0);

    let text = ool_audit_report_text(&report);
    assert!(text.contains("planes=6"));
    assert!(text.contains("slots=192"));
    assert!(text.contains("hash="));
    assert!(text.contains("slot_classes:"));
    assert!(!text.contains("[["));
    assert!(!text.contains("slot 31"));
    assert!(!text.contains("x=42"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn shipped_ool_audit_covers_clean_overlay_files_when_assets_present() {
    let game_dir = Path::new(DEFAULT_GAME_DIR);
    if ![
        SAVED_GAM_FILENAME,
        SAVED_OOL_FILENAME,
        BRIT_OOL_FILENAME,
        UNDER_OOL_FILENAME,
        INIT_OOL_FILENAME,
    ]
    .iter()
    .all(|name| game_dir.join(name).exists())
    {
        return;
    }

    let report = audit_ool_files(game_dir).unwrap();
    assert_eq!(report.planes.len(), OOL_AUDIT_SOURCE_COUNT);
    assert_eq!(report.total_slots, OOL_AUDIT_SOURCE_COUNT * OOL_SLOTS);
    assert_eq!(report.source_counts, [1; OOL_AUDIT_SOURCE_COUNT]);
    assert_eq!(
        report.class_counts.iter().sum::<usize>(),
        report.total_slots
    );
    assert_eq!(
        report
            .planes
            .iter()
            .map(|plane| plane.non_empty_slots)
            .sum::<usize>(),
        report.non_empty_slots
    );
    assert!(report.non_empty_slots > 0);
    assert!(report.class_counts[OolSlotClass::Empty.index()] > 0);

    for plane in &report.planes {
        assert_eq!(plane.total_slots, OOL_SLOTS);
        assert_eq!(plane.role_counts.iter().sum::<usize>(), OOL_SLOTS);
        assert_eq!(plane.class_counts.iter().sum::<usize>(), OOL_SLOTS);
        assert_eq!(plane.phase_counts.iter().sum::<usize>(), OOL_SLOTS);
        assert_eq!(plane.tile_class_counts.iter().sum::<usize>(), OOL_SLOTS);
    }

    let text = ool_audit_report_text(&report);
    assert!(text.contains("planes=6"));
    assert!(text.contains("sources:"));
    assert!(!text.contains("[["));
    assert!(!text.contains("slot 0"));
    assert!(!text.contains("x="));
}
