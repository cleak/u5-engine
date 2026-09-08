//! Check the shipped `.TLK` data against `catalogs/quest-graph.md`'s
//! validation rules.
//!
//! §11 asks for exactly this: "A future QA verifier can execute or statically
//! interpret the shipped `.TLK` records through the public VM contract and
//! compare them against this catalog." This is the static half - it reads the
//! keyword tables, not the responses - and answers the reachability question
//! the catalog's rules 1 through 4 pose:
//!
//! 1. every dungeon word of §4 is present in the rule data and accepted by the
//!    Word-of-Power path;
//! 2. `DAWN` and `IMPERA` are distinct keywords, not one token;
//! 3. the three Shadowlord names and their shard/flame pairings are
//!    discoverable through conversation;
//! 4. the Crown, Sceptre, Amulet, sandalwood box, grapple and moonstone rule
//!    each connect to at least one NPC route.
//!
//! Sanitized by construction: it prints catalog names, scene families and
//! counts. No dialogue text, no keyword strings from the file, and no offsets
//! leave the process - a keyword is reported only as "the catalog's own name
//! matched, N times".
//!
//! Usage: `cargo run -p u5-tui --example quest_graph_verify -- <DIR>`

use std::collections::BTreeMap;
use std::path::Path;
use u5_runtime::*;

/// The four talk files, by the stem their scene family uses.
const TLK_STEMS: [&str; 4] = ["CASTLE", "TOWNE", "DWELLING", "KEEP"];

/// One row of the catalog the verifier checks for.
struct CatalogTerm {
    section: &'static str,
    name: &'static str,
    /// Whether the term is expected to be reachable from conversation at all.
    /// §4 records that Doom's word has no clean TLK clue.
    conversation_expected: bool,
    /// Spellings the catalog's own prose says the clue may take instead of
    /// the plain name. Two shipped cases, both named in the catalog:
    ///
    /// - §4: "Goeth speaks in reversed wording", and Destard's `INOPIA`
    ///   appears only as `AIPONI`;
    /// - §9's moongate-stone rule is spoken as `MOONGATE` and `STONE` rather
    ///   than as one compound word.
    aliases: &'static [&'static str],
}

const TERMS: &[CatalogTerm] = &[
    // §3 password gates
    CatalogTerm {
        section: "§3",
        name: "DAWN",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§3",
        name: "IMPERA",
        conversation_expected: true,
        aliases: &[],
    },
    // §4 dungeon words
    CatalogTerm {
        section: "§4",
        name: "FALLAX",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§4",
        name: "VILIS",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§4",
        name: "INOPIA",
        conversation_expected: true,
        aliases: &["AIPONI"],
    },
    CatalogTerm {
        section: "§4",
        name: "MALUM",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§4",
        name: "AVIDUS",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§4",
        name: "INFAMA",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§4",
        name: "IGNAVUS",
        conversation_expected: true,
        aliases: &[],
    },
    // "no clean TLK clue identified in this pass"
    CatalogTerm {
        section: "§4",
        name: "VERAMOCOR",
        conversation_expected: false,
        aliases: &[],
    },
    // §5 Shadowlord names
    CatalogTerm {
        section: "§5",
        name: "FAULINEI",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§5",
        name: "ASTAROTH",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§5",
        name: "NOSFENTOR",
        conversation_expected: true,
        aliases: &[],
    },
    // §5 shard and flame vocabulary
    CatalogTerm {
        section: "§5",
        name: "SHARD",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§5",
        name: "FLAME",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§5",
        name: "TRUTH",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§5",
        name: "LOVE",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§5",
        name: "COURAGE",
        conversation_expected: true,
        aliases: &[],
    },
    // §6 royal artifacts
    CatalogTerm {
        section: "§6",
        name: "CROWN",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§6",
        name: "SCEPTRE",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§6",
        name: "AMULET",
        conversation_expected: true,
        aliases: &[],
    },
    // §6/§9 mobility
    CatalogTerm {
        section: "§6",
        name: "GRAPPLE",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§9",
        name: "MOONSTONE",
        conversation_expected: true,
        aliases: &["MOONGATE", "STONE"],
    },
    // §7 the box
    CatalogTerm {
        section: "§7",
        name: "SANDALWOOD",
        conversation_expected: true,
        aliases: &[],
    },
    // §8 mantras
    CatalogTerm {
        section: "§8",
        name: "AHM",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "MU",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "RA",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "BEH",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "CAH",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "SUMM",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "OM",
        conversation_expected: true,
        aliases: &[],
    },
    CatalogTerm {
        section: "§8",
        name: "LUM",
        conversation_expected: true,
        aliases: &[],
    },
];

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: quest_graph_verify <DIR>");
    let dir = Path::new(&dir);

    // Rule 1's second half is an engine question, not a data one: every word
    // must be accepted by the Word-of-Power path.
    let mut unrouted = Vec::new();
    for seal in WORD_OF_POWER_SEALS {
        if dungeon_scene_for_word_of_power(seal.word).is_none() {
            unrouted.push(seal.word);
        }
    }
    println!(
        "rule 1, engine half: {} of {} published words route to a dungeon seal{}",
        WORD_OF_POWER_SEALS.len() - unrouted.len(),
        WORD_OF_POWER_SEALS.len(),
        if unrouted.is_empty() {
            String::new()
        } else {
            format!(" (unrouted: {unrouted:?})")
        }
    );

    // Rule 1's data half, plus rules 2 through 4: does each catalog term
    // appear as a conversation keyword anywhere?
    let mut hits: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    // `formats/tlk.md §9`: `0x8E` ALTERNATE-FONT is "used in matched pairs
    // around mantras and Words of Power", so those terms live in **response**
    // streams - an NPC teaches them - rather than in the keyword table the
    // player types into. Rule 3's "discoverable through conversation" is
    // therefore a question about the whole blob, not only its keywords.
    let mut spoken: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut records = 0usize;
    let mut keywords = 0usize;
    for stem in TLK_STEMS {
        let path = dir.join(format!("{stem}.TLK"));
        let Ok(scenes) = parse_tlk(&path) else {
            println!("  {stem}.TLK: absent or unreadable");
            continue;
        };
        println!("  {stem}.TLK: {} record(s)", scenes.len());
        for fields in scenes.values() {
            records += 1;
            let blob: String = fields.join(" ").to_ascii_uppercase();
            for term in TERMS {
                if blob.contains(term.name) || term.aliases.iter().any(|alias| blob.contains(alias))
                {
                    *spoken.entry(term.name).or_default() += 1;
                }
            }
            for pair in fields.get(5..).unwrap_or_default().chunks_exact(2) {
                keywords += 1;
                for term in TERMS {
                    // The same four-character, case-folding compare the Talk
                    // dispatcher uses (`conversation.md §2`).
                    if talk_keyword_matches(&pair[0], term.name)
                        || term
                            .aliases
                            .iter()
                            .any(|alias| talk_keyword_matches(&pair[0], alias))
                    {
                        let entry = hits.entry(term.name).or_default();
                        if !entry.contains(&stem) {
                            entry.push(stem);
                        }
                    }
                }
            }
        }
    }
    println!("scanned {records} NPC record(s), {keywords} keyword slot(s)");

    let mut missing = 0usize;
    let mut unexpected = 0usize;
    for term in TERMS {
        let keyword = hits.get(term.name);
        let said = spoken.get(term.name).copied().unwrap_or(0);
        let reach = match (keyword, said) {
            (Some(stems), 0) => format!("keyword in {}", stems.join(", ")),
            (Some(stems), n) => format!("keyword in {}, spoken by {n} record(s)", stems.join(", ")),
            (None, 0) => String::new(),
            (None, n) => format!("spoken by {n} record(s)"),
        };
        if reach.is_empty() {
            if term.conversation_expected {
                missing += 1;
                println!(
                    "  {} {:<10} NOT REACHABLE from any talk file",
                    term.section, term.name
                );
            } else {
                unexpected += 1;
                println!(
                    "  {} {:<10} absent, as the catalog records",
                    term.section, term.name
                );
            }
        } else {
            println!("  {} {:<10} {reach}", term.section, term.name);
        }
    }
    println!(
        "\n{} catalog term(s) checked, {missing} unreachable, {unexpected} absent by catalog note",
        TERMS.len()
    );
    // A short term is a substring of ordinary words - `RA` and `OM` land
    // inside dozens of them - so a high spoken count is not evidence of a
    // clue. The check this tool makes is reachability, not attribution: a
    // count of zero is the finding.
    println!("note: short mantras match inside ordinary words; only a zero here is a claim");
}
