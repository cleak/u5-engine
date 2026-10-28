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
import json
import os
import pathlib
import re
import signal
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1] / "paired"
PROFILES = pathlib.Path.home() / ".local/share/u5/engine"


def run_scenario(cmd: list[str], env: dict | None = None) -> subprocess.CompletedProcess:
    """Run one scenario in its own process group, and take the group down
    with us.

    `game-dev-u5-paired` spawns a screenshot loop that grabs a frame twice
    a second and a DOSBox alongside it. Killing the suite left all three
    running: measured 2026-09-19, two loops from runs that had ended the
    previous day were still writing, 8 GB between them, and a third was
    left behind by a run stopped earlier the same night.

    A new session makes the children killable as a unit, and the handlers
    make sure they are killed on the two signals a stopped suite actually
    receives - including a `KeyboardInterrupt`, which reaches the parent
    before `subprocess.run` can propagate anything.
    """
    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=True,
        env=env,
    )

    def stop(_signum=None, _frame=None):
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass

    previous = {
        number: signal.signal(number, lambda s, f: (stop(), sys.exit(128 + s)))
        for number in (signal.SIGINT, signal.SIGTERM)
    }
    try:
        stdout, stderr = process.communicate()
    except BaseException:
        stop()
        raise
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)
        # A scenario that returned still leaves the loop and the emulator
        # behind if the harness itself died mid-run.
        stop()
    return subprocess.CompletedProcess(cmd, process.returncode, stdout, stderr)


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


def scenario_fixed_clock(path: pathlib.Path) -> str | None:
    """The `# fixed-clock: HH:MM:SS.CC` a scenario pins, if any.

    A scenario anchored on a published re-seed (`prng.md` Section 3) expects
    one particular outcome, and which one depends on the clock the re-seed
    reads. It names that clock itself so a suite run cannot measure it at
    another; the header wins over `--fixed-clock` for that scenario.
    """
    match = re.search(r"^# fixed-clock:\s*(\d\d:\d\d:\d\d\.\d\d)\s*$", path.read_text(), re.M)
    return match.group(1) if match else None


EMULATOR_LOAD_LATENCY_ENV = "U5_EMULATOR_LOAD_LATENCY_MS"


def scenario_emulator_load_latency(path: pathlib.Path) -> str | None:
    """The `# emulator-load-latency: MS` a scenario pins, if any.

    `intro.md` Section 7, "Timing of the load path": the stock game's
    Journey Onward is untimed work that DOSBox at 3000 cycles takes about
    216 ms over after the border appears (plus about 1.7 s when the tile
    archive loads), while the engine loads at once, so an idle-draw count
    taken after a load starts a few draws apart. "A parity harness that
    counts idle draws against this emulator may model the gap as an optional
    emulator-profile latency between the key read and the first idle pass.
    ... It is a test setting, not a game rule, and must never be expressed
    as ticks." Whole milliseconds; the header wins over
    `--emulator-load-latency` for that scenario, as `# fixed-clock:` does.
    """
    match = re.search(r"^# emulator-load-latency:\s*(\d+)\s*$", path.read_text(), re.M)
    return match.group(1) if match else None


def scenario_environment(path: pathlib.Path, base: dict | None = None) -> tuple[dict | None, list[str]]:
    """The environment a scenario runs under, and a note per header that set it.

    `None` when no header pins anything, so the run inherits the suite's own
    environment (and with it any `--fixed-clock` or `--emulator-load-latency`).
    """
    environ = dict(os.environ if base is None else base)
    notes = []
    pinned = scenario_fixed_clock(path)
    if pinned is not None:
        environ["U5_FIXED_HOST_CLOCK"] = pinned
        notes.append(f"clock {path.stem}\tfixed at {pinned} by its header")
    latency = scenario_emulator_load_latency(path)
    if latency is not None:
        environ[EMULATOR_LOAD_LATENCY_ENV] = latency
        notes.append(f"load  {path.stem}\temulator load latency {latency} ms by its header")
    return (environ if notes else None), notes


def captures_are_usable(artifact: str) -> bool:
    """Did this run capture the game frame, or something else?

    DOSBox is normally captured at its native 640x400, frame-filling. Some
    runs catch the window before it has taken that mode and produce a
    1920x1080 desktop shot with the frame in a small off-centre region.
    Every cell then decodes from the wrong pixels, and the result is not an
    error but a finding: blank message windows that score as `stock-idle`
    and a viewport that differs by the same large count on every beat.

    Measured 2026-09-18: four scenarios in one full pass and six in the
    next, each with *every* beat unusable. Two false defect reports were
    filed off the first batch before the cause was found
    (`cleak/u5-engine#39`).

    `paired_compare` refuses to decode them, which is the right floor, but a
    scenario that decodes nothing has measured nothing - so the suite retries
    it once rather than reporting a verdict it does not have.
    """
    if not artifact:
        return False
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    from PIL import Image

    from paired_compare import capture_is_frame_filling

    shots = sorted(pathlib.Path(artifact).glob("dosbox-*.png"))
    if not shots:
        return True
    return all(capture_is_frame_filling(Image.open(shot)) for shot in shots)


def compare(artifact: str) -> tuple[str, str]:
    """Classify one artifact with `paired_compare.py`.

    Returns the leading word of its summary line - `match`, `DIFFER`,
    `RERUN` or `PROBE` - and the counts that follow it. A comparison that
    cannot run at all is reported rather than swallowed, because a silent
    pass here is the exact failure this wrapper exists to prevent.

    `PROBE` has to be in that list. A scenario declaring
    `# not-a-comparison:` drives one side on purpose, and
    `paired_compare.is_probe` reports it as `PROBE` precisely so it
    "neither counts as a difference nor pretends to be a match". This
    wrapper did not know the word, so it fell through to the
    no-summary-line arm and reported four such scenarios as `RERUN`
    every pass - three of the 2026-09-19 pass's 28.
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
        for word in ("match", "DIFFER", "RERUN", "PROBE"):
            if line.startswith(word):
                return word, line.split(":", 1)[-1].strip()
    return "RERUN", (result.stderr.strip().splitlines() or ["no summary line"])[-1]


def key_skew_note(artifact: str) -> str:
    """The harness's measured per-key delivery skew, when it recorded one.

    With seed control both sides start from the same roll stream, and the
    original draws once per idle world pass, so how far apart the two sides
    received each key is part of whether a drawn pool row can agree at all.
    """
    try:
        record = json.loads((pathlib.Path(artifact) / "record.json").read_text())
    except (OSError, ValueError):
        return ""
    skew = record.get("key_skew") or {}
    if not skew.get("paired_keys"):
        return ""
    return (
        f"\tkey skew {record.get('key_sync', '?')}: max {skew['max_abs_skew_ms']:.0f} ms, "
        f"mean {skew['mean_abs_skew_ms']:.0f} ms over {skew['paired_keys']} keys"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine-dir", default=".")
    parser.add_argument("--list", action="store_true")
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="skip the visual rebuild; only safe when nothing else has built since",
    )
    parser.add_argument(
        "--fixed-clock",
        metavar="HH:MM:SS.CC",
        help=(
            "seed control: both sides read this DOS time-of-day at every PRNG "
            "seeding event (prng.md 3), so their roll streams can be compared "
            "draw for draw. Sets U5_FIXED_HOST_CLOCK, which the clean engine "
            "reads and the RemoteGameDev DOSBox adapter answers with a resident "
            "time-of-day hook."
        ),
    )
    parser.add_argument(
        "--key-sync",
        choices=("serial", "sendevent"),
        help=(
            "how the harness hands `both` keys to the two sides. `serial` (the "
            "harness default) focuses each window in turn, so DOSBox gets every "
            "key about 1.3 s after the engine - several idle-pass draws of the "
            "gameplay stream (prng.md 4). `sendevent` keeps the engine focused "
            "and gives DOSBox the same key in the same instant. Sets "
            "U5_PAIRED_KEY_SYNC for `game-dev-u5-paired`."
        ),
    )
    parser.add_argument(
        "--reaction-probe",
        type=int,
        metavar="MS",
        help=(
            "record when each side's frame reacted to every `both` key, sampling "
            "for up to MS milliseconds. Sets U5_PAIRED_REACTION_PROBE_MS."
        ),
    )
    parser.add_argument(
        "--emulator-load-latency",
        type=int,
        metavar="MS",
        help=(
            "engine side only: pause MS wall-clock milliseconds between a "
            "title-menu load's key read and the first idle pass, modelling "
            "the stock game's untimed load work under DOSBox (intro.md 7, "
            "'a test setting, not a game rule'). Sets "
            "U5_EMULATOR_LOAD_LATENCY_MS, which only the clean engine reads. "
            "No default; about 280 fits fixed 3000 cycles with a preview "
            "already run."
        ),
    )
    parser.add_argument("scenarios", nargs="*")
    args = parser.parse_args()
    if args.fixed_clock:
        # Inherited by `game-dev-u5-paired`, and from it by both sides.
        os.environ["U5_FIXED_HOST_CLOCK"] = args.fixed_clock
    if args.key_sync:
        os.environ["U5_PAIRED_KEY_SYNC"] = args.key_sync
    if args.reaction_probe:
        os.environ["U5_PAIRED_REACTION_PROBE_MS"] = str(args.reaction_probe)
    if args.emulator_load_latency is not None:
        if args.emulator_load_latency < 0:
            parser.error("--emulator-load-latency must be zero or more milliseconds")
        # Inherited by `game-dev-u5-paired` and from it by both launched
        # sides, but only the clean engine reads it; DOSBox has no use for
        # it and the stock side keeps its own load time.
        os.environ[EMULATOR_LOAD_LATENCY_ENV] = str(args.emulator_load_latency)

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
        env, notes = scenario_environment(ROOT / f"{name}.tsv")
        for note in notes:
            print(note, flush=True)
        result = run_scenario(cmd, env)
        artifact_of = lambda out: (
            re.findall(r"/[\w./-]*artifacts/u5/paired/[\w.-]+", out) or [""]
        )[-1]
        # One retry for a run whose captures were not the game frame. See
        # `captures_are_usable`.
        if result.returncode == 0 and not captures_are_usable(artifact_of(result.stdout)):
            print(f"recap {name}\tcaptures were not the game frame; re-running", flush=True)
            result = run_scenario(cmd, env)
        # A walk-in town scenario that disagreed may have met NPC traffic
        # rather than a defect: whichever side is refused a step arrives in
        # a different cell and everything after it diverges. Those agree
        # most of the time, so one retry turns a false difference back into
        # the match it is, and a real one survives both runs. See
        # `paired_compare.is_traffic_variable`.
        if result.returncode == 0:
            sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
            from paired_compare import is_traffic_variable

            if is_traffic_variable(name):
                first, _counts = compare(artifact_of(result.stdout))
                # A probe never agrees by construction, so retrying one
                # spends two minutes to reach the same non-verdict.
                # `town-entry-walk-probe` is both traffic-variable and a
                # probe, and was re-run every pass on that basis.
                if first not in ("match", "PROBE"):
                    print(
                        f"retry {name}\tread {first} on a walk-in route; re-running",
                        flush=True,
                    )
                    result = run_scenario(cmd, env)
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
        if launched and wanted_shots and not captures_are_usable(artifact):
            failures += 1
            print(
                f"BADCAP {name}\t{artifact}\tcaptures were not the game frame twice; "
                "measured nothing",
                flush=True,
            )
            continue
        if not launched:
            failures += 1
            if result.returncode == 0:
                artifact = f"{artifact}\t(no engine capture; is the visual build current?)"
            print(f"FAIL {name}\t{artifact}", flush=True)
            continue
        verdict, detail = compare(artifact)
        # A probe is not a verdict in either direction, so it is neither a
        # failure nor a match.
        if verdict not in ("match", "PROBE"):
            failures += 1
        print(f"{verdict:<6} {name}\t{artifact}\t{detail}{key_skew_note(artifact)}", flush=True)
    for name, why in blocked:
        print(f"skip {name}\t{why}", flush=True)
    print(
        f"\n{len(runnable)} run, {failures} not matching, {len(blocked)} blocked"
    )
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
