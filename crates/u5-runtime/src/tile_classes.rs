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
    /// `catalogs/tile-catalog.md` §2 names `96..103` the **River**
    /// class - "River terrain frames; not a door range".
    River,
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
/// `catalogs/tile-catalog.md §2`: the lower-half tile classes
/// (terrain, path, wall, furniture, river) tile contiguously from
/// the water range upward. Anchor each *_FIRST to the previous
/// class's *_LAST + 1 so adding or resizing a class automatically
/// shifts the later ranges.
pub const TILE_TERRAIN_FIRST: u8 = TILE_WATER_LAST + 1;
pub const TILE_TERRAIN_LAST: u8 = 0x0F;
/// Inclusive path tile range `0x10..=0x17` (stone + brick paved paths).
pub const TILE_PATH_FIRST: u8 = TILE_TERRAIN_LAST + 1;
pub const TILE_PATH_LAST: u8 = 0x17;
/// Inclusive wall tile range `0x18..=0x3F` (castle/town/dungeon walls plus
/// decorative wall variants).
pub const TILE_WALL_FIRST: u8 = TILE_PATH_LAST + 1;
pub const TILE_WALL_LAST: u8 = 0x3F;
/// Inclusive furniture tile range `0x40..=0x5F` (tables, beds, bookshelves,
/// stairs, ladders, sign posts, brazier).
pub const TILE_FURNITURE_FIRST: u8 = TILE_WALL_LAST + 1;
pub const TILE_FURNITURE_LAST: u8 = 0x5F;
/// Inclusive river tile range `0x60..=0x67`.
/// `catalogs/tile-catalog.md` §3: "96..103 | terrain | River terrain
/// frames. The obsolete door classification for this range is withdrawn."
/// §7 adds: "Top-down doors are not the obsolete contiguous decimal
/// `96..103` range; every shipped Look entry in that range is river
/// terrain." The live door identifiers are `0xB8`/`0xB9`/`0xBA`/`0xBB`
/// and the magic-locked `0x97`/`0x98`; they are owned by the command
/// predicates, not by this coarse range table.
pub const TILE_RIVER_FIRST: u8 = TILE_FURNITURE_LAST + 1;
pub const TILE_RIVER_LAST: u8 = 0x67;
/// `catalogs/tile-catalog.md §2`: the upper-half tile classes
/// (decoration, barrier, special, vehicle, vehicle-art, NPC)
/// tile contiguously from the river range upward. Anchor each
/// *_FIRST to the previous class's *_LAST + 1 so adding or
/// resizing a class automatically shifts the later ranges.
pub const TILE_DECORATION_FIRST: u8 = TILE_RIVER_LAST + 1;
pub const TILE_DECORATION_LAST: u8 = 0x6F;
/// Inclusive Sceptre-dissolvable barrier/field range `0x70..=0x7F`.
pub const TILE_BARRIER_FIRST: u8 = TILE_DECORATION_LAST + 1;
pub const TILE_BARRIER_LAST: u8 = 0x7F;
/// Inclusive special-tile range `0x80..=0x9F` (pendulum, shrines, fountains,
/// fields, fire/poison/sleep effects).
pub const TILE_SPECIAL_FIRST: u8 = TILE_BARRIER_LAST + 1;
pub const TILE_SPECIAL_LAST: u8 = 0x9F;
/// Inclusive vehicle tile range `0xA0..=0xBB` (horse, ship, skiff, carpet).
pub const TILE_VEHICLE_FIRST: u8 = TILE_SPECIAL_LAST + 1;
pub const TILE_VEHICLE_LAST: u8 = 0xBB;
/// Inclusive vehicle-art-only range `0xBC..=0xBF` (balloon art).
pub const TILE_VEHICLE_ART_FIRST: u8 = TILE_VEHICLE_LAST + 1;
pub const TILE_VEHICLE_ART_LAST: u8 = 0xBF;
/// Inclusive NPC sprite range `0xC0..=0xFF` (townspeople, guards, named).
/// The NPC band runs to the top of the 8-bit tile-id space, so the
/// last NPC tile id is the largest representable byte. Anchored to
/// [`u8::MAX`] so the band-last value derives from the tile-id width
/// rather than restating `0xFF` as a bare literal.
pub const TILE_NPC_FIRST: u8 = TILE_VEHICLE_ART_LAST + 1;
pub const TILE_NPC_LAST: u8 = u8::MAX;

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

// The withdrawn `TileAnimationFamily` / `tile_animation_family` /
// `tile_animation_cycle_length` trio used to live here. It classified
// `0x01..=0x03` water as a four-frame tile-id family, `0xEC..=0xEF` as a
// "whirlpool" family and moongate frames as a sixteen-frame cycle.
//
// `RETRACTIONS.md` R148 withdrew all of that: "The animator touches exactly
// five id ranges and no others ... and no water, lava, torch or brazier tile
// is among them", `0xEC..=0xEF` is the standard of Britannia, and
// `overworld.md §9.1` says the moongate counter "is not a member of the
// global tile-animation families ... it has no frame selector".
//
// The surviving classifier is [`crate::static_tile_animation_family`], which
// reads the five published families and has no catch-all. Water does animate,
// but through the display-layer treatment in [`crate::water_scroll`], not
// through a tile-id family.

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
        TILE_RIVER_FIRST..=TILE_RIVER_LAST => TileClass::River,
        TILE_DECORATION_FIRST..=TILE_DECORATION_LAST => TileClass::Decoration,
        TILE_BARRIER_FIRST..=TILE_BARRIER_LAST => TileClass::Barrier,
        TILE_SPECIAL_FIRST..=TILE_SPECIAL_LAST => TileClass::Special,
        TILE_VEHICLE_FIRST..=TILE_VEHICLE_LAST => TileClass::Vehicle,
        TILE_VEHICLE_ART_FIRST..=TILE_VEHICLE_ART_LAST => TileClass::VehicleArt,
        TILE_NPC_FIRST..=TILE_NPC_LAST => TileClass::Npc,
    }
}
