//! `--quest-journal-report [--spoilers N] [GAME_DIR]` (Phase B design
//! 5.2.11, 5.3.3; build plan M5-02).
//!
//! The report prints the quest journal of a folder's save and exits. It
//! touches nothing: it reads SAVED.GAM and nothing else, never writes the
//! OOL mirrors (it does not call `load_play_options_from_save`), never
//! reads SAVED.WPS, never prepares a writable game folder and never runs
//! the legacy-root migration. When the runtime mirror of the folder already
//! holds a SAVED.GAM it reads that one, because it is the save the player
//! is actually playing.

use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use u5_host_desktop::{Config, HostEnv, paths};
use u5_runtime::enhancements::journal::SpoilerTier;
use u5_runtime::quest_journal::{QuestSnapshot, derive, report, text};
use u5_runtime::{SAVED_GAM_FILENAME, read_save_image_file};

use crate::runtime_game_dir::runtime_mirror_path;

/// The folder whose SAVED.GAM the report reads: the runtime mirror of
/// `game_dir` under `runtime_root` if it already holds a SAVED.GAM, or
/// `game_dir` itself. Read-only path probes only.
pub fn report_save_dir(game_dir: &Path, runtime_root: &Path) -> PathBuf {
    let canonical = game_dir
        .canonicalize()
        .unwrap_or_else(|_| game_dir.to_path_buf());
    let mirror = runtime_mirror_path(runtime_root, &canonical);
    if mirror.join(SAVED_GAM_FILENAME).is_file() {
        mirror
    } else {
        game_dir.to_path_buf()
    }
}

/// The whole report for the save in `save_dir`, or the one line that says
/// why it cannot be read. Never a partial journal.
pub fn quest_journal_report_text(save_dir: &Path, tier: SpoilerTier) -> Result<String, String> {
    let snapshot = read_save_image_file(&save_dir.join(SAVED_GAM_FILENAME), SAVED_GAM_FILENAME)
        .and_then(|bytes| QuestSnapshot::from_save_bytes(&bytes))
        .map_err(|error| text::cannot_read_save(&error.to_string()))?;
    Ok(report::format_text(&derive(&snapshot, tier)))
}

/// The game folder the report reads: the one given, or `data_dir` from
/// `config.toml`, as the default launch uses. It runs no setup.
fn report_game_dir(explicit: Option<&Path>, env: &HostEnv) -> io::Result<PathBuf> {
    if let Some(dir) = explicit {
        return Ok(dir.to_path_buf());
    }
    Config::load(&paths::config_file(env))
        .map_err(io::Error::other)?
        .data_dir
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no game folder is set up; give GAME_DIR or run --setup first",
            )
        })
}

/// Run the report: the text on stdout and status 0, or the reason on
/// stderr and status 2 when SAVED.GAM cannot be read (5.2.12).
pub fn run_quest_journal_report(explicit: Option<&Path>, tier: SpoilerTier) -> ExitCode {
    let env = HostEnv::from_process();
    let game_dir = match report_game_dir(explicit, &env) {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("Error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let save_dir = report_save_dir(&game_dir, &paths::runtime_root(&env));
    match quest_journal_report_text(&save_dir, tier) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(line) => {
            eprintln!("{line}");
            ExitCode::from(2)
        }
    }
}
