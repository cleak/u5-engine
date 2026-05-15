//! Named tile-id range constants and a coarse-class classifier per
//! `catalogs/tile-catalog.md` §3.
//!
//! Gameplay systems already consume narrower predicates (door, stair, water,
//! mountain, vehicle) but a single coarse-class enum is useful for
//! presentation, debug logging, and future renderer work.

/// Coarse tile classification per `tile-catalog.md` §3 row groupings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileClass {
    Sentinel,
    Water,
    Terrain,
    Path,
    Wall,
    Furniture,
    Door,
    Decoration,
    Barrier,
    Special,
    Vehicle,
    VehicleArt,
    Npc,
}

/// Inclusive water tile range `0x01..=0x04` (deep / coastal / shoals / swamp).
pub const TILE_WATER_FIRST: u8 = 0x01;
pub const TILE_WATER_LAST: u8 = 0x04;
/// Inclusive terrain tile range `0x05..=0x0F` (grass through hills/peaks).
pub const TILE_TERRAIN_FIRST: u8 = 0x05;
pub const TILE_TERRAIN_LAST: u8 = 0x0F;
/// Inclusive path tile range `0x10..=0x17` (stone + brick paved paths).
pub const TILE_PATH_FIRST: u8 = 0x10;
pub const TILE_PATH_LAST: u8 = 0x17;
/// Inclusive wall tile range `0x18..=0x3F` (castle/town/dungeon walls plus
/// decorative wall variants).
pub const TILE_WALL_FIRST: u8 = 0x18;
pub const TILE_WALL_LAST: u8 = 0x3F;
/// Inclusive furniture tile range `0x40..=0x5F` (tables, beds, bookshelves,
/// stairs, ladders, sign posts, brazier).
pub const TILE_FURNITURE_FIRST: u8 = 0x40;
pub const TILE_FURNITURE_LAST: u8 = 0x5F;
/// Inclusive door tile range `0x60..=0x67` (door variants).
pub const TILE_DOOR_FIRST: u8 = 0x60;
pub const TILE_DOOR_LAST: u8 = 0x67;
/// Inclusive decoration tile range `0x68..=0x6F` (mosaics, banners, glyphs).
pub const TILE_DECORATION_FIRST: u8 = 0x68;
pub const TILE_DECORATION_LAST: u8 = 0x6F;
/// Inclusive Sceptre-dissolvable barrier/field range `0x70..=0x7F`.
pub const TILE_BARRIER_FIRST: u8 = 0x70;
pub const TILE_BARRIER_LAST: u8 = 0x7F;
/// Inclusive special-tile range `0x80..=0x9F` (pendulum, shrines, fountains,
/// fields, fire/poison/sleep effects).
pub const TILE_SPECIAL_FIRST: u8 = 0x80;
pub const TILE_SPECIAL_LAST: u8 = 0x9F;
/// Inclusive vehicle tile range `0xA0..=0xBB` (horse, ship, skiff, carpet).
pub const TILE_VEHICLE_FIRST: u8 = 0xA0;
pub const TILE_VEHICLE_LAST: u8 = 0xBB;
/// Inclusive vehicle-art-only range `0xBC..=0xBF` (balloon art).
pub const TILE_VEHICLE_ART_FIRST: u8 = 0xBC;
pub const TILE_VEHICLE_ART_LAST: u8 = 0xBF;
/// Inclusive NPC sprite range `0xC0..=0xFF` (townspeople, guards, named).
pub const TILE_NPC_FIRST: u8 = 0xC0;
pub const TILE_NPC_LAST: u8 = 0xFF;

/// Classify a 0..=255 tile id into the coarse `TileClass` group from
/// `catalogs/tile-catalog.md` §3. Distinct from `tile_helpers::tile_class`,
/// which returns a short label string for player-facing diagnostics.
pub const fn coarse_tile_class(tile: u8) -> TileClass {
    match tile {
        0x00 => TileClass::Sentinel,
        TILE_WATER_FIRST..=TILE_WATER_LAST => TileClass::Water,
        TILE_TERRAIN_FIRST..=TILE_TERRAIN_LAST => TileClass::Terrain,
        TILE_PATH_FIRST..=TILE_PATH_LAST => TileClass::Path,
        TILE_WALL_FIRST..=TILE_WALL_LAST => TileClass::Wall,
        TILE_FURNITURE_FIRST..=TILE_FURNITURE_LAST => TileClass::Furniture,
        TILE_DOOR_FIRST..=TILE_DOOR_LAST => TileClass::Door,
        TILE_DECORATION_FIRST..=TILE_DECORATION_LAST => TileClass::Decoration,
        TILE_BARRIER_FIRST..=TILE_BARRIER_LAST => TileClass::Barrier,
        TILE_SPECIAL_FIRST..=TILE_SPECIAL_LAST => TileClass::Special,
        TILE_VEHICLE_FIRST..=TILE_VEHICLE_LAST => TileClass::Vehicle,
        TILE_VEHICLE_ART_FIRST..=TILE_VEHICLE_ART_LAST => TileClass::VehicleArt,
        TILE_NPC_FIRST..=TILE_NPC_LAST => TileClass::Npc,
    }
}
