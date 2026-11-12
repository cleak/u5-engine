//! The cursor cases of M3-05: append, in-place edit, withdrawal, clear,
//! cap and rebuild. Rows are made-up placeholders, never game text.

use super::*;
use crate::test_fixtures::{open_grid, test_state};

fn state_with(rows: usize) -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.clear_message_window();
    for i in 0..rows {
        state.push_message_entry(format!("row {i}"), false);
    }
    state
}

#[test]
fn an_unchanged_transcript_is_unchanged() {
    let state = state_with(5);
    let cursor = TranscriptCursor::of(&state);
    assert_eq!(cursor.diff_state(&state), TranscriptDiff::Unchanged);
    assert_eq!(
        cursor
            .diff_state(&state)
            .new_entries(state.message_entries()),
        Some(&[][..])
    );
}

#[test]
fn appended_rows_are_the_new_entries() {
    let mut state = state_with(5);
    let cursor = TranscriptCursor::of(&state);
    state.push_message_entry("new a", true);
    state.push_message_entry("new b", false);
    let diff = cursor.diff_state(&state);
    assert_eq!(
        diff,
        TranscriptDiff::Anchored {
            first_new: 5,
            replaced_last: false
        }
    );
    let new = diff.new_entries(state.message_entries()).unwrap();
    assert_eq!(new.len(), 2);
    assert!(new[0].is_command_echo);
}

#[test]
fn an_empty_cursor_takes_everything_as_new() {
    let empty = state_with(0);
    let cursor = TranscriptCursor::of(&empty);
    assert!(cursor.is_empty());
    let state = state_with(3);
    assert_eq!(
        cursor.diff_state(&state),
        TranscriptDiff::Anchored {
            first_new: 0,
            replaced_last: false
        }
    );
    assert_eq!(cursor.diff_state(&empty), TranscriptDiff::Unchanged);
}

#[test]
fn an_in_place_edit_replaces_the_last_entry() {
    let mut state = state_with(5);
    let cursor = TranscriptCursor::of(&state);
    state
        .message_transcript
        .last_mut()
        .unwrap()
        .text
        .push_str(" more");
    state.push_message_entry("after", false);
    assert_eq!(
        cursor.diff_state(&state),
        TranscriptDiff::Anchored {
            first_new: 4,
            replaced_last: true
        }
    );
}

#[test]
fn a_withdrawn_echo_anchors_with_nothing_new() {
    let mut state = state_with(5);
    state.push_message_entry("echo", true);
    let cursor = TranscriptCursor::of(&state);
    state.message_transcript.pop();
    let diff = cursor.diff_state(&state);
    assert_eq!(
        diff,
        TranscriptDiff::Anchored {
            first_new: 5,
            replaced_last: true
        }
    );
    assert_eq!(diff.new_entries(state.message_entries()).unwrap().len(), 0);
}

#[test]
fn a_clear_is_unanchored() {
    let mut state = state_with(5);
    let cursor = TranscriptCursor::of(&state);
    state.clear_message_window();
    assert_eq!(
        cursor.diff_state(&state),
        TranscriptDiff::Unanchored(Unanchored::Cleared)
    );
    state.push_message_entry("fresh", false);
    assert_eq!(
        cursor.diff_state(&state),
        TranscriptDiff::Unanchored(Unanchored::Cleared)
    );
    assert_eq!(
        cursor
            .diff_state(&state)
            .new_entries(state.message_entries()),
        None
    );
}

#[test]
fn a_rebuild_is_unanchored() {
    let state = state_with(5);
    let cursor = TranscriptCursor::of(&state);
    let mut rebuilt = test_state(open_grid(), 5, 5);
    rebuilt.clear_message_window();
    for i in 0..8 {
        rebuilt.push_message_entry(format!("other {i}"), false);
    }
    assert_eq!(
        cursor.diff_state(&rebuilt),
        TranscriptDiff::Unanchored(Unanchored::Rebuilt)
    );
}

#[test]
fn at_the_cap_a_unique_tail_still_anchors() {
    let mut state = state_with(MESSAGE_TRANSCRIPT_CAPACITY);
    let cursor = TranscriptCursor::of(&state);
    state.push_message_entry("x", false);
    state.push_message_entry("y", false);
    assert_eq!(state.message_entries().len(), MESSAGE_TRANSCRIPT_CAPACITY);
    let diff = cursor.diff_state(&state);
    assert_eq!(
        diff,
        TranscriptDiff::Anchored {
            first_new: MESSAGE_TRANSCRIPT_CAPACITY - 2,
            replaced_last: false
        }
    );
    let new = diff.new_entries(state.message_entries()).unwrap();
    assert_eq!(
        new.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
        ["x", "y"]
    );
}

#[test]
fn at_the_cap_a_scrolled_out_tail_is_unanchored() {
    let mut state = state_with(MESSAGE_TRANSCRIPT_CAPACITY);
    let cursor = TranscriptCursor::of(&state);
    for i in 0..MESSAGE_TRANSCRIPT_CAPACITY {
        state.push_message_entry(format!("flood {i}"), false);
    }
    assert_eq!(
        cursor.diff_state(&state),
        TranscriptDiff::Unanchored(Unanchored::Capped)
    );
}

#[test]
fn at_the_cap_a_repeated_tail_is_ambiguous() {
    let mut state = test_state(open_grid(), 5, 5);
    state.clear_message_window();
    for _ in 0..MESSAGE_TRANSCRIPT_CAPACITY {
        state.push_message_entry("same", false);
    }
    let cursor = TranscriptCursor::of(&state);
    state.push_message_entry("same", false);
    assert_eq!(
        cursor.diff_state(&state),
        TranscriptDiff::Unanchored(Unanchored::Ambiguous)
    );
}

#[test]
fn below_the_cap_a_tail_never_shifts() {
    // Below the cap nothing is dropped from the front, so a tail found
    // earlier than its own place is a different transcript.
    let state = state_with(6);
    let cursor = TranscriptCursor::of(&state);
    let mut other = test_state(open_grid(), 5, 5);
    other.clear_message_window();
    for i in 3..9 {
        other.push_message_entry(format!("row {i}"), false);
    }
    assert!(!cursor.diff_state(&other).is_anchored());
}

#[test]
fn the_cursor_debug_prints_no_text() {
    let state = state_with(4);
    let printed = format!("{:?}", TranscriptCursor::of(&state));
    assert!(!printed.contains("row"));
    assert!(printed.contains("len: 4"));
}

#[test]
fn the_cursor_is_copy_and_reads_only() {
    fn is_copy<T: Copy>() {}
    is_copy::<TranscriptCursor>();
    let state = state_with(4);
    let before = crate::enhancements::trajectory_digest(&state);
    let cursor = TranscriptCursor::of(&state);
    let _ = cursor.diff_state(&state);
    assert_eq!(crate::enhancements::trajectory_digest(&state), before);
}
