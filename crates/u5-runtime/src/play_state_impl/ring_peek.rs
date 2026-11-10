//! The ring peek: a read-only look at the top-down cells around the
//! faithful 11x11 window, for the grow-left layout (Phase B design 2.3.6).
//!
//! Declared as a child module of `chunk_09`, so the peek can use that
//! chunk's private top-down helpers with no visibility change. For now it
//! holds only the light policy the `display.grow_left_light` setting
//! chooses; the peek types and `peek_top_down_ring` arrive with the ring
//! peek packages.

/// How far daylight reaches in the wider ring (2.2.11, D2.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeekLightPolicy {
    /// Daylight reaches the whole canvas.
    #[default]
    Scaled,
    /// Daylight reaches only the original distance.
    Faithful,
}
