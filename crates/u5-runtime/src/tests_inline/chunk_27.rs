
    /// Local clean asset folder, or `None` when it is not installed. Every
    /// test that reads original game data is skipped without it; the runtime
    /// crate never embeds asset bytes.
    fn local_clean_assets() -> Option<std::path::PathBuf> {
        let dir = std::env::var_os("U5_CLEAN_ASSETS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_GAME_DIR));
        dir.join(PROPORT_PCS_FILE).is_file().then_some(dir)
    }

    #[test]
    fn fonts_proportional_font_directory_matches_the_shipped_asset_shape() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        let font = load_proportional_font(&dir).expect("PROPORT.PCS glyph directory loads");

        // `formats/font-pcs.md §8` leaves the glyph inventory unpublished; the
        // shipped directory holds one glyph per code 0x20..=0x7a.
        assert_eq!(font.first_code, PCS_FIRST_CODE);
        assert_eq!(font.glyphs.len(), 91);
        for (slot, glyph) in font.glyphs.iter().enumerate() {
            assert_eq!(
                (glyph.bitmap.width, glyph.bitmap.height),
                (PCS_GLYPH_BITMAP_WIDTH, PCS_GLYPH_HEIGHT),
                "glyph slot {slot} cell size"
            );
            assert!(
                usize::from(glyph.advance_width) <= PCS_GLYPH_BITMAP_WIDTH,
                "glyph slot {slot} ink width {} exceeds the cell",
                glyph.advance_width
            );
            // The stored width word is the glyph's ink width: no lit pixel may
            // fall outside it, and (except for the blank space glyph) the
            // rightmost ink column must be the last one.
            let ink_columns = (0..PCS_GLYPH_BITMAP_WIDTH)
                .filter(|x| {
                    (0..PCS_GLYPH_HEIGHT).any(|y| glyph.bitmap.pixel(*x, y) == Some(1))
                })
                .collect::<Vec<_>>();
            let widest = ink_columns.last().map_or(0, |x| x + 1);
            assert_eq!(
                widest,
                usize::from(glyph.advance_width),
                "glyph slot {slot} ink extent does not match its stored width word"
            );
        }
    }

    #[test]
    fn fonts_proportional_advance_table_matches_shipped_font() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        let font = load_proportional_font(&dir).expect("PROPORT.PCS glyph directory loads");

        // `cleak/u5-spec#70`: the checked-in observation-derived advance table
        // must equal the fitted rule applied to the shipped glyph strips.
        assert_eq!(
            proportional_advance_table_from_font(&font),
            PROPORTIONAL_ADVANCE_TABLE
        );
        assert_eq!(
            PROPORTIONAL_ADVANCE_TABLE.width_for_byte(b' ').unwrap(),
            usize::from(PCS_SPACE_ADVANCE)
        );
        for code in PCS_FIRST_CODE..=b'z' {
            let glyph = font.glyph_for_code(code).expect("shipped glyph");
            let expected = if code == b' ' {
                usize::from(PCS_SPACE_ADVANCE)
            } else {
                usize::from(glyph.advance_width) + usize::from(PCS_GLYPH_ADVANCE_GAP)
            };
            assert_eq!(
                PROPORTIONAL_ADVANCE_TABLE.width_for_byte(code).unwrap(),
                expected,
                "advance for code {code}"
            );
        }
        // Codes with no glyph in the shipped directory stay at zero rather
        // than being guessed; the layout engine rejects them.
        for code in 0..PCS_FIRST_CODE {
            assert_eq!(PROPORTIONAL_ADVANCE_TABLE.width_for_byte(code).unwrap(), 0);
        }
        for code in (b'z' + 1)..(PROPORTIONAL_WIDTH_TABLE_LEN as u8) {
            assert_eq!(PROPORTIONAL_ADVANCE_TABLE.width_for_byte(code).unwrap(), 0);
        }
    }

    #[test]
    fn story_layout_publishes_a_region_for_every_text_consuming_step() {
        assert!(intro_story_text_region(INTRO_INLINE_DOORWAY_STEP).is_none());
        assert!(intro_story_text_region(INTRO_STORY_STEP_COUNT).is_none());
        for step in 0..INTRO_STORY_STEP_COUNT {
            if step == INTRO_INLINE_DOORWAY_STEP {
                continue;
            }
            let region = intro_story_text_region(step)
                .unwrap_or_else(|| panic!("intro story step {step} needs a text region"));
            assert_eq!(region.left, INTRO_STORY_TEXT_LEFT);
            assert_eq!(region.right, INTRO_STORY_TEXT_RIGHT);
            assert!(region.top_y < 200, "step {step} text top {}", region.top_y);
            if let Some(gutter) = region.gutter {
                assert!(gutter.top_y <= gutter.bottom_y, "step {step} gutter rows");
                assert!(gutter.left < gutter.right, "step {step} gutter columns");
                assert!(gutter.right <= INTRO_STORY_TEXT_RIGHT, "step {step} gutter right");
            }
        }
    }

    #[test]
    fn story_layout_line_bounds_narrow_only_inside_the_gutter() {
        // Step 13 draws STORY6.16 #0 at (176, 0); the measured text is
        // narrowed to x 0..168 through line y=108 and runs full width from
        // line y=117.
        let region = intro_story_text_region(13).unwrap();
        assert_eq!(region.line_bounds(0), (0, 168));
        assert_eq!(region.line_bounds(108), (0, 168));
        assert_eq!(
            region.line_bounds(117),
            (INTRO_STORY_TEXT_LEFT, INTRO_STORY_TEXT_RIGHT)
        );
        // Step 3 narrows in the middle: full width above and below the art.
        let region = intro_story_text_region(3).unwrap();
        assert_eq!(
            region.line_bounds(27),
            (INTRO_STORY_TEXT_LEFT, INTRO_STORY_TEXT_RIGHT)
        );
        assert_eq!(region.line_bounds(36), (210, 318));
        assert_eq!(region.line_bounds(153), (210, 318));
        assert_eq!(
            region.line_bounds(162),
            (INTRO_STORY_TEXT_LEFT, INTRO_STORY_TEXT_RIGHT)
        );
    }

    fn uniform_advance_table(advance: u8) -> ProportionalWidthTable {
        let mut widths = [0u8; PROPORTIONAL_WIDTH_TABLE_LEN];
        for code in PCS_FIRST_CODE..=b'z' {
            widths[usize::from(code)] = advance;
        }
        widths[usize::from(b' ')] = PCS_SPACE_ADVANCE;
        ProportionalWidthTable::new(widths)
    }

    #[test]
    fn story_layout_indents_paragraphs_and_starts_at_the_region_top() {
        let widths = uniform_advance_table(5);
        let region = ProportionalTextRegion::full_width(20);
        let placed =
            layout_proportional_justified_paragraph(&widths, &region, b"\n{ab cd\0", 200).unwrap();
        // The leading hard newline costs one line, then `{` indents by 15.
        assert_eq!(placed[0].y, 20 + PROPORTIONAL_LINE_STRIDE);
        assert_eq!(placed[0].x, PROPORTIONAL_PARAGRAPH_INDENT);
        assert_eq!(placed[0].code, b'a');
        // The final line of a paragraph keeps natural 5-pixel spaces.
        let space_gap = placed[2].x - placed[1].x;
        assert_eq!(space_gap, 5 + u16::from(PCS_SPACE_ADVANCE));
    }

    #[test]
    fn story_layout_justifies_every_line_but_the_last_of_a_paragraph() {
        let widths = uniform_advance_table(5);
        // Each glyph advances 5 px (4 px of ink plus the separator column),
        // so four two-glyph words plus three natural 5 px spaces measure
        // 53 px of ink. A rectangle ending at column 58 cannot take the fifth
        // word, and justifying the first line spreads 5 extra pixels over its
        // three spaces as 6, 7 and 7.
        let region = ProportionalTextRegion {
            top_y: 0,
            left: 0,
            right: 58,
            gutter: None,
            first_line_left: None,
            space_advance: PCS_SPACE_ADVANCE,
        };
        let placed =
            layout_proportional_justified_paragraph(&widths, &region, b"aa bb cc dd ee\0", 200)
                .unwrap();
        let first_line: Vec<_> = placed.iter().filter(|glyph| glyph.y == 0).collect();
        assert_eq!(first_line.len(), 8);
        // Justified: the last glyph's rightmost ink column lands on `right`.
        let last = first_line.last().unwrap();
        let ink_width = 5 - usize::from(PCS_GLYPH_ADVANCE_GAP);
        assert_eq!(
            usize::from(last.x) + ink_width - 1,
            usize::from(region.right)
        );
        let gaps: Vec<u16> = (0..3)
            .map(|word| first_line[word * 2 + 2].x - first_line[word * 2 + 1].x - 5)
            .collect();
        assert_eq!(gaps, vec![6, 7, 7], "extra pixels go to the rightmost spaces");
        // The paragraph's last line is left-aligned with natural spacing.
        let second_line: Vec<_> = placed
            .iter()
            .filter(|glyph| glyph.y == PROPORTIONAL_LINE_STRIDE)
            .collect();
        assert_eq!(second_line.len(), 2);
        assert_eq!(second_line[0].x, 0);
    }

    #[test]
    fn story_layout_breaks_words_at_soft_hyphens_and_draws_the_hyphen() {
        let widths = uniform_advance_table(5);
        let region = ProportionalTextRegion {
            top_y: 0,
            left: 0,
            right: 39,
            gutter: None,
            first_line_left: None,
            space_advance: PCS_SPACE_ADVANCE,
        };
        let placed =
            layout_proportional_justified_paragraph(&widths, &region, b"aa bb_cc_dd\0", 200)
                .unwrap();
        let first: String = placed
            .iter()
            .filter(|glyph| glyph.y == 0)
            .map(|glyph| glyph.code as char)
            .collect();
        let second: String = placed
            .iter()
            .filter(|glyph| glyph.y == PROPORTIONAL_LINE_STRIDE)
            .map(|glyph| glyph.code as char)
            .collect();
        assert_eq!(first, "aabbcc-");
        assert_eq!(second, "dd");
        // The soft-break marker itself never reaches the framebuffer.
        assert!(
            placed
                .iter()
                .all(|glyph| glyph.code != STORY_SOFT_BREAK_MARKER)
        );
    }

    #[test]
    fn story_layout_rejects_codes_the_advance_table_never_measured() {
        let widths = uniform_advance_table(5);
        let region = ProportionalTextRegion::full_width(0);
        assert!(
            layout_proportional_justified_paragraph(&widths, &region, b"a\x01b\0", 200).is_err()
        );
    }

    /// Golden geometry for the twenty text-consuming intro story steps,
    /// measured off a black-box run of the original (`cleak/u5-spec#70`):
    /// `(step, first line y, first glyph x, last line y, last glyph's right
    /// ink column, line count, glyph count)`. Only geometry and counts are
    /// recorded here; the narrative text itself stays in `STORY.DAT`.
    const INTRO_STORY_MEASURED_GEOMETRY: [(usize, u16, u16, u16, u16, usize, usize); 20] = [
        (0, 128, 195, 191, 286, 8, 214),
        (1, 9, 15, 171, 274, 19, 531),
        (2, 67, 15, 166, 289, 12, 341),
        (3, 0, 15, 180, 84, 21, 484),
        (4, 18, 15, 189, 55, 20, 548),
        (5, 0, 191, 171, 223, 20, 516),
        (7, 136, 203, 181, 164, 6, 142),
        (8, 9, 15, 63, 207, 7, 313),
        (9, 0, 15, 72, 89, 9, 350),
        (10, 0, 15, 63, 178, 8, 352),
        (11, 9, 15, 63, 179, 7, 312),
        (12, 9, 15, 63, 97, 7, 288),
        (13, 0, 15, 171, 223, 20, 595),
        (14, 41, 199, 185, 141, 17, 483),
        (15, 0, 15, 162, 304, 19, 574),
        (16, 0, 15, 171, 35, 20, 596),
        (17, 9, 15, 162, 105, 18, 571),
        (18, 0, 189, 180, 46, 21, 646),
        (19, 9, 15, 171, 205, 19, 631),
        (20, 9, 15, 171, 213, 19, 604),
    ];

    #[test]
    fn story_layout_reproduces_the_measured_intro_slide_geometry() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        if !dir.join(STORY_DAT_FILE).is_file() {
            return;
        }
        let Some(records) = load_story_records(&dir).expect("STORY.DAT loads") else {
            return;
        };
        for (step, first_y, first_x, last_y, last_right, line_count, glyph_count) in
            INTRO_STORY_MEASURED_GEOMETRY
        {
            let record_index = if step < INTRO_INLINE_DOORWAY_STEP {
                step
            } else {
                step - 1
            };
            let text = records
                .record(record_index)
                .unwrap_or_else(|| panic!("STORY.DAT record {record_index}"));
            let region = intro_story_text_region(step).unwrap();
            let placed = layout_proportional_justified_paragraph(
                &PROPORTIONAL_ADVANCE_TABLE,
                &region,
                text.as_bytes(),
                200,
            )
            .unwrap_or_else(|err| panic!("intro story step {step} layout: {err}"));

            assert_eq!(placed.len(), glyph_count, "step {step} glyph count");
            assert_eq!(placed[0].y, first_y, "step {step} first line row");
            assert_eq!(placed[0].x, first_x, "step {step} first glyph column");
            let last = placed.last().unwrap();
            assert_eq!(last.y, last_y, "step {step} last line row");
            let last_width = PROPORTIONAL_ADVANCE_TABLE.width_for_byte(last.code).unwrap()
                - usize::from(PCS_GLYPH_ADVANCE_GAP);
            assert_eq!(
                usize::from(last.x) + last_width - 1,
                usize::from(last_right),
                "step {step} last glyph right column"
            );
            let rows = placed
                .iter()
                .map(|glyph| glyph.y)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(rows.len(), line_count, "step {step} line count");
            for row in &rows {
                assert_eq!(
                    (row - first_y) % PROPORTIONAL_LINE_STRIDE,
                    0,
                    "step {step} row {row} is off the 9-pixel line grid"
                );
            }
            // Every glyph must sit inside its line's measured bounds.
            for glyph in &placed {
                let (left, right) = region.line_bounds(glyph.y);
                let left = match region.first_line_left {
                    Some(first_left) if glyph.y == first_y => first_left,
                    _ => left,
                };
                let width = PROPORTIONAL_ADVANCE_TABLE.width_for_byte(glyph.code).unwrap()
                    - usize::from(PCS_GLYPH_ADVANCE_GAP);
                assert!(glyph.x >= left, "step {step} glyph left of its band");
                assert!(
                    usize::from(glyph.x) + width - 1 <= usize::from(right),
                    "step {step} glyph past its band right edge"
                );
            }
        }
    }


    /// Golden geometry for the three character-creation proportional screens,
    /// measured off captures of the original (`cleak/u5-spec#70`):
    /// `(QUESTION.DAT record, first line y, first glyph x, last line y, last
    /// glyph's right ink column, line count, glyph count)`. Record 15 is the
    /// dilemma shown in the captured question screen; every dilemma record
    /// uses the same rectangle.
    const CHARGEN_MEASURED_GEOMETRY: [(usize, u16, u16, u16, u16, usize, usize); 3] = [
        (0, 9, 15, 180, 288, 20, 617),
        (1, 0, 15, 189, 112, 22, 705),
        (15, 152, 0, 179, 280, 4, 179),
    ];

    #[test]
    fn story_layout_reproduces_the_measured_chargen_geometry() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        if !dir.join(QUESTION_DAT_FILE).is_file() {
            return;
        }
        let Some(records) = load_question_records(&dir).expect("QUESTION.DAT loads") else {
            return;
        };
        for (record, first_y, first_x, last_y, last_right, line_count, glyph_count) in
            CHARGEN_MEASURED_GEOMETRY
        {
            let region = match record {
                0 => CHARGEN_GYPSY_TEXT_REGION,
                1 => CHARGEN_RESULT_TEXT_REGION,
                _ => CHARGEN_QUESTION_TEXT_REGION,
            };
            let text = records
                .records
                .get(record)
                .unwrap_or_else(|| panic!("QUESTION.DAT record {record}"));
            let placed = layout_proportional_justified_paragraph(
                &PROPORTIONAL_ADVANCE_TABLE,
                &region,
                text.as_bytes(),
                200,
            )
            .unwrap_or_else(|err| panic!("chargen record {record} layout: {err}"));

            assert_eq!(placed.len(), glyph_count, "record {record} glyph count");
            assert_eq!(placed[0].y, first_y, "record {record} first line row");
            assert_eq!(placed[0].x, first_x, "record {record} first glyph column");
            let last = placed.last().unwrap();
            assert_eq!(last.y, last_y, "record {record} last line row");
            let last_width = PROPORTIONAL_ADVANCE_TABLE.width_for_byte(last.code).unwrap()
                - usize::from(PCS_GLYPH_ADVANCE_GAP);
            assert_eq!(
                usize::from(last.x) + last_width - 1,
                usize::from(last_right),
                "record {record} last glyph right column"
            );
            let rows = placed
                .iter()
                .map(|glyph| glyph.y)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(rows.len(), line_count, "record {record} line count");
        }
    }

