//! The dungeon gem map: its shown set, panels and titles (Phase B design
//! 10.2.3, 10.2.6, 10.2.7 "Dungeon level panels", 10.2.9 L6 and L7; build
//! plan M3-13).
//!
//! **The shown set.** On the current level, the cells the original gem's
//! flood reaches (`view_overlay_footprint`, `dungeon-mode.md §12.2`) plus
//! the cells of that level the record remembers; on every other level of
//! the dungeon, only the remembered cells. Nothing is taken from data the
//! player never saw. Every level comes from the game's own dungeon grid,
//! which holds all eight levels of the dungeon the party is in, so no file
//! is read.
//!
//! **The glyph policy** (10.2.3, D10.6). Under `perceived`, the default,
//! each cell is drawn as its [`DungeonLook`](crate::DungeonLook): a `0xD?`
//! secret door as wall and the hidden pits `0x61`, `0x69` and the bomb
//! trap `0x62` as passage, until the record says the party found them
//! ([`GemKnowledge::perceived_cell`]); a found secret door draws as a door
//! even after the dungeon reloads. Under `gem-faithful` each cell is the
//! raw byte, as the original gem's table draws it (`dungeon-mode.md
//! §12.4`). Either way the glyph itself comes from the engine's own
//! `draw_dungeon_view_glyph` (10.3.4), so its colours are the 1.0.1
//! engine's, pinned nowhere here (D-M3.1).
//!
//! **Panels need evidence** (10.2.7, leak L7). A level other than the
//! current one gets a panel only when the party remembers cells on it, or
//! a ladder or pit drawn on an adjacent level that has cells leads to it;
//! the second kind is black. The slot above level 1 is `Surface`. The slot
//! below the deepest level is the Underworld: a panel only when a drawn
//! ladder or pit leads there, titled `Underworld` only once the party has
//! been on the Underworld surface. Every other slot is plain black, with
//! no title.
//!
//! **Link boxes and pages** (10.2.6, M3-14). [`DungeonShown::linked`]
//! says which ladder and pit cells get a yellow link box: both ends in the
//! shown set. [`DungeonShown::page_step`] is PgUp and PgDn on a lone
//! panel, skipping the slots without a panel.

use u5_runtime::enhancements::GlyphPolicy;
use u5_runtime::{Area, Direction, PlayState, ViewOverlayFootprint};

use super::knowledge::{FeatureKey, GemKnowledge};
use crate::memory::DUNGEON_LEVEL_COUNT;
use crate::perceived::{perceived_byte, perceived_dungeon_cell};

/// Cells on a level's side.
const SIDE: usize = 8;

/// What the dungeon view reads besides the game: the record and the glyph
/// policy. The default reads no record, so only the flood is shown.
#[derive(Clone, Copy, Debug, Default)]
pub struct DungeonInputs<'a> {
    pub know: Option<&'a GemKnowledge>,
    pub glyphs: GlyphPolicy,
}

/// How a shown dungeon cell is lit (10.2.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DungeonLit {
    /// A cell of the gem's own flood: full map brightness.
    Flood,
    /// A remembered cell: 70% before the map's own brightness.
    Remembered,
}

/// One shown dungeon cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DungeonCell {
    /// The cell's byte in the game's grid.
    pub raw: u8,
    /// The byte whose gem glyph is drawn: the perceived byte, or `raw`
    /// under `gem-faithful`.
    pub drawn: u8,
    pub lit: DungeonLit,
}

/// What a panel slot of the stack shows (10.2.6, 10.2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DungeonSlot {
    /// Nothing: plain black, no title.
    Empty,
    /// Above level 1: the title `Surface` only.
    Surface,
    /// A dungeon level, 0-based. `cells` is false for a level shown only
    /// because a drawn ladder or pit leads to it: a black panel.
    Level { level: u8, cells: bool },
    /// Below the deepest level, when a drawn ladder or pit leads there;
    /// `named` once the party has been on the Underworld surface.
    Underworld { named: bool },
}

/// The dungeon shown set of one open gem view (10.2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonShown {
    /// The dungeon's record (0..8) and its name (gazetteer §5.1).
    pub record: u8,
    pub name: &'static str,
    /// The party's level, cell and facing.
    pub level: u8,
    pub party: (u8, u8),
    pub facing: Direction,
    pub glyphs: GlyphPolicy,
    pub underworld_known: bool,
    /// Per level, row-major: the shown cell or `None` (black).
    levels: Vec<[Option<DungeonCell>; SIDE * SIDE]>,
    /// Per level and the Underworld (index 8): a drawn ladder or pit on an
    /// adjacent level with cells leads here.
    led_to: [bool; DUNGEON_LEVEL_COUNT as usize + 1],
}

impl DungeonShown {
    /// The shown set of the dungeon the party is in, or `None` outside a
    /// dungeon or with no dungeon gem view open.
    pub fn sample(state: &PlayState, inputs: DungeonInputs<'_>) -> Option<Self> {
        let Area::Dungeon { scene, level } = state.area else {
            return None;
        };
        let Some(ViewOverlayFootprint::Dungeon { cells: flood, .. }) =
            state.view_overlay_footprint()
        else {
            return None;
        };
        let record = scene.record as u8;
        let count = DUNGEON_LEVEL_COUNT as usize;
        let mut levels = vec![[None; SIDE * SIDE]; count];
        let remembered = |lvl: u8, x: u8, y: u8| {
            inputs
                .know
                .is_some_and(|know| know.remembered(record, lvl, x, y))
        };
        for (lvl, cells) in levels.iter_mut().enumerate() {
            let lvl = lvl as u8;
            for y in 0..SIDE as u8 {
                for x in 0..SIDE as u8 {
                    let lit = if lvl == level && flood.contains(&(x, y)) {
                        DungeonLit::Flood
                    } else if remembered(lvl, x, y) {
                        DungeonLit::Remembered
                    } else {
                        continue;
                    };
                    let raw = state.dungeon_cell(lvl, usize::from(x), usize::from(y));
                    let drawn = match inputs.glyphs {
                        GlyphPolicy::GemFaithful => raw,
                        GlyphPolicy::Perceived => {
                            let cell = inputs.know.map_or(perceived_dungeon_cell(raw), |know| {
                                know.perceived_cell(record, lvl, x, y, raw)
                            });
                            perceived_byte(cell, raw)
                        }
                    };
                    cells[usize::from(y) * SIDE + usize::from(x)] =
                        Some(DungeonCell { raw, drawn, lit });
                }
            }
        }
        let mut led_to = [false; DUNGEON_LEVEL_COUNT as usize + 1];
        for (lvl, cells) in levels.iter().enumerate() {
            for cell in cells.iter().flatten() {
                let (up, down) = leads(cell.drawn);
                if up && lvl > 0 {
                    led_to[lvl - 1] = true;
                }
                if down {
                    led_to[lvl + 1] = true;
                }
            }
        }
        Some(Self {
            record,
            name: scene.name(),
            level,
            party: (state.player.x as u8, state.player.y as u8),
            facing: state.player.facing,
            glyphs: inputs.glyphs,
            underworld_known: inputs
                .know
                .is_some_and(|know| know.is_known(&FeatureKey::Underworld)),
            levels,
            led_to,
        })
    }

    /// The shown cell `(x, y)` of `level`, or `None` (black).
    pub fn cell(&self, level: u8, x: u8, y: u8) -> Option<DungeonCell> {
        if usize::from(x) >= SIDE || usize::from(y) >= SIDE {
            return None;
        }
        self.levels
            .get(usize::from(level))?
            .get(usize::from(y) * SIDE + usize::from(x))
            .copied()
            .flatten()
    }

    /// Whether `level` has any shown cell.
    pub fn has_cells(&self, level: u8) -> bool {
        self.levels
            .get(usize::from(level))
            .is_some_and(|cells| cells.iter().any(Option::is_some))
    }

    /// What the slot of `level` (-1 the surface, 8 the Underworld) shows.
    pub fn slot(&self, level: i32) -> DungeonSlot {
        let count = i32::from(DUNGEON_LEVEL_COUNT);
        match level {
            -1 => DungeonSlot::Surface,
            l if (0..count).contains(&l) => {
                let lvl = l as u8;
                let cells = lvl == self.level || self.has_cells(lvl);
                if cells || self.led_to[l as usize] {
                    DungeonSlot::Level { level: lvl, cells }
                } else {
                    DungeonSlot::Empty
                }
            }
            l if l == count && self.led_to[count as usize] => DungeonSlot::Underworld {
                named: self.underworld_known,
            },
            _ => DungeonSlot::Empty,
        }
    }

    /// The title over the slot of `level` (10.2.6): `<Dungeon> level N`
    /// over the current level, `Level N` over another, `Surface` above
    /// level 1, `Underworld` only after a visit; none over an empty slot.
    pub fn title(&self, level: i32) -> Option<String> {
        match self.slot(level) {
            DungeonSlot::Empty | DungeonSlot::Underworld { named: false } => None,
            DungeonSlot::Surface => Some("Surface".to_owned()),
            DungeonSlot::Underworld { named: true } => Some("Underworld".to_owned()),
            DungeonSlot::Level { level, .. } if level == self.level => {
                Some(format!("{} level {}", self.name, level + 1))
            }
            DungeonSlot::Level { level, .. } => Some(format!("Level {}", level + 1)),
        }
    }

    /// Whether the slot of `level` is a panel a page can show (10.2.6
    /// "Paging"): a dungeon level with a panel, or the Underworld's black
    /// panel. The surface (a title only) and empty slots are not.
    pub fn has_panel(&self, level: i32) -> bool {
        matches!(
            self.slot(level),
            DungeonSlot::Level { .. } | DungeonSlot::Underworld { .. }
        )
    }

    /// The page after one PgUp (`up`) or PgDn from `page` (10.2.6): the
    /// nearest level above or below the paged one that has a panel,
    /// skipping the levels without one. `page` itself when there is none.
    pub fn page_step(&self, page: i8, up: bool) -> i8 {
        let count = i32::from(DUNGEON_LEVEL_COUNT);
        let step = if up { -1 } else { 1 };
        let mut level = i32::from(self.level) + i32::from(page);
        loop {
            level += step;
            if !(-1..=count).contains(&level) {
                return page;
            }
            if self.has_panel(level) {
                return (level - i32::from(self.level)) as i8;
            }
        }
    }

    /// Whether the shown cell `(x, y)` of `level` gets a ladder link box
    /// (10.2.6): it is a ladder or a pit whose other end, the same cell of
    /// the adjacent level, is also in the shown set. Both ends must be
    /// shown, so a landing the player has not seen is never boxed; the other
    /// level need not be on screen (on 4:3 it never is). A way down links
    /// to the cell below when the cell below is a way up, or when it is a
    /// pit (the fall lands there). Only ladder and pit cells are boxed: a
    /// pit's landing on plain floor is not.
    pub fn linked(&self, level: u8, x: u8, y: u8) -> bool {
        let Some(cell) = self.cell(level, x, y) else {
            return false;
        };
        let (up, down) = leads(cell.drawn);
        let below = level
            .checked_add(1)
            .filter(|&l| l < DUNGEON_LEVEL_COUNT)
            .and_then(|l| self.cell(l, x, y));
        let above = level.checked_sub(1).and_then(|l| self.cell(l, x, y));
        let joins = |upper: DungeonCell, lower: DungeonCell| {
            leads(upper.drawn).1 && (is_pit(upper.drawn) || leads(lower.drawn).0)
        };
        (down && below.is_some_and(|lower| joins(cell, lower)))
            || (up && above.is_some_and(|upper| joins(upper, cell)))
    }

    /// Every shown cell of `level` as `(x, y, cell)`.
    pub fn cells(&self, level: u8) -> impl Iterator<Item = (u8, u8, DungeonCell)> + '_ {
        self.levels
            .get(usize::from(level))
            .into_iter()
            .flat_map(|cells| cells.iter().enumerate())
            .filter_map(|(index, cell)| {
                cell.map(|cell| ((index % SIDE) as u8, (index / SIDE) as u8, cell))
            })
    }
}

/// Whether a drawn byte shows a way up and a way down (10.2.7): the ladder
/// classes `0x1?` up, `0x2?` down, `0x3?` both, and the visible pit
/// `0x60` (`0x68`) down. A trap glyph under `gem-faithful` says nothing
/// about where it leads.
pub fn leads(drawn: u8) -> (bool, bool) {
    match drawn >> 4 {
        0x1 => (true, false),
        0x2 => (false, true),
        0x3 => (true, true),
        0x6 if drawn & 0x07 == 0 => (false, true),
        _ => (false, false),
    }
}

/// Whether a drawn byte is the visible pit `0x60` (`0x68`).
fn is_pit(drawn: u8) -> bool {
    drawn >> 4 == 0x6 && drawn & 0x07 == 0
}

#[cfg(test)]
#[path = "dungeon_tests.rs"]
mod tests;
