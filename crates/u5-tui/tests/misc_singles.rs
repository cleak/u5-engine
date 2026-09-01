//! Focused pins for the `misc-singles` published-spec gap batch,
//! shell side.

use u5_runtime::test_fixtures::{open_world_grid, world_state};
use u5_runtime::{YellSession, YesNoPromptKind, YesNoPromptSession};
use u5_tui::play_loop::{
    play_state_accepts_typeahead, play_state_free_text_line_reader,
    play_state_honours_typeahead_queue, play_state_typeahead_setting_in_force,
};

/// `systems/input.md §6`, "Case Folding and One-Keystroke-per-Turn":
/// "Three things write the setting and nothing else does. The player
/// toggles it from the world command dispatcher ..., combat offers a
/// second, independent copy of the same toggle ..., and the free-text
/// line reader saves the setting, forces type-ahead on for the duration
/// of a typed line, and restores it afterwards. That last one means
/// typing a name, a keyword, or a word of power always honours the
/// queue regardless of the player's choice."
///
/// Restated in `§8` step 2: "Disable the buffer flush. The flush gate
/// (Section 6) is cleared so the player can type ahead. The prompt
/// restores it on exit."
#[test]
fn free_text_line_reader_forces_typeahead_on_and_keeps_the_queue() {
    let mut state = world_state(open_world_grid(), 4, 5);
    // The player's own choice is OFF - "Off is the default at startup".
    state.typeahead_buffer_enabled = false;
    assert!(play_state_accepts_typeahead(&state));
    assert!(!play_state_free_text_line_reader(&state));
    assert!(!play_state_typeahead_setting_in_force(&state));

    // A word-of-power prompt is the free-text line reader.
    state.active_yell = Some(YellSession {
        buffer: String::new(),
    });

    assert!(play_state_free_text_line_reader(&state));
    // The queue survives the prompt even though the modal gate is shut.
    assert!(!play_state_accepts_typeahead(&state));
    assert!(play_state_honours_typeahead_queue(&state));
    // "forces type-ahead on for the duration of a typed line ...
    // regardless of the player's choice".
    assert!(play_state_typeahead_setting_in_force(&state));

    // "and restores it afterwards": nothing wrote the player's setting,
    // so closing the prompt puts the original choice back in force.
    state.active_yell = None;
    assert!(!state.typeahead_buffer_enabled);
    assert!(!play_state_typeahead_setting_in_force(&state));
}

/// `systems/input.md §8`: "Single-character prompts (Y/N, a digit, a
/// target-slot letter) run the loop exactly once" — they are not the
/// free-text line reader, so they keep the ordinary flush.
#[test]
fn single_character_prompts_still_flush_the_queue() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.typeahead_buffer_enabled = true;
    state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::ExitToDos,
    });

    assert!(!play_state_free_text_line_reader(&state));
    assert!(!play_state_accepts_typeahead(&state));
    assert!(!play_state_honours_typeahead_queue(&state));
    assert!(!play_state_typeahead_setting_in_force(&state));
}
