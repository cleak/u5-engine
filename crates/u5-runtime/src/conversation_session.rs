//! Interactive conversation session state machine per
//! `systems/conversation.md` §6.
//!
//! The session wraps the per-stream byte-runner in a stateful loop:
//! the player presses Talk, sees the greeting, and then the engine
//! enters a keyword-input prompt that keeps reading lines until the
//! NPC says Bye. Each typed keyword resolves to one of:
//!
//! - the empty Bye shortcut,
//! - one of the five reserved functional words (NAME/JOB/WORK/BYE/THANK),
//! - an ordinary NPC keyword pair,
//! - or the "I cannot help thee" no-match response.
//!
//! The session owns the per-conversation print-mask state, the active
//! NPC's raw blob fields, the resolved avatar name, and the running
//! branch-flag bitmap. Each call returns the rendered text from the
//! resolved response plus a transition outcome so the harness knows
//! whether to keep prompting or end the conversation.

use std::collections::HashMap;

use crate::tlk_control_codes::*;
use crate::tlk_runner::{TlkRunEvent, TlkRunInputs, TlkRunOutput, TlkRunStop, run_tlk_stream_from};

/// Phase the conversation is currently in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConversationSessionPhase {
    /// Conversation just opened; greeting / description need to be
    /// presented before any keyword input is accepted.
    #[default]
    Opened,
    /// Greeting was presented; keyword input loop is active.
    AwaitingKeyword,
    /// A TLK `0x84` ASK-PARTY-NAME prompt is waiting for a free-text
    /// party-member name, then resumes the response at `cursor`.
    AwaitingAskPartyName { field_idx: usize, cursor: usize },
    /// A TLK `0x88` ASK-WHO prompt is waiting for a free-text party-member
    /// name, then resumes the response at `cursor`.
    AwaitingAskWho { field_idx: usize, cursor: usize },
    /// A TLK `0x85` GOLD-PAYMENT prompt is waiting for a yes/no answer,
    /// then resumes the response from the payment opcode at `cursor`.
    AwaitingGoldPayment {
        field_idx: usize,
        cursor: usize,
        amount: u16,
    },
    /// NPC's Bye response is being presented; the session is about
    /// to close. The harness flushes the response and then calls
    /// `acknowledge_close`.
    PresentingBye,
    /// Conversation finished; the session may be dropped.
    Closed,
}

/// The session's per-NPC stable input view.
pub struct ConversationContext<'a> {
    /// Avatar's display name (substituted for `0x81`).
    pub avatar_name: &'a str,
    /// Per-scene branch-flag slot (consulted by `0x8C`).
    pub branch_flags: u32,
    /// Party moral standing (consulted by `0xFE` IF-ELSE-ALT).
    pub moral_standing: u8,
    /// Common-word dictionary (128 slots); `None` is acceptable.
    pub dictionary: Option<&'a [&'a str; COMMON_WORD_DICTIONARY_ENTRIES]>,
    /// Whether the player accepts conversation gold-payment prompts.
    pub gold_payment_accepted: bool,
    /// Party gold available for the current prompt, used to refuse
    /// unaffordable payments.
    pub gold_available: Option<u16>,
    /// Live party-member names, already trimmed of trailing NUL padding.
    /// ASK-PARTY-NAME and ASK-WHO compare the next typed line against
    /// this list and pass the matched 1-based slot to the TLK runner.
    pub party_member_names: &'a [&'a [u8]],
}

/// Accepted or refused gold payment emitted by a TLK response stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversationGoldPayment {
    pub amount: u16,
    pub accepted: bool,
}

/// Output of stepping the session.
#[derive(Clone, Debug, Default)]
pub struct ConversationSessionOutput {
    /// Rendered text from the most-recent response stream.
    pub text: String,
    /// New branch-flag bits the response set via `0x87`.
    pub branch_flags_set: u32,
    /// Action grants encountered in the response.
    pub action_grants: Vec<TlkActionDispatchVerb>,
    /// Gold payments encountered in the response.
    pub gold_payments: Vec<ConversationGoldPayment>,
    /// Signal-flag bits encountered in the response (`0x86` arg <`A`).
    pub signal_flags: Vec<u8>,
    /// Matched 1-based slot from an answered ASK-PARTY-NAME prompt.
    pub asked_party_name: Option<u8>,
    /// Matched 1-based slot from an answered ASK-WHO prompt.
    pub asked_who: Option<u8>,
    /// `true` when this step ended the conversation (Bye fired or the
    /// stream ended unrecoverably).
    pub ended: bool,
}

/// Holder for a conversation in progress.
#[derive(Clone, Debug)]
pub struct ConversationSession {
    pub phase: ConversationSessionPhase,
    /// Raw blob fields for the active NPC. Index 0 = Name,
    /// 1 = Description, 2 = Greeting, 3 = Job, 4 = Bye, 5+ = keyword
    /// pairs (alternating keyword bytes / response bytes).
    pub fields: Vec<Vec<u8>>,
    /// Decoded plain-text fields (used for keyword name matching;
    /// the byte-runner consumes the raw fields directly).
    pub decoded_fields: Vec<String>,
    /// Number of keyword lines processed so far (telemetry / UI).
    pub keyword_turns: u32,
}

impl ConversationSession {
    /// Create a session for the supplied NPC blob.
    pub fn new(fields: Vec<Vec<u8>>, decoded_fields: Vec<String>) -> Self {
        Self {
            phase: ConversationSessionPhase::Opened,
            fields,
            decoded_fields,
            keyword_turns: 0,
        }
    }

    /// Run the greeting through the byte runner. Caller should display
    /// the rendered text and then transition to keyword input.
    pub fn present_greeting(&mut self, ctx: &ConversationContext<'_>) -> ConversationSessionOutput {
        self.phase = ConversationSessionPhase::AwaitingKeyword;
        self.run_field_from(2, 0, ctx, 0, 0)
    }

    /// Feed one typed keyword line. Returns the rendered response.
    pub fn submit_keyword(
        &mut self,
        line: &str,
        ctx: &ConversationContext<'_>,
    ) -> ConversationSessionOutput {
        match self.phase {
            ConversationSessionPhase::AwaitingKeyword => {}
            ConversationSessionPhase::AwaitingAskPartyName { field_idx, cursor } => {
                let slot = tlk_ask_party_name_match(line.trim().as_bytes(), ctx.party_member_names);
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                let mut out = self.run_field_from(field_idx, cursor, ctx, slot, 0);
                out.asked_party_name = Some(slot);
                return out;
            }
            ConversationSessionPhase::AwaitingAskWho { field_idx, cursor } => {
                let slot = tlk_ask_party_name_match(line.trim().as_bytes(), ctx.party_member_names);
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                let mut out = self.run_field_from(field_idx, cursor, ctx, 0, slot);
                out.asked_who = Some(slot);
                return out;
            }
            ConversationSessionPhase::AwaitingGoldPayment {
                field_idx,
                cursor,
                amount: _,
            } => {
                let Some(accepted) = conversation_payment_answer(line) else {
                    return ConversationSessionOutput::default();
                };
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                return self
                    .run_field_from_with_options(field_idx, cursor, ctx, 0, 0, accepted, false);
            }
            _ => return ConversationSessionOutput::default(),
        }
        self.keyword_turns = self.keyword_turns.saturating_add(1);
        let input = line.trim().as_bytes();
        let input_upper: Vec<u8> = input
            .iter()
            .map(|b| (*b & 0x7F).to_ascii_uppercase())
            .collect();
        let kind = tlk_player_input_kind(&input_upper);
        let field_idx = match kind {
            TlkPlayerInputKind::EmptyByeShortcut => 4usize,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry) => 0,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry) => 3,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath) => 4,
            TlkPlayerInputKind::OrdinaryKeywordScan => self
                .find_ordinary_keyword_response_index(&input_upper)
                .unwrap_or(usize::MAX),
        };
        let mut out = ConversationSessionOutput::default();
        if field_idx == usize::MAX {
            out.text = TLK_NO_KEYWORD_MATCH_MESSAGE.to_string();
            return out;
        }
        if matches!(kind, TlkPlayerInputKind::EmptyByeShortcut) {
            out.text.push_str(TLK_EMPTY_INPUT_BYE_MESSAGE);
        }
        let response = self.run_field_from(field_idx, 0, ctx, 0, 0);
        out.text.push_str(&response.text);
        out.branch_flags_set |= response.branch_flags_set;
        out.action_grants.extend(response.action_grants);
        out.gold_payments.extend(response.gold_payments);
        out.signal_flags.extend(response.signal_flags);
        out.asked_party_name = response.asked_party_name;
        out.asked_who = response.asked_who;
        out.ended |= response.ended;
        // Empty input or BYE/THANK closes the conversation.
        if matches!(
            kind,
            TlkPlayerInputKind::EmptyByeShortcut
                | TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath),
        ) {
            self.phase = ConversationSessionPhase::PresentingBye;
            out.ended = true;
        }
        out
    }

    /// Current prompt text for the active outer input loop.
    pub fn prompt_message(&self) -> String {
        match self.phase {
            ConversationSessionPhase::AwaitingKeyword => "Your interest?".to_string(),
            ConversationSessionPhase::AwaitingAskPartyName { .. } => "Name?".to_string(),
            ConversationSessionPhase::AwaitingAskWho { .. } => "Who?".to_string(),
            ConversationSessionPhase::AwaitingGoldPayment { amount, .. } => {
                format!("Pay {amount} gold? (Y/N)")
            }
            ConversationSessionPhase::Opened => "Your interest?".to_string(),
            ConversationSessionPhase::PresentingBye | ConversationSessionPhase::Closed => {
                String::new()
            }
        }
    }

    /// Caller has finished presenting the Bye response; close the
    /// session.
    pub fn acknowledge_close(&mut self) {
        self.phase = ConversationSessionPhase::Closed;
    }

    /// Returns `true` when the session is closed and may be dropped.
    pub fn is_closed(&self) -> bool {
        matches!(self.phase, ConversationSessionPhase::Closed)
    }

    fn find_ordinary_keyword_response_index(&self, input_upper: &[u8]) -> Option<usize> {
        // Pairs start at field index 5: (keyword, response, keyword,
        // response, ...). Scan keyword positions; the response is the
        // next index.
        let mut idx = 5usize;
        while idx + 1 < self.decoded_fields.len() {
            let keyword_field = &self.decoded_fields[idx];
            let keyword = keyword_field.trim().as_bytes();
            if !keyword.is_empty() && tlk_keyword_matches(keyword, input_upper) {
                return Some(idx + 1);
            }
            idx += 2;
        }
        None
    }

    fn run_field_from(
        &mut self,
        field_idx: usize,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_party_name_response: u8,
        ask_who_response: u8,
    ) -> ConversationSessionOutput {
        self.run_field_from_with_options(
            field_idx,
            start,
            ctx,
            ask_party_name_response,
            ask_who_response,
            ctx.gold_payment_accepted,
            true,
        )
    }

    fn run_field_from_with_options(
        &mut self,
        field_idx: usize,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_party_name_response: u8,
        ask_who_response: u8,
        gold_payment_accepted: bool,
        yield_on_gold_payment: bool,
    ) -> ConversationSessionOutput {
        let mut out = ConversationSessionOutput::default();
        let Some(run) = self.fields.get(field_idx).map(|bytes| {
            let inputs = make_inputs(
                ctx,
                ask_party_name_response,
                ask_who_response,
                gold_payment_accepted,
                yield_on_gold_payment,
            );
            run_tlk_stream_from(bytes, start, &inputs)
        }) else {
            return out;
        };
        self.absorb_run(field_idx, &run, &mut out);
        out
    }

    fn absorb_run(
        &mut self,
        field_idx: usize,
        run: &TlkRunOutput,
        out: &mut ConversationSessionOutput,
    ) {
        out.text.push_str(&run.text);
        out.branch_flags_set |= run.branch_flags_set;
        out.action_grants.extend(run.action_grants.iter().copied());
        out.gold_payments
            .extend(run.events.iter().filter_map(|event| match event {
                TlkRunEvent::GoldPayment { amount, accepted } => Some(ConversationGoldPayment {
                    amount: *amount,
                    accepted: *accepted,
                }),
                _ => None,
            }));
        out.signal_flags.extend(run.signal_flags.iter().copied());
        match run.stop {
            TlkRunStop::AskingPartyName(cursor) => {
                self.phase = ConversationSessionPhase::AwaitingAskPartyName { field_idx, cursor };
            }
            TlkRunStop::AskingWho(cursor) => {
                self.phase = ConversationSessionPhase::AwaitingAskWho { field_idx, cursor };
            }
            TlkRunStop::AskingGoldPayment { cursor, amount } => {
                self.phase = ConversationSessionPhase::AwaitingGoldPayment {
                    field_idx,
                    cursor,
                    amount,
                };
            }
            TlkRunStop::EndOfStream | TlkRunStop::NulTerminator => {
                // End-of-stream forces a hard close; the keyword loop must
                // not continue prompting after that.
                self.phase = ConversationSessionPhase::PresentingBye;
                out.ended = true;
            }
            _ => {}
        }
    }
}

fn make_inputs<'a>(
    ctx: &'a ConversationContext<'a>,
    ask_party_name_response: u8,
    ask_who_response: u8,
    gold_payment_accepted: bool,
    yield_on_gold_payment: bool,
) -> TlkRunInputs<'a> {
    TlkRunInputs {
        avatar_name: ctx.avatar_name,
        branch_flags: ctx.branch_flags,
        moral_standing: ctx.moral_standing,
        dictionary: ctx.dictionary,
        curse_seen: false,
        gold_payment_accepted,
        gold_available: ctx.gold_available,
        ask_party_name_response,
        ask_who_response,
        yield_on_pause: false,
        yield_on_ask: true,
        yield_on_gold_payment,
    }
}

fn conversation_payment_answer(line: &str) -> Option<bool> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return Some(false);
    }
    match trimmed.bytes().next()?.to_ascii_uppercase() {
        b'Y' => Some(true),
        b'N' | b' ' => Some(false),
        _ => None,
    }
}

/// Lookup-style helper: convert a `HashMap<u16, Vec<Vec<u8>>>` raw
/// blob and a `HashMap<u16, Vec<String>>` decoded blob into the two
/// vectors a [`ConversationSession`] needs.
pub fn fields_for_npc(
    raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
    decoded: &HashMap<u16, Vec<String>>,
    npc_id: u16,
) -> Option<(Vec<Vec<u8>>, Vec<String>)> {
    let raw = raw_blob.get(&npc_id)?.clone();
    let decoded = decoded.get(&npc_id)?.clone();
    Some((raw, decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(text: &str) -> Vec<u8> {
        text.bytes().map(|b| b ^ TLK_TEXT_XOR_MASK).collect()
    }

    fn ctx() -> ConversationContext<'static> {
        ConversationContext {
            avatar_name: "Cal",
            branch_flags: 0,
            moral_standing: 0,
            dictionary: None,
            gold_payment_accepted: false,
            gold_available: None,
            party_member_names: &[],
        }
    }

    fn baseline_session() -> ConversationSession {
        // Fields: Name, Description, Greeting, Job, Bye, K1, R1, K2, R2
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings, traveller."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("GRAN"),
            enc("Short answer."),
            enc("GRANDPA"),
            enc("Long answer."),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings, traveller.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "GRAN".to_string(),
            "Short answer.".to_string(),
            "GRANDPA".to_string(),
            "Long answer.".to_string(),
        ];
        ConversationSession::new(raw, decoded)
    }

    #[test]
    fn present_greeting_renders_field_2_and_transitions_to_keyword_loop() {
        let mut s = baseline_session();
        let out = s.present_greeting(&ctx());
        assert!(out.text.contains("Greetings"));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn submit_name_uses_field_0() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("name", &ctx());
        assert!(out.text.contains("Ada"));
    }

    #[test]
    fn submit_job_uses_field_3() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("job", &ctx());
        assert!(out.text.contains("mend"));
    }

    #[test]
    fn ask_party_name_prompt_matches_next_line_then_resumes_response() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("JOIN"),
            {
                let mut bytes = enc("Name thy companion.");
                bytes.push(TLK_CODE_ASK_PARTY_NAME);
                bytes.extend_from_slice(&enc(" Done."));
                bytes.push(TLK_CODE_END_OF_RESPONSE);
                bytes
            },
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "JOIN".to_string(),
            "Name thy companion.".to_string(),
        ];
        let party_names: [&[u8]; 2] = [b"AVATAR", b"IOLO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);

        let first = s.submit_keyword("join", &context);
        assert_eq!(first.text, "Name thy companion.");
        assert!(matches!(
            s.phase,
            ConversationSessionPhase::AwaitingAskPartyName { .. }
        ));
        assert_eq!(s.prompt_message(), "Name?");

        let second = s.submit_keyword("iolo", &context);
        assert_eq!(second.asked_party_name, Some(2));
        assert_eq!(second.text, " Done.");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn gold_payment_prompt_waits_for_answer_then_resumes_branch() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("PAY"),
            {
                let mut bytes = enc("A toll.");
                bytes.extend_from_slice(&[TLK_CODE_GOLD_PAYMENT, b'0', b'2', b'5']);
                bytes.push(TLK_CODE_GOTO_LABEL_FIRST);
                bytes.extend_from_slice(&enc(" Paid."));
                bytes.push(TLK_CODE_END_OF_RESPONSE);
                bytes.push(TLK_CODE_GOTO_LABEL_LAST);
                bytes.extend_from_slice(&enc(" Refused."));
                bytes.push(TLK_CODE_END_OF_RESPONSE);
                bytes
            },
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "PAY".to_string(),
            "A toll.".to_string(),
        ];
        let context = ConversationContext {
            gold_payment_accepted: true,
            gold_available: Some(30),
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);

        let prompt = s.submit_keyword("pay", &context);
        assert_eq!(prompt.text, "A toll.");
        assert_eq!(s.prompt_message(), "Pay 25 gold? (Y/N)");
        assert!(matches!(
            s.phase,
            ConversationSessionPhase::AwaitingGoldPayment { amount: 25, .. }
        ));

        let paid = s.submit_keyword("y", &context);
        assert_eq!(paid.text, " Paid.");
        assert_eq!(
            paid.gold_payments,
            vec![ConversationGoldPayment {
                amount: 25,
                accepted: true
            }]
        );
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn gold_payment_empty_answer_declines_and_takes_refusal_branch() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("PAY"),
            {
                let mut bytes = vec![TLK_CODE_GOLD_PAYMENT, b'0', b'2', b'5'];
                bytes.push(TLK_CODE_GOTO_LABEL_FIRST);
                bytes.extend_from_slice(&enc(" Paid."));
                bytes.push(TLK_CODE_END_OF_RESPONSE);
                bytes.push(TLK_CODE_GOTO_LABEL_LAST);
                bytes.extend_from_slice(&enc(" Refused."));
                bytes.push(TLK_CODE_END_OF_RESPONSE);
                bytes
            },
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "PAY".to_string(),
            String::new(),
        ];
        let context = ConversationContext {
            gold_payment_accepted: true,
            gold_available: Some(30),
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);
        s.submit_keyword("pay", &context);

        let declined = s.submit_keyword("", &context);
        assert_eq!(declined.text, " Refused.");
        assert_eq!(
            declined.gold_payments,
            vec![ConversationGoldPayment {
                amount: 25,
                accepted: false
            }]
        );
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn submit_work_alias_uses_job_field() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("work", &ctx());
        assert!(out.text.contains("mend"));
    }

    #[test]
    fn submit_bye_uses_field_4_and_ends_session() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("bye", &ctx());
        assert!(out.text.contains("Farewell"));
        assert!(out.ended);
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
    }

    #[test]
    fn empty_input_ends_session_via_bye_shortcut() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("", &ctx());
        assert!(out.text.starts_with(TLK_EMPTY_INPUT_BYE_MESSAGE));
        assert!(out.text.contains("Farewell"));
        assert!(out.ended);
    }

    #[test]
    fn ordinary_keyword_returns_matched_response() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("grandpa", &ctx());
        assert!(out.text.contains("Long"));
    }

    #[test]
    fn ordinary_keyword_short_match_picks_first_match() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        // "GRAN" is a prefix-keyword; "gran news" should match it
        // before "grandpa" (which never matches because of the space
        // boundary).
        let out = s.submit_keyword("gran news", &ctx());
        assert!(out.text.contains("Short"));
    }

    #[test]
    fn no_match_keyword_returns_polite_refusal() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("xyzzy", &ctx());
        assert_eq!(out.text, TLK_NO_KEYWORD_MATCH_MESSAGE);
    }

    #[test]
    fn keyword_loop_increments_turn_counter() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        s.submit_keyword("job", &ctx());
        s.submit_keyword("name", &ctx());
        assert_eq!(s.keyword_turns, 2);
    }

    #[test]
    fn acknowledge_close_transitions_to_closed_phase() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        s.submit_keyword("bye", &ctx());
        s.acknowledge_close();
        assert!(s.is_closed());
    }

    #[test]
    fn submit_keyword_before_greeting_is_a_noop() {
        let mut s = baseline_session();
        let out = s.submit_keyword("job", &ctx());
        assert!(out.text.is_empty());
    }
}
