// Exploratory playtest round 2, shops (`shops2`): healer, inn, guild,
// herbalist, shipwright and selector row-state findings.

fn pf2_rows(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.trim_end().to_string())
        .collect()
}

fn pf2_keys(state: &mut PlayState, keys: &[char]) {
    for &key in keys {
        handle_play_key_input(state, key, "", Path::new("")).unwrap();
    }
}

fn pf2_row_index(rows: &[String], want: &str) -> usize {
    rows.iter()
        .rposition(|row| row == want)
        .unwrap_or_else(|| panic!("no row {want:?}: {rows:#?}"))
}

/// A Trinsic healer visit already at its service question, with a party of
/// three: Shamino poisoned, Iolo dead.
fn pf2_healer_state() -> PlayState {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(6).unwrap(),
        floor: 0,
    };
    three_member_party(&mut state);
    state.gold = 3000;
    state.party[1].status = b'P';
    state.party[2].status = b'D';
    state.party[2].hp = 0;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::WoundsOfHonour,
    ));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    // Yes prints the powers line and the service question through the
    // production path, so the question's own tail is what the echo meets.
    pf2_keys(&mut state, &['Y']);
    state
}

/// Playtest F45. `shops.md §8.C` Healers: "C/H/R echo `Curing`, `Healing`,
/// or `Resurrect`". The engine echoed the invented `Resurrecting`.
#[test]
fn pf2_healer_resurrect_echoes_the_published_word() {
    let mut state = pf2_healer_state();
    pf2_keys(&mut state, &['R']);
    let rows = pf2_rows(&state);
    pf2_row_index(&rows, "\"What is the nature of thy need?\" Resurrect");
    assert!(!rows.iter().any(|row| row.contains("Resurrecting")), "{rows:#?}");
}

/// Playtest F47. `shops.md §8.B`: the question ends `need?" ` (closing quote
/// and space). §8.C: the echo is followed by `\n\n"Who needs my aid?" `, and
/// the paid introduction, the untreatable refusal and the continuation each
/// open with `\n\n` - one blank row each.
#[test]
fn pf2_healer_transcript_takes_one_blank_before_each_reply() {
    let mut state = pf2_healer_state();
    pf2_keys(&mut state, &['C']);
    let rows = pf2_rows(&state);
    let echo = pf2_row_index(&rows, "\"What is the nature of thy need?\" Curing");
    assert_eq!(rows[echo + 1..], ["", "\"Who needs my aid?\""], "{rows:#?}");

    pf2_keys(&mut state, &['2', '\r']);
    let rows = pf2_rows(&state);
    let aid = pf2_row_index(&rows, "\"Who needs my aid?\"");
    assert_eq!(rows[aid + 1], "", "{rows:#?}");
    assert!(
        rows[aid + 2].starts_with("\"I can cure thy poisoned body"),
        "{rows:#?}"
    );

    pf2_keys(&mut state, &['Y']);
    let rows = pf2_rows(&state);
    let paid = rows
        .iter()
        .rposition(|row| row.ends_with("pay?\" Yes"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[paid + 1], "", "{rows:#?}");
    assert_eq!(
        rows[paid + 2], "\"Is there any other way in which I may",
        "{rows:#?}"
    );

    // Heal on the dead Iolo: the refusal sits one blank row under the
    // selector's question.
    let mut state = pf2_healer_state();
    pf2_keys(&mut state, &['H', '3', '\r']);
    let rows = pf2_rows(&state);
    let aid = pf2_row_index(&rows, "\"Who needs my aid?\"");
    assert_eq!(rows[aid + 1], "", "{rows:#?}");
    assert_eq!(rows[aid + 2], "\"Thou hast no need of this art!\"", "{rows:#?}");
}

/// A Hotel Brittany visit at its entry greeting, party of three.
fn pf2_inn_state() -> PlayState {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(20).unwrap(),
        floor: 0,
    };
    three_member_party(&mut state);
    state.gold = 3000;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::Greeting {
        inn: Inn::HotelBrittany,
    }));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    pf2_keys(&mut state, &['Y']);
    state
}

/// Playtest F48. `shops.md §8.B`: the inn question ends `night?" `; §8.C:
/// "L and R echo those uppercase letters", the capacity check prints `\n\n`
/// first, the Rest and Leave-rate offers echo a bare `Yes`/`No` "then
/// `\n\n`", and the Leave rate begins "After `\n\n`" - one blank row each.
#[test]
fn pf2_inn_echoes_continue_their_rows_and_replies_take_one_blank() {
    let mut state = pf2_inn_state();
    pf2_keys(&mut state, &['L']);
    let rows = pf2_rows(&state);
    let echo = pf2_row_index(&rows, "night?\" L");
    assert_eq!(rows[echo + 1], "", "{rows:#?}");
    assert!(rows[echo + 2].ends_with(" asks,"), "{rows:#?}");
    assert_eq!(rows[echo + 3..], ["\"Who will", "stay?\""], "{rows:#?}");

    pf2_keys(&mut state, &['2', '\r']);
    let rows = pf2_rows(&state);
    let stay = pf2_row_index(&rows, "stay?\"");
    assert_eq!(rows[stay + 1], "", "{rows:#?}");
    assert_eq!(rows[stay + 2], "\"The rate for", "{rows:#?}");

    pf2_keys(&mut state, &['Y']);
    let rows = pf2_rows(&state);
    let yes = pf2_row_index(&rows, "it?\" Yes");
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert_eq!(rows[yes + 2], "\"I thank thee.\"", "{rows:#?}");

    let mut state = pf2_inn_state();
    pf2_keys(&mut state, &['R']);
    let rows = pf2_rows(&state);
    let echo = pf2_row_index(&rows, "night?\" R");
    assert_eq!(rows[echo + 1], "", "{rows:#?}");
    assert!(rows[echo + 2].starts_with('"'), "{rows:#?}");
    pf2_keys(&mut state, &['Y']);
    let rows = pf2_rows(&state);
    let yes = pf2_row_index(&rows, "it?\" Yes");
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert_eq!(rows[yes + 2], "\"Have a pleasant", "{rows:#?}");
}

fn pf2_guild_state() -> PlayState {
    use crate::shop_runtime::GuildShopState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 3000;
    state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::Greeting {
        shop: crate::shops::GuildShop::TheGuild,
    }));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    pf2_keys(&mut state, &['Y']);
    state
}

fn pf2_herbalist_state() -> PlayState {
    use crate::shop_runtime::ReagentShopState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(1).unwrap(),
        floor: 0,
    };
    state.gold = 3000;
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
        herbalist: Herbalist::TheHerbalist,
    }));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    pf2_keys(&mut state, &['Y']);
    state
}

/// Playtest F46. `shops.md §8.C` Guild: the menu ends `Thy concern?" `, the
/// offer is `\n\n"` + record + `\n\nInterested?" `, the decline
/// `No\n\n"What else, then?\n\n` and the paid reply `\n"Sold!"` after
/// `Yes\n`: a space before each echo and one blank row before each reply.
#[test]
fn pf2_guild_prompts_keep_their_space_and_replies_take_one_blank() {
    let mut state = pf2_guild_state();
    pf2_keys(&mut state, &['a']);
    let rows = pf2_rows(&state);
    let echo = pf2_row_index(&rows, "Thy concern?\" a");
    assert_eq!(rows[echo + 1], "", "{rows:#?}");
    assert!(rows[echo + 2].starts_with('"'), "{rows:#?}");

    pf2_keys(&mut state, &['N']);
    let rows = pf2_rows(&state);
    let no = pf2_row_index(&rows, "Interested?\" No");
    assert_eq!(rows[no + 1], "", "{rows:#?}");
    assert_eq!(rows[no + 2], "\"What else, then?", "{rows:#?}");

    pf2_keys(&mut state, &['a', 'Y']);
    let rows = pf2_rows(&state);
    let yes = pf2_row_index(&rows, "Interested?\" Yes");
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert_eq!(rows[yes + 2], "\"Sold!\"", "{rows:#?}");
}

/// Playtest F46. `shops.md §8.C` Reagents: `\nThy interest?" `, the offer
/// `\n\n"` + record + ` Is this thy need?" `, the decline
/// `No\n\n"What else?\n\n` and the paid `\n"I thank thee!"` after `Yes\n`.
#[test]
fn pf2_herbalist_prompts_keep_their_space_and_replies_take_one_blank() {
    let mut state = pf2_herbalist_state();
    pf2_keys(&mut state, &['A']);
    let rows = pf2_rows(&state);
    let question = pf2_row_index(&rows, "Thy interest?\"");
    assert_eq!(rows[question + 1], "", "{rows:#?}");
    assert!(rows[question + 2].starts_with('"'), "{rows:#?}");

    pf2_keys(&mut state, &['N']);
    let rows = pf2_rows(&state);
    let no = rows
        .iter()
        .rposition(|row| row.ends_with("need?\" No"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[no + 1], "", "{rows:#?}");
    assert_eq!(rows[no + 2], "\"What else?", "{rows:#?}");

    pf2_keys(&mut state, &['A', 'Y']);
    let rows = pf2_rows(&state);
    let yes = rows
        .iter()
        .rposition(|row| row.ends_with("need?\" Yes"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert_eq!(rows[yes + 2], "\"I thank thee!\"", "{rows:#?}");
}

/// Playtest F50. `shops.md §8.C` Shipwrights: "Invalid service keys redraw
/// record `119`, unlike invalid initial Y/N keys, which wait silently." The
/// redraw is byte-identical to the menu already in the slot, so the slot
/// flush took it for no write and nothing was appended.
#[test]
fn pf2_shipwright_invalid_menu_key_redraws_the_menu() {
    // A synthetic record `119`: the engine carries no copy of the shipped
    // menu text (playtest Q16), so the menu needs a `SHOPPE.DAT`.
    let dir = debug_game_dir();
    let shoppe = shoppe_dat_with_records(&[(
        crate::shops::SHOPPE_RECORD_SHIPWRIGHT_MENU,
        b"\n\n\"Menu line.\n\nMenu question?\"".as_slice(),
    )]);
    std::fs::write(dir.join("SHOPPE.DAT"), shoppe).unwrap();
    let menu = shipwright_rows_after(&['Y'], &dir);
    let question = menu.last().cloned().expect("menu rows");
    assert_eq!(question, "Menu question?\"", "{menu:#?}");
    let redrawn = shipwright_rows_after(&['Y', 'x'], &dir);
    assert_eq!(
        redrawn.iter().filter(|row| **row == question).count(),
        2,
        "{redrawn:#?}"
    );
    assert_eq!(redrawn.last(), Some(&question), "{redrawn:#?}");
    // An invalid key at the entry question still waits silently.
    let entry = shipwright_rows_after(&[], &dir);
    assert_eq!(shipwright_rows_after(&['x'], &dir), entry);
    let _ = fs::remove_dir_all(dir);
}
