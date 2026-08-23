//! Named constants for the `.TLK` byte-runner control bytes per
//! `systems/conversation.md` §7.2-§7.7. The runner classifies each byte by
//! value range and either emits printable output, runs a control action, or
//! stops the current stream.
//!
//! These constants give the runner, conversation session, and gameplay
//! wrappers spec-named control bytes instead of magic numbers.

/// `conversation.md §7.5` print-mask state for the byte runner.
/// `0x8E` toggles the mask's high bit. While the mask is flipped,
/// printable bytes are queued without the high bit, so spaces and
/// literal-newline bytes inside the run no longer trigger the
/// normal immediate flush. Shipped dialogue uses matched `0x8E`
/// pairs around short protected uppercase strings (mantras, Words
/// of Power, passwords, coordinate-letter notations).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkPrintMaskState {
    /// Default state: spaces and literal newlines flush the word
    /// buffer normally.
    NormalBreaks,
    /// Inside a `0x8E` protected run: spaces and literal newlines
    /// do not act as buffer breakpoints.
    ProtectedRun,
}

impl TlkPrintMaskState {
    /// `conversation.md §7.5`: returns the post-toggle state
    /// produced by emitting a `0x8E` PROTECT-RUN code.
    pub const fn toggle(self) -> Self {
        match self {
            Self::NormalBreaks => Self::ProtectedRun,
            Self::ProtectedRun => Self::NormalBreaks,
        }
    }

    /// `conversation.md §7.5`: returns `true` when the buffer's
    /// soft-break bytes (space, literal newline) should still
    /// trigger a flush. The protected-run state suppresses the
    /// normal immediate flush.
    pub const fn flushes_on_break(self) -> bool {
        matches!(self, Self::NormalBreaks)
    }
}

/// `conversation.md §5` shipped reserved-keyword table size. The
/// engine-owned vocabulary that lives outside the `.TLK` files
/// holds thirty-four entries: five functional words (`NAME`, `JOB`,
/// `WORK`, `BYE`, `THANK`) and twenty-nine profanity / default
/// rebuke words. The table is checked before the per-NPC blob
/// keyword scan.
/// `conversation.md §2` published Talk-entry refusal strings the
/// command emits before the conversation engine ever runs.
///
/// - [`TALK_NOBODY_HERE_MESSAGE`] — facing tile (and one talk-through
///   step beyond) has no NPC.
/// - [`TALK_SLEEPING_MESSAGE`] — located NPC's live tile is the
///   sleeping form; conversation is skipped.
/// - [`TALK_NO_RESPONSE_MESSAGE`] — located NPC's live tile is the
///   praying/meditating/unavailable form.
pub const TALK_NOBODY_HERE_MESSAGE: &str = "Nobody's here!";
pub const TALK_SLEEPING_MESSAGE: &str = "Zzzzzz...";
pub const TALK_NO_RESPONSE_MESSAGE: &str = "No response!";

/// `conversation.md §2` Talk status-tile filter — sleeping NPC tile.
/// The Talk command compares the candidate NPC's renderer active-object
/// frame byte against this constant; on match it emits
/// [`TALK_SLEEPING_MESSAGE`] and aborts before any shop-trigger or
/// dialog-index dispatch runs.
pub const TALK_STATUS_TILE_SLEEPING: u8 = 0xAB;

/// `conversation.md §2` Talk status-tile filter — praying / meditating
/// NPC tile. On match the Talk command emits
/// [`TALK_NO_RESPONSE_MESSAGE`] and aborts.
pub const TALK_STATUS_TILE_PRAYING: u8 = 0x9D;

/// `conversation.md §2`: route a candidate NPC's live tile byte to the
/// matching entry-time refusal message, or `None` when the NPC is
/// available for the normal conversation entry. Per `cleak/u5-spec#44`
/// only the two published status-tile constants gate the refusal; all
/// other tile values fall through.
pub const fn talk_status_tile_refusal(live_tile: u8) -> Option<&'static str> {
    match live_tile {
        TALK_STATUS_TILE_SLEEPING => Some(TALK_SLEEPING_MESSAGE),
        TALK_STATUS_TILE_PRAYING => Some(TALK_NO_RESPONSE_MESSAGE),
        _ => None,
    }
}

pub const RESERVED_KEYWORD_TABLE_ENTRIES: usize = 34;

/// `conversation.md §5` count of functional-word entries in the
/// reserved table (NAME, JOB, WORK, BYE, THANK).
pub const RESERVED_KEYWORD_FUNCTIONAL_COUNT: usize = 5;

/// `conversation.md §5` count of profanity / default-rebuke entries
/// in the reserved table.
pub const RESERVED_KEYWORD_REBUKE_COUNT: usize =
    RESERVED_KEYWORD_TABLE_ENTRIES - RESERVED_KEYWORD_FUNCTIONAL_COUNT;

pub const RESERVED_KEYWORD_FUNCTIONAL_WORDS: [&[u8]; RESERVED_KEYWORD_FUNCTIONAL_COUNT] =
    [b"NAME", b"JOB", b"WORK", b"BYE", b"THANK"];

pub const RESERVED_KEYWORD_REBUKE_WORDS: [&[u8]; RESERVED_KEYWORD_REBUKE_COUNT] = [
    b"FUCK",
    b"SHIT",
    b"DAMN",
    b"DICK",
    b"PRICK",
    b"PUSSY",
    b"CUNT",
    b"ASS",
    b"BUTT",
    b"BOOGER",
    b"PISS",
    b"JACK OFF",
    b"MASTURBATE",
    b"SUCK",
    b"FART",
    b"TITS",
    b"BOOB",
    b"MELONS",
    b"BLOW",
    b"PENIS",
    b"BREAST",
    b"CLIT",
    b"BALLS",
    b"SCROTUM",
    b"NUTS",
    b"BULLSHIT",
    b"CUM",
    b"CROTCH",
    b"MOTHERFUCKER",
];

/// `conversation.md §7.7` per-blob label count. The byte runner
/// supports up to fifteen distinct label bytes per NPC blob,
/// occupying values `0x91..=0x9F`. Labels are byte-level flow
/// markers, not globally unique names; shipped blobs commonly reuse
/// the same label byte multiple times (transfer + record). Anchored
/// to `TLK_LABEL_LAST - TLK_LABEL_FIRST + 1` so the label-count
/// derives from the published label-byte band.
pub const TLK_LABEL_BYTE_COUNT: usize = (TLK_LABEL_LAST - TLK_LABEL_FIRST + 1) as usize;

/// `conversation.md §7.7`: returns the zero-based label index
/// `0..=14` for a label byte in the `0x91..=0x9F` range, or `None`
/// for non-label bytes. Useful for callers that want a dense index
/// rather than the raw control byte.
pub const fn tlk_label_index(byte: u8) -> Option<u8> {
    if byte < TLK_LABEL_FIRST || byte > TLK_LABEL_LAST {
        None
    } else {
        Some(byte - TLK_LABEL_FIRST)
    }
}

/// `conversation.md §4` mandatory leading entries every NPC blob
/// begins with, in disk order. After these five NUL-terminated
/// entries comes the variable-size keyword body (alternating
/// keyword string + response stream pairs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkLeadingEntry {
    /// Index 0 — display name (e.g. "Jennifer").
    Name,
    /// Index 1 — short prose description (e.g. "a weathered girl").
    Description,
    /// Index 2 — greeting on first address.
    Greeting,
    /// Index 3 — `JOB`-keyword response.
    Job,
    /// Index 4 — `BYE`-keyword and conversation-exit response.
    Bye,
}

/// `conversation.md §4`: number of mandatory leading entries (5).
pub const TLK_LEADING_ENTRY_COUNT: usize = 5;

/// `conversation.md §4`: in-blob index `0..=4` for the supplied
/// leading entry.
pub const fn tlk_leading_entry_index(entry: TlkLeadingEntry) -> usize {
    match entry {
        TlkLeadingEntry::Name => 0,
        TlkLeadingEntry::Description => 1,
        TlkLeadingEntry::Greeting => 2,
        TlkLeadingEntry::Job => 3,
        TlkLeadingEntry::Bye => 4,
    }
}

/// `conversation.md §2` reasons the resident "can talk now"
/// liveness gate refuses the Talk command before any narration runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkRefusal {
    /// Player is in an active combat round.
    InCombat,
    /// Active player is currently asleep.
    Asleep,
    /// Active player is currently starving.
    Starving,
    /// Engine is already running another conversation.
    AlreadyInConversation,
}

/// `conversation.md §2`: select the most-applicable refusal reason
/// for the liveness gate. Returns `None` when the gate accepts and
/// the Talk handler can proceed.
pub const fn talk_liveness_refusal(
    in_combat: bool,
    asleep: bool,
    starving: bool,
    already_in_conversation: bool,
) -> Option<TalkRefusal> {
    if in_combat {
        Some(TalkRefusal::InCombat)
    } else if asleep {
        Some(TalkRefusal::Asleep)
    } else if starving {
        Some(TalkRefusal::Starving)
    } else if already_in_conversation {
        Some(TalkRefusal::AlreadyInConversation)
    } else {
        None
    }
}

/// `conversation.md §3` shipped per-class `.TLK` NPC counts (each
/// includes the leading sentinel slot, so live NPCs use indices
/// `2..npc_count`).
pub const TOWNE_TLK_NPCS: usize = 48;
pub const DWELLING_TLK_NPCS: usize = 15;
pub const CASTLE_TLK_NPCS: usize = 40;
pub const KEEP_TLK_NPCS: usize = 32;

/// `conversation.md §3` `.TLK` file class selected from the active
/// scene byte. Talk dispatch only resolves a dialog index against the
/// file matching the scene's location class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkFileClass {
    /// Scenes `1..=8` — `TOWNE.TLK`.
    Towne,
    /// Scenes `9..=16` — `DWELLING.TLK`.
    Dwelling,
    /// Scenes `17..=24` — `CASTLE.TLK`.
    Castle,
    /// Scenes `25..=32` — `KEEP.TLK`.
    Keep,
}

impl TlkFileClass {
    /// `conversation.md §3` shipped header NPC count for this class
    /// (includes the leading sentinel slot).
    pub const fn shipped_npc_count(self) -> usize {
        match self {
            Self::Towne => TOWNE_TLK_NPCS,
            Self::Dwelling => DWELLING_TLK_NPCS,
            Self::Castle => CASTLE_TLK_NPCS,
            Self::Keep => KEEP_TLK_NPCS,
        }
    }
}

/// `conversation.md §3`: classify the active scene byte to its
/// `.TLK` file class. Mapping is `(scene_id - 1) >> 3` over
/// `1..=32`; scene `0` (overworld) and any value above `32` have no
/// `.TLK` file because Talk is unavailable there.
pub const fn tlk_class_for_scene(scene_byte: u8) -> Option<TlkFileClass> {
    if scene_byte == 0 || scene_byte > 32 {
        return None;
    }
    Some(match (scene_byte - 1) >> 3 {
        0 => TlkFileClass::Towne,
        1 => TlkFileClass::Dwelling,
        2 => TlkFileClass::Castle,
        _ => TlkFileClass::Keep,
    })
}

/// `conversation.md §8`: shared common-word dictionary has 128
/// entries. The dialogue runner and the shop renderer apply different
/// byte-range biases when reaching this same logical table.
pub const COMMON_WORD_DICTIONARY_ENTRIES: usize = 128;

/// `catalogs/quest-graph.md §3` Resistance-trust password. NPCs allied
/// with the Resistance accept this typed answer as a trust gate that
/// unlocks anti-Blackthorn branches, Council-member help, and useful
/// item grants. The wider trust system is data-authored in each NPC's
/// `.TLK` keyword tree; this constant just pins the spec-named string.
pub const QUEST_PASSWORD_RESISTANCE: &str = "DAWN";
/// `catalogs/quest-graph.md §3` Blackthorn-side oppression password.
/// Typing this proves Blackthorn-aligned allegiance to NPCs that gate
/// hostile or dangerous infiltration branches.
pub const QUEST_PASSWORD_OPPRESSION: &str = "IMPERA";

/// `conversation.md §7.6` ASK-PARTY-NAME (`0x84`) match. The typed
/// answer is compared against each live party member's name with
/// the bit-7-stripping case-insensitive convention also used by
/// the ordinary keyword scanner. Returns the matched 1-based slot
/// index on a successful match, or `0` when no member's name
/// matches.
///
/// The match is whole-string equality after stripping bit 7 and
/// folding case; the function does not look for substrings or word
/// boundaries. Empty names never match (callers should skip empty
/// roster slots before passing them in).
pub fn tlk_ask_party_name_match(typed: &[u8], party_member_names: &[&[u8]]) -> u8 {
    for (zero_index, name) in party_member_names.iter().enumerate() {
        if name.is_empty() || name.len() != typed.len() {
            continue;
        }
        let mut matched = true;
        let mut i = 0;
        while i < name.len() {
            let n = name[i] & 0x7F;
            let t = typed[i] & 0x7F;
            if !n.eq_ignore_ascii_case(&t) {
                matched = false;
                break;
            }
            i += 1;
        }
        if matched {
            return (zero_index + 1) as u8;
        }
    }
    0
}

/// `shops.md §4.2` shared common-word NUL-sentinel count, including
/// token `0x00` plus the published empty dictionary rows. Text consumers
/// treat these as word-boundary sentinels rather than as word substitutions.
pub const COMMON_WORD_DICTIONARY_NUL_SENTINELS: usize = 11;

/// `shops.md §4.2` SHOPPE.DAT phrase-token byte range. Bytes
/// `0x80..=0xFF` in a record payload index the 128-entry pointer
/// table; the conversation engine uses the same table through its
/// own low-byte range.
pub const SHOPPE_PHRASE_TOKEN_FIRST: u8 = 0x80;
pub const SHOPPE_PHRASE_TOKEN_LAST: u8 = 0xFF;

/// `conversation.md §8` / `formats/tlk.md §9,§10` TLK dialogue
/// dictionary token range: `0x01..=0x80`, biased by one onto the
/// 128-entry common-word pointer table, so `0x01` is entry zero and
/// `0x80` is entry 127.
///
/// **`0x80` is a dictionary token, not a control code.** The older
/// framing — "high bit clear means dictionary token", with the band
/// ending at `0x7F` — is off by one at exactly that boundary, and
/// `formats/tlk.md §9`'s dispatch table marks `0x80` "not a control
/// code". Two independent checks:
///
/// * **Arithmetic.** The table has 128 entries and needs 128 token
///   values. `0x01..=0x7F` is 127, so under the old bound the last
///   entry was unreachable by any token.
/// * **Shipped bytes.** `0x80` appears mid-payload thirteen times
///   across the shipped corpus, always between ordinary text, and
///   entry 127 is `work`: `TOWNE.TLK` has `I <0x80> hard-` and
///   `We <0x80> long days,`; `CASTLE.TLK` has `Mystic arms <0x80>
///   near`; `KEEP.TLK` has `reference <0x80> known`. Every one reads
///   as English only if `0x80` expands to a word, and rendered as a
///   control byte instead — which is what we did.
pub const TLK_DICTIONARY_TOKEN_FIRST: u8 = 0x01;
pub const TLK_DICTIONARY_TOKEN_LAST: u8 = 0x80;

/// `conversation.md §8` / `formats/tlk.md §10`: TLK dialogue
/// dictionary tokens run `TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST`
/// (`0x01..=0x80`). Token `0x01` resolves to dictionary entry zero and
/// token `0x80` to entry 127; note the band is **not** "high bit
/// clear", since its last member has bit seven set.
pub const fn tlk_dictionary_index(token: u8) -> Option<usize> {
    if token < TLK_DICTIONARY_TOKEN_FIRST || token > TLK_DICTIONARY_TOKEN_LAST {
        None
    } else {
        Some((token - TLK_DICTIONARY_TOKEN_FIRST) as usize)
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

/// `formats/tlk.md §9.2`: on-disk encoding of the printable double
/// quote (`"`, low-ASCII `0x22`) under the bit-7 XOR scheme. The
/// byte runner remembers the previous emitted printable byte and
/// suppresses a quote when that previous byte was also a quote so
/// adjacent `""` segments collapse to a single visible quote.
/// Promote the sentinel so the dedup helper can name the on-disk
/// byte instead of repeating the bare literal `0xA2`.
pub const TLK_DOUBLE_QUOTE_ENCODED: u8 = b'"' ^ TLK_TEXT_XOR_MASK;

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
pub const TLK_CODE_LABEL_RECORD: u8 = 0x90;
pub const TLK_LABEL_FIRST: u8 = 0x91;
/// `conversation.md §7.7`: the label band ends at the last
/// GOTO-label byte (0x9F). Anchored to
/// [`TLK_CODE_GOTO_LABEL_LAST`] so the label band end and the
/// GOTO-label pair end share one source of truth.
pub const TLK_LABEL_LAST: u8 = TLK_CODE_GOTO_LABEL_LAST;
/// `conversation.md §7.7`: the GOTO-label pair (0x9E, 0x9F) sits
/// immediately past the engine-control byte band and forms a
/// two-byte pair. Anchor the first label to
/// [`TLK_CONTROL_CODE_LAST`] + 1 and the last label to
/// FIRST + 1 so the GOTO-label adjacency has one source of
/// truth.
pub const TLK_CODE_GOTO_LABEL_FIRST: u8 = TLK_CONTROL_CODE_LAST + 1;
pub const TLK_CODE_GOTO_LABEL_LAST: u8 = TLK_CODE_GOTO_LABEL_FIRST + 1;
pub const TLK_CODE_END_OF_RESPONSE: u8 = 0xFF;

/// `conversation.md §6` keyword-input loop prompt. The conversation
/// engine prints this string into the active text window at every
/// loop iteration, leaving the cursor on the next line ready to
/// accept input. The literal `\n` and trailing `:` are part of the
/// published envelope; display layers that own their own newline
/// handling may strip the embedded newline.
pub const TLK_KEYWORD_PROMPT: &str = "Your interest?\n:";

/// `conversation.md §9` opening preamble emitted before the NPC
/// Description entry.
pub const TLK_OPENING_DESCRIPTION_PREFIX: &str = "Thou seest ";

/// `conversation.md §6` empty-input shortcut. Pressing Enter on an
/// empty line prints this line and runs the NPC's `Bye` entry; it
/// is the most common way conversations end.
pub const TLK_EMPTY_INPUT_BYE_MESSAGE: &str = "BYE\n\n";

/// `conversation.md §6` no-match response. When both the reserved
/// keyword scan and the ordinary keyword scan fail, the keyword
/// input loop prints this line and returns to step 1 to prompt
/// again. The trailing `\n\n` is part of the spec's published
/// envelope; callers may strip the spacing in display layers that
/// own their own line breaks.
pub const TLK_NO_KEYWORD_MATCH_MESSAGE: &str = "I cannot help thee with that.\n\n";

pub const TLK_RESERVED_REBUKE_MESSAGE: &str =
    "With language like that, how did you become an Avatar?\n\n";
pub const TLK_RESERVED_REBUKE_PAUSE_LIMIT: u8 = 28;

/// `conversation.md §7` top-level byte-runner dispatcher class. Each
/// byte read from any text stream (the five leading entries, every
/// keyword response, IF/ELSE arm bodies, GOTO targets) is classified
/// by value range into exactly one of these branches. The classifier
/// only names the dispatch class; the per-class side effects live in
/// the dedicated control-code constants (`TLK_CODE_*`) and printable
/// text path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkByteRunnerClass {
    /// `0x01..=0x7F` — nonzero high-bit-clear dictionary token; the
    /// byte indexes the shared 128-entry common-word pointer table
    /// and the pointed word is expanded inline into the output.
    DictionaryToken,
    /// `0x9E..=0x9F` — GOTO-LABEL codes. Their high bit is set but
    /// they participate in label dispatch, not the ordinary control
    /// table.
    GotoLabel,
    /// `0xA0..=0xFD` — high-bit-set printable bytes. The word buffer
    /// strips the high bit before glyph output; the `0x8E` print-mask
    /// toggle controls whether the queued byte keeps that high bit as
    /// a soft-break marker.
    PrintableText,
    /// `0x80..=0x9D` — engine control codes (the §7.2..§7.6 table).
    /// `0x9E..=0x9F` are carved out to the `GotoLabel` branch above.
    ControlCode,
    /// `0xFE` — multi-byte command introducer that aliases `0x8C`
    /// IF/ELSE.
    IfElseAlias,
    /// `0xFF` — end-of-response. The runner flushes the pending word
    /// buffer and signals the keyword input loop to prompt again.
    EndOfResponse,
    /// `0x00` — null byte. Not a dispatcher class in the published
    /// classification; appears as a blob terminator/skip-byte at the
    /// I/O layer rather than reaching the dispatcher in normal flows.
    NullByte,
}

/// `conversation.md §7` printable-text byte range. The byte runner's
/// printable branch accepts high-bit-set bytes in `0xA0..=0xFD`. The
/// word-buffer strips the high bit before glyph output.
/// `conversation.md §7`: the printable-text byte band begins
/// immediately past the label band end (0x9F). Anchored to
/// [`TLK_LABEL_LAST`] + 1 so the label→printable-text adjacency
/// has one source of truth.
pub const TLK_PRINTABLE_TEXT_FIRST: u8 = TLK_LABEL_LAST + 1;
pub const TLK_PRINTABLE_TEXT_LAST: u8 = 0xFD;

/// `conversation.md §7` engine-control byte range. The byte runner's
/// control-code branch accepts `0x80..=0x9D`; the `0x9E..=0x9F` GOTO
/// label pair is carved out by [`TLK_CODE_GOTO_LABEL_FIRST`] /
/// [`TLK_CODE_GOTO_LABEL_LAST`] before this range is matched. The
/// control-code range begins at `0x81`.
///
/// This is a **literal, deliberately not** anchored to
/// [`TLK_DICTIONARY_TOKEN_LAST`] `+ 1`. The two bands being adjacent
/// by construction is what let the dictionary band's off-by-one at
/// `0x80` propagate straight into the control-code band and steal a
/// token from it. The adjacency is a fact about these two published
/// values, not a rule either of them enforces on the other.
pub const TLK_CONTROL_CODE_FIRST: u8 = 0x81;
pub const TLK_CONTROL_CODE_LAST: u8 = 0x9D;

/// `conversation.md §7`: classify a byte by the value-range table that
/// the byte runner's top-level dispatcher follows in order. The order
/// matters because `0x9E..=0x9F` would otherwise be subsumed by the
/// `0x81..=0x9F` control-code range; the dispatcher carves the GOTO
/// pair out first.
pub const fn tlk_byte_runner_class(byte: u8) -> TlkByteRunnerClass {
    match byte {
        0x00 => TlkByteRunnerClass::NullByte,
        TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST => {
            TlkByteRunnerClass::DictionaryToken
        }
        TLK_CODE_GOTO_LABEL_FIRST..=TLK_CODE_GOTO_LABEL_LAST => TlkByteRunnerClass::GotoLabel,
        TLK_PRINTABLE_TEXT_FIRST..=TLK_PRINTABLE_TEXT_LAST => TlkByteRunnerClass::PrintableText,
        TLK_CONTROL_CODE_FIRST..=TLK_CONTROL_CODE_LAST => TlkByteRunnerClass::ControlCode,
        TLK_CODE_IF_ELSE_ALT => TlkByteRunnerClass::IfElseAlias,
        TLK_CODE_END_OF_RESPONSE => TlkByteRunnerClass::EndOfResponse,
    }
}

/// `conversation.md §7.6`: returns `true` when the `0xFE` IF-ELSE-ALT
/// runner branches to the supplied target label. The runner branches
/// when the shared moral-standing selector is at or above the
/// threshold byte; below the threshold the runner falls through to
/// the *then* arm without touching the target-label argument.
pub const fn tlk_if_else_alt_branches(standing: u8, threshold: u8) -> bool {
    standing >= threshold
}

/// `conversation.md §7.6` published `0x86` ACTION-DISPATCH letter
/// verbs `A..=K`. The argument byte is masked to seven bits before
/// dispatch; values below `b'A'` set generic one-conversation signal
/// flags rather than running the global action table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkActionDispatchVerb {
    /// `A` — raise the shared food counter (and refresh presentation).
    RaiseFood,
    /// `B` — raise the shared gold counter.
    RaiseGold,
    /// `C` — raise the ordinary key counter.
    RaiseKeys,
    /// `D` — raise the gem counter.
    RaiseGems,
    /// `E` — raise the torch counter.
    RaiseTorches,
    /// `F` — set the outdoor Klimb-gear / Grapple-gate save byte.
    SetGrappleGate,
    /// `G` — raise the carried magic-carpet counter.
    RaiseCarpets,
    /// `H` — set the Sextant carried-item flag.
    SetSextantCarried,
    /// `I` — set the Spyglass carried-item flag.
    SetSpyglassCarried,
    /// `J` — set the Black Badge carried-item flag.
    SetBlackBadgeCarried,
    /// `K` — raise the skull/special-key counter.
    RaiseSkullKeys,
}

/// `conversation.md §7.6`: classify the post-mask `0x86` action-dispatch
/// argument byte. Returns the published letter verb for `A..=K`; any
/// value below `b'A'` (the generic one-conversation signal-flag band)
/// or above `b'K'` returns `None`.
pub const fn tlk_action_dispatch_verb(arg: u8) -> Option<TlkActionDispatchVerb> {
    Some(match arg {
        b'A' => TlkActionDispatchVerb::RaiseFood,
        b'B' => TlkActionDispatchVerb::RaiseGold,
        b'C' => TlkActionDispatchVerb::RaiseKeys,
        b'D' => TlkActionDispatchVerb::RaiseGems,
        b'E' => TlkActionDispatchVerb::RaiseTorches,
        b'F' => TlkActionDispatchVerb::SetGrappleGate,
        b'G' => TlkActionDispatchVerb::RaiseCarpets,
        b'H' => TlkActionDispatchVerb::SetSextantCarried,
        b'I' => TlkActionDispatchVerb::SetSpyglassCarried,
        b'J' => TlkActionDispatchVerb::SetBlackBadgeCarried,
        b'K' => TlkActionDispatchVerb::RaiseSkullKeys,
        _ => return None,
    })
}

/// `conversation.md §7.6`: returns `true` when the `0x86` argument
/// byte (after the seven-bit mask) falls in the generic
/// one-conversation signal-flag band — values below `b'A'` set a
/// per-conversation flag rather than running the global action table.
pub const fn tlk_action_dispatch_is_signal_flag(arg: u8) -> bool {
    arg < b'A'
}

/// `quest-flags.md §4`: numeric `0x86` action-dispatch arguments
/// below `A` address the generic transient conversation signal band.
/// Values are post-mask (`0x00..=0x40`), so the array needs exactly
/// `b'A'` slots.
pub const TLK_GENERIC_SIGNAL_COUNT: usize = b'A' as usize;
/// `conversation.md §7.6`: numeric `0x86` action-dispatch signal
/// slots increment through the shared capped byte helper.
pub const TLK_GENERIC_SIGNAL_CAP: u8 = 99;

/// `quest-flags.md §5`: final conversation cleanup first checks a
/// three-slot resource/special transient band before scanning generic
/// signal flags.
pub const CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT: usize = 3;

/// `quest-flags.md §5`: after generic signals, cleanup scans two
/// eight-slot transient signal arrays from high to low.
pub const CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT: usize = 8;

/// `quest-flags.md §5` / `shops.md §6.2`: the shared town/conversation
/// sentinel uses a no-slot marker distinct from tracked slot indices.
pub const CONVERSATION_SHARED_NO_SLOT_SENTINEL: u8 = 0xFF;

/// `quest-flags.md §5`: random gold fallback subtracts `1..=15`.
pub const fn conversation_cleanup_gold_debit_from_seed(seed: u8) -> u16 {
    (seed as u16 % 15) + 1
}

/// `conversation.md §7.6`: decode the `0x85` GOLD-PAYMENT introducer's
/// three argument bytes. Each byte is masked to seven bits and
/// interpreted as an ASCII decimal digit; the three digits compose a
/// hundreds/tens/units amount in the range `0..=999`. Returns `None`
/// for any argument byte that does not yield an ASCII digit
/// `0x30..=0x39` after the seven-bit mask.
pub const fn tlk_gold_payment_amount(arg0: u8, arg1: u8, arg2: u8) -> Option<u16> {
    let h = arg0 & 0x7F;
    let t = arg1 & 0x7F;
    let u = arg2 & 0x7F;
    if h < b'0' || h > b'9' || t < b'0' || t > b'9' || u < b'0' || u > b'9' {
        return None;
    }
    Some((h - b'0') as u16 * 100 + (t - b'0') as u16 * 10 + (u - b'0') as u16)
}

/// `formats/tlk.md §9.1` argument-byte width for the GOLD-PAYMENT
/// introducer (`0x85`): three ASCII digit bytes encoding the decimal
/// gold amount.
pub const TLK_GOLD_PAYMENT_ARGUMENT_BYTES: u8 = 3;
/// `formats/tlk.md §9.1` argument-byte width for the ACTION-DISPATCH
/// (`0x86`) and IF-ELSE (`0x8C`) introducers: a single argument byte
/// each, masked to seven bits at the runtime layer.
pub const TLK_ONE_BYTE_INTRODUCER_ARGUMENT_BYTES: u8 = 1;
/// `formats/tlk.md §9.1` argument-byte width for the IF-ELSE-ALT
/// introducer (`0xFE`): two argument bytes (a moral-standing threshold
/// followed by a target label byte).
pub const TLK_IF_ELSE_ALT_ARGUMENT_BYTES: u8 = 2;

/// `conversation.md` §7.6: argument-byte width for each multi-byte
/// introducer code. Returns `None` for codes that take no follow-up bytes.
pub const fn tlk_introducer_argument_count(code: u8) -> Option<u8> {
    match code {
        TLK_CODE_GOLD_PAYMENT => Some(TLK_GOLD_PAYMENT_ARGUMENT_BYTES),
        TLK_CODE_ACTION_DISPATCH | TLK_CODE_IF_ELSE => Some(TLK_ONE_BYTE_INTRODUCER_ARGUMENT_BYTES),
        TLK_CODE_IF_ELSE_ALT => Some(TLK_IF_ELSE_ALT_ARGUMENT_BYTES),
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
/// helper compares against the five functional words with the fixed
/// table's space-boundary match and returns the entry to run.
pub fn reserved_keyword_effect(input: &[u8]) -> Option<ReservedKeywordEffect> {
    Some(match reserved_functional_keyword_index(input)? {
        0 => ReservedKeywordEffect::NameEntry,
        1 | 2 => ReservedKeywordEffect::JobEntry,
        3 | 4 => ReservedKeywordEffect::ByePath,
        _ => return None,
    })
}

pub fn reserved_functional_keyword_index(input: &[u8]) -> Option<usize> {
    RESERVED_KEYWORD_FUNCTIONAL_WORDS
        .iter()
        .position(|keyword| tlk_keyword_matches(keyword, input))
}

pub fn reserved_rebuke_keyword_index(input: &[u8]) -> Option<usize> {
    RESERVED_KEYWORD_REBUKE_WORDS
        .iter()
        .position(|keyword| tlk_keyword_matches(keyword, input))
}

/// `conversation.md §6`: maximum keyword length the input pipeline
/// accepts (free-text input is capped at fifteen characters with
/// backspace handling).
pub const TLK_INPUT_MAX_LEN: usize = 15;

/// `conversation.md §6` fan-out the keyword input loop
/// performs after reading a free-text line. The empty-input shortcut
/// runs the NPC's Bye entry; a functional reserved-table hit runs the
/// published engine entry; a rebuke hit prints the fixed chastisement;
/// everything else falls through to the per-NPC keyword-pair scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkPlayerInputKind {
    /// Player pressed Enter on an empty line — engine prints
    /// `BYE\n\n` and runs the NPC's Bye entry through the byte
    /// runner.
    EmptyByeShortcut,
    /// Input matched one of the five reserved functional words —
    /// engine runs the named published entry.
    Reserved(ReservedKeywordEffect),
    /// Input matched one of the fixed rebuke words; engine prints
    /// the reserved chastisement and returns to the keyword prompt.
    ReservedRebuke { table_index: usize },
    /// Reserved scan missed — engine walks the per-NPC ordinary
    /// keyword/response pairs after the five mandatory leading
    /// entries.
    OrdinaryKeywordScan,
}

/// `conversation.md §6`: fold the keyword-loop's observable outcomes
/// for the typed input. Caller normally supplies an uppercased buffer,
/// and the shared keyword matcher also folds case and strips bit 7.
pub fn tlk_player_input_kind(input: &[u8]) -> TlkPlayerInputKind {
    if input.is_empty() {
        return TlkPlayerInputKind::EmptyByeShortcut;
    }
    if let Some(effect) = reserved_keyword_effect(input) {
        return TlkPlayerInputKind::Reserved(effect);
    }
    if let Some(index) = reserved_rebuke_keyword_index(input) {
        return TlkPlayerInputKind::ReservedRebuke {
            table_index: RESERVED_KEYWORD_FUNCTIONAL_COUNT + index,
        };
    }
    TlkPlayerInputKind::OrdinaryKeywordScan
}

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
/// `0x9E..=0x9F` GOTO-LABEL range is a sub-range of the `0x81..=0x9F`
/// control band and must take precedence.
pub const fn classify_tlk_byte(byte: u8) -> TlkByteKind {
    match byte {
        0x00 => TlkByteKind::Nul,
        TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST => TlkByteKind::DictionaryToken,
        TLK_CODE_GOTO_LABEL_FIRST | TLK_CODE_GOTO_LABEL_LAST => TlkByteKind::GotoLabel,
        TLK_CONTROL_CODE_FIRST..=TLK_CODE_GOTO_LABEL_LAST => TlkByteKind::ControlByte,
        TLK_PRINTABLE_TEXT_FIRST..=TLK_PRINTABLE_TEXT_LAST => TlkByteKind::PrintableText,
        TLK_CODE_IF_ELSE_ALT => TlkByteKind::IfElseAlias,
        TLK_CODE_END_OF_RESPONSE => TlkByteKind::EndOfResponse,
    }
}
