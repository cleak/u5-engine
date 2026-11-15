//! Phase 3a, the feed-path proof (design 6.5, 6.6.3; plan M7-05 commit 1):
//! the `AutoKey` table matches the terminal's chars, and a key pushed into
//! `typed_keys` in `EnhSet::Feed` is the same to the game as the same key
//! arriving as a `KeyboardInput`, at a combat command prompt and in an
//! open targeting cursor.

use std::collections::VecDeque;

use u5_runtime::auto_combat::{AutoPrompt, auto_prompt};
use u5_runtime::combat_actor::{
    COMBAT_ACTOR_FLAG_SELECTABLE_40, COMBAT_ACTOR_FLAG_SELECTABLE_80, CombatActorDescriptor,
};
use u5_runtime::combat_arena::COMBAT_ARENA_SIDE;
use u5_runtime::constants::OOL_SLOTS;
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::{ActiveObject, PlayState};

use super::*;
use crate::enh::equivalence::{Scene, ShellRun, Timeline, compare_shell_runs, run_shell};
use crate::enh::{EnhSet, VisualLaunchConfig, configure_enh_sets};
use crate::{VisualState, visual_play_command_char};

const DIRECTIONS: [InputDirection; 8] = [
    InputDirection::North,
    InputDirection::South,
    InputDirection::East,
    InputDirection::West,
    InputDirection::Northwest,
    InputDirection::Northeast,
    InputDirection::Southwest,
    InputDirection::Southeast,
];

/// Every key the planner can emit.
fn every_auto_key() -> Vec<AutoKey> {
    let mut keys = vec![
        AutoKey::Attack,
        AutoKey::Pass,
        AutoKey::Confirm,
        AutoKey::Cancel,
    ];
    keys.extend(
        DIRECTIONS
            .into_iter()
            .filter(|direction| direction.is_cardinal())
            .map(AutoKey::Step),
    );
    keys.extend(DIRECTIONS.into_iter().map(AutoKey::Cursor));
    keys
}

const SKELETON_CLASS: u8 = 33;

fn place(state: &mut PlayState, slot: usize, tile: u8, row: [u8; 8]) {
    state.active_objects[slot] = ActiveObject {
        type_byte: tile,
        tile,
        x: usize::from(row[6]),
        y: usize::from(row[7]),
        ..ActiveObject::empty()
    };
    state.combat_actors[slot] = CombatActorDescriptor::from_row(row);
}

/// A fight on open ground: member 0 is prompted at (5, 5), a skeleton
/// stands two cells east, so `A` opens a targeting cursor.
pub(super) fn arena() -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.combat_active = true;
    state.combat_round_loop_prologue_ran = true;
    state.time_stop_counter = u8::MAX;
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    for member in state.party.iter_mut() {
        member.status = b'G';
        member.hp = 20;
        member.max_hp = 20;
    }
    place(
        &mut state,
        0,
        0x80,
        [20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5],
    );
    place(
        &mut state,
        16,
        0x90,
        [
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            SKELETON_CLASS,
            16,
            0,
            7,
            5,
        ],
    );
    state.pending_combat_actor_slot = Some(0);
    state.prng_state = 0x3270;
    state
}

#[test]
fn every_auto_key_types_the_char_the_terminal_sends() {
    for state in [arena(), test_state(open_grid(), 5, 5)] {
        for key in every_auto_key() {
            let typed = auto_typed_key(key);
            assert!(!typed.shift && !typed.control, "{key:?}: no modifier");
            assert_eq!(
                visual_play_command_char(&state, typed.key, typed.shift, typed.control),
                Some(key.input_char()),
                "{key:?}"
            );
        }
    }
}

/// The test-only feed: pushes the scheduled keys into the game's key queue
/// in `EnhSet::Feed`, as the auto-combat feed does. Frames count from 0,
/// the index of the shell's `step`.
#[derive(Resource, Default)]
struct TestFeed {
    frame: usize,
    keys: VecDeque<(usize, AutoKey)>,
}

fn feed_test_keys(mut feed: ResMut<TestFeed>, visual: Option<ResMut<VisualState>>) {
    let frame = feed.frame;
    feed.frame += 1;
    let Some(mut visual) = visual else {
        return;
    };
    while feed.keys.front().is_some_and(|(at, _)| *at == frame) {
        let (_, key) = feed.keys.pop_front().unwrap();
        visual.typed_keys.push_back(auto_typed_key(key));
    }
}

fn install_test_feed(keys: Vec<(usize, AutoKey)>) -> impl FnOnce(&mut App) {
    move |app: &mut App| {
        configure_enh_sets(app);
        app.insert_resource(TestFeed {
            frame: 0,
            keys: keys.into(),
        })
        .add_systems(Update, feed_test_keys.in_set(EnhSet::Feed));
    }
}

const FRAMES: usize = 60;
/// The player's `A`, opening the cursor for the cursor cases.
const OPEN_AT: usize = 5;
const KEY_AT: usize = 25;

/// The typed run (`key` as a `KeyboardInput` tap) and the fed run (`key`
/// pushed in `EnhSet::Feed`) in the same frame, after `before` typed by
/// the player in both.
fn typed_and_fed(before: &[(usize, AutoKey)], key: AutoKey) -> (ShellRun, ShellRun) {
    let mut shared = Timeline::new(FRAMES);
    for (frame, early) in before {
        shared.game_tap(*frame, auto_typed_key(*early).key);
    }
    let mut typed = shared.clone();
    typed.game_tap(KEY_AT, auto_typed_key(key).key);
    let launch = VisualLaunchConfig::classic();
    let scene = Scene::synthetic();
    let typed = run_shell(arena(), &scene, &launch, |_| {}, &typed, None);
    let fed = run_shell(
        arena(),
        &scene,
        &launch,
        install_test_feed(vec![(KEY_AT, key)]),
        &shared,
        None,
    );
    (typed, fed)
}

fn assert_fed_equals_typed(before: &[(usize, AutoKey)], key: AutoKey) {
    let (typed, fed) = typed_and_fed(before, key);
    assert_eq!(compare_shell_runs(&typed, &fed), [], "{key:?}");
    let digest = |frame: usize| typed.frames[frame].digest;
    assert_ne!(
        digest(KEY_AT - 1),
        digest(FRAMES - 1),
        "{key:?}: the key must change the game"
    );
}

#[test]
fn a_fed_key_at_the_combat_prompt_equals_the_typed_key() {
    assert!(matches!(
        auto_prompt(&arena()),
        AutoPrompt::Command {
            slot: 0,
            roster: Some(0)
        }
    ));
    for key in [
        AutoKey::Attack,
        AutoKey::Pass,
        AutoKey::Step(InputDirection::North),
        AutoKey::Step(InputDirection::West),
    ] {
        assert_fed_equals_typed(&[], key);
    }
}

#[test]
fn a_fed_key_in_an_open_cursor_equals_the_typed_key() {
    let before = [(OPEN_AT, AutoKey::Attack)];
    // The player's `A` really opens the cursor, and it is still open when
    // the key arrives.
    let mut shell = crate::enh::equivalence::Shell::new(
        arena(),
        &Scene::synthetic(),
        &VisualLaunchConfig::classic(),
        |_| {},
    );
    let mut timeline = Timeline::new(KEY_AT);
    timeline.game_tap(OPEN_AT, KeyCode::KeyA);
    for frame in 0..KEY_AT {
        shell.step(timeline.frame(frame));
    }
    assert!(
        matches!(
            auto_prompt(&shell.visual().state),
            AutoPrompt::Cursor { slot: 0 }
        ),
        "{:?}",
        auto_prompt(&shell.visual().state)
    );
    for key in [
        AutoKey::Cursor(InputDirection::East),
        AutoKey::Cursor(InputDirection::Southeast),
        AutoKey::Cursor(InputDirection::Northwest),
        AutoKey::Confirm,
        AutoKey::Cancel,
    ] {
        assert_fed_equals_typed(&before, key);
    }
}
