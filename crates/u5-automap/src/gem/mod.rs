//! The enhanced gem map (Phase B design chapter 10, 10.3.1).
//!
//! - [`knowledge`]: the [`GemKnowledge`] record of what the player has
//!   discovered, its [`KnowEvent`]s and [`FeatureKey`]s (10.3.2);
//! - [`probe`]: the observer, [`KnowProbe`], [`probe`](probe::probe) and
//!   [`observe_command`], which read the game and never change it (10.3.3);
//! - [`probe_more`]: the observer's Look and dungeon events (M3-06);
//! - [`payload`]: the `GEMK` payload codec and the attach on the
//!   foundation's sidecar store (10.3.7, M3-07);
//! - [`seen`]: the dungeon and town-side cells the player has seen, per
//!   frame and when a gem view opens (10.3.3, M3-06);
//! - [`session`]: one play session's record as both frontends drive it:
//!   attach, probe and observe, write after a save, new game (10.3.5,
//!   10.3.6, M3-08);
//! - [`layout`]: the gem canvas, the outdoor layout (10.2.4, D-M3.3,
//!   M3-10), the town layout (10.2.5, M3-12) and the dungeon panel table
//!   (10.2.6, M3-13);
//! - [`dungeon`]: the dungeon shown set, its panels and titles (10.2.6,
//!   M3-13);
//! - [`features`]: the features of a view, from the drawn tiles of the
//!   shown set and the record, and what each may tell (10.2.7, M3-15);
//! - [`labels`]: label placement and the order of the labels (10.2.7,
//!   L2, D10.4, M3-16);
//! - [`shown`]: the shown set, the cells drawn and their tiles, cached per
//!   open in [`GemViewState`] (10.2.3, D-M3.5, M3-10);
//! - [`raster`]: [`render_gem_map`], the CPU raster (10.3.4, M3-10).

pub mod dungeon;
pub mod features;
pub mod knowledge;
pub mod labels;
pub mod layout;
pub mod payload;
pub mod probe;
pub mod probe_more;
pub mod raster;
pub mod seen;
pub mod session;
pub mod shown;

pub use dungeon::{DungeonCell, DungeonInputs, DungeonLit, DungeonShown, DungeonSlot, leads};
pub use features::{
    CHEST_BROWN, FeatureCategory, FeatureCell, GemFeature, NEUTRAL_GREY, QUESTION_TAG_PX,
    dungeon_features, gem_features, outdoor_features, town_features, trade_label,
};
pub use knowledge::{
    DungeonMark, EdgeSide, ExitRun, FeatureKey, GEM_PAYLOAD_VERSION, GEM_SIDECAR_TAG, GateCell,
    GemKnowledge, KeeperMark, KnowEvent, LinkDir, MarkKind, SIDE_WINDOW_BYTES, SIDE_WINDOW_HEIGHT,
    SIDE_WINDOW_WIDTH, SIGN_TRADES, SignCell, TRADES, TownLink, Trade,
};
pub use labels::{
    GroupLayout, LabelDraw, LabelItem, LabelPlan, LabelText, Placed, Placement, TagAt,
    gem_label_plan, place_labels, text_tag_size,
};
pub use layout::{
    DUNGEON_MIN_CELL_PX, DUNGEON_PANEL_CELLS, DUNGEON_PANEL_TABLE, DungeonLayout,
    GEM_CLASSIC_CANVAS, GemCanvas, GlyphScale, OUTDOOR_CELL_PX, OutdoorLayout,
    TOWN_ENTRANCE_FLOOR_ROW, TOWN_FLOOR_CELLS, TownLayout, dungeon_cell_size, gem_canvas,
};
pub use payload::{
    AttachOutcome, EmptyReason, PayloadError, attach, attach_payload, decode_payload,
    encode_payload,
};
pub use probe::{
    KnowEvents, KnowProbe, TOWN_LINK_TILES, attach_events, edge_side, observe_command,
    observe_commands, probe,
};
pub use probe_more::{ENTRANCE_NAME_COLUMNS, entrance_record_for_column};
pub use raster::{
    DUNGEON_REMEMBERED_PERCENT, DUNGEON_WRAP_PERCENT, DungeonLuts, GemImage, GemRender,
    LINK_YELLOW, MapArt, PARTY_MARKER_PX, PARTY_SPRITE_PX, TOWN_SIDE_PERCENT, dungeon_glyph_pixels,
    dungeon_panel_levels, exit_arrow, facing_arrow, map_percent, party_sprite_tile, question_tag,
    remembered_channel, render_gem_map, town_side_lut,
};
pub use seen::{
    GemWindow, SideWindow, faithful_window_cells, frame_seen_events, gem_view_events,
    observation_open, side_window, side_windows,
};
pub use session::{GEM_SUMMARY_PREFIX, GemSession, summary_line};
pub use shown::{
    GemShown, GemViewState, OutdoorShown, TownInputs, TownPart, TownShown, gem_view_open,
    town_part, town_world_plane,
};
