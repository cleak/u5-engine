//! The trajectory digest: every field group moves it, nothing else does,
//! and maps hash in key order (design 1.3.12).

use std::hash::{Hash, Hasher};

use super::*;
use crate::LookTable;
use crate::test_fixtures::{open_grid, open_world_grid, test_state, world_state};

/// A town at its command prompt.
fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn group_index(name: &str) -> usize {
    DIGEST_GROUPS
        .iter()
        .position(|group| *group == name)
        .unwrap_or_else(|| panic!("no digest group `{name}`"))
}

/// Applies `change` to a copy of `base` and checks that exactly the named
/// group moved, and with it the state digest.
fn assert_moves_only(group: &str, base: &PlayState, change: impl FnOnce(&mut PlayState)) {
    let mut changed = base.clone();
    change(&mut changed);
    let before = digest_groups(base);
    let after = digest_groups(&changed);
    let index = group_index(group);
    for (i, name) in DIGEST_GROUPS.iter().enumerate() {
        if i == index {
            assert_ne!(before[i], after[i], "group `{name}` did not move");
        } else {
            assert_eq!(before[i], after[i], "group `{name}` moved for `{group}`");
        }
    }
    assert_ne!(
        trajectory_digest(base).state,
        trajectory_digest(&changed).state,
        "state digest did not move for `{group}`"
    );
}

#[test]
fn a_clone_has_the_same_digest() {
    for state in [town(), world_state(open_world_grid(), 10, 20)] {
        assert_eq!(trajectory_digest(&state), trajectory_digest(&state.clone()));
        assert_eq!(digest_groups(&state), digest_groups(&state.clone()));
    }
}

#[test]
fn a_town_and_the_world_differ() {
    assert_ne!(
        trajectory_digest(&town()).state,
        trajectory_digest(&world_state(open_world_grid(), 10, 20)).state
    );
}

#[test]
fn position_moves_the_position_group() {
    let base = town();
    assert_moves_only("position", &base, |s| s.grid[0] ^= 1);
    assert_moves_only("position", &base, |s| s.wind_save_byte ^= 1);
    assert_moves_only("position", &base, |s| s.saved_scene_scratch = Some(3));
}

#[test]
fn objects_move_the_objects_group() {
    let base = town();
    assert_moves_only("objects", &base, |s| s.opened_town_doors.push((1, 0, 2, 3)));
    assert_moves_only("objects", &base, |s| {
        s.removed_town_npc_flags.insert(7, 1);
    });
    assert_moves_only("objects", &base, |s| s.natural_moongate_counter ^= 1);
}

#[test]
fn the_clock_moves_the_clock_group() {
    let base = town();
    assert_moves_only("clock", &base, |s| s.cleanup_previous_hour ^= 1);
    assert_moves_only("clock", &base, |s| {
        s.overworld_input_helper_entry_owed ^= true;
    });
    assert_moves_only("clock", &base, |s| s.time_stop_counter ^= 1);
}

#[test]
fn the_party_moves_the_party_group() {
    let base = town();
    assert_moves_only("party", &base, |s| s.gold ^= 1);
    assert_moves_only("party", &base, |s| s.reagents[0] ^= 1);
    assert_moves_only("party", &base, |s| s.torches ^= 1);
}

#[test]
fn world_memory_moves_the_world_group() {
    let base = town();
    assert_moves_only("world", &base, |s| s.moral_standing ^= 1);
    assert_moves_only("world", &base, |s| {
        s.talk_branch_flags.insert(3, 0x10);
    });
    assert_moves_only("world", &base, |s| s.shrine_ruin_flags[0] ^= 1);
}

#[test]
fn light_moves_the_light_group() {
    let base = town();
    assert_moves_only("light", &base, |s| s.ambient_light ^= 1);
    assert_moves_only("light", &base, |s| s.visibility_grid[0] ^= 1);
    assert_moves_only("light", &base, |s| s.local_light_mask[0] ^= true);
}

#[test]
fn combat_moves_the_combat_group() {
    let base = town();
    assert_moves_only("combat", &base, |s| s.combat_round_counter ^= 1);
    assert_moves_only("combat", &base, |s| s.combat_terrain[0][0] ^= 1);
    assert_moves_only("combat", &base, |s| s.combat_cursor_blink ^= true);
}

#[test]
fn open_commands_move_the_commands_group() {
    let base = town();
    assert_moves_only("commands", &base, |s| s.cast_resources_spent ^= true);
    assert_moves_only("commands", &base, |s| s.search_acting_member = Some(1));
    assert_moves_only("commands", &base, |s| s.pending_look_prompt_turn ^= true);
}

#[test]
fn presentation_moves_the_presentation_group() {
    let base = town();
    assert_moves_only("presentation", &base, |s| s.sound_effect_serial += 1);
    assert_moves_only("presentation", &base, |s| s.cutscene_black_viewport ^= true);
    assert_moves_only("presentation", &base, |s| {
        s.keys_buffered_during_hold.push_back(('A', String::new()));
    });
}

#[test]
fn the_message_window_moves_the_transcript_group() {
    let base = town();
    assert_moves_only("transcript", &base, |s| s.message.push('x'));
    assert_moves_only("transcript", &base, |s| s.diagnostics.push("d".into()));
    assert_moves_only("transcript", &base, |s| s.message_flushed.push('y'));
}

#[test]
fn loaded_tables_move_the_tables_group() {
    let base = town();
    assert_moves_only("tables", &base, |s| {
        s.look_table = Some(LookTable {
            descriptions: vec!["a wall".into()],
        });
    });
    assert_moves_only("tables", &base, |s| {
        s.world_overlays.britannia = Some(Vec::new());
    });
}

#[test]
fn the_absorption_arena_is_hashed_as_a_whole_state() {
    let mut base = town();
    base.endgame_absorption_arena = Some(Box::new(town()));
    assert_moves_only("world", &base, |s| {
        if let Some(arena) = s.endgame_absorption_arena.as_mut() {
            arena.gold ^= 1;
        }
    });
}

#[test]
fn hold_clock_readings_hash_by_their_bits() {
    let base = town();
    assert_moves_only("presentation", &base, |s| {
        s.pending_text_hold = Some(crate::text_hold::PendingTextHold {
            due_at_secs: Some(1.5),
            ..Default::default()
        });
    });
}

#[test]
fn prng_turn_and_transcript_are_reported_apart() {
    let base = town();
    let digest = trajectory_digest(&base);

    let mut changed = base.clone();
    changed.prng_state ^= 1;
    let moved = trajectory_digest(&changed);
    assert_ne!(moved.prng, digest.prng);
    assert_ne!(moved.state, digest.state);
    assert_eq!(moved.turn, digest.turn);
    assert_eq!(moved.transcript, digest.transcript);

    let mut changed = base.clone();
    changed.turn += 1;
    let moved = trajectory_digest(&changed);
    assert_eq!(moved.turn, digest.turn + 1);
    assert_ne!(moved.state, digest.state);
    assert_eq!(moved.prng, digest.prng);

    let mut changed = base.clone();
    changed.message.push_str("Pass");
    let moved = trajectory_digest(&changed);
    assert_ne!(moved.transcript, digest.transcript);
    assert_ne!(moved.state, digest.state);
}

#[test]
fn maps_hash_in_key_order_not_insertion_order() {
    let mut a = town();
    let mut b = town();
    for key in 0..40u8 {
        a.talk_branch_flags.insert(key, u32::from(key) * 3);
        a.removed_town_npc_flags.insert(key, u32::from(key));
    }
    // A different capacity and the reverse insertion order give a different
    // iteration order; the digest must not see it.
    b.talk_branch_flags = HashMap::with_capacity(1024);
    b.removed_town_npc_flags = HashMap::with_capacity(512);
    for key in (0..40u8).rev() {
        b.talk_branch_flags.insert(key, u32::from(key) * 3);
        b.removed_town_npc_flags.insert(key, u32::from(key));
    }
    assert_eq!(trajectory_digest(&a), trajectory_digest(&b));
}

#[test]
fn the_canonical_form_is_little_endian_fnv_1a() {
    let mut h = Canonical::new();
    0x1234_u16.hash(&mut h);
    assert_eq!(h.finish(), fnv1a64(&[0x34, 0x12]));

    // `usize` is widened to 64 bits, so a digest does not depend on the
    // host's pointer width.
    let mut h = Canonical::new();
    7_usize.hash(&mut h);
    assert_eq!(h.finish(), fnv1a64(&7_u64.to_le_bytes()));

    // A slice carries its length first.
    let mut h = Canonical::new();
    [1_u8, 2].as_slice().hash(&mut h);
    let mut expected = 2_u64.to_le_bytes().to_vec();
    expected.extend([1, 2]);
    assert_eq!(h.finish(), fnv1a64(&expected));
}

/// The source scan behind "every field is classified": the destructure has
/// no rest pattern, names as many fields as the struct declares, and the
/// module never falls back on `Debug` text or `DefaultHasher`.
#[test]
fn every_play_state_field_is_classified() {
    let digest = include_str!("../digest.rs");
    let start = digest.find("    let PlayState {").expect("destructure");
    let end = start + digest[start..].find("} = s;").expect("destructure end");
    let pattern = &digest[start..end];
    assert!(!pattern.contains(".."), "the destructure must not use `..`");
    let named = pattern
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && line.ends_with(','))
        .count();

    let declared = include_str!("../../play_state_struct.rs");
    let body_start = declared.find("pub struct PlayState {").expect("PlayState");
    let body_end = body_start + declared[body_start..].find("\n}\n").expect("struct end");
    let fields = declared[body_start..body_end]
        .lines()
        .filter(|line| line.starts_with("    pub ") || line.starts_with("    pub(crate) "))
        .count();
    assert_eq!(named + SKIPPED_FIELDS.len(), fields);

    let code: String = digest
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for banned in ["DefaultHasher", "{:?}"] {
        assert!(!code.contains(banned), "digest.rs uses `{banned}`");
    }
}
