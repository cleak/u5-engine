// Regressions for the p6town exploratory play-test findings (K01..K19).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// Playtest K03/K10. `town-mode.md §13`: "After the map has been loaded and
/// the host recorded, a settlement that hosts a Shadowlord prints one line
/// naming which — an air of falsehood, of hatred, or of cowardice doth
/// surround thee". An overworld `E`-Enter replaced the message slot the
/// load-time pass had printed into, so the line never reached the window.
#[test]
fn p6town_entering_a_hideout_town_prints_its_air_line() {
    let dir = debug_game_dir();
    write_all_location_family_fixtures(&dir);
    let entry = published_world_location_entries()
        .into_iter()
        .find(|entry| matches!(entry.target, PlayTarget::Town(scene) if scene.byte == 2))
        .expect("Britain has a published overworld entry");
    let mut grid = open_world_grid();
    grid[world_cell_index(entry.x, entry.y)] = entry.expected_tile.unwrap();
    let mut state = world_state(grid, entry.x, entry.y);
    state.area = Area::World { plane: entry.plane };
    state.active_objects[0].z = entry.plane.save_floor();
    // Falsehood in Britain, Hatred in Trinsic, Cowardice in Moonglow.
    state.shadowlord_hideouts = [2, 6, 1];

    state.begin_command_echo_for(Command::Enter);
    state.enter_current_location(&dir).unwrap();

    let rows: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let name = rows
        .iter()
        .rposition(|row| *row == "BRITAIN")
        .expect("entry narration row");
    // u5-spec#300 Q4: one blank row between the centred name and `An air
    // of`, then the notice's tone (`(1, 2000, 60000, 1, 6600)`), once.
    assert_eq!(
        &rows[name + 1..],
        &["", "An air of", "falsehood doth", "surround thee..."]
    );
    // §13 "The notice's exact shape": the notice ends with its own line feed
    // (playtest P13-07).
    assert!(!state.message_row_open_mid_line);
    assert!(!state.message.contains("hatred"));
    assert_eq!(
        state
            .sound_effects_after(0)
            .iter()
            .filter(|effect| **effect == SoundEffect::ShadowlordPresenceNotice)
            .count(),
        1
    );
    let _ = fs::remove_dir_all(dir);
}

/// Playtest K03/K10, the other half of §13: "a settlement hosting none prints
/// nothing".
#[test]
fn p6town_entering_a_town_hosting_no_shadowlord_prints_no_air_line() {
    let dir = debug_game_dir();
    write_all_location_family_fixtures(&dir);
    let entry = published_world_location_entries()
        .into_iter()
        .find(|entry| matches!(entry.target, PlayTarget::Town(scene) if scene.byte == 2))
        .expect("Britain has a published overworld entry");
    let mut grid = open_world_grid();
    grid[world_cell_index(entry.x, entry.y)] = entry.expected_tile.unwrap();
    let mut state = world_state(grid, entry.x, entry.y);
    state.area = Area::World { plane: entry.plane };
    state.active_objects[0].z = entry.plane.save_floor();
    state.shadowlord_hideouts = [4, 6, 1];

    state.begin_command_echo_for(Command::Enter);
    state.enter_current_location(&dir).unwrap();

    assert!(!state
        .message_entries()
        .iter()
        .any(|entry| entry.text.starts_with("An air of")));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest K05. `time.md §10`: the surrender path "advances time in repeated
/// twenty-minute cleanup calls until the hour byte reaches 08:00", and
/// `npc-schedules.md §5` names the schedule processor's only callers as the
/// town turn loop and the H-Hole-Up burst. The wait must not walk NPCs.
#[test]
fn p6town_arrest_wait_to_morning_runs_no_schedule_pass() {
    // The same NPC engages and steps on a single schedule pass
    // (`a133_mode_four_engages_from_the_waypoint_distance_alone`).
    let mut state = g6_town_with(vec![g6_npc(1, 15, 5, 4, (6, 6, 0))], 5, 5);
    state.clock = GameClock::new(6, 30).unwrap();
    let ticks = state.advance_town_arrest_wait_to_morning();
    assert!(ticks > 0);
    assert_eq!(state.clock.hour, 8);
    assert_eq!(g6_pos(&state, 0), (15, 5), "no NPC moves during the wait");
}

/// Playtest K16. `npc-schedules.md §9.2` / `commands.md §3`: contact is
/// evaluated in the town epilogue *after* the command, so a conversation it
/// opens follows the command's own result instead of replacing it.
#[test]
fn p6town_contact_conversation_keeps_the_commands_result_line() {
    let dir = debug_game_dir();
    let keeper = NpcSlot {
        slot: 1,
        type_byte: 0x54,
        dialog_id: 0x81,
        schedule: [4, 4, 4, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
        name: None,
    };
    let idle = NpcSlot {
        slot: 0,
        type_byte: 0,
        dialog_id: 0,
        schedule: [0; 16],
        name: None,
    };
    let mut state = test_state(open_grid(), 5, 5);
    let Area::Town { scene, floor } = state.area else {
        unreachable!()
    };
    state.load_scheduled_npcs(&[idle, keeper]);
    state.begin_command_echo(top_down_command_echo('O').unwrap());
    state.message = "Nothing to open!".to_string();
    state.advance_npc_schedules();
    assert!(state.pending_town_npc_contact.is_some());
    let outcome = state.apply_town_npc_contact_event(scene, floor, &dir).unwrap();
    assert!(outcome.is_some());
    assert!(state.diagnostics_contain("opens conversation contact"));
    state.flush_message_slot();
    let rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    assert!(
        rows.iter().any(|row| row.ends_with("Nothing to open!")),
        "the command's result survives the contact: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `town-mode.md §10` (u5-spec#300 Q18): a resident Shadowlord (roster type
/// `0xFC`) "is therefore a **valid conflict target**: the attack reaches the
/// NPC-conflict chain ... which forces arena 10 for `0xFC..0xFF`", with no
/// karma loss, no alarm and no removal mark. Playtest K08 had refused it
/// with `Nothing to attack!`.
#[test]
fn spec300_q18_attack_on_a_resident_shadowlord_opens_a_conflict() {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut shadowlord = g6_npc(31, 6, 5, 0, (6, 5, 0));
    shadowlord.type_byte = SHADOWLORD_ACTOR_TILE;
    let mut state = g6_town_with(vec![shadowlord], 5, 5);
    assert!(state.town_attack_target_at(0, 6, 5).is_some());
    let karma = state.moral_standing;

    assert_eq!(
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert!(state.combat_active);
    assert_ne!(state.message, ATTACK_NOTHING_TO_ATTACK_REFUSAL);
    assert_eq!(
        state.pending_town_conflict.map(|pending| pending.type_byte),
        Some(SHADOWLORD_ACTOR_TILE)
    );
    assert_eq!(state.moral_standing, karma);
    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);
    assert!(state.removed_town_npc_flags.values().all(|mask| *mask == 0));
    let _ = fs::remove_dir_all(dir);
}

/// Playtest K12. `conversation.md §2` step 5: the five dispatcher branches,
/// the silent-figure (cowering) index among them, "every one of them follows
/// the dispatcher's own leading line feed" - the blank row the guard refusal
/// already leaves under `Talk-<dir>`.
#[test]
fn p6town_cowering_and_scare_talk_follow_the_dispatchers_leading_feed() {
    for dialog_id in [TOWN_NPC_COWERING_DIALOG_ID, TOWN_NPC_BRUSHOFF_DIALOG_ID] {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
            NpcSlot {
                slot: 1,
                type_byte: 0x50,
                dialog_id,
                schedule: [3, 3, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0, 6, 12, 18, 22],
                name: None,
            },
        ]);
        state.begin_command_echo_for(Command::Talk);
        assert_eq!(
            state.talk_facing_with_dialogue(&HashMap::new()),
            MoveOutcome::Talked
        );
        assert!(
            state.message.starts_with('\n'),
            "dialog 0x{dialog_id:02X}: {:?}",
            state.message
        );
    }
}

/// `town-mode.md §14` (u5-spec#300 Q19, R601): Talk on dialogue `0xFE`
/// prints the dispatcher's leading line feed and `"Begone,` / `vermin!"`,
/// then runs the whole-party damage pass (each non-Dead slot takes `1..8`);
/// the flight helper is not given the talked-to NPC's number, so that NPC
/// is not rewritten. Playtest K13 had printed nothing and rewritten it.
#[test]
fn spec300_q19_scare_talk_prints_begone_vermin_and_damages_the_party() {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 0x50,
            dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
            schedule: [7, 7, 7, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ]);
    let hp = state.party[0].hp;
    state.talk_facing_with_dialogue(&HashMap::new());
    assert_eq!(state.message, format!("\n{TOWN_NPC_BRUSHOFF_LINE}"));
    assert!((1..=8).contains(&(hp - state.party[0].hp)));
    assert_eq!(&state.npcs[0].schedule[..3], &[7, 7, 7]);
}

/// `conversation.md §2` step 5, the scare index row (added 2026-10-25,
/// u5-spec#300 Q19, R601): Talk on `0xFE` prints `"Begone,` / `vermin!"`
/// after the dispatcher's feed, then runs the whole-party damage pass (each
/// non-Dead slot takes its own `1..8`) and the flight helper. On an explicit
/// Talk the helper does not get the talked-to NPC, so that record keeps its
/// schedule; on automatic contact it is the contacting NPC.
#[test]
fn p6town_scare_talk_prints_begone_and_damages_the_living_party() {
    let npcs = [
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 0x50,
            dialog_id: TOWN_NPC_BRUSHOFF_DIALOG_ID,
            schedule: [7, 7, 7, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            name: None,
        },
    ];
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    let template = state.party[0].clone();
    state.party[0].hp = 100;
    state.party.push(PartyMember { slot: 1, hp: 100, status: b'D', ..template.clone() });
    state.party.push(PartyMember { slot: 2, hp: 100, status: b'S', ..template });
    state.load_scheduled_npcs(&npcs);

    assert_eq!(state.talk_facing_with_dialogue(&HashMap::new()), MoveOutcome::Talked);

    assert_eq!(state.message, format!("\n{TOWN_NPC_BRUSHOFF_RESPONSE}"));
    assert_eq!(TOWN_NPC_BRUSHOFF_RESPONSE, "\"Begone,\nvermin!\"");
    for slot in [0, 2] {
        let lost = 100 - state.party[slot].hp;
        assert!((1..=8).contains(&lost), "slot {slot} lost {lost}");
    }
    assert_eq!(state.party[1].hp, 100, "a Dead slot is skipped");
    assert_eq!(&state.npcs[0].schedule[..3], &[7, 7, 7]);

    // Automatic conversation contact hands the flight helper the contacting NPC.
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&npcs);
    state.talk_entered_by_contact = true;
    state.talk_facing_with_dialogue(&HashMap::new());
    state.talk_entered_by_contact = false;
    assert_eq!(&state.npcs[0].schedule[..3], &[3, 3, 3]);
}

/// Playtest K19. `view.md §3` step 3 reads the Look target "through the
/// shared live-map tile query", and `town-mode.md §15` has that accessor
/// return the loaded floor's `(31,31)` cell for any out-of-range coordinate,
/// for "every caller". Look West from column 0 printed nothing at all.
#[test]
fn p6town_look_off_the_town_grid_describes_the_substituted_corner_cell() {
    let mut grid = open_grid();
    grid[31 * 32 + 31] = 0x05;
    let mut state = test_state(grid, 0, 5);
    assert!(matches!(state.area, Area::Town { .. }));
    let expected = format!(
        "{LOOK_RESULT_PREFIX}\n{}",
        state.look_description(0x05, None, 31)
    );

    assert_eq!(state.look_direction_with_table(Direction::West, None), MoveOutcome::Observed);

    assert_eq!(state.message, expected);
}

/// Playtest K01. `formats/signs-dat.md §4`: "Bytes `0x29..0x31`" select macro
/// fragments, "Other bytes print as their low-seven-bit character value", and
/// the high bit "does not change which low-seven-bit character is emitted".
/// High-bit `,` `.` `/` `0` `1` are outside the raw macro range and print as
/// themselves; the low-bit macro and separator bytes keep their meaning.
#[test]
fn p6town_sign_high_bit_punctuation_prints_its_low_seven_bit_character() {
    let high = |text: &[u8]| text.iter().map(|byte| byte | 0x80).collect::<Vec<u8>>();
    assert_eq!(decode_sign_payload(&high(b"DEATH, 3/02/1.")), "DEATH, 3/02/1.");
    assert_eq!(decode_sign_payload(&high(b"BRITISH'S")), "BRITISH'S");
    assert_eq!(decode_sign_payload(&[0x29, b'A', 0x31, 0x26, 0x27]), "*A*--");
}
