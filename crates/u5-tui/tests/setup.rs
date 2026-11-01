//! Game-data setup glue: CLI flags, the terminal wizard, and the binary's
//! no-argument and error paths in an isolated HOME.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use u5_gamedata::MANIFEST;
use u5_host_desktop::{DesktopHost, HostEnv, Platform, Resolution, ResolveRequest};
use u5_tui::*;

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-setup-{label}-{}-{unique}",
            std::process::id()
        ));
        for sub in ["home", "tmp", "cwd"] {
            fs::create_dir_all(dir.join(sub)).unwrap();
        }
        Self(dir)
    }
    fn home(&self) -> PathBuf {
        self.0.join("home")
    }
    fn host(&self) -> DesktopHost {
        DesktopHost::new(HostEnv {
            platform: Platform::Unix,
            vars: HashMap::new(),
            home: Some(self.home()),
            temp_dir: self.0.join("tmp"),
            current_dir: Some(self.0.join("cwd")),
            exe_dir: None,
        })
    }
    /// The engine binary with nothing from this environment leaking in.
    fn engine(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_u5-engine"));
        command
            .env_clear()
            .env("HOME", self.home())
            .env("TMPDIR", self.0.join("tmp"))
            .current_dir(self.0.join("cwd"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn populate(dir: &Path, skip: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    for file in MANIFEST {
        if !skip.contains(&file.name) {
            fs::write(dir.join(file.name), format!("synthetic {}", file.name)).unwrap();
        }
    }
}

fn pristine_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("U5_TEST_ASSET_DIR")?);
    dir.is_dir().then_some(dir)
}

#[test]
fn setup_and_game_dir_flags_parse() {
    let command = parse_cli_command(["--setup"]).unwrap();
    assert!(command.args.setup);
    assert!(!command.args.has_mode());
    assert_eq!(command.explicit_game_dir, None);

    let command = parse_cli_command(["--play", "--game-dir", "/games/u5"]).unwrap();
    assert_eq!(command.explicit_game_dir, Some(PathBuf::from("/games/u5")));
    assert!(command.args.has_mode());
    let args = command.finish(PathBuf::from("/managed")).unwrap();
    assert_eq!(args.game_dir, PathBuf::from("/managed"));

    assert!(parse_cli_command(["--game-dir", "/a", "/b"]).is_err());
    assert!(parse_cli_command(["/a", "/b"]).is_err());
    assert!(parse_cli_command(["--game-dir"]).is_err());

    // No folder: nothing defaults to a developer path any more.
    let bare = parse_cli_command(Vec::<String>::new()).unwrap();
    assert_eq!(bare.explicit_game_dir, None);
    assert_eq!(bare.args.game_dir, PathBuf::new());
    assert!(!bare.args.has_mode());
    assert!(bare.args.needs_game_data());
    assert!(
        !parse_cli_command(["--audio-suite", "out"])
            .unwrap()
            .args
            .needs_game_data()
    );
}

#[test]
fn terminal_wizard_picks_a_discovered_folder() {
    let scratch = Scratch::new("tw-pick");
    let game = scratch.home().join("GOG Games").join("Ultima 5");
    populate(&game, &[]);
    let host = scratch.host();
    let Resolution::NeedsWizard(need) = host.resolve(&ResolveRequest {
        explicit: None,
        force_setup: true,
    }) else {
        panic!("--setup always asks");
    };
    let mut input = Cursor::new(b"1\ny\n".to_vec());
    let mut output = Vec::new();
    let ready = run_terminal_wizard(&host, &need, &mut input, &mut output)
        .unwrap()
        .expect("setup completes");
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("1) "), "{text}");
    assert!(text.contains("ready"), "{text}");
    assert!(text.contains("Use this folder? [Y/n]"), "{text}");
    assert!(text.contains("Run with --setup to change it."), "{text}");
    assert!(ready.data_dir.join("BRIT.DAT").is_file());
    assert_eq!(
        host.load_config().unwrap().game_dir.unwrap(),
        game.canonicalize().unwrap()
    );
}

#[test]
fn terminal_wizard_reports_missing_files_for_a_typed_path() {
    let scratch = Scratch::new("tw-missing");
    let game = scratch.home().join("copy");
    populate(&game, &["TILES.16", "KARMA.DAT"]);
    let host = scratch.host();
    let need = WizardNeed {
        reason: "No Ultima V game files were found on this computer.".into(),
        candidates: Vec::new(),
        notes: Vec::new(),
    };
    let mut input = Cursor::new(b"h\n~/copy\nq\n".to_vec());
    let mut output = Vec::new();
    let result = run_terminal_wizard(&host, &need, &mut input, &mut output).unwrap();
    assert!(result.is_none());
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("innoextract"), "help text shown: {text}");
    assert!(
        text.contains("2 of the 63 files the game needs are missing"),
        "{text}"
    );
    assert!(text.contains("KARMA.DAT, TILES.16"), "{text}");
    assert!(!host.paths.config_file.exists());
}

#[test]
fn terminal_wizard_ends_cleanly_at_end_of_input() {
    let scratch = Scratch::new("tw-eof");
    let host = scratch.host();
    let need = WizardNeed {
        reason: "Setup was requested.".into(),
        candidates: Vec::new(),
        notes: Vec::new(),
    };
    let mut input = Cursor::new(Vec::new());
    let mut output = Vec::new();
    assert!(
        run_terminal_wizard(&host, &need, &mut input, &mut output)
            .unwrap()
            .is_none()
    );
}

#[test]
fn path_expansion_handles_quotes_and_home() {
    let home = Path::new("/home/ann");
    assert_eq!(
        expand_path("'~/GOG Games/Ultima 5'", Some(home)),
        home.join("GOG Games/Ultima 5")
    );
    assert_eq!(expand_path("\"/x y\"", Some(home)), PathBuf::from("/x y"));
    assert_eq!(expand_path("~", Some(home)), home);
}

#[test]
fn bare_launch_without_data_or_terminal_explains_what_to_do() {
    let scratch = Scratch::new("bin-nodata");
    let output = scratch.engine().output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.starts_with("Error: No Ultima V game files were found"),
        "{stderr}"
    );
    assert!(stderr.contains("u5-engine --setup"), "{stderr}");
    assert!(!stderr.contains("Custom {"), "{stderr}");
}

#[test]
fn bare_launch_lists_a_partial_install_it_found() {
    let scratch = Scratch::new("bin-partial");
    populate(
        &scratch.home().join("GOG Games").join("Ultima 5"),
        &["SIGNS.DAT"],
    );
    let output = scratch.engine().arg("--play").output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("none has every file"), "{stderr}");
    assert!(
        stderr.contains("1 of 63 required files missing"),
        "{stderr}"
    );
}

#[test]
fn explicit_incomplete_folder_fails_with_a_plain_report() {
    let scratch = Scratch::new("bin-explicit");
    let game = scratch.0.join("game");
    populate(&game, &["CASTLE.DAT"]);
    let output = scratch
        .engine()
        .args(["--play-script", "q"])
        .arg(&game)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(!stderr.contains("Custom {"), "{stderr}");
    assert!(!stderr.contains("kind: "), "{stderr}");
    assert!(stderr.contains("CASTLE.DAT"), "{stderr}");
    assert!(
        stderr.contains("1 of the 63 files the game needs is missing"),
        "{stderr}"
    );
    assert!(stderr.contains("u5-engine --setup"), "{stderr}");
}

#[test]
fn setup_with_a_folder_sets_it_up_without_asking() {
    let Some(pristine) = pristine_dir() else {
        return;
    };
    let scratch = Scratch::new("bin-setup-dir");
    let copy = scratch.0.join("lowercase");
    fs::create_dir_all(&copy).unwrap();
    for entry in fs::read_dir(&pristine).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        fs::copy(entry.path(), copy.join(name)).unwrap();
    }
    let output = scratch
        .engine()
        .args(["--setup", "--location-audit"])
        .arg(scratch.0.join("audit.txt"))
        .arg(&copy)
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stderr}");
    let config = fs::read_to_string(scratch.home().join(".config/u5-engine/config.toml")).unwrap();
    assert!(config.contains("version = \"gog-1.0\""), "{config}");
    assert!(scratch.0.join("audit.txt").is_file());
}

#[test]
fn bare_route_smoke_finds_a_nested_lowercase_gog_copy() {
    let Some(pristine) = pristine_dir() else {
        return;
    };
    let scratch = Scratch::new("bin-discover");
    let data = scratch
        .home()
        .join("GOG Games")
        .join("Ultima 4+5+6")
        .join("Ultima 5")
        .join("data");
    fs::create_dir_all(&data).unwrap();
    for entry in fs::read_dir(&pristine).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        fs::copy(entry.path(), data.join(name)).unwrap();
    }
    let output = scratch.engine().arg("--route-smoke").output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("Found the game files in"), "{stderr}");
    assert!(
        stdout.contains("Route smoke: all cases passed."),
        "{stdout}"
    );
    // The source copy was only read.
    assert!(data.join("brit.dat").is_file());
    assert!(!data.join("SAVED.WPS").exists());
}
