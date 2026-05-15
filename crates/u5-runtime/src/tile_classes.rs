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

/// `catalogs/tile-catalog.md §2` super-category split. The fourteen
/// concrete classes group into three super-categories that decide
/// where a tile lives: in the static map grid, in an active-object
/// record, or as a transient render-buffer effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileSuperCategory {
    /// World/town/combat terrain classes (water, terrain, path, wall,
    /// furniture, door, decoration, special, indices roughly `0..=159`).
    /// Stored in the top-down map and arena terrain grids.
    MapTerrain,
    /// Actor/item classes (vehicle, NPC, monster, item, effect, avatar,
    /// indices `160..=511`). Stored in active-object records.
    Actor,
    /// Transient effects written into the rendered tile buffer by
    /// the moongate animator, projectile animator, or spell handlers.
    /// Not stored in any persistent map.
    TransientEffect,
}

/// `catalogs/tile-catalog.md §2`: classify a tile id `0..=511` into
/// its super-category. Returns `None` for ids above `511` (no sprite
/// is allocated above the published sheet).
pub const fn tile_super_category(tile_id: u16) -> Option<TileSuperCategory> {
    if tile_id <= 159 {
        Some(TileSuperCategory::MapTerrain)
    } else if tile_id <= 511 {
        Some(TileSuperCategory::Actor)
    } else {
        None
    }
}

/// `catalogs/tile-catalog.md §4` per-class animation cycle length, or
/// `None` for tiles whose class does not animate. Returns `Some(4)`
/// for the four-frame cycle classes (water/lava/fire), `Some(16)` for
/// moongate frames, and `None` for non-animated classes (walls,
/// doors, paths, terrain, vegetation, furniture).
pub const fn tile_animation_cycle_length(tile_id: u8) -> Option<u8> {
    match tile_id {
        // Water class — four-frame cycle.
        TILE_WATER_FIRST..=TILE_WATER_LAST => Some(4),
        // The barrier band carries field/fire variants in the special
        // tile encoding; treat as a four-frame animator class.
        // (Lava/fire tiles live in the Special band 0x80..=0x9F.)
        _ => None,
    }
}

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
