//! The desktop resolution pipeline.
//!
//! Order: `--game-dir` / positional argument, then `U5_GAME_DIR`, then the
//! saved configuration, then discovery; when none of those yields usable
//! data the caller has to run a setup wizard.
//!
//! A folder given on the command line (or in `U5_GAME_DIR`) whose files all
//! carry their canonical names is used exactly as given, as it always was:
//! the QA harnesses and fixtures depend on that. Anything else (a saved or
//! discovered folder, a chosen folder, or a given folder that only works
//! case-insensitively or holds the data in a subfolder) is validated and
//! installed into a managed, case-correct copy, and the engine runs from
//! that copy.

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use u5_gamedata::{
    Discovered, InstallError, SearchLimits, ValidationReport, discover, install, required_files,
    same_dir, validate_dir, validate_location,
};

use crate::config::{Config, ConfigError};
use crate::discovery::DesktopDiscovery;
use crate::paths::{DesktopPaths, GAME_DIR_ENV, HostEnv};

/// How far below a chosen or given folder to look for the data.
pub const CHOSEN_FOLDER_LIMITS: SearchLimits = SearchLimits {
    max_depth: 4,
    max_dirs: 4000,
};

/// Where the game folder came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    CommandLine,
    Environment,
    SavedConfig,
    Discovered { source: String },
    Setup,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandLine => write!(f, "the command line"),
            Self::Environment => write!(f, "{GAME_DIR_ENV}"),
            Self::SavedConfig => write!(f, "saved settings"),
            Self::Discovered { source } => write!(f, "automatic search ({source})"),
            Self::Setup => write!(f, "setup"),
        }
    }
}

/// Game data ready for the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadyGameData {
    /// The folder to hand the engine.
    pub data_dir: PathBuf,
    /// The folder the data came from.
    pub source_dir: PathBuf,
    pub origin: Origin,
    /// Release label (`gog-1.0`, `unknown`), when it was checked.
    pub version: Option<String>,
    /// For a given folder used as-is: its validation report when it was
    /// incomplete (the engine is still started, exactly as before, and the
    /// report explains a later failure).
    pub incomplete: Option<Box<ValidationReport>>,
    /// Things worth telling the player (one line each).
    pub notes: Vec<String>,
}

/// Why a wizard is needed, and what discovery found meanwhile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardNeed {
    pub reason: String,
    pub candidates: Vec<Discovered>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Resolution {
    Ready(ReadyGameData),
    NeedsWizard(WizardNeed),
}

#[derive(Clone, Debug, Default)]
pub struct ResolveRequest {
    /// `--game-dir` or the positional argument.
    pub explicit: Option<PathBuf>,
    /// `--setup`: skip config and discovery and ask.
    pub force_setup: bool,
}

#[derive(Debug)]
pub enum SetupError {
    /// The chosen folder does not hold usable data.
    Invalid(Box<ValidationReport>),
    Install(InstallError),
    Config(ConfigError),
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(report) => write!(f, "{}", report.summary().trim_end()),
            Self::Install(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SetupError {}

impl From<InstallError> for SetupError {
    fn from(error: InstallError) -> Self {
        match error {
            InstallError::NotUsable(report) => Self::Invalid(report),
            other => Self::Install(other),
        }
    }
}

impl From<ConfigError> for SetupError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

/// The desktop host: paths, configuration, discovery and the pipeline.
#[derive(Clone, Debug)]
pub struct DesktopHost {
    pub env: HostEnv,
    pub paths: DesktopPaths,
    pub discovery: DesktopDiscovery,
}

impl DesktopHost {
    pub fn from_process() -> Self {
        Self::new(HostEnv::from_process())
    }

    pub fn new(env: HostEnv) -> Self {
        Self {
            paths: DesktopPaths::from_env(&env),
            discovery: DesktopDiscovery::new(env.clone()),
            env,
        }
    }

    /// Run the pipeline.
    pub fn resolve(&self, request: &ResolveRequest) -> Resolution {
        if let Some(dir) = &request.explicit {
            if request.force_setup {
                // `--setup <DIR>`: set that folder up without asking.
                return match self.commit(dir, Origin::CommandLine) {
                    Ok(ready) => Resolution::Ready(ready),
                    Err(error) => Resolution::NeedsWizard(WizardNeed {
                        reason: format!("{} cannot be used:\n{error}", dir.display()),
                        candidates: self.discover(),
                        notes: Vec::new(),
                    }),
                };
            }
            return Resolution::Ready(self.use_given_folder(dir, Origin::CommandLine));
        }
        if request.force_setup {
            return Resolution::NeedsWizard(WizardNeed {
                reason: "Setup was requested.".to_string(),
                candidates: self.discover(),
                notes: Vec::new(),
            });
        }
        if let Some(dir) = self.env.var_path(GAME_DIR_ENV) {
            return Resolution::Ready(self.use_given_folder(&dir, Origin::Environment));
        }
        let mut notes = Vec::new();
        match self.resolve_from_config() {
            Ok(Some(ready)) => return Resolution::Ready(ready),
            Ok(None) => {}
            Err(note) => notes.push(note),
        }
        let candidates = self.discover();
        for found in candidates.iter().filter(|found| found.report.is_usable()) {
            match self.commit(
                &found.data_root,
                Origin::Discovered {
                    source: found.candidate.source.clone(),
                },
            ) {
                Ok(mut ready) => {
                    ready.notes.splice(0..0, notes);
                    ready.notes.push(format!(
                        "Found the game files in {}. Run with --setup to choose a different folder.",
                        found.data_root.display()
                    ));
                    return Resolution::Ready(ready);
                }
                Err(error) => notes.push(format!(
                    "Found game files in {} but could not set them up: {error}",
                    found.data_root.display()
                )),
            }
        }
        Resolution::NeedsWizard(WizardNeed {
            reason: if candidates.is_empty() {
                "No Ultima V game files were found on this computer.".to_string()
            } else {
                "Some folders look like Ultima V, but none has every file the game needs."
                    .to_string()
            },
            candidates,
            notes,
        })
    }

    /// The saved configuration's data, when it still validates. `Err` is a
    /// note for the player about a stale or damaged configuration.
    fn resolve_from_config(&self) -> Result<Option<ReadyGameData>, String> {
        let config = Config::load(&self.paths.config_file)
            .map_err(|error| format!("Ignoring settings: {error}"))?;
        if let Some(data_dir) = &config.data_dir {
            let report = validate_dir(data_dir);
            if report.is_usable() && report.case_fixes().is_empty() {
                return Ok(Some(ReadyGameData {
                    data_dir: data_dir.clone(),
                    source_dir: config.game_dir.clone().unwrap_or_else(|| data_dir.clone()),
                    origin: Origin::SavedConfig,
                    version: Some(report.version.label()),
                    incomplete: None,
                    notes: Vec::new(),
                }));
            }
        }
        let Some(game_dir) = &config.game_dir else {
            return Ok(None);
        };
        match self.commit(game_dir, Origin::SavedConfig) {
            Ok(ready) => Ok(Some(ready)),
            Err(_) => Err(format!(
                "The game folder in your settings ({}) no longer has the game files.",
                game_dir.display()
            )),
        }
    }

    /// Use a folder the player named directly (command line or
    /// environment). See the module docs for when it is used as-is.
    pub fn use_given_folder(&self, dir: &Path, origin: Origin) -> ReadyGameData {
        if has_every_required_file_exactly(dir) {
            return ReadyGameData {
                data_dir: dir.to_path_buf(),
                source_dir: dir.to_path_buf(),
                origin,
                version: None,
                incomplete: None,
                notes: Vec::new(),
            };
        }
        let report = validate_location(dir, CHOSEN_FOLDER_LIMITS);
        if report.is_usable() {
            let dest = self.managed_dir_for(&report.dir);
            match install(&report, &dest) {
                Ok(outcome) => {
                    let why = if report.dir != dir {
                        format!(
                            "the game files are in its subfolder {}",
                            report.dir.display()
                        )
                    } else {
                        "its file names are not in the original upper case".to_string()
                    };
                    return ReadyGameData {
                        data_dir: outcome.data_dir,
                        source_dir: report.dir.clone(),
                        origin,
                        version: Some(report.version.label()),
                        incomplete: None,
                        notes: vec![format!(
                            "Using a normalised copy of {} in {} because {why}.",
                            dir.display(),
                            dest.display()
                        )],
                    };
                }
                Err(error) => {
                    return ReadyGameData {
                        data_dir: dir.to_path_buf(),
                        source_dir: dir.to_path_buf(),
                        origin,
                        version: None,
                        incomplete: None,
                        notes: vec![format!("Could not make a normalised copy: {error}")],
                    };
                }
            }
        }
        // Incomplete: hand it over unchanged (fixtures and partial test
        // folders rely on this) but keep the report for error messages.
        let report = validate_dir(dir);
        ReadyGameData {
            data_dir: dir.to_path_buf(),
            source_dir: dir.to_path_buf(),
            origin,
            version: None,
            incomplete: Some(Box::new(report)),
            notes: Vec::new(),
        }
    }

    /// Validate `source` (searching a few levels down), install it into its
    /// managed copy and remember it in the configuration.
    pub fn commit(&self, source: &Path, origin: Origin) -> Result<ReadyGameData, SetupError> {
        let report = self.validate(source);
        if !report.is_usable() {
            return Err(SetupError::Invalid(Box::new(report)));
        }
        let dest = self.managed_dir_for(&report.dir);
        let outcome = install(&report, &dest)?;
        let version = report.version.label();
        let config = Config {
            game_dir: Some(absolute(&report.dir)),
            data_dir: Some(absolute(&outcome.data_dir)),
            version: Some(version.clone()),
        };
        config.save(&self.paths.config_file)?;
        let mut notes = Vec::new();
        if let u5_gamedata::VersionIdentification::Unknown { .. } = report.version {
            notes.push(
                "Warning: these game files differ from the release this engine was verified \
                 against. The game will probably work; if something looks wrong, try the \
                 GOG.com release."
                    .to_string(),
            );
        }
        Ok(ReadyGameData {
            data_dir: outcome.data_dir,
            source_dir: report.dir,
            origin,
            version: Some(version),
            incomplete: None,
            notes,
        })
    }

    /// Validate a folder the player is considering.
    pub fn validate(&self, path: &Path) -> ValidationReport {
        validate_location(path, CHOSEN_FOLDER_LIMITS)
    }

    pub fn discover(&self) -> Vec<Discovered> {
        discover(&[&self.discovery])
    }

    pub fn load_config(&self) -> Result<Config, ConfigError> {
        Config::load(&self.paths.config_file)
    }

    /// The managed copy for a data folder: one per source folder, so two
    /// installs never share saves. A folder already inside the managed area
    /// is its own copy.
    pub fn managed_dir_for(&self, data_root: &Path) -> PathBuf {
        let managed_root = self.paths.game_data_root();
        if let (Ok(root), Ok(managed)) = (data_root.canonicalize(), managed_root.canonicalize())
            && root.starts_with(&managed)
            && root != managed
        {
            return data_root.to_path_buf();
        }
        if same_dir(data_root, &managed_root) {
            return data_root.to_path_buf();
        }
        managed_root.join(managed_directory_name(data_root))
    }
}

/// Every required file exists under its exact canonical name (no reads).
pub fn has_every_required_file_exactly(dir: &Path) -> bool {
    required_files().all(|file| dir.join(file.name).is_file())
}

fn absolute(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// `<folder name>-<16 hex digits of a hash of the full path>`.
pub fn managed_directory_name(source: &Path) -> String {
    let source = absolute(source);
    let label: String = source
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("ultima-v")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let normalized = source
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in normalized.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{label}-{hash:016x}")
}
