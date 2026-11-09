//! The host contract: where stored settings come from and go to, what the
//! host can do, the parity lock, and the notes resolution produces
//! (Phase B design 1.3.3, 1.3.11 and 1.3.14).
//!
//! The engine never reads `config.toml`, the environment or the command
//! line. A host implements [`SettingsSource`] and [`SettingsSink`] over its
//! own store (the desktop host over `config.toml`, a phone port over its
//! platform preferences), decodes its command line into [`CliSetting`]s,
//! decides the [`LockState`] and describes itself with
//! [`HostCapabilities`]. [`resolve`](super::resolve::resolve) turns those
//! into [`ResolvedEnhancements`](super::resolve::ResolvedEnhancements).

use std::fmt;

use super::catalogue::Preset;

/// A stored or resolved setting value. A host converts its own store's
/// values into these; colours, paths, enum words and chords are text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<String>),
}

impl fmt::Display for SettingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingValue::Bool(value) => write!(f, "{value}"),
            SettingValue::Int(value) => write!(f, "{value}"),
            SettingValue::Str(text) if text.is_empty() => f.write_str("\"\""),
            SettingValue::Str(text) => f.write_str(text),
            SettingValue::List(items) => write!(f, "[{}]", items.join(", ")),
        }
    }
}

/// Where stored settings come from. Desktop: `config.toml`. A phone port:
/// its platform preferences store.
pub trait SettingsSource {
    /// The value stored under the dotted key `section.name`, if any.
    fn get(&self, key: &str) -> Option<SettingValue>;
    /// Every key present, for unknown-key notes.
    fn keys(&self) -> Vec<String>;
}

/// Where changed settings go. Called with changed keys only.
pub trait SettingsSink: Send + Sync {
    fn store(&self, changes: &[(String, SettingValue)]) -> Result<(), String>;
}

/// One setting from the command line, in command-line order. The host's
/// parser has already rejected unknown flags and keys (design 1.2.4) and
/// turned every catalogue alias into one of these forms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliSetting {
    /// `--preset NAME`.
    Preset(Preset),
    /// `--enable KEY`: a boolean key set to true.
    Enable(String),
    /// `--disable KEY`: a boolean key set to false.
    Disable(String),
    /// `--set KEY=VALUE`: any key, the value as typed.
    Set { key: String, value: String },
}

impl fmt::Display for CliSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliSetting::Preset(preset) => write!(f, "--preset {}", preset.key()),
            CliSetting::Enable(key) => write!(f, "--enable {key}"),
            CliSetting::Disable(key) => write!(f, "--disable {key}"),
            CliSetting::Set { key, value } => write!(f, "--set {key}={value}"),
        }
    }
}

/// Whether the parity lock holds (design 1.3.11). Read once at launch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockState {
    #[default]
    Off,
    Locked(LockReason),
}

/// Why the parity lock holds. The reason only labels the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockReason {
    /// `--parity-lock`.
    CliFlag,
    /// `U5_PARITY_LOCK=1`.
    EnvVar,
    /// `U5_FIXED_HOST_CLOCK` or `U5_EMULATOR_LOAD_LATENCY_MS`, which exist
    /// only for parity runs.
    QaEnv,
    /// A QA mode: route smoke, the frame and route suites, and the like.
    QaMode,
}

/// Something a catalogue row needs from the host (design 1.3.14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Windows that can go fullscreen.
    Windowed,
    Keyboard,
    /// F1 to F24.
    FunctionKeys,
    Pointer,
    Gamepad,
}

impl Capability {
    /// Plain-language name for notes.
    pub fn label(self) -> &'static str {
        match self {
            Capability::Windowed => "a window",
            Capability::Keyboard => "a keyboard",
            Capability::FunctionKeys => "function keys",
            Capability::Pointer => "a mouse or pointer",
            Capability::Gamepad => "a gamepad",
        }
    }
}

/// What the host can do. The resolver forces off, with a note, every
/// setting that needs something the host lacks, so settings pages can hide
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostCapabilities {
    pub windowed: bool,
    pub keyboard: bool,
    pub function_keys: bool,
    pub pointer: bool,
    pub gamepad: bool,
}

impl HostCapabilities {
    /// A host that can do everything: the desktop host.
    pub const ALL: Self = Self {
        windowed: true,
        keyboard: true,
        function_keys: true,
        pointer: true,
        gamepad: true,
    };

    pub fn has(self, capability: Capability) -> bool {
        match capability {
            Capability::Windowed => self.windowed,
            Capability::Keyboard => self.keyboard,
            Capability::FunctionKeys => self.function_keys,
            Capability::Pointer => self.pointer,
            Capability::Gamepad => self.gamepad,
        }
    }
}

/// Why a value was not used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueProblem {
    NotOnOff,
    NotWholeNumber,
    OutOfRange { min: i64, max: i64 },
    NotOneOf(&'static [&'static str]),
    NotColour,
    NotText,
    NotList,
    ItemNotOneOf { item: String },
}

const NOT_ON_OFF: &str = "not true or false";
const NOT_WHOLE_NUMBER: &str = "not a whole number";
const NOT_COLOUR: &str = "not a colour like #rrggbb";
const NOT_TEXT: &str = "not text";
const NOT_LIST: &str = "not a list";

impl fmt::Display for ValueProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueProblem::NotOnOff => f.write_str(NOT_ON_OFF),
            ValueProblem::NotWholeNumber => f.write_str(NOT_WHOLE_NUMBER),
            ValueProblem::OutOfRange { min, max } => write!(f, "not in {min}..{max}"),
            ValueProblem::NotOneOf(words) => write!(f, "not one of {}", words.join(", ")),
            ValueProblem::NotColour => f.write_str(NOT_COLOUR),
            ValueProblem::NotText => f.write_str(NOT_TEXT),
            ValueProblem::NotList => f.write_str(NOT_LIST),
            ValueProblem::ItemNotOneOf { item } => write!(f, "{item} is not a choice"),
        }
    }
}

const UNKNOWN_KEY: &str = "unknown key, ignored";
const NOT_AVAILABLE: &str = "not available in this build";
const NOT_A_SWITCH: &str = "not an on/off setting, ignored (use --set)";
const CHEATS_NOT_ALLOWED: &str = "ignored, cheats are not allowed (cheats.enabled is off)";
const LOCK_IGNORED: &str = "parity lock: ignored";

/// One problem found while resolving. Resolution never fails: every
/// problem becomes a note, and the setting concerned keeps its default.
/// Notes go to the log and to `--show-settings`, one line each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingNote {
    /// A stored key the catalogue does not have, with the closest
    /// available key when one is near.
    UnknownKey {
        key: String,
        hint: Option<&'static str>,
    },
    /// A key whose feature has not shipped in this build (U13). A stored
    /// value is kept by the host; the default is used.
    NotAvailable { key: &'static str },
    /// A value of the wrong type or out of range; the default is used.
    BadValue {
        key: &'static str,
        value: String,
        problem: ValueProblem,
        default: String,
    },
    /// `--enable` or `--disable` named a valued setting.
    NotASwitch { key: &'static str },
    /// A cheat key without `cheats.enabled`; it stays off.
    CheatsNotAllowed { key: &'static str },
    /// The host lacks what the setting needs; it is off (or unbound).
    Unsupported {
        key: &'static str,
        value: Option<String>,
        missing: Capability,
    },
    /// Command-line settings ignored because the parity lock holds.
    LockIgnored { settings: Vec<String> },
}

impl SettingNote {
    /// The key the note is about, if it is about one key.
    pub fn key(&self) -> Option<&str> {
        match self {
            SettingNote::UnknownKey { key, .. } => Some(key),
            SettingNote::NotAvailable { key }
            | SettingNote::BadValue { key, .. }
            | SettingNote::NotASwitch { key }
            | SettingNote::CheatsNotAllowed { key }
            | SettingNote::Unsupported { key, .. } => Some(key),
            SettingNote::LockIgnored { .. } => None,
        }
    }
}

impl fmt::Display for SettingNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingNote::UnknownKey { key, hint } => {
                write!(f, "{key}: {UNKNOWN_KEY}")?;
                if let Some(hint) = hint {
                    write!(f, " (did you mean {hint}?)")?;
                }
                Ok(())
            }
            SettingNote::NotAvailable { key } => write!(f, "{key}: {NOT_AVAILABLE}"),
            SettingNote::BadValue {
                key,
                value,
                problem,
                default,
            } => write!(f, "{key} = {value}: {problem}, using {default}"),
            SettingNote::NotASwitch { key } => write!(f, "{key}: {NOT_A_SWITCH}"),
            SettingNote::CheatsNotAllowed { key } => write!(f, "{key}: {CHEATS_NOT_ALLOWED}"),
            SettingNote::Unsupported {
                key,
                value,
                missing,
            } => {
                match value {
                    Some(value) => write!(f, "{key} = {value}")?,
                    None => f.write_str(key)?,
                }
                write!(
                    f,
                    ": needs {}, which this device does not have; turned off",
                    missing.label()
                )
            }
            SettingNote::LockIgnored { settings } => {
                write!(f, "{LOCK_IGNORED} {}", settings.join(" "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_read_as_one_plain_line() {
        let notes = [
            SettingNote::UnknownKey {
                key: "display.growleft".into(),
                hint: Some("display.grow_left"),
            },
            SettingNote::BadValue {
                key: "journal.spoiler_tier",
                value: "7".into(),
                problem: ValueProblem::OutOfRange { min: 1, max: 4 },
                default: "2".into(),
            },
            SettingNote::Unsupported {
                key: "keys.menu",
                value: Some("F12".into()),
                missing: Capability::FunctionKeys,
            },
            SettingNote::LockIgnored {
                settings: vec!["--enable journal.page".into()],
            },
        ];
        let lines: Vec<String> = notes.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "display.growleft: unknown key, ignored (did you mean display.grow_left?)",
                "journal.spoiler_tier = 7: not in 1..4, using 2",
                "keys.menu = F12: needs function keys, which this device does not have; \
                 turned off",
                "parity lock: ignored --enable journal.page",
            ]
        );
        assert!(lines.iter().all(|line| !line.contains('\n')));
        assert_eq!(notes[0].key(), Some("display.growleft"));
        assert_eq!(notes[3].key(), None);
    }

    #[test]
    fn cli_settings_print_as_typed() {
        assert_eq!(
            CliSetting::Preset(Preset::Recommended).to_string(),
            "--preset recommended"
        );
        assert_eq!(
            CliSetting::Set {
                key: "display.scale_mode".into(),
                value: "sharp".into()
            }
            .to_string(),
            "--set display.scale_mode=sharp"
        );
    }

    #[test]
    fn all_capabilities_means_every_capability() {
        for capability in [
            Capability::Windowed,
            Capability::Keyboard,
            Capability::FunctionKeys,
            Capability::Pointer,
            Capability::Gamepad,
        ] {
            assert!(HostCapabilities::ALL.has(capability));
            assert!(!capability.label().is_empty());
        }
        assert_eq!(LockState::default(), LockState::Off);
    }
}
