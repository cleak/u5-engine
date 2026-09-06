//! Where does `0x88` ASK-WHO appear in a shipped `.TLK` blob?
//!
//! `cleak/u5-spec#198`: the stock game's opening for a `DWELLING.TLK`
//! NPC stops to ask the party's name and this engine does not. Reports
//! field indices and byte offsets only - no dialogue text.
//!
//! Usage: `cargo run -p u5-tui --example tlk_ask_who_census -- <DIR> <FILE> <ID>`

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: <DIR> <FILE> <ID>");
    let file = args.next().expect("usage: <DIR> <FILE> <ID>");
    let id: u16 = args
        .next()
        .expect("usage: <DIR> <FILE> <ID>")
        .parse()
        .unwrap();
    let raw = parse_tlk_raw(&Path::new(&dir).join(&file)).expect("TLK must parse");
    let Some(fields) = raw.get(&id) else {
        println!("no blob for id {id}");
        return;
    };
    println!("{file} id {id}: {} field(s)", fields.len());
    for (index, field) in fields.iter().enumerate() {
        let codes: Vec<String> = field
            .iter()
            .enumerate()
            .filter(|(_, byte)| (0x80..=0x8F).contains(*byte) || **byte == 0xA2)
            .map(|(at, byte)| format!("{at}:0x{byte:02X}"))
            .collect();
        println!(
            "  field {index:>2}: {:>4} bytes, control/quote bytes [{}]",
            field.len(),
            codes.join(" ")
        );
    }
}
