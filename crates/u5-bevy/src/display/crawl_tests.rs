//! The crawl's pure parts (design 2.2.4, 2.2.5, 2.3.8; build plan M1-11):
//! `MessageHistory::sync` over the transcript's real edits, the row
//! classes over rows the runtime's own layout placed, and the history
//! projection.

use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::tlk_control_codes::TLK_SCOPED_PROMPT;
use u5_runtime::{
    CommandEcho, CommandEchoJoin, LiveRowKind, MESSAGE_TRANSCRIPT_CAPACITY, MessageEntry,
    MessageWindowRow, PlayState, TLK_ASK_WHO_PROMPT, TLK_KEYWORD_PROMPT,
    layout_message_window_with_continuation, message_log_from_entries,
};

use super::*;

fn state() -> PlayState {
    test_state(open_grid(), 5, 5)
}

/// The shell's `keep` filter without its name substitution.
fn keep(text: &str) -> Option<String> {
    (!text.trim().is_empty()).then(|| text.to_string())
}

fn texts(entries: impl IntoIterator<Item = MessageEntry>) -> Vec<String> {
    entries.into_iter().map(|entry| entry.text).collect()
}

fn committed_texts(history: &MessageHistory) -> Vec<String> {
    texts(history.committed().iter().cloned())
}

fn echo(state: &mut PlayState, text: &str) {
    state.push_message_entry(text, true);
}

fn say(state: &mut PlayState, text: &str) {
    state.push_message_entry(text, false);
}

/// Submit `line` through the log the way the shell's input line does:
/// typed a character at a time, then cleared.
fn submit(log: &mut TypedLog, line: &str) {
    for end in 1..=line.len() {
        log.observe(&line[..end]);
    }
    log.observe("");
}

/// What the faithful frame places for `state`'s transcript, as the shell
/// calls the layout, with the echo flags and whether a live row (or an
/// inline cursor) was placed.
fn place(
    state: &PlayState,
    live_input: Option<&str>,
    open_prompt: Option<&str>,
) -> (Vec<MessageWindowRow>, Vec<bool>, bool) {
    let entries = state.message_entries();
    let log = message_log_from_entries(entries, keep);
    let layout = layout_message_window_with_continuation(
        &log,
        live_input,
        open_prompt,
        false,
        LiveRowKind::CommandRow,
    );
    let (lines, typed) = tagged_log(entries, None, keep).expect("the mask mirrors the log");
    let echo = echo_row_flags(&layout.rows, &lines, &typed, live_input.is_some());
    let live = live_input.is_some() || layout.inline_cursor.is_some();
    (layout.rows, echo, live)
}

fn row_texts(rows: &[MessageWindowRow]) -> Vec<&str> {
    rows.iter().map(|row| row.text.as_str()).collect()
}

// ---------------------------------------------------------------- sync

#[test]
fn sync_append_commits_nothing() {
    let mut s = state();
    let mut history = MessageHistory::new();
    echo(&mut s, "North");
    history.sync(s.message_entries());
    echo(&mut s, "South");
    say(&mut s, "Blocked!");
    history.sync(s.message_entries());
    assert!(history.committed().is_empty());
    assert_eq!(history.live(), s.message_entries());
}

#[test]
fn sync_an_in_place_last_entry_edit_commits_nothing() {
    // `commit_command_echo`: a direction handler renders its own `Look-`
    // and the direction, which replaces the echo's text in place.
    let mut s = state();
    let mut history = MessageHistory::new();
    say(&mut s, "Grass");
    s.begin_command_echo(CommandEcho {
        text: "Look-",
        join: CommandEchoJoin::AwaitsDirection,
    });
    history.sync(s.message_entries());
    assert_eq!(texts(s.message_entries().to_vec()), ["Grass", "Look-"]);
    s.message = "Look-North".into();
    assert!(s.commit_command_echo());
    assert_eq!(texts(s.message_entries().to_vec()), ["Grass", "Look-North"]);
    history.sync(s.message_entries());
    assert!(history.committed().is_empty());
    assert_eq!(history.live(), s.message_entries());
}

#[test]
fn sync_open_row_completion_commits_nothing() {
    // A prompt left open mid-row (`Dost thou wish to leave? `) and its
    // answer written on that same row.
    let mut s = state();
    let mut history = MessageHistory::new();
    say(&mut s, "Avatar");
    s.emit_message_line("Dost thou wish to leave? ");
    history.sync(s.message_entries());
    s.emit_message_line_continuing_row("Yes\n");
    assert_eq!(
        texts(s.message_entries().to_vec()),
        ["Avatar", "Dost thou wish to leave? Yes"]
    );
    history.sync(s.message_entries());
    assert!(history.committed().is_empty());
    assert_eq!(history.live(), s.message_entries());
}

#[test]
fn sync_overflow_past_64_commits_exactly_what_left() {
    let mut s = state();
    let mut history = MessageHistory::new();
    let mut pushed = Vec::new();
    // One entry a sync, then bursts of several between syncs.
    for step in 0..(MESSAGE_TRANSCRIPT_CAPACITY + 40) {
        let text = format!("line {step}");
        say(&mut s, &text);
        pushed.push(text);
        if step < MESSAGE_TRANSCRIPT_CAPACITY + 10 || step % 7 == 0 {
            history.sync(s.message_entries());
        }
    }
    history.sync(s.message_entries());
    assert_eq!(s.message_entries().len(), MESSAGE_TRANSCRIPT_CAPACITY);
    let left = pushed.len() - MESSAGE_TRANSCRIPT_CAPACITY;
    // Every entry that left is committed, in order, up to the trim.
    let committed = committed_texts(&history);
    assert_eq!(committed.len(), HISTORY_KEPT_ENTRIES.min(left));
    assert_eq!(committed, pushed[left - committed.len()..left]);
    assert_eq!(history.live(), s.message_entries());
}

#[test]
fn sync_overflow_with_an_edit_of_the_newest_entry() {
    let mut s = state();
    for step in 0..MESSAGE_TRANSCRIPT_CAPACITY - 1 {
        say(&mut s, &format!("line {step}"));
    }
    s.emit_message_line("Dost thou wish to leave? ");
    let mut history = MessageHistory::new();
    history.sync(s.message_entries());
    s.emit_message_line_continuing_row("No\n");
    say(&mut s, "Fine.");
    history.sync(s.message_entries());
    assert_eq!(committed_texts(&history), ["line 0"]);
    assert_eq!(history.live(), s.message_entries());
}

#[test]
fn sync_a_clear_commits_the_whole_window() {
    let mut s = state();
    let mut history = MessageHistory::new();
    echo(&mut s, "North");
    say(&mut s, "Blocked!");
    history.sync(s.message_entries());
    s.clear_message_window();
    history.sync(s.message_entries());
    assert_eq!(committed_texts(&history), ["North", "Blocked!"]);
    // Output after the clear starts a fresh mirror.
    say(&mut s, "Thou art absorbed.");
    history.sync(s.message_entries());
    assert_eq!(committed_texts(&history), ["North", "Blocked!"]);
    assert_eq!(texts(history.live().to_vec()), ["Thou art absorbed."]);
}

#[test]
fn sync_a_clear_and_new_output_between_syncs() {
    let mut s = state();
    let mut history = MessageHistory::new();
    echo(&mut s, "North");
    say(&mut s, "Blocked!");
    history.sync(s.message_entries());
    s.clear_message_window();
    say(&mut s, "Fresh");
    history.sync(s.message_entries());
    assert_eq!(committed_texts(&history), ["North", "Blocked!"]);
}

#[test]
fn sync_the_revision_decrement_commits_nothing() {
    // `abort_command_echo` pops the echo and decrements the revision: the
    // echo the window never kept is not history either.
    let mut s = state();
    let mut history = MessageHistory::new();
    say(&mut s, "Grass");
    say(&mut s, "Swamp");
    s.begin_command_echo(CommandEcho {
        text: "Look-",
        join: CommandEchoJoin::AwaitsDirection,
    });
    history.sync(s.message_entries());
    assert_eq!(s.message_entries().len(), 3);
    s.abort_command_echo();
    assert_eq!(texts(s.message_entries().to_vec()), ["Grass", "Swamp"]);
    history.sync(s.message_entries());
    assert!(history.committed().is_empty());
    assert_eq!(history.live(), s.message_entries());
}

#[test]
fn sync_a_withdrawn_bare_marker_commits_nothing() {
    let mut s = state();
    say(&mut s, "Hull weak!");
    let mut entries = s.message_entries().to_vec();
    let mut marker = entries[0].clone();
    marker.text.clear();
    marker.glyphs.clear();
    marker.bare_marker = true;
    entries.push(marker);
    let mut after = entries[0].clone();
    after.text = "Attacked!".into();
    entries.push(after);
    let mut history = MessageHistory::new();
    history.sync(&entries);
    let withdrawn = [entries[0].clone(), entries[2].clone()];
    history.sync(&withdrawn);
    assert!(history.committed().is_empty());
    assert_eq!(history.live(), withdrawn);
}

#[test]
fn sync_the_revision_copies_commit_nothing() {
    // A scene rebuild or a location entry carries the transcript (and its
    // revision) into a new state: the same entries, so nothing changed.
    let mut s = state();
    echo(&mut s, "Enter");
    say(&mut s, "Britain");
    let mut history = MessageHistory::new();
    history.sync(s.message_entries());
    let next = s.clone();
    history.sync(next.message_entries());
    let mut rebuilt = state();
    for entry in next.message_entries() {
        rebuilt.push_message_entry(entry.text.clone(), entry.is_command_echo);
    }
    history.sync(rebuilt.message_entries());
    assert!(history.committed().is_empty());
}

#[test]
fn sync_a_combat_arenas_own_clear() {
    // The arena draws its own transcript, cleared when it opens; the world
    // transcript it replaced has left the window.
    let mut world = state();
    echo(&mut world, "North");
    say(&mut world, "Attacked!");
    let mut history = MessageHistory::new();
    history.sync(world.message_entries());
    let mut arena = world.clone();
    arena.clear_message_window();
    history.sync(arena.message_entries());
    say(&mut arena, "BATS");
    echo(&mut arena, "Attack-");
    history.sync(arena.message_entries());
    assert_eq!(committed_texts(&history), ["North", "Attacked!"]);
    // Its own clear at the end of the fight.
    arena.clear_message_window();
    history.sync(arena.message_entries());
    assert_eq!(
        committed_texts(&history),
        ["North", "Attacked!", "BATS", "Attack-"]
    );
}

#[test]
fn sync_keeps_only_what_the_tallest_window_shows() {
    let mut s = state();
    let mut history = MessageHistory::new();
    for step in 0..400 {
        say(&mut s, &format!("{step}"));
        if step % 3 == 0 {
            s.push_explicit_blank_message_entry();
        }
        history.sync(s.message_entries());
    }
    let showing = history
        .committed()
        .iter()
        .filter(|entry| !entry.text.trim().is_empty())
        .count();
    assert_eq!(showing, HISTORY_KEPT_ENTRIES);
    assert!(history.committed().len() <= 2 * HISTORY_KEPT_ENTRIES);
}

// ------------------------------------------------------------- classes

#[test]
fn echo_rows_are_typed_and_output_is_game() {
    let mut s = state();
    echo(&mut s, "North");
    echo(&mut s, "South");
    say(&mut s, "Blocked!");
    let (rows, echo, live) = place(&s, Some(""), None);
    assert_eq!(row_texts(&rows), ["North", "South", "Blocked!", ""]);
    assert_eq!(
        classify_rows(&rows, &TypedLog::new(), &echo, live),
        [
            RowClass::Typed,
            RowClass::Typed,
            RowClass::Game,
            RowClass::Live
        ]
    );
}

#[test]
fn wrapped_echoes_are_typed_to_their_last_row() {
    let mut s = state();
    echo(&mut s, "Enter the Shrine of the Codex!");
    say(&mut s, "Peace be upon thee");
    let (rows, echo, live) = place(&s, Some(""), None);
    assert_eq!(
        row_texts(&rows),
        [
            "Enter the",
            "Shrine of the",
            "Codex!",
            "Peace be upon",
            "thee",
            ""
        ]
    );
    // Only the first row carries the end cap; its continuations are plain
    // rows, and so are the handler's rows below them (the live row has its
    // own).
    assert_eq!(
        rows.iter().map(|row| row.prefixed).collect::<Vec<_>>(),
        [true, false, false, false, false, true]
    );
    assert_eq!(
        classify_rows(&rows, &TypedLog::new(), &echo, live),
        [
            RowClass::Typed,
            RowClass::Typed,
            RowClass::Typed,
            RowClass::Game,
            RowClass::Game,
            RowClass::Live
        ]
    );
}

#[test]
fn output_directly_under_a_hyphen_echo_is_game() {
    // `Jimmy-` / `Chest unlocked` sit on adjacent rows, and the second is
    // the handler's, not a wrap of the first.
    let mut s = state();
    echo(&mut s, "Jimmy-");
    say(&mut s, "Chest unlocked");
    let (rows, echo, _) = place(&s, None, None);
    assert_eq!(row_texts(&rows), ["Jimmy-", "Chest unlocked"]);
    assert_eq!(
        classify_rows(&rows, &TypedLog::new(), &echo, false),
        [RowClass::Typed, RowClass::Game]
    );
}

/// The mockup's exchange (2.2.4; the dialogue is invented for it).
fn keyword_exchange(typed: &mut TypedLog) -> PlayState {
    let mut s = state();
    echo(&mut s, "Talk-North");
    say(&mut s, "A baker waves.");
    s.emit_message_line(TLK_KEYWORD_PROMPT);
    submit(typed, "bread");
    s.emit_message_line_continuing_row("bread\n");
    say(&mut s, "\"Rye and honey!\"");
    s.emit_message_line(TLK_KEYWORD_PROMPT);
    typed.observe("j");
    typed.observe("jo");
    s
}

#[test]
fn a_keyword_exchange() {
    let mut typed = TypedLog::new();
    let s = keyword_exchange(&mut typed);
    let (rows, echo, live) = place(&s, Some("jo"), Some(":"));
    assert_eq!(
        row_texts(&rows),
        [
            "Talk-North",
            "A baker waves.",
            "Your interest?",
            ":bread",
            "\"Rye and honey!\"",
            "Your interest?",
            ":jo"
        ]
    );
    assert!(live);
    assert_eq!(
        classify_rows(&rows, &typed, &echo, live),
        [
            RowClass::Typed,
            RowClass::Game,
            RowClass::Prompt,
            RowClass::Mixed { from: 1 },
            RowClass::Game,
            RowClass::Prompt,
            RowClass::Live
        ]
    );
}

#[test]
fn a_committed_input_row_without_its_submission_is_game() {
    // Nothing the shell sent ends the row, so its tail is not called typed.
    let mut typed = TypedLog::new();
    let s = keyword_exchange(&mut typed);
    let (rows, echo, live) = place(&s, Some("jo"), Some(":"));
    let classes = classify_rows(&rows, &TypedLog::new(), &echo, live);
    assert_eq!(classes[3], RowClass::Game);
}

#[test]
fn a_scoped_prompt() {
    let mut s = state();
    say(&mut s, "\"Seen it?\"");
    s.emit_message_line(TLK_SCOPED_PROMPT);
    let (rows, echo, live) = place(&s, Some(""), Some(":"));
    assert_eq!(row_texts(&rows), ["\"Seen it?\"", "You respond-", ":"]);
    assert_eq!(
        classify_rows(&rows, &TypedLog::new(), &echo, live),
        [RowClass::Game, RowClass::Prompt, RowClass::Live]
    );
}

#[test]
fn an_ask_who_prompt() {
    let mut s = state();
    s.emit_message_line(TLK_ASK_WHO_PROMPT);
    let (rows, echo, live) = place(&s, Some("Ava"), Some(":"));
    assert_eq!(
        row_texts(&rows),
        ["\"What is thy", "name?\"", "You respond-", ":Ava"]
    );
    // The question stays: only the row directly above `:` is the prompt.
    assert_eq!(
        classify_rows(&rows, &TypedLog::new(), &echo, live),
        [
            RowClass::Game,
            RowClass::Game,
            RowClass::Prompt,
            RowClass::Live
        ]
    );
}

#[test]
fn output_written_on_a_bare_marker_row_is_game() {
    // `vehicles.md §6`: under hoisted sail the step's refusal is written on
    // the marker row, after its end cap. The row is prefixed, but the game
    // wrote it.
    let mut s = state();
    echo(&mut s, "Yell HOIST!");
    let mut entries = s.message_entries().to_vec();
    let mut marker = entries[0].clone();
    marker.text.clear();
    marker.glyphs.clear();
    marker.is_command_echo = false;
    marker.bare_marker = true;
    entries.push(marker);
    let mut refusal = entries[0].clone();
    refusal.is_command_echo = false;
    refusal.text = "BREAKING UP!".into();
    refusal.glyphs = u5_runtime::ordinary_glyphs_from_engine_text("BREAKING UP!");
    entries.push(refusal);
    let log = message_log_from_entries(&entries, keep);
    let layout =
        layout_message_window_with_continuation(&log, None, None, false, LiveRowKind::CommandRow);
    assert_eq!(row_texts(&layout.rows), ["Yell HOIST!", "BREAKING UP!"]);
    assert!(layout.rows[1].prefixed);
    let (lines, typed) = tagged_log(&entries, None, keep).unwrap();
    let echo = echo_row_flags(&layout.rows, &lines, &typed, false);
    assert_eq!(
        classify_rows(&layout.rows, &TypedLog::new(), &echo, false),
        [RowClass::Typed, RowClass::Game]
    );
}

#[test]
fn a_prompt_line_not_above_an_input_row_is_game() {
    let mut s = state();
    say(&mut s, "Your interest?");
    say(&mut s, "Nothing.");
    let (rows, echo, _) = place(&s, None, None);
    assert_eq!(
        classify_rows(&rows, &TypedLog::new(), &echo, false),
        [RowClass::Game, RowClass::Game]
    );
}

#[test]
fn a_spell_name() {
    // The spell row echoes upper-cased; the shell sent what was typed.
    let mut typed = TypedLog::new();
    let mut s = state();
    echo(&mut s, "Cast");
    say(&mut s, "For what spell?");
    submit(&mut typed, "in lor");
    say(&mut s, ":IN LOR");
    say(&mut s, "Success!");
    let (rows, echo, live) = place(&s, Some(""), None);
    assert_eq!(
        row_texts(&rows),
        ["Cast", "For what spell?", ":IN LOR", "Success!", ""]
    );
    assert_eq!(
        classify_rows(&rows, &typed, &echo, live),
        [
            RowClass::Typed,
            RowClass::Game,
            RowClass::Mixed { from: 1 },
            RowClass::Game,
            RowClass::Live
        ]
    );
}

#[test]
fn a_shop_quantity() {
    let mut typed = TypedLog::new();
    let mut s = state();
    say(&mut s, "How much? 5");
    say(&mut s, "Iolo has 5");
    submit(&mut typed, "5");
    let (rows, echo, _) = place(&s, None, None);
    assert_eq!(
        classify_rows(&rows, &typed, &echo, false),
        // The game's own `5` after ordinary text is not the player's.
        [RowClass::Mixed { from: 10 }, RowClass::Game]
    );
}

#[test]
fn a_whole_row_typed_under_a_prompt_is_typed() {
    // The sage's topic is echoed on its own row under the prompt.
    let mut typed = TypedLog::new();
    let mut s = state();
    say(&mut s, "You respond:");
    say(&mut s, "SPIR");
    say(&mut s, "SPIR");
    submit(&mut typed, "spir");
    let (rows, echo, _) = place(&s, None, None);
    assert_eq!(
        classify_rows(&rows, &typed, &echo, false),
        [RowClass::Game, RowClass::Typed, RowClass::Game]
    );
}

#[test]
fn rows_that_did_not_come_from_the_lines_fall_back_to_the_end_cap() {
    let mut s = state();
    echo(&mut s, "North");
    say(&mut s, "Blocked!");
    let (lines, typed) = tagged_log(s.message_entries(), None, keep).unwrap();
    let rows = vec![
        MessageWindowRow {
            row: 22,
            column: 25,
            text: "Elsewhere".into(),
            glyphs: Vec::new(),
            prefixed: true,
            trailing_spaces: 0,
        },
        MessageWindowRow {
            row: 23,
            column: 24,
            text: "Other".into(),
            glyphs: Vec::new(),
            prefixed: false,
            trailing_spaces: 0,
        },
    ];
    assert_eq!(echo_row_flags(&rows, &lines, &typed, false), [true, false]);
    // A live row that happens to read like the newest line is still the
    // live row, not that line.
    let mut rows = place(&s, Some("Blocked!"), None).0;
    assert_eq!(row_texts(&rows), ["North", "Blocked!", "Blocked!"]);
    assert_eq!(
        echo_row_flags(&rows, &lines, &typed, true),
        [true, false, false]
    );
    // Rows that do not end in the log fall back.
    rows[0].text = "Elsewhere".into();
    assert_eq!(
        echo_row_flags(&rows, &lines, &typed, true),
        [true, false, true]
    );
}

#[test]
fn the_tagged_log_is_the_shells_log() {
    let mut s = state();
    echo(&mut s, "Enter the Shrine of the Codex!");
    s.push_runic_message_entry("AN RUNE ROW");
    s.push_centered_message_entry("BATS");
    s.push_explicit_blank_message_entry();
    say(&mut s, "Peace");
    let mut calls = Vec::new();
    let (lines, typed) = tagged_log(s.message_entries(), Some("Pending"), |text| {
        calls.push(text.to_string());
        keep(text)
    })
    .unwrap();
    let mut log = message_log_from_entries(s.message_entries(), keep);
    log.push_output("Pending");
    assert_eq!(lines, log.lines());
    assert_eq!(
        typed,
        [true, true, true, false, false, false, false, false],
        "{lines:#?}"
    );
    // The shell's renderer is consulted once per entry that reaches it.
    assert_eq!(
        calls,
        [
            "Enter the Shrine of the Codex!",
            "AN RUNE ROW",
            "BATS",
            "Peace"
        ]
    );
}

#[test]
fn a_renderer_that_changes_the_text_still_lines_up() {
    // Name substitution changes a row's width, and so its wrap.
    let mut s = state();
    echo(&mut s, "Talk-");
    say(&mut s, "@ says hello to thee");
    let render = |text: &str| keep(text).map(|text| text.replace('@', "Shamino the Ranger"));
    let (lines, typed) = tagged_log(s.message_entries(), None, render).unwrap();
    let log = message_log_from_entries(s.message_entries(), render);
    assert_eq!(lines, log.lines());
    assert_eq!(typed, [true, false, false, false]);
}

// ------------------------------------------------------------- history

#[test]
fn a_prompt_row_never_enters_the_history() {
    let mut typed = TypedLog::new();
    let mut s = keyword_exchange(&mut typed);
    typed.observe("");
    s.emit_message_line_continuing_row("jo\n");
    say(&mut s, "Bye.");
    let mut history = MessageHistory::new();
    history.sync(s.message_entries());
    s.clear_message_window();
    history.sync(s.message_entries());
    let rows = history.older_rows(&typed, OPEN_WINDOW_ROWS_MAX, keep);
    assert_eq!(
        rows.iter()
            .map(|row| (row.text.as_str(), row.class))
            .collect::<Vec<_>>(),
        [
            ("Talk-North", RowClass::Typed),
            ("A baker waves.", RowClass::Game),
            (":bread", RowClass::Mixed { from: 1 }),
            ("\"Rye and honey!\"", RowClass::Game),
            (":jo", RowClass::Mixed { from: 1 }),
            ("Bye.", RowClass::Game),
        ]
    );
    assert!(rows.iter().all(|row| row.text != "Your interest?"));
}

#[test]
fn the_history_fills_the_tallest_window_past_the_logs_cap() {
    let mut s = state();
    let mut history = MessageHistory::new();
    for step in 0..30 {
        echo(&mut s, &format!("Step {step}"));
        say(&mut s, &format!("Result {step}"));
    }
    history.sync(s.message_entries());
    s.clear_message_window();
    history.sync(s.message_entries());
    let rows = history.older_rows(&TypedLog::new(), OPEN_WINDOW_ROWS_MAX, keep);
    assert_eq!(rows.len(), OPEN_WINDOW_ROWS_MAX);
    // The newest seventeen rows, oldest first, with no blank between turns.
    let want: Vec<(String, RowClass)> = (0..30)
        .flat_map(|step| {
            [
                (format!("Step {step}"), RowClass::Typed),
                (format!("Result {step}"), RowClass::Game),
            ]
        })
        .collect();
    let got: Vec<(String, RowClass)> = rows.into_iter().map(|row| (row.text, row.class)).collect();
    assert_eq!(got, want[want.len() - OPEN_WINDOW_ROWS_MAX..]);
}

#[test]
fn history_projection_matches_the_window_over_a_run_boundary() {
    // A long wrapped entry next to short ones: every run's rows must equal
    // the rows one projection gives where that projection still holds them.
    let mut s = state();
    for step in 0..8 {
        echo(&mut s, "Look-North");
        say(
            &mut s,
            &format!("A long row of text number {step} that wraps over three rows"),
        );
    }
    let rows = project_history_rows(s.message_entries(), &TypedLog::new(), 11, keep);
    let log = message_log_from_entries(s.message_entries(), keep);
    let window: Vec<&str> = log
        .lines()
        .iter()
        .map(|line| line.text.as_str())
        .filter(|text| !text.is_empty())
        .collect();
    let got: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
    assert_eq!(got, window[window.len() - 11..]);
}

#[test]
fn the_typed_log_records_submissions_only() {
    let mut log = TypedLog::new();
    submit(&mut log, "bread");
    log.observe("x");
    log.observe("xy");
    assert_eq!(log.current(), "xy");
    submit(&mut log, "job");
    assert_eq!(log.submitted().collect::<Vec<_>>(), ["job", "bread"]);
    for step in 0..TYPED_LOG_DEPTH + 5 {
        log.record(format!("{step}"));
    }
    assert_eq!(log.submitted().count(), TYPED_LOG_DEPTH);
}

#[test]
fn the_prompt_lines_come_from_the_literals() {
    assert_eq!(prompt_lines(), ["Your interest?", "You respond-"]);
}
