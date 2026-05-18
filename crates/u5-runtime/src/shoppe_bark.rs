//! SHOPPE.DAT bark loader and placeholder renderer per
//! `formats/shoppe-dat.md` and `systems/shops.md` §4.
//!
//! Each bark record is a NUL-terminated byte stream that mixes
//! printable ASCII, seven single-byte placeholder sigils (gold,
//! quantity, vendor name, item name, place name, shop name,
//! time-of-day), and high-bit phrase tokens that index the shared
//! 128-entry common-word dictionary. This module parses the per-
//! record byte slices out of the on-disk file and renders one against
//! a [`ShoppeBarkContext`].

use std::io;
use std::path::Path;

use crate::shops::{
    shop_placeholder_kind, shoppe_time_of_day_word, ShopPlaceholderKind, SHOPPE_DAT_RECORD_SLOTS,
};
use crate::tlk_control_codes::{shoppe_dictionary_index, COMMON_WORD_DICTIONARY_ENTRIES};

/// Per-record sliced view of a loaded SHOPPE.DAT.
#[derive(Clone, Debug, Default)]
pub struct ShoppeRecords {
    pub records: Vec<Vec<u8>>,
}

impl ShoppeRecords {
    /// Returns the raw bytes for the supplied record id, or `None`
    /// when the id is outside the known record-slot count or the
    /// slot is empty.
    pub fn record(&self, id: usize) -> Option<&[u8]> {
        self.records.get(id).map(|v| v.as_slice())
    }

    /// Total record slot count (always [`SHOPPE_DAT_RECORD_SLOTS`]
    /// when loaded from a well-formed file).
    pub fn slot_count(&self) -> usize {
        self.records.len()
    }
}

/// Per-render substitution context.
#[derive(Clone, Debug, Default)]
pub struct ShoppeBarkContext<'a> {
    pub gold: u16,
    pub quantity: u16,
    pub vendor_name: &'a str,
    pub item_name: &'a str,
    pub place_name: &'a str,
    pub shop_name: &'a str,
    pub hour: u8,
    /// Optional common-word dictionary; when present, high-bit phrase
    /// tokens expand inline. `None` renders them as `[t<n>]`.
    pub dictionary: Option<&'a [&'a str; COMMON_WORD_DICTIONARY_ENTRIES]>,
}

/// Parse a SHOPPE.DAT byte buffer into its 196 NUL-terminated records.
/// Records that exceed the buffer or read as empty produce empty
/// vectors at the right index so callers can look them up by id
/// without bounds-checking.
pub fn parse_shoppe_records(bytes: &[u8]) -> ShoppeRecords {
    let mut records = Vec::with_capacity(SHOPPE_DAT_RECORD_SLOTS);
    let mut pos = 0usize;
    while pos < bytes.len() && records.len() < SHOPPE_DAT_RECORD_SLOTS {
        let start = pos;
        while pos < bytes.len() && bytes[pos] != 0 {
            pos += 1;
        }
        records.push(bytes[start..pos].to_vec());
        if pos < bytes.len() {
            pos += 1; // skip the NUL terminator
        }
    }
    while records.len() < SHOPPE_DAT_RECORD_SLOTS {
        records.push(Vec::new());
    }
    ShoppeRecords { records }
}

/// Read SHOPPE.DAT from disk and parse it.
pub fn load_shoppe_records(path: &Path) -> io::Result<ShoppeRecords> {
    let bytes = std::fs::read(path)?;
    Ok(parse_shoppe_records(&bytes))
}

/// Render one bark record byte slice into a String, substituting the
/// seven placeholder sigils and expanding high-bit phrase-token
/// indices through the optional dictionary.
pub fn render_shoppe_bark(bytes: &[u8], ctx: &ShoppeBarkContext) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if byte == 0 {
            break;
        }
        if let Some(kind) = shop_placeholder_kind(byte) {
            match kind {
                ShopPlaceholderKind::Gold => out.push_str(&ctx.gold.to_string()),
                ShopPlaceholderKind::Quantity => out.push_str(&ctx.quantity.to_string()),
                ShopPlaceholderKind::VendorName => out.push_str(ctx.vendor_name),
                ShopPlaceholderKind::ItemName => out.push_str(ctx.item_name),
                ShopPlaceholderKind::PlaceName => out.push_str(ctx.place_name),
                ShopPlaceholderKind::ShopName => out.push_str(ctx.shop_name),
                ShopPlaceholderKind::TimeOfDay => out.push_str(shoppe_time_of_day_word(ctx.hour)),
            }
            continue;
        }
        if let Some(idx) = shoppe_dictionary_index(byte) {
            if let Some(dict) = ctx.dictionary {
                if let Some(word) = dict.get(idx).filter(|w| !w.is_empty()) {
                    out.push_str(word);
                    continue;
                }
            }
            out.push_str(&format!("[t{idx:02X}]"));
            continue;
        }
        if (0x20..0x7F).contains(&byte) {
            out.push(byte as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_splits_nul_terminated_records() {
        let bytes = b"hi\0bye\0";
        let records = parse_shoppe_records(bytes);
        assert_eq!(records.record(0).unwrap(), b"hi");
        assert_eq!(records.record(1).unwrap(), b"bye");
        // Remaining slots are padded empty so lookups never panic.
        assert!(records.record(SHOPPE_DAT_RECORD_SLOTS - 1).is_some());
    }

    #[test]
    fn render_substitutes_gold_quantity_and_time_of_day() {
        // % gold $ vendor & item @ time-of-day
        let bytes = b"Pay %, %; @ greetings!";
        let ctx = ShoppeBarkContext {
            gold: 25,
            hour: 9,
            ..Default::default()
        };
        let rendered = render_shoppe_bark(bytes, &ctx);
        assert!(rendered.contains("25"));
        assert!(rendered.contains("morning"));
    }

    #[test]
    fn render_substitutes_vendor_item_place_shop_names() {
        let bytes = b"$ at #: & for ^ gold; *";
        let ctx = ShoppeBarkContext {
            vendor_name: "Alric",
            item_name: "Mace",
            place_name: "Yew",
            shop_name: "Armoury",
            quantity: 3,
            ..Default::default()
        };
        let rendered = render_shoppe_bark(bytes, &ctx);
        assert!(rendered.contains("Alric"));
        assert!(rendered.contains("Mace"));
        assert!(rendered.contains("Yew"));
        assert!(rendered.contains("Armoury"));
        assert!(rendered.contains("3"));
    }

    #[test]
    fn dictionary_token_expands_when_dictionary_supplied() {
        let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        dict[0x05] = "swords";
        let bytes = vec![b'b', b'u', b'y', b' ', 0x85u8];
        let ctx = ShoppeBarkContext {
            dictionary: Some(&dict),
            ..Default::default()
        };
        let rendered = render_shoppe_bark(&bytes, &ctx);
        assert!(rendered.contains("buy"));
        assert!(rendered.contains("swords"));
    }

    #[test]
    fn dictionary_token_without_dictionary_uses_placeholder() {
        let bytes = vec![0x82u8];
        let rendered = render_shoppe_bark(&bytes, &ShoppeBarkContext::default());
        assert!(rendered.contains("[t02]"));
    }

    #[test]
    fn null_byte_terminates_render() {
        let bytes = b"first\0second";
        let rendered = render_shoppe_bark(bytes, &ShoppeBarkContext::default());
        assert_eq!(rendered, "first");
    }

    #[test]
    fn non_printable_low_bytes_are_stripped() {
        let bytes = vec![b'a', 0x01, 0x1F, b'b'];
        let rendered = render_shoppe_bark(&bytes, &ShoppeBarkContext::default());
        assert_eq!(rendered, "ab");
    }

    #[test]
    fn time_of_day_word_changes_with_hour() {
        let bytes = b"@";
        let morning = render_shoppe_bark(
            bytes,
            &ShoppeBarkContext {
                hour: 8,
                ..Default::default()
            },
        );
        let afternoon = render_shoppe_bark(
            bytes,
            &ShoppeBarkContext {
                hour: 14,
                ..Default::default()
            },
        );
        let evening = render_shoppe_bark(
            bytes,
            &ShoppeBarkContext {
                hour: 22,
                ..Default::default()
            },
        );
        assert_eq!(morning, "morning");
        assert_eq!(afternoon, "afternoon");
        assert_eq!(evening, "evening");
    }
}
