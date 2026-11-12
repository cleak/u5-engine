//! What the player has discovered, for the enhanced gem map (Phase B design
//! 10.2.8, 10.3.2, 10.3.3; build plan M3-05).
//!
//! [`GemKnowledge`] is the record the gem map draws names from. It holds
//! only scene bytes, coordinates and small codes, never game text: names
//! come from the gazetteer at draw time (10.3.7). It changes only through
//! [`GemKnowledge::apply`], one [`KnowEvent`] at a time; the events come
//! from the observer in [`super::probe`], which reads the game and never
//! changes it.
//!
//! Every bit here is something the game showed the player. When an event
//! could not be told apart from another, the observer records nothing, so
//! the record errs towards a "?" too many, never a name too early
//! (10.2.8).
//!
//! `Debug` prints counts only, never a coordinate, scene or name, so a log
//! line cannot leak a map.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use u5_runtime::DungeonScene;
use u5_runtime::shop_session::ActiveShopSession;

use crate::memory::DungeonBits;

/// The sidecar tag of the gem map's record (10.3.7).
pub const GEM_SIDECAR_TAG: [u8; 4] = *b"GEMK";
/// The payload version this build writes (10.3.7).
pub const GEM_PAYLOAD_VERSION: u16 = 1;

/// Width of a town's side window, in world cells (10.3.7 tag 11).
pub const SIDE_WINDOW_WIDTH: u8 = 48;
/// Height of a town's side window, in world cells.
pub const SIDE_WINDOW_HEIGHT: u8 = 36;
/// Bytes of one side window's bit set (48 x 36 bits).
pub const SIDE_WINDOW_BYTES: usize = SIDE_WINDOW_WIDTH as usize * SIDE_WINDOW_HEIGHT as usize / 8;

/// `flags` bit: the party has been on the Underworld surface.
const FLAG_UNDERWORLD: u8 = 1 << 0;
/// `flags` bit: the party has entered the Codex.
const FLAG_CODEX: u8 = 1 << 1;

/// The nine shop trades of `ActiveShopSession` (10.3.3 "Shop trades").
///
/// The code (`as u8`) is what the payload stores; never reorder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Trade {
    Arms = 0,
    Healer = 1,
    HorseTrader = 2,
    Guild = 3,
    Innkeeper = 4,
    Reagent = 5,
    ShipBroker = 6,
    Tavern = 7,
    Sage = 8,
}

/// Every trade, in code order.
pub const TRADES: [Trade; 9] = [
    Trade::Arms,
    Trade::Healer,
    Trade::HorseTrader,
    Trade::Guild,
    Trade::Innkeeper,
    Trade::Reagent,
    Trade::ShipBroker,
    Trade::Tavern,
    Trade::Sage,
];

/// The shop-sign tiles and the trade each names (`tile-catalog.md §3`,
/// 10.3.3). `0xF0` (provisioner) and `0xF1` (government) are absent on
/// purpose: no session kind matches them, so they stay "?" until the spec
/// says which merchant they belong to (S10.1). Tavern and Sage have no
/// sign at all.
pub const SIGN_TRADES: [(u8, Trade); 7] = [
    (0xF2, Trade::Arms),
    (0xF3, Trade::Healer),
    (0xF4, Trade::HorseTrader),
    (0xF5, Trade::Guild),
    (0xF6, Trade::Innkeeper),
    (0xF7, Trade::Reagent),
    (0xF9, Trade::ShipBroker),
];

impl Trade {
    /// The trade of an open shop session. The three arms sessions (plain,
    /// local stock and stocked) are one trade.
    pub fn of_session(session: &ActiveShopSession) -> Self {
        match session {
            ActiveShopSession::Arms(_)
            | ActiveShopSession::ArmsLocal(..)
            | ActiveShopSession::ArmsStocked(..) => Self::Arms,
            ActiveShopSession::Healer(..) => Self::Healer,
            ActiveShopSession::Innkeeper(_) => Self::Innkeeper,
            ActiveShopSession::Reagent(_) => Self::Reagent,
            ActiveShopSession::Sage(_) => Self::Sage,
            ActiveShopSession::Tavern(_) => Self::Tavern,
            ActiveShopSession::HorseTrader(_) => Self::HorseTrader,
            ActiveShopSession::ShipBroker(_) => Self::ShipBroker,
            ActiveShopSession::Guild(_) => Self::Guild,
        }
    }

    /// The trade's sign tile, or `None` for Tavern and Sage.
    pub fn sign_tile(self) -> Option<u8> {
        SIGN_TRADES
            .iter()
            .find(|(_, trade)| *trade == self)
            .map(|(tile, _)| *tile)
    }

    /// The trade a drawn sign tile names, or `None` (any other tile, and
    /// the unmapped `0xF0` and `0xF1`).
    pub fn from_sign_tile(tile: u8) -> Option<Self> {
        SIGN_TRADES
            .iter()
            .find(|(sign, _)| *sign == tile)
            .map(|(_, trade)| *trade)
    }

    /// Whether the trade is placed by its keeper's cell, having no sign.
    pub fn is_unsigned(self) -> bool {
        self.sign_tile().is_none()
    }

    /// The payload code.
    pub fn code(self) -> u8 {
        self as u8
    }

    /// The trade of a payload code.
    pub fn from_code(code: u8) -> Option<Self> {
        TRADES.get(code as usize).copied()
    }
}

/// Which way a town ladder or stair cell leads, as the party used it (L5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LinkDir {
    Up = 0,
    Down = 1,
}

impl LinkDir {
    pub fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

/// A side of a town floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EdgeSide {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

/// What the party did at a dungeon cell (10.3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MarkKind {
    /// A ladder climbed, from either end.
    Ladder = 0,
    /// A fountain a party member drank from.
    Fountain = 1,
    /// A chest the party opened.
    Chest = 2,
    /// A trap or pit found: fallen through, searched, set off.
    Trap = 3,
    /// A secret door found by Search.
    SecretDoor = 4,
}

/// Where the keeper of an unsigned trade stood when the shop opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeeperMark {
    pub scene: u8,
    pub floor: i8,
    pub x: u8,
    pub y: u8,
    pub trade: Trade,
}

/// One end of a town ladder or stairs the party used, and which way it
/// leads from this cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TownLink {
    pub scene: u8,
    pub floor: i8,
    pub x: u8,
    pub y: u8,
    pub dir: LinkDir,
}

/// A town edge the party crossed. `at` is the crossed cell's position
/// along the side (x on north and south, y on east and west); the
/// renderer extends it to the run of passable edge cells it lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExitRun {
    pub scene: u8,
    pub side: EdgeSide,
    pub at: u8,
}

/// A dungeon cell where the party did something.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DungeonMark {
    pub record: u8,
    pub level: u8,
    pub x: u8,
    pub y: u8,
    pub kind: MarkKind,
}

/// A place in the world or in a town: what the probe saw, as a world plane
/// code (0 Britannia, 1 Underworld) and a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GateCell {
    pub plane: u8,
    pub x: u8,
    pub y: u8,
}

/// A sign cell (10.3.3 "sign read").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignCell {
    pub scene: u8,
    pub floor: i8,
    pub x: u8,
    pub y: u8,
}

/// One discovery (10.3.3). Each sets at most a few bits of the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KnowEvent {
    /// The party is inside the town, castle, keep, dwelling or dungeon
    /// with this scene byte.
    PlaceEntered { scene: u8 },
    /// The party is on the Underworld surface.
    UnderworldVisited,
    /// The party entered the shrine of this virtue (`ShrineVirtue` order).
    ShrineEntered { virtue: u8 },
    /// The party entered the Codex.
    CodexEntered,
    /// A Look printed this dungeon entrance's name (M3-06).
    EntranceNamed { record: u8 },
    /// The party travelled through a moongate from or to this cell.
    GateUsed(GateCell),
    /// The player read this sign with Look (M3-06).
    SignRead(SignCell),
    /// A shop of `trade` opened in `scene`; for Tavern and Sage, `keeper`
    /// is where the keeper stood, when that was unambiguous.
    ShopOpened {
        scene: u8,
        trade: Trade,
        keeper: Option<(i8, u8, u8)>,
    },
    /// The party used a town ladder or stairs: one end per event.
    TownLinkUsed(TownLink),
    /// The party left or entered a town across this edge.
    TownExit(ExitRun),
    /// The party did something at a dungeon cell (M3-06).
    DungeonMarked(DungeonMark),
    /// Dungeon cells seen, as row bits ORed into the level (M3-06).
    DungeonCellsSeen {
        record: u8,
        level: u8,
        rows: [u8; 8],
    },
    /// One cell of a town's side window seen from outside (M3-06),
    /// `index = y * 48 + x` in the window.
    SideCellSeen { scene: u8, index: u16 },
}

/// A feature the gem map may name, as asked by the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeatureKey {
    /// A town, castle, keep, dwelling or dungeon, by scene byte.
    Place {
        scene: u8,
    },
    /// A dungeon entrance outdoors: entered, or named by Look (D10.7).
    Entrance {
        record: u8,
    },
    Shrine {
        virtue: u8,
    },
    Codex,
    Underworld,
    Gate(GateCell),
    Sign(SignCell),
    /// A trade in a town: labels every drawn sign of it there.
    Trade {
        scene: u8,
        trade: Trade,
    },
    /// A town ladder or stairs cell, whichever way the party used it.
    TownLink {
        scene: u8,
        floor: i8,
        x: u8,
        y: u8,
    },
    /// The exact edge cell the party crossed.
    Exit(ExitRun),
    DungeonMark(DungeonMark),
}

/// The gem map's knowledge record (10.3.2).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct GemKnowledge {
    entered: BTreeSet<u8>,
    shrines: u8,
    flags: u8,
    named_entrances: u8,
    gates: BTreeSet<GateCell>,
    signs: BTreeSet<SignCell>,
    trades: BTreeSet<(u8, Trade)>,
    keepers: BTreeSet<KeeperMark>,
    town_links: BTreeSet<TownLink>,
    exits: BTreeSet<ExitRun>,
    cells: DungeonBits,
    marks: BTreeSet<DungeonMark>,
    sides: BTreeMap<u8, Box<[u8; SIDE_WINDOW_BYTES]>>,
}

impl GemKnowledge {
    /// An empty record: everything is "?".
    pub fn new() -> Self {
        Self::default()
    }

    /// Folds one event into the record. True when a bit changed (drives a
    /// redraw).
    pub fn apply(&mut self, event: &KnowEvent) -> bool {
        match *event {
            KnowEvent::PlaceEntered { scene } => self.entered.insert(scene),
            KnowEvent::UnderworldVisited => set_bit(&mut self.flags, FLAG_UNDERWORLD),
            KnowEvent::ShrineEntered { virtue } => {
                virtue < 8 && set_bit(&mut self.shrines, 1 << virtue)
            }
            KnowEvent::CodexEntered => set_bit(&mut self.flags, FLAG_CODEX),
            KnowEvent::EntranceNamed { record } => {
                record < 8 && set_bit(&mut self.named_entrances, 1 << record)
            }
            KnowEvent::GateUsed(cell) => self.gates.insert(cell),
            KnowEvent::SignRead(cell) => self.signs.insert(cell),
            KnowEvent::ShopOpened {
                scene,
                trade,
                keeper,
            } => {
                let mut changed = self.trades.insert((scene, trade));
                if let (true, Some((floor, x, y))) = (trade.is_unsigned(), keeper) {
                    changed |= self.keepers.insert(KeeperMark {
                        scene,
                        floor,
                        x,
                        y,
                        trade,
                    });
                }
                changed
            }
            KnowEvent::TownLinkUsed(link) => self.town_links.insert(link),
            KnowEvent::TownExit(run) => self.exits.insert(run),
            KnowEvent::DungeonMarked(mark) => self.marks.insert(mark),
            KnowEvent::DungeonCellsSeen {
                record,
                level,
                rows,
            } => self.cells.merge_level_rows(record, level, rows),
            KnowEvent::SideCellSeen { scene, index } => {
                let index = index as usize;
                if index >= SIDE_WINDOW_BYTES * 8 {
                    return false;
                }
                let bits = self
                    .sides
                    .entry(scene)
                    .or_insert_with(|| Box::new([0; SIDE_WINDOW_BYTES]));
                set_bit(&mut bits[index / 8], 1 << (index % 8))
            }
        }
    }

    /// Whether the player has discovered `feature`.
    pub fn is_known(&self, feature: &FeatureKey) -> bool {
        match *feature {
            FeatureKey::Place { scene } => self.entered.contains(&scene),
            FeatureKey::Entrance { record } => {
                (record < 8 && self.named_entrances & (1 << record) != 0)
                    || DungeonScene::from_record(record)
                        .is_ok_and(|scene| self.entered.contains(&scene.byte))
            }
            FeatureKey::Shrine { virtue } => virtue < 8 && self.shrines & (1 << virtue) != 0,
            FeatureKey::Codex => self.flags & FLAG_CODEX != 0,
            FeatureKey::Underworld => self.flags & FLAG_UNDERWORLD != 0,
            FeatureKey::Gate(cell) => self.gates.contains(&cell),
            FeatureKey::Sign(cell) => self.signs.contains(&cell),
            FeatureKey::Trade { scene, trade } => self.trades.contains(&(scene, trade)),
            FeatureKey::TownLink { scene, floor, x, y } => self
                .town_links
                .iter()
                .any(|l| l.scene == scene && l.floor == floor && l.x == x && l.y == y),
            FeatureKey::Exit(run) => self.exits.contains(&run),
            FeatureKey::DungeonMark(mark) => self.marks.contains(&mark),
        }
    }

    /// Whether dungeon cell `(x, y)` of `record`, `level` is remembered.
    pub fn remembered(&self, record: u8, level: u8, x: u8, y: u8) -> bool {
        self.cells.is_set(record, level, x, y)
    }

    /// The directions the party used a town ladder or stairs cell in.
    pub fn town_link_dirs(
        &self,
        scene: u8,
        floor: i8,
        x: u8,
        y: u8,
    ) -> impl Iterator<Item = LinkDir> + '_ {
        self.town_links
            .iter()
            .filter(move |l| l.scene == scene && l.floor == floor && l.x == x && l.y == y)
            .map(|l| l.dir)
    }

    /// The keeper cells of unsigned trades in `scene`.
    pub fn keepers_in(&self, scene: u8) -> impl Iterator<Item = &KeeperMark> + '_ {
        self.keepers.iter().filter(move |k| k.scene == scene)
    }

    /// Whether cell `index` (`y * 48 + x`) of `scene`'s side window was
    /// seen from outside.
    pub fn side_seen(&self, scene: u8, index: u16) -> bool {
        let index = index as usize;
        index < SIDE_WINDOW_BYTES * 8
            && self
                .sides
                .get(&scene)
                .is_some_and(|bits| bits[index / 8] & (1 << (index % 8)) != 0)
    }

    /// Whether nothing at all is recorded.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    // Read-only views for the payload codec (M3-07) and the renderer.

    pub fn entered(&self) -> &BTreeSet<u8> {
        &self.entered
    }
    /// Shrines, flags and named entrances: tag 2's three bytes.
    pub fn flag_bytes(&self) -> [u8; 3] {
        [self.shrines, self.flags, self.named_entrances]
    }
    pub fn gates(&self) -> &BTreeSet<GateCell> {
        &self.gates
    }
    pub fn signs(&self) -> &BTreeSet<SignCell> {
        &self.signs
    }
    pub fn trades(&self) -> &BTreeSet<(u8, Trade)> {
        &self.trades
    }
    pub fn keepers(&self) -> &BTreeSet<KeeperMark> {
        &self.keepers
    }
    pub fn town_links(&self) -> &BTreeSet<TownLink> {
        &self.town_links
    }
    pub fn exits(&self) -> &BTreeSet<ExitRun> {
        &self.exits
    }
    pub fn cells(&self) -> &DungeonBits {
        &self.cells
    }
    pub fn marks(&self) -> &BTreeSet<DungeonMark> {
        &self.marks
    }
    pub fn sides(&self) -> &BTreeMap<u8, Box<[u8; SIDE_WINDOW_BYTES]>> {
        &self.sides
    }
}

fn set_bit(byte: &mut u8, mask: u8) -> bool {
    let before = *byte;
    *byte |= mask;
    *byte != before
}

impl fmt::Debug for GemKnowledge {
    /// Counts only: never a coordinate, a scene byte or a name.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GemKnowledge")
            .field("entered", &self.entered.len())
            .field("shrines", &self.shrines.count_ones())
            .field("flags", &self.flags.count_ones())
            .field("named_entrances", &self.named_entrances.count_ones())
            .field("gates", &self.gates.len())
            .field("signs", &self.signs.len())
            .field("trades", &self.trades.len())
            .field("keepers", &self.keepers.len())
            .field("town_links", &self.town_links.len())
            .field("exits", &self.exits.len())
            .field("cells", &self.cells.total())
            .field("marks", &self.marks.len())
            .field(
                "side_cells",
                &self
                    .sides
                    .values()
                    .map(|bits| bits.iter().map(|b| b.count_ones()).sum::<u32>())
                    .sum::<u32>(),
            )
            .finish()
    }
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod tests;
