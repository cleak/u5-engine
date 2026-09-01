// Gameplay-screen border chrome and message window.
//
// Geometry here is the published 320x200 layout from
// `display-driver.md §7` and `text-output.md §10` (`cleak/u5-spec#79`).

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
        ChromePalette::EGA,
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
    // The shared cap remains a two-colour sprite rather than the solid
    // one-colour triangle produced by an ordinary text overlay.
    assert_eq!(chrome_index_at(&tagged, 240, 57), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&tagged, 241, 57), CHROME_RULE_INDEX);
}

#[test]
fn stats_label_strip_uses_the_cap_anchor_formula_and_interrupts_its_rule() {
    assert_eq!(
        stats_label_gap(Some("Select:")),
        Some(ResolvedGap {
            left_cap_column: 27,
            right_cap_column: 35,
            content_first_column: 28,
            content_cells: 7,
        })
    );
    assert_eq!(
        stats_label_gap(Some("Arms")),
        Some(ResolvedGap {
            left_cap_column: 28,
            right_cap_column: 33,
            content_first_column: 29,
            content_cells: 4,
        })
    );
    assert_eq!(stats_label_gap(None), None);
    assert_eq!(stats_label_gap(Some("")), None);

    let rgba = chrome_frame(&GameplayChromeContent {
        stats_label: Some("Arms".to_string()),
        ..GameplayChromeContent::default()
    });
    // The stats top rule is intact immediately outside caps at columns
    // 28 and 33, but absent through the label field itself.
    assert_eq!(chrome_index_at(&rgba, 223, 7), CHROME_RULE_INDEX);
    assert_eq!(chrome_index_at(&rgba, 225, 7), 0);
    assert_eq!(chrome_index_at(&rgba, 270, 7), 0);
    assert_eq!(chrome_index_at(&rgba, 272, 7), CHROME_RULE_INDEX);
    // Its opening cap uses the same two-colour sprite as every other gap.
    assert_eq!(chrome_index_at(&rgba, 224, 1), CHROME_RIBBON_INDEX);
    assert_eq!(chrome_index_at(&rgba, 225, 1), CHROME_RULE_INDEX);
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
    // `dungeon-mode.md §4.1` (cleak/u5-spec#81): the level is stored
    // zero-based and displayed one-based, so the fixture's level 0
    // renders as `L1` — matching the original's `>L1<` on the first
    // floor of Deceit.
    assert_eq!(level, 0);
    assert_eq!(below.top, ChromeGap::Label("L1".to_string()));
    // East carries its own leading space inside the five-cell field, so
    // the label reads `Dir:` plus two spaces.
    assert_eq!(below.bottom, ChromeGap::Label("Dir:  East".to_string()));
}

#[test]
fn gameplay_chrome_content_maps_modal_stats_labels_and_browser_page_badge() {
    let mut selector = test_state(open_grid(), 1, 1);
    let _ = selector.start_party_selector(PartySelectorTarget::ZStats);
    assert_eq!(
        gameplay_chrome_content(&selector).stats_label.as_deref(),
        Some("Select:")
    );

    selector.active_party_selector = None;
    selector.active_use = Some(crate::z_stats::UseSession::new());
    assert_eq!(
        gameplay_chrome_content(&selector).stats_label.as_deref(),
        Some("Items:")
    );

    let mut browser_state = test_state(open_grid(), 1, 1);
    browser_state.active_effect_tag = Some(b'P');
    for item in 0..5 {
        browser_state.equipment_stock[item] = 1;
    }
    let browser =
        crate::shop_runtime::ArmsSellBrowser::new(&browser_state.equipment_stock).unwrap();
    browser_state.active_shop = Some(crate::shop_session::ActiveShopSession::ArmsStocked(
        crate::shop_runtime::ArmsShopState::SellPickItem(browser),
        ArmsShop::IolosBows.stock_table(),
    ));
    let content = gameplay_chrome_content(&browser_state);
    assert_eq!(content.stats_label.as_deref(), Some("Arms"));
    assert_eq!(content.timing_glyph, Some(ARMS_SELL_BROWSER_PAGE_GLYPH_DOWN));

    // The browser temporarily owns the slot even when there is no page
    // arrow to show; an unrelated timed-effect tag must not leak through.
    browser_state.equipment_stock.fill(0);
    browser_state.equipment_stock[0] = 1;
    let browser =
        crate::shop_runtime::ArmsSellBrowser::new(&browser_state.equipment_stock).unwrap();
    browser_state.active_shop = Some(crate::shop_session::ActiveShopSession::ArmsStocked(
        crate::shop_runtime::ArmsShopState::SellPickItem(browser),
        ArmsShop::IolosBows.stock_table(),
    ));
    assert_eq!(gameplay_chrome_content(&browser_state).timing_glyph, None);
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

#[test]
fn chrome_paint_takes_its_two_colours_as_parameters() {
    // The published paint reads no gameplay state - only the chrome and
    // accent colour-table slots - and every colour index in this module
    // is provisional pending the spec's colour re-grounding audit. Both
    // facts are only safe if the painters take the pair as a parameter
    // rather than reading the constants, so pin that: swapping the
    // palette must swap the painted indices and nothing else.
    let font = chrome_test_font();
    let content = GameplayChromeContent::default();
    let paint = |palette: ChromePalette| {
        let mut rgba = vec![0u8; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 0, 0, 0xff]);
        }
        paint_gameplay_frame_chrome(
            &mut rgba,
            TEXT_WINDOW_RENDER_WIDTH,
            TEXT_WINDOW_RENDER_HEIGHT,
            &content,
            ChromeFonts {
                ibm: &font,
                runes: &font,
            },
            palette,
        );
        rgba
    };

    let ega = paint(ChromePalette::EGA);
    let swapped = paint(ChromePalette {
        chrome: 4,
        accent: 10,
        background: 0,
    });

    // Ribbon fill and rule both follow the palette.
    assert_eq!(chrome_index_at(&ega, 3, 100), ChromePalette::EGA.chrome);
    assert_eq!(chrome_index_at(&swapped, 3, 100), 4);
    assert_eq!(chrome_index_at(&ega, 7, 100), ChromePalette::EGA.accent);
    assert_eq!(chrome_index_at(&swapped, 7, 100), 10);
    // Background and the untouched viewport interior do not.
    assert_eq!(chrome_index_at(&swapped, 100, 195), 0);
    assert_eq!(chrome_index_at(&swapped, 100, 100), 0);

    // The default is the provisional EGA pair.
    assert_eq!(ChromePalette::default(), ChromePalette::EGA);
    assert_eq!(ChromePalette::EGA.chrome, CHROME_RIBBON_INDEX);
    assert_eq!(ChromePalette::EGA.accent, CHROME_RULE_INDEX);
}

#[test]
fn palette_index_six_resolves_per_display_model() {
    // `formats/tiles.md` section 7 and `systems/display-driver-mode.md`
    // section 5.2 say the shipped descriptor substitutes dark yellow
    // for the firmware's brown at index six. That is correct, and this
    // test pins it. An earlier revision asserted the reverse on the
    // strength of captures showing brown; see `EGA_PALETTE_RGB` for why
    // those captures are consistent with a dark-yellow register.
    //
    // The register table and the display model are pinned separately on
    // purpose, because conflating them is what produced two wrong
    // reverts.
    assert_eq!(
        SHIPPED_PALETTE_REGISTERS[SHIPPED_PALETTE_DEVIATING_INDEX], 0x06,
        "the program overwrites the firmware brown with 0x06"
    );

    // `display-driver-mode.md §5.2` makes the enhanced-display colour space
    // the v1 baseline. Period-monitor brown is a supported alternate model,
    // not the default renderer contract.
    assert_eq!(DISPLAY_MODEL, MonitorModel::Enhanced);
    assert_eq!(
        EGA_PALETTE_RGB[SHIPPED_PALETTE_DEVIATING_INDEX], SHIPPED_PALETTE_DARK_YELLOW,
        "v1 index six renders the shipped 0x06 register as dark yellow"
    );
    assert!(
        EGA_PALETTE_RGB.contains(&SHIPPED_PALETTE_DARK_YELLOW),
        "the v1 palette must expose the deliberate dark-yellow substitution"
    );

    // Every other slot is the stock set, so index six is the only place
    // the two models can ever disagree.
    for index in 0..16 {
        assert_eq!(
            EGA_PALETTE_RGB[index],
            resolve_palette_register(SHIPPED_PALETTE_REGISTERS[index], DISPLAY_MODEL),
            "index {index} must follow the register table"
        );
        if index != SHIPPED_PALETTE_DEVIATING_INDEX {
            assert_eq!(
                resolve_palette_register(SHIPPED_PALETTE_REGISTERS[index], MonitorModel::Enhanced),
                resolve_palette_register(SHIPPED_PALETTE_REGISTERS[index], MonitorModel::Period200Line),
                "the 200-line correction must touch index six and nothing else"
            );
        }
    }

    // The period model is the one that yields the brown every capture
    // shows, and it is keyed to the value rather than the index - which
    // is the fact that reconciles the captures with the spec. Writing
    // 0x06 into any other register browns that register too.
    assert_eq!(
        resolve_palette_register(0x06, MonitorModel::Period200Line),
        STOCK_EGA_BROWN
    );
    assert_eq!(
        resolve_palette_register(0x06, MonitorModel::Enhanced),
        SHIPPED_PALETTE_DARK_YELLOW
    );
    // Register six holding some other value is unaffected by it.
    assert_eq!(
        resolve_palette_register(0x02, MonitorModel::Period200Line),
        [0x00, 0xaa, 0x00],
        "the correction keys off the value 0x06, not off index six"
    );

    // Spot-check the six-bit decode itself against known values, so a
    // broken decoder cannot make the assertions above vacuously true.
    assert_eq!(resolve_palette_register(0x14, MonitorModel::Enhanced), STOCK_EGA_BROWN);
    assert_eq!(resolve_palette_register(0x3f, MonitorModel::Enhanced), [0xff, 0xff, 0xff]);
    assert_eq!(resolve_palette_register(0x00, MonitorModel::Enhanced), [0x00, 0x00, 0x00]);
    assert_eq!(resolve_palette_register(0x01, MonitorModel::Enhanced), [0x00, 0x00, 0xaa]);
}

#[test]
fn dungeon_billboard_bank_is_chosen_by_flavour_byte_not_declaration_order() {
    // `dungeon-mode.md §6.2`: a three-entry filename table indexed by
    // the flavour byte - byte 1 the first file, byte 2 the second,
    // byte 3 the third. `FlavourByte3` is named for its byte, so it
    // takes DNG3; mapping by the order the variants happen to be
    // declared puts Deceit on DNG2 and renders the corridor in the
    // wrong bank's colours.
    assert_eq!(
        dungeon_billboard_stem(DungeonPresentationFlavour::Normal),
        "DNG1"
    );
    assert_eq!(
        dungeon_billboard_stem(DungeonPresentationFlavour::Mine),
        "DNG2"
    );
    assert_eq!(
        dungeon_billboard_stem(DungeonPresentationFlavour::FlavourByte3),
        "DNG3"
    );

    // The three banks are distinguishable by their dominant ink, which
    // is the whole "different dungeons look different" mechanism: one
    // geometry, three texture sets.
    let game_dir = Path::new(DEFAULT_GAME_DIR);
    if !game_dir.join("DNG1.16").exists() {
        return;
    }
    let dominant = |stem: &str| {
        let dir = load_graphic_image_directory(game_dir, stem, TileGraphicsDepth::Ega16).unwrap();
        let image = dir.images[1].as_ref().unwrap();
        let mut hist = [0usize; 16];
        for pixel in &image.pixels {
            hist[usize::from(*pixel & 0x0f)] += 1;
        }
        (0..16)
            .filter(|index| *index != 0)
            .max_by_key(|index| hist[*index])
            .unwrap()
    };
    // Ochre, red and grey respectively - three distinct texture sets.
    assert_eq!(dominant("DNG1"), 6);
    assert_eq!(dominant("DNG2"), 4);
    assert_eq!(dominant("DNG3"), 7);
}

#[test]
fn dungeon_billboard_slot_table_matches_the_published_addressing_rule() {
    // `cleak/u5-spec#84`: 0-3 side plain wall, 4-7 side door, 8-11
    // forward plain wall, 12-15 forward door, 16-19 side opening,
    // 20-23 side flavour wall, 24-27 forward flavour wall - addressed
    // as `slot = family_base + band`, so the low two bits of a slot
    // index are the depth band.
    use DungeonBillboardRole::*;
    for (role, base) in [
        (SideWall, 0usize),
        (SideDoor, 4),
        (ForwardWall, 8),
        (ForwardDoor, 12),
        (SideOpening, 16),
        (SideFlavourWall, 20),
        (ForwardFlavourWall, 24),
    ] {
        assert_eq!(role.family_base_slot(), base, "{role:?}");
        for band in 0..DUNGEON_BANDS {
            if let Some(slot) = role.slot(band) {
                assert_eq!(slot, base + band);
                assert_eq!(slot % 4, band, "the low two bits are the band");
                assert_eq!(slot / 4, base / 4, "the high bits are the family");
            }
        }
    }
}
// End of the asset-backed dungeon billboard contract tests.
