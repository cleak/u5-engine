//! The modified marker (Phase B design 7.3.7, 7.2.10; build plan M8-02,
//! D-M8.1).
//!
//! A save made after a class (c) cheat changed the game is marked
//! modified in the foundation's META sidecar, `SaveMeta { cheats_used }`,
//! through [`SidecarStore`]; `SAVED.GAM` is never touched. The trigger is
//! the foundation's save watcher: on each [`SaveStamped`] of a tainted
//! session `cheat_marker` writes the flag stamped with that save. A new
//! character's stamp starts the session clean and removes the old marker.
//!
//! When gameplay starts (and after a slot load) `cheat_marker_attach`
//! reads the marker against the save on disk:
//!
//! - `Fresh` with the flag: the session starts tainted, "MODIFIED";
//! - `SameLineage`, `Foreign` or `Corrupt` (renamed `.bad` by the store):
//!   "unknown", untainted;
//! - `Missing`, or `Fresh` without the flag: clean, "unmodified".
//!
//! Both run only while a [`CheatSession`] exists, so with cheats off the
//! marker is neither read nor written (7.4.2), and the store is opened
//! only from these two systems.

use std::path::PathBuf;

use bevy::prelude::*;
use u5_runtime::enhancements::{
    Freshness, SAVE_META_PAYLOAD_VERSION, SaveMeta, SaveStamp, SidecarStore, TAG_SAVE_META,
};

use super::CheatSession;
use crate::VisualState;
use crate::enh::EnhLaunch;
use crate::enh::sidecar_io::{SaveStamped, SaveWatch, save_watcher};
use crate::enh::toast::{ShellBadges, Toasts};

/// Shown once when the marker could not be written (7.2.10). The save
/// itself is already on disk and is never touched.
pub const MARKER_WRITE_ERROR: &str = "Cheats: this save could not be marked modified.";
/// Logged with it.
const MARKER_WRITE_LOG: &str = "cheats: the modified marker could not be written";
/// Logged when a new character's old marker could not be removed.
const MARKER_RESET_LOG: &str = "cheats: the old modified marker could not be removed";

/// What the loaded save's marker says (7.3.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerState {
    /// No marker, or a fresh one without the flag.
    #[default]
    Clean,
    /// A fresh marker with the flag, or a cheat used in this session.
    Modified,
    /// A marker for another save: it cannot say.
    Unknown,
}

impl MarkerState {
    /// The marker state a read gives.
    pub fn of_read(freshness: Freshness, meta: Option<SaveMeta>) -> Self {
        match freshness {
            Freshness::Fresh if meta.is_some_and(|meta| meta.cheats_used) => Self::Modified,
            Freshness::Fresh | Freshness::Missing => Self::Clean,
            Freshness::SameLineage | Freshness::Foreign | Freshness::Corrupt => Self::Unknown,
        }
    }
}

/// Every marker read or write this process made, for the zero-footprint
/// tests (the counting stub of 7.4.2).
#[cfg(test)]
pub(crate) static MARKER_IO: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn count_io() {
    #[cfg(test)]
    MARKER_IO.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

/// The marker systems. The write needs the save watcher, which owns
/// [`SaveStamped`]; it is present when the host gave a save folder and
/// `cheats.enabled` (a sidecar key) is on.
pub(super) fn add_marker(app: &mut App) {
    app.add_systems(
        Update,
        cheat_marker_attach
            .in_set(crate::enh::EnhSet::Observe)
            .run_if(resource_exists::<CheatSession>)
            .run_if(resource_exists::<VisualState>),
    );
    if app.world().contains_resource::<SaveWatch>() {
        app.add_systems(
            Update,
            cheat_marker
                .in_set(crate::enh::EnhSet::Observe)
                .after(save_watcher)
                .after(super::apply::cheat_apply)
                .run_if(resource_exists::<CheatSession>),
        );
    }
}

/// The save folder the host handed in.
fn save_dir(launch: &EnhLaunch, watch: Option<&SaveWatch>) -> Option<PathBuf> {
    watch
        .map(|watch| watch.save_dir().to_path_buf())
        .or_else(|| launch.0.save_dir.clone())
}

/// Reads the marker when gameplay starts and after a slot load.
fn cheat_marker_attach(
    mut session: ResMut<CheatSession>,
    visual: Res<VisualState>,
    launch: Res<EnhLaunch>,
    watch: Option<Res<SaveWatch>>,
    badges: Option<ResMut<ShellBadges>>,
) {
    if !visual.is_added() && !session.attach_requested {
        return;
    }
    session.attach_requested = false;
    let Some(dir) = save_dir(&launch, watch.as_deref()) else {
        return;
    };
    let (Some(store), Some(now)) = (SidecarStore::open(&dir), SaveStamp::read(&dir)) else {
        return;
    };
    count_io();
    let (freshness, payload) = store.read(TAG_SAVE_META, &now);
    let meta = payload.and_then(|payload| SaveMeta::decode(&payload.bytes));
    session.marker = MarkerState::of_read(freshness, meta);
    if session.marker == MarkerState::Modified {
        session.tainted = true;
    }
    if let Some(mut badges) = badges {
        badges.modified = session.tainted;
    }
}

/// Writes the marker on each new save of a tainted session; a new
/// character starts clean (7.3.7).
fn cheat_marker(
    mut session: ResMut<CheatSession>,
    mut stamped: EventReader<SaveStamped>,
    launch: Res<EnhLaunch>,
    watch: Res<SaveWatch>,
    mut toasts: Option<ResMut<Toasts>>,
    mut badges: Option<ResMut<ShellBadges>>,
) {
    for event in stamped.read() {
        let store = || {
            save_dir(&launch, Some(&watch))
                .as_deref()
                .and_then(SidecarStore::open)
        };
        if event.new_game {
            session.tainted = false;
            session.marker = MarkerState::Clean;
            if let Some(badges) = badges.as_mut() {
                badges.modified = false;
            }
            if let Some(store) = store() {
                count_io();
                if let Err(error) = store.reset(&[TAG_SAVE_META]) {
                    warn!("{MARKER_RESET_LOG}: {error}");
                }
            }
            continue;
        }
        if !session.tainted {
            continue;
        }
        let Some(store) = store() else {
            continue;
        };
        count_io();
        let payload = SaveMeta { cheats_used: true }.encode();
        match store.write(
            TAG_SAVE_META,
            &event.stamp,
            SAVE_META_PAYLOAD_VERSION,
            &payload,
        ) {
            Ok(_) => {
                session.marker = MarkerState::Modified;
                session.marker_retry = false;
            }
            Err(error) => {
                warn!("{MARKER_WRITE_LOG}: {error}");
                session.marker_retry = true;
                if let Some(toasts) = toasts.as_mut() {
                    toasts.note(MARKER_WRITE_ERROR);
                }
            }
        }
    }
}
