//! Sanitized visual-suite manifest comparison.
//!
//! The frame suites deliberately write clean metadata rather than pixels into
//! `manifest.txt`. This module turns those manifests into a small regression
//! gate that can be used in CI or local review without committing game assets.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ManifestFrame {
    dimensions: String,
    frame_kind: String,
    hash: String,
    nonblack: u64,
    metadata: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ParsedManifest {
    coverage: BTreeMap<String, String>,
    frames: BTreeMap<String, ManifestFrame>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManifestCompareReport {
    pub baseline_frames: usize,
    pub candidate_frames: usize,
    pub baseline_coverage: usize,
    pub candidate_coverage: usize,
    pub differences: Vec<String>,
}

impl ManifestCompareReport {
    pub fn is_clean(&self) -> bool {
        self.differences.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.is_clean() {
            format!(
                "Manifest comparison clean: {} frame(s), {} coverage row(s).",
                self.candidate_frames, self.candidate_coverage
            )
        } else {
            let mut summary = format!(
                "Manifest comparison failed: {} difference(s).\n",
                self.differences.len()
            );
            for difference in &self.differences {
                summary.push_str("- ");
                summary.push_str(difference);
                summary.push('\n');
            }
            summary
        }
    }
}

pub fn compare_manifest_files(
    baseline_path: &Path,
    candidate_path: &Path,
) -> io::Result<ManifestCompareReport> {
    let baseline = fs::read_to_string(baseline_path)?;
    let candidate = fs::read_to_string(candidate_path)?;
    compare_manifest_text(&baseline, &candidate)
}

pub fn compare_manifest_text(
    baseline_text: &str,
    candidate_text: &str,
) -> io::Result<ManifestCompareReport> {
    let baseline = parse_manifest(baseline_text)?;
    let candidate = parse_manifest(candidate_text)?;
    let mut report = ManifestCompareReport {
        baseline_frames: baseline.frames.len(),
        candidate_frames: candidate.frames.len(),
        baseline_coverage: baseline.coverage.len(),
        candidate_coverage: candidate.coverage.len(),
        differences: Vec::new(),
    };

    compare_coverage(&baseline, &candidate, &mut report);
    compare_frames(&baseline, &candidate, &mut report);
    Ok(report)
}

fn parse_manifest(text: &str) -> io::Result<ParsedManifest> {
    let mut parsed = ParsedManifest::default();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.first() == Some(&"coverage") {
            if fields.len() != 3 {
                return Err(parse_error(
                    line_index,
                    "coverage row must have three columns",
                ));
            }
            parsed
                .coverage
                .insert(fields[1].to_string(), fields[2].to_string());
            continue;
        }
        if fields.len() < 5 {
            return Err(parse_error(
                line_index,
                "frame row must have at least five tab-separated columns",
            ));
        }
        let Some(hash_index) = fields.iter().position(|field| field.starts_with("hash ")) else {
            return Err(parse_error(line_index, "frame row is missing hash field"));
        };
        let Some(nonblack_index) = fields
            .iter()
            .position(|field| field.starts_with("nonblack "))
        else {
            return Err(parse_error(
                line_index,
                "frame row is missing nonblack field",
            ));
        };
        let hash = fields[hash_index]
            .strip_prefix("hash ")
            .unwrap()
            .to_string();
        let nonblack = fields[nonblack_index]
            .strip_prefix("nonblack ")
            .unwrap()
            .parse::<u64>()
            .map_err(|_| parse_error(line_index, "nonblack field is not an integer"))?;
        let metadata = fields
            .iter()
            .enumerate()
            .filter(|(index, _)| *index > 2 && *index != hash_index && *index != nonblack_index)
            .map(|(_, field)| (*field).to_string())
            .collect();
        let label = fields[0].to_string();
        let previous = parsed.frames.insert(
            label.clone(),
            ManifestFrame {
                dimensions: fields[1].to_string(),
                frame_kind: fields[2].to_string(),
                hash,
                nonblack,
                metadata,
            },
        );
        if previous.is_some() {
            return Err(parse_error(
                line_index,
                &format!("duplicate frame label `{label}`"),
            ));
        }
    }
    Ok(parsed)
}

fn compare_coverage(
    baseline: &ParsedManifest,
    candidate: &ParsedManifest,
    report: &mut ManifestCompareReport,
) {
    for key in baseline.coverage.keys() {
        match candidate.coverage.get(key) {
            Some(value) if value == &baseline.coverage[key] => {}
            Some(value) => report.differences.push(format!(
                "coverage `{key}` changed from `{}` to `{value}`",
                baseline.coverage[key]
            )),
            None => report
                .differences
                .push(format!("coverage `{key}` is missing")),
        }
    }
    for key in candidate.coverage.keys() {
        if !baseline.coverage.contains_key(key) {
            report.differences.push(format!(
                "coverage `{key}` is new with `{}`",
                candidate.coverage[key]
            ));
        }
    }
}

fn compare_frames(
    baseline: &ParsedManifest,
    candidate: &ParsedManifest,
    report: &mut ManifestCompareReport,
) {
    let labels: BTreeSet<_> = baseline
        .frames
        .keys()
        .chain(candidate.frames.keys())
        .collect();
    for label in labels {
        match (baseline.frames.get(label), candidate.frames.get(label)) {
            (Some(expected), Some(actual)) => compare_frame(label, expected, actual, report),
            (Some(_), None) => report
                .differences
                .push(format!("frame `{label}` is missing")),
            (None, Some(actual)) => {
                report.differences.push(format!("frame `{label}` is new"));
                if actual.nonblack == 0 {
                    report
                        .differences
                        .push(format!("frame `{label}` is all black"));
                }
            }
            (None, None) => {}
        }
    }
}

fn compare_frame(
    label: &str,
    expected: &ManifestFrame,
    actual: &ManifestFrame,
    report: &mut ManifestCompareReport,
) {
    if expected.nonblack != 0 && actual.nonblack == 0 {
        report
            .differences
            .push(format!("frame `{label}` is all black"));
    }
    if expected.dimensions != actual.dimensions {
        report.differences.push(format!(
            "frame `{label}` dimensions changed from `{}` to `{}`",
            expected.dimensions, actual.dimensions
        ));
    }
    if expected.frame_kind != actual.frame_kind {
        report.differences.push(format!(
            "frame `{label}` kind changed from `{}` to `{}`",
            expected.frame_kind, actual.frame_kind
        ));
    }
    if expected.hash != actual.hash {
        report.differences.push(format!(
            "frame `{label}` hash changed from `{}` to `{}`",
            expected.hash, actual.hash
        ));
    }
    if expected.nonblack != actual.nonblack {
        report.differences.push(format!(
            "frame `{label}` nonblack count changed from `{}` to `{}`",
            expected.nonblack, actual.nonblack
        ));
    }
    if expected.metadata != actual.metadata {
        report.differences.push(format!(
            "frame `{label}` metadata changed from `{}` to `{}`",
            expected.metadata.join("\t"),
            actual.metadata.join("\t")
        ));
    }
}

fn parse_error(line_index: usize, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("manifest line {}: {message}", line_index + 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE: &str = "\
# Ultima V Bevy visual frame suite manifest
coverage\ttotal-frames\t2
world-play\t320x200\tvisual frame\thash 1111111111111111\tnonblack 42\treview=world
combat-play\t320x200\tvisual combat frame\thash 2222222222222222\tnonblack 99\treview=combat
";

    #[test]
    fn compare_manifest_text_accepts_identical_sanitized_manifest() {
        let report = compare_manifest_text(BASELINE, BASELINE).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.baseline_frames, 2);
        assert_eq!(report.candidate_frames, 2);
        assert!(report.summary().contains("2 frame"));
    }

    #[test]
    fn compare_manifest_text_reports_structural_and_hash_differences() {
        let candidate = "\
coverage\ttotal-frames\t2
coverage\tnew-coverage\t1/1
world-play\t320x200\tvisual frame\thash aaaaaaaaaaaaaaaa\tnonblack 0\treview=world
new-frame\t320x200\tvisual frame\thash 3333333333333333\tnonblack 7\treview=new
";
        let report = compare_manifest_text(BASELINE, candidate).unwrap();
        assert!(!report.is_clean());
        assert!(
            report
                .differences
                .iter()
                .any(|diff| diff.contains("coverage `new-coverage` is new"))
        );
        assert!(
            report
                .differences
                .iter()
                .any(|diff| diff.contains("frame `combat-play` is missing"))
        );
        assert!(
            report
                .differences
                .iter()
                .any(|diff| diff.contains("frame `world-play` is all black"))
        );
        assert!(
            report
                .differences
                .iter()
                .any(|diff| diff.contains("frame `world-play` hash changed"))
        );
        assert!(
            report
                .differences
                .iter()
                .any(|diff| diff.contains("frame `new-frame` is new"))
        );
    }

    #[test]
    fn compare_manifest_text_parses_tui_manifest_rows() {
        let baseline = "\
britannia\t320x200\tworld frame\tturn 0\tat (136, 146) facing North\thash 1111111111111111\tnonblack 25
";
        let report = compare_manifest_text(baseline, baseline).unwrap();
        assert!(report.is_clean());
        assert_eq!(report.baseline_coverage, 0);
        assert_eq!(report.baseline_frames, 1);
    }

    #[test]
    fn compare_manifest_text_accepts_baseline_black_route_frame() {
        let baseline = "\
coverage\ttotal-routes\t1
route-dungeon-dark\t176x176\tdungeon first-person viewport\thash 0000000000000000\tnonblack 0\tcommands 1\tState: DUNGEON
";
        let report = compare_manifest_text(baseline, baseline).unwrap();
        assert!(report.is_clean(), "{:?}", report.differences);
    }
}
