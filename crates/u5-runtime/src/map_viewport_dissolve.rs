//! Blocking map-viewport dissolves shared by Blackthorn rescue and dungeon Search.

use crate::{DisplayPixelRect, DungeonScene, PlayState, RectangleDissolve};

/// `display-driver-abi.md §9.6`: inclusive gameplay viewport rectangle used by
/// all three published map-viewport callers.
pub const MAP_VIEWPORT_DISSOLVE_RECT: DisplayPixelRect = DisplayPixelRect {
    x0: 8,
    y0: 8,
    x1: 183,
    y1: 183,
};

/// `blackthorn.md §7`: the party tile in the rescue dissolve-in is centred in
/// the eleven-by-eleven map viewport.
pub const BLACKTHORN_RESCUE_PARTY_CELL: (u8, u8) = (5, 5);

/// Caller-composed hidden surface consumed by one map-viewport dissolve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapViewportDissolveSource {
    /// Rescue/refuge entry: colour zero only, before scratch-state clearing.
    BlackthornRescueBlack,
    /// Rescue/refuge exit: colour zero plus the on-foot party tile at `(5,5)`.
    BlackthornRescuePartyOnBlack { cell: (u8, u8) },
    /// Lit dungeon Search: the first-person view after the visit-local rewrite.
    DungeonSearchReveal {
        scene: DungeonScene,
        level: u8,
        x: u8,
        y: u8,
        original_cell: u8,
        revealed_cell: u8,
    },
}

/// Completed execution record for one self-paced blocking driver call.
///
/// This mirrors the runtime's other completed blocking playback records. The
/// driver visit order is actually exhausted here; frontends need only present
/// the caller-composed end state after the call returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapViewportDissolvePlayback {
    pub source: MapViewportDissolveSource,
    pub rect: DisplayPixelRect,
    pub copied_pixels: usize,
    pub world_ticks_advanced: u8,
    pub caller_redraws_during_dissolve: u8,
}

pub fn run_map_viewport_dissolve(source: MapViewportDissolveSource) -> MapViewportDissolvePlayback {
    let rect = MAP_VIEWPORT_DISSOLVE_RECT;
    let mut dissolve = RectangleDissolve::new((
        rect.x0 as u16,
        rect.y0 as u16,
        rect.x1 as u16,
        rect.y1 as u16,
    ))
    .expect("published map viewport dissolve rectangle is valid");
    let mut copied_pixels = 0usize;
    while dissolve.next_pixel().is_some() {
        copied_pixels += 1;
    }
    debug_assert_eq!(
        copied_pixels,
        MAP_VIEWPORT_DISSOLVE_RECT.width() * MAP_VIEWPORT_DISSOLVE_RECT.height()
    );
    MapViewportDissolvePlayback {
        source,
        rect,
        copied_pixels,
        world_ticks_advanced: 0,
        caller_redraws_during_dissolve: 0,
    }
}

impl PlayState {
    pub(crate) fn run_map_viewport_dissolve(&mut self, source: MapViewportDissolveSource) {
        self.pending_map_viewport_dissolves
            .push(run_map_viewport_dissolve(source));
    }

    /// Drain the blocking calls completed since the frontend last presented
    /// them. Multiple calls retain caller order (the rescue path queues two).
    pub fn take_pending_map_viewport_dissolves(&mut self) -> Vec<MapViewportDissolvePlayback> {
        std::mem::take(&mut self.pending_map_viewport_dissolves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_viewport_dissolve_exhausts_the_exact_inclusive_rectangle() {
        let playback = run_map_viewport_dissolve(MapViewportDissolveSource::BlackthornRescueBlack);
        assert_eq!(playback.rect, MAP_VIEWPORT_DISSOLVE_RECT);
        assert_eq!(playback.copied_pixels, 176 * 176);
        assert_eq!(playback.world_ticks_advanced, 0);
        assert_eq!(playback.caller_redraws_during_dissolve, 0);
    }
}
