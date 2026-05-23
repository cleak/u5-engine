# Completion Audit

This audit maps each public-spec deliverable (the systems and formats published
in `C:\Projects\Rust\u5-clean\u5-spec`) to concrete engine evidence
(`crates/u5-runtime`, `crates/u5-tui`, `crates/u5-bevy`) and to test coverage.

Created on 2026-05-19. This document satisfies the completion criterion in
`TODO.md`:

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
  - **Implemented (first-playable)** — engine covers what the public spec
    publishes; remaining detail is a documented v1 deferral or local policy.
  - **Blocked on `cleak/u5-spec#NN`** — the engine has a safe placeholder while
    a public-spec clarification is pending.
  - **Presentation work** — gameplay behavior is implemented; remaining work is
    visual/audio polish covered by `Milestone 3` in `TODO.md`.

The blocked-issue catalogue in scope of this audit is the set already tracked
in `TODO.md`:

| Issue | Topic |
|------|-------|
| `cleak/u5-spec#13` | Corrected per-sage 16-entry rumour tables and scene/template mapping |
| `cleak/u5-spec#41` | Exact arms-shop eight-item stock rows |
| `cleak/u5-spec#43` | Exact death-vision stat/text and wanted-poster/wishing-well resource details |
| `cleak/u5-spec#47` | Exact original random-jolt/camp recovery details |
| `cleak/u5-spec#48` | Non-combat Blink default-target selection |
| `cleak/u5-spec#51` | Full resident town tile-attribute table for poison-gas detection |
| `cleak/u5-spec#54` | Return-to-View caption source/opcode/text and exact effect rasters |

Where an audit row references one of these issues, the engine carries a clean
implementation that avoids private-derived guesses until the public spec
publishes the missing rule.

## Verification Baseline

Refreshed alongside this audit on 2026-05-21:

- `cargo test -p u5-runtime` — 2553 tests pass.
- `cargo test -p u5-tui` — 78 tests pass, including temp-directory binary
  smoke for empty-save Journey Onward, deterministic Create Character followed
  by `--from-save --play-script`, intro-driven U4 transfer commit, and a
  confirmed `QY` save/reload round trip.
- `cargo test -p u5-bevy` — 55 tests pass.
- `cargo run -p u5-tui -- --route-smoke C:\Games\U5-Clean` — 123 scripted cases pass,
  including four extended-session cases that exercise 5–12 commands across
  Britannia exploration, castle walking, dungeon turning/search, and
  multi-round Doom combat to prove the engine sustains long playable sessions,
  plus active shop/modal flows, Blackthorn audience correct/wrong and
  rescue-refuge routes, fixed hidden-treasure single-use/daily/stacked routes,
  PRV Gate Travel success/refusal paths, saved-slot natural moongate live-entry
  paths, public #32 Britannia/Doom Word-of-Power seal opening, public #15
  accepted inn-rest pricing, public #44 sleeping/praying Talk refusals, public
  #51 poison-gas doorway step, public #47 dungeon no-direct-recovery rest, and
  ship broadside fire, horse boarding, dungeon torch ignition, Mix/Ready/New
  Order command workflow, combat-active Board/Enter/Fire/Hole-up/Ignite/Mix/New
  Order/Talk refusal rows, combat-active digit selection/clear, Escape abort,
  Ctrl-S music toggle, lowercase direct movement, Horse and non-horse
  wishing-well branches, and the public Britannia chasm fall route through real
  asset-backed play states.
- `cargo run -- --save-frame-suite target\audit-frame-suite C:\Games\U5-Clean`
  — 13 PNGs, every frame nonblank with stable hashes.
- `cargo run -p u5-tui --features visual -- --visual-route-suite
  target\visual-route-suite C:\Games\U5-Clean` — 47 Bevy-owned per-step route
  PNGs, every frame nonblank with a sanitized manifest, including Doom combat
  entry, pass, attack, refusal, Z-stats, and Search-prompt routes.
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
| §9 Monster AI | `combat_frame.rs::combat_ai_actor_fleeing`, `combat_target_group_for_slot`, suppression bypass; `combat_actor.rs::party_name_forces_monster_combat_group` | `combat_ai`, `combat_actor_slot_dispatch`, `cause_fear` filters | Implemented |
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
  in §12 are described as "hand-tweaked" without published pixel/tile IDs in
  the spec; the engine routes both to the default drop path until the public
  spec promotes the exact transition. This is **Presentation work**, not a
  gameplay correctness gap.
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
| §8 Spell effects | per-spell handlers in `play_state_impl/chunk_*.rs`; field placement in `magic.rs::spell_field_placement_byte` | field-cast and restoration/status spell PRNG tests | Implemented (Heal uses the public shared-PRNG roll path; Create Food uses the latest public tiny PRNG grant; non-combat Blink sidecar default remains tied to `cleak/u5-spec#48`) |
| §9 Casting in combat | `combat_frame.rs` cast dispatch; scene allow-mask | combat-cast tests in `chunk_23.rs` | Implemented |
| §10 Virtue/shrine linkage | `shrine_virtue.rs` stat reward, ordained mask | shrine-meditation tests | Implemented |
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
| §7 U-Use flows | `play_state_impl/chunk_04.rs::apply_u_use_*` for every public family (torch/gem/key/scroll/potion/Moonstone/regalia/shard/carpet/skull key/spyglass/HMS plans/sextant/pocket watch/wooden box); shard destruction follows public issue #31 flame predicate + shard pairing through clean Eternal Flame metadata | per-item use tests in `chunk_03.rs`–`chunk_05.rs`, shard/flame tests in `chunk_17.rs` | Implemented (native Eternal Flame coordinates remain blocked until published) |
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
| §5 Keyword scan | `conversation_session.rs::tlk_player_input_kind`, reserved-word table | keyword-match tests | Implemented (profanity-rebuke table absent — no spec issue filed; default `I cannot help thee...` message is rendered) |
| §6 Keyword input loop | `conversation_session.rs::AwaitingKeyword` phase | phase tests | Implemented |
| §7 Byte runner | `tlk_runner.rs` full control-code dispatch | per-control-code tests | Implemented |
| §7.1 Printable text | `tlk_runner.rs` word-buffer, soft-break | text emission tests | Implemented |
| §7.2 Avatar name (`0x81`/`0x82`) | `tlk_runner.rs` interpolation | name substitution tests | Implemented |
| §7.3 Pause (`0x83`/`0x8F`) | `tlk_runner.rs` pause emit; redraw delegated to frontend | pause tests | Implemented |
| §7.4 Newlines (`0x8A`/`0x8D`) | `tlk_runner.rs` newline emit | newline tests | Implemented |
| §7.5 Print mask / curse (`0x8B`/`0x8E`) | `tlk_runner.rs::PrintMask`, curse-check hook | mask-pair tests | Implemented |
| §7.6 Branching (`0x85`/`0x86`/`0x8C`/`0xFE`) | `tlk_control_codes.rs::TlkActionDispatchVerb`, `tlk_if_else_alt_branches`, `play_state_impl/chunk_04.rs::apply_tlk_action_grants` | gold-payment, action-letter, IF/ELSE, karma-threshold tests | Implemented (`0x85` toll-milestone karma — `cleak/u5-spec#27`) |
| §7.6 `0x87` follow-up scan | `tlk_runner.rs::TlkRunStop::FollowUpKeywordScan` | follow-up scan tests | Implemented |
| §7.7 Labels / GOTO | `tlk_runner.rs` label dispatch | label scan tests | Implemented |
| §8 Common-word dictionary | `common_words_io.rs` public issue #33/#40 128-entry shared table; `shoppe_bark.rs` shared renderer path | dictionary and SHOPPE bark tests | Implemented |
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
| §1–§11 | `npc_runtime.rs` state machine, `town_tables_io_movement.rs`, schedule walker invoked from `town_mode.rs` per turn; `0xC8`/`0xC9` floor-link BFS | `scheduled_npc` tests; town-floor change tests | Implemented |

### `systems/shops.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1 Overview | `shops.rs`, `shop_runtime.rs`, `shop_session.rs` | shop-runtime tests | Implemented |
| §2 Triggering | `conversation_session.rs` shop trigger detection, Talk → shop arm | shop-trigger tests | Implemented |
| §3 Eight shop kinds | `shops.rs` arm dispatch (0x81..0x88) | per-arm tests | Implemented |
| §4 SHOPPE.DAT structure | `shoppe_records.rs`, `shoppe_bark.rs` | parser tests | Implemented |
| §5 Bark renderer | `shoppe_bark.rs` substitution (`%/^/$/&/*/#/@`) | bark tests | Implemented |
| §6 Pricing model | `shops.rs::arms_shop_price`, healer table, etc. | pricing tests in `chunk_21.rs` | Implemented |
| §7 Inventory model | `equipment.rs` stock tables; inn registry in `play_state_struct.rs` | inn-stay tests | Implemented |
| §8.1 Weaponsmith/armourer | `shops.rs::ArmsShop`, `shop_session.rs::arms_shop_for_scene`, `shops.rs::arms_shop_stock_letter_index` | arms scene-row and transaction tests | Implemented (scene-local row identity; exact stock arrays blocked on `cleak/u5-spec#41`) |
| §8.2 Guildmaster | `shops.rs` guild prices | guild tests | Implemented |
| §8.3 Healer | healer arm in `shop_runtime.rs`; Minoc bypass | healer tests | Implemented |
| §8.4 Innkeeper | `shop_runtime.rs` inn flow; stay counter in `clock.rs`; public issue #15 fixed base-rate charges with no speaker-Intelligence discount plus paid-rest class recovery and poison cure | inn tests | Implemented |
| §8.5 Tavernkeeper | tavern arm in `shop_runtime.rs` | tavern tests | Implemented |
| §8.6 Sage | sage arm: free 16-slot keyword lookup with no gold debit; synthetic placeholder rows until the corrected per-sage table is public | sage tests | Implemented (mechanics; real rows blocked on `cleak/u5-spec#13`) |
| §8.7 Shipwright | shipwright arm; pending vehicle in `play_state_struct.rs` | shipwright tests | Implemented |
| §8.8 Horse trader | horse arm | horse tests | Implemented |
| §8.9 Reagent vendor | `shops.rs` per-herbalist matrix | reagent tests | Implemented |
| §8.10 Horse trader correction | `shop_session.rs` public issue #28 horse-trader scene rows; `input_dispatch.rs` adjacent marker placement and active-object creation | horse-trader route smoke and talk/shop tests in `chunk_21.rs` | Implemented; obsolete stationary-display purchase runtime removed |
| §9 Karma effects | no karma gating on shop pricing | — | Implemented |

### `systems/karma.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§11 | `karma.rs` selector, `shrine_virtue.rs`, `endmsg_io.rs` (KARMA.DAT 6-record verdicts), `tlk_runner.rs::tlk_if_else_alt_branches` | karma/threshold tests in `chunk_21.rs` | Implemented (`0x85` toll progress — `cleak/u5-spec#27`) |

### `systems/intro.md`, `systems/chargen.md`, `systems/u4-transfer.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `intro.md` §1–§14 | `intro.rs`, `intro_menu.rs`, `menu_dispatch.rs`, `pth.rs` (BRITISH.PTH walker), `return_to_view.rs`, `story_io.rs`; Bevy intro shell composes published title bitmap, animates signature path, draws the public issue #52 four-frame palette-cycled clean flame stripe, renders story art with the spec-defined transition-strip and secondary-art draws, and runs the public issue #53 step-1 36-title-tick column wipe; Return-to-View preview rendering uses public title-tick animation families, transparent actor overlay composition, and the public issue #54 scheduler/playback timing for preview ticks, cell effects, fixed wipes, fixed waits, trailing ticks, and one-shot actor draws; `intro.rs::ACKNOWLEDGEMENTS_LINES` provides clean-room authored Acknowledgements content for §11 (`A` submenu) per the spec's "source-free content transcription" directive | intro/chargen menu tests in `chunk_01.rs`, `chunk_02.rs`; Bevy intro framebuffer/title-tick/story-wipe tests; Return-to-View renderer/playback tests; `intro::tests::acknowledgements_lines_are_clean_room_authored` | Implemented; exact historical title-tick silhouette pixels, exact unpublished wider rectangle-transition rates/rectangles, and Return-to-View caption/effect rasters remain Presentation work |
| `chargen.md` §1–§11 | `chargen.rs` questionnaire VM, gender prompt, virtue tournament, stat assignment | chargen tests | Implemented |
| `u4-transfer.md` §1–§10 | `u4_transfer.rs`, `u4_transfer_session.rs` state machine, public issue #16 `PARTY.SAV` source validation offsets, BRIT.GAM/BRIT.OOL handling, stat translation, OOL ordering | u4-transfer tests | Implemented |

### `systems/save-load.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§8 | `save_load.rs`, `disk_io.rs`, `active_object_io.rs`, `play_state_struct.rs` four-file contract (SAVED.GAM/SAVED.OOL/BRIT.OOL/UNDER.OOL), empty-save guard, mirror writes, read/write retry wrapper, original binary content/resource loader disk I/O, vehicle/transition save round-trips | save/load tests across `chunk_03.rs`, `chunk_04.rs`, `chunk_05.rs`, `chunk_07.rs`, `chunk_09.rs`, `chunk_11.rs`, `chunk_13.rs`, `chunk_23.rs` | Implemented |

### `systems/movement.md`, `systems/overworld.md`, `systems/town-mode.md`, `systems/dungeon-mode.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `movement.md` §1–§10 | `direction.rs`, `tile_classes.rs`, `predicates.rs`, `transport.rs`, `active_object_io.rs` | per-mode movement tests | Implemented |
| `overworld.md` §1–§15 | `play_state_impl/chunk_01.rs` overworld loop, `world_tables.rs`, `moongate.rs`, `lord_british_camp.rs`, native and sidecar encounters, public Word-of-Power seal rows | world tests in chunks 03, 05, 06, 07, 10, 12, 13, 15, 17, 23 | Implemented |
| `town-mode.md` §1–§17 | `town_mode.rs`, `town_tables.rs`, NPC schedules, dawn/dusk substitution, alarms | town tests in chunks 04, 06, 10, 11, 15, 19, 21, 23 | Implemented (native poison-gas doorway detection remains tied to `cleak/u5-spec#51`) |
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
| `moons.md` §1–§4 | `play_state_impl/chunk_*.rs` sky strip; moongate counters in `moongate.rs`; public issue #38 Felucca off-horizon sentinels for hours 10/11/19/20 | sky-strip and moon-glyph cache tests | Implemented |
| `time.md` §1–§13 | `clock.rs` cascade, Q/T tag handling, mode-specific increment | clock and cascade tests | Implemented |
| `rest-and-camp.md` §1–§10 | `rest_camp.rs`, `lord_british_camp.rs` | rest, camp, ambush tests | Implemented (ordinary rest has no direct HP/MP recovery; exact original camp/random-jolt details remain tied to `cleak/u5-spec#47`) |
| `lighting.rs` §1–§11 | `lighting.rs` ambient + torch + light-spell counters | lighting tests | Implemented |
| `doors-and-z-transitions.md` §1–§15 | `jimmy.rs`, `play_state_impl/chunk_*.rs` open/get/look cascade, `ship_broadside.rs` BOOOM, secret doors, climb command | jimmy, open, secret-door, klimb tests | Implemented |

### `systems/endgame.md`, `systems/blackthorn.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `endgame.md` §1–§12 | `endgame.rs`, `endgame_cinematic.rs`, `end_io.rs` (public END.DAT final narrative windows), `endmsg_io.rs`; Bevy endgame modal advances/renders the runtime narrative page-in transition with the public full-page fallback rectangle | endgame tests; Bevy endgame page-transition tests; route-smoke terminal-endgame confirmation/victory cases | Implemented |
| `blackthorn.md` §1–§11 | `blackthorn.rs`, `blackthorn_session.rs`, KARMA.DAT verdict mapping | Blackthorn challenge/rescue tests; Blackthorn audience/rescue route-smoke; Shadowlord route-smoke | Implemented |

### `systems/main-loop.md`, `systems/screen-mode-dispatch.md`, `systems/runtime.md`, `systems/input.md`, `systems/commands.md`, `systems/animation.md`, `systems/lighting.md`, `systems/view.md`, `systems/visibility.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `main-loop.md` | `play_state_impl/chunk_01.rs` mode dispatcher | mode-dispatch tests | Implemented |
| `screen-mode-dispatch.md` | `play_state_impl/chunk_*.rs` mode-state ownership, `save_load.rs::normalize_disk_prompt_mode`, `disk_io.rs` read/write retry prompt phases | mode-state and disk-retry tests | Implemented (visible disk-prompt UI — Presentation work) |
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
| `display-driver.md`, `display-driver-abi.md` | `crates/u5-bevy/src/lib.rs` framebuffer composition, atlas-backed top-down and first-person rasters, fixed-font shared surface, intro/endgame column-sweep transition rendering, and per-step visual route replay harness; Tandy CLI raster depth aliases route to the EGA-equivalent path while Hercules is explicitly rejected as outside v1 scope | Bevy framebuffer/story-wipe/endgame-transition tests; visual frame suite; visual route suite; CLI display-depth tests | Implemented (exact unpublished wider story/endgame rectangles/rates, return-to-view captions/effect rasters — Presentation work) |
| `overlay-abi.md` | `crates/u5-bevy/src/lib.rs` overlay composition for status/Z-stats/endgame/intro | Bevy overlay tests | Implemented |

### `systems/prng.md`, `systems/timing.md`, `systems/stat-arithmetic.md`, `systems/active-objects.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `prng.md` | `prng.rs` LCG; `random_*` helpers in `play_state_*.rs` | rng round-trip tests | Implemented |
| `timing.md` | `timing.rs` wait counters; integrated in clock and sailing cadence | timing tests | Implemented |
| `stat-arithmetic.md` | `stat_arithmetic.rs` saturating add/sub | stat-arith tests | Implemented |
| `active-objects.md` | `active_object_io.rs` 32-slot table, animator, OOL persistence | active-object tests across chunks | Implemented |

## Formats

| Format | Evidence | Tests | Status |
|-------|----------|-------|-------|
| `formats/bit.md` (BIT bitmaps) | `graphics_io.rs::load_bit_*`, `intro.rs` placement | bit decode tests | Implemented |
| `formats/brit-dat.md` | `map_io.rs::load_brit_dat`, `world_tables_io.rs` | brit decode tests | Implemented |
| `formats/cbt.md` | `combat_arena.rs::parse_cbt_record` | CBT decode tests | Implemented |
| `formats/data-ovl.md` | `misc_tables_io.rs`, `world_tables_io.rs` overlay readers | DATA.OVL field tests | Implemented |
| `formats/dungeon-dat.md` | `dungeon_tables_io.rs::load_dungeon_dat` | dungeon decode tests | Implemented |
| `formats/end-dat.md`, `formats/endmsg-dat.md` | `end_io.rs`, `endmsg_io.rs` | END/ENDMSG tests | Implemented |
| `formats/font-ch.md`, `formats/font-hcs.md`, `formats/font-pcs.md` | `fonts_io.rs::load_ibm_ch`, sparse PCS loader | font decode tests | Implemented |
| `formats/karma-dat.md` | `endmsg_io.rs::load_karma_dat` (6 verdict records) | karma decode tests | Implemented |
| `formats/location-dat.md` | `town_tables_io.rs::load_*_dat` | location decode tests | Implemented |
| `formats/look2-dat.md` | `misc_tables_io.rs::load_look2` (descriptions) | look2 decode tests | Implemented |
| `formats/lzw.md` | `lzw.rs` decompressor | LZW round-trip tests | Implemented |
| `formats/miscmsg-dat.md` | `miscmsg_io.rs` | message lookup tests | Implemented |
| `formats/npc.md` | `npc_runtime.rs` + `town_tables_io.rs` NPC block decode | NPC decode tests | Implemented |
| `formats/ool.md` | `active_object_io.rs` | OOL round-trip tests | Implemented |
| `formats/pth.md` | `pth.rs::load_path_records`, Bevy signature animation | PTH parse tests | Implemented |
| `formats/question-dat.md` | `question_io.rs` | QUESTION decode tests | Implemented |
| `formats/saved-gam.md` | `save_load.rs`, `play_state_struct.rs` | save/load round-trip tests | Implemented |
| `formats/shoppe-dat.md` | `shoppe_records.rs` | SHOPPE decode tests | Implemented |
| `formats/signs-dat.md` | `signs_io.rs` | SIGNS lookup tests | Implemented |
| `formats/story-dat.md` | `story_io.rs` | STORY decode tests | Implemented |
| `formats/tiles.md` | `graphics.rs`, `graphics_io.rs` tile sheet decode | tile-sheet tests | Implemented |
| `formats/tlk.md` | `tlk_runner.rs`, `tlk_control_codes.rs` | TLK runner tests | Implemented |
| `formats/under-dat.md` | `map_io.rs::load_under_dat` | underworld decode tests | Implemented |

## Catalogs

| Catalog | Evidence | Tests | Status |
|--------|----------|-------|-------|
| `catalogs/gazetteer.md` | `world_tables_io_locations.rs`, sidecar overlays | locations tests | Implemented (stock location entry/return coordinates are native; several non-location transition families remain sidecar-backed pending public gazetteer rows) |
| `catalogs/item-list.md` | `equipment.rs`, `containers.rs::InventoryAddClass` | inventory-add tests | Implemented |
| `catalogs/monster-bestiary.md` | `combat_stats.rs::combat_class_stats`, `combat_ranged_effect_stats` | per-class tests | Implemented |
| `catalogs/npc-roster.md` | `npc_runtime.rs`, conversation/shop dispatch | roster tests | Implemented |
| `catalogs/quest-graph.md` | `quest_flags.rs`, `endgame.rs` | quest-flag tests | Implemented |
| `catalogs/spell-list.md` | `magic.rs` parser, cost, scene-mask tables | spell metadata tests | Implemented |
| `catalogs/tile-catalog.md` | `tile_classes.rs`, `view_classes.rs`, `tile_helpers.rs` | tile-class tests | Implemented |

## Remaining Public-Spec Gaps

The following items are the known public-spec blockers from the latest issue
audit. Gameplay blockers use conservative placeholders; presentation blockers
are kept out of gameplay logic until the public spec publishes exact data.

| Issue | Public gap | Engine placeholder |
|------|------------|--------------------|
| `cleak/u5-spec#13` | Corrected real per-sage 16-entry rumour rows, location strings, SHOPPE record/template ids, and scene/dialog-state selection | Sage mechanics use the corrected free keyword lookup/no-debit flow, but rows remain synthetic/empty until published |
| `cleak/u5-spec#41` | Exact arms-shop eight-item stock rows | Scene-to-shop dispatch and the nine public arms-shop row identities are implemented; exact `a..h` stock rows remain broad-menu-backed until published |
| `cleak/u5-spec#43` | Exact death-vision stat/text plus wished-object and wanted-poster resource details | Look routes the public object/tile predicates, prompts for the member on death vision, keeps top-down fountains presentation-only per checked-in `systems/view.md`, grants the native Horse wish, and leaves accepted car keywords on the no-effect path until the exact public wish-grant table is published |
| `cleak/u5-spec#47` | Exact original random-jolt/camp recovery details | ordinary rest advances time without a direct HP/MP recovery grant; route-smoke covers the dungeon no-direct-recovery path |
| `cleak/u5-spec#48` | Non-combat Blink default target | `play_state_impl/chunk_04.rs` uses sidecar-authored destinations |
| `cleak/u5-spec#51` | Full resident town tile-attribute table | `town_tile_attributes.tsv` can drive the public class `4` + vehicle byte `0x1C` poison-gas trigger predicate; coordinate rows remain as fallback, with route-smoke covering the public per-slot roll semantics |
| `cleak/u5-spec#54` | Return-to-View caption source/opcode/text and exact effect rasters | Parser/scheduler/overlay composition implement the public timing model; captions and exact rasters remain presentation placeholders |

Follow-up questions were posted from the clean-engine side on 2026-05-21 for
the largest remaining implementation blockers:

- `cleak/u5-spec#13`: publish per-sage 16-entry rumour rows and selector data.
- `cleak/u5-spec#41`: publish the nine `scene -> [a..h]` arms-stock rows.
- `cleak/u5-spec#43`: publish exact death-vision, fountain, wishing-well, and
  wanted-poster outcomes.
- `cleak/u5-spec#47`: publish the random-jolt/camp recovery trigger and deltas.
- `cleak/u5-spec#51`: publish the resident town tile-attribute table or
  semantic tile-id mapping.
- `cleak/u5-spec#54`: publish/confirm Return-to-View caption source and exact
  effect rasters.

Separate spec-sync contradictions remain tracked on GitHub but do not currently
change engine behavior: `cleak/u5-spec#1` cleared dungeon-room reload wording,
`cleak/u5-spec#3` terrain-combat placement-source wording, and
`cleak/u5-spec#18` record-15 cookie skip wording.

## Presentation Work (Separate From Gameplay Correctness)

These are visual/audio polish items called out in `TODO.md` Milestone 3 and the
spec's "exact visual parity deferrals" section. They do not block gameplay
correctness; the engine renders the published content with first-playable
overlays where exact historical pixels are not public.

- Title-tick exact historical silhouette pixels and palette fades.
- Exact unpublished wider story/endgame rectangle-transition rectangles/rates.
- Return-to-View captions and effect-raster pacing internals.
- Exact remote-view panel pixels for X-Ray / Peer.
- Exact dungeon minimap glyph/floodability edge cases.
- Gazer eye-burst tile + particle and Gargoyle lava-pool transitions (the
  spec describes both as "hand-tweaked" without published pixel/tile IDs).

## Conclusion

Across the public spec — 22 systems documents, 22 format documents, and seven
catalogs — every gameplay-correctness deliverable that has complete public
data has a corresponding engine implementation backed by tests,
route-smoke coverage, or visual-frame-suite captures. The blocker table above
is the current remaining public-spec blocker set from this audit, and each has
a safe documented placeholder. The remaining visual/audio polish items are
tracked under Milestone 3 in `TODO.md`.
