//! Per-save sidecars, first half: save detection (Phase B design 1.3.9).
//!
//! Some features keep a sidecar file beside `SAVED.GAM` and stamp it with
//! the exact bytes of the save it belongs to. They need to know that a save
//! was written, and the foundation adds no `PlayState` field and edits
//! neither the save writer nor the scene-rebuild paths to tell them.
//! [`SaveWatcher`] observes saves from outside instead, from `&PlayState`
//! and the file's metadata, and both frontends share it:
//!
//! - It hashes `SAVED.GAM` when the `SaveGame` yes/no prompt was open at the
//!   previous poll and is closed now, or when the file's length or
//!   modification time changed since the previous poll (a buffered `Q`, `Y`
//!   inside one frame or one terminal command).
//! - It returns a [`SaveStamp`] only when the FNV-1a-64 of the bytes
//!   differs from the last one it saw, so one save gives one stamp.
//! - It exists only while a sidecar feature is on
//!   ([`sidecar_feature_on`]).
//!
//! Residual risk, documented: on a filesystem with coarse timestamps, two
//! buffered saves inside one tick without a visible prompt look like one
//! save. The sidecar then reads as `SameLineage` at the next load, never as
//! a wrong map.
//!
//! The hash is [`fnv1a64`], the engine's own std-only copy of the FNV-1a
//! the desktop host uses for its runtime directory names; it is never
//! `DefaultHasher`, whose output may change between Rust releases.
//!
//! The second half (the sidecar store, its container and freshness) is
//! design 1.3.10.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use super::catalogue::{FeatureToggles, Toggle};
use crate::{
    PlayState, SAVE_AVATAR_NAME_OFFSET, SAVE_CHARACTER_NAME_LEN, SAVE_DAY_OFFSET, SAVE_HOUR_OFFSET,
    SAVE_MINUTE_OFFSET, SAVE_MONTH_OFFSET, SAVE_YEAR_OFFSET, SAVED_GAM_FILENAME, YesNoPromptKind,
    read_save_image_file,
};

/// The toggles whose features keep a per-save sidecar, and so need the
/// [`SaveWatcher`] (design 1.3.9, 1.3.10): the three exploration-memory
/// toggles (chapter 4's `automap.u5e`), the cheats marker (chapter 7's
/// `save-meta.u5e`), the clue log (chapter 8's `journal.u5e`) and the gem
/// map (chapter 10's `gemmap.u5e`). Their rows arrive with those chapters;
/// a key that is not a catalogue toggle yet never turns the watcher on.
pub const SIDECAR_KEYS: &[&str] = &[
    "automap.dungeon",
    "automap.town",
    "automap.overworld",
    "cheats.enabled",
    "journal.clues",
    "gem_map.enhanced",
];

/// Whether the toggles in force turn on a feature that keeps a sidecar.
pub fn sidecar_feature_on(toggles: FeatureToggles) -> bool {
    SIDECAR_KEYS
        .iter()
        .filter_map(|key| Toggle::from_key(key))
        .any(|toggle| toggles.on(toggle))
}

/// FNV-1a, 64 bits: offset basis `0xcbf29ce484222325`, prime
/// `0x100000001b3`. The same function as the desktop host's private copy,
/// which a `u5-tui` test pins to this one.
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}

/// What a sidecar is stamped with: the save it belongs to (design 1.3.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveStamp {
    /// [`fnv1a64`] of the exact `SAVED.GAM` bytes.
    pub fnv: u64,
    /// The file's length.
    pub len: u32,
    /// Record zero's nine-byte name (`formats/saved-gam.md §3.1`): the
    /// lineage.
    pub avatar: [u8; 9],
    /// The calendar (`formats/saved-gam.md §5`): year (u16, little
    /// endian), month, day, hour, minute.
    pub date: [u8; 6],
}

impl SaveStamp {
    /// The stamp of a save image.
    pub fn of_image(bytes: &[u8]) -> Self {
        let byte = |offset: usize| bytes.get(offset).copied().unwrap_or(0);
        let mut avatar = [0; SAVE_CHARACTER_NAME_LEN];
        for (index, slot) in avatar.iter_mut().enumerate() {
            *slot = byte(SAVE_AVATAR_NAME_OFFSET + index);
        }
        Self {
            fnv: fnv1a64(bytes),
            len: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
            avatar,
            date: [
                byte(SAVE_YEAR_OFFSET),
                byte(SAVE_YEAR_OFFSET + 1),
                byte(SAVE_MONTH_OFFSET),
                byte(SAVE_DAY_OFFSET),
                byte(SAVE_HOUR_OFFSET),
                byte(SAVE_MINUTE_OFFSET),
            ],
        }
    }

    /// The stamp of `<save_dir>/SAVED.GAM`. `None` when there is no save
    /// image there (missing, unreadable or the wrong length).
    pub fn read(save_dir: &Path) -> Option<Self> {
        read_save_image_file(&save_dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)
            .ok()
            .map(|bytes| Self::of_image(&bytes))
    }
}

/// The file metadata the watcher compares between polls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileMeta {
    len: u64,
    modified: Option<SystemTime>,
}

impl FileMeta {
    fn of(save_dir: &Path) -> Option<Self> {
        let meta = fs::metadata(save_dir.join(SAVED_GAM_FILENAME)).ok()?;
        Some(Self {
            len: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// Detects saves from outside the runtime (design 1.3.9). See the module
/// documentation for the rules.
///
/// Make one with [`SaveWatcher::new`] when gameplay starts, so the save
/// already on disk is known and gives no stamp, then call
/// [`poll`](Self::poll) once per frame or terminal command that consumed a
/// key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveWatcher {
    /// The last stamp seen, returned or primed.
    last: Option<SaveStamp>,
    /// The file's metadata at the last poll.
    meta: Option<FileMeta>,
    /// The `SaveGame` prompt was open at the last poll.
    save_prompt_open: bool,
}

impl SaveWatcher {
    /// A watcher that knows the save now in `save_dir`.
    pub fn new(save_dir: &Path) -> Self {
        let mut watcher = Self::default();
        watcher.prime(save_dir);
        watcher
    }

    /// Takes the save now in `save_dir` as seen, without a stamp: at
    /// gameplay start, and when a sidecar feature turns on in play.
    pub fn prime(&mut self, save_dir: &Path) {
        self.meta = FileMeta::of(save_dir);
        self.last = SaveStamp::read(save_dir);
        self.save_prompt_open = false;
    }

    /// The stamp of the last save seen, if any: what a feature compares its
    /// sidecar with when it reads it.
    pub fn last_stamp(&self) -> Option<SaveStamp> {
        self.last
    }

    /// Call once per frame or command that consumed a key. Returns the new
    /// save's stamp when a save was written since the last poll.
    pub fn poll(&mut self, state: &PlayState, save_dir: &Path) -> Option<SaveStamp> {
        let open = save_prompt_open(state);
        let prompt_closed = self.save_prompt_open && !open;
        self.save_prompt_open = open;
        let meta = FileMeta::of(save_dir);
        if !prompt_closed && meta == self.meta {
            return None;
        }
        self.meta = meta;
        self.hash(save_dir)
    }

    /// After something rewrote `SAVED.GAM` outside the play loop (a slot
    /// load, and after [`reset`](Self::reset) a new game): hashes the file
    /// whatever its metadata says.
    pub fn poll_rewritten(&mut self, save_dir: &Path) -> Option<SaveStamp> {
        self.meta = FileMeta::of(save_dir);
        self.save_prompt_open = false;
        self.hash(save_dir)
    }

    /// A new game (character creation or the Ultima IV transfer) forgets
    /// the last save, so the poll that follows stamps the new one even if
    /// its bytes happen to match.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn hash(&mut self, save_dir: &Path) -> Option<SaveStamp> {
        let stamp = SaveStamp::read(save_dir)?;
        if self.last.is_some_and(|last| last.fnv == stamp.fnv) {
            return None;
        }
        self.last = Some(stamp);
        Some(stamp)
    }
}

/// The `SaveGame` yes/no prompt is open.
fn save_prompt_open(state: &PlayState) -> bool {
    state
        .active_yes_no_prompt
        .is_some_and(|session| matches!(session.kind, YesNoPromptKind::SaveGame))
}

#[cfg(test)]
mod tests;
