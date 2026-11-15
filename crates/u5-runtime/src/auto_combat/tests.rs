//! The auto-combat key table and settings defaults (design 6.2.8, 6.3.2,
//! 6.3.4, 6.3.5; build plan M7-01).

use super::*;
use crate::input_codes::{InputByteClass, input_byte_class, input_code_direction};

const DIRECTIONS: [InputDirection; 8] = [
    InputDirection::North,
    InputDirection::South,
    InputDirection::East,
    InputDirection::West,
    InputDirection::Northwest,
    InputDirection::Northeast,
    InputDirection::Southwest,
    InputDirection::Southeast,
];

/// Every key the planner can emit.
fn every_key() -> Vec<AutoKey> {
    let mut keys = vec![
        AutoKey::Attack,
        AutoKey::Pass,
        AutoKey::Confirm,
        AutoKey::Cancel,
    ];
    keys.extend(
        DIRECTIONS
            .iter()
            .filter(|direction| direction.is_cardinal())
            .map(|direction| AutoKey::Step(*direction)),
    );
    keys.extend(
        DIRECTIONS
            .iter()
            .map(|direction| AutoKey::Cursor(*direction)),
    );
    keys
}

#[test]
fn the_input_char_table_matches_the_design() {
    // 6.3.2 and 6.3.4: 'A', ' ', '\r', 0x1B, and the input layer's
    // direction codes.
    assert_eq!(AutoKey::Attack.input_char(), 'A');
    assert_eq!(AutoKey::Pass.input_char(), ' ');
    assert_eq!(AutoKey::Confirm.input_char(), '\r');
    assert_eq!(AutoKey::Cancel.input_char(), '\u{1b}');
    let expected = [
        (InputDirection::West, 0x01),
        (InputDirection::East, 0x02),
        (InputDirection::North, 0x03),
        (InputDirection::South, 0x04),
        (InputDirection::Northwest, 0xD3),
        (InputDirection::Southwest, 0xD4),
        (InputDirection::Northeast, 0xD5),
        (InputDirection::Southeast, 0xD6),
    ];
    for (direction, byte) in expected {
        assert_eq!(AutoKey::Cursor(direction).input_char(), char::from(byte));
        if direction.is_cardinal() {
            assert_eq!(AutoKey::Step(direction).input_char(), char::from(byte));
            assert!((0x01..=0x04).contains(&byte));
        } else {
            assert!((0xD3..=0xD6).contains(&byte));
        }
    }
}

#[test]
fn every_key_is_a_byte_the_input_layer_accepts() {
    for key in every_key() {
        let byte = u8::try_from(u32::from(key.input_char())).expect("one byte");
        assert_ne!(input_byte_class(byte), InputByteClass::None, "{key:?}");
        assert_ne!(
            input_byte_class(byte),
            InputByteClass::FunctionKey,
            "{key:?}"
        );
        // 6.6 (h): never a digit and never a Control chord.
        assert!(!key.input_char().is_ascii_digit(), "{key:?}");
        match key {
            AutoKey::Step(direction) | AutoKey::Cursor(direction) => {
                assert_eq!(input_code_direction(byte), Some(direction));
            }
            _ => assert_eq!(input_code_direction(byte), None, "{key:?}"),
        }
    }
}

#[test]
fn steps_are_cardinal_only() {
    for direction in DIRECTIONS {
        assert_eq!(
            AutoKey::Step(direction).is_well_formed(),
            direction.is_cardinal()
        );
        assert!(AutoKey::Cursor(direction).is_well_formed());
    }
}

#[test]
fn the_script_tokens_match_the_design() {
    // 6.3.5: forms 1.0.1 already parses. u5-tui checks each against its
    // script reader.
    assert_eq!(AutoKey::Attack.script_token(), "A");
    assert_eq!(AutoKey::Pass.script_token(), "pass");
    assert_eq!(AutoKey::Confirm.script_token(), "enter");
    assert_eq!(AutoKey::Cancel.script_token(), "\x1b");
    for key in every_key() {
        let token = key.script_token();
        assert!(!token.is_empty());
        assert_eq!(token.trim(), token, "{key:?}: the reader trims tokens");
    }
    // A step and a cursor move the same way are the same physical key;
    // every other pair of keys has its own token.
    for direction in DIRECTIONS
        .iter()
        .filter(|direction| direction.is_cardinal())
    {
        assert_eq!(
            AutoKey::Step(*direction).script_token(),
            AutoKey::Cursor(*direction).script_token()
        );
    }
    let distinct: std::collections::HashSet<&str> = every_key()
        .into_iter()
        .filter(|key| !matches!(key, AutoKey::Step(_)))
        .map(AutoKey::script_token)
        .collect();
    assert_eq!(distinct.len(), 4 + DIRECTIONS.len());
}

#[test]
fn the_defaults_are_the_faithful_game() {
    let settings = CombatSettings::default();
    assert!(!settings.auto && !settings.allies && !settings.fast_forward);
    assert_eq!(settings.tactic, AutoTactic::Fight);
    assert_eq!(settings.targeting, AutoTargeting::Nearest);
    assert_eq!(settings.interrupt, InterruptMode::Swallow);
    assert_eq!(settings.step_ms, 150);
    assert_eq!(settings.pause_below_hp_percent, 25);
    assert_eq!(settings.action_cap, 200);
    assert_eq!(settings.victory, VictoryAction::Stop);
    assert!(!settings.keep_between_fights);
    assert!(settings.pause_on_focus_loss);
    assert!(!settings.potions);
    assert_eq!(settings.mp_reserve, MpReserve::OneHeal);
    assert!(settings.spell_allow_list.is_empty());
    assert!((STEP_MS_MIN..=STEP_MS_MAX).contains(&settings.step_ms));
    assert!((ACTION_CAP_MIN..=ACTION_CAP_MAX).contains(&settings.action_cap));
    assert!(settings.pause_below_hp_percent <= PAUSE_BELOW_HP_MAX);

    assert_eq!(AutoEngagement::default().members, [false; 6]);
    assert!(!AutoEngagement::default().any());
    assert!(!AutoEngagement::default().until_done);
    let fight = AutoFightRuntime::default();
    assert_eq!(
        (fight.actions, fight.no_progress, fight.blocked_dirs),
        (0, 0, 0)
    );
    assert_eq!(fight.stopping, None);
    assert!(!fight.owns_open_attempt);
}
