//! Chapter 6's `[combat]` rows and their typed view (design 6.2.8, 6.4.1;
//! build plan M7-01). Phases 3c and 4d ship in M7-07; allies (Phase 5)
//! stays hidden (U13), so the tests that need it use the catalogue's test
//! override.

use super::*;
use crate::auto_combat::{ACTION_CAP_DEFAULT, PAUSE_BELOW_HP_DEFAULT, STEP_MS_DEFAULT};
use crate::constants::SPELL_CODES;
use crate::enhancements::catalogue::{
    ApplyAt, COMBAT_P3C, COMBAT_P4D, COMBAT_P5, CatalogueRow, EffectClass, OPTIONS, OptionDefault,
    OptionKind, PRESET_DEVIATION_EXCEPTION, PRESETS, Phase, SHIPPED_PHASES, TOGGLES, Tier,
    settings_table,
};
use crate::enhancements::resolve::{ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{
    CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue, SettingsSource,
};

const ALL_COMBAT: &[Phase] = &[COMBAT_P3C, COMBAT_P4D, COMBAT_P5];

/// What 6.2.8 says each row is: key, kind, default, apply point, tier and
/// shipping phase. The three toggles are `None` kinds.
type DesignRow = (
    &'static str,
    Option<(OptionKind, OptionDefault)>,
    ApplyAt,
    Tier,
    Phase,
);

const DESIGN_ROWS: &[DesignRow] = &[
    (AUTO_KEY, None, ApplyAt::Frame, Tier::Feature, COMBAT_P3C),
    (
        ALLIES_KEY,
        None,
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P5,
    ),
    (
        FAST_FORWARD_KEY,
        None,
        ApplyAt::IdlePrompt,
        Tier::Deviation,
        COMBAT_P4D,
    ),
    (
        TACTIC_KEY,
        Some((
            OptionKind::Enum(&["fight", "support", "magic"]),
            OptionDefault::Str("fight"),
        )),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        TARGETING_KEY,
        Some((
            OptionKind::Enum(&["nearest", "weakest"]),
            OptionDefault::Str("nearest"),
        )),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        INTERRUPT_KEY,
        Some((
            OptionKind::Enum(&["swallow", "deliver"]),
            OptionDefault::Str("swallow"),
        )),
        ApplyAt::Frame,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        STEP_MS_KEY,
        Some((
            OptionKind::Int { min: 0, max: 2000 },
            OptionDefault::Int(150),
        )),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        PAUSE_BELOW_HP_KEY,
        Some((OptionKind::Int { min: 0, max: 100 }, OptionDefault::Int(25))),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        ACTION_CAP_KEY,
        Some((
            OptionKind::Int { min: 20, max: 1000 },
            OptionDefault::Int(200),
        )),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        VICTORY_KEY,
        Some((
            OptionKind::Enum(&["stop", "exit"]),
            OptionDefault::Str("stop"),
        )),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        KEEP_BETWEEN_FIGHTS_KEY,
        Some((OptionKind::Bool, OptionDefault::Bool(false))),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        PAUSE_ON_FOCUS_LOSS_KEY,
        Some((OptionKind::Bool, OptionDefault::Bool(true))),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        POTIONS_KEY,
        Some((OptionKind::Bool, OptionDefault::Bool(false))),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        MP_RESERVE_KEY,
        Some((
            OptionKind::Enum(&["none", "heal"]),
            OptionDefault::Str("heal"),
        )),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
    ),
    (
        SPELLS_KEY,
        Some((OptionKind::List(&SPELL_CODES), OptionDefault::List(&[]))),
        ApplyAt::IdlePrompt,
        Tier::Feature,
        COMBAT_P3C,
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

/// Resolves as if every chapter 6 phase had shipped.
fn shipped(stored: Option<&dyn SettingsSource>, cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve_as_shipped(input(stored, cli), ALL_COMBAT)
}

fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
    resolved.notes.iter().map(ToString::to_string).collect()
}

// --- The rows. ---------------------------------------------------------

#[test]
fn every_row_matches_the_design() {
    for (key, kind, apply, tier, ships) in DESIGN_ROWS {
        let row = CatalogueRow::find(key).unwrap_or_else(|| panic!("{key} has a row"));
        assert_eq!(row.ships(), *ships, "{key}");
        assert!(row.tags().is_empty(), "{key}");
        match (row, kind) {
            (CatalogueRow::Toggle(spec), None) => {
                assert!(!spec.driven, "{key}");
                assert_eq!(spec.class, EffectClass::InputDriven, "{key}");
                assert_eq!(spec.apply, *apply, "{key}");
                assert_eq!(spec.tier, *tier, "{key}");
                assert!(spec.needs.is_empty(), "{key}");
            }
            (CatalogueRow::Option(spec), Some((kind, default))) => {
                assert_eq!(spec.kind, *kind, "{key}");
                assert_eq!(spec.default, *default, "{key}");
                assert_eq!(spec.class, Some(EffectClass::InputDriven), "{key}");
                assert_eq!(spec.apply, *apply, "{key}");
                assert_eq!(spec.tier, *tier, "{key}");
                assert!(spec.deviation_values.is_empty(), "{key}");
                // A parameter of auto-combat: inert while it is off, never a
                // switch, no alias.
                assert_eq!(spec.parent, Some(Toggle::CombatAuto), "{key}");
                assert_eq!((spec.toggle, spec.switch), (None, None), "{key}");
                assert!(spec.cli_aliases.is_empty(), "{key}");
                assert!(spec.needs.is_empty(), "{key}");
            }
            (row, kind) => panic!("{key} is the wrong kind of row: {row:?} {kind:?}"),
        }
        assert!(!row.label().trim().is_empty(), "{key}");
    }
    // Fifteen rows, in 6.2.8's order, and the key list is the same.
    let rows: Vec<&str> = CatalogueRow::all()
        .filter(|row| row.key().starts_with("combat."))
        .map(CatalogueRow::key)
        .collect();
    let design: Vec<&str> = DESIGN_ROWS.iter().map(|(key, ..)| *key).collect();
    assert_eq!(rows.len(), 15);
    assert_eq!(rows, design);
    assert_eq!(COMBAT_KEYS, design.as_slice());
}

#[test]
fn the_toggles_and_aliases_match_the_design() {
    let expected = [
        (Toggle::CombatAuto, AUTO_KEY, "--auto-combat"),
        (Toggle::CombatAllies, ALLIES_KEY, "--auto-combat-allies"),
        (
            Toggle::CombatFastForward,
            FAST_FORWARD_KEY,
            "--auto-combat-fast-forward",
        ),
    ];
    for (toggle, key, alias) in expected {
        assert_eq!(toggle.key(), key);
        assert_eq!(toggle.spec().cli_aliases, [alias]);
    }
    let combat_toggles: Vec<Toggle> = TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("combat."))
        .map(|spec| spec.toggle)
        .collect();
    assert_eq!(
        combat_toggles,
        [
            Toggle::CombatAuto,
            Toggle::CombatAllies,
            Toggle::CombatFastForward
        ]
    );
    // The two sub-toggles say they need the master to do anything.
    for toggle in [Toggle::CombatAllies, Toggle::CombatFastForward] {
        let deps = toggle.spec().soft_deps;
        assert_eq!(deps.len(), 1);
        assert_eq!((deps[0].prefers, deps[0].fallback), (AUTO_KEY, "inert"));
    }
    // No other row of the catalogue is an auto-combat alias.
    let aliases: Vec<&str> = CatalogueRow::all()
        .flat_map(|row| row.cli_aliases().iter().copied())
        .filter(|alias| alias.starts_with("--auto-combat"))
        .collect();
    assert_eq!(
        aliases,
        [
            "--auto-combat",
            "--auto-combat-allies",
            "--auto-combat-fast-forward"
        ]
    );
    // The one deviation-tier row is the preset exception (U6).
    let deviations: Vec<&str> = CatalogueRow::all()
        .filter(|row| row.key().starts_with("combat."))
        .filter(|row| row.tier_for(None) != Tier::Feature)
        .map(CatalogueRow::key)
        .collect();
    assert_eq!(deviations, [PRESET_DEVIATION_EXCEPTION]);
}

#[test]
fn phases_3c_and_4d_have_shipped_and_allies_has_not() {
    // M7-07: auto-combat, its options and fast-forward are live; Phase 5
    // (allies) is out of M7.
    assert!(SHIPPED_PHASES.contains(&COMBAT_P3C));
    assert!(SHIPPED_PHASES.contains(&COMBAT_P4D));
    assert!(!SHIPPED_PHASES.contains(&COMBAT_P5));
    assert_eq!(
        ALL_COMBAT
            .iter()
            .map(|phase| (phase.chapter, phase.name))
            .collect::<Vec<_>>(),
        [(6, "3c"), (6, "4d"), (6, "5")]
    );
    for key in COMBAT_KEYS {
        assert_eq!(
            CatalogueRow::find(key).unwrap().is_shipped(),
            *key != ALLIES_KEY,
            "{key}"
        );
    }
    // The generated table lists allies as later and the rest as shipped.
    let table = settings_table();
    for key in COMBAT_KEYS {
        let line = table
            .lines()
            .find(|line| line.starts_with(&format!("| {key} | ")))
            .unwrap_or_else(|| panic!("{key} is in the table"));
        let later = line.contains("| later (chapter 6 ");
        assert_eq!(later, *key == ALLIES_KEY, "{line}");
    }
    assert!(table.contains(
        "| combat.auto | Auto-combat | bool = false | b, F | feature | --auto-combat | - | yes |"
    ));
    assert!(table.contains(
        "| combat.fast_forward | Faster fights: changes timing, so fights differ from normal \
         pace | bool = false | b, P | deviation | --auto-combat-fast-forward | - | yes |"
    ));
    assert!(table.contains("| int 0..2000 = 150 | b, P | feature | - | - | yes |"));
}

// --- Resolution of the hidden row. ----------------------------------------

#[test]
fn allies_resolves_to_the_default_with_a_note_and_the_rest_resolve() {
    let cli = [
        enable(AUTO_KEY),
        enable(ALLIES_KEY),
        enable(FAST_FORWARD_KEY),
        set(STEP_MS_KEY, "0"),
        set(TACTIC_KEY, "magic"),
    ];
    let resolved = resolve(input(None, &cli));
    let combat = resolved.combat();
    assert!(combat.auto && combat.fast_forward && !combat.allies);
    assert_eq!(combat.step_ms, 0);
    assert_eq!(
        note_lines(&resolved),
        ["combat.allies: not available in this build"]
    );
    // A stored allies value is kept by the host and reported the same way.
    let stored = Stored::default()
        .with(ALLIES_KEY, SettingValue::Bool(true))
        .with(ACTION_CAP_KEY, SettingValue::Int(50));
    let resolved = resolve(input(Some(&stored), &[]));
    assert!(!resolved.combat().allies);
    assert_eq!(resolved.combat().action_cap, 50);
    assert_eq!(
        note_lines(&resolved),
        ["combat.allies: not available in this build"]
    );
}

#[test]
fn presets_set_auto_and_the_fast_forward_exception_and_skip_allies() {
    use crate::enhancements::catalogue::Preset;
    for preset in PRESETS {
        let resolved = resolve(input(None, &[CliSetting::Preset(*preset)]));
        let combat = resolved.combat();
        let on = matches!(preset, Preset::Recommended | Preset::Everything);
        assert_eq!(combat.auto, on, "{preset:?}");
        // The one deviation-tier key in a preset (U6).
        assert_eq!(combat.fast_forward, on, "{preset:?}");
        assert!(!combat.allies, "{preset:?}");
        // Options stay at their defaults.
        assert_eq!(
            CombatSettings {
                auto: false,
                fast_forward: false,
                ..combat
            },
            CombatSettings::default(),
            "{preset:?}"
        );
        // Skipping the hidden row is silent (U13).
        assert!(
            !note_lines(&resolved)
                .iter()
                .any(|line| line.starts_with("combat.")),
            "{preset:?}: {:?}",
            note_lines(&resolved)
        );
    }
}

#[test]
fn once_shipped_the_presets_name_the_design_keys() {
    // Recommended and Everything both set `auto` and the `fast_forward`
    // exception; only Everything sets `allies` (6.4.1, U2, U6).
    let toggles = |preset| shipped(None, &[CliSetting::Preset(preset)]).combat();
    use crate::enhancements::catalogue::Preset;
    let classic = toggles(Preset::Classic);
    assert_eq!(classic, CombatSettings::default());
    let recommended = toggles(Preset::Recommended);
    assert!(recommended.auto && recommended.fast_forward && !recommended.allies);
    let everything = toggles(Preset::Everything);
    assert!(everything.auto && everything.fast_forward && everything.allies);
    // Options stay at their defaults: presets set toggles only.
    assert_eq!(
        CombatSettings {
            auto: false,
            allies: false,
            fast_forward: false,
            ..everything
        },
        CombatSettings::default()
    );
}

// --- The typed view. -----------------------------------------------------

#[test]
fn the_defaults_match_the_design() {
    let defaults = CombatSettings::default();
    assert!(!defaults.auto);
    assert_eq!(ResolvedEnhancements::classic().combat(), defaults);
    assert_eq!(shipped(None, &[]).combat(), defaults);
    for reason in [LockReason::CliFlag, LockReason::EnvVar, LockReason::QaMode] {
        assert_eq!(ResolvedEnhancements::locked(reason).combat(), defaults);
    }
    // The view's defaults are the catalogue's.
    let option = |key: &str| OPTIONS.iter().find(|spec| spec.key == key).unwrap();
    assert_eq!(
        option(STEP_MS_KEY).default,
        OptionDefault::Int(i64::from(STEP_MS_DEFAULT))
    );
    assert_eq!(
        option(PAUSE_BELOW_HP_KEY).default,
        OptionDefault::Int(i64::from(PAUSE_BELOW_HP_DEFAULT))
    );
    assert_eq!(
        option(ACTION_CAP_KEY).default,
        OptionDefault::Int(i64::from(ACTION_CAP_DEFAULT))
    );
    assert_eq!(
        option(STEP_MS_KEY).kind,
        OptionKind::Int {
            min: i64::from(STEP_MS_MIN),
            max: i64::from(STEP_MS_MAX)
        }
    );
    assert_eq!(
        option(ACTION_CAP_KEY).kind,
        OptionKind::Int {
            min: i64::from(ACTION_CAP_MIN),
            max: i64::from(ACTION_CAP_MAX)
        }
    );
}

#[test]
fn every_row_round_trips_through_the_view() {
    let cli = [
        enable(AUTO_KEY),
        enable(ALLIES_KEY),
        enable(FAST_FORWARD_KEY),
        set(TACTIC_KEY, "support"),
        set(TARGETING_KEY, "weakest"),
        set(INTERRUPT_KEY, "deliver"),
        set(STEP_MS_KEY, "0"),
        set(PAUSE_BELOW_HP_KEY, "0"),
        set(ACTION_CAP_KEY, "1000"),
        set(VICTORY_KEY, "exit"),
        enable(KEEP_BETWEEN_FIGHTS_KEY),
        disable(PAUSE_ON_FOCUS_LOSS_KEY),
        enable(POTIONS_KEY),
        set(MP_RESERVE_KEY, "none"),
        set(SPELLS_KEY, "GP,IL"),
    ];
    let resolved = shipped(None, &cli);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    let expected = CombatSettings {
        auto: true,
        allies: true,
        fast_forward: true,
        tactic: AutoTactic::Support,
        targeting: AutoTargeting::Weakest,
        interrupt: InterruptMode::Deliver,
        step_ms: 0,
        pause_below_hp_percent: 0,
        action_cap: 1000,
        victory: VictoryAction::Exit,
        keep_between_fights: true,
        pause_on_focus_loss: false,
        potions: true,
        mp_reserve: MpReserve::None,
        spell_allow_list: vec!["GP".into(), "IL".into()],
    };
    assert_eq!(resolved.combat(), expected);

    // The same values from the file.
    let stored = Stored::default()
        .with(AUTO_KEY, SettingValue::Bool(true))
        .with(ALLIES_KEY, SettingValue::Bool(true))
        .with(FAST_FORWARD_KEY, SettingValue::Bool(true))
        .with(TACTIC_KEY, SettingValue::Str("support".into()))
        .with(TARGETING_KEY, SettingValue::Str("weakest".into()))
        .with(INTERRUPT_KEY, SettingValue::Str("deliver".into()))
        .with(STEP_MS_KEY, SettingValue::Int(0))
        .with(PAUSE_BELOW_HP_KEY, SettingValue::Int(0))
        .with(ACTION_CAP_KEY, SettingValue::Int(1000))
        .with(VICTORY_KEY, SettingValue::Str("exit".into()))
        .with(KEEP_BETWEEN_FIGHTS_KEY, SettingValue::Bool(true))
        .with(PAUSE_ON_FOCUS_LOSS_KEY, SettingValue::Bool(false))
        .with(POTIONS_KEY, SettingValue::Bool(true))
        .with(MP_RESERVE_KEY, SettingValue::Str("none".into()))
        .with(
            SPELLS_KEY,
            SettingValue::List(vec!["GP".into(), "IL".into()]),
        );
    let resolved = shipped(Some(&stored), &[]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert_eq!(resolved.combat(), expected);

    // Every enum word maps to its own variant.
    for (index, word) in TACTIC_WORDS.iter().enumerate() {
        let tactic = shipped(None, &[set(TACTIC_KEY, word)]).combat().tactic;
        assert_eq!(tactic as usize, index, "{word}");
    }
    for (index, word) in MP_RESERVE_WORDS.iter().enumerate() {
        let reserve = shipped(None, &[set(MP_RESERVE_KEY, word)])
            .combat()
            .mp_reserve;
        assert_eq!(reserve as usize, index, "{word}");
    }
}

#[test]
fn an_option_alone_leaves_the_game_classic() {
    // Inert without the master toggle (1.3.5).
    let resolved = shipped(None, &[set(STEP_MS_KEY, "500"), enable(POTIONS_KEY)]);
    assert!(resolved.notes.is_empty(), "{:?}", note_lines(&resolved));
    assert!(resolved.toggles.is_classic());
    let view = resolved.combat();
    assert!(!view.auto);
    assert_eq!(view.step_ms, 500);
}

#[test]
fn malformed_values_fall_back_with_a_note() {
    // `auto = "yes"` in the file is not a bool: off, with a note, and the
    // rest of the file still loads (6.2.8, R4.2).
    let stored = Stored::default()
        .with(AUTO_KEY, SettingValue::Str("yes".into()))
        .with(STEP_MS_KEY, SettingValue::Int(400))
        .with(TACTIC_KEY, SettingValue::Str("berserk".into()))
        .with(SPELLS_KEY, SettingValue::List(vec!["XYZZY".into()]));
    let resolved = shipped(Some(&stored), &[]);
    let view = resolved.combat();
    assert!(!view.auto);
    assert_eq!(view.step_ms, 400);
    assert_eq!(view.tactic, AutoTactic::Fight);
    assert!(view.spell_allow_list.is_empty());
    let lines = note_lines(&resolved);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(
        lines.iter().any(|line| line.starts_with("combat.auto = ")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("combat.tactic = ")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("combat.spells = ")),
        "{lines:?}"
    );
    for note in &resolved.notes {
        assert!(matches!(note, SettingNote::BadValue { .. }), "{note:?}");
    }
}

#[test]
fn out_of_range_numbers_keep_the_default_and_the_view_clamps() {
    // The resolver rejects a value outside the row's range with a note
    // and uses the default.
    for (key, value, field) in [
        (STEP_MS_KEY, "2001", 0),
        (ACTION_CAP_KEY, "19", 1),
        (ACTION_CAP_KEY, "1001", 1),
        (PAUSE_BELOW_HP_KEY, "101", 2),
    ] {
        let resolved = shipped(None, &[set(key, value)]);
        assert_eq!(resolved.notes.len(), 1, "{key} = {value}");
        let view = resolved.combat();
        let got = [
            view.step_ms,
            view.action_cap,
            u16::from(view.pause_below_hp_percent),
        ][field];
        let default = [
            STEP_MS_DEFAULT,
            ACTION_CAP_DEFAULT,
            u16::from(PAUSE_BELOW_HP_DEFAULT),
        ];
        assert_eq!(got, default[field], "{key} = {value}");
    }
    // The bounds themselves are kept.
    let view = shipped(None, &[set(STEP_MS_KEY, "2000"), set(ACTION_CAP_KEY, "20")]).combat();
    assert_eq!((view.step_ms, view.action_cap), (2000, 20));
    let view = shipped(None, &[set(PAUSE_BELOW_HP_KEY, "100")]).combat();
    assert_eq!(view.pause_below_hp_percent, 100);
}
