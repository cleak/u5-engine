# QA Scenario Registry

`qa/scenarios.tsv` is the versioned, machine-readable scenario registry
required by the clean remote-development plan. It is the system of record for
stable reproduction inputs, expected sanitized results, required lanes and
platforms, and the public specification revision each expectation was reviewed
against. GitHub issues remain the queue for unfinished work; `TODO.md` and the
status matrix are derived snapshots.

The registry contains only identifiers, commands, hashes of sanitized
manifests, and clean prose. It never contains asset bytes, frame images,
dialogue, or private analysis.

## Columns

| Column | Meaning |
|---|---|
| `id` | Stable lowercase dotted identifier, e.g. `suite.route-smoke`. |
| `subsystem` | Engine area under test. |
| `frontend` | `runtime`, `tui`, `bevy`, `dosbox`, or `host`. |
| `lanes` | `\|`-separated lanes: `source`, `asset`, `bevy`, `dosbox`. |
| `platforms` | `\|`-separated: `windows`, `linux`, `steamos`. |
| `verification` | `automated` or `human`. |
| `spec_path` | Public spec path, or `*` for cross-cutting suites. |
| `spec_commit` | Public spec commit the expectation was reviewed against. |
| `setup` | Engine command with `{profile}` and `{out}` placeholders, or a `procedure:` id for human checks. |
| `expected_kind` | `manifest-sha256`, `exit-zero`, or `human-checklist`. |
| `expected_value` | Sanitized manifest SHA-256, `0`, or a checklist id. |
| `baseline_engine_commit` | Engine commit that produced `expected_value`. |
| `notes` | Clean prose. |

`crates/u5-tui/tests/scenario_registry.rs` validates the file shape, id
uniqueness, lane/platform vocabularies, hash formats, and that every automated
engine command still parses with the current CLI.

## Updating expectations

Deterministic manifest hashes are the authoritative automation signal. When a
change intentionally alters a suite manifest:

1. run the suite from a complete writable asset copy on the integration
   candidate;
2. confirm a second run at the same commit reproduces the hash byte for byte;
3. update `expected_value`, `baseline_engine_commit`, and, if the spec moved,
   `spec_commit`; and
4. describe the behavioral reason in the commit message.

A hash that differs at the same `baseline_engine_commit` is a determinism or
harness defect, not a new baseline. Scenarios tied to a changed spec document
must be re-reviewed against the new spec commit before their expectation is
accepted.
