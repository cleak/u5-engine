#!/usr/bin/env python3
"""Which published literals does no handler actually print?

`spec_literal_coverage.py` asks whether the engine *contains* each
published line. `engine_message_audit.py` asks the reverse. Neither
notices a constant that exists, matches the spec exactly, and is read by
nothing - which is how `dungeon-mode.md §8.1`'s five `J`-Jimmy results
sat beside a handler printing this engine's own prose for months.

A constant is "wired" when its name appears anywhere but its own
declaration and the `lib.rs` re-export list - including in its own
module, where a constant often sits beside its one handler. Tests do not
count: a literal only a test names is one nothing prints.

The allowlist below carries the constants that legitimately have no
reader, each with its reason.

Usage: unwired_literals.py   (from the engine repository root)
"""

import pathlib
import re
import sys

# Published constants with no reader, and why that is not a defect.
EXPLAINED = {
    "WORLD_WATERFALL_TABLE_FILE": (
        "`overworld.md §8` retracted the sidecar: 'No `world_waterfalls.tsv` "
        "or equivalent sidecar table is part of the promoted contract.' The "
        "name stays so a profile carrying one is still recognised."
    ),
    "READY_REMOVE_PRESENT_PREFIX": (
        "The composed form `inventory.md §5.2` explicitly does *not* use: "
        "'each slot has its own wording, and only the helm's happens to "
        "match the composed form'. Kept as the documented near-miss."
    ),
    "READY_COMBAT_ARMOUR_LOCK_REFUSAL": (
        "Measured silent 2026-09-07 (`combat-ready-armour`): the original "
        "answers this branch with no line at all. §5.2 scopes its row to "
        "*undecided* combat, which the capture may not have been in, so the "
        "measurement stands and the difference is `cleak/u5-spec#247`."
    ),
    "DUNGEON_MOVEMENT_NOT_IN_DOORWAY_REFUSAL": (
        "`dungeon-mode.md §9` calls it 'reachable from more than one arm, so "
        "a spec should treat it as a movement refusal rather than binding it "
        "to one key' - and names no arm. Binding it anywhere would be a guess."
    ),
    "DUNGEON_KLIMB_WITH_WHAT_REFUSAL": (
        "§8.1's climb-with-equipment arm: 'whether it is specifically the "
        "*no-grapple* refusal is **probable, not established**', so no caller "
        "may treat the gating byte as confirmed."
    ),
    "DUNGEON_CHEST_GET_ECHO": (
        "The dungeon `Get` echo comes from `command_echo`'s own table, which "
        "gives `('Get', Complete)` underground. This is the same text as a "
        "standalone literal."
    ),
    "DIRECTION_PROMPT_CANCEL_LITERAL": (
        "Superseded by `DIRECTION_PROMPT_LABEL_PASS`, which carries the "
        "row-completion rule with it."
    ),
}


def main() -> int:
    root = pathlib.Path("crates")
    if not root.is_dir():
        print("run from the engine repository root")
        return 2
    sources = {p: p.read_text(errors="replace") for p in root.rglob("*.rs")}
    declared: list[tuple[str, str, pathlib.Path]] = []
    for path, text in sources.items():
        for match in re.finditer(
            r'pub const ([A-Z0-9_]+): &str = ("(?:[^"\\]|\\.)*")', text
        ):
            declared.append((match.group(1), match.group(2), path))

    unwired = []
    for name, literal, declaring in declared:
        readers = 0
        for path, text in sources.items():
            if path.name == "lib.rs":
                continue
            if "tests_inline" in str(path) or path.name.startswith("test"):
                continue
            hits = len(re.findall(rf"\b{name}\b", text))
            if path == declaring:
                # The declaration itself is not a reader.
                hits -= 1
            readers += hits
        if readers == 0:
            unwired.append((name, literal))

    unexplained = [(n, l) for n, l in unwired if n not in EXPLAINED]
    for name, literal in unwired:
        mark = "explained" if name in EXPLAINED else "UNWIRED  "
        print(f"{mark} {name} = {literal}")
        if name in EXPLAINED:
            print(f"          {EXPLAINED[name]}")
    print(
        f"\n{len(declared)} published string constants, {len(unwired)} with no "
        f"reader, {len(unexplained)} of those unexplained"
    )
    return 1 if unexplained else 0


if __name__ == "__main__":
    sys.exit(main())
