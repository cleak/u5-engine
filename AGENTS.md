## Knowledge Logging - Mandatory

If `journal/capture/notes.py` exists in this repository, use it to journal work.
A session with no journal entries is a failed session when the journal hook is
present.

Log a `plan` note early, before writing code:

```powershell
python journal/capture/notes.py --agent codex plan "Plan summary"
```

Log findings, decisions, problems, tool notes, and risks as they happen:

```powershell
python journal/capture/notes.py --agent codex finding "What was found"
python journal/capture/notes.py --agent codex decision "What was decided and why"
python journal/capture/notes.py --agent codex dead_end "What did not work"
python journal/capture/notes.py --agent codex tool "Tool outcome"
python journal/capture/notes.py --agent codex risk "Risk or uncertainty"
```

Log a `final` note before stopping:

```powershell
python journal/capture/notes.py --agent codex final "Summary of outcome"
```

Allowed kinds: `plan`, `finding`, `decision`, `dead_end`, `tool`, `risk`,
`final`.

Keep entries terse and factual. Never log secrets, tokens, passwords, large
diffs, raw game-data dumps, or copyrighted content. Append only; never edit or
delete previous entries.

If the journal exists but has not been bootstrapped, run:

```powershell
python journal/capture/notes.py --bootstrap --agent codex
```

## Project Context

This repository is a clean-room implementation of the classic DOS game Ultima V.
The implementation target is Rust with Bevy.

Current local paths:

- Clean workspace: `C:\Projects\Rust\u5-clean`
- Engine repository: `C:\Projects\Rust\u5-clean\u5-engine`
- Clean asset files: `C:\Games\U5-Clean`
- Current public/specification repository: `C:\Projects\Rust\u5-clean\u5-spec`

The specification repository is read-only from this clean workspace. Agents may
read `C:\Projects\Rust\u5-clean\u5-spec` as clean input, but must not create,
edit, delete, stage, commit, or pull changes there unless the user explicitly
directs that specific operation.

The repository currently contains a narrow Rust verification harness. It reads
local game assets at runtime and writes sanitized aggregate reports. The
repository must not include original game assets, raw map dumps, dialogue
transcripts, binary offsets copied from private analysis, or copyrighted content.

## Clean-Room Rules

- Do not look directly at decompiled source.
- Do not decompile, disassemble, or reverse engineer the original game.
- Do not derive implementation details from private/decompiled-source analysis.
- Do not inspect sibling or external private-analysis workspaces, original
  source repositories, disassembly repositories, or decompiler output.
- Do not ask another agent to inspect private analysis for this repository.
- Use only clean-room inputs: this repository, the current public spec in
  `C:\Projects\Rust\u5-clean\u5-spec`, and the user's local asset files in
  `C:\Games\U5-Clean`.
- Asset reads must be runtime/local and should avoid emitting raw data. Reports
  should stay aggregate, diagnostic, or hash-based unless the user explicitly
  provides clean-room-safe expected values.
- If a requested task appears to require private/decompiled-source knowledge,
  stop and ask for a clean-room-safe spec, test, or user-authored description.
- Communication from private analysis to this repository must happen through
  clean spec updates pulled into `C:\Projects\Rust\u5-clean\u5-spec`, or through
  user-provided clean-room-safe descriptions.
- Do not edit `C:\Projects\Rust\u5-clean\u5-spec` from engine work. If the spec
  needs changes, ask the user for the clean-room-safe update path.

## Engineering Direction

- Prefer small, verifiable slices that compare implementation behavior against
  the public spec and local assets without copying asset content into the repo.
- Keep parsers and model code separated from Bevy presentation code as the
  project grows.
- Add focused tests for deterministic parsing, scene binding, data validation,
  and gameplay rules as public spec coverage expands.
- Preserve user changes in the worktree. Do not revert unrelated edits.
