# Status Matrix

This matrix summarizes current implementation status against the active
full-game goal. It is intentionally evidence-oriented: passing tests are useful
only for the behavior they actually cover.

Last refreshed on 2026-08-24 in the current worktree after reconciling public
issues through `#144`, correcting slot-zero transport-marker serialization,
and routing active-object bytes through the actor half of the atlas. Public
issue `#112` resolved the stale spec prose in `8a73d12`; runtime behavior
already follows the settled `#94` contract. Public commit `82daf8d` resolves
`#113`: Ready charges exactly once per invocation, at nominal 2/1/1-minute
world/town/dungeon cost with shared Quickness and Negate Time modifiers.
The shared exploration party-capability gate is now live in runtime, TUI, and
Bevy: it scans ascending for Good/Poisoned, paces `Zzzzzz...` sleep passes
without accepting input, runs town's independent wake rolls before underfoot
effects, skips the dungeon post-action hook, and sends total defeat—including
Stonegate's scripted wipe—through the ordinary rescue on the next iteration.
Public issue `#124`, resolved by clean-spec commit `d3863ef`, makes overworld
sleep a full ordinary two-minute turn and writes the unchanged complete live
object table to the current plane mirror before overworld rescue. Dungeon
defeat's transient teardown is already satisfied by the engine's permanently
resident ordinary atlas and lack of dungeon-only graphics-bank references, so
it draws and mutates nothing before the immediate shared rescue. The TUI and
Bevy route suites now exercise that overworld defeat write through the real
command-boundary gate and validate a synthetic far-slot record after rescue.
Public issue `#125`, resolved by clean-spec commit `0a0b867`, corrects combat
field contact to the common post-dispatch actor tail. The acting descriptor is
the target, only its linked renderer record is skipped during the ascending
32-record marker scan, Poison/Sleep/Fire are passable and non-consuming, and
Energy is blocking with no contact arm. Exact conditional PRNG consumption and
direct raw Poison/Fire damage are implemented for both player and automatic
dispatches. Public issue `#126`, resolved by clean-spec commit `b600bc6`, adds
the terrain-first arms: exact `0x04` is Poison and exact `0x8F`/`0xBC` are Fire,
with all other terrain bytes falling through to the marker scan. The selected
terrain arm suppresses markers even when Poison is rejected. Doom absorption is
a separate earlier committed non-digit player-action check against companion
row 1 while the actor stands on row 2; digits, refusals, and automatic actors
bypass it.
Public `#141`, resolved by clean commit `74e5053`, replaces the town return
snapshot model with the published canonical-object lifecycle. Every published
foot/horse/carpet/frigate/skiff marker enters unchanged and without an outdoor
turn; entry writes all 32 live slots to the current plane `.OOL`; exit uses the
fixed scene coordinate and scene-`0x19` plane rule, reloads all 32 destination
records, retains slot-zero auxiliaries, and only then places queued shipwright
delivery. The accepted path owns no cached pre-entry coordinate, plane, marker,
grid, or object snapshot. Failed coordinate lookup consumes the normal
two-minute outdoor action.
Public `#142` and `#143` are implemented from `fb05888` and `5933b35`: Push
and all 40 stock Enter routes now use their exact transcripts, failures,
ordering, and action results. Public `#144`, resolved by clean commit `5cba5e9`,
corrects the Shrine of the Codex approach to an ordained passage and an
unordained two-line refusal followed by the south push. The same audit removed
the retired `0xFC` player sentinel: slot index zero alone identifies the
player, both slot-zero bytes carry the saved transport marker, and companion
bytes render at actor-atlas index `byte + 256`. Combat party records now use
the four published class actor bytes rather than a shared foot marker.
The older World-row `open-south-step` / `ordained-block` labels and its
495-case count are superseded by the current 514-case run and the
`unordained-refusal` / `ordained-passage` routes recorded above.
Public issues `#127` and `#128` are resolved and implemented from `21698d6` and
`98dfd45`: the certificate uses exact one-cell TH/ST/space encoding and encoded
centering, while conversation gold payments use automatic affordability,
in-place success, the exact refusal line, and nested-loop stop propagation.
Public `#129` is resolved and implemented from `a7e55bf`: monster-special gates
use a lazy shared-PRNG cascade, exact contiguous 32/256 acceptance, fresh summon
X/Y draws, one placement attempt, handled-turn termination, and failed-summon
ordinary-AI continuation. Public `#130` is resolved and implemented from
`e335918`: local View clears `(8,8)..(183,183)`, draws its 128x128 raster at
absolute `(32,32)`, leaves the side panel and message window untouched, and
closes through ordinary world redraw. Public `#131` is resolved and implemented
from `60ac944`: party-owner Intelligence and monster-class endurance feed the
signed unclamped score; equality lands; every shared caller draws the skewed
roll once; and Tremor/Poison Wind use the separate target-weight gate.
Any older long Rendering-row phrase that says local View overlays compose into
the side panel is superseded by this correction: only the settled `#130`
gameplay-viewport composition is current.
Any older long Combat-row phrase that says `#131` blocks possession resistance
is likewise superseded: the placeholder is gone and the full `60ac944`
predicate/caller census is current.
The final public `#11` correction likewise supersedes older fear/repel and
conjuration wording: Kill rejects class ids 14/15/47; Cause Fear and Repel
Undead directly write 1 HP and fleeing bit `0x02`; Repel does not kill or
credit XP; the conjuration family shares whole-candidate `0..=15` probes;
Swarm places four actors at one cell; and only successful party Summon stamps
controlled. Public `#132` closed in `1e28720`; protected Kill rejection now
spends the shared charge and 7 MP, skips resistance PRNG and target effects,
reports `Failed!`, and commits the combat action without reopening a prompt.
Public `cleak/u5-spec#109` closed in `574f1d8`; its destructive ordinary-alarm
and resident-Shadowlord pursuit/flight schedule rewrites, dialogue sentinels,
all-floor/all-index scope, exact shared-PRNG draws, and fixed-slot-4 defect are
implemented. Public `#108` closed in `06494e0`; its exact host-clock PRNG seed
equation and gameplay re-seed timing are implemented.
Public `#107` closed in `bc0c761` and confirms the allocator's exact wrapped-byte inclusive ±5
screen predicate, current-player coordinate source, and floor-independent
comparison. Public `#106` closed in `b34ae69`; the two Blackthorn rescue calls and
the three reaching lit-dungeon Search outcomes now run the shared blocking
`(8,8)..(183,183)` dissolve in their published order, while bombs, narration-
only Search, darkness, and every Open outcome bypass it. Both frontends now
acknowledge those completed blocking-call records when they present the final
caller-composed state. Public `#103` closed
in `a4167b0`; the engine implements its exact generic-adjacent impact gate,
type-only combat classes, independent terrain/transport arena selection, and
high-to-low continuation after returning combat, alongside the Sand Trap,
whirlpool, ranged-reaction, and `#102` prune paths.
A fresh audit of closed issue `#79` corrected the resident gameplay text
descriptors and control semantics: window 0 is full-screen, window 1 is stats,
window 2 is the shared message/prompt rectangle with its cursor initially on
the bottom row, window 3 remains unused, line feed is combined CR+LF, and
scrolling copies rather than blanks the vacated bottom row.
A full public-tree inventory then added the previously omitted boot, launcher,
disk-prompt, display-mode, common-word, and sage-rumour documents to the audit.
The shared-word pass corrected both consumers: TLK now preserves exact
leading/pending spacing and per-glyph ordinary/runic font identity through the
live transcript, while SHOPPE applies its separate lookahead spacing rule and
rejects a referenced empty dictionary slot as malformed content.
The next renderer audit corrected `view.md §4` production behavior that old
self-consistent mask tests had preserved: deep water is no longer a no-op,
river corners select secondary versus modal sources from their individual
shoreline bits, and roads use the seven published connection masks plus blank
elbow notches. The three tile-class lookup copies now share one canonical
table.
The corrected `view.md §4.2` telescope/spyglass path is implemented as a true
sky renderer: the daylight branch selects and damages a party member without
painting, while the night branch consumes exactly 160 PRNG draws for 80 stars,
steps eight bodies from the saved calendar, and overlays the live Shadowlord
locations. The withdrawn party-centred Britannia chunk-map stand-in has been
deleted.
Any older evidence prose in the long matrix rows that says "Spyglass
Britannia chunk-map", "Spyglass chunk-map", or "Britannia-overview" names the
superseded suite case; the current case is `britannia-spyglass-night-sky`, and
the current overlay mode is `SkyView`.
The endgame certificate
was the last gate on an unpublished contract, so the **victory ending is
reachable and renders end to end**.

**There is no longer any `panic!` in `crates/` that stands for an unimplemented
published contract.** Every refusal that remains is structural (a graphical
screen with no terminal surface) or an injection guard. See
`docs/completion-audit.md`, "Refusals that remain".

Earlier refresh note, kept for context: the six-package presentation-parity pass
(intro title/menu, gameplay screen chrome, visibility/lighting, command echo
transcript, intro story slides, endgame/chargen/U4/harness) landed on
`intro-preflourish-phase` on 2026-08-22. Numbers in the rows below were
re-measured on 2026-08-23 against a copy of `C:\Games\U5-Clean`; see "Current
Verification Baseline".

Read spec contracts from `cleak/u5-spec` on GitHub — the issues, and the
document text through
`gh api -H "Accept: application/vnd.github.raw" repos/cleak/u5-spec/contents/<path>`.
Do **not** read the local checkout: `C:\Projects\Rust\u5-clean\u5-spec` is
read-only from this workspace and is stale at `9a898d1`, many commits and
several retractions behind spec head. Checking any of the rows below against
those local files would disagree with this document.

| Area | Current status | Evidence | Remaining risk |
|---|---|---|---|
| Intro/menu | Terminal and Bevy intro shells use the runtime menu dispatcher. The Bevy title sequence renders `TITLE.BIT`/`BRITISH.BIT` through the published whole-page publish phases, drives the `BRITISH.PTH` signature path, and runs the corrected `#67` flourish script (85 presentation steps at the `#77` 14 ms cadence, palette index 9). The menu screen is `ULTIMA.16` slot 0 (the logo) over the measured rounded blue chrome, with the four title-tick flame bands from slots 1..=4; the withdrawn `#52` procedural flame stripe and `#63` box-glyph frame are gone. Story slides render all 21 steps from observation-derived proportional metrics, including step 6 from the published `#69` doorway text, and the `#53` reveals are the driver's rectangle dissolve rather than the withdrawn column sweep. Acknowledgements plays the published `#72` phase sequence: the credits page is composed on the hidden surface, the two `STARTSC` pillars rise up the screen centre over the live menu, eighteen paced steps part them outward to publish the 16 + 288 + 16 by 137 credits artwork on rows 63..=199, the menu is rebuilt on the hidden surface with the Acknowledgements row highlighted while the credits are still displayed, and any key (including `Esc`) runs the mirrored close and sink phases that publish it back. The withdrawn horizontal slab-wipe model is gone, and the `ULTIMA` logo rows are never touched. Return-to-View plays back the published `#54` preview. Chargen renders the fixed-cell name/gender prompts at the published `chargen.md §5.1` cells into the live menu window without clearing it, then the CREATE panels with measured paragraph boxes. `U4` transfer validates its source from the `#16` `PARTY.SAV` offsets and treats missing or unreadable media as the retryable branch (message, key, back to menu, nothing written); a valid source now draws the published `#73` preview screen (`crates/u5-runtime/src/u4_transfer_preview.rs` owns the window rectangles, prompt-frame cells, both panel geometries, the eight-row field-label column, the `§6.3` pages and the stage machine; `crates/u5-bevy/src/u4_transfer.rs` composites them onto one persistent surface edited in place, since `§6` has no double buffering and no page swap). The U5-side seed pair is `INIT.GAM`/`INIT.OOL`; `BRIT.GAM` was withdrawn by `#88` and does not ship at all. | `intro_menu`, `menu_dispatch`, `chargen`, `u4_transfer`, `u4_transfer_preview`, `story_layout`, runtime PTH parser tests, TUI binary temp-directory smoke for empty-save Journey Onward, deterministic Create Character followed by `--from-save --play-script`, Bevy intro title/flourish/tick/menu/chrome/story/acknowledgements/chargen-prompt/U4-transfer tests, and the `--visual-frame-suite` intro PNGs (`intro-menu`, `intro-finished-menu`, `intro-story-00`..`-20`, `intro-return-to-view`, `intro-chargen-name-prompt`/`-gender-prompt`/`-gender-echo`, `intro-acknowledgements-risen`/`-parting`/`-credits`/`-closing`, `intro-u4-transfer-found`, `intro-u4-transfer-panels`). | No open spec gate remains on this path: `#72` and `#73` are both closed and implemented, and `#78` was answered in the same sweep. Terminal `--intro` deliberately refuses story, Return-to-View, acknowledgements and transfer — they are graphical screens with no terminal surface, and the refusal is by design, not a gap. |
| World mode | Movement, vehicles, hazards, moongates, Gate Travel, plane transitions, native and sidecar encounters, active objects, fixed hidden-treasure search/pickup persistence, save/load, published stock location entry/return coordinates, Shadowlord native save slots/rerolls, public issue #32 Word-of-Power seal opening, public issue #31 Eternal-Flame-gated shard destruction through the three published native positions plus the matching encounter-north handshake, and many commands are implemented. Hoisted-sail movement follows the public wind wait-tick table, including the two-wait into-wind case. Natural moongate live-tile refresh and entry use refreshed cached moon-glyph bytes from the public hour table, including public issue #38 Felucca phase-0 glyph entries at hours 10/11/19/20. Gate *appearance* now follows the `overworld.md §9.1` presence-phase model in `crates/u5-runtime/src/moongate_phase.rs`: the presence counter is a sixteen-step global position, not an on/off flag; phase 0 draws the ground plate, `1..15` draw a composed frame whose bottom N pixel rows are the top N rows of the moon-gate tile through scratch tile `0x116`, and phase 16 draws tile `0xDC` on the ordinary tile path. The ground plate is grass in play and `0x44` in the endgame scene. The counter is persistent save-backed state at `SAVED.GAM` offset `0x02E1` — it was previously initialised to zero in every constructor and dropped on save. The withdrawn per-render-frame moongate animator is deleted, along with `MOONGATE_ANIMATOR_DAYTIME_THRESHOLD`, whose gate was both inverted and misattributed: it belongs to the `visibility.md §12.6` night-time light beacon, which runs only after dark and never holds a moongate. Active-object allocation runs the published `active-objects.md §4` ten-phase eviction cascade when the ordinary acquisition range is full; the allocator previously ran phase 1 only and returned `None`, so a full range silently dropped horse purchases, dropped items, shipwright deliveries and encounter spawns. The public Britannia chasm at `(54, 138)` preserves transport, applies Dex-gated one-HP fall checks, and transitions to the Underworld through the native route; ordinary water movement is one-cell transport movement with no current-sweep sidecar; the forced whirlpool branch lands at `(34, 18)`, and the fixed narrative gate route covers both open south-step and ordained-block outcomes. | `cargo test -p u5-runtime`, world tests across chunks 03, 06, 07, 10, 12, 13, 15, 17, 23; focused sailing, Gate Travel, fixed hidden-treasure, chasm, whirlpool, fixed narrative gate, Word-of-Power seal, shard/flame U-Use, natural-moongate cache, retired-waterfall-sidecar ignore coverage, and exhaustive all-40 public world-location entry/restore tests cover wind cadence, live-gate entry, live-gate clearing, cache refresh, exact native Eternal Flame coordinates, cached-byte-only dispatch, and stock location table round trips; `--route-smoke` passed 495 cases, re-run against a copy of the asset directory on 2026-08-22, including all 40 published stock world-location entry rows, native shrine/Codex quest routes, TLK-backed conversation routes across all 32 named-location scenes, save/reload checkpoints for boarded horse, Gate Travel, chasm fall, fixed hidden treasure, horse-trader delivery, ship X-it/skiff, dungeon ladders, and dungeon exits, plus Spyglass Britannia chunk-map, H-Hole-up rest, utility U-Use items, ship broadside fire, horse boarding, Gate Travel success/refusal routes, saved-slot natural moongate live-entry/clear routes, public chasm fall route, forced whirlpool Underworld branch, fixed narrative gate open/ordained-block routes, fixed hidden-treasure zero-key/single-use/daily/stacked routes, public #31 native shard/Eternal Flame destruction routes, public #32 Britannia/Doom Word-of-Power seal routes, public #48 Blink ray landing, Create Food casting, Locate, In Lor/Light/Open, restore-spell, active-effect, all-cardinal directed Sleep/Poison Wind/Death Wind/Flame Wind combat casts, combat field marker casts/removal, combat directed-utility tile casts, special death-marker Kill casts, combat-entry party descriptor routes, combat terminal cleanup routes, dungeon level, dungeon field/dispel, dungeon Open chest, Mix/Ready/New Order workflow, hourly provision/poison/starvation/ring routes, dispatcher refusal routes, public #44 sleeping/praying Talk refusals, native town walk-on stair routes, dungeon-to-world return restoration, dungeon torch ignition, public #21 active-monster attack/contact ambush routes, public #51 poison-gas step, public #47 dungeon no-direct-recovery rest and completed long-camp recovery, Shadowlord town-entry/Yell/Stonegate routes, Blackthorn audience/rescue routes, active shop/modal routes including all-nine-tavern public #13 lore selectors, accepted shipwright dock deliveries and all nine public arms-shop first-stock purchase and terminator-refusal rows, and terminal endgame confirmations plus missing-box jitter and full victory cinematic routes. | Some moongate/plane-transition coordinate coverage still depends on published rows or sidecars. `overworld.md §9.2`'s blocking moongate transit presentation is now implemented (`8d41816`) — a 256-step dissolve reusing the shared `DissolveVisitOrder` primitive, then a 15-to-1 countdown at two BIOS ticks per phase. The `active-objects.md §8` outdoor walker's first phase is implemented too: adjacent hostile engagement and the whirlpool transition already fired from the post-turn effects pass, and the two **ranged** reactions landed with `cleak/u5-spec#90`'s payload — transport-dependent, differing in kind, with a frigate's hull absorbing the impact and every other state rolling independently per living member. |
| Town mode | Movement, NPC schedules, stairs, trap doors, grid-boundary exit prompts, doors, pickups, native inn bed H-Hole-up gate, talk, shops, Blackthorn paths, alarms, Search object-table pickups, Search trap narration, object-table chest contents/traps, native tile `0x04` post-step/pre-clock poison-gas doorway rolls, public issue #43 top-down fountain/wishing-well/death-vision/framed Yew wanted-poster Look specials, sanitized all-shipped-town-family `LOCATION.DAT` authored-cell aggregate audit coverage, and save/load are implemented. The withdrawn tile-`0x59`/`town_exit_tiles.tsv` exit path is removed; `0x59` remains the telescope Look trigger. Clean spec issue #110 fixes every edge to the transport-sensitive `(31,31)` terrain sample and true out-of-grid occupancy coordinate: blocked terrain, `N`, and Escape consume a normal town turn, while `Y` exits without one. NPC schedules preserve movement state after boundary hours, use `0xC8`/`0xC9` multi-goal floor-link routing, and keep slot zero as the player's sole representation; the retracted automatic player-as-NPC mirror has been removed from town entry and reload paths. A matching hideout town now records its resident Shadowlord, honors the row-4 suppression and one-at-a-time gate, installs the stationary `0xFC` descriptor at the highest free NPC index (with index-31 overwrite fallback), links it through the ordinary active-object allocator at the published per-town cell, applies the day-keyed crop/orchard blight before replacing the shared stream with a fresh host-clock seed, and ends successful Hatred/Cowardice installs with the exact 32-draw destructive roster sweep. Ordinary alarms rewrite every occupied roster slot across all floors, special-case exact types `0xFC`, `0xD8`, and `0x70`, and use persisted mode-3/6/7 schedules plus `0xFD`/`0xFE` Talk sentinels rather than synthetic alarm-state markers. Those public destructive schedule/dialogue fields survive save/reload through a narrow engine-owned mutation ledger while original `.NPC` assets stay read-only. The free-roaming town-object pass scans `0x10`/`0x11` records in ascending slot order, consumes the published chance/axis/sign draws, applies only the exact `0xA2`/`0x43` pen blockers, and defers map-edge rejection until after direction selection so valid inward edge moves remain possible. Yell summons retain their separate name handshake across player synchronization, and acquiring an empty active-object record detaches any stale off-floor NPC link before later schedule ticks. | Town tests in chunks 04, 06, 10, 11, 13, 15, 19, 21, 22, 23, 24 plus focused resident-Shadowlord guard/row/allocation/full-roster/summon-handshake/blight/sweep tests, fixed-slot-4 defect and exact 32-draw coverage, exact ordinary-alarm special/draw/all-floor routing, destructive pursuit/flight field preservation, mutation-ledger round-trip/reload/malformed-row coverage, free-roaming eligibility/floor/PRNG/pen/terrain/occupancy/edge-boundary coverage, `0xFD`/`0xFE` Talk/contact tests, Yell stale-link/player-sync ownership regression coverage, exact host-clock seed vectors, `scheduled_npc`, slot-zero scheduler/relink/Talk/attack skip coverage, shipped `.NPC` boundary-hour/multi-floor scheduler corpus coverage when local clean assets are present, shipped `LOCATION.DAT` aggregate owner/class/view audit coverage when local clean assets are present, `town_search`, `object_pickup`, native inn bed/no-inn refusal H-Hole-up tests, `town_surface_fountain`, `town_surface_wishing_well`, `town_look_routes`, chest/trap filters, native poison-gas and retired-sidecar non-trigger tests, route-smoke native walk-on stair up/down/crossing coverage, all three native shard-destruction routes, route-smoke/visual-route town attack death-mask removal, guard alarm, hostile adjacent alarm, guard arrest refusal/surrender coverage, and route-smoke public #51 poison-gas step coverage. | Per-cell semantic interpretation, full schedule/AI audit, and richer visual presentation remain. |
| Dungeon mode | Facing-relative movement, fields, traps, room combat handoff, public issue #12/#19 `DUNGEON.CBT` party/source placement, ordinary setup classes, non-Doom special auxiliary-byte post-write rules, `0xEC..0xEF` random-special pre-roll selectors, `0xF?` room-trigger handling, navigable `0xA?` room-helper state with source-scan suppression, non-walkable `0xE?` heavy-door presentation variants, public issue #21 active-monster ambush combat handoff, Jimmy/Open/Get/Search chest handling, generated chest rewards, teleports, exits, ladders, and save/load are implemented. Public #101's call-site-controlled fresh/reuse wandering-monster setup is wired for direct entry, accepted level changes, combat return, loaded/resumed reuse, and stats-view preservation. Its eight-byte record mapping treats family zero as valid and `dep1 == 0xFF` as authoritative inactivity, and the record round-trips through the main save. Lighting follows the corrected contracts: the ambient byte is the squared-distance threshold rather than a radius, and public issue #42's local light is a squared-distance disc (`dx*dx + dy*dy <= 10`, 37 cells) rather than the withdrawn Chebyshev reading. Public #106's lit Search tail now narrates, rewrites exact `0x61`, the rewriting `0xC?` skeleton branch, or `0xD?`, composes the changed first-person state, and exhausts one blocking viewport dissolve before returning. | Dungeon tests in chunks 05, 12, 13, 18, 20, 21, 23, 27 plus route-smoke and Bevy visual-route coverage for the `0xE?` blocking branch; focused parser/renderer tests pin billboard and masked-sprite bank shapes, object-family selection, fresh placement success/failure, family-zero activity, save/resume preservation, field geometry, decoration tones, ordinary/Negate Time monster pose rules, exact viewport-dissolve coverage, all three reaching Search branches, and the darkness/bomb/narration-only/Open bypasses. | The corridor draws the published flavour-selected `DNG1`/`DNG2`/`DNG3` billboard families and public #100's `ITEMS`/`MON0`-`MON7` backward pass. The sprite header is decoded as a sprite count, masks composite with set-bit transparency, and malformed present banks fail at load. #84's retracted discriminator is gone. #101's forced poses and exact four-band tone/delay contract are implemented; tone generation remains silent because the modern frontends have no PC-speaker backend, while the specified delay cadence remains represented by the typed presentation sweep. |
| Combat | Combat frame setup/restore, player commands, monster AI, spell paths, fields, damage/status/death, victory/defeat restoration, and special handoffs have broad tests. Recent work covers public issue #5 terrain/dungeon-room party placement plus descriptor owner-link seeding, public issue #6/#7 controlled/fleeing bits, public issue #8 non-party sleep storage plus own-turn 1-in-17 wake dispatch, public issue #9/#22 cardinal directed wind-cone targeting, controlled non-party player dispatch, summoned actor control flags, Conjure/Swarm/Summon placement streams, the exact combat live-terrain rewrites for Vanish/Magic Lock/Unlock Magic/Open, temporary default death/drop markers, party corpse markers, vanish-on-death actor clearing, Gazer/Gargoyle special-death markers, public issue #125 common post-dispatch field contact with acting-slot targeting, linked-renderer-only skip, exact conditional raw-damage draws, passable Poison/Sleep/Fire, and blocking-only Energy, public issue #126 exact terrain-first Poison/Fire contact (`0x04`, `0x8F`, `0xBC`) plus the separate earlier Doom companion-band absorption check for committed non-digit player actions, public issue #10 arena-cursor field targeting and no-random-gate marker placement, public issue #3 terrain-combat resident spawn counts, all sixteen asset-backed `BRIT.CBT` outdoor arena placement records, and one-in-nine early-spawn replacement-tile path, public issue #21 stock-floor dungeon active-monster ambush arena, combat-local ambush/camp reveal-slot records with trigger consumption and one/two-cell terrain stamping after committed actor movement, combat round-counter wrap with combat-safe one-minute clock advance, the combat-only cursor-blink tick, the `magic.md` runtime tag `T` Negate Time combat gate (the automatic actor driver returns immediately, skipping every self-acting actor's turn while the party is still prompted normally — we previously had no combat gate at all, and the gate sits past the `PlayerReady` arm so the party's own dispatch is untouched), renderer-facing cursor/secondary-marker hooks, wound morale/flee movement, Saduj/name faction grouping, Doom/Shadow Lord suppression bypasses, non-party Sleep Field disable state, blocked-arena-cell dispatch skips, and public #129's bounded monster ability hook (`possess -> blink -> summon-daemon`) rather than a general AI script runner. That hook now draws lazily after Pass-1 gates, accepts exact `0..=31` blink/summon gates from `0..=255`, draws fresh summon X then Y in `0..=15`, attempts only that cell, ends handled turns immediately, and continues ordinary AI after any summon failure. | Combat-heavy tests in chunk 23 plus focused `combat_ai`, `combat_actor_slot_dispatch`, `combat_ambush_reveal`, exact terrain contact classification and priority, Doom action-tail timing, `arena_field_contact`, `combat_field_placement_callback`, `directed_spell_status`, `combat_monster_default_death_materializes`, `combat_round_counter`, `combat_cursor_blink`, combat marker rendering, directed-utility live-tile tests, Bevy visual frame-suite combat arena/death-marker gallery, `summon`, `conjure`, `swarm`, `cause_fear`, `combat_setup`, and `terrain_combat` filters; chunk 23 now exhaustively pins every published combat stat row, ranged/effect side row, exact gate boundaries, Pass-1-before-special PRNG order, one-probe summon success/failure paths, handled-turn termination, every public replacement row, and every local clean `BRIT.CBT` outdoor arena placement coordinate inside the visible 11x11 arena. | Public #131 blocks replacement of the conservative possession resistance-result predicate because the shared score formula/comparison is not yet published. Full parity audit is also still needed for monster class behavior and combat visual presentation; human replacement-byte visual review, exact resident post-round marker pixels, and remaining combat visual details should continue to be checked against published details. |
| Magic | All 48 parser rows route through implemented, scene-gated, or correct-refusal paths; major combat spell families are implemented. The `magic.md §7`/`§9` four-bit scene allow mask is now **enforced**, reporting `Not here!` ahead of charge consumption, through one gate decision (`cast_dispatcher_gate`) that the live `cast_spell_resource_gate` routes through. The crate previously modelled the contract twice and disagreed with itself: `constants.rs` carried the transposed legend `catalogs/spell-list.md §4` withdraws, and `cast_dispatcher_gate` had no production caller. Two live defects fell out of the merge — Blink was exempt from the central scene gate (published `C/O`, so indoor and dungeon casts now refuse before the direction prompt), and X-Ray used an area-only check blind to `combat_active`. The all-48 production audit then found a second stale family: Vanish, Open, Magic Lock, and Unlock Magic were still routed to a retracted combat-failure substitute; Vanish used a broad dynamic-object range instead of thirteen live tiles, and Open used Unlock Magic's magic-lock mapping. One shared helper now applies the exact town/combat tile maps, acting-combatant origin, Open chest-bit arm, spend-before-follow-up order, non-escapable direction poll, and quiet Space/Pass result. The level gate is level-vs-circle by construction now that the circle is re-derived from the spell id. All 48 mask rows were checked against the published `Allowed` column. Rel Hur follows the public prompt-to-wind table, Heal uses the public shared-PRNG `0..=60` roll/halved recovery path, Create Food uses the latest public tiny PRNG grant, and Blink follows the public #48 non-combat ray-to-farthest-grass rule plus combat target-cell movement. The live combat-cast absorption gate now uses the shared active-effect predicate, so only a live `N` code/duration pair short-circuits before resource consumption; a stale `N` tag with a zero duration no longer absorbs casts. Protection remains timer/display state only, matching the published original-game defect: the invented live `+3` party spell-defense consumer, constant, helper, and misleading tests are removed. | `magic.rs` exhaustive directed-utility tile tables, cast/follow-up tests in chunks 16, 17, 18 and 23, runtime full suite, TUI asset-backed route smoke, and Bevy visual routes. | Some non-load-bearing visuals remain clean substitute overlays rather than exact presentation. |
| Rest/camp | H-Hole-up prompt flow, native town inn bed gating (`0x48..=0x49` in published inn scenes, sidecars only as overrides), watch prompt validation, status restoration, no-direct-recovery ordinary rest, completed long-camp recovery, and Lord British camp event are implemented. Wilderness camp advances in five-minute ticks, regenerates rings once per tick, skips hourly provision/poison/starvation maintenance, and probes interruption only when the hour changes; a successful probe installs a fresh host-clock seed before selecting the ambush row. Dungeon rest retains its separate cadence. | Rest, camp, long-camp recovery, native inn bed/no-inn refusal, sidecar override, exact wilderness hour-boundary/reseed and ring-cadence tests in chunks 01, 03, 13, 19 plus route-smoke and Bevy visual-route H-Hole-up, public #47 dungeon no-direct-recovery rest, completed long-camp recovery, and hourly provision/poison/starvation/ring coverage. | Remaining risk is visual/pacing parity around rest/camp prompts, not the #47 recovery contract. |
| Quest/Codex | Shrine meditation, ordained/Codex masks, Codex urn sidecar reads, virtue-page stamping, shrine Codex turn-in rewards, all-virtues-complete detection, the menu-level Codex challenge, Blackthorn audience/rescue paths, Shadowlord shard/flame progress, Word-of-Power seals, and terminal endgame checks are implemented. | `codex_challenge`, `menu_dispatch`, `shrine_virtue`, `karma`, `quest_flags`, `endgame`, `blackthorn`, `world_tables_io_locations`, and chunks 13/16/17/21/23 tests; route-smoke and visual-route coverage now exercise all eight native shrine meditations, a Codex urn read, a shrine Codex turn-in, a completed-shrine offering, Blackthorn, Shadowlord, shard/flame, Word-of-Power, and terminal endgame paths. | Exact shrine standing persistence and broader content-specific NPC quest side effects still depend on further public spec coverage. |
| Shops | Arms, healers, inns, taverns, sages, reagent sellers, guilds, shipwrights, horse traders, and companion flows are modeled. Scene-specific shop dispatch uses the published per-kind rows, including the nine public arms-shop stocked `a..h` rows; scene misses fall through to no-shop feedback. Taverns use the public #13 round/secondary/provisions/lore selector table, retain the explicit Anything-else Y/N state, render state list/follow-up records and one random provision quote record, apply the speaker-Intelligence price and 25-food pack size, distinguish completed, gold-exhausted partial, no-need, and one-food charity outcomes, and gate both lore and the Falsehood surcharge on the published continuation result. Arms buy quotes use the public #41 equipment-id to SHOPPE.DAT record mapping; sage fee quotes, short-funds refusals, paid success barks, and published guild/reagent/healer/horse-trader Talk-entry preambles render from `SHOPPE.DAT` with public PRNG timing. Inn rest/leave/pickup prices use the public #15 Intelligence-adjusted formula, paid inn rest applies class-based HP/MP recovery, public #23 shipwright purchases queue delivery at the hosting scene's published exterior dock coordinate, public #28 has replaced the old stationary-display purchase model with horse-trader placement, active shop states render through the published Talk-to-shop inherited window-2 handoff, and #62's closed contract supplies the per-state transcript/wait/clear behavior plus inn Pickup and arms Sell window-1 panels. | `shop_session`, `shop_runtime`, chunk 13 pricing/table tests, end-to-end talk/shop tests in chunk 21 including exact provision pack arithmetic, menu/quote/follow-up asset rendering, partial-surcharge suppression, charity/refusal exits, tavern selector routing, arms quote/invalid-selector behavior, sage PRNG timing, sage short-funds exit behavior, fixed shipwright dock delivery, and shared entry-preamble routing; `shoppe_bark` sanitized `SHOPPE.DAT` render-audit tests; plus asset-backed route-smoke active-shop/modal flows including explicit tavern continuation frames, accepted public-rate inn rest, accepted all-stable horse-trader purchases, accepted shipwright frigate/skiff dock deliveries, all nine public arms-shop first-stock purchase and terminator-refusal rows, no-marker horse-trader refusal, and recovery/placement validation. | Exact visual polish and every shop content edge should continue to be audited; #62's stated ordinal-cluster and paint-order residuals remain provenance boundaries, not open implementation blockers. |
| Conversations | TLK runner, keyword loop, scoped prompts, ASK-PARTY-NAME, ASK-WHO, roster-companion recruitment prompts, action dispatch, shop routing, public issue #33/#40 built-in shared dictionary expansion for TLK and SHOPPE text, TLK `0x85` toll-progress karma behavior, and TLK `0x87` follow-up keyword scan are implemented without committing transcripts. Shared dictionary tokens obey the catalog's exact leading/pending-space rules; empty TLK slots emit the raw token in the runic font, and `0x8E` protected spans retain per-glyph `RUNES.CH` selection through Bevy and TUI. SHOPPE uses its distinct lookahead spacing contract and treats referenced empty slots as malformed. Conversation entry now preserves the shared PRNG for known NPCs; strangers install a fresh host-clock seed, draw the published coin flip, and either stay silent or introduce their Name field. Resident Falsehood cleanup likewise installs the fresh seed before inventory inspection and applies the published keys/gems/torches, equipment, scroll, potion, then gold theft cascade. | `conversation_session`, `tlk_runner`, `shoppe_bark`, and chunk 21 tests, including populated/empty token sequences, protected-span transcript and frontend font selection, SHOPPE token/token and token/text spacing, focused known/stranger greeting seed timing, Falsehood inventory-priority and PRNG timing, `active_conversation`, `ask_who`, `conversation_join`, `town_raw_tlk_gold_payment`, and a sanitized shipped-TLK corpus side-effect-control audit for public `0x86` action letters, `0x85` payments, `0x87` follow-up scans, IF/ELSE controls, and ASK-PARTY-NAME prompts, plus route-smoke and visual-route TLK reserved-word coverage across town/dwelling/castle/keep families. | Exact authored keyword paths for content-specific side effects and NPC memory flags need continuing audit. |
| Save/load | Known save fields, active objects, spell/reagent stock, transport markers, boarded ship hull/skiff state, overlays, dungeon working buffer, mirror files, shared active-effect code/duration, and the exact queued-vehicle X/Y/packed-class bytes are covered. Queued shipwright delivery survives a save before town exit and successful delivery clears only its class byte. Save/load uses the typed disk-role session and shared retry wrapper; load refreshes the per-plane mirrors, while save reads `UNDER.OOL` then `BRIT.OOL`, builds canonical `SAVED.OOL`, conditionally rewrites unchanged `UNDER.OOL` according to the entry disk role, and never writes `BRIT.OOL`. | Save/load tests in chunks 03, 04, 05, 07, 09, 11, 12, 13, 23, 25 plus `cli_binary_play_script_confirmed_save_round_trips_to_temp_save`; focused tests cover disk retries and role restoration, exact queued-vehicle preservation/increment/wrap/delivery, active-effect round trips, load mirror writes, save staging I/O counts, and synthetic plus local-clean `.OOL` aggregate audits. | Unknown byte preservation must be kept when adding durable fields; uncommon `.OOL` auxiliary byte meanings outside published families remain catalog work, and exact historical floppy labels and swap timing remain presentation compatibility work. |
| Clean-room hygiene | Runtime reads local assets; repo excludes game assets and generated raw dumps. | `.gitignore`, report policy, parser tests, clean status checks. | Continue reviewing any new reports or fixtures before commit. |

The latest contract reconciliation also removes two fallback models: town and
dungeon containers now share `traps.md §2.1`'s zero/one/many acting-member
picker (including disabled-member re-prompt, cancellation, and prompted name
echo), and `visibility.md §12.4`'s local-light mask is cached between its three
published refresh triggers. Beacon stamps are applied after that cached mask
and before the visibility carve.

Public combat issue #105 is implemented from `b1e8e08`: exact Shape B and
Escape narration, party-side Escape gating and cleanup, free-refusal versus
committed-action maintenance, entry-only ring vanishing, and exact
victory/defeat strings. Combat placement now also stamps the published faction
bits (`0x80` party, `0x40` ordinary monsters, `0x20` passive classes 8/9).

## Current Verification Baseline

Run these before broad gameplay commits:

```powershell
cargo fmt --all -- --check
cargo test -p u5-runtime --lib
cargo test -p u5-bevy
cargo test -p u5-tui --features visual
cargo clippy --workspace --all-targets
git diff --check
```

Measured on 2026-08-24 in the current worktree: **3223** u5-runtime, **183**
u5-bevy, **103** u5-tui (14 + 51 + 38), `cargo fmt --all -- --check` clean, and
`cargo clippy --workspace --all-targets` with **zero errors**. Clippy still
emits a broad existing style-warning baseline (331 warnings in the runtime test
target in this run, many duplicated); warnings are not gated.

For visual/raster work, also run the asset-backed suites. **Point them at a
copy of the asset directory, not at `C:\Games\U5-Clean` itself** - the install
is a read-only clean-room input, the harness paths take a directory they both
read and write, and it has been corrupted that way before. The engine now
refuses a write destination that resolves to `DEFAULT_GAME_DIR`
(`u5_runtime::test_fixtures::assert_writable_game_dir`, installed on
`install_intro_assets`, `install_canonical_intro_bit_asset` and the three suite
output directories), and `copy_asset_writable` clears the read-only bit Windows
`fs::copy` would otherwise propagate into a scratch copy.

```powershell
cargo run --features visual -- --route-smoke <asset-copy>
cargo run --features visual -- --visual-frame-suite target\visual-frame-suite <asset-copy>
cargo run --features visual -- --visual-route-suite target\visual-route-suite <asset-copy>
cargo run -- --save-frame-suite target\frame-suite <asset-copy>
cargo run -- --compare-frame-manifests target\baseline\manifest.txt target\frame-suite\manifest.txt
cargo run --features visual -- --visual --scene BRITANNIA <asset-copy>
```

Measured on 2026-08-24 in the current worktree from a writable copy of the
read-only local asset input: `--route-smoke` **all 514 cases passed**,
`--visual-frame-suite` **193 PNGs**, `--visual-route-suite` **1910 PNGs**, which include the whole
victory ending through the shared-moongate rise/hold/sink raster sequence and
on to the certificate.

## Remaining Boundaries

The final audit is published in `docs/completion-audit.md`. It maps every
public-spec system, format, and catalog to concrete engine evidence and test
coverage, and lists what is published but not implemented. Visual-only
deferrals are explicitly separated from gameplay-correctness gaps.

No fully published gameplay contract is currently known to be unimplemented. The
event-driven input boundary now performs the `main-loop.md §4` scene-byte
dispatch directly, with the historical exit-pending flag collapsed as §14
permits. Dungeon first-person wall/scenery selection is the published #84
seven-family billboard table; the withdrawn sparse-coordinate interpretation
and its retracted numeric pixel-ratio self-check are not retained.

The dungeon backward presentation pass is implemented from public #100 and
#101: masked `ITEMS` objects, `MON0`-`MON7` monsters, exact fresh/reuse setup and
record semantics, fountain points, field strobes, Negate Time's forced pose,
wall-decoration states and tone sweep, and the raw-`0x08` rising-pit overlay.

`docs/review-heuristics.md` records the mechanical checks used for future audit
passes. The remaining items in `TODO.md` are presentation polish or explicitly
unpublished details rather than known gameplay-contract gaps. Public issue
`#113` is resolved and implemented from `82daf8d`; `#114` is resolved and
implemented from `7046ca8` with exact combat-overlay gating and pixels. Public
commit `edba057` resolves `#115`; exact selected-bottle flash geometry/timing,
Orange/Purple combat object rewrites and wake restoration, and White's frozen
threshold-32 twenty-frame repaint sequence are implemented without substitute
overlay marks. The Bevy frontend consumes the flash as a blocking one-shot
event over the pre-effect framebuffer, then runs White from the typed
one-BIOS-tick playback cadence without double-advancing ordinary presentation
state. Public issue
`#116` / commit `01e2e1b` supplies the complete eight-row timing table: rumble
target `8,000 + 1,600i` and two sweeps of `10,000 + 4,000i` iterations. Every
selected colour executes the exact sound-disabled work through one shared
runtime helper. Both live and scripted Bevy input stop when the blocking flash
begins; terminal and headless raster paths finish the flash and all twenty
White frames before accepting another command or capturing the final frame.

Public issue `#117` is resolved by commit `fcc8181`. Return-to-View now renders
the exact opaque 15-step base/portal row splice in both directions and the
corner-first plus `0xB8` 256-write single-cell convergence, including direct
overlay/backing source selection and the 31 eight-write preview checkpoints.
The Bevy frontend also loops that expanded frame sequence at the shipped
`0x09` restart and treats Escape as an immediate ordinary abort on every frame;
it no longer freezes on the final frame or waits there before accepting Escape.
Public issues `#118` and `#120` are resolved by commits `12485b3` and `36780cb`.
Subtitle ignition implements the exact two-pass countdown/tails, polling and
abort order, `0x3500` Galois vector, gate/pitch recurrences, 45/50 pacing ratio,
publication anchors, normal/slow burst totals, and the non-consuming abort-key
handoff into the menu's first input poll. Public issue `#119` is
resolved by commit `58e9b9c`; the arms `S` browser now owns exact four-row
nonzero-id paging, normalized controls, fixed short-label rows, inverse
selection, refusal/sale continuation, and random-draw boundaries. Public issue
`#121` / commit `5b9445f` supplies the exact three-cell page badge and its
none/down/up/both fixed-font byte sequences. The gameplay compositor now owns
that badge and the `Arms`/`Select:`/`Items:` stats-ribbon labels, preserving the
shared two-colour cap raster instead of allowing the text overlay to flatten it.

Public issue `#122` / commit `c869c5b` closes the last presentation input gap.
The live first start/menu reveal copies `(1,0)` before its first odd-visit poll;
the loader consumes a pending abort, completes the rectangle plainly and skips
subtitle ignition while preserving the caller's automatic Return-to-View
one-shot. Return-to-View consumes aborts at ordinary preview ticks and cell-
convergence checkpoints, then restores the menu for a fresh poll rather than
dispatching the abort key as a command.

Continue refreshing the completion audit and this matrix whenever a behavior
moves between safe-placeholder and spec-backed implementation.
