//! Restarting the game process when the shell asks (Phase B design 3.3.5
//! "Relaunch").
//!
//! Bevy's window loop runs once per process, so a slot restore in play ends
//! the app with a `ShellExit` that asks for a relaunch, and the host starts
//! the game again with `--boot-key J`: the intro's boot poll then takes the
//! Journey Onward shortcut and loads the restored save.
//!
//! - **Arguments.** The original command line, with any old `--boot-key`
//!   dropped and the new one appended. One-shot setup steps that already
//!   ran are dropped too (`--settings`, `--slot-load NAME`, and `--setup`
//!   with the folder it set up, which `config.toml` now names), so the
//!   relaunch neither reopens a window the player closed, restores a slot
//!   twice, nor turns into another mode.
//! - **Unix.** The process is replaced with `exec`, keeping the PID and
//!   the terminal.
//! - **Elsewhere.** The executable is started again, not waited for, and
//!   the caller exits 0.
//! - **Failure.** [`SpawnFailed`]: the caller rolls the restore back,
//!   prints the reason and exits non-zero.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::Path;
use std::process::Command;

/// The relaunch could not start the new process. The window is already
/// closed, so the session cannot go on; the caller undoes what it
/// changed so the next launch finds the game as it was.
#[derive(Debug)]
pub struct SpawnFailed(pub io::Error);

impl fmt::Display for SpawnFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not restart the game: {}", self.0)
    }
}

impl std::error::Error for SpawnFailed {}

impl From<SpawnFailed> for io::Error {
    fn from(failed: SpawnFailed) -> Self {
        io::Error::new(failed.0.kind(), failed.to_string())
    }
}

const BOOT_KEY_FLAG: &str = "--boot-key";
/// A launch by `--slot-load NAME` already restored that slot; restoring it
/// again on a relaunch would undo the load the relaunch is for.
const SLOT_LOAD_FLAG: &str = "--slot-load";

/// The relaunch's arguments (without the program name): `argv` in order,
/// less any old `--boot-key C`, `--slot-load NAME`, `--settings` and
/// `--setup` (and, with `--setup`, the folder `setup_dir` it was given, as
/// `--game-dir DIR` or a bare `DIR`), then `--boot-key` and `boot_key` when there is one.
pub fn relaunch_args(
    argv: &[OsString],
    boot_key: Option<u8>,
    setup_dir: Option<&OsStr>,
) -> Vec<OsString> {
    let had_setup = argv.iter().any(|arg| arg == "--setup");
    let setup_dir = setup_dir.filter(|_| had_setup);
    let mut dropped_dir = false;
    let mut out = Vec::with_capacity(argv.len() + 2);
    let mut args = argv.iter().peekable();
    while let Some(arg) = args.next() {
        if arg == BOOT_KEY_FLAG || arg == SLOT_LOAD_FLAG {
            args.next();
            continue;
        }
        if arg == "--settings" || arg == "--setup" {
            continue;
        }
        if let Some(dir) = setup_dir
            && !dropped_dir
        {
            if arg == "--game-dir" && args.peek().is_some_and(|next| *next == dir) {
                args.next();
                dropped_dir = true;
                continue;
            }
            if arg == dir {
                dropped_dir = true;
                continue;
            }
        }
        out.push(arg.clone());
    }
    if let Some(key) = boot_key {
        out.push(OsString::from(BOOT_KEY_FLAG));
        out.push(OsString::from(char::from(key).to_string()));
    }
    out
}

/// Restarts this executable with [`relaunch_args`] built from the
/// process's own command line. On Unix this returns only on failure; `Ok`
/// (elsewhere) means the new process is running and the caller exits 0.
pub fn relaunch(boot_key: Option<u8>, setup_dir: Option<&Path>) -> Result<(), SpawnFailed> {
    let program = std::env::current_exe().map_err(SpawnFailed)?;
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    let args = relaunch_args(&argv, boot_key, setup_dir.map(Path::as_os_str));
    relaunch_program(&program, &args)
}

/// Starts `program` with `args` in this process's place.
pub fn relaunch_program(program: &Path, args: &[OsString]) -> Result<(), SpawnFailed> {
    let mut command = Command::new(program);
    command.args(args);
    replace_or_spawn(command)
}

#[cfg(unix)]
fn replace_or_spawn(mut command: Command) -> Result<(), SpawnFailed> {
    use std::os::unix::process::CommandExt as _;
    // `exec` returns only when the new program could not be started.
    Err(SpawnFailed(command.exec()))
}

#[cfg(not(unix))]
fn replace_or_spawn(mut command: Command) -> Result<(), SpawnFailed> {
    // Not waited for: the new game outlives this process, which exits.
    command.spawn().map(drop).map_err(SpawnFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn an_old_boot_key_is_dropped_and_the_rest_keep_their_order() {
        let argv = os(&[
            "--visual-playable",
            "--boot-key",
            "x",
            "--preset",
            "recommended",
            "--mute",
            "/games/u5",
        ]);
        assert_eq!(
            relaunch_args(&argv, Some(b'J'), None),
            os(&[
                "--visual-playable",
                "--preset",
                "recommended",
                "--mute",
                "/games/u5",
                "--boot-key",
                "J",
            ])
        );
    }

    #[test]
    fn a_bare_launch_gains_only_the_boot_key() {
        assert_eq!(
            relaunch_args(&[], Some(b'J'), None),
            os(&["--boot-key", "J"])
        );
        assert_eq!(relaunch_args(&[], None, None), Vec::<OsString>::new());
        // Without a new key, an old one is still dropped.
        assert_eq!(
            relaunch_args(&os(&["--boot-key", "J", "--parity-lock"]), None, None),
            os(&["--parity-lock"])
        );
    }

    #[test]
    fn values_that_look_like_flags_are_kept() {
        // Only the flag itself is matched; the value after `--set` stays.
        let argv = os(&["--set", "menu.quicksave=false", "--boot-key", "--setup"]);
        assert_eq!(
            relaunch_args(&argv, Some(b'J'), None),
            os(&["--set", "menu.quicksave=false", "--boot-key", "J"])
        );
    }

    #[test]
    fn one_shot_setup_steps_are_not_repeated() {
        assert_eq!(
            relaunch_args(&os(&["--settings"]), Some(b'J'), None),
            os(&["--boot-key", "J"])
        );
        assert_eq!(
            relaunch_args(&os(&["--setup"]), Some(b'J'), None),
            os(&["--boot-key", "J"])
        );
        // `--setup DIR`: the folder is in config.toml now, so the relaunch
        // is the default launch, not the report a bare DIR would run.
        let dir = OsStr::new("/games/u5");
        assert_eq!(
            relaunch_args(
                &os(&["--setup", "/games/u5", "--mute"]),
                Some(b'J'),
                Some(dir)
            ),
            os(&["--mute", "--boot-key", "J"])
        );
        assert_eq!(
            relaunch_args(
                &os(&["--game-dir", "/games/u5", "--setup"]),
                Some(b'J'),
                Some(dir)
            ),
            os(&["--boot-key", "J"])
        );
        // Without `--setup` a folder is part of the launch and stays.
        assert_eq!(
            relaunch_args(
                &os(&["--visual-playable", "/games/u5"]),
                Some(b'J'),
                Some(dir)
            ),
            os(&["--visual-playable", "/games/u5", "--boot-key", "J"])
        );
    }

    /// A launch by `--slot-load` restored its slot once; a later relaunch
    /// (a load in play) must not restore it again.
    #[test]
    fn a_slot_load_is_not_repeated() {
        assert_eq!(
            relaunch_args(
                &os(&["--slot-load", "Despise", "--named-slots"]),
                Some(b'J'),
                None
            ),
            os(&["--named-slots", "--boot-key", "J"])
        );
    }

    #[test]
    fn a_missing_program_is_a_spawn_failure() {
        let missing = std::env::temp_dir().join("u5-engine-relaunch-no-such-program");
        let failed = relaunch_program(&missing, &os(&["--boot-key", "J"])).unwrap_err();
        assert_eq!(failed.0.kind(), io::ErrorKind::NotFound);
        assert!(
            failed
                .to_string()
                .starts_with("could not restart the game: ")
        );
        let as_io: io::Error = failed.into();
        assert_eq!(as_io.kind(), io::ErrorKind::NotFound);
    }
}
