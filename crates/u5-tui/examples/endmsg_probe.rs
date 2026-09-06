//! Report `ENDMSG.DAT`'s record *shapes*, to answer "asset or program?".
//!
//! A line that appears on the stock game's screen and not in ours has
//! two possible homes: a record of the shipped file the engine is
//! mis-rendering, or a literal in the original's code that the spec has
//! not published. Those need opposite fixes, and guessing wrong costs a
//! release cycle. This prints every record's length and line structure -
//! never its text, which is copyrighted asset content - so the question
//! can be settled by arithmetic.
//!
//! It settled `cleak/u5-spec#208`: on a `Yes` answer with no sandalwood
//! box the original prints a nine-glyph line before record 10's "pull up
//! a chair" exchange, and the shipped file's eleven records have no room
//! for it - record 10 is four lines, two blank and two of 29 and 27
//! characters, which are exactly the two lines that *do* appear.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: endmsg_probe <PROFILE_DIR>");
    let messages = require_endgame_messages(Path::new(&dir)).expect("ENDMSG.DAT must load");
    println!("{} records", messages.records.len());
    for (index, record) in messages.records.iter().enumerate() {
        let first = record
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("");
        println!(
            "  record {index:>2}: {:>3} chars, {} line(s), first non-blank {:>3} chars, \
             ends with the reply lead-in: {}",
            record.len(),
            record.lines().count(),
            first.len(),
            record.ends_with("You reply: "),
        );
    }
}
