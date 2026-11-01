//! Temp-directory tests for validation, case folding, nested folders,
//! version identification and the normalising install.

use std::fs;
use std::path::{Path, PathBuf};

use u5_gamedata::*;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "u5-gamedata-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = make_tree_writable(&self.0);
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn make_tree_writable(dir: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let mut perms = fs::metadata(&path)?.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        let _ = fs::set_permissions(&path, perms);
        if path.is_dir() {
            make_tree_writable(&path)?;
        }
    }
    Ok(())
}

/// Synthetic stand-in bytes for a manifest file (never real game data).
fn fake_bytes(name: &str) -> Vec<u8> {
    format!("synthetic test content for {name}\n").into_bytes()
}

/// Write every manifest file (optionally including saves) with `rename`
/// applied to each name.
fn populate(dir: &Path, with_saves: bool, rename: impl Fn(&str) -> String) {
    fs::create_dir_all(dir).unwrap();
    for file in MANIFEST {
        if file.is_save() && !with_saves {
            continue;
        }
        fs::write(dir.join(rename(file.name)), fake_bytes(file.name)).unwrap();
    }
}

fn pristine_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("U5_TEST_ASSET_DIR")?);
    dir.is_dir().then_some(dir)
}

#[test]
fn complete_synthetic_set_is_usable_but_unknown_version() {
    let tmp = TempDir::new("complete");
    populate(tmp.path(), true, str::to_string);
    let report = validate_dir(tmp.path());
    assert!(report.is_usable(), "{}", report.summary());
    assert!(report.missing_required().is_empty());
    assert!(report.case_fixes().is_empty());
    match &report.version {
        VersionIdentification::Unknown { closest, differing } => {
            assert_eq!(*closest, Some("gog-1.0"));
            assert_eq!(differing.len(), required_files().count());
        }
        other => panic!("expected unknown version, got {other:?}"),
    }
    assert!(report.summary().contains("Warning"));
    assert!(report.status_line().starts_with("ready"));
}

#[test]
fn lowercase_names_resolve_and_install_upper_case() {
    let source = TempDir::new("lower-src");
    let dest = TempDir::new("lower-dest");
    populate(source.path(), true, |name| name.to_ascii_lowercase());
    let report = validate_dir(source.path());
    assert!(report.is_usable(), "{}", report.summary());
    assert_eq!(report.case_fixes().len(), MANIFEST.len());
    assert!(report.summary().contains("upper-case"));

    let managed = dest.path().join("data");
    let outcome = install(&report, &managed).unwrap();
    assert_eq!(outcome.copied.len(), MANIFEST.len());
    for file in MANIFEST {
        let target = managed.join(file.name);
        assert_eq!(
            fs::read(&target).unwrap(),
            fake_bytes(file.name),
            "{}",
            file.name
        );
    }
    // The managed copy now validates with no case fixes, and the source was
    // not touched.
    let again = validate_dir(&managed);
    assert!(again.is_usable());
    assert!(again.case_fixes().is_empty());
    assert!(source.path().join("brit.dat").is_file());
    assert!(!source.path().join("BRIT.DAT").exists() || cfg!(any(windows, target_os = "macos")));
}

#[test]
fn mixed_case_names_resolve() {
    let tmp = TempDir::new("mixed");
    populate(tmp.path(), false, |name| {
        let (stem, ext) = name.split_once('.').unwrap();
        let mut chars = stem.chars();
        let first = chars.next().unwrap();
        format!(
            "{first}{}.{}",
            chars.as_str().to_ascii_lowercase(),
            ext.to_ascii_lowercase()
        )
    });
    let report = validate_dir(tmp.path());
    assert!(report.is_usable(), "{}", report.summary());
}

#[test]
fn missing_files_are_reported_in_plain_language() {
    let tmp = TempDir::new("missing");
    populate(tmp.path(), false, str::to_string);
    fs::remove_file(tmp.path().join("TILES.16")).unwrap();
    fs::remove_file(tmp.path().join("BRIT.DAT")).unwrap();
    let report = validate_dir(tmp.path());
    assert!(!report.is_usable());
    assert_eq!(report.missing_required(), vec!["BRIT.DAT", "TILES.16"]);
    assert_eq!(report.version, VersionIdentification::Incomplete);
    let summary = report.summary();
    assert!(
        summary.contains("2 of the 63 files the game needs are missing"),
        "{summary}"
    );
    assert!(summary.contains("BRIT.DAT, TILES.16"), "{summary}");
    assert!(summary.contains("incomplete"), "{summary}");
    assert!(
        report
            .status_line()
            .contains("2 of 63 required files missing")
    );
    // Install refuses an unusable report.
    let dest = TempDir::new("missing-dest");
    assert!(matches!(
        install(&report, dest.path()),
        Err(InstallError::NotUsable(_))
    ));
}

#[test]
fn saves_are_optional() {
    let tmp = TempDir::new("nosaves");
    populate(tmp.path(), false, str::to_string);
    let report = validate_dir(tmp.path());
    assert!(report.is_usable());
    assert_eq!(report.present_count(), required_files().count());
}

#[test]
fn unrelated_folder_says_it_is_not_ultima() {
    let tmp = TempDir::new("unrelated");
    fs::write(tmp.path().join("README.TXT"), b"hello").unwrap();
    let report = validate_dir(tmp.path());
    assert!(!report.is_usable());
    assert!(
        report
            .summary()
            .contains("does not look like an Ultima V folder")
    );
    assert_eq!(locate_data_root(tmp.path(), SearchLimits::default()), None);
}

#[test]
fn nonexistent_folder_reports_dir_error() {
    let report = validate_dir(Path::new("/definitely/not/here/u5"));
    assert!(!report.is_usable());
    assert_eq!(report.dir_error.as_deref(), Some("it does not exist"));
    assert!(report.summary().contains("could not be opened"));
}

#[test]
fn nested_subfolder_is_located() {
    let tmp = TempDir::new("nested");
    // Shaped like a multi-game bundle: the DOS files sit two levels down,
    // next to sibling games and a few unrelated files.
    let bundle = tmp.path().join("Ultima 4+5+6");
    fs::create_dir_all(bundle.join("Ultima 4").join("data")).unwrap();
    fs::write(bundle.join("Ultima 4").join("data").join("WORLD.MAP"), b"x").unwrap();
    fs::write(bundle.join("goggame.ico"), b"x").unwrap();
    let u5 = bundle.join("Ultima 5").join("data");
    populate(&u5, true, str::to_string);

    assert_eq!(
        locate_data_root(tmp.path(), SearchLimits::default()),
        Some(u5.clone())
    );
    // Too shallow a search misses it.
    let shallow = SearchLimits {
        max_depth: 1,
        ..SearchLimits::default()
    };
    assert_eq!(locate_data_root(tmp.path(), shallow), None);

    let report = validate_location(tmp.path(), SearchLimits::default());
    assert_eq!(report.dir, u5);
    assert!(report.is_usable());
}

#[test]
fn nested_partial_copy_is_still_located_for_a_useful_report() {
    let tmp = TempDir::new("nested-partial");
    let inner = tmp.path().join("game");
    populate(&inner, false, str::to_string);
    fs::remove_file(inner.join("SIGNS.DAT")).unwrap();
    let report = validate_location(tmp.path(), SearchLimits::default());
    assert_eq!(report.dir, inner);
    assert_eq!(report.missing_required(), vec!["SIGNS.DAT"]);
}

#[test]
fn hidden_directories_are_not_searched() {
    let tmp = TempDir::new("hidden");
    populate(&tmp.path().join(".cache").join("u5"), false, str::to_string);
    assert_eq!(locate_data_root(tmp.path(), SearchLimits::default()), None);
}

#[cfg(unix)]
#[test]
fn unreadable_file_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    // Root ignores permission bits; nothing to test then.
    if fs::read_dir("/root").is_ok() {
        return;
    }
    let tmp = TempDir::new("unreadable");
    populate(tmp.path(), false, str::to_string);
    let path = tmp.path().join("KARMA.DAT");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    let report = validate_dir(tmp.path());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!report.is_usable());
    let unreadable = report.unreadable();
    assert_eq!(unreadable.len(), 1);
    assert_eq!(unreadable[0].0, "KARMA.DAT");
    assert!(
        report
            .summary()
            .contains("KARMA.DAT exists but could not be read")
    );
}

#[test]
fn install_refreshes_static_files_and_keeps_saves() {
    let source = TempDir::new("refresh-src");
    let dest = TempDir::new("refresh-dest");
    populate(source.path(), true, str::to_string);
    let report = validate_dir(source.path());
    let first = install(&report, dest.path()).unwrap();
    assert_eq!(first.copied.len(), MANIFEST.len());

    // The player saves; a static file is corrupted in the managed copy.
    fs::write(dest.path().join("SAVED.GAM"), b"progress").unwrap();
    fs::write(dest.path().join("TILES.16"), b"damaged").unwrap();
    let second = install(&report, dest.path()).unwrap();
    assert_eq!(second.copied, vec!["TILES.16"]);
    assert!(second.saves_kept.contains(&"SAVED.GAM"));
    assert_eq!(
        fs::read(dest.path().join("SAVED.GAM")).unwrap(),
        b"progress"
    );
    assert_eq!(
        fs::read(dest.path().join("TILES.16")).unwrap(),
        fake_bytes("TILES.16")
    );
    assert_eq!(second.unchanged.len(), required_files().count() - 1);
}

#[test]
fn install_into_canonical_source_is_a_no_op_and_lowercase_in_place_is_refused() {
    let tmp = TempDir::new("inplace");
    populate(tmp.path(), false, str::to_string);
    let report = validate_dir(tmp.path());
    let outcome = install(&report, tmp.path()).unwrap();
    assert!(outcome.copied.is_empty());

    let lower = TempDir::new("inplace-lower");
    populate(lower.path(), false, |name| name.to_ascii_lowercase());
    let report = validate_dir(lower.path());
    assert!(matches!(
        install(&report, lower.path()),
        Err(InstallError::DestinationIsSource(_))
    ));
}

#[cfg(unix)]
#[test]
fn install_from_read_only_source_leaves_writable_copy() {
    use std::os::unix::fs::PermissionsExt;
    let source = TempDir::new("ro-src");
    let dest = TempDir::new("ro-dest");
    populate(source.path(), true, str::to_string);
    for file in MANIFEST {
        fs::set_permissions(
            source.path().join(file.name),
            fs::Permissions::from_mode(0o444),
        )
        .unwrap();
    }
    let report = validate_dir(source.path());
    install(&report, dest.path()).unwrap();
    for file in MANIFEST {
        let meta = fs::metadata(dest.path().join(file.name)).unwrap();
        assert!(!meta.permissions().readonly(), "{}", file.name);
    }
}

#[test]
fn discovery_dedups_and_ranks() {
    let tmp = TempDir::new("discover");
    let good = tmp.path().join("GOG Games").join("Ultima 5");
    populate(&good, false, str::to_string);
    let partial = tmp.path().join("Games").join("ultima-v");
    populate(&partial, false, str::to_string);
    fs::remove_file(partial.join("TOWNE.TLK")).unwrap();
    let nothing = tmp.path().join("Empty");
    fs::create_dir_all(&nothing).unwrap();

    let tmp_root = tmp.path().to_path_buf();
    let provider = move || {
        vec![
            Candidate::new(tmp_root.join("Games"), "games folder", 2),
            Candidate::new(tmp_root.join("Empty"), "empty", 2),
            Candidate::new(tmp_root.join("Missing"), "missing", 2),
            Candidate::new(tmp_root.join("GOG Games"), "GOG", 2),
            // Same data reached again, deeper: reported once.
            Candidate::new(tmp_root.join("GOG Games").join("Ultima 5"), "GOG direct", 0),
        ]
    };
    let found = discover(&[&provider]);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!(found[0].data_root, good);
    assert_eq!(found[0].candidate.source, "GOG");
    assert!(found[0].report.is_usable());
    assert_eq!(found[1].data_root, partial);
    assert!(!found[1].report.is_usable());
}

#[test]
fn pristine_copy_is_identified_as_gog() {
    let Some(dir) = pristine_dir() else {
        eprintln!("U5_TEST_ASSET_DIR not set; skipping");
        return;
    };
    let report = validate_dir(&dir);
    assert!(report.is_usable(), "{}", report.summary());
    assert_eq!(
        report.version.known_id(),
        Some("gog-1.0"),
        "{:?}",
        report.version
    );
    // Every manifest file, saves included, ships with the release.
    assert_eq!(report.present_count(), MANIFEST.len());
}

#[test]
fn pristine_copy_lowercased_still_identifies_and_installs() {
    let Some(dir) = pristine_dir() else {
        return;
    };
    let lower = TempDir::new("pristine-lower");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        fs::copy(entry.path(), lower.path().join(name)).unwrap();
    }
    let report = validate_dir(lower.path());
    assert_eq!(report.version.known_id(), Some("gog-1.0"));
    let dest = TempDir::new("pristine-dest");
    let outcome = install(&report, dest.path()).unwrap();
    assert_eq!(outcome.copied.len(), MANIFEST.len());
    assert_eq!(
        validate_dir(dest.path()).version.known_id(),
        Some("gog-1.0")
    );
}
