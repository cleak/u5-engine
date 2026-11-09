//! The idle-prompt predicate, shell half, and the run condition every
//! consumer uses (Phase B design 1.2.6, 1.3.7).
//!
//! The runtime half, [`u5_runtime::PlayState::idle_command_prompt`], answers from the
//! game state. The shell adds what only it knows: keys queued for the next
//! read, the speaker voice (an effect owns the thread, `audio.md §2`), the
//! idle redraw after a sweep, the emulator-profile load hold and the
//! shell's own input block (`visual_input_blocked`). Together they are
//! `IdleView::kind`, and [`at_idle_prompt`] is the run condition built
//! on it: opening an overlay, applying a toggle held for the idle prompt,
//! applying a cheat and the start-sound typers all use it.
//!
//! Staying open is a weaker test than opening (`IdleView::still_at`).
//! The game's own idle steps sound ambient effects (`audio.md §8.10`); a
//! voice that starts while the player reads is not the game leaving its
//! prompt, so it does not close an overlay. Everything else does.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use u5_runtime::enhancements::IdlePromptKind;

use crate::{EmulatorLoadLatency, SpeakerQueue, SpeakerVoice, VisualState, visual_input_blocked};

/// The shell half of the predicate (design 1.3.7): nothing is queued for
/// the next read, no speaker effect sounds or waits, no idle redraw is
/// owed after a sweep, the load hold is over and the shell reads keys.
pub(crate) fn shell_quiescent(
    visual: &VisualState,
    speaker_busy: bool,
    load_holding: bool,
) -> bool {
    visual.typed_keys.is_empty()
        && !speaker_busy
        && !visual.sweep_idle_redraw_pending
        && !load_holding
        && !visual_input_blocked(&visual.state)
}

/// What the predicate reads, borrowed. Systems that also write
/// [`VisualState`] build one from their own parameters; the rest take an
/// [`IdleProbe`].
pub(crate) struct IdleView<'a> {
    /// `None` outside gameplay (the title screen, a test app): nothing is
    /// idle there.
    pub visual: Option<&'a VisualState>,
    /// The speaker plays, or effects wait for it.
    pub speaker_playing: bool,
    /// The emulator-profile load hold is in force.
    pub load_holding: bool,
}

impl IdleView<'_> {
    /// The idle command prompt the game waits at, both halves considered.
    pub(crate) fn kind(&self) -> Option<IdlePromptKind> {
        let visual = self.visual?;
        // An effect recorded and not yet sent owns the thread too.
        let speaker_busy =
            self.speaker_playing || visual.sound_effect_cursor != visual.state.sound_effect_serial;
        if !shell_quiescent(visual, speaker_busy, self.load_holding) {
            return None;
        }
        visual.state.idle_command_prompt()
    }

    /// Whether the game waits at one of `kinds`.
    pub(crate) fn at(&self, kinds: &[IdlePromptKind]) -> bool {
        self.kind().is_some_and(|kind| kinds.contains(&kind))
    }

    /// Whether an overlay opened at one of `kinds` may stay open: the game
    /// is still at that prompt and the shell still reads keys. A speaker
    /// voice the idle steps started does not count against it, and no key
    /// can be queued while the router owns the keyboard.
    pub(crate) fn still_at(&self, kinds: &[IdlePromptKind]) -> bool {
        let Some(visual) = self.visual else {
            return false;
        };
        !self.load_holding
            && !visual.sweep_idle_redraw_pending
            && !visual_input_blocked(&visual.state)
            && visual
                .state
                .idle_command_prompt()
                .is_some_and(|kind| kinds.contains(&kind))
    }
}

/// Whether the speaker plays or effects wait for it.
pub(crate) fn speaker_playing(
    queue: Option<&SpeakerQueue>,
    voices: &Query<(), With<SpeakerVoice>>,
) -> bool {
    queue.is_some_and(|queue| !queue.0.is_empty()) || !voices.is_empty()
}

/// Whether the emulator-profile load hold is in force.
pub(crate) fn load_holding(hold: Option<&EmulatorLoadLatency>) -> bool {
    hold.is_some_and(|hold| hold.holding())
}

/// Everything the idle predicate reads, as one read-only system parameter.
#[derive(SystemParam)]
pub struct IdleProbe<'w, 's> {
    visual: Option<Res<'w, VisualState>>,
    speaker_queue: Option<Res<'w, SpeakerQueue>>,
    speaker_voices: Query<'w, 's, (), With<SpeakerVoice>>,
    load_latency: Option<Res<'w, EmulatorLoadLatency>>,
}

impl IdleProbe<'_, '_> {
    pub(crate) fn view(&self) -> IdleView<'_> {
        IdleView {
            visual: self.visual.as_deref(),
            speaker_playing: speaker_playing(self.speaker_queue.as_deref(), &self.speaker_voices),
            load_holding: load_holding(self.load_latency.as_deref()),
        }
    }
}

/// Run condition: the game waits at one of `kinds` with nothing pending,
/// runtime and shell halves both (design 1.3.7).
pub fn at_idle_prompt(kinds: &'static [IdlePromptKind]) -> impl Fn(IdleProbe) -> bool + Clone {
    move |probe: IdleProbe| probe.view().at(kinds)
}

#[cfg(test)]
mod tests;
