//! Chapter 7's `[cheats]` rows and their typed view (design 7.2.9, 7.3.2,
//! 7.4.1, 7.4.5; build plan M8-01). Every row is hidden in this build
//! (U13), so the resolver tests use the catalogue's test override.

use super::*;
use crate::enhancements::catalogue::{
    ApplyAt, CHEATS_P0, CHEATS_P1, CHEATS_P2, CHEATS_P3, CHEATS_P4, CHEATS_P5, CatalogueRow,
    EffectClass, PRESETS, Phase, Preset, TOGGLES, Tag, Tier, ToggleSpec, help_line,
};
use crate::enhancements::resolve::{ResolveInput, resolve, resolve_as_shipped};
use crate::enhancements::source::{CliSetting, HostCapabilities, LockReason};

const ALL_CHEATS: &[Phase] = &[
    CHEATS_P0, CHEATS_P1, CHEATS_P2, CHEATS_P3, CHEATS_P4, CHEATS_P5,
];

/// Design 7.2.9's keys, in its order, each with the phase of 7.5 that
/// ships it.
const DESIGN_KEYS: &[(&str, Phase)] = &[
    ("cheats.enabled", CHEATS_P1),
    ("cheats.heal_all", CHEATS_P2),
    ("cheats.cure_poison", CHEATS_P2),
    ("cheats.resurrect_all", CHEATS_P2),
    ("cheats.max_gold", CHEATS_P2),
    ("cheats.max_food", CHEATS_P2),
    ("cheats.refill_arrows", CHEATS_P2),
    ("cheats.refill_quarrels", CHEATS_P2),
    ("cheats.max_reagents", CHEATS_P2),
    ("cheats.repair_hull", CHEATS_P2),
    ("cheats.lock_hp", CHEATS_P2),
    ("cheats.lock_mp", CHEATS_P2),
    ("cheats.lock_food", CHEATS_P2),
    ("cheats.lock_gold", CHEATS_P2),
    ("cheats.lock_keys", CHEATS_P2),
    ("cheats.lock_gems", CHEATS_P2),
    ("cheats.lock_torches", CHEATS_P2),
    ("cheats.lock_arrows", CHEATS_P2),
    ("cheats.lock_reagents", CHEATS_P2),
    ("cheats.remember_locks", CHEATS_P2),
    ("cheats.stat_edit", CHEATS_P3),
    ("cheats.teleport", CHEATS_P4),
    ("cheats.state_watch", CHEATS_P5),
];

fn enable(key: &str) -> CliSetting {
    CliSetting::Enable(key.to_string())
}

fn input(cli: &[CliSetting], lock: LockState) -> ResolveInput<'_> {
    ResolveInput {
        stored: None,
        cli,
        lock,
        caps: HostCapabilities::ALL,
    }
}

/// Resolves as if every chapter 7 phase had shipped.
fn shipped(keys: &[&str]) -> ResolvedEnhancements {
    let cli: Vec<CliSetting> = keys.iter().map(|key| enable(key)).collect();
    resolve_as_shipped(input(&cli, LockState::Off), ALL_CHEATS)
}

fn every_key() -> Vec<&'static str> {
    DESIGN_KEYS.iter().map(|(key, _)| *key).collect()
}

#[test]
fn the_rows_match_the_design() {
    let rows: Vec<&ToggleSpec> = TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("cheats."))
        .collect();
    assert_eq!(
        rows.iter().map(|spec| spec.key).collect::<Vec<_>>(),
        every_key(),
        "7.2.9's keys, in its order"
    );
    for (spec, (_, phase)) in rows.iter().zip(DESIGN_KEYS) {
        assert_eq!(spec.ships, *phase, "{}", spec.key);
        assert!(spec.is_shipped(), "{} ships with M8-06", spec.key);
        assert!(spec.cli_aliases.is_empty(), "{} has no alias", spec.key);
        assert!(!spec.driven && spec.needs.is_empty(), "{}", spec.key);
        assert!(
            help_line(spec.key).is_some(),
            "{} has a help line",
            spec.key
        );
        match spec.key {
            "cheats.state_watch" => {
                // Class (a), next frame, never gated and in no preset (U7).
                assert_eq!(spec.class, EffectClass::Presentation);
                assert_eq!(spec.apply, ApplyAt::Frame);
                assert_ne!(spec.tier, Tier::Feature);
            }
            "cheats.remember_locks" => {
                assert_eq!(spec.class, EffectClass::StateAltering);
                assert_eq!(spec.apply, ApplyAt::NextLaunch);
                assert_eq!(spec.tier, Tier::Cheat);
            }
            _ => {
                assert_eq!(spec.class, EffectClass::StateAltering, "{}", spec.key);
                assert_eq!(spec.apply, ApplyAt::IdlePrompt, "{}", spec.key);
                assert_eq!(spec.tier, Tier::Cheat, "{}", spec.key);
            }
        }
    }
    assert!(CHEATS_P0.is_shipped(), "the runtime rules have shipped");
    for phase in ALL_CHEATS {
        assert!(phase.is_shipped(), "{phase:?} ships with M8-06");
    }
}

/// 7.2.7 and U17: the Teleport row names its spoiler.
#[test]
fn the_teleport_row_carries_its_spoiler_label() {
    let Some(CatalogueRow::Toggle(spec)) = CatalogueRow::find("cheats.teleport") else {
        panic!("cheats.teleport is a toggle");
    };
    assert_eq!(
        spec.label,
        "Teleport (Spoiler: lists every place in the game)"
    );
    assert!(spec.tags.contains(&Tag::SpoilerRisk));
    // No other cheat row claims to be a spoiler.
    for spec in TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("cheats.") && spec.key != "cheats.teleport")
    {
        assert!(!spec.label.contains("Spoiler"), "{}", spec.key);
    }
}

#[test]
fn no_preset_names_a_cheat() {
    for preset in PRESETS {
        for entry in preset.entries() {
            assert!(
                !entry.key.starts_with("cheats."),
                "{preset:?}: {}",
                entry.key
            );
        }
        // Even with every cheat phase shipped, no preset turns one on.
        let resolved = resolve_as_shipped(
            input(&[CliSetting::Preset(*preset)], LockState::Off),
            ALL_CHEATS,
        );
        assert_eq!(resolved.cheats(), CheatToggles::default(), "{preset:?}");
    }
    assert!(Preset::Classic.entries().is_empty());
}

/// M8-06 shipped every phase: the plain resolver, as a host calls it,
/// gives what the shipped-phase fixture gives.
#[test]
fn shipped_rows_resolve_as_the_host_resolves_them() {
    let cli: Vec<CliSetting> = every_key().into_iter().map(enable).collect();
    let resolved = resolve(input(&cli, LockState::Off));
    assert!(resolved.notes.is_empty(), "{:?}", resolved.notes);
    assert_eq!(resolved.cheats(), shipped(&every_key()).cheats());
    assert!(resolved.cheats().any_on());
}

#[test]
fn every_key_on_gives_every_field() {
    let view = shipped(&every_key());
    let locks = LockPermissions {
        health: true,
        mana: true,
        food: true,
        gold: true,
        keys: true,
        gems: true,
        torches: true,
        arrows: true,
        reagents: true,
    };
    assert_eq!(
        view.cheats(),
        CheatToggles {
            enabled: true,
            heal_all: true,
            cure_poison: true,
            resurrect_all: true,
            max_gold: true,
            max_food: true,
            refill_arrows: true,
            refill_quarrels: true,
            max_reagents: true,
            repair_hull: true,
            locks,
            remember_locks: true,
            stat_edit: true,
            teleport: true,
            state_watch: true,
        }
    );
    assert!(view.cheats().any_state_altering());
    assert!(view.cheats().any_on());
}

/// 7.4.5: the parity lock gives every field false, whatever the CLI says.
#[test]
fn the_view_is_all_false_under_the_lock() {
    let cli: Vec<CliSetting> = every_key().into_iter().map(enable).collect();
    let locked = resolve_as_shipped(
        input(&cli, LockState::Locked(LockReason::CliFlag)),
        ALL_CHEATS,
    );
    assert_eq!(locked.cheats(), CheatToggles::default());
    // Even a hand-built value with bits on and the lock set reads all off.
    let mut forged = shipped(&every_key());
    forged.lock = LockState::Locked(LockReason::EnvVar);
    assert_eq!(forged.cheats(), CheatToggles::default());
}

/// D7.1, 7.4.1: with `enabled` off every class (c) key is off; the watch
/// keeps its own key.
#[test]
fn the_view_is_all_false_without_the_switch() {
    let keys: Vec<&str> = every_key()
        .into_iter()
        .filter(|key| *key != "cheats.enabled" && *key != "cheats.state_watch")
        .collect();
    let view = shipped(&keys).cheats();
    assert_eq!(view, CheatToggles::default());
    assert!(!view.any_on());

    let mut with_watch = keys.clone();
    with_watch.push("cheats.state_watch");
    let view = shipped(&with_watch).cheats();
    assert_eq!(
        view,
        CheatToggles {
            state_watch: true,
            ..CheatToggles::default()
        }
    );
    assert!(view.any_on() && !view.any_state_altering());

    // The view masks too: bits forced on without the switch read off.
    let mut forged = shipped(&[]);
    for spec in TOGGLES
        .iter()
        .filter(|spec| spec.key.starts_with("cheats."))
    {
        if spec.key != "cheats.enabled" && spec.key != "cheats.state_watch" {
            forged.toggles.set(spec.toggle, true);
        }
    }
    assert_eq!(forged.cheats(), CheatToggles::default());
}

/// 7.3.2, D-M8.5: the two sub-options alone turn nothing on.
#[test]
fn sub_options_alone_turn_nothing_on() {
    for key in ["cheats.refill_quarrels", "cheats.remember_locks"] {
        let view = shipped(&["cheats.enabled", key]).cheats();
        assert!(view.enabled, "{key}");
        assert!(!view.any_state_altering(), "{key}");
        assert!(!view.any_on(), "{key}");
    }
    let view = shipped(&["cheats.enabled"]).cheats();
    assert!(!view.any_on(), "the switch alone enables nothing");
}

#[test]
fn each_cheat_alone_turns_the_page_on() {
    for key in every_key() {
        if matches!(
            key,
            "cheats.enabled" | "cheats.refill_quarrels" | "cheats.remember_locks"
        ) {
            continue;
        }
        let view = shipped(&["cheats.enabled", key]).cheats();
        assert!(view.any_on(), "{key}");
        assert_eq!(
            view.any_state_altering(),
            key != "cheats.state_watch",
            "{key}"
        );
    }
}

#[test]
fn allows_needs_the_switch() {
    let off = CheatToggles::default();
    assert!(!off.allows(true));
    let on = CheatToggles {
        enabled: true,
        ..off
    };
    assert!(on.allows(true));
    assert!(!on.allows(false));
    assert!(!LockPermissions::default().any());
}
