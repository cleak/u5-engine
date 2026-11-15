//! Auto-combat's terminal half on the player's files (Phase B design
//! 6.6.2; build plan M7-04).
//!
//! - Two fixed-clock runs of an `@auto` script are byte-identical: the
//!   transcript, the log and `SAVED.GAM` after the fight's save.
//! - An `@auto` run equals a feature-off replay of its `--auto-combat-log`:
//!   the transcript without the shell's own lines, the state digest and
//!   PRNG after every command, and `SAVED.GAM`. The same holds for an
//!   interactive run fed `@auto` on stdin, which shows the control line
//!   costs no extra wait pass.
//! - Whole-party auto ends each fixture's fight in victory or a hand-back
//!   within the caps.
//!
//! The fixed-clock runs are child processes of this test binary, so the
//! clock variable is set on the child, never on this process. (The engine
//! binary itself treats `U5_FIXED_HOST_CLOCK` as a QA run and locks every
//! enhancement, so the children pass the settings explicitly.) Each test
//! skips without `U5_TEST_ASSET_DIR`.

use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::process::Command;

use u5_runtime::auto_combat::{CombatSettings, HandBack};
use u5_runtime::enhancements::trajectory_digest;
use u5_runtime::test_fixtures::configured_original_asset_dir;
use u5_runtime::{
    ActiveObject, DiskPromptSession, PlayOptions, PlayState, PlayTarget, SAVED_GAM_FILENAME,
    STEADY_PHASE, WorldPlane,
};
use u5_tui::*;

const CHILD_MODE: &str = "U5_AUTO_COMBAT_CHILD";
const CHILD_DIR: &str = "U5_AUTO_COMBAT_DIR";
const CHILD_FIXTURE: &str = "U5_AUTO_COMBAT_FIXTURE";
const CHILD_LOG: &str = "U5_AUTO_COMBAT_LOG";
const FIXED_CLOCK: &str = "10:20:30.40";
const BEGIN: &str = "@@begin";
const END: &str = "@@end";

#[derive(Clone, Copy, Debug)]
enum Fixture {
    /// A world-object fight on open Britannia ground.
    Terrain,
    /// The same world-object fight on Underworld ground.
    Underworld,
}

impl Fixture {
    fn name(self) -> &'static str {
        match self {
            Fixture::Terrain => "terrain",
            Fixture::Underworld => "underworld",
        }
    }

    fn from_name(name: &str) -> Self {
        match name {
            "terrain" => Fixture::Terrain,
            "underworld" => Fixture::Underworld,
            other => panic!("unknown fixture {other}"),
        }
    }

    /// The fight under way, its first prompt open.
    fn state(self, game_dir: &Path) -> io::Result<PlayState> {
        match self {
            Fixture::Terrain | Fixture::Underworld => {
                let plane = match self {
                    Fixture::Underworld => WorldPlane::Underworld,
                    _ => WorldPlane::Britannia,
                };
                let options = PlayOptions {
                    target: PlayTarget::World(plane),
                    ..PlayOptions::default()
                };
                let mut state = PlayState::load_scene(game_dir, options)?;
                let trigger = ActiveObject {
                    type_byte: 0x50,
                    tile: 0xc0,
                    x: state.player.x,
                    y: state.player.y,
                    z: plane.save_floor(),
                    phase: STEADY_PHASE,
                    aux1: 0,
                    aux3: 0,
                };
                state.enter_terrain_combat_from_world_object(game_dir, plane, 1, trigger)?;
                Ok(state)
            }
        }
    }
}

fn auto_on() -> CombatSettings {
    CombatSettings {
        auto: true,
        step_ms: 0,
        ..CombatSettings::default()
    }
}

/// A fresh writable copy of the player's files.
fn game_copy(source: &Path, label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("auto-combat")
        .join(format!("{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
        }
    }
    dir
}

/// Watches a run: the session (auto-combat, when on) and the digest after
/// every command.
struct DigestWatch {
    session: Option<TerminalSession>,
    digests: Vec<String>,
}

impl PlayObserver for DigestWatch {
    fn before_command(&mut self, state: &PlayState) {
        if let Some(session) = self.session.as_mut() {
            session.before_command(state);
        }
    }

    fn after_command(&mut self, state: &PlayState) {
        if let Some(session) = self.session.as_mut() {
            session.after_command(state);
        }
        self.digests.push(trajectory_digest(state).to_string());
    }

    fn take_token(&mut self, token: &str, state: &PlayState) -> bool {
        self.session
            .as_mut()
            .is_some_and(|session| session.take_token(token, state))
    }

    fn take_auto_line(&mut self, line: &str, state: &PlayState) -> bool {
        self.session
            .as_mut()
            .is_some_and(|session| session.take_auto_line(line, state))
    }

    fn auto_key(&mut self, state: &PlayState) -> Option<u5_runtime::auto_combat::AutoKey> {
        self.session.as_mut()?.auto_key(state)
    }

    fn typed_line(&mut self, line: &str) {
        if let Some(session) = self.session.as_mut() {
            session.typed_line(line);
        }
    }
}

/// The child's side: one run, printed between the markers, then the
/// digests, then the save.
#[test]
fn child_run() {
    let Ok(mode) = std::env::var(CHILD_MODE) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(CHILD_DIR).unwrap());
    let fixture = Fixture::from_name(&std::env::var(CHILD_FIXTURE).unwrap());
    let log = PathBuf::from(std::env::var(CHILD_LOG).unwrap());
    let mut state = fixture.state(&dir).unwrap();
    let (auto, form) = match mode.as_str() {
        "script-auto" => (true, AutoLogForm::Script),
        "stdin-auto" => (true, AutoLogForm::Stdin),
        _ => (false, AutoLogForm::Script),
    };
    let session = auto.then(|| {
        let auto = TerminalAutoCombat::with_log(auto_on(), Some(&log), form)
            .unwrap()
            .unwrap();
        TerminalSession::default().with_auto_combat(Some(auto))
    });
    let mut watch = DigestWatch {
        session,
        digests: Vec::new(),
    };
    println!("{BEGIN}");
    match mode.as_str() {
        "script-auto" => {
            let commands = split_play_script("@auto party;pass");
            run_play_script_commands_watched(&mut state, &dir, &commands, None, Some(&mut watch))
                .unwrap();
        }
        "script-replay" => {
            let commands = split_play_script(&fs::read_to_string(&log).unwrap());
            run_play_script_commands_watched(&mut state, &dir, &commands, None, Some(&mut watch))
                .unwrap();
        }
        "stdin-auto" => {
            let mut stdin = Cursor::new("@auto party\n \n".as_bytes());
            run_interactive_play_loop_watched(&mut state, &dir, None, &mut stdin, Some(&mut watch))
                .unwrap();
        }
        "stdin-replay" => {
            let lines = fs::read(&log).unwrap();
            let mut stdin = Cursor::new(lines);
            run_interactive_play_loop_watched(&mut state, &dir, None, &mut stdin, Some(&mut watch))
                .unwrap();
        }
        other => panic!("unknown mode {other}"),
    }
    println!("{END}");
    for digest in &watch.digests {
        println!("@@digest {digest}");
    }
    println!("@@final {}", trajectory_digest(&state));
    println!("@@state {}", play_script_state_line(&state));
    let mut disk = DiskPromptSession::single_directory();
    match state.write_save_files_with_disk_session(&dir, &mut disk) {
        Ok(()) => println!("@@saved"),
        Err(error) => println!("@@save-error {error}"),
    }
}

/// What a child printed and saved.
struct ChildRun {
    transcript: String,
    tail: Vec<String>,
    saved: Vec<u8>,
    log: Vec<u8>,
}

fn run_child(source: &Path, fixture: Fixture, mode: &str, label: &str, log: &Path) -> ChildRun {
    let dir = game_copy(source, label);
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_run", "--nocapture", "--test-threads=1"])
        .env(CHILD_MODE, mode)
        .env(CHILD_DIR, &dir)
        .env(CHILD_FIXTURE, fixture.name())
        .env(CHILD_LOG, log)
        .env(u5_runtime::FIXED_HOST_CLOCK_ENV, FIXED_CLOCK)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{label}: child failed\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let begin = stdout.find(BEGIN).expect("begin marker") + BEGIN.len() + 1;
    let end = stdout.find(END).expect("end marker");
    let tail = stdout[end + END.len()..]
        .lines()
        .skip(1)
        .take_while(|line| line.starts_with("@@"))
        .map(str::to_string)
        .collect();
    let run = ChildRun {
        transcript: stdout[begin..end].to_string(),
        tail,
        saved: fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap_or_default(),
        log: fs::read(log).unwrap_or_default(),
    };
    let _ = fs::remove_dir_all(&dir);
    run
}

fn log_path(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("auto-combat");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{label}-{}.log", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

/// The shell's own lines: the script labels and count, and auto's notes.
fn is_shell_line(line: &str) -> bool {
    line.starts_with("script[")
        || line.starts_with("Script mode:")
        || line.starts_with("Auto: ")
        || line.starts_with("Auto off")
        || line.starts_with("Auto-combat: ")
        || line == "Victory - auto off"
}

fn game_transcript(transcript: &str) -> String {
    transcript
        .lines()
        .filter(|line| !is_shell_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// An interactive transcript without the prompts and the auto key lines:
/// a replayed line is read where an auto key was printed.
fn interactive_transcript(transcript: &str) -> String {
    transcript
        .replace("> auto ", "\u{0}auto ")
        .replace("> ", "")
        .lines()
        .filter(|line| !line.starts_with("\u{0}auto ") && !is_shell_line(line))
        .collect::<Vec<_>>()
        .join("\n")
}

const FIXTURES: [Fixture; 2] = [Fixture::Terrain, Fixture::Underworld];

#[test]
fn two_fixed_clock_runs_are_byte_identical() {
    let Some(source) = configured_original_asset_dir() else {
        return;
    };
    for fixture in FIXTURES {
        let name = fixture.name();
        let (log_a, log_b) = (
            log_path(&format!("{name}-a")),
            log_path(&format!("{name}-b")),
        );
        let a = run_child(
            &source,
            fixture,
            "script-auto",
            &format!("{name}-a"),
            &log_a,
        );
        let b = run_child(
            &source,
            fixture,
            "script-auto",
            &format!("{name}-b"),
            &log_b,
        );
        assert!(
            a.transcript.contains("script[1.1]: auto "),
            "{name}: auto ran"
        );
        assert!(!a.log.is_empty(), "{name}: the log has the keys");
        assert_eq!(a.transcript, b.transcript, "{name}: transcripts");
        assert_eq!(a.tail, b.tail, "{name}: digests");
        assert_eq!(a.log, b.log, "{name}: logs");
        assert!(
            a.tail.iter().any(|line| line == "@@saved"),
            "{name}: saved {:?}",
            a.tail.last()
        );
        assert!(a.saved == b.saved, "{name}: SAVED.GAM bytes");
    }
}

#[test]
fn an_auto_script_run_equals_its_logs_feature_off_replay() {
    let Some(source) = configured_original_asset_dir() else {
        return;
    };
    for fixture in FIXTURES {
        let name = fixture.name();
        let log = log_path(&format!("{name}-script"));
        let on = run_child(&source, fixture, "script-auto", &format!("{name}-on"), &log);
        let off = run_child(
            &source,
            fixture,
            "script-replay",
            &format!("{name}-off"),
            &log,
        );
        assert!(
            !String::from_utf8_lossy(&on.log).contains("@auto"),
            "{name}: the log is a plain script"
        );
        assert_eq!(
            game_transcript(&on.transcript),
            game_transcript(&off.transcript),
            "{name}: transcripts"
        );
        assert_eq!(on.tail, off.tail, "{name}: digests, PRNG and state");
        assert!(on.saved == off.saved, "{name}: SAVED.GAM bytes");
    }
}

#[test]
fn an_interactive_auto_run_equals_its_logs_stdin_replay() {
    let Some(source) = configured_original_asset_dir() else {
        return;
    };
    for fixture in FIXTURES {
        let name = fixture.name();
        let log = log_path(&format!("{name}-stdin"));
        let on = run_child(
            &source,
            fixture,
            "stdin-auto",
            &format!("{name}-in-on"),
            &log,
        );
        let off = run_child(
            &source,
            fixture,
            "stdin-replay",
            &format!("{name}-in-off"),
            &log,
        );
        assert!(on.transcript.contains("> auto "), "{name}: auto ran");
        assert_eq!(
            interactive_transcript(&on.transcript),
            interactive_transcript(&off.transcript),
            "{name}: transcripts"
        );
        assert_eq!(on.tail, off.tail, "{name}: digests, PRNG and state");
        assert!(on.saved == off.saved, "{name}: SAVED.GAM bytes");
    }
}

#[test]
fn whole_party_auto_ends_each_fight_within_the_caps() {
    let Some(source) = configured_original_asset_dir() else {
        return;
    };
    for fixture in FIXTURES {
        let name = fixture.name();
        let dir = game_copy(&source, &format!("{name}-caps"));
        let mut state = fixture.state(&dir).unwrap();
        let settings = auto_on();
        let mut hooks = AutoCombatHooks::new(settings.clone()).unwrap();
        let commands = vec!["@auto party".to_string()];
        let mut ran = 0;
        replay_play_script_commands_hooked(&mut state, &dir, &commands, &mut hooks, |_, _, _| {
            ran += 1;
            Ok(())
        })
        .unwrap();
        let auto = hooks.auto();
        assert_eq!(
            hooks.notes().first().map(String::as_str),
            Some("Auto: party")
        );
        assert!(ran > 0, "{name}: auto typed keys");
        assert_eq!(ran, auto.keys_typed(), "{name}: one command per key");
        assert!(
            ran <= usize::from(settings.action_cap),
            "{name}: {ran} keys within the action cap"
        );
        let ended = auto.last_hand_back();
        assert!(
            matches!(
                ended,
                Some(
                    HandBack::Victory
                        | HandBack::FightOver
                        | HandBack::Hurt(_)
                        | HandBack::NoTarget
                        | HandBack::Stuck
                        | HandBack::ActionCap
                        | HandBack::UnexpectedPrompt
                )
            ),
            "{name}: ended by {ended:?}"
        );
        assert!(!auto.engagement().any(), "{name}: engagement cleared");
        eprintln!("{name}: {ran} keys, {ended:?}");
        let _ = fs::remove_dir_all(&dir);
    }
}
