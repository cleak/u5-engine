# Architecture

`u5-engine` is a clean-room Rust implementation that consumes the public
specification in `../u5-spec` and local user-owned game assets at runtime. The
repository must not contain original asset dumps, dialogue transcripts,
decompiled source, private offsets, or generated tables copied from protected
material.

## Crates

| Crate | Role |
|---|---|
| `u5-runtime` | Core gameplay state, parsers, asset readers, command handling, save/load, combat, shops, conversations, magic, rendering helpers, and tests. |
| `u5-tui` | Terminal/debug frontend, CLI parser, scripted play runner, intro loop, save-frame helper, and raster diagnostics. |
| `u5-bevy` | Feature-gated visual frontend that renders `PlayState` through atlas-backed top-down, dungeon, combat, intro, and modal panels. |

`PlayState` is the gameplay authority. Frontends translate input into runtime
commands and render state; they should not duplicate movement, combat, inventory,
conversation, or save rules.

## Runtime Data Flow

1. Startup options choose a source: direct scene, `INIT.GAM`, `SAVED.GAM`,
   scripted debug entry, or the intro/menu dispatcher.
2. Asset readers decode local files from the user-supplied game directory.
   Runtime reads are allowed; committed extracted content is not.
3. Optional clean-room sidecars next to the game assets provide public or
   user-authored metadata whose exact original resident table is not published.
4. `PlayState` owns command dispatch, turn advancement, post-turn effects,
   transition outcomes, save/load state, and modal prompt sessions.
5. TUI and Bevy frontends render the same state and feed the same handlers.

## Clean-Room Boundaries

- `../u5-spec` is read-only in this workspace.
- Use public spec prose/data, engine code, user-authored clean-room knowledge,
  and local runtime asset observations only.
- Do not inspect decompiled or disassembled source, private analysis notes, raw
  address tables, or `u5-decomp`.
- If the public spec omits exact behavior, keep the runtime conservative,
  preserve sidecar override support where useful, and document the gap.

## Verification Layers

| Layer | Typical command |
|---|---|
| Runtime unit tests | `cargo test -p u5-runtime` |
| CLI/TUI smoke tests | `cargo test -p u5-tui` |
| Formatting | `cargo fmt -- --check` |
| Scripted play smoke | `cargo run -- --play-script "d;empty;q" C:\Games\U5-Clean` |
| Raster hash smoke | `cargo run -- --play-script "idle:1;q" --raster-diagnostics C:\Games\U5-Clean` |
| Headless frame capture | `cargo run -- --save-frame screenshots\britannia.png --scene BRITANNIA C:\Games\U5-Clean` |
| Bevy frame suite | `cargo run --features visual -- --visual-frame-suite target\visual-frame-suite C:\Games\U5-Clean` |
| Bevy visual smoke | `cargo run --features visual -- --visual --scene BRITANNIA C:\Games\U5-Clean` |

When changing shared gameplay behavior, prefer focused runtime tests first and
then run the full `u5-runtime` suite before committing.
