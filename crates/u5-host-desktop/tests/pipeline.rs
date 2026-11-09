//! The desktop resolution pipeline against temp HOME folders.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use u5_gamedata::MANIFEST;
use u5_host_desktop::*;

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-host-pipeline-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(dir.join("home")).unwrap();
        Self(dir)
    }
    fn home(&self) -> PathBuf {
        self.0.join("home")
    }
    /// A Unix-style host whose HOME, cwd and exe folder are inside the
    /// scratch folder, with no environment variables at all.
    fn host(&self, vars: &[(&str, &Path)]) -> DesktopHost {
        let home = self.home();
        fs::create_dir_all(home.join("cwd")).unwrap();
        DesktopHost::new(HostEnv {
            platform: Platform::Unix,
            vars: vars
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.as_os_str().to_owned()))
                .collect::<HashMap<_, _>>(),
            home: Some(home.clone()),
            temp_dir: self.0.join("tmp"),
            current_dir: Some(home.join("cwd")),
            exe_dir: Some(home.join("bin")),
        })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn populate(dir: &Path, rename: impl Fn(&str) -> String) {
    fs::create_dir_all(dir).unwrap();
    for file in MANIFEST {
        fs::write(
            dir.join(rename(file.name)),
            format!("synthetic {}", file.name),
        )
        .unwrap();
    }
}

fn ready(resolution: Resolution) -> ReadyGameData {
    match resolution {
        Resolution::Ready(ready) => ready,
        Resolution::NeedsWizard(need) => panic!("expected ready, needs wizard: {need:#?}"),
    }
}

fn wizard(resolution: Resolution) -> WizardNeed {
    match resolution {
        Resolution::NeedsWizard(need) => need,
        Resolution::Ready(ready) => panic!("expected wizard, got ready: {ready:#?}"),
    }
}

#[test]
fn explicit_canonical_folder_is_used_exactly_as_given() {
    let scratch = Scratch::new("explicit");
    let game = scratch.0.join("game");
    populate(&game, str::to_string);
    let host = scratch.host(&[]);
    let result = ready(host.resolve(&ResolveRequest {
        explicit: Some(game.clone()),
        force_setup: false,
    }));
    assert_eq!(result.data_dir, game);
    assert_eq!(result.origin, Origin::CommandLine);
    assert!(result.notes.is_empty());
    // Nothing was written: no config, no managed copy.
    assert!(!host.paths.config_file.exists());
    assert!(!host.paths.game_data_root().exists());
}

#[test]
fn explicit_lowercase_folder_gets_a_normalised_copy() {
    let scratch = Scratch::new("explicit-lower");
    let game = scratch.0.join("game");
    populate(&game, |name| name.to_ascii_lowercase());
    let host = scratch.host(&[]);
    let result = ready(host.resolve(&ResolveRequest {
        explicit: Some(game.clone()),
        force_setup: false,
    }));
    assert!(result.data_dir.starts_with(host.paths.game_data_root()));
    assert!(result.data_dir.join("BRIT.DAT").is_file());
    assert!(result.notes[0].contains("upper case"), "{:?}", result.notes);
    // Not remembered: a one-off command-line folder never edits settings.
    assert!(!host.paths.config_file.exists());
}

#[test]
fn explicit_incomplete_folder_is_passed_through_with_its_report() {
    let scratch = Scratch::new("explicit-partial");
    let game = scratch.0.join("fixture");
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("TILES.16"), b"x").unwrap();
    let host = scratch.host(&[]);
    let result = ready(host.resolve(&ResolveRequest {
        explicit: Some(game.clone()),
        force_setup: false,
    }));
    assert_eq!(result.data_dir, game);
    let report = result.incomplete.expect("incomplete report");
    assert_eq!(report.missing_required().len(), 62);
}

#[test]
fn environment_variable_is_second() {
    let scratch = Scratch::new("env");
    let game = scratch.0.join("env-game");
    populate(&game, str::to_string);
    let host = scratch.host(&[(GAME_DIR_ENV, &game)]);
    let result = ready(host.resolve(&ResolveRequest::default()));
    assert_eq!(result.data_dir, game);
    assert_eq!(result.origin, Origin::Environment);
}

#[test]
fn discovery_finds_a_nested_gog_layout_then_config_remembers_it() {
    let scratch = Scratch::new("discover");
    // GOG-style layout with the DOS files nested a level down.
    let install = scratch.home().join("GOG Games").join("Ultima 4+5+6");
    fs::create_dir_all(install.join("Ultima 4")).unwrap();
    let data = install.join("Ultima 5").join("data");
    populate(&data, str::to_string);
    let host = scratch.host(&[]);

    let first = ready(host.resolve(&ResolveRequest::default()));
    assert!(
        matches!(first.origin, Origin::Discovered { .. }),
        "{:?}",
        first.origin
    );
    assert_eq!(first.source_dir, data);
    assert!(first.data_dir.starts_with(host.paths.game_data_root()));
    assert!(first.notes.iter().any(|note| note.contains("--setup")));
    let config = host.load_config().unwrap();
    assert_eq!(
        config.game_dir.as_deref(),
        Some(data.canonicalize().unwrap().as_path())
    );
    assert_eq!(config.version.as_deref(), Some("unknown"));

    // Second launch: straight from the saved settings, same managed copy.
    let second = ready(host.resolve(&ResolveRequest::default()));
    assert_eq!(second.origin, Origin::SavedConfig);
    assert_eq!(
        second.data_dir.canonicalize().unwrap(),
        first.data_dir.canonicalize().unwrap()
    );

    // A save made in the managed copy survives re-running setup.
    fs::write(first.data_dir.join("SAVED.GAM"), b"progress").unwrap();
    host.commit(&install, Origin::Setup).unwrap();
    assert_eq!(
        fs::read(first.data_dir.join("SAVED.GAM")).unwrap(),
        b"progress"
    );
}

#[test]
fn stale_config_falls_back_to_discovery_then_wizard() {
    let scratch = Scratch::new("stale");
    let host = scratch.host(&[]);
    Config {
        game_dir: Some(scratch.0.join("gone")),
        data_dir: Some(scratch.0.join("also-gone")),
        version: Some("gog-1.0".into()),
    }
    .save(&host.paths.config_file)
    .unwrap();
    let need = wizard(host.resolve(&ResolveRequest::default()));
    assert!(
        need.notes
            .iter()
            .any(|note| note.contains("no longer has the game files"))
    );
    assert!(need.candidates.is_empty());
    assert!(need.reason.contains("No Ultima V game files"));
}

#[test]
fn damaged_config_is_ignored_with_a_note() {
    let scratch = Scratch::new("damaged");
    let host = scratch.host(&[]);
    fs::create_dir_all(host.paths.config_file.parent().unwrap()).unwrap();
    fs::write(&host.paths.config_file, "game_dir = [").unwrap();
    let need = wizard(host.resolve(&ResolveRequest::default()));
    assert!(
        need.notes[0].contains("Ignoring settings"),
        "{:?}",
        need.notes
    );
}

#[test]
fn partial_install_is_offered_to_the_wizard_but_not_used() {
    let scratch = Scratch::new("partial");
    let game = scratch.home().join("Games").join("ultima-v");
    populate(&game, str::to_string);
    fs::remove_file(game.join("TOWNE.DAT")).unwrap();
    let host = scratch.host(&[]);
    let need = wizard(host.resolve(&ResolveRequest::default()));
    assert_eq!(need.candidates.len(), 1);
    assert_eq!(
        need.candidates[0].report.missing_required(),
        vec!["TOWNE.DAT"]
    );
    assert!(need.reason.contains("none has every file"));
}

#[test]
fn force_setup_asks_even_when_config_is_good_and_setup_dir_commits() {
    let scratch = Scratch::new("force");
    let game = scratch.home().join("GOG Games").join("Ultima 5");
    populate(&game, str::to_string);
    let host = scratch.host(&[]);
    ready(host.resolve(&ResolveRequest::default()));
    let need = wizard(host.resolve(&ResolveRequest {
        explicit: None,
        force_setup: true,
    }));
    assert_eq!(need.candidates.len(), 1);
    assert!(need.candidates[0].report.is_usable());

    // `--setup <DIR>` sets that folder up without asking.
    let other = scratch.0.join("other");
    populate(&other, |name| name.to_ascii_lowercase());
    let result = ready(host.resolve(&ResolveRequest {
        explicit: Some(other.clone()),
        force_setup: true,
    }));
    assert_eq!(result.origin, Origin::CommandLine);
    assert_eq!(
        host.load_config().unwrap().game_dir.unwrap(),
        other.canonicalize().unwrap()
    );
}

#[test]
fn commit_of_a_bad_folder_explains_what_is_missing() {
    let scratch = Scratch::new("bad");
    let game = scratch.0.join("bad");
    populate(&game, str::to_string);
    fs::remove_file(game.join("SHOPPE.DAT")).unwrap();
    let host = scratch.host(&[]);
    let error = host.commit(&game, Origin::Setup).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("SHOPPE.DAT"), "{text}");
    assert!(
        text.contains("1 of the 63 files the game needs is missing"),
        "{text}"
    );
    assert!(!host.paths.config_file.exists());
}

#[test]
fn choosing_the_managed_copy_itself_reuses_it() {
    let scratch = Scratch::new("managed");
    let game = scratch.0.join("game");
    populate(&game, str::to_string);
    let host = scratch.host(&[]);
    let first = host.commit(&game, Origin::Setup).unwrap();
    let again = host.commit(&first.data_dir, Origin::Setup).unwrap();
    assert_eq!(
        again.data_dir.canonicalize().unwrap(),
        first.data_dir.canonicalize().unwrap()
    );
}

#[test]
fn commit_keeps_every_enhancement_section_and_changes_only_its_three_keys() {
    let scratch = Scratch::new("sections");
    let game = scratch.home().join("GOG Games").join("Ultima 5");
    populate(&game, str::to_string);
    let host = scratch.host(&[]);
    let sections = "\
# My own note.
game_dir = \"/old/place\" # chosen in 2025
data_dir = \"/old/copy\"
version = \"gog-1.0\"
future_key = [1, 2]

[enhancements]
preset = \"modern\"   # keep this

[display]
vsync = false

[cheats.engaged]
hp = true
";
    fs::create_dir_all(host.paths.config_file.parent().unwrap()).unwrap();
    fs::write(&host.paths.config_file, sections).unwrap();

    let result = host.commit(&game, Origin::Setup).unwrap();
    let config = host.load_config().unwrap();
    let text = fs::read_to_string(&host.paths.config_file).unwrap();
    let game_dir = config.game_dir.clone().unwrap();
    let data_dir = config.data_dir.clone().unwrap();
    assert_eq!(game_dir, game.canonicalize().unwrap());
    assert_eq!(
        data_dir.canonicalize().unwrap(),
        result.data_dir.canonicalize().unwrap()
    );
    let quote = |path: &Path| toml_edit::Value::from(path.to_str().unwrap()).to_string();
    let expected = sections
        .replace("\"/old/place\"", &quote(&game_dir))
        .replace("\"/old/copy\"", &quote(&data_dir))
        .replace("\"gog-1.0\"", &format!("\"{}\"", config.version.unwrap()));
    assert_eq!(text, expected);
    assert!(
        !host
            .paths
            .config_file
            .with_file_name("config.toml.bak")
            .exists()
    );
}

#[test]
fn a_damaged_config_is_backed_up_before_discovery_commits_over_it() {
    let scratch = Scratch::new("damaged-backup");
    let game = scratch.home().join("GOG Games").join("Ultima 5");
    populate(&game, str::to_string);
    let host = scratch.host(&[]);
    let damaged = "game_dir = [\n[display]\nvsync = false\n";
    fs::create_dir_all(host.paths.config_file.parent().unwrap()).unwrap();
    fs::write(&host.paths.config_file, damaged).unwrap();

    let ready = ready(host.resolve(&ResolveRequest::default()));
    assert!(
        ready
            .notes
            .iter()
            .any(|note| note.contains("Ignoring settings")),
        "{:?}",
        ready.notes
    );
    let backup = host.paths.config_file.with_file_name("config.toml.bak");
    assert_eq!(fs::read_to_string(backup).unwrap(), damaged);
    let text = fs::read_to_string(&host.paths.config_file).unwrap();
    assert!(text.starts_with("# Ultima V engine settings"), "{text}");
    assert_eq!(
        host.load_config().unwrap().game_dir.unwrap(),
        game.canonicalize().unwrap()
    );
}
