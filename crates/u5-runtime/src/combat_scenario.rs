//! Scripted combat scenario driver for tests and verification.
//!
//! A scenario is a sequence of typed-input lines fed through the
//! combat input pipeline against an existing PlayState. Each line is
//! one player turn; the driver applies the matching combat player
//! command, allows the round walker to fast-forward to the next
//! player slot, and records the resulting state transitions for
//! inspection.

use crate::combat_actor::{CombatRoundLoopExit, COMBAT_PARTY_ACTOR_SLOTS};
use crate::combat_frame::CombatActorSlotDispatchApplication;
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
    /// Print combat status (no-op turn for the round walker).
    StatsPanel,
    /// `Q` — combat Quit / abandon party (defeat exit).
    Quit,
    /// `X` — combat X-it cleanup (only succeeds when no foes remain).
    Xit,
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
        state.ensure_pending_combat_player_turn();

        let Some(actor_slot) = state.pending_combat_actor_slot.take() else {
            // Combat is over (no player slot ready and walker is idle).
            // Determine exit reason by inspecting the most recently
            // applied control state.
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

        let pre_combat_active = state.combat_active;
        let mut should_break = false;
        match input {
            CombatScenarioInput::AttackDirection(dir) | CombatScenarioInput::Move(dir) => {
                let attacker_group =
                    crate::combat_actor::resolve_combat_target_group(actor_slot, None, false);
                let _ = state.apply_combat_step_or_attack_primitive(
                    actor_slot,
                    attacker_group,
                    *dir,
                    true,
                );
            }
            CombatScenarioInput::Pass | CombatScenarioInput::StatsPanel => {
                // Pass / Z spend the player's turn without other
                // effects.
            }
            CombatScenarioInput::Quit => {
                state
                    .apply_combat_round_loop_exit(crate::combat_actor::CombatRoundLoopExit::Defeat);
                result.steps.push(CombatScenarioStep::Exited(
                    crate::combat_actor::CombatRoundLoopExit::Defeat,
                ));
                result.final_exit = Some(crate::combat_actor::CombatRoundLoopExit::Defeat);
                should_break = true;
            }
            CombatScenarioInput::Xit => {
                if !crate::combat_actor::combat_has_active_not_dead_non_party_actor(
                    &state.combat_actors,
                ) {
                    state.apply_combat_round_loop_exit(
                        crate::combat_actor::CombatRoundLoopExit::LeaveCombat,
                    );
                    result.steps.push(CombatScenarioStep::Exited(
                        crate::combat_actor::CombatRoundLoopExit::LeaveCombat,
                    ));
                    result.final_exit = Some(crate::combat_actor::CombatRoundLoopExit::LeaveCombat);
                    should_break = true;
                }
            }
        }

        if should_break {
            break;
        }
        result
            .steps
            .push(CombatScenarioStep::AppliedToSlot(actor_slot));
        if !pre_combat_active || !state.combat_active {
            break;
        }
    }

    result.combat_active_at_end = state.combat_active;
    let _ = consume_round_walk_application_history;
    result
}

/// Test-only helper to peek at the round walker's per-slot dispatch
/// records and return the most recent exit, if any.
pub fn consume_round_walk_application_history(
    applications: &[CombatActorSlotDispatchApplication],
) -> Option<CombatRoundLoopExit> {
    use crate::combat_actor::CombatRoundLoopControl;
    for entry in applications.iter().rev() {
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

    #[test]
    fn quit_input_marks_defeat_exit() {
        let mut state = crate::test_fixtures::test_state(crate::test_fixtures::open_grid(), 5, 5);
        // Pretend combat is active with one fake actor in the player
        // slot. We exercise only the Quit branch since it does not
        // need full combat actor setup to verify the exit path.
        state.combat_active = true;
        state.pending_combat_actor_slot = Some(0);
        let result = run_combat_scenario(&mut state, &[CombatScenarioInput::Quit]);
        assert_eq!(result.final_exit, Some(CombatRoundLoopExit::Defeat));
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
