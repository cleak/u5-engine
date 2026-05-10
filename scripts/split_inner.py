"""Sub-split part_04 (impl PlayState) and part_16 (mod tests) into <1000-line chunks.

After running split_lib.py, two parts remain too large because they consist
of a single brace-wrapped block. This script:

  * For part_04: strips the outer `impl PlayState { ... }`, splits the body
    at method boundaries (lines starting `fn ` or `pub fn ` at depth 1),
    writes each chunk to `parts/play_state_impl/chunk_NN.rs` (raw method
    bodies), and rewrites part_04 as the `impl PlayState { include!(...) }`
    wrapper.

  * For part_16: strips the outer `#[cfg(test)] pub mod tests { use super::*; ... }`,
    splits the body at test/helper boundaries, writes each chunk to
    `parts/tests/chunk_NN.rs`, and rewrites part_16 as the wrapper.
"""

from __future__ import annotations

from pathlib import Path

PARTS_DIR = Path("crates/u5-runtime/src/parts")
TARGET_LINES = 900


def find_inner_item_boundaries(body_lines: list[str]) -> list[int]:
    """Find boundaries inside a body at depth 0 of the body itself.

    Body lines have leading indent. Items begin at the lowest indent level
    used in the body (typically 4 spaces). We find lines that start with
    that minimum indent followed by a keyword, and walk back over leading
    attributes/doc-comments.
    """
    # Both bodies we handle are at depth 1, so item lines start with exactly
    # 4 spaces. Computing min_indent dynamically is unsafe because raw string
    # literals can contain lines at column 0.
    base = "    "

    keywords = (
        "pub fn ", "fn ", "pub async fn ", "async fn ",
        "pub unsafe fn ", "unsafe fn ",
        "pub const ", "const ", "pub struct ", "struct ",
        "pub enum ", "enum ", "pub static ", "static ",
        "pub type ", "type ",
    )

    starts: list[int] = []
    # Track brace depth relative to body (depth 0 = top of body).
    depth = 0
    in_str = False
    str_ch = ""
    block_comment = 0
    for i, line in enumerate(body_lines):
        if depth == 0 and block_comment == 0 and not in_str:
            # Check if this line starts a top-level inner item.
            if line.startswith(base) and not line.startswith(base + " "):
                rest = line[len(base):]
                if any(rest.startswith(k) for k in keywords):
                    # Walk back over preceding attributes/doc-comments at this indent.
                    start = i
                    while start > 0:
                        prev = body_lines[start - 1]
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
                    starts.append(start)
        # Update brace tracker (mirrors the depth logic in split_lib.py).
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
            j += 1
    return starts


def chunk_body(body_lines: list[str], boundaries: list[int]) -> list[tuple[int, int]]:
    if not boundaries:
        return [(0, len(body_lines))]
    # Ensure first boundary is 0
    if boundaries[0] != 0:
        boundaries = [0] + boundaries

    chunks: list[tuple[int, int]] = []
    cur = 0
    bi = 0
    while cur < len(body_lines):
        target = cur + TARGET_LINES
        chosen = None
        # advance bi past entries <= cur
        while bi < len(boundaries) and boundaries[bi] <= cur:
            bi += 1
        # find the largest boundary <= target
        while bi < len(boundaries):
            b = boundaries[bi]
            if b > target:
                if chosen is None:
                    chosen = b
                    bi += 1
                break
            chosen = b
            bi += 1
        if chosen is None:
            chunks.append((cur, len(body_lines)))
            break
        chunks.append((cur, chosen))
        cur = chosen
    return chunks


def split_part_04() -> None:
    part_path = PARTS_DIR / "part_04.rs"
    text = part_path.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    # Expect: lines[0] == "impl PlayState {\n", lines[-2] == "}\n", lines[-1] == "\n"
    assert lines[0].rstrip() == "impl PlayState {", lines[0]
    # Find the matching closing brace at column 0.
    close_idx = None
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].rstrip() == "}":
            close_idx = i
            break
    assert close_idx is not None
    body = lines[1:close_idx]
    print(f"part_04 body: {len(body)} lines")

    boundaries = find_inner_item_boundaries(body)
    print(f"  found {len(boundaries)} method boundaries")
    chunks = chunk_body(body, boundaries)
    print(f"  splitting into {len(chunks)} chunks")

    sub_dir = PARTS_DIR / "play_state_impl"
    sub_dir.mkdir(parents=True, exist_ok=True)
    for old in sub_dir.glob("chunk_*.rs"):
        old.unlink()

    # `include!` does not work inside an impl block, so each chunk wraps its
    # methods in its own `impl PlayState { ... }`. The wrapper part_04.rs then
    # include!s the chunks at top level.
    new_part_lines: list[str] = []
    for idx, (s, e) in enumerate(chunks, start=1):
        chunk_path = sub_dir / f"chunk_{idx:02d}.rs"
        wrapped = "impl PlayState {\n" + "".join(body[s:e]) + "}\n"
        chunk_path.write_text(wrapped, encoding="utf-8")
        new_part_lines.append(f'include!("play_state_impl/chunk_{idx:02d}.rs");\n')
        print(f"    chunk_{idx:02d}.rs: {e - s + 2} lines")
    part_path.write_text("".join(new_part_lines), encoding="utf-8")
    print(f"  rewrote part_04.rs ({len(new_part_lines)} lines)")


def split_part_16() -> None:
    part_path = PARTS_DIR / "part_16.rs"
    text = part_path.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    # Expect:
    #   lines[0] == "#[cfg(test)]\n"
    #   lines[1] == "pub mod tests {\n"
    #   lines[2] == "    use super::*;\n"
    #   lines[-1] == "}"
    assert lines[0].rstrip() == "#[cfg(test)]", lines[0]
    assert lines[1].rstrip() == "pub mod tests {", lines[1]
    # Find closing brace
    close_idx = None
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].rstrip() == "}":
            close_idx = i
            break
    assert close_idx is not None
    header_lines = lines[0:3]  # cfg, mod open, use super::*
    footer_lines = lines[close_idx:]  # closing brace
    body = lines[3:close_idx]
    print(f"part_16 body: {len(body)} lines")

    boundaries = find_inner_item_boundaries(body)
    print(f"  found {len(boundaries)} test/helper boundaries")
    chunks = chunk_body(body, boundaries)
    print(f"  splitting into {len(chunks)} chunks")

    sub_dir = PARTS_DIR / "tests"
    sub_dir.mkdir(parents=True, exist_ok=True)
    for old in sub_dir.glob("chunk_*.rs"):
        old.unlink()

    new_part_lines: list[str] = list(header_lines)
    for idx, (s, e) in enumerate(chunks, start=1):
        chunk_path = sub_dir / f"chunk_{idx:02d}.rs"
        chunk_path.write_text("".join(body[s:e]), encoding="utf-8")
        new_part_lines.append(f'    include!("tests/chunk_{idx:02d}.rs");\n')
        print(f"    chunk_{idx:02d}.rs: {e - s} lines")
    new_part_lines.extend(footer_lines)
    part_path.write_text("".join(new_part_lines), encoding="utf-8")
    print(f"  rewrote part_16.rs ({len(new_part_lines)} lines)")


if __name__ == "__main__":
    split_part_04()
    split_part_16()
