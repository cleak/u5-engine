# Completion Audit

This audit maps each public-spec deliverable (the systems and formats published
in `C:\Projects\Rust\u5-clean\u5-spec`) to concrete engine evidence
(`crates/u5-runtime`, `crates/u5-tui`, `crates/u5-bevy`) and to test coverage.

Created on 2026-05-19; last refreshed on 2026-08-22 at `d4fc579`, after the
six-package presentation-parity pass, the `cleak/u5-spec` sweep that closed
every issue including our own `#78`-`#83`, and the work implemented from those
answers.

**Read spec contracts from the GitHub issues, not from the local checkout.**
`C:\Projects\Rust\u5-clean\u5-spec` is read-only from this workspace and is
stale at `9a898d1`, behind spec head `8192d67` and several of its retractions
(notably `#42` local light, `#53` reveal transitions, `#54` Return-to-View,
`#65`/`#66`/`#67` title sequence, `#69` doorway text, `#70` font metrics,
`#80` floor pages, `#82` endgame/chargen). The spec queue is now empty - 83
closed, 0 open - and head has moved several times past `9a898d1`, so an audit
checked against the local files would disagree with this document.

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

Where an audit row references a pending issue, the engine carries a clean
implementation or conservative placeholder that avoids private-derived guesses
until the public spec publishes the missing rule.

## Verification Baseline

Runtime, TUI, route-smoke, and visual-route results refreshed alongside this
audit on 2026-05-24; Bevy frame-suite evidence remains from the latest display
audit:

- `cargo test -p u5-runtime` — 2634 tests pass, including stats-panel
  combat-row inverse-video style coverage.
- `cargo test -p u5-runtime published_location --tests` - 6 focused tests pass,
  including exhaustive entry and return coverage for all forty public
  world-location rows without sidecars.
- `cargo test -p u5-tui` — 79 tests pass, including temp-directory binary
  smoke for empty-save Journey Onward, deterministic Create Character followed
  by `--from-save --play-script`, intro-driven U4 transfer commit, and a
  confirmed `QY` save/reload round trip.
- `cargo test -p u5-bevy` — 67 tests pass.
- `cargo run -p u5-tui -- --route-smoke C:\Games\U5-Clean` — 493 scripted cases pass,
  The same run can now write a sanitized 2183-frame initial/per-command/final
  route manifest that compares cleanly against itself with
  `--compare-frame-manifests`, including TLK-backed reserved-word and no-match
  conversation routes across
  all 32 named-location scenes in the town, dwelling, castle, and keep
  dialogue families,
  including all 40 published stock world-location entry rows and four
  extended-session cases that exercise 5–12 commands across
  Britannia exploration, castle walking, dungeon turning/search, and
  multi-round Doom combat to prove the engine sustains long playable sessions,
  save/reload checkpoints across transport, plane transitions, fixed hidden
  treasure, horse-trader delivery, ship X-it/skiff, dungeon ladders, and
  dungeon exits,
  plus active shop/modal flows, Blackthorn audience correct/wrong and
  rescue-refuge routes, fixed hidden-treasure zero-key/single-use/daily/stacked routes,
  PRV Gate Travel success/refusal paths, saved-slot natural moongate live-entry
  paths, public #31 native shard/Eternal Flame destruction routes, public #32 Britannia/Doom Word-of-Power seal opening, public #15
  accepted inn-rest pricing, public #44 sleeping/praying Talk refusals, public
  #48 Blink ray landing, all-cardinal directed Sleep/Poison Wind/Death Wind/Flame Wind
  combat casts, combat field marker casts/removal, combat utility fallback casts, targeted Magic Missile/Tremor/Repel
  Undead/Charm/Polymorph/Clone casts, Conjure/Swarm/Summon Daemon routes,
  special death-marker Kill routes, combat-entry party descriptor routes, and combat terminal cleanup routes,
  #51 poison-gas doorway step, public #47 dungeon no-direct-recovery rest,
  completed long-camp recovery, and hourly ring tick, public #13
  all-nine-tavern lore selector routes plus sage paid-success/short-funds paths, public #41
  all nine arms-shop first-stock purchases and terminator-letter refusals, public #28
  all-stable horse-trader purchases, accepted shipwright frigate/skiff dock
  deliveries, save/reload durability for queued shipwright deliveries before
  town exit, native town walk-on stair up/down/crossing routes,
  town attack/alarm/arrest routes, and
  ship broadside fire, horse boarding, dungeon torch ignition, Mix/Ready/New
  Order command workflow, combat-active Board/Enter/Fire/Hole-up/Ignite/Mix/New
  Order/Talk refusal rows, combat-active digit selection/clear, Escape abort,
  Ctrl-S music toggle, lowercase direct movement, Horse and non-horse
  wishing-well branches, public #56 terminal endgame missing-box jitter and full
  victory cinematic routes, the public Britannia chasm fall route, the forced
  whirlpool Underworld branch, and fixed narrative gate open/ordained-block
  routes through real asset-backed play states.
- `cargo run -- --save-frame-suite target\codex-view-class-gallery-frame-suite C:\Games\U5-Clean`
  — 16 PNGs, every frame nonblank with stable hashes, including gem/Peer/X-Ray
  surface View class galleries.
- `cargo run -p u5-tui --features visual -- --visual-frame-suite
  target\codex-view-class-gallery-visual-frame-suite C:\Games\U5-Clean` — 163 Bevy-owned PNGs,
  every frame nonblank with a sanitized manifest, including all sixteen public
  `BRIT.CBT` outdoor arena gallery frames with accepted early replacement rolls,
  all one hundred twelve public `DUNGEON.CBT` dungeon-room terrain records with
  source scanning disabled, prompt/modal frames for world, town, dungeon,
  combat, and Talk, surface View class galleries for gem/Peer/X-Ray modes, plus
  combat status-highlight and death/field/cursor marker galleries.
- `cargo run -p u5-tui --features visual -- --visual-route-suite
  target\visual-route-suite C:\Games\U5-Clean` — 1780 Bevy-owned per-step route
  PNGs, every frame nonblank with a sanitized manifest, including all 40
  published stock world-location entry rows, TLK-backed reserved-word
  conversation routes across all 32 named-location scenes, exact
  TUI-label ship/castle/shop/dungeon/Doom/combat-field/terrain-exit aliases,
  light-decay, all nine public arms-shop first-stock purchases and terminator
  refusals, horse-trader purchases, accepted healer cure/heal/resurrect, all four public shipwright
  delivery-row purchases, spell routes for Locate, In Lor/Light/Open, restore, active effects,
  all-cardinal directed Sleep/Poison Wind/Death Wind/Flame Wind combat casts, combat utility fallback casts, targeted
  Magic Missile/Tremor/Repel Undead/Charm/Polymorph/Clone, Conjure/Swarm/Summon
  Daemon, dungeon levels, dungeon fields/dispel, dungeon Open chest, utility
  item use, Gate Travel success/refusal, natural moongate live-entry, chasm
  fall, forced whirlpool Underworld branch, fixed narrative gate open/ordained-block
  routes, H-Hole-up rest, save/refusal, castle dispatcher/workflow overlays,
  dungeon SJOG/refusal paths, TUI-parity world/town/dungeon movement/pass/look/view/status,
  Minoc daily fixed-hidden, hourly status/ring, native stair, dungeon
  rest/ladder/exit/search, active-monster ambush routes, fixed hidden-treasure, Blackthorn
  audience/rescue routes, debug-enter town/dungeon transitions, ship X-it/skiff
  and hoisted-sail movement routes, extended Britannia/castle/dungeon routes,
  Shadowlord town entry/Yell/Stonegate, all three native shard vanquish paths,
  public Word-of-Power seal opening paths, public #56 endgame class-tableau
  restoration, broad Doom combat command/pass routes covering digit
  selection, direct movement, command refusals/prompts, Ready, Yell, and X-it,
  and real-key Bevy keyboard routes for movement, pass, Ctrl-S music toggle,
  save refusal, conversation/shrine/shop line buffers, direction prompts, Yell
  text, Ready/Z-stats modal pickers, U-Use, M-Mix, H-Hole-up/Rest watch, New Order, Backspace, Enter, and prompt-safe Escape.
- `cargo fmt -- --check` passed, and `git diff --check` reported only
  CRLF-normalization warnings.

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
| §7 Per-round structure | `combat_driver.rs` round-walk classifier; `combat_frame.rs` post-round maintenance report | `chunk_23.rs` round-cycle and post-round maintenance tests | Implemented |
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
| §5 C-Cast | `play_state_impl/chunk_04.rs` cast handler, `magic.rs::cast_dispatcher_gate` | cast scene-gate tests in `chunk_17.rs`, `chunk_18.rs` | Implemented |
| §6 M-Mix | `play_state_impl/chunk_04.rs` mix handler; `magic.rs::SPELL_SELECTOR_IGNORED_LETTERS` (J/O) | mix recipe tests | Implemented |
| §7 Prerequisites | `magic.rs::cast_dispatcher_gate` (`CastGateOutcome`) | gate tests | Implemented |
| §8 Spell effects | per-spell handlers in `play_state_impl/chunk_*.rs`; field placement in `magic.rs::spell_field_placement_byte` | field-cast and restoration/status spell PRNG tests | Implemented (Heal uses the public shared-PRNG roll path; Create Food uses the latest public tiny PRNG grant; non-combat Blink follows public #48 directional ray-to-farthest-grass behavior) |
| §9 Casting in combat | `combat_frame.rs` cast dispatch; scene allow-mask | combat-cast tests in `chunk_23.rs` | Implemented |
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
| `intro.md` §1–§14 | `intro.rs`, `intro_menu.rs`, `menu_dispatch.rs`, `pth.rs` (BRITISH.PTH walker), `return_to_view.rs`, `story_io.rs`; Bevy intro shell composes published title bitmap, animates signature path, draws the four title-tick flame bands from `ULTIMA.16` records 1..=4 (the public issue #52 procedural flame stripe was withdrawn and its generator deleted), renders all 21 story slides with the spec-defined transition-strip and secondary-art draws plus step 6's published #69 doorway text, and reveals story step 1 and the STARTSC menu loader through the public issue #53 rectangle dissolve (the 36-tick and 320-tick column sweeps were withdrawn) before sampling menu input; Return-to-View preview rendering uses public title-tick animation families, transparent actor overlay composition, public #54 fixed strip captions from LoadMapStrip, high-opcode no-ops, 4x19 source strip loading, `(x, y + 7)` cell-effect coordinates, and scheduler/playback timing for preview ticks, cell effects, fixed wipes, fixed waits, trailing ticks, and one-shot actor draws; §11 (`A` submenu) is an artwork screen, not a text screen - `render_acknowledgements_intro_frame` draws the ULTIMA logo panel and the credits parchment at their published origins, and any key restores the menu. No credits text is authored; the earlier `ACKNOWLEDGEMENTS_LINES` clean-room-authored constant was removed. The terminal harness has no surface for it and still fails loudly through `intro.rs::require_acknowledgements_contract` | intro/chargen menu tests in `chunk_01.rs`, `chunk_02.rs`; Bevy intro framebuffer/title-tick/story-wipe tests; Bevy acknowledgements panel/restore tests; Return-to-View renderer/playback tests; `intro::tests::acknowledgements_contract_refuses_placeholder_lines` | Partial; §11's bottom-up entry and top-down exit slab wipes still need the published stride/cadence (cleak/u5-spec#72), and exact historical title-tick silhouette pixels and exact Return-to-View effect rasters remain Presentation work |
| `chargen.md` §1–§11 | `chargen.rs` questionnaire VM, gender prompt, virtue tournament, stat assignment | chargen tests | Implemented |
| `u4-transfer.md` §1–§10 | `u4_transfer.rs`, `u4_transfer_session.rs` state machine, public issue #16 `PARTY.SAV` source validation offsets, BRIT.GAM/BRIT.OOL handling, stat translation, OOL ordering | u4-transfer tests | Implemented |

### `systems/save-load.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§8 | `save_load.rs`, `disk_io.rs`, `active_object_io.rs`, `play_state_struct.rs` four-file contract (SAVED.GAM/SAVED.OOL/BRIT.OOL/UNDER.OOL), empty-save guard, mirror writes including load-time and save-time extra `UNDER.OOL` branches, read/write retry wrapper, original binary content/resource loader disk I/O, vehicle/transition save round-trips | save/load tests across `chunk_03.rs`, `chunk_04.rs`, `chunk_05.rs`, `chunk_07.rs`, `chunk_09.rs`, `chunk_11.rs`, `chunk_13.rs`, `chunk_23.rs` | Implemented |

### `systems/movement.md`, `systems/overworld.md`, `systems/town-mode.md`, `systems/dungeon-mode.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `movement.md` §1–§10 | `direction.rs`, `tile_classes.rs`, `predicates.rs`, `transport.rs`, `active_object_io.rs`; native static terrain predicates cover the published foot, horse, carpet, ship, and facing-sensitive skiff tile sets | per-mode movement tests plus exhaustive 0..=255 transport predicate tests | Implemented |
| `overworld.md` §1–§15 | `play_state_impl/chunk_01.rs` overworld loop, `world_tables.rs`, `moongate.rs`, `lord_british_camp.rs`, native and sidecar encounters, public Word-of-Power seal rows | world tests in chunks 03, 05, 06, 07, 10, 12, 13, 15, 17, 23 | Implemented |
| `town-mode.md` §1–§17 | `town_mode.rs`, `town_tables.rs`, `location_audit.rs`, NPC schedules, dawn/dusk substitution, alarms | town tests in chunks 04, 06, 10, 11, 15, 19, 21, 23, 24, including sanitized shipped `LOCATION.DAT` aggregate owner/class/view audits when local clean assets are present | Implemented (public #51 tile `0x04` poison-gas step behavior is native; coordinate and tile-attribute sidecars no longer trigger this branch) |
| `dungeon-mode.md` §1–§17 | `play_state_impl/chunk_*.rs` dungeon loop, `dungeon_tables.rs`, raster in `crates/u5-bevy/src/lib.rs` first-person draw | dungeon tests in chunks 05, 12, 13, 18, 20, 23 | Implemented |

### `systems/encounters.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§14 | `play_state_impl/chunk_*.rs` random spawn probe (native + sidecar), fortunes-of-war counter, sleep-ambush in `rest_camp.rs`, dungeon-room arena selection, public #21 dungeon active-monster ambush setup | encounter and ambush tests | Implemented |

### `systems/vehicles.md`, `systems/weather.md`, `systems/moons.md`, `systems/time.md`, `systems/rest-and-camp.md`, `systems/lighting.md`, `systems/doors-and-z-transitions.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `vehicles.md` §1–§11 | `transport.rs`, `play_state_impl/chunk_*.rs` Board/X-it/Yell sails, `ship_broadside.rs` Fire | vehicle save/load round-trip tests, broadside tests | Implemented |
| `weather.md` §1–§11 | `wind.rs`, Rel Hur cast in `magic.rs`, sail cadence | wind cast and sailing tests, including the two-wait into-wind case | Implemented |
| `moons.md` §1–§4 | `play_state_impl/chunk_*.rs` sky strip; moongate counters in `moongate.rs`; public issue #38 Felucca phase-0 glyph bytes for hours 10/11/19/20 | sky-strip and moon-glyph cache tests | Implemented |
| `time.md` §1–§13 | `clock.rs` cascade, Q/T tag handling, mode-specific increment | clock and cascade tests | Implemented |
| `rest-and-camp.md` §1–§10 | `rest_camp.rs`, `lord_british_camp.rs`, native town inn bed gate in `play_state_impl/chunk_07.rs`, `play_state_impl/chunk_08.rs::apply_completed_long_camp_recovery`, and hourly Ring of Regeneration tick in `chunk_09.rs` | rest, native inn bed/no-inn refusal, sidecar override, camp, ambush, long-camp recovery, and hourly ring tests | Implemented (ordinary rest has no direct HP/MP recovery; current checked-in spec matches public #47 issue-comment behavior) |
| `lighting.rs` §1–§11 | `lighting.rs` ambient + torch + light-spell counters | lighting tests | Implemented |
| `doors-and-z-transitions.md` §1–§15 | `jimmy.rs`, `play_state_impl/chunk_*.rs` open/get/look cascade, `ship_broadside.rs` BOOOM, secret doors, climb command | jimmy, open, secret-door, klimb tests | Implemented |

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
| `animation.md` | `animation.rs`; visual cadence in `u5-bevy` | animation tests | Implemented |
| `view.md` | `view_classes.rs`, V-View overlays in `play_state_impl/chunk_*.rs`; explicit View/Peer/X-Ray overlay modes and peer/gem alternate-bank raster branch; Bevy overlay draws | View overlay route-smoke (surface, dungeon, Spyglass chunk-map, Peer, X-Ray) | Implemented (exact remote-view pixels and chunk-map pixel parity — Presentation work) |
| `visibility.md` | `visibility.rs`, persistent radius-5 visibility grid / companion terrain band, light-mask wrap | persistent visibility-buffer and visibility tests | Implemented |

### `systems/text-output.md`, `systems/stats-panel.md`, `systems/display-driver.md`, `systems/display-driver-abi.md`, `systems/overlay-abi.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `text-output.md` §1–§7 | `text_wrap.rs` fixed-cell text window, control bytes `0xFB`..`0xFF`, padded numeric printer, descriptor cursors | text-wrap tests in `chunk_13.rs`, `chunk_14.rs` | Implemented |
| `stats-panel.md` §1–§5 | `stats_panel.rs` party rows, active-cursor handling, combat overlays | stats-panel tests | Implemented |
| `display-driver.md`, `display-driver-abi.md` | `crates/u5-bevy/src/lib.rs` framebuffer composition, atlas-backed top-down and first-person rasters, fixed-font shared surface, intro column-sweep and endgame certificate-rectangle rendering, and per-step visual route replay harness; Tandy CLI raster depth aliases route to the EGA-equivalent path while Hercules is explicitly rejected as outside v1 scope | Bevy framebuffer/story-wipe/STARTSC/endgame-transition tests; visual frame suite; visual route suite; CLI display-depth tests | Implemented with public story/menu-loader timing; exact late endgame rectangle primitive/cadence and Return-to-View effect rasters remain presentation work |
| `overlay-abi.md` | `crates/u5-bevy/src/lib.rs` overlay composition for status/Z-stats/endgame/intro | Bevy overlay tests | Implemented |

### `systems/prng.md`, `systems/timing.md`, `systems/stat-arithmetic.md`, `systems/active-objects.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `prng.md` | `prng.rs` LCG; `random_*` helpers in `play_state_*.rs` | rng round-trip tests | Implemented |
| `timing.md` | `timing.rs` wait counters; integrated in clock and sailing cadence | timing tests | Implemented |
| `stat-arithmetic.md` | `stat_arithmetic.rs` saturating add/sub | stat-arith tests | Implemented |
| `active-objects.md` | `active_object_io.rs` 32-slot table, animator, OOL persistence, `ool_audit.rs` aggregate active-object overlay census | active-object tests across chunks plus synthetic and local-clean `.OOL` aggregate audit coverage | Implemented |

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

There are none. Every issue in `cleak/u5-spec` is closed, including the six
this engine filed (`#78` intro/menu, `#79` gameplay chrome, `#80` per-scene
floor pages, `#81` command echoes and dungeon tables, `#82` endgame/chargen/
`PARTY.SAV`, `#83` light-byte semantics). What is left is engine work against
published contracts, and a small number of details the spec states honestly
that it does not publish.

### The last gate on an unpublished contract is down

The endgame certificate wording was the final one. `endgame.md §9.1`-`§9.5`
published it, so `endgame_certificate_lines` now builds the screen and **the
victory ending is reachable and rendering end to end** - rite beats, tableau
exit, the `§7.1` fade to black, six `END.DAT` windows, the certificate on its
parchment, the elapsed-time report, and the `§9.5` terminal hold. Evidence:
`route-endgame-box-full-victory-cinematic-29-empty.png` in the visual route
suite, and route-smoke's validator requires `cinematic_is_finished()` so the
case fails if the ending stops short.

### Panics that remain, and what each one is

Six `panic!` sites still cite a spec issue. Only one of them is an
unimplemented contract; the rest are structural. Derived by grepping
`crates/` rather than from any summary:

| Site | Kind |
|---|---|
| `u5-bevy` `require_published_u4_transfer_preview_presentation` | **Unimplemented published contract.** `#73` is closed and `u4-transfer.md §6.1`-`§6.6` publish the per-field cursor cells, the label strip, the "Found:" summary page, the stage machine and the finish. The graphical preview has not been built against them yet. This is the one real remaining implementation gap in this area. |
| `u5-tui` terminal story / Return-to-View / transfer preview | No terminal surface. These are graphical screens; `--intro` refuses rather than printing a diagnostic substitute. Not spec gaps. |
| `u5-runtime` `require_acknowledgements_contract` | Same: the graphical path draws the credits artwork, the terminal harness cannot. |
| `u5-runtime` display title-tick operation | An injection guard - the caller must supply the `ULTIMA` bands rather than generated clean-room frames. Not a gap. |

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
  as the single blocking call `#53` specifies rather than pacing it.
- The alternate-depth (`.4`) conversion of the archives named in `#82`.

### Engine work still outstanding

- **Dungeon first-person wall tables** (`#81` item 5). The corridor renders as
  an untextured wireframe; no wall/scenery table exists in `crates/`. Whether
  the tables were published is being checked.
- `#42`'s local-light mask cadence, and the night-time beacon gate
  (`visibility.md §12.6`).
- The decomp side has a cross-document contradiction sweep in flight that may
  yet touch contracts already implemented here.

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

Verified on 2026-08-22 at `d4fc579`: 2815 u5-runtime, 155 u5-bevy, 96 u5-tui
tests pass, `cargo fmt --all -- --check` is clean, `--route-smoke` passes all
cases, `--visual-frame-suite` writes 187 PNGs and `--visual-route-suite` writes
1814. Every asset-backed run used a copy of the asset directory.
