//! Where desktop installs of the game usually live.
//!
//! [`DesktopDiscovery`] lists candidate folders for the current platform;
//! `u5_gamedata::discover` then searches each a few levels deep (GOG
//! installers nest the DOS files, for example inside a multi-game bundle)
//! and validates what it finds. Broad roots such as `~/Games` are not
//! searched wholesale: only their children whose names look like Ultima
//! are.

use std::fs;
use std::path::{Path, PathBuf};

use u5_gamedata::{Candidate, DiscoveryProvider};

use crate::paths::{HostEnv, Platform};

/// Depth for a folder that is (or directly holds) one game install.
const GAME_DEPTH: usize = 4;
/// Depth for a launcher/Wine folder that may hold `drive_c/...` below it.
const PREFIX_DEPTH: usize = 6;
/// Depth for the current folder and the executable's folder.
const NEARBY_DEPTH: usize = 2;

/// The desktop [`DiscoveryProvider`].
#[derive(Clone, Debug)]
pub struct DesktopDiscovery {
    env: HostEnv,
    /// Read GOG's registry keys (Windows builds only; ignored elsewhere).
    pub use_registry: bool,
}

impl DesktopDiscovery {
    pub fn new(env: HostEnv) -> Self {
        Self {
            env,
            use_registry: true,
        }
    }
}

impl DiscoveryProvider for DesktopDiscovery {
    fn candidates(&self) -> Vec<Candidate> {
        let mut list = CandidateList::default();
        if let Some(dir) = &self.env.current_dir {
            list.direct(dir.clone(), "current folder", NEARBY_DEPTH);
        }
        if let Some(dir) = &self.env.exe_dir {
            list.direct(dir.clone(), "next to the program", NEARBY_DEPTH);
        }
        match self.env.platform {
            Platform::Windows => windows_candidates(&self.env, &mut list, self.use_registry),
            Platform::MacOs => macos_candidates(&self.env, &mut list),
            Platform::Unix => unix_candidates(&self.env, &mut list),
        }
        list.items
    }
}

#[derive(Default)]
struct CandidateList {
    items: Vec<Candidate>,
}

impl CandidateList {
    /// A specific folder (listed even if it does not exist; `discover`
    /// skips missing ones, and tests can see the full list).
    fn direct(&mut self, path: PathBuf, source: &str, depth: usize) {
        if !self.items.iter().any(|item| item.path == path) {
            self.items.push(Candidate::new(path, source, depth));
        }
    }

    /// The children of `root` whose names look like an Ultima install.
    fn ultima_children(&mut self, root: &Path, source: &str, depth: usize) {
        for child in ultima_named_children(root) {
            self.direct(child, source, depth);
        }
    }
}

/// Folder names that suggest an Ultima V (or Ultima bundle) install.
pub fn looks_like_ultima(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("ultima") || lower.contains("warriors of destiny") || lower == "u5"
}

fn ultima_named_children(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().is_dir() && looks_like_ultima(&entry.file_name().to_string_lossy())
        })
        .map(|entry| entry.path())
        .collect();
    children.sort();
    children
}

/// Every subfolder of `root` (for Wine prefixes / bottles lists).
fn subfolders(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();
    children
}

/// Search a Wine-style `drive_c` for GOG installs.
fn wine_drive_c(list: &mut CandidateList, drive_c: &Path, source: &str) {
    list.ultima_children(&drive_c.join("GOG Games"), source, GAME_DEPTH);
    for program_files in ["Program Files (x86)", "Program Files"] {
        let base = drive_c.join(program_files);
        list.ultima_children(&base.join("GOG Galaxy").join("Games"), source, GAME_DEPTH);
        list.ultima_children(&base.join("GOG.com"), source, GAME_DEPTH);
    }
}

fn windows_candidates(env: &HostEnv, list: &mut CandidateList, use_registry: bool) {
    if use_registry {
        for (path, source) in registry_candidates() {
            list.direct(path, &source, GAME_DEPTH);
        }
    }
    for drive in ['C', 'D', 'E', 'F', 'G', 'H'] {
        let root = PathBuf::from(format!("{drive}:\\"));
        if drive != 'C' && !root.is_dir() {
            continue;
        }
        let gog_games = root.join("GOG Games");
        for name in [
            "Ultima 5",
            "Ultima V",
            "Ultima 4+5+6",
            "Ultima V Warriors of Destiny",
        ] {
            list.direct(gog_games.join(name), "GOG", GAME_DEPTH);
        }
        list.ultima_children(&gog_games, "GOG", GAME_DEPTH);
        list.ultima_children(
            &root.join("GOG Galaxy").join("Games"),
            "GOG Galaxy",
            GAME_DEPTH,
        );
        list.ultima_children(&root.join("Games"), "games folder", GAME_DEPTH);
    }
    for var in ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"] {
        if let Some(base) = env.var_path(var) {
            list.ultima_children(
                &base.join("GOG Galaxy").join("Games"),
                "GOG Galaxy",
                GAME_DEPTH,
            );
            list.ultima_children(&base.join("GOG.com"), "GOG", GAME_DEPTH);
        }
    }
    if env.var_path("ProgramFiles(x86)").is_none() {
        // The GOG Galaxy library default, even when the variable is absent.
        list.ultima_children(
            Path::new(r"C:\Program Files (x86)\GOG Galaxy\Games"),
            "GOG Galaxy",
            GAME_DEPTH,
        );
    }
    if let Some(home) = &env.home {
        list.ultima_children(&home.join("Games"), "games folder", GAME_DEPTH);
        list.ultima_children(&home.join("GOG Games"), "GOG", GAME_DEPTH);
    }
}

fn unix_candidates(env: &HostEnv, list: &mut CandidateList) {
    let Some(home) = env.home.clone() else {
        return;
    };
    // GOG's Linux/offline installers and hand-made copies.
    for name in ["Ultima 5", "Ultima V", "Ultima 4+5+6"] {
        list.direct(home.join("GOG Games").join(name), "GOG", GAME_DEPTH);
    }
    list.ultima_children(&home.join("GOG Games"), "GOG", GAME_DEPTH);
    // ~/Games is Lutris's and Heroic's default install root.
    list.ultima_children(&home.join("Games"), "games folder (Lutris)", PREFIX_DEPTH);
    let heroic = home.join("Games").join("Heroic");
    list.ultima_children(&heroic, "Heroic", GAME_DEPTH);
    for prefix in subfolders(&heroic.join("Prefixes")) {
        wine_drive_c(list, &prefix.join("drive_c"), "Heroic Wine prefix");
        // Some Heroic versions nest the prefix one level deeper.
        wine_drive_c(
            list,
            &prefix.join("pfx").join("drive_c"),
            "Heroic Wine prefix",
        );
    }
    // (Heroic's Flatpak build installs under ~/Games/Heroic too.)
    // Wine's default prefix.
    wine_drive_c(list, &home.join(".wine").join("drive_c"), "Wine prefix");
    // Lutris keeps per-game Wine prefixes under ~/Games/<slug> (covered
    // above) and sometimes under its data folder.
    for prefix in subfolders(&home.join(".local/share/lutris/prefixes")) {
        wine_drive_c(list, &prefix.join("drive_c"), "Lutris Wine prefix");
    }
    // Bottles, native and Flatpak.
    for bottles in [
        home.join(".local/share/bottles/bottles"),
        home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
    ] {
        for bottle in subfolders(&bottles) {
            wine_drive_c(list, &bottle.join("drive_c"), "Bottles");
        }
    }
    // DOSBox users often keep a DOS tree in their home.
    for dos in ["dos", "DOS", "dosbox", "DOSBox"] {
        list.ultima_children(&home.join(dos), "DOS folder", GAME_DEPTH);
    }
}

fn macos_candidates(env: &HostEnv, list: &mut CandidateList) {
    // GOG's macOS builds install app bundles into /Applications by
    // default; the DOS files sit inside the bundle.
    list.ultima_children(Path::new("/Applications"), "Applications", PREFIX_DEPTH);
    let Some(home) = env.home.clone() else {
        return;
    };
    list.ultima_children(&home.join("Applications"), "Applications", PREFIX_DEPTH);
    list.ultima_children(&home.join("GOG Games"), "GOG", GAME_DEPTH);
    list.ultima_children(&home.join("Games"), "games folder", GAME_DEPTH);
    list.ultima_children(
        &home.join("Library/Application Support/GOG.com/Galaxy/Applications"),
        "GOG Galaxy",
        PREFIX_DEPTH,
    );
    // Wine wrappers (CrossOver bottles).
    for bottle in subfolders(&home.join("Library/Application Support/CrossOver/Bottles")) {
        wine_drive_c(list, &bottle.join("drive_c"), "CrossOver");
    }
}

/// GOG's registry entries: `HKLM\SOFTWARE\[WOW6432Node\]GOG.com\Games\<id>`
/// with a `path` value (and `gameName`). Only Ultima entries are kept.
#[cfg(windows)]
fn registry_candidates() -> Vec<(PathBuf, String)> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    let mut found = Vec::new();
    for (hive, subkey) in [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\GOG.com\Games"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\GOG.com\Games"),
        (HKEY_CURRENT_USER, r"SOFTWARE\GOG.com\Games"),
    ] {
        let Ok(games) = RegKey::predef(hive).open_subkey_with_flags(subkey, KEY_READ) else {
            continue;
        };
        for id in games.enum_keys().filter_map(Result::ok) {
            let Ok(game) = games.open_subkey_with_flags(&id, KEY_READ) else {
                continue;
            };
            let Ok(path) = game.get_value::<String, _>("path") else {
                continue;
            };
            let name: String = game.get_value("gameName").unwrap_or_default();
            if looks_like_ultima(&name) || looks_like_ultima(&path) {
                let label = if name.is_empty() {
                    "GOG (registry)".to_string()
                } else {
                    format!("GOG: {name}")
                };
                found.push((PathBuf::from(path), label));
            }
        }
    }
    found
}

#[cfg(not(windows))]
fn registry_candidates() -> Vec<(PathBuf, String)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn scratch(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-host-discovery-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn host(platform: Platform, home: &Path) -> HostEnv {
        HostEnv {
            platform,
            vars: HashMap::new(),
            home: Some(home.to_path_buf()),
            temp_dir: std::env::temp_dir(),
            current_dir: Some(home.join("cwd")),
            exe_dir: Some(home.join("bin")),
        }
    }

    #[test]
    fn name_filter() {
        assert!(looks_like_ultima("Ultima 5"));
        assert!(looks_like_ultima("ULTIMA V - Warriors of Destiny"));
        assert!(looks_like_ultima("ultima-v"));
        assert!(looks_like_ultima("u5"));
        assert!(!looks_like_ultima("Doom"));
    }

    #[test]
    fn unix_lists_gog_lutris_heroic_wine_and_bottles() {
        let home = scratch("unix");
        for dir in [
            "GOG Games/Ultima 4+5+6",
            "Games/ultima-v",
            "Games/doom",
            "Games/Heroic/Ultima 5",
            "Games/Heroic/Prefixes/ultima5/drive_c/GOG Games/Ultima 5",
            ".wine/drive_c/GOG Games/Ultima 5",
            ".wine/drive_c/Program Files (x86)/GOG Galaxy/Games/Ultima 5",
            ".local/share/bottles/bottles/gaming/drive_c/GOG Games/Ultima V",
            ".var/app/com.usebottles.bottles/data/bottles/bottles/b2/drive_c/GOG Games/Ultima 5",
        ] {
            fs::create_dir_all(home.join(dir)).unwrap();
        }
        let discovery = DesktopDiscovery::new(host(Platform::Unix, &home));
        let candidates = discovery.candidates();
        let paths: Vec<&Path> = candidates.iter().map(|c| c.path.as_path()).collect();
        for expected in [
            "cwd",
            "bin",
            "GOG Games/Ultima 5",
            "GOG Games/Ultima 4+5+6",
            "Games/ultima-v",
            "Games/Heroic/Ultima 5",
            "Games/Heroic/Prefixes/ultima5/drive_c/GOG Games/Ultima 5",
            ".wine/drive_c/GOG Games/Ultima 5",
            ".wine/drive_c/Program Files (x86)/GOG Galaxy/Games/Ultima 5",
            ".local/share/bottles/bottles/gaming/drive_c/GOG Games/Ultima V",
            ".var/app/com.usebottles.bottles/data/bottles/bottles/b2/drive_c/GOG Games/Ultima 5",
        ] {
            assert!(
                paths.contains(&home.join(expected).as_path()),
                "missing {expected}: {paths:#?}"
            );
        }
        assert!(!paths.contains(&home.join("Games/doom").as_path()));
        // No duplicates.
        let mut sorted = paths.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), paths.len());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn macos_lists_application_bundles() {
        let home = scratch("mac");
        fs::create_dir_all(home.join("Applications/Ultima 4+5+6.app/Contents")).unwrap();
        fs::create_dir_all(home.join("Applications/Other.app")).unwrap();
        let candidates = DesktopDiscovery::new(host(Platform::MacOs, &home)).candidates();
        assert!(
            candidates.iter().any(
                |c| c.path == home.join("Applications/Ultima 4+5+6.app") && c.search_depth >= 5
            )
        );
        assert!(!candidates.iter().any(|c| c.path.ends_with("Other.app")));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn windows_lists_gog_defaults() {
        let home = scratch("win");
        let mut env = host(Platform::Windows, &home);
        env.vars
            .insert("ProgramFiles(x86)".into(), home.join("PF86").into());
        fs::create_dir_all(home.join("PF86/GOG Galaxy/Games/Ultima 5")).unwrap();
        let mut discovery = DesktopDiscovery::new(env);
        discovery.use_registry = false;
        let candidates = discovery.candidates();
        let paths: Vec<PathBuf> = candidates.iter().map(|c| c.path.clone()).collect();
        assert!(paths.contains(&PathBuf::from(r"C:\").join("GOG Games").join("Ultima 5")));
        assert!(paths.contains(&PathBuf::from(r"C:\").join("GOG Games").join("Ultima V")));
        assert!(paths.contains(&home.join("PF86/GOG Galaxy/Games/Ultima 5")));
        fs::remove_dir_all(home).unwrap();
    }
}
