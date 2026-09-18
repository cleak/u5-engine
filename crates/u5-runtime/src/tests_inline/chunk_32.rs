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
fn dungeon_sleep_gate_advances_cleanup_but_skips_underfoot_post_action() {
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
    assert_eq!(state.turn, 1);
    assert_eq!((state.clock.hour, state.clock.minute), (8, 1));
    assert_eq!(state.message, PARTY_SLEEP_LINE);
    assert!(matches!(state.area, Area::Dungeon { level: 0, .. }));
    assert_eq!((state.player.x, state.player.y), (1, 1));
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
    assert!(matches!(
        state.area,
        Area::Town { scene, floor: 0 } if scene.byte == BLACKTHORN_RESCUE_HANDOFF_SCENE
    ));
    let _ = fs::remove_dir_all(dir);
}
