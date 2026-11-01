//! The desktop configuration file (`config.toml`).
//!
//! It remembers the game folder the player chose, the managed (normalised)
//! copy the engine actually runs from, and the release that copy was
//! validated as. Unknown keys are ignored, so older and newer builds can
//! share one file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// The folder the player chose (or discovery found).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_dir: Option<PathBuf>,
    /// The managed, case-normalised copy the engine runs from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
    /// Release id the data validated as (`gog-1.0`, or `unknown`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(PathBuf, io::Error),
    Parse(PathBuf, String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, error) => write!(
                f,
                "could not access the settings file {}: {}",
                path.display(),
                u5_gamedata::friendly_io_error(error)
            ),
            Self::Parse(path, message) => write!(
                f,
                "the settings file {} is damaged ({message}); run with --setup to recreate it",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Load the file; a missing file is an empty config.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(ConfigError::Io(path.to_path_buf(), error)),
        };
        toml::from_str(&text)
            .map_err(|error| ConfigError::Parse(path.to_path_buf(), error.message().to_string()))
    }

    /// Write the file (creating its folder), via a temporary file.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let body = toml::to_string_pretty(self)
            .map_err(|error| ConfigError::Parse(path.to_path_buf(), error.to_string()))?;
        let text = format!(
            "# Ultima V engine settings. Written by the setup wizard; safe to edit.\n\
             # game_dir: the folder you chose. data_dir: the managed copy the game runs from.\n\
             {body}"
        );
        let io = |error| ConfigError::Io(path.to_path_buf(), error);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(io)?;
        }
        let temp = path.with_extension("toml.tmp");
        fs::write(&temp, text).map_err(io)?;
        fs::rename(&temp, path).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_including_windows_paths() {
        let dir = std::env::temp_dir().join(format!(
            "u5-host-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("nested").join("config.toml");
        assert_eq!(Config::load(&path).unwrap(), Config::default());
        let config = Config {
            game_dir: Some(PathBuf::from(r"C:\GOG Games\Ultima 5")),
            data_dir: Some(PathBuf::from(
                "/home/ann/.local/share/u5-engine/game-data/x",
            )),
            version: Some("gog-1.0".into()),
        };
        config.save(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# Ultima V engine settings"));
        assert_eq!(Config::load(&path).unwrap(), config);
        fs::write(&path, "game_dir = [").unwrap();
        let error = Config::load(&path).unwrap_err().to_string();
        assert!(error.contains("--setup"), "{error}");
        fs::write(&path, "future_key = 1\nversion = \"gog-1.0\"\n").unwrap();
        assert_eq!(
            Config::load(&path).unwrap().version.as_deref(),
            Some("gog-1.0")
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
