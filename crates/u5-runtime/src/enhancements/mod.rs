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
//! - [`source`]: the host contract: [`SettingsSource`], [`SettingsSink`],
//!   [`CliSetting`], [`HostCapabilities`], the parity [`LockState`] and the
//!   [`SettingNote`]s resolution reports (design 1.3.3, 1.3.11, 1.3.14).
//! - [`mod@resolve`]: the pure, total [`resolve()`] that turns those into
//!   [`ResolvedEnhancements`] (design 1.3.2, 1.3.3).
//!
//! Nothing in the game calls into this module yet. The desktop host's
//! adapter, command-line flags and parity lock live in `u5-tui`
//! (`enhancement_settings`); the launch config that hands the result to the
//! shell follows in M0-10.

pub mod catalogue;
pub mod resolve;
pub mod source;

pub use catalogue::{
    ApplyAt, CatalogueRow, EffectClass, FeatureToggles, OPTIONS, OptionDefault, OptionKind,
    OptionSpec, PRESETS, Phase, Preset, PresetEntry, PresetValue, SHIPPED_PHASES, SoftDep,
    TOGGLE_KEYS, TOGGLES, Tag, Tier, Toggle, ToggleSpec, settings_table,
};
pub use resolve::{
    CHEATS_SWITCH, KeyBindings, PRESET_KEY, PresetLabel, ResolveInput, ResolvedEnhancements,
    SettingValues, resolve,
};
pub use source::{
    Capability, CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue,
    SettingsSink, SettingsSource, ValueProblem,
};
