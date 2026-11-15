//! Turn attribution: which record answered each line the player typed
//! (Phase B design 8.3.4; build plan M6-02).
//!
//! The observer in the shell snapshots the conversation before the frame's
//! keys run and diffs the message transcript after them with the shared
//! [`TranscriptCursor`](crate::enhancements::TranscriptCursor). This module
//! turns that diff into [`Turn`]s. It is pure: it reads rows, raw fields and
//! a phase, and calls only [`tlk_keyword_lookup`], [`record_of`] and
//! [`declaration_index`]. It never runs a stream or a session method.
//!
//! - **Echo.** A submitted line is the upper-cased echo the runtime writes
//!   onto the open `:` row of a prompt. A row is an echo when it starts with
//!   that row's text and the row above it is the first row of one of the
//!   engine's prompt constants. The empty line's `BYE` echo looks up as the
//!   Bye entry and is dropped.
//! - **Lookup.** At the top-level prompt the echo goes through
//!   [`tlk_keyword_lookup`], the session's own matcher, and [`record_of`]:
//!   `NAME` is the name-heard mark, `JOB` the Job, a pair its pair; Bye,
//!   rebukes and empty lines are dropped, a miss is kept only with
//!   `clue_misses`.
//! - **Scoped.** At a scoped prompt the turn is `Scoped { decl, arm: 0xFF }`
//!   under the top-level record that opened the block.
//! - **ASK-WHO** answers are never logged.
//! - **Several submits in one frame.** Only the first echo's phase is known
//!   from the snapshot. A later echo is a top-level turn only when the row
//!   above it is the head of [`TLK_KEYWORD_PROMPT`]; under the ASK-WHO
//!   prompt it is dropped; under the scoped prompt it is logged as scoped in
//!   the block of the frame's previous scoped turn, or dropped when that
//!   block is not known (a missing entry beats a wrong one).
//! - **Holds.** A reply split by a pause keeps its turn open while a hold is
//!   pending; the turn closes at the next echo, at the first step with no
//!   hold and no new rows, or when the conversation ends.
//! - **Variant and runic.** The variant hash is taken over the reply's rows
//!   as the transcript holds them (blank rows and prompt rows left out), and
//!   the runic flag is set when any glyph is in the runic font.
//!
//! No row text leaves this module: a [`Turn`] carries the record, a hash,
//! the runic flag and, for a miss only, the bytes the player typed.

use super::dependence::{TestInputs, declaration_index, record_dependence};
use super::{ClueEntry, DialogKey, EntryFlags, GameMinute, NpcKey, RecordAddr, TestBits};
use super::{MISS_MAX_LEN, record_of, variant_hash};
use crate::conversation_session::{ConversationSessionPhase, tlk_keyword_lookup};
use crate::tlk_control_codes::{
    TLK_ASK_WHO_PROMPT, TLK_KEYWORD_PROMPT, TLK_KEYWORD_PROMPT_OPEN_LINE, TLK_SCOPED_PROMPT,
};
use crate::tlk_runner::TlkGlyphFont;
use crate::{Area, MessageEntry, PlayState};

/// The first row of a prompt constant: the row an echo sits under.
fn head(prompt: &'static str) -> &'static str {
    let mut rows = prompt.rsplit('\n');
    rows.next();
    rows.next().unwrap_or("")
}

/// The rows of a prompt above its open `:` row, oldest first.
fn rows_above_open(prompt: &'static str) -> impl DoubleEndedIterator<Item = &'static str> {
    let mut rows: Vec<&'static str> = prompt.split('\n').collect();
    rows.pop();
    rows.into_iter()
}

/// The prompt an echo was typed under, read from the rows above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptSeen {
    TopLevel,
    AskWho,
    Scoped,
}

/// The prompt whose rows end just above `now[index]`, if any. ASK-WHO's
/// prompt ends with the scoped prompt's rows, so it is tried first.
fn prompt_above(now: &[MessageEntry], index: usize) -> Option<PromptSeen> {
    let ends_with = |prompt: &'static str| {
        let rows: Vec<&str> = rows_above_open(prompt).collect();
        index >= rows.len()
            && now[index - rows.len()..index]
                .iter()
                .zip(&rows)
                .all(|(entry, row)| entry.text == *row)
    };
    if ends_with(TLK_ASK_WHO_PROMPT) {
        Some(PromptSeen::AskWho)
    } else if ends_with(TLK_KEYWORD_PROMPT) {
        Some(PromptSeen::TopLevel)
    } else if ends_with(TLK_SCOPED_PROMPT) {
        Some(PromptSeen::Scoped)
    } else {
        None
    }
}

/// The line echoed on `now[index]`, when that row is an echo: the text
/// after the open `:` row's own text, under a prompt's head row.
fn echo_at(now: &[MessageEntry], index: usize) -> Option<&str> {
    let line = now[index].text.strip_prefix(TLK_KEYWORD_PROMPT_OPEN_LINE)?;
    if line.is_empty() || index == 0 {
        return None;
    }
    let above = &now[index - 1].text;
    [head(TLK_KEYWORD_PROMPT), head(TLK_SCOPED_PROMPT)]
        .contains(&above.as_str())
        .then_some(line)
}

/// Whether `entry` is one of the prompts' own rows: never part of a reply.
fn is_prompt_row(entry: &MessageEntry) -> bool {
    entry.text == TLK_KEYWORD_PROMPT_OPEN_LINE
        || entry.text == head(TLK_KEYWORD_PROMPT)
        || entry.text == head(TLK_SCOPED_PROMPT)
}

/// What the observer knows of the conversation for one step: who, the
/// blob, and the phase the session was in before this frame's keys.
#[derive(Clone, Copy, Debug)]
pub struct TurnContext<'a> {
    pub fields: &'a [Vec<u8>],
    pub decoded: &'a [String],
    /// The phase before this frame's keys ran; `None` when no conversation
    /// was open then (the opening frame).
    pub phase: Option<ConversationSessionPhase>,
    /// Keep a miss (`clue_misses`).
    pub misses: bool,
}

/// One attributed turn, closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    pub record: RecordAddr,
    /// For a scoped turn, the top-level record that opened its block, when
    /// it was heard in this session.
    pub parent: Option<RecordAddr>,
    /// [`variant_hash`] of the reply's rows.
    pub variant: u16,
    /// A glyph of the reply is in the runic font.
    pub runic: bool,
    /// What the player typed, at most 15 bytes; only for a miss.
    pub typed: Vec<u8>,
}

impl Turn {
    /// The log entry of this turn, heard by `npc` from `dialog` at `at`:
    /// its test bits from the dependence scan of `fields` on the state
    /// before (`pre`) and after (`post`) the turn (8.3.13), the runic and
    /// reply flags set.
    pub fn entry(
        &self,
        npc: NpcKey,
        dialog: DialogKey,
        fields: &[Vec<u8>],
        pre: &TestInputs,
        post: &TestInputs,
        at: GameMinute,
    ) -> ClueEntry {
        let dep = record_dependence(fields, self.record);
        let mut entry = ClueEntry::heard(
            npc,
            dialog,
            self.record,
            self.variant,
            TestBits::capture(&dep, pre, post),
            at,
        );
        entry.flags.set(EntryFlags::RUNIC, self.runic);
        entry.flags.set(EntryFlags::REPLY, dep.reply());
        entry
    }
}

/// A turn whose reply is still arriving.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenTurn {
    /// `None`: a line that is not logged (Bye, a rebuke, ASK-WHO), still
    /// owning its reply rows so they are not hashed into another turn.
    record: Option<RecordAddr>,
    parent: Option<RecordAddr>,
    rows: Vec<Vec<u8>>,
    runic: bool,
    typed: Vec<u8>,
}

impl OpenTurn {
    fn close(self) -> Option<Turn> {
        let record = self.record?;
        Some(Turn {
            record,
            parent: self.parent,
            variant: variant_hash(self.rows.iter().map(Vec::as_slice)),
            runic: self.runic,
            typed: self.typed,
        })
    }
}

/// The attribution state of one conversation: the open turn and the last
/// top-level record. Session-only, never saved.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnAttributor {
    open: Option<OpenTurn>,
    /// The transcript's last row at the previous step belongs to the open
    /// turn, so an in-place edit of it replaces that row.
    owns_last_row: bool,
    /// The last top-level record heard: the opener of a scoped turn.
    last_top: Option<RecordAddr>,
    /// Turns dropped because a diff did not anchor or a block was unknown.
    dropped: u32,
}

impl TurnAttributor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Turns dropped so far (the debug counter of 8.3.4 item 10).
    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Whether a turn is waiting for more of its reply.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// One step over an anchored diff: `now` is the transcript, `now[first_new..]`
    /// the new rows, `replaced_last` whether the cursor's last row was edited in
    /// place, and `hold_pending` whether a text hold is still waiting after
    /// the frame. Returns the turns that closed.
    pub fn step(
        &mut self,
        ctx: &TurnContext<'_>,
        now: &[MessageEntry],
        first_new: usize,
        replaced_last: bool,
        hold_pending: bool,
    ) -> Vec<Turn> {
        let mut closed = Vec::new();
        if replaced_last
            && self.owns_last_row
            && let Some(open) = self.open.as_mut()
        {
            open.rows.pop();
        }
        let mut first_echo = true;
        let mut frame_block: Option<u8> = None;
        let new_rows = now.len().saturating_sub(first_new);
        self.owns_last_row = false;
        for index in first_new..now.len() {
            if let Some(line) = echo_at(now, index) {
                closed.extend(self.open.take().and_then(OpenTurn::close));
                let seen = if first_echo {
                    ctx.phase
                        .map(Seen::Phase)
                        .or_else(|| prompt_above(now, index).map(Seen::Prompt))
                } else {
                    prompt_above(now, index).map(Seen::Prompt)
                };
                first_echo = false;
                let (record, parent) = self.lookup(ctx, seen, line, &mut frame_block);
                let typed = if record == Some(RecordAddr::Miss) {
                    line.bytes().take(MISS_MAX_LEN).collect()
                } else {
                    Vec::new()
                };
                self.open = Some(OpenTurn {
                    record,
                    parent,
                    rows: Vec::new(),
                    runic: false,
                    typed,
                });
                continue;
            }
            let entry = &now[index];
            if entry.text.is_empty() || is_prompt_row(entry) {
                continue;
            }
            if let Some(open) = self.open.as_mut() {
                open.rows.push(entry.text.as_bytes().to_vec());
                open.runic |= entry
                    .glyphs
                    .iter()
                    .any(|glyph| glyph.font == TlkGlyphFont::Runic);
                self.owns_last_row = index + 1 == now.len();
            }
        }
        if !hold_pending && new_rows == 0 && !replaced_last {
            closed.extend(self.open.take().and_then(OpenTurn::close));
        }
        closed
    }

    /// The diff did not anchor: the open turn is dropped, and so is
    /// anything this frame would have attributed.
    pub fn unanchored(&mut self) {
        if self.open.take().is_some() {
            self.dropped = self.dropped.saturating_add(1);
        }
        self.owns_last_row = false;
    }

    /// The conversation ended (or its state was replaced): the open turn
    /// closes with what it has.
    pub fn finish(&mut self) -> Option<Turn> {
        self.owns_last_row = false;
        self.open.take().and_then(OpenTurn::close)
    }

    /// The record an echo answers, and its parent.
    fn lookup(
        &mut self,
        ctx: &TurnContext<'_>,
        seen: Option<Seen>,
        line: &str,
        frame_block: &mut Option<u8>,
    ) -> (Option<RecordAddr>, Option<RecordAddr>) {
        let top_level = || {
            record_of(
                tlk_keyword_lookup(Some(ctx.fields), ctx.decoded, line),
                ctx.misses,
            )
        };
        match seen {
            Some(Seen::Phase(
                ConversationSessionPhase::AwaitingKeyword
                | ConversationSessionPhase::AwaitingGoldRefusalKeyword,
            ))
            | Some(Seen::Prompt(PromptSeen::TopLevel)) => {
                let record = top_level();
                if let Some(record) = record.filter(|record| record.is_topic()) {
                    self.last_top = Some(record);
                }
                *frame_block = None;
                (record, None)
            }
            Some(Seen::Phase(ConversationSessionPhase::AwaitingScopedKeyword { label })) => {
                let Some(decl) = declaration_index(ctx.fields, label) else {
                    self.dropped = self.dropped.saturating_add(1);
                    return (None, None);
                };
                *frame_block = Some(decl);
                (
                    Some(RecordAddr::Scoped {
                        decl,
                        arm: RecordAddr::ARM_ANSWERED,
                    }),
                    self.last_top,
                )
            }
            Some(Seen::Prompt(PromptSeen::Scoped)) => match *frame_block {
                Some(decl) => (
                    Some(RecordAddr::Scoped {
                        decl,
                        arm: RecordAddr::ARM_ANSWERED,
                    }),
                    self.last_top,
                ),
                None => {
                    self.dropped = self.dropped.saturating_add(1);
                    (None, None)
                }
            },
            // ASK-WHO answers, and anything typed outside a prompt, are
            // never logged.
            _ => (None, None),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Seen {
    Phase(ConversationSessionPhase),
    Prompt(PromptSeen),
}

/// [`TurnAttributor::step`] as one call for a fresh attributor: the turns
/// `now[first_new..]` closes, with no hold pending.
pub fn attribute_turns(ctx: &TurnContext<'_>, now: &[MessageEntry], first_new: usize) -> Vec<Turn> {
    let mut attributor = TurnAttributor::new();
    let mut turns = attributor.step(ctx, now, first_new, false, false);
    turns.extend(attributor.finish());
    turns
}

/// The inline `T<keyword>` route (8.3.4 item 8): talk and ask in one call.
/// The record comes from the session's own matcher on the fields, which do
/// not change during a conversation.
pub fn inline_turn_record(
    fields: &[Vec<u8>],
    decoded: &[String],
    keyword: &str,
    misses: bool,
) -> Option<RecordAddr> {
    record_of(tlk_keyword_lookup(Some(fields), decoded, keyword), misses)
}

/// Who is being spoken to and what the tests read, from `&PlayState`
/// (8.3.4 "Sources read"). `None` without a conversation, outside a
/// town-family scene, or before the runtime has named the slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversationSnapshot {
    pub npc: NpcKey,
    pub dialog: DialogKey,
    pub phase: ConversationSessionPhase,
    pub hold_pending: bool,
    pub inputs: TestInputs,
}

impl ConversationSnapshot {
    pub fn of(state: &PlayState) -> Option<Self> {
        let session = state.active_conversation.as_ref()?;
        let slot = state.active_conversation_npc_slot?;
        let Area::Town { scene, .. } = state.area else {
            return None;
        };
        let family = scene.byte.checked_sub(1).filter(|byte| *byte < 32)? >> 3;
        let dialog_id = state.npcs.iter().find(|npc| npc.slot == slot)?.dialog_id;
        Some(Self {
            npc: NpcKey {
                scene: scene.byte,
                slot: u8::try_from(slot).ok()?,
            },
            dialog: DialogKey { family, dialog_id },
            phase: session.phase,
            hold_pending: state.pending_text_hold.is_some(),
            inputs: test_inputs(state, slot),
        })
    }
}

/// The four inputs of 8.3.13 for the person in `slot`: the acquaintance
/// bit, moral standing, gold and party size.
pub fn test_inputs(state: &PlayState, slot: usize) -> TestInputs {
    let acquainted = match state.area {
        Area::Town { scene, .. } => {
            u8::try_from(slot).is_ok_and(|bit| state.talk_branch_flag_is_set_for_scene(scene, bit))
        }
        _ => false,
    };
    TestInputs {
        acquainted,
        standing: state.moral_standing,
        gold: state.gold,
        party_size: u8::try_from(state.party.len()).unwrap_or(u8::MAX),
    }
}

/// The game's date and time as a [`GameMinute`].
pub fn game_minute(state: &PlayState) -> GameMinute {
    let clock = &state.clock;
    GameMinute::from_calendar(clock.year, clock.month, clock.day, clock.hour, clock.minute)
}

#[cfg(test)]
#[path = "attribute_tests.rs"]
mod tests;
