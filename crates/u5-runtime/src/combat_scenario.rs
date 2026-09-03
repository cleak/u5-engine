//! Scripted combat scenario driver for tests and verification.
//!
//! A scenario is a sequence of typed-input lines fed through the
//! combat input pipeline against an existing PlayState. Each line is
//! one player turn; the driver applies the matching combat player
//! command, allows the round walker to fast-forward to the next
//! player slot, and records the resulting state transitions for
//! inspection.

use crate::combat_actor::{CombatRoundLoopControl, CombatRoundLoopExit};
use crate::combat_frame::{
    CombatActorSlotDispatchApplication, CombatPlayerCommandAction, CombatPlayerCommandInput,
    CombatRoundWalkApplication, CombatRoundWalkStopReason,
};
use crate::play_state_struct::PlayState;

/// One scripted combat input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatScenarioInput {
    /// `A` followed by an inline direction code (1-4).
    AttackDirection(u8),
    /// Single direction code in `[1, 4]` — west/east/north/south.
    Move(u8),
    /// Pass / wait one phase.
    Pass,
    /// `Z` — open combat status; the action ends after the modal closes.
    StatsPanel,
    /// `Q` — free combat-scene refusal.
    Quit,
    /// `X` — free combat-scene refusal.
    Xit,
    /// Escape — request cleanup; succeeds only when no active foes remain.
    Escape,
}

/// What happened on one scripted step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatScenarioStep {
    /// Player input was applied to the active combat actor slot.
    AppliedToSlot(usize),
    /// No active combatant; the script tried to step without an actor.
    NoActiveCombatant,
    /// Combat exited (victory / defeat / escape).
    Exited(CombatRoundLoopExit),
}

/// Outcome of running an entire scripted combat scenario.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CombatScenarioResult {
    pub steps: Vec<CombatScenarioStep>,
    pub final_exit: Option<CombatRoundLoopExit>,
    pub combat_active_at_end: bool,
}

/// Drive `state.combat_actors`, `pending_combat_actor_slot`, and
/// related runtime state through the scripted input list. Returns
/// the per-step record so tests can assert on intermediate
/// transitions without relying on private internals.
pub fn run_combat_scenario(
    state: &mut PlayState,
    script: &[CombatScenarioInput],
) -> CombatScenarioResult {
    let mut result = CombatScenarioResult::default();
    for input in script {
        // Make sure the round walker has advanced to a player slot or
        // exited combat before we look for an input target.
        let entry_walk = state.ensure_pending_combat_player_turn();
        if let Some(exit) = entry_walk
            .as_ref()
            .and_then(consume_round_walk_application_history)
        {
            state.apply_combat_round_loop_exit(exit);
            result.steps.push(CombatScenarioStep::Exited(exit));
            result.final_exit = Some(exit);
            break;
        }

        let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
            result.steps.push(CombatScenarioStep::NoActiveCombatant);
            break;
        };

        if !state.combat_active {
            result
                .steps
                .push(CombatScenarioStep::Exited(CombatRoundLoopExit::LeaveCombat));
            result.final_exit = Some(CombatRoundLoopExit::LeaveCombat);
            break;
        }

        let command_input = scenario_command_input(input);
        let Some(application) =
            state.apply_combat_player_command_with_inputs(actor_slot, command_input)
        else {
            result.steps.push(CombatScenarioStep::NoActiveCombatant);
            break;
        };

        if let CombatRoundLoopControl::Exit(exit) = application.control_after {
            state.apply_combat_round_loop_exit(exit);
            result.steps.push(CombatScenarioStep::Exited(exit));
            result.final_exit = Some(exit);
            break;
        }

        result
            .steps
            .push(CombatScenarioStep::AppliedToSlot(actor_slot));
        if application.reprompt {
            state.pending_combat_actor_slot = Some(actor_slot);
            continue;
        }
        if matches!(
            application.action,
            CombatPlayerCommandAction::PromptForAttackDirection
        ) {
            state.pending_combat_actor_slot = Some(actor_slot);
            continue;
        }
        if let Some(exit) = advance_combat_round_after_scenario_actor(state, actor_slot) {
            result.steps.push(CombatScenarioStep::Exited(exit));
            result.final_exit = Some(exit);
            break;
        }
    }

    result.combat_active_at_end = state.combat_active;
    result
}

fn scenario_command_input(input: &CombatScenarioInput) -> CombatPlayerCommandInput {
    match input {
        CombatScenarioInput::AttackDirection(direction) => {
            CombatPlayerCommandInput::AttackDirection(*direction)
        }
        CombatScenarioInput::Move(direction) => CombatPlayerCommandInput::Direction(*direction),
        CombatScenarioInput::Pass => CombatPlayerCommandInput::Key(' '),
        CombatScenarioInput::StatsPanel => CombatPlayerCommandInput::Key('Z'),
        CombatScenarioInput::Quit => CombatPlayerCommandInput::Key('Q'),
        CombatScenarioInput::Xit => CombatPlayerCommandInput::Key('X'),
        CombatScenarioInput::Escape => CombatPlayerCommandInput::Key('\u{1b}'),
    }
}

fn advance_combat_round_after_scenario_actor(
    state: &mut PlayState,
    actor_slot: usize,
) -> Option<CombatRoundLoopExit> {
    state.next_combat_actor_slot = actor_slot.saturating_add(1).min(crate::COMBAT_ACTOR_SLOTS);
    for _ in 0..crate::COMBAT_ROUND_WALK_DRAIN_LIMIT {
        if !state.combat_active || state.pending_combat_actor_slot.is_some() {
            return None;
        }
        let start_slot = state.next_combat_actor_slot.min(crate::COMBAT_ACTOR_SLOTS);
        let application = state.apply_combat_round_walk_from_slot(
            start_slot,
            crate::COMBAT_PHASE_REFRESH_CONSTANT,
            false,
        );
        state.next_combat_actor_slot = match application.stop_reason {
            CombatRoundWalkStopReason::EndOfRound => 0,
            CombatRoundWalkStopReason::AwaitingPlayer
            | CombatRoundWalkStopReason::AutomaticAction
            | CombatRoundWalkStopReason::Exit => application.next_slot,
        };
        if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
            state.pending_combat_actor_slot =
                ready_player_slot_from_scenario_round_walk(&application);
        }
        if let Some(exit) = consume_round_walk_application_history(&application) {
            state.apply_combat_round_loop_exit(exit);
            return Some(exit);
        }
        if !matches!(
            application.stop_reason,
            CombatRoundWalkStopReason::EndOfRound
        ) || state.pending_combat_actor_slot.is_some()
        {
            return None;
        }
    }
    None
}

fn ready_player_slot_from_scenario_round_walk(
    application: &CombatRoundWalkApplication,
) -> Option<usize> {
    application.applications.iter().rev().find_map(|entry| {
        let CombatActorSlotDispatchApplication::Slot { slot, action, .. } = entry else {
            return None;
        };
        if matches!(
            action,
            crate::combat_frame::CombatActorDispatchAction::PlayerReady
        ) {
            Some(*slot)
        } else {
            None
        }
    })
}

/// Test-only helper to peek at the round walker's per-slot dispatch
/// records and return the most recent exit, if any.
pub fn consume_round_walk_application_history(
    application: &CombatRoundWalkApplication,
) -> Option<CombatRoundLoopExit> {
    for entry in application.applications.iter().rev() {
        match entry {
            CombatActorSlotDispatchApplication::EndOfRound { control }
            | CombatActorSlotDispatchApplication::Slot {
                control_after: control,
                ..
            } => {
                if let CombatRoundLoopControl::Exit(exit) = control {
                    return Some(*exit);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adjacent_skeleton_combat_state() -> PlayState {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 5, 5);
        state.combat_active = true;
        // `combat.md` Section 5.3 step 8's round-loop entry prologue - one full
        // world tick, of "variable and unbounded" draw cost - is a once-per-
        // encounter event. This fixture assembles a fight already under way, so
        // the prologue is already spent and its world tick's gameplay draw does
        // not sit in front of the seeded rolls below.
        state.combat_round_loop_prologue_ran = true;
        state.combat_terrain = [[0x04; crate::COMBAT_ARENA_SIDE]; crate::COMBAT_ARENA_SIDE];
        state.active_objects = vec![crate::ActiveObject::empty(); crate::OOL_SLOTS];
        state.active_objects[0] = crate::ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 5,
            y: 5,
            ..crate::ActiveObject::empty()
        };
        state.active_objects[8] = crate::ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 6,
            y: 5,
            ..crate::ActiveObject::empty()
        };
        state.combat_actors[0] = crate::CombatActorDescriptor::from_row([
            20,
            1,
            crate::COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.combat_actors[8] = crate::CombatActorDescriptor::from_row([
            20,
            1,
            crate::COMBAT_ACTOR_FLAG_SELECTABLE_40,
            33,
            8,
            0,
            6,
            5,
        ]);
        state.pending_combat_actor_slot = Some(0);
        state.party[0].status = b'G';
        state.party[0].hp = 1;
        state.party[0].max_hp = 20;
        // Seed re-chosen after `RETRACTIONS.md` R311 moved the shared
        // stream: the random-cardinal fallback is drawn lazily instead of
        // four codes up front, so an AI dispatch that never reaches the
        // fallback no longer spends four draws. (R308's prologue tick is
        // presentation-only and draws nothing, so it moves no seed.) This
        // seed keeps the skeleton's blow lethal.
        state.prng_state = 0x3270;
        state
    }

    #[test]
    fn quit_input_is_a_free_refusal_for_the_same_actor() {
        let mut state = adjacent_skeleton_combat_state();
        let result = run_combat_scenario(&mut state, &[CombatScenarioInput::Quit]);
        assert_eq!(result.steps, vec![CombatScenarioStep::AppliedToSlot(0)]);
        assert_eq!(result.final_exit, None);
        assert!(state.combat_active);
        assert_eq!(state.pending_combat_actor_slot, Some(0));
        assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (6, 5));
    }

    #[test]
    fn pass_input_records_round_walker_defeat_exit() {
        let mut state = adjacent_skeleton_combat_state();

        let result = run_combat_scenario(&mut state, &[CombatScenarioInput::Pass]);

        assert_eq!(
            result.steps,
            vec![
                CombatScenarioStep::AppliedToSlot(0),
                CombatScenarioStep::Exited(CombatRoundLoopExit::Defeat)
            ]
        );
        assert_eq!(result.final_exit, Some(CombatRoundLoopExit::Defeat));
        assert!(!result.combat_active_at_end);
        assert!(!state.combat_active);
    }

    #[test]
    fn empty_script_returns_no_steps() {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 5, 5);
        let result = run_combat_scenario(&mut state, &[]);
        assert!(result.steps.is_empty());
        assert_eq!(result.final_exit, None);
    }

    #[test]
    fn script_with_no_active_combatant_reports_no_active_combatant_step() {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 5, 5);
        // combat_active is false → ensure_pending_combat_player_turn
        // does not allocate a slot.
        let result = run_combat_scenario(
            &mut state,
            &[CombatScenarioInput::Pass, CombatScenarioInput::Pass],
        );
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0], CombatScenarioStep::NoActiveCombatant);
    }
}
