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
use crate::tlk_runner::{run_tlk_stream, TlkRunEvent, TlkRunInputs, TlkRunOutput, TlkRunStop};

/// Phase the conversation is currently in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConversationSessionPhase {
    /// Conversation just opened; greeting / description need to be
    /// presented before any keyword input is accepted.
    #[default]
    Opened,
    /// Greeting was presented; keyword input loop is active.
    AwaitingKeyword,
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
        let mut out = ConversationSessionOutput::default();
        if let Some(bytes) = self.fields.get(2) {
            let inputs = make_inputs(ctx, 0, 0);
            let run = run_tlk_stream(bytes, &inputs);
            self.absorb_run(&run, &mut out);
        }
        self.phase = ConversationSessionPhase::AwaitingKeyword;
        out
    }

    /// Feed one typed keyword line. Returns the rendered response.
    pub fn submit_keyword(
        &mut self,
        line: &str,
        ctx: &ConversationContext<'_>,
    ) -> ConversationSessionOutput {
        if !matches!(self.phase, ConversationSessionPhase::AwaitingKeyword) {
            return ConversationSessionOutput::default();
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
        if let Some(bytes) = self.fields.get(field_idx) {
            let inputs = make_inputs(ctx, 0, 0);
            let run = run_tlk_stream(bytes, &inputs);
            self.absorb_run(&run, &mut out);
        }
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

    fn absorb_run(&mut self, run: &TlkRunOutput, out: &mut ConversationSessionOutput) {
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
        if matches!(
            run.stop,
            TlkRunStop::EndOfStream | TlkRunStop::NulTerminator
        ) {
            // End-of-stream forces a hard close; the keyword loop must
            // not continue prompting after that.
            self.phase = ConversationSessionPhase::PresentingBye;
            out.ended = true;
        }
    }
}

fn make_inputs<'a>(
    ctx: &'a ConversationContext<'a>,
    ask_party_name_response: u8,
    ask_who_response: u8,
) -> TlkRunInputs<'a> {
    TlkRunInputs {
        avatar_name: ctx.avatar_name,
        branch_flags: ctx.branch_flags,
        moral_standing: ctx.moral_standing,
        dictionary: ctx.dictionary,
        curse_seen: false,
        gold_payment_accepted: ctx.gold_payment_accepted,
        gold_available: ctx.gold_available,
        ask_party_name_response,
        ask_who_response,
        yield_on_pause: false,
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
