//! The gem map's features: what gets a mark, a "?" or a label (Phase B
//! design 10.2.3 rule 2, 10.2.7, 10.2.8, 10.2.9 L1, L3, L4, L8 and L9;
//! build plan M3-15).
//!
//! Features are extracted per view from the **drawn tiles of the shown set
//! only** ([`GemShown`]). A cell carries a feature when it is in the shown
//! set and either its drawn tile is one of the tiles of the feature's kind,
//! or the record says the player already discovered that feature there:
//!
//! | View | Kind | Drawn tile |
//! |---|---|---|
//! | outdoors, town sides | place | a location marker `0x10..0x1B`, or the collapsed dungeon entrance `0xDF` |
//! | outdoors | moongate | the live gate `0xDC` |
//! | outdoors, town | sign | `0x89`, `0x8A`, `0xA0`, `0xA4`, `0xF8` |
//! | town | shop | a shop sign `0xF0..0xF7`, `0xF9` |
//! | town | ladder, stairs | `0xC4..0xC9`, one mark per adjoining group |
//! | town | exit | a run of foot-passable edge cells, as drawn, that the party can walk to |
//! | dungeon | ladder | `0x1?`, `0x2?`, `0x3?` |
//! | dungeon | fountain, chest | `0x5?`, the closed chest `0x4?` |
//! | dungeon | pit | `0x60` (and `0x68`); other `0x6?` only under `gem-faithful` |
//!
//! The record is never read for a feature's existence, only for what the
//! player knows about a cell the map already shows: a discovered feature
//! keeps its mark where the drawn tile now hides it (a sunk moongate, an
//! opened chest), because the player already knows it. Tavern and Sage
//! have no sign tile, so they appear only at the keeper cell the record
//! holds (L4). `0xF0` and `0xF1` name no trade the record can hold, so
//! they stay "?" (S10.1). The town floor and the town sides are terrain
//! only, so no person, monster or hidden object (a buried moonstone, a
//! shard, the Amulet, the arms cache) ever becomes a feature, and an
//! object standing over a place entrance cannot raise a "?" (L9).
//!
//! **Identity** comes from the gazetteer (`catalogs/gazetteer.md` §5.1, §7,
//! §8.1): a marker drawn at a published coordinate is that place; a marker
//! nowhere in it is a "?" for good. The Shrine of the Codex is a place
//! (L8). The gazetteer gives a drawn marker its name, never its existence.
//!
//! **What a feature tells** ([`GemFeature`]): an undiscovered one has no
//! category, no colour and no label, only its cell, so a "?" says
//! "something is here" and nothing about what (L1). Its ring is
//! [`NEUTRAL_GREY`]. Labels are fixed words or published names, each a
//! `&'static str`; never sign text, a person's name or conversation text.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use u5_runtime::enhancements::GlyphPolicy;
use u5_runtime::predicates::{foot_terrain_accepts, is_town_door_tile};
use u5_runtime::world_tables::{PUBLISHED_SHRINE_COORDINATES, WorldEntryNarrationClass};
use u5_runtime::{
    DungeonScene, PlayTarget, SCENE_OVERWORLD, WorldPlane, published_world_location_entries,
};

use super::dungeon::DungeonShown;
use super::knowledge::{
    DungeonMark, EdgeSide, ExitRun, FeatureKey, GateCell, GemKnowledge, LinkDir, MarkKind,
    SignCell, Trade,
};
use super::layout::TOWN_FLOOR_CELLS;
use super::probe_more::entrance_record_for_column;
use super::shown::{GemShown, OutdoorShown, TownPart, TownShown};
use crate::memory::DUNGEON_LEVEL_COUNT;
use crate::raster::MAP_PALETTE;

/// The "?" tag's ring and an unknown feature's ring: EGA 7, light grey,
/// the same for every kind (10.2.7, L1).
pub const NEUTRAL_GREY: [u8; 3] = MAP_PALETTE[7];

/// The chest ring: brown `#AA5500` (10.2.7), pinned here because the
/// map's 200-line palette may render EGA 6 otherwise.
pub const CHEST_BROWN: [u8; 3] = [0xAA, 0x55, 0x00];

/// The "?" tag's side in pixels (10.2.7).
pub const QUESTION_TAG_PX: i32 = 12;

/// The location-marker tiles `0x10..0x1B` (`tile-catalog.md §3`).
pub const PLACE_MARKER_TILES: std::ops::RangeInclusive<u8> = 0x10..=0x1B;
/// The collapsed dungeon entrance (`tile-catalog.md §3`).
pub const COLLAPSED_ENTRANCE_TILE: u8 = 0xDF;
/// The live moongate (`overworld.md §9.1`).
pub const MOONGATE_TILE: u8 = 0xDC;
/// The sign and poster tiles (`view.md §3`).
pub const SIGN_TILES: [u8; 5] = [0x89, 0x8A, 0xA0, 0xA4, 0xF8];
/// The shop-sign tiles (`tile-catalog.md §3`).
pub const SHOP_SIGN_TILES: [u8; 9] = [0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF9];
/// The town stairs `0xC4..0xC7`.
pub const STAIR_TILES: std::ops::RangeInclusive<u8> = 0xC4..=0xC7;
/// The town ladders `0xC8` (up) and `0xC9` (down).
pub const LADDER_TILES: std::ops::RangeInclusive<u8> = 0xC8..=0xC9;

/// The Shrine of the Codex's cell (`gazetteer.md` §7).
const CODEX_CELL: (u8, u8) = (233, 233);

/// The gazetteer §5.1 names of the town-family scenes 1..=32; the three
/// unnamed dwellings are `Hut` (10.2.7). The two castles carry the
/// approved map's short names (the mockups' README and the 10.2.4 sketch:
/// `Castle British`, `Blackthorn`), so the label fits beside the castle.
const PLACE_NAMES: [&str; 32] = [
    "Moonglow",
    "Britain",
    "Jhelom",
    "Yew",
    "Minoc",
    "Trinsic",
    "Skara Brae",
    "New Magincia",
    "Fogsbane",
    "Stormcrow",
    "Greyhaven",
    "Waveguide",
    "Iolo's Hut",
    "Hut",
    "Hut",
    "Hut",
    "Castle British",
    "Blackthorn",
    "West Britanny",
    "North Britanny",
    "East Britanny",
    "Paws",
    "Cove",
    "Buccaneer's Den",
    "Ararat",
    "Bordermarch",
    "Farthing",
    "Windemere",
    "Stonegate",
    "The Lycaeum",
    "Empath Abbey",
    "Serpent's Hold",
];

/// A feature's category: its ring colour and label kind (10.2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureCategory {
    /// A town or village: white 15.
    Town,
    /// A castle or keep: light blue 9.
    Castle,
    /// A hut or lighthouse: light green 10.
    Dwelling,
    /// A dungeon entrance: light red 12.
    Dungeon,
    /// A shrine: light cyan 11.
    Shrine,
    /// Another place, the Shrine of the Codex: light magenta 13 (L8).
    OtherPlace,
    /// A moongate: light magenta 13.
    Moongate,
    /// A sign: light grey 7.
    Sign,
    /// A shop: light cyan 11.
    Shop,
    /// A town ladder or stairs: light green 10.
    TownLink,
    /// A town exit: a white IBM.CH arrow, no ring.
    Exit,
    /// A dungeon ladder: light green 10.
    Ladder,
    /// A fountain: light cyan 11.
    Fountain,
    /// A chest: brown `#AA5500`.
    Chest,
    /// A trap or pit: light red 12.
    Trap,
}

impl FeatureCategory {
    /// The category's colour, at full brightness.
    pub fn colour(self) -> [u8; 3] {
        match self {
            Self::Town | Self::Exit => MAP_PALETTE[15],
            Self::Castle => MAP_PALETTE[9],
            Self::Dwelling | Self::TownLink | Self::Ladder => MAP_PALETTE[10],
            Self::Dungeon | Self::Trap => MAP_PALETTE[12],
            Self::Shrine | Self::Shop | Self::Fountain => MAP_PALETTE[11],
            Self::OtherPlace | Self::Moongate => MAP_PALETTE[13],
            Self::Sign => MAP_PALETTE[7],
            Self::Chest => CHEST_BROWN,
        }
    }
}

/// Where a feature is, in its view's own cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FeatureCell {
    /// Outdoors: the offset from the party.
    Outdoor { dx: i32, dy: i32 },
    /// In a town: a canvas cell of [`TownShown`]. For an exit, the middle
    /// cell of its edge run, and the side it leads out of.
    Town {
        fx: i32,
        fy: i32,
        exit: Option<EdgeSide>,
    },
    /// In a dungeon: a cell of a level.
    Dungeon { level: u8, x: u8, y: u8 },
}

/// One feature of an open view (10.2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GemFeature {
    pub cell: FeatureCell,
    /// Whether the drawn tile carries the feature (rule 2's tile arm);
    /// false for a discovered feature the drawn tile hides.
    pub drawn: bool,
    /// The squared distance from the party in cells, for label order
    /// (10.2.7); a dungeon feature on another level counts 400 more.
    pub distance: u32,
    category: FeatureCategory,
    known: bool,
    label: &'static str,
    key: Option<FeatureKey>,
}

impl GemFeature {
    /// Whether the player has discovered it.
    pub fn known(&self) -> bool {
        self.known
    }

    /// Its category, only once discovered (L1).
    pub fn category(&self) -> Option<FeatureCategory> {
        self.known.then_some(self.category)
    }

    /// Its label, only once discovered: a fixed word or a published name.
    pub fn label(&self) -> Option<&'static str> {
        self.known.then_some(self.label)
    }

    /// The ring colour: the category's once discovered, otherwise the
    /// neutral grey (L1).
    pub fn ring_colour(&self) -> [u8; 3] {
        self.category()
            .map_or(NEUTRAL_GREY, FeatureCategory::colour)
    }

    /// The kind, known or not. Only the no-spoiler audit asks this of an
    /// undiscovered feature (M3-18); nothing that draws does.
    pub(crate) fn kind_for_audit(&self) -> FeatureCategory {
        self.category
    }

    /// The record key the feature is known by, when it has an identity.
    pub fn key(&self) -> Option<FeatureKey> {
        self.key
    }

    /// The side a town exit leads out of.
    pub fn exit_side(&self) -> Option<EdgeSide> {
        match self.cell {
            FeatureCell::Town { exit, .. } => exit,
            _ => None,
        }
    }
}

/// The features of one open view (10.2.7), in a fixed order: by cell, then
/// kind. Empty for [`GemShown::Empty`].
pub fn gem_features(shown: &GemShown, know: &GemKnowledge) -> Vec<GemFeature> {
    match shown {
        GemShown::Empty => Vec::new(),
        GemShown::Outdoor(shown) => outdoor_features(shown, know),
        GemShown::Town(shown) => town_features(shown, know),
        GemShown::Dungeon(shown) => dungeon_features(shown, know),
    }
}

/// A place of the gazetteer: where, what it is called, how it is known.
#[derive(Clone, Copy, Debug)]
struct Place {
    plane: WorldPlane,
    /// The seven ordinary dungeon entrances are on both planes.
    both: bool,
    x: u8,
    y: u8,
    key: FeatureKey,
    category: FeatureCategory,
    label: &'static str,
}

impl Place {
    fn at(&self, plane: WorldPlane, x: u8, y: u8) -> bool {
        (self.plane == plane || self.both) && (self.x, self.y) == (x, y)
    }
}

/// Every place of the gazetteer: the forty scene rows (§5.1), the seven
/// mapped shrines (§7) and the Shrine of the Codex (§7, §8.1).
fn places() -> &'static [Place] {
    static PLACES: OnceLock<Vec<Place>> = OnceLock::new();
    PLACES.get_or_init(|| {
        let mut out = Vec::new();
        for entry in published_world_location_entries() {
            let class = entry.narration_class;
            let category = match class {
                Some(WorldEntryNarrationClass::Towne | WorldEntryNarrationClass::Village) => {
                    FeatureCategory::Town
                }
                Some(
                    WorldEntryNarrationClass::Castle
                    | WorldEntryNarrationClass::Keep
                    | WorldEntryNarrationClass::LordBritish
                    | WorldEntryNarrationClass::Blackthorn,
                ) => FeatureCategory::Castle,
                Some(WorldEntryNarrationClass::Hut | WorldEntryNarrationClass::Lighthouse) => {
                    FeatureCategory::Dwelling
                }
                _ => FeatureCategory::Dungeon,
            };
            let (key, category, label) = match entry.target {
                PlayTarget::Town(scene) => (
                    FeatureKey::Place { scene: scene.byte },
                    category,
                    PLACE_NAMES
                        .get(usize::from(scene.byte) - 1)
                        .copied()
                        .unwrap_or("Hut"),
                ),
                PlayTarget::Dungeon(scene) => (
                    FeatureKey::Entrance {
                        record: scene.record as u8,
                    },
                    FeatureCategory::Dungeon,
                    scene.name(),
                ),
                PlayTarget::World(_) => continue,
            };
            out.push(Place {
                plane: entry.plane,
                both: entry.accepts_both_world_planes,
                x: entry.x as u8,
                y: entry.y as u8,
                key,
                category,
                label,
            });
        }
        for (virtue, x, y) in PUBLISHED_SHRINE_COORDINATES {
            // Spirituality's (0, 0) is a sentinel, not a cell (§7).
            if (x, y) == (0, 0) {
                continue;
            }
            out.push(Place {
                plane: WorldPlane::Britannia,
                both: false,
                x: x as u8,
                y: y as u8,
                key: FeatureKey::Shrine {
                    virtue: virtue.index() as u8,
                },
                category: FeatureCategory::Shrine,
                label: virtue.name(),
            });
        }
        out.push(Place {
            plane: WorldPlane::Britannia,
            both: false,
            x: CODEX_CELL.0,
            y: CODEX_CELL.1,
            key: FeatureKey::Codex,
            category: FeatureCategory::OtherPlace,
            label: "Codex",
        });
        out
    })
}

/// The place a marker drawn at world cell `(x, y)` is: the gazetteer row
/// at that cell, or for a collapsed entrance the dungeon of its column
/// (`view.md §3` row 5c).
fn place_of_marker(plane: WorldPlane, x: u8, y: u8, tile: u8) -> Option<Place> {
    if let Some(place) = places().iter().find(|place| place.at(plane, x, y)) {
        return Some(*place);
    }
    if tile != COLLAPSED_ENTRANCE_TILE {
        return None;
    }
    let record = entrance_record_for_column(x)?;
    let scene = DungeonScene::from_record(record).ok()?;
    Some(Place {
        plane,
        both: true,
        x,
        y,
        key: FeatureKey::Entrance { record },
        category: FeatureCategory::Dungeon,
        label: scene.name(),
    })
}

fn plane_code(plane: WorldPlane) -> u8 {
    match plane {
        WorldPlane::Britannia => 0,
        WorldPlane::Underworld => 1,
    }
}

/// Whether a drawn tile is a place marker.
pub fn is_place_marker(tile: u8) -> bool {
    PLACE_MARKER_TILES.contains(&tile) || tile == COLLAPSED_ENTRANCE_TILE
}

/// The features of one world cell, outdoors or on a town side: a place, a
/// moongate (outdoors only: the sides draw the static map, which has no
/// live gate), a sign.
fn world_cell_features(
    plane: WorldPlane,
    (x, y): (u8, u8),
    tile: u8,
    know: &GemKnowledge,
    mut push: impl FnMut(FeatureCategory, bool, &'static str, Option<FeatureKey>, bool),
) {
    // A place: on its marker, or known where something else is drawn.
    if is_place_marker(tile) {
        match place_of_marker(plane, x, y, tile) {
            Some(place) => push(
                place.category,
                know.is_known(&place.key),
                place.label,
                Some(place.key),
                true,
            ),
            None => push(FeatureCategory::OtherPlace, false, "", None, true),
        }
    } else if let Some(place) = places()
        .iter()
        .find(|place| place.at(plane, x, y) && know.is_known(&place.key))
    {
        push(place.category, true, place.label, Some(place.key), false);
    }
    // A moongate: while the live gate is drawn there, or used there (L3).
    let gate = FeatureKey::Gate(GateCell {
        plane: plane_code(plane),
        x,
        y,
    });
    let gate_known = know.is_known(&gate);
    if tile == MOONGATE_TILE || gate_known {
        push(
            FeatureCategory::Moongate,
            gate_known,
            "Moongate",
            Some(gate),
            tile == MOONGATE_TILE,
        );
    }
    // A sign: on its tile, or read there.
    let sign = FeatureKey::Sign(SignCell {
        scene: SCENE_OVERWORLD,
        floor: plane.save_floor(),
        x,
        y,
    });
    let sign_known = know.is_known(&sign);
    if SIGN_TILES.contains(&tile) || sign_known {
        push(
            FeatureCategory::Sign,
            sign_known,
            "Sign",
            Some(sign),
            SIGN_TILES.contains(&tile),
        );
    }
}

fn squared(dx: i32, dy: i32) -> u32 {
    (dx * dx + dy * dy) as u32
}

fn feature(
    cell: FeatureCell,
    distance: u32,
    (category, known, label, key, drawn): (
        FeatureCategory,
        bool,
        &'static str,
        Option<FeatureKey>,
        bool,
    ),
) -> GemFeature {
    GemFeature {
        cell,
        drawn,
        distance,
        category,
        known,
        label,
        key,
    }
}

/// The outdoor features (10.2.4, 10.2.7).
pub fn outdoor_features(shown: &OutdoorShown, know: &GemKnowledge) -> Vec<GemFeature> {
    let mut out = Vec::new();
    for (dx, dy, tile) in shown.cells() {
        let cell = shown.world_cell(dx, dy);
        world_cell_features(shown.plane, cell, tile, know, |c, k, l, key, d| {
            out.push(feature(
                FeatureCell::Outdoor { dx, dy },
                squared(dx, dy),
                (c, k, l, key, d),
            ));
        });
    }
    out
}

/// A trade's label (10.2.7): its trade word.
pub fn trade_label(trade: Trade) -> &'static str {
    match trade {
        Trade::Arms => "Armoury",
        Trade::Healer => "Healer",
        Trade::HorseTrader => "Stable",
        Trade::Guild => "Guild",
        Trade::Innkeeper => "Inn",
        Trade::Reagent => "Apothecary",
        Trade::ShipBroker => "Shipwright",
        Trade::Tavern => "Tavern",
        Trade::Sage => "Sage",
    }
}

/// A town ladder or stairs cell's label from the tile drawn there and the
/// directions the party used it in (L5): `Ladder up`, `Stairs down` ...;
/// the bare word when it went both ways.
fn town_link_label(tile: u8, dirs: &BTreeSet<LinkDir>) -> &'static str {
    let stairs = !LADDER_TILES.contains(&tile);
    let up = dirs.contains(&LinkDir::Up);
    let down = dirs.contains(&LinkDir::Down);
    match (stairs, up, down) {
        (true, true, false) => "Stairs up",
        (true, false, true) => "Stairs down",
        (true, ..) => "Stairs",
        (false, true, false) => "Ladder up",
        (false, false, true) => "Ladder down",
        (false, ..) => "Ladder",
    }
}

/// The town features (10.2.5, 10.2.7): signs, shops, ladders and stairs,
/// the Tavern and Sage at their keepers' cells, the exits, and the places,
/// moongates and signs of the land seen beside the floor.
pub fn town_features(shown: &TownShown, know: &GemKnowledge) -> Vec<GemFeature> {
    let (scene, floor) = (shown.scene, shown.floor);
    let (px, py) = (i32::from(shown.party.0), i32::from(shown.party.1));
    let links = link_groups(shown);
    let mut out = Vec::new();
    for (fx, fy, part, tile) in shown.cells() {
        let Some(tile) = tile else { continue };
        let distance = squared(fx - px, fy - py);
        let cell = FeatureCell::Town { fx, fy, exit: None };
        match part {
            TownPart::Strip => {}
            TownPart::Side => {
                let Some((plane, x, y)) = shown.side_world(fx, fy) else {
                    continue;
                };
                world_cell_features(plane, (x, y), tile, know, |c, k, l, key, d| {
                    out.push(feature(cell, distance, (c, k, l, key, d)));
                });
            }
            TownPart::Floor => {
                let (x, y) = (fx as u8, fy as u8);
                let mut push = |args| out.push(feature(cell, distance, args));
                let sign = FeatureKey::Sign(SignCell { scene, floor, x, y });
                let sign_known = know.is_known(&sign);
                if SIGN_TILES.contains(&tile) || sign_known {
                    push((
                        FeatureCategory::Sign,
                        sign_known,
                        "Sign",
                        Some(sign),
                        SIGN_TILES.contains(&tile),
                    ));
                }
                if SHOP_SIGN_TILES.contains(&tile) {
                    // 0xF0 and 0xF1 name no trade: "?" for good (S10.1).
                    let trade = Trade::from_sign_tile(tile);
                    let key = trade.map(|trade| FeatureKey::Trade { scene, trade });
                    push((
                        FeatureCategory::Shop,
                        key.is_some_and(|key| know.is_known(&key)),
                        trade.map_or("", trade_label),
                        key,
                        true,
                    ));
                }
                // Ladders and stairs: one feature per drawn group, at its
                // first cell, known once any of its cells was used; a used
                // cell that no longer draws a link keeps its own mark.
                let link_tile = is_link_tile(tile);
                let group = if link_tile {
                    match links.get(&(x, y)) {
                        Some(group) if group[0] == (x, y) => group.clone(),
                        _ => Vec::new(),
                    }
                } else {
                    vec![(x, y)]
                };
                let dirs: BTreeSet<LinkDir> = group
                    .iter()
                    .flat_map(|&(gx, gy)| know.town_link_dirs(scene, floor, gx, gy))
                    .collect();
                if !group.is_empty() && (link_tile || !dirs.is_empty()) {
                    push((
                        FeatureCategory::TownLink,
                        !dirs.is_empty(),
                        town_link_label(tile, &dirs),
                        Some(FeatureKey::TownLink { scene, floor, x, y }),
                        link_tile,
                    ));
                }
                // Tavern and Sage: only where the record saw the keeper (L4).
                for keeper in know.keepers_in(scene) {
                    if keeper.floor == floor && (keeper.x, keeper.y) == (x, y) {
                        push((
                            FeatureCategory::Shop,
                            true,
                            trade_label(keeper.trade),
                            Some(FeatureKey::Trade {
                                scene,
                                trade: keeper.trade,
                            }),
                            false,
                        ));
                    }
                }
            }
        }
    }
    town_exits(shown, know, &mut out);
    out
}

/// Whether a drawn floor tile is a ladder or stairs.
fn is_link_tile(tile: u8) -> bool {
    STAIR_TILES.contains(&tile) || LADDER_TILES.contains(&tile)
}

/// The drawn ladder and stairs groups of the floor: four-way adjacent
/// cells of the same kind (stairs with stairs, ladders with the same
/// ladder) are one stairway. Each cell maps to its group's cells,
/// row-major, so the first is the group's mark.
fn link_groups(shown: &TownShown) -> HashMap<(u8, u8), Vec<(u8, u8)>> {
    let kind = |tile: u8| {
        if STAIR_TILES.contains(&tile) {
            Some(0)
        } else if LADDER_TILES.contains(&tile) {
            Some(tile)
        } else {
            None
        }
    };
    let at = |x: i32, y: i32| -> Option<u8> {
        (0..TOWN_FLOOR_CELLS)
            .contains(&x)
            .then_some(())
            .and_then(|()| (0..TOWN_FLOOR_CELLS).contains(&y).then_some(()))
            .and_then(|()| shown.tile(x, y))
            .and_then(kind)
    };
    let mut out: HashMap<(u8, u8), Vec<(u8, u8)>> = HashMap::new();
    for y in 0..TOWN_FLOOR_CELLS {
        for x in 0..TOWN_FLOOR_CELLS {
            let Some(k) = at(x, y) else { continue };
            if out.contains_key(&(x as u8, y as u8)) {
                continue;
            }
            let mut group = vec![(x, y)];
            let mut next = 0;
            while next < group.len() {
                let (cx, cy) = group[next];
                next += 1;
                for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
                    if at(nx, ny) == Some(k) && !group.contains(&(nx, ny)) {
                        group.push((nx, ny));
                    }
                }
            }
            let mut cells = group
                .into_iter()
                .map(|(x, y)| (x as u8, y as u8))
                .collect::<Vec<_>>();
            cells.sort_by_key(|&(x, y)| (y, x));
            for &cell in &cells {
                out.insert(cell, cells.clone());
            }
        }
    }
    out
}

/// The floor cell at place `at` along `side`.
fn edge_cell(side: EdgeSide, at: i32) -> (i32, i32) {
    let last = TOWN_FLOOR_CELLS - 1;
    match side {
        EdgeSide::North => (at, 0),
        EdgeSide::South => (at, last),
        EdgeSide::West => (0, at),
        EdgeSide::East => (last, at),
    }
}

/// Whether the party can walk a drawn floor tile, for the exits' reach:
/// foot-passable (`tile-catalog.md §5`) or a door it can open
/// (`doors-and-z-transitions.md §2`). A hidden wall is neither.
fn walkable(tile: u8) -> bool {
    foot_terrain_accepts(tile) || is_town_door_tile(tile)
}

/// Steps from the party to each floor cell over [`walkable`] drawn tiles,
/// four ways, row-major; `None` where it cannot walk.
fn reach(shown: &TownShown) -> Vec<Option<u16>> {
    let side = TOWN_FLOOR_CELLS as usize;
    let mut steps = vec![None; side * side];
    let (px, py) = (usize::from(shown.party.0), usize::from(shown.party.1));
    if px >= side || py >= side {
        return steps;
    }
    let mut queue = std::collections::VecDeque::from([(px, py)]);
    steps[py * side + px] = Some(0u16);
    while let Some((x, y)) = queue.pop_front() {
        let here = steps[y * side + x].unwrap_or(0);
        let next = [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ];
        for (nx, ny) in next {
            if nx >= side || ny >= side || steps[ny * side + nx].is_some() {
                continue;
            }
            if !shown.tile(nx as i32, ny as i32).is_some_and(walkable) {
                continue;
            }
            steps[ny * side + nx] = Some(here + 1);
            // An edge cell is where the party leaves: the walk stops there,
            // so land outside the walls is reached only through a gate.
            if nx == 0 || ny == 0 || nx == side - 1 || ny == side - 1 {
                continue;
            }
            queue.push_back((nx, ny));
        }
    }
    steps
}

/// The exits (10.2.7): each run of foot-passable edge cells as drawn
/// (`tile-catalog.md §5`) that the party can walk to over the drawn floor,
/// marked at the run's cell it reaches first (the gate it would leave
/// by). The walk stops at the edge, where the party leaves. A hidden wall
/// is not passable, so a hidden way out is never marked, and a strip of
/// land outside the walls the party cannot reach is no exit unless the
/// party left by it. An exit the party used keeps its mark at the cell it
/// crossed when its run is out of reach or no longer drawn.
fn town_exits(shown: &TownShown, know: &GemKnowledge, out: &mut Vec<GemFeature>) {
    let scene = shown.scene;
    let (px, py) = (i32::from(shown.party.0), i32::from(shown.party.1));
    let steps = reach(shown);
    let side_cells = TOWN_FLOOR_CELLS as usize;
    let steps_at = |(fx, fy): (i32, i32)| steps[fy as usize * side_cells + fx as usize];
    let used: Vec<ExitRun> = know
        .exits()
        .iter()
        .filter(|run| run.scene == scene)
        .copied()
        .collect();
    for side in [
        EdgeSide::North,
        EdgeSide::East,
        EdgeSide::South,
        EdgeSide::West,
    ] {
        let passable = |at: i32| {
            let (fx, fy) = edge_cell(side, at);
            shown.tile(fx, fy).is_some_and(foot_terrain_accepts)
        };
        let mut covered = Vec::new();
        let mut at = 0;
        while at < TOWN_FLOOR_CELLS {
            if !passable(at) {
                at += 1;
                continue;
            }
            let start = at;
            while at < TOWN_FLOOR_CELLS && passable(at) {
                at += 1;
            }
            let run = start..at;
            covered.push(run.clone());
            let known = used
                .iter()
                .find(|exit| exit.side == side && run.contains(&i32::from(exit.at)));
            // Marked where the party reaches it first; a run it cannot
            // reach only when it used it, at the cell it crossed.
            let first = run
                .clone()
                .filter_map(|at| Some((steps_at(edge_cell(side, at))?, at)))
                .min()
                .map(|(_, at)| at)
                .or(known.map(|exit| i32::from(exit.at)));
            let Some(first) = first else {
                continue;
            };
            let (fx, fy) = edge_cell(side, first);
            out.push(feature(
                FeatureCell::Town {
                    fx,
                    fy,
                    exit: Some(side),
                },
                squared(fx - px, fy - py),
                (
                    FeatureCategory::Exit,
                    known.is_some(),
                    "Exit",
                    known.map(|exit| FeatureKey::Exit(*exit)),
                    true,
                ),
            ));
        }
        for exit in used.iter().filter(|exit| exit.side == side) {
            if covered.iter().any(|run| run.contains(&i32::from(exit.at))) {
                continue;
            }
            let (fx, fy) = edge_cell(side, i32::from(exit.at));
            if shown.tile(fx, fy).is_none() {
                continue;
            }
            out.push(feature(
                FeatureCell::Town {
                    fx,
                    fy,
                    exit: Some(side),
                },
                squared(fx - px, fy - py),
                (
                    FeatureCategory::Exit,
                    true,
                    "Exit",
                    Some(FeatureKey::Exit(*exit)),
                    false,
                ),
            ));
        }
    }
}

/// A dungeon ladder's label (10.2.7) from its class: `Up`, `Down`, or
/// `Exit up` on level 1 and `Exit down` on the deepest level; `Up/Down`
/// for the two-way ladder.
fn ladder_label(byte: u8, level: u8) -> &'static str {
    match byte >> 4 {
        0x1 if level == 0 => "Exit up",
        0x1 => "Up",
        0x2 if level + 1 == DUNGEON_LEVEL_COUNT => "Exit down",
        0x2 => "Down",
        0x3 => "Up/Down",
        _ => "Ladder",
    }
}

/// The kind a drawn dungeon byte carries a "?" for, under `glyphs`.
fn dungeon_kind(drawn: u8, glyphs: GlyphPolicy) -> Option<MarkKind> {
    match drawn >> 4 {
        0x1..=0x3 => Some(MarkKind::Ladder),
        0x4 => Some(MarkKind::Chest),
        0x5 => Some(MarkKind::Fountain),
        0x6 if drawn & 0x07 == 0 => Some(MarkKind::Trap),
        0x6 if glyphs == GlyphPolicy::GemFaithful => Some(MarkKind::Trap),
        _ => None,
    }
}

/// The dungeon features (10.2.6, 10.2.7) of every level with shown cells:
/// ladders, fountains, closed chests and visible pits as drawn, and the
/// ones the party found or used where the drawn glyph no longer shows them
/// (an opened chest). Secret doors are never a feature.
pub fn dungeon_features(shown: &DungeonShown, know: &GemKnowledge) -> Vec<GemFeature> {
    let (px, py) = (i32::from(shown.party.0), i32::from(shown.party.1));
    let mut out = Vec::new();
    for level in 0..DUNGEON_LEVEL_COUNT {
        for (x, y, cell) in shown.cells(level) {
            let distance = squared(i32::from(x) - px, i32::from(y) - py)
                + if level == shown.level { 0 } else { 400 };
            let at = FeatureCell::Dungeon { level, x, y };
            let mark = |kind| DungeonMark {
                record: shown.record,
                level,
                x,
                y,
                kind,
            };
            let drawn_kind = dungeon_kind(cell.drawn, shown.glyphs);
            for kind in [
                MarkKind::Ladder,
                MarkKind::Fountain,
                MarkKind::Chest,
                MarkKind::Trap,
            ] {
                let key = FeatureKey::DungeonMark(mark(kind));
                let known = know.is_known(&key);
                let drawn = drawn_kind == Some(kind);
                if !drawn && !known {
                    continue;
                }
                let (category, label) = match kind {
                    MarkKind::Ladder => (
                        FeatureCategory::Ladder,
                        ladder_label(if drawn { cell.drawn } else { cell.raw }, level),
                    ),
                    MarkKind::Fountain => (FeatureCategory::Fountain, "Fountain"),
                    MarkKind::Chest => (FeatureCategory::Chest, "Chest"),
                    _ => (FeatureCategory::Trap, "Trap"),
                };
                out.push(feature(
                    at,
                    distance,
                    (category, known, label, Some(key), drawn),
                ));
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;
