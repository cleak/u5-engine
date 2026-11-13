//! The gem map's shown set: the cells it draws and the tile drawn at each
//! (Phase B design 10.2.3; build plan M3-10, D-M3.5).
//!
//! **Outdoors** the shown set is every cell of the canvas window around the
//! party ([`OutdoorLayout`]), wrapping at 256 on both axes, on whichever
//! plane the party is on (Britannia or the Underworld). Each cell's tile is
//! `gem_sample_tile(x, y, terrain_only = true)`: the live terrain after the
//! static-tile animation, never a tile taken from an active object, so no
//! person, monster or hidden thing reaches the map (D-M3.5). The live
//! moongate stays, because it is written into the world grid, not drawn as
//! an object. The party's own ship or mount shows through the party marker.
//!
//! The set is computed once when the view opens and kept in
//! [`GemViewState`] (10.3.4 step 1); the render only reads it. Every read
//! here is `&PlayState`.
//!
//! **In a town** ([`TownShown`], 10.2.5, M3-12) the shown set is the whole
//! 32 x 32 floor, each cell `gem_sample_tile(x, y, terrain_only = true)`:
//! the live terrain only, so no townsfolk or other object is drawn
//! (10.2.12, pending D0.2). Beside it lies the Britannia land around the
//! town's gazetteer entrance, cut open at the entrance column, from the
//! static world map (`StaticMaps`), and only the cells the record says the
//! party has seen (tag 11; 10.2.5 "only land the party has seen"). Every
//! other side cell is black: a cell outside the side window, a cell never
//! seen, every cell with `gem_map.town_surroundings` off, and every cell
//! when the world map could not be read. The strips above and below the
//! floor show the floor's south-east corner cell, as the world-tile getter
//! returns it past the floor edge (`town-mode.md §15`, `visibility.md §3`).
//!
//! **In a dungeon** ([`DungeonShown`], 10.2.6, M3-13) the shown set is the
//! gem's flood on the current level plus the remembered cells of every
//! level, under the glyph policy; see [`super::dungeon`].

use u5_runtime::{Area, PlayState, ViewOverlayKind, ViewOverlayMode, WORLD_SIDE, WorldPlane};

use super::dungeon::{DungeonInputs, DungeonShown};
use super::knowledge::GemKnowledge;
use super::layout::{
    DungeonLayout, GemCanvas, OutdoorLayout, TOWN_ENTRANCE_FLOOR_ROW, TOWN_FLOOR_CELLS, TownLayout,
};
use super::seen::side_window;

/// The outdoor shown set (10.2.3, 10.2.4): one drawn tile per canvas cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutdoorShown {
    pub plane: WorldPlane,
    /// The party's world cell, the window's centre.
    pub party: (u8, u8),
    pub layout: OutdoorLayout,
    /// The drawn tiles, row-major over the layout's `dx` and `dy` spans.
    tiles: Vec<u8>,
}

impl OutdoorShown {
    /// The shown set for the party's current window, or `None` off the
    /// world map.
    pub fn sample(state: &PlayState, layout: OutdoorLayout) -> Option<Self> {
        let Area::World { plane } = state.area else {
            return None;
        };
        let (cols, rows) = layout.drawn();
        let mut tiles = Vec::with_capacity(cols as usize * rows as usize);
        let (px, py) = (state.player.x as isize, state.player.y as isize);
        for dy in layout.dy.0..=layout.dy.1 {
            for dx in layout.dx.0..=layout.dx.1 {
                tiles.push(state.gem_sample_tile(px + dx as isize, py + dy as isize, true));
            }
        }
        Some(Self {
            plane,
            party: (wrap(px), wrap(py)),
            layout,
            tiles,
        })
    }

    /// The drawn tile at offset `(dx, dy)` from the party, or `None`
    /// outside the window.
    pub fn tile(&self, dx: i32, dy: i32) -> Option<u8> {
        let (cols, _) = self.layout.drawn();
        if !(self.layout.dx.0..=self.layout.dx.1).contains(&dx)
            || !(self.layout.dy.0..=self.layout.dy.1).contains(&dy)
        {
            return None;
        }
        let col = (dx - self.layout.dx.0) as usize;
        let row = (dy - self.layout.dy.0) as usize;
        self.tiles.get(row * cols as usize + col).copied()
    }

    /// The world cell at offset `(dx, dy)`, wrapped.
    pub fn world_cell(&self, dx: i32, dy: i32) -> (u8, u8) {
        (
            wrap(self.party.0 as isize + dx as isize),
            wrap(self.party.1 as isize + dy as isize),
        )
    }

    /// Every shown cell as `(dx, dy, tile)`, row-major.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32, u8)> + '_ {
        let (cols, _) = self.layout.drawn();
        let (x0, y0) = (self.layout.dx.0, self.layout.dy.0);
        self.tiles.iter().enumerate().map(move |(index, &tile)| {
            let index = index as i32;
            (x0 + index % cols as i32, y0 + index / cols as i32, tile)
        })
    }

    /// Whether world cell `(x, y)` is in the shown set.
    pub fn contains(&self, x: u8, y: u8) -> bool {
        self.offset(x, y).is_some()
    }

    /// The offset `(dx, dy)` from the party at which world cell `(x, y)` is
    /// drawn, or `None` outside the window. The nearer way round the wrap
    /// wins when both are in the window.
    pub fn offset(&self, x: u8, y: u8) -> Option<(i32, i32)> {
        let offset = |cell: u8, party: u8, span: (i32, i32)| {
            let d = i32::from(cell.wrapping_sub(party));
            // Try the offset both ways round the 256-cell wrap.
            [d - WORLD_SIDE as i32, d]
                .into_iter()
                .filter(|d| (span.0..=span.1).contains(d))
                .min_by_key(|d| d.abs())
        };
        Some((
            offset(x, self.party.0, self.layout.dx)?,
            offset(y, self.party.1, self.layout.dy)?,
        ))
    }
}

fn wrap(value: isize) -> u8 {
    value.rem_euclid(WORLD_SIDE as isize) as u8
}

/// Which part of a town view a cell is (10.2.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TownPart {
    /// The 32 x 32 floor, at the map brightness.
    Floor,
    /// Above or below the floor: the floor's south-east corner cell.
    Strip,
    /// Beside the floor: the land around the entrance, if seen.
    Side,
}

/// What a town view reads besides the game (10.3.4 step 1): the record's
/// seen side cells, the static world map of the town's plane
/// ([`town_world_plane`]), and `gem_map.town_surroundings`. The default
/// reads nothing, so every side cell is black.
#[derive(Clone, Copy, Debug, Default)]
pub struct TownInputs<'a> {
    pub know: Option<&'a GemKnowledge>,
    /// The decoded 256 x 256 world map, row-major.
    pub world: Option<&'a [u8]>,
    pub surroundings: bool,
}

/// The world plane whose static map a town view's sides need: the plane
/// of the town's side window, when the party is in a town that has one.
pub fn town_world_plane(state: &PlayState) -> Option<WorldPlane> {
    let Area::Town { scene, .. } = state.area else {
        return None;
    };
    side_window(scene.byte).map(|window| window.plane)
}

/// The town shown set (10.2.3, 10.2.5): one drawn tile, or black, per
/// canvas cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TownShown {
    pub scene: u8,
    pub floor: i8,
    /// The party's floor cell.
    pub party: (u8, u8),
    pub layout: TownLayout,
    /// The floor's south-east corner cell, drawn in the strips.
    pub corner: u8,
    /// The drawn tiles, row-major over the layout's columns and rows;
    /// `None` is black.
    tiles: Vec<Option<u8>>,
}

impl TownShown {
    /// The shown set of the floor the party is on, or `None` outside a
    /// town or on a canvas too small for the floor.
    pub fn sample(state: &PlayState, canvas: GemCanvas, inputs: TownInputs<'_>) -> Option<Self> {
        let Area::Town { scene, floor } = state.area else {
            return None;
        };
        let layout = TownLayout::new(canvas)?;
        let last = TOWN_FLOOR_CELLS as isize - 1;
        let corner = state.gem_sample_tile(last, last, true);
        let window = side_window(scene.byte);
        let world = inputs
            .world
            .filter(|world| world.len() == WORLD_SIDE * WORLD_SIDE && inputs.surroundings);
        let side_tile = |fx: i32, fy: i32| -> Option<u8> {
            let (window, world, know) = (window?, world?, inputs.know?);
            let dx = TownLayout::entrance_dx(fx)?;
            let dy = fy - TOWN_ENTRANCE_FLOOR_ROW;
            let (ex, ey) = window.entrance;
            let x = wrap(ex as isize + dx as isize);
            let y = wrap(ey as isize + dy as isize);
            let index = window.index_of(x, y)?;
            know.side_seen(scene.byte, index)
                .then(|| {
                    world
                        .get(usize::from(y) * WORLD_SIDE + usize::from(x))
                        .copied()
                })
                .flatten()
        };
        let mut tiles = Vec::new();
        for fy in layout.rows.0..=layout.rows.1 {
            for fx in layout.cols.0..=layout.cols.1 {
                tiles.push(match town_part(fx, fy) {
                    TownPart::Floor => Some(state.gem_sample_tile(fx as isize, fy as isize, true)),
                    TownPart::Strip => Some(corner),
                    TownPart::Side => side_tile(fx, fy),
                });
            }
        }
        Some(Self {
            scene: scene.byte,
            floor,
            party: (state.player.x as u8, state.player.y as u8),
            layout,
            corner,
            tiles,
        })
    }

    /// The drawn tile at floor cell `(fx, fy)`: `None` for a black cell or
    /// one off the canvas.
    pub fn tile(&self, fx: i32, fy: i32) -> Option<u8> {
        let (cols, rows) = (self.layout.cols, self.layout.rows);
        if !(cols.0..=cols.1).contains(&fx) || !(rows.0..=rows.1).contains(&fy) {
            return None;
        }
        let width = (cols.1 - cols.0 + 1) as usize;
        let at = (fy - rows.0) as usize * width + (fx - cols.0) as usize;
        self.tiles.get(at).copied().flatten()
    }

    /// The world plane and cell a side cell `(fx, fy)` shows, or `None`
    /// for a floor or strip cell, a town with no side window, or a cell
    /// past the window. Whether the cell is drawn is [`TownShown::tile`].
    pub fn side_world(&self, fx: i32, fy: i32) -> Option<(WorldPlane, u8, u8)> {
        if town_part(fx, fy) != TownPart::Side {
            return None;
        }
        let window = side_window(self.scene)?;
        let dx = TownLayout::entrance_dx(fx)?;
        let dy = fy - TOWN_ENTRANCE_FLOOR_ROW;
        let (ex, ey) = window.entrance;
        let x = wrap(ex as isize + dx as isize);
        let y = wrap(ey as isize + dy as isize);
        window.index_of(x, y)?;
        Some((window.plane, x, y))
    }

    /// Every canvas cell as `(fx, fy, part, tile)`, row-major.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32, TownPart, Option<u8>)> + '_ {
        let (cols, rows) = (self.layout.cols, self.layout.rows);
        let width = cols.1 - cols.0 + 1;
        self.tiles.iter().enumerate().map(move |(index, &tile)| {
            let index = index as i32;
            let (fx, fy) = (cols.0 + index % width, rows.0 + index / width);
            (fx, fy, town_part(fx, fy), tile)
        })
    }
}

/// The part of a town view floor cell `(fx, fy)` is in.
pub fn town_part(fx: i32, fy: i32) -> TownPart {
    let on = |v: i32| (0..TOWN_FLOOR_CELLS).contains(&v);
    match (on(fx), on(fy)) {
        (true, true) => TownPart::Floor,
        (true, false) => TownPart::Strip,
        (false, _) => TownPart::Side,
    }
}

/// The shown set of one open view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GemShown {
    /// Nothing to draw: no gem view.
    Empty,
    Outdoor(OutdoorShown),
    Town(TownShown),
    Dungeon(DungeonShown),
}

/// One open gem view's state (10.3.2): the canvas, the cached shown set
/// and the inputs fixed when it opened. The shell builds it on open and on
/// resize; [`render_gem_map`](super::raster::render_gem_map) only reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GemViewState {
    pub canvas: GemCanvas,
    /// `display.map_brightness` when the view opened, for
    /// `gem_map.brightness_link` (10.2.10). 100 means off.
    pub display_map_brightness: u8,
    /// The dungeon level page on a lone panel: the shown level's offset
    /// from the party's, 0 on open (10.2.6, M3-14). The shell changes it
    /// with PgUp and PgDn through [`DungeonShown::page_step`].
    ///
    /// [`DungeonShown::page_step`]: super::dungeon::DungeonShown::page_step
    pub page: i8,
    /// The sweep step to draw, when a sweep runs (M3-11).
    pub sweep_step: Option<u8>,
    pub shown: GemShown,
}

impl GemViewState {
    /// The state for the view now open on `state`, at `canvas`, with no
    /// town inputs: a town view's sides are all black. The shown set is
    /// empty unless a gem view (not Peer, X-Ray or the sky) is up.
    pub fn open(state: &PlayState, canvas: GemCanvas, display_map_brightness: u8) -> Self {
        Self::open_with(state, canvas, display_map_brightness, TownInputs::default())
    }

    /// [`GemViewState::open`] with what a town view reads besides the
    /// game (10.2.5).
    pub fn open_with(
        state: &PlayState,
        canvas: GemCanvas,
        display_map_brightness: u8,
        town: TownInputs<'_>,
    ) -> Self {
        Self::open_all(
            state,
            canvas,
            display_map_brightness,
            town,
            DungeonInputs::default(),
        )
    }

    /// [`GemViewState::open_with`] with what a dungeon view reads besides
    /// the game (10.2.6): the record and the glyph policy.
    pub fn open_all(
        state: &PlayState,
        canvas: GemCanvas,
        display_map_brightness: u8,
        town: TownInputs<'_>,
        dungeon: DungeonInputs<'_>,
    ) -> Self {
        let shown =
            if gem_view_open(state) {
                match state.area {
                    Area::World { .. } => OutdoorShown::sample(state, OutdoorLayout::new(canvas))
                        .map_or(GemShown::Empty, GemShown::Outdoor),
                    Area::Town { .. } => TownShown::sample(state, canvas, town)
                        .map_or(GemShown::Empty, GemShown::Town),
                    Area::Dungeon { .. } => DungeonShown::sample(state, dungeon)
                        .map_or(GemShown::Empty, GemShown::Dungeon),
                }
            } else {
                GemShown::Empty
            };
        Self {
            canvas,
            display_map_brightness,
            page: 0,
            sweep_step: None,
            shown,
        }
    }

    /// Whether this view is a dungeon map drawn as one panel: the 4:3
    /// canvas, or `gem_map.dungeon_stack` (`stack`) off. Only this view
    /// pages (10.2.6, D10.12).
    pub fn is_one_panel_dungeon(&self, stack: bool) -> bool {
        matches!(self.shown, GemShown::Dungeon(_))
            && DungeonLayout::new(self.canvas, stack).is_some_and(|layout| layout.panels == 1)
    }

    /// One PgUp (`up`) or PgDn on a one-panel dungeon map: the page moves
    /// to the nearest level with a panel. Returns whether it moved; any
    /// other view is left as it is.
    pub fn turn_page(&mut self, up: bool, stack: bool) -> bool {
        if !self.is_one_panel_dungeon(stack) {
            return false;
        }
        let GemShown::Dungeon(shown) = &self.shown else {
            return false;
        };
        let page = shown.page_step(self.page, up);
        std::mem::replace(&mut self.page, page) != page
    }
}

/// Whether the game's own view overlay is a gem view (10.3.2): the
/// overlay is up in [`ViewOverlayMode::GemView`] and is not the sky.
pub fn gem_view_open(state: &PlayState) -> bool {
    state.active_view_overlay.as_ref().is_some_and(|overlay| {
        overlay.mode == ViewOverlayMode::GemView && !matches!(overlay.kind, ViewOverlayKind::Sky(_))
    })
}

#[cfg(test)]
#[path = "shown_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "town_tests.rs"]
mod town_tests;
