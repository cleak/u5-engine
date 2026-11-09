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
//! The second half is the store (design 1.3.10): [`SidecarStore`] keeps
//! each feature's file in [`SIDECAR_DIR`] beside `SAVED.GAM`, in one
//! container format (magic `U5EH`, a feature tag, the feature's payload
//! version, the [`SaveStamp`] of the save it belongs to, the payload and a
//! trailing FNV-1a-64), and reads it back with its [`Freshness`] against
//! the save now on disk:
//!
//! - It is written only after a detected save, atomically (a temporary
//!   file written through [`write_disk_file`], synced, then renamed over
//!   the old one, and the folder synced on Unix), so a crash leaves the
//!   old file or the new one, never half of one.
//! - A file that does not decode is [`Freshness::Corrupt`]; reading it
//!   renames it to `<name>.bad`, keeping one copy for inspection.
//! - A file from a newer build (a newer container, or a payload version
//!   newer than the writer's) is read-only here: it is never overwritten
//!   or renamed.
//! - Errors never fail a save. The store only touches its own folder; it
//!   never changes `SAVED.GAM` or another game file.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::catalogue::{FeatureToggles, Toggle};
use crate::disk_io::write_disk_file;
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

/// The per-save enhancement folder, beside `SAVED.GAM` (design 1.3.10).
/// Host code reuses it: the runtime mirror never copies this folder from a
/// source folder over the mirror's own.
pub const SIDECAR_DIR: &str = ".u5-engine-enh";

/// Chapter 4's exploration memory, `automap.u5e`.
pub const TAG_AUTOMAP: [u8; 4] = *b"AMAP";
/// The save's flags, `save-meta.u5e` ([`SaveMeta`]).
pub const TAG_SAVE_META: [u8; 4] = *b"META";
/// Chapter 8's clue log, `journal.u5e`.
pub const TAG_JOURNAL: [u8; 4] = *b"QJRN";
/// Chapter 10's gem map discoveries, `gemmap.u5e`.
pub const TAG_GEM_MAP: [u8; 4] = *b"GEMK";

/// Every per-save file and its feature tag. No other per-save file name
/// exists.
pub const SIDECAR_FILES: &[([u8; 4], &str)] = &[
    (TAG_AUTOMAP, "automap.u5e"),
    (TAG_SAVE_META, "save-meta.u5e"),
    (TAG_JOURNAL, "journal.u5e"),
    (TAG_GEM_MAP, "gemmap.u5e"),
];

/// The file name a tag is kept under, or `None` for a tag no feature owns.
pub fn sidecar_file_name(tag: [u8; 4]) -> Option<&'static str> {
    SIDECAR_FILES
        .iter()
        .find(|(known, _)| *known == tag)
        .map(|(_, name)| *name)
}

/// The container's first four bytes.
pub const CONTAINER_MAGIC: [u8; 4] = *b"U5EH";
/// The container version this build reads and writes. It gates the header
/// only; each feature versions its own payload.
pub const CONTAINER_VERSION: u8 = 1;
/// Magic, version, tag, payload version, stamp (FNV, length, Avatar name,
/// date) and payload length.
const HEADER_LEN: usize = 42;
const CHECKSUM_LEN: usize = 8;

/// A decoded container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub tag: [u8; 4],
    pub payload_version: u16,
    pub stamp: SaveStamp,
    pub payload: Vec<u8>,
}

/// Why bytes are not a container this build can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerError {
    /// Shorter than its header or than the length it declares.
    Truncated,
    /// Not `U5EH`.
    BadMagic,
    /// Container version zero.
    BadVersion,
    /// Written by a newer build; its header is not this build's to read.
    NewerContainer(u8),
    /// Longer than the length it declares.
    TrailingBytes,
    /// The trailing FNV-1a-64 does not match.
    BadChecksum,
}

/// The container bytes for one feature's payload (all integers little
/// endian):
///
/// ```text
/// off  size  field
///   0     4  magic "U5EH"
///   4     1  container version (1)
///   5     4  feature tag, ASCII
///   9     2  payload version
///  11     8  stamp: FNV-1a-64 of the SAVED.GAM bytes
///  19     4  SAVED.GAM length
///  23     9  Avatar name (lineage)
///  32     6  game date: year u16, month, day, hour, minute
///  38     4  payload length n
///  42     n  payload (owned by the feature)
/// 42+n    8  FNV-1a-64 of bytes 0..42+n
/// ```
pub fn encode_container(
    tag: [u8; 4],
    stamp: &SaveStamp,
    payload_version: u16,
    payload: &[u8],
) -> io::Result<Vec<u8>> {
    let payload_len = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "sidecar payload too large"))?;
    let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len() + CHECKSUM_LEN);
    bytes.extend_from_slice(&CONTAINER_MAGIC);
    bytes.push(CONTAINER_VERSION);
    bytes.extend_from_slice(&tag);
    bytes.extend_from_slice(&payload_version.to_le_bytes());
    bytes.extend_from_slice(&stamp.fnv.to_le_bytes());
    bytes.extend_from_slice(&stamp.len.to_le_bytes());
    bytes.extend_from_slice(&stamp.avatar);
    bytes.extend_from_slice(&stamp.date);
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(payload);
    let checksum = fnv1a64(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    Ok(bytes)
}

/// Reads container bytes back. See [`encode_container`] for the layout.
pub fn decode_container(bytes: &[u8]) -> Result<Container, ContainerError> {
    if bytes.len() < CONTAINER_MAGIC.len() + 1 {
        return Err(ContainerError::Truncated);
    }
    if bytes[..4] != CONTAINER_MAGIC {
        return Err(ContainerError::BadMagic);
    }
    match bytes[4] {
        0 => return Err(ContainerError::BadVersion),
        CONTAINER_VERSION => {}
        newer => return Err(ContainerError::NewerContainer(newer)),
    }
    if bytes.len() < HEADER_LEN {
        return Err(ContainerError::Truncated);
    }
    let array = |offset: usize| -> [u8; 8] {
        let mut out = [0; 8];
        out.copy_from_slice(&bytes[offset..offset + 8]);
        out
    };
    let u32_at = |offset: usize| {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    };
    let payload_len = u32_at(38) as usize;
    let body_len = HEADER_LEN + payload_len;
    let total = body_len + CHECKSUM_LEN;
    if bytes.len() < total {
        return Err(ContainerError::Truncated);
    }
    if bytes.len() > total {
        return Err(ContainerError::TrailingBytes);
    }
    if u64::from_le_bytes(array(body_len)) != fnv1a64(&bytes[..body_len]) {
        return Err(ContainerError::BadChecksum);
    }
    let mut tag = [0; 4];
    tag.copy_from_slice(&bytes[5..9]);
    let mut avatar = [0; 9];
    avatar.copy_from_slice(&bytes[23..32]);
    let mut date = [0; 6];
    date.copy_from_slice(&bytes[32..38]);
    Ok(Container {
        tag,
        payload_version: u16::from_le_bytes([bytes[9], bytes[10]]),
        stamp: SaveStamp {
            fnv: u64::from_le_bytes(array(11)),
            len: u32_at(19),
            avatar,
            date,
        },
        payload: bytes[HEADER_LEN..body_len].to_vec(),
    })
}

/// How a sidecar relates to the save now on disk (design 1.3.10). What to
/// do with each is the feature's choice; the defaults are: `Fresh` is used
/// as is, `SameLineage` is kept with a note, `Foreign` is ignored and
/// replaced at the next save, and `Corrupt` is replaced (the store already
/// renamed it to `.bad`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    /// Stamp equals the current SAVED.GAM.
    Fresh,
    /// Same Avatar, current date not earlier: saved elsewhere.
    SameLineage,
    /// Other Avatar, or an earlier date: another character.
    Foreign,
    Missing,
    Corrupt,
}

impl Freshness {
    /// A sidecar stamped `stamp` against the save stamped `now`.
    pub fn of(stamp: &SaveStamp, now: &SaveStamp) -> Self {
        if stamp.fnv == now.fnv && stamp.len == now.len {
            Self::Fresh
        } else if stamp.avatar == now.avatar && date_key(&now.date) >= date_key(&stamp.date) {
            Self::SameLineage
        } else {
            Self::Foreign
        }
    }
}

/// The calendar in a comparable order: year, month, day, hour, minute.
fn date_key(date: &[u8; 6]) -> (u16, u8, u8, u8, u8) {
    (
        u16::from_le_bytes([date[0], date[1]]),
        date[2],
        date[3],
        date[4],
        date[5],
    )
}

/// A sidecar's payload and the payload version it was written with. A
/// feature whose own version is older treats it as read-only: the memory
/// is used in RAM and the store never overwrites the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidecarPayload {
    pub version: u16,
    pub bytes: Vec<u8>,
}

/// What [`SidecarStore::write`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidecarWrite {
    Written,
    /// The file on disk is from a newer build (a newer container or payload
    /// version) and was left as it is.
    KeptNewer,
}

/// The sidecar folder of one save directory (design 1.3.10). Make one with
/// [`open`](Self::open) from the save folder the host handed in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidecarStore {
    dir: PathBuf,
}

impl SidecarStore {
    /// The store for `save_dir`, the folder holding `SAVED.GAM`. `None` when
    /// it is not a folder or its `SAVED.GAM` is read-only (a pristine or
    /// shared install): the features then run from memory. The folder
    /// itself is made on the first write.
    pub fn open(save_dir: &Path) -> Option<Self> {
        if !save_dir.is_dir() {
            return None;
        }
        if let Ok(meta) = fs::metadata(save_dir.join(SAVED_GAM_FILENAME))
            && meta.permissions().readonly()
        {
            return None;
        }
        Some(Self {
            dir: save_dir.join(SIDECAR_DIR),
        })
    }

    /// `<save dir>/.u5-engine-enh`.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The path a tag's file is kept at, or `None` for a tag no feature
    /// owns.
    pub fn path(&self, tag: [u8; 4]) -> Option<PathBuf> {
        sidecar_file_name(tag).map(|name| self.dir.join(name))
    }

    /// Reads a feature's sidecar and its freshness against `now`, the
    /// stamp of the save on disk. The payload comes back for every
    /// decodable file, `Foreign` included; the feature applies its policy.
    ///
    /// - No file, an unknown tag or an unreadable file: `Missing`.
    /// - A container from a newer build: `Missing`, and the file is kept.
    /// - Anything else that does not decode, or carries another tag:
    ///   `Corrupt`, and the file is renamed to `<name>.bad`.
    pub fn read(&self, tag: [u8; 4], now: &SaveStamp) -> (Freshness, Option<SidecarPayload>) {
        let Some(path) = self.path(tag) else {
            return (Freshness::Missing, None);
        };
        let Ok(bytes) = fs::read(&path) else {
            return (Freshness::Missing, None);
        };
        match decode_container(&bytes) {
            Ok(container) if container.tag == tag => (
                Freshness::of(&container.stamp, now),
                Some(SidecarPayload {
                    version: container.payload_version,
                    bytes: container.payload,
                }),
            ),
            Err(ContainerError::NewerContainer(_)) => (Freshness::Missing, None),
            _ => {
                let _ = fs::rename(&path, bad_path(&path));
                (Freshness::Corrupt, None)
            }
        }
    }

    /// Writes a feature's sidecar for the save stamped `stamp`: only after
    /// the [`SaveWatcher`] detected that save. Temporary file, sync,
    /// rename, sync the folder (Unix). A file from a newer build is kept
    /// ([`SidecarWrite::KeptNewer`]).
    ///
    /// An error here never fails the save: `SAVED.GAM` is already written
    /// and this never touches it. The caller logs it and shows it once.
    pub fn write(
        &self,
        tag: [u8; 4],
        stamp: &SaveStamp,
        payload_version: u16,
        payload: &[u8],
    ) -> io::Result<SidecarWrite> {
        self.write_staged(tag, stamp, payload_version, payload, true)
    }

    /// [`write`](Self::write), stopping before the rename when `rename` is
    /// false: what a crash between the two leaves behind.
    fn write_staged(
        &self,
        tag: [u8; 4],
        stamp: &SaveStamp,
        payload_version: u16,
        payload: &[u8],
        rename: bool,
    ) -> io::Result<SidecarWrite> {
        let path = self.path(tag).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "not a sidecar feature tag")
        })?;
        if let Ok(existing) = fs::read(&path) {
            match decode_container(&existing) {
                Err(ContainerError::NewerContainer(_)) => return Ok(SidecarWrite::KeptNewer),
                Ok(container)
                    if container.tag == tag && container.payload_version > payload_version =>
                {
                    return Ok(SidecarWrite::KeptNewer);
                }
                _ => {}
            }
        }
        let bytes = encode_container(tag, stamp, payload_version, payload)?;
        fs::create_dir_all(&self.dir)?;
        let temp = temp_path(&path);
        let result = write_disk_file(&temp, &bytes)
            .and_then(|_| fs::File::open(&temp)?.sync_all())
            .and_then(|()| {
                if rename {
                    fs::rename(&temp, &path)?;
                    sync_dir(&self.dir)?;
                }
                Ok(())
            });
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result.map(|()| SidecarWrite::Written)
    }

    /// A new character (creation or the Ultima IV transfer): removes the
    /// files of `tags`, the enabled features only. A disabled feature's
    /// file is left, and reads as `Foreign` when it is turned on, because a
    /// new character starts at the factory date.
    pub fn reset(&self, tags: &[[u8; 4]]) -> io::Result<()> {
        for path in tags.iter().filter_map(|tag| self.path(*tag)) {
            for path in [temp_path(&path), path] {
                match fs::remove_file(&path) {
                    Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// `<name>.bad`: the one kept copy of a corrupt sidecar.
fn bad_path(path: &Path) -> PathBuf {
    with_suffix(path, ".bad")
}

/// `<name>.tmp`: a write in progress.
fn temp_path(path: &Path) -> PathBuf {
    with_suffix(path, ".tmp")
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Makes the rename durable: syncs the folder on Unix. Elsewhere the
/// rename is all the platform offers.
fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// The payload version of [`SaveMeta`].
pub const SAVE_META_PAYLOAD_VERSION: u16 = 1;

/// `save-meta.u5e` (tag `META`): the save's flags, today only "cheats
/// used". Chapter 7 writes it only after a cheat was applied in the
/// session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveMeta {
    pub cheats_used: bool,
}

impl SaveMeta {
    const CHEATS_USED: u8 = 0x01;

    /// Payload version 1: one flags byte.
    pub fn encode(&self) -> Vec<u8> {
        vec![if self.cheats_used {
            Self::CHEATS_USED
        } else {
            0
        }]
    }

    /// Reads a payload of any version: the first byte holds the flags this
    /// build knows; unknown flags and later bytes are skipped. `None` for
    /// an empty payload.
    pub fn decode(payload: &[u8]) -> Option<Self> {
        let flags = *payload.first()?;
        Some(Self {
            cheats_used: flags & Self::CHEATS_USED != 0,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod store_tests;
