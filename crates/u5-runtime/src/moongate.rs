//! Natural-moongate live-tile refresh helpers per `overworld.md` §9.

use crate::NATURAL_MOONGATE_COUNTER_MAX;

/// `overworld.md §9` shared natural-moongate gate-presence counter
/// outcome for one cleanup pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NaturalMoongateCounterStep {
    /// Night hours `20..=23` and `0..=4`: counter increases toward
    /// [`NATURAL_MOONGATE_COUNTER_MAX`].
    Increase,
    /// Daytime hours `5..=19`: counter decreases toward zero.
    Decrease,
}

/// `overworld.md §9`: classify the cleanup pass for the shared
/// gate-presence counter from the current hour. Hours `20..=23` and
/// `0..=4` (inclusive) are the night band that grows the counter; all
/// other hours shrink it.
pub const fn natural_moongate_counter_step(hour: u8) -> NaturalMoongateCounterStep {
    if hour >= 20 || hour <= 4 {
        NaturalMoongateCounterStep::Increase
    } else {
        NaturalMoongateCounterStep::Decrease
    }
}

/// `overworld.md §9`: advance the shared counter for one cleanup pass.
/// Increases saturate at [`NATURAL_MOONGATE_COUNTER_MAX`]; decreases
/// floor at zero.
pub const fn natural_moongate_advance_counter(counter: u8, hour: u8) -> u8 {
    match natural_moongate_counter_step(hour) {
        NaturalMoongateCounterStep::Increase => {
            if counter >= NATURAL_MOONGATE_COUNTER_MAX {
                NATURAL_MOONGATE_COUNTER_MAX
            } else {
                counter + 1
            }
        }
        NaturalMoongateCounterStep::Decrease => {
            if counter == 0 {
                0
            } else {
                counter - 1
            }
        }
    }
}

/// `overworld.md §9` saved-Moonstone-slot eligibility check for the
/// natural-moongate live-tile refresh. Surface (overworld) eligibility
/// requires `slot_scene == current_scene`, `slot_z == current_z`, and
/// the saved `(slot_x, slot_y)` falling inside the active 32-by-32
/// loaded chunk window. Interior and town-family non-combat scenes use
/// only the scene/Z match (`window` is then `None`).
pub fn natural_moongate_slot_eligible(
    slot_scene: u8,
    slot_z: u8,
    slot_x: u8,
    slot_y: u8,
    current_scene: u8,
    current_z: u8,
    chunk_window: Option<(u8, u8, u8, u8)>,
) -> bool {
    if slot_scene != current_scene || slot_z != current_z {
        return false;
    }
    let Some((x0, y0, w, h)) = chunk_window else {
        return true;
    };
    slot_x >= x0 && slot_x < x0.saturating_add(w) && slot_y >= y0 && slot_y < y0.saturating_add(h)
}

/// `overworld.md §9`: live-gate entry hook secondary outcome — when the
/// hook clears the `0xDC` cell, an hour-`0` minute-`<10` window
/// dispatches to the shrine/urn meditate overlay; otherwise the cached
/// moon-glyph slot warps the party. Returns `true` for the meditate
/// dispatch.
pub const fn natural_moongate_dispatches_meditate(hour: u8, minute: u8) -> bool {
    hour == 0 && minute < 10
}

/// `overworld.md §9`: pick the cached-moon-glyph slot the live-gate
/// entry hook reads when warping after clearing the `0xDC` cell.
/// Before noon (`hour < 12`) the hook reads the first cached glyph;
/// from noon onward it reads the second. Returns `0` for first glyph,
/// `1` for second.
pub const fn natural_moongate_cached_glyph_slot(hour: u8) -> u8 {
    if hour < 12 { 0 } else { 1 }
}

/// `overworld.md §9` fixed narrative gate location: surface plane
/// world coordinate `(233, 235)` for the post-action special-tile
/// branch.
pub const NARRATIVE_GATE_X: u8 = 233;
pub const NARRATIVE_GATE_Y: u8 = 235;
