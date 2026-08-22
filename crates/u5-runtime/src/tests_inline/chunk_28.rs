
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
                layout_proportional_paragraph_glyphs(&PROPORTIONAL_WIDTH_TABLE, boxed, &text, 200)
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
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"
{ab cd ", 200).unwrap();
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
        let single = layout_proportional_paragraph_glyphs(&widths, &boxed, b"a
b ", 200).unwrap();
        assert_eq!(single[1].y, PROPORTIONAL_LINE_STRIDE);
        let double = layout_proportional_paragraph_glyphs(&widths, &boxed, b"a

b ", 200).unwrap();
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
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb cc dd ee ", 200).unwrap();
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
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb_cc_dd ", 200).unwrap();
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
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"abcd ", 200).unwrap();
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
            layout_proportional_paragraph_glyphs(&widths, &boxed, b"aa bb cc ", 200).unwrap();
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
                200,
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
                200,
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
        assert!(gate.samples_input_at(0), "every second visited pixel");
        assert!(!gate.samples_input_at(1));
        gate.note_fixed_cell_glyph_drawn();
        assert!(!gate.is_armed());
        gate.note_fixed_cell_glyph_drawn();
        assert!(!gate.is_armed());
        assert!(!gate.samples_input_at(0), "a cleared gate never polls");
    }

    #[test]
    fn dissolve_rejects_an_inverted_rectangle() {
        assert!(RectangleDissolve::new((10, 10, 9, 20)).is_err());
        assert!(RectangleDissolve::new((10, 10, 20, 9)).is_err());
    }
