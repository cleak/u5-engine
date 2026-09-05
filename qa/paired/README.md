# Paired play-test scenarios

Each `.tsv` file drives the RemoteGameDev `game-dev-u5-paired` harness: the
same keystrokes go to the clean engine and to stock `ULTIMA.EXE` under the
pinned DOSBox Staging 0.82.2, both started from fresh writable copies of the
same asset set, and every `shot` step captures both windows side by side.

Columns are tab separated: `side` (`both`, `engine`, `dosbox`), `action`
(`wait` milliseconds, `key` xdotool key names separated by spaces, `type`
printable text, `shot` label, `note` text), `value`, and an optional caption.

Captures are black-box comparison evidence and stay private under
`~/artifacts/u5/paired/`; only sanitized `record.json` metadata may be copied
into evidence. Differences found this way are classified per the remote
development plan before any fix: engine defect, harness defect, spec gap,
asset/profile problem, or platform defect.

The DOS boot preamble (production card, title flourish, signature) is skipped
with a Space keypress on both sides; the engine accepts the same skip.
