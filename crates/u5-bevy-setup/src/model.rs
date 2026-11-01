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
}

pub struct WizardModel {
    pub screen: Screen,
    pub candidates: Vec<Discovered>,
    /// Why the wizard is running (shown on the welcome screen).
    pub reason: String,
    /// A transient line under the candidate list ("No folder chosen.").
    pub notice: Option<String>,
    pub outcome: Option<WizardOutcome>,
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
            help_return: None,
            pending_pick: None,
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
