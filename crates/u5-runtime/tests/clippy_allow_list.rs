//! Ratchet for the crate-level clippy allow list (Phase B decision D1.15).
//!
//! CI runs clippy with `-D warnings`. The lints `src/lib.rs` allows
//! crate-wide are pinned here so the list can only shrink: adding a lint
//! means editing this test as well, in the same reviewed change, and every
//! allowed lint must carry its reason in the comment above the attribute.

/// The lints `src/lib.rs` allows crate-wide. Remove an entry once the code
/// it covers is fixed; do not add one without a reason in `lib.rs`.
const ALLOWED: &[&str] = &[
    "clippy::too_many_arguments",
    "clippy::type_complexity",
    "clippy::identity_op",
    "clippy::needless_range_loop",
];

const LIB_RS: &str = include_str!("../src/lib.rs");

/// The lint paths named by every crate-level `#![allow(...)]` attribute.
fn crate_level_allows(source: &str) -> Vec<String> {
    let mut lints = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("#![allow(") {
        let after = &rest[start + "#![allow(".len()..];
        let end = after
            .find(")]")
            .expect("crate-level allow attribute is closed");
        lints.extend(
            after[..end]
                .split(',')
                .map(str::trim)
                .filter(|lint| !lint.is_empty())
                .map(str::to_string),
        );
        rest = &after[end..];
    }
    lints
}

#[test]
fn crate_level_clippy_allow_list_only_shrinks() {
    assert_eq!(crate_level_allows(LIB_RS), ALLOWED);
}

#[test]
fn every_allowed_lint_has_a_documented_reason() {
    for lint in ALLOWED {
        let name = lint.trim_start_matches("clippy::");
        assert!(
            LIB_RS
                .lines()
                .any(|line| line.starts_with("// - ") && line.contains(&format!("`{name}`"))),
            "{lint} is allowed crate-wide but lib.rs gives no reason for it"
        );
    }
}

#[test]
fn the_parser_reads_every_entry_of_a_multi_line_attribute() {
    let source = "//! doc\n#![allow(\n    clippy::a,\n    clippy::b\n)]\n#![allow(clippy::c)]\n";
    assert_eq!(
        crate_level_allows(source),
        ["clippy::a", "clippy::b", "clippy::c"]
    );
}
