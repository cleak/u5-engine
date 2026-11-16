#!/usr/bin/env python3
"""The clue log's clean-room guard in CI (Phase B design 8.3.7, 8.6 T9b).

A player sidecar (`*.u5e`) is the player's own data: it may hold words a
person said, so none is ever committed, and nothing that exports or
packages files from a save folder may carry `journal.u5e`.

Checks, over the files git tracks:

1. no tracked path ends in `.u5e` (any case);
2. the only exporter that copies save folders, the runtime mirror
   (`crates/u5-tui/src/runtime_game_dir.rs`), skips the sidecar folder
   (its behaviour is pinned by `mirror_skips_the_enhancement_sidecar_folder`);
3. no packaging or QA tool names the sidecar folder or a `.u5e` file, so
   none can sweep one into an artifact.

Exit status 0 when clean, 1 with one line per problem otherwise.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

SIDECAR_DIR = ".u5-engine-enh"
MIRROR = "crates/u5-tui/src/runtime_game_dir.rs"
MIRROR_SKIP = "if is_enhancement_sidecar_dir(&entry.file_name()) {\n            continue;"
# Where packaging and export code lives; any mention of a sidecar there is
# a problem. This file names them itself and is left out.
EXPORT_PREFIXES = (".github/workflows/", "qa/tools/", "scripts/", "packaging/")


def tracked(root: Path) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout
    return [path for path in out.decode("utf-8").split("\0") if path]


def problems(root: Path, files: list[str]) -> list[str]:
    found = [f"{path}: a committed player sidecar" for path in files if path.lower().endswith(".u5e")]
    mirror = root / MIRROR
    if not mirror.is_file() or MIRROR_SKIP not in mirror.read_text(encoding="utf-8"):
        found.append(f"{MIRROR}: the runtime mirror no longer skips {SIDECAR_DIR}")
    me = Path(__file__).resolve()
    for path in files:
        if not path.startswith(EXPORT_PREFIXES) or (root / path).resolve() == me:
            continue
        if path.startswith("qa/tools/test_check_u5e_hygiene"):
            continue
        try:
            text = (root / path).read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        lower = text.lower()
        if SIDECAR_DIR in lower or ".u5e" in lower:
            found.append(f"{path}: names a player sidecar ({SIDECAR_DIR} or .u5e)")
    return found


def main(argv: list[str]) -> int:
    root = Path(argv[1]) if len(argv) > 1 else Path(__file__).resolve().parents[2]
    found = problems(root, tracked(root))
    for line in found:
        print(f"error: {line}")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
