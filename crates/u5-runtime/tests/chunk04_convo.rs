//! `systems/conversation.md` §7.6 (table row `0x84` RECRUIT-SPEAKER) —
//! the reserve-roster half of the JOIN mechanism that lives in
//! `play_state_impl/chunk_04.rs::apply_conversation_join_candidate`.

use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::*;

fn roster_record(
    slot: u8,
    name: &[u8; SAVE_CHARACTER_NAME_LEN],
    class_byte: u8,
) -> PartyRosterRecord {
    PartyRosterRecord {
        member: PartyMember {
            slot,
            class_byte,
            status: b'G',
            climb_stat: 10 + slot,
            mana: slot,
            hp: 20 + u16::from(slot),
            max_hp: 30 + u16::from(slot),
            level: 1 + slot,
        },
        name: *name,
        gender: SAVE_GENDER_MALE_BYTE,
        experience: u16::from(slot) * 100,
        stay_counter: slot,
        strength: 15 + slot,
        intelligence: 18 + slot,
        equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
    }
}

/// §7.6: the reserve portion is "scanned from the last slot downwards", so
/// two reserve records carrying the same name resolve to the *highest* slot,
/// not the first one a forward scan reaches.
#[test]
fn recruit_speaker_scans_the_reserve_roster_from_the_last_slot_downwards() {
    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        roster_record(0, b"AVATAR\0\0\0", b'A'),
        roster_record(1, b"GWENNO\0\0\0", b'B'),
        roster_record(2, b"GWENNO\0\0\0", b'C'),
    ];

    let joined = state
        .apply_conversation_join_candidate("Gwenno", 0)
        .expect("a reserve GWENNO record is available");

    assert_eq!(joined, "GWENNO joined.");
    assert_eq!(state.party.len(), 2);
    // The record pulled in is the one from the last reserve slot.
    assert_eq!(state.party[1].class_byte, b'C');
    assert_eq!(state.party_experience[1], 200);
    // The other duplicate stays behind in the reserve.
    assert_eq!(state.party_roster[2].member.class_byte, b'B');
}

/// §7.6: the compare is "case-insensitively, with bit 7 stripped", so an
/// obfuscated roster name still matches the plain speaker name.
#[test]
fn recruit_speaker_name_compare_strips_bit_seven() {
    let obfuscated_gwenno = {
        let mut name = [0u8; SAVE_CHARACTER_NAME_LEN];
        for (index, byte) in b"GWENNO".iter().enumerate() {
            name[index] = byte ^ 0x80;
        }
        name
    };
    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        roster_record(0, b"AVATAR\0\0\0", b'A'),
        roster_record(1, &obfuscated_gwenno, b'B'),
    ];

    let joined = state
        .apply_conversation_join_candidate("Gwenno", 0)
        .expect("bit 7 is stripped from both sides of the compare");

    assert!(joined.ends_with("joined."));
    assert_eq!(state.party.len(), 2);
    assert_eq!(state.party[1].class_byte, b'B');
}

/// §7.6 matches "opening characters", with §6 step 5's space boundary: the
/// speaker's name may stop short of the roster name only at a literal space.
#[test]
fn recruit_speaker_matches_opening_characters_at_a_space_boundary_only() {
    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        roster_record(0, b"AVATAR\0\0\0", b'A'),
        roster_record(1, b"MAX SPUR\0", b'B'),
    ];

    assert_eq!(
        state.apply_conversation_join_candidate("Max", 0).as_deref(),
        Some("MAX SPUR joined.")
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        roster_record(0, b"AVATAR\0\0\0", b'A'),
        roster_record(1, b"GWENNO\0\0\0", b'B'),
    ];

    // No boundary after `GWEN`, so this is not a match.
    assert_eq!(state.apply_conversation_join_candidate("Gwen", 0), None);
    assert_eq!(state.party.len(), 1);
}

/// §7.6: on a match "that record's inn-lodging marker is cleared".
#[test]
fn recruit_speaker_clears_the_joined_records_inn_lodging_marker() {
    let mut state = test_state(open_grid(), 1, 1);
    let lodged = roster_record(1, b"GWENNO\0\0\0", b'B');
    state.party_roster = vec![roster_record(0, b"AVATAR\0\0\0", b'A'), lodged.clone()];
    state.inn_registry = vec![InnGuestRecord {
        scene_marker: 0x11,
        name: *b"GWENNO\0\0\0",
        member: lodged.member,
        strength: lodged.strength,
        intelligence: lodged.intelligence,
        experience: lodged.experience,
        equipment: lodged.equipment,
        stay_counter: 3,
    }];

    state
        .apply_conversation_join_candidate("Gwenno", 0)
        .expect("a reserve GWENNO record is available");

    assert_eq!(state.inn_registry.len(), 1);
    assert_eq!(state.inn_registry[0].scene_marker, 0);
    // Nothing else about the guest slot is rewritten.
    assert_eq!(state.inn_registry[0].stay_counter, 3);
}

/// §7.6: "the engine then removes the NPC from the live scene."
#[test]
fn recruit_speaker_removes_the_speaking_npc_from_the_live_scene() {
    let mut state = test_state(open_grid(), 1, 1);
    state.party_roster = vec![
        roster_record(0, b"AVATAR\0\0\0", b'A'),
        roster_record(1, b"GWENNO\0\0\0", b'B'),
    ];
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x10,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
        NpcSlot {
            slot: 2,
            type_byte: 1,
            dialog_id: 0x11,
            schedule: [0, 0, 0, 5, 5, 5, 4, 4, 4, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert!(state.npcs.iter().any(|npc| npc.slot == 1));
    assert!(state.npcs.iter().any(|npc| npc.slot == 2));
    state.active_conversation_npc_slot = Some(1);

    state
        .apply_conversation_join_candidate("Gwenno", 0)
        .expect("a reserve GWENNO record is available");

    assert_eq!(state.party.len(), 2);
    assert!(
        !state.npcs.iter().any(|npc| npc.slot == 1),
        "the recruited speaker is gone from the live scene"
    );
    // Unrelated NPCs are untouched.
    assert!(state.npcs.iter().any(|npc| npc.slot == 2));
}
