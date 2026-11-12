//! The cells the player has seen, for the gem map's record (Phase B design
//! 10.2.5, 10.3.3 "dungeon cells seen" and "town-side cells seen"; build
//! plan M3-06).
//!
//! Two sources, both pure reads of `&PlayState`:
//!
//! - [`frame_seen_events`], every frame: in a dungeon, the cells the
//!   faithful corridor frame paints ([`dungeon_corridor_cells`], chapter 4,
//!   4.2.2); on the world, the unhidden cells of the faithful 11 x 11
//!   window (4.2.2, 4.3.3), kept only where they fall in a town's side
//!   window. Nothing is recorded while the faithful view is not the
//!   corridor or the map window (4.3.3): combat, an open view overlay, a
//!   shrine, the Codex, Blackthorn, the endgame, or a visibility grid that
//!   is dirty, not ready or still sweeping.
//! - [`gem_view_events`], when a gem view opens: a dungeon gem's flood
//!   cells (`view_overlay_footprint`), or an outdoor gem's drawn window,
//!   again kept only inside the side windows.
//!
//! Nothing else ever marks a side cell. In particular the grow-left ring
//! (chapter 2's widened view) is never read: it shows more than the
//! original does, and would make the gem map a free scouting tool (4.2.2).
//!
//! **The side window** of a town is 48 x 36 Britannia cells around its
//! gazetteer entrance (`published_world_location_entries`), cut open at the
//! entrance column as the town gem view draws it (10.2.5): columns 0..24
//! are the 24 world columns west of the entrance (column 23 touches the
//! floor's west edge), columns 24..48 the 24 east of it, and rows 0..36
//! are world rows `entrance_y - 18 ..= entrance_y + 17`. Index
//! `row * 48 + column` is the bit of tag 11 (10.3.7).

use std::sync::OnceLock;

use u5_runtime::{
    Area, PlayState, PlayTarget, VIEWPORT_PLAYER_COL, VIEWPORT_PLAYER_ROW, VIEWPORT_SIDE,
    VISIBILITY_HIDDEN, ViewOverlayFootprint, ViewOverlayKind, ViewOverlayMode, WORLD_SIDE,
    WorldPlane, published_world_location_entries, visibility_grid_active_index,
};

use super::knowledge::{KnowEvent, SIDE_WINDOW_HEIGHT, SIDE_WINDOW_WIDTH};
use super::probe::KnowEvents;
use crate::observe::dungeon_corridor_cells;

/// World columns on each side of the entrance in a side window.
pub const SIDE_HALF_WIDTH: u8 = SIDE_WINDOW_WIDTH / 2;
/// World rows above the entrance row in a side window.
pub const SIDE_ROWS_ABOVE: u8 = SIDE_WINDOW_HEIGHT / 2;

/// One town's side window: the scene and its gazetteer entrance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideWindow {
    pub scene: u8,
    pub plane: WorldPlane,
    pub entrance: (u8, u8),
}

impl SideWindow {
    /// The bit index of world cell `(x, y)` in this window, or `None`
    /// outside it (including the entrance column itself).
    pub fn index_of(&self, x: u8, y: u8) -> Option<u16> {
        let (ex, ey) = self.entrance;
        let dx = x.wrapping_sub(ex) as i8 as i16;
        let column = match dx {
            -24..=-1 => (dx + SIDE_HALF_WIDTH as i16) as u16,
            1..=24 => (dx - 1 + SIDE_HALF_WIDTH as i16) as u16,
            _ => return None,
        };
        let dy = y.wrapping_sub(ey) as i8 as i16;
        let row = dy + SIDE_ROWS_ABOVE as i16;
        if !(0..SIDE_WINDOW_HEIGHT as i16).contains(&row) {
            return None;
        }
        Some(row as u16 * SIDE_WINDOW_WIDTH as u16 + column)
    }

    /// The world cell of bit `index`, or `None` past the window.
    pub fn cell_of(&self, index: u16) -> Option<(u8, u8)> {
        let width = SIDE_WINDOW_WIDTH as u16;
        if index >= width * SIDE_WINDOW_HEIGHT as u16 {
            return None;
        }
        let (column, row) = ((index % width) as i16, (index / width) as i16);
        let dx = if column < SIDE_HALF_WIDTH as i16 {
            column - SIDE_HALF_WIDTH as i16
        } else {
            column - SIDE_HALF_WIDTH as i16 + 1
        };
        let dy = row - SIDE_ROWS_ABOVE as i16;
        let (ex, ey) = self.entrance;
        Some((
            (ex as i16 + dx).rem_euclid(WORLD_SIDE as i16) as u8,
            (ey as i16 + dy).rem_euclid(WORLD_SIDE as i16) as u8,
        ))
    }
}

/// Every town-family scene with exactly one gazetteer entrance, in table
/// order. A scene with more than one row has no single entrance to centre
/// a window on and gets none.
pub fn side_windows() -> &'static [SideWindow] {
    static WINDOWS: OnceLock<Vec<SideWindow>> = OnceLock::new();
    WINDOWS.get_or_init(|| {
        let entries = published_world_location_entries();
        let mut out: Vec<SideWindow> = Vec::new();
        for entry in &entries {
            let PlayTarget::Town(scene) = entry.target else {
                continue;
            };
            let rows = entries
                .iter()
                .filter(|other| other.target == entry.target)
                .count();
            if rows != 1 {
                continue;
            }
            out.push(SideWindow {
                scene: scene.byte,
                plane: entry.plane,
                entrance: (entry.x as u8, entry.y as u8),
            });
        }
        out
    })
}

/// The side window of `scene`, if it has one.
pub fn side_window(scene: u8) -> Option<&'static SideWindow> {
    side_windows().iter().find(|window| window.scene == scene)
}

/// The side-cell events for world cells seen on `plane`.
fn side_events(plane: WorldPlane, cells: impl IntoIterator<Item = (u8, u8)>) -> KnowEvents {
    let windows: Vec<&SideWindow> = side_windows()
        .iter()
        .filter(|window| window.plane == plane)
        .collect();
    let mut events = KnowEvents::new();
    for (x, y) in cells {
        for window in &windows {
            if let Some(index) = window.index_of(x, y) {
                events.push(KnowEvent::SideCellSeen {
                    scene: window.scene,
                    index,
                });
            }
        }
    }
    events
}

/// Whether the faithful view is the corridor or the map window, so its
/// cells count as seen (design 4.3.3).
pub fn observation_open(state: &PlayState) -> bool {
    if state.combat_active
        || state.active_view_overlay.is_some()
        || state.active_shrine.is_some()
        || state.codex_presentation_steps != 0
        || state.active_blackthorn.is_some()
        || state.endgame.is_some()
    {
        return false;
    }
    match state.area {
        Area::Dungeon { .. } => true,
        Area::Town { .. } | Area::World { .. } => {
            !state.visibility_dirty
                && state.visibility_buffers_ready
                && state.visibility_sweep.is_none()
        }
    }
}

/// The world cells of the faithful 11 x 11 window the visibility producer
/// did not hide (4.2.2), wrapped at 256.
pub fn faithful_window_cells(state: &PlayState) -> Vec<(u8, u8)> {
    let mut cells = Vec::new();
    for row in 0..VIEWPORT_SIDE {
        for col in 0..VIEWPORT_SIDE {
            let Some(at) = visibility_grid_active_index(row, col) else {
                continue;
            };
            if state.visibility_grid[at] == VISIBILITY_HIDDEN {
                continue;
            }
            let x = state.player.x as isize + col as isize - VIEWPORT_PLAYER_COL as isize;
            let y = state.player.y as isize + row as isize - VIEWPORT_PLAYER_ROW as isize;
            cells.push((
                x.rem_euclid(WORLD_SIDE as isize) as u8,
                y.rem_euclid(WORLD_SIDE as isize) as u8,
            ));
        }
    }
    cells
}

/// The cells seen on this frame (10.3.3): the corridor's in a dungeon, the
/// faithful window's side cells on the world, nothing in a town or while
/// the faithful view is something else.
pub fn frame_seen_events(state: &PlayState) -> KnowEvents {
    if !observation_open(state) {
        return KnowEvents::new();
    }
    match state.area {
        Area::Dungeon { .. } => {
            let Some(corridor) = dungeon_corridor_cells(state) else {
                return KnowEvents::new();
            };
            let mut rows = [0u8; 8];
            for (x, y) in corridor.positions() {
                rows[y as usize & 7] |= 1 << (x & 7);
            }
            vec![KnowEvent::DungeonCellsSeen {
                record: corridor.record,
                level: corridor.level,
                rows,
            }]
        }
        Area::World { plane } => side_events(plane, faithful_window_cells(state)),
        Area::Town { .. } => KnowEvents::new(),
    }
}

/// The outdoor gem map's drawn window: the cells from `half_cols` west to
/// `half_cols` east of the party and `half_rows` north to `half_rows`
/// south, inclusive, wrapping. The renderer passes the cells its canvas
/// touches (10.2.4: `W / 4` by `H / 4` at 4 px, centred on the party).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GemWindow {
    pub half_cols: u16,
    pub half_rows: u16,
}

/// The cells a gem view showed, on the frame it is open (10.3.3): a
/// dungeon gem's flood cells, or the outdoor window's side cells (the
/// faithful 32 x 32 footprint, plus `window` when the enhanced map drew a
/// wider one). Peer, X-Ray and the sky record nothing.
pub fn gem_view_events(state: &PlayState, window: Option<GemWindow>) -> KnowEvents {
    let Some(overlay) = state.active_view_overlay.as_ref() else {
        return KnowEvents::new();
    };
    if overlay.mode != ViewOverlayMode::GemView {
        return KnowEvents::new();
    }
    match (state.area, state.view_overlay_footprint()) {
        (Area::Dungeon { scene, .. }, Some(ViewOverlayFootprint::Dungeon { level, cells })) => {
            let mut rows = [0u8; 8];
            for (x, y) in cells {
                rows[y as usize & 7] |= 1 << (x & 7);
            }
            vec![KnowEvent::DungeonCellsSeen {
                record: scene.record as u8,
                level,
                rows,
            }]
        }
        (Area::World { plane }, Some(ViewOverlayFootprint::Surface { cells }))
            if overlay.kind == ViewOverlayKind::Surface =>
        {
            let mut all = cells;
            if let Some(window) = window {
                let (px, py) = (state.player.x as isize, state.player.y as isize);
                let (hc, hr) = (
                    (window.half_cols as isize).min(WORLD_SIDE as isize / 2),
                    (window.half_rows as isize).min(WORLD_SIDE as isize / 2),
                );
                for dy in -hr..=hr {
                    for dx in -hc..=hc {
                        all.push((
                            (px + dx).rem_euclid(WORLD_SIDE as isize) as u8,
                            (py + dy).rem_euclid(WORLD_SIDE as isize) as u8,
                        ));
                    }
                }
                all.sort_unstable();
                all.dedup();
            }
            side_events(plane, all)
        }
        _ => KnowEvents::new(),
    }
}

#[cfg(test)]
#[path = "seen_tests.rs"]
mod tests;
