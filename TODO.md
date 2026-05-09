# Ultima V First-Playable TODO

This repository is currently a clean-room Rust verification and terminal
first-playable harness for Ultima V. It is not yet the final Bevy game. The
current harness reads local clean asset files at runtime, keeps copyrighted game
data out of the repo, and implements a substantial amount of movement,
animation, save/load, command routing, and transition behavior behind focused
tests.

Last known verification state:

- `cargo fmt -- --check` passed.
- `cargo test` passed with 628 tests.
- `cargo run -- --play-script "z;q" C:\Games\U5-Clean` ran successfully.
- `cargo run -- --help` is not a supported CLI path and returns `unknown option
  --help` after building successfully.

Current worktree context when this TODO was written:

- `README.md` modified.
- `reports/lb-throne-room-slice.txt` modified.
- `src/main.rs` modified.
- `AGENTS.md` untracked.

Before starting a major slice, inspect `git status --short` and preserve user
changes. Do not revert unrelated work.

## Clean-Room Constraints

- Use only:
  - this repository,
  - the public spec at `C:\Projects\Rust\u5-spec`,
  - local clean asset files at `C:\Games\U5-Clean`,
  - user-authored clean-room descriptions or tests.
- Do not inspect decompiled or disassembled source.
- Do not decompile, disassemble, or reverse engineer the original binaries.
- Do not commit original game assets, raw map dumps, dialogue transcripts,
  private offsets, or copyrighted extracted content.
- Runtime asset reads are acceptable; reports should stay aggregate,
  diagnostic, or hash-based unless the user supplies clean-room-safe expected
  values.
- When exact original behavior is not public, prefer a sidecar table or
  deterministic first-playable approximation and document the gap clearly.

## Immediate Next Actions

These are the safest next slices for a new contributor.

1. Review and checkpoint the current worktree.
   - Run `git status --short`.
   - Review `git diff -- src/main.rs README.md reports/lb-throne-room-slice.txt`.
   - Separate intentional engine changes from generated report changes.
   - Decide whether `AGENTS.md` should be committed or left local.
   - Re-run `cargo test` with `RUSTC_WRAPPER` cleared:
     ```powershell
     $env:RUSTC_WRAPPER=''; cargo test
     ```

2. Clean up naming around dungeon climb.
   - `src/main.rs` still has `climb_dungeon_or_placeholder`.
   - The function now implements real dungeon ladder behavior plus safe refusal
     for missing clean return metadata.
   - Rename it to something like `climb_dungeon`.
   - Update any tests or comments that still imply it is only a placeholder.
   - Add no behavior changes in the rename slice.

3. Add a short CLI usage path.
   - `--help` currently returns an error.
   - Add a minimal usage print that lists documented smoke commands:
     - `cargo run -- C:\Games\U5-Clean`
     - `cargo run -- --play C:\Games\U5-Clean`
     - `cargo run -- --play-script "z;q" C:\Games\U5-Clean`
     - `cargo run -- --play --scene DUNGEON:0 --floor 0 C:\Games\U5-Clean`
   - Add parser tests for `--help` and `-h`.
   - Keep this as a no-asset path so it can run anywhere.

4. Make one small command-placeholder improvement.
   - Current explicit placeholders include Ready, Yell, Attack, shop flow, and
     some combat handoffs.
   - Pick one command family and move it from "out of scope" to a clean,
     deterministic first-playable implementation.
   - Add focused tests for turn consumption, messages, state mutation, and
     mode-specific routing.

## Milestone 1: Terminal First-Playable Completion

Goal: finish the current terminal harness as a coherent, playable,
non-combat-first version before moving into Bevy.

### Command Routing

- Audit every top-level command in world, town, and dungeon modes.
  - Confirm whether it is implemented, intentionally refused, or still a
    placeholder.
  - Ensure implemented commands have tests for:
    - valid path,
    - invalid mode,
    - no-turn refusal,
    - consumed-turn failure,
    - interaction with post-turn tile effects.
  - Ensure command letters do not accidentally fall through to movement.

- Ready (`R`).
  - Current behavior reports out of scope.
  - Decide first-playable target:
    - no-op with inventory/equipment summary, or
    - minimal equip-slot mutation if public save layout is sufficient.
  - Add tests for town, world, and dungeon routing.
  - Confirm no turn is spent unless public spec says otherwise.

- Yell (`Y`).
  - Current behavior reports out of scope in some modes and sails in ship mode.
  - Separate ship sail toggle from generic Yell behavior.
  - Implement generic mode-appropriate response if public spec supports it.
  - Add tests that ship `Y` still toggles sails and non-ship `Y` does not.

- Attack (`A`).
  - Current top-down path reports out of scope.
  - For the non-combat first-playable milestone, decide whether Attack should:
    - refuse cleanly without turn when no target is present,
    - report target contact when an active object is adjacent,
    - or route to a future combat handoff placeholder.
  - Add tests for no target, adjacent target, and mode-specific behavior.

- Talk (`T`).
  - Current town talk reaches NPC envelopes and one-shot keyword lookup.
  - Remaining work:
    - full keyword loop,
    - conversation side effects,
    - shop trigger routing,
    - richer refusal paths for non-NPC targets.
  - Keep decoded `.TLK` content runtime-only; do not commit transcripts.

- Use (`U`).
  - Current first-playable supports torch, gem, key, and Moonstones.
  - Remaining work:
    - audit all usable inventory items in the public spec,
    - add safe refusals for unsupported items,
    - add tests for inventory mutation and turn rules,
    - map more item effects only when public clean-room behavior is available.

### Movement And Transitions

- World movement.
  - Already supports wrapping, passability, active-object blocking, vehicles,
    wind-driven ship behavior, waterfalls, damage sidecars, encounters, and
    plane transitions.
  - Remaining work:
    - replace sidecar-only transition coordinates where public default tables
      become available,
    - verify every vehicle transport against public passability rules,
    - add more route-level smoke scripts that cross multiple transition types,
    - audit horse stride edge cases around hazards, moongates, and encounters.

- Town movement.
  - Already supports stairs, trap doors, exit tiles, NPC blocking, schedules,
    doors, secret doors, fire sources, pickups, and floor reloads.
  - Remaining work:
    - exact town stair subtype table,
    - exact town boundary tile values,
    - exact trap-door/chute encodings,
    - exact item/tile pickup mappings,
    - richer interaction with shop/counter furniture.

- Dungeon movement.
  - Already supports facing-relative movement, turning, ladders, fall traps,
    bomb traps, fields, wind tiles, scripted teleports, exit tiles, heavy doors,
    room-trigger diagnostics, and text proxy view.
  - Remaining work:
    - rename `climb_dungeon_or_placeholder`,
    - complete exact dungeon exit-cell identities when public data is available,
    - replace sidecar-heavy-door rows with public low-nibble rules when safe,
    - implement room combat handoff or a stronger non-combat room outcome,
    - verify dungeon view/flood map edge cases against public spec.

- Area transition invariants.
  - Every command that changes scene, plane, floor, or dungeon level should
    return a transition outcome.
  - Transition outcomes should suppress destination underfoot effects for the
    same input when needed.
  - Ordinary turn-consuming movement-like actions should still run normal
    post-turn effects.
  - Add regression tests when touching:
    - Gate Travel,
    - moongates,
    - waterfalls,
    - world plane transitions,
    - town trap doors/exits,
    - dungeon teleports/exits,
    - climb transitions.

### Sidecar Metadata Reduction

The harness currently uses clean-room sidecar TSV files when exact original
tables are not yet public or not yet encoded.

- Inventory all sidecar files currently supported:
  - `world_locations.tsv`
  - `world_plane_transitions.tsv`
  - `world_get_tiles.tsv`
  - `object_pickups.tsv`
  - `world_waterfalls.tsv`
  - `world_damage_tiles.tsv`
  - `world_encounters.tsv`
  - `shrines.tsv`
  - `dungeon_deeper_transitions.tsv`
  - `dungeon_teleports.tsv`
  - `dungeon_wind_tiles.tsv`
  - `dungeon_exit_tiles.tsv`
  - `dungeon_doors.tsv`
  - `dungeon_chests.tsv`
  - `secret_doors.tsv`
  - `town_fire_sources.tsv`
  - `town_pushables.tsv`
  - `town_get_tiles.tsv`
  - `town_rest_beds.tsv`
  - `town_stairs.tsv`
  - `town_trap_doors.tsv`
  - `town_exit_tiles.tsv`
  - `town_locks.tsv`
  - `blink_targets.tsv`
  - `moongates.tsv`
  - `location_floor_pages.tsv`
  - `location_entry_y.tsv`

- For each sidecar:
  - identify whether the public spec now has enough information to implement a
    default table,
  - keep sidecar override support for focused tests,
  - add tests for missing file, malformed row, duplicate row, tile guard match,
    tile guard mismatch, and normal behavior fallback,
  - update README when sidecar behavior changes.

## Milestone 2: Save/Load Completeness

Goal: make save/load robust enough that a player can leave and resume a
first-playable session without losing supported state.

- Preserve unknown bytes.
  - Keep round-tripping unmapped `SAVED.GAM` and `SAVED.OOL` bytes.
  - Add regression tests whenever a new save field is written.
  - Prefer patching known fields over reserializing an invented save image.

- Active objects.
  - Existing work preserves embedded active-object rows and relinks scheduled
    NPCs.
  - Remaining work:
    - audit all active-object mutations for save inclusion,
    - verify world overlay cache behavior across plane transitions,
    - verify parked vehicle persistence after board/exit/fire/removal,
    - test save/load after town floor changes and dungeon transitions.

- Party state.
  - Remaining work:
    - persistent party order table once public,
    - complete equipment/readied-item fields,
    - exact status byte transitions for all supported spells and hazards,
    - exact HP/MP recovery and damage formulas where still first-playable.

- Quest and shrine state.
  - Current shrine implementation uses public ordained/Codex masks and
    first-playable standing.
  - Remaining work:
    - exact shrine standing byte layout,
    - complete quest flags related to doors, NPCs, and permanent world changes,
    - save/load tests for shrine completion and Codex turn-in.

- Vehicles and timing tags.
  - Remaining work:
    - exact ship facing/sail marker variants,
    - exact hull/skiff persistence,
    - all timing/status tags beyond currently recognized `Q` and `T`,
    - save/load after wind-driven movement and vehicle transitions.

## Milestone 3: Rendering And Presentation

Goal: move from diagnostic terminal rendering to a real playable visual
experience.

### Bevy Integration

- Establish a Bevy app shell.
  - Game state resource wraps or adapts the existing `PlayState`.
  - Input systems call existing command handlers.
  - Rendering systems consume state without duplicating gameplay rules.
  - Asset loading uses local `C:\Games\U5-Clean` files at runtime.

- Separate engine core from presentation.
  - Move pure parsing/model/gameplay code out of `main.rs`.
  - Suggested modules:
    - `assets`
    - `save`
    - `world`
    - `town`
    - `dungeon`
    - `commands`
    - `magic`
    - `active_objects`
    - `render`
    - `cli`
  - Keep terminal harness as a test/debug frontend.

- Build a top-down renderer.
  - Render world and town tiles from decoded tile sheets.
  - Render active objects with phase animation.
  - Respect line-of-sight and light radius.
  - Show status/message panels.
  - Verify with screenshots or pixel hashes where practical.

- Build a dungeon renderer.
  - Replace current text proxy with public first-person wireframe rendering.
  - Implement distance bands, side-wall mirroring, door/wall variants, and
    darkness gate.
  - Keep a diagnostic text view for tests.

- UI and input.
  - Implement prompt modes for spell names, directions, yes/no choices, talk
    keywords, mix recipes, save prompts, and dungeon exit prompts.
  - Preserve typeahead/buffer behavior where currently modeled.
  - Add scripted integration tests for common flows.

### Exact Visual Parity Deferrals

- Public spec still calls out optional exactness gaps:
  - story step-1 rectangle transition helper timing,
  - Return-to-View raster/pacing internals,
  - broader `EGA.DRV` behavior,
  - exact remote-view panel for X-Ray/Peer,
  - exact dungeon minimap glyph/floodability edge cases.
- These are not required for a first playable, but should be tracked if visual
  parity becomes the target.

## Milestone 4: Magic And Effects

Goal: finish non-combat spell effects and keep combat-only spells safely routed
until combat exists.

- Spell parser and resources.
  - Existing parser/resource gates are broad and heavily tested.
  - Continue adding tests for:
    - scene gates before charge consumption,
    - charge before mana/level where public spec requires asymmetry,
    - successful resource mutation,
    - failed cast turn rules.

- Implement remaining non-combat effects.
  - Audit all 48 spells against `C:\Projects\Rust\u5-spec\systems\magic.md`.
  - For each spell, classify:
    - already implemented,
    - implemented as first-playable approximation,
    - combat-only,
    - unimplemented but allowed,
    - out-of-scene refusal.

- Known approximations/gaps.
  - Heal uses fixed first-playable HP amount.
  - Great Heal and Resurrect use max HP until exact math is needed.
  - Create Food uses first-playable fixed amount and cap behavior remains open.
  - Rel Hur wind order is deterministic but exact original order remains open.
  - Blink default range is sidecar-authored.
  - X-Ray and Peer use first-playable map projections.
  - Dungeon escape-helper spell split remains open.
  - Combat-side active-effect consumers remain out of scope.

- Gate Travel.
  - Recently fixed to report a transition outcome so destination underfoot
    transitions do not retrigger during the same cast.
  - Keep regression tests for:
    - world source,
    - town source,
    - dungeon source,
    - shipboard refusal,
    - invalid slot,
    - landing on a transition coordinate.

## Milestone 5: Combat Handoff And Combat

Goal: decide whether to keep combat out of scope for first-playable or implement
a minimal combat loop.

- Current combat placeholders.
  - Hostile world object contact reports combat out of scope.
  - Dungeon room triggers report arena diagnostics and mark room-helper state.
  - Dungeon room-helper state fires before the next key but does not enter
    combat.
  - Some spells are parsed but only meaningful once combat actors exist.

- Minimal combat handoff.
  - Load arena metadata safely from local assets at runtime.
  - Snapshot current active-object table.
  - Switch to combat scene/state.
  - Place party and enemy actors.
  - Provide an immediate debug resolution path for first-playable testing.
  - Restore prior world/town/dungeon state on exit.
  - Add tests for active-object save/restore and coordinate preservation.

- Full combat loop.
  - Actor initiative/phase model.
  - Player movement and targeting.
  - Monster AI.
  - Combat field placement and contact.
  - Damage, defense, status, death, rewards, loot, and escape.
  - Monster combat-AI runner instruction set and class effect map are still
    called out as remaining public-spec parity work.

## Milestone 6: Content Systems

Goal: turn diagnostic interactions into game-like content.

- Shops.
  - Current talk can identify shop triggers but shop UI is out of scope.
  - Implement:
    - shop type detection,
    - buy/sell menus,
    - price tables,
    - inventory updates,
    - gold validation,
    - clean refusal/cancel paths.

- Containers and pickups.
  - Current sidecars can grant food, gold, keys, gems, and torches.
  - Remaining work:
    - original tile-object item-code mapping,
    - chest content/trap generator,
    - dungeon chest trap effects,
    - object pickup semantics for all relevant active-object families,
    - persistence rules after pickup.

- Doors, locks, and secrets.
  - Current sidecars cover town locks, dungeon doors, and secret doors.
  - Remaining work:
    - exact surface lock-state byte pairs,
    - exact dungeon low-nibble split,
    - lockpick formulas and key-break rules,
    - NPC pickpocket rewards and failure consequences,
    - cannon/fire durability details.

- NPC schedules and conversations.
  - Current schedules link and move NPCs in town-family scenes.
  - Remaining work:
    - complete cached-waypoint arrival semantics,
    - floor-changing schedule paths through stairs,
    - conversation keyword loops and side effects,
    - shop/service conversations,
    - NPC memory flags such as thanked/picked/quest state.

- Encounters.
  - Current world encounters can spawn via sidecar after consumed turns.
  - Remaining work:
    - exact terrain threshold and monster tables,
    - encounter suppression/eligibility rules,
    - ambush checks during rest,
    - transition into combat or debug resolution.

## Milestone 7: Data And Asset Coverage

Goal: keep asset readers complete while preserving repository cleanliness.

- Existing local decoders cover many files and LZW resources.
- Continue adding tests that verify:
  - declared decoded lengths,
  - parser shape,
  - aggregate hashes or counts,
  - no raw asset dumps in repo outputs.

- Areas to audit:
  - all `.DAT` map and metadata files used by first-playable systems,
  - all `.OOL` active-object overlays,
  - tile sheets and masks,
  - fonts,
  - proportional text assets,
  - bitmap resources used by eventual Bevy UI,
  - any distribution variants the user wants to support.

- Do not add audio/music unless a target distribution actually includes it.
  - Public spec notes `.XMI` music is not present in the analyzed clean DOS
    baseline.

## Milestone 8: Testing Strategy

Goal: keep the project easy to change safely.

- Preserve fast unit tests.
  - The current suite is large but fast.
  - Continue writing narrow tests for every new behavior.

- Add scenario tests.
  - Scripted play sessions should cover:
    - world to town to world,
    - world to dungeon to world,
    - dungeon ladder chains,
    - town floor changes,
    - vehicle board/move/exit,
    - save/reload after movement,
    - Gate Travel,
    - moongates,
    - waterfall/plane transition interactions,
    - rest and light decay.

- Add artifact checks.
  - Ensure reports do not include raw copyrighted content.
  - Add tests or scripts that scan generated report files for accidental raw
    dumps if feasible.

- Add frontend tests once Bevy exists.
  - Smoke launch.
  - Screenshot or frame-hash checks for known scenes.
  - Input flow tests for prompts.
  - No overlapping UI text in common panels.

## Milestone 9: Documentation And Handoff

Goal: make the project approachable without reading the entire codebase.

- Split the README.
  - Keep quick start at the top.
  - Move sidecar reference into `docs/sidecars.md`.
  - Move command reference into `docs/commands.md`.
  - Move architecture notes into `docs/architecture.md`.

- Add a first-playable status matrix.
  - Rows: each command/system.
  - Columns:
    - world,
    - town,
    - dungeon,
    - implemented,
    - first-playable approximation,
    - sidecar required,
    - tests,
    - public-spec gap.

- Document common dev commands.
  - `cargo fmt -- --check`
  - `cargo test`
  - `cargo run -- C:\Games\U5-Clean`
  - `cargo run -- --play C:\Games\U5-Clean`
  - `cargo run -- --play-script "z;q" C:\Games\U5-Clean`
  - Note that `RUSTC_WRAPPER` may need clearing if it points to missing
    `sccache`:
    ```powershell
    $env:RUSTC_WRAPPER=''; cargo test
    ```

- Keep clean-room provenance clear.
  - Every behavior derived from public spec should point to the relevant spec
    file.
  - Every behavior that is first-playable-only should say so.
  - Every exactness gap should say whether it is blocked on public spec,
    intentional v1 deferral, or Bevy presentation work.

## Completion Criteria For The Original Goal

The active project goal is not complete until these are true:

- A user can start a new testing session without intro or character creation.
- The player is dropped directly into gameplay.
- World, town, and dungeon movement are playable through the intended frontend.
- Animation ticks are visible and integrated with movement/idle timing.
- Area transitions work across:
  - world to town,
  - town to world,
  - world to dungeon,
  - dungeon to world,
  - town floor changes,
  - dungeon level changes,
  - Britannia/Underworld plane changes,
  - moongate or Gate Travel teleport paths.
- Save/load can preserve the supported first-playable session state.
- Combat, intro, and character creation may remain absent or stubbed, but their
  absence must not break movement and transition play.
- The project builds and runs from documented commands.
- Tests cover all critical movement, animation, and transition paths.
- The implementation remains clean-room safe.

Do not mark the goal complete just because tests pass. Run a completion audit
against the criteria above and verify each item with code, tests, command
output, or a real scripted/playable session.
