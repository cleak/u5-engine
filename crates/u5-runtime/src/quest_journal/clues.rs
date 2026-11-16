//! Chapter 8's Clues and Topics tabs, derived (Phase B design 8.2.3-8.2.5,
//! 8.2.8; build plan M6-03).
//!
//! [`derive_clues`] turns the clue log, the player's own `.TLK` blobs and a
//! few facts of the play state into the rows a tab shows. It is pure: no
//! file, no RNG, no session method. The blobs come through
//! [`DialogSource`], which the shell fills from its live cache; every name,
//! description and keyword is read from them at runtime, never stored.
//!
//! **No spoilers (8.2.8).** A town or person appears only once the log
//! holds a turn with them or the save's name-known bank says the party was
//! introduced; the gazetteer orders the towns and never adds one. Nothing
//! is starred unless the player starred it. The runic mark shows from tier
//! 3. The "may differ now" line follows 8.3.13 and never says what changed.

use std::collections::{BTreeMap, HashMap};

use crate::enhancements::clues::{
    AskedState, ClueEntry, ClueLog, Dependence, DialogKey, EntryFlags, GameMinute, NpcKey,
    RecordAddr, TestInputs, asked_state, game_minute, note_owner, record_dependence,
};
use crate::enhancements::journal::SpoilerTier;
use crate::tlk_runner::{TlkRunInputs, run_tlk_stream};
use crate::{
    Area, PUBLISHED_COMMON_WORD_DICTIONARY, PlayState, decode_tlk_field, town_resident_name,
};

/// The most clues pinned to the side panel (8.2.5).
pub const MAX_PANEL_PINS: usize = 5;
/// The longest search text (the conversation's own input cap).
pub const MAX_SEARCH_LEN: usize = 15;

/// The last town-family scene byte (`catalogs/gazetteer.md` §5): the
/// name-known bank holds one mask per town scene.
const LAST_TOWN_SCENE: u8 = 32;
/// The bank's bits per scene: one per roster slot.
const NAME_KNOWN_BITS: u8 = 32;

/// Group and row labels. Our own words; no game text.
pub const PINNED: &str = "Pinned";
pub const INTRODUCED: &str = "Introduced, nothing logged";
pub const NOTHING_LOGGED: &str = "Introduced, nothing logged.";
pub const MAY_DIFFER: &str = "may differ now";
pub const RUNIC: &str = "runic";
pub const PINNED_MARK: &str = "pinned";
pub const HIDDEN_MARK: &str = "hidden";
pub const STALE_MARK: &str = "stale";
/// The topic label of a Job turn (`formats/tlk.md §7`, leading record 3).
pub const JOB: &str = "JOB";
/// A scoped answer: the prompt was answered (S8.1 gives arms no address).
pub const ANSWER: &str = "ANSWER";
/// A record whose blob could not be read.
pub const UNKNOWN_TOPIC: &str = "?";
/// A place with no gazetteer name.
pub const ELSEWHERE: &str = "Elsewhere";
pub const NO_CLUES: &str = "No clues logged yet.";
/// The Notes tab's row for the save itself.
pub const SAVE_NOTE: &str = "This save";
/// A row whose note is not written yet.
pub const NO_NOTE: &str = "No note yet. N writes one.";
/// The prefix of a note in the details.
pub const NOTE_PREFIX: &str = "Note: ";
pub const NO_MATCHES: &str = "Nothing matches.";

/// One of chapter 8's tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CluesTab {
    #[default]
    Clues,
    Topics,
    /// The player's notes (`journal.notes`, 8.2.4).
    Notes,
}

/// The Clues tab's two views; `V` switches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClueView {
    #[default]
    People,
    Place,
}

impl ClueView {
    pub fn other(self) -> Self {
        match self {
            ClueView::People => ClueView::Place,
            ClueView::Place => ClueView::People,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ClueView::People => "People",
            ClueView::Place => "Place",
        }
    }
}

/// The filters of 8.2.4. Every filter but [`ClueFilter::Hidden`] leaves
/// hidden entries out; `Hidden` shows only them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClueFilter {
    #[default]
    All,
    Unread,
    Starred,
    Here,
    Today,
    Hidden,
}

impl ClueFilter {
    pub const ALL: [ClueFilter; 6] = [
        ClueFilter::All,
        ClueFilter::Unread,
        ClueFilter::Starred,
        ClueFilter::Here,
        ClueFilter::Today,
        ClueFilter::Hidden,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ClueFilter::All => "all",
            ClueFilter::Unread => "unread",
            ClueFilter::Starred => "starred",
            ClueFilter::Here => "here",
            ClueFilter::Today => "today",
            ClueFilter::Hidden => "hidden",
        }
    }

    /// The next (or, with `step < 0`, the previous) filter, wrapping.
    pub fn step(self, step: isize) -> Self {
        let at = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        let next = (at as isize + step).rem_euclid(Self::ALL.len() as isize) as usize;
        Self::ALL[next]
    }
}

/// The raw fields of a dialog's blob, from the player's own `.TLK` files.
pub trait DialogSource {
    fn fields(&self, dialog: DialogKey) -> Option<&[Vec<u8>]>;
}

impl DialogSource for HashMap<DialogKey, Vec<Vec<u8>>> {
    fn fields(&self, dialog: DialogKey) -> Option<&[Vec<u8>]> {
        self.get(&dialog).map(Vec::as_slice)
    }
}

/// No blobs: every label falls back to its place.
pub struct NoDialogs;

impl DialogSource for NoDialogs {
    fn fields(&self, _: DialogKey) -> Option<&[Vec<u8>]> {
        None
    }
}

/// The play-state facts the tabs read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClueContext {
    /// The scene the party is in (0 outdoors): the `here` filter.
    pub scene: u8,
    pub now: GameMinute,
    /// The save's name-known bank (`talk_branch_flags`): scene byte to one
    /// bit per roster slot.
    pub introduced: HashMap<u8, u32>,
    pub standing: u8,
    pub gold: u16,
    pub party_size: u8,
    pub tier: SpoilerTier,
    /// Pins show at the top of the Clues tab: the side panel is off.
    pub pins_at_top: bool,
}

impl ClueContext {
    /// The live game's facts. Reads only.
    pub fn from_play_state(state: &PlayState, tier: SpoilerTier, pins_at_top: bool) -> Self {
        Self {
            scene: match state.area {
                Area::Town { scene, .. } => scene.byte,
                Area::Dungeon { scene, .. } => scene.byte,
                Area::World { .. } => 0,
            },
            now: game_minute(state),
            introduced: state.talk_branch_flags.clone(),
            standing: state.moral_standing,
            gold: state.gold,
            party_size: u8::try_from(state.party.len()).unwrap_or(u8::MAX),
            tier,
            pins_at_top,
        }
    }

    fn acquainted(&self, npc: NpcKey) -> bool {
        npc.slot < NAME_KNOWN_BITS
            && self
                .introduced
                .get(&npc.scene)
                .is_some_and(|bits| bits & (1 << npc.slot) != 0)
    }

    /// The test inputs for `npc` now (8.3.13 step 2).
    fn inputs(&self, npc: NpcKey) -> TestInputs {
        TestInputs {
            acquainted: self.acquainted(npc),
            standing: self.standing,
            gold: self.gold,
            party_size: self.party_size,
        }
    }

    /// The people the bank says the party was introduced to, in town
    /// scenes only, by scene then slot.
    fn introduced_people(&self) -> Vec<NpcKey> {
        let mut people: Vec<NpcKey> = self
            .introduced
            .iter()
            .filter(|(scene, _)| (1..=LAST_TOWN_SCENE).contains(*scene))
            .flat_map(|(scene, bits)| {
                (0..NAME_KNOWN_BITS)
                    .filter(move |slot| bits & (1 << slot) != 0)
                    .map(move |slot| NpcKey {
                        scene: *scene,
                        slot,
                    })
            })
            .collect();
        people.sort_unstable();
        people
    }
}

/// What the player asked of the tabs: never saved.
#[derive(Clone, Copy, Debug)]
pub struct ClueQuery<'a> {
    pub tab: CluesTab,
    pub view: ClueView,
    pub filter: ClueFilter,
    /// Incremental search text; empty for none.
    pub search: &'a str,
    /// The groups the player opened.
    pub open: &'a [GroupKey],
    /// The Quest filter (8.2.7 "Enter on a step"): only these entries,
    /// from `trackers::linked_entries`. `None` for no filter.
    pub only: Option<&'a [usize]>,
}

impl Default for ClueQuery<'_> {
    fn default() -> Self {
        Self {
            tab: CluesTab::Clues,
            view: ClueView::People,
            filter: ClueFilter::All,
            search: "",
            open: &[],
            only: None,
        }
    }
}

/// A group a row opens and closes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GroupKey {
    Pinned,
    Person(NpcKey),
    Town(u8),
    Introduced,
    Topic(String),
}

/// What a row stands for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowKind {
    /// A group: its key, whether it is open, and the entries in it (the
    /// ones opening it marks read).
    Group {
        key: GroupKey,
        open: bool,
        entries: Vec<usize>,
    },
    /// A person row with no entries of its own on this row.
    Person(NpcKey),
    /// A log entry, by index.
    Entry(usize),
    /// A line of text with nothing to act on.
    Note,
    /// A row of the Notes tab: what the note is on (`note_owner`, owner).
    Noted { owner_kind: u8, owner: u16 },
}

/// A row's mark column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RowMark {
    #[default]
    None,
    /// `o`: not read yet.
    Unread,
    /// `*`: starred by the player.
    Star,
}

impl RowMark {
    pub fn glyph(self) -> char {
        match self {
            RowMark::None => ' ',
            RowMark::Unread => 'o',
            RowMark::Star => '*',
        }
    }
}

/// One row of a tab.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClueRow {
    pub kind: RowKind,
    /// Indent level: 0 for a top group.
    pub depth: u8,
    pub mark: RowMark,
    pub label: String,
    /// How often an entry was heard, shown above one.
    pub count: Option<u16>,
    /// The right-hand text: a date, or a group's size.
    pub right: String,
    /// The details shown for the selected row.
    pub detail: Vec<String>,
}

impl ClueRow {
    pub fn entry(&self) -> Option<usize> {
        match self.kind {
            RowKind::Entry(index) => Some(index),
            RowKind::Noted {
                owner_kind: note_owner::CLUE,
                owner,
            } => Some(usize::from(owner)),
            _ => None,
        }
    }

    /// What `N` writes a note on from this row: a clue, a person, or (on
    /// the Notes tab) whatever the row is about.
    pub fn note_owner(&self) -> Option<(u8, u16)> {
        match &self.kind {
            RowKind::Entry(index) => Some((note_owner::CLUE, u16::try_from(*index).ok()?)),
            RowKind::Person(npc)
            | RowKind::Group {
                key: GroupKey::Person(npc),
                ..
            } => Some((note_owner::PERSON, note_owner::person(*npc))),
            RowKind::Noted { owner_kind, owner } => Some((*owner_kind, *owner)),
            _ => None,
        }
    }

    /// Every string the row shows (spoiler tests).
    pub fn strings(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.label.as_str())
            .chain(std::iter::once(self.right.as_str()))
            .chain(self.detail.iter().map(String::as_str))
    }
}

/// What a tab shows.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CluesView {
    pub rows: Vec<ClueRow>,
    /// Captures since the last save (8.2.9).
    pub unsaved: u32,
    /// Entry rows the filter and search keep (before groups fold).
    pub matches: usize,
    /// Topic entries not read, hidden ones aside.
    pub unread: usize,
    pub starred: usize,
    /// Entries pinned to the panel.
    pub pinned: Vec<usize>,
}

impl CluesView {
    pub fn strings(&self) -> impl Iterator<Item = &str> {
        self.rows.iter().flat_map(ClueRow::strings)
    }
}

/// `N clues since your last save` (8.2.9), or `None` with nothing unsaved.
pub fn unsaved_line(unsaved: u32) -> Option<String> {
    match unsaved {
        0 => None,
        1 => Some("1 clue since your last save".to_string()),
        n => Some(format!("{n} clues since your last save")),
    }
}

/// The game's date of a [`GameMinute`]: `month-day-year`.
pub fn date_label(at: GameMinute) -> String {
    let days = at.0 / 1440;
    let year = days / (13 * 28);
    let month = (days / 28) % 13 + 1;
    let day = days % 28 + 1;
    format!("{month}-{day}-{year}")
}

/// `hh:mm` of a [`GameMinute`].
pub fn time_label(at: GameMinute) -> String {
    let minutes = at.0 % 1440;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn day_of(at: GameMinute) -> u32 {
    at.0 / 1440
}

/// A blob field decoded for display: trimmed, `None` when empty.
fn decoded(fields: &[Vec<u8>], index: usize) -> Option<String> {
    let field = fields.get(index)?;
    let (text, _) = decode_tlk_field(field, 0, field.len());
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// A Name or Description field as the window prints it: run through the
/// byte runner with the common-word dictionary (`formats/tlk.md §10`), so
/// a word stored as a dictionary token ("the") is kept. Rows are joined
/// with single spaces; `None` when empty.
fn rendered(fields: &[Vec<u8>], index: usize) -> Option<String> {
    let field = fields.get(index)?;
    let dictionary = PUBLISHED_COMMON_WORD_DICTIONARY;
    let inputs = TlkRunInputs {
        avatar_name: "",
        branch_flags: 0,
        moral_standing: 0,
        dictionary: Some(&dictionary),
        gold_available: None,
        npc_slot: None,
        ask_who_response: 0,
        yield_on_pause: false,
        mark_holds: false,
        yield_on_ask: false,
    };
    let output = run_tlk_stream(field, &inputs);
    let bytes: Vec<u8> = output
        .rendered_glyphs
        .iter()
        .map(|glyph| glyph.byte)
        .collect();
    let text = String::from_utf8_lossy(&bytes);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    // The window's framing quotes belong to the reply, not the name.
    let text = text.trim_matches(|c: char| c == '"' || c.is_whitespace());
    (!text.is_empty()).then(|| text.to_string())
}

/// The topic label of a record: the answering NPC's own keyword record,
/// upper-cased (8.2.3).
pub fn topic_label(fields: Option<&[Vec<u8>]>, record: RecordAddr) -> String {
    match record {
        RecordAddr::Leading(3) => JOB.to_string(),
        RecordAddr::Pair(k) => fields
            .and_then(|fields| decoded(fields, 2 * usize::from(k) + 5))
            .map(|keyword| keyword.to_ascii_uppercase())
            .unwrap_or_else(|| UNKNOWN_TOPIC.to_string()),
        RecordAddr::Scoped { .. } => ANSWER.to_string(),
        RecordAddr::Leading(_) | RecordAddr::Miss => UNKNOWN_TOPIC.to_string(),
    }
}

/// A town's gazetteer name, or [`ELSEWHERE`].
pub fn town_label(scene: u8) -> &'static str {
    town_resident_name(scene).unwrap_or(ELSEWHERE)
}

/// The derived facts of one log, shared by every view.
struct Facts<'a> {
    log: &'a ClueLog,
    source: &'a dyn DialogSource,
    ctx: &'a ClueContext,
    /// People whose NAME turn the log holds, with their dialog.
    named: HashMap<NpcKey, DialogKey>,
    /// Any dialog seen for a person.
    dialogs: HashMap<NpcKey, DialogKey>,
}

impl<'a> Facts<'a> {
    fn new(log: &'a ClueLog, source: &'a dyn DialogSource, ctx: &'a ClueContext) -> Self {
        let mut named = HashMap::new();
        let mut dialogs = HashMap::new();
        for entry in log.entries() {
            dialogs.entry(entry.npc).or_insert(entry.dialog);
            if entry.record == RecordAddr::Leading(0) {
                named.insert(entry.npc, entry.dialog);
            }
        }
        Self {
            log,
            source,
            ctx,
            named,
            dialogs,
        }
    }

    fn entry(&self, index: usize) -> &'a ClueEntry {
        &self.log.entries()[index]
    }

    /// The person's name once a NAME turn is logged; otherwise "a person
    /// in <town>" (8.2.3). The log never guesses from the greeting.
    fn person(&self, npc: NpcKey) -> String {
        self.named
            .get(&npc)
            .and_then(|dialog| self.source.fields(*dialog))
            .and_then(|fields| rendered(fields, 0))
            .unwrap_or_else(|| format!("a person in {}", town_label(npc.scene)))
    }

    /// The slot's description, read live, when a dialog is known.
    fn description(&self, npc: NpcKey) -> Option<String> {
        let dialog = self.dialogs.get(&npc)?;
        let text = rendered(self.source.fields(*dialog)?, 1)?;
        let text = text.trim_end_matches(['.', ',', ' ']).to_string();
        (!text.is_empty()).then_some(text)
    }

    /// A person with their town: `Name (Town)`, or "a person in Town".
    fn person_in_town(&self, npc: NpcKey) -> String {
        if self.named.contains_key(&npc) {
            format!("{} ({})", self.person(npc), town_label(npc.scene))
        } else {
            self.person(npc)
        }
    }

    fn topic(&self, entry: &ClueEntry) -> String {
        topic_label(self.source.fields(entry.dialog), entry.record)
    }

    fn may_differ(&self, entry: &ClueEntry) -> bool {
        let heard: Vec<ClueEntry> = self
            .log
            .entries()
            .iter()
            .filter(|e| e.npc == entry.npc && e.dialog == entry.dialog && e.record == entry.record)
            .copied()
            .collect();
        let dependence = self
            .source
            .fields(entry.dialog)
            .map_or(Dependence::Unknown, |fields| {
                record_dependence(fields, entry.record)
            });
        asked_state(&heard, &dependence, &self.ctx.inputs(entry.npc)) == AskedState::MayDiffer
    }

    fn mark(&self, entry: &ClueEntry) -> RowMark {
        if entry.flags.has(EntryFlags::STAR) {
            RowMark::Star
        } else if !entry.flags.has(EntryFlags::READ) {
            RowMark::Unread
        } else {
            RowMark::None
        }
    }

    /// The selected entry's details: the `clue_text = off` body (8.2.3),
    /// then when, how often and the marks.
    fn entry_detail(&self, index: usize) -> Vec<String> {
        let entry = self.entry(index);
        // The description is on the person's own row and details.
        let body = format!(
            "You asked {} about {}.",
            self.person(entry.npc),
            self.topic(entry)
        );
        // The row shows the date; the details add the time.
        let mut facts = vec![format!("at {}", time_label(entry.last))];
        if entry.count > 1 {
            facts.push(format!("x{}", entry.count));
        }
        if self.may_differ(entry) {
            facts.push(MAY_DIFFER.to_string());
        }
        if self.ctx.tier >= SpoilerTier::Hints && entry.flags.has(EntryFlags::RUNIC) {
            facts.push(RUNIC.to_string());
        }
        for (flag, word) in [
            (EntryFlags::PANEL, PINNED_MARK),
            (EntryFlags::HIDDEN, HIDDEN_MARK),
            (EntryFlags::STALE, STALE_MARK),
        ] {
            if entry.flags.has(flag) {
                facts.push(word.to_string());
            }
        }
        let mut detail = vec![body, facts.join("  ")];
        if let Some(note) = self.clue_note(index) {
            detail.push(format!("{NOTE_PREFIX}{note}"));
        }
        detail
    }

    fn entry_row(&self, index: usize, depth: u8, label: String) -> ClueRow {
        let entry = self.entry(index);
        ClueRow {
            kind: RowKind::Entry(index),
            depth,
            mark: self.mark(entry),
            label,
            count: (entry.count > 1).then_some(entry.count),
            right: date_label(entry.last),
            detail: self.entry_detail(index),
        }
    }

    fn passes(&self, entry: &ClueEntry, filter: ClueFilter) -> bool {
        let hidden = entry.flags.has(EntryFlags::HIDDEN);
        match filter {
            ClueFilter::Hidden => hidden,
            _ if hidden => false,
            ClueFilter::All => true,
            ClueFilter::Unread => !entry.flags.has(EntryFlags::READ),
            ClueFilter::Starred => entry.flags.has(EntryFlags::STAR),
            ClueFilter::Here => entry.npc.scene == self.ctx.scene,
            ClueFilter::Today => day_of(entry.last) == day_of(self.ctx.now),
        }
    }

    /// The player's note on a clue, by index.
    fn clue_note(&self, index: usize) -> Option<&'a str> {
        let owner = u16::try_from(index).ok()?;
        self.log
            .note_for(note_owner::CLUE, owner)
            .map(|note| note.text.as_str())
    }

    fn person_note(&self, npc: NpcKey) -> Option<&'a str> {
        self.log
            .note_for(note_owner::PERSON, note_owner::person(npc))
            .map(|note| note.text.as_str())
    }

    /// Search over topic, name, place and the player's notes, ignoring
    /// case.
    fn matches(&self, index: usize, search: &str) -> bool {
        let entry = self.entry(index);
        search.is_empty()
            || [
                self.topic(entry),
                self.person(entry.npc),
                town_label(entry.npc.scene).to_string(),
            ]
            .iter()
            .any(|text| contains_ignoring_case(text, search))
            || self
                .clue_note(index)
                .is_some_and(|note| contains_ignoring_case(note, search))
            || self
                .person_note(entry.npc)
                .is_some_and(|note| contains_ignoring_case(note, search))
    }

    fn person_matches(&self, npc: NpcKey, search: &str) -> bool {
        search.is_empty()
            || contains_ignoring_case(&self.person(npc), search)
            || contains_ignoring_case(town_label(npc.scene), search)
            || self
                .person_note(npc)
                .is_some_and(|note| contains_ignoring_case(note, search))
    }
}

fn contains_ignoring_case(text: &str, search: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&search.trim().to_ascii_lowercase())
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// The rows of `query.tab` (8.2.3, 8.2.4).
pub fn derive_clues(
    log: &ClueLog,
    source: &dyn DialogSource,
    ctx: &ClueContext,
    query: &ClueQuery<'_>,
) -> CluesView {
    let facts = Facts::new(log, source, ctx);
    let topics: Vec<usize> = (0..log.entries().len())
        .filter(|index| log.entries()[*index].record.is_topic())
        .collect();
    let visible: Vec<usize> = topics
        .iter()
        .copied()
        .filter(|index| {
            let entry = facts.entry(*index);
            facts.passes(entry, query.filter)
                && facts.matches(*index, query.search)
                && query.only.is_none_or(|only| only.contains(index))
        })
        .collect();
    let searching = !query.search.trim().is_empty();
    let is_open = |key: &GroupKey| searching || query.open.contains(key);
    let pinned: Vec<usize> = topics
        .iter()
        .copied()
        .filter(|index| facts.entry(*index).flags.has(EntryFlags::PANEL))
        .collect();

    let mut rows = Vec::new();
    match query.tab {
        CluesTab::Clues => {
            if ctx.pins_at_top && !pinned.is_empty() && query.only.is_none() {
                rows.push(group_row(
                    GroupKey::Pinned,
                    PINNED.to_string(),
                    true,
                    pinned.clone(),
                    &facts,
                    plural(pinned.len(), "clue"),
                ));
                for index in &pinned {
                    let entry = facts.entry(*index);
                    let label = format!("{} {}", facts.topic(entry), facts.person(entry.npc));
                    rows.push(facts.entry_row(*index, 1, label));
                }
            }
            match query.view {
                ClueView::People => people_rows(&facts, &visible, query, &is_open, &mut rows),
                ClueView::Place => place_rows(&facts, &visible, query, &is_open, &mut rows),
            }
        }
        CluesTab::Topics => topic_rows(&facts, &visible, &is_open, &mut rows),
        CluesTab::Notes => note_rows(&facts, query.search, &mut rows),
    }
    if rows.is_empty() {
        let text = if log.entries().is_empty() && !searching {
            NO_CLUES
        } else {
            NO_MATCHES
        };
        rows.push(ClueRow {
            kind: RowKind::Note,
            depth: 0,
            mark: RowMark::None,
            label: text.to_string(),
            count: None,
            right: String::new(),
            detail: Vec::new(),
        });
    }
    let readable = |index: &&usize| !facts.entry(**index).flags.has(EntryFlags::HIDDEN);
    CluesView {
        rows,
        unsaved: log.unsaved(),
        matches: visible.len(),
        unread: topics
            .iter()
            .filter(readable)
            .filter(|index| !facts.entry(**index).flags.has(EntryFlags::READ))
            .count(),
        starred: topics
            .iter()
            .filter(|index| facts.entry(**index).flags.has(EntryFlags::STAR))
            .count(),
        pinned,
    }
}

fn group_row(
    key: GroupKey,
    label: String,
    open: bool,
    entries: Vec<usize>,
    facts: &Facts<'_>,
    right: String,
) -> ClueRow {
    let unread = entries
        .iter()
        .filter(|index| !facts.entry(**index).flags.has(EntryFlags::READ))
        .count();
    let mut detail = vec![format!(
        "{}, {} unread",
        plural(entries.len(), "topic"),
        unread
    )];
    if let GroupKey::Person(npc) = key {
        let mut first = facts.person_in_town(npc);
        if let Some(description) = facts.description(npc) {
            first = format!("{first}, {description}");
        }
        detail.insert(0, first);
        if let Some(note) = facts.person_note(npc) {
            detail.push(format!("{NOTE_PREFIX}{note}"));
        }
    }
    ClueRow {
        kind: RowKind::Group { key, open, entries },
        depth: 0,
        mark: if unread > 0 {
            RowMark::Unread
        } else {
            RowMark::None
        },
        label,
        count: None,
        right,
        detail,
    }
}

/// Entries by person, in the order they were heard.
fn by_person(facts: &Facts<'_>, visible: &[usize]) -> BTreeMap<NpcKey, Vec<usize>> {
    let mut people: BTreeMap<NpcKey, Vec<usize>> = BTreeMap::new();
    for index in visible {
        people
            .entry(facts.entry(*index).npc)
            .or_default()
            .push(*index);
    }
    for entries in people.values_mut() {
        entries.sort_by_key(|index| (facts.entry(*index).first, *index));
    }
    people
}

/// The introduced people with no topic in the log, when the filter shows
/// every entry.
fn introduced_only(
    facts: &Facts<'_>,
    query: &ClueQuery<'_>,
    logged: &BTreeMap<NpcKey, Vec<usize>>,
) -> Vec<NpcKey> {
    if query.filter != ClueFilter::All {
        return Vec::new();
    }
    let has_topic = |npc: &NpcKey| {
        logged.contains_key(npc)
            || facts
                .log
                .entries()
                .iter()
                .any(|entry| entry.npc == *npc && entry.record.is_topic())
    };
    let mut people = facts.ctx.introduced_people();
    // A NAME turn alone also makes a person known.
    for npc in facts.named.keys() {
        if !people.contains(npc) {
            people.push(*npc);
        }
    }
    people.sort_unstable();
    people
        .into_iter()
        .filter(|npc| !has_topic(npc))
        .filter(|npc| facts.person_matches(*npc, query.search))
        .collect()
}

fn person_note_row(facts: &Facts<'_>, npc: NpcKey, depth: u8) -> ClueRow {
    let mut detail = vec![facts.person_in_town(npc)];
    if let Some(description) = facts.description(npc) {
        detail[0] = format!("{}, {description}", detail[0]);
    }
    detail.push(NOTHING_LOGGED.to_string());
    if let Some(note) = facts.person_note(npc) {
        detail.push(format!("{NOTE_PREFIX}{note}"));
    }
    ClueRow {
        kind: RowKind::Person(npc),
        depth,
        mark: RowMark::None,
        label: facts.person_in_town(npc),
        count: None,
        right: String::new(),
        detail,
    }
}

/// People view: each person with their topics, the most recent first, then
/// the people introduced with nothing logged.
fn people_rows(
    facts: &Facts<'_>,
    visible: &[usize],
    query: &ClueQuery<'_>,
    is_open: &dyn Fn(&GroupKey) -> bool,
    rows: &mut Vec<ClueRow>,
) {
    let people = by_person(facts, visible);
    let mut order: Vec<(&NpcKey, &Vec<usize>)> = people.iter().collect();
    order.sort_by_key(|(npc, entries)| {
        let last = entries
            .iter()
            .map(|index| facts.entry(*index).last)
            .max()
            .unwrap_or_default();
        (std::cmp::Reverse(last), **npc)
    });
    for (npc, entries) in order {
        let key = GroupKey::Person(*npc);
        let open = is_open(&key);
        rows.push(group_row(
            key,
            facts.person_in_town(*npc),
            open,
            entries.clone(),
            facts,
            plural(entries.len(), "topic"),
        ));
        if open {
            for index in entries {
                rows.push(facts.entry_row(*index, 1, facts.topic(facts.entry(*index))));
            }
        }
    }
    let introduced = introduced_only(facts, query, &people);
    if !introduced.is_empty() {
        let key = GroupKey::Introduced;
        let open = is_open(&key);
        rows.push(ClueRow {
            kind: RowKind::Group {
                key,
                open,
                entries: Vec::new(),
            },
            depth: 0,
            mark: RowMark::None,
            label: INTRODUCED.to_string(),
            count: None,
            // The count is in the details, so the label is never cut.
            right: String::new(),
            detail: vec![
                NOTHING_LOGGED.to_string(),
                plural(introduced.len(), "person").replace("persons", "people"),
            ],
        });
        if open {
            for npc in introduced {
                rows.push(person_note_row(facts, npc, 1));
            }
        }
    }
}

/// Place view: towns in gazetteer order (scene byte order,
/// `catalogs/gazetteer.md §5`), each person, each topic. Only towns with a
/// logged or introduced person appear.
fn place_rows(
    facts: &Facts<'_>,
    visible: &[usize],
    query: &ClueQuery<'_>,
    is_open: &dyn Fn(&GroupKey) -> bool,
    rows: &mut Vec<ClueRow>,
) {
    let people = by_person(facts, visible);
    let introduced = introduced_only(facts, query, &people);
    let mut towns: BTreeMap<u8, Vec<NpcKey>> = BTreeMap::new();
    for npc in people.keys().chain(introduced.iter()) {
        towns.entry(npc.scene).or_default().push(*npc);
    }
    for (scene, mut npcs) in towns {
        npcs.sort_unstable();
        npcs.dedup();
        let entries: Vec<usize> = npcs
            .iter()
            .filter_map(|npc| people.get(npc))
            .flatten()
            .copied()
            .collect();
        let key = GroupKey::Town(scene);
        let open = is_open(&key);
        // A town of introductions only shows no count.
        let right = if entries.is_empty() {
            String::new()
        } else {
            plural(entries.len(), "topic")
        };
        rows.push(group_row(
            key,
            town_label(scene).to_string(),
            open,
            entries,
            facts,
            right,
        ));
        if !open {
            continue;
        }
        for npc in npcs {
            match people.get(&npc) {
                Some(entries) => {
                    let mut row = person_note_row(facts, npc, 1);
                    row.label = facts.person(npc);
                    row.right = plural(entries.len(), "topic");
                    row.detail.retain(|line| line != NOTHING_LOGGED);
                    rows.push(row);
                    for index in entries {
                        rows.push(facts.entry_row(*index, 2, facts.topic(facts.entry(*index))));
                    }
                }
                None => {
                    let mut row = person_note_row(facts, npc, 1);
                    row.label = facts.person(npc);
                    rows.push(row);
                }
            }
        }
    }
}

/// Topics: every keyword, alphabetically by the exact string, with each
/// person who answered it (8.2.3). No fuzzy merging.
fn topic_rows(
    facts: &Facts<'_>,
    visible: &[usize],
    is_open: &dyn Fn(&GroupKey) -> bool,
    rows: &mut Vec<ClueRow>,
) {
    let mut topics: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for index in visible {
        topics
            .entry(facts.topic(facts.entry(*index)))
            .or_default()
            .push(*index);
    }
    for (topic, mut entries) in topics {
        entries.sort_by_key(|index| (facts.entry(*index).first, *index));
        let key = GroupKey::Topic(topic.clone());
        let open = is_open(&key);
        let mut people: Vec<NpcKey> = entries.iter().map(|i| facts.entry(*i).npc).collect();
        people.sort_unstable();
        people.dedup();
        let right = plural(people.len(), "person").replace("persons", "people");
        let mut row = group_row(key, topic, open, entries.clone(), facts, right);
        row.detail = vec![format!(
            "Asked of {}.",
            plural(people.len(), "person").replace("persons", "people")
        )];
        rows.push(row);
        if open {
            for index in entries {
                let label = facts.person_in_town(facts.entry(index).npc);
                rows.push(facts.entry_row(index, 1, label));
            }
        }
    }
}

/// The Notes tab (8.2.4): the save's own note first (always listed, so `N`
/// can write one), then the notes on people, then the notes on clues, each
/// in the order written. Search keeps the rows whose label or note
/// matches.
fn note_rows(facts: &Facts<'_>, search: &str, rows: &mut Vec<ClueRow>) {
    let save = facts.log.note_for(note_owner::SAVE, 0);
    let mut listed = vec![(
        note_owner::SAVE,
        0,
        SAVE_NOTE.to_string(),
        save.map(|note| note.text.clone()),
    )];
    for kind in [note_owner::PERSON, note_owner::CLUE] {
        for note in facts
            .log
            .notes()
            .iter()
            .filter(|note| note.owner_kind == kind)
        {
            let label = match kind {
                note_owner::PERSON => facts.person_in_town(note_owner::person_of(note.owner)),
                _ => match facts.log.entries().get(usize::from(note.owner)) {
                    Some(entry) => format!("{} {}", facts.topic(entry), facts.person(entry.npc)),
                    None => continue,
                },
            };
            listed.push((kind, note.owner, label, Some(note.text.clone())));
        }
    }
    for (owner_kind, owner, label, note) in listed {
        let shown = note.clone().unwrap_or_else(|| NO_NOTE.to_string());
        if !search.trim().is_empty()
            && !contains_ignoring_case(&label, search)
            && !note
                .as_deref()
                .is_some_and(|note| contains_ignoring_case(note, search))
        {
            continue;
        }
        let detail = match note {
            Some(note) => vec![format!("{NOTE_PREFIX}{note}")],
            None => vec![shown],
        };
        rows.push(ClueRow {
            kind: RowKind::Noted { owner_kind, owner },
            depth: 0,
            mark: RowMark::None,
            label,
            count: None,
            right: String::new(),
            detail,
        });
    }
}

/// The journal side panel's clue lines (8.2.5, 8.2.7): `Clues: N unread *
/// M` once the log holds a topic, then `Pinned` and each pinned clue. Each
/// line is cut to `columns`.
pub fn panel_clue_lines(
    log: &ClueLog,
    source: &dyn DialogSource,
    ctx: &ClueContext,
    columns: usize,
) -> Vec<String> {
    let view = derive_clues(
        log,
        source,
        &ClueContext {
            pins_at_top: false,
            ..ctx.clone()
        },
        &ClueQuery::default(),
    );
    let facts = Facts::new(log, source, ctx);
    let mut lines = Vec::new();
    if log.entries().iter().any(|entry| entry.record.is_topic()) {
        let candidates = [
            format!("Clues: {} unread * {}", view.unread, view.starred),
            format!("Clues {} unread *{}", view.unread, view.starred),
            format!("Clues o{} *{}", view.unread, view.starred),
        ];
        let line = candidates
            .iter()
            .find(|line| line.chars().count() <= columns)
            .unwrap_or(&candidates[2]);
        lines.push(line.clone());
    }
    if !view.pinned.is_empty() {
        lines.push(PINNED.to_string());
        for index in view.pinned.iter().take(MAX_PANEL_PINS) {
            let entry = facts.entry(*index);
            // A name once heard, else the town: the panel is 16 cells.
            let who = if facts.named.contains_key(&entry.npc) {
                facts.person(entry.npc)
            } else {
                town_label(entry.npc.scene).to_string()
            };
            lines.push(format!(" {} {who}", facts.topic(entry)));
        }
    }
    lines
        .into_iter()
        .map(|line| line.chars().take(columns).collect())
        .collect()
}

#[cfg(test)]
#[path = "clues_tests.rs"]
mod tests;
