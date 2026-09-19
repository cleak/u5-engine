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
        "gives `('Get', Complete)` underground."
    ),
    "DIRECTION_PROMPT_CANCEL_LITERAL": (
        "Superseded by `DIRECTION_PROMPT_LABEL_PASS`, which carries the "
        "row-completion rule with it."
    ),
    "DUNGEON_KLIMB_PASS": (
        "`dungeon-mode.md §8.1`'s `Pass\\n\\n` is produced by the shared "
        "direction-prompt cancel: the word completes the open `Klimb-U/D-` "
        "row and the dungeon arm adds the blank its second feed makes."
    ),
    "DUNGEON_FACING_LABEL_PREFIX": (
        "A duplicate of `gameplay_chrome::DUNGEON_FACING_LABEL`, which is "
        "the one the frame painter composes `Dir: North` from. Two names "
        "for one published label; this is the unused twin."
    ),
    "UPMARKET_INN_AFFORDABILITY_REFUSAL_BARK": (
        "A fragment of a longer composed literal: the inn refusal prints "
        '`\"Highwaymen!\\nCheap, at that!\\nOUT!\" `, which the handler '
        "holds whole."
    ),
    # Published spellings kept for conformance, with no handler that could
    # read them: the engine does not open the original executables, and the
    # no-save lines reach the player as the loader's composed error.
    "ULTIMA_EXE_FILENAME": (
        "An original executable this engine never opens; kept as the "
        "published asset name."
    ),
    "INTRO_OVL_FILENAME": ("See `ULTIMA_EXE_FILENAME`."),
    "ENDGAME_TITLE_STRIP_ARCHIVE": (
        "The archive name the endgame's title strip comes from; the loader "
        "composes its path from the story layout rather than this constant."
    ),
    "QUEST_PASSWORD_RESISTANCE": (
        "The two Blackthorn passwords are recognised by the TLK control-code "
        "comparison, which case-folds the typed word rather than comparing "
        "against these names. They are the published spellings, asserted by "
        "the conformance tests."
    ),
    "QUEST_PASSWORD_OPPRESSION": ("See `QUEST_PASSWORD_RESISTANCE`."),
    "SAVE_PROMPT_NO_REPLY": (
        "The declined save prints the shared `No.` line; this is the "
        "published bare word beside it, asserted by the conformance tests."
    ),
    "LOAD_EMPTY_SAVE_LINE_1": (
        "The three no-save lines reach the player as the loader's composed "
        "error - `No active game. Please create a character or transfer one "
        "from Ultima IV.` - rather than as three prints."
    ),
    "LOAD_EMPTY_SAVE_LINE_2": ("See `LOAD_EMPTY_SAVE_LINE_1`."),
    "LOAD_EMPTY_SAVE_LINE_3": ("See `LOAD_EMPTY_SAVE_LINE_1`."),
    # A real gap, tracked rather than explained away.
    "VEHICLE_BROKER_PARTIAL_AFFORD_PREFIX": (
        "GAP, `cleak/u5-engine#43`. `shops.md`'s tavern-provisions table "
        "gives the partial-afford row as `\\n\\n\"Thou canst\\nafford only `, "
        "the number served, `!\"\\n\\n` - and the constant's own doc "
        "misattributes it to a vehicle broker. The tavern's provisions "
        "outcomes render engineering prose instead (`sold 2/5 provision "
        "packs for 10 gold; food +20`), so none of that row's four arms is "
        "printed as published."
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
