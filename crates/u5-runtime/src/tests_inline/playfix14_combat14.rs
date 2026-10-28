// Regressions for the fourteenth exploratory playtest's combat findings
// (P14-xx). Each test names the finding and the published sentence it rests
// on.
mod playfix14_combat14 {
    use super::*;

    fn p14_rows(state: &PlayState) -> Vec<String> {
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.trim_end().to_string())
            .collect()
    }

    /// A one-member combat with a dragon two cells east of the caster, and
    /// the `Creature:` cursor opened through the interactive cast prompt.
    fn creature_prompt_open(code: &str) -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state
            .active_objects
            .resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 8,
            hp: 30,
            max_hp: 30,
            level: 8,
        }];
        let spell_index = spell_index_from_code(code).unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[target_slot] = CombatActorDescriptor::for_monster_placement(
            combat_class_stats(39).unwrap(),
            target_slot as u8,
            5,
            3,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            2,
        );
        state.active_cast = Some(CastSession::new(0));
        for ch in code.chars() {
            state.step_active_cast(ch, "", Path::new("")).unwrap();
        }
        assert!(state
            .step_active_cast('\r', "", Path::new(""))
            .unwrap()
            .is_none());
        assert!(matches!(
            state
                .active_cast_followup
                .as_ref()
                .map(|session| session.kind),
            Some(CastFollowupKind::CombatCreatureCursor { .. })
        ));
        state
    }

    /// P14-01. `magic.md §8` handler-family map: Charm, Polymorph and Clone
    /// "Prompt `Creature:`" - one label row. Escape completes that row, the
    /// way every published cancel completes its own prompt's row
    /// (`magic.md §5.1` `On who: None!`); it does not print the label again
    /// under the waiting one.
    #[test]
    fn p14_01_creature_prompt_escape_completes_the_waiting_row_once() {
        for code in ["AEX", "BRX", "IQX"] {
            let mut state = creature_prompt_open(code);
            state.flush_message_slot();
            assert_eq!(
                p14_rows(&state).last().map(String::as_str),
                Some(COMBAT_CREATURE_TARGET_PROMPT.trim_end()),
                "{code}: the label is the logged open row"
            );
            assert!(state
                .step_active_cast_followup('\u{1b}', "", Path::new(""))
                .unwrap()
                .is_none());
            state.flush_message_slot();
            let rows = p14_rows(&state);
            let labelled = rows
                .iter()
                .filter(|row| row.starts_with("Creature:"))
                .collect::<Vec<_>>();
            assert_eq!(labelled, vec!["Creature: None!"], "{code}: {rows:#?}");
            assert!(!state.message_row_open_mid_line, "{code}");
        }
    }

    /// P14-02. `stats-panel.md §2.2.1` row 15: combat poisoning of a Good
    /// member raises the deferred request, and `§2.3` / `combat.md §8`
    /// drain it "at the top of a human-controlled actor's command prompt
    /// ... before the keystroke is read". A committed cast drives the round
    /// walker to the next actor's prompt *inside* the same command, so that
    /// prompt is where the `P` must reach the panel - not one prompt later,
    /// once the command epilogue has raised the request after the drain.
    #[test]
    fn p14_02_a_deferred_combat_change_reaches_the_panel_at_the_next_actors_prompt() {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.repaint_stats_panel();
        // The command epilogue's entry framing, then the poison site's own
        // mechanism (`combat_frame.rs` Poison Wind arm).
        state.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Immediate;
        state.stats_panel_command_baseline = crate::stats_panel::stats_panel_snapshot_of(&state);
        state.party[0].status = b'P';
        state.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
        assert_eq!(state.stats_panel.statuses[0], Some(b'G'));

        // The walker reaches the next human actor's prompt mid-command.
        state.pending_combat_actor_slot = None;
        state.ensure_pending_combat_player_turn();
        assert!(state.pending_combat_actor_slot.is_some());
        assert_eq!(
            state.stats_panel.statuses[0],
            Some(b'P'),
            "the waiting prompt shows the poisoned status"
        );
        assert!(!state.stats_panel_refresh_requested);
    }
}
