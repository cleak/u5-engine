#!/usr/bin/env bash
# Warning-line gate: build with every rustc warning denied, then fail on any
# output line that still reads as a warning.
#
# `RUSTFLAGS=-Dwarnings` turns rustc warnings into errors, but it does not
# touch lines a build script prints with `cargo:warning=` or warnings cargo
# itself reports (an unused manifest key, a profile it ignores). Those still
# reach the log as `warning: ...` and the build still succeeds, so this script
# scans the combined output for them as well.
#
# Usage, from the repository root:
#
#   qa/tools/check_no_build_warnings.sh                 # the default builds
#   qa/tools/check_no_build_warnings.sh -p u5-runtime   # one custom build
#
# With no arguments it runs the two builds CI gates: the whole workspace with
# all targets, and `-p u5-tui --features visual` with all targets. With
# arguments it runs one `cargo build --locked <args>` instead. `CARGO` names
# the cargo to run (default `cargo`); `CARGO_TARGET_DIR` and the rest of the
# environment pass through unchanged.
#
# Exit status: 0 when every build succeeds with no warning line, 1 when a
# build fails or a warning line is found.

set -uo pipefail

cargo_bin="${CARGO:-cargo}"

# A warning line: `warning:` or `warning[CODE]:` at the start, after optional
# indentation. `cargo:warning=` lines from build scripts reach the log in this
# form too, as `warning: <crate>@<version>: <text>`.
warning_line='^[[:space:]]*warning(\[[^]]*\])?:'

# Cargo's own note when it stops after a failed crate. The failure is already
# reported by the build status, so this line is not listed as a warning.
build_failed_note='^[[:space:]]*warning: build failed, waiting for other jobs to finish'

# CI sets CARGO_TERM_COLOR=always, which wraps `warning` in colour codes; they
# are stripped before matching so a coloured log is judged like a plain one.
strip_colour() {
  sed -E $'s/\x1b\\[[0-9;]*[A-Za-z]//g' "$1"
}

warning_lines() {
  strip_colour "$1" | grep -En "$warning_line" | grep -Ev "^[0-9]+:${build_failed_note#^}"
}

status=0
log=$(mktemp)
trap 'rm -f "$log"' EXIT

run_build() {
  echo "==> RUSTFLAGS=-Dwarnings $cargo_bin build --locked $*"
  RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Dwarnings" \
    "$cargo_bin" build --locked "$@" 2>&1 | tee "$log"
  local build_status=${PIPESTATUS[0]}
  if [[ $build_status -ne 0 ]]; then
    echo "::error::cargo build --locked $* failed with -Dwarnings (exit $build_status)"
    status=1
  fi
  if warning_lines "$log" >/dev/null; then
    echo "::error::cargo build --locked $* printed warning lines:"
    warning_lines "$log"
    status=1
  fi
}

if [[ $# -gt 0 ]]; then
  run_build "$@"
else
  run_build --workspace --all-targets
  run_build -p u5-tui --features visual --all-targets
fi

if [[ $status -eq 0 ]]; then
  echo "No build warnings."
fi
exit "$status"
