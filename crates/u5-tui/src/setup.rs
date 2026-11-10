//! Game-data setup glue for the desktop frontends.
//!
//! Connects the desktop host (`u5-host-desktop`: config, discovery, the
//! resolution pipeline) to this binary: the terminal setup wizard, the
//! windowed wizard (`u5-bevy-setup`, run in a child process because Bevy's
//! window loop can start only once per process), friendly error text, and
//! the default no-argument launch shared with the `u5-engine-gui` binary.
//! The engine crates never see any of this; they get a directory path.
//!
//! The windowed wizard's Enhancements page (`--settings`, Phase B design
//! 1.2.2) is fed from here too: [`DesktopWizardHost`] turns the enhancement
//! catalogue and `config.toml` into the page's generic rows and stores what
//! the player changed through the `config.toml` adapter.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use u5_gamedata::{Discovered, known_version};
use u5_host_desktop::{
    DesktopHost, HOW_TO_GET_THE_FILES, Origin, ReadyGameData, Resolution, ResolveRequest,
    WizardNeed,
};

/// The internal flag the windowed wizard's child process is started with.
pub const SETUP_WINDOW_FLAG: &str = "--setup-window";

/// The internal flag the Enhancements page's child process is started with
/// (`--settings`). `--parity-lock` may follow it.
pub const SETTINGS_WINDOW_FLAG: &str = "--settings-window";

/// Child exit code: the player closed the wizard without choosing.
#[cfg(feature = "visual")]
const WIZARD_CANCELLED_EXIT: u8 = 2;
/// Child exit code: setup was saved, but the player chose not to play now.
#[cfg(feature = "visual")]
const WIZARD_SAVED_ONLY_EXIT: u8 = 3;

/// How to ask the player when the pipeline needs a wizard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WizardMode {
    /// The windowed wizard (visual builds), falling back to the terminal
    /// when no window can be opened and stdin is a terminal.
    Window,
    /// The terminal wizard (only if stdin is a terminal).
    Terminal,
}

impl WizardMode {
    /// The windowed wizard for the visual game, the terminal otherwise.
    pub fn for_visual(visual: bool) -> Self {
        if visual && cfg!(feature = "visual") {
            Self::Window
        } else {
            Self::Terminal
        }
    }
}

/// Resolve the game folder for this run, asking the player if needed.
///
/// Notes for the player go to stderr so a harness's stdout is unchanged.
pub fn resolve_game_data(
    explicit: Option<PathBuf>,
    force_setup: bool,
    mode: WizardMode,
) -> Result<ReadyGameData, String> {
    let host = DesktopHost::from_process();
    let request = ResolveRequest {
        explicit,
        force_setup,
    };
    match host.resolve(&request) {
        Resolution::Ready(ready) => {
            print_notes(&ready.notes);
            Ok(ready)
        }
        Resolution::NeedsWizard(need) => {
            print_notes(&need.notes);
            run_wizard(&host, &need, mode)
        }
    }
}

fn print_notes(notes: &[String]) {
    for note in notes {
        eprintln!("{note}");
    }
}

fn run_wizard(
    host: &DesktopHost,
    need: &WizardNeed,
    mode: WizardMode,
) -> Result<ReadyGameData, String> {
    if mode == WizardMode::Window {
        match run_window_wizard_child(&need.reason) {
            Ok(true) => {
                return match host.resolve(&ResolveRequest::default()) {
                    Resolution::Ready(ready) => {
                        print_notes(&ready.notes);
                        Ok(ready)
                    }
                    Resolution::NeedsWizard(need) => Err(format!(
                        "Setup finished, but the game files still cannot be used. {}",
                        need.reason
                    )),
                };
            }
            Ok(false) => {
                return Err(
                    "Setup was cancelled; the game needs its data files to start.".to_string(),
                );
            }
            Err(error) => {
                eprintln!("The setup window could not be opened ({error}).");
                if !io::stdin().is_terminal() {
                    return Err(no_data_message(need));
                }
                eprintln!("Continuing in the terminal.");
            }
        }
    }
    if !io::stdin().is_terminal() {
        return Err(no_data_message(need));
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout();
    match run_terminal_wizard(host, need, &mut input, &mut output) {
        Ok(Some(ready)) => Ok(ready),
        Ok(None) => Err("Setup was cancelled; the game needs its data files to start.".to_string()),
        Err(error) => Err(format!("Setup failed: {error}")),
    }
}

/// The message for a run that needs the wizard but cannot show one.
pub fn no_data_message(need: &WizardNeed) -> String {
    let mut text = format!("{}\n", need.reason);
    for found in &need.candidates {
        text.push_str(&format!(
            "  {} ({}): {}\n",
            found.data_root.display(),
            found.candidate.source,
            found.report.status_line()
        ));
    }
    text.push_str(
        "Run `u5-engine --setup` in a terminal to choose the game folder, pass it on the \
         command line (`u5-engine <GAME_DIR>`), or set U5_GAME_DIR.",
    );
    text
}

/// Start the windowed wizard in a child process. `Ok(true)` when the
/// player finished setup, `Ok(false)` when they cancelled.
#[cfg(feature = "visual")]
fn run_window_wizard_child(reason: &str) -> io::Result<bool> {
    let exe = std::env::current_exe()?;
    let status = std::process::Command::new(exe)
        .arg(SETUP_WINDOW_FLAG)
        .arg(reason)
        .status()?;
    match status.code() {
        Some(0) => Ok(true),
        Some(code) if code == i32::from(WIZARD_CANCELLED_EXIT) => Ok(false),
        Some(code) if code == i32::from(WIZARD_SAVED_ONLY_EXIT) => {
            // Saved, but the player quit from the final screen: not an error.
            println!("Setup is saved. Run u5-engine again to play.");
            std::process::exit(0);
        }
        _ => Err(io::Error::other(format!(
            "the setup window exited with {status}"
        ))),
    }
}

#[cfg(not(feature = "visual"))]
fn run_window_wizard_child(_reason: &str) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this build has no windowed setup",
    ))
}

/// The child-process side of the windowed wizard (`--setup-window`).
#[cfg(feature = "visual")]
pub fn run_setup_window_child(reason: Option<String>) -> ExitCode {
    use std::sync::Arc;
    let host = DesktopHost::from_process();
    let candidates = host.discover();
    let reason =
        reason.unwrap_or_else(|| "Choose the folder with your Ultima V game files.".into());
    let parity_locked = crate::enhancement_settings::lock_state_for_process(false)
        != u5_runtime::enhancements::LockState::Off;
    let wizard_host: Arc<dyn u5_bevy_setup::WizardHost> = Arc::new(DesktopWizardHost {
        host,
        parity_locked,
    });
    match u5_bevy_setup::run_setup_wizard(wizard_host, reason, candidates) {
        u5_bevy_setup::WizardOutcome::Completed(_) => ExitCode::SUCCESS,
        u5_bevy_setup::WizardOutcome::SavedOnly(_) => ExitCode::from(WIZARD_SAVED_ONLY_EXIT),
        // The setup wizard never ends from its Enhancements page; that
        // page returns to the welcome screen.
        u5_bevy_setup::WizardOutcome::SettingsClosed { .. }
        | u5_bevy_setup::WizardOutcome::Cancelled => ExitCode::from(WIZARD_CANCELLED_EXIT),
    }
}

#[cfg(not(feature = "visual"))]
pub fn run_setup_window_child(_reason: Option<String>) -> ExitCode {
    eprintln!("This build has no windowed setup; run `u5-engine --setup` instead.");
    ExitCode::FAILURE
}

/// `--settings`: show the Enhancements page in its own window process.
/// `Ok(true)` when the player left it with Save or Cancel (start the game),
/// `Ok(false)` when they closed the window (do not).
#[cfg(feature = "visual")]
pub fn open_settings_page(parity_lock_flag: bool) -> Result<bool, String> {
    let failed = |error: io::Error| format!("The settings window could not be opened ({error}).");
    let exe = std::env::current_exe().map_err(failed)?;
    let mut command = std::process::Command::new(exe);
    command.arg(SETTINGS_WINDOW_FLAG);
    if parity_lock_flag {
        command.arg("--parity-lock");
    }
    let status = command.status().map_err(failed)?;
    match status.code() {
        Some(0) => Ok(true),
        Some(code) if code == i32::from(WIZARD_CANCELLED_EXIT) => Ok(false),
        _ => Err(format!("The settings window exited with {status}.")),
    }
}

#[cfg(not(feature = "visual"))]
pub fn open_settings_page(_parity_lock_flag: bool) -> Result<bool, String> {
    Err(
        "This build has no settings window. Edit config.toml, or pass --preset, \
         --enable, --disable or --set for one launch."
            .to_string(),
    )
}

/// The child-process side of `--settings` (`--settings-window`): the
/// wizard showing only its Enhancements page. The lock comes from the
/// environment, which the child inherits, or `--parity-lock` after the flag.
#[cfg(feature = "visual")]
pub fn run_settings_window_child(args: &[String]) -> ExitCode {
    use std::sync::Arc;
    let lock = crate::enhancement_settings::lock_state_for_process(
        args.iter().any(|arg| arg == "--parity-lock"),
    );
    let wizard_host: Arc<dyn u5_bevy_setup::WizardHost> = Arc::new(DesktopWizardHost {
        host: DesktopHost::from_process(),
        parity_locked: lock != u5_runtime::enhancements::LockState::Off,
    });
    match u5_bevy_setup::run_settings_wizard(wizard_host) {
        u5_bevy_setup::WizardOutcome::SettingsClosed { .. } => ExitCode::SUCCESS,
        _ => ExitCode::from(WIZARD_CANCELLED_EXIT),
    }
}

#[cfg(not(feature = "visual"))]
pub fn run_settings_window_child(_args: &[String]) -> ExitCode {
    eprintln!("This build has no settings window.");
    ExitCode::FAILURE
}

/// The desktop implementation of the windowed wizard's host.
#[cfg(feature = "visual")]
pub struct DesktopWizardHost {
    pub host: DesktopHost,
    /// The parity lock is on for this launch: the Enhancements page hides
    /// its Cheats group.
    pub parity_locked: bool,
}

#[cfg(feature = "visual")]
impl u5_bevy_setup::WizardHost for DesktopWizardHost {
    fn discover(&self) -> Vec<Discovered> {
        self.host.discover()
    }

    fn validate(&self, path: &Path) -> u5_gamedata::ValidationReport {
        self.host.validate(path)
    }

    fn commit(&self, path: &Path) -> Result<u5_bevy_setup::CommitSummary, String> {
        let ready = self
            .host
            .commit(path, Origin::Setup)
            .map_err(|error| error.to_string())?;
        Ok(u5_bevy_setup::CommitSummary {
            message: commit_message(&ready),
            data_dir: ready.data_dir,
        })
    }

    fn start_folder_pick(&self) -> Option<std::sync::mpsc::Receiver<Option<PathBuf>>> {
        Some(u5_host_desktop::dialogs::spawn_folder_picker(
            "Choose the Ultima V game folder",
            self.host.env.home.clone(),
        ))
    }

    fn help_text(&self) -> String {
        HOW_TO_GET_THE_FILES.to_string()
    }

    fn settings_rows(&self) -> Option<u5_bevy_setup::SettingsRows> {
        Some(wizard_settings(
            &self.host.paths.config_file,
            self.parity_locked,
        ))
    }

    fn save_settings(&self, changes: &u5_bevy_setup::SettingsChanges) -> Result<(), String> {
        save_wizard_settings(&self.host.paths.config_file, changes)
    }
}

/// The Enhancements page's groups, in order (design 1.2.2), by the section
/// of a setting's key. A section not listed goes after Combat.
#[cfg(feature = "visual")]
const SETTING_GROUPS: &[(&str, &[&str])] = &[
    ("Display", &["display"]),
    ("Maps", &["automap", "gem_map"]),
    ("Journal", &["journal"]),
    ("Menu and saves", &["menu", "input", "enhancements"]),
    ("Audio", &["audio"]),
    ("Combat", &["combat"]),
    ("Other", &[]),
    ("Cheats", &["cheats"]),
];

#[cfg(feature = "visual")]
fn setting_group(key: &str) -> usize {
    let section = key.split('.').next().unwrap_or(key);
    SETTING_GROUPS
        .iter()
        .position(|(_, sections)| sections.contains(&section))
        .unwrap_or_else(|| {
            SETTING_GROUPS
                .iter()
                .position(|(name, _)| *name == "Other")
                .unwrap_or(0)
        })
}

/// What the Enhancements page shows for the settings in `config.toml`.
///
/// Rows are the catalogue's shipped rows that a page can edit: every
/// toggle, and every on/off, word-choice or small-number setting. The
/// foundation's own keys (the file format, the preset, which is the
/// page's preset row, and the `[keys]` chords) and free-form values
/// (colours, paths, lists) are left to `config.toml`. Values are the
/// file's, resolved without the command line or the lock: the page edits
/// the file for later launches.
#[cfg(feature = "visual")]
pub fn wizard_settings(config: &Path, locked: bool) -> u5_bevy_setup::SettingsRows {
    use crate::enhancement_settings::resolve_launch;
    use u5_runtime::enhancements::{LockState, PRESETS};

    let settings = resolve_launch(Some(config), &[], LockState::Off);
    let mut notes = settings.warnings.clone();
    if u5_host_desktop::ConfigDocument::load(config)
        .ok()
        .is_some_and(|doc| doc.damaged().is_some())
    {
        notes.push(format!(
            "Saving here keeps the damaged file as {} and starts a fresh one.",
            u5_host_desktop::settings::backup_path(config).display()
        ));
    }
    let rows = wizard_rows(&settings.resolved);
    let presets = PRESETS
        .iter()
        .map(|preset| preset_choice(*preset, &rows))
        .collect();
    let preset = PRESETS
        .iter()
        .position(|preset| *preset == settings.resolved.preset.base())
        .unwrap_or(0);
    u5_bevy_setup::SettingsRows {
        rows,
        presets,
        preset,
        locked,
        notes,
    }
}

/// The page's rows for one resolution, grouped in [`SETTING_GROUPS`]
/// order and in catalogue order within a group.
#[cfg(feature = "visual")]
fn wizard_rows(
    resolved: &u5_runtime::enhancements::ResolvedEnhancements,
) -> Vec<u5_bevy_setup::SettingRow> {
    use u5_bevy_setup::{RowKind, RowRole, RowValue, SettingRow};
    use u5_runtime::enhancements::{
        CHEATS_SWITCH, CatalogueRow, OptionDefault, OptionKind, SettingValue, Tier,
    };

    /// Small number ranges become a choice; larger ones stay in the file.
    const MAX_NUMBER_CHOICES: i64 = 10;

    let mut rows: Vec<(usize, SettingRow)> = Vec::new();
    for row in CatalogueRow::all().filter(|row| row.is_shipped()) {
        let (kind, value, default, tier) = match row {
            CatalogueRow::Toggle(spec) => (
                RowKind::Switch,
                RowValue::Switch(resolved.toggles.on(spec.toggle)),
                RowValue::Switch(false),
                spec.tier,
            ),
            CatalogueRow::Option(spec) => {
                if spec.class.is_none() && spec.key != CHEATS_SWITCH {
                    continue;
                }
                let kind = match spec.kind {
                    OptionKind::Bool => RowKind::Switch,
                    OptionKind::Enum(words) => RowKind::Choice(
                        words
                            .iter()
                            .map(|word| (word.to_string(), word.to_string()))
                            .collect(),
                    ),
                    OptionKind::Int { min, max } if max - min < MAX_NUMBER_CHOICES => {
                        RowKind::Choice(
                            (min..=max)
                                .map(|number| (number.to_string(), number.to_string()))
                                .collect(),
                        )
                    }
                    _ => continue,
                };
                let as_row = |value: &SettingValue| match value {
                    SettingValue::Bool(flag) => RowValue::Switch(*flag),
                    SettingValue::Int(number) => RowValue::Choice(number.to_string()),
                    other => RowValue::Choice(other.to_string()),
                };
                let default = match spec.default {
                    OptionDefault::Bool(flag) => RowValue::Switch(flag),
                    OptionDefault::Int(number) => RowValue::Choice(number.to_string()),
                    OptionDefault::Str(text) => RowValue::Choice(text.to_string()),
                    OptionDefault::List(_) => continue,
                };
                let value = resolved
                    .values
                    .get(spec.key)
                    .map_or_else(|| default.clone(), as_row);
                (kind, value, default, spec.tier)
            }
        };
        let key = row.key();
        let role = if key == CHEATS_SWITCH {
            RowRole::AllowCheats
        } else if key.starts_with("cheats.") && tier == Tier::Cheat {
            RowRole::Cheat
        } else if key.starts_with("cheats.") {
            RowRole::WatchState
        } else {
            RowRole::Setting
        };
        let group = setting_group(key);
        rows.push((
            group,
            SettingRow {
                key: key.to_string(),
                label: row.label().to_string(),
                group: SETTING_GROUPS[group].0.to_string(),
                tags: row
                    .tags()
                    .iter()
                    .map(|tag| tag.label().to_string())
                    .collect(),
                kind,
                role,
                value,
                default,
            },
        ));
    }
    rows.sort_by_key(|(group, _)| *group);
    rows.into_iter().map(|(_, row)| row).collect()
}

/// A preset as the page sees it: the value it gives every page row any
/// preset names. Rows no preset names (preferences, cheats) are not
/// listed, so choosing a preset leaves them alone (design 1.4.2).
#[cfg(feature = "visual")]
fn preset_choice(
    preset: u5_runtime::enhancements::Preset,
    rows: &[u5_bevy_setup::SettingRow],
) -> u5_bevy_setup::PresetChoice {
    use u5_runtime::enhancements::{CliSetting, LockState, PRESETS};

    let governed: Vec<&str> = PRESETS
        .iter()
        .flat_map(|preset| preset.shipped_entries().map(|(entry, _)| entry.key))
        .collect();
    let applied = crate::enhancement_settings::resolve_launch(
        None,
        &[CliSetting::Preset(preset)],
        LockState::Off,
    );
    let applied_rows = wizard_rows(&applied.resolved);
    let values = applied_rows
        .into_iter()
        .filter(|row| governed.contains(&row.key.as_str()))
        .filter(|row| rows.iter().any(|page_row| page_row.key == row.key))
        .map(|row| (row.key, row.value))
        .collect();
    u5_bevy_setup::PresetChoice {
        key: preset.key().to_string(),
        label: preset.label().to_string(),
        values,
    }
}

/// Store what the Enhancements page changed in `config.toml`, key by key,
/// through the same sink the menu overlay will use. The preset goes to
/// `enhancements.preset`; a number choice is stored as a number.
#[cfg(feature = "visual")]
pub fn save_wizard_settings(
    config: &Path,
    changes: &u5_bevy_setup::SettingsChanges,
) -> Result<(), String> {
    use u5_bevy_setup::RowValue;
    use u5_runtime::enhancements::{
        CatalogueRow, OptionKind, PRESET_KEY, SettingValue, SettingsSink,
    };

    let mut stored = Vec::new();
    if let Some(preset) = &changes.preset {
        stored.push((PRESET_KEY.to_string(), SettingValue::Str(preset.clone())));
    }
    for (key, value) in &changes.values {
        let value = match value {
            RowValue::Switch(flag) => SettingValue::Bool(*flag),
            RowValue::Choice(text) => match CatalogueRow::find(key) {
                Some(CatalogueRow::Option(spec)) if matches!(spec.kind, OptionKind::Int { .. }) => {
                    text.parse()
                        .map(SettingValue::Int)
                        .unwrap_or_else(|_| SettingValue::Str(text.clone()))
                }
                _ => SettingValue::Str(text.clone()),
            },
        };
        stored.push((key.clone(), value));
    }
    if stored.is_empty() {
        return Ok(());
    }
    crate::enhancement_settings::ConfigSink::new(config).store(&stored)
}

fn version_name(label: Option<&str>) -> String {
    match label {
        Some(label) => match known_version(label) {
            Some(version) => version.name.to_string(),
            None => "a release this engine was not verified against".to_string(),
        },
        None => "not checked".to_string(),
    }
}

fn commit_message(ready: &ReadyGameData) -> String {
    let mut text = format!(
        "Using the game files from {} ({}). The game runs from its own copy in {}, where \
         your saved games are kept.",
        ready.source_dir.display(),
        version_name(ready.version.as_deref()),
        ready.data_dir.display()
    );
    for note in &ready.notes {
        text.push(' ');
        text.push_str(note);
    }
    text
}

/// The terminal setup wizard.
///
/// Lists what discovery found, takes a number or a folder path, shows the
/// validation report, and on confirmation installs the data and saves the
/// choice. `Ok(None)` when the player quits (or input ends).
pub fn run_terminal_wizard(
    host: &DesktopHost,
    need: &WizardNeed,
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> io::Result<Option<ReadyGameData>> {
    let mut candidates: Vec<Discovered> = need.candidates.clone();
    writeln!(output, "Ultima V setup")?;
    writeln!(output, "==============")?;
    writeln!(output, "{}", need.reason)?;
    writeln!(
        output,
        "The engine needs the data files from your own copy of Ultima V. They are only \
         read; setup copies them into the engine's data folder."
    )?;
    loop {
        writeln!(output)?;
        if candidates.is_empty() {
            writeln!(output, "No game folders were found automatically.")?;
        } else {
            writeln!(output, "Folders found:")?;
            for (index, found) in candidates.iter().enumerate() {
                writeln!(
                    output,
                    "  {}) {}\n     {} - {}",
                    index + 1,
                    found.data_root.display(),
                    found.candidate.source,
                    found.report.status_line()
                )?;
            }
        }
        writeln!(
            output,
            "Enter a number, the path of the game folder, 'r' to search again, 'h' for how \
             to get the files, or 'q' to quit."
        )?;
        write!(output, "> ")?;
        output.flush()?;
        let Some(answer) = read_answer(input)? else {
            writeln!(output)?;
            return Ok(None);
        };
        let path = match answer.as_str() {
            "" => continue,
            "q" | "Q" | "quit" | "exit" => return Ok(None),
            "h" | "H" | "help" | "?" => {
                writeln!(output)?;
                write!(output, "{HOW_TO_GET_THE_FILES}")?;
                continue;
            }
            "r" | "R" => {
                candidates = host.discover();
                continue;
            }
            text => match text.parse::<usize>() {
                Ok(number) if (1..=candidates.len()).contains(&number) => {
                    candidates[number - 1].data_root.clone()
                }
                Ok(_) => {
                    writeln!(output, "There is no folder with that number.")?;
                    continue;
                }
                Err(_) => expand_path(text, host.env.home.as_deref()),
            },
        };
        let report = host.validate(&path);
        writeln!(output)?;
        write!(output, "{}", report.summary())?;
        if !report.is_usable() {
            writeln!(
                output,
                "Choose another folder, or 'h' for how to get the files."
            )?;
            continue;
        }
        write!(output, "Use this folder? [Y/n] ")?;
        output.flush()?;
        let Some(confirm) = read_answer(input)? else {
            writeln!(output)?;
            return Ok(None);
        };
        if !(confirm.is_empty()
            || confirm.eq_ignore_ascii_case("y")
            || confirm.eq_ignore_ascii_case("yes"))
        {
            continue;
        }
        match host.commit(&report.dir, Origin::Setup) {
            Ok(ready) => {
                writeln!(output, "{}", commit_message(&ready))?;
                writeln!(
                    output,
                    "Saved in {}. Run with --setup to change it.",
                    host.paths.config_file.display()
                )?;
                output.flush()?;
                return Ok(Some(ready));
            }
            Err(error) => {
                writeln!(output, "Setup failed: {error}")?;
            }
        }
    }
}

fn read_answer(input: &mut dyn BufRead) -> io::Result<Option<String>> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    Ok(Some(line.trim().to_string()))
}

/// Accept pasted paths: strip quotes, expand a leading `~`.
pub fn expand_path(text: &str, home: Option<&Path>) -> PathBuf {
    let trimmed = text
        .trim()
        .trim_matches(|ch| ch == '"' || ch == '\'')
        .to_string();
    if let Some(home) = home {
        if trimmed == "~" {
            return home.to_path_buf();
        }
        if let Some(rest) = trimmed.strip_prefix("~/") {
            return home.join(rest);
        }
    }
    PathBuf::from(trimmed)
}

/// An error from a run, rendered for people: the message itself (never
/// Rust's debug form), plus a plain report of what is missing when the
/// game folder turns out to be incomplete.
pub fn describe_failure(error: &io::Error, game_data: Option<&ReadyGameData>) -> String {
    let mut text = error.to_string();
    let Some(game_data) = game_data else {
        return text;
    };
    let report = match &game_data.incomplete {
        Some(report) => (**report).clone(),
        None => u5_gamedata::validate_dir(&game_data.data_dir),
    };
    if report.is_usable() {
        if error.kind() == io::ErrorKind::NotFound
            && !text.contains(&*game_data.data_dir.to_string_lossy())
        {
            text.push_str(&format!(
                " (while using the game folder {})",
                game_data.data_dir.display()
            ));
        }
        return text;
    }
    text.push_str("\n\n");
    text.push_str(&report.summary());
    text.push_str("Run `u5-engine --setup` to choose a complete game folder.");
    text
}

/// The no-argument launch: resolve the game data (with the windowed wizard
/// when needed) and start the windowed game. Shared by `u5-engine` (no
/// arguments, visual build) and `u5-engine-gui`.
#[cfg(feature = "visual")]
pub fn run_default_launch(
    force_setup: bool,
    cli: &[u5_runtime::enhancements::CliSetting],
) -> Result<(), String> {
    let ready = resolve_game_data(None, force_setup, WizardMode::Window)?;
    let game_dir = crate::prepare_writable_game_dir(&ready.data_dir)
        .map_err(|error| describe_failure(&error, Some(&ready)))?;
    // The default launch reads config.toml (Phase B design 1.2.4), with
    // `cli` on top.
    let settings = crate::enhancement_settings::launch_enhancements(
        crate::enhancement_settings::SettingsFile::Read,
        cli,
        false,
    );
    let launch = crate::enhancement_settings::visual_launch_config(settings, &game_dir);
    // `display.raster` feeds the 1.0.1 launch parameter (design 2.4.1).
    let depth = crate::enhancement_settings::launch_raster_depth(
        u5_runtime::TileGraphicsDepth::Ega16,
        &launch.enhancements,
    );
    u5_bevy::run_visual_intro_loop_with(&game_dir, depth, launch)
        .map(|_exit| ())
        .map_err(|error| describe_failure(&error, Some(&ready)))
}

/// Entry point of the `u5-engine-gui` binary (no console on Windows).
///
/// Errors cannot be printed there, so they also go to a native message box
/// and to the log file in the data folder.
#[cfg(feature = "visual")]
pub fn run_gui_main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some(SETUP_WINDOW_FLAG) {
        return run_setup_window_child(args.next());
    }
    if first.as_deref() == Some(SETTINGS_WINDOW_FLAG) {
        return run_settings_window_child(&args.collect::<Vec<_>>());
    }
    // `--settings` is the one generic flag this binary takes (design
    // 1.2.4): the Enhancements page first, then the game.
    if first.as_deref() == Some("--settings") {
        match open_settings_page(false) {
            Ok(true) => {}
            Ok(false) => return ExitCode::SUCCESS,
            Err(message) => {
                report_gui_error(&message);
                return ExitCode::FAILURE;
            }
        }
    }
    let force_setup = first.as_deref() == Some("--setup");
    // This binary takes no settings flags; the stored settings apply.
    match run_default_launch(force_setup, &[]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            report_gui_error(&message);
            ExitCode::FAILURE
        }
    }
}

/// Show an error to a player who may have no console: stderr, the log file
/// and a native message box.
#[cfg(feature = "visual")]
pub fn report_gui_error(message: &str) {
    eprintln!("Error: {message}");
    let host = DesktopHost::from_process();
    u5_host_desktop::append_log(&host.paths, message);
    u5_host_desktop::dialogs::show_error_dialog(
        "Ultima V",
        &format!(
            "{message}\n\n(Also written to {}.)",
            host.paths.log_file().display()
        ),
    );
}
