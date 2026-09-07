#!/usr/bin/env python3
"""Report message-window text the engine prints that the spec never quotes.

The inverse of `spec_literal_coverage.py`. Anything the engine writes into
`message` - or emits through the message-line helpers - is text the player
reads, and every such line should be a literal the clean specification
publishes. A line that appears nowhere in the spec is a *candidate*
invention: a sentence the original never prints.

This is a reading list, never a defect list. Diagnostics that are
deliberately harness-only, text the spec describes without quoting, and
lines assembled from published fragments all show up here and are all
fine. Every hit needs a human read of the contract it belongs to.

Usage: engine_message_audit.py <SPEC_DIR> <ENGINE_SRC_DIR>...
"""

import pathlib
import re
import sys

# `self.message = "..."`, `.message = format!("...")`, and the emit/print
# helpers, each with the literal as the first thing inside.
ASSIGN = re.compile(r"""message\s*=\s*(?:format!\()?"((?:[^"\\]|\\.){4,120})\"""")
EMIT = re.compile(
    r"""(?:emit_message_line|emit_command_echo_line|emit_combat_print|"""
    r"""emit_centered_message_line|emit_message_line_continuing_row)"""
    r"""\(\s*(?:&?format!\()?"((?:[^"\\]|\\.){4,120})\""""
)
PLACEHOLDER = re.compile(r"\{[^}]*\}")


def spec_text(spec_dir: pathlib.Path) -> str:
    return "\n".join(
        path.read_text(errors="replace") for path in spec_dir.rglob("*.md")
    )


def literals(source: pathlib.Path):
    for path in sorted(source.rglob("*.rs")):
        if "tests_inline" in path.parts or path.name.startswith("tests"):
            continue
        text = path.read_text(errors="replace")
        for number, line in enumerate(text.splitlines(), start=1):
            for pattern in (ASSIGN, EMIT):
                for literal in pattern.findall(line):
                    yield path, number, literal


def main() -> None:
    if len(sys.argv) < 3:
        raise SystemExit(__doc__)
    spec = spec_text(pathlib.Path(sys.argv[1]))
    seen: dict[str, tuple[str, int]] = {}
    for source in sys.argv[2:]:
        for path, number, literal in literals(pathlib.Path(source)):
            # Compare on the longest run of fixed text in the template, so a
            # composed line is judged on the words it always prints.
            runs = [run.strip() for run in PLACEHOLDER.split(literal)]
            longest = max(runs, key=len, default="")
            longest = longest.replace("\\n", " ").strip()
            if len(longest) < 5:
                continue
            if longest in spec:
                continue
            seen.setdefault(literal, (str(path), number))
    for literal, (path, number) in sorted(seen.items(), key=lambda item: item[1]):
        print(f"{path}:{number}: {literal!r}")
    print(f"\n{len(seen)} message literal(s) absent from the specification")


if __name__ == "__main__":
    main()
