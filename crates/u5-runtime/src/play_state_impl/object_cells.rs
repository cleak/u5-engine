//! Which map the viewport shows (Phase B design 9.3.7).
//!
//! A child module of `chunk_09`, as the ring peek is, so later packages
//! can read the top-down buffers without making them public. It holds
//! [`MapViewKind`] and [`PlayState::map_view_kind`], the one predicate the
//! grow-left layout (2.2.8), the brightness step (9.3.3) and the sprite
//! composite (9.3.8) share.

use crate::*;

/// The kind of map the 11x11 viewport shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapViewKind {
    /// The top-down world (either plane).
    World,
    /// The top-down town, castle, keep or village.
    Town,
    /// A combat arena.
    Combat,
    /// The first-person dungeon view.
    Dungeon,
}

impl MapViewKind {
    /// The plain top-down view, world or town (2.2.8).
    pub fn is_top_down(self) -> bool {
        matches!(self, Self::World | Self::Town)
    }
}

impl PlayState {
    /// Which map the viewport shows, if any. Read-only; takes `&self` and
    /// draws nothing from the RNG.
    ///
    /// It mirrors the branch order of the shell's base-frame render: the
    /// pending Blackthorn rescue, then the audience, then the shrine
    /// presentation, all `None`; then
    /// [`PlayState::render_top_down_base_frame`], which draws combat first
    /// ([`MapViewKind::Combat`]), then the dungeon when there is no
    /// top-down render area ([`MapViewKind::Dungeon`]), otherwise the world
    /// or a town. It is also `None` while a view overlay (the gem view and
    /// its kin), the endgame or a visibility sweep is present, since those
    /// keep the faithful frame.
    pub fn map_view_kind(&self) -> Option<MapViewKind> {
        // The rescue's own render. During its live-grid stretch it draws
        // nothing and the ordinary map shows, so the switch to the fixed
        // screen comes on the first frame the rescue paints (2.2.8);
        // otherwise the held black field, a queued playback or the
        // running cinematic owns the viewport.
        let rescue_live_grid = self.blackthorn_rescue_active()
            && self.blackthorn_rescue_viewport
                == crate::blackthorn::BlackthornRescueViewport::LiveGrid;
        if !rescue_live_grid
            && (!self.pending_blackthorn_rescue_playbacks.is_empty()
                || self.blackthorn_rescue_active()
                || self.cutscene_black_viewport)
        {
            return None;
        }
        if self.blackthorn_audience_map.is_some() || self.shrine_presentation_map.is_some() {
            return None;
        }
        if self.active_view_overlay.is_some()
            || self.endgame.is_some()
            || self.visibility_sweep.is_some()
        {
            return None;
        }
        // `render_top_down_viewport`: combat first, then the dungeon when
        // there is no top-down render area.
        if self.combat_active {
            return Some(MapViewKind::Combat);
        }
        Some(match self.top_down_render_area() {
            None => MapViewKind::Dungeon,
            Some(TopDownRenderArea::World(_)) => MapViewKind::World,
            Some(TopDownRenderArea::Town) => MapViewKind::Town,
        })
    }
}
