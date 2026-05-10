"""Convert parts/play_state_impl/chunk_NN.rs and parts/tests/chunk_NN.rs
into proper Rust modules.

Each chunk is wrapped (or not) and lacks `use` declarations; we add a
`use crate::*;` plus the standard std imports so cross-references resolve."""

from __future__ import annotations

from pathlib import Path
import re

OLD_PSI = Path("crates/u5-runtime/src/parts/play_state_impl")
NEW_PSI = Path("crates/u5-runtime/src/play_state_impl")
OLD_TESTS = Path("crates/u5-runtime/src/parts/tests")
NEW_TESTS = Path("crates/u5-runtime/src/tests")


COMMON_USES = (
    "use std::collections::{HashMap, VecDeque};\n"
    "use std::fs;\n"
    "use std::io;\n"
    "use std::path::{Path, PathBuf};\n"
    "\n"
    "use crate::*;\n"
    "\n"
)

TEST_USES = COMMON_USES.replace(
    "use crate::*;\n",
    "use crate::*;\nuse crate::test_fixtures::*;\n",
)


def main() -> int:
    NEW_PSI.mkdir(parents=True, exist_ok=True)
    psi_chunks = sorted(OLD_PSI.glob("chunk_*.rs"))
    psi_names: list[str] = []
    for cf in psi_chunks:
        text = cf.read_text(encoding="utf-8")
        # Each chunk starts with `impl PlayState {` and ends with `}`.
        # We prepend the use declarations.
        out = COMMON_USES + text
        target = NEW_PSI / cf.name
        target.write_text(out, encoding="utf-8")
        psi_names.append(cf.stem)
    # Build mod.rs declaring each chunk as a module.
    mod_rs = (
        "//! Implementation methods for `PlayState`, split across chunks for "
        "the <1000-line rule.\n"
        "//!\n"
        "//! These were originally `parts/play_state_impl/chunk_NN.rs` and "
        "lived under an `include!` wrapper; they're now proper modules each "
        "wrapping their own `impl PlayState { ... }` block.\n"
        "\n"
    )
    for n in psi_names:
        mod_rs += f"mod {n};\n"
    (NEW_PSI / "mod.rs").write_text(mod_rs, encoding="utf-8")
    print(f"Wrote {NEW_PSI}/mod.rs with {len(psi_names)} chunk modules")

    NEW_TESTS.mkdir(parents=True, exist_ok=True)
    test_chunks = sorted(OLD_TESTS.glob("chunk_*.rs"))
    test_names: list[str] = []
    for cf in test_chunks:
        text = cf.read_text(encoding="utf-8")
        # Strip the `pub use crate::test_fixtures::*;` line that was inserted
        # earlier when the chunks lived under `pub mod tests`. The new file
        # already has its own `use crate::test_fixtures::*;` in its preamble.
        text = re.sub(
            r"^\s*pub use crate::test_fixtures::\*;\s*\n",
            "",
            text,
            count=1,
            flags=re.MULTILINE,
        )
        # The chunk lines were originally indented by 4 spaces (inside `mod tests`).
        # Outdent.
        outdented = "".join(
            (line[4:] if line.startswith("    ") else line)
            for line in text.splitlines(keepends=True)
        )
        out = TEST_USES + outdented
        target = NEW_TESTS / cf.name
        target.write_text(out, encoding="utf-8")
        test_names.append(cf.stem)

    mod_rs = (
        "//! u5-runtime internal tests, split across chunks for the <1000-line rule.\n"
        "//!\n"
        "//! These were originally `parts/tests/chunk_NN.rs` and lived under an "
        "`include!` wrapper inside `pub mod tests` of part_16.rs.\n"
        "\n"
    )
    for n in test_names:
        mod_rs += f"mod {n};\n"
    (NEW_TESTS / "mod.rs").write_text(mod_rs, encoding="utf-8")
    print(f"Wrote {NEW_TESTS}/mod.rs with {len(test_names)} chunk modules")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
