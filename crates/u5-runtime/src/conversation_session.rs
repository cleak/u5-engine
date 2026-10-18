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

use crate::constants::SAVE_PARTY_SIZE_MAX;
use crate::map_io::talk_branch_flag_mask;
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
    /// `0x88` ASK-WHO compares the next typed line against this list with
    /// the published first-four-characters rule and passes the matched
    /// 1-based slot to the TLK runner. Its length is also the active party
    /// size that `0x84` RECRUIT-SPEAKER tests against the six-member cap.
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
    /// `conversation.md §7.6`: a `0x84` RECRUIT-SPEAKER reached this
    /// step's stream with room in the party, so the caller must run the
    /// reserve-roster recruitment for the speaking NPC. The code takes no
    /// argument and reads no input, so there is nothing else to carry.
    pub recruit_speaker: bool,
    /// Transitional bridge to the play-state roster path, which still
    /// keys its insertion off this field. `Some(0)` accompanies every
    /// `recruit_speaker`, because §7.6 gives RECRUIT-SPEAKER no player
    /// answer to report; nothing else ever sets it.
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
    /// The step's stream ended on `0x82` END-STREAM, directly or through a
    /// nested label head or alias. `conversation.md §7.2`: the code "Stop[s]
    /// the current stream and signal[s] the caller 'this stream is
    /// finished'", and §7.7 has a nested response that "reaches a
    /// byte-runner stop condition" report stop to the label handler.
    ///
    /// `0xFF` is deliberately not counted: §7.2 has it "return control to
    /// the keyword input loop", and the dwelling rat's Description entry,
    /// which resumes after its ASK-WHO and ends on `0xFF`, reaches
    /// `Your interest?` in the original (`dwelling-talk-after-entry`, beat
    /// `name`).
    pub end_stream_reached: bool,
    /// Shared moral-standing selector after any `0x89` / `0x8A` writes in
    /// this step's streams, or `None` when none ran. `conversation.md
    /// §7.4` makes those two codes the byte runner's only direct writers
    /// of the selector; the caller assigns this value rather than
    /// re-deriving a delta.
    pub moral_standing: Option<u8>,
    /// `conversation.md §7.5` / §10 "Town alarm" (u5-spec#298 Q14): how many
    /// `0x8B` ALARM codes this step's streams reached. The caller runs the
    /// town alarm sweep once for each; "conversation passes no arguments and
    /// reads no result".
    pub alarm_sweeps: u8,
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
            .extend(crate::ordinary_glyphs_from_engine_text(text));
    }

    /// `conversation.md §6`: a per-NPC response "finish[es] its ordinary
    /// quote/newline framing, then reprompt[s]", and the empty-input exit
    /// runs the Bye response "in quotes". Measured on a decoded capture
    /// (`cleak/u5-spec#198`): a blank row, the response in double quotes,
    /// a blank row, then the reprompt.
    ///
    /// The framing wraps the *response*, not the envelope around it -
    /// the `BYE` header the empty-input exit prints first stays outside
    /// the quotes, which is why this sits here rather than around the
    /// finished output.
    ///
    /// A response whose own stream already opened a quote
    /// (`tlk.md §9.2`'s `0xA2`) is left alone; the runner's
    /// adjacent-quote dedup does not reach this layer.
    fn push_framed_response(&mut self, rendered: &TlkRenderedText) {
        let already_quoted = rendered.text.trim_start().starts_with('"');
        // The closing quote hugs the text: a response whose own stream
        // ends in line feeds keeps them *outside* the quotes, where the
        // original puts them. A `0x83` / `0x8F` hold marker among those
        // trailing feeds is not text either - a stream that closes its own
        // quote, feeds twice and key-waits before an ASK-WHO (Anthony's
        // `reci`) must not get a second closing quote after the hold, which
        // showed as a stray lone-quote row (playtest F05, `conversation.md
        // §7.6` "What ASK-WHO prints").
        let is_hold = |byte: u8| crate::text_hold::TextHoldKind::from_marker(byte).is_some();
        let body_len = rendered
            .text
            .bytes()
            .rposition(|byte| byte != b'\n' && !is_hold(byte))
            .map_or(0, |index| index + 1);
        // §6's framing opens with a line feed, but the `BYE` header of
        // the empty-input exit has already printed its own two - "Print
        // `BYE` followed by two line feeds, run the NPC's mandatory Bye
        // response in quotes" - so do not add a third.
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.push_plain_text("\n");
        } else if self.text.is_empty() {
            self.push_plain_text("\n");
        }
        if !already_quoted {
            self.push_plain_text("\"");
        }
        self.text.push_str(&rendered.text[..body_len]);
        self.rendered_glyphs
            .extend(rendered.glyphs.iter().take(body_len).copied());
        // `conversation.md §9`: "The renderer drops a quote that would
        // immediately follow another quote" - a response whose stream
        // ends on its own closing quote keeps just that one. Playtest E02
        // (`thank thee!""`).
        if !already_quoted && !rendered.text[..body_len].ends_with('"') {
            self.push_plain_text("\"");
        }
        if body_len < rendered.text.len() {
            self.text.push_str(&rendered.text[body_len..]);
            self.rendered_glyphs
                .extend(rendered.glyphs.iter().skip(body_len).copied());
        }
        // §6's framing closes with a blank row before the reprompt, the
        // same shape the no-match literal already carries. Two line
        // feeds, because `text-output.md §5`'s feed is a combined
        // CR+LF, so a run of *k* leaves *k - 1* blank rows. Feeds the
        // stream put in front of a hold marker count towards the two.
        let tail_feeds = rendered.text.as_bytes()[body_len..]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count();
        if tail_feeds < 2 || !rendered.text.ends_with(|c: char| is_hold(c as u8)) {
            while !self.text.ends_with("\n\n") {
                self.push_plain_text("\n");
            }
        }
    }

    /// [`Self::push_framed_response`] for a fixed engine literal.
    fn set_framed_text(&mut self, text: &str) {
        self.text.clear();
        self.rendered_glyphs.clear();
        self.push_framed_response(&TlkRenderedText::plain(text));
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
    /// `conversation.md §7.6` / §10: roster slot of the NPC being spoken
    /// to. It is the branch-flag bit `0x8C` tests and `0x88` sets; the
    /// script can neither choose nor forge it. `None` until the caller
    /// names one, which makes those tests read clear and those sets
    /// no-ops (§10).
    npc_slot: Option<u8>,
    /// Branch-flag bits an ASK-WHO in this session has set. The caller
    /// merges them into the scene bank only after the step returns, but
    /// §7.6's one-time reward form tests the bit "now (that is, the prompt
    /// just matched)" with the very next `0x8C` of the resumed stream, so
    /// every run in the session reads them over the caller's snapshot
    /// (playtest F01).
    branch_flags_written: u32,
    /// Set for the duration of one step when a stream in it reached
    /// `0x84` RECRUIT-SPEAKER.
    recruit_speaker_pending: bool,
    /// Labelled records currently executing after a GOTO, innermost last,
    /// with the blob position each one was entered at.
    label_records_running: Vec<(u8, usize)>,
    /// The pending `0x88` ASK-WHO was reached by the opening's Description
    /// entry (directly or through a label head it transferred to), so the
    /// resumed stream is still the Description entry's: `conversation.md
    /// §9` step 2, "If the Description entry signals a stop, the opener
    /// returns and nothing further is printed."
    opening_ask_who: bool,
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
            npc_slot: None,
            branch_flags_written: 0,
            recruit_speaker_pending: false,
            label_records_running: Vec::new(),
            opening_ask_who: false,
        }
    }

    /// `conversation.md §7.6` / §10: name the roster slot of the NPC
    /// being spoken to. `0x8C` IF-ELSE tests that slot's branch-flag bit
    /// and `0x88` ASK-WHO sets it; both read as no-ops until this is set,
    /// because §10 asks an unnameable index to build a zero mask.
    pub fn set_npc_slot(&mut self, slot: Option<u8>) {
        self.npc_slot = slot;
    }

    /// Run the greeting through the byte runner. Caller should display
    /// the rendered text and then transition to keyword input.
    /// `conversation.md §9` step 2's Description entry.
    ///
    /// The section describes it as text printed before the greeting, and
    /// for most NPCs that is all it is. It is an ordinary byte stream
    /// though, and at least one shipped NPC puts the *whole* opening in
    /// it - the description, the quoted greeting, a pause, and a `0x88`
    /// ASK-WHO - with an empty Greeting entry
    /// (`cleak/u5-spec#198`). Running it through the session is what
    /// lets that stop suspend the conversation, the way the greeting
    /// path's already does.
    pub fn present_description(
        &mut self,
        ctx: &ConversationContext<'_>,
    ) -> ConversationSessionOutput {
        let out = self.run_field_from(1, 0, ctx, 0);
        self.opening_ask_who = self.awaiting_ask_who();
        out
    }

    /// Whether the opening has already suspended for input, so the
    /// caller must not run the greeting on top of it.
    ///
    /// A Description entry that GOTOs a labelled block with a scoped prompt
    /// (`conversation.md §7.7`) suspends it as well: the prompt belongs to
    /// the label handler the Description stream entered, so §9 step 3's
    /// greeting or self-introduction must not run over it and reset the
    /// phase to the top-level loop (playtest F03, Fiona's food question).
    pub fn opening_suspended(&self) -> bool {
        matches!(
            self.phase,
            ConversationSessionPhase::AwaitingAskWho { .. }
                | ConversationSessionPhase::AwaitingScopedKeyword { .. }
        )
    }

    pub fn present_greeting(&mut self, ctx: &ConversationContext<'_>) -> ConversationSessionOutput {
        self.phase = ConversationSessionPhase::AwaitingKeyword;
        self.recruit_speaker_pending = false;
        self.run_field_from(2, 0, ctx, 0)
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
        self.recruit_speaker_pending = false;
        if !introduces_itself {
            return ConversationSessionOutput::default();
        }
        let mut output = self.run_field_from(0, 0, ctx, 0);
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
        self.recruit_speaker_pending = false;
        match self.phase {
            ConversationSessionPhase::AwaitingKeyword => {}
            ConversationSessionPhase::AwaitingScopedKeyword { label } => {
                return self.submit_scoped_keyword(label, line, ctx);
            }
            ConversationSessionPhase::AwaitingAskWho { field_idx, cursor } => {
                // §7.6 publishes ASK-WHO's own, looser match rule: the
                // first four characters of a member's name, found at line
                // start or immediately after a space.
                let input = capped_tlk_input_bytes(line);
                let slot = tlk_ask_who_match(&input, ctx.party_member_names);
                let from_opening = std::mem::take(&mut self.opening_ask_who);
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                // §7.6: on a match ASK-WHO "sets the active scene's
                // branch-flag bit for the NPC currently speaking". The
                // resume cursor is already past the control byte, so the
                // runner's own setter arm never re-runs on this path and
                // the session owns the write. It lands before the stream
                // resumes, so a following `0x8C` sees it. §10: an
                // unnameable slot builds a zero mask, so the set is a no-op.
                let written = if slot != 0 {
                    self.npc_slot.map_or(0, talk_branch_flag_mask)
                } else {
                    0
                };
                self.branch_flags_written |= written;
                let mut out = self.run_field_from(field_idx, cursor, ctx, slot);
                out.branch_flags_set |= written;
                // `conversation.md §7`'s `0x88` row: the code prints an
                // acknowledgement of its own after the typed line - the
                // affirmative one on a match, "on empty input or no match ...
                // the dismissive one". Neither is published and neither is in
                // the stream, so the resumed run carries only the quote pair
                // and this engine printed nothing.
                //
                // Both measured 2026-09-12 and neither published
                // (`cleak/u5-spec#266`): a matched member gives
                // `"A pleasure!` (`qa/paired/dwelling-talk-askwho-match.tsv`)
                // and anything else gives `"If you say so...`
                // (`qa/paired/dwelling-talk-after-entry.tsv`, beat `name`).
                // Each opens on a fresh row under the answer, with an opening
                // quote and no closing one.
                let acknowledgement = if slot == 0 {
                    TLK_ASK_WHO_DISMISSIVE_LINE
                } else {
                    TLK_ASK_WHO_AFFIRMATIVE_LINE
                };
                let mut rendered = TlkRenderedText::plain(&format!("\n\"{acknowledgement}"));
                rendered.push_rendered(&out.rendered_text().trimmed_ask_who_quote_pair());
                out.text = rendered.text;
                out.rendered_glyphs = rendered.glyphs;
                out.asked_who = Some(slot);
                // `conversation.md §9` step 2: "If the Description entry
                // signals a stop, the opener returns and nothing further is
                // printed", and §7.7 has a label head that "reaches a
                // byte-runner stop condition" report stop to its caller. A
                // Britain wizard's description GOTOs a head that asks the
                // name and, unacquainted, prints `Good day!"`, a feed and
                // `0x82`. Measured (`town-talk-speaker`, beat `name`): the
                // original ends the conversation there - no Bye entry, no
                // `Your interest?`, the command prompt on the next row -
                // where this engine ran on into the keyword loop.
                if from_opening {
                    if self.awaiting_ask_who() {
                        self.opening_ask_who = true;
                    } else if out.end_stream_reached
                        && matches!(self.phase, ConversationSessionPhase::AwaitingKeyword)
                    {
                        self.phase = ConversationSessionPhase::PresentingBye;
                        out.ended = true;
                    }
                }
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
            // `conversation.md §6`: the rebuke "prints `With language like
            // that, how did you become an Avatar?`, emits the same
            // quote/newline framing used by other reserved responses, then
            // runs a bounded pause loop" of "at most twenty-eight" passes
            // with an early key-abort, and returns to the keyword prompt.
            // That loop is the `0x83` timed pause's shape, so it rides the
            // same hold.
            out.set_framed_text(TLK_RESERVED_REBUKE_MESSAGE);
            debug_assert_eq!(
                u16::from(TLK_RESERVED_REBUKE_PAUSE_LIMIT),
                crate::text_hold::TEXT_HOLD_TIMED_PAUSE_PASSES
            );
            out.push_plain_text(crate::text_hold::TextHoldKind::TimedPause.marker_str());
            return out;
        }
        let field_idx = match kind {
            TlkPlayerInputKind::EmptyByeShortcut => 4usize,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry) => 0,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry) => 3,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath) => 4,
            TlkPlayerInputKind::ReservedRebuke { .. } => unreachable!(),
            TlkPlayerInputKind::OrdinaryKeywordScan => self
                .find_ordinary_keyword_response_index(&input_upper)
                .unwrap_or(usize::MAX),
        };
        if field_idx == usize::MAX {
            // Framed like a response stream, quotes included. `conversation.md
            // §6` step 7 writes the literal without quote characters, and
            // playtest E05 read that as "unquoted"; the stock capture says
            // otherwise: `dwelling-talk-after-entry` (rc1 `20261017-023408`,
            // rc2 `20261017-142857`) shows `"I cannot help` / `thee with
            // that."` on both of its no-match beats. Spec question raised.
            out.set_framed_text(TLK_NO_KEYWORD_MATCH_MESSAGE);
            return out;
        }
        // `conversation.md §5` step 3 and §7's result table both tie the
        // printed header to the *empty-input* shortcut alone: "If the player
        // pressed Enter on an empty line, the engine prints `BYE\n\n`" and
        // "Empty input | Print `BYE` followed by two line feeds". It stands in
        // for the echo the player never typed.
        //
        // A typed `BYE` has already echoed itself onto the `:` row, so
        // printing the header there shows the word twice. Measured 2026-09-11
        // (`town-talk/bye`): the original reads `:BYE` and then the response,
        // this engine read `:BYE` / `BYE` / the response.
        if matches!(kind, TlkPlayerInputKind::EmptyByeShortcut) {
            out.push_plain_text(TLK_EMPTY_INPUT_BYE_MESSAGE);
        }
        let response = self.run_field_from(field_idx, 0, ctx, 0);
        // `conversation.md` §7's reserved-keyword table: index 0, `NAME`,
        // "Run the Name entry with the **fixed name prefix**". The section
        // names the prefix but does not quote it; measured 2026-09-10
        // (`castle-talk/name`), the original answers `"My name is Chuckles"`
        // where this engine answered `"Chuckles"` - the bare Name entry, the
        // same field the introduction prefixes with `I am called `.
        // Published wording requested in cleak/u5-spec.
        if matches!(
            kind,
            TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry)
        ) {
            let mut rendered = TlkRenderedText::plain(TLK_NAME_KEYWORD_PREFIX);
            rendered.push_rendered(&response.rendered_text());
            out.push_framed_response(&rendered);
        } else {
            out.push_framed_response(&response.rendered_text());
        }
        out.branch_flags_set |= response.branch_flags_set;
        out.action_grants.extend(response.action_grants);
        out.gold_payments.extend(response.gold_payments);
        out.signal_flags.extend(response.signal_flags);
        out.alarm_sweeps = out.alarm_sweeps.saturating_add(response.alarm_sweeps);
        out.recruit_speaker |= response.recruit_speaker;
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
            ConversationSessionPhase::AwaitingAskWho { .. } => {
                crate::TLK_ASK_WHO_PROMPT.to_string()
            }
            ConversationSessionPhase::AwaitingGoldRefusalKeyword => TLK_KEYWORD_PROMPT.to_string(),
            ConversationSessionPhase::Opened => TLK_KEYWORD_PROMPT.to_string(),
            ConversationSessionPhase::PresentingBye | ConversationSessionPhase::Closed => {
                String::new()
            }
        }
    }

    /// Whether the next typed line answers a `0x88` ASK-WHO name prompt.
    /// `conversation.md §6` step 3: the empty-input BYE shortcut "belongs to
    /// [the keyword] prompt alone"; an empty line here prints the dismissive
    /// acknowledgement instead.
    pub fn awaiting_ask_who(&self) -> bool {
        matches!(self.phase, ConversationSessionPhase::AwaitingAskWho { .. })
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

    /// `conversation.md §7.6`: a `0x84` RECRUIT-SPEAKER ran during the
    /// step that just completed, so the caller should recruit the speaking
    /// NPC from the reserve roster.
    pub fn recruit_speaker_pending(&self) -> bool {
        self.recruit_speaker_pending
    }

    /// Transitional alias for [`Self::recruit_speaker_pending`].
    ///
    /// `0x84` is no longer a prompt at all — §7.6: "There is no player
    /// prompt and no input read" — so nothing in this session ever waits
    /// for a party name. The old name survives only for the play-state
    /// caller that still spells the recruit signal this way.
    pub fn awaiting_ask_party_name(&self) -> bool {
        self.recruit_speaker_pending()
    }

    fn find_ordinary_keyword_response_index(&self, input_upper: &[u8]) -> Option<usize> {
        // Pairs start at field index 5: (keyword, response, keyword,
        // response, ...). Scan keyword positions; the response is the
        // next index.
        let mut idx = TLK_LEADING_ENTRY_COUNT;
        if idx % 2 == 0 {
            idx += 1;
        }
        while idx + 1 < self.decoded_fields.len() {
            let keyword_field = &self.decoded_fields[idx];
            let keyword = keyword_field.trim().as_bytes();
            if tlk_keyword_matches(keyword, input_upper) {
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
        ask_who_response: u8,
    ) -> ConversationSessionOutput {
        let (_, field_starts) = self.blob_view();
        let Some(&field_start) = field_starts.get(field_idx) else {
            return ConversationSessionOutput::default();
        };
        self.run_blob_from(field_start + start, ctx, ask_who_response)
    }

    /// The loaded NPC blob as one byte string - the records joined by the
    /// NUL terminators they were split at - and where each record starts.
    ///
    /// `conversation.md §7.7`: a label transfer "rewinds the stream pointer
    /// to the start of the loaded NPC blob" and scans it for the `0x90
    /// <label>` declaration, which shipped content always puts at the head
    /// of a record. Running every stream against the whole blob is what lets
    /// a transfer land in another record; each stream still ends at its own
    /// NUL.
    fn blob_view(&self) -> (Vec<u8>, Vec<usize>) {
        let mut blob = Vec::new();
        let mut starts = Vec::with_capacity(self.fields.len());
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                blob.push(0);
            }
            starts.push(blob.len());
            blob.extend_from_slice(field);
        }
        (blob, starts)
    }

    /// The record holding blob position `pos`, and `pos` relative to it.
    fn field_at(field_starts: &[usize], pos: usize) -> (usize, usize) {
        let index = field_starts
            .iter()
            .rposition(|start| *start <= pos)
            .unwrap_or(0);
        (index, pos - field_starts.get(index).copied().unwrap_or(0))
    }

    fn run_blob_from(
        &mut self,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_who_response: u8,
    ) -> ConversationSessionOutput {
        let (blob, field_starts) = self.blob_view();
        let mut out = ConversationSessionOutput::default();
        let mut cursor = start;
        loop {
            let inputs = make_inputs(
                ctx,
                self.branch_flags_written,
                self.moral_standing.unwrap_or(ctx.moral_standing),
                self.npc_slot,
                ask_who_response,
            );
            let run = run_tlk_stream_from(&blob, cursor, &inputs);
            self.absorb_run(&field_starts, &run, ctx, &mut out);
            match run.stop {
                TlkRunStop::KeywordAlias(resume) => {
                    // `conversation.md §7.6`: `0x87` is positional. Skip the
                    // remainder of this record, any run of terminators, and
                    // the whole record that follows; run the record after
                    // that as a nested stream. Records arrive here already
                    // split, so the byte walk collapses to "two records on"
                    // from the record the code sits in.
                    let (alias_field, _) = Self::field_at(&field_starts, resume.saturating_sub(1));
                    let alias_target = alias_field + TLK_KEYWORD_ALIAS_RECORD_SKIP;
                    if alias_target < self.fields.len() {
                        let nested = self.run_field_from(alias_target, 0, ctx, 0);
                        // "If the nested stream signals stop, the outer
                        // stream stops too; otherwise the saved position is
                        // restored."
                        let nested_signalled_stop = nested.response_signalled_stop || nested.ended;
                        merge_session_output(&mut out, nested);
                        if nested_signalled_stop
                            || !matches!(self.phase, ConversationSessionPhase::AwaitingKeyword)
                        {
                            break;
                        }
                    }
                    cursor = resume;
                }
                TlkRunStop::LabelTransfer(label) => {
                    self.follow_label_transfer(&blob, label, run.consumed, ctx, &mut out);
                    break;
                }
                _ => break,
            }
        }
        out
    }

    /// `conversation.md §7.7`: a `0x91..0x9F` byte "records the label byte,
    /// rewinds the stream pointer to the start of the loaded NPC blob, and
    /// enters the label handler", which finds the `0x90 <label>` declaration
    /// and "resumes the stream immediately past the label byte". The record
    /// found runs as an ordinary stream - §7.6's routing record, "Reached by
    /// a GOTO from a keyword response", runs its own IF-ELSE, welcome line,
    /// PAUSE and ASK-WHO - and only then can the labelled block's scoped
    /// prompt open.
    ///
    /// u5-spec#298 Q20, "What a GOTO runs first": the head is the stream
    /// "up to the first NUL ... A repeated label is not a boundary. Inside
    /// the head every byte keeps its ordinary meaning: a label byte in
    /// `0x91..0x9F` met as a code is a live GOTO at that point, including one
    /// to the block's own label, which restarts the block". "A self-GOTO
    /// re-enters the label handler as a nested call, and the outer call
    /// returns as soon as the inner one stops, so a retry loop never opens a
    /// second scoped prompt for the outer level." The earlier reading here -
    /// a GOTO back to the running block was its sub-record separator - is
    /// withdrawn. `from` is kept for the caller's record position only.
    fn follow_label_transfer(
        &mut self,
        blob: &[u8],
        label: u8,
        from: usize,
        ctx: &ConversationContext<'_>,
        out: &mut ConversationSessionOutput,
    ) {
        let _ = from;
        let Some(resume) = find_label_declaration(blob, label) else {
            return;
        };
        // The depth bound only stops a head that GOTOs itself with nothing
        // in between that stops the stream; shipped content has no such head
        // (§7.7's census: the one self-GOTO head asks ASK-WHO first).
        if self.label_records_running.len() < TLK_LABEL_TRANSFER_DEPTH_LIMIT {
            let phase_before = self.phase;
            self.label_records_running.push((label, resume));
            let nested = self.run_blob_from(resume, ctx, 0);
            self.label_records_running.pop();
            let nested_signalled_stop = nested.response_signalled_stop || nested.ended;
            merge_session_output(out, nested);
            if nested_signalled_stop || self.phase != phase_before {
                return;
            }
        }
        if self.has_scoped_label_records(label) {
            self.phase = ConversationSessionPhase::AwaitingScopedKeyword { label };
        }
    }

    fn absorb_run(
        &mut self,
        field_starts: &[usize],
        run: &TlkRunOutput,
        ctx: &ConversationContext<'_>,
        out: &mut ConversationSessionOutput,
    ) {
        out.text.push_str(&run.text);
        out.rendered_glyphs.extend_from_slice(&run.rendered_glyphs);
        out.branch_flags_set |= run.branch_flags_set;
        self.branch_flags_written |= run.branch_flags_set;
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
        out.alarm_sweeps = out.alarm_sweeps.saturating_add(tlk_run_alarm_count(run));
        out.response_signalled_stop |= matches!(run.stop, TlkRunStop::EndOfResponse);
        out.end_stream_reached |= matches!(run.stop, TlkRunStop::EndOfStream);
        self.absorb_moral_standing(run, out);
        self.absorb_recruit_speaker(run, ctx, out);
        match run.stop {
            TlkRunStop::AskingWho(pos) => {
                let (field_idx, cursor) = Self::field_at(field_starts, pos);
                self.phase = ConversationSessionPhase::AwaitingAskWho { field_idx, cursor };
            }
            TlkRunStop::GoldPaymentRefused { .. } => {
                self.phase = ConversationSessionPhase::AwaitingGoldRefusalKeyword;
            }
            _ => {}
        }
    }

    /// `conversation.md §7.7`: the scoped prompt reads "another free-text
    /// answer, scan[s] the fixed reserved-word table, and then match[es]
    /// label-scoped keywords inside the labelled block. ... This is how an
    /// NPC can ask a follow-up question and route the answer without
    /// returning to the top-level keyword list." Reserved words are
    /// suppressed ("top-level reserved words such as `NAME`, `JOB`, `WORK`,
    /// `BYE`, and `THANK` do not run their ordinary global responses"), so
    /// they simply take part in the scoped match. Empty input "prints the
    /// prompt's BYE line and reissues the scoped prompt".
    ///
    /// Engine reading of the untraced part (`OPEN-QUESTIONS.md`, "What a
    /// label block's scoped prompt does with the records after the head"):
    /// see [`Self::scoped_block`]. An unmatched answer runs the block's
    /// default arm; whichever record ran, a NUL-ended response then "opens
    /// the ordinary top-level keyword loop as the fallback path" - the next
    /// prompt is the top-level one, unless the record itself re-entered a
    /// label block (the shipped "ask again" arm is a lone GOTO to its own
    /// block) or paused on ASK-WHO.
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

        self.phase = ConversationSessionPhase::AwaitingKeyword;
        let Some(block) = self.scoped_block(label) else {
            // No records follow the head: nothing scoped can run, so the
            // answer goes to §7.7's top-level fallback.
            return self.submit_keyword(line, ctx);
        };
        let field_idx = block
            .keywords
            .iter()
            .find(|(keyword_idx, _)| {
                let keyword = self.fields[*keyword_idx]
                    .iter()
                    .map(|b| b & 0x7F)
                    .collect::<Vec<_>>();
                tlk_keyword_matches(keyword.trim_ascii(), &input_upper)
            })
            .map_or(block.default_arm, |(_, response_idx)| *response_idx);
        let response = self.run_field_from(field_idx, 0, ctx, 0);
        let mut out = ConversationSessionOutput::default();
        // The scoped answer is a response record like any keyword answer,
        // so it takes §6's ordinary quote/newline framing.
        out.push_framed_response(&response.rendered_text());
        let text = std::mem::take(&mut out.text);
        let glyphs = std::mem::take(&mut out.rendered_glyphs);
        merge_session_output(&mut out, response);
        out.text = text;
        out.rendered_glyphs = glyphs;
        out
    }

    /// Whether a GOTO to `label` opens a scoped prompt: the block must have
    /// at least one record after its head for the prompt to match against.
    fn has_scoped_label_records(&self, label: u8) -> bool {
        self.scoped_block(label).is_some()
    }

    /// The records a labelled block's scoped prompt matches against.
    ///
    /// u5-spec#298 Q20 settles the head: it runs "up to the first NUL", and
    /// "A repeated label is not a boundary". What follows the head is not
    /// traced (`OPEN-QUESTIONS.md`), so the layout is read off the shipped
    /// `.TLK` data, where every one of the 259 labelled blocks with records
    /// after its head has an odd count of them: the record right after the
    /// head is the arm run when no scoped keyword matches (the census's
    /// "'ask again' arm of a yes/no question" when it is a lone GOTO to the
    /// block's own label, otherwise a line such as "Just let me know
    /// when."), and the rest are keyword/response pairs (`y`/`n` in the
    /// main), up to the next record that opens with the `0x90` declaration
    /// marker - the next block, or the blob's final `0x90 0x9F` record. An
    /// unpaired trailing keyword record is ignored.
    fn scoped_block(&self, label: u8) -> Option<ScopedLabelBlock> {
        let (blob, field_starts) = self.blob_view();
        let resume = find_label_declaration(&blob, label)?;
        let (head_idx, _) = Self::field_at(&field_starts, resume);
        let records_end = (head_idx + 1..self.fields.len())
            .find(|idx| self.fields[*idx].first() == Some(&TLK_CODE_LABEL_RECORD))
            .unwrap_or(self.fields.len());
        let default_arm = head_idx + 1;
        if default_arm >= records_end {
            return None;
        }
        let keywords = (default_arm + 1..records_end)
            .step_by(2)
            .filter(|keyword_idx| keyword_idx + 1 < records_end)
            .map(|keyword_idx| (keyword_idx, keyword_idx + 1))
            .collect();
        Some(ScopedLabelBlock {
            default_arm,
            keywords,
        })
    }

    /// `conversation.md §7.6`: a `0x84` RECRUIT-SPEAKER in the stream
    /// recruits the speaking NPC. The code takes no argument, prompts for
    /// nothing, and reads no input, so all this layer does is decide the
    /// cap and hand the caller the signal; the reserve-roster scan and
    /// the record swap belong to the party-state owner.
    ///
    /// "If the party is already at the six-member cap the engine prints
    /// the ... refusal and recruits nobody."
    fn absorb_recruit_speaker(
        &mut self,
        run: &TlkRunOutput,
        ctx: &ConversationContext<'_>,
        out: &mut ConversationSessionOutput,
    ) {
        if !run
            .events
            .iter()
            .any(|event| matches!(event, TlkRunEvent::RecruitSpeaker))
        {
            return;
        }
        if ctx.party_member_names.len() >= SAVE_PARTY_SIZE_MAX as usize {
            out.push_plain_text(TLK_RECRUIT_SPEAKER_FULL_PARTY_REFUSAL);
            return;
        }
        self.recruit_speaker_pending = true;
        out.recruit_speaker = true;
        out.asked_party_name = Some(0);
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

/// The records after a labelled block's head, as fields of the session's
/// blob: see [`ConversationSession::scoped_block`].
struct ScopedLabelBlock {
    default_arm: usize,
    /// `(keyword record, response record)` pairs, in blob order.
    keywords: Vec<(usize, usize)>,
}

/// Bound on nested label transfers inside one step. `conversation.md §7.7`
/// allows fifteen labels per blob, so a deeper chain can only be a loop.
const TLK_LABEL_TRANSFER_DEPTH_LIMIT: usize = 15;

/// `conversation.md §7.7`: "the scan looks for `0x90`, checks whether the
/// byte after it equals the active label, and on a match resumes the stream
/// immediately past the label byte; otherwise it keeps scanning." Returns
/// that resume position, scanning from the blob start.
fn find_label_declaration(blob: &[u8], label: u8) -> Option<usize> {
    if !is_tlk_label_byte(label) {
        return None;
    }
    // u5-spec#298 Q20: "The declaration search also accepts the final-marker
    // value `0x9F` in place of the declaration marker before the label; no
    // shipped record contains that pairing".
    blob.windows(2)
        .position(|pair| {
            (pair[0] == TLK_CODE_LABEL_RECORD || pair[0] == TLK_LABEL_LAST) && pair[1] == label
        })
        .map(|pos| pos + 2)
}

/// `conversation.md §7.5`: each `0x8B` the runner reached is one alarm sweep.
pub fn tlk_run_alarm_count(run: &TlkRunOutput) -> u8 {
    let count = run
        .events
        .iter()
        .filter(|event| matches!(event, TlkRunEvent::Alarm))
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn merge_session_output(out: &mut ConversationSessionOutput, nested: ConversationSessionOutput) {
    out.text.push_str(&nested.text);
    out.rendered_glyphs.extend(nested.rendered_glyphs);
    out.branch_flags_set |= nested.branch_flags_set;
    out.action_grants.extend(nested.action_grants);
    out.gold_payments.extend(nested.gold_payments);
    out.signal_flags.extend(nested.signal_flags);
    out.alarm_sweeps = out.alarm_sweeps.saturating_add(nested.alarm_sweeps);
    out.recruit_speaker |= nested.recruit_speaker;
    out.asked_party_name = nested.asked_party_name.or(out.asked_party_name);
    out.asked_who = nested.asked_who.or(out.asked_who);
    out.moral_standing = nested.moral_standing.or(out.moral_standing);
    out.response_signalled_stop |= nested.response_signalled_stop;
    out.end_stream_reached |= nested.end_stream_reached;
    out.ended |= nested.ended;
}

fn make_inputs<'a>(
    ctx: &'a ConversationContext<'a>,
    branch_flags_written: u32,
    moral_standing: u8,
    npc_slot: Option<u8>,
    ask_who_response: u8,
) -> TlkRunInputs<'a> {
    TlkRunInputs {
        avatar_name: ctx.avatar_name,
        branch_flags: ctx.branch_flags | branch_flags_written,
        moral_standing,
        dictionary: ctx.dictionary,
        gold_available: ctx.gold_available,
        npc_slot,
        ask_who_response,
        // `conversation.md §7.3` (corrected 2026-10-12, R520): `0x83` is a
        // timed pause of up to 28 passes that a key ends and consumes, and
        // `0x8F` blocks for one key and emits nothing; "Every response
        // stream ... goes through the same byte runner, so both codes behave
        // identically on every path" and "Neither code ever stops a
        // response". So the stream is not cut here: each code leaves a
        // `text_hold` marker and the message layer holds the rest of the
        // output behind it (`PlayState::emit_tlk_message`).
        yield_on_pause: false,
        mark_holds: true,
        yield_on_ask: true,
    }
}

/// `conversation.md §6` step 2 caps the line at fifteen characters; §7.6:
/// "a line containing only a space is *not* empty", so the typed bytes are
/// kept as typed rather than trimmed - only a line terminator is dropped.
fn capped_tlk_input_bytes(line: &str) -> Vec<u8> {
    line.trim_end_matches(['\r', '\n'])
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

    /// `conversation.md §6`'s response framing, as the tests expect to
    /// see it: a line feed, the response in double quotes, and any
    /// trailing line feeds the response's own stream carried left
    /// outside the closing quote.
    fn framed_response(text: &str) -> String {
        framed_response_after("", text)
    }

    /// [`framed_response`] appended to text a handler has already
    /// emitted - the `BYE` header, for instance, which supplies its own
    /// line feeds.
    fn framed_response_after(prefix: &str, text: &str) -> String {
        let lead = if prefix.is_empty() || !prefix.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let mut out = if text.trim_start().starts_with('"') {
            format!("{prefix}{lead}{text}")
        } else {
            let body = text.trim_end_matches('\n');
            let tail = &text[body.len()..];
            let close = if body.ends_with('"') { "" } else { "\"" };
            format!("{prefix}{lead}\"{body}{close}{tail}")
        };
        while !out.ends_with("\n\n") {
            out.push('\n');
        }
        out
    }

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
    fn keyword_response_ending_in_its_own_quote_is_not_doubled() {
        // `conversation.md §9`: "The renderer drops a quote that would
        // immediately follow another quote". Playtest E02
        // (`"Iolo and I both thank thee!""`).
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Hi."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("JOIN"),
            enc("We thank thee!\""),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Hi.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "JOIN".to_string(),
            "We thank thee!\"".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());
        let out = s.submit_keyword("join", &ctx());
        assert_eq!(out.text, "\n\"We thank thee!\"\n\n");
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

    fn recruit_blob() -> (Vec<Vec<u8>>, Vec<String>) {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("JOIN"),
            {
                let mut bytes = enc("I shall come.");
                bytes.push(TLK_CODE_RECRUIT_SPEAKER);
                bytes.extend_from_slice(&enc(" Lead on."));
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
            "I shall come.".to_string(),
        ];
        (raw, decoded)
    }

    #[test]
    fn recruit_speaker_runs_inline_without_prompting_for_a_name() {
        // `conversation.md §7.6`: `0x84` has "no player prompt and no
        // input read" — the whole response emits in one step and the
        // engine recruits the speaker itself.
        let (raw, decoded) = recruit_blob();
        let party_names: [&[u8]; 2] = [b"AVATAR", b"IOLO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);

        let out = s.submit_keyword("join", &context);
        assert_eq!(out.text, framed_response("I shall come. Lead on."));
        assert!(out.recruit_speaker);
        assert!(s.recruit_speaker_pending());
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
        assert_eq!(s.prompt_message(), TLK_KEYWORD_PROMPT);
    }

    #[test]
    fn recruit_speaker_refuses_at_the_six_member_cap() {
        // §7.6: "If the party is already at the six-member cap the engine
        // prints the ... refusal and recruits nobody."
        let (raw, decoded) = recruit_blob();
        let party_names: [&[u8]; 6] = [
            b"AVATAR",
            b"IOLO",
            b"SHAMINO",
            b"DUPRE",
            b"MARIAH",
            b"GEOFFREY",
        ];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);

        let out = s.submit_keyword("join", &context);
        assert!(!out.recruit_speaker);
        assert_eq!(out.asked_party_name, None);
        assert!(!s.recruit_speaker_pending());
        // §6's framing closes its quote against the text, so a refusal
        // whose own literal carries trailing line feeds is split by the
        // closing quote. Match the body.
        assert!(
            out.text
                .contains(TLK_RECRUIT_SPEAKER_FULL_PARTY_REFUSAL.trim_end_matches('\n'))
        );
    }

    #[test]
    fn recruit_speaker_pending_clears_on_the_next_keyword() {
        let (raw, decoded) = recruit_blob();
        let party_names: [&[u8]; 1] = [b"AVATAR"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);
        assert!(s.submit_keyword("join", &context).recruit_speaker);
        assert!(!s.submit_keyword("job", &context).recruit_speaker);
        assert!(!s.recruit_speaker_pending());
    }

    fn ask_who_blob() -> (Vec<Vec<u8>>, Vec<String>) {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("NAMES"),
            {
                let mut bytes = enc("Who art thou?");
                bytes.push(TLK_CODE_ASK_WHO);
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
            "NAMES".to_string(),
            "Who art thou?".to_string(),
        ];
        (raw, decoded)
    }

    #[test]
    fn ask_who_accepts_a_first_four_character_hit_after_a_space() {
        // §7.6: the engine "takes the first four characters of that
        // member's name and searches for them as a substring of the typed
        // line", accepting only at line start or after a literal space.
        let (raw, decoded) = ask_who_blob();
        let party_names: [&[u8]; 2] = [b"AVATAR", b"IOLO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.set_npc_slot(Some(6));
        s.present_greeting(&context);

        let first = s.submit_keyword("names", &context);
        assert_eq!(first.text, framed_response("Who art thou?"));
        assert!(matches!(
            s.phase,
            ConversationSessionPhase::AwaitingAskWho { .. }
        ));
        assert_eq!(s.prompt_message(), crate::TLK_ASK_WHO_PROMPT);

        let second = s.submit_keyword("my friend Iolo", &context);
        assert_eq!(second.asked_who, Some(2));
        // `conversation.md §7`: a matched member takes the affirmative
        // acknowledgement, measured as `"A pleasure!` (`cleak/u5-spec#266`).
        assert_eq!(
            second.text,
            format!("\n\"{}{}", crate::TLK_ASK_WHO_AFFIRMATIVE_LINE, " Done.")
        );
        // §7.6: ASK-WHO is the in-stream setter for the bank `0x8C` tests,
        // and the bit is the speaking NPC's own roster slot.
        assert_eq!(second.branch_flags_set, 1u32 << 6);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    /// Playtest F01. `conversation.md §7.6`, the one-time reward form: after
    /// the ASK-WHO, "IF-ELSE with a *bonus* target label - if the bit is set
    /// now (that is, the prompt just matched a party member), transfer to the
    /// bonus record". The bit a match sets must be visible to the very next
    /// `0x8C` of the resumed stream, before the caller merges it.
    #[test]
    fn ask_who_match_is_visible_to_the_next_if_else_in_the_same_stream() {
        let mut response = enc("Welcome!");
        response.push(TLK_CODE_ASK_WHO);
        response.extend([TLK_CODE_IF_ELSE, 0x92]);
        response.extend(enc_with_stop("Indeed.", TLK_CODE_END_OF_RESPONSE));
        let mut bonus = vec![TLK_CODE_LABEL_RECORD, 0x92];
        bonus.extend([TLK_CODE_STANDING_UP; 5]);
        bonus.extend(enc_with_stop("A bonus!", TLK_CODE_END_OF_RESPONSE));
        let names: [&[u8]; 1] = [b"AVATAR"];
        let context = ConversationContext {
            party_member_names: &names,
            moral_standing: 10,
            ..ctx()
        };

        let mut s = label_session(response.clone(), vec![enc("OTHER"), bonus.clone()]);
        s.submit_keyword("ask", &context);
        assert!(s.awaiting_ask_who());
        let matched = s.submit_keyword("Avatar", &context);
        assert!(matched.text.contains("A bonus!"), "{:?}", matched.text);
        assert!(!matched.text.contains("Indeed."), "{:?}", matched.text);
        assert_eq!(matched.moral_standing, Some(15));
        assert_eq!(matched.branch_flags_set, 1 << 3);

        let mut s = label_session(response, vec![enc("OTHER"), bonus]);
        s.submit_keyword("ask", &context);
        let missed = s.submit_keyword("nobody", &context);
        assert!(missed.text.contains("Indeed."), "{:?}", missed.text);
        assert_eq!(missed.moral_standing, None);
    }

    /// Playtest F05. `conversation.md §7.6` "What ASK-WHO prints": the
    /// handler emits the only opening quote before its question, and a
    /// stray lone quote row is the symptom of getting the quotes wrong.
    /// Anthony's `reci` record is a closed quote, two feeds, a `0x8F`
    /// key-wait and the ASK-WHO: the framing's closing quote must not land
    /// after the hold marker, where it became a row of its own.
    #[test]
    fn a_hold_before_ask_who_gets_no_second_closing_quote() {
        let mut response = enc("I can't yet.\"");
        response.extend([
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_WAIT_KEY,
            TLK_CODE_ASK_WHO,
        ]);
        let mut s = label_session(response, Vec::new());
        let out = s.submit_keyword("ask", &ctx());
        assert!(s.awaiting_ask_who());
        let hold = crate::text_hold::TextHoldKind::WaitKey.marker_str();
        assert_eq!(out.text, format!("\n\"I can't yet.\"\n\n{hold}"));
    }

    #[test]
    fn ask_who_rejects_a_hit_inside_a_longer_word() {
        let (raw, decoded) = ask_who_blob();
        let party_names: [&[u8]; 1] = [b"IOLO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.set_npc_slot(Some(6));
        s.present_greeting(&context);
        s.submit_keyword("names", &context);

        let second = s.submit_keyword("triolo", &context);
        assert_eq!(second.asked_who, Some(0));
        assert_eq!(second.branch_flags_set, 0);
    }

    #[test]
    fn ask_who_empty_input_never_sets_the_bit() {
        let (raw, decoded) = ask_who_blob();
        let party_names: [&[u8]; 1] = [b"IOLO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.set_npc_slot(Some(6));
        s.present_greeting(&context);
        s.submit_keyword("names", &context);

        let second = s.submit_keyword("   ", &context);
        assert_eq!(second.asked_who, Some(0));
        assert_eq!(second.branch_flags_set, 0);
    }

    #[test]
    fn ask_who_caps_typed_answer_at_fifteen_bytes() {
        let (raw, decoded) = ask_who_blob();
        let party_names: [&[u8]; 1] = [b"ABCDEFGHIJKLMNO"];
        let context = ConversationContext {
            party_member_names: &party_names,
            ..ctx()
        };
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&context);
        s.submit_keyword("names", &context);

        let second = s.submit_keyword("ABCDEFGHIJKLMNOEXTRA", &context);
        assert_eq!(second.asked_who, Some(1));
        assert_eq!(
            second.text,
            format!("\n\"{}{}", crate::TLK_ASK_WHO_AFFIRMATIVE_LINE, " Done.")
        );
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
        assert_eq!(paid.text, framed_response("A toll. Paid."));
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
        assert_eq!(
            refused.text,
            framed_response(TLK_GOLD_PAYMENT_REFUSAL_MESSAGE)
        );
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

        // `conversation.md` §7: `NAME` runs the Name entry "with the fixed
        // name prefix" - measured 2026-09-10 (`castle-talk/name`) as
        // `My name is `.
        let name = s.submit_keyword("name", &context);
        assert_eq!(
            name.text,
            framed_response(&format!("{TLK_NAME_KEYWORD_PREFIX}Ada"))
        );
        assert!(!name.ended);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingGoldRefusalKeyword
        );

        let nested = s.submit_keyword("", &context);
        assert_eq!(
            nested.text,
            framed_response_after(TLK_EMPTY_INPUT_BYE_MESSAGE, "Farewell.")
        );
        assert!(nested.ended);
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);

        // A response carrying an explicit stop also unwinds immediately and
        // receives no synthetic mandatory-Bye output.
        let mut stopped = ConversationSession::new(raw, decoded);
        stopped.present_greeting(&context);
        stopped.submit_keyword("pay", &context);
        let response = stopped.submit_keyword("help", &context);
        assert_eq!(response.text, framed_response("Nested response."));
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
        assert_eq!(out.text, framed_response(TLK_GOLD_PAYMENT_REFUSAL_MESSAGE));
        assert_eq!(out.moral_standing, None);

        let mut paid = ConversationSession::new(raw, decoded);
        paid.present_greeting(&affordable);
        let out = paid.submit_keyword("pay", &affordable);
        assert_eq!(out.text, framed_response("Paid."));
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
        // A *typed* `BYE` has already echoed itself; `conversation.md §5`
        // step 3 gives the printed `BYE\n\n` header to the empty-input
        // shortcut alone.
        assert!(!out.text.starts_with(TLK_EMPTY_INPUT_BYE_MESSAGE));
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
    fn keyword_alias_runs_the_record_two_further_on_without_reading_input() {
        // `conversation.md §7.6`: `0x87` is positional — skip the rest of
        // this record, any terminators, and the whole record that follows,
        // then run the record after that. There is no keyword comparison
        // and no player input, so a bare "gran" resolves the alias.
        let mut response = enc("Base ");
        response.push(TLK_CODE_KEYWORD_ALIAS);
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

        // The typed line carries no remainder at all; under the
        // withdrawn keyword-scan reading this emitted nothing.
        let out = s.submit_keyword("gran", &ctx());
        // The nested record ends with `0xFF`, which signals stop, so the
        // outer stream stops too and " after." never runs.
        assert_eq!(out.text, framed_response("Base Nested"));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn keyword_alias_restores_the_saved_position_when_the_nested_run_does_not_stop() {
        // §7.6: "If the nested stream signals stop, the outer stream stops
        // too; otherwise the saved position is restored and the outer
        // stream continues where it left off."
        let mut response = enc("Base ");
        response.push(TLK_CODE_KEYWORD_ALIAS);
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
            enc("Nested"),
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

        let out = s.submit_keyword("gran", &ctx());
        assert_eq!(out.text, framed_response("Base Nested after."));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn keyword_alias_chains_through_successive_alias_records() {
        // §7.6: "Aliases chain: three keywords in a row whose responses
        // are each a lone `0x87` all resolve to the fourth keyword's
        // response, because each nested run re-enters the same handler."
        let alias = vec![TLK_CODE_KEYWORD_ALIAS];
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("GRAN"),
            alias.clone(),
            enc("GRAM"),
            alias,
            enc("GRANDPA"),
            enc_with_stop("He is well.", TLK_CODE_END_OF_RESPONSE),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
            "GRAN".to_string(),
            String::new(),
            "GRAM".to_string(),
            String::new(),
            "GRANDPA".to_string(),
            "He is well.".to_string(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());

        let out = s.submit_keyword("gran", &ctx());
        assert_eq!(out.text, framed_response("He is well."));
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
        assert_eq!(out.text, framed_response("Fifteen."));
    }

    #[test]
    fn no_match_keyword_returns_polite_refusal() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("xyzzy", &ctx());
        // The stock capture frames it in quotes (see the arm's comment); the
        // E05 reading that dropped them made `dwelling-talk-after-entry` differ.
        assert_eq!(out.text, framed_response(TLK_NO_KEYWORD_MATCH_MESSAGE));
        assert!(out.text.contains("\"I cannot help thee with that.\""));
        assert!(out.text.ends_with("\n\n"));
    }

    #[test]
    fn reserved_rebuke_keyword_returns_to_prompt_without_ending() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());

        let out = s.submit_keyword("ASS HAT", &ctx());

        // §6: reserved-response framing, then the bounded pause loop.
        assert_eq!(
            out.text,
            "\n\"With language like that, how did you become an Avatar?\"\n\n\u{1}"
        );
        assert!(!out.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    /// u5-spec#298 Q20 (`conversation.md §7.7`, "What a GOTO runs first"): a
    /// label byte inside the head "is a live GOTO at that point, including
    /// one to the block's own label, which restarts the block", as a nested
    /// call whose stop ends the outer one. The shipped name-challenge block
    /// asks ASK-WHO, prints a retry line and GOTOs itself; a wrong name
    /// re-asks rather than opening the scoped prompt.
    #[test]
    fn a_self_goto_in_the_head_restarts_the_block_instead_of_ending_it() {
        let mut response = enc("Listen.");
        response.push(0x93);
        let mut record = vec![TLK_CODE_LABEL_RECORD, 0x93];
        record.extend(enc(" Thy name?"));
        record.push(TLK_CODE_ASK_WHO);
        record.extend(enc(" Nay."));
        record.push(0x93);
        let mut s = label_session(response, vec![enc("OTHER"), enc("x"), record]);

        let out = s.submit_keyword("ask", &ctx());
        assert!(out.text.contains("Listen. Thy name?"), "{:?}", out.text);
        assert!(matches!(s.phase, ConversationSessionPhase::AwaitingAskWho { .. }));

        // A wrong name: the retry line, then the self-GOTO re-runs the head
        // and stops at its ASK-WHO again - no scoped prompt opens.
        let retry = s.submit_keyword("nobody", &ctx());
        assert!(retry.text.contains("Nay."), "{:?}", retry.text);
        assert!(retry.text.contains("Thy name?"), "the block restarts: {:?}", retry.text);
        assert!(
            matches!(s.phase, ConversationSessionPhase::AwaitingAskWho { .. }),
            "{:?}",
            s.phase
        );
    }

    /// A keyword whose response GOTOs a labelled yes/no block shaped like
    /// the shipped ones (playtest F03): the head asks the question, the
    /// record after it is the default arm, then `y` / `n` pairs, then the
    /// blob's final `0x90 0x9F` record.
    fn scoped_yes_no_session() -> ConversationSession {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Shall I join thee?"));
        let mut yes = enc("Lead on!\"");
        yes.push(TLK_CODE_RECRUIT_SPEAKER);
        let mut no = vec![TLK_CODE_STANDING_DOWN];
        no.extend(enc("Pity."));
        let mut final_marker = vec![TLK_CODE_LABEL_RECORD, TLK_LABEL_LAST];
        final_marker.extend(enc("@"));
        label_session(
            vec![0x91],
            vec![
                enc("GRAN"),
                enc("Top-level answer."),
                head,
                enc("Just say when."),
                enc("y"),
                yes,
                enc("n"),
                no,
                final_marker,
            ],
        )
    }

    fn party_ctx<'a>(names: &'a [&'a [u8]]) -> ConversationContext<'a> {
        ConversationContext {
            party_member_names: names,
            moral_standing: 10,
            ..ctx()
        }
    }

    /// Playtest F03. `conversation.md §7.7`: the scoped prompt reads
    /// "another free-text answer ... and then match[es] label-scoped
    /// keywords inside the labelled block. ... This is how an NPC can ask a
    /// follow-up question and route the answer". A `y` answer runs the
    /// block's own `y` record - here the `0x84` that recruits the speaker.
    #[test]
    fn scoped_prompt_matches_the_blocks_own_keyword_records() {
        let names: [&[u8]; 1] = [b"AVATAR"];
        let context = party_ctx(&names);
        let mut s = scoped_yes_no_session();
        let first = s.submit_keyword("ask", &context);
        assert_eq!(first.text, framed_response("Shall I join thee?"));
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        assert_eq!(s.prompt_message(), TLK_KEYWORD_PROMPT);

        let yes = s.submit_keyword("y", &context);
        assert_eq!(yes.text, framed_response("Lead on!\""));
        assert!(yes.recruit_speaker, "the scoped `y` record's 0x84 ran");
        assert!(s.recruit_speaker_pending());
        // §7.7: a NUL-ended scoped response falls back to the ordinary
        // top-level keyword loop.
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);

        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &context);
        let no = s.submit_keyword("n", &context);
        assert_eq!(no.text, framed_response("Pity."));
        assert_eq!(no.moral_standing, Some(9));
        assert!(!no.recruit_speaker);
    }

    /// Playtest F03, engine reading of `OPEN-QUESTIONS.md`'s untraced
    /// default arm: an answer no scoped keyword matches runs the record
    /// right after the head, then the top-level loop takes over - it is
    /// not matched against the top-level keyword list.
    #[test]
    fn scoped_prompt_unmatched_answer_runs_the_default_arm() {
        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &ctx());
        let other = s.submit_keyword("gran", &ctx());
        assert_eq!(other.text, framed_response("Just say when."));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
        let top = s.submit_keyword("gran", &ctx());
        assert_eq!(top.text, framed_response("Top-level answer."));
    }

    /// The shipped "ask again" arm is a lone GOTO to the block's own label:
    /// it re-asks the question and reopens the scoped prompt.
    #[test]
    fn a_default_arm_that_goes_to_its_own_block_asks_again() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Well?"));
        let mut s = label_session(
            vec![0x91],
            vec![head, vec![0x91], enc("y"), enc("Good.")],
        );
        s.submit_keyword("ask", &ctx());
        let again = s.submit_keyword("maybe", &ctx());
        assert!(again.text.contains("Well?"), "{:?}", again.text);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        let yes = s.submit_keyword("y", &ctx());
        assert_eq!(yes.text, framed_response("Good."));
    }

    /// A block with nothing after its head has no records to match, so a
    /// GOTO to it opens no scoped prompt.
    #[test]
    fn a_block_without_records_after_its_head_opens_no_scoped_prompt() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Topic?"));
        let mut s = label_session(vec![0x91], vec![head]);
        let first = s.submit_keyword("ask", &ctx());
        assert_eq!(first.text, framed_response("Topic?"));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    /// `conversation.md §9` step 2 / §7.7: a Description entry that GOTOs a
    /// scoped yes/no block suspends the opening on that prompt; the greeting
    /// must not run over it and drop the answer into the top-level loop
    /// (playtest F03, Fiona).
    #[test]
    fn a_description_goto_into_a_scoped_block_suspends_the_opening() {
        let mut description = enc("a kind woman.");
        description.push(0x91);
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Art thou hungry?"));
        let raw = vec![
            enc("Fia"),
            description,
            enc("Hello."),
            enc("I serve."),
            enc("Bye."),
            enc("y"),
            enc("Top-level y."),
            head,
            enc("Good."),
            enc("y"),
            enc("Eat."),
        ];
        let decoded = raw
            .iter()
            .map(|field| field.iter().map(|b| char::from(b ^ TLK_TEXT_XOR_MASK)).collect())
            .collect();
        let mut s = ConversationSession::new(raw, decoded);
        s.present_description(&ctx());
        assert!(s.opening_suspended(), "{:?}", s.phase);
        assert!(!s.awaiting_ask_who());
        let yes = s.submit_keyword("y", &ctx());
        assert_eq!(yes.text, framed_response("Eat."));
    }

    #[test]
    fn scoped_prompt_empty_input_reissues_without_closing() {
        let mut s = scoped_yes_no_session();
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
    fn scoped_prompt_suppresses_reserved_words() {
        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &ctx());

        // §7.7: `NAME` does not run the global Name response; it is just
        // an answer no scoped keyword matches.
        let name = s.submit_keyword("name", &ctx());
        assert_eq!(name.text, framed_response("Just say when."));
        assert!(!name.text.contains("Ada"));
        assert!(!name.ended);
        let bye = {
            let mut s = scoped_yes_no_session();
            s.submit_keyword("ask", &ctx());
            s.submit_keyword("bye", &ctx())
        };
        assert!(!bye.ended);
        assert!(!bye.text.contains("Farewell"));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    /// u5-spec#298 Q20: "A repeated label is not a boundary." A head that
    /// reaches its own label while it is still running restarts the block
    /// rather than ending there and opening the scoped prompt (the withdrawn
    /// reading). Nothing in this synthetic head stops the stream, so only
    /// the engine's nesting bound ends the loop.
    #[test]
    fn a_repeated_label_in_a_running_head_is_a_goto_not_a_boundary() {
        let mut response = enc("Listen.");
        response.push(0x93);
        let mut record = vec![TLK_CODE_LABEL_RECORD, 0x93];
        record.extend(enc(" Again."));
        record.push(0x93);
        let mut s = label_session(response, vec![enc("OTHER"), enc("x"), record]);

        let out = s.submit_keyword("ask", &ctx());
        assert!(
            out.text.matches("Again.").count() > 1,
            "the self-GOTO restarts the head: {:?}",
            out.text
        );
    }

    /// u5-spec#298 Q20: the declaration search "also accepts the final-marker
    /// value `0x9F` in place of the declaration marker before the label".
    #[test]
    fn the_final_marker_also_declares_a_label() {
        let blob = [0x00, TLK_LABEL_LAST, 0x93, 0xC1];
        assert_eq!(find_label_declaration(&blob, 0x93), Some(3));
    }

    fn label_session(response: Vec<u8>, records: Vec<Vec<u8>>) -> ConversationSession {
        let mut raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc("I mend gear."),
            enc("Farewell."),
            enc("ASK"),
            response,
        ];
        raw.extend(records);
        let decoded = raw
            .iter()
            .map(|field| field.iter().map(|b| (b ^ TLK_TEXT_XOR_MASK) as char).collect())
            .collect();
        let mut s = ConversationSession::new(raw, decoded);
        s.set_npc_slot(Some(3));
        s.present_greeting(&ctx());
        s
    }

    /// Audit A074. `conversation.md §7.6`/§7.7: a taken IF-ELSE "transfer[s]
    /// to the labelled record named by its argument", found by rewinding to
    /// the blob start and scanning for `0x90 <label>` - and shipped
    /// declarations always open their own record, after a NUL.
    #[test]
    fn taken_if_else_lands_in_a_labelled_record_elsewhere_in_the_blob() {
        let mut response = vec![TLK_CODE_IF_ELSE, 0x92];
        response.extend(enc_with_stop("Who art thou?", TLK_CODE_END_OF_RESPONSE));
        let mut record = vec![TLK_CODE_LABEL_RECORD, 0x92];
        record.extend(enc_with_stop("Welcome back, friend.", TLK_CODE_END_OF_RESPONSE));
        let mut s = label_session(response, vec![enc("OTHER"), record]);

        let known = ConversationContext {
            branch_flags: 1 << 3,
            ..ctx()
        };
        let out = s.submit_keyword("ask", &known);
        assert_eq!(out.text, framed_response("Welcome back, friend."));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);

        let stranger = s.submit_keyword("ask", &ctx());
        assert_eq!(stranger.text, framed_response("Who art thou?"));
    }

    /// Audit A074. §7.7: a GOTO "rewinds ... and enters the label handler",
    /// which "resumes the stream immediately past the label byte". §7.6's
    /// routing record is "Reached by a GOTO from a keyword response" and runs
    /// its own welcome line and ASK-WHO, so the labelled record's bytes run.
    #[test]
    fn goto_runs_the_labelled_record_it_lands_on() {
        let mut response = enc("Listen.");
        response.push(0x93);
        let mut record = vec![TLK_CODE_LABEL_RECORD, 0x93];
        record.extend(enc(" Thou art welcome here."));
        record.push(TLK_CODE_ASK_WHO);
        record.extend(enc_with_stop("Good.", TLK_CODE_END_OF_RESPONSE));
        let mut s = label_session(response, vec![enc("OTHER"), enc("x"), record]);

        let out = s.submit_keyword("ask", &ctx());
        assert!(
            out.text.contains("Listen. Thou art welcome here."),
            "{:?}",
            out.text
        );
        let ConversationSessionPhase::AwaitingAskWho { field_idx, .. } = s.phase else {
            panic!("the record's ASK-WHO is reached: {:?}", s.phase);
        };
        assert_eq!(field_idx, 9, "the prompt resumes inside the labelled record");
        let answer = s.submit_keyword("", &ctx());
        assert!(answer.text.contains("Good."), "{:?}", answer.text);
    }

    /// `conversation.md §9` step 2 and §7.7: a Description entry that
    /// GOTOs an ASK-WHO head whose resumed stream reaches `0x82` signals a
    /// stop, and the opener returns - the conversation ends with no Bye and
    /// no keyword prompt. A head that runs on to its NUL keeps the
    /// conversation open. Shape of `town-talk-speaker`'s Britain wizard and
    /// `dwelling-talk-after-entry`'s rat respectively.
    fn opening_ask_who_session(head_tail: Vec<u8>) -> ConversationSession {
        let mut description = enc("a grave wizard.");
        description.push(0x91);
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91, TLK_CODE_ASK_WHO];
        head.extend(head_tail);
        let raw = vec![
            enc("Annon"),
            description,
            Vec::new(),
            enc("I served."),
            enc("Good luck!"),
            head,
        ];
        let decoded = raw
            .iter()
            .map(|field| field.iter().map(|b| char::from(b ^ TLK_TEXT_XOR_MASK)).collect())
            .collect();
        ConversationSession::new(raw, decoded)
    }

    #[test]
    fn an_opening_ask_who_whose_stream_ends_on_end_stream_ends_the_conversation() {
        let mut s = opening_ask_who_session(enc_with_stop("Good day!", TLK_CODE_END_STREAM));
        s.present_description(&ctx());
        assert!(s.opening_suspended(), "{:?}", s.phase);
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(answer.text.ends_with("Good day!"), "{:?}", answer.text);
        assert!(answer.ended, "the Description entry signalled a stop");
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
        assert!(!answer.text.contains("Good luck!"), "no Bye entry runs");
    }

    #[test]
    fn an_opening_ask_who_whose_stream_reaches_its_nul_keeps_the_conversation_open() {
        let mut s = opening_ask_who_session(enc("Squeek..."));
        s.present_description(&ctx());
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(!answer.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn an_opening_ask_who_whose_stream_ends_on_end_of_response_keeps_the_conversation_open() {
        let mut s = opening_ask_who_session(enc_with_stop("Squeek...", TLK_CODE_END_OF_RESPONSE));
        s.present_description(&ctx());
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(!answer.ended, "0xFF returns to the keyword loop");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    #[test]
    fn a_keyword_ask_who_whose_stream_ends_on_end_stream_reprompts() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91, TLK_CODE_ASK_WHO];
        head.extend(enc_with_stop("Good day!", TLK_CODE_END_STREAM));
        let mut s = label_session(vec![0x91], vec![enc("OTHER"), enc("x"), head]);
        s.submit_keyword("ask", &ctx());
        assert!(s.awaiting_ask_who(), "{:?}", s.phase);
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(!answer.ended, "only the opener's stop ends the conversation");
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
