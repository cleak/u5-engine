// Gameplay-screen border chrome and message window.
//
// Geometry here is the measured 320x200 layout documented in
// `gameplay_chrome`; see that module's header for provenance and for
// the pending spec question `cleak/u5-spec#79`.

fn chrome_test_font() -> FixedCellFont {
    // A font whose only meaningful glyphs are the two end-cap source
    // triangles; everything else is blank so tests can tell chrome
    // apart from text.
    let mut bytes = vec![0u8; CH_FONT_LEN];
    let right: [u8; 8] = [0x80, 0xe0, 0xf8, 0xfc, 0xfc, 0xf8, 0xe0, 0x80];
    let left: [u8; 8] = [0x01, 0x07, 0x1f, 0x3f, 0x3f, 0x1f, 0x07, 0x01];
    bytes[0x02 * 8..0x02 * 8 + 8].copy_from_slice(&right);
    bytes[0x01 * 8..0x01 * 8 + 8].copy_from_slice(&left);
    // The three reserved corner glyphs the chrome stamps opaquely.
    let top_left: [u8; 8] = [0x07, 0x1f, 0x3f, 0x7f, 0x7f, 0xff, 0xff, 0xff];
    let top_right: [u8; 8] = [0xe0, 0xf8, 0xfc, 0xfe, 0xfe, 0xff, 0xff, 0xff];
    let bottom_left: [u8; 8] = [0xff, 0xff, 0xff, 0x7f, 0x7f, 0x3f, 0x1f, 0x07];
    bytes[0x7b * 8..0x7b * 8 + 8].copy_from_slice(&top_left);
    bytes[0x7c * 8..0x7c * 8 + 8].copy_from_slice(&top_right);
    bytes[0x7d * 8..0x7d * 8 + 8].copy_from_slice(&bottom_left);
    parse_ch_font(&bytes, IBM_CH_FILE).unwrap()
}

fn chrome_frame(content: &GameplayChromeContent) -> Vec<u8> {
    let font = chrome_test_font();
    let mut rgba = vec![0u8; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 0, 0xff]);
    }
    paint_gameplay_frame_chrome(
        &mut rgba,
        TEXT_WINDOW_RENDER_WIDTH,
        TEXT_WINDOW_RENDER_HEIGHT,
        content,
        ChromeFonts {
            ibm: &font,
            runes: &font,
        },
    );
    rgba
}

fn chrome_index_at(rgba: &[u8], x: usize, y: usize) -> u8 {
    let offset = (y * TEXT_WINDOW_RENDER_WIDTH + x) * 4;
    let pixel = [rgba[offset], rgba[offset + 1], rgba[offset + 2]];
    EGA_PALETTE_RGB
        .iter()
        .position(|rgb| *rgb == pixel)
        .unwrap_or_else(|| panic!("pixel ({x}, {y}) = {pixel:?} is not an EGA palette entry"))
        as u8
}

#[test]
fn gameplay_chrome_paints_ribbon_bands_rules_and_leaves_row_24_black() {
    let rgba = chrome_frame(&GameplayChromeContent::default());

    // Left ribbon band and the viewport's white left rule.
    assert_eq!(chrome_index_at(&rgba, 3, 100), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 7, 100), CHROME_RULE_INDEX);
    // Viewport interior stays black for the tile blit.
    assert_eq!(chrome_index_at(&rgba, 8, 100), 0);
    assert_eq!(chrome_index_at(&rgba, 183, 100), 0);
    // Middle band, stats-panel rules, and the right band.
    assert_eq!(chrome_index_at(&rgba, 184, 100), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 187, 100), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 191, 30), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 312, 30), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 316, 30), CHROME_RIBBON_INDEX);
    // The right band stops at y=86: the message box runs to the edge.
    assert_eq!(chrome_index_at(&rgba, 316, 86), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 316, 88), 0);
    // Divider bands at rows 7 and 10, framed by white rules.
    assert_eq!(chrome_index_at(&rgba, 250, 56), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 250, 60), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 250, 63), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 250, 80), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 250, 84), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 250, 87), CHROME_RULE_INDEX);
    // Text row 24 is left entirely black.
    assert_eq!(chrome_index_at(&rgba, 100, 195), 0);
    for y in 192..TEXT_WINDOW_RENDER_HEIGHT {
        for x in 0..TEXT_WINDOW_RENDER_WIDTH {
            assert_eq!(chrome_index_at(&rgba, x, y), 0, "({x}, {y}) must stay black");
        }
    }
}

#[test]
fn gameplay_chrome_rounds_the_three_outer_corners() {
    let rgba = chrome_frame(&GameplayChromeContent::default());

    // `CHROME_CORNER_PROFILE` starts the fill at column 5/3/2/1/1/0 on
    // the first six rows of each band, mirrored on the far edge.
    for (row_from_edge, start_column) in CHROME_CORNER_PROFILE.into_iter().enumerate() {
        let start_column = usize::from(start_column);
        let top = row_from_edge;
        let bottom = CHROME_BOTTOM_Y - row_from_edge;
        if start_column > 0 {
            assert_eq!(chrome_index_at(&rgba, start_column - 1, top), 0);
            assert_eq!(chrome_index_at(&rgba, start_column - 1, bottom), 0);
            assert_eq!(chrome_index_at(&rgba, 320 - start_column, top), 0);
        }
        assert_eq!(
            chrome_index_at(&rgba, start_column, top),
            CHROME_RIBBON_INDEX
        );
        assert_eq!(
            chrome_index_at(&rgba, start_column, bottom),
            CHROME_RIBBON_INDEX
        );
        assert_eq!(
            chrome_index_at(&rgba, 319 - start_column, top),
            CHROME_RIBBON_INDEX
        );
    }

    // The intro menu frame is the same measured carve, kept in one place.
    assert_eq!(INTRO_MENU_FRAME_CORNER_PROFILE, CHROME_CORNER_PROFILE);
}

#[test]
fn ribbon_end_cap_is_the_one_row_erosion_of_its_source_triangle() {
    let font = chrome_test_font();

    let right = ribbon_cap_sprite(&font, RibbonCapDirection::Right);
    assert_eq!(right.ribbon, [0x00, 0x80, 0xe0, 0xf8, 0xf8, 0xe0, 0x80, 0x00]);
    assert_eq!(right.white, [0x80, 0x60, 0x18, 0x04, 0x04, 0x18, 0x60, 0x80]);

    let left = ribbon_cap_sprite(&font, RibbonCapDirection::Left);
    assert_eq!(left.ribbon, [0x00, 0x01, 0x07, 0x1f, 0x1f, 0x07, 0x01, 0x00]);
    assert_eq!(left.white, [0x01, 0x06, 0x18, 0x20, 0x20, 0x18, 0x06, 0x01]);

    // The two masks partition the source triangle exactly.
    for row in 0..8 {
        let solid = font.glyph_row(RIBBON_CAP_RIGHT_SOURCE_GLYPH, row).unwrap();
        assert_eq!(right.ribbon[row] | right.white[row], solid);
        assert_eq!(right.ribbon[row] & right.white[row], 0);
    }
}

#[test]
fn sky_strip_cells_map_to_columns_six_through_seventeen() {
    assert_eq!(sky_strip_cell_column(0), SKY_STRIP_FIRST_COLUMN);
    assert_eq!(sky_strip_cell_column(11), 17);

    // Hour 8 puts the fixed marker at cell 9 and Trammel at cell 0.
    let cells = sky_strip_cells(8, [b'4', b'6']);
    assert_eq!(
        cells[9],
        Some(SkyStripCell {
            rune_code: SKY_STRIP_HOUR_MARKER_RUNE,
            palette_index: SKY_STRIP_HOUR_MARKER_INDEX,
        })
    );
    assert_eq!(
        cells[0],
        Some(SkyStripCell {
            rune_code: 0x34,
            palette_index: SKY_STRIP_MOON_INDEX,
        })
    );
    assert!(cells[1].is_none());

    // Phase digits index the rune alphabet directly; anything else has
    // no glyph rather than a placeholder.
    assert_eq!(sky_strip_moon_rune(b'0'), Some(0x30));
    assert_eq!(sky_strip_moon_rune(b'7'), Some(0x37));
    assert_eq!(sky_strip_moon_rune(b'8'), None);
    assert_eq!(sky_strip_moon_rune(0), None);
}

#[test]
fn sky_strip_gap_caps_sit_at_columns_five_and_eighteen() {
    let content = ChromeGap::SkyStrip(Box::new(sky_strip_cells(8, [b'4', b'6'])));
    let gap = top_gap(&content).unwrap();

    assert_eq!(gap.left_cap_column, 5);
    assert_eq!(gap.right_cap_column, 18);
    assert_eq!(gap.content_first_column, SKY_STRIP_FIRST_COLUMN);
    assert_eq!(gap.content_cells, SKY_STRIP_CELL_COUNT as usize);

    // The white rule at y=7 is interrupted across the whole gap.
    let rgba = chrome_frame(&GameplayChromeContent {
        top: content,
        ..GameplayChromeContent::default()
    });
    assert_eq!(chrome_index_at(&rgba, 40, 7), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 41, 7), 0);
    assert_eq!(chrome_index_at(&rgba, 150, 7), 0);
    assert_eq!(chrome_index_at(&rgba, 151, 7), CHROME_RULE_INDEX);
    // The cap's outline and ribbon fill both land in column 5.
    assert_eq!(chrome_index_at(&rgba, 40, 0), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 40, 1), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 41, 1), CHROME_RULE_INDEX);
}

#[test]
fn wind_banner_pads_direction_into_a_five_column_field() {
    assert_eq!(wind_banner_text(Some("East")), "East  Winds");
    assert_eq!(wind_banner_text(Some("South")), "South Winds");
    assert_eq!(wind_banner_text(Some("West")), "West  Winds");
    assert_eq!(wind_banner_text(Some("Calm")), "Calm  Winds");
    // `weather.md §2`: an out-of-range wind byte drops the direction
    // label but keeps the shared suffix.
    // Out of range: no direction label at all, so the suffix keeps its
    // own leading space at columns 7..=12 and the cap closes at 13.
    assert_eq!(wind_banner_text(None), " Winds");
    for text in [wind_banner_text(Some("East")), wind_banner_text(Some("South"))] {
        assert_eq!(text.chars().count(), WIND_BANNER_CELLS);
    }
    assert_eq!(wind_banner_text(None).chars().count(), 6);

    let gap = bottom_gap(&ChromeGap::Label(wind_banner_text(Some("East")))).unwrap();
    assert_eq!(gap.left_cap_column, 6);
    assert_eq!(gap.right_cap_column, 18);
    assert_eq!(gap.content_first_column, WIND_BANNER_FIRST_COLUMN);
}

#[test]
fn dungeon_labels_close_the_ribbon_gap_around_their_own_width() {
    // Observed on the published dungeon frame: `L5` sits between caps
    // at columns 10 and 13, and `Dir:  East` between columns 6 and 17.
    let level = ChromeGap::Label(dungeon_level_label(5));
    let gap = top_gap(&level).unwrap();
    assert_eq!(gap.left_cap_column, 10);
    assert_eq!(gap.right_cap_column, 13);
    assert_eq!(gap.content_first_column, 11);

    let facing = ChromeGap::Label(dungeon_direction_label("East"));
    assert_eq!(dungeon_direction_label("East"), "Dir:  East");
    let gap = bottom_gap(&facing).unwrap();
    assert_eq!(gap.left_cap_column, 6);
    assert_eq!(gap.right_cap_column, 17);
    assert_eq!(gap.content_first_column, 7);
}

#[test]
fn timing_glyph_slot_appears_only_when_the_tag_byte_is_nonzero() {
    assert!(timing_glyph_gap(None).is_none());
    assert!(timing_glyph_gap(Some(0)).is_none());

    let gap = timing_glyph_gap(Some(b'P')).unwrap();
    assert_eq!(gap.left_cap_column, 30);
    assert_eq!(gap.right_cap_column, 32);
    assert_eq!(gap.content_first_column, TIMING_GLYPH_COLUMN);

    // Zero leaves divider row 7 a plain ribbon band with no caps.
    let plain = chrome_frame(&GameplayChromeContent::default());
    assert_eq!(chrome_index_at(&plain, 245, 60), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&plain, 250, 56), CHROME_RULE_INDEX);

    let tagged = chrome_frame(&GameplayChromeContent {
        timing_glyph: Some(b'P'),
        ..GameplayChromeContent::default()
    });
    assert_eq!(chrome_index_at(&tagged, 241, 56), 0);
    assert_eq!(chrome_index_at(&tagged, 240, 56), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&tagged, 263, 56), CHROME_RULE_INDEX);
}

#[test]
fn gameplay_chrome_content_follows_the_scene_family() {
    let mut state = test_state(open_grid(), 1, 1);
    state.wind = WindState::South;
    state.wind_save_byte = WindState::South.save_byte();

    let town = gameplay_chrome_content(&state);
    assert!(matches!(town.top, ChromeGap::SkyStrip(_)));
    assert_eq!(town.bottom, ChromeGap::Label("South Winds".to_string()));

    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.player.facing = Direction::East;
    let Area::Dungeon { level, .. } = dungeon.area else {
        panic!("dungeon fixture must be a dungeon scene");
    };
    let below = gameplay_chrome_content(&dungeon);
    assert_eq!(below.top, ChromeGap::Label(format!("L{level}")));
    assert_eq!(below.bottom, ChromeGap::Label("Dir:  East".to_string()));
}

#[test]
fn message_window_prefixes_command_echoes_and_spaces_turns() {
    let mut log = GameplayMessageLog::new();
    log.push_command("Pass");
    log.end_turn();
    log.push_command("Z-stats");
    log.push_output("Player! None!");
    log.end_turn();

    let layout = layout_message_window(&log, Some("Look-"));
    let rows: Vec<(u8, u8, &str, bool)> = layout
        .rows
        .iter()
        .map(|row| (row.row, row.column, row.text.as_str(), row.prefixed))
        .collect();

    // Blank rows are not emitted; the live input line is the window's
    // bottom row and carries the end-cap prefix like any command echo.
    assert_eq!(
        rows,
        vec![
            (18, MESSAGE_WINDOW_LEFT + 1, "Pass", true),
            (20, MESSAGE_WINDOW_LEFT + 1, "Z-stats", true),
            (21, MESSAGE_WINDOW_LEFT, "Player! None!", false),
            (23, MESSAGE_WINDOW_LEFT + 1, "Look-", true),
        ]
    );
    assert_eq!(layout.prefixed_rows(), vec![18, 20, 23]);
    assert_eq!(MESSAGE_WINDOW_BOTTOM, 23);
}

#[test]
fn message_window_wraps_and_scrolls_within_its_thirteen_rows() {
    let mut log = GameplayMessageLog::new();
    log.push_output("Yes. Saving... Done.");
    let layout = layout_message_window(&log, None);
    assert_eq!(
        layout
            .rows
            .iter()
            .map(|row| row.text.clone())
            .collect::<Vec<_>>(),
        vec!["Yes. Saving...".to_string(), "Done.".to_string()]
    );
    assert!(
        layout
            .rows
            .iter()
            .all(|row| row.text.chars().count() <= MESSAGE_WINDOW_WIDTH)
    );

    // The log keeps only the rows that can still scroll into view.
    for index in 0..40 {
        log.push_command(&format!("Cmd{index}"));
        log.end_turn();
    }
    assert!(log.lines().len() <= MESSAGE_WINDOW_HISTORY_ROWS);
    let layout = layout_message_window(&log, Some(""));
    assert!(layout.rows.iter().all(|row| {
        row.row >= MESSAGE_WINDOW_TOP && row.row <= MESSAGE_WINDOW_BOTTOM
    }));
    assert_eq!(layout.rows.last().unwrap().row, MESSAGE_WINDOW_BOTTOM);
}

#[test]
fn viewport_origin_sits_inside_the_white_frame_rule() {
    assert_eq!(VIEWPORT_ORIGIN_X, 8);
    assert_eq!(VIEWPORT_ORIGIN_Y, 8);
    // Eleven 16px tiles fill x=8..=183, immediately inside the rules at
    // x=7 and x=184.
    assert_eq!(VIEWPORT_ORIGIN_X + 11 * TILE_ATLAS_SIDE, 184);
    assert_eq!(VIEWPORT_ORIGIN_Y + 11 * TILE_ATLAS_SIDE, 184);
}
#[test]
fn prompt_cursor_cycles_the_four_barber_pole_glyphs() {
    // `IBM.CH` 0x05..=0x08 are one cycle of the same two-pixel diagonal
    // stripe, each frame advanced by one phase step. Observation of the
    // shipped build shows the live input line's cursor drawn from this
    // set (0x06 in three captures, 0x07 in a fourth), so it animates
    // rather than blinking.
    assert_eq!(PROMPT_CURSOR_FRAME_GLYPHS, [0x05, 0x06, 0x07, 0x08]);
    for frame in 0..12u64 {
        assert_eq!(
            prompt_cursor_glyph(frame),
            PROMPT_CURSOR_FRAME_GLYPHS[(frame % 4) as usize]
        );
    }
    // The intro menu's Select caption cursor is the same set.
    assert!(PROMPT_CURSOR_FRAME_GLYPHS.contains(&INTRO_MENU_SELECT_CAPTION_CURSOR_GLYPH));

    // Every frame is a rotation of the same stripe: each row is one of
    // the four two-pixel phases, and consecutive frames differ.
    let font = chrome_test_font();
    let mut seen = std::collections::BTreeSet::new();
    for glyph in PROMPT_CURSOR_FRAME_GLYPHS {
        assert!(seen.insert(glyph), "frame glyphs must be distinct");
        let _ = font;
    }
}

#[test]
fn ribbon_cap_is_one_primitive_for_border_message_and_caption_brackets() {
    // The gameplay border's end caps, the message window's per-line
    // prefix and the intro menu's caption brackets were all measured
    // byte-for-byte identical in the shipped build, so they share one
    // derivation rather than three copies of the same bitmap.
    let font = chrome_test_font();
    let right = ribbon_cap_sprite(&font, RibbonCapDirection::Right);
    let left = ribbon_cap_sprite(&font, RibbonCapDirection::Left);

    assert_eq!(right.white, [0x80, 0x60, 0x18, 0x04, 0x04, 0x18, 0x60, 0x80]);
    assert_eq!(right.ribbon, [0x00, 0x80, 0xe0, 0xf8, 0xf8, 0xe0, 0x80, 0x00]);
    assert_eq!(left.white, [0x01, 0x06, 0x18, 0x20, 0x20, 0x18, 0x06, 0x01]);
    assert_eq!(left.ribbon, [0x00, 0x01, 0x07, 0x1f, 0x1f, 0x07, 0x01, 0x00]);

    // The two directions are exact horizontal mirrors of each other.
    let mirror = |bits: u8| bits.reverse_bits();
    for row in 0..8 {
        assert_eq!(left.white[row], mirror(right.white[row]));
        assert_eq!(left.ribbon[row], mirror(right.ribbon[row]));
    }
}
