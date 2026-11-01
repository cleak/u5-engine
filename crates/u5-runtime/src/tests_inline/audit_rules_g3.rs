// Closed-audit coverage, group g3. Each test asserts one normative rule from
// `systems/audio.md`, `systems/npc-schedules.md`, `catalogs/item-list.md`,
// `formats/location-dat.md`, `systems/traps.md`,
// `systems/display-driver-mode.md`, `formats/font-pcs.md`,
// `formats/under-dat.md`, `formats/karma-dat.md` or `systems/runtime.md`
// that the engine implements but no earlier test asserted. The spec file and
// section are named in each test's name and comment. Text literals are either
// published by the spec or invented placeholders.

// ---------------------------------------------------------------------------
// formats/karma-dat.md
// ---------------------------------------------------------------------------

#[test]
fn g3_karma_dat_s4_blackthorn_frames_the_record_with_a_leading_feed_and_quotes() {
    // formats/karma-dat.md §4: the consumer adds "a leading line feed and an
    // opening quote before it, one closing quote after it".
    assert_eq!(crate::blackthorn::BLACKTHORN_RESCUE_VERDICT_OPEN, "\n\"");
    assert_eq!(crate::blackthorn::BLACKTHORN_RESCUE_VERDICT_CLOSE, "\"");
}

#[test]
fn g3_karma_dat_s2_parser_requires_no_fixed_record_length_or_file_size() {
    // formats/karma-dat.md §2: "free to index a parsed list of six strings
    // ... provided it ... does not require a fixed record length".
    let mut bytes = Vec::new();
    for record in ["a", "bbbbbbbbbbbbbbbbbbbb", "cc", "", "ddd", "e"] {
        bytes.extend_from_slice(record.as_bytes());
        bytes.push(0);
    }
    assert_ne!(bytes.len(), KARMA_DAT_LEN);
    let records = parse_karma_dat(&bytes).unwrap();
    assert_eq!(records.len(), KARMA_DAT_RECORDS);
    assert_eq!(records[1].len(), 20);
    assert_eq!(records[3], "");
}

// ---------------------------------------------------------------------------
// formats/under-dat.md
// ---------------------------------------------------------------------------

fn g3_under_bytes_with_slot_pattern() -> Vec<u8> {
    // Each stored block carries its own slot number in every byte except the
    // first, which carries a local-offset marker.
    let mut bytes = vec![0u8; UNDER_DAT_LEN];
    for slot in 0..WORLD_CHUNK_COUNT {
        for offset in 0..CHUNK_BYTES {
            bytes[slot * CHUNK_BYTES + offset] = slot as u8;
        }
    }
    bytes
}

#[test]
fn g3_under_dat_s11_decoder_rejects_any_length_but_65536() {
    // formats/under-dat.md §11: "The file length is exactly 65,536 bytes."
    assert!(decode_underworld_map_bytes(&vec![0; UNDER_DAT_LEN - 1]).is_err());
    assert!(decode_underworld_map_bytes(&vec![0; UNDER_DAT_LEN + 1]).is_err());
    assert!(decode_underworld_map_bytes(&vec![0; UNDER_DAT_LEN]).is_ok());
    assert!(
        WorldLiveChunkBuffer::from_underworld_bytes(&vec![0; UNDER_DAT_LEN - 256], 8, 8, |_| {
            LiveChunkSubstitutionPolicy::NONE
        })
        .is_err()
    );
}

#[test]
fn g3_under_dat_s12_materializer_places_slot_n_at_chunk_x_y() {
    // formats/under-dat.md §12 steps 2-4: slot -> (slot mod 16, slot / 16),
    // its 16 row-major rows copied into that region of the 256x256 grid.
    let bytes = g3_under_bytes_with_slot_pattern();
    let grid = decode_underworld_map_bytes(&bytes).unwrap();
    assert_eq!(grid.len(), WORLD_CELLS);
    for slot in [0usize, 1, 15, 16, 17, 130, 255] {
        let (cx, cy) = (slot % 16, slot / 16);
        for (lx, ly) in [(0, 0), (15, 0), (0, 15), (7, 9), (15, 15)] {
            let (x, y) = (cx * 16 + lx, cy * 16 + ly);
            assert_eq!(grid[world_cell_index(x, y)], slot as u8, "slot {slot} ({x},{y})");
        }
    }
    // §3: file_offset = chunk_slot * 256 + local_y * 16 + local_x, and the
    // materialized cell is the byte at that offset for every cell.
    let mut distinct = vec![0u8; UNDER_DAT_LEN];
    for (i, byte) in distinct.iter_mut().enumerate() {
        *byte = (i * 7 + i / 256) as u8;
    }
    let grid = decode_underworld_map_bytes(&distinct).unwrap();
    for y in 0..WORLD_SIDE {
        for x in 0..WORLD_SIDE {
            let offset = under_file_offset(x as u8, y as u8);
            assert_eq!(offset, under_dat_offset(x, y));
            assert_eq!(
                offset,
                ((y / 16) * 16 + x / 16) * 256 + (y % 16) * 16 + (x % 16)
            );
            assert_eq!(grid[world_cell_index(x, y)], distinct[offset]);
        }
    }
}

#[test]
fn g3_under_dat_s4_live_window_wraps_across_both_world_edges() {
    // formats/under-dat.md §4: "x = -1 samples column 255", "A viewport
    // centered near a world edge can draw from chunks on both sides".
    let mut bytes = vec![0u8; UNDER_DAT_LEN];
    bytes[under_file_offset(255, 255)] = 0x31;
    bytes[under_file_offset(0, 0)] = 0x32;
    bytes[under_file_offset(255, 0)] = 0x33;
    bytes[under_file_offset(0, 255)] = 0x34;
    let buffer = WorldLiveChunkBuffer::from_underworld_bytes(&bytes, 0, 0, |_| {
        LiveChunkSubstitutionPolicy::NONE
    })
    .unwrap();
    assert!(buffer.contains_world_tile(255, 255));
    assert!(buffer.contains_world_tile(0, 0));
    assert_eq!(buffer.tile_at(255, 255), 0x31);
    assert_eq!(buffer.tile_at(0, 0), 0x32);
    assert_eq!(buffer.tile_at(255, 0), 0x33);
    assert_eq!(buffer.tile_at(0, 255), 0x34);
}

#[test]
fn g3_under_dat_s10_live_chunk_substitutions_apply_to_underworld_and_never_write_back() {
    // formats/under-dat.md §10: the chunk loader applies its substitution
    // pass "to Underworld chunks on exactly the same terms as surface
    // chunks", each conditional, and "never written back".
    let mut bytes = vec![0u8; UNDER_DAT_LEN];
    bytes[under_file_offset(4, 4)] = 0x16;
    bytes[under_file_offset(5, 4)] = 0x17;
    bytes[under_file_offset(6, 4)] = 0x18;
    bytes[under_file_offset(7, 4)] = 0x19;
    let pristine = bytes.clone();

    let unsealed = WorldLiveChunkBuffer::from_underworld_bytes(&bytes, 8, 8, |_| {
        LiveChunkSubstitutionPolicy::NONE
    })
    .unwrap();
    assert_eq!(unsealed.tile_at(4, 4), 0x16);
    assert_eq!(unsealed.tile_at(7, 4), 0x19);

    let sealed = WorldLiveChunkBuffer::from_underworld_bytes(&bytes, 8, 8, |_| {
        LiveChunkSubstitutionPolicy {
            seal_entrances: true,
            ruin_shrine: true,
        }
    })
    .unwrap();
    for x in 4..=6 {
        assert_eq!(sealed.tile_at(x, 4), 0xDF);
    }
    assert_eq!(sealed.tile_at(7, 4), 0x1A);
    assert_eq!(bytes, pristine, "the file bytes are never rewritten");
}

// ---------------------------------------------------------------------------
// formats/font-pcs.md
// ---------------------------------------------------------------------------

#[test]
fn g3_font_pcs_s4_1_resident_advance_table_matches_the_published_values() {
    // formats/font-pcs.md §4.1, the full 0x20..0x7F table.
    let published: [(u8, u8); 96] = [
        (0x20, 0), (0x21, 2), (0x22, 6), (0x23, 7), (0x24, 7), (0x25, 7), (0x26, 7), (0x27, 3),
        (0x28, 4), (0x29, 4), (0x2A, 5), (0x2B, 6), (0x2C, 3), (0x2D, 3), (0x2E, 2), (0x2F, 7),
        (0x30, 6), (0x31, 5), (0x32, 6), (0x33, 6), (0x34, 6), (0x35, 6), (0x36, 6), (0x37, 6),
        (0x38, 6), (0x39, 6), (0x3A, 2), (0x3B, 3), (0x3C, 5), (0x3D, 5), (0x3E, 5), (0x3F, 6),
        (0x40, 7), (0x41, 7), (0x42, 6), (0x43, 6), (0x44, 6), (0x45, 6), (0x46, 6), (0x47, 6),
        (0x48, 6), (0x49, 4), (0x4A, 7), (0x4B, 7), (0x4C, 6), (0x4D, 7), (0x4E, 6), (0x4F, 6),
        (0x50, 7), (0x51, 6), (0x52, 7), (0x53, 6), (0x54, 6), (0x55, 6), (0x56, 6), (0x57, 7),
        (0x58, 6), (0x59, 6), (0x5A, 6), (0x5B, 3), (0x5C, 7), (0x5D, 3), (0x5E, 7), (0x5F, 8),
        (0x60, 3), (0x61, 5), (0x62, 5), (0x63, 4), (0x64, 5), (0x65, 5), (0x66, 5), (0x67, 5),
        (0x68, 5), (0x69, 2), (0x6A, 3), (0x6B, 5), (0x6C, 2), (0x6D, 7), (0x6E, 6), (0x6F, 5),
        (0x70, 5), (0x71, 5), (0x72, 4), (0x73, 4), (0x74, 4), (0x75, 5), (0x76, 5), (0x77, 7),
        (0x78, 5), (0x79, 5), (0x7A, 4), (0x7B, 0), (0x7C, 0), (0x7D, 0), (0x7E, 0), (0x7F, 0),
    ];
    assert_eq!(PROPORTIONAL_WIDTH_TABLE_LEN, 128);
    for (code, advance) in published {
        assert_eq!(
            PROPORTIONAL_WIDTH_TABLE.width_for_byte(code).unwrap(),
            usize::from(advance),
            "code {code:#04x}"
        );
    }
}

#[test]
fn g3_font_pcs_s4_2_glyph_advances_entry_plus_one_and_space_has_no_gap() {
    // formats/font-pcs.md §4.2: "Every drawn glyph advances the pen by its
    // table entry plus one pixel"; "A space advances by the descriptor's
    // space advance with no extra pixel"; §4.1 `{` is 15 and `_` is 0.
    assert_eq!(PCS_GLYPH_ADVANCE_GAP, 1);
    assert_eq!(PCS_SPACE_ADVANCE, 5);
    assert_eq!(PROPORTIONAL_BRACE_INDENT, 15);
    let boxed = ProportionalLayoutDescriptor::full_width(0, 0);
    let place = |text: &[u8]| {
        layout_proportional_paragraph_glyphs(
            &PROPORTIONAL_WIDTH_TABLE,
            &boxed,
            text,
            PROPORTIONAL_DRAW_CLIP_Y,
        )
        .unwrap()
    };
    // 'A' is 7 wide -> 'B' starts at 8.
    let ab = place(b"AB\0");
    assert_eq!((ab[0].x, ab[1].x), (0, 8));
    // A space adds exactly the descriptor's 5.
    let a_b = place(b"A B\0");
    assert_eq!(a_b[1].x, 8 + 5);
    // `_` measures zero; `{` measures 15 and draws nothing.
    let soft = place(b"A_B\0");
    assert_eq!(soft.len(), 2);
    assert_eq!(soft[1].x, 8);
    let brace = place(b"{A\0");
    assert_eq!((brace.len(), brace[0].x), (1, 15));
    // §8: a control byte other than NUL and newline is treated as a space.
    let tab = place(b"A\tB\0");
    assert_eq!(tab.len(), 2);
    assert_eq!(tab[1].x, 8 + 5);
}

#[test]
fn g3_font_pcs_s5_line_advance_is_nine_pixels() {
    // formats/font-pcs.md §5 item 5: "The line advance is a fixed 9 pixels".
    assert_eq!(PROPORTIONAL_LINE_STRIDE, 9);
    let boxed = ProportionalLayoutDescriptor::full_width(0, 30);
    let placed = layout_proportional_paragraph_glyphs(
        &PROPORTIONAL_WIDTH_TABLE,
        &boxed,
        b"A\nB\0",
        PROPORTIONAL_DRAW_CLIP_Y,
    )
    .unwrap();
    assert_eq!(placed[1].y - placed[0].y, 9);
}

#[test]
fn g3_font_pcs_s5_line_is_accepted_only_while_strictly_below_available_width() {
    // formats/font-pcs.md §5 item 2: "A line is accepted only while the
    // accumulated advance is strictly less than the available width, so the
    // right edge is exclusive." 'A' advances 8, so a width of 16 cannot take
    // a third glyph after two: after "A A" (8 + 5 + ... ) the walk breaks.
    let narrow = |right: u16| ProportionalLayoutDescriptor {
        right_a: right,
        right_b: right,
        ..ProportionalLayoutDescriptor::full_width(0, 0)
    };
    // "AA AA": "AA" measures 16. With 17 available, 16 < 17 so the space is
    // measured too, overflow follows at the next glyph and the break lands on
    // the space: line two starts with the second word.
    let placed = layout_proportional_paragraph_glyphs(
        &PROPORTIONAL_WIDTH_TABLE,
        &narrow(17),
        b"AA AA\0",
        PROPORTIONAL_DRAW_CLIP_Y,
    )
    .unwrap();
    assert_eq!(placed.iter().filter(|g| g.y == 0).count(), 2);
    assert_eq!(placed.iter().filter(|g| g.y == 9).count(), 2);
    // With exactly 16 available the second glyph's advance reaches the edge,
    // so the measurement stops before the space (16 is not < 16): there is
    // no break byte inside the measured run, and the token is split.
    let placed = layout_proportional_paragraph_glyphs(
        &PROPORTIONAL_WIDTH_TABLE,
        &narrow(16),
        b"AA AA\0",
        PROPORTIONAL_DRAW_CLIP_Y,
    )
    .unwrap();
    assert_eq!(placed.iter().filter(|g| g.y == 0).count(), 1);
}

#[test]
fn g3_font_pcs_s3_glyph_index_is_code_minus_0x20_with_no_glyph_outside() {
    // formats/font-pcs.md §3: "Glyph index n (0-based) corresponds to
    // character code 0x20 + n"; codes beyond the count have no glyph.
    let mut body = Vec::new();
    let count = 3u16;
    body.extend_from_slice(&count.to_le_bytes());
    for slot in 0..count {
        body.extend_from_slice(&(8 + slot * 12).to_le_bytes());
    }
    for width in [0u16, 3, 5] {
        body.extend_from_slice(&width.to_le_bytes());
        body.extend_from_slice(&(PCS_GLYPH_HEIGHT as u16).to_le_bytes());
        body.extend_from_slice(&[0u8; PCS_GLYPH_HEIGHT]);
    }
    let font = parse_proportional_font_body(&body, "fixture.pcs").unwrap();
    assert_eq!(font.first_code, 0x20);
    assert_eq!(font.glyph_for_code(0x20).unwrap().advance_width, 0);
    assert_eq!(font.glyph_for_code(0x21).unwrap().advance_width, 3);
    assert_eq!(font.glyph_for_code(0x22).unwrap().advance_width, 5);
    assert!(font.glyph_for_code(0x23).is_none());
    assert!(font.glyph_for_code(0x1F).is_none());
}

#[test]
fn g3_font_pcs_s7_declared_decoded_length_must_match() {
    // formats/font-pcs.md §7: "Treat the first four bytes as the LZW decoded
    // length ... and require the decoded byte count to match it."
    let mut body = Vec::new();
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&4u16.to_le_bytes());
    body.extend_from_slice(&3u16.to_le_bytes());
    body.extend_from_slice(&(PCS_GLYPH_HEIGHT as u16).to_le_bytes());
    body.extend_from_slice(&[0b1110_0000; PCS_GLYPH_HEIGHT]);
    let mut wrapped = lzw_envelope_with_literal_body(&body);
    assert!(parse_proportional_font_resource(&wrapped).is_ok());
    let declared = u32::from_le_bytes([wrapped[0], wrapped[1], wrapped[2], wrapped[3]]);
    wrapped[..4].copy_from_slice(&(declared + 1).to_le_bytes());
    assert!(parse_proportional_font_resource(&wrapped).is_err());
}

// ---------------------------------------------------------------------------
// systems/display-driver-mode.md
// ---------------------------------------------------------------------------

#[test]
fn g3_display_mode_s2_surface_is_320_by_200() {
    // systems/display-driver-mode.md §2: 320 pixels by 200 scanlines.
    assert_eq!(DISPLAY_SURFACE_WIDTH, 320);
    assert_eq!(DISPLAY_SURFACE_HEIGHT, 200);
    let surface = EgaDisplaySurface::new();
    assert_eq!(surface.front_pixels().len(), 320 * 200);
    assert_eq!(surface.read_pixel(319, 199), Some(0));
    assert_eq!(surface.read_pixel(320, 0), None);
    assert_eq!(surface.read_pixel(0, 200), None);
}

#[test]
fn g3_display_mode_s5_3_drawing_colour_register_draws_nothing_until_the_next_draw() {
    // systems/display-driver-mode.md §5.3: "Setting it does not itself
    // trigger a draw; the next draw call that needs a colour observes its
    // current value." §6: "The EGA entry stores the index unchanged."
    let mut surface = EgaDisplaySurface::new();
    for colour in 0..16u8 {
        surface.set_current_color(colour);
        assert_eq!(surface.current_color(), colour);
    }
    surface.set_current_color(9);
    assert!(surface.front_pixels().iter().all(|p| *p == 0));
    surface.plot_pixel(3, 4);
    assert_eq!(surface.read_pixel(3, 4), Some(9));
    surface.set_current_color(5);
    assert_eq!(surface.read_pixel(3, 4), Some(9), "setting does not redraw");
    surface.fill_rect_current_color(DisplayPixelRect {
        x0: 10,
        y0: 10,
        x1: 11,
        y1: 11,
    });
    assert_eq!(surface.read_pixel(10, 10), Some(5));
    assert_eq!(surface.read_pixel(11, 11), Some(5));
}

#[test]
fn g3_display_mode_s5_3_glyph_entry_ignores_the_drawing_colour_register() {
    // systems/display-driver-mode.md §5.3: "The text and glyph entries do
    // not consult this register; they take their foreground and background
    // colours directly through their register arguments."
    let font = parse_ch_font(&vec![0xF0; CH_FONT_LEN], "fixture.ch").unwrap();
    let mut surface = EgaDisplaySurface::new();
    surface.set_current_color(12);
    surface.draw_fixed_glyph_cell(&font, b'A', 0, 0, 2, 1).unwrap();
    assert_eq!(surface.read_pixel(0, 0), Some(2));
    assert_eq!(surface.read_pixel(7, 0), Some(1));
    assert!(
        surface.front_pixels()[..DISPLAY_SURFACE_WIDTH * 8]
            .iter()
            .all(|p| *p != 12)
    );
    assert_eq!(surface.current_color(), 12);
}

#[test]
fn g3_display_mode_s7_cga_selects_palette_one_black_cyan_magenta_white() {
    // systems/display-driver-mode.md §7: "palette one - black, cyan,
    // magenta, white - with the background and border black at low
    // intensity".
    assert_eq!(CGA_PALETTE_RGB[0], [0x00, 0x00, 0x00]);
    assert_eq!(CGA_PALETTE_RGB[1], [0x55, 0xff, 0xff]);
    assert_eq!(CGA_PALETTE_RGB[2], [0xff, 0x55, 0xff]);
    assert_eq!(CGA_PALETTE_RGB[3], [0xff, 0xff, 0xff]);
}

#[test]
fn g3_display_mode_s6_low_colour_ui_set_uses_only_two_bit_values() {
    // systems/display-driver-mode.md §6: "The CGA and Hercules drawing-colour
    // entries reduce the requested index to two bits ... which is why the
    // low-colour user-interface colour set ... uses only values 0..3."
    assert!(UI_COLOUR_TABLE_LOW.iter().all(|value| *value <= 3));
    for slot in 0..UI_COLOUR_TABLE_SLOTS {
        assert!(ui_colour_slot(slot, false) <= 3);
    }
}

#[test]
fn g3_display_mode_s2_release_returns_draws_to_the_visible_page() {
    // systems/display-driver-mode.md §2/§3: the visible page is page zero and
    // subsequent draws target it once the back buffer is released.
    let mut surface = EgaDisplaySurface::new();
    surface.init_back_buffer();
    surface.set_render_target(DisplayRenderTarget::Back);
    surface.release_back_buffer();
    assert_eq!(surface.render_target(), DisplayRenderTarget::Front);
    surface.set_current_color(4);
    surface.fill_rect_current_color(DisplayPixelRect {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    });
    assert_eq!(surface.read_pixel(0, 0), Some(4));
}

// ---------------------------------------------------------------------------
// systems/traps.md
// ---------------------------------------------------------------------------

fn g3_trap_member(slot: usize, status: u8, hp: u16) -> PartyMember {
    PartyMember {
        slot: slot as u8,
        class_byte: b'F',
        status,
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 7,
        hp,
        max_hp: 40,
        level: 2,
    }
}

#[test]
fn g3_traps_s3_acid_has_no_status_gate_and_kills_at_zero() {
    // systems/traps.md §3 effect 0: "applies no status gate ... an Ashes
    // member can be damaged here, and on reaching zero hit points the damage
    // helper marks the character Dead, zeroes hit points, and clears the
    // active-member hint when it named that slot."
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![g3_trap_member(0, b'A', 35), g3_trap_member(1, b'S', 1)];
    let _ = state.apply_acid_trap_effect(0);
    assert!(state.party[0].hp < 35, "an Ashes member takes acid damage");

    state.active_player = Some(1);
    let _ = state.apply_acid_trap_effect(1);
    assert_eq!(state.party[1].hp, 0);
    assert_eq!(state.party[1].status, b'D');
    assert_eq!(state.active_player, None);
}

#[test]
fn g3_traps_s3_poison_helper_two_gates_and_no_other_field() {
    // systems/traps.md §3: gate 1 is the unsigned party-count check, gate 2 is
    // a single Dead comparison; Ashes, Sleeping, Charmed and Poisoned are all
    // overwritten with Poisoned and "no hit points, no maximum hit points, no
    // magic points" change.
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![
        g3_trap_member(0, b'A', 0),
        g3_trap_member(1, b'S', 12),
        g3_trap_member(2, b'C', 12),
        g3_trap_member(3, b'P', 12),
        g3_trap_member(4, b'D', 0),
    ];
    let before = state.party.clone();
    for slot in 0..4 {
        assert!(state.apply_trap_poison_status_to_slot(slot), "slot {slot}");
        assert_eq!(state.party[slot].status, b'P');
        assert_eq!(state.party[slot].hp, before[slot].hp);
        assert_eq!(state.party[slot].max_hp, before[slot].max_hp);
        assert_eq!(state.party[slot].mana, before[slot].mana);
    }
    assert!(!state.apply_trap_poison_status_to_slot(4));
    assert_eq!(state.party[4].status, b'D');
    // Out of range: silently ignored, nothing written.
    let snapshot = state.party.clone();
    assert!(!state.apply_trap_poison_status_to_slot(5));
    assert!(!state.apply_trap_poison_status_to_slot(usize::MAX));
    assert_eq!(state.party, snapshot);
}

#[test]
fn g3_traps_s3_gas_converts_an_ashes_member_to_poisoned() {
    // systems/traps.md §3 effect 3: "Ashes members are overwritten with
    // Poisoned along with the rest."
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![g3_trap_member(0, b'A', 0), g3_trap_member(1, b'D', 0)];
    let _ = state.apply_gas_trap_effect();
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[1].status, b'D');
}

#[test]
fn g3_traps_s4_forced_mix_without_a_target_still_applies_party_wide_gas() {
    // systems/traps.md §4 safety rule: with no valid single target the
    // resolver "may still play the rolled message and apply a party-wide Bomb
    // or Gas result, but must make Acid or single-target Poison a no-op".
    let template = {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![g3_trap_member(0, b'D', 0), g3_trap_member(1, b'S', 20)];
        state
    };
    assert_eq!(template.mixer_trap_target_slot(), None);
    let mut seen_gas = false;
    let mut seen_poison = false;
    for seed in 0..=u16::MAX {
        let mut state = template.clone();
        state.prng_state = seed;
        let mut probe = seed;
        let family = shared_trap_effect_family_from_index(
            u5_prng_range_u16(&mut probe, 0, 7) as u8,
            false,
        );
        match family {
            TrapEffect::Gas if !seen_gas => {
                let _ = state.wrong_mix_trap_message();
                assert_eq!(state.party[1].status, b'P', "gas still poisons the sleeper");
                seen_gas = true;
            }
            TrapEffect::Poison if !seen_poison => {
                let _ = state.wrong_mix_trap_message();
                assert_eq!(state.party, template.party, "single-target poison is a no-op");
                seen_poison = true;
            }
            _ => {}
        }
        if seen_gas && seen_poison {
            break;
        }
    }
    assert!(seen_gas && seen_poison);
}

#[test]
fn g3_traps_s2_resolver_plays_the_sting_before_printing_the_word() {
    // systems/traps.md §2 step 1: "Play the short trap audio sting", then
    // step 3 prints the selected trap message.
    let mut state = test_state(open_grid(), 1, 1);
    let serial = state.sound_effect_serial;
    let _ = state.apply_shared_trap_effect_to_slot(0);
    assert_eq!(state.sound_effects_after(serial), vec![SoundEffect::TrapRumble]);
    state.flush_message_slot();
    let words = ["ACID!", "POISON!", "BOMB!", "GAS!"];
    assert!(
        state
            .message_transcript
            .iter()
            .any(|entry| words.contains(&entry.text.trim_end())),
        "{:?}",
        state.message_transcript
    );
}

#[test]
fn g3_traps_s4_dungeon_chest_trap_test_is_the_low_three_bits() {
    // systems/traps.md §4: "Underground the test is the cell's low *three*
    // bits, not its low bit alone, so a chest cell whose low nibble is 4 is
    // trapped"; the cell is rewritten to the opened form whether or not the
    // trap fired.
    let run = |tile: u8| {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = tile;
        let mut state = dungeon_state(grid, 0, 1, 1);
        let serial = state.sound_effect_serial;
        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);
        let sprang = state
            .sound_effects_after(serial)
            .contains(&SoundEffect::TrapRumble);
        (sprang, state.grid[dungeon_cell_index(0, 1, 1)])
    };
    assert_eq!(DUNGEON_CHEST_TRAP_SUBTYPE_MASK, 0x07);
    let (sprang, cell) = run(0x44);
    assert!(sprang, "low nibble 4 is trapped");
    assert_eq!(cell, dungeon_open_chest_rewrite(0x44));
    let (sprang, cell) = run(0x48);
    assert!(!sprang, "bit 3 alone is not the trap sub-type");
    assert_eq!(cell, dungeon_open_chest_rewrite(0x48));
}

#[test]
fn g3_traps_s4_surface_combat_cleanup_stamps_a_corpse_and_clears_the_hint() {
    // systems/traps.md §4: if the chosen member is then Dead the surface site
    // sets a marker bit on its combatant record (not removing it), writes
    // decimal thirty - a corpse-class object - into both leading bytes of the
    // linked world-object entry, and clears the active hint naming it.
    assert_eq!(COMBAT_PARTY_CORPSE_TILE, 30);
    let mut state = combat_player_command_state(8, 5);
    let actors_before = state.combat_actors.len();
    state.party[0].hp = 0;
    state.party[0].status = b'D';
    state.active_player = Some(0);
    state.apply_surface_container_trap_combat_cleanup(0);
    assert_eq!(state.combat_actors.len(), actors_before);
    assert!(state.combat_actors[0].is_marked_dead());
    let linked = usize::from(state.combat_actors[0].active_object_slot);
    assert_eq!(state.active_objects[linked].type_byte, 30);
    assert_eq!(state.active_objects[linked].tile, 30);
    assert_eq!(state.active_player, None);

    // A living member triggers no cleanup step at all.
    let mut alive = combat_player_command_state(8, 5);
    alive.active_player = Some(0);
    let linked = usize::from(alive.combat_actors[0].active_object_slot);
    let object_before = alive.active_objects[linked].clone();
    alive.apply_surface_container_trap_combat_cleanup(0);
    assert!(!alive.combat_actors[0].is_marked_dead());
    assert_eq!(alive.active_objects[linked], object_before);
    assert_eq!(alive.active_player, Some(0));
}

// ---------------------------------------------------------------------------
// systems/audio.md
// ---------------------------------------------------------------------------

fn g3_rumble_shape(program: &crate::audio::SpeakerProgram) -> (usize, u32, u32) {
    let freqs = program.frequencies();
    (
        freqs.len(),
        freqs.iter().copied().min().unwrap_or(0),
        freqs.iter().copied().max().unwrap_or(0),
    )
}

#[test]
fn g3_audio_s5_2_surface_falls_descent_realises_1005_hz_not_800() {
    // systems/audio.md §5.2: "300 updates: 2500, 2495, ... 1005 Hz, against a
    // nominal target of 800 Hz."
    let freqs = crate::audio::surface_falls_descent().frequencies();
    assert_eq!(freqs.len(), 300);
    assert_eq!(freqs[0], 2500);
    assert_eq!(freqs[1], 2495);
    assert_eq!(*freqs.last().unwrap(), 1005);
    assert!(crate::audio::surface_falls_descent().ends_with_stop());
}

#[test]
fn g3_audio_s5_3_rumble_rows_have_the_published_counts_and_ranges() {
    // systems/audio.md §5.3 recipe table: iterations = ceil(target / step)
    // over the inclusive range 100..maximum.
    use crate::audio::{RumbleJitter, SoundEffect as Fx};
    let mut j = RumbleJitter::new();
    let rows: [(Fx, usize, u32); 9] = [
        (Fx::TrapRumble, 75, 500),
        (Fx::DamageRumble, 160, 2000),
        (Fx::RoughSeasImpactRumble, 300, 2000),
        (Fx::CombatMonsterHitBurst, 300, 2000),
        (Fx::CombatPartyHitBurst, 75, 500),
        (Fx::CombatDraggedUnderRumble, 75, 500),
        (Fx::CombatDraggedUnderReleaseRumble, 7000, 600),
        (Fx::DungeonElectricPushBack, 500, 20000),
        (Fx::StatusFieldRumble, 50, 3500),
    ];
    for (effect, updates, max_hz) in rows {
        let program = effect.program(&mut j);
        let (count, low, high) = g3_rumble_shape(&program);
        assert_eq!(count, updates, "{effect:?}");
        assert!(low >= 100 && high <= max_hz, "{effect:?} {low}..{high}");
        assert!(program.ends_with_stop());
    }
    // Footstep: update count equals the intensity.
    for intensity in [15u8, 11, 7, 3] {
        let program = Fx::DungeonFootstep { intensity }.program(&mut j);
        assert_eq!(program.tone_count(), usize::from(intensity));
        assert!(g3_rumble_shape(&program).2 <= 20000);
    }
    // Ship collision (§7.4): the twenty-update `(100, 2000, 300)` rumble.
    let ship = Fx::ShipCollisionRumble.program(&mut j);
    assert_eq!(ship.tone_count(), 20);
    assert!(g3_rumble_shape(&ship).2 <= 300);
}

#[test]
fn g3_audio_s5_3_two_part_sting_parts_draw_from_their_own_bands() {
    // systems/audio.md §5.3: first part 25 updates in 100..1000 Hz, second 25
    // in 100..1500 Hz, a 20-unit silent hold between.
    let mut j = crate::audio::RumbleJitter::new();
    let program = crate::audio::two_part_sting(&mut j);
    let freqs = program.frequencies();
    assert_eq!(freqs.len(), 50);
    assert!(freqs[..25].iter().all(|f| (100..=1000).contains(f)));
    assert!(freqs[25..].iter().all(|f| (100..=1500).contains(f)));
    let silence = program
        .ops
        .iter()
        .find_map(|op| match op {
            crate::audio::SpeakerOp::Silence { nanos } => Some(*nanos),
            _ => None,
        })
        .unwrap();
    assert_eq!(silence, crate::audio::calibrated_nanos(20));
}

#[test]
fn g3_audio_s5_4_7_ordination_chime_rows_match_the_published_table() {
    // systems/audio.md §5.4.7 ordination chime: (increment, iterations,
    // initial, delta), idle 1, seven rows, 33,000 iterations, largest
    // comparison 64,492 and no wrap.
    use crate::audio::EnvelopeSegment as E;
    let published = [
        E::new(9, 1000, 7_000, 1, 3300),
        E::new(10, 1000, 6_000, 1, 3925),
        E::new(21, 1000, 3_000, 1, 3925),
        E::new(21, 1000, 3_000, 1, 3925),
        E::new(21, 1000, 3_000, 1, 3925),
        E::new(21, 1000, 3_000, 1, 3700),
        E::new(8, 500, 8_000, 1, 3925),
    ];
    assert_eq!(crate::audio::SHRINE_ORDINATION_CHIME, published);
    let total: u32 = published.iter().map(|s| s.iterations).sum();
    assert_eq!(total, 33_000);
    let max_comparison = published
        .iter()
        .map(|s| i64::from(s.initial_comparison) + i64::from(s.delta) * (i64::from(s.iterations) - 1))
        .max()
        .unwrap();
    assert_eq!(max_comparison, 64_492);
    let program = crate::audio::SoundEffect::ShrineOrdinationChime
        .program(&mut crate::audio::RumbleJitter::new());
    let envelopes: Vec<_> = program
        .ops
        .iter()
        .filter_map(|op| match op {
            crate::audio::SpeakerOp::Envelope(s) => Some(*s),
            _ => None,
        })
        .collect();
    assert_eq!(envelopes, published.to_vec());
}

#[test]
fn g3_audio_s5_4_7_920_step_swell_geometry_and_callers() {
    // systems/audio.md §5.4.7: 920 runs, comparison 2000..24950 rising then
    // 25000..2050 falling in steps of 50, delta 0, idle 1; per-caller
    // increment and iterations per run.
    use crate::audio::{SoundEffect as Fx, SpeakerOp};
    let cases = [
        (Fx::ShrineOfferingSwell, 2700u16, 200u32, 184_000u32),
        (Fx::ShrineTurnInSwell, 3100, 150, 138_000),
        (Fx::ShardSwell, 2640, 200, 184_000),
        (Fx::BlackthornExecutionSwell, 2640, 200, 184_000),
    ];
    for (effect, period, per_run, total) in cases {
        let program = effect.program(&mut crate::audio::RumbleJitter::new());
        let runs: Vec<_> = program
            .ops
            .iter()
            .filter_map(|op| match op {
                SpeakerOp::Envelope(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 920, "{effect:?}");
        assert!(runs.iter().all(|s| s.period == period && s.iterations == per_run));
        assert!(runs.iter().all(|s| s.delta == 0 && s.idle == 1));
        assert_eq!(runs.iter().map(|s| s.iterations).sum::<u32>(), total);
        assert_eq!(runs[0].initial_comparison, 2000);
        assert_eq!(runs[459].initial_comparison, 24_950);
        assert_eq!(runs[460].initial_comparison, 25_000);
        assert_eq!(runs[919].initial_comparison, 2050);
        assert!(program.ends_with_stop());
    }
}

#[test]
fn g3_audio_s6_shared_variant_table_matches_the_published_rows() {
    // systems/audio.md §6: (period, first, second, delta, iterations).
    let published: [(u16, u16, u16, i32, u32); 9] = [
        (8810, 2700, 32700, 3, 10000),
        (7830, 3000, 31000, 2, 14000),
        (7060, 1000, 37000, 2, 18000),
        (6550, 100, 45000, 2, 22000),
        (5950, 5000, 31000, 1, 26000),
        (5570, 4000, 34000, 1, 30000),
        (5180, 2500, 36500, 1, 34000),
        (4820, 1000, 39000, 1, 38000),
        (4480, 1, 42000, 1, 42000),
    ];
    assert_eq!(crate::audio::SHARED_VARIANT_IDLE, 1);
    for (index, row) in crate::audio::SHARED_VARIANTS.iter().enumerate() {
        assert_eq!(
            (
                row.phase_period,
                row.first_initial_comparison,
                row.second_initial_comparison,
                row.delta_magnitude,
                row.iterations_per_envelope
            ),
            published[index],
            "variant {index}"
        );
    }
    // Step 1: rumble (800, 8000 + 1600v, 700).
    let mut j = crate::audio::RumbleJitter::new();
    for v in 0..9u8 {
        let lead = crate::audio::shared_variant_lead(v, &mut j);
        let (count, low, high) = g3_rumble_shape(&lead);
        assert_eq!(count, (8000 + 1600 * usize::from(v)).div_ceil(800));
        assert!(low >= 100 && high <= 700);
    }
}

#[test]
fn g3_audio_s6_1_mass_target_leads_use_their_circle_targets() {
    // systems/audio.md §6.1 mass-target item 1: T = 16000 (Sleep, circle 5),
    // 19200 (Poison Wind, circle 7), 20800 (circles 8) with step 800.
    let mut j = crate::audio::RumbleJitter::new();
    for (circle, target) in [(5u8, 16_000u32), (7, 19_200), (8, 20_800)] {
        assert_eq!(8000 + 1600 * u32::from(circle), target);
        let lead = crate::audio::circle_rumble_lead(circle, &mut j);
        assert_eq!(lead.tone_count() as u32, target.div_ceil(800));
        assert!(g3_rumble_shape(&lead).2 <= 700);
    }
}

#[test]
fn g3_audio_s8_3_named_envelopes_have_the_published_tuples() {
    // systems/audio.md §8.3, §6.1, §8.6, §8.7: (delta, initial, iterations,
    // idle, period) tuples are exact program constants.
    use crate::audio::*;
    assert_eq!(POSSESSION_ENVELOPE, EnvelopeSegment::new(2, 1000, 30_000, 1, 3100));
    assert_eq!(MONSTER_SUMMON_ENVELOPE, EnvelopeSegment::new(15, 1000, 5_000, 1, 2760));
    assert_eq!(PLAYER_SUMMON_ENVELOPE, EnvelopeSegment::new(5, 500, 12_000, 1, 2760));
    assert_eq!(MOONGATE_TRANSIT_ENVELOPE, EnvelopeSegment::new(2, 2000, 30_000, 1, 5900));
    assert_eq!(
        SHADOWLORD_YELL_SUMMON_ENVELOPE,
        EnvelopeSegment::new(2, 2000, 30_000, 1, 10_400)
    );
    assert_eq!(MAGIC_ABSORBED_ENVELOPE, EnvelopeSegment::new(2, 1000, 28_000, 1, 9800));
    assert_eq!(
        SHADOWLORD_PRESENCE_NOTICE_ENVELOPE,
        EnvelopeSegment::new(1, 2000, 60_000, 1, 6600)
    );
    assert_eq!(
        BLACKTHORN_THRONE_APPROACH_ENVELOPE,
        EnvelopeSegment::new(5, 100, 13_000, 1, 2800)
    );
    assert_eq!(ENDGAME_RESTORATION_ENVELOPE, EnvelopeSegment::new(1, 5000, 40_000, 1, 8800));
    assert_eq!(ENDGAME_TABLEAU_ENVELOPE, EnvelopeSegment::new(1, 10_000, 50_000, 1, 5200));
    // The faint path plays the identical possession envelope.
    let mut j = RumbleJitter::new();
    assert_eq!(
        SoundEffect::ControlledPartyFaint.program(&mut j),
        SoundEffect::Possession.program(&mut j)
    );
    // §8.6: the presence notice's comparison climbs 2000..61,999, no wrap.
    let last = 2000u32 + 60_000 - 1;
    assert_eq!(last, 61_999);
    assert!(last < 65_536);
}

#[test]
fn g3_audio_s8_6_camp_apparition_envelopes_match_the_published_rows() {
    // systems/audio.md §8.6 camp apparition rows.
    use crate::audio::EnvelopeSegment as E;
    use crate::lord_british_camp::*;
    assert_eq!(LORD_BRITISH_CAMP_APPARITION_ENVELOPE, E::new(6, 2500, 10_000, 1, 2620));
    assert_eq!(LORD_BRITISH_CAMP_FIGURE_PERIODS, [2620, 2620, 2620, 3700, 3900, 4160]);
    for period in LORD_BRITISH_CAMP_FIGURE_PERIODS {
        assert_eq!(lord_british_camp_figure_note(period), E::new(13, 200, 5_000, 1, period));
    }
    assert_eq!(LORD_BRITISH_CAMP_STING_SHORT, E::new(13, 200, 5_000, 1, 5500));
    assert_eq!(LORD_BRITISH_CAMP_STING_LONG, E::new(1, 2500, 60_000, 1, 5500));
}

#[test]
fn g3_audio_s8_6_2_rescue_restoration_increment_is_36400_over_slot_plus_7() {
    // systems/audio.md §8.6.2: per-slot phase increment 36400 / (slot + 7):
    // 5200, 4550, 4044, 3640, 3309, 3033; idle 1, 30,000 iterations,
    // initial 2000, delta +2; the formula continues past six slots.
    let expected = [5200u16, 4550, 4044, 3640, 3309, 3033];
    for (slot, period) in expected.iter().enumerate() {
        assert_eq!(
            crate::audio::blackthorn_rescue_restoration_envelope(slot),
            crate::audio::EnvelopeSegment::new(2, 2000, 30_000, 1, *period),
            "slot {slot}"
        );
    }
    assert_eq!(
        crate::audio::blackthorn_rescue_restoration_envelope(6).period,
        (36_400 / 13) as u16
    );
}

#[test]
fn g3_audio_s7_4_dungeon_footstep_tail_is_7_then_3_on_alternate_ticks() {
    // systems/audio.md §7.4 "The tail": after the step's 15/11 pair, "the
    // second later tick plays the 7 and the fourth plays the 3, and later
    // ticks are silent until the next committed step re-arms 15".
    let mut state = dungeon_state(open_dungeon_record(), 0, 4, 4);
    state.commit_dungeon_step_position(5, 4);
    let mut per_tick = Vec::new();
    for _ in 0..8 {
        let serial = state.sound_effect_serial;
        state.tick_dungeon_footstep();
        per_tick.push(state.sound_effects_after(serial));
    }
    use crate::audio::SoundEffect as Fx;
    assert!(per_tick[0].is_empty());
    assert_eq!(per_tick[1], vec![Fx::DungeonFootstep { intensity: 7 }]);
    assert!(per_tick[2].is_empty());
    assert_eq!(per_tick[3], vec![Fx::DungeonFootstep { intensity: 3 }]);
    assert!(per_tick[4..].iter().all(|t| t.is_empty()), "{per_tick:?}");
}

#[test]
fn g3_audio_s8_10_window_admits_corners_and_ties_go_north_within_a_column() {
    // systems/audio.md §8.10: "The starting bound admits every cell of the
    // window, corners included"; ties go west first, and "within a column,
    // north before south"; nothing outside the window counts.
    use crate::audio::{AmbientScenerySource as S, select_ambient_scenery_source};
    assert_eq!(crate::audio::AMBIENT_SCENERY_WINDOW_RADIUS, 5);
    let corner = select_ambient_scenery_source(|dx, dy| (dx == 5 && dy == 5).then_some(S::Fountain));
    assert_eq!(corner, Some(S::Fountain));
    // Same column, equal distance: (0,-2) north vs (0,2) south.
    let tie = select_ambient_scenery_source(|dx, dy| match (dx, dy) {
        (0, -2) => Some(S::Waterfall),
        (0, 2) => Some(S::Fountain),
        _ => None,
    });
    assert_eq!(tie, Some(S::Waterfall));
    // Strictly nearer wins regardless of scan order.
    let nearer = select_ambient_scenery_source(|dx, dy| match (dx, dy) {
        (-4, 0) => Some(S::Waterfall),
        (3, 1) => Some(S::GrandfatherClock),
        _ => None,
    });
    assert_eq!(nearer, Some(S::GrandfatherClock));
}

#[test]
fn g3_audio_s8_10_terrain_classifiers_match_the_published_tiles() {
    // systems/audio.md §8.10 source classes and minstrel band 0x5C..0x5F.
    use crate::audio::{AmbientScenerySource as S, ambient_scenery_terrain_source};
    for tile in 0u8..=255 {
        let expected = match tile {
            0xFA | 0xFB => Some(S::GrandfatherClock),
            0xD4..=0xD7 => Some(S::Waterfall),
            0xD8..=0xDB => Some(S::Fountain),
            _ => None,
        };
        assert_eq!(ambient_scenery_terrain_source(tile), expected, "tile {tile:#04x}");
    }
    assert_eq!(crate::audio::AMBIENT_MINSTREL_ACTOR_TILES, 0x5c..=0x5f);
    // Waterfall (20, 60, 10000) -> 3 updates; fountain (10, 30, 25000) -> 3.
    let mut j = crate::audio::RumbleJitter::new();
    let waterfall = crate::audio::SoundEffect::AmbientWaterfallRumble.program(&mut j);
    assert_eq!(waterfall.tone_count(), 3);
    assert!(g3_rumble_shape(&waterfall).2 <= 10_000);
    let fountain = crate::audio::SoundEffect::AmbientFountainRumble.program(&mut j);
    assert_eq!(fountain.tone_count(), 3);
    assert!(g3_rumble_shape(&fountain).2 <= 25_000);
    // Clock tick-tock: 3 units at 3000 / 2000 Hz.
    let tick = crate::audio::SoundEffect::AmbientClockTick { residue: 0 }.program(&mut j);
    assert_eq!(tick.frequencies(), vec![3000]);
    assert_eq!(tick.ops[0].nanos(true) >= crate::audio::calibrated_nanos(3), true);
    let silent = crate::audio::SoundEffect::AmbientClockTick { residue: 2 }.program(&mut j);
    assert_eq!(silent.tone_count(), 0);
}

#[test]
fn g3_audio_s8_10_countdown_drops_at_residues_0_and_4_without_a_clock() {
    // systems/audio.md §8.10: the hour-chime countdown "drops by one on every
    // call at residue 0 or 4, whether or not a clock is in range"; the
    // sub-counter is free-running over 0..7.
    let mut state = test_state(open_grid(), 10, 10);
    state.twelve_hour_audio_repeats = 3;
    state.ambient_audio_sub_tick = 0;
    let serial = state.sound_effect_serial;
    for _ in 0..8 {
        state.tick_ambient_audio_repeats();
    }
    assert_eq!(state.twelve_hour_audio_repeats, 1);
    assert_eq!(state.ambient_audio_sub_tick, 0);
    assert!(state.sound_effects_after(serial).is_empty());
}

// ---------------------------------------------------------------------------
// formats/location-dat.md
// ---------------------------------------------------------------------------

fn g3_game_dir_without_floor_table() -> std::path::PathBuf {
    let dir = debug_game_dir();
    let _ = std::fs::remove_file(dir.join(LOCATION_FLOOR_TABLE_FILE));
    dir
}

#[test]
fn g3_location_dat_s4_1_every_scene_matches_the_published_base_page_and_run() {
    // formats/location-dat.md §4.1: the complete per-scene table
    // (scene, base page, first page of the run, last page of the run).
    let published: [(u8, usize, usize, usize); 32] = [
        (1, 0, 0, 1),
        (2, 2, 2, 3),
        (3, 4, 4, 5),
        (4, 7, 6, 7),
        (5, 8, 8, 9),
        (6, 10, 10, 11),
        (7, 12, 12, 13),
        (8, 14, 14, 15),
        (9, 0, 0, 2),
        (10, 3, 3, 5),
        (11, 6, 6, 8),
        (12, 9, 9, 11),
        (13, 12, 12, 12),
        (14, 13, 13, 13),
        (15, 14, 14, 14),
        (16, 15, 15, 15),
        (17, 1, 0, 4),
        (18, 6, 5, 9),
        (19, 10, 10, 10),
        (20, 11, 11, 11),
        (21, 12, 12, 12),
        (22, 13, 13, 13),
        (23, 14, 14, 14),
        (24, 15, 15, 15),
        (25, 0, 0, 1),
        (26, 2, 2, 3),
        (27, 4, 4, 4),
        (28, 5, 5, 5),
        (29, 6, 6, 6),
        (30, 7, 7, 9),
        (31, 10, 10, 12),
        (32, 14, 13, 15),
    ];
    let dir = g3_game_dir_without_floor_table();
    for (scene_byte, base, first, last) in published {
        let scene = Scene::new(scene_byte).unwrap();
        let run = location_page_run(scene);
        assert_eq!(
            (run.base_page, run.first_page, run.last_page),
            (base, first, last),
            "scene {scene_byte}"
        );
        assert_eq!(resolve_location_base_page(&dir, scene).unwrap(), base);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn g3_location_dat_s4_signed_floor_byte_adds_to_the_base_page() {
    // formats/location-dat.md §4 steps 3-4: the floor byte is signed eight
    // bit and is added to the base page; exactly 1,024 bytes are read at
    // `page * 1024`. A higher page is a higher floor.
    let dir = g3_game_dir_without_floor_table();
    let mut castle = vec![0u8; 16 * 1024];
    for page in 0..16 {
        castle[page * 1024..(page + 1) * 1024].fill(page as u8 + 0x40);
    }
    std::fs::write(dir.join("CASTLE.DAT"), &castle).unwrap();
    let lord_british = Scene::new(17).unwrap();
    let blackthorn = Scene::new(18).unwrap();
    for (scene, floor, page) in [
        (lord_british, -1i8, 0usize),
        (lord_british, 0, 1),
        (lord_british, 3, 4),
        (blackthorn, -1, 5),
        (blackthorn, 0, 6),
        (blackthorn, 3, 9),
    ] {
        assert_eq!(resolve_location_floor_page(&dir, scene, floor).unwrap(), page);
        let grid = load_floor(&dir, scene, floor).unwrap();
        assert_eq!(grid.len(), 1024);
        assert!(grid.iter().all(|b| *b == page as u8 + 0x40), "{} floor {floor}", scene.byte);
    }
    // 0xFF read as a byte is floor -1, the page before the base page.
    assert_eq!(
        resolve_location_floor_page(&dir, lord_british, 0xFFu8 as i8).unwrap(),
        0
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn g3_location_dat_s2_roster_index_is_scene_minus_one_and_seven() {
    // formats/location-dat.md §2: "the location's roster index is
    // `(scene - 1) & 7`", the file family `(scene - 1) >> 3`.
    for scene_byte in 1..=32u8 {
        let scene = Scene::new(scene_byte).unwrap();
        assert_eq!(scene.block, usize::from((scene_byte - 1) & 7));
        let expected_file = ["TOWNE.DAT", "DWELLING.DAT", "CASTLE.DAT", "KEEP.DAT"]
            [usize::from((scene_byte - 1) >> 3)];
        assert_eq!(location_dat_filename(scene_byte), Some(expected_file));
    }
    assert!(Scene::new(0).is_err());
    assert!(Scene::new(33).is_err());
}

#[test]
fn g3_location_dat_s6_npc_start_marker_class_ignores_only_the_low_bit() {
    // formats/location-dat.md §6: "The adjacent tile values `0x48` and
    // `0x49` ... one paired marker class by ignoring the low bit".
    for tile in 0u8..=255 {
        assert_eq!(is_npc_start_marker(tile), tile == 0x48 || tile == 0x49, "{tile:#04x}");
    }
}

#[test]
fn g3_location_dat_s10_night_band_is_hours_0_to_4_and_20_to_23() {
    // formats/location-dat.md §10: "the pass runs if and only if the
    // in-game hour is in the night band: hours `0..4` or `20..23`".
    for hour in 0u8..24 {
        assert_eq!(
            is_town_night_hour(hour),
            hour <= 4 || hour >= 20,
            "hour {hour}"
        );
    }
}

#[test]
fn g3_location_dat_s10_xor_is_generic_and_targets_only_the_cell_south() {
    // formats/location-dat.md §10: "applies an XOR of `0xDD` to the cell
    // immediately south"; "the byte-level rule is generic (`paired = paired
    // ^ 0xDD`)"; the marker itself is not rewritten.
    let mut grid = vec![0x05u8; 1024];
    grid[4 * 32 + 9] = 0x87;
    grid[5 * 32 + 9] = 0x12;
    let before = grid.clone();
    apply_dawn_dusk_substitution(&mut grid);
    assert_eq!(grid[4 * 32 + 9], 0x87);
    assert_eq!(grid[5 * 32 + 9], 0x12 ^ 0xDD);
    for (i, (a, b)) in before.iter().zip(grid.iter()).enumerate() {
        if i != 5 * 32 + 9 {
            assert_eq!(a, b, "cell {i} untouched");
        }
    }
}

#[test]
fn g3_location_dat_s10_ordinary_hour_change_does_not_toggle_the_gate() {
    // formats/location-dat.md §10: "callers should run it exactly when
    // crossing those two boundaries, not on every daylight recompute".
    let mut grid = open_grid();
    grid[2 * 32 + 4] = 0x87;
    grid[3 * 32 + 4] = 0x44;
    let mut state = test_state(grid, 1, 1);
    for hour in [12u8, 3, 18, 21, 23] {
        state.clock = GameClock::new(hour, 59).unwrap();
        assert_eq!(state.pass_turn(), MoveOutcome::Passed);
        assert_eq!(state.grid[3 * 32 + 4], 0x44, "crossing into hour {}", hour + 1);
    }
}

#[test]
fn g3_location_dat_s6_blight_returns_immediately_with_no_resident_shadowlord() {
    // formats/location-dat.md §6 "Farmland and orchard blight": the pass
    // "returns immediately when that record says 'none', which is the
    // ordinary case for every location in the game".
    let mut state = test_state(
        vec![SHADOWLORD_BLIGHT_STANDING_CROP_TILE; TOWN_GRID_BYTES],
        5,
        5,
    );
    state.area = Area::Town {
        scene: Scene::new(SCENE_TRINSIC).unwrap(),
        floor: 0,
    };
    state.clock.day = 11;
    state.shadowlord_hideouts = [SHADOWLORD_VANQUISHED; 3];
    state.prng_state = 0x1234;
    assert_eq!(state.install_shadowlord_entry_encounter(), None);
    assert!(state.grid.iter().all(|t| *t == SHADOWLORD_BLIGHT_STANDING_CROP_TILE));
    assert_eq!(state.prng_state, 0x1234, "no reseed when the pass does not run");
}

#[test]
fn g3_location_dat_s11_rtv_captions_follow_the_strip_argument() {
    // formats/location-dat.md §11: the `0x06` strip argument selects the
    // fixed chapter caption.
    let strips = ReturnToViewMapStrips {
        strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
    };
    for (strip, caption) in [
        (0u8, "The Summoning"),
        (1, "The Journey"),
        (2, "The Arrival"),
        (3, "The Welcoming"),
    ] {
        let mut state = ReturnToViewPreviewState::default();
        state
            .apply_command(&strips, 0, ReturnToViewCommand::LoadMapStrip { strip })
            .unwrap();
        assert_eq!(state.current_caption, Some(caption));
        assert_eq!(state.current_strip, Some(strip));
    }
}

#[test]
fn g3_location_dat_s11_rtv_hide_clear_set_cell_and_move_and_tick() {
    // formats/location-dat.md §11 command table rows 0x01, 0x02, 0x0A, 0x0C
    // and 0x0D.
    let mut strips = ReturnToViewMapStrips {
        strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
    };
    for (i, cell) in strips.strips[1].iter_mut().enumerate() {
        *cell = 0x20 + (i % 7) as u8;
    }
    let cell = |x: usize, y: usize| y * RTV_STRIP_VISIBLE_COLUMNS + x;
    let mut state = ReturnToViewPreviewState::default();
    state
        .apply_command(&strips, 0, ReturnToViewCommand::LoadMapStrip { strip: 1 })
        .unwrap();
    // 0x0A: write a tile into both the visible and the backing plane.
    state
        .apply_command(
            &strips,
            1,
            ReturnToViewCommand::SetMapCell { tile: 0x77, x: 3, y: 2 },
        )
        .unwrap();
    assert_eq!(state.terrain[cell(3, 2)], 0x77);
    assert_eq!(state.backing[cell(3, 2)], 0x77);
    // 0x00 then a tick scatters the actor into the planes.
    state
        .apply_command(
            &strips,
            2,
            ReturnToViewCommand::SetActor { slot: 5, tile: 0x31, x: 3, y: 2 },
        )
        .unwrap();
    state
        .apply_command(&strips, 3, ReturnToViewCommand::RunPreviewTick { ticks: 1 })
        .unwrap();
    assert_eq!(state.terrain[cell(3, 2)], 0);
    assert_eq!(state.overlay[cell(3, 2)], 0x31);
    // 0x0D: restore the backing at the old cell, step one cell, seven ticks.
    let ticks = state.total_ticks;
    state
        .apply_command(
            &strips,
            4,
            ReturnToViewCommand::MoveActorAndTick { slot: 5, direction: 1 },
        )
        .unwrap();
    assert_eq!(state.total_ticks, ticks + 7);
    assert_eq!((state.actors[5].x, state.actors[5].y), (4, 2));
    assert_eq!(state.terrain[cell(3, 2)], 0x77, "backing restored at the old cell");
    // 0x01: clear the actor's tiles and drawability, restore its cell.
    state
        .apply_command(&strips, 5, ReturnToViewCommand::HideActor { slot: 5 })
        .unwrap();
    assert!(!state.actors[5].drawable);
    assert_eq!((state.actors[5].tile0, state.actors[5].tile1), (0, 0));
    assert_eq!(state.terrain[cell(4, 2)], state.backing[cell(4, 2)]);
    // 0x0C: clear tile and drawability bytes for all thirty-two slots.
    for slot in [0u8, 9, 31] {
        state
            .apply_command(
                &strips,
                6,
                ReturnToViewCommand::SetActor { slot, tile: 0x40, x: 0, y: 0 },
            )
            .unwrap();
    }
    assert_eq!(state.drawable_actor_count(), 3);
    state
        .apply_command(&strips, 7, ReturnToViewCommand::ClearActors)
        .unwrap();
    assert_eq!(state.drawable_actor_count(), 0);
    assert!(state.actors.iter().all(|a| a.tile0 == 0 && a.tile1 == 0));
    assert_eq!(state.actors.len(), 32);
}

// ---------------------------------------------------------------------------
// catalogs/item-list.md
// ---------------------------------------------------------------------------

#[test]
fn g3_item_list_s5_1_equipment_table_matches_every_published_row() {
    // catalogs/item-list.md §5.1: (name, class tag, R-Ready burden, equipped
    // weight stat, base price, attack max) for all forty-eight ids.
    let published: [(&str, u8, u8, u8, u16, u8); 48] = [
        ("Leather Helm", 0x80, 0, 1, 15, 0),
        ("Chain Coif", 0x80, 1, 2, 50, 0),
        ("Iron Helm", 0x80, 2, 3, 120, 0),
        ("Spiked Helm", 0x80, 3, 3, 150, 4),
        ("Small Shield", 0x20, 2, 2, 40, 0),
        ("Large Shield", 0x20, 3, 3, 70, 0),
        ("Spiked Shield", 0x20, 4, 3, 120, 6),
        ("Magic Shield", 0x20, 0, 5, 2000, 0),
        ("Jewel Shield", 0x20, 0, 0, 0, 0),
        ("Cloth Armour", 0x40, 0, 1, 20, 0),
        ("Leather Armour", 0x40, 2, 2, 50, 0),
        ("Ring Mail", 0x40, 4, 3, 100, 0),
        ("Scale Mail", 0x40, 6, 4, 150, 0),
        ("Chain Mail", 0x40, 10, 5, 300, 0),
        ("Plate Mail", 0x40, 12, 7, 700, 0),
        ("Mystic Armour", 0x40, 0, 10, 0, 0),
        ("Dagger", 0x20, 1, 0, 1, 6),
        ("Sling", 0x30, 2, 0, 10, 6),
        ("Club", 0x20, 3, 0, 5, 8),
        ("Flaming Oil", 0x30, 2, 0, 5, 8),
        ("Main Gauche", 0x20, 3, 1, 15, 8),
        ("Spear", 0x20, 4, 0, 7, 10),
        ("Throwing Axe", 0x20, 6, 0, 3, 10),
        ("Short Sword", 0x20, 5, 0, 40, 12),
        ("Mace", 0x20, 7, 0, 50, 15),
        ("Morning Star", 0x20, 8, 0, 60, 15),
        ("Bow", 0x30, 8, 0, 75, 10),
        ("Arrows", 0x00, 0, 0, 10, 1),
        ("Crossbow", 0x30, 6, 0, 150, 12),
        ("Quarrels", 0x00, 0, 0, 15, 1),
        ("Long Sword", 0x20, 9, 0, 70, 15),
        ("2H Hammer", 0x30, 16, 0, 85, 20),
        ("2H Axe", 0x30, 15, 0, 150, 20),
        ("2H Sword", 0x30, 13, 0, 200, 20),
        ("Halberd", 0x30, 18, 0, 250, 30),
        ("Sword of Chaos", 0x30, 0, 0, 0, 99),
        ("Magic Bow", 0x30, 0, 0, 800, 15),
        ("Silver Sword", 0x20, 8, 0, 250, 12),
        ("Magic Axe", 0x20, 0, 0, 1000, 20),
        ("Glass Sword", 0x20, 5, 0, 0, 99),
        ("Jeweled Sword", 0x20, 0, 0, 0, 1),
        ("Mystic Sword", 0x30, 0, 1, 0, 30),
        ("Ring of Invisibility", 0x02, 0, 0, 450, 0),
        ("Ring of Protection", 0x02, 0, 2, 500, 0),
        ("Ring of Regeneration", 0x02, 0, 0, 200, 0),
        ("Amulet/Turning", 0x04, 0, 0, 900, 0),
        ("Spiked Collar", 0x04, 0, 2, 240, 0),
        ("Ankh", 0x04, 0, 0, 0, 0),
    ];
    assert_eq!(EQUIPMENT_COUNT, 48);
    for (id, (name, tag, burden, weight, price, attack)) in published.into_iter().enumerate() {
        assert_eq!(EQUIPMENT_NAMES[id], name, "id {id}");
        assert_eq!(EQUIPMENT_CLASS_TAGS[id], tag, "{name} class tag");
        assert_eq!(EQUIPMENT_READY_BURDENS[id], burden, "{name} burden");
        assert_eq!(EQUIPMENT_EQUIPPED_WEIGHTS[id], weight, "{name} weight");
        assert_eq!(equipment_base_price(id), Some(price), "{name} price");
        assert_eq!(equipment_attack_max(id), Some(attack), "{name} attack max");
    }
}

#[test]
fn g3_item_list_s5_1_2_short_labels_and_the_thirteen_character_threshold() {
    // catalogs/item-list.md §5.1.2: the complete short-label row, and "Ten
    // canonical names reach the buy list's thirteen-character substitution
    // threshold: ids 6, 10, 15, 35, 40, 42, 43, 44, 45, and 46".
    let published: [&str; 48] = [
        "Leath Helm", "Chain Coif", "Iron Helm", "Spkd. Helm", "Sm. Shield", "Lg. Shield",
        "Spkd. Shld", "Mag. Shld", "Jewel Shld", "Cloth", "Leather", "Ring Mail", "Scale",
        "Chain", "Plate", "Myst. Armr", "Dagger", "Sling", "Club", "Flame Oil", "Main Gauch",
        "Spear", "Thrwng Axe", "Sht. Sword", "Mace", "Morn. Star", "Bow", "Arrows", "Crossbow",
        "Quarrels", "Long Sword", "2H Hammer", "2H Axe", "2H Sword", "Halberd", "Chaos Swrd",
        "Magic Bow", "Silver Swd", "Magic Axe", "Glass Swrd", "Jewel Swrd", "Myst. Swrd",
        "Inv. Ring", "Prot. Ring", "Regen Ring", "Am/Turning", "Sp. Collar", "Ankh",
    ];
    assert_eq!(EQUIPMENT_SHORT_LABELS, published);
    let long: Vec<usize> = (0..48).filter(|id| EQUIPMENT_NAMES[*id].len() >= 13).collect();
    assert_eq!(long, vec![6, 10, 15, 35, 40, 42, 43, 44, 45, 46]);
}

#[test]
fn g3_item_list_s5_3_weapon_range_caps_and_effect_codes() {
    // catalogs/item-list.md §5.3: the item-id keyed range/effect table;
    // every row not listed has range cap zero.
    let published: [(usize, u8, u8); 11] = [
        (16, 3, 0),
        (17, 4, 7),
        (19, 4, 3),
        (21, 5, 0),
        (22, 4, 2),
        (25, 2, 0),
        (26, 7, 0),
        (28, 8, 0),
        (34, 2, 0),
        (36, 15, 0),
        (38, 15, 2),
    ];
    for id in 0..EQUIPMENT_COUNT {
        let row = published.iter().find(|(row_id, _, _)| *row_id == id);
        let (cap, effect) = row.map_or((0, 0), |(_, cap, effect)| (*cap, *effect));
        assert_eq!(equipment_weapon_range_cap(id), Some(cap), "id {id} range cap");
        if cap != 0 {
            assert_eq!(equipment_weapon_effect_code(id), Some(effect), "id {id} effect");
        }
    }
}

#[test]
fn g3_item_list_s5_1_1_class_tag_groups_match_the_published_id_lists() {
    // catalogs/item-list.md §5.1.1: the class-tag to id-list table.
    let groups: [(u8, &[usize]); 7] = [
        (0x80, &[0, 1, 2, 3]),
        (0x40, &[9, 10, 11, 12, 13, 14, 15]),
        (
            0x20,
            &[4, 5, 6, 7, 8, 16, 18, 20, 21, 22, 23, 24, 25, 30, 37, 38, 39, 40],
        ),
        (0x30, &[17, 19, 26, 28, 31, 32, 33, 34, 35, 36, 41]),
        (0x02, &[42, 43, 44]),
        (0x04, &[45, 46, 47]),
        (0x00, &[27, 29]),
    ];
    let mut seen = 0;
    for (tag, ids) in groups {
        for id in ids {
            assert_eq!(EQUIPMENT_CLASS_TAGS[*id], tag, "id {id}");
            seen += 1;
        }
    }
    assert_eq!(seen, 48, "the seven groups cover every id exactly once");
}

// ---------------------------------------------------------------------------
// systems/npc-schedules.md
// ---------------------------------------------------------------------------

fn g3_route_npc() -> RuntimeNpc {
    let mut schedule = [0u8; 16];
    schedule[NPC_SCHEDULE_TIME_OFFSET..NPC_SCHEDULE_TIME_OFFSET + 4].copy_from_slice(&[1, 2, 3, 4]);
    RuntimeNpc {
        slot: 1,
        type_byte: 0x50,
        dialog_id: 0,
        schedule,
        state: NPC_STATE_REPLAY_QUEUE,
        x: 5,
        y: 5,
        z: 0,
        cached_wp: 0,
        move_queue: vec![NPC_PATH_DIR_EAST, NPC_PATH_DIR_EAST, NPC_PATH_DIR_SOUTH],
        move_queue_pos: 1,
        stuck_counter: 0,
        active_object: None,
    }
}

#[test]
fn g3_npc_schedules_s9_1_stuck_counter_has_three_disjoint_bands() {
    // systems/npc-schedules.md §9.1 table: 0 replans unconditionally,
    // 1..199 takes the one-in-three draw, 200..204 ages.
    assert_eq!(npc_stuck_counter_band(0), NpcStuckBand::ReplanUnconditionally);
    for counter in [1u16, 2, 3, 4, 100, 199] {
        assert_eq!(npc_stuck_counter_band(counter), NpcStuckBand::ReplanDraw, "{counter}");
    }
    for counter in 200u16..=204 {
        assert_eq!(npc_stuck_counter_band(counter), NpcStuckBand::Aging, "{counter}");
    }
}

#[test]
fn g3_npc_schedules_s9_1_failed_replan_assigns_200_and_ages_five_passes() {
    // systems/npc-schedules.md §9.1: "The value `200` is ... assigned
    // outright"; "entry values `200, 201, 202, 203, 204` are five
    // consecutive passes ... and the **sixth** turn sees a counter of `0`".
    let mut npc = g3_route_npc();
    npc.assign_failed_replan_stuck_value();
    assert_eq!(npc.stuck_counter, 200);
    let mut entries = Vec::new();
    while npc_stuck_counter_band(npc.stuck_counter) == NpcStuckBand::Aging {
        entries.push(npc.stuck_counter);
        npc.age_high_band_stuck_counter();
    }
    assert_eq!(entries, vec![200, 201, 202, 203, 204]);
    assert_eq!(npc.stuck_counter, 0);
}

#[test]
fn g3_npc_schedules_s4_refused_step_counter_resets_the_queue_above_three() {
    // systems/npc-schedules.md §4: "once the counter is greater than three,
    // the move-queue read pointer is reset to the inactive sentinel and the
    // counter is cleared".
    let mut npc = g3_route_npc();
    for expected in 1u16..=3 {
        npc.note_refused_queued_route_step();
        assert_eq!(npc.stuck_counter, expected);
        assert_eq!(npc.peek_move_queue_direction(), Some(NPC_PATH_DIR_EAST));
    }
    npc.note_refused_queued_route_step();
    assert_eq!(npc.stuck_counter, 0);
    assert_eq!(npc.peek_move_queue_direction(), None, "queue inactive");
}

#[test]
fn g3_npc_schedules_s6_arrival_refresh_does_three_things_together() {
    // systems/npc-schedules.md §6: "Both refresh sites do the same three
    // things together: write the cache, return the NPC to the idle state,
    // and discard any queued route."
    let mut npc = g3_route_npc();
    npc.stuck_counter = 2;
    npc.set_settled_at_waypoint(2);
    assert_eq!(npc.cached_wp, 2);
    assert_eq!(npc.state, NPC_STATE_IDLE);
    assert_eq!(npc.peek_move_queue_direction(), None);
    assert!(npc.move_queue.is_empty());
}

#[test]
fn g3_npc_schedules_s6_floor_classification_orders_basement_below_zero() {
    // systems/npc-schedules.md §6 table plus the signed ordering: 0xFF
    // orders below 0x00.
    assert_eq!(schedule_floor_state(0, 0, 0), 2);
    assert_eq!(schedule_floor_state(0, 0xFF, 0), 7);
    assert_eq!(schedule_floor_state(0, 1, 0), 6);
    assert_eq!(schedule_floor_state(0xFF, 0, 0), 5);
    assert_eq!(schedule_floor_state(1, 0, 0), 4);
    assert_eq!(schedule_floor_state(1, 2, 0), 8);
    assert_eq!(schedule_floor_state(0xFF, 1, 0), 8);
    // From the basement's point of view the entry floor is above.
    assert_eq!(schedule_floor_state(0xFF, 0, 0xFF), 6);
}
