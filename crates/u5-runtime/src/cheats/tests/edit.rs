//! Edit party and stores (design 7.2.6, 7.3.3; build plan M8-03): every
//! field at both ends of its range and one step outside, the status rules,
//! `[L]`, the slot-0 mirrors through a save, untouched out-of-range values
//! and the empty Apply.

use super::*;

fn stats(strength: u8, dexterity: u8, intelligence: u8) -> MemberEdit {
    MemberEdit {
        strength: Some(strength),
        dexterity: Some(dexterity),
        intelligence: Some(intelligence),
        ..MemberEdit::default()
    }
}

fn member_edit(slot: usize, fields: MemberEdit) -> CheatEdit {
    CheatEdit::Member { slot, fields }
}

/// Applies `edit` and checks it either changed the state or found it
/// already so; a refusal fails.
fn accepted(state: &mut PlayState, edit: CheatEdit) {
    match apply_cheat_edits(state, std::slice::from_ref(&edit)) {
        Ok(_) | Err(CheatRefusal::NothingToDo) => {}
        Err(refusal) => panic!("{edit:?} refused: {refusal:?}"),
    }
}

/// Applies `edit`, expects `OutOfRange { min, max }` and an untouched state.
fn refused(state: &mut PlayState, edit: CheatEdit, min: u32, max: u32) {
    let before = format!("{state:?}");
    assert_eq!(
        apply_cheat_edits(state, std::slice::from_ref(&edit)),
        Err(CheatRefusal::OutOfRange { min, max }),
        "{edit:?}"
    );
    assert_eq!(
        format!("{state:?}"),
        before,
        "{edit:?} left the state alone"
    );
}

#[test]
fn every_member_field_at_both_ends_and_one_step_outside() {
    let mut state = town();
    for slot in [0, 1] {
        for value in [CHEAT_STAT_MIN, CHEAT_STAT_MAX] {
            accepted(&mut state, member_edit(slot, stats(value, value, value)));
            for stat in [
                PrimaryStat::Strength,
                PrimaryStat::Dexterity,
                PrimaryStat::Intelligence,
            ] {
                assert_eq!(member_stat(&state, slot, stat), Some(value));
            }
        }
        for value in [CHEAT_STAT_MIN - 1, CHEAT_STAT_MAX + 1] {
            for fields in [
                MemberEdit {
                    strength: Some(value),
                    ..MemberEdit::default()
                },
                MemberEdit {
                    dexterity: Some(value),
                    ..MemberEdit::default()
                },
                MemberEdit {
                    intelligence: Some(value),
                    ..MemberEdit::default()
                },
            ] {
                refused(&mut state, member_edit(slot, fields), 1, 30);
            }
        }
    }

    // HP: 1..=max for the living.
    let hp = |hp| MemberEdit {
        hp: Some(hp),
        ..MemberEdit::default()
    };
    for value in [1, 240] {
        accepted(&mut state, member_edit(0, hp(value)));
        assert_eq!(state.party[0].hp, value);
    }
    refused(&mut state, member_edit(0, hp(0)), 1, 240);
    refused(&mut state, member_edit(0, hp(241)), 1, 240);
    // A Dead or Ashes member has no HP range.
    refused(&mut state, member_edit(3, hp(10)), 0, 0);
    refused(&mut state, member_edit(4, hp(10)), 0, 0);

    // MP: 0..=class target, from the INT the same edit sets.
    let mana = |mana, intelligence| MemberEdit {
        mana: Some(mana),
        intelligence,
        ..MemberEdit::default()
    };
    accepted(&mut state, member_edit(0, mana(0, Some(24))));
    accepted(&mut state, member_edit(0, mana(24, None)));
    assert_eq!(state.party[0].mana, 24);
    refused(&mut state, member_edit(0, mana(25, None)), 0, 24);
    refused(&mut state, member_edit(0, mana(21, Some(20))), 0, 20);
    // Bard: half INT (16 / 2 = 8).
    accepted(&mut state, member_edit(1, mana(8, Some(16))));
    refused(&mut state, member_edit(1, mana(9, None)), 0, 8);
    // A fighter has no published target.
    refused(&mut state, member_edit(3, mana(0, None)), 0, 0);

    // XP: 0..=9999.
    let xp = |xp| MemberEdit {
        experience: Some(xp),
        ..MemberEdit::default()
    };
    for value in [0, CHEAT_EXPERIENCE_MAX] {
        accepted(&mut state, member_edit(2, xp(value)));
        assert_eq!(state.party_experience[2], value);
    }
    refused(
        &mut state,
        member_edit(2, xp(CHEAT_EXPERIENCE_MAX + 1)),
        0,
        9999,
    );

    assert_eq!(
        apply_cheat_edits(&mut state, &[member_edit(5, stats(9, 9, 9))]),
        Err(CheatRefusal::NoSuchMember)
    );
}

#[test]
fn every_store_at_both_ends_and_one_step_outside() {
    let mut state = town();
    let words: [fn(u16) -> StoresEdit; 2] = [
        |v| StoresEdit {
            food: Some(v),
            ..StoresEdit::default()
        },
        |v| StoresEdit {
            gold: Some(v),
            ..StoresEdit::default()
        },
    ];
    for edit in words {
        for value in [0, CHEAT_FOOD_MAX] {
            accepted(&mut state, CheatEdit::Stores(edit(value)));
        }
        refused(
            &mut state,
            CheatEdit::Stores(edit(CHEAT_FOOD_MAX + 1)),
            0,
            9999,
        );
    }
    assert_eq!((state.food, state.gold), (9999, 9999));

    let mut bytes: Vec<Box<dyn Fn(u8) -> StoresEdit>> = vec![
        Box::new(|v| StoresEdit {
            keys: Some(v),
            ..StoresEdit::default()
        }),
        Box::new(|v| StoresEdit {
            gems: Some(v),
            ..StoresEdit::default()
        }),
        Box::new(|v| StoresEdit {
            torches: Some(v),
            ..StoresEdit::default()
        }),
        Box::new(|v| StoresEdit {
            arrows: Some(v),
            ..StoresEdit::default()
        }),
        Box::new(|v| StoresEdit {
            quarrels: Some(v),
            ..StoresEdit::default()
        }),
    ];
    for index in 0..REAGENT_COUNT {
        bytes.push(Box::new(move |v| {
            let mut edit = StoresEdit::default();
            edit.reagents[index] = Some(v);
            edit
        }));
    }
    for edit in &bytes {
        for value in [0, CHEAT_BYTE_STOCK_MAX] {
            accepted(&mut state, CheatEdit::Stores(edit(value)));
        }
        refused(
            &mut state,
            CheatEdit::Stores(edit(CHEAT_BYTE_STOCK_MAX + 1)),
            0,
            99,
        );
    }
    assert_eq!((state.keys, state.gems, state.torches), (99, 99, 99));
    assert_eq!(state.equipment_stock[EQUIPMENT_ID_ARROWS], 99);
    assert_eq!(state.equipment_stock[EQUIPMENT_ID_QUARRELS], 99);
    assert_eq!(state.reagents, [99; REAGENT_COUNT]);
}

#[test]
fn dead_zeroes_hp_and_good_on_the_dead_sets_max() {
    let mut state = town();
    let status = |status| MemberEdit {
        status: Some(status),
        ..MemberEdit::default()
    };
    accepted(&mut state, member_edit(0, status(StoredStatus::Dead)));
    assert_eq!((state.party[0].status, state.party[0].hp), (b'D', 0));
    assert!(!state.party[0].living());

    assert_eq!(state.party[3].status, b'D');
    accepted(&mut state, member_edit(3, status(StoredStatus::Good)));
    assert_eq!((state.party[3].status, state.party[3].hp), (b'G', 150));

    // Poisoned to Good keeps the HP it had.
    accepted(&mut state, member_edit(1, status(StoredStatus::Good)));
    assert_eq!((state.party[1].status, state.party[1].hp), (b'G', 60));

    // A typed HP in the same edit wins over the max.
    accepted(
        &mut state,
        member_edit(
            0,
            MemberEdit {
                status: Some(StoredStatus::Good),
                hp: Some(7),
                ..MemberEdit::default()
            },
        ),
    );
    assert_eq!((state.party[0].status, state.party[0].hp), (b'G', 7));
}

#[test]
fn ashes_is_shown_but_cannot_be_chosen_or_changed() {
    assert_eq!(StoredStatus::of_byte(b'A'), None);
    assert_eq!(StoredStatus::of_byte(b'C'), None, "charmed is never stored");
    let letters: Vec<u8> = StoredStatus::ALL.iter().map(|s| s.byte()).collect();
    assert_eq!(letters, *b"GPSD");
    for status in StoredStatus::ALL {
        assert_eq!(StoredStatus::of_byte(status.byte()), Some(status));
        let mut state = town();
        let before = format!("{state:?}");
        assert_eq!(
            apply_cheat_edits(
                &mut state,
                &[member_edit(
                    4,
                    MemberEdit {
                        status: Some(status),
                        ..MemberEdit::default()
                    }
                )]
            ),
            Err(CheatRefusal::NothingToDo)
        );
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
fn status_cannot_be_edited_in_combat_but_stats_can() {
    let mut state = combat();
    let before = format!("{state:?}");
    for status in StoredStatus::ALL {
        assert_eq!(
            apply_cheat_edits(
                &mut state,
                &[member_edit(
                    0,
                    MemberEdit {
                        status: Some(status),
                        ..MemberEdit::default()
                    }
                )]
            ),
            Err(CheatRefusal::InCombat)
        );
    }
    assert_eq!(format!("{state:?}"), before);
    accepted(&mut state, member_edit(0, stats(11, 12, 13)));
    assert_eq!(state.avatar_stats.strength, 11);
}

#[test]
fn recompute_level_follows_the_camp_rule() {
    for (xp, level) in [(0, 1), (99, 1), (100, 2), (6399, 7), (6400, 8), (9999, 8)] {
        let mut state = town();
        state.party_experience[0] = xp;
        state.party[0].level = 0;
        apply_cheat_edits(&mut state, &[CheatEdit::RecomputeLevel { slot: 0 }]).unwrap();
        let member = state.party[0];
        assert_eq!(member.level, level, "XP {xp}");
        assert_eq!(member.max_hp, 30 * u16::from(level), "XP {xp}");
        assert_eq!(member.hp, member.max_hp, "XP {xp}");
        // No stat reward.
        assert_eq!(state.avatar_stats, town().avatar_stats);
        // Again: nothing to do.
        assert_eq!(
            apply_cheat_edits(&mut state, &[CheatEdit::RecomputeLevel { slot: 0 }]),
            Err(CheatRefusal::NothingToDo)
        );
    }
    // A Dead member gets the new maximum but stays at 0 HP.
    let mut state = town();
    state.party_experience[3] = 6400;
    apply_cheat_edits(&mut state, &[CheatEdit::RecomputeLevel { slot: 3 }]).unwrap();
    assert_eq!(
        (
            state.party[3].level,
            state.party[3].max_hp,
            state.party[3].hp
        ),
        (8, 240, 0)
    );
    // Edits run in order: the XP set first is the one [L] reads.
    let mut state = town();
    apply_cheat_edits(
        &mut state,
        &[
            member_edit(
                1,
                MemberEdit {
                    experience: Some(100),
                    ..MemberEdit::default()
                },
            ),
            CheatEdit::RecomputeLevel { slot: 1 },
        ],
    )
    .unwrap();
    assert_eq!((state.party[1].level, state.party[1].max_hp), (2, 60));
    assert_eq!(
        apply_cheat_edits(&mut state, &[CheatEdit::RecomputeLevel { slot: 9 }]),
        Err(CheatRefusal::NoSuchMember)
    );
}

/// The four slot-0 mirrors and an untouched out-of-range STR, through a
/// real save and `load_play_options_from_save`.
#[test]
fn slot_zero_edit_updates_the_mirrors_and_survives_a_save() {
    let mut template = saved_game_seed_bytes(17, 0, 15, 15);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    let dir = debug_game_dir();
    fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let mut state = world();
    apply_cheat_edits(
        &mut state,
        &[
            member_edit(
                0,
                MemberEdit {
                    experience: Some(6400),
                    ..stats(17, 18, 19)
                },
            ),
            CheatEdit::RecomputeLevel { slot: 0 },
            member_edit(
                4,
                MemberEdit {
                    dexterity: Some(5),
                    ..MemberEdit::default()
                },
            ),
            CheatEdit::Stores(StoresEdit {
                food: Some(1234),
                keys: Some(42),
                ..StoresEdit::default()
            }),
        ],
    )
    .unwrap();
    assert_eq!(
        state.avatar_stats,
        AvatarStats {
            strength: 17,
            dexterity: 18,
            intelligence: 19
        }
    );
    assert_eq!(state.party_strengths[0], 17);
    assert_eq!(state.party[0].climb_stat, 18);
    assert_eq!(state.party_intelligence[0], 19);
    assert_eq!(
        state.party_strengths[4], 45,
        "untouched out-of-range STR kept"
    );
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(
        options.avatar_stats,
        AvatarStats {
            strength: 17,
            dexterity: 18,
            intelligence: 19
        }
    );
    assert_eq!(options.party[0].climb_stat, 18);
    assert_eq!(options.party_strengths[0], 17);
    assert_eq!(options.party_intelligence[0], 19);
    assert_eq!(options.party[0].level, 8);
    assert_eq!(options.party[0].max_hp, 240);
    assert_eq!(options.party_experience[0], 6400);
    assert_eq!(options.party[4].climb_stat, 5);
    assert_eq!(options.party[4].status, b'A');
    assert_eq!(
        options.party_strengths[4], 45,
        "untouched out-of-range STR kept"
    );
    assert_eq!((options.food, options.keys), (1234, 42));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_empty_apply_is_nothing_to_do() {
    for (label, mut state) in fixtures() {
        let before = format!("{state:?}");
        for edits in [
            Vec::new(),
            vec![member_edit(0, MemberEdit::default())],
            vec![CheatEdit::Stores(StoresEdit::default())],
            vec![
                member_edit(0, stats(20, 20, 24)),
                CheatEdit::Stores(StoresEdit {
                    food: Some(state.food),
                    ..StoresEdit::default()
                }),
            ],
        ] {
            assert_eq!(
                apply_cheat_edits(&mut state, &edits),
                Err(CheatRefusal::NothingToDo),
                "{label} {edits:?}"
            );
        }
        assert_eq!(format!("{state:?}"), before, "{label}");
    }
}

#[test]
fn a_refusal_anywhere_leaves_the_whole_apply_undone() {
    let mut state = town();
    let before = format!("{state:?}");
    assert_eq!(
        apply_cheat_edits(
            &mut state,
            &[
                CheatEdit::Stores(StoresEdit {
                    food: Some(5),
                    ..StoresEdit::default()
                }),
                member_edit(1, stats(31, 1, 1)),
            ]
        ),
        Err(CheatRefusal::OutOfRange { min: 1, max: 30 })
    );
    assert_eq!(format!("{state:?}"), before);
    state.pending_text_hold = Some(PendingTextHold::default());
    assert_eq!(
        apply_cheat_edits(&mut state, &[member_edit(1, stats(9, 9, 9))]),
        Err(CheatRefusal::NotAtIdlePrompt)
    );
}

#[test]
fn edits_touch_no_prng_turn_or_clock() {
    for (label, before) in fixtures() {
        let mut state = before.clone();
        let _ = apply_cheat_edits(
            &mut state,
            &[
                member_edit(0, stats(9, 9, 9)),
                CheatEdit::RecomputeLevel { slot: 1 },
                CheatEdit::Stores(StoresEdit {
                    gold: Some(1),
                    ..StoresEdit::default()
                }),
            ],
        );
        assert_eq!(
            (state.prng_state, state.turn, state.clock),
            (before.prng_state, before.turn, before.clock),
            "{label}"
        );
    }
}
