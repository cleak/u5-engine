//! The `@auto` control line, the log forms and the off and lock paths
//! (design 6.6.2; build plan M7-04). The fights themselves run on the
//! player's files in `tests/auto_combat.rs`.

use super::*;
use u5_runtime::enhancements::{LockReason, Toggle, resolve_as_shipped};
use u5_runtime::test_fixtures::{open_grid, test_state};

fn on() -> CombatSettings {
    CombatSettings {
        auto: true,
        ..CombatSettings::default()
    }
}

/// `combat.auto` through the resolver, as if its phase had shipped.
fn resolved_auto(lock: LockState) -> CombatSettings {
    let cli = [CliSetting::Enable("combat.auto".to_string())];
    resolve_as_shipped(
        ResolveInput {
            stored: None,
            cli: &cli,
            lock,
            caps: HostCapabilities::ALL,
        },
        &[Toggle::CombatAuto.spec().ships],
    )
    .combat()
}

#[test]
fn the_control_line_parses_every_form() {
    let members = |list: &[usize]| {
        let mut members = [false; ROSTER_POSITIONS];
        for position in list {
            members[position - 1] = true;
        }
        Some(AutoControl::Members(members))
    };
    assert_eq!(parse_auto_control("@auto"), Some(AutoControl::Prompted));
    assert_eq!(
        parse_auto_control("  @auto \n"),
        Some(AutoControl::Prompted)
    );
    assert_eq!(parse_auto_control("@auto party"), Some(AutoControl::Party));
    assert_eq!(parse_auto_control("@auto until"), Some(AutoControl::Until));
    assert_eq!(parse_auto_control("@auto off"), Some(AutoControl::Off));
    assert_eq!(parse_auto_control("@auto 1,3"), members(&[1, 3]));
    assert_eq!(parse_auto_control("@auto 2 6"), members(&[2, 6]));
    assert_eq!(parse_auto_control("@auto 0"), Some(AutoControl::Unknown));
    assert_eq!(parse_auto_control("@auto 7"), Some(AutoControl::Unknown));
    assert_eq!(parse_auto_control("@auto x"), Some(AutoControl::Unknown));
    // Not the control line: another feature's token, or the `@` key.
    assert_eq!(parse_auto_control("@automap-open"), None);
    assert_eq!(parse_auto_control("@gemmap"), None);
    assert_eq!(parse_auto_control("@"), None);
    assert_eq!(parse_auto_control("A"), None);
}

#[test]
fn off_or_locked_there_is_no_auto_and_no_log_file() {
    let dir = std::env::temp_dir().join(format!("u5-auto-log-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auto.log");
    let _ = std::fs::remove_file(&path);
    // The parser's default and the feature off.
    assert!(!CombatSettings::default().auto);
    assert!(TerminalAutoCombat::new(CombatSettings::default()).is_none());
    assert!(AutoCombatHooks::new(CombatSettings::default()).is_none());
    // Under the lock `--auto-combat` is ignored.
    let locked = resolved_auto(LockState::Locked(LockReason::CliFlag));
    assert!(!locked.auto);
    assert!(
        TerminalAutoCombat::with_log(locked, Some(&path), AutoLogForm::Script)
            .unwrap()
            .is_none()
    );
    assert!(!path.exists(), "the lock creates no log");
    // Unlocked it arms, and the log is created.
    let unlocked = resolved_auto(LockState::Off);
    assert!(unlocked.auto);
    assert!(
        TerminalAutoCombat::with_log(unlocked, Some(&path), AutoLogForm::Script)
            .unwrap()
            .is_some()
    );
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn until_it_ships_the_terminal_settings_leave_auto_off() {
    let cli = [CliSetting::Enable("combat.auto".to_string())];
    assert!(!terminal_combat_settings(&cli, LockState::Off).auto);
}

#[test]
fn every_auto_key_has_a_stdin_line_that_types_it() {
    use u5_runtime::InputDirection;
    let keys = [
        AutoKey::Attack,
        AutoKey::Pass,
        AutoKey::Confirm,
        AutoKey::Cancel,
        AutoKey::Step(InputDirection::North),
        AutoKey::Cursor(InputDirection::Southeast),
        AutoKey::Cursor(InputDirection::Northwest),
    ];
    for key in keys {
        let line = format!("{}\n", auto_key_stdin_line(key));
        let typed = crate::play_loop::play_input_key_and_suffix(&line);
        assert_eq!(typed, Some((key.input_char(), String::new())), "{key:?}");
    }
}

#[test]
fn outside_combat_the_line_is_taken_and_engages_nobody() {
    let state = test_state(open_grid(), 1, 1);
    let mut auto = TerminalAutoCombat::new(on()).unwrap();
    assert_eq!(
        auto.control("@auto party", &state),
        "Auto-combat: not in combat"
    );
    assert!(!auto.engagement().any());
    assert_eq!(auto.next_key(&state), (None, None));
    assert_eq!(auto.control("@auto off", &state), "Auto off");
}

#[test]
fn engagement_follows_the_line_and_clears_when_the_fight_ends() {
    let mut state = test_state(open_grid(), 1, 1);
    state.combat_active = true;
    let mut auto = TerminalAutoCombat::new(on()).unwrap();
    assert_eq!(auto.control("@auto 1,3", &state), "Auto: member 1,3");
    assert_eq!(
        auto.engagement().members,
        [true, false, true, false, false, false]
    );
    assert_eq!(auto.control("@auto until", &state), "Auto: until done");
    assert!(auto.engagement().until_done);
    state.combat_active = false;
    let (key, note) = auto.next_key(&state);
    assert_eq!(key, None);
    assert_eq!(note.as_deref(), Some("Auto off"));
    assert_eq!(auto.last_hand_back(), Some(HandBack::FightOver));
    assert!(!auto.engagement().any());
}

#[test]
fn replay_script_puts_each_tokens_keys_in_its_place() {
    let mut hooks = AutoCombatHooks::new(on()).unwrap();
    hooks.typed = vec![(1, "A".into()), (1, "enter".into()), (3, "pass".into())];
    let commands: Vec<String> = ["pass", "@auto party", "x", "@auto", "@gemmap"]
        .iter()
        .map(|c| c.to_string())
        .collect();
    assert_eq!(
        hooks.replay_script(&commands),
        ["pass", "A", "enter", "x", "pass", "@gemmap"]
    );
}

/// Phase 4d (design 6.3.4; plan M7-07): the terminal has no cues, so
/// `combat.fast_forward` changes nothing here. The shell never reads it,
/// and the same lines give the same notes and keys with it on.
#[test]
fn fast_forward_leaves_the_terminal_unchanged() {
    for source in [
        include_str!("../auto_combat.rs"),
        include_str!("../main.rs"),
        include_str!("../lib.rs"),
        include_str!("../play_loop.rs"),
    ] {
        assert!(!source.contains("fast_forward"));
    }
    let run = |fast_forward: bool| {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        let mut auto = TerminalAutoCombat::new(CombatSettings {
            fast_forward,
            ..on()
        })
        .unwrap();
        let mut seen = vec![auto.control("@auto until", &state)];
        for _ in 0..3 {
            seen.push(format!("{:?}", auto.next_key(&state)));
        }
        state.combat_active = false;
        seen.push(format!("{:?}", auto.next_key(&state)));
        (seen, auto.keys_typed(), auto.last_hand_back())
    };
    assert_eq!(run(true), run(false));
}
