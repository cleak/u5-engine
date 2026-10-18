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

/// `combat.md §8.1`: the turn banner is "emitted at the start of every
/// keyboard-driven combatant's turn, *before any key is read*". Playtest F35:
/// the command that opened the fight left the round unwalked, so the first
/// banner only appeared together with the next key's echo.
#[test]
fn playtest_f35_the_first_turn_banner_is_printed_before_the_first_key() {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();

    assert!(state.combat_active);
    let actor_slot = state
        .pending_combat_actor_slot
        .expect("a keyboard-driven combatant is already at the prompt");
    let banner = state
        .combat_turn_banner_for_actor(actor_slot)
        .expect("a party actor has a banner");
    let texts = combat2a_entry_texts(&state);
    let conflict = texts
        .iter()
        .position(|text| *text == combat_banner_line())
        .unwrap_or_else(|| panic!("the conflict banner is printed: {texts:?}"));
    assert!(
        texts[conflict..]
            .iter()
            .any(|text| text == banner.trim_matches('\n')),
        "the turn banner follows the conflict banner before any key: {texts:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `combat.md §8.1`: the banner ends with its colon and the turn handler's one
/// line feed, so "whatever the player types next is announced on a fresh
/// row" directly beneath it - measured as `Short Sword:` / ` South`
/// (`qa/paired/combat-same-exit.tsv`). Playtest F36: while the prompt waited
/// its marker sat one row lower than the row its echo then landed on.
#[test]
fn playtest_f36_the_waiting_marker_sits_on_the_row_its_echo_lands_on() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(6, 5);
    // `Iolo, armed with` fills the row and `bare hands:` does not, so the
    // banner's own feed - not a full-row wrap - opens the marker row.
    state.party_names[0] = *b"Iolo\0\0\0\0\0";
    state.ensure_pending_combat_player_turn();

    let waiting = combat_message_window_rows(&state);
    let marker = waiting
        .iter()
        .rposition(|row| row.starts_with('>'))
        .unwrap_or_else(|| panic!("the marker row is drawn: {waiting:?}"));
    assert_eq!(waiting[marker - 1], "bare hands:", "rows: {waiting:?}");

    handle_play_key_input(&mut state, ' ', "", game_dir).unwrap();
    let after = combat_message_window_rows(&state);
    let echo = after
        .iter()
        .position(|row| row == ">Pass")
        .unwrap_or_else(|| panic!("the echo is drawn: {after:?}"));
    assert_eq!(after[echo - 1], "bare hands:", "rows: {after:?}");
}
