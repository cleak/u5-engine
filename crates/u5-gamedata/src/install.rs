//! Normalising install: copy validated files, under their canonical
//! upper-case names, into a managed data directory the caller chooses.
//!
//! The managed directory is what the engine is pointed at. It is always
//! case-correct and writable, whatever the source looked like (a read-only
//! install, lower-case names from an extractor, a nested bundle folder, a
//! phone's document picker). Saves are copied in only when the managed
//! directory has none, so re-running setup never clobbers progress.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::FileRole;
use crate::sha256::sha256_hex;
use crate::validate::{FileStatus, ValidationReport, friendly_io_error};

/// What [`install`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallOutcome {
    pub data_dir: PathBuf,
    /// Canonical names written (new or refreshed).
    pub copied: Vec<&'static str>,
    /// Canonical names already identical in the managed directory.
    pub unchanged: Vec<&'static str>,
    /// Saves the managed directory already had, left untouched.
    pub saves_kept: Vec<&'static str>,
}

#[derive(Debug)]
pub enum InstallError {
    /// The report says required files are missing or unreadable.
    NotUsable(Box<ValidationReport>),
    /// The destination is the source itself and its names are not canonical
    /// (this step never renames files in someone else's folder).
    DestinationIsSource(PathBuf),
    Io {
        action: &'static str,
        path: PathBuf,
        error: io::Error,
    },
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUsable(report) => write!(
                f,
                "the game files in {} are incomplete:\n{}",
                report.dir.display(),
                report.summary()
            ),
            Self::DestinationIsSource(path) => write!(
                f,
                "cannot normalise {} in place; choose a different data folder",
                path.display()
            ),
            Self::Io {
                action,
                path,
                error,
            } => write!(
                f,
                "could not {action} {}: {}",
                path.display(),
                friendly_io_error(error)
            ),
        }
    }
}

impl std::error::Error for InstallError {}

fn io_err<'a>(action: &'static str, path: &'a Path) -> impl FnOnce(io::Error) -> InstallError + 'a {
    move |error| InstallError::Io {
        action,
        path: path.to_path_buf(),
        error,
    }
}

/// Copy the files of a usable `report` into `dest` with canonical names.
///
/// Static files are written when missing or different (so a corrected or
/// updated source refreshes the copy); saves only when `dest` has none.
/// Writes go through a temporary file and a rename, and every written file
/// is left writable. When `dest` is the validated folder itself and its
/// names are already canonical, nothing is copied.
pub fn install(report: &ValidationReport, dest: &Path) -> Result<InstallOutcome, InstallError> {
    if !report.is_usable() {
        return Err(InstallError::NotUsable(Box::new(report.clone())));
    }
    if same_dir(&report.dir, dest) {
        if !report.case_fixes().is_empty() {
            return Err(InstallError::DestinationIsSource(dest.to_path_buf()));
        }
        return Ok(InstallOutcome {
            data_dir: dest.to_path_buf(),
            unchanged: report
                .files
                .iter()
                .filter(|check| check.is_present())
                .map(|check| check.file.name)
                .collect(),
            ..InstallOutcome::default()
        });
    }
    fs::create_dir_all(dest).map_err(io_err("create", dest))?;
    let mut outcome = InstallOutcome {
        data_dir: dest.to_path_buf(),
        ..InstallOutcome::default()
    };
    for check in &report.files {
        let FileStatus::Present {
            resolved,
            size,
            sha256,
        } = &check.status
        else {
            continue;
        };
        let name = check.file.name;
        let target = dest.join(name);
        // A differently cased twin in the destination (say `saved.gam`
        // written by some other tool) would shadow nothing on Linux but
        // confuse people; the canonical name is the only one we manage.
        let existing = existing_file(dest, name).map_err(io_err("read", dest))?;
        match check.file.role {
            FileRole::Save => {
                if existing.is_some() {
                    outcome.saves_kept.push(name);
                    continue;
                }
            }
            FileRole::Required => {
                if let Some(existing) = &existing
                    && existing == &target
                    && same_contents(existing, *size, sha256)
                {
                    outcome.unchanged.push(name);
                    continue;
                }
            }
        }
        if let Some(existing) = existing
            && existing != target
        {
            // Replace the miscased twin with the canonical name.
            make_writable(&existing);
            fs::remove_file(&existing).map_err(io_err("remove", &existing))?;
        }
        copy_atomic(&resolved.path, &target)?;
        outcome.copied.push(name);
    }
    Ok(outcome)
}

/// The file in `dir` whose name folds to `name`, preferring an exact match.
fn existing_file(dir: &Path, name: &str) -> io::Result<Option<PathBuf>> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Ok(Some(exact));
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|actual| actual.eq_ignore_ascii_case(name))
            && entry.path().is_file()
        {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

fn same_contents(path: &Path, size: u64, sha256: &str) -> bool {
    match fs::metadata(path) {
        Ok(meta) if meta.len() == size => {
            fs::read(path).is_ok_and(|bytes| sha256_hex(&bytes) == sha256)
        }
        _ => false,
    }
}

fn copy_atomic(source: &Path, target: &Path) -> Result<(), InstallError> {
    let bytes = fs::read(source).map_err(io_err("read", source))?;
    let file_name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = target.with_file_name(format!(".{file_name}.u5-install-tmp"));
    if temp.exists() {
        make_writable(&temp);
        let _ = fs::remove_file(&temp);
    }
    fs::write(&temp, &bytes).map_err(io_err("write", &temp))?;
    make_writable(&temp);
    if target.exists() {
        // Windows refuses to rename over a read-only file.
        make_writable(target);
    }
    fs::rename(&temp, target).map_err(io_err("write", target))?;
    Ok(())
}

fn make_writable(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        let mut permissions = meta.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(path, permissions);
        }
    }
}

/// Whether two paths name the same existing directory.
pub fn same_dir(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
