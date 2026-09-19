//! Print a profile's party roster and where it stands.
//!
//! Seeds are opaque: `qa/paired/seeds.tsv` names a profile and a scenario
//! trusts it to hold the right party, and nothing checks. Establishing
//! that every combat and dungeon seed in the suite is solo-party took
//! decoding roster panels out of capture PNGs, which is a long way round
//! for a question the save answers directly.
//!
//! Reports names, status letters and the party's position - no map dump
//! and no item data.
//!
//! Usage: `cargo run -p u5-tui --example party_probe -- <PROFILE_DIR>`

use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    println!(
        "{:?}: party at ({}, {}) facing {:?}",
        state.area, state.player.x, state.player.y, state.player.facing
    );
    println!("party members: {}", state.party.len());
    for (index, member) in state.party.iter().enumerate() {
        let name = state
            .party_names
            .get(index)
            .and_then(|raw| party_name_to_string(raw))
            .unwrap_or_else(|| "<unnamed>".to_string());
        println!(
            "  {index}: {name:<10} status {} hp {}/{}",
            member.status as char, member.hp, member.max_hp
        );
    }
}
