//! Stonegate's trapdoor scripted-death presentation contract.

use crate::{DisplayPixelRect, PlayState};

/// Inclusive map viewport cleared before the Stonegate trapdoor tableau.
pub const STONEGATE_TRAPDOOR_VIEWPORT_RECT: DisplayPixelRect = DisplayPixelRect {
    x0: 8,
    y0: 8,
    x1: 183,
    y1: 183,
};

pub const STONEGATE_TRAPDOOR_BLACK_COLOUR: u8 = 0;
pub const STONEGATE_TRAPDOOR_GRID_TILE: u8 = 0x8f;
pub const STONEGATE_TRAPDOOR_DESCENDING_TONE_FIRST: u16 = 1_000;
pub const STONEGATE_TRAPDOOR_DESCENDING_TONE_LAST: u16 = 251;
pub const STONEGATE_TRAPDOOR_DESCENDING_TONE_STEPS: u16 = 750;
pub const STONEGATE_TRAPDOOR_DESCENDING_TONE_PACING: (u16, u16) = (40, 1);
pub const STONEGATE_TRAPDOOR_RUMBLE_MIN: u16 = 100;
pub const STONEGATE_TRAPDOOR_RUMBLE_MAX: u16 = 500;
pub const STONEGATE_TRAPDOOR_RUMBLE_FRAGMENT_UNITS: u16 = 40;
pub const STONEGATE_TRAPDOOR_RUMBLE_BUDGET_UNITS: u16 = 3_000;

/// Completed blocking presentation record waiting for a frontend.
///
/// The original sequence has no resumable state and advances no gameplay
/// time. Frontends without a PC-speaker backend can still reproduce the exact
/// viewport clear and acknowledge the two published sound envelopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StonegateTrapdoorPlayback {
    pub viewport_rect: DisplayPixelRect,
    pub viewport_fill_colour: u8,
    pub descending_tone_first: u16,
    pub descending_tone_last: u16,
    pub descending_tone_steps: u16,
    pub descending_tone_pacing: (u16, u16),
    pub rumble_min: u16,
    pub rumble_max: u16,
    pub rumble_fragment_units: u16,
    pub rumble_budget_units_per_member: u16,
    pub party_slots_visited: usize,
    pub stats_panel_repaints: usize,
    pub gameplay_minutes_advanced: u8,
    pub uses_dissolve: bool,
}

impl StonegateTrapdoorPlayback {
    pub const fn complete(party_slots: usize) -> Self {
        Self {
            viewport_rect: STONEGATE_TRAPDOOR_VIEWPORT_RECT,
            viewport_fill_colour: STONEGATE_TRAPDOOR_BLACK_COLOUR,
            descending_tone_first: STONEGATE_TRAPDOOR_DESCENDING_TONE_FIRST,
            descending_tone_last: STONEGATE_TRAPDOOR_DESCENDING_TONE_LAST,
            descending_tone_steps: STONEGATE_TRAPDOOR_DESCENDING_TONE_STEPS,
            descending_tone_pacing: STONEGATE_TRAPDOOR_DESCENDING_TONE_PACING,
            rumble_min: STONEGATE_TRAPDOOR_RUMBLE_MIN,
            rumble_max: STONEGATE_TRAPDOOR_RUMBLE_MAX,
            rumble_fragment_units: STONEGATE_TRAPDOOR_RUMBLE_FRAGMENT_UNITS,
            rumble_budget_units_per_member: STONEGATE_TRAPDOOR_RUMBLE_BUDGET_UNITS,
            party_slots_visited: party_slots,
            stats_panel_repaints: party_slots,
            gameplay_minutes_advanced: 0,
            uses_dissolve: false,
        }
    }
}

impl PlayState {
    pub fn take_pending_stonegate_trapdoor_playback(
        &mut self,
    ) -> Option<StonegateTrapdoorPlayback> {
        self.pending_stonegate_trapdoor_playback.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_records_exact_non_dissolving_blocking_sequence() {
        let playback = StonegateTrapdoorPlayback::complete(6);
        assert_eq!(playback.viewport_rect.width(), 176);
        assert_eq!(playback.viewport_rect.height(), 176);
        assert_eq!(playback.viewport_fill_colour, 0);
        assert_eq!(playback.descending_tone_first, 1_000);
        assert_eq!(playback.descending_tone_last, 251);
        assert_eq!(playback.descending_tone_steps, 750);
        assert_eq!(playback.descending_tone_pacing, (40, 1));
        assert_eq!(playback.rumble_min, 100);
        assert_eq!(playback.rumble_max, 500);
        assert_eq!(playback.rumble_fragment_units, 40);
        assert_eq!(playback.rumble_budget_units_per_member, 3_000);
        assert_eq!(playback.party_slots_visited, 6);
        assert_eq!(playback.stats_panel_repaints, 6);
        assert_eq!(playback.gameplay_minutes_advanced, 0);
        assert!(!playback.uses_dissolve);
    }
}
