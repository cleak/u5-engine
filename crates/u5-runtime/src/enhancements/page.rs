//! The settings pages' rows, shared by the setup wizard's Enhancements
//! page and the menu overlay's Settings tabs (Phase B design 1.2.2, 3.2.7;
//! build plan D-M4.5).
//!
//! Everything here is pure and reads only the catalogue and a
//! [`ResolvedEnhancements`]:
//!
//! - the page model: [`SettingsRows`], [`SettingRow`] and the working
//!   [`SettingsPage`] with its presets, focus, questions and
//!   [`SettingsChanges`] (moved from `u5-bevy-setup`, which re-exports
//!   them, so the wizard is unchanged);
//! - the row generator: [`settings_rows`] in [`SETTING_GROUPS`] order and
//!   [`preset_choice`] (moved from the desktop host's `setup.rs`). The
//!   wizard builds its rows from the file's resolution, the menu from the
//!   live one, so the two show the same rows for the same resolution;
//! - the row notes the menu shows (3.2.7, 3.3.6; D-M4.4): the effective
//!   apply point, [`effective_apply`], which reads "next launch" when the
//!   plugin a row needs was not installed at launch, and the "(set by
//!   command line)" mark from [`ResolvedEnhancements::cli_keys`];
//! - [`DORMANT_WITH_MENU`], the features whose plugin the menu overlay
//!   installs dormant at launch (the second arm of the shell's
//!   `feature_wanted`), so the menu can switch them on in play.

use super::catalogue::{
    ApplyAt, CatalogueRow, FeatureToggles, OptionDefault, OptionKind, PRESETS, Phase, Preset,
    SHIPPED_PHASES, Tier, Toggle, help_line, named_stops,
};
use super::display::RASTER_KEY;
use super::resolve::{CHEATS_SWITCH, ResolveInput, ResolvedEnhancements, checked_value, resolve};
use super::source::{CliSetting, HostCapabilities, LockState, SettingValue};

/// What the Enhancements page shows: the rows as the host describes them.
/// The page knows rows, not the catalogue: the host decides which settings
/// exist, their groups and order, and what each preset sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsRows {
    /// In display order. Rows of one group are adjacent; the page shows a
    /// group heading wherever the group changes. Cheat rows
    /// ([`RowRole::is_cheat_group`]) always form the last, collapsed group.
    pub rows: Vec<SettingRow>,
    /// The presets, in the order the page lists them.
    pub presets: Vec<PresetChoice>,
    /// The preset the stored settings start from (index into `presets`).
    pub preset: usize,
    /// The parity lock is on for this launch: the Cheats group is not
    /// shown at all.
    pub locked: bool,
    /// Lines the host wants shown above the rows (a damaged file, ...).
    pub notes: Vec<String>,
}

/// One setting on the Enhancements page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    /// The host's name for the setting, handed back in [`SettingsChanges`].
    pub key: String,
    /// Plain-language label.
    pub label: String,
    /// Group heading ("Display", "Maps", ...).
    pub group: String,
    /// Short tags shown after the label ("wider view").
    pub tags: Vec<String>,
    pub kind: RowKind,
    pub role: RowRole,
    /// The value now stored (or resolved) for this launch.
    pub value: RowValue,
    /// The value "Defaults" puts back.
    pub default: RowValue,
    /// A sub-option, drawn indented under the row before it (the grow-left
    /// rows under "Wider world view", design 2.4.1).
    pub indent: bool,
    /// The plain sentence the page shows while the row has the focus
    /// (design 10.2.1 for the gem map's rows); `None` when the label says
    /// enough.
    pub help: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// On or off ([`RowValue::Switch`]).
    Switch,
    /// One of these values, as (value, label) ([`RowValue::Choice`]).
    Choice(Vec<(String, String)>),
}

/// How a row behaves inside the Cheats group (design 1.2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowRole {
    /// An ordinary setting.
    Setting,
    /// "Allow cheats": first in the Cheats group; turning it on asks first.
    AllowCheats,
    /// A cheat: greyed until cheats are allowed.
    Cheat,
    /// "Watch game state": in the Cheats group, never greyed.
    WatchState,
}

impl RowRole {
    /// Whether the row belongs to the Cheats group.
    pub fn is_cheat_group(self) -> bool {
        self != RowRole::Setting
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowValue {
    Switch(bool),
    Choice(String),
}

/// One preset and the values it gives the rows it governs. Rows it does
/// not list (preferences, cheats) are left as they are when it is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetChoice {
    /// The host's name for it, handed back in [`SettingsChanges`].
    pub key: String,
    pub label: String,
    pub values: Vec<(String, RowValue)>,
}

/// What Save hands the host: the preset, if it changed, and every row
/// whose value must be stored for the page's settings to hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsChanges {
    pub preset: Option<String>,
    pub values: Vec<(String, RowValue)>,
}

impl SettingsChanges {
    pub fn is_empty(&self) -> bool {
        self.preset.is_none() && self.values.is_empty()
    }
}

/// A question the Enhancements page is waiting on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// Turning "Allow cheats" on (the row index).
    AllowCheats(usize),
    /// "Defaults": everything back to its default.
    Defaults,
    /// Choosing this preset over changes made on top of the current one:
    /// keep them, or take the preset as it is.
    KeepChanges(usize),
}

/// The preset the page's current values show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixLabel {
    Preset(usize),
    /// Matches no preset; started from this one.
    Custom(usize),
}

/// Something on the Enhancements page the keyboard focus can rest on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusItem {
    Preset(usize),
    Row(usize),
    CheatsGroup,
    Defaults,
    Cancel,
    Save,
}

/// The Enhancements page's working state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPage {
    pub rows: Vec<SettingRow>,
    pub presets: Vec<PresetChoice>,
    pub locked: bool,
    pub notes: Vec<String>,
    /// The preset chosen last; Custom is "based on" it.
    pub base: usize,
    /// Whether the Cheats group is open. It starts closed.
    pub cheats_open: bool,
    pub focus: usize,
    pub confirm: Option<Confirm>,
    /// A line under the preset row ("Settings were not saved: ...").
    pub notice: Option<String>,
    initial_values: Vec<RowValue>,
    initial_base: usize,
}

impl SettingsPage {
    pub fn new(settings: SettingsRows) -> Self {
        let base = settings
            .preset
            .min(settings.presets.len().saturating_sub(1));
        Self {
            initial_values: settings.rows.iter().map(|row| row.value.clone()).collect(),
            initial_base: base,
            rows: settings.rows,
            presets: settings.presets,
            locked: settings.locked,
            notes: settings.notes,
            base,
            cheats_open: false,
            focus: 0,
            confirm: None,
            notice: None,
        }
    }

    fn row_index(&self, key: &str) -> Option<usize> {
        self.rows.iter().position(|row| row.key == key)
    }

    /// Whether the Cheats group exists on this page: it has rows and the
    /// parity lock is off.
    pub fn shows_cheats(&self) -> bool {
        !self.locked && self.rows.iter().any(|row| row.role.is_cheat_group())
    }

    /// Whether the row is drawn: every ordinary row, and the Cheats group's
    /// rows while it is shown and open.
    pub fn row_visible(&self, index: usize) -> bool {
        let Some(row) = self.rows.get(index) else {
            return false;
        };
        !row.role.is_cheat_group() || (self.shows_cheats() && self.cheats_open)
    }

    /// Whether "Allow cheats" is on.
    pub fn cheats_allowed(&self) -> bool {
        self.rows
            .iter()
            .any(|row| row.role == RowRole::AllowCheats && row.value == RowValue::Switch(true))
    }

    /// Whether the row is greyed: a cheat while cheats are not allowed.
    /// "Watch game state" never is.
    pub fn row_greyed(&self, index: usize) -> bool {
        self.rows
            .get(index)
            .is_some_and(|row| row.role == RowRole::Cheat && !self.cheats_allowed())
    }

    /// The Cheats group's rows in display order: "Allow cheats" first,
    /// then the cheats, then "Watch game state".
    pub fn cheat_rows(&self) -> Vec<usize> {
        let mut rows: Vec<usize> = (0..self.rows.len())
            .filter(|index| self.rows[*index].role.is_cheat_group())
            .collect();
        rows.sort_by_key(|index| match self.rows[*index].role {
            RowRole::AllowCheats => 0,
            RowRole::Cheat | RowRole::Setting => 1,
            RowRole::WatchState => 2,
        });
        rows
    }

    /// Everything the focus visits, top to bottom.
    pub fn focus_items(&self) -> Vec<FocusItem> {
        let mut items: Vec<FocusItem> = (0..self.presets.len()).map(FocusItem::Preset).collect();
        items.extend(
            (0..self.rows.len())
                .filter(|index| !self.rows[*index].role.is_cheat_group())
                .map(FocusItem::Row),
        );
        if self.shows_cheats() {
            items.push(FocusItem::CheatsGroup);
            if self.cheats_open {
                items.extend(self.cheat_rows().into_iter().map(FocusItem::Row));
            }
        }
        items.extend([FocusItem::Defaults, FocusItem::Cancel, FocusItem::Save]);
        items
    }

    pub fn focused(&self) -> Option<FocusItem> {
        self.focus_items().get(self.focus).copied()
    }

    /// Puts the focus on `item`, if the page shows it.
    pub fn focus_on(&mut self, item: FocusItem) {
        if let Some(index) = self.focus_items().iter().position(|it| *it == item) {
            self.focus = index;
        }
    }

    /// The values a preset gives, as (row index, value), skipping keys the
    /// page has no row for.
    fn preset_values(&self, preset: usize) -> Vec<(usize, RowValue)> {
        self.presets
            .get(preset)
            .map(|choice| {
                choice
                    .values
                    .iter()
                    .filter_map(|(key, value)| {
                        self.row_index(key).map(|index| (index, value.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn matches_preset(&self, preset: usize) -> bool {
        self.preset_values(preset)
            .iter()
            .all(|(index, value)| self.rows[*index].value == *value)
    }

    /// The preset the current values show: the base preset first, then the
    /// others in order, or Custom based on the base.
    pub fn mix(&self) -> MixLabel {
        std::iter::once(self.base)
            .chain(0..self.presets.len())
            .find(|preset| *preset < self.presets.len() && self.matches_preset(*preset))
            .map_or(MixLabel::Custom(self.base), MixLabel::Preset)
    }

    /// "Classic", or "Custom (based on Recommended)".
    pub fn mix_text(&self) -> String {
        let label = |preset: usize| {
            self.presets
                .get(preset)
                .map_or("", |choice| choice.label.as_str())
        };
        match self.mix() {
            MixLabel::Preset(preset) => label(preset).to_string(),
            MixLabel::Custom(base) => format!("Custom (based on {})", label(base)),
        }
    }

    /// Rows the base preset governs whose value the player changed on top
    /// of it.
    fn changes_on_base(&self) -> Vec<(usize, RowValue)> {
        self.preset_values(self.base)
            .into_iter()
            .filter(|(index, value)| self.rows[*index].value != *value)
            .map(|(index, _)| (index, self.rows[index].value.clone()))
            .collect()
    }

    fn apply_preset(&mut self, preset: usize, keep_changes: bool) {
        let kept = if keep_changes {
            self.changes_on_base()
        } else {
            Vec::new()
        };
        for (index, value) in self.preset_values(preset) {
            self.rows[index].value = value;
        }
        for (index, value) in kept {
            self.rows[index].value = value;
        }
        self.base = preset;
    }

    /// Chooses the preset at `preset`, asking first when changes on top
    /// of the base preset would be lost. False for no such preset.
    pub fn select_preset(&mut self, preset: usize) -> bool {
        if preset >= self.presets.len() {
            return false;
        }
        self.focus_on(FocusItem::Preset(preset));
        if self.changes_on_base().is_empty() {
            self.apply_preset(preset, false);
        } else {
            self.confirm = Some(Confirm::KeepChanges(preset));
        }
        true
    }

    /// Switches the row at `index`, or steps a choice row to its next
    /// value. False for a hidden or greyed row.
    pub fn toggle_row(&mut self, index: usize) -> bool {
        if !self.row_visible(index) || self.row_greyed(index) {
            return false;
        }
        self.focus_on(FocusItem::Row(index));
        let row = &self.rows[index];
        let next = match (&row.kind, &row.value) {
            (RowKind::Switch, RowValue::Switch(on)) => RowValue::Switch(!on),
            (RowKind::Choice(choices), value) => {
                if choices.is_empty() {
                    return false;
                }
                let at = choices
                    .iter()
                    .position(|(choice, _)| RowValue::Choice(choice.clone()) == *value);
                let next = at.map_or(0, |at| (at + 1) % choices.len());
                RowValue::Choice(choices[next].0.clone())
            }
            // A switch row holding a choice value is a host error; turn
            // it into a plain switch.
            (RowKind::Switch, RowValue::Choice(_)) => RowValue::Switch(true),
        };
        if row.role == RowRole::AllowCheats && next == RowValue::Switch(true) {
            self.confirm = Some(Confirm::AllowCheats(index));
            return true;
        }
        self.rows[index].value = next;
        true
    }

    fn defaults(&mut self) {
        for row in &mut self.rows {
            row.value = row.default.clone();
        }
        // The preset whose values are the defaults: Classic.
        if let Some(preset) = (0..self.presets.len()).find(|preset| self.matches_preset(*preset)) {
            self.base = preset;
        }
    }

    /// Answers the open question (`yes` is its first answer). False when
    /// none is open.
    pub fn answer(&mut self, yes: bool) -> bool {
        let Some(confirm) = self.confirm.take() else {
            return false;
        };
        match confirm {
            Confirm::AllowCheats(index) if yes => self.rows[index].value = RowValue::Switch(true),
            Confirm::Defaults if yes => self.defaults(),
            Confirm::KeepChanges(preset) => self.apply_preset(preset, yes),
            Confirm::AllowCheats(_) | Confirm::Defaults => {}
        }
        true
    }

    /// Moves the focus to the next (`forward`) or previous item, wrapping.
    pub fn move_focus(&mut self, forward: bool) -> bool {
        let count = self.focus_items().len();
        if count == 0 {
            return false;
        }
        self.focus = if forward {
            (self.focus + 1) % count
        } else {
            (self.focus + count - 1) % count
        };
        true
    }

    /// What Save stores: the preset if it changed, every row whose value
    /// changed since the page opened, and, after a preset change, every
    /// row the new preset governs that the player set apart from it (so
    /// a key stored before cannot override the new preset).
    pub fn changes(&self) -> SettingsChanges {
        let preset_changed = self.base != self.initial_base;
        let governed: Vec<(usize, RowValue)> = if preset_changed {
            self.preset_values(self.base)
        } else {
            Vec::new()
        };
        let values = self
            .rows
            .iter()
            .enumerate()
            .filter(|(index, row)| {
                row.value != self.initial_values[*index]
                    || governed
                        .iter()
                        .any(|(governed, value)| governed == index && row.value != *value)
            })
            .map(|(_, row)| (row.key.clone(), row.value.clone()))
            .collect();
        SettingsChanges {
            preset: preset_changed.then(|| self.presets[self.base].key.clone()),
            values,
        }
    }
}

/// The Enhancements page's groups, in order (design 1.2.2), by the section
/// of a setting's key. A section not listed goes in "Other".
pub const SETTING_GROUPS: &[(&str, &[&str])] = &[
    ("Display", &["display"]),
    ("Maps", &["automap", "gem_map"]),
    ("Journal", &["journal"]),
    ("Menu and saves", &["menu", "input", "enhancements"]),
    ("Audio", &["audio"]),
    ("Combat", &["combat"]),
    ("Other", &[]),
    ("Cheats", &["cheats"]),
];

/// The index in [`SETTING_GROUPS`] of the group `key` belongs to.
pub fn setting_group(key: &str) -> usize {
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

/// Small number ranges become a choice; larger ones offer their named
/// stops, or stay in the file.
const MAX_NUMBER_CHOICES: i64 = 10;

/// The rows for one resolution, grouped in [`SETTING_GROUPS`] order and in
/// catalogue order within a group, each sub-option just under its parent.
///
/// Rows are the catalogue's shipped rows that a page can edit: every
/// toggle, and every on/off, word-choice or small-number setting. The
/// foundation's own keys (the file format, the preset, which is the
/// page's preset row, and the `[keys]` chords) and free-form values
/// (colours, paths, lists) are left to `config.toml`.
pub fn settings_rows(resolved: &ResolvedEnhancements) -> Vec<SettingRow> {
    settings_rows_in(resolved, SHIPPED_PHASES)
}

/// [`settings_rows`] as a build that has shipped the phases `shipped`
/// would list them: the rows of those phases only. Every page uses
/// [`SHIPPED_PHASES`]; the menu's tests use this to see the rows of a
/// phase that ships later in the milestone (build plan M4-07).
pub fn settings_rows_in(resolved: &ResolvedEnhancements, shipped: &[Phase]) -> Vec<SettingRow> {
    // (group, parent row key for a sub-option, row)
    let mut rows: Vec<(usize, Option<&'static str>, SettingRow)> = Vec::new();
    for row in CatalogueRow::all().filter(|row| shipped.contains(&row.ships())) {
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
                // `display.raster` has no row while both of its values draw
                // the same tiles (Phase B design 2.4.1); it stays in the file
                // and `--raster-depth`.
                if spec.key == RASTER_KEY {
                    continue;
                }
                let kind = match spec.kind {
                    OptionKind::Bool => RowKind::Switch,
                    OptionKind::Enum(words) => RowKind::Choice(
                        words
                            .iter()
                            .filter(|word| spec.value_is_shipped(word))
                            .map(|word| (word.to_string(), word.to_string()))
                            .collect(),
                    ),
                    OptionKind::Int { min, max, step }
                        if (max - min) / step < MAX_NUMBER_CHOICES =>
                    {
                        RowKind::Choice(
                            (min..=max)
                                .step_by(step as usize)
                                .map(|number| (number.to_string(), number.to_string()))
                                .collect(),
                        )
                    }
                    // A longer range offers its named stops (build plan
                    // D-M2.4), and the stored value when it is not one.
                    // The stops are percents (design 9.2.1).
                    OptionKind::Int { .. } => {
                        let Some(stops) = named_stops(spec.key) else {
                            continue;
                        };
                        let mut numbers = stops.to_vec();
                        if let Some(SettingValue::Int(stored)) = resolved.values.get(spec.key)
                            && !numbers.contains(stored)
                        {
                            numbers.push(*stored);
                        }
                        RowKind::Choice(
                            numbers
                                .iter()
                                .map(|number| (number.to_string(), format!("{number}%")))
                                .collect(),
                        )
                    }
                    _ => continue,
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
                    .map_or_else(|| default.clone(), RowValue::of);
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
        // A sub-option whose parent has a row of its own (the grow-left
        // rows under `display.grow_left`; `display.brightness_scope` under
        // the valued `display.brightness`, whose driven switch shares its
        // key) sits indented under it. One whose parent has no row stays
        // where it is, unindented (below).
        let parent = match row {
            CatalogueRow::Option(spec) => spec
                .parent
                .map(|toggle| toggle.spec())
                .filter(|parent| shipped.contains(&parent.ships))
                .map(|parent| parent.key),
            CatalogueRow::Toggle(_) => None,
        };
        rows.push((
            group,
            parent,
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
                indent: parent.is_some(),
                help: help_line(key).map(str::to_string),
            },
        ));
    }
    rows.sort_by_key(|(group, _, _)| *group);
    // A parent without a row of its own leaves its sub-options at the top
    // level.
    let keys: Vec<String> = rows.iter().map(|(_, _, row)| row.key.clone()).collect();
    for (_, parent, row) in rows.iter_mut() {
        if parent.is_some_and(|parent| !keys.iter().any(|key| key == parent)) {
            *parent = None;
            row.indent = false;
        }
    }
    // Each sub-option moves to just under its parent, in catalogue order.
    let (subs, tops): (Vec<_>, Vec<_>) = rows.into_iter().partition(|(_, p, _)| p.is_some());
    let mut ordered = Vec::with_capacity(tops.len() + subs.len());
    for (_, _, row) in tops {
        let key = row.key.clone();
        ordered.push(row);
        ordered.extend(
            subs.iter()
                .filter(|(_, parent, _)| *parent == Some(key.as_str()))
                .map(|(_, _, sub)| sub.clone()),
        );
    }
    ordered
}

impl RowValue {
    /// A resolved value as a row shows it: a flag is a switch, anything
    /// else a choice in its text form.
    pub fn of(value: &SettingValue) -> Self {
        match value {
            SettingValue::Bool(flag) => RowValue::Switch(*flag),
            SettingValue::Int(number) => RowValue::Choice(number.to_string()),
            other => RowValue::Choice(other.to_string()),
        }
    }

    /// The value to store for the row `key`: a switch as a flag, and a
    /// choice as a number for a number setting, as text otherwise.
    pub fn to_setting(&self, key: &str) -> SettingValue {
        match self {
            RowValue::Switch(flag) => SettingValue::Bool(*flag),
            RowValue::Choice(text) => match CatalogueRow::find(key) {
                Some(CatalogueRow::Option(spec)) if matches!(spec.kind, OptionKind::Int { .. }) => {
                    text.parse()
                        .map(SettingValue::Int)
                        .unwrap_or_else(|_| SettingValue::Str(text.clone()))
                }
                _ => SettingValue::Str(text.clone()),
            },
        }
    }
}

/// A preset as a page sees it: the value it gives every page row any
/// preset names. Rows no preset names (preferences, cheats) are not
/// listed, so choosing a preset leaves them alone (design 1.4.2).
pub fn preset_choice(preset: Preset, rows: &[SettingRow]) -> PresetChoice {
    let governed: Vec<&str> = PRESETS
        .iter()
        .flat_map(|preset| preset.shipped_entries().map(|(entry, _)| entry.key))
        .collect();
    let applied = resolve(ResolveInput {
        stored: None,
        cli: &[CliSetting::Preset(preset)],
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    let values = settings_rows(&applied)
        .into_iter()
        .filter(|row| governed.contains(&row.key.as_str()))
        .filter(|row| rows.iter().any(|page_row| page_row.key == row.key))
        .map(|row| (row.key, row.value))
        .collect();
    PresetChoice {
        key: preset.key().to_string(),
        label: preset.label().to_string(),
        values,
    }
}

/// The rows, the presets and the starting preset for one resolution, with
/// the host's `notes` above them. `locked` hides the Cheats group.
pub fn settings_page_rows(
    resolved: &ResolvedEnhancements,
    locked: bool,
    notes: Vec<String>,
) -> SettingsRows {
    settings_page_rows_in(resolved, locked, notes, SHIPPED_PHASES)
}

/// [`settings_page_rows`] for a build that has shipped the phases
/// `shipped` ([`settings_rows_in`]).
pub fn settings_page_rows_in(
    resolved: &ResolvedEnhancements,
    locked: bool,
    notes: Vec<String>,
    shipped: &[Phase],
) -> SettingsRows {
    let rows = settings_rows_in(resolved, shipped);
    let presets = PRESETS
        .iter()
        .map(|preset| preset_choice(*preset, &rows))
        .collect();
    let preset = PRESETS
        .iter()
        .position(|preset| *preset == resolved.preset.base())
        .unwrap_or(0);
    SettingsRows {
        rows,
        presets,
        preset,
        locked,
        notes,
    }
}

/// The features whose plugin the menu overlay installs at launch even
/// while they are off, because the dormant install is proven inert (build
/// plan D-M4.4): the side fill, brightness, sprite compositing and the
/// gem map. The menu can switch them on in play. Every other feature that
/// was off at launch applies at the next launch.
pub const DORMANT_WITH_MENU: &[Toggle] = &[
    Toggle::DisplayPillarFill,
    Toggle::DisplayBrightness,
    Toggle::DisplaySpriteComposite,
    Toggle::GemMapEnhanced,
];

/// Whether `t`'s plugin is installed dormant at launch: the menu overlay
/// is on and `t` is in [`DORMANT_WITH_MENU`].
pub fn dormant_install(launch: FeatureToggles, t: Toggle) -> bool {
    launch.on(Toggle::MenuOverlay) && DORMANT_WITH_MENU.contains(&t)
}

/// Keys whose features read them only when they start, so a change in play
/// waits for the next launch even with the feature on: the side-space
/// arbiter's settings, the maps' brightness and the gem map's options
/// (`EnhLaunch`).
pub const READ_AT_LAUNCH: &[&str] = &[
    "display.panel_priority",
    "display.panel_guard",
    "display.map_brightness",
];

/// The feature a key belongs to: a toggle's own, a valued setting's
/// switch, or a sub-option's parent. `None` for the foundation's keys.
pub fn key_feature(key: &str) -> Option<Toggle> {
    match CatalogueRow::find(key)? {
        CatalogueRow::Toggle(spec) => Some(spec.toggle),
        CatalogueRow::Option(spec) => spec.toggle.or(spec.parent),
    }
}

/// The catalogue's apply point for `key` (3.4.1); a key the catalogue does
/// not have applies at the next launch.
pub fn catalogue_apply(key: &str) -> ApplyAt {
    match CatalogueRow::find(key) {
        Some(CatalogueRow::Toggle(spec)) => spec.apply,
        Some(CatalogueRow::Option(spec)) => spec.apply,
        None => ApplyAt::NextLaunch,
    }
}

/// Whether the plugin `feature` needs was installed at a launch with the
/// toggles `launch` (D-M4.4):
///
/// - the scale policy and pixel shape: a launch away from the faithful
///   fit (either of them, or the grow-left canvas);
/// - the window keys (fullscreen and vsync): a launch with one of them on,
///   or with the menu overlay, which installs the window wish (M4-08,
///   3.3.8); the F11 key only with itself on;
/// - the output gain (volume and the player's mute): `VolumePlugin`'s
///   condition (3.4.2), the overlay or either of them, or focus mute,
///   which brings it;
/// - focus mute: `FocusMutePlugin`'s condition, itself or the overlay;
/// - the grow-left layout: only with itself on;
/// - every other feature: on at launch, or installed dormant
///   ([`dormant_install`]).
pub fn installed_at_launch(feature: Toggle, launch: FeatureToggles) -> bool {
    use Toggle::*;
    let any = |set: &[Toggle]| set.iter().any(|t| launch.on(*t));
    match feature {
        DisplayScaleMode | DisplayPixelAspect => {
            any(&[DisplayScaleMode, DisplayPixelAspect, DisplayGrowLeft])
        }
        DisplayFullscreen | DisplayVsync => any(&[
            DisplayFullscreen,
            DisplayFullscreenKey,
            DisplayVsync,
            MenuOverlay,
        ]),
        AudioVolume | AudioMuted => {
            any(&[MenuOverlay, AudioVolume, AudioMuted, AudioMuteOnFocusLoss])
        }
        AudioMuteOnFocusLoss => any(&[MenuOverlay, AudioMuteOnFocusLoss]),
        _ => launch.on(feature) || dormant_install(launch, feature),
    }
}

/// When a change to `key` made in play takes effect in a run launched
/// with the toggles `launch` (3.2.7, D-M4.4): the catalogue's point, or
/// [`ApplyAt::NextLaunch`] when the feature's plugin was not installed at
/// launch or reads the key only when it starts ([`READ_AT_LAUNCH`], the
/// gem map's options).
pub fn effective_apply(key: &str, launch: FeatureToggles) -> ApplyAt {
    let apply = catalogue_apply(key);
    // The gem map reads its options from the launch config.
    let gem_option = key.starts_with("gem_map.")
        && matches!(CatalogueRow::find(key), Some(CatalogueRow::Option(_)));
    if apply == ApplyAt::NextLaunch || READ_AT_LAUNCH.contains(&key) || gem_option {
        return ApplyAt::NextLaunch;
    }
    match key_feature(key) {
        Some(feature) if !installed_at_launch(feature, launch) => ApplyAt::NextLaunch,
        _ => apply,
    }
}

/// Puts a valued change made in play on `live` (D-M4.4): the checked
/// value, and for a valued toggle (such as `display.brightness`) its
/// switch, on while the value is away from its default. Returns the value
/// put there, or `None` when it was refused (no such shipped key, or a bad
/// value) and nothing changed.
pub fn apply_value(
    live: &mut ResolvedEnhancements,
    key: &str,
    value: SettingValue,
) -> Option<SettingValue> {
    let value = checked_value(key, value)?;
    let Some(CatalogueRow::Option(spec)) = CatalogueRow::find(key) else {
        return None;
    };
    if !live.values.set(key, value.clone()) {
        return None;
    }
    if let Some(toggle) = spec.toggle {
        live.toggles.set(toggle, value != spec.default_value());
    }
    Some(value)
}

/// What a settings row says beside its value (3.2.7, 3.3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowNote {
    /// When a change applies in this run: "next prompt" or "next launch"
    /// unless it is the next frame.
    pub apply: ApplyAt,
    /// "(set by command line)".
    pub by_cli: bool,
}

/// The note of each of `rows`, in order, for a run launched with the
/// toggles `launch` whose resolution is `resolved`.
pub fn row_notes(
    rows: &[SettingRow],
    resolved: &ResolvedEnhancements,
    launch: FeatureToggles,
) -> Vec<RowNote> {
    rows.iter()
        .map(|row| RowNote {
            apply: effective_apply(&row.key, launch),
            by_cli: resolved.set_by_cli(&row.key),
        })
        .collect()
}

#[cfg(test)]
mod tests;
