// Regressions for the second combat exploratory-playtest batch (F24-F42).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// The transcript's entry texts, oldest first, with blank rows as `""`.
fn combat2a_entry_texts(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// `main-loop.md §6`: the party-capability check belongs to the three
/// exploration loops and "combat has no such check of its own".
/// `combat.md §8.1`: a prompted sleeper's asleep arm draws `0..255`, wakes
/// below 16 and prints `Zzzzz...` "and the turn ends with no key read", and
/// `§14`: "A wipe that leaves a sleeping member instead simply passes turns
/// until someone wakes or dies". Playtest F24: the exploration gate ran its
/// world sleep pass in the arena instead, forever, without reading a key.
#[test]
fn playtest_f24_a_sleeping_party_in_an_arena_takes_combat_turns_until_it_wakes() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.party[0].status = b'S';
    state.party[0].hp = 999;
    state.party[0].max_hp = 999;
    state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    state.pending_combat_actor_slot = None;
    state.next_combat_actor_slot = 0;
    state.message_transcript.clear();
    assert_eq!(state.party_capability(), PartyCapability::Sleeping);

    let mut ready = false;
    for _ in 0..64 {
        match state.apply_exploration_turn_gate(game_dir).unwrap() {
            ExplorationTurnGateOutcome::Ready { .. } => {
                ready = true;
                break;
            }
            ExplorationTurnGateOutcome::Slept { .. } => {}
            ExplorationTurnGateOutcome::Rescued { .. } => {
                panic!("an arena sleeper is not a total-party defeat")
            }
        }
    }

    assert!(
        ready,
        "the asleep arm's 1/16 wake draw must eventually wake the sleeper"
    );
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_actor_slot, Some(0));
    assert!(!state.combat_actors[0].is_status_disabled());
    let texts = combat2a_entry_texts(&state);
    assert!(
        texts.iter().any(|text| text == COMBAT_ASLEEP_TURN_LINE),
        "the combat asleep arm narrated the sleeping turns: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == PARTY_SLEEP_LINE),
        "the world sleep pass never runs inside an arena: {texts:?}"
    );
}
