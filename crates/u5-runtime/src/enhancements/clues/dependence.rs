//! When an answer can change (Phase B design 8.3.13, U14).
//!
//! The byte runner has two branch tests (`0x8C`, the acquaintance bit, and
//! `0xFE`, moral standing at or above a threshold) and two codes whose text
//! depends on state (`0x85` gold, `0x84` party room); a follow-up prompt
//! or an `0x88` makes the answer depend on the player's reply
//! (`formats/tlk.md §9, §9.1`; `systems/conversation.md §7.6, §7.7`).
//!
//! [`record_dependence`] walks the bytes an answer can run without new
//! input and lists the tests it meets. It follows the format's rules and
//! runs nothing: no session method, no stream runner, no state. The
//! thresholds and amounts it reads come from the player's own file at
//! runtime; they are compared here and never stored.
//!
//! [`asked_state`] is the display rule (step 3): an asked record is
//! "same" only when the player already heard the branch the person would
//! give now; otherwise, or when the scan cannot tell, "may differ".

use std::collections::HashSet;

use super::{ClueEntry, RecordAddr};

/// More tests than this in reach gives [`Dependence::Unknown`].
pub const MAX_TESTS: usize = 8;

const CODE_RECRUIT: u8 = 0x84;
const CODE_GOLD: u8 = 0x85;
const CODE_ACTION: u8 = 0x86;
const CODE_ALIAS: u8 = 0x87;
const CODE_ASK_WHO: u8 = 0x88;
const CODE_IF_KNOWN: u8 = 0x8C;
const CODE_END_STREAM: u8 = 0x82;
const CODE_DECLARATION: u8 = 0x90;
const CODE_IF_STANDING: u8 = 0xFE;
const CODE_END_RESPONSE: u8 = 0xFF;
/// `0x8C 0xFF`: a nested keyword prompt, not a label.
const OPERAND_PROMPT: u8 = 0xFF;

fn is_label(byte: u8) -> bool {
    (0x91..=0x9F).contains(&byte)
}

/// One test an answer meets, in scan order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Test {
    /// `0x8C`: this person has been told the party's name.
    Acquainted,
    /// `0xFE t`: moral standing at or above `t`.
    Standing(u8),
    /// `0x85 nnn`: the party holds at least `nnn` gold.
    Gold(u16),
    /// `0x84`: the party has room (below six).
    PartyRoom,
}

/// What an answer's reach depends on (8.3.13 step 1).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Dependence {
    Known {
        /// At most [`MAX_TESTS`], in scan order.
        tests: Vec<Test>,
        /// The answer asks something back: a follow-up prompt or `0x88`.
        reply: bool,
        /// An `0x86` is in reach.
        action: bool,
        /// A label transfer or branch target is in reach.
        branches: bool,
    },
    /// Too many tests, a missing label or a malformed record.
    Unknown,
}

impl Dependence {
    /// No tests and no reply: the text is fixed (step 3, first row).
    pub fn is_fixed(&self) -> bool {
        matches!(self, Dependence::Known { tests, reply: false, .. } if tests.is_empty())
    }

    /// 8.3.6: a record rendered statically after a relaunch has no tests,
    /// no reply, no `0x86` and no label code.
    pub fn renders_statically(&self) -> bool {
        matches!(
            self,
            Dependence::Known {
                tests,
                reply: false,
                action: false,
                branches: false,
            } if tests.is_empty()
        )
    }

    pub fn tests(&self) -> &[Test] {
        match self {
            Dependence::Known { tests, .. } => tests,
            Dependence::Unknown => &[],
        }
    }

    pub fn reply(&self) -> bool {
        matches!(self, Dependence::Known { reply: true, .. })
    }
}

/// The four inputs the tests read (8.3.13 step 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TestInputs {
    /// This slot's acquaintance bit (`talk_branch_flags`).
    pub acquainted: bool,
    pub standing: u8,
    pub gold: u16,
    pub party_size: u8,
}

/// The party is full at six (`conversation.md §7.6`).
const PARTY_CAP: u8 = 6;

/// The outcome bits of `tests` on `inputs`: bit `i` set when test `i`
/// passes. At most [`MAX_TESTS`] tests are evaluated.
pub fn evaluate(tests: &[Test], inputs: &TestInputs) -> u8 {
    tests
        .iter()
        .take(MAX_TESTS)
        .enumerate()
        .fold(0, |bits, (i, test)| {
            let pass = match *test {
                Test::Acquainted => inputs.acquainted,
                Test::Standing(threshold) => inputs.standing >= threshold,
                Test::Gold(amount) => inputs.gold >= amount,
                Test::PartyRoom => inputs.party_size < PARTY_CAP,
            };
            bits | (u8::from(pass) << i)
        })
}

/// The display state of an asked record (8.3.13 step 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AskedState {
    Same,
    MayDiffer,
}

/// Step 3 for the entries heard of one record of one person, whose reach
/// is `dep`, on the state `now`. With no entry the record was not asked,
/// and the safe answer is [`AskedState::MayDiffer`].
pub fn asked_state(entries: &[ClueEntry], dep: &Dependence, now: &TestInputs) -> AskedState {
    if entries.is_empty() {
        return AskedState::MayDiffer;
    }
    let Dependence::Known { tests, reply, .. } = dep else {
        return AskedState::MayDiffer;
    };
    if *reply {
        return AskedState::MayDiffer;
    }
    if tests.is_empty() {
        return AskedState::Same;
    }
    let bits = evaluate(tests, now);
    let heard = entries.iter().any(|entry| {
        !entry.tests.is_unknown()
            && usize::from(entry.tests.n) == tests.len()
            && entry.tests.pre == bits
            && entry.tests.post == bits
    });
    if heard {
        AskedState::Same
    } else {
        AskedState::MayDiffer
    }
}

/// The label declarations of a blob, in blob order: `(field index,
/// label)` for each record that starts with `0x90 <label>`.
fn declarations(fields: &[Vec<u8>]) -> Vec<(usize, u8)> {
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| match field.as_slice() {
            [CODE_DECLARATION, label, ..] if is_label(*label) => Some((index, *label)),
            _ => None,
        })
        .collect()
}

/// How many labelled blocks a blob declares.
pub fn block_count(fields: &[Vec<u8>]) -> usize {
    declarations(fields).len()
}

/// `RecordAddr::Scoped::decl` for a block entered by `label` (8.3.4 step
/// 4): the position, among the declarations in blob order, of the first
/// one with that label, which is the one a transfer finds.
pub fn declaration_index(fields: &[Vec<u8>], label: u8) -> Option<u8> {
    declarations(fields)
        .iter()
        .position(|(_, l)| *l == label)
        .and_then(|index| u8::try_from(index).ok())
}

/// Step 1: the reach of `record` in a blob's raw fields (bit-7 encoded,
/// NUL-split, as `parse_tlk_blob_fields_raw` gives them).
pub fn record_dependence(fields: &[Vec<u8>], record: RecordAddr) -> Dependence {
    let mut walk = Walk::new(fields);
    match record {
        RecordAddr::Leading(_) | RecordAddr::Pair(_) => {
            let Some(index) = record.field().filter(|index| *index < fields.len()) else {
                return Dependence::Unknown;
            };
            walk.record(index, 0);
        }
        RecordAddr::Scoped { decl, .. } => {
            if usize::from(decl) >= walk.decls.len() {
                return Dependence::Unknown;
            }
            walk.block(usize::from(decl));
        }
        RecordAddr::Miss => return Dependence::Unknown,
    }
    walk.finish()
}

/// How a scanned stream ended.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// It ran to its NUL.
    ReachedNul,
    /// A stop, a nested prompt or a transfer.
    Stopped,
}

struct Walk<'a> {
    fields: &'a [Vec<u8>],
    decls: Vec<(usize, u8)>,
    tests: Vec<Test>,
    reply: bool,
    action: bool,
    branches: bool,
    unknown: bool,
    records: HashSet<usize>,
    blocks: HashSet<usize>,
}

impl<'a> Walk<'a> {
    fn new(fields: &'a [Vec<u8>]) -> Self {
        Self {
            fields,
            decls: declarations(fields),
            tests: Vec::new(),
            reply: false,
            action: false,
            branches: false,
            unknown: false,
            records: HashSet::new(),
            blocks: HashSet::new(),
        }
    }

    fn finish(self) -> Dependence {
        if self.unknown || self.tests.len() > MAX_TESTS {
            Dependence::Unknown
        } else {
            Dependence::Known {
                tests: self.tests,
                reply: self.reply,
                action: self.action,
                branches: self.branches,
            }
        }
    }

    fn test(&mut self, test: Test) {
        self.tests.push(test);
        if self.tests.len() > MAX_TESTS {
            self.unknown = true;
        }
    }

    /// Scans record `index` from byte `skip`. Each record is scanned once.
    fn record(&mut self, index: usize, skip: usize) -> Flow {
        if self.unknown || !self.records.insert(index) {
            return Flow::Stopped;
        }
        let Some(field) = self.fields.get(index) else {
            self.unknown = true;
            return Flow::Stopped;
        };
        let bytes = field.get(skip..).unwrap_or(&[]);
        let mut at = 0;
        while at < bytes.len() && !self.unknown {
            let code = bytes[at];
            let args = &bytes[at + 1..];
            match code {
                CODE_GOLD => {
                    let Some(amount) = args.get(..3).and_then(gold_amount) else {
                        self.unknown = true;
                        break;
                    };
                    self.test(Test::Gold(amount));
                    at += 4;
                }
                CODE_ACTION => {
                    if args.is_empty() {
                        self.unknown = true;
                        break;
                    }
                    self.action = true;
                    at += 2;
                }
                CODE_IF_KNOWN => {
                    let Some(&target) = args.first() else {
                        self.unknown = true;
                        break;
                    };
                    self.test(Test::Acquainted);
                    // The operand is a target, not a GOTO: the stream falls
                    // through when the bit is clear. `0xFF` opens a prompt.
                    if target != OPERAND_PROMPT {
                        self.label(target);
                    }
                    at += 2;
                }
                CODE_IF_STANDING => {
                    let [threshold, target, ..] = *args else {
                        self.unknown = true;
                        break;
                    };
                    self.test(Test::Standing(threshold));
                    self.label(target);
                    at += 3;
                }
                CODE_RECRUIT => {
                    self.test(Test::PartyRoom);
                    at += 1;
                }
                CODE_ASK_WHO => {
                    self.reply = true;
                    at += 1;
                }
                CODE_ALIAS => {
                    // The nested record's stop would end this stream too;
                    // scanning on only adds tests, the safe direction.
                    let Some(target) = self.alias_target(index) else {
                        self.unknown = true;
                        break;
                    };
                    self.record(target, 0);
                    at += 1;
                }
                label if is_label(label) => {
                    // A live GOTO: the stream continues in that block and
                    // never comes back.
                    self.label(label);
                    return Flow::Stopped;
                }
                CODE_END_RESPONSE | CODE_END_STREAM => return Flow::Stopped,
                _ => at += 1,
            }
        }
        if self.unknown {
            Flow::Stopped
        } else {
            Flow::ReachedNul
        }
    }

    /// `0x87`: past the rest of this record, any run of terminators and
    /// the whole record after them (`conversation.md §7.6`).
    fn alias_target(&self, index: usize) -> Option<usize> {
        let mut next = index + 1;
        while self.fields.get(next).is_some_and(Vec::is_empty) {
            next += 1;
        }
        let target = next + 1;
        (target < self.fields.len()).then_some(target)
    }

    /// Follows a transfer to `label`: the first declaration with it.
    fn label(&mut self, label: u8) {
        self.branches = true;
        match self.decls.iter().position(|(_, l)| *l == label) {
            Some(decl) => self.block(decl),
            None => self.unknown = true,
        }
    }

    /// Scans block `decl`: its head, and when the head reaches its NUL
    /// (a follow-up prompt) the default arm and every answer.
    fn block(&mut self, decl: usize) {
        self.branches = true;
        if self.unknown || !self.blocks.insert(decl) {
            // Each block once; a self-GOTO ends the walk.
            return;
        }
        let head = self.decls[decl].0;
        let end = self
            .decls
            .get(decl + 1)
            .map_or(self.fields.len(), |(index, _)| *index);
        if self.record(head, 2) != Flow::ReachedNul {
            return;
        }
        self.reply = true;
        if head + 1 < end {
            self.record(head + 1, 0);
        }
        // Keyword records sit at head + 2, + 4, ...; their answers follow.
        let mut answer = head + 3;
        while answer < end && !self.unknown {
            self.record(answer, 0);
            answer += 2;
        }
    }
}

/// `0x85`'s three argument bytes, each masked to seven bits, as a decimal
/// number (`formats/tlk.md §9.1`).
fn gold_amount(digits: &[u8]) -> Option<u16> {
    digits.iter().try_fold(0u16, |amount, byte| {
        let digit = (byte & 0x7F).checked_sub(b'0').filter(|d| *d <= 9)?;
        Some(amount * 10 + u16::from(digit))
    })
}
