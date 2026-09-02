    #[test]
    fn z_stats_opens_browser_stats_page_without_turn() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'B',
                status: b'G',
                climb_stat: 11,
                mana: 4,
                hp: 10,
                max_hp: 20,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'M',
                status: b'P',
                climb_stat: 13,
                mana: 5,
                hp: 6,
                max_hp: 30,
                level: 3,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"MARIA\0\0\0\0"];
        state.party_strengths = vec![12, 14];
        state.party_intelligence = vec![16, 18];
        state.party_experience = vec![1234, 5678];

        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (5, 5));
        assert_eq!(state.turn, 0);
        // inventory.md §4: "The command starts by choosing a character...
        // outside combat they use the normal party-member selector."
        assert!(state.active_z_stats.is_none());
        assert_eq!(state.selector_highlight(), Some(0));
        assert_eq!(state.roster_box_label(), Some("Select:"));
        assert_eq!(state.message, PARTY_SELECTION_PROMPT);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.turn, 0);
        assert!(state.active_party_selector.is_none());
        assert_eq!(state.selector_highlight(), None);
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Stats)
        );
        // §4 stats-page field list, with no invented field labels in
        // front of the class and status label-table values.
        assert!(state.message.contains("AVATAR"));
        assert!(state.message.contains("Bard"));
        assert!(state.message.contains("good"));
        assert!(state.message.contains("Strength     12"));
        assert!(state.message.contains("Dexterity    11"));
        assert!(state.message.contains("Intellect    16"));
        assert!(state.message.contains("HP        10/20"));
        assert!(state.message.contains("MP            4"));
        assert!(state.message.contains("Exp        1234"));
        assert!(!state.message.contains("STR "));
    }

    #[test]
    fn z_stats_equipment_page_skips_empty_slots_and_falls_back() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 11,
                max_hp: 21,
                level: 1,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_equipment = default_party_equipment(2);
        state.party_equipment[0][EQUIP_SLOT_HELM] = 1;
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_ID_BOW as u8;

        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));

        assert_eq!(state.turn, 0);
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Equipment)
        );
        assert!(state.message.contains("helm: "));
        assert!(state.message.contains(equipment_name(EQUIPMENT_ID_BOW)));
        assert!(!state.message.contains("offhand:"));

        assert!(state.step_active_z_stats('2', ""));
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Equipment))
        );
        // `inventory.md §4.7`: "if all six slots are empty the page prints
        // the `(None ready)` placeholder rather than a blank list."
        assert!(state.message.contains(Z_STATS_NONE_READY_PLACEHOLDER));
    }

    /// `inventory.md §4.7`: "Long pages **do not paginate**: the navigator
    /// scans forward or backward for the next slot with a non-zero count, so
    /// empty slots are skipped rather than shown as blank rows", and "The row
    /// scanner walks a caller-supplied counter band forward or backward from a
    /// mutable cursor, skipping zero-count rows for ordinary inventory
    /// browsing."
    ///
    /// This test previously pinned the opposite behaviour under the name
    /// `z_stats_magic_inventory_pages_show_zero_rows`: an invented
    /// `Rows 1-8 of 48` page counter, `Ginseng: 0` reagent rows and
    /// `GP Magic Missile: 0 (zero)` spell rows, with `]` jumping a whole
    /// eight-row panel. All three contradict §4.7 and are now asserted absent.
    #[test]
    fn z_stats_inventory_pages_skip_zero_slots_and_never_paginate() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [3, 0, 0, 0, 0, 0, 0, 0];
        state.spell_charges = [0; SPELL_COUNT];
        state.spell_charges[IN_LOR_SPELL_INDEX] = 2;
        state.keys = 0;
        state.gems = 2;
        state.torches = 0;
        state.climbing_gear = 1;
        state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
        state.scroll_stock[SCROLL_LIGHT_INDEX] = 1;
        state.potion_stock[POTION_BLUE_INDEX] = 4;
        state.equipment_stock = [0; EQUIPMENT_COUNT];
        state.equipment_stock[EQUIPMENT_ID_BOW] = 2;

        // `inventory.md §4`: "The first two pages are character-specific:
        // page 1 is the primary stat page and page 2 is the equipment page.
        // Later inventory pages walk shared counter bands for reagents, spell
        // charges, special/use items, and the weapons/armour stash." Reagents
        // is therefore the **third** page, two forward steps from the stats
        // page. The engine's spell-book page is not one of the six published
        // pages (`§4.7`), so it does not sit between them.
        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));
        assert!(state.step_active_z_stats('>', ""));

        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Reagents)
        );
        assert!(state.message.contains("Sulfur Ash: 3"));
        // The seven empty reagent slots are skipped, not drawn as blank or
        // zero rows.
        assert!(
            !state.message.contains("Ginseng"),
            "reagents page message was {:?}",
            state.message
        );
        assert!(!state.message.contains(": 0"));

        assert!(state.step_active_z_stats('>', ""));
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::Spells)
        );
        assert!(state.message.contains("IL Light: 2"));
        // No page counter, and none of the forty-seven zero-charge slots.
        assert!(
            !state.message.contains("Rows"),
            "spells page message was {:?}",
            state.message
        );
        assert!(!state.message.contains("Magic Missile"));
        assert!(!state.message.contains("(zero)"));

        // One displayable slot, so a forward scan finds no next non-zero slot
        // and the band stays where it is.
        assert!(state.step_active_z_stats(']', ""));
        assert!(state.message.contains("IL Light: 2"));
        assert!(!state.message.contains("Rows"));

        // Ten displayable slots on a forty-eight-slot band: the panel holds
        // eight, and each key moves the scan by **one** non-zero slot rather
        // than by a panel of eight.
        for index in 1..10 {
            state.spell_charges[index] = 1;
        }
        let rows: Vec<String> = (0..10)
            .map(|index| {
                format!(
                    "{} {}: {}",
                    SPELL_CODES[index],
                    spell_common_name(index).unwrap(),
                    if index == 0 { 2 } else { 1 }
                )
            })
            .collect();

        assert!(state.step_active_z_stats('Z', ""));
        assert!(state.message.contains(&rows[0]));
        assert!(state.message.contains(&rows[7]));
        assert!(!state.message.contains(&rows[8]));

        assert!(state.step_active_z_stats(']', ""));
        assert!(
            !state.message.contains(&rows[0]),
            "one forward scan should drop exactly the first slot: {:?}",
            state.message
        );
        assert!(state.message.contains(&rows[1]));
        assert!(state.message.contains(&rows[8]));
        assert!(!state.message.contains(&rows[9]));

        assert!(state.step_active_z_stats(']', ""));
        assert!(state.message.contains(&rows[2]));
        assert!(state.message.contains(&rows[9]));
        assert!(!state.message.contains(&rows[1]));

        // §4.7 publishes the scan but not what happens past the end of a
        // band; the engine's conservative reading is that a scan with no
        // further non-zero slot leaves the cursor alone, and in particular
        // never wraps back to the top of the band.
        assert!(state.step_active_z_stats(']', ""));
        assert!(state.message.contains(&rows[9]));
        assert!(!state.message.contains(&rows[1]));

        assert!(state.step_active_z_stats('[', ""));
        assert!(state.message.contains(&rows[1]));
        assert!(!state.message.contains(&rows[9]));

        assert!(state.step_active_z_stats('>', ""));
        assert_eq!(
            state.active_z_stats.as_ref().map(|session| session.page),
            Some(ZStatsPage::SpecialUse)
        );
        assert!(state.message.contains("Gems: 2"));
        assert!(state.message.contains("Grapple: 1"));
        assert!(state.message.contains("Sextant: 1"));
        assert!(state.message.contains("Scroll LV: 1"));
        assert!(state.message.contains("Blue Potion: 4"));
        assert!(!state.message.contains("Keys:"));

        assert!(state.step_active_z_stats('>', ""));
        assert!(state.message.contains(&format!(
            "{}: 2",
            equipment_name(EQUIPMENT_ID_BOW)
        )));
        assert!(!state.message.contains(equipment_name(EQUIPMENT_ID_CROSSBOW)));
    }

    /// `inventory.md §4.7`: "`(None owned!)` | An inventory page has no slot
    /// with a non-zero count", and "When no displayable row exists, the panel
    /// prints the none placeholder and waits for a key before returning to
    /// the page loop."
    ///
    /// All four published inventory pages - Reagents, Spells, Items and
    /// Armaments - skip zero-count slots, so all four can run out of
    /// displayable rows. Before this batch the Reagents and Spells pages
    /// rendered their empty slots as rows and the placeholder was unreachable
    /// on both.
    #[test]
    fn every_z_stats_inventory_page_reaches_the_none_owned_placeholder() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.spell_charges = [0; SPELL_COUNT];
        state.keys = 0;
        state.gems = 0;
        state.torches = 0;
        state.climbing_gear = 0;
        state.special_items = [0; SPECIAL_ITEM_COUNT];
        state.scroll_stock = [0; SCROLL_COUNT];
        state.potion_stock = [0; POTION_COUNT];
        state.equipment_stock = [0; EQUIPMENT_COUNT];

        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));

        for page in [
            ZStatsPage::Reagents,
            ZStatsPage::Spells,
            ZStatsPage::SpecialUse,
            ZStatsPage::EquipmentStock,
        ] {
            assert!(state.step_active_z_stats('>', ""));
            assert_eq!(
                state.active_z_stats.as_ref().map(|session| session.page),
                Some(page)
            );
            assert!(
                state.message.contains(Z_STATS_NONE_OWNED_PLACEHOLDER),
                "{page:?} page message was {:?}",
                state.message
            );
        }
    }

    #[test]
    fn z_stats_spell_book_filters_by_class_and_level_without_using_charges() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'B',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 2,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 8,
            },
        ];
        state.spell_charges = [0; SPELL_COUNT];

        // `magic.md §11` publishes the spell-book panel's contract: "A
        // per-character spell-book panel lists the spells the character can
        // attempt to cast ... The list is filtered by the character's class
        // and level (Mages and Druids have full access; other classes see a
        // smaller subset). The list does *not* depend on charges."
        //
        // It is **not**, however, one of the six pages the Z-stats page loop
        // cycles through: `inventory.md §4.7` publishes six pages - attributes,
        // equipment, and four inventory pages - and no spell-book page is
        // among them. So the panel is rendered directly here rather than
        // navigated to. This test used to assert two forward steps land on it,
        // which the published page sequence rules out.
        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        let session = ZStatsSession {
            selected_party_index: 0,
            page: ZStatsPage::SpellBook,
            inventory_cursor: 0,
        };
        assert!(
            !ZStatsPage::ORDERED.contains(&ZStatsPage::SpellBook),
            "inventory.md §4.7 publishes six pages and no spell-book page"
        );
        state.message = state.render_z_stats_session(&session);
        assert!(state.message.contains("Z-stats: Spell Book page"));
        assert!(state.message.contains("C1 MP1 IL"));
        assert!(state.message.contains("In Lor / Light"));
        assert!(state.message.contains("C2 MP2 AS"));
        assert!(!state.message.contains("C3 LV"));
        assert!(!state.message.contains("IL Light: 0"));

        // The Fighter in slot 2 is one of the classes that "see a smaller
        // subset" - here, none at all. Rendered directly for the same reason:
        // `inventory.md §4` says a number-key jump preserves "whether the
        // current character page is the stats page or the equipment page",
        // which are the only two character-specific pages it publishes.
        let fighter = ZStatsSession {
            selected_party_index: 1,
            page: ZStatsPage::SpellBook,
            inventory_cursor: 0,
        };
        state.message = state.render_z_stats_session(&fighter);
        assert!(state.message.contains("No spell access."));
    }

    #[test]
    fn z_stats_navigation_rejects_out_of_range_party_and_exits() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 11,
            max_hp: 21,
            level: 1,
        });
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_equipment = default_party_equipment(2);

        assert_eq!(state.z_stats(), MoveOutcome::Observed);
        assert!(state.step_active_z_stats('>', ""));
        assert!(state.step_active_z_stats('2', ""));
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Equipment))
        );

        assert!(state.step_active_z_stats('6', ""));
        assert!(state.message.contains("Party has 2 members."));
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Equipment))
        );

        assert!(state.step_active_z_stats(' ', ""));
        assert!(state.active_z_stats.is_none());
        assert_eq!(state.message, "Z-stats closed.");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn active_cast_prompt_collects_selector_and_dispatches_spell() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party[0].mana = IN_LOR_COST;
        state.party[0].level = IN_LOR_COST;
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_cast.is_some());
        assert_eq!(state.turn, 0);
        assert!(state.message.contains("Spell name: _"));

        assert_eq!(
            handle_play_key_input(&mut state, 'I', "L", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_cast.is_some());
        // `magic.md §5` Step 2: "each letter prints its associated rune
        // word followed by a space" - `IL` echoes as `In Lor `.
        assert!(state.message.contains("Spell name: In Lor "));
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 1);

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_cast.is_none());
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.light_spell_counter, IN_LOR_LIGHT_DURATION);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn active_cast_prompt_ignores_j_o_and_supports_backspace_cancel() {
        let mut state = test_state(open_grid(), 5, 5);

        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(state.step_active_cast('J', "OI", Path::new("")).unwrap().is_none());
        assert!(state.message.contains("Spell name: I"));
        assert!(state.step_active_cast('\u{8}', "", Path::new("")).unwrap().is_none());
        assert!(state.message.contains("Spell name: _"));
        assert!(state.step_active_cast('\u{1b}', "", Path::new("")).unwrap().is_none());
        assert!(state.active_cast.is_none());
        assert_eq!(state.message, "None!");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn directed_utility_followup_spends_first_ignores_escape_and_accepts_pass() {
        let mut state = test_state(open_grid(), 5, 5);
        state.spell_charges[VANISH_SPELL_INDEX] = 1;
        state.party[0].mana = VANISH_COST;
        state.party[0].level = VANISH_COST;

        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(state
            .step_active_cast('A', "Y", Path::new(""))
            .unwrap()
            .is_none());
        assert!(state
            .step_active_cast(' ', "", Path::new(""))
            .unwrap()
            .is_none());
        assert!(state.active_cast_followup.is_some());
        assert_eq!(state.spell_charges[VANISH_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 0);

        assert!(state
            .step_active_cast_followup('\u{1b}', "", Path::new(""))
            .unwrap()
            .is_none());
        assert!(state.active_cast_followup.is_some());
        assert!(state.message.starts_with(SPELL_DIRECTION_PROMPT_PREFIX));

        let result = state
            .step_active_cast_followup(' ', "", Path::new(""))
            .unwrap()
            .expect("Space/Pass should finish the already-spent Vanish cast");
        assert_eq!(result.0, MoveOutcome::Cast);
        assert_eq!(result.1, None);
        assert!(state.active_cast_followup.is_none());
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, DIRECTION_PROMPT_LABEL_PASS);
    }

    #[test]
    fn active_cast_direction_followup_collects_cardinal_before_spending_resources() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(state.step_active_cast('I', "P", Path::new("")).unwrap().is_none());
        assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
        assert!(state.active_cast.is_none());
        assert!(state.active_cast_followup.is_some());
        assert!(state.message.starts_with("Direction-"));
        assert!(state.message.contains("Space passes"));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, BLINK_COST);
        assert_eq!(state.turn, 0);

        assert!(
            state
                .step_active_cast_followup('X', "", Path::new(""))
                .unwrap()
                .is_none()
        );
        assert!(state.active_cast_followup.is_some());
        assert!(state.message.starts_with("Direction-"));

        let result = state
            .step_active_cast_followup('6', "", Path::new(""))
            .unwrap()
            .expect("east direction should finish Blink");
        assert_eq!(result.0, MoveOutcome::Cast);
        assert_eq!(result.1, None);
        assert!(state.active_cast_followup.is_none());
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Blinked East to (15, 1) in BRITANNIA.");
    }

    #[test]
    fn active_cast_party_target_followup_collects_party_slot() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'M',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: HEAL_COST,
                hp: 20,
                max_hp: 20,
                level: HEAL_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 3,
                max_hp: 20,
                level: 1,
            },
        ];
        state.spell_charges[HEAL_SPELL_INDEX] = 1;

        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(state.step_active_cast('M', "", Path::new("")).unwrap().is_none());
        assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
        assert!(state.active_cast_followup.is_some());
        assert!(state.message.contains("Whom?"));
        assert_eq!(state.spell_charges[HEAL_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, HEAL_COST);
        assert_eq!(state.turn, 0);

        assert!(
            state
                .step_active_cast_followup('9', "", Path::new(""))
                .unwrap()
                .is_none()
        );
        assert!(state.active_cast_followup.is_some());

        let result = state
            .step_active_cast_followup('2', "", Path::new(""))
            .unwrap()
            .expect("party target should finish Heal");
        assert_eq!(result.0, MoveOutcome::Cast);
        assert!(state.party[1].hp > 3);
        assert_eq!(state.spell_charges[HEAL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(state.message.starts_with("Healed party member 2"));
    }

    #[test]
    fn active_cast_gate_phase_followup_collects_moon_phase() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = GATE_TRAVEL_COST;
        state.party[0].level = GATE_TRAVEL_COST;

        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(state.step_active_cast('P', "RV", Path::new("")).unwrap().is_none());
        assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
        assert!(state.active_cast_followup.is_some());
        assert!(state.message.contains("To phase?"));
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, GATE_TRAVEL_COST);
        assert_eq!(state.turn, 0);

        assert!(
            state
                .step_active_cast_followup('9', "", Path::new(""))
                .unwrap()
                .is_none()
        );
        assert!(state.active_cast_followup.is_some());

        let result = state
            .step_active_cast_followup('1', "", Path::new(""))
            .unwrap()
            .expect("phase choice should finish Gate Travel");
        assert_eq!(result.0, MoveOutcome::Blocked);
        assert!(state.active_cast_followup.is_none());
        assert_eq!(state.spell_charges[GATE_TRAVEL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Gate Travel phase 1 is not set.");
    }

    /// `magic.md §6` Step 6: "The handler prints `Mixing...`, pauses
    /// briefly, then subtracts the requested quantity from each selected
    /// raw reagent counter", and Step 7 prints the completion message only
    /// afterwards. `text-output.md` §11 owns how the two land in one
    /// transcript: "**Both lines are emitted, in the order they occur**",
    /// because "Text output is a *stream* into a windowed grid" with no
    /// current-message slot to overwrite. So the mix transcript is the
    /// pause line followed by the completion line, not the completion line
    /// alone. An earlier revision of this test omitted the pause line.
    #[test]
    fn active_mix_prompt_collects_spell_reagent_and_quantity() {
        let mut state = test_state(open_grid(), 5, 5);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 2;

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_mix.is_some());
        assert!(state.message.contains(MMIX_SPELL_PROMPT_MESSAGE));

        assert_eq!(
            handle_play_key_input(&mut state, 'I', "L", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_mix.is_some());
        assert!(state.message.contains("Mix reagents:"));
        assert!(state.message.contains("Sulfur Ash (2)"));

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains(">* 1. Sulfur Ash"));

        assert_eq!(
            handle_play_key_input(&mut state, 'M', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains(MMIX_QUANTITY_PROMPT_MESSAGE));

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_mix.is_some());
        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_mix.is_none());
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 1);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Mixing...
Mixed 1 IL charge; stock is 1.");
    }

    #[test]
    fn active_new_order_prompt_swaps_non_leader_slots() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        });
        state.party.push(PartyMember {
            slot: 2,
            class_byte: b'M',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 4,
            hp: 18,
            max_hp: 18,
            level: 2,
        });
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"MARIA\0\0\0\0"];

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_new_order.is_some());
        assert!(state.message.contains("choose first member"));

        assert_eq!(
            handle_play_key_input(&mut state, '2', "3", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_new_order.is_none());
        assert_eq!(state.party[1].slot, 2);
        assert_eq!(state.party[2].slot, 1);
        assert_eq!(&state.party_names[1], b"MARIA\0\0\0\0");
        assert_eq!(&state.party_names[2], b"IOLO\0\0\0\0\0");
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "New order: party slots 2 and 3 swapped.");
    }

    #[test]
    fn active_yell_prompt_submits_free_text_word() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_yell.is_some());
        assert!(state.message.contains("Yell what?"));

        assert_eq!(
            handle_play_key_input(&mut state, 'f', "allax", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_yell.is_none());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Yelled FALLAX"));
        assert!(state.message.contains("Nothing happens."));
        assert!(!state.message.contains("Word of Power"));
    }

    #[test]
    fn active_direction_prompt_routes_attack_and_cancel() {
        let mut state = world_state(open_world_grid(), 5, 5);

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.message, "Attack where?");

        assert_eq!(
            handle_play_key_input(&mut state, '9', "x", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_none());
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Attacked East"));

        assert_eq!(
            handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_none());
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, DIRECTION_PROMPT_LABEL_PASS);
    }

    #[test]
    fn active_direction_prompt_routes_fire_and_push() {
        let dir = debug_game_dir();

        let mut ship = world_state(open_world_grid(), 5, 5);
        ship.player.transport = TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
        };
        ship.player.facing = Direction::South;

        assert_eq!(
            handle_play_key_input(&mut ship, 'F', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(ship.active_direction_prompt.is_some());
        assert_eq!(ship.message, "Fire- which direction?");
        assert_eq!(
            handle_play_key_input(&mut ship, '4', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(ship.active_direction_prompt.is_none());
        assert_eq!(ship.turn, 1);
        assert!(ship.message.contains("BOOOM! Ship broadside fired West"));

        fs::write(dir.join(TOWN_PUSHABLE_TABLE_FILE), "CASTLE:0 0 2 1 44\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 44;
        let mut push = test_state(grid, 1, 1);
        assert_eq!(
            handle_play_key_input(&mut push, 'P', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(push.active_direction_prompt.is_some());
        assert_eq!(push.message, "Push-");
        assert_eq!(
            handle_play_key_input(&mut push, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(push.active_direction_prompt.is_none());
        assert_eq!(push.grid[32 + 3], 44);
        assert_eq!(push.turn, 1);
        assert_eq!(push.message, "Pushed!");
        assert_eq!(
            push.message_entries()
                .iter()
                .map(|entry| entry.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Push-East", "Pushed!"]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_direction_prompt_routes_get_open_and_search() {
        let dir = debug_game_dir();

        fs::write(
            dir.join(WORLD_GET_TILE_TABLE_FILE),
            "UNDERWORLD 0 0 5 55 GOLD 7\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut get = world_state(grid, 255, 0);
        get.player.facing = Direction::South;
        assert_eq!(
            handle_play_key_input(&mut get, 'G', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(get.active_direction_prompt.is_some());
        assert_eq!(get.message, "Get-");
        assert_eq!(
            handle_play_key_input(&mut get, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(get.active_direction_prompt.is_none());
        assert_eq!(get.grid[world_cell_index(0, 0)], 5);
        assert_eq!(get.gold, DEFAULT_GOLD_STOCK + 7);
        assert_eq!(get.turn, 1);
        assert_eq!(get.player.facing, Direction::South);

        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_UNLOCKED_TILE;
        let mut open = test_state(grid, 1, 1);
        open.player.facing = Direction::South;
        assert_eq!(
            handle_play_key_input(&mut open, 'O', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(open.active_direction_prompt.is_some());
        assert_eq!(open.message, "Open-");
        assert_eq!(
            handle_play_key_input(&mut open, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(open.active_direction_prompt.is_none());
        assert_eq!(open.grid[32 + 2], TOWN_DOOR_CLEARED_TILE);
        assert_eq!(open.turn, 1);
        assert_eq!(open.player.facing, Direction::South);
        assert_eq!(open.message, "Opened!");

        fs::write(
            dir.join(SECRET_DOOR_TABLE_FILE),
            "TOWN CASTLE:0 0 2 1 184\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 24;
        let mut search = test_state(grid, 1, 1);
        search.player.facing = Direction::South;
        assert_eq!(
            handle_play_key_input(&mut search, 'S', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(search.active_direction_prompt.is_some());
        assert_eq!(search.message, "Search-");
        assert_eq!(
            handle_play_key_input(&mut search, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(search.active_direction_prompt.is_none());
        assert_eq!(search.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert_eq!(search.turn, 1);
        assert_eq!(search.player.facing, Direction::South);
        assert_eq!(search.message, "Revealed secret door at (2, 1).");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_direction_prompt_routes_top_down_look_without_spending_turn() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(LOOK2_DAT_FILE),
            look2_bytes(&[(16, "east road"), (17, "south road")]),
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 16;
        grid[2 * 32 + 1] = 17;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::South;

        assert_eq!(
            handle_play_key_input(&mut state, 'L', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_some());
        assert_eq!(state.message, "Look-");

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_direction_prompt.is_none());
        assert!(state.message.contains("east road at (2, 1)"));
        assert!(!state.message.contains("south road"));
        assert_eq!(state.player.facing, Direction::South);
        assert_eq!(state.turn, 0);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_yes_no_prompt_routes_save_cancel_and_dungeon_exit() {
        let dir = debug_game_dir();
        fs::write(dir.join("SAVED.GAM"), saved_game_seed_bytes(0, 0xff, 10, 20)).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);

        assert_eq!(
            handle_play_key_input(&mut state, 'Q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_yes_no_prompt.is_some());
        assert_eq!(state.message, SAVE_PROMPT_MESSAGE);

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_yes_no_prompt.is_none());
        assert_eq!(state.message, "No.");
        assert!(!dir.join("SAVED.OOL").exists());
        assert_eq!(state.turn, 0);

        let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert_eq!(
            handle_play_key_input(&mut dungeon, 'Q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(dungeon.active_yes_no_prompt.is_some());
        assert_eq!(dungeon.message, "Exit to DOS?");
        assert_eq!(
            handle_play_key_input(&mut dungeon, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Quit
        );
        assert!(dungeon.active_yes_no_prompt.is_none());
        assert_eq!(dungeon.message, "Yes. Exiting to DOS.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_use_picker_refuses_when_no_usable_items_are_available() {
        let mut state = dungeon_state(vec![0; DUNGEON_SIDE * DUNGEON_SIDE], 0, 1, 1);

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.message, "No usable items.");
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn active_use_picker_uses_pocket_watch_and_closes() {
        let mut state = test_state(open_grid(), 5, 5);
        state.clock = GameClock::with_date(139, 1, 1, 13, 0).unwrap();
        state.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_use.is_some());
        assert!(state.message.contains("Pocket Watch"));

        assert_eq!(
            handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.message, "Pocket Watch: 1:00 P.M.");
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn shared_use_picker_escape_prints_its_published_cancel_literal() {
        let mut state = test_state(open_grid(), 5, 5);
        state.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;

        handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();

        assert!(state.active_use.is_none());
        assert_eq!(state.message, ITEM_PICKER_ESCAPE_MESSAGE);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn active_use_picker_uses_scroll_stock_row() {
        let mut state = test_state(open_grid(), 5, 5);
        state.scroll_stock[SCROLL_LIGHT_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains("Scroll LV"));

        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.scroll_stock[SCROLL_LIGHT_INDEX], 0);
        assert_eq!(state.light_spell_counter, SCROLL_LIGHT_DURATION - 1);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn active_use_picker_potion_prompts_for_target_after_consuming_stock() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party[0].hp = 4;
        state.party[0].max_hp = 25;
        state.potion_stock[POTION_YELLOW_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains("Yellow Potion"));

        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.potion_stock[POTION_YELLOW_INDEX], 0);
        assert_eq!(state.turn, 0);
        assert!(state.active_use.is_some());
        assert!(state.message.contains("choose party member"));

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_use.is_none());
        assert_eq!(state.turn, 1);
        assert!(state.party[0].hp > 4);
        assert!(state.message.contains("potion"));
        assert!(state.message.contains("party member 1"));
    }

    #[test]
    fn active_use_picker_wind_scroll_prompts_for_direction_after_consuming_stock() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains("Scroll HR"));

        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.scroll_stock[SCROLL_WIND_CHANGE_INDEX], 0);
        assert_eq!(state.turn, 0);
        assert!(state.active_use.is_some());
        assert!(state.message.contains("choose direction"));

        assert_eq!(
            handle_play_key_input(&mut state, '6', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_use.is_none());
        assert_eq!(state.wind, WindState::East);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Wind change!"));
    }

    #[test]
    fn active_use_picker_resurrection_scroll_prompts_for_target_after_consuming_stock() {
        let mut state = test_state(open_grid(), 5, 5);
        state.party[0].status = b'D';
        state.party[0].hp = 0;
        state.scroll_stock[SCROLL_RESURRECTION_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.message.contains("Scroll CIM"));

        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.scroll_stock[SCROLL_RESURRECTION_INDEX], 0);
        assert_eq!(state.turn, 0);
        assert!(state.active_use.is_some());
        assert!(state.message.contains("choose party member"));

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_use.is_none());
        assert_eq!(state.turn, 1);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 1);
        assert!(state.message.contains("Resurrection! party member 1"));
    }

    #[test]
    fn active_use_picker_lists_shadowlord_shards_and_routes_to_handler() {
        let mut state = test_state(open_grid(), 5, 5);
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_use.is_some());
        assert!(state.message.contains("Shard of Falsehood"));

        assert_eq!(
            handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
        assert_eq!(state.turn, 1);
        // `quest-graph.md §5` "Presentation order": the heading and the aloft
        // line completed by the shard's own virtue word print "before any gate
        // is evaluated", and then "only the **position** gate produces the
        // shared no-effect result". The party is nowhere near a published
        // destruction row, so that is the gate that speaks. The old single-line
        // per-gate refusal this test used to pin is one of the two divergences
        // §5 names explicitly.
        assert_eq!(
            state.message,
            "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nNo effect!"
        );
    }

    #[test]
    fn inline_use_routes_shadowlord_shard_names_to_handler() {
        let cases = [
            // `quest-graph.md §5`: each shard's prologue is completed by its
            // own virtue word - Falsehood, Hatred, or Cowardice - and the
            // off-position refusal is the one shared no-effect result.
            (
                "Falsehood",
                SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
                "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nNo effect!",
            ),
            (
                "Shard Hatred",
                SPECIAL_ITEM_SHARD_HATRED_INDEX,
                "Shard of Hatred!\nThou dost hold the evil shard aloft: Hatred!\nNo effect!",
            ),
            (
                "CW",
                SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
                "Shard of Cowardice!\nThou dost hold the evil shard aloft: Cowardice!\nNo effect!",
            ),
        ];

        for (suffix, item_index, expected_message) in cases {
            let mut state = test_state(open_grid(), 5, 5);
            state.special_items[item_index] = 1;

            assert_eq!(
                handle_play_key_input(&mut state, 'U', suffix, Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );

            assert!(state.active_use.is_none());
            assert_eq!(state.special_items[item_index], 1);
            assert_eq!(state.turn, 1);
            assert_eq!(state.message, expected_message);
        }
    }

    #[test]
    fn inline_use_prompt_and_carpet_alias_remain_unambiguous() {
        let prompt = use_prompt_message();
        assert!(!prompt.contains("UT torch"));
        assert!(!prompt.contains("UG gem"));
        assert!(prompt.contains("UC carpet"));
        assert!(prompt.contains("shard names"));

        let mut state = world_state(open_world_grid(), 5, 5);
        state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "C", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 0);
        assert!(matches!(state.player.transport, TransportState::Carpet { .. }));
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn use_shadowlord_shard_refuses_missing_and_vanquished_states_without_consuming() {
        let mut missing = test_state(open_grid(), 5, 5);
        assert_eq!(
            missing
                .use_shadowlord_shard(SHADOWLORD_HATRED_INDEX, None)
                .unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(missing.message, "No Shard of Hatred!");
        assert_eq!(missing.turn, 0);

        // The vanquished case is only actually exercised **on** the shard's
        // published destruction row: off-position the handler refuses at the
        // position gate and never reaches the Shadowlord tests at all.
        // `quest-graph.md §5` fixes Cowardice's row as Serpent's Hold,
        // basement floor `0xFF`, party at `(15, 16)`.
        //
        // A vanquished Shadowlord can never be the active named encounter,
        // because the Yell path "checks whether that Shadowlord is still alive
        // before creating the summoned encounter state". The handshake gate
        // therefore fails, and §5 phase 5 makes that failure silent: "the
        // handler simply returns. It prints no refusal line, so from the
        // player's side the sequence stops after the cast-into-the-flame line
        // with nothing further happening."
        let mut vanquished = test_state(open_grid(), 15, 16);
        vanquished.area = Area::Town {
            scene: Scene::new(SCENE_SERPENTS_HOLD).unwrap(),
            floor: -1,
        };
        vanquished.special_items[SPECIAL_ITEM_SHARD_COWARDICE_INDEX] = 1;
        vanquished.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX] = SHADOWLORD_VANQUISHED;
        assert_eq!(
            vanquished
                .use_shadowlord_shard(SHADOWLORD_COWARDICE_INDEX, None)
                .unwrap(),
            MoveOutcome::Blocked
        );
        // §5: "a refused attempt ... leaves the shard in the party's
        // possession."
        assert_eq!(vanquished.special_items[SPECIAL_ITEM_SHARD_COWARDICE_INDEX], 1);
        assert_eq!(
            vanquished.message,
            "Shard of Cowardice!\nThou dost hold the evil shard aloft: Cowardice!\nThou dost cast it into the Flame of Courage!"
        );
        assert_eq!(vanquished.turn, 0);
    }

    #[test]
    fn use_shadowlord_shard_with_matching_flame_vanquishes_and_consumes() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 15, 9);
        state.area = Area::Town {
            scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
            floor: 2,
        };
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
        state.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
        let z = state.current_floor().unwrap();
        state.active_objects.push(
            state
                .shadowlord_name_encounter_object(SHADOWLORD_FALSEHOOD_INDEX, 15, 8, z)
                .unwrap(),
        );

        assert_eq!(
            state
                .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
                .unwrap(),
            MoveOutcome::Used
        );

        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 0);
        assert!(state.shadowlord_vanquished(SHADOWLORD_FALSEHOOD_INDEX));
        assert_eq!(
            state.removed_town_npc_flags.get(&STONEGATE_SCENE_BYTE),
            Some(&(1 << SHADOWLORD_FALSEHOOD_STONEGATE_NPC_SLOT))
        );
        assert!(!state.shadowlord_name_encounter_present(SHADOWLORD_FALSEHOOD_INDEX));
        assert_eq!(state.turn, 1);
        assert_eq!(
            state.message,
            "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nThou dost cast it into the Flame of Truth!\nFAULINEI is vanquished! Cleared 1 encounter(s)."
        );
    }

    #[test]
    fn published_shadowlord_shard_flames_cover_all_three_issue_31_positions() {
        let dir = debug_game_dir();
        let cases = [
            (
                SHADOWLORD_FALSEHOOD_INDEX,
                SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
                SCENE_THE_LYCAEUM,
                2,
                15,
                9,
                EternalFlame::Truth,
                "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nThou dost cast it into the Flame of Truth!\nFAULINEI is vanquished! Cleared 1 encounter(s).",
            ),
            (
                SHADOWLORD_HATRED_INDEX,
                SPECIAL_ITEM_SHARD_HATRED_INDEX,
                SCENE_EMPATH_ABBEY,
                1,
                15,
                3,
                EternalFlame::Love,
                "Shard of Hatred!\nThou dost hold the evil shard aloft: Hatred!\nThou dost cast it into the Flame of Love!\nASTAROTH is vanquished! Cleared 1 encounter(s).",
            ),
            (
                SHADOWLORD_COWARDICE_INDEX,
                SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
                SCENE_SERPENTS_HOLD,
                -1,
                15,
                16,
                EternalFlame::Courage,
                "Shard of Cowardice!\nThou dost hold the evil shard aloft: Cowardice!\nThou dost cast it into the Flame of Courage!\nNOSFENTOR is vanquished! Cleared 1 encounter(s).",
            ),
        ];

        for (shadowlord, item, scene, floor, x, y, flame, message) in cases {
            let mut state = test_state(open_grid(), x, y);
            state.area = Area::Town {
                scene: Scene::new(scene).unwrap(),
                floor,
            };
            state.special_items[item] = 1;
            state.shadowlord_hideouts[shadowlord] = 1;
            assert_eq!(
                state
                    .published_eternal_flame_at_current_position()
                    .map(|entry| entry.flame),
                Some(flame),
                "published flame at scene {scene} floor {floor} ({x}, {y})"
            );

            state.summoned_shadowlord = Some(shadowlord);
            let z = state.current_floor().unwrap();
            state.active_objects.push(
                state
                    .shadowlord_name_encounter_object(shadowlord, x, y - 1, z)
                    .unwrap(),
            );

            assert_eq!(
                state.use_shadowlord_shard(shadowlord, Some(&dir)).unwrap(),
                MoveOutcome::Used,
                "shard {shadowlord} should succeed at its published flame"
            );
            assert_eq!(state.special_items[item], 0);
            assert!(state.shadowlord_vanquished(shadowlord));
            assert_eq!(state.turn, 1);
            // `quest-graph.md §5` "Presentation order" on a successful
            // destruction: the heading plus the aloft line completed by the
            // shard's own virtue word (phase 1), the cast-into-the-flame line
            // completed by the **opposed** principle's word - Truth, Love or
            // Courage (phase 4), and a closing line "naming the destroyed
            // Shadowlord" (phase 6). The three shard rows differ in exactly
            // those three words and in nothing else.
            assert_eq!(state.message, message);
        }
    }

    #[test]
    fn published_shadowlord_shard_flames_require_exact_scene_floor_and_position() {
        let cases = [
            (SCENE_THE_LYCAEUM, 2, 15, 9),
            (SCENE_EMPATH_ABBEY, 1, 15, 3),
            (SCENE_SERPENTS_HOLD, -1, 15, 16),
        ];

        for (scene, floor, x, y) in cases {
            let mut exact = test_state(open_grid(), x, y);
            exact.area = Area::Town {
                scene: Scene::new(scene).unwrap(),
                floor,
            };
            assert!(
                exact.published_eternal_flame_at_current_position().is_some(),
                "sanity: exact flame exists for scene {scene} floor {floor}"
            );

            let mut wrong_x = exact.clone();
            wrong_x.player.x = x.wrapping_sub(1);
            assert!(wrong_x.published_eternal_flame_at_current_position().is_none());

            let mut wrong_y = exact.clone();
            wrong_y.player.y = y.wrapping_sub(1);
            assert!(wrong_y.published_eternal_flame_at_current_position().is_none());

            let mut wrong_floor = exact.clone();
            wrong_floor.area = Area::Town {
                scene: Scene::new(scene).unwrap(),
                floor: floor + 1,
            };
            assert!(wrong_floor.published_eternal_flame_at_current_position().is_none());

            let mut wrong_scene = exact;
            wrong_scene.area = Area::Town {
                scene: Scene::new(SCENE_LORD_BRITISHS_CASTLE).unwrap(),
                floor,
            };
            assert!(wrong_scene.published_eternal_flame_at_current_position().is_none());
        }
    }

    #[test]
    fn use_shadowlord_shard_rejects_cardinal_adjacent_flame_tile() {
        let dir = debug_game_dir();
        fs::write(dir.join(ETERNAL_FLAME_TABLE_FILE), "CASTLE:0 0 5 5 TRUTH\n").unwrap();
        let mut grid = open_grid();
        grid[5 * 32 + 5] = 0x76;
        let mut state = test_state(grid, 5, 4);
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
        state.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
        let z = state.current_floor().unwrap();
        state.active_objects.push(
            state
                .shadowlord_name_encounter_object(SHADOWLORD_FALSEHOOD_INDEX, 5, 3, z)
                .unwrap(),
        );

        assert_eq!(
            state
                .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
        assert!(state.shadowlord_alive(SHADOWLORD_FALSEHOOD_INDEX));
        // `quest-graph.md §5`: the position gate requires the party's "X, Y,
        // scene, and floor" to equal the shard's row, and it is the only gate
        // that "produces the shared no-effect result" - after the
        // unconditional heading and aloft line.
        assert_eq!(
            state.message,
            "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nNo effect!"
        );
    }

    #[test]
    fn use_shadowlord_shard_matching_flame_requires_live_encounter_north() {
        let dir = debug_game_dir();
        fs::write(dir.join(ETERNAL_FLAME_TABLE_FILE), "CASTLE:0 0 5 5 TRUTH\n").unwrap();
        let mut grid = open_grid();
        grid[5 * 32 + 5] = 0x76;
        let mut state = test_state(grid, 5, 5);
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;

        assert_eq!(
            state
                .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
        assert!(!state.shadowlord_vanquished(SHADOWLORD_FALSEHOOD_INDEX));
        assert_eq!(state.turn, 0);
        // `quest-graph.md §5` phase 4: the cast-into-the-flame line, completed
        // by the opposed principle's word, prints "**before** testing whether
        // a Shadowlord is on the flame and whether the handshake matches".
        // Phase 5: "If either of those two gates fails, the handler simply
        // returns. It prints no refusal line." Printing one is the second
        // divergence §5 names explicitly.
        assert_eq!(
            state.message,
            "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nThou dost cast it into the Flame of Truth!"
        );
    }

    #[test]
    fn use_shadowlord_shard_rejects_diagonal_flame_tile() {
        let dir = debug_game_dir();
        fs::write(dir.join(ETERNAL_FLAME_TABLE_FILE), "CASTLE:0 0 5 5 TRUTH\n").unwrap();
        let mut grid = open_grid();
        grid[5 * 32 + 5] = 0x76;
        let mut state = test_state(grid, 4, 4);
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
        state.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
        let z = state.current_floor().unwrap();
        state.active_objects.push(
            state
                .shadowlord_name_encounter_object(SHADOWLORD_FALSEHOOD_INDEX, 5, 4, z)
                .unwrap(),
        );

        assert_eq!(
            state
                .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
        assert!(state.shadowlord_alive(SHADOWLORD_FALSEHOOD_INDEX));
        // `quest-graph.md §5`: the position gate requires the party's "X, Y,
        // scene, and floor" to equal the shard's row, and it is the only gate
        // that "produces the shared no-effect result" - after the
        // unconditional heading and aloft line.
        assert_eq!(
            state.message,
            "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nNo effect!"
        );
    }

    #[test]
    fn use_shadowlord_shard_rejects_wrong_flame_without_consuming() {
        let dir = debug_game_dir();
        fs::write(dir.join(ETERNAL_FLAME_TABLE_FILE), "CASTLE:0 0 5 5 LOVE 16\n").unwrap();
        let mut state = test_state(open_grid(), 5, 5);
        state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
        state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
        state.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
        let z = state.current_floor().unwrap();
        state.active_objects.push(
            state
                .shadowlord_name_encounter_object(SHADOWLORD_FALSEHOOD_INDEX, 6, 5, z)
                .unwrap(),
        );

        assert_eq!(
            state
                .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
        assert!(state.shadowlord_alive(SHADOWLORD_FALSEHOOD_INDEX));
        assert!(state.shadowlord_name_encounter_present(SHADOWLORD_FALSEHOOD_INDEX));
        assert_eq!(state.turn, 0);
        // `quest-graph.md §5`: the position gate is per-shard - "The party's
        // X, Y, scene, and floor must all equal the row for the shard's index"
        // - so standing on a Flame of Love with the Shard of Falsehood is a
        // position mismatch, not a distinct mismatched-flame refusal. §5
        // publishes exactly one refusal result here.
        assert_eq!(
            state.message,
            "Shard of Falsehood!\nThou dost hold the evil shard aloft: Falsehood!\nNo effect!"
        );
    }

    #[test]
    fn inline_use_suffix_rejects_torch_and_gem_aliases() {
        let mut state = test_state(open_grid(), 5, 5);
        state.torches = 1;
        state.gems = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "T", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.torches, 1);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, use_prompt_message());

        assert_eq!(
            handle_play_key_input(&mut state, 'U', "G", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_use.is_none());
        assert_eq!(state.gems, 1);
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, use_prompt_message());
    }

    #[test]
    fn combat_z_stats_binds_pending_party_actor() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'M',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 2,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"MARIA\0\0\0\0"];
        state.combat_active = true;
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            32,
            1,
            0,
            5,
            5,
        ]);
        state.pending_combat_actor_slot = Some(1);

        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.turn, 0);
        assert_eq!(
            state
                .active_z_stats
                .as_ref()
                .map(|session| (session.selected_party_index, session.page)),
            Some((1, ZStatsPage::Stats))
        );
        assert!(state.message.contains("MARIA"));
    }

    #[test]
    fn shadowlord_midnight_reroll_skips_vanquished_and_samples_living_slots() {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.clock = GameClock::with_date(139, 4, 5, 23, 59).unwrap();
        state.shadowlord_hideouts = [1, SHADOWLORD_VANQUISHED, 2];
        state.prng_state = 0x1234;
        let (expected_hideouts, expected_prng_state) = expected_shadowlord_prng_reroll(
            state.shadowlord_hideouts,
            state.prng_state,
            state.current_shadowlord_hideout_id(),
        );

        state.advance_turn_with_minutes(1);

        assert_eq!(state.clock.day, 6);
        assert_eq!(state.shadowlord_hideouts, expected_hideouts);
        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(
            state.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX],
            SHADOWLORD_VANQUISHED
        );
        assert!(PlayState::shadowlord_slot_is_living(
            state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX]
        ));
        assert!(PlayState::shadowlord_slot_is_living(
            state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX]
        ));
    }

    // `shadowlord_reroll_uses_plain_uniform_slots_without_current_rejection`
    // was removed here. It asserted that the midnight walker draws plainly and
    // uniformly with no reject-and-redraw, which `time.md §7` contradicts
    // directly: the pass "rejects it when either of these holds, then draws
    // again", one of the two conditions being "the candidate equals the
    // party's current scene byte". Nothing in `time.md` or
    // `catalogs/quest-graph.md` retracts that rule. The published contract is
    // pinned by `shadowlord_reroll_rejects_the_party_scene_and_redraws` in
    // `tests_inline/recovered_chunk_17.rs`.

    #[test]
    fn shadowlord_current_hideout_id_uses_virtue_town_scene_ids_only() {
        let mut state = test_state(open_grid(), 1, 1);

        state.area = Area::Town {
            scene: Scene::new(1).unwrap(),
            floor: 0,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), Some(1));

        state.area = Area::Town {
            scene: Scene::new(8).unwrap(),
            floor: 0,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), Some(8));

        state.area = Area::Town {
            scene: Scene::new(9).unwrap(),
            floor: 0,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), None);

        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        assert_eq!(state.current_shadowlord_hideout_id(), None);
    }

    /// `time.md §7`: "For each slot whose high bit is clear, the midnight
    /// pass draws a candidate id uniformly from `1..8` inclusive and rejects
    /// it when either of these holds, then draws again: the candidate equals
    /// the party's current scene byte, or the candidate equals the value
    /// currently stored in **any** of the three slots, including the slot
    /// being rerolled and any slot already rewritten earlier in the same
    /// pass."
    ///
    /// The rejection set is read live from the running array rather than from
    /// a pre-pass snapshot, which is what makes §7's two stated consequences
    /// true: "a living Shadowlord never stays in the same town two days
    /// running, and no two living Shadowlords share a town". Because each
    /// rejection consumes a draw, a model without the rejection also predicts
    /// the wrong final PRNG state, not merely the wrong ids.
    ///
    /// The walker's own gate is the high bit, not liveness: §7 says a `0`
    /// slot means "not yet placed" and "the reroll walker rewrites it on the
    /// first day rollover".
    fn expected_shadowlord_prng_reroll(
        previous: [u8; SHADOWLORD_COUNT],
        mut prng_state: u16,
        current: Option<u8>,
    ) -> ([u8; SHADOWLORD_COUNT], u16) {
        let mut hideouts = previous;
        for slot in 0..SHADOWLORD_COUNT {
            if !PlayState::shadowlord_slot_is_rerollable(hideouts[slot]) {
                continue;
            }
            loop {
                let candidate = u5_prng_range_u16(
                    &mut prng_state,
                    u16::from(SHADOWLORD_HIDEOUT_MIN),
                    u16::from(SHADOWLORD_HIDEOUT_MAX),
                ) as u8;
                if Some(candidate) == current || hideouts.contains(&candidate) {
                    continue;
                }
                hideouts[slot] = candidate;
                break;
            }
        }
        (hideouts, prng_state)
    }

    #[test]
    fn shadowlord_midnight_reroll_samples_each_living_slot_uniformly() {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(1).unwrap(),
            floor: 0,
        };
        state.clock = GameClock::with_date(139, 4, 5, 23, 59).unwrap();
        state.shadowlord_hideouts = [1, 2, SHADOWLORD_VANQUISHED];
        state.prng_state = 0x3456;
        let (expected_hideouts, expected_prng_state) = expected_shadowlord_prng_reroll(
            state.shadowlord_hideouts,
            state.prng_state,
            state.current_shadowlord_hideout_id(),
        );

        state.advance_turn_with_minutes(1);

        assert_eq!(state.shadowlord_hideouts, expected_hideouts);
        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(
            state.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX],
            SHADOWLORD_VANQUISHED
        );
    }

    #[test]
    fn new_order_swaps_nonleader_party_positions_and_consumes_turn() {
        // commands.md §6: a successful nonzero-slot swap exchanges the whole
        // roster records and marks the turn as consumed.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'P',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'S',
                climb_stat: 30,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 3,
            },
        ];

        assert_eq!(
            handle_play_key_input(&mut state, 'N', "23", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state
                .party
                .iter()
                .map(|member| member.slot)
                .collect::<Vec<_>>(),
            vec![0, 2, 1]
        );
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "New order: party slots 2 and 3 swapped.");
    }

    #[test]
    fn new_order_refuses_swaps_involving_leader_slot_one() {
        // commands.md §6: if either selected slot is slot zero (one-based
        // slot 1), the command refuses without consuming a turn — the leader
        // must remain first.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'G',
                climb_stat: 30,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 3,
            },
        ];
        let original = state.party.clone();

        assert_eq!(state.new_order_from_suffix("12"), MoveOutcome::Blocked);
        assert_eq!(state.message, "The leader must remain first.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);

        assert_eq!(state.new_order_from_suffix("31"), MoveOutcome::Blocked);
        assert_eq!(state.message, "The leader must remain first.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn new_order_same_nonzero_slot_consumes_turn_as_noop() {
        // commands.md §6: picking the same nonzero slot twice is accepted as
        // a behavioural no-op, but the turn is still consumed.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
        ];
        let original = state.party.clone();

        assert_eq!(state.new_order_from_suffix("22"), MoveOutcome::Used);
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "New order: party slot 2 unchanged.");
    }

    #[test]
    fn new_order_refusals_preserve_party_without_turn() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
        ];
        let original = state.party.clone();

        assert_eq!(state.new_order_from_suffix(""), MoveOutcome::PromptDeclined);
        assert_eq!(
            state.message,
            "New order? Use N12 to swap party slots 1 and 2."
        );
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);

        assert_eq!(state.new_order_from_suffix("11"), MoveOutcome::Blocked);
        assert_eq!(state.message, "The leader must remain first.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);

        assert_eq!(state.new_order_from_suffix("13"), MoveOutcome::Blocked);
        assert_eq!(state.message, "Party has 2 members.");
        assert_eq!(state.party, original);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn new_order_changes_caster_position_for_inline_cast() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
        ];
        state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
        state.ambient_light = FULL_DARKNESS;

        // commands.md §6: swapping nonleader slots 2 and 3 consumes one turn.
        assert_eq!(
            handle_play_key_input(&mut state, 'N', "23", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.party[1].slot, 2);
        assert_eq!(state.turn, 1);

        // Casting from one-based slot 2 should now spend the mana that was
        // originally on slot 2 but moved to runtime position 1.
        assert_eq!(
            handle_play_key_input(&mut state, 'C', "2LI", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[1].slot, 2);
        assert_eq!(state.party[1].mana, 0);
        assert_eq!(state.party[2].slot, 1);
        assert_eq!(state.party[2].mana, 0);
        assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
        assert_eq!(state.turn, 2);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn new_order_swaps_strength_and_equipment_sidecars() {
        // commands.md §6: New Order swaps the whole roster records (names,
        // stats, equipment, counters) and consumes a turn. Use slots 2 and 3
        // because slot 0 is the leader, which the spec refuses to move.
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 5,
                mana: 0,
                hp: 9,
                max_hp: 19,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: 10,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'G',
                climb_stat: 20,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"DUPRE\0\0\0\0"];
        state.party_strengths = vec![10, 12, 34];
        state.party_intelligence = vec![20, 21, 22];
        state.party_experience = vec![100, 200, 300];
        state.party_equipment = vec![
            [
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
            ],
            [
                1,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
            ],
            [
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                16,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
                EQUIPMENT_EMPTY,
            ],
        ];

        assert_eq!(state.new_order_from_suffix("23"), MoveOutcome::Used);

        assert_eq!(
            state.party_names,
            vec![*b"AVATAR\0\0\0", *b"DUPRE\0\0\0\0", *b"IOLO\0\0\0\0\0"]
        );
        assert_eq!(state.party_strengths, vec![10, 34, 12]);
        assert_eq!(state.party_intelligence, vec![20, 22, 21]);
        assert_eq!(state.party_experience, vec![100, 300, 200]);
        assert_eq!(state.party_equipment[1][EQUIP_SLOT_WEAPON], 16);
        assert_eq!(state.party_equipment[2][EQUIP_SLOT_HELM], 1);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn new_order_save_export_writes_active_roster_order() {
        // saved-gam.md section 3: the travelling order is the order of the first
        // party-size roster records. After New Order, save export must write
        // the active party positions, not each member's original slot id.
        let dir = debug_game_dir();
        fs::write(dir.join("SAVED.GAM"), saved_game_seed_bytes(0, 0xff, 10, 20)).unwrap();
        fs::write(dir.join("SAVED.OOL"), vec![0; SAVED_OOL_LEN]).unwrap();
        write_empty_ool_mirrors(&dir);
        let mut state = world_state(open_world_grid(), 10, 20);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: 5,
                mana: 1,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'P',
                climb_stat: 10,
                mana: 2,
                hp: 11,
                max_hp: 21,
                level: 2,
            },
            PartyMember {
                slot: 2,
                class_byte: b'F',
                status: b'S',
                climb_stat: 20,
                mana: 3,
                hp: 12,
                max_hp: 22,
                level: 3,
            },
        ];
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"DUPRE\0\0\0\0"];
        state.party_strengths = vec![30, 12, 34];
        state.party_intelligence = vec![30, 13, 35];
        state.party_experience = vec![100, 200, 300];
        state.party_stay_counters = vec![1, 2, 3];
        state.party_equipment = default_party_equipment(3);
        state.party_equipment[1][EQUIP_SLOT_HELM] = 1;
        state.party_equipment[2][EQUIP_SLOT_WEAPON] = 16;

        assert_eq!(state.new_order_from_suffix("23"), MoveOutcome::Used);
        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        let second = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN;
        let third = second + SAVE_CHARACTER_RECORD_LEN;
        assert_eq!(&saved[second..second + SAVE_CHARACTER_NAME_LEN], b"DUPRE\0\0\0\0");
        assert_eq!(saved[second + SAVE_CHARACTER_CLASS_OFFSET], b'F');
        assert_eq!(saved[second + SAVE_CHARACTER_STATUS_OFFSET], b'S');
        assert_eq!(saved[second + SAVE_CHARACTER_STR_OFFSET], 34);
        assert_eq!(saved[second + SAVE_CHARACTER_DEX_OFFSET], 20);
        assert_eq!(saved[second + SAVE_CHARACTER_INT_OFFSET], 35);
        assert_eq!(saved[second + SAVE_CHARACTER_MANA_OFFSET], 3);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_HP_OFFSET), 12);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_MAX_HP_OFFSET), 22);
        assert_eq!(u16_at(&saved, second + SAVE_CHARACTER_EXPERIENCE_OFFSET), 300);
        assert_eq!(saved[second + SAVE_CHARACTER_STAY_COUNTER_OFFSET], 3);
        assert_eq!(saved[second + SAVE_CHARACTER_LEVEL_OFFSET], 3);
        assert_eq!(saved[second + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIP_SLOT_WEAPON], 16);

        assert_eq!(&saved[third..third + SAVE_CHARACTER_NAME_LEN], b"IOLO\0\0\0\0\0");
        assert_eq!(saved[third + SAVE_CHARACTER_CLASS_OFFSET], b'B');
        assert_eq!(saved[third + SAVE_CHARACTER_STATUS_OFFSET], b'P');
        assert_eq!(saved[third + SAVE_CHARACTER_STR_OFFSET], 12);
        assert_eq!(saved[third + SAVE_CHARACTER_DEX_OFFSET], 10);
        assert_eq!(saved[third + SAVE_CHARACTER_INT_OFFSET], 13);
        assert_eq!(saved[third + SAVE_CHARACTER_MANA_OFFSET], 2);
        // `time.md` §5, unconditional part: "A member whose status is
        // exactly Poisoned loses **exactly 1 current hit point** ... This is
        // per member per turn, independently, not a shared roll and not an
        // hourly effect." New Order consumes a turn, so this Poisoned
        // record's seeded 11 hit points are exported as 10. The Sleeping
        // record above is "skipped entirely" and keeps its 12. An earlier
        // revision of this test expected 11 here, from before the per-turn
        // poison tick existed.
        assert_eq!(u16_at(&saved, third + SAVE_CHARACTER_HP_OFFSET), 10);
        assert_eq!(u16_at(&saved, third + SAVE_CHARACTER_MAX_HP_OFFSET), 21);
        assert_eq!(u16_at(&saved, third + SAVE_CHARACTER_EXPERIENCE_OFFSET), 200);
        assert_eq!(saved[third + SAVE_CHARACTER_STAY_COUNTER_OFFSET], 2);
        assert_eq!(saved[third + SAVE_CHARACTER_LEVEL_OFFSET], 2);
        assert_eq!(saved[third + SAVE_CHARACTER_EQUIPMENT_OFFSET + EQUIP_SLOT_HELM], 1);
        let _ = fs::remove_dir_all(dir);
    }

    /// `inventory.md §5`/§8: R-Ready costs a turn in every mode, and a
    /// refusal costs exactly what a success costs. An earlier revision of
    /// this suite asserted the equip and the unequip were both free; §8
    /// was re-challenged and re-derived from the shipped binaries and
    /// stands as published, so that assertion is withdrawn.
    #[test]
    fn ready_equipment_equips_and_unequips_charging_one_turn_each() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![50];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[EQUIPMENT_ID_ARROWS] = 5;
        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'R', "1/26", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.party_equipment[0][EQUIP_SLOT_WEAPON],
            EQUIPMENT_ID_BOW as u8
        );
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 0);
        assert_eq!(state.turn, 1);

        assert_eq!(state.ready_equipment_from_suffix("1/26"), MoveOutcome::Used);

        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 1);
        assert_eq!(state.turn, 2);
    }

    /// `inventory.md §8`: every refusal costs what a success costs. The
    /// silently refused ammunition row is the interesting one — §9 records
    /// that the `R` arm marks the actor as having acted at entry, three
    /// call levels above the ammunition test, so that early exit cannot
    /// reach the charge. The refusal stays silent; only the turn moves.
    #[test]
    fn ready_equipment_charges_a_turn_on_every_refusal() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![50];
        state.party_equipment = default_party_equipment(1);

        // Silent ammunition-row refusal: no message written, turn spent.
        state.message = "sentinel".to_string();
        assert_eq!(
            state.ready_equipment_from_suffix(&format!("1/{EQUIPMENT_ID_ARROWS}")),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "sentinel");
        assert_eq!(state.turn, 1);

        // Empty-handed refusal.
        assert_eq!(
            state.ready_equipment_from_suffix(&format!("1/{EQUIPMENT_ID_BOW}")),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, format!("No carried {} to ready.", equipment_name(EQUIPMENT_ID_BOW)));
        assert_eq!(state.turn, 2);

        // Missing-ammunition refusal for a carried ranged weapon.
        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix(&format!("1/{EQUIPMENT_ID_BOW}")),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "No arrows for that weapon.");
        assert_eq!(state.turn, 3);
    }

    /// `inventory.md §5`/§8 establishes the sole mode-loop charge and that
    /// the picker stays open across repeated attempts. Public issue #113
    /// confirms that nothing in the subtree adds a per-attempt clock advance.
    #[test]
    fn active_ready_picker_applies_exactly_one_mode_loop_charge() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![50];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        state.equipment_stock[EQUIPMENT_ID_ARROWS] = 5;

        assert_eq!(
            handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_ready.is_some());
        assert!(state.message.contains("choose party member"));
        // Opening the picker is the invocation, and it is what costs the
        // turn: `inventory.md §8` charges even for opening it and
        // immediately backing out.
        assert_eq!(state.turn, 1);

        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
        assert!(state.message.contains("26: Bow"));

        // inventory.md §5: Space and Enter are separate accepted confirm
        // bytes. Use Space for the first attempt and Enter for the second.
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
        assert_eq!(
            state.party_equipment[0][EQUIP_SLOT_WEAPON],
            EQUIPMENT_ID_BOW as u8
        );
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 0);
        assert_eq!(state.turn, 1);
        assert!(state.active_ready.is_some());
        assert!(state.message.contains("Readied Bow"));
        assert!(state.message.contains("26: Bow"));

        handle_play_key_input(&mut state, '\n', "", Path::new("")).unwrap();
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 1);
        assert_eq!(state.turn, 1);
        assert!(state.message.contains("Unequipped Bow"));

        handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
        assert!(state.active_ready.is_none());
        assert_eq!(state.message, READY_PICKER_ESCAPE_MESSAGE);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn ready_invocation_uses_exact_mode_clock_cost_and_shared_time_modifiers() {
        let dungeon = Area::Dungeon {
            scene: DungeonScene::new(FIRST_DUNGEON_SCENE_BYTE).unwrap(),
            level: 0,
        };
        let cases = [
            ("world normal", Area::World { plane: WorldPlane::Britannia }, None, 2),
            ("town normal", test_state(open_grid(), 1, 1).area, None, 1),
            ("dungeon normal", dungeon, None, 1),
            (
                "world Quickness",
                Area::World { plane: WorldPlane::Britannia },
                Some((QUICKNESS_ACTIVE_EFFECT_TAG, QUICKNESS_ACTIVE_EFFECT_DURATION)),
                1,
            ),
            (
                "town Quickness",
                test_state(open_grid(), 1, 1).area,
                Some((QUICKNESS_ACTIVE_EFFECT_TAG, QUICKNESS_ACTIVE_EFFECT_DURATION)),
                1,
            ),
            (
                "dungeon Quickness",
                dungeon,
                Some((QUICKNESS_ACTIVE_EFFECT_TAG, QUICKNESS_ACTIVE_EFFECT_DURATION)),
                1,
            ),
            (
                "world Negate Time",
                Area::World { plane: WorldPlane::Britannia },
                Some((NEGATE_TIME_ACTIVE_EFFECT_TAG, TIME_STOP_DURATION)),
                0,
            ),
            (
                "town Negate Time",
                test_state(open_grid(), 1, 1).area,
                Some((NEGATE_TIME_ACTIVE_EFFECT_TAG, TIME_STOP_DURATION)),
                0,
            ),
            (
                "dungeon Negate Time",
                dungeon,
                Some((NEGATE_TIME_ACTIVE_EFFECT_TAG, TIME_STOP_DURATION)),
                0,
            ),
        ];

        for (label, area, effect, expected_minutes) in cases {
            let mut state = match area {
                Area::World { .. } => world_state(open_world_grid(), 1, 1),
                Area::Town { .. } | Area::Dungeon { .. } => test_state(open_grid(), 1, 1),
            };
            state.area = area;
            state.clock = GameClock::new(12, 0).unwrap();
            state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
            if let Some((tag, counter)) = effect {
                state.active_effect_tag = Some(tag);
                state.active_effect_counter = counter;
            }

            assert_eq!(
                handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap(),
                PlayInputDisposition::Continue,
                "{label}"
            );

            assert!(state.active_ready.is_some(), "{label}");
            assert_eq!(state.turn, 1, "{label}");
            assert_eq!((state.clock.hour, state.clock.minute), (12, expected_minutes), "{label}");
        }
    }

    #[test]
    fn ready_member_selection_cancel_and_item_picker_escape_use_distinct_literals() {
        let mut member_cancel = test_state(open_grid(), 1, 1);
        member_cancel.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        handle_play_key_input(&mut member_cancel, 'R', "", Path::new("")).unwrap();
        handle_play_key_input(&mut member_cancel, '\u{1b}', "", Path::new("")).unwrap();
        assert!(member_cancel.active_ready.is_none());
        assert_eq!(member_cancel.message, ITEM_PICKER_ESCAPE_MESSAGE);

        let mut picker_exit = test_state(open_grid(), 1, 1);
        picker_exit.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        handle_play_key_input(&mut picker_exit, 'R', "", Path::new("")).unwrap();
        handle_play_key_input(&mut picker_exit, '1', "", Path::new("")).unwrap();
        handle_play_key_input(&mut picker_exit, '\u{1b}', "", Path::new("")).unwrap();
        assert!(picker_exit.active_ready.is_none());
        assert_eq!(picker_exit.message, READY_PICKER_ESCAPE_MESSAGE);
    }

    #[test]
    fn ready_member_selector_cycles_confirms_highlight_and_retries_zero() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 10,
            max_hp: 10,
            level: 1,
        });
        state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0"];
        state.party_equipment = default_party_equipment(2);
        state.party_strengths = vec![50; 2];
        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;

        handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '0', "", Path::new("")).unwrap();
        assert_eq!(
            state.active_ready.as_ref().unwrap().selected_party_index,
            None
        );

        handle_play_key_input(
            &mut state,
            char::from(INPUT_CODE_SOUTH),
            "",
            Path::new(""),
        )
        .unwrap();
        assert_eq!(state.active_ready.as_ref().unwrap().cursor, 1);

        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
        assert_eq!(
            state.active_ready.as_ref().unwrap().selected_party_index,
            Some(1)
        );
        assert!(state.message.contains("party member 2"));
    }

    #[test]
    fn ready_and_use_picker_actions_accept_native_vertical_and_corner_codes() {
        for (code, ready, use_action) in [
            (
                INPUT_CODE_NORTH,
                ReadyInputAction::PreviousItem,
                UseInputAction::PreviousItem,
            ),
            (
                INPUT_CODE_SOUTH,
                ReadyInputAction::NextItem,
                UseInputAction::NextItem,
            ),
            (
                INPUT_CODE_NORTHWEST,
                ReadyInputAction::PagePrevious,
                UseInputAction::PagePrevious,
            ),
            (
                INPUT_CODE_NORTHEAST,
                ReadyInputAction::PagePrevious,
                UseInputAction::PagePrevious,
            ),
            (
                INPUT_CODE_SOUTHWEST,
                ReadyInputAction::PageNext,
                UseInputAction::PageNext,
            ),
            (
                INPUT_CODE_SOUTHEAST,
                ReadyInputAction::PageNext,
                UseInputAction::PageNext,
            ),
        ] {
            let key = char::from(code);
            assert_eq!(ready_input_action(key), ready);
            assert_eq!(use_input_action(key), use_action);
        }
        assert_eq!(ready_input_action(' '), ReadyInputAction::Confirm);
        assert_eq!(use_input_action(' '), UseInputAction::Confirm);
        assert_eq!(ready_input_action('\u{1b}'), ReadyInputAction::Exit);
        assert_eq!(use_input_action('\u{1b}'), UseInputAction::Exit);
    }

    #[test]
    fn ready_magic_ring_vanish_closes_picker_without_done_literal() {
        let mut state = test_state(open_grid(), 1, 1);
        state.turn = 5;
        state.party_strengths = vec![50];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[EQUIPMENT_ID_RING_INVISIBILITY] = 1;

        handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();

        assert!(state.active_ready.is_none());
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_RING_INVISIBILITY], 0);
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
        assert!(state.message.contains("vanished"));
        assert!(!state.message.contains(READY_PICKER_ESCAPE_MESSAGE));
        assert_eq!(state.turn, 6);
    }

    #[test]
    fn active_ready_picker_rejects_invalid_party_without_closing() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![50];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[16] = 1;

        handle_play_key_input(&mut state, 'R', "", Path::new("")).unwrap();
        handle_play_key_input(&mut state, '2', "", Path::new("")).unwrap();

        assert!(state.active_ready.is_some());
        assert!(state.message.contains("Party has 1 member"));

        handle_play_key_input(&mut state, '1', "", Path::new("")).unwrap();
        assert!(state.message.contains("16: Dagger"));
    }

    #[test]
    fn ready_equipment_unequipping_invisibility_ring_clears_combat_hidden_flag() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_RING] = EQUIPMENT_ID_RING_INVISIBILITY as u8;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            4,
        ]);
        state.active_objects[0].type_byte = 0x5c;
        state.active_objects[0].tile = 0x5d;
        assert!(apply_combat_linked_invisibility(
            &mut state.combat_actors[0],
            &mut state.active_objects,
        )
        .unwrap()
        .changed());
        state.visibility_dirty = false;

        assert_eq!(
            state.ready_equipment_from_suffix("1/42"),
            MoveOutcome::Used
        );

        assert_eq!(state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_RING_INVISIBILITY], 1);
        assert!(!state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.active_objects[0].tile, 0x5c);
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Unequipped Ring of Invisibility from party member 1; stock is 1."
        );
    }

    #[test]
    fn ready_equipment_enforces_stock_ammunition_and_strength_gates() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![5];
        state.party_equipment = default_party_equipment(1);

        assert_eq!(
            state.ready_equipment_from_suffix("1/16"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "No carried Dagger to ready.");

        state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix("1/26"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "No arrows for that weapon.");

        state.equipment_stock[EQUIPMENT_ID_ARROWS] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix("1/26"),
            MoveOutcome::Blocked
        );
        assert!(state.message.contains("not strong enough"));
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], EQUIPMENT_EMPTY);
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_BOW], 1);
    }

    #[test]
    fn ready_equipment_refuses_ammunition_rows_silently() {
        // inventory.md §6/§8/§9: arrows and quarrels are carried
        // ammunition stocks, not readied equipment. Selecting either row
        // exits the cascade at the very top with no mutation and no
        // message at all - the silent refusal is unique among the
        // cascade's exits.
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![60];
        state.party_equipment = default_party_equipment(1);

        for ammo_id in [EQUIPMENT_ID_ARROWS, EQUIPMENT_ID_QUARRELS] {
            // Carried stock present: still refused, still silent.
            state.equipment_stock[ammo_id] = 5;
            state.message = "sentinel".to_string();
            assert_eq!(
                state.ready_equipment_from_suffix(&format!("1/{ammo_id}")),
                MoveOutcome::Blocked
            );
            assert_eq!(
                state.message, "sentinel",
                "ammunition row {ammo_id} must not print a message"
            );
            assert_eq!(state.equipment_stock[ammo_id], 5);
            assert_eq!(state.party_equipment[0], default_party_equipment(1)[0]);

            // Carried stock empty: the exit is above the stock gate, so it
            // stays silent rather than printing the no-carried refusal.
            state.equipment_stock[ammo_id] = 0;
            state.message = "sentinel".to_string();
            assert_eq!(
                state.ready_equipment_from_suffix(&format!("1/{ammo_id}")),
                MoveOutcome::Blocked
            );
            assert_eq!(
                state.message, "sentinel",
                "empty ammunition row {ammo_id} must stay silent"
            );
            assert_eq!(state.party_equipment[0], default_party_equipment(1)[0]);
        }
    }

    #[test]
    fn ranged_weapon_ammo_gate_matches_published_helper() {
        // inventory.md §6: the live R-Ready gate and the published
        // ranged_weapon_required_ammo helper must agree on every item id.
        assert_eq!(
            ranged_weapon_required_ammo(ITEM_ID_BOW),
            Some(ITEM_ID_ARROWS)
        );
        assert_eq!(
            ranged_weapon_required_ammo(ITEM_ID_MAGIC_BOW),
            Some(ITEM_ID_ARROWS)
        );
        assert_eq!(
            ranged_weapon_required_ammo(ITEM_ID_CROSSBOW),
            Some(ITEM_ID_QUARRELS)
        );

        // Every other equipment id carries no ammunition prerequisite.
        for item_id in 0..EQUIPMENT_COUNT {
            let expected = matches!(
                item_id,
                EQUIPMENT_ID_BOW | EQUIPMENT_ID_MAGIC_BOW | EQUIPMENT_ID_CROSSBOW
            );
            assert_eq!(
                ranged_weapon_required_ammo(item_id as u8).is_some(),
                expected,
                "ammunition prerequisite mismatch for item {item_id}"
            );
        }

        // Crossbow with no quarrels is refused; adding one clears the gate.
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![60];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[EQUIPMENT_ID_CROSSBOW] = 1;
        assert_eq!(
            state.ready_equipment_from_suffix("1/28"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "No quarrels for that weapon.");
        state.equipment_stock[EQUIPMENT_ID_QUARRELS] = 1;
        assert_eq!(state.ready_equipment_from_suffix("1/28"), MoveOutcome::Used);
        assert_eq!(
            state.party_equipment[0][EQUIP_SLOT_WEAPON],
            EQUIPMENT_ID_CROSSBOW as u8
        );
        // The ammunition stock itself is never consumed by readying.
        assert_eq!(state.equipment_stock[EQUIPMENT_ID_QUARRELS], 1);
    }

    #[test]
    fn equipment_module_classifiers_agree_with_live_dispatch_constants() {
        // The equipment module restates published tables that the live
        // paths also encode as bare constants. These are the drift joins:
        // if either side is edited alone this test fails rather than the
        // engine silently diverging from the spec.

        // inventory.md §3.1 class-tag bytes.
        assert_eq!(EQUIPMENT_CLASS_HELM, EQUIPMENT_TAG_HELM);
        assert_eq!(EQUIPMENT_CLASS_BODY_ARMOUR, EQUIPMENT_TAG_ARMOUR);
        assert_eq!(EQUIPMENT_CLASS_ONE_HAND, EQUIPMENT_TAG_ONE_HAND);
        assert_eq!(EQUIPMENT_CLASS_TWO_HAND, EQUIPMENT_TAG_TWO_HAND);
        assert_eq!(EQUIPMENT_CLASS_RING, EQUIPMENT_TAG_RING);
        assert_eq!(EQUIPMENT_CLASS_AMULET, EQUIPMENT_TAG_AMULET);
        assert_eq!(EQUIPMENT_CLASS_NONE, EQUIPMENT_TAG_AMMO);

        // Every id in the live class table decodes to a published tag,
        // and only the two ammunition rows carry the no-slot tag.
        for item_id in 0..EQUIPMENT_COUNT {
            let tag = equipment_class_tag(EQUIPMENT_CLASS_TAGS[item_id])
                .unwrap_or_else(|| panic!("item {item_id} has an unpublished class tag"));
            assert_eq!(
                tag == EquipmentClassTag::None,
                matches!(item_id, EQUIPMENT_ID_ARROWS | EQUIPMENT_ID_QUARRELS),
                "no-slot class tag mismatch for item {item_id}"
            );
        }

        // Ranged-weapon ids are shared between the two constant families.
        assert_eq!(usize::from(ITEM_ID_BOW), EQUIPMENT_ID_BOW);
        assert_eq!(usize::from(ITEM_ID_MAGIC_BOW), EQUIPMENT_ID_MAGIC_BOW);
        assert_eq!(usize::from(ITEM_ID_CROSSBOW), EQUIPMENT_ID_CROSSBOW);
        assert_eq!(usize::from(ITEM_ID_ARROWS), EQUIPMENT_ID_ARROWS);
        assert_eq!(usize::from(ITEM_ID_QUARRELS), EQUIPMENT_ID_QUARRELS);

        // inventory.md §7: the U-Use potion and scroll counter orders the
        // live dispatch matches on are the orders the classifiers publish.
        assert_eq!(POTION_USE_EFFECT_COUNT, POTION_COUNT);
        assert_eq!(
            potion_use_effect(POTION_BLUE_INDEX),
            Some(PotionUseEffect::Wake)
        );
        assert_eq!(
            potion_use_effect(POTION_YELLOW_INDEX),
            Some(PotionUseEffect::Heal)
        );
        assert_eq!(
            potion_use_effect(POTION_RED_INDEX),
            Some(PotionUseEffect::CurePoison)
        );
        assert_eq!(
            potion_use_effect(POTION_GREEN_INDEX),
            Some(PotionUseEffect::Poison)
        );
        assert_eq!(
            potion_use_effect(POTION_ORANGE_INDEX),
            Some(PotionUseEffect::Sleep)
        );
        assert_eq!(
            potion_use_effect(POTION_PURPLE_INDEX),
            Some(PotionUseEffect::PoofPresentation)
        );
        assert_eq!(
            potion_use_effect(POTION_BLACK_INDEX),
            Some(PotionUseEffect::CombatInvisibility)
        );
        assert_eq!(
            potion_use_effect(POTION_WHITE_INDEX),
            Some(PotionUseEffect::VisibilitySweep)
        );
        assert_eq!(potion_use_effect(POTION_COUNT), None);

        assert_eq!(SCROLL_USE_EFFECT_COUNT, SCROLL_COUNT);
        assert_eq!(
            scroll_use_effect(SCROLL_LIGHT_INDEX),
            Some(ScrollUseEffect::Light)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_WIND_CHANGE_INDEX),
            Some(ScrollUseEffect::WindChange)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_PROTECTION_INDEX),
            Some(ScrollUseEffect::Protection)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_NEGATE_MAGIC_INDEX),
            Some(ScrollUseEffect::NegateMagic)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_VIEW_INDEX),
            Some(ScrollUseEffect::View)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_SUMMON_DAEMON_INDEX),
            Some(ScrollUseEffect::SummonDaemon)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_RESURRECTION_INDEX),
            Some(ScrollUseEffect::Resurrection)
        );
        assert_eq!(
            scroll_use_effect(SCROLL_NEGATE_TIME_INDEX),
            Some(ScrollUseEffect::NegateTime)
        );
        assert_eq!(scroll_use_effect(SCROLL_COUNT), None);
    }

    #[test]
    fn ready_equipment_respects_hand_and_slot_occupancy() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![60];
        state.party_equipment = default_party_equipment(1);
        state.equipment_stock[31] = 1;
        state.equipment_stock[4] = 1;
        state.equipment_stock[16] = 1;

        assert_eq!(state.ready_equipment_from_suffix("1/31"), MoveOutcome::Used);
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], 31);

        assert_eq!(
            state.ready_equipment_from_suffix("1/4"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Weapon hand holds a two-handed item.");

        assert_eq!(
            state.ready_equipment_from_suffix("1/16"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Remove current weapon first.");
    }

    #[test]
    fn ready_equipment_locks_body_armour_changes_in_combat_only() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party_strengths = vec![60];
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_ARMOUR] = 9;
        state.equipment_stock[9] = 0;
        state.equipment_stock[10] = 1;
        state.equipment_stock[16] = 1;

        assert_eq!(
            state.ready_equipment_from_suffix("1/9"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_ARMOUR], 9);
        assert_eq!(state.equipment_stock[9], 0);
        assert_eq!(state.message, "Cannot change armour in combat.");

        assert_eq!(
            state.ready_equipment_from_suffix("1/10"),
            MoveOutcome::Blocked
        );
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_ARMOUR], 9);
        assert_eq!(state.equipment_stock[10], 1);
        assert_eq!(state.message, "Cannot change armour in combat.");

        assert_eq!(
            state.ready_equipment_from_suffix("1/16"),
            MoveOutcome::Used
        );
        assert_eq!(state.party_equipment[0][EQUIP_SLOT_WEAPON], 16);
        assert_eq!(state.equipment_stock[16], 0);
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn cast_reveal_clears_combat_hidden_flags_and_marks_redraw() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = REVEAL_COST;
        state.party[0].level = REVEAL_COST;
        state.spell_charges[REVEAL_SPELL_INDEX] = 1;
        state.combat_actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            32,
            7,
            0,
            4,
            5,
        ]);
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1QW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[REVEAL_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(!state.combat_actors[7].is_hidden_or_unrevealed());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Revealed 1 combat actor(s).");
    }

    #[test]
    fn cast_reveal_requires_combat_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].mana = REVEAL_COST;
        state.party[0].level = REVEAL_COST;
        state.spell_charges[REVEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1QW", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[REVEAL_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, REVEAL_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_invisibility_marks_current_combat_actor_hidden() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = INVISIBILITY_COST;
        state.party[0].level = INVISIBILITY_COST;
        state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.active_objects[0].type_byte = 0x5c;
        state.active_objects[0].tile = 0x5c;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(state.combat_actors[0].is_hidden_or_unrevealed());
        assert_eq!(state.active_objects[0].tile, COMBAT_HIDDEN_ACTIVE_OBJECT_TILE);
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Invisibility!");
    }

    #[test]
    fn cast_invisibility_requires_combat_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].mana = INVISIBILITY_COST;
        state.party[0].level = INVISIBILITY_COST;
        state.spell_charges[INVISIBILITY_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1LS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[INVISIBILITY_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, INVISIBILITY_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_cause_fear_forces_hostile_combat_actors_to_critical_hp() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = CAUSE_FEAR_COST;
        state.party[0].level = CAUSE_FEAR_COST;
        state.party_intelligence[0] = u8::MAX;
        state.spell_charges[CAUSE_FEAR_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            50,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DAEMON,
            6,
            0,
            6,
            5,
        ]);
        state.combat_actors[7] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_GIANT_RAT,
            7,
            0,
            7,
            5,
        ]);
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            25,
            1,
            COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            COMBAT_CLASS_PYTHON,
            8,
            0,
            8,
            5,
        ]);

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1CIQ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[CAUSE_FEAR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.combat_actors[0].hp_or_wound, 20);
        assert_eq!(
            state.combat_actors[6].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().max_hp)
        );
        assert_eq!(state.combat_actors[7].hp_or_wound, 20);
        assert_eq!(
            state.combat_actors[8].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
        );
        assert_eq!(
            state.message,
            "Cause Fear affected 2 combat actor(s).\nDaemon missed!\n\nAvatar, armed with bare hands:"
        );
    }

    #[test]
    fn cast_cause_fear_requires_combat_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party[0].mana = CAUSE_FEAR_COST;
        state.party[0].level = CAUSE_FEAR_COST;
        state.spell_charges[CAUSE_FEAR_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1CIQ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[CAUSE_FEAR_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, CAUSE_FEAR_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_cause_fear_has_no_faction_filter_for_charmed_monsters() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party[0].mana = CAUSE_FEAR_COST;
        state.party[0].level = CAUSE_FEAR_COST;
        state.party_intelligence[0] = u8::MAX;
        state.spell_charges[CAUSE_FEAR_SPELL_INDEX] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.combat_actors[6] = CombatActorDescriptor::from_row([
            50,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_TEAM_TOGGLE,
            COMBAT_CLASS_DAEMON,
            6,
            0,
            6,
            5,
        ]);
        state.combat_actors[7] = CombatActorDescriptor::from_row([
            50,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_PYTHON,
            7,
            0,
            7,
            5,
        ]);

        assert_eq!(state.cast_cause_fear(0), MoveOutcome::Cast);

        assert_eq!(
            state.combat_actors[6].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_DAEMON).unwrap().max_hp)
        );
        assert_eq!(
            state.combat_actors[7].hp_or_wound,
            cause_fear_forced_current_hp(combat_class_stats(COMBAT_CLASS_PYTHON).unwrap().max_hp)
        );
        assert_eq!(state.message, "Cause Fear affected 2 combat actor(s).");
    }

    #[test]
    fn cast_restore_spells_clear_status_and_heal_hp_after_resource_gates() {
        let mut cure = dungeon_state(open_dungeon_record(), 0, 1, 1);
        cure.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 10,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
        ];
        cure.spell_charges[CURE_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut cure, 'C', "1AN2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(cure.party[1].status, b'G');
        assert_eq!(cure.spell_charges[CURE_SPELL_INDEX], 0);
        assert_eq!(cure.party[0].mana, 2);
        assert_eq!(cure.turn, 1);
        assert_eq!(cure.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(cure.message, "Cured party member 2.");

        let mut awaken = dungeon_state(open_dungeon_record(), 0, 1, 1);
        awaken.party = vec![
            cure.party[0].clone(),
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 8,
                max_hp: 20,
                level: 1,
            },
        ];
        awaken.party[0].mana = 3;
        awaken.spell_charges[AWAKEN_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut awaken, 'C', "1AZ", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(awaken.party[1].status, b'G');
        assert_eq!(awaken.party[2].status, b'S');
        assert_eq!(awaken.spell_charges[AWAKEN_SPELL_INDEX], 0);
        assert_eq!(awaken.party[0].mana, 2);
        assert_eq!(awaken.turn, 1);
        assert_eq!(awaken.message, "Awakened party member 2.");

        let mut heal = dungeon_state(open_dungeon_record(), 0, 1, 1);
        heal.party = cure.party;
        heal.party[0].mana = 3;
        heal.party[1].hp = 8;
        heal.party[1].max_hp = 25;
        heal.spell_charges[HEAL_SPELL_INDEX] = 1;
        let mut expected_prng = heal.prng_state;
        let expected_raw_roll =
            u5_prng_range_u16(&mut expected_prng, 0, u16::from(HEAL_RAW_ROLL_MAX)) as u8;
        let expected_heal = heal_spell_amount_from_raw_roll(expected_raw_roll);

        assert_eq!(
            handle_play_key_input(&mut heal, 'C', "1M2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(heal.party[1].hp, 8 + expected_heal);
        assert_eq!(heal.prng_state, expected_prng);
        assert_eq!(heal.spell_charges[HEAL_SPELL_INDEX], 0);
        assert_eq!(heal.party[0].mana, 2);
        assert_eq!(heal.turn, 1);
        assert_eq!(
            heal.message,
            format!(
                "Healed party member 2 for {expected_heal} HP ({}/25).",
                8 + expected_heal
            )
        );

        let mut great_heal = dungeon_state(open_dungeon_record(), 0, 1, 1);
        great_heal.party = heal.party.clone();
        great_heal.party[0].mana = GREAT_HEAL_COST;
        great_heal.party[0].level = GREAT_HEAL_COST;
        great_heal.party[1].hp = 4;
        great_heal.party[1].max_hp = 22;
        great_heal.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut great_heal, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(great_heal.party[1].hp, 22);
        assert_eq!(great_heal.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(great_heal.party[0].mana, 0);
        assert_eq!(great_heal.turn, 1);
        assert_eq!(
            great_heal.message,
            "Great healed party member 2 for 18 HP (22/22)."
        );

        let mut resurrect = dungeon_state(open_dungeon_record(), 0, 1, 1);
        resurrect.party = great_heal.party.clone();
        resurrect.party[0].mana = RESURRECT_COST;
        resurrect.party[0].level = RESURRECT_COST;
        resurrect.party[1].status = b'D';
        resurrect.party[1].class_byte = b'B';
        resurrect.party[1].hp = 0;
        resurrect.party[1].max_hp = 19;
        resurrect.party[1].mana = 0;
        resurrect.party[1].level = 1;
        resurrect.party_experience = vec![0, 350];
        resurrect.party_intelligence = vec![30, 13];
        resurrect.moral_standing = 99;
        resurrect.spell_charges[RESURRECT_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut resurrect, 'C', "1CIM2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(resurrect.party[1].status, b'G');
        assert_eq!(resurrect.party[1].hp, 1);
        assert_eq!(resurrect.party[1].mana, 6);
        assert_eq!(resurrect.party[1].level, 3);
        assert_eq!(resurrect.party[1].max_hp, 90);
        assert_eq!(resurrect.party_experience[1], 350);
        assert_eq!(resurrect.spell_charges[RESURRECT_SPELL_INDEX], 0);
        assert_eq!(resurrect.party[0].mana, 0);
        assert_eq!(resurrect.turn, 1);
        assert_eq!(resurrect.message, "Resurrected party member 2 (1/90).");
    }

    #[test]
    fn cast_great_heal_refuses_during_dungeon_combat_active_substate() {
        // magic.md §8: Great Heal fails during the dungeon combat-active
        // substate, even on living non-Dead targets. The cast still spends
        // resources because the gate runs after charge/MP/level.
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: GREAT_HEAL_COST,
                hp: 10,
                max_hp: 20,
                level: GREAT_HEAL_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 4,
                max_hp: 22,
                level: 1,
            },
        ];
        state.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
        state.combat_active = true;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[1].hp, 4);
        assert_eq!(state.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(state.message, "Failed!");
    }

    #[test]
    fn heal_and_great_heal_skip_only_dead_targets() {
        // magic.md §8: Heal and Great Heal have a spell-specific
        // Dead-status gate; Ashes and zero-HP non-Dead records remain
        // accepted targets, and Heal leaves the status byte unchanged.
        let mut heal = dungeon_state(open_dungeon_record(), 0, 1, 1);
        heal.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: HEAL_COST,
                hp: 10,
                max_hp: 20,
                level: HEAL_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'F',
                status: b'A',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 30,
                level: 1,
            },
        ];
        heal.spell_charges[HEAL_SPELL_INDEX] = 1;
        let mut expected_prng = heal.prng_state;
        let expected_heal = heal_spell_amount_from_raw_roll(u5_prng_range_u16(
            &mut expected_prng,
            0,
            u16::from(HEAL_RAW_ROLL_MAX),
        ) as u8);

        assert_eq!(
            handle_play_key_input(&mut heal, 'C', "1M2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(heal.party[1].status, b'A');
        assert_eq!(heal.party[1].hp, expected_heal);
        assert_eq!(heal.prng_state, expected_prng);
        assert_eq!(heal.spell_charges[HEAL_SPELL_INDEX], 0);
        assert_eq!(heal.party[0].mana, 0);
        assert_eq!(heal.turn, 1);
        assert!(heal.message.starts_with("Healed party member 2"));

        let mut great_heal = heal.clone();
        great_heal.turn = 0;
        great_heal.party[0].mana = GREAT_HEAL_COST;
        great_heal.party[0].level = GREAT_HEAL_COST;
        great_heal.party[1].hp = 0;
        great_heal.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut great_heal, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(great_heal.party[1].status, b'A');
        assert_eq!(great_heal.party[1].hp, 30);
        assert_eq!(great_heal.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(great_heal.party[0].mana, 0);
        assert_eq!(great_heal.turn, 1);
        assert_eq!(
            great_heal.message,
            "Great healed party member 2 for 30 HP (30/30)."
        );
    }

    #[test]
    fn heal_amount_helper_matches_public_roll_range() {
        assert_eq!(heal_spell_amount_from_raw_roll(0), 1);
        assert_eq!(heal_spell_amount_from_raw_roll(1), 1);
        assert_eq!(heal_spell_amount_from_raw_roll(60), 30);

        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.prng_state = 0x1234;
        let mut expected_prng = state.prng_state;
        let expected_roll =
            u5_prng_range_u16(&mut expected_prng, 0, u16::from(HEAL_RAW_ROLL_MAX)) as u8;
        assert_eq!(
            state.heal_spell_amount(),
            heal_spell_amount_from_raw_roll(expected_roll)
        );
        assert_eq!(state.prng_state, expected_prng);

        let mut seen = [false; 31];
        for seed in 0..=4096 {
            state.prng_state = seed;
            let amount = state.heal_spell_amount();
            assert!((1..=30).contains(&amount));
            seen[amount as usize] = true;
        }
        assert!(seen[1]);
        assert!(seen[30]);
    }

    #[test]
    fn resurrect_rescales_experience_when_moral_standing_is_below_threshold() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: RESURRECT_COST,
                hp: 10,
                max_hp: 20,
                level: RESURRECT_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'M',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 30,
                level: 1,
            },
        ];
        state.party_experience = vec![0, 300];
        state.party_intelligence = vec![30, 21];
        state.moral_standing = 75;
        state.spell_charges[RESURRECT_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1CIM2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party_experience[1], 400);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[1].hp, 1);
        assert_eq!(state.party[1].mana, 21);
        assert_eq!(state.party[1].level, 4);
        assert_eq!(state.party[1].max_hp, 120);
        assert_eq!(state.message, "Resurrected party member 2 (1/120).");
    }

    #[test]
    fn cast_restore_target_prompt_precedes_resource_consumption() {
        let mut missing_target = dungeon_state(open_dungeon_record(), 0, 1, 1);
        missing_target.spell_charges[CURE_SPELL_INDEX] = 1;
        missing_target.party[0].mana = CURE_COST;
        missing_target.party[0].level = CURE_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_target, 'C', "1AN", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_target.spell_charges[CURE_SPELL_INDEX], 1);
        assert_eq!(missing_target.party[0].mana, CURE_COST);
        assert_eq!(missing_target.turn, 0);
        assert_eq!(
            missing_target.message,
            "Whom? Use C1AN2 to cure party member 2."
        );

        let mut invalid_target = dungeon_state(open_dungeon_record(), 0, 1, 1);
        invalid_target.spell_charges[CURE_SPELL_INDEX] = 1;
        invalid_target.party[0].mana = CURE_COST;
        invalid_target.party[0].level = CURE_COST;

        assert_eq!(
            handle_play_key_input(&mut invalid_target, 'C', "1AN2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(invalid_target.spell_charges[CURE_SPELL_INDEX], 1);
        assert_eq!(invalid_target.party[0].mana, CURE_COST);
        assert_eq!(invalid_target.turn, 0);
        assert_eq!(invalid_target.message, "Party has 1 member.");
    }

    #[test]
    fn cast_restore_non_applicable_target_consumes_cast_and_fails() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party[0].status = b'G';
        state.spell_charges[CURE_SPELL_INDEX] = 1;
        state.party[0].mana = CURE_COST;
        state.party[0].level = CURE_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AN1", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.spell_charges[CURE_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Failed!");

        let mut great_heal_dead = dungeon_state(open_dungeon_record(), 0, 1, 1);
        great_heal_dead.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: GREAT_HEAL_COST,
                hp: 10,
                max_hp: 20,
                level: GREAT_HEAL_COST,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 20,
                level: 1,
            },
        ];
        great_heal_dead.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;

        assert_eq!(
            handle_play_key_input(&mut great_heal_dead, 'C', "1MV2", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(great_heal_dead.party[1].status, b'D');
        assert_eq!(great_heal_dead.party[1].hp, 0);
        assert_eq!(great_heal_dead.spell_charges[GREAT_HEAL_SPELL_INDEX], 0);
        assert_eq!(great_heal_dead.party[0].mana, 0);
        assert_eq!(great_heal_dead.turn, 1);
        assert_eq!(great_heal_dead.message, "Failed!");

        let mut resurrect_living = dungeon_state(open_dungeon_record(), 0, 1, 1);
        resurrect_living.party[0].status = b'G';
        resurrect_living.party[0].hp = 10;
        resurrect_living.spell_charges[RESURRECT_SPELL_INDEX] = 1;
        resurrect_living.party[0].mana = RESURRECT_COST;
        resurrect_living.party[0].level = RESURRECT_COST;

        assert_eq!(
            handle_play_key_input(&mut resurrect_living, 'C', "1CIM1", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(resurrect_living.party[0].status, b'G');
        assert_eq!(resurrect_living.party[0].hp, 10);
        assert_eq!(resurrect_living.spell_charges[RESURRECT_SPELL_INDEX], 0);
        assert_eq!(resurrect_living.party[0].mana, 0);
        assert_eq!(resurrect_living.turn, 1);
        assert_eq!(resurrect_living.message, "Failed!");
    }

    /// `lighting.md §8`: "Great Light, *Vas Lor*, sets the same counter
    /// to 255 counter units"; `catalogs/spell-list.md` agrees - "`Vas Lor`
    /// starts or refreshes the same counter at 255 units."
    ///
    /// The clock is moved to a night hour so §4's light-spell floor of 18
    /// is the value actually under test. `lighting.md` §3 withdraws the
    /// dungeon-depth forced-dark reading - "a dungeon level index counts
    /// upward from zero at the top of the stack, so it never sets the high
    /// bit, and the ambient value computed while the party is inside a
    /// dungeon is simply whatever the clock produces" - so at noon this
    /// dungeon recomputes to the full-daylight 50 and the floor is a no-op
    /// ("In daylight, where the ambient value is already 50, lighting a
    /// torch or casting a light spell changes nothing at all"). An earlier
    /// revision of this test read noon-in-a-dungeon as forced dark; that
    /// reading is retracted.
    #[test]
    fn cast_vas_lor_sets_great_light_counter() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.spell_charges[VAS_LOR_SPELL_INDEX] = 1;
        state.party[0].mana = 3;
        state.party[0].level = 3;
        state.clock = GameClock::new(22, 0).unwrap();
        state.ambient_light = FULL_DARKNESS;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1VL", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.spell_charges[VAS_LOR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.light_spell_counter, VAS_LOR_LIGHT_DURATION);
        assert_eq!(VAS_LOR_LIGHT_DURATION, 255);
        assert_eq!(state.ambient_light, LIGHT_SPELL_FLOOR);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Light!");
    }

    #[test]
    fn cast_an_sanct_safely_opens_dungeon_chest_underfoot() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.spell_charges[OPEN_SPELL_INDEX] = 1;
        state.party[0].mana = OPEN_SPELL_COST;
        state.party[0].level = OPEN_SPELL_COST;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x78);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(
            state.message,
            "Safely opened dungeon chest at (1, 1) on DUNGEON:0 level 0; trap generator bypassed by An Sanct, marked visit-local open chest."
        );
    }

    #[test]
    fn cast_an_sanct_opens_without_applying_clean_sidecar_chest_grants() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(DUNGEON_CHEST_TABLE_FILE),
            "DUNGEON:0 0 1 1 0x4b GOLD 12 TORCHES 1\n",
        )
        .unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.spell_charges[OPEN_SPELL_INDEX] = 1;
        state.party[0].mana = OPEN_SPELL_COST;
        state.party[0].level = OPEN_SPELL_COST;
        let starting_gold = state.gold;
        let starting_torches = state.torches;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1AS", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x78);
        assert_eq!(state.gold, starting_gold);
        assert_eq!(state.torches, starting_torches);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert!(
            state
                .message
                .contains("trap generator bypassed by An Sanct")
        );
        assert!(!state.message.contains("authored chest grants"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_an_sanct_opens_ordinary_surface_doors_without_open_tracker() {
        let mut town_grid = open_grid();
        town_grid[1 * 32 + 2] = 0xB9;
        let mut town = test_state(town_grid, 1, 1);
        town.spell_charges[OPEN_SPELL_INDEX] = 1;
        town.party[0].mana = OPEN_SPELL_COST;
        town.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut town, 'C', "1AS6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(town.grid[1 * 32 + 2], 0xb8);
        assert_eq!(town.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(town.party[0].mana, 0);
        assert_eq!(town.turn, 1);
        assert_eq!(town.clock, GameClock::new(12, 1).unwrap());
        assert!(town.visibility_dirty);
        assert_eq!(town.door_tracker, None);
        let Area::Town { scene, floor } = town.area else {
            unreachable!("test state should be a town");
        };
        assert!(!town.is_recorded_open_town_door(scene, floor, 2, 1));
        assert_eq!(town.message, "Success!");

        let mut world_grid = open_world_grid();
        world_grid[world_cell_index(4, 5)] = 0xBB;
        let mut world = britannia_state(world_grid, 5, 5);
        world.spell_charges[OPEN_SPELL_INDEX] = 1;
        world.party[0].mana = OPEN_SPELL_COST;
        world.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut world, 'C', "1AS4", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(world.grid[world_cell_index(4, 5)], 0xba);
        assert_eq!(world.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(world.party[0].mana, 0);
        assert_eq!(world.turn, 1);
        assert_eq!(world.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(world.door_tracker, None);
        assert_eq!(world.message, "Success!");
    }

    #[test]
    fn cast_an_sanct_failures_preserve_public_resource_ordering() {
        let mut no_charge = dungeon_state(open_dungeon_record(), 0, 1, 1);
        no_charge.party[0].mana = OPEN_SPELL_COST;
        no_charge.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut no_charge, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(no_charge.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(no_charge.party[0].mana, OPEN_SPELL_COST);
        assert_eq!(no_charge.turn, 0);
        assert_eq!(no_charge.message, "None mixed!");

        let mut no_modeled_target = test_state(open_grid(), 1, 1);
        no_modeled_target.spell_charges[OPEN_SPELL_INDEX] = 1;
        no_modeled_target.party[0].mana = OPEN_SPELL_COST;
        no_modeled_target.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut no_modeled_target, 'C', "1AS6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(no_modeled_target.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(no_modeled_target.party[0].mana, 0);
        assert_eq!(no_modeled_target.turn, 1);
        assert_eq!(no_modeled_target.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(no_modeled_target.message, "Failed!");

        let mut wrong_underfoot = dungeon_state(open_dungeon_record(), 0, 1, 1);
        wrong_underfoot.spell_charges[OPEN_SPELL_INDEX] = 1;
        wrong_underfoot.party[0].mana = OPEN_SPELL_COST;
        wrong_underfoot.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            handle_play_key_input(&mut wrong_underfoot, 'C', "1AS", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(wrong_underfoot.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(wrong_underfoot.party[0].mana, 0);
        assert_eq!(wrong_underfoot.turn, 1);
        assert_eq!(wrong_underfoot.message, "Failed!");
    }

    #[test]
    fn combat_open_rewrites_the_adjacent_live_arena_door() {
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = 0x97;
        let mut state = britannia_state(grid, 5, 5);
        state.combat_active = true;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_terrain[5][6] = 0xB9;
        state.spell_charges[OPEN_SPELL_INDEX] = 1;
        state.party[0].mana = OPEN_SPELL_COST;
        state.party[0].level = OPEN_SPELL_COST;
        state.visibility_dirty = false;

        assert_eq!(
            state
                .cast_spell_from_suffix("1AS6", Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.grid[world_cell_index(6, 5)], 0x97);
        assert_eq!(state.combat_terrain[5][6], 0xB8);
        assert_eq!(state.spell_charges[OPEN_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert_eq!(state.message, "Success!");
    }

    #[test]
    fn combat_open_clears_the_adjacent_kind_one_chest_trap_bit() {
        let mut state = test_state(open_grid(), 5, 5);
        state.combat_active = true;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_objects.resize(3, ActiveObject::empty());
        state.active_objects[2] = ActiveObject {
            type_byte: COMBAT_DEFAULT_DEATH_DROP_TILE,
            tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
            x: 6,
            y: 5,
            z: 7,
            aux1: 0xA5,
            ..ActiveObject::empty()
        };
        state.spell_charges[OPEN_SPELL_INDEX] = 1;
        state.party[0].mana = OPEN_SPELL_COST;
        state.party[0].level = OPEN_SPELL_COST;

        assert_eq!(
            state
                .cast_spell_from_suffix("1AS6", Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.active_objects[2].aux1, 0x25);
        assert_eq!(state.message, "Success!");
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn combat_vanish_rewrites_only_the_adjacent_live_arena_tile() {
        let mut object_state = test_state(open_grid(), 5, 5);
        object_state.combat_active = true;
        object_state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        object_state.active_objects.resize(3, ActiveObject::empty());
        object_state.active_objects[1] = ActiveObject {
            type_byte: 0x50,
            tile: 0x50,
            x: 6,
            y: 5,
            ..ActiveObject::empty()
        };
        object_state.combat_terrain[5][6] = 0x90;
        object_state.spell_charges[VANISH_SPELL_INDEX] = 1;
        object_state.party[0].mana = VANISH_COST;
        object_state.party[0].level = VANISH_COST;
        object_state.visibility_dirty = false;

        assert_eq!(
            object_state
                .cast_spell_from_suffix("1AY6", Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(object_state.active_objects[1].tile, 0x50);
        assert_eq!(object_state.combat_terrain[5][6], VANISH_CLEARED_TILE);
        assert_eq!(object_state.spell_charges[VANISH_SPELL_INDEX], 0);
        assert_eq!(object_state.party[0].mana, 0);
        assert_eq!(object_state.turn, 1);
        assert_eq!(object_state.message, "POOF!");

        let mut field_state = test_state(open_grid(), 5, 5);
        field_state.combat_active = true;
        field_state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        field_state.active_objects.resize(3, ActiveObject::empty());
        field_state.active_objects[2] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_FIRE,
            tile: COMBAT_FIELD_KIND_FIRE,
            x: 6,
            y: 5,
            ..ActiveObject::empty()
        };
        field_state.spell_charges[VANISH_SPELL_INDEX] = 1;
        field_state.party[0].mana = VANISH_COST;
        field_state.party[0].level = VANISH_COST;

        assert_eq!(
            field_state
                .cast_spell_from_suffix("1AY6", Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(field_state.active_objects[2].tile, COMBAT_FIELD_KIND_FIRE);
        assert_eq!(field_state.message, "Failed!");
    }

    #[test]
    fn combat_vanish_failure_spends_resources_without_removing_actor() {
        let mut state = test_state(open_grid(), 5, 5);
        state.combat_active = true;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            1,
            0,
            6,
            5,
        ]);
        state.spell_charges[VANISH_SPELL_INDEX] = 1;
        state.party[0].mana = VANISH_COST;
        state.party[0].level = VANISH_COST;
        let target_before = state.combat_actors[1];

        assert_eq!(
            state
                .cast_spell_from_suffix("1AY6", Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.combat_actors[1], target_before);
        assert_eq!(state.spell_charges[VANISH_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Failed!");
    }

    #[test]
    fn cast_blink_scans_cardinal_ray_and_leaves_intervening_lock() {
        let mut grid = open_world_grid();
        grid[world_cell_index(2, 1)] = 97;
        let mut state = britannia_state(grid, 1, 1);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;
        state.visibility_dirty = false;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (15, 1));
        assert_eq!(state.grid[world_cell_index(2, 1)], 97);
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Blinked East to (15, 1) in BRITANNIA.");
    }

    #[test]
    fn cast_blink_ignores_active_object_occupancy_on_farthest_grass() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 15,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (15, 1));
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (15, 1));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Blinked East to (15, 1) in BRITANNIA.");
    }

    #[test]
    fn cast_blink_preserves_transport_while_slot_zero_uses_supported_save_marker() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.player.transport = TransportState::Horse {
            type_byte: 0x10,
            tile: 0x10,
        };
        state.sync_player_object();
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (15, 1));
        assert_eq!(
            state.player.transport,
            TransportState::Horse {
                type_byte: 0x10,
                tile: 0x10,
            }
        );
        assert_eq!(state.active_objects[0].tile, state.player.transport.save_marker());
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Blinked East to (15, 1) in BRITANNIA.");
    }

    #[test]
    fn cast_blink_rejects_indoor_context_before_spending_resources() {
        let mut state = test_state(open_grid(), 1, 1);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 1);
        assert_eq!(state.party[0].mana, BLINK_COST);
        assert_eq!(state.turn, 0);
        assert_eq!(state.message, "Not here!");
    }

    #[test]
    fn cast_blink_lands_on_farthest_grass_without_step_effect() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 2 1 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1IP6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((state.player.x, state.player.y), (15, 1));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Blinked East to (15, 1) in BRITANNIA.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_blink_rejections_preserve_public_resource_ordering() {
        // `catalogs/spell-list.md §5` id 17 publishes Blink as `C/O`, so the
        // direction prompt is only reachable in a scene the mask allows.
        // This case used to run in an indoor scene, where `magic.md §5`
        // step 3's context gate now refuses first.
        let mut missing_direction = britannia_state(open_world_grid(), 1, 1);
        missing_direction.spell_charges[BLINK_SPELL_INDEX] = 1;
        missing_direction.party[0].mana = BLINK_COST;
        missing_direction.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_direction, 'C', "1IP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(missing_direction.spell_charges[BLINK_SPELL_INDEX], 1);
        assert_eq!(missing_direction.party[0].mana, BLINK_COST);
        assert_eq!(missing_direction.turn, 0);
        assert_eq!(missing_direction.message, "Direction? Use C1IP6.");

        // `magic.md §5` step 3: the context gate runs before the handler's
        // direction prompt, so an indoor Blink reports `Not here!` rather
        // than asking for a direction, and spends nothing.
        let mut indoor = test_state(open_grid(), 1, 1);
        indoor.spell_charges[BLINK_SPELL_INDEX] = 1;
        indoor.party[0].mana = BLINK_COST;
        indoor.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut indoor, 'C', "1IP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(indoor.spell_charges[BLINK_SPELL_INDEX], 1);
        assert_eq!(indoor.party[0].mana, BLINK_COST);
        assert_eq!(indoor.turn, 0);
        assert_eq!(indoor.message, "Not here!");

        let mut passed = britannia_state(open_world_grid(), 1, 1);
        passed.spell_charges[BLINK_SPELL_INDEX] = 1;
        passed.party[0].mana = BLINK_COST;
        passed.party[0].level = BLINK_COST;
        assert_eq!(passed.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(passed
            .step_active_cast('I', "P", Path::new(""))
            .unwrap()
            .is_none());
        assert!(passed
            .step_active_cast(' ', "", Path::new(""))
            .unwrap()
            .is_none());
        let pass_result = passed
            .step_active_cast_followup(' ', "", Path::new(""))
            .unwrap()
            .expect("Pass should finish Blink");
        assert_eq!(pass_result.0, MoveOutcome::Cast);
        assert_eq!((passed.player.x, passed.player.y), (1, 1));
        assert_eq!(passed.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(passed.party[0].mana, 0);
        assert_eq!(passed.turn, 1);
        assert_eq!(passed.message, DIRECTION_PROMPT_LABEL_PASS);

        let mut no_grass_grid = open_world_grid();
        for x in 2..=15 {
            no_grass_grid[world_cell_index(x, 1)] = 0x04;
        }
        let mut missing_row = britannia_state(no_grass_grid, 1, 1);
        missing_row.spell_charges[BLINK_SPELL_INDEX] = 1;
        missing_row.party[0].mana = BLINK_COST;
        missing_row.party[0].level = BLINK_COST;

        assert_eq!(
            handle_play_key_input(&mut missing_row, 'C', "1IP6", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!((missing_row.player.x, missing_row.player.y), (1, 1));
        assert_eq!(missing_row.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(missing_row.party[0].mana, 0);
        assert_eq!(missing_row.turn, 1);
        assert_eq!(missing_row.message, "Failed!");
    }

    #[test]
    fn combat_allowed_blink_moves_linked_actor_to_selected_coordinate() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            1,
            1,
        ]);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;
        state.visibility_dirty = false;

        assert_eq!(
            state.cast_spell_from_suffix("1IP3,1", Path::new("")).unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!((state.player.x, state.player.y), (1, 1));
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (3, 1));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (3, 1));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
        assert!(state.visibility_dirty);
        assert_eq!(state.message, "Blinked to (3, 1).");
    }

    #[test]
    fn combat_blink_failure_spends_resources_without_moving_actor() {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[1][2] = 0x0c;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            1,
            1,
        ]);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;
        state.visibility_dirty = false;

        assert_eq!(
            state
                .cast_spell_from_suffix("1IP2,1", Path::new(""))
                .unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (1, 1));
        assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (1, 1));
        assert_eq!(state.spell_charges[BLINK_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.message, "Failed!");
    }

    #[test]
    fn active_combat_blink_followup_moves_cursor_and_confirms_coordinate() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.spell_charges[BLINK_SPELL_INDEX] = 1;
        state.party[0].mana = BLINK_COST;
        state.party[0].level = BLINK_COST;

        assert_eq!(
            state.start_combat_cast_spell_prompt(0, false),
            MoveOutcome::Observed
        );
        assert!(state
            .step_active_cast('I', "P", Path::new(""))
            .unwrap()
            .is_none());
        assert!(state
            .step_active_cast(' ', "", Path::new(""))
            .unwrap()
            .is_none());
        assert!(state.active_cast_followup.is_some());
        assert!(state.message.contains("Target?"));

        assert!(state
            .step_active_cast_followup('6', "", Path::new(""))
            .unwrap()
            .is_none());
        assert!(state.message.contains("(6, 5)"));
        let result = state
            .step_active_cast_followup(' ', "", Path::new(""))
            .unwrap()
            .expect("coordinate confirmation should finish Blink");
        assert_eq!(result.0, MoveOutcome::Cast);
        assert_eq!(result.1, Some((0, false)));
        assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), (6, 5));
        assert_eq!(state.message, "Blinked to (6, 5).");
    }

    #[test]
    fn cast_uus_por_moves_up_one_dungeon_level_without_ladder() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(2, 1, 1)] = 0x10;
        let mut state = dungeon_state(grid, 3, 1, 1);
        state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1UP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 2 });
        assert_eq!(state.active_objects[0].z, 2);
        assert_eq!(state.spell_charges[UUS_POR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(state.message, "Up! Changed to DUNGEON:0 (Deceit) level 3.");
    }

    #[test]
    fn cast_des_por_moves_down_one_dungeon_level_without_ladder() {
        let scene = DungeonScene::new(33).unwrap();
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(4, 1, 1)] = 0x10;
        let mut state = dungeon_state(grid, 3, 1, 1);
        state.spell_charges[DES_POR_SPELL_INDEX] = 1;
        state.party[0].mana = 4;
        state.party[0].level = 4;

        assert_eq!(
            handle_play_key_input(&mut state, 'C', "1DP", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(state.area, Area::Dungeon { scene, level: 4 });
        assert_eq!(state.active_objects[0].z, 4);
        assert_eq!(state.spell_charges[DES_POR_SPELL_INDEX], 0);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.turn, 1);
        assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
        assert_eq!(
            state.message,
            "Down! Changed to DUNGEON:0 (Deceit) level 5."
        );
    }

    #[test]
    fn cast_dungeon_field_spells_place_public_field_bytes() {
        for (suffix, spell_index, cost, starting_tile, expected_tile, expected_message) in [
            (
                "1FGI6",
                FIRE_FIELD_SPELL_INDEX,
                FIELD_SPELL_COST,
                0x00,
                0x82,
                "Fire field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
            (
                "1GIN6",
                POISON_FIELD_SPELL_INDEX,
                FIELD_SPELL_COST,
                0x00,
                0x81,
                "Poison field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
            (
                "1GIZ6",
                SLEEP_FIELD_SPELL_INDEX,
                FIELD_SPELL_COST,
                0x08,
                0x88,
                "Sleep field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
            (
                "1GIS6",
                ENERGY_FIELD_SPELL_INDEX,
                ENERGY_FIELD_COST,
                0x08,
                0x8b,
                "Energy field placed East at (2, 1) on DUNGEON:0 level 0.",
            ),
        ] {
            let mut grid = open_dungeon_record();
            grid[dungeon_cell_index(0, 2, 1)] = starting_tile;
            let mut state = dungeon_state(grid, 0, 1, 1);
            state.spell_charges[spell_index] = 1;
            state.party[0].mana = cost;
            state.party[0].level = cost;
            state.visibility_dirty = false;

            assert_eq!(
                handle_play_key_input(&mut state, 'C', suffix, Path::new("")).unwrap(),
                PlayInputDisposition::Continue
            );

            assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], expected_tile);
            assert_eq!(state.spell_charges[spell_index], 0);
            assert_eq!(state.party[0].mana, 0);
            assert_eq!(state.turn, 1);
            assert_eq!(state.clock, GameClock::new(12, 1).unwrap());
            assert!(state.visibility_dirty);
            assert_eq!(state.message, expected_message);
        }
    }
