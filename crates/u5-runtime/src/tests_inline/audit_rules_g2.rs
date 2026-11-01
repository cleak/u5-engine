// Closed-audit coverage, group g2: normative rules of `systems/shops.md`,
// `systems/animation.md`, `formats/saved-gam.md`, `systems/stats-panel.md`,
// `systems/movement.md`, `formats/tlk.md`, `formats/look2-dat.md` and
// `formats/endmsg-dat.md` that the engine implements but no earlier test
// asserted. Each test names its spec file and section. Text fixtures are
// invented placeholders or literals the spec itself publishes.

// ---------------------------------------------------------------------------
// formats/endmsg-dat.md
// ---------------------------------------------------------------------------

/// `formats/endmsg-dat.md §2`: "Embedded line feeds for paragraph breaks" -
/// the record keeps its interior line feed rather than ending at it.
#[test]
fn g2_endmsg_s2_records_keep_their_embedded_line_feeds() {
    let mut bytes = Vec::new();
    for index in 0..ENDMSG_DAT_RECORDS {
        if index == 4 {
            bytes.extend_from_slice(b"first line\nsecond line");
        } else {
            bytes.extend_from_slice(format!("record {index}").as_bytes());
        }
        bytes.push(0x00);
    }
    let messages = parse_endgame_messages(&bytes).expect("eleven records parse");
    assert_eq!(messages.records.len(), ENDMSG_DAT_RECORDS);
    assert_eq!(messages.records[4], "first line\nsecond line");
    assert_eq!(messages.rite_messages()[1], "first line\nsecond line");
}

// ---------------------------------------------------------------------------
// formats/look2-dat.md
// ---------------------------------------------------------------------------

/// Build a LOOK2.DAT image whose pool holds `strings` in the given order and
/// whose 512 table words point at `pointers[entry] = string index`; entries
/// without a pointer use string 0.
fn g2_look2_image(strings: &[&str], pointers: &[(usize, usize)]) -> Vec<u8> {
    let mut offsets = Vec::new();
    let mut pool = Vec::new();
    for string in strings {
        offsets.push((LOOK2_TABLE_LEN + pool.len()) as u16);
        pool.extend_from_slice(string.as_bytes());
        pool.push(0);
    }
    let mut table = vec![offsets[0]; LOOK2_TILE_COUNT];
    for (entry, string) in pointers {
        table[*entry] = offsets[*string];
    }
    let mut bytes = Vec::with_capacity(LOOK2_TABLE_LEN + pool.len());
    for word in table {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend(pool);
    bytes
}

/// `formats/look2-dat.md §6`: "The file must be at least 1,024 bytes long."
#[test]
fn g2_look2_s6_rejects_a_file_shorter_than_the_offset_table() {
    let bytes = vec![0u8; LOOK2_TABLE_LEN - 1];
    assert!(parse_look2_dat(&bytes).is_err());
    assert!(parse_look2_dat(&[]).is_err());
}

/// `formats/look2-dat.md §6`: every offset must be "less than the file length
/// after ignoring a final DOS end-of-file marker if present".
#[test]
fn g2_look2_s6_rejects_an_offset_at_the_meaningful_end_of_file() {
    let mut bytes = g2_look2_image(&["*", "grass"], &[(5, 1)]);
    bytes.push(DOS_EOF_MARKER);
    assert!(parse_look2_dat(&bytes).is_ok());
    // Point entry 7 at the EOF marker itself: that is past meaningful data.
    let marker_offset = (bytes.len() - 1) as u16;
    bytes[14..16].copy_from_slice(&marker_offset.to_le_bytes());
    assert!(parse_look2_dat(&bytes).is_err());
}

/// `formats/look2-dat.md §6`: "Offsets need not be sorted by tile id;
/// consumers should not depend on monotonic ordering."
#[test]
fn g2_look2_s6_unsorted_offsets_resolve_their_own_strings() {
    // Pool order: sentinel, "zeta", "alpha". Tile 3 points at the later
    // string and tile 200 at the earlier one, so offsets fall as ids rise.
    let bytes = g2_look2_image(&["*", "zeta", "alpha"], &[(3, 2), (200, 1)]);
    let table = parse_look2_dat(&bytes).expect("unsorted table parses");
    assert_eq!(table.description(3), Some("alpha"));
    assert_eq!(table.description(200), Some("zeta"));
}

/// `formats/look2-dat.md §1`/`§6`: "Duplicate offsets are valid and
/// expected"; the table is "a many-to-one mapping".
#[test]
fn g2_look2_s6_duplicate_offsets_share_one_string() {
    let bytes = g2_look2_image(&["*", "a shared wall"], &[(0x40, 1), (0x41, 1), (0x42, 1)]);
    let table = parse_look2_dat(&bytes).unwrap();
    for tile in [0x40, 0x41, 0x42] {
        assert_eq!(table.description(tile), Some("a shared wall"));
    }
}

/// `formats/look2-dat.md §4`: "Each glyph occupies several distinct positions
/// in the string pool rather than one shared position, so a reader must
/// compare the *string*, not the offset, to recognise a sentinel."
#[test]
fn g2_look2_s4_sentinel_is_recognised_by_string_not_offset() {
    // A second copy of the terrain sentinel at its own offset (string 2), and
    // the object sentinel at entry 256.
    let bytes = g2_look2_image(&["*", "x", "*", "a target"], &[
        (0x59, 2),
        (0x10, 3),
        (LOOK2_DAT_TERRAIN_ENTRIES, 1),
        (LOOK2_DAT_TERRAIN_ENTRIES + 0x12, 1),
    ]);
    let table = parse_look2_dat(&bytes).unwrap();
    let copy = table.description(0x59).unwrap();
    assert!(table.is_sentinel(copy), "the second `*` copy is still the sentinel");
    assert!(table.is_sentinel(table.description(LOOK2_DAT_TERRAIN_ENTRIES + 0x12).unwrap()));
    assert!(!table.is_sentinel(table.description(0x10).unwrap()));
}

/// `formats/look2-dat.md §3`: object-domain id `k` reads entry `0x100 + k`;
/// "The same numeral therefore names two unrelated things" - resolve the
/// domain before indexing.
#[test]
fn g2_look2_s3_object_domain_id_reads_the_upper_half_of_the_table() {
    let bytes = g2_look2_image(&["*", "x", "an invented beast"], &[
        (LOOK2_DAT_TERRAIN_ENTRIES, 1),
        (LOOK2_DAT_TERRAIN_ENTRIES + 0xC0, 2),
    ]);
    let table = parse_look2_dat(&bytes).unwrap();
    let state = test_state(open_grid(), 1, 1);
    assert_eq!(
        state.look_object_description(0xC0, Some(&table)),
        "an invented beast"
    );
    // The identically numbered terrain entry is a different row.
    assert_eq!(table.description(0xC0), Some("*"));
}

// ---------------------------------------------------------------------------
// formats/tlk.md
// ---------------------------------------------------------------------------

fn g2_tlk_text(text: &str) -> Vec<u8> {
    text.bytes().map(|byte| byte ^ TLK_TEXT_XOR_MASK).collect()
}

/// `formats/tlk.md §4`/`§6`: the header is a count word then `npc_count`
/// `(npc_id, blob_offset)` entries; the header is `4N + 2` bytes, id `1` is a
/// real NPC whose blob begins right after the header, and the last blob is
/// addressed by the last entry.
#[test]
fn g2_tlk_s4_s6_header_entries_are_id_then_offset_and_id_one_is_real() {
    let mut first = g2_tlk_text("Ann");
    first.push(0);
    first.extend(g2_tlk_text("a placeholder"));
    first.push(0);
    let mut second = g2_tlk_text("Bob");
    second.push(0);
    second.extend(g2_tlk_text("another placeholder"));
    second.push(0);

    let count = 2u16;
    let header_len = 4 * count as usize + 2;
    assert_eq!(header_len, 10);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&(header_len as u16).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&((header_len + first.len()) as u16).to_le_bytes());
    bytes.extend(first);
    bytes.extend(second);

    let parsed = parse_tlk_bytes(&bytes).expect("well-formed header");
    assert_eq!(parsed.len(), 2, "every id 1..=count addresses a blob");
    assert_eq!(parsed[&1][0], "Ann");
    assert_eq!(parsed[&2][0], "Bob", "the final blob is referenced, not a tail");
}

/// `formats/tlk.md §4`: the engine reads "a second fixed-window read of one
/// thousand twenty-four bytes at the matched blob offset" - a blob longer
/// than that window is cut at the window.
#[test]
fn g2_tlk_s4_blob_load_is_a_fixed_1024_byte_window() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&6u16.to_le_bytes());
    // One unterminated run of 1100 text bytes, then its NUL.
    bytes.extend(std::iter::repeat(b'A' ^ TLK_TEXT_XOR_MASK).take(1100));
    bytes.push(0);

    let raw = parse_tlk_blob_fields_raw(&bytes).unwrap();
    let fields = &raw[&1];
    assert_eq!(fields[0].len(), TLK_BLOB_FIXED_WINDOW);
}

/// `formats/tlk.md §8`: "A literal space (`0x20`) is stored as `0xA0`; a
/// literal `'A'` (`0x41`) as `0xC1`" and the runner strips bit 7 to emit them.
#[test]
fn g2_tlk_s8_runner_strips_bit_seven_from_text_bytes() {
    let inputs = crate::tlk_runner::TlkRunInputs {
        avatar_name: "X",
        ..Default::default()
    };
    let out = crate::tlk_runner::run_tlk_stream(&[0xC1, 0xA0, 0xC2], &inputs);
    assert_eq!(out.text, "A B");
}

// ---------------------------------------------------------------------------
// systems/movement.md
// ---------------------------------------------------------------------------

/// `systems/movement.md §8.1` difficult-terrain table, every terrain id:
/// `0x04 0x06 0x07 0x08 0x1E 0x1F` cost one extra call and two minutes with
/// `Slow progress!`; `0x09..0x0F` cost two calls and four minutes with
/// `Very slow!`; all others nothing. Grass `0x05` is free and `0x0A` takes the
/// higher cost.
#[test]
fn g2_movement_s8_1_difficult_terrain_table_covers_every_terrain_id() {
    for tile in 0u8..=u8::MAX {
        let step = world_difficult_terrain_step(tile);
        match tile {
            0x04 | 0x06 | 0x07 | 0x08 | 0x1E | 0x1F => {
                let step = step.expect("slow row");
                assert_eq!(step.extra_object_update_calls, 1, "{tile:#04x}");
                assert_eq!(step.extra_minutes, 2, "{tile:#04x}");
                assert_eq!(step.line.trim_end(), "Slow progress!", "{tile:#04x}");
            }
            0x09..=0x0F => {
                let step = step.expect("very slow row");
                assert_eq!(step.extra_object_update_calls, 2, "{tile:#04x}");
                assert_eq!(step.extra_minutes, 4, "{tile:#04x}");
                assert_eq!(step.line.trim_end(), "Very slow!", "{tile:#04x}");
            }
            _ => assert_eq!(step, None, "{tile:#04x}"),
        }
    }
    assert_eq!(world_slow_movement_line(0x05), None);
}

/// `systems/movement.md §4` "Known top-down movement query families": the
/// dispatcher matches the four-wide family run, so every facing value of a
/// family uses that family's predicate.
#[test]
fn g2_movement_s4_query_families_select_their_predicates() {
    for tile in 0u8..=u8::MAX {
        for query in 0x10u8..=0x13 {
            assert_eq!(tile_class_dispatcher_accepts(tile, query), horse_terrain_accepts(tile));
        }
        for query in 0x14u8..=0x17 {
            assert_eq!(tile_class_dispatcher_accepts(tile, query), carpet_terrain_accepts(tile));
        }
        for query in 0x1Cu8..=0x1F {
            assert_eq!(tile_class_dispatcher_accepts(tile, query), foot_terrain_accepts(tile));
        }
        // Under sail and furled share the ship predicate.
        for query in 0x20u8..=0x27 {
            assert_eq!(tile_class_dispatcher_accepts(tile, query), ship_terrain_accepts(tile));
        }
        for query in 0x28u8..=0x2B {
            assert_eq!(
                tile_class_dispatcher_accepts(tile, query),
                skiff_terrain_accepts(tile, query & 0x03),
                "skiff query {query:#04x} tile {tile:#04x}"
            );
        }
    }
}

fn g2_swamp_state(members: &[(u8, u8)]) -> PlayState {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = BRIT_SWAMP_TILE;
    let mut state = world_state(grid, 5, 5);
    state.player.transport = TransportState::Foot;
    let template = state.party[0];
    state.party.clear();
    state.party_names.clear();
    for (index, (status, dexterity)) in members.iter().enumerate() {
        let mut member = template;
        member.slot = index as u8;
        member.status = *status;
        member.climb_stat = *dexterity;
        member.hp = 200;
        member.max_hp = 200;
        state.party.push(member);
        state.party_names.push(*b"MEMBER\0\0\0");
    }
    state.prng_state = 0x2468;
    state
}

/// `systems/movement.md §8.2`: the outdoor save draws "from the inclusive
/// range `1..30`".
#[test]
fn g2_movement_s8_2_swamp_save_draws_one_through_thirty_inclusive() {
    let mut state = g2_swamp_state(&[(b'G', 10)]);
    let mut seen = [false; 31];
    for _ in 0..4000 {
        let roll = state.outdoor_swamp_poison_roll();
        assert!((1..=30).contains(&roll), "roll {roll}");
        seen[roll as usize] = true;
    }
    assert!(seen[1] && seen[30], "both ends of 1..30 are reachable");
}

/// `systems/movement.md §8.2`: "Dead or already Poisoned | Skip the member
/// without a random draw." The only draw is the trailing world tick.
#[test]
fn g2_movement_s8_2_swamp_save_skips_dead_and_poisoned_without_a_draw() {
    let mut state = g2_swamp_state(&[(b'D', 0), (b'P', 0)]);
    let mut reference = state.clone();
    let _ = reference.advance_visual_tick();

    state.append_world_status_tile_message(WorldPlane::Underworld);

    assert_eq!(state.prng_state, reference.prng_state);
    assert_eq!(state.party[0].status, b'D');
    assert_eq!(state.party[1].status, b'P');
}

/// `systems/movement.md §8.2`: "Roll no greater than Dexterity | Keep the
/// member's status"; "Roll greater than Dexterity | Change status to
/// Poisoned". A Sleeping member is eligible ("Any other status, including
/// Sleeping").
#[test]
fn g2_movement_s8_2_swamp_save_compares_the_roll_with_dexterity() {
    let probe = g2_swamp_state(&[(b'S', 0)]);
    let roll = probe.clone().outdoor_swamp_poison_roll();

    // Dexterity equal to the roll saves.
    let mut saved = g2_swamp_state(&[(b'S', roll)]);
    saved.append_world_status_tile_message(WorldPlane::Underworld);
    assert_eq!(saved.party[0].status, b'S');

    // One below the roll fails, and the sleeper becomes Poisoned.
    let mut failed = g2_swamp_state(&[(b'S', roll - 1)]);
    failed.append_world_status_tile_message(WorldPlane::Underworld);
    assert_eq!(failed.party[0].status, b'P');
}

/// `systems/movement.md §8.2`: "Each newly poisoned member produces one bare
/// status line ... Multiple failures produce multiple lines in party order",
/// and the draws are taken "separately for each eligible member in party
/// order".
#[test]
fn g2_movement_s8_2_each_failed_member_prints_one_bare_line() {
    let mut state = g2_swamp_state(&[(b'G', 0), (b'D', 0), (b'G', 0)]);
    state.append_world_status_tile_message(WorldPlane::Underworld);
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[1].status, b'D');
    assert_eq!(state.party[2].status, b'P');
    let lines = transcript_texts(&state)
        .into_iter()
        .filter(|row| row == OUTDOOR_SWAMP_POISONED_LINE)
        .count();
    assert_eq!(lines, 2);
}

// ---------------------------------------------------------------------------
// systems/stats-panel.md
// ---------------------------------------------------------------------------

fn g2_panel_lines(state: &PlayState) -> Vec<String> {
    render_stats_panel(state, state.active_player)
        .lines()
        .map(str::to_string)
        .collect()
}

/// `systems/stats-panel.md §4`: "If a row index is at or beyond the current
/// travelling-party size, the row is filled with **fifteen spaces** and
/// nothing else is drawn."
#[test]
fn g2_stats_panel_s4_absent_party_rows_are_fifteen_spaces() {
    let state = test_state(open_grid(), 1, 1);
    assert_eq!(state.party.len(), 1);
    let lines = g2_panel_lines(&state);
    for row in &lines[1..STATS_PANEL_PARTY_ROWS] {
        assert_eq!(row, &" ".repeat(15));
    }
}

/// `systems/stats-panel.md §4` worked example: name padded to nine cells, a
/// blank marker cell, HP right-justified in four cells, then the status
/// letter.
#[test]
fn g2_stats_panel_s4_party_row_field_layout() {
    let mut state = test_state(open_grid(), 1, 1);
    state.party_names[0] = *b"BAFF\0\0\0\0\0";
    state.party[0].hp = 60;
    state.party[0].status = b'G';
    let lines = g2_panel_lines(&state);
    assert_eq!(lines[0], format!("{:<9}{}{:>4}{}", "BAFF", ' ', 60, 'G'));
    assert_eq!(lines[0], "BAFF        60G");
}

/// `systems/stats-panel.md §6`: zero to three ladder spaces, then the ` G:`
/// label, so "for every gold value from 0 to 9999 ... the **last digit sits
/// in column 38**", and the worked examples `5`, `150` and `9999`.
#[test]
fn g2_stats_panel_s6_gold_ladder_keeps_the_last_digit_in_column_38() {
    let column = |row: &str, absolute: usize| row.chars().nth(absolute - 24).unwrap();
    for (gold, expected_middle) in [
        (0u16, "    G:0"),
        (5, "    G:5"),
        (150, "  G:150"),
        (9999, " G:9999"),
    ] {
        let mut state = test_state(open_grid(), 1, 1);
        state.food = 63;
        state.gold = gold;
        let counter = g2_panel_lines(&state)[STATS_PANEL_PARTY_ROWS].clone();
        assert_eq!(counter.chars().count(), 15, "{gold}");
        assert_eq!(&counter[8..], expected_middle, "{gold}");
        let last = gold.to_string().chars().last().unwrap();
        assert_eq!(column(&counter, 38), last, "{gold}");
    }
}

/// `systems/stats-panel.md §6` "Food": `F:` in columns 24..25, the counter
/// "at its natural decimal width, with no field padding", then spaces until
/// column 32, "so the food group always occupies columns 24..31".
#[test]
fn g2_stats_panel_s6_food_group_fills_columns_24_to_31() {
    for (food, group) in [(0u16, "F:0     "), (63, "F:63    "), (9999, "F:9999  ")] {
        let mut state = test_state(open_grid(), 1, 1);
        state.food = food;
        state.gold = 150;
        let counter = g2_panel_lines(&state)[STATS_PANEL_PARTY_ROWS].clone();
        assert_eq!(&counter[..8], group, "{food}");
    }
}

/// `systems/stats-panel.md §7` worked examples: three fixed spaces, a fourth
/// only when both month and day are below ten, then `M-D-YYY` with the year
/// zero-padded to three digits.
#[test]
fn g2_stats_panel_s7_date_row_leading_space_rule() {
    for (month, day, expected) in [
        (4u8, 5u8, "    4-5-139    "),
        (12, 25, "   12-25-139   "),
        (12, 5, "   12-5-139    "),
        (4, 15, "   4-15-139    "),
    ] {
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::with_date(139, month, day, 9, 0).unwrap();
        let lines = g2_panel_lines(&state);
        assert_eq!(lines[STATS_PANEL_PARTY_ROWS + 1], expected, "{month}-{day}");
    }
    let mut state = test_state(open_grid(), 1, 1);
    state.clock = GameClock::with_date(7, 4, 5, 9, 0).unwrap();
    assert_eq!(g2_panel_lines(&state)[STATS_PANEL_PARTY_ROWS + 1].trim(), "4-5-007");
}

/// `systems/stats-panel.md §2.2.1` fact 2: "An immediate repaint neither
/// tests nor clears the pending request; only the four mode-loop consumers
/// clear it."
#[test]
fn g2_stats_panel_s2_2_1_immediate_repaint_leaves_the_request_pending() {
    let mut state = test_state(open_grid(), 1, 1);
    state.request_stats_panel_refresh();
    state.repaint_stats_panel();
    assert!(state.stats_panel_refresh_requested);
}

/// `systems/stats-panel.md §2.3`: each consumer does "test the request,
/// refresh the whole panel if it is set, clear it"; a clear request paints
/// nothing (§2.4: "the next command prompt finds the request clear and paints
/// nothing").
#[test]
fn g2_stats_panel_s2_3_drain_repaints_only_when_requested() {
    let mut state = test_state(open_grid(), 1, 1);
    state.repaint_stats_panel();
    let painted = state.stats_panel.counter_row.clone();
    state.gold = if state.gold == 1 { 2 } else { 1 };

    state.drain_stats_panel_refresh();
    assert_eq!(state.stats_panel.counter_row, painted, "no request, no repaint");

    state.request_stats_panel_refresh();
    state.drain_stats_panel_refresh();
    assert_ne!(state.stats_panel.counter_row, painted);
    assert!(!state.stats_panel_refresh_requested, "the consumer clears it");
}

// ---------------------------------------------------------------------------
// formats/saved-gam.md
// ---------------------------------------------------------------------------

/// `formats/saved-gam.md §4`: the stats panel reads the transport marker "to
/// choose the ship-hull middle-counter presentation when the marker is in the
/// ship family `0x20..0x27`. Marker values outside that ship family do not
/// select the stats-panel hull counter."
#[test]
fn g2_saved_gam_s4_only_ship_markers_select_the_hull_counter() {
    for marker in 0u8..=u8::MAX {
        let expected = if (0x20..=0x27).contains(&marker) {
            StatsPanelMiddleCounter::ShipHullCondition
        } else {
            StatsPanelMiddleCounter::PartyGold
        };
        assert_eq!(stats_panel_middle_counter(marker), expected, "{marker:#04x}");
    }
}

/// `formats/saved-gam.md §3.1`/`§3.3`: `+0x1F` is the record's own
/// inn-lodging marker, and "The last record's marker is the final byte of
/// the roster, so no inventory byte is ever read as a marker."
#[test]
fn g2_saved_gam_s3_3_inn_marker_is_each_records_last_byte() {
    assert_eq!(SAVE_CHARACTER_INN_MARKER_OFFSET, 0x1F);
    let last_marker =
        SAVE_ROSTER_OFFSET + (SAVE_ROSTER_SLOT_COUNT - 1) * SAVE_CHARACTER_RECORD_LEN
            + SAVE_CHARACTER_INN_MARKER_OFFSET;
    assert_eq!(last_marker, 0x0201);
    assert_eq!(last_marker + 1, SAVE_FOOD_OFFSET);
}

/// `formats/saved-gam.md §9.2`: two back-to-back per-location tables of
/// thirty-two four-byte slots, removed at `0x05B4`, name-known at `0x0634`,
/// "ending flush against the active-object table at `0x06B4`".
#[test]
fn g2_saved_gam_s9_2_npc_mask_tables_meet_without_a_gap() {
    assert_eq!(SAVE_NPC_MASK_SCENE_COUNT, 32);
    assert_eq!(SAVE_NPC_MASK_BYTES_PER_SCENE, 4);
    assert_eq!(SAVE_NPC_REMOVED_MASKS_OFFSET, 0x05B4);
    assert_eq!(
        SAVE_NPC_REMOVED_MASKS_OFFSET + SAVE_NPC_MASK_BANK_LEN,
        SAVE_NPC_NAME_KNOWN_MASKS_OFFSET
    );
    assert_eq!(SAVE_NPC_NAME_KNOWN_MASKS_OFFSET, 0x0634);
    assert_eq!(
        SAVE_NPC_NAME_KNOWN_MASKS_OFFSET + SAVE_NPC_MASK_BANK_LEN,
        SAVE_ACTIVE_OBJECT_TABLE_OFFSET
    );
    assert_eq!(SAVE_ACTIVE_OBJECT_TABLE_OFFSET, 0x06B4);
}

/// `formats/saved-gam.md §12.1`: "Each runtime record contains eight words"
/// at record offsets `0, 2, ... 14`: state, X, Y, floor, type mirror,
/// dialogue index, linked object (zero when none) and cached waypoint. The
/// band opens and closes with four and two opaque bytes; slot zero is
/// reserved.
#[test]
fn g2_saved_gam_s12_1_runtime_record_word_layout() {
    assert_eq!(SAVE_NPC_BAND_RUNTIME_WORD_COUNT, 8);
    assert_eq!(SAVE_NPC_BAND_RUNTIME_RECORD_LEN, 16);
    assert_eq!(
        [
            SAVE_NPC_BAND_RUNTIME_STATE_OFFSET,
            SAVE_NPC_BAND_RUNTIME_X_OFFSET,
            SAVE_NPC_BAND_RUNTIME_Y_OFFSET,
            SAVE_NPC_BAND_RUNTIME_Z_OFFSET,
            SAVE_NPC_BAND_RUNTIME_TYPE_MIRROR_OFFSET,
            SAVE_NPC_BAND_RUNTIME_DIALOGUE_OFFSET,
            SAVE_NPC_BAND_RUNTIME_LINKED_OBJECT_OFFSET,
            SAVE_NPC_BAND_RUNTIME_CACHED_WP_OFFSET,
        ],
        [0, 2, 4, 6, 8, 10, 12, 14]
    );
    assert_eq!(SAVE_NPC_BAND_RUNTIME_LINKED_OBJECT_NONE, 0);
    assert_eq!(SAVE_NPC_BAND_OPAQUE_HEAD_LEN, 4);
    assert_eq!(SAVE_NPC_BAND_OPAQUE_TAIL_LEN, 2);
    assert_eq!(SAVE_NPC_BAND_RESERVED_SLOT, 0);
}

/// `formats/saved-gam.md §12.2`: "A route buffer holds up to sixteen two-byte
/// runs"; the cursor is a byte offset and `0xFFFF` means inactive.
#[test]
fn g2_saved_gam_s12_2_route_buffer_is_sixteen_two_byte_runs() {
    assert_eq!(SAVE_NPC_BAND_ROUTE_RUN_LEN, 2);
    assert_eq!(SAVE_NPC_BAND_ROUTE_MAX_RUNS, 16);
    assert_eq!(
        SAVE_NPC_BAND_ROUTE_MAX_RUNS * SAVE_NPC_BAND_ROUTE_RUN_LEN,
        SAVE_NPC_BAND_ROUTE_BUFFER_LEN
    );
    assert_eq!(SAVE_NPC_BAND_ROUTE_CURSOR_INACTIVE, 0xFFFF);
}

// ---------------------------------------------------------------------------
// systems/animation.md
// ---------------------------------------------------------------------------

/// `systems/animation.md §6.1`: the selector table "is initialised to the
/// identity map at startup, and after that only the five-family pass ever
/// writes it. An id outside the five windows therefore draws as itself for
/// the entire program run" - including the hourglass `0xE8..0xEB` (§6.2) and
/// the water, lava and fire ids that animate only driver-side (§6, §12).
#[test]
fn g2_animation_s6_1_ids_outside_the_five_windows_never_change() {
    let in_window = |tile: u8| {
        matches!(tile, 0xD4..=0xD7 | 0xD8..=0xDB | 0x80..=0x83 | 0xEC..=0xEF | 0xFA..=0xFD)
    };
    for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
        let clock = AnimationClock::at_static_tile_phase(phase);
        for tile in 0u8..=u8::MAX {
            if in_window(tile) {
                assert!(static_tile_animation_family(tile).is_some(), "{tile:#04x}");
                continue;
            }
            assert_eq!(static_tile_animation_family(tile), None, "{tile:#04x}");
            assert_eq!(clock.resolve_static_tile(tile), tile, "{tile:#04x} phase {phase}");
        }
    }
}

/// `systems/animation.md §6`: "The counter is incremented once at the end of
/// the pass, whichever path was taken" - both the bit-0-clear early exit and
/// the full gated path advance it by exactly one.
#[test]
fn g2_animation_s6_counter_advances_once_per_pass_on_every_path() {
    for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
        let mut clock = AnimationClock::at_static_tile_phase(phase);
        clock.tick_static_tiles();
        assert_eq!(
            clock,
            AnimationClock::at_static_tile_phase(phase + 1),
            "phase {phase}"
        );
    }
}

// ---------------------------------------------------------------------------
// systems/shops.md
// ---------------------------------------------------------------------------

/// `systems/shops.md §6`: inn, horse, shipwright, provision, guild and
/// reagent quotes use `raw + trunc(raw * (100 - 3 * Intelligence) / 100)`,
/// and "The division truncates toward zero" - including the negative
/// adjustment of a speaker above Intelligence 33.
#[test]
fn g2_shops_s6_intelligence_adjustment_truncates_toward_zero() {
    // Positive adjustment: 7 * (100 - 60) / 100 = 2.8 -> 2.
    assert_eq!(shop_intelligence_adjusted_price(7, 20), 9);
    // Negative adjustment: 7 * (100 - 120) / 100 = -1.4 -> -1, not -2.
    assert_eq!(shop_intelligence_adjusted_price(7, 40), 6);
    // 100 * (100 - 99) / 100 = 1.
    assert_eq!(shop_intelligence_adjusted_price(100, 33), 101);
    // 10 at Intelligence 20: the §8.5 worked unit price of 14.
    assert_eq!(shop_intelligence_adjusted_price(10, 20), 14);
}

/// `systems/shops.md §6`: guild offers are "bundles of three keys, four
/// gems, or five torches".
#[test]
fn g2_shops_s6_guild_bundle_quantities() {
    assert_eq!(guild_bundle_quantity(GuildCommodity::Keys), 3);
    assert_eq!(guild_bundle_quantity(GuildCommodity::Gems), 4);
    assert_eq!(guild_bundle_quantity(GuildCommodity::Torches), 5);
}

/// `systems/shops.md §6`: the reagent bundle-quantity matrix, every cell,
/// with unavailable cells (`—`) in the same places as the price matrix.
#[test]
fn g2_shops_s6_reagent_bundle_quantity_matrix() {
    use Reagent::*;
    let order = [
        SulfurAsh, Ginseng, Garlic, SpiderSilk, BloodMoss, BlackPearl, Nightshade, Mandrake,
    ];
    let rows: [(Herbalist, [Option<u8>; 8]); 5] = [
        (
            Herbalist::TheHerbalist,
            [None, Some(10), Some(6), Some(4), None, None, Some(1), Some(1)],
        ),
        (
            Herbalist::HealersHerbs,
            [Some(12), Some(8), Some(8), Some(2), Some(4), None, None, None],
        ),
        (
            Herbalist::TheAlchemist,
            [Some(14), Some(8), None, None, Some(6), Some(6), None, None],
        ),
        (
            Herbalist::Mysticism,
            [None, None, None, Some(2), Some(2), Some(2), Some(1), Some(1)],
        ),
        (
            Herbalist::TheSharperMage,
            [None, None, None, None, Some(4), None, Some(1), Some(1)],
        ),
    ];
    for (herbalist, quantities) in rows {
        for (reagent, quantity) in order.iter().zip(quantities) {
            assert_eq!(
                herbalist_reagent_bundle_quantity(herbalist, *reagent),
                quantity,
                "{herbalist:?} {reagent:?}"
            );
            assert_eq!(
                herbalist_reagent_price(herbalist, *reagent).is_some(),
                quantity.is_some(),
                "{herbalist:?} {reagent:?} availability"
            );
        }
    }
}

/// `systems/shops.md §8.B`: after a non-arms entry greeting the input
/// continuation depends on the cursor column: `0` -> `\n:`, `1..11` -> one
/// space and no colon, `12..15` -> `\n\n:`.
#[test]
fn g2_shops_s8_b_entry_continuation_follows_the_cursor_column() {
    assert_eq!(crate::shoppe_records::shop_entry_input_continuation(0), "\n:");
    for column in 1..=11 {
        assert_eq!(crate::shoppe_records::shop_entry_input_continuation(column), " ");
    }
    for column in 12..=15 {
        assert_eq!(crate::shoppe_records::shop_entry_input_continuation(column), "\n\n:");
    }
}

/// `systems/shops.md §6.1`/`§8.5`: "The random post-transaction surcharge ...
/// runs on the completed-purchase exit only. A partially served purchase -
/// one that ended because gold ran out - returns without charging it"; the
/// nothing-served charity ending is not a completed purchase either.
#[test]
fn g2_shops_s6_1_provision_surcharge_only_on_the_completed_exit() {
    assert!(ProvisionPurchaseCompletion::Completed.surcharge_applies());
    assert!(!ProvisionPurchaseCompletion::GoldExhausted.surcharge_applies());
    assert!(!ProvisionPurchaseCompletion::Charity.surcharge_applies());
}

fn g2_shopkeeper_state(dialog_id: u8, times: [u8; 4]) -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(2).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    let mut schedule = [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
    schedule[12..16].copy_from_slice(&times);
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id,
            schedule,
            name: None,
        },
    ]);
    state
}

/// Hour 12 with these boundaries selects waypoint 2, so the keeper is not at
/// the working waypoint 1.
const G2_CLOSED_TIMES: [u8; 4] = [0, 8, 11, 20];
/// Hour 12 with these boundaries selects waypoint 1.
const G2_OPEN_TIMES: [u8; 4] = [0, 8, 16, 20];

/// `systems/shops.md §2` "Open-for-business gate": both the cached reached
/// waypoint and the hour-selected waypoint must be waypoint 1; "On failure
/// the dispatcher emits one fixed refusal", published verbatim with a
/// newline before the preamble and after the closing quote.
#[test]
fn g2_shops_s2_closed_keeper_prints_the_fixed_merchant_refusal() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x86, G2_CLOSED_TIMES);
    assert_eq!(state.clock.hour, 12);

    state.talk_facing_with_dialogue(&dialogue);

    assert_eq!(
        state.message,
        "\nA merchant says:\n\"Come see me at\nmy shoppe, when\nit's open!\"\n"
    );
    assert!(state.active_shop.is_none());
}

/// `systems/shops.md §2`: "The cache records arrival, not merely the latest
/// hour's destination. Thus a keeper who has not reached the working
/// period's waypoint can refuse even after opening time."
#[test]
fn g2_shops_s2_unreached_working_waypoint_refuses_even_in_hours() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x86, G2_OPEN_TIMES);
    let keeper = state.npcs.iter().position(|npc| npc.slot == 1).unwrap();
    state.npcs[keeper].cached_wp = 0;

    state.talk_facing_with_dialogue(&dialogue);

    assert!(state.message.contains("Come see me at"), "{:?}", state.message);
    assert!(state.active_shop.is_none());
}

/// `systems/shops.md §2`: "Schedule refusal takes precedence over horse
/// refusal".
#[test]
fn g2_shops_s2_schedule_refusal_precedes_the_horse_refusal() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x86, G2_CLOSED_TIMES);
    state.player.transport = TransportState::Horse {
        type_byte: FIRST_PLAYABLE_HORSE_TILE,
        tile: FIRST_PLAYABLE_HORSE_TILE,
    };

    state.talk_facing_with_dialogue(&dialogue);

    assert!(state.message.contains("Come see me at"), "{:?}", state.message);
    assert!(!state.message.contains("HORSE"));
    assert!(state.active_shop.is_none());
}

/// `systems/shops.md §8.4`: "Entering the inn's service menu unconditionally
/// clears the single shared timed-effect slot ... This happens on entry and
/// is not conditional on renting a room or on paying anything."
#[test]
fn g2_shops_s8_4_inn_entry_clears_the_timed_effect_slot() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x88, G2_OPEN_TIMES);
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 20;

    state.talk_facing_with_dialogue(&dialogue);

    assert!(state.active_shop.is_some(), "{:?}", state.message);
    assert_eq!(state.active_effect_tag, None);
}

fn g2_key(state: &mut PlayState, key: char) {
    handle_play_key_input(state, key, "", Path::new("")).unwrap();
}

/// `systems/shops.md §8.C` "Arms sell refusals": "Empty Sell inherits the
/// opening quote from `Sell\n\n"`, then prints token-expanded
/// `Thou hast nothing to sell!"\ngrowls $.\n`."
#[test]
fn g2_shops_s8_c_empty_sell_is_refused_with_the_growled_line() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x81, G2_OPEN_TIMES);
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.talk_facing_with_dialogue(&dialogue);
    // §8.B stage 2: one key at the pause, then the Buy/Sell question.
    g2_key(&mut state, 'x');
    g2_key(&mut state, 'S');
    let rows = transcript_texts(&state);
    let tail = &rows[rows.len() - 3..];
    assert!(rows[rows.len() - 4].ends_with("Sell"), "{rows:?}");
    assert_eq!(
        tail,
        ["", "\"Thou hast nothing to sell!\"", "growls Gwenneth."],
        "{rows:?}"
    );
}

/// `systems/shops.md §8.B`: healer Yes prints `Yes\n\n`, then
/// `"We have powers to Cure, Heal, or Resurrect."\n`, then token-expanded
/// `says $.\n\n"What is the nature of thy need?" `; §8.0 names the Trinsic
/// healer vendor Leila.
#[test]
fn g2_shops_s8_b_healer_yes_prints_the_powers_line() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x87, G2_OPEN_TIMES);
    state.area = Area::Town {
        scene: Scene::new(6).unwrap(),
        floor: 0,
    };
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    assert!(
        state.message.ends_with(
            "\"We have powers to Cure, Heal, or Resurrect.\"\nsays Leila.\n\n\"What is the nature of thy need?\" "
        ),
        "{:?}",
        state.message
    );
}

const G2_INN_CONTINUATION: &str = "\"Is there\nanything more\nI can do for\nthee?\" ";

/// `systems/shops.md §8.C` "Pickup with no guest at this inn": token-expanded
/// `\n\n"No one here is from thy party!"\nsays $.\n\n`, then the inn's
/// continuation question.
#[test]
fn g2_shops_s8_c_pickup_with_no_local_guest_is_refused() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x88, G2_OPEN_TIMES);
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    g2_key(&mut state, 'P');
    assert_eq!(
        state.message,
        format!("\n\n\"No one here is from thy party!\"\nsays Donya.\n\n{G2_INN_CONTINUATION}")
    );
}

fn g2_member(slot: u8, status: u8) -> PartyMember {
    PartyMember {
        slot,
        class_byte: b'F',
        status,
        climb_stat: 10,
        mana: 0,
        hp: 30,
        max_hp: 30,
        level: 1,
    }
}

/// `systems/shops.md §8.C` "Pickup with six travelling members":
/// `\n\nOne must first be left behind!\n\n`; "this check precedes the
/// no-guests check" (there is no guest here either).
#[test]
fn g2_shops_s8_c_pickup_with_a_full_party_is_refused_first() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x88, G2_OPEN_TIMES);
    while state.party.len() < 6 {
        let slot = state.party.len() as u8;
        state.party.push(g2_member(slot, b'G'));
        state.party_names.push(*b"MEMBER\0\0\0");
    }
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    g2_key(&mut state, 'P');
    assert_eq!(
        state.message,
        format!("\n\nOne must first be left behind!\n\n{G2_INN_CONTINUATION}")
    );
}

/// `systems/shops.md §8.4`: "If the stored guest status is Poisoned when the
/// guest is picked up, the pickup path converts the returned record to Dead,
/// clears current hit points, and prints "Thy friend has died, by the way.""
/// The single local guest is selected without a picker or confirmation, and
/// the bill is `adjusted(base_rate * 10, Intelligence)` times the stay
/// counter with zero billed as one (§6, §8.C "Pickup bill").
#[test]
fn g2_shops_s8_4_poisoned_guest_is_returned_dead() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x88, G2_OPEN_TIMES);
    state.gold = 100;
    state.party_intelligence = vec![20];
    state.inn_registry.push(InnGuestRecord {
        registry_slot: 15,
        scene_marker: 2,
        name: *b"GUEST\0\0\0\0",
        member: g2_member(15, b'P'),
        strength: 10,
        intelligence: 10,
        experience: 0,
        equipment: [0xFF; EQUIPMENT_SLOT_COUNT],
        stay_counter: 0,
    });
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    g2_key(&mut state, 'P');

    // The Wayfarer Inn's base rate is 2: raw 20, adjusted at Intelligence 20
    // to 20 + trunc(20 * 40 / 100) = 28, times one billable unit.
    assert!(
        state.message.contains("\"That will be 28 gold, please.\"\n\n\""),
        "{:?}",
        state.message
    );
    assert!(
        state.message.contains("Thy friend has died, by the way.\"\nsays Donya.\n\n"),
        "{:?}",
        state.message
    );
    assert_eq!(state.gold, 72);
    assert_eq!(state.party.len(), 2);
    assert_eq!(state.party[1].status, b'D');
    assert_eq!(state.party[1].hp, 0);
}

/// `systems/shops.md §8.B` arms entry: after the pause key, `\n$ says,\n"`,
/// one of two equal-probability greetings, then `" `.
#[test]
fn g2_shops_s8_b_arms_greeting_is_one_of_two_variants() {
    let dialogue = HashMap::new();
    let variants = [
        "Hail, friend! Wouldst thou Buy or Sell?",
        "Greetings, traveller! Wish ye to Buy, or hast thou wares to Sell?",
    ];
    let mut seen = [false; 2];
    for seed in 1u16..=64 {
        let mut state = g2_shopkeeper_state(0x81, G2_OPEN_TIMES);
        state.prng_state = seed.wrapping_mul(0x2545);
        state.talk_facing_with_dialogue(&dialogue);
        g2_key(&mut state, 'x');
        let index = variants
            .iter()
            .position(|variant| state.message == format!("\nGwenneth says,\n\"{variant}\" "))
            .unwrap_or_else(|| panic!("seed {seed}: {:?}", state.message));
        seen[index] = true;
    }
    assert_eq!(seen, [true, true]);
}

/// `systems/shops.md §8.B`: "On Buy, the echo is followed by one uniformly
/// selected affirmation from `Very good!\n`, `Excellent!\n`, `Fine, fine!\n`,
/// `But of course!\n`, then one independently selected stock introduction
/// from `We have:`, `We stock:`, `Thou canst buy:`, `We've got:`", before the
/// listing's two leading line feeds.
#[test]
fn g2_shops_s8_b_buy_heading_draws_from_both_published_pools() {
    let dialogue = HashMap::new();
    let affirmations = ["Very good!", "Excellent!", "Fine, fine!", "But of course!"];
    let introductions = ["We have:", "We stock:", "Thou canst buy:", "We've got:"];
    let mut seen_affirmation = [false; 4];
    let mut seen_introduction = [false; 4];
    for seed in 1u16..=200 {
        let mut state = g2_shopkeeper_state(0x81, G2_OPEN_TIMES);
        state.prng_state = seed.wrapping_mul(0x3D09);
        state.talk_facing_with_dialogue(&dialogue);
        g2_key(&mut state, 'x');
        g2_key(&mut state, 'B');
        let body = state.message.strip_prefix('"').expect("the Buy echo's open quote");
        let (affirmation, rest) = body.split_once('\n').unwrap();
        let (introduction, rest) = rest.split_once('\n').unwrap();
        assert!(rest.starts_with("\na..."), "seed {seed}: {:?}", state.message);
        let a = affirmations.iter().position(|text| *text == affirmation);
        let b = introductions.iter().position(|text| *text == introduction);
        seen_affirmation[a.unwrap_or_else(|| panic!("{affirmation:?}"))] = true;
        seen_introduction[b.unwrap_or_else(|| panic!("{introduction:?}"))] = true;
    }
    assert_eq!(seen_affirmation, [true; 4]);
    assert_eq!(seen_introduction, [true; 4]);
}

/// `systems/shops.md §8.1`: after `Sold!` the arms flow prints "the post-item
/// prompt `"Anything else,\n` followed by `milady?` when the speaking
/// member's gender field is the female value and `sir?` otherwise".
#[test]
fn g2_shops_s8_1_post_item_prompt_breaks_the_line_after_the_comma() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x81, G2_OPEN_TIMES);
    state.gold = 5000;
    state.talk_facing_with_dialogue(&dialogue);
    for key in ['x', 'B', 'a', 'Y'] {
        g2_key(&mut state, key);
    }
    assert!(
        state.message.contains("Sold!\n\"Anything else,\nsir?"),
        "{:?}",
        state.message
    );
}

/// `systems/shops.md §8.C` healer: "Paid Heal introduction |
/// `\n\n"I can heal thee `", then "Fee and confirmation | Token-expanded
/// `for % gold.\n\nWilt thou\npay?" `".
#[test]
fn g2_shops_s8_c_heal_offer_wording() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x87, G2_OPEN_TIMES);
    state.area = Area::Town {
        scene: Scene::new(6).unwrap(),
        floor: 0,
    };
    state.party[0].hp = 10;
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    g2_key(&mut state, 'H');
    let fee = state
        .message
        .strip_prefix("\n\n\"I can heal thee for ")
        .and_then(|rest| rest.strip_suffix(" gold.\n\nWilt thou\npay?\" "))
        .unwrap_or_else(|| panic!("{:?}", state.message));
    assert!(fee.parse::<u16>().is_ok(), "{fee:?}");
}

/// `systems/shops.md §8.C` "Leave rate": after `\n\n`, `"The rate for\nour
/// most comfortable room will be `, token-expanded `% gold per month, due at
/// check-out.`, then `\nWilt thou take\nit?" `; the rate is
/// `adjusted(base_rate * 10, speaker_intelligence)` (§8.4).
#[test]
fn g2_shops_s8_c_leave_quotes_the_adjusted_monthly_rate() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x88, G2_OPEN_TIMES);
    state.party_intelligence = vec![20, 20];
    state.party.push(g2_member(1, b'G'));
    state.party_names.push(*b"FRIEND\0\0\0");
    state.talk_facing_with_dialogue(&dialogue);
    for key in ['Y', 'L', '2', '\r'] {
        g2_key(&mut state, key);
    }
    // Wayfarer base 2: raw 20, adjusted at Intelligence 20 to 28.
    assert_eq!(
        state.message,
        "\n\n\"The rate for\nour most comfortable room will be 28 gold per month, due at check-out.\nWilt thou take\nit?\" "
    );
}

/// `systems/shops.md §8.C` "Capacity check before Rest or Leave": "First
/// `\n\n`. If full: `"I am sorry,\n`, honorific, then `, but we\nhave no
/// room\navailable."\n\n`", with the Wayfarer Inn's limit of three lodged
/// guests (§8.4 rate table) and `sir` for a male speaker.
#[test]
fn g2_shops_s8_c_full_inn_refuses_rest() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x88, G2_OPEN_TIMES);
    for slot in 13..16u8 {
        state.inn_registry.push(InnGuestRecord {
            registry_slot: slot,
            scene_marker: 2,
            name: *b"GUEST\0\0\0\0",
            member: g2_member(slot, b'G'),
            strength: 10,
            intelligence: 10,
            experience: 0,
            equipment: [0xFF; EQUIPMENT_SLOT_COUNT],
            stay_counter: 0,
        });
    }
    let gold = state.gold;
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    g2_key(&mut state, 'R');
    assert!(
        state
            .message
            .starts_with("\n\n\"I am sorry,\nsir, but we\nhave no room\navailable.\"\n\n"),
        "{:?}",
        state.message
    );
    assert_eq!(state.gold, gold);
}

/// `systems/shops.md §8.B`/`§8.C`: reagent Yes prints `"Fine! We sell:\n\n`,
/// then each stocked reagent as an uppercase compact letter, `...` and its
/// short name, letters consecutive with unavailable reagents omitted, then
/// `\nThy interest?" `. Moonglow's The Herbalist stocks the five reagents of
/// its §6 matrix row.
#[test]
fn g2_shops_s8_c_reagent_stock_list_layout() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x85, G2_OPEN_TIMES);
    state.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    assert_eq!(
        state.message,
        "\"Fine! We sell:\n\nA...Ginseng\nB...Garlic\nC...Spider Silk\nD...Nightshade\nE...Mandrake\n\nThy interest?\" "
    );
}

/// `systems/shops.md §8.B`/`§8.C` guild: `Yes\n\n"We sell:\n\n`, then
/// `a.........Keys\n`, `b.........Gems\n`, `c......Torches\n\n`, then
/// `Thy concern?" `; each row is fourteen columns.
#[test]
fn g2_shops_s8_c_guild_stock_rows_are_fourteen_columns() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x86, G2_OPEN_TIMES);
    state.area = Area::Town {
        scene: Scene::new(8).unwrap(),
        floor: 0,
    };
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    assert_eq!(
        state.message,
        "\"We sell:\n\na.........Keys\nb.........Gems\nc......Torches\n\nThy concern?\" "
    );
    for row in ["a.........Keys", "b.........Gems", "c......Torches"] {
        assert_eq!(row.len(), 14);
    }
}

fn g2_horse_sale_state() -> PlayState {
    let mut state = g2_shopkeeper_state(0x83, G2_OPEN_TIMES);
    state.area = Area::Town {
        scene: Scene::new(6).unwrap(),
        floor: 0,
    };
    // South (1, 2) and north (1, 0) both hold placement tiles; south is
    // probed first.
    state.grid[2 * 32 + 1] = 0x44;
    state.grid[1] = 0x05;
    state.gold = 1000;
    state.party_intelligence = vec![20];
    state
}

/// `systems/shops.md §3`/`§6`/`§8.10`: Trinsic's stable quotes base `100`
/// adjusted for the speaker (`100 + trunc(100 * 40 / 100) = 140` at
/// Intelligence 20); the placement probe runs "south, north, east, west" and
/// accepts tiles `0x44`, `0x45`, `0x05`; the horse record carries the base
/// horse tile `0x10` in both type and frame bytes, the probe cell, the
/// party's floor and zero auxiliary bytes.
#[test]
fn g2_shops_s8_10_horse_sale_places_the_horse_at_the_first_probe_cell() {
    let dialogue = HashMap::new();
    let mut state = g2_horse_sale_state();
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    assert!(state.message.contains("140"), "{:?}", state.message);
    g2_key(&mut state, 'Y');
    assert_eq!(state.gold, 860);
    let horse = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == 0x10)
        .copied()
        .expect("a horse object was written");
    assert_eq!(horse.tile, 0x10);
    assert_eq!((horse.x, horse.y), (1, 2), "south wins over north");
    assert_eq!(horse.z, 0);
    assert_eq!((horse.aux1, horse.aux3), (0, 0));
}

/// `systems/shops.md §8.10`: the sale writes "all three auxiliary/animation
/// bytes cleared to zero" - record bytes `+5`, `+6` and `+7`.
#[test]
fn g2_shops_s8_10_horse_sale_clears_the_animation_byte() {
    let dialogue = HashMap::new();
    let mut state = g2_horse_sale_state();
    state.talk_facing_with_dialogue(&dialogue);
    g2_key(&mut state, 'Y');
    g2_key(&mut state, 'Y');
    let horse = state
        .active_objects
        .iter()
        .find(|object| object.type_byte == 0x10)
        .copied()
        .expect("a horse object was written");
    assert_eq!(horse.phase, 0);
}

/// `systems/stats-panel.md §9`: the label strip's opening cap goes in column
/// `30 - (L / 2)`, the label in the next `L` columns and the closing cap
/// after it; `Select:` puts the caps in 27 and 35 with text in 28..34, an
/// odd label centres on 31 and an even one straddles 30 and 31.
#[test]
fn g2_stats_panel_s9_label_strip_is_centred_by_measurement() {
    assert_eq!(crate::gameplay_chrome::stats_label_strip_span("Select:".len()), (27, 28, 35));
    assert_eq!(crate::gameplay_chrome::stats_label_strip_span("Items:".len()), (27, 28, 34));
    for length in 1usize..=13 {
        let (open, first, close) = crate::gameplay_chrome::stats_label_strip_span(length);
        assert_eq!(usize::from(open), 30 - length / 2);
        assert_eq!(usize::from(first), usize::from(open) + 1);
        assert_eq!(usize::from(close), usize::from(open) + length + 1);
        let last = usize::from(first) + length - 1;
        if length % 2 == 1 {
            assert_eq!(usize::from(first) + last, 62, "odd label centred on 31");
        } else {
            assert_eq!(usize::from(first) + last, 61, "even label straddles 30/31");
        }
    }
}

/// `systems/stats-panel.md §8` effect-code table: `P`, `Q`, `C`, `N`, `T`
/// letters and the `0x0E`, `0x1C`, `0x1D` regalia pictograms.
#[test]
fn g2_stats_panel_s8_timed_effect_codes() {
    assert_eq!(PROTECTION_ACTIVE_EFFECT_TAG, b'P');
    assert_eq!(QUICKNESS_ACTIVE_EFFECT_TAG, b'Q');
    assert_eq!(MASS_CHARM_ACTIVE_EFFECT_TAG, b'C');
    assert_eq!(NEGATE_MAGIC_ACTIVE_EFFECT_TAG, b'N');
    assert_eq!(NEGATE_TIME_ACTIVE_EFFECT_TAG, b'T');
    assert_eq!(AMULET_LB_ACTIVE_EFFECT_TAG, 0x0E);
    assert_eq!(CROWN_LB_ACTIVE_EFFECT_TAG, 0x1C);
    assert_eq!(BLACK_BADGE_ACTIVE_EFFECT_TAG, 0x1D);
}

fn g2_sell_state(seed: u16) -> PlayState {
    let mut state = g2_shopkeeper_state(0x81, G2_OPEN_TIMES);
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    // Item id 8 has a zero base price (§8.1 sell-browser table).
    state.equipment_stock[8] = 1;
    state.prng_state = seed;
    state
}

/// `systems/shops.md §8.1` sell browser: "A zero-price selection prints
/// token-expanded `\n\n"That, I cannot buy from thee."\nsays $.`, without a
/// trailing line feed, then takes its normal continuation", which "prints
/// `\n\n"`, its selected continuation question, and `" `"; state is
/// unchanged.
#[test]
fn g2_shops_s8_1_zero_price_sell_selection_is_refused() {
    let dialogue = HashMap::new();
    let mut state = g2_sell_state(0x1111);
    let gold = state.gold;
    state.talk_facing_with_dialogue(&dialogue);
    for key in ['x', 'S', '\r'] {
        g2_key(&mut state, key);
    }
    let continuation = state
        .message
        .strip_prefix("\n\n\"That, I cannot buy from thee.\"\nsays Gwenneth.\n\n\"")
        .and_then(|rest| rest.strip_suffix("\" "))
        .unwrap_or_else(|| panic!("{:?}", state.message));
    assert!(
        [
            "What else can ye offer me?",
            "What else hath ye to sell?",
            "What else doth thou wish to sell?",
            "What other arms wilt thou sell?",
        ]
        .contains(&continuation),
        "{continuation:?}"
    );
    assert_eq!(state.gold, gold);
    assert_eq!(state.equipment_stock[8], 1);
}

/// `systems/shops.md §8.1` "Random draws and waits": nonempty Sell entry draws
/// uniformly over four questions, and each continuation draws uniformly over
/// four more; the entry question appears as `"<question>" `.
#[test]
fn g2_shops_s8_1_sell_prompts_draw_from_the_published_pools() {
    let dialogue = HashMap::new();
    let entries = [
        "Which item wouldst thou like to sell?",
        "What dost thou wish to sell?",
        "Show me what ye got...",
        "What dost thou have for me to buy?",
    ];
    let continuations = [
        "What else can ye offer me?",
        "What else hath ye to sell?",
        "What else doth thou wish to sell?",
        "What other arms wilt thou sell?",
    ];
    let mut seen_entry = [false; 4];
    let mut seen_continuation = [false; 4];
    for seed in 1u16..=200 {
        let mut state = g2_sell_state(seed.wrapping_mul(0x4E35));
        state.talk_facing_with_dialogue(&dialogue);
        g2_key(&mut state, 'x');
        g2_key(&mut state, 'S');
        let entry = state
            .message
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix("\" "))
            .unwrap_or_else(|| panic!("{:?}", state.message));
        seen_entry[entries.iter().position(|text| *text == entry).expect(entry)] = true;
        g2_key(&mut state, '\r');
        let continuation = state
            .message
            .rsplit_once("\n\n\"")
            .and_then(|(_, rest)| rest.strip_suffix("\" "))
            .unwrap_or_else(|| panic!("{:?}", state.message));
        seen_continuation[continuations
            .iter()
            .position(|text| *text == continuation)
            .expect(continuation)] = true;
    }
    assert_eq!(seen_entry, [true; 4]);
    assert_eq!(seen_continuation, [true; 4]);
}

/// `systems/shops.md §2` "Contact and turn accounting": "the shared dispatcher
/// first changes behavior 4 at the cached reached waypoint to behavior 1.
/// This also applies to shopkeepers and happens before either shop refusal";
/// "Explicit Talk, including either refusal, has the ordinary acted outcome".
#[test]
fn g2_shops_s2_refused_talk_still_stands_the_keeper_down_and_takes_a_turn() {
    let dialogue = HashMap::new();
    let mut state = g2_shopkeeper_state(0x86, G2_CLOSED_TIMES);
    let keeper = state.npcs.iter().position(|npc| npc.slot == 1).unwrap();
    let at = NPC_SCHEDULE_AI_OFFSET + state.npcs[keeper].cached_wp;
    state.npcs[keeper].schedule[at] = 4;
    let turn = state.turn;

    state.talk_facing_with_dialogue(&dialogue);

    assert!(state.message.contains("Come see me at"), "{:?}", state.message);
    assert_eq!(state.npcs[keeper].schedule[at], 1);
    assert_eq!(state.turn, turn + 1);
}
