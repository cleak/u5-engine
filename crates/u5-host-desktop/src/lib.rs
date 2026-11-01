//! Desktop host for game-data setup: per-user paths, the `config.toml`
//! file, desktop install discovery, the resolution pipeline and native
//! dialogs.
//!
//! Everything platform-specific about *finding and remembering* the game
//! data lives here, on top of the platform-neutral `u5-gamedata`. A phone or
//! console port replaces this crate (and the wizard frontends) and keeps the
//! engine unchanged; the engine only ever receives a directory path.

pub mod config;
#[cfg(feature = "native-dialogs")]
pub mod dialogs;
pub mod discovery;
pub mod help;
pub mod paths;
pub mod pipeline;

use std::io::Write as _;

pub use config::{Config, ConfigError};
pub use discovery::{DesktopDiscovery, looks_like_ultima};
pub use help::HOW_TO_GET_THE_FILES;
pub use paths::{
    CONFIG_ENV, DATA_DIR_ENV, DesktopPaths, GAME_DIR_ENV, HostEnv, Platform, RUNTIME_DIR_ENV,
    legacy_temp_runtime_root, migrate_legacy_runtime_root, runtime_root,
};
pub use pipeline::{
    CHOSEN_FOLDER_LIMITS, DesktopHost, Origin, ReadyGameData, Resolution, ResolveRequest,
    SetupError, WizardNeed, has_every_required_file_exactly, managed_directory_name,
};

/// The runtime save-mirror root for this process, migrating saves from the
/// old temp-folder location on first use. Returns the root and, when a
/// migration happened, a message for the player.
pub fn runtime_root_for_process() -> (std::path::PathBuf, Option<String>) {
    let env = HostEnv::from_process();
    let root = runtime_root(&env);
    let message = match migrate_legacy_runtime_root(&env, &root) {
        Ok(message) => message,
        Err(error) => Some(format!(
            "Could not move saved games from {} to {}: {}",
            legacy_temp_runtime_root(&env).display(),
            root.display(),
            u5_gamedata::friendly_io_error(&error)
        )),
    };
    (root, message)
}

/// Append a timestamped line to the desktop log file (best effort).
pub fn append_log(paths: &DesktopPaths, message: &str) {
    let path = paths.log_file();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[unix {seconds}] {message}");
    }
}
