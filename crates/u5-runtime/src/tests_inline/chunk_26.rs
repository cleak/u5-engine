#[test]
fn visual_asset_audit_summarizes_tile_and_fixed_font_entries_without_raw_pixels() {
    let atlas = test_fixtures::synthetic_tile_atlas(TileGraphicsDepth::Ega16);
    let tile_entry = audit_tile_atlas_resource(TILES_EGA_FILE, &atlas);
    assert_eq!(tile_entry.kind, VisualAssetAuditKind::TileAtlas);
    assert_eq!(tile_entry.depth, Some(TileGraphicsDepth::Ega16));
    assert_eq!(tile_entry.item_count, TILE_ATLAS_TILE_COUNT);
    assert_eq!(tile_entry.cell_width, TILE_ATLAS_SIDE);
    assert_eq!(tile_entry.cell_height, TILE_ATLAS_SIDE);
    assert_eq!(tile_entry.total_pixels, TILE_ATLAS_PIXEL_LEN);
    assert_eq!(tile_entry.max_value, 15);
    assert_eq!(tile_entry.value_mask, 0xffff);
    assert!(tile_entry.nonzero_pixels > 0);

    let mut font_bytes = vec![0u8; CH_FONT_LEN];
    font_bytes[0] = 0xff;
    font_bytes[CH_FONT_LEN - 1] = 0x01;
    let font =
        parse_fixed_font_body(&font_bytes, IBM_CH_FILE, CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
            .unwrap();
    let font_entry = audit_fixed_font_resource(IBM_CH_FILE, &font);
    assert_eq!(font_entry.kind, VisualAssetAuditKind::FixedFont);
    assert_eq!(font_entry.depth, None);
    assert_eq!(font_entry.item_count, FIXED_FONT_GLYPH_COUNT);
    assert_eq!(font_entry.cell_width, CH_FONT_CELL_WIDTH);
    assert_eq!(font_entry.cell_height, CH_FONT_CELL_HEIGHT);
    assert_eq!(
        font_entry.total_pixels,
        FIXED_FONT_GLYPH_COUNT * CH_FONT_CELL_WIDTH * CH_FONT_CELL_HEIGHT
    );
    assert_eq!(font_entry.nonzero_pixels, 9);
    assert_eq!(font_entry.max_value, 1);
    assert_eq!(font_entry.value_mask, 0x3);
    let font_total_pixels = font_entry.total_pixels;

    let report = visual_asset_audit_report_from_entries(vec![tile_entry, font_entry]);
    assert_eq!(report.entries.len(), 2);
    assert_eq!(report.kind_counts[VisualAssetAuditKind::TileAtlas.index()], 1);
    assert_eq!(report.kind_counts[VisualAssetAuditKind::FixedFont.index()], 1);
    assert_eq!(
        report.total_items,
        TILE_ATLAS_TILE_COUNT + FIXED_FONT_GLYPH_COUNT
    );
    assert_eq!(
        report.total_pixels,
        TILE_ATLAS_PIXEL_LEN + font_total_pixels
    );

    let text = visual_asset_audit_report_text(&report);
    assert!(text.contains("resources=2"));
    assert!(text.contains("tile-atlas=1"));
    assert!(text.contains("fixed-font=1"));
    assert!(text.contains("hash="));
    assert!(!text.contains("[["));
    assert!(!text.contains("glyph"));
    assert!(!text.contains("row"));
    assert!(!text.contains("pixels=["));
}

#[test]
fn shipped_visual_asset_audit_covers_tile_atlases_and_fixed_fonts_when_assets_present() {
    let game_dir = Path::new(DEFAULT_GAME_DIR);
    if ![
        TILES_EGA_FILE,
        TILES_CGA_FILE,
        IBM_CH_FILE,
        RUNES_CH_FILE,
        IBM_HCS_FILE,
        RUNES_HCS_FILE,
    ]
    .iter()
    .all(|name| game_dir.join(name).exists())
    {
        return;
    }

    let report = audit_visual_assets(game_dir).unwrap();
    assert_eq!(report.entries.len(), 6);
    assert_eq!(report.kind_counts[VisualAssetAuditKind::TileAtlas.index()], 2);
    assert_eq!(report.kind_counts[VisualAssetAuditKind::FixedFont.index()], 4);
    assert_eq!(
        report.total_items,
        TILE_ATLAS_TILE_COUNT * 2 + FIXED_FONT_GLYPH_COUNT * 4
    );
    assert!(report.total_pixels > 0);
    assert!(report.nonzero_pixels > 0);

    for entry in &report.entries {
        assert!(entry.total_pixels > 0);
        assert!(entry.nonzero_pixels > 0);
        assert!(entry.content_hash != 0);
        match entry.kind {
            VisualAssetAuditKind::TileAtlas => {
                assert_eq!(entry.item_count, TILE_ATLAS_TILE_COUNT);
                assert_eq!(entry.cell_width, TILE_ATLAS_SIDE);
                assert_eq!(entry.cell_height, TILE_ATLAS_SIDE);
                assert!(entry.max_value < entry.depth.unwrap().pixel_limit());
            }
            VisualAssetAuditKind::FixedFont => {
                assert_eq!(entry.item_count, FIXED_FONT_GLYPH_COUNT);
                assert!(entry.max_value <= 1);
                assert!(entry.value_mask & !0x3 == 0);
            }
        }
    }

    let text = visual_asset_audit_report_text(&report);
    assert!(text.contains("resources=6"));
    assert!(text.contains(TILES_EGA_FILE));
    assert!(text.contains(IBM_CH_FILE));
    assert!(!text.contains("[["));
    assert!(!text.contains("glyph"));
    assert!(!text.contains("row"));
}
