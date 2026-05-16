//! Named constants for the `.TLK` byte-runner control bytes per
//! `systems/conversation.md` §7.2-§7.7. The runner classifies each byte by
//! value range and either emits printable output, runs a control action, or
//! stops the current stream.
//!
//! The full byte runner is not yet wired into the engine; these constants
//! exist so that helpers and tests can refer to spec-named codes without
//! magic numbers.

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

pub const RESERVED_KEYWORD_TABLE_ENTRIES: usize = 34;

/// `conversation.md §5` count of functional-word entries in the
/// reserved table (NAME, JOB, WORK, BYE, THANK).
pub const RESERVED_KEYWORD_FUNCTIONAL_COUNT: usize = 5;

/// `conversation.md §5` count of profanity / default-rebuke entries
/// in the reserved table.
pub const RESERVED_KEYWORD_REBUKE_COUNT: usize =
    RESERVED_KEYWORD_TABLE_ENTRIES - RESERVED_KEYWORD_FUNCTIONAL_COUNT;

/// `conversation.md §7.7` per-blob label count. The byte runner
/// supports up to fifteen distinct label bytes per NPC blob,
/// occupying values `0x91..=0x9F`. Labels are byte-level flow
/// markers, not globally unique names; shipped blobs commonly reuse
/// the same label byte multiple times (transfer + record).
pub const TLK_LABEL_BYTE_COUNT: usize = 15;

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

/// `conversation.md §6` keyword-input loop prompt. The conversation
/// engine prints this string into the active text window at every
/// loop iteration, leaving the cursor on the next line ready to
/// accept input. The literal `\n` and trailing `:` are part of the
/// published envelope; display layers that own their own newline
/// handling may strip the embedded newline.
pub const TLK_KEYWORD_PROMPT: &str = "Your interest?\n:";

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
pub const TLK_NO_KEYWORD_MATCH_MESSAGE: &str = "I cannot help thee with that.";

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

/// `conversation.md §7`: classify a byte by the value-range table that
/// the byte runner's top-level dispatcher follows in order. The order
/// matters because `0x9E..=0x9F` would otherwise be subsumed by the
/// `0x80..=0x9F` control-code range; the dispatcher carves the GOTO
/// pair out first.
pub const fn tlk_byte_runner_class(byte: u8) -> TlkByteRunnerClass {
    match byte {
        0x00 => TlkByteRunnerClass::NullByte,
        0x01..=0x7F => TlkByteRunnerClass::DictionaryToken,
        0x9E..=0x9F => TlkByteRunnerClass::GotoLabel,
        0xA0..=0xFD => TlkByteRunnerClass::PrintableText,
        0x80..=0x9D => TlkByteRunnerClass::ControlCode,
        0xFE => TlkByteRunnerClass::IfElseAlias,
        0xFF => TlkByteRunnerClass::EndOfResponse,
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
    Some(
        (h - b'0') as u16 * 100
            + (t - b'0') as u16 * 10
            + (u - b'0') as u16,
    )
}

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

/// `conversation.md §6` three-way fan-out the keyword input loop
/// performs after reading a free-text line. The empty-input shortcut
/// runs the NPC's Bye entry; a reserved-table hit runs the published
/// engine entry; everything else falls through to the per-NPC
/// keyword-pair scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkPlayerInputKind {
    /// Player pressed Enter on an empty line — engine prints
    /// `BYE\n\n` and runs the NPC's Bye entry through the byte
    /// runner.
    EmptyByeShortcut,
    /// Input matched one of the five reserved functional words —
    /// engine runs the named published entry.
    Reserved(ReservedKeywordEffect),
    /// Reserved scan missed — engine walks the per-NPC ordinary
    /// keyword/response pairs after the five mandatory leading
    /// entries.
    OrdinaryKeywordScan,
}

/// `conversation.md §6`: fold the keyword-loop's three observable
/// outcomes for the typed input. Caller supplies an uppercased buffer
/// (the input pipeline already capitalises the line). Profanity /
/// default rebuke matching is not part of this fan-out — the engine
/// sweeps it independently as a side-effect of the reserved scan.
pub fn tlk_player_input_kind(input: &[u8]) -> TlkPlayerInputKind {
    if input.is_empty() {
        return TlkPlayerInputKind::EmptyByeShortcut;
    }
    if let Some(effect) = reserved_keyword_effect(input) {
        return TlkPlayerInputKind::Reserved(effect);
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
