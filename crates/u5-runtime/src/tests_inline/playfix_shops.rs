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
