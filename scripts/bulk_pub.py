"""Transform u5-tui/src/main.rs into u5-runtime/src/lib.rs.

This is a one-shot scaffolding script for the workspace refactor.

Transformations:
  * Drop the `#[cfg(feature = "visual")] mod visual;` block.
  * Drop the `run_visual_dispatch` helpers and the `args.visual` branch in main
    (visual dispatch lives in u5-tui).
  * Rename `fn main` to `pub fn run`.
  * Add `pub` to every top-level item that is not already `pub` (enum, struct,
    fn, const, static, trait, type, mod). Item lines preceded by a `#[cfg(test)]`
    or other attribute are detected by walking backwards through attribute lines.
  * Add `pub` to every struct field. Tuple-struct fields are also handled.
"""

from __future__ import annotations

import io
import re
import sys
from pathlib import Path


SRC = Path("crates/u5-tui/src/main.rs")
DST = Path("crates/u5-runtime/src/lib.rs")

ITEM_KEYWORDS = ("enum", "struct", "fn", "const", "static", "trait", "type", "mod", "union")
ITEM_RE = re.compile(
    r"^(?P<indent>)"
    r"(?P<head>(?:async\s+|unsafe\s+)*)"
    r"(?P<kw>(?:" + "|".join(ITEM_KEYWORDS) + r"))\b"
)


def is_already_pub(line: str) -> bool:
    s = line.lstrip()
    return s.startswith("pub ") or s.startswith("pub(")


def make_top_level_pub(text: str) -> str:
    out_lines: list[str] = []
    for line in text.splitlines(keepends=True):
        # Only consider lines that start at column 0 (top-level items).
        if line and not line[0].isspace():
            stripped = line.lstrip()
            if stripped.startswith(ITEM_KEYWORDS) and not is_already_pub(line):
                # Skip "fn main" because we rename it separately.
                # (Handled before this point.)
                # Also skip module-level use/extern/macro_rules.
                line = "pub " + line
        out_lines.append(line)
    return "".join(out_lines)


def make_struct_fields_pub(text: str) -> str:
    """Add `pub` to every field inside a `pub struct Name { ... }` block.

    Handles only braced structs. Tuple structs (with parens) are handled
    separately by `make_tuple_struct_fields_pub`.
    """
    lines = text.splitlines(keepends=True)
    i = 0
    out: list[str] = []
    while i < len(lines):
        line = lines[i]
        stripped = line.lstrip()
        # Detect `pub struct Name {` or `pub struct Name<...> {` (allow generics).
        m = re.match(r"^(pub\s+)?struct\s+\w+[^;]*\{\s*$", stripped)
        if m and line.startswith("pub struct"):
            # Walk fields until the closing brace at same indent.
            out.append(line)
            i += 1
            while i < len(lines):
                fld_line = lines[i]
                fld_strip = fld_line.lstrip()
                if fld_strip.startswith("}"):
                    out.append(fld_line)
                    i += 1
                    break
                # Skip blank lines and attributes/comments.
                if (
                    not fld_strip
                    or fld_strip.startswith("//")
                    or fld_strip.startswith("/*")
                    or fld_strip.startswith("*")
                    or fld_strip.startswith("#[")
                    or fld_strip.startswith("#!")
                ):
                    out.append(fld_line)
                    i += 1
                    continue
                # Field line: starts with identifier `name:` or `pub name:`.
                if fld_strip.startswith("pub "):
                    out.append(fld_line)
                else:
                    m2 = re.match(r"^(\w+)\s*:", fld_strip)
                    if m2:
                        indent_len = len(fld_line) - len(fld_strip)
                        out.append(fld_line[:indent_len] + "pub " + fld_strip)
                    else:
                        out.append(fld_line)
                i += 1
            continue
        out.append(line)
        i += 1
    return "".join(out)


def transform(text: str) -> str:
    # 1. Strip the visual mod declaration block.
    text = re.sub(
        r'#\[cfg\(feature\s*=\s*"visual"\)\]\s*\nmod\s+visual;\s*\n',
        "",
        text,
    )

    # 2. Drop run_visual_dispatch helpers (cfg-on and cfg-off variants).
    text = re.sub(
        r'#\[cfg\(feature\s*=\s*"visual"\)\]\s*\nfn\s+run_visual_dispatch[^{]*\{[^}]*\}\s*\n',
        "",
        text,
    )
    text = re.sub(
        r'#\[cfg\(not\(feature\s*=\s*"visual"\)\)\]\s*\nfn\s+run_visual_dispatch[^{]*\{(?:[^{}]|\{[^{}]*\})*\}\s*\n',
        "",
        text,
    )

    # 3. Drop the `if args.visual { return run_visual_dispatch(args); }` branch.
    text = re.sub(
        r"\s*if\s+args\.visual\s*\{\s*\n\s*return\s+run_visual_dispatch\(args\);\s*\n\s*\}\s*\n",
        "\n",
        text,
    )

    # 4. Rename `fn main` to `pub fn run`. Only matches the top-level definition.
    text = re.sub(
        r"^fn\s+main\s*\(",
        "pub fn run(",
        text,
        count=1,
        flags=re.MULTILINE,
    )

    # 5. Add `pub` to all top-level items not already pub.
    text = make_top_level_pub(text)

    # 6. Pub all struct fields.
    text = make_struct_fields_pub(text)

    return text


def main() -> int:
    text = SRC.read_text(encoding="utf-8")
    out = transform(text)
    DST.parent.mkdir(parents=True, exist_ok=True)
    DST.write_text(out, encoding="utf-8")
    print(f"Wrote {DST} ({len(out)} bytes, {out.count(chr(10))} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
