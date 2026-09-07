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

# Two kinds of write into `message` are not lines the game prints: a frame
# suite or probe writing text in order to *render* a window, and the Bevy
# shell surfacing a Rust error it cannot otherwise report. Both carry this
# marker and are skipped; it is deliberately verbose so it cannot be added by
# accident, and every use should say in a comment why the line is not
# player-facing.
FIXTURE_OPT_OUT = "audit: not a player-facing line"


def skipped_lines(text: str) -> set[int]:
    """Line numbers inside `#[cfg(test)]` items.

    Test code sets `message` to whatever the assertion needs, which says
    nothing about what the game prints. The scan brace-matches from the
    attribute so nested modules and functions both fall inside.
    """
    skipped: set[int] = set()
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.strip() not in ("#[cfg(test)]", "#[test]"):
            continue
        depth = 0
        started = False
        for number in range(index, len(lines)):
            depth += lines[number].count("{") - lines[number].count("}")
            started = started or "{" in lines[number]
            skipped.add(number + 1)
            if started and depth <= 0:
                break
    return skipped


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
            if number in skipped_lines(text):
                continue
            if FIXTURE_OPT_OUT in line:
                continue
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
            # Judge on the longest run of fixed text: a template's
            # placeholders and its line breaks both split it.
            runs = [
                run.strip()
                for part in PLACEHOLDER.split(literal)
                for run in part.split("\\n")
            ]
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
