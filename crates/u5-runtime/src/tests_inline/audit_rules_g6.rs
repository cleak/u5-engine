// Closed-audit rule coverage, group g6: `systems/intro.md`,
// `systems/karma.md`, `systems/containers.md`, `formats/data-ovl.md`,
// `systems/weather.md`, `systems/input.md`, `systems/quest-flags.md`,
// `systems/launcher.md`, `formats/pth.md`.
//
// Each test names the spec file and section it asserts. Engine behaviour is
// not changed here; a test the engine fails is kept and marked
// `#[ignore = "divergence: <id>"]`.

// ---------------------------------------------------------------------------
// formats/pth.md
// ---------------------------------------------------------------------------

/// `formats/pth.md §4`: "The byte value `0x00` ... terminates the current
/// segment ... No other byte terminates a segment." A sign bit with a zero
/// magnitude (`0x80`, `0x08`, `0x88`) is an ordinary zero-movement stroke.
#[test]
fn g6_pth_s4_only_the_nul_byte_terminates_a_segment() {
    let mut bytes = Vec::with_capacity(BRITISH_PTH_LEN);
    for len in [856usize, 548, 411, 964] {
        let mut segment = vec![0x11u8; len];
        segment[0] = 0x80;
        segment[1] = 0x08;
        segment[2] = 0x88;
        bytes.extend(segment);
        bytes.push(0);
    }
    let pth = parse_british_pth(&bytes).unwrap();
    assert_eq!(pth.segments.len(), 4);
    assert_eq!(
        pth.segments.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![856, 548, 411, 964]
    );
    for stroke in &pth.segment(0).unwrap()[..3] {
        assert_eq!((stroke.dx, stroke.dy), (0, 0));
        assert!(stroke.pen_down);
    }
}

/// `formats/pth.md §5`: "This is not a sticky state. The next byte returns
/// to pen-down behaviour if both of its nibble magnitudes are two or
/// smaller." Either nibble at three or more lifts the pen for that byte.
#[test]
fn g6_pth_s5_pen_up_is_per_byte_not_sticky() {
    let sequence = [0x30u8, 0x11, 0x03, 0x22, 0xF0, 0xA2];
    let pen: Vec<bool> = sequence
        .iter()
        .map(|byte| pth_decode_byte(*byte).unwrap().pen_down)
        .collect();
    assert_eq!(pen, vec![false, true, false, true, false, true]);
    // `0xF0`: horizontal magnitude 7, negative; vertical zero.
    let skip = pth_decode_byte(0xF0).unwrap();
    assert_eq!((skip.dx, skip.dy), (-7, 0));
    // `0xA2`: horizontal magnitude 2 negative, vertical 2 positive.
    let down = pth_decode_byte(0xA2).unwrap();
    assert_eq!((down.dx, down.dy), (-2, 2));
}

/// `formats/pth.md §2`: the shipped file's four zero bytes are at offsets
/// `856`, `1405`, `1817` and `2782`, and the last is the file's final byte.
#[test]
fn g6_pth_s2_shipped_file_nul_offsets() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let bytes = std::fs::read(dir.join(BRITISH_PTH_FILE)).unwrap();
    assert_eq!(bytes.len(), BRITISH_PTH_LEN);
    let zeros: Vec<usize> = bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == 0)
        .map(|(offset, _)| offset)
        .collect();
    assert_eq!(zeros, vec![856, 1405, 1817, 2782]);
    let pth = parse_british_pth(&bytes).unwrap();
    assert_eq!(
        pth.segments.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![856, 548, 411, 964]
    );
}

// ---------------------------------------------------------------------------
// systems/launcher.md
// ---------------------------------------------------------------------------

/// `systems/launcher.md §3`: "Startup folds the first character to uppercase"
/// and "Only the first character of the first argument is relevant".
#[test]
fn g6_launcher_s3_only_the_first_character_selects_the_driver() {
    for (arg, family) in [
        ("C", DisplayDriverFamily::Cga),
        ("e", DisplayDriverFamily::Ega),
        ("t", DisplayDriverFamily::Tandy),
        ("T", DisplayDriverFamily::Tandy),
        ("h", DisplayDriverFamily::Hercules),
        ("H", DisplayDriverFamily::Hercules),
        ("Ega", DisplayDriverFamily::Ega),
        ("hx", DisplayDriverFamily::Hercules),
        ("cGA", DisplayDriverFamily::Cga),
    ] {
        assert_eq!(
            parse_explicit_driver_selector(Some(arg)),
            Some(family),
            "{arg}"
        );
    }
    // A later character never rescues an unrecognised first one.
    assert_eq!(parse_explicit_driver_selector(Some("xE")), None);
    assert_eq!(parse_explicit_driver_selector(Some("")), None);
}

// ---------------------------------------------------------------------------
// systems/quest-flags.md
// ---------------------------------------------------------------------------

fn g6_falsehood_host_state() -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.resident_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    state.keys = 0;
    state.gems = 0;
    state.torches = 0;
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.scroll_stock = [0; SCROLL_COUNT];
    state.potion_stock = [0; POTION_COUNT];
    state
}

/// `systems/quest-flags.md §5`: "If the recorded value is anything but the
/// Shadowlord of Falsehood - the Hatred value, the Cowardice value, or the
/// no-host marker ... the cleanup returns immediately. Nothing is printed and
/// nothing is taken."
#[test]
fn g6_quest_flags_s5_hatred_and_cowardice_hosts_suppress_the_theft() {
    for host in [SHADOWLORD_HATRED_INDEX, SHADOWLORD_COWARDICE_INDEX] {
        let mut state = g6_falsehood_host_state();
        state.resident_shadowlord = Some(host);
        state.keys = 3;
        state.gems = 3;
        state.torches = 3;
        state.gold = 50;
        state.equipment_stock[10] = 2;
        let prng = state.prng_state;
        assert!(!state.run_final_conversation_cleanup_with_seed(0x0123));
        assert_eq!((state.keys, state.gems, state.torches), (3, 3, 3));
        assert_eq!(state.gold, 50);
        assert_eq!(state.equipment_stock[10], 2);
        assert_eq!(state.prng_state, prng, "no draw is taken");
    }
}

/// `systems/quest-flags.md §5`: "At most one item, or one gold amount, is
/// lost per conversation." With all three of keys, gems and torches carried,
/// exactly one unit of one of them goes, and nothing further down the
/// cascade is touched.
#[test]
fn g6_quest_flags_s5_theft_takes_exactly_one_unit() {
    for seed in [0x0001u16, 0x0123, 0x0456, 0x0789, 0x0abc, 0x0fff] {
        let mut state = g6_falsehood_host_state();
        state.keys = 3;
        state.gems = 3;
        state.torches = 3;
        state.gold = 40;
        state.equipment_stock[47] = 1;
        state.scroll_stock[7] = 1;
        assert!(state.run_final_conversation_cleanup_with_seed(seed));
        let total = u16::from(state.keys) + u16::from(state.gems) + u16::from(state.torches);
        assert_eq!(total, 8, "seed {seed:#06x}");
        assert_eq!(state.gold, 40);
        assert_eq!(state.equipment_stock[47], 1);
        assert_eq!(state.scroll_stock[7], 1);
    }
}

/// `systems/quest-flags.md §5` step 1: the pick "re-draw[s] until it lands on
/// one the party actually has". With torches the only stocked counter, the
/// torch counter loses the unit on every seed.
#[test]
fn g6_quest_flags_s5_theft_redraws_until_it_hits_a_carried_counter() {
    for seed in [0x0001u16, 0x0222, 0x0333, 0x0444, 0x0555] {
        let mut state = g6_falsehood_host_state();
        state.torches = 2;
        assert!(state.run_final_conversation_cleanup_with_seed(seed));
        assert_eq!((state.keys, state.gems, state.torches), (0, 0, 1));
    }
}

/// `systems/quest-flags.md §5` step 4: the gold fallback subtracts `1..15`
/// "floored at zero".
#[test]
fn g6_quest_flags_s5_gold_fallback_floors_at_zero() {
    let mut state = g6_falsehood_host_state();
    state.gold = 0;
    assert!(state.run_final_conversation_cleanup_with_seed(0x0321));
    assert_eq!(state.gold, 0);

    let mut state = g6_falsehood_host_state();
    state.gold = 100;
    assert!(state.run_final_conversation_cleanup_with_seed(0x0321));
    assert!((85..=99).contains(&state.gold), "gold {}", state.gold);
}

/// `systems/quest-flags.md §4`: "The write is a saturating add of one at the
/// indexed slot rather than a Boolean bit, so a script that emits the same
/// index twice leaves the value at two."
#[test]
fn g6_quest_flags_s4_same_generic_index_twice_reads_two() {
    let mut state = test_state(open_grid(), 1, 1);
    state.conversation_signal_flags = [0; TLK_GENERIC_SIGNAL_COUNT];
    state.record_tlk_signal_flags(&[3, 3]);
    assert_eq!(state.conversation_signal_flags[3], 2);
    assert!(
        state
            .conversation_signal_flags
            .iter()
            .enumerate()
            .all(|(index, value)| index == 3 || *value == 0)
    );
}

/// `systems/quest-flags.md §3` step 1: "The active scene id selects the
/// slot. This is per scene, not merely per `.TLK` file class." Setting a bit
/// in one scene's word leaves every other scene's word untouched.
#[test]
fn g6_quest_flags_s3_branch_bank_is_one_word_per_scene() {
    let mut state = test_state(open_grid(), 1, 1);
    state.talk_branch_flags.clear();
    let first = Scene::new(1).unwrap();
    let second = Scene::new(2).unwrap();
    state.merge_talk_branch_flags(first, tlk_scene_branch_mask(5));
    assert!(tlk_scene_branch_is_set(
        state.talk_branch_flags[&first.byte],
        5
    ));
    assert!(!tlk_scene_branch_is_set(
        state
            .talk_branch_flags
            .get(&second.byte)
            .copied()
            .unwrap_or(0),
        5
    ));
    // OR semantics: a second bit joins the first.
    state.merge_talk_branch_flags(first, tlk_scene_branch_mask(9));
    assert_eq!(state.talk_branch_flags[&first.byte], (1 << 5) | (1 << 9));
}

// ---------------------------------------------------------------------------
// systems/weather.md
// ---------------------------------------------------------------------------

fn g6_ship_state(wind: WindState, type_byte: u8, tile: u8, counter: u8) -> PlayState {
    let mut state = world_state(open_world_grid(), 100, 100);
    state.wind = wind;
    state.wind_save_byte = wind.save_byte();
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[1] = ActiveObject {
        type_byte,
        tile,
        x: 60,
        y: 60,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: counter,
    };
    state
}

/// `systems/weather.md §7` steps 1-2: "Calm wind: no move, and the counter is
/// not touched." and "An 'every turn' entry: the slot moves, and the counter
/// is not touched."
#[test]
fn g6_weather_s7_calm_and_every_turn_leave_the_counter_alone() {
    let mut state = g6_ship_state(WindState::Calm, 0x2C, 0x2C, 7);
    let before = (state.active_objects[1].x, state.active_objects[1].y);
    assert_eq!(
        state.try_drift_active_ship(1, &mut None),
        ActiveShipWind::Stalled
    );
    assert_eq!(state.active_objects[1].aux3, 7);
    assert_eq!(
        (state.active_objects[1].x, state.active_objects[1].y),
        before
    );

    // North-facing frame under an East wind is "every turn".
    let mut state = g6_ship_state(WindState::East, 0x2C, 0x2C, 7);
    let _ = state.try_drift_active_ship(1, &mut None);
    assert_eq!(state.active_objects[1].aux3, 7);
}

/// `systems/weather.md §7`: "An inherited value at or above the cap makes the
/// first capped pass skip and reset; an inherited 255 wraps to 0, so the ship
/// moves on that pass."
#[test]
fn g6_weather_s7_inherited_counter_values() {
    let mut state = g6_ship_state(WindState::North, 0x2C, 0x2C, 200);
    assert_eq!(
        state.try_drift_active_ship(1, &mut None),
        ActiveShipWind::Stalled
    );
    assert_eq!(state.active_objects[1].aux3, 0);

    let mut state = g6_ship_state(WindState::North, 0x2C, 0x2C, 255);
    let _ = state.try_drift_active_ship(1, &mut None);
    assert_eq!(state.active_objects[1].aux3, 0, "255 wraps to 0 and moves");
}

/// `systems/weather.md §7`: "The row is selected by the type byte (record
/// byte 0), not by the displayed-tile byte." A north type byte showing an
/// east tile still takes the North-wind "2 of 3" row (an east row would be
/// "every turn" and leave the counter alone).
#[test]
fn g6_weather_s7_cadence_row_comes_from_the_type_byte() {
    let mut state = g6_ship_state(WindState::North, 0x2C, 0x2D, 0);
    let _ = state.try_drift_active_ship(1, &mut None);
    assert_eq!(state.active_objects[1].aux3, 1);
}

// ---------------------------------------------------------------------------
// systems/input.md
// ---------------------------------------------------------------------------

fn g6_typed_quantity(keys: &[u8]) -> (crate::shop_runtime::TypedQuantityEntry, bool) {
    let mut entry = crate::shop_runtime::TypedQuantityEntry::default();
    let mut submitted = false;
    for key in keys {
        submitted = entry.apply(*key);
        if submitted {
            break;
        }
    }
    (entry, submitted)
}

/// `systems/input.md §8`, typed-number reader: "Escape erases the whole echo
/// and returns the cursor to the first echo cell **without cancelling the
/// prompt**" and "**the west direction code erases one as well**".
#[test]
fn g6_input_s8_typed_number_escape_clears_and_west_erases() {
    let (entry, submitted) = g6_typed_quantity(&[b'4', b'2', 0x1B]);
    assert!(!submitted, "Escape does not end the prompt");
    assert_eq!(entry.as_str(), "");
    let (entry, _) = g6_typed_quantity(&[b'4', b'2', 0x1B, b'7']);
    assert_eq!(entry.as_str(), "7", "the prompt stays open after Escape");

    let (entry, submitted) = g6_typed_quantity(&[b'4', b'2', INPUT_CODE_WEST]);
    assert!(!submitted);
    assert_eq!(entry.as_str(), "4");
    let (entry, _) = g6_typed_quantity(&[b'4', b'2', 0x08]);
    assert_eq!(entry.as_str(), "4");
}

/// `systems/input.md §8`: "Return is the only exit, and an empty buffer
/// submits as zero; and every other key - space, letter, punctuation - is
/// discarded with no echo"; "a leading `+` or `-` is accepted only in the
/// first position".
#[test]
fn g6_input_s8_typed_number_return_only_exit_and_discards() {
    let (entry, submitted) = g6_typed_quantity(&[b'\r']);
    assert!(submitted);
    assert_eq!(entry.quantity(), 0);

    let (entry, submitted) = g6_typed_quantity(&[b' ', b'A', b'.', b'3', b'x']);
    assert!(!submitted);
    assert_eq!(entry.as_str(), "3");

    let (entry, _) = g6_typed_quantity(&[b'5', b'-']);
    assert_eq!(entry.as_str(), "5", "a sign after a digit is dropped");
    let (entry, _) = g6_typed_quantity(&[b'-', b'-', b'4']);
    assert_eq!(entry.as_str(), "-4");

    let (entry, submitted) = g6_typed_quantity(&[b'1', b'2', b'\n']);
    assert!(submitted);
    assert_eq!(entry.quantity(), 12);
}

// ---------------------------------------------------------------------------
// systems/containers.md
// ---------------------------------------------------------------------------

/// `systems/containers.md §4`: the full forty-eight-row equipment pool, in
/// save order, with "Disabled" rows at 8, 15, 35, 39, 40, 41 and 47.
#[test]
fn g6_containers_s4_equipment_pool_threshold_table() {
    let expected: [Option<u8>; 48] = [
        Some(10),
        Some(10),
        Some(15),
        Some(20),
        Some(10),
        Some(15),
        Some(20),
        Some(28),
        None,
        Some(15),
        Some(15),
        Some(20),
        Some(20),
        Some(20),
        Some(24),
        None,
        Some(5),
        Some(10),
        Some(10),
        Some(10),
        Some(10),
        Some(10),
        Some(10),
        Some(10),
        Some(15),
        Some(15),
        Some(15),
        Some(10),
        Some(15),
        Some(10),
        Some(20),
        Some(20),
        Some(20),
        Some(20),
        Some(20),
        None,
        Some(23),
        Some(23),
        Some(23),
        None,
        None,
        None,
        Some(23),
        Some(23),
        Some(23),
        Some(23),
        Some(15),
        None,
    ];
    for (row, threshold) in expected.iter().enumerate() {
        assert_eq!(chest_secondary_pool_threshold(row), *threshold, "row {row}");
    }
    // A disabled row never succeeds, at any class or roll.
    for row in [8usize, 15, 35, 39, 40, 41, 47] {
        assert_eq!(chest_secondary_pool_threshold(row), None, "row {row}");
    }
}

/// `systems/containers.md §4.1`: "Each successful grant prints one fixed line
/// that names only the family" and "Each line below ends with exactly one
/// line feed."
#[test]
fn g6_containers_s4_1_family_lines() {
    for (line, expected) in [
        (CHEST_TRANSCRIPT_FOOD_LINE, "some food!\n"),
        (CHEST_TRANSCRIPT_TORCHES_LINE, "some torches!\n"),
        (CHEST_TRANSCRIPT_GEMS_LINE, "a gem!\n"),
        (CHEST_TRANSCRIPT_KEYS_LINE, "a ring of keys!\n"),
        (CHEST_TRANSCRIPT_SCROLL_LINE, "a scroll!\n"),
        (CHEST_TRANSCRIPT_POTION_LINE, "a potion!\n"),
        (CHEST_TRANSCRIPT_GOLD_LINE, "a sack of gold!\n"),
        (CHEST_TRANSCRIPT_MARKER_LINE, "a chest!\n"),
        (CHEST_TRANSCRIPT_HELM_LINE, "a helm!\n"),
        (CHEST_TRANSCRIPT_SHIELD_LINE, "a shield!\n"),
        (CHEST_TRANSCRIPT_ARMOUR_LINE, "some armour!\n"),
        (CHEST_TRANSCRIPT_WEAPON_LINE, "a weapon!\n"),
        (CHEST_TRANSCRIPT_RING_LINE, "a ring!\n"),
        (CHEST_TRANSCRIPT_AMULET_LINE, "an amulet!\n"),
    ] {
        assert_eq!(line, expected);
    }
    assert_eq!(CHEST_TRANSCRIPT_FOUND_HEADING, "Found:\n");
    assert_eq!(CHEST_TRANSCRIPT_EMPTY_LINE, "Chest empty!\n");
    assert_eq!(CHEST_TRANSCRIPT_TRAPPED_LINE, "Trapped!\n");
}

/// `systems/containers.md §4`: the equipment pool is "four helms, five
/// shields, seven armours, twenty-six weapons, three rings, and three
/// amulets", and `§4.1`: "Equipment rows print the category line only".
#[test]
fn g6_containers_s4_1_equipment_line_follows_the_save_order_band() {
    let mut counts = std::collections::HashMap::new();
    for row in 0..CHEST_SECONDARY_POOL_ROW_COUNT {
        *counts
            .entry(chest_secondary_pool_transcript_line(row))
            .or_insert(0usize) += 1;
    }
    assert_eq!(counts[CHEST_TRANSCRIPT_HELM_LINE], 4);
    assert_eq!(counts[CHEST_TRANSCRIPT_SHIELD_LINE], 5);
    assert_eq!(counts[CHEST_TRANSCRIPT_ARMOUR_LINE], 7);
    assert_eq!(counts[CHEST_TRANSCRIPT_WEAPON_LINE], 26);
    assert_eq!(counts[CHEST_TRANSCRIPT_RING_LINE], 3);
    assert_eq!(counts[CHEST_TRANSCRIPT_AMULET_LINE], 3);
    assert_eq!(
        chest_secondary_pool_transcript_line(3),
        CHEST_TRANSCRIPT_HELM_LINE
    );
    assert_eq!(
        chest_secondary_pool_transcript_line(4),
        CHEST_TRANSCRIPT_SHIELD_LINE
    );
    assert_eq!(
        chest_secondary_pool_transcript_line(9),
        CHEST_TRANSCRIPT_ARMOUR_LINE
    );
    assert_eq!(
        chest_secondary_pool_transcript_line(16),
        CHEST_TRANSCRIPT_WEAPON_LINE
    );
    assert_eq!(
        chest_secondary_pool_transcript_line(42),
        CHEST_TRANSCRIPT_RING_LINE
    );
    assert_eq!(
        chest_secondary_pool_transcript_line(45),
        CHEST_TRANSCRIPT_AMULET_LINE
    );
}

/// `systems/containers.md §4.1` (issue #298 Q18): "both tile bytes of the new
/// active-object record hold the family id"; "The quantity or subtype rides in
/// the record's fifth byte"; each grant takes "the highest free slot searching
/// down from slot 31"; "The record's last byte is cleared outside combat".
#[test]
fn g6_containers_s4_1_staged_grant_record_shape() {
    let mut state = test_state(open_grid(), 1, 1);
    // Family ids: gold 2, then helm 9.
    assert!(state.stage_surface_chest_grant(2, 1, 2, 17));
    assert!(state.stage_surface_chest_grant(2, 1, 9, 3));
    let gold = state.active_objects[31];
    assert_eq!((gold.type_byte, gold.tile), (2, 2));
    assert_eq!((gold.x, gold.y), (2, 1));
    assert_eq!(gold.aux1, 17);
    assert_eq!(gold.aux3, 0);
    let helm = state.active_objects[30];
    assert_eq!((helm.type_byte, helm.tile, helm.aux1), (9, 9, 3));
}

/// `systems/containers.md §4.1`: "The record's last byte is ... set to `0x20`
/// when the scene byte is `0x80` or above."
#[test]
fn g6_containers_s4_1_staged_grant_marks_combat_records() {
    let mut state = test_state(open_grid(), 1, 1);
    state.combat_active = true;
    assert!(state.stage_surface_chest_grant(2, 1, 8, 1));
    assert_eq!(state.active_objects[31].aux3, 0x20);
}

/// `systems/containers.md §4`: the slot allocator "returns the first index
/// whose class byte is free; when every entry is occupied it returns index
/// zero as well. The caller reads a returned index of zero as 'no slot
/// available' ... index zero of the active-object table can never receive a
/// chest-loot grant even when it is genuinely free".
#[test]
fn g6_containers_s4_slot_zero_never_receives_a_grant() {
    let mut state = test_state(open_grid(), 1, 1);
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    for slot in 1..OOL_SLOTS {
        state.active_objects[slot] = ActiveObject {
            type_byte: 0x50,
            tile: 0x50,
            x: 9,
            y: 9,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
    }
    state.active_objects[0] = ActiveObject::empty();
    assert!(!state.stage_surface_chest_grant(2, 1, 2, 5));
    assert!(state.active_objects[0].is_empty());
}

/// `systems/containers.md §4.1` step 3: "If no grant succeeded - nothing
/// rolled, or every success was dropped - print `Chest empty!`". With the
/// active-object table full, every success is dropped.
#[test]
fn g6_containers_s4_1_full_table_prints_chest_empty() {
    for seed in [0x0001u16, 0x1234, 0x7777] {
        let mut state = test_state(open_grid(), 1, 1);
        state
            .active_objects
            .resize(OOL_SLOTS, ActiveObject::empty());
        for slot in 1..OOL_SLOTS {
            state.active_objects[slot] = ActiveObject {
                type_byte: 0x50,
                tile: 0x50,
                x: 9,
                y: 9,
                z: 0,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            };
        }
        state.prng_state = seed;
        assert_eq!(
            state.generate_surface_object_chest_content(2, 1, 30),
            CHEST_TRANSCRIPT_EMPTY_LINE
        );
    }
}

/// `systems/containers.md §4.1`: "Before the first successful grant print the
/// heading `Found:` plus a line feed, once per opening; each grant then prints
/// just its row."
#[test]
fn g6_containers_s4_1_found_heading_prints_once() {
    let mut seen_multi = false;
    for seed in 0..64u16 {
        let mut state = test_state(open_grid(), 1, 1);
        state.prng_state = seed;
        let text = state.generate_surface_object_chest_content(2, 1, 30);
        if text == CHEST_TRANSCRIPT_EMPTY_LINE {
            continue;
        }
        assert!(text.starts_with(CHEST_TRANSCRIPT_FOUND_HEADING), "{text:?}");
        assert_eq!(text.matches("Found:").count(), 1, "{text:?}");
        let staged = state
            .active_objects
            .iter()
            .skip(1)
            .filter(|o| !o.is_empty())
            .count();
        assert_eq!(text.lines().count(), staged + 1, "{text:?}");
        if staged > 1 {
            seen_multi = true;
        }
    }
    assert!(seen_multi, "multiple rows can succeed for one chest");
}

/// `systems/containers.md §4.1`, draw order: "an index whose threshold
/// exceeds the class is skipped **without** its `1..30` draw". A class-zero
/// chest has no eligible primary row and one equipment attempt
/// (`floor(0 / 2) + 1`), so it spends exactly the one pool-index draw and
/// reports empty.
#[test]
fn g6_containers_s4_1_class_zero_chest_takes_one_draw() {
    let mut state = test_state(open_grid(), 1, 1);
    state.prng_state = 0x2468;
    let mut expected = state.prng_state;
    let _ = u5_prng_range_u16(&mut expected, 0, 47);
    assert_eq!(
        state.generate_surface_object_chest_content(2, 1, 0),
        CHEST_TRANSCRIPT_EMPTY_LINE
    );
    assert_eq!(state.prng_state, expected);
}

// ---------------------------------------------------------------------------
// formats/data-ovl.md
// ---------------------------------------------------------------------------

/// `formats/data-ovl.md §5.2`: `UNDER.DAT` "is exactly 65,536 bytes, which is
/// 256 chunks of 256 bytes, one per cell of the chunk grid", and its chunk
/// index "is the identity map": the chunk at grid index `n` is stored at
/// `n * 256`.
#[test]
fn g6_data_ovl_s5_2_underworld_chunk_index_is_the_identity() {
    assert_eq!(UNDER_DAT_LEN, 65_536);
    for chunk_y in 0..16usize {
        for chunk_x in 0..16usize {
            let grid_index = chunk_y * 16 + chunk_x;
            assert_eq!(
                under_dat_offset(chunk_x * 16, chunk_y * 16),
                grid_index * 256
            );
            assert_eq!(
                under_dat_offset(chunk_x * 16 + 15, chunk_y * 16 + 15),
                grid_index * 256 + 255
            );
        }
    }
    // Decoding stores block `n` at chunk `n`.
    let mut file = vec![0u8; UNDER_DAT_LEN];
    for (block, chunk) in file.chunks_mut(256).enumerate() {
        chunk.fill(block as u8);
    }
    let grid = decode_underworld_map_bytes(&file).unwrap();
    for chunk_y in 0..16usize {
        for chunk_x in 0..16usize {
            let cell = (chunk_y * 16 + 7) * WORLD_SIDE + chunk_x * 16 + 3;
            assert_eq!(grid[cell] as usize, chunk_y * 16 + chunk_x);
        }
    }
}

/// `formats/data-ovl.md §5.2`: "In shipped data exactly 205 entries are
/// non-sentinel, and their values run `0, 1, 2, ... 204` in ascending grid
/// order".
#[test]
fn g6_data_ovl_s5_2_shipped_chunk_index_runs_in_ascending_order() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let data = std::fs::read(dir.join("DATA.OVL")).unwrap();
    let table = find_britannia_chunk_index(&data).unwrap();
    let stored: Vec<u8> = table
        .iter()
        .copied()
        .filter(|entry| *entry != BRIT_WATER_SENTINEL)
        .collect();
    assert_eq!(stored.len(), 205);
    assert_eq!(stored, (0..205).map(|n| n as u8).collect::<Vec<_>>());
    assert_eq!(table.len() - stored.len(), 51);
}

/// `formats/data-ovl.md §5.1`: "Wind-status labels, stored in the public
/// presentation order Calm, North, South, East, and West".
#[test]
fn g6_data_ovl_s5_1_wind_labels_in_presentation_order() {
    let labels: Vec<&str> = (0..=4u8)
        .map(|byte| WindState::from_save_byte(byte).name())
        .collect();
    assert_eq!(labels, vec!["Calm", "North", "South", "East", "West"]);
}

// ---------------------------------------------------------------------------
// systems/karma.md
// ---------------------------------------------------------------------------

/// `systems/karma.md §4.1`: the step counter's "only increment is in the
/// resident per-turn party-upkeep pass ... which advances it by one and
/// saturates it at two hundred fifty-five."
#[test]
fn g6_karma_s4_1_step_counter_saturates_at_255() {
    let mut state = test_state(open_grid(), 1, 1);
    state.toll_progress = 254;
    state.advance_turn();
    assert_eq!(state.toll_progress, 255);
    state.advance_turn();
    assert_eq!(state.toll_progress, 255);
}

fn g6_payment_state(speaker_type: u8) -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
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
            type_byte: speaker_type,
            dialog_id: 1,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.active_conversation_npc_slot = Some(1);
    state
}

/// `systems/karma.md §4.1` gate 1: "An unaffordable demand prints the refusal
/// line and never reaches any standing logic." The purse and the counter are
/// unchanged.
#[test]
fn g6_karma_s4_1_unaffordable_payment_touches_nothing() {
    let mut state = g6_payment_state(TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS);
    state.gold = 4;
    state.moral_standing = 30;
    state.toll_progress = TOLL_PROGRESS_MILESTONE;
    state.apply_tlk_gold_payments(&[crate::conversation_session::ConversationGoldPayment {
        amount: 5,
        accepted: true,
    }]);
    assert_eq!(state.gold, 4);
    assert_eq!(state.moral_standing, 30);
    assert_eq!(state.toll_progress, TOLL_PROGRESS_MILESTONE);
}

/// `systems/karma.md §4.1`: "The zero-gold bonus is **nested inside gate 3**
/// ... a payment that empties the purse but fails the class gate or the
/// threshold grants nothing."
#[test]
fn g6_karma_s4_1_zero_gold_bonus_needs_the_class_and_threshold_gates() {
    // Wrong class, aged counter, purse emptied.
    let mut state = g6_payment_state(TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS + 4);
    state.gold = 5;
    state.moral_standing = 30;
    state.toll_progress = TOLL_PROGRESS_MILESTONE;
    state.apply_tlk_gold_payments(&[crate::conversation_session::ConversationGoldPayment {
        amount: 5,
        accepted: true,
    }]);
    assert_eq!(state.gold, 0);
    assert_eq!(state.moral_standing, 30);
    assert_eq!(state.toll_progress, TOLL_PROGRESS_MILESTONE);

    // Right class, counter not yet aged, purse emptied.
    let mut state = g6_payment_state(TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS);
    state.gold = 5;
    state.moral_standing = 30;
    state.toll_progress = TOLL_PROGRESS_MILESTONE - 1;
    state.apply_tlk_gold_payments(&[crate::conversation_session::ConversationGoldPayment {
        amount: 5,
        accepted: true,
    }]);
    assert_eq!(state.gold, 0);
    assert_eq!(state.moral_standing, 30);
}

/// `systems/karma.md §4.1`: "a qualifying payment can raise standing at most
/// once per one hundred turn-consuming actions, and a second qualifying
/// payment on the following turn grants nothing."
#[test]
fn g6_karma_s4_1_milestone_is_a_turn_cooldown() {
    let mut state = g6_payment_state(TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS);
    state.gold = 50;
    state.moral_standing = 30;
    state.toll_progress = TOLL_PROGRESS_MILESTONE;
    let pay = [crate::conversation_session::ConversationGoldPayment {
        amount: 3,
        accepted: true,
    }];
    state.apply_tlk_gold_payments(&pay);
    assert_eq!(state.moral_standing, 31);
    state.advance_turn();
    state.apply_tlk_gold_payments(&pay);
    assert_eq!(state.moral_standing, 31);
    assert_eq!(state.toll_progress, 1);
    for _ in 0..99 {
        state.advance_turn();
    }
    assert_eq!(state.toll_progress, TOLL_PROGRESS_MILESTONE);
    state.apply_tlk_gold_payments(&pay);
    assert_eq!(state.moral_standing, 32);
}

// ---------------------------------------------------------------------------
// systems/containers.md §9: corpse searches
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum G6CorpseExpected {
    Staged { food: bool, quantity: u8 },
    Plague,
    Narration(usize),
}

/// `systems/containers.md §9`, "The exact branch tree and shared
/// gameplay-PRNG consumption": arm split `0..7` (zero stages contents);
/// contents `0..3` (zero food) then quantity `1..3`; cleared arm `0..31`
/// with exactly `19` selecting Plague; otherwise an upper bound `0..3` and a
/// selector `0..upper_bound`.
fn g6_corpse_expected(seed: u16) -> (G6CorpseExpected, u16) {
    let mut prng = seed;
    if u5_prng_range_u16(&mut prng, 0, 7) == 0 {
        let food = u5_prng_range_u16(&mut prng, 0, 3) == 0;
        let quantity = u5_prng_range_u16(&mut prng, 1, 3) as u8;
        return (G6CorpseExpected::Staged { food, quantity }, prng);
    }
    if u5_prng_range_u16(&mut prng, 0, 31) == 19 {
        return (G6CorpseExpected::Plague, prng);
    }
    let upper = u5_prng_range_u16(&mut prng, 0, 3);
    let selector = u5_prng_range_u16(&mut prng, 0, upper) as usize;
    (G6CorpseExpected::Narration(selector), prng)
}

fn g6_corpse_state(seed: u16) -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.active_objects.push(ActiveObject {
        type_byte: 0x1F,
        tile: 0x1F,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.party[0].status = b'G';
    state.prng_state = seed;
    state
}

/// `systems/containers.md §9`: the corpse-search branch tree, its draw
/// counts, the slot fate on each arm and the exact appended line ("Search
/// prints `Thou dost find` followed by a line break ... then appends exactly
/// one of these lines").
#[test]
fn g6_containers_s9_corpse_search_branch_tree_and_transcript() {
    let narration = ["nothing!", "worms!", "guts!", "a bloody pulp!"];
    let mut seen = [false; 4];
    let mut seen_staged = false;
    let mut seen_plague = false;
    for seed in 0..4096u16 {
        let (expected, expected_prng) = g6_corpse_expected(seed);
        let mut state = g6_corpse_state(seed);
        state.resolve_corpse_search(1, 0);
        assert_eq!(state.prng_state, expected_prng, "seed {seed}");
        match expected {
            G6CorpseExpected::Staged { food, quantity } => {
                seen_staged = true;
                // "Only the minority arm ... leaves the slot in place and
                // rewrites it into a later food/gold pickup".
                let slot = state.active_objects[1];
                assert_eq!(slot.type_byte, if food { 0x0F } else { 0x02 });
                assert_eq!(slot.aux1, quantity);
                assert_eq!(
                    state.message,
                    format!(
                        "{SEARCH_PREAMBLE}{}\n",
                        if food { "food!" } else { "gold!" }
                    )
                );
                assert_eq!(state.party[0].status, b'G');
            }
            G6CorpseExpected::Plague => {
                seen_plague = true;
                assert!(state.active_objects[1].is_empty(), "slot cleared first");
                assert_eq!(state.message, format!("{SEARCH_PREAMBLE}Plague!\n"));
                assert_eq!(state.party[0].status, b'P');
            }
            G6CorpseExpected::Narration(selector) => {
                seen[selector] = true;
                assert!(state.active_objects[1].is_empty(), "slot cleared first");
                assert_eq!(
                    state.message,
                    format!("{SEARCH_PREAMBLE}{}\n", narration[selector])
                );
                assert_eq!(state.party[0].status, b'G');
            }
        }
    }
    assert!(seen_staged && seen_plague && seen.iter().all(|s| *s));
}

/// `systems/containers.md §9`, Plague consequence: "the game directly
/// replaces the selected member's status with Poisoned ... There is no
/// current-status gate". The selected slot, not the leader, is the victim.
#[test]
fn g6_containers_s9_plague_overwrites_the_selected_members_status() {
    let seed = (0..=u16::MAX)
        .find(|seed| g6_corpse_expected(*seed).0 == G6CorpseExpected::Plague)
        .unwrap();
    let mut state = g6_corpse_state(seed);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'B',
        status: b'S',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 8,
        hp: DEFAULT_PARTY_HP,
        max_hp: DEFAULT_PARTY_MAX_HP,
        level: 8,
    });
    state.resolve_corpse_search(1, 1);
    assert_eq!(state.party[1].status, b'P');
    assert_eq!(state.party[1].hp, DEFAULT_PARTY_HP, "no HP change");
    assert_eq!(state.party[0].status, b'G');
}

// ---------------------------------------------------------------------------
// systems/karma.md §7: Codex turn-in attribute lines; §7.1 restoration
// ---------------------------------------------------------------------------

/// `systems/karma.md §7`: "Each applicable attribute prints `Strength +1\n`,
/// `Dexterity +1\n`, or `Intelligence +1\n`, in that order; the line still
/// prints when the attribute is already at its cap", and the rewards "write
/// to the Avatar record" and "clamp at thirty".
#[test]
fn g6_karma_s7_turn_in_attribute_lines_print_at_cap() {
    assert_eq!(crate::commands::SHRINE_REWARD_STRENGTH_LINE, "Strength +1");
    assert_eq!(
        crate::commands::SHRINE_REWARD_DEXTERITY_LINE,
        "Dexterity +1"
    );
    assert_eq!(
        crate::commands::SHRINE_REWARD_INTELLIGENCE_LINE,
        "Intelligence +1"
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.avatar_stats = AvatarStats {
        strength: 30,
        dexterity: 30,
        intelligence: 30,
    };
    let notes = state.apply_shrine_stat_reward(ShrineVirtue::Spirituality);
    assert_eq!(
        notes,
        vec!["Strength +1", "Dexterity +1", "Intelligence +1"]
    );
    assert_eq!(
        state.avatar_stats,
        AvatarStats {
            strength: 30,
            dexterity: 30,
            intelligence: 30,
        }
    );

    // Humility: no attribute.
    let notes = state.apply_shrine_stat_reward(ShrineVirtue::Humility);
    assert!(notes.is_empty());

    // Honor: Strength then Intelligence, each +1 below the cap.
    state.avatar_stats = AvatarStats {
        strength: 12,
        dexterity: 12,
        intelligence: 12,
    };
    let notes = state.apply_shrine_stat_reward(ShrineVirtue::Honor);
    assert_eq!(notes, vec!["Strength +1", "Intelligence +1"]);
    assert_eq!(
        state.avatar_stats,
        AvatarStats {
            strength: 13,
            dexterity: 12,
            intelligence: 13,
        }
    );
}

/// `systems/karma.md §7.1`: "any Word of Power beside any ruined shrine opens
/// the prompts, but only the Word mapped to that shrine can restore it." The
/// Compassion word at the Honesty shrine, answered with Compassion's own
/// virtue and mantra, restores nothing.
#[test]
fn g6_karma_s7_1_only_the_shrines_own_word_restores_it() {
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    let mut world = world_state(open_world_grid(), shrine_x + 1, shrine_y);
    world.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    let shrine_index = world_cell_index(shrine_x, shrine_y);
    world.grid[shrine_index] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x85;
    world.shrine_ruin_flags[1] = 0x00;
    world.visibility_dirty = false;

    assert_eq!(world.yell_command(Some("VILIS")), MoveOutcome::Used);
    assert!(world.active_shrine_restoration.is_some());
    assert_eq!(world.step_active_shrine_restoration('C', "ompassion"), None);
    assert_eq!(world.step_active_shrine_restoration('M', "u"), None);
    assert_eq!(world.step_active_shrine_restoration('M', "u"), None);
    assert_eq!(
        world.step_active_shrine_restoration('M', "u"),
        Some(MoveOutcome::Used)
    );
    assert!(world.active_shrine_restoration.is_none());
    assert_eq!(world.grid[shrine_index], WORLD_RUINED_SHRINE_TILE);
    assert_eq!(world.shrine_ruin_flags[0], 0x85);
    assert_eq!(world.shrine_ruin_flags[1], 0x00);
    assert_eq!(world.message, "\n");
}

/// `systems/karma.md §7.1`: "Print `\nUpon what virtue\ndost thou\nmeditate?\n\n:`
/// and read up to fifteen characters", then `\nMantra:` three times.
#[test]
fn g6_karma_s7_1_restoration_prompt_literals() {
    assert_eq!(crate::z_stats::SHRINE_RESTORATION_INPUT_MAX_LEN, 15);
    assert_eq!(
        crate::z_stats::SHRINE_RESTORATION_VIRTUE_PROMPT,
        "\nUpon what virtue\ndost thou\nmeditate?\n\n:"
    );
    assert_eq!(
        crate::z_stats::SHRINE_RESTORATION_MANTRA_PROMPT,
        "\nMantra:"
    );
}

// ---------------------------------------------------------------------------
// systems/karma.md §6, §12: verdict bands and the factory seed
// ---------------------------------------------------------------------------

/// `systems/karma.md §6`: the Blackthorn selector maps "zero through nineteen
/// selects record zero, twenty through thirty-nine selects record one, forty
/// through fifty-nine selects record two, sixty through seventy-nine selects
/// record three, and eighty through ninety-nine selects record four"; the
/// camp event uses the same scale below eighty, "while values eighty and
/// above select record five. That camp event does not select record four."
#[test]
fn g6_karma_s6_verdict_band_edges() {
    for standing in 0..=99u8 {
        let band = usize::from(standing / 20);
        assert_eq!(blackthorn_karma_record_index(standing), band, "{standing}");
        let camp = lord_british_camp_karma_record_index(standing);
        assert_eq!(camp, if standing >= 80 { 5 } else { band }, "{standing}");
        assert_ne!(camp, 4, "{standing}");
    }
    for (standing, record) in [(19u8, 0usize), (20, 1), (39, 1), (40, 2), (59, 2), (60, 3)] {
        assert_eq!(blackthorn_karma_record_index(standing), record);
    }
}

/// `systems/karma.md §6`: "The data file `KARMA.DAT` (761 bytes) holds six
/// NUL-terminated text records".
#[test]
fn g6_karma_s6_shipped_karma_dat_shape() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let bytes = std::fs::read(dir.join(KARMA_DAT_FILE)).unwrap();
    assert_eq!(bytes.len(), 761);
    assert_eq!(bytes.iter().filter(|byte| **byte == 0).count(), 6);
    assert_eq!(parse_karma_dat(&bytes).unwrap().len(), 6);
}

/// `systems/karma.md §12`, "Initial standing seed": "The scalar selector is
/// seeded by the factory save image to the value **seventy-five** ... The
/// step counter in the same band is seeded to zero." `§3`: the selector is
/// "persisted at save offset `0x02E2`".
#[test]
fn g6_karma_s12_factory_seed_standing_and_step_counter() {
    assert_eq!(SAVE_MORAL_STANDING_OFFSET, 0x02E2);
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let seed = std::fs::read(dir.join(INIT_GAM_FILENAME)).unwrap();
    assert_eq!(seed[SAVE_MORAL_STANDING_OFFSET], 75);
    assert_eq!(seed[SAVE_TOLL_PROGRESS_OFFSET], 0);
}

// ---------------------------------------------------------------------------
// systems/intro.md §6.1: lower menu window geometry
// ---------------------------------------------------------------------------

/// `systems/intro.md §6.1`: the frame band covers text rows 15..24 ("pixel
/// rows `120..199` across the full width"), with "a one-pixel
/// palette-index-`15` rectangle with rules at `y = 127` and `y = 192` over
/// `x = 7..312` ... an interior of `x = 8..311`, `y = 128..191` in the text
/// background (black)", the band itself in palette index `1`.
#[test]
fn g6_intro_s6_1_menu_window_geometry() {
    assert_eq!(INTRO_MENU_FRAME_ANCHOR_COLUMN, 0);
    assert_eq!(INTRO_MENU_FRAME_ANCHOR_ROW, 15);
    assert_eq!(INTRO_MENU_FRAME_WIDTH_CELLS, 40);
    assert_eq!(INTRO_MENU_FRAME_HEIGHT_CELLS, 10);
    assert_eq!(
        (INTRO_MENU_FRAME_TOP_Y, INTRO_MENU_FRAME_BOTTOM_Y),
        (120, 199)
    );
    assert_eq!(INTRO_MENU_FRAME_RULE_Y, 127);
    assert_eq!(INTRO_MENU_FRAME_BOTTOM_RULE_Y, 192);
    assert_eq!(
        (INTRO_MENU_FRAME_RULE_X0, INTRO_MENU_FRAME_RULE_X1),
        (7, 312)
    );
    assert_eq!(
        (
            INTRO_MENU_FRAME_INTERIOR_LEFT_X,
            INTRO_MENU_FRAME_INTERIOR_RIGHT_X
        ),
        (8, 311)
    );
    assert_eq!(
        (
            INTRO_MENU_FRAME_INTERIOR_TOP_Y,
            INTRO_MENU_FRAME_INTERIOR_BOTTOM_Y
        ),
        (128, 191)
    );
    assert_eq!(INTRO_MENU_FRAME_BORDER_COLOR, 1);
    assert_eq!(INTRO_MENU_FRAME_OUTLINE_COLOR, 15);
    assert_eq!(INTRO_MENU_FRAME_INTERIOR_COLOR, 0);
}

/// `systems/intro.md §6.1`: "The top-left corner's ink begins at column `5`
/// on its first row and then at columns `3`, `2`, `1`, `1`, `0`, `0`, `0` on
/// the remaining seven; the other three corners are its horizontal, vertical
/// and diagonal mirrors."
#[test]
fn g6_intro_s6_1_rounded_corner_profile() {
    let top: Vec<u16> = (120..128)
        .map(|row| intro_menu_frame_border_start_column(row).unwrap())
        .collect();
    assert_eq!(top, vec![5, 3, 2, 1, 1, 0, 0, 0]);
    let bottom: Vec<u16> = (192..200)
        .rev()
        .map(|row| intro_menu_frame_border_start_column(row).unwrap())
        .collect();
    assert_eq!(bottom, vec![5, 3, 2, 1, 1, 0, 0, 0]);
    assert_eq!(intro_menu_frame_border_start_column(119), None);
    assert_eq!(intro_menu_frame_border_start_column(160), Some(0));
}

/// `systems/intro.md §6.1`, captions: the copyright caption is placed by the
/// centred-caption helper, "For a twenty-seven character string the helper's
/// rule `start_col = 18 - floor(len / 2)` gives column `5`", on row 24; the
/// select prompt sits at "column `15` of row 15".
#[test]
fn g6_intro_s6_1_caption_cells() {
    let text = "Copyright 1988 Lord British";
    assert_eq!(text.len(), 27);
    assert!(INTRO_MENU_COPYRIGHT_CAPTION.contains(text));
    assert_eq!(
        usize::from(INTRO_MENU_COPYRIGHT_CAPTION_COLUMN),
        18 - text.len() / 2
    );
    assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION_ROW, 24);
    assert_eq!(INTRO_MENU_SELECT_CAPTION_COLUMN, 15);
    assert_eq!(INTRO_MENU_SELECT_CAPTION_ROW, 15);
}

/// `systems/containers.md §3`, "A staged rotting body or moldy corpse is not
/// Get-acceptable": the scan "skips such an object exactly as it skips an
/// actor, and keeps scanning the other objects at the coordinate ... If
/// another acceptable object shares the cell, that object is granted with its
/// ordinary line ... the body object stays on the cell".
#[test]
fn g6_containers_s3_get_skips_a_body_and_grants_the_item_beneath() {
    for body_class in [30u8, 31] {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        let body = ActiveObject {
            type_byte: body_class,
            tile: body_class,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects.push(body);
        state.active_objects.push(ActiveObject {
            type_byte: 0x02,
            tile: 0x02,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 5,
            aux3: 0,
        });
        state.gold = 10;
        assert!(!get_object_scan_accepts(body_class));

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(state.gold, 15, "class {body_class}");
        assert_eq!(state.active_objects[1], body, "the body stays on the cell");
        assert!(
            state.active_objects[2].is_empty(),
            "the gold slot is consumed"
        );
        let _ = fs::remove_dir_all(dir);
    }
}
