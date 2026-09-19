#!/usr/bin/env python3
"""Report message-window text that looks like an engine diagnostic.

`engine_message_audit.py` compares whole literals against the published
text, so it cannot see a `format!` - the placeholders make the rendered
line something no spec search will match, and the scan skips it. That is
how `Opened object chest at (12, 7); Acid trap hit party member 2 for 20
HP.` sat in the message window: every published word in it is published,
and the parts that are not published are `{x}`, `{y}`, a slot index and a
damage count.

`commands.md §8.1` names the categories, for P-Push and by the engine's
own practice everywhere else: a command "never prints tile ids,
coordinates, active-object slot numbers, terrain-class names, or the word
`blocked`".

So this looks for those shapes in anything written to `message` or handed
to a message-line helper:

- a coordinate pair - `({x}, {y})`, `at ({a}, {b})`;
- a hex tile id - `0x{...}`;
- a slot or index named as such - `slot {n}`, `member {n}`, `record {n}`;
- a bare count of engine work - `{n} cell(s)`, `{n} encounter(s)`.

A reading list, never a defect list. A published line can legitimately
carry a number - `31 food!`, `Surcharge`-free gold totals, `Hull now 99!`
- and the shapes above are the ones the section names, not every
placeholder. Every hit needs a read of the contract it belongs to.

Usage: message_diagnostic_audit.py <ENGINE_SRC_DIR>...
"""

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from engine_message_audit import FIXTURE_OPT_OUT, skipped_lines

# `message = format!("...")` and the emit helpers taking a `format!`.
FORMATTED = re.compile(
    r"""(?:message\s*=\s*|emit_message_line\(|emit_combat_print\(|"""
    r"""emit_message_line_continuing_row\(|emit_centered_message_line\(|"""
    r"""emit_command_echo_line\()\s*&?format!\(\s*"((?:[^"\\]|\\.){4,200})\"""",
    re.S,
)

SHAPES = (
    (re.compile(r"\(\s*\{[^}]*\}\s*,\s*\{[^}]*\}\s*\)"), "coordinate pair"),
    (re.compile(r"0x\{"), "hex tile id"),
    (re.compile(r"\b(?:slot|member|record|index)\s+\{"), "slot or index"),
    (re.compile(r"\{[^}]*\}\s+(?:cell|encounter|barrier|object|tick)\(?s?\)?\b"), "engine work count"),
)


def main() -> int:
    hits = 0
    scanned = 0
    for root in (pathlib.Path(a) for a in sys.argv[1:]):
        for path in sorted(root.rglob("*.rs")):
            text = path.read_text(errors="replace")
            skipped = skipped_lines(text)
            for match in FORMATTED.finditer(text):
                number = text.count("\n", 0, match.start()) + 1
                if number in skipped:
                    continue
                line = text.splitlines()[number - 1]
                if FIXTURE_OPT_OUT in line:
                    continue
                literal = match.group(1)
                scanned += 1
                for shape, why in SHAPES:
                    if shape.search(literal):
                        hits += 1
                        print(f"{path}:{number}: {why}: {literal!r}")
                        break
    print(f"\n{hits} diagnostic-shaped message(s) of {scanned} formatted line(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
