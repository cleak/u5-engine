// Regressions for the pass-11 exploratory-playtest findings (p11text, S##).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// A Britannia state on a Honesty shrine marker, mid-meditation at the
/// first `Mantra:` ask.
fn p11text_shrine_at_first_mantra(dir: &Path) -> PlayState {
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
    let mut state = britannia_state(grid, 10, 20);
    handle_play_key_input(&mut state, 'E', "", dir).unwrap();
    state.flush_staged_narration();
    handle_play_key_input(&mut state, 'H', "ONESTY\r", dir).unwrap();
    state.flush_staged_narration();
    state.adopt_shrine_prompt_row();
    state
}

/// S18: `karma.md §7` ends the meditation on a blank mantra "on the current
/// prompt row", and `text-output.md §10.4` has the next poll's feed close
/// that row. Nothing lets the closed meditation suppress later feeds:
/// `commands.md §5.8` is `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!` for
/// every later command, not just the first one.
#[test]
fn playtest_s18_blank_mantra_does_not_leave_the_row_open_for_later_commands() {
    let dir = debug_game_dir();
    let mut state = p11text_shrine_at_first_mantra(&dir);
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    assert!(state.active_shrine.is_none());
    state.flush_staged_narration();
    for _ in 0..2 {
        handle_play_key_input(&mut state, 'G', "", &dir).unwrap();
        handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", &dir).unwrap();
        state.flush_staged_narration();
        let tail = p4window_rows_from(&p4window_rows(&state), ">Get-North");
        assert_eq!(
            tail.get(..3).map(<[String]>::to_vec),
            Some(vec![
                ">Get-North".to_string(),
                String::new(),
                "Nothing to get!".to_string()
            ]),
            "{:?}",
            p4window_rows(&state)
        );
    }
    let _ = fs::remove_dir_all(dir);
}

/// S19: `karma.md §7`, "Both key waits precede the closing record": "the
/// closing line wrapped across the message strip, one blank row, and then
/// the prompt row: the ordinary command-boundary blank of
/// systems/text-output.md Section 10.4 is spent here like anywhere else."
/// The quest record's open-row one-shot belongs to the second key wait and
/// must not survive it onto the next command.
#[test]
fn playtest_s19_closing_record_key_spends_the_quest_rows_one_shot() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 5, 5);
    let mut session = ShrineSession::new(ShrineVirtue::Honesty);
    session.phase = ShrinePhase::AltarQuest;
    state.active_shrine = Some(session);
    // What the `AltarAnnouncement` arm leaves for the second key wait.
    state.surface_command_row_follows_history = true;
    let _ = state.step_active_shrine(' ', "", &dir);
    assert!(state.active_shrine.is_none());
    assert!(!state.surface_command_row_follows_history);
    let _ = fs::remove_dir_all(dir);
}

/// S23 read `magic.md §6` step 4 - "The `Type M to mix:` instruction line
/// carries no trailing feed of its own" - with `text-output.md §10.4` as an
/// Escape at the reagent list leaving no blank row under the help. The
/// paired `magic-mix` captures contradict it: every stock run (rc13
/// 2026-10-25, rc14 2026-10-26) shows `Type M to mix:`, one blank row, then
/// the prompt marker, and rc13's engine matched that before S23 changed it.
/// The capture wins; the spec question is noted. Escape sets the row flag
/// itself, whatever printed before it.
#[test]
fn playtest_s23_escape_at_the_reagent_list_leaves_one_blank_under_the_help() {
    for toggles in [&['\r'][..], &['\r', 'j', '\r'][..]] {
        let dir = debug_game_dir();
        let mut state = p4window_world_state();
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 9;
        state.reagents[REAGENT_GINSENG] = 9;
        p4window_keys(&mut state, &dir, &['M', 'I', 'L', '\r']);
        assert!(state.active_mix.is_some(), "{:?}", state.message);
        p4window_keys(&mut state, &dir, toggles);
        state.message_row_open_mid_line = true;
        p4window_keys(&mut state, &dir, &['\u{1b}']);
        assert!(state.active_mix.is_none());
        assert!(!state.message_row_open_mid_line);
        let rows = p4window_rows(&state);
        let help = rows
            .iter()
            .rposition(|row| row == "Type M to mix:")
            .unwrap_or_else(|| panic!("help on screen: {rows:?}"));
        assert_eq!(rows.get(help + 1).map(String::as_str), Some(""), "{rows:?}");
        assert_eq!(rows.get(help + 2).map(String::as_str), Some(">"), "{rows:?}");
        let _ = fs::remove_dir_all(dir);
    }
}

/// S17/S21: `commands.md §5.5`, overworld Enter class table: "anything else
/// | `What?\n`, producing `Enter_What?\n`; no action", and §3: "Overworld:
/// only `Enter What?` on an unrecognised tile ... Town and dungeon: `Enter
/// what?`". Plain grass is the commonest unrecognised tile.
#[test]
fn playtest_s17_overworld_enter_on_grass_prints_capital_what() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 10, 20);
    state.begin_command_echo_for(Command::Enter);
    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Blocked
    );
    state.commit_command_echo();
    assert_eq!(state.turn, 0);
    assert_eq!(
        playtest_transcript_rows(&state).last().map(String::as_str),
        Some("Enter What?")
    );
    let _ = fs::remove_dir_all(dir);
}

/// S04: `inventory.md §4.7` field widths (issue #302 Q2): "magic points
/// take two cells, space-padded (`Magic:_7`, `Magic:25`)", and the executed
/// row 8 is `____Magic:_7`. A two-digit value follows the colon directly.
#[test]
fn playtest_s04_attribute_page_magic_points_take_two_cells() {
    let mut state = test_state(open_grid(), 5, 5);
    for (mana, expected) in [(7, "    Magic: 7"), (25, "    Magic:25"), (99, "    Magic:99")] {
        state.party[0].mana = mana;
        assert_eq!(state.z_stats_for_party(0), MoveOutcome::Observed);
        let session = state.active_z_stats.clone().expect("a live page");
        assert_eq!(state.z_stats_panel_rows(&session)[7], expected);
        state.active_z_stats = None;
    }
}

/// S08: `magic.md §8`: "The entire coordinate pair, including its
/// punctuation and separating space, uses the runic font (RUNES.CH) ... In
/// Wis ... invokes the same formatter directly." The Sextant's row was
/// already runic; Locate's was logged in the text font.
#[test]
fn playtest_s08_in_wis_prints_its_coordinate_pair_in_runes() {
    let mut state = britannia_state(open_world_grid(), 0x23, 0xaf);
    state.spell_charges[IN_WIS_SPELL_INDEX] = 1;
    state.party[0].mana = IN_WIS_COST;
    state.party[0].level = IN_WIS_COST;
    handle_play_key_input(&mut state, 'C', "1WI", Path::new("")).unwrap();
    state.flush_message_slot();
    let entry = state
        .message_transcript
        .iter()
        .rev()
        .find(|entry| entry.text == "K'P\", C'D\"")
        .unwrap_or_else(|| panic!("{:?}", playtest_transcript_rows(&state)));
    assert!(!entry.glyphs.is_empty());
    assert!(
        entry
            .glyphs
            .iter()
            .all(|glyph| glyph.font == TlkGlyphFont::Runic),
        "{:?}",
        entry.glyphs
    );
    // Logged once, with the printer's leading feed still its own row.
    let rows = playtest_transcript_rows(&state);
    assert_eq!(rows.iter().filter(|row| row.contains("K'P")).count(), 1, "{rows:?}");
    assert!(!state.message_row_open_mid_line);
}

/// S09: `formats/signs-dat.md §6`: the Yew poster's "border letters above
/// are the resident sign-border glyph fragments used by the original text
/// layer, not prose", and the placard is given "in the runic font, in the
/// same code-point notation as the poster above". The frame is runic cells,
/// as on the placard; the letters `a`..`g` must not print as Roman text.
#[test]
fn playtest_s09_yew_wanted_poster_frame_is_drawn_in_the_runic_font() {
    let state = test_state(open_grid(), 5, 5);
    let poster = state
        .sign_message_at(None, 4, 0, 21, 17)
        .unwrap()
        .expect("the fixed poster");
    let rows: Vec<Vec<TlkRenderedGlyph>> = poster
        .rendered_lines()
        .map(<[TlkRenderedGlyph]>::to_vec)
        .collect();
    assert_eq!(rows.len(), 9);
    let runic = |glyph: &TlkRenderedGlyph| glyph.font == TlkGlyphFont::Runic;
    assert!(rows[0].iter().all(runic), "{:?}", rows[0]);
    assert!(rows[8].iter().all(runic), "{:?}", rows[8]);
    for row in &rows[1..8] {
        assert!(runic(row.first().unwrap()) && runic(row.last().unwrap()), "{row:?}");
        assert_eq!(row.first().unwrap().byte, b'g');
    }
    // The poster's own words stay in the text font.
    let wanted: Vec<u8> = rows[1][1..rows[1].len() - 1].iter().map(|glyph| glyph.byte).collect();
    assert_eq!(wanted, b"   Wanted:   ");
    assert!(rows[1][4..11].iter().all(|glyph| glyph.font == TlkGlyphFont::Ordinary));
    assert!(poster.text.starts_with("abbbbbbbbbbbbbc\ng   Wanted:   g\n"));
}

/// S20: `text-output.md §10.2`: the arena's turn handler "emits the pair
/// after the turn banner on every pass, before ... the key read, with no
/// gate", and `commands.md §5.1`: the marker "persists in the
/// message-window scrollback exactly where it was drawn". A party-side Z
/// "opens that character's own sheet silently, with no prompt" (`combat.md
/// §8`), so `\nStatus:_`'s feed closes the marker row; it never erases it
/// (§10.4).
#[test]
fn playtest_s20_combat_z_keeps_the_marker_the_turn_handler_drew() {
    let game_dir = Path::new(".");
    let mut state = combat_player_command_state(8, 5);
    state.pending_combat_actor_slot = Some(0);
    state.emit_combat_print("\nAvatar, armed\nwith bare hands:\n");
    state.combat_prompt_row_opened_by_banner = true;
    let before = p4window_rows(&state);
    assert_eq!(
        before.iter().rev().find(|row| !row.is_empty()).map(String::as_str),
        Some(">"),
        "marker row waits for the key: {before:?}"
    );

    handle_play_key_input(&mut state, 'Z', "", game_dir).unwrap();
    assert!(state.active_z_stats.is_some());
    let rows = p4window_rows(&state);
    let status = rows
        .iter()
        .rposition(|row| row.starts_with("Status:"))
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[status - 1], ">", "{rows:?}");
    assert_eq!(rows[status - 2], "with bare hands:", "{rows:?}");

    // Escape completes the prompt's own row; the marker stays drawn.
    handle_play_key_input(&mut state, '\u{1b}', "", game_dir).unwrap();
    let rows = p4window_rows(&state);
    let done = rows
        .iter()
        .rposition(|row| row == &format!("{Z_STATS_STATUS_PROMPT}{Z_STATS_DONE_MESSAGE}"))
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[done - 1], ">", "{rows:?}");
}

/// S24: `commands.md §5.4`: the direction prompt "accepts only the four
/// directions and Space" and appends the chosen word to the open line, and
/// `§5.8` gives `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!`. The terminal
/// harness's inline direction form must give the rows the prompt gives:
/// the same word and blank for a cardinal, and no diagonal look or push.
#[test]
fn playtest_s24_inline_direction_forms_match_the_prompt_form() {
    let dir = debug_game_dir();
    for (key, alias) in [
        ('G', 'w'),
        ('S', 'w'),
        ('O', 'w'),
        ('J', 'w'),
        ('P', 'w'),
        ('g', 'w'),
        ('L', 'y'),
        ('l', 'y'),
        ('P', 'y'),
    ] {
        let mut inline = p4window_world_state();
        handle_play_key_input(&mut inline, key, &alias.to_string(), &dir).unwrap();
        inline.flush_staged_narration();

        let mut prompted = p4window_world_state();
        handle_play_key_input(&mut prompted, key.to_ascii_uppercase(), "", &dir).unwrap();
        let code = match alias {
            'w' => char::from(INPUT_CODE_NORTH),
            _ => char::from(INPUT_CODE_NORTHWEST),
        };
        handle_play_key_input(&mut prompted, code, "", &dir).unwrap();
        prompted.flush_staged_narration();

        assert_eq!(
            playtest_transcript_rows(&inline),
            playtest_transcript_rows(&prompted),
            "{key}{alias}"
        );
        assert_eq!(p4window_rows(&inline), p4window_rows(&prompted), "{key}{alias}");
        assert_eq!(inline.turn, prompted.turn, "{key}{alias}");
        assert_eq!(
            inline.active_direction_prompt.is_some(),
            alias == 'y',
            "{key}{alias}: a diagonal leaves the prompt waiting"
        );
    }
    let mut get = p4window_world_state();
    handle_play_key_input(&mut get, 'G', "w", &dir).unwrap();
    assert_eq!(
        p4window_rows_from(&p4window_rows(&get), ">Get-North").get(..3).map(<[String]>::to_vec),
        Some(vec![">Get-North".to_string(), String::new(), "Nothing to get!".to_string()]),
    );
    let _ = fs::remove_dir_all(dir);
}

/// S07: `dungeon-mode.md §6.4` (u5-spec#298 Q12): "A `0xB?` wall one cell
/// ahead whose low nibble is non-zero routes to the per-dungeon chamber-name
/// banner handler", drawn as "a solid white block with black runes exactly
/// its rows high and its width in cells wide" at origin `(8 x column, 8 x
/// row)`. "Confirmed on the original in EGA: facing Deceit's first-level
/// banner wall showed a solid white block with black runes covering exactly
/// x 56..135, y 80..95." Only when `n` is within the dungeon's banner count.
#[test]
fn playtest_s07_dungeon_chamber_banner_draws_its_inverse_block() {
    // A blank runic font: every cell is all background, so the inverse
    // block is solid white and its extent is exact.
    let blank_runes = parse_ch_font(&vec![0; CH_FONT_LEN], RUNES_CH_FILE).unwrap();
    let mut atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
    atlas.runic_font = Some(blank_runes);
    let white = TileGraphicsDepth::Ega16.pixel_limit() - 1;
    let render = |record: u8, wall: u8| {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 5, 4)] = wall;
        let mut state = dungeon_state(grid, 0, 4, 4);
        state.area = Area::Dungeon {
            scene: DungeonScene::from_record(record).unwrap(),
            level: 0,
        };
        state.player.facing = Direction::East;
        state.torch_counter = 10;
        state.render_top_down_frame(VIEWPORT_PLAYER_ROW, &atlas).unwrap().unwrap()
    };
    let white_box = |viewport: &TileViewport| {
        let mut bounds: Option<(usize, usize, usize, usize)> = None;
        for y in 0..viewport.height {
            for x in 0..viewport.width {
                if viewport.pixels[y * viewport.width + x] == white {
                    let (sx, sy) = (x + VIEWPORT_ORIGIN_X, y + VIEWPORT_ORIGIN_Y);
                    bounds = Some(match bounds {
                        None => (sx, sx, sy, sy),
                        Some((x0, x1, y0, y1)) => (x0.min(sx), x1.max(sx), y0.min(sy), y1.max(sy)),
                    });
                }
            }
        }
        bounds
    };
    // Deceit (record 0), banner 1: BOTTOMLESS / PIT at (7, 10), 2 x 10.
    assert_eq!(white_box(&render(0, 0xb1)), Some((56, 135, 80, 95)));
    // Covetous (record 4), banner 2: UPPER / CRYPTS at (9, 10), 2 x 6.
    assert_eq!(white_box(&render(4, 0xb2)), Some((72, 119, 80, 95)));
    // A plain wall, an index past the dungeon's count, and Destard (none).
    assert_eq!(white_box(&render(0, 0xb0)), None);
    assert_eq!(white_box(&render(0, 0xb2)), None);
    assert_eq!(white_box(&render(2, 0xb1)), None);
    // Every published banner is its published size.
    for (record, n, rows, cells) in [
        (0, 1, 2, 10),
        (1, 1, 3, 7),
        (3, 1, 2, 9),
        (4, 1, 1, 8),
        (4, 2, 2, 6),
        (4, 3, 2, 6),
        (4, 4, 2, 7),
        (6, 1, 2, 10),
    ] {
        let banner = dungeon_chamber_banner(record, n).unwrap();
        assert_eq!(banner.rows.len(), rows, "{record}/{n}");
        assert!(banner.rows.iter().all(|row| row.len() == cells), "{record}/{n}");
    }
}
