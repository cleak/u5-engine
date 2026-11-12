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
//!   harness requires in every `engine.log`. `visual_launch_config` turns
//!   the result into the shell's `VisualLaunchConfig`.
//!
//! Only the default launch (and `u5-engine-gui`, which is that launch) reads
//! the file ([`SettingsFile::Read`]); every other launch resolves from the
//! command line alone (1.2.4).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use toml_edit::{Array, Item, Value};
use u5_host_desktop::{ConfigDocument, DesktopPaths, HostEnv};
use u5_runtime::TileGraphicsDepth;
use u5_runtime::enhancements::display::{BEZEL_IMAGE_KEY, PILLAR_FILL_KEY, RASTER_WORDS};
use u5_runtime::enhancements::{
    ApplyAt, BEZEL_MAX_SIDE, BezelDecode, BezelImage, BezelMailbox, CatalogueRow, CliSetting,
    HostCapabilities, LockReason, LockState, OPTIONS, OptionDefault, OptionKind, PRESET_KEY,
    PresetLabel, ResolveInput, ResolvedEnhancements, SettingValue, SettingsSink, SettingsSource,
    raster_word, resolve,
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
    bezel_updates: Option<BezelMailbox>,
}

impl ConfigSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            bezel_updates: None,
        }
    }

    /// Also decodes a bezel chosen in play and posts it to `updates`
    /// (design 2.3.3), so the engine never opens the file.
    pub fn with_bezel_updates(mut self, updates: BezelMailbox) -> Self {
        self.bezel_updates = Some(updates);
        self
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// After a change to the fill or the bezel path: when the fill is a
    /// bezel, decode the stored path and post the picture, or why it could
    /// not be used.
    fn post_bezel(&self, doc: &ConfigDocument, changes: &[(String, SettingValue)]) {
        let Some(updates) = &self.bezel_updates else {
            return;
        };
        let touched = changes
            .iter()
            .any(|(key, _)| key == PILLAR_FILL_KEY || key == BEZEL_IMAGE_KEY);
        let text = |key: &str| match doc.get(key).and_then(setting_value) {
            Some(SettingValue::Str(text)) => text,
            _ => String::new(),
        };
        if !touched || text(PILLAR_FILL_KEY) != "bezel" {
            return;
        }
        let path = bezel_path(&text(BEZEL_IMAGE_KEY), self.path.parent());
        updates.post(match decode_bezel_file(&path) {
            Ok(image) => BezelDecode::Image(image),
            Err(reason) => BezelDecode::Failed(reason),
        });
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
        self.post_bezel(&doc, changes);
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
) -> LaunchSettings {
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
    settings
}

/// The tile graphics a play launch draws with (design 2.4.1): the
/// `--raster-depth` the command line gave, unless the resolved
/// `display.raster` chose another set. `--raster-depth` is that setting's
/// alias; both of its words load the 16-colour tiles in this build.
pub fn launch_raster_depth(
    command_line: TileGraphicsDepth,
    resolved: &ResolvedEnhancements,
) -> TileGraphicsDepth {
    let word = raster_word(resolved);
    if Some(word) == RASTER_WORDS.first().copied() {
        return command_line;
    }
    TileGraphicsDepth::from_key(word).unwrap_or(command_line)
}

/// A bezel picture's path as stored: an empty path is no picture, and a
/// relative one is taken from the folder of the file that stores it (the
/// current folder when the launch read no file).
fn bezel_path(stored: &str, base: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(stored.trim());
    match base {
        Some(base) if path.is_relative() && !stored.trim().is_empty() => base.join(path),
        _ => path,
    }
}

/// Decodes `display.bezel_image` (design 2.2.11, 2.3.3): PNG only, at most
/// [`BEZEL_MAX_SIDE`] pixels on a side, into straight RGBA. The error is a
/// sentence for the player's log.
pub fn decode_bezel(bytes: &[u8]) -> Result<BezelImage, String> {
    use image::ImageDecoder;
    let decoder = image::codecs::png::PngDecoder::new(io::Cursor::new(bytes))
        .map_err(|_| "the bezel picture is not a PNG file".to_string())?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err("the bezel picture is empty".to_string());
    }
    if width > BEZEL_MAX_SIDE || height > BEZEL_MAX_SIDE {
        return Err(format!(
            "the bezel picture is {width}x{height}; at most {BEZEL_MAX_SIDE} pixels on a side \
             can be used"
        ));
    }
    let image = image::DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("the bezel picture could not be decoded ({error})"))?;
    let rgba = image.into_rgba8().into_raw();
    Ok(BezelImage {
        width,
        height,
        rgba: rgba.into(),
    })
}

/// [`decode_bezel`] for a file.
pub fn decode_bezel_file(path: &Path) -> Result<BezelImage, String> {
    if path.as_os_str().is_empty() {
        return Err("no bezel picture is set (display.bezel_image)".to_string());
    }
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "the bezel picture {} could not be read ({})",
            path.display(),
            u5_gamedata::friendly_io_error(&error)
        )
    })?;
    decode_bezel(&bytes)
}

/// The bezel a launch shows (design 2.3.3): decoded when the fill is a
/// bezel, `None` otherwise. A picture that cannot be used gives `None`,
/// which the engine shows as black bars, and a warning.
pub fn launch_bezel(resolved: &ResolvedEnhancements, config: Option<&Path>) -> LaunchBezel {
    if resolved.values.str(PILLAR_FILL_KEY) != Some("bezel") {
        return LaunchBezel::default();
    }
    let stored = resolved.values.str(BEZEL_IMAGE_KEY).unwrap_or_default();
    match decode_bezel_file(&bezel_path(stored, config.and_then(Path::parent))) {
        Ok(image) => LaunchBezel {
            image: Some(image),
            warning: None,
        },
        Err(reason) => LaunchBezel {
            image: None,
            warning: Some(format!("{STARTUP_PREFIX} {reason}; the bars stay black")),
        },
    }
}

/// What [`launch_bezel`] made of the bezel setting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchBezel {
    pub image: Option<BezelImage>,
    /// The line to print when the picture could not be used.
    pub warning: Option<String>,
}

/// The shell's launch config for one play launch (design 1.3.3).
///
/// Settings changed in play are stored back into the file this launch read,
/// and only when it read it cleanly: never under the lock (the file is not
/// even opened), never for a launch that does not read the file, and never
/// over a damaged or unreadable file. The save folder is the writable game
/// folder the launch runs from.
#[cfg(feature = "visual")]
pub fn visual_launch_config(
    settings: LaunchSettings,
    save_dir: &Path,
) -> u5_bevy::VisualLaunchConfig {
    let bezel = launch_bezel(&settings.resolved, settings.config.as_deref());
    if let Some(warning) = &bezel.warning {
        eprintln!("{warning}");
    }
    let bezel_updates = BezelMailbox::new();
    let settings_sink = settings
        .config
        .filter(|_| settings.warnings.is_empty() && settings.resolved.lock == LockState::Off)
        .map(|path| {
            std::sync::Arc::new(ConfigSink::new(path).with_bezel_updates(bezel_updates.clone()))
                as std::sync::Arc<dyn SettingsSink>
        });
    u5_bevy::VisualLaunchConfig {
        bezel_updates: settings_sink.as_ref().map(|_| bezel_updates),
        enhancements: settings.resolved,
        settings_sink,
        save_dir: Some(save_dir.to_path_buf()),
        bezel: bezel.image,
    }
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

/// Catalogue aliases that `cli.rs` parses itself: `--preset`, and the
/// existing `--raster-depth`, which is `display.raster`'s alias and stays
/// the 1.0.1 launch parameter (design 2.4.1).
pub const SELF_PARSED_ALIASES: &[&str] = &["--preset", "--raster-depth"];

/// The catalogue alias `flag` names, if any. Only shipped rows' aliases
/// exist (U13), and the [`SELF_PARSED_ALIASES`] are not returned.
pub fn find_cli_alias(flag: &str) -> Option<CliAlias> {
    find_cli_alias_in(CatalogueRow::all().filter(|row| row.is_shipped()), flag)
}

/// [`find_cli_alias`] over `rows`.
pub fn find_cli_alias_in(
    rows: impl IntoIterator<Item = CatalogueRow>,
    flag: &str,
) -> Option<CliAlias> {
    for row in rows {
        let switch = is_switch(row);
        for alias in row.cli_aliases() {
            if SELF_PARSED_ALIASES.contains(alias) {
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
            .filter(|alias| !SELF_PARSED_ALIASES.contains(alias))
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

/// A valued row that `--enable` and `--disable` still accept, because the
/// catalogue gives it the values they set (D-M2.3), such as
/// `display.brightness`. Its alias stays valued (`--brightness 70`).
fn has_switch_value(row: CatalogueRow) -> bool {
    matches!(row, CatalogueRow::Option(spec) if spec.switch.is_some())
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
    if switch && !is_switch(row) && !has_switch_value(row) {
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
    use u5_runtime::enhancements::{Preset, SettingNote, Toggle};

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
        // Of Recommended's rows only `display.sprite_composite` (chapter 9
        // P5) has shipped, so it is the one toggle the preset turns on.
        assert_eq!(
            settings.resolved.toggles.iter_on().collect::<Vec<_>>(),
            [Toggle::DisplaySpriteComposite]
        );
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
        // A valued toggle with a switch value takes `--enable` and
        // `--disable` (D-M2.3); one without still refuses.
        assert!(check_cli_key("--enable", "display.brightness", true).is_ok());
        assert!(check_cli_key("--disable", "display.brightness", true).is_ok());
        assert!(check_cli_key("--set", "display.brightness", false).is_ok());
        assert!(check_cli_key("--enable", "display.vignette", true).is_err());
        assert!(check_cli_key("--enable", "display.brightness_scope", true).is_err());
        assert!(check_cli_key("--disable", "enhancements.preset", true).is_err());
    }

    #[test]
    fn aliases_come_from_shipped_rows_only() {
        // `--preset` is parsed by `cli.rs` itself; the display rows of
        // chapter 2 P1, P2 and P8, chapter 9 P1's `--brightness` and P5's
        // `--sprite-composite` are the only shipped rows with aliases so far.
        assert_eq!(find_cli_alias("--preset"), None);
        assert_eq!(find_cli_alias("--no-preset"), None);
        assert_eq!(find_cli_alias("--quest-journal"), None);
        assert_eq!(
            find_cli_alias("--grow-left"),
            Some(CliAlias::Switch {
                key: "display.grow_left",
                on: true
            })
        );
        assert_eq!(
            cli_alias_usage()
                .lines()
                .map(|line| line.split_whitespace().next().unwrap())
                .collect::<Vec<_>>(),
            // Toggles first, then valued settings, each in catalogue order.
            [
                "--fullscreen,",
                "--fullscreen-key,",
                "--grow-left,",
                "--sprite-composite,",
                "--scale-mode",
                "--pixel-aspect",
                "--pillar-fill",
                "--bezel-image",
                "--vsync,",
                "--brightness"
            ]
        );
        for row in CatalogueRow::all().filter(|row| row.is_shipped()) {
            // `--preset` and `--raster-depth` are parsed by `cli.rs` itself.
            for alias in row
                .cli_aliases()
                .iter()
                .filter(|a| !SELF_PARSED_ALIASES.contains(a) && **a != "--preset")
            {
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

    /// The `[display]` aliases of design 2.3.3. The P1 rows (scale,
    /// fullscreen, vsync) shipped in M1-03, the fill in M1-04 and the wider
    /// view in M1-16; each parses exactly when its row has shipped.
    #[test]
    fn display_aliases_parse_as_the_design_lists_them() {
        let display = || CatalogueRow::all().filter(|row| row.key().starts_with("display."));
        for (flag, alias) in [
            (
                "--scale-mode",
                CliAlias::Valued {
                    key: "display.scale_mode",
                },
            ),
            (
                "--pixel-aspect",
                CliAlias::Valued {
                    key: "display.pixel_aspect",
                },
            ),
            (
                "--pillar-fill",
                CliAlias::Valued {
                    key: "display.pillar_fill",
                },
            ),
            (
                "--bezel-image",
                CliAlias::Valued {
                    key: "display.bezel_image",
                },
            ),
            (
                "--fullscreen",
                CliAlias::Switch {
                    key: "display.fullscreen",
                    on: true,
                },
            ),
            (
                "--no-fullscreen",
                CliAlias::Switch {
                    key: "display.fullscreen",
                    on: false,
                },
            ),
            (
                "--fullscreen-key",
                CliAlias::Switch {
                    key: "display.fullscreen_key",
                    on: true,
                },
            ),
            (
                "--no-fullscreen-key",
                CliAlias::Switch {
                    key: "display.fullscreen_key",
                    on: false,
                },
            ),
            (
                "--grow-left",
                CliAlias::Switch {
                    key: "display.grow_left",
                    on: true,
                },
            ),
            (
                "--no-grow-left",
                CliAlias::Switch {
                    key: "display.grow_left",
                    on: false,
                },
            ),
            (
                "--vsync",
                CliAlias::Switch {
                    key: "display.vsync",
                    on: true,
                },
            ),
            (
                "--no-vsync",
                CliAlias::Switch {
                    key: "display.vsync",
                    on: false,
                },
            ),
            // Chapter 9 (9.4.1), hidden until their phases ship.
            (
                "--brightness",
                CliAlias::Valued {
                    key: "display.brightness",
                },
            ),
            (
                "--vignette",
                CliAlias::Valued {
                    key: "display.vignette",
                },
            ),
            (
                "--sprite-composite",
                CliAlias::Switch {
                    key: "display.sprite_composite",
                    on: true,
                },
            ),
            (
                "--no-sprite-composite",
                CliAlias::Switch {
                    key: "display.sprite_composite",
                    on: false,
                },
            ),
        ] {
            assert_eq!(find_cli_alias_in(display(), flag), Some(alias), "{flag}");
            let key = match alias {
                CliAlias::Valued { key } | CliAlias::Switch { key, .. } => key,
            };
            let shipped = CatalogueRow::find(key).unwrap().is_shipped();
            assert_eq!(
                find_cli_alias(flag),
                shipped.then_some(alias),
                "{flag} parses exactly when its row has shipped"
            );
        }
        // Valued keys have no `--no-` form; `--raster-depth` stays cli.rs's
        // own flag.
        for flag in [
            "--no-scale-mode",
            "--no-pillar-fill",
            "--raster-depth",
            "--no-brightness",
            "--no-vignette",
        ] {
            assert_eq!(find_cli_alias_in(display(), flag), None, "{flag}");
        }
        assert!(SELF_PARSED_ALIASES.contains(&"--raster-depth"));
    }

    /// Chapter 10's `--gem-map` and `--no-gem-map` (10.2.1, 10.4.1): the
    /// main toggle's alias, the only one of `[gem_map]`. Hidden until G2
    /// ships, so it parses only through the full catalogue.
    #[test]
    fn the_gem_map_alias_parses_once_its_row_ships() {
        let gem = || CatalogueRow::all().filter(|row| row.key().starts_with("gem_map."));
        let shipped = CatalogueRow::find("gem_map.enhanced").unwrap().is_shipped();
        for (flag, on) in [("--gem-map", true), ("--no-gem-map", false)] {
            let alias = CliAlias::Switch {
                key: "gem_map.enhanced",
                on,
            };
            assert_eq!(find_cli_alias_in(gem(), flag), Some(alias), "{flag}");
            assert_eq!(find_cli_alias(flag), shipped.then_some(alias), "{flag}");
        }
        // No sub-option has an alias; the generic flags reach them.
        let aliases: Vec<&str> = gem()
            .flat_map(|row| row.cli_aliases().iter().copied())
            .collect();
        assert_eq!(aliases, ["--gem-map"]);
        for flag in ["--gem-map-labels", "--no-gem-map-labels", "--gem-map-png"] {
            assert_eq!(find_cli_alias_in(gem(), flag), None, "{flag}");
        }
        // The usage lists it exactly when its row has shipped.
        assert_eq!(cli_alias_usage().contains("--gem-map"), shipped);
    }

    #[test]
    fn display_raster_feeds_the_launch_parameter() {
        use u5_runtime::enhancements::{DISPLAY_P1, resolve_as_shipped};
        let resolved = |cli: &[CliSetting]| {
            resolve_as_shipped(
                ResolveInput {
                    stored: None,
                    cli,
                    lock: LockState::Off,
                    caps: HostCapabilities::ALL,
                },
                &[DISPLAY_P1],
            )
        };
        let classic = ResolvedEnhancements::classic();
        assert_eq!(
            launch_raster_depth(TileGraphicsDepth::Ega16, &classic),
            TileGraphicsDepth::Ega16
        );
        let tandy = resolved(&[CliSetting::Set {
            key: "display.raster".into(),
            value: "tandy".into(),
        }]);
        assert!(tandy.notes.is_empty());
        assert!(
            tandy.is_classic(),
            "raster is a launch parameter, not a toggle"
        );
        // Both words draw the 16-colour tiles in this build.
        assert_eq!(
            launch_raster_depth(TileGraphicsDepth::Ega16, &tandy),
            TileGraphicsDepth::Ega16
        );
        assert_eq!(
            launch_raster_depth(TileGraphicsDepth::Ega16, &resolved(&[])),
            TileGraphicsDepth::Ega16
        );
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

    #[cfg(feature = "visual")]
    #[test]
    fn launch_config_stores_only_into_a_cleanly_read_file() {
        let dir = temp_dir("launch-config");
        let save_dir = dir.join("data");
        let path = dir.join("config.toml");

        // The default launch with a missing file: classic, and settings
        // changed in play go back to that file.
        let launch =
            visual_launch_config(resolve_launch(Some(&path), &[], LockState::Off), &save_dir);
        assert!(launch.enhancements.is_classic());
        assert!(launch.settings_sink.is_some());
        assert_eq!(launch.save_dir.as_deref(), Some(save_dir.as_path()));

        // A launch that does not read the file stores nothing.
        let launch = visual_launch_config(resolve_launch(None, &[], LockState::Off), &save_dir);
        assert!(launch.settings_sink.is_none());
        assert_eq!(launch.save_dir.as_deref(), Some(save_dir.as_path()));

        // The lock never opens the file and never stores.
        let locked = resolve_launch(Some(&path), &[], LockState::Locked(LockReason::EnvVar));
        let launch = visual_launch_config(locked, &save_dir);
        assert!(launch.settings_sink.is_none());
        assert!(!launch.wants_enhancements());

        // A damaged file is never written over from play.
        fs::write(&path, "this is [not toml").unwrap();
        let damaged = resolve_launch(Some(&path), &[], LockState::Off);
        assert!(!damaged.warnings.is_empty());
        let launch = visual_launch_config(damaged, &save_dir);
        assert!(launch.settings_sink.is_none());
        assert!(launch.enhancements.is_classic());
        assert_eq!(fs::read_to_string(&path).unwrap(), "this is [not toml");
        fs::remove_dir_all(&dir).unwrap();
    }

    // --- The bezel (design 2.2.11, 2.3.3; build plan M1-04). -------------

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = image::RgbaImage::new(width, height);
        for (x, y, pixel) in pixels.enumerate_pixels_mut() {
            *pixel = image::Rgba([x as u8, y as u8, 0x40, 0xff]);
        }
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(pixels)
            .write_to(&mut io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn a_png_bezel_decodes_to_rgba() {
        let image = decode_bezel(&png(3, 2)).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.rgba.len(), 3 * 2 * 4);
        // Pixel (2, 1).
        assert_eq!(
            &image.rgba[(3 + 2) * 4..(3 + 2) * 4 + 4],
            &[2, 1, 0x40, 0xff]
        );
    }

    #[test]
    fn a_bezel_that_cannot_be_used_is_refused() {
        // Not a PNG.
        assert!(decode_bezel(b"GIF89a not a png").is_err());
        assert!(decode_bezel(&[]).is_err());
        // Larger than 8192 on a side, either way.
        for (width, height) in [(BEZEL_MAX_SIDE + 1, 1), (1, BEZEL_MAX_SIDE + 1)] {
            let error = decode_bezel(&png(width, height)).unwrap_err();
            assert!(error.contains("8192"), "{error}");
        }
        assert!(decode_bezel(&png(BEZEL_MAX_SIDE, 1)).is_ok());
        // A missing file, and no file at all.
        let dir = temp_dir("bezel-missing");
        assert!(decode_bezel_file(&dir.join("nope.png")).is_err());
        assert!(decode_bezel_file(Path::new("")).is_err());
    }

    fn bezel_launch(cli: &[CliSetting]) -> ResolvedEnhancements {
        resolve_launch(None, cli, LockState::Off).resolved
    }

    fn set(key: &str, value: &str) -> CliSetting {
        CliSetting::Set {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn the_launch_decodes_the_bezel_only_for_a_bezel_fill() {
        let dir = temp_dir("bezel-launch");
        let path = dir.join("bezel.png");
        fs::write(&path, png(4, 4)).unwrap();
        let path_text = path.to_string_lossy().into_owned();

        // Another fill never opens the file.
        let resolved = bezel_launch(&[
            set("display.pillar_fill", "ribbon"),
            set("display.bezel_image", &path_text),
        ]);
        assert_eq!(launch_bezel(&resolved, None), LaunchBezel::default());

        let resolved = bezel_launch(&[
            set("display.pillar_fill", "bezel"),
            set("display.bezel_image", &path_text),
        ]);
        let bezel = launch_bezel(&resolved, None);
        assert_eq!(bezel.warning, None);
        assert_eq!(
            bezel.image.map(|image| (image.width, image.height)),
            Some((4, 4))
        );

        // A relative path is taken from the settings file's folder.
        let config = dir.join("config.toml");
        let resolved = bezel_launch(&[
            set("display.pillar_fill", "bezel"),
            set("display.bezel_image", "bezel.png"),
        ]);
        assert!(launch_bezel(&resolved, Some(&config)).image.is_some());

        // Missing, not a PNG, or no path: black, with a warning.
        fs::write(dir.join("text.png"), "plain text").unwrap();
        for stored in ["missing.png", "text.png", ""] {
            let resolved = bezel_launch(&[
                set("display.pillar_fill", "bezel"),
                set("display.bezel_image", stored),
            ]);
            let bezel = launch_bezel(&resolved, Some(&config));
            assert_eq!(bezel.image, None, "{stored}");
            let warning = bezel.warning.unwrap();
            assert!(warning.starts_with("enhancements: "), "{warning}");
            assert!(warning.ends_with("the bars stay black"), "{warning}");
        }
    }

    #[test]
    fn the_sink_decodes_a_bezel_chosen_in_play() {
        let dir = temp_dir("bezel-sink");
        fs::write(dir.join("frame.png"), png(8, 2)).unwrap();
        let config = dir.join("config.toml");
        fs::write(&config, "[display]\npillar_fill = \"bezel\"\n").unwrap();
        let mailbox = BezelMailbox::new();
        let sink = ConfigSink::new(&config).with_bezel_updates(mailbox.clone());

        // A change that touches neither key posts nothing.
        sink.store(&[("display.vsync".into(), SettingValue::Bool(false))])
            .unwrap();
        assert_eq!(mailbox.take(), None);

        sink.store(&[(
            "display.bezel_image".into(),
            SettingValue::Str("frame.png".into()),
        )])
        .unwrap();
        match mailbox.take() {
            Some(BezelDecode::Image(image)) => assert_eq!((image.width, image.height), (8, 2)),
            other => panic!("{other:?}"),
        }
        sink.store(&[(
            "display.bezel_image".into(),
            SettingValue::Str("gone.png".into()),
        )])
        .unwrap();
        assert!(matches!(mailbox.take(), Some(BezelDecode::Failed(_))));

        // With another fill chosen, the path is only stored.
        sink.store(&[
            (
                "display.pillar_fill".into(),
                SettingValue::Str("ribbon".into()),
            ),
            (
                "display.bezel_image".into(),
                SettingValue::Str("frame.png".into()),
            ),
        ])
        .unwrap();
        assert_eq!(mailbox.take(), None);
        let stored = fs::read_to_string(&config).unwrap();
        assert!(stored.contains("bezel_image = \"frame.png\""), "{stored}");
    }
}
