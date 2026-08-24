//! Interactive conversation session state machine per
//! `systems/conversation.md` §6.
//!
//! The session wraps the per-stream byte-runner in a stateful loop:
//! the player presses Talk, sees the greeting, and then the engine
//! enters a keyword-input prompt that keeps reading lines until the
//! NPC says Bye. Each typed keyword resolves to one of:
//!
//! - the empty Bye shortcut,
//! - one of the reserved functional words (NAME/JOB/WORK/BYE/THANK),
//! - one of the reserved rebuke words,
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
use crate::tlk_runner::{
    TlkRenderedGlyph, TlkRenderedText, TlkRunEvent, TlkRunInputs, TlkRunOutput, TlkRunStop,
    run_tlk_stream_from,
};

/// Phase the conversation is currently in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConversationSessionPhase {
    /// Conversation just opened; greeting / description need to be
    /// presented before any keyword input is accepted.
    #[default]
    Opened,
    /// Greeting was presented; keyword input loop is active.
    AwaitingKeyword,
    /// A labelled block opened a scoped `Your interest?` prompt. Top-level
    /// reserved responses are suppressed while this phase is active.
    AwaitingScopedKeyword { label: u8 },
    /// A TLK `0x84` ASK-PARTY-NAME prompt is waiting for a free-text
    /// party-member name, then resumes the response at `cursor`.
    AwaitingAskPartyName { field_idx: usize, cursor: usize },
    /// A TLK `0x88` ASK-WHO prompt is waiting for a free-text party-member
    /// name, then resumes the response at `cursor`.
    AwaitingAskWho { field_idx: usize, cursor: usize },
    /// `conversation.md §7.6`: an unaffordable `0x85` demand has stopped
    /// its response and entered the routine's nested ordinary keyword loop.
    /// Nonterminating turns reprompt inside the loop; every path that returns
    /// from it ends the enclosing conversation.
    AwaitingGoldRefusalKeyword,
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
    /// Party gold available to the current response. `0x85` itself does
    /// not ask for confirmation: the surrounding authored answer record
    /// already represents consent, and affordability alone decides the
    /// control's accepted/refused result.
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
    /// Per-cell ordinary/runic font selection aligned with `text`.
    pub rendered_glyphs: Vec<TlkRenderedGlyph>,
    /// New branch-flag bits the response set.
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
    /// The selected response returned the byte-runner's explicit stop
    /// result (`0xFF`). Ordinary top-level responses still reprompt, but
    /// the nested gold-refusal loop propagates this stop through the whole
    /// conversation without synthesising a Bye line.
    pub response_signalled_stop: bool,
    /// Shared moral-standing selector after any `0x89` / `0x8A` writes in
    /// this step's streams, or `None` when none ran. `conversation.md
    /// §7.4` makes those two codes the byte runner's only direct writers
    /// of the selector; the caller assigns this value rather than
    /// re-deriving a delta.
    pub moral_standing: Option<u8>,
}

impl ConversationSessionOutput {
    pub fn rendered_text(&self) -> TlkRenderedText {
        TlkRenderedText {
            text: self.text.clone(),
            glyphs: self.rendered_glyphs.clone(),
        }
    }

    fn set_plain_text(&mut self, text: &str) {
        self.text.clear();
        self.rendered_glyphs.clear();
        self.push_plain_text(text);
    }

    fn push_plain_text(&mut self, text: &str) {
        self.text.push_str(text);
        self.rendered_glyphs
            .extend(text.bytes().map(TlkRenderedGlyph::ordinary));
    }
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
    /// Live shared moral-standing selector once a stream in this session
    /// has written it with `0x89` / `0x8A`. A single step can run several
    /// streams (a `0x87` follow-up scan, a scoped-prompt record), and the
    /// caller's [`ConversationContext`] snapshot is fixed for the whole
    /// step, so the second stream would otherwise re-read a stale value
    /// and lose the first stream's write.
    moral_standing: Option<u8>,
}

struct KeywordMatch {
    response_idx: usize,
    remainder: Vec<u8>,
}

impl ConversationSession {
    /// Create a session for the supplied NPC blob.
    pub fn new(fields: Vec<Vec<u8>>, decoded_fields: Vec<String>) -> Self {
        Self {
            phase: ConversationSessionPhase::Opened,
            fields,
            decoded_fields,
            keyword_turns: 0,
            moral_standing: None,
        }
    }

    /// Run the greeting through the byte runner. Caller should display
    /// the rendered text and then transition to keyword input.
    pub fn present_greeting(&mut self, ctx: &ConversationContext<'_>) -> ConversationSessionOutput {
        self.phase = ConversationSessionPhase::AwaitingKeyword;
        self.run_field_from(2, 0, ctx, 0, 0)
    }

    /// `conversation.md §9`: a stranger either says nothing after the
    /// description or introduces itself from the Name entry. The caller owns
    /// the host-clock reseed and fair coin because those mutate PlayState PRNG.
    pub fn present_stranger_opening(
        &mut self,
        ctx: &ConversationContext<'_>,
        introduces_itself: bool,
    ) -> ConversationSessionOutput {
        self.phase = ConversationSessionPhase::AwaitingKeyword;
        if !introduces_itself {
            return ConversationSessionOutput::default();
        }
        let mut output = self.run_field_from(0, 0, ctx, 0, 0);
        let mut rendered = TlkRenderedText::plain("I am called ");
        rendered.push_rendered(&output.rendered_text());
        output.text = rendered.text;
        output.rendered_glyphs = rendered.glyphs;
        output
    }

    /// Feed one typed keyword line. Returns the rendered response.
    pub fn submit_keyword(
        &mut self,
        line: &str,
        ctx: &ConversationContext<'_>,
    ) -> ConversationSessionOutput {
        match self.phase {
            ConversationSessionPhase::AwaitingKeyword => {}
            ConversationSessionPhase::AwaitingScopedKeyword { label } => {
                return self.submit_scoped_keyword(label, line, ctx);
            }
            ConversationSessionPhase::AwaitingAskPartyName { field_idx, cursor } => {
                let input = capped_tlk_input_bytes(line);
                let slot = tlk_ask_party_name_match(&input, ctx.party_member_names);
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                let mut out = self.run_field_from(field_idx, cursor, ctx, slot, 0);
                out.asked_party_name = Some(slot);
                return out;
            }
            ConversationSessionPhase::AwaitingAskWho { field_idx, cursor } => {
                let input = capped_tlk_input_bytes(line);
                let slot = tlk_ask_party_name_match(&input, ctx.party_member_names);
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                let mut out = self.run_field_from(field_idx, cursor, ctx, 0, slot);
                out.asked_who = Some(slot);
                return out;
            }
            ConversationSessionPhase::AwaitingGoldRefusalKeyword => {
                // §7.6: refusal calls the ordinary keyword loop as a nested
                // prompt. Nonterminating turns remain nested; an explicit
                // response stop or the mandatory Bye path unwinds both loops.
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                let mut out = self.submit_keyword(line, ctx);
                if out.ended || out.response_signalled_stop {
                    self.phase = ConversationSessionPhase::PresentingBye;
                    out.ended = true;
                } else {
                    self.phase = ConversationSessionPhase::AwaitingGoldRefusalKeyword;
                }
                return out;
            }
            _ => return ConversationSessionOutput::default(),
        }
        self.keyword_turns = self.keyword_turns.saturating_add(1);
        let input = capped_tlk_input_bytes(line);
        let input_upper: Vec<u8> = input
            .iter()
            .map(|b| (*b & 0x7F).to_ascii_uppercase())
            .collect();
        let kind = tlk_player_input_kind(&input_upper);
        let mut out = ConversationSessionOutput::default();
        if matches!(kind, TlkPlayerInputKind::ReservedRebuke { .. }) {
            out.set_plain_text(TLK_RESERVED_REBUKE_MESSAGE);
            return out;
        }
        let field_idx = match kind {
            TlkPlayerInputKind::EmptyByeShortcut => 4usize,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry) => 0,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry) => 3,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath) => 4,
            TlkPlayerInputKind::ReservedRebuke { .. } => unreachable!(),
            TlkPlayerInputKind::OrdinaryKeywordScan => self
                .find_ordinary_keyword_match_from(&input_upper, TLK_LEADING_ENTRY_COUNT)
                .map(|matched| matched.response_idx)
                .unwrap_or(usize::MAX),
        };
        if field_idx == usize::MAX {
            out.set_plain_text(TLK_NO_KEYWORD_MATCH_MESSAGE);
            return out;
        }
        if matches!(
            kind,
            TlkPlayerInputKind::EmptyByeShortcut
                | TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath),
        ) {
            out.push_plain_text(TLK_EMPTY_INPUT_BYE_MESSAGE);
        }
        let followup_input = match kind {
            TlkPlayerInputKind::OrdinaryKeywordScan => self
                .find_ordinary_keyword_match_from(&input_upper, TLK_LEADING_ENTRY_COUNT)
                .map(|matched| matched.remainder)
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let response = self.run_field_from_for_input(field_idx, 0, ctx, 0, 0, &followup_input);
        out.text.push_str(&response.text);
        out.rendered_glyphs.extend(response.rendered_glyphs);
        out.branch_flags_set |= response.branch_flags_set;
        out.action_grants.extend(response.action_grants);
        out.gold_payments.extend(response.gold_payments);
        out.signal_flags.extend(response.signal_flags);
        out.asked_party_name = response.asked_party_name;
        out.asked_who = response.asked_who;
        out.moral_standing = response.moral_standing.or(out.moral_standing);
        out.response_signalled_stop |= response.response_signalled_stop;
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
            ConversationSessionPhase::AwaitingKeyword => TLK_KEYWORD_PROMPT.to_string(),
            ConversationSessionPhase::AwaitingScopedKeyword { .. } => {
                TLK_KEYWORD_PROMPT.to_string()
            }
            ConversationSessionPhase::AwaitingAskPartyName { .. } => "Name?".to_string(),
            ConversationSessionPhase::AwaitingAskWho { .. } => "Who?".to_string(),
            ConversationSessionPhase::AwaitingGoldRefusalKeyword => TLK_KEYWORD_PROMPT.to_string(),
            ConversationSessionPhase::Opened => TLK_KEYWORD_PROMPT.to_string(),
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

    pub fn npc_name(&self) -> Option<String> {
        self.decoded_fields
            .first()
            .map(String::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or_else(|| {
                let raw = self.fields.first()?;
                let text: String = raw
                    .iter()
                    .take_while(|byte| **byte != 0)
                    .map(|byte| (byte & 0x7F) as char)
                    .collect();
                let text = text.trim().to_string();
                (!text.is_empty()).then_some(text)
            })
    }

    pub fn awaiting_ask_party_name(&self) -> bool {
        matches!(
            self.phase,
            ConversationSessionPhase::AwaitingAskPartyName { .. }
        )
    }

    fn find_ordinary_keyword_response_index(&self, input_upper: &[u8]) -> Option<usize> {
        self.find_ordinary_keyword_match_from(input_upper, TLK_LEADING_ENTRY_COUNT)
            .map(|matched| matched.response_idx)
    }

    fn find_ordinary_keyword_match_from(
        &self,
        input_upper: &[u8],
        start_idx: usize,
    ) -> Option<KeywordMatch> {
        // Pairs start at field index 5: (keyword, response, keyword,
        // response, ...). Scan keyword positions; the response is the
        // next index.
        let mut idx = start_idx.max(TLK_LEADING_ENTRY_COUNT);
        if idx % 2 == 0 {
            idx += 1;
        }
        while idx + 1 < self.decoded_fields.len() {
            let keyword_field = &self.decoded_fields[idx];
            let keyword = keyword_field.trim().as_bytes();
            if let Some(remainder) = tlk_keyword_match_remainder(keyword, input_upper) {
                return Some(KeywordMatch {
                    response_idx: idx + 1,
                    remainder,
                });
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
        self.run_field_from_for_input(
            field_idx,
            start,
            ctx,
            ask_party_name_response,
            ask_who_response,
            &[],
        )
    }

    fn run_field_from_for_input(
        &mut self,
        field_idx: usize,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_party_name_response: u8,
        ask_who_response: u8,
        followup_input_upper: &[u8],
    ) -> ConversationSessionOutput {
        self.run_field_from_with_options(
            field_idx,
            start,
            ctx,
            ask_party_name_response,
            ask_who_response,
            followup_input_upper,
        )
    }

    fn run_field_from_with_options(
        &mut self,
        field_idx: usize,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_party_name_response: u8,
        ask_who_response: u8,
        followup_input_upper: &[u8],
    ) -> ConversationSessionOutput {
        let mut out = ConversationSessionOutput::default();
        let mut cursor = start;
        while let Some(run) = self.fields.get(field_idx).map(|bytes| {
            let inputs = make_inputs(
                ctx,
                self.moral_standing.unwrap_or(ctx.moral_standing),
                ask_party_name_response,
                ask_who_response,
            );
            run_tlk_stream_from(bytes, cursor, &inputs)
        }) {
            self.absorb_run(field_idx, &run, &mut out);
            let TlkRunStop::FollowUpKeywordScan(next_cursor) = run.stop else {
                break;
            };

            if let Some(matched) =
                self.find_ordinary_keyword_match_from(followup_input_upper, field_idx + 1)
            {
                let nested = self.run_field_from_for_input(
                    matched.response_idx,
                    0,
                    ctx,
                    0,
                    0,
                    &matched.remainder,
                );
                merge_session_output(&mut out, nested);
                if !matches!(self.phase, ConversationSessionPhase::AwaitingKeyword) {
                    break;
                }
            }

            cursor = next_cursor;
        }
        out
    }

    fn absorb_run(
        &mut self,
        field_idx: usize,
        run: &TlkRunOutput,
        out: &mut ConversationSessionOutput,
    ) {
        out.text.push_str(&run.text);
        out.rendered_glyphs.extend_from_slice(&run.rendered_glyphs);
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
        out.response_signalled_stop |= matches!(run.stop, TlkRunStop::EndOfResponse);
        self.absorb_moral_standing(run, out);
        match run.stop {
            TlkRunStop::AskingPartyName(cursor) => {
                self.phase = ConversationSessionPhase::AwaitingAskPartyName { field_idx, cursor };
            }
            TlkRunStop::AskingWho(cursor) => {
                self.phase = ConversationSessionPhase::AwaitingAskWho { field_idx, cursor };
            }
            TlkRunStop::GoldPaymentRefused { .. } => {
                self.phase = ConversationSessionPhase::AwaitingGoldRefusalKeyword;
            }
            TlkRunStop::LabelTransfer(label) => {
                if self.has_scoped_label_records(label) {
                    self.phase = ConversationSessionPhase::AwaitingScopedKeyword { label };
                }
            }
            TlkRunStop::EndOfStream | TlkRunStop::NulTerminator => {}
            _ => {}
        }
    }

    fn submit_scoped_keyword(
        &mut self,
        label: u8,
        line: &str,
        ctx: &ConversationContext<'_>,
    ) -> ConversationSessionOutput {
        self.keyword_turns = self.keyword_turns.saturating_add(1);
        let input = capped_tlk_input_bytes(line);
        let input_upper: Vec<u8> = input
            .iter()
            .map(|b| (*b & 0x7F).to_ascii_uppercase())
            .collect();

        if input_upper.is_empty() {
            let mut out = ConversationSessionOutput::default();
            out.set_plain_text(TLK_EMPTY_INPUT_BYE_MESSAGE);
            self.phase = ConversationSessionPhase::AwaitingScopedKeyword { label };
            return out;
        }

        if let Some(response) = self.find_scoped_label_response(label, &input_upper) {
            self.phase = ConversationSessionPhase::AwaitingKeyword;
            return self.run_ephemeral_stream(&response, ctx, 0, 0);
        }

        self.phase = ConversationSessionPhase::AwaitingKeyword;
        if let Some(field_idx) = self.find_ordinary_keyword_response_index(&input_upper) {
            return self.run_field_from(field_idx, 0, ctx, 0, 0);
        }

        let mut out = ConversationSessionOutput::default();
        out.set_plain_text(TLK_NO_KEYWORD_MATCH_MESSAGE);
        out
    }

    fn has_scoped_label_records(&self, label: u8) -> bool {
        self.fields
            .iter()
            .any(|bytes| find_next_label_record(bytes, label, 0).is_some())
    }

    fn find_scoped_label_response(&self, label: u8, input_upper: &[u8]) -> Option<Vec<u8>> {
        for bytes in &self.fields {
            let mut search_from = 0usize;
            while let Some((record_start, record_end)) =
                find_next_label_record(bytes, label, search_from)
            {
                if let Some(response) = find_scoped_response_in_record(
                    &bytes[record_start..record_end],
                    label,
                    input_upper,
                ) {
                    return Some(response);
                }
                search_from = record_end.saturating_add(1);
            }
        }
        None
    }

    fn run_ephemeral_stream(
        &mut self,
        bytes: &[u8],
        ctx: &ConversationContext<'_>,
        ask_party_name_response: u8,
        ask_who_response: u8,
    ) -> ConversationSessionOutput {
        let inputs = make_inputs(
            ctx,
            self.moral_standing.unwrap_or(ctx.moral_standing),
            ask_party_name_response,
            ask_who_response,
        );
        let run = run_tlk_stream_from(bytes, 0, &inputs);
        let mut out = ConversationSessionOutput::default();
        self.absorb_ephemeral_run(&run, &mut out);
        out
    }

    fn absorb_ephemeral_run(&mut self, run: &TlkRunOutput, out: &mut ConversationSessionOutput) {
        out.text.push_str(&run.text);
        out.rendered_glyphs.extend_from_slice(&run.rendered_glyphs);
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
        out.response_signalled_stop |= matches!(run.stop, TlkRunStop::EndOfResponse);
        self.absorb_moral_standing(run, out);
        match run.stop {
            TlkRunStop::LabelTransfer(label) if self.has_scoped_label_records(label) => {
                self.phase = ConversationSessionPhase::AwaitingScopedKeyword { label };
            }
            TlkRunStop::GoldPaymentRefused { .. } => {
                self.phase = ConversationSessionPhase::AwaitingGoldRefusalKeyword;
            }
            TlkRunStop::EndOfStream | TlkRunStop::NulTerminator => {}
            _ => {}
        }
    }

    /// `conversation.md §7.4`: carry a stream's `0x89` / `0x8A` write of
    /// the shared moral-standing selector into both the session's live
    /// value (so a later stream in the same step reads it) and the step
    /// output (so the caller assigns it to the party record).
    fn absorb_moral_standing(&mut self, run: &TlkRunOutput, out: &mut ConversationSessionOutput) {
        if let Some(standing) = run.moral_standing {
            self.moral_standing = Some(standing);
            out.moral_standing = Some(standing);
        }
    }
}

fn merge_session_output(out: &mut ConversationSessionOutput, nested: ConversationSessionOutput) {
    out.text.push_str(&nested.text);
    out.rendered_glyphs.extend(nested.rendered_glyphs);
    out.branch_flags_set |= nested.branch_flags_set;
    out.action_grants.extend(nested.action_grants);
    out.gold_payments.extend(nested.gold_payments);
    out.signal_flags.extend(nested.signal_flags);
    out.asked_party_name = nested.asked_party_name.or(out.asked_party_name);
    out.asked_who = nested.asked_who.or(out.asked_who);
    out.moral_standing = nested.moral_standing.or(out.moral_standing);
    out.response_signalled_stop |= nested.response_signalled_stop;
    out.ended |= nested.ended;
}

fn tlk_keyword_match_remainder(keyword: &[u8], input_upper: &[u8]) -> Option<Vec<u8>> {
    if !tlk_keyword_matches(keyword, input_upper) {
        return None;
    }
    let mut start = keyword.len().min(input_upper.len());
    while start < input_upper.len() && input_upper[start] == b' ' {
        start += 1;
    }
    Some(input_upper[start..].to_vec())
}

fn find_next_label_record(bytes: &[u8], label: u8, from: usize) -> Option<(usize, usize)> {
    if !is_tlk_label_byte(label) {
        return None;
    }
    let mut pos = from;
    while pos + 1 < bytes.len() {
        if bytes[pos] == TLK_CODE_LABEL_RECORD && bytes[pos + 1] == label {
            let record_start = pos + 2;
            let mut record_end = record_start;
            while record_end < bytes.len() && bytes[record_end] != TLK_CODE_LABEL_RECORD {
                record_end += 1;
            }
            return Some((record_start, record_end));
        }
        pos += 1;
    }
    None
}

fn find_scoped_response_in_record(record: &[u8], label: u8, input_upper: &[u8]) -> Option<Vec<u8>> {
    let mut segments = record
        .split(|byte| *byte == label)
        .filter(|segment| !segment.is_empty());
    while let Some(keyword) = segments.next() {
        let Some(response) = segments.next() else {
            break;
        };
        if tlk_keyword_matches(keyword, input_upper) {
            return Some(response.to_vec());
        }
    }
    None
}

fn make_inputs<'a>(
    ctx: &'a ConversationContext<'a>,
    moral_standing: u8,
    ask_party_name_response: u8,
    ask_who_response: u8,
) -> TlkRunInputs<'a> {
    TlkRunInputs {
        avatar_name: ctx.avatar_name,
        branch_flags: ctx.branch_flags,
        moral_standing,
        dictionary: ctx.dictionary,
        curse_seen: false,
        gold_available: ctx.gold_available,
        ask_party_name_response,
        ask_who_response,
        yield_on_pause: false,
        yield_on_ask: true,
    }
}

fn capped_tlk_input_bytes(line: &str) -> Vec<u8> {
    line.trim()
        .as_bytes()
        .iter()
        .take(TLK_INPUT_MAX_LEN)
        .copied()
        .collect()
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

    fn enc_with_stop(text: &str, stop: u8) -> Vec<u8> {
        let mut bytes = enc(text);
        bytes.push(stop);
        bytes
    }

    fn ctx() -> ConversationContext<'static> {
        ConversationContext {
            avatar_name: "Cal",
            branch_flags: 0,
            moral_standing: 0,
            dictionary: None,
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
        let mut s = ConversationSession::new(raw.clone(), decoded.clone());
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
    fn ask_party_name_caps_typed_answer_at_fifteen_bytes() {
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
        let party_names: [&[u8]; 1] = [b"ABCDEFGHIJKLMNO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);
        s.submit_keyword("join", &context);

        let second = s.submit_keyword("ABCDEFGHIJKLMNOEXTRA", &context);
        assert_eq!(second.asked_party_name, Some(1));
        assert_eq!(second.text, " Done.");
    }

    #[test]
    fn affordable_gold_payment_debits_without_an_extra_confirmation_prompt() {
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
                bytes.extend_from_slice(&enc(" Paid."));
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
            gold_available: Some(30),
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);

        let paid = s.submit_keyword("pay", &context);
        assert_eq!(paid.text, "A toll. Paid.");
        assert_eq!(
            paid.gold_payments,
            vec![ConversationGoldPayment {
                amount: 25,
                accepted: true
            }]
        );
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
        assert_eq!(s.prompt_message(), TLK_KEYWORD_PROMPT);
    }

    #[test]
    fn unaffordable_gold_payment_enters_one_nested_prompt_then_closes() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("PAY"),
            {
                let mut bytes = vec![TLK_CODE_GOLD_PAYMENT, b'0', b'2', b'5'];
                bytes.extend_from_slice(&enc(" Paid."));
                bytes.push(TLK_CODE_END_OF_RESPONSE);
                bytes
            },
            enc("HELP"),
            enc_with_stop("Nested response.", TLK_CODE_END_OF_RESPONSE),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "PAY".to_string(),
            String::new(),
            "HELP".to_string(),
            "Nested response.".to_string(),
        ];
        let context = ConversationContext {
            gold_available: Some(10),
            ..ctx()
        };
        let mut s = ConversationSession::new(raw.clone(), decoded.clone());
        s.present_greeting(&context);
        let refused = s.submit_keyword("pay", &context);
        assert_eq!(refused.text, TLK_GOLD_PAYMENT_REFUSAL_MESSAGE);
        assert_eq!(
            refused.gold_payments,
            vec![ConversationGoldPayment {
                amount: 25,
                accepted: false
            }]
        );
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingGoldRefusalKeyword
        );
        assert_eq!(s.prompt_message(), TLK_KEYWORD_PROMPT);

        let name = s.submit_keyword("name", &context);
        assert_eq!(name.text, "Ada");
        assert!(!name.ended);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingGoldRefusalKeyword
        );

        let nested = s.submit_keyword("", &context);
        assert_eq!(nested.text, "BYE\n\nFarewell.");
        assert!(nested.ended);
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);

        // A response carrying an explicit stop also unwinds immediately and
        // receives no synthetic mandatory-Bye output.
        let mut stopped = ConversationSession::new(raw, decoded);
        stopped.present_greeting(&context);
        stopped.submit_keyword("pay", &context);
        let response = stopped.submit_keyword("help", &context);
        assert_eq!(response.text, "Nested response.");
        assert!(response.ended);
        assert!(!response.text.contains("Farewell"));
    }

    /// `conversation.md §7.6`: refusal stops before the byte after the
    /// third digit, while acceptance continues in place. A side effect in
    /// that tail therefore runs only on the affordable path.
    #[test]
    fn unaffordable_gold_payment_does_not_execute_the_success_tail() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("PAY"),
            {
                let mut bytes = vec![TLK_CODE_GOLD_PAYMENT, b'0', b'2', b'5'];
                bytes.push(TLK_CODE_STANDING_DOWN);
                bytes.extend_from_slice(&enc("Paid."));
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
        let affordable = ConversationContext {
            moral_standing: 40,
            gold_available: Some(30),
            ..ctx()
        };
        let unaffordable = ConversationContext {
            moral_standing: 40,
            gold_available: Some(10),
            ..ctx()
        };

        let mut declined = ConversationSession::new(raw.clone(), decoded.clone());
        declined.present_greeting(&unaffordable);
        let out = declined.submit_keyword("pay", &unaffordable);
        assert_eq!(out.text, TLK_GOLD_PAYMENT_REFUSAL_MESSAGE);
        assert_eq!(out.moral_standing, None);

        let mut paid = ConversationSession::new(raw, decoded);
        paid.present_greeting(&affordable);
        let out = paid.submit_keyword("pay", &affordable);
        assert_eq!(out.text, "Paid.");
        assert_eq!(out.moral_standing, Some(39));
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
        assert!(out.text.starts_with(TLK_EMPTY_INPUT_BYE_MESSAGE));
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
    fn end_stream_and_nul_terminate_current_entry_without_closing_conversation() {
        let raw = vec![
            enc_with_stop("Ada", TLK_CODE_END_STREAM),
            enc_with_stop("a quiet smith", TLK_CODE_END_STREAM),
            enc_with_stop("Greetings.", TLK_CODE_END_STREAM),
            enc_with_stop("I mend gear.", TLK_CODE_END_STREAM),
            enc_with_stop("Farewell.", TLK_CODE_END_STREAM),
            enc("TRADE"),
            enc_with_stop("Bring iron.", 0),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "TRADE".to_string(),
            "Bring iron.".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);

        let greeting = s.present_greeting(&ctx());
        assert!(greeting.text.contains("Greetings."));
        assert!(!greeting.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);

        let job = s.submit_keyword("job", &ctx());
        assert!(job.text.contains("I mend gear."));
        assert!(!job.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);

        let ordinary = s.submit_keyword("trade", &ctx());
        assert!(ordinary.text.contains("Bring iron."));
        assert!(!ordinary.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
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
    fn set_flag_follow_up_keyword_scan_runs_response_for_remaining_input() {
        let mut response = enc("Base ");
        response.push(TLK_CODE_SET_FLAG);
        response.extend_from_slice(&enc(" after."));
        response.push(TLK_CODE_END_OF_RESPONSE);
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("GRAN"),
            response,
            enc("NEWS"),
            enc_with_stop("Nested", TLK_CODE_END_OF_RESPONSE),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "GRAN".to_string(),
            String::new(),
            "NEWS".to_string(),
            "Nested".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());

        let out = s.submit_keyword("gran news", &ctx());
        assert_eq!(out.text, "Base Nested after.");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn set_flag_follow_up_keyword_scan_restores_stream_on_miss() {
        let mut response = enc("Base ");
        response.push(TLK_CODE_SET_FLAG);
        response.extend_from_slice(&enc(" after."));
        response.push(TLK_CODE_END_OF_RESPONSE);
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("GRAN"),
            response,
            enc("NEWS"),
            enc_with_stop("Nested", TLK_CODE_END_OF_RESPONSE),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "GRAN".to_string(),
            String::new(),
            "NEWS".to_string(),
            "Nested".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());

        let out = s.submit_keyword("gran gossip", &ctx());
        assert_eq!(out.text, "Base  after.");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn ordinary_keyword_input_is_capped_at_fifteen_bytes() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("ABCDEFGHIJKLMNO"),
            enc("Fifteen."),
            enc("ABCDEFGHIJKLMNOP"),
            enc("Sixteen."),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "ABCDEFGHIJKLMNO".to_string(),
            "Fifteen.".to_string(),
            "ABCDEFGHIJKLMNOP".to_string(),
            "Sixteen.".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());

        let out = s.submit_keyword("ABCDEFGHIJKLMNOP", &ctx());
        assert_eq!(out.text, "Fifteen.");
    }

    #[test]
    fn no_match_keyword_returns_polite_refusal() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("xyzzy", &ctx());
        assert_eq!(out.text, TLK_NO_KEYWORD_MATCH_MESSAGE);
        assert!(out.text.ends_with("\n\n"));
    }

    #[test]
    fn reserved_rebuke_keyword_returns_to_prompt_without_ending() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());

        let out = s.submit_keyword("ASS HAT", &ctx());

        assert_eq!(out.text, TLK_RESERVED_REBUKE_MESSAGE);
        assert!(!out.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn label_transfer_enters_scoped_prompt_and_matches_local_keyword() {
        let mut scoped = enc("Topic?");
        scoped.push(0x91);
        scoped.push(TLK_CODE_END_OF_RESPONSE);
        scoped.push(TLK_CODE_LABEL_RECORD);
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("APPLE"));
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("Local answer."));
        scoped.push(TLK_CODE_END_OF_RESPONSE);
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("ASK"),
            scoped,
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "ASK".to_string(),
            "Topic?".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());

        let first = s.submit_keyword("ask", &ctx());
        assert_eq!(first.text, "Topic?");
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        assert_eq!(s.prompt_message(), TLK_KEYWORD_PROMPT);

        let second = s.submit_keyword("apple", &ctx());
        assert_eq!(second.text, "Local answer.");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn scoped_prompt_empty_input_reissues_without_closing() {
        let mut scoped = enc("Topic?");
        scoped.push(0x91);
        scoped.push(TLK_CODE_LABEL_RECORD);
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("APPLE"));
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("Local answer."));
        scoped.push(TLK_CODE_END_OF_RESPONSE);
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("ASK"),
            scoped,
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "ASK".to_string(),
            "Topic?".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());
        s.submit_keyword("ask", &ctx());

        let empty = s.submit_keyword("", &ctx());
        assert_eq!(empty.text, TLK_EMPTY_INPUT_BYE_MESSAGE);
        assert!(!empty.ended);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
    }

    #[test]
    fn scoped_prompt_suppresses_reserved_words_before_top_level_fallback() {
        let mut scoped = enc("Topic?");
        scoped.push(0x91);
        scoped.push(TLK_CODE_LABEL_RECORD);
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("APPLE"));
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("Local answer."));
        scoped.push(TLK_CODE_END_OF_RESPONSE);
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("ASK"),
            scoped,
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "ASK".to_string(),
            "Topic?".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());
        s.submit_keyword("ask", &ctx());

        let name = s.submit_keyword("name", &ctx());
        assert_eq!(name.text, TLK_NO_KEYWORD_MATCH_MESSAGE);
        assert!(!name.text.contains("Ada"));
        assert!(!name.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn scoped_prompt_unmatched_keyword_can_fall_back_to_top_level_pair() {
        let mut scoped = enc("Topic?");
        scoped.push(0x91);
        scoped.push(TLK_CODE_LABEL_RECORD);
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("APPLE"));
        scoped.push(0x91);
        scoped.extend_from_slice(&enc("Local answer."));
        scoped.push(TLK_CODE_END_OF_RESPONSE);
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("ASK"),
            scoped,
            enc("GRAN"),
            enc("Top-level answer."),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "ASK".to_string(),
            "Topic?".to_string(),
            "GRAN".to_string(),
            "Top-level answer.".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());
        s.submit_keyword("ask", &ctx());

        let fallback = s.submit_keyword("gran", &ctx());
        assert_eq!(fallback.text, "Top-level answer.");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
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
