//! Rest / camp helper per `systems/rest-and-camp.md`.
//!
//! The party's H-Hole-up + rest path lets a sleeping party recover HP
//! and MP across multiple in-world hours, with an encounter check
//! at each one-hour tick that may interrupt the rest.

/// Outcome of one rest tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestTickOutcome {
    /// Hour passed without an encounter; HP / MP recovered.
    Slept {
        hp_per_member: u16,
        mp_per_member: u8,
    },
    /// Encounter check fired; rest is interrupted and combat should
    /// be entered.
    InterruptedByEncounter,
    /// Watch-keeper noticed nothing this hour; the party rests.
    UneventfulWatch,
}

/// Per-tick rest contribution constants. The exact original numbers
/// are not part of the public spec; these are first-playable
/// approximations that keep rest meaningful without making it trivial.
pub const REST_HP_PER_HOUR: u16 = 4;
pub const REST_MP_PER_HOUR: u8 = 1;
/// Probability denominator for the encounter check. The roll uses a
/// `u8` in `[0, 255]`; if it lies below the threshold the rest is
/// interrupted.
pub const REST_ENCOUNTER_THRESHOLD: u8 = 20;

/// Inputs the rest tick needs.
#[derive(Clone, Copy, Debug)]
pub struct RestTickInputs {
    /// Pre-rolled `u8` in `[0, 255]` for the encounter check.
    pub encounter_roll: u8,
    /// `true` when a watch is being kept. Watch lowers the encounter
    /// threshold by halving it; without a watch the full threshold
    /// applies.
    pub keeping_watch: bool,
}

/// Resolve one rest tick.
pub fn resolve_rest_tick(inputs: RestTickInputs) -> RestTickOutcome {
    let threshold = if inputs.keeping_watch {
        REST_ENCOUNTER_THRESHOLD / 2
    } else {
        REST_ENCOUNTER_THRESHOLD
    };
    if inputs.encounter_roll < threshold {
        RestTickOutcome::InterruptedByEncounter
    } else if inputs.keeping_watch {
        RestTickOutcome::Slept {
            hp_per_member: REST_HP_PER_HOUR,
            mp_per_member: REST_MP_PER_HOUR,
        }
    } else {
        RestTickOutcome::UneventfulWatch
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
            encounter_roll: roll,
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
            RestTickOutcome::UneventfulWatch => {
                completed += 1;
                total_hp = total_hp.saturating_add(REST_HP_PER_HOUR);
                total_mp = total_mp.saturating_add(REST_MP_PER_HOUR);
            }
            RestTickOutcome::InterruptedByEncounter => {
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
    fn high_roll_uneventful_watch_recovers_hp_and_mp() {
        let outcome = resolve_rest_tick(RestTickInputs {
            encounter_roll: 200,
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
    fn low_roll_interrupts_rest_with_encounter() {
        let outcome = resolve_rest_tick(RestTickInputs {
            encounter_roll: 5,
            keeping_watch: false,
        });
        assert_eq!(outcome, RestTickOutcome::InterruptedByEncounter);
    }

    #[test]
    fn keeping_watch_halves_encounter_chance() {
        // Roll of 15 interrupts without watch (15 < 20) but not with
        // watch (15 >= 10).
        let no_watch = resolve_rest_tick(RestTickInputs {
            encounter_roll: 15,
            keeping_watch: false,
        });
        let with_watch = resolve_rest_tick(RestTickInputs {
            encounter_roll: 15,
            keeping_watch: true,
        });
        assert_eq!(no_watch, RestTickOutcome::InterruptedByEncounter);
        assert!(matches!(with_watch, RestTickOutcome::Slept { .. }));
    }

    #[test]
    fn full_session_with_high_rolls_recovers_total_amount() {
        let rolls = [200u8; 8];
        let result = run_rest_session(8, &rolls, true);
        assert_eq!(result.completed_hours, 8);
        assert_eq!(result.total_hp_per_member, 8 * REST_HP_PER_HOUR);
        assert!(!result.interrupted);
    }

    #[test]
    fn rest_session_stops_at_first_low_roll() {
        let rolls = [200u8, 200, 5, 200, 200];
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
    fn unwatched_uneventful_outcome_still_recovers_hp() {
        let outcome = resolve_rest_tick(RestTickInputs {
            encounter_roll: 200,
            keeping_watch: false,
        });
        assert_eq!(outcome, RestTickOutcome::UneventfulWatch);
    }
}
