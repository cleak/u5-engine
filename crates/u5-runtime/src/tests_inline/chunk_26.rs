#[test]
fn visual_asset_audit_summarizes_tile_and_fixed_font_entries_without_raw_pixels() {
    let atlas = test_fixtures::synthetic_tile_atlas(TileGraphicsDepth::Ega16);
    let tile_entry = audit_tile_atlas_resource(TILES_EGA_FILE, &atlas);
    assert_eq!(tile_entry.kind, VisualAssetAuditKind::TileAtlas);
    assert_eq!(tile_entry.depth, Some(TileGraphicsDepth::Ega16));
    assert_eq!(tile_entry.item_count, TILE_ATLAS_TILE_COUNT);
    assert_eq!(tile_entry.populated_items, TILE_ATLAS_TILE_COUNT);
    assert_eq!(tile_entry.cell_width, TILE_ATLAS_SIDE);
    assert_eq!(tile_entry.cell_height, TILE_ATLAS_SIDE);
    assert_eq!(tile_entry.total_pixels, TILE_ATLAS_PIXEL_LEN);
    assert_eq!(tile_entry.mask_pixels, 0);
    assert_eq!(tile_entry.mask_nonzero_pixels, 0);
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
    assert_eq!(font_entry.populated_items, FIXED_FONT_GLYPH_COUNT);
    assert_eq!(font_entry.cell_width, CH_FONT_CELL_WIDTH);
    assert_eq!(font_entry.cell_height, CH_FONT_CELL_HEIGHT);
    assert_eq!(
        font_entry.total_pixels,
        FIXED_FONT_GLYPH_COUNT * CH_FONT_CELL_WIDTH * CH_FONT_CELL_HEIGHT
    );
    assert_eq!(font_entry.nonzero_pixels, 9);
    assert_eq!(font_entry.mask_pixels, 0);
    assert_eq!(font_entry.mask_nonzero_pixels, 0);
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
fn visual_asset_audit_summarizes_image_directories_and_sprite_sheets_without_raw_pixels() {
    let image_a = GraphicImage {
        width: 2,
        height: 2,
        pixels: vec![0, 1, 2, 3],
    };
    let image_b = GraphicImage {
        width: 3,
        height: 1,
        pixels: vec![4, 0, 5],
    };
    let directory = GraphicImageDirectory {
        depth: TileGraphicsDepth::Ega16,
        images: vec![None, Some(image_a.clone()), Some(image_b)],
    };
    let directory_entry = audit_graphic_image_directory_resource("SAMPLE.16", &directory);
    assert_eq!(
        directory_entry.kind,
        VisualAssetAuditKind::GraphicImageDirectory
    );
    assert_eq!(directory_entry.depth, Some(TileGraphicsDepth::Ega16));
    assert_eq!(directory_entry.item_count, 3);
    assert_eq!(directory_entry.populated_items, 2);
    assert_eq!(directory_entry.cell_width, 3);
    assert_eq!(directory_entry.cell_height, 2);
    assert_eq!(directory_entry.total_pixels, 7);
    assert_eq!(directory_entry.nonzero_pixels, 5);
    assert_eq!(directory_entry.mask_pixels, 0);
    assert_eq!(directory_entry.mask_nonzero_pixels, 0);
    assert_eq!(directory_entry.value_mask, 0x3f);

    let sheet = GraphicSpriteSheet {
        depth: TileGraphicsDepth::Cga4,
        sprites: vec![
            Some(GraphicSprite {
                image: image_a,
                transparent_mask: vec![0, 1, 1, 0],
            }),
            None,
        ],
    };
    let sheet_entry = audit_graphic_sprite_sheet_resource("ITEMS.4", &sheet);
    assert_eq!(
        sheet_entry.kind,
        VisualAssetAuditKind::GraphicSpriteSheet
    );
    assert_eq!(sheet_entry.depth, Some(TileGraphicsDepth::Cga4));
    assert_eq!(sheet_entry.item_count, 2);
    assert_eq!(sheet_entry.populated_items, 1);
    assert_eq!(sheet_entry.cell_width, 2);
    assert_eq!(sheet_entry.cell_height, 2);
    assert_eq!(sheet_entry.total_pixels, 4);
    assert_eq!(sheet_entry.nonzero_pixels, 3);
    assert_eq!(sheet_entry.mask_pixels, 4);
    assert_eq!(sheet_entry.mask_nonzero_pixels, 2);
    assert_eq!(sheet_entry.value_mask, 0xf);

    let report = visual_asset_audit_report_from_entries(vec![directory_entry, sheet_entry]);
    assert_eq!(
        report.kind_counts[VisualAssetAuditKind::GraphicImageDirectory.index()],
        1
    );
    assert_eq!(
        report.kind_counts[VisualAssetAuditKind::GraphicSpriteSheet.index()],
        1
    );
    let text = visual_asset_audit_report_text(&report);
    assert!(text.contains("graphic-image-directory=1"));
    assert!(text.contains("graphic-sprite-sheet=1"));
    assert!(!text.contains("pixels=["));
    assert!(!text.contains("[["));
    assert!(!text.contains("row"));
}

#[test]
fn shipped_visual_asset_audit_covers_paired_graphics_and_fixed_fonts_when_assets_present() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let game_dir = game_dir.as_path();
    let mut required = [
        TILES_EGA_FILE,
        TILES_CGA_FILE,
        IBM_CH_FILE,
        RUNES_CH_FILE,
        IBM_HCS_FILE,
        RUNES_HCS_FILE,
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect::<Vec<_>>();
    for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
        required.extend(
            GRAPHIC_IMAGE_DIRECTORY_STEMS
                .iter()
                .map(|stem| tile_graphics_file_name(stem, depth)),
        );
        required.extend(
            GRAPHIC_SPRITE_SHEET_STEMS
                .iter()
                .map(|stem| tile_graphics_file_name(stem, depth)),
        );
    }
    if !required.iter().all(|name| game_dir.join(name).exists()) {
        return;
    }

    let report = audit_visual_assets(game_dir).unwrap();
    assert_eq!(
        report.entries.len(),
        6 + (GRAPHIC_IMAGE_DIRECTORY_STEMS.len() + GRAPHIC_SPRITE_SHEET_STEMS.len()) * 2
    );
    assert_eq!(report.kind_counts[VisualAssetAuditKind::TileAtlas.index()], 2);
    assert_eq!(report.kind_counts[VisualAssetAuditKind::FixedFont.index()], 4);
    assert_eq!(
        report.kind_counts[VisualAssetAuditKind::GraphicImageDirectory.index()],
        GRAPHIC_IMAGE_DIRECTORY_STEMS.len() * 2
    );
    assert_eq!(
        report.kind_counts[VisualAssetAuditKind::GraphicSpriteSheet.index()],
        GRAPHIC_SPRITE_SHEET_STEMS.len() * 2
    );
    assert_eq!(
        report.total_items,
        report.entries.iter().map(|entry| entry.item_count).sum()
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
            VisualAssetAuditKind::GraphicImageDirectory => {
                assert!(entry.item_count >= entry.populated_items);
                assert!(entry.populated_items > 0);
                assert!(entry.cell_width > 0);
                assert!(entry.cell_height > 0);
                assert_eq!(entry.mask_pixels, 0);
                assert_eq!(entry.mask_nonzero_pixels, 0);
                assert!(entry.max_value < entry.depth.unwrap().pixel_limit());
            }
            VisualAssetAuditKind::GraphicSpriteSheet => {
                assert!(entry.item_count >= entry.populated_items);
                assert!(entry.populated_items > 0);
                assert!(entry.cell_width > 0);
                assert!(entry.cell_height > 0);
                assert!(entry.mask_pixels >= entry.mask_nonzero_pixels);
                assert!(entry.mask_pixels > 0);
                assert!(entry.max_value < entry.depth.unwrap().pixel_limit());
            }
        }
    }

    let text = visual_asset_audit_report_text(&report);
    assert!(text.contains("resources=56"));
    assert!(text.contains(TILES_EGA_FILE));
    assert!(text.contains(IBM_CH_FILE));
    assert!(text.contains("STARTSC.16"));
    assert!(text.contains("ITEMS.4"));
    assert!(!text.contains("[["));
    assert!(!text.contains("glyph"));
    assert!(!text.contains("row"));
}
