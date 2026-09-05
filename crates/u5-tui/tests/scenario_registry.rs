//! Validates `qa/scenarios.tsv`, the machine-readable QA scenario registry.
//!
//! The registry is a clean-room system of record: it carries stable ids,
//! reproduction commands, sanitized expected hashes, and the public spec
//! revision each expectation was reviewed against. It must never contain
//! asset content, so this test only checks shape and internal consistency.

use std::collections::{BTreeSet, HashSet};

use u5_tui::parse_cli_args;

const REGISTRY: &str = include_str!("../../../qa/scenarios.tsv");

const COLUMNS: [&str; 13] = [
    "id",
    "subsystem",
    "frontend",
    "lanes",
    "platforms",
    "verification",
    "spec_path",
    "spec_commit",
    "setup",
    "expected_kind",
    "expected_value",
    "baseline_engine_commit",
    "notes",
];
const FRONTENDS: [&str; 5] = ["runtime", "tui", "bevy", "dosbox", "host"];
const LANES: [&str; 4] = ["source", "asset", "bevy", "dosbox"];
const PLATFORMS: [&str; 3] = ["windows", "linux", "steamos"];

struct Scenario<'a> {
    fields: Vec<&'a str>,
}

impl<'a> Scenario<'a> {
    fn get(&self, column: &str) -> &'a str {
        let index = COLUMNS
            .iter()
            .position(|name| *name == column)
            .expect("known column");
        self.fields[index]
    }
}

fn scenarios() -> Vec<Scenario<'static>> {
    let mut lines = REGISTRY
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let header: Vec<&str> = lines.next().expect("header row").split('\t').collect();
    assert_eq!(
        header, COLUMNS,
        "registry header must match the documented columns"
    );
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), COLUMNS.len(), "column count in row: {line}");
            Scenario { fields }
        })
        .collect()
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn assert_list_subset(value: &str, allowed: &[&str], label: &str, id: &str) {
    let mut seen = BTreeSet::new();
    for item in value.split('|') {
        assert!(allowed.contains(&item), "{id}: unknown {label} `{item}`");
        assert!(seen.insert(item), "{id}: duplicate {label} `{item}`");
    }
}

#[test]
fn scenario_registry_rows_are_well_formed() {
    let rows = scenarios();
    assert!(!rows.is_empty(), "registry must list at least one scenario");
    let mut ids = HashSet::new();
    for row in &rows {
        let id = row.get("id");
        assert!(
            !id.is_empty()
                && id.bytes().all(|byte| byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || byte == b'.'
                    || byte == b'-'),
            "scenario id must be lowercase dotted-kebab: {id}"
        );
        assert!(ids.insert(id), "duplicate scenario id {id}");
        assert!(
            !row.get("subsystem").is_empty(),
            "{id}: subsystem is required"
        );
        assert!(
            FRONTENDS.contains(&row.get("frontend")),
            "{id}: unknown frontend"
        );
        assert_list_subset(row.get("lanes"), &LANES, "lane", id);
        assert_list_subset(row.get("platforms"), &PLATFORMS, "platform", id);
        assert!(
            !row.get("spec_path").is_empty(),
            "{id}: spec_path is required"
        );
        assert!(
            is_hex(row.get("spec_commit"), 40),
            "{id}: spec_commit must be a full lowercase SHA-1"
        );
        assert!(
            is_hex(row.get("baseline_engine_commit"), 40),
            "{id}: baseline_engine_commit must be a full lowercase SHA-1"
        );
        assert!(!row.get("setup").is_empty(), "{id}: setup is required");
        assert!(
            !row.get("notes").contains("hash "),
            "{id}: notes must not embed frame hashes"
        );
        match (row.get("verification"), row.get("expected_kind")) {
            ("automated", "manifest-sha256") => {
                assert!(
                    is_hex(row.get("expected_value"), 64),
                    "{id}: expected manifest hash"
                );
                assert!(
                    row.get("setup").starts_with("u5-engine "),
                    "{id}: manifest suites run the engine binary"
                );
                assert!(
                    row.get("setup").contains("{out}") && row.get("setup").contains("{profile}")
                );
            }
            ("automated", "exit-zero") => {
                assert_eq!(row.get("expected_value"), "0", "{id}: exit code")
            }
            ("human", "human-checklist") => {
                assert!(
                    row.get("setup").starts_with("procedure:"),
                    "{id}: human rows use procedure ids"
                );
                assert!(!row.get("expected_value").is_empty(), "{id}: checklist id");
            }
            (verification, kind) => {
                panic!("{id}: unsupported verification/expectation pair {verification}/{kind}")
            }
        }
    }
}

#[test]
fn automated_engine_scenarios_parse_with_the_current_cli() {
    for row in scenarios() {
        let setup = row.get("setup");
        if !setup.starts_with("u5-engine ") {
            continue;
        }
        let concrete = setup
            .replace("{out}", "out")
            .replace("{profile}", "profile");
        let args = concrete.split(' ').skip(1);
        parse_cli_args(args).unwrap_or_else(|error| panic!("{}: {error}", row.get("id")));
    }
}

#[test]
fn source_lane_scenarios_never_require_assets() {
    for row in scenarios() {
        if row.get("lanes").split('|').any(|lane| lane == "source") {
            assert!(
                !row.get("setup").contains("{profile}"),
                "{}: source lane must not take an asset profile",
                row.get("id")
            );
            assert_eq!(
                row.get("lanes"),
                "source",
                "{}: source lane rows are source-only",
                row.get("id")
            );
        }
    }
}
