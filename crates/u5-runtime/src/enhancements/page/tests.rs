//! The shared rows, their notes and the command-line marks (design 3.2.7,
//! 3.3.6; build plan M4-06, D-M4.4, D-M4.5). The page model's own tests are
//! the wizard's (`u5-bevy-setup/tests/wizard.rs`), which run it unchanged.

use super::*;
use crate::enhancements::{PRESET_KEY, SettingNote};

fn resolved(cli: &[CliSetting]) -> ResolvedEnhancements {
    resolve(ResolveInput {
        stored: None,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    })
}

fn enable(key: &str) -> CliSetting {
    CliSetting::Enable(key.to_string())
}

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn toggles(on: &[Toggle]) -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    for toggle in on {
        toggles.set(*toggle, true);
    }
    toggles
}

/// The menu overlay alone: a 4:3 launch with the menu.
fn menu_only() -> FeatureToggles {
    toggles(&[Toggle::MenuOverlay])
}

#[test]
fn the_page_rows_are_the_generator_rows_with_the_presets() {
    let launch = resolved(&[CliSetting::Preset(Preset::Recommended)]);
    let page = settings_page_rows(&launch, true, vec!["a note".to_string()]);
    assert_eq!(page.rows, settings_rows(&launch));
    assert_eq!(page.presets.len(), PRESETS.len());
    for (choice, preset) in page.presets.iter().zip(PRESETS) {
        assert_eq!(*choice, preset_choice(*preset, &page.rows));
    }
    assert_eq!(PRESETS[page.preset], Preset::Recommended);
    assert!(page.locked);
    assert_eq!(page.notes, ["a note"]);
}

#[test]
fn a_preset_choice_names_only_governed_rows_on_the_page() {
    let rows = settings_rows(&ResolvedEnhancements::classic());
    let everything = preset_choice(Preset::Everything, &rows);
    assert_eq!(everything.key, "everything");
    for (key, _) in &everything.values {
        assert!(rows.iter().any(|row| row.key == *key), "{key} has a row");
        assert!(
            PRESETS
                .iter()
                .any(|preset| preset.shipped_entries().any(|(entry, _)| entry.key == key)),
            "{key} is named by a preset"
        );
    }
    // Classic gives every governed row its default.
    let classic = preset_choice(Preset::Classic, &rows);
    for (key, value) in &classic.values {
        let row = rows.iter().find(|row| row.key == *key).unwrap();
        assert_eq!(*value, row.default, "{key}");
    }
}

#[test]
fn row_values_store_as_the_catalogue_types_them() {
    assert_eq!(
        RowValue::Switch(true).to_setting("display.sprite_composite"),
        SettingValue::Bool(true)
    );
    assert_eq!(
        RowValue::Choice("70".into()).to_setting("display.brightness"),
        SettingValue::Int(70)
    );
    assert_eq!(
        RowValue::Choice("sharp".into()).to_setting("display.scale_mode"),
        SettingValue::Str("sharp".into())
    );
    for value in [
        SettingValue::Bool(false),
        SettingValue::Int(85),
        SettingValue::Str("map".into()),
    ] {
        let row = RowValue::of(&value);
        let key = match value {
            SettingValue::Bool(_) => "display.composite_halo",
            SettingValue::Int(_) => "display.brightness",
            _ => "display.brightness_scope",
        };
        assert_eq!(row.to_setting(key), value, "{key} round trip");
    }
}

#[test]
fn the_command_line_marks_the_keys_it_set() {
    let launch = resolved(&[
        enable("display.sprite_composite"),
        set("display.brightness", "70"),
    ]);
    assert_eq!(
        launch.cli_keys,
        ["display.sprite_composite", "display.brightness"]
    );
    assert!(launch.set_by_cli("display.brightness"));
    assert!(!launch.set_by_cli("display.composite_halo"));

    // Twice is marked once.
    let twice = resolved(&[
        enable("display.sprite_composite"),
        CliSetting::Disable("display.sprite_composite".into()),
    ]);
    assert_eq!(twice.cli_keys, ["display.sprite_composite"]);
}

#[test]
fn a_refused_value_or_unknown_key_marks_nothing() {
    let launch = resolved(&[
        set("display.brightness", "7"),
        set("display.no_such_key", "1"),
        enable("display.brightness_scope"),
    ]);
    assert!(launch.cli_keys.is_empty(), "{:?}", launch.cli_keys);
    assert!(!launch.notes.is_empty());
}

#[test]
fn a_preset_marks_the_preset_row_and_clears_what_it_puts_back() {
    let launch = resolved(&[
        enable("display.sprite_composite"),
        set("display.brightness", "70"),
        CliSetting::Preset(Preset::Classic),
    ]);
    // `display.sprite_composite` is a preset row; `display.brightness` is
    // a preference no preset sets, so its mark stays.
    assert_eq!(launch.cli_keys, ["display.brightness", PRESET_KEY]);
    let by_set = resolved(&[set(PRESET_KEY, "everything")]);
    assert_eq!(by_set.cli_keys, [PRESET_KEY]);
}

#[test]
fn the_stored_settings_and_the_lock_mark_nothing() {
    assert!(ResolvedEnhancements::classic().cli_keys.is_empty());
    let locked = resolve(ResolveInput {
        stored: None,
        cli: &[enable("display.sprite_composite")],
        lock: LockState::Locked(crate::enhancements::LockReason::CliFlag),
        caps: HostCapabilities::ALL,
    });
    assert!(locked.cli_keys.is_empty());
    assert!(
        locked
            .notes
            .iter()
            .any(|note| matches!(note, SettingNote::LockIgnored { .. }))
    );
}

#[test]
fn row_notes_carry_the_mark_and_the_apply_point() {
    let launch = resolved(&[enable("display.sprite_composite")]);
    let rows = settings_rows(&launch);
    let notes = row_notes(&rows, &launch, launch.toggles);
    let note = |key: &str| notes[rows.iter().position(|row| row.key == key).unwrap()];
    assert!(note("display.sprite_composite").by_cli);
    assert_eq!(note("display.sprite_composite").apply, ApplyAt::Frame);
    assert!(!note("display.composite_halo").by_cli);
    assert_eq!(note("display.composite_halo").apply, ApplyAt::Frame);
    // The grow-left layout was off at launch.
    assert_eq!(note("display.grow_left").apply, ApplyAt::NextLaunch);
}

#[test]
fn grow_left_and_scaling_from_a_faithful_launch_wait_for_the_next_launch() {
    let launch = menu_only();
    for key in [
        "display.grow_left",
        "display.crawl",
        "display.scale_mode",
        "display.pixel_aspect",
    ] {
        assert_eq!(effective_apply(key, launch), ApplyAt::NextLaunch, "{key}");
    }
    // A grow-left launch installed both.
    let grow = toggles(&[Toggle::MenuOverlay, Toggle::DisplayGrowLeft]);
    for key in ["display.grow_left", "display.crawl", "display.scale_mode"] {
        assert_eq!(effective_apply(key, grow), catalogue_apply(key), "{key}");
    }
    // A scaled 4:3 launch installed the scaling, not the canvas.
    let scaled = toggles(&[Toggle::MenuOverlay, Toggle::DisplayScaleMode]);
    assert_eq!(
        effective_apply("display.pixel_aspect", scaled),
        ApplyAt::Frame
    );
    assert_eq!(
        effective_apply("display.grow_left", scaled),
        ApplyAt::NextLaunch
    );
}

#[test]
fn the_dormant_features_apply_in_play_only_with_the_menu() {
    let keys = [
        "display.pillar_fill",
        "display.brightness",
        "display.brightness_scope",
        "display.sprite_composite",
        "display.composite_halo",
        "gem_map.enhanced",
    ];
    for key in keys {
        assert_eq!(
            effective_apply(key, menu_only()),
            catalogue_apply(key),
            "{key} with the menu"
        );
        assert_eq!(
            effective_apply(key, FeatureToggles::CLASSIC),
            ApplyAt::NextLaunch,
            "{key} without it"
        );
    }
    assert_eq!(
        DORMANT_WITH_MENU,
        [
            Toggle::DisplayPillarFill,
            Toggle::DisplayBrightness,
            Toggle::DisplaySpriteComposite,
            Toggle::GemMapEnhanced,
        ],
        "D-M4.4's list; the window wish and audio join in M4-08"
    );
    for toggle in DORMANT_WITH_MENU {
        assert!(dormant_install(menu_only(), *toggle));
        assert!(!dormant_install(FeatureToggles::CLASSIC, *toggle));
    }
    assert!(!dormant_install(menu_only(), Toggle::DisplayGrowLeft));
}

#[test]
fn keys_read_at_launch_and_next_launch_rows_never_apply_in_play() {
    let all = toggles(
        &crate::enhancements::TOGGLES
            .iter()
            .map(|spec| spec.toggle)
            .collect::<Vec<_>>(),
    );
    for key in [
        "display.map_brightness",
        "display.panel_priority",
        "gem_map.labels",
        "gem_map.level_paging",
        "menu.overlay",
        "menu.title_layer",
        "display.raster",
    ] {
        assert_eq!(effective_apply(key, all), ApplyAt::NextLaunch, "{key}");
    }
    assert_eq!(effective_apply("no.such_key", all), ApplyAt::NextLaunch);
    // The foundation's preset has no feature: its catalogue point holds.
    assert_eq!(
        effective_apply(PRESET_KEY, menu_only()),
        ApplyAt::IdlePrompt
    );
}

#[test]
fn the_window_keys_need_a_window_key_at_launch_until_m4_08() {
    assert_eq!(
        effective_apply("display.vsync", menu_only()),
        ApplyAt::NextLaunch
    );
    let window = toggles(&[Toggle::MenuOverlay, Toggle::DisplayFullscreen]);
    assert_eq!(effective_apply("display.vsync", window), ApplyAt::Frame);
    assert_eq!(
        effective_apply("display.fullscreen_key", window),
        ApplyAt::NextLaunch,
        "F11 needs its own binding at launch"
    );
}

#[test]
fn a_switched_off_feature_installed_at_launch_applies_at_its_point() {
    let grow = toggles(&[Toggle::DisplayGrowLeft]);
    assert_eq!(
        effective_apply("display.grow_left", grow),
        catalogue_apply("display.grow_left")
    );
}

#[test]
fn a_valued_change_sets_the_value_and_its_switch() {
    let mut live = ResolvedEnhancements::classic();
    assert!(!live.toggles.on(Toggle::DisplayBrightness));
    assert_eq!(
        apply_value(&mut live, "display.brightness", SettingValue::Int(70)),
        Some(SettingValue::Int(70))
    );
    assert_eq!(live.values.int("display.brightness"), Some(70));
    assert!(live.toggles.on(Toggle::DisplayBrightness));
    assert_eq!(live.display_light().brightness, 70);
    apply_value(&mut live, "display.brightness", SettingValue::Int(100));
    assert!(!live.toggles.on(Toggle::DisplayBrightness), "100 is off");

    // A sub-option sets no switch.
    apply_value(
        &mut live,
        "display.composite_halo",
        SettingValue::Bool(false),
    );
    assert_eq!(live.values.bool("display.composite_halo"), Some(false));
    assert!(live.toggles.is_classic());
}

#[test]
fn a_bad_valued_change_changes_nothing() {
    let mut live = ResolvedEnhancements::classic();
    for (key, value) in [
        ("display.brightness", SettingValue::Int(7)),
        ("display.brightness", SettingValue::Str("bright".into())),
        ("display.scale_mode", SettingValue::Str("blurry".into())),
        ("display.sprite_composite", SettingValue::Bool(true)),
        ("no.such_key", SettingValue::Bool(true)),
    ] {
        assert_eq!(apply_value(&mut live, key, value), None, "{key}");
    }
    assert_eq!(live, ResolvedEnhancements::classic());
}
