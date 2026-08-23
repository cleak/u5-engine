# Architecture

`u5-engine` is a clean-room Rust implementation that consumes the public
`cleak/u5-spec` specification and local user-owned game assets at runtime. The
repository must not contain original asset dumps, dialogue transcripts,
decompiled source, private offsets, or generated tables copied from protected
material.

Read spec text from GitHub, not from the local `../u5-spec` checkout: that
checkout is read-only from this workspace and is stale by many commits and
several retractions. Use the issues, and
`gh api -H "Accept: application/vnd.github.raw" repos/cleak/u5-spec/contents/<path>`
for document text.

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

Asset-backed layers take `<asset-copy>`: a scratch **copy** of the asset
directory, never `C:\Games\U5-Clean` itself. The install is a read-only
clean-room input, these harness paths take a directory they both read and write,
and it has been corrupted that way before. The engine refuses a write
destination resolving to `DEFAULT_GAME_DIR`, and `copy_asset_writable` clears
the read-only bit Windows `fs::copy` propagates into scratch copies.

| Layer | Typical command |
|---|---|
| Runtime unit tests | `cargo test -p u5-runtime --lib` |
| CLI/TUI smoke tests | `cargo test -p u5-tui --features visual` |
| Bevy tests | `cargo test -p u5-bevy` |
| Formatting | `cargo fmt --all -- --check` |
| Lints | `cargo clippy --workspace --all-targets` |
| Scripted play smoke | `cargo run -- --play-script "d;empty;q" <asset-copy>` |
| Raster hash smoke | `cargo run -- --play-script "idle:1;q" --raster-diagnostics <asset-copy>` |
| Headless frame capture | `cargo run -- --save-frame screenshots\britannia.png --scene BRITANNIA <asset-copy>` |
| Scripted route smoke | `cargo run --features visual -- --route-smoke <asset-copy>` |
| Bevy frame suite | `cargo run --features visual -- --visual-frame-suite target\visual-frame-suite <asset-copy>` |
| Bevy route suite | `cargo run --features visual -- --visual-route-suite target\visual-route-suite <asset-copy>` |
| Bevy visual smoke | `cargo run --features visual -- --visual --scene BRITANNIA <asset-copy>` |

When changing shared gameplay behavior, prefer focused runtime tests first and
then run the full `u5-runtime` suite before committing. `docs/status-matrix.md`
carries the current measured counts for each layer.
