// Closed-audit follow-up, group g8: rules the engine implements but no test
// asserted, from `systems/overworld.md`, `systems/conversation.md`,
// `systems/display-driver-abi.md`, `catalogs/gazetteer.md`,
// `systems/rest-and-camp.md`, `systems/lighting.md`, `systems/prng.md` and
// `formats/miscmsg-dat.md`. Each test names its file and section. Text used
// as fixture content (names, record bodies) is invented placeholder text;
// every asserted literal is one the cited spec section publishes.

fn ag8_name(text: &str) -> [u8; SAVE_CHARACTER_NAME_LEN] {
    let mut name = [0u8; SAVE_CHARACTER_NAME_LEN];
    for (slot, byte) in name.iter_mut().zip(text.bytes()) {
        *slot = byte;
    }
    name
}

fn ag8_member(slot: u8, class_byte: u8, status: u8, dex: u8, hp: u16, max_hp: u16, mana: u8) -> PartyMember {
    PartyMember {
        slot,
        class_byte,
        status,
        climb_stat: dex,
        mana,
        hp,
        max_hp,
        level: 1,
    }
}

// ---------------------------------------------------------------------------
// systems/prng.md
// ---------------------------------------------------------------------------

#[test]
fn ag8_prng_s2_range_is_low_plus_masked_advanced_state_mod_width_for_every_state() {
    // `prng.md §2`: "`low + ((state & 0x7FFF) modulo (high - low + 1))`" on
    // the advanced state, "always in the requested inclusive range", with
    // the "normal small modulo bias" preserved.
    let ranges: [(u16, u16); 6] = [(1, 30), (0, 2), (5, 5), (0, 255), (100, 2000), (0, 63)];
    for start in 0..=u16::MAX {
        let advanced = u5_prng_advance_state(start);
        for (low, high) in ranges {
            let mut prng = U5Prng::new(start);
            let value = prng.next_range_u16(low, high);
            let width = high - low + 1;
            assert_eq!(value, low + (advanced & 0x7FFF) % width, "state {start:#06x} range {low}..={high}");
            assert!((low..=high).contains(&value));
            // §2: exactly one state advance per call.
            assert_eq!(prng.state(), advanced);
        }
    }
}

#[test]
fn ag8_prng_s3_seed_primitive_is_a_plain_assignment() {
    // `prng.md §3`: the one primitive "takes a 16-bit value and installs it
    // as the new state with no mixing, validation, or return value"; the
    // shipped initial value is zero.
    assert_eq!(DEFAULT_PRNG_STATE, 0);
    for value in [0u16, 1, 0x0FFF, 0x1234, 0xFFFF] {
        let mut prng = U5Prng::new(0xBEEF);
        prng.set_state(value);
        assert_eq!(prng.state(), value);
        assert_eq!(prng.advance(), u5_prng_advance_state(value));
    }
}

#[test]
fn ag8_prng_s3_host_clock_seed_is_twelve_bits_for_every_time_of_day() {
    // `prng.md §3`: "the result is in `0..4095`" - only twelve bits are ever
    // seeded, so a fresh game is one of 4,096.
    for hour in 0..24u8 {
        for minute in 0..60u8 {
            for second in 0..60u8 {
                for hundredth in 0..100u8 {
                    let seed = host_clock_prng_seed(hour, minute, second, hundredth);
                    assert!(seed <= 0x0FFF);
                }
            }
        }
    }
}

#[test]
fn ag8_prng_s3_shifted_seconds_lose_their_high_bits_before_packing() {
    // `prng.md §3`: "The byte-width shifts happen before the fields are
    // packed; in particular, shifting seconds can discard high bits." A
    // seconds field of 32 shifts to 256, which is byte 0.
    for (hour, minute, hundredth) in [(0u8, 0u8, 0u8), (12, 34, 78), (23, 59, 99)] {
        for second in 0..28u8 {
            assert_eq!(
                host_clock_prng_seed(hour, minute, second + 32, hundredth),
                host_clock_prng_seed(hour, minute, second, hundredth),
            );
        }
    }
}

#[test]
fn ag8_prng_s3_packed_word_addition_carries_from_the_low_byte() {
    // `prng.md §3`: "The packed-word addition is 16-bit and wraps, including
    // an ordinary carry from the low byte into the high byte." 00:59:00.99
    // packs `0x0063 + 0x00EC = 0x014F`; XOR `0x91EB` gives `0x90A4`, masked
    // `0x0A4`. A byte-wise add without the carry would give `0x1A4`.
    assert_eq!(host_clock_prng_seed(0, 59, 0, 99), 0x00A4);
}

#[test]
fn ag8_prng_s4_damage_rumble_does_not_touch_the_gameplay_stream() {
    // `prng.md §4`: "The random-rumble audio jitter source is a separate
    // stream and must not be conflated with the game PRNG." The shared
    // party-damage presentation plays the damage rumble; the gameplay state
    // word must not move.
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].hp = 30;
    state.party[0].max_hp = 30;
    state.prng_state = 0x1357;
    let _ = state.apply_shared_party_damage(0, 3);
    assert_eq!(state.prng_state, 0x1357);
    assert_eq!(state.party[0].hp, 27);
}

#[test]
fn ag8_prng_s4_major_flash_takes_one_gameplay_draw_per_band() {
    // `prng.md §4`: "The major-event full-viewport flash/rumble consumes one
    // gameplay-PRNG draw for each of its 1,856 drawing bands, including when
    // sound is muted." The runtime takes the draws itself; muting is a
    // frontend concern that never reaches this path.
    let mut state = test_state(open_grid(), 1, 1);
    state.prng_state = 0x0ACE;
    let mut expected = 0x0ACEu16;
    for _ in 0..1856 {
        expected = u5_prng_advance_state(expected);
    }
    state.emit_major_flash();
    assert_eq!(state.prng_state, expected);
}

// ---------------------------------------------------------------------------
// systems/lighting.md
// ---------------------------------------------------------------------------

#[test]
fn ag8_lighting_s4_counters_are_read_as_booleans() {
    // `lighting.md §4`: "the remaining duration never influences the value.
    // A torch with one minute left lights exactly as far as a freshly lit
    // one."
    for remaining in 1..=u8::MAX {
        assert_eq!(apply_personal_light(FULL_DARKNESS, remaining, 0), TORCH_LIGHT_FLOOR);
        assert_eq!(apply_personal_light(FULL_DARKNESS, 0, remaining), LIGHT_SPELL_FLOOR);
        assert_eq!(apply_personal_light(FULL_DARKNESS, remaining, remaining), LIGHT_SPELL_FLOOR);
    }
}

#[test]
fn ag8_lighting_s4_personal_light_is_a_maximum_that_never_lowers_ambient() {
    // `lighting.md §4`: `effective = max(ambient, 18 if light spell active,
    // 10 if torch active)`; "Neither source ever lowers the ambient value."
    for ambient in 0..=FULL_DAYLIGHT {
        for (torch, spell) in [(0u8, 0u8), (1, 0), (0, 1), (1, 1)] {
            let mut expected = ambient;
            if spell != 0 {
                expected = expected.max(LIGHT_SPELL_FLOOR);
            }
            if torch != 0 {
                expected = expected.max(TORCH_LIGHT_FLOOR);
            }
            let effective = apply_personal_light(ambient, torch, spell);
            assert_eq!(effective, expected, "ambient {ambient} torch {torch} spell {spell}");
            assert!(effective >= ambient);
        }
    }
}

#[test]
fn ag8_lighting_s5_decay_is_saturating_subtraction() {
    // `lighting.md §5`: "if the remaining counter is greater than the spent
    // increment, subtract the increment; otherwise set the counter to zero.
    // Counters never underflow or wrap."
    for counter in 0..=u8::MAX {
        for increment in [0u8, 1, 2, 9, 10, 60, 255] {
            let expected = counter.saturating_sub(increment);
            assert_eq!(decay_light_counter(counter, increment), expected);
        }
    }
}

#[test]
fn ag8_lighting_s7_light_spell_floor_lights_61_cells_reaching_four() {
    // `lighting.md §7` table: a light spell burning in the dark has the
    // effective value 18, which puts 61 of the 121 viewport cells inside the
    // gate and reaches four cells along a row or column.
    let state = britannia_state(open_world_grid(), 100, 100);
    let carve = state.surface_visibility_carve_with_light_threshold(100, 100, 5, 18, true);
    assert_eq!(carve.iter().filter(|lit| **lit).count(), 61);
    assert!(state.world_cell_visible_with_light_threshold(100, 100, 104, 100, 5, 18));
    assert!(!state.world_cell_visible_with_light_threshold(100, 100, 105, 100, 5, 18));
    assert!(state.world_cell_visible_with_light_threshold(100, 100, 100, 96, 5, 18));
    assert!(!state.world_cell_visible_with_light_threshold(100, 100, 100, 95, 5, 18));
}

#[test]
fn ag8_lighting_s7_torch_floor_lights_37_cells_reaching_three() {
    // `lighting.md §7` table: a torch in the dark, no spell, is 10: 37 cells
    // and three along a row or column.
    let state = britannia_state(open_world_grid(), 100, 100);
    let carve = state.surface_visibility_carve_with_light_threshold(100, 100, 5, 10, true);
    assert_eq!(carve.iter().filter(|lit| **lit).count(), 37);
    assert!(state.world_cell_visible_with_light_threshold(100, 100, 103, 100, 5, 10));
    assert!(!state.world_cell_visible_with_light_threshold(100, 100, 104, 100, 5, 10));
    assert!(state.world_cell_visible_with_light_threshold(100, 100, 97, 100, 5, 10));
    assert!(!state.world_cell_visible_with_light_threshold(100, 100, 96, 100, 5, 10));
}

#[test]
fn ag8_lighting_s3_published_ambient_values_are_the_two_endpoints() {
    // `lighting.md §3`: "On the original light scale, full daylight is 50
    // and full darkness is 2", and values "51 or higher are special 'do not
    // recompute' sentinels".
    assert_eq!(FULL_DAYLIGHT, 50);
    assert_eq!(FULL_DARKNESS, 2);
    for value in 0..=FULL_DAYLIGHT {
        assert!(!ambient_is_sentinel(value));
    }
    for value in FULL_DAYLIGHT + 1..=u8::MAX {
        assert!(ambient_is_sentinel(value));
    }
}

#[test]
fn ag8_lighting_s3_forced_dark_z_test_reads_the_high_bit() {
    // `lighting.md §3`: "any Z with its high bit set - that is, any value
    // above one hundred twenty-seven - pins the ambient value at 2 for every
    // hour", and ordinary dungeon levels (which never set the bit) take the
    // clock value.
    for hour in 0..24u8 {
        for z in 128..=u8::MAX {
            assert_eq!(daylight_base_value(hour, 0, false, z), FULL_DARKNESS);
        }
    }
    for z in 0..=7u8 {
        assert_eq!(daylight_base_value(12, 0, false, z), FULL_DAYLIGHT);
    }
}

// ---------------------------------------------------------------------------
// systems/rest-and-camp.md
// ---------------------------------------------------------------------------

#[test]
fn ag8_rest_s7_1_level_line_is_the_published_stream() {
    // `rest-and-camp.md §7.1` step 5: `\n"Hail,_`, the member's name,
    // `!\nFor_thy_valiant_deeds,_I_shall_reward_thee!\n`,
    // `Thou_art_now_level_`, the level as a plain decimal, `,_and\n`, one
    // stat word, then `"_`.
    assert_eq!(
        lord_british_camp_level_line("Placeholder", 4, LordBritishCampStatReward::Strength),
        "\n\"Hail, Placeholder!\nFor thy valiant deeds, I shall reward thee!\nThou art now level 4, and\nstronger!\" "
    );
    assert_eq!(
        lord_british_camp_level_line("Zed", 12, LordBritishCampStatReward::Dexterity),
        "\n\"Hail, Zed!\nFor thy valiant deeds, I shall reward thee!\nThou art now level 12, and\nquicker!\" "
    );
    assert!(
        lord_british_camp_level_line("Zed", 2, LordBritishCampStatReward::Intelligence)
            .ends_with("\nwiser!\" ")
    );
}

#[test]
fn ag8_rest_s7_1_apparition_and_vanish_lines() {
    // `rest-and-camp.md §7.1` steps 1 and 7.
    assert_eq!(LORD_BRITISH_CAMP_APPARITION_LINE, "An apparition!\n");
    assert_eq!(
        LORD_BRITISH_CAMP_VANISH_LINE,
        "\n\nThe strangely familiar old man vanishes...\n"
    );
}

#[test]
fn ag8_rest_s7_1_envelopes_follow_the_published_tuples() {
    // `rest-and-camp.md §7.1`, tuple order `(delta, initial comparison,
    // iterations, idle, period)`.
    use crate::audio::EnvelopeSegment;
    use crate::lord_british_camp::*;
    assert_eq!(
        LORD_BRITISH_CAMP_APPARITION_ENVELOPE,
        EnvelopeSegment::new(6, 2500, 10_000, 1, 2620)
    );
    assert_eq!(LORD_BRITISH_CAMP_FIGURE_PERIODS, [2620, 2620, 2620, 3700, 3900, 4160]);
    for period in LORD_BRITISH_CAMP_FIGURE_PERIODS {
        assert_eq!(
            lord_british_camp_figure_note(period),
            EnvelopeSegment::new(13, 200, 5_000, 1, period)
        );
    }
    assert_eq!(LORD_BRITISH_CAMP_STING_SHORT, EnvelopeSegment::new(13, 200, 5_000, 1, 5500));
    assert_eq!(LORD_BRITISH_CAMP_STING_LONG, EnvelopeSegment::new(1, 2500, 60_000, 1, 5500));
}

#[test]
fn ag8_rest_s7_stat_reward_adds_one_capped_at_thirty() {
    // `rest-and-camp.md §7`: "The selected stat is increased by one with a
    // cap of 30."
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![
        ag8_member(0, b'A', b'G', 30, 30, 30, 0),
        ag8_member(1, b'F', b'G', 29, 30, 30, 0),
    ];
    state.normalize_party_progress_vectors();
    state.avatar_stats.strength = 30;
    state.party_strengths[0] = 30;
    state.party_strengths[1] = 29;

    state.apply_lord_british_camp_stat_reward_kind(0, LordBritishCampStatReward::Strength);
    assert_eq!(state.avatar_stats.strength, 30);
    state.apply_lord_british_camp_stat_reward_kind(1, LordBritishCampStatReward::Strength);
    assert_eq!(state.party_strengths[1], 30);
    state.apply_lord_british_camp_stat_reward_kind(1, LordBritishCampStatReward::Strength);
    assert_eq!(state.party_strengths[1], 30);
    state.apply_lord_british_camp_stat_reward_kind(1, LordBritishCampStatReward::Dexterity);
    assert_eq!(state.party[1].climb_stat, 30);
    state.apply_lord_british_camp_stat_reward_kind(1, LordBritishCampStatReward::Dexterity);
    assert_eq!(state.party[1].climb_stat, 30);
}

#[test]
fn ag8_rest_s5_camp_mp_write_is_an_assignment_that_can_lower_mp() {
    // `rest-and-camp.md §5`: "The MP write is an assignment, not an
    // addition, so a class row on this list can have its magic points
    // reduced by camping if its current MP was already above the target".
    let mut state = test_state(open_grid(), 1, 1);
    state.camp_cooldown = 0;
    state.party = vec![
        ag8_member(0, b'A', b'G', 20, 30, 30, 0),
        ag8_member(1, b'M', b'G', 20, 30, 30, 40),
        ag8_member(2, b'B', b'G', 20, 30, 30, 40),
    ];
    state.normalize_party_progress_vectors();
    state.avatar_stats.intelligence = 12;
    state.party_intelligence = vec![12, 20, 20];

    state.apply_completed_long_camp_recovery(8, None, b"GGG");

    assert_eq!(state.party[0].mana, 12);
    assert_eq!(state.party[1].mana, 20);
    assert_eq!(state.party[2].mana, 10);
}

#[test]
fn ag8_rest_s5_member_poisoned_during_the_camp_still_recovers_and_stays_poisoned() {
    // `rest-and-camp.md §5`: "A member poisoned during the camp is still
    // eligible; a member already poisoned at entry is not", and "Poisoned
    // members keep Poisoned status; rest does not cure poison."
    let mut state = test_state(open_grid(), 1, 1);
    state.camp_cooldown = 0;
    state.party = vec![
        ag8_member(0, b'F', b'P', 20, 1, 80, 0),
        ag8_member(1, b'F', b'P', 20, 1, 80, 0),
    ];
    state.normalize_party_progress_vectors();

    state.apply_completed_long_camp_recovery(8, None, b"GP");

    assert!(state.party[0].hp >= 2 && state.party[0].hp <= 64);
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[1].hp, 1);
    assert_eq!(state.party[1].status, b'P');
}

#[test]
fn ag8_rest_s5_hp_gain_is_one_to_sixty_three_capped_at_maximum() {
    // `rest-and-camp.md §5`: "adds a uniform random `1..63` HP, rolled
    // independently per member, and caps current HP at maximum HP."
    for seed in 0..64u16 {
        let mut state = test_state(open_grid(), 1, 1);
        state.camp_cooldown = 0;
        state.party = vec![
            ag8_member(0, b'F', b'G', 20, 10, 200, 0),
            ag8_member(1, b'F', b'G', 20, 10, 200, 0),
        ];
        state.normalize_party_progress_vectors();
        state.prng_state = seed;
        let mut expected = seed;
        let first = u5_prng_range_u16(&mut expected, 1, 63);
        let second = u5_prng_range_u16(&mut expected, 1, 63);

        state.apply_completed_long_camp_recovery(6, None, b"GG");

        assert_eq!(state.party[0].hp, 10 + first);
        assert_eq!(state.party[1].hp, 10 + second);
        assert_eq!(state.prng_state, expected);
    }
    let mut capped = test_state(open_grid(), 1, 1);
    capped.camp_cooldown = 0;
    capped.party = vec![ag8_member(0, b'F', b'G', 20, 79, 80, 0)];
    capped.normalize_party_progress_vectors();
    capped.apply_completed_long_camp_recovery(9, None, b"G");
    assert_eq!(capped.party[0].hp, 80);
}

#[test]
fn ag8_rest_s6_sleep_ambush_restores_entry_statuses_without_curing_or_reviving() {
    // `rest-and-camp.md §6`: "members who entered the rest pass poisoned
    // remain Poisoned, while other eligible sleepers/watch participants are
    // restored to Good. This restoration is not a poison cure, and it does
    // not turn Dead members into active combatants."
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![
        ag8_member(0, b'A', b'S', 20, 30, 30, 0),
        ag8_member(1, b'F', b'S', 20, 30, 30, 0),
        ag8_member(2, b'F', b'G', 20, 30, 30, 0),
        ag8_member(3, b'F', b'D', 20, 0, 30, 0),
    ];
    let restored = state.restore_sleep_ambush_party_statuses(b"GPGG");
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(state.party[1].status, b'P');
    assert_eq!(state.party[2].status, b'G');
    assert_eq!(state.party[3].status, b'D');
    assert_eq!(state.party[3].hp, 0);
    assert_eq!(restored, 2);
}

#[test]
fn ag8_rest_s5_town_bed_sleeps_only_good_members_and_cleanup_wakes_them() {
    // `rest-and-camp.md §5`: "active party members who are in Good status
    // are temporarily marked Sleeping for the elapsed-rest loop. Cleanup
    // changes any Sleeping member back to Good".
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![
        ag8_member(0, b'A', b'G', 20, 30, 30, 0),
        ag8_member(1, b'F', b'P', 20, 30, 30, 0),
        ag8_member(2, b'F', b'D', 20, 0, 30, 0),
        ag8_member(3, b'F', b'C', 20, 30, 30, 0),
    ];
    assert_eq!(state.mark_town_rest_sleepers(), 1);
    assert_eq!(
        state.party.iter().map(|member| member.status).collect::<Vec<_>>(),
        vec![b'S', b'P', b'D', b'C']
    );
    assert_eq!(state.wake_town_rest_sleepers(), 1);
    assert_eq!(
        state.party.iter().map(|member| member.status).collect::<Vec<_>>(),
        vec![b'G', b'P', b'D', b'C']
    );
}

#[test]
fn ag8_rest_s4_the_four_sleep_lines_are_distinct_literals() {
    // `rest-and-camp.md §4`: dangerous camp `Zzzzzz...\n\n` (six z), town
    // bed `Zzzzzzz...\n` (seven z), paid inn `Zzzzzz....\n\n` (four dots),
    // sleeping combatant `Zzzzz...\n` (five z). "Do not apply one generic
    // sleep sentence to all four surfaces."
    assert_eq!(CAMP_BODY_SLEEP_LINE, "Zzzzzz...\n\n");
    assert_eq!(crate::commands::TOWN_BED_REST_SLEEP_LINE, "Zzzzzzz...");
    assert_eq!(crate::commands::PAID_INN_REST_SLEEP_LINE, "Zzzzzz....");
    assert_eq!(crate::combat_actor::COMBAT_ASLEEP_TURN_LINE, "Zzzzz...");
    // §4: the town bed prompt is `For how many hours? ` with no range text.
    assert_eq!(crate::commands::TOWN_BED_HOURS_PROMPT, "For how many hours? ");
}

#[test]
fn ag8_rest_s5_cooldown_is_persisted_at_its_published_offset() {
    // `rest-and-camp.md §5`: "It occupies the byte at `SAVED.GAM` offset
    // `0x02E6`".
    assert_eq!(SAVE_CAMP_COOLDOWN_OFFSET, 0x02E6);
    assert_eq!(crate::rest_camp::COMPLETED_LONG_CAMP_COOLDOWN_HOURS, 14);
}

#[test]
fn ag8_rest_s7_apparition_gate_accepts_exactly_zero_to_twenty_four() {
    // `rest-and-camp.md §5`/`§7`: "draws from the closed range `0..99` and
    // accepts `0..24`, giving twenty-five accepted outcomes in one hundred".
    let accepted = (0..=99u8).filter(|roll| lord_british_camp_event_triggered(*roll)).count();
    assert_eq!(accepted, 25);
    assert!((0..=24u8).all(lord_british_camp_event_triggered));
}

// ---------------------------------------------------------------------------
// systems/conversation.md
// ---------------------------------------------------------------------------

fn ag8_enc(text: &str) -> Vec<u8> {
    text.bytes().map(|byte| byte ^ TLK_TEXT_XOR_MASK).collect()
}

fn ag8_conversation() -> crate::conversation_session::ConversationSession {
    let records = [
        "Placeholder",
        "a placeholder figure",
        "Placeholder greeting.",
        "Placeholder job.",
        "Placeholder farewell.",
        "TOPIC",
        "Placeholder topic answer.",
    ];
    crate::conversation_session::ConversationSession::new(
        records.iter().map(|text| ag8_enc(text)).collect(),
        records.iter().map(|text| text.to_string()).collect(),
    )
}

fn ag8_ctx() -> crate::conversation_session::ConversationContext<'static> {
    crate::conversation_session::ConversationContext {
        avatar_name: "Placeholder",
        branch_flags: 0,
        moral_standing: 0,
        dictionary: None,
        gold_available: None,
        party_member_names: &[],
    }
}

#[test]
fn ag8_conversation_s3_class_is_scene_minus_one_shifted_right_three() {
    // `conversation.md §3`: "it is `(scene_id − 1) >> 3`, mapping scenes
    // `1..8` to towns, `9..16` to dwellings, `17..24` to castles, and
    // `25..32` to keeps."
    for scene in 0..=u8::MAX {
        let expected = match scene {
            1..=8 => Some(TlkFileClass::Towne),
            9..=16 => Some(TlkFileClass::Dwelling),
            17..=24 => Some(TlkFileClass::Castle),
            25..=32 => Some(TlkFileClass::Keep),
            _ => None,
        };
        assert_eq!(tlk_class_for_scene(scene), expected, "scene {scene}");
    }
}

#[test]
fn ag8_conversation_s3_header_and_fixed_read_windows() {
    // `conversation.md §3`: 48/15/40/32 entries; four-byte entries; the
    // loader reads "the first 512 bytes" and then "a second fixed-window read
    // of 1024 bytes"; text bytes carry the high bit (§4).
    assert_eq!(TlkFileClass::Towne.shipped_npc_count(), 48);
    assert_eq!(TlkFileClass::Dwelling.shipped_npc_count(), 15);
    assert_eq!(TlkFileClass::Castle.shipped_npc_count(), 40);
    assert_eq!(TlkFileClass::Keep.shipped_npc_count(), 32);
    assert_eq!(TLK_HEADER_ENTRY_LEN, 4);
    assert_eq!(TLK_HEADER_FIXED_READ, 512);
    assert_eq!(TLK_BLOB_FIXED_WINDOW, 1024);
    assert_eq!(TLK_TEXT_XOR_MASK, 0x80);
    // Every class header (`4 × count + 2`) fits inside the 512-byte read.
    for class in [TlkFileClass::Towne, TlkFileClass::Dwelling, TlkFileClass::Castle, TlkFileClass::Keep] {
        assert!(4 * class.shipped_npc_count() + 2 <= TLK_HEADER_FIXED_READ);
    }
}

#[test]
fn ag8_conversation_s2_status_tile_gate_tests_exactly_two_ids() {
    // `conversation.md §2` step 4: the gate compares "against exactly two
    // values" - `0x9D` prints `No response!`, `0xAB` prints `"Zzzzzz..."` -
    // and "neighbouring ids such as the mirror-with-reflection and
    // broken-mirror tiles do not".
    for tile in 0..=u8::MAX {
        let expected = match tile {
            0x9D => Some("No response!"),
            0xAB => Some("\"Zzzzzz...\""),
            _ => None,
        };
        assert_eq!(talk_status_tile_refusal(tile), expected, "tile {tile:#04x}");
    }
}

#[test]
fn ag8_conversation_s2_talk_through_white_list_is_exact() {
    // `conversation.md §2` step 3: "the tile ids `0x29`, `0x94..0x9C`,
    // `0xA5`, `0xAE`, `0xBA..0xBB`, `0xBE` and `0xCA..0xCB`; every other tile
    // id is opaque to Talk", and the mirror `0x9D` is outside it.
    for tile in 0..=u8::MAX {
        let expected = matches!(tile, 0x29 | 0x94..=0x9C | 0xA5 | 0xAE | 0xBA..=0xBB | 0xBE | 0xCA..=0xCB);
        assert_eq!(is_talk_through_tile(tile), expected, "tile {tile:#04x}");
    }
    assert!(!is_talk_through_tile(0x9D));
}

#[test]
fn ag8_conversation_s2_guard_escape_is_waypoint_parity_with_a_nonzero_dialogue() {
    // `conversation.md §2` step 5 (R479): "an even index always gives the
    // guard refusal and an odd index with a non-zero dialogue index always
    // dispatches"; an odd index with a zero word also gets the guard line.
    for waypoint in 0..12usize {
        for dialog in [0u8, 1, 0x7F, 0xFF] {
            assert_eq!(
                talk_guard_sprite_dispatches(waypoint, dialog),
                waypoint % 2 == 1 && dialog != 0,
                "waypoint {waypoint} dialog {dialog:#04x}"
            );
        }
    }
}

#[test]
fn ag8_conversation_s2_typed_talk_lines_and_the_guard_refusal_rows() {
    // `conversation.md §2`: the typed command's own lines "each carries its
    // leading line feed inside the stored string", and the guard answers
    // `The guard offers` on one row and `no response!` on the next - "one
    // stored literal", which the sixteen-column window breaks at its space.
    assert_eq!(TALK_NOBODY_HERE_LINE, "\nNobody's here!");
    assert_eq!(crate::tlk_control_codes::TALK_GUARD_NO_RESPONSE_LINE, "The guard offers no response!");
    assert_eq!(
        wrap_text(crate::tlk_control_codes::TALK_GUARD_NO_RESPONSE_LINE, MESSAGE_WINDOW_WIDTH, 0),
        vec!["The guard offers".to_string(), "no response!".to_string()]
    );
    assert_eq!(talk_non_speaker_refusal_for_sprite(Some(crate::tlk_control_codes::TALK_GUARD_SPRITE)), "\nThe guard offers no response!");
    assert_eq!(talk_non_speaker_refusal_for_sprite(None), "\nNo response!");
}

#[test]
fn ag8_conversation_s5_reserved_table_has_thirty_four_words_in_published_order() {
    // `conversation.md §5`: the fixed reserved table, indices 0..33.
    let functional: Vec<&[u8]> = vec![b"NAME", b"JOB", b"WORK", b"BYE", b"THANK"];
    let rebuke: Vec<&[u8]> = vec![
        b"FUCK", b"SHIT", b"DAMN", b"DICK", b"PRICK", b"PUSSY", b"CUNT", b"ASS", b"BUTT",
        b"BOOGER", b"PISS", b"JACK OFF", b"MASTURBATE", b"SUCK", b"FART", b"TITS", b"BOOB",
        b"MELONS", b"BLOW", b"PENIS", b"BREAST", b"CLIT", b"BALLS", b"SCROTUM", b"NUTS",
        b"BULLSHIT", b"CUM", b"CROTCH", b"MOTHERFUCKER",
    ];
    assert_eq!(RESERVED_KEYWORD_FUNCTIONAL_WORDS.to_vec(), functional);
    assert_eq!(RESERVED_KEYWORD_REBUKE_WORDS.to_vec(), rebuke);
    assert_eq!(RESERVED_KEYWORD_TABLE_ENTRIES, functional.len() + rebuke.len());
    assert_eq!(RESERVED_KEYWORD_TABLE_ENTRIES, 34);
}

#[test]
fn ag8_conversation_s6_reserved_words_match_at_a_word_start_with_no_end_check() {
    // `conversation.md §6` step 4 (R578): "each reserved word is accepted by
    // the same rule as step 5 - its first occurrence in the line must start
    // the line or follow a literal space, and nothing after it is checked.
    // So `JOBS` runs the Job entry and `WELL BYE` takes the BYE path".
    assert_eq!(
        tlk_player_input_kind(b"JOBS"),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry)
    );
    assert_eq!(
        tlk_player_input_kind(b"WELL BYE"),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath)
    );
    assert_eq!(
        tlk_player_input_kind(b"THANKS"),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath)
    );
    assert_eq!(
        tlk_player_input_kind(b"WORKER"),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry)
    );
    assert_eq!(
        tlk_player_input_kind(b"NAMES"),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry)
    );
    assert_eq!(tlk_player_input_kind(b"XJOB"), TlkPlayerInputKind::OrdinaryKeywordScan);
    assert_eq!(tlk_player_input_kind(b""), TlkPlayerInputKind::EmptyByeShortcut);
}

#[test]
fn ag8_conversation_s6_ordinary_match_uses_the_first_occurrence_and_a_start_boundary() {
    // `conversation.md §6` step 5 (R577): "`JOINED` and `I WANT TO JOIN`
    // match `join`; `REJOIN` and `JO IN` do not", and the scoped table of
    // §7.7 ("`MAYBE YES` ... the first `Y` is mid-word, so the later clean
    // one is never tried").
    assert!(tlk_keyword_matches(b"join", b"JOINED"));
    assert!(tlk_keyword_matches(b"join", b"I WANT TO JOIN"));
    assert!(!tlk_keyword_matches(b"join", b"REJOIN"));
    assert!(!tlk_keyword_matches(b"join", b"JO IN"));
    for line in [&b"Y"[..], b"YES", b"YONDER", b"GO YES"] {
        assert!(tlk_keyword_matches(b"y", line));
    }
    for line in [&b"OKAY"[..], b"MAYBE", b"BYE", b"MAYBE YES"] {
        assert!(!tlk_keyword_matches(b"y", line));
    }
    for line in [&b"NAY"[..], b"NO", b"NAME"] {
        assert!(tlk_keyword_matches(b"n", line));
    }
    // Bit 7 is stripped on both sides, so an encoded keyword matches.
    assert!(tlk_keyword_matches(&ag8_enc("join"), b"JOIN"));
}

#[test]
fn ag8_conversation_s7_6_ask_who_name_rule_is_capped_at_four_characters() {
    // `conversation.md §7.6`: "Four characters is a cap, not a requirement.
    // A roster name shorter than four characters contributes only its own
    // characters ... while a zero-length name never matches at all", and "a
    // count of zero examines no slot at all".
    let long: [&[u8]; 1] = [b"PLACEHOLDER"];
    assert_eq!(tlk_ask_who_match(b"PLAC", &long), 1);
    assert_eq!(tlk_ask_who_match(b"I AM PLACX", &long), 1);
    assert_eq!(tlk_ask_who_match(b"PLA", &long), 0);
    let short: [&[u8]; 1] = [b"ZQ"];
    assert_eq!(tlk_ask_who_match(b"ZQUARE", &short), 1);
    assert_eq!(tlk_ask_who_match(b"HELLO ZQ", &short), 1);
    let one: [&[u8]; 1] = [b"Q"];
    assert_eq!(tlk_ask_who_match(b"QUIET", &one), 1);
    let empty: [&[u8]; 2] = [b"", b"ZQ"];
    assert_eq!(tlk_ask_who_match(b"ZQ", &empty), 2);
    let none: [&[u8]; 0] = [];
    assert_eq!(tlk_ask_who_match(b"ANYTHING", &none), 0);
    // "a line containing only a space is *not* empty, runs the scan
    // normally, and ends on the no-match arm."
    assert_eq!(tlk_ask_who_match(b" ", &long), 0);
    // Case folds on both sides.
    assert_eq!(tlk_ask_who_match(b"placeholder", &long), 1);
}

#[test]
fn ag8_conversation_s7_6_gold_payment_digits_are_masked_to_seven_bits() {
    // `conversation.md §7.6` `0x85`: "Collect three argument bytes, mask each
    // to seven bits, interpret them as ASCII decimal digits".
    assert_eq!(tlk_gold_payment_amount(b'0', b'0', b'5'), Some(5));
    assert_eq!(tlk_gold_payment_amount(b'0' | 0x80, b'2' | 0x80, b'5' | 0x80), Some(25));
    assert_eq!(tlk_gold_payment_amount(b'9', b'9', b'9'), Some(999));
    assert_eq!(tlk_gold_payment_amount(b'A', b'0', b'0'), None);
    assert_eq!(TLK_GOLD_PAYMENT_ARGUMENT_BYTES, 3);
}

#[test]
fn ag8_conversation_s7_6_action_letters_a_to_k_and_signal_band() {
    // `conversation.md §7.6`: letters `A..K` dispatch through the global
    // table; "Small numeric argument bytes below the letter range (values
    // `< 0x40`) take a separate path".
    use TlkActionDispatchVerb::*;
    let expected = [
        (b'A', RaiseFood),
        (b'B', RaiseGold),
        (b'C', RaiseKeys),
        (b'D', RaiseGems),
        (b'E', RaiseTorches),
        (b'F', SetGrappleGate),
        (b'G', RaiseCarpets),
        (b'H', SetSextantCarried),
        (b'I', SetSpyglassCarried),
        (b'J', SetBlackBadgeCarried),
        (b'K', RaiseSkullKeys),
    ];
    for (letter, verb) in expected {
        assert_eq!(tlk_action_dispatch_verb(letter), Some(verb));
        assert!(!tlk_action_dispatch_is_signal_flag(letter));
    }
    assert_eq!(tlk_action_dispatch_verb(b'L'), None);
    for small in 0..0x40u8 {
        assert!(tlk_action_dispatch_is_signal_flag(small));
        assert_eq!(tlk_action_dispatch_verb(small), None);
    }
    assert_eq!(TLK_GENERIC_SIGNAL_CAP, 99);
}

#[test]
fn ag8_conversation_s7_6_counter_letters_add_one_and_item_letters_write_ff() {
    // `conversation.md §7.6`: "Every counter-style letter (`A..G`, `K`) adds
    // exactly **one** ... using the shared capped-add helper", food and gold
    // capped at 9999, the byte counters at 99; "`H`, `I`, `J` instead write
    // the sentinel `0xFF` directly".
    use TlkActionDispatchVerb::*;
    let mut state = test_state(open_grid(), 1, 1);
    state.food = 9998;
    state.gold = 9998;
    state.keys = 98;
    state.gems = 98;
    state.torches = 98;
    state.climbing_gear = 98;
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 98;
    state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = 98;
    let counters = [RaiseFood, RaiseGold, RaiseKeys, RaiseGems, RaiseTorches, SetGrappleGate, RaiseCarpets, RaiseSkullKeys];
    state.apply_tlk_action_grants(&counters);
    assert_eq!((state.food, state.gold), (9999, 9999));
    assert_eq!((state.keys, state.gems, state.torches, state.climbing_gear), (99, 99, 99, 99));
    assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 99);
    assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 99);
    state.apply_tlk_action_grants(&counters);
    assert_eq!((state.food, state.gold), (9999, 9999));
    assert_eq!((state.keys, state.gems, state.torches, state.climbing_gear), (99, 99, 99, 99));
    assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 99);
    assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 99);

    state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 0;
    state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 0;
    state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = 0;
    state.apply_tlk_action_grants(&[SetSextantCarried, SetSpyglassCarried, SetBlackBadgeCarried]);
    assert_eq!(state.special_items[SPECIAL_ITEM_SEXTANT_INDEX], 0xFF);
    assert_eq!(state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX], 0xFF);
    assert_eq!(state.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX], 0xFF);
}

#[test]
fn ag8_conversation_engine_owned_literals_match_the_published_text() {
    // `conversation.md §6` (prompt, no-match line, empty-line `BYE`, NAME
    // prefix, rebuke and its 28-pass bound), §7.6 (ASK-WHO question,
    // acknowledgements, gold refusal, full-party refusal), §7.7 (scoped
    // prompt and its empty-line reply) and §9 (`You see ` lead-in).
    assert_eq!(TLK_KEYWORD_PROMPT, "Your interest?\n:");
    assert_eq!(TLK_NO_KEYWORD_MATCH_MESSAGE, "I cannot help thee with that.\n\n");
    assert_eq!(TLK_EMPTY_INPUT_BYE_MESSAGE, "BYE\n\n");
    assert_eq!(crate::tlk_control_codes::TLK_NAME_KEYWORD_PREFIX, "My name is ");
    assert!(TLK_RESERVED_REBUKE_MESSAGE.starts_with("With language like that, how did you become an Avatar?"));
    assert_eq!(TLK_RESERVED_REBUKE_PAUSE_LIMIT, 28);
    assert_eq!(TLK_ASK_WHO_PROMPT, "\"What is thy name?\"\n\nYou respond-\n:");
    assert_eq!(TLK_ASK_WHO_AFFIRMATIVE_LINE, "A pleasure!");
    assert_eq!(TLK_ASK_WHO_DISMISSIVE_LINE, "If you say so...");
    assert_eq!(TLK_GOLD_PAYMENT_REFUSAL_MESSAGE, "\"Thou hast not enough gold!\"\n\n");
    assert_eq!(
        TLK_RECRUIT_SPEAKER_FULL_PARTY_REFUSAL,
        "\"Thou hast no room for me in thy party! Seek me again if one of thy members doth leave\nthee."
    );
    assert_eq!(crate::tlk_control_codes::TLK_SCOPED_PROMPT, "You respond-\n:");
    assert_eq!(crate::tlk_control_codes::TLK_SCOPED_EMPTY_INPUT_LINE, "\"What didst thou say?");
    assert_eq!(TLK_OPENING_DESCRIPTION_PREFIX, "You see ");
    assert_eq!(TLK_INPUT_MAX_LEN, 15);
}

#[test]
fn ag8_conversation_s6_name_runs_the_name_entry_after_the_fixed_prefix() {
    // `conversation.md §6`: "its fixed words are exactly `My name is `,
    // including one trailing space. The fixed output also supplies the
    // opening double quote immediately before `My`".
    let mut session = ag8_conversation();
    session.present_greeting(&ag8_ctx());
    let out = session.submit_keyword("NAME", &ag8_ctx());
    assert!(out.text.contains("\"My name is Placeholder"), "{:?}", out.text);
    assert!(!out.ended);
}

#[test]
fn ag8_conversation_s6_typed_thank_runs_the_bye_entry_without_the_bye_line() {
    // `conversation.md §6` step 4: "`BYE` and `THANK` run the fixed Bye path
    // ... A typed `BYE` or `THANK` runs the Bye entry **without** printing
    // the `BYE` line".
    let mut session = ag8_conversation();
    session.present_greeting(&ag8_ctx());
    let out = session.submit_keyword("THANK YOU", &ag8_ctx());
    assert!(out.text.contains("Placeholder farewell."), "{:?}", out.text);
    assert!(!out.text.contains("BYE"));
    assert!(out.ended);
}

#[test]
fn ag8_conversation_s6_reserved_job_wins_over_an_ordinary_keyword_scan() {
    // `conversation.md §6`: the reserved scan runs first; `JOBS` runs the
    // Job entry and an unmatched line prints the no-match line.
    let mut session = ag8_conversation();
    session.present_greeting(&ag8_ctx());
    let job = session.submit_keyword("JOBS", &ag8_ctx());
    assert!(job.text.contains("Placeholder job."), "{:?}", job.text);
    let miss = session.submit_keyword("NOTHING", &ag8_ctx());
    assert!(miss.text.contains("I cannot help thee with that."), "{:?}", miss.text);
    let topic = session.submit_keyword("A TOPICAL QUESTION", &ag8_ctx());
    assert!(topic.text.contains("Placeholder topic answer."), "{:?}", topic.text);
}

#[test]
fn ag8_conversation_s10_falsehood_theft_redraws_until_it_picks_a_held_counter() {
    // `conversation.md §10` item 1: "the cleanup takes one draw from the
    // inclusive range `0..2`: `0` picks keys, `1` gems, `2` torches. If the
    // party has none of the picked item it draws again from `0..2`, one draw
    // per attempt, until it picks a counter the party holds".
    let mut chosen = None;
    for seed in 0..4096u16 {
        let mut probe = seed;
        let mut draws = 1;
        while u5_prng_range_u16(&mut probe, 0, 2) != 2 {
            draws += 1;
        }
        if draws >= 3 {
            chosen = Some((seed, probe));
            break;
        }
    }
    let (seed, expected_state) = chosen.expect("some seed needs several draws to reach torches");
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(4).unwrap(),
        floor: 0,
    };
    state.resident_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    state.keys = 0;
    state.gems = 0;
    state.torches = 3;
    state.gold = 50;

    assert!(state.run_final_conversation_cleanup_with_seed(seed));
    assert_eq!(state.torches, 2);
    assert_eq!(state.gold, 50);
    assert_eq!(state.prng_state, expected_state);
}

// ---------------------------------------------------------------------------
// systems/overworld.md
// ---------------------------------------------------------------------------

#[test]
fn ag8_overworld_s4_scroll_base_steps_back_in_the_upper_or_left_half() {
    // `overworld.md §4`: "Round the party's tile position down to the
    // nearest multiple of sixteen. If the party's position within that chunk
    // is in the upper or left half (its low nibble is less than eight), step
    // the scroll base back another sixteen."
    for position in 0..256usize {
        let rounded = position / 16 * 16;
        let expected = if position % 16 < 8 { (rounded + 256 - 16) % 256 } else { rounded };
        assert_eq!(world_scroll_base_axis(position), expected, "position {position}");
        assert_eq!(world_scroll_base_axis(position) % 16, 0);
    }
    assert_eq!(world_scroll_base(7, 24), (240, 16));
}

#[test]
fn ag8_overworld_s4_tile_reader_splits_the_32_window_into_four_quadrants() {
    // `overworld.md §4`: "projects (world X, world Y) into this buffer by
    // subtracting the scroll base, masking to five bits per axis, splitting
    // on the sixth bit to pick the quadrant, and indexing in row-major within
    // that quadrant." The quadrants are origin, east, south, southeast.
    let base = (240usize, 16usize);
    for dy in 0..32usize {
        for dx in 0..32usize {
            let x = (base.0 + dx) % 256;
            let y = (base.1 + dy) % 256;
            let quadrant = usize::from(dy >= 16) * 2 + usize::from(dx >= 16);
            assert_eq!(
                live_buffer_index(base, x, y),
                quadrant * 256 + (dy % 16) * 16 + (dx % 16),
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn ag8_overworld_s3_substitutions_are_conditional_and_target_published_tiles() {
    // `overworld.md §3`: dungeon entrance tiles `0x16..0x18` become `0xDF`
    // only while the word is unspoken; shrine `0x19` becomes `0x1A` only
    // while ruined. "Both substitutions are conditional; neither is
    // unconditional."
    let none = LiveChunkSubstitutionPolicy::NONE;
    let seal = LiveChunkSubstitutionPolicy {
        seal_entrances: true,
        ruin_shrine: false,
    };
    let ruin = LiveChunkSubstitutionPolicy {
        seal_entrances: false,
        ruin_shrine: true,
    };
    for tile in 0..=u8::MAX {
        assert_eq!(live_chunk_substituted_tile(tile, none), tile);
        let sealed = if (0x16..=0x18).contains(&tile) { 0xDF } else { tile };
        assert_eq!(live_chunk_substituted_tile(tile, seal), sealed);
        let ruined = if tile == 0x19 { 0x1A } else { tile };
        assert_eq!(live_chunk_substituted_tile(tile, ruin), ruined);
    }
}

#[test]
fn ag8_overworld_s7_encounter_fires_when_threshold_exceeds_the_draw() {
    // `overworld.md §7`: "The mode loop draws a uniform integer in `[1, 30]`.
    // If the threshold exceeds the draw, the spawner fires."
    for threshold in 0..=31u8 {
        for roll in 1..=30u8 {
            assert_eq!(random_encounter_probe_fires(threshold, roll), threshold > roll);
        }
    }
}

#[test]
fn ag8_overworld_s6_2_1_broadside_announcement_literal() {
    // `overworld.md §6.2.1`: "`* BOOOM! *` followed by a blank line, before
    // the shot".
    assert_eq!(OUTDOOR_BROADSIDE_BOOM_MESSAGE, "* BOOOM! *\n\n");
}

#[test]
fn ag8_overworld_s6_2_2_twelve_breath_geometries_never_sample_the_party_cell() {
    // `overworld.md §6.2.2`: "In twelve of the forty-four breath geometries
    // the sampled path never reaches the party's cell at all, and a
    // broadside fired three cells away never samples the party's cell".
    let party = OUTDOOR_RANGED_ATTACK_PARTY_CELL;
    let mut geometries = 0;
    let mut never_reach = 0;
    for dy in -3i32..=3 {
        for dx in -3i32..=3 {
            if (dx, dy) == (0, 0) || dx.abs() + dy.abs() == 1 {
                continue;
            }
            geometries += 1;
            let attacker = (party.0 + dx, party.1 + dy);
            let samples = outdoor_ranged_attack_samples(attacker, party);
            if samples.iter().all(|sample| (sample.column, sample.row) != party) {
                never_reach += 1;
            }
        }
    }
    assert_eq!(geometries, 44);
    assert_eq!(never_reach, 12);
    for (dx, dy) in [(3, 0), (-3, 0), (0, 3), (0, -3)] {
        let attacker = (party.0 + dx, party.1 + dy);
        let samples = outdoor_ranged_attack_samples(attacker, party);
        assert!(samples.iter().all(|sample| (sample.column, sample.row) != party), "({dx}, {dy})");
    }
}

#[test]
fn ag8_overworld_s6_2_4_whole_party_pass_rolls_once_per_living_member_below_the_party_size() {
    // `overworld.md §6.2.4`: walk slots from zero; "for each slot index that
    // is **below the party-size byte** and whose **status byte is not the
    // dead marker**, it draws its own **fresh, independent** uniform integer
    // in the closed interval `[1, 8]`". "Maximum hit points, experience,
    // level, magic points and equipment are untouched".
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![
        ag8_member(0, b'A', b'G', 20, 50, 60, 7),
        ag8_member(1, b'F', b'D', 20, 0, 60, 0),
        ag8_member(2, b'M', b'S', 20, 50, 60, 9),
        ag8_member(3, b'B', b'P', 20, 50, 60, 4),
    ];
    state.prng_state = 0x2468;
    let mut expected = 0x2468u16;
    let rolls: Vec<u16> = (0..3).map(|_| u5_prng_range_u16(&mut expected, 1, 8)).collect();

    let applied = state.apply_outdoor_impact_party_damage();

    assert_eq!(applied.iter().map(|hit| hit.slot).collect::<Vec<_>>(), vec![0, 2, 3]);
    assert_eq!(state.party[0].hp, 50 - rolls[0]);
    assert_eq!(state.party[1].hp, 0);
    assert_eq!(state.party[2].hp, 50 - rolls[1]);
    assert_eq!(state.party[3].hp, 50 - rolls[2]);
    assert_eq!(state.prng_state, expected);
    for (index, mana) in [(0, 7u8), (2, 9), (3, 4)] {
        assert_eq!(state.party[index].max_hp, 60);
        assert_eq!(state.party[index].mana, mana);
        assert_eq!(state.party[index].level, 1);
    }
}

#[test]
fn ag8_overworld_s6_2_4_death_of_the_selected_member_clears_the_selector() {
    // `overworld.md §6.2.4`: "if the member that just died is the currently
    // selected character, writes the published 'none selected' value";
    // clamps HP at zero and writes Dead.
    let mut state = test_state(open_grid(), 1, 1);
    state.party = vec![
        ag8_member(0, b'A', b'G', 20, 50, 60, 0),
        ag8_member(1, b'F', b'G', 20, 2, 60, 0),
    ];
    state.active_player = Some(1);
    let hit = state.apply_shared_party_damage(1, 8);
    assert!(hit.died);
    assert_eq!(state.party[1].hp, 0);
    assert_eq!(state.party[1].status, b'D');
    assert_eq!(state.active_player, None);

    state.active_player = Some(0);
    let _ = state.apply_shared_party_damage(0, 1);
    assert_eq!(state.active_player, Some(0));
}

#[test]
fn ag8_overworld_s8_1_falls_and_whirlpool_lines_and_their_sixteen_column_rows() {
    // `overworld.md §8.1`: `F-A-L-L-S!!!\n`, `Falling into underworld!!\n`,
    // `\nWHIRLPOOL!\n`; the message window holds sixteen characters per row,
    // so the transition line renders as `Falling into` / `underworld!!`.
    assert_eq!(crate::commands::OVERWORLD_FALLS_BANNER, "F-A-L-L-S!!!\n");
    assert_eq!(
        crate::commands::OVERWORLD_FALLS_UNDERWORLD_NARRATION,
        "Falling into underworld!!\n"
    );
    assert_eq!(crate::commands::OVERWORLD_WHIRLPOOL_BANNER, "\nWHIRLPOOL!\n");
    assert_eq!(MESSAGE_WINDOW_LEFT, 24);
    assert_eq!(MESSAGE_WINDOW_RIGHT, 39);
    assert_eq!(MESSAGE_WINDOW_WIDTH, 16);
    assert_eq!(
        wrap_text("Falling into underworld!!", MESSAGE_WINDOW_WIDTH, 0),
        vec!["Falling into".to_string(), "underworld!!".to_string()]
    );
    assert_eq!(wrap_text("F-A-L-L-S!!!", MESSAGE_WINDOW_WIDTH, 0), vec!["F-A-L-L-S!!!".to_string()]);
}

#[test]
fn ag8_overworld_s9_1_phase_composition_rises_n_rows_out_of_the_ground() {
    // `overworld.md §9.1`: phase `N` in `1..15` is "the ground tile, with its
    // bottom *N* pixel rows replaced by the top *N* pixel rows of the
    // moon-gate tile"; 0 is ground and 16 the whole gate.
    let side = TILE_ATLAS_SIDE;
    let ground: Vec<u8> = (0..side * side).map(|index| (index / side) as u8).collect();
    let gate: Vec<u8> = (0..side * side).map(|index| 0x80 | (index / side) as u8).collect();
    for rows in 0..=16u8 {
        let mut scratch = vec![0xEE; side * side];
        compose_moongate_phase_frame(&mut scratch, &ground, &gate, rows).unwrap();
        let ground_rows = side - rows as usize;
        for row in 0..side {
            let expected = if row < ground_rows { row as u8 } else { 0x80 | (row - ground_rows) as u8 };
            assert!(scratch[row * side..(row + 1) * side].iter().all(|pixel| *pixel == expected), "phase {rows} row {row}");
        }
        let draw = moongate_phase_draw(rows);
        match rows {
            0 => assert_eq!(draw, MoongatePhaseDraw::Ground),
            16 => assert_eq!(draw, MoongatePhaseDraw::WholeGate),
            n => assert_eq!(draw, MoongatePhaseDraw::Composed { rows: n }),
        }
    }
    // The three-tile inventory: gate `0xDC`, ground `5` (endgame `0x44`),
    // scratch `0x116`.
    assert_eq!(moongate_phase_gate_tile(), 0xDC);
    assert_eq!(moongate_phase_ground_tile(false), 5);
    assert_eq!(moongate_phase_ground_tile(true), 0x44);
    assert_eq!(MOONGATE_PHASE_SCRATCH_TILE, 0x116);
}

#[test]
fn ag8_overworld_s9_presence_counter_rises_by_night_and_falls_by_day() {
    // `overworld.md §9`: "During night hours `20..23` and `0..4`, the counter
    // increases toward sixteen ... During hours `5..19`, the counter
    // decreases toward zero".
    for hour in 0..24u8 {
        let night = hour <= 4 || hour >= 20;
        for counter in 0..=16u8 {
            let expected = if night { (counter + 1).min(16) } else { counter.saturating_sub(1) };
            assert_eq!(natural_moongate_advance_counter(counter, hour), expected, "hour {hour} counter {counter}");
        }
    }
    assert_eq!(SAVE_NATURAL_MOONGATE_COUNTER_OFFSET, 0x02E1);
}

#[test]
fn ag8_overworld_s9_2_transit_outcome_depends_on_the_clock() {
    // `overworld.md §9.2`: "If the clock is in hour `0` and the minute is
    // below `10`, it reports success" to the kneel overlay; otherwise "before
    // noon it uses the first cached glyph, and from noon onward it uses the
    // second".
    for hour in 0..24u8 {
        for minute in 0..60u8 {
            assert_eq!(natural_moongate_dispatches_meditate(hour, minute), hour == 0 && minute < 10);
        }
        assert_eq!(natural_moongate_cached_glyph_slot(hour), u8::from(hour >= 12));
    }
}

#[test]
fn ag8_overworld_s9_2_transit_stage_counts() {
    // `overworld.md §9.2`: 255 of 256 pixels plotted, a world tick every
    // eight steps, fifteen stage-B phases from 15 to 1 at two BIOS ticks each,
    // ending at zero and restoring terrain 5.
    use crate::moongate_transit::*;
    assert_eq!(MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS, 256);
    assert_eq!(MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS, 255);
    assert_eq!(MOONGATE_TRANSIT_STAGE_A_WORLD_TICK_EVERY, 8);
    assert_eq!(MOONGATE_TRANSIT_STAGE_B_STEPS, 15);
    assert_eq!(MOONGATE_TRANSIT_STAGE_B_FIRST_PHASE, 15);
    assert_eq!(MOONGATE_TRANSIT_STAGE_B_LAST_PHASE, 1);
    assert_eq!(MOONGATE_TRANSIT_STAGE_B_STEP_BIOS_TICKS, 2);
    assert_eq!(MOONGATE_TRANSIT_END_COUNTER, 0);
    assert_eq!(MOONGATE_TRANSIT_CLEARED_TERRAIN, 5);
    assert_eq!(MOONGATE_TRANSIT_PARTY_PAINT_NOTHING_MARKER, 0x16);
    let order = moongate_transit_stage_a_pixel_order().unwrap();
    let distinct: std::collections::HashSet<usize> = order.iter().copied().collect();
    assert_eq!(distinct.len(), order.len());
    assert!(order.len() == 255 || order.len() == 256);
}

fn ag8_troll_state(party: Vec<PartyMember>, names: &[&str], strengths: Vec<u8>) -> PlayState {
    let mut grid = open_world_grid();
    grid[world_cell_index(2, 1)] = TROLL_BRIDGE_TILE_FIRST;
    let mut state = britannia_state(grid, 2, 1);
    state.party = party;
    state.normalize_party_progress_vectors();
    state.party_names = names.iter().map(|name| ag8_name(name)).collect();
    state.party_strengths = strengths;
    state
}

fn ag8_troll_triggered(base: &PlayState) -> PlayState {
    let seed = (0u32..8192)
        .find(|&seed| {
            let mut probe = base.clone();
            probe.prng_state = seed as u16;
            probe.apply_troll_bridge_ambush(WorldPlane::Britannia);
            probe.active_yes_no_prompt.is_some()
        })
        .expect("some seed triggers the ambush and catches a member");
    let mut state = base.clone();
    state.prng_state = seed as u16;
    state.apply_troll_bridge_ambush(WorldPlane::Britannia);
    state
}

#[test]
fn ag8_overworld_s7_1_dead_and_asleep_skip_and_the_toll_uses_the_first_able_member() {
    // `overworld.md §7.1` step 2: "A Dead or Asleep member is skipped with no
    // line and no roll"; step 4: the toll uses "the Strength of the **first
    // member able to act** ... not of the member who was caught".
    let base = ag8_troll_state(
        vec![
            ag8_member(0, b'A', b'G', 30, 30, 30, 0),
            ag8_member(1, b'F', b'S', 0, 30, 30, 0),
            ag8_member(2, b'F', b'D', 0, 0, 30, 0),
            ag8_member(3, b'F', b'G', 0, 30, 30, 0),
        ],
        &["Alpha", "Bravo", "Charlie", "Delta"],
        vec![10, 20, 25, 30],
    );
    let state = ag8_troll_triggered(&base);
    let texts: Vec<String> = state.message_entries().iter().map(|entry| entry.text.clone()).collect();
    let all = texts.join("\n");
    assert!(all.contains("Alpha sneaks across..."), "{all}");
    assert!(all.contains("Delta sneaks across..."), "{all}");
    assert!(!all.contains("Bravo"), "{all}");
    assert!(!all.contains("Charlie"), "{all}");
    // Alpha (Strength 10) is the first able member; Delta was caught.
    assert_eq!(
        state.active_yes_no_prompt,
        Some(YesNoPromptSession::new(YesNoPromptKind::TrollBridgeToll { toll: 69 }))
    );
}

#[test]
fn ag8_overworld_s7_1_toll_prints_in_a_two_column_space_padded_field() {
    // `overworld.md §7.1` step 3: "the toll as a decimal right-justified in a
    // two-column space-padded field". Strength 30 gives a toll of 9.
    let base = ag8_troll_state(vec![ag8_member(0, b'A', b'G', 0, 30, 30, 0)], &["Alpha"], vec![30]);
    let state = ag8_troll_triggered(&base);
    assert_eq!(state.message, "Caught!\n\nThe trolls demand a  9 gp toll!\n\nDost thou pay?");
}

#[test]
fn ag8_overworld_s7_1_exactly_affording_the_toll_pays() {
    // `overworld.md §7.1` step 5: "If gold is still non-negative the toll is
    // paid ... Exactly affording the toll pays."
    let mut state = britannia_state(open_world_grid(), 2, 1);
    state.gold = 54;
    state.active_yes_no_prompt = Some(YesNoPromptSession::new(YesNoPromptKind::TrollBridgeToll { toll: 54 }));
    let dir = debug_game_dir();
    state.resolve_troll_bridge_toll_payment(54, &dir).unwrap();
    assert_eq!(state.gold, 0);
    assert!(!state.combat_active);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ag8_overworld_s7_1_trigger_and_save_ranges() {
    // `overworld.md §7.1`: tiles `0x6A`/`0x6B`; trigger draw `0..7`; sneak
    // roll `1..30`; toll `99 - 3 x Strength`, signed (Strength 33 is free,
    // larger is negative); the troll is tile `0xE4`.
    assert_eq!((TROLL_BRIDGE_TILE_FIRST, TROLL_BRIDGE_TILE_LAST), (0x6A, 0x6B));
    assert_eq!((TROLL_BRIDGE_TRIGGER_ROLL_LOW, TROLL_BRIDGE_TRIGGER_ROLL_HIGH), (0, 7));
    assert_eq!((TROLL_BRIDGE_SNEAK_ROLL_LOW, TROLL_BRIDGE_SNEAK_ROLL_HIGH), (1, 30));
    assert_eq!(TROLL_BRIDGE_SPRITE_TILE, 0xE4);
    for strength in 0..=40u8 {
        assert_eq!(troll_bridge_toll_for_strength(strength), 99 - 3 * i16::from(strength));
    }
    assert!(troll_bridge_toll_for_strength(34) < 0);
    assert_eq!(TROLL_BRIDGE_ANNOUNCE_LINE, "\nThou spieth trolls under the bridge!\n\n");
    assert_eq!(TROLL_BRIDGE_EVADED_LINE, "Trolls evaded!\n");
}

// ---------------------------------------------------------------------------
// catalogs/gazetteer.md
// ---------------------------------------------------------------------------

fn ag8_location_scene_byte(target: PlayTarget) -> u8 {
    match target {
        PlayTarget::Town(scene) => scene.byte,
        PlayTarget::Dungeon(scene) => scene.byte,
        PlayTarget::World(_) => panic!("published rows never target the world"),
    }
}

#[test]
fn ag8_gazetteer_s5_1_forty_entry_return_table_and_narration_join() {
    // `gazetteer.md §5.1`: the full forty-entry X/Y table, its accepted
    // plane, the narration class, the centred proper name and its column,
    // and the stock live tile.
    use WorldEntryNarrationClass::*;
    type Row = (u8, WorldPlane, usize, usize, WorldEntryNarrationClass, Option<&'static str>, Option<u8>, u8);
    let b = WorldPlane::Britannia;
    let u = WorldPlane::Underworld;
    let rows: [Row; 40] = [
        (1, b, 232, 135, Towne, Some("MOONGLOW"), Some(4), 0x14),
        (2, b, 81, 106, Towne, Some("BRITAIN"), Some(4), 0x14),
        (3, b, 36, 222, Towne, Some("JHELOM"), Some(5), 0x14),
        (4, b, 58, 43, Towne, Some("YEW"), Some(6), 0x14),
        (5, b, 159, 20, Towne, Some("MINOC"), Some(5), 0x14),
        (6, b, 106, 184, Towne, Some("TRINSIC"), Some(4), 0x14),
        (7, b, 22, 128, Towne, Some("SKARA BRAE"), Some(3), 0x14),
        (8, b, 187, 169, Village, Some("NEW MAGINCIA"), Some(2), 0x13),
        (9, b, 88, 120, Lighthouse, Some("FOGSBANE"), Some(4), 0x1B),
        (10, b, 152, 24, Lighthouse, Some("STORMCROW"), Some(3), 0x1B),
        (11, b, 104, 216, Lighthouse, Some("GREYHAVEN"), Some(3), 0x1B),
        (12, b, 216, 120, Lighthouse, Some("WAVEGUIDE"), Some(3), 0x1B),
        (13, b, 45, 62, Hut, Some("IOLO'S HUT"), Some(3), 0x10),
        (14, b, 176, 208, Hut, None, None, 0x10),
        (15, b, 201, 59, Hut, None, None, 0x10),
        (16, b, 153, 91, Hut, None, None, 0x10),
        (17, b, 86, 107, LordBritish, None, None, 0x3E),
        (18, b, 196, 245, Blackthorn, None, None, 0x39),
        (19, b, 84, 106, Village, Some("WEST BRITANNY"), Some(1), 0x13),
        (20, b, 86, 105, Village, Some("NORTH BRITANNY"), Some(1), 0x13),
        (21, b, 88, 106, Village, Some("EAST BRITANNY"), Some(1), 0x13),
        (22, b, 98, 145, Village, Some("PAWS"), Some(6), 0x13),
        (23, b, 136, 90, Village, Some("COVE"), Some(6), 0x13),
        (24, b, 136, 158, Towne, Some("BUCCANEER'S DEN"), Some(0), 0x14),
        (25, u, 49, 58, Keep, Some("ARARAT"), Some(5), 0x12),
        (26, b, 15, 160, Keep, Some("BORDERMARCH"), Some(2), 0x12),
        (27, b, 64, 240, Keep, Some("FARTHING"), Some(4), 0x12),
        (28, b, 248, 8, Keep, Some("WINDEMERE"), Some(3), 0x12),
        (29, b, 148, 74, Keep, Some("STONEGATE"), Some(3), 0x12),
        (30, b, 218, 107, Castle, Some("THE LYCAEUM"), Some(2), 0x15),
        (31, b, 28, 50, Castle, Some("EMPATH ABBEY"), Some(2), 0x15),
        (32, b, 146, 241, Castle, Some("SERPENT'S HOLD"), Some(1), 0x15),
        (33, b, 240, 73, Dungeon, Some("DECEIT"), Some(5), 0x18),
        (34, b, 91, 67, Cave, Some("DESPISE"), Some(4), 0x16),
        (35, b, 72, 168, Cave, Some("DESTARD"), Some(4), 0x16),
        (36, b, 126, 20, Dungeon, Some("WRONG"), Some(5), 0x18),
        (37, b, 156, 27, Dungeon, Some("COVETOUS"), Some(4), 0x18),
        (38, b, 58, 102, Mine, Some("SHAME"), Some(5), 0x17),
        (39, b, 239, 240, Mine, Some("HYTHLOTH"), Some(4), 0x17),
        (40, u, 128, 128, Cave, None, None, 0x16),
    ];
    let entries = published_world_location_entries();
    assert_eq!(entries.len(), 40);
    for (scene, plane, x, y, class, name, column, tile) in rows {
        let entry = entries
            .iter()
            .find(|entry| ag8_location_scene_byte(entry.target) == scene)
            .unwrap_or_else(|| panic!("scene {scene} missing"));
        assert_eq!((entry.plane, entry.x, entry.y), (plane, x, y), "scene {scene}");
        assert_eq!(entry.narration_class, Some(class), "scene {scene}");
        assert_eq!(entry.proper_name, name, "scene {scene}");
        assert_eq!(entry.name_column, column, "scene {scene}");
        assert_eq!(entry.expected_tile, Some(tile), "scene {scene}");
        // §5.1: the seven ordinary dungeon rows carry an entrance on both
        // planes; Doom is Underworld-only.
        assert_eq!(entry.accepts_both_world_planes, (33..=39).contains(&scene), "scene {scene}");
    }
    // "all forty pairs are distinct".
    let pairs: std::collections::HashSet<(usize, usize)> = entries.iter().map(|entry| (entry.x, entry.y)).collect();
    assert_eq!(pairs.len(), 40);
}

#[test]
fn ag8_gazetteer_s5_1_live_tile_sets_select_the_narration() {
    // `gazetteer.md §5.1`: `T` = `{0x10, 0x12, 0x13, 0x14, 0x15, 0x1B, 0x39,
    // 0x3E}` (town helper) and `D` = `{0x16, 0x17, 0x18}` (dungeon helper);
    // "the actual live tile selects the narration".
    use WorldEntryNarrationClass::*;
    for tile in 0..=u8::MAX {
        let expected = match tile {
            0x10 => Some((Hut, WorldEntryHelper::Town, "hut")),
            0x12 => Some((Keep, WorldEntryHelper::Town, "keep")),
            0x13 => Some((Village, WorldEntryHelper::Town, "village")),
            0x14 => Some((Towne, WorldEntryHelper::Town, "towne")),
            0x15 => Some((Castle, WorldEntryHelper::Town, "castle")),
            0x1B => Some((Lighthouse, WorldEntryHelper::Town, "lighthouse")),
            0x39 => Some((Blackthorn, WorldEntryHelper::Town, "the palace of Blackthorn!")),
            0x3E => Some((LordBritish, WorldEntryHelper::Town, "the Castle of Lord British!")),
            0x16 => Some((Cave, WorldEntryHelper::Dungeon, "cave")),
            0x17 => Some((Mine, WorldEntryHelper::Dungeon, "mine")),
            0x18 => Some((Dungeon, WorldEntryHelper::Dungeon, "dungeon")),
            _ => None,
        };
        let actual = WorldEntryNarrationClass::from_live_tile(tile).map(|class| (class, class.helper(), class.text()));
        assert_eq!(actual, expected, "tile {tile:#04x}");
    }
}

#[test]
fn ag8_gazetteer_s3_scene_partition_names_the_storage_key() {
    // `gazetteer.md §3`: scene bytes `1..8` Town, `9..16` Dwelling, `17..24`
    // Castle, `25..32` Keep, sub-map key `(scene - 1) & 7`; zero is the
    // overworld and values outside `1..32` do not name a town-mode location.
    for scene in 1..=32u8 {
        let family = match scene {
            1..=8 => "TOWNE",
            9..=16 => "DWELLING",
            17..=24 => "CASTLE",
            _ => "KEEP",
        };
        assert_eq!(Scene::new(scene).unwrap().key(), format!("{family}:{}", (scene - 1) & 7));
    }
    assert!(Scene::new(0).is_err());
    assert!(Scene::new(33).is_err());
}

#[test]
fn ag8_gazetteer_s5_resident_names_in_resident_order() {
    // `gazetteer.md §5`: resident names for the thirty-two town-mode rows;
    // scenes 14..16 are blank.
    let expected: [(u8, Option<&str>); 32] = [
        (1, Some("Moonglow")),
        (2, Some("Britain")),
        (3, Some("Jhelom")),
        (4, Some("Yew")),
        (5, Some("Minoc")),
        (6, Some("Trinsic")),
        (7, Some("Skara Brae")),
        (8, Some("New Magincia")),
        (9, Some("Fogsbane")),
        (10, Some("Stormcrow")),
        (11, Some("Greyhaven")),
        (12, Some("Waveguide")),
        (13, Some("Iolo's Hut")),
        (14, None),
        (15, None),
        (16, None),
        (17, Some("Lord British's Castle")),
        (18, Some("Lord Blackthorn's Castle")),
        (19, Some("West Britanny")),
        (20, Some("North Britanny")),
        (21, Some("East Britanny")),
        (22, Some("Paws")),
        (23, Some("Cove")),
        (24, Some("Buccaneer's Den")),
        (25, Some("Ararat")),
        (26, Some("Bordermarch")),
        (27, Some("Farthing")),
        (28, Some("Windemere")),
        (29, Some("Stonegate")),
        (30, Some("The Lycaeum")),
        (31, Some("Empath Abbey")),
        (32, Some("Serpent's Hold")),
    ];
    for (scene, name) in expected {
        assert_eq!(town_resident_name(scene), name, "scene {scene}");
    }
    assert_eq!(town_resident_name(0), None);
    assert_eq!(town_resident_name(33), None);
}

#[test]
fn ag8_gazetteer_s6_dungeon_scene_record_name_and_flavour_order() {
    // `gazetteer.md §6`: scenes 33..40 bind records 0..7, Deceit through
    // Doom, with flavour byte 3 for Deceit/Wrong/Covetous and the mine
    // flavour for Shame/Hythloth.
    let expected = [
        ("Deceit", DungeonPresentationFlavour::FlavourByte3),
        ("Despise", DungeonPresentationFlavour::Normal),
        ("Destard", DungeonPresentationFlavour::Normal),
        ("Wrong", DungeonPresentationFlavour::FlavourByte3),
        ("Covetous", DungeonPresentationFlavour::FlavourByte3),
        ("Shame", DungeonPresentationFlavour::Mine),
        ("Hythloth", DungeonPresentationFlavour::Mine),
        ("Doom", DungeonPresentationFlavour::Normal),
    ];
    for (record, (name, flavour)) in expected.into_iter().enumerate() {
        let scene = DungeonScene::new(33 + record as u8).unwrap();
        assert_eq!(scene.record, record);
        assert_eq!(scene.key(), format!("DUNGEON:{record}"));
        assert_eq!(scene.name(), name);
        assert_eq!(scene.presentation_flavour(), flavour);
    }
    assert!(DungeonScene::new(32).is_err());
    assert!(DungeonScene::new(41).is_err());
}

#[test]
fn ag8_gazetteer_s7_one_shrine_table_in_virtue_order_with_mantras() {
    // `gazetteer.md §7`: "There is **one** resident shrine coordinate table"
    // in the standard virtue order, Spirituality at the `(0, 0)` sentinel,
    // each virtue with exactly one mantra.
    use ShrineVirtue::*;
    let expected = [
        (Honesty, 233, 66, "Ahm"),
        (Compassion, 128, 92, "Mu"),
        (Valor, 36, 229, "Ra"),
        (Justice, 73, 11, "Beh"),
        (Sacrifice, 205, 45, "Cah"),
        (Honor, 81, 207, "Summ"),
        (Spirituality, 0, 0, "Om"),
        (Humility, 231, 216, "Lum"),
    ];
    for (index, (virtue, x, y, mantra)) in expected.into_iter().enumerate() {
        assert_eq!(PUBLISHED_SHRINE_COORDINATES[index], (virtue, x, y));
        assert_eq!(WORLD_SHRINE_COORDINATES[index], (x, y));
        assert_eq!(virtue.mantra(), mantra);
        assert_eq!(virtue.index(), index);
    }
    for entry in published_shrine_entries() {
        assert_eq!(entry.plane, WorldPlane::Britannia);
    }
}

// ---------------------------------------------------------------------------
// systems/display-driver-abi.md
// ---------------------------------------------------------------------------

#[test]
fn ag8_display_s9_2_line_is_front_buffer_only_and_includes_both_endpoints() {
    // `display-driver-abi.md §9.2`: `0x33` is "a textbook integer Bresenham
    // step over the major axis ... so the line includes both endpoints and
    // covers all eight octants", and "lines are front-buffer-only regardless
    // of the render-target selector".
    let centre = (100i32, 100i32);
    for (dx, dy) in [(9, 3), (3, 9), (-3, 9), (-9, 3), (-9, -3), (-3, -9), (3, -9), (9, -3), (0, 7), (7, 0)] {
        let mut surface = EgaDisplaySurface::new();
        surface.execute(EgaDisplayOperation::SetRenderTarget(DisplayRenderTarget::Back)).unwrap();
        surface.execute(EgaDisplayOperation::SetCurrentColor(12)).unwrap();
        surface
            .execute(EgaDisplayOperation::DrawLine {
                x0: centre.0,
                y0: centre.1,
                x1: centre.0 + dx,
                y1: centre.1 + dy,
            })
            .unwrap();
        let lit = surface.front_pixels().iter().filter(|pixel| **pixel == 12).count();
        assert_eq!(lit as i32, dx.abs().max(dy.abs()) + 1, "({dx}, {dy})");
        assert_eq!(surface.read_pixel(centre.0 as usize, centre.1 as usize), Some(12));
        assert_eq!(surface.read_pixel((centre.0 + dx) as usize, (centre.1 + dy) as usize), Some(12));
        assert!(surface.back_pixels().iter().all(|pixel| *pixel == 0));
    }
}

#[test]
fn ag8_display_s9_2_pixel_read_ignores_the_render_target() {
    // `display-driver-abi.md §9.2`: `0x24` "always reads from the visible
    // page and ignores the render-target selector".
    let mut surface = EgaDisplaySurface::new();
    let rect = normalize_clamp_pixel_rect(10, 10, 10, 10).unwrap();
    surface.fill_back_rect(rect, 5);
    surface.fill_rect(rect, 9);
    surface.execute(EgaDisplayOperation::SetRenderTarget(DisplayRenderTarget::Back)).unwrap();
    assert_eq!(
        surface.execute(EgaDisplayOperation::ReadPixel { x: 10, y: 10 }).unwrap(),
        EgaDispatchResult::Pixel(9)
    );
}

#[test]
fn ag8_display_s9_1_colour_register_holds_four_bits_and_height_is_200() {
    // `display-driver-abi.md §9.1`: `0x2D` "stores a 4-bit drawing colour";
    // §5 slot 0 returns "the screen height constant, 200"; 38 slots of three
    // bytes.
    let mut surface = EgaDisplaySurface::new();
    surface.execute(EgaDisplayOperation::SetCurrentColor(0x1F)).unwrap();
    assert_eq!(surface.current_color(), 0x0F);
    assert_eq!(
        surface.execute(EgaDisplayOperation::ScreenHeight).unwrap(),
        EgaDispatchResult::ScreenHeight(200)
    );
    for slot in 0..EGA_DRIVER_SLOT_COUNT {
        assert_eq!(ega_driver_dispatch_offset(slot), Some(slot * 3));
        assert_eq!(ega_driver_dispatch_slot(slot * 3), Some(slot));
        assert_eq!(ega_driver_dispatch_slot(slot * 3 + 1), None);
    }
}

#[test]
fn ag8_display_s9_6_dissolve_visits_each_pixel_once_in_a_scattered_deterministic_order() {
    // `display-driver-abi.md §9.6`: "every pixel inside the inclusive
    // rectangle is visited exactly once", "deterministic and reproducible
    // across calls with the same rectangle dimensions", and "not row-major,
    // not column-major".
    let rect = normalize_clamp_pixel_rect(8, 8, 23, 23).unwrap();
    let collect = || {
        let mut state = EgaDissolveState::new(rect);
        let mut order = Vec::new();
        while let Some(pixel) = state.next_pixel() {
            order.push(pixel);
        }
        order
    };
    let first = collect();
    assert_eq!(first, collect());
    assert_eq!(first.len(), 256);
    let distinct: std::collections::HashSet<(usize, usize)> = first.iter().copied().collect();
    assert_eq!(distinct.len(), 256);
    assert!(distinct.iter().all(|&(x, y)| (8..=23).contains(&x) && (8..=23).contains(&y)));
    let row_major: Vec<(usize, usize)> = (8..=23).flat_map(|y| (8..=23).map(move |x| (x, y))).collect();
    let column_major: Vec<(usize, usize)> = (8..=23).flat_map(|x| (8..=23).map(move |y| (x, y))).collect();
    assert_ne!(first, row_major);
    assert_ne!(first, column_major);
}

#[test]
fn ag8_display_s9_6_dissolve_reads_hidden_and_writes_visible_whatever_the_selector() {
    // `display-driver-abi.md §9.6`: "The dissolve entry itself always reads
    // the hidden surface and always writes the visible page; it ignores the
    // render-target selector entirely."
    let rect = normalize_clamp_pixel_rect(8, 8, 183, 183).unwrap();
    for target in [DisplayRenderTarget::Front, DisplayRenderTarget::Back] {
        let mut surface = EgaDisplaySurface::new();
        surface.fill_back_rect(rect, 7);
        surface.execute(EgaDisplayOperation::SetRenderTarget(target)).unwrap();
        surface.execute(EgaDisplayOperation::DissolveBackToFront(rect)).unwrap();
        for (y, x) in [(8usize, 8usize), (183, 183), (100, 50)] {
            assert_eq!(surface.read_pixel(x, y), Some(7));
        }
        assert_eq!(surface.read_pixel(7, 8), Some(0));
        assert_eq!(surface.read_pixel(184, 183), Some(0));
        assert!(surface.back_pixels()[8 * DISPLAY_SURFACE_WIDTH + 8] == 7);
    }
}

#[test]
fn ag8_display_s9_5_message_fast_path_reads_only_the_left_edge() {
    // `display-driver-abi.md §9.5`: on the left-edge-192 path "The requested
    // top, right and bottom edges are all ignored ... produce
    // **byte-identical** rasters", each destination scanline takes the one
    // eight below it, and "The left three quarters of every scanline the
    // path touches is neither read nor written."
    let seed = |surface: &mut EgaDisplaySurface| {
        for y in 0..DISPLAY_SURFACE_HEIGHT {
            let rect = normalize_clamp_pixel_rect(0, y as i32, 319, y as i32).unwrap();
            surface.fill_rect(rect, (y % 16) as u8);
        }
    };
    let mut a = EgaDisplaySurface::new();
    seed(&mut a);
    let before = a.front_pixels().to_vec();
    let mut b = a.clone();
    a.execute(EgaDisplayOperation::ScrollRect {
        rect: DisplayPixelRect { x0: 192, y0: 8, x1: 319, y1: 87 },
        rows: -8,
    })
    .unwrap();
    b.execute(EgaDisplayOperation::ScrollRect {
        rect: DisplayPixelRect { x0: 192, y0: 88, x1: 200, y1: 199 },
        rows: 3,
    })
    .unwrap();
    assert_eq!(a.front_pixels(), b.front_pixels());
    for y in 0..DISPLAY_SURFACE_HEIGHT {
        for x in 0..DISPLAY_SURFACE_WIDTH {
            let now = a.front_pixels()[y * DISPLAY_SURFACE_WIDTH + x];
            if x < 192 || y < 88 {
                assert_eq!(now, before[y * DISPLAY_SURFACE_WIDTH + x], "({x}, {y}) moved");
            } else if y + 8 < DISPLAY_SURFACE_HEIGHT {
                assert_eq!(now, before[(y + 8) * DISPLAY_SURFACE_WIDTH + x], "({x}, {y})");
            }
        }
    }
}

#[test]
fn ag8_display_s10_endgame_remap_table_and_tile_set() {
    // `display-driver-abi.md §10`: the sixteen-entry map ("5 becomes 1; 13
    // becomes 9") and the twenty-two remapped tiles.
    use crate::endgame::{ENDGAME_TILESET_NIBBLE_MAP, ENDGAME_TILESET_REMAP_TILES};
    assert_eq!(ENDGAME_TILESET_NIBBLE_MAP, [0, 5, 4, 4, 2, 1, 2, 7, 8, 12, 12, 12, 10, 9, 14, 15]);
    let mut tiles = ENDGAME_TILESET_REMAP_TILES.to_vec();
    tiles.sort_unstable();
    let mut expected = vec![
        0x44, 0x5C, 0x5D, 0x90, 0x92, 0x94, 0x96, 0x9B, 0xAB, 0xAC, 0xAF, 0xB0, 0xB1, 0xBF, 0xDC, 0x08,
        0x0E, 0x1A, 0x38, 0x39, 0x3A, 0x3B,
    ];
    expected.sort_unstable();
    assert_eq!(tiles, expected);
    let mut pixels: Vec<u8> = (0..16).collect();
    crate::endgame::remap_endgame_tileset_pixels(&mut pixels);
    assert_eq!(pixels, ENDGAME_TILESET_NIBBLE_MAP.to_vec());
}

#[test]
fn ag8_display_s9_6_single_cell_converge_is_the_published_permutation() {
    // `display-driver-abi.md §9.6`: "Call 0 writes source pixel `(0,0)`.
    // Calls 1 through 255 begin with eight-bit state 1, write `(x = state >>
    // 4, y = state & 15)`, then shift the state right and XOR the result with
    // `0xB8` when the old low bit was one", a 256-position permutation.
    let order = crate::return_to_view::return_to_view_single_cell_write_coordinates();
    assert_eq!(order[0], (0, 0));
    let mut state = 1u8;
    for write in order.iter().skip(1) {
        assert_eq!(*write, (state >> 4, state & 0x0F));
        let low = state & 1;
        state >>= 1;
        if low != 0 {
            state ^= 0xB8;
        }
    }
    let distinct: std::collections::HashSet<(u8, u8)> = order.iter().copied().collect();
    assert_eq!(distinct.len(), 256);
}

#[test]
fn ag8_display_s10_idle_strip_advances_modulo_four() {
    // `display-driver-abi.md §10`: the idle strip is "a four-frame strip drawn
    // at `(0, 65)` with size `320 x 49`" and the entry "advances the counter
    // modulo four".
    assert_eq!((TITLE_TICK_FRAME_X, TITLE_TICK_FRAME_Y), (0, 65));
    assert_eq!((TITLE_TICK_FRAME_WIDTH, TITLE_TICK_FRAME_HEIGHT), (320, 49));
    assert_eq!(TITLE_TICK_FRAME_COUNT, 4);
    for frame in 0..4u8 {
        assert_eq!(title_tick_next_frame(frame), (frame + 1) % 4);
    }
}

// ---------------------------------------------------------------------------
// formats/miscmsg-dat.md
// ---------------------------------------------------------------------------

fn ag8_miscmsg_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    for index in 0..MISCMSG_DAT_RECORDS {
        bytes.extend_from_slice(format!("REC{index}").as_bytes());
        bytes.push(0);
    }
    bytes
}

#[test]
fn ag8_miscmsg_s2_records_are_sequential_with_no_header_or_offset_table() {
    // `formats/miscmsg-dat.md §2`: "forty-seven NUL-terminated records
    // stored sequentially", header "None", offset table "None in the file".
    let messages = parse_misc_messages(&ag8_miscmsg_bytes()).unwrap();
    assert_eq!(messages.records.len(), 47);
    for index in 0..47 {
        assert_eq!(messages.record(index), Some(format!("REC{index}").as_str()));
    }
}

#[test]
fn ag8_miscmsg_s6_a_record_past_the_count_is_missing_not_the_next_family() {
    // `formats/miscmsg-dat.md §6`: "If a consumer requests a record outside
    // the available count, a modern implementation should report a
    // missing-message error. Falling through to the next family can produce
    // misleading virtue or Codex text."
    let messages = parse_misc_messages(&ag8_miscmsg_bytes()).unwrap();
    assert_eq!(messages.record(47), None);
    assert_eq!(messages.record(usize::MAX), None);
    assert_eq!(messages.urn_codex_for_virtue_index(8), None);
}

#[test]
fn ag8_miscmsg_s4_tile_glyph_text_expands_the_four_codes() {
    // `formats/miscmsg-dat.md §4`: `@` space, `[` TH, `]` NG, `_` ST.
    assert_eq!(render_miscmsg_tile_glyph_text("[E@KI]@MU_"), "THE KING MUST");
    assert_eq!(render_miscmsg_tile_glyph_text("PLAIN TEXT."), "PLAIN TEXT.");
}
