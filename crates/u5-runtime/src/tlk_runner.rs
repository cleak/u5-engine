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
use crate::map_io::talk_branch_flag_is_set;
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
    /// Common-word dictionary (128 slots; index by token byte `0x01..=0x7F`).
    /// `None` is acceptable — token bytes then expand to `"[w<n>]"`.
    pub dictionary: Option<&'a [&'a str; COMMON_WORD_DICTIONARY_ENTRIES]>,
    /// `0x8B` curse-check result. `true` means the player typed something
    /// the engine treats as a curse during the current conversation; the
    /// runner uses it to gate the immediately following control byte the
    /// same way the original does (per `conversation.md` §7.5).
    pub curse_seen: bool,
    /// `0x85` GOLD-PAYMENT prompt result. `true` means the player accepted
    /// the gold deduction; `false` means they declined or had insufficient
    /// gold. The runner records the amount regardless and uses this flag
    /// to choose between the [`TLK_GOLD_PAYMENT_PAID_LABEL`] and
    /// [`TLK_GOLD_PAYMENT_REFUSED_LABEL`] follow-ups. Those two label
    /// values are an engine convention, not published by the spec — see
    /// their doc comments.
    pub gold_payment_accepted: bool,
    /// Optional party gold available to the payment prompt. When supplied,
    /// the runner treats an otherwise accepted payment as refused if the
    /// requested amount exceeds this value.
    pub gold_available: Option<u16>,
    /// `0x84` ASK-PARTY-NAME response: 1-based party-slot index that
    /// matched, or `0` for no match. The runner stores it for the caller
    /// but does not branch on it directly (the shipped blobs gate the
    /// follow-up text via separate flags).
    pub ask_party_name_response: u8,
    /// `0x88` ASK-WHO response: 1-based party-slot index, or `0` for
    /// cancel/no match. Recorded for the caller.
    pub ask_who_response: u8,
    /// `0x83` PAUSE / `0x8F` WAIT-KEY behaviour. When `true`, the runner
    /// stops at each pause/wait-key with [`TlkRunStop::PausedAt`] /
    /// [`TlkRunStop::WaitingKey`] so the caller can flush a page and
    /// resume. When `false`, the runner treats pauses as no-ops and
    /// wait-key as a single newline and keeps going.
    pub yield_on_pause: bool,
    /// `0x84` ASK-PARTY-NAME / `0x88` ASK-WHO behaviour. When `true`,
    /// the runner stops immediately after the ask code so the
    /// interactive conversation wrapper can collect a free-text answer
    /// and resume the same stream with the matched party slot.
    pub yield_on_ask: bool,
    /// `0x85` GOLD-PAYMENT behaviour. When `true`, the runner stops
    /// before applying the payment branch so the interactive wrapper
    /// can ask the player whether to pay and then resume from the
    /// payment opcode with the selected answer.
    pub yield_on_gold_payment: bool,
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
    /// Stopped at `0x84` ASK-PARTY-NAME (only when `yield_on_ask` is set).
    AskingPartyName(usize),
    /// Stopped at `0x88` ASK-WHO (only when `yield_on_ask` is set).
    AskingWho(usize),
    /// Stopped at `0x85` GOLD-PAYMENT (only when
    /// `yield_on_gold_payment` is set). `cursor` points back to the
    /// payment opcode so a caller can resume from the branch point
    /// after the player answers.
    AskingGoldPayment { cursor: usize, amount: u16 },
    /// Encountered a malformed multi-byte introducer (short arg span).
    MalformedIntroducer(usize),
    /// Encountered an unresolved GOTO-LABEL target (label byte not found
    /// in the stream beyond the current cursor). The runner stops to
    /// avoid an infinite loop.
    UnresolvedGotoLabel(u8),
    /// Hit `0x87` SET-FLAG / follow-up keyword scan. `cursor` points
    /// just past the control byte; the conversation wrapper owns the
    /// recursive keyword scan and then resumes this stream from there.
    FollowUpKeywordScan(usize),
    /// Encountered a `0x91..=0x9F` label byte through ordinary stream
    /// execution. The conversation session owns the labelled-record
    /// handler and any scoped prompt that follows.
    LabelTransfer(u8),
    /// Ran into a `0x90 <label>` record-declaration marker
    /// (`conversation.md` §7.7) while executing a stream. Both marker
    /// bytes are consumed; the payload is the declared label.
    ///
    /// This is distinct from [`TlkRunStop::LabelTransfer`]: that is a
    /// GOTO *to* a label, this is arriving at a label's *declaration*,
    /// which means the current record ended without its own terminator.
    ///
    /// **Spec gap.** §7.7 publishes the marker's shape and its role in
    /// the label scan, but not what the byte runner does on reaching one
    /// through ordinary in-stream execution. Rather than guess between
    /// "inert separator, keep running into the next record" and "the
    /// record ended", the runner surfaces the marker and stops. Callers
    /// that need the other reading should resume from `consumed`.
    LabelRecordMarker(u8),
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
    /// `0x84` ASK-PARTY-NAME: 1-based slot match (0 = no match).
    AskedPartyName(u8),
    /// `0x88` ASK-WHO: 1-based slot match (0 = cancel/no match).
    AskedWho(u8),
    /// `0x8B` CURSE-CHECK was reached.
    CurseChecked { curse_seen: bool },
    /// `0x8F` WAIT-KEY synthesised newline (when `yield_on_pause` false).
    WaitKeyTreatedAsNewline,
    /// `0x83` PAUSE encountered (when `yield_on_pause` false).
    PauseSkipped,
    /// `0x8C` IF/ELSE branch entered: `taken_else` reflects whether the
    /// flag-bit was set (per §7.6 the *set* path is the ELSE arm).
    IfElseBranchTaken { bit: u8, taken_else: bool },
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
    /// Mask of branch-flag bits the stream set.
    ///
    /// Kept for conversation-session compatibility with callers that
    /// merge branch effects, though the public `0x87` contract is a
    /// follow-up keyword scan rather than a direct bit setter.
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
                if leading_space_pending {
                    out.text.push(' ');
                    leading_space_pending = false;
                }
                out.text.push_str(inputs.avatar_name);
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
                out.text.push('\n');
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
                out.text.push('\n');
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
            TLK_CODE_SET_FLAG => {
                out.stop = TlkRunStop::FollowUpKeywordScan(pos);
                out.consumed = pos;
                return out;
            }
            TLK_CODE_ASK_PARTY_NAME => {
                if inputs.yield_on_ask {
                    out.stop = TlkRunStop::AskingPartyName(pos);
                    out.consumed = pos;
                    return out;
                }
                let slot = inputs.ask_party_name_response;
                out.events.push(TlkRunEvent::AskedPartyName(slot));
            }
            TLK_CODE_ASK_WHO => {
                if inputs.yield_on_ask {
                    out.stop = TlkRunStop::AskingWho(pos);
                    out.consumed = pos;
                    return out;
                }
                let slot = inputs.ask_who_response;
                out.events.push(TlkRunEvent::AskedWho(slot));
            }
            TLK_CODE_GOLD_PAYMENT => {
                let code_start = pos - 1;
                let arg_start = pos;
                let span = bytes.get(pos..pos + 3);
                let Some(span) = span else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 3;
                let arg_end = pos;
                if let Some(amount) = tlk_gold_payment_amount(span[0], span[1], span[2]) {
                    if inputs.yield_on_gold_payment {
                        out.stop = TlkRunStop::AskingGoldPayment {
                            cursor: code_start,
                            amount,
                        };
                        out.consumed = pos;
                        return out;
                    }
                    let accepted = inputs.gold_payment_accepted
                        && inputs
                            .gold_available
                            .map_or(true, |available| available >= amount);
                    out.events
                        .push(TlkRunEvent::GoldPayment { amount, accepted });
                    let target_label = if accepted {
                        TLK_GOLD_PAYMENT_PAID_LABEL
                    } else {
                        TLK_GOLD_PAYMENT_REFUSED_LABEL
                    };
                    if let Some(target_pos) =
                        find_label_position_excluding(bytes, target_label, arg_start, arg_end)
                    {
                        out.events.push(TlkRunEvent::GotoLabel {
                            from: TLK_CODE_GOLD_PAYMENT,
                            to: target_label,
                        });
                        pos = target_pos;
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
                let Some(&arg) = bytes.get(pos) else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 1;
                let bit = arg & 0x7F;
                let flag_set = talk_branch_flag_is_set(inputs.branch_flags, bit);
                out.events.push(TlkRunEvent::IfElseBranchTaken {
                    bit,
                    taken_else: flag_set,
                });
                // Per §7.6: if the flag bit is *set*, the runner jumps
                // straight to the ELSE arm. Arms are delimited by GOTO
                // label bytes (`0x91..=0x9F`); the not-taken arm is
                // skipped by scanning forward to the next label byte.
                if flag_set {
                    pos = skip_to_next_label(bytes, pos);
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
            TLK_CODE_LABEL_RECORD => {
                // `conversation.md` §7.7 / `tlk.md` §9: a label is
                // *declared* by the two-byte record marker `0x90 <label>`,
                // and `0x90` is "data structure, not ordinary printable
                // text". Both bytes belong to the marker, so both are
                // consumed here.
                //
                // Consuming only the `0x90` — which is what the silent
                // catch-all did — left the declaration's label byte to be
                // re-dispatched by the arm below, turning a record *header*
                // into a [`TlkRunStop::LabelTransfer`], i.e. reading a
                // declaration as a GOTO.
                let Some(&label) = bytes.get(pos) else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 1;
                // Reaching a record separator by falling through in-stream
                // means the current record ran past its own terminator into
                // the next record's header. §7.7 does not publish what the
                // dispatcher does here, so the runner refuses to invent
                // flow: it stops with a named reason and lets the
                // conversation session, which owns labelled-block and
                // scoped-prompt handling, decide. See the doc comment on
                // [`TlkRunStop::LabelRecordMarker`].
                out.stop = TlkRunStop::LabelRecordMarker(label);
                out.consumed = pos;
                return out;
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
                    if let Some(dict) = inputs.dictionary {
                        let expansion = dict.get(idx).copied().unwrap_or("");
                        if expansion.is_empty() {
                            leading_space_pending = true;
                        } else {
                            if leading_space_pending {
                                out.text.push(' ');
                                leading_space_pending = false;
                            }
                            out.text.push_str(expansion);
                            last_emitted = expansion.bytes().last();
                        }
                    } else {
                        // Fallback placeholder keeps the runner
                        // deterministic even without dictionary bytes.
                        if leading_space_pending {
                            out.text.push(' ');
                            leading_space_pending = false;
                        }
                        out.text.push_str(&format!("[w{byte:02X}]"));
                        last_emitted = Some(b']');
                    }
                } else if (TLK_PRINTABLE_TEXT_FIRST..=TLK_PRINTABLE_TEXT_LAST).contains(&byte) {
                    let glyph = byte ^ TLK_TEXT_XOR_MASK;
                    if byte == TLK_DOUBLE_QUOTE_ENCODED && last_emitted == Some(b'"') {
                        // §7.5 double-quote dedup: collapse adjacent ""
                        // into a single visible quote.
                        last_emitted = None;
                        continue;
                    }
                    if matches!(print_mask, TlkPrintMaskState::ProtectedRun) {
                        // Protected runs keep spaces from triggering a
                        // soft flush — the rendered text is identical in
                        // our single-pass model, but we record the state
                        // for completeness.
                    }
                    if leading_space_pending {
                        out.text.push(' ');
                        leading_space_pending = false;
                    }
                    out.text.push(glyph as char);
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
                    // failing test. `0x90` LABEL-RECORD lived here unnoticed
                    // for exactly that reason. The byte is now recorded as an
                    // event so callers and tests can see it, and `debug_assert`
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

/// Skip past the next label byte (`0x91..=0x9F`). Used by the IF-ELSE
/// taken-branch logic to jump over the not-taken arm.
fn skip_to_next_label(bytes: &[u8], from: usize) -> usize {
    let mut pos = from;
    while pos < bytes.len() {
        if is_tlk_label_byte(bytes[pos]) {
            return pos + 1;
        }
        pos += 1;
    }
    pos
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
    fn set_flag_yields_follow_up_keyword_scan_without_consuming_next_byte() {
        let mut bytes = vec![TLK_CODE_SET_FLAG];
        bytes.extend_from_slice(&enc("tail"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.stop, TlkRunStop::FollowUpKeywordScan(1));
        assert_eq!(out.consumed, 1);
        assert!(out.text.is_empty());

        let resumed = run_tlk_stream_from(&bytes, out.consumed, &TlkRunInputs::default());
        assert_eq!(resumed.text, "tail");
    }

    #[test]
    fn if_else_falls_through_when_flag_clear() {
        // 0x8C arg=2; "THEN" arm; label 0x91; "ELSE" arm; EOR.
        let mut bytes = vec![TLK_CODE_IF_ELSE, 0x02];
        bytes.extend_from_slice(&enc("then"));
        bytes.push(0x91);
        bytes.extend_from_slice(&enc("else"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert!(out.text.starts_with("then"));
    }

    #[test]
    fn if_else_jumps_past_label_when_flag_set() {
        let mut bytes = vec![TLK_CODE_IF_ELSE, 0x02];
        bytes.extend_from_slice(&enc("then"));
        bytes.push(0x91);
        bytes.extend_from_slice(&enc("else"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                branch_flags: 1u32 << 2,
                ..Default::default()
            },
        );
        // With the flag bit set, the runner skips to past 0x91 — the
        // "else" arm — so "then" must not appear.
        assert!(!out.text.contains("then"));
        assert!(out.text.contains("else"));
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
                gold_payment_accepted: true,
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
    fn gold_payment_branches_to_paid_or_refused_label_by_affordability() {
        let mut bytes = vec![TLK_CODE_GOLD_PAYMENT, b'0', b'2', b'5'];
        bytes.push(TLK_GOLD_PAYMENT_PAID_LABEL);
        bytes.extend_from_slice(&enc("paid"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        bytes.push(TLK_GOLD_PAYMENT_REFUSED_LABEL);
        bytes.extend_from_slice(&enc("refused"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);

        let paid = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                gold_payment_accepted: true,
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
                gold_payment_accepted: true,
                gold_available: Some(10),
                ..Default::default()
            },
        );
        assert_eq!(refused.text, "refused");
        assert!(refused.events.iter().any(|event| {
            matches!(
                event,
                TlkRunEvent::GoldPayment {
                    amount: 25,
                    accepted: false
                }
            )
        }));
    }

    #[test]
    fn ask_party_name_and_ask_who_record_responses() {
        let bytes = vec![
            TLK_CODE_ASK_PARTY_NAME,
            TLK_CODE_ASK_WHO,
            TLK_CODE_END_OF_RESPONSE,
        ];
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                ask_party_name_response: 2,
                ask_who_response: 3,
                ..Default::default()
            },
        );
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, TlkRunEvent::AskedPartyName(2)))
        );
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, TlkRunEvent::AskedWho(3)))
        );
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
    fn ask_codes_can_yield_and_resume_same_stream() {
        let mut inputs = TlkRunInputs {
            yield_on_ask: true,
            ..Default::default()
        };
        let mut bytes = enc("Name:");
        bytes.push(TLK_CODE_ASK_PARTY_NAME);
        bytes.extend_from_slice(&enc("Done"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);

        let first = run_tlk_stream(&bytes, &inputs);
        assert_eq!(first.text, "Name:");
        assert!(matches!(
            first.stop,
            TlkRunStop::AskingPartyName(cursor) if cursor == enc("Name:").len() + 1
        ));

        inputs.yield_on_ask = false;
        inputs.ask_party_name_response = 2;
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
        assert_eq!(out.text, "Britannia");
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
        assert_eq!(out.text, "the");
    }

    #[test]
    fn empty_dictionary_entry_adds_space_before_next_text() {
        let dict: [&str; COMMON_WORD_DICTIONARY_ENTRIES] = [""; COMMON_WORD_DICTIONARY_ENTRIES];
        let mut bytes = vec![0x01u8];
        bytes.extend_from_slice(&enc("word"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = run_tlk_stream(
            &bytes,
            &TlkRunInputs {
                dictionary: Some(&dict),
                ..Default::default()
            },
        );
        assert_eq!(out.text, " word");
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
        let dir = std::env::var_os("U5_CLEAN_ASSETS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(crate::DEFAULT_GAME_DIR));
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

    /// `conversation.md §7.7` / `tlk.md §9`: a label is declared by the
    /// two-byte marker `0x90 <label>`. Both bytes belong to the marker.
    ///
    /// Before `0x90` had a dispatcher case it fell into the unclassified
    /// arm and was skipped, leaving its label byte to be re-dispatched as
    /// a GOTO — a record *declaration* read as a transfer. This asserts
    /// the two outcomes are distinguishable.
    #[test]
    fn label_record_marker_consumes_its_label_and_is_not_a_goto_transfer() {
        let mut bytes = enc("body");
        bytes.push(TLK_CODE_LABEL_RECORD);
        bytes.push(0x93);
        bytes.extend_from_slice(&enc("next record"));
        let out = render(&bytes);

        assert_eq!(out.text, "body");
        assert_eq!(out.stop, TlkRunStop::LabelRecordMarker(0x93));
        // Both marker bytes consumed: 4 text + 0x90 + 0x93.
        assert_eq!(out.consumed, 6);
        assert!(
            !matches!(out.stop, TlkRunStop::LabelTransfer(_)),
            "a declaration must not read as a transfer"
        );
        // And nothing reached the unclassified arm.
        assert!(
            !out.events
                .iter()
                .any(|event| matches!(event, TlkRunEvent::UnclassifiedByte { .. }))
        );
    }

    /// A bare `0x90` with no label byte after it is a truncated marker,
    /// reported the same way as a short multi-byte introducer argument
    /// span rather than silently ignored.
    #[test]
    fn truncated_label_record_marker_reports_malformed() {
        let bytes = vec![TLK_CODE_LABEL_RECORD];
        let out = render(&bytes);
        assert_eq!(out.stop, TlkRunStop::MalformedIntroducer(1));
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
}
