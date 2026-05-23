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

- `cargo test -p u5-runtime` passed on 2026-05-23, including 2568 tests
  (latest verification includes public `cleak/u5-spec#47` hourly
  poison/provision/ring ordering, public #28 horse-trader adjacent placement
  priority plus no-marker refusal, public #15 inn pickup stay-counter billing,
  combat command-flow regressions for pending Z-stats/Cast actor liveness, public
  `cleak/u5-spec#41` arms-shop scene-row coverage, public issue #3
  terrain-combat replacement-tile main path, and public issue #21 dungeon
  active-monster ambush setup, combat round maintenance, combat-local
  ambush/camp reveal-slot helper coverage, and disk I/O retry wrapper
  coverage).
- `cargo test -p u5-tui` passed on 2026-05-23, including 79 tests.
- `cargo test -p u5-tui --features visual` passed on 2026-05-23.
- `cargo test -p u5-bevy` passed on 2026-05-23, including 56 tests.
- `cargo fmt -- --check` passed on 2026-05-23 after the latest Rust changes.
- `git diff --check` passed on 2026-05-23; the only output was existing
  CRLF-normalization warnings.
- Representative raster smoke checks with local assets produced nonblank hashes:
  `BRITANNIA` top-down `fd923dc0f87a9f3c`, `BRITANNIA` after movement
  `1eb882f27b1d216c`, route-smoke Britannia movement `bef4c9fc1eecf9fb`,
  `CASTLE:0` top-down `be84488b7b199310`, and `DUNGEON:0` first-person
  `161ad48dd2a91725`.
- `cargo run -p u5-tui -- --route-smoke C:\Games\U5-Clean` passed on
  2026-05-23 with 134 scripted route cases (including expanded active-shop/modal
  routes for arms, healer, inn, reagent, tavern, horse trader, shipwright,
  guild, and sage flows, plus four extended-session
  cases: 12-step Britannia exploration with Z-stats and Look, 10-step castle
  walk-and-rest, 9-step dungeon turn-and-search, 5-round Doom combat pass, and
  focused Create Food, fountain Look, Yew wanted-poster Look,
  Horse/non-horse wishing-well branches, death-vision Look, public
  #44 sleeping/praying Talk refusals, public #48 Blink ray landing,
  light-decay, dungeon ladder-chain,
  dungeon-to-world return, hourly provision/poison/starvation/ring passes, public
  #32 Britannia/Doom Word-of-Power seal opening routes, public #15 accepted
  inn-rest pricing, public #13 sage paid-success/short-funds paths, public #31
  native shard/Eternal Flame destruction routes, and public #21
  active-monster attack/contact ambush routes)
  covering world/town look and
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
  Jimmy/Open/refusal command routing, and combat pass, active-player digit
  selection/clear, raw Escape abort (`54a47033c570623a`), Ctrl-S music toggle
  (`74636efa99055e5d`), lowercase direct movement, Attack, Board, Cast, Down,
  Enter, Fire, Get, Hole-up, Ignite, Jimmy, Klimb, Mix, New Order, Open, Push,
  Ready, Talk, Up, View, West, X-it, Yell, Z-stats, refusal, and Search-prompt
  command routing, plus combat Look label-only routing (`25343578fe3b2a4d`),
  world dispatcher refusals (`2936fb4c6ff7e3ef`),
  fixed hidden-treasure zero-key, single-use, daily-cache, and stacked Underworld search
  routes (`669a8d5524328039`, `d2b7835b9994c5d8`, `e7b98faee10c7445`),
  PRV Gate Travel success/refusal routes (`8989fd97ff26da04`,
  `483b27d450a54309`, `d4a86e6efb5c8978`, `b862359d883858c9`),
  saved-slot natural moongate live-entry routes (`8989fd97ff26da04`,
  `0242f19174914479`), public Britannia chasm fall route
  (`3f4fdf2e53e4e269`), public #48 Blink ray landing
  (`f4b691ac224b385e`), public #51 poison-gas doorway step
  (`836b6cd5af06c44e`), and public #47 dungeon no-direct-recovery rest
  (`161ad48dd2a91725`) plus hourly Ring of Regeneration
  (`be84488b7b199310`), plus ship broadside fire
  (`a7f1e8c1d62d7388`), horse boarding (`c346e297d616e667`),
  dungeon torch ignition (`06a7a60a0f84fb96`), and a combined Mix/Ready/New
  Order command workflow (`93018f522ce292ec`),
  town dispatcher refusals (`3126df0494b5870b`), town party overlay routes
  (`26d7ef40b57084af`), and terminal endgame missing-box
  confirmation (`ff6287fbb741bd85`) and Wooden Box victory confirmation
  (`0cf64339ccad08e1`), plus Blackthorn audience correct-password
  (`338605b812db4ced`), wrong-password (`b700db952bc67bbf`), and
  rescue-refuge (`3c4488f67ab70cb5`) paths.
- The TUI binary integration tests include temp-directory startup and save
  smoke for Journey Onward's empty-save return-to-menu path, deterministic
  Create Character followed by `--from-save --play-script`, intro-driven U4
  transfer commit from `PARTY.SAV`, and a confirmed `QY` save/reload
  round trip. These tests mutate only per-test temporary asset directories,
  never `C:\Games\U5-Clean`.
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
- `cargo run -p u5-tui --features visual -- --visual-route-suite
  target\visual-route-suite C:\Games\U5-Clean` passed on 2026-05-23 and
  wrote 81 nonblank Bevy-owned per-step route PNGs plus a sanitized
  manifest: `route-world-movement-00-initial` `f68b906acde0bd4a`,
  `route-world-movement-01-d` `ec7c5878d044dda6`,
  `route-world-movement-02-idle` `949d4d0fb006d273`,
  `route-town-status-modal-00-initial` `2beb3b7734800e11`,
  `route-town-status-modal-01-z` `bee4e11801862ad1`,
  `route-town-view-overlay-00-initial` `2beb3b7734800e11`,
  `route-town-view-overlay-01-v` `3dac257b8d2986d5`,
  `route-town-view-overlay-02-idle` `5d5af54c5d7eb0f0`,
  `route-britannia-look-00-initial` `f68b906acde0bd4a`,
  `route-britannia-look-01-l6` `da5ca5200c222d0f`,
  `route-britannia-spyglass-chunk-map-00-initial` `ee035bc3da0ecedd`,
  `route-britannia-spyglass-chunk-map-01-usp` `4d75505e3140a852`,
  `route-castle-save-refusal-00-initial` `2beb3b7734800e11`,
  `route-castle-save-refusal-01-q` `c58e3249e4d12730`,
  `route-castle-save-refusal-02-n` `6465878cfb486dd1`,
  `route-world-board-horse-00-initial` `dad599d00fa00a6a`,
  `route-world-board-horse-01-b` `402223dd79b07b77`,
  `route-ship-broadside-fire-00-initial` `8b7440c1476c5f31`,
  `route-ship-broadside-fire-01-f6` `edd09405aa4bbd41`,
  `route-dungeon-movement-search-00-initial` `67e7e116d8be67aa`,
  `route-dungeon-movement-search-01-w` `5be54e4dfc5e923f`,
  `route-dungeon-movement-search-02-a` `55d65ca1a5c74e9f`, and
  `route-dungeon-movement-search-03-s6` `fbcbfb63d205e997`,
  `route-dungeon-ignite-torch-00-initial` `29289813c0f0397c`, and
  `route-dungeon-ignite-torch-01-i` `462ca23693fa9e2a`,
  `route-dungeon-exit-refusal-00-initial` `67e7e116d8be67aa`,
  `route-dungeon-exit-refusal-01-q` `50baeabcaa6d8347`,
  `route-dungeon-exit-refusal-02-n` `bbceaa4d74f5a7ea`,
  `route-shop-sage-topic-miss-00-initial` `eafb6cf3478f4c49`,
  `route-shop-sage-topic-miss-01-mantra` `67cfc176459efad8`,
  `route-doom-combat-trigger-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-trigger-01-empty` `a2619c7eb20c407d`,
  `route-doom-combat-pass-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-pass-01-empty` `a2619c7eb20c407d`,
  `route-doom-combat-pass-02-empty` `3cf0bc15e87d80a5`,
  `route-doom-combat-attack-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-attack-01-empty` `a2619c7eb20c407d`,
  `route-doom-combat-attack-02-a6` `3cf0bc15e87d80a5`,
  `route-doom-combat-board-refusal-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-board-refusal-01-empty` `a2619c7eb20c407d`,
  `route-doom-combat-board-refusal-02-b` `3cf0bc15e87d80a5`,
  `route-doom-combat-z-stats-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-z-stats-01-empty` `a2619c7eb20c407d`,
  `route-doom-combat-z-stats-02-z` `3cf0bc15e87d80a5`,
  `route-doom-combat-search-prompt-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-search-prompt-01-empty` `a2619c7eb20c407d`, and
  `route-doom-combat-search-prompt-02-s` `3cf0bc15e87d80a5`. The
  2026-05-23 expansion added public #48 Blink
  `route-britannia-blink-east-ray-01-c1ip6` `17ceb1f94bc6c6e3`,
  public #51 poison gas `route-castle-poison-gas-step-01-d`
  `33ebf44d5ea24373`, public #15 inn rest
  `route-shop-inn-rest-accept-02-y` `2591d02e2c602824`, public #13 sage
  paid/short-funds outcomes `3d1bdb8ea9234398` / `c73a1d5ec1594b41`, and
  public #43 fountain, wishing-well, and death-vision Look endings
  `ce66d6a727da0f4c`, `cb49b5dc01b81ad4`, and `bc9b32161912f71d`. The
  #28 horse-trader expansion adds accepted Horse & Rider, Stablehouse, and
  Wishing Well routes through visual step `*-02-y`.
- Bevy visual screenshot smoke with local assets produced a nonblank
  `792x1182` PNG at `target\codex-fixed-font-status-smoke.png`.
- Bevy intro screenshot smoke with local assets produced a nonblank
  `1240x930` PNG at `target\codex-intro-title-tick-smoke.png`, capturing an
  early `BRITISH.PTH` signature-animation frame plus the title-tick overlay.
- Bevy unit framebuffer coverage includes world, town, dungeon, combat, intro
  title art, signature path rendering, title-tick strip bounds, finished-menu
  title-surface overlay, spec-ordered intro story transition/secondary art
  draws, status, endgame modal surfaces, and the deterministic visual
  frame-suite compositor. Visual route-suite tests cover per-step route case
  definitions, nonblank local-clean route PNG output, sanitized manifests, and
  unchanged-frame rejection for scripted transitions.
- `--save-frame <PATH>` is the current headless PNG capture path for local
  asset visual checks.
- `cargo run -- --help` is a supported no-asset usage path.
- The latest checkpointed engine commit at the time of this refresh included
  container Search object-table and trap-narration parity work.
- Clean spec refresh on 2026-05-23 still found `cleak/u5-spec` at `e34af6b`
  with no newer public push visible from this workspace. Issue answers from the
  public tracker remain the clean source for behavior where checked-in spec
  prose lags.
- The spec checkout used for the most recent audit was `5b816cc Complete
  cleanroom specification`.

Current worktree context when this TODO was refreshed:

- `git status --short` in `u5-engine` already contained broad local edits from
  the ongoing implementation/audit pass; preserve unrelated user changes.
- `git status --short` was clean in `u5-spec` at `e34af6b`.
- `journal/capture/notes.py` was not present in the workspace, engine, or spec
  repository.
- Town-family exit thresholds now prompt both when stepped onto and when
  observed underfoot after a consumed turn; accepting exits through clean
  return metadata, refusing leaves town mode active.
- Natural moongate live-tile refresh now keeps mode-zero scene/light cleanup
  from advancing the shared gate-presence counter. The cached Trammel/Felucca
  glyph bytes now refresh from the public hour-indexed tables on construction,
  hour changes, and status redraw, and live-gate entry decodes only the cached
  byte instead of recomputing the table at entry time. The engine now follows
  public `cleak/u5-spec#38` for Felucca hours 10/11/19/20: they are high-bit
  off-horizon sentinels, so natural-gate entry does not route them through
  Moonstone slot 0.
- Bevy title-tick rendering now follows public issue #52 at cleanroom
  replacement depth: a four-frame palette-cycled procedural flame stripe using
  the published EGA bright/dim color pairs. Exact historical driver-resident
  silhouette pixels remain out of scope.
- Inn rest, leave, and pickup charges now follow the latest public
  `cleak/u5-spec#15` guidance: Intelligence-adjusted arithmetic using the
  speaking member's Intelligence. Pickup adjusts `base_rate * 10` before
  multiplying by the billable stay counter. Paid inn rest advances eight hours,
  wakes sleepers, cures poison, and applies class-based night restoration: full
  HP/MP targets for Avatar/Mage-style classes and half targets for Bards.
- U4 transfer source validation now follows public `cleak/u5-spec#16`:
  fixed 532-byte `PARTY.SAV`, public offsets for move/moon/dungeon counters,
  gold/food/keys/torches/gems/sextants counters, leading class/name, and the
  eight-byte no-transferable-data virtue gate.
- Rest/camp ordinary HP/MP recovery follows the latest public guidance in
  `cleak/u5-spec#47`: rest advances time without a separate direct recovery
  grant. Hourly Ring of Regeneration is time-owned, non-combat only, checks the
  ring equipment slot, heals living wearers by exactly 1 HP on the 1-in-8 roll,
  and clamps at max HP. Exact original random-jolt/camp recovery details remain
  unresolved.
- Non-combat Blink follows the latest public `cleak/u5-spec#48` guidance: it
  prompts for a cardinal direction and lands on the farthest legal grass cell
  along the bounded ray. Route-smoke and Bevy visual-route coverage exercise an
  eastward Britannia ray.
- Create Food follows the latest public `cleak/u5-spec#49` guidance with a
  tiny `0..=2` food PRNG grant capped at the party food cap.
- TLK `0x85` accepted toll payments debit gold, increment the toll-progress
  counter, and apply the published milestone karma behavior from
  `cleak/u5-spec#27`.
- Hourly poison and starvation follow the latest public `cleak/u5-spec#50`
  guidance: poison is fixed `-1 HP` per poisoned living member, and starvation
  rolls `1..=8` independently for each non-dead slot.
- Town poison-gas doorway cells use the latest public `cleak/u5-spec#51`
  predicate when clean tile attributes are available (`tile_class == 4` and
  `vehicle_byte == 0x1C`), with a `0..=29` per-non-poisoned-slot roll compared
  against each member's Dexterity after committed movement steps and before
  turn-clock advancement. Coordinate sidecar rows remain as fallback until the
  full resident tile-attribute table is published.
- Talk-triggered arms shops use the public `cleak/u5-spec#41` scene-to-row
  identity table and exact per-row `a..h` stock arrays; visible buy choices stop
  at the `0xFF` terminator.
- Public `cleak/u5-spec#28` corrected the old stationary-display purchase path
  to horse-trader sale rows. The obsolete stationary-display purchase runtime is
  removed, and horse-trader runtime/talk-shop/route-smoke/visual tests cover
  Intelligence-adjusted quotes, local marker placement, no-marker refusal, and
  accepted purchases for all three public stables.
- Shadowlord shard U-Use follows public `cleak/u5-spec#31` exact native
  positions and requires the matching live Shadowlord/name encounter north of
  the party; route smoke covers Lycaeum, Empath Abbey, and Serpent's Hold native
  paths.
- The 2026-05-23 clean-engine audit found no broad newly answered engine slice
  left unimplemented in the pulled spec. Follow-up questions remain current for
  response-needed public blockers #1, #3, #13, #18, #31, #41, #43, #47, #51,
  and #54.
- Shop session regression tests now lock the corrected public scene-byte rows
  for taverns, shipwrights, reagent vendors, guildmasters, inns, healers, and
  arms-shop identities, including old wrong-scene negative cases from the
  public issue corrections.
- Public `cleak/u5-spec#43` Look specials now cover top-down fountain drink
  prompts with presentation-only refresh, wishing-well coin and 12-character
  wish input with scene gates, structured accepted keyword matching, a native
  Horse grant, accepted car keywords mapping to the horse-family grant in
  public scenes, death-vision active-object dispatch with member selection, and
  Yew wanted-poster route/visual evidence with clean-authored placeholder text.
  Exact wanted-poster resident text and line breaking remain pending public
  clarification.
- Return-to-View now expands the MISCMAPS command stream into a per-title-tick
  playback timeline for preview ticks, cell-effect timing, fixed-wipe
  rectangles, eight-title-tick waits, trailing ticks, and one-shot actor draws.
  The loader transposes the 19x4 on-disk strip source into the public 4x19
  visible preview, derives captions from LoadMapStrip, and applies the
  `(x, y + 7)` local cell-effect coordinate rule. Exact effect rasters remain
  presentation work.
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
   - Stock town/dungeon entry and return coordinates are published in the
     gazetteer and implemented as native defaults.
   - The current clean spec still omits several exact moongate, plane-transition,
     and special-transition coordinates.
   - Leave safe missing-metadata behavior in place for coordinate families that
     are not yet published through the spec.

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
    supports Shadowlord names and public issue #32 Word-of-Power seal opening:
    matching words at their published coordinates flip the closed seal tile
    with `^ 0xDF` and dirty visibility.
  - Remaining work:
    - audit any newly specified mode-specific Yell presentation effects,
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
    routing. Route-smoke covers the public #44 sleeping/praying status-tile
    refusals before shop/dialog dispatch.
  - Remaining work:
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
  - `common_words.tsv` (now an override for the public issue #33/#40 built-in
    common-word dictionary)
  - `end_narrative_windows.tsv` (now an override for public END.DAT
    final-narrative byte windows)
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
  - Save/load, chargen, and U4 transfer save-pair reads/writes now route
    through the shared `disk_io` wrapper; tests cover zero-byte retry,
    nonzero short-read/short-write success, write-handler phase restoration,
    and fast failure in the modern single-directory path.
  - Original binary content/resource loaders for CBT, END/ENDMSG, MISCMAPS,
    MISCMSG, QUESTION, SIGNS, STORY, SHOPPE, fonts, PTH, and KARMAS now use
    the same disk I/O wrapper; optional clean TSV sidecars remain direct
    filesystem reads.
  - Add regression tests whenever a new save field is written.
  - Prefer patching known fields over reserializing an invented save image.

- Active objects.
  - Existing work preserves embedded active-object rows and relinks scheduled
    NPCs. Transition-save regressions now cover town floor changes, dungeon
    level teleports with a live working buffer, dungeon exits back to an
    overworld plane, both world overlay halves after plane transitions, and
    parked vehicle mutations after board/exit/fire flows.
  - Remaining work:
    - audit all active-object mutations for save inclusion,
    - continue broadening save/load transition coverage when new public
      transition tables are promoted out of sidecars.

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
  - Current tests cover ship/skiff/carpet/horse transport markers, hull/skiff
    side bytes for boarded and parked ships, board/exit/fire active-object
    overlay save/load, skiff/carpet ship-exit fallback save/load, and
    save/load after town and dungeon vehicle-adjacent return-world transitions.
  - Remaining work:
    - continue auditing exact ship facing/sail marker variants against any new
      public marker evidence,
    - continue auditing hull/skiff persistence across shop delivery and exotic
      transition paths,
    - all timing/status tags beyond currently recognized `Q` and `T`.

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
  - Respect line-of-sight, light radius, and public issue #42 local-light
    source masks: radius-three Chebyshev sources, source-to-target blocker
    carving, active-object flame sources, and multiple-source union.
  - Top-down radius-5 raster rendering now drives the public `visibility.md`
    persistent scratch model: an 11-active-cell, 32-byte-stride visibility
    grid plus 16-byte-stride terrain companion band, full rebuild on dirty
    frames, lazy refill on clean frames, fog marker refinement, active-object
    companion stamps, and scratch-byte preservation.
  - Show status/message panels.
  - A spec-backed fixed-cell text-window core now covers four descriptors,
    cursor preservation, style controls, clear/scroll, wrapped strings,
    numeric output, typed-input erasure, and a shared message/prompt/stats
    screen surface used by TUI and Bevy status/modal summaries. Bevy gameplay
    status now renders the shared surface through `IBM.CH` into a texture.
  - Verify with screenshots or pixel hashes where practical.
  - `--visual-route-suite <DIR>` replays representative world, town modal,
    town View-overlay, and dungeon movement/search routes through the Bevy
    full-frame compositor, writes per-step PNGs plus a sanitized manifest, and
    fails if a scripted route step leaves the frame unchanged.

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
  - wider story/endgame rectangle transition helper rates beyond the
    published step-1 one-column-per-title-tick wipe,
  - Return-to-View strip geometry and exact effect rasters (the public #54
    scheduler timing and fixed captions are now modeled in runtime state),
  - broader `EGA.DRV` behavior beyond the canonical EGA/Tandy-equivalent path,
  - exact historical title-tick silhouette pixels,
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
  - Heal now uses the public shared-PRNG `0..=60` roll, halved formula,
    minimum of 1 HP, Dead-only refusal, and max-HP clamp.
  - Great Heal and Resurrect now apply spec-backed core record mutations,
    including dungeon combat-active refusal and resurrection experience, mana,
    level, and max-HP recomputation.
  - Create Food uses the latest public `cleak/u5-spec#49` tiny PRNG grant
    (`0..=2`) and cap behavior.
  - Rel Hur uses the public `weather.md` prompt-to-wind mapping and is covered
    by cast/resource-order tests.
  - Non-combat Blink uses the public `cleak/u5-spec#48` cardinal direction
    prompt and farthest-grass ray landing rule; combat Blink uses the current
    arena state and legal in-arena landing checks.
  - View, Peer, and X-Ray overlays now carry explicit runtime modes, and the
    surface/dungeon overlay rasters apply the public peer/gem alternate
    bank/tint branch for affected cell classes. Exact remote-view panel pixels
    remain presentation parity work.
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
  - Dungeon-room combat now uses the public issue #12/#19 source rules for
    high-bit-masked ordinary monsters, compact source-order placement, skipped
    low special markers except Doom's absorbable field, and party positions
    after the placed ordinary monsters.
  - Dungeon active-monster combat now follows public issue #21: it uses the
    ambush framer path, builds a stock-floor 11-by-11 arena without loading
    `DUNGEON.CBT`, and creates exactly one initial monster at the central-front
    placement.
- Dungeon `0xF?` cells now follow the latest public issue #1 correction as
    room triggers; dungeon Open and Jimmy no longer mutate `0xF?` trigger cells
    or `0xE?` visual wall cells, stale `dungeon_doors.tsv` files are ignored,
    and dungeon Open now uses the public underfoot `0x7?` "Chest opened" /
    default "What?" messages.
  - Terrain combat now uses the public issue #3 combat-class stat-field spawn
    count plus `BRIT.CBT` record placement metadata, clamps requested counts
    above the slot table instead of reproducing the original out-of-bounds edge,
    places the party after the monster slots, and applies the public
    one-in-nine early-spawn replacement-tile roll through the main
    terrain-combat path. Exact visual review of every replacement byte remains
    combat parity/audit work.
  - Combat-frame entry snapshots the previous mode state, loads clean runtime
    arena data, places actors, and restores the suspended state on exit.
  - Tests cover active-object preservation, trigger-slot reconciliation, actor
    setup, room/ambush routes, and several spell/effect paths.

- Full combat loop.
  - Continue auditing actor initiative/phase parity.
  - Continue auditing player movement and targeting parity.
  - Continue auditing monster AI parity.
  - Combat descriptor byte-2 flags now use the public issue #6/#7 controlled
    and flee bits. Charmed/possessed and summoned non-party actors route through
    the player-command path, Conjure uses fresh random arena-coordinate attempts,
    Swarm uses the caster ring, and player Summon uses fixed north-clockwise
    ring order. Issue #8 non-party sleep now has the published per-slot
    countdown/targetability behavior, but exact per-effect starting durations
    and descriptor-byte table wording still need a public spec clarification
    before claiming exact monster sleep wakeup parity.
  - Combat field placement now separates marker materialization from post-step
    contact, uses Poison's unconditional placement path, and gates Fire/Sleep/
    Energy with the public issue #10 one-in-eight clean-engine default.
  - Combat-local ambush/camp reveal records now follow the public helper shape:
    up to eight trigger coordinates, consume-on-fire, one or two in-range
    terrain stamps, out-of-range target sentinels, ordinary-combat clearing, and
    post-committed-movement dispatch for player and AI movement paths.
  - Combat round-counter wrap now applies the one-minute combat-safe clock
    advance, and post-round maintenance sweeps terrain/effect dispatch bytes,
    the magic-effect timer, and transient cursor/secondary-marker visuals
    without aging field active objects.
  - Combat Vanish, Magic Lock, Unlock Magic, and Open use the public issue
    #37/#39 utility fallback: no target prompt, no arena mutation, resources
    consumed after gates, turn advanced, and `Failed!` reported.
  - Default monster death/drop markers, party corpses, vanish-on-death actor
    clearing, Gazer eye-burst, and Gargoyle lava-then-default-death transitions
    are implemented in the temporary combat active-object table; continue
    auditing damage, defense, status, rewards, loot, and escape parity.
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

A first pass of that audit is published in `docs/completion-audit.md`. It maps
every public-spec system, format, and catalog to engine evidence and
test/route-smoke/frame-suite coverage, separates gameplay gaps from visual
polish, and enumerates the outstanding `cleak/u5-spec` issues that gate
exact-parity claims. Refresh that document whenever behavior moves between
safe-placeholder and spec-backed implementation.

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
