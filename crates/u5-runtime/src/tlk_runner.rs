//! `.TLK` byte-runner state machine per `systems/conversation.md` §7.
//!
//! The runner walks one response-stream's raw bytes, classifies each via
//! [`tlk_control_codes`], emits rendered text into a buffer, applies action
//! grants and branch-flag sets, and records prompts encountered along the
//! way. The deterministic-with-inputs pattern matches the rest of the
//! runtime: any byte-level decision that would normally require an input
//! prompt is pre-decided by the caller through [`TlkRunInputs`], so the
//! runner stays pure and testable.
//!
//! This is not the full interactive keyword-loop — that wraps the runner
//! and feeds keyword-response streams through it. The runner itself is the
//! per-stream engine: feed bytes in, get rendered text and side effects
//! out.

use crate::map_io::{talk_branch_flag_is_set, talk_branch_flag_mask};
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
    /// to choose between the `0x9E` "paid" and `0x9F` "refused" follow-ups.
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
}

/// Reason the runner stopped processing the current stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkRunStop {
    /// Hit `0xFF` end-of-response (normal end of a keyword response).
    EndOfResponse,
    /// Hit `0x82` end-of-stream (e.g. NPC walked off, conversation ended).
    EndOfStream,
    /// Hit a NUL byte before any explicit terminator. Treated as an
    /// implicit blob-end.
    NulTerminator,
    /// Exhausted the input slice without finding an explicit terminator.
    Exhausted,
    /// Stopped at `0x83` PAUSE (only when `yield_on_pause` is set).
    PausedAt(usize),
    /// Stopped at `0x8F` WAIT-KEY (only when `yield_on_pause` is set).
    WaitingKey(usize),
    /// Encountered a malformed multi-byte introducer (short arg span).
    MalformedIntroducer(usize),
    /// Encountered an unresolved GOTO-LABEL target (label byte not found
    /// in the stream beyond the current cursor). The runner stops to
    /// avoid an infinite loop.
    UnresolvedGotoLabel(u8),
}

/// One side-effect emitted while running a stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TlkRunEvent {
    /// `0x86` ACTION-DISPATCH letter verb.
    Action(TlkActionDispatchVerb),
    /// `0x86` argument below `'A'` (per-conversation signal flag bit).
    SignalFlag(u8),
    /// `0x87` SET-FLAG: bit index into the branch-flag slot.
    SetFlag(u8),
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
    /// A `0x9E`/`0x9F` GOTO-LABEL was followed.
    GotoLabel { from: u8, to: u8 },
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
    /// Mask of branch-flag bits the stream set via `0x87`.
    pub branch_flags_set: u32,
    /// Action-dispatch verbs encountered, in order.
    pub action_grants: Vec<TlkActionDispatchVerb>,
    /// Per-conversation signal-flag bits seen (post-mask argument < `'A'`).
    pub signal_flags: Vec<u8>,
    /// Side-effect events, in dispatch order. Mainly useful for tests
    /// and for the keyword-loop wrapper that needs to detect when to
    /// prompt for input or end the conversation.
    pub events: Vec<TlkRunEvent>,
    /// Why the runner stopped.
    pub stop: TlkRunStop,
    /// Byte index just past the last byte consumed.
    pub consumed: usize,
}

/// Execute the byte runner over `bytes` until an explicit terminator,
/// yield point, or malformed control sequence.
pub fn run_tlk_stream(bytes: &[u8], inputs: &TlkRunInputs) -> TlkRunOutput {
    let mut out = TlkRunOutput {
        stop: TlkRunStop::Exhausted,
        ..Default::default()
    };
    let mut pos = 0usize;
    let mut print_mask = TlkPrintMaskState::NormalBreaks;
    // Track the last *emitted* printable byte (pre-mask, post-XOR) so we
    // can collapse the on-disk `""` double-quote artefact per §7.5.
    let mut last_emitted: Option<u8> = None;
    // Curse check is reset when the runner starts and may flip later.
    let mut curse_pending = inputs.curse_seen;

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
                out.text.push_str(inputs.avatar_name);
                last_emitted = inputs.avatar_name.bytes().last();
            }
            TLK_CODE_PANEL_NEWLINE | TLK_CODE_LITERAL_NEWLINE => {
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
                let Some(&arg) = bytes.get(pos) else {
                    out.stop = TlkRunStop::MalformedIntroducer(pos);
                    out.consumed = pos;
                    return out;
                };
                pos += 1;
                let bit = arg & 0x7F;
                let mask = talk_branch_flag_mask(bit);
                out.branch_flags_set |= mask;
                out.events.push(TlkRunEvent::SetFlag(bit));
            }
            TLK_CODE_ASK_PARTY_NAME => {
                let slot = inputs.ask_party_name_response;
                out.events.push(TlkRunEvent::AskedPartyName(slot));
            }
            TLK_CODE_ASK_WHO => {
                let slot = inputs.ask_who_response;
                out.events.push(TlkRunEvent::AskedWho(slot));
            }
            TLK_CODE_GOLD_PAYMENT => {
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
                    let accepted = inputs.gold_payment_accepted
                        && inputs
                            .gold_available
                            .map_or(true, |available| available >= amount);
                    out.events
                        .push(TlkRunEvent::GoldPayment { amount, accepted });
                    let target_label = if accepted {
                        TLK_CODE_GOTO_LABEL_FIRST
                    } else {
                        TLK_CODE_GOTO_LABEL_LAST
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
                let branched = tlk_if_else_alt_branches(inputs.moral_standing, threshold);
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
                    // Labels are flow markers; when the runner reaches one
                    // through ordinary fall-through it simply continues.
                    continue;
                }
                if (TLK_DICTIONARY_TOKEN_FIRST..=TLK_DICTIONARY_TOKEN_LAST).contains(&byte) {
                    let idx = byte as usize;
                    let expansion = inputs
                        .dictionary
                        .and_then(|dict| dict.get(idx).copied())
                        .unwrap_or("");
                    if expansion.is_empty() {
                        // Fallback placeholder keeps the runner
                        // deterministic even without dictionary bytes.
                        out.text.push_str(&format!("[w{idx:02X}]"));
                    } else {
                        out.text.push_str(expansion);
                    }
                    last_emitted = expansion.bytes().last().or(Some(b' '));
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
                    out.text.push(glyph as char);
                    last_emitted = Some(glyph);
                } else {
                    // Unrecognised byte — skip it; the original engine's
                    // dispatcher is exhaustive but defensive callers may
                    // pass partial data.
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

    #[test]
    fn newline_control_bytes_emit_newline() {
        let mut bytes = enc("a");
        bytes.push(TLK_CODE_PANEL_NEWLINE);
        bytes.extend_from_slice(&enc("b"));
        bytes.push(TLK_CODE_LITERAL_NEWLINE);
        bytes.extend_from_slice(&enc("c"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        assert_eq!(render(&bytes).text, "a\nb\nc");
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
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, TlkRunEvent::Action(TlkActionDispatchVerb::RaiseFood))));
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
    fn set_flag_records_branch_bit() {
        let mut bytes = vec![TLK_CODE_SET_FLAG, 0x03];
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        let out = render(&bytes);
        assert_eq!(out.branch_flags_set, 1u32 << 3);
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
        bytes.push(TLK_CODE_GOTO_LABEL_FIRST);
        bytes.extend_from_slice(&enc("paid"));
        bytes.push(TLK_CODE_END_OF_RESPONSE);
        bytes.push(TLK_CODE_GOTO_LABEL_LAST);
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
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, TlkRunEvent::AskedPartyName(2))));
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, TlkRunEvent::AskedWho(3))));
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
        dict[0x10] = "Britannia";
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
}
