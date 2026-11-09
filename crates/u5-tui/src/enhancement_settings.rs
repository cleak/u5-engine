//! Enhancement settings on the desktop host: the `config.toml` adapter, the
//! command-line settings, the parity lock and the startup line (Phase B
//! design 1.2.4, 1.2.8, 1.3.3 and 1.3.11).
//!
//! The engine never reads `config.toml`, the environment or the command
//! line for enhancements (`u5_runtime::enhancements`). This module is the
//! desktop side of that contract:
//!
//! - [`TomlSource`] and [`ConfigSink`] adapt a [`ConfigDocument`] to the
//!   engine's [`SettingsSource`] and [`SettingsSink`]. Only sectioned keys
//!   are enhancement settings; the top-level keys belong to
//!   [`u5_host_desktop::Config`].
//! - [`check_cli_key`] and [`find_cli_alias`] back the generic flags of
//!   `cli.rs` (`--enable`, `--disable`, `--set` and the catalogue aliases),
//!   so a mistyped key stops the launch like any unknown option.
//! - [`lock_state`] decides the parity lock: `--parity-lock`,
//!   `U5_PARITY_LOCK=1`, or either parity-only variable
//!   (`U5_FIXED_HOST_CLOCK`, `U5_EMULATOR_LOAD_LATENCY_MS`). QA modes never
//!   get here at all: they never resolve.
//! - [`launch_enhancements`] resolves once per launch and prints the one
//!   startup line, [`PARITY_LOCK_LINE`] under the lock, which the paired
//!   harness requires in every `engine.log`.
//!
//! Only the default launch (and `u5-engine-gui`, which is that launch) reads
//! the file ([`SettingsFile::Read`]); every other launch resolves from the
//! command line alone (1.2.4).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use toml_edit::{Array, Item, Value};
use u5_host_desktop::{ConfigDocument, DesktopPaths, HostEnv};
use u5_runtime::enhancements::{
    ApplyAt, CatalogueRow, CliSetting, HostCapabilities, LockReason, LockState, OPTIONS,
    OptionDefault, OptionKind, PRESET_KEY, PresetLabel, ResolveInput, ResolvedEnhancements,
    SettingValue, SettingsSink, SettingsSource, resolve,
};

/// `U5_PARITY_LOCK=1` forces every enhancement off (1.3.11).
pub const PARITY_LOCK_ENV: &str = "U5_PARITY_LOCK";

/// Variables that exist only for parity runs (D1.4); either one being set
/// locks too.
pub const PARITY_ONLY_ENVS: [&str; 2] = [
    u5_runtime::FIXED_HOST_CLOCK_ENV,
    "U5_EMULATOR_LOAD_LATENCY_MS",
];

/// The startup line of a locked process. `qa/tools/paired_suite.py`
/// requires exactly this line in each run's `engine.log`.
pub const PARITY_LOCK_LINE: &str = "enhancements: parity-locked (all off)";

const STARTUP_PREFIX: &str = "enhancements:";
const CLASSIC_WORD: &str = "classic";
const CUSTOM_WORD: &str = "custom";
const DAMAGED_FILE: &str = "is not valid TOML; using classic settings";
const UNREADABLE_FILE: &str = "could not be read; using classic settings";

/// Whether a launch reads the stored settings (design 1.2.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsFile {
    /// The default launch and `u5-engine-gui`: `config.toml`, then the
    /// command line.
    Read,
    /// Every other launch: the command line alone (`stored: None`).
    Ignore,
}

/// `config.toml`'s enhancement sections as a [`SettingsSource`].
pub struct TomlSource<'a> {
    doc: &'a ConfigDocument,
}

impl<'a> TomlSource<'a> {
    pub fn new(doc: &'a ConfigDocument) -> Self {
        Self { doc }
    }
}

impl SettingsSource for TomlSource<'_> {
    fn get(&self, key: &str) -> Option<SettingValue> {
        if !is_sectioned(key) {
            return None;
        }
        self.doc.get(key).and_then(setting_value)
    }

    fn keys(&self) -> Vec<String> {
        // Top-level keys (`game_dir`, `data_dir`, `version`) are the
        // game-data config's, not enhancement settings.
        self.doc
            .keys()
            .into_iter()
            .filter(|key| is_sectioned(key))
            .collect()
    }
}

fn is_sectioned(key: &str) -> bool {
    key.contains('.')
}

/// A stored item as the engine's value type. Types the catalogue never uses
/// (floats, dates, tables) come through as text, so the resolver reports
/// them as bad values instead of the key silently vanishing.
fn setting_value(item: &Item) -> Option<SettingValue> {
    match item {
        Item::Value(value) => Some(value_to_setting(value)),
        Item::Table(table) => Some(SettingValue::Str(table.to_string().trim().to_string())),
        Item::ArrayOfTables(array) => Some(SettingValue::Str(array.to_string().trim().to_string())),
        Item::None => None,
    }
}

fn value_to_setting(value: &Value) -> SettingValue {
    match value {
        Value::Boolean(flag) => SettingValue::Bool(*flag.value()),
        Value::Integer(number) => SettingValue::Int(*number.value()),
        Value::String(text) => SettingValue::Str(text.value().clone()),
        Value::Array(items) => SettingValue::List(
            items
                .iter()
                .map(|item| match item {
                    Value::String(text) => text.value().clone(),
                    other => other.to_string().trim().to_string(),
                })
                .collect(),
        ),
        other => SettingValue::Str(other.to_string().trim().to_string()),
    }
}

/// The engine's value as a TOML value, for [`ConfigSink`].
pub fn toml_value(value: &SettingValue) -> Value {
    match value {
        SettingValue::Bool(flag) => Value::from(*flag),
        SettingValue::Int(number) => Value::from(*number),
        SettingValue::Str(text) => Value::from(text.as_str()),
        SettingValue::List(items) => {
            Value::Array(items.iter().map(String::as_str).collect::<Array>())
        }
    }
}

/// Changed settings written into `config.toml` key by key. Every other key,
/// section and comment is kept, and a key the player edited by hand while
/// the game ran survives (`ConfigDocument::save_if_changed`).
#[derive(Clone, Debug)]
pub struct ConfigSink {
    path: PathBuf,
}

impl ConfigSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SettingsSink for ConfigSink {
    fn store(&self, changes: &[(String, SettingValue)]) -> Result<(), String> {
        let failed = |error: io::Error| {
            format!(
                "could not save settings to {}: {}",
                self.path.display(),
                u5_gamedata::friendly_io_error(&error)
            )
        };
        let mut doc = ConfigDocument::load(&self.path).map_err(failed)?;
        for (key, value) in changes {
            doc.set(key, toml_value(value));
        }
        doc.save_if_changed().map(|_| ()).map_err(failed)
    }
}

/// The parity lock for this launch. `var` reads one environment variable;
/// tests pass a table, since no test may set the process environment.
pub fn lock_state(parity_lock_flag: bool, var: impl Fn(&str) -> Option<OsString>) -> LockState {
    if parity_lock_flag {
        return LockState::Locked(LockReason::CliFlag);
    }
    if var(PARITY_LOCK_ENV).is_some_and(|value| !value.is_empty() && value != "0") {
        return LockState::Locked(LockReason::EnvVar);
    }
    if PARITY_ONLY_ENVS.iter().any(|name| var(name).is_some()) {
        return LockState::Locked(LockReason::QaEnv);
    }
    LockState::Off
}

/// [`lock_state`] over the process environment.
pub fn lock_state_for_process(parity_lock_flag: bool) -> LockState {
    lock_state(parity_lock_flag, |name| std::env::var_os(name))
}

/// A launch's resolved settings, with what the host has to say about its
/// own file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchSettings {
    pub resolved: ResolvedEnhancements,
    /// The file this launch read, if it read one.
    pub config: Option<PathBuf>,
    /// Host-level warnings, such as a damaged file, one line each.
    pub warnings: Vec<String>,
}

/// Resolves the settings for one launch. Under the lock the file is not
/// even opened (1.3.11).
pub fn resolve_launch(file: Option<&Path>, cli: &[CliSetting], lock: LockState) -> LaunchSettings {
    let mut warnings = Vec::new();
    let mut config = None;
    let doc = match (file, lock) {
        (Some(path), LockState::Off) => {
            config = Some(path.to_path_buf());
            match ConfigDocument::load(path) {
                Ok(doc) if doc.damaged().is_some() => {
                    warnings.push(format!("{} {DAMAGED_FILE}", path.display()));
                    None
                }
                Ok(doc) => Some(doc),
                Err(error) => {
                    warnings.push(format!(
                        "{} {UNREADABLE_FILE} ({})",
                        path.display(),
                        u5_gamedata::friendly_io_error(&error)
                    ));
                    None
                }
            }
        }
        _ => None,
    };
    let source = doc.as_ref().map(TomlSource::new);
    let resolved = resolve(ResolveInput {
        stored: source.as_ref().map(|source| source as &dyn SettingsSource),
        cli,
        lock,
        caps: HostCapabilities::ALL,
    });
    LaunchSettings {
        resolved,
        config,
        warnings,
    }
}

/// The one startup line (1.3.11): the lock line, `enhancements: classic`,
/// or the preset (or `custom`) with every toggle that is on, cheat keys
/// included.
pub fn startup_line(resolved: &ResolvedEnhancements) -> String {
    if let LockState::Locked(_) = resolved.lock {
        return PARITY_LOCK_LINE.to_string();
    }
    if resolved.is_classic() {
        return format!("{STARTUP_PREFIX} {CLASSIC_WORD}");
    }
    let label = match resolved.preset {
        PresetLabel::Preset(preset) => preset.key(),
        PresetLabel::Custom(_) => CUSTOM_WORD,
    };
    let on: Vec<&str> = resolved
        .toggles
        .iter_on()
        .map(|toggle| toggle.key())
        .collect();
    format!("{STARTUP_PREFIX} {label} ({})", on.join(", "))
}

/// Everything a launch prints to stderr: the startup line, then each
/// warning and note on its own line.
pub fn startup_report(settings: &LaunchSettings) -> Vec<String> {
    let mut lines = vec![startup_line(&settings.resolved)];
    lines.extend(
        settings
            .warnings
            .iter()
            .map(|warning| format!("{STARTUP_PREFIX} {warning}")),
    );
    lines.extend(
        settings
            .resolved
            .notes
            .iter()
            .map(|note| format!("{STARTUP_PREFIX} {note}")),
    );
    lines
}

/// Resolves this launch's settings, prints the startup report to stderr and
/// returns the result. Called by the play launches only: QA modes, route
/// smoke and the suites never resolve.
pub fn launch_enhancements(
    file: SettingsFile,
    cli: &[CliSetting],
    parity_lock_flag: bool,
) -> ResolvedEnhancements {
    let lock = lock_state_for_process(parity_lock_flag);
    let env = HostEnv::from_process();
    let path = u5_host_desktop::paths::config_file(&env);
    let file = match file {
        SettingsFile::Read => Some(path.as_path()),
        SettingsFile::Ignore => None,
    };
    let settings = resolve_launch(file, cli, lock);
    if !settings.warnings.is_empty() {
        let paths = DesktopPaths::from_env(&env);
        for warning in &settings.warnings {
            u5_host_desktop::append_log(&paths, &format!("{STARTUP_PREFIX} {warning}"));
        }
    }
    for line in startup_report(&settings) {
        eprintln!("{line}");
    }
    settings.resolved
}

/// `--show-settings`: the settings the default launch would use, with this
/// command line, printed to stdout.
pub fn run_show_settings(cli: &[CliSetting], parity_lock_flag: bool) -> io::Result<()> {
    let lock = lock_state_for_process(parity_lock_flag);
    let path = u5_host_desktop::paths::config_file(&HostEnv::from_process());
    let settings = resolve_launch(Some(&path), cli, lock);
    print!("{}", show_settings_text(&path, &settings));
    Ok(())
}

/// The `--show-settings` report (1.2.4).
pub fn show_settings_text(path: &Path, settings: &LaunchSettings) -> String {
    let resolved = &settings.resolved;
    let config_state = if settings.config.is_none() {
        " (not read: parity lock)"
    } else if !path.exists() {
        " (no file)"
    } else {
        ""
    };
    let lock = match resolved.lock {
        LockState::Off => "off".to_string(),
        LockState::Locked(reason) => format!("on ({})", lock_reason_text(reason)),
    };
    let mut text = format!(
        "config: {}{config_state}\npreset: {}\nlock:   {lock}\non:\n",
        path.display(),
        resolved.preset
    );
    let on = changed_settings(resolved);
    if on.is_empty() {
        text.push_str("  (none)\n");
    }
    for (setting, apply) in on {
        text.push_str(&format!("  {setting:<36} {apply}\n"));
    }
    let notes: Vec<String> = settings
        .warnings
        .iter()
        .cloned()
        .chain(resolved.notes.iter().map(ToString::to_string))
        .collect();
    if !notes.is_empty() {
        text.push_str("notes:\n");
        for note in notes {
            text.push_str(&format!("  {note}\n"));
        }
    }
    text
}

fn lock_reason_text(reason: LockReason) -> &'static str {
    match reason {
        LockReason::CliFlag => "--parity-lock",
        LockReason::EnvVar => "U5_PARITY_LOCK",
        LockReason::QaEnv => "a parity-run variable is set",
        LockReason::QaMode => "QA mode",
    }
}

fn apply_word(apply: ApplyAt) -> &'static str {
    match apply {
        ApplyAt::Frame => "frame",
        ApplyAt::IdlePrompt => "prompt",
        ApplyAt::NextLaunch => "launch",
    }
}

/// Every toggle that is on and every valued setting or chord away from its
/// default, as `key = value` with its apply point. The preset is shown on
/// its own line, so it is left out here.
fn changed_settings(resolved: &ResolvedEnhancements) -> Vec<(String, &'static str)> {
    let mut lines = Vec::new();
    for toggle in resolved.toggles.iter_on() {
        lines.push((
            format!("{} = true", toggle.key()),
            apply_word(toggle.spec().apply),
        ));
    }
    for spec in OPTIONS {
        if spec.key == PRESET_KEY {
            continue;
        }
        let shown = if spec.kind == OptionKind::Chord {
            let chord = resolved.keys.chord(spec.key);
            let default = match spec.default {
                OptionDefault::Str(text) if !text.is_empty() => Some(text),
                _ => None,
            };
            (chord != default).then(|| chord.unwrap_or("(unbound)").to_string())
        } else {
            resolved
                .values
                .get(spec.key)
                .filter(|value| **value != default_setting(spec.default))
                .map(ToString::to_string)
        };
        if let Some(value) = shown {
            lines.push((format!("{} = {value}", spec.key), apply_word(spec.apply)));
        }
    }
    lines
}

fn default_setting(default: OptionDefault) -> SettingValue {
    match default {
        OptionDefault::Bool(flag) => SettingValue::Bool(flag),
        OptionDefault::Int(number) => SettingValue::Int(number),
        OptionDefault::Str(text) => SettingValue::Str(text.to_string()),
        OptionDefault::List(items) => {
            SettingValue::List(items.iter().map(|item| item.to_string()).collect())
        }
    }
}

/// What a catalogue alias stands for on the command line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliAlias {
    /// `--x` (`on`) or `--no-x` (off) for an on/off setting.
    Switch { key: &'static str, on: bool },
    /// `--x VALUE` for a valued setting.
    Valued { key: &'static str },
}

impl CliAlias {
    /// The setting this alias gives, taking the value from `next` when the
    /// alias needs one.
    pub fn setting(
        self,
        flag: &str,
        next: impl FnOnce() -> Option<String>,
    ) -> io::Result<CliSetting> {
        match self {
            CliAlias::Switch { key, on: true } => Ok(CliSetting::Enable(key.to_string())),
            CliAlias::Switch { key, on: false } => Ok(CliSetting::Disable(key.to_string())),
            CliAlias::Valued { key } => {
                let value = next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{flag} requires a value for {key}"),
                    )
                })?;
                Ok(CliSetting::Set {
                    key: key.to_string(),
                    value,
                })
            }
        }
    }
}

/// The catalogue alias `flag` names, if any. Only shipped rows' aliases
/// exist (U13), and `--preset`, which `cli.rs` parses itself, is not
/// returned.
pub fn find_cli_alias(flag: &str) -> Option<CliAlias> {
    for row in CatalogueRow::all().filter(|row| row.is_shipped()) {
        let switch = is_switch(row);
        for alias in row.cli_aliases() {
            if *alias == "--preset" {
                continue;
            }
            if flag == *alias {
                return Some(if switch {
                    CliAlias::Switch {
                        key: row.key(),
                        on: true,
                    }
                } else {
                    CliAlias::Valued { key: row.key() }
                });
            }
            if switch
                && flag
                    .strip_prefix("--no-")
                    .is_some_and(|rest| alias.strip_prefix("--") == Some(rest))
            {
                return Some(CliAlias::Switch {
                    key: row.key(),
                    on: false,
                });
            }
        }
    }
    None
}

/// Help lines for the shipped catalogue aliases, for `--help`. Empty while
/// no shipped row has one besides `--preset`.
pub fn cli_alias_usage() -> String {
    let mut text = String::new();
    for row in CatalogueRow::all().filter(|row| row.is_shipped()) {
        let switch = is_switch(row);
        for alias in row
            .cli_aliases()
            .iter()
            .filter(|alias| **alias != "--preset")
        {
            let usage = if switch {
                format!("{alias}, --no-{}", alias.trim_start_matches("--"))
            } else {
                format!("{alias} <VALUE>")
            };
            let meaning = if switch {
                format!("--enable {} (--no- form: --disable)", row.key())
            } else {
                format!("--set {}=VALUE", row.key())
            };
            text.push_str(&format!("        {usage:<22} {meaning}\n"));
        }
    }
    text
}

fn is_switch(row: CatalogueRow) -> bool {
    match row {
        CatalogueRow::Toggle(_) => true,
        CatalogueRow::Option(spec) => spec.kind == OptionKind::Bool,
    }
}

/// Checks a key given to `--enable`, `--disable` (`switch`) or `--set`
/// against the catalogue. Unknown keys stop the launch, as any unknown
/// option does (1.2.4); a key whose feature has not shipped is accepted
/// and resolved to a note.
pub fn check_cli_key(flag: &str, key: &str, switch: bool) -> io::Result<()> {
    if key == PRESET_KEY && !switch {
        return Ok(());
    }
    let Some(row) = CatalogueRow::find(key) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{flag}: unknown setting `{key}` (see --show-settings and docs/settings.md)"),
        ));
    };
    if switch && !is_switch(row) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{flag}: `{key}` is not an on/off setting; use --set {key}=VALUE"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use u5_runtime::enhancements::{Preset, SettingNote};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-enh-settings-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let table: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), OsString::from(v)))
            .collect();
        move |name| table.get(name).cloned()
    }

    #[test]
    fn lock_triggers_in_order() {
        assert_eq!(lock_state(false, env(&[])), LockState::Off);
        assert_eq!(
            lock_state(true, env(&[])),
            LockState::Locked(LockReason::CliFlag)
        );
        assert_eq!(
            lock_state(false, env(&[("U5_PARITY_LOCK", "1")])),
            LockState::Locked(LockReason::EnvVar)
        );
        assert_eq!(
            lock_state(false, env(&[("U5_PARITY_LOCK", "0")])),
            LockState::Off
        );
        assert_eq!(
            lock_state(false, env(&[("U5_FIXED_HOST_CLOCK", "12:00:00.00")])),
            LockState::Locked(LockReason::QaEnv)
        );
        assert_eq!(
            lock_state(false, env(&[("U5_EMULATOR_LOAD_LATENCY_MS", "280")])),
            LockState::Locked(LockReason::QaEnv)
        );
        // The flag names the reason even when the environment also locks.
        assert_eq!(
            lock_state(true, env(&[("U5_PARITY_LOCK", "1")])),
            LockState::Locked(LockReason::CliFlag)
        );
        assert_eq!(PARITY_ONLY_ENVS[0], "U5_FIXED_HOST_CLOCK");
    }

    #[test]
    fn lock_wins_over_enable_and_skips_the_file() {
        let dir = temp_dir("lock");
        let path = dir.join("config.toml");
        fs::write(&path, "[enhancements]\npreset = \"everything\"\n").unwrap();
        let cli = [
            CliSetting::Enable("enhancements.freeze_overlays".into()),
            CliSetting::Preset(Preset::Everything),
        ];
        let settings = resolve_launch(Some(&path), &cli, LockState::Locked(LockReason::EnvVar));
        assert_eq!(settings.config, None, "the file is not read under the lock");
        assert!(settings.resolved.is_classic());
        assert_eq!(
            settings.resolved.lock,
            LockState::Locked(LockReason::EnvVar)
        );
        assert_eq!(
            settings.resolved.notes,
            [SettingNote::LockIgnored {
                settings: vec![
                    "--enable enhancements.freeze_overlays".into(),
                    "--preset everything".into()
                ]
            }]
        );
        let report = startup_report(&settings);
        assert_eq!(report[0], PARITY_LOCK_LINE);
        assert_eq!(
            report[1],
            "enhancements: parity lock: ignored --enable enhancements.freeze_overlays \
             --preset everything"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unlocked_classic_prints_classic() {
        let settings = resolve_launch(None, &[], LockState::Off);
        assert_eq!(settings.resolved, ResolvedEnhancements::classic());
        assert_eq!(startup_report(&settings), ["enhancements: classic"]);
    }

    #[test]
    fn source_reads_only_sectioned_keys_with_their_types() {
        let dir = temp_dir("source");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "game_dir = \"/g\"\nversion = \"gog-1.0\"\n\n[enhancements]\npreset = \"recommended\"\n\
             schema = 1\n\n[display]\ngrow_left = true\nratio = 1.5\nlist = [\"a\", 2]\n",
        )
        .unwrap();
        let doc = ConfigDocument::load(&path).unwrap();
        let source = TomlSource::new(&doc);
        assert_eq!(
            source.keys(),
            [
                "enhancements.preset",
                "enhancements.schema",
                "display.grow_left",
                "display.ratio",
                "display.list"
            ]
        );
        assert_eq!(source.get("game_dir"), None);
        assert_eq!(
            source.get("enhancements.preset"),
            Some(SettingValue::Str("recommended".into()))
        );
        assert_eq!(
            source.get("enhancements.schema"),
            Some(SettingValue::Int(1))
        );
        assert_eq!(
            source.get("display.grow_left"),
            Some(SettingValue::Bool(true))
        );
        assert_eq!(
            source.get("display.ratio"),
            Some(SettingValue::Str("1.5".into()))
        );
        assert_eq!(
            source.get("display.list"),
            Some(SettingValue::List(vec!["a".into(), "2".into()]))
        );
        assert_eq!(source.get("display.missing"), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_settings_resolve_with_notes() {
        let dir = temp_dir("file");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "game_dir = \"/g\"\n[enhancements]\npreset = \"recommended\"\nschema = 7\n\
             [display]\ngrowleft = true\n",
        )
        .unwrap();
        let settings = resolve_launch(Some(&path), &[], LockState::Off);
        assert_eq!(settings.config.as_deref(), Some(path.as_path()));
        assert!(settings.warnings.is_empty());
        // No feature row has shipped, so Recommended turns nothing on yet.
        assert!(settings.resolved.is_classic());
        assert_eq!(
            settings.resolved.preset,
            PresetLabel::Preset(Preset::Recommended)
        );
        let notes: Vec<String> = settings
            .resolved
            .notes
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(
            notes
                .iter()
                .any(|n| n.starts_with("enhancements.schema = 7")),
            "{notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|n| n.starts_with("display.growleft: unknown key")),
            "{notes:?}"
        );
        assert!(!notes.iter().any(|n| n.contains("game_dir")), "{notes:?}");
        // A command-line `--preset` overrides the file's.
        let cli = [CliSetting::Preset(Preset::Classic)];
        let settings = resolve_launch(Some(&path), &cli, LockState::Off);
        assert_eq!(
            settings.resolved.preset,
            PresetLabel::Preset(Preset::Classic)
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn damaged_or_missing_file_is_classic() {
        let dir = temp_dir("damaged");
        let missing = resolve_launch(Some(&dir.join("none.toml")), &[], LockState::Off);
        assert_eq!(missing.resolved, ResolvedEnhancements::classic());
        assert!(missing.warnings.is_empty());

        let path = dir.join("config.toml");
        fs::write(&path, "[enhancements\npreset = \"everything\"\n").unwrap();
        let damaged = resolve_launch(Some(&path), &[], LockState::Off);
        assert_eq!(damaged.resolved, ResolvedEnhancements::classic());
        assert_eq!(damaged.warnings.len(), 1);
        assert!(damaged.warnings[0].ends_with(DAMAGED_FILE));
        let report = startup_report(&damaged);
        assert_eq!(report[0], "enhancements: classic");
        assert!(report[1].starts_with("enhancements: "));
        // Reading never rewrites the file.
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[enhancements\npreset = \"everything\"\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sink_writes_changed_keys_and_keeps_the_rest() {
        let dir = temp_dir("sink");
        let path = dir.join("config.toml");
        let original = "# mine\ngame_dir = \"/g\"\n\n[automap]\nglyphs = \"x\" # keep\n";
        fs::write(&path, original).unwrap();
        let sink = ConfigSink::new(&path);
        sink.store(&[
            (
                "enhancements.preset".into(),
                SettingValue::Str("recommended".into()),
            ),
            ("display.grow_left".into(), SettingValue::Bool(true)),
            (
                "keys.cheats".into(),
                SettingValue::List(vec!["a".into(), "b".into()]),
            ),
        ])
        .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(original), "{text}");
        let doc = ConfigDocument::load(&path).unwrap();
        let source = TomlSource::new(&doc);
        assert_eq!(
            source.get("enhancements.preset"),
            Some(SettingValue::Str("recommended".into()))
        );
        assert_eq!(
            source.get("display.grow_left"),
            Some(SettingValue::Bool(true))
        );
        assert_eq!(
            source.get("keys.cheats"),
            Some(SettingValue::List(vec!["a".into(), "b".into()]))
        );
        // Storing the same values again changes nothing on disk.
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        sink.store(&[("display.grow_left".into(), SettingValue::Bool(true))])
            .unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sink_failure_is_a_plain_message() {
        let dir = temp_dir("sink-fail");
        // A folder where the file should be cannot be written.
        let path = dir.join("config.toml");
        fs::create_dir_all(&path).unwrap();
        let error = ConfigSink::new(&path)
            .store(&[("display.grow_left".into(), SettingValue::Bool(true))])
            .unwrap_err();
        assert!(error.starts_with("could not save settings to "), "{error}");
        assert!(!error.contains('\n'));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cli_keys_are_checked_against_the_catalogue() {
        assert!(check_cli_key("--enable", "enhancements.freeze_overlays", true).is_ok());
        assert!(check_cli_key("--set", "enhancements.preset", false).is_ok());
        assert!(check_cli_key("--set", "keys.menu", false).is_ok());
        let unknown = check_cli_key("--enable", "display.growleft", true).unwrap_err();
        assert_eq!(unknown.kind(), io::ErrorKind::InvalidInput);
        assert!(
            unknown
                .to_string()
                .contains("unknown setting `display.growleft`")
        );
        let valued = check_cli_key("--enable", "keys.menu", true).unwrap_err();
        assert!(valued.to_string().contains("use --set keys.menu=VALUE"));
        assert!(check_cli_key("--disable", "enhancements.preset", true).is_err());
    }

    #[test]
    fn aliases_come_from_shipped_rows_only() {
        // `--preset` is parsed by `cli.rs` itself, and no other shipped row
        // has an alias yet.
        assert_eq!(find_cli_alias("--preset"), None);
        assert_eq!(find_cli_alias("--no-preset"), None);
        assert_eq!(find_cli_alias("--quest-journal"), None);
        assert_eq!(cli_alias_usage(), "");
        for row in CatalogueRow::all().filter(|row| row.is_shipped()) {
            for alias in row.cli_aliases().iter().filter(|a| **a != "--preset") {
                let found = find_cli_alias(alias).unwrap();
                assert_eq!(
                    found,
                    if is_switch(row) {
                        CliAlias::Switch {
                            key: row.key(),
                            on: true,
                        }
                    } else {
                        CliAlias::Valued { key: row.key() }
                    }
                );
            }
        }
    }

    #[test]
    fn alias_settings() {
        let on = CliAlias::Switch {
            key: "journal.page",
            on: true,
        };
        let off = CliAlias::Switch {
            key: "journal.page",
            on: false,
        };
        let valued = CliAlias::Valued {
            key: "display.scale_mode",
        };
        assert_eq!(
            on.setting("--quest-journal", || None).unwrap(),
            CliSetting::Enable("journal.page".into())
        );
        assert_eq!(
            off.setting("--no-quest-journal", || None).unwrap(),
            CliSetting::Disable("journal.page".into())
        );
        assert_eq!(
            valued.setting("--scale", || Some("sharp".into())).unwrap(),
            CliSetting::Set {
                key: "display.scale_mode".into(),
                value: "sharp".into()
            }
        );
        assert!(valued.setting("--scale", || None).is_err());
    }

    #[test]
    fn show_settings_report() {
        let path = Path::new("/nonexistent/u5-engine/config.toml");
        let settings = resolve_launch(Some(path), &[], LockState::Off);
        assert_eq!(
            show_settings_text(path, &settings),
            "config: /nonexistent/u5-engine/config.toml (no file)\npreset: classic\n\
             lock:   off\non:\n  (none)\n"
        );
        let cli = [CliSetting::Set {
            key: "keys.menu".into(),
            value: "F9".into(),
        }];
        let locked = resolve_launch(Some(path), &cli, LockState::Locked(LockReason::CliFlag));
        assert_eq!(
            show_settings_text(path, &locked),
            "config: /nonexistent/u5-engine/config.toml (not read: parity lock)\n\
             preset: classic\nlock:   on (--parity-lock)\non:\n  (none)\nnotes:\n  \
             parity lock: ignored --set keys.menu=F9\n"
        );
    }

    #[test]
    fn toml_values_round_trip() {
        for value in [
            SettingValue::Bool(true),
            SettingValue::Int(-3),
            SettingValue::Str("Ctrl+M".into()),
            SettingValue::List(vec!["x".into(), "y".into()]),
        ] {
            assert_eq!(value_to_setting(&toml_value(&value)), value);
        }
    }
}
