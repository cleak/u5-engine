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

use crate::shoppe_records::ShoppeBand;
use crate::shops::{
    SAGE_RUMOUR_FEE_QUOTE_RECORD, SAGE_RUMOUR_SHORT_FUNDS_RECORD, SHOPPE_DAT_LEN,
    SHOPPE_DAT_NONEMPTY_RECORDS, SHOPPE_DAT_RECORD_SLOTS, ShopPlaceholderKind,
    sage_rumour_success_record_id_accepted, shop_placeholder_kind, shoppe_time_of_day_word,
};
use crate::tlk_control_codes::{COMMON_WORD_DICTIONARY_ENTRIES, shoppe_dictionary_index};
use crate::{PUBLISHED_COMMON_WORD_DICTIONARY, TextWindowSystem, read_disk_file};

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
    MissingCommonWordDictionary { id: usize },
    EmptyCommonWordDictionaryEntry { token: u8, index: usize },
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
            Self::MissingCommonWordDictionary { id } => write!(
                f,
                "SHOPPE.DAT record {id} has no common-word dictionary for phrase-token expansion"
            ),
            Self::EmptyCommonWordDictionaryEntry { token, index } => write!(
                f,
                "SHOPPE.DAT phrase token {token:#04X} resolves to empty dictionary index {}",
                index + 1
            ),
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
    /// Optional common-word dictionary override; `None` uses the published
    /// common-word table shared with TLK text.
    pub dictionary: Option<&'a [&'a str; COMMON_WORD_DICTIONARY_ENTRIES]>,
}

/// Sanitized render-audit summary for one SHOPPE.DAT band.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShoppeBandAudit {
    pub band: ShoppeBand,
    pub first_record: usize,
    pub last_record: usize,
    pub non_empty_records: usize,
    pub tokenized_records: usize,
    pub placeholder_records: usize,
    pub max_rendered_len: usize,
}

/// Sanitized render-audit summary for SHOPPE.DAT.
///
/// This intentionally records only counts and lengths. It proves the
/// public renderer can visit every shipped non-empty record without
/// committing or printing shop prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShoppeTextAudit {
    pub slot_count: usize,
    pub non_empty_records: usize,
    pub rendered_records: usize,
    pub tokenized_records: usize,
    pub placeholder_records: usize,
    pub max_rendered_len: usize,
    pub bands: Vec<ShoppeBandAudit>,
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
    let bytes = read_disk_file(path)?;
    parse_shoppe_records_checked(&bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

/// Shared SHOPPE.DAT text renderer for shop overlays.
///
/// The renderer owns the validated record table and provides both string
/// expansion and text-window emission. Shop state machines still own the
/// gameplay side effects and exact record selection.
#[derive(Clone, Debug)]
pub struct ShoppeTextRenderer {
    records: ShoppeRecords,
}

impl ShoppeTextRenderer {
    pub fn new(records: ShoppeRecords) -> Self {
        Self { records }
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        Ok(Self::new(load_shoppe_records(path)?))
    }

    pub fn load_from_game_dir(game_dir: &Path) -> io::Result<Self> {
        Self::load(&game_dir.join("SHOPPE.DAT"))
    }

    pub fn records(&self) -> &ShoppeRecords {
        &self.records
    }

    pub fn render_record(
        &self,
        id: usize,
        ctx: &ShoppeBarkContext,
    ) -> Result<String, ShoppeDatError> {
        render_shoppe_record(&self.records, id, ctx)
    }

    pub fn print_record(
        &self,
        system: &mut TextWindowSystem,
        id: usize,
        ctx: &ShoppeBarkContext,
    ) -> Result<String, ShoppeDatError> {
        let rendered = self.render_record(id, ctx)?;
        system.print_wrapped_string(&rendered);
        Ok(rendered)
    }

    pub fn render_sage_rumour_record(
        &self,
        record_id: usize,
        matched_name: &str,
        location: &str,
        dictionary: Option<&[&str; COMMON_WORD_DICTIONARY_ENTRIES]>,
    ) -> Result<String, ShoppeDatError> {
        render_sage_rumour_shoppe_record(
            &self.records,
            record_id,
            matched_name,
            location,
            dictionary,
        )
    }

    pub fn render_sage_fee_quote_record(
        &self,
        fee: u16,
        dictionary: Option<&[&str; COMMON_WORD_DICTIONARY_ENTRIES]>,
    ) -> Result<String, ShoppeDatError> {
        self.render_record(
            SAGE_RUMOUR_FEE_QUOTE_RECORD,
            &ShoppeBarkContext {
                gold: fee,
                dictionary,
                ..Default::default()
            },
        )
    }

    pub fn render_sage_short_funds_record(
        &self,
        dictionary: Option<&[&str; COMMON_WORD_DICTIONARY_ENTRIES]>,
    ) -> Result<String, ShoppeDatError> {
        self.render_record(
            SAGE_RUMOUR_SHORT_FUNDS_RECORD,
            &ShoppeBarkContext {
                dictionary,
                ..Default::default()
            },
        )
    }

    pub fn audit_records(&self, ctx: &ShoppeBarkContext) -> ShoppeTextAudit {
        audit_shoppe_records(&self.records, ctx)
    }
}

impl From<ShoppeRecords> for ShoppeTextRenderer {
    fn from(records: ShoppeRecords) -> Self {
        Self::new(records)
    }
}

/// Render one bark record byte slice into a String, substituting the
/// seven placeholder sigils and expanding high-bit phrase-token
/// indices through the optional dictionary.
pub fn render_shoppe_bark(bytes: &[u8], ctx: &ShoppeBarkContext) -> Result<String, ShoppeDatError> {
    let mut out = String::with_capacity(bytes.len());
    let dictionary = ctx.dictionary.unwrap_or(&PUBLISHED_COMMON_WORD_DICTIONARY);
    for (position, &byte) in bytes.iter().enumerate() {
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
            let word = dictionary.get(idx).copied().unwrap_or("");
            if word.is_empty() {
                return Err(ShoppeDatError::EmptyCommonWordDictionaryEntry {
                    token: byte,
                    index: idx,
                });
            }
            // `shops.md §4.2`: every token has one leading space. A
            // trailing space is emitted only when the following record byte
            // is ordinary text rather than another token or NUL.
            out.push(' ');
            out.push_str(word);
            if bytes
                .get(position + 1)
                .is_some_and(|next| *next != 0 && shoppe_dictionary_index(*next).is_none())
            {
                out.push(' ');
            }
            continue;
        }
        if (0x20..0x7F).contains(&byte) {
            out.push(byte as char);
        }
    }
    Ok(out)
}

/// Look up and render a non-empty SHOPPE.DAT record.
pub fn render_shoppe_record(
    records: &ShoppeRecords,
    id: usize,
    ctx: &ShoppeBarkContext,
) -> Result<String, ShoppeDatError> {
    let record = records.required_record(id)?;
    render_shoppe_bark(record, ctx)
}

/// Render every non-empty record and return sanitized aggregate coverage.
pub fn audit_shoppe_records(records: &ShoppeRecords, ctx: &ShoppeBarkContext) -> ShoppeTextAudit {
    const BANDS: [ShoppeBand; 11] = [
        ShoppeBand::SharedBark,
        ShoppeBand::ArmsDescription,
        ShoppeBand::ArmsSell,
        ShoppeBand::Tavern,
        ShoppeBand::Sage,
        ShoppeBand::HorseTrader,
        ShoppeBand::ShipBroker,
        ShoppeBand::Reagent,
        ShoppeBand::Guild,
        ShoppeBand::Healer,
        ShoppeBand::Innkeeper,
    ];

    let mut audit = ShoppeTextAudit {
        slot_count: records.slot_count(),
        non_empty_records: 0,
        rendered_records: 0,
        tokenized_records: 0,
        placeholder_records: 0,
        max_rendered_len: 0,
        bands: BANDS
            .into_iter()
            .map(|band| {
                let (first, last) = (band.first(), band.last());
                ShoppeBandAudit {
                    band,
                    first_record: first,
                    last_record: last,
                    non_empty_records: 0,
                    tokenized_records: 0,
                    placeholder_records: 0,
                    max_rendered_len: 0,
                }
            })
            .collect(),
    };

    for id in 0..records.slot_count() {
        let Some(record) = records.record(id) else {
            continue;
        };
        if record.is_empty() {
            continue;
        }
        let tokenized = record
            .iter()
            .any(|byte| shoppe_dictionary_index(*byte).is_some());
        let placeholders = record
            .iter()
            .any(|byte| shop_placeholder_kind(*byte).is_some());
        let rendered_len = render_shoppe_bark(record, ctx)
            .expect("validated SHOPPE.DAT record must not reference an empty dictionary entry")
            .len();

        audit.non_empty_records += 1;
        audit.rendered_records += 1;
        audit.max_rendered_len = audit.max_rendered_len.max(rendered_len);
        if tokenized {
            audit.tokenized_records += 1;
        }
        if placeholders {
            audit.placeholder_records += 1;
        }
        if let Some(band) = ShoppeBand::classify(id)
            && let Some(band_audit) = audit.bands.iter_mut().find(|entry| entry.band == band)
        {
            band_audit.non_empty_records += 1;
            band_audit.max_rendered_len = band_audit.max_rendered_len.max(rendered_len);
            if tokenized {
                band_audit.tokenized_records += 1;
            }
            if placeholders {
                band_audit.placeholder_records += 1;
            }
        }
    }

    audit
}

/// Render a public issue #13 sage rumour success record. Paid sage
/// success templates live in SHOPPE.DAT record ids 85-88 and use the ordinary `&` item-name and
/// `*` place-name placeholders for matched-name and location text.
pub fn render_sage_rumour_shoppe_record(
    records: &ShoppeRecords,
    record_id: usize,
    matched_name: &str,
    location: &str,
    dictionary: Option<&[&str; COMMON_WORD_DICTIONARY_ENTRIES]>,
) -> Result<String, ShoppeDatError> {
    if !sage_rumour_success_record_id_accepted(record_id) {
        return Err(ShoppeDatError::MissingRecord {
            id: record_id,
            slots: records.slot_count(),
        });
    }
    render_shoppe_record(
        records,
        record_id,
        &ShoppeBarkContext {
            item_name: matched_name,
            place_name: location,
            dictionary,
            ..Default::default()
        },
    )
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
    fn shoppe_text_renderer_renders_and_prints_to_text_window() {
        let records = ShoppeRecords {
            records: vec![b"$ sells & for % gold in @".to_vec()],
        };
        let renderer = ShoppeTextRenderer::new(records);
        let ctx = ShoppeBarkContext {
            gold: 27,
            vendor_name: "Julia",
            item_name: "keys",
            hour: 16,
            ..Default::default()
        };
        let mut system = TextWindowSystem::new();
        system.set_window_rect(0, 0, 0, 39, 5);
        system.set_active_cursor(0, 0);

        let rendered = renderer.print_record(&mut system, 0, &ctx).unwrap();

        assert_eq!(rendered, "Julia sells keys for 27 gold in afternoon");
        let rows = system.region_rows(0, 0, 39, 5, b' ');
        let compact = rows.join("").replace(' ', "");
        assert!(compact.contains("Juliasellskeysfor27goldinafternoon"));
    }

    #[test]
    fn shoppe_text_renderer_errors_without_partial_window_paint() {
        let records = ShoppeRecords {
            records: vec![Vec::new()],
        };
        let renderer = ShoppeTextRenderer::new(records);
        let mut system = TextWindowSystem::new();
        system.set_window_rect(0, 0, 0, 9, 1);

        let before = system.region_rows(0, 0, 9, 1, b'.');

        assert_eq!(
            renderer.print_record(&mut system, 0, &ShoppeBarkContext::default()),
            Err(ShoppeDatError::EmptyRecord { id: 0 })
        );
        assert_eq!(system.region_rows(0, 0, 9, 1, b'.'), before);
    }

    #[test]
    fn shoppe_text_audit_summarizes_records_without_exposing_text() {
        let mut records = vec![Vec::new(); SHOPPE_DAT_RECORD_SLOTS];
        records[0] = vec![0x80, b' ', b'$', b' ', b'&'];
        records[crate::SHOPPE_RECORDS_REAGENT_FIRST] = vec![b'%', b' ', b'^'];
        records[crate::SAGE_RUMOUR_FEE_QUOTE_RECORD] = vec![b'&', b' ', b'*'];
        let renderer = ShoppeTextRenderer::new(ShoppeRecords { records });
        let audit = renderer.audit_records(&ShoppeBarkContext {
            gold: 27,
            quantity: 3,
            vendor_name: "Julia",
            item_name: "keys",
            place_name: "Moonglow",
            ..Default::default()
        });

        assert_eq!(audit.slot_count, SHOPPE_DAT_RECORD_SLOTS);
        assert_eq!(audit.non_empty_records, 3);
        assert_eq!(audit.rendered_records, 3);
        assert_eq!(audit.tokenized_records, 1);
        assert_eq!(audit.placeholder_records, 3);
        assert!(audit.max_rendered_len > 0);
        let sage = audit
            .bands
            .iter()
            .find(|entry| entry.band == ShoppeBand::Sage)
            .unwrap();
        assert_eq!(sage.non_empty_records, 1);
        assert_eq!(sage.placeholder_records, 1);
    }

    #[test]
    fn local_shoppe_dat_asset_renders_sanitized_audit_when_present() {
        let Some(game_dir) = crate::test_fixtures::configured_original_asset_dir() else {
            return;
        };
        let game_dir = game_dir.as_path();
        let path = game_dir.join("SHOPPE.DAT");
        if !path.exists() {
            return;
        }

        let renderer = ShoppeTextRenderer::load(&path).unwrap();
        let audit = renderer.audit_records(&ShoppeBarkContext {
            gold: 999,
            quantity: 9,
            vendor_name: "Vendor",
            item_name: "Item",
            place_name: "Place",
            shop_name: "Shop",
            hour: 13,
            ..Default::default()
        });

        assert_eq!(audit.slot_count, SHOPPE_DAT_RECORD_SLOTS);
        assert_eq!(audit.non_empty_records, SHOPPE_DAT_NONEMPTY_RECORDS);
        assert_eq!(audit.rendered_records, SHOPPE_DAT_NONEMPTY_RECORDS);
        assert!(audit.tokenized_records > 0);
        assert!(audit.placeholder_records > 0);
        assert!(audit.max_rendered_len > 0);
    }

    #[test]
    fn render_sage_rumour_shoppe_record_accepts_only_success_band() {
        let mut records = ShoppeRecords {
            records: vec![Vec::new(); 93],
        };
        records.records[83] = b"wrong & *".to_vec();
        records.records[85] = b"Ask & in *".to_vec();

        assert_eq!(
            render_sage_rumour_shoppe_record(&records, 85, "Greyson", "Cotham", None).unwrap(),
            "Ask Greyson in Cotham"
        );
        assert_eq!(
            render_sage_rumour_shoppe_record(&records, 83, "Greyson", "Cotham", None),
            Err(ShoppeDatError::MissingRecord { id: 83, slots: 93 })
        );
        assert_eq!(
            render_sage_rumour_shoppe_record(&records, 84, "Greyson", "Cotham", None),
            Err(ShoppeDatError::MissingRecord { id: 84, slots: 93 })
        );
    }

    #[test]
    fn shoppe_text_renderer_renders_sage_quote_and_short_funds_records() {
        let mut records = ShoppeRecords {
            records: vec![Vec::new(); 93],
        };
        records.records[SAGE_RUMOUR_FEE_QUOTE_RECORD] = b"Pay % gold?".to_vec();
        records.records[SAGE_RUMOUR_SHORT_FUNDS_RECORD] = b"No credit.".to_vec();
        let renderer = ShoppeTextRenderer::new(records);

        assert_eq!(
            renderer.render_sage_fee_quote_record(50, None).unwrap(),
            "Pay 50 gold?"
        );
        assert_eq!(
            renderer.render_sage_short_funds_record(None).unwrap(),
            "No credit."
        );
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
        let rendered = render_shoppe_bark(bytes, &ctx).unwrap();
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
        let rendered = render_shoppe_bark(bytes, &ctx).unwrap();
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
        let rendered = render_shoppe_bark(&bytes, &ctx).unwrap();
        assert!(rendered.contains("buy"));
        assert!(rendered.contains("swords"));
    }

    #[test]
    fn render_shoppe_record_uses_published_dictionary_for_tokenized_record() {
        let records = ShoppeRecords {
            records: vec![vec![b'b', b'u', b'y', 0x80]],
        };

        assert_eq!(
            render_shoppe_record(&records, 0, &ShoppeBarkContext::default()).unwrap(),
            "buy the"
        );

        let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        dict[0] = "the";
        let rendered = render_shoppe_record(
            &records,
            0,
            &ShoppeBarkContext {
                dictionary: Some(&dict),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, "buy the");
    }

    #[test]
    fn dictionary_token_without_override_uses_published_table() {
        let bytes = vec![0x82u8];
        let rendered = render_shoppe_bark(&bytes, &ShoppeBarkContext::default()).unwrap();
        assert_eq!(rendered, " of");
    }

    #[test]
    fn dictionary_spacing_distinguishes_token_token_and_token_text() {
        let bytes = vec![0x80, 0x81, b'!'];
        let rendered = render_shoppe_bark(&bytes, &ShoppeBarkContext::default()).unwrap();
        assert_eq!(rendered, " the thou !");
    }

    #[test]
    fn published_empty_dictionary_slot_is_malformed_shop_content() {
        let bytes = vec![b'a', 0x87u8, b'b'];
        assert_eq!(
            render_shoppe_bark(&bytes, &ShoppeBarkContext::default()),
            Err(ShoppeDatError::EmptyCommonWordDictionaryEntry {
                token: 0x87,
                index: 7,
            })
        );
    }

    #[test]
    fn null_byte_terminates_render() {
        let bytes = b"first\0second";
        let rendered = render_shoppe_bark(bytes, &ShoppeBarkContext::default()).unwrap();
        assert_eq!(rendered, "first");
    }

    #[test]
    fn non_printable_low_bytes_are_stripped() {
        let bytes = vec![b'a', 0x01, 0x1F, b'b'];
        let rendered = render_shoppe_bark(&bytes, &ShoppeBarkContext::default()).unwrap();
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
        )
        .unwrap();
        let afternoon = render_shoppe_bark(
            bytes,
            &ShoppeBarkContext {
                hour: 14,
                ..Default::default()
            },
        )
        .unwrap();
        let evening = render_shoppe_bark(
            bytes,
            &ShoppeBarkContext {
                hour: 22,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(morning, "morning");
        assert_eq!(afternoon, "afternoon");
        assert_eq!(evening, "evening");
    }
}
