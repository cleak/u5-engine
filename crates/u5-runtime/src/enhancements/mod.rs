//! Optional enhancements: the shared foundation (Phase B design, chapter 1).
//!
//! Every enhancement is its own toggle, and with every toggle off the game
//! is the 1.0.1 game. This module is the engine-side contract for those
//! toggles. It is pure data and pure functions: it never reads
//! `config.toml`, the environment, OS paths or the command line. The host
//! (`u5-host-desktop`, `u5-tui`, or a port's own host layer) reads stored
//! settings and hands the engine already-resolved values.
//!
//! - [`display`]: the `[display]` settings' typed view,
//!   [`ResolvedEnhancements::display`] and [`DisplaySettings`] (design
//!   2.3.2).
//! - [`catalogue`]: the toggle and option catalogue, effect classes, apply
//!   points, preset tiers, shipping phases and the three presets
//!   (design 1.3.2, 1.4.1 and 1.4.2).
//! - [`source`]: the host contract: [`SettingsSource`], [`SettingsSink`],
//!   [`CliSetting`], [`HostCapabilities`], the parity [`LockState`] and the
//!   [`SettingNote`]s resolution reports (design 1.3.3, 1.3.11, 1.3.14).
//! - [`mod@resolve`]: the pure, total [`resolve()`] that turns those into
//!   [`ResolvedEnhancements`] (design 1.3.2, 1.3.3).
//! - [`digest`]: the [`trajectory_digest`] of a [`PlayState`](crate::PlayState)
//!   that the equivalence harness compares runs with (design 1.3.12).
//! - [`matrix`]: the toggle matrix the equivalence harness runs, derived
//!   from the catalogue: each key alone, a pairwise covering array and all
//!   on except cheats (design 1.3.12).
//! - [`idle`]: the idle-prompt predicate's runtime half,
//!   [`PlayState::idle_command_prompt`](crate::PlayState::idle_command_prompt)
//!   (design 1.3.7).
//! - [`sidecar`]: per-save sidecars: the [`SaveWatcher`] that detects a save
//!   from outside the runtime and its [`SaveStamp`] (design 1.3.9), and the
//!   [`SidecarStore`] that keeps each feature's file beside the save, with
//!   its container and [`Freshness`] (design 1.3.10).
//!
//! Nothing in the game calls into this module yet. The desktop host's
//! adapter, command-line flags and parity lock live in `u5-tui`
//! (`enhancement_settings`); the shell receives the result in
//! `u5_bevy::VisualLaunchConfig`.

pub mod catalogue;
pub mod digest;
pub mod display;
pub mod idle;
pub mod matrix;
pub mod resolve;
pub mod sidecar;
pub mod source;

pub use catalogue::{
    ApplyAt, CatalogueRow, EffectClass, FeatureToggles, OPTIONS, OptionDefault, OptionKind,
    OptionSpec, PANEL_WORDS, PRESETS, Phase, Preset, PresetEntry, PresetValue, SHIPPED_PHASES,
    SIDE_CLAIM_KEYS, SIDE_SPACE_KEYS, SoftDep, TOGGLE_KEYS, TOGGLES, Tag, Tier, Toggle, ToggleSpec,
    settings_table,
};
pub use catalogue::{DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, LAUNCH_PARAMETER_KEYS};
pub use digest::{
    DIGEST_GROUPS, SKIPPED_FIELDS, TrajectoryDigest, digest_groups, trajectory_digest,
};
pub use display::{
    BEZEL_MAX_SIDE, BezelDecode, BezelImage, BezelMailbox, DISPLAY_TOGGLES, DisplaySettings,
    GROW_LEFT_SUB_KEYS, GrowLeft, PillarFill, PixelAspect, RASTER_KEY, REMOVED_DISPLAY_KEYS,
    ScaleMode, raster_word,
};
pub use idle::{GUARDED_DRAINS, GuardedDrain, IdleBlocker, IdlePromptKind};
pub use matrix::{
    ALL_ON_EXCEPT_CHEATS, CHEATS_SECTION, MATRIX_FILE, MatrixRow, PairwiseFactor, alternate_values,
    covering_array, matrix_tsv, pairwise_factors, toggle_matrix,
};
pub use resolve::{
    CHEATS_SWITCH, KeyBindings, PRESET_KEY, PresetLabel, ResolveInput, ResolvedEnhancements,
    SettingValues, resolve, resolve_as_shipped,
};
pub use sidecar::{
    CONTAINER_MAGIC, CONTAINER_VERSION, Container, ContainerError, Freshness,
    SAVE_META_PAYLOAD_VERSION, SIDECAR_DIR, SIDECAR_FILES, SIDECAR_KEYS, SaveMeta, SaveStamp,
    SaveWatcher, SidecarPayload, SidecarStore, SidecarWrite, TAG_AUTOMAP, TAG_GEM_MAP, TAG_JOURNAL,
    TAG_SAVE_META, decode_container, encode_container, fnv1a64, sidecar_feature_on,
    sidecar_file_name,
};
pub use source::{
    Capability, CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue,
    SettingsSink, SettingsSource, ValueProblem,
};
