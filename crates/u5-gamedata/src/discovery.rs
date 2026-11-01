//! Candidate-location discovery.
//!
//! Where game data might live is a platform question (install paths,
//! launchers, registries, sandboxes), so it is a trait: each host supplies a
//! [`DiscoveryProvider`], and [`discover`] turns the candidates into
//! validated, de-duplicated results.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::resolve::{SearchLimits, locate_data_root};
use crate::validate::{ValidationReport, validate_dir};

/// A place that might hold the game data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    /// Where the suggestion came from, for people ("GOG Galaxy", "Wine
    /// prefix", "current folder").
    pub source: String,
    /// How many folder levels below `path` to search for the data.
    pub search_depth: usize,
}

impl Candidate {
    pub fn new(path: impl Into<PathBuf>, source: impl Into<String>, search_depth: usize) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
            search_depth,
        }
    }
}

/// A source of candidate locations. Platforms plug in their own.
pub trait DiscoveryProvider {
    fn candidates(&self) -> Vec<Candidate>;
}

impl<F> DiscoveryProvider for F
where
    F: Fn() -> Vec<Candidate>,
{
    fn candidates(&self) -> Vec<Candidate> {
        self()
    }
}

/// A candidate whose search found something that looks like game data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discovered {
    pub candidate: Candidate,
    /// The folder actually holding the files (may be below the candidate).
    pub data_root: PathBuf,
    pub report: ValidationReport,
}

/// Search every candidate and validate what is found.
///
/// Candidates that do not exist or hold nothing plausible are dropped. The
/// same data folder reached from two candidates is reported once (first
/// candidate wins). Usable results come first, recognised releases before
/// unknown ones, otherwise in provider order.
pub fn discover(providers: &[&dyn DiscoveryProvider]) -> Vec<Discovered> {
    discover_with_limit(providers, 4000)
}

/// [`discover`] with an explicit cap on directories visited per candidate.
pub fn discover_with_limit(
    providers: &[&dyn DiscoveryProvider],
    max_dirs: usize,
) -> Vec<Discovered> {
    let mut seen_candidates = HashSet::new();
    let mut seen_roots = HashSet::new();
    let mut found = Vec::new();
    for provider in providers {
        for candidate in provider.candidates() {
            let key = candidate
                .path
                .canonicalize()
                .unwrap_or_else(|_| candidate.path.clone());
            if !key.is_dir() || !seen_candidates.insert(key) {
                continue;
            }
            let limits = SearchLimits {
                max_depth: candidate.search_depth,
                max_dirs,
            };
            let Some(root) = locate_data_root(&candidate.path, limits) else {
                continue;
            };
            let root_key = root.canonicalize().unwrap_or_else(|_| root.clone());
            if !seen_roots.insert(root_key) {
                continue;
            }
            let report = validate_dir(&root);
            found.push(Discovered {
                candidate,
                data_root: root,
                report,
            });
        }
    }
    // Stable sort keeps provider order within each rank.
    found.sort_by_key(|item| {
        (
            !item.report.is_usable(),
            item.report.version.known_id().is_none(),
        )
    });
    found
}
