//! `--quest-journal-report` (Phase B design 5.2.11, 5.3.3, 5.6; build plan
//! M5-02): R1 parsing, R3 the report touches nothing, status 2 on a short
//! save, and one asset-gated count-only run. R2, the golden text, is in
//! u5-runtime's quest journal tests.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use u5_runtime::enhancements::journal::SpoilerTier;
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, saved_game_seed_bytes, write_empty_ool_mirrors,
};
use u5_runtime::{SAVE_AVATAR_NAME_OFFSET, SAVE_SHRINE_ORDAINED_MASK_OFFSET, SAVED_GAM_FILENAME};
use u5_tui::{parse_cli_command, quest_journal_report_text, runtime_mirror_path};

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "u5-tui-qj-report-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A decodable SAVED.GAM: a named Avatar and `ordained` as the ordained
/// virtue mask.
fn save_bytes(ordained: u8) -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0, 0, 10, 10);
    bytes[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 6].copy_from_slice(b"Avatar");
    bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = ordained;
    bytes
}

fn parse_error(args: &[&str]) -> String {
    let error = parse_cli_command(args.iter().copied()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{error}");
    error.to_string()
}

fn report_tier(args: &[&str]) -> Option<SpoilerTier> {
    parse_cli_command(args.iter().copied())
        .unwrap()
        .args
        .quest_journal_report
}

#[test]
fn r1_the_report_flags_parse() {
    assert_eq!(report_tier(&[]), None);
    assert_eq!(report_tier(&["--play"]), None);
    assert_eq!(
        report_tier(&["--quest-journal-report"]),
        Some(SpoilerTier::Goals)
    );
    for (value, tier) in ["1", "2", "3", "4"].into_iter().zip(SpoilerTier::ALL) {
        assert_eq!(
            report_tier(&["--quest-journal-report", "--spoilers", value]),
            Some(tier)
        );
        // Separate-value arms, in either order.
        assert_eq!(
            report_tier(&["--spoilers", value, "--quest-journal-report"]),
            Some(tier)
        );
    }
    for bad in ["0", "5", "x", "-1"] {
        assert!(
            parse_error(&["--quest-journal-report", "--spoilers", bad])
                .contains("--spoilers takes a tier from 1 to 4"),
            "{bad}"
        );
    }
    assert!(parse_error(&["--quest-journal-report", "--spoilers"]).contains("requires a tier"));
    assert!(
        parse_error(&[
            "--quest-journal-report",
            "--spoilers",
            "2",
            "--spoilers",
            "3"
        ])
        .contains("only be supplied once")
    );
    // The game folder, positional or with --game-dir.
    for args in [
        &["--quest-journal-report", "/x/u5"][..],
        &["--quest-journal-report", "--game-dir", "/x/u5"][..],
    ] {
        let command = parse_cli_command(args.iter().copied()).unwrap();
        assert_eq!(command.explicit_game_dir, Some(PathBuf::from("/x/u5")));
        assert!(command.args.has_mode());
    }
    // A command, not a toggle: it takes no mode and no setting.
    for other in [
        &["--play"][..],
        &["--setup"][..],
        &["--parity-lock"][..],
        &["--show-settings"][..],
        &["--preset", "classic"][..],
    ] {
        let mut args = vec!["--quest-journal-report"];
        args.extend_from_slice(other);
        assert!(
            parse_error(&args).contains("--quest-journal-report prints the journal"),
            "{other:?}"
        );
    }
    // Typos stay rejected; `--spoilers` outside the report waits for its
    // row to ship.
    assert!(parse_error(&["--quest-journa"]).contains("unknown option `--quest-journa`"));
    assert!(parse_error(&["--quest-journal-reports"]).contains("unknown option"));
    let shipped = u5_runtime::enhancements::CatalogueRow::find("journal.spoiler_tier")
        .unwrap()
        .is_shipped();
    if !shipped {
        assert!(parse_error(&["--spoilers", "3"]).contains("unknown option `--spoilers`"));
    }
}

/// Every entry under `dir`: its bytes and mtime (or `None` for a folder or
/// an unreadable file), and its permissions.
fn listing(dir: &Path) -> BTreeMap<PathBuf, (Option<Vec<u8>>, SystemTime, u32)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let entry = entry.unwrap();
            let meta = fs::symlink_metadata(entry.path()).unwrap();
            if meta.is_dir() {
                stack.push(entry.path());
            }
            use std::os::unix::fs::PermissionsExt;
            out.insert(
                entry.path(),
                (
                    meta.is_file()
                        .then(|| fs::read(entry.path()).ok())
                        .flatten(),
                    meta.modified().unwrap(),
                    meta.permissions().mode(),
                ),
            );
        }
    }
    out
}

fn run_report(game_dir: &Path, runtime_root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_u5-engine"))
        .arg("--quest-journal-report")
        .args(extra)
        .arg(game_dir)
        .env(u5_host_desktop::RUNTIME_DIR_ENV, runtime_root)
        .env(
            u5_host_desktop::CONFIG_ENV,
            runtime_root.join("no-config.toml"),
        )
        .output()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn r3_the_report_touches_nothing_and_reads_no_wps() {
    use std::os::unix::fs::PermissionsExt;
    let game = temp_dir("game");
    let base = temp_dir("root");
    let root = base.join("runtime");
    fs::write(game.join(SAVED_GAM_FILENAME), save_bytes(0b10)).unwrap();
    write_empty_ool_mirrors(&game);
    // An unreadable SAVED.WPS: a read would fail the report.
    let wps = game.join("SAVED.WPS");
    fs::write(&wps, b"stale").unwrap();
    fs::set_permissions(&wps, fs::Permissions::from_mode(0o000)).unwrap();
    let before = listing(&game);

    let output = run_report(&game, &root, &["--spoilers", "1"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "{stdout}");
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
    assert!(stdout.starts_with("Quest journal - tier 1 (progress only)\n"));
    assert!(stdout.contains("Ordained"), "{stdout}");
    assert_eq!(listing(&game), before);
    // No mirror, and no runtime root, was created.
    assert!(!root.exists());

    // An existing mirror with a SAVED.GAM is the save being played: the
    // report reads it, and changes nothing there either.
    let mirror = runtime_mirror_path(&root, &game.canonicalize().unwrap());
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join(SAVED_GAM_FILENAME), save_bytes(0)).unwrap();
    let mirror_before = listing(&root);
    let output = run_report(&game, &root, &["--spoilers", "1"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(!stdout.contains("Ordained"), "{stdout}");
    assert_eq!(listing(&game), before);
    assert_eq!(listing(&root), mirror_before);

    fs::set_permissions(&wps, fs::Permissions::from_mode(0o644)).unwrap();
    fs::remove_dir_all(game).unwrap();
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_short_save_exits_with_status_two_and_prints_no_journal() {
    let game = temp_dir("short");
    let base = temp_dir("short-root");
    fs::write(game.join(SAVED_GAM_FILENAME), [0u8; 16]).unwrap();
    let output = run_report(&game, &base.join("runtime"), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("cannot read SAVED.GAM: "), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    // A missing save is the same error.
    fs::remove_file(game.join(SAVED_GAM_FILENAME)).unwrap();
    let output = run_report(&game, &base.join("runtime"), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    fs::remove_dir_all(game).unwrap();
    fs::remove_dir_all(base).unwrap();
}

/// The player's own files, on a writable copy: the stock save holds no
/// game, so the report refuses it with status 2; a character created on the
/// copy then reads at every tier. Counts only; no save content is printed
/// or kept.
#[test]
fn the_report_reads_a_test_assets_copy_at_every_tier() {
    let Some(assets) = configured_original_asset_dir() else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR");
        return;
    };
    if !assets.join(SAVED_GAM_FILENAME).is_file() {
        eprintln!("skipped: no SAVED.GAM in U5_TEST_ASSET_DIR");
        return;
    }
    let copy = temp_dir("assets");
    for entry in fs::read_dir(&assets).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
        }
    }
    let root = copy.join("no-runtime");
    if quest_journal_report_text(&copy, SpoilerTier::Goals).is_err() {
        let output = run_report(&copy, &root, &[]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let made = Command::new(env!("CARGO_BIN_EXE_u5-engine"))
            .args(["--create-character", "Avatar", "--gender", "male"])
            .args(["--chargen-answers", "AAAAAAA"])
            .arg(&copy)
            .output()
            .unwrap();
        assert!(made.status.success());
    }
    let before = listing(&copy);
    let mut lines = 0;
    for tier in SpoilerTier::ALL {
        let report = quest_journal_report_text(&copy, tier).unwrap();
        let count = report.lines().count();
        let virtues = report
            .lines()
            .skip_while(|line| !line.starts_with("Virtues: "))
            .skip(1)
            .take_while(|line| line.starts_with("  "))
            .filter(|line| !line.starts_with("       "))
            .count();
        eprintln!("tier {}: {count} lines", tier.number());
        assert_eq!(virtues, 8, "tier {}", tier.number());
        assert!(count >= lines, "tier {}", tier.number());
        lines = count;
    }
    let output = run_report(&copy, &root, &["--spoilers", "4"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        quest_journal_report_text(&copy, SpoilerTier::Full).unwrap()
    );
    assert!(!root.exists());
    assert_eq!(listing(&copy), before);
    fs::remove_dir_all(copy).unwrap();
}
