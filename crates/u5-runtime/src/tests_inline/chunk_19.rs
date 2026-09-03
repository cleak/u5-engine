











    #[test]
    fn pending_prompt_consumes_quit_key_without_turn() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        let prompt = TownArrestPrompt {
            scene_byte: 1,
            floor: 0,
            npc_slot: 1,
        };
        prompted.pending_town_arrest = Some(prompt);

        assert_eq!(
            handle_play_key_input(&mut prompted, 'q', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(prompted.turn, 0);
        assert_eq!((prompted.player.x, prompted.player.y), (4, 5));
        assert_eq!(prompted.pending_town_arrest, Some(prompt));
        assert_eq!(prompted.message, "Surrender? (Y/N).");

        let mut unprompted = world_state(open_world_grid(), 4, 5);

        assert_eq!(
            handle_play_key_input(&mut unprompted, 'q', "", Path::new("")).unwrap(),
            PlayInputDisposition::Quit
        );
        assert_eq!(unprompted.turn, 0);
    }

    /// `input.md` Section 8: a free-text prompt's Enter "terminates the
    /// prompt, returning the accumulated string" to the caller that asked for
    /// it. A typed line is an answer, never a command, so a conversation
    /// keyword that happens to begin with a lowercase `q` must be consumed by
    /// the conversation rather than by the harness-quit arm further down this
    /// dispatcher. `commands.md` Section 9 puts the published program exit
    /// behind Control + `E`'s "Exit to DOS?" confirmation, so no typed line
    /// may end the session, and never silently.
    ///
    /// The guarantee is an ordering one: every session the shell treats as a
    /// typed-line prompt returns from `handle_play_key_input_inner` before the
    /// `key == 'q'` arm is reached. This pins that ordering for the keyword
    /// prompt; moving the quit arm above it would turn a typed `quest` into an
    /// unsaved exit.
    #[test]
    fn typed_conversation_keyword_starting_with_q_never_quits() {
        let enc = |text: &str| text.bytes().map(|byte| byte ^ 0x80).collect::<Vec<u8>>();

        for (key, suffix) in [('q', ""), ('q', "uest")] {
            let raw = vec![
                enc("Maris"),
                enc("a quiet sage"),
                enc("Greetings"),
                enc("I read books"),
                enc("Farewell"),
            ];
            let decoded = vec![
                "Maris".to_string(),
                "a quiet sage".to_string(),
                "Greetings".to_string(),
                "I read books".to_string(),
                "Farewell".to_string(),
            ];
            let mut state = world_state(open_world_grid(), 4, 5);
            state.active_conversation_npc_slot = Some(1);
            state.active_conversation = Some(Box::new(
                crate::conversation_session::ConversationSession::new(raw, decoded),
            ));

            let disposition =
                handle_play_key_input(&mut state, key, suffix, Path::new("")).unwrap();

            assert_eq!(
                disposition,
                PlayInputDisposition::Continue,
                "typed keyword `{key}{suffix}` must be answered, not dispatched"
            );
            assert_eq!(state.turn, 0);
            assert_eq!((state.player.x, state.player.y), (4, 5));
        }
    }

    /// `commands.md` Section 9, the shared pre-dispatch control-code table:
    /// Control + `E` "Prompts "Exit to DOS?"; a yes answer leaves the game,
    /// anything else prints the refusal and continues", and "None of the four
    /// consumes a turn in any mode".
    ///
    /// `dungeon-mode.md` Section 10 fixes which key owns it - "`Q` is the
    /// ordinary save-game route; the "Exit to DOS?" prompt is a Control
    /// binding in the mode-local table, not a letter" - and `input.md`
    /// Section 8 fixes the shape of the answer: "Single-character prompts
    /// (Y/N, a digit, a target-slot letter) run the loop exactly once", so the
    /// prompt does not re-ask.
    #[test]
    fn control_e_prompts_exit_to_dos_and_only_yes_leaves_the_game() {
        let open = |state: &mut PlayState| {
            let disposition =
                handle_play_key_input(state, PLAY_EXIT_TO_DOS_KEY, "", Path::new("")).unwrap();
            assert_eq!(disposition, PlayInputDisposition::Continue);
            assert_eq!(state.message, "Exit to DOS?");
            assert!(state.active_yes_no_prompt.is_some());
            assert_eq!(state.turn, 0, "the binding consumes no turn");
        };

        // A yes answer leaves the game.
        let mut confirmed = world_state(open_world_grid(), 4, 5);
        open(&mut confirmed);
        assert_eq!(
            handle_play_key_input(&mut confirmed, 'Y', "", Path::new("")).unwrap(),
            PlayInputDisposition::Quit
        );
        assert_eq!(confirmed.message, "Yes. Exiting to DOS.");
        assert_eq!(confirmed.turn, 0);

        // Anything else prints the refusal and continues, in one read: the
        // explicit no, the cancel key, and a key the prompt never named.
        for answer in ['N', '\u{1b}', 'K'] {
            let mut declined = world_state(open_world_grid(), 4, 5);
            open(&mut declined);
            assert_eq!(
                handle_play_key_input(&mut declined, answer, "", Path::new("")).unwrap(),
                PlayInputDisposition::Continue,
                "`{answer:?}` must not leave the game"
            );
            assert_eq!(declined.message, "No.", "`{answer:?}` prints the refusal");
            assert!(
                declined.active_yes_no_prompt.is_none(),
                "`{answer:?}` runs the loop exactly once, it does not re-ask"
            );
            assert_eq!(declined.turn, 0);
        }
    }

    /// `dungeon-mode.md` Section 10: "`Q` is the ordinary save-game route; the
    /// "Exit to DOS?" prompt is a Control binding in the mode-local table, not
    /// a letter." `commands.md` Section 4's `Q` row is the route it takes -
    /// "Save game. Routes to the save-game handler, which prompts whether to
    /// save. On `N`, it returns without writing. On `Y`, it writes the save
    /// files, acknowledges completion, and returns to the caller. This letter
    /// is not the DOS-terminate path by itself." Section 5.2 fixes the echo at
    /// exactly `Quit:`, and Section 3 files `Q` under "no action".
    #[test]
    fn dungeon_q_takes_the_save_route_not_the_program_exit() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);

        let disposition = handle_play_key_input(&mut state, 'Q', "", Path::new("")).unwrap();

        assert_eq!(disposition, PlayInputDisposition::Continue);
        assert_eq!(transcript_texts(&state), vec!["Quit:", SAVE_PROMPT_MESSAGE]);
        assert!(state.message_entries()[0].is_command_echo);
        assert!(!state.message_entries()[1].is_command_echo);
        assert_eq!(state.turn, 0);
        assert!(
            matches!(
                state.active_yes_no_prompt.as_ref().map(|session| session.kind),
                Some(YesNoPromptKind::SaveGame)
            ),
            "the dungeon letter must open the save prompt, not the program exit"
        );
        assert_ne!(state.message, "Exit to DOS?");
    }

    /// The companion to the keyword case above, across the rest of the
    /// typed-line set. `input.md` Section 8 names the prompts that read a whole
    /// line - "NPC conversations accept a four- to six-character keyword;
    /// character creation accepts a name; save filenames are typed in full;
    /// hours-to-rest is a small unsigned number" - and every one of them
    /// returns its accumulated string to its own caller. So each session must
    /// consume a leading lowercase `q` itself, before the dispatcher's
    /// harness-quit arm is reached. Any reordering that let one of these fall
    /// through would make a typed word end the session with no save.
    #[test]
    fn every_typed_line_prompt_consumes_a_leading_q_before_the_quit_arm() {
        let yell = |state: &mut PlayState| {
            state.active_yell = Some(crate::z_stats::YellSession {
                buffer: String::new(),
            });
        };
        let shrine = |state: &mut PlayState| {
            state.active_shrine = Some(crate::z_stats::ShrineSession {
                virtue: crate::shrine_virtue::ShrineVirtue::Honesty,
                phase: crate::z_stats::ShrinePhase::Mantra,
                mantra_buffer: String::new(),
            });
        };
        let wishing_well = |state: &mut PlayState| {
            state.active_wishing_well = Some(crate::z_stats::WishingWellSession {
                direction: Direction::North,
                coin_accepted: true,
            });
        };

        let cases: [(&str, &dyn Fn(&mut PlayState)); 3] = [
            ("yell word", &yell),
            ("shrine mantra", &shrine),
            ("wishing well wish", &wishing_well),
        ];

        for (label, install) in cases {
            for (key, suffix) in [('q', ""), ('q', "uest")] {
                let mut state = world_state(open_world_grid(), 4, 5);
                install(&mut state);

                let disposition =
                    handle_play_key_input(&mut state, key, suffix, Path::new("")).unwrap();

                assert_ne!(
                    disposition,
                    PlayInputDisposition::Quit,
                    "{label} must consume the typed `{key}{suffix}` itself"
                );
            }
        }
    }

    #[test]
    fn pending_prompt_suppresses_idle_visual_tick() {
        let mut prompted = world_state(open_world_grid(), 4, 5);
        let prompt = TownArrestPrompt {
            scene_byte: 1,
            floor: 0,
            npc_slot: 1,
        };
        prompted.pending_town_arrest = Some(prompt);
        prompted.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut prompted, '.', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(prompted.turn, 0);
        assert_eq!(prompted.pending_town_arrest, Some(prompt));
        assert_eq!(prompted.animation.frame, 0);
        assert_eq!(prompted.active_objects[1].phase, 0x22);
        assert_eq!(prompted.active_objects[1].tile, 168);
        assert_eq!(prompted.message, "Surrender? (Y/N).");

        let mut unprompted = world_state(open_world_grid(), 4, 5);
        unprompted.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 6,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            handle_play_key_input(&mut unprompted, '.', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(unprompted.turn, 0);
        assert_eq!(unprompted.animation.frame, 1);
        assert_eq!(unprompted.active_objects[1].phase, 0x21);
        assert_eq!(unprompted.active_objects[1].tile, 169);
        assert_eq!(unprompted.message, "Idle animation tick.");
    }

    #[test]
    fn parse_town_rest_bed_entries_accepts_optional_tile_guard() {
        let entries = parse_town_rest_bed_entries("CASTLE:0 0 1 1 55\nCASTLE:0 0 2 1\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownRestBedEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    expected_tile: Some(55),
                },
                TownRestBedEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 2,
                    y: 1,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_rest_bed_entries("CASTLE:0 0 32 1 55\n").is_err());
        assert!(parse_town_rest_bed_entries("DUNGEON:0 0 1 1 55\n").is_err());
    }

    #[test]
    fn town_rest_bed_tile_predicate_matches_public_pair() {
        assert!(!is_town_rest_bed_tile(0x47));
        assert!(is_town_rest_bed_tile(0x48));
        assert!(is_town_rest_bed_tile(0x49));
        assert!(!is_town_rest_bed_tile(0x4a));
    }

    #[test]
    fn parse_town_stair_entries_accepts_direction_and_optional_tile_guard() {
        let entries =
            parse_town_stair_entries("CASTLE:0 0 1 1 UP 55\nCASTLE:0 1 2 1 both\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownStairEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    kind: TownStairKind::Up,
                    expected_tile: Some(55),
                },
                TownStairEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    kind: TownStairKind::Both,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_stair_entries("CASTLE:0 0 32 1 UP 55\n").is_err());
        assert!(parse_town_stair_entries("DUNGEON:0 0 1 1 UP 55\n").is_err());
        assert!(parse_town_stair_entries("CASTLE:0 0 1 1 SIDEWAYS\n").is_err());
        assert!(parse_town_stair_entries("CASTLE:0 0 1 1 UP\nCASTLE:0 0 1 1 DOWN\n").is_err());
    }

    #[test]
    fn parse_town_trap_door_entries_accepts_target_floor_and_optional_tile_guard() {
        let entries =
            parse_town_trap_door_entries("CASTLE:0 0 1 1 -1 55\nCASTLE:0 1 2 1 0\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownTrapDoorEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    to_floor: -1,
                    expected_tile: Some(55),
                },
                TownTrapDoorEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    to_floor: 0,
                    expected_tile: None,
                },
            ]
        );
        assert!(parse_town_trap_door_entries("CASTLE:0 0 32 1 -1 55\n").is_err());
        assert!(parse_town_trap_door_entries("DUNGEON:0 0 1 1 -1 55\n").is_err());
        assert!(parse_town_trap_door_entries("CASTLE:0 0 1 1 0\n").is_err());
        assert!(parse_town_trap_door_entries("CASTLE:0 0 1 1 -1\nCASTLE:0 0 1 1 -2\n").is_err());
    }

    #[test]
    fn parse_town_lock_entries_accepts_magic_and_locked_rows() {
        let entries =
            parse_town_lock_entries("CASTLE:0 0 1 1 185 184\nCASTLE:0 1 2 1 152 186 MAGIC\n").unwrap();

        assert_eq!(
            entries,
            vec![
                TownLockEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 0,
                    x: 1,
                    y: 1,
                    locked_tile: TOWN_DOOR_PLAIN_LOCKED_TILE,
                    unlocked_tile: TOWN_DOOR_PLAIN_UNLOCKED_TILE,
                    kind: TownLockKind::Locked,
                },
                TownLockEntry {
                    scene: Scene::new(17).unwrap(),
                    floor: 1,
                    x: 2,
                    y: 1,
                    locked_tile: TOWN_DOOR_MAGIC_WINDOWED_TILE,
                    unlocked_tile: TOWN_DOOR_WINDOWED_UNLOCKED_TILE,
                    kind: TownLockKind::Magic,
                },
            ]
        );
        assert!(parse_town_lock_entries("DUNGEON:0 0 1 1 185 184\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 32 1 185 184\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 1 1 184 185\n").is_err());
        assert!(parse_town_lock_entries("CASTLE:0 0 1 1 185 185\n").is_err());
        assert!(parse_town_lock_entries(
            "CASTLE:0 0 1 1 185 184\nCASTLE:0 0 1 1 187 186\n"
        )
        .is_err());
    }

    #[test]
    fn town_hole_up_requires_hours_and_clean_bed_without_turn() {
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.hole_up_command(&dir, None).unwrap(),
            MoveOutcome::Observed
        );
        assert!(state.message.contains("how many hours"));
        assert!(state.active_rest.is_some());
        assert_eq!(state.turn, 0);
        state.active_rest = None;

        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 56\n").unwrap();
        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Not here!");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_accepts_native_inn_bed_without_sidecar() {
        let dir = debug_game_dir();
        let _ = fs::remove_file(dir.join(TOWN_REST_BED_TABLE_FILE));
        let mut grid = open_grid();
        grid[32 + 1] = 0x48;
        let mut state = test_state(grid, 1, 1);
        state.area = Area::Town {
            scene: Scene::new(2).unwrap(),
            floor: 0,
        };
        state.clock = GameClock::new(8, 0).unwrap();

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(9, 0).unwrap());
        assert!(state.message.contains("Rested 1 hour at the inn bed"));
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS)
                + u64::from(TOWN_REST_TICKS_PER_HOUR)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_rejects_native_bed_outside_inn_scene() {
        let dir = debug_game_dir();
        let _ = fs::remove_file(dir.join(TOWN_REST_BED_TABLE_FILE));
        let mut grid = open_grid();
        grid[32 + 1] = 0x48;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Not here!");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_sidecar_still_authorizes_custom_bed_cell() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(9, 0).unwrap());
        assert!(state.message.contains("Rested 1 hour at the inn bed"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_accepts_only_single_nonzero_duration_digit() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);

        assert_eq!(
            state.hole_up_command(&dir, Some(10)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, "Rest hours must be in 1..9.");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, GameClock::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_runs_initial_schedule_burst_and_ten_minute_cleanup() {
        assert_eq!(PlayState::town_rest_target_hour(17, 2), 19);
        assert_eq!(PlayState::town_rest_target_hour(23, 1), 1);
        assert_eq!(PlayState::town_rest_target_hour(22, 2), 1);

        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = npc_open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(17, 30).unwrap();
        state.torch_counter = 100;
        state.light_spell_counter = 90;
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
                type_byte: 1,
                dialog_id: 0,
                schedule: [0, 0, 0, 0, 2, 4, 1, 1, 1, 0, 0, 0, 8, 12, 18, 22],
                name: None,
            },
        ];
        state.load_scheduled_npcs(&slots);

        assert_eq!(
            state.hole_up_command(&dir, Some(2)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(19, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS) + 9
        );
        assert_eq!(state.torch_counter, 10);
        assert_eq!(state.light_spell_counter, 0);
        assert_eq!((state.npcs[0].x, state.npcs[0].y), (4, 1));
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (4, 1)
        );
        assert!(state.message.contains("Rested 2 hours"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_stops_when_rest_surface_rejects_after_elapsed_tick() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[1] = 0x87;
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(19, 50).unwrap();
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 5,
            max_hp: 10,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(2)).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.clock, GameClock::new(20, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS) + 1
        );
        assert_eq!(state.grid[32 + 1], 55 ^ 0xdd);
        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[0].mana, 0);
        assert!(state.message.contains("thrown out"));
        assert!(state.message.contains("woke 1 asleep member(s)"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn town_hole_up_advances_time_without_direct_recovery() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 5,
                hp: 3,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 10,
                level: 8,
            },
        ];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(9, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS)
                + u64::from(TOWN_REST_TICKS_PER_HOUR)
        );
        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[1].hp, 3);
        assert_eq!(state.party[1].mana, 5);
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].mana, 0);
        assert!(state.message.contains("recovered 0 HP"));
        assert!(state.message.contains("and 0 MP"));
        assert!(state.message.contains("woke 2 asleep member(s)"));
        let _ = fs::remove_dir_all(dir);
    }

    /// `commands.md §10`: poisoned and dead members are not treated like
    /// healthy sleepers, so the town bed-rest path skips HP gain for a
    /// poisoned member — "recovered 0 HP".
    ///
    /// The poison tick is not hourly. `time.md §5` puts the status/provision
    /// pass "once per ten-minute step of the town-bed rest loop", where "a
    /// member whose status is exactly Poisoned loses **exactly 1 current hit
    /// point** … per member per turn, independently, not a shared roll and
    /// not an hourly effect", and spells out the consequence: "A poisoned
    /// member in a town bed loses six hit points per simulated hour, because
    /// the rest loop steps every ten minutes."
    ///
    /// A member entering the bed on 4 HP therefore runs out inside the hour,
    /// and the shared party-damage path "stores zero, sets that member's
    /// status to Dead".
    #[test]
    fn town_hole_up_poisoned_member_keeps_status_and_skips_hp_recovery() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 90,
            hp: 4,
            max_hp: 12,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[0].hp, 0);
        assert_eq!(state.party[0].mana, 90);
        assert!(state.message.contains("recovered 0 HP"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_requires_hours_without_turn() {
        let mut state = britannia_state(open_world_grid(), 1, 1);

        assert_eq!(
            state.hole_up_command(Path::new(""), None).unwrap(),
            MoveOutcome::Observed
        );
        assert!(state.message.contains("how many hours"));
        assert!(state.active_rest.is_some());
        assert_eq!(state.turn, 0);
        state.active_rest = None;

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(0)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Rest hours must be in 1..9.");
        assert_eq!(state.turn, 0);

        assert_eq!(
            state.hole_up_command(Path::new(""), Some(10)).unwrap(),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Rest hours must be in 1..9.");
        assert_eq!(state.turn, 0);
    }

    #[test]
    fn active_rest_prompt_accepts_duration_without_watch_for_single_member() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_rest.is_some());
        assert_eq!(state.message, REST_HOURS_PROMPT);
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_rest.is_none());
        assert_eq!(state.turn, 12);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_rest_prompt_collects_watch_member() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_rest.is_some());
        assert_eq!(state.message, REST_WATCH_PROMPT);
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, REST_WATCH_MEMBER_PROMPT);

        assert_eq!(
            handle_play_key_input(&mut state, '2', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_rest.is_none());
        assert_eq!(state.turn, 12);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_rest_prompt_invalid_watcher_rests_without_watch() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        handle_play_key_input(&mut state, 'H', "", &dir).unwrap();
        handle_play_key_input(&mut state, '1', "", &dir).unwrap();
        handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
        handle_play_key_input(&mut state, '2', "", &dir).unwrap();

        assert!(state.active_rest.is_none());
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.turn, 12);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_town_hole_up_prompt_accepts_duration_digit() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();

        assert_eq!(
            handle_play_key_input(&mut state, 'H', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_rest.is_some());
        assert_eq!(state.message, REST_HOURS_PROMPT);
        assert_eq!(state.turn, 0);

        assert_eq!(
            handle_play_key_input(&mut state, '1', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert!(state.active_rest.is_none());
        assert_eq!(state.clock, GameClock::new(9, 0).unwrap());
        assert_eq!(
            state.turn,
            u64::from(TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS)
                + u64::from(TOWN_REST_TICKS_PER_HOUR)
        );
        assert!(state.message.contains("Rested 1 hour"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_accepts_valid_inline_watcher_without_changing_ambush_odds() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        assert_eq!(
            state
                .hole_up_command(
                    &dir,
                    InlineRestRequest {
                        hours: Some(1),
                        watcher: Some(1),
                    },
                )
                .unwrap(),
            MoveOutcome::Rested
        );

        assert!(state.message.starts_with("RESTED!"));
        assert_eq!(state.turn, 12);
        assert!(!state.combat_active);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_rejects_non_good_watcher_but_still_rests() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'B',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 8,
            max_hp: 12,
            level: 8,
        });

        assert_eq!(
            state
                .hole_up_command(
                    &dir,
                    InlineRestRequest {
                        hours: Some(1),
                        watcher: Some(1),
                    },
                )
                .unwrap(),
            MoveOutcome::Rested
        );

        assert!(state.message.starts_with("RESTED!"));
        assert_eq!(state.party[1].status, b'P');
        assert_eq!(state.turn, 12);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn wilderness_camp_advances_twelve_five_minute_ticks_per_hour() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(5, 30).unwrap();
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;
        state.torch_counter = 80;
        state.light_spell_counter = 70;

        assert_eq!(
            state.hole_up_command(&dir, Some(2)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.clock, GameClock::new(7, 30).unwrap());
        assert_eq!(state.turn, 24);
        assert_eq!(state.torch_counter, 0);
        assert_eq!(state.light_spell_counter, 0);
        // Twenty-four five-minute ticks wrap the twelve-frame animation clock.
        assert_eq!(state.animation.frame, 0);
        assert_eq!(state.ambient_light, FULL_DAYLIGHT);
        assert!(state.visibility_dirty);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn wilderness_camp_hour_change_probe_reseeds_only_on_zero() {
        let mut state = britannia_state(open_world_grid(), 0, 15);
        state.prng_state = 0x0002;
        let expected_miss_state = u5_prng_advance_state(state.prng_state);
        assert_eq!(
            state.wilderness_camp_hour_change_ambush_row(0x0456),
            None
        );
        assert_eq!(state.prng_state, expected_miss_state);

        // This published regression seed yields zero for random(0,63).
        state.prng_state = 0x00f0;
        let mut expected_host_stream = 0x0456;
        let expected_row = u5_prng_range_u16(&mut expected_host_stream, 0, 7) as u8;
        assert_eq!(
            state.wilderness_camp_hour_change_ambush_row(0x0456),
            Some(expected_row)
        );
        assert_eq!(state.prng_state, expected_host_stream);
    }

    #[test]
    fn world_rest_with_watch_applies_underfoot_damage_sidecar_each_tick() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
            "BRITANNIA 1 1 DROWNING 5\n",
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.prng_state = 0x0002;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 50,
            max_hp: 50,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.turn, 12);
        assert!(state.party[0].hp < 50);
        assert!(
            state
                .message
                .contains("Underfoot world damage triggered 12 tick(s)")
        );
        assert!(state.message.contains("drowning damage"));
        assert!(state.message.contains("party slot 0"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_advances_time_and_wakes_initial_sleepers() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 5,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'A',
                status: b'S',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 3,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'A',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 0,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 3,
                class_byte: b'A',
                status: b'A',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 4,
                hp: 6,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 4,
                class_byte: b'A',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 98,
                hp: 7,
                max_hp: 8,
                level: 8,
            },
        ];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.party[0].hp, 5);
        assert_eq!(state.party[0].mana, 0);
        assert_eq!(state.party[1].hp, 3);
        assert_eq!(state.party[1].mana, 2);
        assert_eq!(state.party[1].status, b'G');
        assert_eq!(state.party[2].status, b'D');
        assert_eq!(state.party[2].hp, 0);
        assert_eq!(state.party[2].mana, 0);
        assert_eq!(state.party[3].status, b'A');
        assert_eq!(state.party[3].hp, 6);
        assert_eq!(state.party[3].mana, 4);
        assert_eq!(state.party[4].status, b'P');
        assert_eq!(state.party[4].hp, 7);
        assert_eq!(state.party[4].mana, 98);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rest_with_watch_poisoned_members_keep_status_and_skip_hp_recovery() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'P',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 98,
            hp: 3,
            max_hp: 12,
            level: 8,
        }];

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, 3);
        assert_eq!(state.party[0].mana, 98);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn completed_long_camp_recovery_applies_guarded_hp_and_class_mana() {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.avatar_stats.intelligence = 22;
        state.party_intelligence = vec![22, 24, 20, 18, 12, 8];
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 0,
                hp: 1,
                max_hp: 2,
                level: 8,
            },
            PartyMember {
                slot: 1,
                class_byte: b'M',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 1,
                hp: 4,
                max_hp: 10,
                level: 8,
            },
            PartyMember {
                slot: 2,
                class_byte: b'B',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 2,
                hp: 5,
                max_hp: 6,
                level: 8,
            },
            PartyMember {
                slot: 3,
                class_byte: b'F',
                status: b'G',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 3,
                hp: 5,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 4,
                class_byte: b'A',
                status: b'P',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 4,
                hp: 5,
                max_hp: 20,
                level: 8,
            },
            PartyMember {
                slot: 5,
                class_byte: b'M',
                status: b'D',
                climb_stat: DEFAULT_CLIMB_STAT,
                mana: 5,
                hp: 0,
                max_hp: 20,
                level: 8,
            },
        ];
        let entry_statuses = [b'G', b'G', b'G', b'G', b'P', b'D'];

        assert_eq!(
            state.apply_completed_long_camp_recovery(5, Some(3), &entry_statuses),
            (0, 0)
        );

        let (hp, mana) = state.apply_completed_long_camp_recovery(6, Some(3), &entry_statuses);

        assert!(hp >= 3);
        assert_eq!(mana, 53);
        assert_eq!(state.party[0].hp, 2);
        assert_eq!(state.party[0].mana, 22);
        assert!((5..=10).contains(&state.party[1].hp));
        assert_eq!(state.party[1].mana, 24);
        assert_eq!(state.party[2].hp, 6);
        assert_eq!(state.party[2].mana, 10);
        assert_eq!(state.party[3].hp, 5);
        assert_eq!(state.party[3].mana, 3);
        assert_eq!(state.party[4].hp, 5);
        assert_eq!(state.party[4].mana, 4);
        assert_eq!(state.party[5].hp, 0);
        assert_eq!(state.party[5].mana, 5);
        // `rest-and-camp.md §5`: the walk arms the cooldown at 14.
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);
    }

    /// `rest-and-camp.md §5`: "A second camp begun inside fourteen game
    /// hours of the previous one therefore prints the no-effect line and
    /// recovers nothing." The engine had no cooldown field and no gate at
    /// all, so a second camp recovered again immediately.
    #[test]
    fn second_camp_inside_the_cooldown_window_recovers_nothing() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 1,
            max_hp: 90,
            level: 8,
        }];
        let entry_statuses = [b'G'];

        assert_eq!(state.camp_cooldown, 0, "a fresh party is not on cooldown");
        let (first_hp, _) = state.apply_completed_long_camp_recovery(6, None, &entry_statuses);
        assert!(first_hp > 0, "the first camp recovers");
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);

        // A second camp, immediately, recovers nothing and does not
        // extend its own lockout.
        let hp_before = state.party[0].hp;
        assert_eq!(
            state.apply_completed_long_camp_recovery(6, None, &entry_statuses),
            (0, 0)
        );
        assert_eq!(state.party[0].hp, hp_before);
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);

        // Thirteen hours in, still blocked; the fourteenth clears it.
        for _ in 0..COMPLETED_LONG_CAMP_COOLDOWN_HOURS - 1 {
            state.camp_cooldown = camp_cooldown_after_hour_rollover(state.camp_cooldown);
        }
        assert_eq!(state.camp_cooldown, 1);
        assert_eq!(
            state.apply_completed_long_camp_recovery(6, None, &entry_statuses),
            (0, 0)
        );
        state.camp_cooldown = camp_cooldown_after_hour_rollover(state.camp_cooldown);
        assert_eq!(state.camp_cooldown, 0);
        let (hp, _) = state.apply_completed_long_camp_recovery(6, None, &entry_statuses);
        assert!(hp > 0, "the window has expired, so the camp recovers again");
    }

    #[test]
    fn cooldown_refusal_advances_time_before_the_gate_and_prints_the_asset_line() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.camp_cooldown = COMPLETED_LONG_CAMP_COOLDOWN_HOURS;
        state.party[0].hp = 1;
        state.party[0].max_hp = 90;
        let mut expected_prng_state = state.prng_state;
        for _ in 0..6 {
            expected_prng_state = u5_prng_advance_state(expected_prng_state);
        }

        assert_eq!(
            state.hole_up_command(&dir, Some(6)).unwrap(),
            MoveOutcome::Rested
        );

        assert!(!state.combat_active, "fixture seed must complete the camp");
        assert_eq!(state.clock, GameClock::new(14, 0).unwrap());
        assert_eq!(state.camp_cooldown, 8);
        assert_eq!(state.party[0].hp, 1);
        assert_eq!(state.prng_state, expected_prng_state);
        assert!(state.message.starts_with("NO EFFECT!"));
        assert!(!state.message.contains("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn camp_attempt_that_expires_the_cooldown_recovers_and_rearms_it() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.camp_cooldown = 6;
        state.party[0].hp = 1;
        state.party[0].max_hp = 90;

        assert_eq!(
            state.hole_up_command(&dir, Some(6)).unwrap(),
            MoveOutcome::Rested
        );

        assert!(!state.combat_active, "fixture seed must complete the camp");
        assert_eq!(state.clock, GameClock::new(14, 0).unwrap());
        assert!(state.party[0].hp > 1);
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

    /// `rest-and-camp.md §5`: the counter is "reduced by one, floored at
    /// zero, at every hour rollover". The rollover is the clock's, not
    /// the rest loop's, so ordinary play drains it too.
    #[test]
    fn camp_cooldown_decays_on_the_clock_hour_rollover() {
        let mut state = test_state(open_grid(), 1, 1);
        state.camp_cooldown = COMPLETED_LONG_CAMP_COOLDOWN_HOURS;
        state.clock = GameClock::new(9, 30).unwrap();

        // Half an hour is not a rollover.
        state.advance_turn_with_minutes(20);
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);

        // Crossing into the next hour is.
        state.advance_turn_with_minutes(20);
        assert_eq!(state.clock.hour, 10);
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS - 1);

        // Floored at zero rather than wrapping.
        state.camp_cooldown = 0;
        state.advance_turn_with_minutes(60);
        assert_eq!(state.camp_cooldown, 0);
    }

    /// `rest-and-camp.md §5`: "The cooldown is armed whether or not the
    /// marker is stamped, and whether or not any member actually
    /// recovered." A camp whose every member is skipped by a per-member
    /// guard still arms it — the arming sits outside the walk, not inside
    /// a success branch.
    #[test]
    fn camp_arms_the_cooldown_even_when_no_member_recovers() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'D',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 0,
            max_hp: 90,
            level: 8,
        }];
        let entry_statuses = [b'D'];

        assert_eq!(
            state.apply_completed_long_camp_recovery(6, None, &entry_statuses),
            (0, 0),
            "a dead member is skipped by a per-member guard"
        );
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);
    }

    /// A duration of five hours or fewer never reaches the recovery walk,
    /// so it does not arm the cooldown either — §5 puts the arming
    /// "after the recovery walk", and the duration is one of the guards
    /// that gates the walk.
    #[test]
    fn short_camp_neither_recovers_nor_arms_the_cooldown() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: DEFAULT_CLIMB_STAT,
            mana: 0,
            hp: 1,
            max_hp: 90,
            level: 8,
        }];
        let entry_statuses = [b'G'];

        assert_eq!(
            state.apply_completed_long_camp_recovery(
                COMPLETED_LONG_CAMP_MIN_HOURS - 1,
                None,
                &entry_statuses
            ),
            (0, 0)
        );
        assert_eq!(state.camp_cooldown, 0);
    }

    #[test]
    fn short_overworld_camp_bypasses_the_apparition_draw() {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        let mut expected_prng_state = state.prng_state;
        for _ in 0..1 {
            expected_prng_state = u5_prng_advance_state(expected_prng_state);
        }

        assert_eq!(
            state.hole_up_command(&dir, Some(1)).unwrap(),
            MoveOutcome::Rested
        );

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.camp_cooldown, 0);
        assert_eq!(state.camp_month_cookie, 0);
        assert!(!state.message.contains("Lord British-in-disguise"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn dungeon_long_camp_suppresses_the_apparition_draw_before_prng() {
        let dir = debug_game_dir();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.prng_state = 0x0002;
        state.party[0].hp = 1;
        state.party[0].max_hp = 90;
        let mut expected_prng_state = state.prng_state;
        // Eighteen five-minute danger checks, then one recovery draw for
        // the fixture's sole living member. The context gate consumes no
        // apparition draw.
        for _ in 0..19 {
            expected_prng_state = u5_prng_advance_state(expected_prng_state);
        }

        assert_eq!(
            state.hole_up_command(&dir, Some(6)).unwrap(),
            MoveOutcome::Rested
        );

        assert!(!state.combat_active, "fixture seed must complete the camp");
        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.camp_cooldown, COMPLETED_LONG_CAMP_COOLDOWN_HOURS);
        assert_eq!(state.camp_month_cookie, 0);
        assert!(!state.message.contains("Lord British-in-disguise"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lord_british_camp_event_recomputes_level_and_prints_karma_verdict() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(KARMA_DAT_FILE),
            karma_bytes(&[
                "low",
                "twenty",
                "forty",
                "sixty",
                "blackthorn-top",
                "camp-top",
            ]),
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 2, 0);
        state.clock = GameClock::new(0, 0).unwrap();
        // With six hourly wilderness interruption probes, this stream reaches
        // the apparition gate and then selects Intelligence and Dexterity.
        state.prng_state = 0x0010;
        state.avatar_stats = AvatarStats {
            strength: 20,
            dexterity: 20,
            intelligence: 18,
        };
        state.party[0].level = 1;
        state.party[0].hp = 10;
        state.party[0].max_hp = 30;
        state.party[0].mana = 0;
        state.party[0].climb_stat = 20;
        state.party.push(PartyMember {
            slot: 1,
            class_byte: b'F',
            status: b'G',
            climb_stat: 20,
            mana: REST_MANA_CAP,
            hp: 10,
            max_hp: 30,
            level: 1,
        });
        state.party_experience = vec![200, 200];
        state.party_strengths = vec![20, 20];
        state.party_intelligence = vec![18, 24];
        state.moral_standing = 80;
        let mut expected_prng_state = state.prng_state;
        // Six hourly danger checks, two long-camp recovery draws, one
        // apparition draw, and two stat-reward draws.
        for _ in 0..11 {
            expected_prng_state = u5_prng_advance_state(expected_prng_state);
        }

        assert_eq!(state.hole_up_command(&dir, Some(6)).unwrap(), MoveOutcome::Rested);

        assert_eq!(state.prng_state, expected_prng_state);
        assert_eq!(state.party[0].level, 3);
        assert_eq!(state.party[0].hp, 90);
        assert_eq!(state.party[0].max_hp, 90);
        assert_eq!(state.avatar_stats.intelligence, 19);
        assert_eq!(state.party[0].climb_stat, 20);
        assert_eq!(state.party[0].mana, 19);
        assert_eq!(state.party[1].level, 3);
        assert_eq!(state.party[1].hp, 90);
        assert_eq!(state.party[1].max_hp, 90);
        assert_eq!(state.party[1].mana, REST_MANA_CAP);
        assert_eq!(state.party_strengths[1], 20);
        assert_eq!(state.party[1].climb_stat, 21);
        assert!(state.visibility_dirty);
        assert!(state.message.contains("Lord British-in-disguise camp event."));
        assert!(state.message.contains("P1 reached level 3 from 200 XP"));
        assert!(state.message.contains("P2 reached level 3 from 200 XP"));
        assert!(state.message.contains("Intelligence reward"));
        assert!(state.message.contains("Dexterity reward"));
        assert!(state.message.contains("Verdict: camp-top"));
        assert_eq!(state.camp_month_cookie, state.clock.month);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lord_british_camp_event_heals_and_cures_living_members_and_refreshes_dead_bard_mana() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(KARMA_DAT_FILE),
            karma_bytes(&["low", "twenty", "forty", "sixty", "blackthorn", "camp"]),
        )
        .unwrap();
        let mut state = britannia_state(open_world_grid(), 2, 0);
        state.avatar_stats.intelligence = 22;
        state.party = vec![
            PartyMember {
                slot: 0,
                class_byte: b'A',
                status: b'P',
                climb_stat: 20,
                mana: 1,
                hp: 3,
                max_hp: 30,
                level: 1,
            },
            PartyMember {
                slot: 1,
                class_byte: b'B',
                status: b'D',
                climb_stat: 20,
                mana: 0,
                hp: 0,
                max_hp: 30,
                level: 1,
            },
        ];
        state.party_experience = vec![0, 0];
        state.party_intelligence = vec![22, 18];

        state.resolve_lord_british_camp_event(Some(&dir)).unwrap();

        assert_eq!(state.party[0].status, b'G');
        assert_eq!(state.party[0].hp, 30);
        assert_eq!(state.party[0].mana, 22);
        assert_eq!(state.party[1].status, b'D');
        assert_eq!(state.party[1].hp, 0);
        assert_eq!(state.party[1].mana, 9);
        let _ = fs::remove_dir_all(dir);
    }

    /// `dungeon-mode.md §11` step 2: the rest wrapper "elapses the accepted
    /// duration by calling the world-clock advance routine repeatedly", so a
    /// one-hour rest must move the clock a full sixty minutes.
    ///
    /// That sits on top of the iteration's own minute. `dungeon-mode.md §15`:
    /// "The single call site sits at the head of each iteration, ahead of the
    /// render-and-poll step and the command dispatch" — pressing `H` is an
    /// iteration, so it costs its minute before the handler runs and the two
    /// call sites add: 01:45 + 1 + 60 = 02:46.
    #[test]
    fn dungeon_h_key_routes_to_rest_with_watch_with_inline_hours() {
        let dir = debug_game_dir();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.clock = GameClock::new(1, 45).unwrap();
        state.torch_counter = 70;
        state.light_spell_counter = 50;

        assert!(
            state
                .handle_dungeon_key_with_inline('h', &dir, Some(1), None, None, None, None)
                .unwrap()
        );

        assert_eq!(state.clock, GameClock::new(2, 46).unwrap());
        assert_eq!(state.turn, 3);
        // `dungeon-mode.md §7`: "A dungeon turn spends one counter unit" and
        // the decay "is part of the world-clock advance call", so the
        // counters age by the same sixty-one minutes the clock did.
        assert_eq!(state.torch_counter, 9);
        assert_eq!(state.light_spell_counter, 0);
        assert!(state.message.starts_with("RESTED!"));
        let _ = fs::remove_dir_all(dir);
    }

