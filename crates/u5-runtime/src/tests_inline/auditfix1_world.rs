// Closed-audit world-rule gaps (u5-spec#302 audit, aud-world): the paced
// world ticks of the underfoot arms, the `0xFF` probe and the camp
// apparition, the camp cooldown's place after the event, and the tavern
// table-scraps gift draw.

fn auditfix1_world_on(tile: u8) -> PlayState {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = tile;
    let mut state = world_state(grid, 5, 5);
    for member in &mut state.party {
        member.hp = 200;
        member.max_hp = 200;
        // Dexterity 30: the outdoor swamp save's `1..30` draw never fails.
        member.climb_stat = 30;
    }
    state.prng_state = 0x1234;
    state
}

/// `overworld.md §5` gameplay draw order item 7 (issue #305 Q5): "the lava
/// arm's view rebuild is one world tick before its burn rolls" - the wind
/// check comes first, then one `1..8` per living member.
#[test]
fn auditfix1_world_lava_arm_takes_a_world_tick_before_its_burn_rolls() {
    let mut state = auditfix1_world_on(0x8f);
    let members = state.party.iter().filter(|member| member.living()).count();
    assert!(members > 0);

    let mut reference = auditfix1_world_on(0x8f);
    let _ = reference.advance_visual_tick();
    assert_ne!(
        reference.prng_state, 0x1234,
        "a world tick takes the wind draw"
    );
    let burns: Vec<u8> = (0..members)
        .map(|_| reference.random_range_u8(1, 8))
        .collect();

    state
        .append_world_damage_tile_message(None, WorldPlane::Underworld)
        .unwrap();

    assert_eq!(state.prng_state, reference.prng_state);
    for (member, burn) in state.party.iter().zip(&burns) {
        assert_eq!(member.hp, 200 - u16::from(*burn));
    }
}

/// `overworld.md §5` item 7 / `movement.md §8.2`: "the on-foot swamp arm runs
/// one paced world tick after its poison work", "even if everyone saved".
#[test]
fn auditfix1_world_swamp_arm_ends_with_a_full_world_tick() {
    let mut state = auditfix1_world_on(BRIT_SWAMP_TILE);
    state.player.transport = TransportState::Foot;
    let members = state.party.len();

    let mut reference = auditfix1_world_on(BRIT_SWAMP_TILE);
    reference.player.transport = TransportState::Foot;
    for _ in 0..members {
        let _ = reference.outdoor_swamp_poison_roll();
    }
    let after_saves = reference.prng_state;
    let _ = reference.advance_visual_tick();
    assert_ne!(reference.prng_state, after_saves);

    state.append_world_status_tile_message(WorldPlane::Underworld);

    assert!(state.party.iter().all(|member| member.status != b'P'));
    assert_eq!(state.prng_state, reference.prng_state);
}

/// `overworld.md §5` step 2 (R641): "On the first frame entering that state,
/// run one world tick (a full world step with its draws). On later frames
/// while the latch stays set on that tile the probe runs neither a world tick
/// nor a clock call."
#[test]
fn auditfix1_world_underfoot_ff_first_frame_runs_one_world_tick() {
    let mut state = auditfix1_world_on(0xff);
    state.active_effect_tag = None;
    let mut reference = auditfix1_world_on(0xff);
    let _ = reference.advance_visual_tick();

    assert!(state.refresh_world_underfoot_blackout_latch());
    assert_eq!(state.prng_state, reference.prng_state);
    assert_eq!(state.ambient_light, 0);

    let latched = state.prng_state;
    assert!(state.refresh_world_underfoot_blackout_latch());
    assert_eq!(state.prng_state, latched, "a latched frame takes no tick");
}

/// `rest-and-camp.md §5`: "either a miss or a completed event is then followed
/// by the same cooldown write" of 14, so the event's own turn-clock advance
/// (§7.1 step 7) cannot age it.
#[test]
fn auditfix1_world_camp_cooldown_is_written_after_the_apparition_event() {
    let dir = debug_game_dir();
    let mut found = 0;
    for seed in 1u16..600 {
        let mut state = britannia_state(open_world_grid(), 0, 0);
        state.camp_cooldown = 0;
        state.clock = GameClock::new(12, 58).unwrap();
        state.prng_state = seed;
        let outcome = state.hole_up_command(&dir, Some(6)).unwrap();
        if outcome != MoveOutcome::Rested
            || state.combat_active
            || !state.message.contains("An apparition!")
        {
            continue;
        }
        // The event's advance crossed into hour 19.
        assert_eq!(state.clock.hour, 19, "seed {seed:#06x}");
        assert_eq!(
            state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS,
            "seed {seed:#06x}"
        );
        found += 1;
    }
    assert!(found > 0, "no seed reached the apparition event");
    let _ = fs::remove_dir_all(dir);
}
