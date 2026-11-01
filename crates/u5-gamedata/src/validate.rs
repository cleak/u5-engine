//! Validation of a candidate game-data directory.
//!
//! Produces a structured [`ValidationReport`]: which manifest files were
//! found (and under what on-disk name), which are missing or unreadable, and
//! which known release the static files match. Nothing here writes.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::{DataFile, MANIFEST};
use crate::resolve::{DirectoryIndex, ResolvedFile, SearchLimits, locate_data_root};
use crate::sha256::sha256_hex;
use crate::version::{KNOWN_VERSIONS, KnownVersion};

/// Outcome for one manifest file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileStatus {
    Present {
        resolved: ResolvedFile,
        size: u64,
        sha256: String,
    },
    Missing,
    Unreadable {
        path: PathBuf,
        error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileCheck {
    pub file: DataFile,
    pub status: FileStatus,
}

impl FileCheck {
    pub fn is_present(&self) -> bool {
        matches!(self.status, FileStatus::Present { .. })
    }
}

/// Which release the static files match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionIdentification {
    /// Every hashed file matches this known release.
    Known {
        id: &'static str,
        name: &'static str,
    },
    /// All required files exist but some differ from every known release.
    /// The engine may still work; the caller should warn, not refuse.
    Unknown {
        /// The closest known release and the files that differ from it.
        closest: Option<&'static str>,
        differing: Vec<&'static str>,
    },
    /// Required files are missing or unreadable, so no identification.
    Incomplete,
}

impl VersionIdentification {
    pub fn known_id(&self) -> Option<&'static str> {
        match self {
            Self::Known { id, .. } => Some(id),
            _ => None,
        }
    }

    /// A short label suitable for a config file or a status line.
    pub fn label(&self) -> String {
        match self {
            Self::Known { id, .. } => (*id).to_string(),
            Self::Unknown { .. } => "unknown".to_string(),
            Self::Incomplete => "incomplete".to_string(),
        }
    }

    /// A human description.
    pub fn describe(&self) -> String {
        match self {
            Self::Known { name, .. } => format!("recognised as the {name}"),
            Self::Unknown { differing, .. } => format!(
                "complete, but {} file{} differ from every release this engine was \
                 verified against",
                differing.len(),
                if differing.len() == 1 { "" } else { "s" }
            ),
            Self::Incomplete => "incomplete".to_string(),
        }
    }
}

/// The structured result of validating one directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    /// The directory the files were looked for in.
    pub dir: PathBuf,
    /// Set when the directory itself could not be listed.
    pub dir_error: Option<String>,
    pub files: Vec<FileCheck>,
    pub version: VersionIdentification,
    /// Non-fatal observations (case-duplicate names and the like).
    pub warnings: Vec<String>,
}

impl ValidationReport {
    /// Every required file is present and readable.
    pub fn is_usable(&self) -> bool {
        self.dir_error.is_none()
            && self
                .files
                .iter()
                .filter(|check| check.file.is_required())
                .all(FileCheck::is_present)
    }

    pub fn missing_required(&self) -> Vec<&'static str> {
        self.files
            .iter()
            .filter(|check| check.file.is_required() && check.status == FileStatus::Missing)
            .map(|check| check.file.name)
            .collect()
    }

    pub fn unreadable(&self) -> Vec<(&'static str, &str)> {
        self.files
            .iter()
            .filter_map(|check| match &check.status {
                FileStatus::Unreadable { error, .. } => Some((check.file.name, error.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Files found under a differently cased name: `(on-disk, canonical)`.
    pub fn case_fixes(&self) -> Vec<(&str, &'static str)> {
        self.files
            .iter()
            .filter_map(|check| match &check.status {
                FileStatus::Present { resolved, .. } if resolved.case_differs() => {
                    Some((resolved.actual_name.as_str(), check.file.name))
                }
                _ => None,
            })
            .collect()
    }

    pub fn present_count(&self) -> usize {
        self.files.iter().filter(|check| check.is_present()).count()
    }

    pub fn required_count(&self) -> usize {
        self.files
            .iter()
            .filter(|check| check.file.is_required())
            .count()
    }

    /// A one-line status, e.g. for a candidate list.
    pub fn status_line(&self) -> String {
        if let Some(error) = &self.dir_error {
            return format!("cannot be read ({error})");
        }
        if self.is_usable() {
            let mut line = format!("ready - {}", self.version.describe());
            if !self.case_fixes().is_empty() {
                line.push_str("; file names will be upper-cased in the copy");
            }
            return line;
        }
        let missing = self.missing_required().len();
        let unreadable = self.unreadable().len();
        let mut parts = Vec::new();
        if missing > 0 {
            parts.push(format!(
                "{missing} of {} required files missing",
                self.required_count()
            ));
        }
        if unreadable > 0 {
            parts.push(format!(
                "{unreadable} file{} unreadable",
                if unreadable == 1 { "" } else { "s" }
            ));
        }
        format!("not usable - {}", parts.join(", "))
    }

    /// A plain-language, multi-line report for people.
    pub fn summary(&self) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "Folder: {}", self.dir.display());
        if let Some(error) = &self.dir_error {
            let _ = writeln!(text, "This folder could not be opened: {error}");
            return text;
        }
        if self.is_usable() {
            let _ = writeln!(
                text,
                "All {} required game files were found; the data is {}.",
                self.required_count(),
                self.version.describe()
            );
            if let VersionIdentification::Unknown { differing, .. } = &self.version {
                let _ = writeln!(
                    text,
                    "Warning: this copy differs from the verified release in: {}. It will \
                     probably work, but if something looks wrong, try the GOG.com release.",
                    list_names(differing, 12)
                );
            }
            let fixes = self.case_fixes();
            if !fixes.is_empty() {
                let _ = writeln!(
                    text,
                    "{} file name{} use lower or mixed case; the managed copy will use the \
                     original upper-case names.",
                    fixes.len(),
                    if fixes.len() == 1 { "" } else { "s" }
                );
            }
        } else {
            let missing = self.missing_required();
            if !missing.is_empty() {
                let _ = writeln!(
                    text,
                    "{} of the {} files the game needs {} missing from this folder:",
                    missing.len(),
                    self.required_count(),
                    if missing.len() == 1 { "is" } else { "are" }
                );
                let _ = writeln!(text, "  {}", list_names(&missing, 40));
                if missing.len() == self.required_count() {
                    let _ = writeln!(
                        text,
                        "This does not look like an Ultima V folder. Choose the folder that \
                         contains files such as BRIT.DAT and TILES.16."
                    );
                } else {
                    let _ = writeln!(
                        text,
                        "The copy looks incomplete. Re-copy the whole game folder, or \
                         reinstall the game, and try again."
                    );
                }
            }
            for (name, error) in self.unreadable() {
                let _ = writeln!(text, "{name} exists but could not be read: {error}");
            }
        }
        for warning in &self.warnings {
            let _ = writeln!(text, "Note: {warning}");
        }
        text
    }
}

fn list_names(names: &[&str], limit: usize) -> String {
    if names.len() <= limit {
        return names.join(", ");
    }
    format!(
        "{}, and {} more",
        names[..limit].join(", "),
        names.len() - limit
    )
}

/// Validate `dir` as the data root (no searching of subfolders).
pub fn validate_dir(dir: &Path) -> ValidationReport {
    validate_dir_against(dir, MANIFEST, KNOWN_VERSIONS)
}

/// Locate the data root at or below `start` (see [`locate_data_root`]) and
/// validate it. When nothing plausible is found, `start` itself is
/// validated, so the report explains what is missing there.
pub fn validate_location(start: &Path, limits: SearchLimits) -> ValidationReport {
    let root = locate_data_root(start, limits).unwrap_or_else(|| start.to_path_buf());
    validate_dir(&root)
}

/// [`validate_dir`] against an explicit manifest and version table.
pub fn validate_dir_against(
    dir: &Path,
    manifest: &[DataFile],
    versions: &[KnownVersion],
) -> ValidationReport {
    let index = match DirectoryIndex::read(dir) {
        Ok(index) => index,
        Err(error) => {
            return ValidationReport {
                dir: dir.to_path_buf(),
                dir_error: Some(friendly_io_error(&error)),
                files: manifest
                    .iter()
                    .map(|file| FileCheck {
                        file: *file,
                        status: FileStatus::Missing,
                    })
                    .collect(),
                version: VersionIdentification::Incomplete,
                warnings: Vec::new(),
            };
        }
    };
    let mut files = Vec::with_capacity(manifest.len());
    for file in manifest {
        let status = match index.resolve(file.name) {
            None => FileStatus::Missing,
            Some(resolved) => match fs::read(&resolved.path) {
                Ok(bytes) => FileStatus::Present {
                    size: bytes.len() as u64,
                    sha256: sha256_hex(&bytes),
                    resolved,
                },
                Err(error) => FileStatus::Unreadable {
                    path: resolved.path,
                    error: friendly_io_error(&error),
                },
            },
        };
        files.push(FileCheck {
            file: *file,
            status,
        });
    }
    let warnings = index
        .ambiguous_names()
        .into_iter()
        .filter(|name| manifest.iter().any(|file| file.name == name))
        .map(|name| {
            format!("{name} exists more than once with different upper/lower case; using one copy")
        })
        .collect();
    let mut report = ValidationReport {
        dir: dir.to_path_buf(),
        dir_error: None,
        files,
        version: VersionIdentification::Incomplete,
        warnings,
    };
    if report.is_usable() {
        report.version = identify_version(&report.files, versions);
    }
    report
}

fn identify_version(files: &[FileCheck], versions: &[KnownVersion]) -> VersionIdentification {
    let mut closest: Option<(&KnownVersion, Vec<&'static str>)> = None;
    for version in versions {
        let mut differing = Vec::new();
        for known in version.files {
            let matches = files.iter().any(|check| {
                check.file.name == known.name
                    && matches!(&check.status, FileStatus::Present { size, sha256, .. }
                        if *size == known.size && sha256 == known.sha256)
            });
            if !matches {
                differing.push(known.name);
            }
        }
        if differing.is_empty() {
            return VersionIdentification::Known {
                id: version.id,
                name: version.name,
            };
        }
        if closest
            .as_ref()
            .is_none_or(|(_, best)| differing.len() < best.len())
        {
            closest = Some((version, differing));
        }
    }
    match closest {
        Some((version, differing)) => VersionIdentification::Unknown {
            closest: Some(version.id),
            differing,
        },
        None => VersionIdentification::Unknown {
            closest: None,
            differing: Vec::new(),
        },
    }
}

/// Render an io error without Rust's Debug noise.
pub fn friendly_io_error(error: &std::io::Error) -> String {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::NotFound => "it does not exist".to_string(),
        ErrorKind::PermissionDenied => "permission denied".to_string(),
        ErrorKind::NotADirectory => "it is not a folder".to_string(),
        _ => error.to_string(),
    }
}
