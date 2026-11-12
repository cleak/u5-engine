//! Chapter 10's `[gem_map]` rows and their typed view (design 10.3.2,
//! 10.4.1; build plan M3-01). Every row is hidden in this build (U13), so
//! the resolver tests use the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CatalogueRow, EffectClass, GEM_G2, GEM_G3, GEM_G4, GEM_G5, OPTIONS, OptionDefault,
    OptionKind, Phase, Preset, PresetValue, SHIPPED_PHASES, TOGGLES, Tier, named_stops,
};
use crate::enhancements::resolve::{PresetLabel, ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::sidecar::{SIDECAR_KEYS, sidecar_feature_on};
use crate::enhancements::source::{
    CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue, SettingsSource,
};
use crate::enhancements::{FeatureToggles, PRESETS};

const ALL_GEM: &[Phase] = &[GEM_G2, GEM_G3, GEM_G4, GEM_G5];

/// What design 10.4.1 says one sub-option is: key, type, default and the
/// phase that ships it.
const DESIGN_SUB_OPTIONS: &[(&str, OptionKind, OptionDefault, Phase)] = &[
    (
        LABELS_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(true),
        GEM_G5,
    ),
    (
        BRIGHTNESS_LINK_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(false),
        GEM_G2,
    ),
    (
        MAP_BRIGHTNESS_KEY,
        OptionKind::Int { min: 25, max: 100 },
        OptionDefault::Int(70),
        GEM_G2,
    ),
    (
        SWEEP_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(true),
        GEM_G2,
    ),
    (
        TOWN_SURROUNDINGS_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(true),
        GEM_G3,
    ),
    (
        DUNGEON_STACK_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(true),
        GEM_G4,
    ),
    (
        WRAP_ROWS_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(true),
        GEM_G4,
    ),
    (
        LEVEL_PAGING_KEY,
        OptionKind::Bool,
        OptionDefault::Bool(true),
        GEM_G4,
    ),
    (
        GLYPHS_KEY,
        OptionKind::Enum(&["perceived", "gem-faithful"]),
        OptionDefault::Str("perceived"),
        GEM_G4,
    ),
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

/// Resolves as if every chapter 10 phase had shipped.
fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), ALL_GEM)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

fn option(key: &str) -> &'static crate::enhancements::catalogue::OptionSpec {
    OPTIONS.iter().find(|spec| spec.key == key).unwrap()
}

// --- The rows. ---------------------------------------------------------

#[test]
fn the_main_toggle_matches_the_design() {
    let Some(CatalogueRow::Toggle(spec)) = CatalogueRow::find(ENHANCED_KEY) else {
        panic!("{ENHANCED_KEY} is a toggle");
    };
    assert_eq!(spec.toggle, Toggle::GemMapEnhanced);
    assert_eq!(Toggle::GemMapEnhanced.key(), ENHANCED_KEY);
    assert!(!spec.driven);
    assert_eq!(spec.class, EffectClass::Presentation);
    // P: turning it on attaches the record at the next idle prompt.
    assert_eq!(spec.apply, ApplyAt::IdlePrompt);
    assert_eq!(spec.tier, Tier::Feature);
    assert_eq!(spec.ships, GEM_G2);
    assert_eq!(spec.cli_aliases, ["--gem-map"]);
    assert!(spec.tags.is_empty() && spec.needs.is_empty() && spec.soft_deps.is_empty());
    assert_eq!(spec.label, "Enhanced gem map");
    // It is the only `[gem_map]` toggle; it is the last catalogue bit, so
    // no earlier toggle's bit moved.
    let gem_toggles: Vec<&str> = TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("gem_map."))
        .map(|spec| spec.key)
        .collect();
    assert_eq!(gem_toggles, [ENHANCED_KEY]);
    assert_eq!(TOGGLES.last().unwrap().toggle, Toggle::GemMapEnhanced);
}

#[test]
fn every_sub_option_matches_the_design() {
    for (key, kind, default, ships) in DESIGN_SUB_OPTIONS {
        let Some(CatalogueRow::Option(spec)) = CatalogueRow::find(key) else {
            panic!("{key} is a valued setting");
        };
        assert_eq!(spec.kind, *kind, "{key}");
        assert_eq!(spec.default, *default, "{key}");
        assert_eq!(spec.ships, *ships, "{key}");
        assert_eq!(spec.class, Some(EffectClass::Presentation), "{key}");
        assert_eq!(spec.apply, ApplyAt::Frame, "{key}");
        assert_eq!(spec.tier, Tier::Feature, "{key}");
        // Needs: Enhanced gem map. Inert while it is off; never a switch.
        assert_eq!(spec.parent, Some(Toggle::GemMapEnhanced), "{key}");
        assert_eq!(spec.toggle, None, "{key}");
        assert_eq!(spec.switch, None, "{key}");
        assert!(spec.cli_aliases.is_empty(), "{key} has no alias");
        assert!(spec.needs.is_empty() && spec.tags.is_empty(), "{key}");
        assert!(!spec.label.trim().is_empty(), "{key}");
        let deviation: &[&str] = if *key == GLYPHS_KEY {
            &["gem-faithful"]
        } else {
            &[]
        };
        assert_eq!(spec.deviation_values, deviation, "{key}");
    }
    assert_eq!(
        option(GLYPHS_KEY).value_tier("gem-faithful"),
        Tier::Deviation
    );
    assert_eq!(option(GLYPHS_KEY).value_tier("perceived"), Tier::Feature);
    assert_eq!(
        CatalogueRow::find(GLYPHS_KEY)
            .unwrap()
            .tier_for(Some("gem-faithful")),
        Tier::Deviation
    );

    // Ten rows in all, in 10.4.1's order, and the sub-key list is the
    // nine sub-options.
    let rows: Vec<&str> = CatalogueRow::all()
        .filter(|row| row.key().starts_with("gem_map."))
        .map(CatalogueRow::key)
        .collect();
    let mut expected = vec![ENHANCED_KEY];
    expected.extend(DESIGN_SUB_OPTIONS.iter().map(|(key, ..)| *key));
    assert_eq!(rows, expected);
    assert_eq!(GEM_MAP_SUB_KEYS, &expected[1..]);
    for row in CatalogueRow::all().filter(|row| row.key().starts_with("gem_map.")) {
        assert_eq!(row.ships().chapter, 10, "{}", row.key());
    }
}

#[test]
fn no_gem_map_row_shows_every_name() {
    // 10.4.1: there is deliberately no "show all names" option, and no row
    // of the section reveals anything except the deviation-tier glyphs.
    for row in CatalogueRow::all().filter(|row| row.key().starts_with("gem_map.")) {
        let text = format!("{} {}", row.key(), row.label()).to_lowercase();
        for word in ["all names", "show_all", "reveal", "all_names"] {
            assert!(!text.contains(word), "{} offers {word}", row.key());
        }
        if row.key() != GLYPHS_KEY {
            assert_eq!(row.tier_for(None), Tier::Feature, "{}", row.key());
        }
    }
    assert!(option(GLYPHS_KEY).label.contains("spoiler"));
}

/// The rows G2 (M3-11) and G3 (M3-12) ship: the main toggle, its outdoor
/// options and the town surroundings.
const SHIPPED_KEYS: &[&str] = &[
    ENHANCED_KEY,
    BRIGHTNESS_LINK_KEY,
    MAP_BRIGHTNESS_KEY,
    SWEEP_KEY,
    TOWN_SURROUNDINGS_KEY,
];

#[test]
fn only_g2_and_g3_of_chapter_10_ship() {
    for phase in &ALL_GEM[..2] {
        assert!(SHIPPED_PHASES.contains(phase), "{phase:?}");
        assert!(phase.is_shipped());
    }
    for phase in &ALL_GEM[2..] {
        assert!(!SHIPPED_PHASES.contains(phase), "{phase:?}");
        assert!(!phase.is_shipped());
    }
    for phase in ALL_GEM {
        assert_eq!(phase.chapter, 10);
    }
    assert_eq!(
        ALL_GEM.iter().map(|phase| phase.name).collect::<Vec<_>>(),
        ["G2", "G3", "G4", "G5"]
    );
    for row in CatalogueRow::all().filter(|row| row.key().starts_with("gem_map.")) {
        assert_eq!(
            row.is_shipped(),
            SHIPPED_KEYS.contains(&row.key()),
            "{} (U13)",
            row.key()
        );
    }
    // The G2 rows resolve with no note; the main toggle leaves Classic.
    let resolved = resolve(input(
        None,
        &[
            enable(ENHANCED_KEY),
            set(MAP_BRIGHTNESS_KEY, "50"),
            enable(BRIGHTNESS_LINK_KEY),
            disable(SWEEP_KEY),
        ],
    ));
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let view = resolved.gem_map();
    assert!(view.enhanced && view.brightness_link && !view.opts.sweep);
    assert_eq!(view.opts.map_brightness, 50);
    // The G3 row too.
    let resolved = resolve(input(
        None,
        &[enable(ENHANCED_KEY), disable(TOWN_SURROUNDINGS_KEY)],
    ));
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let view = resolved.gem_map();
    assert!(view.enhanced && !view.opts.town_surroundings);
    // The later rows are still refused with a note and change nothing,
    // whether set by name, flag or file.
    for cli in [
        set(GLYPHS_KEY, "gem-faithful"),
        disable(LABELS_KEY),
        disable(DUNGEON_STACK_KEY),
    ] {
        let resolved = resolve(input(None, std::slice::from_ref(&cli)));
        assert!(resolved.is_classic(), "{cli:?}");
        assert!(
            matches!(resolved.notes[..], [SettingNote::NotAvailable { .. }]),
            "{cli:?}: {:?}",
            note_lines(&resolved)
        );
        assert_eq!(resolved.gem_map(), GemMapToggles::default(), "{cli:?}");
    }
    let stored = Stored::default()
        .with(ENHANCED_KEY, SettingValue::Bool(true))
        .with(GLYPHS_KEY, SettingValue::Str("gem-faithful".into()));
    let resolved = resolve(input(Some(&stored), &[]));
    assert!(!resolved.is_classic(), "the file turns the map on");
    let view = resolved.gem_map();
    assert!(view.enhanced);
    assert_eq!(
        view.opts.glyphs,
        GlyphPolicy::Perceived,
        "the hidden glyph row keeps its default"
    );
}

#[test]
fn map_brightness_has_named_stops() {
    let stops = named_stops(MAP_BRIGHTNESS_KEY).unwrap();
    assert_eq!(stops, [70, 85, 100, 55, 40, 25]);
    assert_eq!(stops[0], i64::from(GEM_MAP_BRIGHTNESS_DEFAULT));
    assert_eq!(
        option(MAP_BRIGHTNESS_KEY).kind,
        OptionKind::Int {
            min: i64::from(GEM_MAP_BRIGHTNESS_MIN),
            max: i64::from(GEM_MAP_BRIGHTNESS_MAX),
        }
    );
}

#[test]
fn the_glyph_words_round_trip() {
    for (index, word) in GLYPH_WORDS.iter().enumerate() {
        let policy = GlyphPolicy::from_key(word).unwrap();
        assert_eq!(policy as usize, index);
        assert_eq!(policy.key(), *word);
    }
    assert_eq!(GlyphPolicy::default(), GlyphPolicy::Perceived);
    assert_eq!(GlyphPolicy::from_key("Perceived"), None);
    assert_eq!(GlyphPolicy::from_key("gem_faithful"), None);
}

// --- The typed view. -----------------------------------------------------

#[test]
fn the_defaults_match_the_design() {
    let expected = GemMapToggles {
        enhanced: false,
        labels: true,
        brightness_link: false,
        opts: GemMapOptions {
            map_brightness: 70,
            sweep: true,
            town_surroundings: true,
            dungeon_stack: true,
            wrap_rows: true,
            level_paging: true,
            glyphs: GlyphPolicy::Perceived,
        },
    };
    assert_eq!(GemMapToggles::default(), expected);
    assert_eq!(ResolvedEnhancements::classic().gem_map(), expected);
    assert_eq!(shipped(None, &[]).gem_map(), expected);
    for reason in [LockReason::CliFlag, LockReason::EnvVar, LockReason::QaMode] {
        assert_eq!(ResolvedEnhancements::locked(reason).gem_map(), expected);
    }
    // The view's defaults are the catalogue's.
    for (key, _, default, _) in DESIGN_SUB_OPTIONS {
        assert_eq!(option(key).default, *default);
    }
}

#[test]
fn the_view_reads_file_and_command_line_values() {
    let stored = Stored::default()
        .with(ENHANCED_KEY, SettingValue::Bool(true))
        .with(LABELS_KEY, SettingValue::Bool(false))
        .with(BRIGHTNESS_LINK_KEY, SettingValue::Bool(true))
        .with(MAP_BRIGHTNESS_KEY, SettingValue::Int(55))
        .with(SWEEP_KEY, SettingValue::Bool(false))
        .with(TOWN_SURROUNDINGS_KEY, SettingValue::Bool(false))
        .with(DUNGEON_STACK_KEY, SettingValue::Bool(false))
        .with(WRAP_ROWS_KEY, SettingValue::Bool(false))
        .with(LEVEL_PAGING_KEY, SettingValue::Bool(false))
        .with(GLYPHS_KEY, SettingValue::Str("gem-faithful".into()));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let from_file = GemMapToggles {
        enhanced: true,
        labels: false,
        brightness_link: true,
        opts: GemMapOptions {
            map_brightness: 55,
            sweep: false,
            town_surroundings: false,
            dungeon_stack: false,
            wrap_rows: false,
            level_paging: false,
            glyphs: GlyphPolicy::GemFaithful,
        },
    };
    assert_eq!(resolved.gem_map(), from_file);
    assert_eq!(
        resolved.toggles.iter_on().collect::<Vec<_>>(),
        [Toggle::GemMapEnhanced]
    );

    // The command line wins over the file.
    let resolved = shipped(
        Some(&stored),
        &[
            set(MAP_BRIGHTNESS_KEY, "85"),
            enable(LABELS_KEY),
            set(GLYPHS_KEY, "perceived"),
            disable(ENHANCED_KEY),
        ],
    );
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let view = resolved.gem_map();
    assert!(!view.enhanced);
    assert!(view.labels);
    assert_eq!(view.opts.map_brightness, 85);
    assert_eq!(view.opts.glyphs, GlyphPolicy::Perceived);
    assert!(
        !view.opts.sweep,
        "kept from the file while the toggle is off"
    );
    assert!(resolved.is_classic(), "sub-options of an off feature");

    // The alias spelling resolves through the key (the TUI turns
    // `--gem-map` into `--enable gem_map.enhanced`).
    let resolved = shipped(None, &[enable(ENHANCED_KEY)]);
    assert!(resolved.gem_map().enhanced);
    assert_eq!(resolved.gem_map().opts, GemMapOptions::default());
}

#[test]
fn a_bad_value_falls_back_with_a_note() {
    for value in ["101", "24", "0", "seventy"] {
        let resolved = shipped(None, &[set(MAP_BRIGHTNESS_KEY, value)]);
        assert_eq!(resolved.gem_map().opts.map_brightness, 70, "{value}");
        assert!(resolved.is_classic(), "{value}");
        assert_eq!(resolved.notes.len(), 1, "{value}");
        let line = resolved.notes[0].to_string();
        assert!(
            line.starts_with(&format!("{MAP_BRIGHTNESS_KEY} = {value}: "))
                && line.ends_with("using 70"),
            "{line}"
        );
    }
    assert_eq!(
        note_lines(&shipped(None, &[set(MAP_BRIGHTNESS_KEY, "101")])),
        ["gem_map.map_brightness = 101: not in 25..100, using 70"]
    );
    // The ends of the range are accepted.
    for value in [25u8, 100] {
        let resolved = shipped(None, &[set(MAP_BRIGHTNESS_KEY, &value.to_string())]);
        assert!(resolved.notes.is_empty(), "{value}");
        assert_eq!(resolved.gem_map().opts.map_brightness, value);
    }
    // An unknown glyph word falls back to `perceived`, never to the
    // spoiler.
    let resolved = shipped(None, &[set(GLYPHS_KEY, "all")]);
    assert_eq!(resolved.notes.len(), 1, "{:?}", note_lines(&resolved));
    assert_eq!(resolved.gem_map().opts.glyphs, GlyphPolicy::Perceived);
    // `--enable` on a valued sub-option is refused as for any valued row.
    for key in [MAP_BRIGHTNESS_KEY, GLYPHS_KEY] {
        let resolved = shipped(None, &[enable(key)]);
        assert_eq!(
            note_lines(&resolved),
            [format!("{key}: not an on/off setting, ignored (use --set)")]
        );
        assert_eq!(resolved.gem_map(), GemMapToggles::default());
    }
}

// --- Independence. -------------------------------------------------------

#[test]
fn sub_keys_alone_leave_the_game_classic() {
    for (key, value) in [
        (LABELS_KEY, "false"),
        (BRIGHTNESS_LINK_KEY, "true"),
        (MAP_BRIGHTNESS_KEY, "40"),
        (SWEEP_KEY, "false"),
        (TOWN_SURROUNDINGS_KEY, "false"),
        (DUNGEON_STACK_KEY, "false"),
        (WRAP_ROWS_KEY, "false"),
        (LEVEL_PAGING_KEY, "false"),
        (GLYPHS_KEY, "gem-faithful"),
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
        assert!(!resolved.gem_map().enhanced, "{key}");
        assert!(!sidecar_feature_on(resolved.toggles), "{key}");
    }
    // All nine at once, too.
    let all: Vec<CliSetting> = [
        (LABELS_KEY, "false"),
        (MAP_BRIGHTNESS_KEY, "25"),
        (GLYPHS_KEY, "gem-faithful"),
        (LEVEL_PAGING_KEY, "false"),
    ]
    .iter()
    .map(|(key, value)| set(key, value))
    .collect();
    assert!(shipped(None, &all).is_classic());
}

#[test]
fn the_main_toggle_alone_turns_on_only_itself() {
    let resolved = shipped(None, &[enable(ENHANCED_KEY)]);
    assert!(resolved.notes.is_empty());
    assert_eq!(
        resolved.toggles.iter_on().collect::<Vec<_>>(),
        [Toggle::GemMapEnhanced]
    );
    assert!(!resolved.is_classic());
    // The parity lock forces it off (10.4.5).
    let locked = resolve_as_shipped(
        ResolveInput {
            stored: None,
            cli: &[enable(ENHANCED_KEY), set(MAP_BRIGHTNESS_KEY, "40")],
            lock: LockState::Locked(LockReason::CliFlag),
            caps: HostCapabilities::ALL,
        },
        ALL_GEM,
    );
    assert!(locked.is_classic());
    assert_eq!(locked.gem_map(), GemMapToggles::default());
}

#[test]
fn the_brightness_link_follows_display_map_brightness_below_100() {
    let mut view = GemMapToggles::default();
    assert_eq!(view.map_brightness(100), 70);
    assert_eq!(view.map_brightness(40), 70, "link off: own value");
    view.brightness_link = true;
    assert_eq!(view.map_brightness(100), 70, "100 changes nothing");
    assert_eq!(view.map_brightness(40), 40);
    assert_eq!(view.map_brightness(85), 85);
    view.opts.map_brightness = 55;
    assert_eq!(view.map_brightness(100), 55);
}

#[test]
fn paging_needs_the_stack_and_the_toggle() {
    let mut view = GemMapToggles::default();
    assert!(!view.paging_keys(), "off: no key is bound (10.4.2)");
    view.enhanced = true;
    assert!(view.paging_keys());
    view.opts.dungeon_stack = false;
    assert!(!view.paging_keys(), "one level alone: no paging (10.4.1)");
    view.opts.dungeon_stack = true;
    view.opts.level_paging = false;
    assert!(!view.paging_keys());
}

// --- Presets. ------------------------------------------------------------

#[test]
fn presets_set_only_the_main_toggle_and_never_the_spoiler() {
    for preset in PRESETS {
        for entry in preset.entries() {
            if !entry.key.starts_with("gem_map.") {
                continue;
            }
            // D10.10: Everything names the main toggle; no preset names a
            // sub-option, and gem-faithful is in none.
            assert_eq!(entry.key, ENHANCED_KEY, "{preset:?}");
            assert_eq!(entry.value, PresetValue::On, "{preset:?}");
        }
        assert!(
            preset
                .entries()
                .iter()
                .all(|entry| !(entry.key == GLYPHS_KEY
                    && entry.value == PresetValue::Choice("gem-faithful"))),
            "{preset:?} sets {GLYPHS_KEY} = gem-faithful"
        );
        // Even with every chapter 10 phase shipped, the view keeps the
        // perceived glyphs and the default sub-options.
        let view = shipped(None, &[CliSetting::Preset(*preset)]).gem_map();
        assert_eq!(view.opts, GemMapOptions::default(), "{preset:?}");
        assert!(view.labels && !view.brightness_link, "{preset:?}");
    }
    assert!(
        Preset::Everything
            .entries()
            .iter()
            .any(|entry| entry.key == ENHANCED_KEY)
    );
    assert!(
        Preset::Recommended
            .entries()
            .iter()
            .all(|entry| !entry.key.starts_with("gem_map."))
    );
}

#[test]
fn everything_turns_the_gem_map_on_and_skips_its_hidden_rows() {
    // G2 and G3 have shipped (M3-11, M3-12): Everything applies
    // `gem_map.enhanced`; no preset applies a row of a later phase, and none
    // but Everything turns the map on.
    for preset in PRESETS {
        let on = *preset == Preset::Everything;
        assert_eq!(
            preset.toggles().on(Toggle::GemMapEnhanced),
            on,
            "{preset:?}"
        );
        for (entry, row) in preset.shipped_entries() {
            if entry.key.starts_with("gem_map.") {
                assert!(
                    SHIPPED_KEYS.contains(&entry.key),
                    "{preset:?} {}",
                    entry.key
                );
                assert!(row.is_shipped());
            }
        }
        let resolved = resolve(input(None, &[CliSetting::Preset(*preset)]));
        assert_eq!(resolved.gem_map().enhanced, on, "{preset:?}");
        if on {
            assert_eq!(resolved.gem_map().opts, GemMapOptions::default());
        }
    }
    let classic = resolve(input(None, &[CliSetting::Preset(Preset::Classic)]));
    assert!(classic.is_classic());
    assert_eq!(classic.gem_map(), GemMapToggles::default());
}

// --- The sidecar. ---------------------------------------------------------

#[test]
fn the_gem_map_toggle_turns_the_sidecar_on() {
    assert!(SIDECAR_KEYS.contains(&ENHANCED_KEY));
    assert!(!sidecar_feature_on(FeatureToggles::CLASSIC));
    let mut toggles = FeatureToggles::CLASSIC;
    toggles.set(Toggle::GemMapEnhanced, true);
    assert!(sidecar_feature_on(toggles));
    let resolved = shipped(None, &[enable(ENHANCED_KEY)]);
    assert!(sidecar_feature_on(resolved.toggles));
    // The row has shipped (G2): the plain resolver turns it on too.
    let resolved = resolve(input(None, &[enable(ENHANCED_KEY)]));
    assert!(sidecar_feature_on(resolved.toggles));
}
