//! Chapter 8's `[journal]` rows and their typed view (design 8.4.1,
//! 8.4.5; build plan M6-01, T3c). The rows ship in M6-08 (all but
//! `clue_misses`, which waits for D8.14), so the tests that need every
//! row resolve through the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CLUES_P1A, CLUES_P1B, CLUES_P1C, CLUES_P1D, CLUES_P2A, CLUES_P2B, CatalogueRow,
    EffectClass, OptionDefault, OptionKind, PRESETS, Phase, Preset, SHIPPED_PHASES, Tag, Tier,
    needs_line, prerequisite,
};
use crate::enhancements::resolve::{ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{
    CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue, SettingsSource,
    ValueProblem,
};

const ALL_CLUES: &[Phase] = &[
    CLUES_P1A, CLUES_P1B, CLUES_P1C, CLUES_P1D, CLUES_P2A, CLUES_P2B,
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

fn input<'a>(stored: Option<&'a dyn SettingsSource>, cli: &'a [CliSetting]) -> ResolveInput<'a> {
    ResolveInput {
        stored,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    }
}

fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), ALL_CLUES)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

// --- The rows. -----------------------------------------------------------

#[test]
fn the_six_toggles_match_the_design() {
    for (key, toggle, apply, tier, ships, aliases, tags) in [
        (
            CLUES_KEY,
            Toggle::JournalClues,
            ApplyAt::IdlePrompt,
            Tier::Feature,
            CLUES_P1A,
            &["--clue-log"][..],
            &[][..],
        ),
        (
            CLUE_SCROLLBACK_KEY,
            Toggle::JournalClueScrollback,
            ApplyAt::Frame,
            Tier::Feature,
            CLUES_P1B,
            &[],
            &[],
        ),
        (
            CLUE_TOAST_KEY,
            Toggle::JournalClueToast,
            ApplyAt::Frame,
            Tier::Feature,
            CLUES_P1B,
            &[],
            &[],
        ),
        (
            NOTES_KEY,
            Toggle::JournalNotes,
            ApplyAt::Frame,
            Tier::Feature,
            CLUES_P1B,
            &[],
            &[],
        ),
        (
            CLUE_MISSES_KEY,
            Toggle::JournalClueMisses,
            ApplyAt::Frame,
            Tier::Deviation,
            CLUES_P1D,
            &[],
            &[Tag::SpoilerRisk],
        ),
        (
            QUEST_TRACKER_KEY,
            Toggle::JournalQuestTracker,
            ApplyAt::Frame,
            Tier::Feature,
            CLUES_P2A,
            &["--quest-tracker"],
            &[],
        ),
    ] {
        let Some(CatalogueRow::Toggle(spec)) = CatalogueRow::find(key) else {
            panic!("{key} is a toggle");
        };
        assert_eq!(spec.toggle, toggle);
        assert!(!spec.driven, "{key}");
        assert_eq!(spec.class, EffectClass::Presentation, "{key}");
        assert_eq!(spec.apply, apply, "{key}");
        assert_eq!(spec.tier, tier, "{key}");
        assert_eq!(spec.ships, ships, "{key}");
        assert_eq!(spec.cli_aliases, aliases, "{key}");
        assert_eq!(spec.tags, tags, "{key}");
        assert!(spec.needs.is_empty() && spec.soft_deps.is_empty());
    }
}

#[test]
fn the_four_options_match_the_design() {
    type Row = (
        &'static str,
        OptionKind,
        &'static str,
        Phase,
        Option<Toggle>,
        Option<Toggle>,
        &'static [&'static str],
        &'static [&'static str],
    );
    let rows: [Row; 4] = [
        (
            CLUE_TEXT_KEY,
            OptionKind::Enum(&["off", "live"]),
            "off",
            CLUES_P1B,
            None,
            Some(Toggle::JournalClues),
            &["--clue-text"],
            &[],
        ),
        (
            CLUE_HIGHLIGHT_KEY,
            OptionKind::Enum(&["off", "asked", "asked_elsewhere", "new"]),
            "off",
            CLUES_P1C,
            Some(Toggle::JournalClueHighlight),
            None,
            &["--clue-highlight"],
            &["new"],
        ),
        (
            CLUE_HIGHLIGHT_STYLE_KEY,
            OptionKind::Enum(&["dotted", "tint"]),
            "dotted",
            CLUES_P1C,
            None,
            Some(Toggle::JournalClueHighlight),
            &[],
            &[],
        ),
        (
            TRACKER_EVIDENCE_KEY,
            OptionKind::Enum(&["record", "npc"]),
            "record",
            CLUES_P2B,
            None,
            Some(Toggle::JournalQuestTracker),
            &[],
            &["npc"],
        ),
    ];
    for (key, kind, default, ships, toggle, parent, aliases, deviation) in rows {
        let Some(CatalogueRow::Option(spec)) = CatalogueRow::find(key) else {
            panic!("{key} is a valued setting");
        };
        assert_eq!(spec.kind, kind, "{key}");
        assert_eq!(spec.default, OptionDefault::Str(default), "{key}");
        assert_eq!(spec.class, Some(EffectClass::Presentation), "{key}");
        assert_eq!(spec.apply, ApplyAt::Frame, "{key}");
        assert_eq!(spec.tier, Tier::Feature, "{key}");
        assert_eq!(spec.ships, ships, "{key}");
        assert_eq!(spec.toggle, toggle, "{key}");
        assert_eq!(spec.parent, parent, "{key}");
        assert_eq!(spec.cli_aliases, aliases, "{key}");
        assert_eq!(spec.deviation_values, deviation, "{key}");
    }
    // `clue_highlight` is a feature switch whose toggle is driven.
    assert!(Toggle::JournalClueHighlight.spec().driven);
}

#[test]
fn chapter_8_has_ten_rows_and_no_map_links_yet() {
    let rows: Vec<&str> = CatalogueRow::all()
        .map(CatalogueRow::key)
        .filter(|key| CLUE_KEYS.contains(key))
        .collect();
    assert_eq!(rows.len(), 10);
    assert_eq!(CLUE_KEYS.len(), 10);
    assert_eq!(CatalogueRow::find("journal.clue_map_links"), None, "L1");
    for key in CLUE_KEYS {
        assert!(CatalogueRow::find(key).is_some(), "{key}");
    }
}

#[test]
fn chapter_8_ships_in_m6_08_but_clue_misses_waits_for_d8_14() {
    for phase in [CLUES_P1A, CLUES_P1B, CLUES_P1C, CLUES_P2A, CLUES_P2B] {
        assert!(SHIPPED_PHASES.contains(&phase), "{phase:?}");
    }
    assert!(!SHIPPED_PHASES.contains(&CLUES_P1D), "D8.14");
    for key in CLUE_KEYS {
        let row = CatalogueRow::find(key).unwrap();
        assert_eq!(row.is_shipped(), *key != CLUE_MISSES_KEY, "{key}");
    }
    // The shipped rows take the file with no note; `clue_misses` is the
    // one row still hidden, kept at its default with a note.
    let stored = Stored::default()
        .with(CLUES_KEY, SettingValue::Bool(true))
        .with(CLUE_HIGHLIGHT_KEY, SettingValue::Str("asked".into()))
        .with(QUEST_TRACKER_KEY, SettingValue::Bool(true))
        .with(CLUE_MISSES_KEY, SettingValue::Bool(true));
    let resolved = resolve(input(Some(&stored), &[]));
    let view = resolved.journal().clues;
    assert_eq!(
        view,
        ClueSettings {
            clues: true,
            highlight: ClueHighlight::Asked,
            quest_tracker: true,
            ..ClueSettings::default()
        }
    );
    assert!(!view.misses && !view.spoiler_risk());
    assert!(
        matches!(
            resolved.notes.as_slice(),
            [SettingNote::NotAvailable {
                key: "journal.clue_misses"
            }]
        ),
        "{:?}",
        note_lines(&resolved)
    );
    // Recommended and Everything set the three design keys (D8.3);
    // Classic none, and no preset sets a spoiler value.
    for preset in PRESETS {
        let view = resolve(input(None, &[CliSetting::Preset(*preset)]))
            .journal()
            .clues;
        let on = *preset != Preset::Classic;
        assert_eq!(view.clues && view.quest_tracker, on, "{preset:?}");
        assert_eq!(view.highlight == ClueHighlight::Asked, on, "{preset:?}");
        assert!(!view.spoiler_risk(), "{preset:?}");
    }
}

// --- Resolution with the phases shipped. ---------------------------------

#[test]
fn the_view_reads_file_and_command_line_values() {
    let stored = Stored::default()
        .with(CLUES_KEY, SettingValue::Bool(true))
        .with(CLUE_TEXT_KEY, SettingValue::Str("live".into()))
        .with(
            CLUE_HIGHLIGHT_KEY,
            SettingValue::Str("asked_elsewhere".into()),
        )
        .with(CLUE_HIGHLIGHT_STYLE_KEY, SettingValue::Str("tint".into()))
        .with(CLUE_SCROLLBACK_KEY, SettingValue::Bool(true))
        .with(CLUE_TOAST_KEY, SettingValue::Bool(true))
        .with(NOTES_KEY, SettingValue::Bool(true))
        .with(CLUE_MISSES_KEY, SettingValue::Bool(true))
        .with(QUEST_TRACKER_KEY, SettingValue::Bool(true))
        .with(TRACKER_EVIDENCE_KEY, SettingValue::Str("npc".into()));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let view = resolved.journal().clues;
    assert_eq!(
        view,
        ClueSettings {
            clues: true,
            text: ClueText::Live,
            highlight: ClueHighlight::AskedElsewhere,
            highlight_style: HighlightStyle::Tint,
            scrollback: true,
            toast: true,
            notes: true,
            misses: true,
            quest_tracker: true,
            tracker_evidence: TrackerEvidence::Npc,
        }
    );
    assert!(view.spoiler_risk());
    assert!(resolved.toggles.on(Toggle::JournalClueHighlight));
    // Chapter 5's part is untouched.
    assert!(!resolved.journal().page && !resolved.journal().side_panel);

    // The aliases, and the command line over the file.
    let cli = [
        CliSetting::Disable(CLUES_KEY.into()),
        set(CLUE_HIGHLIGHT_KEY, "off"),
    ];
    let view = shipped(Some(&stored), &cli).journal().clues;
    assert!(!view.clues && !view.toast && !view.notes && !view.misses);
    assert_eq!(view.text, ClueText::Off, "needs clues");
    assert_eq!(view.highlight, ClueHighlight::Off);
    assert!(
        view.scrollback && view.quest_tracker,
        "independent of clues"
    );
}

#[test]
fn a_key_whose_prerequisite_is_off_resolves_off_with_a_note() {
    for key in [CLUE_TEXT_KEY, CLUE_TOAST_KEY, NOTES_KEY, CLUE_MISSES_KEY] {
        assert_eq!(prerequisite(key), Some(CLUES_KEY), "{key}");
        assert_eq!(needs_line(key).as_deref(), Some("needs: journal.clues"));
    }
    for key in [
        CLUES_KEY,
        CLUE_HIGHLIGHT_KEY,
        CLUE_HIGHLIGHT_STYLE_KEY,
        CLUE_SCROLLBACK_KEY,
        QUEST_TRACKER_KEY,
        TRACKER_EVIDENCE_KEY,
    ] {
        assert_eq!(prerequisite(key), None, "{key} needs nothing");
    }

    let cli = [
        enable(CLUE_TOAST_KEY),
        enable(NOTES_KEY),
        enable(CLUE_MISSES_KEY),
        set(CLUE_TEXT_KEY, "live"),
    ];
    let resolved = shipped(None, &cli);
    let view = resolved.journal().clues;
    assert_eq!(
        view,
        ClueSettings::default(),
        "every dependent resolves off"
    );
    assert!(!view.clues, "no key turns its prerequisite on");
    assert!(resolved.is_classic());
    let mut lines = note_lines(&resolved);
    lines.sort();
    assert_eq!(
        lines,
        [
            "journal.clue_misses: needs: journal.clues; off until it is on",
            "journal.clue_text: needs: journal.clues; off until it is on",
            "journal.clue_toast: needs: journal.clues; off until it is on",
            "journal.notes: needs: journal.clues; off until it is on",
        ]
    );

    // With the prerequisite on, the same settings apply quietly.
    let with = [&[enable(CLUES_KEY)][..], &cli].concat();
    let resolved = shipped(None, &with);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let view = resolved.journal().clues;
    assert!(view.clues && view.toast && view.notes && view.misses);
    assert_eq!(view.text, ClueText::Live);

    // A dependent at its default is not noted.
    let quiet = shipped(None, &[set(CLUE_TEXT_KEY, "off")]);
    assert!(quiet.notes.is_empty(), "{:?}", note_lines(&quiet));
}

#[test]
fn kept_text_waits_for_d8_1() {
    let resolved = shipped(None, &[enable(CLUES_KEY), set(CLUE_TEXT_KEY, "kept")]);
    assert_eq!(resolved.journal().clues.text, ClueText::Off);
    assert!(matches!(
        resolved.notes.as_slice(),
        [SettingNote::BadValue {
            key: "journal.clue_text",
            problem: ValueProblem::NotOneOf(_),
            ..
        }]
    ));
    assert_eq!(ClueText::from_key(CLUE_TEXT_KEPT), None);
    assert_eq!(ClueText::Kept.key(), CLUE_TEXT_KEPT);
}

#[test]
fn highlighting_alone_runs_the_observer_session_only() {
    let resolved = shipped(None, &[set(CLUE_HIGHLIGHT_KEY, "asked")]);
    let view = resolved.journal().clues;
    assert!(view.observer_on() && view.session_only());
    assert!(!view.spoiler_risk());
    let both = shipped(None, &[enable(CLUES_KEY), set(CLUE_HIGHLIGHT_KEY, "asked")])
        .journal()
        .clues;
    assert!(both.observer_on() && !both.session_only());
    let none = ClueSettings::default();
    assert!(!none.observer_on() && !none.session_only());
    let new = shipped(None, &[set(CLUE_HIGHLIGHT_KEY, "new")])
        .journal()
        .clues;
    assert!(new.spoiler_risk(), "new is labelled Spoiler:");
}

#[test]
fn presets_set_only_the_three_design_keys_once_shipped() {
    // Resolve each preset as if chapter 8 had shipped.
    for preset in PRESETS {
        let resolved = shipped(None, &[CliSetting::Preset(*preset)]);
        let view = resolved.journal().clues;
        let expected = match preset {
            Preset::Classic => ClueSettings::default(),
            Preset::Recommended | Preset::Everything => ClueSettings {
                clues: true,
                highlight: ClueHighlight::Asked,
                quest_tracker: true,
                ..ClueSettings::default()
            },
        };
        assert_eq!(view, expected, "{preset:?}");
        assert!(!view.spoiler_risk(), "{preset:?}");
    }
}

#[test]
fn t3c_the_parity_lock_gives_every_default() {
    let cli = [
        enable(CLUES_KEY),
        set(CLUE_HIGHLIGHT_KEY, "new"),
        enable(QUEST_TRACKER_KEY),
        set(TRACKER_EVIDENCE_KEY, "npc"),
        enable(CLUE_MISSES_KEY),
    ];
    let stored = Stored::default().with(CLUES_KEY, SettingValue::Bool(true));
    for reason in [LockReason::CliFlag, LockReason::EnvVar, LockReason::QaMode] {
        let locked = resolve_as_shipped(
            ResolveInput {
                stored: Some(&stored),
                cli: &cli,
                lock: LockState::Locked(reason),
                caps: HostCapabilities::ALL,
            },
            ALL_CLUES,
        );
        assert_eq!(locked.journal().clues, ClueSettings::default());
        assert!(locked.is_classic());
        assert_eq!(
            ResolvedEnhancements::locked(reason).journal().clues,
            ClueSettings::default()
        );
    }
    assert_eq!(
        ResolvedEnhancements::classic().journal().clues,
        ClueSettings::default()
    );
}

#[test]
fn the_words_round_trip() {
    for mode in ClueHighlight::ALL {
        assert_eq!(ClueHighlight::from_key(mode.key()), Some(mode));
    }
    for style in [HighlightStyle::Dotted, HighlightStyle::Tint] {
        assert_eq!(HighlightStyle::from_key(style.key()), Some(style));
    }
    for evidence in [TrackerEvidence::Record, TrackerEvidence::Npc] {
        assert_eq!(TrackerEvidence::from_key(evidence.key()), Some(evidence));
    }
    for text in [ClueText::Off, ClueText::Live] {
        assert_eq!(ClueText::from_key(text.key()), Some(text));
    }
    assert_eq!(ClueHighlight::from_key("everything"), None);
}
