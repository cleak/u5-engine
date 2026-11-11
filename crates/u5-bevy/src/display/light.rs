//! Brightness on the map view (Phase B design chapter 9, 9.3.3; build plan
//! D-M2.2): the `[brightness]` pipeline step.
//!
//! This is the seam M2-01 adds. [`add_light`] is called from
//! [`DisplayPlugin`](super::DisplayPlugin)'s `build`; it registers nothing
//! yet. The step itself (M2-03) is registered only while
//! `display.brightness` is below 100, so with the key at its default no
//! step, table or system exists (9.4.2).

use bevy::prelude::*;
use u5_runtime::enhancements::DisplaySettings;

/// Registers the brightness step when `display.brightness` asks for it.
/// Adds nothing yet (M2-01): the step arrives with M2-03.
pub(crate) fn add_light(_app: &mut App, _settings: &DisplaySettings) {}
