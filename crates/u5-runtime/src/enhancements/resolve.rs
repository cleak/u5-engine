//! Pure resolution of enhancement settings (Phase B design 1.2.3, 1.2.4,
//! 1.2.8, 1.3.2 and 1.3.3).
//!
//! [`resolve`] turns what the host hands over (a [`SettingsSource`], the
//! command line as [`CliSetting`]s, the [`LockState`] and the
//! [`HostCapabilities`]) into [`ResolvedEnhancements`], the only thing the
//! engine is given. It is total: it never fails, and every problem becomes
//! a [`SettingNote`] while the setting concerned keeps its default.
//!
//! Precedence, lowest first: built-in defaults, the stored
//! `enhancements.preset`, the other stored keys, `--preset`, then
//! `--enable`, `--disable` and `--set` in command-line order, and last the
//! parity lock. After that, cheat keys without `cheats.enabled` and
//! settings the host cannot honour are forced back to their defaults.
//!
//! A preset sets the keys the presets name and nothing else: applying one
//! puts every key any preset names to what that preset says, or to its
//! default, so `--preset classic` clears a stored preset's toggles but
//! keeps preferences such as `audio.volume`.

use std::fmt;

use super::catalogue::{EffectClass, FeatureToggles};
use super::catalogue::{
    OPTIONS, OptionDefault, OptionKind, OptionSpec, PRESETS, Phase, Preset, PresetEntry,
    PresetValue, SHIPPED_PHASES, TOGGLES, Tier, ToggleSpec,
};
use super::source::{
    Capability, CliSetting, HostCapabilities, LockReason, LockState, SettingNote, SettingValue,
    SettingsSource, ValueProblem,
};

/// The stored key that names the preset.
pub const PRESET_KEY: &str = "enhancements.preset";

/// The section switch every state-altering cheat needs (1.2.3). Cheat keys
/// that only show state, such as `cheats.state_watch`, do not (U7).
pub const CHEATS_SWITCH: &str = "cheats.enabled";

/// The preset shown for the resolved settings: a preset's name when the
/// toggles equal that preset exactly, otherwise Custom with the preset it
/// started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetLabel {
    Preset(Preset),
    Custom(Preset),
}

impl PresetLabel {
    /// The preset the settings started from.
    pub fn base(self) -> Preset {
        match self {
            PresetLabel::Preset(preset) | PresetLabel::Custom(preset) => preset,
        }
    }

    pub fn is_custom(self) -> bool {
        matches!(self, PresetLabel::Custom(_))
    }
}

impl fmt::Display for PresetLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetLabel::Preset(preset) => f.write_str(preset.key()),
            PresetLabel::Custom(base) => write!(f, "custom (based on {})", base.key()),
        }
    }
}

/// Every valued setting except the `[keys]` chords, typed, in catalogue
/// order. A hidden row holds its default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingValues(Vec<(&'static str, SettingValue)>);

impl SettingValues {
    pub fn get(&self, key: &str) -> Option<&SettingValue> {
        self.0
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, value)| value)
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            SettingValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            SettingValue::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            SettingValue::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn list(&self, key: &str) -> Option<&[String]> {
        match self.get(key)? {
            SettingValue::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &SettingValue)> {
        self.0.iter().map(|(key, value)| (*key, value))
    }
}

/// The `[keys]` chords, in catalogue order. `None` is unbound: the host
/// cannot type the chord. The input router (P5) parses the text and
/// refuses collisions; this table only carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBindings(Vec<(&'static str, Option<String>)>);

impl KeyBindings {
    /// The chord bound to `key` (`keys.menu`, ...), if bound.
    pub fn chord(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| *k == key)
            .and_then(|(_, chord)| chord.as_deref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, Option<&str>)> {
        self.0.iter().map(|(key, chord)| (*key, chord.as_deref()))
    }
}

/// Everything the engine is given. Built once per launch on the host side;
/// the engine never reads files or the environment for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedEnhancements {
    pub toggles: FeatureToggles,
    pub values: SettingValues,
    pub keys: KeyBindings,
    pub preset: PresetLabel,
    pub lock: LockState,
    /// For the log and `--show-settings`.
    pub notes: Vec<SettingNote>,
}

impl ResolvedEnhancements {
    /// Every key at its default: the 1.0.1 game.
    pub fn classic() -> Self {
        Catalogue::BUILT_IN.classic(LockState::Off)
    }

    /// Classic under the parity lock.
    pub fn locked(reason: LockReason) -> Self {
        Catalogue::BUILT_IN.classic(LockState::Locked(reason))
    }

    /// Whether no toggle is on. Valued settings that change anything map
    /// to toggles, so this sees them too.
    pub fn is_classic(&self) -> bool {
        self.toggles.is_classic()
    }
}

/// What [`resolve`] reads.
pub struct ResolveInput<'a> {
    /// `None` for launches that do not read the settings file (1.2.4).
    pub stored: Option<&'a dyn SettingsSource>,
    pub cli: &'a [CliSetting],
    pub lock: LockState,
    pub caps: HostCapabilities,
}

/// Resolves the settings against the built-in catalogue. Never fails.
pub fn resolve(input: ResolveInput<'_>) -> ResolvedEnhancements {
    Catalogue::BUILT_IN.resolve(input)
}

/// The catalogue's test override (build plan, M1): resolves against the
/// built-in catalogue as if `also_shipped` had shipped too, so a feature's
/// tests can reach its rows before the package that unhides them (U13).
/// Hosts never call it.
#[doc(hidden)]
pub fn resolve_as_shipped(input: ResolveInput<'_>, also_shipped: &[Phase]) -> ResolvedEnhancements {
    let shipped: Vec<Phase> = SHIPPED_PHASES.iter().chain(also_shipped).copied().collect();
    Catalogue {
        shipped: &shipped,
        ..Catalogue::BUILT_IN
    }
    .resolve(input)
}

/// The catalogue a resolution runs against: the built-in one, or a test
/// fixture with rows later packages add. A toggle's bit is its position in
/// `toggles`; for the built-in catalogue that is `Toggle as u8`, which the
/// catalogue tests pin, so an option's `toggle` names the bit
/// `toggle as usize` here.
#[derive(Clone, Copy)]
pub(crate) struct Catalogue<'a> {
    pub toggles: &'a [ToggleSpec],
    pub options: &'a [OptionSpec],
    pub presets: &'a [(Preset, &'a [PresetEntry])],
    pub shipped: &'a [Phase],
}

const BUILT_IN_PRESETS: &[(Preset, &[PresetEntry])] = &[
    (Preset::Classic, Preset::Classic.entries()),
    (Preset::Recommended, Preset::Recommended.entries()),
    (Preset::Everything, Preset::Everything.entries()),
];

const _: () = assert!(BUILT_IN_PRESETS.len() == PRESETS.len());

#[derive(Clone, Copy)]
enum Row {
    Toggle(usize),
    Option(usize),
}

impl Catalogue<'static> {
    pub(crate) const BUILT_IN: Catalogue<'static> = Catalogue {
        toggles: TOGGLES,
        options: OPTIONS,
        presets: BUILT_IN_PRESETS,
        shipped: SHIPPED_PHASES,
    };
}

impl<'a> Catalogue<'a> {
    /// The row named `key`. A driven toggle is never found: its valued
    /// setting of the same key is the row.
    fn find(&self, key: &str) -> Option<Row> {
        if let Some(index) = self
            .toggles
            .iter()
            .position(|spec| spec.key == key && !spec.driven)
        {
            return Some(Row::Toggle(index));
        }
        self.options
            .iter()
            .position(|spec| spec.key == key)
            .map(Row::Option)
    }

    fn is_shipped(&self, phase: Phase) -> bool {
        self.shipped.contains(&phase)
    }

    fn row_shipped(&self, row: Row) -> bool {
        match row {
            Row::Toggle(index) => self.is_shipped(self.toggles[index].ships),
            Row::Option(index) => self.is_shipped(self.options[index].ships),
        }
    }

    fn entries(&self, preset: Preset) -> &'a [PresetEntry] {
        self.presets
            .iter()
            .find(|(p, _)| *p == preset)
            .map_or(&[], |(_, entries)| *entries)
    }

    pub(crate) fn classic(&self, lock: LockState) -> ResolvedEnhancements {
        State::new(*self).finish(HostCapabilities::ALL, lock)
    }

    pub(crate) fn resolve(&self, input: ResolveInput<'_>) -> ResolvedEnhancements {
        if let LockState::Locked(_) = input.lock {
            // The lock wins over everything, and the stored settings are
            // not even read (1.3.11).
            let mut resolved = self.classic(input.lock);
            if !input.cli.is_empty() {
                resolved.notes.push(SettingNote::LockIgnored {
                    settings: input.cli.iter().map(ToString::to_string).collect(),
                });
            }
            return resolved;
        }
        let mut state = State::new(*self);
        if let Some(stored) = input.stored {
            state.apply_stored(stored);
        }
        for setting in input.cli {
            state.apply_cli(setting);
        }
        state.finish(input.caps, input.lock)
    }

    /// The closest shipped key to a misspelt one, if any is close.
    fn hint(&self, key: &str) -> Option<&'static str> {
        let candidates = self
            .toggles
            .iter()
            .filter(|spec| !spec.driven && self.is_shipped(spec.ships))
            .map(|spec| spec.key)
            .chain(
                self.options
                    .iter()
                    .filter(|spec| self.is_shipped(spec.ships))
                    .map(|spec| spec.key),
            );
        let limit = (key.chars().count() / 4).clamp(1, 3);
        let mut best: Option<(usize, &'static str)> = None;
        for candidate in candidates {
            let distance = edit_distance(key, candidate);
            if distance <= limit && best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, candidate));
            }
        }
        best.map(|(_, candidate)| candidate)
    }
}

/// A resolution in progress: one value per catalogue row.
struct State<'a> {
    cat: Catalogue<'a>,
    toggles: Vec<bool>,
    options: Vec<SettingValue>,
    base: Preset,
    notes: Vec<SettingNote>,
}

impl<'a> State<'a> {
    fn new(cat: Catalogue<'a>) -> Self {
        Self {
            cat,
            toggles: vec![false; cat.toggles.len()],
            options: cat.options.iter().map(default_value).collect(),
            base: Preset::Classic,
            notes: Vec::new(),
        }
    }

    /// Puts every key any preset names to what `preset` says, or to its
    /// default. Rows that do not exist or have not shipped are skipped.
    fn apply_preset(&mut self, preset: Preset) {
        self.base = preset;
        let cat = self.cat;
        let chosen = cat.entries(preset);
        for (_, entries) in cat.presets {
            for entry in *entries {
                let Some(row) = cat.find(entry.key) else {
                    continue;
                };
                if !cat.row_shipped(row) {
                    continue;
                }
                let value = chosen.iter().find(|e| e.key == entry.key).map(|e| e.value);
                match row {
                    Row::Toggle(index) => {
                        self.toggles[index] = value == Some(PresetValue::On);
                    }
                    Row::Option(index) => {
                        let spec = &cat.options[index];
                        self.options[index] = match value {
                            Some(PresetValue::Choice(word)) => {
                                parse_text(spec, word).unwrap_or_else(|_| default_value(spec))
                            }
                            _ => default_value(spec),
                        };
                    }
                }
            }
        }
    }

    fn apply_stored(&mut self, stored: &dyn SettingsSource) {
        if let Some(value) = stored.get(PRESET_KEY) {
            self.set_preset(value);
        }
        for key in stored.keys() {
            if key == PRESET_KEY {
                continue;
            }
            let Some(row) = self.lookup(&key) else {
                continue;
            };
            if let Some(value) = stored.get(&key) {
                self.set_value(row, value);
            }
        }
    }

    fn apply_cli(&mut self, setting: &CliSetting) {
        match setting {
            CliSetting::Preset(preset) => self.apply_preset(*preset),
            CliSetting::Enable(key) | CliSetting::Disable(key) => {
                let on = matches!(setting, CliSetting::Enable(_));
                let Some(row) = self.lookup(key) else {
                    return;
                };
                match row {
                    Row::Toggle(index) => self.toggles[index] = on,
                    Row::Option(index) if self.cat.options[index].kind == OptionKind::Bool => {
                        self.options[index] = SettingValue::Bool(on);
                    }
                    Row::Option(index) => self.notes.push(SettingNote::NotASwitch {
                        key: self.cat.options[index].key,
                    }),
                }
            }
            CliSetting::Set { key, value } => {
                if key == PRESET_KEY {
                    self.set_preset(SettingValue::Str(value.clone()));
                    return;
                }
                let Some(row) = self.lookup(key) else {
                    return;
                };
                let value = match row {
                    Row::Toggle(_) => bool_text(value),
                    Row::Option(index) => typed_text(self.cat.options[index].kind, value),
                };
                self.set_value(row, value);
            }
        }
    }

    /// The row for `key`, or `None` with a note when there is none or it
    /// has not shipped.
    fn lookup(&mut self, key: &str) -> Option<Row> {
        let Some(row) = self.cat.find(key) else {
            let hint = self.cat.hint(key);
            self.notes.push(SettingNote::UnknownKey {
                key: key.to_string(),
                hint,
            });
            return None;
        };
        if !self.cat.row_shipped(row) {
            let key = match row {
                Row::Toggle(index) => self.cat.toggles[index].key,
                Row::Option(index) => self.cat.options[index].key,
            };
            self.notes.push(SettingNote::NotAvailable { key });
            return None;
        }
        Some(row)
    }

    fn set_preset(&mut self, value: SettingValue) {
        let preset = match &value {
            SettingValue::Str(word) => Preset::from_key(word),
            _ => None,
        };
        match preset {
            Some(preset) => self.apply_preset(preset),
            None => {
                let words = self
                    .cat
                    .options
                    .iter()
                    .find(|spec| spec.key == PRESET_KEY)
                    .and_then(|spec| match spec.kind {
                        OptionKind::Enum(words) => Some(words),
                        _ => None,
                    })
                    .unwrap_or(&[]);
                self.notes.push(SettingNote::BadValue {
                    key: PRESET_KEY,
                    value: value.to_string(),
                    problem: ValueProblem::NotOneOf(words),
                    default: Preset::Classic.key().to_string(),
                });
                self.apply_preset(Preset::Classic);
            }
        }
    }

    /// Sets one row, or its default with a note when the value does not
    /// fit (1.2.3: one bad value never discards the others).
    fn set_value(&mut self, row: Row, value: SettingValue) {
        match row {
            Row::Toggle(index) => match value {
                SettingValue::Bool(on) => self.toggles[index] = on,
                value => {
                    self.notes.push(SettingNote::BadValue {
                        key: self.cat.toggles[index].key,
                        value: value.to_string(),
                        problem: ValueProblem::NotOnOff,
                        default: false.to_string(),
                    });
                    self.toggles[index] = false;
                }
            },
            Row::Option(index) => {
                let spec = &self.cat.options[index];
                self.options[index] = match parse_value(spec, value.clone()) {
                    Ok(value) => value,
                    Err(problem) => {
                        let default = default_value(spec);
                        self.notes.push(SettingNote::BadValue {
                            key: spec.key,
                            value: value.to_string(),
                            problem,
                            default: default.to_string(),
                        });
                        default
                    }
                };
            }
        }
    }

    /// The cheat gate and capability forcing, then the resolved types.
    fn finish(mut self, caps: HostCapabilities, lock: LockState) -> ResolvedEnhancements {
        self.enforce(caps);
        let toggles = self.bits();
        let preset = self.label(toggles, caps);
        let cat = self.cat;
        let mut values = Vec::new();
        let mut keys = Vec::new();
        for (index, spec) in cat.options.iter().enumerate() {
            let value = if spec.key == PRESET_KEY {
                SettingValue::Str(self.base.key().to_string())
            } else {
                self.options[index].clone()
            };
            match (spec.kind, value) {
                (OptionKind::Chord, SettingValue::Str(chord)) => {
                    let bound = !chord.is_empty() && chord_supported(spec, &chord, caps);
                    keys.push((spec.key, bound.then_some(chord)));
                }
                (_, value) => values.push((spec.key, value)),
            }
        }
        ResolvedEnhancements {
            toggles,
            values: SettingValues(values),
            keys: KeyBindings(keys),
            preset,
            lock,
            notes: self.notes,
        }
    }

    fn enforce(&mut self, caps: HostCapabilities) {
        let cat = self.cat;
        let cheats_allowed = match cat.find(CHEATS_SWITCH) {
            Some(Row::Toggle(index)) => self.toggles[index],
            Some(Row::Option(index)) => self.options[index] == SettingValue::Bool(true),
            None => false,
        };
        for (index, spec) in cat.toggles.iter().enumerate() {
            if !self.toggles[index] {
                continue;
            }
            if !cheats_allowed && is_gated_cheat(spec.key, Some(spec.class), spec.tier) {
                self.toggles[index] = false;
                self.notes
                    .push(SettingNote::CheatsNotAllowed { key: spec.key });
            } else if let Some(missing) = spec.needs.iter().copied().find(|need| !caps.has(*need)) {
                self.toggles[index] = false;
                self.notes.push(SettingNote::Unsupported {
                    key: spec.key,
                    value: None,
                    missing,
                });
            }
        }
        for (index, spec) in cat.options.iter().enumerate() {
            let default = default_value(spec);
            if self.options[index] == default {
                // Defaults, chords included, are never noted: a host that
                // cannot type F12 leaves `keys.menu` unbound quietly, and
                // its settings pages hide the row.
                continue;
            }
            let missing = spec.needs.iter().copied().find(|need| !caps.has(*need));
            let missing = missing.or_else(|| match &self.options[index] {
                SettingValue::Str(chord)
                    if spec.kind == OptionKind::Chord && !chord_supported(spec, chord, caps) =>
                {
                    Some(Capability::FunctionKeys)
                }
                _ => None,
            });
            if !cheats_allowed && is_gated_cheat(spec.key, spec.class, spec.tier) {
                self.options[index] = default;
                self.notes
                    .push(SettingNote::CheatsNotAllowed { key: spec.key });
            } else if let Some(missing) = missing {
                // An unusable chord is unbound, not put back to a default
                // the player did not choose.
                let forced = if spec.kind == OptionKind::Chord {
                    SettingValue::Str(String::new())
                } else {
                    default
                };
                let value = std::mem::replace(&mut self.options[index], forced);
                self.notes.push(SettingNote::Unsupported {
                    key: spec.key,
                    value: Some(value.to_string()),
                    missing,
                });
            }
        }
    }

    /// One bit per toggle that is on, plus the toggle of every shipped
    /// valued setting away from its default.
    fn bits(&self) -> FeatureToggles {
        let mut bits = FeatureToggles::CLASSIC;
        for (index, on) in self.toggles.iter().enumerate() {
            if *on {
                bits.set_bit(index);
            }
        }
        for (index, spec) in self.cat.options.iter().enumerate() {
            if let Some(toggle) = spec.toggle
                && self.cat.is_shipped(spec.ships)
                && self.options[index] != default_value(spec)
            {
                bits.set_bit(toggle as usize);
            }
        }
        bits
    }

    /// The preset whose toggles these are, the base preset first, or
    /// Custom. Each preset is resolved on the same host, so a toggle the
    /// host cannot honour does not turn a preset into Custom.
    fn label(&self, toggles: FeatureToggles, caps: HostCapabilities) -> PresetLabel {
        let preset_bits = |preset: Preset| {
            let mut state = State::new(self.cat);
            state.apply_preset(preset);
            state.enforce(caps);
            state.bits()
        };
        std::iter::once(self.base)
            .chain(self.cat.presets.iter().map(|(preset, _)| *preset))
            .find(|preset| preset_bits(*preset) == toggles)
            .map_or(PresetLabel::Custom(self.base), PresetLabel::Preset)
    }
}

/// Whether a key is a state-altering cheat behind [`CHEATS_SWITCH`].
fn is_gated_cheat(key: &str, class: Option<EffectClass>, tier: Tier) -> bool {
    key.starts_with("cheats.")
        && key != CHEATS_SWITCH
        && tier == Tier::Cheat
        && class == Some(EffectClass::StateAltering)
}

/// Whether the host can type `chord` for this `[keys]` row.
fn chord_supported(spec: &OptionSpec, chord: &str, caps: HostCapabilities) -> bool {
    spec.needs.iter().all(|need| caps.has(*need))
        && (caps.function_keys || !names_function_key(chord))
}

/// Whether a chord such as `Ctrl+F12` ends in F1 to F24.
fn names_function_key(chord: &str) -> bool {
    let last = chord.rsplit('+').next().unwrap_or(chord).trim();
    let Some(number) = last.strip_prefix('F').or_else(|| last.strip_prefix('f')) else {
        return false;
    };
    number.parse::<u8>().is_ok_and(|n| (1..=24).contains(&n))
}

fn default_value(spec: &OptionSpec) -> SettingValue {
    match spec.default {
        OptionDefault::Bool(value) => SettingValue::Bool(value),
        OptionDefault::Int(value) => SettingValue::Int(value),
        OptionDefault::Str(text) => SettingValue::Str(text.to_string()),
        OptionDefault::List(items) => {
            SettingValue::List(items.iter().map(|item| item.to_string()).collect())
        }
    }
}

/// A stored value checked against its row's kind.
fn parse_value(spec: &OptionSpec, value: SettingValue) -> Result<SettingValue, ValueProblem> {
    match (spec.kind, value) {
        (OptionKind::Bool, value @ SettingValue::Bool(_)) => Ok(value),
        (OptionKind::Bool, _) => Err(ValueProblem::NotOnOff),
        (OptionKind::Int { min, max }, SettingValue::Int(number)) => {
            if (min..=max).contains(&number) {
                Ok(SettingValue::Int(number))
            } else {
                Err(ValueProblem::OutOfRange { min, max })
            }
        }
        (OptionKind::Int { .. }, _) => Err(ValueProblem::NotWholeNumber),
        (OptionKind::Enum(words), SettingValue::Str(word)) if words.contains(&word.as_str()) => {
            Ok(SettingValue::Str(word))
        }
        (OptionKind::Enum(words), _) => Err(ValueProblem::NotOneOf(words)),
        (OptionKind::Color, SettingValue::Str(text)) if is_colour(&text) => {
            Ok(SettingValue::Str(text))
        }
        (OptionKind::Color, _) => Err(ValueProblem::NotColour),
        (OptionKind::Path | OptionKind::Chord, value @ SettingValue::Str(_)) => Ok(value),
        (OptionKind::Path | OptionKind::Chord, _) => Err(ValueProblem::NotText),
        (OptionKind::List(words), SettingValue::List(items)) => {
            match items.iter().find(|item| !words.contains(&item.as_str())) {
                Some(item) => Err(ValueProblem::ItemNotOneOf { item: item.clone() }),
                None => Ok(SettingValue::List(items)),
            }
        }
        (OptionKind::List(_), _) => Err(ValueProblem::NotList),
    }
}

/// A command-line or preset value, as typed, checked against its row.
fn parse_text(spec: &OptionSpec, text: &str) -> Result<SettingValue, ValueProblem> {
    parse_value(spec, typed_text(spec.kind, text))
}

/// The typed value `text` stands for under `kind`; text that does not
/// parse stays text, so the check reports it.
fn typed_text(kind: OptionKind, text: &str) -> SettingValue {
    match kind {
        OptionKind::Bool => bool_text(text),
        OptionKind::Int { .. } => text
            .trim()
            .parse()
            .map_or_else(|_| SettingValue::Str(text.to_string()), SettingValue::Int),
        OptionKind::List(_) => SettingValue::List(
            text.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect(),
        ),
        OptionKind::Enum(_) | OptionKind::Color | OptionKind::Path | OptionKind::Chord => {
            SettingValue::Str(text.to_string())
        }
    }
}

fn bool_text(text: &str) -> SettingValue {
    match text.trim() {
        "true" => SettingValue::Bool(true),
        "false" => SettingValue::Bool(false),
        _ => SettingValue::Str(text.to_string()),
    }
}

fn is_colour(text: &str) -> bool {
    text.len() == 7 && text.starts_with('#') && text[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Levenshtein distance, for "did you mean" hints.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(ca != *cb);
            current[j + 1] = substitute.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enhancements::catalogue::{ApplyAt, Toggle};
    use std::cell::Cell;
    use std::collections::HashSet;

    /// A stored settings file: keys in file order.
    #[derive(Default)]
    struct Stored {
        entries: Vec<(String, SettingValue)>,
        reads: Cell<usize>,
    }

    impl Stored {
        fn with(mut self, key: &str, value: SettingValue) -> Self {
            self.entries.push((key.to_string(), value));
            self
        }
    }

    impl SettingsSource for Stored {
        fn get(&self, key: &str) -> Option<SettingValue> {
            self.reads.set(self.reads.get() + 1);
            self.entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, value)| value.clone())
        }

        fn keys(&self) -> Vec<String> {
            self.reads.set(self.reads.get() + 1);
            self.entries.iter().map(|(key, _)| key.clone()).collect()
        }
    }

    fn text(value: &str) -> SettingValue {
        SettingValue::Str(value.to_string())
    }

    fn enable(key: &str) -> CliSetting {
        CliSetting::Enable(key.to_string())
    }

    fn disable(key: &str) -> CliSetting {
        CliSetting::Disable(key.to_string())
    }

    fn set(key: &str, value: &str) -> CliSetting {
        CliSetting::Set {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    fn input<'a>(
        stored: Option<&'a dyn SettingsSource>,
        cli: &'a [CliSetting],
    ) -> ResolveInput<'a> {
        ResolveInput {
            stored,
            cli,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        }
    }

    // --- A fixture catalogue with rows later packages add. -------------

    const SHIP: Phase = Phase::new(99, "fixture");
    const LATER: Phase = Phase::new(99, "later");
    const FIXTURE_SHIPPED: &[Phase] = &[SHIP];

    /// The resolver ignores `ToggleSpec::toggle` and uses the row's
    /// position; `Toggle` names built-in rows only.
    const fn toggle(
        key: &'static str,
        class: EffectClass,
        tier: Tier,
        ships: Phase,
        needs: &'static [Capability],
    ) -> ToggleSpec {
        ToggleSpec {
            toggle: Toggle::EnhancementsFreezeOverlays,
            key,
            label: "fixture",
            class,
            apply: ApplyAt::Frame,
            tier,
            ships,
            tags: &[],
            cli_aliases: &[],
            soft_deps: &[],
            needs,
            driven: false,
        }
    }

    /// Bit 0: the switch `audio.volume` turns on away from 100. An option
    /// can name only bit 0 in a fixture (see [`toggle`]).
    const VOLUME_SWITCH: usize = 0;
    const JOURNAL_PAGE: usize = 1;
    const DUNGEON: usize = 2;
    const GROW_LEFT: usize = 3;
    const FULLSCREEN_KEY: usize = 4;
    const CHEATS_ENABLED: usize = 5;
    const HEAL_ALL: usize = 6;
    const STATE_WATCH: usize = 7;
    const CLUES: usize = 8;

    const FIXTURE_TOGGLES: &[ToggleSpec] = &[
        toggle(
            "audio.volume_set",
            EffectClass::Presentation,
            Tier::Feature,
            SHIP,
            &[],
        ),
        toggle(
            "journal.page",
            EffectClass::Presentation,
            Tier::Feature,
            SHIP,
            &[],
        ),
        toggle(
            "automap.dungeon",
            EffectClass::Presentation,
            Tier::Feature,
            SHIP,
            &[],
        ),
        toggle(
            "display.grow_left",
            EffectClass::Presentation,
            Tier::Deviation,
            SHIP,
            &[],
        ),
        toggle(
            "display.fullscreen_key",
            EffectClass::Presentation,
            Tier::Feature,
            SHIP,
            &[Capability::Windowed],
        ),
        toggle(
            "cheats.enabled",
            EffectClass::StateAltering,
            Tier::Cheat,
            SHIP,
            &[],
        ),
        toggle(
            "cheats.heal_all",
            EffectClass::StateAltering,
            Tier::Cheat,
            SHIP,
            &[],
        ),
        toggle(
            "cheats.state_watch",
            EffectClass::Presentation,
            Tier::Cheat,
            SHIP,
            &[],
        ),
        toggle(
            "journal.clues",
            EffectClass::Presentation,
            Tier::Feature,
            LATER,
            &[],
        ),
    ];

    const fn option(
        key: &'static str,
        kind: OptionKind,
        default: OptionDefault,
        toggle: Option<Toggle>,
        needs: &'static [Capability],
    ) -> OptionSpec {
        OptionSpec {
            key,
            label: "fixture",
            kind,
            default,
            class: Some(EffectClass::Presentation),
            apply: ApplyAt::Frame,
            tier: Tier::Feature,
            deviation_values: &[],
            ships: SHIP,
            tags: &[],
            cli_aliases: &[],
            parent: match toggle {
                Some(_) => None,
                None => Some(Toggle::EnhancementsFreezeOverlays),
            },
            toggle,
            needs,
        }
    }

    const SCALE_WORDS: &[&str] = &["fit", "sharp", "stretch"];
    const PANEL_WORDS: &[&str] = &["journal", "conversation"];

    fn fixture_options() -> Vec<OptionSpec> {
        let preset = OPTIONS.iter().find(|spec| spec.key == PRESET_KEY).unwrap();
        let chord = |key, default| OptionSpec {
            class: None,
            parent: None,
            kind: OptionKind::Chord,
            needs: &[Capability::Keyboard],
            ..option(
                key,
                OptionKind::Chord,
                OptionDefault::Str(default),
                None,
                &[],
            )
        };
        vec![
            OptionSpec {
                ships: SHIP,
                ..*preset
            },
            option(
                "audio.volume",
                OptionKind::Int { min: 0, max: 100 },
                OptionDefault::Int(100),
                Some(Toggle::EnhancementsFreezeOverlays),
                &[],
            ),
            option(
                "journal.spoiler_tier",
                OptionKind::Int { min: 1, max: 4 },
                OptionDefault::Int(2),
                None,
                &[],
            ),
            option(
                "display.scale_mode",
                OptionKind::Enum(SCALE_WORDS),
                OptionDefault::Str("fit"),
                None,
                &[],
            ),
            option(
                "display.panel_priority",
                OptionKind::List(PANEL_WORDS),
                OptionDefault::List(PANEL_WORDS),
                None,
                &[],
            ),
            option(
                "display.pillar_color",
                OptionKind::Color,
                OptionDefault::Str("#000000"),
                None,
                &[],
            ),
            option(
                "input.mouse_speed",
                OptionKind::Int { min: 1, max: 10 },
                OptionDefault::Int(5),
                None,
                &[Capability::Pointer],
            ),
            chord("keys.menu", "F12"),
            chord("keys.journal", "Ctrl+J"),
        ]
    }

    const FIXTURE_RECOMMENDED: &[PresetEntry] = &[
        PresetEntry {
            key: "journal.page",
            value: PresetValue::On,
        },
        PresetEntry {
            key: "automap.dungeon",
            value: PresetValue::On,
        },
    ];

    const FIXTURE_EVERYTHING: &[PresetEntry] = &[
        PresetEntry {
            key: "journal.page",
            value: PresetValue::On,
        },
        PresetEntry {
            key: "automap.dungeon",
            value: PresetValue::On,
        },
        PresetEntry {
            key: "display.fullscreen_key",
            value: PresetValue::On,
        },
        PresetEntry {
            key: "journal.clues",
            value: PresetValue::On,
        },
        PresetEntry {
            key: "display.scale_mode",
            value: PresetValue::Choice("sharp"),
        },
    ];

    const FIXTURE_PRESETS: &[(Preset, &[PresetEntry])] = &[
        (Preset::Classic, &[]),
        (Preset::Recommended, FIXTURE_RECOMMENDED),
        (Preset::Everything, FIXTURE_EVERYTHING),
    ];

    /// Resolves against the fixture catalogue.
    fn fixture(input: ResolveInput<'_>) -> ResolvedEnhancements {
        let options = fixture_options();
        Catalogue {
            toggles: FIXTURE_TOGGLES,
            options: &options,
            presets: FIXTURE_PRESETS,
            shipped: FIXTURE_SHIPPED,
        }
        .resolve(input)
    }

    fn on(resolved: &ResolvedEnhancements) -> Vec<&'static str> {
        FIXTURE_TOGGLES
            .iter()
            .enumerate()
            .filter(|(index, _)| resolved.toggles.bit(*index))
            .map(|(_, spec)| spec.key)
            .collect()
    }

    fn note_lines(resolved: &ResolvedEnhancements) -> Vec<String> {
        resolved.notes.iter().map(ToString::to_string).collect()
    }

    // --- The built-in catalogue. ---------------------------------------

    #[test]
    fn nothing_stored_and_no_flags_is_classic() {
        let resolved = resolve(input(None, &[]));
        assert_eq!(resolved, ResolvedEnhancements::classic());
        assert!(resolved.is_classic());
        assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Classic));
        assert_eq!(resolved.preset.to_string(), "classic");
        assert!(resolved.notes.is_empty());
        assert_eq!(resolved.values.str(PRESET_KEY), Some("classic"));
        assert_eq!(resolved.values.int("enhancements.schema"), Some(1));
        assert_eq!(resolved.keys.chord("keys.menu"), Some("F12"));
        // Every valued setting is listed: chords as bindings, the rest as
        // values.
        assert_eq!(
            resolved.values.iter().count() + resolved.keys.iter().count(),
            OPTIONS.len()
        );

        let empty = Stored::default();
        assert_eq!(
            resolve(input(Some(&empty), &[])),
            ResolvedEnhancements::classic()
        );
    }

    #[test]
    fn locked_is_classic_with_the_lock() {
        let locked = ResolvedEnhancements::locked(LockReason::QaMode);
        assert!(locked.is_classic());
        assert_eq!(locked.lock, LockState::Locked(LockReason::QaMode));
        assert_eq!(
            ResolvedEnhancements {
                lock: LockState::Off,
                ..locked
            },
            ResolvedEnhancements::classic()
        );
    }

    #[test]
    fn a_stored_preset_with_nothing_shipped_is_still_that_preset() {
        // No preset row has shipped yet, so every preset is Classic's
        // toggles; the stored name is kept as the label.
        let stored = Stored::default().with(PRESET_KEY, text("recommended"));
        let resolved = resolve(input(Some(&stored), &[]));
        assert!(resolved.is_classic());
        assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Recommended));
        assert_eq!(resolved.values.str(PRESET_KEY), Some("recommended"));
        assert!(resolved.notes.is_empty());
    }

    #[test]
    fn built_in_unknown_and_hidden_keys_are_notes() {
        let stored = Stored::default()
            .with("enhancements.presett", text("everything"))
            .with("enhancements.freeze_overlays", SettingValue::Bool(true))
            .with("keys.menu", text("F1"))
            .with("display.alt_enter", SettingValue::Bool(true));
        let resolved = resolve(input(Some(&stored), &[]));
        assert!(resolved.is_classic());
        assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Classic));
        assert_eq!(resolved.keys.chord("keys.menu"), Some("F12"));
        assert_eq!(
            note_lines(&resolved),
            [
                "enhancements.presett: unknown key, ignored (did you mean enhancements.preset?)",
                "enhancements.freeze_overlays: not available in this build",
                "keys.menu: not available in this build",
                // Reserved names are not in the catalogue (U3).
                "display.alt_enter: unknown key, ignored",
            ]
        );
    }

    // --- Precedence. ---------------------------------------------------

    #[test]
    fn precedence_runs_defaults_file_preset_file_keys_cli_preset_cli_flags() {
        // Defaults.
        let resolved = fixture(input(None, &[]));
        assert!(resolved.is_classic());

        // The stored preset is the base.
        let stored = Stored::default().with(PRESET_KEY, text("recommended"));
        assert_eq!(
            on(&fixture(input(Some(&stored), &[]))),
            ["journal.page", "automap.dungeon"]
        );

        // Stored keys override it, whatever their order in the file.
        let stored = Stored::default()
            .with("automap.dungeon", SettingValue::Bool(false))
            .with("audio.volume", SettingValue::Int(50))
            .with(PRESET_KEY, text("recommended"));
        let resolved = fixture(input(Some(&stored), &[]));
        assert_eq!(on(&resolved), ["audio.volume_set", "journal.page"]);

        // `--preset` overrides the stored keys the presets name, but not
        // preferences.
        let cli = [CliSetting::Preset(Preset::Everything)];
        let resolved = fixture(input(Some(&stored), &cli));
        assert_eq!(
            on(&resolved),
            [
                "audio.volume_set",
                "journal.page",
                "automap.dungeon",
                "display.fullscreen_key"
            ]
        );
        assert_eq!(resolved.values.str("display.scale_mode"), Some("sharp"));
        assert_eq!(resolved.values.int("audio.volume"), Some(50));
        let cli = [CliSetting::Preset(Preset::Classic)];
        assert_eq!(
            on(&fixture(input(Some(&stored), &cli))),
            ["audio.volume_set"]
        );

        // The flags apply in command-line order, so the last word on a key
        // wins, and a later `--preset` resets the keys presets name.
        let cli = [
            enable("display.grow_left"),
            disable("journal.page"),
            enable("journal.page"),
            disable("journal.page"),
            set("audio.volume", "100"),
        ];
        let resolved = fixture(input(Some(&stored), &cli));
        assert_eq!(on(&resolved), ["display.grow_left"]);
        let cli = [enable("journal.page"), CliSetting::Preset(Preset::Classic)];
        assert_eq!(on(&fixture(input(None, &cli))), Vec::<&str>::new());
    }

    #[test]
    fn the_lock_wins_and_the_file_is_not_read() {
        let stored = Stored::default()
            .with(PRESET_KEY, text("everything"))
            .with("journal.page", SettingValue::Bool(true));
        let cli = [enable("display.grow_left"), set("audio.volume", "10")];
        let resolved = fixture(ResolveInput {
            lock: LockState::Locked(LockReason::EnvVar),
            ..input(Some(&stored), &cli)
        });
        assert_eq!(
            stored.reads.get(),
            0,
            "a locked launch never reads the file"
        );
        assert!(resolved.is_classic());
        assert_eq!(resolved.lock, LockState::Locked(LockReason::EnvVar));
        assert_eq!(
            note_lines(&resolved),
            ["parity lock: ignored --enable display.grow_left --set audio.volume=10"]
        );

        // The lock alone adds no note.
        let resolved = resolve(ResolveInput {
            lock: LockState::Locked(LockReason::CliFlag),
            ..input(None, &[])
        });
        assert_eq!(resolved, ResolvedEnhancements::locked(LockReason::CliFlag));
    }

    // --- Tolerance. ----------------------------------------------------

    #[test]
    fn a_bad_value_defaults_only_its_own_key() {
        let stored = Stored::default()
            .with(PRESET_KEY, text("recommended"))
            .with("journal.page", SettingValue::Int(1))
            .with("journal.spoiler_tier", SettingValue::Int(7))
            .with("display.scale_mode", text("shrp"))
            .with("display.pillar_color", text("red"))
            .with(
                "display.panel_priority",
                SettingValue::List(vec!["journal".into(), "history".into()]),
            )
            .with("audio.volume", text("loud"))
            .with("display.grow_left", SettingValue::Bool(true));
        let resolved = fixture(input(Some(&stored), &[]));
        // The good keys still apply: the preset's dungeon and grow-left.
        assert_eq!(on(&resolved), ["automap.dungeon", "display.grow_left"]);
        assert_eq!(resolved.values.int("journal.spoiler_tier"), Some(2));
        assert_eq!(resolved.values.str("display.scale_mode"), Some("fit"));
        assert_eq!(resolved.values.str("display.pillar_color"), Some("#000000"));
        assert_eq!(
            resolved.values.list("display.panel_priority"),
            Some(&["journal".to_string(), "conversation".to_string()][..])
        );
        assert_eq!(resolved.values.int("audio.volume"), Some(100));
        assert_eq!(
            note_lines(&resolved),
            [
                "journal.page = 1: not true or false, using false",
                "journal.spoiler_tier = 7: not in 1..4, using 2",
                "display.scale_mode = shrp: not one of fit, sharp, stretch, using fit",
                "display.pillar_color = red: not a colour like #rrggbb, using #000000",
                "display.panel_priority = [journal, history]: history is not a choice, \
                 using [journal, conversation]",
                "audio.volume = loud: not a whole number, using 100",
            ]
        );
    }

    #[test]
    fn a_bad_stored_preset_is_classic_with_a_note() {
        let stored = Stored::default()
            .with(PRESET_KEY, text("fancy"))
            .with("automap.dungeon", SettingValue::Bool(true));
        let resolved = fixture(input(Some(&stored), &[]));
        assert_eq!(on(&resolved), ["automap.dungeon"]);
        assert_eq!(resolved.preset, PresetLabel::Custom(Preset::Classic));
        assert_eq!(
            note_lines(&resolved),
            [
                "enhancements.preset = fancy: not one of classic, recommended, everything, using classic"
            ]
        );
    }

    #[test]
    fn command_line_values_are_typed_by_their_row() {
        let cli = [
            set("audio.volume", "40"),
            set("journal.spoiler_tier", "3"),
            set("display.panel_priority", "conversation, journal"),
            set("display.pillar_color", "#102030"),
            set("journal.page", "true"),
            set(PRESET_KEY, "recommended"),
            set("automap.dungeon", "false"),
            set("display.grow_left", "yes"),
            enable("display.scale_mode"),
        ];
        let resolved = fixture(input(None, &cli));
        assert_eq!(on(&resolved), ["audio.volume_set", "journal.page"]);
        assert_eq!(resolved.values.int("audio.volume"), Some(40));
        assert_eq!(resolved.values.int("journal.spoiler_tier"), Some(3));
        assert_eq!(
            resolved.values.list("display.panel_priority"),
            Some(&["conversation".to_string(), "journal".to_string()][..])
        );
        assert_eq!(resolved.values.str("display.pillar_color"), Some("#102030"));
        assert_eq!(
            note_lines(&resolved),
            [
                "display.grow_left = yes: not true or false, using false",
                "display.scale_mode: not an on/off setting, ignored (use --set)",
            ]
        );
    }

    #[test]
    fn unknown_keys_get_a_hint_when_one_is_close() {
        let stored = Stored::default()
            .with("display.growleft", SettingValue::Bool(true))
            .with("journal.pgae", SettingValue::Bool(true))
            .with("display.history_lines", SettingValue::Int(3))
            .with("zzz", SettingValue::Bool(true));
        let resolved = fixture(input(Some(&stored), &[]));
        assert!(resolved.is_classic());
        assert_eq!(
            note_lines(&resolved),
            [
                "display.growleft: unknown key, ignored (did you mean display.grow_left?)",
                "journal.pgae: unknown key, ignored (did you mean journal.page?)",
                "display.history_lines: unknown key, ignored",
                "zzz: unknown key, ignored",
            ]
        );
        // A hidden key is never offered as a hint.
        let stored = Stored::default().with("journal.cluess", SettingValue::Bool(true));
        let resolved = fixture(input(Some(&stored), &[]));
        assert_eq!(
            note_lines(&resolved),
            ["journal.cluess: unknown key, ignored"]
        );
    }

    #[test]
    fn hidden_rows_resolve_to_their_default() {
        let stored = Stored::default().with("journal.clues", SettingValue::Bool(true));
        let cli = [enable("journal.clues")];
        let resolved = fixture(input(Some(&stored), &cli));
        assert!(!resolved.toggles.bit(CLUES));
        assert!(resolved.is_classic());
        assert_eq!(
            note_lines(&resolved),
            [
                "journal.clues: not available in this build",
                "journal.clues: not available in this build",
            ]
        );
        // Everything names it, and skips it.
        let cli = [CliSetting::Preset(Preset::Everything)];
        assert!(!fixture(input(None, &cli)).toggles.bit(CLUES));
    }

    // --- Capabilities. -------------------------------------------------

    #[test]
    fn the_host_forces_off_what_it_cannot_do() {
        let phone = HostCapabilities {
            windowed: false,
            keyboard: true,
            function_keys: false,
            pointer: false,
            gamepad: true,
        };
        let cli = [
            enable("display.fullscreen_key"),
            enable("journal.page"),
            set("keys.journal", "F5"),
            set("input.mouse_speed", "8"),
        ];
        let resolved = fixture(ResolveInput {
            caps: phone,
            ..input(None, &cli)
        });
        assert_eq!(on(&resolved), ["journal.page"]);
        // F12 is the default, so it is unbound quietly; F5 was asked for.
        assert_eq!(resolved.keys.chord("keys.menu"), None);
        assert_eq!(resolved.keys.chord("keys.journal"), None);
        assert_eq!(resolved.values.int("input.mouse_speed"), Some(5));
        assert_eq!(
            note_lines(&resolved),
            [
                "display.fullscreen_key: needs a window, which this device does not have; \
                 turned off",
                "input.mouse_speed = 8: needs a mouse or pointer, which this device does not \
                 have; turned off",
                "keys.journal = F5: needs function keys, which this device does not have; \
                 turned off",
            ]
        );

        // Ctrl chords stay bound without function keys; nothing is bound
        // without a keyboard.
        let resolved = fixture(ResolveInput {
            caps: phone,
            ..input(None, &[])
        });
        assert_eq!(resolved.keys.chord("keys.journal"), Some("Ctrl+J"));
        let pad = HostCapabilities {
            keyboard: false,
            ..phone
        };
        let resolved = fixture(ResolveInput {
            caps: pad,
            ..input(None, &[])
        });
        assert!(resolved.keys.iter().all(|(_, chord)| chord.is_none()));
        assert!(resolved.notes.is_empty());
    }

    #[test]
    fn a_preset_trimmed_by_the_host_keeps_its_name() {
        let windowless = HostCapabilities {
            windowed: false,
            ..HostCapabilities::ALL
        };
        let cli = [CliSetting::Preset(Preset::Everything)];
        let resolved = fixture(ResolveInput {
            caps: windowless,
            ..input(None, &cli)
        });
        assert!(!resolved.toggles.bit(FULLSCREEN_KEY));
        assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Everything));
    }

    #[test]
    fn function_keys_are_recognised_in_chords() {
        for chord in ["F1", "F12", "Ctrl+F12", "Shift+f5", "F24"] {
            assert!(names_function_key(chord), "{chord}");
        }
        for chord in ["Tab", "Ctrl+F", "Ctrl+Shift+A", "F25", "F0", "Ctrl+M"] {
            assert!(!names_function_key(chord), "{chord}");
        }
    }

    // --- Cheats. -------------------------------------------------------

    #[test]
    fn cheat_keys_are_ignored_without_the_section_switch() {
        let stored = Stored::default()
            .with("cheats.heal_all", SettingValue::Bool(true))
            .with("cheats.state_watch", SettingValue::Bool(true));
        let resolved = fixture(input(Some(&stored), &[]));
        assert!(!resolved.toggles.bit(HEAL_ALL));
        // Watching state alters nothing, so it needs no switch (U7).
        assert!(resolved.toggles.bit(STATE_WATCH));
        assert_eq!(
            note_lines(&resolved),
            ["cheats.heal_all: ignored, cheats are not allowed (cheats.enabled is off)"]
        );

        let cli = [enable("cheats.enabled")];
        let resolved = fixture(input(Some(&stored), &cli));
        assert!(resolved.toggles.bit(CHEATS_ENABLED));
        assert!(resolved.toggles.bit(HEAL_ALL));
        assert!(resolved.notes.is_empty());

        // `--disable cheats.enabled` turns every gated cheat off for the
        // run.
        let stored = stored.with("cheats.enabled", SettingValue::Bool(true));
        let cli = [disable("cheats.enabled")];
        let resolved = fixture(input(Some(&stored), &cli));
        assert!(!resolved.toggles.bit(HEAL_ALL));
        assert!(!resolved.toggles.bit(CHEATS_ENABLED));
    }

    // --- Valued settings and Custom. -----------------------------------

    #[test]
    fn a_volume_of_fifty_alone_is_not_classic() {
        let stored = Stored::default().with("audio.volume", SettingValue::Int(50));
        let resolved = fixture(input(Some(&stored), &[]));
        assert!(!resolved.is_classic());
        assert!(resolved.toggles.bit(VOLUME_SWITCH));
        assert_eq!(resolved.preset, PresetLabel::Custom(Preset::Classic));

        // Back at its default it switches nothing.
        let stored = Stored::default().with("audio.volume", SettingValue::Int(100));
        assert!(fixture(input(Some(&stored), &[])).is_classic());

        // A sub-option is inert while its toggle is off, so alone it
        // leaves the game Classic.
        let stored = Stored::default().with("journal.spoiler_tier", SettingValue::Int(3));
        let resolved = fixture(input(Some(&stored), &[]));
        assert!(resolved.is_classic());
        assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Classic));
        assert_eq!(resolved.values.int("journal.spoiler_tier"), Some(3));
    }

    #[test]
    fn custom_is_detected_from_the_toggles() {
        // A preset changed by hand is Custom, based on that preset.
        let stored = Stored::default()
            .with(PRESET_KEY, text("recommended"))
            .with("journal.page", SettingValue::Bool(false));
        let resolved = fixture(input(Some(&stored), &[]));
        assert_eq!(resolved.preset, PresetLabel::Custom(Preset::Recommended));
        assert!(resolved.preset.is_custom());
        assert_eq!(resolved.preset.base(), Preset::Recommended);
        assert_eq!(resolved.preset.to_string(), "custom (based on recommended)");

        // Toggles that equal a preset exactly show that preset's name.
        let stored = Stored::default()
            .with("journal.page", SettingValue::Bool(true))
            .with("automap.dungeon", SettingValue::Bool(true));
        let resolved = fixture(input(Some(&stored), &[]));
        assert_eq!(resolved.preset, PresetLabel::Preset(Preset::Recommended));

        // A deviation-tier toggle makes any preset Custom.
        let cli = [
            CliSetting::Preset(Preset::Everything),
            enable("display.grow_left"),
        ];
        let resolved = fixture(input(None, &cli));
        assert!(resolved.toggles.bit(GROW_LEFT));
        assert!(resolved.toggles.bit(JOURNAL_PAGE) && resolved.toggles.bit(DUNGEON));
        assert_eq!(resolved.preset, PresetLabel::Custom(Preset::Everything));
    }

    #[test]
    fn the_fixture_matches_the_built_in_shape() {
        // The fixture follows the catalogue rules the built-in rows obey.
        let mut keys = HashSet::new();
        for spec in FIXTURE_TOGGLES {
            assert!(keys.insert(spec.key), "{}", spec.key);
        }
        for spec in fixture_options() {
            assert!(keys.insert(spec.key), "{}", spec.key);
            if spec.class.is_some() {
                assert!(
                    spec.toggle.is_some() != spec.parent.is_some(),
                    "{}",
                    spec.key
                );
            }
        }
        assert_eq!(Toggle::EnhancementsFreezeOverlays as usize, VOLUME_SWITCH);
    }

    #[test]
    fn hints_need_a_close_key() {
        assert_eq!(edit_distance("display.growleft", "display.grow_left"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(
            Catalogue::BUILT_IN.hint("enhancements.shema"),
            Some("enhancements.schema")
        );
        assert_eq!(Catalogue::BUILT_IN.hint("display.grow_left"), None);
    }
}
