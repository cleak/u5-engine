//! M3-06 observer tests: Look (sign read, entrance named), the dungeon
//! events (ladder, pit, fall, the four rewrites, the fountain), the 10.6
//! negatives that must record nothing, and a found secret door that stays
//! a door after the dungeon reloads.
//!
//! Synthetic cases build the transcript rows the game prints with the
//! runtime's own constants; the cases marked "real" drive the dispatcher
//! with the player's own files and skip without `U5_TEST_ASSET_DIR`. No
//! game text is asserted or kept.

use std::path::Path;

use u5_runtime::enhancements::trajectory_digest;
use u5_runtime::input_dispatch::handle_play_key_input;
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
    test_state, world_state,
};
use u5_runtime::{
    ActiveObject, Area, DUNGEON_FOUNTAIN_ACCEPTED, DUNGEON_FOUNTAIN_DECLINED,
    DUNGEON_FOUNTAIN_DRINK_PROMPT, Direction, DungeonLookFocus, DungeonScene, InputDirection,
    LOOK_RESULT_PREFIX, PlayState, SCENE_OVERWORLD, WORD_OF_POWER_SEALED_TILE, WorldPlane,
    YesNoPromptKind, YesNoPromptSession, dungeon_cell_index, input_direction_code,
    world_cell_index,
};

use super::*;
use crate::gem::knowledge::{FeatureKey, GemKnowledge};
use crate::gem::probe::{observe_command, observe_commands, probe};
use crate::perceived::DungeonLook;

const TOWN: u8 = 0x11;
/// A sign tile (`view.md §3`'s sign list).
const SIGN: u8 = 0x89;

fn town(x: usize, y: usize) -> PlayState {
    test_state(open_grid(), x, y)
}

fn britannia(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state
}

/// A person or creature standing on `(x, y)` of the current floor.
fn put_object(state: &mut PlayState, x: usize, y: usize) {
    let z = match state.area {
        Area::Town { floor, .. } => floor,
        Area::World { plane } => plane.save_floor(),
        Area::Dungeon { level, .. } => level as i8,
    };
    state.active_objects.push(ActiveObject {
        type_byte: 0x40,
        tile: 0x40,
        x,
        y,
        z,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
}

/// The rows a completed top-down Look prints: the echo, the blank, the
/// preamble and a description row (`commands.rs`' documented layout).
fn print_look(state: &mut PlayState, direction: Direction) {
    state.push_message_entry(format!("Look-{}", direction.name()), true);
    state.push_message_entry("", false);
    state.push_message_entry(LOOK_RESULT_PREFIX.trim(), false);
    state.push_message_entry("x", false);
}

fn sign_at(scene: u8, floor: i8, x: u8, y: u8) -> KnowEvent {
    KnowEvent::SignRead(SignCell { scene, floor, x, y })
}

// ---- sign read -----------------------------------------------------------

#[test]
fn a_look_at_a_town_sign_records_that_sign() {
    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    let before = probe(&state);
    print_look(&mut state, Direction::North);
    assert_eq!(
        observe_command(&before, &state),
        vec![sign_at(TOWN, 0, 10, 9)]
    );
}

#[test]
fn a_look_at_a_world_sign_records_it_on_the_overworld() {
    let mut state = britannia(0, 100);
    state.grid[world_cell_index(255, 100)] = SIGN;
    let before = probe(&state);
    print_look(&mut state, Direction::West);
    assert_eq!(
        observe_command(&before, &state),
        vec![sign_at(SCENE_OVERWORLD, 0, 255, 100)]
    );
}

#[test]
fn a_look_at_plain_ground_records_nothing() {
    let mut state = town(10, 10);
    let before = probe(&state);
    print_look(&mut state, Direction::North);
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn a_look_at_a_person_on_a_sign_records_nothing() {
    // 10.6: the object case wins; the game printed the person.
    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    put_object(&mut state, 10, 9);
    let before = probe(&state);
    print_look(&mut state, Direction::North);
    assert!(observe_command(&before, &state).is_empty());

    // The person walked off during the Look's turn: still nothing, the
    // probe saw them there.
    let mut moved = state.clone();
    let before = probe(&moved);
    moved.active_objects.last_mut().unwrap().x = 12;
    print_look(&mut moved, Direction::North);
    assert!(observe_command(&before, &moved).is_empty());

    // Someone stepped onto the sign after the Look: nothing either.
    let mut late = town(10, 10);
    late.grid[9 * 32 + 10] = SIGN;
    let before = probe(&late);
    print_look(&mut late, Direction::North);
    put_object(&mut late, 10, 9);
    assert!(observe_command(&before, &late).is_empty());
}

#[test]
fn a_look_that_fails_the_preflight_records_nothing() {
    // The visibility gate clears the message: an echo and no preamble.
    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    let before = probe(&state);
    state.push_message_entry("Look-North", true);
    state.push_message_entry("", false);
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn a_cancelled_look_records_nothing() {
    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    let before = probe(&state);
    state.push_message_entry("Look-", true);
    state.push_message_entry("", false);
    assert!(observe_command(&before, &state).is_empty());
    // A preamble from something else after an unfinished echo is not a Look.
    state.push_message_entry(LOOK_RESULT_PREFIX.trim(), false);
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn an_unanchored_transcript_records_nothing() {
    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    for i in 0..4 {
        state.push_message_entry(format!("row {i}"), false);
    }
    let before = probe(&state);
    // The window cleared and rebuilt: the diff cannot tell new from old.
    state.clear_message_window();
    print_look(&mut state, Direction::North);
    assert!(
        !before.transcript.diff_state(&state).is_anchored(),
        "the case must not anchor"
    );
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn a_look_after_the_party_moved_or_two_commands_records_nothing() {
    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    state.grid[8 * 32 + 10] = SIGN;
    let before = probe(&state);
    state.player.y = 9;
    print_look(&mut state, Direction::North);
    assert!(observe_command(&before, &state).is_empty());

    let mut state = town(10, 10);
    state.grid[9 * 32 + 10] = SIGN;
    let before = probe(&state);
    print_look(&mut state, Direction::North);
    assert!(observe_commands(&before, &state, 2).is_empty());
    // Two Looks in one diff cannot be told apart either.
    let mut twice = town(10, 10);
    twice.grid[9 * 32 + 10] = SIGN;
    let before = probe(&twice);
    print_look(&mut twice, Direction::North);
    print_look(&mut twice, Direction::South);
    assert!(observe_command(&before, &twice).is_empty());
}

// ---- entrance named (D10.7) ----------------------------------------------

#[test]
fn a_look_at_a_named_entrance_records_its_dungeon() {
    let mut state = britannia(71, 70);
    state.grid[world_cell_index(72, 70)] = WORD_OF_POWER_SEALED_TILE;
    let before = probe(&state);
    print_look(&mut state, Direction::East);
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::EntranceNamed { record: 2 }]
    );
}

#[test]
fn a_look_redirected_onto_an_entrance_records_it() {
    // 0xE1 sends the Look one cell east.
    let mut state = britannia(56, 40);
    state.grid[world_cell_index(57, 40)] = u5_runtime::LOOK_REDIRECT_EAST_TILE;
    state.grid[world_cell_index(58, 40)] = WORD_OF_POWER_SEALED_TILE;
    let before = probe(&state);
    print_look(&mut state, Direction::East);
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::EntranceNamed { record: 5 }]
    );
}

#[test]
fn an_entrance_at_an_unnamed_column_or_under_an_object_records_nothing() {
    let mut state = britannia(70, 70);
    state.grid[world_cell_index(71, 70)] = WORD_OF_POWER_SEALED_TILE;
    let before = probe(&state);
    print_look(&mut state, Direction::East);
    assert!(observe_command(&before, &state).is_empty());

    let mut covered = britannia(71, 70);
    covered.grid[world_cell_index(72, 70)] = WORD_OF_POWER_SEALED_TILE;
    put_object(&mut covered, 72, 70);
    let before = probe(&covered);
    print_look(&mut covered, Direction::East);
    assert!(observe_command(&before, &covered).is_empty());
}

#[test]
fn the_entrance_columns_match_the_runtime_look_appender() {
    // `look_description` appends the dungeon's name at exactly these
    // columns (`view.md §3` row 5c); the record's name is the engine's own.
    let state = britannia(1, 1);
    let names: Vec<&str> = (0..8)
        .map(|r| DungeonScene::from_record(r).unwrap().name())
        .collect();
    for x in 0..=255u8 {
        let text = state.look_description(WORD_OF_POWER_SEALED_TILE, None, x as usize);
        let appended = names.iter().position(|name| text.ends_with(name));
        assert_eq!(
            appended.map(|r| r as u8),
            entrance_record_for_column(x),
            "column {x}"
        );
    }
}

#[test]
fn the_record_knows_an_entrance_by_look_without_entering() {
    let mut know = GemKnowledge::new();
    let mut state = britannia(239, 10);
    state.grid[world_cell_index(240, 10)] = WORD_OF_POWER_SEALED_TILE;
    let before = probe(&state);
    print_look(&mut state, Direction::East);
    for event in observe_command(&before, &state) {
        know.apply(&event);
    }
    assert!(know.is_known(&FeatureKey::Entrance { record: 0 }));
    let scene = DungeonScene::from_record(0).unwrap();
    assert!(!know.is_known(&FeatureKey::Place { scene: scene.byte }));
    assert!(!know.is_known(&FeatureKey::Entrance { record: 1 }));
}

// ---- dungeon -------------------------------------------------------------

fn dungeon(level: u8, x: usize, y: usize) -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), level, x, y);
    state.player.facing = Direction::East;
    state
}

fn record_of(state: &PlayState) -> u8 {
    let Area::Dungeon { scene, .. } = state.area else {
        unreachable!()
    };
    scene.record as u8
}

fn set_cell(state: &mut PlayState, level: u8, x: usize, y: usize, byte: u8) {
    state.grid[dungeon_cell_index(level, x, y)] = byte;
}

fn set_level(state: &mut PlayState, level: u8) {
    if let Area::Dungeon { scene, .. } = state.area {
        state.area = Area::Dungeon { scene, level };
    }
}

fn dmark(record: u8, level: u8, x: u8, y: u8, kind: MarkKind) -> KnowEvent {
    KnowEvent::DungeonMarked(DungeonMark {
        record,
        level,
        x,
        y,
        kind,
    })
}

fn seen_cell(record: u8, level: u8, x: u8, y: u8) -> KnowEvent {
    let mut rows = [0; 8];
    rows[y as usize] = 1 << x;
    KnowEvent::DungeonCellsSeen {
        record,
        level,
        rows,
    }
}

#[test]
fn a_ladder_climbed_down_marks_both_ends() {
    let mut state = dungeon(2, 3, 4);
    let r = record_of(&state);
    set_cell(&mut state, 2, 3, 4, 0x20);
    set_cell(&mut state, 3, 3, 4, 0x10);
    let before = probe(&state);
    set_level(&mut state, 3);
    assert_eq!(
        observe_command(&before, &state),
        vec![
            dmark(r, 2, 3, 4, MarkKind::Ladder),
            dmark(r, 3, 3, 4, MarkKind::Ladder),
            seen_cell(r, 3, 3, 4),
        ]
    );
}

#[test]
fn a_ladder_climbed_up_marks_both_ends_and_a_wrong_way_change_nothing() {
    let mut state = dungeon(5, 0, 7);
    let r = record_of(&state);
    set_cell(&mut state, 5, 0, 7, 0x18);
    set_cell(&mut state, 4, 0, 7, 0x30);
    let before = probe(&state);
    set_level(&mut state, 4);
    assert_eq!(
        observe_command(&before, &state),
        vec![
            dmark(r, 5, 0, 7, MarkKind::Ladder),
            dmark(r, 4, 0, 7, MarkKind::Ladder),
            seen_cell(r, 4, 0, 7),
        ]
    );
    // An up ladder underfoot and the party went down (a spell): not a
    // ladder.
    let mut spell = dungeon(5, 0, 7);
    set_cell(&mut spell, 5, 0, 7, 0x10);
    let before = probe(&spell);
    set_level(&mut spell, 6);
    assert!(observe_command(&before, &spell).is_empty());
}

#[test]
fn a_klimb_off_the_top_edge_marks_the_departure_ladder() {
    let mut state = dungeon(0, 6, 1);
    let r = record_of(&state);
    set_cell(&mut state, 0, 6, 1, 0x10);
    let before = probe(&state);
    let after = britannia(91, 59);
    assert_eq!(
        observe_command(&before, &after),
        vec![dmark(r, 0, 6, 1, MarkKind::Ladder)]
    );
    // Leaving level 0 from a plain cell (an exit by other means) marks
    // nothing.
    let mut plain = dungeon(0, 6, 1);
    set_cell(&mut plain, 0, 6, 1, 0x00);
    let before = probe(&plain);
    assert!(observe_command(&before, &britannia(91, 59)).is_empty());
}

#[test]
fn a_pit_climbed_down_is_found() {
    let mut state = dungeon(1, 2, 2);
    let r = record_of(&state);
    set_cell(&mut state, 1, 2, 2, 0x60);
    let before = probe(&state);
    set_level(&mut state, 2);
    assert_eq!(
        observe_command(&before, &state),
        vec![dmark(r, 1, 2, 2, MarkKind::Trap), seen_cell(r, 2, 2, 2)]
    );
}

#[test]
fn a_fall_after_a_step_finds_every_trap_of_the_chain() {
    // East from (2, 5) onto a hidden pit, through a second one below.
    let mut state = dungeon(1, 2, 5);
    let r = record_of(&state);
    set_cell(&mut state, 1, 3, 5, 0x69);
    set_cell(&mut state, 2, 3, 5, 0x61);
    let before = probe(&state);
    // What the chain leaves: each trap's 0x08 cleared, the party two down.
    set_cell(&mut state, 1, 3, 5, 0x61);
    set_level(&mut state, 3);
    state.player.x = 3;
    assert_eq!(
        observe_command(&before, &state),
        vec![
            dmark(r, 1, 3, 5, MarkKind::Trap),
            seen_cell(r, 1, 3, 5),
            dmark(r, 2, 3, 5, MarkKind::Trap),
            seen_cell(r, 2, 3, 5),
            seen_cell(r, 3, 3, 5),
        ]
    );
}

#[test]
fn a_level_change_without_a_trap_or_ladder_records_nothing() {
    // A step and a level change with no fall trap (a spell, a teleport).
    let mut state = dungeon(1, 2, 5);
    let before = probe(&state);
    set_level(&mut state, 2);
    state.player.x = 3;
    assert!(observe_command(&before, &state).is_empty());
    // Two commands: the ladder is dropped.
    let mut ladder = dungeon(2, 3, 4);
    set_cell(&mut ladder, 2, 3, 4, 0x20);
    let before = probe(&ladder);
    set_level(&mut ladder, 3);
    assert!(observe_commands(&before, &ladder, 2).is_empty());
}

#[test]
fn each_rewrite_at_the_party_or_focus_cell_is_recorded() {
    // Facing east from (3, 3): ahead (4, 3), left (3, 2), right (3, 4).
    let cases = [
        ((3, 3), 0x40, 0x70, MarkKind::Chest),
        ((4, 3), 0x61, 0x60, MarkKind::Trap),
        ((3, 2), 0x62, 0x00, MarkKind::Trap),
        ((3, 4), 0x6A, 0x08, MarkKind::Trap),
        ((4, 3), 0xD3, 0xE3, MarkKind::SecretDoor),
    ];
    for ((x, y), was, is, kind) in cases {
        let mut state = dungeon(4, 3, 3);
        let r = record_of(&state);
        set_cell(&mut state, 4, x, y, was);
        let before = probe(&state);
        set_cell(&mut state, 4, x, y, is);
        assert_eq!(
            observe_command(&before, &state),
            vec![dmark(r, 4, x as u8, y as u8, kind)],
            "{was:#04x} -> {is:#04x} at ({x}, {y})"
        );
    }
}

#[test]
fn an_unrelated_rewrite_away_from_the_party_records_nothing() {
    for (x, y) in [(2, 3), (6, 6), (0, 0)] {
        // Behind the party, and far away.
        let mut state = dungeon(4, 3, 3);
        set_cell(&mut state, 4, x, y, 0xD0);
        let before = probe(&state);
        set_cell(&mut state, 4, x, y, 0xE0);
        assert!(observe_command(&before, &state).is_empty(), "({x}, {y})");
    }
    // A rewrite outside the table at the party cell: nothing.
    let mut state = dungeon(4, 3, 3);
    set_cell(&mut state, 4, 3, 3, 0x69);
    let before = probe(&state);
    set_cell(&mut state, 4, 3, 3, 0x61);
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn two_commands_keep_only_rewrites_the_party_stood_on() {
    let mut state = dungeon(4, 3, 3);
    let r = record_of(&state);
    set_cell(&mut state, 4, 3, 3, 0x40);
    set_cell(&mut state, 4, 4, 3, 0xD0);
    let before = probe(&state);
    set_cell(&mut state, 4, 3, 3, 0x70);
    set_cell(&mut state, 4, 4, 3, 0xE0);
    assert_eq!(
        observe_commands(&before, &state, 2),
        vec![dmark(r, 4, 3, 3, MarkKind::Chest)]
    );
}

fn fountain_prompt(state: &mut PlayState, focus: DungeonLookFocus) {
    state.active_yes_no_prompt = Some(YesNoPromptSession::new(
        YesNoPromptKind::DungeonFountainDrink {
            party_index: 0,
            focus,
        },
    ));
    state.push_message_entry(DUNGEON_FOUNTAIN_DRINK_PROMPT.trim_end(), false);
}

#[test]
fn a_fountain_drunk_on_yes_is_marked() {
    let mut state = dungeon(3, 5, 5);
    let r = record_of(&state);
    set_cell(&mut state, 3, 6, 5, 0x52);
    fountain_prompt(&mut state, DungeonLookFocus::Ahead);
    let before = probe(&state);
    state.active_yes_no_prompt = None;
    state.push_message_entry(DUNGEON_FOUNTAIN_ACCEPTED.trim_end(), false);
    assert_eq!(
        observe_command(&before, &state),
        vec![dmark(r, 3, 6, 5, MarkKind::Fountain)]
    );
}

#[test]
fn a_fountain_answered_no_or_still_open_records_nothing() {
    let mut state = dungeon(3, 5, 5);
    set_cell(&mut state, 3, 5, 5, 0x50);
    fountain_prompt(&mut state, DungeonLookFocus::Here);
    let before = probe(&state);
    let mut declined = state.clone();
    declined.active_yes_no_prompt = None;
    declined.push_message_entry(DUNGEON_FOUNTAIN_DECLINED.trim_end(), false);
    assert!(observe_command(&before, &declined).is_empty());
    // An ignored key leaves the prompt up.
    let mut open = state.clone();
    open.push_message_entry(DUNGEON_FOUNTAIN_ACCEPTED.trim_end(), false);
    assert!(observe_command(&before, &open).is_empty());
    // Yes, but the transcript cannot anchor.
    let mut lost = state.clone();
    lost.active_yes_no_prompt = None;
    lost.clear_message_window();
    lost.push_message_entry(DUNGEON_FOUNTAIN_ACCEPTED.trim_end(), false);
    assert!(observe_command(&before, &lost).is_empty());
}

#[test]
fn a_found_secret_door_stays_a_door_after_a_reload() {
    let mut state = dungeon(6, 3, 3);
    let r = record_of(&state);
    set_cell(&mut state, 6, 4, 3, 0xD5);
    let before = probe(&state);
    set_cell(&mut state, 6, 4, 3, 0xE5);
    let mut know = GemKnowledge::new();
    for event in observe_command(&before, &state) {
        know.apply(&event);
    }
    // The dungeon reloads from DUNGEON.DAT: the live byte is 0xD? again.
    let reloaded = know.clone();
    assert_eq!(
        reloaded.perceived_cell(r, 6, 4, 3, 0xD5).look,
        DungeonLook::Door
    );
    // Unfound doors, and the same byte elsewhere, still draw as wall.
    assert_eq!(
        GemKnowledge::new().perceived_cell(r, 6, 4, 3, 0xD5).look,
        DungeonLook::Wall
    );
    assert_eq!(
        reloaded.perceived_cell(r, 6, 5, 3, 0xD5).look,
        DungeonLook::Wall
    );
    assert_eq!(
        reloaded.perceived_cell(r, 5, 4, 3, 0xD5).look,
        DungeonLook::Wall
    );
    // A found pit draws as a pit after the reload too.
    let mut pit = GemKnowledge::new();
    pit.apply(&dmark(r, 1, 2, 2, MarkKind::Trap));
    assert_eq!(pit.perceived_cell(r, 1, 2, 2, 0x61).look, DungeonLook::Pit);
    assert_eq!(
        GemKnowledge::new().perceived_cell(r, 1, 2, 2, 0x61).look,
        DungeonLook::Passage
    );
}

// ---- purity --------------------------------------------------------------

#[test]
fn the_new_events_leave_the_state_unchanged() {
    let mut sign = town(10, 10);
    sign.grid[9 * 32 + 10] = SIGN;
    let sign_before = probe(&sign);
    print_look(&mut sign, Direction::North);
    let mut fountain = dungeon(3, 5, 5);
    set_cell(&mut fountain, 3, 6, 5, 0x52);
    fountain_prompt(&mut fountain, DungeonLookFocus::Ahead);
    let fountain_before = probe(&fountain);
    fountain.active_yes_no_prompt = None;
    fountain.push_message_entry(DUNGEON_FOUNTAIN_ACCEPTED.trim_end(), false);
    for (i, (before, mut state)) in [(sign_before, sign), (fountain_before, fountain)]
        .into_iter()
        .enumerate()
    {
        state.prng_state = 0x4321 + i as u16;
        let digest = trajectory_digest(&state);
        let entries = state.message_entries().to_vec();
        assert_eq!(observe_command(&before, &state).len(), 1);
        assert_eq!(trajectory_digest(&state), digest);
        assert_eq!(state.message_entries(), &entries[..]);
        assert_eq!(state.prng_state, 0x4321 + i as u16);
    }
}

// ---- real: the dispatcher with the player's files ------------------------

fn key(direction: InputDirection) -> char {
    char::from(input_direction_code(direction))
}

/// A transcript with some history, as in play: a cursor over a single
/// open row cannot tell an in-place edit from a replacement (8.3.4), so a
/// fresh fixture's first echo would not anchor.
fn with_history(mut state: PlayState) -> PlayState {
    for i in 0..3 {
        state.push_message_entry(format!("history {i}"), false);
    }
    state
}

/// One command through the dispatcher, probed and observed.
fn run(state: &mut PlayState, k: char, dir: &Path) -> KnowEvents {
    let before = probe(state);
    handle_play_key_input(state, k, "", dir).unwrap();
    observe_command(&before, state)
}

#[test]
fn real_look_at_a_sign_records_it_and_a_person_on_it_does_not() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    let mut state = with_history(town(10, 10));
    state.grid[9 * 32 + 10] = SIGN;
    // Two frames: `L` opens the prompt, the direction completes it.
    assert!(run(&mut state, 'L', &dir).is_empty());
    assert_eq!(
        run(&mut state, key(InputDirection::North), &dir),
        vec![sign_at(TOWN, 0, 10, 9)]
    );
    // Both keys in one command (the terminal's inline form).
    let mut inline = with_history(town(10, 10));
    inline.grid[11 * 32 + 10] = SIGN;
    let before = probe(&inline);
    handle_play_key_input(&mut inline, 'L', "", &dir).unwrap();
    handle_play_key_input(&mut inline, key(InputDirection::South), "", &dir).unwrap();
    assert_eq!(
        observe_command(&before, &inline),
        vec![sign_at(TOWN, 0, 10, 11)]
    );

    let mut person = with_history(town(10, 10));
    person.grid[9 * 32 + 10] = SIGN;
    put_object(&mut person, 10, 9);
    assert!(run(&mut person, 'L', &dir).is_empty());
    assert!(run(&mut person, key(InputDirection::North), &dir).is_empty());

    // Cancelled with Escape.
    let mut cancel = with_history(town(10, 10));
    cancel.grid[9 * 32 + 10] = SIGN;
    assert!(run(&mut cancel, 'L', &dir).is_empty());
    assert!(run(&mut cancel, '\u{1b}', &dir).is_empty());
}

#[test]
fn real_look_at_an_entrance_names_its_dungeon() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    let mut state = with_history(britannia(127, 70));
    state.grid[world_cell_index(126, 70)] = WORD_OF_POWER_SEALED_TILE;
    assert!(run(&mut state, 'L', &dir).is_empty());
    assert_eq!(
        run(&mut state, key(InputDirection::West), &dir),
        vec![KnowEvent::EntranceNamed { record: 3 }]
    );
}

#[test]
fn real_fountain_yes_is_marked_and_no_is_not() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    for (answer, marked) in [('Y', true), ('N', false)] {
        let mut state = with_history(dungeon(3, 5, 5));
        state.torch_counter = 50;
        let r = record_of(&state);
        set_cell(&mut state, 3, 6, 5, 0x50);
        // The prompt as the dungeon Look opens it, then the answer key.
        state.start_dungeon_fountain_drink_prompt(0, DungeonLookFocus::Ahead);
        let events = run(&mut state, answer, &dir);
        let expected = if marked {
            vec![dmark(r, 3, 6, 5, MarkKind::Fountain)]
        } else {
            vec![]
        };
        assert_eq!(events, expected, "answer {answer}");
        assert!(state.active_yes_no_prompt.is_none());
    }
}
