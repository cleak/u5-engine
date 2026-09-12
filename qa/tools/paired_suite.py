#!/usr/bin/env python3
"""Run the paired scenario suite and report how each scenario compared.

`game-dev-u5-paired` runs **one** scenario and needs `--seed-save` for any
scenario carrying a `# requires-seed:` header. Which profile holds that seed
was tribal knowledge: `qa/paired/README.md` says what the seed must *contain*,
not where it lives, and pointing the harness at the wrong one silently
"passes" while comparing nothing - eight runs of `paws-tavern` compared an
in-town save's Talk refusals before I noticed.

This resolves the seed from `qa/paired/seeds.tsv`, runs the scenarios named on
the command line (or every scenario), and prints one line each.

**The status word is the comparison, not the run.** An earlier version printed
`ok` as soon as the run completed, and `ok` reads as "the two sides agree" - it
does not mean that, and on 2026-09-12 a whole session of reports called
scenarios clean on the strength of it. Each artifact is now handed to
`paired_compare.py` and its classification is what gets printed: `match` when
every beat agrees, `DIFFER` with the count when they do not, `RERUN` when the
run measured nothing, and `FAIL` when the run itself did not produce frames.

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


def compare(artifact: str) -> tuple[str, str]:
    """Classify one artifact with `paired_compare.py`.

    Returns the leading word of its summary line - `match`, `DIFFER` or
    `RERUN` - and the counts that follow it. A comparison that cannot run at
    all is reported rather than swallowed, because a silent pass here is the
    exact failure this wrapper exists to prevent.
    """
    tool = pathlib.Path(__file__).resolve().parent / "paired_compare.py"
    if not artifact:
        return "RERUN", "no artifact directory"
    result = subprocess.run(
        [sys.executable, str(tool), artifact],
        capture_output=True,
        text=True,
    )
    for line in reversed(result.stdout.splitlines()):
        for word in ("match", "DIFFER", "RERUN"):
            if line.startswith(word):
                return word, line.split(":", 1)[-1].strip()
    return "RERUN", (result.stderr.strip().splitlines() or ["no summary line"])[-1]


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
        # `game-dev-u5-paired` exits zero when the *run* completed, and a run
        # whose engine side never launched completes in seconds with no shots
        # at all. That is how a whole suite pass once produced 30 artifacts
        # carrying nothing but a DOSBox video: a plain `cargo build --release`
        # elsewhere in the tree had replaced the windowed binary, every engine
        # process died on `--visual-playable ... requires building with
        # --features visual`, and the suite still printed `ok`. A scenario that
        # asks for shots and produces none is a failed run.
        wanted_shots = any(
            line.split("\t")[1:2] == ["shot"]
            for line in (ROOT / f"{name}.tsv").read_text().splitlines()
        )
        captured = bool(artifact) and any(
            pathlib.Path(artifact).glob("engine-*.png")
        )
        launched = result.returncode == 0 and (captured or not wanted_shots)
        if not launched:
            failures += 1
            if result.returncode == 0:
                artifact = f"{artifact}\t(no engine capture; is the visual build current?)"
            print(f"FAIL {name}\t{artifact}", flush=True)
            continue
        verdict, detail = compare(artifact)
        if verdict != "match":
            failures += 1
        print(f"{verdict:<6} {name}\t{artifact}\t{detail}", flush=True)
    for name, why in blocked:
        print(f"skip {name}\t{why}", flush=True)
    print(
        f"\n{len(runnable)} run, {failures} not matching, {len(blocked)} blocked"
    )
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
