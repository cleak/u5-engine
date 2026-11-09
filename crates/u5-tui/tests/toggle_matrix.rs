//! The committed toggle matrix and its coverage (Phase B design 1.3.12,
//! "The matrix"; build plan M0-19).
//!
//! - `qa/enhancements/matrix.tsv` is the generator's output, and parses
//!   back to the generator's rows.
//! - Every catalogue key has a row.
//! - Every pair of values of two pairwise keys appears in some row.
//! - The all-on row turns on every toggle but the cheats.
//! - Every row resolves with no note but "not available yet" (U13).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{
    ALL_ON_EXCEPT_CHEATS, CHEATS_SECTION, CatalogueRow, CliSetting, LockState, MATRIX_FILE,
    MatrixRow, ResolvedEnhancements, SettingNote, TOGGLES, matrix_tsv, pairwise_factors,
    toggle_matrix,
};
use u5_tui::enhancement_settings::resolve_launch;
use u5_tui::{parse_matrix, plan_configurations};

/// Set to `1` to rewrite `qa/enhancements/matrix.tsv` from the catalogue.
const BLESS: &str = "U5_BLESS_TOGGLE_MATRIX";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed_rows() -> Vec<MatrixRow> {
    let text = fs::read_to_string(workspace_root().join(MATRIX_FILE)).expect("the matrix file");
    parse_matrix(&text).expect("the matrix file parses")
}

/// The value each row gives each key, `true` for a bare switch.
fn row_values(row: &MatrixRow) -> BTreeMap<String, String> {
    row.settings
        .iter()
        .filter_map(|setting| match setting {
            CliSetting::Set { key, value } => Some((key.clone(), value.clone())),
            CliSetting::Enable(key) => Some((key.clone(), "true".into())),
            CliSetting::Disable(key) => Some((key.clone(), "false".into())),
            CliSetting::Preset(_) => None,
        })
        .collect()
}

fn resolve(row: &MatrixRow) -> ResolvedEnhancements {
    // As the suite resolves a row: no settings file, no lock.
    resolve_launch(None, &row.settings, LockState::Off).resolved
}

#[test]
fn the_committed_matrix_equals_the_generator() {
    let path = workspace_root().join(MATRIX_FILE);
    let expected = matrix_tsv(&toggle_matrix());
    let actual = fs::read_to_string(&path).unwrap_or_default();
    if actual != expected {
        if std::env::var(BLESS).as_deref() == Ok("1") {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, &expected).unwrap();
        } else {
            panic!(
                "{MATRIX_FILE} is out of date with the catalogue; rerun with {BLESS}=1 to \
                 regenerate it. Expected:\n{expected}"
            );
        }
    }
    assert_eq!(committed_rows(), toggle_matrix());
}

#[test]
fn every_catalogue_key_has_a_row() {
    let rows = committed_rows();
    for key in CatalogueRow::all().map(CatalogueRow::key) {
        assert!(
            rows.iter().any(|row| row_values(row).contains_key(key)),
            "{key} has no row in {MATRIX_FILE}"
        );
    }
}

#[test]
fn every_pair_of_pairwise_values_appears() {
    let rows = committed_rows();
    let factors = pairwise_factors();
    let values: Vec<BTreeMap<String, String>> = rows.iter().map(row_values).collect();
    // A key a row does not name is at its default, the factor's first value.
    let value_of = |row: &BTreeMap<String, String>, f: usize| {
        row.get(factors[f].key)
            .cloned()
            .unwrap_or_else(|| factors[f].values[0].clone())
    };
    for i in 0..factors.len() {
        for j in i + 1..factors.len() {
            for a in &factors[i].values {
                for b in &factors[j].values {
                    assert!(
                        values
                            .iter()
                            .any(|row| value_of(row, i) == *a && value_of(row, j) == *b),
                        "no row has {}={a} with {}={b}",
                        factors[i].key,
                        factors[j].key
                    );
                }
            }
        }
    }
}

#[test]
fn the_all_on_row_turns_on_every_toggle_but_the_cheats() {
    let rows = committed_rows();
    let all_on = rows
        .iter()
        .find(|row| row.name == ALL_ON_EXCEPT_CHEATS)
        .expect("the all-on row");
    let values = row_values(all_on);
    for spec in TOGGLES {
        let on = values.get(spec.key).map(String::as_str) == Some("true");
        assert_eq!(on, !spec.key.starts_with(CHEATS_SECTION), "{}", spec.key);
    }
}

/// A row may name a key that has not shipped (it then resolves to the
/// default until it does), but never a key or value the resolver refuses.
#[test]
fn every_row_resolves_cleanly() {
    for row in committed_rows() {
        let resolved = resolve(&row);
        assert_eq!(resolved.lock, LockState::Off);
        for note in &resolved.notes {
            assert!(
                matches!(note, SettingNote::NotAvailable { .. }),
                "{}: {note}",
                row.name
            );
        }
    }
}

/// Rows that resolve alike share their runs; the first of each kind runs.
#[test]
fn the_suite_runs_each_distinct_resolution_once() {
    let mut configs = vec![("classic".to_string(), ResolvedEnhancements::classic())];
    configs.extend(
        committed_rows()
            .iter()
            .map(|row| (row.name.clone(), resolve(row))),
    );
    let plan = plan_configurations(&configs);
    assert_eq!(plan[0], None, "Classic always runs");
    for (index, same) in plan.iter().enumerate() {
        if let Some(first) = *same {
            assert!(first < index);
            assert_eq!(plan[first], None);
            assert_eq!(configs[first].1.toggles, configs[index].1.toggles);
            assert_eq!(configs[first].1.values, configs[index].1.values);
        }
    }
    // A row whose toggle is on (here forced, as a shipped feature would be)
    // is never folded into Classic.
    let mut forced = ResolvedEnhancements::classic();
    forced.toggles.set(TOGGLES[0].toggle, true);
    let plan = plan_configurations(&[
        ("classic".into(), ResolvedEnhancements::classic()),
        ("forced".into(), forced.clone()),
        ("again".into(), forced),
    ]);
    assert_eq!(plan, [None, None, Some(1)]);
}
