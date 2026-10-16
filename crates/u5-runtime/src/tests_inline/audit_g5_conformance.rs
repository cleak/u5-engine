// Spec-conformance regressions for the 2026-10-16 conformance audit, cluster
// g5: `systems/blackthorn.md`, `systems/conversation.md`,
// `formats/signs-dat.md` and `formats/look2-dat.md`.
//
// Each test names the audit item and the published sentence it pins.

/// Audit A038 / A039. `blackthorn.md §7` step 11: the rescue hands control to
/// scene seventeen "on logical floor one at local position `(10, 10)`, with
/// the clock spun forward until the hour reads 06:00". "Logical floor" is the
/// signed floor byte (`formats/location-dat.md` Section 4), so floor `+1`.
#[test]
fn audit_a038_a039_rescue_lands_on_floor_one_at_six_in_the_morning() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.clock = GameClock::new(14, 30).unwrap();
    state.torch_counter = 9;
    state.food = 0;

    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    let outcome = state.run_blackthorn_rescue_to_handoff(&dir).unwrap();
    assert!(matches!(
        outcome,
        Some(MoveOutcome::Transition(AreaTransition::EnteredLocation(_)))
    ));

    assert_eq!(state.clock.hour, 6, "§7 step 11: the hour reads 06:00");
    match state.area {
        Area::Town { scene, floor } => {
            assert_eq!(scene.byte, BLACKTHORN_RESCUE_HANDOFF_SCENE);
            assert_eq!(floor, 1, "§7 step 11: logical floor one");
        }
        other => panic!("rescue left the party in {other:?}"),
    }
    // §8.1: the food repair and light clears come after the spin.
    assert_eq!(state.torch_counter, 0);
    assert_eq!(state.food, 63);
    let _ = fs::remove_dir_all(dir);
}

/// Audit A030. `blackthorn.md §3`: inside Lord Blackthorn's Castle, "If the
/// party reaches that helper there with nobody able to act and nobody
/// asleep, the helper returns silently, without the audience or a surrender
/// question." The ordinary-surrender fallback is withdrawn (R442).
#[test]
fn audit_a030_defeated_party_in_the_palace_gets_no_surrender_question() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    assert_eq!(state.party_capability(), PartyCapability::Defeated);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    state.message.clear();

    let outcome = state
        .open_town_arrest(
            TownArrestPrompt {
                scene_byte: BLACKTHORN_CAPTIVE_CELL_SCENE,
                floor: 0,
                npc_slot: 1,
            },
            Some(&dir),
        )
        .unwrap();

    assert_eq!(outcome, MoveOutcome::Used);
    assert!(state.pending_town_arrest.is_none());
    assert!(state.active_blackthorn.is_none());
    assert!(state.message.is_empty(), "{:?}", state.message);
    let _ = fs::remove_dir_all(dir);
}
