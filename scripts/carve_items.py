"""Move named top-level items from u5-runtime parts into new modules.

Usage example (driven by main()):
  carve_to_module(
      dest=Path("crates/u5-runtime/src/play_state_struct.rs"),
      summary="The PlayState struct and overlay caches.",
      sources=[Path("crates/u5-runtime/src/parts/part_02.rs")],
      items=["PlayState", "WorldOverlayCache", "WorldReturn"],
  )

The carver walks each source file, finds each top-level item by name (struct,
enum, fn, const, static, type, trait, impl), and pulls out the item along
with its leading attributes/doc-comments/derive lines and the contiguous
`impl` blocks for the same type. Items are written to `dest` in the order
listed; the source file is rewritten without them.

Each new module gets a header with `use crate::*;` plus the standard
std imports so it stays self-sufficient.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path


@dataclass
class TopItem:
    start: int  # 0-based, inclusive (first attribute / doc-comment line)
    end: int    # 0-based, exclusive
    kind: str   # 'struct' | 'enum' | 'fn' | 'const' | 'static' | 'type' | 'trait' | 'impl' | 'union'
    name: str   # for impl: the type name being implemented for
    text: str


KEYWORDS_RE = re.compile(
    r"^(?P<head>(?:pub(?:\([^)]+\))?\s+)?(?:async\s+|unsafe\s+|const\s+|extern\s+(?:\"[^\"]*\"\s+)?)*)"
    r"(?P<kw>fn|struct|enum|const|static|type|trait|impl|union)\b"
)
NAME_RE = re.compile(r"\b(?:fn|struct|enum|const|static|type|trait|union)\s+([A-Za-z_][A-Za-z0-9_]*)")
IMPL_NAME_RE = re.compile(r"impl\s*(?:<[^>]*>\s*)?(?:[A-Za-z_:<>0-9_, '&\s]+\s+for\s+)?([A-Za-z_][A-Za-z0-9_:<>]*)")


def parse_items(text: str) -> list[TopItem]:
    """Walk top-level items in a Rust source file.

    Item starts at column 0; an item's preceding attribute/doc/comment lines
    (also at column 0) are pulled into the item's range so derives stay attached.
    """
    raw_lines = text.splitlines(keepends=True)
    items: list[TopItem] = []
    n = len(raw_lines)
    i = 0
    depth = 0
    in_str = False
    str_ch = ""
    block_comment = 0

    def at_top_level() -> bool:
        return depth == 0 and not in_str and block_comment == 0

    while i < n:
        line = raw_lines[i]
        if at_top_level() and line and not line[0].isspace():
            m = KEYWORDS_RE.match(line)
            if m:
                kw = m.group("kw")
                # Find name.
                if kw == "impl":
                    nm = IMPL_NAME_RE.search(line)
                    name = nm.group(1) if nm else "?"
                else:
                    nm = NAME_RE.search(line)
                    name = nm.group(1) if nm else "?"

                # Walk back to include attributes/doc/comment lines.
                start = i
                while start > 0:
                    prev = raw_lines[start - 1]
                    if prev.startswith("#["):
                        start -= 1
                        continue
                    if prev.startswith("///") or prev.startswith("//!") or prev.startswith("//"):
                        start -= 1
                        continue
                    break

                # Now consume the item body. For const/type/static, ends at semicolon
                # at depth 0. For struct without body (unit struct), ends at semicolon.
                # For everything else with a brace body, walk to matching close brace.
                end = None
                # Decide whether to track braces or semicolon.
                semicolon_terminated_kinds = {"const", "static", "type"}
                if kw in semicolon_terminated_kinds:
                    # Walk forward for `;` at outer depth.
                    # For const/static/type, the value can contain array
                    # literals (`[u8; 3]`, parentheses, etc.), so track
                    # all three bracket pairs.
                    j = i
                    local_depth = 0  # combined {}, [], () depth
                    local_in_str = in_str
                    local_str_ch = str_ch
                    local_bc = block_comment
                    while j < n and end is None:
                        L = raw_lines[j]
                        k = 0
                        Lc = len(L)
                        while k < Lc:
                            c = L[k]
                            if local_bc > 0:
                                if c == "*" and k + 1 < Lc and L[k + 1] == "/":
                                    local_bc -= 1
                                    k += 2
                                    continue
                                if c == "/" and k + 1 < Lc and L[k + 1] == "*":
                                    local_bc += 1
                                    k += 2
                                    continue
                                k += 1
                                continue
                            if local_in_str:
                                if c == "\\" and k + 1 < Lc:
                                    k += 2
                                    continue
                                if c == local_str_ch:
                                    local_in_str = False
                                k += 1
                                continue
                            if c == "/" and k + 1 < Lc and L[k + 1] == "/":
                                break
                            if c == "/" and k + 1 < Lc and L[k + 1] == "*":
                                local_bc += 1
                                k += 2
                                continue
                            if c == '"':
                                local_in_str = True
                                local_str_ch = '"'
                                k += 1
                                continue
                            if c == "'":
                                if k + 2 < Lc and L[k + 2] == "'":
                                    k += 3
                                    continue
                                if k + 3 < Lc and L[k + 1] == "\\" and L[k + 3] == "'":
                                    k += 4
                                    continue
                                k += 1
                                continue
                            if c in "{[(":
                                local_depth += 1
                            elif c in "}])":
                                local_depth -= 1
                            elif c == ";" and local_depth == 0:
                                end = j + 1
                                break
                            k += 1
                        j += 1
                    if end is None:
                        end = n
                else:
                    # Brace-body item. Walk forward until depth returns to 0 after the first `{`.
                    j = i
                    local_depth = 0
                    seen_open = False
                    local_in_str = in_str
                    local_str_ch = str_ch
                    local_bc = block_comment
                    while j < n and end is None:
                        L = raw_lines[j]
                        k = 0
                        Lc = len(L)
                        while k < Lc:
                            c = L[k]
                            if local_bc > 0:
                                if c == "*" and k + 1 < Lc and L[k + 1] == "/":
                                    local_bc -= 1
                                    k += 2
                                    continue
                                if c == "/" and k + 1 < Lc and L[k + 1] == "*":
                                    local_bc += 1
                                    k += 2
                                    continue
                                k += 1
                                continue
                            if local_in_str:
                                if c == "\\" and k + 1 < Lc:
                                    k += 2
                                    continue
                                if c == local_str_ch:
                                    local_in_str = False
                                k += 1
                                continue
                            if c == "/" and k + 1 < Lc and L[k + 1] == "/":
                                break
                            if c == "/" and k + 1 < Lc and L[k + 1] == "*":
                                local_bc += 1
                                k += 2
                                continue
                            if c == '"':
                                local_in_str = True
                                local_str_ch = '"'
                                k += 1
                                continue
                            if c == "'":
                                if k + 2 < Lc and L[k + 2] == "'":
                                    k += 3
                                    continue
                                if k + 3 < Lc and L[k + 1] == "\\" and L[k + 3] == "'":
                                    k += 4
                                    continue
                                k += 1
                                continue
                            if c == "{":
                                seen_open = True
                                local_depth += 1
                            elif c == "}":
                                local_depth -= 1
                                if seen_open and local_depth == 0:
                                    end = j + 1
                                    break
                            elif c == ";" and not seen_open and local_depth == 0:
                                # struct without body (unit struct or tuple struct;)
                                end = j + 1
                                break
                            k += 1
                        if end is not None:
                            break
                        j += 1
                    if end is None:
                        end = n

                items.append(TopItem(
                    start=start,
                    end=end,
                    kind=kw,
                    name=name,
                    text="".join(raw_lines[start:end]),
                ))
                i = end
                continue

        # Update parser state for non-item lines (and lines we didn't recognize).
        L = line
        k = 0
        Lc = len(L)
        while k < Lc:
            c = L[k]
            if block_comment > 0:
                if c == "*" and k + 1 < Lc and L[k + 1] == "/":
                    block_comment -= 1
                    k += 2
                    continue
                if c == "/" and k + 1 < Lc and L[k + 1] == "*":
                    block_comment += 1
                    k += 2
                    continue
                k += 1
                continue
            if in_str:
                if c == "\\" and k + 1 < Lc:
                    k += 2
                    continue
                if c == str_ch:
                    in_str = False
                k += 1
                continue
            if c == "/" and k + 1 < Lc and L[k + 1] == "/":
                break
            if c == "/" and k + 1 < Lc and L[k + 1] == "*":
                block_comment += 1
                k += 2
                continue
            if c == '"':
                in_str = True
                str_ch = '"'
                k += 1
                continue
            if c == "'":
                if k + 2 < Lc and L[k + 2] == "'":
                    k += 3
                    continue
                if k + 3 < Lc and L[k + 1] == "\\" and L[k + 3] == "'":
                    k += 4
                    continue
                k += 1
                continue
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            k += 1
        i += 1

    return items


def write_lines(p: Path, ls: list[str]) -> None:
    p.write_text("".join(ls), encoding="utf-8")


COMMON_HEADER_TMPL = (
    "//! {summary}\n"
    "\n"
    "use std::collections::{{HashMap, VecDeque}};\n"
    "use std::fs;\n"
    "use std::io;\n"
    "use std::path::{{Path, PathBuf}};\n"
    "\n"
    "use crate::*;\n"
    "\n"
)


def carve_to_module(
    dest: Path,
    summary: str,
    sources: list[Path],
    items: list[str],
    extra_imports: str = "",
) -> None:
    """Move all items whose name is in `items` from each source file to `dest`.

    For impl blocks, name is the impl-target type name. We move every impl
    that targets one of the requested types together with the type itself.
    """
    target = set(items)
    moved: list[str] = []
    for src in sources:
        text = src.read_text(encoding="utf-8")
        parsed = parse_items(text)
        keep_lines: list[str] = []
        # Build a list of "is item moved?" decisions.
        cursor = 0
        raw_lines = text.splitlines(keepends=True)
        for it in parsed:
            if it.name in target:
                moved.append(it.text)
                # write the gap before this item
                keep_lines.extend(raw_lines[cursor:it.start])
                cursor = it.end
        # tail
        keep_lines.extend(raw_lines[cursor:])
        # collapse triple+ blank lines down to a double
        out_text = "".join(keep_lines)
        out_text = re.sub(r"\n{3,}", "\n\n", out_text)
        write_lines(src, out_text.splitlines(keepends=True))

    body = "\n".join(moved)
    header = COMMON_HEADER_TMPL.format(summary=summary)
    if extra_imports:
        header += extra_imports + "\n"
    dest.write_text(header + body, encoding="utf-8")
    print(f"Wrote {dest}: {len(moved)} items (~{body.count(chr(10))} body lines)")
