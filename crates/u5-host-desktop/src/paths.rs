//! Desktop per-user locations: configuration, managed game data, runtime
//! (save) mirrors and the log file.
//!
//! | | Windows | macOS | Linux and other Unix |
//! |---|---|---|---|
//! | config | `%APPDATA%\u5-engine` | `~/Library/Application Support/u5-engine` | `$XDG_CONFIG_HOME/u5-engine` (default `~/.config/u5-engine`) |
//! | data | `%LOCALAPPDATA%\u5-engine` | `~/Library/Application Support/u5-engine` | `$XDG_DATA_HOME/u5-engine` (default `~/.local/share/u5-engine`) |
//!
//! Environment overrides (used by tests and harnesses): `U5_ENGINE_CONFIG`
//! (the config file itself), `U5_ENGINE_DATA_DIR` (the data base folder) and
//! `U5_ENGINE_RUNTIME_DIR` (the runtime save-mirror root, as before).

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "u5-engine";
pub const CONFIG_ENV: &str = "U5_ENGINE_CONFIG";
pub const DATA_DIR_ENV: &str = "U5_ENGINE_DATA_DIR";
pub const RUNTIME_DIR_ENV: &str = "U5_ENGINE_RUNTIME_DIR";
pub const GAME_DIR_ENV: &str = "U5_GAME_DIR";
/// The pre-1.1 fallback runtime root under the system temp folder.
pub const LEGACY_TEMP_RUNTIME_NAME: &str = "u5-engine-runtime";

/// Which desktop family's conventions to follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    /// Linux and other freedesktop-style Unix systems.
    Unix,
}

impl Platform {
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Unix
        }
    }
}

/// A snapshot of the environment the paths are derived from, so the rules
/// can be tested for every platform on any platform.
#[derive(Clone, Debug)]
pub struct HostEnv {
    pub platform: Platform,
    pub vars: HashMap<String, OsString>,
    pub home: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub current_dir: Option<PathBuf>,
    pub exe_dir: Option<PathBuf>,
}

impl HostEnv {
    /// The real process environment.
    pub fn from_process() -> Self {
        let vars: HashMap<String, OsString> = std::env::vars_os()
            .filter_map(|(key, value)| key.into_string().ok().map(|key| (key, value)))
            .collect();
        let platform = Platform::current();
        let home = match platform {
            Platform::Windows => vars.get("USERPROFILE").cloned(),
            _ => vars.get("HOME").cloned(),
        }
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
        Self {
            platform,
            vars,
            home,
            temp_dir: std::env::temp_dir(),
            current_dir: std::env::current_dir().ok(),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
        }
    }

    /// A non-empty variable as a path.
    pub fn var_path(&self, key: &str) -> Option<PathBuf> {
        self.vars
            .get(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }

    fn home_join(&self, parts: &[&str]) -> Option<PathBuf> {
        let mut path = self.home.clone()?;
        for part in parts {
            path.push(part);
        }
        Some(path)
    }
}

/// Every per-user location the desktop host uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopPaths {
    pub config_file: PathBuf,
    /// Base folder for managed data (`game-data/`, the log).
    pub data_base: PathBuf,
    /// Root of the per-source writable runtime mirrors.
    pub runtime_root: PathBuf,
}

impl DesktopPaths {
    pub fn from_env(env: &HostEnv) -> Self {
        Self {
            config_file: config_file(env),
            data_base: data_base(env),
            runtime_root: runtime_root(env),
        }
    }

    /// Where managed (normalised) copies of game data live.
    pub fn game_data_root(&self) -> PathBuf {
        self.data_base.join("game-data")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_base.join("u5-engine.log")
    }
}

/// The configuration file.
pub fn config_file(env: &HostEnv) -> PathBuf {
    if let Some(path) = env.var_path(CONFIG_ENV) {
        return path;
    }
    config_base(env).join("config.toml")
}

fn config_base(env: &HostEnv) -> PathBuf {
    let base = match env.platform {
        Platform::Windows => env.var_path("APPDATA"),
        Platform::MacOs => env.home_join(&["Library", "Application Support"]),
        Platform::Unix => env
            .var_path("XDG_CONFIG_HOME")
            .or_else(|| env.home_join(&[".config"])),
    };
    base.unwrap_or_else(|| env.temp_dir.clone())
        .join(APP_DIR_NAME)
}

/// The data base folder (managed game data, log).
pub fn data_base(env: &HostEnv) -> PathBuf {
    if let Some(path) = env.var_path(DATA_DIR_ENV) {
        return path;
    }
    platform_data_base(env).unwrap_or_else(|| env.temp_dir.join(APP_DIR_NAME))
}

/// `%LOCALAPPDATA%\u5-engine`, `~/Library/Application Support/u5-engine`
/// or `$XDG_DATA_HOME/u5-engine` (default `~/.local/share/u5-engine`).
fn platform_data_base(env: &HostEnv) -> Option<PathBuf> {
    let base = match env.platform {
        Platform::Windows => env.var_path("LOCALAPPDATA"),
        Platform::MacOs => env.home_join(&["Library", "Application Support"]),
        Platform::Unix => env
            .var_path("XDG_DATA_HOME")
            .or_else(|| env.home_join(&[".local", "share"])),
    }?;
    Some(base.join(APP_DIR_NAME))
}

/// Root of the writable runtime mirrors that hold saves.
///
/// `U5_ENGINE_RUNTIME_DIR` wins (harnesses set it). Otherwise
/// `%LOCALAPPDATA%` and `$XDG_DATA_HOME` are honoured wherever they are set,
/// as before, and then the platform default: `~/Library/Application
/// Support/u5-engine/runtime` on macOS and `~/.local/share/u5-engine/runtime`
/// elsewhere. The system temp folder is used only when there is no home
/// folder at all; earlier versions fell back to it whenever
/// `XDG_DATA_HOME` was unset, which put saves somewhere a reboot can erase
/// (see [`migrate_legacy_runtime_root`]).
pub fn runtime_root(env: &HostEnv) -> PathBuf {
    if let Some(path) = env.var_path(RUNTIME_DIR_ENV) {
        return path;
    }
    if let Some(path) = env.var_path("LOCALAPPDATA") {
        return path.join(APP_DIR_NAME).join("runtime");
    }
    if let Some(path) = env.var_path("XDG_DATA_HOME") {
        return path.join(APP_DIR_NAME).join("runtime");
    }
    match env.platform {
        Platform::MacOs => {
            env.home_join(&["Library", "Application Support", APP_DIR_NAME, "runtime"])
        }
        _ => env.home_join(&[".local", "share", APP_DIR_NAME, "runtime"]),
    }
    .unwrap_or_else(|| legacy_temp_runtime_root(env))
}

/// The old temp-folder runtime root.
pub fn legacy_temp_runtime_root(env: &HostEnv) -> PathBuf {
    env.temp_dir.join(LEGACY_TEMP_RUNTIME_NAME)
}

/// Copy saves from the old temp-folder runtime root into `root`.
///
/// Runs only when no override is set, the old folder exists, it is not
/// `root` itself, and `root` does not exist yet (so it happens once and
/// never mixes two sets of saves). The old folder is left in place. Returns
/// a message to show the player when something was migrated.
pub fn migrate_legacy_runtime_root(env: &HostEnv, root: &Path) -> io::Result<Option<String>> {
    if env.var_path(RUNTIME_DIR_ENV).is_some() {
        return Ok(None);
    }
    let legacy = legacy_temp_runtime_root(env);
    if !legacy.is_dir() || root.exists() || legacy == root {
        return Ok(None);
    }
    copy_tree(&legacy, root)?;
    Ok(Some(format!(
        "Moved your saved games from the temporary folder {} to {} (the temporary copy \
         was left in place; you can delete it).",
        legacy.display(),
        root.display()
    )))
}

fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else if kind.is_file() {
            fs::copy(&from, &to)?;
            let mut permissions = fs::metadata(&to)?.permissions();
            if permissions.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                permissions.set_readonly(false);
                fs::set_permissions(&to, permissions)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(platform: Platform, vars: &[(&str, &str)], home: Option<&str>) -> HostEnv {
        HostEnv {
            platform,
            vars: vars
                .iter()
                .map(|(k, v)| ((*k).to_string(), OsString::from(v)))
                .collect(),
            home: home.map(PathBuf::from),
            temp_dir: PathBuf::from("/tmp"),
            current_dir: None,
            exe_dir: None,
        }
    }

    #[test]
    fn unix_defaults_and_xdg() {
        let plain = env(Platform::Unix, &[], Some("/home/ann"));
        let paths = DesktopPaths::from_env(&plain);
        assert_eq!(
            paths.config_file,
            PathBuf::from("/home/ann/.config/u5-engine/config.toml")
        );
        assert_eq!(
            paths.data_base,
            PathBuf::from("/home/ann/.local/share/u5-engine")
        );
        assert_eq!(
            paths.runtime_root,
            PathBuf::from("/home/ann/.local/share/u5-engine/runtime")
        );
        assert_eq!(
            paths.game_data_root(),
            PathBuf::from("/home/ann/.local/share/u5-engine/game-data")
        );

        let xdg = env(
            Platform::Unix,
            &[
                ("XDG_CONFIG_HOME", "/cfg"),
                ("XDG_DATA_HOME", "/data"),
                ("XDG_STATE_HOME", ""),
            ],
            Some("/home/ann"),
        );
        let paths = DesktopPaths::from_env(&xdg);
        assert_eq!(
            paths.config_file,
            PathBuf::from("/cfg/u5-engine/config.toml")
        );
        assert_eq!(paths.data_base, PathBuf::from("/data/u5-engine"));
        assert_eq!(paths.runtime_root, PathBuf::from("/data/u5-engine/runtime"));
    }

    #[test]
    fn unix_without_xdg_data_home_no_longer_uses_temp() {
        let plain = env(Platform::Unix, &[("XDG_DATA_HOME", "")], Some("/home/ann"));
        assert!(!runtime_root(&plain).starts_with("/tmp"));
        // Only a missing home falls back to temp.
        let homeless = env(Platform::Unix, &[], None);
        assert_eq!(
            runtime_root(&homeless),
            PathBuf::from("/tmp/u5-engine-runtime")
        );
    }

    #[test]
    fn macos_uses_application_support() {
        let mac = env(Platform::MacOs, &[], Some("/Users/ann"));
        let paths = DesktopPaths::from_env(&mac);
        assert_eq!(
            paths.config_file,
            PathBuf::from("/Users/ann/Library/Application Support/u5-engine/config.toml")
        );
        assert_eq!(
            paths.runtime_root,
            PathBuf::from("/Users/ann/Library/Application Support/u5-engine/runtime")
        );
    }

    #[test]
    fn windows_uses_appdata_and_localappdata() {
        let win = env(
            Platform::Windows,
            &[
                ("APPDATA", r"C:\Users\ann\AppData\Roaming"),
                ("LOCALAPPDATA", r"C:\Users\ann\AppData\Local"),
            ],
            Some(r"C:\Users\ann"),
        );
        let paths = DesktopPaths::from_env(&win);
        assert_eq!(
            paths.config_file,
            PathBuf::from(r"C:\Users\ann\AppData\Roaming")
                .join("u5-engine")
                .join("config.toml")
        );
        assert_eq!(
            paths.runtime_root,
            PathBuf::from(r"C:\Users\ann\AppData\Local")
                .join("u5-engine")
                .join("runtime")
        );
    }

    #[test]
    fn overrides_win() {
        let over = env(
            Platform::Unix,
            &[
                (CONFIG_ENV, "/x/config.toml"),
                (DATA_DIR_ENV, "/x/data"),
                (RUNTIME_DIR_ENV, "/x/runtime"),
                ("XDG_DATA_HOME", "/data"),
            ],
            Some("/home/ann"),
        );
        let paths = DesktopPaths::from_env(&over);
        assert_eq!(paths.config_file, PathBuf::from("/x/config.toml"));
        assert_eq!(paths.data_base, PathBuf::from("/x/data"));
        assert_eq!(paths.runtime_root, PathBuf::from("/x/runtime"));
    }

    fn scratch(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-host-paths-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn legacy_temp_saves_are_migrated_once() {
        let base = scratch("migrate");
        let mut host = env(
            Platform::Unix,
            &[],
            Some(base.join("home").to_str().unwrap()),
        );
        host.temp_dir = base.join("tmp");
        let legacy = legacy_temp_runtime_root(&host).join("game-0123");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("SAVED.GAM"), b"progress").unwrap();

        let root = runtime_root(&host);
        assert!(root.starts_with(base.join("home")));
        let message = migrate_legacy_runtime_root(&host, &root).unwrap();
        assert!(message.unwrap().contains("Moved your saved games"));
        assert_eq!(
            fs::read(root.join("game-0123").join("SAVED.GAM")).unwrap(),
            b"progress"
        );
        // Second launch: the new root exists, nothing happens again.
        fs::write(legacy.join("SAVED.GAM"), b"older").unwrap();
        assert_eq!(migrate_legacy_runtime_root(&host, &root).unwrap(), None);
        assert_eq!(
            fs::read(root.join("game-0123").join("SAVED.GAM")).unwrap(),
            b"progress"
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn no_migration_under_override_or_without_legacy() {
        let base = scratch("nomigrate");
        let mut host = env(
            Platform::Unix,
            &[(RUNTIME_DIR_ENV, base.join("qa").to_str().unwrap())],
            Some(base.join("home").to_str().unwrap()),
        );
        host.temp_dir = base.join("tmp");
        fs::create_dir_all(legacy_temp_runtime_root(&host)).unwrap();
        let root = runtime_root(&host);
        assert_eq!(root, base.join("qa"));
        assert_eq!(migrate_legacy_runtime_root(&host, &root).unwrap(), None);
        assert!(!root.exists());

        host.vars.clear();
        fs::remove_dir_all(legacy_temp_runtime_root(&host)).unwrap();
        let root = runtime_root(&host);
        assert_eq!(migrate_legacy_runtime_root(&host, &root).unwrap(), None);
        fs::remove_dir_all(base).unwrap();
    }
}
