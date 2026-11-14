//! The slot plugin: the Save and Load pages' work (Phase B design 3.2.2,
//! 3.2.3, 3.3.5; build plan M4-11).
//!
//! [`SlotPlugin`] is added by the menu plugin only when `menu.named_slots`
//! is on and the host keeps a slot store (3.4.2: "`named_slots` and a
//! menu"). It carries out the slot pages' commands through the host's
//! [`MenuHost`]:
//!
//! - **Listing.** Opening Save or Load, and every change, lists the slots
//!   again ([`SlotListing`]); thumbnails are decoded once there.
//! - **Save.** A slot save is a real `Q` save: the plugin asks the M4-10
//!   handshake for one ([`SlotSaveRequest`], keeping the token for the
//!   copy), and once the game wrote and the handshake checked the working
//!   save, hands the copy to the host's worker ([`MenuHost::store_slot`]).
//!   Its answer is polled each frame; the toast names the slot, and the
//!   token is released. The slot's place is the published words of
//!   D-M4.8 ([`slot_place_text`]), its picture the faithful frame.
//! - **Load in play.** "Loading..." shows for one frame, then the host
//!   restores the slot ([`MenuHost::restore_slot`]), the shell checks the
//!   working save (it reads back, with an active Avatar), and the same
//!   system leaves a [`ShellExit`] asking for a relaunch with the boot `J`
//!   and ends the app. The menu stays open until then, so no key reaches
//!   the game, and the runtime writes working files only while running a
//!   command, so nothing touches the restored files before the app exits.
//!   Any failure puts the working set back and says why; the session goes
//!   on.
//! - **Rename and Delete** go to the host at once; they touch one small
//!   file or folder.
//!
//! Every toast that carries a name or a reason is a template from
//! [`super::text`] filled with the slot's name or the host's words (D-M4.7).

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};

use bevy::prelude::*;
use u5_automap::gem::slot_place_text;
use u5_runtime::{SAVED_GAM_FILENAME, read_save_image_file, save_image_has_active_avatar};

use super::host::{MenuHost, SlotFailure, SlotInfo, SlotTarget, StoreSlot};
use super::inject::{
    SlotSaveFinished, SlotSaveHandshake, SlotSaveRequest, advance_slot_save, begin_slot_save,
    install_slot_save,
};
use super::pages::{MenuCommand, SaveChoice, SlotEntry, SlotList};
use super::text as t;
use super::unsaved::SaveTracker;
use super::{MenuCommandEvent, MenuState, menu_input};
use crate::VisualState;
use crate::enh::synthetic::SyntheticInputOwner;
use crate::enh::toast::Toasts;
use crate::enh::{EnhSet, Relaunch, ShellExit, ShellExitSlot};

/// The key the restarted game finds waiting at boot after a load: the
/// original early Journey Onward (`systems/intro.md §3` step 6).
pub const LOAD_BOOT_KEY: u8 = b'J';

/// Said when a load cannot hand the restart to the host (a shell with no
/// exit slot); never in the shipped shells.
const NO_RELAUNCH: &str = "this window cannot restart the game";
/// Said when the host's worker went away without an answer.
const STORE_LOST: &str = "the copy stopped";

/// The slot pages' feature plugin.
pub struct SlotPlugin {
    pub host: Arc<dyn MenuHost>,
}

impl Plugin for SlotPlugin {
    fn build(&self, app: &mut App) {
        install_slot_save(app);
        app.insert_resource(SlotHost(self.host.clone()))
            .init_resource::<SlotListing>()
            .init_resource::<SlotJob>()
            .add_systems(
                Update,
                (
                    slot_menu_commands
                        .run_if(on_event::<MenuCommandEvent>)
                        .after(menu_input)
                        .before(begin_slot_save),
                    run_pending_load
                        .run_if(load_pending)
                        .after(slot_menu_commands),
                )
                    .in_set(EnhSet::Apply),
            )
            .add_systems(
                Update,
                (
                    slot_save_finished
                        .run_if(on_event::<SlotSaveFinished>)
                        .after(advance_slot_save),
                    poll_slot_store
                        .run_if(store_pending)
                        .after(slot_save_finished),
                )
                    .in_set(EnhSet::Observe),
            );
    }
}

/// The host's slot services.
#[derive(Resource, Clone)]
pub struct SlotHost(pub Arc<dyn MenuHost>);

/// The slots as last listed, which the pages show.
#[derive(Resource, Debug, Default)]
pub struct SlotListing {
    pub slots: SlotList,
}

impl SlotListing {
    fn get(&self, at: u16) -> Option<&SlotEntry> {
        self.slots.get(usize::from(at))
    }
}

/// What the plugin is in the middle of.
#[derive(Resource, Default)]
pub struct SlotJob {
    /// A save asked of the handshake, waiting for the game's save.
    save: Option<SaveAsked>,
    /// The host's copy under way.
    store: Option<PendingStore>,
    /// A load waiting for "Loading..." to show.
    load: Option<PendingLoad>,
}

impl SlotJob {
    /// A save or its copy is under way.
    pub fn saving(&self) -> bool {
        self.save.is_some() || self.store.is_some()
    }
}

struct SaveAsked {
    target: SlotTarget,
    /// The slot's name for the toasts; `None` for a new slot left unnamed.
    name: Option<String>,
}

struct PendingStore {
    name: Option<String>,
    reply: Mutex<Receiver<Result<SlotInfo, SlotFailure>>>,
}

struct PendingLoad {
    entry: SlotEntry,
    /// Frames the "Loading..." dialog has had to show.
    shown: u8,
}

fn load_pending(job: Res<SlotJob>) -> bool {
    job.load.is_some()
}

fn store_pending(job: Res<SlotJob>) -> bool {
    job.store.is_some()
}

/// The host's slots, thumbnails decoded, or the reason they could not be
/// read.
pub fn list_entries(host: &dyn MenuHost) -> Result<Vec<SlotEntry>, SlotFailure> {
    Ok(host.list_slots()?.into_iter().map(slot_entry).collect())
}

/// One listed slot with its thumbnail decoded to 80x50 RGBA; a thumbnail
/// that does not decode to that size is dropped.
pub fn slot_entry(mut info: SlotInfo) -> SlotEntry {
    let thumb = info
        .thumbnail_png
        .take()
        .and_then(|png| decode_thumbnail(&png));
    SlotEntry { info, thumb }
}

fn decode_thumbnail(png: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let rgba = image.to_rgba8();
    (rgba.dimensions() == (super::thumb::THUMB_WIDTH, super::thumb::THUMB_HEIGHT))
        .then(|| rgba.into_raw())
}

/// The toast for a failure: the busy line, or `template` filled with the
/// name and the reason.
fn failure_toast(toasts: Option<&mut Toasts>, template: &str, name: &str, failure: &SlotFailure) {
    let Some(toasts) = toasts else {
        return;
    };
    match failure {
        SlotFailure::Busy => toasts.note(t::SLOTS_BUSY),
        SlotFailure::Reason(reason) => toasts.note_text(t::fill(template, &[name, reason])),
    }
}

/// Lists the slots again; a failure empties the list and says why.
fn refresh(host: &dyn MenuHost, listing: &mut SlotListing, toasts: Option<&mut Toasts>) {
    match list_entries(host) {
        Ok(entries) => listing.slots = Arc::new(entries),
        Err(failure) => {
            listing.slots = SlotList::default();
            if let Some(toasts) = toasts {
                match failure {
                    SlotFailure::Busy => toasts.note(t::SLOTS_BUSY),
                    SlotFailure::Reason(reason) => {
                        toasts.note_text(t::fill(t::LIST_FAILED, &[&reason]));
                    }
                }
            }
        }
    }
}

/// Carries out the slot pages' commands.
#[allow(clippy::too_many_arguments)]
fn slot_menu_commands(
    mut commands: EventReader<MenuCommandEvent>,
    host: Res<SlotHost>,
    mut listing: ResMut<SlotListing>,
    mut job: ResMut<SlotJob>,
    handshake: Res<SlotSaveHandshake>,
    mut state: ResMut<MenuState>,
    mut requests: EventWriter<SlotSaveRequest>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    for MenuCommandEvent(command) in commands.read() {
        let field = state
            .model()
            .map(|model| model.field().trim().to_string())
            .unwrap_or_default();
        match *command {
            MenuCommand::OpenSave | MenuCommand::OpenLoad => {
                refresh(&*host.0, &mut listing, toasts.as_deref_mut());
            }
            MenuCommand::SaveSlot(choice) => {
                if handshake.busy() || job.saving() {
                    continue;
                }
                let asked = match choice {
                    SaveChoice::New => SaveAsked {
                        name: (!field.is_empty()).then(|| field.clone()),
                        target: SlotTarget::New(field.clone()),
                    },
                    SaveChoice::Quick => SaveAsked {
                        target: SlotTarget::Quick,
                        name: Some(t::QUICK_SLOT.to_string()),
                    },
                    SaveChoice::Existing(at) => {
                        let Some(slot) = listing.get(at) else {
                            continue;
                        };
                        SaveAsked {
                            target: SlotTarget::Existing(slot.info.id.clone()),
                            name: Some(slot.info.name.clone()),
                        }
                    }
                };
                job.save = Some(asked);
                requests.write(SlotSaveRequest {
                    hold_for_store: true,
                });
            }
            MenuCommand::LoadSlot(at) => {
                if let Some(entry) = listing.get(at).filter(|slot| slot.info.damaged.is_none()) {
                    job.load = Some(PendingLoad {
                        entry: entry.clone(),
                        shown: 0,
                    });
                }
            }
            MenuCommand::RenameSlot(at) => {
                let Some(slot) = listing.get(at).cloned() else {
                    continue;
                };
                match host.0.rename_slot(&slot.info.id, &field) {
                    Ok(()) => {
                        if let Some(toasts) = toasts.as_deref_mut() {
                            toasts.short_text(t::fill(t::RENAMED, &[&field]));
                        }
                    }
                    Err(failure) => failure_toast(
                        toasts.as_deref_mut(),
                        t::RENAME_FAILED,
                        &slot.info.name,
                        &failure,
                    ),
                }
                refresh(&*host.0, &mut listing, None);
            }
            MenuCommand::DeleteSlot(at) => {
                let Some(slot) = listing.get(at).cloned() else {
                    continue;
                };
                match host.0.delete_slot(&slot.info.id) {
                    Ok(()) => {
                        if let Some(toasts) = toasts.as_deref_mut() {
                            toasts.short_text(t::fill(t::DELETED, &[&slot.info.name]));
                        }
                    }
                    Err(failure) => failure_toast(
                        toasts.as_deref_mut(),
                        t::DELETE_FAILED,
                        &slot.info.name,
                        &failure,
                    ),
                }
                refresh(&*host.0, &mut listing, None);
                // The cursor stays in the shorter list.
                if let Some(model) = state.model.as_mut() {
                    model.clamp_cursor(listing.slots.len());
                }
            }
            _ => {}
        }
    }
}

/// The working save in `dir` reads back as a save image with an active
/// Avatar (`save-load.md §9`): the shell's own check after a restore.
pub fn check_restored(dir: &std::path::Path) -> Result<(), String> {
    let image = read_save_image_file(&dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)
        .map_err(|error| error.to_string())?;
    if save_image_has_active_avatar(&image) {
        Ok(())
    } else {
        Err("the save holds no game".to_string())
    }
}

/// A load in play (3.3.5 "Restore" steps 2-5).
#[allow(clippy::too_many_arguments)]
fn run_pending_load(
    mut job: ResMut<SlotJob>,
    host: Res<SlotHost>,
    visual: Option<Res<VisualState>>,
    exit_slot: Option<Res<ShellExitSlot>>,
    mut exit: EventWriter<AppExit>,
    mut state: ResMut<MenuState>,
    mut tracker: Option<ResMut<SaveTracker>>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    // One frame for "Loading..." to show: the game thread blocks for the
    // restore (3.3.9).
    if let Some(load) = job.load.as_mut()
        && load.shown == 0
    {
        load.shown = 1;
        return;
    }
    let Some(load) = job.load.take() else {
        return;
    };
    let name = load.entry.info.name.clone();
    let outcome = host.0.restore_slot(&load.entry.info.id).and_then(|()| {
        let checked = match visual.as_deref() {
            Some(visual) => check_restored(&visual.game_dir),
            None => Ok(()),
        };
        let handed = checked.and_then(|()| {
            let slot = exit_slot
                .as_deref()
                .ok_or_else(|| NO_RELAUNCH.to_string())?;
            slot.set(ShellExit {
                relaunch: Some(Relaunch {
                    boot_key: Some(LOAD_BOOT_KEY),
                }),
            });
            Ok(())
        });
        handed.map_err(|reason| {
            // The restore went through but the game cannot go on from
            // it: put the working set back.
            match host.0.roll_back_restore() {
                Ok(()) => SlotFailure::Reason(reason),
                Err(undo) => SlotFailure::Reason(format!("{reason}; {undo}")),
            }
        })
    });
    match outcome {
        Ok(()) => {
            if let (Some(tracker), Some(visual)) = (tracker.as_deref_mut(), visual.as_deref()) {
                tracker.saved_at(visual.state.turn);
            }
            exit.write(AppExit::Success);
        }
        Err(failure) => {
            failure_toast(toasts.as_deref_mut(), t::LOAD_FAILED, &name, &failure);
            if let Some(model) = state.model.as_mut() {
                model.back();
            }
        }
    }
}

/// The handshake ended: hand a checked save to the host's worker, or say
/// why there is nothing to copy.
fn slot_save_finished(
    mut finished: EventReader<SlotSaveFinished>,
    host: Res<SlotHost>,
    mut job: ResMut<SlotJob>,
    mut tracker: Option<ResMut<SaveTracker>>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    for event in finished.read() {
        // Not this plugin's save: whoever asked for it copies it.
        let Some(asked) = job.save.take() else {
            continue;
        };
        match event {
            SlotSaveFinished::Saved(capture) => {
                // Moment 1: the game's own save wrote the working set.
                if let Some(tracker) = tracker.as_deref_mut() {
                    tracker.saved_at(capture.turn);
                }
                let summary = &capture.summary;
                let clock = summary.clock;
                let request = StoreSlot {
                    target: asked.target,
                    avatar: summary.avatar.clone().unwrap_or_default(),
                    place: slot_place_text(summary.target, summary.floor),
                    clock: [
                        clock.year,
                        u16::from(clock.month),
                        u16::from(clock.day),
                        u16::from(clock.hour),
                        u16::from(clock.minute),
                    ],
                    turn: capture.turn,
                    cheats_used: Vec::new(),
                    thumbnail_png: capture.thumbnail_png.clone(),
                };
                job.store = Some(PendingStore {
                    name: asked.name,
                    reply: Mutex::new(host.0.store_slot(request)),
                });
            }
            SlotSaveFinished::Refused => {
                if let Some(toasts) = toasts.as_deref_mut() {
                    toasts.short(t::SAVE_UNAVAILABLE);
                }
            }
            // The handshake said so itself, and the token is released.
            SlotSaveFinished::Unavailable | SlotSaveFinished::Failed(_) => {}
        }
    }
}

/// The host's copy answered: say so, release the token and list again.
#[allow(clippy::too_many_arguments)]
fn poll_slot_store(
    mut job: ResMut<SlotJob>,
    host: Res<SlotHost>,
    mut listing: ResMut<SlotListing>,
    mut handshake: ResMut<SlotSaveHandshake>,
    mut owner: ResMut<SyntheticInputOwner>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    let Some(store) = job.store.as_ref() else {
        return;
    };
    let answer = store
        .reply
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .try_recv();
    let answer = match answer {
        Ok(answer) => answer,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => Err(SlotFailure::Reason(STORE_LOST.to_string())),
    };
    let name = job.store.take().and_then(|store| store.name);
    if let Some(toasts) = toasts.as_deref_mut() {
        match (&answer, name.as_deref()) {
            (Ok(info), _) => toasts.short_text(t::fill(t::SAVED_TO, &[&info.name])),
            (Err(SlotFailure::Busy), _) => toasts.note(t::SLOTS_BUSY),
            (Err(SlotFailure::Reason(reason)), Some(name)) => {
                toasts.note_text(t::fill(t::SLOT_WRITE_FAILED, &[name, reason]));
            }
            (Err(SlotFailure::Reason(reason)), None) => {
                toasts.note_text(t::fill(t::SLOT_WRITE_FAILED_NEW, &[reason]));
            }
        }
    }
    if let Err(failure) = &answer {
        warn!("menu: the slot copy failed: {failure}");
    }
    handshake.finish_store(&mut owner);
    refresh(&*host.0, &mut listing, None);
}

#[cfg(test)]
#[path = "slots_tests.rs"]
mod tests;
