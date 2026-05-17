    #[test]
    fn tile_image_directory_header_widths_match_spec() {
        // formats/tiles.md §5.2: the image-directory layout opens
        // with a two-byte count word, an array of four-byte
        // doubleword offsets, and per-image blocks each opening with
        // a width word and a height word (four bytes total).
        // Promote the three widths so parse_graphic_image_directory_body
        // does not bake `2` and `4` as bare literals.
        assert_eq!(TILE_IMAGE_DIRECTORY_COUNT_BYTES, 2);
        assert_eq!(TILE_IMAGE_DIRECTORY_OFFSET_BYTES, 4);
        assert_eq!(TILE_IMAGE_BLOCK_HEADER_BYTES, 4);
    }

    #[test]
    fn miniature_tile_glyph_record_layout_matches_spec() {
        // formats/tiles.md §5.1.1: each resident miniature tile glyph
        // record describes sixteen rows with two offset bytes per row,
        // for thirty-two bytes per tile. Promote the row count, the
        // bytes-per-row, and the derived record length so a future
        // miniature-glyph decoder has named constants instead of bare
        // literals.
        assert_eq!(MINIATURE_TILE_ROWS, 16);
        assert_eq!(MINIATURE_TILE_OFFSET_BYTES_PER_ROW, 2);
        assert_eq!(MINIATURE_TILE_RECORD_BYTES, 32);
        assert_eq!(
            MINIATURE_TILE_RECORD_BYTES,
            MINIATURE_TILE_ROWS * MINIATURE_TILE_OFFSET_BYTES_PER_ROW
        );
    }

    #[test]
    fn lzw_envelope_length_header_is_four_bytes() {
        // formats/lzw.md §2: the LZW envelope opens with a four-byte
        // little-endian unsigned length followed by the code stream.
        // Promote the header width so decode_lzw_envelope does not
        // encode `4` as a bare literal in two places.
        assert_eq!(LZW_ENVELOPE_LENGTH_HEADER_BYTES, 4);
        assert_eq!(
            LZW_ENVELOPE_LENGTH_HEADER_BYTES,
            std::mem::size_of::<u32>(),
            "the header width must match the u32 length the spec names"
        );
    }

    #[test]
    fn shop_surcharge_band_constants_match_spec() {
        // shops.md §6.2: the post-transaction surcharge subtracts a
        // random `1..64` gold from the party's gold word on a hit;
        // it runs only when the shared town/conversation sentinel is
        // zero. Promote the band edges, the roll mask, and the
        // sentinel value so shop_surcharge_from_roll_seed and
        // apply_shop_surcharge do not bake `1`, `64`, `0x3F`, and
        // the `== 0` enable check as bare literals.
        assert_eq!(SHOP_SURCHARGE_GOLD_MIN, 1);
        assert_eq!(SHOP_SURCHARGE_GOLD_MAX, 64);
        assert_eq!(SHOP_SURCHARGE_ROLL_MASK, 0x3F);
        assert_eq!(SHOP_SURCHARGE_SENTINEL_ENABLES, 0);
        // Every roll seed lands inside the published band.
        for seed in 0u8..=u8::MAX {
            let charge = shop_surcharge_from_roll_seed(seed);
            assert!(
                (SHOP_SURCHARGE_GOLD_MIN..=SHOP_SURCHARGE_GOLD_MAX).contains(&charge),
                "seed {seed} produced {charge}"
            );
        }
    }

    #[test]
    fn arms_shop_pricing_formula_constants_match_spec() {
        // shops.md §6: the arms-shop Buy quote is
        // `base + base * (100 - 3 * intelligence) / 100`, and the
        // Sell offer is `floor(base * 3 * intelligence / 100) + 1`.
        // Promote the shared `100` percent denominator and `3` Int
        // weight, plus the `+1` Sell minimum bias.
        assert_eq!(ARMS_SHOP_PERCENT_DENOMINATOR, 100);
        assert_eq!(ARMS_SHOP_INTELLIGENCE_WEIGHT, 3);
        assert_eq!(ARMS_SHOP_SELL_MIN_OFFER_BIAS, 1);
        // Spot-check buy quote at a few intelligence values.
        // intelligence 0: base + base * 100/100 = 2 * base.
        assert_eq!(arms_shop_buy_quote(100, 0), 200);
        // intelligence ~33.33 (33): factor = 100 - 99 = 1,
        // quote = 100 + 100 * 1 / 100 = 101.
        assert_eq!(arms_shop_buy_quote(100, 33), 101);
        // intelligence 99: factor = 100 - 297 = -197,
        // adjustment = 100 * -197 / 100 = -197,
        // quote = 100 + -197 = -97 → clamps to 0.
        assert_eq!(arms_shop_buy_quote(100, 99), 0);
        // Sell offer floors at 1.
        assert_eq!(arms_shop_sell_offer(100, 0), 1);
        // base=100, int=10 → 100*3*10/100 + 1 = 30 + 1 = 31.
        assert_eq!(arms_shop_sell_offer(100, 10), 31);
    }

    #[test]
    fn shop_placeholder_byte_constants_match_spec() {
        // shops.md §4.1: the SHOPPE.DAT bark renderer reserves seven
        // printable-ASCII bytes as substitution placeholders. None of
        // the shipped record text uses these as literal punctuation,
        // so the renderer always expands them. Promote each so a
        // future renderer does not match `%`/`^`/`$`/`&`/`*`/`#`/`@`
        // as bare literals.
        assert_eq!(SHOP_PLACEHOLDER_GOLD, b'%');
        assert_eq!(SHOP_PLACEHOLDER_QUANTITY, b'^');
        assert_eq!(SHOP_PLACEHOLDER_VENDOR_NAME, b'$');
        assert_eq!(SHOP_PLACEHOLDER_ITEM_NAME, b'&');
        assert_eq!(SHOP_PLACEHOLDER_PLACE_NAME, b'*');
        assert_eq!(SHOP_PLACEHOLDER_SHOP_NAME, b'#');
        assert_eq!(SHOP_PLACEHOLDER_TIME_OF_DAY, b'@');
        // Each named code resolves to the matching placeholder kind.
        assert_eq!(
            shop_placeholder_kind(SHOP_PLACEHOLDER_GOLD),
            Some(ShopPlaceholderKind::Gold)
        );
        assert_eq!(
            shop_placeholder_kind(SHOP_PLACEHOLDER_TIME_OF_DAY),
            Some(ShopPlaceholderKind::TimeOfDay)
        );
        // Ordinary ASCII letters are not placeholders.
        assert_eq!(shop_placeholder_kind(b'A'), None);
        assert_eq!(shop_placeholder_kind(b' '), None);
        assert_eq!(shop_placeholder_kind(0), None);
    }

    #[test]
    fn britannia_chunk_map_renderer_dimensions_match_spec() {
        // view.md §4: the full chunk-map view paints an eight-row by
        // twenty-two-column shorthand map of Britannia chunks,
        // wrapping the chunk walk at the world edges and marking the
        // party's current chunk with a crosshair. The traced LOOKOBJ
        // path that enters this renderer from ordinary Look is keyed
        // by tile id 0x59. Promote the dimensions and trigger tile so
        // future implementations have named constants instead of bare
        // literals.
        assert_eq!(BRITANNIA_CHUNK_MAP_ROWS, 8);
        assert_eq!(BRITANNIA_CHUNK_MAP_COLUMNS, 22);
        assert_eq!(BRITANNIA_CHUNK_MAP_LOOK_TRIGGER_TILE, 0x59);
        // Sanity-check that 8x22 fits inside the message-panel cell
        // dimensions used by other view-system rectangles (well under
        // the 40-column screen width and 25-row height defined by
        // TEXT_SCREEN_COLUMNS/_ROWS).
        assert!(BRITANNIA_CHUNK_MAP_COLUMNS < TEXT_SCREEN_COLUMNS);
        assert!(BRITANNIA_CHUNK_MAP_ROWS < TEXT_SCREEN_ROWS);
    }

    #[test]
    fn monster_reward_unit_derivation_constants_match_spec() {
        // catalogs/monster-bestiary.md §1: the per-class reward unit
        // returned by the damage/death handler is
        // `floor(class_max_hp / 4) + 1`. Promote the divisor and
        // bias so CombatClassStats::reward_unit does not bake `4`
        // and `+ 1` as bare literals.
        assert_eq!(MONSTER_REWARD_UNIT_HP_DIVISOR, 4);
        assert_eq!(MONSTER_REWARD_UNIT_BIAS, 1);
        // Spot-check a few HP totals.
        let stats = |hp: u8| CombatClassStats {
            class: 0,
            name: "test",
            tier: 0,
            speed_seed: 0,
            hp_comparison: 0,
            defense: 0,
            attack_cap: 0,
            max_hp: hp,
            default_spawn_count: 0,
            default_drop_cap: 0,
        };
        // A 0-HP class still credits one unit (the +1 bias).
        assert_eq!(stats(0).reward_unit(), MONSTER_REWARD_UNIT_BIAS);
        // 4 HP -> 4/4 + 1 = 2; 100 HP -> 100/4 + 1 = 26.
        assert_eq!(stats(4).reward_unit(), 2);
        assert_eq!(stats(100).reward_unit(), 26);
        // 255 HP saturates at 255/4 + 1 = 64.
        assert_eq!(stats(255).reward_unit(), 64);
    }

    #[test]
    fn directed_target_walk_max_cells_matches_spec() {
        // magic.md §8: the directed target-walk family (In Zu, In Nox
        // Hur, In Vas Grav Corp, In Flam Hur) builds a directed set
        // of up to twenty-one arena cells from the active target/
        // caster state before scanning combat actor slots for
        // matches. Promote the cap so future target-walk
        // implementations do not bake `21` as a bare literal.
        assert_eq!(DIRECTED_TARGET_WALK_MAX_CELLS, 21);
        // Sanity-check: a 21-cell directed area cannot exceed the
        // 121-cell arena footprint.
        assert!(DIRECTED_TARGET_WALK_MAX_CELLS <= COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE);
    }

    #[test]
    fn quest_password_constants_match_spec_strings() {
        // catalogs/quest-graph.md §3 names two passwords as graph
        // gates: `DAWN` for Resistance trust and `IMPERA` for the
        // Blackthorn-side oppression branches. Promote both so
        // future conversation/data-authoring tooling can reference
        // them as named constants instead of bare string literals.
        assert_eq!(QUEST_PASSWORD_RESISTANCE, "DAWN");
        assert_eq!(QUEST_PASSWORD_OPPRESSION, "IMPERA");
        // The passwords are case-meaningful uppercase identifiers
        // distinct from each other; they should not accidentally
        // alias to the same string after any future normalisation.
        assert_ne!(QUEST_PASSWORD_RESISTANCE, QUEST_PASSWORD_OPPRESSION);
        assert!(QUEST_PASSWORD_RESISTANCE.chars().all(|c| c.is_ascii_uppercase()));
        assert!(QUEST_PASSWORD_OPPRESSION.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn dungeon_pit_fall_and_bomb_trap_byte_constants_match_spec() {
        // doors-and-z-transitions.md §10 names two fall-trap bytes
        // (0x61 visible, 0x69 hidden) that increment Z when stepped
        // on, and two bomb-trap bytes (0x62 visible, 0x6A hidden)
        // that share the 0x6? pit family but do not change Z.
        // Promote each so the trap classifiers do not bake the hex
        // pairs as bare literals.
        assert_eq!(DUNGEON_PIT_FALL_TRAP_VISIBLE, 0x61);
        assert_eq!(DUNGEON_PIT_FALL_TRAP_HIDDEN, 0x69);
        assert_eq!(DUNGEON_PIT_BOMB_TRAP_VISIBLE, 0x62);
        assert_eq!(DUNGEON_PIT_BOMB_TRAP_HIDDEN, 0x6A);
        // The fall-trap classifier accepts both fall bytes and
        // rejects both bomb bytes (and vice versa).
        assert!(is_dungeon_fall_trap(DUNGEON_PIT_FALL_TRAP_VISIBLE));
        assert!(is_dungeon_fall_trap(DUNGEON_PIT_FALL_TRAP_HIDDEN));
        assert!(!is_dungeon_fall_trap(DUNGEON_PIT_BOMB_TRAP_VISIBLE));
        assert!(!is_dungeon_fall_trap(DUNGEON_PIT_BOMB_TRAP_HIDDEN));
        assert!(is_dungeon_bomb_trap(DUNGEON_PIT_BOMB_TRAP_VISIBLE));
        assert!(is_dungeon_bomb_trap(DUNGEON_PIT_BOMB_TRAP_HIDDEN));
        assert!(!is_dungeon_bomb_trap(DUNGEON_PIT_FALL_TRAP_VISIBLE));
        assert!(!is_dungeon_bomb_trap(DUNGEON_PIT_FALL_TRAP_HIDDEN));
        // Neighbouring 0x6? bytes are neither — the 0x60 surface-
        // reset pit and other 0x6? cells keep their own behaviour.
        assert!(!is_dungeon_fall_trap(0x60));
        assert!(!is_dungeon_bomb_trap(0x60));
    }

    #[test]
    fn jimmy_chest_threshold_bias_is_shared_across_object_and_dungeon_paths() {
        // doors-and-z-transitions.md §3: both the per-map object
        // chest and the dungeon chest pick the lock threshold by
        // `(difficulty - member_class + 30) / 2`. The `+30` bias is
        // shared spec data and was duplicated as a bare literal in
        // both helpers. Promote it to JIMMY_CHEST_THRESHOLD_BIAS and
        // pin the resulting threshold equality for matching inputs.
        assert_eq!(JIMMY_CHEST_THRESHOLD_BIAS, 30);
        // A 2*depth value equal to an object difficulty produces the
        // same threshold for the same member-class. Use difficulty
        // 8 (object stat 0x88) versus dungeon depth 4 (`2*4 = 8`).
        let object_stat = 0x80 | 8u8;
        let dungeon_depth = 4u8;
        for class in 0u8..30 {
            let obj = object_chest_jimmy_threshold(object_stat, class).unwrap();
            let dun = dungeon_chest_jimmy_threshold(dungeon_depth, class);
            assert_eq!(obj, dun, "class {class}: object {obj} vs dungeon {dun}");
        }
    }

    #[test]
    fn karma_dat_band_width_matches_rescue_and_camp_selectors() {
        // blackthorn.md §7 and formats/karma-dat.md §4 describe the
        // KARMA.DAT verdict selectors as five (or six) twenty-point
        // bands. Promote the shared band width so the rescue/refuge
        // selector and the Lord British-in-disguise camp selector
        // agree at one named source of truth instead of repeating
        // `20`/`19`/`39`/... as bare literals.
        assert_eq!(KARMA_DAT_BAND_WIDTH, 20);
        // Every band boundary aligns to a multiple of the width.
        // The rescue selector caps at four bands (records 0..=4);
        // the camp selector also seeks directly to record 5 for the
        // top 80..=99 band but does not use the lower bands beyond
        // 0..=3.
        for band in 0u8..5 {
            let lo = band * KARMA_DAT_BAND_WIDTH;
            let hi = lo + KARMA_DAT_BAND_WIDTH - 1;
            assert_eq!(blackthorn_rescue_verdict_record(lo), band);
            assert_eq!(blackthorn_rescue_verdict_record(hi), band);
        }
        // Sanity: the moral-standing selector caps at 99, so the top
        // band's upper edge is exactly 99 (= 4 * 20 + 19).
        assert_eq!(4 * KARMA_DAT_BAND_WIDTH + KARMA_DAT_BAND_WIDTH - 1, 99);
    }

    #[test]
    fn input_code_direction_range_bounds_match_spec() {
        // input.md §5: the eight published direction codes occupy two
        // contiguous high-byte ranges — diagonals 0xD3..=0xD6 and
        // cardinals 0xFB..=0xFE. Promote both range bounds so callers
        // that probe whether a returned byte is a direction can use a
        // named range pair instead of bare hex literals.
        assert_eq!(INPUT_CODE_DIAGONAL_FIRST, 0xD3);
        assert_eq!(INPUT_CODE_DIAGONAL_LAST, 0xD6);
        assert_eq!(INPUT_CODE_CARDINAL_FIRST, 0xFB);
        assert_eq!(INPUT_CODE_CARDINAL_LAST, 0xFE);
        for byte in INPUT_CODE_DIAGONAL_FIRST..=INPUT_CODE_DIAGONAL_LAST {
            let dir = input_code_direction(byte).unwrap();
            assert!(!dir.is_cardinal(), "byte {byte:#04x} should be diagonal");
        }
        for byte in INPUT_CODE_CARDINAL_FIRST..=INPUT_CODE_CARDINAL_LAST {
            let dir = input_code_direction(byte).unwrap();
            assert!(dir.is_cardinal(), "byte {byte:#04x} should be cardinal");
        }
        // Bytes just outside each range do not classify as directions.
        assert_eq!(input_code_direction(INPUT_CODE_DIAGONAL_FIRST - 1), None);
        assert_eq!(input_code_direction(INPUT_CODE_DIAGONAL_LAST + 1), None);
        assert_eq!(input_code_direction(INPUT_CODE_CARDINAL_FIRST - 1), None);
        assert_eq!(input_code_direction(INPUT_CODE_CARDINAL_LAST + 1), None);
    }

    #[test]
    fn text_extended_control_byte_constants_match_spec_codes() {
        // text-output.md §3 / §5: the per-cell emitter recognises five
        // extended high-bit control bytes (0xFB..=0xFF) that mutate
        // the active window's cached style without rendering a glyph.
        // Promote each value so text_control_byte and the emitter's
        // range probe do not bake the hex codes as bare literals.
        assert_eq!(TEXT_CTRL_CENTRE_OFF, 0xFB);
        assert_eq!(TEXT_CTRL_CENTRE_ON, 0xFC);
        assert_eq!(TEXT_CTRL_INVERSE_TOGGLE, 0xFD);
        assert_eq!(TEXT_CTRL_UNDERLINE_TOGGLE, 0xFE);
        assert_eq!(TEXT_CTRL_CLEAR_WINDOW, 0xFF);
        assert_eq!(TEXT_CTRL_RANGE_FIRST, TEXT_CTRL_CENTRE_OFF);
        assert_eq!(TEXT_CTRL_RANGE_LAST, TEXT_CTRL_CLEAR_WINDOW);
        // Each named code resolves to the matching TextControlByte
        // variant; bytes just outside the band are not control bytes.
        assert_eq!(
            text_control_byte(TEXT_CTRL_CENTRE_OFF),
            Some(TextControlByte::CentreOff)
        );
        assert_eq!(
            text_control_byte(TEXT_CTRL_CENTRE_ON),
            Some(TextControlByte::CentreOn)
        );
        assert_eq!(
            text_control_byte(TEXT_CTRL_INVERSE_TOGGLE),
            Some(TextControlByte::InverseToggle)
        );
        assert_eq!(
            text_control_byte(TEXT_CTRL_UNDERLINE_TOGGLE),
            Some(TextControlByte::UnderlineToggle)
        );
        assert_eq!(
            text_control_byte(TEXT_CTRL_CLEAR_WINDOW),
            Some(TextControlByte::ClearWindow)
        );
        assert_eq!(text_control_byte(TEXT_CTRL_RANGE_FIRST - 1), None);
    }

    #[test]
    fn viewport_center_and_max_index_match_viewport_side() {
        // visibility.md §2,§7: the player sits at the viewport centre
        // and the grid is 11 wide. Promote the centre column/row
        // index and the maximum valid index so the fog-refine helpers
        // do not bake `5` and `10` as bare literals next to a
        // VIEWPORT_SIDE constant of 11.
        assert_eq!(VIEWPORT_SIDE, 11);
        assert_eq!(VIEWPORT_CENTER, 5);
        assert_eq!(VIEWPORT_MAX_INDEX, 10);
        assert_eq!(VIEWPORT_CENTER as usize, VIEWPORT_SIDE / 2);
        assert_eq!(VIEWPORT_MAX_INDEX as usize + 1, VIEWPORT_SIDE);
        // The centre cell is the only cell whose folded coordinate
        // equals the centre, and its squared distance is zero.
        assert_eq!(fog_refine_folded_coord(VIEWPORT_CENTER), VIEWPORT_CENTER);
        assert_eq!(
            fog_refine_squared_distance(VIEWPORT_CENTER, VIEWPORT_CENTER),
            0
        );
        // The four viewport corners fold symmetrically back to (0, 0)
        // and (10, 10), giving the same `((5 - 0)^2 + (5 - 0)^2) = 50`.
        for &(col, row) in &[
            (0u8, 0u8),
            (VIEWPORT_MAX_INDEX, 0),
            (0, VIEWPORT_MAX_INDEX),
            (VIEWPORT_MAX_INDEX, VIEWPORT_MAX_INDEX),
        ] {
            assert_eq!(fog_refine_squared_distance(col, row), 50);
        }
    }

    #[test]
    fn save_scene_overworld_matches_main_loop_scene_overworld() {
        // save-load.md §4.2 names the overworld scene byte as
        // SAVE_SCENE_OVERWORLD; main-loop.md §3 names the same value
        // as SCENE_OVERWORLD when classifying the outer-dispatch
        // branch. Both spell out the same byte value (`0`). Pin
        // equality so a future rename or value change on either side
        // cannot silently desync the save-side underworld disk-swap
        // check from the outer-loop scene routing.
        assert_eq!(SAVE_SCENE_OVERWORLD, SCENE_OVERWORLD);
    }

    #[test]
    fn chargen_starting_calendar_matches_endgame_elapsed_time_baseline() {
        // chargen.md §8 names the seeded starting calendar; endgame.md
        // §9 names the elapsed-time baseline the Avatarhood certificate
        // subtracts from the saved world clock. They describe the same
        // year/month/day from two viewpoints, so the two named-constant
        // triples must always agree. Pin equality so a future edit
        // cannot silently desync them.
        assert_eq!(CHARGEN_STARTING_YEAR, ENDGAME_CAMPAIGN_START_YEAR);
        assert_eq!(CHARGEN_STARTING_MONTH, ENDGAME_CAMPAIGN_START_MONTH);
        assert_eq!(CHARGEN_STARTING_DAY, ENDGAME_CAMPAIGN_START_DAY);
    }

    #[test]
    fn disk_prompt_mode_alias_constants_match_spec_normalisation() {
        // screen-mode-dispatch.md §5: the disk-prompt request folds
        // historical modes 2 and 5 to mode 1, and lets other values
        // pass through. Promote the canonical mode and the two
        // aliased inputs so the normalizer does not bake `1`, `2`,
        // and `5` as bare literals.
        assert_eq!(DISK_PROMPT_MODE_CANONICAL, 1);
        assert_eq!(DISK_PROMPT_MODE_ALIAS_A, 2);
        assert_eq!(DISK_PROMPT_MODE_ALIAS_B, 5);
        assert_eq!(
            normalize_disk_prompt_mode(DISK_PROMPT_MODE_ALIAS_A),
            DISK_PROMPT_MODE_CANONICAL
        );
        assert_eq!(
            normalize_disk_prompt_mode(DISK_PROMPT_MODE_ALIAS_B),
            DISK_PROMPT_MODE_CANONICAL
        );
        // Every other input passes through unchanged.
        for mode in 0u8..=u8::MAX {
            if mode == DISK_PROMPT_MODE_ALIAS_A || mode == DISK_PROMPT_MODE_ALIAS_B {
                continue;
            }
            assert_eq!(normalize_disk_prompt_mode(mode), mode);
        }
    }

    #[test]
    fn active_object_eviction_phase_class_ranges_match_spec() {
        // active-objects.md §4: the eviction cascade names four class
        // ranges paired across phases 2..=5 (off-screen) and 6..=9
        // (visible-allowed). Promote each range to a named constant
        // so the eviction predicate does not bake `0x01..=0x0F`,
        // `0x80..`, `0x10`/`0x11`, and `0x30..=0x7F` as bare hex
        // literals.
        assert_eq!(ACTIVE_OBJECT_EVICTION_SCENERY_FIRST, 0x01);
        assert_eq!(ACTIVE_OBJECT_EVICTION_SCENERY_LAST, 0x0F);
        assert_eq!(ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST, 0x80);
        assert_eq!(ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_FIRST, 0x10);
        assert_eq!(ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_LAST, 0x11);
        assert_eq!(ACTIVE_OBJECT_EVICTION_MIDRANGE_FIRST, 0x30);
        assert_eq!(ACTIVE_OBJECT_EVICTION_MIDRANGE_LAST, 0x7F);
        // Scenery range — phases 2 and 6 accept every byte in the
        // band, reject every byte outside it.
        for byte in
            ACTIVE_OBJECT_EVICTION_SCENERY_FIRST..=ACTIVE_OBJECT_EVICTION_SCENERY_LAST
        {
            assert!(active_object_eviction_byte_accepted(byte, 2));
            assert!(active_object_eviction_byte_accepted(byte, 6));
        }
        // Dynamic-actor range, with the protected `0xB5` excluded.
        for byte in ACTIVE_OBJECT_EVICTION_DYNAMIC_FIRST..=u8::MAX {
            let expected = byte != ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE;
            assert_eq!(active_object_eviction_byte_accepted(byte, 3), expected);
            assert_eq!(active_object_eviction_byte_accepted(byte, 7), expected);
        }
        // Door/fixture-like pair.
        for byte in 0u8..=u8::MAX {
            let expected = byte == ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_FIRST
                || byte == ACTIVE_OBJECT_EVICTION_DOOR_FIXTURE_LAST;
            assert_eq!(active_object_eviction_byte_accepted(byte, 4), expected);
            assert_eq!(active_object_eviction_byte_accepted(byte, 8), expected);
        }
        // Midrange object range.
        for byte in
            ACTIVE_OBJECT_EVICTION_MIDRANGE_FIRST..=ACTIVE_OBJECT_EVICTION_MIDRANGE_LAST
        {
            assert!(active_object_eviction_byte_accepted(byte, 5));
            assert!(active_object_eviction_byte_accepted(byte, 9));
        }
    }

    #[test]
    fn combat_to_hit_bias_matches_spec_formula() {
        // catalogs/item-list.md §5.3: the shared combat to-hit
        // helper computes `(attacker - defender + 30) / 2` and
        // compares it against a uniform random byte. Promote the
        // +30 bias so combat_to_hit_score does not bake it as a
        // bare literal. The bias is kept separate from the
        // JIMMY_CHEST_THRESHOLD_BIAS (also 30) because the two
        // contracts share only the magnitude, not the meaning.
        assert_eq!(COMBAT_TO_HIT_BIAS, 30);
        // Two equal-rating actors land at the bias / 2 = 15
        // threshold; combat_hit succeeds against rolls below it.
        assert_eq!(combat_to_hit_score(50, 50), 15);
        assert_eq!(combat_to_hit_score(0, 0), 15);
        // Higher attacker raises the score linearly; +20 difference
        // adds 10 to the score.
        assert_eq!(combat_to_hit_score(60, 50), 20);
    }

    #[test]
    fn moonstone_burial_band_constants_match_spec() {
        // formats/saved-gam.md §7.2: Moonstone burying is accepted on
        // tile ids 4..=10 (a contiguous overworld band) plus 44 and
        // 45 (two extras). Promote the band edges and the extra
        // singletons so moonstone_burial_tile_accepted does not bake
        // the values as a bare match arm.
        assert_eq!(MOONSTONE_BURIAL_BAND_FIRST, 4);
        assert_eq!(MOONSTONE_BURIAL_BAND_LAST, 10);
        assert_eq!(MOONSTONE_BURIAL_TILE_EXTRA_A, 44);
        assert_eq!(MOONSTONE_BURIAL_TILE_EXTRA_B, 45);
        for tile in MOONSTONE_BURIAL_BAND_FIRST..=MOONSTONE_BURIAL_BAND_LAST {
            assert!(moonstone_burial_tile_accepted(tile));
        }
        assert!(moonstone_burial_tile_accepted(MOONSTONE_BURIAL_TILE_EXTRA_A));
        assert!(moonstone_burial_tile_accepted(MOONSTONE_BURIAL_TILE_EXTRA_B));
        // The two extras are adjacent but a tile just outside either
        // boundary is rejected.
        assert!(!moonstone_burial_tile_accepted(MOONSTONE_BURIAL_BAND_FIRST - 1));
        assert!(!moonstone_burial_tile_accepted(MOONSTONE_BURIAL_BAND_LAST + 1));
        assert!(!moonstone_burial_tile_accepted(MOONSTONE_BURIAL_TILE_EXTRA_B + 1));
    }

    #[test]
    fn chargen_seed_defense_byte_matches_saved_gam_spec() {
        // formats/saved-gam.md §3.1: the per-character defense byte
        // at record offset 0x18 is shipped as `7` for every roster
        // slot, and no traced writer recomputes it from readied
        // equipment. Promote the value so seed verifiers and combat
        // damage code can compare against the spec.
        assert_eq!(CHARGEN_SEED_DEFENSE_BYTE, 7);
    }

    #[test]
    fn chargen_starting_calendar_matches_init_gam_seed() {
        // chargen.md §8: a fresh-from-questionnaire save begins at
        // year 139, month 4, day 5, 08:35 of the in-world calendar.
        // The bytes come from INIT.GAM; chargen does not customise
        // them. Promoting the values to named constants lets fixture
        // builders and seed verifiers compare against the spec
        // directly instead of literal magic numbers.
        assert_eq!(CHARGEN_STARTING_YEAR, 139);
        assert_eq!(CHARGEN_STARTING_MONTH, 4);
        assert_eq!(CHARGEN_STARTING_DAY, 5);
        assert_eq!(CHARGEN_STARTING_HOUR, 8);
        assert_eq!(CHARGEN_STARTING_MINUTE, 35);
        // Sanity-check that the seeded month/day are inside the
        // documented calendar ranges (month one-based 1..=13, day
        // one-based 1..=28 per saved-gam.md §5).
        assert!((1..=13).contains(&CHARGEN_STARTING_MONTH));
        assert!((1..=28).contains(&CHARGEN_STARTING_DAY));
        assert!(CHARGEN_STARTING_HOUR < 24);
        assert!(CHARGEN_STARTING_MINUTE < 60);
    }

    #[test]
    fn combat_post_round_magic_effect_timer_ticks_in_open_interval() {
        // combat.md §7: in the post-round maintenance pass, a terrain
        // byte `0xDC` ticks the shared combat magic-effect timer
        // while the timer is nonzero and strictly below sixteen.
        // Promote both edges so a future post-round pass does not
        // bake them as bare literals.
        assert_eq!(COMBAT_POST_ROUND_NO_EFFECT_SENTINEL, 0x16);
        assert_eq!(COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX, 16);
        // The timer ticks for every value in the open interval
        // (0, 16). Zero is inert and a timer at the cap stops.
        assert!(!combat_post_round_magic_effect_timer_ticks(0));
        for timer in 1u8..COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX {
            assert!(combat_post_round_magic_effect_timer_ticks(timer));
        }
        assert!(!combat_post_round_magic_effect_timer_ticks(
            COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX
        ));
        assert!(!combat_post_round_magic_effect_timer_ticks(
            COMBAT_POST_ROUND_MAGIC_EFFECT_TIMER_MAX + 1
        ));
    }

    #[test]
    fn combat_arena_outdoor_slice_helpers_route_through_named_band_ranges() {
        // formats/cbt.md §5: the outdoor metadata band on the
        // 11-wide arena row holds two 6-byte setup tables on row 3
        // (columns 11..=16 and 17..=22), and the sixteen placement
        // X/Y slot coordinates on rows 6 and 7 (columns 11..=26).
        // CBT_SETUP_TABLE_ROW, CBT_PLACEMENT_X_ROW,
        // CBT_PLACEMENT_Y_ROW, and the column ranges already named
        // those positions; route the record accessor methods through
        // the published constants instead of bare numeric literals.
        assert_eq!(CBT_SETUP_TABLE_ROW, 3);
        assert_eq!(*CBT_SETUP_TABLE_A_COLUMNS.start(), 11);
        assert_eq!(*CBT_SETUP_TABLE_B_COLUMNS.start(), 17);
        assert_eq!(CBT_PLACEMENT_X_ROW, 6);
        assert_eq!(CBT_PLACEMENT_Y_ROW, 7);
        assert_eq!(*CBT_PLACEMENT_COLUMNS.start(), 11);
        assert_eq!(CBT_PLACEMENT_SLOT_COUNT, 16);
        // Stamp a synthetic record with monotonically increasing
        // bytes so each slice extracts a known range.
        let mut bytes = [0u8; COMBAT_ARENA_RECORD_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i % 256) as u8;
        }
        let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
        let row_a_base = CBT_SETUP_TABLE_ROW * COMBAT_ARENA_ROW_STRIDE;
        let table_a = record.outdoor_setup_table_a();
        for (i, value) in table_a.iter().enumerate() {
            assert_eq!(
                *value,
                ((row_a_base + *CBT_SETUP_TABLE_A_COLUMNS.start() + i) % 256) as u8
            );
        }
        let placement_x = record.outdoor_placement_x();
        assert_eq!(placement_x.len(), CBT_PLACEMENT_SLOT_COUNT);
    }

    #[test]
    fn natural_moongate_counter_night_band_uses_shared_lighting_hours() {
        // overworld.md §9: the natural-moongate gate-presence counter
        // grows during the surface night band and shrinks during the
        // daylight band. The day/night boundary is the same DAWN_HOUR
        // / DUSK_HOUR pair lighting.md §3 already names. Route the
        // counter-step classifier through those constants so the two
        // specs stay aligned at the same byte boundaries.
        // Every daylight hour produces Decrease.
        for hour in DAWN_HOUR..=DUSK_HOUR {
            assert_eq!(
                natural_moongate_counter_step(hour),
                NaturalMoongateCounterStep::Decrease,
                "hour {hour}"
            );
        }
        // Every night-band hour outside [DAWN_HOUR, DUSK_HOUR] grows
        // the counter.
        for hour in 0..DAWN_HOUR {
            assert_eq!(
                natural_moongate_counter_step(hour),
                NaturalMoongateCounterStep::Increase,
                "pre-dawn hour {hour}"
            );
        }
        for hour in (DUSK_HOUR + 1)..24 {
            assert_eq!(
                natural_moongate_counter_step(hour),
                NaturalMoongateCounterStep::Increase,
                "post-dusk hour {hour}"
            );
        }
    }

    #[test]
    fn world_plane_fall_damage_max_routes_through_named_constant() {
        // overworld.md §2: chasm and whirlpool plane writers apply a
        // random fall-damage roll per conscious party member. Promote
        // the upper bound so world_plane_fall_damage_roll does not
        // bake `5` as a bare modulus literal.
        assert_eq!(WORLD_PLANE_FALL_DAMAGE_MAX, 5);
        // The roll is `1..=WORLD_PLANE_FALL_DAMAGE_MAX`; every value
        // produced by the modulo is in that range.
        for seed in 0u8..=u8::MAX {
            let roll = 1 + (seed % WORLD_PLANE_FALL_DAMAGE_MAX);
            assert!((1..=WORLD_PLANE_FALL_DAMAGE_MAX).contains(&roll));
        }
    }

    #[test]
    fn movement_chair_force_reject_exempt_families_are_named() {
        // movement.md §4: the chair-tile force-reject for `0x90..=0x93`
        // is skipped for the on-foot/avatar query family
        // `0x1C..=0x1F` and for the `0x40` single-tile query. Promote
        // both exemption identities to named constants so the
        // dispatcher does not encode them as bare hex literals inside
        // the force-reject helper.
        assert_eq!(MOVEMENT_QUERY_FOOT_AVATAR_FIRST, 0x1C);
        assert_eq!(MOVEMENT_QUERY_FOOT_AVATAR_LAST, 0x1F);
        assert_eq!(MOVEMENT_QUERY_SINGLE_TILE_0X40, 0x40);
        // On-foot facings are exempt over every chair-variant id.
        for facing in MOVEMENT_QUERY_FOOT_AVATAR_FIRST..=MOVEMENT_QUERY_FOOT_AVATAR_LAST {
            for tile in MOVEMENT_CHAIR_FORCE_REJECT_FIRST..=MOVEMENT_CHAIR_FORCE_REJECT_LAST {
                assert!(
                    !movement_chair_force_reject_applies(facing, tile),
                    "foot facing {facing:#04x} should not reject chair {tile:#04x}"
                );
            }
        }
        // The single-tile 0x40 query is exempt too.
        for tile in MOVEMENT_CHAIR_FORCE_REJECT_FIRST..=MOVEMENT_CHAIR_FORCE_REJECT_LAST {
            assert!(!movement_chair_force_reject_applies(
                MOVEMENT_QUERY_SINGLE_TILE_0X40,
                tile
            ));
        }
        // A non-exempt query (e.g. mounted horse 0x12) does respect
        // the force-reject for every chair-variant id.
        for tile in MOVEMENT_CHAIR_FORCE_REJECT_FIRST..=MOVEMENT_CHAIR_FORCE_REJECT_LAST {
            assert!(movement_chair_force_reject_applies(0x12, tile));
        }
        // Tiles outside the chair range are not subject to the
        // force-reject regardless of query.
        assert!(!movement_chair_force_reject_applies(
            0x12,
            MOVEMENT_CHAIR_FORCE_REJECT_FIRST - 1
        ));
        assert!(!movement_chair_force_reject_applies(
            0x12,
            MOVEMENT_CHAIR_FORCE_REJECT_LAST + 1
        ));
    }

    #[test]
    fn tile_passability_bitset_uses_msb_first_packing() {
        // movement.md §4: the base terrain bitset is a thirty-two-
        // byte run where each byte packs one bit per tile id, read
        // most-significant first. Promote the MSB and the low-bit
        // index mask so the lookup helper does not bake `0x80` and
        // `7` as bare literals.
        assert_eq!(TILE_PASSABILITY_BIT_MSB, 0x80);
        assert_eq!(TILE_PASSABILITY_BIT_INDEX_MASK, 7);
        // Build a one-tile bitset stamped at tile id 0 (bit 7 of the
        // first byte) and confirm only that tile resolves passable.
        let mut bytes = [0u8; TILE_PASSABILITY_LEN];
        bytes[0] = TILE_PASSABILITY_BIT_MSB;
        let passability = TilePassability::from_bytes(&bytes).unwrap();
        assert!(passability.is_passable(0));
        for tile in 1u8..=8 {
            assert!(!passability.is_passable(tile));
        }
        // Stamp tile id 7 (bit 0 of the first byte) — the MSB-first
        // shift makes this the last bit of the first byte.
        let mut bytes = [0u8; TILE_PASSABILITY_LEN];
        bytes[0] = 0x01;
        let passability = TilePassability::from_bytes(&bytes).unwrap();
        assert!(passability.is_passable(7));
        for tile in 0u8..=6 {
            assert!(!passability.is_passable(tile));
        }
    }

    #[test]
    fn dungeon_active_object_spawn_band_matches_spec() {
        // dungeon-mode.md §4.1: dungeon view initialisation accepts
        // only cells in the pit (0x6?) or corridor (0x7?) classes
        // when placing the active-object, and bails after eight
        // random attempts. Promote both edges of the spawn band and
        // the attempt cap to named constants.
        assert_eq!(DUNGEON_ACTIVE_OBJECT_PLACEMENT_ATTEMPTS, 8);
        assert_eq!(DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_FIRST, 0x60);
        assert_eq!(DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_LAST, 0x7F);
        // Every tile in the band is a legal spawn target.
        for tile in
            DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_FIRST..=DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_LAST
        {
            assert!(dungeon_active_object_spawn_accepts(tile), "tile {tile:#04x}");
        }
        // Tiles just outside the band are rejected: 0x5F (fountain)
        // and 0x80 (sleep field).
        assert!(!dungeon_active_object_spawn_accepts(
            DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_FIRST - 1
        ));
        assert!(!dungeon_active_object_spawn_accepts(
            DUNGEON_ACTIVE_OBJECT_SPAWN_TILE_LAST + 1
        ));
        // Passage (0x00) and wall classes are rejected too — only
        // the pit/corridor band is a legal spawn cell.
        assert!(!dungeon_active_object_spawn_accepts(0x00));
        assert!(!dungeon_active_object_spawn_accepts(0xB0));
    }

    #[test]
    fn talk_branch_flag_bank_caps_at_thirty_two_bits() {
        // conversation.md §10: "Branch bit indices `32` and above
        // build a zero mask rather than wrapping, so such tests read
        // as clear and such setters are no-ops." Promote the bank
        // width to a named constant so the mask helper does not
        // bake `32` as a bare literal.
        assert_eq!(TALK_BRANCH_FLAG_BANK_BITS, 32);
        // Bits inside the bank produce nonzero, in-range masks.
        for bit in 0..TALK_BRANCH_FLAG_BANK_BITS {
            let mask = talk_branch_flag_mask(bit);
            assert_eq!(mask, 1u32 << bit);
        }
        // Bits at or beyond the bank build a zero mask. An IF test
        // therefore reads as clear, and a SET-FLAG write does not
        // mutate the slot.
        assert_eq!(talk_branch_flag_mask(TALK_BRANCH_FLAG_BANK_BITS), 0);
        assert_eq!(talk_branch_flag_mask(50), 0);
        assert!(!talk_branch_flag_is_set(u32::MAX, TALK_BRANCH_FLAG_BANK_BITS));
        let mut slot = 0u32;
        assert!(!set_talk_branch_flag(&mut slot, TALK_BRANCH_FLAG_BANK_BITS));
        assert_eq!(slot, 0);
    }

    #[test]
    fn talk_through_tile_band_constants_match_spec() {
        // conversation.md §2: Talk advances `(dx, dy)` once more when
        // the facing tile is a talk-through tile (shop counter, low
        // fence, etc.) so the player can address an NPC across the
        // barrier. Promote the band edges to named constants so the
        // is_talk_through_tile classifier does not bake `64..=71` as
        // bare literals.
        assert_eq!(TALK_THROUGH_TILE_FIRST, 64);
        assert_eq!(TALK_THROUGH_TILE_LAST, 71);
        // Every tile in the band classifies as talk-through.
        for tile in TALK_THROUGH_TILE_FIRST..=TALK_THROUGH_TILE_LAST {
            assert!(is_talk_through_tile(tile), "tile {tile:#04x}");
        }
        // The neighbouring tiles on either side do not.
        assert!(!is_talk_through_tile(TALK_THROUGH_TILE_FIRST - 1));
        assert!(!is_talk_through_tile(TALK_THROUGH_TILE_LAST + 1));
    }

    #[test]
    fn protection_active_effect_adds_three_to_defense() {
        // magic.md §8: while Protection's `P` active-effect tag is
        // live, the resident party-member defense helper adds three
        // points after equipment defense is summed. Promote the
        // bonus to PROTECTION_ACTIVE_EFFECT_DEFENSE_BONUS so the
        // resolve_protection_defense_bonus helper does not bake `3`
        // as a bare literal.
        assert_eq!(PROTECTION_ACTIVE_EFFECT_DEFENSE_BONUS, 3);
        // While active, the helper raises the base defense by the
        // bonus.
        assert_eq!(
            resolve_protection_defense_bonus(
                10,
                Some(PROTECTION_ACTIVE_EFFECT_TAG),
                PROTECTION_ACTIVE_EFFECT_DURATION,
            ),
            10 + PROTECTION_ACTIVE_EFFECT_DEFENSE_BONUS
        );
        // The saturating_add must not wrap when base defense is at
        // or near the byte ceiling.
        assert_eq!(
            resolve_protection_defense_bonus(
                254,
                Some(PROTECTION_ACTIVE_EFFECT_TAG),
                PROTECTION_ACTIVE_EFFECT_DURATION,
            ),
            u8::MAX
        );
        // Without the live tag, base defense passes through
        // unchanged.
        assert_eq!(
            resolve_protection_defense_bonus(
                10,
                Some(QUICKNESS_ACTIVE_EFFECT_TAG),
                QUICKNESS_ACTIVE_EFFECT_DURATION,
            ),
            10
        );
    }

    #[test]
    fn resurrection_max_hp_per_level_matches_spec() {
        // magic.md §8: a successful Resurrect rebuilds the revived
        // member's record and sets maximum HP to thirty times the
        // recomputed level. Promote the multiplier to a named
        // constant so resurrect_party_member_to_hp does not bake
        // `30` as a bare literal.
        assert_eq!(RESURRECTION_MAX_HP_PER_LEVEL, 30);
        // Spot-check a few resurrected-level → max-HP results so
        // future edits cannot quietly change the multiplier.
        for level in [1u16, 3, 5, 8] {
            let max_hp = level * RESURRECTION_MAX_HP_PER_LEVEL;
            assert_eq!(max_hp, level * 30);
        }
    }

    #[test]
    fn karma_crop_or_table_food_debit_is_one() {
        // karma.md §4: picking a crop cell or eating reachable table
        // food reduces the shared moral-standing selector by one
        // (when nonzero). Promote the value to a named constant so
        // the table-food handler and the karma action table do not
        // bake it as a bare `1` / `-1`.
        assert_eq!(KARMA_CROP_OR_TABLE_FOOD_DEBIT, 1);
        assert_eq!(
            KarmaAction::CropOrTableFoodTaken.signed_delta(),
            -(KARMA_CROP_OR_TABLE_FOOD_DEBIT as i16)
        );
    }

    #[test]
    fn shrine_codex_turn_in_moral_increase_is_three() {
        // karma.md §4: shrine Codex turn-in adds +3 to the shared
        // moral-standing selector (and to the per-virtue shrine
        // standing slot), with Humility receiving the same increase
        // a second time as a follow-up bonus. Promote the value to a
        // named associated constant so the shrine handler does not
        // bake `3` as a bare literal in two places.
        assert_eq!(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE, 3);
        // Humility's documented follow-up bonus equals the same +3.
        assert_eq!(
            ShrineVirtue::Humility.codex_turn_in_humility_bonus(),
            ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE
        );
        // Non-Humility virtues do not pay the follow-up bonus.
        for virtue in [
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
        ] {
            assert_eq!(virtue.codex_turn_in_humility_bonus(), 0);
        }
    }

    #[test]
    fn dawn_dusk_hour_constants_match_spec() {
        // lighting.md §3: the surface day-night cycle uses two named
        // gradient hours and six ten-minute levels. Promote the dawn
        // hour, dusk hour, step width, and last-minute boundary to
        // named constants so the daylight_base_value helper does not
        // bake `5`, `19`, `10`, and `59` as bare literals.
        assert_eq!(DAWN_HOUR, 5);
        assert_eq!(DUSK_HOUR, 19);
        assert_eq!(DAWN_DUSK_STEP_MINUTES, 10);
        assert_eq!(LAST_IN_HOUR_MINUTE, 59);
        assert_eq!(DAWN_DUSK_LAST_INDEX, 5);
        assert_eq!(DAWN_DUSK_LIGHT.len(), 6);
        // Six 10-minute levels exactly fill the dawn hour.
        assert_eq!(
            DAWN_DUSK_LIGHT.len() as u8 * DAWN_DUSK_STEP_MINUTES,
            60,
            "six ten-minute steps cover the full dawn/dusk hour"
        );
        // 05:00 starts at the darkest published level and 05:59
        // settles at the brightest published level.
        assert_eq!(daylight_base_value(DAWN_HOUR, 0, false, 0), DAWN_DUSK_LIGHT[0]);
        assert_eq!(
            daylight_base_value(DAWN_HOUR, LAST_IN_HOUR_MINUTE, false, 0),
            DAWN_DUSK_LIGHT[DAWN_DUSK_LAST_INDEX]
        );
        // 19:00 starts at the brightest level (dusk reverses the
        // gradient) and 19:59 settles at the darkest.
        assert_eq!(
            daylight_base_value(DUSK_HOUR, 0, false, 0),
            DAWN_DUSK_LIGHT[DAWN_DUSK_LAST_INDEX]
        );
        assert_eq!(
            daylight_base_value(DUSK_HOUR, LAST_IN_HOUR_MINUTE, false, 0),
            DAWN_DUSK_LIGHT[0]
        );
        // 04:59 and 20:00 are full darkness on the surface.
        assert_eq!(
            daylight_base_value(DAWN_HOUR - 1, LAST_IN_HOUR_MINUTE, false, 0),
            FULL_DARKNESS
        );
        assert_eq!(daylight_base_value(DUSK_HOUR + 1, 0, false, 0), FULL_DARKNESS);
        // The underworld and dungeon depths force full darkness
        // regardless of the hour.
        assert_eq!(daylight_base_value(12, 0, true, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(12, 0, false, 1), FULL_DARKNESS);
    }

    #[test]
    fn town_cannon_named_constants_match_spec() {
        // vehicles.md §8: the town F-Fire path traces a short fixed
        // line for the first blocking target, and on an active-object
        // hit it reduces the shared moral-standing selector by five
        // units (floored at zero by the karma helper). Promote both
        // numbers to named constants so the F-Fire handler does not
        // bake `3` and `5` as bare literals.
        assert_eq!(TOWN_CANNON_RANGE_CELLS, 3);
        assert_eq!(TOWN_CANNON_HIT_KARMA_DEBIT, 5);
        // The town cannon range is the same magnitude as the ship
        // broadside range, but the spec keeps them as independent
        // contracts (one ship-only, one town-only). Both being 3 is
        // a coincidence of the published data, not a shared constant.
        assert_eq!(
            TOWN_CANNON_RANGE_CELLS as u8,
            SHIP_BROADSIDE_RANGE_CELLS,
            "spec keeps these contracts independent even though the v1 \
             values both happen to be three"
        );
    }

    #[test]
    fn random_encounter_threshold_band_constants_match_spec_table() {
        // encounters.md §3: each row of the surface threshold table is
        // a paired (day, night) value keyed on a tile band. Promote
        // each value to a named constant so the table-driven helper
        // does not encode the numbers as bare literals.
        assert_eq!(RANDOM_ENCOUNTER_SAFE_TILE_FIRST, 0x20);
        assert_eq!(RANDOM_ENCOUNTER_SAFE_TILE_LAST, 0x26);
        assert_eq!(RANDOM_ENCOUNTER_SAFE_DAY_THRESHOLD, 0);
        assert_eq!(RANDOM_ENCOUNTER_SAFE_NIGHT_THRESHOLD, 3);
        assert_eq!(RANDOM_ENCOUNTER_WILDERNESS_SWAMP, 0x04);
        assert_eq!(RANDOM_ENCOUNTER_WILDERNESS_BAND_FIRST, 0x09);
        assert_eq!(RANDOM_ENCOUNTER_WILDERNESS_BAND_LAST, 0x0F);
        assert_eq!(RANDOM_ENCOUNTER_WILDERNESS_DAY_THRESHOLD, 2);
        assert_eq!(RANDOM_ENCOUNTER_WILDERNESS_NIGHT_THRESHOLD, 5);
        assert_eq!(RANDOM_ENCOUNTER_DEFAULT_DAY_THRESHOLD, 1);
        assert_eq!(RANDOM_ENCOUNTER_DEFAULT_NIGHT_THRESHOLD, 4);
        // Spot-check that the threshold function still returns the
        // documented value for each band, by day and by night.
        let day = RANDOM_ENCOUNTER_NIGHT_HOUR_LAST + 1;
        let night = 0;
        for safe_tile in RANDOM_ENCOUNTER_SAFE_TILE_FIRST..=RANDOM_ENCOUNTER_SAFE_TILE_LAST {
            assert_eq!(
                random_encounter_threshold(false, safe_tile, day),
                RANDOM_ENCOUNTER_SAFE_DAY_THRESHOLD
            );
            assert_eq!(
                random_encounter_threshold(false, safe_tile, night),
                RANDOM_ENCOUNTER_SAFE_NIGHT_THRESHOLD
            );
        }
        assert_eq!(
            random_encounter_threshold(false, RANDOM_ENCOUNTER_WILDERNESS_SWAMP, day),
            RANDOM_ENCOUNTER_WILDERNESS_DAY_THRESHOLD
        );
        assert_eq!(
            random_encounter_threshold(false, RANDOM_ENCOUNTER_WILDERNESS_SWAMP, night),
            RANDOM_ENCOUNTER_WILDERNESS_NIGHT_THRESHOLD
        );
        for tile in
            RANDOM_ENCOUNTER_WILDERNESS_BAND_FIRST..=RANDOM_ENCOUNTER_WILDERNESS_BAND_LAST
        {
            assert_eq!(
                random_encounter_threshold(false, tile, day),
                RANDOM_ENCOUNTER_WILDERNESS_DAY_THRESHOLD
            );
            assert_eq!(
                random_encounter_threshold(false, tile, night),
                RANDOM_ENCOUNTER_WILDERNESS_NIGHT_THRESHOLD
            );
        }
        // Any other surface tile uses the default band.
        assert_eq!(
            random_encounter_threshold(false, 0x30, day),
            RANDOM_ENCOUNTER_DEFAULT_DAY_THRESHOLD
        );
        assert_eq!(
            random_encounter_threshold(false, 0x30, night),
            RANDOM_ENCOUNTER_DEFAULT_NIGHT_THRESHOLD
        );
    }

    #[test]
    fn npc_pathfind_visit_stamp_encodes_direction_in_high_nibble() {
        // npc-schedules.md §8.2: BFS overwrites each visited cell with
        // `direction << 4`. The shift width and seed value should be
        // named so workspace-builder code does not bake `4` and `0x40`
        // as bare literals.
        assert_eq!(NPC_PATHFIND_DIRECTION_SHIFT, 4);
        assert_eq!(NPC_PATHFIND_START_SEED, 0x40);
        // Visit stamp for each cardinal direction lands in the high
        // nibble exactly, leaving the low nibble clear so the cell's
        // pre-existing marker (open / goal sentinel) can still be read
        // by BFS via capture-before-write.
        assert_eq!(npc_pathfind_visit_stamp(NPC_PATH_DIR_WEST), 0x10);
        assert_eq!(npc_pathfind_visit_stamp(NPC_PATH_DIR_SOUTH), 0x20);
        assert_eq!(npc_pathfind_visit_stamp(NPC_PATH_DIR_EAST), 0x30);
        assert_eq!(npc_pathfind_visit_stamp(NPC_PATH_DIR_NORTH), 0x40);
        // The start-cell seed is by construction the north-direction
        // visit stamp.
        assert_eq!(
            NPC_PATHFIND_START_SEED,
            npc_pathfind_visit_stamp(NPC_PATH_DIR_NORTH)
        );
    }

    #[test]
    fn town_rest_hour_wrap_uses_compatibility_subtrahend() {
        // rest-and-camp.md §4: "If current hour plus digit exceeds 23,
        // the original subtracts 23 rather than applying a normal
        // modulo-24 wrap; preserve that compatibility edge."
        assert_eq!(TOWN_REST_HOUR_WRAP_SUBTRAHEND, 23);
        assert_eq!(TOWN_REST_TICK_BUDGET, 144);
        assert_eq!(TOWN_REST_DURATION_DIGIT_MAX, 9);
        // 22 + 2 = 24 → 24 - 23 = 1 (not modulo 0).
        assert_eq!(PlayState::town_rest_target_hour(22, 2), 1);
        // 23 + 1 = 24 → 24 - 23 = 1 (not modulo 0).
        assert_eq!(PlayState::town_rest_target_hour(23, 1), 1);
        // 23 + 9 = 32 → 32 - 23 = 9 (this is the documented edge —
        // ordinary modulo-24 would yield 8).
        assert_eq!(PlayState::town_rest_target_hour(23, 9), 9);
        // No-wrap path: 5 + 4 = 9 stays at 9.
        assert_eq!(PlayState::town_rest_target_hour(5, 4), 9);
        // Equal-to-23 boundary stays untouched (target == 23, not > 23).
        assert_eq!(PlayState::town_rest_target_hour(22, 1), 23);
    }

    #[test]
    fn wind_drift_named_constants_match_spec() {
        // weather.md §2: the autonomous wind-drift gates use three
        // published thresholds. Promote each one to a named constant
        // so call sites do not pass `0x3f`, `5`, or `192` as raw
        // literals.
        assert_eq!(WIND_DRIFT_OUTER_ROLL_MASK, 0x3F);
        assert_eq!(WIND_DRIFT_CANDIDATE_MODULUS, 5);
        assert_eq!(WIND_DRIFT_CALM_ACCEPT_MIN, 192);
        // Outer gate: zero, and any roll whose low six bits are zero,
        // is accepted. Every roll with at least one set bit in that
        // window is rejected.
        assert!(WindState::autonomous_drift_outer_accepted(0));
        assert!(WindState::autonomous_drift_outer_accepted(0x40));
        assert!(WindState::autonomous_drift_outer_accepted(0xC0));
        for r in 1u8..=63 {
            assert!(!WindState::autonomous_drift_outer_accepted(r));
        }
        // Calm acceptance: strictly below the published threshold is
        // rejected; the threshold and above is accepted.
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(0, WIND_DRIFT_CALM_ACCEPT_MIN - 1),
            None
        );
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(0, WIND_DRIFT_CALM_ACCEPT_MIN),
            Some(WindState::Calm)
        );
    }

    #[test]
    fn dungeon_minimap_glyph_matches_published_class_table() {
        // dungeon-mode.md §12: spot-check the published high-nibble
        // class -> minimap glyph map. Blank classes return None.
        // Passage detail glyphs require bit 0x08 set on a 0x0? cell.
        assert_eq!(dungeon_minimap_glyph(0x00), None);
        assert_eq!(dungeon_minimap_glyph(0x08), Some(0x18));
        assert_eq!(dungeon_minimap_glyph(0x10), Some(0x2E)); // Up ladder
        assert_eq!(dungeon_minimap_glyph(0x20), Some(0x2D)); // Down ladder
        assert_eq!(dungeon_minimap_glyph(0x30), Some(0x2F)); // Two-way ladder
        assert_eq!(dungeon_minimap_glyph(0x40), Some(0x70)); // Closed chest
        assert_eq!(dungeon_minimap_glyph(0x50), Some(0x12)); // Fountain anchor
        // Exact 0x6? bytes carve out specific glyphs before band.
        assert_eq!(dungeon_minimap_glyph(0x60), Some(0x19)); // Plain pit
        assert_eq!(dungeon_minimap_glyph(0x61), Some(0x71)); // Hidden/fall pit
        assert_eq!(dungeon_minimap_glyph(0x69), Some(0x71)); // Hidden/fall pit
        assert_eq!(dungeon_minimap_glyph(0x68), Some(0x12)); // Fired pit
        assert_eq!(dungeon_minimap_glyph(0x62), Some(0x72)); // Other 0x6?
        assert_eq!(dungeon_minimap_glyph(0x6F), Some(0x72));
        // Blank classes.
        assert_eq!(dungeon_minimap_glyph(0x70), None);
        assert_eq!(dungeon_minimap_glyph(0x90), None);
        // Wall classes.
        assert_eq!(dungeon_minimap_glyph(0xB0), Some(0x7F));
        assert_eq!(dungeon_minimap_glyph(0xB1), Some(0x74));
        assert_eq!(dungeon_minimap_glyph(0xC0), Some(0x75));
        assert_eq!(dungeon_minimap_glyph(0xD0), Some(0x76));
        assert_eq!(dungeon_minimap_glyph(0xE0), Some(0x77));
        // Door/room families.
        assert_eq!(dungeon_minimap_glyph(0xA0), Some(0x73));
        assert_eq!(dungeon_minimap_glyph(0xF0), Some(0x73));
    }

    #[test]
    fn tlk_ask_party_name_match_returns_1_based_slot_index() {
        // conversation.md §7.6: ASK-PARTY-NAME compares the typed
        // answer to each live member's name with the bit-7-stripping
        // case-insensitive convention. Returns the 1-based slot
        // index on a match; 0 means no match.
        let names: [&[u8]; 4] = [b"Avatar", b"Shamino", b"Iolo", b"Mariah"];
        assert_eq!(tlk_ask_party_name_match(b"Avatar", &names), 1);
        assert_eq!(tlk_ask_party_name_match(b"shamino", &names), 2);
        assert_eq!(tlk_ask_party_name_match(b"IOLO", &names), 3);
        assert_eq!(tlk_ask_party_name_match(b"Mariah", &names), 4);
        // No match returns 0.
        assert_eq!(tlk_ask_party_name_match(b"Dupre", &names), 0);
        assert_eq!(tlk_ask_party_name_match(b"", &names), 0);
        // Partial prefixes don't match (whole-string equality).
        assert_eq!(tlk_ask_party_name_match(b"Shamin", &names), 0);
        // High-bit-encoded name on the engine side matches a plain
        // typed answer (the typed side is bit-7-stripped too).
        let obfuscated: [u8; 4] = [b'I' | 0x80, b'o' | 0x80, b'l' | 0x80, b'o' | 0x80];
        let obfuscated_names: [&[u8]; 1] = [obfuscated.as_slice()];
        assert_eq!(tlk_ask_party_name_match(b"iolo", &obfuscated_names), 1);
        // Empty roster slots are skipped.
        let with_blank: [&[u8]; 3] = [b"", b"Geoffrey", b""];
        assert_eq!(tlk_ask_party_name_match(b"Geoffrey", &with_blank), 2);
    }

    #[test]
    fn dos_eof_marker_byte_is_ctrl_z() {
        // formats/look2-dat.md §6 (and other format specs) refer to
        // the legacy DOS end-of-file marker 0x1A (Ctrl-Z) as a byte
        // readers ignore when computing meaningful payload length.
        assert_eq!(DOS_EOF_MARKER, 0x1A);
        assert_eq!(DOS_EOF_MARKER, b'\x1A');
    }

    #[test]
    fn foot_avatar_transport_marker_default_and_band_match_spec() {
        // vehicles.md §2: clean-seed foot/avatar transport marker is
        // 0x1C (facing north); the foot family band is 0x1C..=0x1F.
        assert_eq!(TRANSPORT_MARKER_FOOT_DEFAULT, 0x1C);
        assert_eq!(TRANSPORT_MARKER_FOOT_FIRST, 0x1C);
        assert_eq!(TRANSPORT_MARKER_FOOT_LAST, 0x1F);
        // Each byte in the foot band classifies as the foot family.
        for marker in TRANSPORT_MARKER_FOOT_FIRST..=TRANSPORT_MARKER_FOOT_LAST {
            assert_eq!(
                transport_family(marker),
                Some(TransportFamily::Foot),
                "marker {marker:#04x}"
            );
        }
        // The byte just above the band leaves the foot family.
        assert_ne!(transport_family(0x20), Some(TransportFamily::Foot));
    }

    #[test]
    fn encounter_spawn_separation_predicate_matches_published_band() {
        // encounters.md §4: candidate spawn coordinates must be
        // strictly between separations 6 and 250 on both axes. The
        // retry budget is 128 candidates before silent giveup.
        assert_eq!(ENCOUNTER_SPAWN_MIN_SEPARATION, 6);
        assert_eq!(ENCOUNTER_SPAWN_MAX_SEPARATION, 250);
        assert_eq!(ENCOUNTER_SPAWN_RETRY_BUDGET, 128);

        assert!(encounter_spawn_separation_accepts(7, 7));
        assert!(encounter_spawn_separation_accepts(100, 100));
        assert!(encounter_spawn_separation_accepts(249, 249));
        // Equal-to-bounds rejected.
        assert!(!encounter_spawn_separation_accepts(6, 7));
        assert!(!encounter_spawn_separation_accepts(7, 6));
        assert!(!encounter_spawn_separation_accepts(250, 7));
        assert!(!encounter_spawn_separation_accepts(7, 250));
        // Either axis below or above bounds rejected.
        assert!(!encounter_spawn_separation_accepts(0, 100));
        assert!(!encounter_spawn_separation_accepts(5, 50));
        assert!(!encounter_spawn_separation_accepts(255, 100));
    }

    #[test]
    fn shop_refuses_mounted_horse_except_at_horse_trader() {
        // shops.md §2: ordinary shop arms refuse before opening
        // their menu when the party is mounted on a horse; only the
        // horse-trader dialog id (0x83) is reachable on horseback.
        assert_eq!(HORSE_TRADER_DIALOG_ID, 0x83);
        // On foot: nothing refuses.
        for dialog in 0x81u8..=0x88 {
            assert!(!shop_refuses_mounted_horse(dialog, false));
        }
        // Mounted: every shop except 0x83 refuses.
        for dialog in 0x81u8..=0x88 {
            let expected_refusal = dialog != HORSE_TRADER_DIALOG_ID;
            assert_eq!(
                shop_refuses_mounted_horse(dialog, true),
                expected_refusal,
                "dialog {dialog:#04x}"
            );
        }
    }

    #[test]
    fn stationary_display_prompt_classifies_y_n_space_and_discards_others() {
        // shops.md §8.10: the stationary-display purchase Y/N loop
        // accepts Y (offer), N or Space (exit), and silently
        // re-prompts on any other byte.
        use StationaryDisplayPrompt::*;
        assert_eq!(stationary_display_prompt(b'Y'), Offer);
        assert_eq!(stationary_display_prompt(b'y'), Offer);
        assert_eq!(stationary_display_prompt(b'N'), Exit);
        assert_eq!(stationary_display_prompt(b'n'), Exit);
        assert_eq!(stationary_display_prompt(b' '), Exit);
        // Discard everything else (numbers, other letters, controls).
        assert_eq!(stationary_display_prompt(0), Discard);
        assert_eq!(stationary_display_prompt(b'\r'), Discard);
        assert_eq!(stationary_display_prompt(b'A'), Discard);
        assert_eq!(stationary_display_prompt(b'5'), Discard);
    }

    #[test]
    fn common_word_dictionary_nul_sentinel_count_matches_spec() {
        // shops.md §4.2: the shared 128-entry pointer dictionary
        // includes 11 NUL pointers used as word-boundary sentinels.
        // The SHOPPE.DAT phrase-token byte range is 0x80..=0xFF.
        assert_eq!(COMMON_WORD_DICTIONARY_ENTRIES, 128);
        assert_eq!(COMMON_WORD_DICTIONARY_NUL_SENTINELS, 11);
        assert_eq!(SHOPPE_PHRASE_TOKEN_FIRST, 0x80);
        assert_eq!(SHOPPE_PHRASE_TOKEN_LAST, 0xFF);
        // The phrase-token range covers exactly the 128 dictionary
        // entries.
        assert_eq!(
            (SHOPPE_PHRASE_TOKEN_LAST as usize - SHOPPE_PHRASE_TOKEN_FIRST as usize + 1),
            COMMON_WORD_DICTIONARY_ENTRIES
        );
    }

    #[test]
    fn healer_unmatched_row_fees_match_published_table() {
        // shops.md §8.3: the resident cost table has an eighth row
        // (Cure 1, Heal 70, Resurrect 270) that no shipped healer
        // scene maps to. Preserve the published values so custom
        // scenes that point at row 7 still get the engine fees.
        assert_eq!(HEALER_UNMATCHED_ROW_CURE_FEE, 1);
        assert_eq!(HEALER_UNMATCHED_ROW_HEAL_FEE, 70);
        assert_eq!(HEALER_UNMATCHED_ROW_RESURRECT_FEE, 270);
    }

    #[test]
    fn shop_affordability_refusal_barks_match_spec_text() {
        // shops.md §6.1: per-kind refusal lines printed when the
        // affordability gate rejects a purchase. Bark text is
        // presentation only; gold and inventory are unchanged.
        assert_eq!(TAVERN_AFFORDABILITY_REFUSAL_BARK, "Beat it!");
        assert_eq!(UPMARKET_INN_AFFORDABILITY_REFUSAL_BARK, "Highwaymen!");
        assert_eq!(
            VEHICLE_BROKER_PARTIAL_AFFORD_PREFIX,
            "Thou canst afford only "
        );
    }

    #[test]
    fn sage_topic_matches_uses_space_boundary_prefix_per_spec() {
        // shops.md §8.8: the sage uses case-insensitive prefix
        // matching with a strict boundary: the byte after the
        // matched topic must be end-of-input or a space.
        assert_eq!(SAGE_KEYWORD_INPUT_LIMIT, 15);

        // Exact match.
        assert!(sage_topic_matches("LORD", "LORD"));
        // Case-insensitive.
        assert!(sage_topic_matches("lord", "LORD"));
        assert!(sage_topic_matches("Lord", "lord"));
        // Topic followed by space.
        assert!(sage_topic_matches("LORD BRITISH", "LORD"));
        // Longer typed word without a space boundary is rejected.
        assert!(!sage_topic_matches("LORDS", "LORD"));
        assert!(!sage_topic_matches("lordly", "lord"));
        // Typed string too short can't match.
        assert!(!sage_topic_matches("LOR", "LORD"));
        // Empty topic never matches.
        assert!(!sage_topic_matches("LORD", ""));
        // Empty typed input never matches a non-empty topic.
        assert!(!sage_topic_matches("", "LORD"));
    }

    #[test]
    fn inn_per_instance_base_rate_and_gold_gate_match_published_table() {
        // shops.md §8.4 stock inn rate rows. Spot-check every inn
        // against the published base room rate and minimum-gold gate.
        let cases = [
            (Inn::TheWayfarerInn, 2u16, 3u16),
            (Inn::TheWarriorsStead, 3, 4),
            (Inn::TheHauntingInn, 2, 3),
            (Inn::HotelBrittany, 3, 2),
            (Inn::TheSmugglersInn, 2, 2),
            (Inn::TheKingsRansomInn, 3, 2),
        ];
        for (inn, base, gate) in cases {
            assert_eq!(inn.base_room_rate(), base, "{:?} base", inn);
            assert_eq!(inn.minimum_gold_gate(), gate, "{:?} gate", inn);
        }
    }

    #[test]
    fn frigate_initial_starting_state_matches_published_table() {
        // vehicles.md §4: a shipwright-purchased Frigate starts at
        // hull condition 100 with two skiffs aboard when it is
        // placed at the stored sale coordinates on the next
        // overworld entry.
        assert_eq!(FRIGATE_INITIAL_HULL_CONDITION, 100);
        assert_eq!(FRIGATE_INITIAL_SKIFFS, 2);
        // The starting hull is well above the low-hull warning
        // threshold, so a freshly purchased Frigate never prints
        // the low-hull warning on its first board.
        assert!(!ship_boarding_warns_low_hull(FRIGATE_INITIAL_HULL_CONDITION));
    }

    #[test]
    fn town_entry_jail_wakeup_predicate_matches_published_coordinate() {
        // town-mode.md §5: surrendering at a guard-catch event sends
        // the party to Yew (scene 4) floor 0 cell (25, 4). The
        // setup pass skips the permanent-location queue lookup when
        // town entry hits scene 4 / floor 0 / Y == 4.
        assert_eq!(TOWN_ARREST_JAIL_SCENE, 4);
        assert_eq!(TOWN_ARREST_JAIL_FLOOR, 0);
        assert_eq!(TOWN_ARREST_JAIL_X, 25);
        assert_eq!(TOWN_ARREST_JAIL_Y, 4);
        assert!(town_entry_is_jail_wakeup(
            TOWN_ARREST_JAIL_SCENE,
            TOWN_ARREST_JAIL_FLOOR,
            TOWN_ARREST_JAIL_Y
        ));
        assert!(!town_entry_is_jail_wakeup(5, 0, 4));
        assert!(!town_entry_is_jail_wakeup(4, 0, 5));
        assert!(!town_entry_is_jail_wakeup(4, 1, 4));
    }

    #[test]
    fn tile_glyph_digraph_classifies_published_byte_codes() {
        // formats/miscmsg-dat.md §4: Codex/prophecy tile-glyph
        // records use `@` for inter-word space, `[` for TH, `]` for
        // NG, `_` for ER. Other bytes are ordinary text glyphs.
        use TileGlyphDigraph::*;
        assert_eq!(tile_glyph_digraph(b'@'), Some(InterWordSpace));
        assert_eq!(tile_glyph_digraph(b'['), Some(Th));
        assert_eq!(tile_glyph_digraph(b']'), Some(Ng));
        assert_eq!(tile_glyph_digraph(b'_'), Some(Er));
        assert_eq!(InterWordSpace.expansion(), " ");
        assert_eq!(Th.expansion(), "TH");
        assert_eq!(Ng.expansion(), "NG");
        assert_eq!(Er.expansion(), "ER");
        // Ordinary ASCII letters and punctuation are not digraphs.
        assert_eq!(tile_glyph_digraph(b'A'), None);
        assert_eq!(tile_glyph_digraph(b' '), None);
        assert_eq!(tile_glyph_digraph(b'.'), None);
        assert_eq!(tile_glyph_digraph(0), None);
    }

    #[test]
    fn npc_schedule_hour_at_boundary_matches_published_equality_rule() {
        // npc-schedules.md §6: the boundary trigger fires only when
        // the current hour exactly matches one of the four time
        // boundary bytes. Hours strictly between boundaries leave
        // the NPC idle.
        let time = [6u8, 9, 17, 21];
        for hour in 0u8..24 {
            let expected = hour == 6 || hour == 9 || hour == 17 || hour == 21;
            assert_eq!(
                npc_schedule_hour_at_boundary(time, hour),
                expected,
                "hour {hour}"
            );
        }
        // Boundary bytes modulo 24 — out-of-range bytes still fire
        // when the wrapped value matches the wrapped hour.
        assert!(npc_schedule_hour_at_boundary([24, 24, 24, 24], 0));
        assert!(npc_schedule_hour_at_boundary([26, 0, 0, 0], 2));
    }

    #[test]
    fn npc_dynamic_obstacle_blocks_within_radius_strictly() {
        // npc-schedules.md §10: occupants strictly inside the
        // dynamic-obstacle radius (Manhattan distance < 4) are
        // reported as blocked; occupants at or beyond the radius
        // are treated as walkable by the pathfinding workspace.
        let dest_x = 16;
        let dest_y = 12;
        // Same cell: distance 0 -> blocked.
        assert!(npc_dynamic_obstacle_blocks(dest_x, dest_y, dest_x, dest_y));
        // Manhattan 3 < 4 -> blocked.
        assert!(npc_dynamic_obstacle_blocks(dest_x + 1, dest_y + 2, dest_x, dest_y));
        assert!(npc_dynamic_obstacle_blocks(dest_x - 3, dest_y, dest_x, dest_y));
        // Manhattan 4 -> not blocked (boundary is exclusive).
        assert!(!npc_dynamic_obstacle_blocks(dest_x + 4, dest_y, dest_x, dest_y));
        assert!(!npc_dynamic_obstacle_blocks(dest_x + 2, dest_y - 2, dest_x, dest_y));
        // Manhattan 5+: well outside.
        assert!(!npc_dynamic_obstacle_blocks(dest_x + 5, dest_y, dest_x, dest_y));
        assert!(!npc_dynamic_obstacle_blocks(0, 0, dest_x, dest_y));
    }

    #[test]
    fn npc_schedule_state_probe_and_step_groups_match_spec() {
        // npc-schedules.md §7: the five "probe and step" states are
        // InPlaneMove, DescendTowardTarget, AscendTowardTarget,
        // ClimbUpOffFloor, and ClimbDownOffFloor. ReplayQueue is the
        // only other state that produces a visible step (by popping
        // a queued direction).
        use NpcScheduleState::*;
        for state in [
            InPlaneMove,
            DescendTowardTarget,
            AscendTowardTarget,
            ClimbUpOffFloor,
            ClimbDownOffFloor,
        ] {
            assert!(state.is_probe_and_step(), "{state:?}");
            assert!(state.produces_visible_step(), "{state:?}");
        }
        for state in [Empty, Idle, ParkedOffFloor] {
            assert!(!state.is_probe_and_step(), "{state:?}");
            assert!(!state.produces_visible_step(), "{state:?}");
        }
        // ReplayQueue produces visible steps but is not probe-and-step.
        assert!(!ReplayQueue.is_probe_and_step());
        assert!(ReplayQueue.produces_visible_step());
    }

    #[test]
    fn npc_ai_behavior_event_predicates_match_published_table() {
        // npc-schedules.md §9: ApproachAndAttack, ReservedEngage,
        // and RandomChase raise the town-mode attack event when
        // adjacent. GuardOrBlock raises the non-attack guard event
        // instead. BoundedWander and UnboundedWander are the two
        // random-wander behaviours.
        for behavior in [
            NpcAiBehavior::ApproachAndAttack,
            NpcAiBehavior::ReservedEngage,
            NpcAiBehavior::RandomChase,
        ] {
            assert!(behavior.raises_attack_event(), "{behavior:?}");
            assert!(!behavior.raises_guard_event(), "{behavior:?}");
        }
        assert!(NpcAiBehavior::GuardOrBlock.raises_guard_event());
        assert!(!NpcAiBehavior::GuardOrBlock.raises_attack_event());
        for behavior in [
            NpcAiBehavior::Stationary,
            NpcAiBehavior::FollowAtDistance,
            NpcAiBehavior::BoundedWander,
            NpcAiBehavior::UnboundedWander,
        ] {
            assert!(!behavior.raises_attack_event(), "{behavior:?}");
            assert!(!behavior.raises_guard_event(), "{behavior:?}");
        }
        // Wander predicate covers the two random-wander variants.
        assert!(NpcAiBehavior::BoundedWander.is_wander());
        assert!(NpcAiBehavior::UnboundedWander.is_wander());
        for behavior in [
            NpcAiBehavior::Stationary,
            NpcAiBehavior::FollowAtDistance,
            NpcAiBehavior::ApproachAndAttack,
            NpcAiBehavior::ReservedEngage,
            NpcAiBehavior::GuardOrBlock,
            NpcAiBehavior::RandomChase,
        ] {
            assert!(!behavior.is_wander(), "{behavior:?}");
        }
    }

    #[test]
    fn npc_stuck_counter_forces_replan_at_strict_threshold_exceed() {
        // npc-schedules.md §4: the walker resets the move queue when
        // the stuck counter *strictly* exceeds the published
        // threshold (3). Equal-or-below leaves it alone.
        assert_eq!(NPC_STUCK_REPLAN_THRESHOLD, 3);
        for counter in 0u16..=NPC_STUCK_REPLAN_THRESHOLD {
            assert!(!npc_stuck_counter_forces_replan(counter));
        }
        assert!(npc_stuck_counter_forces_replan(NPC_STUCK_REPLAN_THRESHOLD + 1));
        assert!(npc_stuck_counter_forces_replan(100));
        assert!(npc_stuck_counter_forces_replan(u16::MAX));
    }

    #[test]
    fn dungeon_movement_action_maps_published_numpad_directions() {
        // dungeon-mode.md §9: numpad/arrow direction codes route to
        // Forward (North), Back (South), TurnLeft (West),
        // TurnRight (East); diagonals and other values fall through
        // to TurnAround.
        use DungeonMovementAction::*;
        assert_eq!(dungeon_movement_action(InputDirection::North), Forward);
        assert_eq!(dungeon_movement_action(InputDirection::South), Back);
        assert_eq!(dungeon_movement_action(InputDirection::West), TurnLeft);
        assert_eq!(dungeon_movement_action(InputDirection::East), TurnRight);
        for diag in [
            InputDirection::Northwest,
            InputDirection::Northeast,
            InputDirection::Southwest,
            InputDirection::Southeast,
        ] {
            assert_eq!(dungeon_movement_action(diag), TurnAround);
        }
    }

    #[test]
    fn dungeon_renderer_paints_wall_cue_for_published_classes() {
        // dungeon-mode.md §6: the first-person renderer paints a
        // wall cue for cells whose high nibble identifies a wall
        // class (0xB..=0xE) or the 0xF? heavy-door / room-trigger
        // family. Open passages and other low classes paint as void.
        for tile in 0x00u8..=0xAF {
            assert!(
                !dungeon_renderer_paints_wall_cue(tile),
                "tile {tile:#04x} should not paint wall cue"
            );
        }
        for tile in 0xB0u8..=0xFF {
            assert!(
                dungeon_renderer_paints_wall_cue(tile),
                "tile {tile:#04x} should paint wall cue"
            );
        }
    }

    #[test]
    fn dungeon_bomb_search_outcome_dispatches_strictly_above_threshold() {
        // dungeon-mode.md §8: Search-on-0x62 rolls 1..=30 against
        // the shared dungeon-chest threshold. Roll above threshold
        // springs the bomb; equal-or-below leaves the cell alone.
        use DungeonBombSearchOutcome::*;
        // At threshold 15: rolls 16..=30 spring; 1..=15 don't.
        for roll in 1u8..=15 {
            assert_eq!(dungeon_bomb_search_outcome(15, roll), NothingOnPit);
        }
        for roll in 16u8..=30 {
            assert_eq!(dungeon_bomb_search_outcome(15, roll), SpringBomb);
        }
        // Threshold 30 means no roll can spring the bomb.
        for roll in 1u8..=30 {
            assert_eq!(dungeon_bomb_search_outcome(30, roll), NothingOnPit);
        }
        // Threshold 0 means every nonzero roll springs the bomb.
        for roll in 1u8..=30 {
            assert_eq!(dungeon_bomb_search_outcome(0, roll), SpringBomb);
        }
    }

    #[test]
    fn dungeon_chest_search_outcome_dispatches_per_spec() {
        // dungeon-mode.md §8: Search uses the shared dungeon-chest
        // threshold. No-trap branch requires roll > threshold AND
        // the chest byte to be plain (unmarked). Otherwise tier
        // comes from the fresh roll on unmarked chests or the
        // current depth Z on already-marked chests.
        use DungeonChestSearchOutcome::*;
        use DungeonChestTrapTier::*;

        // Plain chest, roll above threshold -> no trap.
        assert_eq!(
            dungeon_chest_search_outcome(15, 20, 7, 3, false),
            NoTrap
        );
        // Plain chest, roll at threshold -> trap branch with fresh tier.
        // fresh_tier=2 -> Simple (< 4).
        assert_eq!(
            dungeon_chest_search_outcome(15, 15, 2, 3, false),
            Trap(Simple)
        );
        // Plain chest, roll below threshold -> trap branch with fresh
        // tier 7 -> Complex (>= 7).
        assert_eq!(
            dungeon_chest_search_outcome(15, 10, 7, 3, false),
            Trap(Complex)
        );
        // Plain chest, fresh tier 5 -> Generic (middle band).
        assert_eq!(
            dungeon_chest_search_outcome(15, 10, 5, 3, false),
            Trap(Generic)
        );
        // Marked chest, any roll -> tier comes from depth Z, not roll.
        // Depth 0 -> Simple; depth 7 -> Complex; depth 5 -> Generic.
        assert_eq!(
            dungeon_chest_search_outcome(15, 30, 1, 0, true),
            Trap(Simple)
        );
        assert_eq!(
            dungeon_chest_search_outcome(15, 30, 1, 7, true),
            Trap(Complex)
        );
        assert_eq!(
            dungeon_chest_search_outcome(15, 30, 1, 5, true),
            Trap(Generic)
        );
        // Marked chest never takes the no-trap branch, even when
        // roll > threshold.
        assert_eq!(
            dungeon_chest_search_outcome(15, 30, 1, 3, true),
            Trap(Simple)
        );
    }

    #[test]
    fn dungeon_field_base_byte_pairs_effect_with_published_byte() {
        // dungeon-mode.md §8: the four energy fields have published
        // base bytes (0x80..=0x83) with paired visit-marker variants
        // (0x88..=0x8B). The generic Energy band has no single base.
        assert_eq!(DUNGEON_FIELD_SLEEP_BASE, 0x80);
        assert_eq!(DUNGEON_FIELD_POISON_GAS_BASE, 0x81);
        assert_eq!(DUNGEON_FIELD_FIRE_BASE, 0x82);
        assert_eq!(DUNGEON_FIELD_ELECTRIC_BASE, 0x83);
        assert_eq!(
            dungeon_field_base_byte(DungeonFieldEffect::Sleep),
            Some(DUNGEON_FIELD_SLEEP_BASE)
        );
        assert_eq!(
            dungeon_field_base_byte(DungeonFieldEffect::PoisonGas),
            Some(DUNGEON_FIELD_POISON_GAS_BASE)
        );
        assert_eq!(
            dungeon_field_base_byte(DungeonFieldEffect::Fire),
            Some(DUNGEON_FIELD_FIRE_BASE)
        );
        assert_eq!(
            dungeon_field_base_byte(DungeonFieldEffect::Electric),
            Some(DUNGEON_FIELD_ELECTRIC_BASE)
        );
        assert_eq!(
            dungeon_field_base_byte(DungeonFieldEffect::Energy),
            None
        );
        // The visit-marker variants round-trip through the existing
        // classifier: base | 0x08 reclassifies to the same effect.
        for base in [
            DUNGEON_FIELD_SLEEP_BASE,
            DUNGEON_FIELD_POISON_GAS_BASE,
            DUNGEON_FIELD_FIRE_BASE,
            DUNGEON_FIELD_ELECTRIC_BASE,
        ] {
            assert_eq!(
                dungeon_field_effect(base),
                dungeon_field_effect(base | DUNGEON_VISIT_MARKER_BIT)
            );
        }
    }

    #[test]
    fn dungeon_fountain_effect_classifies_subtype_per_spec() {
        // dungeon-mode.md §8: fountain cells use the high nibble 0x5
        // and the low nibble selects the drink effect:
        // 0 = Cure, 1 = Heal, 2 = Poison, 3+ = Bad taste.
        use DungeonFountainEffect::*;
        assert_eq!(dungeon_fountain_effect(0x50), Some(Cure));
        assert_eq!(dungeon_fountain_effect(0x51), Some(Heal));
        assert_eq!(dungeon_fountain_effect(0x52), Some(Poison));
        for low in 3u8..=0x0F {
            assert_eq!(dungeon_fountain_effect(0x50 | low), Some(BadTaste));
        }
        // Non-fountain cells return None.
        assert_eq!(dungeon_fountain_effect(0x00), None);
        assert_eq!(dungeon_fountain_effect(0x40), None);
        assert_eq!(dungeon_fountain_effect(0x60), None);
        assert_eq!(dungeon_fountain_effect(0x80), None);
        assert_eq!(DUNGEON_FOUNTAIN_BAD_TASTE_DAMAGE_MAX, 7);
    }

    #[test]
    fn dungeon_klimb_apply_dispatches_by_high_nibble_and_plain_pit() {
        // dungeon-mode.md §13: K-Klimb reads the underfoot byte's
        // high nibble (and the exact 0x60 plain-pit byte) to decide
        // whether to move Z, prompt up/down, fire the surface-reset
        // helper, or refuse the climb.
        use DungeonKlimbApply::*;
        // 0x60 is the surface-reset pit (must be checked before pit
        // family classifies it as PlainPit).
        assert_eq!(dungeon_klimb_apply(0x60), SurfaceResetPit);
        // Up ladders.
        for tile in 0x10u8..=0x1F {
            assert_eq!(dungeon_klimb_apply(tile), UpLadder);
        }
        // Down ladders.
        for tile in 0x20u8..=0x2F {
            assert_eq!(dungeon_klimb_apply(tile), DownLadder);
        }
        // Two-way ladders.
        for tile in 0x30u8..=0x3F {
            assert_eq!(dungeon_klimb_apply(tile), TwoWayPrompt);
        }
        // Other 0x6? bytes (besides 0x60) are NoLevelChange — K-Klimb
        // routes pit/bomb/secret traps elsewhere, not into the ladder
        // apply path.
        for tile in [0x61u8, 0x69, 0x62, 0x6A, 0x67, 0x6F] {
            assert_eq!(dungeon_klimb_apply(tile), NoLevelChange);
        }
        // Wall/door classes and ordinary passages: no level change.
        for tile in [0x00u8, 0x05, 0x80, 0x90, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0] {
            assert_eq!(dungeon_klimb_apply(tile), NoLevelChange);
        }
    }

    #[test]
    fn dungeon_entry_seed_constants_match_published_coordinates() {
        // dungeon-mode.md §3: surface entry → (Z=0, X=1, Y=1) east;
        // underworld entry into non-Doom dungeons → (Z=7, X=7, Y=7)
        // west; Doom (scene 40) uses the surface seed even when
        // reached from the underworld.
        assert_eq!(DUNGEON_ENTRY_SURFACE_Z, 0);
        assert_eq!(DUNGEON_ENTRY_SURFACE_X, 1);
        assert_eq!(DUNGEON_ENTRY_SURFACE_Y, 1);
        assert_eq!(DUNGEON_ENTRY_UNDERWORLD_Z, 7);
        assert_eq!(DUNGEON_ENTRY_UNDERWORLD_X, 7);
        assert_eq!(DUNGEON_ENTRY_UNDERWORLD_Y, 7);
        assert_eq!(DUNGEON_DOOM_SCENE_BYTE, 40);
        // Spot check Doom's exception against the existing helper.
        let doom_from_under =
            dungeon_entry_seed(DUNGEON_DOOM_SCENE_BYTE, true).unwrap();
        assert_eq!(doom_from_under.z, DUNGEON_ENTRY_SURFACE_Z);
        assert_eq!(doom_from_under.x, DUNGEON_ENTRY_SURFACE_X);
        assert_eq!(doom_from_under.y, DUNGEON_ENTRY_SURFACE_Y);
    }

    #[test]
    fn chest_primary_and_secondary_pool_thresholds_match_published_table() {
        // containers.md §4: primary pool has 8 published rows with
        // fixed thresholds; secondary pool has 48 rows with several
        // "Disabled" sentinel entries that never succeed.
        assert_eq!(CHEST_PRIMARY_POOL_ROW_COUNT, 8);
        assert_eq!(
            CHEST_PRIMARY_POOL_THRESHOLDS,
            [7, 7, 15, 9, 17, 17, 3, 25]
        );
        assert_eq!(CHEST_SECONDARY_POOL_ROW_COUNT, 48);

        // Spot checks against the published 48-row table.
        assert_eq!(chest_secondary_pool_threshold(0), Some(10)); // Helm
        assert_eq!(chest_secondary_pool_threshold(7), Some(28)); // Spiked Shield
        assert_eq!(chest_secondary_pool_threshold(8), None); // Shield disabled
        assert_eq!(chest_secondary_pool_threshold(15), None); // Armour disabled
        assert_eq!(chest_secondary_pool_threshold(16), Some(5)); // Dagger
        assert_eq!(chest_secondary_pool_threshold(35), None); // Weapon disabled
        assert_eq!(chest_secondary_pool_threshold(36), Some(23));
        assert_eq!(chest_secondary_pool_threshold(42), Some(23));
        assert_eq!(chest_secondary_pool_threshold(46), Some(15));
        assert_eq!(chest_secondary_pool_threshold(47), None); // Amulet disabled
        // Out-of-range indices.
        assert_eq!(chest_secondary_pool_threshold(48), None);
        assert_eq!(chest_secondary_pool_threshold(usize::MAX), None);
    }

    #[test]
    fn town_chest_open_standing_debits_two_floored_at_zero() {
        // containers.md §4: opening a town-family object-table chest
        // reduces the shared moral-standing selector by two units,
        // clamped at zero. Overworld chests don't apply this penalty
        // (caller-gated).
        assert_eq!(TOWN_CHEST_OPEN_KARMA_DEBIT, 2);
        assert_eq!(town_chest_open_standing(0), 0);
        assert_eq!(town_chest_open_standing(1), 0);
        assert_eq!(town_chest_open_standing(2), 0);
        assert_eq!(town_chest_open_standing(3), 1);
        assert_eq!(town_chest_open_standing(50), 48);
        assert_eq!(town_chest_open_standing(99), 97);
        assert_eq!(town_chest_open_standing(u8::MAX), 253);
    }

    #[test]
    fn combat_field_kind_constants_match_published_spell_helper_table() {
        // catalogs/spell-list.md §6: the combat-side field helper
        // consumes one of four kind bytes per field spell —
        // Poison 0x33, Sleep 0x34, Fire 0x35, Energy 0x36 —
        // separate from the dungeon image bytes 0x80..0x83.
        assert_eq!(COMBAT_FIELD_KIND_POISON, 0x33);
        assert_eq!(COMBAT_FIELD_KIND_SLEEP, 0x34);
        assert_eq!(COMBAT_FIELD_KIND_FIRE, 0x35);
        assert_eq!(COMBAT_FIELD_KIND_ENERGY, 0x36);
        assert_eq!(
            spell_combat_field_kind(FIRE_FIELD_SPELL_INDEX),
            Some(COMBAT_FIELD_KIND_FIRE)
        );
        assert_eq!(
            spell_combat_field_kind(POISON_FIELD_SPELL_INDEX),
            Some(COMBAT_FIELD_KIND_POISON)
        );
        assert_eq!(
            spell_combat_field_kind(SLEEP_FIELD_SPELL_INDEX),
            Some(COMBAT_FIELD_KIND_SLEEP)
        );
        assert_eq!(
            spell_combat_field_kind(ENERGY_FIELD_SPELL_INDEX),
            Some(COMBAT_FIELD_KIND_ENERGY)
        );
        // Non-field spells have no combat kind byte.
        assert_eq!(spell_combat_field_kind(0), None);
        assert_eq!(spell_combat_field_kind(DISPEL_FIELD_SPELL_INDEX), None);
    }

    #[test]
    fn negate_time_spell_install_counter_matches_time_stop_duration() {
        // catalogs/spell-list.md §6: Negate Time (An Tym) starts the
        // shared `T` runtime tag with a 10-count countdown.
        // Confirm the install counter and the byte-classifier agree.
        assert_eq!(TIME_STOP_DURATION, 10);
        assert_eq!(
            ActiveEffectTag::NegateTime.spell_install_counter(),
            Some(TIME_STOP_DURATION)
        );
        assert_eq!(
            ActiveEffectTag::NegateTime.ascii_byte(),
            NEGATE_TIME_ACTIVE_EFFECT_TAG
        );
        assert_eq!(
            active_effect_tag_for_byte(NEGATE_TIME_ACTIVE_EFFECT_TAG),
            Some(ActiveEffectTag::NegateTime)
        );
    }

    #[test]
    fn spell_rune_name_lookup_matches_published_table() {
        // catalogs/spell-list.md §5: spot checks across the
        // eight-circle spell table. Every entry should have a
        // rune-name; out-of-range indices return None.
        assert_eq!(spell_rune_name(0), Some("In Lor"));
        assert_eq!(spell_rune_name(1), Some("Grav Por"));
        assert_eq!(spell_rune_name(4), Some("Mani"));
        assert_eq!(spell_rune_name(12), Some("Vas Lor"));
        assert_eq!(spell_rune_name(19), Some("In Sanct"));
        assert_eq!(spell_rune_name(30), Some("In Vas Por Ylem"));
        assert_eq!(spell_rune_name(37), Some("Xen Corp"));
        assert_eq!(spell_rune_name(42), Some("In Mani Corp"));
        assert_eq!(spell_rune_name(46), Some("Vas Rel Por"));
        assert_eq!(spell_rune_name(47), Some("An Tym"));
        // Every published spell id has a rune-name.
        for index in 0..SPELL_COUNT {
            assert!(spell_rune_name(index).is_some(), "spell {index}");
        }
        assert_eq!(spell_rune_name(SPELL_COUNT), None);
    }

    #[test]
    fn rare_reagent_harvest_quantity_stays_in_2_to_15_range() {
        // containers.md §5: a successful midnight harvest rolls
        // 2..=15 sprigs of the matching reagent.
        assert_eq!(RARE_REAGENT_HARVEST_QUANTITY_MIN, 2);
        assert_eq!(RARE_REAGENT_HARVEST_QUANTITY_MAX, 15);
        for seed in 0u8..=255 {
            let q = rare_reagent_harvest_quantity(seed);
            assert!(
                (RARE_REAGENT_HARVEST_QUANTITY_MIN..=RARE_REAGENT_HARVEST_QUANTITY_MAX)
                    .contains(&q),
                "seed {seed} produced {q}"
            );
        }
        // Hour gate accepts only midnight.
        assert!(rare_reagent_harvest_hour_accepted(0));
        for hour in 1u8..=23 {
            assert!(!rare_reagent_harvest_hour_accepted(hour));
        }
    }

    #[test]
    fn dungeon_chest_reward_ranges_match_published_table() {
        // containers.md §6: per-row quantity / subtype ranges.
        // Food rolls 1..=31, Keys/Gems/Torches roll 1..=3, potion
        // and scroll subtypes roll 0..=7.
        assert_eq!(DUNGEON_CHEST_FOOD_MAX, 31);
        assert_eq!(DUNGEON_CHEST_SMALL_MAX, 3);
        assert_eq!(DUNGEON_CHEST_SUBTYPE_MAX, 7);
    }

    #[test]
    fn inventory_add_equipment_units_grants_ammo_five_others_one() {
        // containers.md §8: equipment-class inventory adds grant
        // one unit per award except for Arrows (id 27) and Quarrels
        // (id 29), which grant five units per award.
        assert_eq!(INVENTORY_ADD_AMMO_UNITS, 5);
        assert_eq!(INVENTORY_ADD_EQUIPMENT_UNITS, 1);
        assert_eq!(
            inventory_add_equipment_units(EQUIPMENT_ID_ARROWS),
            INVENTORY_ADD_AMMO_UNITS
        );
        assert_eq!(
            inventory_add_equipment_units(EQUIPMENT_ID_QUARRELS),
            INVENTORY_ADD_AMMO_UNITS
        );
        // Spot checks: other equipment ids grant one.
        for id in [
            0, 1, 10, 14, 16, 23, 26, 28, 30, 33, 41, 42, 43, 45, 47,
        ] {
            assert_eq!(
                inventory_add_equipment_units(id),
                INVENTORY_ADD_EQUIPMENT_UNITS
            );
        }
    }

    #[test]
    fn proportional_renderer_byte_kind_classifies_published_markers() {
        // text-output.md §8: the proportional renderer consumes
        // text byte-by-byte until NUL. ' ' is a word-break, '\n'
        // is a hard newline, '_' is a soft break, '{' is a
        // paragraph marker, '\0' ends the record.
        use ProportionalRendererByteKind::*;
        assert_eq!(proportional_renderer_byte_kind(0), EndOfRecord);
        assert_eq!(proportional_renderer_byte_kind(b' '), WordBreakSpace);
        assert_eq!(proportional_renderer_byte_kind(b'\n'), HardNewline);
        assert_eq!(proportional_renderer_byte_kind(b'_'), SoftBreak);
        assert_eq!(proportional_renderer_byte_kind(b'{'), ParagraphStart);
        // Visible glyphs round-trip through Glyph.
        assert_eq!(proportional_renderer_byte_kind(b'A'), Glyph(b'A'));
        assert_eq!(proportional_renderer_byte_kind(b'z'), Glyph(b'z'));
        assert_eq!(proportional_renderer_byte_kind(b'.'), Glyph(b'.'));
        assert_eq!(proportional_renderer_byte_kind(b'9'), Glyph(b'9'));
        // `{` is renderer-only — STORY.DAT marker `}` is not a
        // proportional renderer marker.
        assert_eq!(proportional_renderer_byte_kind(b'}'), Glyph(b'}'));
    }

    #[test]
    fn random_encounter_probe_fires_strictly_above_draw() {
        // overworld.md §7 / encounters.md §3: the encounter probe
        // rolls 1..=30 and fires when the threshold is nonzero and
        // strictly greater than the draw. Zero thresholds never fire.
        assert_eq!(RANDOM_ENCOUNTER_DIE, 30);
        assert_eq!(RANDOM_ENCOUNTER_NIGHT_HOUR_LAST, 4);
        assert_eq!(RANDOM_ENCOUNTER_UNDERWORLD_THRESHOLD, 3);

        // Zero threshold rejects every draw.
        for roll in 1..=RANDOM_ENCOUNTER_DIE {
            assert!(!random_encounter_probe_fires(0, roll));
        }
        // For threshold = 5, draws 1..=4 fire, draw 5 does not.
        for roll in 1..=4 {
            assert!(random_encounter_probe_fires(5, roll));
        }
        assert!(!random_encounter_probe_fires(5, 5));
        assert!(!random_encounter_probe_fires(5, RANDOM_ENCOUNTER_DIE));
        // Underworld threshold of 3 fires on rolls 1 and 2 only.
        assert!(random_encounter_probe_fires(
            RANDOM_ENCOUNTER_UNDERWORLD_THRESHOLD,
            1
        ));
        assert!(random_encounter_probe_fires(
            RANDOM_ENCOUNTER_UNDERWORLD_THRESHOLD,
            2
        ));
        assert!(!random_encounter_probe_fires(
            RANDOM_ENCOUNTER_UNDERWORLD_THRESHOLD,
            3
        ));
    }

    #[test]
    fn per_turn_minute_increments_match_published_mode_costs() {
        // time.md §10: indoor mode loops pass one minute per turn,
        // the overworld passes two, the town rest path uses
        // ten-minute cleanup calls until the target hour is reached.
        assert_eq!(MINUTES_PER_INDOOR_TURN, 1);
        assert_eq!(MINUTES_PER_OUTDOOR_TURN, 2);
        assert_eq!(TOWN_REST_CLEANUP_INCREMENT_MINUTES, 10);
        // Combat rounds also use the indoor minute on counter wrap.
        assert_eq!(
            COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES,
            MINUTES_PER_INDOOR_TURN
        );
    }

    #[test]
    fn natural_moongate_live_and_underlying_tile_constants_match_spec() {
        // overworld.md §9: eligible saved Moonstone slots are stamped
        // with the live moon-gate tile (0xDC) while the gate-presence
        // counter is nonzero; when the counter reaches zero, the cell
        // is restored to the underlying terrain tile 5.
        assert_eq!(NATURAL_MOONGATE_LIVE_TILE, 0xDC);
        assert_eq!(NATURAL_MOONGATE_UNDERLYING_TILE, 5);
    }

    #[test]
    fn moongate_animator_render_eligibility_gates_at_full_daylight() {
        // overworld.md §natural-gates: the per-frame natural-moongate
        // animator only stamps a frame when the ambient light is at
        // or above the daytime threshold (= FULL_DAYLIGHT). Below
        // that threshold it resets its phase instead.
        assert_eq!(MOONGATE_ANIMATOR_DAYTIME_THRESHOLD, FULL_DAYLIGHT);
        assert!(!moongate_animator_render_eligible(FULL_DARKNESS));
        assert!(!moongate_animator_render_eligible(0));
        assert!(!moongate_animator_render_eligible(FULL_DAYLIGHT - 1));
        // Cells at the threshold are eligible; sentinel ambient
        // bytes (51+) are also above the threshold.
        assert!(moongate_animator_render_eligible(FULL_DAYLIGHT));
        assert!(moongate_animator_render_eligible(DAYLIGHT_SENTINEL_MIN));
        assert!(moongate_animator_render_eligible(u8::MAX));
        // Dawn/dusk gradient: only the final daytime-floor step
        // matches at FULL_DAYLIGHT itself; lower steps are below.
        for level in DAWN_DUSK_LIGHT {
            assert_eq!(
                moongate_animator_render_eligible(level),
                level >= FULL_DAYLIGHT
            );
        }
    }

    #[test]
    fn water_creature_body_retrieval_constants_match_spec() {
        // active-objects.md §9: after combat exits with the body
        // exit-message state, a restored water-creature trigger slot
        // (0x2C..=0x2F) is rewritten by lowering both sprite bytes
        // by 8 and stamping aux1 = 0x63, aux3 = 0x02.
        assert_eq!(WATER_CREATURE_BODY_TYPE_FIRST, 0x2C);
        assert_eq!(WATER_CREATURE_BODY_TYPE_LAST, 0x2F);
        assert_eq!(WATER_CREATURE_BODY_SPRITE_SHIFT, 8);
        assert_eq!(WATER_CREATURE_BODY_AUX1, 0x63);
        assert_eq!(WATER_CREATURE_BODY_AUX3, 0x02);
    }

    #[test]
    fn blackthorn_failure_victim_slot_names_the_second_party_member() {
        // blackthorn.md §5: a punishable failure beat names the
        // party's second visible member (zero-based slot index 1)
        // as the dragged-away victim. The cutscene actor slot
        // matches this index.
        assert_eq!(BLACKTHORN_FAILURE_VICTIM_SLOT, 1);
        assert_eq!(
            BlackthornCutsceneActor::SecondPartyMember.slot_index() as usize,
            BLACKTHORN_FAILURE_VICTIM_SLOT
        );
    }

    #[test]
    fn end_narrative_window_groups_split_return_home_from_blackthorn_arc() {
        // endgame.md §8: the six fixed END.DAT windows split into
        // two narrative arcs. Windows 1-3 are the return-home arc;
        // windows 4-6 are the Blackthorn judgment / orb arc.
        assert_eq!(
            EndNarrativeWindow::ReturnHomeOpening.group(),
            EndNarrativeGroup::ReturnHome
        );
        assert_eq!(
            EndNarrativeWindow::Homecoming.group(),
            EndNarrativeGroup::ReturnHome
        );
        assert_eq!(
            EndNarrativeWindow::RestlessNight.group(),
            EndNarrativeGroup::ReturnHome
        );
        assert_eq!(
            EndNarrativeWindow::BlackthornJudgmentOpen.group(),
            EndNarrativeGroup::BlackthornJudgment
        );
        assert_eq!(
            EndNarrativeWindow::BlackthornSentence.group(),
            EndNarrativeGroup::BlackthornJudgment
        );
        assert_eq!(
            EndNarrativeWindow::OrbExileResolution.group(),
            EndNarrativeGroup::BlackthornJudgment
        );
        assert_eq!(END_DAT_FILE, "END.DAT");
    }

    #[test]
    fn shrine_meditation_outcome_dispatches_by_quest_state() {
        // karma.md §7: a wrong/blank mantra prints the no-effect
        // branch; a correct mantra dispatches by the shrine quest
        // state bits (ordained, codex-read).
        use ShrineMeditationOutcome::*;

        // Wrong mantra never consults the bits.
        for ordained in [false, true] {
            for codex_read in [false, true] {
                assert_eq!(
                    shrine_meditation_outcome(false, ordained, codex_read),
                    NoEffect
                );
            }
        }

        // Correct mantra: dispatch by the quest-state bit pair.
        assert_eq!(shrine_meditation_outcome(true, false, false), Ordain);
        assert_eq!(shrine_meditation_outcome(true, true, false), AlreadyOrdained);
        assert_eq!(shrine_meditation_outcome(true, true, true), CodexTurnIn);
        assert_eq!(shrine_meditation_outcome(true, false, true), GoldOffering);
    }

    #[test]
    fn story_text_marker_classifies_published_renderer_bytes() {
        // formats/story-dat.md §3: the renderer recognises `{` as a
        // paragraph/page-start, `_` as a soft hyphen / syllable
        // break, `\n` as a hard newline, and `\0` as the record
        // terminator. Other bytes pass through to the glyph renderer.
        assert_eq!(
            story_text_marker(b'{'),
            Some(StoryTextMarker::ParagraphStart)
        );
        assert_eq!(
            story_text_marker(b'_'),
            Some(StoryTextMarker::SoftBreak)
        );
        assert_eq!(
            story_text_marker(b'\n'),
            Some(StoryTextMarker::HardNewline)
        );
        assert_eq!(
            story_text_marker(0),
            Some(StoryTextMarker::RecordEnd)
        );
        // Ordinary ASCII glyphs return None.
        assert_eq!(story_text_marker(b'A'), None);
        assert_eq!(story_text_marker(b'.'), None);
        assert_eq!(story_text_marker(b' '), None);
        // Marker byte constants line up with the enum classifier.
        assert_eq!(STORY_PARAGRAPH_START_MARKER, b'{');
        assert_eq!(STORY_SOFT_BREAK_MARKER, b'_');
        assert_eq!(STORY_HARD_NEWLINE_MARKER, b'\n');
        assert_eq!(STORY_RECORD_END_MARKER, 0);
    }

    #[test]
    fn intro_step_extras_match_published_pixel_coordinates() {
        // intro.md §10: step 1 draws STORY1 subimage 2 at (40, 86)
        // then runs a rectangular transition over (40, 86)..(75, 120);
        // step 6 draws STORY2 subimage 3 at (96, 39); steps 0, 7, 14
        // each pre-draw two TEXT.16 transition subimages at the
        // published pixel coordinates.
        assert_eq!(INTRO_STEP_1_EXTRA_ART_X, 40);
        assert_eq!(INTRO_STEP_1_EXTRA_ART_Y, 86);
        assert_eq!(INTRO_STEP_1_EXTRA_SUBIMAGE, 2);
        assert_eq!(INTRO_STEP_1_RECT_TRANSITION, (40, 86, 75, 120));

        assert_eq!(INTRO_STEP_6_EXTRA_ART_X, 96);
        assert_eq!(INTRO_STEP_6_EXTRA_ART_Y, 39);
        assert_eq!(INTRO_STEP_6_EXTRA_SUBIMAGE, 3);

        assert_eq!(
            intro_step_transition_strips(0),
            Some([(0, 224, 30), (1, 168, 58)])
        );
        assert_eq!(
            intro_step_transition_strips(7),
            Some([(0, 232, 26), (2, 200, 54)])
        );
        assert_eq!(
            intro_step_transition_strips(14),
            Some([(0, 184, 0), (3, 248, 0)])
        );
        // Steps without the pre-draw return None.
        assert_eq!(intro_step_transition_strips(1), None);
        assert_eq!(intro_step_transition_strips(6), None);
        assert_eq!(intro_step_transition_strips(20), None);
        assert_eq!(intro_step_transition_strips(21), None);
    }

    #[test]
    fn title_bit_remaining_placements_match_published_table() {
        // intro.md §3: after the seven-slot initial title mark, the
        // title sequence draws TITLE.BIT 7, TITLE.BIT 8,
        // BRITISH.BIT 0, and TITLE.BIT 9 in that order at the
        // published 320x200 pixel rectangles.
        assert_eq!(TITLE_BIT_REMAINING_PLACEMENTS.len(), 4);
        assert_eq!(
            TITLE_BIT_REMAINING_PLACEMENTS[0],
            TitleBitPlacement {
                asset: TitleBitAsset::Title,
                slot: 7,
                top_left_x: 108,
                top_left_y: 140,
                width: 104,
                height: 33,
            }
        );
        assert_eq!(
            TITLE_BIT_REMAINING_PLACEMENTS[1],
            TitleBitPlacement {
                asset: TitleBitAsset::Title,
                slot: 8,
                top_left_x: 152,
                top_left_y: 0,
                width: 16,
                height: 15,
            }
        );
        assert_eq!(
            TITLE_BIT_REMAINING_PLACEMENTS[2],
            TitleBitPlacement {
                asset: TitleBitAsset::British,
                slot: 0,
                top_left_x: 24,
                top_left_y: 66,
                width: 272,
                height: 62,
            }
        );
        assert_eq!(
            TITLE_BIT_REMAINING_PLACEMENTS[3],
            TitleBitPlacement {
                asset: TitleBitAsset::Title,
                slot: 9,
                top_left_x: 104,
                top_left_y: 160,
                width: 112,
                height: 33,
            }
        );
    }

    #[test]
    fn intro_panel_asset_suffix_pairs_high_and_low_colour_drivers() {
        // boot.md §5 / intro.md §3: the driver family chooses the
        // paired screen-panel asset suffix. EGA and Tandy are
        // high-colour (.16); CGA and Hercules are low-colour (.4).
        assert_eq!(DisplayDriverFamily::Ega.intro_panel_asset_suffix(), ".16");
        assert_eq!(DisplayDriverFamily::Tandy.intro_panel_asset_suffix(), ".16");
        assert_eq!(DisplayDriverFamily::Cga.intro_panel_asset_suffix(), ".4");
        assert_eq!(
            DisplayDriverFamily::Hercules.intro_panel_asset_suffix(),
            ".4"
        );

        assert!(DisplayDriverFamily::Ega.uses_high_colour_intro_assets());
        assert!(DisplayDriverFamily::Tandy.uses_high_colour_intro_assets());
        assert!(!DisplayDriverFamily::Cga.uses_high_colour_intro_assets());
        assert!(!DisplayDriverFamily::Hercules.uses_high_colour_intro_assets());
    }

    #[test]
    fn save_load_published_messages_match_spec_text() {
        // save-load.md §5.2 / §4.2: Quit-and-Save prints the prompt
        // line, the Yes/No reply, "Saving..." and "Done."; the load
        // flow's empty-save guard prints three named lines in order.
        assert_eq!(SAVE_PROMPT_MESSAGE, "Save game?");
        assert_eq!(SAVE_PROMPT_YES_REPLY, "Yes");
        assert_eq!(SAVE_PROMPT_NO_REPLY, "No");
        assert_eq!(SAVE_IN_PROGRESS_MESSAGE, "Saving...");
        assert_eq!(SAVE_DONE_MESSAGE, "Done.");
        assert_eq!(LOAD_EMPTY_SAVE_LINE_1, "No active game");
        assert_eq!(LOAD_EMPTY_SAVE_LINE_2, "Please create a character");
        assert_eq!(LOAD_EMPTY_SAVE_LINE_3, "or transfer one from Ultima IV");
    }

    #[test]
    fn yell_published_message_constants_match_spec_text() {
        // commands.md §11: the ship-aboard Y branch prints the
        // sail-state message; the free-text branch prints
        // "Nothing said." on empty input. Spec text is verbatim.
        assert_eq!(YELL_SAILS_HOISTED_MESSAGE, "Sails hoisted.");
        assert_eq!(YELL_SAILS_FURLED_MESSAGE, "Sails furled.");
        assert_eq!(YELL_NOTHING_SAID_MESSAGE, "Nothing said.");
    }

    #[test]
    fn jimmy_npc_pickpocket_karma_reward_is_two() {
        // doors-and-z-transitions.md §3: a successful NPC pickpocket
        // raises the shared moral-standing selector by +2, clamped
        // at the published cap. Failure does not advance the
        // picked/thanked state and does not apply this increase.
        assert_eq!(JIMMY_NPC_PICKPOCKET_KARMA_REWARD, 2);
        // The standing-clamp helper still owns the 0..=99 cap; the
        // reward is the unclamped per-event delta.
        let standing: u8 = 98;
        let next = standing
            .saturating_add(JIMMY_NPC_PICKPOCKET_KARMA_REWARD)
            .min(MORAL_STANDING_MAX);
        assert_eq!(next, MORAL_STANDING_MAX);
        let standing: u8 = 0;
        let next = standing
            .saturating_add(JIMMY_NPC_PICKPOCKET_KARMA_REWARD)
            .min(MORAL_STANDING_MAX);
        assert_eq!(next, 2);
    }

    #[test]
    fn pushable_floor_stamps_split_cannon_from_generic() {
        // commands.md §8: cannon-family pushes leave the published
        // 0x45 stamp on the vacated cell; every other pushable family
        // leaves the generic 0x44 cobble stamp. Both render as cobble
        // but the stamp byte is load-bearing for the family-matching
        // push/pull rule.
        assert_eq!(PUSHABLE_GENERIC_FLOOR_STAMP, 0x44);
        assert_eq!(PUSHABLE_CANNON_FLOOR_STAMP, 0x45);
        assert_eq!(
            PushableTileFamily::NonRotating5B.floor_stamp(),
            PUSHABLE_GENERIC_FLOOR_STAMP
        );
        assert_eq!(
            PushableTileFamily::ChairFourFacing.floor_stamp(),
            PUSHABLE_GENERIC_FLOOR_STAMP
        );
        assert_eq!(
            PushableTileFamily::NonRotatingA5A6A8A9.floor_stamp(),
            PUSHABLE_GENERIC_FLOOR_STAMP
        );
        assert_eq!(
            PushableTileFamily::NonRotatingAdAf.floor_stamp(),
            PUSHABLE_GENERIC_FLOOR_STAMP
        );
        assert_eq!(
            PushableTileFamily::CannonFourFacing.floor_stamp(),
            PUSHABLE_CANNON_FLOOR_STAMP
        );
    }

    #[test]
    fn new_order_outcome_resolves_cancellation_refusal_and_swap() {
        // commands.md §6: cancelling either selector prompt aborts
        // without consuming a turn; a leader-slot selection refuses
        // the swap with the leader-must-remain-first rule; otherwise
        // the two roster records exchange and the turn is consumed.
        use NewOrderOutcome::*;

        // Cancellation on either side.
        assert_eq!(new_order_outcome(None, None), Cancelled);
        assert_eq!(new_order_outcome(None, Some(2)), Cancelled);
        assert_eq!(new_order_outcome(Some(1), None), Cancelled);

        // Leader-slot refusal.
        assert_eq!(new_order_outcome(Some(0), Some(2)), LeaderRefusal);
        assert_eq!(new_order_outcome(Some(3), Some(0)), LeaderRefusal);
        assert_eq!(new_order_outcome(Some(0), Some(0)), LeaderRefusal);

        // Non-leader swap (same-slot pair is accepted; the swap is
        // a no-op but the caller still consumes the turn).
        assert_eq!(
            new_order_outcome(Some(1), Some(2)),
            Swap { slot_a: 1, slot_b: 2 }
        );
        assert_eq!(
            new_order_outcome(Some(4), Some(4)),
            Swap { slot_a: 4, slot_b: 4 }
        );

        // The bool-returning predicate agrees with the typed result.
        assert!(new_order_swap_accepted(1, 2));
        assert!(new_order_swap_accepted(4, 4));
        assert!(!new_order_swap_accepted(0, 2));
        assert!(!new_order_swap_accepted(3, 0));
    }

    #[test]
    fn party_target_selector_result_distinguishes_explicit_none_from_cancel() {
        // input.md §9: the selector returns three semantic families:
        // a non-negative slot index, an Escape/cancel result, and a
        // distinct explicit-none result produced only by the `0` key.
        use PartyTargetSelectorResult::*;
        // Digits 1..=6 land on slots 0..=5.
        for digit in 1u8..=6 {
            assert_eq!(
                party_target_selector_result(b'0' + digit),
                Some(Slot(digit - 1))
            );
        }
        // `0` is the explicit-none result, distinct from Escape's cancel.
        assert_eq!(party_target_selector_result(b'0'), Some(ExplicitNone));
        assert_eq!(party_target_selector_result(0x1B), Some(Cancel));
        // Other digits, Space/Enter (caller-driven highlight confirm),
        // and arbitrary bytes do not yield a three-family result.
        assert_eq!(party_target_selector_result(b'7'), None);
        assert_eq!(party_target_selector_result(b' '), None);
        assert_eq!(party_target_selector_result(b'\r'), None);
        assert_eq!(party_target_selector_result(b'X'), None);
    }

    #[test]
    fn direction_prompt_label_echoes_published_cardinal_names() {
        // input.md §10,§11: the shared direction prompts echo the
        // cardinal name on a cardinal keystroke and `Pass` on Space.
        // Ignored re-polls produce no echo.
        assert_eq!(
            direction_prompt_label(CardinalPromptAction::Cardinal(InputDirection::North)),
            Some("North")
        );
        assert_eq!(
            direction_prompt_label(CardinalPromptAction::Cardinal(InputDirection::South)),
            Some("South")
        );
        assert_eq!(
            direction_prompt_label(CardinalPromptAction::Cardinal(InputDirection::East)),
            Some("East")
        );
        assert_eq!(
            direction_prompt_label(CardinalPromptAction::Cardinal(InputDirection::West)),
            Some("West")
        );
        assert_eq!(
            direction_prompt_label(CardinalPromptAction::Pass),
            Some("Pass")
        );
        assert_eq!(direction_prompt_label(CardinalPromptAction::Ignored), None);
        assert_eq!(SPELL_DIRECTION_PROMPT_PREFIX, "Direction-");
    }

    #[test]
    fn numeric_prompt_accumulates_digits_and_pops_on_backspace() {
        // input.md §8: numeric prompts apply value = value * 10 +
        // digit and treat Backspace as integer-division by ten.
        // Enter terminates and Discard leaves the accumulator alone.
        use NumericPromptAction::*;
        assert_eq!(numeric_prompt_action(b'3'), AppendDigit(3));
        assert_eq!(numeric_prompt_action(b'0'), AppendDigit(0));
        assert_eq!(numeric_prompt_action(b'9'), AppendDigit(9));
        assert_eq!(numeric_prompt_action(0x08), Pop);
        assert_eq!(numeric_prompt_action(b'\r'), Submit);
        assert_eq!(numeric_prompt_action(b'\n'), Submit);
        assert_eq!(numeric_prompt_action(0x1B), Discard);
        assert_eq!(numeric_prompt_action(b' '), Discard);
        assert_eq!(numeric_prompt_action(b'a'), Discard);

        // Building "1234" digit by digit.
        let mut v = 0u16;
        for digit in [1, 2, 3, 4] {
            v = numeric_prompt_apply(v, AppendDigit(digit));
        }
        assert_eq!(v, 1234);
        // Backspace removes one digit (1234 / 10 = 123).
        v = numeric_prompt_apply(v, Pop);
        assert_eq!(v, 123);
        // Submit/Discard leave the accumulator unchanged.
        assert_eq!(numeric_prompt_apply(v, Submit), 123);
        assert_eq!(numeric_prompt_apply(v, Discard), 123);
        // Pop on zero stays zero.
        assert_eq!(numeric_prompt_apply(0, Pop), 0);
        // Saturating cap: appending digits past u16::MAX clamps.
        assert_eq!(
            numeric_prompt_apply(u16::MAX, AppendDigit(9)),
            u16::MAX
        );
    }

    #[test]
    fn talk_entry_refusal_strings_match_spec_text() {
        // conversation.md §2: the Talk command emits these exact
        // strings at the named entry-time gates before the
        // conversation engine ever runs.
        assert_eq!(TALK_NOBODY_HERE_MESSAGE, "Nobody's here!");
        assert_eq!(TALK_SLEEPING_MESSAGE, "Zzzzzz...");
        assert_eq!(TALK_NO_RESPONSE_MESSAGE, "No response!");
    }

    #[test]
    fn mmix_published_message_constants_match_spec_text() {
        // magic.md §6: the world-mode M-Mix handler prints these
        // exact strings at the named checkpoints, and combat refuses
        // M with a distinct line.
        assert_eq!(MMIX_NO_REAGENTS_OWNED_MESSAGE, "No reagents owned!");
        assert_eq!(MMIX_EMPTY_SELECTION_MESSAGE, "Nothing to mix!");
        assert_eq!(
            MMIX_INSUFFICIENT_REAGENTS_MESSAGE,
            "Insufficient reagents!"
        );
        assert_eq!(MMIX_SPELL_PROMPT_MESSAGE, "For what spell?");
        assert_eq!(MMIX_QUANTITY_PROMPT_MESSAGE, "How much?");
        assert_eq!(MMIX_MIXING_MESSAGE, "Mixing...");
        assert_eq!(MMIX_COMBAT_REFUSAL_MESSAGE, "Mix-Not here");
    }

    #[test]
    fn door_auto_close_tick_counts_down_then_closes() {
        // doors-and-z-transitions.md §5: each turn-consuming pass
        // decrements the four-turn countdown; when it hits zero, the
        // door auto-closes. An idle slot stays idle.
        assert_eq!(door_auto_close_tick(None), DoorAutoCloseTick::Idle);
        assert_eq!(
            door_auto_close_tick(Some(DOOR_AUTO_CLOSE_TURNS)),
            DoorAutoCloseTick::DecrementInPlace(DOOR_AUTO_CLOSE_TURNS - 1)
        );
        assert_eq!(
            door_auto_close_tick(Some(3)),
            DoorAutoCloseTick::DecrementInPlace(2)
        );
        assert_eq!(
            door_auto_close_tick(Some(2)),
            DoorAutoCloseTick::DecrementInPlace(1)
        );
        // 1 → 0 reaches zero this tick, so the door closes.
        assert_eq!(
            door_auto_close_tick(Some(1)),
            DoorAutoCloseTick::CloseAndClear
        );
        // Already-zero slots also close (and the cell rewrite fires
        // before the slot becomes idle).
        assert_eq!(
            door_auto_close_tick(Some(0)),
            DoorAutoCloseTick::CloseAndClear
        );
        // The countdown initialiser is four turns.
        assert_eq!(DOOR_AUTO_CLOSE_TURNS, 4);
    }

    #[test]
    fn dungeon_room_trigger_promoted_visit_byte_remaps_to_0xa_family() {
        // dungeon-mode.md §5: the room-entry helper patches the
        // loaded dungeon image by rewriting the 0xF? trigger to the
        // matching 0xA? helper-state byte, preserving the low nibble
        // (room/arena slot id). The on-disk source byte is unchanged.
        for low in 0u8..=0x0f {
            let trigger = 0xf0 | low;
            assert_eq!(
                dungeon_room_trigger_promoted_visit_byte(trigger),
                Some(0xa0 | low)
            );
            // Round-trip predicates agree.
            assert!(is_dungeon_room_trigger(trigger));
            assert!(is_dungeon_room_helper_state(0xa0 | low));
            assert_eq!(dungeon_room_slot(trigger), low);
            assert_eq!(dungeon_room_slot(0xa0 | low), low);
        }
        // Bytes outside the trigger family return None.
        assert_eq!(dungeon_room_trigger_promoted_visit_byte(0x00), None);
        assert_eq!(dungeon_room_trigger_promoted_visit_byte(0x4a), None);
        assert_eq!(dungeon_room_trigger_promoted_visit_byte(0xa3), None);
        assert_eq!(dungeon_room_trigger_promoted_visit_byte(0xef), None);
    }

    #[test]
    fn town_fire_active_object_hit_subtracts_five_karma_floored_at_zero() {
        // vehicles.md §8: the local cannon path subtracts five units
        // from the moral-standing selector on a successful active-
        // object hit, floored at zero. Door-destroyed hits and empty
        // shots don't apply this penalty (caller-gated).
        assert_eq!(TOWN_FIRE_ACTIVE_OBJECT_HIT_KARMA_DEBIT, 5);
        assert_eq!(town_fire_active_object_hit_standing(0), 0);
        assert_eq!(town_fire_active_object_hit_standing(1), 0);
        assert_eq!(town_fire_active_object_hit_standing(4), 0);
        assert_eq!(town_fire_active_object_hit_standing(5), 0);
        assert_eq!(town_fire_active_object_hit_standing(6), 1);
        assert_eq!(town_fire_active_object_hit_standing(50), 45);
        assert_eq!(town_fire_active_object_hit_standing(99), 94);
        assert_eq!(town_fire_active_object_hit_standing(u8::MAX), 250);
    }

    #[test]
    fn chargen_seed_inventory_constants_match_published_seed() {
        // chargen.md §8: the seed file `INIT.GAM` supplies the
        // starting-inventory counters. Chargen does not customise
        // them; the published seed values are the contract.
        assert_eq!(CHARGEN_SEED_FOOD, 63);
        assert_eq!(CHARGEN_SEED_GOLD, 150);
        assert_eq!(CHARGEN_SEED_KEYS, 2);
        assert_eq!(CHARGEN_SEED_GEMS, 0);
        assert_eq!(CHARGEN_SEED_TORCHES, 4);
        assert_eq!(CHARGEN_SEED_MAGIC_POWDER, 0);

        assert_eq!(CHARGEN_SEED_REAGENT_BLACK_PEARL, 4);
        assert_eq!(CHARGEN_SEED_REAGENT_BLOOD_MOSS, 6);
        assert_eq!(CHARGEN_SEED_REAGENT_GARLIC, 7);
        assert_eq!(CHARGEN_SEED_REAGENT_GINSENG, 6);
        assert_eq!(CHARGEN_SEED_REAGENT_MANDRAKE, 0);
        assert_eq!(CHARGEN_SEED_REAGENT_NIGHTSHADE, 3);
        assert_eq!(CHARGEN_SEED_REAGENT_SPIDER_SILK, 0);
        assert_eq!(CHARGEN_SEED_REAGENT_SULFUROUS_ASH, 0);
    }

    #[test]
    fn age_character_month_counter_caps_at_twenty_five() {
        // time.md §8: the 28-day rollover increments each character
        // record's one-byte month counter, capped at 25. Pickup later
        // treats a stored zero as one billable unit.
        assert_eq!(age_character_month_counter(0), 1);
        assert_eq!(age_character_month_counter(1), 2);
        assert_eq!(
            age_character_month_counter(CHARACTER_MONTH_COUNTER_CAP - 1),
            CHARACTER_MONTH_COUNTER_CAP
        );
        assert_eq!(
            age_character_month_counter(CHARACTER_MONTH_COUNTER_CAP),
            CHARACTER_MONTH_COUNTER_CAP
        );
        // Values above the cap (which shouldn't occur in practice
        // but might appear in malformed saves) clamp without wrapping.
        assert_eq!(
            age_character_month_counter(CHARACTER_MONTH_COUNTER_CAP + 1),
            CHARACTER_MONTH_COUNTER_CAP
        );
        assert_eq!(
            age_character_month_counter(u8::MAX),
            CHARACTER_MONTH_COUNTER_CAP
        );
        assert_eq!(CHARACTER_MONTH_COUNTER_CAP, 25);
    }

    #[test]
    fn conversation_cleanup_reconciliation_follows_published_priority() {
        // quest-flags.md §5: zero-sentinel cleanup branches in fixed
        // priority order, decrementing at most one byte-sized signal
        // per call. Gold-debit fallback only fires when every byte
        // array is empty.
        use ConversationCleanupReconciliation::*;

        // Resource band wins outright when it has any nonzero entry.
        assert_eq!(
            conversation_cleanup_reconciliation(true, false, false),
            ResourceBand
        );
        assert_eq!(
            conversation_cleanup_reconciliation(true, true, true),
            ResourceBand
        );
        // Generic signal array wins when the resource band is empty.
        assert_eq!(
            conversation_cleanup_reconciliation(false, true, false),
            GenericSignalArray
        );
        assert_eq!(
            conversation_cleanup_reconciliation(false, true, true),
            GenericSignalArray
        );
        // Eight-slot arrays take the next priority.
        assert_eq!(
            conversation_cleanup_reconciliation(false, false, true),
            EightSlotSignalArrays
        );
        // Gold debit only when everything else is empty.
        assert_eq!(
            conversation_cleanup_reconciliation(false, false, false),
            GoldDebitFallback
        );
    }

    #[test]
    fn equipped_item_weight_stat_sums_six_slots_with_protection_bonus() {
        // inventory.md §2.1 / catalogs/item-list.md §5.1: the
        // equipped-item statistic helper sums the published
        // equipped-weight column for the six readied slots, treats
        // empty slots as zero, and adds 3 when Protection is active.

        // Fully empty equipment.
        let empty: [u8; EQUIPMENT_SLOT_COUNT] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        assert_eq!(equipped_item_weight_stat(&empty, false), 0);
        assert_eq!(
            equipped_item_weight_stat(&empty, true),
            EQUIPPED_WEIGHT_PROTECTION_BONUS
        );

        // Mixed loadout: Plate Mail (14 → 7) + Magic Shield (7 → 5) +
        // Mystic Sword (41 → 1) + Ring of Protection (43 → 2) +
        // Spiked Collar (46 → 2) = 17. The sixth slot stays empty.
        let mut kit: [u8; EQUIPMENT_SLOT_COUNT] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        kit[0] = 14;
        kit[1] = 7;
        kit[2] = 41;
        kit[3] = 43;
        kit[4] = 46;
        assert_eq!(equipped_item_weight_stat(&kit, false), 17);
        // With Protection active, the bonus stacks on top.
        assert_eq!(
            equipped_item_weight_stat(&kit, true),
            17 + EQUIPPED_WEIGHT_PROTECTION_BONUS
        );

        // The protection bonus is exactly the documented 3 units.
        assert_eq!(EQUIPPED_WEIGHT_PROTECTION_BONUS, 3);

        // Published spot checks against the item-list table.
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[0], 1); // Leather Helm
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[15], 10); // Mystic Armour
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[16], 0); // Dagger
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[20], 1); // Main Gauche
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[43], 2); // Ring of Protection
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[46], 2); // Spiked Collar
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[47], 0); // Ankh
    }

    #[test]
    fn sleep_ambush_restored_status_preserves_poisoned_and_lifts_sleepers() {
        // rest-and-camp.md §6: before handing the ambush row to
        // combat setup, the rest helper restores each rest-local
        // snapshot: Poisoned stays Poisoned; Good/Sleeping become
        // Good; Charmed/Dead/Ashes pass through (they are not
        // promoted to active combatants).
        assert_eq!(
            sleep_ambush_restored_status(CharacterStatus::PoisonedOrRevived),
            CharacterStatus::PoisonedOrRevived
        );
        assert_eq!(
            sleep_ambush_restored_status(CharacterStatus::Good),
            CharacterStatus::Good
        );
        assert_eq!(
            sleep_ambush_restored_status(CharacterStatus::Sleeping),
            CharacterStatus::Good
        );
        assert_eq!(
            sleep_ambush_restored_status(CharacterStatus::Charmed),
            CharacterStatus::Charmed
        );
        assert_eq!(
            sleep_ambush_restored_status(CharacterStatus::Dead),
            CharacterStatus::Dead
        );
        assert_eq!(
            sleep_ambush_restored_status(CharacterStatus::Ashes),
            CharacterStatus::Ashes
        );
    }

    #[test]
    fn blackthorn_rescue_post_print_standing_clamps_to_floor() {
        // karma.md §6: after the rescue path prints its verdict, the
        // moral-standing selector is raised to at least 75. Inputs
        // at or above the floor pass through unchanged.
        assert_eq!(blackthorn_rescue_post_print_standing(0), 75);
        assert_eq!(blackthorn_rescue_post_print_standing(74), 75);
        assert_eq!(
            blackthorn_rescue_post_print_standing(BLACKTHORN_RESCUE_STANDING_FLOOR),
            BLACKTHORN_RESCUE_STANDING_FLOOR
        );
        assert_eq!(blackthorn_rescue_post_print_standing(75), 75);
        assert_eq!(blackthorn_rescue_post_print_standing(76), 76);
        assert_eq!(blackthorn_rescue_post_print_standing(99), 99);
        // 99 is the published selector cap, but the helper does not
        // re-cap: it only enforces the floor.
        assert_eq!(blackthorn_rescue_post_print_standing(255), 255);
    }

    #[test]
    fn wind_setter_outcome_makes_calm_to_calm_a_noop() {
        // weather.md §3: the shared wind setter treats Calm → Calm as
        // a no-op; every other accepted transition plays the wind
        // sound and refreshes the stored wind.
        assert_eq!(
            wind_setter_outcome(WindState::Calm, WindState::Calm),
            WindSetterOutcome::NoOp
        );
        // Calm → cardinal is an Apply.
        for new in [
            WindState::North,
            WindState::South,
            WindState::East,
            WindState::West,
        ] {
            assert_eq!(
                wind_setter_outcome(WindState::Calm, new),
                WindSetterOutcome::Apply
            );
        }
        // Cardinal → Calm is an Apply (the setter accepts Calm
        // programmatically; only Calm → Calm is the no-op).
        assert_eq!(
            wind_setter_outcome(WindState::North, WindState::Calm),
            WindSetterOutcome::Apply
        );
        // Cardinal → cardinal (including same direction) applies.
        assert_eq!(
            wind_setter_outcome(WindState::East, WindState::East),
            WindSetterOutcome::Apply
        );
        assert_eq!(
            wind_setter_outcome(WindState::East, WindState::West),
            WindSetterOutcome::Apply
        );
    }

    #[test]
    fn sky_strip_composed_cells_respects_plot_order_and_visibility() {
        // moons.md §2: hour 0 has FixedHour off-strip (visible only 6..=17),
        // Trammel visible (cell 8 - 0 = 8), Felucca visible (cell 2 - 0 = 2).
        let cells = sky_strip_composed_cells(0);
        assert_eq!(cells[8], Some(SkyStripMarker::Trammel));
        assert_eq!(cells[2], Some(SkyStripMarker::Felucca));
        // All other cells stay blank.
        for (cell_index, slot) in cells.iter().enumerate() {
            if cell_index != 2 && cell_index != 8 {
                assert!(slot.is_none(), "cell {cell_index} should be blank");
            }
        }

        // Hour 6: FixedHour at 17-6=11; Trammel at 8-6=2; Felucca off.
        let cells = sky_strip_composed_cells(6);
        assert_eq!(cells[11], Some(SkyStripMarker::FixedHour));
        assert_eq!(cells[2], Some(SkyStripMarker::Trammel));
        assert!(cells[0].is_none());

        // Hour 8: FixedHour at 9; Trammel at 0; Felucca off.
        let cells = sky_strip_composed_cells(8);
        assert_eq!(cells[9], Some(SkyStripMarker::FixedHour));
        assert_eq!(cells[0], Some(SkyStripMarker::Trammel));

        // Hour 17: FixedHour at 0; Trammel off; Felucca at 26-17=9.
        let cells = sky_strip_composed_cells(17);
        assert_eq!(cells[0], Some(SkyStripMarker::FixedHour));
        assert_eq!(cells[9], Some(SkyStripMarker::Felucca));

        // Plot order: later marker overwrites earlier when they pick
        // the same cell. At hour 15, FixedHour=17-15=2 and Felucca=26-15=11.
        // Trammel is off. Neither overlap. Construct a collision case:
        // hour 6 produces FixedHour=11; Trammel=2; Felucca=20 (off-strip).
        // hour 8 produces FixedHour=9; Trammel=0; Felucca off.
        // Trammel at hour 21 → 32-21=11; FixedHour off (hour > 17).
        // To get a real overwrite we need two markers landing on the
        // same cell. At hour 0: Trammel=8, Felucca=2 — no overlap.
        // At hour 8: FixedHour=9, Trammel=0 — no overlap. The render
        // order itself is what callers depend on; verify the
        // SKY_STRIP_RENDER_ORDER constant is hour, Trammel, Felucca.
        assert_eq!(
            SKY_STRIP_RENDER_ORDER,
            [
                SkyStripMarker::FixedHour,
                SkyStripMarker::Trammel,
                SkyStripMarker::Felucca,
            ]
        );

        // Hour 13 is fully off-strip: FixedHour at 17-13=4 in-range,
        // but Trammel (0..=8 or 21..=23) and Felucca (0..=2 or
        // 15..=23) are both above the horizon, so only FixedHour is set.
        let cells = sky_strip_composed_cells(13);
        assert_eq!(cells[4], Some(SkyStripMarker::FixedHour));
        for (cell_index, slot) in cells.iter().enumerate() {
            if cell_index != 4 {
                assert!(slot.is_none(), "cell {cell_index} should be blank");
            }
        }
    }

    #[test]
    fn shared_trap_effect_family_classifies_combat_and_non_combat() {
        // traps.md §3: combat scenes resolve only to Acid (id 0) or
        // Poison (id 1); non-combat scenes follow the 3/2/2/1 outcome
        // distribution Acid/Poison/Bomb/Gas across 8 equiprobable rolls.
        for index in 0u8..=255 {
            let combat = shared_trap_effect_family_from_index(index, true);
            assert!(matches!(combat, TrapEffect::Acid | TrapEffect::Poison));
            assert!(trap_effect_appears_in_combat(combat));
        }
        // Non-combat distribution mirrors trap_non_combat_outcomes.
        let mut counts = [0u32; 4];
        for index in 0u8..=7 {
            let fam = shared_trap_effect_family_from_index(index, false);
            let bucket = match fam {
                TrapEffect::Acid => 0,
                TrapEffect::Poison => 1,
                TrapEffect::Bomb => 2,
                TrapEffect::Gas => 3,
            };
            counts[bucket] += 1;
        }
        assert_eq!(
            counts[0],
            u32::from(trap_non_combat_outcomes(TrapEffect::Acid))
        );
        assert_eq!(
            counts[1],
            u32::from(trap_non_combat_outcomes(TrapEffect::Poison))
        );
        assert_eq!(
            counts[2],
            u32::from(trap_non_combat_outcomes(TrapEffect::Bomb))
        );
        assert_eq!(
            counts[3],
            u32::from(trap_non_combat_outcomes(TrapEffect::Gas))
        );
        // High bits beyond the low 3 must not change the non-combat
        // family — the resolver masks `index & 7`.
        assert_eq!(
            shared_trap_effect_family_from_index(0xF8, false),
            shared_trap_effect_family_from_index(0x00, false)
        );
    }

    #[test]
    fn light_counter_spend_with_tag_threads_timing_tag() {
        // lighting.md §5: per-turn cleanup applies the same Q/T
        // adjustments to the light-counter spend that it applies to
        // the minute increment before decaying the counter.
        // No tag => cadence spend passes through.
        assert_eq!(
            light_counter_spend_with_tag(LightDecayCadence::TownDungeonCombatTurn, 0),
            Some(1)
        );
        assert_eq!(
            light_counter_spend_with_tag(LightDecayCadence::OverworldTurn, 0),
            Some(2)
        );
        assert_eq!(
            light_counter_spend_with_tag(LightDecayCadence::Wait(7), 0),
            Some(7)
        );
        assert_eq!(
            light_counter_spend_with_tag(LightDecayCadence::ModeZeroRefresh, 0),
            Some(0)
        );
        // T tag suppresses the light-counter advance entirely.
        assert_eq!(
            light_counter_spend_with_tag(
                LightDecayCadence::OverworldTurn,
                TIMING_TAG_NEGATE_TIME
            ),
            None
        );
        assert_eq!(
            light_counter_spend_with_tag(
                LightDecayCadence::Wait(9),
                TIMING_TAG_NEGATE_TIME
            ),
            None
        );
        // Q tag halves the spend with the same one-unit floor the
        // minute increment uses: a town turn's spend of 1 stays at 1.
        assert_eq!(
            light_counter_spend_with_tag(
                LightDecayCadence::TownDungeonCombatTurn,
                TIMING_TAG_QUICKNESS
            ),
            Some(1)
        );
        assert_eq!(
            light_counter_spend_with_tag(
                LightDecayCadence::OverworldTurn,
                TIMING_TAG_QUICKNESS
            ),
            Some(1)
        );
        assert_eq!(
            light_counter_spend_with_tag(
                LightDecayCadence::Wait(8),
                TIMING_TAG_QUICKNESS
            ),
            Some(4)
        );
        // Mode-zero refresh stays at zero under Q.
        assert_eq!(
            light_counter_spend_with_tag(
                LightDecayCadence::ModeZeroRefresh,
                TIMING_TAG_QUICKNESS
            ),
            Some(0)
        );
    }

    #[test]
    fn npc_and_tlk_family_filenames_match_published_extensions() {
        // formats/npc.md §2 / formats/tlk.md §2: the four town-family
        // roster and dialogue filenames are stable string constants
        // and the per-scene-byte helpers must agree with them.
        assert_eq!(TOWNE_NPC_FILENAME, "TOWNE.NPC");
        assert_eq!(DWELLING_NPC_FILENAME, "DWELLING.NPC");
        assert_eq!(CASTLE_NPC_FILENAME, "CASTLE.NPC");
        assert_eq!(KEEP_NPC_FILENAME, "KEEP.NPC");
        assert_eq!(TOWNE_TLK_FILENAME, "TOWNE.TLK");
        assert_eq!(DWELLING_TLK_FILENAME, "DWELLING.TLK");
        assert_eq!(CASTLE_TLK_FILENAME, "CASTLE.TLK");
        assert_eq!(KEEP_TLK_FILENAME, "KEEP.TLK");

        assert_eq!(npc_roster_filename(1), Some(TOWNE_NPC_FILENAME));
        assert_eq!(npc_roster_filename(8), Some(TOWNE_NPC_FILENAME));
        assert_eq!(npc_roster_filename(9), Some(DWELLING_NPC_FILENAME));
        assert_eq!(npc_roster_filename(17), Some(CASTLE_NPC_FILENAME));
        assert_eq!(npc_roster_filename(25), Some(KEEP_NPC_FILENAME));
        assert_eq!(npc_roster_filename(0), None);
        assert_eq!(npc_roster_filename(33), None);

        assert_eq!(npc_tlk_filename(8), Some(TOWNE_TLK_FILENAME));
        assert_eq!(npc_tlk_filename(16), Some(DWELLING_TLK_FILENAME));
        assert_eq!(npc_tlk_filename(24), Some(CASTLE_TLK_FILENAME));
        assert_eq!(npc_tlk_filename(32), Some(KEEP_TLK_FILENAME));
    }

    #[test]
    fn resurrection_rebuilt_current_hp_is_one() {
        // magic.md §8: the spell helper stands the resurrected member
        // up with exactly one hit point; healer callers may top up
        // afterwards.
        assert_eq!(RESURRECTION_REBUILT_CURRENT_HP, 1);
    }

    #[test]
    fn tlk_no_keyword_match_message_matches_published_envelope() {
        // conversation.md §6
        assert_eq!(
            TLK_NO_KEYWORD_MATCH_MESSAGE,
            "I cannot help thee with that."
        );
    }

    #[test]
    fn sceptre_barrier_constants_match_catalog() {
        // catalogs/item-list.md §8
        assert_eq!(SCEPTRE_BARRIER_TILE_FIRST, 0x70);
        assert_eq!(SCEPTRE_BARRIER_TILE_LAST, 0x7F);
        assert_eq!(SCEPTRE_BARRIER_DISSOLVED_TILE, 0x44);
    }

    #[test]
    fn white_potion_sweep_constants_match_catalog() {
        // catalogs/item-list.md §7.2
        assert_eq!(POTION_WHITE_SWEEP_FRAMES, 20);
        assert_eq!(POTION_WHITE_SWEEP_RADIUS, 32);
    }

    #[test]
    fn active_effect_tag_scroll_install_counters_match_spec() {
        // inventory.md §7
        assert_eq!(
            ActiveEffectTag::Protection.scroll_install_counter(),
            Some(100)
        );
        assert_eq!(
            ActiveEffectTag::NegateMagic.scroll_install_counter(),
            Some(20)
        );
        assert_eq!(
            ActiveEffectTag::NegateTime.scroll_install_counter(),
            Some(20)
        );
        // No shipped scroll variants for Quickness or Mass Charm.
        assert_eq!(
            ActiveEffectTag::Quickness.scroll_install_counter(),
            None
        );
        assert_eq!(
            ActiveEffectTag::MassCharm.scroll_install_counter(),
            None
        );
        // Sanity: scroll vs spell counters differ where both exist.
        assert_ne!(
            ActiveEffectTag::Protection.scroll_install_counter(),
            ActiveEffectTag::Protection.spell_install_counter()
        );
        assert_ne!(
            ActiveEffectTag::NegateMagic.scroll_install_counter(),
            ActiveEffectTag::NegateMagic.spell_install_counter()
        );
    }

    #[test]
    fn spell_combat_field_kind_dispatches_per_spec_table() {
        // magic.md §8 combat field-kind dispatch table.
        assert_eq!(spell_combat_field_kind(FIRE_FIELD_SPELL_INDEX), Some(0x35));
        assert_eq!(
            spell_combat_field_kind(POISON_FIELD_SPELL_INDEX),
            Some(0x33)
        );
        assert_eq!(spell_combat_field_kind(SLEEP_FIELD_SPELL_INDEX), Some(0x34));
        assert_eq!(spell_combat_field_kind(ENERGY_FIELD_SPELL_INDEX), Some(0x36));
        // Dispel Field has its own removal helper; not a combat field
        // placement.
        assert_eq!(spell_combat_field_kind(DISPEL_FIELD_SPELL_INDEX), None);
        // Spot-check unrelated spell ids.
        for idx in [0usize, 1, 2, 5, 12, 13, 17, 19, 21, 47] {
            assert_eq!(
                spell_combat_field_kind(idx),
                None,
                "spell {idx} should not place a combat field"
            );
        }
    }

    #[test]
    fn tlk_keyword_loop_envelope_strings_match_spec() {
        // conversation.md §6
        assert_eq!(TLK_KEYWORD_PROMPT, "Your interest?\n:");
        assert_eq!(TLK_EMPTY_INPUT_BYE_MESSAGE, "BYE\n\n");
    }

    #[test]
    fn scroll_spell_labels_match_inventory_md_dispatch_order() {
        // inventory.md §7 + formats/saved-gam.md §7
        assert_eq!(SCROLL_SPELL_LABELS.len(), SCROLL_COUNT);
        assert_eq!(
            SCROLL_SPELL_LABELS,
            ["LV", "HR", "IS", "AI", "IQW", "CKX", "CIM", "AT"]
        );
    }

    #[test]
    fn tavern_drink_prompt_routes_y_n_space_and_discards_others() {
        // shops.md §8.5
        assert_eq!(tavern_drink_prompt(b'Y'), TavernDrinkPrompt::Enter);
        assert_eq!(tavern_drink_prompt(b'y'), TavernDrinkPrompt::Enter);
        assert_eq!(tavern_drink_prompt(b'N'), TavernDrinkPrompt::Leave);
        assert_eq!(tavern_drink_prompt(b'n'), TavernDrinkPrompt::Leave);
        assert_eq!(tavern_drink_prompt(b' '), TavernDrinkPrompt::Leave);
        for byte in [b'A', b'M', b'X', b'1', b'\r', b'\n', 0x00, 0x08, 0x1B, 0xFF] {
            assert_eq!(
                tavern_drink_prompt(byte),
                TavernDrinkPrompt::Discard,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn guild_shop_action_routes_a_b_c_and_exits_on_anything_else() {
        // shops.md §8.2
        assert_eq!(
            guild_shop_action(b'a'),
            GuildShopAction::Purchase(GuildCommodity::Keys)
        );
        assert_eq!(
            guild_shop_action(b'A'),
            GuildShopAction::Purchase(GuildCommodity::Keys)
        );
        assert_eq!(
            guild_shop_action(b'b'),
            GuildShopAction::Purchase(GuildCommodity::Gems)
        );
        assert_eq!(
            guild_shop_action(b'B'),
            GuildShopAction::Purchase(GuildCommodity::Gems)
        );
        assert_eq!(
            guild_shop_action(b'c'),
            GuildShopAction::Purchase(GuildCommodity::Torches)
        );
        assert_eq!(
            guild_shop_action(b'C'),
            GuildShopAction::Purchase(GuildCommodity::Torches)
        );
        for byte in [b'd', b'D', b' ', b'X', b'Y', b'N', b'\r', 0x00, 0x1B, 0xFF] {
            assert_eq!(
                guild_shop_action(byte),
                GuildShopAction::Exit,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn sage_rumour_topic_count_matches_resident_table_size() {
        // shops.md §8.8: shipped resident topic list contains 26 rumour
        // topics.
        assert_eq!(SAGE_RUMOUR_TOPIC_COUNT, 26);
        // Sanity: the input cap stays at 15 chars for free-text entry.
        assert_eq!(SAGE_TOPIC_INPUT_LIMIT, 15);
    }

    #[test]
    fn inn_main_action_routes_r_l_p_exit_and_discard() {
        // shops.md §8.4
        assert_eq!(inn_main_action(b'R'), InnMainAction::Rest);
        assert_eq!(inn_main_action(b'r'), InnMainAction::Rest);
        assert_eq!(inn_main_action(b'L'), InnMainAction::LeaveCompanion);
        assert_eq!(inn_main_action(b'l'), InnMainAction::LeaveCompanion);
        assert_eq!(inn_main_action(b'P'), InnMainAction::PickUpCompanion);
        assert_eq!(inn_main_action(b'p'), InnMainAction::PickUpCompanion);
        assert_eq!(inn_main_action(b' '), InnMainAction::Exit);
        assert_eq!(inn_main_action(0x1B), InnMainAction::Exit);
        for byte in [b'A', b'B', b'C', b'X', b'Y', b'N', b'\r', 0x00, 0x08, 0xFF] {
            assert_eq!(
                inn_main_action(byte),
                InnMainAction::Discard,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn arms_shop_action_routes_b_s_and_exits_on_anything_else() {
        // shops.md §8.1
        assert_eq!(arms_shop_action(b'B'), ArmsShopAction::Buy);
        assert_eq!(arms_shop_action(b'b'), ArmsShopAction::Buy);
        assert_eq!(arms_shop_action(b'S'), ArmsShopAction::Sell);
        assert_eq!(arms_shop_action(b's'), ArmsShopAction::Sell);
        // Anything else — Space, other letters, control bytes — exits.
        for byte in [b' ', b'A', b'X', b'Y', b'N', b'\r', 0x00, 0x1B, 0xFF] {
            assert_eq!(
                arms_shop_action(byte),
                ArmsShopAction::Exit,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn healer_service_action_accepts_c_h_r_and_exit_keys() {
        // shops.md §8.3
        assert_eq!(
            healer_service_action(b'C'),
            HealerServiceAction::Treatment(HealerTreatment::Cure)
        );
        assert_eq!(
            healer_service_action(b'c'),
            HealerServiceAction::Treatment(HealerTreatment::Cure)
        );
        assert_eq!(
            healer_service_action(b'H'),
            HealerServiceAction::Treatment(HealerTreatment::Heal)
        );
        assert_eq!(
            healer_service_action(b'h'),
            HealerServiceAction::Treatment(HealerTreatment::Heal)
        );
        assert_eq!(
            healer_service_action(b'R'),
            HealerServiceAction::Treatment(HealerTreatment::Resurrect)
        );
        assert_eq!(
            healer_service_action(b'r'),
            HealerServiceAction::Treatment(HealerTreatment::Resurrect)
        );
        assert_eq!(healer_service_action(b' '), HealerServiceAction::Exit);
        assert_eq!(healer_service_action(b'\r'), HealerServiceAction::Exit);
        assert_eq!(healer_service_action(b'\n'), HealerServiceAction::Exit);
        for byte in [b'A', b'D', b'M', b'S', b'X', b'Y', b'N', 0x00, 0x08, 0x1B, 0xFF] {
            assert_eq!(
                healer_service_action(byte),
                HealerServiceAction::Discard,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn shipwright_menu_action_accepts_f_s_and_exit_keys() {
        // shops.md §8.7
        assert_eq!(
            shipwright_menu_action(b'F'),
            ShipwrightMenuAction::Purchase(ShipwrightPurchaseKind::Frigate)
        );
        assert_eq!(
            shipwright_menu_action(b'f'),
            ShipwrightMenuAction::Purchase(ShipwrightPurchaseKind::Frigate)
        );
        assert_eq!(
            shipwright_menu_action(b'S'),
            ShipwrightMenuAction::Purchase(ShipwrightPurchaseKind::Skiff)
        );
        assert_eq!(
            shipwright_menu_action(b's'),
            ShipwrightMenuAction::Purchase(ShipwrightPurchaseKind::Skiff)
        );
        assert_eq!(shipwright_menu_action(b' '), ShipwrightMenuAction::Exit);
        assert_eq!(shipwright_menu_action(0x1B), ShipwrightMenuAction::Exit);
        // Any other key silently re-prompts.
        for byte in [b'A', b'M', b'Y', b'N', b'\r', b'\n', 0x00, 0x08, 0x09, 0xFF] {
            assert_eq!(
                shipwright_menu_action(byte),
                ShipwrightMenuAction::Discard,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn text_window_clamp_rectangle_normalises_per_spec() {
        // text-output.md §9: clamp X to 0..=39, Y to 0..=24, then
        // swap inverted pairs to enforce top_left <= bottom_right.
        // Ordinary in-range rectangle round-trips.
        assert_eq!(
            text_window_clamp_rectangle(6, 5, 33, 18),
            (6, 5, 33, 18)
        );
        // Inverted X pair: swap so left <= right.
        assert_eq!(
            text_window_clamp_rectangle(30, 5, 10, 18),
            (10, 5, 30, 18)
        );
        // Inverted Y pair: swap so top <= bottom.
        assert_eq!(
            text_window_clamp_rectangle(0, 24, 39, 0),
            (0, 0, 39, 24)
        );
        // Out-of-range X clamps to 39.
        assert_eq!(
            text_window_clamp_rectangle(50, 0, 200, 12),
            (39, 0, 39, 12)
        );
        // Out-of-range Y clamps to 24.
        assert_eq!(
            text_window_clamp_rectangle(0, 30, 10, 50),
            (0, 24, 10, 24)
        );
        // Full screen.
        assert_eq!(
            text_window_clamp_rectangle(0, 0, 39, 24),
            (0, 0, 39, 24)
        );
        // Single-cell rectangle.
        assert_eq!(
            text_window_clamp_rectangle(20, 12, 20, 12),
            (20, 12, 20, 12)
        );
    }

    #[test]
    fn ship_boarding_warns_low_hull_below_ten() {
        // vehicles.md §4: low-hull warning fires strictly below 10.
        assert_eq!(SHIP_HULL_BOARDING_WARNING_THRESHOLD, 10);
        for hull in 0u8..10 {
            assert!(
                ship_boarding_warns_low_hull(hull),
                "hull {hull} should warn"
            );
        }
        for hull in 10u8..=20 {
            assert!(
                !ship_boarding_warns_low_hull(hull),
                "hull {hull} should not warn"
            );
        }
        // The shipped Frigate-purchase hull starts at 100 — no warning.
        assert!(!ship_boarding_warns_low_hull(100));
    }

    #[test]
    fn ship_sail_toggle_marker_round_trips_hoisted_and_furled() {
        // vehicles.md §6: hoisted 0x20..=0x23 toggles to furled
        // 0x24..=0x27 and vice-versa, preserving the heading
        // encoded in the low two bits.
        for hoisted in 0x20u8..=0x23 {
            let furled = ship_sail_toggle_marker(hoisted).expect("hoisted -> furled");
            assert_eq!(furled, hoisted + 4);
            // Round-trip back to the original marker.
            assert_eq!(ship_sail_toggle_marker(furled), Some(hoisted));
        }
        // Non-ship markers fall through.
        for marker in [
            0x00u8, 0x10, 0x12, 0x14, 0x17, 0x1C, 0x1F, 0x28, 0x2B, 0x2C, 0xFF,
        ] {
            assert_eq!(
                ship_sail_toggle_marker(marker),
                None,
                "marker {marker:#04x} should not toggle"
            );
        }
        // Heading is preserved across the toggle (low two bits stay).
        assert_eq!(
            ship_sail_toggle_marker(0x22).unwrap() & 0x03,
            0x22 & 0x03
        );
        assert_eq!(
            ship_sail_toggle_marker(0x25).unwrap() & 0x03,
            0x25 & 0x03
        );
    }

    #[test]
    fn ship_xit_refused_under_sail_only_for_hoisted_range() {
        // vehicles.md §5
        for marker in 0x20u8..=0x23 {
            assert!(
                ship_xit_refused_under_sail(marker),
                "marker {marker:#04x} should refuse X-Xit under sail"
            );
        }
        // Furled-ship markers must accept X-Xit (subject to landing checks).
        for marker in 0x24u8..=0x27 {
            assert!(
                !ship_xit_refused_under_sail(marker),
                "marker {marker:#04x} (furled) should not refuse"
            );
        }
        // Horse, carpet, foot, skiff, and out-of-range bytes never trigger
        // the sail refusal.
        for marker in [
            0x00u8, 0x10, 0x11, 0x12, 0x14, 0x17, 0x1C, 0x1F, 0x28, 0x2B, 0x2C, 0xFF,
        ] {
            assert!(
                !ship_xit_refused_under_sail(marker),
                "marker {marker:#04x} should not refuse"
            );
        }
    }

    #[test]
    fn cardinal_direction_prompt_action_accepts_cardinals_and_space() {
        // input.md §10,§11
        assert_eq!(
            cardinal_direction_prompt_action(INPUT_CODE_NORTH),
            CardinalPromptAction::Cardinal(InputDirection::North)
        );
        assert_eq!(
            cardinal_direction_prompt_action(INPUT_CODE_SOUTH),
            CardinalPromptAction::Cardinal(InputDirection::South)
        );
        assert_eq!(
            cardinal_direction_prompt_action(INPUT_CODE_EAST),
            CardinalPromptAction::Cardinal(InputDirection::East)
        );
        assert_eq!(
            cardinal_direction_prompt_action(INPUT_CODE_WEST),
            CardinalPromptAction::Cardinal(InputDirection::West)
        );
        assert_eq!(
            cardinal_direction_prompt_action(b' '),
            CardinalPromptAction::Pass
        );
        // Diagonals, letters, function keys, control bytes — all ignored.
        for byte in [
            INPUT_CODE_NORTHWEST,
            INPUT_CODE_NORTHEAST,
            INPUT_CODE_SOUTHWEST,
            INPUT_CODE_SOUTHEAST,
            INPUT_CODE_F1,
            INPUT_CODE_F10,
            b'A',
            b'1',
            0x00,
            0x08,
            0x0D,
            0x1B,
            0xFF,
        ] {
            assert_eq!(
                cardinal_direction_prompt_action(byte),
                CardinalPromptAction::Ignored,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn story_dat_file_constant_is_published() {
        // formats/story-dat.md §2: published filename.
        assert_eq!(STORY_DAT_FILE, "STORY.DAT");
    }

    #[test]
    fn days_per_year_matches_thirteen_month_calendar() {
        // time.md §12: 13 months x 28 days = 364 days/year.
        assert_eq!(DAYS_PER_YEAR, 364);
        assert_eq!(
            DAYS_PER_YEAR,
            DAYS_PER_MONTH as u16 * MONTHS_PER_YEAR as u16
        );
    }

    #[test]
    fn rest_duration_input_classifies_digits_cancel_and_discard() {
        // rest-and-camp.md §4
        for digit in 1u8..=9 {
            assert_eq!(
                rest_duration_input(b'0' + digit),
                RestDurationInput::Hours(digit)
            );
        }
        assert_eq!(rest_duration_input(b'0'), RestDurationInput::Cancel);
        assert_eq!(rest_duration_input(b' '), RestDurationInput::Cancel);
        // Other bytes silently re-prompt.
        for byte in [
            0x00u8, 0x08, 0x0D, 0x1B, b'a', b'A', b'/', b':', b'.', 0x7F, 0xFF,
        ] {
            assert_eq!(
                rest_duration_input(byte),
                RestDurationInput::Discard,
                "byte {byte:#04x}"
            );
        }
    }

    #[test]
    fn chargen_name_input_limit_is_one_below_record_name_field() {
        // chargen.md §4: the prompt accepts up to 8 visible characters;
        // the save record's name field is 9 bytes so the trailing byte
        // stays as seed padding when the player fills the prompt.
        assert_eq!(CHARGEN_NAME_INPUT_LIMIT, 8);
        assert_eq!(CHARGEN_NAME_INPUT_LIMIT, SAVE_CHARACTER_NAME_LEN - 1);
    }

    #[test]
    fn question_dat_dilemma_record_for_pair_walks_lexicographic_order() {
        // formats/question-dat.md §4 spot-checks against the published
        // ordinal-to-pair table.
        assert_eq!(QUESTION_DAT_FILE, "QUESTION.DAT");
        assert_eq!(question_dat_dilemma_record_for_pair(0, 1), Some(2));
        assert_eq!(question_dat_dilemma_record_for_pair(0, 7), Some(8));
        assert_eq!(question_dat_dilemma_record_for_pair(1, 2), Some(9));
        assert_eq!(question_dat_dilemma_record_for_pair(1, 7), Some(14));
        assert_eq!(question_dat_dilemma_record_for_pair(2, 3), Some(15));
        assert_eq!(question_dat_dilemma_record_for_pair(3, 4), Some(20));
        assert_eq!(question_dat_dilemma_record_for_pair(4, 5), Some(24));
        assert_eq!(question_dat_dilemma_record_for_pair(5, 6), Some(27));
        assert_eq!(question_dat_dilemma_record_for_pair(6, 7), Some(29));
        // Diagonal/inverted/out-of-range inputs are unrepresentable.
        assert_eq!(question_dat_dilemma_record_for_pair(0, 0), None);
        assert_eq!(question_dat_dilemma_record_for_pair(5, 5), None);
        assert_eq!(question_dat_dilemma_record_for_pair(7, 0), None);
        assert_eq!(question_dat_dilemma_record_for_pair(7, 8), None);
        assert_eq!(question_dat_dilemma_record_for_pair(0, 8), None);
        // Walking the full pair set covers every dilemma record exactly once
        // in monotonically increasing order.
        let mut expected = QUESTION_DAT_FIRST_DILEMMA_RECORD;
        for lo in 0usize..7 {
            for hi in (lo + 1)..=7 {
                assert_eq!(
                    question_dat_dilemma_record_for_pair(lo, hi),
                    Some(expected),
                    "({lo}, {hi})"
                );
                expected += 1;
            }
        }
        assert_eq!(
            expected,
            QUESTION_DAT_FIRST_DILEMMA_RECORD + QUESTION_DAT_DILEMMA_COUNT
        );
    }

    #[test]
    fn input_prompt_mode_active_only_for_printable_ascii() {
        // input.md §2: prompt mode is open iff the prompt-character
        // byte is printable ASCII (0x20..=0x7E). Idle mode otherwise.
        for byte in 0x20u8..=0x7E {
            assert!(
                input_prompt_mode_active(byte),
                "{byte:#04x} should be prompt mode"
            );
        }
        // Common idle-mode sentinel choices: 0x00, low-ASCII control,
        // 0x7F, and the high-bit range used by direction/function codes.
        for byte in [0x00u8, 0x01, 0x07, 0x08, 0x0D, 0x1B, 0x1F, 0x7F, 0x80, 0xC9, 0xFB, 0xFF] {
            assert!(
                !input_prompt_mode_active(byte),
                "{byte:#04x} should be idle mode"
            );
        }
    }

    #[test]
    fn town_dawn_dusk_gate_toggle_round_trips_shipped_pair() {
        // town-mode.md §6
        assert_eq!(TOWN_DAWN_DUSK_GATE_MARKER_TILE, 0x87);
        assert_eq!(TOWN_DAWN_DUSK_GATE_TOGGLE_XOR, 0xDD);
        assert_eq!(TOWN_DAWN_DUSK_GATE_OPEN_TILE, 0x44);
        assert_eq!(TOWN_DAWN_DUSK_GATE_CLOSED_TILE, 0x99);
        // Cobble <-> portcullis.
        assert_eq!(
            town_dawn_dusk_gate_toggle(TOWN_DAWN_DUSK_GATE_OPEN_TILE),
            TOWN_DAWN_DUSK_GATE_CLOSED_TILE
        );
        assert_eq!(
            town_dawn_dusk_gate_toggle(TOWN_DAWN_DUSK_GATE_CLOSED_TILE),
            TOWN_DAWN_DUSK_GATE_OPEN_TILE
        );
        // Idempotent on a second pass.
        for byte in [0x00u8, 0x44, 0x55, 0x99, 0xAB, 0xFF] {
            assert_eq!(
                town_dawn_dusk_gate_toggle(town_dawn_dusk_gate_toggle(byte)),
                byte,
                "second toggle should restore byte {byte:#04x}"
            );
        }
        // Hour-change boundaries: 5 (out of band) and 20 (into band).
        assert!(town_dawn_dusk_gate_pass_fires_at_hour(5));
        assert!(town_dawn_dusk_gate_pass_fires_at_hour(20));
        for hour in [0u8, 1, 4, 6, 12, 19, 21, 23] {
            assert!(
                !town_dawn_dusk_gate_pass_fires_at_hour(hour),
                "hour {hour} should not fire the gate pass"
            );
        }
    }

    #[test]
    fn sextant_coordinate_letters_split_byte_into_two_letters() {
        // magic.md §8: high and low nibbles map to letters A..=P.
        assert_eq!(sextant_coordinate_letters(0x00), (b'A', b'A'));
        assert_eq!(sextant_coordinate_letters(0x0F), (b'A', b'P'));
        assert_eq!(sextant_coordinate_letters(0xF0), (b'P', b'A'));
        assert_eq!(sextant_coordinate_letters(0xFF), (b'P', b'P'));
        // Mid-range: byte 0x86 -> high nibble 8 -> 'I', low nibble 6 -> 'G'.
        assert_eq!(sextant_coordinate_letters(0x86), (b'I', b'G'));
        assert_eq!(sextant_coordinate_letters(138), (b'I', b'K'));
        // Sanity: every byte input lands in the A..=P square.
        for byte in 0u8..=255 {
            let (high, low) = sextant_coordinate_letters(byte);
            assert!((b'A'..=b'P').contains(&high), "byte {byte:#04x} high {high}");
            assert!((b'A'..=b'P').contains(&low), "byte {byte:#04x} low {low}");
        }
    }

    #[test]
    fn npc_file_arithmetic_helpers_match_spec_strides() {
        // formats/npc.md §3,§4
        assert_eq!(npc_sub_map_offset(0), 0);
        assert_eq!(npc_sub_map_offset(1), NPC_SUB_MAP_LEN);
        assert_eq!(npc_sub_map_offset(7), 7 * NPC_SUB_MAP_LEN);
        // The eight-sub-map total equals the file length.
        assert_eq!(
            npc_sub_map_offset(NPC_SUB_MAPS_PER_FILE),
            NPC_FILE_LEN
        );
        // Schedule record stride within a sub-map.
        assert_eq!(npc_schedule_record_offset(0, 0), 0);
        assert_eq!(npc_schedule_record_offset(0, 1), NPC_SCHEDULE_RECORD_LEN);
        assert_eq!(
            npc_schedule_record_offset(2, 5),
            2 * NPC_SUB_MAP_LEN + 5 * NPC_SCHEDULE_RECORD_LEN
        );
        // Type byte sits inside the type array at sub-map offset
        // NPC_TYPE_ARRAY_OFFSET.
        assert_eq!(npc_type_byte_offset(0, 0), NPC_TYPE_ARRAY_OFFSET);
        assert_eq!(
            npc_type_byte_offset(3, 7),
            3 * NPC_SUB_MAP_LEN + NPC_TYPE_ARRAY_OFFSET + 7
        );
        // Dialog-index byte sits inside the dialog array.
        assert_eq!(npc_dialog_index_offset(0, 0), NPC_DIALOG_ARRAY_OFFSET);
        assert_eq!(
            npc_dialog_index_offset(1, 31),
            NPC_SUB_MAP_LEN + NPC_DIALOG_ARRAY_OFFSET + 31
        );
    }

    #[test]
    fn miscmsg_dat_file_constant_is_published() {
        // formats/miscmsg-dat.md §2: published filename for the
        // Blackthorn audience cluster + shrine/Codex prophecy file.
        assert_eq!(MISCMSG_DAT_FILE, "MISCMSG.DAT");
    }

    #[test]
    fn tlk_if_else_alt_branches_is_at_or_above_threshold() {
        // conversation.md §7.6: 0xFE IF-ELSE-ALT branches when the
        // shared moral-standing selector is at or above the threshold.
        assert!(tlk_if_else_alt_branches(0, 0));
        assert!(tlk_if_else_alt_branches(50, 0));
        assert!(tlk_if_else_alt_branches(50, 50));
        assert!(tlk_if_else_alt_branches(99, 75));
        assert!(!tlk_if_else_alt_branches(74, 75));
        assert!(!tlk_if_else_alt_branches(0, 1));
        // Saturate at the byte cap.
        assert!(tlk_if_else_alt_branches(255, 255));
        assert!(!tlk_if_else_alt_branches(254, 255));
    }

    #[test]
    fn outdoor_active_object_class_immobile_matches_spec_range() {
        // movement.md §4: only 0xE8..=0xEB is the immobile / never-pass
        // family.
        for cls in 0xE8u8..=0xEB {
            assert!(
                outdoor_active_object_class_immobile(cls),
                "{cls:#04x} should be immobile"
            );
        }
        // Adjacent ranges all have other movement predicates.
        for cls in [0xE0u8, 0xE3, 0xE4, 0xE7, 0xEC, 0xEF, 0xF4, 0xFB, 0x00, 0x80, 0xFF] {
            assert!(
                !outdoor_active_object_class_immobile(cls),
                "{cls:#04x} should not be immobile"
            );
        }
        // Sanity: an immobile class never returns a single-tile
        // acceptance via the single-tile-query helper either.
        for cls in 0xE8u8..=0xEB {
            assert_eq!(outdoor_active_object_single_tile_query(cls), None);
        }
    }

    #[test]
    fn text_window_inner_width_and_centred_start_match_spec() {
        // text-output.md §4: window whose corners are columns 6 and 33
        // has width 27 (27 chars fit before wrapping).
        assert_eq!(text_window_inner_width(6, 33), 27);
        assert_eq!(text_window_inner_width(0, 39), 39);
        // Inverted corners collapse to zero.
        assert_eq!(text_window_inner_width(20, 10), 0);
        assert_eq!(text_window_inner_width(15, 15), 0);

        // text-output.md §5: centred start column = (width - line) / 2.
        assert_eq!(text_window_centred_start_column(27, 7), 10);
        assert_eq!(text_window_centred_start_column(27, 27), 0);
        // Lines wider than the window collapse to column 0 rather than
        // producing a negative offset.
        assert_eq!(text_window_centred_start_column(27, 30), 0);
        // Even split.
        assert_eq!(text_window_centred_start_column(40, 10), 15);
    }

    #[test]
    fn active_object_slot_role_partitions_table_per_spec() {
        // active-objects.md §4
        assert_eq!(
            active_object_slot_role(ACTIVE_OBJECT_PLAYER_SLOT),
            Some(ActiveObjectSlotRole::Player)
        );
        for slot in ACTIVE_OBJECT_ORDINARY_FIRST..=ACTIVE_OBJECT_ORDINARY_LAST {
            assert_eq!(
                active_object_slot_role(slot),
                Some(ActiveObjectSlotRole::OrdinaryAcquisition),
                "slot {slot} should be OrdinaryAcquisition"
            );
        }
        for slot in ACTIVE_OBJECT_RESERVED_FIRST..=ACTIVE_OBJECT_RESERVED_LAST {
            assert_eq!(
                active_object_slot_role(slot),
                Some(ActiveObjectSlotRole::Reserved),
                "slot {slot} should be Reserved"
            );
        }
        // Out-of-range indices return None.
        assert_eq!(active_object_slot_role(OOL_SLOTS), None);
        assert_eq!(active_object_slot_role(64), None);
        assert_eq!(active_object_slot_role(usize::MAX), None);
    }

    #[test]
    fn amulet_turning_scatter_threshold_is_byte_midpoint() {
        // combat.md §11
        assert_eq!(AMULET_TURNING_SCATTER_THRESHOLD, 128);
        // The scatter gate fires only when every prerequisite holds and the
        // roll is strictly below the threshold.
        assert!(resolve_amulet_turning_scatter(true, true, true, 0));
        assert!(resolve_amulet_turning_scatter(true, true, true, 127));
        assert!(!resolve_amulet_turning_scatter(true, true, true, 128));
        assert!(!resolve_amulet_turning_scatter(true, true, true, 255));
        // Any missing precondition skips the scatter path, even on a low roll.
        assert!(!resolve_amulet_turning_scatter(false, true, true, 0));
        assert!(!resolve_amulet_turning_scatter(true, false, true, 0));
        assert!(!resolve_amulet_turning_scatter(true, true, false, 0));
    }

    #[test]
    fn terrain_combat_replacement_roll_one_in_nine() {
        // encounters.md §4 + combat.md §5
        assert_eq!(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR, 9);
        // Walk a full 0..=255 seed domain. Exactly the multiples of 9
        // should pick the replacement tile.
        let mut hits = 0;
        for seed in 0u8..=255 {
            let picks = terrain_combat_replacement_roll_picks_replacement(seed);
            assert_eq!(picks, seed % 9 == 0, "seed {seed}");
            if picks {
                hits += 1;
            }
        }
        // 256 seeds; multiples of 9 in 0..=255 are 0, 9, 18, ... 252 → 29
        // values.
        assert_eq!(hits, 29);
    }

    #[test]
    fn combat_arena_file_offsets_match_spec_arithmetic() {
        // formats/cbt.md §3: arena_start = arena_index * 352;
        // row_start = arena_start + row * 32.
        assert_eq!(COMBAT_ARENA_RECORD_LEN, 352);
        assert_eq!(COMBAT_ARENA_ROW_STRIDE, 32);
        assert_eq!(combat_arena_file_offset(0), 0);
        assert_eq!(combat_arena_file_offset(1), 352);
        assert_eq!(combat_arena_file_offset(15), 15 * 352);
        // The 16-arena BRIT.CBT total length equals the next index's
        // base offset.
        assert_eq!(combat_arena_file_offset(BRIT_CBT_RECORDS), BRIT_CBT_FILE_LEN);
        // Row offsets within an arena.
        assert_eq!(combat_arena_row_offset(0, 0), 0);
        assert_eq!(combat_arena_row_offset(0, 5), 5 * 32);
        assert_eq!(combat_arena_row_offset(2, 7), 2 * 352 + 7 * 32);
        // The last row's start sits at offset 352 - 32 within the arena.
        assert_eq!(
            combat_arena_row_offset(0, COMBAT_ARENA_SIDE - 1),
            COMBAT_ARENA_RECORD_LEN - COMBAT_ARENA_ROW_STRIDE
        );
    }

    #[test]
    fn u4_transfer_published_filenames_match_spec() {
        // u4-transfer.md §5,§11
        assert_eq!(U4_TRANSFER_U5_SEED_GAM_FILENAME, "BRIT.GAM");
        assert_eq!(U4_TRANSFER_U5_SEED_OOL_FILENAME, "BRIT.OOL");
        assert_eq!(U4_TRANSFER_U4_SOURCE_FILENAME, "PARTY.SAV");
    }

    #[test]
    fn dungeon_scene_for_word_of_power_inverts_word_table() {
        // catalogs/quest-graph.md §4
        for scene in 33u8..=40 {
            let word = dungeon_word_of_power(scene).expect("word for dungeon scene");
            assert_eq!(
                dungeon_scene_for_word_of_power(word),
                Some(scene),
                "{word} should map back to scene {scene}"
            );
            // Case-insensitive: lowercase form rounds-trips too.
            let lower = word.to_ascii_lowercase();
            assert_eq!(dungeon_scene_for_word_of_power(&lower), Some(scene));
        }
        // Unknown words.
        assert_eq!(dungeon_scene_for_word_of_power(""), None);
        assert_eq!(dungeon_scene_for_word_of_power("DAWN"), None);
        assert_eq!(dungeon_scene_for_word_of_power("FALLAXX"), None);
        assert_eq!(dungeon_scene_for_word_of_power("FALLA"), None);
    }

    #[test]
    fn chargen_starting_map_tuple_matches_init_gam_seed() {
        // chargen.md §8: scene 13 (Iolo's Hut), saved-scene scratch 0,
        // floor/Z 0, X 15, Y 15.
        assert_eq!(CHARGEN_STARTING_SCENE, 13);
        assert_eq!(CHARGEN_STARTING_X, 15);
        assert_eq!(CHARGEN_STARTING_Y, 15);
        assert_eq!(CHARGEN_STARTING_Z, 0);
        assert_eq!(CHARGEN_STARTING_SAVED_SCENE_SCRATCH, 0);
        // Sanity: the starting scene is Iolo's Hut as named by the
        // town_resident_name catalog.
        assert_eq!(town_resident_name(CHARGEN_STARTING_SCENE), Some("Iolo's Hut"));
    }

    #[test]
    fn conversation_cleanup_gold_debit_lands_in_one_through_fifteen() {
        // quest-flags.md §5
        assert_eq!(CONVERSATION_CLEANUP_GOLD_DEBIT_MIN, 1);
        assert_eq!(CONVERSATION_CLEANUP_GOLD_DEBIT_MAX, 15);
        // Walk the full 0..=255 seed domain and verify every result
        // lands in 1..=15.
        let mut seen = [false; 15];
        for seed in 0u8..=255 {
            let amount = conversation_cleanup_gold_debit_amount(seed);
            assert!(
                (CONVERSATION_CLEANUP_GOLD_DEBIT_MIN..=CONVERSATION_CLEANUP_GOLD_DEBIT_MAX)
                    .contains(&amount),
                "seed {seed} produced {amount}"
            );
            seen[(amount - 1) as usize] = true;
        }
        assert!(
            seen.iter().all(|&v| v),
            "every gold-debit amount in 1..=15 should be reachable"
        );
        // Specific anchor values.
        assert_eq!(conversation_cleanup_gold_debit_amount(0), 1);
        assert_eq!(conversation_cleanup_gold_debit_amount(14), 15);
        assert_eq!(conversation_cleanup_gold_debit_amount(15), 1);
    }

    #[test]
    fn equipment_class_tag_slot_family_routes_per_spec() {
        // inventory.md §3.1
        assert_eq!(
            EquipmentClassTag::Helm.slot_family(),
            EquipmentSlotFamily::Helm
        );
        assert_eq!(
            EquipmentClassTag::BodyArmour.slot_family(),
            EquipmentSlotFamily::BodyArmour
        );
        assert_eq!(
            EquipmentClassTag::OneHand.slot_family(),
            EquipmentSlotFamily::Hand
        );
        assert_eq!(
            EquipmentClassTag::TwoHand.slot_family(),
            EquipmentSlotFamily::Hand
        );
        assert_eq!(
            EquipmentClassTag::Ring.slot_family(),
            EquipmentSlotFamily::Ring
        );
        assert_eq!(
            EquipmentClassTag::Amulet.slot_family(),
            EquipmentSlotFamily::Amulet
        );
        assert_eq!(
            EquipmentClassTag::None.slot_family(),
            EquipmentSlotFamily::None
        );
    }

    #[test]
    fn shadowlord_principle_round_trips_slot_and_flame() {
        // catalogs/quest-graph.md §5
        assert_eq!(
            shadowlord_principle_for_slot(0),
            Some(ShadowlordPrinciple::Falsehood)
        );
        assert_eq!(
            shadowlord_principle_for_slot(1),
            Some(ShadowlordPrinciple::Hatred)
        );
        assert_eq!(
            shadowlord_principle_for_slot(2),
            Some(ShadowlordPrinciple::Cowardice)
        );
        for slot in 3usize..16 {
            assert_eq!(shadowlord_principle_for_slot(slot), None);
        }
        for shadowlord in ShadowlordPrinciple::ALL {
            assert_eq!(
                shadowlord_principle_for_slot(shadowlord.slot()),
                Some(shadowlord)
            );
        }
        assert_eq!(
            ShadowlordPrinciple::Falsehood.eternal_flame(),
            EternalFlame::Truth
        );
        assert_eq!(
            ShadowlordPrinciple::Hatred.eternal_flame(),
            EternalFlame::Love
        );
        assert_eq!(
            ShadowlordPrinciple::Cowardice.eternal_flame(),
            EternalFlame::Courage
        );
        // Sanity: the typed enum's slot is consistent with the
        // existing slot-indexed eternal_flame_for_shadowlord helper.
        for shadowlord in ShadowlordPrinciple::ALL {
            assert_eq!(
                eternal_flame_for_shadowlord(shadowlord.slot()),
                Some(shadowlord.eternal_flame())
            );
        }
    }

    #[test]
    fn sailing_wait_pass_minutes_responds_to_hms_cape_rigging_flag() {
        // weather.md §5: ordinary sailing wait passes spend two
        // outdoor minutes; HMS Cape rigging halves that to one.
        assert_eq!(WindState::sailing_wait_pass_minutes(false), 2);
        assert_eq!(WindState::sailing_wait_pass_minutes(true), 1);
    }

    #[test]
    fn transport_marker_facing_decodes_low_two_bits_per_spec() {
        // vehicles.md §2: low two bits of a recognised transport
        // marker decode N/E/S/W as 0/1/2/3. The MountedHorse range
        // only contains the boarded markers 0x12/0x13 (the parked
        // horse objects 0x10/0x11 are not transport state), so the
        // only mounted-horse facings expressible through this helper
        // are South (bits 10) and West (bits 11).
        assert_eq!(transport_marker_facing(0x12), Some(Direction::South));
        assert_eq!(transport_marker_facing(0x13), Some(Direction::West));
        assert_eq!(transport_marker_facing(0x14), Some(Direction::North));
        assert_eq!(transport_marker_facing(0x17), Some(Direction::West));
        assert_eq!(transport_marker_facing(0x1C), Some(Direction::North));
        assert_eq!(transport_marker_facing(0x1F), Some(Direction::West));
        assert_eq!(transport_marker_facing(0x20), Some(Direction::North));
        assert_eq!(transport_marker_facing(0x22), Some(Direction::South));
        assert_eq!(transport_marker_facing(0x25), Some(Direction::East));
        assert_eq!(transport_marker_facing(0x2A), Some(Direction::South));
        // Out-of-range markers stay opaque per spec.
        for marker in [0x00u8, 0x0F, 0x10, 0x11, 0x18, 0x1B, 0x2C, 0xFF] {
            assert_eq!(
                transport_marker_facing(marker),
                None,
                "marker {marker:#04x} should be opaque"
            );
        }
    }

    #[test]
    fn overworld_klimb_entry_gate_orders_grapple_before_on_foot() {
        // doors-and-z-transitions.md §9: the Grapple-flag check is
        // consulted first; the vehicle gate is only reached when the
        // Grapple flag is set.
        assert_eq!(
            overworld_klimb_entry_gate(false, true),
            OverworldKlimbEntryGate::NoGrapple
        );
        // Without the Grapple flag the vehicle state does not matter.
        assert_eq!(
            overworld_klimb_entry_gate(false, false),
            OverworldKlimbEntryGate::NoGrapple
        );
        assert_eq!(
            overworld_klimb_entry_gate(true, false),
            OverworldKlimbEntryGate::NotOnFoot
        );
        assert_eq!(
            overworld_klimb_entry_gate(true, true),
            OverworldKlimbEntryGate::Proceed
        );
    }

    #[test]
    fn spell_field_placement_byte_maps_three_field_spells_per_spec() {
        // magic.md §8 + dungeon-mode.md §8: field-placement spells
        // 14/15/16 write energy-field cell bytes 0x82/0x81/0x80.
        assert_eq!(spell_field_placement_byte(FIRE_FIELD_SPELL_INDEX), Some(0x82));
        assert_eq!(
            spell_field_placement_byte(POISON_FIELD_SPELL_INDEX),
            Some(0x81)
        );
        assert_eq!(spell_field_placement_byte(SLEEP_FIELD_SPELL_INDEX), Some(0x80));
        // Dispel Field (18) removes a field rather than placing one;
        // the helper returns None.
        assert_eq!(spell_field_placement_byte(DISPEL_FIELD_SPELL_INDEX), None);
        // Spot-check a handful of unrelated spell ids.
        for idx in [0usize, 1, 2, 5, 12, 13, 17, 19, 47] {
            assert_eq!(
                spell_field_placement_byte(idx),
                None,
                "spell {idx} should not place a field"
            );
        }
    }

    #[test]
    fn intro_story_step_waits_for_input_skips_auto_opening_only() {
        // intro.md §10: step 0 is the automatic opening transition;
        // steps 1..=20 wait for a non-zero keyboard poll before
        // advancing. Steps outside 0..=20 are not produced by the loop.
        assert!(!intro_story_step_waits_for_input(INTRO_AUTO_OPENING_STEP));
        for step in 1usize..=20 {
            assert!(
                intro_story_step_waits_for_input(step),
                "step {step} should wait for input"
            );
        }
        assert_eq!(INTRO_STORY_STEP_COUNT, 21);
        // Out-of-range steps: not produced.
        for step in [INTRO_STORY_STEP_COUNT, 22, 50, 100] {
            assert!(
                !intro_story_step_waits_for_input(step),
                "step {step} is out of range"
            );
        }
    }

    #[test]
    fn dungeon_search_secret_pit_reveal_matches_spec_table() {
        // dungeon-mode.md §8: Search on 0x61 reports a secret door,
        // rewrites the cell to 0x60, and unless on the deepest level
        // stamps the visit bit on the same X/Y cell one level below.
        for z in 0u8..=6 {
            assert_eq!(
                dungeon_search_secret_pit_reveal(0x61, z),
                Some(DungeonSearchSecretPitReveal::RewriteAndStampLevelBelow),
                "z {z} should stamp level below"
            );
        }
        assert_eq!(
            dungeon_search_secret_pit_reveal(0x61, DUNGEON_LEVEL_BOTTOM),
            Some(DungeonSearchSecretPitReveal::RewriteOnly)
        );
        // Other bytes — including other pit-family bytes — do not take
        // this branch.
        for byte in [0x00u8, 0x60, 0x62, 0x68, 0x69, 0x6A, 0x6F, 0x71, 0xFF] {
            for z in 0u8..=7 {
                assert_eq!(
                    dungeon_search_secret_pit_reveal(byte, z),
                    None,
                    "byte {byte:#04x} z {z} should not reveal"
                );
            }
        }
    }

    #[test]
    fn endgame_campaign_start_baseline_matches_spec() {
        // endgame.md §9: the certificate elapsed-time baseline is
        // year 139, month 4, day 5.
        assert_eq!(ENDGAME_CAMPAIGN_START_YEAR, 139);
        assert_eq!(ENDGAME_CAMPAIGN_START_MONTH, 4);
        assert_eq!(ENDGAME_CAMPAIGN_START_DAY, 5);
        // A clock exactly at the baseline reports zero elapsed time.
        let baseline = GameClock {
            year: ENDGAME_CAMPAIGN_START_YEAR,
            month: ENDGAME_CAMPAIGN_START_MONTH,
            day: ENDGAME_CAMPAIGN_START_DAY,
            hour: 0,
            minute: 0,
        };
        assert_eq!(endgame_elapsed_campaign_time(baseline), (0, 0, 0));
        // One day after the baseline.
        let one_day_later = GameClock {
            year: ENDGAME_CAMPAIGN_START_YEAR,
            month: ENDGAME_CAMPAIGN_START_MONTH,
            day: ENDGAME_CAMPAIGN_START_DAY + 1,
            hour: 0,
            minute: 0,
        };
        assert_eq!(endgame_elapsed_campaign_time(one_day_later), (0, 0, 1));
        // Day-borrow case: clock day < baseline day. Reports one
        // less month and 28-day wrap.
        let day_borrow = GameClock {
            year: ENDGAME_CAMPAIGN_START_YEAR,
            month: ENDGAME_CAMPAIGN_START_MONTH + 1,
            day: ENDGAME_CAMPAIGN_START_DAY - 1,
            hour: 0,
            minute: 0,
        };
        let (y, m, d) = endgame_elapsed_campaign_time(day_borrow);
        assert_eq!((y, m, d), (0, 0, DAYS_PER_MONTH - 1));
    }

    #[test]
    fn npc_schedule_state_classify_round_trips_published_states() {
        // npc-schedules.md §7: the state byte takes values in 0..=8.
        let pairs: [(u8, NpcScheduleState); 9] = [
            (NPC_STATE_EMPTY, NpcScheduleState::Empty),
            (NPC_STATE_IDLE, NpcScheduleState::Idle),
            (NPC_STATE_INPLANE_MOVE, NpcScheduleState::InPlaneMove),
            (NPC_STATE_REPLAY_QUEUE, NpcScheduleState::ReplayQueue),
            (
                NPC_STATE_DESCEND_TOWARD_TARGET,
                NpcScheduleState::DescendTowardTarget,
            ),
            (
                NPC_STATE_ASCEND_TOWARD_TARGET,
                NpcScheduleState::AscendTowardTarget,
            ),
            (NPC_STATE_CLIMB_UP_OFF_FLOOR, NpcScheduleState::ClimbUpOffFloor),
            (
                NPC_STATE_CLIMB_DOWN_OFF_FLOOR,
                NpcScheduleState::ClimbDownOffFloor,
            ),
            (NPC_STATE_PARKED_OFF_FLOOR, NpcScheduleState::ParkedOffFloor),
        ];
        for (raw, state) in pairs {
            assert_eq!(npc_schedule_state_classify(raw), Some(state));
            assert_eq!(state.save_byte(), raw);
        }
        // Out-of-band bytes the engine's writers never produce.
        for byte in 9u8..=255 {
            assert_eq!(
                npc_schedule_state_classify(byte),
                None,
                "byte {byte} should be out-of-band"
            );
        }
    }

    #[test]
    fn input_byte_class_partitions_keyboard_return_bytes_per_spec() {
        // input.md §4: three non-overlapping return families plus the
        // catch-all "no key" outcome.

        // Regular ASCII: printable letters, digits, punctuation, and the
        // accepted control bytes (Enter 0x0D, Backspace 0x08, Escape 0x1B).
        for byte in [
            0x08u8, 0x0D, 0x1B, b' ', b'!', b'0', b'9', b'A', b'Z', b'a', b'z',
            b'~',
        ] {
            assert_eq!(
                input_byte_class(byte),
                InputByteClass::RegularAscii,
                "{byte:#04x} should be RegularAscii"
            );
        }

        // Function-key remap 0xC9..=0xD2.
        for byte in INPUT_CODE_FUNCTION_FIRST..=INPUT_CODE_FUNCTION_LAST {
            assert_eq!(
                input_byte_class(byte),
                InputByteClass::FunctionKey,
                "{byte:#04x} should be FunctionKey"
            );
        }

        // Direction codes: diagonals 0xD3..=0xD6 and cardinals 0xFB..=0xFE.
        for byte in [
            INPUT_CODE_NORTHWEST,
            INPUT_CODE_SOUTHWEST,
            INPUT_CODE_NORTHEAST,
            INPUT_CODE_SOUTHEAST,
            INPUT_CODE_WEST,
            INPUT_CODE_EAST,
            INPUT_CODE_NORTH,
            INPUT_CODE_SOUTH,
        ] {
            assert_eq!(
                input_byte_class(byte),
                InputByteClass::Direction,
                "{byte:#04x} should be Direction"
            );
        }

        // "No key" — unbound control bytes, the gap between function keys
        // and diagonals, the gap between diagonals and cardinals, and 0xFF.
        for byte in [0x00u8, 0x01, 0x07, 0x09, 0x1A, 0x7F, 0x80, 0xC8, 0xD7, 0xFA, 0xFF] {
            assert_eq!(
                input_byte_class(byte),
                InputByteClass::None,
                "{byte:#04x} should be None"
            );
        }
    }

    #[test]
    fn town_arrest_release_loop_constants_match_spec() {
        // time.md §10 town-arrest surrender: 20-minute cleanup increments
        // repeated until the hour byte reaches 08:00.
        assert_eq!(TOWN_ARREST_CLEANUP_INCREMENT_MINUTES, 20);
        assert_eq!(TOWN_ARREST_RELEASE_HOUR, 8);
        assert!(town_arrest_release_loop_done(8));
        for hour in 0u8..24 {
            if hour == TOWN_ARREST_RELEASE_HOUR {
                continue;
            }
            assert!(
                !town_arrest_release_loop_done(hour),
                "hour {hour} should keep the loop running"
            );
        }
    }

    #[test]
    fn sea_creature_spawn_aux_seed_only_for_water_creature_facing_frames() {
        // encounters.md §4: only the pirate-ship / water-creature facing-frame
        // family 0x2C..=0x2F receives the 100-unit aux seed at spawn time.
        // Every other class spawned by the random-encounter spawner uses the
        // default zero seed.
        assert_eq!(SEA_CREATURE_SPAWN_AUX_SEED, 100);
        for class in 0x2Cu8..=0x2F {
            assert!(
                sea_creature_spawn_seeds_aux(class),
                "class {class:#04x} should seed the wander aux"
            );
        }
        // Spot-check non-water-creature classes do not.
        for class in [
            0x00u8, 0x01, 0x2B, 0x30, 0x40, 0x80, 0x84, 0x88, 0x8C, 0x90, 0x94,
            0x98, 0xC0, 0xC4, 0xC8, 0xCC, 0xD0, 0xD4, 0xD8, 0xDC, 0xE0, 0xE4,
            0xEC, 0xF0, 0xF4, 0xF8, 0xFF,
        ] {
            assert!(
                !sea_creature_spawn_seeds_aux(class),
                "class {class:#04x} should not seed the wander aux"
            );
        }
    }

    #[test]
    fn tlk_byte_runner_class_partitions_byte_space_per_spec() {
        // conversation.md §7 top-level dispatcher ranges.
        assert_eq!(tlk_byte_runner_class(0x00), TlkByteRunnerClass::NullByte);
        // 0x01..=0x7F dictionary tokens.
        for byte in [0x01u8, 0x10, 0x40, 0x7F] {
            assert_eq!(
                tlk_byte_runner_class(byte),
                TlkByteRunnerClass::DictionaryToken,
                "{byte:#04x} should be DictionaryToken"
            );
        }
        // 0x80..=0x9D control codes — the 0x9E..=0x9F GOTO pair is
        // carved out below.
        for byte in [0x80u8, 0x83, 0x8C, 0x8E, 0x91, 0x9D] {
            assert_eq!(
                tlk_byte_runner_class(byte),
                TlkByteRunnerClass::ControlCode,
                "{byte:#04x} should be ControlCode"
            );
        }
        // 0x9E..=0x9F GOTO-LABEL.
        assert_eq!(tlk_byte_runner_class(0x9E), TlkByteRunnerClass::GotoLabel);
        assert_eq!(tlk_byte_runner_class(0x9F), TlkByteRunnerClass::GotoLabel);
        // 0xA0..=0xFD printable text path.
        for byte in [0xA0u8, 0xC1, 0xE5, 0xFD] {
            assert_eq!(
                tlk_byte_runner_class(byte),
                TlkByteRunnerClass::PrintableText,
                "{byte:#04x} should be PrintableText"
            );
        }
        // 0xFE IF/ELSE alias.
        assert_eq!(
            tlk_byte_runner_class(0xFE),
            TlkByteRunnerClass::IfElseAlias
        );
        // 0xFF end-of-response.
        assert_eq!(
            tlk_byte_runner_class(0xFF),
            TlkByteRunnerClass::EndOfResponse
        );
        // Cross-check the named control-code constants land in the
        // ControlCode branch (not the GotoLabel carve-out).
        for code in [
            TLK_CODE_PRINT_AVATAR_NAME,
            TLK_CODE_END_STREAM,
            TLK_CODE_PAUSE,
            TLK_CODE_PANEL_NEWLINE,
            TLK_CODE_CURSE_CHECK,
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_PROTECT_RUN,
            TLK_CODE_WAIT_KEY,
        ] {
            assert_eq!(
                tlk_byte_runner_class(code),
                TlkByteRunnerClass::ControlCode,
                "{code:#04x} should classify as ControlCode"
            );
        }
        // The GOTO-LABEL constants share the GotoLabel branch.
        assert_eq!(
            tlk_byte_runner_class(TLK_CODE_GOTO_LABEL_FIRST),
            TlkByteRunnerClass::GotoLabel
        );
        assert_eq!(
            tlk_byte_runner_class(TLK_CODE_GOTO_LABEL_LAST),
            TlkByteRunnerClass::GotoLabel
        );
    }

    #[test]
    fn rest_with_watch_recovers_hp_only_for_good_and_sleeping() {
        // rest-and-camp.md §5: HP recovery is for Good and Sleeping members
        // only. Poisoned members participate in the watch but do not receive
        // ordinary rest HP recovery. Charmed/Dead/Ashes are not in the
        // rest-participating set at all.
        assert!(rest_with_watch_recovers_hp(CharacterStatus::Good));
        assert!(rest_with_watch_recovers_hp(CharacterStatus::Sleeping));
        assert!(!rest_with_watch_recovers_hp(
            CharacterStatus::PoisonedOrRevived
        ));
        assert!(!rest_with_watch_recovers_hp(CharacterStatus::Charmed));
        assert!(!rest_with_watch_recovers_hp(CharacterStatus::Dead));
        assert!(!rest_with_watch_recovers_hp(CharacterStatus::Ashes));
        // Sanity: Poisoned still participates even though it does not
        // recover HP — the two predicates are independent.
        assert!(rest_with_watch_participates(
            CharacterStatus::PoisonedOrRevived
        ));
    }

    #[test]
    fn outdoor_water_creature_attack_aligned_orthogonal_within_three() {
        // active-objects.md §8: ship-like water-creature and pirate frames
        // trigger the attack message + water-creature step path when they
        // share a row or column with the player and the wrapped distance
        // along that axis is within three cells.
        assert_eq!(OUTDOOR_WATER_CREATURE_ADJACENCY_RADIUS, 3);
        // Same-column hits 1..=3 north and south.
        for dy in [-3, -2, -1, 1, 2, 3] {
            assert!(
                outdoor_water_creature_attack_aligned(0, dy),
                "column dy {dy} should align"
            );
        }
        // Same-row hits 1..=3 west and east.
        for dx in [-3, -2, -1, 1, 2, 3] {
            assert!(
                outdoor_water_creature_attack_aligned(dx, 0),
                "row dx {dx} should align"
            );
        }
        // Beyond the radius along the shared axis is not aligned.
        assert!(!outdoor_water_creature_attack_aligned(0, 4));
        assert!(!outdoor_water_creature_attack_aligned(0, -4));
        assert!(!outdoor_water_creature_attack_aligned(4, 0));
        assert!(!outdoor_water_creature_attack_aligned(-4, 0));
        // Zero displacement (same cell as the player) does not trigger.
        assert!(!outdoor_water_creature_attack_aligned(0, 0));
        // Diagonal offsets never trigger, regardless of distance.
        for dx in 1..=3 {
            for dy in 1..=3 {
                assert!(
                    !outdoor_water_creature_attack_aligned(dx, dy),
                    "diagonal ({dx},{dy}) should not align"
                );
                assert!(!outdoor_water_creature_attack_aligned(-dx, dy));
                assert!(!outdoor_water_creature_attack_aligned(dx, -dy));
                assert!(!outdoor_water_creature_attack_aligned(-dx, -dy));
            }
        }
    }

    #[test]
    fn combat_ambush_reveal_slots_max_matches_spec() {
        // combat.md §5
        assert_eq!(COMBAT_AMBUSH_REVEAL_SLOTS_MAX, 8);
    }

    #[test]
    fn sign_body_byte_kind_classifies_payload_per_spec() {
        // formats/signs-dat.md §4
        assert_eq!(sign_body_byte_kind(0x00), SignBodyByteKind::EndOfRecord);
        assert_eq!(sign_body_byte_kind(0x0D), SignBodyByteKind::PauseForKey);
        // Macro pool 0x29..=0x31.
        for byte in 0x29u8..=0x31 {
            assert_eq!(sign_body_byte_kind(byte), SignBodyByteKind::Macro(byte));
        }
        // Separator glyphs.
        assert_eq!(sign_body_byte_kind(0x26), SignBodyByteKind::SeparatorGlyph);
        assert_eq!(sign_body_byte_kind(0x27), SignBodyByteKind::SeparatorGlyph);
        // Other bytes print as low-seven-bit characters; high bit stripped.
        assert_eq!(
            sign_body_byte_kind(b'A'),
            SignBodyByteKind::Character(b'A')
        );
        assert_eq!(
            sign_body_byte_kind(0xC1),
            SignBodyByteKind::Character(b'A')
        );
        assert_eq!(
            sign_body_byte_kind(0x20),
            SignBodyByteKind::Character(0x20)
        );
        assert_eq!(
            sign_body_byte_kind(0xFF),
            SignBodyByteKind::Character(0x7F)
        );
    }

    #[test]
    fn town_fountain_drink_accepts_excludes_dead_and_asleep() {
        // view.md §3
        for status in [
            CharacterStatus::Good,
            CharacterStatus::PoisonedOrRevived,
            CharacterStatus::Charmed,
            CharacterStatus::Ashes,
        ] {
            assert!(
                town_fountain_drink_accepts(status),
                "status {:?} should be eligible",
                status
            );
        }
        assert!(!town_fountain_drink_accepts(CharacterStatus::Dead));
        assert!(!town_fountain_drink_accepts(CharacterStatus::Sleeping));
    }

    #[test]
    fn doom_entrance_unlocks_only_when_all_three_slots_vanquished() {
        // catalogs/quest-graph.md §5
        let v = SHADOWLORD_HIDEOUT_VANQUISHED;
        assert!(all_shadowlords_vanquished([v, v, v]));
        assert!(!all_shadowlords_vanquished([0, v, v]));
        assert!(!all_shadowlords_vanquished([v, 0, v]));
        assert!(!all_shadowlords_vanquished([v, v, 0]));
        assert!(!all_shadowlords_vanquished([1, 2, 3]));
        assert!(!all_shadowlords_vanquished([0xFE, v, v]));
    }

    #[test]
    fn shadowlord_names_match_quest_graph_md_section_5() {
        // catalogs/quest-graph.md §5
        assert_eq!(shadowlord_name_for_slot(0), Some("FAULINEI"));
        assert_eq!(shadowlord_name_for_slot(1), Some("ASTAROTH"));
        assert_eq!(shadowlord_name_for_slot(2), Some("NOSFENTOR"));
        assert_eq!(shadowlord_name_for_slot(3), None);
        // Reverse mapping.
        assert_eq!(shadowlord_slot_for_name("FAULINEI"), Some(0));
        assert_eq!(shadowlord_slot_for_name("ASTAROTH"), Some(1));
        assert_eq!(shadowlord_slot_for_name("NOSFENTOR"), Some(2));
        assert_eq!(shadowlord_slot_for_name("MONDAIN"), None);
        assert_eq!(shadowlord_slot_for_name("faulinei"), None); // case sensitive
        // Round-trip.
        for slot in 0usize..=2 {
            let name = shadowlord_name_for_slot(slot).unwrap();
            assert_eq!(shadowlord_slot_for_name(name), Some(slot));
        }
    }

    #[test]
    fn dungeon_word_of_power_matches_quest_graph_md_section_4() {
        // catalogs/quest-graph.md §4
        assert_eq!(dungeon_word_of_power(33), Some("FALLAX"));
        assert_eq!(dungeon_word_of_power(34), Some("VILIS"));
        assert_eq!(dungeon_word_of_power(35), Some("INOPIA"));
        assert_eq!(dungeon_word_of_power(36), Some("MALUM"));
        assert_eq!(dungeon_word_of_power(37), Some("AVIDUS"));
        assert_eq!(dungeon_word_of_power(38), Some("INFAMA"));
        assert_eq!(dungeon_word_of_power(39), Some("IGNAVUS"));
        assert_eq!(dungeon_word_of_power(40), Some("VERAMOCOR"));
        // Scenes outside the dungeon range have no word.
        for scene in [0u8, 1, 32, 41, 99, 0xFF] {
            assert_eq!(dungeon_word_of_power(scene), None);
        }
    }

    #[test]
    fn trap_revive_only_accepts_dead_slots() {
        // traps.md §3
        assert_eq!(TRAP_REVIVE_STATUS_BYTE, b'P');
        assert!(trap_revive_accepts(CharacterStatus::Dead));
        for status in [
            CharacterStatus::Good,
            CharacterStatus::PoisonedOrRevived,
            CharacterStatus::Sleeping,
            CharacterStatus::Charmed,
            CharacterStatus::Ashes,
        ] {
            assert!(
                !trap_revive_accepts(status),
                "status {:?} should not be revived",
                status
            );
        }
    }

    #[test]
    fn dungeon_torch_increment_bounds_match_spec() {
        // lighting.md §8
        assert_eq!(DUNGEON_TORCH_INCREMENT_MIN, 112);
        assert_eq!(DUNGEON_TORCH_INCREMENT_MAX, 127);
        // Boundary rolls applied to an empty counter.
        assert_eq!(ignite_torch_dungeon(0, DUNGEON_TORCH_INCREMENT_MIN), 112);
        assert_eq!(ignite_torch_dungeon(0, DUNGEON_TORCH_INCREMENT_MAX), 127);
        // Saturating add caps at 255.
        assert_eq!(ignite_torch_dungeon(200, DUNGEON_TORCH_INCREMENT_MAX), 255);
    }

    #[test]
    fn reagent_recipe_bits_match_spell_list_md_section_3() {
        // catalogs/spell-list.md §3
        assert_eq!(Reagent::SulfurAsh.recipe_bit(), 0x80);
        assert_eq!(Reagent::Ginseng.recipe_bit(), 0x40);
        assert_eq!(Reagent::Garlic.recipe_bit(), 0x20);
        assert_eq!(Reagent::SpiderSilk.recipe_bit(), 0x10);
        assert_eq!(Reagent::BloodMoss.recipe_bit(), 0x08);
        assert_eq!(Reagent::BlackPearl.recipe_bit(), 0x04);
        assert_eq!(Reagent::Nightshade.recipe_bit(), 0x02);
        assert_eq!(Reagent::Mandrake.recipe_bit(), 0x01);
        // ORing a sample recipe (Heal: Ginseng + Spider Silk).
        assert_eq!(
            Reagent::Ginseng.recipe_bit() | Reagent::SpiderSilk.recipe_bit(),
            0x50
        );
    }

    #[test]
    fn scroll_grant_label_id_masks_low_three_bits() {
        // catalogs/item-list.md §7.1
        assert_eq!(SCROLL_GRANT_LABEL_MASK, 0x07);
        // Low-three-bit values pass through.
        for label in 0u8..=7 {
            assert_eq!(scroll_grant_label_id(label), label);
        }
        // Higher bits are masked off.
        assert_eq!(scroll_grant_label_id(0x08), 0);
        assert_eq!(scroll_grant_label_id(0x0F), 7);
        assert_eq!(scroll_grant_label_id(0x10), 0);
        assert_eq!(scroll_grant_label_id(0xFF), 7);
    }

    #[test]
    fn launcher_executable_boundary_filenames_match_spec() {
        // launcher.md §2,§5
        assert_eq!(ULTIMA_EXE_FILENAME, "ULTIMA.EXE");
        assert_eq!(DATA_OVL_FILENAME, "DATA.OVL");
        assert_eq!(INTRO_OVL_FILENAME, "INTRO.OVL");
    }

    #[test]
    fn text_window_boot_defaults_match_spec() {
        // text-output.md §9
        assert_eq!(TEXT_WINDOW_DEFAULT_FOREGROUND, 15);
        assert_eq!(TEXT_WINDOW_DEFAULT_BACKGROUND, 0);
        assert_eq!(TEXT_WINDOW_DEFAULT_ACTIVE_INDEX, 0);
        // Packed colour byte: bg in high nibble, fg in low nibble.
        assert_eq!(text_window_default_color_byte(), 0x0F);
    }

    #[test]
    fn text_emitter_byte_kind_classifies_per_spec() {
        // text-output.md §5
        // Newline / carriage return — cursor moves without a glyph.
        assert_eq!(text_emitter_byte_kind(0x0A), EmitterByteKind::LineFeed);
        assert_eq!(text_emitter_byte_kind(0x0D), EmitterByteKind::CarriageReturn);
        // Printable ASCII -> glyph render.
        assert_eq!(
            text_emitter_byte_kind(b'A'),
            EmitterByteKind::Glyph(b'A')
        );
        assert_eq!(
            text_emitter_byte_kind(b' '),
            EmitterByteKind::Glyph(b' ')
        );
        assert_eq!(
            text_emitter_byte_kind(0x7E),
            EmitterByteKind::Glyph(0x7E)
        );
        // Extended control bytes.
        for (byte, kind) in [
            (0xFBu8, TextControlByte::CentreOff),
            (0xFC, TextControlByte::CentreOn),
            (0xFD, TextControlByte::InverseToggle),
            (0xFE, TextControlByte::UnderlineToggle),
            (0xFF, TextControlByte::ClearWindow),
        ] {
            assert_eq!(
                text_emitter_byte_kind(byte),
                EmitterByteKind::Control(kind)
            );
        }
        // Other high-bit bytes have no public glyph meaning.
        for byte in [0x00u8, 0x07, 0x1B, 0x1F, 0x7F, 0x80, 0xA0, 0xFA] {
            assert_eq!(text_emitter_byte_kind(byte), EmitterByteKind::Other);
        }
    }

    #[test]
    fn save_scene_byte_normalised_swaps_combat_marker_for_home_scene() {
        // main-loop.md §11
        // Combat marker -> home scene byte.
        assert_eq!(save_scene_byte_normalised(SCENE_COMBAT_TEMPORARY, 0), 0);
        assert_eq!(save_scene_byte_normalised(SCENE_COMBAT_TEMPORARY, 7), 7);
        assert_eq!(save_scene_byte_normalised(SCENE_COMBAT_TEMPORARY, 33), 33);
        // Non-combat scenes pass through unchanged regardless of home.
        for scene in [0u8, 1, 8, 16, 24, 32, 33, 40, 0x42] {
            assert_eq!(save_scene_byte_normalised(scene, 99), scene);
        }
    }

    #[test]
    fn tile_animation_family_classifies_published_ranges() {
        // animation.md §6
        // Water 0x01..=0x03 — four-frame cycle.
        for tile in 0x01u8..=0x03 {
            assert_eq!(
                tile_animation_family(tile),
                Some(TileAnimationFamily::Water)
            );
            assert_eq!(
                tile_animation_family(tile).unwrap().frame_count(),
                4
            );
        }
        // 0x80..=0x83 short toggle (2 frames).
        for tile in 0x80u8..=0x83 {
            assert_eq!(
                tile_animation_family(tile),
                Some(TileAnimationFamily::EffectShortToggle)
            );
            assert_eq!(
                tile_animation_family(tile).unwrap().frame_count(),
                2
            );
        }
        // 0xD4..=0xD7 first quad terrain.
        for tile in 0xD4u8..=0xD7 {
            assert_eq!(
                tile_animation_family(tile),
                Some(TileAnimationFamily::TerrainQuad1)
            );
        }
        // 0xD8..=0xDB second quad terrain.
        for tile in 0xD8u8..=0xDB {
            assert_eq!(
                tile_animation_family(tile),
                Some(TileAnimationFamily::TerrainQuad2)
            );
        }
        // 0xEC..=0xEF effect quad.
        for tile in 0xECu8..=0xEF {
            assert_eq!(
                tile_animation_family(tile),
                Some(TileAnimationFamily::EffectQuad)
            );
        }
        // 0xFA..=0xFD long toggle (2 frames).
        for tile in 0xFAu8..=0xFD {
            assert_eq!(
                tile_animation_family(tile),
                Some(TileAnimationFamily::LongToggle)
            );
            assert_eq!(
                tile_animation_family(tile).unwrap().frame_count(),
                2
            );
        }
        // Tiles outside the published animator ranges return None.
        for tile in [0x00u8, 0x10, 0x40, 0x7F, 0x84, 0xD3, 0xDC, 0xEB, 0xF0, 0xF9, 0xFE] {
            assert_eq!(
                tile_animation_family(tile),
                None,
                "tile {:#x} should not be in any animator family",
                tile
            );
        }
    }

    #[test]
    fn hidden_treasure_record_special_gates_match_spec() {
        // hidden-treasures.md §2
        // Record 13: requires keys >= 1 and not NPC-occupied.
        assert!(hidden_treasure_record_13_accepts(1, false));
        assert!(hidden_treasure_record_13_accepts(99, false));
        assert!(!hidden_treasure_record_13_accepts(0, false));
        assert!(!hidden_treasure_record_13_accepts(1, true));
        // Record 14: stages once per in-game day.
        assert!(hidden_treasure_record_14_ready(0, 5));
        assert!(hidden_treasure_record_14_ready(4, 5));
        assert!(!hidden_treasure_record_14_ready(5, 5));
        // Record 15: single-use flag must be clear; no NPC.
        assert!(hidden_treasure_record_15_accepts(false, false));
        assert!(!hidden_treasure_record_15_accepts(true, false));
        assert!(!hidden_treasure_record_15_accepts(false, true));
        assert!(!hidden_treasure_record_15_accepts(true, true));
    }

    #[test]
    fn chargen_avatar_seed_header_matches_chargen_md_section_8() {
        // chargen.md §8
        assert_eq!(CHARGEN_AVATAR_SEED_CURRENT_HP, 60);
        assert_eq!(CHARGEN_AVATAR_SEED_MAX_HP, 60);
        assert_eq!(CHARGEN_AVATAR_SEED_EXPERIENCE, 150);
        assert_eq!(CHARGEN_AVATAR_SEED_LEVEL, 2);
        assert_eq!(CHARGEN_AVATAR_SEED_CLASS_BYTE, b'A');
        assert_eq!(CHARGEN_AVATAR_SEED_STATUS_BYTE, b'G');
    }

    #[test]
    fn outdoor_klimb_member_falls_when_dex_below_roll() {
        // doors-and-z-transitions.md §9
        assert_eq!(OUTDOOR_KLIMB_FALL_DIE_LOW, 1);
        assert_eq!(OUTDOOR_KLIMB_FALL_DIE_HIGH, 30);
        assert_eq!(OUTDOOR_KLIMB_FALL_DAMAGE_MIN, 1);
        assert_eq!(OUTDOOR_KLIMB_FALL_DAMAGE_MAX, 5);
        // Roll <= Dex: member holds.
        assert!(!outdoor_klimb_member_falls(20, 1));
        assert!(!outdoor_klimb_member_falls(20, 20));
        // Roll > Dex: member falls.
        assert!(outdoor_klimb_member_falls(20, 21));
        assert!(outdoor_klimb_member_falls(20, 30));
        // Edge: Dex 0 always falls (any roll >= 1 > 0).
        assert!(outdoor_klimb_member_falls(0, 1));
        assert!(outdoor_klimb_member_falls(0, 30));
        // Edge: Dex 30 never falls (max roll 30 == dex).
        assert!(!outdoor_klimb_member_falls(30, 30));
    }

    #[test]
    fn sky_strip_render_order_is_hour_then_trammel_then_felucca() {
        // moons.md §2
        assert_eq!(SKY_STRIP_RENDER_ORDER.len(), 3);
        assert_eq!(SKY_STRIP_RENDER_ORDER[0], SkyStripMarker::FixedHour);
        assert_eq!(SKY_STRIP_RENDER_ORDER[1], SkyStripMarker::Trammel);
        assert_eq!(SKY_STRIP_RENDER_ORDER[2], SkyStripMarker::Felucca);
    }

    #[test]
    fn ship_broadside_direction_accepted_only_perpendicular_to_facing() {
        // vehicles.md §7
        // Ship facing N (0): broadsides E (1) and W (3); bow N (0) and stern S (2) refuse.
        assert!(ship_broadside_direction_accepted(0, 1));
        assert!(ship_broadside_direction_accepted(0, 3));
        assert!(!ship_broadside_direction_accepted(0, 0));
        assert!(!ship_broadside_direction_accepted(0, 2));
        // Ship facing E (1): broadsides N (0) and S (2); bow E (1) and stern W (3) refuse.
        assert!(ship_broadside_direction_accepted(1, 0));
        assert!(ship_broadside_direction_accepted(1, 2));
        assert!(!ship_broadside_direction_accepted(1, 1));
        assert!(!ship_broadside_direction_accepted(1, 3));
        // Ship facing S (2): symmetric with N.
        assert!(ship_broadside_direction_accepted(2, 1));
        assert!(ship_broadside_direction_accepted(2, 3));
        assert!(!ship_broadside_direction_accepted(2, 0));
        assert!(!ship_broadside_direction_accepted(2, 2));
        // Ship facing W (3): symmetric with E.
        assert!(ship_broadside_direction_accepted(3, 0));
        assert!(ship_broadside_direction_accepted(3, 2));
        assert!(!ship_broadside_direction_accepted(3, 1));
        assert!(!ship_broadside_direction_accepted(3, 3));
    }

    #[test]
    fn frigate_purchase_starts_with_full_hull_and_two_skiffs() {
        // vehicles.md §4
        assert_eq!(FRIGATE_PURCHASE_HULL, 100);
        assert_eq!(FRIGATE_PURCHASE_SKIFFS, 2);
    }

    #[test]
    fn outdoor_active_object_single_tile_query_matches_spec_table() {
        // movement.md §4
        // 0xE0..=0xE3 sea-serpent adjacency -> tile 0x07.
        for cls in 0xE0u8..=0xE3 {
            assert_eq!(
                outdoor_active_object_single_tile_query(cls),
                Some(0x07)
            );
        }
        // 0xEC..=0xEF outdoor whirlpool -> tile 0x01.
        for cls in 0xECu8..=0xEF {
            assert_eq!(
                outdoor_active_object_single_tile_query(cls),
                Some(0x01)
            );
        }
        // 0xF4..=0xF7 Corpser -> tile 0x05.
        for cls in 0xF4u8..=0xF7 {
            assert_eq!(
                outdoor_active_object_single_tile_query(cls),
                Some(0x05)
            );
        }
        // 0xF8..=0xFB Rot Worm -> tile 0x04.
        for cls in 0xF8u8..=0xFB {
            assert_eq!(
                outdoor_active_object_single_tile_query(cls),
                Some(0x04)
            );
        }
        // Other classes return None.
        for cls in [
            0x00u8, 0x10, 0x80, 0xC0, 0xE4, 0xEB, 0xF0, 0xF3, 0xFC, 0xFF,
        ] {
            assert_eq!(outdoor_active_object_single_tile_query(cls), None);
        }
    }

    #[test]
    fn shadowlord_hideout_predicates_match_spec() {
        // time.md §7
        assert_eq!(SHADOWLORD_HIDEOUT_FIRST, 1);
        assert_eq!(SHADOWLORD_HIDEOUT_LAST, 8);
        assert_eq!(SHADOWLORD_HIDEOUT_VANQUISHED, 0xFF);
        // Vanquished sentinel.
        assert!(shadowlord_hideout_is_vanquished(SHADOWLORD_HIDEOUT_VANQUISHED));
        for v in [0u8, 1, 7, 8, 0xFE] {
            assert!(!shadowlord_hideout_is_vanquished(v));
        }
        // Live hideout id range.
        for v in 1u8..=8 {
            assert!(shadowlord_hideout_is_live(v));
        }
        for v in [0u8, 9, 32, 0xFE, 0xFF] {
            assert!(!shadowlord_hideout_is_live(v));
        }
    }

    #[test]
    fn party_target_selector_action_decodes_keystrokes() {
        // input.md §9
        // Digits 1..=6 select the matching slot.
        for d in 1u8..=6 {
            assert_eq!(
                party_target_selector_action(b'0' + d),
                PartyTargetSelectorAction::SelectSlot(d - 1)
            );
        }
        // 0, Space, Enter -> confirm.
        for byte in [b'0', b' ', 0x0D, 0x0A] {
            assert_eq!(
                party_target_selector_action(byte),
                PartyTargetSelectorAction::Confirm
            );
        }
        // Escape -> cancel.
        assert_eq!(
            party_target_selector_action(0x1B),
            PartyTargetSelectorAction::Cancel
        );
        // Other bytes are silently discarded.
        for byte in [b'7', b'A', b'a', 0x00, 0xC9, 0xFB, 0xFF] {
            assert_eq!(
                party_target_selector_action(byte),
                PartyTargetSelectorAction::Discard,
                "byte {:#x} should be discarded",
                byte
            );
        }
    }

    #[test]
    fn free_text_input_action_classifies_keystrokes() {
        // input.md §8
        assert_eq!(free_text_input_action(0x08), FreeTextInputAction::Backspace);
        assert_eq!(free_text_input_action(0x0D), FreeTextInputAction::Submit);
        assert_eq!(free_text_input_action(0x0A), FreeTextInputAction::Submit);
        assert_eq!(free_text_input_action(0x1B), FreeTextInputAction::Cancel);
        // Printable ASCII appends.
        assert_eq!(
            free_text_input_action(b'A'),
            FreeTextInputAction::Append(b'A')
        );
        assert_eq!(
            free_text_input_action(b'7'),
            FreeTextInputAction::Append(b'7')
        );
        assert_eq!(
            free_text_input_action(b' '),
            FreeTextInputAction::Append(b' ')
        );
        assert_eq!(
            free_text_input_action(0x7E),
            FreeTextInputAction::Append(0x7E)
        );
        // Other bytes (function keys, direction codes) are discarded.
        for byte in [0x00u8, 0x01, 0x07, 0x1A, 0x7F, 0xC9, 0xFB, 0xFF] {
            assert_eq!(
                free_text_input_action(byte),
                FreeTextInputAction::Discard,
                "byte {:#x} should be discarded",
                byte
            );
        }
    }

    #[test]
    fn visibility_cheap_path_needs_refill_targets_zero_cells() {
        // visibility.md §10
        assert!(visibility_cheap_path_needs_refill(VISIBILITY_USE_COMPANION));
        // Other markers retain their previous-frame state.
        for byte in [
            VISIBILITY_HIDDEN,
            VISIBILITY_CLEAR,
            VISIBILITY_DIM_PERIPHERY,
            VISIBILITY_ALREADY_RENDERED,
            0x42u8,
            0xA0u8,
        ] {
            assert!(
                !visibility_cheap_path_needs_refill(byte),
                "byte {:#x} should not trigger lazy refill",
                byte
            );
        }
    }

    #[test]
    fn outdoor_step_clears_on_destination_targets_moongate_tile() {
        // active-objects.md §8 + overworld.md §9
        assert_eq!(OUTDOOR_STEP_CLEAR_DESTINATION_TILE, 0xDC);
        assert!(outdoor_step_clears_on_destination(0xDC));
        // Other terrain bytes do not auto-clear the slot.
        for tile in [0x00u8, 0x05, 0x44, 0x99, 0xDB, 0xDD, 0xFF] {
            assert!(
                !outdoor_step_clears_on_destination(tile),
                "tile {:#x} should not clear the slot",
                tile
            );
        }
    }

    #[test]
    fn world_location_table_scene_for_row_covers_towns_and_dungeons() {
        // overworld.md §8
        assert_eq!(WORLD_LOCATION_TABLE_TOWN_ROWS, 32);
        assert_eq!(WORLD_LOCATION_TABLE_DUNGEON_ROWS, 8);
        assert_eq!(WORLD_LOCATION_TABLE_TOTAL_ROWS, 40);
        // Town-family rows.
        assert_eq!(world_location_table_scene_for_row(0), Some(1));
        assert_eq!(world_location_table_scene_for_row(31), Some(32));
        // Dungeon-family rows.
        assert_eq!(world_location_table_scene_for_row(32), Some(33));
        assert_eq!(world_location_table_scene_for_row(39), Some(40));
        // Out-of-range rows return None.
        assert_eq!(world_location_table_scene_for_row(40), None);
        assert_eq!(world_location_table_scene_for_row(255), None);
    }

    #[test]
    fn town_exit_lands_underworld_only_for_stonegate() {
        // overworld.md §2
        assert_eq!(TOWN_EXIT_UNDERWORLD_SCENE, 0x19);
        assert!(town_exit_lands_underworld(0x19));
        for scene in [0u8, 1, 8, 16, 17, 24, 26, 32, 0x18, 0x1A, 0xFF] {
            assert!(
                !town_exit_lands_underworld(scene),
                "scene {:#x} should restore the surface plane",
                scene
            );
        }
    }

    #[test]
    fn town_stair_intent_decodes_facing_low_bits() {
        // town-mode.md §7
        // Stair tile family bounds.
        assert_eq!(TOWN_STAIR_TILE_FIRST, 0xC4);
        assert_eq!(TOWN_STAIR_TILE_LAST, 0xC7);
        assert_eq!(TOWN_EXIT_THRESHOLD_TILE, 0x59);
        // 0xC4 (stair_facing=0/N): facing 0 -> Up, facing 2 -> Down,
        // facing 1 or 3 -> Cross.
        assert_eq!(town_stair_intent(0xC4, 0), Some(TownStairIntent::Up));
        assert_eq!(town_stair_intent(0xC4, 2), Some(TownStairIntent::Down));
        assert_eq!(town_stair_intent(0xC4, 1), Some(TownStairIntent::Cross));
        assert_eq!(town_stair_intent(0xC4, 3), Some(TownStairIntent::Cross));
        // 0xC5 (stair_facing=1/E): facing 1 -> Up, facing 3 -> Down.
        assert_eq!(town_stair_intent(0xC5, 1), Some(TownStairIntent::Up));
        assert_eq!(town_stair_intent(0xC5, 3), Some(TownStairIntent::Down));
        // 0xC6 (stair_facing=2/S): facing 2 -> Up, facing 0 -> Down.
        assert_eq!(town_stair_intent(0xC6, 2), Some(TownStairIntent::Up));
        assert_eq!(town_stair_intent(0xC6, 0), Some(TownStairIntent::Down));
        // 0xC7 (stair_facing=3/W): facing 3 -> Up, facing 1 -> Down.
        assert_eq!(town_stair_intent(0xC7, 3), Some(TownStairIntent::Up));
        assert_eq!(town_stair_intent(0xC7, 1), Some(TownStairIntent::Down));
        // Non-stair tiles return None.
        assert_eq!(town_stair_intent(0x00, 0), None);
        assert_eq!(town_stair_intent(0xC3, 0), None);
        assert_eq!(town_stair_intent(0xC8, 0), None);
    }

    #[test]
    fn town_dawn_dusk_substitution_band_wraps_midnight() {
        // town-mode.md §5,§6
        assert_eq!(TOWN_NIGHT_BAND_DUSK_HOUR, 20);
        assert_eq!(TOWN_NIGHT_BAND_DAWN_HOUR, 4);
        // Day band (5..=19) is inactive.
        for hour in 5u8..=19 {
            assert!(
                !town_dawn_dusk_substitution_active(hour),
                "hour {} should be day",
                hour
            );
        }
        // Night band (20..=23, 0..=4) is active.
        for hour in [0u8, 1, 2, 3, 4, 20, 21, 22, 23] {
            assert!(
                town_dawn_dusk_substitution_active(hour),
                "hour {} should be night",
                hour
            );
        }
    }

    #[test]
    fn npc_pathfind_workspace_dims_match_spec() {
        // npc-schedules.md §8.1
        assert_eq!(NPC_PATHFIND_WORKSPACE_SIDE, 32);
        assert_eq!(NPC_PATHFIND_WORKSPACE_LEN, 1024);
    }

    #[test]
    fn npc_state_off_floor_or_empty_covers_states_0_and_8() {
        // npc-schedules.md §7
        assert!(npc_state_off_floor_or_empty(NPC_STATE_EMPTY));
        assert!(npc_state_off_floor_or_empty(NPC_STATE_PARKED_OFF_FLOOR));
        for state in [
            NPC_STATE_IDLE,
            NPC_STATE_INPLANE_MOVE,
            NPC_STATE_REPLAY_QUEUE,
            NPC_STATE_DESCEND_TOWARD_TARGET,
            NPC_STATE_ASCEND_TOWARD_TARGET,
            NPC_STATE_CLIMB_UP_OFF_FLOOR,
            NPC_STATE_CLIMB_DOWN_OFF_FLOOR,
        ] {
            assert!(
                !npc_state_off_floor_or_empty(state),
                "state {} should run a movement dispatch arm",
                state
            );
        }
    }

    #[test]
    fn active_object_eviction_phase_byte_acceptance() {
        // active-objects.md §4
        // Phase 1: only empty slot.
        assert!(active_object_eviction_byte_accepted(0x00, 1));
        assert!(!active_object_eviction_byte_accepted(0x01, 1));
        // Phase 2 + 6: 0x01..=0x0F low-priority scenery.
        for phase in [2u8, 6] {
            assert!(active_object_eviction_byte_accepted(0x01, phase));
            assert!(active_object_eviction_byte_accepted(0x0F, phase));
            assert!(!active_object_eviction_byte_accepted(0x00, phase));
            assert!(!active_object_eviction_byte_accepted(0x10, phase));
        }
        // Phase 3 + 7: 0x80..=0xFF except 0xB5.
        for phase in [3u8, 7] {
            assert!(active_object_eviction_byte_accepted(0x80, phase));
            assert!(active_object_eviction_byte_accepted(0xFF, phase));
            assert!(!active_object_eviction_byte_accepted(
                ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE,
                phase
            ));
            assert!(!active_object_eviction_byte_accepted(0x7F, phase));
        }
        // Phase 4 + 8: 0x10 or 0x11.
        for phase in [4u8, 8] {
            assert!(active_object_eviction_byte_accepted(0x10, phase));
            assert!(active_object_eviction_byte_accepted(0x11, phase));
            assert!(!active_object_eviction_byte_accepted(0x12, phase));
        }
        // Phase 5 + 9: 0x30..=0x7F.
        for phase in [5u8, 9] {
            assert!(active_object_eviction_byte_accepted(0x30, phase));
            assert!(active_object_eviction_byte_accepted(0x7F, phase));
            assert!(!active_object_eviction_byte_accepted(0x2F, phase));
            assert!(!active_object_eviction_byte_accepted(0x80, phase));
        }
        // Phase 10: any except 0xB5.
        assert!(active_object_eviction_byte_accepted(0x00, 10));
        assert!(active_object_eviction_byte_accepted(0x80, 10));
        assert!(active_object_eviction_byte_accepted(0xFF, 10));
        assert!(!active_object_eviction_byte_accepted(
            ACTIVE_OBJECT_EVICTION_PROTECTED_TYPE,
            10
        ));
        // Off-screen gate matches phases 2..=5.
        for phase in [2u8, 3, 4, 5] {
            assert!(active_object_eviction_phase_is_off_screen(phase));
        }
        for phase in [0u8, 1, 6, 7, 8, 9, 10, 11, 99] {
            assert!(!active_object_eviction_phase_is_off_screen(phase));
        }
    }

    #[test]
    fn chest_primary_pool_row_succeeds_uses_class_and_roll_gates() {
        // containers.md §4
        // Threshold 7 (Food), chest class 5 -> ineligible.
        assert!(!chest_primary_pool_row_succeeds(5, 7, 30));
        // Threshold 7 (Food), chest class 7 -> eligible; roll 7 succeeds.
        assert!(chest_primary_pool_row_succeeds(7, 7, 7));
        // Threshold 7, chest class 30 -> eligible; roll 6 fails.
        assert!(!chest_primary_pool_row_succeeds(30, 7, 6));
        // Threshold 17 (Scroll/Potion), chest class 25, roll 30 -> succeeds.
        assert!(chest_primary_pool_row_succeeds(25, 17, 30));
        // Threshold 25 (Chest marker), chest class 24 -> ineligible.
        assert!(!chest_primary_pool_row_succeeds(24, 25, 30));
    }

    #[test]
    fn chest_secondary_pool_attempts_uses_floor_half_plus_one() {
        // containers.md §4
        assert_eq!(chest_secondary_pool_attempts(0), 1);
        assert_eq!(chest_secondary_pool_attempts(1), 1);
        assert_eq!(chest_secondary_pool_attempts(2), 2);
        assert_eq!(chest_secondary_pool_attempts(7), 4);
        assert_eq!(chest_secondary_pool_attempts(30), 16);
        assert_eq!(chest_secondary_pool_attempts(127), 64);
    }

    #[test]
    fn u4_transfer_no_transferable_data_fires_only_when_all_zero() {
        // u4-transfer.md §5
        assert_eq!(U4_TRANSFER_VIRTUE_STANDING_COUNT, 8);
        // All zero -> guard fires.
        assert!(u4_transfer_no_transferable_data(&[0u16; 8]));
        assert!(u4_transfer_no_transferable_data(&[]));
        // Any nonzero word allows normal preview.
        assert!(!u4_transfer_no_transferable_data(&[
            1u16, 0, 0, 0, 0, 0, 0, 0
        ]));
        assert!(!u4_transfer_no_transferable_data(&[
            0u16, 0, 0, 0, 0, 0, 0, 50
        ]));
        assert!(!u4_transfer_no_transferable_data(&[
            10u16, 20, 30, 40, 50, 60, 70, 80
        ]));
    }

    #[test]
    fn save_prompt_decision_accepts_only_yn() {
        // save-load.md §5.2
        assert_eq!(save_prompt_decision(b'Y'), Some(true));
        assert_eq!(save_prompt_decision(b'y'), Some(true));
        assert_eq!(save_prompt_decision(b'N'), Some(false));
        assert_eq!(save_prompt_decision(b'n'), Some(false));
        // Other keys loop the prompt.
        assert_eq!(save_prompt_decision(b'\r'), None);
        assert_eq!(save_prompt_decision(b' '), None);
        assert_eq!(save_prompt_decision(b'A'), None);
        assert_eq!(save_prompt_decision(0x00), None);
        assert_eq!(save_prompt_decision(0xFF), None);
    }

    #[test]
    fn save_image_has_active_avatar_checks_offset_0x0002() {
        // save-load.md §4.2
        let mut image = vec![0u8; SAVED_GAM_LEN];
        // Empty save: byte at 0x0002 is zero.
        assert!(!save_image_has_active_avatar(&image));
        // Active save: byte at 0x0002 is nonzero.
        image[SAVE_AVATAR_NAME_OFFSET] = b'A';
        assert!(save_image_has_active_avatar(&image));
        // Truncated buffer (no avatar-name byte) is treated as inactive.
        let short = vec![0u8; SAVE_AVATAR_NAME_OFFSET];
        assert!(!save_image_has_active_avatar(&short));
    }

    #[test]
    fn blackthorn_cutscene_actor_slots_match_spec_role_table() {
        // blackthorn.md §6
        assert_eq!(blackthorn_cutscene_actor(0), Some(BlackthornCutsceneActor::Avatar));
        assert_eq!(
            blackthorn_cutscene_actor(1),
            Some(BlackthornCutsceneActor::SecondPartyMember)
        );
        assert_eq!(
            blackthorn_cutscene_actor(6),
            Some(BlackthornCutsceneActor::Blackthorn)
        );
        assert_eq!(
            blackthorn_cutscene_actor(7),
            Some(BlackthornCutsceneActor::Attendant)
        );
        assert_eq!(
            blackthorn_cutscene_actor(8),
            Some(BlackthornCutsceneActor::Throne)
        );
        // Slot indices outside the published roles are temporary
        // (caller-private) and have no named role.
        for slot in [2u8, 3, 4, 5, 9, 10, 32, 255] {
            assert_eq!(blackthorn_cutscene_actor(slot), None);
        }
        // Round-trip: actor -> slot -> actor.
        for actor in [
            BlackthornCutsceneActor::Avatar,
            BlackthornCutsceneActor::SecondPartyMember,
            BlackthornCutsceneActor::Blackthorn,
            BlackthornCutsceneActor::Attendant,
            BlackthornCutsceneActor::Throne,
        ] {
            assert_eq!(blackthorn_cutscene_actor(actor.slot_index()), Some(actor));
        }
    }

    #[test]
    fn blackthorn_captive_cell_handoff_matches_spec_coordinates() {
        // blackthorn.md §3
        assert_eq!(BLACKTHORN_CAPTIVE_CELL_SCENE, 18);
        assert_eq!(BLACKTHORN_CAPTIVE_CELL_X, 10);
        assert_eq!(BLACKTHORN_CAPTIVE_CELL_Y, 7);
    }

    #[test]
    fn inn_pickup_morbid_path_targets_only_poisoned() {
        // shops.md §8.4
        assert!(inn_pickup_status_converts_to_dead(
            CharacterStatus::PoisonedOrRevived
        ));
        for status in [
            CharacterStatus::Good,
            CharacterStatus::Sleeping,
            CharacterStatus::Charmed,
            CharacterStatus::Dead,
            CharacterStatus::Ashes,
        ] {
            assert!(
                !inn_pickup_status_converts_to_dead(status),
                "status {:?} should not be converted to Dead by pickup",
                status
            );
        }
        // 28-day month-rollover cap.
        assert_eq!(INN_STAY_COUNTER_MAX, 25);
    }

    #[test]
    fn healer_treatment_accepts_per_status_and_hp() {
        // shops.md §8.3
        // Cure: only Poisoned.
        assert!(healer_treatment_accepts(
            HealerTreatment::Cure,
            CharacterStatus::PoisonedOrRevived,
            20,
            30
        ));
        for status in [
            CharacterStatus::Good,
            CharacterStatus::Sleeping,
            CharacterStatus::Charmed,
            CharacterStatus::Dead,
            CharacterStatus::Ashes,
        ] {
            assert!(!healer_treatment_accepts(
                HealerTreatment::Cure,
                status,
                20,
                30
            ));
        }
        // Heal: refuses Dead and at-max HP; otherwise accepts (including Poisoned).
        assert!(healer_treatment_accepts(
            HealerTreatment::Heal,
            CharacterStatus::Good,
            10,
            30
        ));
        assert!(healer_treatment_accepts(
            HealerTreatment::Heal,
            CharacterStatus::PoisonedOrRevived,
            10,
            30
        ));
        assert!(!healer_treatment_accepts(
            HealerTreatment::Heal,
            CharacterStatus::Good,
            30,
            30
        ));
        assert!(!healer_treatment_accepts(
            HealerTreatment::Heal,
            CharacterStatus::Dead,
            10,
            30
        ));
        // Resurrect: only Dead; Ashes and others refused.
        assert!(healer_treatment_accepts(
            HealerTreatment::Resurrect,
            CharacterStatus::Dead,
            0,
            30
        ));
        assert!(!healer_treatment_accepts(
            HealerTreatment::Resurrect,
            CharacterStatus::Ashes,
            0,
            30
        ));
        assert!(!healer_treatment_accepts(
            HealerTreatment::Resurrect,
            CharacterStatus::Good,
            30,
            30
        ));
    }

    #[test]
    fn arms_shop_buy_quote_applies_intelligence_adjustment() {
        // shops.md §6
        // Speaker INT 0: quote = base + base * 100 / 100 = 2 * base.
        assert_eq!(arms_shop_buy_quote(100, 0), 200);
        // Speaker INT 33: factor = 100 - 99 = 1; adj = 100*1/100 = 1.
        assert_eq!(arms_shop_buy_quote(100, 33), 101);
        // Speaker INT 34: factor = 100 - 102 = -2; adj = -2.
        assert_eq!(arms_shop_buy_quote(100, 34), 98);
        // Speaker INT 50: factor = -50; adj = -50; quote = 50.
        assert_eq!(arms_shop_buy_quote(100, 50), 50);
        // Negative quote clamps at zero.
        assert_eq!(arms_shop_buy_quote(10, 99), 0);
    }

    #[test]
    fn arms_shop_sell_offer_uses_intelligence_proportional_floor() {
        // shops.md §6: offer = floor(base * 3 * int / 100) + 1.
        // base 100, int 0: floor(0)+1 = 1.
        assert_eq!(arms_shop_sell_offer(100, 0), 1);
        // base 100, int 30: floor(9000/100)+1 = 91.
        assert_eq!(arms_shop_sell_offer(100, 30), 91);
        // base 50, int 20: floor(3000/100)+1 = 31.
        assert_eq!(arms_shop_sell_offer(50, 20), 31);
        // base 33, int 7: floor(693/100)+1 = 6+1 = 7.
        assert_eq!(arms_shop_sell_offer(33, 7), 7);
    }

    #[test]
    fn shoppe_time_of_day_word_matches_spec_bands() {
        // shops.md §4.1
        for hour in 0u8..12 {
            assert_eq!(shoppe_time_of_day_word(hour), "morning");
        }
        for hour in 12u8..18 {
            assert_eq!(shoppe_time_of_day_word(hour), "afternoon");
        }
        for hour in 18u8..=23 {
            assert_eq!(shoppe_time_of_day_word(hour), "evening");
        }
        // Out-of-range falls through to the evening band.
        assert_eq!(shoppe_time_of_day_word(24), "evening");
        assert_eq!(shoppe_time_of_day_word(255), "evening");
    }

    #[test]
    fn init_gam_constants_match_chargen_md_section_3() {
        // chargen.md §3
        assert_eq!(INIT_GAM_FILE_LEN, 4_192);
        assert_eq!(INIT_GAM_FILENAME, "INIT.GAM");
        // Seed and working-file lengths match.
        assert_eq!(INIT_GAM_FILE_LEN, SAVED_GAM_LEN);
    }

    #[test]
    fn endgame_needs_tableau_restoration_targets_only_dead() {
        // endgame.md §4
        assert!(endgame_needs_tableau_restoration(CharacterStatus::Dead));
        for status in [
            CharacterStatus::Good,
            CharacterStatus::PoisonedOrRevived,
            CharacterStatus::Sleeping,
            CharacterStatus::Charmed,
            CharacterStatus::Ashes,
        ] {
            assert!(
                !endgame_needs_tableau_restoration(status),
                "status {:?} should not trigger tableau restoration",
                status
            );
        }
    }

    #[test]
    fn intro_story6_secondary_subimage_matches_spec_table() {
        // intro.md §10
        assert_eq!(intro_story6_secondary_subimage(15), Some(3));
        assert_eq!(intro_story6_secondary_subimage(20), Some(3));
        assert_eq!(intro_story6_secondary_subimage(16), Some(5));
        assert_eq!(intro_story6_secondary_subimage(18), Some(5));
        assert_eq!(intro_story6_secondary_subimage(17), Some(7));
        assert_eq!(intro_story6_secondary_subimage(19), Some(7));
        // Steps outside the secondary art-pass have no subimage.
        for step in [0usize, 7, 13, 14, 21, 99] {
            assert_eq!(intro_story6_secondary_subimage(step), None);
        }
    }

    #[test]
    fn conjure_summon_for_roll_distributes_per_spec_weights() {
        // magic.md §8
        assert_eq!(CONJURE_OUTCOME_COUNT, 15);
        // Six Giant Rat outcomes (rolls 0..=5).
        for roll in 0u8..=5 {
            assert_eq!(
                conjure_summon_for_roll(roll),
                Some(ConjureSummon::GiantRat)
            );
        }
        // Five Giant Spider outcomes (rolls 6..=10).
        for roll in 6u8..=10 {
            assert_eq!(
                conjure_summon_for_roll(roll),
                Some(ConjureSummon::GiantSpider)
            );
        }
        // Three Bat outcomes (rolls 11..=13).
        for roll in 11u8..=13 {
            assert_eq!(conjure_summon_for_roll(roll), Some(ConjureSummon::Bat));
        }
        // One Python outcome (roll 14).
        assert_eq!(
            conjure_summon_for_roll(14),
            Some(ConjureSummon::Python)
        );
        // Out-of-range rolls return None.
        assert_eq!(conjure_summon_for_roll(15), None);
        assert_eq!(conjure_summon_for_roll(255), None);
    }

    #[test]
    fn resurrection_max_hp_for_level_is_30_per_level() {
        // magic.md §8
        assert_eq!(RESURRECTION_MAX_HP_PER_LEVEL, 30);
        assert_eq!(resurrection_max_hp_for_level(1), 30);
        assert_eq!(resurrection_max_hp_for_level(5), 150);
        assert_eq!(resurrection_max_hp_for_level(8), 240);
        assert_eq!(resurrection_max_hp_for_level(0), 0);
        // Saturating multiply prevents wrap.
        assert_eq!(resurrection_max_hp_for_level(255), 7650);
    }

    #[test]
    fn spell_charge_add_capped_clamps_at_99() {
        // magic.md §6,§7
        assert_eq!(spell_charge_add_capped(0, 0), 0);
        assert_eq!(spell_charge_add_capped(0, 5), 5);
        assert_eq!(spell_charge_add_capped(95, 4), 99);
        // Exact cap reached.
        assert_eq!(spell_charge_add_capped(50, 49), 99);
        // Cap clamps overflow.
        assert_eq!(spell_charge_add_capped(98, 5), 99);
        assert_eq!(spell_charge_add_capped(99, 1), 99);
        // Saturating add prevents wrap; cap enforces 99.
        assert_eq!(spell_charge_add_capped(200, 200), 99);
    }

    #[test]
    fn shrine_offering_cost_charges_digit_times_100() {
        // karma.md §7
        assert_eq!(ShrineVirtue::shrine_offering_cost(0), None);
        for digit in 1u8..=9 {
            assert_eq!(
                ShrineVirtue::shrine_offering_cost(digit),
                Some(digit as u16 * 100)
            );
        }
        // Out-of-range digits return None.
        assert_eq!(ShrineVirtue::shrine_offering_cost(10), None);
        assert_eq!(ShrineVirtue::shrine_offering_cost(255), None);
    }

    #[test]
    fn combat_range_effect_is_cast_like_recognises_selector_1() {
        // combat.md §11
        assert_eq!(RANGED_EFFECT_CAST_LIKE_SELECTOR, 1);
        assert!(combat_range_effect_is_cast_like(1));
        // Other selector values stay on the ordinary attack path.
        for sel in [0u8, 2, 3, 8, 16, 99, 255] {
            assert!(
                !combat_range_effect_is_cast_like(sel),
                "selector {} should not route cast-like",
                sel
            );
        }
    }

    #[test]
    fn combat_exit_outcome_result_codes_match_spec() {
        // combat.md §14
        assert_eq!(CombatExitOutcome::Victory.result_code(), 1);
        assert_eq!(CombatExitOutcome::Escape.result_code(), 1);
        assert_eq!(CombatExitOutcome::Defeat.result_code(), 0);
    }

    #[test]
    fn combat_split_and_factory_defense_constants_match_spec() {
        // combat.md §12
        assert_eq!(COMBAT_SPLIT_PLACEMENT_ATTEMPTS, 8);
        assert_eq!(CHARACTER_DEFENSE_FACTORY_SEED, 7);
    }

    #[test]
    fn combat_step_direction_delta_matches_spec_codes() {
        // combat.md §11
        assert_eq!(combat_step_direction_delta(COMBAT_DIRECTION_WEST), (-1, 0));
        assert_eq!(combat_step_direction_delta(COMBAT_DIRECTION_EAST), (1, 0));
        assert_eq!(combat_step_direction_delta(COMBAT_DIRECTION_NORTH), (0, -1));
        assert_eq!(combat_step_direction_delta(COMBAT_DIRECTION_SOUTH), (0, 1));
        // Code zero and out-of-range fall through to attack-in-place.
        assert_eq!(combat_step_direction_delta(0), (0, 0));
        assert_eq!(combat_step_direction_delta(5), (0, 0));
        assert_eq!(combat_step_direction_delta(0xFF), (0, 0));
    }

    #[test]
    fn combat_restore_active_player_slot_clears_on_dead_or_sleeping() {
        // combat.md §4
        // Dead and Sleeping suppress the restore.
        assert_eq!(
            combat_restore_active_player_slot(2, CharacterStatus::Dead),
            None
        );
        assert_eq!(
            combat_restore_active_player_slot(2, CharacterStatus::Sleeping),
            None
        );
        // All other statuses restore the saved slot.
        for status in [
            CharacterStatus::Good,
            CharacterStatus::PoisonedOrRevived,
            CharacterStatus::Charmed,
            CharacterStatus::Ashes,
        ] {
            assert_eq!(
                combat_restore_active_player_slot(2, status),
                Some(2),
                "status {:?} should restore",
                status
            );
        }
    }

    #[test]
    fn dungeon_klimb_z_step_respects_level_bounds() {
        // dungeon-mode.md §13
        // Up from interior levels.
        assert_eq!(dungeon_klimb_z_step(7, KlimbDirection::Up), Some(6));
        assert_eq!(dungeon_klimb_z_step(1, KlimbDirection::Up), Some(0));
        // Up from top refuses.
        assert_eq!(dungeon_klimb_z_step(0, KlimbDirection::Up), None);
        // Down from interior levels.
        assert_eq!(dungeon_klimb_z_step(0, KlimbDirection::Down), Some(1));
        assert_eq!(dungeon_klimb_z_step(6, KlimbDirection::Down), Some(7));
        // Down from bottom refuses.
        assert_eq!(dungeon_klimb_z_step(7, KlimbDirection::Down), None);
        // Bounds constants.
        assert_eq!(DUNGEON_LEVEL_TOP, 0);
        assert_eq!(DUNGEON_LEVEL_BOTTOM, 7);
    }

    #[test]
    fn dungeon_minimap_flood_expands_except_through_walls() {
        // dungeon-mode.md §12
        // Wall presentation classes stop the flood walker.
        for tile in [0xB0u8, 0xB7, 0xBF, 0xC0, 0xC8, 0xD0, 0xDF] {
            assert!(
                !dungeon_minimap_flood_expands(tile),
                "wall class 0x{:02X} should stop flood",
                tile
            );
        }
        // Door / room-trigger families expand even though they paint
        // a door glyph.
        for tile in [0xA0u8, 0xA8, 0xE0, 0xE7, 0xF0, 0xFF] {
            assert!(
                dungeon_minimap_flood_expands(tile),
                "door/trigger class 0x{:02X} should expand",
                tile
            );
        }
        // Open / passage / fountain / chest / pit / field expand.
        for tile in [0x00u8, 0x10, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90] {
            assert!(dungeon_minimap_flood_expands(tile));
        }
    }

    #[test]
    fn dungeon_attack_post_combat_z_intent_translates_result_code() {
        // dungeon-mode.md §10
        assert_eq!(dungeon_attack_post_combat_z_intent(5), Some(1));
        assert_eq!(dungeon_attack_post_combat_z_intent(6), Some(-1));
        for code in [0u8, 1, 2, 3, 4, 7, 8, 99, 255] {
            assert_eq!(dungeon_attack_post_combat_z_intent(code), None);
        }
    }

    #[test]
    fn dungeon_search_wall_rewrite_classifies_flavour_and_hidden_walls() {
        // dungeon-mode.md §8
        // Flavour 0xC1, 0xC2 narrate only.
        assert_eq!(
            dungeon_search_wall_rewrite(0xC1),
            Some(DungeonSearchWallRewrite::NarrateOnly)
        );
        assert_eq!(
            dungeon_search_wall_rewrite(0xC2),
            Some(DungeonSearchWallRewrite::NarrateOnly)
        );
        // Other 0xC? values rewrite to 0xB0, marker preserved.
        assert_eq!(
            dungeon_search_wall_rewrite(0xC0),
            Some(DungeonSearchWallRewrite::ToFlavourFind(0xB0))
        );
        assert_eq!(
            dungeon_search_wall_rewrite(0xC8),
            Some(DungeonSearchWallRewrite::ToFlavourFind(0xB8))
        );
        assert_eq!(
            dungeon_search_wall_rewrite(0xC3),
            Some(DungeonSearchWallRewrite::ToFlavourFind(0xB0))
        );
        // Hidden-wall 0xD? rewrites to 0xE?, marker preserved.
        assert_eq!(
            dungeon_search_wall_rewrite(0xD0),
            Some(DungeonSearchWallRewrite::ToHiddenWallReveal(0xE0))
        );
        assert_eq!(
            dungeon_search_wall_rewrite(0xD8),
            Some(DungeonSearchWallRewrite::ToHiddenWallReveal(0xE8))
        );
        assert_eq!(
            dungeon_search_wall_rewrite(0xDF),
            Some(DungeonSearchWallRewrite::ToHiddenWallReveal(0xE8))
        );
        // Other classes are not affected by Search.
        assert_eq!(dungeon_search_wall_rewrite(0x00), None);
        assert_eq!(dungeon_search_wall_rewrite(0xB0), None);
        assert_eq!(dungeon_search_wall_rewrite(0xE0), None);
        assert_eq!(dungeon_search_wall_rewrite(0xF0), None);
    }

    #[test]
    fn dungeon_chest_post_get_clears_class_preserving_marker() {
        // dungeon-mode.md §8
        // Plain unmarked closed chest -> passage (no marker).
        assert_eq!(dungeon_chest_post_get_byte(0x40), Some(0x00));
        // Marked chest variants preserve the visit-marker bit.
        assert_eq!(dungeon_chest_post_get_byte(0x48), Some(0x08));
        // Other low-nibble bits are not preserved.
        assert_eq!(dungeon_chest_post_get_byte(0x47), Some(0x00));
        assert_eq!(dungeon_chest_post_get_byte(0x4F), Some(0x08));
        // Non-chest bytes return None.
        assert_eq!(dungeon_chest_post_get_byte(0x00), None);
        assert_eq!(dungeon_chest_post_get_byte(0x60), None);
        assert_eq!(dungeon_chest_post_get_byte(0xFF), None);
    }

    #[test]
    fn dungeon_renderer_cell_byte_strips_variant_below_0x90() {
        // dungeon-mode.md §6.1
        // Below 0x90 — runtime-variant bit cleared.
        assert_eq!(dungeon_renderer_cell_byte(0x60), 0x60);
        assert_eq!(dungeon_renderer_cell_byte(0x68), 0x60);
        assert_eq!(dungeon_renderer_cell_byte(0x69), 0x61);
        assert_eq!(dungeon_renderer_cell_byte(0x88), 0x80);
        // At/above 0x90 — variant bit preserved as overlay flag.
        assert_eq!(dungeon_renderer_cell_byte(0x90), 0x90);
        assert_eq!(dungeon_renderer_cell_byte(0x98), 0x98);
        assert_eq!(dungeon_renderer_cell_byte(0xB8), 0xB8);
        assert_eq!(dungeon_renderer_cell_byte(0xF8), 0xF8);
    }

    #[test]
    fn dungeon_floor_wrap_coord_uses_8_torus() {
        // dungeon-mode.md §6.1
        assert_eq!(dungeon_floor_wrap_coord(0), 0);
        assert_eq!(dungeon_floor_wrap_coord(7), 7);
        assert_eq!(dungeon_floor_wrap_coord(8), 0);
        assert_eq!(dungeon_floor_wrap_coord(-1), 7);
        assert_eq!(dungeon_floor_wrap_coord(-8), 0);
        assert_eq!(dungeon_floor_wrap_coord(15), 7);
    }

    #[test]
    fn dungeon_room_post_combat_patch_demotes_high_nibble_only() {
        // dungeon-mode.md §5
        assert_eq!(
            dungeon_room_post_combat_patch_byte(0xF0),
            Some(0xA0)
        );
        assert_eq!(
            dungeon_room_post_combat_patch_byte(0xF5),
            Some(0xA5)
        );
        assert_eq!(
            dungeon_room_post_combat_patch_byte(0xFF),
            Some(0xAF)
        );
        // Non-room-trigger cells must not be patched.
        assert_eq!(dungeon_room_post_combat_patch_byte(0x00), None);
        assert_eq!(dungeon_room_post_combat_patch_byte(0xA0), None);
        assert_eq!(dungeon_room_post_combat_patch_byte(0xB7), None);
        // The patched byte's class is now RoomHelperState (0xA?).
        assert_eq!(
            dungeon_cell_class_of(
                dungeon_room_post_combat_patch_byte(0xF3).unwrap()
            ),
            DungeonCellClass::RoomHelperState
        );
    }

    #[test]
    fn dungeon_look_description_byte_normalises_only_0x61() {
        // dungeon-mode.md §3
        assert_eq!(dungeon_look_description_byte(0x61), 0x00);
        // Other 0x6? trap bytes keep their pit/trap class.
        assert_eq!(dungeon_look_description_byte(0x60), 0x60);
        assert_eq!(dungeon_look_description_byte(0x62), 0x62);
        assert_eq!(dungeon_look_description_byte(0x69), 0x69);
        assert_eq!(dungeon_look_description_byte(0x6A), 0x6A);
        // Non-pit-class bytes pass through unchanged.
        assert_eq!(dungeon_look_description_byte(0x00), 0x00);
        assert_eq!(dungeon_look_description_byte(0xB0), 0xB0);
        // Verified class still PitTrap for all 0x6? including 0x61
        // (only the description byte is normalised; the cell class
        // lookup still uses the raw tile).
        assert_eq!(
            dungeon_cell_class_of(0x61),
            DungeonCellClass::PitTrap
        );
    }

    #[test]
    fn dungeon_room_arena_index_in_range_recognises_shipped_bank() {
        // encounters.md §8
        assert_eq!(DUNGEON_CBT_ARENA_COUNT, 112);
        assert!(dungeon_room_arena_index_in_range(0));
        assert!(dungeon_room_arena_index_in_range(111));
        assert!(!dungeon_room_arena_index_in_range(112));
        assert!(!dungeon_room_arena_index_in_range(usize::MAX));
    }

    #[test]
    fn r_ready_unequip_returns_stock_below_cap() {
        // inventory.md §6
        assert!(r_ready_unequip_returns_stock(0));
        assert!(r_ready_unequip_returns_stock(1));
        assert!(r_ready_unequip_returns_stock(EQUIPMENT_STOCK_CAP - 1));
        assert!(!r_ready_unequip_returns_stock(EQUIPMENT_STOCK_CAP));
        assert!(!r_ready_unequip_returns_stock(255));
    }

    #[test]
    fn r_ready_burden_gate_uses_strength_total() {
        // inventory.md §2.1
        // Empty member; light item; plenty of strength -> accept.
        assert!(r_ready_burden_gate_accepts(0, 5, 18));
        // Total exactly equals strength -> accept (≤ comparison).
        assert!(r_ready_burden_gate_accepts(10, 8, 18));
        // Total exceeds strength by one -> refuse.
        assert!(!r_ready_burden_gate_accepts(10, 9, 18));
        // Strength of zero refuses anything that has burden.
        assert!(!r_ready_burden_gate_accepts(0, 1, 0));
        assert!(r_ready_burden_gate_accepts(0, 0, 0));
        // Saturating add prevents wrap; strength 250 still rejects.
        assert!(!r_ready_burden_gate_accepts(200, 200, 250));
    }

    #[test]
    fn overworld_underfoot_blackout_obeys_opaque_exemption() {
        // lighting.md §3
        // Special 0xFF underfoot tile forces ambient zero.
        assert!(overworld_underfoot_forces_dark(
            OVERWORLD_UNDERFOOT_BLACKOUT_TILE,
            0x00
        ));
        assert!(overworld_underfoot_forces_dark(0xFF, 0x42));
        // The 0x0E opaque-state tag exempts the pass.
        assert!(!overworld_underfoot_forces_dark(
            OVERWORLD_UNDERFOOT_BLACKOUT_TILE,
            OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG
        ));
        // Non-special underfoot tiles never trigger the override.
        assert!(!overworld_underfoot_forces_dark(0x00, 0x00));
        assert!(!overworld_underfoot_forces_dark(0xFE, 0x00));
    }

    #[test]
    fn tlk_gold_payment_amount_decodes_three_ascii_digits() {
        // conversation.md §7.6
        // Plain ASCII digits.
        assert_eq!(tlk_gold_payment_amount(b'0', b'0', b'0'), Some(0));
        assert_eq!(tlk_gold_payment_amount(b'1', b'2', b'3'), Some(123));
        assert_eq!(tlk_gold_payment_amount(b'9', b'9', b'9'), Some(999));
        // High-bit-set obfuscated digits — masked to seven bits before
        // decoding (engine reads them straight from the stream).
        assert_eq!(
            tlk_gold_payment_amount(b'1' | 0x80, b'2' | 0x80, b'3' | 0x80),
            Some(123)
        );
        // Non-digit arguments reject.
        assert_eq!(tlk_gold_payment_amount(b'0', b'A', b'0'), None);
        assert_eq!(tlk_gold_payment_amount(b':', b'0', b'0'), None);
    }

    #[test]
    fn tlk_player_input_kind_folds_keyword_loop_outcomes() {
        // conversation.md §6
        assert_eq!(
            tlk_player_input_kind(b""),
            TlkPlayerInputKind::EmptyByeShortcut
        );
        assert_eq!(
            tlk_player_input_kind(b"NAME"),
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry)
        );
        assert_eq!(
            tlk_player_input_kind(b"JOB"),
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry)
        );
        assert_eq!(
            tlk_player_input_kind(b"WORK"),
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry)
        );
        assert_eq!(
            tlk_player_input_kind(b"BYE"),
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath)
        );
        assert_eq!(
            tlk_player_input_kind(b"THANK"),
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath)
        );
        assert_eq!(
            tlk_player_input_kind(b"JOIN"),
            TlkPlayerInputKind::OrdinaryKeywordScan
        );
        assert_eq!(
            tlk_player_input_kind(b"GRAN"),
            TlkPlayerInputKind::OrdinaryKeywordScan
        );
    }

    #[test]
    fn tlk_action_dispatch_verb_covers_published_letters() {
        // conversation.md §7.6
        let cases: &[(u8, TlkActionDispatchVerb)] = &[
            (b'A', TlkActionDispatchVerb::RaiseFood),
            (b'B', TlkActionDispatchVerb::RaiseGold),
            (b'C', TlkActionDispatchVerb::RaiseKeys),
            (b'D', TlkActionDispatchVerb::RaiseGems),
            (b'E', TlkActionDispatchVerb::RaiseTorches),
            (b'F', TlkActionDispatchVerb::SetGrappleGate),
            (b'G', TlkActionDispatchVerb::RaiseCarpets),
            (b'H', TlkActionDispatchVerb::SetSextantCarried),
            (b'I', TlkActionDispatchVerb::SetSpyglassCarried),
            (b'J', TlkActionDispatchVerb::SetBlackBadgeCarried),
            (b'K', TlkActionDispatchVerb::RaiseSkullKeys),
        ];
        for &(arg, verb) in cases {
            assert_eq!(tlk_action_dispatch_verb(arg), Some(verb));
            assert!(!tlk_action_dispatch_is_signal_flag(arg));
        }
        // Below the letter band are signal-flag values; above the
        // letter band has no published meaning.
        assert_eq!(tlk_action_dispatch_verb(b'L'), None);
        assert_eq!(tlk_action_dispatch_verb(0x00), None);
        assert!(tlk_action_dispatch_is_signal_flag(0x00));
        assert!(tlk_action_dispatch_is_signal_flag(b'A' - 1));
        assert!(!tlk_action_dispatch_is_signal_flag(b'L'));
    }

    #[test]
    fn tlk_class_for_scene_partitions_per_spec() {
        // conversation.md §3
        assert_eq!(tlk_class_for_scene(0), None);
        assert_eq!(tlk_class_for_scene(1), Some(TlkFileClass::Towne));
        assert_eq!(tlk_class_for_scene(8), Some(TlkFileClass::Towne));
        assert_eq!(tlk_class_for_scene(9), Some(TlkFileClass::Dwelling));
        assert_eq!(tlk_class_for_scene(16), Some(TlkFileClass::Dwelling));
        assert_eq!(tlk_class_for_scene(17), Some(TlkFileClass::Castle));
        assert_eq!(tlk_class_for_scene(24), Some(TlkFileClass::Castle));
        assert_eq!(tlk_class_for_scene(25), Some(TlkFileClass::Keep));
        assert_eq!(tlk_class_for_scene(32), Some(TlkFileClass::Keep));
        assert_eq!(tlk_class_for_scene(33), None);
        assert_eq!(tlk_class_for_scene(0xFF), None);
        // Shipped NPC counts
        assert_eq!(TlkFileClass::Towne.shipped_npc_count(), TOWNE_TLK_NPCS);
        assert_eq!(
            TlkFileClass::Dwelling.shipped_npc_count(),
            DWELLING_TLK_NPCS
        );
        assert_eq!(TlkFileClass::Castle.shipped_npc_count(), CASTLE_TLK_NPCS);
        assert_eq!(TlkFileClass::Keep.shipped_npc_count(), KEEP_TLK_NPCS);
    }

    #[test]
    fn npc_type_byte_class_recognises_published_special_values() {
        // formats/npc.md §6
        assert_eq!(npc_type_byte_class(0x00), NpcTypeByteClass::Empty);
        assert_eq!(
            npc_type_byte_class(0x01),
            NpcTypeByteClass::DefaultHumanSprite
        );
        assert_eq!(
            npc_type_byte_class(0xFC),
            NpcTypeByteClass::RuntimePlayerMirror
        );
        // Stable shipped sprite-class tags fall through to the
        // ordinary derived-sprite path.
        for tag in [0x50u8, 0x54, 0x70, 0x90, 0xD8] {
            assert_eq!(
                npc_type_byte_class(tag),
                NpcTypeByteClass::OrdinarySpriteClass
            );
        }
        // Occupancy: any non-zero byte is occupied; zero is empty.
        assert!(!npc_type_byte_occupied(NPC_TYPE_EMPTY));
        assert!(npc_type_byte_occupied(NPC_TYPE_DEFAULT_HUMAN_SPRITE));
        assert!(npc_type_byte_occupied(NPC_TYPE_RUNTIME_PLAYER_MIRROR));
        assert!(npc_type_byte_occupied(0x50));
    }

    #[test]
    fn world_tick_path_classifies_per_spec_branches() {
        // main-loop.md §9
        // Combat scene -> blat-copy regardless of dirty flag.
        assert_eq!(
            world_tick_path(SCENE_COMBAT_TEMPORARY, true),
            WorldTickPath::CombatBlatCopy
        );
        assert_eq!(
            world_tick_path(SCENE_COMBAT_TEMPORARY, false),
            WorldTickPath::CombatBlatCopy
        );
        // 2D scene + dirty flag set -> full rebuild.
        assert_eq!(
            world_tick_path(SCENE_OVERWORLD, true),
            WorldTickPath::ProducerFullRebuild
        );
        assert_eq!(
            world_tick_path(1, true),
            WorldTickPath::ProducerFullRebuild
        );
        assert_eq!(
            world_tick_path(33, true),
            WorldTickPath::ProducerFullRebuild
        );
        // 2D scene + clear dirty flag -> lazy refill.
        assert_eq!(
            world_tick_path(SCENE_OVERWORLD, false),
            WorldTickPath::LazyRefill
        );
        assert_eq!(world_tick_path(1, false), WorldTickPath::LazyRefill);
        assert_eq!(world_tick_path(33, false), WorldTickPath::LazyRefill);
    }

    #[test]
    fn outer_loop_flags_skip_overworld_only_when_pending_and_zero_scene() {
        // main-loop.md §4
        let pending = OuterLoopFlags {
            exit_pending: true,
            previous_was_dungeon: false,
        };
        // Pending flag + overworld scene -> skip the redundant overworld pass.
        assert!(pending.should_skip_overworld(SCENE_OVERWORLD));
        // Pending flag but a different scene -> the outer loop still routes
        // normally; no-op cancellation can't produce a non-zero scene byte
        // here, but the predicate is conservatively scoped.
        assert!(!pending.should_skip_overworld(1));
        assert!(!pending.should_skip_overworld(33));

        // Default flags never skip.
        let cleared = OuterLoopFlags::default();
        assert!(!cleared.exit_pending);
        assert!(!cleared.previous_was_dungeon);
        for scene in [0u8, 1, 17, 33, 0xFF] {
            assert!(!cleared.should_skip_overworld(scene));
        }
    }

    #[test]
    fn dungeon_room_clear_bit_position_packs_per_spec() {
        // formats/saved-gam.md §10
        assert_eq!(SAVE_DUNGEON_ROOM_CLEAR_BYTES_PER_DUNGEON, 2);
        assert_eq!(SAVE_DUNGEON_ROOM_CLEAR_ROOMS_PER_DUNGEON, 16);

        // Dungeon 0, room 0 -> byte 0 bit 0.
        assert_eq!(dungeon_room_clear_bit_position(0, 0), Some((0, 0x01)));
        // Dungeon 0, room 7 -> byte 0 bit 7.
        assert_eq!(dungeon_room_clear_bit_position(0, 7), Some((0, 0x80)));
        // Dungeon 0, room 8 -> byte 1 bit 0.
        assert_eq!(dungeon_room_clear_bit_position(0, 8), Some((1, 0x01)));
        // Dungeon 0, room 15 -> byte 1 bit 7.
        assert_eq!(dungeon_room_clear_bit_position(0, 15), Some((1, 0x80)));
        // Dungeon 1, room 0 -> byte 2 bit 0.
        assert_eq!(dungeon_room_clear_bit_position(1, 0), Some((2, 0x01)));
        // Dungeon 7, room 15 -> last bit (byte 15, bit 7).
        assert_eq!(dungeon_room_clear_bit_position(7, 15), Some((15, 0x80)));
        // Out-of-range coordinates.
        assert_eq!(dungeon_room_clear_bit_position(8, 0), None);
        assert_eq!(dungeon_room_clear_bit_position(0, 16), None);
        assert_eq!(dungeon_room_clear_bit_position(255, 255), None);

        // All 128 (dungeon, room) pairs map to distinct bit positions
        // within the 16-byte bitmap.
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for d in 0u8..8 {
            for r in 0u8..16 {
                let (byte, mask) = dungeon_room_clear_bit_position(d, r).unwrap();
                let bit_index = byte * 8 + (mask.trailing_zeros() as usize);
                assert!(seen.insert(bit_index), "duplicate ({d}, {r})");
                assert!(bit_index < 128);
            }
        }
        assert_eq!(seen.len(), 128);
    }

    #[test]
    fn save_character_field_offsets_match_spec_record() {
        // formats/saved-gam.md §3.1
        assert_eq!(SAVE_CHARACTER_NAME_OFFSET, 0x00);
        assert_eq!(SAVE_CHARACTER_NAME_LEN_BYTES, 9);
        assert_eq!(SAVE_CHARACTER_STRENGTH_OFFSET, 0x0C);
        assert_eq!(SAVE_CHARACTER_DEXTERITY_OFFSET, 0x0D);
        assert_eq!(SAVE_CHARACTER_INTELLIGENCE_OFFSET, 0x0E);
        assert_eq!(SAVE_CHARACTER_MAGIC_POINTS_OFFSET, 0x0F);
        assert_eq!(SAVE_CHARACTER_HP_CURRENT_OFFSET, 0x10);
        assert_eq!(SAVE_CHARACTER_HP_MAX_OFFSET, 0x12);
        assert_eq!(SAVE_CHARACTER_EXPERIENCE_OFFSET, 0x14);
        assert_eq!(SAVE_CHARACTER_LEVEL_OFFSET, 0x16);
        assert_eq!(SAVE_CHARACTER_MONTH_COUNTER_OFFSET, 0x17);
        assert_eq!(SAVE_CHARACTER_DEFENSE_BYTE_OFFSET, 0x18);
        assert_eq!(SAVE_CHARACTER_RECORD_LEN, 32);

        // Slot 0 record begins at file offset 0x0002 (two leading
        // save-image bytes precede the roster).
        assert_eq!(
            save_character_field_offset(0, SAVE_CHARACTER_NAME_OFFSET),
            0x0002
        );
        assert_eq!(
            save_character_field_offset(0, SAVE_CHARACTER_LEVEL_OFFSET),
            0x0002 + 0x16
        );
        // Slot 15 record fits inside the 512-byte roster region.
        assert_eq!(
            save_character_field_offset(15, SAVE_CHARACTER_DEFENSE_BYTE_OFFSET),
            0x0002 + 15 * 32 + 0x18
        );
    }

    #[test]
    fn spyglass_usable_requires_overworld_with_stars() {
        // inventory.md §7
        assert!(spyglass_usable(0, true));
        assert!(!spyglass_usable(0, false));
        // Non-overworld scenes refuse regardless of sky state.
        for scene in [1u8, 17, 33, 0xFF] {
            assert!(!spyglass_usable(scene, true));
            assert!(!spyglass_usable(scene, false));
        }
    }

    #[test]
    fn hms_cape_plans_usable_only_aboard_ship() {
        // inventory.md §7
        for marker in 0x20u8..=0x27 {
            assert!(hms_cape_plans_usable(marker), "marker {marker:#x}");
        }
        // Foot, horse, carpet, skiff -> refuse.
        for marker in [0x12u8, 0x13, 0x14, 0x15, 0x16, 0x17, 0x1C, 0x1D, 0x28, 0x2B] {
            assert!(!hms_cape_plans_usable(marker), "marker {marker:#x}");
        }
        assert!(!hms_cape_plans_usable(0x00));
        assert!(!hms_cape_plans_usable(0xFF));
    }

    #[test]
    fn sextant_usable_only_at_overworld_night() {
        // inventory.md §7
        // Overworld at night (hours 0..=5 and 19..=23) -> usable.
        for h in 0u8..=5 {
            assert!(sextant_usable(0, h), "hour {h}");
        }
        for h in 19u8..=23 {
            assert!(sextant_usable(0, h), "hour {h}");
        }
        // Overworld during daylight (hours 6..=18) -> refused.
        for h in 6u8..=18 {
            assert!(!sextant_usable(0, h), "hour {h}");
        }
        // Non-overworld scenes refuse regardless of hour.
        for scene in [1u8, 17, 33, 0xFF] {
            for h in 0u8..24 {
                assert!(!sextant_usable(scene, h), "scene {scene} hour {h}");
            }
        }
    }

    #[test]
    fn intro_story_special_step_predicates_match_spec() {
        // intro.md §10
        assert_eq!(INTRO_TRANSITION_STRIP_STEPS, [0, 7, 14]);
        assert_eq!(
            INTRO_STORY6_SECONDARY_PASS_STEPS,
            [15, 16, 17, 18, 19, 20]
        );
        assert_eq!(INTRO_STORY6_SECONDARY_Y_DELTA, 55);

        // Transition-strip predicate is true only for steps 0, 7, 14.
        for step in 0usize..INTRO_STORY_STEP_COUNT {
            let want = matches!(step, 0 | 7 | 14);
            assert_eq!(intro_step_has_transition_strip(step), want, "step {step}");
        }

        // STORY6 secondary-pass predicate is true only for steps 15..=20.
        for step in 0usize..INTRO_STORY_STEP_COUNT {
            let want = (15..=20).contains(&step);
            assert_eq!(
                intro_step_has_story6_secondary_pass(step),
                want,
                "step {step}"
            );
        }
    }

    #[test]
    fn reserved_keyword_table_size_matches_spec_inventory() {
        // conversation.md §5
        assert_eq!(RESERVED_KEYWORD_TABLE_ENTRIES, 34);
        assert_eq!(RESERVED_KEYWORD_FUNCTIONAL_COUNT, 5);
        assert_eq!(RESERVED_KEYWORD_REBUKE_COUNT, 29);
        assert_eq!(
            RESERVED_KEYWORD_FUNCTIONAL_COUNT + RESERVED_KEYWORD_REBUKE_COUNT,
            RESERVED_KEYWORD_TABLE_ENTRIES
        );
    }

    #[test]
    fn spell_scene_allow_mask_bits_match_spec() {
        // magic.md §9
        assert_eq!(SPELL_SCENE_BIT_DUNGEON, 0x01);
        assert_eq!(SPELL_SCENE_BIT_COMBAT, 0x02);
        assert_eq!(SPELL_SCENE_BIT_INDOOR, 0x04);
        assert_eq!(SPELL_SCENE_BIT_OVERWORLD, 0x08);

        assert_eq!(SpellSceneClass::Dungeon.allow_mask_bit(), 0x01);
        assert_eq!(SpellSceneClass::Combat.allow_mask_bit(), 0x02);
        assert_eq!(SpellSceneClass::Indoor.allow_mask_bit(), 0x04);
        assert_eq!(SpellSceneClass::Overworld.allow_mask_bit(), 0x08);

        // Combat-only spell (mask 0x02) accepts combat, refuses elsewhere.
        let combat_only = 0x02;
        assert!(spell_allowed_in_scene(combat_only, SpellSceneClass::Combat));
        assert!(!spell_allowed_in_scene(combat_only, SpellSceneClass::Dungeon));
        assert!(!spell_allowed_in_scene(combat_only, SpellSceneClass::Indoor));
        assert!(!spell_allowed_in_scene(combat_only, SpellSceneClass::Overworld));

        // Universal spell (mask 0x0F) accepts everywhere.
        let universal = 0x0F;
        for scene in [
            SpellSceneClass::Dungeon,
            SpellSceneClass::Combat,
            SpellSceneClass::Indoor,
            SpellSceneClass::Overworld,
        ] {
            assert!(spell_allowed_in_scene(universal, scene));
        }
        // Empty mask refuses everything.
        for scene in [
            SpellSceneClass::Dungeon,
            SpellSceneClass::Combat,
            SpellSceneClass::Indoor,
            SpellSceneClass::Overworld,
        ] {
            assert!(!spell_allowed_in_scene(0x00, scene));
        }
    }

    #[test]
    fn directed_wind_spell_kill_xp_predicate_matches_spec() {
        // magic.md §8
        assert_eq!(DIRECTED_WIND_MAX_CELLS, 21);
        // Damage winds credit kill XP; status winds do not.
        assert!(DirectedWindSpell::DeathWind.credits_kill_xp());
        assert!(DirectedWindSpell::FlameWind.credits_kill_xp());
        assert!(!DirectedWindSpell::Sleep.credits_kill_xp());
        assert!(!DirectedWindSpell::PoisonWind.credits_kill_xp());
    }

    #[test]
    fn field_spell_kind_byte_tables_match_spec() {
        // magic.md §8
        // Dungeon base bytes.
        assert_eq!(FieldSpellKind::Fire.dungeon_base_byte(), 0x82);
        assert_eq!(FieldSpellKind::Poison.dungeon_base_byte(), 0x81);
        assert_eq!(FieldSpellKind::Sleep.dungeon_base_byte(), 0x80);
        assert_eq!(FieldSpellKind::Energy.dungeon_base_byte(), 0x83);
        // Marker-preserving variants are base | 0x08.
        for k in [
            FieldSpellKind::Fire,
            FieldSpellKind::Poison,
            FieldSpellKind::Sleep,
            FieldSpellKind::Energy,
        ] {
            assert_eq!(k.dungeon_marker_byte(), k.dungeon_base_byte() | 0x08);
        }
        // Combat field-kind bytes.
        assert_eq!(FieldSpellKind::Fire.combat_kind_byte(), 0x35);
        assert_eq!(FieldSpellKind::Poison.combat_kind_byte(), 0x33);
        assert_eq!(FieldSpellKind::Sleep.combat_kind_byte(), 0x34);
        assert_eq!(FieldSpellKind::Energy.combat_kind_byte(), 0x36);
        // Reverse classifier accepts both base and marker variants.
        for (byte, expected) in [
            (0x82u8, FieldSpellKind::Fire),
            (0x8A, FieldSpellKind::Fire),
            (0x81, FieldSpellKind::Poison),
            (0x89, FieldSpellKind::Poison),
            (0x80, FieldSpellKind::Sleep),
            (0x88, FieldSpellKind::Sleep),
            (0x83, FieldSpellKind::Energy),
            (0x8B, FieldSpellKind::Energy),
        ] {
            assert_eq!(field_spell_kind_for_dungeon_byte(byte), Some(expected));
        }
        // Non-field bytes return None.
        assert_eq!(field_spell_kind_for_dungeon_byte(0x00), None);
        assert_eq!(field_spell_kind_for_dungeon_byte(0x84), None);
        assert_eq!(field_spell_kind_for_dungeon_byte(0xFF), None);
    }

    #[test]
    fn active_effect_tag_byte_and_install_counter_match_spec() {
        // magic.md §8
        // ASCII bytes.
        assert_eq!(ActiveEffectTag::Protection.ascii_byte(), b'P');
        assert_eq!(ActiveEffectTag::Quickness.ascii_byte(), b'Q');
        assert_eq!(ActiveEffectTag::MassCharm.ascii_byte(), b'C');
        assert_eq!(ActiveEffectTag::NegateMagic.ascii_byte(), b'N');
        assert_eq!(ActiveEffectTag::NegateTime.ascii_byte(), b'T');

        // Spell-side install counters.
        assert_eq!(ActiveEffectTag::Protection.spell_install_counter(), Some(20));
        assert_eq!(ActiveEffectTag::Quickness.spell_install_counter(), Some(30));
        assert_eq!(ActiveEffectTag::MassCharm.spell_install_counter(), Some(20));
        assert_eq!(ActiveEffectTag::NegateMagic.spell_install_counter(), Some(10));
        // Negate Time C-Cast installs the shared `T` tag with the
        // published 10-count countdown (catalogs/spell-list.md §6).
        assert_eq!(
            ActiveEffectTag::NegateTime.spell_install_counter(),
            Some(TIME_STOP_DURATION)
        );

        // Byte -> tag classification.
        assert_eq!(active_effect_tag_for_byte(b'P'), Some(ActiveEffectTag::Protection));
        assert_eq!(active_effect_tag_for_byte(b'Q'), Some(ActiveEffectTag::Quickness));
        assert_eq!(active_effect_tag_for_byte(b'C'), Some(ActiveEffectTag::MassCharm));
        assert_eq!(active_effect_tag_for_byte(b'N'), Some(ActiveEffectTag::NegateMagic));
        assert_eq!(active_effect_tag_for_byte(b'T'), Some(ActiveEffectTag::NegateTime));
        assert_eq!(active_effect_tag_for_byte(b'A'), None);
        assert_eq!(active_effect_tag_for_byte(0), None);
    }

    #[test]
    fn spawn_terrain_branch_classifier_matches_spec_table() {
        // encounters.md §4
        assert_eq!(SPAWN_WHIRLPOOL_DENOMINATOR, 7);
        assert_eq!(SPAWN_SEA_SERPENT_DENOMINATOR, 3);
        assert_eq!(SPAWN_LOW_TILE_ALLOWANCE_DENOMINATOR, 4);

        // Surface tile 1 -> whirlpool/aquatic special branch.
        assert_eq!(
            spawn_terrain_branch(0x01, false),
            SpawnTerrainBranch::SurfaceTile1WhirlpoolOrAquatic
        );
        // Underworld tile 4 -> Rot Worm direct branch; surface tile 4
        // continues to the land bucket selected by plane.
        assert_eq!(
            spawn_terrain_branch(0x04, true),
            SpawnTerrainBranch::UnderworldTile4RotWorm
        );
        assert_eq!(
            spawn_terrain_branch(0x04, false),
            SpawnTerrainBranch::LandBucket
        );
        // Tile 7 -> sea-serpent adjacency.
        assert_eq!(
            spawn_terrain_branch(0x07, false),
            SpawnTerrainBranch::SeaSerpentAdjacency
        );
        // Town outline tiles 0x0C / 0x0D -> hard reject.
        assert_eq!(
            spawn_terrain_branch(0x0C, false),
            SpawnTerrainBranch::HardReject
        );
        assert_eq!(
            spawn_terrain_branch(0x0D, false),
            SpawnTerrainBranch::HardReject
        );
        // Low/shore/road/bridge bands -> low-tile allowance.
        for t in [0x00u8, 0x02, 0x03, 0x60, 0x6F, 0xD4, 0xD7, 0xE4, 0xE7] {
            assert_eq!(
                spawn_terrain_branch(t, false),
                SpawnTerrainBranch::LowTileAllowance,
                "tile {t:#x}"
            );
        }
        // Land bucket coverage.
        for t in [0x05u8, 0x06, 0x08, 0x09, 0x0E, 0x0F, 0x30, 0x33] {
            assert_eq!(
                spawn_terrain_branch(t, false),
                SpawnTerrainBranch::LandBucket,
                "tile {t:#x}"
            );
        }
        // Other high tiles -> reject.
        for t in [0x10u8, 0x40, 0x80, 0xCF, 0xFF] {
            assert_eq!(
                spawn_terrain_branch(t, false),
                SpawnTerrainBranch::HighTileReject,
                "tile {t:#x}"
            );
        }
    }

    #[test]
    fn encounter_spawner_separation_gate_matches_spec() {
        // encounters.md §4
        assert_eq!(ENCOUNTER_SPAWNER_RETRY_LIMIT, 128);
        assert_eq!(ENCOUNTER_SPAWNER_MIN_SEPARATION, 6);
        assert_eq!(ENCOUNTER_SPAWNER_MAX_SEPARATION, 250);
        assert_eq!(SEA_CREATURE_WANDER_SEED, 100);

        // Inside the visible centre — both axes <= 6 — rejected.
        for d in 0u8..=6 {
            assert!(!encounter_spawner_separation_ok(50 + d, 50, 50, 50));
            assert!(!encounter_spawner_separation_ok(50, 50 + d, 50, 50));
        }
        // Past 6 on both axes — accepted.
        assert!(encounter_spawner_separation_ok(57, 57, 50, 50));
        assert!(encounter_spawner_separation_ok(43, 43, 50, 50)); // dx=7, dy=7
        assert!(encounter_spawner_separation_ok(60, 60, 50, 50)); // dx=10, dy=10
        // dx > 6 but dy <= 6 — rejected.
        assert!(!encounter_spawner_separation_ok(57, 53, 50, 50));
        // Either axis at the boundary — rejected.
        assert!(!encounter_spawner_separation_ok(56, 60, 50, 50)); // dx=6
        assert!(!encounter_spawner_separation_ok(60, 56, 50, 50)); // dy=6
        // Wrapped-near torus distances: dx == 250 -> rejected, dx ==
        // 249 -> accepted (open interval at MAX).
        assert!(!encounter_spawner_separation_ok(0, 100, 250, 100));
        assert!(encounter_spawner_separation_ok(0, 100, 249, 80));
    }

    #[test]
    fn random_spawn_bucket_picker_matches_spec_weights() {
        // encounters.md §4
        // Surface aquatic bucket: cumulative weights are
        // 72, 144, 184, 222, 256.
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 0), Some(0x8C));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 71), Some(0x8C));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 72), Some(0x84));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 143), Some(0x84));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 144), Some(0x88));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 183), Some(0x88));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 184), Some(0x80));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 221), Some(0x80));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 222), Some(0x2C));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_AQUATIC_BUCKET, 255), Some(0x2C));

        // Underworld aquatic bucket: 128, 256.
        assert_eq!(pick_random_spawn_bucket(&UNDERWORLD_AQUATIC_BUCKET, 0), Some(0x84));
        assert_eq!(pick_random_spawn_bucket(&UNDERWORLD_AQUATIC_BUCKET, 127), Some(0x84));
        assert_eq!(pick_random_spawn_bucket(&UNDERWORLD_AQUATIC_BUCKET, 128), Some(0x88));
        assert_eq!(pick_random_spawn_bucket(&UNDERWORLD_AQUATIC_BUCKET, 255), Some(0x88));

        // Surface land bucket counts.
        assert_eq!(SURFACE_LAND_BUCKET.len(), 12);
        // Underworld land bucket counts.
        assert_eq!(UNDERWORLD_LAND_BUCKET.len(), 7);

        // Surface land first entry (Orc, weight 60).
        assert_eq!(pick_random_spawn_bucket(&SURFACE_LAND_BUCKET, 0), Some(0xC0));
        assert_eq!(pick_random_spawn_bucket(&SURFACE_LAND_BUCKET, 59), Some(0xC0));
        // Last surface-land entry (Daemon, weight 1, cumulative 256).
        assert_eq!(pick_random_spawn_bucket(&SURFACE_LAND_BUCKET, 255), Some(0xD8));

        // Empty bucket -> None.
        assert_eq!(pick_random_spawn_bucket(&[], 0), None);
    }

    #[test]
    fn ship_terrain_predicate_matches_spec_water_band() {
        // movement.md §4
        assert_eq!(SHIP_TERRAIN_ACCEPTED_TILES, [0x00, 0x01, 0x02]);
        assert!(ship_terrain_accepts(0x00));
        assert!(ship_terrain_accepts(0x01));
        assert!(ship_terrain_accepts(0x02));
        // 0x03 (shoals) is not a ship-passable tile; only 0..=2 are.
        assert!(!ship_terrain_accepts(0x03));
        // Land bands rejected.
        for t in [0x04u8, 0x05, 0x09, 0x0F, 0x10, 0x60, 0x80, 0xFF] {
            assert!(!ship_terrain_accepts(t), "tile {t:#x}");
        }
        // Water-creature predicate mirrors the ship predicate.
        for t in 0u8..=255 {
            assert_eq!(
                water_creature_terrain_accepts(t),
                ship_terrain_accepts(t)
            );
        }
    }

    #[test]
    fn potion_use_effects_match_spec_display_order() {
        // inventory.md §7
        assert_eq!(POTION_USE_EFFECT_COUNT, 8);
        assert_eq!(POTION_VARIATION_DENOMINATOR, 16);

        assert_eq!(potion_use_effect(0), Some(PotionUseEffect::Wake));
        assert_eq!(potion_use_effect(1), Some(PotionUseEffect::Heal));
        assert_eq!(potion_use_effect(2), Some(PotionUseEffect::CurePoison));
        assert_eq!(potion_use_effect(3), Some(PotionUseEffect::Poison));
        assert_eq!(potion_use_effect(4), Some(PotionUseEffect::Sleep));
        assert_eq!(
            potion_use_effect(5),
            Some(PotionUseEffect::PoofPresentation)
        );
        assert_eq!(
            potion_use_effect(6),
            Some(PotionUseEffect::CombatInvisibility)
        );
        assert_eq!(
            potion_use_effect(7),
            Some(PotionUseEffect::VisibilitySweep)
        );
        assert_eq!(potion_use_effect(8), None);
        assert_eq!(potion_use_effect(255), None);

        // Cross-check against the existing POTION_*_INDEX constants.
        assert_eq!(
            potion_use_effect(POTION_BLUE_INDEX),
            Some(PotionUseEffect::Wake)
        );
        assert_eq!(
            potion_use_effect(POTION_RED_INDEX),
            Some(PotionUseEffect::CurePoison)
        );
        assert_eq!(
            potion_use_effect(POTION_GREEN_INDEX),
            Some(PotionUseEffect::Poison)
        );
        assert_eq!(
            potion_use_effect(POTION_ORANGE_INDEX),
            Some(PotionUseEffect::Sleep)
        );
    }

    #[test]
    fn scroll_use_effects_match_spec_table() {
        // inventory.md §7
        assert_eq!(SCROLL_USE_EFFECT_COUNT, 8);
        assert_eq!(scroll_use_effect(0), Some(ScrollUseEffect::Light));
        assert_eq!(scroll_use_effect(1), Some(ScrollUseEffect::WindChange));
        assert_eq!(scroll_use_effect(2), Some(ScrollUseEffect::Protection));
        assert_eq!(scroll_use_effect(3), Some(ScrollUseEffect::NegateMagic));
        assert_eq!(scroll_use_effect(4), Some(ScrollUseEffect::View));
        assert_eq!(scroll_use_effect(5), Some(ScrollUseEffect::SummonDaemon));
        assert_eq!(scroll_use_effect(6), Some(ScrollUseEffect::Resurrection));
        assert_eq!(scroll_use_effect(7), Some(ScrollUseEffect::NegateTime));
        assert_eq!(scroll_use_effect(8), None);
        assert_eq!(scroll_use_effect(255), None);

        // Item-specific durations.
        assert_eq!(SCROLL_LIGHT_DURATION, 240);
        assert_eq!(SCROLL_PROTECTION_DURATION, 100);
        assert_eq!(SCROLL_NEGATE_MAGIC_DURATION, 20);
        assert_eq!(SCROLL_NEGATE_TIME_DURATION, 20);
    }

    #[test]
    fn local_light_source_tile_matches_spec_candidates() {
        // visibility.md §12
        assert_eq!(LOCAL_LIGHT_MASK_SIDE, 32);
        for t in 0xB0u8..=0xB3 {
            assert!(is_local_light_source_tile(t), "tile {t:#x}");
        }
        for t in 0xBCu8..=0xBF {
            assert!(is_local_light_source_tile(t), "tile {t:#x}");
        }
        assert!(is_local_light_source_tile(0xDC));
        assert!(is_local_light_source_tile(0xDE));
        // Nearby non-source tiles.
        for t in [
            0x00u8, 0x01, 0xAF, 0xB4, 0xBB, 0xC0, 0xDB, 0xDD, 0xDF, 0xE0, 0xFF,
        ] {
            assert!(
                !is_local_light_source_tile(t),
                "tile {t:#x} should not be a local-light source"
            );
        }
    }

    #[test]
    fn active_object_compositor_branch_matches_spec_table() {
        // visibility.md §8
        assert_eq!(VEHICLE_AVATAR_UNDERLAY_MARKER, 0x92);

        // Water-bound class via type byte.
        for t in 0xE8u8..=0xEB {
            assert_eq!(
                active_object_compositor_branch(t, 0),
                ActiveObjectCompositorBranch::WaterBoundCompanion
            );
        }
        assert_eq!(
            active_object_compositor_branch(0x1E, 0),
            ActiveObjectCompositorBranch::WaterBoundCompanion
        );
        assert_eq!(
            active_object_compositor_branch(0x1F, 0),
            ActiveObjectCompositorBranch::WaterBoundCompanion
        );

        // Water-creature class via frame byte (when type byte does not
        // already match the water-bound branch).
        assert_eq!(
            active_object_compositor_branch(0x80, 0x1D),
            ActiveObjectCompositorBranch::WaterCreatureCompanion
        );
        assert_eq!(
            active_object_compositor_branch(0x80, 0x1E),
            ActiveObjectCompositorBranch::WaterCreatureCompanion
        );

        // Vehicle/avatar branch.
        assert_eq!(
            active_object_compositor_branch(0x5C, 0),
            ActiveObjectCompositorBranch::VehicleAvatarCompanion
        );

        // Default helper for everything else.
        assert_eq!(
            active_object_compositor_branch(0x00, 0x00),
            ActiveObjectCompositorBranch::DefaultHelper
        );
        assert_eq!(
            active_object_compositor_branch(0x80, 0x00),
            ActiveObjectCompositorBranch::DefaultHelper
        );
        assert_eq!(
            active_object_compositor_branch(0xC0, 0xC0),
            ActiveObjectCompositorBranch::DefaultHelper
        );
    }

    #[test]
    fn fog_refinement_squared_distance_matches_spec_threshold() {
        // visibility.md §7
        assert_eq!(FOG_REFINE_SQUARED_THRESHOLD, 5);
        // Fold rule: min(coord, 10 - coord).
        assert_eq!(fog_refine_folded_coord(0), 0);
        assert_eq!(fog_refine_folded_coord(5), 5);
        assert_eq!(fog_refine_folded_coord(10), 0);
        assert_eq!(fog_refine_folded_coord(3), 3);
        assert_eq!(fog_refine_folded_coord(7), 3);
        // Centre cell -> distance 0 -> inside core.
        assert_eq!(fog_refine_squared_distance(5, 5), 0);
        assert!(fog_refine_inside_clear_core(5, 5));
        // (5, 4) and (4, 5) -> distance 1 -> inside.
        assert_eq!(fog_refine_squared_distance(5, 4), 1);
        assert_eq!(fog_refine_squared_distance(4, 5), 1);
        // (4, 4) -> 1 + 1 = 2 -> inside.
        assert_eq!(fog_refine_squared_distance(4, 4), 2);
        // (3, 4) -> 4 + 1 = 5 -> still inside (<=5).
        assert_eq!(fog_refine_squared_distance(3, 4), 5);
        assert!(fog_refine_inside_clear_core(3, 4));
        // (3, 3) -> 4 + 4 = 8 -> outside.
        assert_eq!(fog_refine_squared_distance(3, 3), 8);
        assert!(!fog_refine_inside_clear_core(3, 3));
        // Symmetric across the centre — (7, 7) folds to (3, 3).
        assert_eq!(fog_refine_squared_distance(7, 7), 8);
        assert!(!fog_refine_inside_clear_core(7, 7));
    }

    #[test]
    fn local_view_class_for_tile_matches_spec_table_spot_check() {
        // view.md §4
        // Sample one tile from each documented class.
        assert_eq!(local_view_class_for_tile(0x00), LocalViewClass::Empty);
        assert_eq!(local_view_class_for_tile(0xFF), LocalViewClass::Empty);
        assert_eq!(local_view_class_for_tile(0xC0), LocalViewClass::Empty);
        assert_eq!(local_view_class_for_tile(0x05), LocalViewClass::SparseCheckers);
        assert_eq!(local_view_class_for_tile(0x35), LocalViewClass::SparseCheckers);
        assert_eq!(local_view_class_for_tile(0x09), LocalViewClass::SolidFill);
        assert_eq!(local_view_class_for_tile(0x2D), LocalViewClass::SolidFill);
        assert_eq!(local_view_class_for_tile(0x07), LocalViewClass::FilledFrame);
        assert_eq!(local_view_class_for_tile(0x70), LocalViewClass::FilledFrame);
        assert_eq!(local_view_class_for_tile(0x47), LocalViewClass::HorizontalRails);
        assert_eq!(local_view_class_for_tile(0x10), LocalViewClass::CentredBars);
        assert_eq!(local_view_class_for_tile(0x99), LocalViewClass::HollowRectangle);
        assert_eq!(local_view_class_for_tile(0xFE), LocalViewClass::DiagonalStyle);
        assert_eq!(local_view_class_for_tile(0x0B), LocalViewClass::DiagonalStep);
        assert_eq!(local_view_class_for_tile(0x06), LocalViewClass::VegetationHybrid);
        assert_eq!(local_view_class_for_tile(0x60), LocalViewClass::FourCornerRing);
        assert_eq!(local_view_class_for_tile(0x02), LocalViewClass::DiagonalBlits);
        assert_eq!(local_view_class_for_tile(0x01), LocalViewClass::NoopDefault);
        assert_eq!(local_view_class_for_tile(0x04), LocalViewClass::CreatureComposite);
        assert_eq!(local_view_class_for_tile(0xE0), LocalViewClass::VerticalWallDoor);
        assert_eq!(local_view_class_for_tile(0xD8), LocalViewClass::PeerVariant);
        assert_eq!(local_view_class_for_tile(0x20), LocalViewClass::FenceWall);
        // Exhaustive sweep: every tile id classifies (no panic).
        for t in 0u8..=255 {
            let _ = local_view_class_for_tile(t);
        }
    }

    #[test]
    fn wishing_well_keywords_and_view_outcome_match_spec() {
        // view.md §2,§3
        assert_eq!(WISHING_WELL_WISH_KEYWORDS.len(), 6);
        for k in [
            "Corvette",
            "Ferrari",
            "Lamborghini",
            "Lotus",
            "Porsche",
            "Horse",
        ] {
            assert!(wishing_well_wish_accepted(k));
            assert!(wishing_well_wish_accepted(&k.to_lowercase()));
            assert!(wishing_well_wish_accepted(&k.to_uppercase()));
        }
        assert!(!wishing_well_wish_accepted(""));
        assert!(!wishing_well_wish_accepted("Avatar"));
        assert!(!wishing_well_wish_accepted("Anything"));

        // V-View outcome.
        // Combat short-circuits even when gems are present.
        assert_eq!(
            view_command_outcome(0, true),
            ViewCommandOutcome::CombatLabelOnly
        );
        assert_eq!(
            view_command_outcome(5, true),
            ViewCommandOutcome::CombatLabelOnly
        );
        // Outside combat: zero gems refuse, otherwise enter overlay.
        assert_eq!(
            view_command_outcome(0, false),
            ViewCommandOutcome::NoGemRefusal
        );
        assert_eq!(
            view_command_outcome(1, false),
            ViewCommandOutcome::EnterOverlay
        );
        assert_eq!(
            view_command_outcome(99, false),
            ViewCommandOutcome::EnterOverlay
        );
    }

    #[test]
    fn yell_input_max_len_and_context_variants_match_spec() {
        // commands.md §11
        assert_eq!(YELL_INPUT_MAX_LEN, 30);
        // The three context families are distinct.
        let all = [
            YellInputContext::ShadowlordName,
            YellInputContext::WordOfPower,
            YellInputContext::NoEffect,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn pushable_tile_family_classifies_per_spec_table() {
        // commands.md §8
        assert_eq!(
            pushable_tile_family(0x5B),
            Some(PushableTileFamily::NonRotating5B)
        );
        for t in 0x90u8..=0x93 {
            assert_eq!(
                pushable_tile_family(t),
                Some(PushableTileFamily::ChairFourFacing)
            );
        }
        for t in [0xA5u8, 0xA6, 0xA8, 0xA9] {
            assert_eq!(
                pushable_tile_family(t),
                Some(PushableTileFamily::NonRotatingA5A6A8A9)
            );
        }
        for t in 0xADu8..=0xAF {
            assert_eq!(
                pushable_tile_family(t),
                Some(PushableTileFamily::NonRotatingAdAf)
            );
        }
        for t in 0xB4u8..=0xB7 {
            assert_eq!(
                pushable_tile_family(t),
                Some(PushableTileFamily::CannonFourFacing)
            );
        }
        // Adjacent non-pushable tiles return None.
        assert_eq!(pushable_tile_family(0x5A), None);
        assert_eq!(pushable_tile_family(0x5C), None);
        assert_eq!(pushable_tile_family(0x8F), None);
        assert_eq!(pushable_tile_family(0x94), None);
        assert_eq!(pushable_tile_family(0xA7), None);
        assert_eq!(pushable_tile_family(0xAA), None);
        assert_eq!(pushable_tile_family(0xAC), None);
        assert_eq!(pushable_tile_family(0xB0), None);
        assert_eq!(pushable_tile_family(0xB8), None);

        // Only the four-facing families rewrite facing bits on success.
        assert!(PushableTileFamily::ChairFourFacing.rewrites_facing());
        assert!(PushableTileFamily::CannonFourFacing.rewrites_facing());
        assert!(!PushableTileFamily::NonRotating5B.rewrites_facing());
        assert!(!PushableTileFamily::NonRotatingA5A6A8A9.rewrites_facing());
        assert!(!PushableTileFamily::NonRotatingAdAf.rewrites_facing());
    }

    #[test]
    fn new_order_swap_accepted_refuses_leader_slot() {
        // commands.md §6
        // Leader slot 0 is refused on either side.
        assert!(!new_order_swap_accepted(0, 1));
        assert!(!new_order_swap_accepted(1, 0));
        assert!(!new_order_swap_accepted(0, 0));
        // Non-leader pairs are accepted.
        for a in 1usize..6 {
            for b in 1usize..6 {
                assert!(new_order_swap_accepted(a, b));
            }
        }
    }

    #[test]
    fn named_scene_byte_constants_match_npc_roster_table() {
        // catalogs/npc-roster.md §1
        // Cross-check against the existing town_resident_name table.
        let pairs = [
            (SCENE_MOONGLOW, "Moonglow"),
            (SCENE_BRITAIN, "Britain"),
            (SCENE_JHELOM, "Jhelom"),
            (SCENE_YEW, "Yew"),
            (SCENE_MINOC, "Minoc"),
            (SCENE_TRINSIC, "Trinsic"),
            (SCENE_SKARA_BRAE, "Skara Brae"),
            (SCENE_NEW_MAGINCIA, "New Magincia"),
            (SCENE_FOGSBANE, "Fogsbane"),
            (SCENE_STORMCROW, "Stormcrow"),
            (SCENE_GREYHAVEN, "Greyhaven"),
            (SCENE_WAVEGUIDE, "Waveguide"),
            (SCENE_IOLOS_HUT, "Iolo's Hut"),
            (SCENE_LORD_BRITISHS_CASTLE, "Lord British's Castle"),
            (SCENE_LORD_BLACKTHORNS_CASTLE, "Lord Blackthorn's Castle"),
            (SCENE_WEST_BRITANNY, "West Britanny"),
            (SCENE_NORTH_BRITANNY, "North Britanny"),
            (SCENE_EAST_BRITANNY, "East Britanny"),
            (SCENE_PAWS, "Paws"),
            (SCENE_COVE, "Cove"),
            (SCENE_BUCCANEERS_DEN, "Buccaneer's Den"),
            (SCENE_ARARAT, "Ararat"),
            (SCENE_BORDERMARCH, "Bordermarch"),
            (SCENE_FARTHING, "Farthing"),
            (SCENE_WINDEMERE, "Windemere"),
            (SCENE_STONEGATE, "Stonegate"),
            (SCENE_THE_LYCAEUM, "The Lycaeum"),
            (SCENE_EMPATH_ABBEY, "Empath Abbey"),
            (SCENE_SERPENTS_HOLD, "Serpent's Hold"),
        ];
        for (byte, expected_name) in pairs {
            assert_eq!(
                town_resident_name(byte).map(str::to_owned),
                Some(expected_name.to_string()),
                "scene byte {byte}"
            );
        }
        // Lord British's Castle scene is the spec's confirmed Sandalwood
        // Box pickup scene.
        assert_eq!(SCENE_LORD_BRITISHS_CASTLE, 17);
    }

    #[test]
    fn outdoor_movement_chance_gate_classifies_destination_per_spec() {
        // active-objects.md §8
        // OneInTwo: 0x04, 0x06..=0x08, 0x1E..=0x1F.
        assert_eq!(
            outdoor_movement_chance_gate(0x04),
            OutdoorMovementChanceGate::OneInTwo
        );
        for t in 0x06u8..=0x08 {
            assert_eq!(
                outdoor_movement_chance_gate(t),
                OutdoorMovementChanceGate::OneInTwo
            );
        }
        for t in 0x1Eu8..=0x1F {
            assert_eq!(
                outdoor_movement_chance_gate(t),
                OutdoorMovementChanceGate::OneInTwo
            );
        }
        // OneInThree: 0x09..=0x0F.
        for t in 0x09u8..=0x0F {
            assert_eq!(
                outdoor_movement_chance_gate(t),
                OutdoorMovementChanceGate::OneInThree
            );
        }
        // Immediate: 0x05, 0x10..=0x1D, plus everything outside 0x04..=0x1F.
        for t in [0x00u8, 0x05, 0x10, 0x1D, 0x20, 0x80, 0xDC, 0xFF] {
            assert_eq!(
                outdoor_movement_chance_gate(t),
                OutdoorMovementChanceGate::Immediate,
                "tile {t:#x}"
            );
        }

        // Auto-clear destination tile / age cap constants.
        assert_eq!(OUTDOOR_STEP_CLEAR_DESTINATION_TILE, 0xDC);
        assert_eq!(FC_PROXIMITY_AGE_CAP, 20);
    }

    #[test]
    fn outdoor_serpent_dragon_trigger_and_whirlpool_constants_match_spec() {
        // active-objects.md §8
        assert_eq!(OUTDOOR_SERPENT_DRAGON_TRIGGER_DENOMINATOR, 7);
        assert!(outdoor_serpent_dragon_triggers(0));
        for r in 1u8..=6 {
            assert!(!outdoor_serpent_dragon_triggers(r));
        }
        // Adjacency radius for ship-like water-creature attack message.
        assert_eq!(OUTDOOR_WATER_CREATURE_ADJACENCY_RADIUS, 3);
        // Whirlpool emergence coordinate matches the documented underworld
        // entry coordinate.
        assert_eq!(WHIRLPOOL_EMERGENCE_X, 34);
        assert_eq!(WHIRLPOOL_EMERGENCE_Y, 18);
    }

    #[test]
    fn tlk_print_mask_toggle_pairs_per_spec() {
        // conversation.md §7.5
        let normal = TlkPrintMaskState::NormalBreaks;
        let protected = normal.toggle();
        assert_eq!(protected, TlkPrintMaskState::ProtectedRun);
        // Matched 0x8E pair returns to the default.
        assert_eq!(protected.toggle(), TlkPrintMaskState::NormalBreaks);

        // Only the default state flushes on soft-break bytes.
        assert!(TlkPrintMaskState::NormalBreaks.flushes_on_break());
        assert!(!TlkPrintMaskState::ProtectedRun.flushes_on_break());
    }

    #[test]
    fn tlk_label_index_decodes_label_byte_range() {
        // conversation.md §7.7
        assert_eq!(TLK_LABEL_BYTE_COUNT, 15);
        // Label bytes 0x91..=0x9F decode to indices 0..=14.
        for (offset, byte) in (TLK_LABEL_FIRST..=TLK_LABEL_LAST).enumerate() {
            assert_eq!(tlk_label_index(byte), Some(offset as u8));
        }
        // Non-label bytes return None.
        assert_eq!(tlk_label_index(0x90), None);
        assert_eq!(tlk_label_index(0xA0), None);
        assert_eq!(tlk_label_index(0x00), None);
        assert_eq!(tlk_label_index(0xFF), None);
    }

    #[test]
    fn shrine_mantra_table_matches_spec() {
        // karma.md §7
        assert_eq!(SHRINE_MANTRA_INPUT_LIMIT, 12);
        assert_eq!(shrine_mantra_for(ShrineVirtue::Honesty), "Ahm");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Compassion), "Mu");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Valor), "Ra");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Justice), "Beh");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Sacrifice), "Cah");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Honor), "Summ");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Spirituality), "Om");
        assert_eq!(shrine_mantra_for(ShrineVirtue::Humility), "Lum");
        // All mantras fit inside the input cap.
        for v in [
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
            ShrineVirtue::Humility,
        ] {
            assert!(shrine_mantra_for(v).len() <= SHRINE_MANTRA_INPUT_LIMIT);
        }
    }

    #[test]
    fn codex_turnin_stat_reward_matches_spec_table() {
        // karma.md §7
        assert_eq!(CODEX_TURNIN_STAT_INCREMENT, 1);
        assert_eq!(CODEX_TURNIN_STAT_CAP, 30);
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Honesty), (0, 0, 1));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Compassion), (0, 1, 0));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Valor), (1, 0, 0));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Justice), (0, 1, 1));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Sacrifice), (1, 1, 0));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Honor), (1, 0, 1));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Spirituality), (1, 1, 1));
        assert_eq!(codex_turnin_stat_reward(ShrineVirtue::Humility), (0, 0, 0));
        // Cross-check: Humility is the only zero-reward virtue.
        for v in [
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
        ] {
            let (s, d, i) = codex_turnin_stat_reward(v);
            assert!(s + d + i > 0, "{v:?} should grant a stat reward");
        }
    }

    #[test]
    fn blackthorn_challenge_prompt_table_matches_spec() {
        // blackthorn.md §4
        assert_eq!(BLACKTHORN_CHALLENGE_PROMPT_TABLE.len(), 4);
        assert_eq!(blackthorn_challenge_prompt(0), Some(("Honesty", "Ahm")));
        assert_eq!(blackthorn_challenge_prompt(1), Some(("Compassion", "Mu")));
        assert_eq!(blackthorn_challenge_prompt(2), Some(("Valour", "Ra")));
        assert_eq!(blackthorn_challenge_prompt(3), Some(("Justice", "Beh")));
        // The traced loop only iterates the first four ordinals.
        assert_eq!(blackthorn_challenge_prompt(4), None);
        assert_eq!(blackthorn_challenge_prompt(7), None);
        assert_eq!(blackthorn_challenge_prompt(255), None);
        // Cross-check: matches answer-comparison contract.
        for ord in 0u8..4 {
            let (_, expected) = blackthorn_challenge_prompt(ord).unwrap();
            assert!(blackthorn_challenge_answer_matches(expected, expected));
        }
    }

    #[test]
    fn u4_transfer_source_validation_gates_match_spec() {
        // u4-transfer.md §5
        assert_eq!(U4_TRANSFER_GOLD_GEM_FOOD_MAX, 9999);
        assert_eq!(U4_TRANSFER_MOVE_MOON_DUNGEON_MAX, 70);
        assert_eq!(U4_TRANSFER_CLASS_INDEX_MAX, 7);

        // Gold/gem/food range gate.
        assert!(u4_transfer_gold_gem_food_in_range(0));
        assert!(u4_transfer_gold_gem_food_in_range(9999));
        assert!(!u4_transfer_gold_gem_food_in_range(10000));
        assert!(!u4_transfer_gold_gem_food_in_range(65535));

        // Move/moon/dungeon range gate.
        assert!(u4_transfer_move_moon_dungeon_in_range(0));
        assert!(u4_transfer_move_moon_dungeon_in_range(70));
        assert!(!u4_transfer_move_moon_dungeon_in_range(71));
        assert!(!u4_transfer_move_moon_dungeon_in_range(255));

        // Class index range gate.
        for c in 0u8..=7 {
            assert!(u4_transfer_class_index_in_range(c));
        }
        assert!(!u4_transfer_class_index_in_range(8));
        assert!(!u4_transfer_class_index_in_range(255));

        // Name-byte gate: NUL + printable accepted; control bytes rejected.
        assert!(u4_transfer_name_byte_accepted(0));
        assert!(u4_transfer_name_byte_accepted(b'A'));
        assert!(u4_transfer_name_byte_accepted(b' '));
        assert!(u4_transfer_name_byte_accepted(b'~'));
        assert!(!u4_transfer_name_byte_accepted(0x01));
        assert!(!u4_transfer_name_byte_accepted(0x1F));
        assert!(!u4_transfer_name_byte_accepted(0x7F));
        assert!(!u4_transfer_name_byte_accepted(0xFF));
    }

    #[test]
    fn input_function_key_remap_and_cursor_blink_match_spec() {
        // input.md §3,§4
        assert_eq!(INPUT_CODE_F1, 0xC9);
        assert_eq!(INPUT_CODE_F10, 0xD2);
        assert_eq!(INPUT_CODE_FUNCTION_FIRST, INPUT_CODE_F1);
        assert_eq!(INPUT_CODE_FUNCTION_LAST, INPUT_CODE_F10);
        assert_eq!(CURSOR_BLINK_BASE_GLYPH, 4);
        assert_eq!(CURSOR_BLINK_MODULUS, 4658);

        // F1..F10 -> 1..=10.
        assert_eq!(input_function_key_index(0xC9), Some(1));
        assert_eq!(input_function_key_index(0xCA), Some(2));
        assert_eq!(input_function_key_index(0xD2), Some(10));
        // Out-of-range bytes return None.
        assert_eq!(input_function_key_index(0xC8), None);
        assert_eq!(input_function_key_index(0xD3), None); // northwest direction code
        assert_eq!(input_function_key_index(0xFF), None);
        assert_eq!(input_function_key_index(b'A'), None);
    }

    #[test]
    fn town_tile_marker_classifies_harvested_bytes() {
        // town-mode.md §3
        assert_eq!(TOWN_TILE_NPC_START_A, 0x48);
        assert_eq!(TOWN_TILE_NPC_START_B, 0x49);
        assert_eq!(TOWN_TILE_SPAWN_ASTERISK, b'*');
        assert_eq!(TOWN_TILE_DASH_MARKER, b'-');
        assert_eq!(TOWN_TILE_PERIOD_MARKER, b'.');

        assert_eq!(town_tile_marker(0x48), Some(TownTileMarker::NpcStartA));
        assert_eq!(town_tile_marker(0x49), Some(TownTileMarker::NpcStartB));
        assert_eq!(town_tile_marker(b'*'), Some(TownTileMarker::SpawnAsterisk));
        assert_eq!(town_tile_marker(b'-'), Some(TownTileMarker::DashCosmetic));
        assert_eq!(town_tile_marker(b'.'), Some(TownTileMarker::PeriodCosmetic));
        assert_eq!(town_tile_marker(0xC8), Some(TownTileMarker::FloorLinkC8));
        assert_eq!(town_tile_marker(0xC9), Some(TownTileMarker::FloorLinkC9));
        // Ordinary terrain bytes are not markers.
        assert_eq!(town_tile_marker(0x00), None);
        assert_eq!(town_tile_marker(0x01), None);
        assert_eq!(town_tile_marker(0x47), None);
        assert_eq!(town_tile_marker(0x4A), None);
        assert_eq!(town_tile_marker(b' '), None);
        assert_eq!(town_tile_marker(0xC7), None);
        assert_eq!(town_tile_marker(0xCA), None);
        assert_eq!(town_tile_marker(0xFF), None);
    }

    #[test]
    fn active_object_eviction_off_screen_matches_spec_radius() {
        // active-objects.md §4
        assert_eq!(ACTIVE_OBJECT_EVICTION_OFFSCREEN_RADIUS, 5);
        // Inside radius -> on-screen.
        assert!(!active_object_eviction_off_screen(50, 50, 50, 50));
        assert!(!active_object_eviction_off_screen(45, 50, 50, 50));
        assert!(!active_object_eviction_off_screen(55, 50, 50, 50));
        assert!(!active_object_eviction_off_screen(50, 55, 50, 50));
        // At the radius -> still on-screen (strictly greater than).
        assert!(!active_object_eviction_off_screen(45, 55, 50, 50));
        // Past the radius in either axis -> off-screen.
        assert!(active_object_eviction_off_screen(44, 50, 50, 50));
        assert!(active_object_eviction_off_screen(56, 50, 50, 50));
        assert!(active_object_eviction_off_screen(50, 44, 50, 50));
        assert!(active_object_eviction_off_screen(50, 56, 50, 50));
        // Far away in both axes.
        assert!(active_object_eviction_off_screen(10, 10, 100, 100));
    }

    #[test]
    fn active_object_pass_order_matches_spec() {
        // active-objects.md §2
        let (start, end, descending) =
            ActiveObjectPassOrder::RendererHighToLow.iteration();
        assert_eq!(start, OOL_SLOTS - 1);
        assert_eq!(end, 0);
        assert!(descending);

        let (start, end, descending) =
            ActiveObjectPassOrder::AnimatorLowToHigh.iteration();
        assert_eq!(start, 0);
        assert_eq!(end, OOL_SLOTS - 1);
        assert!(!descending);
    }

    #[test]
    fn live_chunk_substituted_tile_matches_spec_rules() {
        // overworld.md §3
        assert_eq!(LIVE_CHUNK_SUBSTITUTION_TARGET_DF, 0xDF);
        assert_eq!(LIVE_CHUNK_SUBSTITUTION_TARGET_1A, 0x1A);
        // 0x16..=0x18 rewrite unconditionally to 0xDF.
        for tile in 0x16u8..=0x18 {
            assert_eq!(live_chunk_substituted_tile(tile, true), 0xDF);
            assert_eq!(live_chunk_substituted_tile(tile, false), 0xDF);
        }
        // 0x19 rewrites only when the classifier accepts.
        assert_eq!(live_chunk_substituted_tile(0x19, true), 0x1A);
        assert_eq!(live_chunk_substituted_tile(0x19, false), 0x19);
        // Other tiles pass through under both classifier states.
        for tile in [0x00u8, 0x01, 0x15, 0x1A, 0x1B, 0xDE, 0xE0, 0xFF] {
            assert_eq!(live_chunk_substituted_tile(tile, true), tile);
            assert_eq!(live_chunk_substituted_tile(tile, false), tile);
        }
    }

    #[test]
    fn calendar_thresholds_and_display_hour_match_spec() {
        // time.md §2,§5
        assert_eq!(MINUTES_PER_HOUR, 60);
        assert_eq!(HOURS_PER_DAY, 24);
        assert_eq!(DAYS_PER_MONTH, 28);
        assert_eq!(MONTHS_PER_YEAR, 13);

        // Display hour rule.
        assert_eq!(display_hour_12h(0), 12);
        assert_eq!(display_hour_12h(1), 1);
        assert_eq!(display_hour_12h(11), 11);
        assert_eq!(display_hour_12h(12), 12);
        assert_eq!(display_hour_12h(13), 1);
        assert_eq!(display_hour_12h(23), 11);
        // GameClock's instance method returns the same value.
        let clock = GameClock::with_date(139, 4, 5, 0, 0).unwrap();
        assert_eq!(clock.display_hour(), display_hour_12h(0));
    }

    #[test]
    fn command_dispatch_status_predicates_match_spec() {
        // main-loop.md §6,§7
        // Only ConsumesTurn runs the per-turn epilogue.
        assert!(CommandDispatchStatus::ConsumesTurn.runs_per_turn_epilogue());
        assert!(!CommandDispatchStatus::NoTurn.runs_per_turn_epilogue());
        assert!(!CommandDispatchStatus::BufferToggle.runs_per_turn_epilogue());
        assert!(!CommandDispatchStatus::RepollNoRedraw.runs_per_turn_epilogue());

        // Only RepollNoRedraw suppresses the redraw.
        assert!(CommandDispatchStatus::ConsumesTurn.requests_redraw());
        assert!(CommandDispatchStatus::NoTurn.requests_redraw());
        assert!(CommandDispatchStatus::BufferToggle.requests_redraw());
        assert!(!CommandDispatchStatus::RepollNoRedraw.requests_redraw());
    }

    #[test]
    fn machine_class_probe_predicates_match_spec() {
        // boot.md §3
        // PCjr-class skips the extended graphics probe.
        assert!(MachineClass::PcOrPcjr.skips_extended_graphics_probe());
        assert!(!MachineClass::At.skips_extended_graphics_probe());
        assert!(!MachineClass::Tandy1000.skips_extended_graphics_probe());
        assert!(!MachineClass::OtherOrGenericXt.skips_extended_graphics_probe());

        // Tandy 1000 ROM-signature hit forces Tandy graphics.
        assert!(MachineClass::Tandy1000.forces_tandy_graphics());
        assert!(!MachineClass::PcOrPcjr.forces_tandy_graphics());
        assert!(!MachineClass::At.forces_tandy_graphics());
        assert!(!MachineClass::OtherOrGenericXt.forces_tandy_graphics());
    }

    #[test]
    fn tlk_leading_entries_match_spec_disk_order() {
        // conversation.md §4
        assert_eq!(TLK_LEADING_ENTRY_COUNT, 5);
        assert_eq!(tlk_leading_entry_index(TlkLeadingEntry::Name), 0);
        assert_eq!(tlk_leading_entry_index(TlkLeadingEntry::Description), 1);
        assert_eq!(tlk_leading_entry_index(TlkLeadingEntry::Greeting), 2);
        assert_eq!(tlk_leading_entry_index(TlkLeadingEntry::Job), 3);
        assert_eq!(tlk_leading_entry_index(TlkLeadingEntry::Bye), 4);
        // All five indices are inside the leading band.
        for entry in [
            TlkLeadingEntry::Name,
            TlkLeadingEntry::Description,
            TlkLeadingEntry::Greeting,
            TlkLeadingEntry::Job,
            TlkLeadingEntry::Bye,
        ] {
            assert!(tlk_leading_entry_index(entry) < TLK_LEADING_ENTRY_COUNT);
        }
    }

    #[test]
    fn conversation_cleanup_sentinel_zero_allows_warning_pass() {
        // quest-flags.md §5
        assert_eq!(CONVERSATION_CLEANUP_SENTINEL_ALLOW, 0);
        assert_eq!(CONVERSATION_CLEANUP_GOLD_DEBIT_MAX, 15);
        assert!(conversation_cleanup_runs_warning(0));
        // Slot 1, slot 2, and the no-slot marker all suppress.
        for v in 1u8..=255 {
            assert!(!conversation_cleanup_runs_warning(v), "value {v}");
        }
    }

    #[test]
    fn title_tick_frame_rectangle_and_cadence_match_spec() {
        // intro.md §5
        assert_eq!(TITLE_TICK_FRAME_X, 0);
        assert_eq!(TITLE_TICK_FRAME_Y, 65);
        assert_eq!(TITLE_TICK_FRAME_WIDTH, 320);
        assert_eq!(TITLE_TICK_FRAME_HEIGHT, 49);
        assert_eq!(TITLE_TICK_FRAME_COUNT, 4);
        // Frame index advances modulo 4.
        assert_eq!(title_tick_next_frame(0), 1);
        assert_eq!(title_tick_next_frame(1), 2);
        assert_eq!(title_tick_next_frame(2), 3);
        assert_eq!(title_tick_next_frame(3), 0);
        // Rectangle stays inside the 320x200 title surface.
        assert!(TITLE_TICK_FRAME_X + TITLE_TICK_FRAME_WIDTH <= 320);
        assert!(TITLE_TICK_FRAME_Y + TITLE_TICK_FRAME_HEIGHT <= 200);
    }

    #[test]
    fn dungeon_facing_helpers_match_spec() {
        // dungeon-mode.md §9
        // Forward delta per facing.
        assert_eq!(dungeon_facing_forward_delta(DUNGEON_FACING_NORTH), Some((0, -1)));
        assert_eq!(dungeon_facing_forward_delta(DUNGEON_FACING_EAST), Some((1, 0)));
        assert_eq!(dungeon_facing_forward_delta(DUNGEON_FACING_SOUTH), Some((0, 1)));
        assert_eq!(dungeon_facing_forward_delta(DUNGEON_FACING_WEST), Some((-1, 0)));
        assert_eq!(dungeon_facing_forward_delta(4), None);

        // Back delta is negation of forward.
        for f in 0u8..4 {
            let fwd = dungeon_facing_forward_delta(f).unwrap();
            let bwd = dungeon_facing_back_delta(f).unwrap();
            assert_eq!((bwd.0, bwd.1), (-fwd.0, -fwd.1));
        }

        // Turning rotates facing modulo 4.
        assert_eq!(dungeon_facing_turn_left(DUNGEON_FACING_NORTH), DUNGEON_FACING_WEST);
        assert_eq!(dungeon_facing_turn_left(DUNGEON_FACING_WEST), DUNGEON_FACING_SOUTH);
        assert_eq!(dungeon_facing_turn_right(DUNGEON_FACING_NORTH), DUNGEON_FACING_EAST);
        assert_eq!(dungeon_facing_turn_right(DUNGEON_FACING_WEST), DUNGEON_FACING_NORTH);
        assert_eq!(dungeon_facing_turn_around(DUNGEON_FACING_NORTH), DUNGEON_FACING_SOUTH);
        assert_eq!(dungeon_facing_turn_around(DUNGEON_FACING_EAST), DUNGEON_FACING_WEST);

        // Two left turns == one turnaround.
        for f in 0u8..4 {
            let two_left = dungeon_facing_turn_left(dungeon_facing_turn_left(f));
            assert_eq!(two_left, dungeon_facing_turn_around(f));
        }
    }

    #[test]
    fn dungeon_chest_trap_tier_matches_spec_bands() {
        // dungeon-mode.md §8
        // Tier < 4 -> Simple.
        for t in 0u8..4 {
            assert_eq!(dungeon_chest_trap_tier(t), DungeonChestTrapTier::Simple);
        }
        // Tier 4..6 -> Generic.
        for t in 4u8..7 {
            assert_eq!(dungeon_chest_trap_tier(t), DungeonChestTrapTier::Generic);
        }
        // Tier >= 7 -> Complex.
        for t in 7u8..=20 {
            assert_eq!(dungeon_chest_trap_tier(t), DungeonChestTrapTier::Complex);
        }
        assert_eq!(dungeon_chest_trap_tier(255), DungeonChestTrapTier::Complex);
    }

    #[test]
    fn dungeon_presentation_flavour_matches_spec_table() {
        // dungeon-mode.md §2
        let cases = [
            (0u8, DungeonPresentationFlavour::FlavourByte3), // Deceit
            (1, DungeonPresentationFlavour::Normal),         // Despise
            (2, DungeonPresentationFlavour::Normal),         // Destard
            (3, DungeonPresentationFlavour::FlavourByte3),   // Wrong
            (4, DungeonPresentationFlavour::FlavourByte3),   // Covetous
            (5, DungeonPresentationFlavour::Mine),           // Shame
            (6, DungeonPresentationFlavour::Mine),           // Hythloth
            (7, DungeonPresentationFlavour::Normal),         // Doom
        ];
        for (record, expected) in cases {
            let scene = DungeonScene::from_record(record).expect("valid record");
            assert_eq!(
                scene.presentation_flavour(),
                expected,
                "record {record}"
            );
        }
    }

    #[test]
    fn npc_schedule_state_for_floor_transition_matches_spec_table() {
        // npc-schedules.md §6 floor-classification table (map = floor 1).
        // both on map -> in-plane (2)
        assert_eq!(
            npc_schedule_state_for_floor_transition(1, 1, 1),
            NPC_STATE_INPLANE_MOVE
        );
        // NPC on map, target above (z < 1) -> climb-up (6)
        assert_eq!(
            npc_schedule_state_for_floor_transition(1, 0, 1),
            NPC_STATE_CLIMB_UP_OFF_FLOOR
        );
        // NPC on map, target below (z > 1) -> climb-down (7)
        assert_eq!(
            npc_schedule_state_for_floor_transition(1, 2, 1),
            NPC_STATE_CLIMB_DOWN_OFF_FLOOR
        );
        // NPC above, target on map -> ascend (5)
        assert_eq!(
            npc_schedule_state_for_floor_transition(0, 1, 1),
            NPC_STATE_ASCEND_TOWARD_TARGET
        );
        // NPC below, target on map -> descend (4)
        assert_eq!(
            npc_schedule_state_for_floor_transition(2, 1, 1),
            NPC_STATE_DESCEND_TOWARD_TARGET
        );
        // Neither on map -> parked off-floor (8)
        assert_eq!(
            npc_schedule_state_for_floor_transition(0, 2, 1),
            NPC_STATE_PARKED_OFF_FLOOR
        );
        assert_eq!(
            npc_schedule_state_for_floor_transition(2, 0, 1),
            NPC_STATE_PARKED_OFF_FLOOR
        );
        assert_eq!(
            npc_schedule_state_for_floor_transition(2, 2, 1),
            NPC_STATE_PARKED_OFF_FLOOR
        );
    }

    #[test]
    fn npc_schedule_waypoint_resolves_per_spec_segments() {
        // npc-schedules.md §3
        // A typical baker schedule: 06 morning -> waypoint 0, 12 noon
        // -> waypoint 1, 18 evening -> waypoint 2, 22 night-home -> wp 1.
        let time = [6u8, 12, 18, 22];
        // 06..=11 in segment [time[0], time[1]) -> waypoint 0.
        for h in 6u8..12 {
            assert_eq!(npc_schedule_waypoint_for_hour(time, h), 0, "hour {h}");
        }
        // 12..=17 -> waypoint 1.
        for h in 12u8..18 {
            assert_eq!(npc_schedule_waypoint_for_hour(time, h), 1, "hour {h}");
        }
        // 18..=21 -> waypoint 2.
        for h in 18u8..22 {
            assert_eq!(npc_schedule_waypoint_for_hour(time, h), 2, "hour {h}");
        }
        // 22..=23 (after the last boundary, in the wrap segment) -> waypoint 1.
        for h in 22u8..24 {
            assert_eq!(npc_schedule_waypoint_for_hour(time, h), 1, "hour {h}");
        }
        // 0..=5 (still in the wrap segment, before time[0]) -> waypoint 1.
        for h in 0u8..6 {
            assert_eq!(npc_schedule_waypoint_for_hour(time, h), 1, "hour {h}");
        }
    }

    #[test]
    fn monster_kill_xp_reward_matches_spec() {
        // combat.md §12 — quarter of max HP plus one.
        assert_eq!(monster_kill_xp_reward(0), 1);
        assert_eq!(monster_kill_xp_reward(3), 1); // 0 + 1
        assert_eq!(monster_kill_xp_reward(4), 2); // 1 + 1
        assert_eq!(monster_kill_xp_reward(40), 11);
        assert_eq!(monster_kill_xp_reward(100), 26);
        assert_eq!(monster_kill_xp_reward(255), 64);
        // u16::MAX yields (65535/4) + 1 = 16384.
        assert_eq!(monster_kill_xp_reward(u16::MAX), 16384);
    }

    #[test]
    fn fire_and_energy_field_raw_damage_match_spec() {
        // combat.md §11
        assert_eq!(FIRE_FIELD_DAMAGE_MIN, 1);
        assert_eq!(FIRE_FIELD_DAMAGE_MAX, 21);
        assert_eq!(ENERGY_FIELD_RAW_DAMAGE, 0);
        // Fire field rolls range 1..=21.
        for seed in 0u8..=20 {
            let dmg = fire_field_raw_damage(seed);
            assert!(
                (FIRE_FIELD_DAMAGE_MIN..=FIRE_FIELD_DAMAGE_MAX).contains(&dmg),
                "seed {seed} -> dmg {dmg}"
            );
        }
        assert_eq!(fire_field_raw_damage(0), 1);
        assert_eq!(fire_field_raw_damage(20), 21);
        // Modulo wraparound covers larger seeds without overflow.
        assert_eq!(fire_field_raw_damage(21), 1);
        assert_eq!(fire_field_raw_damage(42), 1);
    }

    #[test]
    fn monster_wound_classifier_matches_spec_thresholds() {
        // combat.md §9
        assert_eq!(WOUND_MORALE_FLEE_THRESHOLD, 252);
        // 100 HP class.
        assert_eq!(monster_wound_bucket(0, 100), MonsterWoundBucket::Critical);
        assert_eq!(monster_wound_bucket(24, 100), MonsterWoundBucket::Critical);
        assert_eq!(monster_wound_bucket(25, 100), MonsterWoundBucket::Wounded);
        assert_eq!(monster_wound_bucket(49, 100), MonsterWoundBucket::Wounded);
        assert_eq!(
            monster_wound_bucket(50, 100),
            MonsterWoundBucket::LightlyWounded
        );
        assert_eq!(
            monster_wound_bucket(74, 100),
            MonsterWoundBucket::LightlyWounded
        );
        assert_eq!(monster_wound_bucket(75, 100), MonsterWoundBucket::Healthy);
        assert_eq!(monster_wound_bucket(100, 100), MonsterWoundBucket::Healthy);
        // Zero max -> Critical edge.
        assert_eq!(monster_wound_bucket(0, 0), MonsterWoundBucket::Critical);

        // Critical band always sets fleeing.
        assert!(monster_wound_sets_fleeing(0, 100, 0));
        assert!(monster_wound_sets_fleeing(0, 100, 255));
        // Wounded band: fleeing on rolls 0..=251 (252 outcomes), clear on 252..=255.
        assert!(monster_wound_sets_fleeing(30, 100, 0));
        assert!(monster_wound_sets_fleeing(30, 100, 251));
        assert!(!monster_wound_sets_fleeing(30, 100, 252));
        assert!(!monster_wound_sets_fleeing(30, 100, 255));
        // Lightly wounded / healthy: never fleeing.
        assert!(!monster_wound_sets_fleeing(50, 100, 0));
        assert!(!monster_wound_sets_fleeing(80, 100, 251));
    }

    #[test]
    fn quickness_skips_player_input_only_with_zero_roll_and_active_tag() {
        // combat.md §8
        // Active tag + zero roll -> skip.
        assert!(quickness_skips_player_input(true, 0));
        // Active tag + nonzero roll -> proceed.
        assert!(!quickness_skips_player_input(true, 1));
        // Inactive tag -> always proceed regardless of roll.
        assert!(!quickness_skips_player_input(false, 0));
        assert!(!quickness_skips_player_input(false, 1));
        assert!(!quickness_skips_player_input(false, 255));
    }

    #[test]
    fn combat_actor_record_offsets_match_spec_row_order() {
        // combat.md §6
        assert_eq!(COMBAT_ACTOR_RECORD_LEN, 8);
        assert_eq!(CombatActorField::Hp.offset(), 0);
        assert_eq!(CombatActorField::BaseStep.offset(), 1);
        assert_eq!(CombatActorField::Flags.offset(), 2);
        assert_eq!(CombatActorField::OwnerTargetClass.offset(), 3);
        assert_eq!(CombatActorField::Backref.offset(), 4);
        assert_eq!(CombatActorField::Phase.offset(), 5);
        assert_eq!(CombatActorField::ArenaX.offset(), 6);
        assert_eq!(CombatActorField::ArenaY.offset(), 7);
        // All offsets are inside the descriptor.
        for field in [
            CombatActorField::Hp,
            CombatActorField::BaseStep,
            CombatActorField::Flags,
            CombatActorField::OwnerTargetClass,
            CombatActorField::Backref,
            CombatActorField::Phase,
            CombatActorField::ArenaX,
            CombatActorField::ArenaY,
        ] {
            assert!(field.offset() < COMBAT_ACTOR_RECORD_LEN);
        }
    }

    #[test]
    fn inn_leave_and_pickup_bills_match_spec_formulas() {
        // shops.md §8.4
        assert_eq!(INN_LEAVE_DEPOSIT_ROOM_RATE_UNITS, 10);
        // Leave deposit = 10 * adjusted room rate.
        assert_eq!(inn_leave_companion_deposit(0), 0);
        assert_eq!(inn_leave_companion_deposit(7), 70);
        assert_eq!(inn_leave_companion_deposit(15), 150);
        // Pickup bill = adjusted lodging * stay (with zero treated as one).
        assert_eq!(inn_pickup_bill(15, 0), 15);
        assert_eq!(inn_pickup_bill(15, 1), 15);
        assert_eq!(inn_pickup_bill(15, 25), 15 * 25);
        assert_eq!(inn_pickup_bill(0, 4), 0);
        assert_eq!(inn_pickup_bill(0, 0), 0);
    }

    #[test]
    fn shoppe_record_cluster_constants_match_spec_table() {
        // shops.md §4 record-id ranges
        assert_eq!(SHOPPE_RECORDS_SHARED_BARKS_FIRST, 0);
        assert_eq!(SHOPPE_RECORDS_SHARED_BARKS_LAST, 7);
        assert_eq!(SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST, 8);
        assert_eq!(SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST, 48);
        assert_eq!(SHOPPE_RECORDS_ARMS_SELL_FIRST, 49);
        assert_eq!(SHOPPE_RECORDS_ARMS_SELL_LAST, 56);
        assert_eq!(SHOPPE_RECORDS_TAVERN_FIRST, 57);
        assert_eq!(SHOPPE_RECORDS_TAVERN_LAST, 88);
        assert_eq!(SHOPPE_RECORDS_SAGE_FIRST, 84);
        assert_eq!(SHOPPE_RECORDS_SAGE_LAST, 91);
        // Sage cluster overlaps the tavern band per spec.
        assert!(SHOPPE_RECORDS_SAGE_FIRST <= SHOPPE_RECORDS_TAVERN_LAST);
        assert!(SHOPPE_RECORDS_SAGE_LAST > SHOPPE_RECORDS_TAVERN_LAST);
        assert_eq!(SHOPPE_RECORDS_HORSE_TRADER_FIRST, 92);
        assert_eq!(SHOPPE_RECORDS_HORSE_TRADER_LAST, 104);
        assert_eq!(SHOPPE_RECORDS_SHIP_BROKER_FIRST, 105);
        assert_eq!(SHOPPE_RECORDS_SHIP_BROKER_LAST, 126);
        assert_eq!(SHOPPE_RECORDS_REAGENT_FIRST, 127);
        assert_eq!(SHOPPE_RECORDS_REAGENT_LAST, 146);
        assert_eq!(SHOPPE_RECORDS_GUILD_FIRST, 148);
        assert_eq!(SHOPPE_RECORDS_GUILD_LAST, 162);
        assert_eq!(SHOPPE_RECORDS_HEALER_FIRST, 163);
        assert_eq!(SHOPPE_RECORDS_HEALER_LAST, 173);
        assert_eq!(SHOPPE_RECORDS_INNKEEPER_FIRST, 174);
        assert_eq!(SHOPPE_RECORDS_INNKEEPER_LAST, 193);
        // Last innkeeper record fits inside the file's record-slot count.
        assert!(SHOPPE_RECORDS_INNKEEPER_LAST < SHOPPE_DAT_RECORD_SLOTS);
    }

    #[test]
    fn combat_interference_blocks_only_when_all_five_conditions_hold() {
        // magic.md §7
        // Happy "interferes" path: all conditions met.
        assert!(combat_interference_blocks(true, true, true, false, 1));
        // Each individually-failing condition keeps the cast running.
        assert!(!combat_interference_blocks(false, true, true, false, 1)); // unmapped target
        assert!(!combat_interference_blocks(true, false, true, false, 1)); // invalid actor
        assert!(!combat_interference_blocks(true, true, false, false, 1)); // hidden/asleep
        assert!(!combat_interference_blocks(true, true, true, true, 1));   // Negate-Time active
        // Distance != 1 fails.
        assert!(!combat_interference_blocks(true, true, true, false, 0));
        assert!(!combat_interference_blocks(true, true, true, false, 2));
        assert!(!combat_interference_blocks(true, true, true, false, 7));
        // Negate-Time suppression overrides distance.
        assert!(!combat_interference_blocks(true, true, true, true, 1));
    }

    #[test]
    fn spell_selector_ignored_letters_match_spec() {
        // magic.md §5
        assert_eq!(SPELL_SELECTOR_MAX_LEN, 4);
        assert_eq!(SPELL_SELECTOR_IGNORED_LETTERS, b"JO");
        assert!(spell_selector_is_ignored(b'J'));
        assert!(spell_selector_is_ignored(b'j'));
        assert!(spell_selector_is_ignored(b'O'));
        assert!(spell_selector_is_ignored(b'o'));
        // Real selector letters pass through.
        assert!(!spell_selector_is_ignored(b'I'));
        assert!(!spell_selector_is_ignored(b'L'));
        assert!(!spell_selector_is_ignored(b'M'));
        assert!(!spell_selector_is_ignored(b'V'));
        assert!(!spell_selector_is_ignored(b'F'));
        assert!(!spell_selector_is_ignored(b'P'));
        assert!(!spell_selector_is_ignored(b'R'));
        // Non-letters are not stored either, but this predicate is
        // letter-only so non-letters are treated as not-ignored.
        assert!(!spell_selector_is_ignored(b'A'));
        assert!(!spell_selector_is_ignored(b'Z'));
    }

    #[test]
    fn spell_circle_for_partitions_48_spells_into_8_circles() {
        // magic.md §4
        // Spell ids 0..=5 are circle 1, 6..=11 circle 2, etc.
        for spell_id in 0u8..48 {
            let expected_circle = spell_id / 6 + 1;
            assert_eq!(spell_circle_for(spell_id), Some(expected_circle));
        }
        assert_eq!(spell_circle_for(48), None);
        assert_eq!(spell_circle_for(255), None);

        // Boundary spells of each circle.
        assert_eq!(spell_circle_for(0), Some(1));
        assert_eq!(spell_circle_for(5), Some(1));
        assert_eq!(spell_circle_for(6), Some(2));
        assert_eq!(spell_circle_for(11), Some(2));
        assert_eq!(spell_circle_for(42), Some(8));
        assert_eq!(spell_circle_for(47), Some(8));

        // Mana cost == circle == minimum caster level.
        for circle in 1u8..=8 {
            assert_eq!(spell_mana_cost(circle), circle);
            assert_eq!(spell_min_caster_level(circle), circle);
        }
    }

    #[test]
    fn ranged_weapon_required_ammo_matches_spec() {
        // inventory.md §6 / catalogs/item-list.md §5
        assert_eq!(ranged_weapon_required_ammo(ITEM_ID_BOW), Some(ITEM_ID_ARROWS));
        assert_eq!(
            ranged_weapon_required_ammo(ITEM_ID_MAGIC_BOW),
            Some(ITEM_ID_ARROWS)
        );
        assert_eq!(
            ranged_weapon_required_ammo(ITEM_ID_CROSSBOW),
            Some(ITEM_ID_QUARRELS)
        );
        // Ammunition rows themselves have no ammo gate.
        assert_eq!(ranged_weapon_required_ammo(ITEM_ID_ARROWS), None);
        assert_eq!(ranged_weapon_required_ammo(ITEM_ID_QUARRELS), None);
        // Non-ranged equipment ids don't gate on ammo.
        for id in 0u8..26 {
            assert_eq!(ranged_weapon_required_ammo(id), None);
        }
        assert_eq!(ranged_weapon_required_ammo(30), None);
        assert_eq!(ranged_weapon_required_ammo(35), None);
        assert_eq!(ranged_weapon_required_ammo(40), None);
        assert_eq!(ranged_weapon_required_ammo(0xFF), None);
    }

    #[test]
    fn equipment_slot_record_offsets_match_spec_table() {
        // inventory.md §3
        assert_eq!(EQUIPMENT_BLOCK_FIRST_OFFSET, 0x19);
        assert_eq!(EQUIPMENT_BLOCK_LEN, 6);
        assert_eq!(EquipmentSlot::Helm.record_offset(), 0x19);
        assert_eq!(EquipmentSlot::BodyArmour.record_offset(), 0x1A);
        assert_eq!(EquipmentSlot::WeaponHand.record_offset(), 0x1B);
        assert_eq!(EquipmentSlot::OffHand.record_offset(), 0x1C);
        assert_eq!(EquipmentSlot::Ring.record_offset(), 0x1D);
        assert_eq!(EquipmentSlot::AmuletOrNeck.record_offset(), 0x1E);
        // ordered() yields slots in record order; offsets contiguous.
        let ordered = EquipmentSlot::ordered();
        for (i, slot) in ordered.iter().enumerate() {
            assert_eq!(slot.block_index(), i);
            assert_eq!(slot.record_offset(), EQUIPMENT_BLOCK_FIRST_OFFSET + i);
        }
    }

    #[test]
    fn chargen_name_field_constants_match_spec() {
        // chargen.md §4
        assert_eq!(CHARGEN_NAME_INPUT_MAX_LEN, 8);
        assert_eq!(CHARGEN_NAME_FIELD_LEN, 9);
        // The save-record character-name field width must accommodate the
        // 8-character input plus the seed-preserved ninth byte.
        assert!(CHARGEN_NAME_FIELD_LEN > CHARGEN_NAME_INPUT_MAX_LEN);
    }

    #[test]
    fn movement_chair_force_reject_exempts_foot_and_0x40() {
        // movement.md §4
        assert_eq!(MOVEMENT_CHAIR_FORCE_REJECT_FIRST, 0x90);
        assert_eq!(MOVEMENT_CHAIR_FORCE_REJECT_LAST, 0x93);

        // Non-exempt query (e.g. mounted horse 0x12) -> reject for 0x90..=0x93.
        for tile in 0x90u8..=0x93 {
            assert!(movement_chair_force_reject_applies(0x12, tile));
            assert!(movement_chair_force_reject_applies(0x14, tile)); // carpet
            assert!(movement_chair_force_reject_applies(0x28, tile)); // skiff
        }
        // On-foot family 0x1C..=0x1F exempt.
        for q in 0x1Cu8..=0x1F {
            for tile in 0x90u8..=0x93 {
                assert!(!movement_chair_force_reject_applies(q, tile));
            }
        }
        // 0x40 query exempt.
        for tile in 0x90u8..=0x93 {
            assert!(!movement_chair_force_reject_applies(0x40, tile));
        }
        // Outside the chair range -> never the force-reject (other rules apply).
        for q in 0u8..0x20u8 {
            assert!(!movement_chair_force_reject_applies(q, 0x8F));
            assert!(!movement_chair_force_reject_applies(q, 0x94));
            assert!(!movement_chair_force_reject_applies(q, 0x00));
            assert!(!movement_chair_force_reject_applies(q, 0xFF));
        }
    }

    #[test]
    fn ship_broadside_constants_and_apply_damage_match_spec() {
        // vehicles.md §7
        assert_eq!(SHIP_BROADSIDE_RANGE_CELLS, 3);
        assert_eq!(SHIP_BROADSIDE_DAMAGE_MIN, 1);
        assert_eq!(SHIP_BROADSIDE_DAMAGE_MAX, 20);
        assert_eq!(SHIP_BROADSIDE_DEPLETION_BYTE_OFFSET, 5);

        // Stays in place when subtraction does not underflow.
        assert_eq!(ship_broadside_apply_damage(100, 20), Some(80));
        assert_eq!(ship_broadside_apply_damage(20, 20), Some(0));
        // Underflow into the high bit clears the slot (None).
        assert_eq!(ship_broadside_apply_damage(0, 1), None);
        assert_eq!(ship_broadside_apply_damage(10, 11), None);
        // Result with high bit set (would imply underflow).
        assert_eq!(ship_broadside_apply_damage(100, 200), None);
    }

    #[test]
    fn ship_boarding_precondition_accepts_documented_starting_states() {
        // vehicles.md §4
        // Foot family fully accepted.
        for b in 0x1Cu8..=0x1F {
            assert!(ship_boarding_precondition_accepts(b));
        }
        // Carpet north and east only.
        assert!(ship_boarding_precondition_accepts(0x14));
        assert!(ship_boarding_precondition_accepts(0x15));
        assert!(!ship_boarding_precondition_accepts(0x16));
        assert!(!ship_boarding_precondition_accepts(0x17));
        // Skiff family fully accepted.
        for b in 0x28u8..=0x2B {
            assert!(ship_boarding_precondition_accepts(b));
        }
        // Mounted-horse / ship / out-of-range refused.
        assert!(!ship_boarding_precondition_accepts(0x12));
        assert!(!ship_boarding_precondition_accepts(0x20));
        assert!(!ship_boarding_precondition_accepts(0x00));
        assert!(!ship_boarding_precondition_accepts(0xFF));

        // Carpet stow-on-board predicate: only N and E.
        assert!(ship_boarding_stows_carpet(0x14));
        assert!(ship_boarding_stows_carpet(0x15));
        assert!(!ship_boarding_stows_carpet(0x16));
        assert!(!ship_boarding_stows_carpet(0x17));
        assert!(!ship_boarding_stows_carpet(0x1C));
        assert!(!ship_boarding_stows_carpet(0x28));
    }

    #[test]
    fn transport_family_classifier_matches_spec_table() {
        // vehicles.md §2
        for b in 0x12u8..=0x13 {
            assert_eq!(transport_family(b), Some(TransportFamily::MountedHorse));
        }
        for b in 0x14u8..=0x17 {
            assert_eq!(transport_family(b), Some(TransportFamily::MagicCarpet));
        }
        for b in 0x1Cu8..=0x1F {
            assert_eq!(transport_family(b), Some(TransportFamily::Foot));
        }
        for b in 0x20u8..=0x23 {
            assert_eq!(transport_family(b), Some(TransportFamily::ShipHoisted));
        }
        for b in 0x24u8..=0x27 {
            assert_eq!(transport_family(b), Some(TransportFamily::ShipFurled));
        }
        for b in 0x28u8..=0x2B {
            assert_eq!(transport_family(b), Some(TransportFamily::Skiff));
        }
        // Out-of-window markers remain opaque.
        assert_eq!(transport_family(0x00), None);
        assert_eq!(transport_family(0x10), None); // riderless horse object
        assert_eq!(transport_family(0x18), None);
        assert_eq!(transport_family(0x1B), None);
        assert_eq!(transport_family(0x2C), None);
        assert_eq!(transport_family(0xFF), None);

        // Facing index = low two bits within each accepted family.
        assert_eq!(transport_facing_index(0x1C), Some(0)); // foot N
        assert_eq!(transport_facing_index(0x1D), Some(1)); // foot E
        assert_eq!(transport_facing_index(0x1E), Some(2)); // foot S
        assert_eq!(transport_facing_index(0x1F), Some(3)); // foot W
        assert_eq!(transport_facing_index(0x22), Some(2)); // ship S
        assert_eq!(transport_facing_index(0x10), None);    // not a transport
    }

    #[test]
    fn magic_unlock_door_rewrite_only_accepts_wooden_doors() {
        // doors-and-z-transitions.md §7
        assert_eq!(
            magic_unlock_door_rewrite(MAGIC_UNLOCK_CLOSED_WOODEN_A),
            Some(MAGIC_UNLOCK_OPEN_WOODEN_A)
        );
        assert_eq!(
            magic_unlock_door_rewrite(MAGIC_UNLOCK_CLOSED_WOODEN_B),
            Some(MAGIC_UNLOCK_OPEN_WOODEN_B)
        );
        // Already-open variants are not re-rewritten.
        assert_eq!(magic_unlock_door_rewrite(MAGIC_UNLOCK_OPEN_WOODEN_A), None);
        assert_eq!(magic_unlock_door_rewrite(MAGIC_UNLOCK_OPEN_WOODEN_B), None);
        // Magic-locked / regular-locked variants (one byte below 0x97):
        assert_eq!(magic_unlock_door_rewrite(0x95), None);
        assert_eq!(magic_unlock_door_rewrite(0x96), None);
        // Sentinels and non-door tiles.
        assert_eq!(magic_unlock_door_rewrite(0x00), None);
        assert_eq!(magic_unlock_door_rewrite(0xFF), None);
    }

    #[test]
    fn text_control_byte_classifies_extended_set() {
        // text-output.md §3
        assert_eq!(TEXT_WINDOW_COUNT, 4);
        assert_eq!(TEXT_SCREEN_COLUMNS, 40);
        assert_eq!(TEXT_SCREEN_ROWS, 25);
        assert_eq!(text_control_byte(0xFB), Some(TextControlByte::CentreOff));
        assert_eq!(text_control_byte(0xFC), Some(TextControlByte::CentreOn));
        assert_eq!(
            text_control_byte(0xFD),
            Some(TextControlByte::InverseToggle)
        );
        assert_eq!(
            text_control_byte(0xFE),
            Some(TextControlByte::UnderlineToggle)
        );
        assert_eq!(
            text_control_byte(0xFF),
            Some(TextControlByte::ClearWindow)
        );
        // Surrounding/non-control bytes return None.
        assert_eq!(text_control_byte(0x00), None);
        assert_eq!(text_control_byte(0x7F), None);
        assert_eq!(text_control_byte(0x80), None);
        assert_eq!(text_control_byte(0xFA), None);
    }

    #[test]
    fn resurrection_xp_penalty_matches_spec_table() {
        // karma.md §5
        assert_eq!(RESURRECTION_PENALTY_SKIP_THRESHOLD, 98);
        // Standing >= 98 skips the penalty.
        assert!(resurrection_penalty_skipped(98));
        assert!(resurrection_penalty_skipped(99));
        assert!(!resurrection_penalty_skipped(97));
        assert!(!resurrection_penalty_skipped(0));
        // XP unchanged at threshold.
        assert_eq!(resurrection_scaled_xp(98, 1000), 1000);
        assert_eq!(resurrection_scaled_xp(99, 65535), 65535);
        // Standing 50 -> half XP.
        assert_eq!(resurrection_scaled_xp(50, 1000), 500);
        // Standing 0 -> XP becomes 0.
        assert_eq!(resurrection_scaled_xp(0, 9999), 0);
        // No overflow at large XP (60000 * 50 / 100 = 30000).
        assert_eq!(resurrection_scaled_xp(50, 60000), 30000);
    }

    #[test]
    fn stats_panel_middle_counter_picks_ship_hull_for_ship_marker() {
        // stats-panel.md §5
        // Ordinary/non-ship markers -> party gold.
        assert_eq!(
            stats_panel_middle_counter(0x00),
            StatsPanelMiddleCounter::PartyGold
        );
        assert_eq!(
            stats_panel_middle_counter(0x1F),
            StatsPanelMiddleCounter::PartyGold
        );
        assert_eq!(
            stats_panel_middle_counter(0x28),
            StatsPanelMiddleCounter::PartyGold
        );
        assert_eq!(
            stats_panel_middle_counter(0xFF),
            StatsPanelMiddleCounter::PartyGold
        );
        // Ship family 0x20..=0x27 -> hull condition.
        for b in 0x20u8..=0x27 {
            assert_eq!(
                stats_panel_middle_counter(b),
                StatsPanelMiddleCounter::ShipHullCondition,
                "ship marker {b:#x} should select hull"
            );
        }
    }

    #[test]
    fn outdoor_arena_id_for_class_matches_spec_table() {
        // encounters.md §4
        assert_eq!(OUTDOOR_ARENA_COUNT, 16);
        // Linear formula across 0x40..=0x7F.
        for arena in 0u8..16 {
            let first = OUTDOOR_ARENA_CLASS_FIRST + arena * 4;
            for offset in 0u8..4 {
                let class_byte = first + offset;
                assert_eq!(
                    outdoor_arena_id_for_class(class_byte),
                    Some(arena),
                    "class {class_byte:#x} expected arena {arena}"
                );
            }
        }
        // Skiff/pirate-ship special hard-codes arena 1.
        assert_eq!(
            outdoor_arena_id_for_class(OUTDOOR_ARENA_SKIFF_CLASS),
            Some(OUTDOOR_ARENA_SKIFF_INDEX)
        );
        // Out-of-window classes fall through to scripted handling.
        assert_eq!(outdoor_arena_id_for_class(0x00), None);
        assert_eq!(outdoor_arena_id_for_class(0x3F), None);
        assert_eq!(outdoor_arena_id_for_class(0x80), None);
        assert_eq!(outdoor_arena_id_for_class(0xFF), None);
    }

    #[test]
    fn random_encounter_probe_spawn_predicate_matches_spec() {
        // encounters.md §3
        assert_eq!(RANDOM_ENCOUNTER_ROLL_BOUND, 30);
        // Threshold 0 and 1 never spawn.
        for roll in 1u8..=30 {
            assert!(!random_encounter_probe_spawns(roll, 0));
            assert!(!random_encounter_probe_spawns(roll, 1));
        }
        assert_eq!(random_encounter_spawn_outcomes(0), 0);
        assert_eq!(random_encounter_spawn_outcomes(1), 0);
        // Threshold N (N >= 2) -> N-1 spawning rolls (rolls 1..N-1 spawn).
        for threshold in 2u8..=30 {
            for roll in 1u8..=30 {
                let expect = roll < threshold;
                assert_eq!(
                    random_encounter_probe_spawns(roll, threshold),
                    expect,
                    "threshold={threshold} roll={roll}",
                );
            }
            assert_eq!(
                random_encounter_spawn_outcomes(threshold),
                threshold - 1
            );
        }
    }

    #[test]
    fn light_counter_increment_per_cadence_matches_spec() {
        // lighting.md §5
        assert_eq!(
            light_counter_increment(LightDecayCadence::TownDungeonCombatTurn),
            1
        );
        assert_eq!(
            light_counter_increment(LightDecayCadence::OverworldTurn),
            2
        );
        assert_eq!(light_counter_increment(LightDecayCadence::Wait(60)), 60);
        assert_eq!(light_counter_increment(LightDecayCadence::Wait(0)), 0);
        assert_eq!(
            light_counter_increment(LightDecayCadence::ModeZeroRefresh),
            0
        );
        // Chained with decay_light_counter: town turn drains 1 from 5.
        assert_eq!(
            decay_light_counter(
                5,
                light_counter_increment(LightDecayCadence::TownDungeonCombatTurn)
            ),
            4
        );
        // Mode-zero refresh leaves counter unchanged.
        assert_eq!(
            decay_light_counter(
                5,
                light_counter_increment(LightDecayCadence::ModeZeroRefresh)
            ),
            5
        );
    }

    #[test]
    fn visibility_carve_neighbor_order_matches_spec_ring() {
        // visibility.md §5 — W, SW, S, SE, E, NE, N, NW.
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER.len(), 8);
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[0], (-1, 0));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[1], (-1, 1));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[2], (0, 1));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[3], (1, 1));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[4], (1, 0));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[5], (1, -1));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[6], (0, -1));
        assert_eq!(VISIBILITY_CARVE_NEIGHBOR_ORDER[7], (-1, -1));
        // Ring sums to (0, 0): each cardinal cancels its opposite.
        let sum: (i8, i8) = VISIBILITY_CARVE_NEIGHBOR_ORDER
            .iter()
            .fold((0i8, 0i8), |acc, (dx, dy)| (acc.0 + dx, acc.1 + dy));
        assert_eq!(sum, (0, 0));
    }

    #[test]
    fn visibility_in_radius_uses_squared_distance_threshold() {
        // visibility.md §5
        assert!(visibility_in_radius(0, 0));
        assert!(visibility_in_radius(0, 100));
        assert!(visibility_in_radius(100, 100));
        assert!(!visibility_in_radius(101, 100));
        assert!(visibility_in_radius(99, 100));
    }

    #[test]
    fn autonomous_wind_drift_gates_per_spec() {
        // weather.md §2
        // Outer roll: only 0 of 64 advances.
        assert!(WindState::autonomous_drift_outer_accepted(0));
        for r in 1u8..=63 {
            assert!(!WindState::autonomous_drift_outer_accepted(r));
        }
        // Cardinal candidates accept immediately.
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(1, 0),
            Some(WindState::North)
        );
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(2, 0),
            Some(WindState::South)
        );
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(3, 0),
            Some(WindState::East)
        );
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(4, 0),
            Some(WindState::West)
        );
        // Calm requires follow-up roll >= 192.
        assert_eq!(WindState::autonomous_drift_accept_candidate(0, 191), None);
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(0, 192),
            Some(WindState::Calm)
        );
        assert_eq!(
            WindState::autonomous_drift_accept_candidate(0, 255),
            Some(WindState::Calm)
        );
        // Out-of-range candidate -> repeat (None).
        assert_eq!(WindState::autonomous_drift_accept_candidate(5, 200), None);
    }

    #[test]
    fn lord_british_camp_event_helpers_match_spec() {
        // rest-and-camp.md §7
        assert_eq!(LORD_BRITISH_CAMP_EVENT_ROLL_BOUND, 100);
        assert_eq!(LORD_BRITISH_CAMP_EVENT_THRESHOLD, 25);
        assert!(lord_british_camp_event_triggered(0));
        assert!(lord_british_camp_event_triggered(24));
        assert!(!lord_british_camp_event_triggered(25));
        assert!(!lord_british_camp_event_triggered(99));

        // Level recomputation table from spec.
        assert_eq!(level_for_experience(0), 1);
        assert_eq!(level_for_experience(99), 1);
        assert_eq!(level_for_experience(100), 2);
        assert_eq!(level_for_experience(199), 2);
        assert_eq!(level_for_experience(200), 3);
        assert_eq!(level_for_experience(399), 3);
        assert_eq!(level_for_experience(400), 4);
        assert_eq!(level_for_experience(799), 4);
        assert_eq!(level_for_experience(800), 5);
        assert_eq!(level_for_experience(1599), 5);
        assert_eq!(level_for_experience(1600), 6);

        // HP refresh = 30 * level.
        assert_eq!(lord_british_camp_event_hp_for_level(1), 30);
        assert_eq!(lord_british_camp_event_hp_for_level(8), 240);

        // Stat-reward selector.
        assert_eq!(
            lord_british_camp_stat_reward(1),
            Some(LordBritishCampStatReward::Strength)
        );
        assert_eq!(
            lord_british_camp_stat_reward(2),
            Some(LordBritishCampStatReward::Dexterity)
        );
        assert_eq!(
            lord_british_camp_stat_reward(3),
            Some(LordBritishCampStatReward::Intelligence)
        );
        assert_eq!(lord_british_camp_stat_reward(0), None);
        assert_eq!(lord_british_camp_stat_reward(4), None);
        assert_eq!(LORD_BRITISH_CAMP_STAT_REWARD_CAP, 30);
    }

    #[test]
    fn rest_status_predicates_match_spec_tables() {
        // rest-and-camp.md §5
        // Rest-with-watch participation.
        assert!(rest_with_watch_participates(CharacterStatus::Good));
        assert!(rest_with_watch_participates(
            CharacterStatus::PoisonedOrRevived
        ));
        assert!(rest_with_watch_participates(CharacterStatus::Sleeping));
        assert!(!rest_with_watch_participates(CharacterStatus::Charmed));
        assert!(!rest_with_watch_participates(CharacterStatus::Dead));
        assert!(!rest_with_watch_participates(CharacterStatus::Ashes));

        // Town-hours temporary-sleep marking only Good members.
        assert!(town_rest_temp_sleep_marked(CharacterStatus::Good));
        assert!(!town_rest_temp_sleep_marked(
            CharacterStatus::PoisonedOrRevived
        ));
        assert!(!town_rest_temp_sleep_marked(CharacterStatus::Sleeping));
        assert!(!town_rest_temp_sleep_marked(CharacterStatus::Charmed));
        assert!(!town_rest_temp_sleep_marked(CharacterStatus::Dead));
        assert!(!town_rest_temp_sleep_marked(CharacterStatus::Ashes));

        // Cleanup restores Sleeping -> Good only.
        assert!(rest_cleanup_transitions_to_good(CharacterStatus::Sleeping));
        assert!(!rest_cleanup_transitions_to_good(CharacterStatus::Good));
        assert!(!rest_cleanup_transitions_to_good(
            CharacterStatus::PoisonedOrRevived
        ));
        assert!(!rest_cleanup_transitions_to_good(CharacterStatus::Charmed));
        assert!(!rest_cleanup_transitions_to_good(CharacterStatus::Dead));
        assert!(!rest_cleanup_transitions_to_good(CharacterStatus::Ashes));
    }

    #[test]
    fn trap_effect_distribution_predicates_match_spec_tables() {
        // traps.md §3
        // Revive helper families.
        assert!(!trap_effect_uses_revive_helper(TrapEffect::Acid));
        assert!(trap_effect_uses_revive_helper(TrapEffect::Poison));
        assert!(!trap_effect_uses_revive_helper(TrapEffect::Bomb));
        assert!(trap_effect_uses_revive_helper(TrapEffect::Gas));

        // Non-combat outcome counts (3/8, 2/8, 2/8, 1/8 -> sum 8).
        assert_eq!(trap_non_combat_outcomes(TrapEffect::Acid), 3);
        assert_eq!(trap_non_combat_outcomes(TrapEffect::Poison), 2);
        assert_eq!(trap_non_combat_outcomes(TrapEffect::Bomb), 2);
        assert_eq!(trap_non_combat_outcomes(TrapEffect::Gas), 1);
        let total = trap_non_combat_outcomes(TrapEffect::Acid)
            + trap_non_combat_outcomes(TrapEffect::Poison)
            + trap_non_combat_outcomes(TrapEffect::Bomb)
            + trap_non_combat_outcomes(TrapEffect::Gas);
        assert_eq!(total, 8);

        // Combat-class scenes only roll Acid/Poison.
        assert!(trap_effect_appears_in_combat(TrapEffect::Acid));
        assert!(trap_effect_appears_in_combat(TrapEffect::Poison));
        assert!(!trap_effect_appears_in_combat(TrapEffect::Bomb));
        assert!(!trap_effect_appears_in_combat(TrapEffect::Gas));
    }

    #[test]
    fn underworld_stack_alternates_armour_and_weapon() {
        // hidden-treasures.md §3 records 0..=11
        assert_eq!(HIDDEN_TREASURE_UNDERWORLD_STACK_LEN, 12);
        assert_eq!(HIDDEN_TREASURE_UNDERWORLD_STACK_FLOOR, 255);
        assert_eq!(HIDDEN_TREASURE_UNDERWORLD_STACK_X, 233);
        assert_eq!(HIDDEN_TREASURE_UNDERWORLD_STACK_Y, 233);
        for record in HIDDEN_TREASURE_UNDERWORLD_STACK_FIRST
            ..=HIDDEN_TREASURE_UNDERWORLD_STACK_LAST
        {
            let entry = underworld_stack_record(record).expect("in stack range");
            if record % 2 == 0 {
                assert_eq!(entry, (HiddenTreasurePickupClass::Armour, 15));
            } else {
                assert_eq!(entry, (HiddenTreasurePickupClass::Weapon, 41));
            }
        }
        // Outside the stack range -> None.
        assert_eq!(underworld_stack_record(12), None);
        assert_eq!(underworld_stack_record(112), None);
    }

    #[test]
    fn hidden_treasure_pickup_class_variants_are_distinct() {
        // hidden-treasures.md §3 — exhaustively cover the spec's
        // distinct pickup-class column values.
        let all = [
            HiddenTreasurePickupClass::Armour,
            HiddenTreasurePickupClass::Weapon,
            HiddenTreasurePickupClass::Scroll,
            HiddenTreasurePickupClass::RingOfKeys,
            HiddenTreasurePickupClass::Gem,
            HiddenTreasurePickupClass::Potion,
            HiddenTreasurePickupClass::Food,
            HiddenTreasurePickupClass::Torches,
            HiddenTreasurePickupClass::Ring,
            HiddenTreasurePickupClass::MoldyCorpse,
            HiddenTreasurePickupClass::RottingBody,
            HiddenTreasurePickupClass::SackOfGold,
            HiddenTreasurePickupClass::Amulet,
        ];
        assert_eq!(all.len(), 13);
        // Distinctness via pairwise comparison.
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn inventory_add_class_cap_matches_spec_families() {
        // containers.md §8
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Gold), Some(9999));
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Potion), Some(99));
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::ScrollOrPlans),
            Some(99)
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::Equipment),
            Some(99)
        );
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Key), Some(99));
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Torch), Some(99));
        // Uncapped quantity families.
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Gem), None);
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Food), None);
        // Flag-only / refusal families have no quantity counter.
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::SandalwoodBox),
            None
        );
        assert_eq!(inventory_add_class_cap(InventoryAddClass::Moonstone), None);
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::MagicCarpet),
            None
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::ShadowlordShard),
            None
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::CrownOfLordBritish),
            None
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::SceptreOfLordBritish),
            None
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::AmuletOfLordBritish),
            None
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::MustOpenFirst),
            None
        );
        assert_eq!(
            inventory_add_class_cap(InventoryAddClass::NothingToGet),
            None
        );
    }

    #[test]
    fn dungeon_chest_gold_upper_collapses_at_depth_zero() {
        // containers.md §6
        assert_eq!(dungeon_chest_gold_upper(0), 0);
        assert_eq!(dungeon_chest_gold_upper(1), 8);
        assert_eq!(dungeon_chest_gold_upper(7), 56);
        assert!(dungeon_chest_gold_is_zero_width(0));
        assert!(!dungeon_chest_gold_is_zero_width(1));
        assert!(!dungeon_chest_gold_is_zero_width(7));
    }

    #[test]
    fn search_trap_visibility_classifies_per_spec_table() {
        // containers.md §5
        assert_eq!(
            search_trap_visibility(true, 0, false),
            SearchTrapVisibility::NoTrap
        );
        assert_eq!(
            search_trap_visibility(true, 25, false),
            SearchTrapVisibility::NoTrap
        );
        assert_eq!(
            search_trap_visibility(true, 0, true),
            SearchTrapVisibility::SimpleTrap
        );
        assert_eq!(
            search_trap_visibility(true, 9, true),
            SearchTrapVisibility::SimpleTrap
        );
        assert_eq!(
            search_trap_visibility(true, 10, true),
            SearchTrapVisibility::GenericTrap
        );
        assert_eq!(
            search_trap_visibility(true, 20, true),
            SearchTrapVisibility::GenericTrap
        );
        assert_eq!(
            search_trap_visibility(true, 21, true),
            SearchTrapVisibility::ComplexTrap
        );
        assert_eq!(
            search_trap_visibility(false, 5, true),
            SearchTrapVisibility::NoTrap
        );
        assert_eq!(
            search_trap_visibility(false, 5, false),
            SearchTrapVisibility::GenericTrap
        );
    }

    #[test]
    fn search_trap_detection_threshold_matches_spec_formulas() {
        // containers.md §5
        // Not trappable: (30 - stat) / 2
        assert_eq!(search_trap_detection_threshold(false, 0, 10), 10);
        assert_eq!(search_trap_detection_threshold(false, 99, 10), 10);
        assert_eq!(search_trap_detection_threshold(false, 0, 30), 0);
        // Negative raw -> 0
        assert_eq!(search_trap_detection_threshold(false, 0, 100), 0);
        // Trappable: (difficulty - stat + 30) / 2
        assert_eq!(search_trap_detection_threshold(true, 10, 10), 15);
        assert_eq!(search_trap_detection_threshold(true, 30, 0), 30);
        assert_eq!(search_trap_detection_threshold(true, 0, 30), 0);
        // Below the floor -> 0
        assert_eq!(search_trap_detection_threshold(true, 0, 100), 0);
    }

    #[test]
    fn search_location_prefix_classifies_named_scenery() {
        // containers.md §5
        assert_eq!(search_location_prefix(0x2B), Some(SearchLocationPrefix::Stump));
        assert_eq!(search_location_prefix(0x4F), Some(SearchLocationPrefix::Wall));
        assert_eq!(search_location_prefix(0x5A), Some(SearchLocationPrefix::Shelf));
        assert_eq!(
            search_location_prefix(0x5C),
            Some(SearchLocationPrefix::Bookshelf)
        );
        assert_eq!(
            search_location_prefix(0x5D),
            Some(SearchLocationPrefix::Bookshelf)
        );
        assert_eq!(search_location_prefix(0xA1), Some(SearchLocationPrefix::Well));
        assert_eq!(search_location_prefix(0xA5), Some(SearchLocationPrefix::Desk));
        assert_eq!(search_location_prefix(0xA6), Some(SearchLocationPrefix::Barrel));
        assert_eq!(search_location_prefix(0xA8), Some(SearchLocationPrefix::Vanity));
        assert_eq!(
            search_location_prefix(0xAB),
            Some(SearchLocationPrefix::UnderBed)
        );
        assert_eq!(
            search_location_prefix(0xAC),
            Some(SearchLocationPrefix::UnderBed)
        );
        assert_eq!(search_location_prefix(0xAD), Some(SearchLocationPrefix::Dresser));
        assert_eq!(search_location_prefix(0xAF), Some(SearchLocationPrefix::Trunk));
        assert_eq!(search_location_prefix(0xB2), Some(SearchLocationPrefix::Brazier));
        assert_eq!(
            search_location_prefix(0xBC),
            Some(SearchLocationPrefix::Fireplace)
        );
        // Generic find prefix
        assert_eq!(search_location_prefix(0x00), None);
        assert_eq!(search_location_prefix(0x05), None);
        assert_eq!(search_location_prefix(0xFF), None);
        assert_eq!(search_location_prefix(0xDC), None);
    }

    #[test]
    fn npc_link_action_classifies_per_floor_transition() {
        // active-objects.md §6
        assert_eq!(NPC_RUNTIME_DESCRIPTOR_BYTES, 16);
        assert_eq!(npc_link_action(false, true), NpcLinkAction::Allocate);
        assert_eq!(npc_link_action(true, true), NpcLinkAction::UpdateCoordinates);
        assert_eq!(npc_link_action(true, false), NpcLinkAction::Free);
        assert_eq!(npc_link_action(false, false), NpcLinkAction::NoAction);
    }

    #[test]
    fn blackthorn_entry_families_are_distinct() {
        // blackthorn.md §2: two cinematic families.
        let a = BlackthornEntryFamily::AudienceCapture;
        let b = BlackthornEntryFamily::RescueRefuge;
        assert_ne!(a, b);
        assert_eq!(a, BlackthornEntryFamily::AudienceCapture);
        assert_eq!(b, BlackthornEntryFamily::RescueRefuge);
    }

    #[test]
    fn sky_strip_renders_only_for_surface_and_town_family() {
        // moons.md §3
        // Surface (scene 0) renders only on Britannia, not underworld.
        assert!(sky_strip_renders(0, false));
        assert!(!sky_strip_renders(0, true));
        // Town-family scenes render
        for scene in 1..=32u8 {
            assert!(sky_strip_renders(scene, false));
        }
        // Town-family scenes also do not render on the underworld
        // plane (they're never reachable there but the predicate
        // honors the override).
        assert!(!sky_strip_renders(13, true));
        // Dungeon-class scenes are suppressed.
        assert!(!sky_strip_renders(33, false));
        assert!(!sky_strip_renders(40, false));
        assert!(!sky_strip_renders(127, false));
        // Combat marker is suppressed.
        assert!(!sky_strip_renders(0xFF, false));
    }

    #[test]
    fn provision_decrement_hours_match_spec_table() {
        // time.md §5
        assert_eq!(PROVISION_DECREMENT_HOURS, [6, 12, 18]);
        for hour in 0..24u8 {
            let expected = matches!(hour, 6 | 12 | 18);
            assert_eq!(is_provision_decrement_hour(hour), expected);
        }
    }

    #[test]
    fn apply_timing_tag_increment_matches_spec_rules() {
        // time.md §4
        assert_eq!(TIMING_TAG_QUICKNESS, b'Q');
        assert_eq!(TIMING_TAG_NEGATE_TIME, b'T');
        // No tag: increment passes through
        assert_eq!(apply_timing_tag_increment(2, 0), Some(2));
        assert_eq!(apply_timing_tag_increment(20, b' '), Some(20));
        // Q tag: halve, with 1-minute floor for non-zero increments
        assert_eq!(apply_timing_tag_increment(2, b'Q'), Some(1));
        assert_eq!(apply_timing_tag_increment(4, b'Q'), Some(2));
        // Q tag: increment 1 halves to 0 but is floored to 1
        assert_eq!(apply_timing_tag_increment(1, b'Q'), Some(1));
        // Q tag: a zero-increment input stays zero (mode-zero call)
        assert_eq!(apply_timing_tag_increment(0, b'Q'), Some(0));
        // T tag: suppress entirely
        assert_eq!(apply_timing_tag_increment(2, b'T'), None);
        assert_eq!(apply_timing_tag_increment(20, b'T'), None);
    }

    #[test]
    fn talk_liveness_gate_refusals_match_spec_priorities() {
        // conversation.md §2
        // Gate accepts when no refusal applies.
        assert_eq!(talk_liveness_refusal(false, false, false, false), None);
        // Single-condition refusals
        assert_eq!(
            talk_liveness_refusal(true, false, false, false),
            Some(TalkRefusal::InCombat)
        );
        assert_eq!(
            talk_liveness_refusal(false, true, false, false),
            Some(TalkRefusal::Asleep)
        );
        assert_eq!(
            talk_liveness_refusal(false, false, true, false),
            Some(TalkRefusal::Starving)
        );
        assert_eq!(
            talk_liveness_refusal(false, false, false, true),
            Some(TalkRefusal::AlreadyInConversation)
        );
        // Combat takes priority over the others.
        assert_eq!(
            talk_liveness_refusal(true, true, true, true),
            Some(TalkRefusal::InCombat)
        );
        // Asleep beats Starving + AlreadyInConversation.
        assert_eq!(
            talk_liveness_refusal(false, true, true, true),
            Some(TalkRefusal::Asleep)
        );
        // Starving beats AlreadyInConversation.
        assert_eq!(
            talk_liveness_refusal(false, false, true, true),
            Some(TalkRefusal::Starving)
        );
    }

    #[test]
    fn tlk_per_class_npc_counts_match_shipped_data() {
        // conversation.md §3
        assert_eq!(TOWNE_TLK_NPCS, 48);
        assert_eq!(DWELLING_TLK_NPCS, 15);
        assert_eq!(CASTLE_TLK_NPCS, 40);
        assert_eq!(KEEP_TLK_NPCS, 32);
        // Largest class header (TOWNE 48 NPCs * 4 bytes/entry) fits
        // inside the engine's 512-byte fixed header read window.
        assert!(TOWNE_TLK_NPCS * TLK_HEADER_ENTRY_LEN <= TLK_HEADER_FIXED_READ);
    }

    #[test]
    fn common_word_dictionary_index_helpers_match_spec() {
        // conversation.md §8
        assert_eq!(COMMON_WORD_DICTIONARY_ENTRIES, 128);
        // TLK dictionary tokens are 0x01..=0x7F; NUL and high-bit bytes are not tokens.
        assert_eq!(tlk_dictionary_index(0x00), None);
        assert_eq!(tlk_dictionary_index(0x01), Some(1));
        assert_eq!(tlk_dictionary_index(0x7F), Some(127));
        assert_eq!(tlk_dictionary_index(0x80), None);
        assert_eq!(tlk_dictionary_index(0xFF), None);
        // Shoppe phrase tokens use 0x80..=0xFF; shop token 0x80 maps
        // to the same logical entry zero.
        assert_eq!(shoppe_dictionary_index(0x00), None);
        assert_eq!(shoppe_dictionary_index(0x7F), None);
        assert_eq!(shoppe_dictionary_index(0x80), Some(0));
        assert_eq!(shoppe_dictionary_index(0xFF), Some(127));
    }

    #[test]
    fn quest_graph_node_classes_match_spec_table() {
        // catalogs/quest-graph.md §1
        assert_eq!(QUEST_GRAPH_NODE_CLASSES.len(), 8);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[0], QuestGraphNodeClass::Npc);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[1], QuestGraphNodeClass::Keyword);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[2], QuestGraphNodeClass::Knowledge);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[3], QuestGraphNodeClass::Password);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[4], QuestGraphNodeClass::Item);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[5], QuestGraphNodeClass::Place);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[6], QuestGraphNodeClass::Gate);
        assert_eq!(QUEST_GRAPH_NODE_CLASSES[7], QuestGraphNodeClass::Action);
    }

    #[test]
    fn title_screen_layout_constants_match_spec() {
        // intro.md §3
        assert_eq!(TITLE_BIT_INITIAL_PLACEMENTS.len(), 7);
        assert_eq!(
            TITLE_BIT_INITIAL_PLACEMENTS[0],
            TitleBitPlacement {
                asset: TitleBitAsset::Title,
                slot: 0,
                top_left_x: 148,
                top_left_y: 0,
                width: 24,
                height: 3
            }
        );
        assert_eq!(TITLE_BIT_INITIAL_PLACEMENTS[6].slot, 6);
        assert_eq!(TITLE_BIT_INITIAL_PLACEMENTS[6].width, 280);
        assert_eq!(TITLE_BIT_INITIAL_PLACEMENTS[6].height, 61);
        // Each subsequent slot starts at the previous slot's bottom edge.
        for win in TITLE_BIT_INITIAL_PLACEMENTS.windows(2) {
            assert_eq!(
                win[1].top_left_y,
                win[0].top_left_y + win[0].height
            );
        }
        // BRITISH.PTH has 4 pen origins
        assert_eq!(BRITISH_PTH_PEN_ORIGINS.len(), 4);
        assert_eq!(BRITISH_PTH_PEN_ORIGINS[0], (68, 44));
        assert_eq!(BRITISH_PTH_PEN_ORIGINS[3], (105, 167));
        assert_eq!(TITLE_LOWER_BAND_CLEAR_Y, 140);
        assert_eq!(BRITISH_PTH_PEN_ORIGINS.len(), BRITISH_PTH_SEGMENT_COUNT);
    }

    #[test]
    fn return_to_view_constants_match_spec() {
        // intro.md §12
        assert_eq!(MISCMAPS_DAT_FILE, "MISCMAPS.DAT");
        assert_eq!(RTV_STRIP_COUNT, 4);
        assert_eq!(RTV_STRIP_ROWS, 19);
        assert_eq!(RTV_STRIP_COLUMNS, 4);
        assert_eq!(RTV_COMMAND_STREAM_BYTES, 655);
        assert_eq!(RTV_COMMAND_COUNT, 16);
    }

    #[test]
    fn blackthorn_challenge_answer_matcher_substring_case_insensitive() {
        // blackthorn.md §4
        assert_eq!(BLACKTHORN_CHALLENGE_INPUT_LIMIT, 14);
        assert_eq!(BLACKTHORN_CHALLENGE_PROMPT_COUNT, 4);
        // Exact match
        assert!(blackthorn_challenge_answer_matches("Ahm", "Ahm"));
        // Case insensitive
        assert!(blackthorn_challenge_answer_matches("ahm", "Ahm"));
        assert!(blackthorn_challenge_answer_matches("AHM", "ahm"));
        // Substring match (anywhere in buffer)
        assert!(blackthorn_challenge_answer_matches("the answer is Ahm", "Ahm"));
        assert!(blackthorn_challenge_answer_matches("Ahmic", "Ahm"));
        assert!(blackthorn_challenge_answer_matches("xMux", "Mu"));
        // Negative
        assert!(!blackthorn_challenge_answer_matches("Beh", "Ahm"));
        assert!(!blackthorn_challenge_answer_matches("", "Ahm"));
    }

    #[test]
    fn eternal_flame_pairs_with_each_shadowlord_slot() {
        // catalogs/quest-graph.md §5
        assert_eq!(eternal_flame_for_shadowlord(0), Some(EternalFlame::Truth));
        assert_eq!(eternal_flame_for_shadowlord(1), Some(EternalFlame::Love));
        assert_eq!(
            eternal_flame_for_shadowlord(2),
            Some(EternalFlame::Courage)
        );
        assert_eq!(eternal_flame_for_shadowlord(3), None);
        assert_eq!(eternal_flame_for_shadowlord(255), None);
    }

    #[test]
    fn main_quest_requirements_enumerate_per_spec() {
        // catalogs/quest-graph.md §2
        assert_eq!(MainQuestRequirement::ALL.len(), 4);
        assert_eq!(MainQuestRequirement::ALL[0], MainQuestRequirement::RoyalArtifacts);
        assert_eq!(MainQuestRequirement::ALL[1], MainQuestRequirement::DungeonWords);
        assert_eq!(
            MainQuestRequirement::ALL[2],
            MainQuestRequirement::ShardsAndShadowlords
        );
        assert_eq!(
            MainQuestRequirement::ALL[3],
            MainQuestRequirement::SandalwoodBox
        );
    }

    #[test]
    fn sandalwood_box_pickup_constants_match_spec() {
        // catalogs/quest-graph.md §7
        assert_eq!(SANDALWOOD_BOX_PICKUP_SCENE, 17); // CASTLE:0 = Lord British's Castle
        assert_eq!(SANDALWOOD_BOX_PICKUP_X, 18);
        assert_eq!(SANDALWOOD_BOX_PICKUP_Y, 12);
        assert_eq!(SANDALWOOD_BOX_PICKUP_Z, 2);
        assert_eq!(SANDALWOOD_BOX_PICKUP_OBJECT_SLOT, 31);
        assert_eq!(SANDALWOOD_BOX_PICKUP_TAG, 0x0E);
        // Cross-check: the pickup tag matches the InventoryAddClass
        // SandalwoodBox dispatcher entry (containers.md §8).
        assert_eq!(
            inventory_add_class(SANDALWOOD_BOX_PICKUP_TAG),
            InventoryAddClass::SandalwoodBox
        );
        // Scene is the published Lord British's Castle slot.
        assert_eq!(
            town_resident_name(SANDALWOOD_BOX_PICKUP_SCENE),
            Some("Lord British's Castle")
        );
    }

    #[test]
    fn conversation_password_classifies_dawn_and_impera() {
        // catalogs/quest-graph.md §3
        assert_eq!(conversation_password("DAWN"), Some(ConversationPassword::Dawn));
        assert_eq!(conversation_password("dawn"), Some(ConversationPassword::Dawn));
        assert_eq!(
            conversation_password("IMPERA"),
            Some(ConversationPassword::Impera)
        );
        assert_eq!(
            conversation_password("Impera"),
            Some(ConversationPassword::Impera)
        );
        assert_eq!(conversation_password(""), None);
        assert_eq!(conversation_password("DAWN1"), None);
        assert_eq!(conversation_password("BLACKTHORN"), None);
    }

    #[test]
    fn npc_dialog_id_classifier_matches_spec_table() {
        // catalogs/npc-roster.md §4
        assert_eq!(npc_dialog_id_kind(0), NpcDialogIdKind::NoDialogue);
        assert_eq!(
            npc_dialog_id_kind(1),
            NpcDialogIdKind::TlkHeaderSentinel
        );
        assert_eq!(npc_dialog_id_kind(2), NpcDialogIdKind::OrdinaryBlobId);
        assert_eq!(npc_dialog_id_kind(50), NpcDialogIdKind::OrdinaryBlobId);
        assert_eq!(npc_dialog_id_kind(128), NpcDialogIdKind::OrdinaryBlobId);
        for byte in 129..=136u8 {
            assert_eq!(npc_dialog_id_kind(byte), NpcDialogIdKind::HighSpecial);
        }
        assert_eq!(npc_dialog_id_kind(137), NpcDialogIdKind::OrdinaryBlobId);
        assert_eq!(npc_dialog_id_kind(255), NpcDialogIdKind::HighSpecial);
    }

    #[test]
    fn ring_vanish_and_regen_predicates_match_spec() {
        // catalogs/item-list.md §5.4
        assert_eq!(RING_VANISH_DENOMINATOR, 16);
        assert!(ring_immediately_vanishes(0));
        assert!(ring_immediately_vanishes(16));
        assert!(!ring_immediately_vanishes(1));
        assert!(!ring_immediately_vanishes(15));
        assert_eq!(RING_REGEN_DENOMINATOR, 8);
        assert!(ring_regenerates(0));
        assert!(ring_regenerates(8));
        assert!(!ring_regenerates(1));
        assert!(!ring_regenerates(7));
        // 1-in-16 over uniform 0..16 = exactly one of every 16 rolls
        let vanish_count = (0..16u8).filter(|&r| ring_immediately_vanishes(r)).count();
        assert_eq!(vanish_count, 1);
        // 1-in-8 over uniform 0..8 = exactly one of every 8 rolls
        let regen_count = (0..8u8).filter(|&r| ring_regenerates(r)).count();
        assert_eq!(regen_count, 1);
    }

    #[test]
    fn hcs_font_layout_constants_match_spec() {
        // formats/font-hcs.md §2,§3
        assert_eq!(HCS_FONT_LEN, 3072);
        assert_eq!(HCS_GLYPH_COUNT, 128);
        assert_eq!(HCS_GLYPH_BYTES, 24);
        assert_eq!(HCS_CELL_WIDTH, 16);
        assert_eq!(HCS_CELL_HEIGHT, 12);
        assert_eq!(HCS_BYTES_PER_ROW, 2);
        assert_eq!(HCS_GLYPH_COUNT * HCS_GLYPH_BYTES, HCS_FONT_LEN);
        assert_eq!(HCS_CELL_HEIGHT * HCS_BYTES_PER_ROW, HCS_GLYPH_BYTES);
        assert_eq!(HCS_CELL_WIDTH / 8, HCS_BYTES_PER_ROW);
    }

    #[test]
    fn ch_font_layout_constants_match_spec() {
        // formats/font-ch.md §2,§3
        assert_eq!(CH_FONT_LEN, 1024);
        assert_eq!(CH_GLYPH_COUNT, 128);
        assert_eq!(CH_GLYPH_BYTES, 8);
        assert_eq!(CH_CELL_SIDE, 8);
        assert_eq!(CH_GLYPH_COUNT * CH_GLYPH_BYTES, CH_FONT_LEN);
    }

    #[test]
    fn bit_format_constants_match_spec() {
        // formats/bit.md §3,§4.3
        assert_eq!(BIT_POINTER_TABLE_ENTRY_LEN, 4);
        assert_eq!(BIT_ENTRY_COUNT_WORD_LEN, 2);
        assert_eq!(BIT_STRIP_POINTER_NONE, 0);
        assert_eq!(WD_BIT_LETTERING_ROWS, 49);
    }

    #[test]
    fn tile_atlas_size_constants_match_spec() {
        // formats/tiles.md §3,§4,§5.1
        assert_eq!(TILE_PIXEL_SIDE, 16);
        assert_eq!(FLAT_TILE_ATLAS_TILES, 512);
        assert_eq!(EGA_TILE_BYTES, 128);
        assert_eq!(CGA_TILE_BYTES, 64);
        assert_eq!(EGA_FLAT_TILE_ATLAS_BYTES, 65_536);
        assert_eq!(CGA_FLAT_TILE_ATLAS_BYTES, 32_768);
        // 2:1 byte ratio between EGA and CGA encodings
        assert_eq!(EGA_TILE_BYTES, CGA_TILE_BYTES * 2);
        // Tile-byte arithmetic: 16x16 / 2px-per-byte = 128
        assert_eq!(TILE_PIXEL_SIDE * TILE_PIXEL_SIDE / 2, EGA_TILE_BYTES);
    }

    #[test]
    fn pth_decode_byte_matches_spec_encoding() {
        // formats/pth.md §3,§5
        assert_eq!(BRITISH_PTH_LEN, 2_783);
        assert_eq!(BRITISH_PTH_SEGMENT_COUNT, 4);
        // NUL is the segment terminator
        assert_eq!(pth_decode_byte(0), None);
        // Pen-down: both magnitudes <= 2
        let s = pth_decode_byte(0x12).unwrap(); // dx=1 (high), dy=2 (low), positive
        assert_eq!(s, PenStroke { dx: 1, dy: 2, pen_down: true });
        // Pen-up: high magnitude > 2
        let s = pth_decode_byte(0x31).unwrap();
        assert_eq!(s, PenStroke { dx: 3, dy: 1, pen_down: false });
        // Negative deltas (sign bits set)
        let s = pth_decode_byte(0x88).unwrap(); // dx=-0, dy=-0
        assert_eq!(s, PenStroke { dx: 0, dy: 0, pen_down: true });
        let s = pth_decode_byte(0x91).unwrap(); // dx=-1, dy=1, pen_down (mags <=2)
        assert_eq!(s, PenStroke { dx: -1, dy: 1, pen_down: true });
        // Largest magnitude per axis
        let s = pth_decode_byte(0x77).unwrap(); // dx=7, dy=7, pen-up
        assert_eq!(s, PenStroke { dx: 7, dy: 7, pen_down: false });
        let s = pth_decode_byte(0xFF).unwrap(); // dx=-7, dy=-7, pen-up
        assert_eq!(s, PenStroke { dx: -7, dy: -7, pen_down: false });
    }

    #[test]
    fn no_turn_dungeon_action_on_wind_tile_skips_underfoot_wind() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;
        state.visibility_dirty = false;

        assert!(state.handle_dungeon_key('l', &dir).unwrap());

        assert_eq!(state.turn, 0);
        assert_eq!(state.torch_counter, 5);
        assert!(!state.visibility_dirty);
        assert!(!state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_dungeon_wind_tile_extinguishes_underfoot_torch_after_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;
        state.light_spell_counter = 5;
        state.visibility_dirty = false;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Passed
        );

        assert_eq!(state.turn, 1);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 4);
        assert!(state.visibility_dirty);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_wind_tile_sidecar_extinguishes_torch_but_not_light_spell() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;
        state.light_spell_counter = 5;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 4);
        assert!(state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_wind_tile_cell_guard_mismatch_does_not_extinguish_torch() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_WIND_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x71\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.torch_counter = 5;

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.torch_counter, 4);
        assert!(!state.message.contains("breeze blows out the torch"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn consumed_dungeon_turn_on_teleport_sidecar_changes_level_after_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 1 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('a', &dir).unwrap());

        assert_eq!(state.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(state.active_objects[0].z, 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Turned to face"));
        assert!(state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_turn_dungeon_action_on_teleport_sidecar_skips_underfoot_teleport() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 1 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('l', &dir).unwrap());

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert!(!state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_dungeon_teleport_sidecar_changes_level_after_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 1 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 3 })
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(state.active_objects[0].z, 3);
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_scripted_teleport_sidecar_changes_level_and_position() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 2 1 3 4 5 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 3 })
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 3 });
        assert_eq!((state.player.x, state.player.y), (4, 5));
        assert_eq!(
            (
                state.active_objects[0].x,
                state.active_objects[0].y,
                state.active_objects[0].z,
            ),
            (4, 5, 3)
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_teleport_cell_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_TELEPORT_TABLE_FILE),
            "DUNGEON:0 0 2 1 3 4 5 0x71\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("scripted dungeon teleport"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_sidecar_returns_to_world_location_table() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(
            state.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(state.grid[world_cell_index(10, 20)], 5);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("dungeon exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pass_turn_on_dungeon_exit_tile_sidecar_returns_after_turn() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x70\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 10 20 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (10, 20));
        assert_eq!(
            state.active_objects[0].z,
            WorldPlane::Underworld.save_floor()
        );
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Passed."));
        assert!(state.message.contains("Triggered dungeon exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_missing_return_metadata_stays_in_dungeon() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x70\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.active_objects[0].z, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("dungeon exit tile"));
        assert!(
            state
                .message
                .contains("missing clean return-coordinate metadata")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_sidecar_overrides_blocking_cell() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(33).unwrap();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0xB0\n",
        )
        .unwrap();
        fs::write(
            dir.join(WORLD_LOCATION_TABLE_FILE),
            "UNDERWORLD 12 34 DUNGEON:0\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xb0;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Transition(AreaTransition::ExitedDungeon(scene))
        );

        assert_eq!(
            state.area,
            Area::World {
                plane: WorldPlane::Underworld
            }
        );
        assert_eq!((state.player.x, state.player.y), (12, 34));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("dungeon exit tile"));
        assert!(state.message.contains("world-location table point"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_exit_tile_cell_guard_mismatch_keeps_normal_movement() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_EXIT_TILE_TABLE_FILE),
            "DUNGEON:0 0 2 1 0x71\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x70;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(
            state.area,
            Area::Dungeon {
                scene: DungeonScene::new(33).unwrap(),
                level: 0,
            }
        );
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.turn, 1);
        assert!(!state.message.contains("dungeon exit tile"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_energy_field_marker_variants_keep_subtype_reaction() {
        assert_eq!(dungeon_field_effect(0x88), Some(DungeonFieldEffect::Sleep));
        assert_eq!(
            dungeon_field_effect(0x89),
            Some(DungeonFieldEffect::PoisonGas)
        );
        assert_eq!(dungeon_field_effect(0x8a), Some(DungeonFieldEffect::Fire));
        assert_eq!(
            dungeon_field_effect(0x8b),
            Some(DungeonFieldEffect::Electric)
        );
        assert_eq!(dungeon_field_effect(0x90), Some(DungeonFieldEffect::Energy));
        assert_eq!(dungeon_field_effect(0x70), None);
    }

    #[test]
    fn dungeon_room_trigger_marks_visit_local_helper_state_and_reports_arena() {
        let scene = DungeonScene::new(35).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.area = Area::Dungeon { scene, level: 0 };

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("slot 7"));
        assert!(state.message.contains("selected DUNGEON.CBT arena 23"));
        assert!(!state.message.contains("out of scope"));
    }

    #[test]
    fn dungeon_room_trigger_loads_selected_dungeon_cbt_record_when_available() {
        let dir = debug_game_dir();
        let scene = DungeonScene::new(35).unwrap();
        let record = synthetic_combat_arena_record();
        let mut dungeon_cbt = Vec::new();
        for _ in 0..DUNGEON_CBT_RECORDS {
            dungeon_cbt.extend_from_slice(&record);
        }
        fs::write(dir.join(DUNGEON_CBT_FILE), dungeon_cbt).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.area = Area::Dungeon { scene, level: 0 };

        assert_eq!(
            state
                .step_with_game_dir(Direction::East, Some(&dir))
                .unwrap(),
            MoveOutcome::Moved
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
        assert!(state.message.contains("loaded DUNGEON.CBT arena 23"));
        assert!(state.message.contains("terrain[0,0]=0x00"));
        assert!(state.message.contains("16 room source marker(s)"));
        assert!(state.message.contains("1 absorbable-field marker(s)"));
        assert!(state.message.contains("first source 0x30"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_current_room_trigger_fires_before_next_key() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xf3;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('w', Path::new("")).unwrap());

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa3);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("slot 3"));
    }

    #[test]
    fn doom_final_room_trigger_enters_endgame_without_room_rewrite() {
        let scene = DungeonScene::new(40).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(
            DOOM_FINAL_ROOM_LEVEL,
            DOOM_FINAL_ROOM_X,
            DOOM_FINAL_ROOM_Y,
        )] = 0xf0 | DOOM_FINAL_ROOM_SLOT;
        let mut state = dungeon_state(grid, DOOM_FINAL_ROOM_LEVEL, 4, DOOM_FINAL_ROOM_Y);
        state.area = Area::Dungeon {
            scene,
            level: DOOM_FINAL_ROOM_LEVEL,
        };

        assert_eq!(state.step(Direction::East), MoveOutcome::EndgameEntered);

        assert_eq!((state.player.x, state.player.y), (5, 7));
        assert_eq!(
            state.grid[dungeon_cell_index(
                DOOM_FINAL_ROOM_LEVEL,
                DOOM_FINAL_ROOM_X,
                DOOM_FINAL_ROOM_Y,
            )],
            0xf0 | DOOM_FINAL_ROOM_SLOT
        );
        assert_eq!(state.turn, 0);
        assert_eq!(
            state.endgame,
            Some(EndgameState::awaiting_first_confirmation())
        );
        assert!(state.message.contains("Lord British asks"));
    }

    #[test]
    fn endgame_confirmation_gates_victory_on_final_answer_and_box_flag() {
        let dir = debug_game_dir();
        let mut missing_box = dungeon_state(open_dungeon_record(), 0, 1, 1);
        missing_box.enter_endgame();

        assert_eq!(
            handle_play_key_input(&mut missing_box, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            missing_box.endgame,
            Some(EndgameState::awaiting_final_confirmation(true))
        );

        assert_eq!(
            handle_play_key_input(&mut missing_box, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            missing_box
                .endgame
                .as_ref()
                .and_then(|state| state.outcome),
            Some(EndgameOutcome::MissingBoxOrRefused)
        );
        assert_eq!(missing_box.turn, 0);

        let mut victory = dungeon_state(open_dungeon_record(), 0, 1, 1);
        victory.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 1;
        victory.party_names = vec![*b"MARIA\0\0\0\0"];
        victory.clock = GameClock::with_date(141, 5, 6, 12, 0).unwrap();
        victory.enter_endgame();
        victory.resolve_endgame_confirmation(false);
        victory.resolve_endgame_confirmation(true);

        let endgame = victory.endgame.as_ref().unwrap();
        assert_eq!(endgame.first_confirmation, Some(false));
        assert_eq!(endgame.final_confirmation, Some(true));
        assert_eq!(endgame.outcome, Some(EndgameOutcome::Victory));
        assert!(endgame.certificate.as_ref().unwrap().contains("MARIA"));
        assert!(victory.message.contains("sixth day of the fifth month"));
        assert!(victory.message.contains("one hundred forty-one"));
        assert!(victory.message.contains("2 years, 1 month, 1 day"));
        assert!(victory.message.contains("Report this completed quest to Origin"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn enter_endgame_restores_dead_party_for_tableau() {
        // endgame.md §10: dead party members are mutated into a present /
        // restored state for the ending tableau, with current health restored
        // from the stored maximum.
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 60,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 12,
                max_hp: 30,
                level: 4,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 25,
                level: 3,
            },
        ];

        state.enter_endgame();

        for member in &state.party {
            assert_eq!(member.status, b'G');
            assert_eq!(member.hp, member.max_hp);
        }
        assert!(state.endgame.is_some());
    }

    #[test]
    fn moonstone_burial_tile_accepted_matches_spec_set() {
        // formats/saved-gam.md §7.2
        assert_eq!(MOONSTONE_GATE_INVALID_SCENE, 0xFF);
        // Accepted: 4..=10, 44, 45
        for tile in 4..=10u8 {
            assert!(moonstone_burial_tile_accepted(tile));
        }
        assert!(moonstone_burial_tile_accepted(44));
        assert!(moonstone_burial_tile_accepted(45));
        // Rejected: outside the published set
        assert!(!moonstone_burial_tile_accepted(0));
        assert!(!moonstone_burial_tile_accepted(3));
        assert!(!moonstone_burial_tile_accepted(11));
        assert!(!moonstone_burial_tile_accepted(43));
        assert!(!moonstone_burial_tile_accepted(46));
        assert!(!moonstone_burial_tile_accepted(255));
    }

    #[test]
    fn surface_chasm_location_matches_gazetteer() {
        // catalogs/gazetteer.md §8
        assert_eq!(SURFACE_CHASM_X, 54);
        assert_eq!(SURFACE_CHASM_Y, 138);
        assert!(is_surface_chasm_cell(54, 138));
        assert!(!is_surface_chasm_cell(54, 137));
        assert!(!is_surface_chasm_cell(55, 138));
        assert!(!is_surface_chasm_cell(0, 0));
    }

    #[test]
    fn natural_moongate_counter_step_matches_spec_hour_band() {
        // overworld.md §9: 20..=23 and 0..=4 increase; 5..=19 decrease.
        for h in 20..=23u8 {
            assert_eq!(
                natural_moongate_counter_step(h),
                NaturalMoongateCounterStep::Increase
            );
        }
        for h in 0..=4u8 {
            assert_eq!(
                natural_moongate_counter_step(h),
                NaturalMoongateCounterStep::Increase
            );
        }
        for h in 5..=19u8 {
            assert_eq!(
                natural_moongate_counter_step(h),
                NaturalMoongateCounterStep::Decrease
            );
        }
        // Counter saturation
        assert_eq!(natural_moongate_advance_counter(0, 0), 1);
        assert_eq!(
            natural_moongate_advance_counter(NATURAL_MOONGATE_COUNTER_MAX, 0),
            NATURAL_MOONGATE_COUNTER_MAX
        );
        assert_eq!(natural_moongate_advance_counter(5, 12), 4);
        assert_eq!(natural_moongate_advance_counter(0, 12), 0);
        // Slot eligibility — interior (no chunk window)
        assert!(natural_moongate_slot_eligible(13, 0, 5, 5, 13, 0, None));
        assert!(!natural_moongate_slot_eligible(13, 0, 5, 5, 14, 0, None));
        assert!(!natural_moongate_slot_eligible(13, 0, 5, 5, 13, 1, None));
        // Surface chunk-window
        assert!(natural_moongate_slot_eligible(
            0,
            0,
            10,
            10,
            0,
            0,
            Some((0, 0, 32, 32))
        ));
        assert!(!natural_moongate_slot_eligible(
            0,
            0,
            40,
            10,
            0,
            0,
            Some((0, 0, 32, 32))
        ));
        // Live-gate entry hook outcome
        assert!(natural_moongate_dispatches_meditate(0, 0));
        assert!(natural_moongate_dispatches_meditate(0, 9));
        assert!(!natural_moongate_dispatches_meditate(0, 10));
        assert!(!natural_moongate_dispatches_meditate(1, 0));
        // Cached-glyph slot (before noon = 0, noon onward = 1)
        for h in 0..=11u8 {
            assert_eq!(natural_moongate_cached_glyph_slot(h), 0);
        }
        for h in 12..=23u8 {
            assert_eq!(natural_moongate_cached_glyph_slot(h), 1);
        }
        assert_eq!(NARRATIVE_GATE_X, 233);
        assert_eq!(NARRATIVE_GATE_Y, 235);
    }

    #[test]
    fn location_dat_layout_constants_and_filenames_per_spec() {
        // formats/location-dat.md §3
        assert_eq!(LOCATION_DAT_FILE_LEN, 16_384);
        assert_eq!(LOCATION_DAT_BLOCK_LEN, 2_048);
        assert_eq!(LOCATION_DAT_BLOCKS_PER_FILE, 8);
        assert_eq!(LOCATION_DAT_FLOOR_PAGE_LEN, 1_024);
        assert_eq!(LOCATION_DAT_FLOOR_PAGES_PER_BLOCK, 2);
        assert_eq!(
            LOCATION_DAT_BLOCK_LEN * LOCATION_DAT_BLOCKS_PER_FILE,
            LOCATION_DAT_FILE_LEN
        );
        assert_eq!(
            LOCATION_DAT_FLOOR_PAGE_LEN * LOCATION_DAT_FLOOR_PAGES_PER_BLOCK,
            LOCATION_DAT_BLOCK_LEN
        );
        // formats/location-dat.md §2 file family
        for s in 1..=8u8 {
            assert_eq!(location_dat_filename(s), Some("TOWNE.DAT"));
        }
        for s in 9..=16u8 {
            assert_eq!(location_dat_filename(s), Some("DWELLING.DAT"));
        }
        for s in 17..=24u8 {
            assert_eq!(location_dat_filename(s), Some("CASTLE.DAT"));
        }
        for s in 25..=32u8 {
            assert_eq!(location_dat_filename(s), Some("KEEP.DAT"));
        }
        assert_eq!(location_dat_filename(0), None);
        assert_eq!(location_dat_filename(33), None);
    }

    #[test]
    fn npc_roster_and_tlk_filenames_per_spec() {
        // formats/npc.md §2
        assert_eq!(npc_roster_filename(0), None);
        for s in 1..=8u8 {
            assert_eq!(npc_roster_filename(s), Some("TOWNE.NPC"));
            assert_eq!(npc_tlk_filename(s), Some("TOWNE.TLK"));
        }
        for s in 9..=16u8 {
            assert_eq!(npc_roster_filename(s), Some("DWELLING.NPC"));
            assert_eq!(npc_tlk_filename(s), Some("DWELLING.TLK"));
        }
        for s in 17..=24u8 {
            assert_eq!(npc_roster_filename(s), Some("CASTLE.NPC"));
            assert_eq!(npc_tlk_filename(s), Some("CASTLE.TLK"));
        }
        for s in 25..=32u8 {
            assert_eq!(npc_roster_filename(s), Some("KEEP.NPC"));
            assert_eq!(npc_tlk_filename(s), Some("KEEP.TLK"));
        }
        assert_eq!(npc_roster_filename(33), None);
        assert_eq!(npc_tlk_filename(33), None);
        assert_eq!(npc_roster_filename(255), None);
    }

    #[test]
    fn npc_file_layout_constants_match_spec() {
        // formats/npc.md §3,§4
        assert_eq!(NPC_FILE_LEN, 4608);
        assert_eq!(NPC_SUB_MAP_LEN, 576);
        assert_eq!(NPC_SUB_MAPS_PER_FILE, 8);
        assert_eq!(NPC_SUB_MAP_LEN * NPC_SUB_MAPS_PER_FILE, NPC_FILE_LEN);
        // Sub-map sub-blocks
        assert_eq!(NPC_SCHEDULE_ARRAY_LEN, 512);
        assert_eq!(NPC_TYPE_ARRAY_OFFSET, 512);
        assert_eq!(NPC_TYPE_ARRAY_LEN, 32);
        assert_eq!(NPC_DIALOG_ARRAY_OFFSET, 544);
        assert_eq!(NPC_DIALOG_ARRAY_LEN, 32);
        assert_eq!(
            NPC_SCHEDULE_ARRAY_LEN + NPC_TYPE_ARRAY_LEN + NPC_DIALOG_ARRAY_LEN,
            NPC_SUB_MAP_LEN
        );
        // 32 schedule slots × 16 bytes each = 512
        assert_eq!(NPC_SLOTS_PER_SUB_MAP, 32);
        assert_eq!(NPC_SCHEDULE_RECORD_LEN, 16);
        assert_eq!(
            NPC_SLOTS_PER_SUB_MAP * NPC_SCHEDULE_RECORD_LEN,
            NPC_SCHEDULE_ARRAY_LEN
        );
        // Sentinel slot
        assert_eq!(NPC_SENTINEL_SLOT, 0);
        assert_eq!(NPC_EFFECTIVE_SLOTS_PER_SUB_MAP, 31);
    }

    #[test]
    fn town_resident_name_matches_gazetteer_table() {
        // catalogs/gazetteer.md §5
        // Towns
        assert_eq!(town_resident_name(0), None);
        assert_eq!(town_resident_name(1), Some("Moonglow"));
        assert_eq!(town_resident_name(2), Some("Britain"));
        assert_eq!(town_resident_name(3), Some("Jhelom"));
        assert_eq!(town_resident_name(4), Some("Yew"));
        assert_eq!(town_resident_name(5), Some("Minoc"));
        assert_eq!(town_resident_name(6), Some("Trinsic"));
        assert_eq!(town_resident_name(7), Some("Skara Brae"));
        assert_eq!(town_resident_name(8), Some("New Magincia"));
        // Dwellings (5 named, 3 blank)
        assert_eq!(town_resident_name(9), Some("Fogsbane"));
        assert_eq!(town_resident_name(10), Some("Stormcrow"));
        assert_eq!(town_resident_name(11), Some("Greyhaven"));
        assert_eq!(town_resident_name(12), Some("Waveguide"));
        assert_eq!(town_resident_name(13), Some("Iolo's Hut"));
        assert_eq!(town_resident_name(14), None);
        assert_eq!(town_resident_name(15), None);
        assert_eq!(town_resident_name(16), None);
        // Castles
        assert_eq!(town_resident_name(17), Some("Lord British's Castle"));
        assert_eq!(town_resident_name(18), Some("Lord Blackthorn's Castle"));
        assert_eq!(town_resident_name(19), Some("West Britanny"));
        assert_eq!(town_resident_name(20), Some("North Britanny"));
        assert_eq!(town_resident_name(21), Some("East Britanny"));
        assert_eq!(town_resident_name(22), Some("Paws"));
        assert_eq!(town_resident_name(23), Some("Cove"));
        assert_eq!(town_resident_name(24), Some("Buccaneer's Den"));
        // Keeps
        assert_eq!(town_resident_name(25), Some("Ararat"));
        assert_eq!(town_resident_name(26), Some("Bordermarch"));
        assert_eq!(town_resident_name(27), Some("Farthing"));
        assert_eq!(town_resident_name(28), Some("Windemere"));
        assert_eq!(town_resident_name(29), Some("Stonegate"));
        assert_eq!(town_resident_name(30), Some("The Lycaeum"));
        assert_eq!(town_resident_name(31), Some("Empath Abbey"));
        assert_eq!(town_resident_name(32), Some("Serpent's Hold"));
        // Out of town-family range
        assert_eq!(town_resident_name(33), None);
        assert_eq!(town_resident_name(255), None);
    }

    #[test]
    fn town_location_class_and_index_split_per_spec() {
        // town-mode.md §2,§3,§4
        assert_eq!(town_location_class(0), None);
        for s in 1..=8u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Town));
            assert_eq!(town_per_class_index(s), Some(s - 1));
        }
        for s in 9..=16u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Dwelling));
            assert_eq!(town_per_class_index(s), Some(s - 9));
        }
        for s in 17..=24u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Castle));
            assert_eq!(town_per_class_index(s), Some(s - 17));
        }
        for s in 25..=32u8 {
            assert_eq!(town_location_class(s), Some(TownLocationClass::Keep));
            assert_eq!(town_per_class_index(s), Some(s - 25));
        }
        assert_eq!(town_location_class(33), None);
        assert_eq!(town_per_class_index(33), None);
        // Family names
        assert_eq!(TownLocationClass::Town.family_name(), "town");
        assert_eq!(TownLocationClass::Castle.family_name(), "castle");
        // Floor byte signed-eight-bit interpretation
        assert_eq!(town_floor_offset(0), 0);
        assert_eq!(town_floor_offset(1), 1);
        assert_eq!(town_floor_offset(127), 127);
        assert_eq!(town_floor_offset(128), -128);
        assert_eq!(town_floor_offset(255), -1); // basement (one floor below base)
        // Per-location grid + roster constants
        assert_eq!(TOWN_GRID_SIDE, 32);
        assert_eq!(TOWN_GRID_BYTES, 1024);
        assert_eq!(TOWN_NPC_ROSTER_SLOTS, 31);
        assert_eq!(TOWN_NPC_BLOCK_BYTES, 576);
    }

    #[test]
    fn hidden_treasure_rule_special_records_match_spec() {
        // hidden-treasures.md §2
        for r in [0usize, 1, 5, 12, 16, 17, 99, 112] {
            assert_eq!(hidden_treasure_rule(r), HiddenTreasureRule::OneShot);
        }
        assert_eq!(
            hidden_treasure_rule(13),
            HiddenTreasureRule::KeyAndNpcAbsence
        );
        assert_eq!(
            hidden_treasure_rule(14),
            HiddenTreasureRule::DailyCache
        );
        assert_eq!(
            hidden_treasure_rule(15),
            HiddenTreasureRule::SingleUseAndNpcAbsence
        );
        // Stage gates
        // Record 13: requires keys >= 1 and no NPC on the tile
        assert!(hidden_treasure_can_stage(13, 1, false, 0, 0, true));
        assert!(!hidden_treasure_can_stage(13, 0, false, 0, 0, true));
        assert!(!hidden_treasure_can_stage(13, 5, true, 0, 0, true));
        // Record 14: cookie != current day
        assert!(hidden_treasure_can_stage(14, 0, false, 5, 6, true));
        assert!(!hidden_treasure_can_stage(14, 0, false, 7, 7, true));
        // Record 15: flag clear AND no NPC
        assert!(hidden_treasure_can_stage(15, 0, false, 0, 0, true));
        assert!(!hidden_treasure_can_stage(15, 0, false, 0, 0, false));
        assert!(!hidden_treasure_can_stage(15, 0, true, 0, 0, true));
        // Ordinary one-shot record passes the per-record gate (caller
        // owns the found bitmap).
        assert!(hidden_treasure_can_stage(0, 0, true, 0, 0, false));
        assert!(hidden_treasure_can_stage(99, 0, true, 0, 0, false));
    }

    #[test]
    fn signs_dat_directory_constants_match_spec() {
        // formats/signs-dat.md §2,§3
        assert_eq!(SIGNS_DAT_SCENE_DIRECTORY_SLOTS, 33);
        assert_eq!(SIGNS_DAT_SCENE_DIRECTORY_BYTES, 66);
        assert_eq!(
            SIGNS_DAT_SCENE_DIRECTORY_BYTES,
            SIGNS_DAT_SCENE_DIRECTORY_SLOTS * 2
        );
        assert_eq!(SIGNS_DAT_RECORD_HEADER_LEN, 4);
    }

    #[test]
    fn dungeon_file_offset_matches_spec_layout() {
        // formats/dungeon-dat.md §2
        assert_eq!(DUNGEON_DAT_LEN, 4096);
        assert_eq!(DUNGEON_RECORD_LEN, 512);
        assert_eq!(DUNGEON_LEVEL_LEN, 64);
        assert_eq!(DUNGEON_SIDE, 8);
        // First cell of first dungeon record
        assert_eq!(dungeon_file_offset(0, 0, 0, 0), 0);
        // Last cell of first dungeon record
        assert_eq!(dungeon_file_offset(0, 7, 7, 7), DUNGEON_RECORD_LEN - 1);
        // First cell of second record
        assert_eq!(dungeon_file_offset(1, 0, 0, 0), DUNGEON_RECORD_LEN);
        // Doom record (index 7) first cell
        assert_eq!(dungeon_file_offset(7, 0, 0, 0), 7 * DUNGEON_RECORD_LEN);
        // Specific cell math
        assert_eq!(
            dungeon_file_offset(2, 3, 5, 4),
            2 * 512 + 3 * 64 + 4 * 8 + 5
        );
        // The eight dungeon records together fill DUNGEON_DAT
        assert_eq!(8 * DUNGEON_RECORD_LEN, DUNGEON_DAT_LEN);
    }

    #[test]
    fn under_file_offset_matches_spec_layout() {
        // formats/under-dat.md §2
        assert_eq!(UNDER_DAT_LEN, 65_536);
        assert_eq!(UNDER_DAT_LEN, WORLD_CELLS);
        // First and last cells
        assert_eq!(under_file_offset(0, 0), 0);
        assert_eq!(under_file_offset(255, 255), UNDER_DAT_LEN - 1);
        // Every logical chunk is stored, so chunk_slot == stored block index
        // and slot 1 starts at byte 256.
        assert_eq!(under_file_offset(16, 0), 256);
    }

    #[test]
    fn brit_chunk_slot_and_file_offset_match_spec() {
        // formats/brit-dat.md §3
        assert_eq!(WORLD_SIDE, 256);
        assert_eq!(WORLD_CHUNKS_PER_SIDE, 16);
        assert_eq!(WORLD_CHUNK_COUNT, 256);
        assert_eq!(BRIT_DAT_LEN, 52_480);
        assert_eq!(BRIT_STORED_CHUNKS, 205);
        // Chunk slot
        assert_eq!(brit_chunk_slot(0, 0), 0);
        assert_eq!(brit_chunk_slot(15, 15), 0);
        assert_eq!(brit_chunk_slot(16, 0), 1);
        assert_eq!(brit_chunk_slot(0, 16), 16);
        assert_eq!(brit_chunk_slot(255, 255), 16 * 16 - 1);
        // Offset in chunk (row-major)
        assert_eq!(brit_offset_in_chunk(0, 0), 0);
        assert_eq!(brit_offset_in_chunk(1, 0), 1);
        assert_eq!(brit_offset_in_chunk(0, 1), 16);
        assert_eq!(brit_offset_in_chunk(15, 15), 16 * 16 - 1);
        assert_eq!(brit_offset_in_chunk(16, 16), 0);
        // Water-sentinel returns None
        assert_eq!(brit_file_offset(BRIT_WATER_SENTINEL, 100, 100), None);
        // Stored block 0, cell (0, 0)
        assert_eq!(brit_file_offset(0, 0, 0), Some(0));
        // Stored block 5, cell (15, 15)
        assert_eq!(brit_file_offset(5, 15, 15), Some(5 * 256 + 255));
    }

    #[test]
    fn story_dat_size_and_record_count_match_spec() {
        // formats/story-dat.md §2
        assert_eq!(STORY_DAT_LEN, 11_679);
        assert_eq!(STORY_DAT_RECORDS, 20);
        // intro.md §10: total intro narrative steps include one inline
        // doorway step that does not consume a STORY.DAT record.
        assert_eq!(INTRO_STORY_STEP_COUNT, STORY_DAT_RECORDS + 1);
    }

    #[test]
    fn question_dat_layout_constants_match_spec() {
        // formats/question-dat.md §2,§4
        assert_eq!(QUESTION_DAT_RECORDS, 30);
        assert_eq!(QUESTION_DAT_FIRST_DILEMMA_RECORD, 2);
        assert_eq!(QUESTION_DAT_DILEMMA_COUNT, 28);
        assert_eq!(
            QUESTION_DAT_FIRST_DILEMMA_RECORD + QUESTION_DAT_DILEMMA_COUNT,
            QUESTION_DAT_RECORDS
        );
        // C(8, 2) = 28 dilemma pairs
        let mut pair_count = 0;
        for first in 0..ShrineVirtue::ALL.len() {
            for second in (first + 1)..ShrineVirtue::ALL.len() {
                let r = chargen_question_record_for_pair(
                    ShrineVirtue::ALL[first],
                    ShrineVirtue::ALL[second],
                )
                .expect("ordered pair always resolves");
                assert!(r >= QUESTION_DAT_FIRST_DILEMMA_RECORD);
                assert!(r < QUESTION_DAT_RECORDS);
                pair_count += 1;
            }
        }
        assert_eq!(pair_count, QUESTION_DAT_DILEMMA_COUNT);
    }

    #[test]
    fn miscmsg_family_matches_spec_clusters() {
        // formats/miscmsg-dat.md §2,§3
        assert_eq!(MISCMSG_DAT_LEN, 2_745);
        assert_eq!(MISCMSG_DAT_RECORDS, 47);
        for r in 0..=11 {
            assert_eq!(miscmsg_family(r), Some(MiscMsgFamily::BlackthornAudience));
        }
        for r in 12..=19 {
            assert_eq!(miscmsg_family(r), Some(MiscMsgFamily::VirtueWeaknessPhrases));
        }
        for r in 20..=27 {
            assert_eq!(miscmsg_family(r), Some(MiscMsgFamily::VirtueAphorisms));
        }
        for r in 28..=35 {
            assert_eq!(miscmsg_family(r), Some(MiscMsgFamily::ShrineMeditation));
        }
        for r in 36..=46 {
            assert_eq!(miscmsg_family(r), Some(MiscMsgFamily::UrnCodexProphecy));
        }
        assert_eq!(miscmsg_family(47), None);
        assert_eq!(miscmsg_family(255), None);
    }

    #[test]
    fn endmsg_dat_size_and_record_count_match_spec() {
        // formats/endmsg-dat.md §2
        assert_eq!(ENDMSG_DAT_LEN, 786);
        assert_eq!(ENDMSG_DAT_RECORDS, 11);
    }

    #[test]
    fn end_narrative_windows_match_spec_table() {
        // formats/end-dat.md §2,§4
        assert_eq!(END_DAT_LEN, 3_698);
        assert_eq!(END_DAT_WINDOW_COUNT, 6);
        for (n, expected) in [
            (1u8, EndNarrativeWindow::ReturnHomeOpening),
            (2, EndNarrativeWindow::Homecoming),
            (3, EndNarrativeWindow::RestlessNight),
            (4, EndNarrativeWindow::BlackthornJudgmentOpen),
            (5, EndNarrativeWindow::BlackthornSentence),
            (6, EndNarrativeWindow::OrbExileResolution),
        ] {
            assert_eq!(end_narrative_window(n), Some(expected));
            assert_eq!(expected.number(), n);
        }
        // Out-of-range
        assert_eq!(end_narrative_window(0), None);
        assert_eq!(end_narrative_window(7), None);
    }

    #[test]
    fn karma_dat_tier_classifies_six_records() {
        // formats/karma-dat.md §2,§3
        assert_eq!(KARMA_DAT_LEN, 761);
        assert_eq!(KARMA_DAT_RECORDS, 6);
        assert_eq!(karma_dat_tier(0), Some(KarmaDatTier::Lowest));
        assert_eq!(karma_dat_tier(1), Some(KarmaDatTier::Low));
        assert_eq!(karma_dat_tier(2), Some(KarmaDatTier::Middle));
        assert_eq!(karma_dat_tier(3), Some(KarmaDatTier::High));
        assert_eq!(karma_dat_tier(4), Some(KarmaDatTier::Highest));
        assert_eq!(karma_dat_tier(5), Some(KarmaDatTier::HighestCampVariant));
        assert_eq!(karma_dat_tier(6), None);
        assert_eq!(karma_dat_tier(255), None);
        // Cross-check: the Blackthorn rescue selector and the Lord
        // British camp selector reach the right tiers.
        assert_eq!(
            karma_dat_tier(blackthorn_rescue_verdict_record(99) as usize),
            Some(KarmaDatTier::Highest)
        );
        assert_eq!(
            karma_dat_tier(lord_british_camp_verdict_record(99) as usize),
            Some(KarmaDatTier::HighestCampVariant)
        );
    }

    #[test]
    fn lord_british_camp_verdict_bands_match_spec() {
        // formats/karma-dat.md §4 — Lord British-in-disguise camp event
        for s in 0..=19u8 {
            assert_eq!(lord_british_camp_verdict_record(s), 0);
        }
        for s in 20..=39u8 {
            assert_eq!(lord_british_camp_verdict_record(s), 1);
        }
        for s in 40..=59u8 {
            assert_eq!(lord_british_camp_verdict_record(s), 2);
        }
        for s in 60..=79u8 {
            assert_eq!(lord_british_camp_verdict_record(s), 3);
        }
        // Top band -> record 5 (record 4 is never selected by this event)
        for s in 80..=99u8 {
            assert_eq!(lord_british_camp_verdict_record(s), 5);
        }
        // Above-cap behaves like the top band.
        assert_eq!(lord_british_camp_verdict_record(255), 5);
        // Cross-check: the LB camp event never picks record 4.
        for s in 0..=99u8 {
            assert_ne!(lord_british_camp_verdict_record(s), 4);
        }
    }

    #[test]
    fn blackthorn_rescue_verdict_bands_match_spec() {
        // blackthorn.md §7
        assert_eq!(BLACKTHORN_RESCUE_HANDOFF_SCENE, 17);
        assert_eq!(BLACKTHORN_RESCUE_HANDOFF_X, 10);
        assert_eq!(BLACKTHORN_RESCUE_HANDOFF_Y, 10);
        assert_eq!(BLACKTHORN_RESCUE_STANDING_FLOOR, 75);
        // Twenty-point bands: 0..19, 20..39, 40..59, 60..79, 80..99
        for s in 0..=19u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 0);
        }
        for s in 20..=39u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 1);
        }
        for s in 40..=59u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 2);
        }
        for s in 60..=79u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 3);
        }
        for s in 80..=99u8 {
            assert_eq!(blackthorn_rescue_verdict_record(s), 4);
        }
        // Clamps to top band for values above the standing cap.
        assert_eq!(blackthorn_rescue_verdict_record(255), 4);
    }

    #[test]
    fn dungeon_resident_name_and_entry_seed_match_gazetteer() {
        // catalogs/gazetteer.md §6
        assert_eq!(dungeon_resident_name(33), Some("Deceit"));
        assert_eq!(dungeon_resident_name(34), Some("Despise"));
        assert_eq!(dungeon_resident_name(35), Some("Destard"));
        assert_eq!(dungeon_resident_name(36), Some("Wrong"));
        assert_eq!(dungeon_resident_name(37), Some("Covetous"));
        assert_eq!(dungeon_resident_name(38), Some("Shame"));
        assert_eq!(dungeon_resident_name(39), Some("Hythloth"));
        assert_eq!(dungeon_resident_name(40), Some("Doom"));
        assert_eq!(dungeon_resident_name(32), None);
        assert_eq!(dungeon_resident_name(41), None);
        assert_eq!(dungeon_resident_name(0), None);

        // Britannia surface entry: (Z=0, X=1, Y=1) facing east
        for scene in 33..=40u8 {
            assert_eq!(
                dungeon_entry_seed(scene, false),
                Some(DungeonEntrySeed {
                    z: 0,
                    x: 1,
                    y: 1,
                    facing: DUNGEON_FACING_EAST
                })
            );
        }
        // Underworld entry to non-Doom: (Z=7, X=7, Y=7) facing west
        for scene in 33..=39u8 {
            assert_eq!(
                dungeon_entry_seed(scene, true),
                Some(DungeonEntrySeed {
                    z: 7,
                    x: 7,
                    y: 7,
                    facing: DUNGEON_FACING_WEST
                })
            );
        }
        // Doom always uses surface seed
        assert_eq!(
            dungeon_entry_seed(40, true),
            Some(DungeonEntrySeed {
                z: 0,
                x: 1,
                y: 1,
                facing: DUNGEON_FACING_EAST
            })
        );
        // Non-dungeon scenes have no entry seed
        assert_eq!(dungeon_entry_seed(0, false), None);
        assert_eq!(dungeon_entry_seed(41, false), None);
    }

    #[test]
    fn scene_route_classifies_per_main_loop_table() {
        // main-loop.md §3,§4
        assert_eq!(scene_route(0), SceneRoute::Overworld);
        for v in 1..=32u8 {
            assert_eq!(scene_route(v), SceneRoute::TownFamily);
        }
        for v in [33u8, 40, 50, 100, 127] {
            assert_eq!(scene_route(v), SceneRoute::Dungeon);
        }
        for v in 0x40..=0x42u8 {
            assert_eq!(scene_route(v), SceneRoute::IntroOrPreview);
        }
        assert_eq!(scene_route(0xFF), SceneRoute::CombatTemporary);
        // Outside-the-stock-byte high range routes to combat (high
        // values are treated as combat-class by readers).
        assert_eq!(scene_route(0x80), SceneRoute::CombatTemporary);
        assert_eq!(scene_route(0xFE), SceneRoute::CombatTemporary);

        // Stock-named DUNGEON.DAT record indices (33..=40 -> 0..=7)
        assert_eq!(dungeon_record_index(32), None);
        assert_eq!(dungeon_record_index(33), Some(0));
        assert_eq!(dungeon_record_index(40), Some(7));
        assert_eq!(dungeon_record_index(41), None);

        // Per-mode minute increments
        assert_eq!(mode_minute_increment(SceneRoute::Overworld), Some(2));
        assert_eq!(mode_minute_increment(SceneRoute::TownFamily), Some(1));
        assert_eq!(mode_minute_increment(SceneRoute::Dungeon), Some(1));
        assert_eq!(mode_minute_increment(SceneRoute::IntroOrPreview), None);
        assert_eq!(mode_minute_increment(SceneRoute::CombatTemporary), None);
    }

    #[test]
    fn npc_path_direction_codes_match_spec_table() {
        // npc-schedules.md §8.2
        assert_eq!(NPC_PATH_DIR_WEST, 1);
        assert_eq!(NPC_PATH_DIR_SOUTH, 2);
        assert_eq!(NPC_PATH_DIR_EAST, 3);
        assert_eq!(NPC_PATH_DIR_NORTH, 4);
        // Coordinate effects
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_WEST), (-1, 0));
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_SOUTH), (0, 1));
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_EAST), (1, 0));
        assert_eq!(npc_path_direction_offset(NPC_PATH_DIR_NORTH), (0, -1));
        assert_eq!(npc_path_direction_offset(0), (0, 0));
        assert_eq!(npc_path_direction_offset(5), (0, 0));
        // Opposite-direction reversal
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_WEST),
            Some(NPC_PATH_DIR_EAST)
        );
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_EAST),
            Some(NPC_PATH_DIR_WEST)
        );
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_NORTH),
            Some(NPC_PATH_DIR_SOUTH)
        );
        assert_eq!(
            npc_path_direction_opposite(NPC_PATH_DIR_SOUTH),
            Some(NPC_PATH_DIR_NORTH)
        );
        assert_eq!(npc_path_direction_opposite(0), None);
        assert_eq!(npc_path_direction_opposite(5), None);
        // Other §8 constants
        assert_eq!(NPC_PATHFIND_QUEUE_CAPACITY, 32);
        assert_eq!(NPC_FLOOR_LINK_TILE_C8, 0xC8);
        assert_eq!(NPC_FLOOR_LINK_TILE_C9, 0xC9);
    }

    #[test]
    fn animation_phase_step_classifies_per_spec() {
        // active-objects.md §8
        assert_eq!(ANIMATION_PHASE_STEADY_NIBBLE, 0x0F);
        // Steady marker
        assert_eq!(animation_phase_step(0x0F), AnimationPhaseStep::Steady);
        assert_eq!(animation_phase_step(0xFF), AnimationPhaseStep::Steady);
        // AI-eligible (zero nibble)
        assert_eq!(animation_phase_step(0x00), AnimationPhaseStep::AiEligible);
        assert_eq!(animation_phase_step(0xA0), AnimationPhaseStep::AiEligible);
        // Mid-cycle decrement
        assert_eq!(animation_phase_step(0x01), AnimationPhaseStep::Decrement(0));
        assert_eq!(animation_phase_step(0x05), AnimationPhaseStep::Decrement(4));
        assert_eq!(animation_phase_step(0x0E), AnimationPhaseStep::Decrement(13));
        assert_eq!(animation_phase_step(0xA5), AnimationPhaseStep::Decrement(4));
    }

    #[test]
    fn equipment_slot_block_indices_and_ownership_match_spec() {
        // inventory.md §3
        assert_eq!(EQUIPMENT_EMPTY_SLOT_SENTINEL, 0xFF);
        assert_eq!(EquipmentSlot::Helm.block_index(), 0);
        assert_eq!(EquipmentSlot::BodyArmour.block_index(), 1);
        assert_eq!(EquipmentSlot::WeaponHand.block_index(), 2);
        assert_eq!(EquipmentSlot::OffHand.block_index(), 3);
        assert_eq!(EquipmentSlot::Ring.block_index(), 4);
        assert_eq!(EquipmentSlot::AmuletOrNeck.block_index(), 5);
        // Ownership predicate
        let block = [0x05, 0x42, 0xFF, 0xFF, 0x10, 0xFF];
        assert!(character_has_readied(&block, 0x05));
        assert!(character_has_readied(&block, 0x42));
        assert!(character_has_readied(&block, 0x10));
        assert!(!character_has_readied(&block, 0x06));
        assert!(!character_has_readied(&block, 0xFF));
        // All-empty block
        let empty = [EQUIPMENT_EMPTY_SLOT_SENTINEL; 6];
        assert!(!character_has_readied(&empty, 0x05));
    }

    #[test]
    fn equipment_class_tag_round_trip_per_spec() {
        // inventory.md §3.1
        assert_eq!(EQUIPMENT_CLASS_HELM, 0x80);
        assert_eq!(EQUIPMENT_CLASS_BODY_ARMOUR, 0x40);
        assert_eq!(EQUIPMENT_CLASS_ONE_HAND, 0x20);
        assert_eq!(EQUIPMENT_CLASS_TWO_HAND, 0x30);
        assert_eq!(EQUIPMENT_CLASS_RING, 0x02);
        assert_eq!(EQUIPMENT_CLASS_AMULET, 0x04);
        assert_eq!(EQUIPMENT_CLASS_NONE, 0x00);
        // Round trip
        assert_eq!(equipment_class_tag(0x80), Some(EquipmentClassTag::Helm));
        assert_eq!(
            equipment_class_tag(0x40),
            Some(EquipmentClassTag::BodyArmour)
        );
        assert_eq!(equipment_class_tag(0x20), Some(EquipmentClassTag::OneHand));
        assert_eq!(equipment_class_tag(0x30), Some(EquipmentClassTag::TwoHand));
        assert_eq!(equipment_class_tag(0x02), Some(EquipmentClassTag::Ring));
        assert_eq!(equipment_class_tag(0x04), Some(EquipmentClassTag::Amulet));
        assert_eq!(equipment_class_tag(0x00), Some(EquipmentClassTag::None));
        // Unknown bit patterns return None
        assert_eq!(equipment_class_tag(0x01), None);
        assert_eq!(equipment_class_tag(0x10), None);
        assert_eq!(equipment_class_tag(0xFF), None);

        // Cross-check that the existing tag table only uses values we
        // can classify.
        for tag in EQUIPMENT_CLASS_TAGS {
            assert!(equipment_class_tag(tag).is_some(), "unknown tag {tag:#x}");
        }
    }

    #[test]
    fn inventory_caps_match_spec() {
        // inventory.md §2
        assert_eq!(PARTY_GOLD_CAP, 9999);
        assert_eq!(PARTY_FOOD_CAP, 9999);
        assert_eq!(SPELL_CHARGE_CAP, 99);
        assert_eq!(EQUIPMENT_STOCK_CAP, 99);
        assert_eq!(EQUIPMENT_STOCK_BAND_LEN, 48);
        assert_eq!(SPELL_CHARGE_BAND_LEN, 48);
        // Cross-check: shop's existing SHOP_GOLD_CAP matches the
        // inventory layer's gold cap.
        assert_eq!(SHOP_GOLD_CAP as u16, PARTY_GOLD_CAP);
        // Cross-check: spell band len equals SPELL_COUNT.
        assert_eq!(SPELL_CHARGE_BAND_LEN, SPELL_COUNT);
    }

    #[test]
    fn shrine_quest_state_decodes_bit_pair_per_spec() {
        // karma.md §10
        assert_eq!(
            ShrineVirtue::shrine_quest_state(false, false),
            ShrineQuestState::NotStarted
        );
        assert_eq!(
            ShrineVirtue::shrine_quest_state(true, false),
            ShrineQuestState::Ordained
        );
        assert_eq!(
            ShrineVirtue::shrine_quest_state(true, true),
            ShrineQuestState::CodexRead
        );
        assert_eq!(
            ShrineVirtue::shrine_quest_state(false, true),
            ShrineQuestState::Complete
        );

        // "All virtues complete" terminal state: codex=0xFF, ordained=0
        assert!(all_virtues_complete(0, 0xFF));
        // Any ordained bit still set fails the predicate
        assert!(!all_virtues_complete(0x01, 0xFF));
        // Any codex bit still clear fails the predicate
        assert!(!all_virtues_complete(0, 0x7F));
        // Empty state is not "complete"
        assert!(!all_virtues_complete(0, 0));
    }

    #[test]
    fn codex_turn_in_stat_steps_match_spec_table() {
        // karma.md §7
        assert_eq!(ShrineVirtue::Honesty.codex_turn_in_stat_steps(), (0, 0, 1));
        assert_eq!(
            ShrineVirtue::Compassion.codex_turn_in_stat_steps(),
            (0, 1, 0)
        );
        assert_eq!(ShrineVirtue::Valor.codex_turn_in_stat_steps(), (1, 0, 0));
        assert_eq!(
            ShrineVirtue::Justice.codex_turn_in_stat_steps(),
            (0, 1, 1)
        );
        assert_eq!(
            ShrineVirtue::Sacrifice.codex_turn_in_stat_steps(),
            (1, 1, 0)
        );
        assert_eq!(ShrineVirtue::Honor.codex_turn_in_stat_steps(), (1, 0, 1));
        assert_eq!(
            ShrineVirtue::Spirituality.codex_turn_in_stat_steps(),
            (1, 1, 1)
        );
        assert_eq!(
            ShrineVirtue::Humility.codex_turn_in_stat_steps(),
            (0, 0, 0)
        );
        // Humility bonus: +3 only on Humility
        for v in ShrineVirtue::ALL {
            let expected = if matches!(v, ShrineVirtue::Humility) { 3 } else { 0 };
            assert_eq!(v.codex_turn_in_humility_bonus(), expected);
        }
    }

    #[test]
    fn boot_driver_selection_matches_spec() {
        // boot.md §5 explicit selector parsing
        assert_eq!(
            parse_explicit_driver_selector(Some("C")),
            Some(DisplayDriverFamily::Cga)
        );
        assert_eq!(
            parse_explicit_driver_selector(Some("e")),
            Some(DisplayDriverFamily::Ega)
        );
        assert_eq!(
            parse_explicit_driver_selector(Some("Tandy")),
            Some(DisplayDriverFamily::Tandy)
        );
        assert_eq!(
            parse_explicit_driver_selector(Some("h")),
            Some(DisplayDriverFamily::Hercules)
        );
        assert_eq!(parse_explicit_driver_selector(Some("X")), None);
        assert_eq!(parse_explicit_driver_selector(Some("")), None);
        assert_eq!(parse_explicit_driver_selector(None), None);

        // Resolution: explicit wins
        assert_eq!(
            resolve_driver_family(Some(DisplayDriverFamily::Cga), GraphicsCapability::Ega),
            Some(DisplayDriverFamily::Cga)
        );
        // Auto-detect mapping
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::GenericFourColour),
            Some(DisplayDriverFamily::Cga)
        );
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::Ega),
            Some(DisplayDriverFamily::Ega)
        );
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::Tandy),
            Some(DisplayDriverFamily::Tandy)
        );
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::Hercules),
            Some(DisplayDriverFamily::Hercules)
        );
        // EgaSentinel without an explicit selector takes no driver-load
        // path.
        assert_eq!(
            resolve_driver_family(None, GraphicsCapability::EgaSentinel),
            None
        );

        // Filenames
        assert_eq!(DisplayDriverFamily::Cga.driver_filename(), "CGA.DRV");
        assert_eq!(DisplayDriverFamily::Ega.driver_filename(), "EGA.DRV");
        assert_eq!(DisplayDriverFamily::Tandy.driver_filename(), "T1K.DRV");
        assert_eq!(DisplayDriverFamily::Hercules.driver_filename(), "HER.DRV");

        // Tandy low-memory downgrade threshold
        assert_eq!(TANDY_LOW_MEMORY_THRESHOLD_KB, 368);
        assert!(tandy_low_memory_downgrades(367));
        assert!(!tandy_low_memory_downgrades(368));
        assert!(!tandy_low_memory_downgrades(640));
    }

    #[test]
    fn character_class_and_status_letter_round_trip() {
        // formats/saved-gam.md §3.1
        for class in [
            CharacterClass::Avatar,
            CharacterClass::Bard,
            CharacterClass::Fighter,
            CharacterClass::Mage,
            CharacterClass::Druid,
            CharacterClass::Tinker,
            CharacterClass::Paladin,
            CharacterClass::Ranger,
            CharacterClass::Shepherd,
        ] {
            let byte = class.save_byte();
            assert_eq!(character_class_for_byte(byte), Some(class));
        }
        // Specific byte mappings
        assert_eq!(character_class_for_byte(b'A'), Some(CharacterClass::Avatar));
        assert_eq!(character_class_for_byte(b'M'), Some(CharacterClass::Mage));
        assert_eq!(character_class_for_byte(b'P'), Some(CharacterClass::Paladin));
        // Out-of-range bytes
        assert_eq!(character_class_for_byte(0), None);
        assert_eq!(character_class_for_byte(b'X'), None);
        assert_eq!(character_class_for_byte(b'a'), None);
        // Status round-trip
        for status in [
            CharacterStatus::Good,
            CharacterStatus::PoisonedOrRevived,
            CharacterStatus::Sleeping,
            CharacterStatus::Charmed,
            CharacterStatus::Dead,
            CharacterStatus::Ashes,
        ] {
            let byte = status.save_byte();
            assert_eq!(character_status_for_byte(byte), Some(status));
        }
        assert_eq!(
            character_status_for_byte(b'G'),
            Some(CharacterStatus::Good)
        );
        assert_eq!(character_status_for_byte(b'X'), None);
        // Status 'P' is shared between poison and revive paths;
        // class 'P' is Paladin. They don't collide because they live in
        // different record fields.
        assert_eq!(
            character_status_for_byte(b'P'),
            Some(CharacterStatus::PoisonedOrRevived)
        );
        assert_eq!(
            character_class_for_byte(b'P'),
            Some(CharacterClass::Paladin)
        );
    }

    #[test]
    fn paragraph_byte_kind_classifies_per_spec() {
        // formats/font-pcs.md §4
        assert_eq!(paragraph_byte_kind(0x00), ParagraphByteKind::EndOfStream);
        assert_eq!(paragraph_byte_kind(b' '), ParagraphByteKind::SpaceBreak);
        assert_eq!(paragraph_byte_kind(b'\n'), ParagraphByteKind::HardBreak);
        assert_eq!(paragraph_byte_kind(b'\r'), ParagraphByteKind::HardBreak);
        assert_eq!(paragraph_byte_kind(b'_'), ParagraphByteKind::SoftHyphen);
        assert_eq!(paragraph_byte_kind(b'{'), ParagraphByteKind::PageMarker);
        // Glyph cases
        assert_eq!(paragraph_byte_kind(b'A'), ParagraphByteKind::Glyph);
        assert_eq!(paragraph_byte_kind(b'1'), ParagraphByteKind::Glyph);
        assert_eq!(paragraph_byte_kind(b'!'), ParagraphByteKind::Glyph);
        assert_eq!(paragraph_byte_kind(b'}'), ParagraphByteKind::Glyph);
        // Tab is not a special paragraph byte; renderer treats it as a
        // glyph and reads the width table.
        assert_eq!(paragraph_byte_kind(0x09), ParagraphByteKind::Glyph);
    }

    #[test]
    fn wrap_byte_kind_classifies_break_visible_and_control() {
        // text-output.md §6
        assert_eq!(wrap_byte_kind(0x00), WrapByteKind::Break);
        assert_eq!(wrap_byte_kind(b'\n'), WrapByteKind::Break);
        assert_eq!(wrap_byte_kind(b'\r'), WrapByteKind::Break);
        assert_eq!(wrap_byte_kind(b' '), WrapByteKind::Break);
        // Visible: low-ASCII printable except space
        assert_eq!(wrap_byte_kind(b'A'), WrapByteKind::Visible);
        assert_eq!(wrap_byte_kind(b'!'), WrapByteKind::Visible);
        assert_eq!(wrap_byte_kind(b'~'), WrapByteKind::Visible);
        assert_eq!(wrap_byte_kind(b'0'), WrapByteKind::Visible);
        // Control: tab, escape, high-bit, etc.
        assert_eq!(wrap_byte_kind(0x09), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0x1B), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0x7F), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0x80), WrapByteKind::Control);
        assert_eq!(wrap_byte_kind(0xFF), WrapByteKind::Control);
        // Min line-buffer width sanity
        assert!(WRAP_MIN_LINE_BUFFER >= 64);
    }

    #[test]
    fn command_for_letter_covers_full_a_to_z_table() {
        // commands.md §4
        assert_eq!(command_for_letter(b' '), Some(Command::Pass));
        assert_eq!(command_for_letter(b'A'), Some(Command::Attack));
        assert_eq!(command_for_letter(b'B'), Some(Command::Board));
        assert_eq!(command_for_letter(b'C'), Some(Command::Cast));
        assert_eq!(
            command_for_letter(b'D'),
            Some(Command::UnassignedRefusal)
        );
        assert_eq!(command_for_letter(b'E'), Some(Command::Enter));
        assert_eq!(command_for_letter(b'F'), Some(Command::Fire));
        assert_eq!(command_for_letter(b'G'), Some(Command::Get));
        assert_eq!(command_for_letter(b'H'), Some(Command::HoleUp));
        assert_eq!(command_for_letter(b'I'), Some(Command::Ignite));
        assert_eq!(command_for_letter(b'J'), Some(Command::Jimmy));
        assert_eq!(command_for_letter(b'K'), Some(Command::Klimb));
        assert_eq!(command_for_letter(b'L'), Some(Command::Look));
        assert_eq!(command_for_letter(b'M'), Some(Command::Mix));
        assert_eq!(command_for_letter(b'N'), Some(Command::NewOrder));
        assert_eq!(command_for_letter(b'O'), Some(Command::Open));
        assert_eq!(command_for_letter(b'P'), Some(Command::Push));
        assert_eq!(command_for_letter(b'Q'), Some(Command::Quit));
        assert_eq!(command_for_letter(b'R'), Some(Command::Ready));
        assert_eq!(command_for_letter(b'S'), Some(Command::Search));
        assert_eq!(command_for_letter(b'T'), Some(Command::Talk));
        assert_eq!(command_for_letter(b'U'), Some(Command::Use));
        assert_eq!(command_for_letter(b'V'), Some(Command::View));
        assert_eq!(
            command_for_letter(b'W'),
            Some(Command::UnassignedRefusal)
        );
        assert_eq!(command_for_letter(b'X'), Some(Command::Xit));
        assert_eq!(command_for_letter(b'Y'), Some(Command::Yell));
        assert_eq!(command_for_letter(b'Z'), Some(Command::ZStats));
        // Lowercase folded
        assert_eq!(command_for_letter(b'a'), Some(Command::Attack));
        // Outside range
        assert_eq!(command_for_letter(b'0'), None);
        assert_eq!(command_for_letter(0), None);
        // Verb prefix sample
        assert_eq!(Command::Attack.verb_prefix(), "Attack");
        assert_eq!(Command::Cast.verb_prefix(), "Cast");
        assert_eq!(Command::HoleUp.verb_prefix(), "Hole up");
        assert_eq!(Command::NewOrder.verb_prefix(), "New order");
        assert_eq!(Command::Xit.verb_prefix(), "X-it");
        assert_eq!(Command::ZStats.verb_prefix(), "Z-stats");
        assert_eq!(Command::UnassignedRefusal.verb_prefix(), "What?");
    }

    #[test]
    fn intro_menu_action_matches_spec_keys() {
        // intro.md §6
        assert_eq!(intro_menu_action(b'J'), Some(IntroMenuAction::JourneyOnward));
        assert_eq!(
            intro_menu_action(b'C'),
            Some(IntroMenuAction::CreateNewCharacter)
        );
        assert_eq!(
            intro_menu_action(b'T'),
            Some(IntroMenuAction::TransferFromUltimaIv)
        );
        assert_eq!(
            intro_menu_action(b'U'),
            Some(IntroMenuAction::UltimaVIntroduction)
        );
        assert_eq!(
            intro_menu_action(b'A'),
            Some(IntroMenuAction::Acknowledgements)
        );
        assert_eq!(intro_menu_action(b'R'), Some(IntroMenuAction::ReturnToView));
        // Lowercase folded
        assert_eq!(intro_menu_action(b'j'), Some(IntroMenuAction::JourneyOnward));
        assert_eq!(intro_menu_action(b'r'), Some(IntroMenuAction::ReturnToView));
        // Enter / Return -> RepeatCachedSelection
        assert_eq!(
            intro_menu_action(b'\r'),
            Some(IntroMenuAction::RepeatCachedSelection)
        );
        assert_eq!(
            intro_menu_action(b'\n'),
            Some(IntroMenuAction::RepeatCachedSelection)
        );
        // Invalid
        assert_eq!(intro_menu_action(b'B'), None);
        assert_eq!(intro_menu_action(b'X'), None);
        assert_eq!(intro_menu_action(0), None);
        assert_eq!(intro_menu_action(b' '), None);
    }

    #[test]
    fn boardable_family_classifier_matches_spec_table() {
        // vehicles.md §4
        assert_eq!(boardable_family(0x10), Some(BoardableFamily::Horse));
        assert_eq!(boardable_family(0x11), Some(BoardableFamily::Horse));
        // Mounted-horse ranges are not boardable parked objects.
        assert_eq!(boardable_family(0x12), None);
        assert_eq!(boardable_family(0x13), None);
        // Carpet
        assert_eq!(boardable_family(0x1B), Some(BoardableFamily::MagicCarpet));
        assert_eq!(boardable_family(0x14), None);
        // Ship
        for byte in 0x24..=0x27u8 {
            assert_eq!(boardable_family(byte), Some(BoardableFamily::Ship));
        }
        // Skiff
        for byte in 0x28..=0x2Bu8 {
            assert_eq!(boardable_family(byte), Some(BoardableFamily::Skiff));
        }
        assert_eq!(boardable_family(0x2C), None);
        assert_eq!(boardable_family(0x00), None);
        // Mount horse marker
        assert_eq!(mount_horse_marker(0x10), Some(0x12));
        assert_eq!(mount_horse_marker(0x11), Some(0x13));
        assert_eq!(mount_horse_marker(0x12), None);
        assert_eq!(mount_horse_marker(0x1B), None);
        // Ship boarding warning predicate
        assert_eq!(SHIP_BOARDING_HULL_WARNING_THRESHOLD, 10);
        assert!(ship_boarding_warns(0, 2)); // hull below 10
        assert!(ship_boarding_warns(9, 2)); // hull below 10
        assert!(!ship_boarding_warns(10, 2));
        assert!(ship_boarding_warns(50, 0)); // no skiffs
        assert!(!ship_boarding_warns(50, 1));
    }

    #[test]
    fn shoppe_placeholder_classifier_matches_spec_table() {
        // formats/shoppe-dat.md §2,§4
        assert_eq!(SHOPPE_DAT_LEN, 10_135);
        assert_eq!(SHOPPE_DAT_RECORD_SLOTS, 196);
        assert_eq!(SHOPPE_DAT_NONEMPTY_RECORDS, 194);
        assert_eq!(shoppe_placeholder(b'%'), Some(ShoppePlaceholder::Gold));
        assert_eq!(
            shoppe_placeholder(b'^'),
            Some(ShoppePlaceholder::Quantity)
        );
        assert_eq!(
            shoppe_placeholder(b'$'),
            Some(ShoppePlaceholder::VendorName)
        );
        assert_eq!(shoppe_placeholder(b'&'), Some(ShoppePlaceholder::ItemName));
        assert_eq!(
            shoppe_placeholder(b'*'),
            Some(ShoppePlaceholder::PlaceName)
        );
        assert_eq!(shoppe_placeholder(b'#'), Some(ShoppePlaceholder::ShopName));
        assert_eq!(shoppe_placeholder(b'@'), Some(ShoppePlaceholder::TimeOfDay));
        // Ordinary text bytes
        assert_eq!(shoppe_placeholder(b'A'), None);
        assert_eq!(shoppe_placeholder(b' '), None);
        assert_eq!(shoppe_placeholder(b'1'), None);
        assert_eq!(shoppe_placeholder(0x80), None);
    }

    #[test]
    fn reagent_abbreviation_matches_spec_table() {
        // magic.md §2
        assert_eq!(Reagent::SulfurAsh.abbreviation(), "Sulfur Ash");
        assert_eq!(Reagent::Ginseng.abbreviation(), "Ginseng");
        assert_eq!(Reagent::Garlic.abbreviation(), "Garlic");
        assert_eq!(Reagent::SpiderSilk.abbreviation(), "Sp. Silk");
        assert_eq!(Reagent::BloodMoss.abbreviation(), "Blood Moss");
        assert_eq!(Reagent::BlackPearl.abbreviation(), "Blk. Pearl");
        assert_eq!(Reagent::Nightshade.abbreviation(), "Nightshade");
        assert_eq!(Reagent::Mandrake.abbreviation(), "Mandrake");
        // Display name and abbreviation differ only for the two
        // multi-word reagents whose long form does not fit a tight UI
        // line ("Spider Silk"/"Black Pearl").
        for r in REAGENT_VENDOR_ORDER {
            let differs = matches!(r, Reagent::SpiderSilk | Reagent::BlackPearl);
            assert_eq!(r.display_name() != r.abbreviation(), differs);
        }
    }

    #[test]
    fn spell_indoor_absorbs_matches_spec_short_circuits() {
        // catalogs/spell-list.md §4
        // Stonegate absorbs unconditionally
        assert!(spell_indoor_absorbs(false, true, true));
        assert!(spell_indoor_absorbs(false, false, true));
        assert!(spell_indoor_absorbs(true, true, true));
        // Blackthorn absorbs only without the Crown
        assert!(spell_indoor_absorbs(true, false, false));
        assert!(!spell_indoor_absorbs(true, true, false));
        // Other indoor scenes pass through
        assert!(!spell_indoor_absorbs(false, false, false));
        assert!(!spell_indoor_absorbs(false, true, false));
    }

    #[test]
    fn rune_syllable_vocabulary_matches_spec_table() {
        // magic.md §3
        assert_eq!(RUNE_SYLLABLE_VOCABULARY.len(), 24);
        assert_eq!(RUNE_SYLLABLE_VOCABULARY[0], "An");
        assert_eq!(RUNE_SYLLABLE_VOCABULARY[7], "Hur");
        assert_eq!(RUNE_SYLLABLE_VOCABULARY[23], "Zu");
        // Resident syllables accept (case-insensitive)
        assert!(is_resident_rune_syllable("An"));
        assert!(is_resident_rune_syllable("an"));
        assert!(is_resident_rune_syllable("MANI"));
        assert!(is_resident_rune_syllable("Vas"));
        assert!(is_resident_rune_syllable("Quas"));
        assert!(is_resident_rune_syllable("Xen"));
        assert!(is_resident_rune_syllable("Ylem"));
        // Older Ultima lore syllables are rejected
        assert!(!is_resident_rune_syllable("Jux"));
        assert!(!is_resident_rune_syllable("Ort"));
        assert!(!is_resident_rune_syllable("jux"));
        assert!(!is_resident_rune_syllable(""));
        assert!(!is_resident_rune_syllable("Foo"));
        // Cross-check: every spec entry is accepted
        for syllable in RUNE_SYLLABLE_VOCABULARY {
            assert!(is_resident_rune_syllable(syllable));
        }
    }

    #[test]
    fn spell_common_name_covers_all_48_indices() {
        // magic.md §4
        assert_eq!(SPELL_CIRCLE_COUNT, 8);
        assert_eq!(SPELLS_PER_CIRCLE, 6);
        assert_eq!(SPELL_COUNT, SPELL_CIRCLE_COUNT * SPELLS_PER_CIRCLE);

        let expected_first_per_circle = [
            "Light",
            "Open",
            "Great Light",
            "Dispel Field",
            "Swarm",
            "Tremor",
            "Invisibility",
            "Resurrect",
        ];
        for (circle, name) in expected_first_per_circle.iter().enumerate() {
            let idx = circle * SPELLS_PER_CIRCLE;
            assert_eq!(spell_common_name(idx), Some(*name));
        }
        let expected_last_per_circle = [
            "Vanish",
            "Create Food",
            "Blink",
            "Reveal",
            "Quickness",
            "Polymorph",
            "Cause Fear",
            "Negate Time",
        ];
        for (circle, name) in expected_last_per_circle.iter().enumerate() {
            let idx = circle * SPELLS_PER_CIRCLE + (SPELLS_PER_CIRCLE - 1);
            assert_eq!(spell_common_name(idx), Some(*name));
        }
        // Every index 0..=47 returns Some
        for i in 0..SPELL_COUNT {
            assert!(spell_common_name(i).is_some(), "missing name {i}");
        }
        assert_eq!(spell_common_name(48), None);
        assert_eq!(spell_common_name(255), None);
    }

    #[test]
    fn cast_dispatcher_gate_matches_spec_order_and_messages() {
        // magic.md §7
        // Scene gate first: Not here! before charge consumption.
        let r = cast_dispatcher_gate(false, 0, 0, 0, 0);
        assert_eq!(r, CastGateOutcome::NotHere);
        assert!(!r.consumed_charge());
        assert!(!r.consumed_mana());
        assert_eq!(r.message(), "Not here!");

        // No charges: None mixed!, no charge spent.
        let r = cast_dispatcher_gate(true, 0, 99, 8, 1);
        assert_eq!(r, CastGateOutcome::NoneMixed);
        assert!(!r.consumed_charge());
        assert!(!r.consumed_mana());
        assert_eq!(r.message(), "None mixed!");

        // Mana too low: charge spent, mana not.
        let r = cast_dispatcher_gate(true, 1, 2, 8, 5);
        assert_eq!(r, CastGateOutcome::ManaTooLowChargeOnly);
        assert!(r.consumed_charge());
        assert!(!r.consumed_mana());
        assert_eq!(r.message(), "M.P. too low!");

        // Level too low: charge AND mana spent.
        let r = cast_dispatcher_gate(true, 1, 99, 1, 5);
        assert_eq!(r, CastGateOutcome::LevelTooLowChargeAndMana);
        assert!(r.consumed_charge());
        assert!(r.consumed_mana());
        assert_eq!(r.message(), "M.P. too low!");

        // All gates pass.
        let r = cast_dispatcher_gate(true, 1, 99, 8, 5);
        assert_eq!(r, CastGateOutcome::Cast);
        assert!(r.consumed_charge());
        assert!(r.consumed_mana());

        // Heal amount: 0..=60 roll, halved, zero -> 1.
        assert_eq!(heal_spell_amount_from_raw_roll_u8(0), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(1), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(2), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(3), 1);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(4), 2);
        assert_eq!(heal_spell_amount_from_raw_roll_u8(60), 30);
    }

    #[test]
    fn combat_combatant_capacity_matches_spec() {
        // active-objects.md §7
        assert_eq!(COMBAT_MAX_COMBATANTS, 26);
        assert_eq!(COMBAT_MONSTER_SLOT_FIRST, 1);
        assert_eq!(COMBAT_MONSTER_SLOT_LAST, 25);
        assert_eq!(
            COMBAT_MONSTER_SLOT_LAST - COMBAT_MONSTER_SLOT_FIRST + 1,
            COMBAT_MAX_COMBATANTS - 1
        );
        // Player slot 0 plus 25 monster slots == 26 total combatants.
        assert!(COMBAT_PARTY_ACTOR_SLOTS <= COMBAT_MAX_COMBATANTS);
        assert!(COMBAT_MAX_COMBATANTS <= COMBAT_ACTOR_SLOTS);
    }

    #[test]
    fn first_monster_ability_picks_in_spec_order() {
        // combat.md §9
        assert_eq!(MONSTER_ABILITY_POSSESS, 0x0040);
        assert_eq!(MONSTER_ABILITY_BLINK, 0x0800);
        assert_eq!(MONSTER_ABILITY_SUMMON_DAEMON, 0x0400);
        // No ability bits
        assert_eq!(first_monster_ability(0), None);
        assert_eq!(first_monster_ability(0xFFFF & !(0x0040 | 0x0400 | 0x0800)), None);
        // One bit at a time
        assert_eq!(
            first_monster_ability(MONSTER_ABILITY_POSSESS),
            Some(MonsterAbility::Possess)
        );
        assert_eq!(
            first_monster_ability(MONSTER_ABILITY_BLINK),
            Some(MonsterAbility::Blink)
        );
        assert_eq!(
            first_monster_ability(MONSTER_ABILITY_SUMMON_DAEMON),
            Some(MonsterAbility::SummonDaemon)
        );
        // Multiple bits — possess wins, then blink, then summon
        assert_eq!(
            first_monster_ability(MONSTER_ABILITY_POSSESS | MONSTER_ABILITY_BLINK),
            Some(MonsterAbility::Possess)
        );
        assert_eq!(
            first_monster_ability(
                MONSTER_ABILITY_POSSESS
                    | MONSTER_ABILITY_BLINK
                    | MONSTER_ABILITY_SUMMON_DAEMON
            ),
            Some(MonsterAbility::Possess)
        );
        assert_eq!(
            first_monster_ability(MONSTER_ABILITY_BLINK | MONSTER_ABILITY_SUMMON_DAEMON),
            Some(MonsterAbility::Blink)
        );
    }

    #[test]
    fn tile_super_category_splits_per_spec() {
        // catalogs/tile-catalog.md §2
        // Map terrain band 0..=159
        assert_eq!(tile_super_category(0), Some(TileSuperCategory::MapTerrain));
        assert_eq!(tile_super_category(50), Some(TileSuperCategory::MapTerrain));
        assert_eq!(
            tile_super_category(159),
            Some(TileSuperCategory::MapTerrain)
        );
        // Actor band 160..=511
        assert_eq!(tile_super_category(160), Some(TileSuperCategory::Actor));
        assert_eq!(tile_super_category(256), Some(TileSuperCategory::Actor));
        assert_eq!(tile_super_category(511), Some(TileSuperCategory::Actor));
        // Above the published sheet
        assert_eq!(tile_super_category(512), None);
        assert_eq!(tile_super_category(65535), None);
        // Water animates with a four-frame cycle
        assert_eq!(tile_animation_cycle_length(0x01), Some(4));
        assert_eq!(tile_animation_cycle_length(0x04), Some(4));
        // Walls and most other classes do not animate
        assert_eq!(tile_animation_cycle_length(0x18), None);
        assert_eq!(tile_animation_cycle_length(0x60), None);
    }

    #[test]
    fn dungeon_pit_trap_kind_classifies_per_spec_table() {
        // dungeon-mode.md §8
        assert_eq!(dungeon_pit_trap_kind(0x60), Some(DungeonPitTrap::PlainPit));
        assert_eq!(dungeon_pit_trap_kind(0x61), Some(DungeonPitTrap::FallTrap));
        assert_eq!(dungeon_pit_trap_kind(0x69), Some(DungeonPitTrap::FallTrap));
        assert_eq!(dungeon_pit_trap_kind(0x62), Some(DungeonPitTrap::BombTrap));
        assert_eq!(dungeon_pit_trap_kind(0x6A), Some(DungeonPitTrap::BombTrap));
        // Unnamed members of the family
        assert_eq!(
            dungeon_pit_trap_kind(0x63),
            Some(DungeonPitTrap::GenericPitFamily)
        );
        assert_eq!(
            dungeon_pit_trap_kind(0x6F),
            Some(DungeonPitTrap::GenericPitFamily)
        );
        // Outside the band
        assert_eq!(dungeon_pit_trap_kind(0x5F), None);
        assert_eq!(dungeon_pit_trap_kind(0x70), None);
        // Constants
        assert_eq!(DUNGEON_DEEPEST_LEVEL, 7);
        assert_eq!(DUNGEON_VISIT_MARKER_BIT, 0x08);
        // Fall-trap visit-mark predicate
        assert!(dungeon_fall_destination_marks_visit(0x00));
        assert!(dungeon_fall_destination_marks_visit(0x8F));
        assert!(!dungeon_fall_destination_marks_visit(0x90));
        assert!(!dungeon_fall_destination_marks_visit(0xFF));
        // Search rewrite targets
        assert_eq!(DUNGEON_SEARCH_FLAVOR_REWRITE_PRIMARY, 0xB0);
        assert_eq!(DUNGEON_SEARCH_FLAVOR_REWRITE_VISITED, 0xB8);
        assert_eq!(DUNGEON_SEARCH_WALL_REWRITE_PRIMARY, 0xE0);
        assert_eq!(DUNGEON_SEARCH_WALL_REWRITE_VISITED, 0xE8);
    }

    #[test]
    fn fountain_and_energy_field_classifiers_match_spec() {
        // dungeon-mode.md §8 fountain
        assert_eq!(fountain_effect_from_byte(0x50), FountainEffect::Cure);
        assert_eq!(fountain_effect_from_byte(0x51), FountainEffect::Heal);
        assert_eq!(fountain_effect_from_byte(0x52), FountainEffect::Poison);
        // Sub-types 3..=15 all map to BadTaste
        for low in 3..=15u8 {
            assert_eq!(
                fountain_effect_from_byte(0x50 | low),
                FountainEffect::BadTaste
            );
        }
        // High nibble doesn't matter — only the low nibble is read.
        assert_eq!(fountain_effect_from_byte(0xA0), FountainEffect::Cure);

        // dungeon-mode.md §8 energy fields
        assert_eq!(energy_field_kind_from_byte(0x80), EnergyFieldKind::Sleep);
        assert_eq!(energy_field_kind_from_byte(0x81), EnergyFieldKind::Poison);
        assert_eq!(energy_field_kind_from_byte(0x82), EnergyFieldKind::Fire);
        assert_eq!(
            energy_field_kind_from_byte(0x83),
            EnergyFieldKind::Electric
        );
        // Magic-placement preserves the visit-marker bit (0x88..=0x8B);
        // the low-bit-only classifier collapses these to Generic
        // because the renderer still needs to recognise the variant
        // separately. The L-Look text uses the same Generic name.
        assert_eq!(energy_field_kind_from_byte(0x88), EnergyFieldKind::Generic);
        assert_eq!(energy_field_kind_from_byte(0x89), EnergyFieldKind::Generic);
        assert_eq!(energy_field_kind_from_byte(0x8F), EnergyFieldKind::Generic);
    }

    #[test]
    fn dungeon_cell_class_of_matches_high_nibble_table() {
        // dungeon-mode.md §3
        assert_eq!(dungeon_cell_class_of(0x00), DungeonCellClass::Passage);
        assert_eq!(dungeon_cell_class_of(0x0F), DungeonCellClass::Passage);
        assert_eq!(dungeon_cell_class_of(0x10), DungeonCellClass::UpLadder);
        assert_eq!(dungeon_cell_class_of(0x20), DungeonCellClass::DownLadder);
        assert_eq!(
            dungeon_cell_class_of(0x30),
            DungeonCellClass::TwoWayLadder
        );
        assert_eq!(dungeon_cell_class_of(0x40), DungeonCellClass::Chest);
        assert_eq!(dungeon_cell_class_of(0x50), DungeonCellClass::Fountain);
        assert_eq!(dungeon_cell_class_of(0x60), DungeonCellClass::PitTrap);
        assert_eq!(dungeon_cell_class_of(0x69), DungeonCellClass::PitTrap);
        assert_eq!(
            dungeon_cell_class_of(0x70),
            DungeonCellClass::PassageVariant
        );
        assert_eq!(
            dungeon_cell_class_of(0x80),
            DungeonCellClass::EnergyField
        );
        assert_eq!(
            dungeon_cell_class_of(0x90),
            DungeonCellClass::EnergyFieldSecondary
        );
        assert_eq!(
            dungeon_cell_class_of(0xA0),
            DungeonCellClass::RoomHelperState
        );
        for high in 0xB..=0xE {
            assert_eq!(dungeon_cell_class_of(high << 4), DungeonCellClass::Wall);
        }
        assert_eq!(
            dungeon_cell_class_of(0xF0),
            DungeonCellClass::HeavyDoorOrRoomTrigger
        );
        // Convenience predicates
        assert!(DungeonCellClass::Wall.is_wall());
        assert!(!DungeonCellClass::Passage.is_wall());
        assert!(DungeonCellClass::UpLadder.is_ladder());
        assert!(DungeonCellClass::DownLadder.is_ladder());
        assert!(DungeonCellClass::TwoWayLadder.is_ladder());
        assert!(!DungeonCellClass::Chest.is_ladder());
        assert!(DungeonCellClass::Passage.is_passage_like());
        assert!(DungeonCellClass::PassageVariant.is_passage_like());
        assert!(!DungeonCellClass::Wall.is_passage_like());
    }

    #[test]
    fn daylight_base_value_matches_spec_table() {
        // time.md §6
        // Underworld / dungeon depth are always dark
        assert_eq!(daylight_base_value(12, 0, true, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(12, 0, false, 1), FULL_DARKNESS);
        // Pre-dawn / post-dusk surface
        assert_eq!(daylight_base_value(0, 0, false, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(4, 59, false, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(20, 0, false, 0), FULL_DARKNESS);
        assert_eq!(daylight_base_value(23, 0, false, 0), FULL_DARKNESS);
        // Daytime band
        for hour in 6..=18u8 {
            assert_eq!(daylight_base_value(hour, 0, false, 0), FULL_DAYLIGHT);
            assert_eq!(daylight_base_value(hour, 30, false, 0), FULL_DAYLIGHT);
        }
        // Dawn at hour 5
        assert_eq!(daylight_base_value(5, 0, false, 0), 2);
        assert_eq!(daylight_base_value(5, 9, false, 0), 2);
        assert_eq!(daylight_base_value(5, 10, false, 0), 5);
        assert_eq!(daylight_base_value(5, 19, false, 0), 5);
        assert_eq!(daylight_base_value(5, 20, false, 0), 10);
        assert_eq!(daylight_base_value(5, 30, false, 0), 20);
        assert_eq!(daylight_base_value(5, 40, false, 0), 34);
        assert_eq!(daylight_base_value(5, 50, false, 0), 49);
        assert_eq!(daylight_base_value(5, 59, false, 0), 49);
        // Dusk at hour 19 (mirror of dawn)
        assert_eq!(daylight_base_value(19, 0, false, 0), 49);
        assert_eq!(daylight_base_value(19, 9, false, 0), 49);
        assert_eq!(daylight_base_value(19, 10, false, 0), 34);
        assert_eq!(daylight_base_value(19, 20, false, 0), 20);
        assert_eq!(daylight_base_value(19, 30, false, 0), 10);
        assert_eq!(daylight_base_value(19, 40, false, 0), 5);
        assert_eq!(daylight_base_value(19, 50, false, 0), 2);
        assert_eq!(daylight_base_value(19, 59, false, 0), 2);
    }

    #[test]
    fn normalize_disk_prompt_mode_folds_2_and_5_to_1() {
        // screen-mode-dispatch.md §5
        assert_eq!(normalize_disk_prompt_mode(0), 0);
        assert_eq!(normalize_disk_prompt_mode(1), 1);
        assert_eq!(normalize_disk_prompt_mode(2), 1);
        assert_eq!(normalize_disk_prompt_mode(3), 3);
        assert_eq!(normalize_disk_prompt_mode(4), 4);
        assert_eq!(normalize_disk_prompt_mode(5), 1);
        assert_eq!(normalize_disk_prompt_mode(6), 6);
        assert_eq!(normalize_disk_prompt_mode(255), 255);
    }

    #[test]
    fn save_top_level_constants_match_spec() {
        // formats/saved-gam.md §2,§3,§4
        assert_eq!(SAVED_GAM_LEN, 4192);
        assert_eq!(SAVE_LEADING_BYTES_LEN, 2);
        assert_eq!(SAVE_CHARACTER_ROSTER_SLOTS, 16);
        assert_eq!(SAVE_ROSTER_OFFSET, 0x0002);
        assert_eq!(SAVE_CHARACTER_RECORD_LEN, 32);
        // The roster occupies SLOTS*RECORD_LEN = 512 bytes
        assert_eq!(SAVE_CHARACTER_ROSTER_SLOTS * SAVE_CHARACTER_RECORD_LEN, 512);
        // Party-size byte
        assert_eq!(SAVE_PARTY_SIZE_OFFSET, 0x02B5);
        assert_eq!(SAVE_PARTY_SIZE_MIN, 1);
        assert_eq!(SAVE_PARTY_SIZE_MAX, 6);
    }

    #[test]
    fn save_active_object_and_dungeon_buffer_offsets_match_spec() {
        // formats/saved-gam.md §8.1, §8.2
        assert_eq!(SAVE_ACTIVE_OBJECT_TABLE_OFFSET, 0x06B4);
        assert_eq!(SAVE_DUNGEON_WORKING_BUFFER_OFFSET, 0x03B4);
        assert_eq!(SAVE_DUNGEON_WORKING_BUFFER_LEN, 512);
        // 32 records × 8 bytes = 256 bytes
        assert_eq!(OOL_RECORD_LEN * OOL_SLOTS, ACTIVE_OBJECT_SAVE_BYTES);
        assert_eq!(ACTIVE_OBJECT_SAVE_BYTES, 256);
        // Active-object table fits inside the save image
        assert!(SAVE_ACTIVE_OBJECT_TABLE_OFFSET + ACTIVE_OBJECT_SAVE_BYTES <= 0x1060);
    }

    #[test]
    fn save_per_turn_flags_offsets_match_spec() {
        // formats/saved-gam.md §10
        assert_eq!(SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET, 0x033A);
        assert_eq!(SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN, 16);
        // The bitmap occupies 0x033A..=0x0349
        assert_eq!(
            SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET + SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN - 1,
            0x0349
        );
        assert_eq!(SAVE_ACTIVE_PLAYER_NONE, 0xFF);
    }

    #[test]
    fn save_calendar_offsets_and_bounds_match_spec() {
        // formats/saved-gam.md §5
        assert_eq!(SAVE_YEAR_OFFSET, 0x02CE);
        assert_eq!(SAVE_TIMING_STATUS_TAG_OFFSET, 0x02D4);
        assert_eq!(SAVE_ACTIVE_PLAYER_OFFSET, 0x02D5);
        assert_eq!(SAVE_TRANSPORT_MARKER_OFFSET, 0x02D6);
        assert_eq!(SAVE_MONTH_OFFSET, 0x02D7);
        assert_eq!(SAVE_DAY_OFFSET, 0x02D8);
        assert_eq!(SAVE_HOUR_OFFSET, 0x02D9);
        assert_eq!(SAVE_SAVED_HOUR_SNAPSHOT_OFFSET, 0x02DA);
        assert_eq!(SAVE_MINUTE_OFFSET, 0x02DB);
        assert_eq!(SAVE_COMBAT_ROUND_COUNTER_OFFSET, 0x02DC);
        assert_eq!(SAVE_PER_TURN_STATE_OFFSET, 0x02DD);
        assert_eq!(SAVE_AMPM_DISPLAY_OFFSET, 0x02DE);
        // Bounds
        assert_eq!(SAVE_MONTH_MIN, 1);
        assert_eq!(SAVE_MONTH_MAX, 13);
        assert_eq!(SAVE_DAY_MIN, 1);
        assert_eq!(SAVE_DAY_MAX, 28);
        assert_eq!(SAVE_HOUR_MAX, 23);
        assert_eq!(SAVE_MINUTE_MAX, 59);
        // Calendar bytes are contiguous (Month..Hour..Snapshot..Minute..Round..State..AMPM)
        assert_eq!(SAVE_DAY_OFFSET, SAVE_MONTH_OFFSET + 1);
        assert_eq!(SAVE_HOUR_OFFSET, SAVE_DAY_OFFSET + 1);
        assert_eq!(SAVE_SAVED_HOUR_SNAPSHOT_OFFSET, SAVE_HOUR_OFFSET + 1);
        assert_eq!(SAVE_MINUTE_OFFSET, SAVE_SAVED_HOUR_SNAPSHOT_OFFSET + 1);
    }

    #[test]
    fn save_inventory_and_location_offsets_match_spec() {
        // formats/saved-gam.md §6,§7
        assert_eq!(SAVE_FOOD_OFFSET, 0x0202);
        assert_eq!(SAVE_GOLD_OFFSET, 0x0204);
        assert_eq!(SAVE_KEYS_OFFSET, 0x0206);
        assert_eq!(SAVE_GEMS_OFFSET, 0x0207);
        assert_eq!(SAVE_TORCHES_OFFSET, 0x0208);
        assert_eq!(SAVE_GRAPPLE_OFFSET, 0x0209);
        assert_eq!(SAVE_EQUIPMENT_INVENTORY_OFFSET, 0x021A);
        assert_eq!(SAVE_SPELL_CHARGE_BLOCK_OFFSET, 0x024A);
        assert_eq!(SAVE_SCROLL_COUNTERS_OFFSET, 0x027A);
        assert_eq!(SAVE_POTION_COUNTERS_OFFSET, 0x0282);
        assert_eq!(SAVE_REAGENTS_OFFSET, 0x02AA);
        assert_eq!(SAVE_WIND_OFFSET, 0x02EC);
        assert_eq!(SAVE_SAVED_SCENE_SCRATCH_OFFSET, 0x02EE);
        assert_eq!(SAVE_PARTY_Z_OFFSET, 0x02EF);
        assert_eq!(SAVE_PARTY_X_OFFSET, 0x02F0);
        assert_eq!(SAVE_PARTY_Y_OFFSET, 0x02F1);
        assert_eq!(SAVE_PARTY_Z_NO_ACTIVE_MAP, 0xFF);
        // Cross-check: equipment block is 48 bytes wide
        assert_eq!(
            SAVE_SPELL_CHARGE_BLOCK_OFFSET - SAVE_EQUIPMENT_INVENTORY_OFFSET,
            EQUIPMENT_STOCK_BAND_LEN
        );
        // Spell-charge block is 48 bytes wide
        assert_eq!(
            SAVE_SCROLL_COUNTERS_OFFSET - SAVE_SPELL_CHARGE_BLOCK_OFFSET,
            SPELL_CHARGE_BAND_LEN
        );
        // Location-cluster contiguity around Z/X/Y
        assert_eq!(SAVE_PARTY_X_OFFSET, SAVE_PARTY_Z_OFFSET + 1);
        assert_eq!(SAVE_PARTY_Y_OFFSET, SAVE_PARTY_X_OFFSET + 1);
    }

    #[test]
    fn ool_filenames_and_plane_table_layout_match_spec() {
        // formats/ool.md §2,§3
        assert_eq!(SAVED_OOL_FILENAME, "SAVED.OOL");
        assert_eq!(BRIT_OOL_FILENAME, "BRIT.OOL");
        assert_eq!(UNDER_OOL_FILENAME, "UNDER.OOL");
        assert_eq!(INIT_OOL_FILENAME, "INIT.OOL");
        assert_eq!(OOL_PLANE_RECORD_COUNT, 32);
        assert_eq!(OOL_PLANE_RECORD_LEN, 8);
        assert_eq!(OOL_PLANE_TABLE_LEN, 256);
        // SAVED.OOL holds two planes back-to-back
        assert_eq!(OOL_PLANE_TABLE_LEN * 2, SAVED_OOL_FILE_LEN);
        // BRIT.OOL / UNDER.OOL / INIT.OOL each hold one plane
        assert_eq!(OOL_PLANE_TABLE_LEN, PER_PLANE_OOL_FILE_LEN);
        assert_eq!(OOL_PLANE_TABLE_LEN, INIT_OOL_FILE_LEN);
    }

    #[test]
    fn save_load_disk_swap_and_double_write_predicates() {
        // save-load.md §4.2 step 6: enter the underworld disk-swap loop
        // only when overworld scene + non-zero Z.
        assert_eq!(SAVE_SCENE_OVERWORLD, 0);
        assert!(save_load_needs_underworld_disk_swap(0, 1));
        assert!(save_load_needs_underworld_disk_swap(0, 255));
        assert!(!save_load_needs_underworld_disk_swap(0, 0));
        assert!(!save_load_needs_underworld_disk_swap(13, 1));
        assert!(!save_load_needs_underworld_disk_swap(33, 0));

        // save-load.md §5.2 step 5: defensive UNDER.OOL re-flush.
        assert!(save_flow_double_writes_underworld(0));
        assert!(!save_flow_double_writes_underworld(1));
        assert!(save_flow_double_writes_underworld(2));

        // save-load.md §3.1: file lengths and Z sentinel.
        assert_eq!(SAVED_OOL_FILE_LEN, 512);
        assert_eq!(PER_PLANE_OOL_FILE_LEN, 256);
        assert_eq!(INIT_OOL_FILE_LEN, 256);
        assert_eq!(OOL_NO_Z_SENTINEL, 0xFF);
    }

    #[test]
    fn input_direction_codes_match_spec_table() {
        // input.md §5
        assert_eq!(
            input_code_direction(0xD3),
            Some(InputDirection::Northwest)
        );
        assert_eq!(
            input_code_direction(0xD4),
            Some(InputDirection::Southwest)
        );
        assert_eq!(
            input_code_direction(0xD5),
            Some(InputDirection::Northeast)
        );
        assert_eq!(
            input_code_direction(0xD6),
            Some(InputDirection::Southeast)
        );
        assert_eq!(input_code_direction(0xFB), Some(InputDirection::West));
        assert_eq!(input_code_direction(0xFC), Some(InputDirection::East));
        assert_eq!(input_code_direction(0xFD), Some(InputDirection::North));
        assert_eq!(input_code_direction(0xFE), Some(InputDirection::South));
        // Non-direction bytes
        assert_eq!(input_code_direction(b'A'), None);
        assert_eq!(input_code_direction(0x00), None);
        assert_eq!(input_code_direction(0xFF), None);
        // Cardinal predicate
        assert!(InputDirection::North.is_cardinal());
        assert!(InputDirection::South.is_cardinal());
        assert!(InputDirection::East.is_cardinal());
        assert!(InputDirection::West.is_cardinal());
        assert!(!InputDirection::Northwest.is_cardinal());
        assert!(!InputDirection::Southeast.is_cardinal());

        // input.md §6 case fold
        assert_eq!(input_case_fold(b'a'), b'A');
        assert_eq!(input_case_fold(b'z'), b'Z');
        assert_eq!(input_case_fold(b'A'), b'A');
        assert_eq!(input_case_fold(b'0'), b'0');
        assert_eq!(input_case_fold(0xFC), 0xFC);
    }

    #[test]
    fn tlk_file_layout_constants_match_spec() {
        // formats/tlk.md §4-§8
        assert_eq!(TLK_HEADER_ENTRY_LEN, 4);
        assert_eq!(TLK_SENTINEL_NPC_ID, 0x0001);
        assert_eq!(TLK_HEADER_FIXED_READ, 512);
        assert_eq!(TLK_BLOB_FIXED_WINDOW, 1024);
        assert_eq!(TLK_TEXT_XOR_MASK, 0x80);
        // Apply the XOR mask to a printable byte to recover plain
        // ASCII per §8.
        assert_eq!(b'A' ^ TLK_TEXT_XOR_MASK, 0xC1);
        assert_eq!(b' ' ^ TLK_TEXT_XOR_MASK, 0xA0);
    }

    #[test]
    fn reserved_keyword_effect_matches_spec_words() {
        // conversation.md §6
        assert_eq!(TLK_INPUT_MAX_LEN, 15);
        assert_eq!(
            reserved_keyword_effect(b"NAME"),
            Some(ReservedKeywordEffect::NameEntry)
        );
        assert_eq!(
            reserved_keyword_effect(b"JOB"),
            Some(ReservedKeywordEffect::JobEntry)
        );
        assert_eq!(
            reserved_keyword_effect(b"WORK"),
            Some(ReservedKeywordEffect::JobEntry)
        );
        assert_eq!(
            reserved_keyword_effect(b"BYE"),
            Some(ReservedKeywordEffect::ByePath)
        );
        assert_eq!(
            reserved_keyword_effect(b"THANK"),
            Some(ReservedKeywordEffect::ByePath)
        );
        // JOIN and WHO ART THOU are not engine-reserved.
        assert_eq!(reserved_keyword_effect(b"JOIN"), None);
        assert_eq!(reserved_keyword_effect(b"WHO ART THOU"), None);
        // Case sensitivity: caller is responsible for the upper-case fold.
        assert_eq!(reserved_keyword_effect(b"name"), None);
    }

    #[test]
    fn tlk_keyword_match_is_space_boundary_and_bit7_strip() {
        // conversation.md §6
        // Exact match
        assert!(tlk_keyword_matches(b"GRAN", b"GRAN"));
        // Space-boundary match
        assert!(tlk_keyword_matches(b"GRAN", b"GRAN PA"));
        // Not a substring/prefix match without a space boundary
        assert!(!tlk_keyword_matches(b"GRAN", b"GRANDPA"));
        // Bit-7 strip on the keyword side (high-bit obfuscated)
        let obfuscated = [b'G' | 0x80, b'R' | 0x80, b'A' | 0x80, b'N' | 0x80];
        assert!(tlk_keyword_matches(&obfuscated, b"GRAN"));
        // Case insensitive
        assert!(tlk_keyword_matches(b"NAME", b"name"));
        assert!(tlk_keyword_matches(b"name", b"NAME"));
        // Empty keyword never matches
        assert!(!tlk_keyword_matches(b"", b"NAME"));
        // Input shorter than keyword
        assert!(!tlk_keyword_matches(b"NAMEE", b"NAME"));
    }

    #[test]
    fn npc_shop_trigger_classifies_per_spec_table() {
        // formats/npc.md §7
        assert_eq!(
            npc_shop_trigger(0x81),
            Some(NpcShopTrigger::WeaponsmithOrArmourer)
        );
        assert_eq!(npc_shop_trigger(0x82), Some(NpcShopTrigger::TavernOrSage));
        assert_eq!(npc_shop_trigger(0x83), Some(NpcShopTrigger::HorseTrader));
        assert_eq!(
            npc_shop_trigger(0x84),
            Some(NpcShopTrigger::ShipwrightOrBroker)
        );
        assert_eq!(npc_shop_trigger(0x85), Some(NpcShopTrigger::Herbalist));
        assert_eq!(npc_shop_trigger(0x86), Some(NpcShopTrigger::Guild));
        assert_eq!(
            npc_shop_trigger(0x87),
            Some(NpcShopTrigger::HealerOrSanctum)
        );
        assert_eq!(npc_shop_trigger(0x88), Some(NpcShopTrigger::Innkeeper));
        // Outside the shop range returns None — caller routes to TLK
        // blob lookup using ordinary npc_id rules.
        assert_eq!(npc_shop_trigger(0), None);
        assert_eq!(npc_shop_trigger(1), None);
        assert_eq!(npc_shop_trigger(0x80), None);
        assert_eq!(npc_shop_trigger(0x89), None);
        assert_eq!(npc_shop_trigger(0xFF), None);
    }

    #[test]
    fn npc_ai_behavior_classifies_per_spec_table() {
        // formats/npc.md §5.3
        assert_eq!(npc_ai_behavior(0), Some(NpcAiBehavior::Stationary));
        assert_eq!(npc_ai_behavior(1), Some(NpcAiBehavior::BoundedWander));
        assert_eq!(npc_ai_behavior(2), Some(NpcAiBehavior::UnboundedWander));
        assert_eq!(
            npc_ai_behavior(3),
            Some(NpcAiBehavior::FollowAtDistance)
        );
        assert_eq!(
            npc_ai_behavior(4),
            Some(NpcAiBehavior::ApproachAndAttack)
        );
        assert_eq!(npc_ai_behavior(5), Some(NpcAiBehavior::ReservedEngage));
        assert_eq!(npc_ai_behavior(6), Some(NpcAiBehavior::GuardOrBlock));
        assert_eq!(npc_ai_behavior(7), Some(NpcAiBehavior::RandomChase));
        // Values above 7 fall through to None (no-action default)
        assert_eq!(npc_ai_behavior(8), None);
        assert_eq!(npc_ai_behavior(255), None);
    }

    #[test]
    fn schedule_floor_state_matches_spec_table() {
        // npc-schedules.md §6
        // both equal -> 2
        assert_eq!(schedule_floor_state(1, 1, 1), NPC_STATE_INPLANE_MOVE);
        // equal/below -> 7 (target floor index > map floor index)
        assert_eq!(schedule_floor_state(1, 2, 1), NPC_STATE_CLIMB_DOWN_OFF_FLOOR);
        // equal/above -> 6
        assert_eq!(schedule_floor_state(1, 0, 1), NPC_STATE_CLIMB_UP_OFF_FLOOR);
        // below/equal -> 5 (npc floor index > map floor index)
        assert_eq!(schedule_floor_state(2, 1, 1), NPC_STATE_ASCEND_TOWARD_TARGET);
        // above/equal -> 4
        assert_eq!(schedule_floor_state(0, 1, 1), NPC_STATE_DESCEND_TOWARD_TARGET);
        // neither/neither -> 8
        assert_eq!(schedule_floor_state(0, 2, 1), NPC_STATE_PARKED_OFF_FLOOR);
        assert_eq!(schedule_floor_state(2, 0, 1), NPC_STATE_PARKED_OFF_FLOOR);
        assert_eq!(schedule_floor_state(2, 3, 1), NPC_STATE_PARKED_OFF_FLOOR);
    }

    #[test]
    fn tlk_scene_branch_mask_does_not_wrap() {
        // quest-flags.md §3
        assert_eq!(tlk_scene_branch_mask(0), 0x0000_0001);
        assert_eq!(tlk_scene_branch_mask(1), 0x0000_0002);
        assert_eq!(tlk_scene_branch_mask(31), 0x8000_0000);
        // No wrap or clamp: bit 32 and beyond produce zero mask.
        assert_eq!(tlk_scene_branch_mask(32), 0);
        assert_eq!(tlk_scene_branch_mask(255), 0);

        // Setter then tester round-trip
        let slot = tlk_scene_branch_set(0, 5);
        assert!(tlk_scene_branch_is_set(slot, 5));
        assert!(!tlk_scene_branch_is_set(slot, 6));
        // Out-of-range setter is a no-op
        assert_eq!(tlk_scene_branch_set(slot, 32), slot);
        assert!(!tlk_scene_branch_is_set(slot, 32));
    }

    #[test]
    fn conversation_letter_action_table_matches_spec() {
        // quest-flags.md §4
        assert_eq!(
            conversation_letter_action(b'A'),
            Some(ConversationLetterAction::GrantFood)
        );
        assert_eq!(
            conversation_letter_action(b'B'),
            Some(ConversationLetterAction::GrantGold)
        );
        assert_eq!(
            conversation_letter_action(b'C'),
            Some(ConversationLetterAction::GrantKeys)
        );
        assert_eq!(
            conversation_letter_action(b'D'),
            Some(ConversationLetterAction::GrantGems)
        );
        assert_eq!(
            conversation_letter_action(b'E'),
            Some(ConversationLetterAction::GrantTorches)
        );
        assert_eq!(
            conversation_letter_action(b'F'),
            Some(ConversationLetterAction::SetGrappleGate)
        );
        assert_eq!(
            conversation_letter_action(b'G'),
            Some(ConversationLetterAction::GrantMagicCarpet)
        );
        assert_eq!(
            conversation_letter_action(b'H'),
            Some(ConversationLetterAction::SetSextant)
        );
        assert_eq!(
            conversation_letter_action(b'I'),
            Some(ConversationLetterAction::SetSpyglass)
        );
        assert_eq!(
            conversation_letter_action(b'J'),
            Some(ConversationLetterAction::SetBlackBadge)
        );
        assert_eq!(
            conversation_letter_action(b'K'),
            Some(ConversationLetterAction::GrantSkullKeys)
        );
        assert_eq!(conversation_letter_action(b'L'), None);
        assert_eq!(conversation_letter_action(b'a'), None);
        assert_eq!(conversation_letter_action(0), None);
    }

    #[test]
    fn visibility_markers_classify_per_spec() {
        // visibility.md §2
        assert_eq!(VIEWPORT_SIDE, 11);
        assert_eq!(VIEWPORT_ROW_STRIDE, 32);
        assert_eq!(TERRAIN_BAND_ROW_STRIDE, 16);
        assert_eq!(VIEWPORT_PLAYER_ROW, 5);
        assert_eq!(VIEWPORT_PLAYER_COL, 5);
        assert_eq!(visibility_marker(0xFF), VisibilityMarker::Hidden);
        assert_eq!(visibility_marker(0x00), VisibilityMarker::UseCompanion);
        assert_eq!(visibility_marker(0xDD), VisibilityMarker::ClearVisible);
        assert_eq!(visibility_marker(0x1C), VisibilityMarker::DimPeriphery);
        assert_eq!(
            visibility_marker(0x87),
            VisibilityMarker::AlreadyRendered
        );
        assert_eq!(
            visibility_marker(0x42),
            VisibilityMarker::DirectTile(0x42)
        );

        // visibility.md §3 light-radius branch (signed)
        assert_eq!(light_radius_branch(0), LightRadiusBranch::PitchDark);
        assert_eq!(light_radius_branch(1), LightRadiusBranch::Carve(1));
        assert_eq!(light_radius_branch(50), LightRadiusBranch::Carve(50));
        assert_eq!(light_radius_branch(127), LightRadiusBranch::Carve(127));
        assert_eq!(light_radius_branch(128), LightRadiusBranch::DebugFullFill);
        assert_eq!(light_radius_branch(255), LightRadiusBranch::DebugFullFill);
    }

    #[test]
    fn combat_arena_metadata_slices_match_spec() {
        // formats/cbt.md §2 + §5
        // File lengths
        assert_eq!(BRIT_CBT_FILE_LEN, 5_632);
        assert_eq!(DUNGEON_CBT_FILE_LEN, 39_424);
        // Per-arena setup tables A and B on row 3
        assert_eq!(CBT_SETUP_TABLE_ROW, 3);
        assert!(CBT_SETUP_TABLE_A_COLUMNS.contains(&11));
        assert!(CBT_SETUP_TABLE_A_COLUMNS.contains(&16));
        assert!(!CBT_SETUP_TABLE_A_COLUMNS.contains(&17));
        assert!(CBT_SETUP_TABLE_B_COLUMNS.contains(&17));
        assert!(CBT_SETUP_TABLE_B_COLUMNS.contains(&22));
        assert!(!CBT_SETUP_TABLE_B_COLUMNS.contains(&23));
        // Placement-slot rows
        assert_eq!(CBT_PLACEMENT_X_ROW, 6);
        assert_eq!(CBT_PLACEMENT_Y_ROW, 7);
        assert!(CBT_PLACEMENT_COLUMNS.contains(&11));
        assert!(CBT_PLACEMENT_COLUMNS.contains(&26));
        assert!(!CBT_PLACEMENT_COLUMNS.contains(&27));
        assert_eq!(CBT_PLACEMENT_SLOT_COUNT, 16);
    }

    #[test]
    fn active_object_field_offsets_match_spec() {
        // active-objects.md §3
        assert_eq!(ACTIVE_OBJECT_FIELD_TYPE, 0);
        assert_eq!(ACTIVE_OBJECT_FIELD_TILE, 1);
        assert_eq!(ACTIVE_OBJECT_FIELD_X, 2);
        assert_eq!(ACTIVE_OBJECT_FIELD_Y, 3);
        assert_eq!(ACTIVE_OBJECT_FIELD_Z, 4);
        assert_eq!(ACTIVE_OBJECT_FIELD_DEP1, 5);
        assert_eq!(ACTIVE_OBJECT_FIELD_PHASE, 6);
        assert_eq!(ACTIVE_OBJECT_FIELD_DEP3, 7);
        // Cross-check: every field offset is within the published
        // record length.
        for offset in [
            ACTIVE_OBJECT_FIELD_TYPE,
            ACTIVE_OBJECT_FIELD_TILE,
            ACTIVE_OBJECT_FIELD_X,
            ACTIVE_OBJECT_FIELD_Y,
            ACTIVE_OBJECT_FIELD_Z,
            ACTIVE_OBJECT_FIELD_DEP1,
            ACTIVE_OBJECT_FIELD_PHASE,
            ACTIVE_OBJECT_FIELD_DEP3,
        ] {
            assert!(offset < OOL_RECORD_LEN);
        }
    }

    #[test]
    fn active_object_should_prune_matches_spec_radius() {
        // active-objects.md §10
        assert_eq!(ACTIVE_OBJECT_PRUNE_RADIUS, 32);
        assert_eq!(ACTIVE_OBJECT_SAVE_BYTES, 256);
        // Inside the radius: keep
        assert!(!active_object_should_prune(100, 100, 100, 100));
        assert!(!active_object_should_prune(132, 100, 100, 100));
        assert!(!active_object_should_prune(100, 68, 100, 100));
        // Just outside: prune
        assert!(active_object_should_prune(133, 100, 100, 100));
        assert!(active_object_should_prune(67, 100, 100, 100));
        assert!(active_object_should_prune(100, 133, 100, 100));
        assert!(active_object_should_prune(100, 67, 100, 100));
        // Either-axis boundary
        assert!(active_object_should_prune(200, 100, 100, 100));
        assert!(active_object_should_prune(100, 0, 100, 100));
    }

    #[test]
    fn active_object_eviction_phase_matches_spec_cascade() {
        // active-objects.md §4
        // Empty slot is always phase 1.
        assert_eq!(active_object_eviction_phase(0x00, true), Some(1));
        assert_eq!(active_object_eviction_phase(0x00, false), Some(1));

        // 0x01..=0x0F low-priority scenery
        assert_eq!(active_object_eviction_phase(0x01, true), Some(2));
        assert_eq!(active_object_eviction_phase(0x0F, true), Some(2));
        assert_eq!(active_object_eviction_phase(0x01, false), Some(6));

        // 0x80..=0xFF monsters/dynamic actors (except 0xB5)
        assert_eq!(active_object_eviction_phase(0x80, true), Some(3));
        assert_eq!(active_object_eviction_phase(0xFF, true), Some(3));
        assert_eq!(active_object_eviction_phase(0x80, false), Some(7));
        assert_eq!(active_object_eviction_phase(0xB5, true), None);
        assert_eq!(active_object_eviction_phase(0xB5, false), None);

        // 0x10..=0x11 door/fixture-like
        assert_eq!(active_object_eviction_phase(0x10, true), Some(4));
        assert_eq!(active_object_eviction_phase(0x11, true), Some(4));
        assert_eq!(active_object_eviction_phase(0x10, false), Some(8));

        // 0x30..=0x7F items/chests
        assert_eq!(active_object_eviction_phase(0x30, true), Some(5));
        assert_eq!(active_object_eviction_phase(0x7F, true), Some(5));
        assert_eq!(active_object_eviction_phase(0x30, false), Some(9));

        // 0x12..=0x1F NPC/person ranges and 0x20..=0x2F vehicle ranges
        // are protected from off-screen phases but eligible for the
        // last-resort phase 10.
        assert_eq!(active_object_eviction_phase(0x12, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x1F, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x20, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x2F, true), Some(10));
        assert_eq!(active_object_eviction_phase(0x20, false), Some(10));
    }

    #[test]
    fn chargen_questionnaire_always_floors_strength_to_twenty() {
        // chargen.md §7: max STR contribution is 2 per question and there
        // are 7 questions, so the floor always fires.
        assert_eq!(CHARGEN_STR_FLOOR, 20);
        assert_eq!(CHARGEN_STARTING_PARTY_SIZE, 3);

        // Empty winners list: STR should still be floored to 20.
        let stats = chargen_stats_from_winners(&[]);
        assert_eq!(stats.strength, CHARGEN_STR_FLOOR);
        assert_eq!(stats.dexterity, 0);
        assert_eq!(stats.intelligence, 0);

        // Worst-case STR contribution (any seven Spirituality wins):
        // chargen_virtue_stat_delta(Spirituality) is INT-only, so STR
        // remains 0 before the floor.
        let all_spirituality = vec![ShrineVirtue::Spirituality; 7];
        let stats = chargen_stats_from_winners(&all_spirituality);
        assert_eq!(stats.strength, CHARGEN_STR_FLOOR);

        // Best-case STR contribution: any seven full-STR virtues should
        // still floor the result, since 7*max delta < 20 only if delta<3.
        // Either way, the result must be >= floor.
        for v in [
            ShrineVirtue::Honesty,
            ShrineVirtue::Compassion,
            ShrineVirtue::Valor,
            ShrineVirtue::Justice,
            ShrineVirtue::Sacrifice,
            ShrineVirtue::Honor,
            ShrineVirtue::Spirituality,
            ShrineVirtue::Humility,
        ] {
            let stats = chargen_stats_from_winners(&vec![v; 7]);
            assert!(stats.strength >= CHARGEN_STR_FLOOR);
        }
    }

    #[test]
    fn trap_effect_classification_matches_spec_table() {
        // traps.md §3
        assert_eq!(trap_effect_for_id(0), Some(TrapEffect::Acid));
        assert_eq!(trap_effect_for_id(1), Some(TrapEffect::Poison));
        assert_eq!(trap_effect_for_id(2), Some(TrapEffect::Bomb));
        assert_eq!(trap_effect_for_id(3), Some(TrapEffect::Gas));
        assert_eq!(trap_effect_for_id(4), None);
        assert_eq!(trap_effect_for_id(255), None);

        assert_eq!(trap_effect_damage_max(TrapEffect::Acid), Some(30));
        assert_eq!(trap_effect_damage_max(TrapEffect::Bomb), Some(8));
        assert_eq!(trap_effect_damage_max(TrapEffect::Poison), None);
        assert_eq!(trap_effect_damage_max(TrapEffect::Gas), None);

        assert!(!trap_effect_targets_whole_party(TrapEffect::Acid));
        assert!(!trap_effect_targets_whole_party(TrapEffect::Poison));
        assert!(trap_effect_targets_whole_party(TrapEffect::Bomb));
        assert!(trap_effect_targets_whole_party(TrapEffect::Gas));

        // The non-combat lookup table publishes 3/2/2/1 weights for the
        // four effect ids.
        let mut counts = [0u32; 4];
        for index in 0..8u8 {
            let id = shared_trap_effect_id_from_index(index, false);
            counts[usize::from(id)] += 1;
        }
        assert_eq!(counts, [3, 2, 2, 1]);

        // In combat the resolver picks only ids 0 and 1.
        for index in 0..8u8 {
            let id = shared_trap_effect_id_from_index(index, true);
            assert!(id == 0 || id == 1);
        }
    }

    #[test]
    fn inventory_add_class_covers_spec_table() {
        // containers.md §8
        assert_eq!(
            inventory_add_class(0x01),
            InventoryAddClass::MustOpenFirst
        );
        assert_eq!(inventory_add_class(0x02), InventoryAddClass::Gold);
        assert_eq!(inventory_add_class(0x03), InventoryAddClass::Potion);
        assert_eq!(
            inventory_add_class(0x04),
            InventoryAddClass::ScrollOrPlans
        );
        // Equipment rows
        for c in [0x05u8, 0x06, 0x09, 0x0A, 0x0B, 0x0C] {
            assert_eq!(inventory_add_class(c), InventoryAddClass::Equipment);
        }
        assert_eq!(inventory_add_class(0x07), InventoryAddClass::Key);
        assert_eq!(inventory_add_class(0x08), InventoryAddClass::Gem);
        assert_eq!(inventory_add_class(0x0D), InventoryAddClass::Torch);
        assert_eq!(
            inventory_add_class(0x0E),
            InventoryAddClass::SandalwoodBox
        );
        assert_eq!(inventory_add_class(0x0F), InventoryAddClass::Food);
        assert_eq!(inventory_add_class(0x19), InventoryAddClass::Moonstone);
        assert_eq!(inventory_add_class(0x1B), InventoryAddClass::MagicCarpet);
        assert_eq!(
            inventory_add_class(0xB4),
            InventoryAddClass::ShadowlordShard
        );
        assert_eq!(
            inventory_add_class(0xB5),
            InventoryAddClass::CrownOfLordBritish
        );
        assert_eq!(
            inventory_add_class(0xB6),
            InventoryAddClass::SceptreOfLordBritish
        );
        assert_eq!(
            inventory_add_class(0xB7),
            InventoryAddClass::AmuletOfLordBritish
        );
        // Unknown class codes refuse
        assert_eq!(inventory_add_class(0x00), InventoryAddClass::NothingToGet);
        assert_eq!(inventory_add_class(0x10), InventoryAddClass::NothingToGet);
        assert_eq!(inventory_add_class(0x20), InventoryAddClass::NothingToGet);
        assert_eq!(inventory_add_class(0xFF), InventoryAddClass::NothingToGet);
        // Equipment-grant quantities
        assert_eq!(equipment_grant_quantity(0x05), 5);
        assert_eq!(equipment_grant_quantity(0x06), 5);
        assert_eq!(equipment_grant_quantity(0x09), 1);
        assert_eq!(equipment_grant_quantity(0x0C), 1);
    }

    #[test]
    fn dungeon_chest_rows_match_spec_table() {
        // containers.md §6
        assert_eq!(DUNGEON_CHEST_ROWS.len(), 7);
        let expected = [
            (2u8, DungeonChestReward::Food),
            (4, DungeonChestReward::Gold),
            (5, DungeonChestReward::Keys),
            (10, DungeonChestReward::Gems),
            (20, DungeonChestReward::Torches),
            (25, DungeonChestReward::Potion),
            (25, DungeonChestReward::Scroll),
        ];
        for (i, row) in DUNGEON_CHEST_ROWS.iter().enumerate() {
            assert_eq!(row.gate_threshold, expected[i].0);
            assert_eq!(row.reward, expected[i].1);
        }
        // Per-row gate max: 4*depth + 4
        assert_eq!(dungeon_chest_row_gate_max(0), 4);
        assert_eq!(dungeon_chest_row_gate_max(7), 32);
        // Awarded when threshold <= roll
        let food = DUNGEON_CHEST_ROWS[0];
        assert!(dungeon_chest_row_awarded(food, 2));
        assert!(dungeon_chest_row_awarded(food, 31));
        assert!(!dungeon_chest_row_awarded(food, 1));
        let scroll = DUNGEON_CHEST_ROWS[6];
        assert!(dungeon_chest_row_awarded(scroll, 25));
        assert!(!dungeon_chest_row_awarded(scroll, 24));
    }

    #[test]
    fn table_food_get_directional_rules_match_spec() {
        // containers.md §7
        assert_eq!(table_food_get_resulting_tile(0x9B, 0, -1), Some(0x95));
        assert_eq!(table_food_get_resulting_tile(0x9B, 0, 1), None);
        assert_eq!(table_food_get_resulting_tile(0x9B, -1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x9B, 1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x9B, 1, -1), None);
        assert_eq!(table_food_get_resulting_tile(0x9C, 0, -1), Some(0x9A));
        assert_eq!(table_food_get_resulting_tile(0x9C, 0, 1), Some(0x9B));
        assert_eq!(table_food_get_resulting_tile(0x9C, -1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x9C, 1, 0), None);
        assert_eq!(table_food_get_resulting_tile(0x95, 0, -1), None);
    }

    #[test]
    fn jimmy_helpers_match_spec_formulas() {
        // doors-and-z-transitions.md §3
        // Door pick: class > roll
        assert_eq!(JIMMY_DOOR_DIE_LOW, 1);
        assert_eq!(JIMMY_DOOR_DIE_HIGH, 29);
        assert!(jimmy_door_succeeds(20, 19));
        assert!(!jimmy_door_succeeds(20, 20));
        assert!(!jimmy_door_succeeds(20, 21));
        assert!(jimmy_door_succeeds(29, 1));

        // Object chest: requires high bit; threshold = (diff - class + 30)/2
        assert_eq!(object_chest_jimmy_threshold(0x40, 10), None);
        // diff=0x10=16, class=10 -> (16-10+30)/2 = 18
        assert_eq!(object_chest_jimmy_threshold(0x90, 10), Some(18));
        // diff=20, class=40 -> (20-40+30)/2 = 5
        assert_eq!(object_chest_jimmy_threshold(0x94, 40), Some(5));
        // Negative raw -> 0
        assert_eq!(object_chest_jimmy_threshold(0x81, 100), Some(0));
        assert!(object_chest_jimmy_succeeds(18, 1));
        assert!(object_chest_jimmy_succeeds(18, 18));
        assert!(!object_chest_jimmy_succeeds(18, 19));

        // Dungeon chest: threshold = (2*depth - class + 30)/2
        // depth=4, class=20 -> (8-20+30)/2 = 9
        assert_eq!(dungeon_chest_jimmy_threshold(4, 20), 9);
        // depth=8, class=10 -> (16-10+30)/2 = 18
        assert_eq!(dungeon_chest_jimmy_threshold(8, 10), 18);
        assert!(dungeon_chest_jimmy_succeeds(9, 9));
        assert!(!dungeon_chest_jimmy_succeeds(9, 10));

        assert_eq!(DOOR_AUTO_CLOSE_TURNS, 4);
    }

    #[test]
    fn lighting_helpers_match_spec_table() {
        // lighting.md §4
        assert_eq!(apply_personal_light(2, 0, 0), 2);
        assert_eq!(apply_personal_light(2, 1, 0), TORCH_LIGHT_FLOOR);
        assert_eq!(apply_personal_light(2, 0, 1), LIGHT_SPELL_FLOOR);
        // Torch dominates spell when both nonzero
        assert_eq!(apply_personal_light(2, 5, 5), TORCH_LIGHT_FLOOR);
        // Ambient already brighter than the floor wins
        assert_eq!(apply_personal_light(FULL_DAYLIGHT, 5, 5), FULL_DAYLIGHT);
        assert_eq!(apply_personal_light(20, 1, 0), 20);

        // lighting.md §6
        assert!(dungeon_blackout(0, 0));
        assert!(!dungeon_blackout(1, 0));
        assert!(!dungeon_blackout(0, 1));

        // lighting.md §5
        assert_eq!(decay_light_counter(10, 1), 9);
        assert_eq!(decay_light_counter(10, 2), 8);
        assert_eq!(decay_light_counter(2, 5), 0);
        assert_eq!(decay_light_counter(0, 1), 0);

        // lighting.md §3
        assert!(!ambient_is_sentinel(50));
        assert!(ambient_is_sentinel(51));
        assert!(ambient_is_sentinel(255));

        // lighting.md §8
        assert_eq!(ignite_torch_surface(), 240);
        assert_eq!(ignite_torch_dungeon(0, 112), 112);
        assert_eq!(ignite_torch_dungeon(100, 127), 227);
        assert_eq!(ignite_torch_dungeon(200, 127), 255);
        assert_eq!(LIGHT_SPELL_DURATION, 100);
        assert_eq!(GREAT_LIGHT_SPELL_DURATION, 255);
    }

    #[test]
    fn player_sail_wait_ticks_matches_weather_table() {
        // weather.md §5
        // Calm never releases.
        for heading in [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            assert_eq!(WindState::Calm.player_sail_wait_ticks(heading), None);
        }
        // North wind row: N=2, E=0, S=1, W=0
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::North),
            Some(2)
        );
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::East),
            Some(0)
        );
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::South),
            Some(1)
        );
        assert_eq!(
            WindState::North.player_sail_wait_ticks(Direction::West),
            Some(0)
        );
        // South wind row: N=1, E=0, S=2, W=0
        assert_eq!(
            WindState::South.player_sail_wait_ticks(Direction::North),
            Some(1)
        );
        assert_eq!(
            WindState::South.player_sail_wait_ticks(Direction::South),
            Some(2)
        );
        // East wind row: N=0, E=2, S=0, W=1
        assert_eq!(
            WindState::East.player_sail_wait_ticks(Direction::East),
            Some(2)
        );
        assert_eq!(
            WindState::East.player_sail_wait_ticks(Direction::West),
            Some(1)
        );
        // West wind row: N=0, E=1, S=0, W=2
        assert_eq!(
            WindState::West.player_sail_wait_ticks(Direction::East),
            Some(1)
        );
        assert_eq!(
            WindState::West.player_sail_wait_ticks(Direction::West),
            Some(2)
        );
    }

    #[test]
    fn active_ship_cadence_matches_weather_table() {
        // weather.md §7
        for heading in [
            Direction::North,
            Direction::South,
            Direction::East,
            Direction::West,
        ] {
            assert_eq!(WindState::Calm.active_ship_cadence(heading), None);
        }
        // North-facing frame row
        assert_eq!(
            WindState::North.active_ship_cadence(Direction::North),
            Some((2, 3))
        );
        assert_eq!(
            WindState::South.active_ship_cadence(Direction::North),
            Some((3, 4))
        );
        assert_eq!(
            WindState::East.active_ship_cadence(Direction::North),
            Some((1, 1))
        );
        assert_eq!(
            WindState::West.active_ship_cadence(Direction::North),
            Some((1, 1))
        );
        // East-facing frame row
        assert_eq!(
            WindState::East.active_ship_cadence(Direction::East),
            Some((2, 3))
        );
        assert_eq!(
            WindState::West.active_ship_cadence(Direction::East),
            Some((3, 4))
        );
        // South-facing frame row
        assert_eq!(
            WindState::South.active_ship_cadence(Direction::South),
            Some((2, 3))
        );
        assert_eq!(
            WindState::North.active_ship_cadence(Direction::South),
            Some((3, 4))
        );
        // West-facing frame row
        assert_eq!(
            WindState::West.active_ship_cadence(Direction::West),
            Some((2, 3))
        );
        assert_eq!(
            WindState::East.active_ship_cadence(Direction::West),
            Some((3, 4))
        );
    }

    #[test]
    fn karma_actions_apply_with_spec_clamps() {
        // karma.md §4
        // Completed-shrine offering adds the digit, capped at MAX
        assert_eq!(
            apply_karma_action(50, KarmaAction::CompletedShrineOffering { digit: 9 }),
            59
        );
        assert_eq!(
            apply_karma_action(95, KarmaAction::CompletedShrineOffering { digit: 9 }),
            MORAL_STANDING_MAX
        );
        // Codex turn-in: +3 normal, +6 for Humility
        assert_eq!(
            apply_karma_action(50, KarmaAction::CodexShrineTurnIn { humility: false }),
            53
        );
        assert_eq!(
            apply_karma_action(50, KarmaAction::CodexShrineTurnIn { humility: true }),
            56
        );
        assert_eq!(
            apply_karma_action(98, KarmaAction::CodexShrineTurnIn { humility: true }),
            MORAL_STANDING_MAX
        );
        // Town chest: -2, floored at 0
        assert_eq!(apply_karma_action(50, KarmaAction::TownChestOpened), 48);
        assert_eq!(apply_karma_action(1, KarmaAction::TownChestOpened), 0);
        assert_eq!(apply_karma_action(0, KarmaAction::TownChestOpened), 0);
        // Crop/table food: -1 when nonzero, no-op at 0
        assert_eq!(apply_karma_action(2, KarmaAction::CropOrTableFoodTaken), 1);
        assert_eq!(apply_karma_action(0, KarmaAction::CropOrTableFoodTaken), 0);
        // Town cannon hit: -5, floored at 0
        assert_eq!(apply_karma_action(10, KarmaAction::TownCannonHit), 5);
        assert_eq!(apply_karma_action(3, KarmaAction::TownCannonHit), 0);
        // Helped NPC thank-you: +2, capped
        assert_eq!(apply_karma_action(50, KarmaAction::HelpedNpcThankYou), 52);
        assert_eq!(
            apply_karma_action(98, KarmaAction::HelpedNpcThankYou),
            MORAL_STANDING_MAX
        );
        // Toll milestone: +1, +3 if left party with zero gold
        assert_eq!(
            apply_karma_action(
                50,
                KarmaAction::TollMilestone {
                    left_party_with_zero_gold: false
                }
            ),
            51
        );
        assert_eq!(
            apply_karma_action(
                50,
                KarmaAction::TollMilestone {
                    left_party_with_zero_gold: true
                }
            ),
            53
        );
        assert_eq!(
            apply_karma_action(
                98,
                KarmaAction::TollMilestone {
                    left_party_with_zero_gold: true
                }
            ),
            MORAL_STANDING_MAX
        );
    }

    #[test]
    fn look2_dat_offset_table_layout_matches_spec() {
        // formats/look2-dat.md §2,§3
        assert_eq!(LOOK2_DAT_OFFSET_TABLE_LEN, 1024);
        assert_eq!(LOOK2_DAT_TERRAIN_ENTRIES, 256);
        assert_eq!(LOOK2_DAT_OBJECT_ENTRIES, 256);
        assert_eq!(LOOK2_DAT_OBJECT_DOMAIN_BASE, 0x200);
        assert_eq!(
            (LOOK2_DAT_TERRAIN_ENTRIES + LOOK2_DAT_OBJECT_ENTRIES) * 2,
            LOOK2_DAT_OFFSET_TABLE_LEN
        );
        // Terrain entries
        assert_eq!(look2_terrain_table_offset(0), 0);
        assert_eq!(look2_terrain_table_offset(1), 2);
        assert_eq!(look2_terrain_table_offset(255), 510);
        // Object entries are in the upper half
        assert_eq!(look2_object_table_offset(0), 0x200);
        assert_eq!(look2_object_table_offset(1), 0x202);
        assert_eq!(look2_object_table_offset(255), 0x3FE);
        // Verify the two domains are disjoint
        assert!(look2_object_table_offset(0) > look2_terrain_table_offset(255));
    }

    #[test]
    fn sleep_ambush_monster_table_matches_spec() {
        // encounters.md §6
        assert_eq!(sleep_ambush_monster(0), Some(SleepAmbushMonster::GiantRat));
        assert_eq!(sleep_ambush_monster(1), Some(SleepAmbushMonster::GiantRat));
        assert_eq!(sleep_ambush_monster(2), Some(SleepAmbushMonster::Troll));
        assert_eq!(sleep_ambush_monster(3), Some(SleepAmbushMonster::Bat));
        assert_eq!(sleep_ambush_monster(4), Some(SleepAmbushMonster::Slime));
        assert_eq!(sleep_ambush_monster(5), Some(SleepAmbushMonster::GiantSpider));
        assert_eq!(sleep_ambush_monster(6), Some(SleepAmbushMonster::Gremlin));
        assert_eq!(sleep_ambush_monster(7), Some(SleepAmbushMonster::Headless));
        assert_eq!(sleep_ambush_monster(8), None);
        assert_eq!(sleep_ambush_monster(255), None);

        // Effective Giant Rat share = 2/8
        let rat_rows = (0..8u8)
            .filter(|r| sleep_ambush_monster(*r) == Some(SleepAmbushMonster::GiantRat))
            .count();
        assert_eq!(rat_rows, 2);

        // Sleep-ambush interruption: only outcome 0 in 0..64 interrupts.
        assert_eq!(SLEEP_AMBUSH_INTERRUPT_DENOMINATOR, 64);
        assert!(sleep_ambush_rest_interrupted(0));
        for roll in 1..SLEEP_AMBUSH_INTERRUPT_DENOMINATOR {
            assert!(!sleep_ambush_rest_interrupted(roll));
        }
    }

    #[test]
    fn random_encounter_threshold_matches_spec_table() {
        // encounters.md §3
        // Underworld: always 3
        for hour in 0..24u8 {
            assert_eq!(random_encounter_threshold(true, 0x05, hour), 3);
            assert_eq!(random_encounter_threshold(true, 0x20, hour), 3);
        }
        // Surface no-encounter band 0x20..=0x26
        assert_eq!(random_encounter_threshold(false, 0x20, 12), 0);
        assert_eq!(random_encounter_threshold(false, 0x26, 18), 0);
        assert_eq!(random_encounter_threshold(false, 0x20, 0), 3);
        assert_eq!(random_encounter_threshold(false, 0x26, 4), 3);
        // Surface tile 0x04 or wilderness 0x09..=0x0F
        assert_eq!(random_encounter_threshold(false, 0x04, 12), 2);
        assert_eq!(random_encounter_threshold(false, 0x09, 12), 2);
        assert_eq!(random_encounter_threshold(false, 0x0F, 12), 2);
        assert_eq!(random_encounter_threshold(false, 0x04, 0), 5);
        assert_eq!(random_encounter_threshold(false, 0x09, 4), 5);
        // Any other surface tile
        assert_eq!(random_encounter_threshold(false, 0x05, 12), 1);
        assert_eq!(random_encounter_threshold(false, 0x06, 18), 1);
        assert_eq!(random_encounter_threshold(false, 0x05, 0), 4);
        assert_eq!(random_encounter_threshold(false, 0x10, 4), 4);
    }

    #[test]
    fn ship_transport_marker_predicates_match_published_ranges() {
        // vehicles.md §6: hoisted 0x20..=0x23, furled 0x24..=0x27.
        for byte in 0x20..=0x23u8 {
            assert!(is_ship_transport_marker(byte));
            assert!(is_ship_transport_hoisted(byte));
            assert!(!is_ship_transport_furled(byte));
        }
        for byte in 0x24..=0x27u8 {
            assert!(is_ship_transport_marker(byte));
            assert!(!is_ship_transport_hoisted(byte));
            assert!(is_ship_transport_furled(byte));
        }
        for byte in [0x1F, 0x28, 0x00, 0xFFu8] {
            assert!(!is_ship_transport_marker(byte));
        }
    }

    #[test]
    fn ship_transport_heading_index_decodes_low_two_bits() {
        // vehicles.md §6: low two bits encode N=0, E=1, S=2, W=3 in both
        // hoisted and furled ranges.
        assert_eq!(ship_transport_heading_index(0x20), Some(0));
        assert_eq!(ship_transport_heading_index(0x21), Some(1));
        assert_eq!(ship_transport_heading_index(0x22), Some(2));
        assert_eq!(ship_transport_heading_index(0x23), Some(3));
        assert_eq!(ship_transport_heading_index(0x24), Some(0));
        assert_eq!(ship_transport_heading_index(0x27), Some(3));
        assert_eq!(ship_transport_heading_index(0x14), None);
    }

    #[test]
    fn active_object_slot_partition_constants_match_section_four() {
        // active-objects.md §4: slot 0 player; ordinary 1..=23; reserved
        // 24..=31; 0xB5 is the universally protected byte-0; off-screen
        // test radius is five cells.
        assert_eq!(ACTIVE_OBJECT_PLAYER_SLOT, 0);
        assert_eq!(ACTIVE_OBJECT_ORDINARY_FIRST, 1);
        assert_eq!(ACTIVE_OBJECT_ORDINARY_LAST, 23);
        assert_eq!(ACTIVE_OBJECT_RESERVED_FIRST, 24);
        assert_eq!(ACTIVE_OBJECT_RESERVED_LAST, 31);
        assert_eq!(ACTIVE_OBJECT_PROTECTED_TYPE_BYTE, 0xB5);
        assert_eq!(ACTIVE_OBJECT_OFF_SCREEN_RADIUS, 5);
    }

    #[test]
    fn tlk_introducer_argument_widths_match_section_seven_six() {
        // conversation.md §7.6: 0x85 GOLD-PAYMENT takes 3 bytes, 0x86
        // ACTION-DISPATCH and 0x8C IF-ELSE take 1 byte, 0xFE IF-ELSE-ALT
        // takes 2 bytes; other codes take none.
        assert_eq!(tlk_introducer_argument_count(TLK_CODE_GOLD_PAYMENT), Some(3));
        assert_eq!(
            tlk_introducer_argument_count(TLK_CODE_ACTION_DISPATCH),
            Some(1)
        );
        assert_eq!(tlk_introducer_argument_count(TLK_CODE_IF_ELSE), Some(1));
        assert_eq!(tlk_introducer_argument_count(TLK_CODE_IF_ELSE_ALT), Some(2));
        for code in [
            TLK_CODE_PRINT_AVATAR_NAME,
            TLK_CODE_END_STREAM,
            TLK_CODE_PAUSE,
            TLK_CODE_WAIT_KEY,
            TLK_CODE_CURSE_CHECK,
            TLK_CODE_PROTECT_RUN,
            TLK_CODE_END_OF_RESPONSE,
        ] {
            assert_eq!(tlk_introducer_argument_count(code), None);
        }
    }

    #[test]
    fn tile_class_partitions_byte_range_per_catalog_section_three() {
        // catalogs/tile-catalog.md §3 coarse class groupings.
        assert_eq!(coarse_tile_class(0x00), TileClass::Sentinel);
        for tile in TILE_WATER_FIRST..=TILE_WATER_LAST {
            assert_eq!(coarse_tile_class(tile), TileClass::Water);
        }
        assert_eq!(coarse_tile_class(0x05), TileClass::Terrain);
        assert_eq!(coarse_tile_class(0x0F), TileClass::Terrain);
        assert_eq!(coarse_tile_class(0x10), TileClass::Path);
        assert_eq!(coarse_tile_class(0x17), TileClass::Path);
        assert_eq!(coarse_tile_class(0x18), TileClass::Wall);
        assert_eq!(coarse_tile_class(0x3F), TileClass::Wall);
        assert_eq!(coarse_tile_class(0x40), TileClass::Furniture);
        assert_eq!(coarse_tile_class(0x5F), TileClass::Furniture);
        assert_eq!(coarse_tile_class(0x60), TileClass::Door);
        assert_eq!(coarse_tile_class(0x67), TileClass::Door);
        assert_eq!(coarse_tile_class(0x68), TileClass::Decoration);
        assert_eq!(coarse_tile_class(0x6F), TileClass::Decoration);
        assert_eq!(coarse_tile_class(0x70), TileClass::Barrier);
        assert_eq!(coarse_tile_class(0x7F), TileClass::Barrier);
        assert_eq!(coarse_tile_class(0x80), TileClass::Special);
        assert_eq!(coarse_tile_class(0x9F), TileClass::Special);
        assert_eq!(coarse_tile_class(0xA0), TileClass::Vehicle);
        assert_eq!(coarse_tile_class(0xBB), TileClass::Vehicle);
        assert_eq!(coarse_tile_class(0xBC), TileClass::VehicleArt);
        assert_eq!(coarse_tile_class(0xBF), TileClass::VehicleArt);
        assert_eq!(coarse_tile_class(0xC0), TileClass::Npc);
        assert_eq!(coarse_tile_class(0xFF), TileClass::Npc);
    }

    #[test]
    fn classify_tlk_byte_partitions_dispatcher_table_per_section_seven() {
        // conversation.md §7 dispatcher classification order: 0x00 NUL,
        // 0x01..=0x7F dictionary, 0x9E..=0x9F GOTO label (precedes the
        // 0x80..=0x9F control band), 0x80..=0x9F control, 0xA0..=0xFD
        // printable, 0xFE IF-ELSE alias, 0xFF end-of-response.
        assert_eq!(classify_tlk_byte(0x00), TlkByteKind::Nul);
        assert_eq!(classify_tlk_byte(0x01), TlkByteKind::DictionaryToken);
        assert_eq!(classify_tlk_byte(0x7F), TlkByteKind::DictionaryToken);
        assert_eq!(classify_tlk_byte(0x80), TlkByteKind::ControlByte);
        assert_eq!(classify_tlk_byte(0x9D), TlkByteKind::ControlByte);
        assert_eq!(classify_tlk_byte(0x9E), TlkByteKind::GotoLabel);
        assert_eq!(classify_tlk_byte(0x9F), TlkByteKind::GotoLabel);
        assert_eq!(classify_tlk_byte(0xA0), TlkByteKind::PrintableText);
        assert_eq!(classify_tlk_byte(0xFD), TlkByteKind::PrintableText);
        assert_eq!(classify_tlk_byte(0xFE), TlkByteKind::IfElseAlias);
        assert_eq!(classify_tlk_byte(0xFF), TlkByteKind::EndOfResponse);
        // Spot-check the control codes resolve to ControlByte (not IfElseAlias).
        for code in [
            TLK_CODE_PRINT_AVATAR_NAME,
            TLK_CODE_GOLD_PAYMENT,
            TLK_CODE_ACTION_DISPATCH,
            TLK_CODE_IF_ELSE,
        ] {
            assert_eq!(classify_tlk_byte(code), TlkByteKind::ControlByte);
        }
    }

    #[test]
    fn tlk_label_byte_classifier_covers_section_seven_seven_range() {
        // conversation.md §7.7: label bytes 0x91..=0x9F, fifteen entries.
        for byte in TLK_LABEL_FIRST..=TLK_LABEL_LAST {
            assert!(is_tlk_label_byte(byte), "byte 0x{byte:02X} should be label");
        }
        assert!(!is_tlk_label_byte(0x90));
        assert!(!is_tlk_label_byte(0xA0));
        assert!(is_tlk_label_byte(TLK_CODE_GOTO_LABEL_FIRST));
        assert!(is_tlk_label_byte(TLK_CODE_GOTO_LABEL_LAST));
    }

    #[test]
    fn chargen_virtue_stat_deltas_match_spec_table() {
        // chargen.md §6: per-virtue (INT, DEX, STR) deltas table.
        let table: &[(ShrineVirtue, u8, u8, u8)] = &[
            (ShrineVirtue::Honesty, 2, 0, 0),
            (ShrineVirtue::Compassion, 0, 2, 0),
            (ShrineVirtue::Valor, 0, 0, 2),
            (ShrineVirtue::Justice, 1, 1, 0),
            (ShrineVirtue::Sacrifice, 0, 1, 1),
            (ShrineVirtue::Honor, 1, 0, 1),
            (ShrineVirtue::Spirituality, 1, 1, 1),
            (ShrineVirtue::Humility, 0, 0, 0),
        ];
        for (virtue, int, dex, str_) in table {
            let delta = chargen_virtue_stat_delta(*virtue);
            assert_eq!(
                (delta.intelligence, delta.dexterity, delta.strength),
                (*int, *dex, *str_),
                "virtue {} mismatch",
                virtue.name()
            );
        }
    }

    #[test]
    fn class_refreshed_mana_covers_default_branch_per_magic_md_section_eight() {
        // magic.md §8 Resurrection: Avatar (A), Mage (M), and the default
        // class branch receive mana equal to Intelligence; Bard (B)
        // receives half Intelligence.
        assert_eq!(class_refreshed_mana(b'A', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'M', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'B', 24), Some(12));
        // Default branch — every other class letter receives full INT.
        assert_eq!(class_refreshed_mana(b'F', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'P', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'R', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'T', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'D', 24), Some(24));
        assert_eq!(class_refreshed_mana(b'S', 24), Some(24));
    }

    #[test]
    fn intro_story_art_placement_for_step_matches_published_table() {
        // intro.md §10: spot-check primary story-art placements at all
        // file-boundary transitions.
        let p = intro_story_art_placement_for_step(0).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(2).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 136, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(7).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(13).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 0, top_left_x: 176, top_left_y: 0 });
        let p = intro_story_art_placement_for_step(20).unwrap();
        assert_eq!(p, IntroStoryArtPlacement { subimage: 4, top_left_x: 0, top_left_y: 87 });
        assert!(intro_story_art_placement_for_step(21).is_none());
    }

    #[test]
    fn intro_story_art_file_for_step_matches_published_boundaries() {
        // intro.md §10: steps 0-1 STORY1, 2-6 STORY2, 7-8 STORY3, 9-10
        // STORY4, 11-12 STORY5, 13-20 STORY6.
        assert_eq!(intro_story_art_file_for_step(0), Some("STORY1.16"));
        assert_eq!(intro_story_art_file_for_step(1), Some("STORY1.16"));
        assert_eq!(intro_story_art_file_for_step(2), Some("STORY2.16"));
        assert_eq!(intro_story_art_file_for_step(6), Some("STORY2.16"));
        assert_eq!(intro_story_art_file_for_step(7), Some("STORY3.16"));
        assert_eq!(intro_story_art_file_for_step(8), Some("STORY3.16"));
        assert_eq!(intro_story_art_file_for_step(9), Some("STORY4.16"));
        assert_eq!(intro_story_art_file_for_step(10), Some("STORY4.16"));
        assert_eq!(intro_story_art_file_for_step(11), Some("STORY5.16"));
        assert_eq!(intro_story_art_file_for_step(12), Some("STORY5.16"));
        assert_eq!(intro_story_art_file_for_step(13), Some("STORY6.16"));
        assert_eq!(intro_story_art_file_for_step(20), Some("STORY6.16"));
        assert_eq!(intro_story_art_file_for_step(21), None);
        assert_eq!(INTRO_STORY_STEP_COUNT, 21);
        assert_eq!(INTRO_AUTO_OPENING_STEP, 0);
        assert_eq!(INTRO_INLINE_DOORWAY_STEP, 6);
    }

    #[test]
    fn chargen_questionnaire_round_structure_matches_spec_section_six() {
        // chargen.md §6: 3 rounds (4 + 2 + 1 = 7 questions), single-elim.
        assert_eq!(CHARGEN_QUESTION_COUNT, 7);
        assert_eq!(CHARGEN_ROUND_COUNT, 3);
        assert_eq!(CHARGEN_QUESTIONS_PER_ROUND, [4, 2, 1]);
        assert_eq!(
            CHARGEN_QUESTIONS_PER_ROUND.iter().sum::<usize>(),
            CHARGEN_QUESTION_COUNT
        );
    }

    #[test]
    fn npc_dynamic_obstacle_radius_matches_published_threshold() {
        // npc-schedules.md §10: occupied cells are blocked only when the
        // occupant is within Manhattan distance less than four from the
        // NPC's runtime destination.
        assert_eq!(NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS, 4);
    }

    #[test]
    fn npc_schedule_state_constants_match_published_state_machine() {
        // npc-schedules.md §7: 0=empty, 1=idle, 2=in-plane move, 3=replay
        // queue, 4=descend, 5=ascend, 6=climb up off, 7=climb down off,
        // 8=parked off-floor.
        assert_eq!(NPC_STATE_EMPTY, 0);
        assert_eq!(NPC_STATE_IDLE, 1);
        assert_eq!(NPC_STATE_INPLANE_MOVE, 2);
        assert_eq!(NPC_STATE_REPLAY_QUEUE, 3);
        assert_eq!(NPC_STATE_DESCEND_TOWARD_TARGET, 4);
        assert_eq!(NPC_STATE_ASCEND_TOWARD_TARGET, 5);
        assert_eq!(NPC_STATE_CLIMB_UP_OFF_FLOOR, 6);
        assert_eq!(NPC_STATE_CLIMB_DOWN_OFF_FLOOR, 7);
        assert_eq!(NPC_STATE_PARKED_OFF_FLOOR, 8);
        assert_eq!(NPC_STUCK_REPLAN_THRESHOLD, 3);
    }

    #[test]
    fn tile_blocks_sight_propagation_matches_spec_classifier() {
        // visibility.md §6: the sight-blocking spec list.
        for tile in [
            0x09u8, 0x0A, 0x0C, 0x0D, 0x4D, 0x4E, 0x4F, 0x5A, 0x97, 0xB8, 0xB9, 0xBC, 0xD0,
            0xD1, 0xD2, 0xD3, 0xF8, 0xFE, 0xFF,
        ] {
            assert!(
                tile_blocks_sight_propagation(tile),
                "tile 0x{tile:02X} should block sight"
            );
        }
        // Non-listed tiles use the ordinary propagation rule.
        for tile in [0x00u8, 0x05, 0x10, 0x4A, 0x4B, 0x98, 0xBA, 0xBB, 0xC0] {
            assert!(
                !tile_blocks_sight_propagation(tile),
                "tile 0x{tile:02X} should not block sight"
            );
        }
    }

    #[test]
    fn tile_propagates_sight_only_when_adjacent_lists_orthogonal_set() {
        // visibility.md §6 orthogonal-only group.
        for tile in [0x4Au8, 0x4B, 0x98, 0xBA, 0xBB] {
            assert!(tile_propagates_sight_only_when_adjacent(tile));
        }
        for tile in [0x09u8, 0x0A, 0x4D, 0x97, 0xB8] {
            assert!(!tile_propagates_sight_only_when_adjacent(tile));
        }
    }

    #[test]
    fn shop_time_of_day_word_partitions_24_hour_clock() {
        // shops.md §4.1: morning for hours 0..12, afternoon for 12..18,
        // evening for 18..24.
        for hour in 0..12u8 {
            assert_eq!(shop_time_of_day_word(hour), "morning");
        }
        for hour in 12..18u8 {
            assert_eq!(shop_time_of_day_word(hour), "afternoon");
        }
        for hour in 18..24u8 {
            assert_eq!(shop_time_of_day_word(hour), "evening");
        }
    }

    #[test]
    fn game_clock_display_hour_and_am_pm_suffix_match_spec() {
        // time.md §2: display hour is 12 when underlying hour is 0; the
        // hour itself when 1..=12; otherwise hour - 12. AM for 0..12, PM
        // otherwise.
        let clock_at = |hour: u8| GameClock::new(hour, 0).unwrap();
        assert_eq!(clock_at(0).display_hour(), 12);
        assert_eq!(clock_at(0).am_pm_suffix(), "A.M.");
        assert_eq!(clock_at(1).display_hour(), 1);
        assert_eq!(clock_at(11).am_pm_suffix(), "A.M.");
        assert_eq!(clock_at(12).display_hour(), 12);
        assert_eq!(clock_at(12).am_pm_suffix(), "P.M.");
        assert_eq!(clock_at(13).display_hour(), 1);
        assert_eq!(clock_at(23).display_hour(), 11);
        assert_eq!(clock_at(23).am_pm_suffix(), "P.M.");
    }

    #[test]
    fn shrine_virtue_companion_table_matches_karma_md_section_nine() {
        // karma.md §9: virtue-to-companion pairing.
        assert_eq!(ShrineVirtue::Honesty.companion(), ("Mariah", "Mage"));
        assert_eq!(ShrineVirtue::Compassion.companion(), ("Iolo", "Bard"));
        assert_eq!(ShrineVirtue::Valor.companion(), ("Geoffrey", "Fighter"));
        assert_eq!(ShrineVirtue::Justice.companion(), ("Jaana", "Druid"));
        assert_eq!(ShrineVirtue::Sacrifice.companion(), ("Julia", "Tinker"));
        assert_eq!(ShrineVirtue::Honor.companion(), ("Dupre", "Paladin"));
        assert_eq!(ShrineVirtue::Spirituality.companion(), ("Shamino", "Ranger"));
        assert_eq!(ShrineVirtue::Humility.companion(), ("Katrina", "Shepherd"));
    }

    #[test]
    fn read_codex_urn_walks_virtues_in_standard_order() {
        // karma.md §8: walk the eight virtues in standard order, stamp the
        // first ordained-and-not-yet-Codex-read virtue, return the chosen
        // virtue. Honesty is index 0 and so should be picked first when
        // ordained.
        let mut codex = 0u8;
        let outcome = read_codex_urn(
            ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit(),
            &mut codex,
        );
        assert_eq!(outcome, CodexUrnReadOutcome::Stamped(ShrineVirtue::Honesty));
        assert_eq!(codex, ShrineVirtue::Honesty.bit());

        // Second read with same ordained mask should pick Justice next
        // because Honesty's Codex-read bit is now set.
        let outcome = read_codex_urn(
            ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit(),
            &mut codex,
        );
        assert_eq!(outcome, CodexUrnReadOutcome::Stamped(ShrineVirtue::Justice));
        assert_eq!(codex, ShrineVirtue::Honesty.bit() | ShrineVirtue::Justice.bit());
    }

    #[test]
    fn read_codex_urn_returns_completed_when_all_codex_bits_set() {
        // karma.md §8: with all eight Codex-read bits set, the reader takes
        // its completed branch and the saved masks are unchanged.
        let mut codex = 0xFFu8;
        let outcome = read_codex_urn(0xFF, &mut codex);
        assert_eq!(outcome, CodexUrnReadOutcome::Completed);
        assert_eq!(codex, 0xFF);
    }

    #[test]
    fn read_codex_urn_no_ordained_branch_when_no_bits_set() {
        // §8: if no virtue is ordained, no virtue can be stamped.
        let mut codex = 0u8;
        let outcome = read_codex_urn(0, &mut codex);
        assert_eq!(outcome, CodexUrnReadOutcome::NoOrdained);
        assert_eq!(codex, 0);
    }

    #[test]
    fn town_tile_predicates_match_published_catalog_ranges() {
        // catalogs/tile-catalog.md §6: door 96..=103, stair 0xC4..=0xC7,
        // chair 0x8C, NPC floor-link markers 0xC8 and 0xC9.
        assert!(is_town_door_tile(96));
        assert!(is_town_door_tile(99));
        assert!(is_town_door_tile(103));
        assert!(!is_town_door_tile(95));
        assert!(!is_town_door_tile(104));

        assert!(is_town_stair_tile(0xC4));
        assert!(is_town_stair_tile(0xC7));
        assert!(!is_town_stair_tile(0xC3));
        assert!(!is_town_stair_tile(0xC8));

        assert!(is_npc_floor_link_tile(0xC8));
        assert!(is_npc_floor_link_tile(0xC9));
        assert!(!is_npc_floor_link_tile(0xC7));
        assert!(!is_npc_floor_link_tile(0xCA));

        assert_eq!(TOWN_CHAIR_TILE, 0x8C);
    }

    #[test]
    fn spell_damage_caps_and_kill_sentinel_match_spec_table() {
        // catalogs/spell-list.md §5: Magic Missile raw 1..16 (id 1),
        // Fireball raw 1..30 (id 13), Kill is single-target instant kill
        // (id 37). combat.md §11 fixes Fire Field raw at 1..21 and §12
        // names the instant-kill sentinel value 99.
        assert_eq!(SPELL_CODES[MAGIC_MISSILE_SPELL_INDEX], "GP");
        assert_eq!(MAGIC_MISSILE_RAW_DAMAGE_MAX, 16);
        assert_eq!(SPELL_CODES[FIREBALL_SPELL_INDEX], "FV");
        assert_eq!(FIREBALL_RAW_DAMAGE_MAX, 30);
        assert_eq!(SPELL_CODES[KILL_SPELL_INDEX], "CX");
        assert_eq!(FIRE_FIELD_RAW_DAMAGE_MAX, 21);
    }

    #[test]
    fn spell_mp_cost_matches_published_per_spell_cost_constants() {
        // combat.md §10 says cost = (id/6)+1. Cross-check the formula
        // against the named per-spell COST constants for several spells.
        assert_eq!(spell_mp_cost(IN_LOR_SPELL_INDEX), Some(IN_LOR_COST));
        assert_eq!(spell_mp_cost(AWAKEN_SPELL_INDEX), Some(AWAKEN_COST));
        assert_eq!(spell_mp_cost(CURE_SPELL_INDEX), Some(CURE_COST));
        assert_eq!(spell_mp_cost(HEAL_SPELL_INDEX), Some(HEAL_COST));
        assert_eq!(spell_mp_cost(REL_HUR_SPELL_INDEX), Some(REL_HUR_COST));
        assert_eq!(spell_mp_cost(IN_WIS_SPELL_INDEX), Some(IN_WIS_COST));
        assert_eq!(spell_mp_cost(CREATE_FOOD_SPELL_INDEX), Some(CREATE_FOOD_COST));
        assert_eq!(spell_mp_cost(VAS_LOR_SPELL_INDEX), Some(VAS_LOR_COST));
        assert_eq!(spell_mp_cost(BLINK_SPELL_INDEX), Some(BLINK_COST));
        assert_eq!(spell_mp_cost(PROTECTION_SPELL_INDEX), Some(PROTECTION_COST));
        assert_eq!(spell_mp_cost(GREAT_HEAL_SPELL_INDEX), Some(GREAT_HEAL_COST));
        assert_eq!(spell_mp_cost(QUICKNESS_SPELL_INDEX), Some(QUICKNESS_COST));
        assert_eq!(spell_mp_cost(MASS_CHARM_SPELL_INDEX), Some(MASS_CHARM_COST));
        assert_eq!(spell_mp_cost(NEGATE_MAGIC_SPELL_INDEX), Some(NEGATE_MAGIC_COST));
        assert_eq!(spell_mp_cost(PEER_SPELL_INDEX), Some(PEER_COST));
        assert_eq!(spell_mp_cost(RESURRECT_SPELL_INDEX), Some(RESURRECT_COST));
        assert_eq!(spell_mp_cost(GATE_TRAVEL_SPELL_INDEX), Some(GATE_TRAVEL_COST));
        assert_eq!(spell_mp_cost(TIME_STOP_SPELL_INDEX), Some(TIME_STOP_COST));
    }

    #[test]
    fn spell_mp_cost_follows_eight_circles_of_six_layout() {
        // combat.md §10: spell MP cost is (spell_id / 6) + 1.
        // Circle 0 (id 0..5) costs 1; circle 1 (6..11) costs 2; ...
        // circle 7 (42..47) costs 8.
        assert_eq!(spell_mp_cost(0), Some(1));
        assert_eq!(spell_mp_cost(5), Some(1));
        assert_eq!(spell_mp_cost(6), Some(2));
        assert_eq!(spell_mp_cost(11), Some(2));
        assert_eq!(spell_mp_cost(12), Some(3));
        assert_eq!(spell_mp_cost(47), Some(8));
        assert_eq!(spell_mp_cost(48), None);

        assert_eq!(spell_circle_index(0), Some(0));
        assert_eq!(spell_circle_index(5), Some(0));
        assert_eq!(spell_circle_index(47), Some(7));
        assert_eq!(spell_circle_index(48), None);
    }

    #[test]
    fn spell_scene_bit_for_scene_byte_matches_published_partition() {
        // catalogs/spell-list.md §4: scene-byte to single-bit mapping.
        // 0 -> overworld, 1..=32 -> indoor, 33..=127 -> dungeon, >=0x80 -> combat.
        assert_eq!(spell_scene_bit_for_scene_byte(0), SPELL_SCENE_OVERWORLD);
        for byte in 1..=32u8 {
            assert_eq!(
                spell_scene_bit_for_scene_byte(byte),
                SPELL_SCENE_INDOOR,
                "byte {byte} should be indoor"
            );
        }
        for byte in [33u8, 40, 100, 127] {
            assert_eq!(
                spell_scene_bit_for_scene_byte(byte),
                SPELL_SCENE_DUNGEON,
                "byte {byte} should be dungeon"
            );
        }
        for byte in [0x80u8, 0x90, 0xC0, 0xFF] {
            assert_eq!(
                spell_scene_bit_for_scene_byte(byte),
                SPELL_SCENE_COMBAT,
                "byte 0x{byte:02X} should be combat"
            );
        }
    }

    #[test]
    fn capped_add_u8_clamps_at_caller_supplied_cap() {
        // stat-arithmetic.md §2: byte capped add stores cap when the result
        // reaches or exceeds the cap; returns actual delta applied.
        let mut field = 90u8;
        let applied = capped_add_u8(&mut field, 5, 99);
        assert_eq!(field, 95);
        assert_eq!(applied, 5);
        let applied = capped_add_u8(&mut field, 10, 99);
        assert_eq!(field, 99);
        assert_eq!(applied, 4);
        let applied = capped_add_u8(&mut field, 50, 99);
        assert_eq!(field, 99);
        assert_eq!(applied, 0);
    }

    #[test]
    fn capped_add_word_uses_signed_comparison_and_returns_delta() {
        // §2: word capped add uses signed comparison; returns actual delta.
        let mut hp: i16 = 50;
        assert_eq!(capped_add_word(&mut hp, 30, 100), 30);
        assert_eq!(hp, 80);
        assert_eq!(capped_add_word(&mut hp, 50, 100), 20);
        assert_eq!(hp, 100);
        // Negative starting field still observes signed cap.
        let mut hp: i16 = -5;
        assert_eq!(capped_add_word(&mut hp, 10, 100), 10);
        assert_eq!(hp, 5);
    }

    #[test]
    fn floor_sub_u8_floors_at_zero_and_returns_actual_subtracted() {
        // §2: byte floor subtract stores zero when the current value is not
        // greater than the amount; returns actual subtracted.
        let mut field = 7u8;
        assert_eq!(floor_sub_u8(&mut field, 3), 3);
        assert_eq!(field, 4);
        assert_eq!(floor_sub_u8(&mut field, 10), 4);
        assert_eq!(field, 0);
        assert_eq!(floor_sub_u8(&mut field, 5), 0);
        assert_eq!(field, 0);
    }

    #[test]
    fn floor_sub_word_clamps_at_zero_in_signed_comparison() {
        // §2: word floor subtract floors at zero with signed comparison.
        let mut hp: i16 = 30;
        assert_eq!(floor_sub_word(&mut hp, 18), 18);
        assert_eq!(hp, 12);
        assert_eq!(floor_sub_word(&mut hp, 100), 12);
        assert_eq!(hp, 0);
        assert_eq!(floor_sub_word(&mut hp, 5), 0);
        assert_eq!(hp, 0);
    }

    #[test]
    fn directed_step_offsets_reduce_wrapped_distance_to_player() {
        // active-objects.md §8: per-axis one-cell step toward the player on
        // the 256-cell torus. Aligned axes return 0; non-wrapped distances
        // pick the obvious direction; wrapped distances pick the shorter way.

        // Same cell: no movement.
        assert_eq!(directed_step_offsets(10, 10, 10, 10), (0, 0));

        // Player one east + two south: step east first.
        assert_eq!(directed_step_offsets(10, 10, 11, 12), (1, 1));

        // Player west + north: negative steps.
        assert_eq!(directed_step_offsets(10, 10, 8, 5), (-1, -1));

        // Wraparound: actor at 250, player at 5 -> shorter forward (wrap).
        assert_eq!(directed_step_offsets(250, 0, 5, 0), (1, 0));
        // Symmetric: actor at 5, player at 250 -> shorter backward (wrap).
        assert_eq!(directed_step_offsets(5, 0, 250, 0), (-1, 0));

        // Equidistant tie (128 each way) prefers forward step.
        assert_eq!(directed_step_offsets(0, 0, 128, 0), (1, 0));
    }

    #[test]
    fn terrain_chance_gate_denominator_matches_spec_outdoor_table() {
        // active-objects.md §8: half-chance for 0x04, 0x06..=0x08,
        // 0x1E..=0x1F; third-chance for 0x09..=0x0F; no gate for everything
        // else in the outdoor mover range.
        assert_eq!(terrain_chance_gate_denominator(0x04), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x06), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x07), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x08), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x1E), Some(2));
        assert_eq!(terrain_chance_gate_denominator(0x1F), Some(2));
        for tile in 0x09..=0x0F {
            assert_eq!(terrain_chance_gate_denominator(tile), Some(3));
        }
        assert_eq!(terrain_chance_gate_denominator(0x05), None);
        assert_eq!(terrain_chance_gate_denominator(0x10), None);
        assert_eq!(terrain_chance_gate_denominator(0x1D), None);
        assert_eq!(terrain_chance_gate_denominator(0x20), None);
        assert_eq!(terrain_chance_gate_denominator(0x00), None);
        assert_eq!(terrain_chance_gate_denominator(0xFF), None);
    }

    #[test]
    fn type_bypasses_terrain_chance_gate_lists_water_creatures_and_named_monsters() {
        // active-objects.md §8: ship-like water-creature frames 0x2C..=0x2F
        // and Bat/Daemon/Dragon/Mongbat first-frame type bytes bypass the
        // chance gate.
        for byte in 0x2C..=0x2Fu8 {
            assert!(type_bypasses_terrain_chance_gate(byte));
        }
        assert!(type_bypasses_terrain_chance_gate(0x94));
        assert!(type_bypasses_terrain_chance_gate(0xD8));
        assert!(type_bypasses_terrain_chance_gate(0xDC));
        assert!(type_bypasses_terrain_chance_gate(0xF0));
        // Sibling frames are not part of the bypass set.
        assert!(!type_bypasses_terrain_chance_gate(0x95));
        assert!(!type_bypasses_terrain_chance_gate(0xD9));
        assert!(!type_bypasses_terrain_chance_gate(0xDD));
        assert!(!type_bypasses_terrain_chance_gate(0xF1));
        // Random other bytes are not in the bypass set.
        assert!(!type_bypasses_terrain_chance_gate(0x00));
        assert!(!type_bypasses_terrain_chance_gate(0x80));
    }

    #[test]
    fn axis_first_choice_picks_x_or_y_from_one_bit_roll() {
        // active-objects.md §8: a one-bit random value chooses which axis to
        // try first.
        assert_eq!(axis_first_choice(0), Axis::X);
        assert_eq!(axis_first_choice(2), Axis::X);
        assert_eq!(axis_first_choice(1), Axis::Y);
        assert_eq!(axis_first_choice(3), Axis::Y);
    }

    #[test]
    fn fc_sprite_proximity_mask_matches_spec_six_by_six_table() {
        // active-objects.md §8: `0xFC` sprite class proximity-mask table.
        // Listed cells enter the special branch; the rest fall through.
        let listed = [
            (0u8, 2u8),
            (0, 3),
            (0, 4),
            (1, 3),
            (1, 4),
            (2, 2),
            (2, 3),
            (3, 0),
            (3, 1),
            (3, 2),
            (3, 3),
            (4, 0),
            (4, 1),
        ];
        for (dy, dx) in listed {
            assert!(
                fc_sprite_proximity_mask_hits(dy, dx),
                "({dy},{dx}) should hit"
            );
        }
        // Spot-check non-listed cells from inside the half-window.
        for (dy, dx) in [(0u8, 0u8), (0, 1), (1, 0), (1, 1), (1, 2), (2, 0), (2, 1)] {
            assert!(
                !fc_sprite_proximity_mask_hits(dy, dx),
                "({dy},{dx}) should not hit"
            );
        }
        // Row 5 is entirely outside the special branch.
        for dx in 0..=5u8 {
            assert!(!fc_sprite_proximity_mask_hits(5, dx));
        }
        // Cells outside the 6x6 half-window also miss.
        assert!(!fc_sprite_proximity_mask_hits(6, 0));
        assert!(!fc_sprite_proximity_mask_hits(0, 5));
    }

    #[test]
    fn wrap_text_breaks_at_spaces_within_window_width() {
        // text-output.md §6: only space, LF, CR, and NUL are break bytes.
        // Subsequent lines use the full window width.
        let lines = wrap_text("the quick brown fox", 10, 0);
        assert_eq!(lines, vec!["the quick", "brown fox"]);
    }

    #[test]
    fn wrap_text_first_line_uses_remaining_width_after_cursor() {
        // §6: first emitted line uses `window_width - cursor_x_at_entry`.
        let lines = wrap_text("hello world", 10, 5);
        // First line has 5 cells available, "hello" fits but "hello world"
        // doesn't, so wrap before "world".
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn wrap_text_terminates_on_nul_and_handles_hard_newlines() {
        // §6: NUL stops reading; LF/CR force a line emit.
        let lines = wrap_text("line one\nline two\0HIDDEN", 40, 0);
        assert_eq!(lines, vec!["line one", "line two"]);
    }

    #[test]
    fn tile_view_class_matches_spec_lookup_table() {
        // systems/view.md §4: per-tile view class lookup. Spot-check
        // representative tiles from each class plus boundary cases.
        // Class 0 (empty/pass-through)
        assert_eq!(tile_view_class(0x00), 0);
        assert_eq!(tile_view_class(0xC0), 0);
        assert_eq!(tile_view_class(0xCF), 0);
        assert_eq!(tile_view_class(0xFF), 0);
        // Class 1
        assert_eq!(tile_view_class(0x05), 1);
        assert_eq!(tile_view_class(0x30), 1);
        assert_eq!(tile_view_class(0x37), 1);
        // Class 2
        assert_eq!(tile_view_class(0x09), 2);
        assert_eq!(tile_view_class(0x2D), 2);
        // Class 3
        assert_eq!(tile_view_class(0x70), 3);
        assert_eq!(tile_view_class(0x7F), 3);
        assert_eq!(tile_view_class(0x44), 3);
        assert_eq!(tile_view_class(0xDD), 3);
        // Class 4
        assert_eq!(tile_view_class(0x5C), 4);
        assert_eq!(tile_view_class(0xBE), 4);
        // Class 5
        assert_eq!(tile_view_class(0x10), 5);
        assert_eq!(tile_view_class(0x1B), 5);
        assert_eq!(tile_view_class(0x4C), 5);
        assert_eq!(tile_view_class(0xFA), 5);
        // Class 6
        assert_eq!(tile_view_class(0xEC), 6);
        assert_eq!(tile_view_class(0xF9), 6);
        assert_eq!(tile_view_class(0xB8), 6);
        // Class 7
        assert_eq!(tile_view_class(0x4D), 7);
        assert_eq!(tile_view_class(0xFE), 7);
        // Class 8
        assert_eq!(tile_view_class(0x0B), 8);
        assert_eq!(tile_view_class(0x0F), 8);
        // Class 9
        assert_eq!(tile_view_class(0x06), 9);
        assert_eq!(tile_view_class(0x2C), 9);
        // Class A
        assert_eq!(tile_view_class(0x60), 0x0A);
        assert_eq!(tile_view_class(0x69), 0x0A);
        // Class B
        assert_eq!(tile_view_class(0xD4), 0x0B);
        assert_eq!(tile_view_class(0xD7), 0x0B);
        // Class C
        assert_eq!(tile_view_class(0x01), 0x0C);
        // Class D
        assert_eq!(tile_view_class(0x04), 0x0D);
        // Class E
        assert_eq!(tile_view_class(0xE0), 0x0E);
        assert_eq!(tile_view_class(0xE3), 0x0E);
        // Class F
        assert_eq!(tile_view_class(0xD8), 0x0F);
        assert_eq!(tile_view_class(0xDC), 0x0F);
        // Class 0x10
        assert_eq!(tile_view_class(0x20), 0x10);
        assert_eq!(tile_view_class(0x26), 0x10);
    }

    #[test]
    fn decode_end_window_strips_layout_markers_and_terminates_on_nul() {
        // formats/end-dat.md §3: `{` paragraph marker and `_` soft hyphen
        // are layout hints; NUL terminates the rendered output.
        let bytes = b"{Avatar_Standing\nat_the_circle\0HIDDEN";
        assert_eq!(decode_end_window(bytes), "AvatarStanding\natthecircle");
    }

    #[test]
    fn end_narrative_window_returns_decoded_subslice() {
        let raw = b"{Hello\nWorld\0".to_vec();
        let narrative = EndNarrative { raw };
        assert_eq!(narrative.full_text(), "Hello\nWorld");
        assert_eq!(narrative.window(1, 6).as_deref(), Some("Hello"));
        // Out-of-range window returns None per spec §5.
        assert!(narrative.window(0, 999).is_none());
    }

    #[test]
    fn parse_story_records_walks_twenty_records_and_strips_markup() {
        // formats/story-dat.md §2-§3: 20 NUL-terminated records driving the
        // intro story sequence; `{` and `_` are layout markup.
        let mut bytes = Vec::new();
        for index in 0..20usize {
            bytes.push(b'{');
            bytes.extend_from_slice(format!("Page{index}_break").as_bytes());
            bytes.push(0x00);
        }
        bytes.push(0x00); // Empty trailer per §2.

        let records = parse_story_records(&bytes).expect("20 records should parse");

        assert_eq!(records.records.len(), 20);
        assert_eq!(records.record(0), Some("Page0break"));
        assert_eq!(records.record(19), Some("Page19break"));
        assert_eq!(records.record(20), None);
    }

    #[test]
    fn parse_story_records_rejects_short_input() {
        let mut bytes = Vec::new();
        for _ in 0..5usize {
            bytes.extend_from_slice(b"x\0");
        }
        assert!(parse_story_records(&bytes).is_err());
    }

    #[test]
    fn parse_question_records_walks_thirty_records_and_strips_markup() {
        // formats/question-dat.md §2-§3: 30 NUL-terminated records;
        // record 0 = gypsy arrival, 1 = gypsy invitation, 2..=29 = dilemmas.
        // `{` is a paragraph marker and `_` is a soft hyphen; both stripped.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"{Arrival_text");
        bytes.push(0x00);
        bytes.extend_from_slice(b"Invitation");
        bytes.push(0x00);
        for _ in 2..30usize {
            bytes.extend_from_slice(b"Dilemma");
            bytes.push(0x00);
        }

        let records = parse_question_records(&bytes).expect("30 records should parse");

        assert_eq!(records.records.len(), 30);
        assert_eq!(records.gypsy_arrival(), Some("Arrivaltext"));
        assert_eq!(records.gypsy_invitation(), Some("Invitation"));
        // Dilemma records start at ordinal 2.
        assert_eq!(records.dilemma(2), Some("Dilemma"));
        assert_eq!(records.dilemma(29), Some("Dilemma"));
        assert_eq!(records.dilemmas().len(), 28);
    }

    #[test]
    fn parse_question_records_rejects_short_input() {
        // §7: fewer than 30 records is a bad asset.
        let mut bytes = Vec::new();
        for _ in 0..10usize {
            bytes.extend_from_slice(b"x\0");
        }
        assert!(parse_question_records(&bytes).is_err());
    }

    #[test]
    fn chargen_question_record_for_pair_matches_spec_table() {
        // formats/question-dat.md §4: spec lists records 2..=29 mapped to
        // virtue pairs. Spot-check several published rows.
        use ShrineVirtue::*;
        assert_eq!(
            chargen_question_record_for_pair(Honesty, Compassion).unwrap(),
            2
        );
        assert_eq!(
            chargen_question_record_for_pair(Honesty, Humility).unwrap(),
            8
        );
        assert_eq!(
            chargen_question_record_for_pair(Compassion, Valor).unwrap(),
            9
        );
        assert_eq!(
            chargen_question_record_for_pair(Valor, Justice).unwrap(),
            15
        );
        assert_eq!(
            chargen_question_record_for_pair(Spirituality, Humility).unwrap(),
            29
        );
        // Symmetric pair (b, a) returns the same record.
        assert_eq!(
            chargen_question_record_for_pair(Humility, Spirituality).unwrap(),
            29
        );
    }

    #[test]
    fn parse_misc_messages_clusters_records_by_consumer() {
        // formats/miscmsg-dat.md §2-§3: 47 NUL-terminated records grouped as
        // 0-11 Blackthorn audience, 12-19 virtue failing text, 20-27 virtue
        // aphorism, 28-35 shrine meditation, 36-46 urn/Codex prophecy.
        let mut bytes = Vec::new();
        for index in 0..47usize {
            let label = format!("rec{index}");
            bytes.extend_from_slice(label.as_bytes());
            bytes.push(0x00);
        }

        let messages = parse_misc_messages(&bytes).expect("47 records should parse");

        assert_eq!(messages.records.len(), 47);
        assert_eq!(messages.blackthorn_audience().len(), 12);
        assert_eq!(messages.virtue_failing_text().len(), 8);
        assert_eq!(messages.virtue_aphorism().len(), 8);
        assert_eq!(messages.shrine_meditation().len(), 8);
        assert_eq!(messages.urn_codex().len(), 11);
        assert_eq!(messages.record(0), Some("rec0"));
        assert_eq!(messages.record(12), Some("rec12"));
        assert_eq!(messages.record(46), Some("rec46"));
        assert_eq!(messages.record(47), None);
    }

    #[test]
    fn parse_misc_messages_preserves_codex_tile_glyph_bytes() {
        // formats/miscmsg-dat.md §4: Codex tile-glyph bytes (`@`, `[`, `]`,
        // `_`) pass through unchanged for the caller to render through the
        // tile-glyph path.
        let mut bytes = Vec::new();
        for _ in 0..36usize {
            bytes.push(b'a');
            bytes.push(0x00);
        }
        bytes.extend_from_slice(b"TRU[");
        bytes.push(0x00);
        for _ in 37..47usize {
            bytes.push(b'b');
            bytes.push(0x00);
        }

        let messages = parse_misc_messages(&bytes).expect("47 records should parse");
        assert_eq!(messages.record(36), Some("TRU["));
    }

    #[test]
    fn parse_misc_messages_rejects_truncated_or_short_input() {
        // §6: missing terminators and short record counts must be rejected.
        let mut short = Vec::new();
        for _ in 0..10usize {
            short.extend_from_slice(b"x\0");
        }
        assert!(parse_misc_messages(&short).is_err());

        let unterminated = b"hello".to_vec();
        assert!(parse_misc_messages(&unterminated).is_err());
    }

    #[test]
    fn parse_endgame_messages_walks_eleven_nul_terminated_records() {
        // formats/endmsg-dat.md §2-§4: eleven NUL-terminated plain-ASCII
        // records consumed by the endgame Lord British dialogue.
        let labels = [
            "Greetings",
            "First box prompt",
            "Second box prompt",
            "Rite 1",
            "Rite 2",
            "Rite 3",
            "Rite 4",
            "Rite 5",
            "Rite 6",
            "Rite 7",
            "Refusal branch",
        ];
        let mut bytes = Vec::new();
        for label in labels {
            bytes.extend_from_slice(label.as_bytes());
            bytes.push(0x00);
        }

        let messages = parse_endgame_messages(&bytes).expect("11 records should parse");

        assert_eq!(messages.records.len(), 11);
        assert_eq!(messages.initial_greeting(), Some("Greetings"));
        assert_eq!(messages.first_box_prompt(), Some("First box prompt"));
        assert_eq!(messages.second_box_prompt(), Some("Second box prompt"));
        assert_eq!(messages.rite_messages().len(), 7);
        assert_eq!(messages.refusal_branch(), Some("Refusal branch"));
    }

    #[test]
    fn parse_endgame_messages_rejects_unterminated_record() {
        // §5: a missing NUL terminator must be rejected as a bad asset.
        let mut bytes = b"Hello\0World".to_vec();
        // 'World' is not NUL-terminated; parser should error.
        assert!(parse_endgame_messages(&bytes).is_err());

        // Also reject when fewer than 11 records.
        bytes = b"only one record\0".to_vec();
        assert!(parse_endgame_messages(&bytes).is_err());
    }

    #[test]
    fn parse_sign_records_decodes_directory_and_payload() {
        // formats/signs-dat.md §2-§4. Build a minimal SIGNS.DAT image with
        // two scene blocks separated by a zero-scene sentinel. Scene 17 has
        // one record at (0, 5, 6); scene 18 has one record at (1, 7, 8)
        // using divider/decoration glyphs.
        let mut bytes = vec![0u8; 33 * 2];
        let scene17_offset = 66u16;
        bytes[17 * 2..17 * 2 + 2].copy_from_slice(&scene17_offset.to_le_bytes());
        // Scene 17 record + payload + NUL + sentinel = 4 + 5 + 1 + 1 = 11
        let scene18_offset = scene17_offset + 4 + 5 + 1 + 1;
        bytes[18 * 2..18 * 2 + 2].copy_from_slice(&scene18_offset.to_le_bytes());
        // Scene 17 block.
        bytes.extend_from_slice(&[17, 0, 5, 6]);
        bytes.extend_from_slice(b"Hello");
        bytes.push(0x00);
        bytes.push(0x00); // end-of-block sentinel
        // Scene 18 block.
        bytes.extend_from_slice(&[18, 1, 7, 8]);
        bytes.extend_from_slice(&[b'A', 0x26, b'B', 0x29, b'C']);
        bytes.push(0x00);
        bytes.push(0x00); // end-of-block sentinel

        let records = parse_sign_records(&bytes).expect("parse should succeed");
        assert_eq!(records.len(), 2);

        let lookup_17 = find_sign(&records, 17, 0, 5, 6).expect("scene 17 record present");
        assert_eq!(lookup_17.body, "Hello");

        let lookup_18 = find_sign(&records, 18, 1, 7, 8).expect("scene 18 record present");
        assert_eq!(lookup_18.body, "A-B*C");

        // No matching record returns None.
        assert!(find_sign(&records, 17, 1, 1, 1).is_none());
    }

    #[test]
    fn parse_sign_records_rejects_short_directory() {
        // Less than the 66-byte scene directory must error per §2 of the format spec.
        assert!(parse_sign_records(&[0u8; 10]).is_err());
    }

    #[test]
    fn decode_sign_payload_handles_pause_and_high_bit() {
        // §4: 0x0D becomes a newline; high-bit text still prints as the
        // low-seven-bit character.
        let bytes = [b'A', 0x0d, b'B' | 0x80, b'C'];
        assert_eq!(decode_sign_payload(&bytes), "A\nBC");
    }

    #[test]
    fn sky_strip_marker_position_matches_spec_visibility_table() {
        // moons.md §2: Fixed hour marker visible 06:00..17:59 at cell `17 -
        // hour`. Trammel visible 00:00..08:59 at `8 - hour` and 21:00..23:59
        // at `32 - hour`. Felucca visible 00:00..02:59 at `2 - hour` and
        // 15:00..23:59 at `26 - hour`. All other hours are below the horizon.

        // Fixed hour marker boundaries.
        assert_eq!(sky_strip_marker_position(5, SkyStripMarker::FixedHour), None);
        assert_eq!(
            sky_strip_marker_position(6, SkyStripMarker::FixedHour),
            Some(11)
        );
        assert_eq!(
            sky_strip_marker_position(12, SkyStripMarker::FixedHour),
            Some(5)
        );
        assert_eq!(
            sky_strip_marker_position(17, SkyStripMarker::FixedHour),
            Some(0)
        );
        assert_eq!(sky_strip_marker_position(18, SkyStripMarker::FixedHour), None);

        // Trammel windows.
        assert_eq!(
            sky_strip_marker_position(0, SkyStripMarker::Trammel),
            Some(8)
        );
        assert_eq!(
            sky_strip_marker_position(8, SkyStripMarker::Trammel),
            Some(0)
        );
        // Hour 9..20 inclusive is below horizon.
        assert_eq!(sky_strip_marker_position(9, SkyStripMarker::Trammel), None);
        assert_eq!(sky_strip_marker_position(20, SkyStripMarker::Trammel), None);
        assert_eq!(
            sky_strip_marker_position(21, SkyStripMarker::Trammel),
            Some(11)
        );
        assert_eq!(
            sky_strip_marker_position(23, SkyStripMarker::Trammel),
            Some(9)
        );

        // Felucca windows.
        assert_eq!(
            sky_strip_marker_position(0, SkyStripMarker::Felucca),
            Some(2)
        );
        assert_eq!(
            sky_strip_marker_position(2, SkyStripMarker::Felucca),
            Some(0)
        );
        assert_eq!(sky_strip_marker_position(3, SkyStripMarker::Felucca), None);
        assert_eq!(sky_strip_marker_position(14, SkyStripMarker::Felucca), None);
        assert_eq!(
            sky_strip_marker_position(15, SkyStripMarker::Felucca),
            Some(11)
        );
        assert_eq!(
            sky_strip_marker_position(23, SkyStripMarker::Felucca),
            Some(3)
        );
    }

    #[test]
    fn endgame_step_toward_target_prefers_axis_with_greater_distance() {
        // endgame.md §7: each call moves one cell toward target along the axis
        // with the greater remaining distance.
        // Pure horizontal: dx > 0, dy = 0
        assert_eq!(endgame_step_toward_target((0, 5), (3, 5)), (1, 5));
        // Pure vertical: dx = 0, dy < 0
        assert_eq!(endgame_step_toward_target((4, 5), (4, 1)), (4, 4));
        // Diagonal with greater dx
        assert_eq!(endgame_step_toward_target((0, 0), (5, 2)), (1, 0));
        // Diagonal with greater dy
        assert_eq!(endgame_step_toward_target((0, 0), (2, 5)), (0, 1));
        // Negative directions
        assert_eq!(endgame_step_toward_target((10, 10), (3, 7)), (9, 10));
        assert_eq!(endgame_step_toward_target((10, 10), (8, 3)), (10, 9));
        // On target: no movement
        assert_eq!(endgame_step_toward_target((4, 4), (4, 4)), (4, 4));
        // Equal-distance ties: prefer X axis
        assert_eq!(endgame_step_toward_target((0, 0), (3, 3)), (1, 0));
    }

    #[test]
    fn endgame_certificate_word_helpers_cover_calendar_range() {
        assert_eq!(endgame_ordinal_word(1).as_deref(), Some("first"));
        assert_eq!(endgame_ordinal_word(21).as_deref(), Some("twenty-first"));
        assert_eq!(endgame_ordinal_word(28).as_deref(), Some("twenty-eighth"));
        assert_eq!(endgame_ordinal_word(29), None);
        assert_eq!(endgame_cardinal_word(139), "one hundred thirty-nine");
        assert_eq!(endgame_cardinal_word(141), "one hundred forty-one");
        assert_eq!(
            endgame_cardinal_word(2026),
            "two thousand twenty-six"
        );
    }

    #[test]
    fn dungeon_current_room_helper_state_fires_before_next_key_without_rewriting() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0xa4;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('w', Path::new("")).unwrap());

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0xa4);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("slot 4"));
        assert!(state.message.contains("arena 4"));
    }

    #[test]
    fn dungeon_room_helper_state_reports_arena_without_rewriting() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0xa4;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa4);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("room-helper state slot 4"));
        assert!(state.message.contains("arena 4"));
    }

    #[test]
    fn dungeon_movement_rejects_diagonals_and_wraps_bounds() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 0, 0);

        assert_eq!(state.step(Direction::NorthWest), MoveOutcome::Blocked);
        assert_eq!((state.player.x, state.player.y), (0, 0));
        assert_eq!(state.turn, 0);

        assert_eq!(state.step(Direction::North), MoveOutcome::Moved);
        assert_eq!((state.player.x, state.player.y), (0, 7));
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Moved North to (0, 7)"));
    }

    #[test]
    fn dungeon_movement_blocks_active_monster_cell_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Blocked!");
    }

    #[test]
    fn dungeon_play_keys_use_facing_relative_forward_and_back() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;

        assert!(state.handle_dungeon_key('w', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (2, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 1);

        assert!(state.handle_dungeon_key('s', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn dungeon_play_keys_turn_without_changing_cell() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;

        assert!(state.handle_dungeon_key('a', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::North);
        assert_eq!(state.turn, 1);

        assert!(state.handle_dungeon_key('d', Path::new("")).unwrap());
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn dungeon_l_key_looks_instead_of_turning() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x61;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 5;

        assert!(state.handle_dungeon_key('l', Path::new("")).unwrap());

        assert_eq!(state.player.facing, Direction::East);
        assert_eq!(state.turn, 0);
        assert_eq!(state.look_dungeon(), MoveOutcome::Observed);
        assert!(state.message.contains("passage"));
    }

    #[test]
    fn dungeon_talk_reports_no_response_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert!(state.handle_dungeon_key('T', Path::new("")).unwrap());

        assert_eq!(state.message, "Funny, no response!");
        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_i_key_ignites_and_reveals_forward_view() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.torches = 2;

        assert!(state.handle_dungeon_key('I', Path::new("")).unwrap());

        assert_eq!(state.torches, 1);
        assert!((112..=127).contains(&state.torch_counter));
        assert_eq!(state.turn, 1);
        let view = state.render_text_view(5);
        assert!(view.contains("First-person dungeon view"));
        assert!(!view.contains("darkness"));
    }

    #[test]
    fn dungeon_o_key_routes_to_underfoot_open() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);

        assert!(state.handle_dungeon_key('O', Path::new("")).unwrap());

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x7b);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Opened dungeon chest"));
    }

    #[test]
    fn dungeon_v_key_routes_to_gem_map() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.gems = 1;

        assert!(state.handle_dungeon_key('v', Path::new("")).unwrap());

        assert_eq!(state.gems, 0);
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Dungeon view"));
        assert!(state.message.contains("centered flood map"));
    }

    #[test]
    fn dungeon_attack_uses_forward_wrapped_probe_without_direction_prompt() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 0, 1);
        state.player.facing = Direction::West;

        assert!(state.handle_dungeon_key('A', Path::new("")).unwrap());

        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Attacked forward at (7, 1)"));
        assert!(state.message.contains("no target"));
    }

    #[test]
    fn dungeon_attack_forward_monster_clears_active_object_and_consumes_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert!(state.handle_dungeon_key('A', Path::new("")).unwrap());

        assert_eq!(state.turn, 1);
        assert!(state.active_objects[1].is_empty());
        assert!(state.message.contains("Attacked dungeon monster tile 192"));
        assert!(state.message.contains("dungeon combat resolution is pending"));
    }

    #[test]
    fn top_down_uppercase_command_letters_preempt_vi_movement() {
        for (key, expected) in [
            ('A', "Attack where?"),
            ('C', "Cast what?"),
            ('D', "What?"),
            ('M', "Mix what?"),
            ('N', "New order?"),
            ('Q', "Save game?"),
            ('U', "Use what?"),
            ('W', "What?"),
            ('Z', "Z-stats:"),
        ] {
            let mut state = test_state(open_grid(), 5, 5);

            assert!(
                state
                    .handle_top_down_key_with_inline(key, Path::new(""), None, None, None, None)
                    .unwrap()
            );

            assert_eq!((state.player.x, state.player.y), (5, 5));
            assert_eq!(state.turn, 0);
            assert!(
                state.message.contains(expected),
                "{key} reported `{}`",
                state.message
            );
        }
    }

    #[test]
    fn top_down_lowercase_vi_and_wasd_movement_still_routes_before_commands() {
        for (key, expected_position) in [
            ('y', (4, 4)),
            ('w', (5, 4)),
            ('u', (6, 4)),
            ('a', (4, 5)),
            ('d', (6, 5)),
            ('b', (4, 6)),
            ('s', (5, 6)),
            ('n', (6, 6)),
            ('c', (6, 6)),
            ('z', (4, 6)),
        ] {
            let mut state = test_state(open_grid(), 5, 5);

            assert!(
                state
                    .handle_top_down_key_with_inline(key, Path::new(""), None, None, None, None)
                    .unwrap()
            );

            assert_eq!(
                (state.player.x, state.player.y),
                expected_position,
                "{key} routed to `{}`",
                state.message
            );
            assert_eq!(state.turn, 1);
        }
    }

    #[test]
    fn top_down_lowercase_x_routes_to_vehicle_exit() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.player.transport = TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        };
        state.sync_player_object();

        assert!(
            state
                .handle_top_down_key_with_inline('x', Path::new(""), None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.player.transport, TransportState::Foot);
        assert_eq!((state.player.x, state.player.y), (6, 5));
        assert!(state.active_objects.iter().skip(1).any(|object| {
            object.type_byte == 184
                && object.tile == 184
                && object.x == 5
                && object.y == 5
                && object.z == WorldPlane::Underworld.save_floor()
        }));
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "carpet!");
    }

    #[test]
    fn town_enter_uses_stock_refusal_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);

        assert!(
            state
                .handle_top_down_key_with_inline('E', Path::new(""), None, None, None, None)
                .unwrap()
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn dungeon_turn_does_not_animate_top_down_active_objects() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 3,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.pass_turn(), MoveOutcome::Passed);

        let object = state.active_objects[1];
        assert_eq!(object.phase, 0x22);
        assert_eq!(object.tile, 192);
        assert_eq!((object.x, object.y), (3, 3));
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
    }

    #[test]
    fn dungeon_post_turn_active_monster_greedy_steps_toward_party() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 1,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
            MoveOutcome::Passed
        );

        let object = state.active_objects[1];
        assert_eq!((object.x, object.y), (2, 1));
        assert_eq!(object.phase, active_object_phase_from_direction(Direction::West, 0));
        assert!(state.message.contains("Dungeon monster tile 192 moved West to (2, 1)"));
    }

    #[test]
    fn dungeon_post_turn_active_monster_rejects_sleep_field_step() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x80;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 1,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
            MoveOutcome::Passed
        );

        let object = state.active_objects[1];
        assert_eq!((object.x, object.y), (3, 1));
        assert!(!state.message.contains("Dungeon monster tile 192 moved"));
    }

    #[test]
    fn dungeon_post_turn_active_monster_contact_faces_threat_and_consumes_monster() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::North;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 2,
            y: 1,
            z: 0,
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
            MoveOutcome::Used
        );

        assert_eq!(state.player.facing, Direction::East);
        assert!(state.active_objects[1].is_empty());
        assert!(state.message.contains("approaches from the East"));
        assert!(state.message.contains("dungeon combat resolution is pending"));
    }

    #[test]
    fn dungeon_idle_tick_does_not_animate_top_down_active_objects() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 3,
            y: 3,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(state.idle_tick(), MoveOutcome::IdleTick);

        let object = state.active_objects[1];
        assert_eq!(object.phase, 0x22);
        assert_eq!(object.tile, 192);
        assert_eq!((object.x, object.y), (3, 3));
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::new(12, 0).unwrap());
    }

    #[test]
    fn dungeon_mode_refuses_world_vehicle_and_entry_letters_without_turn() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        };

        for (key, expected) in [('B', "Not here!"), ('E', "Not here!"), ('X', "Not here!")] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert_eq!(state.message, expected);
            assert_eq!(
                state.player.transport,
                TransportState::Ship {
                    type_byte: 168,
                    tile: 168,
                    sails_hoisted: false,
                    hull: 0,
                    skiffs: 0,
                }
            );
            assert_eq!((state.player.x, state.player.y), (1, 1));
            assert_eq!(state.turn, 0);
        }

        for key in ['F', 'P'] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert_eq!(state.message, "What?");
            assert_eq!(state.turn, 0);
        }

        assert!(state.handle_dungeon_key('Q', Path::new("")).unwrap());
        assert_eq!(
            state.message,
            "Exit to DOS? Use QY to exit or QN to cancel."
        );
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn dungeon_q_exit_prompt_is_separate_from_save_command() {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(33, 0, 1, 1);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join("SAVED.GAM"), &template).unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            state.message,
            "Exit to DOS? Use QY to exit or QN to cancel."
        );
        assert_eq!(state.turn, 0);
        assert!(!dir.join("SAVED.OOL").exists());

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "N", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, "No.");
        assert_eq!(state.turn, 0);
        assert!(!dir.join("SAVED.OOL").exists());

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "Y", &dir).unwrap(),
            PlayInputDisposition::Quit
        );
        assert_eq!(state.message, "Yes. Exiting to DOS.");
        assert_eq!(state.turn, 0);
        assert!(!dir.join("SAVED.OOL").exists());
        assert_eq!(fs::read(dir.join("SAVED.GAM")).unwrap(), template);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_command_letters_do_not_fall_through_to_diagonal_movement_refusal() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        for (key, expected) in [
            ('C', "Cast what?"),
            ('M', "Mix what?"),
            ('N', "New order?"),
            ('R', "Ready what?"),
            ('U', "Use what?"),
            ('Y', "Yell what?"),
            ('Z', "Z-stats:"),
        ] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert!(
                state.message.contains(expected),
                "{key} reported `{}`",
                state.message
            );
            assert_eq!((state.player.x, state.player.y), (1, 1));
            assert_eq!(state.turn, 0);
        }

        for key in ['7', '9', '1', '3'] {
            assert!(state.handle_dungeon_key(key, Path::new("")).unwrap());
            assert!(state.message.contains("forward, back, and turns only"));
            assert_eq!((state.player.x, state.player.y), (1, 1));
            assert_eq!(state.turn, 0);
        }
    }

