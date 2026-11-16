//! Chapter 7's host-side hardening (design 7.4.3, 7.4.5, 7.6; build plan
//! M8-06):
//!
//! - every cheat on in `config.toml` and on the command line, plus the
//!   parity lock: no cheat resolves, the file is not opened, and the only
//!   lines printed are the foundation's (the locked line, and its note on
//!   the ignored flags); cheats print none;
//! - the terminal equivalence over the matrix's cheats block: each row
//!   resolves with its cheats on and no note, and equals all off with the
//!   saved files compared.

use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{
    CHEATS_BLOCK_PREFIX, CheatToggles, CliSetting, LockReason, LockState, SettingNote, TOGGLES,
    cheats_block, toggle_matrix,
};
use u5_runtime::test_fixtures::{
    open_grid, saved_game_seed_bytes, test_state, write_empty_ool_mirrors,
};
use u5_runtime::{PlayState, SAVED_GAM_FILENAME};
use u5_tui::enhancement_settings::{resolve_launch, startup_report};
use u5_tui::*;

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-tui-cheats-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// A folder holding a seed save, for the scripted `Q`, `Y`.
    fn with_save(label: &str) -> Self {
        let scratch = Self::new(label);
        fs::write(
            scratch.0.join(SAVED_GAM_FILENAME),
            saved_game_seed_bytes(0x01, 0, 5, 5),
        )
        .unwrap();
        write_empty_ool_mirrors(&scratch.0);
        scratch
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

fn cheat_keys() -> Vec<&'static str> {
    TOGGLES
        .iter()
        .map(|spec| spec.key)
        .filter(|key| key.starts_with("cheats."))
        .collect()
}

#[test]
fn every_cheat_under_the_lock_prints_only_the_foundation_s_lines() {
    let scratch = Scratch::new("lock");
    let config = scratch.path().join("config.toml");
    let mut text = String::from("[cheats]\n");
    for key in cheat_keys() {
        text.push_str(&format!("{} = true\n", key.trim_start_matches("cheats.")));
    }
    fs::write(&config, &text).unwrap();
    let lock = LockState::Locked(LockReason::EnvVar);

    // From the file alone: the file is not even opened, and the one line
    // is the locked line.
    let settings = resolve_launch(Some(&config), &[], lock);
    assert_eq!(settings.config, None, "the file is not read under the lock");
    assert_eq!(settings.resolved.cheats(), CheatToggles::default());
    assert!(settings.resolved.is_classic());
    assert_eq!(startup_report(&settings), [PARITY_LOCK_LINE]);

    // The same keys on the command line are accepted and ignored: the
    // foundation's note lists them, and nothing else is printed.
    let cli: Vec<CliSetting> = cheat_keys()
        .into_iter()
        .map(|key| CliSetting::Enable(key.into()))
        .collect();
    let settings = resolve_launch(Some(&config), &cli, lock);
    assert_eq!(settings.resolved.cheats(), CheatToggles::default());
    assert!(
        matches!(
            settings.resolved.notes.as_slice(),
            [SettingNote::LockIgnored { .. }]
        ),
        "{:?}",
        settings.resolved.notes
    );
    let report = startup_report(&settings);
    assert_eq!(report.len(), 2, "{report:?}");
    assert_eq!(report[0], PARITY_LOCK_LINE);
    assert!(report[1].starts_with("enhancements: parity lock: ignored "));
    for key in cheat_keys() {
        assert!(report[1].contains(key), "{key}");
    }

    // Unlocked, the same file does turn them on, so the lock did the work.
    let open = resolve_launch(Some(&config), &[], LockState::Off);
    assert!(open.resolved.cheats().any_on());
    assert!(open.resolved.cheats().teleport);
}

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

/// The cheats block is in the committed matrix the suite reads, and each
/// row equals all off in the terminal, saved files included. Cheats have
/// no terminal feature, so this is the suite's own run of that block.
#[test]
fn every_cheats_block_row_equals_all_off_in_the_terminal() {
    let block = cheats_block();
    let matrix = toggle_matrix();
    assert!(matrix.ends_with(&block));
    assert!(
        block
            .iter()
            .all(|row| row.name.starts_with(CHEATS_BLOCK_PREFIX))
    );
    let commands: Vec<String> = ["pass", "idle:2", "pass"]
        .iter()
        .map(|c| c.to_string())
        .collect();
    for row in block {
        let resolved = resolve_launch(None, &row.settings, LockState::Off).resolved;
        assert!(
            resolved.notes.is_empty(),
            "{}: {:?}",
            row.name,
            resolved.notes
        );
        assert!(resolved.cheats().any_on(), "{}", row.name);
        let (off, on) = (Scratch::with_save("off"), Scratch::with_save("on"));
        let mismatches = check_equivalence(
            &town(),
            Path::new(""),
            &commands,
            terminal_features(&resolved),
            Some((off.path(), on.path())),
        );
        assert_eq!(mismatches, [], "{}", row.name);
    }
}
