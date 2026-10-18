// Regressions for the combat U-Use / Gate Travel exploratory-playtest
// findings (F28-F34).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// A fight under way with two party members, party slot 1 holding the open
/// command prompt (slot 0 is the Avatar), as in `sanct_lor_flash_state`.
fn combat2b_two_member_state() -> PlayState {
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: 1,
        status: b'G',
        climb_stat: 0,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: 1,
    });
    state.party_experience.push(0);
    state.party_strengths.push(20);
    state.party_intelligence.push(20);
    state.party_equipment = default_party_equipment(2);
    state.active_objects[1] = ActiveObject {
        type_byte: 0x81,
        tile: 0x81,
        x: 4,
        y: 5,
        ..ActiveObject::empty()
    };
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 1, 0, 4, 5]);
    state.next_combat_actor_slot = 1;
    state.ensure_pending_combat_player_turn();
    assert_eq!(state.pending_combat_actor_slot, Some(1));
    state
}

/// Open U-Use, walk the picker's highlight onto the row whose request
/// `wanted` accepts, then accept it.
fn combat2b_use_row(
    state: &mut PlayState,
    wanted: impl Fn(UseItemRequest) -> bool,
    game_dir: &std::path::Path,
) {
    handle_play_key_input(state, 'U', "", game_dir).unwrap();
    assert!(state.active_use.is_some(), "the U-Use picker opens");
    let index = state
        .use_item_picker_rows()
        .iter()
        .position(|row| wanted(row.request))
        .expect("the wanted item has a picker row");
    for _ in 0..index {
        handle_play_key_input(state, char::from(INPUT_CODE_SOUTH), "", game_dir).unwrap();
    }
    handle_play_key_input(state, '\r', "", game_dir).unwrap();
}

fn combat2b_rows_text(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// `catalogs/item-list.md §7.1`, scroll 6: "Prints `Resurrection!`.
/// Non-combat scenes prompt for a party member ... Combat-class scenes print
/// `Not here!`." Playtest F29: combat opened `On who:` and refused only
/// after a member had been chosen.
#[test]
fn playtest_f29_the_resurrection_scroll_refuses_in_combat_without_on_who() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat2b_two_member_state();
    state.scroll_stock[SCROLL_RESURRECTION_INDEX] = 1;

    combat2b_use_row(
        &mut state,
        |request| {
            matches!(
                request,
                UseItemRequest::Scroll {
                    index: SCROLL_RESURRECTION_INDEX,
                    ..
                }
            )
        },
        game_dir,
    );

    assert!(state.active_use.is_none(), "no target selector may open");
    // `inventory.md §8`: debited before the branch's scene gate.
    assert_eq!(state.scroll_stock[SCROLL_RESURRECTION_INDEX], 0);
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("On who")),
        "rows: {rows:?}"
    );
    let banner = rows
        .iter()
        .position(|row| row == SCROLL_RESURRECTION_RESULT)
        .unwrap_or_else(|| panic!("the banner prints: {rows:?}"));
    assert_eq!(rows.get(banner + 1).map(String::as_str), Some("Not here!"));
}

/// `catalogs/item-list.md §7.1`, scroll 5: combat "invoke[s] the summon
/// **placement** helper", which "create[s] a hostile actor at the first
/// accepted cell". The caster self-check that stamps the controlled bit is
/// the Kal Xen Corp spell's (`magic.md §8`, `spell-list.md` row 43).
/// Playtest F31: the scroll's Daemon came up at the player's command prompt.
#[test]
fn playtest_f31_the_summon_daemon_scroll_places_a_hostile_daemon() {
    for seed in [0x0001u16, 0x1234, 0xBEEF, 0x7777, 0xFFFF] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.prng_state = seed;
        // Intelligence 1 would make the spell's self-check always fail; the
        // scroll must not consult it either way.
        state.party_intelligence = vec![99; state.party.len()];
        state.scroll_stock[SCROLL_SUMMON_DAEMON_INDEX] = 1;

        let outcome = state.use_scroll(SCROLL_SUMMON_DAEMON_INDEX, None, None);

        let daemons: Vec<_> = state
            .combat_actors
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, actor)| actor.owner_target_class == COMBAT_CLASS_DAEMON)
            .collect();
        if outcome == MoveOutcome::Used {
            assert_eq!(state.message, "Summon Daemon!");
            assert_eq!(daemons.len(), 1, "seed {seed:#06x}");
            for (slot, actor) in daemons {
                assert_eq!(
                    actor.flags & COMBAT_ACTOR_FLAG_CONTROLLED,
                    0,
                    "seed {seed:#06x}: slot {slot} must stay AI-driven"
                );
            }
        } else {
            assert_eq!(state.message, "Summon Daemon!\nFailed!");
            assert!(daemons.is_empty());
        }
    }
}

/// `catalogs/item-list.md §7.1`: the scroll path "prints the scroll-use
/// label and the branch's effect label" before its gate - scroll 4 "Prints
/// `View!`. Combat-class scenes print `Not here!`", scroll 5 "Prints `Summon
/// Daemon!`. ... non-combat scenes print `Not here!`". Playtest F32: both
/// refusals dropped their banner.
#[test]
fn playtest_f32_refused_scrolls_print_their_banner_first() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat2b_two_member_state();
    state.scroll_stock[SCROLL_VIEW_INDEX] = 1;
    combat2b_use_row(
        &mut state,
        |request| {
            matches!(
                request,
                UseItemRequest::Scroll {
                    index: SCROLL_VIEW_INDEX,
                    ..
                }
            )
        },
        game_dir,
    );
    let rows = combat2b_rows_text(&state);
    let banner = rows
        .iter()
        .position(|row| row == "View!")
        .unwrap_or_else(|| panic!("View! prints in combat: {rows:?}"));
    assert_eq!(rows.get(banner + 1).map(String::as_str), Some("Not here!"));

    let mut state = test_state(open_grid(), 5, 5);
    state.scroll_stock[SCROLL_SUMMON_DAEMON_INDEX] = 1;
    handle_play_key_input(&mut state, 'U', "", std::path::Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", std::path::Path::new("")).unwrap();
    let rows = combat2b_rows_text(&state);
    let banner = rows
        .iter()
        .position(|row| row == "Summon Daemon!")
        .unwrap_or_else(|| panic!("Summon Daemon! prints outside combat: {rows:?}"));
    assert_eq!(rows.get(banner + 1).map(String::as_str), Some("Not here!"));
}
