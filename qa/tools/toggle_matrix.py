#!/usr/bin/env python3
"""Run the enhancement toggle matrix on game-dev-server (Phase B design 1.3.12).

`u5-engine --equivalence-suite OUT --matrix qa/enhancements/matrix.tsv GAME_DIR`
runs every route-smoke case under Classic and under each matrix row, and
compares each with the all-off run. This wrapper:

1. prepares a fresh engine profile with `game-dev-u5 prepare-profile --replace
   engine NAME` (or uses `--game-dir`), so no earlier run's files leak in;
2. runs the suite with the release `u5-engine` binary and times it;
3. reads `equivalence-report.tsv`: every matrix row must be reported, and
   every line must say `equal`;
4. checks Classic's route-smoke manifest against the 1.0.1 digest (the
   manifest's sha256, first 16 hex);
5. fails when the whole run takes longer than the budget (ten minutes, the
   design's limit for the full matrix).

It writes `toggle-matrix-summary.json` beside the report and prints every
difference. Nothing here builds the engine; build it first with
`cargo build --release -p u5-tui --features visual --bins`.

Usage:
  python3 qa/tools/toggle_matrix.py [--engine PATH] [--profile NAME |
      --game-dir DIR] [--matrix FILE] [--out DIR] [--budget-seconds N]
      [--expect-route-digest HEX]
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time
from dataclasses import dataclass, field

REPO = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_MATRIX = REPO / "qa" / "enhancements" / "matrix.tsv"
REPORT_FILE = "equivalence-report.tsv"
CLASSIC_MANIFEST_FILE = "classic-route-smoke-manifest.tsv"
SUMMARY_FILE = "toggle-matrix-summary.json"
CLASSIC = "classic"
# The route-smoke manifest digest of the 1.0.1 game (M0-01 baseline).
ROUTE_DIGEST_1_0_1 = "22254c83f2af767b"
# Design 1.3.12 / plan M0-19: the full matrix runs in under ten minutes.
BUDGET_SECONDS = 600


def matrix_row_names(text: str) -> list[str]:
    """The row names of a matrix file, in order (comments and blanks skipped)."""
    names = []
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        names.append(line.split("\t", 1)[0].strip())
    return names


@dataclass
class ReportSummary:
    """What an equivalence report says."""

    cases: set[str] = field(default_factory=set)
    configurations: list[str] = field(default_factory=list)
    results: dict[str, int] = field(default_factory=dict)
    differences: list[str] = field(default_factory=list)

    def to_json(self) -> dict:
        return {
            "cases": len(self.cases),
            "configurations": self.configurations,
            "results": self.results,
            "differences": self.differences,
        }


def summarize_report(text: str) -> ReportSummary:
    """Counts each result and lists every line that is not `equal`."""
    summary = ReportSummary()
    for line in text.splitlines():
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) < 3:
            summary.differences.append(f"malformed report line: {line}")
            continue
        case, configuration, result = fields[0], fields[1], fields[2]
        detail = fields[6] if len(fields) > 6 else ""
        summary.cases.add(case)
        if configuration not in summary.configurations:
            summary.configurations.append(configuration)
        summary.results[result] = summary.results.get(result, 0) + 1
        if result != "equal":
            summary.differences.append(f"{case} {configuration}: {result} {detail}".strip())
    return summary


def manifest_digest(path: pathlib.Path) -> str | None:
    """The route-smoke manifest digest: sha256 of the file, first 16 hex."""
    if not path.is_file():
        return None
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def check(
    summary: ReportSummary,
    expected_rows: list[str],
    digest: str | None,
    expected_digest: str | None,
    seconds: float,
    budget: float,
    exit_code: int,
) -> list[str]:
    """Every reason the run fails; empty when it passes."""
    problems = []
    if exit_code != 0:
        problems.append(f"the suite exited with {exit_code}")
    for name in [CLASSIC, *expected_rows]:
        if name not in summary.configurations:
            problems.append(f"configuration {name} is missing from the report")
    if not summary.cases:
        problems.append("the report lists no case")
    problems.extend(summary.differences)
    if expected_digest is not None and digest != expected_digest:
        problems.append(
            f"Classic route-smoke manifest digest {digest} is not {expected_digest}"
        )
    if seconds > budget:
        problems.append(f"the matrix took {seconds:.0f} s, over the {budget:.0f} s budget")
    return problems


def default_engine() -> pathlib.Path:
    target = os.environ.get("CARGO_TARGET_DIR")
    base = pathlib.Path(target) if target else REPO / "target"
    return base / "release" / "u5-engine"


def prepare_profile(name: str) -> pathlib.Path:
    """A fresh writable copy of the game from game-dev-u5."""
    result = subprocess.run(
        ["game-dev-u5", "prepare-profile", "--replace", "engine", name],
        check=True,
        capture_output=True,
        text=True,
    )
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    if not lines:
        raise SystemExit("game-dev-u5 prepare-profile printed no profile path")
    return pathlib.Path(lines[-1].strip())


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--engine", type=pathlib.Path, default=default_engine())
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--profile", default="toggle-matrix")
    source.add_argument("--game-dir", type=pathlib.Path)
    parser.add_argument("--matrix", type=pathlib.Path, default=DEFAULT_MATRIX)
    parser.add_argument("--out", type=pathlib.Path)
    parser.add_argument("--budget-seconds", type=float, default=BUDGET_SECONDS)
    parser.add_argument("--expect-route-digest", default=ROUTE_DIGEST_1_0_1)
    args = parser.parse_args(argv)

    if not args.engine.is_file():
        raise SystemExit(
            f"no engine at {args.engine}; build it with "
            "`cargo build --release -p u5-tui --features visual --bins`"
        )
    expected_rows = matrix_row_names(args.matrix.read_text())
    game_dir = args.game_dir or prepare_profile(args.profile)
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = args.out or pathlib.Path.home() / "artifacts" / "u5" / "toggle-matrix" / stamp
    out.mkdir(parents=True, exist_ok=True)

    command = [
        str(args.engine),
        "--equivalence-suite",
        str(out),
        "--matrix",
        str(args.matrix),
        str(game_dir),
    ]
    print(" ".join(command), flush=True)
    started = time.monotonic()
    with open(out / "suite.log", "w") as log:
        exit_code = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT).returncode
    seconds = time.monotonic() - started

    report = out / REPORT_FILE
    summary = summarize_report(report.read_text() if report.is_file() else "")
    digest = manifest_digest(out / CLASSIC_MANIFEST_FILE)
    problems = check(
        summary,
        expected_rows,
        digest,
        args.expect_route_digest or None,
        seconds,
        args.budget_seconds,
        exit_code,
    )
    record = {
        "engine": str(args.engine),
        "game_dir": str(game_dir),
        "matrix": str(args.matrix),
        "matrix_rows": len(expected_rows),
        "seconds": round(seconds, 1),
        "budget_seconds": args.budget_seconds,
        "classic_manifest_digest": digest,
        "expected_route_digest": args.expect_route_digest or None,
        "exit_code": exit_code,
        "report": summary.to_json(),
        "problems": problems,
        "passed": not problems,
    }
    (out / SUMMARY_FILE).write_text(json.dumps(record, indent=2) + "\n")

    print(
        f"{len(summary.cases)} case(s) x {len(summary.configurations)} configuration(s) "
        f"in {seconds:.0f} s; results {summary.results}; Classic manifest {digest}"
    )
    for problem in problems:
        print(f"  {problem}")
    print(f"{'PASS' if not problems else 'FAIL'}: {out / SUMMARY_FILE}")
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
