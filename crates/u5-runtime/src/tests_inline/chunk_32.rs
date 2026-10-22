#[test]
fn party_capability_scans_statuses_in_ascending_order() {
    let mut party = vec![default_party()[0]; 4];
    party[0].status = b'D';
    party[1].status = b'S';
    party[2].status = b'P';
    party[3].status = b'G';

    assert_eq!(
        party_capability(&party),
        PartyCapability::CanAct { member_index: 2 }
    );
    party[2].status = b'C';
    party[3].status = b'A';
    assert_eq!(party_capability(&party), PartyCapability::Sleeping);
    party[1].status = b'D';
    assert_eq!(party_capability(&party), PartyCapability::Defeated);
    assert_eq!(party_capability(&[]), PartyCapability::Defeated);
}

#[test]
fn town_sleep_gate_passes_without_input_and_wakes_before_tile_effects() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'S';
    state.repaint_stats_panel();
    state.clock = GameClock::new(8, 0).unwrap();
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut candidate = *seed;
            u5_prng_range_u16(
                &mut candidate,
                0,
                u16::from(TOWN_SLEEP_WAKE_ROLL_MAX),
            ) == 0
        })
        .expect("at least one PRNG seed wakes a town sleeper");

    assert_eq!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Slept { transition: None }
    );
    assert_eq!(state.turn, 1);
    assert_eq!((state.clock.hour, state.clock.minute), (8, 1));
    assert_eq!(state.message, PARTY_SLEEP_LINE);
    assert_eq!(state.party[0].status, b'G');
    // Measured on `asleep-town`, pending `cleak/u5-spec#299` Q23: the town
    // wake leaves the panel showing `S`.
    assert_eq!(state.stats_panel.statuses[0], Some(b'S'));
    assert_eq!(state.active_player, None);

    assert_eq!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Ready { member_index: 0 }
    );
    // The gate reports readiness; it does not select a member.
    // `stats-panel.md §4.1`/§11 keep the active-player selector persistent
    // and moved only by an explicit selection change or the dead/sleeping
    // rule, so an ordinary ready turn leaves it at the none sentinel.
    assert_eq!(state.active_player, None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_sleep_gate_advances_no_time_and_skips_underfoot_post_action() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = DUNGEON_PIT_FALL_TRAP_VISIBLE;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].status = b'S';
    state.clock = GameClock::new(8, 0).unwrap();

    assert_eq!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Slept { transition: None }
    );
    // `dungeon-mode.md §15` (issue #299 Q23): one minute per iteration "not
    // during the all-asleep cycle of Section 4 step 7, which never reaches
    // the advance" (first measured on `qa/paired/asleep-dungeon-clock.tsv`).
    assert_eq!(state.turn, 0);
    assert_eq!((state.clock.hour, state.clock.minute), (8, 0));
    assert_eq!(state.message, PARTY_SLEEP_LINE);
    assert!(matches!(state.area, Area::Dungeon { level: 0, .. }));
    assert_eq!((state.player.x, state.player.y), (1, 1));
    let _ = fs::remove_dir_all(dir);
}

/// `dungeon-mode.md §4` step 6 (issue #299 Q23): after an acted command the
/// post-action helper's first act wakes each Sleeping member with a uniform
/// roll over 0..63 that is below 4, and repaints the panel if anyone woke.
#[test]
fn dungeon_post_action_wake_roll_wakes_a_sleeper_below_four_of_sixty_four() {
    let dir = debug_game_dir();
    let roll_of = |seed: u16| {
        let mut candidate = seed;
        u5_prng_range_u16(&mut candidate, 0, u16::from(DUNGEON_SLEEP_WAKE_ROLL_MAX))
    };
    let waking_seed = (0..=u16::MAX)
        .find(|seed| roll_of(*seed) < u16::from(DUNGEON_SLEEP_WAKE_BELOW))
        .expect("a seed that wakes");
    // A roll the town's 0..15 "zero wakes" rule would also have read as a
    // miss, but which is not below 4 either.
    let staying_seed = (0..=u16::MAX)
        .find(|seed| (4..=63).contains(&roll_of(*seed)))
        .expect("a seed that does not wake");

    for (seed, wakes) in [(waking_seed, true), (staying_seed, false)] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![default_party()[0]; 2];
        state.party[1].status = b'S';
        state.repaint_stats_panel();
        state.prng_state = seed;
        state.turn = 1;
        state
            .apply_dungeon_post_turn_effects_after_turn(0, &dir)
            .unwrap();
        let expected = if wakes { b'G' } else { b'S' };
        assert_eq!(state.party[1].status, expected, "seed {seed:#06x}");
        assert_eq!(state.stats_panel.statuses[1], Some(expected));
    }
    let _ = fs::remove_dir_all(dir);
}

/// F14 (`cleak/u5-engine#47`). `dungeon-mode.md §4` step 7 (issue #299 Q23,
/// `RETRACTIONS.md` R580), first measured on `qa/paired/asleep-dungeon.tsv`:
/// an all-asleep dungeon party is never woken - "a party in which nobody is
/// Good or Poisoned and at least one member is Sleeping never wakes: the
/// original prints the sleep line forever, each on its own row after its
/// marker" - and every pass prints a fresh marker-carrying `Zzzzzz...` row
/// with no command row under it.
#[test]
fn dungeon_all_asleep_passes_never_wake_and_each_prints_its_own_sleep_row() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party = vec![default_party()[0]; 3];
    for member in &mut state.party {
        member.status = b'S';
    }
    state.clock = GameClock::new(8, 0).unwrap();
    assert!(state.party_sleeping_without_input());
    assert!(state.message_window_live_row_suppressed());

    // Long past any plausible turn-count or hour rule: the capture ran
    // hundreds of passes and crossed no wake.
    for _ in 0..300 {
        let revision_before = state.message_transcript_revision;
        assert_eq!(
            state.apply_exploration_turn_gate(&dir).unwrap(),
            ExplorationTurnGateOutcome::Slept { transition: None }
        );
        // The same literal again is still a new row, not an already-flushed
        // slot.
        assert_ne!(state.message_transcript_revision, revision_before);
        // The sleep row, then the fresh row its trailing feed opened.
        let rows = &state.message_transcript[state.message_transcript.len() - 2..];
        assert_eq!(rows[0].text, PARTY_SLEEP_LINE);
        assert!(rows[0].is_command_echo);
        assert!(rows[1].explicit_blank);
    }
    assert!(state.party.iter().all(|member| member.status == b'S'));
    assert_eq!((state.clock.hour, state.clock.minute), (8, 0));
    let window = state
        .message_transcript
        .iter()
        .rev()
        .take(20)
        .filter(|entry| entry.text == PARTY_SLEEP_LINE)
        .count();
    assert_eq!(window, 10);
    assert!(!state.message_row_open_mid_line);
    assert!(state.message_window_live_row_suppressed());
    assert!(state.message_window_cursor_suppressed());
    // The stock window: `>Zzzzzz...` on every other row, a blank between,
    // the bottom row left blank, and no live command row.
    let log = message_log_from_entries(state.message_transcript.iter(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    let layout = layout_message_window(&log, None);
    // Screen rows 11..=23: the newest sleep row sits on 22, so the bottom
    // row 23 is the blank its trailing feed opened.
    let tail: Vec<_> = layout
        .rows
        .iter()
        .filter(|row| row.row >= 18)
        .map(|row| (row.row, row.text.as_str(), row.prefixed))
        .collect();
    assert_eq!(
        tail,
        vec![
            (18, PARTY_SLEEP_LINE, true),
            (20, PARTY_SLEEP_LINE, true),
            (22, PARTY_SLEEP_LINE, true),
        ]
    );

    // Awakened by other means, the loop reads commands again.
    state.party[1].status = b'G';
    assert!(!state.party_sleeping_without_input());
    assert_eq!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Ready { member_index: 1 }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_sleep_gate_consumes_two_minutes_and_runs_the_ordinary_object_tail() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 64, 64);
    state.party[0].status = b'S';
    state.clock = GameClock::new(8, 0).unwrap();
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[1] = ActiveObject {
        type_byte: 0x2c,
        tile: 0x2c,
        x: 200,
        y: 200,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0x44,
        aux3: 0x55,
    };

    assert_eq!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Slept { transition: None }
    );
    assert_eq!(state.turn, 1);
    assert_eq!((state.clock.hour, state.clock.minute), (8, 2));
    assert_eq!(state.active_objects[1].type_byte, 0);
    assert_eq!(state.active_objects[1].phase, STEADY_PHASE);
    assert_eq!(state.active_objects[1].aux3, 0x55);
    assert_eq!(state.message, PARTY_SLEEP_LINE);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_defeat_writer_preserves_memory_and_writes_all_slots_to_current_plane() {
    let dir = debug_game_dir();
    write_empty_ool_mirrors(&dir);
    let mut state = world_state(open_world_grid(), 17, 23);
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[0].aux3 = 0xa5;
    state.active_objects[31] = ActiveObject {
        type_byte: 0x71,
        tile: 0x72,
        x: 73,
        y: 74,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x76,
        aux1: 0x75,
        aux3: 0x77,
    };
    let objects_before = state.active_objects.clone();
    let prng_before = state.prng_state;

    state
        .write_world_defeat_active_object_table(&dir)
        .unwrap();

    assert_eq!(state.active_objects, objects_before);
    assert_eq!(state.prng_state, prng_before);
    assert_eq!(
        fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(),
        encode_active_object_table(&objects_before).unwrap()
    );
    assert_eq!(
        fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
        vec![0; OOL_PLANE_LEN]
    );
    let slot_zero = &fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap()[..OOL_RECORD_LEN];
    assert_eq!(slot_zero[7], 0xa5);
    let slot_31 = &fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap()
        [31 * OOL_RECORD_LEN..OOL_PLANE_LEN];
    assert_eq!(slot_31, &[0x71, 0x72, 73, 74, 0xff, 0x75, 0x76, 0x77]);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_defeat_requires_britannia_gameplay_resources_before_writing() {
    let dir = debug_game_dir();
    write_empty_ool_mirrors(&dir);
    fs::remove_file(dir.join(BRIT_DAT_FILENAME)).unwrap();
    let mut state = world_state(open_world_grid(), 1, 1);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    let objects_before = state.active_objects.clone();

    let error = state
        .write_world_defeat_active_object_table(&dir)
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(state.active_objects, objects_before);
    assert_eq!(
        fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
        vec![0; OOL_PLANE_LEN]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_defeat_gate_persists_the_unmaintained_table_before_rescue() {
    let dir = debug_game_dir();
    write_empty_ool_mirrors(&dir);
    let mut state = world_state(open_world_grid(), 64, 64);
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[1] = ActiveObject {
        type_byte: 0x2c,
        tile: 0x2d,
        x: 200,
        y: 200,
        z: WorldPlane::Britannia.save_floor(),
        phase: 0x43,
        aux1: 0x65,
        aux3: 0x87,
    };
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    let table_before = encode_active_object_table(&state.active_objects).unwrap();

    assert!(matches!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Rescued { .. }
    ));

    assert_eq!(
        fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(),
        table_before
    );
    assert_eq!(
        &table_before[OOL_RECORD_LEN..2 * OOL_RECORD_LEN],
        &[0x2c, 0x2d, 200, 200, 0, 0x65, 0x43, 0x87]
    );
    let _ = fs::remove_dir_all(dir);
}

/// Drive `blackthorn.md §7`'s rescue cinematic from the gate's start to its
/// handoff, the way a shell without a pump does.
fn run_blackthorn_rescue_to_handoff(state: &mut PlayState, dir: &std::path::Path) -> MoveOutcome {
    state
        .run_blackthorn_rescue_to_handoff(dir)
        .unwrap()
        .expect("the cinematic reaches its handoff")
}

/// `blackthorn.md §7`'s "complete print-and-wait order": "Every emission and
/// every BIOS-tick wait of a run, in execution order". The section is explicit
/// that this order is unconditional - it "does not vary with moral standing,
/// roster size, member status, provisions, the incoming scene value or the
/// clock state" - so the whole sequence is one assertion.
///
/// The engine used to print the `KARMA.DAT` verdict and nothing else, because
/// §7's contract list names the beats only by role. The literals are in the
/// section's beat table, which also states that none of them is a data-file
/// record: an implementation "has to carry them".
#[test]
fn blackthorn_rescue_prints_every_published_beat_in_order() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'D';
    state.party[0].hp = 0;

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

    let printed: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .filter(|text| !text.is_empty())
        .collect();
    let position = |needle: &str| {
        printed
            .iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle:?} never printed, got {printed:?}"))
    };

    // Beats 1 to 9, in the order of the section's numbered list. Beat 5 is
    // one literal whose line break falls inside it, so its two rows are
    // checked separately and must stay adjacent in this order.
    let order = [
        "An unending darkness engulfs thee...",
        "Thou hast found refuge.",
        "No evil lives here, only peace and darkness.",
        "But thy slumber is disturbed!",
        "Someone shouts",
        "FORTIS FORTUNA",
        "AVENTARI",
        "There is a peal of thunder!",
        "Strange words are intoned.",
        "Vertigo...",
    ];
    let mut previous = 0;
    for needle in order {
        let index = position(needle);
        assert!(
            index >= previous,
            "{needle:?} is out of order at {index} (previous {previous}) in {printed:?}",
        );
        previous = index;
    }

    // Beat 8a is conditional and this roster is all-Dead, which §7 says is
    // exactly the case that never reaches it: "Beat 8a is unreachable in an
    // unedited shipped save, because the cinematic's own entry condition ...
    // leaves an all-Dead roster."
    assert!(
        !printed.iter().any(|line| line.contains("Not dead!")),
        "an all-Dead roster prints no beat 8a: {printed:?}",
    );
    let _ = fs::remove_dir_all(dir);
}

/// The other side of that condition. A roster that cannot act but is not Dead
/// - `blackthorn.md §7`: "It is reachable for a roster carrying a preserved
/// legacy Ashes status" - prints one beat 8a per such slot, in slot order.
#[test]
fn blackthorn_rescue_prints_one_not_dead_line_per_undead_slot() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    for _ in 0..2 {
        let slot = state.party.len() as u8;
        state.party.push(PartyMember {
            slot,
            class_byte: b'F',
            status: b'G',
            climb_stat: 20,
            mana: 0,
            hp: 40,
            max_hp: 40,
            level: 3,
        });
    }
    for member in &mut state.party {
        member.status = b'A';
        member.hp = 0;
    }
    assert_eq!(state.party_capability(), PartyCapability::Defeated);

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

    let lines = state
        .message_entries()
        .iter()
        .filter(|entry| entry.text.contains("Not dead!"))
        .count();
    assert_eq!(lines, 3, "one line per non-Dead in-party slot");
    // `RETRACTIONS.md` R493: the restore is "neither unconditional nor
    // lossless". The shared revive routine "acts **only** on a slot whose
    // stored status is Dead"; every member here is Ashes, so none of them is
    // revived and all three keep that status - which is the same fact the
    // `Not dead!` count above is measuring. This used to assert that "every
    // member comes back able-bodied", the withdrawn sentence.
    //
    // The healing is not conditional: "the cinematic's own
    // current-equals-maximum copy still heals it to full".
    for member in &state.party {
        assert_eq!(member.status, b'A');
        assert_eq!(member.hp, member.max_hp);
    }
    let _ = fs::remove_dir_all(dir);
}

/// Playtest H48. `blackthorn.md §7`: beat 8 `\n\nStrange words are intoned.`
/// "has **no** trailing line feed, so when beat 8a fires the first `Not
/// dead!` runs straight on from `intoned.` on the same row". With the
/// window's word wrap that reads `are intoned.Not` / `dead!` / `Not dead!`.
#[test]
fn blackthorn_rescue_first_not_dead_runs_on_from_the_intoned_row() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    for _ in 0..2 {
        let slot = state.party.len() as u8;
        state.party.push(PartyMember {
            slot,
            class_byte: b'F',
            status: b'G',
            climb_stat: 20,
            mana: 0,
            hp: 40,
            max_hp: 40,
            level: 3,
        });
    }
    // Slot 0 Ashes, slot 1 Dead, slot 2 Ashes: two beat-8a lines, with the
    // Dead slot's silent revive between beat 8 and the first of them.
    for (member, status) in state.party.iter_mut().zip([b'A', b'D', b'A']) {
        member.status = status;
        member.hp = 0;
    }
    assert_eq!(state.party_capability(), PartyCapability::Defeated);

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

    let log = message_log_from_entries(state.message_entries(), |text| Some(text.to_string()));
    let rows: Vec<&str> = log.lines().iter().map(|line| line.text.as_str()).collect();
    let intoned = rows
        .iter()
        .position(|row| row.contains("intoned."))
        .unwrap_or_else(|| panic!("no beat 8 row in {rows:?}"));
    assert_eq!(
        rows.get(intoned..intoned + 3),
        Some(["are intoned.Not", "dead!", "Not dead!"].as_slice()),
        "rows: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn defeated_gate_enters_the_same_rescue_from_all_exploration_modes() {
    let dir = debug_game_dir();
    let mut states = vec![
        test_state(open_grid(), 1, 1),
        world_state(open_world_grid(), 1, 1),
        dungeon_state(open_dungeon_record(), 0, 1, 1),
    ];

    for state in &mut states {
        state.party[0].status = b'D';
        state.party[0].hp = 0;
        assert!(matches!(
            state.apply_exploration_turn_gate(&dir).unwrap(),
            ExplorationTurnGateOutcome::Rescued { transition: None }
        ));
        assert!(matches!(
            run_blackthorn_rescue_to_handoff(state, &dir),
            MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
                if scene.byte == BLACKTHORN_RESCUE_HANDOFF_SCENE
        ));
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, state.party[0].max_hp);
        assert!(matches!(
            state.apply_exploration_turn_gate(&dir).unwrap(),
            ExplorationTurnGateOutcome::Ready { member_index: 0 }
        ));
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stonegate_scripted_death_reaches_rescue_on_the_next_gate() {
    let dir = debug_game_dir();
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town { scene, floor: 0 };
    state.apply_stonegate_trapdoor_script(0);

    assert_eq!(state.party_capability(), PartyCapability::Defeated);
    assert!(matches!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Rescued { transition: None }
    ));
    assert!(matches!(
        run_blackthorn_rescue_to_handoff(&mut state, &dir),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
            if scene.byte == BLACKTHORN_RESCUE_HANDOFF_SCENE
    ));
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(state.party[0].hp, state.party[0].max_hp);
    // `blackthorn.md §7` step 11: "on logical floor one" (audit A039).
    assert!(matches!(
        state.area,
        Area::Town { scene, floor: 1 } if scene.byte == BLACKTHORN_RESCUE_HANDOFF_SCENE
    ));
    let _ = fs::remove_dir_all(dir);
}

/// `blackthorn.md §7.1`: the two Guardian reveals precede the thunder line and
/// the crowned figure's reveal follows the paired flash, ahead of the verdict -
/// so the reveal hold is split across the two waits rather than paid before
/// the thunder. Measured 2026-10-16 (`stonegate-rescue-pacing`, beat `t44`,
/// `offset-4`).
#[test]
fn blackthorn_rescue_splits_the_cell_reveal_hold_around_the_thunder() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    let mut beats: Vec<crate::narration::NarrationBeat> = Vec::new();
    for _ in 0..64 {
        beats.extend(state.staged_narration.beats().cloned());
        if beats.iter().any(|beat| beat.text.contains("peal of thunder")) {
            break;
        }
        state.step_blackthorn_rescue(&dir, false).unwrap();
    }
    let delay = |needle: &str| {
        beats
            .iter()
            .find(|beat| beat.text.contains(needle))
            .unwrap_or_else(|| panic!("{needle:?} never staged"))
            .delay_secs
    };
    let reveal = crate::narration::bios_ticks_secs(crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL);
    let thunder = delay("peal of thunder");
    assert!(
        (thunder
            - crate::narration::bios_ticks_secs(
                crate::blackthorn::BLACKTHORN_RESCUE_WAIT_BEFORE_THUNDER + 2 * crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL
            ))
        .abs()
            < 1e-9
    );
    let verdict = beats
        .iter()
        .skip_while(|beat| !beat.text.contains("peal of thunder"))
        .nth(1)
        .expect("the verdict follows the thunder")
        .delay_secs;
    assert!(
        (verdict
            - crate::narration::bios_ticks_secs(crate::blackthorn::BLACKTHORN_RESCUE_WAIT_PAIRED_FLASH)
            - reveal)
            .abs()
            < 1e-6
    );
    assert_eq!(3 * crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL, crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEALS);
    let _ = fs::remove_dir_all(dir);
}

/// `blackthorn.md §6.2`/`§7.1` (issue #298 Q44): at a fixed 3000-cycle emulator
/// setting slumber to shout measured 11.65 s and the three reveals 0.90 s,
/// 0.935 s and 0.935 s. The holds are those published spans, each within one
/// BIOS tick, and the paired flash lies inside the published 1.67 to 1.72 s.
#[test]
fn blackthorn_rescue_holds_take_the_published_emulator_spans() {
    let tick = crate::narration::BIOS_TICK_SECS;
    let reveal = crate::narration::bios_ticks_secs(crate::blackthorn::BLACKTHORN_RESCUE_WAIT_CELL_REVEAL);
    for published in [0.90, 0.935] {
        assert!((reveal - published).abs() < tick, "reveal hold {reveal} s against {published} s");
    }
    let slumber_to_shout =
        crate::narration::bios_ticks_secs(crate::blackthorn::BLACKTHORN_RESCUE_WAIT_ENVELOPE_SEQUENCE);
    assert!((slumber_to_shout - 11.65).abs() < tick, "slumber-to-shout hold {slumber_to_shout} s");
    let flashes = crate::narration::bios_ticks_secs(crate::blackthorn::BLACKTHORN_RESCUE_WAIT_PAIRED_FLASH);
    assert!((1.67..=1.72).contains(&flashes), "paired flash hold {flashes} s");
}

/// Playtest E59: `blackthorn.md §7` "Party restoration" - for each slot the
/// cinematic revives the member, copies the maximum into current hit points
/// "and repaints the whole stats panel". The panel used to keep showing the
/// dead roster row after the handoff.
#[test]
fn blackthorn_rescue_restoration_repaints_the_stats_panel_per_slot() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.repaint_stats_panel();
    let dead_row = state.displayed_stats_panel().rows[0].clone();

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

    assert_eq!(state.party[0].status, b'G');
    let live = crate::stats_panel::stats_panel_snapshot_of(&state);
    assert_ne!(state.displayed_stats_panel().rows[0], dead_row);
    assert_eq!(state.displayed_stats_panel().rows, live.rows);
    assert_eq!(state.displayed_stats_panel().statuses, live.statuses);
}

/// Playtest E62: `karma.md §7.1` and `commands.md §11.3` - the restoration
/// matches its virtue and mantras through the shared matcher, restart quirk
/// included, so an answer that starts inside a failed partial match of the
/// key fails ("against the shrine key `hone`, `hhone` and `hohone` fail").
#[test]
fn ruined_shrine_restoration_applies_the_shared_matchers_restart_quirk() {
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    for (virtue, first_mantra, restores) in [
        ("hhonesty", "ahm", false),
        ("honesty", "aahm", false),
        ("I seek honesty", "say ahm", true),
    ] {
        let mut world = world_state(open_world_grid(), shrine_x + 1, shrine_y);
        world.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        let shrine_index = world_cell_index(shrine_x, shrine_y);
        world.grid[shrine_index] = WORLD_RUINED_SHRINE_TILE;
        world.shrine_ruin_flags[0] = 0x81;

        assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
        world.step_active_shrine_restoration('\r', virtue);
        world.step_active_shrine_restoration('\r', first_mantra);
        world.step_active_shrine_restoration('\r', "ahm");
        assert_eq!(
            world.step_active_shrine_restoration('\r', "ahm"),
            Some(MoveOutcome::Used)
        );

        assert_eq!(
            world.grid[shrine_index] == WORLD_SHRINE_TILE,
            restores,
            "{virtue:?} / {first_mantra:?}"
        );
        assert_eq!(world.shrine_ruin_flags[0] == 0x01, restores);
    }
}

/// Playtest H42. `time.md §5`, provision counter already zero: "The party
/// receives the starvation warning, and then each party slot ... takes an
/// independently rolled uniform `1..8`" - the warning belongs to the same
/// upkeep pass as the damage. `main-loop.md §6`: the asleep arm emits the
/// prompt pair and the sleep line, then passes the turn, so the warning
/// prints on the slept-through turn that crosses the hour, on its own row
/// under `Zzzzzz...`. The town arm used to hold it until after the next
/// awake command.
#[test]
fn all_asleep_turn_prints_the_starvation_warning_under_its_own_sleep_row() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'S';
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.food = 0;
    state.clock = GameClock::new(8, 59).unwrap();
    state.cleanup_previous_hour = 8;

    assert!(matches!(
        state.apply_exploration_turn_gate(&dir).unwrap(),
        ExplorationTurnGateOutcome::Slept { transition: None }
    ));
    assert_eq!(state.clock.hour, 9);
    assert!(state.party[0].hp < 60, "the crossing starves the sleeper");
    assert!(state.pending_hourly_status_message.is_none());

    let log = message_log_from_entries(state.message_entries(), |text| Some(text.to_string()));
    let rows: Vec<&str> = log.lines().iter().map(|line| line.text.as_str()).collect();
    let sleep = rows
        .iter()
        .rposition(|row| *row == PARTY_SLEEP_LINE)
        .unwrap_or_else(|| panic!("no sleep row in {rows:?}"));
    assert_eq!(
        rows.get(sleep + 1..),
        Some([STARVATION_WARNING, ""].as_slice()),
        "rows: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}
