//! Platform-neutral game-data setup for the Ultima V engine.
//!
//! This crate knows *what* the engine needs from the user's copy of the game
//! and how to check and normalise it. It knows nothing about where a
//! particular platform keeps things, how settings are stored or how anything
//! is shown: it is std-only, UI-free and has no OS-specific code, so a
//! desktop, phone or console host can each wrap it in its own way.
//!
//! - [`manifest`]: every file the engine reads.
//! - [`validate`]: a structured report of missing / unreadable files and
//!   the release identified by size and SHA-256 ([`version`]).
//! - [`resolve`]: case-insensitive names and nested-folder location.
//! - [`install`]: copy validated files, with canonical names, into a
//!   managed data folder the caller chooses.
//! - [`discovery`]: the [`DiscoveryProvider`] trait hosts implement.
//!
//! The engine itself (`u5-runtime`) does not depend on this crate; it is
//! simply handed the managed folder's path.

pub mod discovery;
pub mod install;
pub mod manifest;
pub mod resolve;
pub mod sha256;
pub mod validate;
pub mod version;

pub use discovery::{Candidate, Discovered, DiscoveryProvider, discover, discover_with_limit};
pub use install::{InstallError, InstallOutcome, install, same_dir};
pub use manifest::{DataFile, FileRole, MANIFEST, manifest_entry, required_files, save_files};
pub use resolve::{DirectoryIndex, ResolvedFile, SearchLimits, canonical_name, locate_data_root};
pub use validate::{
    FileCheck, FileStatus, ValidationReport, VersionIdentification, friendly_io_error,
    validate_dir, validate_dir_against, validate_location,
};
pub use version::{GOG_1_0, KNOWN_VERSIONS, KnownFile, KnownVersion, known_version};

/// Validate the data at or below `start` and, when usable, install it into
/// `dest`. The common one-call path for hosts.
pub fn validate_and_install(
    start: &std::path::Path,
    dest: &std::path::Path,
    limits: SearchLimits,
) -> Result<(ValidationReport, InstallOutcome), InstallError> {
    let report = validate_location(start, limits);
    let outcome = install(&report, dest)?;
    Ok((report, outcome))
}
