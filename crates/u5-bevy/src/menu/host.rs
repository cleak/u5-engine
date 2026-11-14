//! The menu's host services (design 3.3.1, 3.3.3).
//!
//! The shell never touches the file system for slots, the clock or the
//! OS: the host hands it an implementation of [`MenuHost`] in
//! [`VisualLaunchConfig::menu_host`](crate::VisualLaunchConfig::menu_host).
//! u5-tui implements it over u5-host-desktop's slot store
//! (`DesktopMenuHost`), as `DesktopWizardHost` does for the wizard; a
//! phone or console port supplies its own.
//!
//! The types here are the engine's own words for a slot. They carry only
//! what the Save and Load pages show and what a store needs; the host maps
//! them to and from its files.

use std::sync::mpsc::Receiver;

/// A slot's id as the host keys it (its folder name on the desktop). The
/// engine never makes one up: it only hands back ids the host listed.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(pub String);

impl SlotId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One slot as the host lists it (3.2.2, 3.2.3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotInfo {
    pub id: SlotId,
    /// The player's name for it, or `quick`.
    pub name: String,
    /// The quick slot (3.2.2), always listed first.
    pub quick: bool,
    /// Made from a DOS save or the command line without `Q`: no thumbnail
    /// and no turn count (3.3.5 "Imports and CLI").
    pub imported: bool,
    /// Why the slot cannot be loaded, for the log; a damaged slot is
    /// listed greyed and can only be deleted (3.2.3).
    pub damaged: Option<String>,
    /// The Avatar's name.
    pub avatar: String,
    /// Where the party stood, as published words (D-M4.8).
    pub place: String,
    /// The game's year, month, day, hour and minute.
    pub clock: [u16; 5],
    /// `PlayState.turn` at the save; 0 for an import.
    pub turn: u64,
    /// When it was saved, in seconds since 1970, for the order.
    pub saved_at_unix: u64,
    /// Chapter 7's marker: saved after a cheat changed the game.
    pub modified: bool,
    /// The 80x50 thumbnail, PNG-encoded, when the slot has one.
    pub thumbnail_png: Option<Vec<u8>>,
}

/// Where a store goes (3.3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotTarget {
    /// A new slot with this name; an empty name becomes "Slot N".
    New(String),
    /// Write over this slot, keeping its name.
    Existing(SlotId),
    /// The quick slot, made when missing.
    Quick,
}

/// One store: the just-written working set into a slot, with what the
/// slot shows (3.3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreSlot {
    pub target: SlotTarget,
    pub avatar: String,
    /// Published words (D-M4.8), never an internal scene key.
    pub place: String,
    /// Year, month, day, hour, minute.
    pub clock: [u16; 5],
    pub turn: u64,
    /// The cheats used this session (chapter 7); empty until it exists.
    pub cheats_used: Vec<String>,
    pub thumbnail_png: Option<Vec<u8>>,
}

/// Why a slot operation did not happen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotFailure {
    /// Another copy of the game held the slot lock past its wait (3.2.2).
    Busy,
    /// Anything else, as a short sentence for a toast: the OS's own error
    /// text or the store's reason.
    Reason(String),
}

impl std::fmt::Display for SlotFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => f.write_str("the saves are busy"),
            Self::Reason(reason) => f.write_str(reason),
        }
    }
}

/// What the menu asks of its host (3.3.3). Every settings write goes
/// through the foundation's `SettingsSink` instead, so this has slot
/// methods only.
pub trait MenuHost: Send + Sync {
    /// Every slot: the quick slot first, then newest first.
    fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotFailure>;
    /// Copies the just-written working set into a slot, off the game
    /// thread; the answer arrives on the receiver.
    fn store_slot(&self, request: StoreSlot) -> Receiver<Result<SlotInfo, SlotFailure>>;
    /// Backs the working set up, then makes it exactly equal to the slot
    /// and checks it (4,192 bytes, an active Avatar). On failure the host
    /// has already put the working set back as it was.
    fn restore_slot(&self, id: &SlotId) -> Result<(), SlotFailure>;
    /// Puts the working set back as the last restore found it.
    fn roll_back_restore(&self) -> Result<(), SlotFailure>;
    fn rename_slot(&self, id: &SlotId, name: &str) -> Result<(), SlotFailure>;
    fn delete_slot(&self, id: &SlotId) -> Result<(), SlotFailure>;
}

/// A slot store that keeps nothing, for tests that need only that there is
/// one: it lists no slots and refuses everything else.
#[cfg(test)]
pub(crate) struct NoStore;

#[cfg(test)]
impl MenuHost for NoStore {
    fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotFailure> {
        Ok(Vec::new())
    }

    fn store_slot(&self, _request: StoreSlot) -> Receiver<Result<SlotInfo, SlotFailure>> {
        let (send, reply) = std::sync::mpsc::channel();
        let _ = send.send(Err(SlotFailure::Reason("no store".to_string())));
        reply
    }

    fn restore_slot(&self, _id: &SlotId) -> Result<(), SlotFailure> {
        Err(SlotFailure::Reason("no store".to_string()))
    }

    fn roll_back_restore(&self) -> Result<(), SlotFailure> {
        Err(SlotFailure::Reason("no store".to_string()))
    }

    fn rename_slot(&self, _id: &SlotId, _name: &str) -> Result<(), SlotFailure> {
        Err(SlotFailure::Reason("no store".to_string()))
    }

    fn delete_slot(&self, _id: &SlotId) -> Result<(), SlotFailure> {
        Err(SlotFailure::Reason("no store".to_string()))
    }
}
