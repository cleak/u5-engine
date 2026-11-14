//! The menu's slot host on the desktop (Phase B design 3.3.1, 3.3.3,
//! 3.3.5, 3.3.9; build plan M4-11).
//!
//! [`DesktopMenuHost`] is the engine's `MenuHost` over u5-host-desktop's
//! slot store, as `DesktopWizardHost` is the wizard's host:
//!
//! - the store runs on a worker thread and answers on an `mpsc` channel,
//!   as the wizard's folder picker does, so a slow disk never holds the
//!   game's frame;
//! - a restore is checked with [`validate_working_set`] (4,192 bytes, an
//!   active Avatar) inside the store's own restore, which rolls the
//!   working set back by itself on any failure;
//! - listing, rename and delete are short and answer at once.
//!
//! [`attach_menu_host`] puts one in a launch config only when slots can be
//! used: `menu.named_slots` and a menu are on and the parity lock is off.
//! Otherwise the slot store is never opened (3.4.2).
//!
//! [`relaunch_after_shell`] is the one place a shell's exit is turned into
//! a restart; when the restart of a load fails it rolls the load back, so
//! the next launch finds the game unchanged (3.2.3 "A failed relaunch").

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use u5_bevy::menu::host::{
    MenuHost, SlotFailure, SlotId, SlotInfo, SlotTarget as EngineTarget, StoreSlot,
};
use u5_host_desktop::{
    SlotDetails, SlotError, SlotId as StoreId, SlotInfo as StoreInfo, SlotOrigin, SlotStore,
    SlotTarget, StoreRequest,
};
use u5_runtime::enhancements::LockState;

use crate::slot_commands::{open_slot_store, validate_working_set};

/// The engine's slot host over the desktop slot store.
pub struct DesktopMenuHost {
    store: Arc<SlotStore>,
}

impl DesktopMenuHost {
    pub fn new(store: SlotStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    /// The store, for the host's own uses (the relaunch's roll-back).
    pub fn store(&self) -> &SlotStore {
        &self.store
    }
}

fn failure(error: SlotError) -> SlotFailure {
    match error {
        SlotError::Busy => SlotFailure::Busy,
        other => SlotFailure::Reason(other.to_string()),
    }
}

fn store_id(id: &SlotId) -> Result<StoreId, SlotFailure> {
    StoreId::new(id.as_str()).ok_or_else(|| SlotFailure::Reason(format!("no slot {}", id.0)))
}

/// What the engine shows of a listed slot, its thumbnail read when it has
/// one.
fn engine_info(store: &SlotStore, slot: StoreInfo) -> SlotInfo {
    let thumbnail_png = slot
        .has_thumbnail
        .then(|| store.thumbnail(&slot.id))
        .flatten();
    let name = slot.display_name().to_string();
    let meta = slot.meta;
    SlotInfo {
        id: SlotId(slot.id.as_str().to_string()),
        name,
        quick: slot.id.is_quick(),
        imported: matches!(meta.origin, SlotOrigin::Import | SlotOrigin::Cli),
        damaged: slot.damaged,
        avatar: meta.avatar,
        place: meta.place,
        clock: meta.clock,
        turn: meta.turn,
        saved_at_unix: meta.saved_at_unix,
        modified: meta.modified,
        thumbnail_png,
    }
}

/// The store request for an engine store: a menu save, or the quick slot.
fn store_request(request: StoreSlot) -> Result<StoreRequest, SlotFailure> {
    let (target, origin) = match request.target {
        EngineTarget::New(name) => (SlotTarget::New(name), SlotOrigin::Menu),
        EngineTarget::Existing(id) => (SlotTarget::Existing(store_id(&id)?), SlotOrigin::Menu),
        EngineTarget::Quick => (SlotTarget::Quick, SlotOrigin::Quick),
    };
    Ok(StoreRequest {
        target,
        origin,
        engine: env!("CARGO_PKG_VERSION").to_string(),
        details: SlotDetails {
            avatar: request.avatar,
            place: request.place,
            clock: request.clock,
            turn: request.turn,
            modified: !request.cheats_used.is_empty(),
            cheats_used: request.cheats_used,
        },
        thumbnail_png: request.thumbnail_png,
    })
}

impl MenuHost for DesktopMenuHost {
    fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotFailure> {
        let slots = self.store.list().map_err(failure)?;
        Ok(slots
            .into_iter()
            .map(|slot| engine_info(&self.store, slot))
            .collect())
    }

    fn store_slot(&self, request: StoreSlot) -> Receiver<Result<SlotInfo, SlotFailure>> {
        let (send, reply) = mpsc::channel();
        let store = self.store.clone();
        let spawned = std::thread::Builder::new()
            .name("u5-slot-store".to_string())
            .spawn(move || {
                let answer = store_request(request).and_then(|request| {
                    store
                        .store(&request)
                        .map(|slot| engine_info(&store, slot))
                        .map_err(failure)
                });
                let _ = send.send(answer);
            });
        if let Err(error) = spawned {
            // The sender went with the closure: the engine sees the reply
            // disconnected and says the copy stopped.
            eprintln!("menu: could not start the slot copy: {error}");
        }
        reply
    }

    fn restore_slot(&self, id: &SlotId) -> Result<(), SlotFailure> {
        self.store
            .restore(&store_id(id)?, &validate_working_set)
            .map_err(failure)
    }

    fn roll_back_restore(&self) -> Result<(), SlotFailure> {
        self.store.roll_back_restore().map_err(failure)
    }

    fn rename_slot(&self, id: &SlotId, name: &str) -> Result<(), SlotFailure> {
        self.store
            .rename(&store_id(id)?, name)
            .map(drop)
            .map_err(failure)
    }

    fn delete_slot(&self, id: &SlotId) -> Result<(), SlotFailure> {
        self.store.delete(&store_id(id)?).map_err(failure)
    }
}

/// Gives `launch` the desktop slot host for the working folder `working`
/// (copied from `source`) when slots can be used: `menu.named_slots` and a
/// menu (the overlay or the title layer) are on and the parity lock is
/// off. A store that cannot be opened is reported and left out; the menu
/// then offers no slots.
pub fn attach_menu_host(launch: &mut u5_bevy::VisualLaunchConfig, working: &Path, source: &Path) {
    let resolved = &launch.enhancements;
    let menu = resolved.menu();
    if resolved.lock != LockState::Off || !menu.named_slots || !(menu.overlay || menu.title_layer) {
        return;
    }
    match open_slot_store(working, source) {
        Ok(store) => launch.menu_host = Some(Arc::new(DesktopMenuHost::new(store))),
        Err(error) => eprintln!("menu: save slots are not available: {error}"),
    }
}

/// Restarts the game when the shell asked (design 3.3.5 "Relaunch"). When
/// a load's restart cannot start, the load is rolled back through `host`
/// and the reason returned, so the next launch finds the game as it was.
/// On Unix a successful restart never returns; elsewhere this process
/// then exits 0.
pub fn relaunch_after_shell(
    exit: u5_bevy::ShellExit,
    host: Option<&Arc<dyn MenuHost>>,
    setup_dir: Option<&Path>,
) -> io::Result<()> {
    let Some(asked) = exit.relaunch else {
        return Ok(());
    };
    match crate::relaunch::relaunch(asked.boot_key, setup_dir) {
        Ok(()) => Ok(()),
        Err(failed) => Err(relaunch_failed(asked, &failed, host)),
    }
}

/// What a failed restart reports: its reason, and for a load, the load
/// rolled back through `host` (or why it could not be).
fn relaunch_failed(
    asked: u5_bevy::Relaunch,
    failed: &crate::relaunch::SpawnFailed,
    host: Option<&Arc<dyn MenuHost>>,
) -> io::Error {
    let mut message = failed.to_string();
    if asked.boot_key == Some(u5_bevy::menu::slots::LOAD_BOOT_KEY)
        && let Some(host) = host
    {
        match host.roll_back_restore() {
            Ok(()) => message.push_str("; the load was undone and thy game is unchanged"),
            Err(undo) => message.push_str(&format!("; the load could not be undone: {undo}")),
        }
    }
    io::Error::other(message)
}

#[cfg(test)]
#[path = "menu_host_tests.rs"]
mod tests;
