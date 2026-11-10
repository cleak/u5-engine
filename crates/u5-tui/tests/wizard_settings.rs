//! The desktop host's side of the wizard's Enhancements page (Phase B design
//! 1.2.2, package M0-08): the rows and presets `DesktopWizardHost` builds
//! from the catalogue and `config.toml`, and the save of changed keys. The
//! page itself is tested in `u5-bevy-setup`.
#![cfg(feature = "visual")]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use u5_bevy_setup::{RowValue, SettingsChanges, WizardHost, WizardModel};
use u5_host_desktop::{DesktopHost, HostEnv, Platform};
use u5_runtime::enhancements::{Preset, TOGGLES};
use u5_tui::*;

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-wizard-settings-{label}-{}-{unique}",
            std::process::id()
        ));
        for sub in ["home", "tmp", "cwd"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        Self(dir)
    }

    /// A desktop host whose `config.toml` is inside the scratch folder.
    fn host(&self) -> DesktopHost {
        DesktopHost::new(HostEnv {
            platform: Platform::Unix,
            vars: HashMap::from([("U5_ENGINE_CONFIG".to_string(), self.config().into())]),
            home: Some(self.0.join("home")),
            temp_dir: self.0.join("tmp"),
            current_dir: Some(self.0.join("cwd")),
            exe_dir: None,
        })
    }

    fn config(&self) -> PathBuf {
        self.0.join("config.toml")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rows_and_presets_come_from_the_catalogue_and_the_file() {
    let scratch = Scratch::new("rows");
    fs::write(
        scratch.config(),
        "game_dir = \"/g\"\n[enhancements]\npreset = \"recommended\"\n",
    )
    .unwrap();
    let settings = wizard_settings(&scratch.config(), false);
    // The three presets in order, starting from the stored one.
    let presets: Vec<(&str, &str)> = settings
        .presets
        .iter()
        .map(|choice| (choice.key.as_str(), choice.label.as_str()))
        .collect();
    assert_eq!(
        presets,
        [
            ("classic", "Classic"),
            ("recommended", "Recommended"),
            ("everything", "Everything"),
        ]
    );
    assert_eq!(settings.preset, 1);
    assert!(!settings.locked);
    assert!(settings.notes.is_empty());
    // Hidden rows are never listed (U13), and neither are the
    // foundation's own keys: the file format, the preset (the page's
    // preset row) and the `[keys]` chords. In this build the shipped rows
    // with a page row are chapter 2 P1's: scaling, fullscreen and vsync
    // (`display.raster` has none while both its values draw the same).
    for row in &settings.rows {
        assert!(!row.key.starts_with("keys."), "{}", row.key);
        assert_ne!(row.key, "enhancements.schema");
        assert_ne!(row.key, "enhancements.preset");
    }
    assert!(
        TOGGLES
            .iter()
            .filter(|spec| spec.is_shipped() && !spec.driven)
            .all(|spec| spec.key.starts_with("display."))
    );
    let keys: Vec<&str> = settings.rows.iter().map(|row| row.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "display.fullscreen",
            "display.fullscreen_key",
            "display.scale_mode",
            "display.pixel_aspect",
            "display.vsync",
        ]
    );
    // Recommended names no shipped row; Everything sets sharp scaling and
    // the F11 key (2.2.1).
    let values = |key: &str| {
        let choice = settings
            .presets
            .iter()
            .find(|choice| choice.key == key)
            .unwrap();
        let mut values: Vec<String> = choice
            .values
            .iter()
            .map(|value| format!("{value:?}"))
            .collect();
        values.sort();
        values
    };
    // Each preset sets the keys any preset names, at its own values.
    let at_default = [
        "(\"display.fullscreen_key\", Switch(false))",
        "(\"display.scale_mode\", Choice(\"fit\"))",
    ];
    assert_eq!(values("classic"), at_default);
    assert_eq!(values("recommended"), at_default);
    assert_eq!(
        values("everything"),
        [
            "(\"display.fullscreen_key\", Switch(true))",
            "(\"display.scale_mode\", Choice(\"sharp\"))",
        ]
    );

    // The lock reaches the page.
    assert!(wizard_settings(&scratch.config(), true).locked);
    // No file: Classic.
    let empty = Scratch::new("empty");
    assert_eq!(wizard_settings(&empty.config(), false).preset, 0);
    assert!(!empty.config().exists());
}

#[test]
fn a_damaged_file_says_it_will_be_kept() {
    let scratch = Scratch::new("damaged");
    fs::write(scratch.config(), "this is [ not toml\n").unwrap();
    let settings = wizard_settings(&scratch.config(), false);
    assert_eq!(settings.preset, 0);
    assert!(
        settings
            .notes
            .iter()
            .any(|note| note.contains("is not valid TOML")),
        "{:?}",
        settings.notes
    );
    assert!(
        settings
            .notes
            .iter()
            .any(|note| note.contains("config.toml.bak")),
        "{:?}",
        settings.notes
    );
    save_wizard_settings(
        &scratch.config(),
        &SettingsChanges {
            preset: Some("recommended".to_string()),
            values: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(scratch.0.join("config.toml.bak")).unwrap(),
        "this is [ not toml\n"
    );
}

#[test]
fn save_writes_changed_keys_and_keeps_the_rest() {
    let scratch = Scratch::new("save");
    let original = "# mine\ngame_dir = \"/g\"\n\n[audio]\nvolume = 50 # loud\n";
    fs::write(scratch.config(), original).unwrap();
    // Nothing changed: the file is not touched.
    save_wizard_settings(&scratch.config(), &SettingsChanges::default()).unwrap();
    assert_eq!(fs::read_to_string(scratch.config()).unwrap(), original);

    save_wizard_settings(
        &scratch.config(),
        &SettingsChanges {
            preset: Some("everything".to_string()),
            values: vec![
                // A number setting's choice is stored as a number.
                (
                    "enhancements.schema".to_string(),
                    RowValue::Choice("1".into()),
                ),
                ("cheats.enabled".to_string(), RowValue::Switch(true)),
            ],
        },
    )
    .unwrap();
    let text = fs::read_to_string(scratch.config()).unwrap();
    assert!(text.starts_with("# mine\ngame_dir = \"/g\"\n"), "{text}");
    assert!(text.contains("volume = 50 # loud"), "{text}");
    let doc: toml_edit::DocumentMut = text.parse().unwrap();
    assert_eq!(
        doc["enhancements"]["preset"].as_str(),
        Some("everything"),
        "{text}"
    );
    assert_eq!(
        doc["enhancements"]["schema"].as_integer(),
        Some(1),
        "{text}"
    );
    assert_eq!(doc["cheats"]["enabled"].as_bool(), Some(true), "{text}");
    // And the page reads it back.
    assert_eq!(
        wizard_settings(&scratch.config(), false).preset,
        Preset::Everything as usize
    );
}

#[test]
fn desktop_host_feeds_the_settings_only_wizard() {
    let scratch = Scratch::new("host");
    let host = DesktopWizardHost {
        host: scratch.host(),
        parity_locked: false,
    };
    let rows = host.settings_rows().expect("the desktop host has settings");
    assert_eq!(rows.preset, 0);
    let mut model = WizardModel::settings_only(&host);
    assert!(model.outcome.is_none());
    // Choose Recommended and save: only the preset is written.
    assert!(model.apply(u5_bevy_setup::Action::SelectPreset(1), &host));
    assert!(model.apply(u5_bevy_setup::Action::SaveSettings, &host));
    assert_eq!(
        model.outcome,
        Some(u5_bevy_setup::WizardOutcome::SettingsClosed { saved: true })
    );
    let text = fs::read_to_string(scratch.config()).unwrap();
    assert_eq!(text, "[enhancements]\npreset = \"recommended\"\n");
    assert_eq!(host.settings_rows().unwrap().preset, 1);
}
