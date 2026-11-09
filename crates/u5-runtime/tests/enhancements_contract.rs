//! Contract checks for the enhancement foundation (Phase B design 1.3.1,
//! 1.4.1; build plan M0-05).
//!
//! - `docs/settings.md` holds the table [`settings_table`] renders from the
//!   catalogue, so a key cannot be added without its documentation row.
//! - Host code stays out of the engine: the engine crates' enhancement
//!   modules never name `toml`, `std::env` or a host crate, and the engine
//!   crates do not depend on them. Ports replace the host layer only.
//! - `docs/commands.md` lists every `[keys]` row with its default chord and
//!   label, and `docs/sidecars.md` lists every player sidecar file with its
//!   tag, in a section of its own (build plan M0-20).
//! - Every `take_pending_*` and wall-clock `advance_*_to` method the engine
//!   and its frontends name is classified in [`GUARDED_DRAINS`], so a new
//!   pending queue or animator cannot slip past the idle predicate (design
//!   1.3.7, 1.7; build plan M0-12).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{
    CONTAINER_MAGIC, GUARDED_DRAINS, MATRIX_FILE, OPTIONS, OptionDefault, OptionKind, SIDECAR_DIR,
    SIDECAR_FILES, settings_table,
};

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

/// Where the engine and its frontends drain pending work between frames.
const DRAIN_SCAN_DIRS: &[&str] = &[
    "crates/u5-runtime/src",
    "crates/u5-bevy/src",
    "crates/u5-tui/src",
];

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every `take_pending_*` and `advance_*_to` method named in `source`, as a
/// definition or a call: an identifier with that shape followed by `(`.
/// Comments are not scanned.
fn drain_names_in(source: &str) -> BTreeSet<String> {
    let code = code_only(source);
    let mut names = BTreeSet::new();
    for prefix in ["take_pending_", "advance_"] {
        for (start, _) in code.match_indices(prefix) {
            if code[..start].chars().next_back().is_some_and(is_ident_char) {
                continue;
            }
            let rest = &code[start..];
            let len = rest.find(|c| !is_ident_char(c)).unwrap_or(rest.len());
            let name = &rest[..len];
            if !rest[len..].trim_start().starts_with('(') {
                continue;
            }
            let shaped = prefix == "take_pending_"
                || (name.ends_with("_to") && name.len() > "advance_to".len());
            if shaped && name.len() > prefix.len() {
                names.insert(name.to_string());
            }
        }
    }
    names
}

/// Design 1.7, the first risk: a new `take_pending_*` queue or wall-clock
/// `advance_*_to` animator fails here until it is listed in
/// `GUARDED_DRAINS` with the idle blocker that covers it.
#[test]
fn every_pending_drain_is_classified_for_the_idle_predicate() {
    let root = workspace_root();
    let mut found = BTreeSet::new();
    for dir in DRAIN_SCAN_DIRS {
        let mut files = Vec::new();
        rust_files(&root.join(dir), &mut files);
        assert!(!files.is_empty(), "{dir} has sources");
        for path in files {
            found.extend(drain_names_in(&fs::read_to_string(&path).unwrap()));
        }
    }
    let classified: BTreeSet<String> = GUARDED_DRAINS
        .iter()
        .map(|drain| drain.method.to_string())
        .collect();
    let unclassified: Vec<&String> = found.difference(&classified).collect();
    assert!(
        unclassified.is_empty(),
        "classify these in u5_runtime::enhancements::idle::GUARDED_DRAINS, with the \
         idle blocker that holds the predicate false while they have work: {unclassified:?}"
    );
    let stale: Vec<&String> = classified.difference(&found).collect();
    assert!(stale.is_empty(), "no longer named anywhere: {stale:?}");
}

#[test]
fn the_drain_scan_sees_calls_and_definitions_only() {
    let names = drain_names_in(
        "let _ = state.take_pending_potion_flash();\n\
         pub fn advance_text_hold_to (&mut self) {}\n\
         self.staged_narration.advance_to(now);\n\
         let a = advance_by(x); my_take_pending_x(); // take_pending_y()\n\
         const N: &str = \"take_pending_z\";",
    );
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        ["advance_text_hold_to", "take_pending_potion_flash"]
    );
}

fn read_doc(name: &str) -> String {
    fs::read_to_string(workspace_root().join("docs").join(name))
        .unwrap_or_else(|error| panic!("docs/{name}: {error}"))
}

/// The text of `heading`'s section: from the heading to the next `## `.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("no `{heading}` section"));
    let rest = &text[start + heading.len()..];
    &rest[..rest.find("\n## ").unwrap_or(rest.len())]
}

#[test]
fn the_hotkey_docs_list_every_keys_row() {
    let text = read_doc("commands.md");
    let hotkeys = section(&text, "## Enhancement Hotkeys");
    let mut rows = 0;
    for spec in OPTIONS.iter().filter(|spec| spec.kind == OptionKind::Chord) {
        let OptionDefault::Str(chord) = spec.default else {
            panic!("{} has a chord default", spec.key);
        };
        let line = format!("| {chord} | `{}` | {} |", spec.key, spec.label);
        assert!(
            hotkeys.contains(&line),
            "docs/commands.md lacks the hotkey row starting `{line}`"
        );
        rows += 1;
    }
    // No row for a key the catalogue does not have.
    assert_eq!(
        hotkeys.matches("| `keys.").count(),
        rows,
        "docs/commands.md lists a [keys] row the catalogue does not have"
    );
}

#[test]
fn the_sidecar_docs_list_every_player_sidecar() {
    let text = read_doc("sidecars.md");
    let player = section(&text, "## Player Sidecars");
    assert!(player.contains(SIDECAR_DIR));
    let magic = std::str::from_utf8(&CONTAINER_MAGIC).unwrap();
    assert!(player.contains(&format!("`{magic}`")));
    for (tag, name) in SIDECAR_FILES {
        let tag = std::str::from_utf8(tag).unwrap();
        let line = format!("| `{name}` | `{tag}` |");
        assert!(player.contains(&line), "docs/sidecars.md lacks `{line}`");
    }
    assert_eq!(
        player.matches(".u5e` |").count(),
        SIDECAR_FILES.len(),
        "docs/sidecars.md lists a player sidecar the engine does not have"
    );
    // Kept apart from the clean-room metadata sidecars listed before it.
    let metadata = &text[..text.find("## Player Sidecars").unwrap()];
    assert!(!metadata.contains(SIDECAR_DIR));
}

#[test]
fn the_settings_docs_name_files_that_exist() {
    let text = read_doc("settings.md");
    for path in [MATRIX_FILE, "qa/tools/toggle_matrix.py"] {
        assert!(text.contains(path), "docs/settings.md does not name {path}");
        assert!(workspace_root().join(path).is_file(), "{path} is missing");
    }
}

/// GitHub's anchor for a Markdown heading.
fn anchor(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// Every link from the enhancement docs to another doc names a file and a
/// heading that exist.
#[test]
fn the_enhancement_docs_links_resolve() {
    for name in [
        "settings.md",
        "release-notes.md",
        "commands.md",
        "sidecars.md",
    ] {
        let text = read_doc(name);
        for (index, _) in text.match_indices("](") {
            let target = &text[index + 2..];
            let target = &target[..target.find(')').unwrap()];
            if target.contains("://") || !target.contains(".md") {
                continue;
            }
            let (file, fragment) = target.split_once('#').unwrap_or((target, ""));
            let linked = read_doc(file);
            if fragment.is_empty() {
                continue;
            }
            assert!(
                linked
                    .lines()
                    .filter_map(|line| line.strip_prefix('#'))
                    .map(|line| anchor(line.trim_start_matches('#')))
                    .any(|heading| heading == fragment),
                "docs/{name}: no heading `#{fragment}` in {file}"
            );
        }
    }
    assert_eq!(anchor(" Enhancement Hotkeys"), "enhancement-hotkeys");
}
