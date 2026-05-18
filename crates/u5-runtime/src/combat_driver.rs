//! Combat scenario driver — a thin convenience wrapper around the
//! per-actor combat round walker. Tests and the terminal harness can
//! use this to step a complete fight forward without re-implementing
//! the player-input / AI-turn alternation each time.

use crate::combat_actor::CombatRoundLoopExit;
use crate::combat_frame::CombatRoundWalkApplication;

/// Per-step driver outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatDriverStep {
    /// A player slot is ready to act; caller should poll input next.
    AwaitingPlayer(usize),
    /// Round wrapped without stopping; caller may continue or break.
    RoundCycled,
    /// Combat exited (victory / defeat / escape).
    Exited(CombatRoundLoopExit),
}

/// Classify the round walker's most recent application into a driver
/// step.
pub fn classify_round_walk_application(
    application: &CombatRoundWalkApplication,
    pending_player_slot: Option<usize>,
) -> CombatDriverStep {
    if let Some(slot) = pending_player_slot {
        return CombatDriverStep::AwaitingPlayer(slot);
    }
    use crate::combat_actor::CombatRoundLoopControl;
    use crate::combat_frame::CombatActorSlotDispatchApplication;
    for entry in application.applications.iter() {
        if let CombatActorSlotDispatchApplication::Slot { control_after, .. } = entry {
            if let CombatRoundLoopControl::Exit(exit) = control_after {
                return CombatDriverStep::Exited(*exit);
            }
        }
        if let CombatActorSlotDispatchApplication::EndOfRound { control } = entry {
            if let CombatRoundLoopControl::Exit(exit) = control {
                return CombatDriverStep::Exited(*exit);
            }
        }
    }
    CombatDriverStep::RoundCycled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_actor::{CombatRoundLoopControl, CombatRoundLoopExit};
    use crate::combat_frame::{CombatActorSlotDispatchApplication, CombatRoundWalkStopReason};

    fn empty_application() -> CombatRoundWalkApplication {
        CombatRoundWalkApplication {
            start_slot: 0,
            next_slot: 0,
            stop_reason: CombatRoundWalkStopReason::EndOfRound,
            applications: Vec::new(),
        }
    }

    #[test]
    fn awaiting_player_takes_priority_over_cycle() {
        let app = empty_application();
        let step = classify_round_walk_application(&app, Some(2));
        assert_eq!(step, CombatDriverStep::AwaitingPlayer(2));
    }

    #[test]
    fn empty_application_with_no_player_slot_returns_round_cycled() {
        let app = empty_application();
        let step = classify_round_walk_application(&app, None);
        assert_eq!(step, CombatDriverStep::RoundCycled);
    }

    #[test]
    fn end_of_round_exit_control_classifies_as_exited() {
        let mut app = empty_application();
        app.applications
            .push(CombatActorSlotDispatchApplication::EndOfRound {
                control: CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat),
            });
        let step = classify_round_walk_application(&app, None);
        assert_eq!(
            step,
            CombatDriverStep::Exited(CombatRoundLoopExit::LeaveCombat)
        );
    }

    #[test]
    fn slot_with_defeat_exit_classifies_as_exited() {
        let mut app = empty_application();
        app.applications
            .push(CombatActorSlotDispatchApplication::Slot {
                slot: 0,
                phase_tick: None,
                action: crate::combat_frame::CombatActorDispatchAction::Inactive,
                control_after: CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat),
            });
        let step = classify_round_walk_application(&app, None);
        assert_eq!(step, CombatDriverStep::Exited(CombatRoundLoopExit::Defeat));
    }
}
