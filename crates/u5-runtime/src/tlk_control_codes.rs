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
/// `conversation.md §2` step 4 publishes this as the mirror-tile
/// refusal. A paired capture of the original shows it is also what a
/// **non-speaker** answers: Talk at a Britain guard whose dialog index
/// is 0 - §2 step 5's own example of index 0, "a guard" - prints this
/// same line, on ordinary floor with no mirror in play. §2 step 5 does
/// not publish a literal for index 0 at all, and this engine had been
/// filling the hole with Ultima IV's "They give thee a funny look.",
/// which appears nowhere in the specification. Reported on
/// cleak/u5-spec#198.
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
/// the same label byte multiple times (transfer + record).
///
/// A **literal**, taken from §7.7's own words ("up to fifteen label
/// bytes per blob"), not computed from
/// [`TLK_LABEL_LAST`]` - `[`TLK_LABEL_FIRST`]` + 1`. §7.7 publishes
/// the count and the band as two statements and then observes that
/// they agree; a derived count instead *reports* whatever the band
/// says, so a band read short reports a short count with no test able
/// to notice. The agreement is asserted as a consequence in
/// `tlk_label_byte_count_matches_published_band_width`.
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

/// `conversation.md §7.6` ASK-WHO (`0x88`) match.
///
/// The published rule is deliberately looser than the top-level keyword
/// scan: "For each active party slot in order, the engine takes the
/// **first four characters** of that member's name and searches for them
/// as a substring of the typed line. A hit counts only at the start of
/// the line or immediately after a literal space; a hit in the middle of
/// a longer word is rejected and the scan continues with the next member.
/// The first accepted hit ends the scan, sets the bit, and prints the
/// affirmative line. Empty input is its own early exit and never sets the
/// bit."
///
/// Returns the matched 1-based party slot, or `0` for empty input or no
/// match. Both sides are compared with bit 7 stripped and case folded,
/// the same convention the ordinary keyword scanner uses.
///
/// *Corrected:* `0x88` previously ran through a whole-string equality
/// matcher (`tlk_ask_party_name_match`) whose own doc comment said it
/// "does not look for substrings or word boundaries" — the exact opposite
/// of the published rule, and strict enough that a typed "my friend Iolo"
/// never matched.
pub fn tlk_ask_who_match(typed: &[u8], party_member_names: &[&[u8]]) -> u8 {
    if typed.is_empty() {
        return 0;
    }
    let folded: Vec<u8> = typed
        .iter()
        .map(|byte| (byte & 0x7F).to_ascii_uppercase())
        .collect();
    for (zero_index, name) in party_member_names.iter().enumerate() {
        let prefix_len = name.len().min(TLK_ASK_WHO_NAME_PREFIX_LEN);
        if prefix_len == 0 {
            continue;
        }
        let prefix: Vec<u8> = name[..prefix_len]
            .iter()
            .map(|byte| (byte & 0x7F).to_ascii_uppercase())
            .collect();
        let accepted = folded
            .windows(prefix_len)
            .enumerate()
            .any(|(at, window)| (at == 0 || folded[at - 1] == b' ') && window == prefix.as_slice());
        if accepted {
            return (zero_index + 1) as u8;
        }
    }
    0
}

/// `common-word-dictionary.md §3` null-reference count in the resident
/// pointer run: the unreachable index-zero entry plus ten addressable empty
/// slots. These are not word-boundary sentinels.
pub const COMMON_WORD_DICTIONARY_NULL_REFERENCES: usize = 11;

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

// §7.4 moral-standing and newline codes
/// `conversation.md §7.4` STANDING-UP. Raises the shared moral-standing
/// selector by one through the ordinary capped-add writer, clamped at
/// [`crate::MORAL_STANDING_MAX`]. Emits no text and does not touch the
/// word buffer. With [`TLK_CODE_STANDING_DOWN`] it is one of the byte
/// runner's only two direct writers of the selector (`karma.md §4`).
pub const TLK_CODE_STANDING_UP: u8 = 0x89;
/// `conversation.md §7.4` STANDING-DOWN. Lowers the shared
/// moral-standing selector by one through the ordinary capped-subtract
/// writer, floored at zero. Emits no text and does not touch the word
/// buffer.
///
/// *Corrected:* this engine previously named the byte
/// `TLK_CODE_PANEL_NEWLINE` and dispatched it as a newline, following an
/// earlier revision of the spec. §7.4 withdraws that reading: "the
/// literal-newline code is `0x8D` and only `0x8D`; the value `0x8A`
/// acquires its newline meaning only *inside* the word buffer, because
/// the printable-text path rewrites a `0x8D` to `0x8A` after
/// control-code dispatch has already been passed." The two meanings
/// coexist at different stages, so the dispatcher must read `0x8A` as a
/// standing writer and never as a newline. §7.4 also notes the live
/// cost of getting this wrong: five of the eight shipped `0x8A` bytes
/// head the "no" arm of a scoped prompt, two of them gold requests, so
/// declining a gold request must cost the party standing.
pub const TLK_CODE_STANDING_DOWN: u8 = 0x8A;
/// `conversation.md §7.4` LITERAL-NEWLINE — the only newline control
/// code. See [`TLK_CODE_STANDING_DOWN`] for the withdrawn `0x8A`
/// reading.
pub const TLK_CODE_LITERAL_NEWLINE: u8 = 0x8D;

// §7.5 print-mask and curse codes
pub const TLK_CODE_CURSE_CHECK: u8 = 0x8B;
pub const TLK_CODE_PROTECT_RUN: u8 = 0x8E;

// §7.6 branching, recruitment, and transactional codes
/// `conversation.md §7.6` RECRUIT-SPEAKER. Takes **no argument bytes,
/// prompts for nothing, and reads no input**: the engine takes the
/// speaker's own name from the loaded blob's Name entry and matches its
/// opening characters against the reserve portion of the sixteen-slot
/// character roster, scanned from the last slot downwards.
///
/// *Corrected:* this engine previously named the byte
/// `TLK_CODE_ASK_PARTY_NAME` and dispatched it as a free-text "Name?"
/// prompt matched against the *live* party. §7.6 withdraws that reading —
/// "There is no player prompt and no input read" — and §10 repeats it:
/// "RECRUIT-SPEAKER reads no input and is not a prompt".
pub const TLK_CODE_RECRUIT_SPEAKER: u8 = 0x84;
pub const TLK_CODE_GOLD_PAYMENT: u8 = 0x85;
pub const TLK_CODE_ACTION_DISPATCH: u8 = 0x86;
/// `conversation.md §7.6` KEYWORD-ALIAS. Positional, not a search: save
/// the cursor, skip the remainder of the current record, skip any run of
/// terminators, skip the whole record that follows, and run the record
/// after that as a nested stream.
///
/// *Corrected:* the historical mnemonic was SET-FLAG and an earlier
/// revision described the code as a recursive keyword *scan*. §7.6:
/// "Both are wrong. `0x87` does no string comparison, reads no player
/// input, consumes no argument byte, and writes no flag."
pub const TLK_CODE_KEYWORD_ALIAS: u8 = 0x87;
/// `conversation.md §7.6`: how far `0x87` KEYWORD-ALIAS skips, counted in
/// whole records. The published byte walk is "skip forward past the
/// remainder of the current record, past any run of terminators, and past
/// the whole record that follows; run the record after that". Against a
/// blob already split into records that is two records on, and because a
/// response record is preceded by its keyword record, two records on from
/// a response is the *next keyword's response* — the alias reading §7.6
/// gives for all six hundred forty-one shipped occurrences.
pub const TLK_KEYWORD_ALIAS_RECORD_SKIP: usize = 2;
pub const TLK_CODE_ASK_WHO: u8 = 0x88;
pub const TLK_CODE_IF_ELSE: u8 = 0x8C;
/// `conversation.md §7.6`: the reserved `0x8C` argument. Every other
/// argument value names a branch target label; this one, on the set arm,
/// "ends the response and returns to the keyword prompt". It is the guard
/// that fronts the shipped introduce-yourself idiom, usually in the NPC's
/// Name or Greeting entry.
pub const TLK_IF_ELSE_END_RESPONSE_ARGUMENT: u8 = 0xFF;
pub const TLK_CODE_IF_ELSE_ALT: u8 = 0xFE;

// §7.7 labels, GOTO, and scoped prompts (and §7 dispatcher boundaries)
pub const TLK_CODE_LABEL_RECORD: u8 = 0x90;
/// `conversation.md §7.7` label / GOTO-LABEL byte band, published as
/// `0x91..=0x9F`. There is exactly one such band: the bytes the
/// dispatcher routes into label dispatch, the bytes a `0x90 <label>`
/// declaration can name, and the bytes an IF/ELSE argument can target
/// are all the same fifteen values. `TLK_LABEL_FIRST` /
/// [`TLK_LABEL_LAST`] are the only names for it.
///
/// Both boundaries are **literals read off the published text**, and
/// neither derives from the other or from a neighbouring band. §7
/// states the rule directly: "Do not derive either band's boundary
/// from the other. They are adjacent as a fact about the original,
/// not as a rule either enforces on the other." An earlier revision
/// of this file defined the GOTO band as
/// `TLK_CONTROL_CODE_LAST + 1 ..= that + 1`, which turned a wrong
/// `TLK_CONTROL_CODE_LAST` into a wrong two-value GOTO band into a
/// wrong `TLK_LABEL_LAST`, with every test in the chain still green.
///
/// §7.7 confirms the range from shipped content: scanning the four
/// shipped `.TLK` files for `0x90 <label>` declarations finds eleven
/// distinct labels spanning the range end to end, steeply skewed to
/// the low end. Fifteen values are addressable; the four unexercised
/// ones are not evidence of a narrower band.
pub const TLK_LABEL_FIRST: u8 = 0x91;
/// `conversation.md §7.7`: last label / GOTO-LABEL byte. Literal from
/// the published `0x91..=0x9F`; see [`TLK_LABEL_FIRST`]. In shipped
/// content `0x9F` is conventionally the blob's final record marker,
/// which is why it is the second-most common label byte.
pub const TLK_LABEL_LAST: u8 = 0x9F;
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

/// `conversation.md §7.6`: exact fixed output for an unaffordable
/// `0x85` demand. Quotes and both trailing line feeds are visible.
pub const TLK_GOLD_PAYMENT_REFUSAL_MESSAGE: &str = "\"Thou hast not enough gold!\"\n\n";

/// `conversation.md §7.6`: the `0x84` RECRUIT-SPEAKER refusal printed
/// when the active party is already at the six-member cap. The code
/// recruits nobody in that case.
pub const TLK_RECRUIT_SPEAKER_FULL_PARTY_REFUSAL: &str = "\"There is no room for me in thy party.\nSeek me again if one of thy members doth leave thee.\"\n\n";

/// `conversation.md §7.6`: number of leading characters of a party
/// member's name that `0x88` ASK-WHO searches the typed line for.
pub const TLK_ASK_WHO_NAME_PREFIX_LEN: usize = 4;

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
    /// `0x91..=0x9F` — GOTO-LABEL codes. Their high bit is set but
    /// they participate in label dispatch, not the ordinary control
    /// table. All fifteen values, not just the top of the range:
    /// eleven are exercised by shipped content (§7.7).
    GotoLabel,
    /// `0xA0..=0xFD` — high-bit-set printable bytes. The word buffer
    /// strips the high bit before glyph output; the `0x8E` print-mask
    /// toggle controls whether the queued byte keeps that high bit as
    /// a soft-break marker.
    PrintableText,
    /// `0x81..=0x90` — engine control codes (the §7.2..§7.6 table).
    /// `0x80` below is a dictionary token and `0x91..=0x9F` above is
    /// the label band; the control codes are what lies between them.
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
///
/// This is a **literal, deliberately not** anchored to
/// [`TLK_LABEL_LAST`] `+ 1`. Printable text starting one past the
/// label band is an observation about two independently published
/// ranges, not a rule either imposes on the other; deriving it would
/// silently widen the printable band the moment the label band was
/// read short. A test asserts the adjacency as a consequence.
pub const TLK_PRINTABLE_TEXT_FIRST: u8 = 0xA0;
pub const TLK_PRINTABLE_TEXT_LAST: u8 = 0xFD;

/// `conversation.md §7` engine-control byte range, `0x81..=0x90`.
/// §7's dispatcher list publishes the control band as "`0x81..0x9F`
/// (with the exception of the GOTO range above)", and the GOTO range
/// is [`TLK_LABEL_FIRST`]`..=`[`TLK_LABEL_LAST`] — `0x91..=0x9F` — so
/// the control codes proper end at `0x90`. That is also where the
/// §7.2–§7.6 dispatch table ends: [`TLK_CODE_LABEL_RECORD`] is
/// `0x90` and is the last row in it. The two readings agree, which is
/// the coherence check the old value failed.
///
/// Both boundaries are **literals, deliberately not** anchored to a
/// neighbouring band. Anchoring the start to
/// [`TLK_DICTIONARY_TOKEN_LAST`] `+ 1` is what let the dictionary
/// band's off-by-one at `0x80` steal a token; anchoring the label
/// band to this constant `+ 1` is what shrank the label band to two
/// values. Both adjacencies are facts about independently published
/// numbers, not rules either imposes on the other, and tests assert
/// them as consequences rather than definitions.
pub const TLK_CONTROL_CODE_FIRST: u8 = 0x81;
pub const TLK_CONTROL_CODE_LAST: u8 = 0x90;

/// `conversation.md §7`: classify a byte by the value-range table the
/// byte runner's top-level dispatcher follows.
///
/// §7 writes the control band as "`0x81..0x9F` with the exception of
/// the GOTO range", so its listing order carves the label band out
/// first. The arms here hold their *resolved* bands — `0x81..=0x90`
/// and `0x91..=0x9F` — which are disjoint, so arm order is no longer
/// load-bearing and the compiler's exhaustiveness check covers the
/// byte space with no overlap left to resolve. The arms keep §7's
/// order for readability only. While the label band was mis-read as
/// `0x9E..=0x9F` the order *was* load-bearing: it was the only thing
/// stopping the overlapping control arm from swallowing the labels.
pub const fn tlk_byte_runner_class(byte: u8) -> TlkByteRunnerClass {
    match byte {
        0x00 => TlkByteRunnerClass::NullByte,
        TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST => {
            TlkByteRunnerClass::DictionaryToken
        }
        TLK_LABEL_FIRST..=TLK_LABEL_LAST => TlkByteRunnerClass::GotoLabel,
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

/// `quest-flags.md §5` / `shops.md §6.2`: the shared town/conversation
/// sentinel uses a no-slot marker distinct from tracked slot indices.
pub const CONVERSATION_SHARED_NO_SLOT_SENTINEL: u8 = 0xFF;

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
    /// `0x81..=0x90`: engine control byte (Sections 7.2-7.6). §7
    /// writes this band as `0x81..0x9F` minus the GOTO range; with the
    /// GOTO range resolved to `0x91..=0x9F` what is left is
    /// `0x81..=0x90`, ending exactly on the `0x90` LABEL-RECORD row
    /// that closes the §7.2-§7.6 dispatch table.
    ControlByte,
    /// `0x91..=0x9F`: GOTO-LABEL byte (Section 7.7) — the full
    /// fifteen-value band, of which eleven are exercised in shipped
    /// content.
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
/// `conversation.md §7`.
///
/// The GOTO-LABEL arm is a **range** over the whole published label
/// band, not an or-pattern over two values. As an or-pattern it named
/// the band's two boundary constants, and so silently stopped
/// covering the band the moment the band was wider than two values —
/// which it always was. With `0x81..=0x90` and `0x91..=0x9F`
/// disjoint, arm order no longer decides anything; it follows §7's
/// listing order.
pub const fn classify_tlk_byte(byte: u8) -> TlkByteKind {
    match byte {
        0x00 => TlkByteKind::Nul,
        TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST => TlkByteKind::DictionaryToken,
        TLK_LABEL_FIRST..=TLK_LABEL_LAST => TlkByteKind::GotoLabel,
        TLK_CONTROL_CODE_FIRST..=TLK_CONTROL_CODE_LAST => TlkByteKind::ControlByte,
        TLK_PRINTABLE_TEXT_FIRST..=TLK_PRINTABLE_TEXT_LAST => TlkByteKind::PrintableText,
        TLK_CODE_IF_ELSE_ALT => TlkByteKind::IfElseAlias,
        TLK_CODE_END_OF_RESPONSE => TlkByteKind::EndOfResponse,
    }
}
