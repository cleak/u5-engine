//! Case-insensitive file-name resolution and data-root location.
//!
//! The original release uses upper-case 8.3 names, but copies made by
//! extractors, archive tools or case-insensitive file systems often arrive in
//! lower or mixed case. On a case-sensitive file system the engine would then
//! fail to open them, so everything here matches names ignoring ASCII case.

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::{DataFile, MANIFEST};

/// Upper-case form used as the canonical (on-disk, as-shipped) name.
pub fn canonical_name(name: &str) -> String {
    name.to_ascii_uppercase()
}

/// The regular files of one directory, keyed by canonical (upper-case) name.
#[derive(Clone, Debug, Default)]
pub struct DirectoryIndex {
    dir: PathBuf,
    /// canonical name -> every actual entry that folds to it (sorted, so an
    /// exact-case match, if any, can be preferred deterministically).
    entries: BTreeMap<String, Vec<String>>,
}

impl DirectoryIndex {
    /// Index the regular files (or symlinks to files) directly inside `dir`.
    pub fn read(dir: &Path) -> io::Result<Self> {
        let mut entries: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for entry in fs::read_dir(dir)? {
            let Ok(entry) = entry else { continue };
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            // `metadata` follows symlinks, so a linked file counts.
            let is_file = fs::metadata(entry.path())
                .map(|meta| meta.is_file())
                .unwrap_or(false);
            if is_file {
                entries.entry(canonical_name(&name)).or_default().push(name);
            }
        }
        for names in entries.values_mut() {
            names.sort();
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            entries,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Resolve a canonical name to the actual file in this directory.
    ///
    /// An exact-case entry wins; otherwise the first case-folded match.
    pub fn resolve(&self, name: &str) -> Option<ResolvedFile> {
        let canonical = canonical_name(name);
        let names = self.entries.get(&canonical)?;
        let actual = names
            .iter()
            .find(|candidate| **candidate == canonical)
            .unwrap_or(&names[0]);
        Some(ResolvedFile {
            canonical,
            actual_name: actual.clone(),
            path: self.dir.join(actual),
        })
    }

    /// Canonical names that appear more than once with different case
    /// (`brit.dat` next to `BRIT.DAT`); worth a warning, never fatal.
    pub fn ambiguous_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, names)| names.len() > 1)
            .map(|(canonical, _)| canonical.clone())
            .collect()
    }

    /// How many manifest files this directory resolves.
    pub fn manifest_hits(&self, files: &[DataFile]) -> usize {
        files
            .iter()
            .filter(|file| self.entries.contains_key(file.name))
            .count()
    }
}

/// A manifest name resolved against a real directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedFile {
    /// Upper-case name the engine opens.
    pub canonical: String,
    /// The name as it exists on disk.
    pub actual_name: String,
    pub path: PathBuf,
}

impl ResolvedFile {
    pub fn case_differs(&self) -> bool {
        self.actual_name != self.canonical
    }
}

/// Limits for [`locate_data_root`].
#[derive(Clone, Copy, Debug)]
pub struct SearchLimits {
    /// How many directory levels below the start to look (0 = start only).
    pub max_depth: usize,
    /// Stop after visiting this many directories in total.
    pub max_dirs: usize,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_depth: 4,
            max_dirs: 4000,
        }
    }
}

/// Find the directory at or below `start` that holds the game data.
///
/// Installers nest the DOS files in different places (a bundle of several
/// games, a `data` or `game` subfolder, an app bundle), so this walks a few
/// levels breadth first and returns the shallowest directory that resolves
/// the most required manifest files. A directory must resolve at least half
/// of them to count, which keeps unrelated folders (or another Ultima with a
/// few shared names) from being picked. Hidden directories are skipped.
pub fn locate_data_root(start: &Path, limits: SearchLimits) -> Option<PathBuf> {
    let required: Vec<DataFile> = MANIFEST
        .iter()
        .copied()
        .filter(|file| file.is_required())
        .collect();
    let threshold = required.len().div_ceil(2);
    let mut best: Option<(usize, PathBuf)> = None;
    let mut queue = VecDeque::from([(start.to_path_buf(), 0usize)]);
    let mut visited = 0usize;
    while let Some((dir, depth)) = queue.pop_front() {
        visited += 1;
        if visited > limits.max_dirs {
            break;
        }
        let Ok(index) = DirectoryIndex::read(&dir) else {
            continue;
        };
        let hits = index.manifest_hits(&required);
        if hits == required.len() {
            // Breadth first: nothing deeper can beat a complete shallow hit.
            return Some(dir);
        }
        if hits >= threshold && best.as_ref().is_none_or(|(score, _)| hits > *score) {
            best = Some((hits, dir.clone()));
        }
        if depth >= limits.max_depth {
            continue;
        }
        let Ok(children) = fs::read_dir(&dir) else {
            continue;
        };
        let mut subdirs: Vec<PathBuf> = children
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry.file_name().to_string_lossy().starts_with('.')
                    && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
            })
            .map(|entry| entry.path())
            .collect();
        subdirs.sort();
        queue.extend(subdirs.into_iter().map(|path| (path, depth + 1)));
    }
    best.map(|(_, dir)| dir)
}
