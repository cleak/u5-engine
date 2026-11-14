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

// The Enhancements page's model lives in `u5_runtime::enhancements::page`
// (build plan D-M4.5), shared with the menu overlay's Settings tabs. It is
// re-exported here, so the wizard's code and tests are unchanged.
pub use u5_runtime::enhancements::page::{
    Confirm, FocusItem, MixLabel, PresetChoice, RowKind, RowRole, RowValue, SettingRow,
    SettingsChanges, SettingsPage, SettingsRows,
};

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
