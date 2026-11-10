//! The toggle matrix: the configurations the equivalence harness runs
//! (Phase B design 1.3.12, "The matrix"; build plan M0-19).
//!
//! [`toggle_matrix`] derives it from the catalogue, deterministically:
//!
//! - **alone**: one row per catalogue key and non-default value, so every
//!   key has a row. A toggle is turned on; a valued setting is set to each
//!   of its [`alternate_values`] in turn. A key with no other value (a path,
//!   a one-value range) is set to its default, so it still has its row. A
//!   cheat-tier key's row also opens the cheats gate, `cheats.enabled`.
//! - **pairwise**: a covering array over every boolean toggle except the
//!   cheats, together with every valued setting that has an effect class
//!   (each at its default and its alternates), so every pair of values of
//!   two such keys appears in some row.
//! - **all on except cheats**: every toggle on except the cheats section,
//!   deviation-tier toggles included. A driven toggle (the switch of a
//!   valued setting) is reached through that setting, never by name.
//!
//! Classic itself is not a row: the suite always runs it (and compares it
//! with the 1.0.1 route-smoke manifest).
//!
//! The matrix covers every row, shipped or not. A row whose feature has not
//! shipped resolves to its default with a note (U13), so its configuration
//! is Classic until the feature ships; the suite then starts testing it
//! with no edit here.
//!
//! [`matrix_tsv`] writes the rows as `qa/enhancements/matrix.tsv`, the file
//! `u5-engine --equivalence-suite OUT --matrix FILE` reads; a test keeps the
//! committed file equal to this generator.

use std::collections::BTreeSet;

use super::catalogue::{
    CatalogueRow, OPTIONS, OptionDefault, OptionKind, OptionSpec, TOGGLES, Tier, ToggleSpec,
};
use super::resolve::CHEATS_SWITCH;
use super::source::CliSetting;

/// The section every cheat lives in. The pairwise array and the all-on row
/// leave it out (design 1.3.12).
pub const CHEATS_SECTION: &str = "cheats.";

/// The name of the all-on row.
pub const ALL_ON_EXCEPT_CHEATS: &str = "all-on-except-cheats";

/// Where the matrix is committed, relative to the workspace root.
pub const MATRIX_FILE: &str = "qa/enhancements/matrix.tsv";

/// One configuration of the matrix: a name and the settings it resolves,
/// as the command line would give them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixRow {
    pub name: String,
    pub settings: Vec<CliSetting>,
}

/// One key the pairwise array varies, with its values (the default first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairwiseFactor {
    pub key: &'static str,
    pub values: Vec<String>,
}

/// Whether `key` is in the cheats section, or its row is cheat tier.
pub fn is_cheat(row: CatalogueRow) -> bool {
    let tier = match row {
        CatalogueRow::Toggle(spec) => spec.tier,
        CatalogueRow::Option(spec) => spec.tier,
    };
    tier == Tier::Cheat || row.key().starts_with(CHEATS_SECTION)
}

/// The default of a valued setting as `--set` would spell it.
pub fn default_text(spec: &OptionSpec) -> String {
    match spec.default {
        OptionDefault::Bool(value) => value.to_string(),
        OptionDefault::Int(value) => value.to_string(),
        OptionDefault::Str(text) => text.to_string(),
        OptionDefault::List(items) => items.join(","),
    }
}

/// The non-default values the matrix sets a valued setting to, in a fixed
/// order. None of them contains a comma or a tab, so a matrix line holds
/// them as they are; a list therefore takes one word at a time.
///
/// - bool: the other value;
/// - int: the minimum and the maximum, where they differ from the default;
/// - enum: every other word, deviation values included;
/// - colour: white, or black when the default is white;
/// - list: each single word, where it differs from the default;
/// - chord: an F-key no default uses, from F13 on, one per `[keys]` row;
/// - path: none (the suite has no file to point it at).
pub fn alternate_values(spec: &OptionSpec) -> Vec<String> {
    let default = default_text(spec);
    let mut values: Vec<String> = match spec.kind {
        OptionKind::Bool => vec![(default != "true").to_string()],
        OptionKind::Int { min, max } => vec![min.to_string(), max.to_string()],
        OptionKind::Enum(words) => words.iter().map(|w| w.to_string()).collect(),
        OptionKind::Color => vec![if default.eq_ignore_ascii_case("#ffffff") {
            "#000000".to_string()
        } else {
            "#ffffff".to_string()
        }],
        OptionKind::List(words) => words.iter().map(|w| w.to_string()).collect(),
        OptionKind::Chord => vec![spare_chord(spec.key)],
        OptionKind::Path => Vec::new(),
    };
    let mut seen = BTreeSet::new();
    values.retain(|value| *value != default && seen.insert(value.clone()));
    values
}

/// A chord for `key` that no `[keys]` default uses: F13 for the first
/// chord row, F14 for the second, and so on, then the same under Shift.
fn spare_chord(key: &str) -> String {
    let index = OPTIONS
        .iter()
        .filter(|spec| spec.kind == OptionKind::Chord)
        .position(|spec| spec.key == key)
        .unwrap_or(0);
    let shift = if index / 12 % 2 == 1 { "Shift+" } else { "" };
    format!("{shift}F{}", 13 + index % 12)
}

/// The keys the pairwise array varies: every boolean toggle except the
/// cheats, then every valued setting with an effect class that has another
/// value, except the cheats.
pub fn pairwise_factors() -> Vec<PairwiseFactor> {
    let toggles = TOGGLES
        .iter()
        .filter(|spec| !spec.driven && !is_cheat(CatalogueRow::Toggle(spec)))
        .map(|spec: &ToggleSpec| PairwiseFactor {
            key: spec.key,
            values: vec!["false".to_string(), "true".to_string()],
        });
    let options = OPTIONS
        .iter()
        .filter(|spec| spec.class.is_some() && !is_cheat(CatalogueRow::Option(spec)))
        .filter_map(|spec| {
            let alternates = alternate_values(spec);
            (!alternates.is_empty()).then(|| PairwiseFactor {
                key: spec.key,
                values: std::iter::once(default_text(spec))
                    .chain(alternates)
                    .collect(),
            })
        });
    toggles.chain(options).collect()
}

/// A pairwise covering array over factors with `sizes[i]` values each: rows
/// of value indices in which every pair of values of every two factors
/// appears at least once. Deterministic greedy construction: each row
/// starts from the first pair not yet covered, then fills every other factor
/// with the value that covers the most new pairs (the lowest index on a
/// tie, so defaults win ties).
pub fn covering_array(sizes: &[usize]) -> Vec<Vec<usize>> {
    let n = sizes.len();
    let mut uncovered: BTreeSet<(usize, usize, usize, usize)> = BTreeSet::new();
    for i in 0..n {
        for j in i + 1..n {
            for a in 0..sizes[i] {
                for b in 0..sizes[j] {
                    uncovered.insert((i, a, j, b));
                }
            }
        }
    }
    let mut rows = Vec::new();
    while let Some(&(i, a, j, b)) = uncovered.iter().next() {
        let mut row: Vec<Option<usize>> = vec![None; n];
        row[i] = Some(a);
        row[j] = Some(b);
        for k in 0..n {
            if row[k].is_some() {
                continue;
            }
            let gain = |v: usize| {
                row.iter()
                    .enumerate()
                    .filter_map(|(m, value)| value.map(|value| (m, value)))
                    .filter(|&(m, value)| {
                        let pair = if m < k {
                            (m, value, k, v)
                        } else {
                            (k, v, m, value)
                        };
                        uncovered.contains(&pair)
                    })
                    .count()
            };
            // Ties go to the value with the most pairs still open, so the
            // later factors have something to cover.
            let open = |v: usize| {
                uncovered
                    .iter()
                    .filter(|&&(p, a, q, b)| (p == k && a == v) || (q == k && b == v))
                    .count()
            };
            let score = |v: usize| (gain(v), open(v));
            let mut best = 0;
            for v in 1..sizes[k] {
                if score(v) > score(best) {
                    best = v;
                }
            }
            row[k] = Some(best);
        }
        let row: Vec<usize> = row.into_iter().map(|v| v.unwrap_or(0)).collect();
        for p in 0..n {
            for q in p + 1..n {
                uncovered.remove(&(p, row[p], q, row[q]));
            }
        }
        rows.push(row);
    }
    rows
}

fn set(key: &str, value: impl Into<String>) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.into(),
    }
}

/// The settings a cheat-tier row needs before its own: the section switch
/// (design 1.4.5, "each cheat | cheats.enabled | inert"). A cheat alone is
/// the cheat with its gate open.
fn gate(tier: Tier, key: &str) -> Vec<CliSetting> {
    if tier == Tier::Cheat && key != CHEATS_SWITCH {
        vec![set(CHEATS_SWITCH, "true")]
    } else {
        Vec::new()
    }
}

/// Every configuration of the matrix, in a fixed order: the alone rows in
/// catalogue order, the pairwise rows, then all on except cheats.
pub fn toggle_matrix() -> Vec<MatrixRow> {
    let mut rows = Vec::new();
    for row in CatalogueRow::all() {
        match row {
            CatalogueRow::Toggle(spec) => rows.push(MatrixRow {
                name: format!("alone:{}", spec.key),
                settings: [gate(spec.tier, spec.key), vec![set(spec.key, "true")]].concat(),
            }),
            CatalogueRow::Option(spec) => {
                let alternates = alternate_values(spec);
                let values = if alternates.is_empty() {
                    vec![default_text(spec)]
                } else {
                    alternates
                };
                for value in values {
                    rows.push(MatrixRow {
                        name: format!("alone:{}={value}", spec.key),
                        settings: [gate(spec.tier, spec.key), vec![set(spec.key, value)]].concat(),
                    });
                }
            }
        }
    }

    let factors = pairwise_factors();
    let sizes: Vec<usize> = factors.iter().map(|f| f.values.len()).collect();
    for (number, values) in covering_array(&sizes).into_iter().enumerate() {
        // A key at its default needs no setting.
        let settings = factors
            .iter()
            .zip(values)
            .filter(|(_, value)| *value != 0)
            .map(|(factor, value)| set(factor.key, factor.values[value].clone()))
            .collect();
        rows.push(MatrixRow {
            name: format!("pairwise:{:02}", number + 1),
            settings,
        });
    }

    rows.push(MatrixRow {
        name: ALL_ON_EXCEPT_CHEATS.to_string(),
        settings: TOGGLES
            .iter()
            .filter(|spec| !spec.driven && !is_cheat(CatalogueRow::Toggle(spec)))
            .map(|spec| set(spec.key, "true"))
            .collect(),
    });
    rows
}

/// The value `setting` gives `key`, read the way the matrix writes it.
fn setting_text(setting: &CliSetting) -> String {
    match setting {
        CliSetting::Preset(preset) => format!("preset={}", preset.key()),
        CliSetting::Enable(key) => format!("{key}=true"),
        CliSetting::Disable(key) => format!("{key}=false"),
        CliSetting::Set { key, value } => format!("{key}={value}"),
    }
}

/// The matrix as the committed `qa/enhancements/matrix.tsv`: a comment
/// header, then one `NAME<TAB>SETTINGS` line per row, the settings joined by
/// commas.
pub fn matrix_tsv(rows: &[MatrixRow]) -> String {
    let mut text = String::from(
        "# Ultima V enhancement toggle matrix (Phase B design 1.3.12).\n\
         # Generated from the catalogue; a test keeps this file equal to it\n\
         # (cargo test -p u5-tui --test toggle_matrix; U5_BLESS_TOGGLE_MATRIX=1\n\
         # rewrites it). Classic is not listed: the suite always runs it.\n\
         # Rows whose feature has not shipped resolve to Classic until it does.\n\
         # name\tsettings\n",
    );
    for row in rows {
        let settings: Vec<String> = row.settings.iter().map(setting_text).collect();
        text.push_str(&row.name);
        text.push('\t');
        text.push_str(&settings.join(","));
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs_covered(sizes: &[usize], rows: &[Vec<usize>]) -> bool {
        (0..sizes.len()).all(|i| {
            (i + 1..sizes.len()).all(|j| {
                (0..sizes[i]).all(|a| {
                    (0..sizes[j]).all(|b| rows.iter().any(|row| row[i] == a && row[j] == b))
                })
            })
        })
    }

    #[test]
    fn the_covering_array_covers_every_pair() {
        for sizes in [
            vec![],
            vec![2],
            vec![2, 2],
            vec![2, 3, 2],
            vec![2; 10],
            vec![3, 2, 2, 4, 2, 2, 3],
            vec![2; 40],
        ] {
            let rows = covering_array(&sizes);
            assert!(pairs_covered(&sizes, &rows), "{sizes:?}");
            for row in &rows {
                assert_eq!(row.len(), sizes.len());
                assert!(row.iter().zip(&sizes).all(|(v, size)| v < size));
            }
        }
    }

    #[test]
    fn the_covering_array_stays_small() {
        // Exhaustive would be 2^10 and 2^40 rows.
        let ten = covering_array(&[2; 10]).len();
        let forty = covering_array(&[2; 40]).len();
        assert!(ten <= 10, "{ten}");
        assert!(forty <= 16, "{forty}");
        assert!(covering_array(&[2]).is_empty(), "one factor has no pair");
        assert_eq!(covering_array(&[2, 2]).len(), 4);
    }

    #[test]
    fn the_matrix_is_deterministic() {
        assert_eq!(toggle_matrix(), toggle_matrix());
        assert_eq!(matrix_tsv(&toggle_matrix()), matrix_tsv(&toggle_matrix()));
    }

    #[test]
    fn every_catalogue_key_has_a_row() {
        let rows = toggle_matrix();
        for row in CatalogueRow::all() {
            assert!(
                rows.iter().any(|r| r
                    .settings
                    .iter()
                    .any(|s| matches!(s, CliSetting::Set { key, .. } if key == row.key()))),
                "{} has no matrix row",
                row.key()
            );
        }
    }

    #[test]
    fn row_names_are_unique_and_values_fit_a_matrix_line() {
        let rows = toggle_matrix();
        let names: BTreeSet<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names.len(), rows.len());
        for row in &rows {
            assert!(!row.name.contains(char::is_whitespace), "{}", row.name);
            assert_ne!(row.name, "classic");
            for setting in &row.settings {
                let text = setting_text(setting);
                assert!(!text.contains([',', '\t', '\n']), "{text}");
            }
        }
    }

    #[test]
    fn alternates_differ_from_the_default() {
        for spec in OPTIONS {
            let default = default_text(spec);
            for value in alternate_values(spec) {
                assert_ne!(value, default, "{}", spec.key);
            }
        }
        let chords: BTreeSet<String> = OPTIONS
            .iter()
            .filter(|spec| spec.kind == OptionKind::Chord)
            .flat_map(alternate_values)
            .collect();
        let defaults: BTreeSet<String> = OPTIONS
            .iter()
            .filter(|spec| spec.kind == OptionKind::Chord)
            .map(default_text)
            .collect();
        assert!(chords.is_disjoint(&defaults));
        assert_eq!(
            chords.len(),
            OPTIONS
                .iter()
                .filter(|spec| spec.kind == OptionKind::Chord)
                .count(),
            "one spare chord per [keys] row"
        );
    }

    #[test]
    fn the_all_on_row_turns_on_every_toggle_but_the_cheats() {
        let rows = toggle_matrix();
        let all_on = rows
            .iter()
            .find(|row| row.name == ALL_ON_EXCEPT_CHEATS)
            .unwrap();
        for spec in TOGGLES.iter().filter(|spec| !spec.driven) {
            let listed = all_on.settings.contains(&set(spec.key, "true"));
            assert_eq!(
                listed,
                !is_cheat(CatalogueRow::Toggle(spec)),
                "{}",
                spec.key
            );
        }
    }

    #[test]
    fn no_pairwise_factor_is_a_cheat() {
        for factor in pairwise_factors() {
            assert!(!factor.key.starts_with(CHEATS_SECTION), "{}", factor.key);
            assert!(factor.values.len() >= 2, "{}", factor.key);
        }
    }
}
