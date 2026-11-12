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
//!   2.3.2), and chapter 9's [`ResolvedEnhancements::display_light`] and
//!   [`DisplayLight`] (9.4.1).
//! - [`gem_map`]: chapter 10's `[gem_map]` typed view,
//!   [`ResolvedEnhancements::gem_map`] and [`GemMapToggles`], and the
//!   [`GlyphPolicy`] chapter 4 shares (design 10.3.2, 10.4.1).
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
//! - [`light`]: chapter 9's colour transforms, the sRGB transfer pair,
//!   the maps' coverage curve and the [`BrightnessLut`] (design 9.3.2).
//! - [`sprite_mask`]: chapter 9's sprite background masks and halo, and
//!   [`composite_eligible`], the rule for which object cells composite
//!   (design 9.3.6, 9.3.9).
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
pub mod gem_map;
pub mod idle;
pub mod light;
pub mod matrix;
pub mod resolve;
pub mod sidecar;
pub mod source;
pub mod sprite_mask;

pub use catalogue::{
    ApplyAt, CatalogueRow, EffectClass, FeatureToggles, OPTIONS, OptionDefault, OptionKind,
    OptionSpec, PANEL_WORDS, PRESETS, Phase, Preset, PresetEntry, PresetValue, SHIPPED_PHASES,
    SIDE_CLAIM_KEYS, SIDE_SPACE_KEYS, SoftDep, TOGGLE_KEYS, TOGGLES, Tag, Tier, Toggle, ToggleSpec,
    settings_table,
};
pub use catalogue::{
    DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, GEM_G2, GEM_G3, GEM_G4, GEM_G5, LAUNCH_PARAMETER_KEYS,
    LIGHT_P1, LIGHT_P2, LIGHT_P3, LIGHT_P5, named_stops,
};
pub use digest::{
    DIGEST_GROUPS, SKIPPED_FIELDS, TrajectoryDigest, digest_groups, trajectory_digest,
};
pub use display::{
    BEZEL_MAX_SIDE, BRIGHTNESS_OFF, BRIGHTNESS_SUGGESTED, BezelDecode, BezelImage, BezelMailbox,
    BrightnessScope, DISPLAY_TOGGLES, DisplayLight, DisplaySettings, GROW_LEFT_SUB_KEYS, GrowLeft,
    LIGHT_SUB_KEYS, PillarFill, PixelAspect, RASTER_KEY, REMOVED_DISPLAY_KEYS, ScaleMode,
    raster_word,
};
pub use gem_map::{
    GEM_MAP_BRIGHTNESS_DEFAULT, GEM_MAP_BRIGHTNESS_MAX, GEM_MAP_BRIGHTNESS_MIN, GEM_MAP_SUB_KEYS,
    GLYPH_WORDS, GemMapOptions, GemMapToggles, GlyphPolicy,
};
pub use idle::{GUARDED_DRAINS, GuardedDrain, IdleBlocker, IdlePromptKind};
pub use light::{
    BRIGHTNESS_CURVE_EXPONENT, BrightnessLut, coverage_scale, linear_to_srgb, scale_channel,
    srgb_to_linear,
};
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
pub use sprite_mask::{
    ACTOR_HALF_TILES, COMBAT_FIELDS, DARK_BODY, FURNITURE_UNDER, KEEP_THE_BOX, NEAR_EMPTY_PIXELS,
    SpriteMasks, TileMask, composite_eligible, flood_mask, halo_of, keeps_box, mask_bit,
    mask_count,
};
