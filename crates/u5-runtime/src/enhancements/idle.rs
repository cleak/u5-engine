//! The idle-prompt predicate, runtime half (Phase B design 1.2.6, 1.3.7).
//!
//! An overlay opens, a toggle held for the idle prompt applies and a cheat
//! applies only while the game waits for a fresh command key with nothing
//! pending. This module answers the runtime's part of that question from
//! `&PlayState` alone: it reads, never writes, and adds no field. The shell
//! adds its own half (queued keys, the speaker, its input blocks) in
//! `u5_bevy::enh::idle`; the terminal frontend adds "no queued script keys".
//!
//! The predicate is deliberately conservative. A state it wrongly calls busy
//! only refuses an overlay; a state it wrongly calls idle could open one in
//! the middle of a sequence. So every hold, presentation, prompt and queue
//! the shell drains between frames counts as busy ([`IdleBlocker`]).
//!
//! **New drains must be classified.** Every `take_pending_*` and wall-clock
//! `advance_*_to` method a frontend calls is listed in [`GUARDED_DRAINS`]
//! with the blocker that keeps the predicate false while it has work. A
//! source scan (`tests/enhancements_contract.rs`) fails on a name missing
//! from that list (design 1.7, the first risk).

use crate::PartyCapability;
use crate::PlayState;

/// Which idle command prompt the game is waiting at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdlePromptKind {
    /// The world, town, dwelling or dungeon command prompt, outside combat.
    Exploration,
    /// A combat member's bare command prompt.
    Combat,
}

/// Why the game is not at an idle command prompt: the first condition that
/// failed, in the order [`PlayState::idle_blocker`] tests them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IdleBlocker {
    /// A message is held behind a pause or key wait (`pending_text_hold`).
    TextHold,
    /// Staged narration is printing on its timer.
    StagedNarration,
    /// Keys typed during a hold wait to be replayed.
    BufferedHoldKeys,
    /// The White potion or X-Ray visibility sweep is running.
    VisibilitySweep,
    /// A Blackthorn audience, capture, execution, rescue or guard demand.
    Blackthorn,
    /// A cinematic or presentation owns the screen: the shrine and Codex
    /// presentations, the Stonegate black viewport, a spell's frozen
    /// message window, a swallow presentation's marker, a cinematic hold.
    Cinematic,
    /// The endgame, from the absorption hold on.
    Endgame,
    /// A completed presentation waits for the frontend to play it (one of
    /// the `take_pending_*` queues).
    PresentationQueue,
    /// A typed-line reader is open: conversation, Yell, the shrine and
    /// restoration prompts, the wishing well, a shop.
    LineSession,
    /// A modal prompt is open: a selector, a command's own session, a
    /// direction or yes/no prompt, an arrest, a View overlay, a spell's
    /// argument.
    ModalPrompt,
    /// The combat targeting cursor is open.
    TargetingCursor,
    /// A command has not returned: its echo or its turn is still owed.
    CommandInProgress,
    /// No party member can act: the automatic sleep or defeat pass runs.
    PartyNotReady,
    /// Combat is walking its automatic actors or still owes their lines.
    CombatWalk,
    /// A ship under sail advances on its own; the helper does not wait.
    UnderSail,
    /// Work carried across a fight still runs at the next input boundary.
    CarriedAction,
}

/// A wall-clock or queue-draining call a frontend makes between frames, and
/// the blocker that holds the idle predicate false while it has work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuardedDrain {
    /// The method name, as called.
    pub method: &'static str,
    pub blocker: IdleBlocker,
}

/// Every `take_pending_*` and `advance_*_to` method a frontend calls. A new
/// one fails the source scan until it is added here with its blocker and
/// that blocker covers the state it drains.
pub const GUARDED_DRAINS: &[GuardedDrain] = &[
    GuardedDrain {
        method: "take_pending_map_viewport_dissolves",
        blocker: IdleBlocker::PresentationQueue,
    },
    GuardedDrain {
        method: "take_pending_blackthorn_rescue_playbacks",
        blocker: IdleBlocker::PresentationQueue,
    },
    GuardedDrain {
        method: "take_pending_combat_terrain_reveals",
        blocker: IdleBlocker::PresentationQueue,
    },
    GuardedDrain {
        method: "take_pending_potion_flash",
        blocker: IdleBlocker::PresentationQueue,
    },
    GuardedDrain {
        method: "take_pending_stonegate_trapdoor_playback",
        blocker: IdleBlocker::PresentationQueue,
    },
    GuardedDrain {
        method: "advance_text_hold_to",
        blocker: IdleBlocker::TextHold,
    },
    GuardedDrain {
        method: "advance_staged_narration_to",
        blocker: IdleBlocker::StagedNarration,
    },
    GuardedDrain {
        method: "advance_endgame_tableau_animation_to",
        blocker: IdleBlocker::Endgame,
    },
    GuardedDrain {
        method: "advance_blackthorn_rescue_animation_to",
        blocker: IdleBlocker::Blackthorn,
    },
    GuardedDrain {
        method: "advance_blackthorn_audience_composite_to",
        blocker: IdleBlocker::Blackthorn,
    },
];

impl PlayState {
    /// `None` unless the game waits for a fresh command key with nothing
    /// pending (design 1.3.7). `Exploration` is the world, town, dwelling
    /// or dungeon command prompt; `Combat` is a combat member's bare
    /// command prompt.
    pub fn idle_command_prompt(&self) -> Option<IdlePromptKind> {
        match self.idle_blocker() {
            Some(_) => None,
            None if self.combat_active => Some(IdlePromptKind::Combat),
            None => Some(IdlePromptKind::Exploration),
        }
    }

    /// Whether the game waits at the exploration command prompt with
    /// nothing pending.
    pub fn at_idle_command_prompt(&self) -> bool {
        matches!(
            self.idle_command_prompt(),
            Some(IdlePromptKind::Exploration)
        )
    }

    /// The first reason the game is not at an idle command prompt, or
    /// `None` when it is.
    pub fn idle_blocker(&self) -> Option<IdleBlocker> {
        let checks: [(IdleBlocker, bool); 16] = [
            (IdleBlocker::TextHold, self.text_hold_active()),
            (
                IdleBlocker::StagedNarration,
                self.staged_narration_active() || !self.staged_stats_panels.is_empty(),
            ),
            (
                IdleBlocker::BufferedHoldKeys,
                !self.keys_buffered_during_hold.is_empty(),
            ),
            (
                IdleBlocker::VisibilitySweep,
                self.visibility_sweep.is_some(),
            ),
            (IdleBlocker::Blackthorn, self.blackthorn_running()),
            (IdleBlocker::Cinematic, self.cinematic_running()),
            (IdleBlocker::Endgame, self.endgame_running()),
            (IdleBlocker::PresentationQueue, self.presentation_queued()),
            (IdleBlocker::LineSession, self.line_session_open()),
            (IdleBlocker::ModalPrompt, self.modal_prompt_open()),
            (
                IdleBlocker::TargetingCursor,
                self.active_combat_targeting.is_some(),
            ),
            (IdleBlocker::CommandInProgress, self.command_in_progress()),
            (
                IdleBlocker::PartyNotReady,
                !matches!(self.party_capability(), PartyCapability::CanAct { .. }),
            ),
            (
                IdleBlocker::CombatWalk,
                self.combat_active && !self.combat_member_prompt_bare(),
            ),
            (IdleBlocker::UnderSail, self.under_sail_wait_pass_applies()),
            (
                IdleBlocker::CarriedAction,
                !self.combat_active && self.carried_action_owed(),
            ),
        ];
        checks
            .into_iter()
            .find_map(|(blocker, busy)| busy.then_some(blocker))
    }

    fn cinematic_running(&self) -> bool {
        self.cinematic_hold_owns_input()
            || self.cutscene_black_viewport
            || self.shrine_presentation_map.is_some()
            || self.shrine_approach_walk.is_some()
            || self.codex_presentation_steps != 0
            || self.presentation_viewport_inverted
            || self.message_window_flash_hold.is_active()
            || self.party_marker_tile_override.is_some()
    }

    fn blackthorn_running(&self) -> bool {
        self.active_blackthorn.is_some()
            || self.blackthorn_capture.is_some()
            || self.blackthorn_audience_map.is_some()
            || self.pending_blackthorn_rescue.is_some()
            || self.pending_blackthorn_audience_exit
            || self.pending_blackthorn_merciful_death
            || self.pending_blackthorn_execution_swell
            || self.pending_blackthorn_execution_victim.is_some()
            || self.active_blackthorn_guard_demand.is_some()
    }

    fn endgame_running(&self) -> bool {
        self.endgame.is_some()
            || self.endgame_entry_redraw_pending
            || self.endgame_absorption_arena.is_some()
    }

    /// One of the queues a frontend drains with `take_pending_*`.
    fn presentation_queued(&self) -> bool {
        !self.pending_map_viewport_dissolves.is_empty()
            || !self.pending_blackthorn_rescue_playbacks.is_empty()
            || !self.pending_combat_terrain_reveals.is_empty()
            || self.pending_potion_flash.is_some()
            || self.pending_stonegate_trapdoor_playback.is_some()
    }

    fn line_session_open(&self) -> bool {
        self.active_conversation.is_some()
            || self.active_yell.is_some()
            || self.active_shrine.is_some()
            || self.pending_shrine_third_mantra.is_some()
            || self.active_shrine_restoration.is_some()
            || self.active_wishing_well.is_some()
            || self.active_shop.is_some()
    }

    fn modal_prompt_open(&self) -> bool {
        self.active_party_selector.is_some()
            || self.active_z_stats.is_some()
            || self.active_ready.is_some()
            || self.active_use.is_some()
            || self.active_cast.is_some()
            || self.active_cast_followup.is_some()
            || self.pending_cast_argument.is_some()
            || self.active_rest.is_some()
            || self.active_jimmy.is_some()
            || self.active_surface_chest.is_some()
            || self.active_mix.is_some()
            || self.active_new_order.is_some()
            || self.active_direction_prompt.is_some()
            || self.active_yes_no_prompt.is_some()
            || self.pending_town_arrest.is_some()
            || self.active_view_overlay.is_some()
            || self.prompt_waits_in_shared_wait()
    }

    /// A command's echo is open or the turn it charges is still owed, so
    /// its handler has not returned to the mode loop.
    fn command_in_progress(&self) -> bool {
        self.pending_command_echo.is_some()
            || self.pending_modal_wait_turn.is_some()
            || self.exploration_acted_charge_owed
            || self.dungeon_acted_turn_before.is_some()
            || self.pending_talk_shop_turn
            || self.pending_look_prompt_turn
            || self.pending_talk_conversation_turn
    }

    /// A keyboard-driven member's turn is open and nothing of the round
    /// walk is still to print (`combat.md §8`, `§11.1`).
    fn combat_member_prompt_bare(&self) -> bool {
        self.pending_combat_actor_slot.is_some()
            && self.combat_owed_monster_attack_narrations.is_empty()
            && self.combat_pending_divide_line.is_none()
            && self.combat_pending_hazard_lines.is_empty()
            && self.combat_held_faint_lines.is_none()
    }

    /// Work a fight carried over that still runs at the next input boundary
    /// after it returns.
    fn carried_action_owed(&self) -> bool {
        self.pending_dungeon_surface_exit.is_some()
            || self.pending_dungeon_underfoot_reaction.is_some()
            || self.pending_dungeon_post_action_helper
            || self.pending_town_conflict.is_some()
            || self.pending_difficult_terrain_step.is_some()
            || !self.pending_outdoor_reaction_slots.is_empty()
    }
}

#[cfg(test)]
mod tests;
