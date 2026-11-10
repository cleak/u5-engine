//! The `[display]` settings as the engine sees them (Phase B design 2.3.2,
//! 2.4.1): scaling, fullscreen, side fill, the grow-left layout and vsync.
//!
//! The rows themselves are in the catalogue ([`super::catalogue`]), which
//! stays the single list of keys. This module holds their word lists and
//! the typed view [`ResolvedEnhancements::display`], which turns the
//! resolved values into the plain [`DisplaySettings`] struct the shell
//! inserts as a resource. It is plain data: no serde, no Bevy, no paths.
//!
//! Every key is effect class (a), presentation only. With every key at its
//! default, `DisplaySettings::default()` is the 1.0.1 display: Fit
//! scaling, DOS pixels, windowed, black bars, no grow-left layout and the
//! 1.0.1 present mode (vsync).
//!
//! `display.raster` is in the catalogue but not in this struct: it feeds
//! the existing launch parameter (`--raster-depth`), which the host reads
//! with [`raster_word`].

use super::catalogue::Toggle;
use super::resolve::ResolvedEnhancements;
use crate::PeekLightPolicy;

pub const SCALE_MODE_KEY: &str = "display.scale_mode";
pub const PIXEL_ASPECT_KEY: &str = "display.pixel_aspect";
pub const FULLSCREEN_KEY: &str = "display.fullscreen";
pub const FULLSCREEN_KEY_KEY: &str = "display.fullscreen_key";
pub const PILLAR_FILL_KEY: &str = "display.pillar_fill";
pub const PILLAR_COLOR_KEY: &str = "display.pillar_color";
pub const BEZEL_IMAGE_KEY: &str = "display.bezel_image";
pub const GROW_LEFT_KEY: &str = "display.grow_left";
pub const GROW_LEFT_LIGHT_KEY: &str = "display.grow_left_light";
pub const CRAWL_KEY: &str = "display.crawl";
pub const TYPED_COLOUR_KEY: &str = "display.typed_colour";
pub const RIBBON_HP_COLOURS_KEY: &str = "display.ribbon_hp_colours";
pub const DITHER_KEY: &str = "display.dither";
pub const CENTRE_PARTY_KEY: &str = "display.centre_party";
pub const VSYNC_KEY: &str = "display.vsync";
pub const RASTER_KEY: &str = "display.raster";

/// `display.scale_mode`, in [`ScaleMode`] order.
pub const SCALE_MODE_WORDS: &[&str] = &[
    "fit",
    "sharp",
    "integer-square",
    "integer-hybrid",
    "stretch",
];
/// `display.pixel_aspect`, in [`PixelAspect`] order.
pub const PIXEL_ASPECT_WORDS: &[&str] = &["dos", "square"];
/// `display.pillar_fill`, in [`PillarFill`] order.
pub const PILLAR_FILL_WORDS: &[&str] = &["black", "solid", "ribbon", "bezel"];
/// `display.grow_left_light`, in [`PeekLightPolicy`] order.
pub const GROW_LEFT_LIGHT_WORDS: &[&str] = &["scaled", "faithful"];
/// `display.raster`: both render the 16-colour EGA tiles (2.4.1).
pub const RASTER_WORDS: &[&str] = &["ega", "tandy"];

/// The five grow-left sub-toggles and `grow_left_light`: sub-options of
/// `display.grow_left`, inert while it is off (2.3.2). Changing one alone
/// leaves the game Classic.
pub const GROW_LEFT_SUB_KEYS: &[&str] = &[
    GROW_LEFT_LIGHT_KEY,
    CRAWL_KEY,
    TYPED_COLOUR_KEY,
    RIBBON_HP_COLOURS_KEY,
    DITHER_KEY,
    CENTRE_PARTY_KEY,
];

/// Every toggle of the `[display]` section: the three on/off keys and the
/// switches of the four valued keys that are features in their own right
/// (on while the value differs from its default). The shell's display
/// plugin is added while one of them is on (2.3.4).
pub const DISPLAY_TOGGLES: &[Toggle] = &[
    Toggle::DisplayScaleMode,
    Toggle::DisplayPixelAspect,
    Toggle::DisplayFullscreen,
    Toggle::DisplayFullscreenKey,
    Toggle::DisplayPillarFill,
    Toggle::DisplayGrowLeft,
    Toggle::DisplayVsync,
];

/// Keys that removed designs used. A stored value for one resolves to
/// "unknown key" with a note and is otherwise ignored (2.4.1).
pub const REMOVED_DISPLAY_KEYS: &[&str] = &[
    "display.history_panel",
    "display.history_side",
    "display.history_lines",
    "display.history_wheel",
    "display.status_panel",
    "display.status_side",
    "display.status_virtue_line",
    "display.wide_view",
    "display.wide_light",
    "display.wide_objects",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleMode {
    #[default]
    Fit,
    Sharp,
    IntegerSquare,
    IntegerHybrid,
    Stretch,
}

impl ScaleMode {
    const ALL: [ScaleMode; 5] = [
        ScaleMode::Fit,
        ScaleMode::Sharp,
        ScaleMode::IntegerSquare,
        ScaleMode::IntegerHybrid,
        ScaleMode::Stretch,
    ];

    /// The word `display.scale_mode` stores.
    pub fn key(self) -> &'static str {
        SCALE_MODE_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.key() == word)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PixelAspect {
    /// 1.2, the shell's `DISPLAY_PIXEL_ASPECT`: the 1.0.1 picture.
    #[default]
    Dos,
    Square,
}

impl PixelAspect {
    pub fn key(self) -> &'static str {
        PIXEL_ASPECT_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [PixelAspect::Dos, PixelAspect::Square]
            .into_iter()
            .find(|aspect| aspect.key() == word)
    }
}

/// Decoded by the host adapter; the engine never sees a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BezelImage {
    pub width: u32,
    pub height: u32,
    pub rgba: std::sync::Arc<[u8]>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PillarFill {
    #[default]
    Black,
    Solid([u8; 3]),
    Ribbon,
    /// `None` until the adapter has decoded `bezel_image`.
    Bezel(Option<BezelImage>),
}

impl PillarFill {
    /// The word `display.pillar_fill` stores for this fill.
    pub fn key(&self) -> &'static str {
        match self {
            PillarFill::Black => "black",
            PillarFill::Solid(_) => "solid",
            PillarFill::Ribbon => "ribbon",
            PillarFill::Bezel(_) => "bezel",
        }
    }
}

/// The grow-left layout and its sub-toggles (2.2, 2.3.2). Each sub-key is
/// its own toggle and works with any mix of the others; all are inert while
/// `on` is false.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowLeft {
    pub on: bool,
    pub light: PeekLightPolicy,
    pub crawl: bool,
    pub typed_colour: bool,
    pub ribbon_hp_colours: bool,
    pub dither: bool,
    pub centre_party: bool,
}

/// By hand: everything on except `centre_party`, and `light` = Scaled; the
/// layout itself off.
impl Default for GrowLeft {
    fn default() -> Self {
        Self {
            on: false,
            light: PeekLightPolicy::Scaled,
            crawl: true,
            typed_colour: true,
            ribbon_hp_colours: true,
            dither: true,
            centre_party: false,
        }
    }
}

/// The typed `[display]` view the shell receives (2.3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplaySettings {
    pub scale_mode: ScaleMode,
    pub pixel_aspect: PixelAspect,
    pub fullscreen: bool,
    pub fullscreen_key: bool,
    pub pillar_fill: PillarFill,
    pub grow_left: GrowLeft,
    /// True is the 1.0.1 present mode.
    pub vsync: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            scale_mode: ScaleMode::default(),
            pixel_aspect: PixelAspect::default(),
            fullscreen: false,
            fullscreen_key: false,
            pillar_fill: PillarFill::default(),
            grow_left: GrowLeft::default(),
            vsync: true,
        }
    }
}

impl PeekLightPolicy {
    /// The word `display.grow_left_light` stores.
    pub fn key(self) -> &'static str {
        match self {
            PeekLightPolicy::Scaled => GROW_LEFT_LIGHT_WORDS[0],
            PeekLightPolicy::Faithful => GROW_LEFT_LIGHT_WORDS[1],
        }
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [PeekLightPolicy::Scaled, PeekLightPolicy::Faithful]
            .into_iter()
            .find(|policy| policy.key() == word)
    }
}

/// `#rrggbb` as bytes.
fn parse_colour(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

impl ResolvedEnhancements {
    /// The `[display]` section as [`DisplaySettings`] (design 1.3.2,
    /// 2.3.2). The resolver has already checked every value, so a missing
    /// or unexpected one can only come from a hand-built value table; it
    /// reads as the key's default. The bezel is `Bezel(None)`: the host
    /// decodes the image (2.3.3).
    pub fn display(&self) -> DisplaySettings {
        let values = &self.values;
        let flag = |key: &str, default: bool| values.bool(key).unwrap_or(default);
        let defaults = GrowLeft::default();
        let pillar_fill = match values.str(PILLAR_FILL_KEY) {
            Some("solid") => PillarFill::Solid(
                values
                    .str(PILLAR_COLOR_KEY)
                    .and_then(parse_colour)
                    .unwrap_or([0, 0, 0]),
            ),
            Some("ribbon") => PillarFill::Ribbon,
            Some("bezel") => PillarFill::Bezel(None),
            _ => PillarFill::Black,
        };
        DisplaySettings {
            scale_mode: values
                .str(SCALE_MODE_KEY)
                .and_then(ScaleMode::from_key)
                .unwrap_or_default(),
            pixel_aspect: values
                .str(PIXEL_ASPECT_KEY)
                .and_then(PixelAspect::from_key)
                .unwrap_or_default(),
            fullscreen: self.toggles.on(Toggle::DisplayFullscreen),
            fullscreen_key: self.toggles.on(Toggle::DisplayFullscreenKey),
            pillar_fill,
            grow_left: GrowLeft {
                on: self.toggles.on(Toggle::DisplayGrowLeft),
                light: values
                    .str(GROW_LEFT_LIGHT_KEY)
                    .and_then(PeekLightPolicy::from_key)
                    .unwrap_or_default(),
                crawl: flag(CRAWL_KEY, defaults.crawl),
                typed_colour: flag(TYPED_COLOUR_KEY, defaults.typed_colour),
                ribbon_hp_colours: flag(RIBBON_HP_COLOURS_KEY, defaults.ribbon_hp_colours),
                dither: flag(DITHER_KEY, defaults.dither),
                centre_party: flag(CENTRE_PARTY_KEY, defaults.centre_party),
            },
            vsync: flag(VSYNC_KEY, true),
        }
    }
}

/// The `display.raster` word for the host's launch parameter (2.4.1):
/// `ega` or `tandy`.
pub fn raster_word(resolved: &ResolvedEnhancements) -> &str {
    resolved
        .values
        .str(RASTER_KEY)
        .filter(|word| RASTER_WORDS.contains(word))
        .unwrap_or(RASTER_WORDS[0])
}

#[cfg(test)]
mod tests;
