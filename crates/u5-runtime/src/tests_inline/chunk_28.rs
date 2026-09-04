
    /// Local clean asset folder, or `None` when it is not installed. Every
    /// test that reads original game data is skipped without it; the runtime
    /// crate never embeds asset bytes.
    fn local_clean_assets() -> Option<std::path::PathBuf> {
        let dir = crate::test_fixtures::configured_original_asset_dir()?;
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
    fn fonts_proportional_width_table_matches_shipped_font() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        let font = load_proportional_font(&dir).expect("PROPORT.PCS glyph directory loads");

        // `font-pcs.md` section 4.1: entries 0x20..=0x7A are byte-identical to
        // the per-glyph widths stored in PROPORT.PCS.
        assert_eq!(
            proportional_width_table_from_font(&font),
            PROPORTIONAL_WIDTH_TABLE
        );
        for code in PCS_FIRST_CODE..=b'z' {
            let glyph = font.glyph_for_code(code).expect("shipped glyph");
            assert_eq!(
                PROPORTIONAL_WIDTH_TABLE.width_for_byte(code).unwrap(),
                usize::from(glyph.advance_width),
                "width for code {code}"
            );
        }
        // Space's own entry is zero: the space advance is descriptor state,
        // not font metrics. `{` is zero too and is intercepted anyway.
        assert_eq!(PROPORTIONAL_WIDTH_TABLE.width_for_byte(b' ').unwrap(), 0);
        assert_eq!(
            PROPORTIONAL_WIDTH_TABLE
                .width_for_byte(STORY_PARAGRAPH_START_MARKER)
                .unwrap(),
            0
        );
        // The hyphen entry the wrap test reads is 3.
        assert_eq!(PROPORTIONAL_WIDTH_TABLE.width_for_byte(b'-').unwrap(), 3);
        // 0x7B..=0x7F are zero; below 0x20 is not width data and stays zero.
        for code in 0..PCS_FIRST_CODE {
            assert_eq!(PROPORTIONAL_WIDTH_TABLE.width_for_byte(code).unwrap(), 0);
        }
        for code in (b'z' + 1)..(PROPORTIONAL_WIDTH_TABLE_LEN as u8) {
            assert_eq!(PROPORTIONAL_WIDTH_TABLE.width_for_byte(code).unwrap(), 0);
        }
    }

    #[test]
    fn story_layout_publishes_a_paragraph_box_for_every_intro_step() {
        assert!(intro_story_paragraph_box(INTRO_STORY_STEP_COUNT).is_none());
        for step in 0..INTRO_STORY_STEP_COUNT {
            let boxed = intro_story_paragraph_box(step)
                .unwrap_or_else(|| panic!("intro story step {step} needs a paragraph box"));
            assert!(boxed.left_a < boxed.right_a, "step {step} pair A");
            assert!(boxed.left_b < boxed.right_b, "step {step} pair B");
            assert!(boxed.right_a <= 320 && boxed.right_b <= 320, "step {step} right");
            assert!(boxed.band_low <= boxed.band_high, "step {step} band");
            assert_eq!(
                boxed.space_advance, PROPORTIONAL_DEFAULT_SPACE_ADVANCE,
                "step {step} keeps the shipped space advance"
            );
            assert!(boxed.pen_y < 200, "step {step} pen origin row");
        }
        // Step 6 has an ordinary paragraph box like every other step; it just
        // renders the inline doorway lines instead of a STORY.DAT record.
        let doorway = intro_story_paragraph_box(INTRO_INLINE_DOORWAY_STEP).unwrap();
        assert_eq!((doorway.pen_x, doorway.pen_y), (32, 9));
        assert_eq!((doorway.left_a, doorway.right_a), (0, 320));
        assert_eq!(doorway.band_low, doorway.band_high);
    }

    #[test]
    fn story_layout_selects_margin_pair_b_strictly_inside_the_band() {
        // `text-output.md` section 8.1: pair B while band_low < pen_y <
        // band_high, pair A otherwise. Step 13's art column is on the right,
        // so pair A is the narrow one and the band starts below the art.
        let boxed = intro_story_paragraph_box(13).unwrap();
        assert_eq!(boxed.margins_for(0), (0, 170));
        assert_eq!(boxed.margins_for(108), (0, 170));
        assert_eq!(boxed.margins_for(114), (0, 170), "band ends are excluded");
        assert_eq!(boxed.margins_for(117), (0, 320));
        // Step 3 narrows in the middle: full width above and below the art.
        let boxed = intro_story_paragraph_box(3).unwrap();
        assert_eq!(boxed.margins_for(27), (0, 320));
        assert_eq!(boxed.margins_for(36), (210, 320));
        assert_eq!(boxed.margins_for(153), (210, 320));
        assert_eq!(boxed.margins_for(162), (0, 320));
        // A 200..200 band can never match.
        let boxed = intro_story_paragraph_box(9).unwrap();
        assert_eq!(boxed.margins_for(199), (0, 320));
    }

    #[test]
    fn story_layout_publishes_the_inline_doorway_lines_and_their_origins() {
        // `systems/intro.md` section 10.1, answering `cleak/u5-spec#69`.
        assert_eq!(INTRO_DOORWAY_LINES.len(), 2);
        for line in INTRO_DOORWAY_LINES {
            assert_eq!(line.len(), 45, "each doorway line is 45 characters");
            assert!(
                !line.contains(STORY_PARAGRAPH_START_MARKER as char)
                    && !line.contains(STORY_SOFT_BREAK_MARKER as char)
                    && !line.contains(STORY_HARD_NEWLINE_MARKER as char),
                "the doorway lines carry none of the STORY.DAT markers"
            );
        }
        let [first, second] = intro_doorway_paragraph_boxes();
        assert_eq!((first.pen_x, first.pen_y), (32, 9));
        assert_eq!((second.pen_x, second.pen_y), (32, INTRO_DOORWAY_SECOND_LINE_PEN_Y));
        // Explicit origins, not the renderer's line advance.
        assert_ne!(second.pen_y - first.pen_y, PROPORTIONAL_LINE_STRIDE);
        // Neither line wraps or is justified: each ends at its terminator.
        for (boxed, line) in intro_doorway_paragraph_boxes().iter().zip(INTRO_DOORWAY_LINES) {
            let mut text = line.as_bytes().to_vec();
            text.push(0);
            let placed =
                layout_proportional_paragraph_glyphs(&PROPORTIONAL_WIDTH_TABLE, boxed, &text, PROPORTIONAL_DRAW_CLIP_Y)
                    .unwrap();
            assert!(
                placed.iter().all(|glyph| glyph.y == boxed.pen_y),
                "a doorway line must not wrap"
            );
            assert_eq!(placed[0].x, 32);
            // Natural, unjustified spacing: every gap is the plain advance.
            let spaces = line.matches(' ').count();
            let measured: usize = line
                .bytes()
                .map(|byte| {
                    if byte == b' ' {
                        usize::from(PROPORTIONAL_DEFAULT_SPACE_ADVANCE)
                    } else {
                        PROPORTIONAL_WIDTH_TABLE.width_for_byte(byte).unwrap() + 1
                    }
                })
                .sum();
            assert!(spaces > 0);
            let last = placed.last().unwrap();
            let last_width = PROPORTIONAL_WIDTH_TABLE.width_for_byte(last.code).unwrap();
            assert_eq!(usize::from(last.x) + last_width + 1, 32 + measured);
        }
    }

    /// A table where every drawable code has the same width, so the layout
    /// units in these unit tests are easy to reason about.
    fn uniform_width_table(width: u8) -> ProportionalWidthTable {
        let mut widths = [0u8; PROPORTIONAL_WIDTH_TABLE_LEN];
        for code in (PCS_FIRST_CODE + 1)..=b'z' {
            widths[usize::from(code)] = width;
        }
        // Space's own entry is zero, as in the shipped table.
        widths[usize::from(b' ')] = 0;
        ProportionalWidthTable::new(widths)
    }

    #[test]
    fn story_layout_indents_with_the_brace_and_starts_at_the_pen_origin() {
        let widths = uniform_width_table(4);
        let boxed = ProportionalLayoutDescriptor::full_width(0, 20);
        let placed =
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"\n{ab cd\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
        // The leading line feed ends an empty line and is consumed, so the
        // text lands one stride down; `{` then advances a flat 15 and draws
        // nothing.
        assert_eq!(placed[0].y, 20 + PROPORTIONAL_LINE_STRIDE);
        assert_eq!(placed[0].x, PROPORTIONAL_BRACE_INDENT);
        assert_eq!(placed[0].code, b'a');
        // The final line of a paragraph keeps the natural space advance and
        // no plus-one is added to it.
        let space_gap = placed[2].x - placed[1].x;
        assert_eq!(space_gap, 5 + u16::from(PROPORTIONAL_DEFAULT_SPACE_ADVANCE));
    }

    #[test]
    fn story_layout_needs_two_line_feeds_for_a_blank_line() {
        // `text-output.md` section 8.3: exactly one break byte is skipped, so
        // one line feed just ends the line.
        let widths = uniform_width_table(4);
        let boxed = ProportionalLayoutDescriptor::full_width(0, 0);
        let single = layout_proportional_paragraph_glyphs(&widths, &boxed, b"a\nb\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
        assert_eq!(single[1].y, PROPORTIONAL_LINE_STRIDE);
        let double = layout_proportional_paragraph_glyphs(&widths, &boxed, b"a\n\nb\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
        assert_eq!(double[1].y, 2 * PROPORTIONAL_LINE_STRIDE);
    }

    #[test]
    fn story_layout_justifies_every_line_but_the_last_of_a_paragraph() {
        let widths = uniform_width_table(4);
        // Each glyph advances 5 (width 4 plus the inter-glyph gap), so four
        // two-glyph words plus three 5-pixel spaces measure 55. With an
        // available width of 60 the fifth word cannot join, and the first
        // line's 5 pixels of slack spread over its three spaces as 1, 2, 2.
        let boxed = ProportionalLayoutDescriptor {
            right_a: 60,
            right_b: 60,
            ..ProportionalLayoutDescriptor::full_width(0, 0)
        };
        let placed =
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb cc dd ee\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
        let first_line: Vec<_> = placed.iter().filter(|glyph| glyph.y == 0).collect();
        assert_eq!(first_line.len(), 8);
        // Justified: the pen ends exactly on the exclusive right margin, so
        // the last glyph's rightmost ink column is two short of it.
        let last = first_line.last().unwrap();
        assert_eq!(usize::from(last.x) + 4, usize::from(boxed.right_a) - 2 + 1);
        let gaps: Vec<u16> = (0..3)
            .map(|word| first_line[word * 2 + 2].x - first_line[word * 2 + 1].x - 5)
            .collect();
        assert_eq!(
            gaps,
            vec![6, 7, 7],
            "the truncating division's remainder lands on the last spaces"
        );
        // The paragraph's last line is ragged-right with natural spacing.
        let second_line: Vec<_> = placed
            .iter()
            .filter(|glyph| glyph.y == PROPORTIONAL_LINE_STRIDE)
            .collect();
        assert_eq!(second_line.len(), 2);
        assert_eq!(second_line[0].x, 0);
    }

    #[test]
    fn story_layout_breaks_at_a_soft_hyphen_that_fits_and_draws_the_hyphen() {
        let widths = uniform_width_table(4);
        // With a uniform width of 4 every glyph advances 5 and the hyphen
        // costs 5 too, so at an available width of 40 the later soft hyphen
        // fails the `hyphen + accumulated + 1 < available` test and the
        // earlier one is taken.
        let boxed = ProportionalLayoutDescriptor {
            right_a: 40,
            right_b: 40,
            ..ProportionalLayoutDescriptor::full_width(0, 0)
        };
        let placed =
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb_cc_dd\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
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
        assert_eq!(first, "aabb-");
        assert_eq!(second, "ccdd");
        // The marker itself never reaches the framebuffer.
        assert!(
            placed
                .iter()
                .all(|glyph| glyph.code != STORY_SOFT_BREAK_MARKER)
        );
    }

    #[test]
    fn story_layout_emits_an_unbreakable_token_one_byte_per_line() {
        // `text-output.md` section 8.3's degenerate case: no space and no
        // soft hyphen to back up to, so the walk consumes one byte and the
        // line ends rather than looping forever.
        let widths = uniform_width_table(4);
        let boxed = ProportionalLayoutDescriptor {
            right_a: 6,
            right_b: 6,
            ..ProportionalLayoutDescriptor::full_width(0, 0)
        };
        let placed =
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"abcd\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
        assert_eq!(placed.len(), 4);
        // One byte per line while the walk keeps overflowing...
        assert_eq!((placed[0].code, placed[0].x, placed[0].y), (b'a', 0, 0));
        assert_eq!(
            (placed[1].code, placed[1].x, placed[1].y),
            (b'b', 0, PROPORTIONAL_LINE_STRIDE)
        );
        // ...and the final line is the one that reaches NUL, which ends the
        // line cleanly and is never backtracked, so it may overrun the margin.
        assert_eq!(placed[2].y, 2 * PROPORTIONAL_LINE_STRIDE);
        assert_eq!(placed[3].y, 2 * PROPORTIONAL_LINE_STRIDE);
    }

    #[test]
    fn story_layout_treats_a_pen_right_of_the_margin_as_consumed_width() {
        // `text-output.md` section 8.1, and the reason step 18's pen X is 174
        // while its left margin is 148: the pen resets to the margin at the
        // first line break.
        let widths = uniform_width_table(4);
        let boxed = ProportionalLayoutDescriptor {
            pen_x: 20,
            right_a: 45,
            right_b: 45,
            ..ProportionalLayoutDescriptor::full_width(0, 0)
        };
        let placed =
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb cc\0", PROPORTIONAL_DRAW_CLIP_Y).unwrap();
        assert_eq!(placed[0].x, 20, "the first line starts at the pen");
        let wrapped = placed.iter().find(|glyph| glyph.y > 0).unwrap();
        assert_eq!(wrapped.x, 0, "later lines start at the margin");
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
            let boxed = intro_story_paragraph_box(step).unwrap();
            let placed = layout_proportional_paragraph_glyphs(
                &PROPORTIONAL_WIDTH_TABLE,
                &boxed,
                text.as_bytes(),
                PROPORTIONAL_DRAW_CLIP_Y,
            )
            .unwrap_or_else(|err| panic!("intro story step {step} layout: {err}"));

            assert_eq!(placed.len(), glyph_count, "step {step} glyph count");
            assert_eq!(placed[0].y, first_y, "step {step} first line row");
            assert_eq!(placed[0].x, first_x, "step {step} first glyph column");
            let last = placed.last().unwrap();
            assert_eq!(last.y, last_y, "step {step} last line row");
            let last_width = PROPORTIONAL_WIDTH_TABLE.width_for_byte(last.code).unwrap();
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
            // Every glyph must sit inside its line's selected margins. The
            // record's first line may start right of the margin when the pen
            // origin does (step 18).
            for glyph in &placed {
                let (left, right) = boxed.margins_for(glyph.y);
                let left = if glyph.y == first_y {
                    left.max(boxed.pen_x)
                } else {
                    left
                };
                let width = PROPORTIONAL_WIDTH_TABLE.width_for_byte(glyph.code).unwrap();
                assert!(glyph.x >= left, "step {step} glyph left of its margin");
                assert!(
                    usize::from(glyph.x) + width <= usize::from(right),
                    "step {step} glyph past its right margin"
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
            let boxed = match record {
                0 => CHARGEN_GYPSY_PARAGRAPH_BOX,
                1 => CHARGEN_RESULT_PARAGRAPH_BOX,
                _ => CHARGEN_QUESTION_PARAGRAPH_BOX,
            };
            let text = records
                .records
                .get(record)
                .unwrap_or_else(|| panic!("QUESTION.DAT record {record}"));
            let placed = layout_proportional_paragraph_glyphs(
                &PROPORTIONAL_WIDTH_TABLE,
                &boxed,
                text.as_bytes(),
                PROPORTIONAL_DRAW_CLIP_Y,
            )
            .unwrap_or_else(|err| panic!("chargen record {record} layout: {err}"));

            assert_eq!(placed.len(), glyph_count, "record {record} glyph count");
            assert_eq!(placed[0].y, first_y, "record {record} first line row");
            assert_eq!(placed[0].x, first_x, "record {record} first glyph column");
            let last = placed.last().unwrap();
            assert_eq!(last.y, last_y, "record {record} last line row");
            let last_width = PROPORTIONAL_WIDTH_TABLE.width_for_byte(last.code).unwrap();
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



    #[test]
    fn dissolve_visits_every_pixel_of_the_rectangle_exactly_once() {
        // `display-driver-abi.md` section 9.6 bullets 1 and 3.
        for rect in [
            (40u16, 86u16, 75u16, 120u16),
            (0, 0, 319, 100),
            (0, 0, 319, 199),
            (8, 8, 183, 183),
            (5, 7, 5, 7),
        ] {
            let mut dissolve = RectangleDissolve::new(rect).unwrap();
            let count = dissolve.pixel_count() as usize;
            let mut seen = std::collections::HashSet::with_capacity(count);
            while let Some(pixel) = dissolve.next_pixel() {
                assert!(pixel.0 >= rect.0 && pixel.0 <= rect.2, "x inside {rect:?}");
                assert!(pixel.1 >= rect.1 && pixel.1 <= rect.3, "y inside {rect:?}");
                assert!(seen.insert(pixel), "{pixel:?} visited twice in {rect:?}");
            }
            assert_eq!(seen.len(), count, "every pixel of {rect:?} is visited");
            assert!(dissolve.is_complete());

            // Deterministic and reproducible across calls.
            let mut again = RectangleDissolve::new(rect).unwrap();
            let mut first = RectangleDissolve::new(rect).unwrap();
            for _ in 0..count.min(64) {
                assert_eq!(first.next_pixel(), again.next_pixel());
            }
        }
    }

    #[test]
    fn dissolve_order_is_scattered_not_row_or_column_major() {
        // `display-driver-abi.md` section 9.6 bullet 4.
        let rect = (0u16, 0u16, 63u16, 63u16);
        let mut dissolve = RectangleDissolve::new(rect).unwrap();
        let order: Vec<(u16, u16)> = std::iter::from_fn(|| dissolve.next_pixel())
            .take(256)
            .collect();
        let row_major: Vec<(u16, u16)> = (0..256u16).map(|i| (i % 64, i / 64)).collect();
        let column_major: Vec<(u16, u16)> = (0..256u16).map(|i| (i / 64, i % 64)).collect();
        assert_ne!(order, row_major);
        assert_ne!(order, column_major);
        // Consecutive visits jump around rather than stepping by one cell.
        let adjacent = order
            .windows(2)
            .filter(|pair| {
                pair[0].0.abs_diff(pair[1].0) <= 1 && pair[0].1.abs_diff(pair[1].1) <= 1
            })
            .count();
        assert!(
            adjacent * 4 < order.len(),
            "{adjacent} of {} visits were to an adjacent pixel",
            order.len()
        );
    }

    #[test]
    fn dissolve_abort_gate_clears_permanently_on_the_first_glyph() {
        // `display-driver-abi.md` section 9.6: enabled at driver load, cleared
        // permanently the first time a character is drawn through the
        // fixed-cell glyph entry, and never re-enabled.
        let mut gate = DissolveAbortGate::on_driver_load();
        assert!(gate.is_armed());
        assert!(gate.samples_input_after_copy(1), "the first visit polls");
        assert!(!gate.samples_input_after_copy(2));
        assert!(gate.samples_input_after_copy(3));
        gate.note_fixed_cell_glyph_drawn();
        assert!(!gate.is_armed());
        gate.note_fixed_cell_glyph_drawn();
        assert!(!gate.is_armed());
        assert!(!gate.samples_input_after_copy(1), "a cleared gate never polls");
    }

    #[test]
    fn dissolve_rejects_an_inverted_rectangle() {
        assert!(RectangleDissolve::new((10, 10, 9, 20)).is_err());
        assert!(RectangleDissolve::new((10, 10, 20, 9)).is_err());
    }

    #[test]
    fn dissolve_driver_entry_and_caller_side_share_one_visit_order() {
        // One published operation, one order: the dispatch-0x66 driver entry
        // and the caller-side generator must scatter identically, or the same
        // dissolve would look different depending on which path issued it.
        let rect = (40u16, 86u16, 75u16, 120u16);
        let mut caller = RectangleDissolve::new(rect).unwrap();
        let mut driver = crate::display_driver::EgaDissolveState::new(
            crate::display_driver::normalize_clamp_pixel_rect(
                i32::from(rect.0),
                i32::from(rect.1),
                i32::from(rect.2),
                i32::from(rect.3),
            )
            .unwrap(),
        );

        assert_eq!(driver.total_pixels() as u32, caller.pixel_count());
        let mut visited = 0usize;
        while let Some((x, y)) = caller.next_pixel() {
            let from_driver = driver.next_pixel().expect("driver entry visits in step");
            assert_eq!(
                (usize::from(x), usize::from(y)),
                from_driver,
                "visit {visited} differs between the two entries"
            );
            visited += 1;
        }
        assert_eq!(visited, caller.pixel_count() as usize);
        assert!(driver.next_pixel().is_none());
        assert!(driver.is_finished() && caller.is_complete());
    }

    #[test]
    fn story_layout_clips_drawing_at_pen_row_192_without_changing_layout() {
        // `text-output.md` section 8.5: once the pen row reaches 192 glyphs
        // stop being drawn, but the pen still advances exactly as if they
        // were, so the walk through the text is identical.
        let widths = uniform_width_table(4);
        let boxed = ProportionalLayoutDescriptor::full_width(0, 180);
        let text = b"aa\naa\naa\naa\naa\0";

        let clipped =
            layout_proportional_paragraph_glyphs(&widths, &boxed, text, PROPORTIONAL_DRAW_CLIP_Y)
                .unwrap();
        // Lines land at 180, 189, 198, 207, 216; only the first two are drawn.
        assert_eq!(clipped.len(), 4);
        assert!(clipped.iter().all(|glyph| glyph.y < PROPORTIONAL_DRAW_CLIP_Y));
        assert_eq!(clipped[0].y, 180);
        assert_eq!(clipped[2].y, 189);

        // Raising the clip proves the layout itself never changed: the same
        // walk simply keeps drawing.
        let unclipped =
            layout_proportional_paragraph_glyphs(&widths, &boxed, text, u16::MAX).unwrap();
        assert_eq!(unclipped.len(), 10);
        assert_eq!(&unclipped[..4], &clipped[..]);
        assert_eq!(unclipped[9].y, 180 + 4 * PROPORTIONAL_LINE_STRIDE);
    }

    #[test]
    fn story_records_preserve_the_published_markup_counts() {
        // `cleak/u5-spec#70` publishes counts taken from the shipped file: 654
        // author-placed soft hyphens and 36 paragraph-indent braces across the
        // twenty story records. The renderer never hyphenates on its own, so a
        // loader that strips either marker cannot reproduce the original's
        // line breaks - these counts catch that regression directly.
        let Some(dir) = local_clean_assets() else {
            return;
        };
        let Some(records) = load_story_records(&dir).expect("STORY.DAT loads") else {
            return;
        };
        let soft_breaks: usize = records
            .iter()
            .map(|record| {
                record
                    .bytes()
                    .filter(|byte| *byte == STORY_SOFT_BREAK_MARKER)
                    .count()
            })
            .sum();
        let braces: usize = records
            .iter()
            .map(|record| {
                record
                    .bytes()
                    .filter(|byte| *byte == STORY_PARAGRAPH_START_MARKER)
                    .count()
            })
            .sum();
        assert_eq!(soft_breaks, 654, "author-placed soft hyphens");
        assert_eq!(braces, 36, "paragraph-indent braces");
    }

    #[test]
    fn shipped_tlk_headers_have_no_sentinel_row() {
        // The header is a two-byte count then exactly that many four-byte
        // (npc id, blob offset) rows. Decoding the shipped files shows ids
        // running exactly 1..=count with row one's offset equal to the header
        // length, so there is no sentinel and id 1 addresses its own blob.
        // The parser previously skipped row zero as a sentinel and paired each
        // row's offset with the next row's id, shifting every NPC's dialogue.
        let Some(dir) = local_clean_assets() else {
            return;
        };
        for file in [
            CASTLE_TLK_FILENAME,
            TOWNE_TLK_FILENAME,
            DWELLING_TLK_FILENAME,
            KEEP_TLK_FILENAME,
        ] {
            let path = dir.join(file);
            if !path.is_file() {
                continue;
            }
            let bytes = std::fs::read(&path).expect("shipped TLK reads");
            let count = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
            assert!(count > 0, "{file} declares no entries");
            let dialogue = parse_tlk_bytes(&bytes)
                .unwrap_or_else(|err| panic!("{file} parses with the shipped header shape: {err}"));
            assert_eq!(dialogue.len(), count, "{file} yields one blob per header row");
            let mut ids: Vec<u16> = dialogue.keys().copied().collect();
            ids.sort_unstable();
            assert_eq!(
                ids,
                (1..=count as u16).collect::<Vec<_>>(),
                "{file} ids must be exactly 1..=count"
            );
            // Row one's offset is the first byte past the header.
            assert_eq!(
                u16::from_le_bytes([bytes[4], bytes[5]]) as usize,
                2 + count * 4,
                "{file} first blob starts at the header end"
            );
        }
    }

    #[test]
    fn klimb_offers_the_whole_pit_family_as_an_ordinary_descent() {
        // dungeon-mode.md §13.1: the pit family `0x6?` is a non-ladder
        // K-Klimb *descent* case. It enables only the down arm, and that arm
        // runs the same level-step helper a down ladder does, so it reaches
        // the §13.2 exit contract only from the deepest level. The earlier
        // claim that exact `0x60` bypassed the level step and invoked the
        // surface-reset helper directly is withdrawn.
        for tile in 0x60u8..=0x6F {
            assert_eq!(
                dungeon_ladder_delta(tile, ClimbIntent::Down),
                Some(1),
                "pit byte {tile:#04x} steps one level down"
            );
            assert_eq!(
                dungeon_ladder_delta(tile, ClimbIntent::Up),
                None,
                "pit byte {tile:#04x} offers no up arm"
            );
        }
        // The ladder arms are unchanged by the pit correction.
        assert_eq!(dungeon_ladder_delta(0x1F, ClimbIntent::Up), Some(-1));
        assert_eq!(dungeon_ladder_delta(0x2F, ClimbIntent::Down), Some(1));
        assert_eq!(dungeon_ladder_delta(0x3F, ClimbIntent::Up), Some(-1));
        assert_eq!(dungeon_ladder_delta(0x3F, ClimbIntent::Down), Some(1));
    }

    #[test]
    fn level_change_spell_destination_test_is_not_shared_with_klimb() {
        // dungeon-mode.md §13.1: the Up/Down spells refuse a destination in
        // the base `0x0` class or the wall and door-presentation families
        // `0xB?` through `0xE?`. A climb never inspects the cell it lands on,
        // so this predicate has no caller on the K-Klimb path.
        for tile in [0x00u8, 0x0F, 0xB0, 0xC7, 0xD0, 0xEF] {
            assert!(
                !dungeon_level_change_spell_destination_allowed(tile),
                "{tile:#04x} is refused by the level-change spells"
            );
        }
        for tile in [0x10u8, 0x20, 0x30, 0x60, 0x8F, 0x90, 0xA0, 0xF0] {
            assert!(
                dungeon_level_change_spell_destination_allowed(tile),
                "{tile:#04x} is accepted by the level-change spells"
            );
        }
    }

    #[test]
    fn shipped_dungeon_data_puts_plain_pits_above_the_bottom_level() {
        // The withdrawal of the surface-reset pit claim is checkable against
        // the shipped `DUNGEON.DAT` rather than against the sentence that
        // announced it: the spec cites Deceit level zero at (1, 3) and
        // Destard level zero at (7, 3) and (1, 7). If `0x60` ejected the
        // party to Britannia, those three cells would make the top level of
        // two dungeons unleavable downward by pit; they are ordinary
        // descents to level one.
        let Some(dir) = local_clean_assets() else {
            return;
        };
        let path = dir.join(DUNGEON_DAT_FILENAME);
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).expect("shipped DUNGEON.DAT reads");
        assert_eq!(bytes.len(), DUNGEON_DAT_LEN, "shipped DUNGEON.DAT length");
        let level_zero_pits = |record: usize| -> Vec<(usize, usize)> {
            let base = record * DUNGEON_RECORD_LEN;
            (0..DUNGEON_SIDE)
                .flat_map(|y| (0..DUNGEON_SIDE).map(move |x| (x, y)))
                .filter(|(x, y)| bytes[base + y * DUNGEON_SIDE + x] == 0x60)
                .collect()
        };
        // Record order is the published dungeon order; Deceit is record 0 and
        // Destard record 2.
        assert_eq!(level_zero_pits(0), vec![(1, 3)], "Deceit level zero pits");
        assert_eq!(
            level_zero_pits(2),
            vec![(7, 3), (1, 7)],
            "Destard level zero pits"
        );
    }

    /// `catalogs/spell-list.md §5` `Allowed` column, transcribed verbatim
    /// for spell ids `0..=47` in table order. `magic.md §9` states these
    /// `C`/`D`/`I`/`O` labels "were published correctly throughout" and
    /// that only the numeric bit legend needed correcting, so the labels
    /// are the authority for the per-spell mask table.
    const PUBLISHED_SPELL_SCENE_LABELS: [&str; SPELL_COUNT] = [
        "D/I/O",   // 0  IL   In Lor            Light
        "C",       // 1  GP   Grav Por          Magic Missile
        "C/D/I/O", // 2  AZ   An Zu             Awaken
        "C/D/I/O", // 3  AN   An Nox            Cure
        "C/D/I/O", // 4  M    Mani              Heal
        "C/I",     // 5  AY   An Ylem           Vanish
        "C/D/I/O", // 6  AS   An Sanct          Open
        "C",       // 7  ACX  An Xen Corp       Repel Undead
        "O",       // 8  HR   Rel Hur           Wind Change
        "O",       // 9  IW   In Wis            Locate
        "C",       // 10 KX   Kal Xen           Conjure
        "C/D/I/O", // 11 IMX  In Xen Mani       Create Food
        "D/I/O",   // 12 LV   Vas Lor           Great Light
        "C",       // 13 FV   Vas Flam          Fireball
        "C/D",     // 14 FGI  In Flam Grav      Fire Field
        "C/D",     // 15 GIN  In Nox Grav       Poison Field
        "C/D",     // 16 GIZ  In Zu Grav        Sleep Field
        "C/O",     // 17 IP   In Por            Blink
        "C/D",     // 18 AG   An Grav           Dispel Field
        "C/D/I/O", // 19 IS   In Sanct          Protection
        "C/D",     // 20 GIS  In Sanct Grav     Energy Field
        "D",       // 21 PU   Uus Por           Up
        "D",       // 22 DP   Des Por           Down
        "C",       // 23 QW   Wis Quas          Reveal
        "C",       // 24 BIX  In Bet Xen        Swarm
        "C/I",     // 25 AEP  An Ex Por         Magic Lock
        "C/I",     // 26 EIP  In Ex Por         Unlock Magic
        "C/D/I/O", // 27 MV   Vas Mani          Great Heal
        "C",       // 28 IZ   In Zu             Sleep
        "C/D/I/O", // 29 RT   Rel Tym           Quickness
        "C",       // 30 IPVY In Vas Por Ylem   Tremor
        "C",       // 31 AQW  Quas An Wis       Mass Charm
        "C/D/I/O", // 32 AI   In An             Negate Magic
        "I/O",     // 33 AWY  Wis An Ylem       X-Ray
        "C",       // 34 AEX  An Xen Ex         Charm
        "C",       // 35 BRX  Rel Xen Bet       Polymorph
        "C",       // 36 LS   Sanct Lor         Invisibility
        "C",       // 37 CX   Xen Corp          Kill
        "C",       // 38 IQX  In Quas Xen       Clone
        "D/I/O",   // 39 IQW  In Quas Wis       Peer
        "C",       // 40 HIN  In Nox Hur        Poison Wind
        "C",       // 41 CIQ  In Quas Corp      Cause Fear
        "D/I/O",   // 42 CIM  In Mani Corp      Resurrect
        "C",       // 43 CKX  Kal Xen Corp      Summon
        "C",       // 44 CGIV In Vas Grav Corp  Death Wind
        "C",       // 45 FHI  In Flam Hur       Flame Wind
        "D/I/O",   // 46 PRV  Vas Rel Por       Gate Travel
        "C/D/I/O", // 47 AT   An Tym            Negate Time
    ];

    fn published_scene_mask(labels: &str) -> u8 {
        labels.split('/').fold(0u8, |mask, label| {
            mask | match label {
                "C" => SPELL_SCENE_BIT_COMBAT,
                "D" => SPELL_SCENE_BIT_DUNGEON,
                "I" => SPELL_SCENE_BIT_INDOOR,
                "O" => SPELL_SCENE_BIT_OVERWORLD,
                other => panic!("unpublished scene label {other:?}"),
            }
        })
    }

    #[test]
    fn spell_scene_masks_match_published_catalog_labels_for_every_spell() {
        for (index, labels) in PUBLISHED_SPELL_SCENE_LABELS.iter().enumerate() {
            assert!(
                !labels.is_empty(),
                "catalogs/spell-list.md §5 row {index} has no Allowed labels; refusing to \
                 invent a mask",
            );
            assert_eq!(
                SPELL_SCENE_MASKS[index],
                published_scene_mask(labels),
                "spell {index} ({}) mask disagrees with catalogs/spell-list.md §5 Allowed \
                 column {labels}",
                SPELL_CODES[index],
            );
        }

        // `magic.md §9`'s stated confirmation of the corrected legend: the
        // two dungeon-only level-change spells carry `0x02` alone, and the
        // named combat-only attack spells carry `0x01` alone.
        assert_eq!(SPELL_SCENE_MASKS[UUS_POR_SPELL_INDEX], 0x02);
        assert_eq!(SPELL_SCENE_MASKS[DES_POR_SPELL_INDEX], 0x02);
        assert_eq!(SPELL_SCENE_MASKS[MAGIC_MISSILE_SPELL_INDEX], 0x01);
        assert_eq!(SPELL_SCENE_MASKS[REPEL_UNDEAD_SPELL_INDEX], 0x01);
        assert_eq!(SPELL_SCENE_MASKS[KILL_SPELL_INDEX], 0x01);
    }

    #[test]
    fn spell_scene_class_derivation_covers_every_play_scene() {
        // `catalogs/spell-list.md §4` classification bands.
        assert_eq!(
            spell_scene_class_for_scene_byte(SCENE_OVERWORLD),
            SpellSceneClass::Overworld
        );
        assert_eq!(
            spell_scene_class_for_scene_byte(SCENE_TOWN_FAMILY_FIRST),
            SpellSceneClass::Indoor
        );
        assert_eq!(
            spell_scene_class_for_scene_byte(SCENE_TOWN_FAMILY_LAST),
            SpellSceneClass::Indoor
        );
        assert_eq!(
            spell_scene_class_for_scene_byte(SCENE_DUNGEON_FAMILY_FIRST),
            SpellSceneClass::Dungeon
        );
        assert_eq!(
            spell_scene_class_for_scene_byte(SCENE_DUNGEON_FAMILY_LAST),
            SpellSceneClass::Dungeon
        );
        assert_eq!(
            spell_scene_class_for_scene_byte(SCENE_COMBAT_TEMPORARY),
            SpellSceneClass::Combat
        );

        // Live scenes. Both world planes report the single published
        // overworld byte; the catalog does not split Britannia from the
        // Underworld.
        let overworld = britannia_state(open_world_grid(), 1, 1);
        assert_eq!(overworld.current_scene_byte(), SCENE_OVERWORLD);
        assert_eq!(
            overworld.current_spell_scene_class(),
            SpellSceneClass::Overworld
        );

        let underworld = world_state(open_world_grid(), 1, 1);
        assert!(matches!(
            underworld.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        ));
        assert_eq!(underworld.current_scene_byte(), SCENE_OVERWORLD);
        assert_eq!(
            underworld.current_spell_scene_class(),
            SpellSceneClass::Overworld
        );

        let town = test_state(open_grid(), 1, 1);
        assert_eq!(town.current_spell_scene_class(), SpellSceneClass::Indoor);

        let dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert_eq!(dungeon.current_spell_scene_class(), SpellSceneClass::Dungeon);

        // `PlayState` stores no combat scene byte — combat is the
        // `combat_active` flag, and the flag maps back to the published
        // `0xFF` combat-class byte regardless of the map underneath.
        for mut fighting in [
            britannia_state(open_world_grid(), 1, 1),
            test_state(open_grid(), 1, 1),
            dungeon_state(open_dungeon_record(), 0, 1, 1),
        ] {
            fighting.combat_active = true;
            assert_eq!(fighting.current_scene_byte(), SCENE_COMBAT_TEMPORARY);
            assert_eq!(
                fighting.current_spell_scene_class(),
                SpellSceneClass::Combat
            );
        }
    }

    #[test]
    fn cast_context_accepts_exactly_the_published_scenes_for_every_spell() {
        let scenes: [(SpellSceneClass, PlayState); 4] = [
            (
                SpellSceneClass::Overworld,
                britannia_state(open_world_grid(), 1, 1),
            ),
            (SpellSceneClass::Indoor, test_state(open_grid(), 1, 1)),
            (
                SpellSceneClass::Dungeon,
                dungeon_state(open_dungeon_record(), 0, 1, 1),
            ),
            (SpellSceneClass::Combat, {
                let mut fighting = britannia_state(open_world_grid(), 1, 1);
                fighting.combat_active = true;
                fighting
            }),
        ];

        for (class, state) in &scenes {
            assert_eq!(state.current_spell_scene_class(), *class);
            for index in 0..SPELL_COUNT {
                let expected = published_scene_mask(PUBLISHED_SPELL_SCENE_LABELS[index])
                    & class.allow_mask_bit()
                    != 0;
                assert_eq!(
                    state.spell_allowed_in_current_cast_context(index),
                    expected,
                    "spell {index} ({}) in {class:?}: catalogs/spell-list.md §5 publishes {}",
                    SPELL_CODES[index],
                    PUBLISHED_SPELL_SCENE_LABELS[index],
                );
            }
        }

        // Out-of-range ids never pass the gate.
        assert!(
            !scenes[0]
                .1
                .spell_allowed_in_current_cast_context(SPELL_COUNT)
        );
    }

    #[test]
    fn cast_scene_gate_refuses_dungeon_only_spell_on_the_overworld() {
        // `catalogs/spell-list.md §5` ids 21/22 publish Up and Down as `D`.
        for spell_index in [UUS_POR_SPELL_INDEX, DES_POR_SPELL_INDEX] {
            let mut state = britannia_state(open_world_grid(), 1, 1);
            state.spell_charges[spell_index] = 3;
            state.party[0].mana = 20;
            state.party[0].level = 8;

            let outcome =
                state.cast_spell_resource_gate(0, spell_index, DUNGEON_LEVEL_SPELL_COST);

            assert_eq!(outcome, Some(MoveOutcome::Blocked));
            assert_eq!(state.message, "Not here!");
            // `magic.md §7`: the scene gate runs before charge consumption.
            assert_eq!(state.spell_charges[spell_index], 3);
            assert_eq!(state.party[0].mana, 20);
            // `magic.md §5` step 3: no time is consumed on this rejection.
            assert_eq!(state.turn, 0);
        }

        // The same spell in a dungeon passes the scene gate and spends.
        let mut dungeon = dungeon_state(open_dungeon_record(), 1, 1, 1);
        dungeon.spell_charges[UUS_POR_SPELL_INDEX] = 3;
        dungeon.party[0].mana = 20;
        dungeon.party[0].level = 8;
        assert_eq!(
            dungeon.cast_spell_resource_gate(0, UUS_POR_SPELL_INDEX, DUNGEON_LEVEL_SPELL_COST),
            None
        );
        assert_eq!(dungeon.spell_charges[UUS_POR_SPELL_INDEX], 2);
        assert_eq!(dungeon.party[0].mana, 20 - DUNGEON_LEVEL_SPELL_COST);
    }

    #[test]
    fn cast_scene_gate_refuses_combat_only_spell_outside_combat() {
        // `catalogs/spell-list.md §5` id 1 publishes Magic Missile as `C`.
        for mut state in [
            britannia_state(open_world_grid(), 1, 1),
            test_state(open_grid(), 1, 1),
            dungeon_state(open_dungeon_record(), 0, 1, 1),
        ] {
            state.spell_charges[MAGIC_MISSILE_SPELL_INDEX] = 2;
            state.party[0].mana = 9;
            state.party[0].level = 8;

            let outcome = state.cast_spell_resource_gate(0, MAGIC_MISSILE_SPELL_INDEX, 1);

            assert_eq!(outcome, Some(MoveOutcome::Blocked));
            assert_eq!(state.message, "Not here!");
            assert_eq!(state.spell_charges[MAGIC_MISSILE_SPELL_INDEX], 2);
            assert_eq!(state.party[0].mana, 9);
            assert_eq!(state.turn, 0);
        }

        let mut fighting = britannia_state(open_world_grid(), 1, 1);
        fighting.combat_active = true;
        fighting.spell_charges[MAGIC_MISSILE_SPELL_INDEX] = 2;
        fighting.party[0].mana = 9;
        fighting.party[0].level = 8;
        assert_eq!(
            fighting.cast_spell_resource_gate(0, MAGIC_MISSILE_SPELL_INDEX, 1),
            None
        );
        assert_eq!(fighting.spell_charges[MAGIC_MISSILE_SPELL_INDEX], 1);
        assert_eq!(fighting.party[0].mana, 8);
    }

    #[test]
    fn cast_scene_rejection_precedes_the_charges_gate() {
        // `magic.md §7`: gate order is scene, then charges. A dungeon-only
        // spell cast on the overworld with zero charges still reports the
        // scene refusal, not `None mixed!`.
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.spell_charges[UUS_POR_SPELL_INDEX] = 0;
        state.party[0].mana = 20;
        state.party[0].level = 8;

        assert_eq!(
            state.cast_spell_resource_gate(0, UUS_POR_SPELL_INDEX, DUNGEON_LEVEL_SPELL_COST),
            Some(MoveOutcome::Blocked)
        );
        assert_eq!(state.message, "Not here!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn cast_level_gate_compares_caster_level_against_the_spell_circle() {
        // `catalogs/spell-list.md §1`: `mana_cost(id) = circle(id)` and
        // `minimum_level(id) = circle(id)`. Great Heal (id 27) is circle 5
        // and is published `C/D/I/O`, so an indoor cast clears the scene
        // gate and exercises the mana and level gates alone.
        let circle = spell_circle_for(GREAT_HEAL_SPELL_INDEX as u8).unwrap();
        assert_eq!(circle, 5);
        assert_eq!(spell_mana_cost(circle), circle);
        assert_eq!(spell_min_caster_level(circle), circle);

        // Level below the circle: `magic.md §7` gate 8 — charge and mana
        // are both spent, and the turn advances.
        let mut under_level = test_state(open_grid(), 1, 1);
        under_level.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
        under_level.party[0].mana = 9;
        under_level.party[0].level = circle - 1;
        assert_eq!(
            under_level.cast_spell_resource_gate(0, GREAT_HEAL_SPELL_INDEX, GREAT_HEAL_COST),
            Some(MoveOutcome::Blocked)
        );
        assert_eq!(under_level.message, "M.P. too low!");
        assert_eq!(under_level.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(under_level.party[0].mana, 9 - circle);
        assert_eq!(under_level.turn, 1);

        // Level exactly at the circle passes, even though the caster's
        // level is far below their mana pool.
        let mut at_level = test_state(open_grid(), 1, 1);
        at_level.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
        at_level.party[0].mana = 40;
        at_level.party[0].level = circle;
        assert_eq!(
            at_level.cast_spell_resource_gate(0, GREAT_HEAL_SPELL_INDEX, GREAT_HEAL_COST),
            None
        );
        assert_eq!(at_level.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(at_level.party[0].mana, 40 - circle);

        // Mana below the circle: `magic.md §7` gate 7 — the charge is gone
        // but no mana is debited.
        let mut under_mana = test_state(open_grid(), 1, 1);
        under_mana.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
        under_mana.party[0].mana = circle - 1;
        under_mana.party[0].level = 8;
        assert_eq!(
            under_mana.cast_spell_resource_gate(0, GREAT_HEAL_SPELL_INDEX, GREAT_HEAL_COST),
            Some(MoveOutcome::Blocked)
        );
        assert_eq!(under_mana.message, "M.P. too low!");
        assert_eq!(under_mana.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(under_mana.party[0].mana, circle - 1);
        assert_eq!(under_mana.turn, 1);
    }

    #[test]
    fn every_published_spell_cost_equals_its_circle() {
        // `catalogs/spell-list.md §1`: `circle(id) = (id / 6) + 1`, and both
        // the mana cost and the minimum caster level equal that circle. The
        // live gate re-derives the circle from the spell id, so this is the
        // invariant that lets callers keep passing a mana cost.
        for index in 0..SPELL_COUNT {
            let circle = spell_circle_for(index as u8).unwrap();
            assert_eq!(circle, (index / SPELLS_PER_CIRCLE) as u8 + 1);
            assert_eq!(spell_mana_cost(circle), spell_min_caster_level(circle));
        }
        assert_eq!(spell_circle_for(SPELL_COUNT as u8), None);
    }
