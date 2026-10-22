// Regressions for the town-shop exploratory play-test findings (H01..H47).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// H01: `shops.md §8.10` places "a boardable horse object for the party" on
/// an adjacent town cell, and `vehicles.md §4` B-Board "probes the current
/// position" (with a town-only `Nay!` availability check). The party must be
/// able to step onto that horse's cell in a town and board it there.
#[test]
fn playfix4_h01_party_steps_onto_a_town_horse_and_boards_it() {
    let mut state = test_state(open_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 5,
        y: 6,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.step(Direction::South), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (5, 6));
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert!(state.player.transport.is_horse());
}

/// H01: a mounted party still cannot walk onto a second town horse, and an
/// ordinary town object still blocks.
#[test]
fn playfix4_h01_unboardable_town_objects_still_block() {
    let mut state = test_state(open_grid(), 5, 5);
    state.player.transport =
        transport_from_vehicle_object(HORSE_PARKED_FIRST, HORSE_PARKED_FIRST, 0, 0).unwrap();
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 5,
        y: 6,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.step(Direction::South), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (5, 5));
}

fn pf4_rows(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.trim_end().to_string())
        .collect()
}

/// A shop visit whose entry greeting is still in the message slot, as the
/// Talk dispatch leaves it: the greeting's `" ` continuation is printed but
/// the row-open flag has not been stated by anyone.
fn pf4_shop_at_entry(scene: u8, session: crate::shop_session::ActiveShopSession) -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(scene).unwrap(),
        floor: 0,
    };
    state.gold = 3000;
    state.active_shop = Some(session);
    state.message = "\"Welcome, ye?\" ".to_string();
    state.flush_message_slot();
    state.close_message_row();
    state
}

/// H28: `shops.md §8.B` "Every No response echoes `No` without a line feed,
/// then exits through the appropriate closing-bark flow", and `§8.A` renders
/// the nothing-bought bark "including a refused entry prompt". The tavern
/// entry's N and Space are that refusal.
#[test]
fn playfix4_h28_tavern_entry_refusal_takes_the_no_sale_closing_bark() {
    use crate::shop_runtime::{
        ShopTransactionContext, TavernInput, TavernOutcome, TavernState, step_tavern,
    };
    let ctx = ShopTransactionContext {
        party_gold: 100,
        speaker_intelligence: 0,
        world_hour: 12,
        party_size: 1,
        living_party_members: 1,
    };
    for key in [b'N', b'n', b' '] {
        let mut tavern = TavernState::for_tavern(Tavern::TheCatsLair);
        let (mut gold, mut food) = (100, 10);
        assert_eq!(
            step_tavern(&mut tavern, TavernInput::Key(key), ctx, &mut gold, &mut food),
            TavernOutcome::NoSaleExit,
            "{key:#04x}"
        );
        assert_eq!(tavern, TavernState::Exited);
    }
}

/// H28: Space at the tavern entry echoes `No` onto the greeting's row, as N
/// does.
#[test]
fn playfix4_h28_tavern_entry_space_echoes_no_on_the_greeting_row() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;
    for key in ['N', ' '] {
        let mut state = pf4_shop_at_entry(
            22,
            ActiveShopSession::Tavern(TavernState::for_tavern(Tavern::TheCatsLair)),
        );
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        let rows = pf4_rows(&state);
        assert!(
            rows.iter().any(|row| row == "\"Welcome, ye?\" No"),
            "{key:?}: {rows:#?}"
        );
        assert!(state.active_shop.is_none() || state.active_shop.as_ref().unwrap().is_exited());
    }
}

/// H28, end to end against the configured original assets: the refusal is
/// followed by a blank row, the quoted nothing-bought bark and its
/// `says $.` attribution.
#[test]
fn playfix4_h28_tavern_entry_refusal_prints_the_bark_with_assets() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    for key in ['N', ' '] {
        let mut state = pf4_shop_at_entry(
            22,
            ActiveShopSession::Tavern(TavernState::for_tavern(Tavern::TheCatsLair)),
        );
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
        state.flush_message_slot();
        let rows = pf4_rows(&state);
        let no = rows
            .iter()
            .position(|row| row == "\"Welcome, ye?\" No")
            .unwrap_or_else(|| panic!("{key:?}: {rows:#?}"));
        assert_eq!(rows[no + 1], "", "{key:?}: {rows:#?}");
        assert!(rows[no + 2].starts_with('"'), "{key:?}: {rows:#?}");
        assert!(
            rows[no + 2..].iter().any(|row| row == "says Dr. Cat."),
            "{key:?}: {rows:#?}"
        );
    }
}

/// H29: `shops.md §8.B` "A greeting ending in columns 1 through 11 instead
/// receives the single-space continuation", "Space means No where accepted.
/// Every No response echoes `No` without a line feed". At the guild,
/// herbalist and horse trader Space's `No` dropped to a row of its own.
#[test]
fn playfix4_h29_entry_space_no_continues_the_greeting_row() {
    use crate::shop_runtime::{GuildShopState, HorseTraderState, ReagentShopState};
    use crate::shop_session::ActiveShopSession;
    let sessions = [
        (
            22,
            ActiveShopSession::Guild(GuildShopState::Greeting {
                shop: crate::shops::GuildShop::TheGuild,
            }),
        ),
        (
            1,
            ActiveShopSession::Reagent(ReagentShopState::Greeting {
                herbalist: Herbalist::TheHerbalist,
            }),
        ),
        (
            1,
            ActiveShopSession::HorseTrader(HorseTraderState::Greeting {
                stable: Stable::TheStablehouse,
            }),
        ),
    ];
    for (scene, session) in sessions {
        let label = format!("{session:?}");
        let mut state = pf4_shop_at_entry(scene, session);
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
        let rows = pf4_rows(&state);
        assert!(
            rows.iter().any(|row| row == "\"Welcome, ye?\" No"),
            "{label}: {rows:#?}"
        );
        assert!(!rows.iter().any(|row| row == "No"), "{label}: {rows:#?}");
    }
}

/// H30: `view.md §3` opens the shared selector of `inventory.md §4.3`,
/// where digits and arrows only move the indicator and "The message window
/// is untouched apart from the caller's prompt itself." An ignored key or a
/// moved indicator must not print `Who will drink?` a second time.
#[test]
fn playfix4_h30_fountain_selector_keys_do_not_reprint_the_question() {
    let mut grid = open_grid();
    grid[32 + 2] = 0xd8;
    let mut state = test_state(grid, 1, 1);
    three_member_party(&mut state);
    state.player.facing = Direction::East;
    assert_eq!(state.look_facing(), MoveOutcome::Observed);
    state.flush_message_slot();
    assert!(state.active_party_selector.is_some());
    for key in ['x', 'z', '9', '2', INPUT_CODE_SOUTH as char] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        state.flush_message_slot();
        let rows = pf4_rows(&state);
        assert_eq!(
            rows.iter().filter(|row| *row == FOUNTAIN_DRINK_PROMPT).count(),
            1,
            "{key:?}: {rows:#?}"
        );
    }
    assert!(state.active_party_selector.is_some());
}

/// H05: `overworld.md §7.1` step 3 prints `...gp toll!\n\nDost thou pay?`
/// once, "only `Y` and `N` are accepted and every other key is ignored",
/// then echoes the accepted letter onto that question's row. An ignored key
/// used to print the question a second time.
#[test]
fn playfix4_h05_troll_toll_ignored_key_does_not_reprint_the_question() {
    let mut state = britannia_state(open_world_grid(), 2, 1);
    state.gold = 100;
    state.emit_message_line("Caught!\n\nThe trolls demand a 54 gp toll!\n\nDost thou pay?");
    state.active_yes_no_prompt = Some(YesNoPromptSession::new(
        YesNoPromptKind::TrollBridgeToll { toll: 54 },
    ));
    let dir = debug_game_dir();
    for key in ['E', 'x', '\u{1b}'] {
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
        state.flush_message_slot();
        assert!(state.active_yes_no_prompt.is_some(), "{key:?}");
    }
    let rows = pf4_rows(&state);
    assert_eq!(
        rows.iter().filter(|row| row.starts_with(TROLL_BRIDGE_TOLL_QUESTION)).count(),
        1,
        "{rows:#?}"
    );
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    state.flush_message_slot();
    let rows = pf4_rows(&state);
    assert_eq!(
        rows.iter().filter(|row| row.starts_with(TROLL_BRIDGE_TOLL_QUESTION)).count(),
        1,
        "{rows:#?}"
    );
    assert!(rows.iter().any(|row| row == "Dost thou pay?Y"), "{rows:#?}");
    let _ = fs::remove_dir_all(dir);
}

/// H46: `shops.md §8.C` reagent table, "Already carrying 99":
/// `\n\n"Thou canst not carry any more!"\n\n`; consume one command key, then
/// redraw stock. Both line-feed pairs were short by one, so the refusal had
/// no blank row on either side.
#[test]
fn playfix4_h46_herbalist_carry_cap_refusal_has_a_blank_row_each_side() {
    use crate::shop_runtime::ReagentShopState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    state.gold = 3000;
    state.reagents = [99; 8];
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
        herbalist: Herbalist::TheHerbalist,
    }));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    for key in ['Y', 'A', 'Z'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
    }
    state.flush_message_slot();
    let rows = pf4_rows(&state);
    let first = rows
        .iter()
        .position(|row| row == "\"Thou canst not carry any more!\"")
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[first - 1], "", "{rows:#?}");
    assert_eq!(rows[first - 2], "Thy interest?\"", "{rows:#?}");
    assert_eq!(rows[first + 1], "", "{rows:#?}");
    assert!(rows[first + 2].starts_with("A..."), "{rows:#?}");
}

/// H47: `shops.md §8.C` inn table, "Sleep / morning": `Morning!\n`, and "A
/// poisoned member's recovery death adds `\n`, the member name, and
/// ` has\npassed away.\n`" - a blank row between `Morning!` and the name.
#[test]
fn playfix4_h47_inn_rest_recovery_death_takes_a_blank_row_after_morning() {
    let mut state = pf2_inn_state();
    state.party[1].status = b'P';
    state.party[1].hp = 2;
    pf2_keys(&mut state, &['R', 'Y']);
    state.flush_message_slot();
    assert_eq!(state.party[1].status, b'D', "the poisoned member dies overnight");
    let rows = pf4_rows(&state);
    let morning = pf2_row_index(&rows, crate::commands::PAID_INN_REST_MORNING_LINE);
    assert_eq!(
        rows[morning + 1..],
        ["", "SHAMINO has", "passed away."],
        "{rows:#?}"
    );
}
