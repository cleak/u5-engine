#!/usr/bin/env python3
"""Run the paired scenario suite and report which scenarios compared.

`game-dev-u5-paired` runs **one** scenario and needs `--seed-save` for any
scenario carrying a `# requires-seed:` header. Which profile holds that seed
was tribal knowledge: `qa/paired/README.md` says what the seed must *contain*,
not where it lives, and pointing the harness at the wrong one silently
"passes" while comparing nothing - eight runs of `paws-tavern` compared an
in-town save's Talk refusals before I noticed.

This resolves the seed from `qa/paired/seeds.tsv`, runs the scenarios named on
the command line (or every scenario), and prints one line each.

`seeds.tsv` is tab separated:

    scenario<TAB>seed-profile<TAB>note

A scenario with no row and no `# requires-seed:` header runs unseeded. A
scenario with the header and no row is reported as unrunnable rather than run
against nothing.

Usage: paired_suite.py [--engine-dir DIR] [--list] [scenario...]
"""

import argparse
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1] / "paired"
PROFILES = pathlib.Path.home() / ".local/share/u5/engine"


def seeds() -> dict[str, tuple[str, str]]:
    table = ROOT / "seeds.tsv"
    out: dict[str, tuple[str, str]] = {}
    if not table.exists():
        return out
    for line in table.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        out[parts[0]] = (parts[1], parts[2] if len(parts) > 2 else "")
    return out


def needs_seed(path: pathlib.Path) -> bool:
    return "# requires-seed" in path.read_text()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine-dir", default=".")
    parser.add_argument("--list", action="store_true")
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="skip the visual rebuild; only safe when nothing else has built since",
    )
    parser.add_argument("scenarios", nargs="*")
    args = parser.parse_args()

    table = seeds()
    # `seeds.tsv` lives in the same directory and is not a scenario.
    names = args.scenarios or sorted(
        p.stem for p in ROOT.glob("*.tsv") if p.name != "seeds.tsv"
    )
    runnable, blocked = [], []
    for name in names:
        path = ROOT / f"{name}.tsv"
        if not path.exists():
            blocked.append((name, "no such scenario"))
            continue
        if not needs_seed(path):
            runnable.append((name, None))
            continue
        row = table.get(name)
        if row is None:
            blocked.append((name, "no seeds.tsv row"))
            continue
        profile = PROFILES / row[0]
        if not profile.is_dir():
            blocked.append((name, f"seed profile {row[0]} is absent"))
            continue
        runnable.append((name, profile))

    if args.list:
        for name, profile in runnable:
            print(f"run   {name}\t{profile.name if profile else '(unseeded)'}")
        for name, why in blocked:
            print(f"skip  {name}\t{why}")
        print(f"\n{len(runnable)} runnable, {len(blocked)} blocked")
        return

    # The harness runs the `u5-engine` binary, and that binary is only the
    # windowed shell when it was built with `--features visual`. Any plain
    # `cargo build --release` in the same tree overwrites it with the headless
    # one, and the next run then fails with `window matching '^Ultima V' did
    # not appear within 60s` - which reads like a harness fault. Rebuilding
    # here makes the suite own that, instead of every caller remembering it.
    if not args.no_build:
        build = subprocess.run(
            ["cargo", "build", "--release", "--features", "visual"],
            cwd=args.engine_dir,
            capture_output=True,
            text=True,
        )
        if build.returncode != 0:
            print(build.stderr.strip()[-2000:])
            raise SystemExit("visual build failed; not running the suite")

    failures = 0
    for name, profile in runnable:
        cmd = ["game-dev-u5-paired", "--engine-dir", args.engine_dir]
        if profile is not None:
            cmd += ["--seed-save", str(profile)]
        cmd.append(str(ROOT / f"{name}.tsv"))
        result = subprocess.run(cmd, capture_output=True, text=True)
        # The harness prints its artifact directory among a JSON tail; take
        # the last path under the artifact root rather than the last line.
        paths = re.findall(r"/[\w./-]*artifacts/u5/paired/[\w.-]+", result.stdout)
        artifact = paths[-1] if paths else ""
        status = "ok  " if result.returncode == 0 else "FAIL"
        if result.returncode != 0:
            failures += 1
        print(f"{status} {name}\t{artifact}", flush=True)
    for name, why in blocked:
        print(f"skip {name}\t{why}", flush=True)
    print(f"\n{len(runnable)} run, {failures} failed, {len(blocked)} blocked")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
