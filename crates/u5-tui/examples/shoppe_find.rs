//! Report which `SHOPPE.DAT` record ids render text containing a phrase.
//!
//! The paired captures show the original answering some shop outcomes with a
//! rendered record rather than a resident literal. Wiring the engine to the
//! right record needs the record *id*, and the id is all this prints - never
//! the record text, which is copyrighted asset content.

use std::path::Path;
use u5_runtime::shoppe_bark::{ShoppeBarkContext, ShoppeTextRenderer};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: shoppe_find <GAME_DIR> <PHRASE>");
    let phrase = args
        .next()
        .expect("usage: shoppe_find <GAME_DIR> <PHRASE>")
        .to_ascii_lowercase();
    let renderer =
        ShoppeTextRenderer::load_from_game_dir(Path::new(&dir)).expect("SHOPPE.DAT must load");
    let ctx = ShoppeBarkContext {
        gold: 0,
        quantity: 0,
        vendor_name: "",
        item_name: "",
        place_name: "",
        shop_name: "",
        hour: 10,
        dictionary: None,
    };
    let mut hits = 0;
    for id in 0..renderer.records().records.len() {
        let Ok(text) = renderer.render_record(id, &ctx) else {
            continue;
        };
        if text.to_ascii_lowercase().contains(&phrase) {
            println!("record {id}");
            hits += 1;
        }
    }
    println!("{hits} record(s) contain the phrase");
}
