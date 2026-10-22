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
