#!/usr/bin/env python3
"""Turn a paired scenario's scripted walk into a seed save.

A long walk-up is the single biggest source of unmeasurable paired beats
(`cleak/u5-engine#22`): town residents move on their own schedules, so sixteen
scripted steps put the two sides in front of different cells and every beat
after that compares unrelated windows. The fix is to walk once, save, and let
both sides load the result.

This replays a scenario's leading keys through the engine's `--write-seed`
writer and leaves a new seed profile behind. It prints the keys it consumed so
the scenario can be trimmed to match; it does not edit the scenario, because
which beats survive the trim is a judgement call.

Usage:
    reseed.py <SCENARIO> --through <BEAT> [--beside 0xNN] [--name <PROFILE>]

`--through` names the last shot to replay past; every key before it is consumed
into the seed. `--beside` additionally places the party next to the NPC
carrying that `.NPC` dialog byte, for a scenario whose target walks its own
schedule and so may not be where the walk aimed.
"""

import argparse
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1] / "paired"
PROFILES = pathlib.Path.home() / ".local/share/u5/engine"
ENGINE = pathlib.Path(__file__).resolve().parents[2]

# The intro keys reach Journey Onward, which is how the *seed* is loaded in the
# first place; replaying them into the seed would be circular.
INTRO_KEYS = ("space", "j")

# Paired scenarios name arrow keys; the play script takes the legacy letters.
# `Pass` is Space or Enter, and the script's token for it is `return`, because
# an empty script command is filtered out before it reaches the harness.
KEY_TOKENS = {
    "Left": "a",
    "Right": "d",
    "Up": "w",
    "Down": "s",
    "space": "return",
}


def scenario_keys(name: str, through: str) -> list[str]:
    keys: list[str] = []
    seen = False
    for line in (ROOT / f"{name}.tsv").read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        if parts[1] == "key" and len(parts) > 2:
            keys.append(parts[2])
        elif parts[1] == "shot" and len(parts) > 2 and parts[2] == through:
            seen = True
            break
    if not seen:
        raise SystemExit(f"scenario `{name}` has no shot named `{through}`")
    while keys and keys[0] in INTRO_KEYS:
        keys.pop(0)
    return keys


def seed_profile(name: str) -> str:
    for line in (ROOT / "seeds.tsv").read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) >= 2 and parts[0] == name:
            return parts[1]
    raise SystemExit(f"`{name}` has no seeds.tsv row to start from")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario")
    ap.add_argument("--through", required=True)
    ap.add_argument("--beside")
    ap.add_argument("--name")
    args = ap.parse_args()

    keys = scenario_keys(args.scenario, args.through)
    script = ";".join(KEY_TOKENS.get(key, key) for key in keys)
    source = PROFILES / seed_profile(args.scenario)
    if not source.is_dir():
        raise SystemExit(f"seed profile {source} is absent")

    work = PROFILES / f"reseed-work-{args.scenario}"
    shutil.rmtree(work, ignore_errors=True)
    subprocess.run(
        ["game-dev-u5", "prepare-profile", "engine", work.name],
        check=True,
        capture_output=True,
    )
    for stem in ("SAVED.GAM", "SAVED.OOL"):
        shutil.copy(source / stem, work / stem)

    cmd = [
        "cargo", "run", "--release", "-q", "-p", "u5-tui", "--",
        "--write-seed", "--from-save",
    ]
    if args.beside:
        cmd += ["--seed-beside", args.beside]
    if script:
        cmd += ["--play-script", script]
    cmd.append(str(work))
    run = subprocess.run(
        cmd,
        cwd=ENGINE,
        capture_output=True,
        text=True,
        env={**__import__("os").environ,
             "CARGO_TARGET_DIR": str(pathlib.Path.home() / ".cache/u5-alt-target")},
    )
    if run.returncode != 0:
        print(run.stderr.strip()[-1500:], file=sys.stderr)
        raise SystemExit("seed write failed")
    print(run.stdout.strip())

    dest = PROFILES / (args.name or f"seed-{args.scenario}-reseeded")
    shutil.rmtree(dest, ignore_errors=True)
    dest.mkdir(parents=True)
    for stem in ("SAVED.GAM", "SAVED.OOL"):
        shutil.copy(work / stem, dest / stem)
    shutil.rmtree(work, ignore_errors=True)
    print(f"seed profile: {dest.name}")
    print(f"consumed {len(keys)} key(s): {' '.join(keys)}")


if __name__ == "__main__":
    main()
