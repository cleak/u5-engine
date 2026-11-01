// Closed-audit follow-up, group g0: rules the engine implements but no test
// asserted, from `systems/combat.md`, `formats/cbt.md`,
// `systems/hidden-treasures.md`, `formats/brit-dat.md`, `formats/font-ch.md`
// and `formats/lzw.md`.
//
// Each test names its file and section, and asserts the rule as the spec
// states it. Only published literals appear here.

// ---------------------------------------------------------------------------
// formats/lzw.md
// ---------------------------------------------------------------------------

/// Pack `(code, width)` pairs least-significant-bit first, behind the
/// four-byte little-endian decoded length (`formats/lzw.md §2`, `§3`).
fn g0_lzw_envelope(decoded_len: usize, codes: &[(u16, u8)]) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut bit_pos = 0usize;
    for &(code, width) in codes {
        for bit in 0..width {
            if bit_pos / 8 == payload.len() {
                payload.push(0);
            }
            if code & (1u16 << bit) != 0 {
                payload[bit_pos / 8] |= 1u8 << (bit_pos % 8);
            }
            bit_pos += 1;
        }
    }
    let mut bytes = (decoded_len as u32).to_le_bytes().to_vec();
    bytes.extend(payload);
    bytes
}

/// `formats/lzw.md §3`: "Codes start at nine bits. Code width grows by one
/// bit as the dictionary fills. Code width is capped at twelve bits ... The
/// dictionary can hold 4096 entries." A literal-only stream adds one entry
/// per code after the first, starting at code 258, so the dictionary fills
/// the nine-, ten- and eleven-bit spaces after codes 254, 766 and 1790, and
/// fills all 4096 entries after code 3838; past that point the width stays
/// at twelve.
#[test]
fn audit_g0_lzw_3_code_width_grows_with_the_dictionary_and_caps_at_twelve() {
    fn width_for(index: usize) -> u8 {
        match index {
            0..=254 => 9,
            255..=766 => 10,
            767..=1790 => 11,
            _ => 12,
        }
    }
    let literal_count = 4000usize;
    let expected: Vec<u8> = (0..literal_count).map(|i| (i % 256) as u8).collect();
    let mut codes = vec![(LZW_CLEAR_CODE, 9u8)];
    for (index, byte) in expected.iter().enumerate() {
        codes.push((u16::from(*byte), width_for(index)));
    }
    codes.push((LZW_END_CODE, width_for(literal_count)));

    let bytes = g0_lzw_envelope(literal_count, &codes);

    assert_eq!(decode_lzw_envelope(&bytes, "fixture").unwrap(), expected);
}

/// `formats/lzw.md §3`: code 256 "Clear dictionary and return to nine-bit
/// codes", and "After a clear code, the next newly emitted dictionary entry
/// uses the first dynamic code."
#[test]
fn audit_g0_lzw_3_clear_code_returns_to_nine_bits_and_restarts_at_code_258() {
    // 256 literals grow the width to ten bits; the clear arrives at ten.
    let mut codes = vec![(LZW_CLEAR_CODE, 9u8)];
    let mut expected = Vec::new();
    for index in 0..256usize {
        codes.push((index as u16, if index <= 254 { 9 } else { 10 }));
        expected.push(index as u8);
    }
    codes.push((LZW_CLEAR_CODE, 10));
    // Back at nine bits: a literal, then the first dynamic code (258), which
    // names the entry being created (the self-reference case).
    codes.push((u16::from(b'A'), 9));
    codes.push((LZW_FIRST_USER_CODE, 9));
    codes.push((LZW_END_CODE, 9));
    expected.extend_from_slice(b"AAA");
    assert_eq!(LZW_FIRST_USER_CODE, 258);

    let bytes = g0_lzw_envelope(expected.len(), &codes);

    assert_eq!(decode_lzw_envelope(&bytes, "fixture").unwrap(), expected);
}

/// `formats/lzw.md §4`: "Extra unread bytes after a valid end code are not
/// part of the decompressed payload".
#[test]
fn audit_g0_lzw_4_bytes_after_the_end_code_are_ignored() {
    let mut bytes = g0_lzw_envelope(2, &[(LZW_CLEAR_CODE, 9), (7, 9), (9, 9), (LZW_END_CODE, 9)]);
    bytes.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

    assert_eq!(decode_lzw_envelope(&bytes, "fixture").unwrap(), vec![7, 9]);
}

/// `formats/lzw.md §4`: "an overlong expansion ... [is a] content error",
/// and `§2`: a reader "should allocate or bounds-check against the declared
/// decoded length".
#[test]
fn audit_g0_lzw_4_an_overlong_expansion_is_a_content_error() {
    let bytes = g0_lzw_envelope(
        2,
        &[
            (LZW_CLEAR_CODE, 9),
            (1, 9),
            (2, 9),
            (3, 9),
            (LZW_END_CODE, 9),
        ],
    );

    let err = decode_lzw_envelope(&bytes, "fixture").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

/// `formats/lzw.md §2`: "Each compressed resource starts with a four-byte
/// little-endian unsigned length"; `§4`: "missing bytes ... are content
/// errors". A resource too short to hold the length is one.
#[test]
fn audit_g0_lzw_2_a_resource_shorter_than_the_length_header_is_rejected() {
    for len in 0..4 {
        let err = decode_lzw_envelope(&vec![0u8; len], "fixture").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData, "length {len}");
    }
}

/// `formats/lzw.md §2`: the length is little-endian. A declared length of
/// 0x0103 (259) has its low byte first.
#[test]
fn audit_g0_lzw_2_decoded_length_is_little_endian() {
    let mut codes = vec![(LZW_CLEAR_CODE, 9u8)];
    for index in 0..259usize {
        codes.push(((index % 7) as u16, if index <= 254 { 9 } else { 10 }));
    }
    codes.push((LZW_END_CODE, 10));
    let bytes = g0_lzw_envelope(259, &codes);
    assert_eq!(&bytes[..4], &[0x03, 0x01, 0x00, 0x00]);

    assert_eq!(decode_lzw_envelope(&bytes, "fixture").unwrap().len(), 259);
}

// ---------------------------------------------------------------------------
// formats/font-ch.md
// ---------------------------------------------------------------------------

/// `formats/font-ch.md §2` and `§7`: a valid font "is exactly one thousand
/// twenty-four bytes long", and a strict reader must "Reject files whose
/// length is not exactly one thousand twenty-four bytes" - long ones as well
/// as short ones.
#[test]
fn audit_g0_font_ch_7_strict_reader_rejects_every_length_but_1024() {
    assert_eq!(CH_FONT_LEN, 1024);
    assert_eq!(CH_FONT_LEN, 128 * 8);
    for len in [0usize, 1023, 1025, 2048] {
        assert!(
            parse_ch_font(&vec![0; len], IBM_CH_FILE).is_err(),
            "length {len}"
        );
        assert!(
            parse_fixed_font_body(
                &vec![0; len],
                IBM_CH_FILE,
                CH_FONT_CELL_WIDTH,
                CH_FONT_CELL_HEIGHT
            )
            .is_err(),
            "length {len}"
        );
    }
    assert!(parse_ch_font(&vec![0; 1024], IBM_CH_FILE).is_ok());
}

/// `formats/font-ch.md §4`: "Glyph number `n` starts at byte `n * 8` and
/// runs for eight bytes", rows "top to bottom" (`§5`), for every glyph zero
/// through one hundred twenty-seven.
#[test]
fn audit_g0_font_ch_4_glyph_n_occupies_bytes_n_times_8() {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = (offset as u8).wrapping_mul(31).wrapping_add(7);
    }
    let font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    for code in 0..128u8 {
        for row in 0..8usize {
            assert_eq!(
                font.glyph_row(code, row),
                Some(bytes[usize::from(code) * 8 + row]),
                "glyph {code} row {row}"
            );
        }
    }
}

/// `formats/font-ch.md §5`: "Within each row, bits are interpreted from most
/// significant to least significant": bit 7 is the leftmost pixel and bit 0
/// the rightmost, and "A set bit means foreground".
#[test]
fn audit_g0_font_ch_5_bit_seven_is_the_leftmost_pixel_of_each_row() {
    let mut bytes = vec![0u8; CH_FONT_LEN];
    let code = 0x41usize;
    for row in 0..8 {
        bytes[code * 8 + row] = 0x80 >> row;
    }
    let font = parse_fixed_font_body(&bytes, IBM_CH_FILE, CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
        .unwrap();
    let glyph = font.glyph(code as u8).unwrap();
    for row in 0..8 {
        for column in 0..8 {
            let expected = u8::from(column == row);
            assert_eq!(
                glyph.pixels[row * 8 + column],
                expected,
                "row {row} column {column}"
            );
        }
    }
}

/// `formats/font-ch.md §7`: "Reject attempts to index glyphs outside the
/// range zero through one hundred twenty-seven", and `§4`: "The format has
/// no storage for high-bit character codes."
#[test]
fn audit_g0_font_ch_7_glyph_indexes_above_127_are_rejected() {
    let bytes = vec![0xFFu8; CH_FONT_LEN];
    let cell_font = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    let bitmap_font =
        parse_fixed_font_body(&bytes, IBM_CH_FILE, CH_FONT_CELL_WIDTH, CH_FONT_CELL_HEIGHT)
            .unwrap();
    assert_eq!(cell_font.glyph_row(127, 0), Some(0xFF));
    assert!(bitmap_font.glyph(127).is_some());
    for code in [128u8, 200, 255] {
        assert_eq!(cell_font.glyph_row(code, 0), None, "code {code}");
        assert!(bitmap_font.glyph(code).is_none(), "code {code}");
        assert!(
            prepare_fixed_text_cell(&bitmap_font, code, TextCellStyle::default()).is_err(),
            "code {code}"
        );
    }
}

/// `formats/font-ch.md §7`: "Treat all byte values as bitmap data. There are
/// no reserved byte patterns and no metadata bytes to validate", and `§1`:
/// `IBM.CH` and `RUNES.CH` use an identical format, "The active alphabet is
/// selected by choosing which file to load".
#[test]
fn audit_g0_font_ch_1_7_every_byte_is_bitmap_data_and_both_files_share_the_format() {
    let bytes: Vec<u8> = (0..CH_FONT_LEN)
        .map(|offset| (offset % 256) as u8)
        .collect();
    let roman = parse_ch_font(&bytes, IBM_CH_FILE).unwrap();
    let runic = parse_ch_font(&bytes, RUNES_CH_FILE).unwrap();
    for code in 0..128u8 {
        for row in 0..8 {
            assert_eq!(roman.glyph_row(code, row), runic.glyph_row(code, row));
        }
    }
    // Byte 255 is plain pixels: every bit of glyph 31 row 7 is lit.
    assert_eq!(roman.glyph_row(31, 7), Some(0xFF));
}

// ---------------------------------------------------------------------------
// formats/brit-dat.md
// ---------------------------------------------------------------------------

/// `formats/brit-dat.md §2` and `§10`: the file "is exactly 52,480 bytes",
/// 205 stored chunks of 256 bytes; a reader rejects any other length.
#[test]
fn audit_g0_brit_dat_2_10_file_is_exactly_205_chunks_of_256_bytes() {
    assert_eq!(BRIT_DAT_LEN, 52_480);
    assert_eq!(BRIT_STORED_CHUNKS, 205);
    assert_eq!(CHUNK_BYTES, 256);
    assert_eq!(BRIT_DAT_LEN % CHUNK_BYTES, 0);
    let table = synthetic_britannia_chunk_index();
    for len in [
        BRIT_DAT_LEN - CHUNK_BYTES,
        BRIT_DAT_LEN + 1,
        BRIT_DAT_LEN + CHUNK_BYTES,
    ] {
        assert!(
            decode_britannia_map_bytes(&vec![0; len], &table).is_err(),
            "length {len}"
        );
    }
}

/// `formats/brit-dat.md §10`: "Each non-sentinel chunk-index entry is in
/// `0..204`", and "A complete Britannia decoder has a 256-entry chunk-index
/// table."
#[test]
fn audit_g0_brit_dat_10_chunk_index_entries_must_name_a_stored_block() {
    let good = synthetic_britannia_chunk_index();
    assert!(validate_britannia_chunk_index(&good).is_ok());

    let mut bad = good;
    // Replace the block-204 entry with 205: one past the last stored block.
    let slot = good.iter().position(|entry| *entry == 204).unwrap();
    bad[slot] = 205;
    assert!(validate_britannia_chunk_index(&bad).is_err());

    assert!(validate_britannia_chunk_index(&good[..255]).is_err());
}

/// `formats/brit-dat.md §3`: "file_offset = table_entry * 256 +
/// offset_in_chunk" with "offset_in_chunk = local_y * 16 + local_x", and the
/// table, not the slot number, names the stored block.
#[test]
fn audit_g0_brit_dat_3_cells_are_read_through_the_table_row_major_in_the_chunk() {
    let mut table = synthetic_britannia_chunk_index();
    // Chunk slot 17 is chunk (1, 1); point it at stored block 40 and give
    // slot 40 block 17, so the mapping is not the identity.
    table.swap(17, 40);
    let mut bytes = vec![0u8; BRIT_DAT_LEN];
    let local = 3 * CHUNK_SIDE + 5; // local (5, 3)
    bytes[40 * CHUNK_BYTES + local] = 0x77;
    bytes[17 * CHUNK_BYTES + local] = 0x66;

    let grid = decode_britannia_map_bytes(&bytes, &table).unwrap();

    // Slot 17 = chunk (1, 1): world (16 + 5, 16 + 3).
    assert_eq!(grid[world_cell_index(21, 19)], 0x77);
    // Slot 40 = chunk (8, 2): world (128 + 5, 32 + 3).
    assert_eq!(grid[world_cell_index(133, 35)], 0x66);
    // Every other cell of the chunk is the block's own zero bytes.
    assert_eq!(grid[world_cell_index(16, 16)], 0);
    assert_eq!(grid[world_cell_index(31, 31)], 0);
}

/// `formats/brit-dat.md §3` step 4 and `§10`: "Sentinel entries synthesize
/// tile index `1` for every cell in that logical chunk", whatever the file
/// holds.
#[test]
fn audit_g0_brit_dat_10_sentinel_chunks_are_all_deep_water() {
    let table = synthetic_britannia_chunk_index();
    let bytes = vec![0x44u8; BRIT_DAT_LEN];
    let grid = decode_britannia_map_bytes(&bytes, &table).unwrap();
    let water_slot = table
        .iter()
        .position(|entry| *entry == BRIT_WATER_SENTINEL)
        .unwrap();
    let (cx, cy) = (water_slot % 16, water_slot / 16);
    for ly in 0..CHUNK_SIDE {
        for lx in 0..CHUNK_SIDE {
            assert_eq!(
                grid[world_cell_index(cx * 16 + lx, cy * 16 + ly)],
                BRIT_DEEP_WATER_TILE
            );
        }
    }
    assert_eq!(BRIT_DEEP_WATER_TILE, 1);
    assert_eq!(BRIT_WATER_SENTINEL, 255);
}

/// `formats/brit-dat.md §9.1`: "if the chunk being loaded is a word's chunk,
/// seal when that word's saved flag is clear and leave the cell alone when
/// it is set"; for shrines "the shrine tile `0x19` is rewritten ... only when
/// the shrine belonging to this chunk is currently marked ruined".
#[test]
fn audit_g0_brit_dat_9_1_set_word_flag_opens_and_clear_shrine_flag_keeps_intact() {
    let mut grid = vec![5u8; WORLD_CELLS];
    let seal = WORD_OF_POWER_SEALS[1];
    let entrance = world_cell_index(seal.x, seal.y);
    grid[entrance] = seal.unsealed_tile;
    let (sx, sy) = WORLD_SHRINE_COORDINATES[2];
    let shrine = world_cell_index(sx, sy);
    grid[shrine] = WORLD_SHRINE_TILE;

    let mut word_flags = [0u8; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT];
    word_flags[1] = SAVE_QUEST_TILE_FLAG_HIGH_BIT;
    let shrine_flags = [0u8; SAVE_SHRINE_RUIN_FLAG_COUNT];
    apply_world_quest_tile_substitutions(&mut grid, &word_flags, &shrine_flags);

    assert_eq!(
        grid[entrance], seal.unsealed_tile,
        "spoken word leaves the entrance"
    );
    assert_eq!(
        grid[shrine], WORLD_SHRINE_TILE,
        "an intact shrine stays intact"
    );

    // Clearing the word's flag seals the same cell on the next load.
    let mut resealed = vec![5u8; WORLD_CELLS];
    resealed[entrance] = seal.unsealed_tile;
    apply_world_quest_tile_substitutions(
        &mut resealed,
        &[0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        &shrine_flags,
    );
    assert_eq!(resealed[entrance], WORD_OF_POWER_SEALED_TILE);
}

/// `formats/brit-dat.md §9.1`: "Each of the eight Words of Power owns exactly
/// one chunk of the 256-chunk grid ... and the eight owning chunks are all
/// distinct", and shrine ownership "uses a parallel eight-chunk list".
#[test]
fn audit_g0_brit_dat_9_1_the_eight_owner_chunks_of_each_list_are_distinct() {
    let word_chunks: std::collections::HashSet<(usize, usize)> = WORD_OF_POWER_SEALS
        .iter()
        .map(|seal| (seal.x / CHUNK_SIDE, seal.y / CHUNK_SIDE))
        .collect();
    assert_eq!(WORD_OF_POWER_SEALS.len(), 8);
    assert_eq!(word_chunks.len(), 8);
    for (index, seal) in WORD_OF_POWER_SEALS.iter().enumerate() {
        assert_eq!(word_of_power_chunk_owner(seal.x, seal.y), Some(index));
    }

    let shrine_chunks: std::collections::HashSet<(usize, usize)> = WORLD_SHRINE_COORDINATES
        .iter()
        .map(|(x, y)| (x / CHUNK_SIDE, y / CHUNK_SIDE))
        .collect();
    assert_eq!(WORLD_SHRINE_COORDINATES.len(), 8);
    assert_eq!(shrine_chunks.len(), 8);
    for (index, (x, y)) in WORLD_SHRINE_COORDINATES.iter().enumerate() {
        assert_eq!(shrine_chunk_owner(*x, *y), Some(index));
    }
}

/// `formats/brit-dat.md §9.1` and `§10`: the shipped file "contains exactly
/// seven dungeon-entrance cells and seven shrine cells, and zero cells of
/// either `0xDF` or `0x1A`", counted on the table-decoded grid. Needs the
/// local clean asset install.
#[test]
fn audit_g0_brit_dat_10_shipped_grid_has_seven_entrances_and_seven_shrines() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    if !game_dir.join(BRIT_DAT_FILENAME).exists() || !game_dir.join(DATA_OVL_FILENAME).exists() {
        return;
    }
    let grid = load_world_map(&game_dir, WorldPlane::Britannia).unwrap();
    let count =
        |predicate: &dyn Fn(u8) -> bool| grid.iter().filter(|tile| predicate(**tile)).count();
    assert_eq!(count(&|tile| (0x16..=0x18).contains(&tile)), 7);
    assert_eq!(count(&|tile| tile == 0xDF), 0);
    assert_eq!(count(&|tile| tile == 0x19), 7);
    assert_eq!(count(&|tile| tile == 0x1A), 0);
}

// ---------------------------------------------------------------------------
// formats/cbt.md
// ---------------------------------------------------------------------------

/// `formats/cbt.md §2` and `§9`: `DUNGEON.CBT` "is exactly 39,424 bytes and
/// splits into one hundred twelve records"; `BRIT.CBT` is 5,632 bytes and
/// sixteen records; a compatibility loader catches "a truncated or overlong
/// arena bank".
#[test]
fn audit_g0_cbt_9_both_banks_have_exact_sizes_and_reject_overlong_files() {
    assert_eq!(COMBAT_ARENA_RECORD_LEN, 352);
    assert_eq!(BRIT_CBT_FILE_LEN, 5_632);
    assert_eq!(DUNGEON_CBT_FILE_LEN, 39_424);
    assert_eq!(DUNGEON_CBT_RECORDS, 112);

    let dungeon = vec![0u8; DUNGEON_CBT_FILE_LEN];
    let bank = parse_combat_arena_bank(DUNGEON_CBT_FILE, &dungeon, DUNGEON_CBT_RECORDS).unwrap();
    assert_eq!(bank.records.len(), 112);
    assert!(bank.record(111).is_some());
    for len in [
        DUNGEON_CBT_FILE_LEN - 1,
        DUNGEON_CBT_FILE_LEN + 1,
        DUNGEON_CBT_FILE_LEN - COMBAT_ARENA_RECORD_LEN,
        DUNGEON_CBT_FILE_LEN + COMBAT_ARENA_RECORD_LEN,
    ] {
        assert!(
            parse_combat_arena_bank(DUNGEON_CBT_FILE, &vec![0; len], DUNGEON_CBT_RECORDS).is_err(),
            "length {len}"
        );
    }
    assert!(
        parse_combat_arena_bank(
            BRIT_CBT_FILE,
            &vec![0; BRIT_CBT_FILE_LEN + 1],
            BRIT_CBT_RECORDS
        )
        .is_err()
    );
}

/// `formats/cbt.md §9`: "Unknown or currently unconsumed metadata bytes must
/// be preserved byte-for-byte on round trip", for every record of a bank,
/// and the selected record "begins at `arena_index * 352`" (`§2`).
#[test]
fn audit_g0_cbt_9_a_whole_bank_round_trips_byte_for_byte() {
    let bytes: Vec<u8> = (0..BRIT_CBT_FILE_LEN)
        .map(|offset| (offset.wrapping_mul(7) % 251) as u8)
        .collect();
    let bank = parse_combat_arena_bank(BRIT_CBT_FILE, &bytes, BRIT_CBT_RECORDS).unwrap();
    let mut rebuilt = Vec::new();
    for (index, record) in bank.records.iter().enumerate() {
        let start = index * COMBAT_ARENA_RECORD_LEN;
        assert_eq!(
            record.record_bytes().as_slice(),
            &bytes[start..start + COMBAT_ARENA_RECORD_LEN],
            "record {index}"
        );
        // A metadata byte nobody reads (row 10, column 31) survives.
        assert_eq!(record.metadata(10, 31), Some(bytes[start + 10 * 32 + 31]));
        rebuilt.extend_from_slice(&record.record_bytes());
    }
    assert_eq!(rebuilt, bytes);
}

/// `formats/cbt.md §5`, the auxiliary-byte rule table for special ids three
/// through fourteen, and "`16+` | No auxiliary-byte post-write".
#[test]
fn audit_g0_cbt_5_special_id_post_write_table() {
    use DungeonRoomSpecialPostWrite as W;
    let rr = |low, high| W::RandomRange { low, high };
    let rp = |base, low, high| W::RandomRangePlus { base, low, high };
    assert_eq!(dungeon_room_special_post_write(3), rr(0, 7));
    assert_eq!(dungeon_room_special_post_write(4), rr(0, 7));
    assert_eq!(dungeon_room_special_post_write(5), rp(30, 0, 3));
    assert_eq!(dungeon_room_special_post_write(6), rp(4, 0, 2));
    assert_eq!(dungeon_room_special_post_write(7), rp(1, 0, 7));
    assert_eq!(dungeon_room_special_post_write(8), rp(1, 0, 7));
    assert_eq!(dungeon_room_special_post_write(9), rr(0, 3));
    assert_eq!(dungeon_room_special_post_write(10), rp(42, 0, 2));
    assert_eq!(dungeon_room_special_post_write(11), rp(9, 0, 5));
    assert_eq!(dungeon_room_special_post_write(12), rp(45, 0, 2));
    assert_eq!(dungeon_room_special_post_write(13), rp(1, 0, 7));
    assert_eq!(
        dungeon_room_special_post_write(14),
        W::DegenerateDrawThenConstant(1)
    );
    for id in [16u8, 0x1E, 0x1F, 0x3C, 0xE8, 0xE9, 0xEA, 0xEB] {
        assert_eq!(dungeon_room_special_post_write(id), W::None, "id {id:#04x}");
    }
}

/// `formats/cbt.md §5` (issue #308 Q3): "`0xE9` and `0xEA` take the special
/// path like the shipped `0xE8` and `0xEB`" - the excluded `0xE8` family is
/// never an ordinary placement.
#[test]
fn audit_g0_cbt_5_the_whole_e8_field_family_takes_the_special_path() {
    for source in [0xE8u8, 0xE9, 0xEA, 0xEB] {
        assert_eq!(
            dungeon_room_ordinary_setup_class(source),
            None,
            "{source:#04x}"
        );
        assert_eq!(
            DungeonRoomSetupSourceKind::from_source(source),
            DungeonRoomSetupSourceKind::SpecialPlacement(
                DungeonRoomSpecialPlacement::from_setup_id(source)
            ),
            "{source:#04x}"
        );
    }
}

/// `formats/cbt.md §5`: "Runtime `0xA?` room-helper cells use the same arena
/// selection and party-entry readback but skip the sixteen-source placement
/// scan", and the four palette draws belong to the "gated sixteen-source
/// scan".
#[test]
fn audit_g0_cbt_5_an_unscanned_room_reads_no_sources_and_draws_nothing() {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    bytes[DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN] = 0x44;
    let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
    assert!(
        record
            .dungeon_room_setup_sources_with_scan(false)
            .is_empty()
    );
    assert_eq!(record.dungeon_room_setup_sources_with_scan(true).len(), 1);

    let mut prng = 0x1234u16;
    let _ = dungeon_room_random_special_setup_ids(false, &mut prng);
    assert_eq!(prng, 0x1234);

    let setup = dungeon_room_combat_setup_from_record_for_entry(0, &record, 0, false);
    let mut prng = 0x1234u16;
    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 1, &mut prng);
    assert_eq!(prng, 0x1234);
    assert_eq!(instance.placed_count, 0);
}

/// A one-source dungeon-room record for the `formats/cbt.md §5` vectors.
fn g0_room_record(sources: &[(u8, u8, u8)]) -> CombatArenaRecord {
    let mut bytes = vec![0u8; COMBAT_ARENA_RECORD_LEN];
    for (slot, (source, x, y)) in sources.iter().enumerate() {
        bytes[DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + slot] = *source;
        bytes[DUNGEON_ROOM_SOURCE_X_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + slot] = *x;
        bytes[DUNGEON_ROOM_SOURCE_Y_ROW * COMBAT_ARENA_ROW_STRIDE
            + DUNGEON_ROOM_SOURCE_COLUMN
            + slot] = *y;
    }
    CombatArenaRecord::from_record_bytes(&bytes).unwrap()
}

/// The state after the four unconditional palette draws.
fn g0_after_palette(seed: u16) -> u16 {
    let mut state = seed;
    let _ = dungeon_room_random_special_setup_ids(true, &mut state);
    state
}

/// `formats/cbt.md §5`, deterministic setup vector one: "Special id `1` at
/// `(2,4)`, dungeon Z `3` | One active object with raw tile pair `1`,
/// `(X,Y,Z) = (2,4,3)`, auxiliary `16`, no descriptor, no PRNG draw."
#[test]
fn audit_g0_cbt_5_vector_special_id_one_at_level_three() {
    let record = g0_room_record(&[(1, 2, 4)]);
    let setup = dungeon_room_combat_setup_from_record(0, &record);
    let mut prng = 0x4321u16;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 3, &mut prng);

    let object = instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS];
    assert_eq!((object.type_byte, object.tile), (1, 1));
    assert_eq!((object.x, object.y, object.z), (2, 4, 3));
    assert_eq!(object.aux1, 16);
    assert_eq!(object.aux3, COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR);
    assert!(instance.actors.iter().all(|actor| actor.is_empty()));
    assert_eq!(prng, g0_after_palette(0x4321), "no draw beyond the palette");
}

/// `formats/cbt.md §5`, vector two: "Special id `14` at `(6,7)` | One
/// sandalwood-box active object with raw tile pair `14`, auxiliary `1`, no
/// descriptor; consume one degenerate `random_range(0,0)` draw."
#[test]
fn audit_g0_cbt_5_vector_special_id_fourteen_consumes_the_degenerate_draw() {
    let record = g0_room_record(&[(14, 6, 7)]);
    let setup = dungeon_room_combat_setup_from_record(0, &record);
    let mut prng = 0x4321u16;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 2, &mut prng);

    let object = instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS];
    assert_eq!((object.type_byte, object.tile), (14, 14));
    assert_eq!((object.x, object.y), (6, 7));
    assert_eq!(object.aux1, 1);
    assert!(instance.actors.iter().all(|actor| actor.is_empty()));
    let mut expected = g0_after_palette(0x4321);
    let _ = u5_prng_range_u16(&mut expected, 0, 0);
    assert_eq!(prng, expected);
}

/// `formats/cbt.md §5`, vector three: "Special id `0x1E` at `(5,5)` | One
/// corpse active object with raw tile pair and retained auxiliary `0x1E`, no
/// descriptor and no post-write draw."
#[test]
fn audit_g0_cbt_5_vector_corpse_id_keeps_its_raw_auxiliary_byte() {
    let record = g0_room_record(&[(0x1E, 5, 5)]);
    let setup = dungeon_room_combat_setup_from_record(0, &record);
    let mut prng = 0x4321u16;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 2, &mut prng);

    let object = instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS];
    assert_eq!(
        (object.type_byte, object.tile, object.aux1),
        (0x1E, 0x1E, 0x1E)
    );
    assert_eq!((object.x, object.y), (5, 5));
    assert!(instance.actors.iter().all(|actor| actor.is_empty()));
    assert_eq!(prng, g0_after_palette(0x4321));
}

/// `formats/cbt.md §5`, vector four: "Source `0xEC`, whose low-bit-zero
/// palette draw selected class `24` | Ordinary Slime placement: allocate
/// descriptor and active object, derive its renderer tile from class 24,
/// consume the ordinary speed draw, and perform no special auxiliary
/// post-write." `combat.md §5` publishes the seed: from state `0x80DA` "the
/// four palette indexes are `[5,1,7,4]` (classes `[24,21,24,33]`) and leave
/// state `0x752C`".
#[test]
fn audit_g0_cbt_5_vector_vermin_source_places_an_ordinary_slime() {
    let mut palette_state = 0x80DAu16;
    assert_eq!(
        dungeon_room_random_special_setup_ids(true, &mut palette_state),
        [24, 21, 24, 33]
    );
    assert_eq!(palette_state, 0x752C);

    let record = g0_room_record(&[(0xEC, 3, 3)]);
    let setup = dungeon_room_combat_setup_from_record(0, &record);
    let mut prng = 0x80DAu16;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 1, &mut prng);

    let slime = combat_class_stats(24).unwrap();
    let object = instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS];
    assert_eq!(object.tile, 24 * 4 + 0x40);
    assert_eq!(
        object.aux1, slime.max_hp,
        "the class's starting HP, no post-write"
    );
    let actor = instance.actors[COMBAT_PARTY_ACTOR_SLOTS];
    assert!(!actor.is_empty());
    assert_eq!(actor.owner_target_class, 24);
    let mut expected = 0x752Cu16;
    let _ = u5_prng_range_u16(
        &mut expected,
        u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW),
        u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH),
    );
    assert_eq!(
        prng, expected,
        "the four palette draws, then one speed draw"
    );
}

/// `formats/cbt.md §5` ordinary path: "sets the active-object tile pair to
/// `setup_class * 4 + 0x40`. Its auxiliary byte receives the class's
/// starting HP", and "Both paths ... stamp the current dungeon level into
/// it". A source with non-zero low bits still derives its tile from the class.
#[test]
fn audit_g0_cbt_5_ordinary_tile_is_derived_from_the_class_not_the_source_byte() {
    let record = g0_room_record(&[(0x47, 4, 9)]);
    let setup = dungeon_room_combat_setup_from_record(0, &record);
    let mut prng = 0u16;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 5, &mut prng);

    let object = instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS];
    assert_eq!(dungeon_room_ordinary_setup_class(0x47), Some(1));
    assert_eq!((object.type_byte, object.tile), (0x44, 0x44));
    assert_eq!((object.x, object.y, object.z), (4, 9, 5));
    assert_eq!(object.aux1, combat_class_stats(1).unwrap().max_hp);
    let actor = instance.actors[COMBAT_PARTY_ACTOR_SLOTS];
    assert_eq!(
        usize::from(actor.active_object_slot),
        COMBAT_PARTY_ACTOR_SLOTS
    );
}

/// `formats/cbt.md §5`: "ordinary and special draws interleave in
/// source-index order after the four unconditional palette draws", each
/// special id consuming its post-write draw "at that same point in the scan".
#[test]
fn audit_g0_cbt_5_ordinary_and_special_draws_interleave_in_source_order() {
    let record = g0_room_record(&[(3, 1, 1), (0x44, 2, 2), (4, 3, 3)]);
    let setup = dungeon_room_combat_setup_from_record(0, &record);
    let seed = 0x0BADu16;
    let mut prng = seed;

    let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 0, &mut prng);

    let mut expected = g0_after_palette(seed);
    let potion_aux = u5_prng_range_u16(&mut expected, 0, 7) as u8;
    let speed_roll = u5_prng_range_u16(
        &mut expected,
        u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW),
        u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH),
    ) as u8;
    let scroll_aux = u5_prng_range_u16(&mut expected, 0, 7) as u8;
    assert_eq!(prng, expected);
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS].aux1,
        potion_aux
    );
    assert_eq!(
        instance.active_objects[COMBAT_PARTY_ACTOR_SLOTS + 2].aux1,
        scroll_aux
    );
    let actor = instance.actors[COMBAT_PARTY_ACTOR_SLOTS];
    let stats = combat_class_stats(1).unwrap();
    assert_eq!(
        actor.base_step,
        combat_placement_base_step(stats.speed_seed, speed_roll)
    );
}

// ---------------------------------------------------------------------------
// systems/combat.md
// ---------------------------------------------------------------------------

/// `combat.md §5`, deterministic shuffle vector: from the sixteen-swap's
/// state `0x01C0`, "For Giant Rat, the next `1..10` draw returns nine and
/// leaves state `0x80DA`", and "For Bat, the corresponding `1..16` draw
/// returns eleven, then the exact-count sentinel overwrites it with sixteen
/// while state still advances to `0x80DA`".
#[test]
fn audit_g0_combat_5_count_roll_vector_after_the_sixteen_swap() {
    let mut rat = 0x01C0u16;
    assert_eq!(u5_prng_range_u16(&mut rat, 1, 10), 9);
    assert_eq!(rat, 0x80DA);

    let mut bat = 0x01C0u16;
    assert_eq!(u5_prng_range_u16(&mut bat, 1, 16), 11);
    assert_eq!(bat, 0x80DA);
    let bat_stats = combat_class_stats(COMBAT_CLASS_BAT).unwrap();
    assert_eq!(bat_stats.default_spawn_count, 16);
    assert_eq!(resolve_terrain_combat_setup_count(16, 0, 11, 0, false), 16);
}

/// `combat.md §5.2`: the speed variation "subtracts 4 to give an offset in
/// `[-4, +3]` ... If the result **exceeds 30**, the engine reverts to the
/// class's unmodified speed rating". For a speed-30 class "Its reachable base
/// steps are 26, 27, 28 and 29 at one chance in eight each, and 30 at four
/// chances in eight"; and "A very slow class is protected from wrapping below
/// zero by the same revert".
#[test]
fn audit_g0_combat_5_2_speed_variation_reverts_instead_of_clamping() {
    let steps: Vec<u8> = (0..8)
        .map(|roll| combat_placement_base_step(30, roll))
        .collect();
    assert_eq!(steps, vec![26, 27, 28, 29, 30, 30, 30, 30]);
    let phases: Vec<u8> = steps.iter().map(|step| 36 - step).collect();
    assert_eq!(phases, vec![10, 9, 8, 7, 6, 6, 6, 6]);

    assert_eq!(
        combat_placement_base_step(28, 7),
        28,
        "28 + 3 = 31 reverts to 28"
    );
    assert_eq!(combat_placement_base_step(28, 6), 30, "28 + 2 = 30 is kept");
    assert_eq!(
        combat_placement_base_step(2, 0),
        2,
        "2 - 4 would wrap; it reverts"
    );
    assert_eq!(combat_placement_base_step(2, 2), 0, "2 - 2 = 0 is kept");
}

/// `combat.md §8`, Shape B: `E` Enter, `F` Fire, `H` Hole up, `I` Ignite
/// (printed as `Ignite torch`), `L` Look, `M` Mix, `N` New order, `Q` Quit
/// and `V` View take the `"-Not here"` tail; "The responder always
/// re-prompts without cost", and `§8.1` gives every shape-B letter the full
/// re-prompt with the whole banner.
#[test]
fn audit_g0_combat_8_every_not_here_letter_refuses_and_reprints_the_banner() {
    let game_dir = std::path::Path::new(".");
    for (key, label) in [
        ('F', "Fire"),
        ('H', "Hole up"),
        ('I', "Ignite torch"),
        ('L', "Look"),
        ('M', "Mix"),
        ('N', "New order"),
        ('V', "View"),
    ] {
        let mut state = combat_player_command_state(8, 5);
        let torches = state.torches;
        assert_eq!(
            handle_play_key_input(&mut state, key, "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            state.message,
            format!("{label}-Not here\n\nAvatar, armed with bare hands:\n"),
            "key {key}"
        );
        assert_eq!(state.pending_combat_actor_slot, Some(0), "key {key}");
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (5, 5));
        assert_eq!(
            state.torches, torches,
            "key {key}: no torch counter is touched"
        );
    }
}

/// A combat-frame snapshot carrying the entry-mode high bit and the
/// first-party-exit record, for the `combat.md §3` and `§14` vectors.
fn g0_combat_snapshot(
    state: &PlayState,
    high_bit: bool,
    established: Option<u8>,
) -> CombatFrameSnapshot {
    CombatFrameSnapshot {
        area: state.area,
        player: state.player,
        active_objects: state.active_objects.clone(),
        active_player: state.active_player,
        combat_terrain: state.combat_terrain,
        dungeon_room_clear_on_success: None,
        enter_endgame_after_successful_combat: false,
        endgame_messages: None,
        endgame_tableau_map: None,
        encounter_mode_high_bit: high_bit,
        suppress_controlled_faint_sleep_tick: false,
        exit_announced: false,
        established_exit_direction_code: established,
        dungeon_post_combat: None,
        any_foe_seated: true,
    }
}

/// `combat.md §3`, edge vector: "Actor at `(0,5)` requests West while the
/// transport marker is ship-family | `West\n\nStay with ship!\n`; no tone;
/// actor remains."
#[test]
fn audit_g0_combat_3_ship_family_edge_refusal_is_silent_and_keeps_the_actor() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 99,
        skiffs: 1,
    };
    state.combat_actors[0].x = 0;
    state.active_objects[0].x = 0;
    let serial = state.sound_effect_serial;

    assert_eq!(
        handle_play_key_input(&mut state, char::from(INPUT_CODE_WEST), "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "West\n\nStay with ship!\n");
    assert!(state.sound_effects_after(serial).is_empty(), "no tone");
    assert!(!state.combat_actors[0].is_empty(), "the actor remains");
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (0, 5));
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

/// `combat.md §3`, edge vector: "Constrained encounter already records North;
/// a party actor at the east edge requests East |
/// `East\n\nAll must use the same exit!\n`; bump tone; actor remains and the
/// stored direction stays North."
#[test]
fn audit_g0_combat_3_constrained_encounter_refuses_a_second_exit_direction() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(5, 9);
    let edge = (COMBAT_ARENA_SIDE - 1) as u8;
    state.combat_actors[0].x = edge;
    state.active_objects[0].x = usize::from(edge);
    state.combat_frame_snapshot = Some(g0_combat_snapshot(
        &state,
        true,
        Some(COMBAT_DIRECTION_NORTH),
    ));
    let serial = state.sound_effect_serial;

    assert_eq!(
        handle_play_key_input(&mut state, 'd', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.message, "East\n\nAll must use the same exit!\n");
    assert!(
        !state.sound_effects_after(serial).is_empty(),
        "the bump tone plays"
    );
    assert!(!state.combat_actors[0].is_empty(), "the actor remains");
    assert_eq!(state.combat_actors[0].x, edge);
    assert_eq!(
        state
            .combat_frame_snapshot
            .as_ref()
            .unwrap()
            .established_exit_direction_code,
        Some(COMBAT_DIRECTION_NORTH)
    );
    assert_eq!(state.pending_combat_actor_slot, Some(0));
}

/// `combat.md §14`: with a live party-side descriptor and the entry mode's
/// high bit set, Escape appends "`-Not here!` plus a newline and re-prompt[s]
/// the same actor at no cost" - "the refusal is then unconditional, even
/// after victory" - and `§8.1` reprints the banner after a declined Escape.
#[test]
fn audit_g0_combat_14_escape_in_a_room_fight_is_refused_not_here() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(10, 10);
    state.combat_frame_snapshot = Some(g0_combat_snapshot(&state, true, None));

    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        state.message,
        "Escape-Not here!\n\nAvatar, armed with bare hands:\n"
    );
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, Some(0));

    // With every foe gone the room refusal still stands.
    state.combat_actors[8].mark_dead();
    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        state.message,
        "Escape-Not here!\n\nAvatar, armed with bare hands:\n"
    );
    assert!(state.combat_active);
}

// ---------------------------------------------------------------------------
// systems/hidden-treasures.md
// ---------------------------------------------------------------------------

/// The scene byte whose resident name is `name` (`catalogs/gazetteer.md`).
fn g0_scene_named(name: &str) -> u8 {
    (1..=40u8)
        .find(|scene| town_resident_name(*scene) == Some(name))
        .unwrap_or_else(|| panic!("no scene named {name}"))
}

/// A town state one cell west of `(x, y)`, facing east.
fn g0_town_search_state(location: &str, floor: i8, x: usize, y: usize) -> PlayState {
    let mut state = test_state(open_grid(), x - 1, y);
    state.area = Area::Town {
        scene: Scene::new(g0_scene_named(location)).unwrap(),
        floor,
    };
    state.player.facing = Direction::East;
    state
}

/// `hidden-treasures.md §2.1`: a staged record "is narrated with one line
/// from a fixed seventeen-name vocabulary, selected by the record's pickup
/// class", one row per class in the published table. These eight classes
/// had no Search test.
#[test]
fn audit_g0_hidden_treasures_2_1_each_pickup_class_narrates_its_row() {
    let dir = debug_game_dir();
    for (record, location, floor, x, y, row) in [
        (21usize, "Moonglow", 0i8, 23usize, 5usize, "a potion!"),
        (27, "Moonglow", 0, 27, 3, "some torches!"),
        (28, "Moonglow", 0, 27, 29, "a ring!"),
        (40, "Yew", 0, 13, 2, "a moldy corpse!"),
        (48, "Minoc", 0, 27, 8, "a sack of gold!"),
        (55, "Skara Brae", 0, 5, 2, "a rotting body!"),
        (92, "Paws", 0, 3, 12, "a ring of keys!"),
        (93, "Cove", 0, 15, 1, "an amulet!"),
    ] {
        let mut state = g0_town_search_state(location, floor, x, y);
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched,
            "record {record}"
        );
        assert!(state.fixed_hidden_treasure_found(record), "record {record}");
        assert_eq!(
            state.message,
            format!("{SEARCH_PREAMBLE}{row}"),
            "record {record}"
        );
    }
    let _ = fs::remove_dir_all(dir);
}

/// `hidden-treasures.md §2.1`: "Search compares each record's scene, Z, X
/// and Y for exact equality". Record 18 is Moonglow, Z `0`, `(8, 5)`; the
/// same cell one floor up stages nothing.
#[test]
fn audit_g0_hidden_treasures_2_1_a_record_needs_its_own_floor() {
    let dir = debug_game_dir();
    let mut state = g0_town_search_state("Moonglow", 1, 8, 5);

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert!(!state.fixed_hidden_treasure_found(18));
    assert!(
        state
            .active_objects
            .iter()
            .all(|object| object.fixed_hidden_treasure_record().is_none())
    );
    assert_eq!(state.message, SEARCH_NOTHING_FOUND);
    let _ = fs::remove_dir_all(dir);
}

/// `hidden-treasures.md §2`, record 13: "Acquiring any key closes the
/// record; spending every key opens it again. The scan does not modify the
/// keys counter." `§3`: its Get grants "Keys +9" and prints `9 keys!`.
#[test]
fn audit_g0_hidden_treasures_2_record_13_closes_on_any_key_and_reopens_at_zero() {
    let dir = debug_game_dir();
    let mut state = g0_town_search_state("Lord Blackthorn's Castle", -1, 8, 6);
    state.keys = 0;

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );
    assert_eq!(state.keys, 0, "the scan does not modify the keys counter");
    assert_eq!(
        state.get_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Got
    );
    assert_eq!(state.keys, 9);
    assert_eq!(state.message, "\n9 keys!");

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked,
        "holding keys closes the record"
    );
    assert_eq!(state.keys, 9);

    state.keys = 0;
    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched,
        "spending every key opens it again"
    );
    assert!(!state.fixed_hidden_treasure_found(13));
    let _ = fs::remove_dir_all(dir);
}

/// `hidden-treasures.md §2`, record 14: "Factory value is zero, which matches
/// no calendar day, so the record is available on the first search."
#[test]
fn audit_g0_hidden_treasures_2_record_14_factory_cookie_matches_no_day() {
    assert_eq!(FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY, 0);
    let dir = debug_game_dir();
    for day in [1u8, 14, 28] {
        let mut state = g0_town_search_state("Minoc", 0, 2, 2);
        state.clock = GameClock::with_date(139, 4, day, 12, 0).unwrap();
        state.fixed_hidden_treasure_daily_day = FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY;
        assert_eq!(
            state.search_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Searched,
            "day {day}"
        );
        assert_eq!(state.fixed_hidden_treasure_daily_day, day);
    }
    let _ = fs::remove_dir_all(dir);
}

/// `hidden-treasures.md §3`: "record 12 sets the HMS Cape plans and adds no
/// scroll, printing `The plans for the HMS Cape!`" - at state 255 "the HMS
/// Cape plans flag is set instead and no scroll counter changes".
#[test]
fn audit_g0_hidden_treasures_3_record_12_sets_the_plans_and_adds_no_scroll() {
    let mut state = test_state(open_grid(), 4, 8);
    state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = 0;
    let scrolls = state.scroll_stock;
    state
        .active_objects
        .push(ActiveObject::fixed_hidden_treasure_pickup(12, 5, 8, 0));

    assert_eq!(
        state.get_fixed_hidden_treasure_pickup_at(5, 8),
        Some(MoveOutcome::Got)
    );

    assert_ne!(state.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX], 0);
    assert_eq!(state.scroll_stock, scrolls);
    assert_eq!(state.message, "\nThe plans for the HMS Cape!");
}

/// `hidden-treasures.md §3`, the grant caps: gold "capped at 9999"; a potion,
/// a scroll and an equipment item "capped at 99"; keys "Capped at 99"; gems
/// and torches "capped at 99".
#[test]
fn audit_g0_hidden_treasures_3_grants_respect_the_published_caps() {
    fn take(state: &mut PlayState, record: usize) {
        state
            .active_objects
            .push(ActiveObject::fixed_hidden_treasure_pickup(record, 5, 8, 0));
        assert_eq!(
            state.get_fixed_hidden_treasure_pickup_at(5, 8),
            Some(MoveOutcome::Got),
            "record {record}"
        );
    }
    let mut state = test_state(open_grid(), 4, 8);
    state.gold = 9990;
    take(&mut state, 48); // Gold +99
    assert_eq!(state.gold, 9999);

    state.potion_stock[5] = 99;
    take(&mut state, 21); // Purple potion
    assert_eq!(state.potion_stock[5], 99);

    state.scroll_stock[5] = 99;
    take(&mut state, 24); // Summon Daemon scroll
    assert_eq!(state.scroll_stock[5], 99);

    state.equipment_stock[10] = 99;
    take(&mut state, 19); // Leather Armour
    assert_eq!(state.equipment_stock[10], 99);

    state.keys = 97;
    take(&mut state, 92); // Keys +5
    assert_eq!(state.keys, 99);

    state.gems = 97;
    take(&mut state, 61); // Gems +5
    assert_eq!(state.gems, 99);

    state.torches = 97;
    take(&mut state, 59); // Torches +9
    assert_eq!(state.torches, 99);
}

/// `hidden-treasures.md §3`, "Shared cells" and "Body cells with an item":
/// on Yew's `(16, 3)` record 43 (a rotting body) stages first and record 45
/// (the Spear) second, "a third prints `nothing of note.`"; "Get skips the
/// body and takes the item, printing `Spear!` ... The body stays on the cell;
/// a second Get there prints the tile cascade's `Nothing to get!`".
#[test]
fn audit_g0_hidden_treasures_3_body_cell_stages_in_order_and_get_takes_the_item() {
    let dir = debug_game_dir();
    let mut state = g0_town_search_state("Yew", 0, 16, 3);
    let spears = state.equipment_stock[21];

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );
    assert_eq!(state.message, format!("{SEARCH_PREAMBLE}a rotting body!"));
    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );
    assert_eq!(state.message, format!("{SEARCH_PREAMBLE}a weapon!"));
    assert!(state.fixed_hidden_treasure_found(43));
    assert!(state.fixed_hidden_treasure_found(45));
    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, SEARCH_NOTHING_FOUND);

    assert_eq!(
        state.get_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Got
    );
    assert_eq!(state.message, "\nSpear!");
    assert_eq!(state.equipment_stock[21], spears + 1);
    assert!(
        state
            .active_objects
            .iter()
            .any(|object| object.fixed_hidden_treasure_record() == Some(43)),
        "the body stays on the cell"
    );

    let _ = state.get_facing_with_game_dir(&dir).unwrap();
    assert!(
        state.message.contains("Nothing to get!"),
        "{:?}",
        state.message
    );
    let _ = fs::remove_dir_all(dir);
}

/// `hidden-treasures.md §3`: "Each staged record takes the lowest empty
/// active-object slot".
#[test]
fn audit_g0_hidden_treasures_3_a_staged_record_takes_the_lowest_empty_slot() {
    let dir = debug_game_dir();
    let mut state = g0_town_search_state("Moonglow", 0, 8, 5);
    let filler = ActiveObject {
        type_byte: 0x50,
        tile: 0x50,
        x: 1,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.active_objects.truncate(1);
    state.active_objects.push(filler);
    state.active_objects.push(ActiveObject::empty());
    state.active_objects.push(filler);

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );

    assert_eq!(
        state.active_objects[2].fixed_hidden_treasure_record(),
        Some(18)
    );
    assert_eq!(state.active_objects[1], filler);
    assert_eq!(state.active_objects[3], filler);
    let _ = fs::remove_dir_all(dir);
}

/// `combat.md §11.1`, "Messages that leave a row open": "Blink reappearance |
/// `\n<monster> reappears!`, no trailing newline", and `§9`: success "toggles
/// the actor's phase/hidden flag and linked visual tile between visible and
/// hidden". A second blink of a hidden Ghost brings it back.
#[test]
fn audit_g0_combat_11_1_a_second_blink_reappears_and_leaves_the_row_open() {
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
    state.apply_combat_ai_blink_special(actor_slot).unwrap();
    assert_eq!(state.message, "\nGhost disappears!");

    state.message.clear();
    let application = state.apply_combat_ai_blink_special(actor_slot).unwrap();

    assert!(matches!(
        application,
        CombatAiSpecialApplication::Blink {
            visibility: CombatLinkedVisibilityOutcome {
                visibility: CombatLinkedVisibility::Visible,
                ..
            },
            ..
        }
    ));
    assert!(!state.combat_actors[actor_slot].is_phase_suppressed());
    assert_ne!(
        state.active_objects[actor_slot].tile,
        COMBAT_HIDDEN_ACTIVE_OBJECT_TILE
    );
    assert_eq!(state.message, "\nGhost reappears!");
    assert!(state.combat_transcript_row_open);
}

/// `hidden-treasures.md §2`, record 15: "Stages only when that counter is
/// zero and the searched tile is not occupied by an NPC. The skip condition
/// is therefore \"counter non-zero **or** an NPC is present\"." The Glass
/// Sword counter is zero here; the NPC alone refuses it.
#[test]
fn audit_g0_hidden_treasures_2_record_15_is_refused_while_an_npc_stands_there() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 63, 80);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.player.facing = Direction::East;
    state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 0;
    let floor = state.current_floor().expect("Britannia has a floor byte") as u8;
    state.npcs.push(RuntimeNpc {
        slot: 1,
        type_byte: 0x50,
        dialog_id: 0,
        schedule: [0; NPC_SCHEDULE_RECORD_LEN],
        state: NPC_STATE_IDLE,
        x: 64,
        y: 80,
        z: floor,
        cached_wp: 0,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    });

    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    assert!(
        state
            .active_objects
            .iter()
            .all(|object| object.fixed_hidden_treasure_record() != Some(15))
    );

    state.npcs.clear();
    assert_eq!(
        state.search_facing_with_game_dir(&dir).unwrap(),
        MoveOutcome::Searched
    );
    let _ = fs::remove_dir_all(dir);
}
