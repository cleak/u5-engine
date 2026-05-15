//! Named constants for the `.TLK` byte-runner control bytes per
//! `systems/conversation.md` §7.2-§7.7. The runner classifies each byte by
//! value range and either emits printable output, runs a control action, or
//! stops the current stream.
//!
//! The full byte runner is not yet wired into the engine; these constants
//! exist so that helpers and tests can refer to spec-named codes without
//! magic numbers.

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
