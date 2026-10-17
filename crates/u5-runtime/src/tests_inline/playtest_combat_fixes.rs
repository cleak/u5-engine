// Regressions for the combat exploratory-playtest findings (E32-E42).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `combat.md §10` / `magic.md §7`: with Negate Magic's `N` tag active, `C`
/// is absorbed "before the shared spell dispatcher or spell-selection prompt
/// runs. The exact command output is `Cast...\nAbsorbed!\n`". "There is no
/// spell-selection prompt", no charge or MP is spent, and the attempt
/// "completes the current combat action". Playtest E35: the engine opened
/// `Spell name:` and absorbed only after the spell-name exchange.
#[test]
fn playtest_e35_negate_magic_absorbs_combat_cast_before_the_spell_prompt() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.active_player = Some(0);
    state.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = NEGATE_MAGIC_ACTIVE_EFFECT_DURATION;
    let spell_index = spell_index_from_code("GP").unwrap();
    state.spell_charges[spell_index] = 1;
    state.party[0].mana = 9;
    state.party[0].level = 8;
    state.ensure_pending_combat_player_turn();
    let serial = state.sound_effect_serial;

    handle_play_key_input(&mut state, 'C', "", game_dir).unwrap();

    assert!(
        state.active_cast.is_none(),
        "no spell-selection prompt may open"
    );
    let rows = combat_message_window_rows(&state);
    assert!(
        !rows.iter().any(|row| row.contains("Spell name")),
        "rows: {rows:?}"
    );
    let cast_row = rows
        .iter()
        .position(|row| row == ">Cast...")
        .unwrap_or_else(|| panic!("the Cast... echo is drawn: {rows:?}"));
    assert_eq!(rows[cast_row + 1], "Absorbed!", "rows: {rows:?}");
    assert!(
        state
            .sound_effects_after(serial)
            .contains(&SoundEffect::MagicAbsorbed)
    );
    assert_eq!(state.spell_charges[spell_index], 1);
    assert_eq!(state.party[0].mana, 9);
}

/// `combat.md §8.4`: "the shared cast dispatcher emits `Not here!\n`, `None
/// mixed!\n` ... Combat supplies the following banner's leading feed", and
/// `magic.md §7`: the result literals carry trailing feeds and "Combat's
/// subsequent actor banner has its own leading feed, which leaves a blank
/// row after a completed result". Playtest E40: the cast result ran straight
/// into the next banner.
#[test]
fn playtest_e40_a_combat_cast_result_is_followed_by_a_blank_row() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.active_player = Some(0);
    state.party[0].mana = 9;
    state.party[0].level = 8;
    state.ensure_pending_combat_player_turn();

    for key in ['C', 'I', 'L', '\r'] {
        handle_play_key_input(&mut state, key, "", game_dir).unwrap();
    }

    let rows = combat_message_window_rows(&state);
    let result = rows
        .iter()
        .rposition(|row| row == "Not here!")
        .unwrap_or_else(|| panic!("the Not here! result is drawn: {rows:?}"));
    assert_eq!(rows[result + 1], "", "rows: {rows:?}");
    assert_eq!(rows[result + 2], "Avatar, armed", "rows: {rows:?}");
}

/// A caster at (5, 5) with an open arena, ready to type a spell name.
fn playtest_spell_cursor_state() -> PlayState {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.party[0].mana = 60;
    state.party[0].level = 8;
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.spell_charges = [9; SPELL_COUNT];
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([60, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    assert_eq!(
        state.start_combat_cast_spell_prompt(0, false),
        MoveOutcome::Observed
    );
    state
}

/// `combat.md §8.2`'s cursor table: Enter confirms "**unless** the cursor is
/// on the attacker's own cell, in which case nothing happens and the loop
/// reads another key"; Space on that cell cancels. `magic.md §8`: Missile,
/// Fireball and Kill use "the shared `Aim! ` cursor and projectile path", and
/// the field spells "the standard combat arena cursor". Playtest E33: Return
/// on the starting cell killed or wounded the caster and put a Sleep Field
/// on it.
#[test]
fn playtest_e33_spell_cursors_never_confirm_on_the_casters_own_cell() {
    let game_dir = std::path::Path::new("");
    for (first, rest) in [('C', "X"), ('G', "P"), ('F', "V"), ('G', "IZ")] {
        let mut state = playtest_spell_cursor_state();
        state.step_active_cast(first, rest, game_dir).unwrap();
        state.step_active_cast('\r', "", game_dir).unwrap();
        let session = state
            .active_cast_followup
            .as_ref()
            .unwrap_or_else(|| panic!("{first}{rest} opens the arena cursor"));
        assert!(
            matches!(
                session.kind,
                CastFollowupKind::CombatAimCursor { x: 5, y: 5 }
                    | CastFollowupKind::CombatCoordinate { x: 5, y: 5, .. }
            ),
            "{first}{rest}: {:?}",
            session.kind
        );
        assert_eq!(state.message, "Aim! ", "{first}{rest}");
        assert_eq!(state.combat_aim_marker_cell, Some((5, 5)));
        assert!(state.combat_aim_marker_gate, "the cursor raises the gate");

        // Enter on the caster's own cell: nothing happens.
        assert!(state.step_active_cast_followup('\r', "", game_dir).unwrap().is_none());
        assert!(state.active_cast_followup.is_some(), "{first}{rest}");
        assert_eq!(state.party[0].hp, 60, "{first}{rest}");
        assert_eq!(state.party[0].status, b'G', "{first}{rest}");

        // Space on the caster's own cell cancels silently.
        assert!(state.step_active_cast_followup(' ', "", game_dir).unwrap().is_none());
        assert!(state.active_cast_followup.is_none(), "{first}{rest}");
        assert!(!state.combat_aim_marker_gate);
        assert_eq!(state.message, "", "{first}{rest}");
        assert_eq!(state.party[0].hp, 60, "{first}{rest}");
        assert_eq!(state.party[0].status, b'G', "{first}{rest}");
        assert!(
            state
                .active_objects
                .iter()
                .all(|object| object.type_byte != COMBAT_FIELD_KIND_SLEEP),
            "{first}{rest}"
        );
    }
}

/// `magic.md §8` handler-family map: "Player combat C-Cast uses the arena
/// cursor followed by the ordinary projectile/impact resolver" - the shared
/// `Aim! ` cursor of `combat.md §8.2`, whose ranged-arm cancel "returns
/// silently". Playtest E34: the field spells printed an invented
/// `Target? (x, y)` / `Move cursor with` help block, reprinted per step, and
/// answered Escape with `None!`.
#[test]
fn playtest_e34_field_spells_use_the_shared_aim_cursor_without_help_text() {
    let game_dir = std::path::Path::new("");
    let mut state = playtest_spell_cursor_state();
    state.step_active_cast('F', "GI", game_dir).unwrap();
    state.step_active_cast('\r', "", game_dir).unwrap();
    assert_eq!(state.message, "Aim! ");
    assert_eq!(state.open_prompt_line().as_deref(), Some("Aim! "));

    state.step_active_cast_followup('6', "", game_dir).unwrap();
    assert_eq!(state.message, "Aim! ");
    assert_eq!(state.combat_aim_marker_cell, Some((6, 5)));
    assert!(
        !state
            .message_entries()
            .iter()
            .any(|entry| entry.text.contains("Target?") || entry.text.contains("cursor")),
        "no help text reaches the transcript"
    );

    state.step_active_cast_followup('\u{1b}', "", game_dir).unwrap();
    assert!(state.active_cast_followup.is_none());
    assert_eq!(state.message, "");
}

/// `combat.md §8.3`: with the numpad flag clear "typed digits keep their
/// ASCII meaning and reach the active-player-selection handler"; with it set
/// all eight of `1`-`4`/`6`-`9` are remapped. `§8.4` lists "Valid digit 1-6
/// selection". Playtest E36: `2`, `4`, `6` and `8` stepped the actor while
/// `1`, `3` and `5` selected a member - a mix of the two modes.
#[test]
fn playtest_e36_every_typed_combat_digit_selects_a_member() {
    let game_dir = std::path::Path::new(".");
    for digit in ['2', '4', '6', '8'] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.ensure_pending_combat_player_turn();
        let before = (state.combat_actors[0].x, state.combat_actors[0].y);

        handle_play_key_input(&mut state, digit, "", game_dir).unwrap();

        assert_eq!(
            (state.combat_actors[0].x, state.combat_actors[0].y),
            before,
            "{digit} must not step the actor"
        );
        let rows = combat_message_window_rows(&state);
        // `8` is outside `§8`'s "Digits `1`-`6`" and is refused like any
        // other unmapped key.
        let expected = if digit == '8' {
            "What?"
        } else {
            COMBAT_SET_ACTIVE_PLAYER_LABEL.trim_end()
        };
        assert!(
            rows.iter().any(|row| row.contains(expected)),
            "{digit} answers with {expected:?}: {rows:?}"
        );
        assert!(
            !rows.iter().any(|row| row.contains("North")
                || row.contains("South")
                || row.contains("East")
                || row.contains("West")),
            "{digit} printed a direction: {rows:?}"
        );
    }
}

/// A dungeon-room-shaped arena: void (`0xFF`) everywhere except one floor
/// row through the caster's cell at (5, 5).
fn playtest_void_arena_state(spell_index: usize, prng_state: u16) -> PlayState {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_terrain = [[COMBAT_ARENA_VOID_TILE; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.combat_terrain[5] = [0x05; COMBAT_ARENA_SIDE];
    state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    state.party[0].mana = 99;
    state.party[0].level = 8;
    state.party_intelligence = vec![50];
    state.prng_state = prng_state;
    state.spell_charges[spell_index] = 1;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.active_objects[0] = ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    };
    state
}

/// `magic.md §8`, "The shared random arena probe": Conjure, Swarm, Summon and
/// combat Blink put each in-range candidate "to the shared combat spawn-cell
/// validator, which requires terrain that the placed creature's movement
/// family can occupy, rejects the arena's impassable void byte, and rejects
/// any cell already holding a combat actor". Playtest E32: in a dungeon room
/// these spells placed creatures - and Blink the caster - on the void
/// outside the room's walls.
#[test]
fn playtest_e32_arena_probe_spells_never_place_on_the_void() {
    let conjure = spell_index_from_code("KX").unwrap();
    let swarm = spell_index_from_code("BIX").unwrap();
    let summon = spell_index_from_code("CKX").unwrap();
    let mut placed = 0;
    for seed in 0..48u16 {
        let prng = seed.wrapping_mul(0x9e37).wrapping_add(1);
        for spell in [conjure, swarm, summon, BLINK_SPELL_INDEX] {
            let mut state = playtest_void_arena_state(spell, prng);
            let _ = match spell {
                s if s == conjure => state.cast_combat_conjure_spell(0, spell),
                s if s == swarm => state.cast_combat_swarm_spell(0, spell),
                s if s == summon => state.cast_combat_summon_daemon_spell(0, spell),
                _ => state.cast_combat_blink(0),
            };
            for actor in state.combat_actors.iter().filter(|actor| !actor.is_empty()) {
                assert_ne!(
                    state.combat_terrain[usize::from(actor.y)][usize::from(actor.x)],
                    COMBAT_ARENA_VOID_TILE,
                    "spell {spell} seed {prng:#06x} placed an actor on the void at ({}, {})",
                    actor.x,
                    actor.y
                );
            }
            if spell == BLINK_SPELL_INDEX {
                if (state.combat_actors[0].x, state.combat_actors[0].y) != (5, 5) {
                    placed += 1;
                }
            } else if state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS..]
                .iter()
                .any(|actor| !actor.is_empty())
            {
                placed += 1;
            }
        }
    }
    assert!(placed > 0, "some probe reached the floor row");
}

/// `overworld.md §9.2`: the live-gate entry hook "runs before normal input
/// dispatch, on every iteration" and completes "before any key is read" - so
/// the key that arrives is the next command, not the hook's. Playtest E42:
/// after a Gate Travel onto a live gate cell the first key did nothing (North
/// did not move; `Q` was lost, so `Q`,`Y` ran as Yell).
#[test]
fn playtest_e42_the_live_gate_hook_does_not_swallow_the_next_command_key() {
    let dir = debug_game_dir();
    let idx = world_cell_index(5, 5);
    let mut grid = open_world_grid();
    grid[idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.clock = GameClock::new(23, 0).unwrap();
    state.natural_moongate_counter = MOONGATE_PHASE_FULL;
    // An empty saved slot: the hook plays the transit and warps nowhere.
    state.set_cached_moon_glyph_slots(1, 1);
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: MOONSTONE_INVALID_SCENE,
        x: 0,
        y: 0,
        z: 0,
    };

    handle_play_key_input(&mut state, '\u{3}', "", &dir).unwrap();

    assert!(
        state.last_natural_moongate_transit.is_some(),
        "the hook ran first"
    );
    assert_eq!(state.grid[idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
    assert_eq!(
        (state.player.x, state.player.y),
        (5, 4),
        "the key was then dispatched as the North command"
    );
    let _ = fs::remove_dir_all(dir);
}
