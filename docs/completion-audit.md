# Completion Audit

This audit maps each public-spec deliverable (the systems and formats published
in `C:\Projects\Rust\u5-clean\u5-spec`) to concrete engine evidence
(`crates/u5-runtime`, `crates/u5-tui`, `crates/u5-bevy`) and to test coverage.

Created on 2026-05-19; last refreshed on 2026-08-24 at public spec head
`60ac944`, after reconciling the answered public issue queue through `#131`,
including exact monster-special dispatch, the settled local-View contract, and
the shared combat-resistance and distinct target-weight predicates.

**Read spec contracts from GitHub, not from the local checkout** — the issues,
and document text through
`gh api -H "Accept: application/vnd.github.raw" repos/cleak/u5-spec/contents/<path>`.
`C:\Projects\Rust\u5-clean\u5-spec` is read-only from this workspace and is
stale at `9a898d1`, many commits and several retractions behind spec head
(notably `#42` local light, `#53` reveal transitions, `#54` Return-to-View,
`#65`/`#66`/`#67` title sequence, `#69` doorway text, `#70` font metrics,
`#80` floor pages, `#82` endgame/chargen, and `#85`-`#111`). Public `#111`
closed in `dac1f31`; its save-backed combat Cast interference map and exact
writer/revalidation/clear lifecycle are implemented. Public `#110` closed in
`3118380`; its fixed `(31,31)` edge sample, true candidate occupancy probe, and
Y/N/Escape turn accounting are implemented. Public `#109`
closed in `574f1d8`; its exact destructive town alarm and resident-Shadowlord
NPC sweep contract is implemented. Public `#108`
closed in `06494e0`; its host-clock seed equation and gameplay caller timing are
implemented. Public `#107` closed in `bc0c761` and confirms
the allocator's exact wrapped inclusive ±5, player-global, floor-independent
screen predicate. Public `#106` closed in `b34ae69`; its two Blackthorn rescue calls
and one shared three-outcome dungeon Search dissolve tail are implemented.
Public `#103` closed
in `a4167b0`; the exact
generic-adjacent impact gate, class and arena selectors, and high-to-low
continuation across returning combat are implemented alongside the Sand Trap,
whirlpool, ranged-reaction, and prune paths.

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
| `cleak/u5-spec#13` | Tavern/meal/sage selector table plus paid shared 26-row sage rumour topic table and success templates | Table/mechanics implemented, including per-tavern selector letters, explicit Anything-else Y/N continuation, state list/follow-up records, random provision quote selection, Intelligence-adjusted 25-food packs, exact partial/cap/charity exits and surcharge gating, lore continuation gating, SHOPPE.DAT fee quote/short-funds records, post-debit success-record RNG timing, and success rendering |
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
| `cleak/u5-spec#56`/`#82` | Endgame tableau active-object layout, restoration text, sprite mapping, and movement timing | Implemented from the latest public contracts: MISCMAPS record 3, active-object slots, class sprites, scene marker, branch movement, and `0x44`-gated refusal jitter follow the published contract. Entry now processes members in slot order instead of restoring and stacking everyone up front: each Dead member emits exact `LF + name + " lives!" + LF`, restores status/HP, then is placed and walked fully to target before the next slot |
| `cleak/u5-spec#57` | `.NPC` slot-zero sentinel byte policy | Runtime scheduling skips slot zero regardless of stored bytes; validators do not reject a nonzero slot-zero type/tag byte |
| `cleak/u5-spec#58` | Conversation reserved rebuke keyword table | Implemented from latest issue answer: all 34 reserved entries are active, including the 29 rebuke words, space-boundary matching, fixed rebuke text, pause limit, and return-to-prompt behavior |
| `cleak/u5-spec#59` | Overworld fall/transition and damage rules | Implemented: fixed chasm/falls preserves transport, applies Dex-gated one-HP checks, and ignores retired `world_waterfalls.tsv` current-sweep rows at runtime |
| `cleak/u5-spec#60` | Look/View overlay pixel renderer tables, telescope sky, and dungeon minimap exact glyph/flood presentation | Implemented from current `systems/view.md`, including the corrected day/night telescope split, exact 80-star PRNG capture, calendar-driven body rings, Shadowlord-marker geometry, per-display colour slots, and dungeon minimap presentation. The withdrawn Britannia chunk-map model is deleted |
| `cleak/u5-spec#61` | Town free-roaming active-object walker exact rules | Implemented from the latest answer: byte/floor eligibility, 50% gate, four-neighbor `0xA2`/`0x43` blocker gate, query-`0x10` destination classifier, occupancy checks, X-facing writes, Y-facing preservation, and visibility dirties on success |
| `cleak/u5-spec#62` | Live shop-dialogue record selection and window pacing | Closed and implemented to the published boundary: shared `SHOPPE.DAT` selection timing, Talk-to-shop inherited window-2 handoff, per-state transcript/wait/clear behavior, inn Pickup and arms Sell window-1 panels, and resident literal pools. The issue's stated residuals are presentation provenance boundaries, not pending questions |
| `cleak/u5-spec#72` | Acknowledgements screen asset and presentation | Implemented (`6db6135`). `§11.1` settles the asset as three `STARTSC` records with every credit line drawn into the bitmap; `§11.2` settles the presentation as compose / rise / part / keypress / close / sink, and **withdraws in full** the "bottom-up entry wipe, top-down exit wipe with horizontal slabs" model this engine carried. Nothing typesets the credits, so there is no text to author |
| `cleak/u5-spec#73` | Ultima IV transfer preview screen | Implemented (`f3ecfd1`). `§6.1`-`§6.6` publish the window rectangles, prompt-frame cells, both panel geometries, the eight-row field-label column, the pages, the stage machine and the finish. `§6` has no double buffering and no page swap, so the screen is one persistent surface edited in place. `§6.4`'s insert-disk block is dead code in the shipped build and is not drawn |
| `cleak/u5-spec#84` | Dungeon billboard slot-to-role mapping | Implemented; the corridor draws all seven published role families from the flavour-selected billboard bank using `slot = family_base + band`. Closed in `9807eb4`, which removes the retracted numeric pixel-ratio discriminator |
| `cleak/u5-spec#85` | Moongate transit animation distinct from the static gate tile | Answered. Gate *presence* is the `§9.1` sixteen-step composed-frame model and is implemented; the `§9.2` blocking transit presentation is implemented too (`8d41816`) — a 256-step dissolve reusing the shared `DissolveVisitOrder` primitive, then a 15→1 countdown at two BIOS ticks per phase |
| `cleak/u5-spec#86` | Retract `combat.md §7`'s post-round maintenance pass | Retracted upstream and removed here (`60ec07c`) |
| `cleak/u5-spec#87` | Clean-room: `animation.md` provenance cited private paths | Resolved on the spec side. No engine change; recorded because a correction pass introduced the breach, which is the failure mode `docs/review-heuristics.md` warns about |
| `cleak/u5-spec#88` | `PARTY.SAV` field layout and the U5 seed filenames | Implemented (`ee0dc53`). The seed pair is `INIT.GAM`/`INIT.OOL`; `BRIT.GAM` is withdrawn and **does not ship at all**, so the old constant named a file that is never present |
| `cleak/u5-spec#89` | Container acting-member selection, Ashes poison behavior, Ready turn cost, Sextant gate, and one-shot trap clearing | Implemented: both container sites use the shared zero/one/many acting-member selector, including disabled-member re-prompt, cancellation, and prompted name echo; Ashes is poisonable by gas; non-combat Ready always consumes its turn; Sextant requires the Britannia surface at night and uses the published label/coordinate layout; container records/cells clear before the copied trap state resolves |
| `cleak/u5-spec#90` | Outdoor ranged-reaction payload and source identities | Implemented: frigates absorb the transport-dependent hit in hull, other transports roll independently per living member, and the corrected Sea Serpent/Sand Trap classes use the published one-in-eight trigger |
| `cleak/u5-spec#91` | Dialogue byte `0x80` dictionary/control boundary | Reconciled with the published dispatcher classification and shipped-corpus audit |
| `cleak/u5-spec#92` | Beacon stencil table and bright-light source semantics | Implemented: production reads the 512-byte table directly from published `DATA.OVL` offset `0x1F8E`, validates all record classes, and fails loudly on mismatch; structural search remains test-only cross-check. Indoor `0x2A` sources are harvested from raw floors with first/last slot semantics |
| `cleak/u5-spec#93` | Gold-payment continuation, trap corpse marker, chest generation, Ready/Sextant corrections | Reconciled across conversation, trap, container, Ready, Use, and Sextant paths. Ready now uses the published Space/Enter confirmation, native vertical/corner navigation, mode-specific Escape literals, and ring-vanish close result; combat Ready and Z-stats end the acting combatant's action when their shared modal closes; combat U passes the live-actor gate and enters the shared item picker rather than the withdrawn label-only refusal; Pocket Watch includes zero-padded minutes; no invented paid/refused label transfer remains |
| `cleak/u5-spec#94` | Town-family entry cell after withdrawal of the `0x2A` spawn-marker reading | Implemented as fixed column 15, row 30, floor 0; floor changes preserve column and row |
| `cleak/u5-spec#95`/`#96` | Camp cooldown persistence and Lord British apparition caller gate | Implemented at `SAVED.GAM` `0x02E6`/`0x02E7`: time elapses before the refusal gate, refusal does not re-arm, the single bounded draw has no camp-marker write, and only uninterrupted overworld camps longer than five hours can run the event |
| `cleak/u5-spec#97` | Required-disk caller mapping, cache transitions, and retry guard | Implemented by `disk_prompt.rs` with typed roles, fixed/floppy session state, operation-family requests, recursive-error guard, and restoration of the entry role after save |
| `cleak/u5-spec#98` | Queued shipwright delivery save bytes | Implemented at `SAVED.GAM` `0x03AD`, `0x03AE`, and `0x105F`, preserving inactive/opaque bits and whole-byte increment/wrap; delivery clears only the class byte and never stores the queue in `.OOL` |
| `cleak/u5-spec#99` | Shared regalia effect codes and permanent duration | Implemented with Amulet `0x0E`, Crown `0x1C`, Badge `0x1D`, duration `0xFF`, same-item toggle-off, shared-slot clear sites, and the exact palace-guard Badge gate. Timing status is derived from this slot; no independent field can disagree with it |
| `cleak/u5-spec#100` | Dungeon backward-pass sprite source, field strobes, active objects, and decoration states | Implemented from public commit `9807eb4`: corrected sprite-count parsing; strict `ITEMS`/`MON0`-`MON7` dimensions; masked object/monster compositing; exact field pens, ranges, strokes, and endpoints; fountain point frames; normal-flavour six-state decoration updates; raw-`0x08` rising-pit overlay; and ordinary monster pose stepping |
| `cleak/u5-spec#101` | Dungeon setup reuse gate/record mapping, Negate Time forced pose, and decoration stage-5 tone | Implemented from public commits `abd0a17` and `19a0ba1`: call-site-controlled fresh/reuse setup, uniform family selection, authoritative `dep1 == 0xFF` inactive marker with valid family zero, all eight record fields, save/resume preservation, forced Negate Time poses, and exact per-band tone/delay sequences |
| `cleak/u5-spec#102` | Overworld prune type-byte classifier | Implemented from public commit `1fedad0`: byte 0 alone selects the exact four prunable ranges; byte 1 never promotes or vetoes a record. Exhaustive 256-value coverage accompanies production tests for excluded parked vehicles, included `0x2C..=0x2F` and `0xA8`, the `0xB5` hole, trigger, window, seam, and six-field clear |
| `cleak/u5-spec#103` | Generic adjacent-hostile low-water/transport classifiers, combat mapping, and multi-slot continuation | Implemented from public commit `a4167b0`: exact party terrain `0x00..=0x03` plus carpet `0x14..=0x15` or skiff `0x28..=0x2B` selects shared impact; every other recognized hostile enters terrain combat. Type byte alone maps pirates to class 1 with the fixed `Pirates` banner and ordinary `0x40..=0xFF` families by `(type - 0x40) / 4`. Arena selection independently follows the full terrain, frigate, ship-target, aquatic, and Shadow Lord priority table. Reactions remain staged from slot 31 down through 1 and lower slots resume after returning combat while movement stays suppressed |
| `cleak/u5-spec#104` | Sextant caller-level turn accounting | Implemented from public commit `8fc218f`: success, item-specific refusal, picker cancellation, and the no-usable-item branch all return the normal U-Use action result and run the current exploration mode's ordinary turn processing |
| `cleak/u5-spec#105` | Combat Escape narration, free-refusal ring-pass boundary, and exact Shape B punctuation | Implemented from public commit `b1e8e08`: exact Shape B tails; party-side Escape predicate and `Escape-Not here!` / `Escape-Not yet!` / newline-free `Escape!`; occupied descriptor/object cleanup ticks; free re-prompts before maintenance; committed non-digit invisibility/regeneration and active-effect aging; entry-only ring vanishing; and exact victory/defeat strings |
| `cleak/u5-spec#106` | Exact caller schedule, hidden composition, ordering, and cadence for the three map-viewport dissolve sites | Implemented from public commit `b34ae69`: rescue dissolves first to black before scratch/tableau work and then from black plus the centred on-foot party before every handoff write; lit dungeon Search dissolves the post-rewrite first-person view for exact `0x61`, the rewriting `0xC?` skeleton branch, and `0xD?`; darkness, `0x62`, narration-only cases, and every Open outcome bypass it. Every call exhausts the shared `(8,8)..(183,183)` dissolve without a world tick or caller redraw; Bevy and TUI acknowledge the ordered completed-call records when they present the caller-composed end state, so transient playback state cannot accumulate across frames |
| `cleak/u5-spec#107` | Exact phases 2–5 active-object eviction off-screen predicate | Implemented and exhaustively pinned from public commit `bc0c761`: per axis `(candidate - player + 5) mod 256 <= 10`; ±5 is inclusive, ±6 is outside, current player globals are authoritative rather than slot zero or scroll origin, and candidate floor is ignored. The distinct §8.1 scroll-base prune rule remains separate |
| `cleak/u5-spec#108` | Exact host-clock PRNG seed equation and caller sampling timing | Implemented from public commit `06494e0`: one sampled `(hour, minute, second, hundredth)` tuple is combined with the published byte truncations, wrapping addition, `0x91EB` XOR, and twelve-bit mask; exact vectors pin the transform. Gameplay construction and the blight, wilderness-camp, stranger-conversation, and Falsehood-theft sites use the published reseed timing |
| `cleak/u5-spec#109` | Exact town alarm/resident-Shadowlord NPC sweep bytes and predicates | Implemented from public commit `574f1d8`: exact `0x40..=0x73` ordinary band; `0xFC`/`0xD8`/`0x70` alarm pursuit exceptions; mode-3/6/7 destructive schedule rewrites; `0xFD`/`0xFE` Talk sentinels; all-floor occupied-slot alarm scope; shared-stream byte draws; resident all-32-index coin draws; and the fixed-slot-4 defect with Hatred/Cowardice asymmetry |
| `cleak/u5-spec#110` | Town boundary passability sample, off-grid occupancy coordinate, and prompt turn accounting | Implemented from public commit `3118380`: every edge samples loaded floor cell `(31,31)` through the transport predicate; occupancy uses the true `-1`/`32` candidate coordinate; blocked terrain, N, and Escape consume a normal turn while Y exits before turn processing |
| `cleak/u5-spec#111` | Combat C-Cast interference target-map lifecycle | Implemented from public commit `dac1f31`: save-backed 32-victim source map, factory-zero seed, ordinary automatic adjacent-attack writer including miss/overwrite paths, Cast-time actor revalidation, same-actor free re-prompt, completed-victim-action clear, and persistence across rounds, encounters, exits, and save/load |
| `cleak/u5-spec#112` | Reconcile remaining `0x2A` spawn-marker and open-entry wording | Resolved in public commit `8a73d12`; engine behavior was already correct, and the remaining public API names now distinguish fixed player entry, NPC-start-marker harvesting/scrubbing, and beacon-light-source harvesting |
| `cleak/u5-spec#113` | R-Ready possible subtree double-charge and exact clock cost | Resolved in public commit `82daf8d` and implemented: exactly one charge per invocation, nominally 2 minutes in the overworld and 1 in town/dungeon, with Quickness and Negate Time applied by the shared clock path; repeated picker attempts never add charges |
| `cleak/u5-spec#114` | Combat cursor-box and secondary-marker raster geometry | Resolved in public commit `7046ca8` and implemented: exact white two-pixel cursor ring; ordered white/black secondary strokes; lit/player/valid-cell shared gate; base, cursor, secondary composition; solid replacement writes; and display-only clipping without secondary-coordinate prevalidation |
| `cleak/u5-spec#115` | White-potion sweep and Orange/Purple combat presentation rasters/timing | Resolved in public commit `edba057` and implemented: the Bevy frontend consumes the selected-bottle flash as a blocking one-shot over the pre-effect framebuffer, applying the inclusive `(8,8)..(183,183)` paired palette-15 XOR and exact Orange/Purple/White rumble/sweep work; Orange retains the base tile while displaying `0x1E` until the one-in-seventeen wake restores the base or hidden `0x1D`; Purple persistently rewrites both object tile fields to `0x90`; White computes one threshold-32 field, recomposites it for twenty one-tick frames without added raster marks, double presentation advances, visibility-dirty writes, or gameplay-clock calls, and then runs the ordinary idle redraw. Bevy reads the per-frame BIOS-tick count from the typed sweep state rather than duplicating it as a frontend constant; terminal and headless raster frontends synchronously complete the same twenty-frame state before accepting input or capturing their final frame; the already-consumed visibility threshold is no longer retained as write-only playback metadata |
| `cleak/u5-spec#116` | Remaining Blue/Yellow/Red/Green/Black potion flash timing constants | Resolved in public commit `01e2e1b` and implemented for all eight selected bottles: rumble target `8,000 + 1,600i`, each of two sweeps `10,000 + 4,000i`; one shared runtime helper executes all three calibrated work loops for every sound-disabled frontend, live/scripted Bevy input stops at the blocking flash boundary, and terminal/headless paths consume the event before accepting another command |
| `cleak/u5-spec#117` | Return-to-View animated-terrain shimmer and carry-set single-cell dissolve rasters | Resolved in public commit `fcc8181` and implemented: exact opaque base/portal row-splice rasters for steps `1..15` and `15..1`; exact corner-first plus `0xB8` 256-pixel convergence permutation; direct overlay-versus-backing source selection; 31 eight-write preview-tick/input checkpoints with no final checkpoint; helper-owned actor/plane suppression; and opaque palette-index-zero writes |
| `cleak/u5-spec#118` | Subtitle-ignition publish-counter boundary, tail, abort-poll, and speaker cadence | Resolved in public commit `12485b3` and implemented: pass-reset 128/256 countdowns, 110/55 publications per pass, unpublished 31-state tails plus corner fixups, draw-before-advance, per-state keyboard polls, abort ordering with the pending key preserved for the menu's first poll, persistent gate/pitch recurrences, and 45/50 pacing branches |
| `cleak/u5-spec#119` | Arms `S` sell-browser paging, row content, and controls | Resolved in public commit `58e9b9c` and implemented: lowest-nonzero entry, four-row ascending pages, all normalized movement/select/exit keys, safe backward-boundary clamp, fixed short labels and count-255 rows, inverse selection, visible zero-price/ammunition rows, randomized records 49..56, sale/refusal continuation, and exact random-draw boundaries. The shell retains the published window-1 clear/widen/window-2 handoff |
| `cleak/u5-spec#120` | Exact subtitle-ignition fourteen-bit Galois sequence | Resolved in public commit `36780cb` and implemented: seed `0x0001`, direct state-to-coordinate mapping, right shift with conditional `0x3500` XOR, first-sixteen-state vector, first publication anchors, and exact `48/53` normal plus `35/33` slow burst totals |
| `cleak/u5-spec#121` | Arms sell-browser page-status glyph bytes | Resolved in public commit `5b9445f` and implemented: no badge for neither page; exact `02 19 01` following-only, `02 18 01` previous-only, and `02 12 01` both-page fixed-font sequences at local `(6,6)..(8,6)`. The gameplay-chrome compositor owns the badge so both caps retain their two-colour sprite treatment, and the same pass paints the browser's `Arms` stats-ribbon label |
| `cleak/u5-spec#122` | Presentation status-poll phase and abort-key handoff | Resolved in public commit `c869c5b` and implemented: the first gated start/menu dissolve copies before odd visits `1,3,5,...` poll, with pending-at-entry transferring exactly `(1,0)`; the loader's immediate consuming read locally downgrades to plain completion and skips subtitle ignition without changing the caller's title-skip flag, so automatic Return-to-View remains armed. Every Return-to-View preview tick and convergence checkpoint uses a consuming read; the abort key is discarded and the restored menu performs a fresh poll |
| `cleak/u5-spec#123` | Stonegate trapdoor defeat mutation and rescue handoff | Resolved in public commit `4d03a662` and implemented through the shared exploration defeat gate |
| `cleak/u5-spec#124` | Sleeping exploration tails and pre-rescue object persistence/teardown | Resolved in public commit `d3863ef` and implemented, including TUI and Bevy route coverage for the overworld all-32-record `BRIT.OOL` write before rescue |
| `cleak/u5-spec#125` | Combat field-contact actor, timing, scan, blocking, and PRNG semantics | Resolved in public commit `0a0b867` and implemented: common post-dispatch player/automatic hook, acting-slot target, linked-renderer-only skip, ascending marker priority, non-consuming Poison/Sleep/Fire, Energy movement blocking with no contact arm, and exact conditional direct-damage draws |
| `cleak/u5-spec#126` | Combat post-dispatch terrain-hazard priority arms and Doom absorption boundary | Resolved in public commit `b600bc6` and implemented: exact terrain `0x04` maps to Poison, `0x8F`/`0xBC` map to Fire, all other bytes fall through, and terrain selection suppresses marker scanning even after Poison rejection. Doom is a separate no-PRNG committed non-digit player-action tail check against renderer companion row 1 while the live actor stands on row 2; digit selections, parser refusals, and automatic dispatches bypass it |
| `cleak/u5-spec#127` | Closing-certificate TH/ST rune bytes and encoded centering | Resolved in public commit `21698d6` and implemented: `TH` -> `[` (`0x5B`), `ST` -> `_` (`0x5F`), space -> `@` (`0x40`), exact stored vectors, and 20-cell first-line centering at column 10 |
| `cleak/u5-spec#128` | `0x85` refusal text and nested keyword envelope | Resolved in public commit `98dfd45` and implemented: no confirmation read or outcome labels, in-place affordable continuation, exact quoted two-line-feed refusal, pending-word discard, ordinary nested reprompts, and stop/Bye unwind without duplication |
| `cleak/u5-spec#129` | Monster blink/summon PRNG gates, draw order, placement source, and handled result | Resolved in public commit `a7e55bf` and implemented: lazy shared-stream draws, exact `0..=31` gates, fresh X-then-Y `0..=15` summon probe, one attempt, success turn consumption, and failure continuation |
| `cleak/u5-spec#130` | Local-View viewport, placement, side-panel, and close-redraw contract | Resolved in public commit `e335918` and implemented: clear the full inclusive `(8,8)..(183,183)` gameplay viewport, compose the 128x128 raster at absolute `(32,32)`, never touch `x >= 192`, keep diagnostic maps out of the graphical message panel, and use ordinary world redraw on close |
| `cleak/u5-spec#131` | Shared combat-resistance score formula and comparison | Resolved in public commit `60ac944` and implemented: party-owner Intelligence versus monster-class endurance; signed unclamped `truncate_toward_zero((T-C+30)/2)`; one inclusive `0..=60` draw converted to the skewed `1..=30` roll; strict `S > R` blocking so equality lands; all seven shared callers; and the separate Tremor/Poison-Wind `roll >= combat weight` gate with forced-weight-one cases |
| `cleak/u5-spec#133` | Native town door/restraint IDs, Jimmy prisoner-release lifecycle, and committed exploration exits | Resolved and implemented: magic locks break a key before any picker/PRNG path, empty restraints refuse before picker/PRNG, successful release clears live dialogue awareness, changes all three schedule modes to 5, grants the first-time reward, suppresses the mode-5 adjacent attack event, and persists the scene/slot removal mask across save/reload. Dungeon no-keys/no-lock/unavailable/cancel exits each commit one action; Jimmy, Open, and An Sanct share the open-chest rewrite that clears trap/subtype bits and preserves only visit marker `0x08` |
| `cleak/u5-spec#134` | Y-Yell sail-toggle scene gate and clean action result | Resolved in public commit `0b1cfe2` and implemented: a frigate toggles only for unsigned scene bytes `0x00..=0x7f`; `0x80..=0xff` enters the ordinary word prompt. The accepted branch preserves heading, consumes one action, and prints exact `HOIST!`/`FURL!`. Submitting the ordinary prompt empty now also reports acted in world, town, dungeon, and combat instead of retaining the old no-turn placeholder; exhaustive byte, cross-mode input, TUI route, and Bevy visual-route coverage pin the boundary |
| `cleak/u5-spec#135` | Ruined-shrine Word-of-Power mantra handoff | Resolved in public commit `24f4aa4` and implemented: the word-indexed virtue/mantra session asks all four exact prompts, uses case-insensitive substring matching, keeps Escape local to the current field, silently fails after the fourth response, and on coordinate-valid success restores `0x1A` to `0x19` while clearing only the shrine ruin high bit and preserving the seal flag |
| `cleak/u5-spec#136` | Endgame gate partial-phase raster | Resolved in public commit `b5d9cde` and implemented: the victory rite now installs live gate terrain at `(5,4)`, drives the shared save-backed moongate counter through rise `1..15`, four full-height phase-16 ticks, actor exits, sink `15..1`, and final phase-0 floor restore. The Bevy tableau renderer reuses the exact opaque `0x44`/`0xDC` row splice through scratch tile `0x116`; phase 15 retains floor row 0, and active-object sprites composite afterward |
| `cleak/u5-spec#137` | Rectangle-dissolve wall-clock contract | Resolved in public commit `5f1155b`: atomic publication at the blocking-call boundary is normative. The engine's existing completed-call presentation is correct; optional prefix animation has no normative duration and may not add ticks, input consumption, or abort points |
| `cleak/u5-spec#138` | Alternate-depth cinematic rendering scope | Resolved in public commit `230b024`: EGA is the sole pixel-exact v1 target. CGA, Hercules, and Tandy cinematic presentation is optional and must be labeled a modern approximation, so no alternate-depth conversion remains a v1 completion blocker |
| `cleak/u5-spec#139` | Title-flourish timing | Resolved in public commit `0ebc456`: 14 ms for each of 85 logical presentations is the normative modern cadence, nominally 1.190 s total, with no catch-up. The existing scheduler uses this exact deadline; captured acceptance allows mean 14 ms ± 1 ms and total 1.190 s ± 0.100 s |

The answered public clean-room issue queue is reconciled through `#139`.
The later corrective answer on public `#11` is also reconciled: Kill's
protected class-id filter is 14/15/47; Cause Fear and Repel Undead directly
write combat HP 1 and fleeing bit `0x02`; Repel does not enter death or XP
paths; Conjure uses 16 weighted outcomes; Conjure/Swarm/Summon share the
whole-candidate `0..=15` probe; Swarm places up to four actors at one cell; and
party Summon stamps controlled only after self-check success. Public `#132`
closed in `1e28720`: protected Kill rejection follows the shared charge/7-MP
and pre-effect envelope, bypasses resistance PRNG and target effects, reports
`Failed!`, and commits the combat action without reopening either prompt.
Public commit `8a73d12` consistently
separates fixed `(15,30,0)` player entry,
NPC-start coordinates, and beacon-light coordinates. The engine follows that
contract and names the NPC-only harvest/scrub API accordingly. Public commit
`82daf8d` confirms that Ready has no subtree clock call and costs exactly one
mode-owned charge per invocation; the engine now pins the 2/1/1-minute mode
matrix and both shared time modifiers. Public commit `7046ca8` resolves the
combat overlay raster, including the shared eligibility gate and exact pixel
composition; the clean substitutes are removed. Public commit `edba057` resolves
the potion presentation contract as record rewrites and ordinary compositing,
not custom White/sleep/Poof overlays; it also pins the blocking flash geometry,
sound-loop values, frozen White repaint sequence, and no-extra-turn rules.

## Verification Baseline

Re-measured on 2026-08-24 in the current worktree. Asset-backed runs used isolated
temporary save directories and treated `C:\Games\U5-Clean` as read-only input;
the visual asset corpus itself was never modified. The engine refuses a write
destination that resolves to `DEFAULT_GAME_DIR`, and `copy_asset_writable`
clears the read-only bit Windows `fs::copy` propagates into scratch copies.

| Command | Result |
|---|---|
| `cargo test -p u5-runtime --lib` | 3223 pass |
| `cargo test -p u5-bevy` | 183 pass |
| `cargo test -p u5-tui -- --test-threads=1` | 103 pass (14 + 51 + 38) |
| `cargo test --workspace` | pass, including all asset-backed suites and doc tests |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets` | **zero errors**; style warnings remain and are not gated |
| `--route-smoke <asset-copy>` | all **513** scripted cases pass |
| `--visual-frame-suite <asset-copy>` | **193** PNGs plus a sanitized manifest |
| `--visual-route-suite <asset-copy>` | **1906** PNGs plus a sanitized manifest |

The route-smoke corpus spans world, town, dungeon, combat, endgame and shop
play: all 40 published stock world-location entry rows, TLK-backed reserved-word
and no-match conversation routes across all 32 named-location scenes,
save/reload checkpoints for transport and transition continuity, the full
spell/combat command matrix, all nine public arms-shop stock rows, the nine
tavern lore selectors, the four shipwright delivery rows, the three native shard
vanquish routes, the Word-of-Power seals, and the terminal endgame through the
full victory cinematic. Native Jimmy routes additionally prove promptless
magic-lock key breakage, pre-picker empty-restraint refusal, live prisoner
release without the mode-5 adjacent attack, and scene/slot removal-mask
persistence across save/reload. Dedicated ordinary Open and An Sanct routes
start from dungeon chest byte `0x4b` and require the shared `0x78` rewrite,
proving that both live dispatch paths clear trap/subtype bits while preserving
the visit marker. Vehicle routes now validate both furled-ship X-it outcomes
when no landing is adjacent: launching a carried skiff parks the full-hull ship
with its skiff count decremented, while a ship with neither a skiff nor a stowed
carpet reports the canonical no-skiffs warning, preserves transport, and spends
no turn. A dedicated route also proves that overworld defeat writes a
synthetic far-slot live object to the plane OOL mirror before the ordinary
rescue restores the party. Its validator requires
`cinematic_is_finished()`, so an ending that stops short fails the case.

The visual route suite's 1906 frames are all nonblank except exactly one, which
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
| §7 Per-round structure | `combat_driver.rs` round-walk classifier; `combat_frame.rs` combat-only cursor-blink tick | `chunk_23.rs` round-cycle and cursor-blink tests | Implemented. **§7's "post-round maintenance pass" was an invented contract and is retracted.** We had built a row-major sweep classifying each arena cell's terrain byte and dispatching effects, plus a magic-effect timer tick. It was removed in `60ec07c` after being shown inert in our own tree: the report it built was discarded by both call sites and `combat_magic_effect_timer` was write-only. Route-smoke's asset-backed cases pass unchanged without it. What is real is a combat-only cursor highlight (blink toggle, active-actor box, optional secondary marker), which has live renderer consumers. |
| §8 Player commands | `combat_scenario.rs` (`CombatScenarioInput`), `combat_actor.rs` command classification, and `input_dispatch.rs` modal-action completion | command-route smoke plus combat Use/Ready/Z-stats lifecycle tests in `chunk_23.rs` | Implemented. Combat U is a live-actor-gated multistage command that enters the same item picker as world modes; completing or cancelling Use, Ready, or Z-stats then ends that combatant's action exactly once. |
| §9 Monster AI | `combat_frame.rs::apply_combat_ai_turn`, shared resistance/weight helpers, `combat_ai_actor_fleeing`, `combat_target_group_for_slot`, suppression bypass; `combat_actor.rs` exact special gate, resistance arithmetic, weight rules, and `first_monster_ability`; `combat_stats.rs` class trait/endurance rows | `combat_ai`, `combat_actor_slot_dispatch`, shared-resistance skew/score/equality/source tests, target-weight tests, exact gate boundaries, lazy Pass-1/Pass-2 PRNG ordering, one-probe summon success/failure continuation, handled-turn, and exhaustive combat stat/ranged/ability-hook row tests in chunk 23 | Implemented through public `#131`, including possession's exact shared resistance draw and result predicate |
| §10 Spells in combat | `magic.rs` (scene-mask `SPELL_SCENE_COMBAT`), `combat_actor.rs::combat_arena_terrain_contact_kind`, `combat_frame.rs` directed-spell dispatch, Doom companion-band check, and common post-dispatch field contact | `directed_spell_status`, spell-route, exact terrain classifier, terrain-over-marker priority, Doom timing, scan-order, exact-PRNG, Energy-blocking, re-prompt, player/automatic dispatch, and non-consuming field tests in `chunk_23.rs` | Implemented for the published terrain, marker, and Doom action-tail behavior through `cleak/u5-spec#126` |
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
| §5 R-Ready flow | `play_state_impl/chunk_03.rs` ready handler and `z_stats.rs` input classifier | Space/Enter, native vertical/corner navigation, member-cancel, mode-specific Escape, repeated-attempt single-charge, exact 2/1/1-minute mode matrix, Quickness, and Negate Time tests | Implemented from public commit `82daf8d`; Ready charges exactly once per invocation and never from its equipment subtree |
| §6 R-Ready eligibility | strength gate, occupancy, ring-vanish in ready handler | ring-vanish 1-in-16 and picker-close tests | Implemented |
| §7 U-Use flows | `play_state_impl/chunk_04.rs::apply_u_use_*` for every public family (torch/gem/key/scroll/potion/Moonstone/regalia/shard/carpet/skull key/spyglass/HMS plans/sextant/pocket watch/wooden box); the command layer guarantees one normal exploration action for success, refusal, cancellation, and no usable items; combat U enters the same picker and retains the acting slot until the picker terminates; shard destruction follows public issue #31 exact party positions, shard/flame pairing, matching Shadowlord encounter north of the party, and save-backed quest-progress bits | per-item use tests in `chunk_03.rs`–`chunk_07.rs`, active-picker tests in `chunk_17.rs`, combat picker/action-lifecycle tests in `chunk_23.rs`, and shard/flame tests cover all three published native Eternal Flame positions, exact scene/floor/coordinate rejection, U-Use turn accounting, and quest-progress bit mutation | Implemented |
| §8 Implementation contract | `equipment.rs` 0xFF sentinel; carried/readied separation | contract tests | Implemented |
| §9 Boundaries | Ring of Invisibility/Regeneration in combat | combat ring-vanish tests | Implemented |

### `systems/containers.md`, `systems/traps.md`, `systems/hidden-treasures.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `containers.md` §1–§10 | `containers.rs`, `play_state_impl/chunk_04.rs` Get/Open/Jimmy/Search dispatch | chest, Search, object-pickup tests in `chunk_04.rs`, `chunk_19.rs` | Implemented |
| `traps.md` §1–§5 | `traps.rs::TrapEffect`, `containers.rs` trap routing; live dispatch routes through `shared_trap_effect_family_from_index` and both container sites use the shared acting-member selector | trap-effect, acting-member, one-shot container, poison-helper, and effect-distribution tests in chunks 17 and 21 | Implemented. **`§3`'s retraction fixed a serious live bug**: the status helper used by effect ids 1 and 3 is a *poison* primitive, not a revive primitive, and we had shipped the retracted draft verbatim (`if status == 'D' { status = 'P' }`). Poison traps therefore did **nothing at all** to a healthy party — 2/8 of non-combat trap rolls and *half* of every combat roll — and gas traps **resurrected dead members** instead of poisoning the living. `cleak/u5-spec#89` subsequently fixed the acting-member priority, confirmed that gas can poison Ashes, and published the pre-resolution object/cell clear that makes a container trap one-shot |
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
| §7.1 Printable text | `tlk_runner.rs` word-buffer, soft-break, and exact dictionary-token leading/pending-space state | text emission and token-sequencing tests | Implemented. A populated token always emits its leading space and arms pending space for the next printable TLK byte; another token or control byte does not consume it |
| §7.2 Avatar name (`0x81`/`0x82`) | `tlk_runner.rs` interpolation | name substitution tests | Implemented; substitution does not consume a pending dictionary space |
| §7.3 Pause (`0x83`/`0x8F`) | `tlk_runner.rs` pause emit; redraw delegated to frontend | pause tests | Implemented |
| §7.4 Newlines (`0x8A`/`0x8D`) | `tlk_runner.rs` newline emit | newline tests | Implemented |
| §7.5 Print mask / curse (`0x8B`/`0x8E`) | `tlk_runner.rs::PrintMask`, `TlkRenderedGlyph`, curse-check hook; conversation transcript carries ordinary/runic font identity through Bevy and TUI rendering | mask-pair, transcript propagation, active-conversation, and Bevy font-selection tests | Implemented; protected bytes render through `RUNES.CH` rather than being flattened into ordinary text |
| §7.6 Branching (`0x85`/`0x86`/`0x8C`/`0xFE`) | `tlk_control_codes.rs::TlkActionDispatchVerb`, `tlk_if_else_alt_branches`, `play_state_impl/chunk_04.rs::apply_tlk_action_grants` | gold-payment, action-letter, IF/ELSE, karma-threshold tests, plus a sanitized shipped-TLK corpus audit for public action/payment/branch controls | Implemented (`0x85` toll-milestone karma — `cleak/u5-spec#27`) |
| §7.6 `0x87` follow-up scan | `tlk_runner.rs::TlkRunStop::FollowUpKeywordScan` | follow-up scan tests plus shipped-TLK corpus control-shape coverage | Implemented |
| §7.7 Labels / GOTO | `tlk_runner.rs` label dispatch | label scan tests | Implemented |
| §8 Common-word dictionary | `common_words_io.rs` 128-entry shared table from `catalogs/common-word-dictionary.md`; exact TLK and `shoppe_bark.rs` consumers | populated/empty TLK token tests, exact SHOPPE spacing tests, malformed-empty SHOPPE tests, and sanitized full-record render audit | Implemented. Empty TLK slots emit the leading space plus the raw token byte in the runic font and do not arm pending space; an empty SHOPPE token is malformed content |
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
| §4 SHOPPE.DAT structure | `shoppe_records.rs`, `shoppe_bark.rs`, shared dictionary validation | parser, empty-dictionary rejection, and sanitized render-audit tests | Implemented |
| §5 Bark renderer | `shoppe_bark.rs` substitution (`%/^/$/&/*/#/@`) plus exact token leading/trailing spacing | token/token and token/text spacing tests, bark tests, and aggregate coverage that renders every non-empty local `SHOPPE.DAT` record when clean assets are present | Implemented |
| §6 Pricing model | `shops.rs::arms_shop_price`, healer table, etc. | pricing tests in `chunk_21.rs` | Implemented |
| §7 Inventory model | `equipment.rs` stock tables; inn registry in `play_state_struct.rs` | inn-stay tests | Implemented |
| §8.1 Weaponsmith/armourer | `shops.rs::ArmsShop`, `shop_session.rs::arms_shop_for_scene`, `shops.rs::arms_shop_stock_letter_index` | arms scene-row, published stock-row, and transaction tests | Implemented (public #41 scene-to-`a..h` stock rows) |
| §8.2 Guildmaster | `shops.rs` guild prices | guild tests | Implemented |
| §8.3 Healer | healer arm in `shop_runtime.rs`; Minoc bypass | healer tests | Implemented |
| §8.4 Innkeeper | `shop_runtime.rs` inn flow; stay counter in `clock.rs`; public issue #15 Intelligence-adjusted rest, leave, and pickup charges plus paid-rest class recovery and poison death conversion | inn tests | Implemented |
| §8.5 Tavernkeeper | tavern arm in `shop_runtime.rs` | tavern tests, asset-backed menu/quote/follow-up rendering, partial-surcharge and charity production-path tests | Implemented, including the explicit Anything-else Y/N state and 25-food provision packs |
| §8.6 Sage | sage arm: `catalogs/sage-rumours.md` 26-row paid keyword lookup, strict four-letter topic boundary, SHOPPE record 84 fee quote, gold debit before success-template RNG, short-funds exit with SHOPPE record 91, and SHOPPE record 85..=88 success rendering | sage runtime tests plus full public #13 table-sync, SHOPPE rendering, and PRNG-timing tests | Implemented |
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
| `intro.md` §1–§14 | `intro.rs`, `intro_menu.rs`, `menu_dispatch.rs`, `pth.rs` (BRITISH.PTH walker), `return_to_view.rs`, `story_io.rs`; Bevy intro shell composes published title bitmap, animates signature path, draws the four title-tick flame bands from `ULTIMA.16` records 1..=4 (the public issue #52 procedural flame stripe was withdrawn and its generator deleted), renders all 21 story slides with the spec-defined transition-strip and secondary-art draws plus step 6's published #69 doorway text, and reveals story step 1 and the STARTSC menu loader through the public issue #53 rectangle dissolve (the 36-tick and 320-tick column sweeps were withdrawn) before sampling menu input; Return-to-View preview rendering uses public title-tick animation families, transparent actor overlay composition, public #54 fixed strip captions from LoadMapStrip, high-opcode no-ops, 4x19 source strip loading, `(x, y + 7)` cell-effect coordinates, and the exact public #117 shimmer/convergence rasters and checkpoint timing. Its Bevy playback wraps to frame zero when the shipped `0x09` restarts the stream and live Escape follows the same immediate-abort route as every other key instead of being gated to the last expanded frame; §11 (`A` submenu) is an artwork screen, not a text screen - the credits page is published band by band out of the hidden surface at its published origins, and any key (`Esc` included) starts the close phase that publishes the rebuilt menu back. No credits text is authored; the earlier `ACKNOWLEDGEMENTS_LINES` clean-room-authored constant was removed. The terminal harness has no pixel surface for it and still fails loudly through `intro.rs::require_graphical_acknowledgements_surface`. §11.2's full phase sequence is implemented: `intro_acknowledgements.rs` owns the geometry, step lists and one-BIOS-tick-per-step pacing for the part and close phases, and `u5-bevy` composites the rise, part, close and sink phases across a hidden surface and the visible page | intro/chargen menu tests in `chunk_01.rs`, `chunk_02.rs`; Bevy intro framebuffer/title-tick/story-wipe tests; Bevy acknowledgements phase/coverage/pacing tests; Return-to-View renderer/playback tests; subtitle-ignition state/publication/gate-vector tests; `intro_acknowledgements::tests` (exact column coverage, step counts, pacing, the row-63 floor); `intro::tests::acknowledgements_refuses_placeholder_lines_without_a_pixel_surface` | Complete for every fully published intro contract, including public issues #118 and #120's exact subtitle-ignition cadence and sequence |
| `chargen.md` §1–§11 | `chargen.rs` questionnaire VM, gender prompt, virtue tournament, stat assignment | chargen tests | Implemented |
| `u4-transfer.md` §1–§10 | `u4_transfer_preview.rs` — the engine's **single** `PARTY.SAV` parser, per `§5.1`/`§5.2`/`§5.3` (`cleak/u5-spec#88`); `u4_transfer.rs` commit side, `u4_transfer_session.rs` state machine, `INIT.GAM`/`INIT.OOL` seed handling, stat translation, OOL ordering; `crates/u5-bevy/src/u4_transfer.rs` for the `§6` preview screen | u4-transfer tests; `intro-u4-transfer-found` and `intro-u4-transfer-panels` in the visual frame suite | Implemented. `§6.1`-`§6.6` are drawn (`f3ecfd1`). The older parser is retired: it rejected all-zero virtue standings as "no transferable data" when `§5.3` makes that the Avatar **success** condition, so it turned away exactly the completed Ultima IV Avatar the path exists to import; it validated party-wide counters `§5.2` says are never read; and it read the name and class at `0x001A`/`0x0019` rather than `0x001C`/`0x002D`. Its `crates/u5-tui` fixture was built to the same wrong offsets, so parser and fixture agreed with each other and with nothing else. The U5-side seed pair is `INIT.GAM` (4192 bytes) and `INIT.OOL`; `#88` withdrew `BRIT.GAM`, which **does not ship at all** — the old constant named a file that is never present, and the test pinning it could not have failed either way. **Not validated against a real Ultima IV save**: none exists on this machine, so every fixture is synthetic and built from the published layout |

### `systems/save-load.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§8 | `save_load.rs`, `disk_prompt.rs`, `disk_io.rs`, `active_object_io.rs`, `play_state_struct.rs` four-file contract (SAVED.GAM/SAVED.OOL/BRIT.OOL/UNDER.OOL), empty-save guard, load-time mirror refresh, save-time UNDER-then-BRIT staging with the conditional unchanged `UNDER.OOL` write and no `BRIT.OOL` write, typed disk-role restoration, shared active-effect and queued-vehicle bytes, read/write retry wrapper, original binary content/resource loader disk I/O, vehicle/transition save round-trips; `town_npc_mutations.rs` preserves public destructive schedule/dialogue rewrites in a narrow engine-owned companion save without modifying original `.NPC` assets | save/load tests across `chunk_03.rs`, `chunk_04.rs`, `chunk_05.rs`, `chunk_07.rs`, `chunk_09.rs`, `chunk_11.rs`, `chunk_13.rs`, `chunk_23.rs`, plus companion-ledger round-trip, malformed-row rejection, and town-reload reapplication tests | Implemented |

### `systems/movement.md`, `systems/overworld.md`, `systems/town-mode.md`, `systems/dungeon-mode.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `movement.md` §1–§10 | `direction.rs`, `tile_classes.rs`, `predicates.rs`, `transport.rs`, `active_object_io.rs`; native static terrain predicates cover the published foot, horse, carpet, ship, and facing-sensitive skiff tile sets | per-mode movement tests plus exhaustive 0..=255 transport predicate tests | Implemented |
| `overworld.md` §1–§15 | `main_loop.rs` shared party-capability classifier, `play_state_impl/chunk_11.rs` exploration gate, `play_state_impl/chunk_01.rs` overworld loop, `world_tables.rs`, `moongate.rs`, `moongate_phase.rs`, `moongate_transit.rs`, `lord_british_camp.rs`, native and sidecar encounters, public Word-of-Power seal rows | world tests in chunks 03, 05, 06, 07, 10, 12, 13, 15, 17, 23, 30, and 32; moongate phase/transit composition, shared gate/rescue, two-minute sleep/object-tail, full-table defeat persistence, ground-plate, blocking-playback, and save round-trip tests | Implemented for published deterministic behavior. `§9.1` gate presence is a sixteen-step global counter persisted at `SAVED.GAM` offset `0x02E1`; `§9.2` runs the blocking 256-step dissolve and 15-to-1 phase countdown before resolving entry. Scratch tile `0x116` is preserved across composition and doubles as the party-vanishing sprite. Public issue #124, resolved at clean-spec commit `d3863ef`, gives sleeping parties the full ordinary two-minute no-input tail and writes all 32 unchanged live object records—including slot zero—to the current plane mirror before defeat rescue, without running ordinary object maintenance |
| `town-mode.md` §1–§17 | `main_loop.rs` shared party-capability classifier, `play_state_impl/chunk_11.rs` exploration gate, `town_mode.rs`, `town_tables.rs`, `location_audit.rs`, `town_npc_mutations.rs`, `stonegate_trapdoor.rs`, NPC schedules, dawn/dusk substitution, destructive alarms and resident sweeps, exact floor-transition tile classifiers, and the exact free-roaming `0x10`/`0x11` object walker | town tests in chunks 04, 06, 10, 11, 15, 19, 21, 22, 23, 24, and 32, including shared gate/rescue and wake-before-underfoot ordering, exhaustive K-Klimb/trapdoor tile classification, directional link and adjacent climb-over turn rules, native post-turn trapdoor damage/carpet suppression/full reloads, Stonegate's exact same-scene grid/object/party defeat state, next-gate rescue, and blocking presentation record, exact #109 alarm/resident rewrite and persistence coverage, free-roaming PRNG/order/pen/destination/edge-boundary coverage, plus sanitized shipped `LOCATION.DAT` aggregate owner/class/view audits when local clean assets are present | Implemented for the published deterministic contract. Public issue #123, resolved at clean-spec commit `4d03a662`, confirms that Stonegate fills the live grid with `0x8F`, clears all 32 object records before restoring only slot-zero X/Y/Z, kills the in-party roster without changing location or transport, and relies on the ordinary defeat rescue rather than a durable flag. The shared gate now performs that rescue on the next exploration iteration. Public #51 tile `0x04` poison-gas step behavior is native; coordinate and tile-attribute sidecars no longer trigger that branch |
| `dungeon-mode.md` §1–§17 | `main_loop.rs` shared party-capability classifier, `play_state_impl/chunk_11.rs` exploration gate, `play_state_impl/chunk_*.rs` dungeon loop, `dungeon_tables.rs`, raster in `crates/u5-bevy/src/lib.rs` first-person draw | dungeon tests in chunks 05, 12, 13, 18, 20, 23, and 32, including shared gate/rescue and sleep-pass no-post-action coverage | Implemented for published deterministic behavior. Public issue #124, resolved at clean-spec commit `d3863ef`, defines a draw-free, transient graphics teardown immediately before rescue. The engine's ordinary atlas is permanently resident and it holds no dungeon-only corridor/item or monster-bank references, so the specified postconditions require no modeled mutation before the shared rescue |

### `systems/encounters.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| §1–§14 | `play_state_impl/chunk_*.rs` random spawn probe (native + sidecar), fortunes-of-war counter, sleep-ambush in `rest_camp.rs`, dungeon-room arena selection, public #21 dungeon active-monster ambush setup | encounter and ambush tests | Implemented. `§4`/`§9`: a full active-object table does not make an acquisition fail — the spawner acquires *or evicts* — and the "table full" early-out the spec explicitly withdraws is gone. The 128-candidate coordinate loop was already correct and is now pinned |

### `systems/vehicles.md`, `systems/weather.md`, `systems/moons.md`, `systems/time.md`, `systems/rest-and-camp.md`, `systems/lighting.md`, `systems/doors-and-z-transitions.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `vehicles.md` §1–§11 | `transport.rs`, `play_state_impl/chunk_*.rs` Board/X-it/Yell sails, `ship_broadside.rs` Fire | vehicle save/load round-trip tests, broadside tests, exhaustive Yell scene-byte gate tests, live town/dungeon sail routes, and TUI/Bevy X-it launch/no-skiffs route validators | Implemented |
| `weather.md` §1–§11 | `wind.rs`, Rel Hur cast in `magic.rs`, sail cadence | wind cast and sailing tests, including the two-wait into-wind case | Implemented |
| `moons.md` §1–§4 | `play_state_impl/chunk_*.rs` sky strip; moongate counters in `moongate.rs`; public issue #38 Felucca phase-0 glyph bytes for hours 10/11/19/20 | sky-strip and moon-glyph cache tests | Implemented. `§3` publishes that below the surface nothing is drawn, erased, *or cached*; the engine used to refresh the cached glyph bytes on every hour change in every mode, including dungeons, the Underworld plane and town basements, and no longer does (`f976af0`) |
| `time.md` §1–§13 | `clock.rs` cascade, Q/T tag handling, mode-specific increment | clock and cascade tests | Implemented |
| `rest-and-camp.md` §1–§10 | `rest_camp.rs`, `lord_british_camp.rs`, native town inn bed gate in `play_state_impl/chunk_07.rs`, `play_state_impl/chunk_08.rs::apply_completed_long_camp_recovery`, and hourly Ring of Regeneration tick in `chunk_09.rs` | rest, native inn bed/no-inn refusal, sidecar override, camp, ambush, long-camp recovery, and hourly ring tests | Implemented (ordinary rest has no direct HP/MP recovery; current checked-in spec matches public #47 issue-comment behavior) |
| `lighting.rs` §1–§11 | `lighting.rs` ambient + torch + light-spell counters | lighting tests | Implemented |
| `doors-and-z-transitions.md` §1–§15 | `jimmy.rs`, `play_state_impl/chunk_*.rs` open/get/look cascade, `ship_broadside.rs` BOOOM, secret doors, climb command | jimmy, open, secret-door, klimb tests | Implemented. `dungeon-mode.md §8`/`§13.1`/`§13.2` corrected Klimb (`2254649`): the whole pit family `0x6?` — marked and fired variants included, since the dispatcher masks to the high nibble — is an **ordinary descent** calling the same level-step helper a down ladder uses, not a surface-reset ejection. Klimbing Deceit level zero `(1, 3)` or Destard level zero `(7, 3)`/`(1, 7)` used to eject the party to Britannia from the top level of two dungeons. `§13.1` also withdraws the claim that a climb tests the cell it lands on — the ladder or pit underfoot is proof enough — so that predicate moved to the Up/Down level-change spells, and level-zero up runs the surface-reset contract so dungeons stay leavable |

### `systems/endgame.md`, `systems/blackthorn.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `endgame.md` §1–§12 | `endgame.rs`, `endgame_cinematic.rs`, `end_io.rs` (public END.DAT final narrative windows), `endmsg_io.rs`; Bevy endgame modal drives Lord British's entrance, the Orb acknowledgement, the shared-moongate `1..15` rise/four-tick full hold/actor exits/`15..1` sink/floor restore, advances the six fixed END.DAT narrative windows without intro-style page wipes, and presents the full-screen fade before the first window | exact shared-counter sequence and row-splice tests; Bevy actor-over-gate raster test; endgame fade tests; route-smoke and 1,906-frame visual-route terminal-victory coverage | Implemented through public issue #136 |
| `blackthorn.md` §1–§11 | `blackthorn.rs`, `blackthorn_session.rs`, KARMA.DAT verdict mapping | Blackthorn challenge/rescue tests; Blackthorn audience/rescue route-smoke; Shadowlord route-smoke | Implemented |

### `systems/boot.md`, `systems/launcher.md`, `systems/main-loop.md`, `systems/disk-prompt.md`, `systems/runtime.md`, `systems/input.md`, `systems/commands.md`, `systems/animation.md`, `systems/lighting.md`, `systems/view.md`, `systems/visibility.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `boot.md` | `boot.rs` machine/capability classes, explicit selector precedence, driver-family and asset-depth selection, Tandy low-memory fallback | boot driver-selection, machine-class, suffix, and threshold tests | Implemented at the portable engine boundary; firmware probes are represented by typed inputs rather than performed by the modern frontends |
| `launcher.md` | `boot.rs` game-owned startup filenames and C/E/T/H selector parser; intro/menu handoff and permanent resident dispatcher | filename, selector, intro menu, and mode-dispatch tests | Implemented; host packaging remains outside engine semantics as the contract requires |
| `main-loop.md` | `play_state_impl/chunk_01.rs` mode dispatcher | mode-dispatch tests | Implemented |
| `disk-prompt.md` | `disk_prompt.rs` typed required-disk/session state machine and `disk_io.rs` read/write retry phases, with shared error presentation surfaced by TUI and Bevy Journey Onward | alias folding, disk-role restoration, other-floppy/fixed-disk branches, recursion guard, write-protect handler, and retry tests; TUI and Bevy intro disk-error presentation tests | Implemented. The withdrawn `screen-mode-dispatch.md` model is not implemented: this is session-only disk state, not a presentation-mode controller |
| `runtime.md` | `play_state_struct.rs`, `play_options.rs`, boot in `boot.rs` | start-validation tests | Implemented |
| `input.md` | `input_codes.rs`, `input_dispatch.rs` | typeahead tests | Implemented |
| `commands.md` | `commands.rs` + per-command handlers in `play_state_impl/chunk_*.rs`; reference in `docs/commands.md` | per-command route-smoke and unit tests | Implemented |
| `animation.md` | `animation.rs`; visual cadence in `u5-bevy` | `chunk_29.rs` family/gate/selector tests | Implemented against the **replaced** `§6` family list (`774dff0`). The water/lava/fire/wind model this engine carried is withdrawn — "no water, lava, brazier or torch tile animates through this pass at all" — and with it `STATIC_TILE_ANIMATION_FRAME_WRAP` and the single shared frame per family. The five real families are waterfall `0xD4..0xD7`, fountain `0xD8..0xDB`, pendulum `0x80..0x83`, standard of Britannia `0xEC..0xEF`, and clock `0xFA..0xFB` / bellows `0xFC..0xFD`, with nested gating (waterfall and fountain ungated, then bit 0, then bit 1 only inside the bit-0 gate) giving net 1x / 2x / 4x rates, **per-id selectors** so a four-frame family's ids stay a quarter-cycle apart, and `STATIC_TILE_ANIMATION_PERIOD_TICKS = 8`. Visible consequence: repaint cadence drops sharply in ocean and coastal areas. The moongate presence counter is not a member of these families and `tick_static_tiles` never advances it |
| `view.md` | canonical `view_classes.rs` classification, `sky_view.rs`, and V-View painters in `play_state_impl/chunk_04.rs`; explicit View/Peer/X-Ray/Sky overlay modes and Bevy overlay draws | exact 4x4 class masks, all-256 classifier identity, river-corner source selection, all-seven road variants, full gameplay-viewport clear, absolute `(32,32)` compositor origin, unchanged side-panel/message-window regression, ordinary-redraw close regression, View overlay route-smoke, and focused sky calendar/PRNG/daylight-damage/colour/marker tests | Implemented, including exact telescope sky behavior and the corrected local-view class contract. Deep water now draws its single micro-blit; river corners consult their individual shoreline bits; roads open with the sparse checker, use the fixed per-id connection table, and erase rather than paint each elbow notch. Local overlays follow public `#130`: clear `(8,8)..(183,183)`, draw the 128x128 raster at `(32,32)`, never touch the side panel, retain diagnostic text maps only in terminal presentation, and close via ordinary world redraw |
| `visibility.md` | `visibility.rs`, persistent radius-5 visibility grid / companion terrain band, persistent local-light mask, and night-time rotating beacon | persistent visibility-buffer, local-light refresh-cadence, beacon-rotation, and visibility tests | Implemented: `§12.4`'s local-light mask persists unchanged between the three published refresh triggers; refresh precedes beacon stamping and the visibility carve. `§12.6`'s bearing counter and rotating beam advance only while ambient is strictly below full daylight |

### `systems/text-output.md`, `systems/stats-panel.md`, `systems/display-driver.md`, `systems/display-driver-mode.md`, `systems/display-driver-abi.md`, `systems/overlay-abi.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `text-output.md` §1–§10 | `text_wrap.rs` fixed-cell text window, control bytes `0xFB`..`0xFF`, padded numeric printer, exact resident descriptors and cursors; `stats_panel.rs` gameplay assembly | text-wrap tests in `chunk_13.rs`, `chunk_14.rs`, including exact descriptor state, combined CR+LF, and row-below scroll copying | Implemented: window 0 is restored full-screen, window 1 is stats, window 2 is the shared message/prompt rectangle with its cursor initially at `(0, 12)`, and window 3 remains the untouched full-screen default. Scrolling does not blank the vacated bottom row |
| `stats-panel.md` §1–§5 | `stats_panel.rs` party rows, active-cursor handling, combat overlays | stats-panel tests | Implemented |
| `display-driver.md`, `display-driver-abi.md` | `crates/u5-bevy/src/lib.rs` framebuffer composition, atlas-backed top-down and first-person rasters, fixed-font shared surface, rectangle-dissolve and endgame certificate-rectangle rendering, and per-step visual route replay harness; Tandy CLI raster depth aliases route to the EGA-equivalent path while Hercules is explicitly rejected as outside v1 scope | Bevy framebuffer/story-wipe/STARTSC/endgame-transition tests; visual frame suite; visual route suite; CLI display-depth tests | Implemented with public story/menu-loader timing. `§6`/`§8`/`§9.2` back-buffer routing was corrected in `f976af0` and is the largest fix of that batch: the clipped rectangle fill (`0x3F`), the 16-by-16 tile entry (`0x51`) and the fixed-cell glyph entry (`0x5D`) each branch to a real back-buffer body, and the engine treated tile and glyph as front-buffer-only, so **both were silently discarded on the hidden surface** — which would leave the endgame and map-viewport fades dissolving stale pixels. The line entry (`0x33`) is front-buffer-only *regardless* of the selector, which is a draw to the front buffer, not a skipped draw; it was skipping. Return-to-View exact rasters are implemented from `fcc8181`; only explicitly unpublished display timing remains presentation work |
| `display-driver-mode.md` | `display_driver.rs::EgaDisplaySurface` 320x200 indexed front/back surfaces, fixed 16-colour palette mapping, render-target routing, drawing-colour state, and EGA dispatch operations | palette/index-6 tests, surface dimensions, pixel/line/fill, back-buffer copy/dissolve, tile/glyph target, and loaded-asset plane-swap tests | Implemented for the public EGA-compatible v1 target. Alternate-driver exact pixel conversion remains outside the published v1 contract; Tandy's CLI alias is compatibility presentation, not a claim of EGA-identical hardware layout |
| `overlay-abi.md` | `crates/u5-bevy/src/lib.rs` overlay composition for status/Z-stats/endgame/intro | Bevy overlay tests | Implemented |

### `systems/prng.md`, `systems/timing.md`, `systems/stat-arithmetic.md`, `systems/active-objects.md`

| Section | Evidence | Tests | Status |
|--------|----------|-------|--------|
| `prng.md` | `prng.rs` LCG; `random_*` helpers in `play_state_*.rs` | rng round-trip tests | Implemented |
| `timing.md` | `timing.rs` wait counters; integrated in clock and sailing cadence | timing tests | Implemented |
| `stat-arithmetic.md` | `stat_arithmetic.rs` saturating add/sub | stat-arith tests | Implemented |
| `active-objects.md` `§1`-`§7` | `active_object_io.rs` 32-slot table, OOL persistence, `ool_audit.rs` aggregate active-object overlay census | active-object tests across chunks plus synthetic and local-clean `.OOL` aggregate audit coverage | Implemented |
| `active-objects.md` `§4` eviction cascade | `allocate_active_object_slot` → `active_object_eviction_victim`; `active_object_eviction_phase` derived from `active_object_eviction_byte_accepted` + `active_object_eviction_phase_is_off_screen` | allocator cascade tests; the two tests that asserted the old "table full" early-out at horse purchase and Y-Yell Shadowlord install are corrected | Implemented (`a48e2ef`, trigger corroborated by `f976af0`). Previously the allocator ran phase 1 only and returned `None`, so a full ordinary range **silently dropped** horse purchases, dropped items, shipwright deliveries and encounter spawns. Phases run in published order, lowest index up within each; slot 0 and the reserved band `24..=31` are never victims, and type byte `0xB5` is rejected by every phase including last-resort phase 10 |
| `active-objects.md` `§8.1` prune pass | `active_object_type_is_prunable`, `ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT`, and the live overworld turn epilogue in `play_state_impl/chunk_09.rs` | exhaustive 256-value classifier; type-versus-tile, window, seam, boundary, trigger, and record-clear tests | Implemented. Byte 0 alone selects prunable ranges `0x2C..=0x2F`, `0x80..=0xB3`, `0xB8..=0xE7`, and `0xEC..=0xFF`; byte 1 does not participate. The pass keeps offsets `0..=31` from the scroll-base origin in each axis using unsigned eight-bit subtraction, then clears record bytes `0..=5` while preserving phase and DEP3. Slot zero is excluded and the eviction and prune mechanisms remain separate |
| `active-objects.md` `§8` outdoor walker | `play_state_impl/chunk_09.rs` high-to-low walker and staged ranged/generic reactions; `play_state_impl/chunk_07.rs` resumable post-turn reaction dispatcher; `outdoor_ranged_attack.rs` shared payload; `combat_setup.rs` class/arena selectors | production-path walker tests in chunk 06 plus whole-pass suppression, adjacency precedence, two-axis directed fallback, chance refusal, exact 65,536-pair impact-gate coverage, full `0x40..=0xFF` class coverage, arena priorities, Sand Trap, both whirlpool arms, generic combat, combat-return continuation, seam, trigger, obstruction, flight, hull, and whole-party damage | Implemented. A reaction from any slot suppresses movement for every lower slot for the rest of the pass without suppressing their reaction checks; lower slots survive and resume after terrain combat returns. Adjacency preempts ranged classes; blocked first-axis movement tries the other axis while chance refusal ends the attempt. Sand Traps are silent, both whirlpool arms apply shared impact, and the generic arm uses #103's exact impact intersection or full independent class-and-arena combat entry |

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
| `catalogs/common-word-dictionary.md` | `common_words_io.rs`, `tlk_runner.rs`, `shoppe_bark.rs` | all-128-entry table, null-reference, exact TLK/SHOPPE spacing, font, and malformed-content tests | Implemented |
| `catalogs/gazetteer.md` | `world_tables_io_locations.rs`, sidecar overlays | locations tests | Implemented (stock location entry/return coordinates are native; several non-location transition families remain sidecar-backed pending public gazetteer rows) |
| `catalogs/item-list.md` | `equipment.rs`, `containers.rs::InventoryAddClass` | inventory-add tests | Implemented |
| `catalogs/monster-bestiary.md` | `combat_stats.rs::combat_class_stats`, `combat_ranged_effect_stats`, `combat_class_traits` | exhaustive per-class stat, ranged/effect side-row, trait, and ability-hook tests in chunk 23 | Implemented |
| `catalogs/npc-roster.md` | `npc_runtime.rs`, conversation/shop dispatch | roster tests | Implemented |
| `catalogs/quest-graph.md` | `quest_flags.rs`, `endgame.rs` | quest-flag tests | Implemented |
| `catalogs/sage-rumours.md` | `shops.rs` shared paid-topic table and sage session flow | 26-row table-sync, strict topic boundary, fee/debit, SHOPPE template, and PRNG-timing tests | Implemented |
| `catalogs/spell-list.md` | `magic.rs` parser, cost, scene-mask tables | spell metadata tests | Implemented |
| `catalogs/tile-catalog.md` | `tile_classes.rs`, `view_classes.rs`, `tile_helpers.rs` | tile-class tests | Implemented |

## Remaining Public-Spec Gaps

Public issue #109 closed in `574f1d8`; the engine now implements the exact
destructive alarm and resident-entry schedule/dialogue sweeps, including PRNG
consumption and the original fixed-slot-4 defect. Public issue #108 closed in
`06494e0`; the engine implements the exact host-clock seed transform and its
gameplay re-seed sites. The same audit
removed the retracted automatic player-as-NPC mirror from town entry and town
reload paths, leaving the player solely in active-object slot zero. Public issue #106
closed in `b34ae69`; its two Blackthorn rescue calls and the shared dungeon
Search presentation tail are implemented with the exact hidden-source classes,
ordering, inclusive rectangle, blocking visit count, and zero gameplay ticks.
Public issue #107 closed in `bc0c761`; the existing allocator behavior was
confirmed exactly and is now pinned across all 256 wrapped axis separations,
the ±5/±6 boundary, player-global-vs-slot-zero source, and ignored floor.
`#84` and `#100` closed in public spec commit `9807eb4`;
`#101` closed in `abd0a17` and `19a0ba1`, `#102` in `1fedad0`, and `#103` in
`a4167b0`, `#104` in `8fc218f`, `#105` in `b1e8e08`, `#106` in `b34ae69`, and
`#107` in `bc0c761`.

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

**Every fully published gameplay contract is now implemented.** Four entries left this list on
2026-08-23 — `visibility.md §12.6`'s night beacon, `overworld.md §9.2`'s blocking
transit, `active-objects.md §8`'s outdoor walker phase, and `overworld.md §6.2`'s
ranged-attack payload, the last one to close (`cleak/u5-spec#90`).

`§6.2`'s payload is worth recording because it could not have been guessed: it is
**transport-dependent and differs in kind** — aboard a frigate the hull absorbs
the impact and no party member loses hit points, otherwise a whole-party pass
rolls **independently per living member**, one roll each rather than one shared.
It does not route through the combat damage-and-status resolver and carries no
attacker sentinel. Two bugs fell out alongside it: the serpent/dragon trigger was
one-in-**seven** against a published one-in-eight (nothing called it, so the wrong
value passed its own test), and `0xE0..0xE3` was treated as a sea-serpent family
when it is the **Sand Trap** — the sea serpent is `0x88`.

There is no remaining *fully published* gameplay contract known to be
unimplemented. Public `#131` closed the last pending combat-resistance question
in `60ac944`; the conservative placeholder has been removed.
The public `#11` corrections to protected Kill targets, the shared
Cause-Fear/Repel-Undead one-HP fleeing writer, and the exact
Conjure/Swarm/Summon probe, count, and controlled-bit outcomes are implemented
as well. Public `#132` closed the protected-target presentation envelope in
`1e28720`; it is now implemented and regression-tested.
The 2026-08-24 town-transition audit replaced the obsolete decimal
`0x50..=0x57` two-way ladder model with the published directional K-Klimb
links (`0xC8` up, `0xC9`/`0x86` down), cardinal climb-over targets
(`0x4C`, `0xCA`, `0xCB`), exact turn costs, and the post-turn `0x8C`
trapdoor path. Trapdoors now apply independent `1..=8` damage to each non-Dead
party slot, respect magic-carpet suppression, and use the same full reload as
stairs. Stonegate's #123 exception instead stays on the current floor, fills all
1,024 live cells with `0x8F`, clears all 32 active-object records before the
coordinate-only slot-zero tail, and sets every in-party member to zero HP and
Dead without inventing a durable imprisonment flag. Its direct black viewport
fill and published speaker envelopes are retained in typed transient playback.
The 2026-08-24 all-48-spell production audit corrected the one apparent
counterexample rather than treating its existing code as evidence: Vanish,
Open, Magic Lock, and Unlock Magic were all routed to an obsolete combat
failure substitute, non-combat Vanish inspected dynamic objects instead of the
published thirteen live tiles, and Open used Unlock Magic's `0x97`/`0x98`
mapping. One shared live-tile helper now owns the exact town/combat rewrites,
Open's kind-1 chest-bit arm, acting-combatant origin, spend-before-follow-up
ordering, non-escapable poll, and Space/Pass completion.
The event-driven runtime fuses `main-loop.md §4`'s outer dispatch with one
inner-loop input iteration: `handle_play_key_input` classifies the resident scene
byte through `scene_route`, dispatches exactly one world/town/dungeon mode, and a
transition is observed on the next input. Section 14 explicitly permits the
historical exit-pending flag to collapse into that natural control flow, while
the single-directory disk-prompt presentation pass is a no-op. The corridor's
wall/scenery selection is likewise the seven-family billboard table published by
#84, not the withdrawn sparse-coordinate-table interpretation.

The corridor resource loader distinguishes an intentionally graphics-free
fixture from a partial or malformed install. Present `DNG*`, `ITEMS`, and
`MON0`-`MON7` resources must match their published slot counts and dimensions.
The sprite header is now decoded as a sprite count rather than an offset count;
the former parser silently exposed only 10 of 20 `ITEMS` sprites and 3 of 6
sprites in every monster bank.

### Presentation boundaries closed by explicit policy

Public issues `#136`–`#139` close the four former residuals. The endgame gate
is the shared deterministic moongate row splice, not a brightness operation.
Rectangle dissolves publish atomically with no normative wall-clock duration.
EGA is the sole pixel-exact v1 target, so alternate-depth conversion is not a
completion requirement. The title flourish uses the final normative modern
cadence of exactly 14 ms per each of 85 presentations (1.190 s nominal total),
with the published captured-frontend tolerances.

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
reference count, not a reading. The last of them, the outdoor walker's first
phase, is now connected to the production post-turn pass and covered through
that path.

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
- The two shop-owned side-panel shells use the published window-1 clear,
  widened frame, border-cell rows, and window-2 restoration. Public issue
  `#119` / commit `58e9b9c` supplies the arms `S` browser paging, row content,
  controls, continuations, and draw timing, now implemented. Public issue
  `#121` / commit `5b9445f` supplies the exact three-cell page-status badge,
  also implemented through the shared two-colour chrome-cap painter rather
  than the later one-colour text overlay. That compositor now also connects
  the published `Arms`, `Select:`, and `Items:` stats-ribbon labels to live
  browser and selector state.
- Return-to-View effect rasters are no longer deferred. Public issue `#117` /
  commit `fcc8181` publishes the exact row splice, 256-position permutation,
  checkpoint cadence, source selection, opaque-zero rule, and abort state; the
  runtime and Bevy playback path implement them. Public issues `#118` and
  `#120` are resolved; subtitle ignition now pins the exact pass/poll/audio
  cadence and Galois/gate vectors from commits `12485b3` and `36780cb`.
- Local View and dungeon-minimap control-flow/pixel contracts are no longer
  deferred: `view.md §4`/`§6` and `dungeon-mode.md §12` publish the class
  strokes, river and road rules, glyphs, vectors, flood order, and bounds.
  Empirical screenshot comparison remains QA rather than a missing contract.

## Conclusion

Across the current public specification tree, including the boot, launcher,
disk-prompt, display-mode, shared-dictionary, and sage-rumour contracts omitted
from the older inventory, every gameplay-correctness deliverable that has complete public data
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

Verified on 2026-08-24 in the current worktree: 3223 u5-runtime, 183 u5-bevy,
and 103 u5-tui tests pass, `cargo fmt --all -- --check` is clean, `cargo clippy
--workspace --all-targets` reports zero errors (its existing style-warning
baseline is not gated), `--route-smoke` passes all 513 cases,
`--visual-frame-suite` writes 193 PNGs and `--visual-route-suite` writes 1906.
Asset-backed verification used the local asset directory read-only.
