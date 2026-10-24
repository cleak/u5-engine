// Regressions for the pass-8 exploratory-playtest findings (p8play, N02-N26).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `commands.md §3`: "`B` Board | \"Acted\" on every path - all four mounts
/// and both refusals - except the final unknown-target fallthrough". The
/// dungeon arm prints `Not here!` (`vehicles.md §4.1`), not the fallthrough,
/// so it spends the turn that drives the dungeon post-action pass (N06).
#[test]
fn playtest_n06_dungeon_board_refusal_reports_acted() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    let clock_before = state.clock;

    handle_play_key_input(&mut state, 'B', "", &dir).unwrap();

    assert_eq!(state.turn, 1, "dungeon Board refusal must report acted");
    let mut expected = clock_before;
    expected.advance_minutes(1);
    assert_eq!(state.clock, expected, "only the loop-head minute");
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §3`: H is not one of the six forwarding routes and neither of
/// its overworld refusals is a published "no action" producer, so `Sails
/// must be lowered!` and `On land or ship!` each run the overworld per-turn
/// epilogue (N11). The town `Only in bed!` refusal is the published
/// exception (`rest-and-camp.md §3`) and is not touched here.
#[test]
fn playtest_n11_overworld_hole_up_refusals_report_acted() {
    let dir = debug_game_dir();
    let mut ship = britannia_state(open_world_grid(), 45, 62);
    ship.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 77,
        skiffs: 0,
    };
    handle_play_key_input(&mut ship, 'H', "", &dir).unwrap();
    assert_eq!(ship.turn, 1, "Sails must be lowered! must report acted");

    let mut skiff = britannia_state(open_world_grid(), 45, 62);
    skiff.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    handle_play_key_input(&mut skiff, 'H', "", &dir).unwrap();
    assert_eq!(skiff.turn, 1, "On land or ship! must report acted");
    assert!(skiff.active_rest.is_none());
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §3`: I does not forward its handler's value, and
/// `lighting.md §8` only says the `None owned!` refusal leaves the light
/// state unchanged, so the refusal still costs the turn in town and on the
/// overworld (N12).
#[test]
fn playtest_n12_ignite_without_torches_reports_acted() {
    let dir = debug_game_dir();
    let mut world = britannia_state(open_world_grid(), 45, 62);
    world.torches = 0;
    handle_play_key_input(&mut world, 'I', "", &dir).unwrap();
    assert_eq!(world.turn, 1, "overworld None owned! must report acted");
    assert_eq!(world.torch_counter, 0);

    let mut town = test_state(open_grid(), 5, 5);
    town.torches = 0;
    handle_play_key_input(&mut town, 'I', "", &dir).unwrap();
    assert_eq!(town.turn, 1, "town None owned! must report acted");
    let _ = fs::remove_dir_all(dir);
}

/// `commands.md §5.5`: the H family's refusal literal is
/// `On_land_or_ship!\n\n`; it completes the `Hole up- ` echo row and its
/// second feed is a producer blank row of its own, ahead of the turn's
/// derived blank - the same shape `lowered!\n\n` already renders (N13).
#[test]
fn playtest_n13_on_land_or_ship_refusal_keeps_its_producer_blank_row() {
    let dir = debug_game_dir();
    let mut skiff = britannia_state(open_world_grid(), 45, 62);
    skiff.player.transport = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    handle_play_key_input(&mut skiff, 'H', "", &dir).unwrap();

    let entries = skiff.message_entries();
    let at = entries
        .iter()
        .position(|entry| entry.text.ends_with(HOLE_UP_ON_LAND_OR_SHIP_REFUSAL))
        .unwrap_or_else(|| panic!("no refusal row in {entries:?}"));
    assert!(
        entries[at].text.starts_with("Hole up-"),
        "the refusal completes the echo row: {:?}",
        entries[at].text
    );
    let next = entries
        .get(at + 1)
        .unwrap_or_else(|| panic!("no row after the refusal in {entries:?}"));
    assert!(next.explicit_blank && next.producer_blank, "{entries:?}");
    let _ = fs::remove_dir_all(dir);
}

/// `shops.md §8.C`: Accept is `Yes\n`, then affordability, and short funds
/// prints record `163` (guild) or `147` (reagents). The engine's `Yes` echo
/// leaves its feed to the next text, so a record that opens with its own
/// line feed must still leave one blank row under `Yes`, as the paid
/// `\n"Sold!"` / `\n"I thank thee!"` replies do (N02). The records here are
/// synthetic stand-ins with the same leading feed.
#[test]
fn playtest_n02_guild_and_herbalist_short_funds_keep_the_blank_row() {
    use crate::shop_runtime::{GuildShopState, ReagentShopState};
    use crate::shop_session::ActiveShopSession;

    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[
        (163, b"\n\"Guild short.\"".as_slice()),
        (147, b"\n\"Herb short.\"".as_slice()),
    ]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
    let rows_of = |state: &PlayState| -> Vec<String> {
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.trim_end().to_string())
            .collect()
    };

    let mut guild = test_state(open_grid(), 1, 1);
    guild.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    guild.gold = 0;
    guild.active_shop = Some(ActiveShopSession::Guild(GuildShopState::Greeting {
        shop: crate::shops::GuildShop::TheGuild,
    }));
    guild.emit_message_line("\n\"Welcome!\" ");
    guild.open_message_row();
    for key in ['Y', 'a', 'Y'] {
        handle_play_key_input(&mut guild, key, "", &dir).unwrap();
    }
    let rows = rows_of(&guild);
    let yes = rows
        .iter()
        .rposition(|row| row.ends_with("Yes"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert_eq!(rows[yes + 2], "\"Guild short.\"", "{rows:#?}");

    let mut herb = test_state(open_grid(), 1, 1);
    herb.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    herb.gold = 0;
    herb.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
        herbalist: Herbalist::TheHerbalist,
    }));
    herb.emit_message_line("\n\"Welcome!\" ");
    herb.open_message_row();
    for key in ['Y', 'A', 'Y'] {
        handle_play_key_input(&mut herb, key, "", &dir).unwrap();
    }
    let rows = rows_of(&herb);
    let yes = rows
        .iter()
        .rposition(|row| row.ends_with("Yes"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert_eq!(rows[yes + 2], "\"Herb short.\"", "{rows:#?}");
    let _ = fs::remove_dir_all(dir);
}
