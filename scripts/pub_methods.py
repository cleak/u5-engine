"""Add `pub` to top-level method definitions inside `impl` blocks.

Targets all `*.rs` files under crates/u5-runtime/src/. Leaves nested fns
(closures, helper fns inside method bodies) untouched. Detects an impl block
by the file starting with `impl ` (chunk files), or by tracking column-0
`impl ... {` openings and their matching close braces in part files.

Within an impl block, we transform any line matching exactly `    fn ` or
`    async fn ` or `    unsafe fn ` (i.e., 4 spaces of indent, the impl-body
depth) to `    pub fn ` etc.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOTS = [
    Path("crates/u5-runtime/src/parts"),
]

# Match a method line at exactly 4-space indent.
METHOD_RE = re.compile(r"^(    )(async\s+|unsafe\s+|const\s+)?fn ")


def transform_impl_chunk(text: str) -> str:
    """For files that ARE a single `impl X { ... }` block (the chunk files)."""
    out: list[str] = []
    for line in text.splitlines(keepends=True):
        m = METHOD_RE.match(line)
        if m and not line.lstrip().startswith("pub "):
            indent = m.group(1)
            modifier = m.group(2) or ""
            rest = line[len(indent) + len(modifier):]
            # `rest` starts with `fn `.
            line = f"{indent}pub {modifier}{rest}"
        out.append(line)
    return "".join(out)


def transform_part_file(text: str) -> str:
    """For files that have multiple top-level items including impl blocks."""
    out: list[str] = []
    in_impl = False
    depth = 0
    in_str = False
    str_ch = ""
    block_comment = 0

    for line in text.splitlines(keepends=True):
        # Detect start of an impl block at column 0 (no leading whitespace).
        if depth == 0 and not in_str and block_comment == 0:
            stripped = line.lstrip()
            if (
                stripped.startswith("impl ")
                or stripped.startswith("impl<")
                or stripped.startswith("impl<'")
            ) and "{" in line and not line.startswith(" "):
                in_impl = True

        # Apply method pub if in impl and at body depth.
        if in_impl and depth == 1:
            m = METHOD_RE.match(line)
            if m and not line.lstrip().startswith("pub "):
                indent = m.group(1)
                modifier = m.group(2) or ""
                rest = line[len(indent) + len(modifier):]
                line = f"{indent}pub {modifier}{rest}"

        out.append(line)

        # Track brace depth per line.
        j = 0
        L = len(line)
        while j < L:
            c = line[j]
            if block_comment > 0:
                if c == "*" and j + 1 < L and line[j + 1] == "/":
                    block_comment -= 1
                    j += 2
                    continue
                if c == "/" and j + 1 < L and line[j + 1] == "*":
                    block_comment += 1
                    j += 2
                    continue
                j += 1
                continue
            if in_str:
                if c == "\\" and j + 1 < L:
                    j += 2
                    continue
                if c == str_ch:
                    in_str = False
                j += 1
                continue
            if c == "/" and j + 1 < L and line[j + 1] == "/":
                break
            if c == "/" and j + 1 < L and line[j + 1] == "*":
                block_comment += 1
                j += 2
                continue
            if c == '"':
                in_str = True
                str_ch = '"'
                j += 1
                continue
            if c == "'":
                if j + 2 < L and line[j + 2] == "'":
                    j += 3
                    continue
                if j + 3 < L and line[j + 1] == "\\" and line[j + 3] == "'":
                    j += 4
                    continue
                j += 1
                continue
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0 and in_impl:
                    in_impl = False
            j += 1
    return "".join(out)


def is_impl_chunk(text: str) -> bool:
    first = text.lstrip().splitlines()[0] if text.strip() else ""
    return first.startswith("impl ") or first.startswith("impl<")


def main() -> int:
    files: list[Path] = []
    for root in ROOTS:
        files.extend(p for p in root.rglob("*.rs"))
    print(f"Processing {len(files)} files")
    changed = 0
    for f in files:
        text = f.read_text(encoding="utf-8")
        if is_impl_chunk(text):
            new = transform_impl_chunk(text)
        else:
            new = transform_part_file(text)
        if new != text:
            f.write_text(new, encoding="utf-8")
            changed += 1
    print(f"Updated {changed} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
