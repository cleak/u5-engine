#[test]
fn location_cell_owner_classifies_authored_markers_before_visual_classes() {
    assert_eq!(
        classify_location_cell_owner(TOWN_TILE_NPC_START_A),
        LocationCellOwner::NpcStartMarker
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_TILE_NPC_START_B),
        LocationCellOwner::NpcStartMarker
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_TILE_SPAWN_ASTERISK),
        LocationCellOwner::SpawnMarker
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_TILE_DASH_MARKER),
        LocationCellOwner::CosmeticDashMarker
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_TILE_PERIOD_MARKER),
        LocationCellOwner::CosmeticPeriodMarker
    );
    assert_eq!(
        classify_location_cell_owner(NPC_FLOOR_LINK_TILE_C8),
        LocationCellOwner::FloorLinkMarker
    );
    assert_eq!(
        classify_location_cell_owner(NPC_FLOOR_LINK_TILE_C9),
        LocationCellOwner::FloorLinkMarker
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_DAWN_DUSK_GATE_MARKER_TILE),
        LocationCellOwner::DawnDuskGateMarker
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_EXIT_THRESHOLD_TILE),
        LocationCellOwner::TownExit
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_STAIR_TILE_FIRST),
        LocationCellOwner::WalkOnStair
    );
    assert_eq!(
        classify_location_cell_owner(0x50),
        LocationCellOwner::ClimbTransition
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_CHAIR_TILE),
        LocationCellOwner::ChairTrigger
    );
    assert_eq!(
        classify_location_cell_owner(TOWN_POISON_GAS_LIVE_TILE),
        LocationCellOwner::PoisonGas
    );
    assert_eq!(
        classify_location_cell_owner(0xAB),
        LocationCellOwner::SearchInspectable
    );
}

#[test]
fn location_audit_report_text_is_aggregate_only() {
    let mut report = LocationAuditReport {
        physical_pages: Vec::new(),
        logical_floors: Vec::new(),
        total_cells: 0,
        content_hash: 0x1234,
        owner_counts: [0; LOCATION_AUDIT_OWNER_COUNT],
        tile_class_counts: [0; LOCATION_AUDIT_TILE_CLASS_COUNT],
        view_class_counts: [0; LOCATION_AUDIT_VIEW_CLASS_COUNT],
        npc_path_open_count: 0,
        foot_walkable_count: 0,
        dawn_dusk_bottom_row_count: 0,
        dawn_dusk_unexpected_pair_count: 0,
    };
    report.owner_counts[LocationCellOwner::NpcStartMarker.index()] = 2;
    report.owner_counts[LocationCellOwner::Door.index()] = 3;
    report.tile_class_counts[3] = 5;
    report.view_class_counts[2] = 7;
    report.npc_path_open_count = 11;
    report.foot_walkable_count = 13;

    let text = location_audit_report_text(&report);
    assert!(text.contains("physical_pages=0"));
    assert!(text.contains("npc-start-marker=2"));
    assert!(text.contains("door=3"));
    assert!(text.contains("path=5"));
    assert!(text.contains("0x02=7"));
    assert!(text.contains("npc_path_open=11"));
    assert!(text.contains("foot_walkable=13"));
    assert!(!text.contains("32x32"));
    assert!(!text.contains("[["));
    assert!(!text.contains("0xAB"));
}

#[test]
fn synthetic_location_dat_audit_reads_all_four_families_without_raw_report_rows() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "u5-location-audit-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let mut bytes = vec![0u8; LOCATION_DAT_FILE_LEN];
    bytes[0] = TOWN_TILE_NPC_START_A;
    bytes[1] = TOWN_TILE_SPAWN_ASTERISK;
    bytes[2] = NPC_FLOOR_LINK_TILE_C8;
    bytes[3] = TOWN_DAWN_DUSK_GATE_MARKER_TILE;
    bytes[TOWN_GRID_SIDE + 3] = TOWN_DAWN_DUSK_GATE_OPEN_TILE;
    bytes[4] = TOWN_EXIT_THRESHOLD_TILE;
    bytes[5] = TOWN_STAIR_TILE_FIRST;
    bytes[6] = 0x50;
    bytes[7] = TOWN_CHAIR_TILE;
    bytes[8] = TOWN_POISON_GAS_LIVE_TILE;
    bytes[9] = 0xAB;

    for name in ["TOWNE.DAT", "DWELLING.DAT", "CASTLE.DAT", "KEEP.DAT"] {
        fs::write(dir.join(name), &bytes).unwrap();
    }

    let report = audit_location_dat_files(&dir).unwrap();
    assert_eq!(report.physical_pages.len(), 64);
    assert_eq!(report.total_cells, 64 * TOWN_GRID_BYTES);
    assert_eq!(report.owner_counts.iter().sum::<usize>(), report.total_cells);
    assert_eq!(
        report.tile_class_counts.iter().sum::<usize>(),
        report.total_cells
    );
    assert_eq!(
        report.view_class_counts.iter().sum::<usize>(),
        report.total_cells
    );
    assert!(report.npc_path_open_count > 0);
    assert!(report.foot_walkable_count > 0);
    assert_eq!(report.dawn_dusk_bottom_row_count, 0);
    assert_eq!(report.dawn_dusk_unexpected_pair_count, 0);
    assert!(report.owner_counts[LocationCellOwner::NpcStartMarker.index()] > 0);
    assert!(report.owner_counts[LocationCellOwner::FloorLinkMarker.index()] > 0);
    assert!(report.owner_counts[LocationCellOwner::DawnDuskGateMarker.index()] > 0);

    let text = location_audit_report_text(&report);
    assert!(text.contains("physical_pages=64"));
    assert!(text.contains("cells=65536"));
    assert!(text.contains("hash="));
    assert!(text.contains("tile_class_counts:"));
    assert!(text.contains("view_class_counts:"));
    assert!(text.contains("movement_counts"));
    assert!(!text.contains("TOWNE.DAT row"));
    assert!(!text.contains("[["));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn shipped_location_dat_audit_covers_authored_cell_facets_when_assets_present() {
    let game_dir = Path::new(DEFAULT_GAME_DIR);
    if ![
        "TOWNE.DAT",
        "DWELLING.DAT",
        "CASTLE.DAT",
        "KEEP.DAT",
    ]
    .iter()
    .all(|name| game_dir.join(name).exists())
    {
        return;
    }

    let report = audit_location_dat_files(game_dir).unwrap();
    assert_eq!(report.physical_pages.len(), 64);
    assert_eq!(report.total_cells, 64 * TOWN_GRID_BYTES);
    assert_eq!(report.owner_counts.iter().sum::<usize>(), report.total_cells);
    assert_eq!(
        report.tile_class_counts.iter().sum::<usize>(),
        report.total_cells
    );
    assert_eq!(
        report.view_class_counts.iter().sum::<usize>(),
        report.total_cells
    );
    assert!(report.npc_path_open_count > 0);
    assert!(report.foot_walkable_count > 0);
    assert!(report.logical_floors.len() >= 80);
    assert_eq!(report.dawn_dusk_bottom_row_count, 0);
    assert_eq!(report.dawn_dusk_unexpected_pair_count, 0);

    for owner in [
        LocationCellOwner::NpcStartMarker,
        LocationCellOwner::SpawnMarker,
        LocationCellOwner::FloorLinkMarker,
        LocationCellOwner::DawnDuskGateMarker,
        LocationCellOwner::TownExit,
        LocationCellOwner::WalkOnStair,
        LocationCellOwner::ClimbTransition,
        LocationCellOwner::Door,
        LocationCellOwner::ChairTrigger,
        LocationCellOwner::PoisonGas,
        LocationCellOwner::Pushable,
        LocationCellOwner::SearchInspectable,
    ] {
        assert!(
            report.owner_counts[owner.index()] > 0,
            "expected shipped LOCATION.DAT corpus to contain {}",
            owner.label()
        );
    }

    assert!(
        report
            .physical_pages
            .iter()
            .any(|floor| floor.runtime_day_hash != floor.runtime_night_hash)
    );
    assert!(report.physical_pages.iter().all(|floor| {
        floor.owner_counts.iter().sum::<usize>() == TOWN_GRID_BYTES
            && floor.tile_class_counts.iter().sum::<usize>() == TOWN_GRID_BYTES
            && floor.view_class_counts.iter().sum::<usize>() == TOWN_GRID_BYTES
    }));

    let text = location_audit_report_text(&report);
    assert!(text.contains("physical_pages=64"));
    assert!(text.contains("owner_counts:"));
    assert!(text.contains("tile_class_counts:"));
    assert!(text.contains("view_class_counts:"));
    assert!(text.contains("movement_counts"));
    assert!(!text.contains("[["));
    assert!(!text.contains("32x32"));
}
