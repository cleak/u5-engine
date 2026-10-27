// The overworld input helper's entry world step (`timing.md §8.2`: "On the
// overworld the input helper performs one world step when it is entered";
// `stats-panel.md §2.3`: "after that helper's world-tick call and before the
// prompt newline and the key read").
mod input_helper_entry {
    use super::*;

    /// A seed whose first wind roll is non-zero, so one world step on an
    /// object-free map is exactly one gameplay draw.
    fn quiet_wind_seed() -> u16 {
        (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 63) != 0
            })
            .expect("a quiet seed exists")
    }

    fn world_at_prompt() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 10);
        state.active_objects.truncate(1);
        state.prng_state = quiet_wind_seed();
        state
    }

    #[test]
    fn the_entry_step_is_one_unpaced_world_step_with_its_wind_draw() {
        let mut state = world_at_prompt();
        let seed = state.prng_state;
        state.overworld_input_helper_entry_owed = true;

        assert!(state.run_overworld_input_helper_entry_step());

        assert!(
            !state.overworld_input_helper_entry_owed,
            "taken once per entry"
        );
        assert_eq!(
            state.prng_state,
            u5_prng_advance_state(seed),
            "one wind roll"
        );
        assert!(
            !state.run_overworld_input_helper_entry_step(),
            "nothing is owed until the helper is entered again"
        );
        assert_eq!(state.prng_state, u5_prng_advance_state(seed));
    }

    /// Journey Onward: the load owes the first prompt's entry step, and the
    /// session-start freeze (`animation.md §13.2`) makes that first world
    /// step a bare repaint, so the command wait's first timer pass is the
    /// first to draw. Before this the pump's first firing was the frozen
    /// one, and every pass after it ran one draw behind.
    #[test]
    fn the_session_start_freeze_falls_on_the_first_entry_step() {
        let mut state = world_at_prompt();
        let seed = state.prng_state;
        state.session_start_world_step_frozen = true;
        state.overworld_input_helper_entry_owed = true;

        assert!(state.run_overworld_input_helper_entry_step());
        assert!(!state.session_start_world_step_frozen);
        assert_eq!(state.prng_state, seed, "the frozen step draws nothing");

        state.idle_wait_pass(None).expect("idle pass");
        assert_eq!(
            state.prng_state,
            u5_prng_advance_state(seed),
            "first timer pass draws"
        );
    }

    #[test]
    fn a_dispatched_key_owes_the_next_entry_step() {
        let mut state = world_at_prompt();
        assert!(!state.overworld_input_helper_entry_owed);

        handle_play_key_input(&mut state, '~', "", &debug_game_dir()).expect("unmapped key");

        assert!(state.overworld_input_helper_entry_owed);
    }

    /// An open prompt means the command has not returned to the loop yet:
    /// no entry step, and nothing left owed for when the prompt closes - the
    /// key that closes it owes the step again.
    #[test]
    fn an_open_prompt_drops_the_entry_step_without_a_draw() {
        let mut state = world_at_prompt();
        let seed = state.prng_state;
        state.active_yes_no_prompt =
            Some(YesNoPromptSession::new(YesNoPromptKind::TrollBridgeToll {
                toll: 54,
            }));
        state.overworld_input_helper_entry_owed = true;

        assert!(!state.run_overworld_input_helper_entry_step());

        assert!(!state.overworld_input_helper_entry_owed);
        assert_eq!(state.prng_state, seed);
    }

    /// The published entry step is the overworld helper's; a town prompt is
    /// left as it was.
    #[test]
    fn a_town_prompt_takes_no_entry_step() {
        let mut state = test_state(open_grid(), 5, 5);
        state.prng_state = quiet_wind_seed();
        let seed = state.prng_state;
        state.overworld_input_helper_entry_owed = true;

        assert!(!state.run_overworld_input_helper_entry_step());

        assert!(!state.overworld_input_helper_entry_owed);
        assert_eq!(state.prng_state, seed);
    }
}
