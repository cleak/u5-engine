//! The M7 gate in the terminal and the settings file (design 6.4.2, 6.4.5;
//! plan M7-07): with auto-combat off or locked no `[combat]` section is
//! written to a file that had none, and no paired scenario, QA scenario or
//! route-smoke script uses `@auto`.

use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{CliSetting, LockReason, LockState};
use u5_tui::enhancement_settings::resolve_launch;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "u5-auto-combat-gate-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(feature = "visual")]
#[test]
fn off_or_locked_no_combat_section_is_written() {
    use u5_bevy_setup::{RowValue, SettingsChanges};
    use u5_tui::save_wizard_settings;

    let dir = scratch("config");
    let config = dir.join("config.toml");
    let original = "game_dir = \"/g\"\n\n[display]\nvsync = false\n";
    fs::write(&config, original).unwrap();
    let cli = [CliSetting::Enable("combat.auto".to_string())];

    // Locked: the file is not even read, auto is off, nothing is written.
    let locked = resolve_launch(Some(&config), &cli, LockState::Locked(LockReason::CliFlag));
    assert!(!locked.resolved.combat().auto);
    assert_eq!(fs::read_to_string(&config).unwrap(), original);

    // Off: launching, and saving another setting from the wizard, leave
    // the file without a `[combat]` section.
    let off = resolve_launch(Some(&config), &[], LockState::Off);
    assert!(!off.resolved.combat().auto);
    save_wizard_settings(
        &config,
        &SettingsChanges {
            preset: None,
            values: vec![("display.vsync".to_string(), RowValue::Switch(true))],
        },
    )
    .unwrap();
    let text = fs::read_to_string(&config).unwrap();
    assert!(text.contains("vsync = true"), "{text}");
    assert!(
        !text.contains("[combat]") && !text.contains("combat."),
        "{text}"
    );

    // Auto on for one launch from the command line writes nothing either.
    let on = resolve_launch(Some(&config), &cli, LockState::Off);
    assert!(on.resolved.combat().auto);
    assert_eq!(fs::read_to_string(&config).unwrap(), text);
    let _ = fs::remove_dir_all(&dir);
}

/// Every file of `dir`, recursively.
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn no_paired_qa_or_route_smoke_script_uses_auto() {
    // 6.4.5: the parity runs never type the control line, so they run the
    // same with the feature on or off.
    let root = workspace();
    let mut checked = Vec::new();
    files(&root.join("qa/paired"), &mut checked);
    checked.push(root.join("qa/scenarios.tsv"));
    checked.push(root.join("crates/u5-tui/src/route_smoke.rs"));
    assert!(checked.len() > 10, "{checked:?}");
    for path in checked {
        let bytes = fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("@auto"), "{} uses @auto", path.display());
    }
}
