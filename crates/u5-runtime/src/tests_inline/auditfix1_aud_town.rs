// Regressions for the closed-audit town findings (aud-town). Each test names
// the audit rule and the published sentence it rests on.

fn aud_town_attack_fixture(type_byte: u8) -> (std::path::PathBuf, PlayState) {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut npc = g6_npc(9, 6, 5, 0, (6, 5, 0));
    npc.type_byte = type_byte;
    let bystander = g6_npc(10, 12, 12, 0, (12, 12, 0));
    let state = g6_town_with(vec![npc, bystander], 5, 5);
    (dir, state)
}

/// `town-mode.md §10` ("Which targets are refused"): "the only targets
/// refused with `Nothing to attack!` are: a cell with no active object; an
/// object whose type is below `0x40`; a type in the field band `0xE8..0xEF`;
/// and a type in the regalia band `0xB4..0xB7`. Everything else is a valid
/// target."
#[test]
fn aud_town_10_4_attack_refusal_set_is_type_based() {
    for refused in [0x00, 0x0E, 0x3F, 0xB4, 0xB7, 0xE8, 0xEF] {
        assert!(town_attack_target_refused(refused), "0x{refused:02X}");
        assert!(!town_npc_attack_enters_conflict(refused), "0x{refused:02X}");
        assert_eq!(
            town_npc_attack_resolution(refused),
            TownNpcAttackResolution::Refused,
            "0x{refused:02X}"
        );
    }
    for valid in [0x40, 0x50, 0x70, 0x7F, 0x90, 0x94, 0xB8, 0xD8, 0xFC] {
        assert!(town_npc_attack_enters_conflict(valid), "0x{valid:02X}");
    }
    // "The karma penalty and the town alarm apply only to actor types below
    // `0x80`; the lich/death-mage family `0xD8` raises the alarm only; the
    // Shadow Lord actor `0xFC` triggers neither."
    assert!(town_attack_raises_alarm(0x50));
    assert!(town_attack_raises_alarm(0x70));
    assert!(town_attack_raises_alarm(0xD8));
    assert!(!town_attack_raises_alarm(0x90));
    assert!(!town_attack_raises_alarm(0xFC));
}

/// Audit town-mode:10:4. A creature (`0x90` rodent) in a town is a valid
/// conflict target: the attack opens the arena instead of printing
/// `Nothing to attack!`, raises no alarm and, after the fight, records no
/// removal.
#[test]
fn aud_town_10_4_attack_on_a_town_creature_opens_a_conflict() {
    for creature in [0x90u8, 0xD8] {
        let (dir, mut state) = aud_town_attack_fixture(creature);
        let karma = state.moral_standing;
        let bystander_ai = state.npcs[1].schedule;

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "6", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.combat_active, "0x{creature:02X}");
        assert_ne!(state.message, ATTACK_NOTHING_TO_ATTACK_REFUSAL);
        assert_eq!(
            state.pending_town_conflict.map(|pending| pending.type_byte),
            Some(creature)
        );
        assert_eq!(state.moral_standing, karma);
        if creature == 0x90 {
            // No alarm: the bystander's schedule is not rewritten to flight
            // or pursuit.
            assert_eq!(state.npcs[1].schedule, bystander_ai);
        }
        state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);
        assert!(state.removed_town_npc_flags.values().all(|mask| *mask == 0));
        let _ = fs::remove_dir_all(dir);
    }
}

/// Audit town-mode:10:4b. A town attack on the sandalwood box (`0x0E`, type
/// below `0x40`) or on a regalia-band object is refused with `Nothing to
/// attack!`: the object stays, no removal is recorded and no alarm runs.
#[test]
fn aud_town_10_4b_attack_on_the_sandalwood_box_or_regalia_is_refused() {
    for refused in [0x0Eu8, 0xB5] {
        let (dir, mut state) = aud_town_attack_fixture(refused);
        let bystander_ai = state.npcs[1].schedule;
        let objects = state.active_objects.clone();

        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();

        assert!(!state.combat_active);
        assert_eq!(state.message, ATTACK_NOTHING_TO_ATTACK_REFUSAL, "0x{refused:02X}");
        assert_eq!(state.npcs.len(), 2, "0x{refused:02X} target kept");
        assert_eq!(state.active_objects, objects);
        assert!(state.removed_town_npc_flags.values().all(|mask| *mask == 0));
        assert_eq!(state.npcs[1].schedule, bystander_ai);
        let _ = fs::remove_dir_all(dir);
    }
}

/// Audit town-mode:10:7. `npc-schedules.md §3`: the floor test is "a
/// byte-for-byte equality", and §10's refusal set is type-based only, so an
/// ordinary townsperson on a basement floor (`0xFF`) is attacked exactly as
/// on floor 0.
#[test]
fn aud_town_10_7_attack_on_a_basement_townsperson_opens_a_conflict() {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let npc = g6_npc(9, 6, 5, 0, (6, 5, 0xFF));
    let mut state = test_state(g6_open_grid(), 5, 5);
    let Area::Town { scene, .. } = state.area else {
        unreachable!()
    };
    state.area = Area::Town { scene, floor: -1 };
    state.active_objects[0].z = -1;
    state.npcs = vec![npc];
    state.relink_npc_objects();
    assert!(state.town_attack_target_at(-1, 6, 5).is_some());

    handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();

    assert!(state.combat_active);
    assert_ne!(state.message, ATTACK_NOTHING_TO_ATTACK_REFUSAL);
    assert_eq!(
        state.pending_town_conflict.map(|pending| pending.floor),
        Some(-1)
    );
    let _ = fs::remove_dir_all(dir);
}

/// Audit town-mode:14:10. `npc-schedules.md §9.2` / `town-mode.md §14`: the
/// last engagement event the walker produced is handled after each schedule
/// tick, with no floor restriction - a basement AI-4 NPC opens its
/// conversation contact exactly as on floor 0.
#[test]
fn aud_town_14_10_basement_contact_event_is_handled() {
    let dir = debug_game_dir();
    let keeper = NpcSlot {
        slot: 1,
        type_byte: 0x54,
        dialog_id: 0x81,
        schedule: [4, 4, 4, 6, 6, 6, 5, 5, 5, 0xFF, 0xFF, 0xFF, 0, 8, 16, 20],
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
    let Area::Town { scene, .. } = state.area else {
        unreachable!()
    };
    state.area = Area::Town { scene, floor: -1 };
    state.active_objects[0].z = -1;
    state.load_scheduled_npcs(&[idle, keeper]);
    state.advance_npc_schedules();
    assert!(state.pending_town_npc_contact.is_some());
    let outcome = state.apply_town_npc_contact_event(scene, -1, &dir).unwrap();
    assert!(outcome.is_some());
    assert!(state.diagnostics_contain("opens conversation contact"));
    let _ = fs::remove_dir_all(dir);
}

/// Audit town-mode:13:9. `town-mode.md §13` entry order: the install records
/// the host "before its one-at-a-time reject (so a town whose install is
/// rejected still announces its host)", and "in a hideout town, the
/// resident-entry sweep described next runs last". Only no host, the row-`4`
/// guard and the failed capability check skip it - a rejected install (a
/// save loaded with the resident Shadowlord already standing) still sweeps.
#[test]
fn aud_town_13_9_rejected_install_still_runs_the_resident_entry_sweep() {
    let dir = debug_game_dir();
    let scene = Scene::new(6).unwrap();
    fs::write(dir.join("TOWNE.DAT"), open_grid().repeat(16)).unwrap();
    let mut npc = vec![0u8; (scene.block + 1) * 576];
    let base = scene.block * 576;
    for slot in 1..32 {
        let schedule = &mut npc[base + slot * 16..base + slot * 16 + 16];
        schedule[0..3].copy_from_slice(&[0, 0, 0]);
        schedule[3..6].fill(20);
        schedule[6..9].fill(20);
        schedule[9..12].fill(0);
        schedule[12..16].copy_from_slice(&[0, 8, 16, 20]);
        npc[base + 512 + slot] = 0x50;
        npc[base + 544 + slot] = 0x81;
    }
    fs::write(dir.join("TOWNE.NPC"), npc).unwrap();
    fs::write(dir.join("TOWNE.TLK"), [0, 0]).unwrap();
    for rejected in [false, true] {
        let mut options = PlayOptions::default();
        options.target = PlayTarget::Town(scene);
        options.start = Some((5, 5));
        // Hatred hosted in Trinsic.
        options.shadowlord_hideouts = [SHADOWLORD_VANQUISHED, 6, SHADOWLORD_VANQUISHED];
        let mut state = if rejected {
            // Seed a Shadowlord actor so the one-at-a-time check rejects.
            let mut probe = PlayState::load_town_scene(&dir, scene, options.clone()).unwrap();
            assert!(probe.shadowlord_actor_present());
            probe.npcs.iter_mut().for_each(|npc| npc.dialog_id = 0x81);
            let mut objects = probe.active_objects.clone();
            objects.resize(OOL_SLOTS, ActiveObject::empty());
            options.saved_active_objects = Some(objects);
            PlayState::load_town_scene(&dir, scene, options).unwrap()
        } else {
            PlayState::load_town_scene(&dir, scene, options).unwrap()
        };
        assert_eq!(state.resident_shadowlord, Some(SHADOWLORD_HATRED_INDEX));
        assert!(state.message.contains("hatred doth"), "rejected={rejected}");
        assert!(
            state
                .npcs
                .iter()
                .any(|npc| npc.dialog_id == TOWN_NPC_BRUSHOFF_DIALOG_ID),
            "rejected={rejected}: the Hatred sweep wrote no 0xFE"
        );
        if rejected {
            assert!(state.diagnostics_contain("rejected by the one-at-a-time gate"));
        }
        state.flush_message_slot();
    }
    let _ = fs::remove_dir_all(dir);
}
