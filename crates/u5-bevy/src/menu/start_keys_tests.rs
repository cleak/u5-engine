//! The start keys in the real `--visual` app, headless (design 3.2.7
//! "Game", 3.3.8 "Injection", 3.4.2; plan M4-13): each preference works with
//! the overlay off, types its key once per process and only while the live
//! value differs, Ctrl-B follows Ctrl-S on a later frame, the keys equal
//! the same keys typed, and a load starts Sound On and Buffer Off.

use std::fs;

use u5_runtime::enhancements::Toggle;
use u5_runtime::test_fixtures::{
    debug_game_dir, open_grid, saved_game_seed_bytes, test_state, write_empty_ool_mirrors,
};
use u5_runtime::{
    PlayState, SAVE_AVATAR_NAME_OFFSET, SAVED_GAM_FILENAME, SAVED_OOL_LEN,
    load_play_options_from_save,
};

use super::*;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};
use crate::menu::MenuPlugin;

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn launch_with(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

const SOUND_OFF: Toggle = Toggle::AudioStartSoundOff;
const BUFFER_ON: Toggle = Toggle::AudioStartBufferOn;

/// `frames` frames of `state` under `launch`, with no key pressed; the
/// start keys' state after each frame.
fn run(state: PlayState, launch: &VisualLaunchConfig, frames: usize) -> (Shell, Vec<StartKeys>) {
    let mut shell = Shell::new(state, &Scene::synthetic(), launch, |_| {});
    let mut seen = Vec::new();
    for _ in 0..frames {
        shell.step(&[]);
        if let Some(keys) = shell.app.world().get_resource::<StartKeys>() {
            seen.push(keys.clone());
        }
    }
    (shell, seen)
}

#[test]
fn sound_off_types_ctrl_s_once_with_the_overlay_off() {
    let (shell, seen) = run(town(), &launch_with(&[SOUND_OFF]), 60);
    assert!(shell.app.is_plugin_added::<StartKeysPlugin>());
    assert!(!shell.app.is_plugin_added::<MenuPlugin>(), "no overlay");
    let keys = seen.last().unwrap();
    assert_eq!(keys.typed, [SOUND_KEY], "once per process");
    assert!(keys.done());
    let state = &shell.visual().state;
    assert!(!state.music_enabled, "the game's Sound is off");
    assert!(!state.typeahead_buffer_enabled, "Buffer untouched");
    let owner = shell.app.world().resource::<SyntheticInputOwner>();
    assert_eq!(owner.holder(), None, "the token is released");
    assert_eq!(state.turn, town().turn, "Ctrl-S costs no turn");
}

#[test]
fn buffer_on_types_ctrl_b_once_with_the_overlay_off() {
    let (shell, seen) = run(town(), &launch_with(&[BUFFER_ON]), 60);
    assert!(!shell.app.is_plugin_added::<MenuPlugin>());
    assert_eq!(seen.last().unwrap().typed, [BUFFER_KEY]);
    let state = &shell.visual().state;
    assert!(state.typeahead_buffer_enabled);
    assert!(state.music_enabled, "Sound untouched");
}

#[test]
fn ctrl_b_follows_ctrl_s_on_a_later_frame() {
    let (shell, seen) = run(town(), &launch_with(&[SOUND_OFF, BUFFER_ON]), 60);
    assert_eq!(seen.last().unwrap().typed, [SOUND_KEY, BUFFER_KEY]);
    let first = |n: usize| seen.iter().position(|keys| keys.typed.len() == n).unwrap();
    assert!(
        first(2) > first(1),
        "Ctrl-B is queued only after Ctrl-S was read"
    );
    assert_eq!(seen[first(1)].sound, StartStep::Typing);
    assert_eq!(seen[first(2)].sound, StartStep::Done);
    let state = &shell.visual().state;
    assert!(!state.music_enabled && state.typeahead_buffer_enabled);
}

#[test]
fn nothing_is_typed_when_the_live_value_already_matches() {
    let mut state = town();
    state.music_enabled = false;
    state.typeahead_buffer_enabled = true;
    let (shell, seen) = run(state, &launch_with(&[SOUND_OFF, BUFFER_ON]), 30);
    let keys = seen.last().unwrap();
    assert!(keys.typed.is_empty(), "{:?}", keys.typed);
    assert!(keys.done());
    let state = &shell.visual().state;
    assert!(!state.music_enabled && state.typeahead_buffer_enabled);
    assert!(shell.visual().typed_keys.is_empty());
}

#[test]
fn with_the_preferences_off_nothing_is_added_or_typed() {
    let (shell, seen) = run(town(), &VisualLaunchConfig::classic(), 30);
    assert!(seen.is_empty());
    assert!(!shell.app.is_plugin_added::<StartKeysPlugin>());
    assert!(!shell.app.world().contains_resource::<SyntheticInputOwner>());
    let state = &shell.visual().state;
    assert!(state.music_enabled && !state.typeahead_buffer_enabled);
    // The overlay alone does not type them either.
    let (shell, _) = run(town(), &launch_with(&[Toggle::MenuOverlay]), 30);
    assert!(!shell.app.is_plugin_added::<StartKeysPlugin>());
    let state = &shell.visual().state;
    assert!(state.music_enabled && !state.typeahead_buffer_enabled);
}

/// The start keys equal the same keys typed in the frames they were fed
/// (digest with the transcript, the faithful frame and the key queue).
#[test]
fn the_start_keys_equal_typed_keys() {
    for toggles in [&[SOUND_OFF][..], &[BUFFER_ON], &[SOUND_OFF, BUFFER_ON]] {
        let mismatches = check_shell_equivalence(
            &town(),
            &Scene::synthetic(),
            &Timeline::new(60),
            &launch_with(toggles),
            |_| {},
        );
        assert!(mismatches.is_empty(), "{toggles:?}: {mismatches:?}");
    }
}

/// With the preferences off a load starts with the game's Sound On and
/// Buffer Off, whatever they were when the game was saved: neither is in
/// the save (`audio.md §3`, `input.md §6`), so one key per process is
/// enough.
#[test]
fn a_load_starts_with_sound_on_and_buffer_off() {
    let dir = debug_game_dir();
    let mut image = saved_game_seed_bytes(0, 0, 10, 20);
    image[SAVE_AVATAR_NAME_OFFSET] = b'A';
    fs::write(dir.join(SAVED_GAM_FILENAME), image).unwrap();
    fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    let load = || {
        let options = load_play_options_from_save(&dir).unwrap();
        PlayState::load_scene(&dir, options).unwrap()
    };
    let mut state = load();
    assert!(state.music_enabled, "Sound On");
    assert!(!state.typeahead_buffer_enabled, "Buffer Off");
    state.music_enabled = false;
    state.typeahead_buffer_enabled = true;
    state.save_game_command(&dir, Some(true)).unwrap();
    let state = load();
    assert!(state.music_enabled, "Sound On after a save with Sound Off");
    assert!(
        !state.typeahead_buffer_enabled,
        "Buffer Off after a save with Buffer On"
    );
    let _ = fs::remove_dir_all(dir);
}
