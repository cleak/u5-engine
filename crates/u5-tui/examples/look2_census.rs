//! Sanitized census of `LOOK2.DAT`'s description table.
//!
//! `cleak/u5-spec#198`: eleven Stonegate NPCs come back described `x`,
//! which is either shipped data or a filler convention the engine's
//! `LookTable::is_sentinel` - which compares only against entry 0 - is
//! failing to recognise. This counts how the table is populated without
//! reproducing it: entry 0's own text, how many entries repeat it, and
//! the ten most repeated descriptions with their counts.
//!
//! Usage: `cargo run -p u5-tui --example look2_census -- <PROFILE_DIR>`

use std::collections::BTreeMap;
use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: look2_census <DIR>");
    let table = load_look_table(Path::new(&dir)).expect("LOOK2.DAT must parse");

    let entry_zero = table.description(0).unwrap_or("<none>");
    println!("entries: {}", table.descriptions.len());
    println!("entry 0: {entry_zero:?}");

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for description in &table.descriptions {
        *counts.entry(description.as_str()).or_default() += 1;
    }
    println!(
        "entries equal to entry 0: {}",
        counts.get(entry_zero).copied().unwrap_or(0)
    );
    println!("empty entries: {}", counts.get("").copied().unwrap_or(0));

    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("most repeated descriptions:");
    for (description, count) in ranked.iter().take(10) {
        println!("  {count:>4}  {description:?}");
    }
}
