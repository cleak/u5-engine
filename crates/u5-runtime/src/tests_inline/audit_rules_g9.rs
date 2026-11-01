// Conformance tests for the closed audit, cluster g9: rules the engine
// implements but no test asserted. Spec files covered:
// `systems/town-mode.md`, `systems/commands.md`, `systems/timing.md`,
// `systems/vehicles.md`, `formats/tiles.md`, `catalogs/npc-roster.md`,
// `formats/dungeon-dat.md`, `systems/disk-prompt.md`, `formats/story-dat.md`.
// Every test cites the file and section it asserts. No unpublished game text
// appears here: literals are the ones the spec itself publishes.

fn g9_asset_dir_with(file: &str) -> Option<std::path::PathBuf> {
    let dir = crate::test_fixtures::configured_original_asset_dir()?;
    dir.join(file).is_file().then_some(dir)
}

fn g9_scratch_dir(tag: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("u5-g9-{tag}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ---------------------------------------------------------------------------
// systems/disk-prompt.md
// ---------------------------------------------------------------------------

#[test]
fn g9_disk_prompt_s6_boot_selects_program_disk_plain_console_one_floppy() {
    // `disk-prompt.md §6`: "At boot the engine sets the required disk to the
    // Program disk, the presentation context to plain console, and the floppy
    // count to one", and records the current drive for the Program disk.
    for drive in [DosDrive::FIRST_FLOPPY, DosDrive::SINGLE_DIRECTORY] {
        let session = DiskPromptSession::boot(drive);
        assert_eq!(session.required_disk(), RequiredDisk::Program);
        assert_eq!(session.presentation(), DiskPromptPresentation::PlainConsole);
        assert_eq!(session.floppy_count(), 1);
        assert_eq!(session.drive_for(RequiredDisk::Program), Some(drive));
        assert_eq!(session.handler(), DiskErrorHandler::InsertDiskPrompt);
        assert_eq!(session.already_prompted(), None);
    }
}

#[test]
fn g9_disk_prompt_s5_step1_two_floppies_first_error_is_a_silent_retry_on_the_other_drive() {
    // `disk-prompt.md §5` step 1: with a floppy letter recorded and more than
    // one floppy known, the handler swaps the recorded letter to the other
    // floppy; when the cache does not name the disk it sets the cache and
    // returns without showing anything.
    let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
    // Learn a second floppy by answering B for the Britannia disk.
    session.request_disk(REQUIRED_DISK_BRITANNIA_INDEX).unwrap();
    assert!(session.begin_error_handling());
    let _ = session.current_error_action();
    assert_eq!(
        session.record_prompt_key(b'B', true),
        PromptSelectionOutcome::Accepted
    );
    session.end_error_handling();
    assert_eq!(session.floppy_count(), 2);

    // A known disk request invalidates the cache (§7), so the next error is
    // a first attempt.
    assert_eq!(
        session.request_disk(REQUIRED_DISK_PROGRAM_INDEX).unwrap(),
        DiskRequestOutcome::SelectKnownDrive(DosDrive::FIRST_FLOPPY)
    );
    assert_eq!(session.already_prompted(), None);
    assert!(session.begin_error_handling());
    assert_eq!(
        session.current_error_action(),
        DiskErrorAction::SilentRetry {
            drive: DosDrive::SECOND_FLOPPY
        }
    );
    assert_eq!(session.already_prompted(), Some(RequiredDisk::Program));
    assert_eq!(
        session.drive_for(RequiredDisk::Program),
        Some(DosDrive::SECOND_FLOPPY)
    );
    session.end_error_handling();

    // Second failure with the cache naming the disk: the known-drive prompt.
    assert!(session.begin_error_handling());
    assert_eq!(
        session.current_error_action(),
        DiskErrorAction::Prompt {
            disk: RequiredDisk::Program,
            known_drive: Some(DosDrive::FIRST_FLOPPY),
            presentation: DiskPromptPresentation::PlainConsole,
        }
    );
}

#[test]
fn g9_disk_prompt_s5_unknown_drive_form_rejects_a_nonletter_and_uppercases_letters() {
    // `disk-prompt.md §5` step 3: "the handler uppercases the key and
    // proposes it to the drive selector. The selector rejects a nonletter
    // before asking the operating system to change drives."
    let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
    session.request_disk(REQUIRED_DISK_U4_PLAYER_INDEX).unwrap();
    assert!(session.begin_error_handling());
    assert!(matches!(
        session.current_error_action(),
        DiskErrorAction::Prompt { known_drive: None, .. }
    ));
    assert_eq!(
        session.record_prompt_key(b'1', true),
        PromptSelectionOutcome::RePrompt
    );
    assert_eq!(session.drive_for(RequiredDisk::UltimaIvPlayer), None);
    assert_eq!(
        session.record_prompt_key(b'a', true),
        PromptSelectionOutcome::Accepted
    );
    assert_eq!(
        session.drive_for(RequiredDisk::UltimaIvPlayer),
        Some(DosDrive::FIRST_FLOPPY)
    );
    // Answering the first floppy letter does not record a second drive.
    assert_eq!(session.floppy_count(), 1);
    // Only index 3 propagates to the Britannia entry.
    assert_eq!(session.drive_for(RequiredDisk::Britannia), None);
}

#[test]
fn g9_disk_prompt_s5_picture_context_is_carried_into_the_prompt() {
    // `disk-prompt.md §5` step 3 / §2: the presentation byte only decides
    // whether the prompt is drawn over a picture.
    let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
    session.set_presentation(DiskPromptPresentation::PictureOverlay);
    session.request_disk(REQUIRED_DISK_BRITANNIA_INDEX).unwrap();
    assert!(session.begin_error_handling());
    assert_eq!(
        session.current_error_action(),
        DiskErrorAction::Prompt {
            disk: RequiredDisk::Britannia,
            known_drive: None,
            presentation: DiskPromptPresentation::PictureOverlay,
        }
    );
}

#[test]
fn g9_disk_prompt_s3_write_protect_handler_is_a_distinct_family() {
    // `disk-prompt.md §3` / §8: the write wrapper installs the write-protect
    // prompt, and the recursion guard restores it afterwards.
    let mut session = DiskPromptSession::single_directory();
    session.install_write_error_handler();
    assert!(session.begin_error_handling());
    assert_eq!(session.handler(), DiskErrorHandler::ImmediateReturnGuard);
    assert_eq!(
        session.current_error_action(),
        DiskErrorAction::WriteProtectPrompt {
            presentation: DiskPromptPresentation::PlainConsole
        }
    );
    session.end_error_handling();
    assert_eq!(session.handler(), DiskErrorHandler::WriteProtectPrompt);
    session.restore_insert_disk_handler();
    assert_eq!(session.handler(), DiskErrorHandler::InsertDiskPrompt);
}

#[test]
fn g9_disk_prompt_s7_unknown_drive_request_does_not_prompt_or_touch_the_cache() {
    // `disk-prompt.md §7`: the requester "does not prompt"; it selects and
    // invalidates the cache only when the letter is already known.
    let mut session = DiskPromptSession::boot(DosDrive::FIRST_FLOPPY);
    assert_eq!(
        session.request_disk(REQUIRED_DISK_ALIAS_B_INDEX).unwrap(),
        DiskRequestOutcome::DriveUnknown
    );
    assert_eq!(session.required_disk(), RequiredDisk::Britannia);
    assert_eq!(session.handler(), DiskErrorHandler::InsertDiskPrompt);
}

// ---------------------------------------------------------------------------
// formats/story-dat.md
// ---------------------------------------------------------------------------

#[test]
fn g9_story_dat_s5_unknown_printable_bytes_pass_through() {
    // `story-dat.md §5`: "Unknown printable bytes should be passed through to
    // the proportional-font renderer."
    let mut bytes = Vec::new();
    for _ in 0..20 {
        bytes.extend_from_slice(b"{a~|b\0");
    }
    bytes.push(0);
    let records = parse_story_records(&bytes).unwrap();
    assert_eq!(records.record(0), Some("{a~|b"));
}

#[test]
fn g9_story_dat_s6_empty_trailer_is_not_a_twenty_first_page() {
    // `story-dat.md §2` / §6: the empty trailer "should not be exposed as a
    // twenty-first story page".
    let mut bytes = Vec::new();
    for _ in 0..20 {
        bytes.extend_from_slice(b"{p\0");
    }
    bytes.push(0);
    let records = parse_story_records(&bytes).unwrap();
    assert_eq!(records.iter().count(), STORY_DAT_RECORDS);
    assert_eq!(records.record(20), None);
}

#[test]
fn g9_story_dat_s2_shipped_records_have_the_published_lengths_and_positions() {
    // `story-dat.md §2`: the twenty positions and text lengths and a
    // 11,679-byte file. (§3's "Every record in the shipped file begins with
    // `{`" is not asserted: several shipped records open with a line feed
    // before their first `{`; reported to the spec owner.)
    let Some(dir) = g9_asset_dir_with(STORY_DAT_FILE) else {
        return;
    };
    let raw = fs::read(dir.join(STORY_DAT_FILE)).unwrap();
    assert_eq!(raw.len(), STORY_DAT_LEN);
    let positions = [
        0usize, 273, 971, 1424, 2051, 2756, 3437, 3619, 4022, 4467, 4906, 5310, 5678, 6443, 7066,
        7803, 8562, 9291, 10112, 10903,
    ];
    let lengths = [
        272usize, 697, 452, 626, 704, 680, 181, 402, 444, 438, 403, 367, 764, 622, 736, 758, 728,
        820, 790, 774,
    ];
    let records = load_story_records(&dir).unwrap().unwrap();
    for (index, (&position, &length)) in positions.iter().zip(lengths.iter()).enumerate() {
        assert_ne!(raw[position], 0, "record {index} is non-empty");
        assert!(records.record(index).unwrap().contains('{'), "record {index} carries an indent");
        assert_eq!(raw[position + length], 0, "record {index} terminator");
        assert_eq!(records.record(index).unwrap().len(), length, "record {index}");
    }
    // "A twenty-first, empty record begins at `11678`".
    assert_eq!(raw[11678], 0);
}

// ---------------------------------------------------------------------------
// formats/dungeon-dat.md
// ---------------------------------------------------------------------------

#[test]
fn g9_dungeon_dat_s1_s2_record_is_selected_by_scene_minus_33_and_kept_byte_exact() {
    // `dungeon-dat.md §1`/§2: dungeon-major 512-byte records, scene 33 loads
    // record zero, and §3/§6 "Unknown low-nibble values must be preserved".
    let dir = g9_scratch_dir("dungeon-dat");
    let mut file = vec![0u8; DUNGEON_DAT_LEN];
    for (index, byte) in file.iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    fs::write(dir.join(DUNGEON_DAT_FILENAME), &file).unwrap();
    for scene_byte in 33u8..=40 {
        let scene = DungeonScene::new(scene_byte).unwrap();
        assert_eq!(scene.record, usize::from(scene_byte - 33));
        let record = load_dungeon_record(&dir, scene).unwrap();
        let start = scene.record * 512;
        assert_eq!(record, file[start..start + 512].to_vec());
        // `cell_index = level_start + y * 8 + x`
        let (level, x, y) = (5usize, 3usize, 6usize);
        assert_eq!(record[level * 64 + y * 8 + x], file[start + level * 64 + y * 8 + x]);
    }
}

#[test]
fn g9_dungeon_dat_s6_wrong_file_size_is_rejected() {
    // `dungeon-dat.md §6`: "The file is exactly 4,096 bytes."
    let dir = g9_scratch_dir("dungeon-dat-short");
    fs::write(dir.join(DUNGEON_DAT_FILENAME), vec![0u8; DUNGEON_DAT_LEN - 1]).unwrap();
    let scene = DungeonScene::new(33).unwrap();
    assert!(load_dungeon_record(&dir, scene).is_err());
    fs::write(dir.join(DUNGEON_DAT_FILENAME), vec![0u8; DUNGEON_DAT_LEN + 1]).unwrap();
    assert!(load_dungeon_record(&dir, scene).is_err());
}

#[test]
fn g9_dungeon_dat_s3_high_nibble_dispatch_table() {
    // `dungeon-dat.md §3` table: the sixteen upper-nibble families.
    use DungeonCellClass as C;
    let expected = [
        C::Passage,
        C::UpLadder,
        C::DownLadder,
        C::TwoWayLadder,
        C::Chest,
        C::Fountain,
        C::PitTrap,
        C::PassageVariant,
        C::EnergyField,
        C::EnergyFieldSecondary,
        C::RoomHelperState,
        C::Wall,
        C::Wall,
        C::Wall,
        C::HeavyDoorVariant,
        C::RoomTrigger,
    ];
    for (nibble, class) in expected.iter().enumerate() {
        for low in [0u8, 7, 15] {
            assert_eq!(dungeon_cell_class_of(((nibble as u8) << 4) | low), *class);
        }
    }
    // §3 retraction: "forward movement rejects only `0xB?`, `0xC?` and `0xD?`".
    for nibble in 0u8..16 {
        assert_eq!(
            dungeon_cell_class_of(nibble << 4).is_wall(),
            (0xB..=0xD).contains(&nibble),
            "nibble {nibble:X}"
        );
    }
}

#[test]
fn g9_dungeon_dat_s4_room_arena_arithmetic_and_doom_slot_fifteen_is_the_last_record() {
    // `dungeon-dat.md §4`: arena_bank = 0 if record <= 1 else record - 1;
    // arena_index = bank * 16 + (cell & 0x0F). Doom room fifteen selects the
    // final record of the arena file.
    for record in 0usize..8 {
        let expected_bank = if record <= 1 { 0 } else { record - 1 };
        assert_eq!(crate::tile_helpers::dungeon_arena_bank(record), expected_bank);
    }
    for scene_byte in 33u8..=40 {
        let scene = DungeonScene::new(scene_byte).unwrap();
        for slot in 0u8..16 {
            let bank = if scene.record <= 1 { 0 } else { scene.record - 1 };
            assert_eq!(
                dungeon_room_arena_index(scene, 0xF0 | slot),
                bank * 16 + usize::from(slot)
            );
        }
    }
    let doom = DungeonScene::new(40).unwrap();
    assert_eq!(
        dungeon_room_arena_index(doom, 0xFF),
        crate::combat_arena::DUNGEON_CBT_RECORDS - 1
    );
}

#[test]
fn g9_dungeon_dat_s4_demotion_rewrites_cleared_triggers_to_0xa_on_every_level() {
    // `dungeon-dat.md §4`: the demotion pass rewrites matching `0xF?` cells
    // "back to `0xA?`" with the low nibble preserved, "over all eight levels
    // of the working record", and never to `0xE?`.
    let scene = DungeonScene::new(33).unwrap();
    let mut bitmap = [0u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN];
    assert!(crate::tile_helpers::set_dungeon_room_clear_bit(&mut bitmap, scene, 5));
    let mut grid = vec![0u8; DUNGEON_RECORD_LEN];
    grid[0] = 0xF5; // level 0
    grid[7 * 64 + 7 * 8 + 7] = 0xF5; // level 7, deepest cell
    grid[3 * 64] = 0xF6; // a different, uncleared room
    grid[4 * 64] = 0xE5; // a door-presentation byte is untouched
    crate::tile_helpers::apply_dungeon_room_clear_bitmap(&mut grid, scene, &bitmap);
    assert_eq!(grid[0], 0xA5);
    assert_eq!(grid[7 * 64 + 7 * 8 + 7], 0xA5);
    assert_eq!(grid[3 * 64], 0xF6);
    assert_eq!(grid[4 * 64], 0xE5);
}

#[test]
fn g9_dungeon_dat_s2_s4_shipped_room_trigger_census() {
    // `dungeon-dat.md §4`: "The shipped Despise record contains no `0xF?`
    // room-trigger cells. The other seven room-bearing dungeons use all
    // sixteen low-nibble slots", and Doom's room fifteen sits on the deepest
    // level at `(X=5, Y=7)`.
    let Some(dir) = g9_asset_dir_with(DUNGEON_DAT_FILENAME) else {
        return;
    };
    for scene_byte in 33u8..=40 {
        let scene = DungeonScene::new(scene_byte).unwrap();
        let record = load_dungeon_record(&dir, scene).unwrap();
        let mut slots = [false; 16];
        for cell in &record {
            if cell >> 4 == 0xF {
                slots[usize::from(cell & 0x0F)] = true;
            }
        }
        if scene_byte == 34 {
            assert!(slots.iter().all(|used| !used), "Despise has no triggers");
        } else {
            assert!(slots.iter().all(|used| *used), "scene {scene_byte}");
        }
    }
    let doom = load_dungeon_record(&dir, DungeonScene::new(40).unwrap()).unwrap();
    assert_eq!(doom[7 * 64 + 7 * 8 + 5], 0xFF);
}

// ---------------------------------------------------------------------------
// formats/tiles.md
// ---------------------------------------------------------------------------

#[test]
fn g9_tiles_s3_s4_row_strides_follow_the_depth_formulas() {
    // `tiles.md §3`: `.16` stride `((w + 7) / 8) * 4`; §4: `.4` stride
    // `(w + 3) / 4`, with no padding.
    for width in 1usize..=40 {
        assert_eq!(
            graphic_image_row_stride(width, TileGraphicsDepth::Ega16).unwrap(),
            ((width + 7) / 8) * 4
        );
        assert_eq!(
            graphic_image_row_stride(width, TileGraphicsDepth::Cga4).unwrap(),
            (width + 3) / 4
        );
    }
}

#[test]
fn g9_tiles_s3_ega_row_padding_bytes_are_skipped() {
    // `tiles.md §3`: padding bytes "contribute no rendered pixels"; a 9-pixel
    // row occupies 8 bytes, of which the first five carry pixels.
    let mut body = Vec::new();
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&6u32.to_le_bytes());
    body.extend_from_slice(&9u16.to_le_bytes());
    body.extend_from_slice(&2u16.to_le_bytes());
    body.extend_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x90, 0xFF, 0xFF, 0xFF]);
    body.extend_from_slice(&[0xAB, 0xCD, 0xEF, 0x01, 0x20, 0xFF, 0xFF, 0xFF]);
    let directory =
        parse_graphic_image_directory_body(&body, TileGraphicsDepth::Ega16, "g9").unwrap();
    let image = directory.images[0].as_ref().unwrap();
    assert_eq!(
        image.pixels,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0, 1, 2]
    );
}

#[test]
fn g9_tiles_s5_3_mask_is_one_flat_bit_run_with_no_row_stride() {
    // `tiles.md §5.3`: "`(width × height + 7) / 8` bytes ... most-significant
    // bit first, no per-row padding"; a set bit means transparent.
    let image_offset = 6u16; // 2 + 1 sprite * 4
    // 3x3 CGA image: one byte per row.
    let image_block = [3u8, 0, 3, 0, 0, 0, 0];
    let mask_offset = image_offset + image_block.len() as u16;
    let mut body = Vec::new();
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&image_offset.to_le_bytes());
    body.extend_from_slice(&mask_offset.to_le_bytes());
    body.extend_from_slice(&image_block);
    body.extend_from_slice(&3u16.to_le_bytes());
    body.extend_from_slice(&3u16.to_le_bytes());
    // 9 bits: 1 0 1 | 1 0 1 | 0 1 0 then pad.
    body.extend_from_slice(&[0b1011_0101, 0b0000_0000]);
    let sheet = parse_graphic_sprite_sheet_body(&body, TileGraphicsDepth::Cga4, "g9").unwrap();
    let sprite = sheet.sprites[0].as_ref().unwrap();
    assert_eq!(sprite.transparent_mask, vec![1, 0, 1, 1, 0, 1, 0, 1, 0]);
}

#[test]
fn g9_tiles_s5_2_zero_offset_slot_is_an_empty_slot() {
    // `tiles.md §5.2`: "Slots may carry an offset of zero, which the engine
    // treats as 'empty slot, skip'".
    let mut body = Vec::new();
    body.extend_from_slice(&3u16.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&14u32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&4u16.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.push(0b0001_1011);
    let directory =
        parse_graphic_image_directory_body(&body, TileGraphicsDepth::Cga4, "g9").unwrap();
    assert_eq!(directory.images.len(), 3);
    assert!(directory.images[0].is_none());
    assert_eq!(directory.images[1].as_ref().unwrap().pixels, vec![0, 1, 2, 3]);
    assert!(directory.images[2].is_none());
}

#[test]
fn g9_tiles_s7_cga_palette_is_black_cyan_magenta_white() {
    // `tiles.md §7`: the CGA driver "selects palette **one** (black, cyan,
    // magenta, white)".
    let [black, cyan, magenta, white] = CGA_PALETTE_RGB;
    assert_eq!(black, [0, 0, 0]);
    assert!(cyan[1] == cyan[2] && cyan[1] > cyan[0]);
    assert!(magenta[0] == magenta[2] && magenta[0] > magenta[1]);
    assert_eq!(white, [0xff, 0xff, 0xff]);
}

#[test]
fn g9_tiles_s6_1_hourglass_frames_block_npc_paths_and_pass_projectiles() {
    // `tiles.md §6.1`: "All four hourglass ids block NPC pathfinding and all
    // four pass projectiles".
    for tile in 0xE8u8..=0xEB {
        assert!(npc_path_tile_obstacle(tile), "{tile:#x}");
        assert!(!outdoor_projectile_tile_blocks(tile), "{tile:#x}");
    }
}

fn g9_directory_shape(dir: &Path, stem: &str, depth: TileGraphicsDepth) -> Option<Vec<Option<(usize, usize)>>> {
    if !dir.join(tile_graphics_file_name(stem, depth)).exists() {
        return None;
    }
    let directory = load_graphic_image_directory(dir, stem, depth).unwrap();
    Some(
        directory
            .images
            .iter()
            .map(|image| image.as_ref().map(|image| (image.width, image.height)))
            .collect(),
    )
}

#[test]
fn g9_tiles_s6_shipped_panel_directories_match_the_roster() {
    // `tiles.md §6` file roster and the per-record inventories.
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
        for (stem, count) in [
            ("CREATE", 11usize),
            ("ULTIMA", 5),
            ("STARTSC", 3),
            ("ENDSC", 1),
            ("END1", 3),
            ("END2", 3),
            ("STORY1", 3),
            ("STORY2", 4),
            ("STORY3", 2),
            ("STORY4", 2),
            ("STORY5", 2),
            ("STORY6", 8),
        ] {
            if let Some(shape) = g9_directory_shape(&dir, stem, depth) {
                assert_eq!(shape.len(), count, "{stem} {depth:?}");
            }
        }
        if let Some(shape) = g9_directory_shape(&dir, "STARTSC", depth) {
            assert_eq!(
                shape,
                vec![Some((16, 137)), Some((288, 137)), Some((16, 137))]
            );
        }
        if let Some(shape) = g9_directory_shape(&dir, "ENDSC", depth) {
            assert_eq!(shape, vec![Some((260, 168))]);
        }
        if let Some(shape) = g9_directory_shape(&dir, "ULTIMA", depth) {
            let last_height = if depth == TileGraphicsDepth::Ega16 { 50 } else { 49 };
            assert_eq!(
                shape,
                vec![
                    Some((319, 61)),
                    Some((288, 49)),
                    Some((288, 49)),
                    Some((288, 49)),
                    Some((288, last_height)),
                ]
            );
        }
    }
}

#[test]
fn g9_tiles_s6_dungeon_billboard_directories_are_identical_in_shape() {
    // `tiles.md §6`: "All three directories are byte-identical - same slot
    // count, same empty slots, same per-image dimensions".
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
        let shapes: Vec<_> = ["DNG1", "DNG2", "DNG3"]
            .iter()
            .filter_map(|stem| g9_directory_shape(&dir, stem, depth))
            .collect();
        for shape in &shapes {
            assert_eq!(shape, &shapes[0]);
        }
    }
}

#[test]
fn g9_tiles_s6_monster_sheets_are_two_poses_of_three_depths() {
    // `tiles.md §6`: "Pose 0 occupies records 0 through 2 and pose 1 records
    // 3 through 5. Within each pose the dimensions are 24 x 66, 16 x 25, and
    // 8 x 6."
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let expected = [(24usize, 66usize), (16, 25), (8, 6), (24, 66), (16, 25), (8, 6)];
    for depth in [TileGraphicsDepth::Ega16, TileGraphicsDepth::Cga4] {
        for index in 0..=7 {
            let stem = format!("MON{index}");
            if !dir.join(tile_graphics_file_name(&stem, depth)).exists() {
                continue;
            }
            let sheet = load_graphic_sprite_sheet(&dir, &stem, depth).unwrap();
            let dims: Vec<_> = sheet
                .sprites
                .iter()
                .map(|sprite| {
                    let sprite = sprite.as_ref().unwrap();
                    (sprite.image.width, sprite.image.height)
                })
                .collect();
            assert_eq!(dims, expected, "{stem} {depth:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// catalogs/npc-roster.md
// ---------------------------------------------------------------------------

fn g9_roster(dir: &Path, scene_byte: u8) -> Vec<NpcSlot> {
    let tlk_name = npc_tlk_filename(scene_byte).unwrap();
    let tlk = parse_tlk(&dir.join(tlk_name)).unwrap();
    parse_npc_block(dir, Scene::new(scene_byte).unwrap(), &tlk).unwrap()
}

#[test]
fn g9_npc_roster_s3_published_rows_decode_exactly() {
    // `npc-roster.md §3`: TOWNE:0 slot 3 (dlg 1, tag 40, A 05-15 m0
    // @(13,17,0); B 15-17/18-05 m0 @(16,13,1); C 17-18 m0 @(20,23,0)) and
    // TOWNE:3 slot 13 (dlg 18, tag 44, A 09-15 m0 @(19,24,-1); B 15-23/01-09
    // m1 @(22,1,0); C 23-01 m0 @(17,28,-1)).
    let Some(dir) = g9_asset_dir_with(TOWNE_NPC_FILENAME) else {
        return;
    };
    let moonglow = g9_roster(&dir, 1);
    let slot = &moonglow[3];
    assert_eq!((slot.dialog_id, slot.type_byte), (1, 0x40));
    assert_eq!(
        slot.schedule,
        [0, 0, 0, 13, 16, 20, 17, 13, 23, 0, 1, 0, 5, 15, 17, 18]
    );
    let yew = g9_roster(&dir, 4);
    let slot = &yew[13];
    assert_eq!((slot.dialog_id, slot.type_byte), (18, 0x44));
    assert_eq!(
        slot.schedule,
        [0, 1, 0, 19, 22, 17, 24, 1, 28, 0xFF, 0, 0xFF, 9, 15, 23, 1]
    );
}

#[test]
fn g9_npc_roster_s4_guard_demand_and_shop_trigger_counts_per_family() {
    // `npc-roster.md §4`: "the 28 high ids in `TOWNE` are 23 shop triggers
    // plus 5 guard-demand markers, `CASTLE` has 17 plus 8, `KEEP` has 6 shop
    // triggers and no guard-demand marker, and `DWELLING` ... has none", and
    // the high ids are exactly `0x81..0x88` and `0xFF`.
    let Some(dir) = g9_asset_dir_with(TOWNE_NPC_FILENAME) else {
        return;
    };
    for (scenes, shops, guards) in [
        (1u8..=8u8, 23usize, 5usize),
        (9..=16, 0, 0),
        (17..=24, 17, 8),
        (25..=32, 6, 0),
    ] {
        let mut shop_count = 0;
        let mut guard_count = 0;
        for scene_byte in scenes.clone() {
            for slot in g9_roster(&dir, scene_byte).iter().skip(1) {
                if !npc_type_byte_occupied(slot.type_byte) {
                    continue;
                }
                match slot.dialog_id {
                    0x81..=0x88 => shop_count += 1,
                    0xFF => guard_count += 1,
                    id => assert!(id < 0x81, "scene {scene_byte} id {id:#x}"),
                }
            }
        }
        assert_eq!((shop_count, guard_count), (shops, guards), "{scenes:?}");
    }
}

#[test]
fn g9_npc_roster_s4_named_special_slots() {
    // `npc-roster.md §4`: CASTLE:1 reuses dialogue id 12 for six guard slots
    // (19..=24); tag `78` "appears on exactly one roster slot, in his
    // castle"; `FC` is "Authored on Stonegate slots 1-3".
    let Some(dir) = g9_asset_dir_with(CASTLE_NPC_FILENAME) else {
        return;
    };
    let blackthorn = g9_roster(&dir, 18);
    let twelve: Vec<usize> = blackthorn
        .iter()
        .filter(|slot| npc_type_byte_occupied(slot.type_byte) && slot.dialog_id == 12)
        .map(|slot| slot.slot)
        .collect();
    assert_eq!(twelve, vec![19, 20, 21, 22, 23, 24]);

    let mut tag78 = Vec::new();
    for scene_byte in 1u8..=32 {
        for slot in g9_roster(&dir, scene_byte).iter().skip(1) {
            if slot.type_byte == 0x78 {
                tag78.push((scene_byte, slot.slot));
            }
        }
    }
    assert_eq!(tag78, vec![(18, 5)]);

    let stonegate = g9_roster(&dir, 29);
    for slot in 1..=3 {
        assert_eq!(stonegate[slot].type_byte, 0xFC, "Stonegate slot {slot}");
    }
}

// ---------------------------------------------------------------------------
// systems/vehicles.md
// ---------------------------------------------------------------------------

fn g9_parked(type_byte: u8, x: usize, y: usize, aux1: u8, aux3: u8) -> ActiveObject {
    ActiveObject {
        type_byte,
        tile: 0,
        x,
        y,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1,
        aux3,
    }
}

#[test]
fn g9_vehicles_s4_horse_boarding_adds_two_to_the_object_byte() {
    // `vehicles.md §4` table: the horse "writes the corresponding
    // mounted-horse marker by adding two to the object byte".
    for parked in [HORSE_PARKED_FIRST, HORSE_PARKED_LAST] {
        let mut state = world_state(open_world_grid(), 3, 3);
        state.active_objects.push(g9_parked(parked, 3, 3, 0, 0));
        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
        assert_eq!(state.player.transport.save_marker(), parked + 2);
        assert_eq!(state.message, "horse");
    }
}

#[test]
fn g9_vehicles_s4_carpet_boarding_writes_marker_0x14() {
    // `vehicles.md §4` table: Magic carpet `0x1B` "writes carpet transport
    // marker `0x14`"; §4.1 prints `carpet`.
    let mut state = world_state(open_world_grid(), 3, 3);
    state.active_objects.push(g9_parked(CARPET_PARKED, 3, 3, 0, 0));
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert_eq!(state.player.transport.save_marker(), 0x14);
    assert_eq!(state.message, "carpet");
}

#[test]
fn g9_vehicles_s4_only_the_furled_frigate_run_is_boardable() {
    // `vehicles.md §4`: "Only the furled-sail object run is boardable", and
    // the `0x2C..0x2F` block is refused "with the ordinary rejection line".
    for byte in [0x20u8, 0x23, 0x2C, 0x2F] {
        let mut state = world_state(open_world_grid(), 3, 3);
        state.active_objects.push(g9_parked(byte, 3, 3, 50, 1));
        assert_eq!(state.board_vehicle(), MoveOutcome::Blocked, "{byte:#x}");
        assert_eq!(state.message, "What?");
        assert!(state.player.transport.is_foot());
        assert_eq!(state.turn, 0);
    }
}

#[test]
fn g9_vehicles_s4_mounted_party_cannot_board_a_ship() {
    // `vehicles.md §4` Ship: the gate accepts foot, carpet and skiff; "if the
    // current state is outside that accepted set it prints the stock 'On
    // foot' refusal and makes no state change".
    let mut state = world_state(open_world_grid(), 3, 3);
    state.player.transport = TransportState::Horse {
        type_byte: HORSE_MOUNTED_FIRST,
        tile: transport_visual_tile_for_marker(HORSE_MOUNTED_FIRST).unwrap(),
    };
    state.sync_player_object();
    state.active_objects.push(g9_parked(SHIP_PARKED_FIRST, 3, 3, 50, 1));
    let before = state.player.transport;
    assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);
    assert_eq!(state.message, BOARD_ON_FOOT_REFUSAL);
    assert_eq!(state.player.transport, before);
    assert!(!state.active_objects[1].is_empty());
}

#[test]
fn g9_vehicles_s4_1_board_lines_capitalise_only_ship() {
    // `vehicles.md §4.1`: `horse`, `carpet`, `skiff`, and `Ship` "capital `S`
    // unlike the other three".
    let horse = TransportState::Horse { type_byte: 0x12, tile: 0 };
    let carpet = TransportState::Carpet { type_byte: 0x14, tile: 0 };
    let skiff = TransportState::Skiff { type_byte: 0x28, tile: 0 };
    let ship = TransportState::Ship {
        type_byte: 0x24,
        tile: 0,
        sails_hoisted: false,
        hull: 99,
        skiffs: 2,
    };
    assert_eq!(horse.board_line(), "horse");
    assert_eq!(carpet.board_line(), "carpet");
    assert_eq!(skiff.board_line(), "skiff");
    assert_eq!(ship.board_line(), "Ship");
}

#[test]
fn g9_vehicles_s6_horse_and_carpet_have_two_frames_changed_only_by_east_west() {
    // `vehicles.md §2`/§6: "only an east or a west step rewrites the frame; a
    // north or south step leaves the frame exactly as it was"; frigates and
    // skiffs compose the direction into the low two bits.
    for first in [HORSE_MOUNTED_FIRST, 0x14u8] {
        let state = if first == HORSE_MOUNTED_FIRST {
            TransportState::Horse { type_byte: first, tile: 0 }
        } else {
            TransportState::Carpet { type_byte: first, tile: 0 }
        };
        assert_eq!(state.save_marker_with_facing(Direction::East), first);
        assert_eq!(state.save_marker_with_facing(Direction::West), first + 1);
        let west = state.with_facing(Direction::West);
        assert_eq!(west.save_marker_with_facing(Direction::North), first + 1);
        assert_eq!(west.save_marker_with_facing(Direction::South), first + 1);
    }
    let skiff = TransportState::Skiff { type_byte: 0x28, tile: 0 };
    for (direction, low) in [
        (Direction::North, 0u8),
        (Direction::East, 1),
        (Direction::South, 2),
        (Direction::West, 3),
    ] {
        assert_eq!(skiff.save_marker_with_facing(direction), 0x28 + low);
    }
}

#[test]
fn g9_vehicles_s6_yell_sail_toggle_moves_the_marker_one_run_keeping_heading() {
    // `vehicles.md §6` Ship Sails: "furling adds a run, hoisting subtracts
    // one" and the heading bits are left alone; §11 commands literals.
    let mut state = world_state(open_world_grid(), 3, 3);
    state.player.transport = TransportState::Ship {
        type_byte: SHIP_PARKED_FIRST + 2,
        tile: FIRST_PLAYABLE_FRIGATE_TILE + 2,
        sails_hoisted: false,
        hull: 60,
        skiffs: 1,
    };
    state.sync_player_object();
    assert_eq!(state.player.transport.save_marker(), 0x26);
    assert_eq!(state.yell_command(None), MoveOutcome::SailToggled);
    assert_eq!(state.message, YELL_SAILS_HOISTED_MESSAGE);
    assert_eq!(state.player.transport.save_marker(), 0x22);
    assert_eq!(state.yell_command(None), MoveOutcome::SailToggled);
    assert_eq!(state.message, YELL_SAILS_FURLED_MESSAGE);
    assert_eq!(state.player.transport.save_marker(), 0x26);
    // Toggling consumed a turn each time (`commands.md §11`: acted).
    assert_eq!(state.turn, 2);
}

#[test]
fn g9_vehicles_s5_skiff_xit_refuses_both_bridge_ids() {
    // `vehicles.md §5`: "The check ignores the low bit of the tile id, so the
    // two ids are always refused together".
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = 0x6B;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    state.sync_player_object();
    assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);
    assert_eq!(state.message, "\nNot here!\n");
    assert!(matches!(state.player.transport, TransportState::Skiff { .. }));
}

#[test]
fn g9_vehicles_s5_horse_xit_parks_a_riderless_horse_in_place() {
    // `vehicles.md §5`: "X-it succeeds unconditionally for the mounted-horse
    // family. It parks the riderless horse object at the current coordinate";
    // §5.1 prints `horse!`.
    let mut state = world_state(open_world_grid(), 6, 6);
    state.player.transport = TransportState::Horse {
        type_byte: HORSE_MOUNTED_LAST,
        tile: transport_visual_tile_for_marker(HORSE_MOUNTED_LAST).unwrap(),
    };
    state.sync_player_object();
    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);
    assert_eq!(state.message, "horse!");
    assert!(state.player.transport.is_foot());
    assert_eq!((state.player.x, state.player.y), (6, 6));
    let parked = state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| !object.is_empty())
        .copied()
        .unwrap();
    assert!((HORSE_PARKED_FIRST..=HORSE_PARKED_LAST).contains(&parked.type_byte));
    assert_eq!((parked.x, parked.y), (6, 6));
}

#[test]
fn g9_vehicles_s6_vehicle_prefixes() {
    // `vehicles.md §6` table: horse "Ride", carpet "Fly", skiff "Row"; a
    // frigate facing change prints `Head ` plus the direction word.
    use crate::commands::{vehicle_movement_echo, MovementEchoPrefix as P};
    assert_eq!(vehicle_movement_echo(Direction::North, P::Ride).unwrap().text, "Ride North");
    assert_eq!(vehicle_movement_echo(Direction::East, P::Fly).unwrap().text, "Fly East");
    assert_eq!(vehicle_movement_echo(Direction::South, P::Row).unwrap().text, "Row South");
    assert_eq!(vehicle_movement_echo(Direction::West, P::Head).unwrap().text, "Head West");
    assert_eq!(vehicle_movement_echo(Direction::West, P::None).unwrap().text, "West");
    assert!(vehicle_movement_echo(Direction::NorthEast, P::Ride).is_none());
}

#[test]
fn g9_vehicles_s8_cannon_tiles_fire_along_their_own_facing() {
    // `vehicles.md §8`: "tile `180` fires north, `181` east, `182` south and
    // `183` west".
    use crate::commands::town_cannon_tile_fire_direction as fire;
    assert_eq!(fire(180), Some(Direction::North));
    assert_eq!(fire(181), Some(Direction::East));
    assert_eq!(fire(182), Some(Direction::South));
    assert_eq!(fire(183), Some(Direction::West));
    assert_eq!(fire(179), None);
    assert_eq!(fire(184), None);
}

#[test]
fn g9_vehicles_s6_underfoot_gas_requires_exact_on_foot_marker() {
    // `vehicles.md §6` Underfoot table / `town-mode.md §10`: the gas save
    // requires "the transport marker to be exactly the on-foot value, so
    // every mount and vessel, and the on-foot alias value, is exempt".
    assert!(town_poison_gas_live_tile_matches(0x04, 0x1C));
    for marker in [0x1Du8, 0x12, 0x14, 0x24, 0x28] {
        assert!(!town_poison_gas_live_tile_matches(0x04, marker), "{marker:#x}");
    }
    assert!(!town_poison_gas_live_tile_matches(0x05, 0x1C));
}

// ---------------------------------------------------------------------------
// systems/town-mode.md
// ---------------------------------------------------------------------------

#[test]
fn g9_town_mode_s16_free_roaming_walker_eligibility_and_pen_gate() {
    // `town-mode.md §16` walker table: eligible bytes are `(byte0 & 0xFE) ==
    // 0x10`; the pen gate blocks on exactly `0xA2` or `0x43`.
    let object = |type_byte| ActiveObject {
        type_byte,
        tile: type_byte,
        x: 1,
        y: 1,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    for byte in 0u8..=255 {
        assert_eq!(
            town_free_roaming_object_eligible(object(byte)),
            byte == 0x10 || byte == 0x11,
            "{byte:#x}"
        );
        assert_eq!(
            town_free_roaming_pen_tile_blocks(byte),
            byte == 0xA2 || byte == 0x43,
            "{byte:#x}"
        );
    }
}

#[test]
fn g9_town_mode_s10_acceptance_debit_applies_below_0x80_only() {
    // `town-mode.md §10` "Acceptance debit": "an actor type below `0x80`
    // (townsfolk, named characters and guards alike) costs 5 points ...
    // `0xFC` and every other type at or above `0x80` do neither."
    assert_eq!(TOWN_ATTACK_KARMA_DEBIT, 5);
    for byte in 0x40u8..=0xFF {
        assert_eq!(town_attack_acceptance_debits(byte), byte < 0x80, "{byte:#x}");
    }
    assert!(town_attack_raises_alarm(0xD8));
    assert!(!town_attack_raises_alarm(0xFC));
}

#[test]
fn g9_town_mode_s14_forced_pursuit_rewrites_modes_and_zeroes_boundaries_only() {
    // `town-mode.md §14`: forced pursuit writes mode `6` below type `0x2F`,
    // otherwise `7`, zeroes all four time boundaries, "preserve all waypoint
    // coordinates and the dialogue index".
    for (type_byte, mode) in [(0x2Eu8, 6u8), (0x2F, 7), (0x70, 7), (0xFC, 7)] {
        let slot = NpcSlot {
            slot: 4,
            type_byte,
            dialog_id: 9,
            schedule: [1, 2, 0, 3, 4, 5, 6, 7, 8, 0, 1, 0, 7, 12, 18, 21],
            name: None,
        };
        let mut npc = RuntimeNpc::from_slot(&slot, 10);
        npc.force_town_pursuit();
        assert_eq!(&npc.schedule[0..3], &[mode; 3]);
        assert_eq!(&npc.schedule[3..12], &slot.schedule[3..12]);
        assert_eq!(&npc.schedule[12..16], &[0; 4]);
        assert_eq!(npc.dialog_id, 9);
    }
}

#[test]
fn g9_town_mode_s14_forced_flight_preconditions_and_result() {
    // `town-mode.md §14`: flight needs type `0x40..0x73` and either dialogue
    // `0xFE` or a nonzero time boundary; it writes mode `3`, dialogue `0xFD`,
    // and preserves boundaries and waypoints.
    let make = |type_byte, dialog_id, times: [u8; 4]| {
        let mut schedule = [0u8; 16];
        schedule[3..12].copy_from_slice(&[1, 2, 3, 4, 5, 6, 0, 0, 0]);
        schedule[12..16].copy_from_slice(&times);
        RuntimeNpc::from_slot(
            &NpcSlot {
                slot: 2,
                type_byte,
                dialog_id,
                schedule,
                name: None,
            },
            0,
        )
    };
    let mut accepted = make(0x50, 7, [0, 9, 0, 0]);
    assert!(accepted.force_town_flight());
    assert_eq!(&accepted.schedule[0..3], &[3; 3]);
    assert_eq!(&accepted.schedule[12..16], &[0, 9, 0, 0]);
    assert_eq!(&accepted.schedule[3..9], &[1, 2, 3, 4, 5, 6]);
    assert_eq!(accepted.dialog_id, 0xFD);

    let mut sentinel = make(0x73, 0xFE, [0; 4]);
    assert!(sentinel.force_town_flight());

    for (type_byte, dialog_id, times) in [
        (0x3Fu8, 7u8, [1u8, 0, 0, 0]),
        (0x74, 7, [1, 0, 0, 0]),
        (0x50, 7, [0, 0, 0, 0]),
    ] {
        let mut rejected = make(type_byte, dialog_id, times);
        let before = rejected.clone();
        assert!(!rejected.force_town_flight());
        assert_eq!(rejected, before);
    }
}

#[test]
fn g9_town_mode_s14_dialogue_sentinels_route_to_canned_lines() {
    // `town-mode.md §14`: Talk recognises `0xFD` and `0xFE` independently of
    // the `.TLK` record; `0xFE` is the shouted brush-off.
    assert!(town_npc_dispatcher_sentinel_line(0xFD).unwrap().starts_with('\n'));
    let brush = town_npc_dispatcher_sentinel_line(0xFE).unwrap();
    assert_eq!(brush, format!("\n{TOWN_NPC_BRUSHOFF_RESPONSE}"));
    assert!(brush.contains("Begone,") && brush.contains("vermin!"));
    for id in [0u8, 1, 12, 0x81, 0xFC, 0xFF] {
        assert!(town_npc_dispatcher_sentinel_line(id).is_none());
    }
}

#[test]
fn g9_town_mode_s13_resident_shadowlord_schedule_is_pinned_to_its_cell() {
    // `town-mode.md §13`: the installed record carries `0xFC`, floor `0`, and
    // "a stationary three-waypoint schedule pinned to that same cell".
    let npc = RuntimeNpc::from_resident_shadowlord(31, 15, 9, 13);
    assert_eq!(npc.type_byte, 0xFC);
    for wp in 0..3 {
        assert_eq!(npc.waypoint_position(wp), (15, 9, 0));
    }
    assert_eq!((npc.x, npc.y, npc.z), (15, 9, 0));
}

fn g9_town_party(statuses: &[u8]) -> Vec<PartyMember> {
    statuses
        .iter()
        .enumerate()
        .map(|(index, &status)| PartyMember {
            slot: index as u8,
            class_byte: b'F',
            status,
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        })
        .collect()
}

#[test]
fn g9_town_mode_s7_2_poisoned_member_is_selectable_and_free() {
    // `town-mode.md §7.2`: "Any other status letter, Poisoned included, is
    // selectable", and a successful selection is free.
    let mut state = test_state(open_grid(), 5, 5);
    state.party = g9_town_party(&[b'G', b'P']);
    let turn = state.turn;
    state.set_active_player_command('2');
    assert_eq!(state.active_player, Some(1));
    assert_eq!(state.turn, turn);
}

#[test]
fn g9_town_mode_s7_2_invalid_digit_costs_the_town_turn() {
    // `town-mode.md §7.2`: `Invalid!` (beyond the party size, a Dead or an
    // Asleep member) "costs the one-minute town turn" and leaves the
    // selection unchanged.
    for (statuses, digit) in [
        (vec![b'G', b'G'], '3'),
        (vec![b'G', b'D'], '2'),
        (vec![b'G', b'S'], '2'),
        (vec![b'G'], '9'),
    ] {
        let mut state = test_state(open_grid(), 5, 5);
        state.party = g9_town_party(&statuses);
        state.active_player = Some(0);
        let turn = state.turn;
        state.set_active_player_command(digit);
        assert_eq!(state.active_player, Some(0), "{statuses:?} {digit}");
        assert_eq!(state.turn, turn + 1, "{statuses:?} {digit}");
        assert!(state.message.contains("Invalid!"));
    }
}

#[test]
fn g9_town_mode_s7_2_zero_clears_the_selection_for_free() {
    // `town-mode.md §7.2`: `0` prints `None!`, clears the active player, and
    // is free.
    let mut state = test_state(open_grid(), 5, 5);
    state.party = g9_town_party(&[b'G', b'G']);
    state.active_player = Some(1);
    let turn = state.turn;
    state.set_active_player_command('0');
    assert_eq!(state.active_player, None);
    assert_eq!(state.turn, turn);
    assert!(state.message.contains("None!"));
}

// ---------------------------------------------------------------------------
// systems/commands.md
// ---------------------------------------------------------------------------

#[test]
fn g9_commands_s5_2_d_and_w_refusals_carry_their_letter() {
    // `commands.md §5.2`: `D` → `D-What?`, `W` → `W-What?`, any unmapped key
    // → `What?`.
    use crate::commands::unassigned_refusal_echo as echo;
    assert_eq!(echo(b'D'), "D-What?");
    assert_eq!(echo(b'W'), "W-What?");
    assert_eq!(echo(b'Q'), "What?");
}

#[test]
fn g9_commands_s5_3_trailing_punctuation_contract() {
    // `commands.md §5.2`/§5.3: Board, X-it, Yell and Hole up end in a space;
    // dungeon Attack has no hyphen; `Open-` prints in every scene; `New
    // Order` has no trailing newline; dungeon Search is `Search...`.
    use crate::commands::{command_echo, CommandEchoMode as M};
    let echo = |command, mode| command_echo(command, mode).unwrap().text;
    assert_eq!(echo(Command::Board, M::Surface), "Board ");
    assert_eq!(echo(Command::Xit, M::Surface), "X-it ");
    assert_eq!(echo(Command::Yell, M::Surface), "Yell ");
    assert_eq!(echo(Command::HoleUp, M::Surface), "Hole up- ");
    assert_eq!(echo(Command::Attack, M::Dungeon), "Attack");
    assert_eq!(echo(Command::Attack, M::Surface), "Attack-");
    assert_eq!(echo(Command::Open, M::Dungeon), "Open-");
    assert_eq!(echo(Command::Open, M::Surface), "Open-");
    assert_eq!(echo(Command::Get, M::Dungeon), "Get");
    assert_eq!(echo(Command::Search, M::Dungeon), "Search...");
    assert_eq!(echo(Command::NewOrder, M::Surface), "New Order");
    assert_eq!(echo(Command::Quit, M::Surface), "Quit:");
}

#[test]
fn g9_commands_s8_push_rotates_only_chair_and_cannon_families() {
    // `commands.md §8`: chairs `0x90..0x93` and cannons `0xB4..0xB7` rewrite
    // the facing bits on a push; the other pushable families keep their byte.
    use crate::commands::pushable_oriented_tile as orient;
    assert_eq!(orient(0x90, Direction::East), 0x91);
    assert_eq!(orient(0x93, Direction::North), 0x90);
    assert_eq!(orient(0xB6, Direction::West), 0xB7);
    for tile in [0x5Bu8, 0xA5, 0xA6, 0xA8, 0xA9, 0xAD, 0xAE, 0xAF] {
        assert_eq!(orient(tile, Direction::South), tile);
    }
    // Non-pushable bytes are never rewritten.
    assert_eq!(orient(0x44, Direction::East), 0x44);
}

#[test]
fn g9_commands_s8_2_cannon_stamp_differs_from_the_generic_stamp() {
    // `commands.md §8.2`: "The generic stamp `0x44` and the cannon-family stamp
    // `0x45`" - the byte is load-bearing for family matching.
    use crate::commands::{pushable_tile_family, PUSHABLE_CANNON_FLOOR_STAMP, PUSHABLE_GENERIC_FLOOR_STAMP};
    assert_eq!(PUSHABLE_GENERIC_FLOOR_STAMP, 0x44);
    assert_eq!(PUSHABLE_CANNON_FLOOR_STAMP, 0x45);
    assert_eq!(pushable_tile_family(0xB5).unwrap().floor_stamp(), 0x45);
    for tile in [0x5Bu8, 0x92, 0xA5, 0xAF] {
        assert_eq!(pushable_tile_family(tile).unwrap().floor_stamp(), 0x44);
    }
    for tile in [0x5Au8, 0x94, 0xA7, 0xAA, 0xAC, 0xB0, 0xB3, 0xB8] {
        assert!(pushable_tile_family(tile).is_none(), "{tile:#x}");
    }
}

#[test]
fn g9_commands_s11_yell_sail_gate_is_frigate_and_unsigned_scene_below_0x80() {
    // `commands.md §11`: toggle for scenes `0..=127` with a frigate marker;
    // scenes `128..=255` and every non-frigate marker take the word prompt.
    for scene in 0u8..=255 {
        assert_eq!(yell_routes_to_ship_sails(scene, true), scene < 0x80, "{scene}");
        assert!(!yell_routes_to_ship_sails(scene, false));
    }
}

#[test]
fn g9_commands_s11_shadowlord_names_only_in_the_three_flame_keeps() {
    // `commands.md §11`: "Only the three Eternal Flame keeps - The Lycaeum,
    // Empath Abbey, and Serpent's Hold - accept Shadowlord names"; "Only the
    // outdoor scene accepts Words of Power".
    use crate::commands::{yell_input_context, YellInputContext as Y};
    for scene in 0u8..=255 {
        let expected = match scene {
            0 => Y::WordOfPower,
            30 | 31 | 32 => Y::ShadowlordName,
            _ => Y::NoEffect,
        };
        assert_eq!(yell_input_context(scene), expected, "{scene}");
    }
}

#[test]
fn g9_commands_s11_1_word_entrance_tiles() {
    // `commands.md §11.1` per-word entrance tile table; sealed form `0xDF`.
    let expected = [
        ("FALLAX", 0x18u8),
        ("VILIS", 0x16),
        ("INOPIA", 0x16),
        ("MALUM", 0x18),
        ("AVIDUS", 0x18),
        ("INFAMA", 0x17),
        ("IGNAVUS", 0x17),
        ("VERAMOCOR", 0x16),
    ];
    for (seal, (word, tile)) in WORD_OF_POWER_SEALS.iter().zip(expected) {
        assert_eq!(seal.word, word);
        assert_eq!(seal.unsealed_tile, tile, "{word}");
    }
    assert_eq!(WORD_OF_POWER_SEALED_TILE, 0xDF);
}

// ---------------------------------------------------------------------------
// systems/timing.md
// ---------------------------------------------------------------------------

#[test]
fn g9_timing_s7_1_outer_unit_sits_in_the_published_band() {
    // `timing.md §7.1`: one outer calibrated unit is 0.88 ms, band 0.79 to
    // 0.97 ms; outer counts are exact program constants.
    let unit = crate::audio::calibrated_nanos(1);
    assert!((790_000..=970_000).contains(&unit), "{unit}");
    assert_eq!(crate::audio::calibrated_nanos(200), 200 * unit);
}

#[test]
fn g9_timing_s8_2_idle_world_step_band_is_numeric_and_inclusive() {
    // `timing.md §8.2`: no idle world step for scene values `0x21` through
    // `0x7F` inclusive; combat `0xFF` and town/overworld do step.
    for scene in 0u8..=255 {
        assert_eq!(
            idle_world_step_suppressed_for_scene(scene),
            (0x21..=0x7F).contains(&scene),
            "{scene:#x}"
        );
    }
}

#[test]
fn g9_timing_s6_3_stonegate_descent_is_750_tones_at_forty_units() {
    // `timing.md §6.3`: "Stonegate trapdoor descending sweep | Direct wait |
    // 40 | Once per tone, 750 tones"; `town-mode.md §10` step 2 runs from
    // 1000 down to 251.
    assert_eq!(crate::audio::STONEGATE_DESCENT_TONES, 750);
    assert_eq!(crate::audio::STONEGATE_DESCENT_HOLD_UNITS, 40);
    assert_eq!(crate::audio::STONEGATE_DESCENT_TOP_HZ, 1000);
    assert_eq!(crate::audio::STONEGATE_DESCENT_BOTTOM_HZ, 251);
}
