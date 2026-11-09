//! The trajectory-equivalence harness, terminal layer, on synthetic states
//! (Phase B design 1.3.12, package M0-18): `@` tokens, the on and off runs,
//! dummy features that are and are not equivalent, the matrix file and the
//! command line. The asset-backed suite runs on game-dev-server.

use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{
    CliSetting, EffectClass, Preset, ResolvedEnhancements, Toggle, trajectory_digest,
};
use u5_runtime::test_fixtures::{
    open_grid, saved_game_seed_bytes, test_state, write_empty_ool_mirrors,
};
use u5_runtime::{PlayState, SAVED_GAM_FILENAME};
use u5_tui::*;

/// Any catalogue toggle will do: the dummies stand in for real features.
fn some_toggle() -> Toggle {
    u5_runtime::enhancements::TOGGLES[0].toggle
}

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn script(commands: &[&str]) -> Vec<String> {
    commands.iter().map(|c| (*c).to_string()).collect()
}

struct Scratch(PathBuf);

impl Scratch {
    /// A folder holding a seed save, for the scripted `Q`, `Y`.
    fn with_save(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-equivalence-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x01, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&dir);
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A presentation feature: it reads the state on `@peek` and types nothing.
struct Peek {
    seen: Vec<u64>,
}

impl TerminalFeature for Peek {
    fn toggle(&self) -> Toggle {
        some_toggle()
    }
    fn class(&self) -> EffectClass {
        EffectClass::Presentation
    }
    fn owns_token(&self, token: &str) -> bool {
        token == "@peek"
    }
    fn on_token(&mut self, _token: &str, state: &PlayState) -> Vec<String> {
        self.seen.push(state.turn);
        Vec::new()
    }
}

/// An input-driven feature: `@auto` types two passes, as a player could.
struct AutoPass;

impl TerminalFeature for AutoPass {
    fn toggle(&self) -> Toggle {
        some_toggle()
    }
    fn class(&self) -> EffectClass {
        EffectClass::InputDriven
    }
    fn owns_token(&self, token: &str) -> bool {
        token == "@auto"
    }
    fn on_token(&mut self, _token: &str, _state: &PlayState) -> Vec<String> {
        vec!["pass".into(), "pass".into()]
    }
}

/// The deliberate offender: it claims to be presentation but acts on the
/// game by typing a key on `@peek`.
struct SneakyPeek;

impl TerminalFeature for SneakyPeek {
    fn toggle(&self) -> Toggle {
        some_toggle()
    }
    fn class(&self) -> EffectClass {
        EffectClass::Presentation
    }
    fn owns_token(&self, token: &str) -> bool {
        token == "@peek"
    }
    fn on_token(&mut self, _token: &str, _state: &PlayState) -> Vec<String> {
        vec!["pass".into()]
    }
}

/// The other offender: hooks that write the state directly when they take
/// a token.
struct StateWriter;

impl ScriptTokenHooks for StateWriter {
    fn intercept(
        &mut self,
        _index: usize,
        _token: &str,
        state: &mut PlayState,
    ) -> ScriptTokenIntercept {
        state.gold = state.gold.wrapping_add(1);
        ScriptTokenIntercept::Consumed
    }
}

const SCRIPT: [&str; 6] = ["pass", "@peek", "idle:2", "@auto", "pass", "@peek"];

#[test]
fn classic_turns_no_terminal_feature_on() {
    assert!(terminal_features(&ResolvedEnhancements::classic()).is_empty());
}

#[test]
fn classic_equals_all_off_with_the_save_compared() {
    let (off_dir, on_dir) = (Scratch::with_save("off"), Scratch::with_save("on"));
    let commands = script(&["pass", "idle:2", "pass"]);
    let mismatches = check_equivalence(
        &town(),
        Path::new(""),
        &commands,
        terminal_features(&ResolvedEnhancements::classic()),
        Some((off_dir.path(), on_dir.path())),
    );
    assert_eq!(mismatches, []);
    // The scripted `Q`, `Y` really saved, so the save bytes were compared.
    let record = run_trace(
        town(),
        Path::new(""),
        &commands,
        &mut NoScriptTokenHooks,
        Some(on_dir.path()),
    );
    assert_eq!(record.error, None);
    let seed = u5_runtime::enhancements::fnv1a64(&saved_game_seed_bytes(0x01, 0, 5, 5));
    let saved = record
        .saves
        .iter()
        .find(|(name, _)| *name == SAVED_GAM_FILENAME)
        .and_then(|(_, hash)| *hash);
    assert!(saved.is_some_and(|hash| hash != seed), "{:?}", record.saves);
}

#[test]
fn equivalent_dummy_features_pass() {
    let (off_dir, on_dir) = (Scratch::with_save("off"), Scratch::with_save("on"));
    let mismatches = check_equivalence(
        &town(),
        Path::new(""),
        &script(&SCRIPT),
        vec![Box::new(Peek { seen: Vec::new() }), Box::new(AutoPass)],
        Some((off_dir.path(), on_dir.path())),
    );
    assert_eq!(mismatches, []);
}

#[test]
fn a_presentation_feature_that_types_a_key_is_caught() {
    let mismatches = check_equivalence(
        &town(),
        Path::new(""),
        &script(&SCRIPT),
        vec![Box::new(SneakyPeek), Box::new(AutoPass)],
        None,
    );
    assert!(!mismatches.is_empty());
    assert!(mismatches.iter().any(|m| m.what == "final digest"));
}

#[test]
fn a_state_writing_feature_is_caught() {
    let commands = script(&SCRIPT);
    let on = run_trace(town(), Path::new(""), &commands, &mut StateWriter, None);
    // Its tokens are removed from the off run, as a class (a) feature's.
    let events: Vec<TokenEvent> = [1, 3, 5]
        .into_iter()
        .map(|index| TokenEvent {
            index,
            token: commands[index].clone(),
            class: EffectClass::Presentation,
            emitted: Vec::new(),
        })
        .collect();
    let off = run_trace(
        town(),
        Path::new(""),
        &commands,
        &mut ReplayHooks::new(&events),
        None,
    );
    let mismatches = compare_runs(&off, &on);
    assert!(
        mismatches.iter().any(|m| m.what.starts_with("step ")),
        "{mismatches:?}"
    );
    assert!(mismatches.iter().any(|m| m.what == "final digest"));
}

#[test]
fn a_taken_token_runs_nothing_and_reports_no_step() {
    let mut hooks = FeatureHooks::new(vec![Box::new(Peek { seen: Vec::new() })]);
    let record = run_trace(
        town(),
        Path::new(""),
        &script(&["@peek", "pass", "@peek"]),
        &mut hooks,
        None,
    );
    assert_eq!(record.steps.len(), 1);
    assert_eq!(record.steps[0].index, 1);
    // Not even the exploration-gate settle or the input-helper entry step:
    // a script of tokens alone leaves the state as it was.
    let tokens_only = run_trace(
        town(),
        Path::new(""),
        &script(&["@peek", "@peek"]),
        &mut FeatureHooks::new(vec![Box::new(Peek { seen: Vec::new() })]),
        None,
    );
    assert!(tokens_only.steps.is_empty());
    assert_eq!(tokens_only.final_digest, Some(trajectory_digest(&town())));
    assert_eq!(hooks.events().len(), 2);
}

#[test]
fn emitted_commands_report_the_token_index() {
    let mut hooks = FeatureHooks::new(vec![Box::new(AutoPass)]);
    let record = run_trace(
        town(),
        Path::new(""),
        &script(&["pass", "@auto"]),
        &mut hooks,
        None,
    );
    let steps: Vec<(usize, &str)> = record
        .steps
        .iter()
        .map(|step| (step.index, step.command.as_str()))
        .collect();
    assert_eq!(steps, [(0, "pass"), (1, "pass"), (1, "pass")]);
    assert_eq!(
        hooks.events(),
        [TokenEvent {
            index: 1,
            token: "@auto".into(),
            class: EffectClass::InputDriven,
            emitted: vec!["pass".into(), "pass".into()],
        }]
    );
}

/// With its feature off an `@` token is not special: it is parsed exactly
/// as 1.0.1 parses it.
#[test]
fn an_unowned_token_runs_as_the_plain_script_runs_it() {
    for token in ["@journal", "@auto", "@"] {
        let commands = script(&["pass", token, "pass"]);
        let plain = {
            let mut state = town();
            replay_play_script_commands(&mut state, Path::new(""), &commands, |_, _, _| Ok(()))
                .unwrap();
            trajectory_digest(&state)
        };
        for hooks in [
            &mut NoScriptTokenHooks as &mut dyn ScriptTokenHooks,
            &mut FeatureHooks::new(vec![Box::new(Peek { seen: Vec::new() })]),
            &mut ReplayHooks::default(),
        ] {
            let record = run_trace(town(), Path::new(""), &commands, hooks, None);
            assert_eq!(record.final_digest, Some(plain), "{token}");
            assert_eq!(record.steps.len(), 3, "{token}");
        }
    }
}

#[test]
fn only_at_tokens_are_offered_to_the_hooks() {
    struct Greedy(Vec<String>);
    impl ScriptTokenHooks for Greedy {
        fn intercept(
            &mut self,
            _index: usize,
            token: &str,
            _state: &mut PlayState,
        ) -> ScriptTokenIntercept {
            self.0.push(token.to_string());
            ScriptTokenIntercept::PassThrough
        }
    }
    let mut hooks = Greedy(Vec::new());
    run_trace(
        town(),
        Path::new(""),
        &script(&["pass", " @peek", "a@b", "idle:1"]),
        &mut hooks,
        None,
    );
    assert_eq!(hooks.0, [" @peek"]);
    assert!(is_enhancement_token("@journal"));
    assert!(!is_enhancement_token("journal@"));
    assert_eq!(ENHANCEMENT_TOKEN_PREFIX, '@');
}

#[test]
fn the_off_run_removes_only_the_tokens_the_on_run_took() {
    // `@auto` was taken (class (b), two passes); `@other` was not.
    let events = [TokenEvent {
        index: 0,
        token: "@auto".into(),
        class: EffectClass::InputDriven,
        emitted: vec!["pass".into(), "pass".into()],
    }];
    let commands = script(&["@auto", "@other"]);
    let off = run_trace(
        town(),
        Path::new(""),
        &commands,
        &mut ReplayHooks::new(&events),
        None,
    );
    let steps: Vec<(usize, &str)> = off
        .steps
        .iter()
        .map(|step| (step.index, step.command.as_str()))
        .collect();
    assert_eq!(steps, [(0, "pass"), (0, "pass"), (1, "@other")]);
}

#[test]
fn the_matrix_file_parses_rows_and_refuses_mistakes() {
    let key = u5_runtime::enhancements::TOGGLES[0].key;
    let text = format!("# name\tsettings\n\nall-on\tpreset=everything\nalone\t{key}=true\nempty\n");
    let rows = parse_matrix(&text).unwrap();
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["all-on", "alone", "empty"]);
    assert_eq!(rows[0].settings, [CliSetting::Preset(Preset::Everything)]);
    assert_eq!(
        rows[1].settings,
        [CliSetting::Set {
            key: key.to_string(),
            value: "true".to_string(),
        }]
    );
    assert!(rows[2].settings.is_empty());

    for bad in [
        "row\tno.such_key=true",
        "row\tpreset=nonsense",
        "row\tjust-a-word",
        "classic\tpreset=classic",
        "twice\t\ntwice\t",
    ] {
        assert!(parse_matrix(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_suite_flags_parse_and_refuse_other_modes() {
    let args = parse_cli_args([
        "--equivalence-suite",
        "out",
        "--matrix",
        "matrix.tsv",
        "game",
    ])
    .unwrap();
    assert_eq!(args.equivalence_suite, Some(PathBuf::from("out")));
    assert_eq!(args.equivalence_matrix, Some(PathBuf::from("matrix.tsv")));
    assert!(args.has_mode());
    assert!(args.is_qa_mode());

    for bad in [
        &["--matrix", "m.tsv", "game"][..],
        &["--equivalence-suite"][..],
        &["--equivalence-suite", "out", "--route-smoke", "game"][..],
        &[
            "--equivalence-suite",
            "out",
            "--play-script",
            "pass",
            "game",
        ][..],
        &[
            "--equivalence-suite",
            "out",
            "--preset",
            "everything",
            "game",
        ][..],
        &[
            "--equivalence-suite",
            "a",
            "--equivalence-suite",
            "b",
            "game",
        ][..],
    ] {
        assert!(parse_cli_args(bad.iter().copied()).is_err(), "{bad:?}");
    }
    assert!(cli_usage().contains("--equivalence-suite"));
}

/// Classic against 1.0.1 on every route-smoke case, with real game files.
/// Runs only when `U5_TEST_ASSET_DIR` names a writable copy of them.
#[test]
fn classic_equals_the_plain_route_smoke_on_real_files() {
    let Some(game_dir) = u5_runtime::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    if !game_dir.join("CASTLE.DAT").exists() {
        return;
    }
    let atlas =
        u5_runtime::load_tile_atlas(&game_dir, u5_runtime::TileGraphicsDepth::Ega16).unwrap();
    let case = route_smoke_cases()
        .into_iter()
        .find(|case| case.name == "castle-pass")
        .or_else(|| route_smoke_cases().into_iter().next())
        .unwrap();
    let plain = run_route_smoke_case(&game_dir, &atlas, &case);
    let traced = run_route_smoke_case_traced(
        &game_dir,
        &atlas,
        &case,
        &mut FeatureHooks::new(terminal_features(&ResolvedEnhancements::classic())),
        &mut NoRouteTrace,
    );
    match (plain, traced) {
        (Ok(plain), Ok(traced)) => assert_eq!(plain, traced),
        (Err(plain), Err(traced)) => assert_eq!(plain.to_string(), traced.to_string()),
        (plain, traced) => panic!("{plain:?} / {traced:?}"),
    }
}

/// One edit to a run record.
type Change = fn(&mut RunRecord);

#[test]
fn every_compared_part_can_differ() {
    let base = run_trace(
        town(),
        Path::new(""),
        &script(&["pass"]),
        &mut NoScriptTokenHooks,
        None,
    );
    assert_eq!(compare_runs(&base, &base.clone()), []);
    let changes: [(&str, Change); 6] = [
        ("final state line", |r| r.final_state_line.push('x')),
        ("final frame", |r| r.final_frame_hash = Some(1)),
        ("scripted save", |r| r.save_error = Some("full".into())),
        ("error", |r| r.error = Some("failed".into())),
        ("commands run", |r| r.steps.clear()),
        ("final digest", |r| {
            r.final_digest = None;
        }),
    ];
    for (what, change) in changes {
        let mut other = base.clone();
        change(&mut other);
        let mismatches = compare_runs(&base, &other);
        assert!(
            mismatches.iter().any(|m| m.what == what),
            "{what}: {mismatches:?}"
        );
    }
    let mut saved = base.clone();
    saved.saves = vec![(SAVED_GAM_FILENAME, Some(1))];
    let mut other = saved.clone();
    other.saves = vec![(SAVED_GAM_FILENAME, Some(2))];
    assert!(
        compare_runs(&saved, &other)
            .iter()
            .any(|m| m.what == "saved SAVED.GAM")
    );
}
