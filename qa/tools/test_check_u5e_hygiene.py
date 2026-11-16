"""Self-test of check_u5e_hygiene.py over synthetic trees; no git needed."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_u5e_hygiene as hygiene  # noqa: E402


class HygieneTest(unittest.TestCase):
    def tree(self, files: dict[str, str]) -> Path:
        root = Path(tempfile.mkdtemp(prefix="u5e-hygiene-"))
        for path, text in files.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text, encoding="utf-8")
        return root

    def mirror(self) -> dict[str, str]:
        return {hygiene.MIRROR: "fn m() {\n        " + hygiene.MIRROR_SKIP + "\n}\n"}

    def test_a_clean_tree_passes(self):
        files = {**self.mirror(), "qa/tools/x.py": "print(1)\n"}
        root = self.tree(files)
        self.assertEqual(hygiene.problems(root, list(files)), [])

    def test_a_committed_sidecar_fails_in_any_case(self):
        files = {**self.mirror(), "saves/JOURNAL.U5E": "x"}
        root = self.tree(files)
        self.assertEqual(len(hygiene.problems(root, list(files))), 1)

    def test_an_exporter_naming_the_folder_fails(self):
        files = {**self.mirror(), "scripts/pack.sh": "cp -r saves/.u5-engine-enh out\n"}
        root = self.tree(files)
        self.assertEqual(len(hygiene.problems(root, list(files))), 1)

    def test_a_mirror_that_stops_skipping_fails(self):
        files = {hygiene.MIRROR: "fn m() {}\n"}
        root = self.tree(files)
        self.assertEqual(len(hygiene.problems(root, list(files))), 1)


if __name__ == "__main__":
    unittest.main()
