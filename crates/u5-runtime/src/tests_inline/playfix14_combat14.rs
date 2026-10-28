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
}
