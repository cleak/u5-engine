# Ultima V Implementation TODO

This repository is the clean-room Rust implementation of Ultima V. It reads
local clean asset files at runtime, keeps copyrighted game data out of the repo,
and now includes broad runtime coverage for command routing, movement,
animation, save/load, combat, shops, conversation, character creation, intro
flows, magic, rest/camp, Blackthorn/Codex paths, and the terminal endgame.

This file is a working handoff checklist, not an authoritative status database.
Many older milestone bullets below were written during the first-playable phase.
Before treating any item as missing, verify current code with `rg`, read the
matching public spec section, and check focused tests. Do not implement from
this file alone.

Last known verification state:

- `cargo test -p u5-runtime` passed on 2026-05-19, including 2447 tests.
- `cargo test -p u5-tui --features visual` passed on 2026-05-19.
- `cargo test -p u5-bevy` passed on 2026-05-19, including 44 tests.
- `cargo fmt -- --check` passed on 2026-05-19 after the latest Rust changes.
- Representative raster smoke checks with local assets produced nonblank hashes:
  `BRITANNIA` top-down `fd923dc0f87a9f3c`, `BRITANNIA` after movement
  `1eb882f27b1d216c`, route-smoke Britannia movement `bef4c9fc1eecf9fb`,
  `CASTLE:0` top-down `be84488b7b199310`, and `DUNGEON:0` first-person
  `161ad48dd2a91725`.
- `cargo run -p u5-tui -- --route-smoke C:\Games\U5-Clean` passed on
  2026-05-19 with 59 scripted route cases covering world/town look and
  save-refusal prompts, surface/town/dungeon View overlays, Spyglass
  Britannia chunk-map overlay (`2ea15622716e09aa`), Peer and X-Ray overlays,
  U-Use utility items including Pocket Watch/Sextant/Magic Carpet
  (`c2f7ff2c1000c8fd`), HMS Cape plans (`8c425fda6007db98`), and Wooden Box
  (`d5684b90a48f2d73`),
  Shadowlord town entry (`1e04222a325a2f67`), Shadowlord-name Yell
  (`abad79297a559cd2`), Stonegate Shadowlord entry presentation
  (`c7190c94f55a2af2`),
  H-Hole-up rest in Britannia (`8d7e6e0336279317`) and a dungeon
  (`22ccb05a46f3140e`), Underworld startup, debug-entered town return to world,
  Underworld-to-town debug entry, ship X-it/skiff launch, hoisted-sail movement,
  dungeon turn/block movement, dungeon exit confirmation/refusal, a Doom room
  trigger that enters a combat raster viewport, dungeon Attack/Search/Get/
  Jimmy/Open/refusal command routing, and combat pass, Attack, Cast, Get,
  Jimmy, Open, Push, Klimb, Ready, Z-stats, refusal, View, Yell, X-it, and
  Search-prompt command routing, plus combat Look label-only routing
  (`25343578fe3b2a4d`), world dispatcher refusals (`2936fb4c6ff7e3ef`),
  town dispatcher refusals (`3126df0494b5870b`), town party overlay routes
  (`26d7ef40b57084af`), and terminal endgame missing-box
  confirmation (`ff6287fbb741bd85`) and Wooden Box victory confirmation
  (`0cf64339ccad08e1`).
- `cargo run -p u5-tui -- --save-frame-suite target\codex-frame-suite
  C:\Games\U5-Clean` passed on 2026-05-19 and wrote thirteen nonblank PNGs:
  `britannia` `859a1bdabe5c9b7a`, `britannia-step` `05b13e47da048fe6`,
  `castle` `bda625019405af09`, lit `dungeon` `91ea22aa5e09c692`,
  synthetic `combat` `b4cbe49dac94affd`, `surface-view`
  `68717971b9dc1fbf`, `dungeon-view` `e43eb41821c7a3b2`,
  `peer-view` `bd4c5606fd27c054`, `x-ray-view` `bd4c5606fd27c054`,
  `intro-menu` `7bf01c36de552e16`, `status-window`
  `bf7a428a4b00ad2b`, `z-stats-modal` `61b033bfa2488b46`, and
  `endgame-status` `532cb7f1bdd03ffd`.
- `cargo run -p u5-tui --features visual -- --visual-frame-suite
  target\codex-britannia-chunk-map-suite C:\Games\U5-Clean` passed on
  2026-05-19 and wrote seventeen nonblank Bevy-owned PNGs plus a sanitized
  manifest:
  `world-play` `f68b906acde0bd4a`, `world-after-step`
  `b9720ab18affa566`, `town-play` `2beb3b7734800e11`,
  `dungeon-play` `67e7e116d8be67aa`, `dungeon-dark`
  `29289813c0f0397c`, `combat-play` `9b1937b3e807ba05`,
  `surface-view-overlay` `c82bbd585b1d8f6b`, `dungeon-view-overlay`
  `e995f913cb07aca2`, `britannia-chunk-map-overlay`
  `2e843bf012b76297`, `peer-view-overlay` `34e217c3c9fdc23c`,
  `x-ray-view-overlay` `34e217c3c9fdc23c`, `z-stats-modal`
  `bee4e11801862ad1`, `endgame-status` `a2362c168f42d288`,
  `intro-menu` `74547d4e4d487e9c`, `intro-finished-menu`
  `fc0e64c23363f715`, `intro-story-art` `34dfde7e247537f4`, and
  `intro-return-to-view`
  `a52f8c3db8e33102`.
- Bevy visual screenshot smoke with local assets produced a nonblank
  `792x1182` PNG at `target\codex-fixed-font-status-smoke.png`.
- Bevy intro screenshot smoke with local assets produced a nonblank
  `1240x930` PNG at `target\codex-intro-title-tick-smoke.png`, capturing an
  early `BRITISH.PTH` signature-animation frame plus the title-tick overlay.
- Bevy unit framebuffer coverage includes world, town, dungeon, combat, intro
  title art, signature path rendering, title-tick strip bounds, finished-menu
  title-surface overlay, spec-ordered intro story transition/secondary art
  draws, status, endgame modal surfaces, and the deterministic visual
  frame-suite compositor.
- `--save-frame <PATH>` is the current headless PNG capture path for local
  asset visual checks.
- `cargo run -- --help` is a supported no-asset usage path.
- The latest checkpointed engine commit at the time of this refresh included
  container Search object-table and trap-narration parity work.
- The spec checkout used for the most recent audit was `5b816cc Complete
  cleanroom specification`.

Current worktree context when this TODO was refreshed:

- `git status --short` was clean in `u5-engine`.
- `git status --short` was clean in `u5-spec`.
- `journal/capture/notes.py` was not present in the workspace, engine, or spec
  repository.
- Town-family exit thresholds now prompt both when stepped onto and when
  observed underfoot after a consumed turn; accepting exits through clean
  return metadata, refusing leaves town mode active.
- Natural moongate live-tile refresh now keeps mode-zero scene/light cleanup
  from advancing the shared gate-presence counter. The remaining exact gap is
  the public moon-glyph phase table that feeds the cached Trammel/Felucca slot
  used by live-gate entry; current tests seed that cache directly. This is
  tracked in `cleak/u5-spec#38`; a 2026-05-19 public-source check confirmed
  the high-level destination order but not the DOS glyph table, byte-to-slot
  mapping, or cache refresh timing needed for implementation.
- Rest/camp HP/MP recovery amounts are blocked on clean spec clarification in
  `cleak/u5-spec#47`; the engine's prompt flow, time cadence, ambush predicate,
  status restoration, and Lord British camp event are spec-backed.
- Non-combat Blink default target selection is blocked on clean spec
  clarification in `cleak/u5-spec#48`; the engine keeps sidecar-authored
  destinations rather than inventing a range/search rule.
- Create Food's numeric grant is blocked on clean spec clarification in
  `cleak/u5-spec#49`; the current 100-food grant remains a documented local
  policy until the public amount is published.
- TLK `0x85` gold payments debit accepted affordable payments. The
  toll-style moral-standing milestone remains blocked on clean spec
  clarification in `cleak/u5-spec#27` because the public spec does not yet
  publish the toll-progress counter, milestone predicate, reset/increment
  rules, or qualifying payment contexts.
- Hourly poison and starvation damage use a one-HP first-playable policy while
  the exact public damage amount/range and per-member roll semantics are
  blocked on clean spec clarification in `cleak/u5-spec#50`.
- Town poison-gas doorway cells are implemented through clean sidecar metadata,
  but the exact public roll odds and per-member semantics are blocked on clean
  spec clarification in `cleak/u5-spec#51`.
- Route smoke now exercises a debug-enter world-to-castle-to-world round trip
  using clean return metadata in memory, an Underworld-to-castle entry,
  seeded ship/skiff sailing routes, a Spyglass-triggered Britannia chunk-map
  overlay, world/dungeon H-Hole-up rest routes, and direct U-Use routes for
  key utility items, plus broader dungeon and combat command branches; native
  exact coordinate-table coverage still depends on public gazetteer/sidecar
  rows.
- Combat AI now threads party-name faction grouping into live target scans,
  applies wound morale/flee inversion in production turns, honors Doom and
  Shadow Lord suppression-filter bypasses, mutates non-party Sleep Field
  disable state, prefers summon-daemon placement near the current step
  direction, and skips actors standing on loaded blocked arena terrain.
- Search now runs clean object-pickup table matches before live-tile scans,
  keeps active treasure-marker priority ahead of trap narration, and narrates
  surface/town object trap metadata without clearing the object slot.

Before starting a major slice, inspect `git status --short` and preserve user
changes. Do not revert unrelated work.

## Clean-Room Constraints

- Use only:
  - this repository,
  - the public spec at `C:\Projects\Rust\u5-clean\u5-spec`,
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

1. Review and checkpoint the current worktree before editing.
   - Run `git status --short` in both `u5-engine` and `u5-spec`.
   - Preserve unrelated local changes.
   - Re-run the relevant tests with `RUSTC_WRAPPER` cleared:
     ```powershell
     $env:RUSTC_WRAPPER=''; cargo test
     ```

2. Keep the CLI usage path covered.
   - `--help` and `-h` are supported no-asset paths.
   - Parser tests cover both flags, and a binary-level regression verifies
     `u5-engine --help` prints usage without touching local game assets.

3. Work from current code/spec gaps, not this file alone.
   - Ready, Yell, Attack, Use, Talk, shops, magic, rest/camp, combat handoff,
     Blackthorn/Codex progression, intro/chargen, and endgame have all moved
     beyond their early placeholder status.
   - Before implementing a remaining item below, confirm it is still absent with
     `rg`, read the matching clean spec section, and add focused tests for turn
     consumption, messages, state mutation, and mode-specific routing.

4. Do not invent exact transition coordinates.
   - The current clean spec still omits several exact overworld, moongate,
     dungeon-return, and special-transition coordinates.
   - Leave the safe "missing clean return-coordinate metadata" behavior in place
     until those coordinates are published through the spec.

5. Prefer completion-audit work over broad rewrites.
   - Map one explicit public spec behavior to current engine evidence.
   - If behavior is implemented, update or delete stale TODOs only when they
     would otherwise mislead the next implementation pass.
   - If behavior is missing and public enough to implement, add a focused test
     first or in the same patch.

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
  - Current behavior opens the Ready picker with carried-stock, ammunition,
    strength, occupied-slot, hand-occupancy, ring-vanish, and combat
    body-armour gates.
  - Remaining work:
    - audit any item-specific equipment rules newly published in the spec,
    - keep town, world, dungeon, and combat routing covered by tests,
    - confirm turn-spend behavior when parity details are published.

- Yell (`Y`).
  - Current behavior separates ship sail toggles from generic Yell input and
    supports dungeon words and Shadowlord names.
  - Remaining work:
    - audit any newly specified mode-specific Yell effects,
    - keep tests proving ship `Y` toggles sails and non-ship `Y` does not.

- Attack (`A`).
  - Current top-down paths prompt for direction, handle misses, report adjacent
    target contact, raise town alarms where appropriate, and can enter
    terrain-combat handoff for combat-class world objects.
  - Remaining work:
    - audit parity for non-hostile and special story-object attacks,
    - keep tests for no target, adjacent target, alarm, and combat routing.

- Talk (`T`).
  - Current town talk reaches NPC envelopes, scoped prompts, reserved words,
    repeated keyword lookup, action dispatch `A` through `K`, and runtime shop
    routing.
  - Remaining work:
    - audit exact sleeping/no-response status-tile mapping when public spec
      issue #44 is answered,
    - audit richer refusal paths for non-NPC targets,
    - keep side effects and shop routing covered as new clean spec details are
      published.
  - Keep decoded `.TLK` content runtime-only; do not commit transcripts.

- Use (`U`).
  - Current behavior covers the public inventory families: torches, gems, keys,
    scrolls, potions, Moonstones, regalia, Shards, magic carpet, skull keys,
    spyglass, HMS plans, sextant, pocket watch, and wooden box.
  - Remaining work:
    - audit exact refusal text and presentation parity,
    - keep tests for inventory mutation, turn rules, and mode gates current,
    - map further edge-case effects only when public clean-room behavior is
      available.

### Movement And Transitions

- World movement.
  - Already supports wrapping, passability, active-object blocking, vehicles,
    wind-driven ship behavior, waterfalls, damage sidecars, encounters, and
    plane transitions.
  - Remaining work:
    - replace sidecar-only transition coordinates where public default tables
      become available,
    - verify every vehicle transport against public passability rules,
    - continue expanding route-smoke scripts across more transition types,
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
    room combat handoff, and text proxy view.
  - Remaining work:
    - complete exact dungeon exit-cell identities when public data is available,
    - replace sidecar-heavy-door rows with public low-nibble rules when safe,
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
  - `world_encounters.tsv` (now overrides matching terrain before native fallback)
  - `shrines.tsv`
  - `dungeon_deeper_transitions.tsv`
  - `dungeon_teleports.tsv`
  - `dungeon_exit_tiles.tsv`
  - `dungeon_doors.tsv`
  - `dungeon_chests.tsv`
  - `secret_doors.tsv`
  - `town_fire_sources.tsv` (now an override for native `0xB4..=0xB7` cannons)
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
  - `common_words.tsv`
  - `end_narrative_windows.tsv`
  - `SAVED.WPS`
  - `SAVED.BTH`

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
    - exact HP/MP recovery and hourly poison/starvation damage formulas where
      still first-playable.

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

- A first visual slice now exists behind `cargo run --features visual --
  --visual ...`. It opens one Bevy window, renders a single CPU-generated
  RGBA framebuffer of the current viewport into one `Image`, and routes
  keyboard input through the same handlers used by the terminal play loop.
  Town/world scenes use the tile-atlas top-down view; dungeon scenes use a
  clean first-person raster with the public light gate, wall/feature cues, and
  active dungeon object overlays.

- Establish a Bevy app shell. (visual slice landed)
  - Game state resource wraps or adapts the existing `PlayState`. (done)
  - Input systems call existing command handlers. (done)
  - Rendering systems consume state without duplicating gameplay rules. (done)
  - Asset loading uses local `C:\Games\U5-Clean` files at runtime. (done)

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
  - A spec-backed fixed-cell text-window core now covers four descriptors,
    cursor preservation, style controls, clear/scroll, wrapped strings,
    numeric output, typed-input erasure, and a shared message/prompt/stats
    screen surface used by TUI and Bevy status/modal summaries. Bevy gameplay
    status now renders the shared surface through `IBM.CH` into a texture.
  - Verify with screenshots or pixel hashes where practical.

- Build a dungeon renderer.
  - Continue replacing the terminal text proxy with the public first-person
    raster where frontends can consume pixels directly.
  - Current raster covers distance bands, side-wall mirroring, door/wall
    variants, darkness gate, field/feature cues, and active object overlays.
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
  - Audit all 48 spells against
    `C:\Projects\Rust\u5-clean\u5-spec\systems\magic.md`.
  - For each spell, classify:
    - already implemented,
    - implemented as first-playable approximation,
    - combat-only,
    - unimplemented but allowed,
    - out-of-scene refusal.

- Known approximations/gaps.
  - Heal now uses the public halved-roll formula with a minimum of 1 HP.
  - Great Heal and Resurrect now apply spec-backed core record mutations,
    including dungeon combat-active refusal and resurrection experience, mana,
    level, and max-HP recomputation.
  - Create Food uses first-playable fixed amount and cap behavior remains
    blocked on `cleak/u5-spec#49`.
  - Rel Hur uses the public `weather.md` prompt-to-wind mapping and is covered
    by cast/resource-order tests.
  - Blink default range is sidecar-authored outside combat; combat Blink uses
    the current arena state and legal in-arena landing checks. Non-combat
    default target selection remains blocked on `cleak/u5-spec#48`.
  - X-Ray and Peer use first-playable map projections; visual parity remains
    open.
  - Dungeon Up/Down spells implement the public one-level movement hook inside
    level bounds; the command-overlay dungeon escape helper remains separate
    and does not currently imply a spell-dispatch gap.
  - Combat-side active-effect consumers are implemented broadly, but parity
    still needs audit coverage.

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

Goal: continue replacing first-playable combat coverage with spec-backed parity
as public details become available.

- Current combat handoff.
  - Hostile world-object contact, Attack against combat-class objects, dungeon
    rooms, rest ambushes, and outdoor encounters can enter a combat frame.
  - Combat-frame entry snapshots the previous mode state, loads clean runtime
    arena data, places actors, and restores the suspended state on exit.
  - Tests cover active-object preservation, trigger-slot reconciliation, actor
    setup, room/ambush routes, and several spell/effect paths.

- Full combat loop.
  - Continue auditing actor initiative/phase parity.
  - Continue auditing player movement and targeting parity.
  - Continue auditing monster AI parity.
  - Continue auditing combat field placement and contact parity.
  - Default monster death/drop markers and vanish-on-death actor clearing are
    implemented in the temporary combat active-object table; continue auditing
    Gazer/Gargoyle special-death presentation, damage, defense, status,
    rewards, loot, and escape parity.
  - Monster combat-AI runner instruction set and class effect map are still
    called out as remaining public-spec parity work.

## Milestone 6: Content Systems

Goal: turn diagnostic interactions into game-like content.

- Shops.
  - Current talk can identify shop triggers and route into runtime shop flows
    with menus, pricing, inventory/gold updates, and clean cancel/refusal paths.
  - Remaining work:
    - audit every service against newly published spec details,
    - keep runtime `.DAT` reads out of committed generated content,
    - preserve focused tests for each shop type and branch.

- Containers and pickups.
  - Current object-table, tile, Search, and dungeon chest paths can grant food,
    gold, keys, gems, torches, scrolls, potions, equipment, Moonstones, magic
    carpets, regalia, shards, HMS Cape plans, and the Sandalwood Box through
    the shared inventory-add path.
  - Remaining work:
    - preserve the public `hidden-treasures.md` table fingerprint/count
      regression when editing fixed Search treasure rows,
    - continue validating authored object-table/item-code coverage with local
      assets without committing raw asset dumps,
    - preserve consumed-object persistence tests as new durable save fields are
      added.

- Doors, locks, and secrets.
  - Current sidecars cover town locks, dungeon doors, and secret doors.
  - Remaining work:
    - exact surface lock-state byte pairs,
    - exact dungeon low-nibble split,
    - cannon/fire durability details.

- NPC schedules and conversations.
  - Current schedules link and move NPCs in town-family scenes, preserve
    cached-waypoint movement state until a transition settles, and route
    floor changes through the `0xC8`/`0xC9` floor-link marker BFS.
  - Conversation sessions cover ASK-PARTY-NAME, ASK-WHO, non-`JOIN`
    recruitment prompts for roster companions, and non-roster name prompts
    without accidental joins.
  - Remaining work:
    - exact audit of every authored schedule/AI edge,
    - conversation side-effect audit beyond the currently known action letters,
    - TLK `0x87` semantics pending public spec clarification
      (cleak/u5-spec#46),
    - TLK `0x85` toll milestone karma pending public toll-progress counter
      clarification (cleak/u5-spec#27),
    - shop/service conversations,
    - NPC memory flags such as thanked/picked/quest state.

- Encounters.
  - Current world encounters can spawn either from clean sidecar rows or from
    the native public threshold/terrain-bucket spawner after eligible consumed
    overworld turns. Sleep ambushes, fortunes-of-war count rerolls, dungeon room
    encounters, and terrain combat setup are implemented.
  - Remaining work:
    - continue auditing authored scripted encounter cells and combat arena
      presentation against public data,
    - keep native weighted spawn-bucket tests aligned with `encounters.md`,
    - preserve the town-hostility boundary: ordinary town attacks stay in
      town/NPC alarm paths unless future public evidence adds a framer caller.

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
  - Quick-start and compatibility notes remain in the README.
  - Sidecar reference is now summarized in `docs/sidecars.md`.
  - Command routing reference is now summarized in `docs/commands.md`.
  - Architecture notes are now summarized in `docs/architecture.md`.

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
  - Initial matrix lives in `docs/status-matrix.md`; refresh it when major
    gameplay or frontend coverage changes.

- Document common dev commands.
  - `cargo fmt -- --check`
  - `cargo test`
  - `cargo run -- C:\Games\U5-Clean`
  - `cargo run -- --play C:\Games\U5-Clean`
  - `cargo run -- --play-script "z;q" C:\Games\U5-Clean`
  - `cargo run -- --save-frame screenshots\britannia.png --scene BRITANNIA C:\Games\U5-Clean`
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

## Completion Criteria For The Active Goal

The active project goal is full-game completion, not just first-playable
movement. Do not mark the goal complete just because tests pass. Run a
completion audit against the criteria below and verify each item with code,
tests, command output, screenshots, or a real scripted/playable session.

- The intro, Journey Onward, character creation, and Ultima IV transfer flows
  are playable through the intended frontend.
- World, town, dungeon, combat, shop, conversation, Blackthorn, Codex, shrine,
  and endgame modes are reachable through normal gameplay paths.
- Every A-Z/Space command routes by mode according to `u5-spec`, with turn
  costs, prompts, refusals, state mutation, and post-turn effects verified.
- Area transitions work across world/town, town/world, world/dungeon,
  dungeon/world, town floors, dungeon levels, Britannia/Underworld plane
  changes, moongates, waterfalls/chasm paths, and Gate Travel.
- Save/load preserves all supported durable state, including sidecars used for
  clean-room semantic state whose exact original save offsets are not public.
- Combat setup, player commands, monster turns, victory/defeat, loot,
  post-combat reconciliation, special arena triggers, and the Doom endgame
  handoff match the public specs.
- Magic, inventory use, equipment, shops, rest/camp, conversation side effects,
  quest flags, Shadowlord/shard progress, shrine/Codex progress, Blackthorn
  story state, and final endgame state have focused runtime tests.
- Runtime asset reads stay clean-room safe: no committed original assets, raw
  dumps, dialogue transcripts, private offsets, decompiled source, or
  disassembly-derived implementation artifacts.
- The TUI/CLI and Bevy frontend build and run from documented commands.
- Screenshots or frame captures verify representative world, town, dungeon,
  combat, intro, and endgame scenes without blank frames or overlapping UI.
- A final completion audit maps each public spec deliverable to concrete engine
  evidence and calls out any remaining public-spec gaps instead of assuming
  parity from proxy signals.
