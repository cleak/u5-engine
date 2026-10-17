// Exploratory-playtest shop findings (`playfix-shops`).

fn roster_record_bytes(bytes: &[u8], slot: usize) -> &[u8] {
    let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
    &bytes[record..record + SAVE_CHARACTER_RECORD_LEN]
}

/// Playtest E43. `systems/shops.md` §8.4: Leave moves the companion into the
/// inn registry, "the active roster is compacted, and the party-size byte is
/// decremented", and "a player can leave a companion at an inn, save, reload
/// weeks later, and pick up the companion". `formats/saved-gam.md` §3.1 puts
/// the guest marker at the record's own byte `0x1F`.
///
/// The regression: the guest was written one byte into the next roster record
/// (Shamino saved as `hamino` over the compacted Iolo), so the reload lost Iolo
/// and the pickup garbled the roster further.
#[test]
fn inn_leave_save_reload_pickup_round_trips_every_roster_record() {
    const INN_SCENE: u8 = 20;
    let mut template = saved_game_seed_bytes(0, 0, 15, 15);
    write_shipped_roster_layout(&mut template);
    template[SAVE_PARTY_SIZE_OFFSET] = 3;
    // A distinguishable gender on the departing record, so the test sees
    // whether the byte travels with it.
    template[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN + SAVE_CHARACTER_GENDER_OFFSET] =
        SAVE_GENDER_FEMALE_BYTE;
    let dir = shipped_layout_save_dir(&template);

    let options = load_play_options_from_save(&dir).unwrap();
    let mut state = PlayState::load_scene(&dir, options).unwrap();
    let outcome = state.leave_inn_companion(INN_SCENE, 1, 0).unwrap();
    assert_eq!(outcome.guest.registry_slot, 2);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    let tail = SAVE_CHARACTER_RECORD_LEN - 1;
    assert_eq!(saved[SAVE_PARTY_SIZE_OFFSET], 2);
    assert_eq!(
        roster_record_bytes(&saved, 0),
        roster_record_bytes(&template, 0)
    );
    // The compacted active roster: Iolo moved up, whole.
    assert_eq!(
        &roster_record_bytes(&saved, 1)[..tail],
        &roster_record_bytes(&template, 2)[..tail]
    );
    // Shamino is intact in the vacated record, with the stay counter cleared
    // and the inn's scene in the record's own marker byte.
    let mut lodged = roster_record_bytes(&template, 1).to_vec();
    lodged[SAVE_CHARACTER_STAY_COUNTER_OFFSET] = 0;
    lodged[tail] = INN_SCENE;
    assert_eq!(roster_record_bytes(&saved, 2), &lodged[..]);
    for slot in 3..SAVE_ROSTER_SLOT_COUNT {
        assert_eq!(
            roster_record_bytes(&saved, slot),
            roster_record_bytes(&template, slot),
            "roster record {slot}"
        );
    }

    // Reload: the party is Avatar and Iolo, Shamino is the lodged guest.
    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.inn_registry.len(), 1);
    assert_eq!(options.inn_registry[0].registry_slot, 2);
    assert_eq!(&options.inn_registry[0].name[..7], b"Shamino");
    let mut state = PlayState::load_scene(&dir, options).unwrap();
    assert_eq!(state.party.len(), 2);
    assert_eq!(&state.party_names[1][..4], b"Iolo");

    state.pickup_inn_guest(INN_SCENE, 0, 0).unwrap();
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(saved[SAVE_PARTY_SIZE_OFFSET], 3);
    assert_eq!(
        &roster_record_bytes(&saved, 1)[..tail],
        &roster_record_bytes(&template, 2)[..tail]
    );
    // Pickup clears the returned record's marker to zero.
    lodged[tail] = INN_REGISTRY_EMPTY_MARKER;
    assert_eq!(roster_record_bytes(&saved, 2), &lodged[..]);
    assert!(decode_inn_registry(&saved).is_empty());
    let _ = fs::remove_dir_all(dir);
}

/// Playtest E43, pickup half: a guest parked beyond the next active slot is
/// rotated into it, and the records between move up whole with any guest
/// slots they carry (`systems/shops.md` §8.4 "the registry view is compacted
/// as needed").
#[test]
fn inn_pickup_from_a_far_slot_rotates_records_and_registry_slots() {
    let mut template = saved_game_seed_bytes(0, 0, 15, 15);
    write_shipped_roster_layout(&mut template);
    template[SAVE_PARTY_SIZE_OFFSET] = 1;
    // Mariah (record 3) is lodged at scene 20, Iolo (record 2) at scene 7.
    template[inn_registry_marker_offset(3)] = 20;
    template[inn_registry_marker_offset(2)] = 7;
    let dir = shipped_layout_save_dir(&template);

    let options = load_play_options_from_save(&dir).unwrap();
    let mut state = PlayState::load_scene(&dir, options).unwrap();
    let mariah = state
        .inn_registry
        .iter()
        .position(|guest| guest.registry_slot == 3)
        .unwrap();
    state.pickup_inn_guest(20, mariah, 0).unwrap();
    assert_eq!(state.inn_registry.len(), 1);
    assert_eq!(state.inn_registry[0].registry_slot, 3);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    let tail = SAVE_CHARACTER_RECORD_LEN - 1;
    assert_eq!(saved[SAVE_PARTY_SIZE_OFFSET], 2);
    for (saved_slot, template_slot) in [(1, 3), (2, 1), (3, 2)] {
        let mut expected = roster_record_bytes(&template, template_slot).to_vec();
        if template_slot == 3 {
            expected[SAVE_CHARACTER_STAY_COUNTER_OFFSET] = 25;
        }
        assert_eq!(
            &roster_record_bytes(&saved, saved_slot)[..tail],
            &expected[..tail],
            "saved roster record {saved_slot}"
        );
    }
    assert_eq!(saved[inn_registry_marker_offset(3)], 7);
    let registry = decode_inn_registry(&saved);
    assert_eq!(registry.len(), 1);
    assert_eq!(&registry[0].name[..4], b"Iolo");
    let _ = fs::remove_dir_all(dir);
}

fn arms_buy_list_state(gold: u16) -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = gold;
    state.active_shop = Some(crate::shop_session::ActiveShopSession::ArmsStocked(
        crate::shop_runtime::ArmsShopState::BuyPickItem,
        ArmsShop::IolosBows.stock_table(),
    ));
    state
}

fn arms_state_after(
    state: &mut PlayState,
    keys: &[char],
) -> Option<crate::shop_runtime::ArmsShopState> {
    for &key in keys {
        handle_play_key_input(state, key, "", Path::new("")).unwrap();
    }
    match state.active_shop {
        Some(crate::shop_session::ActiveShopSession::ArmsStocked(arms, _)) => Some(arms),
        _ => None,
    }
}

/// Playtest E45. `systems/shops.md §8.1`: "The short-funds branch ... **exits
/// the whole arms buy flow** ... the next thing the player sees is the
/// farewell"; `§8.A`: short funds "exits the shop flow". The visit used to
/// fall back to the Buy/Sell prompt, so a later `S` opened the sell browser.
#[test]
fn arms_short_funds_ends_the_visit() {
    let mut state = arms_buy_list_state(0);
    let quoted = arms_state_after(&mut state, &['a']);
    assert!(matches!(
        quoted,
        Some(crate::shop_runtime::ArmsShopState::BuyConfirm { .. })
    ));
    assert_eq!(arms_state_after(&mut state, &['Y']), None);
    assert!(state.active_shop.is_none());
    assert_eq!(state.gold, 0);
}

/// Playtest E46. `systems/shops.md §8.1`: at the cap the shop prints
/// `"Thou canst not carry any more!"` and `says <shopkeeper>.`, "waits for one
/// key, and returns to the buy list"; the listing comes back with the
/// post-item prompt and "the next valid item letter starts another quote
/// without another `B`".
#[test]
fn arms_carry_cap_refusal_waits_one_key_then_returns_to_the_buy_list() {
    use crate::shop_runtime::ArmsShopState;
    let mut state = arms_buy_list_state(500);
    let dagger = ArmsShop::IolosBows.stock_table().item_ids[0];
    state.equipment_stock[usize::from(dagger)] = 99;

    assert_eq!(
        arms_state_after(&mut state, &['a', 'Y']),
        Some(ArmsShopState::BuyCapRefusalWait)
    );
    let refusal = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>()
        .join("\n")
        + &state.message;
    assert!(
        refusal.contains("\"Thou canst not carry any more!\""),
        "{refusal:?}"
    );
    assert!(
        !state.message.contains("Anything else"),
        "{:?}",
        state.message
    );

    // The waited key is consumed, whatever it is, and the list comes back.
    assert_eq!(
        arms_state_after(&mut state, &['a']),
        Some(ArmsShopState::BuyPickItem)
    );
    assert!(
        state.message.contains("Anything else"),
        "{:?}",
        state.message
    );
    assert!(state.message.contains("a...Dagger"), "{:?}", state.message);
    // The next letter quotes without another `B`.
    assert!(matches!(
        arms_state_after(&mut state, &['b']),
        Some(ArmsShopState::BuyConfirm { .. })
    ));
}

/// Playtest E54. `systems/shops.md §8.A`, "Arms item quote and confirmation":
/// "Only `Y` and `N` advance. Other keys leave the quote/prompt visible and do
/// not redraw or consume a random draw." An invalid key re-rendered the quote
/// with the fallback `Wouldst thou buy one?` instead of the drawn prompt.
#[test]
fn arms_invalid_key_at_the_buy_confirmation_keeps_the_drawn_prompt() {
    let mut state = arms_buy_list_state(500);
    arms_state_after(&mut state, &['a']);
    let quoted = state.message.clone();
    let rng = state.prng_state;
    arms_state_after(&mut state, &['x']);
    assert_eq!(state.message, quoted);
    assert_eq!(state.prng_state, rng);
    arms_state_after(&mut state, &['x']);
    assert_eq!(state.message, quoted);
}

/// Playtest E56. `systems/shops.md §8.C`: "Print `a.........Keys\n`,
/// `b.........Gems\n`, `c......Torches\n\n`". The rows printed the lowercase
/// commodity names.
#[test]
fn guild_stock_rows_print_capitalised_names() {
    assert_eq!(
        crate::input_dispatch::guild_menu_rows(),
        "a.........Keys\nb.........Gems\nc......Torches\n"
    );
}
