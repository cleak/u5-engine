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


# Absences read and explained on 2026-09-19 against spec HEAD. Each one
# is either prose the filter mistakes for a line, or a line the engine
# composes at run time rather than storing whole. Listing them keeps the
# report at "nothing unexplained" so a new absence stands out, which is
# the only way a reading list stays useful once it has been read once.
#
# Re-check an entry rather than trusting it if the citing section moves.
VERIFIED_BENIGN = {
    # Prose fragments ending in a colon, not game lines.
    "screen-panel graphics:": "formats/bit.md prose",
    "alphabet is exactly five values:": "formats/npc.md prose",
    "as a flat unsigned index:": "npc-schedules.md prose",
    "lore conflict:": "shops.md prose",
    "refusal is another stationary acted case:": "town-mode.md prose",
    # Composed from a prompt prefix plus a refusal, never stored whole.
    # `commands.md §5` gives `Klimb-` as the prompt; the refusals are
    # `-On foot!`, `On foot!` and `Down!`.
    "Klimb-Down!": "commands.md, prompt prefix plus refusal",
    "Klimb-On foot!": "commands.md, prompt prefix plus refusal",
    "Klimb--On foot!": "town-mode.md, the refusal carries its own hyphen",
    # combat.md §14 uses this one to say the engine must *not* do it.
    "Klimb-What?": "combat.md negative statement",
    # `<equipment name>!`, composed from the shared name table.
    "Leather Armour!": "commands.md example of the composed form",
    # `No <item>!`, composed the same way.
    "No Potion!": "inventory.md example of the composed form",
    "No Sceptre!": "inventory.md example of the composed form",
    "No Skull Keys!": "inventory.md example of the composed form",
    # `Attacked` plus an optional ` from the <compass>` plus `!`.
    "Attacked from the north!": "dungeon-mode.md composed form",
    # Sections that name a line in order to deny it.
    "Invisibility!": "magic.md: Sanct Lor prints no such banner",
    "Nobody can cast!": "magic.md: no such sentence is printed",
    "Hey!! What's going on here???": "dungeon-mode.md: must never occur",
    # An example of a prompt, not a prompt.
    "Y/N?": "input.md example of a prompt character",
}


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
        missing = sorted(
            literal
            for literal in found
            if literal not in haystack and literal not in VERIFIED_BENIGN
        )
        total += len(found)
        missing_total += len(missing)
        if missing:
            print(f"{document.relative_to(spec_dir)}: {len(missing)}/{len(found)} absent")
            for literal in missing:
                print(f"    {literal!r}")
    print(
        f"\n{missing_total} unexplained of {total} candidate literals "
        f"({len(VERIFIED_BENIGN)} previously read and explained)"
    )


if __name__ == "__main__":
    main()
