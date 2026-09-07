#!/usr/bin/env python3
"""Report published game-text literals the engine does not contain.

The clean specification quotes the shipped game's own strings in inline
code spans. Every one of them that is player-visible text should appear
somewhere in the engine's sources; a literal that appears nowhere is a
*candidate* gap - a published line the engine cannot print.

This is a reading list, never a defect list. It cannot tell a real gap
from a line the engine composes at run time, from a literal the spec
renders with its underscore-for-space convention in a way this filter
mis-normalises, or from prose that merely looks like a game line. Every
hit needs a human read of the section it came from.

Usage: spec_literal_coverage.py <SPEC_DIR> <ENGINE_SRC_DIR>...
"""

import pathlib
import re
import sys

SPAN = re.compile(r"`([^`\n]{3,48})`")
# A published line looks like text: letters and spaces, and it ends in
# punctuation or carries at least one space.
TEXTY = re.compile(r"^[A-Za-z][A-Za-z0-9 '.,!?:;()\-/]*$")
SKIP_TOKENS = (
    "u5-decomp",
    "systems/",
    "catalogs/",
    "formats/",
    ".md",
    ".rs",
    ".dat",
    ".DAT",
    "()",
    "::",
    "0x",
    "§",
)


def candidates(text: str) -> set[str]:
    found = set()
    for span in SPAN.findall(text):
        literal = span.replace("_", " ").strip()
        if any(token in span for token in SKIP_TOKENS):
            continue
        if not TEXTY.match(literal):
            continue
        # A shipped line ends in its own punctuation. Prose fragments in
        # the surrounding sentence almost never do, which is what keeps
        # this list readable.
        if not literal.endswith(("!", "?", ":")):
            continue
        if len(literal.split()) > 7:
            continue
        if literal.lower() in {"yes", "no", "none", "on", "off"}:
            continue
        found.add(literal)
    return found


def main() -> None:
    if len(sys.argv) < 3:
        raise SystemExit(__doc__)
    spec_dir = pathlib.Path(sys.argv[1])
    sources = [pathlib.Path(argument) for argument in sys.argv[2:]]
    haystack = "\n".join(
        path.read_text(errors="replace")
        for source in sources
        for path in source.rglob("*.rs")
    )
    total = 0
    missing_total = 0
    for document in sorted(spec_dir.rglob("*.md")):
        found = candidates(document.read_text(errors="replace"))
        missing = sorted(literal for literal in found if literal not in haystack)
        total += len(found)
        missing_total += len(missing)
        if missing:
            print(f"{document.relative_to(spec_dir)}: {len(missing)}/{len(found)} absent")
            for literal in missing:
                print(f"    {literal!r}")
    print(f"\n{missing_total} absent of {total} candidate literals")


if __name__ == "__main__":
    main()
