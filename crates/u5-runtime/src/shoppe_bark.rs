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

use std::path::Path;
use std::{error::Error, fmt, io};

use crate::shops::{
    SHOPPE_DAT_LEN, SHOPPE_DAT_NONEMPTY_RECORDS, SHOPPE_DAT_RECORD_SLOTS, ShopPlaceholderKind,
    shop_placeholder_kind, shoppe_time_of_day_word,
};
use crate::tlk_control_codes::{COMMON_WORD_DICTIONARY_ENTRIES, shoppe_dictionary_index};

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

    /// Returns a non-empty record or a precise asset error. Shop
    /// overlays use this when selecting records by hardcoded id: per
    /// `formats/shoppe-dat.md §8`, a missing shop text record should
    /// surface as an asset error rather than a partial menu.
    pub fn required_record(&self, id: usize) -> Result<&[u8], ShoppeDatError> {
        let Some(record) = self.record(id) else {
            return Err(ShoppeDatError::MissingRecord {
                id,
                slots: self.records.len(),
            });
        };
        if record.is_empty() {
            return Err(ShoppeDatError::EmptyRecord { id });
        }
        Ok(record)
    }

    /// Total record slot count (always [`SHOPPE_DAT_RECORD_SLOTS`]
    /// when loaded from a well-formed file).
    pub fn slot_count(&self) -> usize {
        self.records.len()
    }
}

/// Validation errors for the shipped SHOPPE.DAT container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShoppeDatError {
    InvalidLength { actual: usize, expected: usize },
    UnterminatedRecord { record_id: usize, offset: usize },
    WrongRecordCount { actual: usize, expected: usize },
    WrongNonEmptyRecordCount { actual: usize, expected: usize },
    TrailingBytes { offset: usize, len: usize },
    MissingRecord { id: usize, slots: usize },
    EmptyRecord { id: usize },
}

impl fmt::Display for ShoppeDatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { actual, expected } => {
                write!(f, "SHOPPE.DAT length {actual} != expected {expected}")
            }
            Self::UnterminatedRecord { record_id, offset } => {
                write!(
                    f,
                    "SHOPPE.DAT record {record_id} starting at byte {offset} is not NUL-terminated"
                )
            }
            Self::WrongRecordCount { actual, expected } => {
                write!(f, "SHOPPE.DAT record count {actual} != expected {expected}")
            }
            Self::WrongNonEmptyRecordCount { actual, expected } => write!(
                f,
                "SHOPPE.DAT non-empty record count {actual} != expected {expected}"
            ),
            Self::TrailingBytes { offset, len } => {
                write!(
                    f,
                    "SHOPPE.DAT has trailing bytes after record slots at {offset}/{len}"
                )
            }
            Self::MissingRecord { id, slots } => {
                write!(
                    f,
                    "SHOPPE.DAT record {id} is outside loaded slot count {slots}"
                )
            }
            Self::EmptyRecord { id } => write!(f, "SHOPPE.DAT record {id} is empty"),
        }
    }
}

impl Error for ShoppeDatError {}

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

/// Parse and validate a shipped SHOPPE.DAT buffer.
pub fn parse_shoppe_records_checked(bytes: &[u8]) -> Result<ShoppeRecords, ShoppeDatError> {
    if bytes.len() != SHOPPE_DAT_LEN {
        return Err(ShoppeDatError::InvalidLength {
            actual: bytes.len(),
            expected: SHOPPE_DAT_LEN,
        });
    }

    let mut records = Vec::with_capacity(SHOPPE_DAT_RECORD_SLOTS);
    let mut pos = 0usize;
    while records.len() < SHOPPE_DAT_RECORD_SLOTS && pos < bytes.len() {
        let start = pos;
        while pos < bytes.len() && bytes[pos] != 0 {
            pos += 1;
        }
        if pos >= bytes.len() {
            return Err(ShoppeDatError::UnterminatedRecord {
                record_id: records.len(),
                offset: start,
            });
        }
        records.push(bytes[start..pos].to_vec());
        pos += 1;
    }

    if records.len() < SHOPPE_DAT_RECORD_SLOTS {
        if records.len() < SHOPPE_DAT_NONEMPTY_RECORDS {
            return Err(ShoppeDatError::WrongRecordCount {
                actual: records.len(),
                expected: SHOPPE_DAT_RECORD_SLOTS,
            });
        }
        while records.len() < SHOPPE_DAT_RECORD_SLOTS {
            records.push(Vec::new());
        }
    } else if pos != bytes.len() {
        return Err(ShoppeDatError::TrailingBytes {
            offset: pos,
            len: bytes.len(),
        });
    }

    let non_empty = records.iter().filter(|record| !record.is_empty()).count();
    if non_empty != SHOPPE_DAT_NONEMPTY_RECORDS {
        return Err(ShoppeDatError::WrongNonEmptyRecordCount {
            actual: non_empty,
            expected: SHOPPE_DAT_NONEMPTY_RECORDS,
        });
    }

    Ok(ShoppeRecords { records })
}

/// Read SHOPPE.DAT from disk and validate it before exposing records.
pub fn load_shoppe_records(path: &Path) -> io::Result<ShoppeRecords> {
    let bytes = std::fs::read(path)?;
    parse_shoppe_records_checked(&bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
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

/// Look up and render a non-empty SHOPPE.DAT record.
pub fn render_shoppe_record(
    records: &ShoppeRecords,
    id: usize,
    ctx: &ShoppeBarkContext,
) -> Result<String, ShoppeDatError> {
    records
        .required_record(id)
        .map(|record| render_shoppe_bark(record, ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_shoppe_dat_bytes() -> Vec<u8> {
        let mut bytes = Vec::with_capacity(SHOPPE_DAT_LEN);
        bytes.extend(std::iter::repeat_n(b'a', 9746));
        bytes.push(0);
        for _ in 1..SHOPPE_DAT_NONEMPTY_RECORDS {
            bytes.push(b'x');
            bytes.push(0);
        }
        bytes.push(0);
        bytes.push(0);
        assert_eq!(bytes.len(), SHOPPE_DAT_LEN);
        bytes
    }

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
    fn checked_parse_accepts_shipped_shape() {
        let bytes = valid_shoppe_dat_bytes();
        let records = parse_shoppe_records_checked(&bytes).unwrap();
        assert_eq!(records.slot_count(), SHOPPE_DAT_RECORD_SLOTS);
        assert_eq!(
            records
                .records
                .iter()
                .filter(|record| !record.is_empty())
                .count(),
            SHOPPE_DAT_NONEMPTY_RECORDS
        );
    }

    #[test]
    fn checked_parse_accepts_single_empty_trailer_with_padded_final_slot() {
        let mut bytes = Vec::with_capacity(SHOPPE_DAT_LEN);
        bytes.extend(std::iter::repeat_n(b'a', 9747));
        bytes.push(0);
        for _ in 1..SHOPPE_DAT_NONEMPTY_RECORDS {
            bytes.push(b'x');
            bytes.push(0);
        }
        bytes.push(0);
        assert_eq!(bytes.len(), SHOPPE_DAT_LEN);

        let records = parse_shoppe_records_checked(&bytes).unwrap();
        assert_eq!(records.slot_count(), SHOPPE_DAT_RECORD_SLOTS);
        assert_eq!(
            records
                .records
                .iter()
                .filter(|record| !record.is_empty())
                .count(),
            SHOPPE_DAT_NONEMPTY_RECORDS
        );
        assert_eq!(records.record(SHOPPE_DAT_RECORD_SLOTS - 1).unwrap(), b"");
    }

    #[test]
    fn checked_parse_rejects_wrong_length() {
        let bytes = vec![0; SHOPPE_DAT_LEN - 1];
        assert!(matches!(
            parse_shoppe_records_checked(&bytes),
            Err(ShoppeDatError::InvalidLength { .. })
        ));
    }

    #[test]
    fn checked_parse_rejects_unterminated_record() {
        let mut bytes = valid_shoppe_dat_bytes();
        *bytes.last_mut().unwrap() = b'x';
        assert!(matches!(
            parse_shoppe_records_checked(&bytes),
            Err(ShoppeDatError::UnterminatedRecord { record_id: 195, .. })
        ));
    }

    #[test]
    fn checked_parse_rejects_trailing_bytes_after_record_slots() {
        let mut bytes = Vec::with_capacity(SHOPPE_DAT_LEN);
        for _ in 0..SHOPPE_DAT_RECORD_SLOTS {
            bytes.push(0);
        }
        bytes.resize(SHOPPE_DAT_LEN, 0);
        assert!(matches!(
            parse_shoppe_records_checked(&bytes),
            Err(ShoppeDatError::TrailingBytes { .. })
        ));
    }

    #[test]
    fn required_record_errors_on_out_of_range_and_empty_slots() {
        let records = parse_shoppe_records_checked(&valid_shoppe_dat_bytes()).unwrap();
        assert!(records.required_record(0).is_ok());
        assert!(matches!(
            records.required_record(SHOPPE_DAT_NONEMPTY_RECORDS),
            Err(ShoppeDatError::EmptyRecord { .. })
        ));
        assert!(matches!(
            records.required_record(SHOPPE_DAT_RECORD_SLOTS),
            Err(ShoppeDatError::MissingRecord { .. })
        ));
    }

    #[test]
    fn render_shoppe_record_uses_required_lookup() {
        let bytes = b"hello\0";
        let records = parse_shoppe_records(bytes);
        assert_eq!(
            render_shoppe_record(&records, 0, &ShoppeBarkContext::default()).unwrap(),
            "hello"
        );
        assert!(matches!(
            render_shoppe_record(&records, 1, &ShoppeBarkContext::default()),
            Err(ShoppeDatError::EmptyRecord { id: 1 })
        ));
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
