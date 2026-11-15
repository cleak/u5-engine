//! Automatic combat in the Bevy shell (Phase B design 6.3.4; build plan
//! M7-05).
//!
//! The planner lives in `u5_runtime::auto_combat` and answers with an
//! [`AutoKey`]. The shell types that key into the game exactly as a key
//! the player typed: one [`TypedKey`] pushed into
//! `VisualState::typed_keys` in `EnhSet::Feed`, which `drive_visual` then
//! drains through `visual_play_command_char` and `handle_play_key_input`,
//! the per-key path of every typed key (the foundation's synthetic-input
//! rule, R3.5). [`auto_typed_key`] is the one table between the two; its
//! test pins `visual_play_command_char` of each mapped key to
//! [`AutoKey::input_char`], so the terminal and the window cannot drift.

use bevy::prelude::*;
use u5_runtime::InputDirection;
use u5_runtime::auto_combat::AutoKey;

use crate::TypedKey;

/// The physical key the shell types for `key` (6.3.4): no modifier, so
/// the command prompt and the targeting cursor read it as the player's
/// own keypress.
///
/// | AutoKey | Key |
/// |---|---|
/// | Attack | A |
/// | Pass | Space |
/// | Step / Cursor | arrows, Home, PgUp, End, PgDn |
/// | Confirm | Enter |
/// | Cancel | Escape |
// The feed (M7-06) is the first caller outside the tests.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn auto_typed_key(key: AutoKey) -> TypedKey {
    let code = match key {
        AutoKey::Attack => KeyCode::KeyA,
        AutoKey::Pass => KeyCode::Space,
        AutoKey::Step(direction) | AutoKey::Cursor(direction) => match direction {
            InputDirection::North => KeyCode::ArrowUp,
            InputDirection::South => KeyCode::ArrowDown,
            InputDirection::East => KeyCode::ArrowRight,
            InputDirection::West => KeyCode::ArrowLeft,
            InputDirection::Northwest => KeyCode::Home,
            InputDirection::Northeast => KeyCode::PageUp,
            InputDirection::Southwest => KeyCode::End,
            InputDirection::Southeast => KeyCode::PageDown,
        },
        AutoKey::Confirm => KeyCode::Enter,
        AutoKey::Cancel => KeyCode::Escape,
    };
    TypedKey {
        key: code,
        shift: false,
        control: false,
    }
}

#[cfg(test)]
mod feed_tests;
