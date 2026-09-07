//! Stock a profile's save with items so a paired scenario can measure the
//! result lines that only a party carrying them can reach.
//!
//! The shipped starting party carries three items, which is enough for the
//! `hut-use-items` scenarios and nothing else: the potion, scroll and
//! special-item result lines need a save that holds those. Writing one is a
//! clean-room-safe engine operation - the counts come from the command line,
//! not from any asset - and the harness then seeds DOSBox and the engine from
//! the same file.
//!
//! Usage: seed_inventory <PROFILE_DIR> [potions|scrolls|specials|spells|reagents|mana|keys|gems|torches|status<slot>]=<N>...

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: seed_inventory <PROFILE_DIR> ...");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    for arg in args {
        let (what, count) = arg
            .split_once('=')
            .expect("each argument is <what>=<count>");
        let count: u8 = match count.parse() {
            Ok(count) => count,
            // A one-character value is a status letter.
            Err(_) if count.len() == 1 => count.as_bytes()[0],
            Err(err) => panic!("count is a byte or a status letter: {err}"),
        };
        match what {
            "potions" => state.potion_stock = [count; POTION_COUNT],
            "scrolls" => state.scroll_stock = [count; SCROLL_COUNT],
            "specials" => state.special_items = [count; SPECIAL_ITEM_COUNT],
            "keys" => state.keys = count,
            "gems" => state.gems = count,
            "torches" => state.torches = count,
            "spells" => state.spell_charges = [count; SPELL_COUNT],
            "equipment" => state.equipment_stock = [count; EQUIPMENT_COUNT],
            "strength" => {
                for strength in &mut state.party_strengths {
                    *strength = count;
                }
            }
            "reagents" => state.reagents = [count; REAGENT_COUNT],
            "mana" => {
                for member in &mut state.party {
                    member.mana = count;
                }
            }
            // `status<slot>=<byte>` sets one member's status letter, so a
            // scenario can measure a path that needs a dead or poisoned
            // member without playing one into that state.
            // `special<index>=<count>` sets one special-item counter, so a
            // scenario can stand a single row in the U-Use picker and reach
            // it with no keypress counting at all.
            _ if what.starts_with("special") && what.len() > "special".len() => {
                let index: usize = what
                    .trim_start_matches("special")
                    .parse()
                    .expect("special<index>");
                state.special_items[index] = count;
            }
            _ if what.starts_with("status") => {
                let slot: usize = what
                    .trim_start_matches("status")
                    .parse()
                    .expect("status<slot>");
                state.party[slot].status = count;
                if count == b'D' {
                    state.party[slot].hp = 0;
                }
            }
            other => panic!("unknown inventory field `{other}`"),
        }
    }
    state.write_save_files(dir).expect("save must write");
    println!("stocked {}", dir.display());
}
