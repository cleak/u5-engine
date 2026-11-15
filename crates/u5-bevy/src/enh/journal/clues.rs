//! The clue log in the Bevy shell: the observer, the live `.TLK` cache and
//! the sidecar I/O (Phase B design 8.3.4, 8.3.5, 8.3.7; build plan M6-02).
//!
//! Systems, each in one of the foundation's [`EnhSet`]s (8.3.12):
//!
//! - `clue_lifecycle` (`Apply`, after the toggles apply): puts the
//!   resources in and takes them out as `journal.clues` and
//!   `journal.clue_highlight` turn on and off. Turning `clues` off drops
//!   the log, its unsaved captures with it, and writes nothing.
//! - `clue_read` (`Feed`): reads `journal.u5e` when gameplay starts, after
//!   a slot load ([`ClueLogResource::request_read`]) and when `clues` turns
//!   on (at the idle prompt, its apply point). Each read replaces the log.
//! - `clue_observe_pre` (`Feed`, last, after every synthetic key source):
//!   the conversation's phase and test inputs before this frame's keys.
//! - `clue_observe_post` (`Observe`): diffs the transcript against the
//!   cursor kept from the last post, attributes the turns
//!   ([`u5_runtime::enhancements::clues::attribute`]) and folds them into
//!   the log.
//! - `clue_sidecar_write` (`Observe`, after the save watcher): writes the
//!   log on each new save, stamped with it; a new character's stamp resets
//!   the log and removes its file first.
//!
//! **Session-only mode.** With `clue_highlight` on and `clues` off there is
//! an observer and no log: the turns go into an asked set that is dropped
//! when the conversation ends, and no file is opened or stat'ed.
//!
//! **Replays.** While a synthetic key source holds the input token (keys
//! typed for the player, not by them) nothing is captured (D8.10).
//!
//! **Zero footprint.** The plugin is added only when `journal.clues` or
//! `journal.clue_highlight` is on at launch, never under Classic or the
//! parity lock (the foundation plugin itself is absent then); every
//! resource exists only while its key is on. Every system reads
//! `Res<VisualState>`, never `ResMut`, and calls only pure functions: the
//! game's state, keys, random draws and save bytes are the 1.0.1 ones.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use u5_runtime::enhancements::clues::{
    ClueLog, ConversationSnapshot, DialogKey, EncodeOptions, JOURNAL_PAYLOAD_VERSION, NpcKey,
    Reconciled, RecordAddr, TestInputs, Turn, TurnAttributor, TurnContext, encode_payload,
    game_minute, reconcile, test_inputs,
};
use u5_runtime::enhancements::{
    ClueText, FeatureToggles, Freshness, SaveStamp, SidecarStore, TAG_JOURNAL, Toggle,
    TranscriptCursor, TranscriptDiff,
};

use crate::VisualState;
use crate::enh::sidecar_io::{SaveStamped, SaveWatch};
use crate::enh::synthetic::SyntheticInputOwner;
use crate::enh::toast::Toasts;
use crate::enh::{EnhLaunch, EnhSet, Toggles, VisualLaunchConfig, toggle_on};

/// The clue log's toggle.
pub const CLUES: Toggle = Toggle::JournalClues;
/// Highlighting's driven toggle: on while `clue_highlight` is not `off`.
pub const HIGHLIGHT: Toggle = Toggle::JournalClueHighlight;

/// Shown once when the log could not be written. The save is on disk and
/// is never touched.
pub const CLUE_WRITE_ERROR: &str = "Clue log: the clues could not be saved.";
/// Logged when a new character's old log could not be removed.
const CLUE_RESET_ERROR: &str = "clue log: the old log could not be removed";

/// Whether the observer is wanted under `toggles` (8.3.4).
pub fn observer_wanted(toggles: FeatureToggles) -> bool {
    toggles.on(CLUES) || toggles.on(HIGHLIGHT)
}

/// Whether the log (and with it the sidecar and the TLK cache) is wanted.
pub fn log_wanted(toggles: FeatureToggles) -> bool {
    toggles.on(CLUES)
}

// ---------------------------------------------------------------------------
// Resources.

/// What the observer carries between its systems. Exists only while
/// `clues` or `clue_highlight` is on.
#[derive(Resource, Debug, Default)]
pub struct ClueObserver {
    /// The transcript as the last post left it.
    cursor: Option<TranscriptCursor>,
    /// The conversation before this frame's keys, from the pre system.
    pre: Option<PreSnapshot>,
    /// A synthetic key source held the input this frame.
    replay: bool,
    session: Option<ObservedSession>,
    /// Session-only mode: what was asked in this conversation.
    asked: Vec<(NpcKey, DialogKey, RecordAddr)>,
}

#[derive(Clone, Copy, Debug)]
struct PreSnapshot {
    snapshot: ConversationSnapshot,
}

/// One conversation as the observer follows it: who, the blob cloned once,
/// the attribution state and the inputs before the open turn.
#[derive(Debug)]
struct ObservedSession {
    npc: NpcKey,
    dialog: DialogKey,
    fields: Vec<Vec<u8>>,
    decoded: Vec<String>,
    attributor: TurnAttributor,
    /// The test inputs before the turn that is still open.
    open_pre: TestInputs,
}

impl ClueObserver {
    /// The asked set of session-only mode: `(person, dialog, record)` asked
    /// in the conversation in progress.
    pub fn asked(&self) -> &[(NpcKey, DialogKey, RecordAddr)] {
        &self.asked
    }

    /// Turns dropped in the conversation in progress (8.3.4 item 10).
    pub fn dropped(&self) -> u32 {
        self.session
            .as_ref()
            .map_or(0, |session| session.attributor.dropped())
    }

    /// Whether a conversation is being followed.
    pub fn in_conversation(&self) -> bool {
        self.session.is_some()
    }
}

/// The clue log of the save in play. Exists only while `clues` is on.
#[derive(Resource, Debug)]
pub struct ClueLogResource {
    log: ClueLog,
    /// The file is from a newer build: never written over (R3.1).
    read_only: bool,
    read_requested: bool,
}

impl Default for ClueLogResource {
    /// An empty log that reads the save's file at the next frame.
    fn default() -> Self {
        Self {
            log: ClueLog::new(),
            read_only: false,
            read_requested: true,
        }
    }
}

impl ClueLogResource {
    pub fn log(&self) -> &ClueLog {
        &self.log
    }

    pub fn log_mut(&mut self) -> &mut ClueLog {
        &mut self.log
    }

    /// Whether the file on disk is from a newer build.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// A slot load rewrote the save: the next frame reads its log, which
    /// replaces this one (captures not yet saved are dropped).
    pub fn request_read(&mut self) {
        self.read_requested = true;
    }
}

/// One parsed `.TLK` per family (8.3.5): names, descriptions, keywords and
/// static renders for the log's pages, read from the player's own game
/// directory on first use. RAM only, read-only, and dropped when the keys
/// that need it turn off.
#[derive(Resource, Debug, Default)]
pub struct TlkLiveCache {
    families: HashMap<u8, Option<Arc<HashMap<u16, Vec<Vec<u8>>>>>>,
}

impl TlkLiveCache {
    /// The raw fields of `dialog`'s blob, reading its family's file from
    /// `game_dir` the first time. `None` when the file or the blob is
    /// missing; a missing file is not read again.
    pub fn blob(&mut self, game_dir: &Path, dialog: DialogKey) -> Option<&[Vec<u8>]> {
        let table = self
            .families
            .entry(dialog.family)
            .or_insert_with(|| load_family(game_dir, dialog.family));
        table
            .as_ref()?
            .get(&u16::from(dialog.dialog_id))
            .map(Vec::as_slice)
    }

    /// How many families were read (tests and diagnostics).
    pub fn families_read(&self) -> usize {
        self.families.len()
    }
}

/// `formats/tlk.md §2`: a family's file is the one its first scene uses.
fn load_family(game_dir: &Path, family: u8) -> Option<Arc<HashMap<u16, Vec<Vec<u8>>>>> {
    let scene = family.checked_mul(8)?.checked_add(1)?;
    let name = u5_runtime::npc_tlk_filename(scene)?;
    u5_runtime::parse_tlk_raw(&game_dir.join(name))
        .ok()
        .map(Arc::new)
}

// ---------------------------------------------------------------------------
// The plugin.

/// Adds the plugin when the observer is wanted at launch. Called from
/// [`crate::enh::EnhancementsPlugin`], itself absent under Classic and the
/// parity lock.
pub(crate) fn add_clue_log(app: &mut App, launch: &VisualLaunchConfig) {
    if observer_wanted(launch.enhancements.toggles) {
        app.add_plugins(ClueLogPlugin {
            toggles: launch.enhancements.toggles,
        });
    }
}

/// The clue log's plugin.
pub struct ClueLogPlugin {
    pub toggles: FeatureToggles,
}

impl Plugin for ClueLogPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClueObserver::default());
        if log_wanted(self.toggles) {
            app.insert_resource(ClueLogResource::default())
                .insert_resource(TlkLiveCache::default());
        }
        app.add_systems(
            Update,
            clue_lifecycle
                .in_set(EnhSet::Apply)
                .after(crate::enh::apply_pending_toggles)
                .run_if(resource_changed::<Toggles>),
        )
        .add_systems(
            Update,
            (
                clue_read
                    .before(clue_observe_pre)
                    .run_if(resource_exists::<ClueLogResource>),
                clue_observe_pre
                    .after(crate::enh::synthetic::feed_synthetic_keys)
                    .run_if(resource_exists::<ClueObserver>),
            )
                .in_set(EnhSet::Feed)
                .run_if(resource_exists::<VisualState>),
        )
        .add_systems(
            Update,
            clue_observe_post
                .in_set(EnhSet::Observe)
                .run_if(resource_exists::<VisualState>)
                .run_if(resource_exists::<ClueObserver>),
        );
        // The save watcher exists only when the host gave a save folder; it
        // owns the `SaveStamped` event.
        if app.world().contains_resource::<SaveWatch>() {
            app.add_systems(
                Update,
                clue_sidecar_write
                    .in_set(EnhSet::Observe)
                    .after(crate::enh::sidecar_io::save_watcher)
                    .after(clue_observe_post)
                    .run_if(resource_exists::<ClueLogResource>)
                    .run_if(toggle_on(CLUES)),
            );
        }
    }
}

/// Puts in and takes out the resources as the keys change in play.
fn clue_lifecycle(
    mut commands: Commands,
    toggles: Res<Toggles>,
    observer: Option<Res<ClueObserver>>,
    log: Option<Res<ClueLogResource>>,
    cache: Option<Res<TlkLiveCache>>,
) {
    let toggles = toggles.0;
    match (observer_wanted(toggles), observer.is_some()) {
        (true, false) => commands.insert_resource(ClueObserver::default()),
        (false, true) => commands.remove_resource::<ClueObserver>(),
        _ => {}
    }
    match (log_wanted(toggles), log.is_some()) {
        // The new log reads the save's file at the next frame.
        (true, false) => commands.insert_resource(ClueLogResource::default()),
        // Unsaved captures are dropped, as the game drops unsaved progress;
        // nothing is written.
        (false, true) => commands.remove_resource::<ClueLogResource>(),
        _ => {}
    }
    match (log_wanted(toggles), cache.is_some()) {
        (true, false) => commands.insert_resource(TlkLiveCache::default()),
        (false, true) => commands.remove_resource::<TlkLiveCache>(),
        _ => {}
    }
}

/// The save folder the host handed in, if any.
fn save_dir(launch: &EnhLaunch, watch: Option<&SaveWatch>) -> Option<PathBuf> {
    watch
        .map(|watch| watch.save_dir().to_path_buf())
        .or_else(|| launch.0.save_dir.clone())
}

/// Reads the log at gameplay start, after a slot load and on turn-on.
fn clue_read(
    mut log: ResMut<ClueLogResource>,
    visual: Res<VisualState>,
    launch: Res<EnhLaunch>,
    watch: Option<Res<SaveWatch>>,
    toasts: Option<ResMut<Toasts>>,
) {
    if !visual.is_added() && !log.read_requested {
        return;
    }
    log.read_requested = false;
    let read = read_journal(save_dir(&launch, watch.as_deref()).as_deref());
    log.log = read.log;
    log.read_only = read.read_only;
    if let Some(note) = read.note {
        info!("{}", note.text());
        if let Some(mut toasts) = toasts {
            toasts.note(note.text());
        }
    }
}

/// `journal.u5e` of the save in `save_dir`, reconciled (8.3.7). With no
/// folder, a read-only install or no save, the log starts empty. A file
/// whose payload does not decode is renamed `.bad`.
pub(crate) fn read_journal(save_dir: Option<&Path>) -> Reconciled {
    io_probe::touch(save_dir);
    let missing = || reconcile(Freshness::Missing, None);
    let Some(dir) = save_dir else {
        return missing();
    };
    let (Some(store), Some(now)) = (SidecarStore::open(dir), SaveStamp::read(dir)) else {
        return missing();
    };
    let (freshness, payload) = store.read(TAG_JOURNAL, &now);
    let read = reconcile(freshness, payload.as_ref());
    if read.rename_bad
        && let Some(path) = store.path(TAG_JOURNAL)
    {
        let mut bad = path.clone().into_os_string();
        bad.push(".bad");
        if let Err(error) = std::fs::rename(&path, &bad) {
            warn!("clue log: the unreadable log could not be renamed: {error}");
        }
    }
    read
}

/// The conversation before this frame's keys run.
fn clue_observe_pre(
    mut observer: ResMut<ClueObserver>,
    visual: Res<VisualState>,
    synthetic: Option<Res<SyntheticInputOwner>>,
) {
    observer.replay = synthetic.is_some_and(|owner| owner.holder().is_some());
    observer.pre = ConversationSnapshot::of(&visual.state).map(|snapshot| PreSnapshot { snapshot });
}

/// The turns since the last post, folded into the log (or the asked set).
fn clue_observe_post(
    mut observer: ResMut<ClueObserver>,
    visual: Res<VisualState>,
    toggles: Res<Toggles>,
    mut log: Option<ResMut<ClueLogResource>>,
) {
    let state = &visual.state;
    let observer = &mut *observer;
    let pre = observer.pre.take();
    let replay = std::mem::take(&mut observer.replay);
    let diff = observer.cursor.map(|cursor| cursor.diff_state(state));
    observer.cursor = Some(TranscriptCursor::of(state));
    let now = ConversationSnapshot::of(state);

    if replay {
        // Keys typed for the player: nothing of this frame is attributed,
        // and the turn they interrupted is not finished from their rows.
        if let Some(session) = observer.session.as_mut() {
            session.attributor.unanchored();
        }
        if now.is_none() {
            end_session(observer);
        }
        return;
    }

    let same_person = |session: &ObservedSession| {
        now.is_some_and(|now| now.npc == session.npc && now.dialog == session.dialog)
    };
    let mut closed: Vec<(Turn, TestInputs)> = Vec::new();
    if let Some(session) = observer.session.as_mut() {
        let frame_pre = pre.map_or(session.open_pre, |pre| pre.snapshot.inputs);
        let was_open = session.attributor.is_open();
        let ctx = TurnContext {
            fields: &session.fields,
            decoded: &session.decoded,
            phase: pre.map(|pre| pre.snapshot.phase),
            misses: false,
        };
        let entries = state.message_entries();
        let hold = state.pending_text_hold.is_some();
        let turns = match diff {
            Some(TranscriptDiff::Unchanged) => {
                session
                    .attributor
                    .step(&ctx, entries, entries.len(), false, hold)
            }
            Some(TranscriptDiff::Anchored {
                first_new,
                replaced_last,
            }) => session
                .attributor
                .step(&ctx, entries, first_new, replaced_last, hold),
            Some(TranscriptDiff::Unanchored(_)) | None => {
                session.attributor.unanchored();
                Vec::new()
            }
        };
        let carried = session.open_pre;
        for (index, turn) in turns.into_iter().enumerate() {
            let turn_pre = if index == 0 && was_open {
                carried
            } else {
                frame_pre
            };
            closed.push((turn, turn_pre));
        }
        if session.attributor.is_open() && !(was_open && closed.is_empty()) {
            session.open_pre = frame_pre;
        }
        if !same_person(session) {
            let open_pre = session.open_pre;
            closed.extend(session.attributor.finish().map(|turn| (turn, open_pre)));
        }
    }
    let ended = observer
        .session
        .as_ref()
        .is_some_and(|session| !same_person(session));
    if !closed.is_empty()
        && let Some(session) = observer.session.as_ref()
    {
        let slot = usize::from(session.npc.slot);
        let post = test_inputs(state, slot);
        let at = game_minute(state);
        let logging = toggles.0.on(CLUES);
        for (turn, turn_pre) in closed {
            let entry = turn.entry(
                session.npc,
                session.dialog,
                &session.fields,
                &turn_pre,
                &post,
                at,
            );
            match log.as_deref_mut().filter(|_| logging) {
                Some(log) => {
                    log.log.fold(entry);
                }
                None => observer.asked.push((entry.npc, entry.dialog, entry.record)),
            }
        }
    }
    if ended {
        end_session(observer);
    }
    // A conversation that opened this frame: its opening is not a turn.
    if observer.session.is_none()
        && let (Some(now), Some(session)) = (now, state.active_conversation.as_ref())
    {
        observer.session = Some(ObservedSession {
            npc: now.npc,
            dialog: now.dialog,
            fields: session.fields.clone(),
            decoded: session.decoded_fields.clone(),
            attributor: TurnAttributor::new(),
            open_pre: now.inputs,
        });
    }
}

/// The conversation ended: its session-only asked set goes with it.
fn end_session(observer: &mut ClueObserver) {
    observer.session = None;
    observer.asked.clear();
}

/// Writes the log after each new save (8.3.7). A new character's stamp
/// first resets the log and removes its file.
fn clue_sidecar_write(
    mut log: ResMut<ClueLogResource>,
    mut stamped: EventReader<SaveStamped>,
    launch: Res<EnhLaunch>,
    watch: Res<SaveWatch>,
    toggles: Res<Toggles>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    for event in stamped.read() {
        let store = SidecarStore::open(watch.save_dir());
        io_probe::touch(Some(watch.save_dir()));
        if event.new_game {
            log.log = ClueLog::new();
            log.read_only = false;
            if let Some(store) = &store
                && let Err(error) = store.reset(&[TAG_JOURNAL])
            {
                warn!("{CLUE_RESET_ERROR}: {error}");
            }
            continue;
        }
        if log.read_only {
            continue;
        }
        let Some(store) = store else {
            continue;
        };
        let options = EncodeOptions {
            keep_text: launch.0.enhancements.journal().clues.text == ClueText::Kept,
            keep_misses: toggles.0.on(Toggle::JournalClueMisses),
        };
        let written = encode_payload(&log.log, options)
            .map_err(|error| format!("{error:?}"))
            .and_then(|payload| {
                store
                    .write(TAG_JOURNAL, &event.stamp, JOURNAL_PAYLOAD_VERSION, &payload)
                    .map_err(|error| error.to_string())
            });
        match written {
            Ok(_) => log.log.mark_saved(),
            Err(error) => {
                warn!("{CLUE_WRITE_ERROR}: {error}");
                if let Some(toasts) = toasts.as_mut() {
                    toasts.note(CLUE_WRITE_ERROR);
                }
            }
        }
    }
}

/// T3's recording shim: every path that opens, stats, writes or removes
/// `journal.u5e` names its save folder to [`touch`](io_probe::touch)
/// first. Recorded only in test builds.
pub(crate) mod io_probe {
    use std::path::Path;

    #[cfg(test)]
    static TOUCHED: std::sync::Mutex<Vec<Option<std::path::PathBuf>>> =
        std::sync::Mutex::new(Vec::new());

    #[inline]
    pub(crate) fn touch(save_dir: Option<&Path>) {
        #[cfg(test)]
        TOUCHED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(save_dir.map(Path::to_path_buf));
        #[cfg(not(test))]
        let _ = save_dir;
    }

    /// How often `save_dir` was touched so far.
    #[cfg(test)]
    pub(crate) fn touches(save_dir: &Path) -> usize {
        TOUCHED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|dir| dir.as_deref() == Some(save_dir))
            .count()
    }
}

#[cfg(test)]
mod tests;
