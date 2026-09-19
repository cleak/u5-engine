#!/usr/bin/env python3
"""Find engine comments quoting specification text that has been withdrawn.

`cleak/u5-engine#2`'s process lesson, in its own words: "An engine doc
comment quoting the spec is not evidence either. The quote is precisely what
goes stale, and it is more convincing than bare code, so it is worse than no
citation at all." Every one of the 106 contracts that audit found was an
earlier spec revision the engine had implemented and then cited.

`cleak/u5-spec#149` asks upstream for a retraction index so that becomes "a
targeted check rather than a full sweep". `RETRACTIONS.md` is that index, so
this is that check: take every quoted phrase from its **withdrawn** column and
look for it in the engine's source.

A hit is not automatically a defect. A comment that quotes a withdrawn
sentence *in order to record its withdrawal* is doing the right thing - the
fixes in this repository all read that way. So hits whose surrounding text
acknowledges the retraction are reported separately from those that do not,
and only the second kind needs a look.

Usage: stale_citation_audit.py [--all]
       (from the engine repository root, with RETRACTIONS.md fetched from
       spec HEAD - the local checkout is stale and must not be used, which is
       the same lesson.)
"""

import pathlib
import re
import subprocess
import sys

ACKNOWLEDGEMENTS = ("withdraw", "retract", "corrected", "no longer", "earlier revision")
CONTEXT = 700


def retraction_index() -> str:
    """`RETRACTIONS.md` from spec HEAD, never from the local checkout."""
    result = subprocess.run(
        ["gh", "api", "repos/cleak/u5-spec/contents/RETRACTIONS.md", "--jq", ".content"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise SystemExit(f"could not fetch RETRACTIONS.md: {result.stderr.strip()}")
    import base64

    return base64.b64decode(result.stdout).decode("utf-8", "replace")


def withdrawn_phrases(index: str) -> list[tuple[str, str]]:
    out = []
    for line in index.splitlines():
        if not line.startswith("| R"):
            continue
        cells = [cell.strip() for cell in line.split("|")]
        if len(cells) < 7:
            continue
        rid, withdrawn = cells[1], cells[5]
        for quoted in re.findall(r'"([^"]{25,120})"', withdrawn):
            phrase = re.sub(r"[`*]", "", quoted).strip()
            if len(phrase) >= 25 and phrase.count(" ") >= 3:
                out.append((rid, re.sub(r"\s+", " ", phrase)))
    return out


def main() -> None:
    show_all = "--all" in sys.argv
    index = retraction_index()
    phrases = withdrawn_phrases(index)
    sources = [
        (path, re.sub(r"\s*(///|//)?\s+", " ", path.read_text(errors="ignore")))
        for path in pathlib.Path("crates").rglob("*.rs")
    ]

    stale, acknowledged = [], []
    for rid, phrase in phrases:
        for path, flat in sources:
            index_of = flat.find(phrase)
            if index_of < 0:
                continue
            window = flat[max(0, index_of - CONTEXT) : index_of + CONTEXT].lower()
            hit = (rid, str(path), phrase)
            if any(word in window for word in ACKNOWLEDGEMENTS) or rid.lower() in window:
                acknowledged.append(hit)
            else:
                stale.append(hit)
            break

    print(f"withdrawn phrases checked: {len(phrases)}")
    print(f"quoted in the engine, retraction acknowledged: {len(acknowledged)}")
    print(f"quoted in the engine, retraction NOT acknowledged: {len(stale)}")
    for rid, path, phrase in stale:
        print(f"  {rid:6} {path}\n         {phrase!r}")
    if show_all:
        print("\nacknowledged:")
        for rid, path, phrase in acknowledged:
            print(f"  {rid:6} {path}\n         {phrase!r}")
    sys.exit(1 if stale else 0)


if __name__ == "__main__":
    main()
