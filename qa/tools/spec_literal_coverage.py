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

# Documents that quote text without publishing it.
NON_CONTRACT_DOCUMENTS = {
    "RETRACTIONS.md",
    "NEXT-STEPS.md",
    "EXTRACTION.md",
    "OPEN-QUESTIONS.md",
}

SPAN = re.compile(r"`([^`\n]{3,48})`")
# A published line looks like text: letters and spaces, and it ends in
# punctuation or carries at least one space.
# A published line looks like text, and the specification writes its own
# line feeds as `\n` escapes inside the span - exactly as Rust source
# does, so the escape is matched rather than normalised away. Without the
# backslash here every literal written that way was skipped in silence:
# `\nNo land nearby!\n` and `\nNo skiffs on board!\n`
# (`vehicles.md §5.1`) were both absent from the engine and neither was
# reported until 2026-09-19.
TEXTY = re.compile(r"^[\\A-Za-z][\\A-Za-z0-9 'n.,!?:;()\-/]*$")
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
    # --- Read 2026-09-19, once the escape handling above was fixed. ---
    # A command echo plus its own completion: the echo is the direction or
    # the verb, and the engine holds the two apart because the row is
    # completed rather than reprinted.
    "East\\nEscape!": "combat.md, direction echo plus `Escape!`",
    "West\\nLeave!": "combat.md, direction echo plus `Leave!`",
    "West\\n\\nStay with ship!": "combat.md, direction echo plus the refusal",
    "East\\n\\nAll must use the same exit!": "combat.md, echo plus refusal",
    "Enter cave\\nAttacked at entrance!": "commands.md, echo plus refusal",
    "Push\\nNot here!": "commands.md/dungeon-mode.md, echo plus refusal",
    "You find:\\nA hidden door!": "dungeon-mode.md, Search preamble plus find",
    "X-it \\nUnder sail!": "vehicles.md, X-it echo plus the refusal",
    "a well.\\n\\nDrop a coin?": "view.md, the Look description plus the prompt",
    "Cast...\\nAbsorbed!": "combat/magic/audio, the cast echo plus the result",
    # `<class name>` plus a stored line. `combat.md §11.1` is explicit that
    # the stored part is " escapes!" alone and the name comes from the
    # shared actor-name printer.
    "Orc escapes!": "combat.md §11.1, class name plus ` escapes!`",
    "Shadow Lord escapes!": "combat.md §11.1, class name plus ` escapes!`",
    # `<vehicle kind>!`, composed from the transport's own name.
    "skiff!": "vehicles.md §5.1, the vehicle kind plus `!`",
    # Read from the asset at run time, not stored in the engine: the camp
    # result messages come out of the shipped data through
    # `rest_camp::load_camp_result_messages`.
    "Party rested!": "rest-and-camp.md, loaded from the asset at run time",
    # Present, in `graphics.rs`, as part of a larger string.
    "REGISTER:": "shops.md, present in the engine",
}

# Published lines the engine does **not** have and cannot yet implement,
# each with the upstream question blocking it. These stay out of
# `VERIFIED_BENIGN` deliberately: they are real gaps, not false hits, and
# the count below keeps them in view.
AWAITING_SPEC = {
    "Field dissolved!": "cleak/u5-spec#287 - what is the Sceptre's fallback helper?",
}


def candidates(text: str) -> set[str]:
    found = set()
    for span in SPAN.findall(text):
        literal = span.replace("_", " ").strip()
        # A published line carries its own feeds, and the specification
        # writes them as `\n` escapes inside the span - exactly as Rust
        # source does. Strip only the leading and trailing ones: the
        # filters below test the line's own text, and an interior escape
        # is part of it (`Klimb-\nWith What?`).
        #
        # Without this every line written with its feeds was skipped in
        # silence, because the `endswith` test below saw the escape rather
        # than the punctuation. `\nNo land nearby!\n` and
        # `\nNo skiffs on board!\n` (`vehicles.md §5.1`) were both absent
        # from the engine and neither was reported until 2026-09-19.
        literal = re.sub(r"^(?:\\[nr])+|(?:\\[nr])+$", "", literal).strip()
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
        # A withdrawn wording is not a published line. `RETRACTIONS.md`
        # quotes both sides of every correction, and the other three are
        # process documents rather than contracts, so a literal found
        # *only* there is one the original does not print - `Beat it!`
        # against the surviving `BEAT IT!`, and the four wound words the
        # combat contract no longer carries. Counting them made the engine
        # owe text the spec had already taken back.
        if document.name in NON_CONTRACT_DOCUMENTS:
            continue
        found = candidates(document.read_text(errors="replace"))
        missing = sorted(
            literal
            for literal in found
            if literal not in haystack
            and literal not in VERIFIED_BENIGN
            and literal not in AWAITING_SPEC
        )
        total += len(found)
        missing_total += len(missing)
        if missing:
            print(f"{document.relative_to(spec_dir)}: {len(missing)}/{len(found)} absent")
            for literal in missing:
                print(f"    {literal!r}")
    print(
        f"\n{missing_total} unexplained of {total} candidate literals "
        f"({len(VERIFIED_BENIGN)} previously read and explained, "
        f"{len(AWAITING_SPEC)} awaiting a spec answer)"
    )
    for literal, why in sorted(AWAITING_SPEC.items()):
        print(f"  blocked: {literal!r} - {why}")


if __name__ == "__main__":
    main()
