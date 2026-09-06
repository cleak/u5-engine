//! Sanitized census of one `DUNGEON.CBT` arena record's setup sources.
//!
//! `cleak/u5-engine#9`: the engine wins Doom's final room in one Pass
//! because its arena has no combatants. This reports how that record's
//! sixteen source markers classify - counts by kind, no grid dump.
//!
//! Usage: `cargo run -p u5-tui --example dungeon_cbt_census -- <DIR> [INDEX...]`

use std::collections::BTreeMap;
use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: <DIR> [INDEX...]");
    let bank = load_dungeon_cbt(Path::new(&dir)).expect("DUNGEON.CBT must parse");
    let indices: Vec<usize> = args.filter_map(|a| a.parse().ok()).collect();
    let indices = if indices.is_empty() {
        (0..DUNGEON_CBT_ARENA_COUNT).collect()
    } else {
        indices
    };
    for index in indices {
        let Some(record) = bank.record(index) else {
            println!("arena {index}: absent");
            continue;
        };
        let setup = dungeon_room_combat_setup_from_record_for_entry(index, record, 0, true);
        let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
        for source in &setup.setup_sources {
            let name = match source.kind {
                DungeonRoomSetupSourceKind::OrdinaryCombatant { .. } => "ordinary",
                DungeonRoomSetupSourceKind::AbsorbableField => "absorbable-field",
                DungeonRoomSetupSourceKind::SpecialPlacement(_) => "special",
            };
            *kinds.entry(name).or_default() += 1;
        }
        let summary: Vec<String> = kinds
            .iter()
            .map(|(name, count)| format!("{name}={count}"))
            .collect();
        println!(
            "arena {index:>3}: {} source(s) [{}]",
            setup.setup_sources.len(),
            summary.join(" ")
        );
    }
}
