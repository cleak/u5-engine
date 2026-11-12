//! Chapter 9's `[display]` rows and their typed view (design 9.4.1; build
//! plan M2-01, D-M2.3, D-M2.4). Every row is hidden in this build (U13), so
//! the resolver tests use the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CatalogueRow, EffectClass, LIGHT_P1, LIGHT_P2, LIGHT_P3, LIGHT_P5, OPTIONS,
    OptionDefault, OptionKind, Phase, Preset, SHIPPED_PHASES, Tier, named_stops,
};
use crate::enhancements::resolve::{PresetLabel, ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{
    CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue, SettingsSource,
};

const ALL_LIGHT: &[Phase] = &[LIGHT_P1, LIGHT_P2, LIGHT_P3, LIGHT_P5];

/// What design 9.4.1 says one chapter 9 key is.
struct Expected {
    key: &'static str,
    kind: OptionKind,
    default: OptionDefault,
    ships: Phase,
    alias: Option<&'static str>,
    /// The toggle the value switches, for a valued toggle.
    toggle: Option<Toggle>,
    /// The toggle it is a sub-option of.
    parent: Option<Toggle>,
    switch: Option<(i64, i64)>,
}

/// Design 9.4.1, the valued rows. `sprite_composite` is a plain toggle and
/// is checked on its own.
const DESIGN_OPTIONS: &[Expected] = &[
    Expected {
        key: "display.brightness",
        kind: OptionKind::Int { min: 25, max: 100 },
        default: OptionDefault::Int(100),
        ships: LIGHT_P1,
        alias: Some("--brightness"),
        toggle: Some(Toggle::DisplayBrightness),
        parent: None,
        switch: Some((70, 100)),
    },
    Expected {
        key: "display.brightness_scope",
        kind: OptionKind::Enum(&["map", "frame"]),
        default: OptionDefault::Str("map"),
        ships: LIGHT_P1,
        alias: None,
        toggle: None,
        parent: Some(Toggle::DisplayBrightness),
        switch: None,
    },
    Expected {
        key: "display.map_brightness",
        kind: OptionKind::Int { min: 25, max: 100 },
        default: OptionDefault::Int(100),
        ships: LIGHT_P2,
        alias: None,
        toggle: Some(Toggle::DisplayMapBrightness),
        parent: None,
        switch: None,
    },
    Expected {
        key: "display.vignette",
        kind: OptionKind::Int { min: 0, max: 50 },
        default: OptionDefault::Int(0),
        ships: LIGHT_P3,
        alias: Some("--vignette"),
        toggle: Some(Toggle::DisplayVignette),
        parent: None,
        switch: None,
    },
    Expected {
        key: "display.composite_halo",
        kind: OptionKind::Bool,
        default: OptionDefault::Bool(true),
        ships: LIGHT_P5,
        alias: None,
        toggle: None,
        parent: Some(Toggle::DisplaySpriteComposite),
        switch: None,
    },
];

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

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn enable(key: &str) -> CliSetting {
    CliSetting::Enable(key.to_string())
}

fn disable(key: &str) -> CliSetting {
    CliSetting::Disable(key.to_string())
}

fn input<'a>(stored: Option<&'a dyn SettingsSource>, cli: &'a [CliSetting]) -> ResolveInput<'a> {
    ResolveInput {
        stored,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    }
}

/// Resolves as if every chapter 9 phase had shipped.
fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), ALL_LIGHT)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

fn option(key: &str) -> &'static crate::enhancements::catalogue::OptionSpec {
    OPTIONS.iter().find(|spec| spec.key == key).unwrap()
}

// --- The rows. ---------------------------------------------------------

#[test]
fn every_chapter_9_row_matches_the_design() {
    for expected in DESIGN_OPTIONS {
        let Some(CatalogueRow::Option(spec)) = CatalogueRow::find(expected.key) else {
            panic!("{} is a valued setting", expected.key);
        };
        assert_eq!(spec.kind, expected.kind, "{}", expected.key);
        assert_eq!(spec.default, expected.default, "{}", expected.key);
        assert_eq!(
            spec.class,
            Some(EffectClass::Presentation),
            "{}",
            expected.key
        );
        assert_eq!(spec.apply, ApplyAt::Frame, "{}", expected.key);
        assert_eq!(spec.tier, Tier::Feature, "{}", expected.key);
        assert!(spec.deviation_values.is_empty(), "{}", expected.key);
        assert!(spec.tags.is_empty(), "{}", expected.key);
        assert!(spec.needs.is_empty(), "{}", expected.key);
        assert_eq!(spec.ships, expected.ships, "{}", expected.key);
        assert_eq!(
            spec.cli_aliases,
            expected.alias.as_slice(),
            "{}",
            expected.key
        );
        assert_eq!(spec.toggle, expected.toggle, "{}", expected.key);
        assert_eq!(spec.parent, expected.parent, "{}", expected.key);
        assert_eq!(spec.switch, expected.switch, "{}", expected.key);
        assert!(!spec.label.trim().is_empty(), "{}", expected.key);
        if let Some(toggle) = expected.toggle {
            // A valued toggle's switch is driven, shares the key and is
            // never a row of its own.
            assert!(toggle.spec().driven, "{}", expected.key);
            assert_eq!(toggle.key(), expected.key);
            assert_eq!(toggle.spec().ships, expected.ships);
        }
    }
    let Some(CatalogueRow::Toggle(composite)) = CatalogueRow::find("display.sprite_composite")
    else {
        panic!("display.sprite_composite is a toggle");
    };
    assert_eq!(composite.toggle, Toggle::DisplaySpriteComposite);
    assert!(!composite.driven);
    assert_eq!(composite.class, EffectClass::Presentation);
    assert_eq!(composite.apply, ApplyAt::Frame);
    assert_eq!(composite.tier, Tier::Feature);
    assert_eq!(composite.ships, LIGHT_P5);
    assert_eq!(composite.cli_aliases, ["--sprite-composite"]);
    assert!(composite.needs.is_empty() && composite.tags.is_empty());

    // Six rows in all, as 9.4.1 lists them.
    let rows: Vec<&str> = CatalogueRow::all()
        .filter(|row| row.ships().chapter == 9)
        .map(CatalogueRow::key)
        .collect();
    assert_eq!(
        rows,
        [
            "display.sprite_composite",
            "display.brightness",
            "display.brightness_scope",
            "display.map_brightness",
            "display.vignette",
            "display.composite_halo",
        ]
    );
}

#[test]
fn p1_p2_and_p5_of_chapter_9_ship_in_this_build() {
    // M2-01 added the rows; M2-03 ships P1 (`brightness` and
    // `brightness_scope`), M2-06 ships P5 (`sprite_composite` and
    // `composite_halo`) and M3-11 ships P2 (`map_brightness`) with the gem
    // map's brightness link. P3 is optional (L3).
    for phase in [LIGHT_P1, LIGHT_P2, LIGHT_P5] {
        assert!(SHIPPED_PHASES.contains(&phase), "{phase:?}");
    }
    assert!(!SHIPPED_PHASES.contains(&LIGHT_P3));
    for row in CatalogueRow::all().filter(|row| row.ships().chapter == 9) {
        let shipped = matches!(
            row.key(),
            "display.brightness"
                | "display.brightness_scope"
                | "display.map_brightness"
                | "display.sprite_composite"
                | "display.composite_halo"
        );
        assert_eq!(row.is_shipped(), shipped, "{}", row.key());
    }
    // The shipped rows resolve with no note; the switches leave Classic.
    for cli in [
        enable("display.brightness"),
        set("display.brightness", "70"),
        enable("display.sprite_composite"),
        set("display.map_brightness", "70"),
    ] {
        let resolved = resolve(input(None, std::slice::from_ref(&cli)));
        assert!(!resolved.is_classic(), "{cli:?}");
        assert_eq!(note_lines(&resolved), Vec::<String>::new(), "{cli:?}");
    }
    let resolved = resolve(input(None, &[set("display.map_brightness", "70")]));
    assert_eq!(resolved.display_map_brightness(), 70);
    // The halo alone is a sub-option of an off feature: no note, still
    // Classic.
    let resolved = resolve(input(None, &[set("display.composite_halo", "false")]));
    assert!(resolved.is_classic());
    assert_eq!(note_lines(&resolved), Vec::<String>::new());
    // The hidden one is refused with a note and leaves the game Classic.
    let resolved = resolve(input(None, &[set("display.vignette", "20")]));
    assert!(resolved.is_classic());
    assert!(
        matches!(resolved.notes[..], [SettingNote::NotAvailable { .. }]),
        "{:?}",
        note_lines(&resolved)
    );
}

#[test]
fn only_brightness_has_a_switch_value() {
    for spec in OPTIONS {
        let Some((enable, disable)) = spec.switch else {
            continue;
        };
        assert_eq!(spec.key, "display.brightness");
        let OptionKind::Int { min, max } = spec.kind else {
            panic!("{} has a switch but is not an Int row", spec.key);
        };
        assert!((min..=max).contains(&enable) && (min..=max).contains(&disable));
        assert_eq!(
            spec.default,
            OptionDefault::Int(disable),
            "off is the default"
        );
        assert_ne!(enable, disable);
        assert!(spec.toggle.is_some(), "a switch belongs to a valued toggle");
    }
    assert_eq!(
        (i64::from(BRIGHTNESS_SUGGESTED), i64::from(BRIGHTNESS_OFF)),
        option(BRIGHTNESS_KEY).switch.unwrap()
    );
}

// --- The switch value (D-M2.3). -----------------------------------------

#[test]
fn enable_sets_70_and_disable_sets_100() {
    let resolved = shipped(None, &[enable("display.brightness")]);
    assert_eq!(note_lines(&resolved), Vec::<String>::new());
    assert_eq!(resolved.values.int(BRIGHTNESS_KEY), Some(70));
    assert_eq!(resolved.display_light().brightness, 70);
    assert!(resolved.display_light().brightness_on());
    assert_eq!(
        resolved.toggles.iter_on().collect::<Vec<_>>(),
        [Toggle::DisplayBrightness]
    );
    assert!(!resolved.is_classic());

    // `--disable` over a stored 55 is 100: off, and Classic.
    let stored = Stored::default().with(BRIGHTNESS_KEY, SettingValue::Int(55));
    let resolved = shipped(Some(&stored), &[]);
    assert_eq!(resolved.display_light().brightness, 55);
    let resolved = shipped(Some(&stored), &[disable("display.brightness")]);
    assert_eq!(note_lines(&resolved), Vec::<String>::new());
    assert_eq!(resolved.values.int(BRIGHTNESS_KEY), Some(100));
    assert!(!resolved.display_light().brightness_on());
    assert!(resolved.is_classic());

    // The last flag wins, as for any other key.
    let resolved = shipped(
        None,
        &[
            set("display.brightness", "40"),
            enable("display.brightness"),
        ],
    );
    assert_eq!(resolved.display_light().brightness, 70);
    let resolved = shipped(
        None,
        &[
            enable("display.brightness"),
            set("display.brightness", "40"),
        ],
    );
    assert_eq!(resolved.display_light().brightness, 40);
}

#[test]
fn other_valued_rows_still_refuse_enable() {
    // `map_brightness` and `vignette` have no switch value (D-M2.3 names
    // only `brightness`): `--enable` is a note, as for `scale_mode`.
    for key in [MAP_BRIGHTNESS_KEY, VIGNETTE_KEY, BRIGHTNESS_SCOPE_KEY] {
        let resolved = shipped(None, &[enable(key)]);
        assert!(resolved.is_classic(), "{key}");
        assert_eq!(
            note_lines(&resolved),
            [format!("{key}: not an on/off setting, ignored (use --set)")]
        );
    }
    // A Bool sub-option takes `--enable` and `--disable` as before.
    let resolved = shipped(None, &[disable(COMPOSITE_HALO_KEY)]);
    assert!(resolved.notes.is_empty());
    assert!(!resolved.display_light().halo);
}

#[test]
fn a_brightness_out_of_range_falls_back_with_a_note() {
    for value in ["101", "24", "0", "seventy"] {
        let resolved = shipped(None, &[set("display.brightness", value)]);
        assert_eq!(resolved.values.int(BRIGHTNESS_KEY), Some(100), "{value}");
        assert!(resolved.is_classic(), "{value}");
        assert_eq!(resolved.notes.len(), 1, "{value}");
        let line = resolved.notes[0].to_string();
        assert!(
            line.starts_with(&format!("display.brightness = {value}: "))
                && line.ends_with("using 100"),
            "{line}"
        );
    }
    let resolved = shipped(None, &[set("display.brightness", "101")]);
    assert_eq!(
        note_lines(&resolved),
        ["display.brightness = 101: not in 25..100, using 100"]
    );
    // The ends of the range are accepted.
    for value in [25, 100] {
        let resolved = shipped(None, &[set("display.brightness", &value.to_string())]);
        assert!(resolved.notes.is_empty(), "{value}");
        assert_eq!(i64::from(resolved.display_light().brightness), value);
    }
}

// --- Independence. -------------------------------------------------------

#[test]
fn sub_options_alone_stay_classic() {
    for (key, value) in [
        (BRIGHTNESS_SCOPE_KEY, "frame"),
        (COMPOSITE_HALO_KEY, "false"),
    ] {
        let resolved = shipped(None, &[set(key, value)]);
        assert!(
            resolved.notes.is_empty(),
            "{key}: {:?}",
            note_lines(&resolved)
        );
        assert!(resolved.is_classic(), "{key} alone must add nothing");
        assert_eq!(
            resolved.preset,
            PresetLabel::Preset(Preset::Classic),
            "{key}"
        );
        // The stored value is kept, inert.
        let light = resolved.display_light();
        assert!(light.is_off(), "{key}");
    }
    let light = shipped(None, &[set(BRIGHTNESS_SCOPE_KEY, "frame")]).display_light();
    assert_eq!(light.scope, BrightnessScope::Frame);
    let light = shipped(None, &[set(COMPOSITE_HALO_KEY, "false")]).display_light();
    assert!(!light.halo);
    assert_eq!(LIGHT_SUB_KEYS, [BRIGHTNESS_SCOPE_KEY, COMPOSITE_HALO_KEY]);
}

#[test]
fn each_switch_alone_turns_on_only_its_own_toggle() {
    for (cli, toggle) in [
        (enable(BRIGHTNESS_KEY), Toggle::DisplayBrightness),
        (set(BRIGHTNESS_KEY, "85"), Toggle::DisplayBrightness),
        (set(MAP_BRIGHTNESS_KEY, "70"), Toggle::DisplayMapBrightness),
        (set(VIGNETTE_KEY, "30"), Toggle::DisplayVignette),
        (enable(SPRITE_COMPOSITE_KEY), Toggle::DisplaySpriteComposite),
    ] {
        let resolved = shipped(None, std::slice::from_ref(&cli));
        assert!(resolved.notes.is_empty(), "{cli:?}");
        assert_eq!(
            resolved.toggles.iter_on().collect::<Vec<_>>(),
            [toggle],
            "{cli:?}"
        );
    }
    // Set back to their defaults, the valued ones switch nothing.
    for (key, value) in [
        (BRIGHTNESS_KEY, "100"),
        (MAP_BRIGHTNESS_KEY, "100"),
        (VIGNETTE_KEY, "0"),
    ] {
        assert!(shipped(None, &[set(key, value)]).is_classic(), "{key}");
    }
}

// --- The typed view. -----------------------------------------------------

#[test]
fn the_defaults_are_off() {
    let light = ResolvedEnhancements::classic().display_light();
    assert_eq!(light, DisplayLight::default());
    assert_eq!(
        light,
        DisplayLight {
            brightness: 100,
            scope: BrightnessScope::Map,
            composite: false,
            halo: true,
        }
    );
    assert!(light.is_off() && !light.brightness_on());
    assert_eq!(shipped(None, &[]).display_light(), DisplayLight::default());
    assert_eq!(
        ResolvedEnhancements::locked(LockReason::QaMode).display_light(),
        DisplayLight::default()
    );
    assert_eq!(DisplaySettings::default().light, DisplayLight::default());
}

#[test]
fn map_brightness_is_read_alone_and_never_from_brightness() {
    // 9.3.4 / D9.4b: the maps take `display.map_brightness` only.
    assert_eq!(
        ResolvedEnhancements::classic().display_map_brightness(),
        100
    );
    assert_eq!(shipped(None, &[]).display_map_brightness(), 100);
    for value in ["25", "55", "70", "85"] {
        let resolved = shipped(None, &[set(BRIGHTNESS_KEY, value)]);
        assert_eq!(resolved.display_map_brightness(), 100, "{value}");
    }
    let resolved = shipped(None, &[set(MAP_BRIGHTNESS_KEY, "70")]);
    assert_eq!(resolved.display_map_brightness(), 70);
    // The main view's brightness is unaffected by the map key.
    assert_eq!(resolved.display_light(), DisplayLight::default());
    // An out-of-range file value never reaches the maps as below 25.
    let stored = Stored::default().with(MAP_BRIGHTNESS_KEY, SettingValue::Int(5));
    let value = shipped(Some(&stored), &[]).display_map_brightness();
    assert!(
        (BRIGHTNESS_MIN..=BRIGHTNESS_OFF).contains(&value),
        "{value}"
    );
    assert_eq!(
        shipped(None, &[set(MAP_BRIGHTNESS_KEY, "25")]).display_map_brightness(),
        BRIGHTNESS_MIN
    );
}

#[test]
fn the_view_reads_file_and_command_line_values() {
    let stored = Stored::default()
        .with(BRIGHTNESS_KEY, SettingValue::Int(55))
        .with(BRIGHTNESS_SCOPE_KEY, SettingValue::Str("frame".into()))
        .with(SPRITE_COMPOSITE_KEY, SettingValue::Bool(true))
        .with(COMPOSITE_HALO_KEY, SettingValue::Bool(false));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let light = DisplayLight {
        brightness: 55,
        scope: BrightnessScope::Frame,
        composite: true,
        halo: false,
    };
    assert_eq!(resolved.display_light(), light);
    // `display()` carries the same view for the shell's `Display`.
    assert_eq!(resolved.display().light, light);
    // The command line wins over the file.
    let resolved = shipped(
        Some(&stored),
        &[
            CliSetting::Disable(SPRITE_COMPOSITE_KEY.into()),
            set(BRIGHTNESS_KEY, "85"),
        ],
    );
    let light = resolved.display_light();
    assert!(!light.composite);
    assert_eq!(light.brightness, 85);
    assert!(!light.halo, "kept while its parent is off");
}

#[test]
fn presets_set_compositing_and_never_set_brightness() {
    // Recommended and Everything name `sprite_composite` (U16), and P5 has
    // shipped (M2-06), so both turn it on; the halo keeps its default.
    for preset in [Preset::Recommended, Preset::Everything] {
        assert!(
            preset
                .entries()
                .iter()
                .any(|entry| entry.key == SPRITE_COMPOSITE_KEY),
            "{preset:?}"
        );
        assert!(preset.toggles().on(Toggle::DisplaySpriteComposite));
        let resolved = resolve(input(None, &[CliSetting::Preset(preset)]));
        let light = resolved.display_light();
        assert!(light.composite, "{preset:?}");
        assert!(light.halo, "{preset:?}");
    }
    // Classic does not.
    let resolved = resolve(input(None, &[CliSetting::Preset(Preset::Classic)]));
    assert!(!resolved.display_light().composite);
    // No preset names brightness, its scope, map brightness, the vignette
    // or the halo (D9.4, D9.5, D9.6), even with every phase shipped.
    for preset in [Preset::Classic, Preset::Recommended, Preset::Everything] {
        for key in [
            BRIGHTNESS_KEY,
            BRIGHTNESS_SCOPE_KEY,
            MAP_BRIGHTNESS_KEY,
            VIGNETTE_KEY,
            COMPOSITE_HALO_KEY,
        ] {
            assert!(
                preset.entries().iter().all(|entry| entry.key != key),
                "{preset:?} names {key}"
            );
        }
        let resolved = shipped(None, &[CliSetting::Preset(preset)]);
        let light = resolved.display_light();
        assert!(!light.brightness_on(), "{preset:?}");
        assert!(!resolved.toggles.on(Toggle::DisplayMapBrightness));
        assert!(!resolved.toggles.on(Toggle::DisplayVignette));
    }
}

// --- Named stops (D-M2.4). ----------------------------------------------

#[test]
fn named_stops_fit_their_rows() {
    assert_eq!(
        named_stops(BRIGHTNESS_KEY),
        Some(&[100, 85, 70, 55, 40, 25][..])
    );
    for spec in OPTIONS {
        let Some(stops) = named_stops(spec.key) else {
            continue;
        };
        let OptionKind::Int { min, max } = spec.kind else {
            panic!("{} has stops but is not an Int row", spec.key);
        };
        assert!(max - min + 1 >= 10, "{} is short enough to list", spec.key);
        let OptionDefault::Int(default) = spec.default else {
            unreachable!();
        };
        assert_eq!(
            stops.first(),
            Some(&default),
            "{} lists off first",
            spec.key
        );
        for stop in stops {
            assert!((min..=max).contains(stop), "{} stop {stop}", spec.key);
        }
        let mut sorted = stops.to_vec();
        sorted.dedup();
        assert_eq!(sorted.len(), stops.len(), "{} repeats a stop", spec.key);
        if let Some((enable, _)) = spec.switch {
            assert!(stops.contains(&enable), "{} lists its suggestion", spec.key);
        }
    }
    assert_eq!(named_stops("display.panel_guard"), None);
    assert_eq!(named_stops("display.crawl"), None);
}

#[test]
fn the_scope_words_round_trip() {
    for (index, word) in BRIGHTNESS_SCOPE_WORDS.iter().enumerate() {
        let scope = BrightnessScope::from_key(word).unwrap();
        assert_eq!(scope as usize, index);
        assert_eq!(scope.key(), *word);
    }
    assert_eq!(BrightnessScope::from_key("Map"), None);
}
