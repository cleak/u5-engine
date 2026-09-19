//! How often do the `conversation.md` §7.3 pause codes appear in the
//! shipped `.TLK` blobs?
//!
//! §7.3 gives `0x83` PAUSE and `0x8F` WAIT-KEY as the pagination
//! mechanism for long responses. This engine ran both as no-ops, so the
//! question is how much text that hides. Reports counts only - no
//! dialogue text, no field contents.
//!
//! Usage: `cargo run -p u5-tui --example tlk_pause_census -- <DIR>`

use std::path::Path;
use u5_runtime::*;

const FILES: [&str; 4] = ["CASTLE.TLK", "TOWNE.TLK", "DWELLING.TLK", "KEEP.TLK"];

fn main() {
    let dir = std::env::args().nth(1).expect("usage: <DIR>");
    let dir = Path::new(&dir);
    let (mut pauses, mut waits, mut fields, mut with_either) = (0usize, 0usize, 0usize, 0usize);
    for file in FILES {
        let Ok(raw) = parse_tlk_raw(&dir.join(file)) else {
            println!("{file}: not present");
            continue;
        };
        let (mut p, mut w, mut f, mut e) = (0usize, 0usize, 0usize, 0usize);
        for blob in raw.values() {
            for field in blob {
                f += 1;
                let fp = field.iter().filter(|b| **b == TLK_CODE_PAUSE).count();
                let fw = field.iter().filter(|b| **b == TLK_CODE_WAIT_KEY).count();
                if fp + fw > 0 {
                    e += 1;
                }
                p += fp;
                w += fw;
            }
        }
        println!("{file:<13} fields {f:>5}  pause {p:>4}  wait-key {w:>4}  fields with either {e:>4}");
        pauses += p;
        waits += w;
        fields += f;
        with_either += e;
    }
    println!("total        fields {fields:>5}  pause {pauses:>4}  wait-key {waits:>4}  fields with either {with_either:>4}");
}
