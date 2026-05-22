//! Rest / camp helper per `systems/rest-and-camp.md`.
//!
//! The party's H-Hole-up + rest path advances in-world time, with the
//! public sleep-ambush interruption predicate able to stop the rest.

use crate::{SLEEP_AMBUSH_INTERRUPT_DENOMINATOR, sleep_ambush_rest_interrupted};

/// Outcome of one rest tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestTickOutcome {
    /// Hour passed without an encounter; ordinary rest has no direct
    /// HP / MP recovery.
    Slept {
        hp_per_member: u16,
        mp_per_member: u8,
    },
    /// Sleep-ambush check fired; rest is interrupted and combat should
    /// be entered.
    InterruptedByAmbush,
}

/// `cleak/u5-spec#47`: ordinary rest is time advancement; no explicit
/// HP or MP recovery is applied by the rest helper itself. Hourly
/// poison, starvation, provisions, and ambient time effects still run
/// through the normal clock path.
pub const REST_HP_PER_HOUR: u16 = 0;
pub const REST_MP_PER_HOUR: u8 = 0;
pub const REST_INTERRUPT_DENOMINATOR: u8 = SLEEP_AMBUSH_INTERRUPT_DENOMINATOR;

/// Inputs the rest tick needs.
#[derive(Clone, Copy, Debug)]
pub struct RestTickInputs {
    /// Pre-rolled `u8`; the public rest path treats `roll % 64 == 0`
    /// as the sleep-ambush interruption.
    pub interrupt_roll: u8,
    /// `true` when a watch is being kept. The public interruption
    /// predicate remains one-in-sixty-four either way; the watcher
    /// selection belongs to prompt/status handling.
    pub keeping_watch: bool,
}

/// Resolve one rest tick.
pub fn resolve_rest_tick(inputs: RestTickInputs) -> RestTickOutcome {
    let _ = inputs.keeping_watch;
    if sleep_ambush_rest_interrupted(inputs.interrupt_roll % REST_INTERRUPT_DENOMINATOR) {
        RestTickOutcome::InterruptedByAmbush
    } else if inputs.keeping_watch {
        RestTickOutcome::Slept {
            hp_per_member: REST_HP_PER_HOUR,
            mp_per_member: REST_MP_PER_HOUR,
        }
    } else {
        RestTickOutcome::Slept {
            hp_per_member: REST_HP_PER_HOUR,
            mp_per_member: REST_MP_PER_HOUR,
        }
    }
}

/// Multi-tick driver. Resolves up to `hours` ticks, stopping at the
/// first interruption. Returns the total HP recovered (per member) and
/// the number of completed hours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestSessionResult {
    pub completed_hours: u8,
    pub total_hp_per_member: u16,
    pub total_mp_per_member: u8,
    pub interrupted: bool,
}

pub fn run_rest_session(hours: u8, rolls: &[u8], keeping_watch: bool) -> RestSessionResult {
    let mut completed = 0u8;
    let mut total_hp = 0u16;
    let mut total_mp = 0u8;
    let mut interrupted = false;
    for i in 0..hours {
        let roll = *rolls.get(i as usize).unwrap_or(&255);
        let outcome = resolve_rest_tick(RestTickInputs {
            interrupt_roll: roll,
            keeping_watch,
        });
        match outcome {
            RestTickOutcome::Slept {
                hp_per_member,
                mp_per_member,
            } => {
                completed += 1;
                total_hp = total_hp.saturating_add(hp_per_member);
                total_mp = total_mp.saturating_add(mp_per_member);
            }
            RestTickOutcome::InterruptedByAmbush => {
                interrupted = true;
                break;
            }
        }
    }
    RestSessionResult {
        completed_hours: completed,
        total_hp_per_member: total_hp,
        total_mp_per_member: total_mp,
        interrupted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_roll_uneventful_watch_advances_without_direct_recovery() {
        let outcome = resolve_rest_tick(RestTickInputs {
            interrupt_roll: 200,
            keeping_watch: true,
        });
        assert_eq!(
            outcome,
            RestTickOutcome::Slept {
                hp_per_member: REST_HP_PER_HOUR,
                mp_per_member: REST_MP_PER_HOUR,
            }
        );
    }

    #[test]
    fn zero_mod_sixty_four_interrupts_rest_with_ambush() {
        let outcome = resolve_rest_tick(RestTickInputs {
            interrupt_roll: 64,
            keeping_watch: false,
        });
        assert_eq!(outcome, RestTickOutcome::InterruptedByAmbush);
    }

    #[test]
    fn keeping_watch_does_not_modulate_ambush_predicate() {
        let no_watch = resolve_rest_tick(RestTickInputs {
            interrupt_roll: 1,
            keeping_watch: false,
        });
        let with_watch = resolve_rest_tick(RestTickInputs {
            interrupt_roll: 1,
            keeping_watch: true,
        });
        assert!(matches!(no_watch, RestTickOutcome::Slept { .. }));
        assert!(matches!(with_watch, RestTickOutcome::Slept { .. }));
    }

    #[test]
    fn full_session_with_high_rolls_has_no_direct_recovery_total() {
        let rolls = [200u8; 8];
        let result = run_rest_session(8, &rolls, true);
        assert_eq!(result.completed_hours, 8);
        assert_eq!(result.total_hp_per_member, 0);
        assert_eq!(result.total_mp_per_member, 0);
        assert!(!result.interrupted);
    }

    #[test]
    fn rest_session_stops_at_first_low_roll() {
        let rolls = [200u8, 200, 0, 200, 200];
        let result = run_rest_session(5, &rolls, false);
        assert_eq!(result.completed_hours, 2);
        assert!(result.interrupted);
    }

    #[test]
    fn rest_session_with_no_rolls_treats_default_as_safe() {
        let result = run_rest_session(3, &[], true);
        assert_eq!(result.completed_hours, 3);
        assert!(!result.interrupted);
    }

    #[test]
    fn unwatched_safe_outcome_has_no_direct_recovery() {
        let outcome = resolve_rest_tick(RestTickInputs {
            interrupt_roll: 200,
            keeping_watch: false,
        });
        assert_eq!(
            outcome,
            RestTickOutcome::Slept {
                hp_per_member: REST_HP_PER_HOUR,
                mp_per_member: REST_MP_PER_HOUR,
            }
        );
    }
}
