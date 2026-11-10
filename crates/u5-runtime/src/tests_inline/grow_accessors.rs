// Phase B M1-08 (widescreen design 2.3.1): the read-only accessors the
// grow-left compositor reads - the chrome's tab extents and the stats
// panel's counter and date rows. Each must agree with what the faithful
// frame paints, and none may change the state.

/// A font with every glyph blank: the chrome pass then paints the gap
/// cells (caps included) black, so a gap's extent is visible as pixels.
fn grow_blank_font() -> FixedCellFont {
    parse_ch_font(&vec![0u8; CH_FONT_LEN], IBM_CH_FILE).unwrap()
}

fn grow_chrome_frame(content: &GameplayChromeContent) -> Vec<u8> {
    let font = grow_blank_font();
    let mut rgba = vec![0u8; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4];
    for pixel in rgba.as_chunks_mut::<4>().0 {
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

/// The painted frame has no band blue anywhere in the extent on the band
/// lines that are not the rule (the blank font leaves the gap black but
/// for the caps' white accent strokes), and band blue just outside it.
fn assert_gap_painted_at(rgba: &[u8], rect: GapPixelRect, rule_y: usize, label: &str) {
    for y in rect.y0..=rect.y1 {
        if y == rule_y {
            continue;
        }
        for x in rect.x0..=rect.x1 {
            let index = chrome_index_at(rgba, x, y);
            assert!(
                index == 0 || index == CHROME_RULE_INDEX,
                "{label}: ({x}, {y}) inside the gap is {index}"
            );
            // Between the caps there is only the (blank) content.
            if x >= rect.x0 + CH_CELL_SIDE && x + CH_CELL_SIDE <= rect.x1 {
                assert_eq!(index, 0, "{label}: ({x}, {y}) content");
            }
        }
        assert_eq!(
            chrome_index_at(rgba, rect.x0 - 1, y),
            CHROME_RIBBON_INDEX,
            "{label}: left of the gap at y {y}"
        );
        assert_eq!(
            chrome_index_at(rgba, rect.x1 + 1, y),
            CHROME_RIBBON_INDEX,
            "{label}: right of the gap at y {y}"
        );
    }
}

fn grow_chrome_cases() -> Vec<(&'static str, GameplayChromeContent)> {
    let mut cases = vec![("unbroken", GameplayChromeContent::default())];
    let mut town = test_state(open_grid(), 1, 1);
    town.wind = WindState::South;
    town.wind_save_byte = WindState::South.save_byte();
    cases.push(("town", gameplay_chrome_content(&town)));
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.player.facing = Direction::East;
    cases.push(("dungeon", gameplay_chrome_content(&dungeon)));
    for label in ["N", "Northeast Winds", "a label longer than the banner field"] {
        cases.push((
            label,
            GameplayChromeContent {
                top: ChromeGap::Label(label.to_string()),
                bottom: ChromeGap::Label(label.to_string()),
                ..GameplayChromeContent::default()
            },
        ));
    }
    cases
}

#[test]
fn chrome_gap_extents_are_the_gaps_the_chrome_pass_paints() {
    for (label, content) in grow_chrome_cases() {
        let extents = gameplay_chrome_gap_extents(&content);
        assert_eq!(extents.top, top_gap(&content.top), "{label}");
        assert_eq!(extents.bottom, bottom_gap(&content.bottom), "{label}");
        let rgba = grow_chrome_frame(&content);
        match extents.top_rect() {
            Some(rect) => {
                assert_eq!((rect.y0, rect.y1), (0, 7), "{label}");
                assert_gap_painted_at(&rgba, rect, 7, &format!("{label} top"));
            }
            None => {
                for x in 8..184 {
                    assert_eq!(chrome_index_at(&rgba, x, 3), CHROME_RIBBON_INDEX, "{label}");
                }
            }
        }
        match extents.bottom_rect() {
            Some(rect) => {
                assert_eq!((rect.y0, rect.y1), (184, 191), "{label}");
                assert_gap_painted_at(&rgba, rect, 184, &format!("{label} bottom"));
            }
            None => {
                for x in 8..184 {
                    assert_eq!(chrome_index_at(&rgba, x, 188), CHROME_RIBBON_INDEX, "{label}");
                }
            }
        }
    }
}

#[test]
fn chrome_gap_extents_of_the_town_tabs() {
    let mut town = test_state(open_grid(), 1, 1);
    town.wind = WindState::South;
    town.wind_save_byte = WindState::South.save_byte();
    let before = format!("{town:?}");
    let extents = gameplay_chrome_gap_extents(&gameplay_chrome_content(&town));
    assert_eq!(format!("{town:?}"), before, "read-only");
    // Twelve sky cells from column 6, caps at 5 and 18: the tab is centred
    // on the viewport (x 8..183, centre 96).
    let sky = extents.top_rect().unwrap();
    assert_eq!((sky.x0, sky.x1, sky.width()), (40, 151, 112));
    assert_eq!(sky.x0 + sky.x1 + 1, 2 * 96);
    // `South Winds` (eleven cells) from the fixed column 7, caps at 6 and 18.
    let wind = extents.bottom_rect().unwrap();
    assert_eq!((wind.x0, wind.x1, wind.width()), (48, 151, 104));
    assert_eq!(extents.bottom.unwrap().content_cells, 11);
}

/// What the faithful panel paints at screen rows 8 and 9, columns 24..=39.
fn painted_counter_and_date(state: &PlayState) -> (String, String) {
    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    paint_stats_panel_text_window(&mut system, state, None);
    let rows = system.region_rows(
        STATS_PANEL_TEXT_LEFT,
        STATS_COUNTER_TOP,
        STATS_PANEL_TEXT_LEFT + STATS_PANEL_WIDTH as u8,
        STATS_COUNTER_BOTTOM,
        b' ',
    );
    (rows[0].clone(), rows[1].clone())
}

fn assert_rows_match_the_panel(state: &PlayState, label: &str) {
    let before = format!("{state:?}");
    let counter = stats_panel_counter_text(state).to_string();
    let date = stats_panel_date_text(state).to_string();
    assert_eq!(format!("{state:?}"), before, "{label}: read-only");
    let (painted_counter, painted_date) = painted_counter_and_date(state);
    assert_eq!(painted_counter.trim_end(), counter.trim_end(), "{label}: counter");
    // Only trailing blanks are trimmed: the date's leading centring spaces
    // are compared too.
    assert_eq!(painted_date.trim_end(), date.trim_end(), "{label}: date");
}

#[test]
fn stats_row_accessors_return_what_the_panel_paints() {
    let mut cases: Vec<(&str, PlayState)> = Vec::new();

    let mut plain = test_state(open_grid(), 1, 1);
    plain.food = 63;
    plain.gold = 150;
    plain.clock.month = 4;
    plain.clock.day = 5;
    plain.clock.year = 139;
    plain.repaint_stats_panel();
    assert_eq!(stats_panel_counter_text(&plain).trim_end(), "F:63      G:150");
    assert_eq!(stats_panel_date_text(&plain).trim_end(), "    4-5-139");
    cases.push(("plain", plain.clone()));

    let mut wide = plain.clone();
    wide.food = 9999;
    wide.gold = 9999;
    wide.clock.month = 12;
    wide.clock.day = 25;
    wide.repaint_stats_panel();
    cases.push(("9999 and a two-digit date", wide.clone()));

    let mut edited = plain.clone();
    edited.gold = 65535;
    edited.repaint_stats_panel();
    cases.push(("edited gold past column 38", edited));

    let mut ship = plain.clone();
    ship.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 150,
        skiffs: 1,
    };
    ship.repaint_stats_panel();
    assert_eq!(stats_panel_counter_text(&ship), "F:63    Ship:150");
    cases.push(("ship, hull into column 39", ship));

    // A stale panel: the wishing well's debit is not on screen yet, so the
    // accessor must not show it either.
    let mut stale = plain.clone();
    stale.gold = 149;
    stale.clock.day = 6;
    assert_eq!(stats_panel_counter_text(&stale).trim_end(), "F:63      G:150");
    assert_eq!(stats_panel_date_text(&stale).trim_end(), "    4-5-139");
    cases.push(("stale", stale));

    let mut held = plain.clone();
    held.stats_panel_hold = Some(crate::stats_panel::stats_panel_snapshot_of(&wide));
    assert_eq!(stats_panel_counter_text(&held), wide.stats_panel.counter_row);
    cases.push(("staged hold", held));

    let mut inverse = plain.clone();
    inverse.stats_panel_window_inverse = true;
    cases.push(("inverted window", inverse));

    let empty = test_state(open_grid(), 1, 1);
    cases.push(("never refreshed", empty));

    for (label, state) in &cases {
        assert_rows_match_the_panel(state, label);
    }
}
