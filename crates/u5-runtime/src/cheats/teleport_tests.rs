//! Chapter 7 Phase 4 (design 7.3.6, D7.8, D7.9; build plan M8-04): the
//! carry-options refactor's golden test and the teleport runtime.

use std::fs;
use std::path::Path;

use super::*;
use crate::test_fixtures::{debug_game_dir, open_world_grid, world_state};

/// FNV-1a, so a golden digest never depends on the std hasher.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn britannia(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state
        .rebuild_world_live_chunks_from_grid(WorldPlane::Britannia)
        .unwrap();
    state
}

/// Every `.OOL` file in the game directory, by name.
fn ool_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.to_ascii_uppercase().ends_with(".OOL"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|name| {
            let bytes = fs::read(dir.join(&name)).unwrap();
            (name, bytes)
        })
        .collect()
}

/// The fixture's eight moonstone slots: both planes, a far and a near
/// overworld warp, two castle floors, two dungeons and one empty slot.
fn golden_slots() -> [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT] {
    let slot = |scene, x, y, z| MoonstoneGateSlot { scene, x, y, z };
    let mut slots = [slot(MOONSTONE_INVALID_SCENE, 0, 0, 0); MOONSTONE_SLOT_COUNT];
    let wanted = [
        slot(0, 100, 100, 0),
        slot(0, 50, 60, 1),
        slot(17, 6, 7, 0),
        slot(18, 15, 15, 0),
        slot(33, 1, 1, 0),
        slot(34, 3, 5, 2),
        slot(0, 7, 6, 0),
    ];
    for (index, wanted) in wanted.into_iter().enumerate().take(MOONSTONE_SLOT_COUNT) {
        slots[index] = wanted;
    }
    slots
}

/// The digest of one gate travel from a fixed overworld state into one
/// slot: the watch snapshot, the transcript, the sound serials and the
/// `.OOL` bytes, with the destination's own outcome.
fn gate_travel_digest(phase: usize) -> u64 {
    let dir = debug_game_dir();
    let mut state = britannia(5, 5);
    state.moonstone_slots = golden_slots();
    state.player.transport = TransportState::Horse {
        type_byte: 160,
        tile: 160,
    };
    state.emit_message_line("one".to_string());
    state.emit_sound_effect(SoundEffect::BlockedStep);
    state.emit_sound_effect(SoundEffect::ActionSnap);
    let destination = gate_travel_destination_in(&dir, state.moonstone_slots[phase]).unwrap();
    let mut text = format!("{destination:?}\n");
    if let GateTravelDestination::Ready {
        target,
        floor,
        start,
    } = destination
    {
        let outcome = state.apply_gate_travel(&dir, phase, target, floor, start);
        text.push_str(&format!("{:?}\n", outcome.map_err(|err| err.to_string())));
    }
    // The PRNG is seeded from the host clock at every scene load, so it is
    // the one watched value a golden digest cannot hold.
    let watched: Vec<_> = watch_snapshot(&state)
        .into_iter()
        .filter(|(field, _)| *field != WatchField::Prng)
        .collect();
    text.push_str(&format!("{watched:?}\n"));
    text.push_str(&format!(
        "{:?} {:?} {}\n",
        state.message_transcript, state.message, state.message_row_open_mid_line
    ));
    text.push_str(&format!(
        "{} {:?}\n",
        state.sound_effect_serial,
        state.sound_effects_with_serials_after(0)
    ));
    text.push_str(&format!(
        "{:?} {} {:?}\n",
        state.player, state.turn, state.clock
    ));
    let mut bytes = text.into_bytes();
    for (name, file) in ool_bytes(&dir) {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&file);
    }
    let _ = fs::remove_dir_all(dir);
    fnv(&bytes)
}

/// D7.8: the extraction of `carried_play_options` and
/// `carry_presentation_into` is byte-identical for every moonstone slot.
/// The digests were recorded from `apply_gate_travel` before the refactor.
#[test]
fn gate_travel_golden_digest_per_moonstone_slot_is_unchanged_by_the_carry_refactor() {
    let digests: Vec<u64> = (0..MOONSTONE_SLOT_COUNT).map(gate_travel_digest).collect();
    assert_eq!(digests, GOLDEN_GATE_TRAVEL_DIGESTS.to_vec());
}

const GOLDEN_GATE_TRAVEL_DIGESTS: [u64; MOONSTONE_SLOT_COUNT] = [
    0x5036_4669_2f8b_d6a3,
    0xff64_aefc_b890_7278,
    0x7117_1bb5_7472_253c,
    0xaa8d_ff24_f993_6b16,
    0x03be_a89f_110d_b20e,
    0x9e65_5e79_a8d0_c843,
    0x9ef8_b9a0_bc65_22f4,
    0xc526_e62b_82d0_20d9,
];
