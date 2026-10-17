// `shops.md §8.C` "Shared closing envelope": "Both ordinary farewell pools
// ... use the same sequence: `\n\n"`, the selected record, a line feed only
// when the resulting local cursor column is nonzero, then token-expanded
// `says $.\n`." Every shop kind, not only the shipwright and the arms shop.

/// Press `keys` at `session`, whose prompt `prompt` is the window's last row
/// and was left open by its closing `" ` (or `:`), then return the rows from
/// that prompt row on.
fn closing_envelope_rows_after(
    session: crate::shop_session::ActiveShopSession,
    prompt: &str,
    keys: &[char],
    game_dir: &Path,
    stale_closed_flag: bool,
) -> Vec<String> {
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    state.gold = 500;
    state.active_shop = Some(session);
    state.emit_message_line(format!("\n{prompt}"));
    state.open_message_row();
    // An exit key that echoes nothing must not depend on whatever the last
    // emission left, so those cases leave the flag stale on purpose.
    if stale_closed_flag {
        state.close_message_row();
    }
    for key in keys {
        handle_play_key_input(&mut state, *key, "", game_dir).unwrap();
    }
    state.flush_message_slot();
    let rows: Vec<String> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.trim_end().to_string())
        .collect();
    let start = rows
        .iter()
        .rposition(|row| row.starts_with(prompt.trim_end()))
        .unwrap_or_else(|| panic!("prompt row: {rows:#?}"));
    rows[start..].to_vec()
}

/// The prompt row (with its echo, if any), exactly one blank row, then the
/// quoted farewell and its attribution.
fn assert_closing_envelope(rows: &[String], echo: &str) {
    assert!(rows[0].ends_with(echo), "{rows:#?}");
    assert_eq!(rows[1], "", "one blank row under the prompt: {rows:#?}");
    assert!(rows[2].starts_with('"'), "the quoted record: {rows:#?}");
    assert!(
        rows.iter().skip(2).all(|row| !row.is_empty()),
        "nothing doubles the envelope's blank: {rows:#?}"
    );
}

#[test]
fn every_shop_kinds_ordinary_farewell_opens_with_the_shared_envelope() {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::{GuildShop, Healer, Herbalist, Inn};

    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let inn = Inn::TheWayfarerInn;

    // Guild and herbalist: Space at the stock question exits, echoing
    // nothing (`§8.C`).
    let rows = closing_envelope_rows_after(
        ActiveShopSession::Guild(GuildShopState::PickItem {
            shop: GuildShop::TheGuild,
        }),
        "Thy concern?\" ",
        &[' '],
        &dir,
        true,
    );
    assert_closing_envelope(&rows, "concern?\"");
    let rows = closing_envelope_rows_after(
        ActiveShopSession::Reagent(ReagentShopState::PickReagent {
            herbalist: Herbalist::TheHerbalist,
        }),
        "Thy interest?\" ",
        &[' '],
        &dir,
        true,
    );
    assert_closing_envelope(&rows, "interest?\"");

    // Healer: the continuation's `No` (`§8.C`, "No ends the visit").
    let rows = closing_envelope_rows_after(
        ActiveShopSession::Healer(HealerShopState::Continuation, Healer::TheHealersMission),
        "aid thee?\" ",
        &['N'],
        &dir,
        false,
    );
    assert_closing_envelope(&rows, "aid thee?\" No");

    // Inn: Space at the service question, and the continuation's `No`.
    let rows = closing_envelope_rows_after(
        ActiveShopSession::Innkeeper(InnkeeperState::ServiceMenu { inn }),
        "night?\" ",
        &[' '],
        &dir,
        true,
    );
    assert_closing_envelope(&rows, "night?\"");
    let rows = closing_envelope_rows_after(
        ActiveShopSession::Innkeeper(InnkeeperState::Continuation { inn }),
        "thee?\" ",
        &['N'],
        &dir,
        false,
    );
    assert_closing_envelope(&rows, "thee?\" No");

    // The inn's entry Space: `:No`, then the envelope exactly once - it used
    // to stack the entry path's own `\n\n` on top of it.
    let rows = closing_envelope_rows_after(
        ActiveShopSession::Innkeeper(InnkeeperState::Greeting { inn }),
        ":",
        &[' '],
        &dir,
        false,
    );
    assert_closing_envelope(&rows, ":No");
}

/// The healer's declined fee returns to its continuation question, which
/// ends `aid thee?" `; the dispatcher states that row open, so the next `No`
/// echoes onto it and the farewell's envelope follows (`shops.md §8.C`).
#[test]
fn a_quoted_shop_question_leaves_its_row_open_for_the_next_echo() {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::Healer;

    let mut state = test_state(open_grid(), 1, 1);
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Confirm {
            service: HealerService::Cure,
            slot: 0,
            cost: 10,
        },
        Healer::TheHealersMission,
    ));
    state.emit_message_line("\n\"Wilt thou\npay?\" ");
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    state.flush_message_slot();
    assert!(state.message.ends_with("aid thee?\" "), "{:?}", state.message);
    assert!(state.message_row_open_mid_line);

    // Stale on the way in or not, the dispatcher's statement is what holds.
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    state.flush_message_slot();
    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert_eq!(rows.last(), Some(&"aid thee?\" No"), "{rows:#?}");
}
