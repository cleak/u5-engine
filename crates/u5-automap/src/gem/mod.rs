//! The enhanced gem map (Phase B design chapter 10, 10.3.1).
//!
//! - [`knowledge`]: the [`GemKnowledge`] record of what the player has
//!   discovered, its [`KnowEvent`]s and [`FeatureKey`]s (10.3.2);
//! - [`probe`]: the observer, [`KnowProbe`], [`probe`](probe::probe) and
//!   [`observe_command`], which read the game and never change it (10.3.3).

pub mod knowledge;
pub mod probe;

pub use knowledge::{
    DungeonMark, EdgeSide, ExitRun, FeatureKey, GEM_PAYLOAD_VERSION, GEM_SIDECAR_TAG, GateCell,
    GemKnowledge, KeeperMark, KnowEvent, LinkDir, MarkKind, SIDE_WINDOW_BYTES, SIDE_WINDOW_HEIGHT,
    SIDE_WINDOW_WIDTH, SIGN_TRADES, SignCell, TRADES, TownLink, Trade,
};
pub use probe::{
    KnowEvents, KnowProbe, TOWN_LINK_TILES, attach_events, edge_side, observe_command,
    observe_commands, probe,
};
