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

**Do not reseed a scenario whose measured behaviour begins inside a cutscene.**
`systems/blackthorn.md` Section 2 calls the audience and rescue handlers
cinematic: they "replace the ordinary map loop while they run" and hand control
back through an explicit scene transition. A cutscene in progress is not part of
`SAVED.GAM`, so a seed written inside one reloads as an ordinary party standing
in the scene and the cutscene never happens. Seed such a scenario *before* its
trigger and let the script perform it, or leave the walk in place.
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
    ap.add_argument(
        "--resume-after",
        help="rewrite the scenario to start after this shot, dropping the walk",
    )
    ap.add_argument(
        "--contact-opens",
        type=int,
        default=0,
        metavar="KEYS",
        help="the seeded target opens on contact; how many keys reach its prompt",
    )
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

    if args.resume_after:
        rewrite(args.scenario, args.resume_after, dest.name, args.contact_opens)
        print(f"rewrote {args.scenario}.tsv and its seeds.tsv row")


SEEDED_OPENING = """both\twait\t9000
both\tkey\tspace
both\twait\t4000
both\tkey\tj
both\twait\t9000
both\tshot\tseeded\tJourney Onward on the seeded cell
"""

CONTACT_STEP = """both\tkey\tspace
both\twait\t4000
both\tshot\t{label}\t{caption}
"""

# The arms overlay is the one with a pause between its welcome and its entry
# question (`shops.md` §8.B stage 2), so it needs two keys to reach the
# question; every other overlay prints its greeting and waits in one step.
CONTACT_LABELS = {
    1: [("opened", "Contact opens the overlay")],
    2: [
        ("welcome", "Contact opens the overlay: the welcome"),
        ("greeted", "The attribution and the entry question"),
    ],
}

NOTE = """#
# Seeded rather than walked to. The walk-up form of this scenario scripted its
# way in from the town gate, and the resident walks its own schedule while
# those steps run, so the two sides arrived with it in different cells and
# every later beat compared unrelated windows (`cleak/u5-engine#22`).
# `qa/tools/reseed.py` replayed that walk once into the seed named below.
"""

CONTACT_NOTE = """#
# `shops.md` §2: a shopkeeper's "conversation-contact event also reaches it
# automatically during a town schedule pass, even if the player has issued no
# Talk command", so the first Pass beside the keeper opens the overlay and the
# script drives it from there rather than issuing Talk.
"""


def rewrite(name: str, resume_after: str, profile: str, contact_opens: bool) -> None:
    path = ROOT / f"{name}.tsv"
    lines = path.read_text().splitlines()
    header = [line for line in lines if line.startswith("#")]
    body: list[str] = []
    keep = False
    for line in lines:
        parts = line.split("\t")
        if not keep:
            if len(parts) > 2 and parts[1] == "shot" and parts[2] == resume_after:
                keep = True
            continue
        body.append(line)
    header = [line for line in header if not line.startswith("# requires-seed")]
    note = NOTE + (CONTACT_NOTE if contact_opens else "")
    steps = CONTACT_LABELS.get(contact_opens, [])
    opening = SEEDED_OPENING + "".join(
        CONTACT_STEP.format(label=label, caption=caption) for label, caption in steps
    )
    requires = f"# requires-seed: profile `{profile}`, built by qa/tools/reseed.py\n"
    path.write_text(
        "\n".join(header) + "\n" + note + requires + opening + "\n".join(body) + "\n"
    )

    table = ROOT / "seeds.tsv"
    rows = []
    for line in table.read_text().splitlines():
        if line.startswith(f"{name}\t"):
            line = f"{name}\t{profile}\tthe walk-up replayed once by reseed.py"
        rows.append(line)
    table.write_text("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()
