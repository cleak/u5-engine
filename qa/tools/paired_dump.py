#!/usr/bin/env python3
"""Print both sides of one paired beat as decoded text rows.

`paired_compare.py` says *which* rows of a beat disagree and classifies why;
this prints the rows themselves, side by side, so the disagreement can be read.
It decodes the same way the comparator does - the message window only, through
the capture's own 4:3 frame geometry - so what it prints is what the comparator
compared.

The decoded text stays in the process. It is game text, and only what an
engineer reads leaves; nothing here writes to the repository.

Usage: paired_dump.py <ARTIFACT_DIR> <BEAT> [BEAT...]
       paired_dump.py --latest <SCENARIO> <BEAT> [BEAT...]
"""

import pathlib
import sys

import paired_compare


def dump(artifact: pathlib.Path, label: str) -> None:
    print(f"=== {artifact.name} / {label}")
    for side, prefix in (("stock (DOSBox)", "dosbox"), ("engine", "engine")):
        png = artifact / f"{prefix}-{label}.png"
        if not png.is_file():
            print(f"--- {side}: no capture")
            continue
        rows = paired_compare.decode(png)
        if rows is None:
            print(f"--- {side}: capture too small to decode")
            continue
        print(f"--- {side}")
        for index, row in enumerate(rows):
            print(f"{index + paired_compare.TOP:2d} |{paired_compare.row_text(row)}|")


def main() -> None:
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    if args[0] == "--latest":
        found = paired_compare.latest_artifacts([args[1]])
        if not found:
            raise SystemExit(f"no run found for scenario `{args[1]}`")
        artifact, labels = found[0], args[2:]
    else:
        artifact, labels = pathlib.Path(args[0]), args[1:]
    if not labels:
        raise SystemExit("name at least one beat")
    for label in labels:
        dump(artifact, label)


if __name__ == "__main__":
    main()
