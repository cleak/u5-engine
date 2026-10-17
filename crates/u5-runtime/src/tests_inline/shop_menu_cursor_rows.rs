// `shops.md §8.B`/`§8.C`: every key-reading service prompt that ends on a
// closing quote and one space keeps the cursor on its own row, with no
// command row opened beneath it (`text-output.md §10.6`).

/// Measured 2026-10-16 (`paws-tavern`, beat `menu`): after the tavern
/// menu's `thy travels?" ` the original keeps the cursor on the prompt row;
/// this engine drew an empty command row under it.
#[test]
fn the_tavern_menu_keeps_its_cursor_on_the_prompt_row() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Greeting {
        tavern: Tavern::TheCatsLair,
    }));
    state.emit_message_line("\n\"Welcome!\"\n\n:");
    state.close_message_row();

    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();

    assert!(
        matches!(
            state.active_shop,
            Some(ActiveShopSession::Tavern(TavernState::Menu { .. }))
        ),
        "{:?}",
        state.active_shop
    );
    assert!(state.message_window_live_row_suppressed());
}

#[test]
fn every_quoted_shop_key_prompt_suppresses_the_command_row() {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::{GuildCommodity, GuildShop, Healer, Herbalist, Inn, Reagent, Stable};

    let tavern = Tavern::TheCatsLair;
    let inn = Inn::TheWayfarerInn;
    let herbalist = Herbalist::TheHerbalist;
    let shop = GuildShop::TheGuild;
    let quoted_prompts = [
        // `thy travels?" ` - the state menu record and its closing quote.
        ActiveShopSession::Tavern(TavernState::Menu {
            tavern,
            continuation_ready: false,
        }),
        // `Thy choice?" `
        ActiveShopSession::Tavern(TavernState::BlueBoarDrinkList {
            tavern,
            continuation_ready: false,
        }),
        // `to drink?" `
        ActiveShopSession::Tavern(TavernState::ConfirmEnoughDrink {
            tavern,
            continuation_ready: false,
        }),
        // `What is the nature of thy need?" `
        ActiveShopSession::Healer(HealerShopState::PickService, Healer::TheHealersMission),
        // `Wilt thou\npay?" `
        ActiveShopSession::Healer(
            HealerShopState::Confirm {
                service: HealerService::Cure,
                slot: 0,
                cost: 10,
            },
            Healer::TheHealersMission,
        ),
        // `aid thee?" `
        ActiveShopSession::Healer(HealerShopState::Continuation, Healer::TheHealersMission),
        // `Wilt thou take\nit?" `, twice.
        ActiveShopSession::Innkeeper(InnkeeperState::ConfirmRest {
            inn,
            base_room_rate: 3,
            total_price: 3,
        }),
        ActiveShopSession::Innkeeper(InnkeeperState::ConfirmLeaveCompanion {
            inn,
            party_index: 1,
            deposit: 3,
        }),
        // `I can do for\nthee?" `
        ActiveShopSession::Innkeeper(InnkeeperState::Continuation { inn }),
        // `Thy interest?" ` and ` Is this thy need?" `
        ActiveShopSession::Reagent(ReagentShopState::PickReagent { herbalist }),
        ActiveShopSession::Reagent(ReagentShopState::ConfirmBundle {
            herbalist,
            reagent: Reagent::Garlic,
            price: 5,
        }),
        // `Thy concern?" ` and `Interested?" `
        ActiveShopSession::Guild(GuildShopState::PickItem { shop }),
        ActiveShopSession::Guild(GuildShopState::ConfirmBundle {
            shop,
            commodity: GuildCommodity::Keys,
            price: 5,
        }),
        // `Deal?" `
        ActiveShopSession::HorseTrader(HorseTraderState::ConfirmPurchase {
            stable: Stable::HorseAndRider,
            price: 100,
        }),
    ];
    for session in quoted_prompts {
        let mut state = test_state(open_grid(), 1, 1);
        state.active_shop = Some(session.clone());
        assert!(
            state.message_window_live_row_suppressed(),
            "{session:?} draws a command row under its quoted prompt"
        );
    }

    // A selector that owns the panel is not this shape, and stays as it was.
    let mut state = test_state(open_grid(), 1, 1);
    state.active_shop = Some(ActiveShopSession::Innkeeper(
        InnkeeperState::PickLeaveCompanion {
            inn,
            deposit: 3,
            highlight: 0,
        },
    ));
    assert!(!state.message_window_live_row_suppressed());
}
