"""Move tests that exercise moved-TUI symbols out of u5-runtime into u5-tui.

For each `parts/tests/chunk_NN.rs`, parse the body into individual
`fn` items (tests + helpers). For each item, if its body references any
of the moved TUI symbols, mark it as moving. Otherwise, keep it in place.

Output:
  crates/u5-tui/tests/play_loop.rs  -- the moved tests and their helpers,
                                       wrapped in `mod tests { use ... }`.
  crates/u5-runtime/src/parts/tests/chunk_NN.rs  -- updated, missing the
                                                   moved items.

Tests that need helpers will need helpers moved too. Helper-detection: any
top-level `fn helper_*` that is referenced by a moved test but only used
within the test chunk is moved as well.
"""

from __future__ import annotations

import re
from pathlib import Path

CHUNK_DIR = Path("crates/u5-runtime/src/parts/tests")
DEST = Path("crates/u5-tui/tests/play_loop.rs")

TUI_SYMBOLS = {
    "raster_diagnostic_line",
    "play_input_key_and_suffix",
    "play_input_typeahead_chars",
    "is_simple_typeahead_key",
    "is_typeahead_toggle_token",
    "ansi_navigation_key",
    "ansi_function_key",
    "unclassified_escape_sequence",
    "handle_empty_play_input",
    "handle_play_script_command",
    "play_script_idle_tick_count",
    "play_script_command_label",
    "play_script_state_line",
    "run_play_loop",
    "run_play_script_commands",
    "print_play_frame",
    "print_play_script_snapshot",
}


def split_items(body: list[str]) -> list[tuple[int, int, str]]:
    """Return list of (start_line_idx, end_line_idx_exclusive, item_text).

    Items are at exactly 4-space indent within the body. Each item begins at
    its first attribute/comment line and ends at the line after its closing
    brace.
    """
    base = "    "
    keywords = ("fn ",)

    # Track brace depth at body level (depth 0 = item-list; depth 1+ = inside item).
    depth = 0
    in_str = False
    str_ch = ""
    block_comment = 0

    items: list[tuple[int, int]] = []
    cur_start: int | None = None
    pending_start: int | None = None

    for i, line in enumerate(body):
        if depth == 0 and not in_str and block_comment == 0:
            stripped_full = line.lstrip(" ")
            if line.startswith(base) and not line.startswith(base + " "):
                # candidate item line
                rest = line[len(base):]
                if any(rest.startswith(k) for k in keywords):
                    # Walk back to include attributes/doc-comments
                    start = i
                    while start > 0:
                        prev = body[start - 1]
                        if not prev.startswith(base):
                            break
                        prev_rest = prev[len(base):]
                        if (
                            prev_rest.startswith("#[")
                            or prev_rest.startswith("///")
                            or prev_rest.startswith("//!")
                            or prev_rest.startswith("//")
                        ):
                            start -= 1
                            continue
                        break
                    pending_start = start
                    cur_start = start

        # Update brace tracker
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
                if depth == 0 and cur_start is not None:
                    items.append((cur_start, i + 1))
                    cur_start = None
            j += 1

    return [(s, e, "".join(body[s:e])) for s, e in items]


def fn_name(item_text: str) -> str | None:
    m = re.search(r"fn ([A-Za-z_][A-Za-z0-9_]*)\b", item_text)
    return m.group(1) if m else None


def main() -> int:
    chunk_files = sorted(CHUNK_DIR.glob("chunk_*.rs"))
    moved_items: list[tuple[str, str]] = []  # (chunk_name, item_text)

    # First pass: collect moves.
    for cf in chunk_files:
        text = cf.read_text(encoding="utf-8")
        body = text.splitlines(keepends=True)
        items = split_items(body)

        keep_ranges: list[tuple[int, int]] = []
        chunk_moves: list[tuple[int, int, str]] = []
        for s, e, content in items:
            referenced = any(re.search(r"\b" + sym + r"\b", content) for sym in TUI_SYMBOLS)
            if referenced:
                chunk_moves.append((s, e, content))
            else:
                keep_ranges.append((s, e))

        if not chunk_moves:
            continue

        # Build new body keeping only the non-moved spans.
        new_body: list[str] = []
        cursor = 0
        # Sort moves by start
        chunk_moves_sorted = sorted(chunk_moves, key=lambda t: t[0])
        for s, e, _ in chunk_moves_sorted:
            new_body.extend(body[cursor:s])
            cursor = e
        new_body.extend(body[cursor:])
        cf.write_text("".join(new_body), encoding="utf-8")
        for s, e, content in chunk_moves_sorted:
            moved_items.append((cf.stem, content))
        print(f"  {cf.name}: moved {len(chunk_moves_sorted)} items, kept {len(items) - len(chunk_moves_sorted)}")

    if not moved_items:
        print("No items moved.")
        return 0

    # Build the destination file.
    DEST.parent.mkdir(parents=True, exist_ok=True)
    header = (
        "//! Tests for `u5_tui::play_loop`.\n"
        "//!\n"
        "//! Moved from `u5-runtime` when the play loop and its helpers were\n"
        "//! lifted into the TUI crate. Tests that exercise both runtime and\n"
        "//! TUI symbols live here because the TUI symbols are no longer in\n"
        "//! `u5-runtime`'s scope.\n"
        "\n"
        "use std::collections::HashMap;\n"
        "use std::fs;\n"
        "use std::io;\n"
        "use std::path::{Path, PathBuf};\n"
        "use std::sync::OnceLock;\n"
        "\n"
        "use u5_runtime::*;\n"
        "use u5_tui::*;\n"
        "\n"
    )

    out_lines = [header]
    out_lines.append("// ---- moved test bodies ----\n\n")
    for chunk_name, item in moved_items:
        out_lines.append(f"// from {chunk_name}\n")
        # Items are 4-space-indented (they were inside `mod tests { ... }`).
        # Outdent by 4.
        for line in item.splitlines(keepends=True):
            if line.startswith("    "):
                out_lines.append(line[4:])
            else:
                out_lines.append(line)
        out_lines.append("\n")

    DEST.write_text("".join(out_lines), encoding="utf-8")
    print(f"Wrote {DEST} with {len(moved_items)} items")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
