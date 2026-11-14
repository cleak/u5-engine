//! Named save slots: the slot store (Phase B design 3.3.5, 3.2.2, 3.2.3,
//! 3.2.9 "Slots without the overlay", D3.7, D3.9).
//!
//! A slot is a copy of a game's whole working set, kept outside every game
//! folder:
//!
//! ```text
//! <data_base>/saves/<work-key>/
//!   slots/<slot-id>/
//!     SAVED.GAM SAVED.OOL BRIT.OOL UNDER.OOL
//!     SAVED.WPS .u5-engine-town-npc-mutations
//!     .u5-engine-enh/    copied whole when present
//!     slot.toml  thumb.png
//!   .backup/     the last restore's backup
//!   .lock        writer lock
//! ```
//!
//! - **Keyed by the working folder.** `<work-key>` is [`work_key`] of the
//!   canonical folder `SAVED.GAM` lives in, so slots stay with the save
//!   they copy. [`SlotStore::open`] refuses a slot root inside the working
//!   folder or any folder the caller names (the source tree, the mirror
//!   root).
//! - **Opaque bytes.** The store copies the six [`WORKING_SET_FILES`] and
//!   the [`SIDECAR_DIR_NAME`] folder byte for byte, and never parses a
//!   save or a sidecar. What a slot shows (Avatar, place, clock, turn)
//!   comes from the caller in [`SlotDetails`].
//! - **One writer.** Every operation takes `.lock` (`create_new`, stale
//!   after [`LOCK_STALE_AFTER`]), waiting up to [`LOCK_WAIT`].
//! - **Crash safety.** A store builds the slot in `slots/.tmp-<id>-<pid>/`,
//!   syncs it, moves any old copy to `slots/.old-<id>`, renames the new
//!   one into place and only then deletes the old one. A restore backs the
//!   working set up into `.backup/`, writes a `.restore-pending` mark, and
//!   replaces each file through `<name>.u5slot.tmp` and a rename; any
//!   failure, or a crash found later, rolls the working set back from the
//!   backup. Every operation first repairs what a crash left behind, so a
//!   crash leaves either the old state or the new one, never a mix.
//! - **Exact restore.** A file the slot lacks is deleted from the working
//!   set, and so is the sidecar folder, so a slot imported from a DOS save
//!   (only the four original files) does not inherit the previous
//!   playthrough's `SAVED.WPS`, town mutations or sidecars.
//!
//! This is host code: no Bevy, no engine crate. The caller checks what the
//! bytes mean (the restore's `validate` hook) and fills the metadata.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paths::{SAVE_IMAGE_LEN, SIDECAR_DIR_NAME, WORKING_SET_FILES, work_key};

/// The `slot.toml` format this build writes and reads. A slot with a
/// higher number was made by a newer version and is listed as damaged.
pub const SLOT_FORMAT: u32 = 1;
/// The folder under the per-game slot root that holds one folder per slot.
pub const SLOTS_DIR: &str = "slots";
/// Each slot's metadata file.
pub const SLOT_FILE: &str = "slot.toml";
/// Each slot's optional 80x50 thumbnail (absent for imports).
pub const THUMB_FILE: &str = "thumb.png";
/// The last restore's copy of the working set.
pub const BACKUP_DIR: &str = ".backup";
/// The writer lock.
pub const LOCK_FILE: &str = ".lock";
/// Present from the moment a restore (or a roll-back) starts changing the
/// working set until it has finished; finding it means a crash, and the
/// working set is rolled back from [`BACKUP_DIR`].
pub const RESTORE_MARK: &str = ".restore-pending";
/// A lock older than this is a crashed writer's and is broken.
pub const LOCK_STALE_AFTER: Duration = Duration::from_secs(30);
/// How long an operation waits for another writer (design 3.2.2).
pub const LOCK_WAIT: Duration = Duration::from_secs(2);
/// The longest slot name, in printable ASCII characters (3.2.2).
pub const SLOT_NAME_MAX: usize = 24;
/// The quick slot's id and name.
pub const QUICK_SLOT_ID: &str = "quick";

const TMP_PREFIX: &str = ".tmp-";
const OLD_PREFIX: &str = ".old-";
const DEL_PREFIX: &str = ".del-";
const BACKUP_TMP_PREFIX: &str = ".backup.tmp-";
const BACKUP_OLD: &str = ".backup.old";
/// Suffix of a working file (or the sidecar folder) on its way in.
pub const WORKING_TMP_SUFFIX: &str = ".u5slot.tmp";
/// Suffix of the working sidecar folder on its way out.
const WORKING_OLD_SUFFIX: &str = ".u5slot.old";
const SAVED_GAM: &str = "SAVED.GAM";
const ID_PREFIX: char = 's';
const ID_MAX_LEN: usize = 32;

/// The fault points of a store, in order (tests only).
pub const STORE_STEPS: &[&str] = &[
    "store.locked",
    "store.copied",
    "store.synced",
    "store.old_renamed",
    "store.placed",
    "store.old_deleted",
];

/// The fault points of a restore, in order (tests only).
pub const RESTORE_STEPS: &[&str] = &[
    "restore.locked",
    "restore.verified",
    "restore.backed_up",
    "restore.marked",
    "restore.file.0",
    "restore.file.1",
    "restore.file.2",
    "restore.file.3",
    "restore.file.4",
    "restore.file.5",
    "restore.sidecar",
    "restore.validated",
    "restore.unmarked",
];

/// A slot's folder name: `quick`, or `s0001`, `s0002` and so on. Never the
/// player's name for it, so a rename touches only `slot.toml`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(String);

impl SlotId {
    /// An id as found on disk or typed: 1 to 32 of `a-z`, `0-9`, `-`, `_`.
    pub fn new(id: &str) -> Option<Self> {
        let valid = !id.is_empty()
            && id.len() <= ID_MAX_LEN
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
        valid.then(|| Self(id.to_string()))
    }

    pub fn quick() -> Self {
        Self(QUICK_SLOT_ID.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_quick(&self) -> bool {
        self.0 == QUICK_SLOT_ID
    }

    /// The number of an `sNNNN` id.
    fn number(&self) -> Option<u32> {
        self.0.strip_prefix(ID_PREFIX)?.parse().ok()
    }
}

impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How a slot was made (`slot.toml` `origin`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotOrigin {
    /// The menu's Save page, after a real Q save.
    #[default]
    Menu,
    /// The quick slot.
    Quick,
    /// The Save page's "Import current save": a copy without Q.
    Import,
    /// `--slot-save`: a copy without Q.
    Cli,
}

/// `slot.toml` (design 3.3.5). Unknown keys are ignored; a missing key
/// reads as its default, and a missing or unknown `format` marks the slot
/// damaged.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SlotMeta {
    pub format: u32,
    pub name: String,
    pub saved_at_unix: u64,
    pub engine: String,
    /// The source folder the working set came from, for display (D3.7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<String>,
    pub origin: SlotOrigin,
    pub avatar: String,
    /// Where the party stood (`area_status_label`): always a place the
    /// player has been (foundation 1.4.7).
    pub place: String,
    /// Year, month, day, hour, minute.
    pub clock: [u16; 5],
    pub turn: u64,
    /// Chapter 7's marker: a class (c) change happened in the session.
    pub modified: bool,
    pub cheats_used: Vec<String>,
    /// Every copied file's SHA-256, keyed by its path in the slot with `/`
    /// separators (`SAVED.GAM`, `.u5-engine-enh/automap.u5e`).
    pub sha256: BTreeMap<String, String>,
}

/// What the caller knows about the saved game (design 3.3.5): the Q-save
/// path fills it from the live state, the imports from the save image.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotDetails {
    pub avatar: String,
    pub place: String,
    pub clock: [u16; 5],
    pub turn: u64,
    pub modified: bool,
    pub cheats_used: Vec<String>,
}

/// Where a store goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotTarget {
    /// A new slot with this name; an empty name becomes "Slot N".
    New(String),
    /// Overwrite this slot, keeping its name.
    Existing(SlotId),
    /// The quick slot, made when missing.
    Quick,
}

/// One store (design 3.3.5 step 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRequest {
    pub target: SlotTarget,
    pub origin: SlotOrigin,
    /// This build's version, recorded as `engine`.
    pub engine: String,
    pub details: SlotDetails,
    pub thumbnail_png: Option<Vec<u8>>,
}

/// One listed slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotInfo {
    pub id: SlotId,
    pub meta: SlotMeta,
    /// Why the slot cannot be loaded (a missing file, the wrong length, a
    /// digest mismatch, an unreadable `slot.toml`). A damaged slot is
    /// listed greyed and can be deleted, never loaded (3.2.3).
    pub damaged: Option<String>,
    pub has_thumbnail: bool,
}

impl SlotInfo {
    /// The name to show: the stored one, or the id when `slot.toml` could
    /// not be read.
    pub fn display_name(&self) -> &str {
        if self.meta.name.is_empty() {
            self.id.as_str()
        } else {
            &self.meta.name
        }
    }
}

/// Why a slot operation did not happen. The `Display` text is a sentence
/// for a toast or the terminal.
#[derive(Debug)]
pub enum SlotError {
    /// Another writer held the lock for [`LOCK_WAIT`].
    Busy,
    /// The slot is damaged; it can be deleted but not loaded.
    Damaged {
        name: String,
        reason: String,
    },
    /// No slot with that id or name.
    NotFound(String),
    /// The name is not 1 to 24 printable ASCII characters, or the quick
    /// slot's.
    InvalidName(String),
    /// The slot root would sit in a game folder, the source tree or the
    /// mirror, or the operation does not apply.
    Refused(String),
    /// The restored working set failed the caller's check; it was rolled
    /// back.
    Invalid(String),
    Io(io::Error),
    /// The test fault hook stopped the operation here, as a crash would:
    /// nothing was cleaned up and the lock was left behind.
    Crashed(&'static str),
}

impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => f.write_str("Slots are busy (another copy of the game?)"),
            Self::Damaged { name, reason } => {
                write!(f, "Slot {name} is damaged ({reason}) and cannot be loaded")
            }
            Self::NotFound(name) => write!(f, "There is no slot named {name}"),
            Self::InvalidName(reason) | Self::Refused(reason) | Self::Invalid(reason) => {
                f.write_str(reason)
            }
            Self::Io(error) => f.write_str(&u5_gamedata::friendly_io_error(error)),
            Self::Crashed(step) => write!(f, "stopped at {step} (test fault)"),
        }
    }
}

impl std::error::Error for SlotError {}

impl From<io::Error> for SlotError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// A test hook called at each named step; returning true stops the
/// operation there as a crash would.
#[cfg(test)]
pub(crate) type FaultHook = std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// The slot store of one working folder.
pub struct SlotStore {
    /// `saves_root/<work-key>`.
    root: PathBuf,
    /// The canonical working folder.
    working: PathBuf,
    source_dir: Option<PathBuf>,
    lock_wait: Duration,
    #[cfg(test)]
    fault: Option<FaultHook>,
}

impl fmt::Debug for SlotStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlotStore")
            .field("root", &self.root)
            .field("working", &self.working)
            .finish_non_exhaustive()
    }
}

impl SlotStore {
    /// The store for `working_dir`, under `saves_root/<work-key>`. Touches
    /// nothing on disk. Refused when the slot root would sit inside the
    /// working folder or inside any of `forbidden` (the pristine source
    /// tree, the mirror root): slots never go there (3.3.5).
    pub fn open(
        saves_root: &Path,
        working_dir: &Path,
        forbidden: &[&Path],
    ) -> Result<Self, SlotError> {
        let working = working_dir.canonicalize().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("game folder {}: {error}", working_dir.display()),
            )
        })?;
        let root = lenient_canonical(saves_root).join(work_key(&working));
        let mut fences = vec![working.clone()];
        fences.extend(forbidden.iter().map(|path| lenient_canonical(path)));
        if let Some(fence) = fences.iter().find(|fence| root.starts_with(fence)) {
            return Err(SlotError::Refused(format!(
                "Save slots cannot be kept inside {}",
                fence.display()
            )));
        }
        Ok(Self {
            root,
            working,
            source_dir: None,
            lock_wait: LOCK_WAIT,
            #[cfg(test)]
            fault: None,
        })
    }

    /// Records the source folder in each new `slot.toml` (D3.7).
    pub fn with_source_dir(mut self, source: &Path) -> Self {
        self.source_dir = Some(source.to_path_buf());
        self
    }

    /// How long to wait for another writer; [`LOCK_WAIT`] by default.
    pub fn with_lock_wait(mut self, wait: Duration) -> Self {
        self.lock_wait = wait;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_fault(mut self, hook: FaultHook) -> Self {
        self.fault = Some(hook);
        self
    }

    /// `saves_root/<work-key>`.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The canonical working folder the slots copy.
    pub fn working_dir(&self) -> &Path {
        &self.working
    }

    /// Every slot, the quick slot first and then newest first, after
    /// repairing what a crashed writer left (3.3.5): stray `.tmp-*` slots
    /// are deleted, an `.old-*` whose slot is missing is put back, and an
    /// unfinished restore is rolled back.
    pub fn list(&self) -> Result<Vec<SlotInfo>, SlotError> {
        self.locked(|store| {
            store.repair()?;
            store.list_unlocked()
        })
    }

    /// The slot with this id, or else the newest whose name matches,
    /// ignoring ASCII case.
    pub fn find(&self, id_or_name: &str) -> Result<SlotInfo, SlotError> {
        let slots = self.list()?;
        let wanted = id_or_name.trim();
        slots
            .iter()
            .find(|slot| slot.id.as_str() == wanted)
            .or_else(|| {
                slots
                    .iter()
                    .find(|slot| slot.meta.name.eq_ignore_ascii_case(wanted))
            })
            .cloned()
            .ok_or_else(|| SlotError::NotFound(wanted.to_string()))
    }

    /// Copies the working set into a slot (design 3.3.5 step 6). The old
    /// slot, if any, stays whole until the new one is in place.
    pub fn store(&self, request: &StoreRequest) -> Result<SlotInfo, SlotError> {
        if let SlotTarget::New(name) = &request.target
            && !name.trim().is_empty()
        {
            check_name(name)?;
        }
        self.locked(|store| store.store_locked(request))
    }

    /// Makes the working set exactly equal to the slot (3.3.5 "Restore"):
    /// verify, back up, replace or delete each file, replace or delete the
    /// sidecar folder, then `validate` the working folder. A failure rolls
    /// the working set back byte for byte. A damaged slot is refused and
    /// nothing is touched.
    pub fn restore(
        &self,
        id: &SlotId,
        validate: &dyn Fn(&Path) -> Result<(), String>,
    ) -> Result<(), SlotError> {
        self.locked(|store| store.restore_locked(id, validate))
    }

    /// Puts the working set back as the last restore found it, from
    /// `.backup/` (a failed relaunch, 3.3.5).
    pub fn roll_back_restore(&self) -> Result<(), SlotError> {
        self.locked(|store| {
            store.repair()?;
            if !store.root.join(BACKUP_DIR).is_dir() {
                return Err(SlotError::Refused("There is no load to undo.".to_string()));
            }
            store.roll_back()
        })
    }

    /// Renames a slot; only `slot.toml` changes. The quick slot keeps its
    /// name.
    pub fn rename(&self, id: &SlotId, name: &str) -> Result<SlotInfo, SlotError> {
        let name = check_name(name)?;
        if id.is_quick() {
            return Err(SlotError::InvalidName(
                "The quick slot keeps its name.".to_string(),
            ));
        }
        self.locked(|store| {
            store.repair()?;
            let dir = store.slot_dir(id);
            if !dir.is_dir() {
                return Err(SlotError::NotFound(id.to_string()));
            }
            let mut meta = read_meta(&dir).map_err(|reason| SlotError::Damaged {
                name: id.to_string(),
                reason,
            })?;
            meta.name = name.clone();
            let text = toml::to_string(&meta).map_err(io::Error::other)?;
            let tmp = dir.join(format!("{SLOT_FILE}{WORKING_TMP_SUFFIX}"));
            write_synced(&tmp, text.as_bytes())?;
            fs::rename(&tmp, dir.join(SLOT_FILE))?;
            sync_dir(&dir)?;
            store.read_info(id)
        })
    }

    /// Deletes a slot, damaged or not.
    pub fn delete(&self, id: &SlotId) -> Result<(), SlotError> {
        self.locked(|store| {
            store.repair()?;
            let dir = store.slot_dir(id);
            if !dir.is_dir() {
                return Err(SlotError::NotFound(id.to_string()));
            }
            let doomed = store
                .slots_dir()
                .join(format!("{DEL_PREFIX}{id}-{}", std::process::id()));
            fs::rename(&dir, &doomed)?;
            sync_dir(&store.slots_dir())?;
            fs::remove_dir_all(&doomed)?;
            Ok(())
        })
    }

    // --- locking -------------------------------------------------------

    /// Runs `body` under the lock. A crash (the test hook) leaves the lock
    /// and everything else behind; any other failure repairs what it can
    /// before the lock goes.
    fn locked<T>(&self, body: impl FnOnce(&Self) -> Result<T, SlotError>) -> Result<T, SlotError> {
        let guard = self.lock()?;
        let result = body(self);
        match &result {
            Err(SlotError::Crashed(_)) => guard.leak(),
            Err(_) => {
                let _ = self.repair();
            }
            Ok(_) => {}
        }
        result
    }

    fn lock(&self) -> Result<LockGuard, SlotError> {
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(LOCK_FILE);
        let deadline = Instant::now() + self.lock_wait;
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let _ = writeln!(file, "pid {}", std::process::id());
                    let _ = file.sync_all();
                    return Ok(LockGuard { path, held: true });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if lock_is_stale(&path) {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if Instant::now() >= deadline {
                        return Err(SlotError::Busy);
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn fault(&self, _step: &'static str) -> Result<(), SlotError> {
        #[cfg(test)]
        if self.fault.as_ref().is_some_and(|hook| hook(_step)) {
            return Err(SlotError::Crashed(_step));
        }
        Ok(())
    }

    // --- layout --------------------------------------------------------

    fn slots_dir(&self) -> PathBuf {
        self.root.join(SLOTS_DIR)
    }

    fn slot_dir(&self, id: &SlotId) -> PathBuf {
        self.slots_dir().join(id.as_str())
    }

    /// The working set's files that exist, by their canonical name. An
    /// exact-case match wins over another spelling.
    fn working_files(&self) -> io::Result<HashMap<&'static str, PathBuf>> {
        let mut found: HashMap<&'static str, PathBuf> = HashMap::new();
        for entry in fs::read_dir(&self.working)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let Some(canonical) = WORKING_SET_FILES
                .iter()
                .copied()
                .find(|candidate| candidate.eq_ignore_ascii_case(name))
            else {
                continue;
            };
            if name == canonical || !found.contains_key(canonical) {
                found.insert(canonical, entry.path());
            }
        }
        Ok(found)
    }

    // --- listing and repair --------------------------------------------

    fn list_unlocked(&self) -> Result<Vec<SlotInfo>, SlotError> {
        let mut slots = Vec::new();
        let dir = self.slots_dir();
        if !dir.is_dir() {
            return Ok(slots);
        }
        for id in slot_ids(&dir)? {
            slots.push(self.read_info(&id)?);
        }
        slots.sort_by(|a, b| {
            b.id.is_quick()
                .cmp(&a.id.is_quick())
                .then(b.meta.saved_at_unix.cmp(&a.meta.saved_at_unix))
                .then(b.id.cmp(&a.id))
        });
        Ok(slots)
    }

    fn read_info(&self, id: &SlotId) -> Result<SlotInfo, SlotError> {
        let dir = self.slot_dir(id);
        if !dir.is_dir() {
            return Err(SlotError::NotFound(id.to_string()));
        }
        let (meta, damaged) = match read_meta(&dir) {
            Ok(meta) => {
                let damaged = verify_slot(&dir, &meta);
                (meta, damaged)
            }
            Err(reason) => (SlotMeta::default(), Some(reason)),
        };
        Ok(SlotInfo {
            id: id.clone(),
            has_thumbnail: dir.join(THUMB_FILE).is_file(),
            meta,
            damaged,
        })
    }

    /// Undoes what a crash left (under the lock): an unfinished restore or
    /// roll-back is rolled back, stray tmp and delete folders go, an
    /// `.old-<id>` returns when its slot is missing, and half-made backups
    /// and working tmp files are cleared.
    fn repair(&self) -> Result<(), SlotError> {
        // The backup swap first: the roll-back below reads `.backup/`.
        let backup = self.root.join(BACKUP_DIR);
        let backup_old = self.root.join(BACKUP_OLD);
        if self.root.is_dir() {
            for entry in fs::read_dir(&self.root)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(BACKUP_TMP_PREFIX) {
                    remove_any(&entry.path())?;
                }
            }
        }
        if backup_old.exists() {
            if backup.exists() {
                remove_any(&backup_old)?;
            } else {
                fs::rename(&backup_old, &backup)?;
            }
        }
        if self.root.join(RESTORE_MARK).exists() {
            self.roll_back()?;
        }
        let slots = self.slots_dir();
        if slots.is_dir() {
            for entry in fs::read_dir(&slots)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(TMP_PREFIX) || name.starts_with(DEL_PREFIX) {
                    remove_any(&entry.path())?;
                } else if let Some(id) = name.strip_prefix(OLD_PREFIX) {
                    let live = slots.join(id);
                    if live.exists() || SlotId::new(id).is_none() {
                        remove_any(&entry.path())?;
                    } else {
                        fs::rename(entry.path(), &live)?;
                    }
                }
            }
            sync_dir(&slots)?;
        }
        self.clear_working_leftovers()?;
        Ok(())
    }

    fn clear_working_leftovers(&self) -> io::Result<()> {
        for entry in fs::read_dir(&self.working)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let ours = name.ends_with(WORKING_TMP_SUFFIX) || name.ends_with(WORKING_OLD_SUFFIX);
            let stem = name
                .strip_suffix(WORKING_TMP_SUFFIX)
                .or_else(|| name.strip_suffix(WORKING_OLD_SUFFIX))
                .unwrap_or("");
            let known = stem == SIDECAR_DIR_NAME
                || WORKING_SET_FILES
                    .iter()
                    .any(|file| file.eq_ignore_ascii_case(stem));
            if ours && known {
                remove_any(&entry.path())?;
            }
        }
        Ok(())
    }

    // --- store ---------------------------------------------------------

    fn store_locked(&self, request: &StoreRequest) -> Result<SlotInfo, SlotError> {
        self.repair()?;
        self.fault("store.locked")?;
        let slots = self.slots_dir();
        fs::create_dir_all(&slots)?;
        let (id, name) = match &request.target {
            SlotTarget::Quick => (SlotId::quick(), QUICK_SLOT_ID.to_string()),
            SlotTarget::Existing(id) => {
                let dir = self.slot_dir(id);
                if !dir.is_dir() {
                    return Err(SlotError::NotFound(id.to_string()));
                }
                let name = read_meta(&dir)
                    .ok()
                    .map(|meta| meta.name)
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| id.to_string());
                (id.clone(), name)
            }
            SlotTarget::New(name) => {
                let id = next_id(&slots)?;
                let name = if name.trim().is_empty() {
                    format!("Slot {}", id.number().unwrap_or(0))
                } else {
                    check_name(name)?
                };
                (id, name)
            }
        };
        let files = self.working_files()?;
        if !files.contains_key(SAVED_GAM) {
            return Err(SlotError::Refused(
                "There is no saved game to copy.".to_string(),
            ));
        }

        let tmp = slots.join(format!("{TMP_PREFIX}{id}-{}", std::process::id()));
        remove_any(&tmp)?;
        fs::create_dir(&tmp)?;
        let mut sha256 = BTreeMap::new();
        for name in WORKING_SET_FILES {
            if let Some(path) = files.get(name) {
                let bytes = fs::read(path)?;
                write_synced(&tmp.join(name), &bytes)?;
                sha256.insert(name.to_string(), u5_gamedata::sha256::sha256_hex(&bytes));
            }
        }
        let sidecar = self.working.join(SIDECAR_DIR_NAME);
        if sidecar.is_dir() {
            copy_tree(
                &sidecar,
                &tmp.join(SIDECAR_DIR_NAME),
                SIDECAR_DIR_NAME,
                &mut Some(&mut sha256),
            )?;
        }
        if let Some(png) = &request.thumbnail_png {
            write_synced(&tmp.join(THUMB_FILE), png)?;
        }
        let details = &request.details;
        let meta = SlotMeta {
            format: SLOT_FORMAT,
            name,
            saved_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            engine: request.engine.clone(),
            source_dir: self
                .source_dir
                .as_ref()
                .map(|dir| dir.to_string_lossy().into_owned()),
            origin: request.origin,
            avatar: details.avatar.clone(),
            place: details.place.clone(),
            clock: details.clock,
            turn: details.turn,
            modified: details.modified,
            cheats_used: details.cheats_used.clone(),
            sha256,
        };
        let text = toml::to_string(&meta).map_err(io::Error::other)?;
        write_synced(&tmp.join(SLOT_FILE), text.as_bytes())?;
        self.fault("store.copied")?;

        sync_dir(&tmp)?;
        self.fault("store.synced")?;

        let live = self.slot_dir(&id);
        let old = slots.join(format!("{OLD_PREFIX}{id}"));
        if live.exists() {
            remove_any(&old)?;
            fs::rename(&live, &old)?;
            sync_dir(&slots)?;
        }
        self.fault("store.old_renamed")?;

        fs::rename(&tmp, &live)?;
        sync_dir(&slots)?;
        self.fault("store.placed")?;

        remove_any(&old)?;
        sync_dir(&slots)?;
        self.fault("store.old_deleted")?;
        self.read_info(&id)
    }

    // --- restore -------------------------------------------------------

    fn restore_locked(
        &self,
        id: &SlotId,
        validate: &dyn Fn(&Path) -> Result<(), String>,
    ) -> Result<(), SlotError> {
        self.repair()?;
        self.fault("restore.locked")?;
        let info = self.read_info(id)?;
        if let Some(reason) = info.damaged.clone() {
            return Err(SlotError::Damaged {
                name: info.display_name().to_string(),
                reason,
            });
        }
        self.fault("restore.verified")?;
        self.back_up_working_set()?;
        self.fault("restore.backed_up")?;
        self.set_mark("restore")?;
        self.fault("restore.marked")?;

        let applied = self
            .apply_set(&self.slot_dir(id), "restore")
            .and_then(|()| validate(&self.working).map_err(SlotError::Invalid));
        match applied {
            Err(crash @ SlotError::Crashed(_)) => return Err(crash),
            Err(error) => {
                self.roll_back()?;
                return Err(error);
            }
            Ok(()) => {}
        }
        self.fault("restore.validated")?;
        self.clear_mark()?;
        self.fault("restore.unmarked")?;
        Ok(())
    }

    /// Copies the working set as it is into `.backup/`, through a tmp
    /// folder, so `.backup/` is always one whole set. A file absent from
    /// the backup was absent from the working set.
    fn back_up_working_set(&self) -> Result<(), SlotError> {
        let tmp = self
            .root
            .join(format!("{BACKUP_TMP_PREFIX}{}", std::process::id()));
        remove_any(&tmp)?;
        fs::create_dir_all(&tmp)?;
        for (name, path) in self.working_files()? {
            let bytes = fs::read(path)?;
            write_synced(&tmp.join(name), &bytes)?;
        }
        let sidecar = self.working.join(SIDECAR_DIR_NAME);
        if sidecar.is_dir() {
            copy_tree(&sidecar, &tmp.join(SIDECAR_DIR_NAME), "", &mut None)?;
        }
        sync_dir(&tmp)?;
        let backup = self.root.join(BACKUP_DIR);
        let old = self.root.join(BACKUP_OLD);
        if backup.exists() {
            remove_any(&old)?;
            fs::rename(&backup, &old)?;
        }
        fs::rename(&tmp, &backup)?;
        sync_dir(&self.root)?;
        remove_any(&old)?;
        Ok(())
    }

    /// Makes the working set equal to the set in `source` (a slot or the
    /// backup): each file through `<name>.u5slot.tmp` and a rename, or
    /// deleted when `source` lacks it; then the sidecar folder, replaced
    /// whole or deleted.
    fn apply_set(&self, source: &Path, steps: &'static str) -> Result<(), SlotError> {
        let existing = self.working_files()?;
        for (index, name) in WORKING_SET_FILES.iter().enumerate() {
            let target = existing
                .get(name)
                .cloned()
                .unwrap_or_else(|| self.working.join(name));
            let from = source.join(name);
            if from.is_file() {
                let bytes = fs::read(&from)?;
                let tmp = self.working.join(format!("{name}{WORKING_TMP_SUFFIX}"));
                write_synced(&tmp, &bytes)?;
                fs::rename(&tmp, &target)?;
            } else if target.exists() {
                fs::remove_file(&target)?;
            }
            self.fault(file_step(steps, index))?;
        }
        let sidecar = self.working.join(SIDECAR_DIR_NAME);
        let from = source.join(SIDECAR_DIR_NAME);
        if from.is_dir() {
            let tmp = self
                .working
                .join(format!("{SIDECAR_DIR_NAME}{WORKING_TMP_SUFFIX}"));
            let old = self
                .working
                .join(format!("{SIDECAR_DIR_NAME}{WORKING_OLD_SUFFIX}"));
            remove_any(&tmp)?;
            copy_tree(&from, &tmp, "", &mut None)?;
            remove_any(&old)?;
            if sidecar.exists() {
                fs::rename(&sidecar, &old)?;
            }
            fs::rename(&tmp, &sidecar)?;
            remove_any(&old)?;
        } else {
            remove_any(&sidecar)?;
        }
        sync_dir(&self.working)?;
        self.fault(if steps == "restore" {
            "restore.sidecar"
        } else {
            "rollback.sidecar"
        })?;
        Ok(())
    }

    /// Puts the working set back from `.backup/`, under the mark so a
    /// crash part way is finished by the next repair.
    fn roll_back(&self) -> Result<(), SlotError> {
        let backup = self.root.join(BACKUP_DIR);
        if !backup.is_dir() {
            // A mark without a backup cannot happen (the mark follows the
            // backup); nothing was changed, so only the mark goes.
            return self.clear_mark();
        }
        self.set_mark("rollback")?;
        self.apply_set(&backup, "rollback")?;
        self.clear_mark()?;
        self.clear_working_leftovers()?;
        Ok(())
    }

    fn set_mark(&self, what: &str) -> Result<(), SlotError> {
        write_synced(&self.root.join(RESTORE_MARK), what.as_bytes())?;
        sync_dir(&self.root)?;
        Ok(())
    }

    fn clear_mark(&self) -> Result<(), SlotError> {
        remove_any(&self.root.join(RESTORE_MARK))?;
        sync_dir(&self.root)?;
        Ok(())
    }
}

/// The per-file fault point names.
fn file_step(steps: &str, index: usize) -> &'static str {
    const RESTORE: [&str; 6] = [
        "restore.file.0",
        "restore.file.1",
        "restore.file.2",
        "restore.file.3",
        "restore.file.4",
        "restore.file.5",
    ];
    const ROLLBACK: [&str; 6] = [
        "rollback.file.0",
        "rollback.file.1",
        "rollback.file.2",
        "rollback.file.3",
        "rollback.file.4",
        "rollback.file.5",
    ];
    if steps == "restore" {
        RESTORE[index]
    } else {
        ROLLBACK[index]
    }
}

/// Removes the lock when dropped, unless a crash (the test hook) leaked it.
struct LockGuard {
    path: PathBuf,
    held: bool,
}

impl LockGuard {
    fn leak(mut self) {
        self.held = false;
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if self.held {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Whether the lock is older than [`LOCK_STALE_AFTER`], or gone.
fn lock_is_stale(path: &Path) -> bool {
    match fs::metadata(path).and_then(|meta| meta.modified()) {
        Ok(modified) => SystemTime::now()
            .duration_since(modified)
            .is_ok_and(|age| age >= LOCK_STALE_AFTER),
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

/// A slot name as stored: trimmed, 1 to [`SLOT_NAME_MAX`] printable ASCII
/// characters.
pub fn check_name(name: &str) -> Result<String, SlotError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(SlotError::InvalidName("A slot needs a name.".to_string()));
    }
    if name.len() > SLOT_NAME_MAX {
        return Err(SlotError::InvalidName(format!(
            "A slot name has at most {SLOT_NAME_MAX} letters."
        )));
    }
    if !name.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(SlotError::InvalidName(
            "A slot name uses plain letters, digits and signs only.".to_string(),
        ));
    }
    Ok(name.to_string())
}

/// The ids of every live slot folder (skipping `.tmp-*`, `.old-*` and
/// anything else that is not an id).
fn slot_ids(dir: &Path) -> io::Result<Vec<SlotId>> {
    let mut ids = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if let Some(id) = entry.file_name().to_str().and_then(SlotId::new) {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// The next `sNNNN` id.
fn next_id(dir: &Path) -> io::Result<SlotId> {
    let highest = slot_ids(dir)?
        .iter()
        .filter_map(SlotId::number)
        .max()
        .unwrap_or(0);
    Ok(SlotId(format!("{ID_PREFIX}{:04}", highest + 1)))
}

/// `slot.toml`, or why it cannot be read.
fn read_meta(dir: &Path) -> Result<SlotMeta, String> {
    let text = fs::read_to_string(dir.join(SLOT_FILE))
        .map_err(|_| format!("{SLOT_FILE} is missing or unreadable"))?;
    toml::from_str(&text).map_err(|_| format!("{SLOT_FILE} is not valid"))
}

/// Why a slot cannot be loaded, if it cannot (3.2.3): an unknown format,
/// a missing file, `SAVED.GAM` of the wrong length, a digest mismatch, or
/// a working-set or sidecar file `slot.toml` does not list.
fn verify_slot(dir: &Path, meta: &SlotMeta) -> Option<String> {
    if meta.format == 0 {
        return Some(format!("{SLOT_FILE} has no format number"));
    }
    if meta.format > SLOT_FORMAT {
        return Some("it was made by a newer version".to_string());
    }
    if !meta.sha256.contains_key(SAVED_GAM) {
        return Some(format!("{SAVED_GAM} is missing"));
    }
    for (key, digest) in &meta.sha256 {
        let Some(path) = slot_path(dir, key) else {
            return Some(format!("{SLOT_FILE} names a bad path"));
        };
        let Ok(bytes) = fs::read(&path) else {
            return Some(format!("{key} is missing"));
        };
        if key == SAVED_GAM && bytes.len() as u64 != SAVE_IMAGE_LEN {
            return Some(format!("{SAVED_GAM} has the wrong length"));
        }
        if u5_gamedata::sha256::sha256_hex(&bytes) != *digest {
            return Some(format!("{key} does not match its digest"));
        }
    }
    for name in WORKING_SET_FILES {
        if dir.join(name).exists() && !meta.sha256.contains_key(name) {
            return Some(format!("{name} is not listed in {SLOT_FILE}"));
        }
    }
    let sidecar = dir.join(SIDECAR_DIR_NAME);
    if sidecar.is_dir() {
        let mut found = BTreeMap::new();
        if tree_keys(&sidecar, SIDECAR_DIR_NAME, &mut found).is_err() {
            return Some(format!("{SIDECAR_DIR_NAME} is unreadable"));
        }
        if let Some(key) = found.keys().find(|key| !meta.sha256.contains_key(*key)) {
            return Some(format!("{key} is not listed in {SLOT_FILE}"));
        }
    }
    None
}

/// A `slot.toml` key as a path inside the slot, or `None` when it would
/// leave the slot.
fn slot_path(dir: &Path, key: &str) -> Option<PathBuf> {
    let relative = Path::new(key);
    let plain = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    (plain && !key.is_empty()).then(|| dir.join(relative))
}

/// The digest keys of every file under `dir`, as `prefix/relative/path`.
fn tree_keys(dir: &Path, prefix: &str, out: &mut BTreeMap<String, ()>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let key = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if entry.file_type()?.is_dir() {
            tree_keys(&entry.path(), &key, out)?;
        } else {
            out.insert(key, ());
        }
    }
    Ok(())
}

/// Copies the tree `from` to the new folder `to`, syncing each file and
/// folder. With `digests`, each file's SHA-256 is recorded under
/// `prefix/relative/path`.
fn copy_tree(
    from: &Path,
    to: &Path,
    prefix: &str,
    digests: &mut Option<&mut BTreeMap<String, String>>,
) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let key = format!("{prefix}/{}", name.to_string_lossy());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &to.join(&name), &key, digests)?;
        } else if kind.is_file() {
            let bytes = fs::read(entry.path())?;
            write_synced(&to.join(&name), &bytes)?;
            if let Some(digests) = digests.as_deref_mut() {
                digests.insert(key, u5_gamedata::sha256::sha256_hex(&bytes));
            }
        }
    }
    sync_dir(to)
}

/// Writes a file and `fsync`s it.
fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// `fsync`s a folder, so renames and new entries in it are durable. Only
/// Unix can open a folder for this; elsewhere the renames are left to the
/// file system.
fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}

/// Removes a file or a folder tree; a missing path is fine.
fn remove_any(path: &Path) -> io::Result<()> {
    let result = match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) => Err(error),
    };
    match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// `path` with its longest existing ancestor canonicalised, so a folder
/// that does not exist yet can be compared with ones that do.
fn lenient_canonical(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|dir| dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let mut existing = absolute.as_path();
    let mut rest = Vec::new();
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            let mut out = canonical;
            for part in rest.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return absolute,
        }
    }
}

#[cfg(test)]
#[path = "slots_tests.rs"]
mod tests;
