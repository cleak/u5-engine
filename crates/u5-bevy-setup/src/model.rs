//! The wizard's state machine, independent of Bevy so it can be tested
//! directly. The Bevy plugin only renders [`WizardModel::screen`] and feeds
//! button presses back in as [`Action`]s.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, TryRecvError};

use u5_gamedata::{Discovered, ValidationReport};

/// What the wizard needs from its host platform.
///
/// The desktop frontend implements this over `u5-host-desktop`; another
/// port supplies its own (a document picker, a sandbox data folder).
pub trait WizardHost: Send + Sync + 'static {
    /// Search the usual places again.
    fn discover(&self) -> Vec<Discovered>;
    /// Validate a folder the player is considering (may search below it).
    fn validate(&self, path: &Path) -> ValidationReport;
    /// Install the folder's data, remember the choice, and describe it.
    fn commit(&self, path: &Path) -> Result<CommitSummary, String>;
    /// Open the platform's folder picker. The answer arrives on the
    /// channel (`None` = cancelled). `None` here means no picker exists.
    fn start_folder_pick(&self) -> Option<Receiver<Option<PathBuf>>>;
    /// "How to get the files" text.
    fn help_text(&self) -> String;
    /// The Enhancements page's rows, presets and notes, read fresh each
    /// time the page opens. `None` (the default) means the host has no
    /// enhancement settings, and the wizard offers no such page.
    fn settings_rows(&self) -> Option<SettingsRows> {
        None
    }
    /// Store what the player changed on the Enhancements page. Called with
    /// changed keys only, and never with an empty change.
    fn save_settings(&self, _changes: &SettingsChanges) -> Result<(), String> {
        Err("this system cannot store settings".to_string())
    }
}

/// A finished setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSummary {
    /// The folder the engine should run from.
    pub data_dir: PathBuf,
    /// A short description for the final screen.
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WizardOutcome {
    /// Set up; start the game.
    Completed(CommitSummary),
    /// Set up and saved, but the player quit instead of starting the game.
    SavedOnly(CommitSummary),
    /// The settings-only wizard ([`WizardModel::settings_only`]) left its
    /// Enhancements page with Save (`saved`) or Cancel; start the game.
    SettingsClosed { saved: bool },
    /// Nothing was set up.
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    /// The detected folders and the ways to choose another.
    Candidates,
    /// Waiting for the native folder picker.
    Picking,
    /// A folder's validation result.
    Review {
        path: PathBuf,
        report: Box<ValidationReport>,
        /// Set when installing it failed.
        error: Option<String>,
    },
    Help,
    Done(CommitSummary),
    /// The Enhancements page ([`WizardModel::settings`] holds its state).
    Enhancements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    SelectCandidate(usize),
    ChooseFolder,
    Rescan,
    ShowHelp,
    Back,
    UseFolder,
    Start,
    Quit,
    /// Open the Enhancements page (from the welcome screen).
    OpenSettings,
    /// Enhancements page: choose the preset at this index.
    SelectPreset(usize),
    /// Enhancements page: switch the row at this index, or step a choice
    /// row to its next value.
    ToggleSetting(usize),
    /// Enhancements page: open or close the Cheats group.
    ToggleCheatsGroup,
    /// Enhancements page: back to every default (asks first).
    SettingsDefaults,
    /// Enhancements page: store the changes.
    SaveSettings,
    /// Enhancements page: move the keyboard focus.
    FocusNext,
    FocusPrev,
    /// Enhancements page: what Space does to the focused item.
    ActivateFocus,
    /// A confirmation's first answer ("Allow cheats", "Keep my changes").
    ConfirmYes,
    /// A confirmation's second answer ("Cancel", "Use the preset as it is").
    ConfirmNo,
}

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

    fn focus_on(&mut self, item: FocusItem) {
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

    fn select_preset(&mut self, preset: usize) -> bool {
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

    fn toggle_row(&mut self, index: usize) -> bool {
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

    fn answer(&mut self, yes: bool) -> bool {
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

    fn move_focus(&mut self, forward: bool) -> bool {
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

pub struct WizardModel {
    pub screen: Screen,
    pub candidates: Vec<Discovered>,
    /// Why the wizard is running (shown on the welcome screen).
    pub reason: String,
    /// A transient line under the candidate list ("No folder chosen.").
    pub notice: Option<String>,
    pub outcome: Option<WizardOutcome>,
    /// The Enhancements page's state while it is open.
    pub settings: Option<SettingsPage>,
    /// Whether the welcome screen offers the Enhancements page (the host
    /// has settings; see [`WizardModel::offer_settings`]).
    pub offers_settings: bool,
    /// The wizard is only the Enhancements page (`u5-engine --settings`):
    /// leaving the page finishes it.
    settings_only: bool,
    help_return: Option<Box<Screen>>,
    // Behind a mutex only so the model is `Sync` (a Bevy resource).
    pending_pick: Option<Mutex<Receiver<Option<PathBuf>>>>,
}

impl WizardModel {
    pub fn new(reason: impl Into<String>, candidates: Vec<Discovered>) -> Self {
        Self {
            screen: Screen::Welcome,
            candidates,
            reason: reason.into(),
            notice: None,
            outcome: None,
            settings: None,
            offers_settings: false,
            settings_only: false,
            help_return: None,
            pending_pick: None,
        }
    }

    /// Offer the Enhancements page on the welcome screen when the host has
    /// settings.
    pub fn offer_settings(mut self, host: &dyn WizardHost) -> Self {
        self.offers_settings = host.settings_rows().is_some();
        self
    }

    /// A wizard that is only the Enhancements page (`u5-engine --settings`).
    /// Save or Cancel finishes it with [`WizardOutcome::SettingsClosed`]. A
    /// host without settings finishes it at once.
    pub fn settings_only(host: &dyn WizardHost) -> Self {
        let mut model = Self::new(String::new(), Vec::new());
        model.settings_only = true;
        match host.settings_rows() {
            Some(rows) => {
                model.offers_settings = true;
                model.settings = Some(SettingsPage::new(rows));
                model.screen = Screen::Enhancements;
            }
            None => model.outcome = Some(WizardOutcome::SettingsClosed { saved: false }),
        }
        model
    }

    /// Leave the Enhancements page: finish a settings-only wizard, or go
    /// back to the welcome screen.
    fn close_settings(&mut self, saved: bool) {
        self.settings = None;
        if self.settings_only {
            self.outcome = Some(WizardOutcome::SettingsClosed { saved });
        } else {
            self.screen = Screen::Welcome;
            if saved {
                self.notice = Some("Settings saved.".to_string());
            }
        }
    }

    /// Apply an action on the Enhancements page.
    fn apply_settings(&mut self, action: Action, host: &dyn WizardHost) -> bool {
        let Some(page) = self.settings.as_mut() else {
            return false;
        };
        if page.confirm.is_some() {
            // A question is open: only its answers (and Back, which
            // cancels it) do anything.
            return match action {
                Action::ConfirmYes => page.answer(true),
                Action::ConfirmNo => page.answer(false),
                Action::Back => page.confirm.take().is_some(),
                _ => false,
            };
        }
        let action = match (action, page.focused()) {
            (Action::ActivateFocus, Some(item)) => match item {
                FocusItem::Preset(preset) => Action::SelectPreset(preset),
                FocusItem::Row(index) => Action::ToggleSetting(index),
                FocusItem::CheatsGroup => Action::ToggleCheatsGroup,
                FocusItem::Defaults => Action::SettingsDefaults,
                FocusItem::Cancel => Action::Back,
                FocusItem::Save => Action::SaveSettings,
            },
            (action, _) => action,
        };
        match action {
            Action::SelectPreset(preset) => page.select_preset(preset),
            Action::ToggleSetting(index) => page.toggle_row(index),
            Action::ToggleCheatsGroup => {
                if !page.shows_cheats() {
                    return false;
                }
                page.cheats_open = !page.cheats_open;
                page.focus_on(FocusItem::CheatsGroup);
                true
            }
            Action::SettingsDefaults => {
                page.confirm = Some(Confirm::Defaults);
                true
            }
            Action::FocusNext => page.move_focus(true),
            Action::FocusPrev => page.move_focus(false),
            Action::Back => {
                self.close_settings(false);
                true
            }
            Action::SaveSettings => {
                let changes = page.changes();
                if changes.is_empty() {
                    self.close_settings(false);
                    return true;
                }
                match host.save_settings(&changes) {
                    Ok(()) => self.close_settings(true),
                    Err(error) => {
                        page.notice = Some(format!("Settings were not saved: {error}"));
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.outcome.is_some()
    }

    /// Apply a player action. Returns whether anything changed.
    pub fn apply(&mut self, action: Action, host: &dyn WizardHost) -> bool {
        if self.is_finished() {
            return false;
        }
        let screen = self.screen.clone();
        match (screen, action) {
            (Screen::Done(summary), Action::Quit) => {
                self.outcome = Some(WizardOutcome::SavedOnly(summary));
            }
            (Screen::Enhancements, Action::Quit) => {
                self.settings = None;
                self.outcome = Some(WizardOutcome::Cancelled);
            }
            (Screen::Enhancements, action) => return self.apply_settings(action, host),
            (_, Action::Quit) => {
                self.outcome = Some(WizardOutcome::Cancelled);
            }
            (Screen::Help, Action::Back) => {
                self.screen = self
                    .help_return
                    .take()
                    .map(|screen| *screen)
                    .unwrap_or(Screen::Candidates);
            }
            (current, Action::ShowHelp) if current != Screen::Help => {
                self.help_return = Some(Box::new(current));
                self.screen = Screen::Help;
            }
            (Screen::Welcome, Action::Continue) => {
                self.screen = Screen::Candidates;
            }
            (Screen::Welcome, Action::OpenSettings) => match host.settings_rows() {
                Some(rows) => {
                    self.notice = None;
                    self.settings = Some(SettingsPage::new(rows));
                    self.screen = Screen::Enhancements;
                }
                None => return false,
            },
            (Screen::Candidates, Action::SelectCandidate(index)) => {
                let Some(found) = self.candidates.get(index) else {
                    return false;
                };
                // Re-validate: the folder may have changed since discovery.
                let report = host.validate(&found.data_root);
                self.screen = Screen::Review {
                    path: found.data_root.clone(),
                    report: Box::new(report),
                    error: None,
                };
            }
            (Screen::Candidates | Screen::Review { .. }, Action::ChooseFolder) => {
                match host.start_folder_pick() {
                    Some(receiver) => {
                        self.pending_pick = Some(Mutex::new(receiver));
                        self.notice = None;
                        self.screen = Screen::Picking;
                        // A picker that answered synchronously (macOS).
                        self.poll(host);
                    }
                    None => {
                        self.notice = Some(
                            "This system has no folder picker; start the program with \
                             --setup <folder> instead."
                                .to_string(),
                        );
                        self.screen = Screen::Candidates;
                    }
                }
            }
            (Screen::Candidates, Action::Rescan) => {
                self.candidates = host.discover();
                self.notice = Some(if self.candidates.is_empty() {
                    "Still nothing found.".to_string()
                } else {
                    format!("Found {} folder(s).", self.candidates.len())
                });
            }
            (Screen::Candidates, Action::Back) => {
                self.screen = Screen::Welcome;
            }
            (Screen::Review { .. }, Action::Back) => {
                self.screen = Screen::Candidates;
            }
            (Screen::Review { path, report, .. }, Action::UseFolder) => {
                if !report.is_usable() {
                    return false;
                }
                match host.commit(&path) {
                    Ok(summary) => self.screen = Screen::Done(summary),
                    Err(error) => {
                        self.screen = Screen::Review {
                            path,
                            report,
                            error: Some(error),
                        }
                    }
                }
            }
            (Screen::Done(summary), Action::Start) => {
                self.outcome = Some(WizardOutcome::Completed(summary));
            }
            _ => return false,
        }
        true
    }

    /// Check for a folder-picker answer. Returns whether anything changed.
    pub fn poll(&mut self, host: &dyn WizardHost) -> bool {
        let Some(receiver) = &self.pending_pick else {
            return false;
        };
        let polled = match receiver.lock() {
            Ok(receiver) => receiver.try_recv(),
            Err(_) => Err(TryRecvError::Disconnected),
        };
        let answer = match polled {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => None,
        };
        self.pending_pick = None;
        match answer {
            Some(path) => {
                let report = host.validate(&path);
                // Show the folder that actually holds the files when the
                // data sits in a subfolder of the chosen one.
                self.screen = Screen::Review {
                    path: report.dir.clone(),
                    report: Box::new(report),
                    error: None,
                };
            }
            None => {
                self.notice = Some("No folder was chosen.".to_string());
                self.screen = Screen::Candidates;
            }
        }
        true
    }
}
