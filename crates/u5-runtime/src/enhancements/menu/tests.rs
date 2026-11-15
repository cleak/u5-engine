//! Chapter 3's `[menu]`, `[audio]` and `[input]` rows and their typed views
//! (design 3.3.3, 3.3.6, 3.4.1; build plan M4-02). No row ships in this
//! build (U13), so the resolver tests use the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CatalogueRow, EffectClass, MENU_P1, MENU_P2, MENU_P3, MENU_P4, MENU_P5, MENU_P7,
    OPTIONS, OptionDefault, OptionKind, OptionSpec, PRESETS, Phase, Preset, PresetValue,
    SHIPPED_PHASES, TOGGLES, Tier, ToggleSpec, help_line, named_stops, settings_table,
};
use crate::enhancements::resolve::{PresetLabel, ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{
    Capability, CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue,
    SettingsSource, ValueProblem,
};

/// The phases M4 ships, in order. Phase 5 (input) and Phase 7 (in-app
/// load) are not in M4.
const M4_PHASES: &[Phase] = &[MENU_P1, MENU_P2, MENU_P3, MENU_P4];

/// What design 3.4.1 says one toggle is: key, class, apply point, tier,
/// alias and the phase that ships it (build plan M4-02).
const DESIGN_TOGGLES: &[(
    Toggle,
    &str,
    EffectClass,
    ApplyAt,
    Tier,
    Option<&str>,
    Phase,
)] = &[
    (
        Toggle::MenuOverlay,
        OVERLAY_KEY,
        EffectClass::InputDriven,
        ApplyAt::NextLaunch,
        Tier::Feature,
        Some("--menu-overlay"),
        MENU_P1,
    ),
    (
        Toggle::MenuTitleLayer,
        TITLE_LAYER_KEY,
        EffectClass::InputDriven,
        ApplyAt::NextLaunch,
        Tier::Feature,
        Some("--title-layer"),
        MENU_P3,
    ),
    (
        Toggle::MenuNamedSlots,
        NAMED_SLOTS_KEY,
        EffectClass::InputDriven,
        ApplyAt::IdlePrompt,
        Tier::Feature,
        Some("--named-slots"),
        MENU_P2,
    ),
    (
        Toggle::AudioMuted,
        MUTED_KEY,
        EffectClass::Presentation,
        ApplyAt::Frame,
        Tier::Feature,
        Some("--mute"),
        MENU_P1,
    ),
    (
        Toggle::AudioMuteOnFocusLoss,
        MUTE_ON_FOCUS_LOSS_KEY,
        EffectClass::Presentation,
        ApplyAt::Frame,
        Tier::Feature,
        Some("--mute-on-focus-loss"),
        MENU_P1,
    ),
    (
        Toggle::AudioStartSoundOff,
        START_SOUND_OFF_KEY,
        EffectClass::InputDriven,
        ApplyAt::NextLaunch,
        Tier::Deviation,
        Some("--start-sound-off"),
        MENU_P4,
    ),
    (
        Toggle::AudioStartBufferOn,
        START_BUFFER_ON_KEY,
        EffectClass::InputDriven,
        ApplyAt::NextLaunch,
        Tier::Deviation,
        Some("--start-buffer-on"),
        MENU_P4,
    ),
    (
        Toggle::InputGamepad,
        GAMEPAD_KEY,
        EffectClass::InputDriven,
        ApplyAt::Frame,
        Tier::Feature,
        None,
        MENU_P5,
    ),
    (
        Toggle::InputMouse,
        MOUSE_KEY,
        EffectClass::InputDriven,
        ApplyAt::Frame,
        Tier::Feature,
        None,
        MENU_P5,
    ),
    (
        Toggle::InputExtraBindings,
        EXTRA_BINDINGS_KEY,
        EffectClass::InputDriven,
        ApplyAt::Frame,
        Tier::Feature,
        None,
        MENU_P5,
    ),
];

/// One valued row of design 3.4.1: key, type, default, class, apply point,
/// alias, parent, switched toggle and phase.
struct DesignOption {
    key: &'static str,
    kind: OptionKind,
    default: OptionDefault,
    class: EffectClass,
    apply: ApplyAt,
    alias: Option<&'static str>,
    parent: Option<Toggle>,
    toggle: Option<Toggle>,
    ships: Phase,
}

const DESIGN_OPTIONS: &[DesignOption] = &[
    DesignOption {
        key: QUICKSAVE_KEY,
        kind: OptionKind::Bool,
        default: OptionDefault::Bool(true),
        class: EffectClass::InputDriven,
        apply: ApplyAt::IdlePrompt,
        alias: None,
        parent: Some(Toggle::MenuNamedSlots),
        toggle: None,
        ships: MENU_P2,
    },
    DesignOption {
        key: CONFIRM_UNSAVED_KEY,
        kind: OptionKind::Bool,
        default: OptionDefault::Bool(true),
        class: EffectClass::Presentation,
        apply: ApplyAt::Frame,
        alias: Some("--confirm-unsaved"),
        parent: Some(Toggle::MenuOverlay),
        toggle: None,
        ships: MENU_P4,
    },
    DesignOption {
        key: QUIT_TO_TITLE_KEY,
        kind: OptionKind::Enum(&["relaunch", "in-app"]),
        default: OptionDefault::Str("relaunch"),
        class: EffectClass::InputDriven,
        apply: ApplyAt::NextLaunch,
        alias: None,
        parent: Some(Toggle::MenuOverlay),
        toggle: None,
        ships: MENU_P3,
    },
    DesignOption {
        key: VOLUME_KEY,
        kind: OptionKind::Int {
            min: 0,
            max: 100,
            step: 5,
        },
        default: OptionDefault::Int(100),
        class: EffectClass::Presentation,
        apply: ApplyAt::Frame,
        alias: Some("--volume"),
        parent: None,
        toggle: Some(Toggle::AudioVolume),
        ships: MENU_P1,
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

/// Resolves as if chapter 3 Phases 1-4 had shipped.
fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), M4_PHASES)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

fn option(key: &str) -> &'static OptionSpec {
    OPTIONS.iter().find(|spec| spec.key == key).unwrap()
}

fn toggle_spec(key: &str) -> &'static ToggleSpec {
    TOGGLES.iter().find(|spec| spec.key == key).unwrap()
}

fn is_chapter_3(key: &str) -> bool {
    ["menu.", "audio.", "input."]
        .iter()
        .any(|section| key.starts_with(section))
}

// --- The rows. ---------------------------------------------------------

#[test]
fn every_toggle_matches_the_design() {
    for (toggle, key, class, apply, tier, alias, ships) in DESIGN_TOGGLES {
        let Some(CatalogueRow::Toggle(spec)) = CatalogueRow::find(key) else {
            panic!("{key} is a toggle");
        };
        assert_eq!(spec.toggle, *toggle, "{key}");
        assert_eq!(toggle.key(), *key);
        assert!(!spec.driven, "{key}");
        assert_eq!(spec.class, *class, "{key}");
        assert_eq!(spec.apply, *apply, "{key}");
        assert_eq!(spec.tier, *tier, "{key}");
        assert_eq!(spec.cli_aliases, alias.as_slice(), "{key}");
        assert_eq!(spec.ships, *ships, "{key}");
        assert!(spec.tags.is_empty() && spec.soft_deps.is_empty(), "{key}");
        assert!(!spec.label.trim().is_empty(), "{key}");
    }
    // Only the Phase 5 rows need something of the host: a gamepad, a
    // pointer, a keyboard. Everything else works on every host.
    for (_, key, ..) in DESIGN_TOGGLES {
        let needs = toggle_spec(key).needs;
        let expected: &[Capability] = match *key {
            GAMEPAD_KEY => &[Capability::Gamepad],
            MOUSE_KEY => &[Capability::Pointer],
            EXTRA_BINDINGS_KEY => &[Capability::Keyboard],
            _ => &[],
        };
        assert_eq!(needs, expected, "{key}");
    }
    // The volume's switch: driven, sharing the key, never a row of its own.
    let volume = Toggle::AudioVolume.spec();
    assert!(volume.driven);
    assert_eq!(volume.key, VOLUME_KEY);
    assert_eq!(volume.class, EffectClass::Presentation);
    assert_eq!(volume.ships, MENU_P1);
    assert!(volume.cli_aliases.is_empty());
}

#[test]
fn every_valued_row_matches_the_design() {
    for row in DESIGN_OPTIONS {
        let Some(CatalogueRow::Option(spec)) = CatalogueRow::find(row.key) else {
            panic!("{} is a valued setting", row.key);
        };
        assert_eq!(spec.kind, row.kind, "{}", row.key);
        assert_eq!(spec.default, row.default, "{}", row.key);
        assert_eq!(spec.class, Some(row.class), "{}", row.key);
        assert_eq!(spec.apply, row.apply, "{}", row.key);
        assert_eq!(spec.tier, Tier::Feature, "{}", row.key);
        assert_eq!(spec.cli_aliases, row.alias.as_slice(), "{}", row.key);
        assert_eq!(spec.parent, row.parent, "{}", row.key);
        assert_eq!(spec.toggle, row.toggle, "{}", row.key);
        assert_eq!(spec.ships, row.ships, "{}", row.key);
        assert_eq!(spec.switch, None, "{} has no --enable value", row.key);
        assert!(spec.deviation_values.is_empty(), "{}", row.key);
        assert!(spec.needs.is_empty() && spec.tags.is_empty(), "{}", row.key);
    }
    // `in-app` waits for Phase 7; no other value waits.
    let quit = option(QUIT_TO_TITLE_KEY);
    assert_eq!(quit.later_values, [("in-app", MENU_P7)]);
    assert_eq!(quit.value_ships("in-app"), Some(MENU_P7));
    assert_eq!(quit.value_ships("relaunch"), None);
    assert!(!quit.value_is_shipped("in-app"));
    assert!(quit.value_is_shipped("relaunch"));
    for spec in OPTIONS.iter().filter(|spec| spec.key != QUIT_TO_TITLE_KEY) {
        assert!(spec.later_values.is_empty(), "{}", spec.key);
    }
}

#[test]
fn the_rows_are_chapter_3s_in_the_designs_order() {
    let rows: Vec<&str> = CatalogueRow::all()
        .map(CatalogueRow::key)
        .filter(|key| is_chapter_3(key))
        .collect();
    assert_eq!(
        rows,
        [
            OVERLAY_KEY,
            TITLE_LAYER_KEY,
            NAMED_SLOTS_KEY,
            MUTED_KEY,
            MUTE_ON_FOCUS_LOSS_KEY,
            START_SOUND_OFF_KEY,
            START_BUFFER_ON_KEY,
            GAMEPAD_KEY,
            MOUSE_KEY,
            EXTRA_BINDINGS_KEY,
            QUICKSAVE_KEY,
            CONFIRM_UNSAVED_KEY,
            QUIT_TO_TITLE_KEY,
            VOLUME_KEY,
        ]
    );
    for key in &rows {
        assert_eq!(CatalogueRow::find(key).unwrap().ships().chapter, 3, "{key}");
    }
    assert_eq!(START_KEYS, [START_SOUND_OFF_KEY, START_BUFFER_ON_KEY]);
    assert_eq!(INPUT_KEYS, [GAMEPAD_KEY, MOUSE_KEY, EXTRA_BINDINGS_KEY]);
    // The new toggles follow every earlier bit, so none moved.
    let first = Toggle::MenuOverlay as usize;
    assert_eq!(first, Toggle::GemMapEnhanced as usize + 1);
    assert!(TOGGLES[first..].iter().all(|spec| is_chapter_3(spec.key)));
    // Reserved, not in the catalogue (U3).
    assert!(CatalogueRow::find("menu.escape_opens_menu").is_none());
}

#[test]
fn m4_ships_phases_1_to_4_only() {
    // M4-13 ships the menu's Phases 1-4; the optional 5 and 7 are not in M4.
    for phase in [MENU_P1, MENU_P2, MENU_P3, MENU_P4, MENU_P5, MENU_P7] {
        assert_eq!(phase.chapter, 3);
        let shipped = M4_PHASES.contains(&phase);
        assert_eq!(SHIPPED_PHASES.contains(&phase), shipped, "{phase:?}");
        assert_eq!(phase.is_shipped(), shipped, "{phase:?}");
    }
    assert_eq!(
        [MENU_P1, MENU_P2, MENU_P3, MENU_P4, MENU_P5, MENU_P7].map(|phase| phase.name),
        [
            "Phase 1", "Phase 2", "Phase 3", "Phase 4", "Phase 5", "Phase 7"
        ]
    );
    // Every row but Phase 5's `input.*` is visible (U13).
    for row in CatalogueRow::all().filter(|row| is_chapter_3(row.key())) {
        let hidden = INPUT_KEYS.contains(&row.key());
        assert_eq!(row.is_shipped(), !hidden, "{}", row.key());
    }
}

#[test]
fn volume_has_named_stops_on_its_steps_and_rows_have_help() {
    let stops = named_stops(VOLUME_KEY).unwrap();
    assert_eq!(stops, [100, 75, 50, 25, 0]);
    assert_eq!(stops[0], i64::from(VOLUME_DEFAULT));
    for stop in stops {
        assert_eq!(stop % i64::from(VOLUME_STEP), 0, "{stop}");
    }
    for key in [
        OVERLAY_KEY,
        TITLE_LAYER_KEY,
        NAMED_SLOTS_KEY,
        QUICKSAVE_KEY,
        CONFIRM_UNSAVED_KEY,
        VOLUME_KEY,
        MUTE_ON_FOCUS_LOSS_KEY,
        START_SOUND_OFF_KEY,
        START_BUFFER_ON_KEY,
    ] {
        let help = help_line(key).unwrap_or_else(|| panic!("{key} has a help line"));
        assert!(help.ends_with('.'), "{key}");
    }
    // The start keys say what they type; the volume says it is output only.
    assert!(help_line(START_SOUND_OFF_KEY).unwrap().contains("Ctrl-S"));
    assert!(help_line(START_BUFFER_ON_KEY).unwrap().contains("Ctrl-B"));
    assert!(help_line(VOLUME_KEY).unwrap().contains("loudness only"));
}

#[test]
fn quit_to_title_words_round_trip() {
    for choice in [QuitToTitle::Relaunch, QuitToTitle::InApp] {
        assert_eq!(QuitToTitle::from_key(choice.key()), Some(choice));
    }
    assert_eq!(QuitToTitle::default(), QuitToTitle::Relaunch);
    assert_eq!(QuitToTitle::from_key("restart"), None);
}

// --- The typed views. --------------------------------------------------

#[test]
fn the_views_over_the_defaults_are_the_original() {
    let defaults = MenuSettings::default();
    assert!(!defaults.overlay && !defaults.title_layer && !defaults.named_slots);
    assert!(defaults.quicksave && defaults.confirm_unsaved);
    assert_eq!(defaults.quit_to_title, QuitToTitle::Relaunch);
    assert!(!defaults.any_menu() && !defaults.quick_slot());
    let audio = AudioSettings::default();
    assert_eq!(audio.volume_percent, 100);
    assert!(!audio.gain_changed() && !audio.types_start_keys());
    assert_eq!(audio.volume(), 1.0);

    for resolved in [
        ResolvedEnhancements::classic(),
        ResolvedEnhancements::locked(LockReason::CliFlag),
        resolve(input(None, &[])),
        shipped(None, &[]),
    ] {
        assert_eq!(resolved.menu(), defaults);
        assert_eq!(resolved.audio(), audio);
        assert!(resolved.is_classic());
    }
    // The views agree with the catalogue's defaults.
    for row in DESIGN_OPTIONS {
        let value = shipped(None, &[]).values.get(row.key).cloned().unwrap();
        let expected = match row.default {
            OptionDefault::Bool(flag) => SettingValue::Bool(flag),
            OptionDefault::Int(number) => SettingValue::Int(number),
            OptionDefault::Str(word) => SettingValue::Str(word.to_string()),
            OptionDefault::List(_) => unreachable!(),
        };
        assert_eq!(value, expected, "{}", row.key);
    }
}

#[test]
fn the_views_read_file_values() {
    let stored = Stored::default()
        .with(OVERLAY_KEY, SettingValue::Bool(true))
        .with(TITLE_LAYER_KEY, SettingValue::Bool(true))
        .with(NAMED_SLOTS_KEY, SettingValue::Bool(true))
        .with(QUICKSAVE_KEY, SettingValue::Bool(false))
        .with(CONFIRM_UNSAVED_KEY, SettingValue::Bool(false))
        .with(QUIT_TO_TITLE_KEY, SettingValue::Str("relaunch".into()))
        .with(VOLUME_KEY, SettingValue::Int(35))
        .with(MUTED_KEY, SettingValue::Bool(true))
        .with(MUTE_ON_FOCUS_LOSS_KEY, SettingValue::Bool(true))
        .with(START_SOUND_OFF_KEY, SettingValue::Bool(true))
        .with(START_BUFFER_ON_KEY, SettingValue::Bool(true));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert_eq!(
        resolved.menu(),
        MenuSettings {
            overlay: true,
            title_layer: true,
            named_slots: true,
            quicksave: false,
            confirm_unsaved: false,
            quit_to_title: QuitToTitle::Relaunch,
        }
    );
    assert_eq!(
        resolved.audio(),
        AudioSettings {
            volume_percent: 35,
            muted: true,
            mute_on_focus_loss: true,
            start_sound_off: true,
            start_buffer_on: true,
        }
    );
    assert!(resolved.toggles.on(Toggle::AudioVolume));
    assert!(!resolved.menu().quick_slot());
}

#[test]
fn the_views_read_command_line_values_over_the_file() {
    let stored = Stored::default()
        .with(VOLUME_KEY, SettingValue::Int(35))
        .with(MUTED_KEY, SettingValue::Bool(true));
    let cli = [
        enable(OVERLAY_KEY),
        enable(NAMED_SLOTS_KEY),
        set(VOLUME_KEY, "80"),
        disable(MUTED_KEY),
        enable(MUTE_ON_FOCUS_LOSS_KEY),
        disable(CONFIRM_UNSAVED_KEY),
        enable(START_BUFFER_ON_KEY),
    ];
    let resolved = shipped(Some(&stored), &cli);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let menu = resolved.menu();
    assert!(menu.overlay && menu.named_slots && !menu.title_layer);
    assert!(menu.quick_slot() && !menu.confirm_unsaved);
    assert!(menu.any_menu());
    let audio = resolved.audio();
    assert_eq!(audio.volume_percent, 80);
    assert!(!audio.muted && audio.mute_on_focus_loss);
    assert!(!audio.start_sound_off && audio.start_buffer_on);
    assert!(audio.gain_changed() && audio.types_start_keys());
    assert_eq!(audio.volume(), 0.8);
}

#[test]
fn volume_off_its_steps_falls_back_with_a_note() {
    let resolved = shipped(None, &[set(VOLUME_KEY, "7")]);
    assert_eq!(
        note_lines(&resolved),
        ["audio.volume = 7: not a step of 5, using 100"]
    );
    assert!(matches!(
        resolved.notes[..],
        [SettingNote::BadValue {
            problem: ValueProblem::NotAStep { step: 5 },
            ..
        }]
    ));
    assert_eq!(resolved.audio().volume_percent, 100);
    assert!(resolved.is_classic(), "the fallback is the default");

    // The same from the file, and out of range.
    let stored = Stored::default().with(VOLUME_KEY, SettingValue::Int(52));
    let resolved = shipped(Some(&stored), &[]);
    assert_eq!(
        note_lines(&resolved),
        ["audio.volume = 52: not a step of 5, using 100"]
    );
    let resolved = shipped(None, &[set(VOLUME_KEY, "105")]);
    assert_eq!(
        note_lines(&resolved),
        ["audio.volume = 105: not in 0..100, using 100"]
    );
    // Every step is accepted, the ends included.
    for percent in (0..=100).step_by(5) {
        let resolved = shipped(None, &[set(VOLUME_KEY, &percent.to_string())]);
        assert!(resolved.notes.is_empty(), "{percent}");
        assert_eq!(i64::from(resolved.audio().volume_percent), percent);
        assert_eq!(resolved.toggles.on(Toggle::AudioVolume), percent != 100);
    }
    // `--volume` is valued, never a switch.
    let resolved = shipped(None, &[enable(VOLUME_KEY)]);
    assert_eq!(
        note_lines(&resolved),
        ["audio.volume: not an on/off setting, ignored (use --set)"]
    );
}

#[test]
fn the_audio_view_guards_a_hand_built_volume() {
    // The resolver never yields these; the view still stays on the steps.
    let mut resolved = shipped(None, &[set(VOLUME_KEY, "50")]);
    for (value, expected) in [(7, 5), (-3, 0), (250, 100), (99, 95)] {
        resolved.values = resolve_values_with(VOLUME_KEY, value);
        assert_eq!(resolved.audio().volume_percent, expected, "{value}");
    }
}

/// The resolved values of a shipped build with `key` forced to `value`,
/// bypassing the resolver's checks.
fn resolve_values_with(key: &str, value: i64) -> crate::enhancements::resolve::SettingValues {
    let stored = Stored::default();
    let base = shipped(Some(&stored), &[]);
    let pairs: Vec<(&'static str, SettingValue)> = base
        .values
        .iter()
        .map(|(k, v)| {
            if k == key {
                (k, SettingValue::Int(value))
            } else {
                (k, v.clone())
            }
        })
        .collect();
    crate::enhancements::resolve::SettingValues::from_pairs(pairs)
}

#[test]
fn in_app_resolves_to_relaunch_with_a_note_until_phase_7() {
    let resolved = shipped(None, &[set(QUIT_TO_TITLE_KEY, "in-app")]);
    assert_eq!(
        note_lines(&resolved),
        ["menu.quit_to_title = in-app: not available in this build, using relaunch"]
    );
    assert_eq!(resolved.menu().quit_to_title, QuitToTitle::Relaunch);
    let stored = Stored::default().with(QUIT_TO_TITLE_KEY, SettingValue::Str("in-app".into()));
    let resolved = shipped(Some(&stored), &[]);
    assert_eq!(resolved.notes.len(), 1, "{:?}", note_lines(&resolved));
    assert_eq!(resolved.menu().quit_to_title, QuitToTitle::Relaunch);

    // Once Phase 7 ships, the value is taken as it is.
    let mut phases = M4_PHASES.to_vec();
    phases.push(MENU_P7);
    let resolved = resolve_as_shipped(input(None, &[set(QUIT_TO_TITLE_KEY, "in-app")]), &phases);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert_eq!(resolved.menu().quit_to_title, QuitToTitle::InApp);
    // A word that is no choice at all is the usual bad value.
    let resolved = shipped(None, &[set(QUIT_TO_TITLE_KEY, "restart")]);
    assert_eq!(
        note_lines(&resolved),
        ["menu.quit_to_title = restart: not one of relaunch, in-app, using relaunch"]
    );
}

#[test]
fn input_rows_are_not_available_in_this_build() {
    // Even with every M4 phase shipped, Phase 5 is not.
    for key in INPUT_KEYS {
        for cli in [enable(key), set(key, "true")] {
            let resolved = shipped(None, &[cli]);
            assert_eq!(
                note_lines(&resolved),
                [format!("{key}: not available in this build")]
            );
            assert!(resolved.is_classic(), "{key}");
        }
        let stored = Stored::default().with(key, SettingValue::Bool(true));
        let resolved = shipped(Some(&stored), &[]);
        assert_eq!(
            note_lines(&resolved),
            [format!("{key}: not available in this build")]
        );
        assert!(resolved.is_classic());
    }
    // With Phase 5 shipped they resolve, each needing its device.
    let mut phases = M4_PHASES.to_vec();
    phases.push(MENU_P5);
    let resolved = resolve_as_shipped(
        input(None, &[enable(GAMEPAD_KEY), enable(MOUSE_KEY)]),
        &phases,
    );
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert!(resolved.toggles.on(Toggle::InputGamepad) && resolved.toggles.on(Toggle::InputMouse));
}

#[test]
fn only_the_phase_5_rows_are_not_available_in_this_build() {
    // The build as it ships: every `input.*` key (Phase 5) resolves to its
    // default with the note, and the game stays Classic.
    let cli: Vec<CliSetting> = CatalogueRow::all()
        .filter(|row| is_chapter_3(row.key()) && !row.ships().is_shipped())
        .map(|row| match row {
            CatalogueRow::Toggle(spec) => enable(spec.key),
            CatalogueRow::Option(spec) => set(
                spec.key,
                match spec.key {
                    VOLUME_KEY => "50",
                    QUIT_TO_TITLE_KEY => "relaunch",
                    _ => "false",
                },
            ),
        })
        .collect();
    let resolved = resolve(input(None, &cli));
    assert_eq!(
        resolved.notes.len(),
        cli.len(),
        "{:?}",
        note_lines(&resolved)
    );
    for note in &resolved.notes {
        assert!(matches!(note, SettingNote::NotAvailable { .. }), "{note}");
    }
    assert!(resolved.is_classic());
    assert_eq!(resolved.menu(), MenuSettings::default());
    assert_eq!(resolved.audio(), AudioSettings::default());
    assert_eq!(cli.len(), INPUT_KEYS.len());
    // Every other chapter 3 key resolves as asked, with no note.
    let resolved = resolve(input(
        None,
        &[enable(OVERLAY_KEY), enable(START_SOUND_OFF_KEY)],
    ));
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert!(resolved.menu().overlay && resolved.audio().start_sound_off);
}

#[test]
fn sub_keys_alone_leave_the_game_classic() {
    // Each sub-option is inert while its parent is off (1.3.5).
    for cli in [
        set(QUICKSAVE_KEY, "false"),
        disable(CONFIRM_UNSAVED_KEY),
        set(QUIT_TO_TITLE_KEY, "relaunch"),
    ] {
        let resolved = shipped(None, std::slice::from_ref(&cli));
        assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
        assert!(resolved.is_classic(), "{cli}");
        assert!(!resolved.menu().any_menu());
    }
    let stored = Stored::default()
        .with(QUICKSAVE_KEY, SettingValue::Bool(false))
        .with(CONFIRM_UNSAVED_KEY, SettingValue::Bool(false));
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.is_classic());
    assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Classic));
    // The volume is a valued toggle: away from 100 it is not Classic, so
    // its plugin is added (1.3.5), and back at 100 it is.
    let resolved = shipped(None, &[set(VOLUME_KEY, "50")]);
    assert!(!resolved.is_classic());
    assert_eq!(
        resolved.toggles.iter_on().collect::<Vec<_>>(),
        [Toggle::AudioVolume]
    );
    assert!(shipped(None, &[set(VOLUME_KEY, "100")]).is_classic());
}

#[test]
fn each_toggle_alone_turns_on_only_itself() {
    let mut phases = M4_PHASES.to_vec();
    phases.push(MENU_P5);
    for (toggle, key, ..) in DESIGN_TOGGLES {
        let resolved = resolve_as_shipped(input(None, &[enable(key)]), &phases);
        assert!(
            resolved.notes.is_empty(),
            "{key}: {:?}",
            note_lines(&resolved)
        );
        assert_eq!(
            resolved.toggles.iter_on().collect::<Vec<_>>(),
            [*toggle],
            "{key}"
        );
    }
    // The start preferences each work with everything else off (3.4.1).
    let audio = shipped(None, &[enable(START_SOUND_OFF_KEY)]).audio();
    assert!(audio.start_sound_off && !audio.start_buffer_on && audio.types_start_keys());
    assert!(!audio.gain_changed());
}

#[test]
fn presets_skip_hidden_rows_and_never_set_the_start_keys() {
    // The build as it ships (M4-13): no preset sets a start key, and only
    // Everything sets a chapter 3 row.
    for preset in PRESETS {
        let resolved = resolve(input(None, &[CliSetting::Preset(*preset)]));
        let audio = resolved.audio();
        assert!(!audio.types_start_keys(), "{preset:?}");
        if *preset != Preset::Everything {
            assert_eq!(resolved.menu(), MenuSettings::default(), "{preset:?}");
            assert_eq!(audio, AudioSettings::default(), "{preset:?}");
        }
        assert_eq!(
            resolved,
            shipped(None, &[CliSetting::Preset(*preset)]),
            "{preset:?}: the shipped build is the M4 build"
        );
    }
    // Everything sets the four feature toggles of 3.4.1 and nothing else
    // of this chapter: no preference, no start key.
    let resolved = shipped(None, &[CliSetting::Preset(Preset::Everything)]);
    let menu = resolved.menu();
    assert!(menu.overlay && menu.named_slots && menu.title_layer);
    assert!(menu.quicksave && menu.confirm_unsaved);
    let audio = resolved.audio();
    assert_eq!(
        audio,
        AudioSettings {
            mute_on_focus_loss: true,
            ..AudioSettings::default()
        }
    );
    // Recommended holds no key of this chapter (U2).
    let resolved = shipped(None, &[CliSetting::Preset(Preset::Recommended)]);
    assert_eq!(resolved.menu(), MenuSettings::default());
    assert_eq!(resolved.audio(), AudioSettings::default());

    // The start keys, the preferences and the input rows are in no preset.
    for preset in PRESETS {
        for entry in preset.entries() {
            let never = START_KEYS.contains(&entry.key)
                || INPUT_KEYS.contains(&entry.key)
                || [
                    VOLUME_KEY,
                    MUTED_KEY,
                    QUICKSAVE_KEY,
                    CONFIRM_UNSAVED_KEY,
                    QUIT_TO_TITLE_KEY,
                ]
                .contains(&entry.key);
            assert!(!never, "{preset:?} sets {}", entry.key);
        }
    }
    // The chapter 3 entries of Everything are exactly the design's.
    let everything: Vec<(&str, PresetValue)> = Preset::Everything
        .entries()
        .iter()
        .filter(|entry| is_chapter_3(entry.key))
        .map(|entry| (entry.key, entry.value))
        .collect();
    assert_eq!(
        everything,
        [
            (OVERLAY_KEY, PresetValue::On),
            (NAMED_SLOTS_KEY, PresetValue::On),
            (TITLE_LAYER_KEY, PresetValue::On),
            (MUTE_ON_FOCUS_LOSS_KEY, PresetValue::On),
        ]
    );
    // A stored volume survives `--preset classic`: presets set toggles,
    // never preferences.
    let stored = Stored::default().with(VOLUME_KEY, SettingValue::Int(40));
    let resolved = shipped(Some(&stored), &[CliSetting::Preset(Preset::Classic)]);
    assert_eq!(resolved.audio().volume_percent, 40);
}

#[test]
fn the_lock_gives_the_defaults() {
    let stored = Stored::default()
        .with(OVERLAY_KEY, SettingValue::Bool(true))
        .with(VOLUME_KEY, SettingValue::Int(20));
    let resolved = resolve_as_shipped(
        ResolveInput {
            stored: Some(&stored),
            cli: &[enable(START_SOUND_OFF_KEY)],
            lock: LockState::Locked(LockReason::CliFlag),
            caps: HostCapabilities::ALL,
        },
        M4_PHASES,
    );
    assert!(resolved.is_classic());
    assert_eq!(resolved.menu(), MenuSettings::default());
    assert_eq!(resolved.audio(), AudioSettings::default());
}

// --- The docs table. ---------------------------------------------------

#[test]
fn the_docs_table_lists_the_m4_rows_as_shipped() {
    let table = settings_table();
    for line in [
        "| menu.overlay | Menu on F12 at the command prompt | bool = false | b, L | feature | --menu-overlay | - | yes |",
        "| menu.title_layer | Menu on F12 at the title screen | bool = false | b, L | feature | --title-layer | - | yes |",
        "| menu.named_slots | Named save slots | bool = false | b, P | feature | --named-slots | - | yes |",
        "| audio.muted | Mute | bool = false | a, F | feature | --mute | - | yes |",
        "| audio.start_sound_off | Begin with the game's sound off (types Ctrl-S once) | bool = false | b, L | deviation | --start-sound-off | - | yes |",
        "| input.gamepad | Play with a gamepad | bool = false | b, F | feature | - | a gamepad | later (chapter 3 Phase 5) |",
        "| menu.quicksave | A quick save slot | bool = true | b, P | feature | - | - | yes |",
        "| menu.confirm_unsaved | Ask before leaving with unsaved play | bool = true | a, F | feature | --confirm-unsaved | - | yes |",
        "| menu.quit_to_title | Return to the title by restarting, or in the window (later) | enum relaunch\\|in-app = relaunch (in-app later, chapter 3 Phase 7) | b, L | feature | - | - | yes |",
        "| audio.volume | Volume in percent (output only; 100 is the original) | int 0..100 by 5 = 100 | a, F | feature | --volume | - | yes |",
    ] {
        assert!(table.contains(&format!("{line}\n")), "missing: {line}");
    }
    // The driven switch is not a row of its own.
    assert_eq!(table.matches("| audio.volume |").count(), 1);
}
