//! `.TLK` byte-runner state machine per `systems/conversation.md` §7.
//!
//! The runner walks one response-stream's raw bytes, classifies each via
//! [`tlk_control_codes`], emits rendered text into a buffer, applies action
//! grants, and records prompts encountered along the
//! way. The deterministic-with-inputs pattern matches the rest of the
//! runtime: any byte-level decision that would normally require an input
//! prompt is pre-decided by the caller through [`TlkRunInputs`], so the
//! runner stays pure and testable.
//!
//! This is not the full interactive keyword-loop — that wraps the runner
//! and feeds keyword-response streams through it. The runner itself is the
//! per-stream engine: feed bytes in, get rendered text and side effects
//! out.

use crate::karma::{KarmaAction, apply_karma_action};
use crate::map_io::{TALK_BRANCH_FLAG_BANK_BITS, talk_branch_flag_is_set, talk_branch_flag_mask};
use crate::tlk_control_codes::*;

/// Inputs the runner needs to interpret control codes that would otherwise
/// require interactive prompts or external state lookups.
#[derive(Clone, Debug, Default)]
pub struct TlkRunInputs<'a> {
    /// Avatar's display name (substituted on `0x81`).
    pub avatar_name: &'a str,
    /// Branch-flag bitmap for the active scene; consulted by `0x8C`.
    pub branch_flags: u32,
    /// Moral-standing byte consulted by `0xFE` IF-ELSE-ALT.
    pub moral_standing: u8,
    /// Common-word dictionary (128 slots; index by token byte `0x01..=0x80`).
    /// `None` is acceptable — token bytes then expand to `"[w<n>]"`.
    pub dictionary: Option<&'a [&'a str; COMMON_WORD_DICTIONARY_ENTRIES]>,
    /// `0x8B` curse-check result. `true` means the player typed something
    /// the engine treats as a curse during the current conversation; the
    /// runner uses it to gate the immediately following control byte the
    /// same way the original does (per `conversation.md` §7.5).
    pub curse_seen: bool,
    /// Optional party gold available to `0x85` GOLD-PAYMENT. The authored
    /// surrounding record already represents the player's yes answer; the
    /// control byte reads no additional confirmation. When supplied, a
    /// demand above this value takes the refusal stop. `None` is useful for
    /// structural tools and treats the demand as affordable.
    pub gold_available: Option<u16>,
    /// `conversation.md §7.6` / §10: roster slot of the NPC currently
    /// speaking. This is the branch-flag bit index that `0x8C` IF-ELSE
    /// tests and that `0x88` ASK-WHO sets — "The bit index is always
    /// supplied by the engine ... so a script can neither choose nor
    /// forge it." `None` means the caller could not name a slot; §10 asks
    /// that such an index build a zero mask, so those tests read as clear
    /// and those setters are no-ops.
    pub npc_slot: Option<u8>,
    /// `0x88` ASK-WHO response: 1-based party-slot index, or `0` for
    /// cancel/no match. Recorded for the caller.
    pub ask_who_response: u8,
    /// `0x83` PAUSE / `0x8F` WAIT-KEY behaviour. When `true`, the runner
    /// stops at each pause/wait-key with [`TlkRunStop::PausedAt`] /
    /// [`TlkRunStop::WaitingKey`] so the caller can flush a page and
    /// resume. When `false`, the runner treats pauses as no-ops and
    /// wait-key as a single newline and keeps going.
    pub yield_on_pause: bool,
    /// `0x88` ASK-WHO behaviour. When `true`, the runner stops
    /// immediately after the ask code so the interactive conversation
    /// wrapper can collect a free-text answer and resume the same stream
    /// with the matched party slot. `0x84` RECRUIT-SPEAKER is *not*
    /// governed by this flag: §7.6 gives it no prompt and no input read.
    pub yield_on_ask: bool,
}

/// Reason the runner stopped processing the current stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkRunStop {
    /// Hit `0xFF` end-of-response (normal end of a keyword response).
    EndOfResponse,
    /// Hit `0x82` end-of-stream for the current fixed entry.
    EndOfStream,
    /// Hit a NUL byte before any explicit terminator. Treated as the
    /// current field terminator.
    NulTerminator,
    /// Exhausted the input slice without finding an explicit terminator.
    Exhausted,
    /// Stopped at `0x83` PAUSE (only when `yield_on_pause` is set).
    PausedAt(usize),
    /// Stopped at `0x8F` WAIT-KEY (only when `yield_on_pause` is set).
    WaitingKey(usize),
    /// Stopped at `0x88` ASK-WHO (only when `yield_on_ask` is set).
    AskingWho(usize),
    /// `conversation.md §7.6`: an unaffordable `0x85` demand stops the
    /// current response before the byte after its third digit. The
    /// runner emits the exact refusal line; the conversation wrapper owns
    /// the nested ordinary keyword loop and stop propagation.
    GoldPaymentRefused { amount: u16 },
    /// Encountered a malformed multi-byte introducer (short arg span).
    MalformedIntroducer(usize),
    /// Encountered an unresolved GOTO-LABEL target (label byte not found
    /// in the stream beyond the current cursor). The runner stops to
    /// avoid an infinite loop.
    UnresolvedGotoLabel(u8),
    /// Hit `0x87` KEYWORD-ALIAS. `cursor` is the saved resume position,
    /// just past the control byte.
    ///
    /// `conversation.md §7.6`: the code is **positional**. The wrapper
    /// skips the remainder of this record, any run of terminators, and
    /// the whole record that follows, then runs the record after that as
    /// a nested stream; if the nested stream signals stop the outer
    /// stream stops too, otherwise it resumes from `cursor`. The runner
    /// itself cannot see record boundaries, so it surfaces the code and
    /// lets the session — which holds the split records — do the skip.
    ///
    /// *Corrected:* this was `FollowUpKeywordScan`, and the session
    /// re-scanned the player's typed remainder against later keyword
    /// entries. §7.6 withdraws both that and the SET-FLAG mnemonic:
    /// `0x87` "does no string comparison, reads no player input,
    /// consumes no argument byte, and writes no flag."
    KeywordAlias(usize),
    /// Encountered a `0x91..=0x9F` label byte through ordinary stream
    /// execution. The conversation session owns the labelled-record
    /// handler and any scoped prompt that follows.
    LabelTransfer(u8),
}

/// One side-effect emitted while running a stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TlkRunEvent {
    /// `0x86` ACTION-DISPATCH letter verb.
    Action(TlkActionDispatchVerb),
    /// `0x86` argument below `'A'` (per-conversation signal flag bit).
    SignalFlag(u8),
    /// `0x85` GOLD-PAYMENT (amount, accepted).
    GoldPayment { amount: u16, accepted: bool },
    /// `0x84` RECRUIT-SPEAKER was reached. Carries no payload: §7.6
    /// gives the code no arguments, no prompt and no input read. The
    /// caller performs the reserve-roster recruitment.
    RecruitSpeaker,
    /// `0x88` ASK-WHO: 1-based slot match (0 = cancel/no match).
    AskedWho(u8),
    /// `0x8B` CURSE-CHECK was reached.
    CurseChecked { curse_seen: bool },
    /// `0x8F` WAIT-KEY synthesised newline (when `yield_on_pause` false).
    WaitKeyTreatedAsNewline,
    /// `0x83` PAUSE encountered (when `yield_on_pause` false).
    PauseSkipped,
    /// `0x8C` IF-ELSE branch decision. `bit` is the engine-chosen branch
    /// flag index (the speaking NPC's roster slot), `target_label` is the
    /// code's argument byte, and `taken_else` reflects whether the bit was
    /// set — the arm that transfers to `target_label` (or, for the
    /// reserved `0xFF`, ends the response).
    IfElseBranchTaken {
        bit: u8,
        target_label: u8,
        taken_else: bool,
    },
    /// `0xFE` IF-ELSE-ALT branch decision.
    IfElseAltDecision {
        threshold: u8,
        target_label: u8,
        branched: bool,
    },
    /// A GOTO-LABEL (`0x91..=0x9F`) was followed.
    GotoLabel { from: u8, to: u8 },
    /// A byte reached the dispatcher's unclassified arm. `conversation.md`
    /// §7 partitions the whole byte space, so this event means the runner
    /// is missing a case or the stream is not shipped content. Recorded
    /// rather than skipped so a missing case cannot stay invisible; debug
    /// builds also assert on it.
    UnclassifiedByte { byte: u8, offset: usize },
    /// `0x89` STANDING-UP / `0x8A` STANDING-DOWN wrote the shared
    /// moral-standing selector. `raise` distinguishes the two codes and
    /// `to` is the post-clamp value. One event is recorded per byte even
    /// when the clamp swallowed the step, because scripts stack the
    /// bytes and the count of writers that ran is what a caller checking
    /// `conversation.md §7.4`'s reaction records wants to see.
    MoralStandingWrite { raise: bool, from: u8, to: u8 },
}

/// Fixed-cell font selected for one glyph emitted by the TLK byte runner.
///
/// `conversation.md §7.1`: ordinary queued bytes retain bit seven and render
/// through the resident text font; bytes queued while the `0x8E` mask is
/// flipped have bit seven clear and render through the alternate runic font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TlkGlyphFont {
    #[default]
    Ordinary,
    Runic,
}

/// One display cell emitted by a TLK response, after dictionary expansion and
/// printable-byte decoding but before fixed-font rasterisation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TlkRenderedGlyph {
    pub byte: u8,
    pub font: TlkGlyphFont,
}

impl TlkRenderedGlyph {
    pub const fn ordinary(byte: u8) -> Self {
        Self {
            byte,
            font: TlkGlyphFont::Ordinary,
        }
    }

    pub const fn runic(byte: u8) -> Self {
        Self {
            byte,
            font: TlkGlyphFont::Runic,
        }
    }
}

/// Project engine-authored Rust text onto display cells in `font`.
///
/// `formats/font-ch.md §4`: "The format has no storage for high-bit
/// character codes. The resident text emitter owns that caller-side
/// policy: ordinary cell output ignores high-bit bytes unless an adjacent
/// extended-control path has already consumed them." `text-output.md §5`
/// says the same from the emitter's side — only "a byte with the high bit
/// clear" renders a glyph, and "other high-bit bytes outside the confirmed
/// control range have no public glyph meaning".
///
/// A Rust `&str` is UTF-8, so a character the font cannot address is one
/// uncell-able character, never the two-or-more bytes `str::bytes` would
/// yield. Iterating bytes here would hand the fixed-cell renderer a UTF-8
/// lead byte — `0xC3` for anything in `U+00C0..=U+00FF` — which no `.CH`
/// glyph slot covers and which the renderer rightly refuses to draw. So
/// walk characters and drop the ones outside the font's seven-bit range:
/// an ignored character occupies no cell, which keeps wrapping and column
/// arithmetic aligned with what is actually painted.
pub fn glyphs_from_engine_text(text: &str, font: TlkGlyphFont) -> Vec<TlkRenderedGlyph> {
    text.chars()
        .filter(char::is_ascii)
        .map(|ch| TlkRenderedGlyph {
            byte: ch as u8,
            font,
        })
        .collect()
}

/// [`glyphs_from_engine_text`] in the resident text font.
pub fn ordinary_glyphs_from_engine_text(text: &str) -> Vec<TlkRenderedGlyph> {
    glyphs_from_engine_text(text, TlkGlyphFont::Ordinary)
}

/// Text plus its per-cell font selection. `text` remains the terminal and
/// diagnostics view; `glyphs` is the authoritative graphical presentation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TlkRenderedText {
    pub text: String,
    pub glyphs: Vec<TlkRenderedGlyph>,
}

impl TlkRenderedText {
    pub fn plain(text: impl Into<String>) -> Self {
        let text = text.into();
        let glyphs = ordinary_glyphs_from_engine_text(&text);
        Self { text, glyphs }
    }

    pub fn push_plain(&mut self, text: &str) {
        self.text.push_str(text);
        self.glyphs.extend(ordinary_glyphs_from_engine_text(text));
    }

    pub fn push_rendered(&mut self, rendered: &Self) {
        self.text.push_str(&rendered.text);
        self.glyphs.extend_from_slice(&rendered.glyphs);
    }

    pub fn trimmed(&self) -> Self {
        let first = self
            .glyphs
            .iter()
            .position(|glyph| !glyph.byte.is_ascii_whitespace())
            .unwrap_or(self.glyphs.len());
        let last = self
            .glyphs
            .iter()
            .rposition(|glyph| !glyph.byte.is_ascii_whitespace())
            .map(|index| index + 1)
            .unwrap_or(first);
        let glyphs = self.glyphs[first..last].to_vec();
        let text = glyphs.iter().map(|glyph| char::from(glyph.byte)).collect();
        Self { text, glyphs }
    }

    pub fn rendered_lines(&self) -> impl Iterator<Item = &[TlkRenderedGlyph]> {
        self.glyphs.split(|glyph| glyph.byte == b'\n')
    }
}

impl Default for TlkRunStop {
    fn default() -> Self {
        TlkRunStop::Exhausted
    }
}

/// Result of running one TLK stream end-to-end (or until a yield point).
#[derive(Clone, Debug, Default)]
pub struct TlkRunOutput {
    /// Rendered visible text. The runner strips bit-7 from printable
    /// bytes (`0xA0..=0xFD`) and expands dictionary tokens through the
    /// supplied dictionary (or `[w<n>]` placeholders).
    pub text: String,
    /// Per-cell font-preserving form of [`Self::text`]. This is what a
    /// graphical message-window renderer consumes so matched `0x8E` spans and
    /// empty dictionary entries reach `RUNES.CH` rather than the IBM font.
    pub rendered_glyphs: Vec<TlkRenderedGlyph>,
    /// Mask of branch-flag bits the stream set.
    ///
    /// `conversation.md §7.6`: `0x88` ASK-WHO is the bank's in-stream
    /// setter — "an implementation ... must not conclude that the bank
    /// has no setter." A successful ASK-WHO raises the speaking NPC's own
    /// roster-slot bit here; the caller merges it into the active scene's
    /// durable branch-flag word.
    pub branch_flags_set: u32,
    /// Action-dispatch verbs encountered, in order.
    pub action_grants: Vec<TlkActionDispatchVerb>,
    /// Per-conversation signal-flag bits seen (post-mask argument < `'A'`).
    pub signal_flags: Vec<u8>,
    /// Side-effect events, in dispatch order. Mainly useful for tests
    /// and for the keyword-loop wrapper that needs to detect when to
    /// prompt for input or end the conversation.
    pub events: Vec<TlkRunEvent>,
    /// Shared moral-standing selector after the stream's `0x89` / `0x8A`
    /// writes, or `None` when the stream ran neither code.
    /// `conversation.md §7.4` makes those two the byte runner's only
    /// direct writers of the selector, so a caller applies this by
    /// assignment rather than by re-deriving a delta.
    pub moral_standing: Option<u8>,
    /// Why the runner stopped.
    pub stop: TlkRunStop,
    /// Byte index just past the last byte consumed.
    pub consumed: usize,
}

impl TlkRunOutput {
    pub fn rendered_text(&self) -> TlkRenderedText {
        TlkRenderedText {
            text: self.text.clone(),
            glyphs: self.rendered_glyphs.clone(),
        }
    }
}

fn emit_rendered_glyph(out: &mut TlkRunOutput, glyph: TlkRenderedGlyph) {
    out.text.push(char::from(glyph.byte));
    out.rendered_glyphs.push(glyph);
}

fn emit_rendered_text(out: &mut TlkRunOutput, text: &str, font: TlkGlyphFont) {
    for glyph in glyphs_from_engine_text(text, font) {
        emit_rendered_glyph(out, glyph);
    }
}

/// `conversation.md §7.6`: the refusal arm clears the resident pending-word
/// count before printing its fixed line. The clean runner emits cells eagerly,
/// so reproduce that observable result by dropping the unflushed trailing word
/// from both aligned output views.
fn discard_pending_word(out: &mut TlkRunOutput) {
    let keep = out
        .rendered_glyphs
        .iter()
        .rposition(|glyph| glyph.byte.is_ascii_whitespace())
        .map_or(0, |index| index + 1);
    out.rendered_glyphs.truncate(keep);
    out.text = out
        .rendered_glyphs
        .iter()
        .map(|glyph| char::from(glyph.byte))
        .collect();
}

const fn glyph_font_for_print_mask(mask: TlkPrintMaskState) -> TlkGlyphFont {
    match mask {
        TlkPrintMaskState::NormalBreaks => TlkGlyphFont::Ordinary,
        TlkPrintMaskState::ProtectedRun => TlkGlyphFont::Runic,
    }
}

/// Execute the byte runner over `bytes` until an explicit terminator,
/// yield point, or malformed control sequence.
pub fn run_tlk_stream(bytes: &[u8], inputs: &TlkRunInputs) -> TlkRunOutput {
    run_tlk_stream_from(bytes, 0, inputs)
}

/// Execute the byte runner starting at an already-consumed cursor. Label
/// transfers still scan the whole stream, matching the normal runner.
pub fn run_tlk_stream_from(bytes: &[u8], start: usize, inputs: &TlkRunInputs) -> TlkRunOutput {
    let mut out = TlkRunOutput {
        stop: TlkRunStop::Exhausted,
        ..Default::default()
    };
    let mut pos = start.min(bytes.len());
    let mut print_mask = TlkPrintMaskState::NormalBreaks;
    // Track the last *emitted* printable byte (pre-mask, post-XOR) so we
    // can collapse the on-disk `""` double-quote artefact per §7.5.
    let mut last_emitted: Option<u8> = None;
    let mut leading_space_pending = false;
    // Curse check is reset when the runner starts and may flip later.
    let mut curse_pending = inputs.curse_seen;
    // §7.4: `0x89` and `0x8A` write the shared moral-standing selector
    // in-stream, so a later `0xFE` threshold test in the same stream must
    // read the updated value rather than the caller's entry snapshot.
    let mut moral_standing = inputs.moral_standing;
    // A response can contain more than one payment control. Debit the local
    // affordability view as each one succeeds so later controls cannot spend
    // the same gold twice before the caller applies the emitted events.
    let mut gold_available = inputs.gold_available;

    while pos < bytes.len() {
        let byte = bytes[pos];
        pos += 1;

        match byte {
            0x00 => {
                out.stop = TlkRunStop::NulTerminator;
                out.consumed = pos;
                return out;
            }
            TLK_CODE_END_OF_RESPONSE => {
                out.stop = TlkRunStop::EndOfResponse;
                out.consumed = pos;
                return out;
            }
            TLK_CODE_END_STREAM => {
                out.stop = TlkRunStop::EndOfStream;
                out.consumed = pos;
                return out;
            }
            TLK_CODE_PRINT_AVATAR_NAME => {
                // `conversation.md §8.1`: only the printable-text path
                // consumes the dictionary pending-space flag. Substitutions
                // and every other control leave it armed.
                emit_rendered_text(&mut out, inputs.avatar_name, TlkGlyphFont::Ordinary);
                last_emitted = inputs.avatar_name.bytes().last();
            }
            TLK_CODE_STANDING_UP | TLK_CODE_STANDING_DOWN => {
                // §7.4: both codes emit no text and do not touch the word
                // buffer. `0x8A` is *not* a newline here — the literal
                // newline is `0x8D` and only `0x8D`; `0x8A` only means
                // newline inside the word buffer, downstream of this
                // dispatch, where the printable-text path rewrites `0x8D`
                // to `0x8A`.
                let raise = byte == TLK_CODE_STANDING_UP;
                let action = if raise {
                    KarmaAction::ConversationStandingUp
                } else {
                    KarmaAction::ConversationStandingDown
                };
                let from = moral_standing;
                moral_standing = apply_karma_action(from, action);
                out.moral_standing = Some(moral_standing);
                out.events.push(TlkRunEvent::MoralStandingWrite {
                    raise,
                    from,
                    to: moral_standing,
                });
            }
            TLK_CODE_LITERAL_NEWLINE => {
                emit_rendered_glyph(&mut out, TlkRenderedGlyph::ordinary(b'\n'));
                last_emitted = Some(b'\n');
            }
            TLK_CODE_PAUSE => {
                if inputs.yield_on_pause {
                    out.stop = TlkRunStop::PausedAt(pos);
                    out.consumed = pos;
                    return out;
                }
                out.events.push(TlkRunEvent::PauseSkipped);
            }
            TLK_CODE_WAIT_KEY => {
                if inputs.yield_on_pause {
                    out.stop = TlkRunStop::WaitingKey(pos);
                    out.consumed = pos;
                    return out;
                }
                emit_rendered_glyph(&mut out, TlkRenderedGlyph::ordinary(b'\n'));
                out.events.push(TlkRunEvent::WaitKeyTreatedAsNewline);
                last_emitted = Some(b'\n');
            }
            TLK_CODE_PROTECT_RUN => {
                print_mask = print_mask.toggle();
            }
            TLK_CODE_CURSE_CHECK => {
                out.events.push(TlkRunEvent::CurseChecked {
                    curse_seen: curse_pending,
                });
                // The original engine resets the curse flag after the check
                // so a subsequent CURSE-CHECK only fires if a new curse word
                // was typed in the meantime.
                curse_pending = false;
            }
            TLK_CODE_KEYWORD_ALIAS => {
                out.stop = TlkRunStop::KeywordAlias(pos);
                out.consumed = pos;
                return out;
            }
            TLK_CODE_RECRUIT_SPEAKER => {
                // §7.6: "There is no player prompt and no input read."
                // The code consumes no argument bytes and does not stop
                // the stream; the caller matches the speaker's own name
                // against the reserve roster when it sees the event.
                out.events.push(TlkRunEvent::RecruitSpeaker);
            }
            TLK_CODE_ASK_WHO => {
                if inputs.yield_on_ask {
                    out.stop = TlkRunStop::AskingWho(pos);
                    out.consumed = pos;
                    return out;
                }
                let slot = inputs.ask_who_response;
                if slot != 0 {
                    // §7.6: ASK-WHO is the in-stream setter for the bank
                    // `0x8C` tests, and the bit is the speaking NPC's own
                    // roster slot. An absent slot builds a zero mask (§10).
                    out.branch_flags_set |= inputs.npc_slot.map_or(0, talk_branch_flag_mask);
                }
                out.events.push(TlkRunEvent::AskedWho(slot));
            }
            TLK_CODE_GOLD_PAYMENT => {
                let span = bytes.get(pos..pos + 3);
                let Some(span) = span else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 3;
                if let Some(amount) = tlk_gold_payment_amount(span[0], span[1], span[2]) {
                    let accepted = gold_available.is_none_or(|available| available >= amount);
                    out.events
                        .push(TlkRunEvent::GoldPayment { amount, accepted });
                    if !accepted {
                        discard_pending_word(&mut out);
                        emit_rendered_text(
                            &mut out,
                            TLK_GOLD_PAYMENT_REFUSAL_MESSAGE,
                            TlkGlyphFont::Ordinary,
                        );
                        out.stop = TlkRunStop::GoldPaymentRefused { amount };
                        out.consumed = pos;
                        return out;
                    }
                    if let Some(available) = gold_available.as_mut() {
                        *available -= amount;
                    }
                }
            }
            TLK_CODE_ACTION_DISPATCH => {
                let Some(&raw) = bytes.get(pos) else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 1;
                let arg = raw & 0x7F;
                if tlk_action_dispatch_is_signal_flag(arg) {
                    out.signal_flags.push(arg);
                    out.events.push(TlkRunEvent::SignalFlag(arg));
                } else if let Some(verb) = tlk_action_dispatch_verb(arg) {
                    out.action_grants.push(verb);
                    out.events.push(TlkRunEvent::Action(verb));
                }
            }
            TLK_CODE_IF_ELSE => {
                let arg_start = pos;
                let Some(&target_label) = bytes.get(pos) else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 1;
                let arg_end = pos;
                // §7.6: the argument byte is the **branch target label**,
                // and "The tested bit is chosen by the engine, never by
                // the script" — it is the roster slot of the NPC currently
                // speaking. §7.6 closes: "a clean implementation must not
                // model `0x8C`'s argument as a flag id".
                //
                // *Corrected:* this arm used to mask the argument to seven
                // bits, test that as a flag index, and on a set bit scan
                // forward to the next label byte. Both halves are
                // withdrawn, and the fold also swallowed the reserved
                // `0xFF` argument, which became bit index `0x7F` and so
                // always read as clear.
                let bit = inputs.npc_slot.unwrap_or(TALK_BRANCH_FLAG_BANK_BITS);
                let flag_set = talk_branch_flag_is_set(inputs.branch_flags, bit);
                out.events.push(TlkRunEvent::IfElseBranchTaken {
                    bit,
                    target_label,
                    taken_else: flag_set,
                });
                // Bit clear: fall through in-stream with the byte after
                // the argument, which `pos` already names.
                if flag_set {
                    if target_label == TLK_IF_ELSE_END_RESPONSE_ARGUMENT {
                        // The reserved argument ends the response and
                        // returns to the keyword prompt. This is the guard
                        // that fronts the shipped introduce-yourself idiom.
                        out.stop = TlkRunStop::EndOfResponse;
                        out.consumed = pos;
                        return out;
                    }
                    let Some(target_pos) =
                        find_label_position_excluding(bytes, target_label, arg_start, arg_end)
                    else {
                        out.stop = TlkRunStop::UnresolvedGotoLabel(target_label);
                        out.consumed = pos;
                        return out;
                    };
                    out.events.push(TlkRunEvent::GotoLabel {
                        from: TLK_CODE_IF_ELSE,
                        to: target_label,
                    });
                    pos = target_pos;
                }
            }
            TLK_CODE_IF_ELSE_ALT => {
                let arg_start = pos;
                let span = bytes.get(pos..pos + 2);
                let Some(span) = span else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 2;
                let arg_end = pos;
                let threshold = span[0];
                let target_label = span[1];
                let branched = tlk_if_else_alt_branches(moral_standing, threshold);
                out.events.push(TlkRunEvent::IfElseAltDecision {
                    threshold,
                    target_label,
                    branched,
                });
                if branched {
                    if let Some(target_pos) =
                        find_label_position_excluding(bytes, target_label, arg_start, arg_end)
                    {
                        out.events.push(TlkRunEvent::GotoLabel {
                            from: TLK_CODE_IF_ELSE_ALT,
                            to: target_label,
                        });
                        pos = target_pos;
                    } else {
                        out.stop = TlkRunStop::UnresolvedGotoLabel(target_label);
                        out.consumed = pos;
                        return out;
                    }
                }
            }
            _ => {
                if is_tlk_label_byte(byte) {
                    out.stop = TlkRunStop::LabelTransfer(byte);
                    out.consumed = pos;
                    return out;
                }
                if (TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST).contains(&byte) {
                    let idx = tlk_dictionary_index(byte)
                        .expect("byte is inside the TLK dictionary-token range");
                    // `conversation.md §8.1`: every token emits one leading
                    // space, even when a previous token left the
                    // pending-space flag armed.
                    emit_rendered_glyph(&mut out, TlkRenderedGlyph::ordinary(b' '));
                    if let Some(dict) = inputs.dictionary {
                        let expansion = dict.get(idx).copied().unwrap_or("");
                        if expansion.is_empty() {
                            // §8.2: an empty pointer queues the raw token byte
                            // with bit seven clear, selecting the alternate
                            // font, and does not arm pending spacing.
                            emit_rendered_glyph(&mut out, TlkRenderedGlyph::runic(byte));
                            last_emitted = Some(byte);
                        } else {
                            emit_rendered_text(
                                &mut out,
                                expansion,
                                glyph_font_for_print_mask(print_mask),
                            );
                            last_emitted = expansion.bytes().last();
                            leading_space_pending = true;
                        }
                    } else {
                        // Fallback placeholder keeps the runner
                        // deterministic even without dictionary bytes.
                        emit_rendered_text(
                            &mut out,
                            &format!("[w{byte:02X}]"),
                            glyph_font_for_print_mask(print_mask),
                        );
                        last_emitted = Some(b']');
                        leading_space_pending = true;
                    }
                } else if byte == TLK_CODE_LABEL_RECORD
                    || (TLK_PRINTABLE_TEXT_FIRST..=TLK_PRINTABLE_TEXT_LAST).contains(&byte)
                {
                    // `conversation.md §7.7`, corrected by public issue
                    // #164 / retraction R278: `0x90` is structural only to
                    // the label scanner. The ordinary runner has no control
                    // case, so it follows this printable fall-through and
                    // emits glyph `0x10`; the following `0x91..=0x9F` byte is
                    // dispatched independently as an active GOTO. There is
                    // no label-boundary flush or pending-space reset.
                    let glyph = byte ^ TLK_TEXT_XOR_MASK;
                    if byte == TLK_DOUBLE_QUOTE_ENCODED && last_emitted == Some(b'"') {
                        // §7.5 double-quote dedup: collapse adjacent ""
                        // into a single visible quote.
                        last_emitted = None;
                        continue;
                    }
                    if leading_space_pending {
                        emit_rendered_glyph(&mut out, TlkRenderedGlyph::ordinary(b' '));
                        leading_space_pending = false;
                    }
                    emit_rendered_glyph(
                        &mut out,
                        TlkRenderedGlyph {
                            byte: glyph,
                            font: glyph_font_for_print_mask(print_mask),
                        },
                    );
                    last_emitted = Some(glyph);
                } else {
                    // No published classification claims this byte.
                    //
                    // `conversation.md` §7 partitions the whole byte space
                    // — NUL, dictionary tokens, the named control codes,
                    // the label band, printable text, `0xFE` and `0xFF` —
                    // so the dispatcher above is exhaustive and nothing can
                    // legitimately arrive here. Reaching this arm therefore
                    // means one of two things, and both are worth surfacing
                    // rather than swallowing: a case this runner failed to
                    // implement, or a `.TLK` stream that is not shipped
                    // content.
                    //
                    // This arm used to skip the byte in silence, which made
                    // the first of those two undetectable: a missing case
                    // rendered as absent text with no event, no stop, and no
                    // failing test. Missing bytes are recorded as an event so
                    // callers and tests can see them, and `debug_assert`
                    // turns it into a hard failure in test and dev builds while
                    // leaving release builds able to survive a malformed
                    // third-party file mid-conversation.
                    debug_assert!(
                        false,
                        "TLK byte {byte:#04X} at offset {} reached the byte runner's                          unclassified arm; conversation.md §7 partitions the byte                          space, so this is a missing dispatcher case",
                        pos - 1
                    );
                    out.events.push(TlkRunEvent::UnclassifiedByte {
                        byte,
                        offset: pos - 1,
                    });
                }
            }
        }
    }

    out.consumed = pos;
    out
}

/// Scan forward from byte 0 for the supplied label byte after a label
/// transfer rewinds to the loaded blob start. The first match's position
/// (one past the label) is returned. Labels are not unique inside a blob;
/// the first match wins, matching the original dispatcher's scan
/// behaviour.
pub fn find_label_position(bytes: &[u8], label: u8) -> Option<usize> {
    find_label_position_from(bytes, label, 0)
}

fn find_label_position_excluding(
    bytes: &[u8],
    label: u8,
    exclude_start: usize,
    exclude_end: usize,
) -> Option<usize> {
    if !is_tlk_label_byte(label) {
        return None;
    }
    let mut pos = 0usize;
    while pos < bytes.len() {
        if pos >= exclude_start && pos < exclude_end {
            pos = exclude_end;
            continue;
        }
        if bytes[pos] == label {
            return Some(pos + 1);
        }
        pos += 1;
    }
    None
}

/// Scan forward from `start` for the supplied label byte. Kept for callers
/// that need a bounded local scan; label-transfer paths should scan from the
/// blob start while excluding the introducer argument span currently being
/// consumed.
pub fn find_label_position_from(bytes: &[u8], label: u8, start: usize) -> Option<usize> {
    if !is_tlk_label_byte(label) {
        return None;
    }
    let mut pos = start;
    while pos < bytes.len() {
        if bytes[pos] == label {
            return Some(pos + 1);
        }
        pos += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(bytes: &[u8]) -> TlkRunOutput {
        run_tlk_stream(bytes, &TlkRunInputs::default())
    }

    fn enc(text: &str) -> Vec<u8> {
        text.bytes().map(|b| b ^ TLK_TEXT_XOR_MASK).collect()
    }

    #[test]
    fn renders_printable_text_and_terminates_on_end_of_response() {
        let mut bytes = enc("Hello!");
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.text, "Hello!");
        assert_eq!(out.stop, TlkRunStop::EndOfResponse);
        assert_eq!(out.consumed, bytes.len());
    }

    #[test]
    fn end_of_stream_halts_runner() {
        let mut bytes = enc("Bye");
        bytes.push(TLK_CODE_END_STREAM);
        bytes.extend_from_slice(&enc("ignored"));
        let out = render(&bytes);
        assert_eq!(out.text, "Bye");
        assert_eq!(out.stop, TlkRunStop::EndOfStream);
    }

    #[test]
    fn null_byte_acts_as_blob_terminator() {
        let mut bytes = enc("Hi");
        bytes.push(0x00);
        let out = render(&bytes);
        assert_eq!(out.text, "Hi");
        assert_eq!(out.stop, TlkRunStop::NulTerminator);
    }

    /// `conversation.md §7.4`: "the literal-newline code is `0x8D` and
    /// only `0x8D`". This test used to push `0x8A` as a second newline
    /// byte and assert it rendered one; §7.4 withdraws that reading, so
    /// the byte is now asserted to emit no text at all. The `0x8A`
    /// newline meaning exists only *inside* the word buffer, where the
    /// printable-text path rewrites `0x8D` to `0x8A` after control-code
    /// dispatch has already been passed — a different stage from this
    /// dispatcher.
    #[test]
    fn literal_newline_is_the_only_newline_control_byte() {
        let mut bytes = enc("a");
        bytes.push(TLK_CODE_LITERAL_NEWLINE);
        bytes.extend_from_slice(&enc("b"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        assert_eq!(render(&bytes).text, "a\nb");

        let mut standing = enc("a");
        standing.push(TLK_CODE_STANDING_DOWN);
        standing.extend_from_slice(&enc("b"));
        standing.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&standing);
        assert_eq!(out.text, "ab", "0x8A emits no text and no newline");
    }

    /// `conversation.md §7.4` / `karma.md §4`: `0x89` and `0x8A` are the
    /// byte runner's only direct writers of the shared moral-standing
    /// selector, moving it by one through the capped-add / capped-subtract
    /// writers and emitting no text.
    #[test]
    fn standing_codes_write_the_shared_moral_standing_selector() {
        let mut bytes = vec![TLK_CODE_STANDING_UP; 5];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                moral_standing: 50,
                ..Default::default()
            },
        );
        assert_eq!(out.moral_standing, Some(55));
        assert!(out.text.is_empty());
        assert_eq!(
            out.events
                .iter()
                .filter(|event| matches!(
                    event,
                    TlkRunEvent::MoralStandingWrite { raise: true, .. }
                ))
                .count(),
            5
        );

        let mut down = vec![TLK_CODE_STANDING_DOWN; 3];
        down.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &down,
            &TlkRunInputs {
                moral_standing: 50,
                ..Default::default()
            },
        );
        assert_eq!(out.moral_standing, Some(47));
        assert!(out.text.is_empty());
    }

    /// §7.4 clamps: the raise is capped at ninety-nine and the lower is
    /// floored at zero, both through the ordinary capped writers.
    #[test]
    fn standing_codes_clamp_at_the_published_bounds() {
        let mut up = vec![TLK_CODE_STANDING_UP; 3];
        up.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &up,
            &TlkRunInputs {
                moral_standing: crate::MORAL_STANDING_MAX - 1,
                ..Default::default()
            },
        );
        assert_eq!(out.moral_standing, Some(crate::MORAL_STANDING_MAX));

        let mut down = vec![TLK_CODE_STANDING_DOWN; 3];
        down.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &down,
            &TlkRunInputs {
                moral_standing: 1,
                ..Default::default()
            },
        );
        assert_eq!(out.moral_standing, Some(0));
    }

    /// A stream that runs neither code leaves the selector alone, so the
    /// caller can tell "wrote the entry value back" from "did not write".
    #[test]
    fn stream_without_standing_codes_reports_no_standing_write() {
        let mut bytes = enc("Hail!");
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        assert_eq!(render(&bytes).moral_standing, None);
    }

    /// §7.4 says `0x89`/`0x8A` write "the shared moral-standing
    /// selector", which is the same value `0xFE` IF-ELSE-ALT tests. A
    /// write earlier in a stream must therefore be visible to a later
    /// threshold test in that same stream.
    #[test]
    fn if_else_alt_threshold_reads_the_in_stream_standing_write() {
        // Entry standing 50; three STANDING-DOWN bytes take it to 47,
        // which is below the threshold 48, so the branch is not taken.
        let mut bytes = vec![TLK_CODE_STANDING_DOWN; 3];
        bytes.push(TLK_CODE_IF_ELSE_ALT);
        bytes.push(48);
        bytes.push(0x91);
        bytes.extend_from_slice(&enc("fall-through"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                moral_standing: 50,
                ..Default::default()
            },
        );
        assert!(out.events.iter().any(|event| matches!(
            event,
            TlkRunEvent::IfElseAltDecision {
                branched: false,
                ..
            }
        )));
    }

    #[test]
    fn avatar_name_substitution() {
        let mut bytes = vec![TLK_CODE_PRINT_AVATAR_NAME];
        bytes.extend_from_slice(&enc(", greetings!"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                avatar_name: "Cal",
                ..Default::default()
            },
        );
        assert_eq!(out.text, "Cal, greetings!");
    }

    #[test]
    fn action_dispatch_records_letter_verb() {
        let mut bytes = enc("Take this.");
        bytes.push(TLK_CODE_ACTION_DISPATCH);
        bytes.push(b'A'); // RaiseFood
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.action_grants, vec![TlkActionDispatchVerb::RaiseFood]);
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, TlkRunEvent::Action(TlkActionDispatchVerb::RaiseFood)))
        );
    }

    #[test]
    fn action_dispatch_signal_flag_branch() {
        let mut bytes = vec![TLK_CODE_ACTION_DISPATCH, 0x05];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert!(out.action_grants.is_empty());
        assert_eq!(out.signal_flags, vec![0x05]);
    }

    #[test]
    fn keyword_alias_yields_the_saved_resume_cursor_and_consumes_no_argument() {
        // §7.6: `0x87` "consumes no argument byte". The runner cannot see
        // record boundaries, so it surfaces the code with the saved
        // position and lets the session do the positional skip.
        let mut bytes = vec![TLK_CODE_KEYWORD_ALIAS];
        bytes.extend_from_slice(&enc("tail"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.stop, TlkRunStop::KeywordAlias(1));
        assert_eq!(out.consumed, 1);
        assert!(out.text.is_empty());

        let resumed = run_tlk_stream_from(&bytes, out.consumed, &TlkRunInputs::default());
        assert_eq!(resumed.text, "tail");
    }

    /// `0x8C <label>`; fall-through arm; label `0x91`; target arm; EOR.
    fn if_else_stream(target_label: u8) -> Vec<u8> {
        let mut bytes = vec![TLK_CODE_IF_ELSE, target_label];
        bytes.extend_from_slice(&enc("then"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        bytes.push(0x91);
        bytes.extend_from_slice(&enc("else"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        bytes
    }

    #[test]
    fn if_else_falls_through_when_the_speaking_npc_bit_is_clear() {
        // §7.6: "If the bit is clear, fall through in-stream with the byte
        // after the argument."
        let bytes = if_else_stream(0x91);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(2),
                branch_flags: 0,
                ..Default::default()
            },
        );
        assert!(out.text.starts_with("then"));
        assert!(!out.text.contains("else"));
    }

    #[test]
    fn if_else_transfers_to_the_record_its_argument_names() {
        // §7.6: the argument byte "is the branch target label", and the
        // tested bit is the speaking NPC's roster slot — never the
        // argument. Here the argument names label 0x91 while the bit that
        // decides is slot 2.
        let bytes = if_else_stream(0x91);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(2),
                branch_flags: 1u32 << 2,
                ..Default::default()
            },
        );
        assert!(!out.text.contains("then"));
        assert!(out.text.contains("else"));
        assert!(out.events.iter().any(|e| matches!(
            e,
            TlkRunEvent::GotoLabel {
                from: TLK_CODE_IF_ELSE,
                to: 0x91
            }
        )));
    }

    #[test]
    fn if_else_tests_the_npc_slot_not_the_argument_byte() {
        // The argument is 0x92 here. Under the withdrawn flag-id reading
        // the runner would have tested bit 0x12; under the published one
        // it tests the speaking NPC's slot, which is clear.
        let mut bytes = vec![TLK_CODE_IF_ELSE, 0x92];
        bytes.extend_from_slice(&enc("then"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        bytes.push(0x92);
        bytes.extend_from_slice(&enc("else"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(3),
                branch_flags: 1u32 << 0x12,
                ..Default::default()
            },
        );
        assert!(out.text.starts_with("then"));
    }

    #[test]
    fn if_else_reserved_ff_argument_ends_the_response() {
        // §7.6: "or, for the reserved argument `0xFF`, end the response
        // and return to the keyword prompt". This is the guard that fronts
        // the shipped introduce-yourself idiom.
        let mut bytes = vec![TLK_CODE_IF_ELSE, TLK_IF_ELSE_END_RESPONSE_ARGUMENT];
        bytes.extend_from_slice(&enc("introduction"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let known = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(4),
                branch_flags: 1u32 << 4,
                ..Default::default()
            },
        );
        assert_eq!(known.stop, TlkRunStop::EndOfResponse);
        assert!(known.text.is_empty());

        // A stranger falls through and hears the introduction.
        let stranger = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(4),
                branch_flags: 0,
                ..Default::default()
            },
        );
        assert_eq!(stranger.text, "introduction");
    }

    #[test]
    fn if_else_alt_branches_to_label_when_standing_meets_threshold() {
        let mut bytes = vec![TLK_CODE_IF_ELSE_ALT, 0x80, 0x92];
        bytes.extend_from_slice(&enc("low"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let label_pos = bytes.len();
        bytes.push(0x92);
        bytes.extend_from_slice(&enc("high"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                moral_standing: 0x90,
                ..Default::default()
            },
        );
        // High standing: branched to label 0x92, rendering "high" only.
        assert!(!out.text.contains("low"));
        assert!(out.text.contains("high"));
        let _ = label_pos;
    }

    #[test]
    fn label_transfer_lookup_uses_blob_start_not_current_cursor() {
        let bytes = [
            0x92,
            b'e' | TLK_TEXT_XOR_MASK,
            b'a' | TLK_TEXT_XOR_MASK,
            b'r' | TLK_TEXT_XOR_MASK,
            b'l' | TLK_TEXT_XOR_MASK,
            b'y' | TLK_TEXT_XOR_MASK,
            TLK_CODE_END_OF_RESPONSE,
            TLK_CODE_IF_ELSE_ALT,
            0x01,
            0x92,
        ];
        let branch_cursor = 10;

        assert_eq!(find_label_position(&bytes, 0x92), Some(1));
        assert_eq!(find_label_position_excluding(&bytes, 0x92, 8, 10), Some(1));
        assert_eq!(find_label_position_from(&bytes, 0x92, branch_cursor), None);
    }

    #[test]
    fn gold_payment_records_amount_and_acceptance() {
        let mut bytes = vec![TLK_CODE_GOLD_PAYMENT, b'0', b'5', b'0'];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                gold_available: Some(50),
                ..Default::default()
            },
        );
        let mut saw = false;
        for event in &out.events {
            if let TlkRunEvent::GoldPayment { amount, accepted } = event {
                assert_eq!(*amount, 50);
                assert!(*accepted);
                saw = true;
            }
        }
        assert!(saw, "expected GoldPayment event");
    }

    #[test]
    fn gold_payment_continues_in_place_or_stops_for_refusal() {
        let mut bytes = vec![TLK_CODE_GOLD_PAYMENT, b'0', b'2', b'5'];
        bytes.extend_from_slice(&enc("paid"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);

        let paid = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                gold_available: Some(30),
                ..Default::default()
            },
        );
        assert_eq!(paid.text, "paid");
        assert!(paid.events.iter().any(|event| {
            matches!(
                event,
                TlkRunEvent::GoldPayment {
                    amount: 25,
                    accepted: true
                }
            )
        }));

        let refused = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                gold_available: Some(10),
                ..Default::default()
            },
        );
        assert_eq!(refused.text, TLK_GOLD_PAYMENT_REFUSAL_MESSAGE);
        assert_eq!(refused.stop, TlkRunStop::GoldPaymentRefused { amount: 25 });
        assert_eq!(refused.consumed, 4);
        assert!(refused.events.iter().any(|event| {
            matches!(
                event,
                TlkRunEvent::GoldPayment {
                    amount: 25,
                    accepted: false
                }
            )
        }));
        assert!(!refused.events.iter().any(|event| {
            matches!(
                event,
                TlkRunEvent::GotoLabel {
                    from: TLK_CODE_GOLD_PAYMENT,
                    ..
                }
            )
        }));
    }

    #[test]
    fn multiple_gold_payments_share_one_decreasing_affordability_view() {
        let bytes = [
            TLK_CODE_GOLD_PAYMENT,
            b'0',
            b'2',
            b'0',
            TLK_CODE_GOLD_PAYMENT,
            b'0',
            b'2',
            b'0',
            TLK_CODE_END_OF_RESPONSE,
        ];
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                gold_available: Some(30),
                ..Default::default()
            },
        );
        assert_eq!(out.stop, TlkRunStop::GoldPaymentRefused { amount: 20 });
        assert_eq!(
            out.events
                .iter()
                .filter_map(|event| match event {
                    TlkRunEvent::GoldPayment { accepted, .. } => Some(*accepted),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![true, false]
        );
        assert_eq!(out.text, TLK_GOLD_PAYMENT_REFUSAL_MESSAGE);
    }

    #[test]
    fn gold_refusal_discards_only_the_pending_trailing_word() {
        let mut bytes = enc("kept partial");
        bytes.extend_from_slice(&[TLK_CODE_GOLD_PAYMENT, b'0', b'0', b'5']);
        bytes.extend_from_slice(&enc("unreachable"));
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                gold_available: Some(4),
                ..Default::default()
            },
        );
        assert_eq!(out.text, format!("kept {TLK_GOLD_PAYMENT_REFUSAL_MESSAGE}"));
    }

    #[test]
    fn recruit_speaker_neither_prompts_nor_stops_the_stream() {
        // §7.6: `0x84` has "no player prompt and no input read", takes no
        // argument bytes, and the response keeps emitting around it.
        let mut bytes = enc("Aye. ");
        bytes.push(TLK_CODE_RECRUIT_SPEAKER);
        bytes.extend_from_slice(&enc("I come."));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                // Even with the interactive ask yield armed, RECRUIT-SPEAKER
                // must not stop: `yield_on_ask` governs `0x88` alone.
                yield_on_ask: true,
                ..Default::default()
            },
        );
        assert_eq!(out.text, "Aye. I come.");
        assert_eq!(out.stop, TlkRunStop::EndOfResponse);
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, TlkRunEvent::RecruitSpeaker))
        );
    }

    #[test]
    fn ask_who_sets_the_speaking_npc_branch_bit_on_a_match() {
        // §7.6: ASK-WHO "is the in-stream setter for the bank that `0x8C`
        // tests", and the bit is the speaking NPC's own roster slot.
        let bytes = vec![TLK_CODE_ASK_WHO, TLK_CODE_END_OF_RESPONSE];
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(5),
                ask_who_response: 3,
                ..Default::default()
            },
        );
        assert_eq!(out.branch_flags_set, 1u32 << 5);
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, TlkRunEvent::AskedWho(3)))
        );
    }

    #[test]
    fn ask_who_sets_nothing_on_empty_or_unmatched_input() {
        let bytes = vec![TLK_CODE_ASK_WHO, TLK_CODE_END_OF_RESPONSE];
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: Some(5),
                ask_who_response: 0,
                ..Default::default()
            },
        );
        assert_eq!(out.branch_flags_set, 0);
    }

    #[test]
    fn ask_who_without_a_named_npc_slot_builds_a_zero_mask() {
        // §10: an index the engine cannot name "should make it build a
        // zero mask, so such tests read as clear and such setters are
        // no-ops".
        let bytes = vec![TLK_CODE_ASK_WHO, TLK_CODE_END_OF_RESPONSE];
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                npc_slot: None,
                ask_who_response: 2,
                ..Default::default()
            },
        );
        assert_eq!(out.branch_flags_set, 0);
    }

    #[test]
    fn curse_check_emitted_once_and_then_resets() {
        let bytes = vec![
            TLK_CODE_CURSE_CHECK,
            TLK_CODE_CURSE_CHECK,
            TLK_CODE_END_OF_RESPONSE,
        ];
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                curse_seen: true,
                ..Default::default()
            },
        );
        let curse_events: Vec<_> = out
            .events
            .iter()
            .filter_map(|e| match e {
                TlkRunEvent::CurseChecked { curse_seen } => Some(*curse_seen),
                _ => None,
            })
            .collect();
        assert_eq!(curse_events, vec![true, false]);
    }

    #[test]
    fn wait_key_treated_as_newline_when_yield_disabled() {
        let mut bytes = enc("a");
        bytes.push(TLK_CODE_WAIT_KEY);
        bytes.extend_from_slice(&enc("b"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.text, "a\nb");
    }

    #[test]
    fn yield_on_pause_stops_at_pause_code() {
        let mut bytes = enc("a");
        bytes.push(TLK_CODE_PAUSE);
        bytes.extend_from_slice(&enc("b"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                yield_on_pause: true,
                ..Default::default()
            },
        );
        assert_eq!(out.text, "a");
        assert!(matches!(out.stop, TlkRunStop::PausedAt(_)));
    }

    #[test]
    fn ask_who_can_yield_and_resume_same_stream() {
        let mut inputs = TlkRunInputs {
            yield_on_ask: true,
            ..Default::default()
        };
        let mut bytes = enc("Who:");
        bytes.push(TLK_CODE_ASK_WHO);
        bytes.extend_from_slice(&enc("Done"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);

        let first = run_tlk_stream(&bytes, &inputs);
        assert_eq!(first.text, "Who:");
        assert!(matches!(
            first.stop,
            TlkRunStop::AskingWho(cursor) if cursor == enc("Who:").len() + 1
        ));

        inputs.yield_on_ask = false;
        inputs.ask_who_response = 2;
        let resumed = run_tlk_stream_from(&bytes, first.consumed, &inputs);
        assert_eq!(resumed.text, "Done");
        assert_eq!(resumed.stop, TlkRunStop::EndOfResponse);
    }

    #[test]
    fn print_mask_toggle_does_not_emit_visible_text() {
        let mut bytes = vec![TLK_CODE_PROTECT_RUN];
        bytes.extend_from_slice(&enc("X"));
        bytes.push(TLK_CODE_PROTECT_RUN);
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        assert_eq!(render(&bytes).text, "X");
    }

    #[test]
    fn double_quote_dedup_collapses_adjacent_quotes() {
        // Two encoded quotes back-to-back should render as a single "
        let mut bytes = vec![TLK_DOUBLE_QUOTE_ENCODED, TLK_DOUBLE_QUOTE_ENCODED];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.text, "\"");
    }

    #[test]
    fn dictionary_token_uses_expansion_when_provided() {
        let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        dict[0x0f] = "Britannia";
        let mut bytes = vec![0x10u8];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );
        assert_eq!(out.text, " Britannia");
        assert_eq!(
            out.rendered_glyphs,
            " Britannia"
                .bytes()
                .map(TlkRenderedGlyph::ordinary)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn first_dialogue_dictionary_token_uses_entry_zero() {
        let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        dict[0] = "the";
        let out = run_tlk_stream(
            &[0x01u8, TLK_CODE_END_OF_RESPONSE],
            &TlkRunInputs {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );
        assert_eq!(out.text, " the");
    }

    #[test]
    fn empty_dictionary_entry_emits_raw_runic_token_without_pending_space() {
        let dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        let mut bytes = vec![0x08u8];
        bytes.extend_from_slice(&enc("word"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );
        assert_eq!(out.text.as_bytes(), b" \x08word");
        assert_eq!(out.rendered_glyphs[0], TlkRenderedGlyph::ordinary(b' '));
        assert_eq!(out.rendered_glyphs[1], TlkRenderedGlyph::runic(0x08));
        assert!(
            out.rendered_glyphs[2..]
                .iter()
                .all(|glyph| glyph.font == TlkGlyphFont::Ordinary)
        );
    }

    #[test]
    fn populated_dictionary_tokens_follow_exact_leading_and_pending_space_order() {
        let mut dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        dict[0] = "the";
        dict[1] = "thou";
        let mut bytes = vec![0x01, 0x02];
        bytes.extend_from_slice(&enc("letter"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );
        assert_eq!(out.text, " the thou letter");
    }

    #[test]
    fn protect_run_preserves_runic_font_selection_per_glyph() {
        let mut bytes = vec![TLK_CODE_PROTECT_RUN];
        bytes.extend_from_slice(&enc("INOP"));
        bytes.push(TLK_CODE_PROTECT_RUN);
        bytes.extend_from_slice(&enc(" done"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.text, "INOP done");
        assert!(
            out.rendered_glyphs[..4]
                .iter()
                .all(|glyph| glyph.font == TlkGlyphFont::Runic)
        );
        assert!(
            out.rendered_glyphs[4..]
                .iter()
                .all(|glyph| glyph.font == TlkGlyphFont::Ordinary)
        );
    }

    #[test]
    fn dictionary_token_without_expansion_uses_placeholder() {
        let bytes = vec![0x42u8, TLK_CODE_END_OF_RESPONSE];
        let out = render(&bytes);
        assert!(out.text.contains("[w42]"));
    }

    #[test]
    fn malformed_action_dispatch_reports_short_introducer() {
        let bytes = vec![TLK_CODE_ACTION_DISPATCH];
        let out = render(&bytes);
        assert!(matches!(out.stop, TlkRunStop::MalformedIntroducer(_)));
    }

    #[test]
    fn malformed_gold_payment_reports_short_introducer() {
        let bytes = vec![TLK_CODE_GOLD_PAYMENT, b'1', b'2'];
        let out = render(&bytes);
        assert!(matches!(out.stop, TlkRunStop::MalformedIntroducer(_)));
    }

    #[test]
    fn unresolved_goto_label_reported_and_runner_stops() {
        let bytes = vec![TLK_CODE_IF_ELSE_ALT, 0x00, 0x97];
        let out = render(&bytes);
        assert_eq!(out.stop, TlkRunStop::UnresolvedGotoLabel(0x97));
    }

    #[test]
    fn exhausted_input_stops_with_exhausted_marker() {
        let bytes = enc("no terminator");
        let out = render(&bytes);
        assert_eq!(out.stop, TlkRunStop::Exhausted);
        assert!(!out.text.is_empty());
    }

    #[test]
    fn signal_flag_event_recorded_in_order_with_other_events() {
        let mut bytes = vec![
            TLK_CODE_ACTION_DISPATCH,
            0x05, // signal flag
            TLK_CODE_ACTION_DISPATCH,
            b'B', // RaiseGold
        ];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        let kinds: Vec<_> = out
            .events
            .iter()
            .map(|e| matches!(e, TlkRunEvent::SignalFlag(_)))
            .collect();
        assert_eq!(kinds, vec![true, false]);
    }

    #[test]
    fn find_label_position_returns_byte_after_label() {
        let bytes = vec![0xA1, 0xA2, 0x92, 0xA3];
        let pos = find_label_position(&bytes, 0x92).unwrap();
        assert_eq!(pos, 3);
    }

    #[test]
    fn find_label_position_rejects_non_label_bytes() {
        assert!(find_label_position(&[0xA1], 0x42).is_none());
    }

    /// Local clean asset folder, or `None` when it is not installed.
    /// Same skip discipline as the other asset-backed tests: no asset
    /// bytes are ever embedded in the crate.
    fn local_clean_assets() -> Option<std::path::PathBuf> {
        let dir = crate::test_fixtures::configured_original_asset_dir()?;
        dir.join("CASTLE.TLK").is_file().then_some(dir)
    }

    const SHIPPED_TLK_FILES: [&str; 4] = ["CASTLE.TLK", "TOWNE.TLK", "DWELLING.TLK", "KEEP.TLK"];

    /// Run every field of every NPC blob in the shipped corpus through
    /// the dispatcher, resuming past each yielding stop so that every
    /// byte in a payload position is actually dispatched, and hand each
    /// stream's output to `visit`.
    ///
    /// Bytes consumed as multi-byte introducer arguments are deliberately
    /// not visited: the dispatcher never classifies them, so they are not
    /// in a payload position.
    fn for_each_shipped_stream(
        dir: &std::path::Path,
        mut visit: impl FnMut(&str, u16, usize, &TlkRunOutput),
    ) {
        for file in SHIPPED_TLK_FILES {
            let blobs = crate::map_io::parse_tlk_raw(&dir.join(file))
                .unwrap_or_else(|err| panic!("{file} parses: {err}"));
            let mut npc_ids: Vec<u16> = blobs.keys().copied().collect();
            npc_ids.sort_unstable();
            for npc_id in npc_ids {
                for (field_idx, field) in blobs[&npc_id].iter().enumerate() {
                    let mut cursor = 0usize;
                    // Bounded: every iteration must advance the cursor.
                    while cursor < field.len() {
                        let out = run_tlk_stream_from(
                            field,
                            cursor,
                            &TlkRunInputs {
                                avatar_name: "Avatar",
                                ..Default::default()
                            },
                        );
                        visit(file, npc_id, field_idx, &out);
                        let next = out.consumed.max(cursor + 1);
                        if next <= cursor {
                            break;
                        }
                        cursor = next;
                    }
                }
            }
        }
    }

    /// The census that the silent catch-all used to make impossible.
    ///
    /// `conversation.md` §7 partitions the byte space, so no byte in a
    /// payload position may reach the unclassified arm. This asserts it
    /// mechanically against every shipped stream rather than by reading
    /// the match arms. `0x90` LABEL-RECORD was the resident this found.
    #[test]
    fn shipped_corpus_reaches_no_unclassified_dispatcher_byte() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        let mut census: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
        let mut first_site: std::collections::BTreeMap<u8, String> =
            std::collections::BTreeMap::new();
        for_each_shipped_stream(&dir, |file, npc_id, field_idx, out| {
            for event in &out.events {
                if let TlkRunEvent::UnclassifiedByte { byte, offset } = event {
                    *census.entry(*byte).or_default() += 1;
                    first_site.entry(*byte).or_insert_with(|| {
                        format!("{file} npc {npc_id} field {field_idx} +{offset:#x}")
                    });
                }
            }
        });
        assert!(
            census.is_empty(),
            "shipped corpus reached the unclassified arm: {}",
            census
                .iter()
                .map(|(byte, count)| format!(
                    "{byte:#04X} x{count} (first at {})",
                    first_site[byte]
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    /// Renders real shipped streams and asserts the **expanded text**,
    /// which is the class of test whose absence let a dropped word
    /// survive: the route-smoke harness runs real `.TLK` bytes but
    /// asserts state and frame-kind invariants, and its message-bytes and
    /// hash values are diagnostics rather than assertions, so no test
    /// compared rendered characters against shipped content.
    ///
    /// It asserts a *property* rather than a transcript: a sentinel
    /// avatar name must appear in the output once per dispatched `0x81`,
    /// in every file. No dialogue text is embedded, per the workspace's
    /// no-committed-content rule.
    #[test]
    fn shipped_corpus_renders_the_avatar_name_once_per_print_avatar_name_byte() {
        let Some(dir) = local_clean_assets() else {
            return;
        };
        // Deliberately not a word any dialogue could contain, so a
        // substring count cannot be inflated by shipped text.
        const SENTINEL: &str = "Zzyzx";
        let mut per_file: std::collections::BTreeMap<&str, (usize, usize)> =
            std::collections::BTreeMap::new();
        for file in SHIPPED_TLK_FILES {
            let blobs = crate::map_io::parse_tlk_raw(&dir.join(file))
                .unwrap_or_else(|err| panic!("{file} parses: {err}"));
            let mut npc_ids: Vec<u16> = blobs.keys().copied().collect();
            npc_ids.sort_unstable();
            let entry = per_file.entry(file).or_default();
            for npc_id in npc_ids {
                for field in &blobs[&npc_id] {
                    let mut cursor = 0usize;
                    while cursor < field.len() {
                        let out = run_tlk_stream_from(
                            field,
                            cursor,
                            &TlkRunInputs {
                                avatar_name: SENTINEL,
                                ..Default::default()
                            },
                        );
                        // Count the codes this run actually dispatched,
                        // not every `0x81` byte in the field: bytes
                        // consumed as introducer arguments are not
                        // dispatched, and neither is anything past a stop.
                        let dispatched = field[cursor..out.consumed.max(cursor)]
                            .iter()
                            .filter(|byte| **byte == TLK_CODE_PRINT_AVATAR_NAME)
                            .count();
                        let rendered = out.text.matches(SENTINEL).count();
                        assert!(
                            rendered >= dispatched.min(1) || dispatched == 0,
                            "{file} npc {npc_id}: dispatched {dispatched} PRINT-AVATAR-NAME bytes, rendered {rendered} names"
                        );
                        entry.0 += dispatched;
                        entry.1 += rendered;
                        let next = out.consumed.max(cursor + 1);
                        if next <= cursor {
                            break;
                        }
                        cursor = next;
                    }
                }
            }
        }
        for (file, (dispatched, rendered)) in &per_file {
            assert_eq!(
                rendered, dispatched,
                "{file} dropped the Avatar's name: {dispatched} dispatched, {rendered} rendered"
            );
        }

        // Pinned so that a *drop in reach* announces itself too. An
        // assertion of the form "rendered == dispatched" is satisfied
        // vacuously by dispatching nothing, which is the same silence
        // this whole change is removing.
        //
        // These are dispatch counts, not the raw byte counts: the four
        // files hold 16/35/2/7 = 60 `0x81` bytes, and 59 are reached.
        // The missing one is `CASTLE.TLK` npc 18's second, which sits in
        // field index 41 — past the 40-field cap in
        // `map_io::parse_tlk_blob_fields_raw`, so the parser never yields
        // it. Eleven shipped `CASTLE.TLK` blobs hold 41..=54 fields. That
        // is an upstream truncation, not a dispatcher fault; when it is
        // fixed this expectation becomes 16 and this test will say so.
        let counts: Vec<(&str, usize)> = per_file
            .iter()
            .map(|(file, (dispatched, _))| (*file, *dispatched))
            .collect();
        assert_eq!(
            counts,
            vec![
                ("CASTLE.TLK", 15),
                ("DWELLING.TLK", 2),
                ("KEEP.TLK", 7),
                ("TOWNE.TLK", 35),
            ]
        );
    }

    /// The static half of the same question. The corpus census can only
    /// report byte values shipped content happens to contain; this walks
    /// the entire `0x00..=0xFF` domain so a value no shipped blob uses
    /// still cannot reach the unclassified arm unnoticed.
    #[test]
    fn every_byte_value_has_a_dispatcher_classification() {
        let unclassified: Vec<String> = (0x00u8..=0xFF)
            .filter(|byte| {
                let out = run_tlk_stream(
                    // Trailing argument bytes so multi-byte introducers
                    // are well-formed rather than stopping short.
                    &[*byte, 0xA1, 0xA1, 0xA1],
                    &TlkRunInputs::default(),
                );
                out.events
                    .iter()
                    .any(|event| matches!(event, TlkRunEvent::UnclassifiedByte { .. }))
            })
            .map(|byte| format!("{byte:#04X}"))
            .collect();
        assert!(
            unclassified.is_empty(),
            "bytes with no dispatcher classification: {}",
            unclassified.join(", ")
        );
    }

    /// `conversation.md §7.7`, public issue #164 / R278: `0x90` is a
    /// declaration marker to the scanner but an accidental printable byte to
    /// ordinary execution. Its successor is dispatched independently.
    #[test]
    fn ordinary_label_marker_prints_codepoint_10_then_dispatches_the_label() {
        let mut bytes = enc("A");
        bytes.push(TLK_CODE_LABEL_RECORD);
        bytes.push(0x91);
        let out = render(&bytes);

        assert_eq!(out.text.as_bytes(), &[b'A', 0x10]);
        assert_eq!(out.stop, TlkRunStop::LabelTransfer(0x91));
        assert_eq!(out.consumed, 3);
        assert!(
            !out.events
                .iter()
                .any(|event| matches!(event, TlkRunEvent::UnclassifiedByte { .. }))
        );
    }

    /// A bare `0x90` is printable, not a truncated multi-byte command.
    #[test]
    fn bare_label_record_marker_prints_and_exhausts() {
        let bytes = vec![TLK_CODE_LABEL_RECORD];
        let out = render(&bytes);
        assert_eq!(out.text.as_bytes(), &[0x10]);
        assert_eq!(out.stop, TlkRunStop::Exhausted);
        assert_eq!(out.consumed, 1);
    }

    #[test]
    fn ordinary_label_byte_stops_for_session_label_handler() {
        let mut bytes = enc("Ask");
        bytes.push(0x91);
        bytes.extend_from_slice(&enc("ignored"));
        let out = render(&bytes);

        assert_eq!(out.text, "Ask");
        assert_eq!(out.stop, TlkRunStop::LabelTransfer(0x91));
    }

    /// `formats/font-ch.md` section 4: the `.CH` fonts store codes
    /// `0x00..=0x7F` only, and "ordinary cell output ignores high-bit
    /// bytes". A Rust `&str` is UTF-8, so walking bytes would split a
    /// character such as `U+00D3` into `0xC3 0x93` and hand the fixed-cell
    /// renderer a lead byte no glyph slot covers. Every emitted cell must
    /// be addressable.
    #[test]
    fn engine_text_glyphs_drop_characters_the_ch_fonts_cannot_address() {
        // `input.md` section 5: `0xD3` is the northwest direction code,
        // which reaches the dispatcher's refusal line as `char::from`.
        let leaked = char::from(0xD3);
        let glyphs = ordinary_glyphs_from_engine_text(&format!("Unhandled command `{leaked}`."));

        assert!(
            glyphs.iter().all(|glyph| glyph.byte < 0x80),
            "engine text must not emit a cell the `.CH` fonts cannot address: {:02x?}",
            glyphs.iter().map(|glyph| glyph.byte).collect::<Vec<_>>()
        );
        assert!(
            !glyphs.iter().any(|glyph| glyph.byte == 0xC3),
            "the UTF-8 lead byte of the dropped character must not reach a cell"
        );
        assert_eq!(
            glyphs
                .iter()
                .map(|glyph| char::from(glyph.byte))
                .collect::<String>(),
            "Unhandled command ``.",
            "an ignored character occupies no cell, so the rest of the line closes up"
        );
        assert!(
            glyphs
                .iter()
                .all(|glyph| glyph.font == TlkGlyphFont::Ordinary),
            "the resident text font is unchanged by the drop"
        );
    }

    /// The same rule on the runic side: the alternate font is the same
    /// 128-slot geometry (`formats/font-ch.md` section 3), so it gets the
    /// same filter rather than a wider range.
    #[test]
    fn engine_text_glyphs_apply_the_same_range_to_the_runic_font() {
        let glyphs = glyphs_from_engine_text("Ver\u{e9}?", TlkGlyphFont::Runic);

        assert!(glyphs.iter().all(|glyph| glyph.byte < 0x80));
        assert!(glyphs.iter().all(|glyph| glyph.font == TlkGlyphFont::Runic));
        assert_eq!(
            glyphs
                .iter()
                .map(|glyph| char::from(glyph.byte))
                .collect::<String>(),
            "Ver?"
        );
    }

    /// The leak that actually reached the renderer arrived through the
    /// message window, whose rows are painted cell-by-cell straight from
    /// `glyphs`. Guard that whole path, not just the helper.
    #[test]
    fn message_window_rows_never_carry_a_cell_outside_the_ch_font_range() {
        let mut log = crate::GameplayMessageLog::new();
        log.push_command("North");
        log.push_output(&format!("Unhandled command `{}`.", char::from(0xD3)));
        let layout = crate::layout_message_window(&log, Some(""));

        assert!(
            layout
                .rows
                .iter()
                .flat_map(|row| row.glyphs.iter())
                .all(|glyph| glyph.byte < 0x80),
            "a message-window cell outside `0x00..=0x7F` would panic the fixed-cell renderer"
        );
        let placed = layout
            .rows
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            placed.contains("Unhandled") && placed.contains("command"),
            "the refusal line is still placed, wrapped across the window: {placed:?}"
        );
        assert!(
            !placed.chars().any(|ch| !ch.is_ascii()),
            "the placed text view matches the placed cells: {placed:?}"
        );
    }
}
