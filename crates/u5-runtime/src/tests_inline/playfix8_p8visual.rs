// Regressions for the eighth-pass visual exploratory-playtest findings
// (`p8visual`, N14-N21).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn p8visual_town_with_guard_and_villager() -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    let slots = vec![
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 0x70,
            dialog_id: 0,
            schedule: [0, 0, 0, 5, 5, 5, 5, 5, 5, 0, 0, 0, 0, 6, 12, 18],
            name: None,
        },
        NpcSlot {
            slot: 2,
            type_byte: 0x50,
            dialog_id: 0,
            schedule: [0, 0, 0, 8, 8, 8, 8, 8, 8, 0, 0, 0, 0, 6, 12, 18],
            name: None,
        },
    ];
    state.clock = GameClock::new(9, 0).unwrap();
    state.load_scheduled_npcs(&slots);
    state
}

/// N14: `active-objects.md §4` - the initialiser fills "the first six bytes
/// of the record in one pass, leaving the animation and auxiliary bytes to
/// the caller or later animator" - and `animation.md §13.4`: "town NPCs
/// animated in place without ever stepping". Every town NPC slot was placed
/// with the all-ones freeze sentinel, so no town sprite ever animated.
#[test]
fn playtest_n14_town_npc_slots_are_not_placed_frozen_and_animate() {
    let mut state = p8visual_town_with_guard_and_villager();
    let linked: Vec<usize> = state
        .npcs
        .iter()
        .filter_map(|npc| npc.active_object)
        .collect();
    assert_eq!(linked.len(), 2, "guard and villager are linked");
    for &slot in &linked {
        assert_ne!(
            state.active_objects[slot].phase & 0x0f,
            STEADY_PHASE,
            "slot {slot} must not carry the freeze sentinel"
        );
    }
    let mut changed = vec![false; state.active_objects.len()];
    for _ in 0..400 {
        let before: Vec<u8> = state.active_objects.iter().map(|o| o.tile).collect();
        state.animate_active_objects();
        for (slot, object) in state.active_objects.iter().enumerate() {
            if object.tile != before[slot] {
                changed[slot] = true;
            }
        }
    }
    for &slot in &linked {
        assert!(changed[slot], "slot {slot} advanced at least one frame");
    }
}

/// N14: `npc-schedules.md §11` - "Moving on the player's floor, already
/// linked. Update the linked slot's coordinates." A step rewrote the whole
/// record, restarting the sprite's animation script on every move.
#[test]
fn playtest_n14_an_npc_step_updates_only_the_linked_slot_coordinates() {
    let mut state = p8visual_town_with_guard_and_villager();
    let index = state
        .npcs
        .iter()
        .position(|npc| npc.type_byte == 0x50)
        .unwrap();
    let slot = state.npcs[index].active_object.unwrap();
    state.active_objects[slot].tile = 0x52;
    state.active_objects[slot].phase = 0x23;
    state.npcs[index].x += 1;
    assert!(state.sync_npc_active_object(index, 0));
    let object = state.active_objects[slot];
    assert_eq!(object.x, state.npcs[index].x);
    assert_eq!(object.y, state.npcs[index].y);
    assert_eq!(object.tile, 0x52, "the displayed frame survives the step");
    assert_eq!(object.phase, 0x23, "the script position survives the step");
}

/// N16: `blackthorn.md §7`'s print-and-wait order - "8. Print beat 4. 9.
/// Install the party-on-foot actor ... run the six-row envelope sequence" -
/// and `§7.1` item 4: "Print the thunder line, then run the shared
/// full-viewport flash/low-rumble effect twice consecutively". The cinematic
/// emitted the envelopes and both flashes in one batch as it started, so the
/// envelopes sounded under the darkness beat and the flashes long before the
/// thunder line.
#[test]
fn playtest_n16_rescue_audio_follows_the_beats_it_belongs_to() {
    let mut state = dungeon_state(open_dungeon_record(), 3, 1, 1);
    let serial = state.sound_effect_serial;
    state
        .apply_blackthorn_rescue_refuge(Path::new("/nonexistent-p8visual"))
        .unwrap();
    assert!(
        state.sound_effects_after(serial).is_empty(),
        "nothing sounds before the darkness beat"
    );
    // Release the beats one clock step at a time and log text and audio in
    // the order they happen.
    let mut log: Vec<String> = Vec::new();
    let mut seen = serial;
    let mut now = 0.0;
    while state.staged_narration_active() && now < 120.0 {
        let before: Vec<String> = state
            .staged_narration
            .beats()
            .map(|beat| beat.text.clone())
            .collect();
        state.advance_staged_narration_to(now);
        let after = state.staged_narration.beats().count();
        for text in &before[..before.len() - after] {
            log.push(format!("text:{text}"));
        }
        for effect in state.sound_effects_after(seen) {
            log.push(match effect {
                SoundEffect::BlackthornRescueEnvelopes => "envelopes".to_string(),
                SoundEffect::MajorFlash { .. } => "flash".to_string(),
                other => format!("{other:?}"),
            });
        }
        seen = state.sound_effect_serial;
        now += 0.02;
    }
    let position = |entry: &str| {
        log.iter()
            .position(|item| item == entry)
            .unwrap_or_else(|| panic!("{entry} missing from {log:?}"))
    };
    let third = position(&format!(
        "text:{}",
        crate::blackthorn::BLACKTHORN_RESCUE_REFUGE_THIRD
    ));
    let shout = position(&format!("text:{}", crate::blackthorn::BLACKTHORN_RESCUE_SHOUT));
    let thunder = position(&format!(
        "text:{}",
        crate::blackthorn::BLACKTHORN_RESCUE_THUNDER
    ));
    let envelopes = position("envelopes");
    assert!(third < envelopes && envelopes < shout, "{log:?}");
    let flashes: Vec<usize> = log
        .iter()
        .enumerate()
        .filter(|(_, item)| *item == "flash")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(flashes.len(), 2, "{log:?}");
    assert!(flashes.iter().all(|&index| index > thunder), "{log:?}");
    assert!(
        log[flashes[1] + 1..].iter().any(|item| item.starts_with("text:")),
        "the verdict follows the paired flash: {log:?}"
    );
}
