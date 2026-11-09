//! Optional enhancements: the shared foundation (Phase B design, chapter 1).
//!
//! Every enhancement is its own toggle, and with every toggle off the game
//! is the 1.0.1 game. This module is the engine-side contract for those
//! toggles. It is pure data and pure functions: it never reads
//! `config.toml`, the environment, OS paths or the command line. The host
//! (`u5-host-desktop`, `u5-tui`, or a port's own host layer) reads stored
//! settings and hands the engine already-resolved values.
//!
//! - [`catalogue`]: the toggle and option catalogue, effect classes, apply
//!   points, preset tiers, shipping phases and the three presets
//!   (design 1.3.2, 1.4.1 and 1.4.2).
//!
//! Nothing in the game calls into this module yet. The resolver and the
//! host contract traits follow in the next package (M0-05).

pub mod catalogue;

pub use catalogue::{
    ApplyAt, CatalogueRow, EffectClass, FeatureToggles, OPTIONS, OptionDefault, OptionKind,
    OptionSpec, PRESETS, Phase, Preset, PresetEntry, PresetValue, SHIPPED_PHASES, SoftDep,
    TOGGLE_KEYS, TOGGLES, Tag, Tier, Toggle, ToggleSpec,
};
