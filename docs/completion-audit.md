# Completion Audit

This audit maps each public-spec deliverable (the systems and formats published
in `C:\Projects\Rust\u5-clean\u5-spec`) to concrete engine evidence
(`crates/u5-runtime`, `crates/u5-tui`, `crates/u5-bevy`) and to test coverage.

Created on 2026-05-19; last refreshed on 2026-08-23 at `9e437d5`, after the
audit-and-repair pass that implemented `#72` and `#73`, retracted three
invented or misattributed models, and wired two published mechanisms that had
been fully modelled and never called.

**Read spec contracts from GitHub, not from the local checkout** — the issues,
and document text through
`gh api -H "Accept: application/vnd.github.raw" repos/cleak/u5-spec/contents/<path>`.
`C:\Projects\Rust\u5-clean\u5-spec` is read-only from this workspace and is
stale at `9a898d1`, many commits and several retractions behind spec head
(notably `#42` local light, `#53` reveal transitions, `#54` Return-to-View,
`#65`/`#66`/`#67` title sequence, `#69` doorway text, `#70` font metrics,
`#80` floor pages, `#82` endgame/chargen, and `#85`-`#88` on moongate transit,
combat `§7`, animation provenance and the U4 seed pair). The spec queue is
empty — **88 closed, 0 open** — so an audit checked against the local files
would disagree with this document.

This document satisfies the completion criterion in `TODO.md`:

> A final completion audit maps each public spec deliverable to concrete engine
> evidence and calls out any remaining public-spec gaps instead of assuming
> parity from proxy signals.

## How To Use This File

- Each table row maps a public-spec section to engine code and test coverage.
- "Evidence" cites engine files (and sometimes line ranges) that implement the
  named behavior.
- "Tests" identifies the inline test families that exercise the behavior. Most
  tests live in `crates/u5-runtime/src/tests_inline/chunk_*.rs`.
- "Status" is one of:
  - **Implemented** — engine fully covers the public spec.
  - **Implemented (public-depth)** — engine covers what the public spec
    publishes; remaining detail is a documented v1 deferral, presentation
    deferral, or local compatibility policy.
  - **Blocked on `cleak/u5-spec#NN`** — the engine has a safe placeholder while
    a public-spec clarification is pending.
  - **Presentation work** — gameplay behavior is implemented; remaining work is
    visual/audio polish covered by `Milestone 3` in `TODO.md`.

The public issue catalogue in scope of this audit is the answered-or-blocking
set already tracked in `TODO.md` and the latest GitHub issue sweep:

| Issue | Topic | Engine status |
|------|-------|---------------|
| `cleak/u5-spec#1` | Dungeon `0xF?` room-trigger and `0xA?` room-helper reload behavior | Implemented from latest issue answer and reconciled checked-in spec wording |
| `cleak/u5-spec#3` | Terrain-combat arena records, spawn counts, replacement tiles, placement metadata, and pirate selector | Implemented from latest issue answer, including active-object byte-0 `0x2C..0x2F` pirate arena selector |
| `cleak/u5-spec#5` | Combat party entry placement and descriptor seeding | Implemented for terrain and dungeon-room combat; party descriptor byte 3 links to the party slot while byte 1 keeps class-derived speed |
| `cleak/u5-spec#8` | Combat non-party sleep/disabled state storage and wake timing | Implemented from latest issue answer: descriptor byte 2 carries the sleep/disabled bit, byte 4 remains the active-object link, Sleep/Sleep Field seed no duration counter, and the actor's own dispatch rolls `0..16` with only `16` clearing the bit |
| `cleak/u5-spec#9`/`#22` | Directed Sleep/Wind combat cone targeting | Implemented from latest issue answer; cardinal direction cone targeting replaces target-slot targeting |
| `cleak/u5-spec#10` | Combat arena field marker placement gate | Implemented from latest issue answer; Fire/Poison/Sleep/Energy markers place after confirmed impact without a random materialization gate |
| `cleak/u5-spec#12`/`#19` | Dungeon-room combat party/source placement | Implemented from latest issue answer: published row/column layout, helper scan suppression, ordinary boundary, special id categories, `0xEC..0xEF` pre-rolled palette selectors, non-Doom auxiliary-byte post-write formulas/tables, and Doom marker behavior |
| `cleak/u5-spec#13` | Tavern/meal/sage selector table plus paid shared 26-row sage rumour topic table and success templates | Table/mechanics implemented, including per-tavern selector letters, lore continuation gating, SHOPPE.DAT fee quote/short-funds records, post-debit success-record RNG timing, and success rendering |
| `cleak/u5-spec#15` | Inn Intelligence-adjusted room-rate formula and recovery behavior | Implemented |
| `cleak/u5-spec#18` | Fixed hidden-treasure found bitmap and special record cookies | Implemented from latest issue answer and reconciled checked-in spec wording |
| `cleak/u5-spec#20` | Location and dungeon return coordinates | Main 40-row published coordinate table implemented; remaining shrine/Codex, moongate, plane-transition, Hythloth, and underworld coordinate families are owned by their specific systems |
| `cleak/u5-spec#28` | Horse-trader sale path replacing old stationary-display purchase premise | Implemented |
| `cleak/u5-spec#31` | Eternal-Flame-gated Shadowlord shard destruction predicates | Implemented, including hideout slots and low-byte quest-progress bits |
| `cleak/u5-spec#41` | Exact arms-shop eight-item stock rows and buy transaction quote selector/text flow | Implemented |
| `cleak/u5-spec#43` | Top-down fountain, wishing-well, death-vision, and wanted-poster outcomes | Implemented |
| `cleak/u5-spec#47` | Hourly Ring of Regeneration tick and completed long-camp recovery | Implemented from latest issue answer and reconciled checked-in spec wording |
| `cleak/u5-spec#48` | Non-combat Blink directional ray landing rule | Implemented |
| `cleak/u5-spec#51` | Native tile `0x04` town poison-gas detection | Implemented |
| `cleak/u5-spec#53` | Rectangle-dissolve primitive for the story step-1 reveal, the STARTSC menu-loader reveal, and the endgame's full-screen rectangle | Resolved, and both of the earlier answers it retracted are now corrected in the engine. The one-column-per-title-tick sweep is withdrawn in full: all three callers issue the shared rectangle dissolve (`crates/u5-runtime/src/dissolve.rs`) as one blocking call visiting every pixel once in a deterministic pseudo-random order. The late endgame rectangle is **not** a no-op before certificate setup: it is `endgame.md §7.1`'s fade to black, run once leaving the throne tableau and before the *first* `END.DAT` window, and the ordinary narrative windows carry no page-in rectangle of their own |
| `cleak/u5-spec#54` | Return-to-View strip captions, timing, geometry, and exact effect rasters | Public timing/captions and 4x19 visible geometry implemented; exact effect rasters are explicitly deferred by the clean spec |
| `cleak/u5-spec#56` | Endgame tableau active-object layout, sprite mapping, and movement timing | Implemented from latest issue answer; MISCMAPS record 3, active-object slots, class sprites, scene marker, branch movement, and `0x44`-gated refusal jitter follow the published contract |
| `cleak/u5-spec#57` | `.NPC` slot-zero sentinel byte policy | Runtime scheduling skips slot zero regardless of stored bytes; validators do not reject a nonzero slot-zero type/tag byte |
| `cleak/u5-spec#58` | Conversation reserved rebuke keyword table | Implemented from latest issue answer: all 34 reserved entries are active, including the 29 rebuke words, space-boundary matching, fixed rebuke text, pause limit, and return-to-prompt behavior |
| `cleak/u5-spec#59` | Overworld fall/transition and damage rules | Implemented: fixed chasm/falls preserves transport, applies Dex-gated one-HP checks, and ignores retired `world_waterfalls.tsv` current-sweep rows at runtime |
| `cleak/u5-spec#60` | Look/View overlay pixel renderer tables and dungeon minimap exact glyph/flood presentation | Gameplay-depth View and minimap behavior is implemented from `systems/view.md`; exact per-class 4x4 glyph pixels, source-bank/tint choices, chunk-map pixels, and dungeon minimap renderer details remain clean-spec questions |
| `cleak/u5-spec#61` | Town free-roaming active-object walker exact rules | Implemented from the latest answer: byte/floor eligibility, 50% gate, four-neighbor `0xA2`/`0x43` blocker gate, query-`0x10` destination classifier, occupancy checks, X-facing writes, Y-facing preservation, and visibility dirties on success |
| `cleak/u5-spec#62` | Live shop-dialogue record selection and window pacing | Implemented where published: shared `SHOPPE.DAT` selection timing, Talk-to-shop inherited window-2 handoff, prompt window separation, and inn Pickup window-1 register geometry; a fresh follow-up asks for the live per-state transcript/wait/clear table needed to replace modal summaries frame-accurately |
| `cleak/u5-spec#72` | Acknowledgements screen asset and presentation | Implemented (`6db6135`). `§11.1` settles the asset as three `STARTSC` records with every credit line drawn into the bitmap; `§11.2` settles the presentation as compose / rise / part / keypress / close / sink, and **withdraws in full** the "bottom-up entry wipe, top-down exit wipe with horizontal slabs" model this engine carried. Nothing typesets the credits, so there is no text to author |
| `cleak/u5-spec#73` | Ultima IV transfer preview screen | Implemented (`f3ecfd1`). `§6.1`-`§6.6` publish the window rectangles, prompt-frame cells, both panel geometries, the eight-row field-label column, the pages, the stage machine and the finish. `§6` has no double buffering and no page swap, so the screen is one persistent surface edited in place. `§6.4`'s insert-disk block is dead code in the shipped build and is not drawn |
| `cleak/u5-spec#84` | Dungeon billboard slot-to-role mapping | Implemented; the corridor draws from its billboard banks and the banks moved onto the atlas. The self-check is scoped to `DNG1` because the mapping is not bank-invariant |
| `cleak/u5-spec#85` | Moongate transit animation distinct from the static gate tile | Answered. Gate *presence* is the `§9.1` sixteen-step composed-frame model and is implemented; the `§9.2` blocking transit presentation is **not** (see "Published but not implemented") |
| `cleak/u5-spec#86` | Retract `combat.md §7`'s post-round maintenance pass | Retracted upstream and removed here (`60ec07c`) |
| `cleak/u5-spec#87` | Clean-room: `animation.md` provenance cited private paths | Resolved on the spec side. No engine change; recorded because a correction pass introduced the breach, which is the failure mode `docs/review-heuristics.md` warns about |
| `cleak/u5-spec#88` | `PARTY.SAV` field layout and the U5 seed filenames | Implemented (`ee0dc53`). The seed pair is `INIT.GAM`/`INIT.OOL`; `BRIT.GAM` is withdrawn and **does not ship at all**, so the old constant named a file that is never present |

Where an audit row references a pending issue, the engine carries a clean
implementation or conservative placeholder that avoids private-derived guesses
until the public spec publishes the missing rule. No such row remains: the
queue is empty.

## Verification Baseline

Re-measured on 2026-08-23 at `9e437d5`. Asset-backed runs used a **scratch copy**
of the asset directory, never `C:\Games\U5-Clean` itself: the install is a
read-only clean-room input, the suite paths take a directory they both read and
write, and it has been corrupted that way before. The engine now refuses a write
destination that resolves to `DEFAULT_GAME_DIR`, and `copy_asset_writable`
clears the read-only bit Windows `fs::copy` propagates into scratch copies.

| Command | Result |
|---|---|
| `cargo test -p u5-runtime --lib` | 2902 pass |
| `cargo test -p u5-bevy` | 170 pass |
| `cargo test -p u5-tui --features visual` | 96 pass (11 + 51 + 34) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets` | **zero errors**; style warnings remain and are not gated |
| `--route-smoke <asset-copy>` | all **493** scripted cases pass |
| `--visual-frame-suite <asset-copy>` | **193** PNGs plus a sanitized manifest |
| `--visual-route-suite <asset-copy>` | **1814** PNGs plus a sanitized manifest |

The route-smoke corpus spans world, town, dungeon, combat, endgame and shop
play: all 40 published stock world-location entry rows, TLK-backed reserved-word
and no-match conversation routes across all 32 named-location scenes,
save/reload checkpoints for transport and transition continuity, the full
spell/combat command matrix, all nine public arms-shop stock rows, the nine
tavern lore selectors, the four shipwright delivery rows, the three native shard
vanquish routes, the Word-of-Power seals, and the terminal endgame through the
full victory cinematic. Its validator requires `cinematic_is_finished()`, so an
ending that stops short fails the case.

The visual route suite's 1814 frames are all nonblank except exactly one, which
is black by contract: the `endgame.md §7.1` fade between the throne tableau and
the first `END.DAT` window. The suite also rejects any scripted step that leaves
the frame unchanged, outside the explicit terminal-endgame and Doom
sustained-pass hold frames.

## Systems

### `systems/combat.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1 Overview | `combat_frame.rs` (`CombatExitOutcome`) | `combat_driver` driver tests | Implemented |
| §2 Combat triggers | `combat_setup.rs` (`TerrainCombatSetup`, public #21 dungeon active-monster ambush, dungeon-room triggers) | `tests_inline/chunk_23.rs` arena entry and active-monster ambush floor test | Implemented |
| §3 Arena (11×11) | `combat_arena.rs`, `COMBAT_ARENA_SIDE` | arena CBT decode tests | Implemented |
| §4 Enter/exit framing | `combat_frame.rs::CombatFrameSnapshot`, restore logic | snapshot save/restore tests | Implemented |
| §5 Monster placement | `combat_setup.rs` placement tables A/B; `combat_frame.rs` combat-local ambush/camp reveal-slot records, target stamping, consumption, and ordinary-combat clearing | placement coordinate tests; ambush-reveal helper and post-step integration tests in `chunk_23.rs` | Implemented |
| §6 Actor table | `combat_actor.rs` (32 slots, 6 party) | actor-slot tests | Implemented |
| §7 Per-round structure | `combat_driver.rs` round-walk classifier; `combat_frame.rs` combat-only cursor-blink tick | `chunk_23.rs` round-cycle and cursor-blink tests | Implemented. **§7's "post-round maintenance pass" was an invented contract and is retracted.** We had built a row-major sweep classifying each arena cell's terrain byte and dispatching effects, plus a magic-effect timer tick. It was removed in `60ec07c` after being shown inert in our own tree: the report it built was discarded by both call sites and `combat_magic_effect_timer` was write-only. Route-smoke's 493 asset-backed cases pass unchanged without it. What is real is a combat-only cursor highlight (blink toggle, active-actor box, optional secondary marker), which has live renderer consumers. |
| §8 Player commands | `combat_scenario.rs` (`CombatScenarioInput`) | command-route smoke (route-smoke combat-*) | Implemented |
| §9 Monster AI | `combat_frame.rs::combat_ai_actor_fleeing`, `combat_target_group_for_slot`, suppression bypass; `combat_actor.rs::party_name_forces_monster_combat_group`, `first_monster_ability`; `combat_stats.rs` class trait rows | `combat_ai`, `combat_actor_slot_dispatch`, `cause_fear` filters, exhaustive combat stat/ranged/ability-hook row tests in chunk 23 | Implemented |
| §10 Spells in combat | `magic.rs` (scene-mask `SPELL_SCENE_COMBAT`), `combat_frame.rs` directed-spell dispatch | `directed_spell_status`, spell-route tests in `chunk_23.rs` | Implemented |
| §11 Attack resolution | `combat_frame.rs::resolve_combat_weapon_attack`, `combat_actor.rs::combat_to_hit_score` | weapon-attack tests, ranged tests | Implemented |
| §12 Damage & status | `combat_frame.rs::apply_combat_weapon_damage_to_target`, `apply_combat_monster_death_active_object_effect`, vanish/death-marker logic | `combat_monster_default_death_materializes`, vanish tests | Implemented |
| §13 Per-class data | `combat_stats.rs::combat_class_stats`, `combat_ranged_effect_stats` | combat-class lookup tests | Implemented |
| §14 Victory/defeat/escape | `combat_frame.rs::CombatExitOutcome`, framer restore path | exit/restore tests | Implemented |
| §15 Hooks | `play_state_impl/chunk_04.rs` post-combat reconciliation; potion/sleep handoff | post-combat integration tests | Implemented |
| §16 Class-flag policy | `combat_stats.rs` (no invented flags), `magic.rs` scene allow-mask | scene-gate tests | Implemented |
| §17 Sources | N/A | N/A | — |

Notes:

- Gazer eye-burst tile and Gargoyle lava-pool special-death tile transitions
  in section 12 use the published marker bytes and now have unit, route-smoke,
  and visual-route evidence alongside the vanish-on-death marker path.
- The Gremlin `cast_like_branch` flag in `monster-bestiary.md` §3 is loaded by
  `combat_stats.rs` but §6 of the bestiary documents that "no additional
  Gremlin-specific resource theft or nuisance writer is promoted." The engine
  honors the flag in target/route classification and consumes the action
  through `CombatWeaponAttackResolution::NoOrdinaryDamage`. **Implemented at
  the published level.**

### `systems/magic.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1 Overview | `magic.rs::SPELL_*`, parser tables | metadata tests | Implemented |
| §2 Reagents | `magic.rs::REAGENT_*`, `play_state_struct.rs` counters | reagent stock tests | Implemented |
| §3 Rune vocabulary | `magic.rs::RUNE_*`, spell-name parser | spell-name parse tests | Implemented |
| §4 Eight circles, 48 spells | `magic.rs::spell_circle_for`, `spell_mana_cost`, parser table | spell-cost tests | Implemented |
| §5 C-Cast | `play_state_impl/chunk_04.rs` cast handler, `magic.rs::cast_dispatcher_gate` | cast scene-gate tests in `chunk_17.rs`, `chunk_18.rs` | Implemented. The live `cast_spell_resource_gate` now routes through `cast_dispatcher_gate` rather than duplicating it |
| §6 M-Mix | `play_state_impl/chunk_04.rs` mix handler; `magic.rs::SPELL_SELECTOR_IGNORED_LETTERS` (J/O) | mix recipe tests | Implemented |
| §7 Prerequisites | `magic.rs::cast_dispatcher_gate` (`CastGateOutcome`), `SPELL_SCENE_BIT_*`, `SpellSceneClass`, `spell_scene_class_for_scene_byte` | gate tests; all 48 mask rows checked against the published `Allowed` column | Implemented, and **newly enforced** (`2e76b82`). The four-bit scene allow mask is applied ahead of charge consumption with `Not here!`. The crate previously modelled the contract twice and disagreed with itself: `constants.rs` carried the transposed legend `catalogs/spell-list.md §4` withdraws, and `cast_dispatcher_gate` had no production caller at all while the live gate tested charges, mana and level but never the scene. Two live defects fell out: Blink was exempt from the central gate (published `C/O`), and X-Ray used an area-only check blind to `combat_active`. The level gate is level-vs-circle by construction now that the circle is re-derived from the spell id |
| §8 Spell effects | per-spell handlers in `play_state_impl/chunk_*.rs`; field placement in `magic.rs::spell_field_placement_byte` | field-cast and restoration/status spell PRNG tests | Implemented (Heal uses the public shared-PRNG roll path; Create Food uses the latest public tiny PRNG grant; non-combat Blink follows public #48 directional ray-to-farthest-grass behavior) |
| §9 Casting in combat | `combat_frame.rs` cast dispatch; enforced scene allow-mask; runtime tag `T` Negate Time gate in the automatic actor driver | combat-cast tests in `chunk_23.rs` | Implemented. Under Negate Time the automatic actor driver returns immediately, so every self-acting actor's turn is skipped while the party is still prompted normally; we previously had no combat gate at all. The gate sits past the `PlayerReady` arm, so the party's own dispatch is untouched |
| §10 Virtue/shrine linkage | `shrine_virtue.rs` stat reward, ordained/Codex masks, Codex urn read state, and all-virtues-complete predicate | shrine-meditation, Codex urn, and Codex turn-in tests | Implemented |
| §11 Z-stats integration | `z_stats.rs`, `stats_panel.rs` | z-stats render tests | Implemented |
| §12 Persistence | `save_load.rs` spell-charge and reagent stock | save/load tests | Implemented |
| §13 Boundaries | `magic.rs` 48-spell dispatch (no per-class adjustments) | parser tests | Implemented |

### `systems/inventory.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1 Scope | `equipment.rs`, `containers.rs::InventoryAddClass` | inventory-add tests | Implemented |
| §2 Shared stores | `equipment.rs::EQUIPMENT_COUNT`, `character_record.rs` slots | slot-layout tests | Implemented |
| §3 Character slots | `character_record.rs` six readied-slot bytes | ready-eligibility tests | Implemented |
| §4 Z-Stats browsing | `z_stats.rs` inventory pages | z-stats tests | Implemented |
| §5 R-Ready flow | `play_state_impl/chunk_04.rs` ready handler | ready picker tests | Implemented |
| §6 R-Ready eligibility | strength gate, occupancy, ring-vanish in ready handler | ring-vanish 1-in-16 tests | Implemented |
| §7 U-Use flows | `play_state_impl/chunk_04.rs::apply_u_use_*` for every public family (torch/gem/key/scroll/potion/Moonstone/regalia/shard/carpet/skull key/spyglass/HMS plans/sextant/pocket watch/wooden box); shard destruction follows public issue #31 exact party positions, shard/flame pairing, matching Shadowlord encounter north of the party, and save-backed quest-progress bits | per-item use tests in `chunk_03.rs`–`chunk_05.rs`, shard/flame tests in `chunk_17.rs` cover all three published native Eternal Flame positions, exact scene/floor/coordinate rejection, and quest-progress bit mutation | Implemented |
| §8 Implementation contract | `equipment.rs` 0xFF sentinel; carried/readied separation | contract tests | Implemented |
| §9 Boundaries | Ring of Invisibility/Regeneration in combat | combat ring-vanish tests | Implemented |

### `systems/containers.md`, `systems/traps.md`, `systems/hidden-treasures.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `containers.md` §1–§10 | `containers.rs`, `play_state_impl/chunk_04.rs` Get/Open/Jimmy/Search dispatch | chest, Search, object-pickup tests in `chunk_04.rs`, `chunk_19.rs` | Implemented |
| `traps.md` §1–§5 | `traps.rs::TrapEffect`, `containers.rs` trap routing | trap-effect tests | Implemented |
| `hidden-treasures.md` §1–§4 | `hidden_treasures.rs` 113-record table + per-record gates | `hidden_treasure_*` tests | Implemented |

### `systems/conversation.md`, `formats/tlk.md`, `systems/quest-flags.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `conversation.md` §1 Overview | `conversation_session.rs`, `tlk_runner.rs` | session lifecycle tests | Implemented |
| §2 T-Talk command | `play_state_impl/chunk_04.rs` Talk dispatch, position check | Talk tests in `chunk_21.rs` | Implemented |
| §3 Four `.TLK` files | `tlk_runner.rs` filename dispatch | TLK file load tests | Implemented |
| §4 NPC blob structure | `tlk_runner.rs` leading-entry parser, high-bit strip | parser tests | Implemented |
| §5 Keyword scan | `conversation_session.rs::tlk_player_input_kind`, reserved-word table | keyword-match tests | Implemented for the five functional reserved words plus all 29 published rebuke words from `cleak/u5-spec#58`; unmatched input uses the normal no-match response |
| §6 Keyword input loop | `conversation_session.rs::AwaitingKeyword` phase | phase tests | Implemented |
| §7 Byte runner | `tlk_runner.rs` full control-code dispatch | per-control-code tests | Implemented |
| §7.1 Printable text | `tlk_runner.rs` word-buffer, soft-break | text emission tests | Implemented |
| §7.2 Avatar name (`0x81`/`0x82`) | `tlk_runner.rs` interpolation | name substitution tests | Implemented |
| §7.3 Pause (`0x83`/`0x8F`) | `tlk_runner.rs` pause emit; redraw delegated to frontend | pause tests | Implemented |
| §7.4 Newlines (`0x8A`/`0x8D`) | `tlk_runner.rs` newline emit | newline tests | Implemented |
| §7.5 Print mask / curse (`0x8B`/`0x8E`) | `tlk_runner.rs::PrintMask`, curse-check hook | mask-pair tests | Implemented |
| §7.6 Branching (`0x85`/`0x86`/`0x8C`/`0xFE`) | `tlk_control_codes.rs::TlkActionDispatchVerb`, `tlk_if_else_alt_branches`, `play_state_impl/chunk_04.rs::apply_tlk_action_grants` | gold-payment, action-letter, IF/ELSE, karma-threshold tests, plus a sanitized shipped-TLK corpus audit for public action/payment/branch controls | Implemented (`0x85` toll-milestone karma — `cleak/u5-spec#27`) |
| §7.6 `0x87` follow-up scan | `tlk_runner.rs::TlkRunStop::FollowUpKeywordScan` | follow-up scan tests plus shipped-TLK corpus control-shape coverage | Implemented |
| §7.7 Labels / GOTO | `tlk_runner.rs` label dispatch | label scan tests | Implemented |
| §8 Common-word dictionary | `common_words_io.rs` public issue #33/#40 128-entry shared table; `shoppe_bark.rs` shared renderer path | dictionary, SHOPPE bark, and sanitized full-record render-audit tests | Implemented |
| §9 Conversation flow | `conversation_session.rs` opening preamble (`TLK_OPENING_DESCRIPTION_PREFIX = "Thou seest "`), greeting, keyword loop, Bye cleanup | flow tests | Implemented |
| `tlk.md` §1–§10 (format) | `tlk_runner.rs` decoder, dictionary | parser tests | Implemented |
| `quest-flags.md` §1–§7 | `quest_flags.rs`, `conversation_session.rs` branch-flags state, action-grant cleanup in `play_state_impl/chunk_04.rs::run_final_conversation_cleanup` | branch-flag, cleanup tests | Implemented |

NPC dialogue status-tile (`cleak/u5-spec#44`): the engine applies the public
sleeping/no-response live-tile mapping before shop/dialog dispatch and returns
control to the caller on refusal; route-smoke covers both status-tile refusals
through the asset-backed Talk command path.

### `systems/npc-schedules.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§11 | `npc_runtime.rs` state machine, `town_tables_io_movement.rs`, schedule walker invoked from `town_mode.rs` per turn; `0xC8`/`0xC9` floor-link BFS; town player-slot sync preserves linked scheduled NPC active objects even when their sprite class is `0xFC` | `scheduled_npc` tests; shipped `.NPC` boundary-hour/multi-floor scheduler corpus test when local clean assets are present; town-floor change tests | Implemented |

### `systems/shops.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1 Overview | `shops.rs`, `shop_runtime.rs`, `shop_session.rs` | shop-runtime tests | Implemented |
| §2 Triggering | `conversation_session.rs` shop trigger detection, Talk → shop arm | shop-trigger tests | Implemented |
| §3 Eight shop kinds | `shops.rs` arm dispatch (0x81..0x88) | per-arm tests | Implemented |
| §4 SHOPPE.DAT structure | `shoppe_records.rs`, `shoppe_bark.rs` | parser and sanitized render-audit tests | Implemented |
| §5 Bark renderer | `shoppe_bark.rs` substitution (`%/^/$/&/*/#/@`) | bark tests plus aggregate coverage that renders every non-empty local `SHOPPE.DAT` record when clean assets are present | Implemented |
| §6 Pricing model | `shops.rs::arms_shop_price`, healer table, etc. | pricing tests in `chunk_21.rs` | Implemented |
| §7 Inventory model | `equipment.rs` stock tables; inn registry in `play_state_struct.rs` | inn-stay tests | Implemented |
| §8.1 Weaponsmith/armourer | `shops.rs::ArmsShop`, `shop_session.rs::arms_shop_for_scene`, `shops.rs::arms_shop_stock_letter_index` | arms scene-row, published stock-row, and transaction tests | Implemented (public #41 scene-to-`a..h` stock rows) |
| §8.2 Guildmaster | `shops.rs` guild prices | guild tests | Implemented |
| §8.3 Healer | healer arm in `shop_runtime.rs`; Minoc bypass | healer tests | Implemented |
| §8.4 Innkeeper | `shop_runtime.rs` inn flow; stay counter in `clock.rs`; public issue #15 Intelligence-adjusted rest, leave, and pickup charges plus paid-rest class recovery and poison death conversion | inn tests | Implemented |
| §8.5 Tavernkeeper | tavern arm in `shop_runtime.rs` | tavern tests | Implemented |
| §8.6 Sage | sage arm: shared 26-row paid keyword lookup, strict four-letter topic boundary, SHOPPE record 84 fee quote, gold debit before success-template RNG, short-funds exit with SHOPPE record 91, and SHOPPE record 85..=88 success rendering | sage runtime tests plus full public #13 table-sync, SHOPPE rendering, and PRNG-timing tests | Implemented |
| §8.7 Shipwright | shipwright arm; fixed scene-entry delivery coordinates; pending vehicle in `play_state_struct.rs` | shipwright tests | Implemented |
| §8.8 Horse trader | horse arm | horse tests | Implemented |
| §8.9 Reagent vendor | `shops.rs` per-herbalist matrix | reagent tests | Implemented |
| §8.10 Horse trader correction | `shop_session.rs` public issue #28 horse-trader scene rows; `shop_runtime.rs` Intelligence-adjusted quote/no-marker refusal state machine; `input_dispatch.rs` adjacent marker placement and active-object creation | all-stable horse-trader route smoke/visual routes and talk/shop tests in `chunk_21.rs`, including no-marker refusal | Implemented; obsolete stationary-display purchase runtime removed |
| §9 Karma effects | no karma gating on shop pricing | — | Implemented |

### `systems/karma.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§11 | `karma.rs` selector, `shrine_virtue.rs` Codex turn-in rewards, `endmsg_io.rs` (KARMA.DAT 6-record verdicts), `tlk_runner.rs::tlk_if_else_alt_branches` | karma/threshold and Codex turn-in tests in chunks 13, 16, and 21 | Implemented (`0x85` toll progress — `cleak/u5-spec#27`) |

### `systems/intro.md`, `systems/chargen.md`, `systems/u4-transfer.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `intro.md` §1–§14 | `intro.rs`, `intro_menu.rs`, `menu_dispatch.rs`, `pth.rs` (BRITISH.PTH walker), `return_to_view.rs`, `story_io.rs`; Bevy intro shell composes published title bitmap, animates signature path, draws the four title-tick flame bands from `ULTIMA.16` records 1..=4 (the public issue #52 procedural flame stripe was withdrawn and its generator deleted), renders all 21 story slides with the spec-defined transition-strip and secondary-art draws plus step 6's published #69 doorway text, and reveals story step 1 and the STARTSC menu loader through the public issue #53 rectangle dissolve (the 36-tick and 320-tick column sweeps were withdrawn) before sampling menu input; Return-to-View preview rendering uses public title-tick animation families, transparent actor overlay composition, public #54 fixed strip captions from LoadMapStrip, high-opcode no-ops, 4x19 source strip loading, `(x, y + 7)` cell-effect coordinates, and scheduler/playback timing for preview ticks, cell effects, fixed wipes, fixed waits, trailing ticks, and one-shot actor draws; §11 (`A` submenu) is an artwork screen, not a text screen - the credits page is published band by band out of the hidden surface at its published origins, and any key (`Esc` included) starts the close phase that publishes the rebuilt menu back. No credits text is authored; the earlier `ACKNOWLEDGEMENTS_LINES` clean-room-authored constant was removed. The terminal harness has no pixel surface for it and still fails loudly through `intro.rs::require_graphical_acknowledgements_surface`. §11.2's full phase sequence is implemented: `intro_acknowledgements.rs` owns the geometry, step lists and one-BIOS-tick-per-step pacing for the part and close phases, and `u5-bevy` composites the rise, part, close and sink phases across a hidden surface and the visible page | intro/chargen menu tests in `chunk_01.rs`, `chunk_02.rs`; Bevy intro framebuffer/title-tick/story-wipe tests; Bevy acknowledgements phase/coverage/pacing tests; Return-to-View renderer/playback tests; `intro_acknowledgements::tests` (exact column coverage, step counts, pacing, the row-63 floor); `intro::tests::acknowledgements_refuses_placeholder_lines_without_a_pixel_surface` | Complete for §11 (cleak/u5-spec#72 is closed; the withdrawn slab-wipe model is gone); exact historical title-tick silhouette pixels and exact Return-to-View effect rasters remain Presentation work |
| `chargen.md` §1–§11 | `chargen.rs` questionnaire VM, gender prompt, virtue tournament, stat assignment | chargen tests | Implemented |
| `u4-transfer.md` §1–§10 | `u4_transfer.rs`, `u4_transfer_session.rs` state machine, public issue #16 `PARTY.SAV` source validation offsets, `INIT.GAM`/`INIT.OOL` seed handling, stat translation, OOL ordering; `u4_transfer_preview.rs` + `crates/u5-bevy/src/u4_transfer.rs` for the `§6` preview screen | u4-transfer tests; `intro-u4-transfer-found` and `intro-u4-transfer-panels` in the visual frame suite | Implemented. The U5-side seed pair is `INIT.GAM` (save image, 4192 bytes) and `INIT.OOL` (object overlay); `#88` withdrew `BRIT.GAM`, which **does not ship at all** — the old constant named a file that is never present, and the test pinning it could not have failed either way. `§6.1`-`§6.6` are drawn (`f3ecfd1`) |

### `systems/save-load.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§8 | `save_load.rs`, `disk_io.rs`, `active_object_io.rs`, `play_state_struct.rs` four-file contract (SAVED.GAM/SAVED.OOL/BRIT.OOL/UNDER.OOL), empty-save guard, mirror writes including load-time and save-time extra `UNDER.OOL` branches, read/write retry wrapper, original binary content/resource loader disk I/O, vehicle/transition save round-trips | save/load tests across `chunk_03.rs`, `chunk_04.rs`, `chunk_05.rs`, `chunk_07.rs`, `chunk_09.rs`, `chunk_11.rs`, `chunk_13.rs`, `chunk_23.rs` | Implemented |

### `systems/movement.md`, `systems/overworld.md`, `systems/town-mode.md`, `systems/dungeon-mode.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `movement.md` §1–§10 | `direction.rs`, `tile_classes.rs`, `predicates.rs`, `transport.rs`, `active_object_io.rs`; native static terrain predicates cover the published foot, horse, carpet, ship, and facing-sensitive skiff tile sets | per-mode movement tests plus exhaustive 0..=255 transport predicate tests | Implemented |
| `overworld.md` §1–§15 | `play_state_impl/chunk_01.rs` overworld loop, `world_tables.rs`, `moongate.rs`, `moongate_phase.rs`, `lord_british_camp.rs`, native and sidecar encounters, public Word-of-Power seal rows | world tests in chunks 03, 05, 06, 07, 10, 12, 13, 15, 17, 23; moongate phase composition, ground-plate and save round-trip tests | Implemented **except `§9.2`'s blocking transit presentation** (the two-stage dissolve around the party-vanishing sprite), which is absent — transit is instantaneous. `§9.1` gate presence is implemented (`cd58ac9`): the per-render-frame moongate animator is withdrawn in full and deleted rather than adapted, and presence is a sixteen-step **global** counter — phase 0 draws the ground plate, `1..15` a composed frame whose bottom *N* pixel rows are the top *N* rows of the moon-gate tile via scratch tile `0x116`, phase 16 tile `0xDC` on the ordinary tile path. The ground plate is grass in play and `0x44` in the endgame. The counter is **persistent save-backed state at `SAVED.GAM` offset `0x02E1`**; it was previously not persisted at all, so save/load reloaded a gate at the wrong height. The scratch slot is saved and restored around every composition so `§9.2`'s party-vanishing sprite survives, and the renderer special-cases live terrain only so an overlay-painted `0xDC` keeps the plain tile path |
| `town-mode.md` §1–§17 | `town_mode.rs`, `town_tables.rs`, `location_audit.rs`, NPC schedules, dawn/dusk substitution, alarms | town tests in chunks 04, 06, 10, 11, 15, 19, 21, 23, 24, including sanitized shipped `LOCATION.DAT` aggregate owner/class/view audits when local clean assets are present | Implemented (public #51 tile `0x04` poison-gas step behavior is native; coordinate and tile-attribute sidecars no longer trigger this branch) |
| `dungeon-mode.md` §1–§17 | `play_state_impl/chunk_*.rs` dungeon loop, `dungeon_tables.rs`, raster in `crates/u5-bevy/src/lib.rs` first-person draw | dungeon tests in chunks 05, 12, 13, 18, 20, 23 | Implemented |

### `systems/encounters.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§14 | `play_state_impl/chunk_*.rs` random spawn probe (native + sidecar), fortunes-of-war counter, sleep-ambush in `rest_camp.rs`, dungeon-room arena selection, public #21 dungeon active-monster ambush setup | encounter and ambush tests | Implemented. `§4`/`§9`: a full active-object table does not make an acquisition fail — the spawner acquires *or evicts* — and the "table full" early-out the spec explicitly withdraws is gone. The 128-candidate coordinate loop was already correct and is now pinned |

### `systems/vehicles.md`, `systems/weather.md`, `systems/moons.md`, `systems/time.md`, `systems/rest-and-camp.md`, `systems/lighting.md`, `systems/doors-and-z-transitions.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `vehicles.md` §1–§11 | `transport.rs`, `play_state_impl/chunk_*.rs` Board/X-it/Yell sails, `ship_broadside.rs` Fire | vehicle save/load round-trip tests, broadside tests | Implemented |
| `weather.md` §1–§11 | `wind.rs`, Rel Hur cast in `magic.rs`, sail cadence | wind cast and sailing tests, including the two-wait into-wind case | Implemented |
| `moons.md` §1–§4 | `play_state_impl/chunk_*.rs` sky strip; moongate counters in `moongate.rs`; public issue #38 Felucca phase-0 glyph bytes for hours 10/11/19/20 | sky-strip and moon-glyph cache tests | Implemented. `§3` publishes that below the surface nothing is drawn, erased, *or cached*; the engine used to refresh the cached glyph bytes on every hour change in every mode, including dungeons, the Underworld plane and town basements, and no longer does (`f976af0`) |
| `time.md` §1–§13 | `clock.rs` cascade, Q/T tag handling, mode-specific increment | clock and cascade tests | Implemented |
| `rest-and-camp.md` §1–§10 | `rest_camp.rs`, `lord_british_camp.rs`, native town inn bed gate in `play_state_impl/chunk_07.rs`, `play_state_impl/chunk_08.rs::apply_completed_long_camp_recovery`, and hourly Ring of Regeneration tick in `chunk_09.rs` | rest, native inn bed/no-inn refusal, sidecar override, camp, ambush, long-camp recovery, and hourly ring tests | Implemented (ordinary rest has no direct HP/MP recovery; current checked-in spec matches public #47 issue-comment behavior) |
| `lighting.rs` §1–§11 | `lighting.rs` ambient + torch + light-spell counters | lighting tests | Implemented |
| `doors-and-z-transitions.md` §1–§15 | `jimmy.rs`, `play_state_impl/chunk_*.rs` open/get/look cascade, `ship_broadside.rs` BOOOM, secret doors, climb command | jimmy, open, secret-door, klimb tests | Implemented. `dungeon-mode.md §8`/`§13.1`/`§13.2` corrected Klimb (`2254649`): the whole pit family `0x6?` — marked and fired variants included, since the dispatcher masks to the high nibble — is an **ordinary descent** calling the same level-step helper a down ladder uses, not a surface-reset ejection. Klimbing Deceit level zero `(1, 3)` or Destard level zero `(7, 3)`/`(1, 7)` used to eject the party to Britannia from the top level of two dungeons. `§13.1` also withdraws the claim that a climb tests the cell it lands on — the ladder or pit underfoot is proof enough — so that predicate moved to the Up/Down level-change spells, and level-zero up runs the surface-reset contract so dungeons stay leavable |

### `systems/endgame.md`, `systems/blackthorn.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `endgame.md` §1–§12 | `endgame.rs`, `endgame_cinematic.rs`, `end_io.rs` (public END.DAT final narrative windows), `endmsg_io.rs`; Bevy endgame modal advances the six fixed END.DAT narrative windows without intro-style page wipes and presents the late full-screen certificate rectangle operation before certificate setup | endgame tests; Bevy endgame certificate-rectangle tests; route-smoke terminal-endgame confirmation/victory cases | Implemented |
| `blackthorn.md` §1–§11 | `blackthorn.rs`, `blackthorn_session.rs`, KARMA.DAT verdict mapping | Blackthorn challenge/rescue tests; Blackthorn audience/rescue route-smoke; Shadowlord route-smoke | Implemented |

### `systems/main-loop.md`, `systems/screen-mode-dispatch.md`, `systems/runtime.md`, `systems/input.md`, `systems/commands.md`, `systems/animation.md`, `systems/lighting.md`, `systems/view.md`, `systems/visibility.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `main-loop.md` | `play_state_impl/chunk_01.rs` mode dispatcher | mode-dispatch tests | Implemented |
| `screen-mode-dispatch.md` | `play_state_impl/chunk_*.rs` mode-state ownership, `save_load.rs::normalize_disk_prompt_mode`, `disk_io.rs` read/write retry prompt phases, shared disk retry/error presentation surfaced by TUI and Bevy Journey Onward | mode-state and disk-retry tests; TUI and Bevy intro disk-error presentation tests | Implemented |
| `runtime.md` | `play_state_struct.rs`, `play_options.rs`, boot in `boot.rs` | start-validation tests | Implemented |
| `input.md` | `input_codes.rs`, `input_dispatch.rs` | typeahead tests | Implemented |
| `commands.md` | `commands.rs` + per-command handlers in `play_state_impl/chunk_*.rs`; reference in `docs/commands.md` | per-command route-smoke and unit tests | Implemented |
| `animation.md` | `animation.rs`; visual cadence in `u5-bevy` | `chunk_29.rs` family/gate/selector tests | Implemented against the **replaced** `§6` family list (`774dff0`). The water/lava/fire/wind model this engine carried is withdrawn — "no water, lava, brazier or torch tile animates through this pass at all" — and with it `STATIC_TILE_ANIMATION_FRAME_WRAP` and the single shared frame per family. The five real families are waterfall `0xD4..0xD7`, fountain `0xD8..0xDB`, pendulum `0x80..0x83`, standard of Britannia `0xEC..0xEF`, and clock `0xFA..0xFB` / bellows `0xFC..0xFD`, with nested gating (waterfall and fountain ungated, then bit 0, then bit 1 only inside the bit-0 gate) giving net 1x / 2x / 4x rates, **per-id selectors** so a four-frame family's ids stay a quarter-cycle apart, and `STATIC_TILE_ANIMATION_PERIOD_TICKS = 8`. Visible consequence: repaint cadence drops sharply in ocean and coastal areas. The moongate presence counter is not a member of these families and `tick_static_tiles` never advances it |
| `view.md` | `view_classes.rs`, V-View overlays in `play_state_impl/chunk_*.rs`; explicit View/Peer/X-Ray overlay modes and peer/gem alternate-bank raster branch; Bevy overlay draws | View overlay route-smoke (surface, dungeon, Spyglass chunk-map, Peer, X-Ray) | Implemented (exact remote-view pixels and chunk-map pixel parity — Presentation work) |
| `visibility.md` | `visibility.rs`, persistent radius-5 visibility grid / companion terrain band, light-mask wrap | persistent visibility-buffer and visibility tests | Implemented **except `§12.6`**: the night-time rotating light beacon is not implemented at all — no bearing counter, no rotating beam, nothing. Its gate runs only while ambient is *strictly below* full daylight. `lighting.rs` carries a comment recording that the withdrawn `MOONGATE_ANIMATOR_DAYTIME_THRESHOLD` was this beacon's gate, misattributed to moongates and inverted |

### `systems/text-output.md`, `systems/stats-panel.md`, `systems/display-driver.md`, `systems/display-driver-abi.md`, `systems/overlay-abi.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `text-output.md` §1–§7 | `text_wrap.rs` fixed-cell text window, control bytes `0xFB`..`0xFF`, padded numeric printer, descriptor cursors | text-wrap tests in `chunk_13.rs`, `chunk_14.rs` | Implemented |
| `stats-panel.md` §1–§5 | `stats_panel.rs` party rows, active-cursor handling, combat overlays | stats-panel tests | Implemented |
| `display-driver.md`, `display-driver-abi.md` | `crates/u5-bevy/src/lib.rs` framebuffer composition, atlas-backed top-down and first-person rasters, fixed-font shared surface, rectangle-dissolve and endgame certificate-rectangle rendering, and per-step visual route replay harness; Tandy CLI raster depth aliases route to the EGA-equivalent path while Hercules is explicitly rejected as outside v1 scope | Bevy framebuffer/story-wipe/STARTSC/endgame-transition tests; visual frame suite; visual route suite; CLI display-depth tests | Implemented with public story/menu-loader timing. `§6`/`§8`/`§9.2` back-buffer routing was corrected in `f976af0` and is the largest fix of that batch: the clipped rectangle fill (`0x3F`), the 16-by-16 tile entry (`0x51`) and the fixed-cell glyph entry (`0x5D`) each branch to a real back-buffer body, and the engine treated tile and glyph as front-buffer-only, so **both were silently discarded on the hidden surface** — which would leave the endgame and map-viewport fades dissolving stale pixels. The line entry (`0x33`) is front-buffer-only *regardless* of the selector, which is a draw to the front buffer, not a skipped draw; it was skipping. Exact late endgame rectangle cadence and Return-to-View effect rasters remain presentation work |
| `overlay-abi.md` | `crates/u5-bevy/src/lib.rs` overlay composition for status/Z-stats/endgame/intro | Bevy overlay tests | Implemented |

### `systems/prng.md`, `systems/timing.md`, `systems/stat-arithmetic.md`, `systems/active-objects.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `prng.md` | `prng.rs` LCG; `random_*` helpers in `play_state_*.rs` | rng round-trip tests | Implemented |
| `timing.md` | `timing.rs` wait counters; integrated in clock and sailing cadence | timing tests | Implemented |
| `stat-arithmetic.md` | `stat_arithmetic.rs` saturating add/sub | stat-arith tests | Implemented |
| `active-objects.md` `§1`-`§7` | `active_object_io.rs` 32-slot table, OOL persistence, `ool_audit.rs` aggregate active-object overlay census | active-object tests across chunks plus synthetic and local-clean `.OOL` aggregate audit coverage | Implemented |
| `active-objects.md` `§4` eviction cascade | `allocate_active_object_slot` → `active_object_eviction_victim`; `active_object_eviction_phase` derived from `active_object_eviction_byte_accepted` + `active_object_eviction_phase_is_off_screen` | allocator cascade tests; the two tests that asserted the old "table full" early-out at horse purchase and Y-Yell Shadowlord install are corrected | Implemented (`a48e2ef`, trigger corroborated by `f976af0`). Previously the allocator ran phase 1 only and returned `None`, so a full ordinary range **silently dropped** horse purchases, dropped items, shipwright deliveries and encounter spawns. Phases run in published order, lowest index up within each; slot 0 and the reserved band `24..=31` are never victims, and type byte `0xB5` is rejected by every phase including last-resort phase 10 |
| `active-objects.md` `§8.1` distance gates | `ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW` (`§4`), `ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT` (`§8.1`) | prune/eviction window tests | Implemented. Both gates are **square per-axis windows, not radii** — each axis tested separately against the same bound, no distance and no disc, because a disc would prune the window corners the original keeps. Differences are formed in **unsigned eight-bit** arithmetic, wrapping with the 256-cell coordinate space with no map-seam case; `§8.1` measures forward from the scroll base, the loaded window's origin corner, not a centred band. The `_RADIUS` names asserted a quantity the code does not compute and are gone, and the two constants are kept apart because `§8.1` warns that one shared constant serving both pruning and eviction is a sign the two have been conflated |
| `active-objects.md` `§8` outdoor walker | `active_object_io.rs` predicates only | predicate tests | **Published, not implemented.** `FC_PROXIMITY_AGE_CAP`, `outdoor_serpent_dragon_triggers` and `outdoor_water_creature_attack_aligned` have **zero production call sites**, so the walker's first phase — adjacent hostile engagement, sea-serpent/dragon breath, whirlpool transition, ship broadside — never runs. The step-committer and classifier halves of `§8` are wired; this phase is not |

## Formats

| Format | Evidence | Tests | Status |
|-------|----------|-------|-------|
| `formats/bit.md` (BIT bitmaps) | `fonts_io.rs::parse_title_bit`, `parse_british_bit`, `parse_wd_bit`, and explicit legacy local-loader fallbacks; `intro.rs` placement | bit decode tests, including canonical sparse acceptance, LZW-wrapper rejection on parser entry points, and local asset loader compatibility | Implemented at canonical sparse depth; local preprocessed fallback remains compatibility-only |
| `formats/brit-dat.md` | `map_io.rs::load_brit_dat`, `world_tables_io.rs` | brit decode tests | Implemented |
| `formats/cbt.md` | `combat_arena.rs::parse_cbt_record` | CBT decode tests | Implemented |
| `formats/data-ovl.md` | `misc_tables_io.rs`, `world_tables_io.rs` overlay readers | DATA.OVL field tests | Implemented |
| `formats/dungeon-dat.md` | `dungeon_tables_io.rs::load_dungeon_dat` | dungeon decode tests | Implemented |
| `formats/end-dat.md`, `formats/endmsg-dat.md` | `end_io.rs`, `endmsg_io.rs` | END/ENDMSG tests | Implemented |
| `formats/font-ch.md`, `formats/font-hcs.md`, `formats/font-pcs.md` | `fonts_io.rs::load_ch_font`, `fonts_io.rs::load_hcs_font`, canonical sparse `parse_proportional_font_resource`, explicit legacy local `load_legacy_proportional_font`, and `visual_asset_audit.rs::audit_visual_assets` fixed-font aggregate report | font decode tests plus synthetic and local-clean fixed-font aggregate audits for `IBM.CH`, `RUNES.CH`, `IBM.HCS`, and `RUNES.HCS` when assets are present; sparse PCS tests now reject LZW wrappers on canonical parser entry points and exercise legacy compatibility separately | Implemented at public sparse-resource depth; pre-decoded local variants remain noncanonical compatibility fallbacks |
| `formats/karma-dat.md` | `endmsg_io.rs::load_karma_dat` (6 verdict records) | karma decode tests | Implemented |
| `formats/location-dat.md` | `map_io.rs::load_floor`, `map_io.rs::resolve_location_floor_page`, `town_tables_io.rs::load_*_dat`, `location_audit.rs::audit_location_dat_files` | layout constants, location decode tests, synthetic `LOCATION.DAT` audit, and local clean all-family authored-cell audit when assets are present | Implemented |
| `formats/look2-dat.md` | `misc_tables_io.rs::load_look2` (descriptions) | look2 decode tests | Implemented |
| `formats/lzw.md` | `lzw.rs` decompressor | LZW round-trip tests | Implemented |
| `formats/miscmsg-dat.md` | `miscmsg_io.rs` | message lookup tests | Implemented |
| `formats/npc.md` | `npc_runtime.rs` + `town_tables_io.rs` NPC block decode | NPC decode tests | Implemented |
| `formats/ool.md` | `active_object_io.rs`, `ool_audit.rs::audit_ool_files` | OOL round-trip tests plus synthetic and local-clean aggregate audits over `SAVED.GAM`, `SAVED.OOL`, `BRIT.OOL`, `UNDER.OOL`, and `INIT.OOL` when assets are present | Implemented |
| `formats/pth.md` | `pth.rs::load_path_records`, Bevy signature animation | PTH parse tests | Implemented |
| `formats/question-dat.md` | `question_io.rs` | QUESTION decode tests | Implemented |
| `formats/saved-gam.md` | `save_load.rs`, `play_state_struct.rs` | save/load round-trip tests | Implemented |
| `formats/shoppe-dat.md` | `shoppe_records.rs` | SHOPPE decode tests | Implemented |
| `formats/signs-dat.md` | `signs_io.rs` | SIGNS lookup tests | Implemented |
| `formats/story-dat.md` | `story_io.rs` | STORY decode tests | Implemented |
| `formats/tiles.md` | `graphics.rs`, `graphics_io.rs` tile sheet, paired image-directory, and sprite-sheet decode; `visual_asset_audit.rs::audit_visual_assets` aggregate report for tile atlases, image directories, and sprite sheets | tile-sheet/image-directory/sprite-sheet tests plus synthetic and local-clean aggregate audits for `TILES`, `STARTSC`, `TEXT`, `DNG1`-`DNG3`, `ENDSC`, `END1`/`END2`, `STORY1`-`STORY6`, `ULTIMA`, `CREATE`, `ITEMS`, and `MON0`-`MON7` in both `.16` and `.4` depths when assets are present | Implemented |
| `formats/tlk.md` | `tlk_runner.rs`, `tlk_control_codes.rs` | TLK runner tests | Implemented |
| `formats/under-dat.md` | `map_io.rs::load_under_dat` | underworld decode tests | Implemented |

## Catalogs

| Catalog | Evidence | Tests | Status |
|--------|----------|-------|-------|
| `catalogs/gazetteer.md` | `world_tables_io_locations.rs`, sidecar overlays | locations tests | Implemented (stock location entry/return coordinates are native; several non-location transition families remain sidecar-backed pending public gazetteer rows) |
| `catalogs/item-list.md` | `equipment.rs`, `containers.rs::InventoryAddClass` | inventory-add tests | Implemented |
| `catalogs/monster-bestiary.md` | `combat_stats.rs::combat_class_stats`, `combat_ranged_effect_stats`, `combat_class_traits` | exhaustive per-class stat, ranged/effect side-row, trait, and ability-hook tests in chunk 23 | Implemented |
| `catalogs/npc-roster.md` | `npc_runtime.rs`, conversation/shop dispatch | roster tests | Implemented |
| `catalogs/quest-graph.md` | `quest_flags.rs`, `endgame.rs` | quest-flag tests | Implemented |
| `catalogs/spell-list.md` | `magic.rs` parser, cost, scene-mask tables | spell metadata tests | Implemented |
| `catalogs/tile-catalog.md` | `tile_classes.rs`, `view_classes.rs`, `tile_helpers.rs` | tile-class tests | Implemented |

## Remaining Public-Spec Gaps

There are none. Every issue in `cleak/u5-spec` is closed — 88 of them, including
the six this engine filed (`#78` intro/menu, `#79` gameplay chrome, `#80`
per-scene floor pages, `#81` command echoes and dungeon tables, `#82`
endgame/chargen/`PARTY.SAV`, `#83` light-byte semantics) and the five later ones
(`#84`-`#88`). What is left is engine work against published contracts, and a
small number of details the spec states honestly that it does not publish.

### No refusal stands for an unimplemented contract

The endgame certificate was the last gate on an *unpublished* contract. The
Ultima IV transfer preview was the last gate on a *published but unbuilt* one,
and it was built in `f3ecfd1`.

**Category (a) — refusals that stand for an unimplemented published contract —
is empty.** That is the headline of this refresh. It was verified by grepping
`crates/` for `panic!`, `-> !` and the `forbidden fallback` marker, excluding
`tests_inline/`, `test_fixtures.rs` and `#[cfg(test)]` helpers, and classifying
every hit.

### Refusals that remain, and what each one is

| Site | Kind |
|---|---|
| `u5-tui` `require_terminal_story_renderer_contract` | **(b) Structural — no surface.** Story slides are a graphical screen; `--intro` refuses rather than printing a diagnostic substitute |
| `u5-tui` `require_terminal_return_to_view_renderer_contract` | **(b) Structural — no surface.** The preview is a 304x64 tile strip. `#54` is published and the graphical shell implements it; the terminal harness has no pixel surface to blit it onto |
| `u5-tui` `require_terminal_u4_transfer_renderer_contract` | **(b) Structural — no surface.** Retained deliberately after `#73` shipped: the graphical preview exists, and a text transcript of it would be the invented substitute the no-fallback rule forbids |
| `u5-runtime` `intro.rs::require_graphical_acknowledgements_surface` | **(b) Structural — no surface.** The credit lines are drawn into the `STARTSC` bitmap and nothing typesets them, so printing clean-room-authored credits would invent the one thing the original never types. Replaces the old `require_acknowledgements_contract`, whose message was scoped to the retracted slab cadence |
| `u5-runtime` `display_driver.rs` title-tick operation | **(c) Injection guard.** The caller must supply the `ULTIMA` bands; generated clean-room frames are refused |

`u5-runtime` `intro_acknowledgements.rs` also carries two `panic!` sites in the
part and close phases, but those are internal coverage assertions — they fire if
a phase leaves a column of the band unpublished — not contract gates.

### Published but not implemented

These are honest gaps: the contract is published and the engine does not do it.

- **`visibility.md §12.6` night-time light beacon.** Not implemented at all —
  no bearing counter, no rotating beam. Its gate runs only while ambient is
  strictly below full daylight. The engine's only trace of it is a comment in
  `lighting.rs` recording that the withdrawn `MOONGATE_ANIMATOR_DAYTIME_THRESHOLD`
  was this beacon's gate, misattributed to moongates and inverted.
- **`overworld.md §9.2` blocking moongate transit presentation.** The two-stage
  dissolve around the party-vanishing sprite is absent; transit is instantaneous.
  Gate *presence* (`§9.1`) is implemented.
- **`active-objects.md §8` outdoor walker, first phase.** Adjacent hostile
  engagement, sea-serpent/dragon breath, whirlpool transition and ship broadside
  exist as predicates with zero production call sites, so none of it runs.
- **Dungeon first-person wall/scenery tables.** `#84` published the billboard
  slot-to-role mapping and the corridor now draws from its banks, but no
  wall/scenery table exists in `crates/`, so first-person presentation is not
  parity-checked.
- **The required-disk contract.** There is no disk-swap handling, which is
  correct for a single-directory install but means we carry no model of it. What
  *is* implemented is the part of the swap loop that has a filesystem effect:
  `save_load_needs_underworld_disk_swap`, the `UNDER.OOL` mirror flush and its
  `§5.2` defensive re-flush, and the `screen-mode-dispatch.md §5` disk-prompt
  mode normalizer. No disk labels, prompts or swap timing are modelled.

### Details the spec publishes as unpublished

- The two rune digraph code points. `endgame.md §9.3` publishes that TH and ST
  each occupy one character in the closing title's encoding, and that the
  at-sign is the word space, but not which code points the digraphs use - it
  explicitly allows an engine to supply its own mapping. `runic_line_encoding`
  applies the published word-space rule and leaves the digraphs as two runes;
  that is the only part of the certificate that is not exact.
- The driver's per-step pixel pattern for the brightness entry the endgame gate
  flare drives (`display-driver-abi.md`).
- Any wall-clock length for the rectangle dissolve, so the engine completes it
  as the single blocking call `#53` specifies rather than pacing it. The same
  treatment applies to the acknowledgements rise and sink phases, which `§11.2`
  gives no wait at all.
- The alternate-depth (`.4`) conversion of the archives named in `#82`.
- The 14 ms flourish step is a derived target inside a 10.5-15.8 ms bracket, not
  a measured figure.

### Review heuristics

`docs/review-heuristics.md` records the three mechanical checks that between
them found every real defect in this pass: **does anything read this?** (a
reference count), **is this byte inside the save window?** (a lifetime test with
one subtraction), and **decompose the name** (a name is an assertion no review
checks). None of them requires reading code attentively, and none of the day's
real finds came from doing so.

They are worth running as routine rather than on suspicion, because they catch
two opposite defects that present identically as "a well-tested module with a
confident name" — real code implementing an unreal contract (`combat.md §7`'s
post-round maintenance pass) and unreal code implementing a real contract (the
active-object eviction predicates, the spell scene allow-mask, the outdoor
walker's first phase). All four of those were found by check 1 alone — a
reference count, not a reading — and the last of them is still open.

The file also records why a **repair pass** deserves the same scrutiny as the
thing it repairs: corrections in this project have introduced wrong names while
fixing wrong names, regressed a section via a retracted correction, and leaked
private paths into a public document (`#87`). A correction can breach a boundary
the original error did not.

## Presentation Work (Separate From Gameplay Correctness)

These are visual/audio polish items called out in `TODO.md` Milestone 3 and the
spec's "exact visual parity deferrals" section. They do not block gameplay
correctness; the engine renders the published content with clean substitute
overlays where exact historical pixels are not public.

- Title-tick silhouette pixels are no longer deferred: the four bands are
  `ULTIMA` records 1..=4, read from the shipped asset at runtime. The
  clean-room flame generator and palette-cycle table are deleted.
- Exact per-shop live bark layout, waits, and pacing beyond the inherited Talk-to-shop window handoff.
- Return-to-View effect-raster pacing internals (the preview geometry itself is
  published as `#54` and implemented).
- Exact remote-view panel pixels for X-Ray / Peer (`cleak/u5-spec#60`).
- Exact dungeon minimap glyph/floodability edge cases (`cleak/u5-spec#60`).

## Conclusion

Across the public spec - 22 systems documents, 22 format documents, and seven
catalogs - every gameplay-correctness deliverable that has complete public data
has a corresponding engine implementation backed by tests, route-smoke
coverage, or visual-frame-suite captures.

Presentation is the part that moved most on 2026-08-22: six packages landed the
intro title/menu from `ULTIMA.16`, the measured gameplay screen chrome, the
interior visibility carve with ambient-as-squared-threshold lighting and the
`#42` local-light disc, the command-echo transcript, all 21 intro story slides,
and the endgame's own surface with the chargen prompt screen, the U4 media
branch and the asset-write guards. That is a large step toward parity, not
parity itself: it is measured against black-box observation of the shipped
assets, and the gaps section above lists what is still unpublished or still
known-wrong. Nothing here should be read as pixel-exact parity for a screen it
does not name.

The six issues this engine filed have all been answered, and the work from them
is in: the published chrome contract and command echoes, the two-pass border
end-cap composite shared across ribbon interruptions and the message-window
prompt, the `#83` local-light influence mask, the `#80` per-scene base-page
table (which withdrew the `page = sub_map_index * 2 + floor` model - wrong for
22 of 32 locations), and the `#82` endgame/chargen/`PARTY.SAV` work including
the certificate. The gaps section above is what is genuinely left, and it is
now engine work rather than missing contracts.

The 2026-08-23 pass then closed the last two published-but-unbuilt intro
contracts (`#72` acknowledgements, `#73` transfer preview) and spent the rest of
its effort on the opposite problem: mechanisms the tree already contained that
were wrong, misattributed, or never called. Three models were retracted -
`combat.md §7`'s post-round maintenance pass, the water/lava tile-animation
family list, and the per-render-frame moongate animator - and two published
mechanisms were connected for the first time: the active-object eviction cascade
and the spell scene allow-mask. Neither had ever run in production despite being
fully modelled and heavily tested, which is why `docs/review-heuristics.md`
leads with the reference count.

Two corrections that arrived with those answers are worth recording because
they were wrong in the engine, not merely unimplemented: `TORCH_LIGHT_FLOOR`
and `LIGHT_SPELL_FLOOR` were inverted (magic light is the brighter one), and
the visual frame suite was rendering no menu window at all while the live path
was correct - fixed structurally by driving the suite's intro state through the
real render path rather than a parallel one. A parallel render path in a test
harness hid a defect the harness existed to catch.

The shipped palette is also **not** stock EGA: index 6 is `(170, 170, 0)` dark
yellow, not `(170, 85, 0)` brown, and it is the only index that differs. Forty-
two decoded sub-images contain index 6 - `END1` panel 1 is 32% of it, `STORY1`
slide 0 25%, the `STARTSC` acknowledgements parchment 6% - so several screens
changed hue once it was corrected. Nothing in the game reprograms the palette
after mode setup; apparent recolouring is a restricted plane write mask or a
display effect mutating the loaded asset data, never a palette change.

Verified on 2026-08-23 at `9e437d5`: 2902 u5-runtime, 170 u5-bevy, 96 u5-tui
tests pass, `cargo fmt --all -- --check` is clean, `cargo clippy --workspace
--all-targets` reports zero errors, `--route-smoke` passes all 493 cases,
`--visual-frame-suite` writes 193 PNGs and `--visual-route-suite` writes 1814.
Every asset-backed run used a copy of the asset directory.
