
    pub use crate::test_fixtures::*;

    fn location_pages() -> Vec<u8> {
        let mut pages = Vec::with_capacity(16 * 1024);
        for page in 0..16 {
            pages.extend(std::iter::repeat(page as u8).take(1024));
        }
        pages
    }

    fn write_castle_trap_door_fixture(dir: &Path) {
        fs::write(dir.join("CASTLE.DAT"), location_pages()).unwrap();
        fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 5\n").unwrap();
        fs::write(
            dir.join(TOWN_TRAP_DOOR_TABLE_FILE),
            "CASTLE:0 0 1 1 -1 55\n",
        )
        .unwrap();
    }

    fn write_castle_exit_tile_fixture(dir: &Path) {
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "BRITANNIA 10 20 CASTLE:0\n",
        )
        .unwrap();
        fs::write(dir.join(TOWN_EXIT_TILE_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
    }

    fn town_trap_door_origin_state() -> PlayState {
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        test_state(grid, 1, 1)
    }

    fn town_exit_tile_origin_state() -> PlayState {
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        test_state(grid, 1, 1)
    }

    fn look2_bytes(entries: &[(usize, &str)]) -> Vec<u8> {
        let mut offsets = vec![LOOK2_TABLE_LEN as u16; LOOK2_TILE_COUNT];
        let mut pool = Vec::new();
        pool.extend_from_slice(b"*\0");
        for (tile, description) in entries {
            let offset = LOOK2_TABLE_LEN + pool.len();
            offsets[*tile] = offset as u16;
            pool.extend_from_slice(description.as_bytes());
            pool.push(0);
        }

        let mut bytes = vec![0; LOOK2_TABLE_LEN];
        for (tile, offset) in offsets.iter().enumerate() {
            bytes[tile * 2..tile * 2 + 2].copy_from_slice(&offset.to_le_bytes());
        }
        bytes.extend(pool);
        bytes
    }

    fn lzw_envelope_with_9_bit_codes(decoded_len: usize, codes: &[u16]) -> Vec<u8> {
        let mut payload = Vec::new();
        let mut bit_pos = 0usize;
        for code in codes {
            for bit_offset in 0..LZW_INITIAL_CODE_SIZE {
                if bit_pos / 8 == payload.len() {
                    payload.push(0);
                }
                if code & (1u16 << bit_offset) != 0 {
                    payload[bit_pos / 8] |= 1u8 << (bit_pos % 8);
                }
                bit_pos += 1;
            }
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(decoded_len as u32).to_le_bytes());
        bytes.extend(payload);
        bytes
    }

    fn tlk_bytes(entries: &[(u16, &[&str])]) -> Vec<u8> {
        let count = entries.len() + 1;
        let mut bytes = vec![0; count * 4];
        bytes[0..2].copy_from_slice(&(count as u16).to_le_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
        let mut pool = Vec::new();

        for (index, (id, fields)) in entries.iter().enumerate() {
            let offset = bytes.len() + pool.len();
            let header = (index + 1) * 4;
            bytes[header..header + 2].copy_from_slice(&(offset as u16).to_le_bytes());
            bytes[header + 2..header + 4].copy_from_slice(&id.to_le_bytes());
            for field in *fields {
                for byte in field.bytes() {
                    pool.push(byte | 0x80);
                }
                pool.push(0);
            }
        }

        bytes.extend(pool);
        bytes
    }

    fn karma_bytes(records: &[&str]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for record in records {
            bytes.extend_from_slice(record.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    fn passability_with_tiles(tiles: &[u8]) -> TilePassability {
        let mut bytes = [0; TILE_PASSABILITY_LEN];
        for tile in tiles {
            bytes[(*tile >> 3) as usize] |= 0x80u8 >> (*tile & 7);
        }
        TilePassability::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn tile_graphics_lzw_decodes_literal_stream() {
        let bytes = lzw_envelope_with_9_bit_codes(3, &[LZW_CLEAR_CODE, 1, 2, 3, LZW_END_CODE]);

        assert_eq!(
            decode_lzw_envelope(&bytes, "fixture").unwrap(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn tile_graphics_lzw_handles_kwkwk_self_reference() {
        let bytes = lzw_envelope_with_9_bit_codes(
            3,
            &[
                LZW_CLEAR_CODE,
                b'A' as u16,
                LZW_FIRST_USER_CODE,
                LZW_END_CODE,
            ],
        );

        assert_eq!(decode_lzw_envelope(&bytes, "fixture").unwrap(), b"AAA");
    }

    #[test]
    fn tile_graphics_lzw_rejects_declared_length_mismatch() {
        let bytes = lzw_envelope_with_9_bit_codes(4, &[LZW_CLEAR_CODE, 1, 2, 3, LZW_END_CODE]);

        let err = decode_lzw_envelope(&bytes, "fixture").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn tile_graphics_lzw_rejects_missing_end_code() {
        let bytes = lzw_envelope_with_9_bit_codes(1, &[LZW_CLEAR_CODE, 1]);

        let err = decode_lzw_envelope(&bytes, "fixture").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn karma_dat_parser_reads_six_nul_records_and_ignores_trailing_data() {
        let bytes = karma_bytes(&["low", "twenty", "forty", "sixty", "eighty", "top"]);
        let mut with_trailer = bytes.clone();
        with_trailer.extend_from_slice(b"ignored trailer");

        assert_eq!(
            parse_karma_dat(&with_trailer).unwrap(),
            vec!["low", "twenty", "forty", "sixty", "eighty", "top"]
        );
    }

    #[test]
    fn karma_dat_parser_rejects_missing_records_and_high_bytes() {
        assert!(parse_karma_dat(&karma_bytes(&["one", "two"])).is_err());

        let mut high = karma_bytes(&["one", "two", "three", "four", "five", "six"]);
        high[1] = 0x80;

        assert!(parse_karma_dat(&high).is_err());
    }

    #[test]
    fn karma_verdict_selectors_keep_blackthorn_and_camp_top_bands_distinct() {
        assert_eq!(blackthorn_karma_record_index(0), 0);
        assert_eq!(blackthorn_karma_record_index(79), 3);
        assert_eq!(blackthorn_karma_record_index(80), 4);
        assert_eq!(blackthorn_karma_record_index(255), 4);
        assert_eq!(lord_british_camp_karma_record_index(0), 0);
        assert_eq!(lord_british_camp_karma_record_index(79), 3);
        assert_eq!(lord_british_camp_karma_record_index(80), 5);
        assert_eq!(lord_british_camp_karma_record_index(255), 5);
    }

    #[test]
    fn load_karma_records_reads_optional_karma_dat_file() {
        let dir = debug_game_dir();
        assert!(load_karma_records(&dir).unwrap().is_none());

        fs::write(
            dir.join(KARMA_DAT_FILE),
            karma_bytes(&["low", "twenty", "forty", "sixty", "eighty", "top"]),
        )
        .unwrap();

        let records = load_karma_records(&dir).unwrap().unwrap();
        assert_eq!(records[5], "top");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn tile_graphics_unpacks_ega_atlas_body() {
        let mut body = vec![0; TILE_ATLAS_EGA_BODY_LEN];
        body[0] = 0xab;
        body[TILE_ATLAS_EGA_TILE_STRIDE - 1] = 0xcd;
        body[TILE_ATLAS_EGA_TILE_STRIDE] = 0x12;

        let atlas =
            unpack_tile_atlas_body(&body, TileGraphicsDepth::Ega16, TILES_EGA_FILE).unwrap();

        assert_eq!(atlas.depth, TileGraphicsDepth::Ega16);
        assert_eq!(atlas.pixels.len(), TILE_ATLAS_PIXEL_LEN);
        assert_eq!(&atlas.tile_pixels(0).unwrap()[..2], &[0x0a, 0x0b]);
        assert_eq!(
            &atlas.tile_pixels(0).unwrap()[TILE_ATLAS_TILE_PIXELS - 2..],
            &[0x0c, 0x0d]
        );
        assert_eq!(&atlas.tile_pixels(1).unwrap()[..2], &[0x01, 0x02]);
        assert!(atlas.tile_pixels(TILE_ATLAS_TILE_COUNT).is_none());
    }

    #[test]
    fn tile_graphics_unpacks_cga_atlas_body() {
        let mut body = vec![0; TILE_ATLAS_CGA_BODY_LEN];
        body[0] = 0b1100_1001;
        body[TILE_ATLAS_CGA_TILE_STRIDE - 1] = 0b0110_1110;
        body[TILE_ATLAS_CGA_TILE_STRIDE] = 0b0001_1011;

        let atlas = unpack_tile_atlas_body(&body, TileGraphicsDepth::Cga4, TILES_CGA_FILE).unwrap();

        assert_eq!(atlas.depth, TileGraphicsDepth::Cga4);
        assert_eq!(atlas.pixels.len(), TILE_ATLAS_PIXEL_LEN);
        assert_eq!(&atlas.tile_pixels(0).unwrap()[..4], &[3, 0, 2, 1]);
        assert_eq!(
            &atlas.tile_pixels(0).unwrap()[TILE_ATLAS_TILE_PIXELS - 4..],
            &[1, 2, 3, 2]
        );
        assert_eq!(&atlas.tile_pixels(1).unwrap()[..4], &[0, 1, 2, 3]);
        assert!(atlas.tile_pixels(TILE_ATLAS_TILE_COUNT).is_none());
    }

    #[test]
    fn tile_graphics_rejects_wrong_atlas_body_length() {
        let err = unpack_tile_atlas_body(&[0], TileGraphicsDepth::Cga4, "fixture").unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn tile_graphics_local_clean_tiles_decode_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join(TILES_EGA_FILE).exists() || !game_dir.join(TILES_CGA_FILE).exists() {
            return;
        }

        let ega = load_tile_atlas(game_dir, TileGraphicsDepth::Ega16).unwrap();
        let cga = load_tile_atlas(game_dir, TileGraphicsDepth::Cga4).unwrap();

        assert_eq!(ega.depth, TileGraphicsDepth::Ega16);
        assert_eq!(ega.pixels.len(), TILE_ATLAS_PIXEL_LEN);
        assert_eq!(
            ega.tile_pixels(TILE_ATLAS_TILE_COUNT - 1).unwrap().len(),
            TILE_ATLAS_TILE_PIXELS
        );
        assert!(ega.pixels.iter().all(|pixel| *pixel < 16));

        assert_eq!(cga.depth, TileGraphicsDepth::Cga4);
        assert_eq!(cga.pixels.len(), TILE_ATLAS_PIXEL_LEN);
        assert_eq!(
            cga.tile_pixels(TILE_ATLAS_TILE_COUNT - 1).unwrap().len(),
            TILE_ATLAS_TILE_PIXELS
        );
        assert!(cga.pixels.iter().all(|pixel| *pixel < 4));
    }

    #[test]
    fn tile_graphics_parses_ega_image_directory_body() {
        let mut body = vec![0; 10];
        body[0..2].copy_from_slice(&2u16.to_le_bytes());
        body[6..10].copy_from_slice(&10u32.to_le_bytes());
        body.extend_from_slice(&3u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&[0x12, 0x30, 0, 0]);
        body.extend_from_slice(&[0xab, 0xc0, 0, 0]);

        let directory =
            parse_graphic_image_directory_body(&body, TileGraphicsDepth::Ega16, "fixture").unwrap();

        assert_eq!(directory.depth, TileGraphicsDepth::Ega16);
        assert_eq!(directory.images.len(), 2);
        assert!(directory.images[0].is_none());
        let image = directory.images[1].as_ref().unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.pixels, vec![1, 2, 3, 10, 11, 12]);
    }

    #[test]
    fn tile_graphics_parses_cga_image_directory_body() {
        let mut body = vec![0; 6];
        body[0..2].copy_from_slice(&1u16.to_le_bytes());
        body[2..6].copy_from_slice(&6u32.to_le_bytes());
        body.extend_from_slice(&5u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&[0b0001_1011, 0b1100_0000]);

        let directory =
            parse_graphic_image_directory_body(&body, TileGraphicsDepth::Cga4, "fixture").unwrap();

        let image = directory.images[0].as_ref().unwrap();
        assert_eq!((image.width, image.height), (5, 1));
        assert_eq!(image.pixels, vec![0, 1, 2, 3, 3]);
    }

    #[test]
    fn tile_graphics_parses_sprite_and_mask_body() {
        let image_offset = 6u16;
        let image_block = [3, 0, 1, 0, 0b0001_1000];
        let mask_offset = image_offset + image_block.len() as u16;
        let mut body = Vec::new();
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&image_offset.to_le_bytes());
        body.extend_from_slice(&mask_offset.to_le_bytes());
        body.extend_from_slice(&image_block);
        body.extend_from_slice(&3u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0b1010_0000);

        let sheet =
            parse_graphic_sprite_sheet_body(&body, TileGraphicsDepth::Cga4, "fixture").unwrap();

        assert_eq!(sheet.depth, TileGraphicsDepth::Cga4);
        assert_eq!(sheet.sprites.len(), 1);
        let sprite = sheet.sprites[0].as_ref().unwrap();
        assert_eq!((sprite.image.width, sprite.image.height), (3, 1));
        assert_eq!(sprite.image.pixels, vec![0, 1, 2]);
        assert_eq!(sprite.transparent_mask, vec![1, 0, 1]);
    }

    #[test]
    fn tile_graphics_rejects_sprite_mask_dimension_mismatch() {
        let image_offset = 6u16;
        let image_block = [3, 0, 1, 0, 0b0001_1000];
        let mask_offset = image_offset + image_block.len() as u16;
        let mut body = Vec::new();
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&image_offset.to_le_bytes());
        body.extend_from_slice(&mask_offset.to_le_bytes());
        body.extend_from_slice(&image_block);
        body.extend_from_slice(&4u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0);

        let err =
            parse_graphic_sprite_sheet_body(&body, TileGraphicsDepth::Cga4, "fixture").unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn tile_graphics_local_clean_sprite_sheets_decode_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
            if game_dir
                .join(tile_graphics_file_name("ITEMS", depth))
                .exists()
            {
                let sheet = load_graphic_sprite_sheet(game_dir, "ITEMS", depth).unwrap();
                assert_sprite_sheet_shape(&sheet, 10);
            }
            for monster_sheet in 0..=7 {
                let stem = format!("MON{monster_sheet}");
                if !game_dir
                    .join(tile_graphics_file_name(&stem, depth))
                    .exists()
                {
                    continue;
                }
                let sheet = load_graphic_sprite_sheet(game_dir, &stem, depth).unwrap();
                assert_sprite_sheet_shape(&sheet, 3);
            }
        }
    }

    #[test]
    fn tile_graphics_local_clean_image_directories_decode_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        for (stem, expected_images) in [("TEXT", 6), ("DNG1", 28), ("DNG2", 28), ("DNG3", 28)] {
            for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
                if !game_dir.join(tile_graphics_file_name(stem, depth)).exists() {
                    continue;
                }
                let directory = load_graphic_image_directory(game_dir, stem, depth).unwrap();
                assert_eq!(directory.depth, depth);
                assert_eq!(directory.images.len(), expected_images);
                assert!(directory.images.iter().any(Option::is_some));
                for image in directory.images.iter().flatten() {
                    assert_eq!(image.pixels.len(), image.width * image.height);
                    assert!(
                        image
                            .pixels
                            .iter()
                            .all(|pixel| *pixel < depth.pixel_limit())
                    );
                }
            }
        }
    }

    fn assert_sprite_sheet_shape(sheet: &GraphicSpriteSheet, expected_sprites: usize) {
        assert_eq!(sheet.sprites.len(), expected_sprites);
        for sprite in sheet.sprites.iter().flatten() {
            assert_eq!(
                sprite.image.pixels.len(),
                sprite.image.width * sprite.image.height
            );
            assert_eq!(sprite.transparent_mask.len(), sprite.image.pixels.len());
            assert!(
                sprite
                    .image
                    .pixels
                    .iter()
                    .all(|pixel| *pixel < sheet.depth.pixel_limit())
            );
            assert!(sprite.transparent_mask.iter().all(|pixel| *pixel <= 1));
        }
    }

    #[test]
    fn bit_graphics_parses_single_image_bitmap_body() {
        let mut body = Vec::new();
        body.extend_from_slice(&SINGLE_IMAGE_BIT_FORMAT_MARKER.to_le_bytes());
        body.extend_from_slice(&SINGLE_IMAGE_BIT_MODE_MARKER.to_le_bytes());
        body.extend_from_slice(&9u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&[0b1010_1010, 0b1100_0011, 0b1000_0000]);

        let bitmap = parse_single_image_bit_body(&body, "fixture").unwrap();

        assert_eq!((bitmap.width, bitmap.height), (9, 2));
        assert_eq!(bitmap.pixels.len(), 18);
        assert_eq!(&bitmap.pixels[..10], &[1, 0, 1, 0, 1, 0, 1, 0, 1, 1]);
        assert_eq!(&bitmap.pixels[16..], &[1, 0]);
    }

    #[test]
    fn bit_graphics_parses_sparse_strip_resource_body() {
        // formats/bit.md section 3: a sparse resource starts with an entry
        // count, then four-byte pointer/metadata entries. WD.BIT's one-entry
        // layout is valid because its pointer targets the metadata word, which
        // then doubles as the strip width word.
        let mut body = Vec::new();
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&4u16.to_le_bytes());
        body.extend_from_slice(&9u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&[0b1010_1010, 0b1100_0011, 0b1000_0000]);

        let images = parse_sparse_bit_images(&body, "fixture.bit").unwrap();

        assert_eq!(images.blocks.len(), 1);
        let bitmap = &images.blocks[0];
        assert_eq!((bitmap.width, bitmap.height), (9, 2));
        assert_eq!(bitmap.pixels.len(), 18);
        assert_eq!(&bitmap.pixels[..10], &[1, 0, 1, 0, 1, 0, 1, 0, 1, 1]);
        assert_eq!(&bitmap.pixels[16..], &[1, 0]);
    }

    #[test]
    fn bit_graphics_parse_title_accepts_raw_sparse_resource() {
        let mut body = Vec::new();
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&123u16.to_le_bytes());
        body.extend_from_slice(&10u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&8u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0b1010_0000);

        let title = parse_title_bit(&body).unwrap();

        assert_eq!(title.blocks.len(), 1);
        assert_eq!((title.blocks[0].width, title.blocks[0].height), (8, 1));
        assert_eq!(title.blocks[0].pixels, vec![1, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn bit_graphics_parses_title_bitmap_directory_body() {
        let mut body = Vec::new();
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&6u16.to_le_bytes());
        body.extend_from_slice(&11u16.to_le_bytes());
        body.extend_from_slice(&8u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0b1010_0000);
        body.extend_from_slice(&9u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&[0b1111_0000, 0b1000_0000]);

        let title = parse_title_bit_body(&body, "fixture").unwrap();

        assert_eq!(title.blocks.len(), 2);
        assert_eq!((title.blocks[0].width, title.blocks[0].height), (8, 1));
        assert_eq!(title.blocks[0].pixels, vec![1, 0, 1, 0, 0, 0, 0, 0]);
        assert_eq!((title.blocks[1].width, title.blocks[1].height), (9, 1));
        assert_eq!(title.blocks[1].pixels, vec![1, 1, 1, 1, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn bit_graphics_rejects_bad_markers_and_lengths() {
        let mut bad_marker = Vec::new();
        bad_marker.extend_from_slice(&2u16.to_le_bytes());
        bad_marker.extend_from_slice(&SINGLE_IMAGE_BIT_MODE_MARKER.to_le_bytes());
        bad_marker.extend_from_slice(&8u16.to_le_bytes());
        bad_marker.extend_from_slice(&1u16.to_le_bytes());
        bad_marker.push(0);

        assert!(parse_single_image_bit_body(&bad_marker, "fixture").is_err());

        let mut bad_length = Vec::new();
        bad_length.extend_from_slice(&SINGLE_IMAGE_BIT_FORMAT_MARKER.to_le_bytes());
        bad_length.extend_from_slice(&SINGLE_IMAGE_BIT_MODE_MARKER.to_le_bytes());
        bad_length.extend_from_slice(&8u16.to_le_bytes());
        bad_length.extend_from_slice(&1u16.to_le_bytes());

        assert!(parse_single_image_bit_body(&bad_length, "fixture").is_err());
        assert!(parse_sparse_bit_images(&[0], "fixture.bit").is_err());

        let mut bad_sparse_pointer = Vec::new();
        bad_sparse_pointer.extend_from_slice(&1u16.to_le_bytes());
        bad_sparse_pointer.extend_from_slice(&99u16.to_le_bytes());
        bad_sparse_pointer.extend_from_slice(&0u16.to_le_bytes());
        assert!(parse_sparse_bit_images(&bad_sparse_pointer, "fixture.bit").is_err());
    }

    #[test]
    fn bit_graphics_local_clean_bitmaps_decode_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if game_dir.join(TITLE_BIT_FILE).exists() {
            let title = load_title_bit(game_dir).unwrap();
            assert_eq!(title.blocks.len(), 10);
            assert_eq!(
                title
                    .blocks
                    .iter()
                    .map(|bitmap| (bitmap.width, bitmap.height))
                    .collect::<Vec<_>>(),
                vec![
                    (24, 3),
                    (40, 7),
                    (72, 11),
                    (112, 20),
                    (152, 32),
                    (216, 45),
                    (280, 61),
                    (104, 33),
                    (16, 15),
                    (112, 33),
                ]
            );
            assert!(
                title
                    .blocks
                    .iter()
                    .all(|bitmap| bitmap.pixels.iter().all(|pixel| *pixel <= 1))
            );
        }
        if game_dir.join(BRITISH_BIT_FILE).exists() {
            let british = load_british_bit(game_dir).unwrap();
            assert_eq!((british.width, british.height), (272, 62));
            assert_eq!(british.pixels.len(), 272 * 62);
            assert!(british.pixels.iter().all(|pixel| *pixel <= 1));
        }
        if game_dir.join(WD_BIT_FILE).exists() {
            let wd = load_wd_bit(game_dir).unwrap();
            assert_eq!((wd.width, wd.height), (288, 49));
            assert_eq!(wd.pixels.len(), 288 * 49);
            assert!(wd.pixels.iter().all(|pixel| *pixel <= 1));
        }
    }

    #[test]
    fn font_graphics_parses_fixed_font_body() {
        let mut ch = vec![0; FIXED_FONT_GLYPH_COUNT * CH_FONT_CELL_HEIGHT];
        let a_offset = 65 * CH_FONT_CELL_HEIGHT;
        ch[a_offset] = 0b1000_0001;
        ch[a_offset + 7] = 0b0001_1000;

        let font =
            parse_fixed_font_body(&ch, "fixture.ch", CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
                .unwrap();

        assert_eq!((font.cell_width, font.cell_height), (8, 8));
        assert_eq!(font.glyphs.len(), FIXED_FONT_GLYPH_COUNT);
        let glyph = font.glyph(65).unwrap();
        assert_eq!((glyph.width, glyph.height), (8, 8));
        assert_eq!(&glyph.pixels[..8], &[1, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(&glyph.pixels[56..], &[0, 0, 0, 1, 1, 0, 0, 0]);

        let mut hcs =
            vec![0; FIXED_FONT_GLYPH_COUNT * HCS_FONT_CELL_HEIGHT * (HCS_FONT_CELL_WIDTH / 8)];
        let glyph_offset = HCS_FONT_CELL_HEIGHT * (HCS_FONT_CELL_WIDTH / 8);
        hcs[glyph_offset] = 0b1000_0000;
        hcs[glyph_offset + 1] = 0b0000_0001;

        let font = parse_fixed_font_body(
            &hcs,
            "fixture.hcs",
            HCS_FONT_CELL_WIDTH,
            HCS_FONT_CELL_HEIGHT,
        )
        .unwrap();

        assert_eq!((font.cell_width, font.cell_height), (16, 12));
        assert_eq!(
            font.glyph(1).unwrap().pixels[..16],
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
        );
    }

    #[test]
    fn font_graphics_parses_proportional_font_body() {
        let mut body = Vec::new();
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&6u16.to_le_bytes());
        body.extend_from_slice(&18u16.to_le_bytes());
        body.push(0);
        body.extend_from_slice(&[0; PCS_GLYPH_HEIGHT]);
        body.push(5);
        body.extend_from_slice(&[0b1010_0000, 0b0101_0000, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let font = parse_proportional_font_body(&body, "fixture.pcs").unwrap();

        assert_eq!(font.first_code, PCS_FIRST_CODE);
        assert_eq!(font.glyphs.len(), 2);
        assert_eq!(font.glyph_for_code(0x20).unwrap().advance_width, 0);
        let glyph = font.glyph_for_code(0x21).unwrap();
        assert_eq!(glyph.advance_width, 5);
        assert_eq!(&glyph.bitmap.pixels[..8], &[1, 0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(&glyph.bitmap.pixels[8..16], &[0, 1, 0, 1, 0, 0, 0, 0]);
        assert!(font.glyph_for_code(0x22).is_none());
    }

    #[test]
    fn font_graphics_prepares_fixed_text_cell_styles() {
        let mut ch = vec![0; FIXED_FONT_GLYPH_COUNT * CH_FONT_CELL_HEIGHT];
        let glyph_offset = 65 * CH_FONT_CELL_HEIGHT;
        ch[glyph_offset] = 0b1000_0000;
        let font =
            parse_fixed_font_body(&ch, "fixture.ch", CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
                .unwrap();

        let plain = prepare_fixed_text_cell(&font, 65, TextCellStyle::default()).unwrap();
        assert_eq!(&plain.pixels[..8], &[1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&plain.pixels[56..], &[0; 8]);

        let underline = prepare_fixed_text_cell(
            &font,
            65,
            TextCellStyle {
                underline: true,
                inverse: false,
            },
        )
        .unwrap();
        assert_eq!(&underline.pixels[56..], &[1; 8]);

        let inverse_underlined = prepare_fixed_text_cell(
            &font,
            65,
            TextCellStyle {
                underline: true,
                inverse: true,
            },
        )
        .unwrap();
        assert_eq!(&inverse_underlined.pixels[..8], &[0, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(&inverse_underlined.pixels[56..], &[0; 8]);
    }

    #[test]
    fn font_graphics_rasterizes_fixed_text_line() {
        let mut ch = vec![0; FIXED_FONT_GLYPH_COUNT * CH_FONT_CELL_HEIGHT];
        ch[65 * CH_FONT_CELL_HEIGHT] = 0b1000_0000;
        ch[66 * CH_FONT_CELL_HEIGHT] = 0b0000_0001;
        let font =
            parse_fixed_font_body(&ch, "fixture.ch", CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
                .unwrap();

        let line = rasterize_fixed_text_line(&font, b"AB", TextCellStyle::default()).unwrap();

        assert_eq!((line.width, line.height), (16, 8));
        assert_eq!(line.pixel(0, 0), Some(1));
        assert_eq!(line.pixel(7, 0), Some(0));
        assert_eq!(line.pixel(15, 0), Some(1));
        assert_eq!(line.pixel(16, 0), None);
    }

    #[test]
    fn font_graphics_measures_and_rasterizes_proportional_text_line() {
        let mut body = Vec::new();
        body.extend_from_slice(&3u16.to_le_bytes());
        body.extend_from_slice(&8u16.to_le_bytes());
        body.extend_from_slice(&20u16.to_le_bytes());
        body.extend_from_slice(&32u16.to_le_bytes());
        body.push(0);
        body.extend_from_slice(&[0; PCS_GLYPH_HEIGHT]);
        body.push(2);
        body.extend_from_slice(&[0b1100_0000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        body.push(3);
        body.extend_from_slice(&[0b1010_0000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let font = parse_proportional_font_body(&body, "fixture.pcs").unwrap();

        assert_eq!(measure_proportional_text(&font, b"!!").unwrap(), 4);
        let line = rasterize_proportional_text_line(&font, b"!\"").unwrap();

        assert_eq!((line.width, line.height), (5, PCS_GLYPH_HEIGHT));
        assert_eq!(&line.pixels[..5], &[1, 1, 1, 0, 1]);
    }

    #[test]
    fn font_graphics_parses_sparse_proportional_font_resource() {
        let mut body = Vec::new();
        body.extend_from_slice(&3u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0xaaaa_u16.to_le_bytes());
        body.extend_from_slice(&14u16.to_le_bytes());
        body.extend_from_slice(&0xbbbb_u16.to_le_bytes());
        body.extend_from_slice(&19u16.to_le_bytes());
        body.extend_from_slice(&0xcccc_u16.to_le_bytes());
        body.extend_from_slice(&4u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.push(0b1011_0011);
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0b1000_0000);

        let resource = parse_sparse_proportional_font_resource(&body).unwrap();

        assert_eq!(resource.strips.len(), 2);
        assert_eq!(resource.strip(0).map(|strip| (strip.width, strip.height)), Some((4, 2)));
        assert_eq!(resource.strip(0).unwrap().pixels, vec![1, 0, 1, 1, 0, 0, 1, 1]);
        assert_eq!(resource.strip(1).map(|strip| (strip.width, strip.height)), Some((2, 1)));
        assert_eq!(resource.strip(2), None);
    }

    #[test]
    fn font_graphics_rejects_bad_lengths_offsets_and_widths() {
        assert!(
            parse_fixed_font_body(&[0], "fixture.ch", CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
                .is_err()
        );
        assert!(parse_proportional_font_body(&[0], "fixture.pcs").is_err());

        let mut bad_offset = Vec::new();
        bad_offset.extend_from_slice(&1u16.to_le_bytes());
        bad_offset.extend_from_slice(&0u16.to_le_bytes());
        assert!(parse_proportional_font_body(&bad_offset, "fixture.pcs").is_err());

        let mut bad_width = Vec::new();
        bad_width.extend_from_slice(&1u16.to_le_bytes());
        bad_width.extend_from_slice(&4u16.to_le_bytes());
        bad_width.push((PCS_GLYPH_BITMAP_WIDTH + 1) as u8);
        bad_width.extend_from_slice(&[0; PCS_GLYPH_HEIGHT]);
        assert!(parse_proportional_font_body(&bad_width, "fixture.pcs").is_err());

        let font = parse_fixed_font_body(
            &[0; FIXED_FONT_GLYPH_COUNT * CH_FONT_CELL_HEIGHT],
            "fixture.ch",
            CH_FONT_CELL_WIDTH,
            CH_FONT_CELL_HEIGHT,
        )
        .unwrap();
        assert!(prepare_fixed_text_cell(&font, 0x80, TextCellStyle::default()).is_err());
    }

    #[test]
    fn font_graphics_local_clean_fonts_decode_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        for file_name in [IBM_CH_FILE, RUNES_CH_FILE] {
            if !game_dir.join(file_name).exists() {
                continue;
            }
            let font = load_ch_font(game_dir, file_name).unwrap();
            assert_fixed_font_shape(&font, CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT);
            let line = rasterize_fixed_text_line(&font, b"AV", TextCellStyle::default()).unwrap();
            assert_eq!((line.width, line.height), (16, 8));
            assert_eq!(line.pixels.len(), 16 * 8);
        }
        for file_name in [IBM_HCS_FILE, RUNES_HCS_FILE] {
            if !game_dir.join(file_name).exists() {
                continue;
            }
            let font = load_hcs_font(game_dir, file_name).unwrap();
            assert_fixed_font_shape(&font, HCS_FONT_CELL_WIDTH, HCS_FONT_CELL_HEIGHT);
            let line = rasterize_fixed_text_line(&font, b"AV", TextCellStyle::default()).unwrap();
            assert_eq!((line.width, line.height), (32, 12));
            assert_eq!(line.pixels.len(), 32 * 12);
        }
        if game_dir.join(PROPORT_PCS_FILE).exists() {
            let font = load_proportional_font(game_dir).unwrap();
            let resource = load_proportional_font_resource(game_dir).unwrap();
            assert_eq!(font.first_code, PCS_FIRST_CODE);
            assert_eq!(font.glyphs.len(), 91);
            assert!(font.glyph_for_code(0x7a).is_some());
            assert!(font.glyph_for_code(0x7b).is_none());
            assert!(measure_proportional_text(&font, b"Ultima").is_ok());
            assert!(!resource.strips.is_empty());
            assert!(resource.strips.iter().all(|strip| {
                strip.width > 0
                    && strip.height > 0
                    && strip.pixels.len() == strip.width * strip.height
                    && strip.pixels.iter().all(|pixel| *pixel <= 1)
            }));
            assert!(font.glyphs.iter().all(|glyph| {
                glyph.advance_width as usize <= PCS_GLYPH_BITMAP_WIDTH
                    && glyph.bitmap.width == PCS_GLYPH_BITMAP_WIDTH
                    && glyph.bitmap.height == PCS_GLYPH_HEIGHT
                    && glyph.bitmap.pixels.len() == PCS_GLYPH_BITMAP_WIDTH * PCS_GLYPH_HEIGHT
                    && glyph.bitmap.pixels.iter().all(|pixel| *pixel <= 1)
            }));
            let space = font.glyph_for_code(0x20).unwrap();
            assert_eq!(space.advance_width, 0);
            let line = rasterize_proportional_text_line(&font, b"Ultima").unwrap();
            assert_eq!(line.height, PCS_GLYPH_HEIGHT);
            assert_eq!(line.pixels.len(), line.width * line.height);
        }
    }

    fn assert_fixed_font_shape(font: &FixedFont, cell_width: usize, cell_height: usize) {
        assert_eq!(
            (font.cell_width, font.cell_height),
            (cell_width, cell_height)
        );
        assert_eq!(font.glyphs.len(), FIXED_FONT_GLYPH_COUNT);
        assert!(
            font.glyphs
                .iter()
                .any(|glyph| { glyph.pixels.iter().any(|pixel| *pixel == 1) })
        );
        for glyph in &font.glyphs {
            assert_eq!((glyph.width, glyph.height), (cell_width, cell_height));
            assert_eq!(glyph.pixels.len(), cell_width * cell_height);
            assert!(glyph.pixels.iter().all(|pixel| *pixel <= 1));
        }
    }

