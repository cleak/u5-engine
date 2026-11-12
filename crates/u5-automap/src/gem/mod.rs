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
//! - [`layout`]: the gem canvas and the outdoor layout (10.2.4, D-M3.3,
//!   M3-10);
//! - [`shown`]: the shown set, the cells drawn and their tiles, cached per
//!   open in [`GemViewState`] (10.2.3, D-M3.5, M3-10);
//! - [`raster`]: [`render_gem_map`], the CPU raster (10.3.4, M3-10).

pub mod knowledge;
pub mod layout;
pub mod payload;
pub mod probe;
pub mod probe_more;
pub mod raster;
pub mod seen;
pub mod session;
pub mod shown;

pub use knowledge::{
    DungeonMark, EdgeSide, ExitRun, FeatureKey, GEM_PAYLOAD_VERSION, GEM_SIDECAR_TAG, GateCell,
    GemKnowledge, KeeperMark, KnowEvent, LinkDir, MarkKind, SIDE_WINDOW_BYTES, SIDE_WINDOW_HEIGHT,
    SIDE_WINDOW_WIDTH, SIGN_TRADES, SignCell, TRADES, TownLink, Trade,
};
pub use layout::{GEM_CLASSIC_CANVAS, GemCanvas, OUTDOOR_CELL_PX, OutdoorLayout, gem_canvas};
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
    GemImage, GemRender, MapArt, PARTY_MARKER_PX, PARTY_SPRITE_PX, map_percent, party_sprite_tile,
    render_gem_map,
};
pub use seen::{
    GemWindow, SideWindow, faithful_window_cells, frame_seen_events, gem_view_events,
    observation_open, side_window, side_windows,
};
pub use session::{GEM_SUMMARY_PREFIX, GemSession, summary_line};
pub use shown::{GemShown, GemViewState, OutdoorShown, gem_view_open};
