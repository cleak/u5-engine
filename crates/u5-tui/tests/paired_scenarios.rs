//! Shape checks for the paired play-test scenarios in `qa/paired`.
//!
//! A scenario whose seed save is missing does not fail when it runs: both
//! sides sit on the title menu answering `No active game`, every `shot`
//! captures that menu, and the harness reports `pass` because the two
//! sides agree. The only defence is to say, in the scenario itself, that
//! it needs one - so this pins that every scenario either creates its own
//! character or declares the seed it needs.

use std::fs;
use std::path::Path;

/// The keystroke that opens character creation at the title menu.
const CREATE_CHARACTER_STEP: &str = "\tkey\tc";
const SEED_HEADER: &str = "# requires-seed:";

#[test]
fn every_paired_scenario_creates_a_character_or_declares_its_seed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("qa/paired");
    let mut checked = 0;
    for entry in fs::read_dir(&dir).expect("qa/paired must exist") {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("tsv") {
            continue;
        }
        // `seeds.tsv` is the suite's seed table, not a scenario. It shares the
        // directory and the extension because `qa/tools/paired_suite.py` reads
        // it from beside the scenarios it drives.
        if path.file_name().and_then(|name| name.to_str()) == Some("seeds.tsv") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("scenario must be readable");
        let creates_character = text
            .lines()
            .any(|line| line.ends_with(CREATE_CHARACTER_STEP));
        let declares_seed = text
            .lines()
            .any(|line| line.trim_start().starts_with(SEED_HEADER));
        assert!(
            creates_character || declares_seed,
            "{} neither creates a character nor carries a `{SEED_HEADER}` line; \
             without a seed both sides would sit on the title menu and the run \
             would still report pass",
            path.display()
        );
        if declares_seed {
            let described = text
                .lines()
                .find(|line| line.trim_start().starts_with(SEED_HEADER))
                .map(|line| line.trim_start()[SEED_HEADER.len()..].trim())
                .unwrap_or_default();
            assert!(
                described.len() > 10,
                "{} declares a seed without saying what it must hold",
                path.display()
            );
        }
        checked += 1;
    }
    assert!(checked >= 20, "expected the paired suite, found {checked}");
}
