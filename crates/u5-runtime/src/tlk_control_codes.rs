//! Named constants for the `.TLK` byte-runner control bytes per
//! `systems/conversation.md` §7.2-§7.7. The runner classifies each byte by
//! value range and either emits printable output, runs a control action, or
//! stops the current stream.
//!
//! The full byte runner is not yet wired into the engine; these constants
//! exist so that helpers and tests can refer to spec-named codes without
//! magic numbers.

/// `conversation.md §8`: shared common-word dictionary has 128
/// entries. The dialogue runner and the shop renderer apply different
/// byte-range biases when reaching this same logical table.
pub const COMMON_WORD_DICTIONARY_ENTRIES: usize = 128;

/// `conversation.md §8`: TLK dialogue dictionary tokens are nonzero
/// high-bit-clear bytes (`0x01..=0x7F`); the byte runner's range maps
/// directly to the 128-entry index `0..=127` (less the NUL slot).
pub const fn tlk_dictionary_index(token: u8) -> Option<usize> {
    if token == 0 || token & 0x80 != 0 {
        None
    } else {
        Some(token as usize)
    }
}

/// `conversation.md §8` / `shoppe-dat.md §5`: the shop renderer bias
/// strips the high bit of a phrase token to produce the same logical
/// dictionary index. Token `0x80` resolves to entry zero.
pub const fn shoppe_dictionary_index(token: u8) -> Option<usize> {
    if token & 0x80 == 0 {
        None
    } else {
        Some((token & 0x7F) as usize)
    }
}

/// `formats/tlk.md §5`: each header entry is exactly four bytes
/// (`(blob_offset, npc_id)`).
pub const TLK_HEADER_ENTRY_LEN: usize = 4;
/// `formats/tlk.md §6`: NPC-id `0x0001` is the universal sentinel slot
/// at the head of every `.TLK` file; no live NPC carries this id.
pub const TLK_SENTINEL_NPC_ID: u16 = 0x0001;
/// `formats/tlk.md §4`: the engine performs a single fixed-size header
/// read of 512 bytes (covers any class — TOWNE's 48-NPC header is only
/// 192 bytes).
pub const TLK_HEADER_FIXED_READ: usize = 512;
/// `formats/tlk.md §4`: blob payload window the engine reads at the
/// matched `blob_offset`. Shorter blobs may include bytes from
/// following entries; longer nominal spans are truncated to this
/// window.
pub const TLK_BLOB_FIXED_WINDOW: usize = 1024;
/// `formats/tlk.md §8`: high-bit XOR mask the obfuscated text bytes
/// carry on disk. Stripping it recovers low-ASCII.
pub const TLK_TEXT_XOR_MASK: u8 = 0x80;

// §7.2 player-name and stream-control codes
pub const TLK_CODE_PRINT_AVATAR_NAME: u8 = 0x81;
pub const TLK_CODE_END_STREAM: u8 = 0x82;

// §7.3 pause and key-wait codes
pub const TLK_CODE_PAUSE: u8 = 0x83;
pub const TLK_CODE_WAIT_KEY: u8 = 0x8F;

// §7.4 newline and panel-flush codes
pub const TLK_CODE_PANEL_NEWLINE: u8 = 0x8A;
pub const TLK_CODE_LITERAL_NEWLINE: u8 = 0x8D;

// §7.5 print-mask and curse codes
pub const TLK_CODE_CURSE_CHECK: u8 = 0x8B;
pub const TLK_CODE_PROTECT_RUN: u8 = 0x8E;

// §7.6 branching, recruitment, and transactional codes
pub const TLK_CODE_ASK_PARTY_NAME: u8 = 0x84;
pub const TLK_CODE_GOLD_PAYMENT: u8 = 0x85;
pub const TLK_CODE_ACTION_DISPATCH: u8 = 0x86;
pub const TLK_CODE_SET_FLAG: u8 = 0x87;
pub const TLK_CODE_ASK_WHO: u8 = 0x88;
pub const TLK_CODE_IF_ELSE: u8 = 0x8C;
pub const TLK_CODE_IF_ELSE_ALT: u8 = 0xFE;

// §7.7 labels, GOTO, and scoped prompts (and §7 dispatcher boundaries)
pub const TLK_LABEL_FIRST: u8 = 0x91;
pub const TLK_LABEL_LAST: u8 = 0x9F;
pub const TLK_CODE_GOTO_LABEL_FIRST: u8 = 0x9E;
pub const TLK_CODE_GOTO_LABEL_LAST: u8 = 0x9F;
pub const TLK_CODE_END_OF_RESPONSE: u8 = 0xFF;

/// `conversation.md` §7.6: argument-byte width for each multi-byte
/// introducer code. Returns `None` for codes that take no follow-up bytes.
pub const fn tlk_introducer_argument_count(code: u8) -> Option<u8> {
    match code {
        TLK_CODE_GOLD_PAYMENT => Some(3),
        TLK_CODE_ACTION_DISPATCH | TLK_CODE_IF_ELSE => Some(1),
        TLK_CODE_IF_ELSE_ALT => Some(2),
        _ => None,
    }
}

/// `conversation.md` §7.7: label byte range `0x91..=0x9F`.
pub const fn is_tlk_label_byte(byte: u8) -> bool {
    byte >= TLK_LABEL_FIRST && byte <= TLK_LABEL_LAST
}

/// Result of classifying a single byte through the `.TLK` byte runner per
/// `conversation.md §7`'s dispatcher table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkByteKind {
    /// Byte 0 (NUL): implicit blob-end marker; no action is enumerated by
    /// the spec dispatcher table itself.
    Nul,
    /// Nonzero high-bit-clear `0x01..=0x7F`: common-word dictionary token.
    DictionaryToken,
    /// `0x80..=0x9F` excluding the GOTO range: engine control byte
    /// (Sections 7.2-7.6).
    ControlByte,
    /// `0x9E..=0x9F`: GOTO-LABEL byte (Section 7.7).
    GotoLabel,
    /// `0xA0..=0xFD`: printable text with the high bit set (Section 7.1).
    PrintableText,
    /// `0xFE`: multi-byte introducer aliased to `0x8C` IF-ELSE.
    IfElseAlias,
    /// `0xFF`: end-of-response marker.
    EndOfResponse,
}

/// `conversation.md §5,§6` reserved-keyword effect for the five
/// functional words in the fixed thirty-four-entry table. Returns `None`
/// for inputs that do not match a functional reserved word; callers must
/// then check the profanity/default rebuke list before falling through
/// to the ordinary NPC keyword scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservedKeywordEffect {
    /// `NAME` — run the Name entry with the engine's prefix.
    NameEntry,
    /// `JOB` or `WORK` — run the fixed Job entry.
    JobEntry,
    /// `BYE` or `THANK` — run the fixed Bye path.
    ByePath,
}

/// `conversation.md §6`: classify a typed reserved-keyword input. The
/// caller provides an already-uppercased buffer; this helper compares
/// against the five functional words and returns the entry to run.
/// Profanity/default rebuke matching is not part of the public reserved
/// list — that branch belongs to the engine's profanity sweep below.
pub fn reserved_keyword_effect(input: &[u8]) -> Option<ReservedKeywordEffect> {
    Some(match input {
        b"NAME" => ReservedKeywordEffect::NameEntry,
        b"JOB" | b"WORK" => ReservedKeywordEffect::JobEntry,
        b"BYE" | b"THANK" => ReservedKeywordEffect::ByePath,
        _ => return None,
    })
}

/// `conversation.md §6`: maximum keyword length the input pipeline
/// accepts (free-text input is capped at fifteen characters with
/// backspace handling).
pub const TLK_INPUT_MAX_LEN: usize = 15;

/// `conversation.md §6`: NPC ordinary-keyword space-boundary match. Both
/// keyword and input are bit-7-stripped, case-folded to upper case, and
/// compared from the start. The keyword must end cleanly; the typed
/// input either ends at the same point or has a literal space at that
/// position. Returns `true` for a successful match.
pub fn tlk_keyword_matches(keyword: &[u8], input: &[u8]) -> bool {
    if keyword.is_empty() {
        return false;
    }
    if input.len() < keyword.len() {
        return false;
    }
    let mut idx = 0;
    while idx < keyword.len() {
        let k = keyword[idx] & 0x7F;
        let i = input[idx] & 0x7F;
        if k.eq_ignore_ascii_case(&i) == false {
            return false;
        }
        idx += 1;
    }
    if input.len() == keyword.len() {
        return true;
    }
    input[keyword.len()] == b' '
}

/// Classify one `.TLK` byte through the dispatcher table per
/// `conversation.md §7`. The classification order matters because the
/// `0x9E..=0x9F` GOTO-LABEL range is a sub-range of the `0x80..=0x9F`
/// control band and must take precedence.
pub const fn classify_tlk_byte(byte: u8) -> TlkByteKind {
    match byte {
        0x00 => TlkByteKind::Nul,
        0x01..=0x7F => TlkByteKind::DictionaryToken,
        0x9E | 0x9F => TlkByteKind::GotoLabel,
        0x80..=0x9F => TlkByteKind::ControlByte,
        0xA0..=0xFD => TlkByteKind::PrintableText,
        TLK_CODE_IF_ELSE_ALT => TlkByteKind::IfElseAlias,
        TLK_CODE_END_OF_RESPONSE => TlkByteKind::EndOfResponse,
    }
}
