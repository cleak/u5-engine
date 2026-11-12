//! Map memory, perceived cells and static maps shared by the automap
//! (Phase B design chapter 4) and the enhanced gem map (chapter 10).
//!
//! This crate is engine code, not host code: it uses only `std` and
//! `u5-runtime`, never Bevy, never configuration files, environment
//! variables or OS paths of its own. Any game folder it reads comes in as
//! a plain `&Path` from the frontend (design 4.3.1, 10.3.1).
//!
//! Modules (design 4.3.1, M3-02):
//! - [`perceived`]: how a remembered dungeon cell is drawn, as the player
//!   saw it (4.2.3);
//! - [`static_maps`]: a lazy, load-once cache of the world maps, town
//!   floors and dungeon records (4.3.4);
//! - [`memory`]: the remembered dungeon cells, one bit per cell (4.3.2);
//! - [`observe`]: what the faithful views show, read without changing the
//!   game, starting with the dungeon corridor's cells (4.3.3, M3-04);
//! - [`section`]: the tag, key, length section codec the `AMAP` and
//!   `GEMK` payloads share (4.3.8, 10.3.7).

#![forbid(unsafe_code)]

pub mod memory;
pub mod observe;
pub mod perceived;
pub mod section;
pub mod static_maps;

pub use memory::{DUNGEON_LEVEL_COUNT, DUNGEON_RECORDS, DUNGEON_SIDE_CELLS, DungeonBits};
pub use observe::{CorridorCell, CorridorCells, CorridorLane, dungeon_corridor_cells};
pub use perceived::{DungeonLook, PerceivedCell, perceived_dungeon_cell};
pub use section::{
    SECTION_HEADER_LEN, SectionError, SectionList, SectionRef, read_sections, write_section,
};
pub use static_maps::{DiskSource, StaticMapError, StaticMaps, StaticSource};
