//! The `[gem_map]` settings as the engine sees them (Phase B design 10.3.2,
//! 10.4.1; build plan M3-01, D-M3.2).
//!
//! The rows are in the catalogue ([`super::catalogue`]): the main toggle
//! `gem_map.enhanced` and nine valued sub-options, each inert while the
//! toggle is off. This module holds their word list, [`GlyphPolicy`], and
//! the typed view [`ResolvedEnhancements::gem_map`], which turns the
//! resolved values into the plain [`GemMapToggles`] the gem map's systems
//! read. It is plain data: no serde, no Bevy, no paths.
//!
//! [`GlyphPolicy`] is chapter 4's type too (`automap.glyphs`). It lives here
//! because the catalogue and the typed views are in `u5-runtime`, which the
//! `u5-automap` crate depends on, not the other way round (D-M3.2).
//!
//! Every key is effect class (a). With `enhanced` off the gem map does
//! nothing at all, whatever the sub-options hold (10.4.2). There is
//! deliberately no "show all names" key (10.4.1).

use super::catalogue::Toggle;
use super::resolve::ResolvedEnhancements;

pub const ENHANCED_KEY: &str = "gem_map.enhanced";
pub const LABELS_KEY: &str = "gem_map.labels";
pub const BRIGHTNESS_LINK_KEY: &str = "gem_map.brightness_link";
pub const MAP_BRIGHTNESS_KEY: &str = "gem_map.map_brightness";
pub const SWEEP_KEY: &str = "gem_map.sweep";
pub const TOWN_SURROUNDINGS_KEY: &str = "gem_map.town_surroundings";
pub const DUNGEON_STACK_KEY: &str = "gem_map.dungeon_stack";
pub const WRAP_ROWS_KEY: &str = "gem_map.wrap_rows";
pub const LEVEL_PAGING_KEY: &str = "gem_map.level_paging";
pub const GLYPHS_KEY: &str = "gem_map.glyphs";

/// The sub-options of `gem_map.enhanced`, in catalogue order (10.4.1).
pub const GEM_MAP_SUB_KEYS: &[&str] = &[
    LABELS_KEY,
    BRIGHTNESS_LINK_KEY,
    MAP_BRIGHTNESS_KEY,
    SWEEP_KEY,
    TOWN_SURROUNDINGS_KEY,
    DUNGEON_STACK_KEY,
    WRAP_ROWS_KEY,
    LEVEL_PAGING_KEY,
    GLYPHS_KEY,
];

/// The words of `gem_map.glyphs` (and later `automap.glyphs`), in
/// [`GlyphPolicy`] order.
pub const GLYPH_WORDS: &[&str] = &["perceived", "gem-faithful"];

/// `gem_map.map_brightness`'s default: the map layer at 70% (10.2.10).
pub const GEM_MAP_BRIGHTNESS_DEFAULT: u8 = 70;
/// The lowest `gem_map.map_brightness` (10.4.1).
pub const GEM_MAP_BRIGHTNESS_MIN: u8 = 25;
/// The highest `gem_map.map_brightness`: undimmed. `display.map_brightness`
/// at this value is off, so `brightness_link` changes nothing (10.4.1).
pub const GEM_MAP_BRIGHTNESS_MAX: u8 = 100;

/// How a remembered dungeon cell is drawn (design 4.2.3, 10.4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlyphPolicy {
    /// As the party perceived it: a secret door is drawn as the wall it
    /// looked like, a hidden pit as passage. The no-spoiler default.
    #[default]
    Perceived,
    /// As the original gem's own view draws it, secret doors and hidden
    /// pits included. Deviation tier, in no preset (D10.6).
    GemFaithful,
}

impl GlyphPolicy {
    /// The word the setting stores.
    pub fn key(self) -> &'static str {
        GLYPH_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [GlyphPolicy::Perceived, GlyphPolicy::GemFaithful]
            .into_iter()
            .find(|policy| policy.key() == word)
    }
}

/// The gem map's sub-options (10.3.2), inert while
/// [`GemMapToggles::enhanced`] is off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GemMapOptions {
    /// Percent, 25..=100, for the map layer; labels stay at full.
    pub map_brightness: u8,
    /// The dither sweep on open and close.
    pub sweep: bool,
    /// In towns, the dimmed land outside the gates.
    pub town_surroundings: bool,
    /// In dungeons, every remembered level; off shows the current one.
    pub dungeon_stack: bool,
    /// In dungeons, the wrapped edge rows.
    pub wrap_rows: bool,
    /// PgUp and PgDn page a one-panel dungeon map.
    pub level_paging: bool,
    pub glyphs: GlyphPolicy,
}

impl Default for GemMapOptions {
    fn default() -> Self {
        Self {
            map_brightness: GEM_MAP_BRIGHTNESS_DEFAULT,
            sweep: true,
            town_surroundings: true,
            dungeon_stack: true,
            wrap_rows: true,
            level_paging: true,
            glyphs: GlyphPolicy::Perceived,
        }
    }
}

/// The typed `[gem_map]` view (10.3.2). The default is every key at its
/// catalogue default: off, with labels on and the link off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GemMapToggles {
    /// The main toggle, `gem_map.enhanced`.
    pub enhanced: bool,
    pub labels: bool,
    /// Use `display.map_brightness` instead of `map_brightness` while it is
    /// below 100.
    pub brightness_link: bool,
    pub opts: GemMapOptions,
}

impl Default for GemMapToggles {
    fn default() -> Self {
        Self {
            enhanced: false,
            labels: true,
            brightness_link: false,
            opts: GemMapOptions::default(),
        }
    }
}

impl GemMapToggles {
    /// The map layer's brightness in percent (10.2.10): `display_map` (the
    /// value of `display.map_brightness`) when `brightness_link` is on and
    /// it is below 100, otherwise the gem map's own `map_brightness`.
    pub fn map_brightness(&self, display_map: u8) -> u8 {
        if self.brightness_link && display_map < GEM_MAP_BRIGHTNESS_MAX {
            display_map.max(GEM_MAP_BRIGHTNESS_MIN)
        } else {
            self.opts.map_brightness
        }
    }

    /// Whether PgUp and PgDn may be taken: the main toggle, paging and the
    /// stack all on (10.2.6, 10.4.1: with `dungeon_stack` off there is no
    /// paging).
    pub fn paging_keys(&self) -> bool {
        self.enhanced && self.opts.dungeon_stack && self.opts.level_paging
    }
}

impl ResolvedEnhancements {
    /// The `[gem_map]` section as [`GemMapToggles`] (10.3.2). As for
    /// [`display`](Self::display), a missing or unexpected value reads as
    /// the key's default, and a brightness outside 25..=100 is clamped. A
    /// row that has not shipped has already been resolved to its default
    /// (U13), and the parity lock gives the defaults (10.4.5).
    pub fn gem_map(&self) -> GemMapToggles {
        let values = &self.values;
        let defaults = GemMapToggles::default();
        let flag = |key: &str, default: bool| values.bool(key).unwrap_or(default);
        let map_brightness =
            values
                .int(MAP_BRIGHTNESS_KEY)
                .map_or(defaults.opts.map_brightness, |value| {
                    value.clamp(
                        i64::from(GEM_MAP_BRIGHTNESS_MIN),
                        i64::from(GEM_MAP_BRIGHTNESS_MAX),
                    ) as u8
                });
        GemMapToggles {
            enhanced: self.toggles.on(Toggle::GemMapEnhanced),
            labels: flag(LABELS_KEY, defaults.labels),
            brightness_link: flag(BRIGHTNESS_LINK_KEY, defaults.brightness_link),
            opts: GemMapOptions {
                map_brightness,
                sweep: flag(SWEEP_KEY, defaults.opts.sweep),
                town_surroundings: flag(TOWN_SURROUNDINGS_KEY, defaults.opts.town_surroundings),
                dungeon_stack: flag(DUNGEON_STACK_KEY, defaults.opts.dungeon_stack),
                wrap_rows: flag(WRAP_ROWS_KEY, defaults.opts.wrap_rows),
                level_paging: flag(LEVEL_PAGING_KEY, defaults.opts.level_paging),
                glyphs: values
                    .str(GLYPHS_KEY)
                    .and_then(GlyphPolicy::from_key)
                    .unwrap_or_default(),
            },
        }
    }
}

#[cfg(test)]
mod tests;
