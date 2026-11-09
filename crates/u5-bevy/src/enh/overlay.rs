//! The shared modal overlay model (Phase B design 1.2.6, 1.3.7).
//!
//! One registry, [`OverlayState`], for the menu, the journal page, the
//! full-screen map and the cheats page. It is in the app only while one of
//! those features is on (`add_overlay_model`); with all of them off no
//! overlay resource, event or system exists.
//!
//! - A feature asks with an [`OverlayRequest`]. An overlay opens only at
//!   its idle prompts ([`OverlayId::kinds`], design 1.3.7); otherwise an
//!   [`OverlayRefused`] is sent and a 2 s toast explains, and the key that
//!   asked has already fallen to the game, which dropped it (1.3.6).
//! - At most one is open. Opening another closes the first.
//! - **Live, always (U1).** No faithful system is paused or skipped. While
//!   an overlay is open the router owns every key ([`KeyLatch`]), so the
//!   pump only runs the idle steps an idle player gets.
//! - `close_when_not_idle` closes it when the game leaves that prompt.
//! - On close the held-direction repeat is released, keys still held are
//!   latched by the router until released, and an [`OverlayChanged`] asks
//!   the feature to redraw (remove its node). Overlays draw as `bevy_ui`
//!   nodes over the frame and never write the frame image; one that draws
//!   over the game picture starts from [`copy_last_frame`].
//!
//! The title-screen menu (`menu.title_layer`) is chapter 3's and opens
//! through its own path; nothing here runs before gameplay.

use bevy::prelude::*;
use u5_runtime::enhancements::{FeatureToggles, IdlePromptKind};

use super::EnhSet;
use super::idle::{IdleView, load_holding, speaker_playing};
use super::input::{EnhAction, KeyLatch, KeyMap, install_input_router, is_modifier};
use super::toast::{Toasts, install_shell_services};
use crate::{EmulatorLoadLatency, SpeakerQueue, SpeakerVoice, VisualState};

/// Refusal toasts (1.2.6): one plain line each.
const MENU_REFUSED: &str = "The menu opens at the command prompt.";
const JOURNAL_REFUSED: &str = "The journal opens at the command prompt.";
const MAP_REFUSED: &str = "The map opens at the command prompt.";
const CHEATS_REFUSED: &str = "The cheats page opens at a command prompt.";

const EXPLORATION: &[IdlePromptKind] = &[IdlePromptKind::Exploration];
const EXPLORATION_OR_COMBAT: &[IdlePromptKind] =
    &[IdlePromptKind::Exploration, IdlePromptKind::Combat];

/// The four modal overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverlayId {
    Menu,
    Journal,
    Map,
    Cheats,
}

impl OverlayId {
    pub const ALL: [OverlayId; 4] = [
        OverlayId::Menu,
        OverlayId::Journal,
        OverlayId::Map,
        OverlayId::Cheats,
    ];

    /// The idle prompts it opens at: the exploration prompt, and for the
    /// cheats page also a combat member's bare prompt (1.3.7).
    pub fn kinds(self) -> &'static [IdlePromptKind] {
        match self {
            OverlayId::Cheats => EXPLORATION_OR_COMBAT,
            OverlayId::Menu | OverlayId::Journal | OverlayId::Map => EXPLORATION,
        }
    }

    /// The hotkey action that opens it.
    pub fn opener(self) -> EnhAction {
        match self {
            OverlayId::Menu => EnhAction::OpenMenu,
            OverlayId::Journal => EnhAction::OpenJournal,
            OverlayId::Map => EnhAction::OpenMap,
            OverlayId::Cheats => EnhAction::OpenCheats,
        }
    }

    /// The toast shown when it is asked for away from its prompts.
    pub fn refusal(self) -> &'static str {
        match self {
            OverlayId::Menu => MENU_REFUSED,
            OverlayId::Journal => JOURNAL_REFUSED,
            OverlayId::Map => MAP_REFUSED,
            OverlayId::Cheats => CHEATS_REFUSED,
        }
    }
}

/// How an open overlay treats world time. Every overlay is Live in v1
/// (U1); `Frozen` belongs to the optional P6b, and nothing sets it yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PauseMode {
    #[default]
    Live,
    Frozen,
}

/// The one overlay registry. Present whenever an overlay feature is on.
/// Keys held over an open or close are latched by the router's
/// [`KeyLatch`], not here.
#[derive(Resource, Debug, Default)]
pub struct OverlayState {
    open: Option<OverlayId>,
    /// Live unless `enhancements.freeze_overlays` (P6b).
    mode: PauseMode,
}

impl OverlayState {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The overlay that is open.
    pub fn open(&self) -> Option<OverlayId> {
        self.open
    }

    pub fn frozen(&self) -> bool {
        self.is_open() && self.mode == PauseMode::Frozen
    }
}

/// What a feature asks of the overlay model.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRequest {
    Open(OverlayId),
    /// Closes the overlay if it is the open one.
    Close(OverlayId),
}

/// An open request away from the overlay's prompts. The toast shows
/// [`OverlayId::refusal`].
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayRefused(pub OverlayId);

/// Why an overlay closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// Its feature asked.
    Requested,
    /// Another overlay opened in its place.
    Replaced,
    /// The game left the prompt it opened at.
    NotIdle,
}

/// An overlay opened or closed: its feature draws or removes its node.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayChanged {
    Opened(OverlayId),
    Closed { id: OverlayId, why: CloseReason },
}

/// Whether the toggles in force turn on an overlay feature: the action
/// that opens one is bound.
pub fn overlay_feature_on(toggles: FeatureToggles) -> bool {
    OverlayId::ALL
        .into_iter()
        .any(|id| id.opener().enabled(toggles))
}

/// Adds the overlay model when an overlay feature is on; otherwise adds
/// nothing. Called from [`super::EnhancementsPlugin`] after the router.
pub(super) fn add_overlay_model(app: &mut App, toggles: FeatureToggles) {
    if overlay_feature_on(toggles) {
        install_overlay_model(app);
    }
}

/// The model, its events and its systems. An open overlay needs the
/// router's modal input, so the router is installed too if no key bound it;
/// a refusal needs the toast line, so the shell services come with it.
pub(super) fn install_overlay_model(app: &mut App) {
    if !app.world().contains_resource::<KeyLatch>() {
        install_input_router(app, KeyMap::default());
    }
    install_shell_services(app);
    app.init_resource::<OverlayState>()
        .add_event::<OverlayRequest>()
        .add_event::<OverlayRefused>()
        .add_event::<OverlayChanged>()
        .add_systems(Update, apply_overlay_requests.in_set(EnhSet::Apply))
        .add_systems(Update, toast_overlay_refusals.in_set(EnhSet::Observe))
        .add_systems(
            Update,
            close_when_not_idle
                .run_if(overlay_open)
                .in_set(EnhSet::Observe),
        );
}

fn overlay_open(state: Res<OverlayState>) -> bool {
    state.is_open()
}

/// Opens `id`. The router takes the keyboard on its next run; the keys
/// typed this frame behind the hotkey are taken from the input driver now,
/// so none reaches the game once the overlay is open.
fn open_overlay(
    state: &mut OverlayState,
    latch: &mut KeyLatch,
    keyboard: Option<&mut ButtonInput<KeyCode>>,
    visual: Option<&mut VisualState>,
    changed: &mut EventWriter<OverlayChanged>,
    id: OverlayId,
) {
    if let Some(previous) = state.open.take() {
        changed.write(OverlayChanged::Closed {
            id: previous,
            why: CloseReason::Replaced,
        });
    }
    state.open = Some(id);
    latch.set_modal(true);
    if let Some(keyboard) = keyboard {
        let keys: Vec<KeyCode> = keyboard
            .get_pressed()
            .chain(keyboard.get_just_pressed())
            .copied()
            .filter(|key| !is_modifier(*key))
            .collect();
        for key in keys {
            keyboard.reset(key);
        }
    }
    if let Some(visual) = visual {
        visual.held_direction_repeat.release();
    }
    changed.write(OverlayChanged::Opened(id));
}

/// Closes the open overlay (1.3.7 close handling): releases the held
/// direction, hands the keyboard back (the router latches what is still
/// held) and asks for the redraw.
fn close_overlay(
    state: &mut OverlayState,
    latch: &mut KeyLatch,
    visual: Option<&mut VisualState>,
    changed: &mut EventWriter<OverlayChanged>,
    why: CloseReason,
) {
    let Some(id) = state.open.take() else {
        return;
    };
    latch.set_modal(false);
    if let Some(visual) = visual {
        visual.held_direction_repeat.release();
    }
    changed.write(OverlayChanged::Closed { id, why });
}

/// Opens and closes overlays as features ask, in `EnhSet::Apply`. A
/// feature that asks in the same frame orders its system before this one.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_overlay_requests(
    mut requests: EventReader<OverlayRequest>,
    mut state: ResMut<OverlayState>,
    mut latch: ResMut<KeyLatch>,
    mut keyboard: Option<ResMut<ButtonInput<KeyCode>>>,
    mut visual: Option<ResMut<VisualState>>,
    speaker_queue: Option<Res<SpeakerQueue>>,
    speaker_voices: Query<(), With<SpeakerVoice>>,
    load_latency: Option<Res<EmulatorLoadLatency>>,
    mut refused: EventWriter<OverlayRefused>,
    mut changed: EventWriter<OverlayChanged>,
) {
    for request in requests.read().copied() {
        match request {
            OverlayRequest::Open(id) => {
                if state.open == Some(id) {
                    continue;
                }
                let view = IdleView {
                    visual: visual.as_deref(),
                    speaker_playing: speaker_playing(speaker_queue.as_deref(), &speaker_voices),
                    load_holding: load_holding(load_latency.as_deref()),
                };
                // Switching from one overlay to another (Tab inside the
                // menu) keeps the test that holds the first one open.
                let allowed = if state.is_open() {
                    view.still_at(id.kinds())
                } else {
                    view.at(id.kinds())
                };
                if allowed {
                    open_overlay(
                        &mut state,
                        &mut latch,
                        keyboard.as_deref_mut(),
                        visual.as_deref_mut(),
                        &mut changed,
                        id,
                    );
                } else {
                    refused.write(OverlayRefused(id));
                }
            }
            OverlayRequest::Close(id) => {
                if state.open == Some(id) {
                    close_overlay(
                        &mut state,
                        &mut latch,
                        visual.as_deref_mut(),
                        &mut changed,
                        CloseReason::Requested,
                    );
                }
            }
        }
    }
}

/// Closes the open overlay once the game leaves the prompt it opened at
/// (1.3.7), in `EnhSet::Observe`, after the input driver.
pub(crate) fn close_when_not_idle(
    mut state: ResMut<OverlayState>,
    mut latch: ResMut<KeyLatch>,
    mut visual: Option<ResMut<VisualState>>,
    load_latency: Option<Res<EmulatorLoadLatency>>,
    mut changed: EventWriter<OverlayChanged>,
) {
    let Some(id) = state.open else {
        return;
    };
    let view = IdleView {
        visual: visual.as_deref(),
        // Not read by `still_at`: an ambient voice keeps it open.
        speaker_playing: false,
        load_holding: load_holding(load_latency.as_deref()),
    };
    if view.still_at(id.kinds()) {
        return;
    }
    close_overlay(
        &mut state,
        &mut latch,
        visual.as_deref_mut(),
        &mut changed,
        CloseReason::NotIdle,
    );
}

/// Shows each refusal's 2 s toast (1.2.6). Two in one frame show the last.
fn toast_overlay_refusals(mut refused: EventReader<OverlayRefused>, mut toasts: ResMut<Toasts>) {
    for OverlayRefused(id) in refused.read().copied() {
        toasts.short(id.refusal());
    }
}

/// A copy of a frame the shell presented.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCopy {
    pub width: u32,
    pub height: u32,
    /// RGBA, row by row from the top.
    pub rgba: Vec<u8>,
}

/// Copies the last presented frame (design 1.3.7, "Drawing"), for an
/// overlay that draws over the game picture into its own image. `frame` is
/// the shell's framebuffer handle (`VisualState::image_handle` in play).
/// It reads the asset only: the frame is not marked changed, and the play
/// renderer, which takes `&mut PlayState`, is never called. `None` when the
/// frame has no CPU-side pixels.
pub fn copy_last_frame(images: &Assets<Image>, frame: &Handle<Image>) -> Option<FrameCopy> {
    let image = images.get(frame)?;
    let rgba = image.data.clone()?;
    let size = image.size();
    (rgba.len() == size.x as usize * size.y as usize * 4).then_some(FrameCopy {
        width: size.x,
        height: size.y,
        rgba,
    })
}

#[cfg(test)]
pub(crate) mod tests;
