// Exact player-visible result lines and their fifteen-column rendering.
//
// `overworld.md` Section 8.1, `doors-and-z-transitions.md` Section 12.1 and
// `dungeon-mode.md` Section 8.1 publish these as literal transcripts, with
// line breaks and leading blank rows shown literally, plus the rows the
// gameplay message window renders them into. `cleak/u5-spec#181` and
// `RETRACTIONS.md` R320-R326 own the corrections behind them.

/// Rows the gameplay message window draws for one emitted string, blank rows
/// included. The window's own inter-turn blank is not part of this: these are
/// the rows the string itself produces.
fn rendered_rows(text: &str) -> Vec<String> {
    let mut state = test_state(open_grid(), 1, 1);
    state.message_transcript.clear();
    state.emit_message_line(text);
    let log = message_log_from_entries(state.message_entries(), |line| Some(line.to_string()));
    log.lines()
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>()
}

#[test]
fn the_message_window_wraps_at_fifteen_columns() {
    // `overworld.md §8.1`, "About the fifteen-column figure": "The printer's
    // wrap test uses the difference between the window's right and left
    // columns, which is fifteen."
    assert_eq!(MESSAGE_WINDOW_WIDTH, 15);
}

#[test]
fn the_falls_chain_renders_as_the_published_three_rows() {
    // `overworld.md §8.1`: "the full chain reads" F-A-L-L-S!!! / Falling into
    // / underworld!! - "The break inside `Falling into underworld!!` is **not**
    // in the data. It is produced by the message window's word wrap at width
    // fifteen: the printer breaks on the space after `into`."
    assert_eq!(
        rendered_rows(OVERWORLD_FALLS_BANNER),
        vec!["F-A-L-L-S!!!".to_string()]
    );
    assert_eq!(
        rendered_rows(OVERWORLD_FALLS_UNDERWORLD_NARRATION),
        vec!["Falling into".to_string(), "underworld!!".to_string()]
    );
}

#[test]
fn the_whirlpool_banner_costs_one_leading_blank_row() {
    // `overworld.md §8.1`: "`\nWHIRLPOOL!\n` - note the leading line feed,
    // which costs one blank row", rendered as a blank row then the banner,
    // "with the cursor left on the row below".
    assert_eq!(
        rendered_rows(OVERWORLD_WHIRLPOOL_BANNER),
        vec![String::new(), "WHIRLPOOL!".to_string()]
    );
}

#[test]
fn the_dungeon_exit_renders_blank_verb_plane_blank() {
    // `doors-and-z-transitions.md §12.1`: the two prints compose to a blank
    // row, `Exit to`, the plane name, and a trailing blank row. "The break
    // between `Exit to` and the plane name is **not** in the data": the first
    // string leaves the cursor at column eight and the plane name has no space
    // to break on, so the printer restarts the word at column zero.
    assert_eq!(
        rendered_rows(DUNGEON_EXIT_TO_UNDERWORLD_NARRATION),
        vec![
            String::new(),
            "Exit to".to_string(),
            "Underworld!".to_string(),
            String::new(),
        ]
    );
    assert_eq!(
        rendered_rows(DUNGEON_EXIT_TO_BRITANNIA_NARRATION),
        vec![
            String::new(),
            "Exit to".to_string(),
            "Britannia!".to_string(),
            String::new(),
        ]
    );
}

#[test]
fn the_town_exit_renders_its_prompt_answer_and_plane() {
    // `doors-and-z-transitions.md §12.1`, accepted Ararat case. "Note how this
    // differs from the dungeon form: here the break before the plane name
    // **is** in the data, and the blank row sits *before* `Exit to` rather
    // than after the plane name."
    let mut state = test_state(open_grid(), 1, 1);
    state.message_transcript.clear();
    state.emit_message_line(TOWN_EXIT_PROMPT);
    // The prompt does not echo, and its trailing space leaves the cursor
    // mid-row, so the handler's answer word continues that row.
    state.emit_message_line_continuing_row(format!(
        "{TOWN_EXIT_ACCEPTED_NARRATION}{TOWN_EXIT_TO_UNDERWORLD_NARRATION}"
    ));
    let log = message_log_from_entries(state.message_entries(), |line| Some(line.to_string()));
    let rows: Vec<String> = log.lines().iter().map(|line| line.text.clone()).collect();
    assert_eq!(
        rows,
        vec![
            String::new(),
            "Dost thou wish".to_string(),
            "to leave? Yes".to_string(),
            String::new(),
            "Exit to".to_string(),
            "Underworld!".to_string(),
        ]
    );
}

#[test]
fn the_town_exit_refusal_is_the_declined_word_alone() {
    // `§12.1`: "Declined (`N` or Escape): `No` and nothing else."
    assert_eq!(
        rendered_rows(TOWN_EXIT_DECLINED_NARRATION),
        vec!["No".to_string()]
    );
}

#[test]
fn no_dungeon_consequence_line_carries_a_leading_blank_row() {
    // `dungeon-mode.md §8.1`: "The blank line before each message is the
    // loop's, not the string's ... an implementation that adds one per message
    // will double the spacing." Only the two published *find* lines lead with
    // a line feed, and they are Search's, not the loop's.
    for line in [
        DUNGEON_ROOM_ENTRY_NARRATION,
        DUNGEON_SLEEP_FIELD_LINE,
        DUNGEON_POISON_FIELD_LINE,
        DUNGEON_FIRE_FIELD_LINE,
        DUNGEON_PIT_TRAP_LINE,
        DUNGEON_FALLING_LINE,
        DUNGEON_SPLAT_LINE,
        DUNGEON_BOMB_TRAP_LINE,
        DUNGEON_KABOOM_LINE,
        DUNGEON_ELECTRIC_OUCH_LINE,
        DUNGEON_ELECTRIC_FIELD_LINE,
        DUNGEON_LOOK_DARKNESS_REFUSAL,
        DUNGEON_SEARCH_PREAMBLE,
        DUNGEON_SEARCH_A_PIT,
        DUNGEON_SEARCH_HIDDEN_DOOR,
        DUNGEON_SEARCH_NOTHING_OF_NOTE,
        DUNGEON_KLIMB_UP,
        DUNGEON_KLIMB_DOWN,
        DUNGEON_KLIMB_FAILED,
    ] {
        assert!(
            !line.starts_with('\n'),
            "dungeon line {line:?} carries a leading line feed the loop already supplies"
        );
        assert!(
            !rendered_rows(line).first().is_some_and(String::is_empty),
            "dungeon line {line:?} renders a leading blank row"
        );
    }
    // The two exceptions the spec does publish with a leading line feed.
    assert_eq!(
        rendered_rows(DUNGEON_SEARCH_DARKNESS_REFUSAL),
        vec![
            String::new(),
            "You find:".to_string(),
            "darkness.".to_string(),
        ]
    );
    assert_eq!(
        rendered_rows(DUNGEON_LOOK_DARKNESS_REFUSAL),
        vec!["You see:".to_string(), "darkness.".to_string()]
    );
}

#[test]
fn the_published_dungeon_literals_keep_their_exact_punctuation_and_spacing() {
    // `dungeon-mode.md §8.1`: the details that are easy to lose.
    assert_eq!(DUNGEON_FIRE_FIELD_LINE, "Fire!!\n");
    assert_eq!(DUNGEON_SPLAT_LINE, "      ...splat!\n");
    assert_eq!(DUNGEON_KABOOM_LINE, "KABOOM!!\n");
    assert_eq!(DungeonMovementEcho::TurnAround.literal(), "Turn around.");
    assert_eq!(DUNGEON_FOUNTAIN_ACCEPTED, "Yes.  Gulp!\n");
    // "the four trap-tier lines ... **none of which carries a terminal
    // period**".
    for tier in [
        DUNGEON_SEARCH_NO_TRAP,
        DUNGEON_SEARCH_SIMPLE_TRAP,
        DUNGEON_SEARCH_GENERIC_TRAP,
        DUNGEON_SEARCH_COMPLEX_TRAP,
    ] {
        assert!(!tier.trim_end().ends_with('.'), "{tier:?} gained a period");
    }
    // The dungeon chest Search arm selects among exactly those four by the
    // tier its detection roll computes (`dungeon-mode.md` Section 8), so pin
    // the mapping rather than leaving the tier words free to drift.
    assert_eq!(
        dungeon_chest_search_trap_line("no trap"),
        DUNGEON_SEARCH_NO_TRAP
    );
    assert_eq!(
        dungeon_chest_search_trap_line("simple trap"),
        DUNGEON_SEARCH_SIMPLE_TRAP
    );
    assert_eq!(
        dungeon_chest_search_trap_line("trap"),
        DUNGEON_SEARCH_GENERIC_TRAP
    );
    assert_eq!(
        dungeon_chest_search_trap_line("complex trap"),
        DUNGEON_SEARCH_COMPLEX_TRAP
    );
    // `RETRACTIONS.md` R323: both unlit refusals break after the colon.
    assert_eq!(DUNGEON_LOOK_DARKNESS_REFUSAL, "You see:\ndarkness.\n");
    assert_eq!(DUNGEON_SEARCH_DARKNESS_REFUSAL, "\nYou find:\ndarkness.\n");
    // `RETRACTIONS.md` R324: the preamble and the outcome are different lines.
    assert_eq!(DUNGEON_SEARCH_PREAMBLE, "You find:\n");
    assert_eq!(DUNGEON_SEARCH_NOTHING_OF_NOTE, "Nothing of note.\n");
    // `RETRACTIONS.md` R322: `A pit!` on `0x61`, `A hidden door!` on `0xD?`.
    assert_eq!(DUNGEON_SEARCH_A_PIT, "A pit!\n");
    assert_eq!(DUNGEON_SEARCH_HIDDEN_DOOR, "A hidden door!\n");
}

#[test]
fn the_dungeon_search_outcome_table_matches_the_published_classes() {
    // `dungeon-mode.md §8.1` "Search outcomes".
    assert_eq!(
        dungeon_search_outcome_line(0x00),
        Some(DUNGEON_SEARCH_NOTHING_OF_NOTE)
    );
    for ladder in [0x10, 0x20, 0x30] {
        assert_eq!(
            dungeon_search_outcome_line(ladder),
            Some(DUNGEON_SEARCH_NOTHING_ON_LADDER)
        );
    }
    assert_eq!(
        dungeon_search_outcome_line(0x50),
        Some(DUNGEON_SEARCH_NOTHING_ON_FOUNTAIN)
    );
    assert_eq!(
        dungeon_search_outcome_line(0x70),
        Some(DUNGEON_SEARCH_TREASURE)
    );
    assert_eq!(
        dungeon_search_outcome_line(0x90),
        Some(DUNGEON_SEARCH_IMPOSSIBLE_TILE)
    );
    // "for the heavy-door class and for both door-presentation/room classes".
    for door in [0xA0, 0xE0, 0xF0] {
        assert_eq!(
            dungeon_search_outcome_line(door),
            Some(DUNGEON_SEARCH_NOTHING_ON_DOOR)
        );
    }
    assert_eq!(
        dungeon_search_outcome_line(0xB0),
        Some(DUNGEON_SEARCH_NOTHING_ON_WALL)
    );
    // The pit family, the chest class and the two rewriting wall branches own
    // their own arms and are deliberately absent from the table.
    assert_eq!(dungeon_search_outcome_line(0x40), None);
    assert_eq!(dungeon_search_outcome_line(0x61), None);
    assert_eq!(dungeon_search_outcome_line(0xD0), None);
}

#[test]
fn the_dungeon_field_consequence_table_matches_the_published_rows() {
    // `dungeon-mode.md §8.1` "Post-action underfoot consequences". Electric is
    // a movement-time consequence with its own pair, and the generic class is
    // the table's "Any other underfoot byte: nothing" row.
    assert_eq!(
        dungeon_field_consequence_line(DungeonFieldEffect::Sleep),
        Some(DUNGEON_SLEEP_FIELD_LINE)
    );
    assert_eq!(
        dungeon_field_consequence_line(DungeonFieldEffect::PoisonGas),
        Some(DUNGEON_POISON_FIELD_LINE)
    );
    assert_eq!(
        dungeon_field_consequence_line(DungeonFieldEffect::Fire),
        Some(DUNGEON_FIRE_FIELD_LINE)
    );
    assert_eq!(
        dungeon_field_consequence_line(DungeonFieldEffect::Electric),
        None
    );
    assert_eq!(
        dungeon_field_consequence_line(DungeonFieldEffect::Energy),
        None
    );
}

#[test]
fn the_klimb_failure_uses_the_rising_sweep_not_the_falls_descent() {
    // `dungeon-mode.md §8.1`: `Failed!` carries "the short **rising** sweep
    // `audio.md §5.2` tabulates as the 50-update, delay-1 recipe - 800 Hz
    // stepping up to a last tone of 1976 Hz against a nominal 2000 Hz target -
    // the same recipe the spell-failure tail uses".
    let rising = crate::audio::cast_failure_glissando().frequencies();
    assert_eq!(rising.len(), 50);
    assert_eq!(rising[0], 800);
    assert_eq!(*rising.last().unwrap(), 1976);
    // The falls chain's sweep is the other direction and twenty times longer.
    let descent = crate::audio::surface_falls_descent().frequencies();
    assert_eq!(descent.len(), crate::audio::SURFACE_FALLS_DESCENT_UPDATES);
    assert!(descent[0] > *descent.last().unwrap());
}

#[test]
fn the_four_chest_trap_words_are_the_shared_resolvers_own() {
    // `dungeon-mode.md §8.1`, "The chest trap words": a trapped dungeon chest
    // "opens with the shared trap resolver of `systems/traps.md`, which prints
    // exactly one of `ACID!`, `POISON!`, `BOMB!` or `GAS!` before its effect".
    // `traps.md §3` owns the four literals.
    assert_eq!(trap_effect_message(TrapEffect::Acid), "ACID!\n");
    assert_eq!(trap_effect_message(TrapEffect::Poison), "POISON!\n");
    assert_eq!(trap_effect_message(TrapEffect::Bomb), "BOMB!\n");
    assert_eq!(trap_effect_message(TrapEffect::Gas), "GAS!\n");
}

#[test]
fn the_waterfall_family_is_the_falls_trigger_on_either_plane() {
    // `RETRACTIONS.md` R320: the trigger is "the **waterfall tile family
    // `0xD4..0xD7`**, tested in the cell immediately south of the party at the
    // top of the input helper and under the party in the post-action pass, on
    // **either** plane".
    assert_eq!(WATERFALL_TILE_FIRST, 0xD4);
    assert_eq!(WATERFALL_TILE_LAST, 0xD7);
    for tile in WATERFALL_TILE_FIRST..=WATERFALL_TILE_LAST {
        assert!(is_waterfall_tile(tile));
    }
    assert!(!is_waterfall_tile(WATERFALL_TILE_FIRST - 1));
    assert!(!is_waterfall_tile(WATERFALL_TILE_LAST + 1));
    // `(54, 138)` survives only as the landing cell that gates the plane.
    assert!(is_surface_chasm_cell(54, 138));
    assert_eq!(OVERWORLD_FALLS_FORCED_STEPS_SOUTH, 2);
}
