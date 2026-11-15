//! Chapter 5's `[journal]` rows and their typed view (design 5.3.6, 5.4.1;
//! build plan M5-01). The rows ship in M5-06; the resolver tests that
//! predate it still resolve through the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CatalogueRow, EffectClass, JOURNAL_P3, JOURNAL_P3B, OPTIONS, OptionDefault,
    OptionKind, PRESETS, Phase, Preset, PresetValue, SHIPPED_PHASES, TOGGLES, Tier,
};
use crate::enhancements::resolve::{ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{
    CliSetting, HostCapabilities, LockReason, LockState, SettingValue, SettingsSource,
};

const ALL_JOURNAL: &[Phase] = &[JOURNAL_P3, JOURNAL_P3B];

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

/// Resolves as if both chapter 5 phases had shipped.
fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), ALL_JOURNAL)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

// --- The rows. -----------------------------------------------------------

#[test]
fn the_two_toggles_match_the_design() {
    for (key, toggle, ships, alias) in [
        (PAGE_KEY, Toggle::JournalPage, JOURNAL_P3, "--quest-journal"),
        (
            SIDE_PANEL_KEY,
            Toggle::JournalSidePanel,
            JOURNAL_P3B,
            "--journal-side-panel",
        ),
    ] {
        let Some(CatalogueRow::Toggle(spec)) = CatalogueRow::find(key) else {
            panic!("{key} is a toggle");
        };
        assert_eq!(spec.toggle, toggle);
        assert!(!spec.driven, "{key}");
        assert_eq!(spec.class, EffectClass::Presentation, "{key}");
        assert_eq!(spec.apply, ApplyAt::Frame, "{key}");
        assert_eq!(spec.tier, Tier::Feature, "{key}");
        assert_eq!(spec.ships, ships, "{key}");
        assert_eq!(spec.cli_aliases, [alias], "{key}");
        assert!(spec.tags.is_empty() && spec.needs.is_empty() && spec.soft_deps.is_empty());
    }
    let journal_toggles: Vec<&str> = TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("journal.") && !CLUE_KEYS.contains(&spec.key))
        .map(|spec| spec.key)
        .collect();
    // Chapter 8's rows (`CLUE_KEYS`) have their own tests.
    assert_eq!(journal_toggles, [PAGE_KEY, SIDE_PANEL_KEY]);
}

#[test]
fn the_three_options_match_the_design() {
    let rows: [(
        &str,
        OptionKind,
        OptionDefault,
        Phase,
        Toggle,
        &[&str],
        &[&str],
    ); 3] = [
        (
            SPOILER_TIER_KEY,
            OptionKind::Int { min: 1, max: 4 },
            OptionDefault::Int(2),
            JOURNAL_P3,
            Toggle::JournalPage,
            &["--spoilers"],
            &["3", "4"],
        ),
        (
            MANTRAS_KEY,
            OptionKind::Enum(&["reveal", "never"]),
            OptionDefault::Str("reveal"),
            JOURNAL_P3,
            Toggle::JournalPage,
            &[],
            &[],
        ),
        (
            SIDE_PANEL_SIDE_KEY,
            OptionKind::Enum(&["auto", "left", "right"]),
            OptionDefault::Str("auto"),
            JOURNAL_P3B,
            Toggle::JournalSidePanel,
            &[],
            &[],
        ),
    ];
    for (key, kind, default, ships, parent, aliases, deviation) in rows {
        let Some(CatalogueRow::Option(spec)) = CatalogueRow::find(key) else {
            panic!("{key} is a valued setting");
        };
        assert_eq!(spec.kind, kind, "{key}");
        assert_eq!(spec.default, default, "{key}");
        assert_eq!(spec.ships, ships, "{key}");
        assert_eq!(spec.class, Some(EffectClass::Presentation), "{key}");
        assert_eq!(spec.apply, ApplyAt::Frame, "{key}");
        assert_eq!(spec.tier, Tier::Feature, "{key}");
        assert_eq!(spec.parent, Some(parent), "{key}");
        assert_eq!((spec.toggle, spec.switch), (None, None), "{key}");
        assert_eq!(spec.cli_aliases, aliases, "{key}");
        assert_eq!(spec.deviation_values, deviation, "{key}");
        assert!(spec.needs.is_empty() && spec.tags.is_empty(), "{key}");
    }
    // Tiers 3 and 4 are deviation tier; 1 and 2 are not (5.4.1).
    let tier = CatalogueRow::find(SPOILER_TIER_KEY).unwrap();
    for (value, want) in [
        ("1", Tier::Feature),
        ("2", Tier::Feature),
        ("3", Tier::Deviation),
        ("4", Tier::Deviation),
    ] {
        assert_eq!(tier.tier_for(Some(value)), want, "tier {value}");
    }
    // The settings row names the spoiler tiers as spoilers (1.4.7).
    assert!(tier.label().contains("3 Spoiler:") && tier.label().contains("4 Spoiler:"));

    // Five rows, in 5.4.1's order, all of chapter 5 (chapter 8's rows,
    // `CLUE_KEYS`, are tested on their own).
    let chapter_5 =
        |row: &CatalogueRow| row.key().starts_with("journal.") && !CLUE_KEYS.contains(&row.key());
    let keys: Vec<&str> = CatalogueRow::all()
        .filter(chapter_5)
        .map(CatalogueRow::key)
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_by_key(|key| JOURNAL_KEYS.iter().position(|k| k == key));
    assert_eq!(sorted, JOURNAL_KEYS);
    assert_eq!(keys.len(), 5);
    for row in CatalogueRow::all().filter(chapter_5) {
        assert_eq!(row.ships().chapter, 5, "{}", row.key());
    }
    // No reserved name became a row (U3, U1).
    for reserved in ["journal.f1_alias", "journal.enabled", "journal.pause_world"] {
        assert_eq!(CatalogueRow::find(reserved), None, "{reserved}");
    }
    assert!(OPTIONS.iter().all(|spec| spec.key != "journal.f1_alias"));
}

#[test]
fn every_journal_row_ships_in_m5_06() {
    for phase in ALL_JOURNAL {
        assert!(SHIPPED_PHASES.contains(phase), "{phase:?}");
        assert_eq!(phase.chapter, 5);
    }
    for row in CatalogueRow::all()
        .filter(|row| row.key().starts_with("journal.") && !CLUE_KEYS.contains(&row.key()))
    {
        assert!(row.is_shipped(), "{}", row.key());
    }
    // A shipped row takes the file and the flags, with no note.
    let stored = Stored::default()
        .with(PAGE_KEY, SettingValue::Bool(true))
        .with(SPOILER_TIER_KEY, SettingValue::Int(4));
    let cli = [enable(SIDE_PANEL_KEY), set(MANTRAS_KEY, "never")];
    let resolved = resolve(input(Some(&stored), &cli));
    assert!(resolved.notes.is_empty(), "{:?}", resolved.notes);
    assert_eq!(resolved.journal(), shipped(Some(&stored), &cli).journal());
    let journal = resolved.journal();
    assert!(journal.page && journal.side_panel);
    assert_eq!(journal.tier, SpoilerTier::Full);
    assert_eq!(journal.mantras, MantraDisplay::Never);
    // Everything sets the page and the panel; Recommended neither (U2).
    let everything = resolve(input(None, &[CliSetting::Preset(Preset::Everything)])).journal();
    assert!(everything.page && everything.side_panel);
    assert_eq!(everything.tier, SpoilerTier::Goals);
    let recommended = resolve(input(None, &[CliSetting::Preset(Preset::Recommended)])).journal();
    assert_eq!(recommended, JournalSettings::default());
}

// --- The typed view. -----------------------------------------------------

#[test]
fn the_defaults_match_the_design() {
    let expected = JournalSettings {
        page: false,
        tier: SpoilerTier::Goals,
        mantras: MantraDisplay::Reveal,
        side_panel: false,
        side: JournalPanelSide::Auto,
        clues: ClueSettings::default(),
    };
    assert_eq!(JournalSettings::default(), expected);
    assert!(!expected.any_on());
    assert_eq!(ResolvedEnhancements::classic().journal(), expected);
    assert_eq!(shipped(None, &[]).journal(), expected);
    for reason in [LockReason::CliFlag, LockReason::EnvVar, LockReason::QaMode] {
        assert_eq!(ResolvedEnhancements::locked(reason).journal(), expected);
    }
}

#[test]
fn the_view_reads_file_and_command_line_values() {
    let stored = Stored::default()
        .with(PAGE_KEY, SettingValue::Bool(true))
        .with(SPOILER_TIER_KEY, SettingValue::Int(4))
        .with(MANTRAS_KEY, SettingValue::Str("never".into()))
        .with(SIDE_PANEL_KEY, SettingValue::Bool(true))
        .with(SIDE_PANEL_SIDE_KEY, SettingValue::Str("left".into()));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert_eq!(
        resolved.journal(),
        JournalSettings {
            page: true,
            tier: SpoilerTier::Full,
            mantras: MantraDisplay::Never,
            side_panel: true,
            side: JournalPanelSide::Left,
            clues: ClueSettings::default(),
        }
    );
    // The command line wins over the file, and the two toggles are
    // independent.
    let cli = [
        disable(PAGE_KEY),
        set(SPOILER_TIER_KEY, "1"),
        set(SIDE_PANEL_SIDE_KEY, "right"),
    ];
    let view = shipped(Some(&stored), &cli).journal();
    assert!(!view.page && view.side_panel);
    assert_eq!(view.tier, SpoilerTier::Progress);
    assert_eq!(view.side, JournalPanelSide::Right);
    let view = shipped(None, &[enable(PAGE_KEY)]).journal();
    assert!(view.page && !view.side_panel && view.any_on());
    // A sub-option alone is inert: the toggles stay Classic.
    let resolved = shipped(None, &[set(SPOILER_TIER_KEY, "3")]);
    assert!(resolved.toggles.is_classic());
    assert_eq!(resolved.journal().tier, SpoilerTier::Hints);
    assert!(!resolved.journal().any_on());
}

#[test]
fn a_tier_out_of_range_falls_back_to_two_with_a_note() {
    let stored = Stored::default()
        .with(PAGE_KEY, SettingValue::Bool(true))
        .with(SPOILER_TIER_KEY, SettingValue::Int(7));
    let resolved = shipped(Some(&stored), &[]);
    assert_eq!(
        note_lines(&resolved),
        ["journal.spoiler_tier = 7: not in 1..4, using 2"]
    );
    assert_eq!(resolved.journal().tier, SpoilerTier::Goals);
    assert!(resolved.journal().page);
    for bad in ["0", "5", "two"] {
        let resolved = shipped(None, &[enable(PAGE_KEY), set(SPOILER_TIER_KEY, bad)]);
        assert_eq!(resolved.journal().tier, SpoilerTier::Goals, "{bad}");
        assert_eq!(resolved.notes.len(), 1, "{bad}");
    }
    let resolved = shipped(None, &[set(MANTRAS_KEY, "always")]);
    assert_eq!(resolved.journal().mantras, MantraDisplay::Reveal);
    assert_eq!(resolved.notes.len(), 1);
}

#[test]
fn no_preset_sets_a_tier_above_two() {
    for preset in PRESETS {
        for entry in preset.entries() {
            if entry.key == SPOILER_TIER_KEY {
                assert!(
                    matches!(entry.value, PresetValue::Choice("1" | "2")),
                    "{preset:?}"
                );
            }
            assert_ne!(entry.key, MANTRAS_KEY, "{preset:?}: a preference");
            assert_ne!(entry.key, SIDE_PANEL_SIDE_KEY, "{preset:?}: a preference");
        }
    }
    // Everything turns on the page at tier 2 and the panel once they ship;
    // Recommended holds no journal row of this chapter (U2).
    let everything = shipped(None, &[CliSetting::Preset(Preset::Everything)]).journal();
    assert!(everything.page && everything.side_panel);
    assert_eq!(everything.tier, SpoilerTier::Goals);
    let recommended = shipped(None, &[CliSetting::Preset(Preset::Recommended)]).journal();
    assert_eq!(recommended, JournalSettings::default());
}

#[test]
fn the_words_and_tiers_round_trip() {
    for (index, word) in MANTRA_WORDS.iter().enumerate() {
        let display = MantraDisplay::from_key(word).unwrap();
        assert_eq!(display as usize, index);
        assert_eq!(display.key(), *word);
    }
    for (index, word) in PANEL_SIDE_WORDS.iter().enumerate() {
        let side = JournalPanelSide::from_key(word).unwrap();
        assert_eq!(side as usize, index);
        assert_eq!(side.key(), *word);
    }
    assert_eq!(MantraDisplay::from_key("Reveal"), None);
    assert_eq!(JournalPanelSide::from_key("centre"), None);
    for (index, tier) in SpoilerTier::ALL.into_iter().enumerate() {
        assert_eq!(usize::from(tier.number()), index + 1);
        assert_eq!(
            SpoilerTier::from_number(i64::from(tier.number())),
            Some(tier)
        );
        assert_eq!(tier.is_spoiler(), tier.number() >= 3);
    }
    assert_eq!(SpoilerTier::from_number(0), None);
    assert_eq!(SpoilerTier::from_number(5), None);
    assert!(SpoilerTier::Progress < SpoilerTier::Full);
}
