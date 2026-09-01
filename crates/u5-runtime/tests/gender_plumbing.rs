//! The gender byte of the thirty-two-byte character record, from
//! `SAVED.GAM` through the runtime roster to the one published consumer.
//!
//! `formats/saved-gam.md` §3.1: field offset `0x09`, width one byte —
//! "Gender. `0x0B` for male, `0x0C` for female. Not ASCII; the values are
//! private to the engine." §3.2 pins the order as gender-then-class-then-
//! status against third-party references that swap gender and class.
//!
//! `systems/shops.md` §8.1: a successful arms purchase "prints the fixed
//! success line `Sold!`. ... It then prints the post-item prompt
//! `"Anything else,` followed by `milady?` when the speaking member's gender
//! field is the female value and `sir?` otherwise, or `then?` when no
//! transaction has completed in this visit."
//!
//! §8.A's resident-literal table contrasts this with the shipwright tail,
//! whose "gender test in this branch compares a field against a value that
//! field never holds, so the feminine form is unreachable and the shipped
//! build always prints the masculine form ... the arms tail, by contrast,
//! **selects correctly**." So `milady?` has to be reachable on this path.

use std::path::Path;

use u5_runtime::shop_runtime::ArmsShopState;
use u5_runtime::shop_session::ActiveShopSession;
use u5_runtime::shops::ArmsStockTable;
use u5_runtime::test_fixtures::{open_grid, saved_game_seed_bytes, test_state};
use u5_runtime::*;

/// Drive a whole arms buy — `B`, menu letter, `Y` — and return the message
/// left on screen by the successful purchase.
fn arms_purchase_message(gender: u8) -> String {
    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 1000;
    state.party_intelligence[0] = 10;
    for record in state.party_roster.iter_mut() {
        record.gender = gender;
    }
    state.active_shop = Some(ActiveShopSession::ArmsStocked(
        ArmsShopState::Greeting,
        ArmsStockTable::new([23, 24, 30, 0, 0, 0, 0, 0], 3),
    ));

    handle_play_key_input(&mut state, 'B', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'b', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(
        state.message.contains("Sold!"),
        "the buy did not complete: {:?}",
        state.message
    );
    state.message
}

/// `systems/shops.md` §8.1 / §8.A: the feminine form is reachable on the
/// arms tail. This drives the production render arm, not the formatter, so
/// it fails if the speaker's gender is not actually read from the roster.
#[test]
fn arms_post_item_prompt_reaches_milady_for_a_female_speaker() {
    let message = arms_purchase_message(SAVE_GENDER_FEMALE_BYTE);
    assert!(
        message.contains("Anything else, milady?"),
        "female speaker must reach the feminine tail: {message:?}"
    );
}

/// The same drive with the male byte takes the spec's explicit "otherwise"
/// branch, so the change cannot have simply inverted the test.
#[test]
fn arms_post_item_prompt_stays_sir_for_a_male_speaker() {
    let message = arms_purchase_message(SAVE_GENDER_MALE_BYTE);
    assert!(
        message.contains("Anything else, sir?"),
        "male speaker must take the otherwise branch: {message:?}"
    );
    assert!(!message.contains("milady"));
}

/// `systems/shops.md` §8.1 tests the field "is the female value", an
/// equality test — so a byte that is neither published value (an unset or
/// externally edited slot) falls to the masculine branch rather than being
/// treated as "not male, therefore female".
#[test]
fn an_unpublished_gender_byte_takes_the_otherwise_branch() {
    for byte in [0x00u8, b'M', b'F', 0xFF] {
        let message = arms_purchase_message(byte);
        assert!(
            message.contains("Anything else, sir?"),
            "byte {byte:#04x} must take the otherwise branch: {message:?}"
        );
    }
}

/// `formats/saved-gam.md` §3.1: the gender byte is at record offset `0x09`,
/// between the nine-byte name and the ASCII class letter. Decoding must read
/// that byte and no neighbour of it.
#[test]
fn roster_decode_reads_the_gender_byte_at_record_offset_nine() {
    let mut bytes = saved_game_seed_bytes(1, 0, 1, 1);
    for slot in 0..SAVE_ROSTER_SLOT_COUNT {
        let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
        bytes[record + SAVE_CHARACTER_GENDER_OFFSET] = if slot % 2 == 0 {
            SAVE_GENDER_FEMALE_BYTE
        } else {
            SAVE_GENDER_MALE_BYTE
        };
        // The class letter next door must survive untouched: §3.2 exists
        // because external references swap these two positions.
        bytes[record + SAVE_CHARACTER_CLASS_OFFSET] = b'F';
    }

    let roster = save_load::decode_party_roster(&bytes);
    for (slot, record) in roster.iter().enumerate() {
        let expected = if slot % 2 == 0 {
            SAVE_GENDER_FEMALE_BYTE
        } else {
            SAVE_GENDER_MALE_BYTE
        };
        assert_eq!(record.gender, expected, "slot {slot}");
        assert_eq!(record.is_female(), slot % 2 == 0, "slot {slot}");
        assert_eq!(record.member.class_byte, b'F', "slot {slot}");
    }
}

/// The offset is anchored, not guessed: gender sits immediately before the
/// class byte, which sits immediately before status.
#[test]
fn gender_class_and_status_are_adjacent_in_that_order() {
    assert_eq!(SAVE_CHARACTER_GENDER_OFFSET, SAVE_CHARACTER_NAME_LEN);
    assert_eq!(
        SAVE_CHARACTER_CLASS_OFFSET,
        SAVE_CHARACTER_GENDER_OFFSET + 1
    );
    assert_eq!(
        SAVE_CHARACTER_STATUS_OFFSET,
        SAVE_CHARACTER_CLASS_OFFSET + 1
    );
    assert_eq!(SAVE_GENDER_MALE_BYTE, 0x0B);
    assert_eq!(SAVE_GENDER_FEMALE_BYTE, 0x0C);
}

/// The gender byte has no parallel active-party vector, so it is carried by
/// member identity. `party_roster` is not reshuffled by the inn's
/// leave/pick-up helpers or by New Order, so a slot-indexed read would hand
/// the record parked at index 1 to whoever the active party now holds there.
#[test]
fn the_carried_gender_follows_the_member_not_the_slot_index() {
    let mut name_a = [0u8; SAVE_CHARACTER_NAME_LEN];
    name_a[..5].copy_from_slice(b"Julia");
    let mut name_b = [0u8; SAVE_CHARACTER_NAME_LEN];
    name_b[..5].copy_from_slice(b"Iolo\0");

    let mut roster = party::default_party_roster(1);
    let mut female = roster[0].clone();
    female.name = name_a;
    female.gender = SAVE_GENDER_FEMALE_BYTE;
    let mut male = roster[0].clone();
    male.name = name_b;
    male.gender = SAVE_GENDER_MALE_BYTE;
    roster = vec![female, male];

    // Slot 1 of the active party holds Julia even though roster index 1 is
    // Iolo's record.
    assert_eq!(
        party::party_roster_carried_gender(&roster, 1, Some(&name_a)),
        SAVE_GENDER_FEMALE_BYTE
    );
    assert_eq!(
        party::party_roster_carried_gender(&roster, 0, Some(&name_b)),
        SAVE_GENDER_MALE_BYTE
    );
    // An all-zero (unnamed) slot has no identity to match, so it falls back
    // to the slot index.
    assert_eq!(
        party::party_roster_carried_gender(&roster, 0, Some(&[0; SAVE_CHARACTER_NAME_LEN])),
        SAVE_GENDER_FEMALE_BYTE
    );
    // And a slot past the end of an empty roster takes the synthesised
    // default rather than panicking.
    assert_eq!(
        party::party_roster_carried_gender(&[], 3, Some(&name_a)),
        SAVE_GENDER_MALE_BYTE
    );
}

/// `systems/chargen.md` §4 writes the chosen gender into the avatar record
/// at "the field one byte beyond the name", using the same `0x0B`/`0x0C`
/// codes. That is the byte `decode_party_roster` reads back, so chargen
/// already feeds this roster field and is not a second source of truth.
#[test]
fn chargen_gender_choice_round_trips_into_the_decoded_roster() {
    for male in [true, false] {
        let mut bytes = saved_game_seed_bytes(1, 0, 1, 1);
        let stats = chargen::ChargenStats {
            strength: 20,
            dexterity: 20,
            intelligence: 20,
        };
        chargen::apply_chargen_to_save(&mut bytes, b"Jaana", male, stats).unwrap();

        let roster = save_load::decode_party_roster(&bytes);
        assert_eq!(
            roster[0].gender,
            if male {
                SAVE_GENDER_MALE_BYTE
            } else {
                SAVE_GENDER_FEMALE_BYTE
            }
        );
        assert_eq!(roster[0].is_female(), !male);
        // §3.2: the class byte next door is preserved by chargen, so the
        // read-back cannot be picking up the wrong field.
        assert_eq!(roster[0].member.class_byte, b'A');
    }
}

/// `formats/saved-gam.md` §3.1 publishes a fixed-width name field and
/// promises nothing about uniqueness, so two roster records can carry the
/// same name. The record standing at the active slot must win over a
/// same-named record elsewhere in the roster.
#[test]
fn a_duplicate_name_does_not_beat_the_record_at_the_active_slot() {
    let mut shared = [0u8; SAVE_CHARACTER_NAME_LEN];
    shared[..5].copy_from_slice(b"Julia");

    let base = party::default_party_roster(1).remove(0);
    let mut decoy = base.clone();
    decoy.name = shared;
    decoy.gender = SAVE_GENDER_MALE_BYTE;
    let mut occupant = base;
    occupant.name = shared;
    occupant.gender = SAVE_GENDER_FEMALE_BYTE;
    let roster = vec![decoy, occupant];

    assert_eq!(
        party::party_roster_carried_gender(&roster, 1, Some(&shared)),
        SAVE_GENDER_FEMALE_BYTE,
        "the record at the active slot must win over an earlier name twin"
    );
    assert_eq!(
        party::party_roster_carried_gender(&roster, 0, Some(&shared)),
        SAVE_GENDER_MALE_BYTE
    );
}
