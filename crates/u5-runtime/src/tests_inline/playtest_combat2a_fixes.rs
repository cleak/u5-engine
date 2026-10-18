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

/// `rest-and-camp.md §4`: "the sleeping combatant's `Zzzzz...\n` (five z,
/// three dots)", and `combat.md §8.1`: every banner opens with its own
/// newline. Playtest F39: `Zzzzz...` ran straight into the next banner.
#[test]
fn playtest_f39_a_blank_row_separates_the_asleep_line_from_the_next_banner() {
    let mut state = combat_player_command_state(8, 5);
    state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    state.combat_actors[0].phase_counter = 1;
    state.message_transcript.clear();

    let _ = state.apply_combat_actor_slot_dispatch(0, 30);
    state.open_pending_combat_player_turn(Some(0));

    let texts = combat2a_entry_texts(&state);
    let asleep = texts
        .iter()
        .position(|text| text == COMBAT_ASLEEP_TURN_LINE)
        .unwrap_or_else(|| panic!("the asleep line is printed: {texts:?}"));
    let banner = state.combat_turn_banner_for_actor(0).unwrap();
    assert_eq!(
        texts[asleep + 1..asleep + 3],
        [String::new(), banner.trim_matches('\n').to_string()],
        "transcript: {texts:?}"
    );
}

/// `inventory.md §5` step 2: with no displayable item R-Ready "prints the
/// normal \"nothing to ready\" refusal and exits". Playtest F26: the picker
/// stayed open behind the refusal and swallowed every key up to an Escape.
#[test]
fn playtest_f26_an_empty_handed_ready_closes_the_command() {
    let game_dir = std::path::Path::new(".");
    let mut state = world_state(open_world_grid(), 10, 20);
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    for block in &mut state.party_equipment {
        block.fill(EQUIPMENT_EMPTY_SLOT_SENTINEL);
    }

    handle_play_key_input(&mut state, 'R', "", game_dir).unwrap();
    if state
        .active_ready
        .as_ref()
        .is_some_and(|session| session.selected_party_index.is_none())
    {
        handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    }

    assert!(
        state.active_ready.is_none(),
        "no picker opens behind the refusal"
    );
    assert!(
        state.message.contains("handed!"),
        "the refusal is printed: {:?}",
        state.message
    );
    // The next key is an ordinary command again.
    let before = (state.player.x, state.player.y);
    handle_play_key_input(&mut state, 'd', "", game_dir).unwrap();
    assert_ne!((state.player.x, state.player.y), before);
}

/// `commands.md §5` verb table: R prints `Ready...\n\n`; `combat.md §8`'s R
/// row only binds the selection to the acting combatant. Playtest F40: the
/// arena's `Item: ` prompt sat directly under `Ready...`.
#[test]
fn playtest_f40_combat_ready_keeps_the_blank_row_before_the_item_prompt() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.equipment_stock[0] = 2;
    state.ensure_pending_combat_player_turn();

    handle_play_key_input(&mut state, 'R', "", game_dir).unwrap();

    assert!(state.active_ready.is_some());
    let rows = combat_message_window_rows(&state);
    let echo = rows
        .iter()
        .position(|row| row == ">Ready...")
        .unwrap_or_else(|| panic!("the Ready echo is drawn: {rows:?}"));
    assert_eq!(rows[echo + 1], "", "rows: {rows:?}");
    assert!(rows[echo + 2].starts_with("Item:"), "rows: {rows:?}");
}

/// `combat.md §4.1`: the conflict banner is printed once, and both banners go
/// "through the ordinary wrap-aware string printer"; nothing reprints an
/// earlier row. Playtest F42: an overworld monster's attack during a command
/// re-joined the command's result with the engagement text, and the flush
/// printed `With what? *** CONFLICT ***` under the banner.
#[test]
fn playtest_f42_an_outdoor_engagement_does_not_reprint_the_command_result() {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.climbing_gear = 0;
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

    handle_play_key_input(&mut state, 'K', "", &dir).unwrap();

    let texts = combat2a_entry_texts(&state);
    assert!(
        state.combat_active,
        "the adjacent hostile engaged: {texts:?}"
    );
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.contains("CONFLICT"))
            .count(),
        1,
        "transcript: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text.contains("With what? ")),
        "transcript: {texts:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// `combat.md §14`: "The stored string has one leading and one trailing
/// newline". Playtest F38: after a Kill spell's `Merchant killed!` the
/// `VICTORY!` line followed with no blank row.
#[test]
fn playtest_f38_victory_after_a_cast_kill_keeps_its_leading_blank_row() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(6, 5);
    state.ensure_pending_combat_player_turn();
    // A cast whose result killed the last foe: the result line is in the
    // slot, the foe is gone.
    state.combat_actors[8].mark_dead();
    state.message = "Giant Rat killed!".to_string();
    state.pending_combat_actor_slot = None;

    crate::input_dispatch::finish_combat_cast_actor_action(&mut state, 0, true);

    let texts = combat2a_entry_texts(&state);
    let killed = texts
        .iter()
        .position(|text| text == "Giant Rat killed!")
        .unwrap_or_else(|| panic!("the result is printed: {texts:?}"));
    assert_eq!(
        texts[killed + 1..killed + 3],
        [String::new(), "VICTORY!".to_string()],
        "transcript: {texts:?}"
    );
    let _ = game_dir;
}

/// `combat.md §14`: `BATTLE IS LOST!` "begins with a newline and has no
/// trailing newline", and `text-output.md §10.4`: for a result that does not
/// end in a feed "the next poll's leading feed closes the open row and no
/// blank row appears". Playtest F41: the world prompt took a blank row under
/// the arena's last line.
#[test]
fn playtest_f41_the_world_prompt_follows_the_battle_lost_line_directly() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 0);
    state.combat_actors[0].y = 0;
    state.active_objects[0].y = 0;
    state.pending_combat_actor_slot = Some(0);
    state.next_combat_actor_slot = 1;

    handle_play_key_input(&mut state, 'w', "", game_dir).unwrap();

    assert!(!state.combat_active, "the edge exit ended the fight");
    let last = state.message_entries().last().expect("a transcript");
    assert_eq!(last.text, "BATTLE IS LOST!");
    assert!(last.leaves_row_open, "the loss line leaves its row open");
    let mut log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    let layout = layout_message_window_with_prompt(&log, Some(""), None, false);
    let rows: Vec<&str> = layout.rows.iter().map(|row| row.text.as_str()).collect();
    let lost = rows
        .iter()
        .position(|row| *row == "BATTLE IS LOST!")
        .unwrap_or_else(|| panic!("the loss line is drawn: {rows:?}"));
    assert_eq!(layout.rows[lost + 1].row, layout.rows[lost].row + 1);
    assert!(
        layout.rows[lost + 1].prefixed,
        "the prompt marker: {rows:?}"
    );
}

/// `combat.md §6.3`: "the accepted drop branch leaves a chest object standing
/// on the dead monster's cell ... The G-Get command does not pick these
/// records up", and `containers.md §3`: "a container record answers
/// `Open_it_first!`" without clearing the slot. Playtest F25: the arena's Get
/// deleted the chest and granted nothing.
#[test]
fn playtest_f25_arena_get_refuses_a_drop_chest_and_leaves_it_standing() {
    let mut state = combat_player_command_state(8, 5);
    state.active_objects[9] = ActiveObject {
        type_byte: COMBAT_DEFAULT_DEATH_DROP_TILE,
        tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
        x: 6,
        y: 5,
        phase: STEADY_PHASE,
        aux1: 12,
        ..ActiveObject::empty()
    };

    let outcome = state.get_combat_actor_direction(0, Direction::East);

    assert_eq!(outcome, MoveOutcome::Blocked);
    assert_eq!(state.message, GET_MUST_OPEN_FIRST_REFUSAL);
    assert_eq!(
        state.active_objects[9].type_byte,
        COMBAT_DEFAULT_DEATH_DROP_TILE
    );
    assert_eq!(state.active_objects[9].aux1, 12);
}

/// `combat.md §8`, the `O` row: the shared Open handler "carries no
/// combat-specific branch of its own, so in an arena it behaves as it does on
/// the surface", and `containers.md §4.1` publishes that chest helper's
/// transcript - `Found:` rows or `Chest empty!` - and stamps its staged grants
/// `0x20` "when the scene byte is `0x80` or above". Playtest F25: the arena
/// answered `Nothing to open!` beside a drop chest.
#[test]
fn playtest_f25_arena_open_runs_the_chest_helper_on_a_drop_chest() {
    let mut state = combat_player_command_state(8, 5);
    let turn_before = state.turn;
    state.active_objects[9] = ActiveObject {
        type_byte: COMBAT_DEFAULT_DEATH_DROP_TILE,
        tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
        x: 6,
        y: 5,
        phase: STEADY_PHASE,
        aux1: 30,
        ..ActiveObject::empty()
    };

    let outcome = state.open_combat_actor_direction(0, Direction::East);

    assert_eq!(outcome, MoveOutcome::ContainerOpened);
    assert!(
        state.message.starts_with(CHEST_TRANSCRIPT_FOUND_HEADING)
            || state.message == CHEST_TRANSCRIPT_EMPTY_LINE,
        "{:?}",
        state.message
    );
    assert_ne!(
        state.active_objects[9].type_byte, COMBAT_DEFAULT_DEATH_DROP_TILE,
        "the chest record is consumed"
    );
    let staged: Vec<ActiveObject> = state
        .active_objects
        .iter()
        .copied()
        .filter(|object| !object.is_empty() && (object.x, object.y) == (6, 5))
        .collect();
    assert!(
        staged.iter().all(|object| object.aux3 == 0x20),
        "{staged:?}"
    );
    assert_eq!(state.turn, turn_before, "the arena spends no world turn");
}

/// `containers.md §4`: "A container may be found by Search through the map
/// object table. Search announces only the container's assessed trap state".
/// Playtest F25: the arena's Search printed nothing beside a drop chest.
#[test]
fn playtest_f25_arena_search_assesses_a_drop_chest() {
    let mut state = combat_player_command_state(8, 5);
    state.active_objects[9] = ActiveObject {
        type_byte: COMBAT_DEFAULT_DEATH_DROP_TILE,
        tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
        x: 6,
        y: 5,
        phase: STEADY_PHASE,
        aux1: 0x80 | 12,
        ..ActiveObject::empty()
    };

    let outcome = state.search_combat_actor_direction(0, Direction::East);

    assert_eq!(outcome, MoveOutcome::Searched);
    assert!(
        state.message.starts_with(SEARCH_PREAMBLE),
        "{:?}",
        state.message
    );
    assert_eq!(
        state.active_objects[9].aux1,
        0x80 | 12,
        "Search grants nothing"
    );
}
