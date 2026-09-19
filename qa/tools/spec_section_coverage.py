#!/usr/bin/env python3
"""Report specification sections the engine never cites.

`cleak/u5-engine#2` audited all ~85 published documents and found 106
contracts the engine did not implement, but the machine-readable finding
set did not survive the run, so the issue's remaining claims cannot be
re-read. This rebuilds an enumerable stand-in from the two things that do
survive: the published documents and the engine's own citations.

Every behavioural slice in this engine cites the sentence it implements -
`systems/combat.md §14`, `inventory.md Section 7`, `text-output.md §10.4`.
So a published section that no engine comment cites anywhere is a
*candidate* gap: either nothing implements it, or something does and
forgot to say so.

This is a reading list, never a defect list. Headings that carry no
contract (overviews, tables of contents, provenance appendices) are
uncited for good reason, and a section may be implemented under a
neighbouring section's citation. Every hit needs a read.

Documents are fetched from spec HEAD, never from the local checkout,
which is `cleak/u5-engine#2`'s own process lesson.

Bare `§8.2` citations are attributed to the last document named in the
same file, which is how this repository writes them once a nearby
comment has already given the path.

Usage: spec_section_coverage.py [--doc systems/combat.md] [--all]
"""

import json
import pathlib
import re
import subprocess
import sys

SPEC_REPO = "cleak/u5-spec"
HEADING = re.compile(r"^(#{2,4})\s+(?:Section\s+)?(\d+(?:\.\d+)*)[.)]?\s+(.*)$")
# `combat.md §14`, `systems/combat.md Section 14`, `combat.md 14.1`
CITE = re.compile(r"([A-Za-z0-9-]+)\.md[`\s]*(?:§|Section\s+|\s)\s*(\d+(?:\.\d+)*)")
# A bare `§8.2` or `Section 8.2`, which this repository writes once a
# nearby comment has already named the document. Attributed to the last
# document mentioned in the same file, which is how they read.
BARE = re.compile(r"(?:§|Section\s+)\s*(\d+(?:\.\d+)*)")
DOC = re.compile(r"([A-Za-z0-9-]+)\.md")


def spec_tree():
    out = subprocess.run(
        ["gh", "api", f"repos/{SPEC_REPO}/git/trees/HEAD?recursive=1"],
        capture_output=True, text=True, check=True,
    ).stdout
    return [
        e["path"]
        for e in json.loads(out)["tree"]
        if e["path"].endswith(".md") and e["path"].count("/") == 1
    ]


def fetch(path):
    return subprocess.run(
        ["gh", "api", f"repos/{SPEC_REPO}/contents/{path}?ref=HEAD",
         "-H", "Accept: application/vnd.github.raw"],
        capture_output=True, text=True, check=True,
    ).stdout


def engine_citations(root):
    cited = set()

    def record(stem, number):
        cited.add((stem, number))
        # A citation of 14.1 is evidence for 14 as well.
        while "." in number:
            number = number.rsplit(".", 1)[0]
            cited.add((stem, number))

    for rs in root.rglob("*.rs"):
        text = rs.read_text(errors="replace")
        for stem, number in CITE.findall(text):
            record(stem, number)
        # Walk the file once, carrying the last document named, so a bare
        # `§8.2` two lines under `conversation.md §8.1` counts for
        # `conversation.md`. Only ever attributed within one file.
        current = None
        for line in text.splitlines():
            for match in DOC.finditer(line):
                current = match.group(1)
            if current is None:
                continue
            for number in BARE.findall(line):
                record(current, number)
    return cited


def main():
    argv = sys.argv[1:]
    only = None
    if "--doc" in argv:
        only = argv[argv.index("--doc") + 1]
    verbose = "--all" in argv

    root = pathlib.Path("crates")
    if not root.is_dir():
        sys.exit("run from the engine repository root")
    cited = engine_citations(root)

    paths = [only] if only else spec_tree()
    total = uncited = 0
    report = []
    for path in sorted(paths):
        stem = pathlib.PurePosixPath(path).stem
        try:
            body = fetch(path)
        except subprocess.CalledProcessError:
            continue
        missing = []
        for line in body.splitlines():
            m = HEADING.match(line)
            if not m:
                continue
            number, title = m.group(2), m.group(3).strip()
            total += 1
            if (stem, number) not in cited:
                uncited += 1
                missing.append(f"    §{number} {title}")
        if missing and (verbose or not only):
            report.append(f"{path}  ({len(missing)} uncited)")
            report.extend(missing)
    print("\n".join(report))
    print(f"\nnumbered sections: {total}, uncited by the engine: {uncited}")


if __name__ == "__main__":
    main()
