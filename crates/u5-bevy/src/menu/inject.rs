//! The slot save's handshake: the menu saves by typing the game's own `Q`
//! and `Y` (Phase B design 3.3.4 "Synthetic input", 3.3.5 "Save to slot"
//! steps 1-5; plan M4-10).
//!
//! A [`SlotSaveRequest`] (the Save page sends it, M4-11) runs these steps:
//!
//! 1. [`can_save`]: the game waits at the idle exploration prompt, no other
//!    feature holds the synthetic-key token, and `Q` would open the game's
//!    `Save game?` prompt there (`PlayState::save_command_available`). If
//!    not, nothing happens: [`SlotSaveFinished::Refused`].
//! 2. The menu closes, the token is taken as
//!    [`SyntheticOwner::MenuSlotSave`], `Q` is queued and a "Saving..."
//!    toast shows. The feed types it once the menu is closed, through the
//!    path a typed key takes.
//! 3. `Y` is queued only once `Q` was read and the frame shows the save
//!    prompt, so the two are never queued together: with Buffer Off the
//!    read of `Q` empties the queue (`input.md §6`), and a `Y` queued with
//!    it would be lost. If `Q` opened anything else, the save stops with
//!    "Thou canst not save at this moment."; whatever `Q` printed stands,
//!    as if the player had typed it ([`SlotSaveFinished::Unavailable`]).
//! 4. Once `Y` was read, the prompt has closed, the speaker is idle and the
//!    game is back at its prompt, the working `SAVED.GAM` is checked: it
//!    reads back as a save image, holds an active Avatar and was written
//!    since step 2. The thumbnail is taken from the game's own frame
//!    ([`super::thumb`]). The result is [`SlotSaveFinished::Saved`].
//! 5. From step 2 until the token is released the router swallows every
//!    physical key (enh/input.rs), so no key can reach the game or start
//!    another save. With [`SlotSaveRequest::hold_for_store`] the token is
//!    kept after step 4 until the slot copy calls
//!    [`SlotSaveHandshake::finish_store`]; otherwise it is released there.
//!
//! The handshake is in the app only with named slots and a slot store
//! (3.4.2): without them none of this module runs.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use u5_runtime::enhancements::IdlePromptKind;
use u5_runtime::z_stats::YesNoPromptKind;
use u5_runtime::{
    SAVED_GAM_FILENAME, SaveImageSummary, read_save_image_file, save_image_has_active_avatar,
    save_image_summary,
};

use super::text as t;
use super::thumb::frame_thumbnail;
use crate::VisualState;
use crate::enh::EnhSet;
use crate::enh::idle::IdleProbe;
use crate::enh::overlay::{OverlayId, OverlayRequest, OverlayState, apply_overlay_requests};
use crate::enh::synthetic::{
    SyntheticInputOwner, SyntheticKey, SyntheticOwner, install_synthetic_input,
};
use crate::enh::toast::Toasts;

/// The key that opens the save prompt and the key that answers it.
pub const SAVE_KEY: SyntheticKey = SyntheticKey::plain(KeyCode::KeyQ);
pub const YES_KEY: SyntheticKey = SyntheticKey::plain(KeyCode::KeyY);

/// How long a step may wait for the game before the handshake gives up. A
/// step only waits for the game to read a key or to finish printing, so
/// this is reached only if something holds the game (which no key can end
/// while physical keys are swallowed).
pub const STEP_LIMIT: Duration = Duration::from_secs(10);

/// Asks for a slot save (3.3.5).
#[derive(Event, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotSaveRequest {
    /// Keep the token after the save so the slot copy runs with physical
    /// keys still swallowed; the copy calls
    /// [`SlotSaveHandshake::finish_store`].
    pub hold_for_store: bool,
}

/// What a working save written by the menu's `Q` and `Y` holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotSaveCapture {
    /// The working `SAVED.GAM`, as read back.
    pub image: Vec<u8>,
    /// What the image says (the Avatar, the clock, the place).
    pub summary: SaveImageSummary,
    /// `PlayState.turn` at the save.
    pub turn: u64,
    /// The game's frame at the save, box filtered to 80x50, as PNG.
    pub thumbnail_png: Option<Vec<u8>>,
}

/// How a slot save's handshake ended.
#[derive(Event, Clone, Debug, PartialEq, Eq)]
pub enum SlotSaveFinished {
    /// [`can_save`] failed: nothing was typed.
    Refused,
    /// `Q` did not open the save prompt; nothing more was typed.
    Unavailable,
    /// The working save was written and checked.
    Saved(SlotSaveCapture),
    /// `Y` was typed but the working save is not right; the reason, for
    /// the log.
    Failed(String),
}

/// Where the handshake is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlotSavePhase {
    #[default]
    Idle,
    /// `Q` is queued or typed; waiting for the frame that answers it.
    AwaitPrompt,
    /// `Y` is queued or typed; waiting for the save to finish.
    AwaitSaved,
    /// The save is checked; the slot copy holds the token.
    Storing,
}

/// The handshake's state.
#[derive(Resource, Debug, Default)]
pub struct SlotSaveHandshake {
    phase: SlotSavePhase,
    hold_for_store: bool,
    /// When `Q` was queued, to the whole second: the save must be no older.
    started: Option<SystemTime>,
    /// Real time spent in the current step.
    waited: Duration,
}

impl SlotSaveHandshake {
    pub fn phase(&self) -> SlotSavePhase {
        self.phase
    }

    /// A save is under way, the slot copy included.
    pub fn busy(&self) -> bool {
        self.phase != SlotSavePhase::Idle
    }

    /// Ends the slot copy that [`SlotSaveRequest::hold_for_store`] asked
    /// for, and releases the token.
    pub fn finish_store(&mut self, owner: &mut SyntheticInputOwner) {
        if self.phase == SlotSavePhase::Storing {
            self.end(owner);
        }
    }

    fn end(&mut self, owner: &mut SyntheticInputOwner) {
        owner.release(SyntheticOwner::MenuSlotSave);
        *self = Self::default();
    }

    fn step(&mut self, phase: SlotSavePhase) {
        self.phase = phase;
        self.waited = Duration::ZERO;
    }
}

/// Whether `me` may type a command now (3.3.4): the game waits at the idle
/// exploration prompt and no other owner holds the token.
pub fn can_inject_command(owner: &SyntheticInputOwner, me: SyntheticOwner, idle: bool) -> bool {
    idle && owner.free_or(me)
}

/// Whether the menu may save now (3.3.4): it may type, and `Q` would open
/// the save prompt.
pub(crate) fn can_save(visual: &VisualState, owner: &SyntheticInputOwner, idle: bool) -> bool {
    can_inject_command(owner, SyntheticOwner::MenuSlotSave, idle)
        && visual.state.save_command_available()
}

const EXPLORATION: &[IdlePromptKind] = &[IdlePromptKind::Exploration];

/// Adds the handshake, once. Called by the menu plugin when named slots and
/// a slot store are on.
pub(crate) fn install_slot_save(app: &mut App) {
    if app.world().contains_resource::<SlotSaveHandshake>() {
        return;
    }
    install_synthetic_input(app);
    app.init_resource::<SlotSaveHandshake>()
        .add_event::<SlotSaveRequest>()
        .add_event::<SlotSaveFinished>()
        .add_systems(
            Update,
            begin_slot_save
                .run_if(on_event::<SlotSaveRequest>)
                .in_set(EnhSet::Apply)
                .after(super::menu_input)
                .before(apply_overlay_requests),
        )
        .add_systems(
            Update,
            advance_slot_save
                .run_if(handshake_busy)
                .in_set(EnhSet::Observe),
        );
}

fn handshake_busy(handshake: Res<SlotSaveHandshake>) -> bool {
    handshake.busy()
}

/// Whether the game waits at the idle exploration prompt for a save. With
/// the menu open the weaker staying-open test applies, as it does for the
/// menu itself: an ambient sound the idle steps started does not refuse
/// the save, and `Q` waits for it in the queue as a typed key would.
fn idle_for_save(probe: &IdleProbe, overlay: Option<&OverlayState>) -> bool {
    let view = probe.view();
    let menu_open = overlay.is_some_and(|overlay| overlay.open() == Some(OverlayId::Menu));
    view.at(EXPLORATION) || (menu_open && view.still_at(EXPLORATION))
}

/// Steps 1 and 2.
#[allow(clippy::too_many_arguments)]
fn begin_slot_save(
    mut requests: EventReader<SlotSaveRequest>,
    mut handshake: ResMut<SlotSaveHandshake>,
    mut owner: ResMut<SyntheticInputOwner>,
    probe: IdleProbe,
    visual: Option<Res<VisualState>>,
    overlay: Option<Res<OverlayState>>,
    mut overlay_requests: EventWriter<OverlayRequest>,
    toasts: Option<ResMut<Toasts>>,
    mut finished: EventWriter<SlotSaveFinished>,
) {
    let Some(request) = requests.read().last().copied() else {
        return;
    };
    if handshake.busy() {
        return;
    }
    let idle = idle_for_save(&probe, overlay.as_deref());
    if !visual.is_some_and(|visual| can_save(&visual, &owner, idle)) {
        finished.write(SlotSaveFinished::Refused);
        return;
    }
    if overlay
        .as_deref()
        .is_some_and(|overlay| overlay.open() == Some(OverlayId::Menu))
    {
        overlay_requests.write(OverlayRequest::Close(OverlayId::Menu));
    }
    if owner.take(SyntheticOwner::MenuSlotSave).is_err()
        || owner
            .queue(SyntheticOwner::MenuSlotSave, [SAVE_KEY])
            .is_err()
    {
        finished.write(SlotSaveFinished::Refused);
        return;
    }
    if let Some(mut toasts) = toasts {
        toasts.note(t::SAVING);
    }
    *handshake = SlotSaveHandshake {
        phase: SlotSavePhase::AwaitPrompt,
        hold_for_store: request.hold_for_store,
        started: Some(whole_second(SystemTime::now())),
        waited: Duration::ZERO,
    };
}

/// `time` to the whole second below: a file system that keeps coarse
/// modification times still dates the save no earlier.
fn whole_second(time: SystemTime) -> SystemTime {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    UNIX_EPOCH + Duration::from_secs(since.as_secs())
}

fn save_prompt_open(visual: &VisualState) -> bool {
    visual
        .state
        .active_yes_no_prompt
        .map(|session| session.kind)
        == Some(YesNoPromptKind::SaveGame)
}

/// Steps 3 and 4, after the input driver has read this frame's keys.
#[allow(clippy::too_many_arguments)]
fn advance_slot_save(
    time: Res<Time>,
    mut handshake: ResMut<SlotSaveHandshake>,
    mut owner: ResMut<SyntheticInputOwner>,
    probe: IdleProbe,
    visual: Option<Res<VisualState>>,
    images: Option<Res<Assets<Image>>>,
    toasts: Option<ResMut<Toasts>>,
    mut finished: EventWriter<SlotSaveFinished>,
) {
    let Some(visual) = visual else {
        return;
    };
    let mut toasts = toasts;
    let phase = handshake.phase;
    if phase == SlotSavePhase::Storing {
        return;
    }
    handshake.waited = handshake.waited.saturating_add(time.delta());
    // The toast shows until the save ends; another feature's toast is not
    // replaced.
    if let Some(toasts) = toasts.as_deref_mut()
        && toasts.current().is_none()
    {
        toasts.note(t::SAVING);
    }
    let timed_out = handshake.waited >= STEP_LIMIT;
    // A key of ours not yet fed or not yet read: the game has not answered.
    let key_pending = owner.queued() > 0 || !visual.typed_keys.is_empty();
    match phase {
        SlotSavePhase::AwaitPrompt => {
            if key_pending && !timed_out {
                return;
            }
            if !key_pending && save_prompt_open(&visual) {
                let _ = owner.queue(SyntheticOwner::MenuSlotSave, [YES_KEY]);
                handshake.step(SlotSavePhase::AwaitSaved);
                return;
            }
            if let Some(toasts) = toasts.as_deref_mut() {
                toasts.short(t::SAVE_UNAVAILABLE);
            }
            handshake.end(&mut owner);
            finished.write(SlotSaveFinished::Unavailable);
        }
        SlotSavePhase::AwaitSaved => {
            let view = probe.view();
            let speaker_busy = view.speaker_playing
                || visual.sound_effect_cursor != visual.state.sound_effect_serial;
            let settled = !key_pending
                && !save_prompt_open(&visual)
                && !speaker_busy
                && visual.state.at_idle_command_prompt();
            if !settled && !timed_out {
                return;
            }
            let started = handshake.started.unwrap_or(UNIX_EPOCH);
            let checked = check_working_save(&visual.game_dir, started);
            match checked {
                Ok((image, summary)) => {
                    let thumbnail_png = images
                        .as_deref()
                        .and_then(|images| images.get(&visual.image_handle))
                        .and_then(frame_thumbnail);
                    finished.write(SlotSaveFinished::Saved(SlotSaveCapture {
                        image,
                        summary,
                        turn: visual.state.turn,
                        thumbnail_png,
                    }));
                    if handshake.hold_for_store {
                        handshake.step(SlotSavePhase::Storing);
                    } else {
                        handshake.end(&mut owner);
                    }
                }
                Err(reason) => {
                    warn!("{}: {reason}", t::SAVE_FAILED);
                    if let Some(toasts) = toasts.as_deref_mut() {
                        toasts.note(t::SAVE_FAILED);
                    }
                    handshake.end(&mut owner);
                    finished.write(SlotSaveFinished::Failed(reason));
                }
            }
        }
        SlotSavePhase::Idle | SlotSavePhase::Storing => {}
    }
}

/// Step 4's check of the working save in `game_dir`: it reads back, holds
/// an active Avatar and is no older than `started`.
pub fn check_working_save(
    game_dir: &Path,
    started: SystemTime,
) -> Result<(Vec<u8>, SaveImageSummary), String> {
    let path = game_dir.join(SAVED_GAM_FILENAME);
    let image =
        read_save_image_file(&path, SAVED_GAM_FILENAME).map_err(|error| error.to_string())?;
    if !save_image_has_active_avatar(&image) {
        return Err(NO_AVATAR.to_string());
    }
    let written = fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .map_err(|error| error.to_string())?;
    if written < started {
        return Err(NOT_WRITTEN.to_string());
    }
    let summary = save_image_summary(&image).map_err(|error| error.to_string())?;
    Ok((image, summary))
}

/// Logged when the working save has no Avatar after the menu's save.
const NO_AVATAR: &str = "the working save holds no active game";
/// Logged when the working save is older than the menu's `Q`.
const NOT_WRITTEN: &str = "the working save was not written by this save";

#[cfg(test)]
mod tests;
