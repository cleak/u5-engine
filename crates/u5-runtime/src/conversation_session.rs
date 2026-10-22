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
    /// A labelled block's head reached its NUL and opened the scoped
    /// `You respond-` prompt (`conversation.md §7.7` step 2). The five
    /// functional reserved words are inert while this phase is active.
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
    /// The selected response reached `0xFF` (or a passing IF-ELSE with the
    /// reserved `0xFF` argument). `conversation.md §7.2` (u5-spec#299 Q1,
    /// R561/R562): that is **not** a stop - it opens a fresh keyword prompt
    /// nested where the stream stands, and leaving that prompt runs the Bye
    /// entry and ends the conversation - printing the `BYE` line first only
    /// when the player left with an empty line (R579). The session models the
    /// nested prompt as the keyword prompt itself, since every outer level
    /// does nothing after it; on the opener path it means the rest of the
    /// opener never runs.
    pub response_signalled_stop: bool,
    /// The step's stream ended on `0x82` END-STREAM, directly or through a
    /// nested label head or alias. `conversation.md §7.2` (u5-spec#299 Q1,
    /// R560): `0x82` is the plain **stop**, and "every level of the
    /// conversation treats it the same way: the conversation is over" - on
    /// the opener path, in a keyword answer and in a scoped answer, with no
    /// `BYE` and no Bye entry.
    ///
    /// `0xFF` is deliberately not counted: it opens a nested keyword prompt
    /// instead (see [`Self::response_signalled_stop`]), and the dwelling
    /// rat's Description entry, which resumes after its ASK-WHO and ends on
    /// `0xFF`, reaches `Your interest?` in the original
    /// (`dwelling-talk-after-entry`, beat `name`).
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
        // (`thank thee!""`). The quote it would follow may be the framing's
        // own opening one: an empty Job or Bye entry shows a single quote,
        // not `""` (playtest F07, `tlk.md §9.2`).
        if !already_quoted && !self.text.ends_with('"') {
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

    /// The closing half of [`Self::push_framed_response`] - the closing
    /// quote hugging the text, then `§6`'s blank row before the reprompt -
    /// applied to text this output already holds. An ASK-WHO resume prints
    /// its acknowledgement with an opening quote and no closing one
    /// (`conversation.md §7.6`, "What ASK-WHO prints"); when the resumed
    /// stream then reaches a prompt that the keyword path would have framed,
    /// this gives it the same frame.
    fn close_framed_tail(&mut self) {
        let is_hold = |byte: u8| crate::text_hold::TextHoldKind::from_marker(byte).is_some();
        let body_len = self
            .text
            .bytes()
            .rposition(|byte| byte != b'\n' && !is_hold(byte))
            .map_or(0, |index| index + 1);
        // `conversation.md §9`: "The renderer drops a quote that would
        // immediately follow another quote".
        let mut tail_start = body_len;
        if !self.text[..body_len].ends_with('"') {
            self.text.insert(body_len, '"');
            let quote = crate::ordinary_glyphs_from_engine_text("\"");
            let at = body_len.min(self.rendered_glyphs.len());
            self.rendered_glyphs.splice(at..at, quote);
            tail_start += 1;
        }
        let tail_feeds = self.text.as_bytes()[tail_start..]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count();
        if tail_feeds < 2 || !self.text.ends_with(|c: char| is_hold(c as u8)) {
            while !self.text.ends_with("\n\n") {
                self.push_plain_text("\n");
            }
        }
    }

    /// The opening half of [`Self::push_framed_response`] for a response
    /// that reported a `0x82` stop. `conversation.md §7.2` (u5-spec#299 Q1):
    /// "The closing quote and the two row breaks that normally come before
    /// the prompt are added only when a response ends at its NUL", and
    /// inside the Bye entry "a stop only skips the closing quote and the two
    /// row breaks". The stream's own text, trailing feeds included, is kept
    /// as it is.
    fn push_stopped_response(&mut self, rendered: &TlkRenderedText) {
        let already_quoted = rendered.text.trim_start().starts_with('"');
        if self.text.is_empty() || !self.text.ends_with('\n') {
            self.push_plain_text("\n");
        }
        if !already_quoted {
            self.push_plain_text("\"");
        }
        self.text.push_str(&rendered.text);
        self.rendered_glyphs.extend(rendered.glyphs.iter().copied());
    }

    /// `conversation.md §7.6` "What ASK-WHO prints" + §7.7 HEAD: a response
    /// that reaches an ASK-WHO with nothing printed after its last quote - a
    /// scoped answer or Bye entry that is a bare GOTO into a head opening on
    /// IF-ELSE and `0x88` (Judge Dryden's `y`, Jimmy's and Phillip's `bye`) -
    /// leaves that quote as the last printable byte, so the handler's own
    /// opening quote is dropped and the question continues it: one quote, no
    /// row breaks between. The framing above had closed that quote onto a
    /// row of its own and padded a blank under it before the prompt's quote,
    /// a stray lone `"` row (playtest G03). The question's own quote, on the
    /// row the lone quote stood on, is that same single visible quote.
    fn continue_lone_quote_into_ask_who(&mut self) {
        const LONE_QUOTE_TAIL: &str = "\n\"\n\n";
        if !self.text.ends_with(LONE_QUOTE_TAIL)
            || self.rendered_glyphs.len() != self.text.len()
        {
            return;
        }
        let keep = self.text.len() - (LONE_QUOTE_TAIL.len() - 1);
        self.text.truncate(keep);
        self.rendered_glyphs.truncate(keep);
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
    /// `conversation.md §7.7`: what the record paused on the pending `0x88`
    /// ASK-WHO belongs to, when it belongs to a labelled block. Once the
    /// answer resumes it, a head that reaches its NUL opens its scoped
    /// prompt, an answer or default arm that reaches its NUL falls back to
    /// the top-level loop, and either one that stops ends the conversation.
    ask_who_frame: Option<ScopedFrame>,
    /// `conversation.md §9` / §7.7 (u5-spec#299 Q20): the Description entry
    /// ran a GOTO, and "A GOTO in the Description or the Greeting ends the
    /// conversation once its block is done" - so the greeting must not run
    /// after it, whatever the block left open.
    opening_goto: bool,
    /// The pending `0x88` ASK-WHO was reached by the opening's Description
    /// entry (directly or through a label head it transferred to), so the
    /// resumed stream is still the Description entry's: `conversation.md
    /// §9` step 2, "If the Description entry signals a stop, the opener
    /// returns and nothing further is printed."
    opening_ask_who: bool,
    /// The pending `0x88` ASK-WHO was reached by a stranger's
    /// self-introduction (`conversation.md §9` step 3), which "runs the
    /// **Name** entry ... then closes the quote". The close - and the two row
    /// breaks the opener prints after it - is owed once the resumed Name
    /// entry finishes without signalling a stop (playtest F04).
    introduction_ask_who: bool,
    /// `conversation.md §7.7` step 1 / `formats/tlk.md §9.2`: a label head
    /// is running and has emitted nothing since its own HEAD quote. The
    /// head's stream output starts empty, so a GOTO it meets at once must
    /// read the quote from here: the sentinel "suppresses a quote when the
    /// previous printable byte was also a quote".
    head_quote_open: bool,
    /// The Description entry reached `0xFF` (or a passing IF-ELSE `0xFF`)
    /// without suspending: `conversation.md §7.2` (u5-spec#299 Q1, R561)
    /// opens the keyword prompt nested there, so the rest of the opener -
    /// the blank rows, the acquaintance test, the reseed and coin flip and
    /// the greeting - never runs.
    opening_prompt_opened: bool,
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
            ask_who_frame: None,
            opening_goto: false,
            opening_ask_who: false,
            introduction_ask_who: false,
            head_quote_open: false,
            opening_prompt_opened: false,
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
        let (mut out, end) = self.run_field_tracked(1, 0, ctx, 0);
        self.opening_ask_who = self.awaiting_ask_who();
        if end == StreamEnd::Goto {
            // §9 / §7.7: the block the Description jumped to owns what
            // follows. A head or arm that ended in `0xFF` left the nested
            // keyword prompt open, which is §6's loop.
            self.opening_goto = true;
            if self.phase == ConversationSessionPhase::Opened {
                self.phase = ConversationSessionPhase::AwaitingKeyword;
            }
        } else if !self.opening_suspended() {
            // `conversation.md §9` step 2 (u5-spec#299 Q1): "If the
            // Description entry signals a stop, the opener returns and
            // nothing further is printed" - no blank rows, acquaintance
            // test, reseed, coin flip, greeting, keyword prompt, `BYE` or Bye
            // entry; only the final cleanup. A `0xFF` instead opens the
            // keyword prompt right there (§7.2), and the greeting never runs.
            self.end_opener_on_stop(&mut out);
            self.opening_prompt_opened = !out.ended && out.response_signalled_stop;
            if self.opening_prompt_opened {
                self.phase = ConversationSessionPhase::AwaitingKeyword;
            }
        }
        out
    }

    /// `conversation.md §7.2` / §9 (u5-spec#299 Q1, R560): a `0x82` stop
    /// reported by an opener entry - the Description, the Greeting or the
    /// self-introduction's Name run - makes the opener return, and the
    /// conversation is over with only the final cleanup to run.
    fn end_opener_on_stop(&mut self, out: &mut ConversationSessionOutput) {
        if out.end_stream_reached
            && matches!(
                self.phase,
                ConversationSessionPhase::Opened | ConversationSessionPhase::AwaitingKeyword
            )
        {
            self.phase = ConversationSessionPhase::PresentingBye;
            out.ended = true;
        }
    }

    /// The opener has ended the conversation (`0x82` stop on the opener
    /// path); the caller prints what ran, skips the prompt and closes.
    pub fn has_ended(&self) -> bool {
        matches!(
            self.phase,
            ConversationSessionPhase::PresentingBye | ConversationSessionPhase::Closed
        )
    }

    /// Whether the opening has already suspended for input, so the
    /// caller must not run the greeting on top of it.
    ///
    /// A Description entry that GOTOs a labelled block suspends it as well:
    /// `conversation.md §9` / §7.7 (u5-spec#299 Q20), "A GOTO in the
    /// Description or the Greeting ends the conversation once its block is
    /// done", so §9 step 3's greeting or self-introduction never runs after
    /// it (playtest F03, Fiona's food question).
    pub fn opening_suspended(&self) -> bool {
        self.opening_goto
            || self.opening_prompt_opened
            || matches!(
                self.phase,
                ConversationSessionPhase::AwaitingAskWho { .. }
                    | ConversationSessionPhase::AwaitingScopedKeyword { .. }
            )
    }

    pub fn present_greeting(&mut self, ctx: &ConversationContext<'_>) -> ConversationSessionOutput {
        self.phase = ConversationSessionPhase::AwaitingKeyword;
        self.recruit_speaker_pending = false;
        let mut out = self.run_field_from(2, 0, ctx, 0);
        self.end_opener_on_stop(&mut out);
        out
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
        self.introduction_ask_who = self.awaiting_ask_who();
        self.end_opener_on_stop(&mut output);
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
        let mut out = self.submit_keyword_unframed_ask_who(line, ctx);
        if self.awaiting_ask_who() {
            out.continue_lone_quote_into_ask_who();
        }
        out
    }

    fn submit_keyword_unframed_ask_who(
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
                let from_introduction = std::mem::take(&mut self.introduction_ask_who);
                let frame = self.ask_who_frame.take();
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
                let (mut out, end) = self.run_field_tracked(field_idx, cursor, ctx, slot);
                out.branch_flags_set |= written;
                // `conversation.md §7.7`: the resumed record is still the
                // block head, answer or default arm that paused.
                if let Some(frame) = frame {
                    match end {
                        StreamEnd::Nul => self.settle_nul_end(frame),
                        StreamEnd::Suspended if self.ask_who_frame.is_none() => {
                            self.ask_who_frame = Some(frame);
                        }
                        StreamEnd::Stop => self.end_in_block(&mut out),
                        _ => {}
                    }
                }
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
                let stopped =
                    out.response_signalled_stop || out.end_stream_reached || out.ended;
                if from_introduction && self.awaiting_ask_who() {
                    // A second ASK-WHO in the same Name entry: the close is
                    // still owed after it.
                    self.introduction_ask_who = true;
                } else if !stopped
                    && (from_introduction
                        || matches!(
                            self.phase,
                            ConversationSessionPhase::AwaitingScopedKeyword { .. }
                        )
                        || (frame == Some(ScopedFrame::Arm) && end == StreamEnd::Nul))
                {
                    // `conversation.md §9` step 3: the self-introduction's
                    // Name entry ran to its end, so the opener "closes the
                    // quote" and prints its two row breaks - skipped only
                    // "when the entry that ran signals a stop" (playtest F04).
                    //
                    // `§7.7`: a head that reaches its NUL "emit[s] the quote
                    // and two row breaks and go[es] to PROMPT", and an answer
                    // or default arm that does "Emit[s] the quote and two row
                    // breaks" before the top-level loop (Fenelon `wond`,
                    // Anthony `reci`, Sutek, Dryden).
                    out.close_framed_tail();
                }
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
                //
                // u5-spec#299 Q1 (R560/R564) confirms the reading and widens
                // it: a stop ends the conversation on every path, so a
                // keyword answer's ASK-WHO whose resumed stream reaches
                // `0x82` ends it too, with no `BYE`.
                if from_opening && self.awaiting_ask_who() {
                    self.opening_ask_who = true;
                } else if out.end_stream_reached
                    && matches!(self.phase, ConversationSessionPhase::AwaitingKeyword)
                {
                    self.phase = ConversationSessionPhase::PresentingBye;
                    out.ended = true;
                }
                return out;
            }
            ConversationSessionPhase::AwaitingGoldRefusalKeyword => {
                // §7.6: refusal calls the ordinary keyword loop as a nested
                // prompt. Nonterminating turns remain nested; an explicit
                // response stop or the mandatory Bye path unwinds both loops.
                //
                // u5-spec#299 Q1 (R561): "A per-NPC response that signals
                // stop" is a `0x82` stop, which the inner call already turns
                // into `ended`; `0xFF` opens a further nested prompt, which
                // this loop's own reprompt stands for.
                self.phase = ConversationSessionPhase::AwaitingKeyword;
                let mut out = self.submit_keyword(line, ctx);
                if out.ended {
                    self.phase = ConversationSessionPhase::PresentingBye;
                    out.ended = true;
                } else if self.phase == ConversationSessionPhase::AwaitingKeyword {
                    // A GOTO in the nested loop may have opened a block's
                    // scoped prompt or an ASK-WHO; leave those in charge.
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
        // `conversation.md §6` step 6 / §9 step 5 (u5-spec#299 Q1, R564): a
        // keyword answer (or `NAME`/`JOB` response) that reports a `0x82`
        // stop ends the conversation at once, with no `BYE`, no Bye entry
        // and no closing quote or row breaks.
        let stopped = response.end_stream_reached
            && matches!(self.phase, ConversationSessionPhase::AwaitingKeyword);
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
            if stopped {
                out.push_stopped_response(&rendered);
            } else {
                out.push_framed_response(&rendered);
            }
        } else if stopped {
            out.push_stopped_response(&response.rendered_text());
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
        out.end_stream_reached |= response.end_stream_reached;
        out.ended |= response.ended;
        // Empty input or BYE/THANK closes the conversation - unless the Bye
        // entry GOTOs a labelled block that is still asking (`conversation.md
        // §7.7.1`, Gwenno: "leave with an empty line: the Bye entry jumps to
        // block 3, and `Y` there leads to block 4") - and so does a stopped
        // answer (u5-spec#299 Q1, R564).
        if stopped
            || (matches!(
                kind,
                TlkPlayerInputKind::EmptyByeShortcut
                    | TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath),
            ) && matches!(
                self.phase,
                ConversationSessionPhase::AwaitingKeyword | ConversationSessionPhase::PresentingBye
            ))
        {
            self.phase = ConversationSessionPhase::PresentingBye;
            out.ended = true;
        }
        out
    }

    /// Current prompt text for the active outer input loop.
    pub fn prompt_message(&self) -> String {
        match self.phase {
            ConversationSessionPhase::AwaitingKeyword => TLK_KEYWORD_PROMPT.to_string(),
            // `conversation.md §7.7` step 2 (u5-spec#299 Q20, R573): the
            // scoped prompt is `You respond-`, "**not** `Your interest?`".
            ConversationSessionPhase::AwaitingScopedKeyword { .. } => {
                TLK_SCOPED_PROMPT.to_string()
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

    /// Whether the next typed line answers a labelled block's scoped
    /// `You respond-` prompt. `conversation.md §7.7` step 2: an empty line
    /// there only re-asks; it never reaches §6's `BYE` shortcut.
    pub fn awaiting_scoped_keyword(&self) -> bool {
        matches!(
            self.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { .. }
        )
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
        tlk_ordinary_keyword_response_index(Some(&self.fields), &self.decoded_fields, input_upper)
    }

    fn run_field_from(
        &mut self,
        field_idx: usize,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_who_response: u8,
    ) -> ConversationSessionOutput {
        self.run_field_tracked(field_idx, start, ctx, ask_who_response)
            .0
    }

    /// [`Self::run_field_from`], also reporting how the stream ended.
    fn run_field_tracked(
        &mut self,
        field_idx: usize,
        start: usize,
        ctx: &ConversationContext<'_>,
        ask_who_response: u8,
    ) -> (ConversationSessionOutput, StreamEnd) {
        let (_, field_starts) = self.blob_view();
        let Some(&field_start) = field_starts.get(field_idx) else {
            return (ConversationSessionOutput::default(), StreamEnd::Nul);
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
    ) -> (ConversationSessionOutput, StreamEnd) {
        let (blob, field_starts) = self.blob_view();
        let mut out = ConversationSessionOutput::default();
        let mut cursor = start;
        let end;
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
                        let outer_head_quote_open = self.head_quote_open;
                        self.head_quote_open &= out.text.is_empty();
                        let (nested, nested_end) = self.run_field_tracked(alias_target, 0, ctx, 0);
                        self.head_quote_open = outer_head_quote_open;
                        // "If the nested stream signals stop, the outer
                        // stream stops too; otherwise the saved position is
                        // restored."
                        // u5-spec#299 Q1 (R560): `0x82` is the plain stop;
                        // `0xFF`'s nested prompt also never returns to here.
                        let nested_signalled_stop = nested.response_signalled_stop
                            || nested.end_stream_reached
                            || nested.ended;
                        merge_session_output(&mut out, nested);
                        if nested_signalled_stop
                            || !matches!(self.phase, ConversationSessionPhase::AwaitingKeyword)
                        {
                            end = nested_end;
                            break;
                        }
                    }
                    cursor = resume;
                }
                TlkRunStop::LabelTransfer(label) => {
                    self.follow_label_transfer(&blob, label, run.consumed, ctx, &mut out);
                    end = StreamEnd::Goto;
                    break;
                }
                TlkRunStop::RecruitingSpeaker(resume) => {
                    // `conversation.md §7.6`'s `0x84` row (u5-spec#299 Q20):
                    // "The party-full refusal does **not** stop; the stream
                    // runs on to its NUL" - `absorb_recruit_speaker` has
                    // printed it. Otherwise the recruit "reports a stop: the
                    // conversation ends at once with no `BYE` and no Bye
                    // entry"; so does the no-match diagnostic, whose text is
                    // not published (the caller prints none).
                    if !out.recruit_speaker {
                        cursor = resume;
                        continue;
                    }
                    self.phase = ConversationSessionPhase::PresentingBye;
                    out.ended = true;
                    end = StreamEnd::Stop;
                    break;
                }
                stop => {
                    end = StreamEnd::of_runner_stop(stop);
                    break;
                }
            }
        }
        (out, end)
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
    /// withdrawn (R574). `from` is kept for the caller's record position only.
    ///
    /// u5-spec#299 Q20 publishes the rest as a state machine (§7.7 HEAD):
    /// the head reaching its NUL opens the scoped prompt; a head that stops
    /// finishes the block, and "Every way out of the machine reports a stop
    /// to whatever ran the GOTO".
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
        if self.label_records_running.len() >= TLK_LABEL_TRANSFER_DEPTH_LIMIT {
            return;
        }
        // u5-spec#299 Q20, §7.7 step 1 (HEAD): "Emit the quote, then run the
        // head through the byte runner." The quote is the ordinary sentinel
        // (`formats/tlk.md §9.2`), dropped after another quote. `out` holds
        // only the calling stream's own text: when that is empty and the
        // caller is itself a head that jumped at once, the quote before it
        // is that head's (Gorn's block 3 head jumps straight to block 4).
        let after_quote = if out.text.is_empty() {
            self.head_quote_open
        } else {
            out.text.ends_with('"')
        };
        if !after_quote {
            out.push_plain_text("\"");
        }
        self.label_records_running.push((label, resume));
        let outer_head_quote_open = std::mem::replace(&mut self.head_quote_open, true);
        let (nested, end) = self.run_blob_from(resume, ctx, 0);
        self.head_quote_open = outer_head_quote_open;
        self.label_records_running.pop();
        merge_session_output(out, nested);
        match end {
            // "If the head reaches its NUL, emit the quote and two row
            // breaks and go to PROMPT."
            StreamEnd::Nul => {
                out.close_framed_tail();
                self.settle_nul_end(ScopedFrame::Head(label));
            }
            StreamEnd::Suspended => self.ask_who_frame = Some(ScopedFrame::Head(label)),
            // "If the head stops (`0x82`, a successful `0x84`, a GOTO, ...),
            // the block is finished and reports a stop. No prompt opens."
            // A GOTO has already run its own block; `0xFF` and the gold
            // refusal left their nested keyword prompt open.
            StreamEnd::Stop => self.end_in_block(out),
            StreamEnd::Goto | StreamEnd::NestedPrompt => {}
        }
    }

    /// `conversation.md §7.7`: a block record that reached its NUL. A head
    /// opens the scoped prompt (step 2); an answer or default arm falls back
    /// to "a fresh top-level keyword loop" (step 5), which this session's
    /// ordinary keyword phase is. "When the player leaves it, it runs the Bye
    /// entry ... The block then reports a stop", which ends the
    /// conversation as leaving the outer loop would.
    fn settle_nul_end(&mut self, frame: ScopedFrame) {
        self.phase = match frame {
            ScopedFrame::Head(label) => ConversationSessionPhase::AwaitingScopedKeyword { label },
            ScopedFrame::Arm => ConversationSessionPhase::AwaitingKeyword,
        };
    }

    /// `conversation.md §7.7`: a block record that stopped. "Every way out
    /// of the machine reports a stop to whatever ran the GOTO", and a GOTO
    /// "is always the last thing its calling stream does", so the
    /// conversation ends with no `BYE` and no Bye entry (§7.2).
    fn end_in_block(&mut self, out: &mut ConversationSessionOutput) {
        self.phase = ConversationSessionPhase::PresentingBye;
        out.ended = true;
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

    /// `conversation.md §7.7` (u5-spec#299 Q20), the scoped state machine's
    /// PROMPT, RESERVED and SCOPED steps for one typed line.
    ///
    /// - An empty line prints `"What didst thou say?` and repeats PROMPT.
    /// - RESERVED: "`NAME`, `JOB`, `WORK`, `BYE` and `THANK` do nothing here.
    ///   The scan moves on, and the typed line is left intact"; "A rebuke
    ///   word prints the rebuke and runs its bounded pause loop exactly as at
    ///   the top level ..., then the block restarts at HEAD".
    /// - SCOPED: the block's keyword/answer pairs in blob order, first match
    ///   wins; "**No match:** emit the quote, then run the **default arm**".
    ///   A record that reaches its NUL goes to FALLBACK, the top-level loop;
    ///   one that stops ends the conversation.
    ///
    /// *Corrected:* the engine's earlier reading ran the default arm the
    /// same way but printed `BYE` for an empty line and prompted with
    /// `Your interest?` (R573, R575).
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
            // `conversation.md §7.7` step 2 (u5-spec#299 Q20, R575): "print
            // two row breaks, `"What didst thou say?` ..., and two row
            // breaks, then repeat PROMPT. The head is **not** re-run, nothing
            // ends, and no `BYE` is printed." The first break closes the
            // `:` row the caller already committed.
            let mut out = ConversationSessionOutput::default();
            out.set_plain_text(&format!("\n{TLK_SCOPED_EMPTY_INPUT_LINE}\n\n"));
            self.phase = ConversationSessionPhase::AwaitingScopedKeyword { label };
            return out;
        }

        self.phase = ConversationSessionPhase::AwaitingKeyword;
        if reserved_rebuke_keyword_index(&input_upper).is_some() {
            // Step 3: the rebuke as at the top level, then HEAD again: "the
            // head is printed again and a fresh scoped prompt opens".
            let mut out = ConversationSessionOutput::default();
            out.set_framed_text(TLK_RESERVED_REBUKE_MESSAGE);
            out.push_plain_text(crate::text_hold::TextHoldKind::TimedPause.marker_str());
            let (blob, _) = self.blob_view();
            self.follow_label_transfer(&blob, label, 0, ctx, &mut out);
            return out;
        }

        let block = self.scoped_block(label);
        let field_idx = block.as_ref().and_then(|block| {
            block
                .keywords
                .iter()
                .find(|(keyword_idx, _)| {
                    let keyword = self.fields[*keyword_idx]
                        .iter()
                        .map(|b| b & 0x7F)
                        .collect::<Vec<_>>();
                    tlk_keyword_matches(keyword.trim_ascii(), &input_upper)
                })
                .map(|(_, response_idx)| *response_idx)
                .or(block.default_arm)
        });
        let (response, end) = match field_idx {
            Some(field_idx) => self.run_field_tracked(field_idx, 0, ctx, 0),
            // No record follows the head: an empty default arm, which
            // reaches its NUL at once. Not a shipped shape (§7.7's census).
            None => (ConversationSessionOutput::default(), StreamEnd::Nul),
        };
        // `conversation.md §7.2` (u5-spec#299 Q1): "if a scoped yes/no
        // question's 'no' answer ends in `0x82`, answering 'no' puts the
        // player straight back in the command loop" - no `BYE`, no Bye entry,
        // and no closing quote or row breaks.
        let stopped = end == StreamEnd::Stop && response.end_stream_reached;
        let mut out = ConversationSessionOutput::default();
        // "emit the quote, then run" the record; on its NUL "Emit the quote
        // and two row breaks" - §6's ordinary response framing.
        if stopped {
            out.push_stopped_response(&response.rendered_text());
        } else {
            out.push_framed_response(&response.rendered_text());
        }
        let text = std::mem::take(&mut out.text);
        let glyphs = std::mem::take(&mut out.rendered_glyphs);
        merge_session_output(&mut out, response);
        out.text = text;
        out.rendered_glyphs = glyphs;
        match end {
            StreamEnd::Nul => self.settle_nul_end(ScopedFrame::Arm),
            StreamEnd::Suspended => self.ask_who_frame = Some(ScopedFrame::Arm),
            StreamEnd::Stop => self.end_in_block(&mut out),
            StreamEnd::Goto | StreamEnd::NestedPrompt => {}
        }
        out
    }

    /// The records of a labelled block after its head.
    ///
    /// `conversation.md §7.7` (u5-spec#299 Q20), "Block layout": "A labelled
    /// block runs from its declaration to the next declaration. Its records,
    /// each ending at a NUL, are in order: 1. the **head** ...; 2. the
    /// **default arm** - the one record immediately after the head. It has
    /// no keyword; 3. zero or more **keyword/answer pairs** ... paired by
    /// position only". "A label byte repeated inside a block is not a record
    /// separator" (R574). An unpaired trailing keyword record (the shipped
    /// `DWELLING.TLK` id 15 block 3) is ignored. "The record scans also
    /// treat the final-marker value `0x9F` as a record end, but no shipped
    /// head or default arm contains that value", so records end at their NUL
    /// here.
    fn scoped_block(&self, label: u8) -> Option<ScopedLabelBlock> {
        let (blob, field_starts) = self.blob_view();
        let resume = find_label_declaration(&blob, label)?;
        let (head_idx, _) = Self::field_at(&field_starts, resume);
        let records_end = (head_idx + 1..self.fields.len())
            .find(|idx| self.fields[*idx].first() == Some(&TLK_CODE_LABEL_RECORD))
            .unwrap_or(self.fields.len());
        let default_arm = head_idx + 1;
        if default_arm >= records_end {
            return Some(ScopedLabelBlock {
                default_arm: None,
                keywords: Vec::new(),
            });
        }
        let keywords = (default_arm + 1..records_end)
            .step_by(2)
            .filter(|keyword_idx| keyword_idx + 1 < records_end)
            .map(|keyword_idx| (keyword_idx, keyword_idx + 1))
            .collect();
        Some(ScopedLabelBlock {
            default_arm: Some(default_arm),
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

/// How one session stream ended, as `conversation.md §7.7`'s state machine
/// needs to know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamEnd {
    /// Reached its NUL (or the end of the loaded blob).
    Nul,
    /// Paused on a `0x88` ASK-WHO prompt.
    Suspended,
    /// Ran a GOTO. The target block has already settled what comes next,
    /// and "a block always reports a stop".
    Goto,
    /// Stopped on `0xFF` (or the unaffordable `0x85`), which leaves a nested
    /// keyword prompt open (§7.2); also the malformed-stream stops, kept
    /// conservative.
    NestedPrompt,
    /// Stopped on `0x82`, or on a successful `0x84`.
    Stop,
}

impl StreamEnd {
    fn of_runner_stop(stop: TlkRunStop) -> Self {
        match stop {
            TlkRunStop::NulTerminator
            | TlkRunStop::Exhausted
            | TlkRunStop::PausedAt(_)
            | TlkRunStop::WaitingKey(_) => Self::Nul,
            TlkRunStop::AskingWho(_) => Self::Suspended,
            TlkRunStop::LabelTransfer(_) => Self::Goto,
            TlkRunStop::EndOfStream | TlkRunStop::RecruitingSpeaker(_) => Self::Stop,
            TlkRunStop::EndOfResponse
            | TlkRunStop::GoldPaymentRefused { .. }
            | TlkRunStop::MalformedIntroducer(_)
            | TlkRunStop::UnresolvedGotoLabel(_)
            | TlkRunStop::KeywordAlias(_) => Self::NestedPrompt,
        }
    }
}

/// Which labelled-block record a paused stream belongs to (`conversation.md
/// §7.7`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopedFrame {
    /// The head of the block with this label.
    Head(u8),
    /// A scoped answer or the default arm.
    Arm,
}

/// The records after a labelled block's head, as fields of the session's
/// blob: see [`ConversationSession::scoped_block`].
struct ScopedLabelBlock {
    /// The record right after the head; `None` when the next declaration
    /// follows the head at once.
    default_arm: Option<usize>,
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
/// What `conversation.md §6` steps 3 to 5 select for one typed line at the
/// top-level keyword prompt, before anything runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlkKeywordLookup {
    /// Step 3: an empty line takes the shortcut to the Bye entry.
    EmptyLine,
    /// Step 4: a rebuke word from the fixed table.
    Rebuke,
    /// The record that answers: the Name (0), Job (3) or Bye (4) entry for a
    /// functional reserved word, otherwise an ordinary keyword's response.
    Field(usize),
    /// Step 7: neither scan matched; `I cannot help thee with that.`
    NoMatch,
}

/// `conversation.md §6` steps 2 to 5 as one lookup: the line is capped at
/// the reader's fifteen characters, the reserved table is scanned first,
/// then the ordinary pairs up to the first label declaration, all with the
/// shared word-start matcher ([`tlk_keyword_matches`], §6 step 5 and §7.7).
/// This is the session's own top-level matcher; the inline `T<keyword>`
/// route asks it too, so both answer the same lines. `raw_fields` locates
/// the declarations; without it (a string-only blob) the pair walk runs to
/// the end of the blob.
pub fn tlk_keyword_lookup(
    raw_fields: Option<&[Vec<u8>]>,
    decoded_fields: &[String],
    line: &str,
) -> TlkKeywordLookup {
    let input_upper: Vec<u8> = capped_tlk_input_bytes(line)
        .iter()
        .map(|b| (*b & 0x7F).to_ascii_uppercase())
        .collect();
    match tlk_player_input_kind(&input_upper) {
        TlkPlayerInputKind::EmptyByeShortcut => TlkKeywordLookup::EmptyLine,
        TlkPlayerInputKind::ReservedRebuke { .. } => TlkKeywordLookup::Rebuke,
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::NameEntry) => TlkKeywordLookup::Field(0),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::JobEntry) => TlkKeywordLookup::Field(3),
        TlkPlayerInputKind::Reserved(ReservedKeywordEffect::ByePath) => TlkKeywordLookup::Field(4),
        TlkPlayerInputKind::OrdinaryKeywordScan => {
            tlk_ordinary_keyword_response_index(raw_fields, decoded_fields, &input_upper)
                .map_or(TlkKeywordLookup::NoMatch, TlkKeywordLookup::Field)
        }
    }
}

/// `conversation.md §6` step 5's ordinary pair walk: the response record of
/// the first keyword accepted, or `None`.
fn tlk_ordinary_keyword_response_index(
    raw_fields: Option<&[Vec<u8>]>,
    decoded_fields: &[String],
    input_upper: &[u8],
) -> Option<usize> {
    let raw_len = raw_fields.map_or(decoded_fields.len(), <[Vec<u8>]>::len);
    // `conversation.md §4` (issue #299 Q22, `RETRACTIONS.md` R582): pairs
    // are found by record position alone - keyword k is record 2k+5, its
    // response record 2k+6 - and continue only up to the blob's first
    // label declaration; nothing inside or after a labelled block is
    // tested at the top-level prompt (§6 step 5), so a scoped `y` typed
    // here gets the cannot-help line. A keyword record just before the
    // declaration keeps the declaration record as its response: that is
    // `TOWNE.TLK` id 23's `Don't ask!` (§7.7, R583). The declaration
    // record itself, when it falls in a keyword position, never matches,
    // so stopping before it loses nothing.
    let first_declaration = (TLK_LEADING_ENTRY_COUNT..raw_len)
        .find(|idx| raw_fields.and_then(|raw| raw[*idx].first()) == Some(&TLK_CODE_LABEL_RECORD))
        .unwrap_or(raw_len);
    let mut idx = TLK_LEADING_ENTRY_COUNT;
    if idx % 2 == 0 {
        idx += 1;
    }
    while idx < first_declaration && idx + 1 < decoded_fields.len() {
        let keyword_field = &decoded_fields[idx];
        let keyword = keyword_field.trim().as_bytes();
        if tlk_keyword_matches(keyword, input_upper) {
            return Some(idx + 1);
        }
        idx += 2;
    }
    None
}

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

    /// [`enc_with_stop`] with each `\n` written as `0x8D` LITERAL-NEWLINE,
    /// the stream's own line feed (a plain encoded `\n` is `0x8A`).
    fn enc_nl_with_stop(text: &str, stop: u8) -> Vec<u8> {
        let mut bytes: Vec<u8> = text
            .bytes()
            .map(|b| {
                if b == b'\n' {
                    TLK_CODE_LITERAL_NEWLINE
                } else {
                    b ^ TLK_TEXT_XOR_MASK
                }
            })
            .collect();
        bytes.push(stop);
        bytes
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

    /// Playtest F07. `conversation.md §9`: "The renderer drops a quote that
    /// would immediately follow another quote", so an empty Job or Bye
    /// entry renders one quote, not `""`.
    #[test]
    fn an_empty_entry_renders_a_single_quote() {
        let raw = vec![enc("Fl"), enc("a mage"), enc("Hi."), Vec::new(), Vec::new()];
        let decoded = vec![
            "Fl".to_string(),
            "a mage".to_string(),
            "Hi.".to_string(),
            String::new(),
            String::new(),
        ];
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());
        let job = s.submit_keyword("job", &ctx());
        assert_eq!(job.text, "\n\"\n\n");
        let bye = s.submit_keyword("", &ctx());
        assert_eq!(bye.text, format!("{TLK_EMPTY_INPUT_BYE_MESSAGE}\"\n\n"));
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
        assert!(out.recruit_speaker);
        assert!(s.recruit_speaker_pending());
        // u5-spec#299 Q20, §7.6's `0x84` row: "A successful recruit reports
        // a stop: the conversation ends at once with no `BYE` and no Bye
        // entry" - the rest of the stream does not run.
        assert_eq!(out.text, framed_response("I shall come."));
        assert!(out.ended);
        assert!(!out.text.contains("Farewell"));
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
        assert_eq!(s.prompt_message(), "");
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
        // u5-spec#299 Q20: "The party-full refusal does **not** stop; the
        // stream runs on" and the conversation goes on.
        assert!(out.text.contains(" Lead on."), "{:?}", out.text);
        assert!(!out.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
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
        // The recruit ended the conversation; a later step reports nothing.
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
            enc("WHOM"),
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
            "WHOM".to_string(),
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

        let first = s.submit_keyword("whom", &context);
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

    /// rc3 residual (Fenelon `wond`, Anthony `reci`, Sutek, Dryden):
    /// an ASK-WHO answer that resumes into a GOTO whose labelled head reaches
    /// its NUL opens the scoped `You respond-` prompt, after "the quote and
    /// two row breaks" of `§7.7` step 1.
    #[test]
    fn ask_who_resuming_into_a_scoped_head_frames_it_like_a_keyword_response() {
        let mut response = enc("Who?");
        response.push(TLK_CODE_ASK_WHO);
        response.extend([TLK_CODE_LITERAL_NEWLINE, TLK_CODE_LITERAL_NEWLINE, 0x91]);
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Wilt thou learn?"));
        let names: [&[u8]; 1] = [b"AVATAR"];
        let context = party_ctx(&names);
        let mut s = label_session(
            response,
            vec![head, enc("Later then."), enc("y"), enc("Listen.")],
        );
        s.submit_keyword("ask", &context);
        assert!(s.awaiting_ask_who());
        let answered = s.submit_keyword("Avatar", &context);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        // §7.7 step 1 (u5-spec#299 Q20): the HEAD emits its own quote
        // before the head and the quote and two row breaks after it.
        assert_eq!(
            answered.text,
            format!(
                "\n\"{}\n\n\"Wilt thou learn?\"\n\n",
                crate::TLK_ASK_WHO_AFFIRMATIVE_LINE
            )
        );
        assert_eq!(answered.rendered_text().glyphs.len(), answered.text.len());

        // An ASK-WHO resume that simply returns to the top-level loop keeps
        // §7.6's contract: the stream supplies what follows.
        let mut plain = enc("Who?");
        plain.push(TLK_CODE_ASK_WHO);
        plain.extend(enc(" Done."));
        let mut s = label_session(plain, Vec::new());
        s.submit_keyword("ask", &context);
        let answered = s.submit_keyword("Avatar", &context);
        assert_eq!(
            answered.text,
            format!("\n\"{} Done.", crate::TLK_ASK_WHO_AFFIRMATIVE_LINE)
        );
    }

    /// Playtest F04 follow-up. `conversation.md §9` step 3: the stranger
    /// "runs the **Name** entry ... then closes the quote", and the closing
    /// quote "and its two row breaks are skipped [only] when the entry that
    /// ran signals a stop". A Name entry that pauses on ASK-WHO owes them
    /// once the answer resumes it to its end.
    #[test]
    fn a_self_introduction_ask_who_answer_closes_the_introduction() {
        let mut name = enc("Grum\"");
        name.extend([
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_IF_ELSE,
            0xFF,
            TLK_CODE_ASK_WHO,
        ]);
        let raw = vec![
            name,
            enc("a knight."),
            enc("Well met."),
            enc("I guard."),
            enc("Farewell."),
        ];
        let decoded = raw
            .iter()
            .map(|field| field.iter().map(|b| char::from(b ^ TLK_TEXT_XOR_MASK)).collect())
            .collect();
        let mut s = ConversationSession::new(raw, decoded);
        s.set_npc_slot(Some(1));
        s.present_stranger_opening(&ctx(), true);
        assert!(s.awaiting_ask_who());
        let answered = s.submit_keyword("nobody", &ctx());
        assert_eq!(
            answered.text,
            format!("\n\"{}\"\n\n", crate::TLK_ASK_WHO_DISMISSIVE_LINE)
        );
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
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
        s.submit_keyword("whom", &context);

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
        s.submit_keyword("whom", &context);

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
        s.submit_keyword("whom", &context);

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
            enc_nl_with_stop("Nested response.\n", TLK_CODE_END_STREAM),
            enc("MORE"),
            enc_nl_with_stop("More.\"\n\n", TLK_CODE_END_OF_RESPONSE),
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
            "Nested response.\n".to_string(),
            "MORE".to_string(),
            "More.\"\n\n".to_string(),
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

        // A response carrying an explicit stop (`0x82`) also unwinds
        // immediately and receives no synthetic mandatory-Bye output.
        // u5-spec#299 Q1 (R561): `0xFF` is not that stop - it opens a further
        // nested prompt, so the refusal loop keeps prompting.
        let mut stopped = ConversationSession::new(raw, decoded);
        stopped.present_greeting(&context);
        stopped.submit_keyword("pay", &context);
        let more = stopped.submit_keyword("more", &context);
        assert!(!more.ended);
        assert_eq!(
            stopped.phase,
            ConversationSessionPhase::AwaitingGoldRefusalKeyword
        );
        let response = stopped.submit_keyword("help", &context);
        assert_eq!(response.text, "\n\"Nested response.\n");
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
        let out = s.submit_keyword("i want grandpa", &ctx());
        assert!(out.text.contains("Short"));
    }

    /// u5-spec#299 Q20 (R577): "A typed `grandpa` matches a `gran` keyword,
    /// so when an NPC has both `gran` and `grandpa`, which of them answers is
    /// decided by their order in the blob: the earlier one wins for any line
    /// it matches."
    #[test]
    fn the_earlier_keyword_wins_for_any_line_it_matches() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        let out = s.submit_keyword("grandpa", &ctx());
        assert_eq!(out.text, framed_response("Short answer."));
        // `JOBS` runs the Job entry (R578).
        let job = s.submit_keyword("jobs", &ctx());
        assert_eq!(job.text, framed_response("I mend gear."));
    }

    /// `conversation.md §7.2` / §6 step 6 / §9 step 5 (u5-spec#299 Q1,
    /// R560/R564): `0x82` is a stop and "every level of the conversation
    /// treats it the same way: the conversation is over". A keyword answer
    /// that stops ends it with no `BYE`, no Bye entry and no closing quote
    /// or row breaks; one that reaches its NUL reprompts. This test pinned
    /// the withdrawn "terminate the fixed leading entries without ending the
    /// whole conversation" reading.
    #[test]
    fn end_stream_ends_the_conversation_and_nul_reprompts() {
        let raw = vec![
            enc("Ada"),
            enc("a quiet smith"),
            enc("Greetings."),
            enc_nl_with_stop("I mend gear.\n", TLK_CODE_END_STREAM),
            enc("Farewell."),
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

        let ordinary = s.submit_keyword("trade", &ctx());
        assert!(ordinary.text.contains("Bring iron."));
        assert!(!ordinary.ended);
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);

        let job = s.submit_keyword("job", &ctx());
        assert_eq!(job.text, "\n\"I mend gear.\n", "no closing quote or row breaks");
        assert!(job.ended);
        assert!(!job.text.contains("BYE") && !job.text.contains("Farewell"));
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
    }

    /// `conversation.md §9` step 2 (u5-spec#299 Q1): after an opener stop
    /// the opener returns; a Description that reaches `0x82` ends the
    /// conversation, and one that reaches `0xFF` opens the keyword prompt
    /// right there so the greeting never runs.
    #[test]
    fn a_description_stop_ends_the_opener_and_its_ff_opens_the_prompt() {
        let session = |description: Vec<u8>| {
            let raw = vec![
                enc("Ada"),
                description,
                enc("Greetings."),
                enc("I mend gear."),
                enc("Farewell."),
            ];
            let decoded = raw
                .iter()
                .map(|field| field.iter().map(|b| char::from(b ^ TLK_TEXT_XOR_MASK)).collect())
                .collect();
            ConversationSession::new(raw, decoded)
        };

        let mut stopped = session(enc_nl_with_stop("a rat.\n", TLK_CODE_END_STREAM));
        let out = stopped.present_description(&ctx());
        assert!(out.ended);
        assert!(stopped.has_ended());

        let mut nested = session(enc_nl_with_stop("a rat.\n\n", TLK_CODE_END_OF_RESPONSE));
        let out = nested.present_description(&ctx());
        assert!(!out.ended);
        assert!(nested.opening_suspended(), "the greeting must not run");
        assert_eq!(nested.phase, ConversationSessionPhase::AwaitingKeyword);
        assert_eq!(nested.prompt_message(), TLK_KEYWORD_PROMPT);
        let bye = nested.submit_keyword("", &ctx());
        assert!(bye.ended && bye.text.contains("Farewell."));

        let mut plain = session(enc("a rat."));
        plain.present_description(&ctx());
        assert!(!plain.opening_suspended() && !plain.has_ended());
    }

    /// `conversation.md §7.2` (u5-spec#299 Q1): a scoped answer ending in
    /// `0x82` "puts the player straight back in the command loop", and a
    /// labelled block whose head stops opens no scoped prompt (§7.7, "when
    /// the head stops").
    #[test]
    fn a_stopped_scoped_answer_or_head_ends_the_conversation() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Art thou sure?"));
        let mut s = label_session(
            vec![0x91],
            vec![
                head,
                enc("Hm."),
                enc("N"),
                enc_nl_with_stop("Then go.\n", TLK_CODE_END_STREAM),
            ],
        );
        s.submit_keyword("ask", &ctx());
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        let no = s.submit_keyword("n", &ctx());
        assert!(no.ended, "{:?}", no.text);
        assert!(no.text.ends_with("Then go.\n"), "{:?}", no.text);
        assert!(!no.text.contains("Farewell") && !no.text.contains("BYE"));
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);

        let mut stopped_head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        stopped_head.extend(enc_nl_with_stop("Begone!\n", TLK_CODE_END_STREAM));
        let mut s = label_session(
            vec![0x91],
            vec![stopped_head, enc("Hm."), enc("N"), enc("Then go.")],
        );
        let out = s.submit_keyword("ask", &ctx());
        assert!(out.ended, "{:?}", s.phase);
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
    }

    #[test]
    fn ordinary_keyword_short_match_picks_first_match() {
        let mut s = baseline_session();
        s.present_greeting(&ctx());
        // "GRAN" comes first in the blob, so it answers every line it
        // matches (`conversation.md §6`, u5-spec#299 Q20).
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
        // §7.7 step 1: the HEAD emits its quote before the head runs.
        assert!(out.text.contains("Listen.\" Thy name?"), "{:?}", out.text);
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

    /// Playtest F03. `conversation.md §7.7` step 4 (u5-spec#299 Q20): "Test
    /// the block's keyword/answer pairs in blob order ...; the first keyword
    /// accepted wins." A `y` answer runs the block's own `y` record - here
    /// the `0x84` that recruits the speaker.
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
        // u5-spec#299 Q20 (R573): `You respond-`, not `Your interest?`.
        assert_eq!(s.prompt_message(), TLK_SCOPED_PROMPT);

        let yes = s.submit_keyword("y", &context);
        assert_eq!(yes.text, framed_response("Lead on!\""));
        assert!(yes.recruit_speaker, "the scoped `y` record's 0x84 ran");
        assert!(s.recruit_speaker_pending());
        // u5-spec#299 Q20 (§7.7, "Recruitment inside a scoped answer"): "A
        // successful `0x84` stops: the conversation ends at once with no
        // `BYE` and no Bye entry".
        assert!(yes.ended);
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);

        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &context);
        let no = s.submit_keyword("n", &context);
        assert_eq!(no.text, framed_response("Pity."));
        assert_eq!(no.moral_standing, Some(9));
        assert!(!no.recruit_speaker);
    }

    /// Playtest F03, `conversation.md §7.7` (u5-spec#299 Q20, R576): "No
    /// scoped match runs the block's default arm" - the record right after
    /// the head - and only its NUL leads to the top-level loop; the line is
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

    /// `conversation.md §6` step 5: the ordinary walk "ends at the blob's
    /// first label declaration", so a scoped keyword is not a top-level one -
    /// `y` there gets `I cannot help thee with that.` - and under the
    /// word-start match (R577) a scoped `n` does not answer `NEWS`.
    #[test]
    fn the_ordinary_keyword_walk_stops_at_the_first_label_declaration() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Well?"));
        let mut s = label_session(
            vec![0x91],
            vec![
                head,
                enc("Default."),
                enc("n"),
                enc("No answer."),
                enc("y"),
                enc("Yes answer."),
            ],
        );
        for line in ["y", "news"] {
            let out = s.submit_keyword(line, &ctx());
            assert_eq!(out.text, framed_response(TLK_NO_KEYWORD_MATCH_MESSAGE), "{line}");
            assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
        }
    }

    /// u5-spec#299 Q20, §7.7's executed matcher table against a block whose
    /// keywords are `n` then `y`.
    #[test]
    fn the_scoped_match_follows_the_published_matcher_table() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Well?"));
        let session = || {
            label_session(
                vec![0x91],
                vec![
                    head.clone(),
                    enc("Default."),
                    enc("n"),
                    enc("No answer."),
                    enc("y"),
                    enc("Yes answer."),
                ],
            )
        };
        for (line, record) in [
            ("Y", "Yes answer."),
            ("YES", "Yes answer."),
            ("YONDER", "Yes answer."),
            ("GO YES", "Yes answer."),
            ("NAY", "No answer."),
            ("NO", "No answer."),
            ("NAME", "No answer."),
            ("OKAY", "Default."),
            ("MAYBE", "Default."),
            ("BYE", "Default."),
            ("MAYBE YES", "Default."),
        ] {
            let mut s = session();
            s.submit_keyword("ask", &ctx());
            let out = s.submit_keyword(line, &ctx());
            assert_eq!(out.text, framed_response(record), "{line}");
            // An answer that reaches its NUL falls back to the top level.
            assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword, "{line}");
            assert_eq!(s.prompt_message(), TLK_KEYWORD_PROMPT, "{line}");
        }
    }

    /// u5-spec#299 Q20, §7.7 step 1 with `formats/tlk.md §9.2`: each HEAD
    /// emits the quote sentinel, which is dropped after another quote. A
    /// head that jumps at once (Gorn's block 3 head "jumps straight to block
    /// 4", §7.7.1) prints nothing between its quote and the next head's, so
    /// one quote shows, not `""`.
    #[test]
    fn a_head_that_jumps_at_once_shows_a_single_quote() {
        let mut first = vec![TLK_CODE_LABEL_RECORD, 0x91];
        first.extend(enc("Art thou ready?"));
        let second = vec![TLK_CODE_LABEL_RECORD, 0x92, 0x93];
        let mut third = vec![TLK_CODE_LABEL_RECORD, 0x93];
        third.extend(enc("Then come."));
        let mut s = label_session(
            vec![0x91],
            vec![
                first,
                enc("Pity."),
                enc("y"),
                vec![0x92],
                second,
                enc("Unreached."),
                third,
                enc("Pity."),
            ],
        );
        s.submit_keyword("ask", &ctx());
        let yes = s.submit_keyword("y", &ctx());
        assert_eq!(yes.text, framed_response("Then come."));
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x93 }
        );
    }

    /// u5-spec#299 Q20, §7.7 step 4: "If the record that ran stops, the block
    /// reports a stop (the conversation ends ...)". Gorn's blocks 1 and 2
    /// "accept only `y` and their default arms end the conversation".
    #[test]
    fn a_scoped_record_that_stops_ends_the_conversation() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Wilt thou help?"));
        let session = || {
            label_session(
                vec![0x91],
                vec![
                    head.clone(),
                    enc_with_stop("Then begone!", TLK_CODE_END_STREAM),
                    enc("y"),
                    enc_with_stop("Good. Go now.", TLK_CODE_END_STREAM),
                ],
            )
        };
        for line in ["no", "y"] {
            let mut s = session();
            s.submit_keyword("ask", &ctx());
            let out = s.submit_keyword(line, &ctx());
            assert!(out.ended, "{line}");
            assert!(!out.text.contains("Farewell"), "no Bye entry: {:?}", out.text);
            assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
        }
    }

    /// u5-spec#299 Q20, §7.7 step 3: "A rebuke word prints the rebuke and
    /// runs its bounded pause loop exactly as at the top level ..., then the
    /// block restarts at HEAD: the head is printed again and a fresh scoped
    /// prompt opens."
    #[test]
    fn a_rebuke_at_the_scoped_prompt_restarts_the_block_head() {
        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &ctx());
        let out = s.submit_keyword("ass", &ctx());
        let pause = crate::text_hold::TextHoldKind::TimedPause.marker_str();
        assert_eq!(
            out.text,
            format!(
                "\n\"With language like that, how did you become an Avatar?\"\n\n{pause}\"Shall I join thee?\"\n\n"
            )
        );
        assert!(!out.ended);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        assert_eq!(s.prompt_message(), TLK_SCOPED_PROMPT);
    }

    /// u5-spec#299 Q20, §7.7.1 Gwenno: "leave with an empty line: the Bye
    /// entry jumps to block 3, and `Y` there leads to block 4". The Bye
    /// entry's GOTO hands the conversation to that block's prompt.
    #[test]
    fn a_bye_entry_goto_into_a_scoped_block_keeps_the_conversation_open() {
        let mut bye = enc("Wait!");
        bye.push(0x91);
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Shall I come?"));
        let raw = vec![
            enc("Gwenno"),
            enc("a bard."),
            enc("Hello."),
            enc("I sing."),
            bye,
            enc("x"),
            enc("y"),
            head,
            enc("Pity."),
            enc("y"),
            enc("Splendid."),
        ];
        let decoded = raw
            .iter()
            .map(|field| field.iter().map(|b| char::from(b ^ TLK_TEXT_XOR_MASK)).collect())
            .collect();
        let mut s = ConversationSession::new(raw, decoded);
        s.present_greeting(&ctx());
        let out = s.submit_keyword("", &ctx());
        assert!(out.text.contains("Shall I come?"), "{:?}", out.text);
        assert!(!out.ended);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        let yes = s.submit_keyword("y", &ctx());
        assert_eq!(yes.text, framed_response("Splendid."));
    }

    /// u5-spec#299 Q20, `conversation.md §9` / §7.7: "A GOTO in the
    /// Description or the Greeting ends the conversation once its block is
    /// done". A Description whose block head ends in `0xFF` (the shipped
    /// `Greetings, <name>."` heads) leaves the nested keyword prompt open;
    /// the Greeting entry must not run after it.
    #[test]
    fn a_description_goto_whose_head_opens_a_nested_prompt_suspends_the_greeting() {
        let mut description = enc("a jester.");
        description.push(0x91);
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc_with_stop("Greetings again.\"", TLK_CODE_END_OF_RESPONSE));
        let raw = vec![
            enc("Chuckles"),
            description,
            enc("Hello."),
            enc("I jest."),
            enc("Bye."),
            enc("x"),
            enc("y"),
            head,
        ];
        let decoded = raw
            .iter()
            .map(|field| field.iter().map(|b| char::from(b ^ TLK_TEXT_XOR_MASK)).collect())
            .collect();
        let mut s = ConversationSession::new(raw, decoded);
        let out = s.present_description(&ctx());
        assert_eq!(out.text, "a jester.\"Greetings again.\"");
        assert!(s.opening_suspended());
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
        let job = s.submit_keyword("job", &ctx());
        assert_eq!(job.text, framed_response("I jest."));
    }

    /// `conversation.md §4` and §6 step 5 (issue #299 Q22, `RETRACTIONS.md`
    /// R582): the top-level walk ends at the blob's first label declaration,
    /// so "a keyword that lives inside a labelled block, such as a scoped
    /// `y`, gets `I cannot help thee with that.` at this prompt".
    #[test]
    fn a_scoped_keyword_typed_at_the_top_level_prompt_gets_the_cannot_help_line() {
        let mut s = scoped_yes_no_session();
        let out = s.submit_keyword("y", &ctx());
        assert!(out.text.contains("I cannot help thee with that."), "{:?}", out.text);
        assert!(!out.recruit_speaker, "the scoped `y` record must not run");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    /// `conversation.md §7.7` (issue #299 Q22, `RETRACTIONS.md` R583): the
    /// `TOWNE.TLK` id 23 shape. One extra record before the first
    /// declaration shifts every later pair, so `caug` and `tort` sit in
    /// response positions and never match, and the last keyword-position
    /// record, `Don't ask!`, takes the declaration record as its response:
    /// the marker prints as codepoint `0x10` and the label byte after it is
    /// a live GOTO into block 1, whose head opens its scoped prompt.
    #[test]
    fn an_odd_record_count_shifts_the_pairs_and_dont_ask_runs_the_declaration() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Hmph."));
        let mut final_marker = vec![TLK_CODE_LABEL_RECORD, TLK_LABEL_LAST];
        final_marker.extend(enc("@"));
        let records = vec![
            enc("An extra line."),
            enc("CAUG"),
            enc("Caught answer."),
            enc("TORT"),
            enc("Don't ask!"),
            head,
            enc("Well."),
            enc("y"),
            enc("Yes."),
            final_marker,
        ];
        // The helper's leading `ASK` keyword would answer `DON'T ASK!` under
        // the word-start matcher (§6 step 5, R577: `ASK` follows a literal
        // space), so this blob's first keyword is one the line cannot hit.
        let mut s = label_session(enc("Asked."), records.clone());
        s.fields[TLK_LEADING_ENTRY_COUNT] = enc("QQQ");
        s.decoded_fields[TLK_LEADING_ENTRY_COUNT] = "QQQ".to_string();
        for dead in ["caug", "tort"] {
            let out = s.submit_keyword(dead, &ctx());
            assert!(out.text.contains("I cannot help thee with that."), "{dead}: {:?}", out.text);
        }
        let out = s.submit_keyword("DON'T ASK!", &ctx());
        assert!(
            out.text.contains("\u{10}\"Hmph."),
            "the marker glyph, then block 1's HEAD quote (§7.7 step 1) and head: {:?}",
            out.text
        );
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
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

    /// u5-spec#299 Q20, §7.7 step 1: "If the head reaches its NUL, emit the
    /// quote and two row breaks and go to PROMPT" - whatever follows it. With
    /// nothing after the head (not a shipped shape: the census's 53 such
    /// heads all stop first) every answer runs an empty default arm, which
    /// reaches its NUL and falls back to the top-level loop.
    #[test]
    fn a_head_that_reaches_its_nul_opens_the_scoped_prompt_even_with_no_records() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Topic?"));
        let mut s = label_session(vec![0x91], vec![head]);
        let first = s.submit_keyword("ask", &ctx());
        assert_eq!(first.text, framed_response("Topic?"));
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        let answer = s.submit_keyword("anything", &ctx());
        assert!(!answer.ended);
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

    /// u5-spec#299 Q20 (R575): an empty line at the scoped prompt prints
    /// `"What didst thou say?` between two pairs of row breaks and re-asks
    /// `You respond-` without the head; no `BYE`.
    #[test]
    fn scoped_prompt_empty_input_reissues_without_closing() {
        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &ctx());

        let empty = s.submit_keyword("", &ctx());
        assert_eq!(empty.text, "\n\"What didst thou say?\n\n");
        assert!(!empty.text.contains("BYE"));
        assert!(!empty.text.contains("Shall I join thee?"));
        assert!(!empty.ended);
        assert_eq!(s.prompt_message(), "You respond-\n:");
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
    }

    #[test]
    fn scoped_prompt_suppresses_reserved_words() {
        let mut s = scoped_yes_no_session();
        s.submit_keyword("ask", &ctx());

        // §7.7: `NAME` does not run the global Name response; the line is
        // left intact, so it matches the scoped `n` as text (u5-spec#299
        // Q20: "`NAME` matches a scoped keyword `n`").
        let name = s.submit_keyword("name", &ctx());
        assert_eq!(name.text, framed_response("Pity."));
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
        // §7.2: a stream ending in `0xFF` "must carry its own" closing quote
        // and row breaks; shipped streams do.
        let mut record = vec![TLK_CODE_LABEL_RECORD, 0x92];
        record.extend(enc_with_stop(
            "Welcome back, friend.\"",
            TLK_CODE_END_OF_RESPONSE,
        ));
        let mut s = label_session(response, vec![enc("OTHER"), record]);

        let known = ConversationContext {
            branch_flags: 1 << 3,
            ..ctx()
        };
        let out = s.submit_keyword("ask", &known);
        assert_eq!(out.text, framed_response("Welcome back, friend.\""));
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);

        let stranger = s.submit_keyword("ask", &ctx());
        assert_eq!(stranger.text, framed_response("Who art thou?"));
    }

    /// u5-spec#299 Q20, §7.7: "IF/ELSE branches reuse the same machinery",
    /// and for the head it leaves "an IF-ELSE that transfers" is a stop. The
    /// shipped stranger blocks (Q21) ask ASK-WHO and then `0x8C` to another
    /// block: a match must run *that* block's head and open *its* scoped
    /// prompt, not the first block's.
    #[test]
    fn a_taken_if_else_in_a_head_runs_the_target_block_and_its_prompt() {
        let mut first = vec![TLK_CODE_LABEL_RECORD, 0x91];
        first.extend(enc("Thy name?\""));
        // The shipped shape: ASK-WHO, the quote it discards, two feeds, then
        // the IF-ELSE (`DWELLING.TLK` id 10).
        first.push(TLK_CODE_ASK_WHO);
        first.extend(enc("\""));
        first.extend([
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_LITERAL_NEWLINE,
            TLK_CODE_IF_ELSE,
            0x92,
            TLK_CODE_END_STREAM,
        ]);
        let mut second = vec![TLK_CODE_LABEL_RECORD, 0x92];
        second.extend(enc("Art thou the Avatar?"));
        let names: [&[u8]; 1] = [b"AVATAR"];
        let context = party_ctx(&names);
        let session = || {
            label_session(
                vec![0x91],
                vec![
                    first.clone(),
                    enc("First default."),
                    second.clone(),
                    enc("Second default."),
                    enc("y"),
                    enc("Second yes."),
                ],
            )
        };
        let mut s = session();
        s.submit_keyword("ask", &context);
        assert!(s.awaiting_ask_who());
        let matched = s.submit_keyword("avatar", &context);
        assert_eq!(
            matched.text,
            format!(
                "\n\"{}\n\n\"Art thou the Avatar?\"\n\n",
                crate::TLK_ASK_WHO_AFFIRMATIVE_LINE
            )
        );
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x92 }
        );
        let yes = s.submit_keyword("y", &context);
        assert_eq!(yes.text, framed_response("Second yes."));

        // No match: the IF-ELSE falls through to `0x82`, which ends it.
        let mut s = session();
        s.submit_keyword("ask", &context);
        let missed = s.submit_keyword("nobody", &context);
        assert!(missed.ended);
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
        // §7.7 step 1 (u5-spec#299 Q20): "Emit the quote, then run the head".
        assert!(
            out.text.contains("Listen.\" Thou art welcome here."),
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

    /// u5-spec#299 Q20, §7.7 step 1: the resumed head is still a head, so
    /// reaching its NUL opens the block's scoped prompt.
    #[test]
    fn an_opening_ask_who_whose_head_reaches_its_nul_opens_the_scoped_prompt() {
        let mut s = opening_ask_who_session(enc("Squeek..."));
        s.present_description(&ctx());
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(!answer.ended);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        assert_eq!(s.prompt_message(), TLK_SCOPED_PROMPT);
    }

    #[test]
    fn an_opening_ask_who_whose_stream_ends_on_end_of_response_keeps_the_conversation_open() {
        let mut s = opening_ask_who_session(enc_with_stop("Squeek...", TLK_CODE_END_OF_RESPONSE));
        s.present_description(&ctx());
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(!answer.ended, "0xFF opens a nested keyword prompt");
        assert_eq!(s.phase, ConversationSessionPhase::AwaitingKeyword);
    }

    /// u5-spec#299 Q20, §7.7 step 1: "If the head stops (`0x82`, ...), the
    /// block is finished and reports a stop", and the GOTO that ran it was
    /// the last thing its keyword answer did - the conversation ends with no
    /// `BYE` and no Bye entry. The resumed head is still that head.
    /// u5-spec#299 Q1 (R560/R564): a stop ends the conversation on every
    /// path, not only on the opener's. (Was
    /// `a_keyword_ask_who_whose_stream_ends_on_end_stream_reprompts`, which
    /// pinned the withdrawn reprompt.)
    #[test]
    fn a_keyword_goto_head_that_stops_after_ask_who_ends_the_conversation() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91, TLK_CODE_ASK_WHO];
        head.extend(enc_with_stop("Good day!", TLK_CODE_END_STREAM));
        let mut s = label_session(vec![0x91], vec![enc("OTHER"), enc("x"), head]);
        s.submit_keyword("ask", &ctx());
        assert!(s.awaiting_ask_who(), "{:?}", s.phase);
        let answer = s.submit_keyword("NAME", &ctx());
        assert!(answer.ended, "a stop ends the conversation");
        assert!(!answer.text.contains("Farewell"));
        assert_eq!(s.phase, ConversationSessionPhase::PresentingBye);
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

    /// Playtest G03. `conversation.md §7.7`: a non-empty scoped line prints
    /// two row breaks, the matched answer "emit[s] the quote", and a bare GOTO
    /// into a head opening on IF-ELSE and `0x88` emits the HEAD quote (dropped
    /// after the first) before ASK-WHO's own opening quote (dropped too,
    /// §7.6). One quote, no rows between it and the question: the answer row,
    /// a blank, then `"What is thy name?"`. The engine drew a lone `"` row and
    /// a blank above the question's quote (Judge Dryden's `inqu`/`y`; Jimmy's
    /// and Phillip's Bye entries take the same shape through the keyword
    /// path).
    #[test]
    fn a_bare_goto_into_an_ask_who_head_continues_the_answer_quote() {
        let mut head = vec![TLK_CODE_LABEL_RECORD, 0x91];
        head.extend(enc("Art thou here to confess?"));
        let mut ask = vec![
            TLK_CODE_LABEL_RECORD,
            0x92,
            TLK_CODE_IF_ELSE,
            0x93,
            TLK_CODE_ASK_WHO,
        ];
        ask.extend(enc("\"Done."));
        let names: [&[u8]; 1] = [b"AVATAR"];
        let context = party_ctx(&names);
        let mut s = label_session(
            vec![0x91],
            vec![head, enc("Later then."), enc("y"), vec![0x92], ask.clone(), enc("x")],
        );
        s.submit_keyword("ask", &context);
        assert_eq!(
            s.phase,
            ConversationSessionPhase::AwaitingScopedKeyword { label: 0x91 }
        );
        let answered = s.submit_keyword("y", &context);
        assert!(s.awaiting_ask_who());
        assert_eq!(answered.text, "\n");
        assert_eq!(answered.rendered_text().glyphs.len(), answered.text.len());

        // The keyword path: a response that is a bare GOTO to the same head.
        let mut s = label_session(vec![0x92], vec![enc("x"), ask]);
        let asked = s.submit_keyword("ask", &context);
        assert!(s.awaiting_ask_who());
        assert_eq!(asked.text, "\n");
    }
}
