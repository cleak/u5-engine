//! The desktop configuration file (`config.toml`).
//!
//! It remembers the game folder the player chose, the managed (normalised)
//! copy the engine actually runs from, and the release that copy was
//! validated as. Unknown keys are ignored, and saving keeps them (through
//! [`ConfigDocument`]), so older and newer builds can share one file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::Value;

use crate::settings::ConfigDocument;

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

    /// Write this config's three keys into the file (creating its folder),
    /// via a temporary file.
    ///
    /// Every other key, section and comment in the file is kept, so setup
    /// never deletes enhancement settings (Phase B design 1.3.4). With no
    /// file, the text 1.0.1 wrote is written. A file that is not valid TOML
    /// is first copied to `config.toml.bak`, then replaced by that text. An
    /// unchanged file is not rewritten.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let body = toml::to_string_pretty(self)
            .map_err(|error| ConfigError::Parse(path.to_path_buf(), error.to_string()))?;
        let text = format!(
            "# Ultima V engine settings. Written by the setup wizard; safe to edit.\n\
             # game_dir: the folder you chose. data_dir: the managed copy the game runs from.\n\
             {body}"
        );
        let io = |error| ConfigError::Io(path.to_path_buf(), error);
        let mut document = ConfigDocument::load(path).map_err(io)?;
        document
            .start_fresh(&text)
            .map_err(|error| ConfigError::Parse(path.to_path_buf(), error.to_string()))?;
        // `to_string_pretty` succeeded, so every path is UTF-8.
        let path_value =
            |dir: &Option<PathBuf>| dir.as_ref().and_then(|dir| dir.to_str()).map(Value::from);
        document.assign("game_dir", path_value(&self.game_dir));
        document.assign("data_dir", path_value(&self.data_dir));
        document.assign("version", self.version.as_deref().map(Value::from));
        document.save_if_changed().map_err(io)?;
        Ok(())
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

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "u5-host-config-{label}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn config(&self) -> PathBuf {
            self.0.join("config.toml")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample() -> Config {
        Config {
            game_dir: Some(PathBuf::from(r"C:\GOG Games\Ultima 5")),
            data_dir: Some(PathBuf::from(
                "/home/ann/.local/share/u5-engine/game-data/x",
            )),
            version: Some("gog-1.0".into()),
        }
    }

    /// What 1.0.1's `Config::save` wrote for [`sample`], byte for byte.
    const SAMPLE_1_0_1: &str = "\
# Ultima V engine settings. Written by the setup wizard; safe to edit.
# game_dir: the folder you chose. data_dir: the managed copy the game runs from.
game_dir = 'C:\\GOG Games\\Ultima 5'
data_dir = \"/home/ann/.local/share/u5-engine/game-data/x\"
version = \"gog-1.0\"
";

    #[test]
    fn with_no_file_the_1_0_1_bytes_are_written() {
        let scratch = Scratch::new("golden");
        let path = scratch.config();
        sample().save(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE_1_0_1);
        // Matches the serialiser 1.0.1 used, header included.
        let body = toml::to_string_pretty(&sample()).unwrap();
        assert!(SAMPLE_1_0_1.ends_with(&body));
        assert_eq!(SAMPLE_1_0_1.lines().count(), body.lines().count() + 2);

        let partial = scratch.0.join("partial.toml");
        Config {
            version: Some("unknown".into()),
            ..Config::default()
        }
        .save(&partial)
        .unwrap();
        assert!(
            fs::read_to_string(&partial)
                .unwrap()
                .ends_with("\nversion = \"unknown\"\n")
        );
    }

    #[test]
    fn a_1_0_1_file_round_trips_byte_for_byte_and_is_not_rewritten() {
        let scratch = Scratch::new("unchanged");
        let path = scratch.config();
        fs::write(&path, SAMPLE_1_0_1).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config, sample());
        #[cfg(unix)]
        let inode = {
            use std::os::unix::fs::MetadataExt as _;
            fs::metadata(&path).unwrap().ino()
        };
        config.save(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE_1_0_1);
        // Saved only when changed: the file was not replaced.
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
        }
    }

    #[test]
    fn save_keeps_every_section_it_does_not_own() {
        let scratch = Scratch::new("sections");
        let path = scratch.config();
        let extra = "\
future_key = 1 # from a newer build

[enhancements]
preset = \"modern\"

[display]
vsync = false
";
        let original = format!("{SAMPLE_1_0_1}{extra}");
        fs::write(&path, &original).unwrap();
        let moved = Config {
            game_dir: Some(PathBuf::from("/games/u5")),
            data_dir: None,
            version: Some("unknown".into()),
        };
        moved.save(&path).unwrap();
        let expected = original
            .replace(r"'C:\GOG Games\Ultima 5'", "\"/games/u5\"")
            .replace(
                "data_dir = \"/home/ann/.local/share/u5-engine/game-data/x\"\n",
                "",
            )
            .replace("\"gog-1.0\"", "\"unknown\"");
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        assert_eq!(Config::load(&path).unwrap(), moved);
    }

    #[test]
    fn a_damaged_file_is_backed_up_then_replaced_by_the_1_0_1_text() {
        let scratch = Scratch::new("damaged");
        let path = scratch.config();
        let damaged = "game_dir = [\n[display]\nvsync = false\n";
        fs::write(&path, damaged).unwrap();
        assert!(Config::load(&path).is_err());
        sample().save(&path).unwrap();
        assert_eq!(
            fs::read_to_string(scratch.0.join("config.toml.bak")).unwrap(),
            damaged
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE_1_0_1);
        assert!(!scratch.0.join("config.toml.tmp").exists());
    }
}
