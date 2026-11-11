//! Sprite compositing (Phase B design chapter 9, 9.3.5-9.3.9; build plan
//! D-M2.2): the `[composite]` pipeline step and its sprite masks.
//!
//! This is the seam M2-01 adds. [`add_composite`] is called from
//! [`DisplayPlugin`](super::DisplayPlugin)'s `build`; it registers nothing
//! yet. The step (M2-06) is registered only while
//! `display.sprite_composite` is on, so with it off there is no step, no
//! mask resource and no call into the runtime's object-cell peek (9.4.2).

use bevy::prelude::*;
use u5_runtime::enhancements::DisplaySettings;

/// Registers the compositing step when `display.sprite_composite` is on.
/// Adds nothing yet (M2-01).
pub(crate) fn add_composite(_app: &mut App, _settings: &DisplaySettings) {}
