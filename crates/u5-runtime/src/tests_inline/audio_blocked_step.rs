// `systems/audio.md §7.4` blocked-step scope regressions.
//
// §7.4 names "exactly four call sites in the shipped game [that] carry the
// blocked-step recipe": one overworld, one town, two combat. Each test below
// pins one row of that table or one of the published exceptions to it.

/// A tile `foot_terrain_accepts` refuses, so a step onto it is refused for
/// terrain rather than for an object.
const IMPASSABLE_FOOT_TILE: u8 = 0x01;
/// A dungeon byte `is_dungeon_walkable` refuses (high nibble in `0xB..=0xD`).
const IMPASSABLE_DUNGEON_TILE: u8 = 0xB0;

#[test]
fn a_refused_overworld_step_beeps() {
    // audio.md §7.4, as corrected: "a rejected overworld step beeps too, with
    // the identical recipe; the earlier sentence under-scoped by one mode"
    // (RETRACTIONS.md). "Otherwise the path prints `Blocked!`" and beeps.
    let mut grid = open_world_grid();
    grid[world_cell_index(11, 20)] = IMPASSABLE_FOOT_TILE;
    let mut state = world_state(grid, 10, 20);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state
            .step_world(Direction::East, 11, 20, WorldPlane::Underworld, None)
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::BlockedStep]
    );
}

#[test]
fn an_accepted_overworld_step_stays_silent() {
    // audio.md §7.4: "Successful top-down movement has no corresponding
    // footstep sound. The beep is a rejection cue and must not be attached to
    // ordinary movement." §9 repeats it for "successful top-down walking".
    let mut state = world_state(open_world_grid(), 10, 20);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state
            .step_world(Direction::East, 11, 20, WorldPlane::Underworld, None)
            .unwrap(),
        MoveOutcome::Moved
    );
    assert!(state.sound_effects_after(serial).is_empty());
}

#[test]
fn a_refused_under_sail_overworld_step_stays_silent() {
    // audio.md §7.4: under sail the path prints `BREAKING UP!`, `COLLISION!`,
    // or `Docked!`, and "**No 165 Hz beep occurs on any under-sail path**".
    // §11 lists "any under-sail refusal" in the beep's not-produced-by column.
    // 0x05 is land: `ship_terrain_accepts` takes only 0x00..=0x02, so the
    // fixture's default grid already refuses a ship here.
    let mut state = world_state(open_world_grid(), 10, 20);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        sails_hoisted: true,
        hull: 40,
        skiffs: 0,
    };
    assert!(state.player.transport.is_ship_under_sail());
    // Take the wind out of the sail gate so the step reaches the tile test.
    state.wind = WindState::East;
    state.sail_cadence = u8::MAX;
    let serial = state.sound_effect_serial;

    assert_eq!(
        state
            .step_world(Direction::East, 11, 20, WorldPlane::Underworld, None)
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::ShipCollisionRumble],
        "the collision rumble replaces the 165 Hz blocked-step beep"
    );
}

#[test]
fn a_refused_town_step_beeps_on_both_arms() {
    // audio.md §7.4 town row: "Prints `Blocked!`, beeps, flushes type-ahead.
    // Two refusal arms (object occupancy, tile-class refusal) share one tail."
    let mut grid = open_grid();
    grid[4 * 32 + 5] = IMPASSABLE_FOOT_TILE;
    let mut tile_class_refusal = test_state(grid, 4, 4);
    let serial = tile_class_refusal.sound_effect_serial;
    assert_eq!(tile_class_refusal.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!(tile_class_refusal.message, "Blocked!");
    assert_eq!(
        tile_class_refusal.sound_effects_after(serial),
        vec![SoundEffect::BlockedStep],
        "the tile-class refusal arm"
    );

    let mut object_refusal = test_state(open_grid(), 4, 4);
    object_refusal.active_objects.push(ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 5,
        y: 4,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert!(
        object_refusal.blocking_object_at(5, 4).is_some(),
        "the fixture must place a blocking object on the destination"
    );
    let serial = object_refusal.sound_effect_serial;
    assert_eq!(object_refusal.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!(object_refusal.message, "Blocked!");
    assert_eq!(
        object_refusal.sound_effects_after(serial),
        vec![SoundEffect::BlockedStep],
        "the object-occupancy refusal arm"
    );
}

#[test]
fn an_accepted_town_step_stays_silent() {
    // audio.md §9: "successful top-down walking" has no acknowledgement sound.
    let mut state = test_state(open_grid(), 4, 4);
    let serial = state.sound_effect_serial;
    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
    assert!(state.sound_effects_after(serial).is_empty());
}

#[test]
fn a_refused_dungeon_step_stays_silent_on_both_arms() {
    // audio.md §7.4: the dungeon carries **zero** of the four sites - "Silent.
    // No sound call on either refusal arm" - and §9 lists "a rejected dungeon
    // step, on either refusal arm" among the explicit silence boundaries.
    // "A frontend that ports the 165 Hz beep into the dungeon is adding a
    // sound the original does not have."
    let mut wall = dungeon_state(open_dungeon_record(), 0, 4, 4);
    wall.grid[dungeon_cell_index(0, 5, 4)] = IMPASSABLE_DUNGEON_TILE;
    let serial = wall.sound_effect_serial;
    assert_eq!(
        wall.step_dungeon(Direction::East, 5, 4, DungeonScene::new(40).unwrap(), 0, None)
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(wall.message, "Blocked!");
    assert!(
        wall.sound_effects_after(serial).is_empty(),
        "the dungeon rejection makes no sound call: {:?}",
        wall.sound_effects_after(serial)
    );

    let mut diagonal = dungeon_state(open_dungeon_record(), 0, 4, 4);
    let serial = diagonal.sound_effect_serial;
    assert_eq!(
        diagonal
            .step_dungeon(
                Direction::NorthEast,
                5,
                3,
                DungeonScene::new(40).unwrap(),
                0,
                None
            )
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert!(diagonal.sound_effects_after(serial).is_empty());
}

#[test]
fn the_blocked_step_recipe_is_the_published_reference_cue() {
    // audio.md §7.4: "a blocking 165 Hz tone held for 200 calibrated units",
    // and "this is the reference cue for the whole anchor". The census in the
    // same section gives the 220/150 Hz pair a separate, **unidentified**
    // event: "Do not conflate it with the blocked step." Nothing in the engine
    // emits it, which is what §7.4 asks for.
    assert_eq!(audio::BLOCKED_STEP_HZ, 165);
    assert_eq!(audio::BLOCKED_STEP_HOLD_UNITS, 200);
    let program = SoundEffect::BlockedStep.program(&mut audio::RumbleJitter::new());
    assert_eq!(program.frequencies(), vec![165]);
}

#[test]
fn the_summon_flash_tiles_follow_the_published_class_arithmetic() {
    // audio.md §8.3.1: "flash tile = creature class x 4 + 320", and "the settle
    // tile that replaces it is creature class x 4 + 64" - which is the
    // renderer-facing sprite byte the engine already derives.
    for class in [0u8, 1, COMBAT_CLASS_DAEMON, 31, 63] {
        assert_eq!(
            combat_class_summon_flash_tile(class),
            u16::from(class) * 4 + 320
        );
        assert_eq!(
            combat_class_sprite_byte(class),
            class.wrapping_mul(4).wrapping_add(64)
        );
    }
    // The flash bank sits above the 8-bit tile range for every class.
    assert!(combat_class_summon_flash_tile(0) > u16::from(u8::MAX));
    // The converge order the flash reuses is a 256-position permutation that
    // visits every sub-pixel of the cell exactly once (§8.3.1, "256 plots
    // covering 256 distinct sub-pixels").
    let order = crate::return_to_view::return_to_view_single_cell_write_coordinates();
    let mut seen = [false; 256];
    for (x, y) in order {
        let index = usize::from(x) * 16 + usize::from(y);
        assert!(!seen[index], "the converge visits ({x}, {y}) twice");
        seen[index] = true;
    }
    assert!(seen.iter().all(|visited| *visited));
}

#[test]
fn an_accepted_combat_exit_snaps() {
    // audio.md §7.4: "The accepted exit arm prints `Escape!` and plays the
    // 40-update action snap." §11 lists "the accepted combat exit (`Escape!`)"
    // among the action snap's producers - it is the generic snap shared with
    // eight further sites, not a bespoke escape cue.
    let mut state = test_state(open_grid(), 4, 4);
    state.combat_active = true;
    // No unmarked party-side actor remains, so the cleanup is accepted.
    state.combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    state.combat_actors[6] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 0, 0, 5, 5]);
    assert_eq!(
        state.combat_escape_cleanup_decision(),
        CombatEscapeCleanupDecision::Accepted
    );

    let serial = state.sound_effect_serial;
    let application = state.apply_combat_escape_cleanup();
    assert_eq!(application.decision, CombatEscapeCleanupDecision::Accepted);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );
}

#[test]
fn a_refused_combat_exit_cleanup_stays_silent_here() {
    // audio.md §7.4 lists exactly two beeping combat refusals, and neither is
    // the Escape cleanup's own `Not here!`/`Not yet!` arms: those reach the
    // out-of-arena sites, not this helper. §9 keeps unlisted refusals silent.
    let mut state = test_state(open_grid(), 4, 4);
    state.combat_active = true;
    state.combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
    state.combat_actors[6] =
        CombatActorDescriptor::from_row([10, 1, COMBAT_ACTOR_FLAG_SELECTABLE_40, 32, 0, 0, 5, 5]);
    assert_ne!(
        state.combat_escape_cleanup_decision(),
        CombatEscapeCleanupDecision::Accepted
    );

    let serial = state.sound_effect_serial;
    let application = state.apply_combat_escape_cleanup();
    assert_ne!(application.decision, CombatEscapeCleanupDecision::Accepted);
    assert!(state.sound_effects_after(serial).is_empty());
}

#[test]
fn the_two_ring_vanish_paths_share_the_snap_and_order_it_per_path() {
    // audio.md §8.1, as corrected: both paths are "a 1-in-16 random roll with
    // no player interaction" - "there is no confirmation prompt" and any text
    // describing a cancelled confirmation is withdrawn (RETRACTIONS.md). Both
    // play the same 40-update action snap; only their step order differs, so
    // neither ordering may be asserted across both.
    //
    // Ready/equip path: "print `Ring vanishes!`, destroy the item, then play
    // the 40-update action snap."
    let mut ready = test_state(open_grid(), 1, 1);
    ready.equipment_stock[EQUIPMENT_ID_RING_REGENERATION] = 1;
    ready.turn = 4;
    let serial = ready.sound_effect_serial;
    assert_eq!(
        ready.ready_equipment(InlineReadyRequest {
            party_index: 0,
            item_id: EQUIPMENT_ID_RING_REGENERATION,
        }),
        MoveOutcome::Used
    );
    assert!(ready.message.ends_with("but it vanished."));
    assert_eq!(
        ready.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY,
        "the Ready path destroys before it sounds"
    );
    assert_eq!(
        ready.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );

    // Terrain-combat-entry path: "print `A ring has vanished!`, play the
    // 40-update action snap, then remove the item." Same recipe, same odds.
    let mut entry = test_state(open_grid(), 1, 1);
    entry.combat_active = true;
    entry.party_equipment.resize(
        entry.party.len().max(1),
        [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
    );
    entry.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_REGENERATION as u8;
    entry.combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    entry.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 3, 3]);
    let serial = entry.sound_effect_serial;
    let outcome = entry.apply_combat_magic_ring_pass_to_slot(0, 1, 0);
    assert_eq!(
        outcome.and_then(|outcome| outcome.vanished_ring),
        Some(EQUIPMENT_ID_RING_REGENERATION as u8),
        "roll 0 is the 1-in-16 destruction"
    );
    assert_eq!(entry.message, "A ring has vanished!");
    assert_eq!(
        entry.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );
    assert_eq!(entry.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
}
