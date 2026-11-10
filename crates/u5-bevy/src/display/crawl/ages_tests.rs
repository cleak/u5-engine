//! The crawl's ages (design 2.2.4, 2.3.8): the tiers, a scripted walk (the
//! mockup's F sequence), the clock frozen in a conversation, and the
//! alignment across an in-place edit, a withdrawal, an overflow and a clear.

use u5_runtime::MessageEntry;

use super::*;

fn entry(text: &str, echo: bool) -> MessageEntry {
    MessageEntry {
        text: text.to_string(),
        producer_blank: false,
        glyphs: Vec::new(),
        is_command_echo: echo,
        continues_open_row: false,
        prompt_pair_suppressed: false,
        leaves_row_open: false,
        centered: false,
        explicit_blank: false,
        prompt_column_row: false,
        bare_marker: false,
        starts_fresh_row: false,
    }
}

fn ages(crawl: &CrawlAges, now: &[MessageEntry]) -> Vec<u64> {
    (0..now.len()).map(|index| crawl.age_of(index)).collect()
}

#[test]
fn the_tiers_are_fresh_aged_oldest_then_gone() {
    let tiers: Vec<Option<usize>> = (0..7).map(tier_of).collect();
    assert_eq!(
        tiers,
        [Some(0), Some(1), Some(1), Some(2), Some(2), None, None]
    );
}

/// make_c04f.py's F sequence: one command per turn from turn 1, with
/// `Slow progress!` on turn 3, then four idle turns.
#[test]
fn a_scripted_walk_ages_each_row_by_the_turn_it_appeared() {
    let walk = ["North", "East", "East", "North", "North"];
    let mut crawl = CrawlAges::new();
    let mut now: Vec<MessageEntry> = Vec::new();
    crawl.sync(&now, 0, false);
    let mut seen = Vec::new();
    for turn in 1..=9u64 {
        if let Some(command) = walk.get(turn as usize - 1) {
            now.push(entry(command, true));
            if turn == 3 {
                now.push(entry("Slow progress!", false));
            }
        }
        crawl.sync(&now, turn, false);
        seen.push(ages(&crawl, &now));
    }
    assert_eq!(seen[0], [0]);
    assert_eq!(seen[2], [2, 1, 0, 0]);
    assert_eq!(seen[4], [4, 3, 2, 2, 1, 0]);
    // Turn 9: everything but the last command has aged out.
    let last = &seen[8];
    let tiers: Vec<Option<usize>> = last.iter().map(|age| tier_of(*age)).collect();
    assert_eq!(last, &[8, 7, 6, 6, 5, 4]);
    assert_eq!(tiers, [None, None, None, None, None, Some(2)]);
}

#[test]
fn ages_stand_still_while_a_conversation_is_open() {
    let mut crawl = CrawlAges::new();
    let mut now = vec![entry("Talk-North", true), entry("A baker waves.", false)];
    crawl.sync(&now, 10, true);
    now.push(entry(":bread", false));
    crawl.sync(&now, 11, true);
    crawl.sync(&now, 14, true);
    assert_eq!(ages(&crawl, &now), [0, 0, 0]);
    assert_eq!(crawl.clock(), 0);
    // The conversation ends; ageing resumes from the next turn.
    crawl.sync(&now, 14, false);
    assert_eq!(ages(&crawl, &now), [0, 0, 0]);
    crawl.sync(&now, 15, false);
    assert_eq!(ages(&crawl, &now), [1, 1, 1]);
}

#[test]
fn an_edit_in_place_keeps_the_first_appearance() {
    let mut crawl = CrawlAges::new();
    let mut now = vec![entry("North", true), entry("Talk-", true)];
    crawl.sync(&now, 1, false);
    crawl.sync(&now, 2, false);
    now[1] = entry("Talk-East", true);
    crawl.sync(&now, 2, false);
    assert_eq!(ages(&crawl, &now), [1, 1]);
    now.push(entry("Funny, no response!", false));
    crawl.sync(&now, 2, false);
    assert_eq!(ages(&crawl, &now), [1, 1, 0]);
}

#[test]
fn a_withdrawn_echo_and_an_overflow_keep_the_rest() {
    let mut crawl = CrawlAges::new();
    let mut now: Vec<MessageEntry> = (0..64)
        .map(|index| entry(&format!("line {index}"), false))
        .collect();
    crawl.sync(&now, 1, false);
    crawl.sync(&now, 3, false);
    // Two new entries push two old ones out of the front.
    now.drain(..2);
    now.push(entry("Blocked!", false));
    now.push(entry("North", true));
    crawl.sync(&now, 3, false);
    let got = ages(&crawl, &now);
    assert!(got[..62].iter().all(|age| *age == 2), "{got:?}");
    assert_eq!(&got[62..], &[0, 0]);
    // The newest echo taken back: the rest keep their turns.
    let mut withdrawn = now.clone();
    withdrawn.pop();
    crawl.sync(&withdrawn, 4, false);
    let got = ages(&crawl, &withdrawn);
    assert_eq!(got[61], 3);
    assert_eq!(got[62], 1, "Blocked! keeps its turn");
}

#[test]
fn a_clear_starts_every_row_afresh() {
    let mut crawl = CrawlAges::new();
    let now = vec![entry("North", true), entry("East", true)];
    crawl.sync(&now, 1, false);
    crawl.sync(&now, 4, false);
    assert_eq!(ages(&crawl, &now), [3, 3]);
    let arena = vec![entry("BATS", false)];
    crawl.sync(&arena, 4, false);
    assert_eq!(ages(&crawl, &arena), [0]);
}
