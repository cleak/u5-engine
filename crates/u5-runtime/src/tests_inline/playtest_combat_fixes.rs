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
