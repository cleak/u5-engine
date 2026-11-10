//! The `[display]` rows and their typed view (design 2.3.2, 2.4.1; build
//! plan M1-01). The rows stay hidden until their phases ship (U13), so the
//! resolver tests use the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CatalogueRow, DISPLAY_P1, DISPLAY_P2, DISPLAY_P8, EffectClass, FeatureToggles,
    LAUNCH_PARAMETER_KEYS, OPTIONS, OptionDefault, OptionKind, Phase, Preset, Tag, Tier,
};
use crate::enhancements::resolve::{ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{
    CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue, SettingsSource,
};

const ALL_DISPLAY: &[Phase] = &[DISPLAY_P1, DISPLAY_P2, DISPLAY_P8];

/// What design 2.4.1 says one key is.
struct Expected {
    key: &'static str,
    kind: ExpectedKind,
    tier: Tier,
    apply: ApplyAt,
    ships: Phase,
    alias: Option<&'static str>,
}

enum ExpectedKind {
    /// A plain on/off toggle, default off.
    Toggle,
    Option(OptionKind, OptionDefault),
}

const fn row(
    key: &'static str,
    kind: ExpectedKind,
    ships: Phase,
    alias: Option<&'static str>,
) -> Expected {
    Expected {
        key,
        kind,
        tier: Tier::Feature,
        apply: ApplyAt::Frame,
        ships,
        alias,
    }
}

const fn bool_option(default: bool) -> ExpectedKind {
    ExpectedKind::Option(OptionKind::Bool, OptionDefault::Bool(default))
}

/// Design 2.4.1, "Every toggle", row by row.
const DESIGN_ROWS: &[Expected] = &[
    row(
        "display.scale_mode",
        ExpectedKind::Option(
            OptionKind::Enum(&[
                "fit",
                "sharp",
                "integer-square",
                "integer-hybrid",
                "stretch",
            ]),
            OptionDefault::Str("fit"),
        ),
        DISPLAY_P1,
        Some("--scale-mode"),
    ),
    row(
        "display.pixel_aspect",
        ExpectedKind::Option(
            OptionKind::Enum(&["dos", "square"]),
            OptionDefault::Str("dos"),
        ),
        DISPLAY_P1,
        Some("--pixel-aspect"),
    ),
    row(
        "display.fullscreen",
        ExpectedKind::Toggle,
        DISPLAY_P1,
        Some("--fullscreen"),
    ),
    row(
        "display.fullscreen_key",
        ExpectedKind::Toggle,
        DISPLAY_P1,
        Some("--fullscreen-key"),
    ),
    row(
        "display.pillar_fill",
        ExpectedKind::Option(
            OptionKind::Enum(&["black", "solid", "ribbon", "bezel"]),
            OptionDefault::Str("black"),
        ),
        DISPLAY_P2,
        Some("--pillar-fill"),
    ),
    row(
        "display.pillar_color",
        ExpectedKind::Option(OptionKind::Color, OptionDefault::Str("#000000")),
        DISPLAY_P2,
        None,
    ),
    row(
        "display.bezel_image",
        ExpectedKind::Option(OptionKind::Path, OptionDefault::Str("")),
        DISPLAY_P2,
        Some("--bezel-image"),
    ),
    Expected {
        tier: Tier::Deviation,
        ..row(
            "display.grow_left",
            ExpectedKind::Toggle,
            DISPLAY_P8,
            Some("--grow-left"),
        )
    },
    row(
        "display.grow_left_light",
        ExpectedKind::Option(
            OptionKind::Enum(&["scaled", "faithful"]),
            OptionDefault::Str("scaled"),
        ),
        DISPLAY_P8,
        None,
    ),
    row("display.crawl", bool_option(true), DISPLAY_P8, None),
    row("display.typed_colour", bool_option(true), DISPLAY_P8, None),
    row(
        "display.ribbon_hp_colours",
        bool_option(true),
        DISPLAY_P8,
        None,
    ),
    row("display.dither", bool_option(true), DISPLAY_P8, None),
    row("display.centre_party", bool_option(false), DISPLAY_P8, None),
    row(
        "display.vsync",
        bool_option(true),
        DISPLAY_P1,
        Some("--vsync"),
    ),
    Expected {
        apply: ApplyAt::NextLaunch,
        ..row(
            "display.raster",
            ExpectedKind::Option(
                OptionKind::Enum(&["ega", "tandy"]),
                OptionDefault::Str("ega"),
            ),
            DISPLAY_P1,
            Some("--raster-depth"),
        )
    },
];

/// The grow-left labels of 2.4.1, as the player sees them.
const DESIGN_LABELS: &[(&str, &str)] = &[
    (
        "display.grow_left",
        "Wider world view (shows more than the original)",
    ),
    (
        "display.grow_left_light",
        "Daylight reach: whole screen, or original distance",
    ),
    ("display.crawl", "Messages over the map"),
    ("display.typed_colour", "Typed text in cyan"),
    (
        "display.ribbon_hp_colours",
        "Health colours (red when low, green when poisoned)",
    ),
    ("display.dither", "Fade effects"),
    (
        "display.centre_party",
        "Keep the party centred when the side column is open",
    ),
];

/// A stored settings file.
#[derive(Default)]
struct Stored(Vec<(String, SettingValue)>);

impl Stored {
    fn with(mut self, key: &str, value: SettingValue) -> Self {
        self.0.push((key.to_string(), value));
        self
    }
}

impl SettingsSource for Stored {
    fn get(&self, key: &str) -> Option<SettingValue> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.clone())
    }

    fn keys(&self) -> Vec<String> {
        self.0.iter().map(|(key, _)| key.clone()).collect()
    }
}

fn text(value: &str) -> SettingValue {
    SettingValue::Str(value.to_string())
}

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn input<'a>(stored: Option<&'a dyn SettingsSource>, cli: &'a [CliSetting]) -> ResolveInput<'a> {
    ResolveInput {
        stored,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    }
}

/// Resolves as if every `[display]` phase had shipped.
fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), ALL_DISPLAY)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

// --- The rows. ---------------------------------------------------------

#[test]
fn every_display_row_matches_the_design() {
    let display_rows: Vec<CatalogueRow> = CatalogueRow::all()
        .filter(|row| row.key().starts_with("display."))
        .filter(|row| !["display.panel_priority", "display.panel_guard"].contains(&row.key()))
        .collect();
    assert_eq!(
        display_rows.len(),
        DESIGN_ROWS.len(),
        "one row per 2.4.1 key"
    );
    for expected in DESIGN_ROWS {
        let row = CatalogueRow::find(expected.key)
            .unwrap_or_else(|| panic!("{} is in the catalogue", expected.key));
        assert_eq!(row.ships(), expected.ships, "{}", expected.key);
        assert!(
            !row.is_shipped(),
            "{} is hidden until it ships",
            expected.key
        );
        assert_eq!(
            row.cli_aliases(),
            expected.alias.as_slice(),
            "{}",
            expected.key
        );
        assert!(!row.label().trim().is_empty(), "{}", expected.key);
        let (class, apply, tier) = match (row, &expected.kind) {
            (CatalogueRow::Toggle(spec), ExpectedKind::Toggle) => {
                assert!(!spec.driven, "{}", expected.key);
                (Some(spec.class), spec.apply, spec.tier)
            }
            (CatalogueRow::Option(spec), ExpectedKind::Option(kind, default)) => {
                assert_eq!(spec.kind, *kind, "{}", expected.key);
                assert_eq!(spec.default, *default, "{}", expected.key);
                assert!(spec.deviation_values.is_empty(), "{}", expected.key);
                (spec.class, spec.apply, spec.tier)
            }
            _ => panic!("{} has the wrong kind of row", expected.key),
        };
        assert_eq!(class, Some(EffectClass::Presentation), "{}", expected.key);
        assert_eq!(apply, expected.apply, "{}", expected.key);
        assert_eq!(tier, expected.tier, "{}", expected.key);
    }
    for (key, label) in DESIGN_LABELS {
        assert_eq!(CatalogueRow::find(key).unwrap().label(), *label);
    }
    assert!(
        CatalogueRow::find("display.grow_left")
            .unwrap()
            .tags()
            .contains(&Tag::WiderView)
    );
}

/// Every boolean row's alias has a `--no-` form; the valued rows take a
/// value. `raster` keeps the existing `--raster-depth`.
#[test]
fn boolean_display_aliases_have_a_no_form() {
    for expected in DESIGN_ROWS {
        let Some(alias) = expected.alias else {
            continue;
        };
        let switch = match CatalogueRow::find(expected.key).unwrap() {
            CatalogueRow::Toggle(_) => true,
            CatalogueRow::Option(spec) => spec.kind == OptionKind::Bool,
        };
        let expect_switch = matches!(
            expected.key,
            "display.fullscreen" | "display.fullscreen_key" | "display.grow_left" | "display.vsync"
        );
        assert_eq!(switch, expect_switch, "{alias}");
        assert!(alias.starts_with("--") && !alias.starts_with("--no-"));
    }
    assert_eq!(
        CatalogueRow::find("display.raster").unwrap().cli_aliases(),
        ["--raster-depth"]
    );
}

/// The valued keys that are features switch a toggle; the grow-left and
/// fill sub-keys are inert sub-options; `raster` is a launch parameter.
#[test]
fn display_options_switch_or_belong_to_a_toggle() {
    let option = |key: &str| OPTIONS.iter().find(|spec| spec.key == key).unwrap();
    for (key, toggle) in [
        ("display.scale_mode", Toggle::DisplayScaleMode),
        ("display.pixel_aspect", Toggle::DisplayPixelAspect),
        ("display.pillar_fill", Toggle::DisplayPillarFill),
        ("display.vsync", Toggle::DisplayVsync),
    ] {
        assert_eq!(option(key).toggle, Some(toggle), "{key}");
        assert_eq!(option(key).parent, None, "{key}");
        assert!(toggle.spec().driven);
        assert!(DISPLAY_TOGGLES.contains(&toggle));
    }
    for key in GROW_LEFT_SUB_KEYS {
        assert_eq!(option(key).parent, Some(Toggle::DisplayGrowLeft), "{key}");
        assert_eq!(option(key).toggle, None, "{key}");
    }
    for key in ["display.pillar_color", "display.bezel_image"] {
        assert_eq!(option(key).parent, Some(Toggle::DisplayPillarFill), "{key}");
    }
    assert_eq!(LAUNCH_PARAMETER_KEYS, ["display.raster"]);
    assert_eq!(
        (
            option("display.raster").toggle,
            option("display.raster").parent
        ),
        (None, None)
    );
    for toggle in [
        Toggle::DisplayFullscreen,
        Toggle::DisplayFullscreenKey,
        Toggle::DisplayGrowLeft,
    ] {
        assert!(!toggle.spec().driven);
        assert!(DISPLAY_TOGGLES.contains(&toggle));
    }
    assert_eq!(
        DISPLAY_TOGGLES.len(),
        crate::enhancements::TOGGLES
            .iter()
            .filter(|spec| spec.key.starts_with("display."))
            .count()
    );
}

#[test]
fn no_preset_sets_a_display_row_in_this_build() {
    // Everything names `scale_mode = sharp` and `fullscreen_key`; both are
    // hidden until P1 ships, so every preset is still Classic here.
    for preset in [Preset::Recommended, Preset::Everything] {
        for toggle in DISPLAY_TOGGLES {
            assert!(!preset.toggles().on(*toggle), "{preset:?} {toggle:?}");
        }
    }
    // The wider view is in no preset at all (D2.19).
    for preset in [Preset::Classic, Preset::Recommended, Preset::Everything] {
        assert!(
            preset
                .entries()
                .iter()
                .all(|entry| entry.key != GROW_LEFT_KEY && !GROW_LEFT_SUB_KEYS.contains(&entry.key)),
            "{preset:?}"
        );
    }
}

// --- The typed view. ---------------------------------------------------

#[test]
fn the_defaults_are_the_1_0_1_display() {
    let settings = ResolvedEnhancements::classic().display();
    assert_eq!(settings, DisplaySettings::default());
    assert_eq!(settings.scale_mode, ScaleMode::Fit);
    assert_eq!(settings.pixel_aspect, PixelAspect::Dos);
    assert!(!settings.fullscreen && !settings.fullscreen_key);
    assert_eq!(settings.pillar_fill, PillarFill::Black);
    assert!(settings.vsync, "vsync on is the 1.0.1 present mode");
    assert_eq!(
        settings.grow_left,
        GrowLeft {
            on: false,
            light: PeekLightPolicy::Scaled,
            crawl: true,
            typed_colour: true,
            ribbon_hp_colours: true,
            dither: true,
            centre_party: false,
        }
    );
    assert_eq!(raster_word(&ResolvedEnhancements::classic()), "ega");
    // With every phase shipped, nothing set is still the default.
    assert_eq!(shipped(None, &[]).display(), DisplaySettings::default());
    assert!(shipped(None, &[]).is_classic());
    // The lock gives the defaults whatever was asked.
    let locked = ResolvedEnhancements::locked(LockReason::QaMode);
    assert_eq!(locked.display(), DisplaySettings::default());
}

#[test]
fn the_view_reads_file_values() {
    let stored = Stored::default()
        .with("display.scale_mode", text("integer-hybrid"))
        .with("display.pixel_aspect", text("square"))
        .with("display.fullscreen", SettingValue::Bool(true))
        .with("display.fullscreen_key", SettingValue::Bool(true))
        .with("display.pillar_fill", text("solid"))
        .with("display.pillar_color", text("#AA5500"))
        .with("display.grow_left", SettingValue::Bool(true))
        .with("display.grow_left_light", text("faithful"))
        .with("display.crawl", SettingValue::Bool(false))
        .with("display.typed_colour", SettingValue::Bool(false))
        .with("display.ribbon_hp_colours", SettingValue::Bool(false))
        .with("display.dither", SettingValue::Bool(false))
        .with("display.centre_party", SettingValue::Bool(true))
        .with("display.vsync", SettingValue::Bool(false))
        .with("display.raster", text("tandy"));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert_eq!(
        resolved.display(),
        DisplaySettings {
            scale_mode: ScaleMode::IntegerHybrid,
            pixel_aspect: PixelAspect::Square,
            fullscreen: true,
            fullscreen_key: true,
            pillar_fill: PillarFill::Solid([0xAA, 0x55, 0x00]),
            grow_left: GrowLeft {
                on: true,
                light: PeekLightPolicy::Faithful,
                crawl: false,
                typed_colour: false,
                ribbon_hp_colours: false,
                dither: false,
                centre_party: true,
            },
            vsync: false,
        }
    );
    assert_eq!(raster_word(&resolved), "tandy");
    for toggle in DISPLAY_TOGGLES {
        assert!(resolved.toggles.on(*toggle), "{toggle:?}");
    }
    // Ribbon and bezel fills; the bezel waits for the host to decode it.
    let stored = Stored::default().with("display.pillar_fill", text("ribbon"));
    assert_eq!(
        shipped(Some(&stored), &[]).display().pillar_fill,
        PillarFill::Ribbon
    );
    let stored = Stored::default()
        .with("display.pillar_fill", text("bezel"))
        .with("display.bezel_image", text("frame.png"));
    assert_eq!(
        shipped(Some(&stored), &[]).display().pillar_fill,
        PillarFill::Bezel(None)
    );
}

#[test]
fn the_view_reads_command_line_values_over_the_file() {
    let stored = Stored::default()
        .with("display.scale_mode", text("stretch"))
        .with("display.grow_left", SettingValue::Bool(true))
        .with("display.vsync", SettingValue::Bool(false));
    let cli = [
        set("display.scale_mode", "sharp"),
        CliSetting::Disable("display.grow_left".into()),
        CliSetting::Enable("display.vsync".into()),
        set("display.crawl", "false"),
        CliSetting::Enable("display.fullscreen".into()),
    ];
    let resolved = shipped(Some(&stored), &cli);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let settings = resolved.display();
    assert_eq!(settings.scale_mode, ScaleMode::Sharp);
    assert!(!settings.grow_left.on);
    assert!(!settings.grow_left.crawl, "kept while its parent is off");
    assert!(settings.vsync);
    assert!(settings.fullscreen);
    let on: Vec<Toggle> = resolved.toggles.iter_on().collect();
    assert_eq!(on, [Toggle::DisplayScaleMode, Toggle::DisplayFullscreen]);
}

#[test]
fn a_bad_file_value_falls_back_to_its_default_with_a_note() {
    let stored = Stored::default()
        .with("display.grow_left_light", text("wide"))
        .with("display.scale_mode", text("huge"))
        .with("display.pillar_color", text("brown"))
        .with("display.crawl", SettingValue::Int(1))
        .with("display.grow_left", SettingValue::Bool(true));
    let resolved = shipped(Some(&stored), &[]);
    let settings = resolved.display();
    assert!(settings.grow_left.on, "the good value still applies");
    assert_eq!(settings.grow_left.light, PeekLightPolicy::Scaled);
    assert_eq!(settings.scale_mode, ScaleMode::Fit);
    assert!(settings.grow_left.crawl);
    assert_eq!(
        note_lines(&resolved),
        [
            "display.grow_left_light = wide: not one of scaled, faithful, using scaled",
            "display.scale_mode = huge: not one of fit, sharp, integer-square, \
             integer-hybrid, stretch, using fit",
            "display.pillar_color = brown: not a colour like #rrggbb, using #000000",
            "display.crawl = 1: not true or false, using true",
        ]
    );
    assert!(!resolved.toggles.on(Toggle::DisplayScaleMode));
}

#[test]
fn removed_keys_are_unknown_keys() {
    let mut stored = Stored::default();
    for key in REMOVED_DISPLAY_KEYS {
        stored = stored.with(key, SettingValue::Bool(true));
    }
    for resolved in [
        shipped(Some(&stored), &[]),
        resolve(input(Some(&stored), &[])),
    ] {
        assert!(resolved.is_classic());
        assert_eq!(resolved.notes.len(), REMOVED_DISPLAY_KEYS.len());
        for (note, key) in resolved.notes.iter().zip(REMOVED_DISPLAY_KEYS) {
            assert!(
                matches!(note, SettingNote::UnknownKey { key: k, .. } if k == key),
                "{note}"
            );
            assert!(note.to_string().contains("unknown key"), "{note}");
        }
    }
    for key in REMOVED_DISPLAY_KEYS {
        assert!(CatalogueRow::find(key).is_none(), "{key}");
    }
}

#[test]
fn a_sub_key_of_an_off_feature_stays_classic() {
    for (key, value) in [
        ("display.crawl", "false"),
        ("display.typed_colour", "false"),
        ("display.ribbon_hp_colours", "false"),
        ("display.dither", "false"),
        ("display.centre_party", "true"),
        ("display.grow_left_light", "faithful"),
        ("display.pillar_color", "#AA5500"),
        ("display.bezel_image", "frame.png"),
        ("display.raster", "tandy"),
    ] {
        let resolved = shipped(None, &[set(key, value)]);
        assert!(
            resolved.notes.is_empty(),
            "{key}: {:?}",
            note_lines(&resolved)
        );
        assert!(resolved.is_classic(), "{key} alone must add nothing");
        assert_eq!(resolved.toggles, FeatureToggles::CLASSIC, "{key}");
        assert_eq!(
            resolved.preset,
            crate::enhancements::PresetLabel::Preset(Preset::Classic),
            "{key}"
        );
    }
    // A valued feature away from its default does switch its toggle.
    for (key, value, toggle) in [
        ("display.scale_mode", "sharp", Toggle::DisplayScaleMode),
        ("display.pixel_aspect", "square", Toggle::DisplayPixelAspect),
        ("display.pillar_fill", "solid", Toggle::DisplayPillarFill),
        ("display.vsync", "false", Toggle::DisplayVsync),
    ] {
        let resolved = shipped(None, &[set(key, value)]);
        assert_eq!(resolved.toggles.iter_on().collect::<Vec<_>>(), [toggle]);
        // Set back to its default, it switches nothing.
        let default = match OPTIONS.iter().find(|spec| spec.key == key).unwrap().default {
            OptionDefault::Str(word) => word.to_string(),
            OptionDefault::Bool(value) => value.to_string(),
            _ => unreachable!(),
        };
        assert!(shipped(None, &[set(key, &default)]).is_classic(), "{key}");
    }
}

#[test]
fn hidden_display_rows_resolve_to_their_defaults() {
    // The real build: nothing in `[display]` has shipped yet.
    let stored = Stored::default()
        .with("display.grow_left", SettingValue::Bool(true))
        .with("display.scale_mode", text("sharp"));
    let resolved = resolve(input(Some(&stored), &[set("display.vsync", "false")]));
    assert!(resolved.is_classic());
    assert_eq!(resolved.display(), DisplaySettings::default());
    assert_eq!(
        note_lines(&resolved),
        [
            "display.grow_left: not available in this build",
            "display.scale_mode: not available in this build",
            "display.vsync: not available in this build",
        ]
    );
}

#[test]
fn a_driven_toggle_cannot_be_set_by_name() {
    // `display.vsync` is a valued setting; `--enable` reaches the setting,
    // never its switch directly, and `true` is its default.
    let resolved = shipped(None, &[CliSetting::Enable("display.vsync".into())]);
    assert!(resolved.is_classic());
    // A valued key that is not a switch refuses `--enable`.
    let resolved = shipped(None, &[CliSetting::Enable("display.scale_mode".into())]);
    assert!(resolved.is_classic());
    assert_eq!(
        note_lines(&resolved),
        ["display.scale_mode: not an on/off setting, ignored (use --set)"]
    );
}

#[test]
fn word_lists_round_trip() {
    for (index, word) in SCALE_MODE_WORDS.iter().enumerate() {
        let mode = ScaleMode::from_key(word).unwrap();
        assert_eq!(mode as usize, index);
        assert_eq!(mode.key(), *word);
    }
    for word in PIXEL_ASPECT_WORDS {
        assert_eq!(PixelAspect::from_key(word).unwrap().key(), *word);
    }
    for word in GROW_LEFT_LIGHT_WORDS {
        assert_eq!(PeekLightPolicy::from_key(word).unwrap().key(), *word);
    }
    for (fill, word) in [
        (PillarFill::Black, "black"),
        (PillarFill::Solid([1, 2, 3]), "solid"),
        (PillarFill::Ribbon, "ribbon"),
        (PillarFill::Bezel(None), "bezel"),
    ] {
        assert_eq!(fill.key(), word);
        assert!(PILLAR_FILL_WORDS.contains(&word));
    }
    assert_eq!(ScaleMode::from_key("Fit"), None);
    assert_eq!(parse_colour("#aa5500"), Some([0xAA, 0x55, 0x00]));
    assert_eq!(parse_colour("#aa55"), None);
    assert_eq!(parse_colour("aa5500"), None);
}
