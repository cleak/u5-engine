//! `save_command_available` against the dispatcher on synthetic states, and
//! `save_image_summary` on synthetic images (plan M4-10). The route-smoke
//! cross-check is its own test in u5-tui (`tests/save_predicate.rs`).

use std::path::Path;

use crate::test_fixtures::{
    dungeon_state, open_dungeon_record, open_grid, open_world_grid, saved_game_seed_bytes,
    test_state, world_state,
};
use crate::z_stats::{YesNoPromptKind, YesNoPromptSession};
use crate::{
    Area, NATURAL_MOONGATE_TERRAIN_TILE, PlayState, PlayTarget, SAVE_AVATAR_NAME_OFFSET,
    SAVE_PARTY_SIZE_OFFSET, SAVE_SCENE_OFFSET, SAVE_Z_OFFSET, SAVED_GAM_LEN, handle_play_key_input,
    world_cell_index,
};

use super::save_image_summary;

/// Whether `Q` typed into a clone of `state` opens the save prompt.
fn q_opens_save(state: &PlayState) -> bool {
    let mut clone = state.clone();
    let before = clone.active_yes_no_prompt.map(|session| session.kind);
    let _ = handle_play_key_input(&mut clone, 'Q', "", Path::new(""));
    before != Some(YesNoPromptKind::SaveGame)
        && clone.active_yes_no_prompt.map(|session| session.kind) == Some(YesNoPromptKind::SaveGame)
}

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn world() -> PlayState {
    world_state(open_world_grid(), 20, 20)
}

fn dungeon() -> PlayState {
    dungeon_state(open_dungeon_record(), 0, 3, 3)
}

#[test]
fn at_the_idle_prompt_the_predicate_agrees_with_the_dispatcher() {
    for (label, state) in [("town", town()), ("world", world()), ("dungeon", dungeon())] {
        assert!(
            state.at_idle_command_prompt(),
            "{label}: the fixture is idle"
        );
        assert!(state.save_command_available(), "{label}");
        assert!(q_opens_save(&state), "{label}: Q opens the prompt");
    }
}

#[test]
fn an_open_prompt_takes_the_q() {
    let mut state = town();
    state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::ExitToDos,
    });
    assert!(!state.save_command_available());
    assert!(!q_opens_save(&state));
    // The save prompt itself: Q is no answer there, and no new prompt
    // opens.
    let mut state = town();
    state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    assert!(!state.save_command_available());
    assert!(!q_opens_save(&state));
}

#[test]
fn combat_and_pending_work_are_never_save_points() {
    let mut state = town();
    state.combat_active = true;
    assert!(!state.save_command_available(), "save-load.md §5.1");

    let mut state = town();
    state
        .keys_buffered_during_hold
        .push_back(('x', String::new()));
    assert!(!state.at_idle_command_prompt());
    assert!(!state.save_command_available());
}

#[test]
fn a_drunk_party_is_refused_because_q_may_stagger() {
    let mut state = town();
    state.town_drunkenness_counter = 3;
    assert!(state.at_idle_command_prompt());
    assert!(!state.save_command_available());
    // Some draws stagger, some reach the prompt: the gate is a coin toss.
    let outcomes: Vec<bool> = (0..32u16)
        .map(|seed| {
            let mut state = state.clone();
            state.prng_state = seed.wrapping_mul(2654).wrapping_add(17);
            q_opens_save(&state)
        })
        .collect();
    assert!(outcomes.contains(&false), "some Q staggers: {outcomes:?}");
}

#[test]
fn a_live_moongate_runs_before_the_q() {
    let mut state = world();
    let idx = world_cell_index(state.player.x, state.player.y);
    state.grid[idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    assert_eq!(state.current_live_natural_moongate_cell_index(), Some(idx));
    assert!(!state.save_command_available());
    // A warp's arrival holds the hook back for one command: Q is a save.
    state.natural_moongate_hook_held_for_warp_arrival = true;
    assert!(state.save_command_available());
}

#[test]
fn a_dungeon_room_trigger_runs_before_the_q() {
    for tile in [0xf0, 0xa3] {
        let mut record = open_dungeon_record();
        record[crate::dungeon_cell_index(0, 3, 3)] = tile;
        let state = dungeon_state(record, 0, 3, 3);
        assert!(matches!(state.area, Area::Dungeon { level: 0, .. }));
        assert_eq!(state.dungeon_cell(0, 3, 3), tile);
        assert!(!state.save_command_available(), "tile {tile:#04x}");
    }
}

// ---------------------------------------------------------------------------
// The summary.

fn named(name: &[u8]) -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0x01, 0, 5, 5);
    bytes[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + name.len()].copy_from_slice(name);
    bytes[SAVE_PARTY_SIZE_OFFSET] = 1;
    bytes
}

#[test]
fn the_summary_reads_the_avatar_clock_and_place() {
    let summary = save_image_summary(&named(b"Ann")).unwrap();
    assert_eq!(summary.avatar.as_deref(), Some("Ann"));
    assert_eq!(summary.party_size, 1);
    assert_eq!((summary.clock.hour, summary.clock.minute), (8, 35));
    assert_eq!(
        summary.target,
        PlayTarget::Town(crate::Scene::new(0x01).unwrap())
    );
    assert_eq!(summary.floor, 0);

    // A full nine-byte name has no terminator.
    let summary = save_image_summary(&named(b"Abcdefghi")).unwrap();
    assert_eq!(summary.avatar.as_deref(), Some("Abcdefghi"));
}

#[test]
fn an_empty_save_has_no_avatar_and_is_no_error() {
    let summary = save_image_summary(&saved_game_seed_bytes(0, 0, 10, 10)).unwrap();
    assert_eq!(summary.avatar, None);
    assert!(matches!(summary.target, PlayTarget::World(_)));
}

#[test]
fn the_summary_reads_a_dungeon_level_and_refuses_bad_images() {
    let mut bytes = named(b"Ann");
    bytes[SAVE_SCENE_OFFSET] = 33;
    bytes[SAVE_Z_OFFSET] = 4;
    let summary = save_image_summary(&bytes).unwrap();
    assert!(matches!(summary.target, PlayTarget::Dungeon(_)));
    assert_eq!(summary.floor, 4);

    bytes[SAVE_Z_OFFSET] = 8;
    assert!(save_image_summary(&bytes).is_err(), "no ninth level");
    bytes[SAVE_SCENE_OFFSET] = 41;
    bytes[SAVE_Z_OFFSET] = 0;
    assert!(save_image_summary(&bytes).is_err(), "no scene 41");
    assert!(save_image_summary(&vec![0; SAVED_GAM_LEN - 1]).is_err());
}
