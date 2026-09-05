//! Report what `conversation.md §2`'s Talk gates see around the party.
//!
//! `cleak/u5-spec#198` asks which gate makes the stock game answer
//! `No response!` for an NPC that Look identifies normally. Step 4 reads
//! "the live map tile occupying the resolved cell", and that byte is the
//! one thing a screen comparison cannot see - it is under the NPC's own
//! sprite. This prints it, for the party's cell and its four neighbours,
//! from whatever save the given profile holds.
//!
//! Usage: `cargo run -p u5-tui --example talk_gate_probe -- <PROFILE_DIR>`
//!
//! Sanitized by construction: tile ids and dialog ids only, no map dump
//! and no dialogue text.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: talk_gate_probe <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");

    println!(
        "scene {:?} floor/level {:?} party at ({}, {}) facing {}",
        state.area,
        state.area,
        state.player.x,
        state.player.y,
        Direction::name(state.player.facing),
    );
    for direction in [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ] {
        let (dx, dy) = direction.delta();
        let x = state.player.x as isize + dx;
        let y = state.player.y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            continue;
        }
        let (x, y) = (x as usize, y as usize);
        let tile = state.talk_status_tile_at(x, y);
        let npc = state.npc_at_current_floor(x, y);
        println!(
            "  {:<5} ({x:>2},{y:>2}) floor tile 0x{tile:02x}{}  npc {}",
            Direction::name(direction),
            match tile {
                TALK_MIRROR_TILE => " [MIRROR -> No response!]",
                0xab => " [BED -> Zzzzzz...]",
                t if is_talk_through_tile(t) => " [talk-through]",
                _ => "",
            },
            match npc {
                Some(npc) => format!(
                    "slot {} dialog-id {} (0x{:02x}) type 0x{:02x}",
                    npc.slot, npc.dialog_id, npc.dialog_id, npc.type_byte
                ),
                None => "none".to_string(),
            },
        );
    }

    // Is the grid the gate reads composited with NPC sprites? If it is,
    // step 4 can never see the floor under a talker, which would make
    // the mirror arm unreachable for exactly the case it exists for.
    let mut checked = 0usize;
    let mut equal_to_npc_type = 0usize;
    for y in 0..32usize {
        for x in 0..32usize {
            let Some(npc) = state.npc_at_current_floor(x, y) else {
                continue;
            };
            checked += 1;
            if state.talk_status_tile_at(x, y) == npc.type_byte {
                equal_to_npc_type += 1;
            }
        }
    }
    println!(
        "npcs on this floor: {checked}; cells whose gate tile equals the npc's own type byte: {equal_to_npc_type}"
    );

    // The roster, so a paired scenario can be aimed at a talker instead
    // of at empty floor - which is what two earlier Talk scenarios did.
    for y in 0..32usize {
        for x in 0..32usize {
            let Some(npc) = state.npc_at_current_floor(x, y) else {
                continue;
            };
            println!(
                "  npc slot {:>2} at ({x:>2},{y:>2}) dialog-id {:>3} floor tile 0x{:02x}",
                npc.slot,
                npc.dialog_id,
                state.talk_status_tile_at(x, y),
            );
        }
    }
}
