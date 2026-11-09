//! Contract checks for the enhancement foundation (Phase B design 1.3.1,
//! 1.4.1; build plan M0-05).
//!
//! - `docs/settings.md` holds the table [`settings_table`] renders from the
//!   catalogue, so a key cannot be added without its documentation row.
//! - Host code stays out of the engine: the engine crates' enhancement
//!   modules never name `toml`, `std::env` or a host crate, and the engine
//!   crates do not depend on them. Ports replace the host layer only.

use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::settings_table;

const BEGIN: &str = "<!-- BEGIN GENERATED: settings table (cargo test -p u5-runtime) -->\n";
const END: &str = "<!-- END GENERATED -->\n";

/// Set to `1` to rewrite the generated block of `docs/settings.md`.
const BLESS: &str = "U5_BLESS_SETTINGS_DOCS";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_docs_table_equals_the_catalogue() {
    let path = workspace_root().join("docs/settings.md");
    let text = fs::read_to_string(&path).expect("docs/settings.md exists");
    let start = text
        .find(BEGIN)
        .expect("docs/settings.md has the begin marker")
        + BEGIN.len();
    let end = start
        + text[start..]
            .find(END)
            .expect("docs/settings.md has the end marker");
    let expected = settings_table();
    if text[start..end] == expected {
        return;
    }
    if std::env::var(BLESS).as_deref() == Ok("1") {
        let updated = format!("{}{expected}{}", &text[..start], &text[end..]);
        fs::write(&path, updated).expect("rewrite docs/settings.md");
        return;
    }
    panic!(
        "docs/settings.md is out of date with the catalogue; rerun with {BLESS}=1 \
         to regenerate it. Expected table:\n{expected}"
    );
}

/// Names the engine's enhancement code must not use. Comments are not
/// scanned, so prose may still mention `config.toml`.
const HOST_ONLY: &[&str] = &[
    "toml::",
    "toml_edit",
    "use toml",
    "extern crate toml",
    "std::env",
    "env::var",
    "env!(",
    "u5_host_desktop",
    "u5_tui",
    "rfd::",
];

/// Host crates and host-only libraries the engine crates must not depend
/// on.
const HOST_DEPENDENCIES: &[&str] = &["toml", "toml_edit", "u5-host-desktop", "u5-tui", "rfd"];

/// The engine crates (1.3.1), and their enhancement modules where they
/// exist yet: `u5-bevy/src/enh` arrives in P5 and `u5-automap` in chapter 4.
const ENGINE_CRATES: &[&str] = &["u5-runtime", "u5-bevy", "u5-bevy-setup", "u5-automap"];
const ENHANCEMENT_MODULES: &[&str] = &[
    "crates/u5-runtime/src/enhancements",
    "crates/u5-bevy/src/enh",
    "crates/u5-automap/src",
];

/// The source with `//` comments removed. String contents are kept.
fn code_only(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn host_names_in(source: &str) -> Vec<&'static str> {
    let code = code_only(source);
    HOST_ONLY
        .iter()
        .copied()
        .filter(|name| code.contains(name))
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn engine_enhancement_modules_hold_no_host_code() {
    let root = workspace_root();
    let mut files = Vec::new();
    for module in ENHANCEMENT_MODULES {
        rust_files(&root.join(module), &mut files);
    }
    assert!(
        files.iter().any(|path| path.ends_with("resolve.rs")),
        "the scan must at least see u5-runtime's enhancement module"
    );
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        let found = host_names_in(&source);
        assert!(
            found.is_empty(),
            "{} names host-only code {found:?}; host code belongs in u5-host-desktop or u5-tui",
            path.display()
        );
    }
}

#[test]
fn engine_crates_do_not_depend_on_host_crates() {
    let root = workspace_root();
    for krate in ENGINE_CRATES {
        let Ok(manifest) = fs::read_to_string(root.join("crates").join(krate).join("Cargo.toml"))
        else {
            continue;
        };
        for line in manifest.lines() {
            let name = line.split(['=', ' ', '.']).next().unwrap_or("").trim();
            assert!(
                !HOST_DEPENDENCIES.contains(&name),
                "{krate} depends on host-only {name}"
            );
        }
    }
}

#[test]
fn the_scan_catches_host_code_and_ignores_comments() {
    assert_eq!(
        host_names_in("let doc: toml_edit::DocumentMut = toml::from_str(x);"),
        ["toml::", "toml_edit"]
    );
    assert_eq!(
        host_names_in("let v = std::env::var(\"X\");"),
        ["std::env", "env::var"]
    );
    assert_eq!(
        host_names_in("use u5_host_desktop::settings;"),
        ["u5_host_desktop"]
    );
    assert!(
        host_names_in("// reads config.toml via toml_edit and std::env\nlet x = 1;").is_empty()
    );
    assert!(host_names_in("const NOTE: &str = \"config.toml\";").is_empty());
}
