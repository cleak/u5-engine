// Closed-audit follow-up, group 1: rules the engine implements but no test
// asserted. Spec files: systems/dungeon-mode.md, systems/endgame.md,
// catalogs/monster-bestiary.md, systems/time.md, formats/npc.md,
// formats/bit.md, systems/stat-arithmetic.md.
//
// Each test names its file and section. Asset-gated tests follow the
// `configured_original_asset_dir` pattern and only read the local files.

// ---------------------------------------------------------------------
// systems/stat-arithmetic.md
// ---------------------------------------------------------------------

/// `stat-arithmetic.md §2/§4`: the word floor-subtract compares signed and
/// "Decrease[s] by the requested amount only when the current value is
/// greater than the amount; otherwise store zero" - a negative field is not
/// greater than a positive amount, so it is stored as zero rather than being
/// driven further negative or wrapped by an unsigned clamp.
#[test]
fn g1_stat_arithmetic_s2_word_floor_subtract_from_a_negative_field_stores_zero() {
    let mut field: i16 = -5;
    floor_sub_word(&mut field, 3);
    assert_eq!(field, 0);
    let mut field: i16 = i16::MIN + 1;
    floor_sub_word(&mut field, 1);
    assert_eq!(field, 0);
}

/// `stat-arithmetic.md §2`: capped add stores the cap when "the resulting
/// value reaches or exceeds the cap" - exact arrival and an already-over-cap
/// field both end on the cap, for both widths.
#[test]
fn g1_stat_arithmetic_s2_capped_add_reaching_or_exceeding_the_cap_stores_the_cap() {
    let mut byte = 94u8;
    capped_add_u8(&mut byte, 5, 99);
    assert_eq!(byte, 99);
    let mut byte = 120u8;
    capped_add_u8(&mut byte, 1, 99);
    assert_eq!(byte, 99, "a field already above the cap still stores the cap");

    let mut word: i16 = 9990;
    capped_add_word(&mut word, 9, 9999);
    assert_eq!(word, 9999);
    let mut word: i16 = 50;
    capped_add_word(&mut word, 0, 40);
    assert_eq!(word, 40);
}

/// `stat-arithmetic.md §2`: floor subtract decreases "only when the current
/// value is greater than the amount" - an equal amount stores zero, and a
/// byte result always lies in `0..old_value`.
#[test]
fn g1_stat_arithmetic_s2_floor_subtract_of_an_equal_amount_stores_zero() {
    for old in 0u8..=40 {
        for amount in 0u8..=40 {
            let mut field = old;
            floor_sub_u8(&mut field, amount);
            let expected = if old > amount { old - amount } else { 0 };
            assert_eq!(field, expected, "{old} - {amount}");
            assert!(field <= old);
        }
    }
    let mut word: i16 = 7;
    floor_sub_word(&mut word, 7);
    assert_eq!(word, 0);
}

fn g1_member(slot: u8, status: u8, hp: u16, max_hp: u16) -> PartyMember {
    PartyMember {
        slot,
        class_byte: b'F',
        status,
        climb_stat: 20,
        mana: 0,
        hp,
        max_hp,
        level: 1,
    }
}

/// `stat-arithmetic.md §3` caller table, "Party food/provision cadence":
/// "Hourly provision use subtracts the number of active eaters and floors at
/// zero." Two units against four eaters at a meal hour leave zero, not a
/// wrapped counter.
#[test]
fn g1_stat_arithmetic_s3_provision_use_floors_the_food_counter_at_zero() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.clock = GameClock::with_date(139, 4, 5, 12, 0).unwrap();
    state.party = (0..4).map(|slot| g1_member(slot, b'G', 20, 20)).collect();
    state.food = 2;
    state.apply_status_provision_pass(true);
    assert_eq!(state.food, 0);
}

/// `stat-arithmetic.md §3` "Party HP recovery and healing": the Ring of
/// Regeneration check "add[s] HP up to the member's current maximum HP".
/// A wearer already at maximum never goes above it, however many rolls land.
#[test]
fn g1_stat_arithmetic_s3_ring_regeneration_never_raises_hp_above_maximum() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party = vec![g1_member(0, b'G', 30, 30)];
    state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]];
    state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    for seed in 0..64u16 {
        state.prng_state = seed;
        state.apply_hourly_ring_regeneration_tick();
        assert_eq!(state.party[0].hp, 30);
    }
}

// ---------------------------------------------------------------------
// formats/bit.md
// ---------------------------------------------------------------------

fn g1_bit_record(width: u16, height: u16, rows: &[u8]) -> Vec<u8> {
    let mut record = Vec::new();
    record.extend_from_slice(&width.to_le_bytes());
    record.extend_from_slice(&height.to_le_bytes());
    record.extend_from_slice(rows);
    record
}

fn g1_bit_list(records: &[Vec<u8>], offsets: Option<&[u16]>) -> Vec<u8> {
    let count = records.len() as u16;
    let mut body = count.to_le_bytes().to_vec();
    let mut next = 2 + 2 * count;
    let mut computed = Vec::new();
    for record in records {
        computed.push(next);
        next += record.len() as u16;
    }
    for offset in offsets.unwrap_or(&computed) {
        body.extend_from_slice(&offset.to_le_bytes());
    }
    for record in records {
        body.extend_from_slice(record);
    }
    body
}

/// `bit.md §3`: "The row stride is `max(1, ceil(width / 8))` bytes ... [a]
/// zero-width record ... still reserves one byte per row, all of it
/// padding", so the record after an eight-row zero-width record starts
/// `4 + 8` bytes later. "A validator that computes the space glyph's record
/// as four bytes will read the remaining ... records at the wrong offsets."
#[test]
fn g1_bit_s3_zero_width_record_reserves_one_padding_byte_per_row() {
    let space = g1_bit_record(0, 8, &[0u8; 8]);
    let glyph = g1_bit_record(3, 1, &[0b1010_0000]);
    let body = g1_bit_list(&[space.clone(), glyph.clone()], None);
    let images = parse_bit_sub_image_list(&body, "fixture.pcs").unwrap();
    assert_eq!(images.len(), 2);
    assert_eq!((images[0].width, images[0].height), (0, 8));
    assert!(images[0].pixels.is_empty(), "a zero-width record paints nothing");
    assert_eq!(images[1].pixels, vec![1, 0, 1]);

    // The four-byte reading of the space record is rejected.
    let wrong = g1_bit_list(&[g1_bit_record(0, 8, &[]), glyph], Some(&[6, 10]));
    assert!(parse_bit_sub_image_list(&wrong, "fixture.pcs").is_err());
    let _ = space;
}

/// `bit.md §3`: "consecutive offsets differ by exactly
/// `4 + max(1, ceil(width / 8)) * height` of the earlier record". A
/// 12-wide record is two bytes per row; an offset table that strides it as
/// a packed `ceil(12 * 3 / 8)` = 5-byte image is rejected.
#[test]
fn g1_bit_s3_consecutive_offsets_follow_the_byte_aligned_row_stride() {
    let first = g1_bit_record(12, 3, &[0xFF, 0xF0, 0x80, 0x00, 0x00, 0x10]);
    let second = g1_bit_record(8, 1, &[0x01]);
    let body = g1_bit_list(&[first.clone(), second.clone()], None);
    let images = parse_bit_sub_image_list(&body, "fixture.bit").unwrap();
    assert_eq!(images.len(), 2);
    // Row 0: twelve set pixels; padding bits after column 11 are not pixels.
    assert_eq!(&images[0].pixels[..12], &[1; 12]);
    assert_eq!(&images[0].pixels[12..24], &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&images[0].pixels[24..], &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(images[1].pixels, vec![0, 0, 0, 0, 0, 0, 0, 1]);

    let packed_stride = g1_bit_list(&[first, second], Some(&[6, 6 + 4 + 5]));
    assert!(parse_bit_sub_image_list(&packed_stride, "fixture.bit").is_err());
}

/// `bit.md §6`: "Require `count` to be at least one and the offset table to
/// fit in the image" and "Require every offset to leave room for a
/// four-byte sub-image header."
#[test]
fn g1_bit_s6_offset_table_and_record_headers_must_fit_the_image() {
    // A count whose table runs past the end.
    let mut body = 5u16.to_le_bytes().to_vec();
    body.extend_from_slice(&12u16.to_le_bytes());
    assert!(parse_bit_sub_image_list(&body, "fixture.bit").is_err());

    // A first offset that leaves only two header bytes.
    let mut body = 1u16.to_le_bytes().to_vec();
    body.extend_from_slice(&4u16.to_le_bytes());
    body.extend_from_slice(&8u16.to_le_bytes());
    assert!(parse_bit_sub_image_list(&body, "fixture.bit").is_err());
}

/// `bit.md §5`: "Record indices are zero-based and the whole range `0`
/// through `count - 1` is addressable ... any implementation that treats
/// index `0` as a header slot or reserves the last record is wrong."
#[test]
fn g1_bit_s5_every_directory_index_including_the_last_is_a_real_record() {
    let records: Vec<Vec<u8>> = (0..10u8)
        .map(|index| g1_bit_record(8, 1, &[1 << (index % 8)]))
        .collect();
    let body = g1_bit_list(&records, None);
    let title = parse_title_bit_body(&body, "fixture").unwrap();
    assert_eq!(title.blocks.len(), 10);
    assert_eq!(title.blocks[0].pixels, vec![0, 0, 0, 0, 0, 0, 0, 1]);
    // Record 9 of 10 - the live intro draw §5 cites - is present.
    assert_eq!(title.blocks[9].pixels, vec![0, 0, 0, 0, 0, 0, 1, 0]);
}

/// `bit.md §2.1, §4.3, §3`: the shipped files classify structurally. Only
/// `WD.BIT` parses raw (count 1, offset 4, 288x49 at 36 bytes a row,
/// accounting for the whole file); the enveloped files declare decoded
/// lengths `TITLE.BIT` 5364 and `BRITISH.BIT` 2116; `WD.BIT` marks 1624
/// lettering positions; `PROPORT.PCS` holds 91 records of a flat 12-byte
/// stride, record 0 being the zero-width space.
#[test]
fn g1_bit_s2_shipped_files_classify_and_measure_as_published() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let read = |name: &str| fs::read(game_dir.join(name)).ok();
    if let Some(wd) = read(WD_BIT_FILE) {
        assert_eq!(wd.len(), 2 + 2 + 4 + 36 * 49);
        assert_eq!(u16::from_le_bytes([wd[0], wd[1]]), 1);
        assert_eq!(u16::from_le_bytes([wd[2], wd[3]]), 4);
        let raw = parse_bit_sub_image_list(&wd, WD_BIT_FILE).unwrap();
        assert_eq!((raw[0].width, raw[0].height), (288, 49));
        assert_eq!(raw[0].pixels.iter().filter(|pixel| **pixel != 0).count(), 1624);
    }
    for (name, declared) in [(TITLE_BIT_FILE, 5364u32), (BRITISH_BIT_FILE, 2116)] {
        if let Some(bytes) = read(name) {
            assert!(
                parse_bit_sub_image_list(&bytes, name).is_err(),
                "{name} must not parse raw"
            );
            assert_eq!(
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                declared,
                "{name} declared decoded length"
            );
        }
    }
    if let Some(pcs) = read(PROPORT_PCS_FILE) {
        let records = parse_bit_family_resource(&pcs, PROPORT_PCS_FILE).unwrap();
        assert_eq!(records.len(), 91);
        assert_eq!((records[0].width, records[0].height), (0, 8));
        assert!(records.iter().all(|record| record.height == 8 && record.width <= 8));
    }
}

// ---------------------------------------------------------------------
// formats/npc.md
// ---------------------------------------------------------------------

/// `npc.md §2`: scene bytes `1..8` are TOWNE, `9..16` DWELLING, `17..24`
/// CASTLE and `25..32` KEEP; the family is `(scene - 1) >> 3`, the sub-map
/// `(scene - 1) & 7`; "Values outside `1..32` do not select one of the four
/// town-style NPC roster files", scene zero included.
#[test]
fn g1_npc_s2_scene_byte_selects_family_and_sub_map() {
    for byte in 0u8..=255 {
        let scene = Scene::new(byte);
        if !(1..=32).contains(&byte) {
            assert!(scene.is_err(), "scene {byte} selects no roster file");
            assert_eq!(Family::from_scene(byte), None);
            continue;
        }
        let scene = scene.unwrap();
        let expected_family = match (byte - 1) >> 3 {
            0 => Family::Towne,
            1 => Family::Dwelling,
            2 => Family::Castle,
            _ => Family::Keep,
        };
        assert_eq!(scene.family, expected_family, "scene {byte}");
        assert_eq!(scene.block, usize::from((byte - 1) & 7), "scene {byte}");
        assert_eq!(scene.byte, byte, "the public scene byte stays one-based");
    }
}

fn g1_npc_files(game_dir: &Path) -> Option<Vec<(&'static str, Vec<u8>)>> {
    let mut files = Vec::new();
    for name in ["TOWNE.NPC", "DWELLING.NPC", "CASTLE.NPC", "KEEP.NPC"] {
        files.push((name, fs::read(game_dir.join(name)).ok()?));
    }
    Some(files)
}

/// `npc.md §3, §9, §10`: every file is exactly 4,608 bytes; in shipped data
/// slot zero's sixteen schedule bytes and its dialog index are zero in every
/// sub-map.
#[test]
fn g1_npc_s3_shipped_files_have_the_fixed_size_and_zeroed_sentinels() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Some(files) = g1_npc_files(&game_dir) else {
        return;
    };
    for (name, bytes) in &files {
        assert_eq!(bytes.len(), 4608, "{name}");
        for sub_map in 0..8 {
            let base = sub_map * 576;
            assert!(bytes[base..base + 16].iter().all(|byte| *byte == 0), "{name}:{sub_map} slot 0");
            assert_eq!(bytes[base + 544], 0, "{name}:{sub_map} slot 0 dialog");
        }
    }
}

/// `npc.md §5.2`: "The shipped waypoint `Z` alphabet is exactly five values:
/// `0x00`, `0x01`, `0x02`, `0x03`, and `0xFF`" and `0xFF` "is used by
/// forty-one waypoints across four locations" - Yew 14, Lord British's
/// Castle 18, Lord Blackthorn's Castle 8, Serpent's Hold 1. "The highest
/// value that appears anywhere is `0x03`, in Lord Blackthorn's Castle."
#[test]
fn g1_npc_s5_2_shipped_waypoint_floor_alphabet_and_basement_counts() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Some(files) = g1_npc_files(&game_dir) else {
        return;
    };
    let mut basement_total = 0;
    for (name, bytes) in &files {
        for sub_map in 0..8 {
            let base = sub_map * 576;
            let mut basement = 0;
            for slot in 1..32 {
                for waypoint in 0..3 {
                    let z = bytes[base + slot * 16 + NPC_SCHEDULE_Z_OFFSET + waypoint];
                    assert!(matches!(z, 0x00..=0x03 | 0xFF), "{name}:{sub_map} slot {slot} z {z:#04x}");
                    if z == 0x03 {
                        assert_eq!((*name, sub_map), ("CASTLE.NPC", 1), "0x03 only in Blackthorn's castle");
                    }
                    if z == 0xFF {
                        basement += 1;
                    }
                }
            }
            let expected = match (*name, sub_map) {
                ("TOWNE.NPC", 3) => 14,
                ("CASTLE.NPC", 0) => 18,
                ("CASTLE.NPC", 1) => 8,
                ("KEEP.NPC", 7) => 1,
                _ => 0,
            };
            assert_eq!(basement, expected, "{name}:{sub_map} basement waypoints");
            basement_total += basement;
        }
    }
    assert_eq!(basement_total, 41);
}

/// `npc.md §7`: "Dialog index `1` is **not** reserved ... exactly one
/// occupied roster slot in each of the four class files carries it:
/// `TOWNE:0` slot 3, `DWELLING:0` slot 1, `CASTLE:0` slot 13, and `KEEP:0`
/// slot 1."
#[test]
fn g1_npc_s7_dialog_index_one_is_an_ordinary_id_used_once_per_file() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Some(files) = g1_npc_files(&game_dir) else {
        return;
    };
    let expected = [
        ("TOWNE.NPC", 3usize),
        ("DWELLING.NPC", 1),
        ("CASTLE.NPC", 13),
        ("KEEP.NPC", 1),
    ];
    for ((name, bytes), (expected_name, expected_slot)) in files.iter().zip(expected) {
        assert_eq!(*name, expected_name);
        let mut hits = Vec::new();
        for sub_map in 0..8 {
            for slot in 1..32 {
                let base = sub_map * 576;
                if bytes[base + 512 + slot] != 0 && bytes[base + 544 + slot] == 1 {
                    hits.push((sub_map, slot));
                }
            }
        }
        assert_eq!(hits, vec![(0, expected_slot)], "{name}");
    }
}

/// `npc.md §6`: "Three shipped roster slots carry [`0xFC`] - Stonegate slots
/// one, two and three (`KEEP.NPC` sub-map 4)", "authored `0xFC` with no
/// dialogue".
#[test]
fn g1_npc_s6_stonegate_slots_one_to_three_are_shadow_lord_class() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Ok(keep) = fs::read(game_dir.join("KEEP.NPC")) else {
        return;
    };
    let base = 4 * 576;
    for slot in 1..=3 {
        assert_eq!(keep[base + 512 + slot], 0xFC, "Stonegate slot {slot}");
        assert_eq!(keep[base + 544 + slot], 0, "Stonegate slot {slot} dialog");
    }
    let mut total = 0;
    for name in ["TOWNE.NPC", "DWELLING.NPC", "CASTLE.NPC", "KEEP.NPC"] {
        let bytes = fs::read(game_dir.join(name)).unwrap();
        for sub_map in 0..8 {
            for slot in 1..32 {
                if bytes[sub_map * 576 + 512 + slot] == 0xFC {
                    total += 1;
                }
            }
        }
    }
    assert_eq!(total, 3, "only the three Stonegate slots are authored 0xFC");
}

// ---------------------------------------------------------------------
// systems/time.md
// ---------------------------------------------------------------------

/// `time.md §4`: `Q` halves the increment, "If the original increment was
/// non-zero but halving rounds it down to zero, it is forced back to one";
/// `T` leaves the minute counter unadvanced; "No other value in this byte
/// alters the increment."
#[test]
fn g1_time_s4_timing_status_tag_scales_only_for_q_and_t() {
    assert_eq!(TimingStatusTag::from_save_byte(b'Q'), TimingStatusTag::HalfTime);
    assert_eq!(TimingStatusTag::from_save_byte(b'T'), TimingStatusTag::NoMinuteLight);
    for base in 0u8..=40 {
        let halved = TimingStatusTag::HalfTime.effective_minutes(base);
        let expected = if base == 0 { 0 } else { (base / 2).max(1) };
        assert_eq!(halved, expected, "Q on {base}");
        assert_eq!(TimingStatusTag::NoMinuteLight.effective_minutes(base), 0);
        assert_eq!(TimingStatusTag::Normal.effective_minutes(base), base);
    }
    for byte in 0u8..=255 {
        if matches!(byte, b'Q' | b'T') {
            continue;
        }
        let tag = TimingStatusTag::from_save_byte(byte);
        assert_eq!(tag.effective_minutes(2), 2, "tag byte {byte:#04x}");
        assert_eq!(tag.save_byte(), byte, "tag byte {byte:#04x} round-trips");
    }
}

/// `time.md §5`: minutes wrap at 60, hours at 24, "The day check is
/// 'greater than 28'", "`month > 13` triggers reset to `1` and year
/// increments". A larger increment "simply adds them to the minute counter
/// and lets the cascade handle any resulting hour or day rolls."
#[test]
fn g1_time_s5_cascade_rolls_every_field_through_its_fixed_threshold() {
    let mut clock = GameClock::with_date(200, 13, 28, 23, 59).unwrap();
    clock.advance_minutes(1);
    assert_eq!(clock, GameClock::with_date(201, 1, 1, 0, 0).unwrap());

    let mut clock = GameClock::with_date(200, 5, 27, 23, 50).unwrap();
    clock.advance_minutes(20);
    assert_eq!(clock, GameClock::with_date(200, 5, 28, 0, 10).unwrap());

    let mut clock = GameClock::with_date(200, 5, 28, 23, 50).unwrap();
    clock.advance_minutes(20);
    assert_eq!(clock, GameClock::with_date(200, 6, 1, 0, 10).unwrap());

    // 364 days a year: one year of days returns to the same date.
    let mut clock = GameClock::with_date(200, 1, 1, 12, 0).unwrap();
    for _ in 0..DAYS_PER_YEAR {
        clock.advance_day();
    }
    assert_eq!(clock, GameClock::with_date(201, 1, 1, 12, 0).unwrap());
}

/// `time.md §8`: the three rare-reagent harvest cookies and the cycling
/// hidden-treasure cookie are "zeroed when the day wraps from 28 to 1, never
/// at ordinary midnight".
#[test]
fn g1_time_s8_month_boundary_zeroes_the_day_cookies_and_ordinary_midnight_does_not() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.clock = GameClock::with_date(139, 4, 27, 23, 59).unwrap();
    state.rare_reagent_harvest_days = [27; RARE_REAGENT_HARVEST_POINT_COUNT];
    state.fixed_hidden_treasure_daily_day = 27;
    state.advance_turn_with_minutes(1);
    assert_eq!(state.clock.day, 28);
    assert_eq!(state.rare_reagent_harvest_days, [27; RARE_REAGENT_HARVEST_POINT_COUNT]);
    assert_eq!(state.fixed_hidden_treasure_daily_day, 27);

    state.clock = GameClock::with_date(139, 4, 28, 23, 59).unwrap();
    state.advance_turn_with_minutes(1);
    assert_eq!((state.clock.month, state.clock.day), (5, 1));
    assert_eq!(state.rare_reagent_harvest_days, [0; RARE_REAGENT_HARVEST_POINT_COUNT]);
    assert_eq!(state.fixed_hidden_treasure_daily_day, 0);
}

/// `time.md §5` hour-gated part: "Provisions are decremented only at 06:00,
/// 12:00, and 18:00 ... No food is spent at other hour changes."
#[test]
fn g1_time_s5_provisions_are_spent_only_at_the_three_meal_hours() {
    for hour in 0u8..24 {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.clock = GameClock::with_date(139, 4, 5, hour, 0).unwrap();
        state.party = (0..3).map(|slot| g1_member(slot, b'G', 20, 20)).collect();
        state.food = 50;
        state.apply_status_provision_pass(true);
        let expected = if matches!(hour, 6 | 12 | 18) { 47 } else { 50 };
        assert_eq!(state.food, expected, "hour {hour}");
    }
}

/// `time.md §6` stage three: "Daylight that *did not* change does not force a
/// visibility repaint."
#[test]
fn g1_time_s6_an_unchanged_daylight_recompute_leaves_visibility_clean() {
    let mut state = britannia_state(open_world_grid(), 10, 20);
    state.clock = GameClock::new(12, 0).unwrap();
    state.torch_counter = 0;
    state.light_spell_counter = 0;
    state.ambient_light = 0;
    state.recompute_daylight();
    assert_eq!(state.ambient_light, FULL_DAYLIGHT);
    assert!(state.visibility_dirty, "a changed value marks visibility dirty");
    state.visibility_dirty = false;
    state.recompute_daylight();
    assert!(!state.visibility_dirty);
}

/// `time.md §7`: a living hideout slot holds a town scene byte `1..8`, a
/// vanquished slot `0xFF`, and "`0` means 'not yet placed' ... treat `0` as
/// neither 'in a town' nor 'vanquished'".
#[test]
fn g1_time_s7_hideout_zero_is_neither_live_nor_vanquished() {
    assert!(!shadowlord_hideout_is_live(0));
    assert!(!shadowlord_hideout_is_vanquished(0));
    for town in 1u8..=8 {
        assert!(shadowlord_hideout_is_live(town));
        assert!(!shadowlord_hideout_is_vanquished(town));
    }
    for other in 9u8..=40 {
        assert!(!shadowlord_hideout_is_live(other), "scene {other} is never a hideout");
    }
    assert!(shadowlord_hideout_is_vanquished(0xFF));
    assert!(!shadowlord_hideout_is_live(0xFF));
}

// ---------------------------------------------------------------------
// catalogs/monster-bestiary.md
// ---------------------------------------------------------------------

const G1_GROUP_BANNERS: [&str; 48] = [
    "WIZARDS", "BARD", "FIGHTER", "x", "VILLAGER", "MERCHANT", "JESTER", "BARD", "PIRATES", "x",
    "CHILD", "BEGGAR", "GUARDS", "x", "BLACKTHORN", "LORD BRITISH", "SEA HORSES", "SQUIDS",
    "SEA SERPENTS", "SHARKS", "GIANT RATS", "BATS", "SPIDERS", "GHOSTS", "SLIME", "GREMLINS",
    "MIMICS", "REAPERS", "GAZERS", "x", "GARGOYLE", "INSECTS", "ORCS", "SKELETONS", "SNAKES",
    "ETTINS", "HEADLESSES", "WISPS", "DAEMONS", "DRAGONS", "SAND TRAPS", "TROLLS", "x", "x",
    "MONGBATS", "CORPSERS", "ROTWORMS", "SHADOW LORD",
];

/// `monster-bestiary.md §2.2`: the forty-eight-entry group banner table,
/// indexed by class id.
#[test]
fn g1_bestiary_s2_2_group_banner_table_matches_every_published_entry() {
    for (class, expected) in G1_GROUP_BANNERS.iter().enumerate() {
        assert_eq!(
            combat_class_group_banner_name(class as u8),
            Some(*expected),
            "class {class}"
        );
    }
    assert_eq!(combat_class_group_banner_name(48), None);
}

/// `monster-bestiary.md §2.2`: "Every entry is uppercase, letters and spaces
/// only ... The longest is twelve characters ... twenty-two of the
/// forty-eight are" the singular name uppercased with an `S` appended.
#[test]
fn g1_bestiary_s2_2_group_banner_shape_and_derived_count() {
    let mut derived = 0;
    for class in 0u8..48 {
        let banner = combat_class_group_banner_name(class).unwrap();
        assert!(banner.len() <= 12, "class {class}");
        if banner != "x" {
            assert!(
                banner.chars().all(|c| c.is_ascii_uppercase() || c == ' '),
                "class {class}: {banner}"
            );
        }
        // Classes 8 and 9 have no singular name (§1); the engine's row label
        // for class 8 comes from the banner itself.
        if matches!(class, 8 | 9) {
            continue;
        }
        let singular = combat_class_stats(class).unwrap().name;
        if format!("{}S", singular.to_ascii_uppercase()) == banner {
            derived += 1;
        }
    }
    assert_eq!(derived, 22);
    assert_eq!(
        (0u8..48)
            .map(|class| combat_class_group_banner_name(class).unwrap().len())
            .max(),
        Some(12)
    );
}

/// `monster-bestiary.md §1/§2.2`: classes 3, 9, 13, 29, 42 and 43 carry the
/// one-character placeholder `x`; "An engine that falls back to the singular
/// name when the group entry looks empty will print `Avatar`, `Wanderer` or
/// `Crawler` where the original prints a single `x`."
#[test]
fn g1_bestiary_s2_2_placeholder_classes_print_x_not_their_singular_name() {
    for class in 0u8..48 {
        let banner = combat_class_group_banner_name(class).unwrap();
        assert_eq!(
            banner == "x",
            matches!(class, 3 | 9 | 13 | 29 | 42 | 43),
            "class {class}"
        );
    }
}

/// `monster-bestiary.md §2.2`: "A hostile whose masked sprite byte is below
/// `0x40` never indexes [the table] and prints the fixed literal `PIRATES`".
#[test]
fn g1_bestiary_s2_2_masked_sprite_below_0x40_prints_the_pirates_literal() {
    for byte in 0u8..0x40 {
        assert_eq!(outdoor_combat_group_banner_name(byte), Some("PIRATES"), "{byte:#04x}");
    }
    assert_eq!(outdoor_combat_group_banner_name(0x94), Some("BATS"));
    assert_eq!(outdoor_combat_group_banner_name(0xFC), Some("SHADOW LORD"));
}

/// `monster-bestiary.md §3`: "Classes two through eleven all carry selector
/// `1` and payload `0`"; class one (Bard) has selector `3`, payload `0`; none
/// of rows 1..11 has scene resistance, theft or pre-gate bypass.
#[test]
fn g1_bestiary_s3_party_and_townsfolk_side_rows_are_published() {
    let bard = combat_ranged_effect_stats(1).unwrap();
    assert_eq!((bard.range_effect_selector, bard.payload), (3, 0));
    for class in 1u8..=11 {
        let row = combat_ranged_effect_stats(class).unwrap();
        if class >= 2 {
            assert_eq!((row.range_effect_selector, row.payload), (1, 0), "class {class}");
        }
        assert!(!row.scene_resistance && !row.food_theft_branch && !row.pre_gate_bypass);
    }
}

/// `monster-bestiary.md §5-§8` trait columns, every class: blank cells mean
/// no decoded trait.
#[test]
fn g1_bestiary_s4_decoded_trait_columns_for_every_class() {
    // (class, splits, half, immune, zero-sel, vanish, special, possess, blink,
    //  summon, poison, turnable, teleport)
    #[rustfmt::skip]
    const ROWS: &[(u8, [bool; 12])] = &[
        (0, [false, false, false, false, false, false, false, false, false, false, true, false]),
        (12, [false; 12]),
        (13, [false, false, true, false, true, false, false, true, false, false, true, true]),
        (14, [false, false, true, false, true, false, true, false, false, false, true, true]),
        (15, [false, false, true, false, true, false, false, true, false, false, true, true]),
        (16, [false, false, false, false, false, false, false, false, false, false, true, false]),
        (17, [false, false, false, false, false, false, false, false, false, true, false, false]),
        (18, [false; 12]),
        (19, [false; 12]),
        (20, [false, false, false, false, false, false, false, false, false, true, false, false]),
        (21, [false; 12]),
        (22, [false, false, false, false, false, false, false, false, false, true, false, false]),
        (23, [false, true, false, false, false, false, false, true, false, false, false, false]),
        (24, [true, false, false, false, false, false, false, false, false, false, false, false]),
        (25, [false; 12]),
        (26, [false, false, false, true, false, false, false, false, false, false, false, false]),
        (27, [false, false, false, true, false, false, false, false, false, false, true, false]),
        (28, [false, false, false, false, false, true, true, false, false, false, true, false]),
        (29, [false; 12]),
        (30, [true, false, false, true, false, true, false, false, false, false, false, false]),
        (31, [false; 12]),
        (32, [false, false, false, true, false, false, false, false, false, false, false, false]),
        (33, [false, true, false, false, false, false, false, false, false, false, false, false]),
        (34, [false, false, false, false, false, false, false, false, false, true, false, false]),
        (35, [false, false, false, true, false, false, false, false, false, false, false, false]),
        (36, [false, false, false, true, false, false, false, false, false, false, false, false]),
        (37, [false, false, false, false, false, false, true, false, false, false, false, true]),
        (38, [false, true, false, false, false, false, true, false, false, false, true, false]),
        (39, [false, false, false, false, false, false, false, false, true, false, false, false]),
        (40, [false; 12]),
        (41, [false; 12]),
        (44, [false; 12]),
        (45, [false; 12]),
        (46, [false, false, false, false, false, false, false, false, false, true, false, false]),
        (47, [false, true, false, false, true, false, true, false, false, false, true, true]),
    ];
    for &(class, expected) in ROWS {
        let t = combat_class_traits(class).unwrap();
        let actual = [
            t.splits,
            t.physical_half,
            t.physical_immune,
            t.zero_selector_stat_row,
            t.vanish_branch,
            t.special_death,
            t.possess,
            t.blink,
            t.summon_daemon,
            t.poison_status_attack,
            t.turnable_attack,
            t.teleport_capable,
        ];
        assert_eq!(actual, expected, "class {class} ({})", t.name);
    }
}

/// `monster-bestiary.md §11 R337 note / §2`: the `zero-selector stat row`
/// trait is carried by exactly six classes (Mimic, Reaper, Gargoyle, Orc,
/// Ettin, Headless); the vanish branch by exactly four (Wanderer,
/// Blackthorn, Lord British, Shadow Lord).
#[test]
fn g1_bestiary_s4_zero_selector_and_vanish_rosters_are_exact() {
    let zero: Vec<u8> = (0u8..48)
        .filter(|class| combat_class_traits(*class).unwrap().zero_selector_stat_row)
        .collect();
    assert_eq!(zero, vec![26, 27, 30, 32, 35, 36]);
    let vanish: Vec<u8> = (0u8..48)
        .filter(|class| combat_class_traits(*class).unwrap().vanish_branch)
        .collect();
    assert_eq!(vanish, vec![13, 14, 15, 47]);
}

/// `monster-bestiary.md §2.1`: "Eighteen of the forty-eight classes are their
/// own companion".
#[test]
fn g1_bestiary_s2_1_eighteen_classes_are_their_own_companion() {
    let own: Vec<u8> = (0u8..48)
        .filter(|class| combat_class_companion(*class) == Some(*class))
        .collect();
    assert_eq!(own.len(), 18, "{own:?}");
    assert_eq!(combat_class_companion(48), None);
}

/// `monster-bestiary.md §5-§8`: the published per-class Reward unit, Drop cap
/// and Charm threshold columns.
#[test]
fn g1_bestiary_s5_published_reward_drop_and_charm_columns() {
    #[rustfmt::skip]
    const ROWS: &[(u8, u8, u8, u8)] = &[
        (16, 8, 0, 20), (17, 13, 0, 8), (18, 18, 0, 8), (19, 6, 0, 5),
        (20, 3, 5, 5), (21, 2, 0, 5), (22, 3, 5, 5), (23, 6, 0, 10), (24, 3, 0, 2),
        (25, 3, 12, 10), (31, 2, 0, 1), (34, 3, 0, 8), (46, 2, 0, 6),
        (26, 8, 20, 12), (27, 11, 25, 12), (28, 6, 0, 25), (29, 9, 0, 12), (30, 11, 0, 5),
        (32, 3, 11, 10), (33, 6, 13, 5), (35, 8, 17, 12), (36, 6, 12, 8), (37, 11, 0, 20),
        (38, 19, 0, 25), (39, 25, 30, 25), (40, 21, 25, 5), (41, 4, 15, 9), (44, 6, 5, 15),
        (45, 11, 0, 8),
        (12, 25, 5, 10), (13, 25, 0, 30), (14, 25, 0, 30), (15, 25, 0, 30), (47, 25, 0, 30),
    ];
    for &(class, reward, drop_cap, charm) in ROWS {
        let stats = combat_class_stats(class).unwrap();
        assert_eq!(stats.reward_unit(), reward, "class {class} reward unit");
        assert_eq!(stats.default_drop_cap, drop_cap, "class {class} drop cap");
        assert_eq!(stats.mass_charm_threshold(), charm, "class {class} charm threshold");
    }
}

// ---------------------------------------------------------------------
// systems/endgame.md
// ---------------------------------------------------------------------

/// `endgame.md §3` step 4: the remap "turns red and brown into green, green
/// into red, blue into magenta and leaves grey, dark grey, yellow and white
/// alone" over "a fixed set of twenty-two tiles".
#[test]
fn g1_endgame_s3_tileset_remap_colour_semantics_and_tile_count() {
    let map = |index: u8| {
        let mut pixel = [index];
        remap_endgame_tileset_pixels(&mut pixel);
        pixel[0]
    };
    // EGA: 1 blue, 2 green, 4 red, 5 magenta, 6 brown, 7 grey, 8 dark grey,
    // 14 yellow, 15 white.
    assert_eq!(map(4), 2, "red to green");
    assert_eq!(map(6), 2, "brown to green");
    assert_eq!(map(2), 4, "green to red");
    assert_eq!(map(1), 5, "blue to magenta");
    for unchanged in [7u8, 8, 14, 15] {
        assert_eq!(map(unchanged), unchanged);
    }
    let mut tiles = ENDGAME_TILESET_REMAP_TILES.to_vec();
    tiles.sort_unstable();
    tiles.dedup();
    assert_eq!(tiles.len(), 22);
    assert!(endgame_tileset_remap_applies(0x44), "the chamber floor goes green");
    assert!(!endgame_tileset_remap_applies(0x4C + 0x100), "actor sprites are untouched");
}

/// `endgame.md §5`: the decision table - victory only when the effective
/// answer is Yes **and** the saved box flag is set.
#[test]
fn g1_endgame_s5_outcome_needs_both_the_final_yes_and_the_box() {
    assert_eq!(endgame_outcome(true, true), EndgameOutcome::Victory);
    assert_eq!(endgame_outcome(true, false), EndgameOutcome::MissingBoxOrRefused);
    assert_eq!(endgame_outcome(false, true), EndgameOutcome::MissingBoxOrRefused);
    assert_eq!(endgame_outcome(false, false), EndgameOutcome::MissingBoxOrRefused);
}

/// `endgame.md §4` tableau layout: party slot `i` starts at (5,9) with the
/// published wedge targets, and "Party slots are populated only for active
/// party indices below the current party count"; Lord British is slot 31,
/// actor byte `0x7C`, starting at (5,8) and settling at (5,3).
#[test]
fn g1_endgame_s4_tableau_placements_follow_the_slot_table() {
    let targets = [(5, 5), (4, 6), (6, 6), (3, 7), (5, 7), (7, 7)];
    for (slot, target) in targets.iter().enumerate() {
        let placement = endgame_tableau_party_placement(slot, b'A').unwrap();
        assert_eq!(placement.active_object_slot, slot);
        assert_eq!(placement.start, (5, 9));
        assert_eq!(placement.target, *target);
    }
    assert!(endgame_tableau_party_placement(6, b'A').is_none());

    let party: Vec<PartyMember> = (0..3).map(|slot| g1_member(slot, b'G', 10, 10)).collect();
    let placements = endgame_tableau_actor_placements(&party);
    let party_slots: Vec<usize> = placements
        .iter()
        .filter(|p| matches!(p.role, EndgameTableauActorRole::PartyMember(_)))
        .map(|p| p.active_object_slot)
        .collect();
    assert_eq!(party_slots, vec![0, 1, 2]);
    let lord = placements
        .iter()
        .find(|p| p.role == EndgameTableauActorRole::LordBritish)
        .unwrap();
    assert_eq!(lord.active_object_slot, 31);
    assert_eq!((lord.tile, lord.type_byte), (0x7C, 0x7C));
    assert_eq!((lord.start, lord.target), ((5, 8), (5, 3)));
    assert!(
        !placements
            .iter()
            .any(|p| p.role == EndgameTableauActorRole::SandalwoodBox),
        "slot 6 is never created before the rite"
    );
}

/// `endgame.md §4/§7`: the box is slot 6, actor byte `0x0E`, created already
/// at (5,4); it later becomes the Orb, actor byte `0x08`.
#[test]
fn g1_endgame_s7_box_placement_and_orb_byte() {
    let box_placement = endgame_tableau_box_placement();
    assert_eq!(box_placement.active_object_slot, 6);
    assert_eq!((box_placement.start, box_placement.target), ((5, 4), (5, 4)));
    assert_eq!(box_placement.tile, 0x0E);
    assert_eq!(ENDGAME_TABLEAU_ORB_ACTOR_BYTE, 0x08);
    assert_eq!(ENDGAME_GATE_CELL, (5, 4));
}

/// `endgame.md §4` "The actor-byte index space": a zero grid cell draws tile
/// `companion_byte + 256`, and companion byte `0x16` "means **draw
/// nothing**".
#[test]
fn g1_endgame_s4_actor_bytes_index_the_upper_tile_half() {
    for (byte, tile) in [
        (0x40u8, 320usize),
        (0x44, 324),
        (0x48, 328),
        (0x4C, 332),
        (0x0E, 270),
        (0x08, 264),
        (0x7C, 380),
    ] {
        assert_eq!(actor_tile_for_byte(byte), Some(tile), "{byte:#04x}");
    }
    assert_eq!(actor_tile_for_byte(0x16), None);
}

/// `endgame.md §6`: the jitter helper commits "the first candidate whose
/// local scene-buffer cell is the authored walkable marker `0x44`. Other
/// cell values are blocked."
#[test]
fn g1_endgame_s6_only_the_authored_0x44_marker_is_walkable_for_jitter() {
    for tile in 0u8..=255 {
        assert_eq!(endgame_tableau_cell_tile_walkable(tile), tile == 0x44, "{tile:#04x}");
    }
}

/// `endgame.md §8.1` "Per-window bindings".
#[test]
fn g1_endgame_s8_1_per_window_panel_bindings() {
    let expected = [
        ("END1", 0u8, 167u16, 124u16, 0u16, 0u16, 1u8),
        ("END1", 1, 191, 90, 64, 0, 2),
        ("END1", 2, 192, 95, 0, 52, 3),
        ("END2", 0, 173, 98, 0, 0, 4),
        ("END2", 1, 157, 90, 0, 92, 5),
        ("END2", 2, 153, 110, 160, 0, 6),
    ];
    for (index, row) in expected.iter().enumerate() {
        let panel = crate::story_layout::endgame_narrative_panel(index).unwrap();
        assert_eq!(
            (
                panel.archive,
                panel.slot,
                panel.width,
                panel.height,
                panel.top_left_x,
                panel.top_left_y,
                panel.end_dat_record
            ),
            *row,
            "window {}",
            index + 1
        );
    }
    assert!(crate::story_layout::endgame_narrative_panel(6).is_none());
}

/// `endgame.md §8.2` "Per-window paragraph rectangles", and "all six windows
/// lay out with the shipped default of five" for the space advance.
#[test]
fn g1_endgame_s8_2_per_window_paragraph_descriptors() {
    let expected = [
        ((172u16, 66u16), (172u16, 320u16), (0u16, 320u16), (126u16, 200u16)),
        ((0, 92), (0, 320), (0, 320), (126, 200)),
        ((0, 9), (0, 320), (196, 320), (42, 148)),
        ((179, 38), (179, 320), (0, 320), (100, 200)),
        ((0, 9), (0, 320), (161, 320), (82, 200)),
        ((0, 0), (0, 154), (0, 320), (112, 200)),
    ];
    for (index, (pen, outside, inside, band)) in expected.iter().enumerate() {
        let d = crate::story_layout::endgame_narrative_paragraph_box(index).unwrap();
        assert_eq!((d.pen_x, d.pen_y), *pen, "window {}", index + 1);
        assert_eq!((d.left_a, d.right_a), *outside, "window {}", index + 1);
        assert_eq!((d.left_b, d.right_b), *inside, "window {}", index + 1);
        assert_eq!((d.band_low, d.band_high), *band, "window {}", index + 1);
        assert_eq!(d.space_advance, 5, "window {}", index + 1);
    }
}

/// `endgame.md §8.2`: only windows 1 and 4 draw `TEXT` title strips - window
/// 1 slot 0 at (216,0) then slot 4 at (152,28); window 4 slot 5 at (224,0)
/// then slot 0 at (176,0).
#[test]
fn g1_endgame_s8_2_title_strips_belong_to_windows_one_and_four() {
    let strips = |index: usize| {
        crate::story_layout::endgame_narrative_title_strips(index)
            .iter()
            .map(|strip| (strip.slot, strip.x, strip.y))
            .collect::<Vec<_>>()
    };
    assert_eq!(strips(0), vec![(0, 216, 0), (4, 152, 28)]);
    assert_eq!(strips(3), vec![(5, 224, 0), (0, 176, 0)]);
    for index in [1usize, 2, 4, 5] {
        assert!(strips(index).is_empty(), "window {}", index + 1);
    }
}

/// `endgame.md §8.2/§8.4/§9.1`: proportional prose is stamped in palette
/// index 15; the parchment is drawn at (40,0); the certificate cursor starts
/// at row 1.
#[test]
fn g1_endgame_s8_ink_parchment_and_certificate_origin() {
    assert_eq!(crate::endgame_narrative_frame::ENDGAME_NARRATIVE_INK_INDEX, 15);
    assert_eq!(
        (ENDGAME_CERTIFICATE_PARCHMENT_X, ENDGAME_CERTIFICATE_PARCHMENT_Y),
        (40, 0)
    );
    assert_eq!(ENDGAME_CERTIFICATE_FIRST_ROW, 1);
}

/// `endgame.md §9.4`: the elapsed interval borrows twenty-eight days from the
/// month delta and thirteen months from the year delta, against the fixed
/// start 139/4/5.
#[test]
fn g1_endgame_s9_4_elapsed_interval_borrows_days_then_months() {
    let at = |y, m, d| endgame_elapsed_campaign_time(GameClock::with_date(y, m, d, 0, 0).unwrap());
    // Day and month both borrow: 140/3/2 - 139/4/5.
    assert_eq!(at(140, 3, 2), (0, 11, 25));
    // Month borrow only.
    assert_eq!(at(141, 2, 9), (1, 11, 4));
    // No borrow.
    assert_eq!(at(140, 6, 10), (1, 2, 5));
}

// ---------------------------------------------------------------------
// systems/dungeon-mode.md
// ---------------------------------------------------------------------

/// `dungeon-mode.md §6.6` (corrected 2026-10-12, R511): the `0x6?` pit
/// draws no rising sprite at all, and its floor pit sprite only "when the
/// low three bits are zero"; "a class-6 cell with any of its low three bits
/// set (a bare `0x62`, for instance) draws no pit sprite at all".
#[test]
#[ignore = "divergence: dungeon-6.6-class6-pit-sprite"]
fn g1_dungeon_s6_6_class_six_pit_sprite_is_floor_only_and_needs_low_bits_clear() {
    assert_eq!(dungeon_object_family_slots(0x60), (None, Some(8)));
    for cell in [0x61u8, 0x62, 0x63, 0x67] {
        assert_eq!(dungeon_object_family_slots(cell), (None, None), "{cell:#04x}");
    }
}

/// `dungeon-mode.md §5`: the writer deny-list "holds six `(dungeon, room)`
/// pairs ... rooms one, six, eleven, and twelve of the Wrong bank and rooms
/// zero and eleven of the Covetous bank", keyed "by the raw dungeon record
/// number, while the bit index uses the collapsed arena bank".
#[test]
fn g1_dungeon_s5_room_clear_deny_list_is_six_raw_record_pairs() {
    let mut denied = Vec::new();
    for record in 0u8..8 {
        for room in 0u8..16 {
            if dungeon_room_clear_is_denied(record, room) {
                denied.push((record, room));
            }
        }
    }
    assert_eq!(denied, vec![(3, 1), (3, 6), (3, 11), (3, 12), (4, 0), (4, 11)]);
    // The collapsed bank of Wrong is 2; bank number 2 used as a record
    // (Destard) denies nothing.
    assert_eq!(dungeon_arena_bank(3), 2);
    assert!(!dungeon_room_clear_is_denied(2, 1));
}

/// `dungeon-mode.md §14`: `arena_bank = 0 if dungeon_record <= 1 else
/// dungeon_record - 1`; Deceit `0..15` through Doom `96..111`, and "Doom
/// room id fifteen reaches record `111`".
#[test]
fn g1_dungeon_s14_arena_index_arithmetic_per_dungeon() {
    let banks: Vec<usize> = (0..8).map(dungeon_arena_bank).collect();
    assert_eq!(banks, vec![0, 0, 1, 2, 3, 4, 5, 6]);
    for (record, first) in [(0u8, 0usize), (2, 16), (3, 32), (4, 48), (5, 64), (6, 80), (7, 96)] {
        let scene = DungeonScene::from_record(record).unwrap();
        assert_eq!(dungeon_room_arena_index(scene, 0xF0), first, "record {record}");
        assert_eq!(dungeon_room_arena_index(scene, 0xFF), first + 15, "record {record}");
    }
    assert_eq!(
        dungeon_room_arena_index(DungeonScene::new(40).unwrap(), 0xFF),
        111
    );
}

/// `dungeon-mode.md §8` chest lifecycle: "A successful dungeon Jimmy rewrites
/// a locked chest cell to the variant bit plus the closed-chest class, which
/// clears the lock/trap sub-type."
#[test]
fn g1_dungeon_s8_jimmy_rewrite_keeps_only_the_variant_bit() {
    for low in 0u8..=0xF {
        let tile = 0x40 | low;
        assert_eq!(dungeon_jimmy_chest_rewrite(tile), 0x40 | (low & 0x08), "{tile:#04x}");
    }
}

/// `dungeon-mode.md §4.1`: "The level byte is stored zero-based and
/// displayed as one more than its stored value ... the displayed range is
/// one through eight."
#[test]
fn g1_dungeon_s4_1_level_label_is_one_based() {
    for level in 0u8..8 {
        assert_eq!(dungeon_display_level(level), u16::from(level) + 1);
    }
}

/// `dungeon-mode.md §3`: facing "zero north, one east, two south, three
/// west".
#[test]
fn g1_dungeon_s3_facing_codes() {
    for (code, facing) in [
        (0u8, Direction::North),
        (1, Direction::East),
        (2, Direction::South),
        (3, Direction::West),
    ] {
        assert_eq!(dungeon_facing_save_code(facing), Some(code));
        assert_eq!(dungeon_facing_from_save_code(code), Some(facing));
    }
    for code in 4u8..=255 {
        assert_eq!(dungeon_facing_from_save_code(code), None);
    }
}

/// `dungeon-mode.md §12.4`: "Only the wall presentation classes `0xB?`,
/// `0xC?`, and `0xD?` stop expansion. Heavy-door and room-trigger families
/// (`0xA?`, `0xE?`, and `0xF?`) paint door glyphs but still return expand";
/// "The open-chest class paints nothing and still returns expand."
#[test]
fn g1_dungeon_s12_4_minimap_flood_stops_only_at_walls() {
    for tile in 0u8..=255 {
        assert_eq!(
            dungeon_minimap_expands(tile),
            !matches!(tile >> 4, 0xB..=0xD),
            "{tile:#04x}"
        );
    }
}

/// `dungeon-mode.md §13.1`: Up is offered when "the cell carries the
/// climbable-with-equipment mark and the party's Grapple byte ... is
/// non-zero"; the mark is "bit `0x08` ... of the stored cell byte, read raw
/// ... so it is honoured at *any* class".
#[test]
fn g1_dungeon_s13_1_gear_mark_offers_up_at_any_class_only_with_the_grapple() {
    let mut state = dungeon_state(open_dungeon_record(), 2, 1, 1);
    for class in 0u8..=0xF {
        let marked = (class << 4) | 0x08;
        let plain = class << 4;
        state.climbing_gear = 1;
        assert!(state.dungeon_klimb_gear_up(marked), "{marked:#04x}");
        assert!(!state.dungeon_klimb_gear_up(plain), "{plain:#04x}");
        state.climbing_gear = 0;
        assert!(!state.dungeon_klimb_gear_up(marked), "{marked:#04x} without grapple");
    }
}

/// `dungeon-mode.md §12`: the `Dir-` chooser reads Up/Left/Right/Down as
/// Ahead/Left/Right/Here "independently of the current facing", and "Letter
/// alternatives such as A/R/L/H and Escape are ignored".
#[test]
fn g1_dungeon_s12_relative_chooser_reads_only_direction_codes() {
    let code = |byte: u8| char::from(byte);
    assert_eq!(dungeon_chooser_focus_from_key(code(INPUT_CODE_NORTH)), Some(DungeonLookFocus::Ahead));
    assert_eq!(dungeon_chooser_focus_from_key(code(INPUT_CODE_WEST)), Some(DungeonLookFocus::Left));
    assert_eq!(dungeon_chooser_focus_from_key(code(INPUT_CODE_EAST)), Some(DungeonLookFocus::Right));
    assert_eq!(dungeon_chooser_focus_from_key(code(INPUT_CODE_SOUTH)), Some(DungeonLookFocus::Here));
    for key in ['A', 'R', 'L', 'H', 'a', 'r', 'l', 'h', '\u{1b}'] {
        assert_eq!(dungeon_chooser_focus_from_key(key), None, "{key:?}");
    }
}

/// `dungeon-mode.md §12` step 3: "right and left rotate that facing by one
/// quarter turn before stepping; here uses the party's current cell", and
/// the focus "wraps through the eight-by-eight level grid".
#[test]
fn g1_dungeon_s12_focus_coordinates_rotate_with_facing_and_wrap() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 0, 7);
    state.player.facing = Direction::North;
    assert_eq!(state.dungeon_look_focus_coord(DungeonLookFocus::Ahead), (0, 6));
    assert_eq!(state.dungeon_look_focus_coord(DungeonLookFocus::Right), (1, 7));
    assert_eq!(state.dungeon_look_focus_coord(DungeonLookFocus::Left), (7, 7));
    assert_eq!(state.dungeon_look_focus_coord(DungeonLookFocus::Here), (0, 7));
    state.player.facing = Direction::South;
    assert_eq!(state.dungeon_look_focus_coord(DungeonLookFocus::Ahead), (0, 0));
    assert_eq!(state.dungeon_look_focus_coord(DungeonLookFocus::Right), (7, 7));
}

/// `dungeon-mode.md §14`: the contact path "determines the cardinal direction
/// from the party to the active monster using wrapped adjacency".
#[test]
fn g1_dungeon_s14_contact_direction_uses_wrapped_adjacency() {
    let state = dungeon_state(open_dungeon_record(), 0, 0, 0);
    assert_eq!(state.dungeon_direction_from_player_to(7, 0), Some(Direction::West));
    assert_eq!(state.dungeon_direction_from_player_to(0, 7), Some(Direction::North));
    assert_eq!(state.dungeon_direction_from_player_to(1, 0), Some(Direction::East));
    assert_eq!(state.dungeon_direction_from_player_to(0, 1), Some(Direction::South));
    assert_eq!(state.dungeon_direction_from_player_to(1, 1), None);
}

/// `dungeon-mode.md §14.1`: the eight families' "spawn-count stat bytes are
/// ten, sixteen, four, six, sixteen, thirteen, four, and three respectively,
/// so Bat and Slime always place the full sixteen".
#[test]
fn g1_dungeon_s14_1_wandering_family_spawn_counts() {
    let counts: Vec<u8> = DUNGEON_MONSTER_COMBAT_CLASSES
        .iter()
        .map(|class| combat_class_stats(*class).unwrap().default_spawn_count)
        .collect();
    assert_eq!(counts, vec![10, 16, 4, 6, 16, 13, 4, 3]);
    for (family, class) in DUNGEON_MONSTER_COMBAT_CLASSES.iter().enumerate() {
        let count = combat_class_stats(*class).unwrap().default_spawn_count;
        if matches!(family, 1 | 4) {
            for seed in [0u8, 7, 200] {
                assert_eq!(resolve_combat_spawn_count(count, seed, None), 16);
            }
        }
    }
}

/// `dungeon-mode.md §8` fountains: sub-type 0 "Sets status to `'G'`", 1
/// "Sets current HP to max without changing status", 2 "Sets status to
/// `'P'`", 3+ "a random HP-damage roll in the inclusive range `0..7`".
#[test]
fn g1_dungeon_s8_fountain_subtypes_apply_their_published_effect() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party = vec![g1_member(0, b'P', 5, 30)];
    assert_eq!(state.apply_dungeon_fountain_effect(0, 0x50).as_deref(), Some("Cured!\n"));
    assert_eq!((state.party[0].status, state.party[0].hp), (b'G', 5));

    state.party[0].status = b'P';
    assert_eq!(state.apply_dungeon_fountain_effect(0, 0x51).as_deref(), Some("Healed!\n"));
    assert_eq!((state.party[0].status, state.party[0].hp), (b'P', 30));

    state.party[0].status = b'G';
    assert_eq!(state.apply_dungeon_fountain_effect(0, 0x52).as_deref(), Some("Poisoned!\n"));
    assert_eq!((state.party[0].status, state.party[0].hp), (b'P', 30));

    for subtype in 3u8..=0xF {
        for seed in 0..32u16 {
            state.party[0] = g1_member(0, b'G', 30, 30);
            state.prng_state = seed;
            assert_eq!(
                state.apply_dungeon_fountain_effect(0, 0x50 | subtype).as_deref(),
                Some("Bad taste.\n")
            );
            assert!(state.party[0].hp >= 23, "damage stays within 0..7");
        }
    }
}

fn g1_dungeon_dat(game_dir: &Path) -> Option<Vec<u8>> {
    fs::read(game_dir.join("DUNGEON.DAT")).ok()
}

fn g1_dungeon_byte(bytes: &[u8], dungeon: usize, z: usize, x: usize, y: usize) -> u8 {
    // `dungeon-mode.md §3`: "file offset `dungeon * 512 + Z * 64 + Y * 8 + X`".
    bytes[dungeon * 512 + z * 64 + y * 8 + x]
}

/// `dungeon-mode.md §3/§5/§8` shipped-data facts: the open-chest class
/// "never occurs in the shipped dungeon file"; the shipped records "contain
/// no `0xA?` cells"; "every static chest cell ... has the variant bit clear"
/// and "Every dungeon chest cell in the shipped dungeon map is trapped";
/// "All six bomb cells ... are plain `0x62`"; "The shipped data holds 25
/// plain `0x08` cells".
#[test]
fn g1_dungeon_s8_shipped_cell_census() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Some(bytes) = g1_dungeon_dat(&game_dir) else {
        return;
    };
    assert_eq!(bytes.len(), 8 * 512);
    assert!(bytes.iter().all(|cell| cell >> 4 != 0x7), "no open-chest class");
    assert!(bytes.iter().all(|cell| cell >> 4 != 0xA), "no room-helper class");
    for cell in bytes.iter().filter(|cell| *cell >> 4 == 0x4) {
        assert_eq!(cell & 0x08, 0, "chest {cell:#04x} variant bit");
        assert_ne!(cell & 0x07, 0, "chest {cell:#04x} is trapped");
    }
    let bombs: Vec<u8> = bytes
        .iter()
        .copied()
        .filter(|cell| matches!(cell, 0x62 | 0x6A))
        .collect();
    assert_eq!(bombs, vec![0x62; 6]);
    assert_eq!(bytes.iter().filter(|cell| **cell == 0x08).count(), 25);
}

/// `dungeon-mode.md §8/§13/§14`: Destard level zero carries `0x60` at (7,3)
/// and (1,7) and Deceit level zero at (1,3); Despise "has no `0xF?`
/// room-trigger cells"; Doom's final room is the level-seven room-id-fifteen
/// trigger at (5,7), reached only by the level-six fall trap above it; no
/// level-seven fall trap exists.
#[test]
fn g1_dungeon_s8_shipped_landmark_cells() {
    let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let Some(bytes) = g1_dungeon_dat(&game_dir) else {
        return;
    };
    assert_eq!(g1_dungeon_byte(&bytes, 2, 0, 7, 3), 0x60);
    assert_eq!(g1_dungeon_byte(&bytes, 2, 0, 1, 7), 0x60);
    assert_eq!(g1_dungeon_byte(&bytes, 0, 0, 1, 3), 0x60);
    assert!(bytes[512..1024].iter().all(|cell| cell >> 4 != 0xF), "Despise has no room cells");
    assert_eq!(g1_dungeon_byte(&bytes, 7, 7, 5, 7), 0xFF);
    assert!(matches!(g1_dungeon_byte(&bytes, 7, 6, 5, 7), 0x61 | 0x69));
    for dungeon in 0..8 {
        for y in 0..8 {
            for x in 0..8 {
                let cell = g1_dungeon_byte(&bytes, dungeon, 7, x, y);
                assert!(!matches!(cell, 0x61 | 0x69), "level-seven fall trap in {dungeon}");
            }
        }
    }
}

/// `dungeon-mode.md §8`: fall and bomb traps roll "an independent inclusive
/// `1..8` HP-damage roll to each non-Dead member among the active party's
/// first six slots ... Sleeping, Poisoned and Ashes members are included;
/// only Dead members and slots outside the active party are skipped, without
/// a damage draw."
#[test]
fn g1_dungeon_s8_floor_trap_sweep_skips_only_dead_members_and_their_draws() {
    let mut with_dead = dungeon_state(open_dungeon_record(), 0, 1, 1);
    with_dead.party = vec![
        g1_member(0, b'G', 30, 30),
        g1_member(1, b'D', 0, 30),
        g1_member(2, b'S', 30, 30),
        g1_member(3, b'P', 30, 30),
    ];
    let mut without_dead = dungeon_state(open_dungeon_record(), 0, 1, 1);
    without_dead.party = vec![
        g1_member(0, b'G', 30, 30),
        g1_member(2, b'S', 30, 30),
        g1_member(3, b'P', 30, 30),
    ];
    for seed in [1u16, 0x1234, 0xBEEF] {
        with_dead.prng_state = seed;
        without_dead.prng_state = seed;
        for member in with_dead.party.iter_mut().chain(without_dead.party.iter_mut()) {
            if member.status != b'D' {
                member.hp = 30;
            }
        }
        with_dead.apply_dungeon_floor_trap_damage();
        without_dead.apply_dungeon_floor_trap_damage();
        assert_eq!(with_dead.prng_state, without_dead.prng_state, "a Dead slot takes no draw");
        assert_eq!((with_dead.party[1].status, with_dead.party[1].hp), (b'D', 0));
        for (a, b) in [(0usize, 0usize), (2, 1), (3, 2)] {
            assert_eq!(with_dead.party[a].hp, without_dead.party[b].hp);
            assert!((22..=29).contains(&with_dead.party[a].hp), "1..8 damage");
        }
    }
}

/// `time.md §5` unconditional part: "If the member is Dead and is also the
/// currently selected active member, the active-member selector is cleared
/// to its no-selection sentinel" - on every invocation, hour change or not.
#[test]
fn g1_time_s5_status_pass_clears_a_selector_that_points_at_a_dead_member() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.party = vec![g1_member(0, b'G', 20, 20), g1_member(1, b'D', 0, 20)];
    state.food = 50;
    state.active_player = Some(1);
    state.apply_status_provision_pass(false);
    assert_eq!(state.active_player, None);

    state.active_player = Some(0);
    state.apply_status_provision_pass(false);
    assert_eq!(state.active_player, Some(0), "a living selection is kept");
}

/// `endgame.md §1/§10/§11`: the endgame "does not advance world time" - the
/// world clock is "Read for the certificate ... It is not advanced by the
/// endgame" - and does not "consume turns".
#[test]
fn g1_endgame_s10_the_sequence_never_advances_the_clock_or_the_turn() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 1;
    state.clock = GameClock::with_date(141, 5, 6, 12, 34).unwrap();
    let clock = state.clock;
    let turn = state.turn;
    state.enter_endgame();
    while state.advance_endgame_display_frame() {}
    state.resolve_endgame_confirmation(false);
    state.resolve_endgame_confirmation(true);
    for _ in 0..64 {
        let _ = state.advance_endgame_display_frame();
    }
    assert_eq!(state.clock, clock);
    assert_eq!(state.turn, turn);
}

/// `endgame.md §5.1`: "Each yes/no prompt is a **blocking single-key read**
/// that accepts only `Y` or `N` and re-reads on anything else."
#[test]
fn g1_endgame_s5_1_box_question_ignores_keys_other_than_y_and_n() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.enter_endgame();
    // The greeting's any-key wait.
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    let waiting = state.endgame.clone();
    for key in ['Q', 'X', '1', ' ', '\r'] {
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
        assert_eq!(state.endgame, waiting, "{key:?} must be re-read");
    }
    handle_play_key_input(&mut state, 'N', "", &dir).unwrap();
    assert_eq!(state.endgame, Some(EndgameState::awaiting_final_confirmation(false)));
    let _ = fs::remove_dir_all(dir);
}
