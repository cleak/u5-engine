#!/usr/bin/env python3
"""Report engine comments quoting spec prose that spec HEAD no longer has.

`cleak/u5-engine#2`, the 2026-08-26 conformance audit, named the process
defect this exists to answer:

    a passing test is not evidence of conformance when the test was
    written against a spec revision that has since been retracted. Any
    future audit must re-fetch document text from spec HEAD rather than
    trusting an engine doc comment.

The engine cites the specification constantly, and quotes it in doc
comments so a reader can check the rule without leaving the file. When
upstream retracts or rewrites a sentence, those quotes silently become
the *old* contract - and a doc comment quoting a retracted sentence
beside a test pinning the retracted behaviour is exactly how the audit's
findings survived unnoticed.

Usage:

    # fetch spec HEAD first - a local checkout is routinely behind
    gh api repos/cleak/u5-spec/tarball/master | tar xz -C /tmp/spec-head
    python3 qa/tools/spec_quote_audit.py /tmp/spec-head crates

What it does and does not tell you
----------------------------------

A hit means *this exact wording is not in any published document*. It
does **not** mean the engine is wrong. Three things produce hits, and
only the first is a defect:

1. The spec retracted or rewrote the rule. The comment now describes
   behaviour that is no longer the contract, and the code beside it may
   too. This is what the tool is for.
2. The spec kept the rule and reworded it. The behaviour is right and
   the quote needs refreshing.
3. The comment paraphrases, reorders or joins clauses, quotes the
   retracted wording *inside* a note that says it is retracted, or the
   quote marks in a long reflowed comment do not bound what the author
   meant to quote. False positive.

Precision is modest by construction. On the 2026-09-06 run it reported
165 hits out of 2,450 quoted runs, and the sample triaged by hand was
category 2 or 3 every time - including sites that already carry the
correction beside the retracted sentence they quote. Treat the output as
a reading list, never as a defect list.

So triage every hit against the cited document by hand. Comparison
ignores punctuation, case and whitespace, because comment reflow and the
spec's typographic quotes and minus signs would otherwise dominate the
output.
"""

import pathlib
import re
import sys
import unicodedata

QUOTE = re.compile(r"[\"“]([^\"“”]{60,600}?)[\"”]")
MIN_WORDS = 12
WINDOW = 8


def normalise(text: str) -> str:
    """Letters, digits and single spaces only."""
    text = unicodedata.normalize("NFKD", text).lower()
    text = re.sub(r"[^a-z0-9 ]+", " ", text)
    return re.sub(r"\s+", " ", text).strip()


def comment_blocks(path: pathlib.Path):
    """Yield `(first_line_number, joined_text)` per run of comment lines."""
    block: list[str] = []
    start = 0
    for number, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("//"):
            if not block:
                start = number
            block.append(stripped.lstrip("/").strip())
        elif block:
            yield start, " ".join(block)
            block = []
    if block:
        yield start, " ".join(block)


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    spec_root = pathlib.Path(sys.argv[1])
    engine_root = pathlib.Path(sys.argv[2])
    corpus = "\n".join(
        normalise(path.read_text(errors="replace")) for path in spec_root.rglob("*.md")
    )
    if not corpus:
        print(f"no .md files under {spec_root}", file=sys.stderr)
        return 2

    checked = 0
    stale = []
    for path in sorted(engine_root.rglob("*.rs")):
        if "/target/" in str(path):
            continue
        for start, text in comment_blocks(path):
            for quote in QUOTE.findall(text):
                if not re.match(r"^[A-Za-z(\[]", quote.strip()):
                    continue
                # Editorial marks a quoting author adds - `Move[s]`,
                # `commit[s]` - are not in the source sentence.
                words = normalise(re.sub(r"\[[^\]]{0,12}\]", "", quote)).split()
                if len(words) < MIN_WORDS:
                    continue
                checked += 1
                windows = (
                    " ".join(words[index : index + WINDOW])
                    for index in range(max(1, len(words) - WINDOW + 1))
                )
                if any(window in corpus for window in windows):
                    continue
                stale.append((path, start, quote))

    print(
        f"checked {checked} quoted runs of {MIN_WORDS}+ words; "
        f"{len(stale)} have no {WINDOW}-word run anywhere in the published spec"
    )
    for path, start, quote in stale:
        print(f"\n{path}:{start}\n  {quote[:240]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
