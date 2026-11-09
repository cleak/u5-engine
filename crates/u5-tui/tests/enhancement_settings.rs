//! Enhancement settings on the command line and the parity lock (Phase B
//! design 1.2.4 and 1.3.11, package M0-07): the generic flags, their
//! rejections, `--show-settings` from the binary, and source scans that pin
//! which launches resolve settings and which read `config.toml`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use u5_runtime::enhancements::{CliSetting, Preset};
use u5_tui::*;

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-enh-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(dir.join("home")).unwrap();
        Self(dir)
    }

    fn config(&self) -> PathBuf {
        self.0.join("config.toml")
    }

    /// The engine binary with nothing from this environment leaking in and
    /// its settings file inside the scratch folder.
    fn engine(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_u5-engine"));
        command
            .env_clear()
            .env("HOME", self.0.join("home"))
            .env("U5_ENGINE_CONFIG", self.config())
            .current_dir(&self.0)
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

fn parse_error(args: &[&str]) -> String {
    let error = parse_cli_command(args.iter().copied()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{error}");
    error.to_string()
}

#[test]
fn generic_flags_parse_in_command_line_order() {
    let args = parse_cli_args([
        "--preset",
        "recommended",
        "--enable",
        "enhancements.freeze_overlays",
        "--play",
        "--disable",
        " enhancements.freeze_overlays ,",
        "--set",
        "keys.menu=F9",
        "--set",
        "enhancements.preset = everything",
        "--parity-lock",
        r"C:\Games\U5-Clean",
    ])
    .unwrap();
    assert!(args.play);
    assert!(args.parity_lock);
    assert!(!args.show_settings);
    assert_eq!(
        args.enhancements,
        [
            CliSetting::Preset(Preset::Recommended),
            CliSetting::Enable("enhancements.freeze_overlays".into()),
            CliSetting::Disable("enhancements.freeze_overlays".into()),
            CliSetting::Set {
                key: "keys.menu".into(),
                value: "F9".into()
            },
            CliSetting::Set {
                key: "enhancements.preset".into(),
                value: "everything".into()
            },
        ]
    );
}

#[test]
fn no_settings_flags_means_no_settings() {
    for argv in [
        &["--play", r"C:\Games\U5-Clean"][..],
        &["--visual-playable", r"C:\Games\U5-Clean"],
        &["--route-smoke", r"C:\Games\U5-Clean"],
        &[],
    ] {
        let args = parse_cli_args(argv.iter().copied()).unwrap();
        assert!(args.enhancements.is_empty());
        assert!(!args.parity_lock);
        // Only the frontend marks the default launch as reading the file.
        assert!(!args.settings_file);
    }
}

#[test]
fn enable_takes_a_comma_list() {
    let args = parse_cli_args([
        "--enable",
        "enhancements.freeze_overlays,enhancements.freeze_overlays",
    ])
    .unwrap();
    assert_eq!(args.enhancements.len(), 2);
}

#[test]
fn unknown_options_and_keys_are_rejected() {
    // The rejection of unknown options stays.
    assert!(parse_error(&["--bogus"]).contains("unknown option `--bogus`"));
    assert!(parse_error(&["--quest-journal"]).contains("unknown option"));
    assert!(parse_error(&["--no-preset"]).contains("unknown option"));
    assert!(
        parse_error(&["--enable", "display.growleft"])
            .contains("unknown setting `display.growleft`")
    );
    assert!(parse_error(&["--set", "nope.key=1"]).contains("unknown setting `nope.key`"));
    assert!(parse_error(&["--enable", "keys.menu"]).contains("not an on/off setting"));
    assert!(parse_error(&["--set", "keys.menu"]).contains("KEY=VALUE"));
    assert!(parse_error(&["--preset", "max"]).contains("unknown preset `max`"));
    for flag in ["--preset", "--enable", "--disable", "--set"] {
        assert!(parse_error(&[flag]).contains("requires"), "{flag}");
    }
    assert!(parse_error(&["--enable", " , "]).contains("requires KEY"));
}

#[test]
fn qa_modes_take_no_settings_but_accept_the_lock() {
    for mode in [
        &["--route-smoke"][..],
        &["--save-frame-suite", "out"],
        &["--visual-frame-suite", "out"],
        &["--visual-route-suite", "out"],
        &["--audio-suite", "out"],
        &["--write-seed"],
        &["--save-frame", "x.png"],
    ] {
        let mut argv: Vec<&str> = mode.to_vec();
        argv.extend(["--preset", "everything", "GAME"]);
        assert!(
            parse_error(&argv).contains("apply to play launches only"),
            "{mode:?}"
        );
        let mut locked: Vec<&str> = mode.to_vec();
        locked.extend(["--parity-lock", "GAME"]);
        let args = parse_cli_args(locked).unwrap();
        assert!(args.parity_lock && args.is_qa_mode(), "{mode:?}");
    }
}

#[test]
fn show_settings_stands_alone() {
    let args = parse_cli_args(["--show-settings", "--preset", "classic", "--parity-lock"]).unwrap();
    assert!(args.show_settings);
    assert!(args.has_mode());
    assert!(!args.needs_game_data());
    assert!(parse_error(&["--show-settings", "--play"]).contains("--show-settings"));
    assert!(parse_error(&["--show-settings", "--setup"]).contains("--show-settings"));
}

#[test]
fn settings_flag_opens_the_page_before_the_default_launch() {
    let args = parse_cli_args(["--settings"]).unwrap();
    assert!(args.settings);
    assert!(!args.has_mode());
    assert!(!args.is_qa_mode());
    let args = parse_cli_args(["--settings", "--preset", "recommended", "--parity-lock"]).unwrap();
    assert!(args.settings && args.parity_lock);
    // A folder being set up is still the default launch.
    let command = parse_cli_command(["--settings", "--setup", "GAME"]).unwrap();
    assert!(command.args.settings && command.args.setup);
    for argv in [
        &["--settings", "--play"][..],
        &["--settings", "--visual"],
        &["--settings", "--route-smoke"],
        &["--settings", "GAME"],
    ] {
        assert!(parse_error(argv).contains("--settings opens"), "{argv:?}");
    }
    assert!(parse_error(&["--show-settings", "--settings"]).contains("--show-settings"));
    assert!(!parse_cli_args(["--play", "GAME"]).unwrap().settings);
}

#[test]
fn usage_lists_the_generic_flags() {
    for flag in [
        "--preset <NAME>",
        "--enable <KEY[,KEY...]>",
        "--disable <KEY[,KEY...]>",
        "--set <KEY=VALUE>",
        "--parity-lock",
        "--show-settings",
        "--settings  ",
        "U5_PARITY_LOCK",
    ] {
        assert!(CLI_USAGE.contains(flag), "{flag}");
    }
    // No shipped row has an alias besides `--preset` yet.
    assert_eq!(cli_usage(), CLI_USAGE);
}

#[test]
fn binary_shows_settings_from_the_file() {
    let scratch = Scratch::new("show");
    fs::write(
        scratch.config(),
        "game_dir = \"/g\"\n[enhancements]\npreset = \"recommended\"\n[display]\ngrowleft = true\n",
    )
    .unwrap();
    let output = scratch.engine().arg("--show-settings").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.starts_with(&format!("config: {}\n", scratch.config().display())),
        "{stdout}"
    );
    assert!(stdout.contains("\npreset: recommended\n"), "{stdout}");
    assert!(stdout.contains("\nlock:   off\n"), "{stdout}");
    assert!(
        stdout.contains("display.growleft: unknown key, ignored"),
        "{stdout}"
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn binary_lock_skips_the_file() {
    let scratch = Scratch::new("lock");
    fs::write(
        scratch.config(),
        "[enhancements]\npreset = \"everything\"\n",
    )
    .unwrap();
    for (env, reason) in [
        (Some(("U5_PARITY_LOCK", "1")), "on (U5_PARITY_LOCK)"),
        (
            Some(("U5_EMULATOR_LOAD_LATENCY_MS", "280")),
            "on (a parity-run variable is set)",
        ),
        (None, "on (--parity-lock)"),
    ] {
        let mut command = scratch.engine();
        command.args([
            "--show-settings",
            "--enable",
            "enhancements.freeze_overlays",
        ]);
        match env {
            Some((name, value)) => {
                command.env(name, value);
            }
            None => {
                command.arg("--parity-lock");
            }
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("(not read: parity lock)"), "{stdout}");
        assert!(stdout.contains("\npreset: classic\n"), "{stdout}");
        assert!(
            stdout.contains(&format!("\nlock:   {reason}\n")),
            "{stdout}"
        );
        assert!(
            stdout.contains("parity lock: ignored --enable enhancements.freeze_overlays"),
            "{stdout}"
        );
    }
}

#[test]
fn binary_rejects_an_unknown_key_before_anything_else() {
    let scratch = Scratch::new("reject");
    let output = scratch
        .engine()
        .args(["--enable", "display.growleft"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("unknown setting `display.growleft`"),
        "{stderr}"
    );
    assert!(!scratch.config().exists());
}

#[cfg(not(feature = "visual"))]
#[test]
fn binary_without_a_window_explains_settings() {
    let scratch = Scratch::new("settings");
    let output = scratch.engine().arg("--settings").output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no settings window"), "{stderr}");
    assert!(!scratch.config().exists());
}

// ---- source scans (design 1.3.11, 1.2.4) ----

fn crate_source(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

/// Top-level items of a source file, split at each line that starts a new
/// item, so a scan can say which function a name appears in.
fn top_level_items(source: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in source.lines() {
        let starts_item = ["fn ", "pub fn ", "#[cfg(", "#[test]", "struct ", "impl "]
            .iter()
            .any(|start| line.starts_with(start));
        if starts_item || items.is_empty() {
            items.push(String::new());
        }
        let item = items.last_mut().unwrap();
        item.push_str(line);
        item.push('\n');
    }
    items
}

fn item_name(item: &str) -> &str {
    let after = item
        .split("fn ")
        .nth(1)
        .unwrap_or("")
        .split(['(', '<'])
        .next()
        .unwrap_or("");
    after.trim()
}

const RESOLVE_NAMES: [&str; 5] = [
    "launch_enhancements",
    "resolve_launch",
    "enhancement_settings",
    "ResolveInput",
    "enhancements::resolve",
];

#[test]
fn qa_modes_never_resolve() {
    for file in [
        "src/route_smoke.rs",
        "src/save_frame.rs",
        "src/audio_suite.rs",
        "src/visual_manifest.rs",
        "src/play_loop.rs",
        "src/intro_loop.rs",
        "../u5-bevy/src/lib.rs",
        "../u5-bevy/src/enh/mod.rs",
        "../u5-bevy/src/enh/input.rs",
        "../u5-bevy/src/enh/idle.rs",
        "../u5-bevy/src/enh/overlay.rs",
        "../u5-bevy/src/enh/synthetic.rs",
        "../u5-bevy/src/enh/toast.rs",
    ] {
        let source = crate_source(file);
        for name in RESOLVE_NAMES {
            assert!(!source.contains(name), "{file} names {name}");
        }
    }
    // In main.rs only the two visual play launches resolve, through one
    // helper; dispatch and every QA path do not.
    let main = crate_source("src/main.rs");
    let resolving: Vec<String> = top_level_items(&main)
        .iter()
        .filter(|item| RESOLVE_NAMES.iter().any(|name| item.contains(name)))
        .map(|item| item_name(item).to_string())
        .collect();
    assert_eq!(resolving, ["launch_config"]);
    let callers: Vec<String> = top_level_items(&main)
        .iter()
        .filter(|item| item.contains("launch_config(&args)"))
        .map(|item| item_name(item).to_string())
        .collect();
    assert_eq!(callers, ["run_visual", "run_visual_intro"]);
}

#[test]
fn only_the_default_launch_reads_the_file() {
    let read = "SettingsFile::Read";
    let setup = crate_source("src/setup.rs");
    let readers: Vec<String> = top_level_items(&setup)
        .iter()
        .filter(|item| item.contains(read))
        .map(|item| item_name(item).to_string())
        .collect();
    // `u5-engine-gui` runs `run_default_launch` too.
    assert_eq!(readers, ["run_default_launch"]);
    assert!(crate_source("src/bin/u5-engine-gui.rs").contains("run_gui_main"));

    let main = crate_source("src/main.rs");
    assert_eq!(main.matches(read).count(), 1);
    assert_eq!(main.matches("settings_file = true").count(), 1);
    let marker = top_level_items(&main)
        .into_iter()
        .find(|item| item.contains("settings_file = true"))
        .unwrap();
    assert_eq!(item_name(&marker), "run");
    assert!(marker.contains("if default_launch {\n        args.intro = true;"));

    for file in [
        "src/cli.rs",
        "src/play_loop.rs",
        "src/route_smoke.rs",
        "src/equivalence.rs",
    ] {
        let source = crate_source(file);
        assert!(!source.contains(read), "{file}");
        assert!(!source.contains("ConfigDocument"), "{file}");
    }
}
