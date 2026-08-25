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

2026-08-24, current worktree after the audit-and-repair pass through public
issue `#116`, the corrected telescope/Spyglass sky renderer, the shared
item-picker audit, and the shared exploration party-capability gate. The gate
now records the first Good/Poisoned member, advances no-input sleep passes,
runs town's wake-before-underfoot rolls, skips dungeon post-action effects, and
routes Stonegate's scripted wipe through the ordinary Blackthorn rescue on the
  next iteration. Public issue `#124`, resolved by clean-spec commit `d3863ef`,
now gives overworld sleep its full ordinary two-minute turn tail and makes
overworld defeat write the unchanged complete live object table to the current
plane mirror before rescue. Dungeon defeat's graphics teardown is a no-op in
this engine's permanently resident-atlas architecture and mutates no gameplay
  or presentation state before rescue. Public issues `#125` and `#126`, resolved
  by clean-spec commits `0a0b867` and `b600bc6`, move combat field contact to the
  common post-dispatch tail, target the acting descriptor, make Energy
  blocking-only, and fix Poison/Fire draw order and raw ranges. Terrain contact
  recognizes only swamp `0x04` as Poison and lava/fireplace `0x8F`/`0xBC` as
  Fire, with terrain taking priority over markers. Doom absorption is separate:
  committed non-digit player actions inspect row 1 of the renderer companion
  band while the live actor stands on row 2, before the common hook. The
  town-cell audit now labels `0x2A` as the harvested beacon
light source rather than the withdrawn spawn-marker model, and its NPC-only
harvest/scrub APIs are named accordingly. Public issue `#112` is resolved in
spec commit `8a73d12`; public commit `82daf8d` resolves `#113` with exactly one
Ready charge per invocation and nominal 2/1/1-minute world/town/dungeon costs:

Public issues `#127` and `#128` are resolved in clean-spec commits `21698d6`
and `98dfd45`. The certificate now emits exact `[`/`_`/`@` TH/ST/space cells
and centres from the 20-cell encoded first line. TLK `0x85` has no extra
confirmation or outcome labels: affordable demands continue in place, while
refusals print the exact fixed line and run the nested ordinary keyword loop.
Public issue `#129` is resolved in clean-spec commit `a7e55bf`: monster special
hooks now draw lazily from the shared PRNG, use exact contiguous 32/256 gates,
draw summon X then Y independently, attempt one cell, consume the actor's turn
only on handled success, and continue ordinary AI after every summon failure.
Public `#130` is resolved in clean-spec commit `e335918`: local View clears the
full `(8,8)..(183,183)` gameplay viewport, draws its 128x128 raster at absolute
`(32,32)`, never touches the side panel, and closes through ordinary world
redraw rather than saved-background restoration.
Public `#131` is resolved in clean-spec commit `60ac944`. The engine now uses
the published signed, unclamped resistance score and skewed `1..30` roll for
possession and every shared caller, while Tremor and Poison Wind use their
distinct target-weight comparison with the published forced-weight cases.
The 2026-08-24 corrective spell audit also reconciled the final public `#11`
answer: Kill excludes protected classes 14/15/47; Cause Fear and Repel Undead
write exactly 1 HP plus fleeing; Repel neither kills nor awards XP; Conjure has
sixteen weighted outcomes; all three conjuration spells use whole-candidate
`0..=15` probes; Swarm places up to four actors at one accepted cell; and only
successful party Summon stamps its Daemon controlled. Public `#132` is resolved
in clean-spec commit `1e28720`: protected Kill rejection occurs after the charge
and 7 MP are spent, skips resistance PRNG and target effects, reports `Failed!`,
and commits the combat action without reopening the cursor or actor prompt.

- `cargo fmt --all -- --check` clean.
- `cargo test -p u5-runtime --lib` 3222 passed.
- `cargo test -p u5-bevy` 181 passed.
- `cargo test -p u5-tui -- --test-threads=1` 103 passed (14 + 51 + 38).
- `cargo clippy --workspace --all-targets` **zero errors**. Style warnings
  remain across the existing workspace (333 in the runtime test target in the
  latest run, many duplicated) and are not gated.
- `--route-smoke` all 513 scripted cases passed, including the overworld-defeat
  pre-rescue OOL-persistence path through the real command-boundary gate.
- `--visual-frame-suite` wrote 193 PNGs and runs to completion.
- `--visual-route-suite` wrote 1906 PNGs, including the whole victory
  ending through to the certificate. Exactly one is black by contract: the
  `endgame.md §7.1` fade-to-black frame between the throne tableau and the first
  `END.DAT` window.
- **The victory ending is reachable.** `endgame.md §9.1`-`§9.5` published the
  certificate wording, which was the last gate on an unpublished contract
  anywhere in the engine. The ending runs rite beats, tableau exit, the `§7.1`
  fade, six `END.DAT` windows, the certificate on its parchment, the
  elapsed-time report, and the `§9.5` terminal hold; route-smoke's validator
  requires `cinematic_is_finished()`, so the case fails if it stops short.
- **No `panic!` in `crates/` stands for an unimplemented published contract.**
  The Ultima IV transfer preview was the last one and it is built (`f3ecfd1`).
  Every refusal that remains is structural (a graphical screen with no terminal
  surface) or an injection guard; see `docs/completion-audit.md`, "Refusals that
  remain".
- Public commit `574f1d8` closed `cleak/u5-spec#109`; ordinary alarms now
  destructively rewrite all occupied roster slots across every floor through
  the exact `0xFC`/`0xD8`/`0x70` pursuit exceptions and shared-stream byte
  draws, while resident Hatred/Cowardice entry sweeps consume all 32 coin draws
  and deliberately reproduce the fixed-slot-4 type-test defect. Pursuit,
  flight, and `0xFD`/`0xFE` Talk routing are implemented without synthetic NPC
  alarm markers. Public commit `06494e0` closed `cleak/u5-spec#108`; the exact host-clock
  seed transform and its caller timing are implemented for gameplay-state
  construction, Shadowlord blight, wilderness camp, stranger greetings, and
  Falsehood theft. Public commit
  `bc0c761` closed `#107` and confirmed
  the allocator's wrapped inclusive ±5,
  current-player-global, floor-independent screen predicate; the engine's
  existing behavior now has exhaustive separation and source tests. `#106`
  closed in public commit `b34ae69` and is implemented. `#103` closed in public commit
  `a4167b0`; its exact generic-adjacent impact gate, type-only combat class
  mapping, independent terrain/transport arena selector, and high-to-low
  continuation across returning combat are implemented. `#102` is also closed
  and implemented.
- The town free-roaming object walker now treats the map edge at the published
  destination-bounds stage rather than as an invented pen blocker. Edge animals
  can choose valid inward steps; outward choices still consume the chance,
  axis, and sign draws before failing without mutating the record.
- A fresh audit of closed issue `#79` corrected the resident gameplay text
  state: window 0 is restored full-screen, window 1 is the stats panel, window
  2 is the message rectangle with its cursor initially on the bottom row, and
  window 3 remains the unused default. Line feed is combined CR+LF, scrolling
  copies the row below into the vacated bottom row without blanking it, and the
  live prompt shares window 2's bottom row.
- The public shared-word catalog now drives exact TLK and SHOPPE token
  semantics. TLK preserves the unconditional leading space, pending-space
  state, empty-token raw runic glyph, and `0x8E` runic font through both
  frontends; SHOPPE enforces its distinct trailing-space rule and rejects an
  empty referenced dictionary entry as malformed content.
- Asset-backed runs used a **copy** of the asset directory. `C:\Games\U5-Clean`
  is a read-only clean-room input; the engine now refuses a write destination
  that resolves to `DEFAULT_GAME_DIR`, and `copy_asset_writable` clears the
  read-only bit Windows `fs::copy` propagates into scratch copies.
- Spec contracts come from `cleak/u5-spec` on GitHub - the issues, and document
  text through `gh api -H "Accept: application/vnd.github.raw"
  repos/cleak/u5-spec/contents/<path>`. The local `u5-spec` checkout is
  read-only from this workspace and stale at `9a898d1`, many commits and
  several retractions behind head.
- `docs/review-heuristics.md` records the three mechanical checks that found
  most of this pass's defects. Run them as routine, not on suspicion.

Earlier verification detail, kept for history:


- `cargo test -p u5-runtime --lib` passed on 2026-05-24, including 2653 tests
  (latest verification includes public `cleak/u5-spec#47` hourly
  poison/provision/ring ordering, public #28 horse-trader adjacent placement
  priority plus no-marker refusal, public #15 inn pickup stay-counter billing,
  combat command-flow regressions for pending Z-stats/Cast actor liveness, public
  `cleak/u5-spec#41` arms-shop scene-row coverage, public issue #3
  terrain-combat replacement-tile main path, public-spec View/Peer/X-Ray
  overlay raster-class and dungeon minimap glyph-id coverage, sanitized
  aggregate `LOCATION.DAT` authored-cell audit coverage, sanitized aggregate
  `.OOL` active-object overlay audit coverage, sanitized aggregate tile-atlas
  and fixed-font audit coverage, expanded sanitized paired-graphics image-directory
  and sprite-sheet audit coverage, and public issue #21 dungeon
  active-monster ambush setup, combat round maintenance, combat-local ambush/camp
  reveal-slot helper coverage, stats-panel combat inverse-video overlay
  coverage, disk I/O retry wrapper coverage, and shared TUI/Bevy Journey Onward
  disk-error presentation).
- `cargo test -p u5-runtime published_location --tests` passed on 2026-05-24,
  including exhaustive clean fixture coverage for entering and restoring all 40
  public world-location rows without a sidecar.
- `cargo test -p u5-tui` passed on 2026-05-24, including 79 tests.
- `cargo test -p u5-tui --features visual` passed on 2026-05-23.
- `cargo test -p u5-bevy` passed on 2026-05-24, including 67 tests.
- `cargo fmt -- --check` passed on 2026-05-24 after the latest Rust changes.
- `git diff --check` passed on 2026-05-24; the only output was existing
  CRLF-normalization warnings.
- Representative raster smoke checks with local assets produced nonblank hashes:
  `BRITANNIA` top-down `fd923dc0f87a9f3c`, `BRITANNIA` after movement
  `1eb882f27b1d216c`, route-smoke Britannia movement `bef4c9fc1eecf9fb`,
  `CASTLE:0` top-down `be84488b7b199310`, and `DUNGEON:0` first-person
  `161ad48dd2a91725`.
- `cargo run -p u5-tui -- --route-smoke C:\Games\U5-Clean` passed on
  2026-05-24 with 495 scripted route cases and the sanitized
  `--route-smoke-manifest` wrote 2183 initial/per-command/final frame rows
  that compare cleanly against themselves (including all 40 published stock
  world-location entry rows, native shrine/Codex quest routes, TLK-backed
  conversation routes across all 32 named-location scenes, plus save/reload checkpoints
  for boarded horse, Gate Travel, chasm fall, fixed hidden treasure,
  horse-trader delivery, ship X-it/skiff, dungeon ladders, and dungeon exits,
  plus expanded active-shop/modal
  routes for arms, healer, inn, reagent, tavern, horse trader, shipwright,
  guild, and sage flows, plus four extended-session
  cases: 12-step Britannia exploration with Z-stats and Look, 10-step castle
  walk-and-rest, 9-step dungeon turn-and-search, 5-round Doom combat pass, and
  focused Create Food, fountain Look, Yew wanted-poster Look, town
  attack/alarm/arrest routes,
  Horse/non-horse wishing-well branches, death-vision Look, public
  #44 sleeping/praying Talk refusals, public #48 Blink ray landing,
  Locate, In Lor/Light/Open, restore-spell, active-effect, all-cardinal directed Sleep/Poison
  Wind/Death Wind/Flame Wind combat casts, combat field marker casts/removal,
  combat directed-utility tile casts,
  targeted Magic Missile/Tremor/Repel Undead/Charm/Polymorph/Clone,
  Conjure/Swarm/Summon Daemon, special death-marker Kill combat spell casts,
  asset-backed combat-entry party descriptor routes, and combat terminal cleanup routes,
  dungeon level, dungeon
  field/dispel, dungeon Open chest, light-decay, dungeon ladder-chain,
  dungeon-to-world return, hourly provision/poison/starvation/ring passes, public
  #32 Britannia/Doom Word-of-Power seal opening routes, public #15 accepted
  inn-rest pricing, public #13 all-nine-tavern lore selector routes plus
  sage paid-success/short-funds paths, public #31
  native shard/Eternal Flame destruction routes, native town walk-on stair
  up/down/crossing routes, accepted shipwright frigate/skiff purchase routes
  that verify published dock-coordinate delivery, and public #21
  active-monster attack/contact ambush routes)
  covering world/town look and
  save-refusal prompts, surface/town/dungeon View overlays, the corrected
  telescope/Spyglass night-sky overlay, Peer and X-Ray overlays,
  U-Use utility items including Pocket Watch/Sextant/Magic Carpet
  (`c2f7ff2c1000c8fd`), HMS Cape plans (`8c425fda6007db98`), and Wooden Box
  (`d5684b90a48f2d73`),
  Shadowlord town entry (`1e04222a325a2f67`), Shadowlord-name Yell
  (`abad79297a559cd2`), Stonegate Shadowlord entry presentation
  (`c7190c94f55a2af2`),
  H-Hole-up rest in Britannia (`8d7e6e0336279317`) and a dungeon
  (`22ccb05a46f3140e`), Underworld startup, debug-entered town return to world,
  Underworld-to-town debug entry, ship X-it/skiff launch, hoisted-sail movement,
  dungeon turn/block movement, public `0xE?` heavy-door blocking, dungeon exit confirmation/refusal, a Doom room
  trigger that enters a combat raster viewport, dungeon Attack/Search/Get/
  Jimmy/Open/refusal command routing, and combat pass, active-player digit
  selection/clear, Escape foes-remain refusal, Ctrl-S music toggle
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
  (`3f4fdf2e53e4e269`), forced whirlpool Underworld branch
  (`1a25aca8d540a7fe`), fixed narrative gate open/ordained-block routes
  (`f41fe34d7c89a48b`, `061731b8753aad9c`), public #48 Blink ray landing
  (`f4b691ac224b385e`), public #51 poison-gas doorway step
  (`836b6cd5af06c44e`), public #47 dungeon no-direct-recovery rest
  (`161ad48dd2a91725`), completed long-camp recovery, plus hourly Ring of Regeneration
  (`be84488b7b199310`), plus ship broadside fire
  (`a7f1e8c1d62d7388`), horse boarding (`c346e297d616e667`),
  dungeon torch ignition (`06a7a60a0f84fb96`), and a combined Mix/Ready/New
  Order command workflow (`93018f522ce292ec`),
  town dispatcher refusals (`3126df0494b5870b`), town party overlay routes
  (`26d7ef40b57084af`), and terminal endgame missing-box
  confirmation/jitter plus Wooden Box victory confirmation/full cinematic
  routes, plus Blackthorn audience correct-password
  (`338605b812db4ced`), wrong-password (`b700db952bc67bbf`), and
  rescue-refuge (`3c4488f67ab70cb5`) paths.
- Public #41 arms-shop route coverage includes first-stock purchases across all
  nine published stocked rows plus terminator-letter refusal rows that verify no
  gold or equipment mutation before submenu exit.
- The TUI binary integration tests include temp-directory startup and save
  smoke for Journey Onward's empty-save return-to-menu path, deterministic
  Create Character followed by `--from-save --play-script`, intro-driven U4
  transfer commit from `PARTY.SAV`, and a confirmed `QY` save/reload
  round trip. These tests mutate only per-test temporary asset directories,
  never `C:\Games\U5-Clean`.
- `cargo run -p u5-tui -- --save-frame-suite target\codex-view-class-gallery-frame-suite
  C:\Games\U5-Clean` passed on 2026-05-24 and wrote sixteen nonblank PNGs:
  `britannia` `859a1bdabe5c9b7a`, `britannia-step` `05b13e47da048fe6`,
  `castle` `bda625019405af09`, lit `dungeon` `91ea22aa5e09c692`,
  composed `combat` `49bcd6e0986745fd`, `surface-view`
  `b8d9ef46cd161b93`, `dungeon-view` `8629ba329e58a747`,
  `peer-view` `4c9e7bb65a91b568`, `x-ray-view` `4c9e7bb65a91b568`,
  `surface-view-class-gallery` `1c725f8e26c826f4`,
  `peer-view-class-gallery` `1c725f8e26c826f4`,
  `x-ray-view-class-gallery` `9861d57c4d8f3dbd`, `intro-menu`
  `7bf01c36de552e16`, `status-window`
  `bf7a428a4b00ad2b`, `z-stats-modal` `61b033bfa2488b46`, and
  `endgame-status` `532cb7f1bdd03ffd`.
- `cargo run -p u5-tui --features visual -- --visual-frame-suite
  target\codex-view-class-gallery-visual-frame-suite C:\Games\U5-Clean` passed on 2026-05-24 and
  wrote 163 nonblank Bevy-owned PNGs plus a sanitized manifest, including all
  16 public `BRIT.CBT` outdoor arenas with accepted early replacement rolls,
  all 112 public `DUNGEON.CBT` dungeon-room terrain records with source scanning
  disabled, prompt/modal frames for world, town, dungeon, combat, and Talk,
  surface/town View class galleries for gem, Peer, and X-Ray modes, and combat
  status-highlight plus death/field/cursor marker galleries:
  `world-play` `f68b906acde0bd4a`, `world-after-step`
  `b9720ab18affa566`, `town-play` `2beb3b7734800e11`,
  `dungeon-play` `67e7e116d8be67aa`, `dungeon-dark`
  `29289813c0f0397c`, `combat-play` `9b1937b3e807ba05`,
  `combat-status-highlight` `8f619b26cbe87bed`,
  `surface-view-overlay` `2ee1809341456a23`, `dungeon-view-overlay`
  `450b8690ef5bc292`, `night-sky-overlay` `ab070c8b603b0cc0`,
  `peer-view-overlay` `2c64191172043730`,
  `x-ray-view-overlay` `2c64191172043730`, `z-stats-modal`
  `bee4e11801862ad1`, `endgame-status` `d6c3450bd51d97f0`,
  `surface-view-class-gallery` `1c725f8e26c826f4`,
  `peer-view-class-gallery` `2640d7f6238170bc`,
  `x-ray-view-class-gallery` `70ce97b654df9d85`,
  `combat-arena-00` `774828109138f22a`, `combat-arena-15`
  `f5708df6d90c001b`, `dungeon-combat-arena-000`
  `d40e5e2b05532e84`, `dungeon-combat-arena-111`
  `e0d891685c17aefc`, `combat-marker-gallery`
  `cfc0b921b067ec75`, `intro-menu` `9713a4bbd31395e8`,
  `intro-finished-menu` `16dfab9fc3d5f489`, `intro-story-art`
  `5aa68210c861bc65`, and `intro-return-to-view`
  `097761f6267d3b94`.
- `cargo run -p u5-tui --features visual -- --visual-route-suite
  target\visual-route-suite C:\Games\U5-Clean` passed on 2026-05-24 and
  wrote 1780 nonblank Bevy-owned per-step route PNGs plus a sanitized
  manifest, with all 40 published stock world-location entry rows, native
  shrine/Codex quest routes, TLK-backed reserved-word conversation routes across
  all 32 named-location scenes, and
  TUI-parity labels for additional ship/castle/shop/dungeon
  and Doom combat aliases as well as world/town/dungeon
  movement/pass/look/view/status, Minoc daily fixed-hidden, hourly
  status/ring, native stair, dungeon rest/long-camp/ladder/exit/search, and
  active-monster ambush routes: `route-world-movement-00-initial` `f68b906acde0bd4a`,
  `route-world-movement-01-d` `ec7c5878d044dda6`,
  `route-world-movement-02-idle` `949d4d0fb006d273`,
  `route-town-status-modal-00-initial` `2beb3b7734800e11`,
  `route-town-status-modal-01-z` `bee4e11801862ad1`,
  `route-town-view-overlay-00-initial` `2beb3b7734800e11`,
  `route-town-view-overlay-01-v` `37d91ad87aa485e1`,
  `route-town-view-overlay-02-idle` `5d5af54c5d7eb0f0`,
  full-frame overlay open/close coverage for world View
  (`route-world-view-overlay-01-v` `2ee1809341456a23`), dungeon View
  (`route-dungeon-view-overlay-01-v` `450b8690ef5bc292`), Peer
  (`route-castle-peer-overlay-01-c1iqw` `37d91ad87aa485e1`), and X-Ray
  (`route-castle-x-ray-overlay-01-c1imx` `703fdeef9d192429`),
  `route-britannia-look-00-initial` `f68b906acde0bd4a`,
  `route-britannia-look-01-l6` `da5ca5200c222d0f`,
  progression frames `route-britannia-utility-use-items-03-uc`
  `1e30ba357d12573d`, `route-ship-hms-cape-plans-use-01-up`
  `7b537b8f442657a8`, `route-britannia-create-food-cast-01-c1imx`
  `57ab276b9708111f`, `route-gate-travel-world-to-underworld-01-c1prv1`
  `fc9f55e0e7eb7f61`, `route-gate-travel-world-to-castle-01-c1prv2`
  `d2a09237e6bbeae9`, `route-gate-travel-invalid-slot-refusal-01-c1prv4`
  `ae99ac800f08d8fa`, `route-gate-travel-shipboard-refusal-01-c1prv2`
  `f72019db2ff927e1`, `route-natural-moongate-trammel-gate-travel-01-idle_1`
  `fc9f55e0e7eb7f61`, `route-natural-moongate-empty-slot-clears-live-tile-01-idle_1`
  `6d146362bede1794`, `route-britannia-chasm-fall-to-underworld-01-s`
  `f4a55f01e90aedc0`, `route-britannia-whirlpool-forced-underworld-01-setup_whirlpool-engagement`
  `b0803c84058f9879`,
  `route-britannia-fixed-narrative-gate-open-south-step-01-empty`
  `ff4767117dbb9b7c`,
  `route-britannia-fixed-narrative-gate-ordained-block-01-empty`
  `ff4767117dbb9b7c`, `route-britannia-hole-up-rest-01-h1`
  `766522e62f639357`, `route-britannia-save-refusal-02-n`
  `a89ba1fbff6881da`, `route-britannia-fixed-hidden-single-use-search-get-02-g6`
  `eb0e32b031d839a3`, `route-underworld-fixed-hidden-stack-search-get-search-03-s6`
  `c6cc1cfe27226f01`, `route-blackthorn-fixed-hidden-zero-key-search-01-s6`
  `038f97f62b047471`, `route-castle-wooden-box-use-01-ub`
  `de28d405fbe94478`, `route-blackthorn-audience-correct-02-ahm`
  `b3016738bb32fbdb`, and `route-blackthorn-rescue-refuge-02-empty`
  `ba5dc7f126f2411c`,
  Shadowlord/quest frames `route-virtue-town-shadowlord-entry-00-initial`
  `d1d1b68786dacfc7`, `route-virtue-town-shadowlord-yell-01-yfaulinei`
  `b7478369c5d5dcaa`, `route-lycaeum-shard-falsehood-vanquish-01-uf`
  `f89b3fec282be2b2`, `route-empath-shard-hatred-vanquish-01-uh`
  `4697d02e7b1b717e`, `route-serpents-hold-shard-cowardice-vanquish-01-ucw`
  `8d6b68ed8daa4599`, `route-stonegate-shadowlord-entry-00-initial`
  `7f3f20ca7b391cc8`, `route-britannia-word-of-power-seal-opens-01-yfallax`
  `09710bf07543b67e`, and
  `route-underworld-doom-word-of-power-seal-opens-01-yveramocor`
  `456def28b0dff5b1`,
  `route-britannia-spyglass-night-sky-00-initial` and
  `route-britannia-spyglass-night-sky-01-usp`,
  `route-castle-save-refusal-00-initial` `2beb3b7734800e11`,
  `route-castle-save-refusal-01-q` `c58e3249e4d12730`,
  `route-castle-save-refusal-02-n` `6465878cfb486dd1`,
  `route-world-board-horse-00-initial` `dad599d00fa00a6a`,
  `route-world-board-horse-01-b` `402223dd79b07b77`,
  debug-enter transition frames for castle entry, castle return-to-world,
  underworld-to-castle entry, castle dispatcher/workflow overlays, and dungeon
  entry/SJOG/refusal/exit-refusal,
  ship/skiff frames for X-it launch and hoisted-sail eastward movement,
  `route-ship-broadside-fire-00-initial` `8b7440c1476c5f31`,
  `route-ship-broadside-fire-01-f6` `edd09405aa4bbd41`,
  `route-dungeon-movement-search-00-initial` `67e7e116d8be67aa`,
  `route-dungeon-movement-search-01-w` `5be54e4dfc5e923f`,
  `route-dungeon-movement-search-02-a` `55d65ca1a5c74e9f`, and
  `route-dungeon-movement-search-03-s6` `fbcbfb63d205e997`, public #1
  `route-dungeon-heavy-door-variant-block-00-initial` and
  `route-dungeon-heavy-door-variant-block-01-idle`,
  `route-dungeon-ignite-torch-00-initial` `29289813c0f0397c`, and
  `route-dungeon-ignite-torch-01-i` `462ca23693fa9e2a`,
  `route-dungeon-exit-refusal-00-initial` `67e7e116d8be67aa`,
  `route-dungeon-exit-refusal-01-q` `50baeabcaa6d8347`,
  `route-dungeon-exit-refusal-02-n` `bbceaa4d74f5a7ea`,
  `route-shop-sage-topic-miss-00-initial` `eafb6cf3478f4c49`,
  `route-shop-sage-topic-miss-01-mantra` `67cfc176459efad8`,
  `route-doom-combat-trigger-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-trigger-01-empty` `30fecda7448a111d`,
  `route-doom-combat-pass-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-pass-01-empty` `30fecda7448a111d`,
  `route-doom-combat-pass-02-empty` `c1ffc74f45610145`,
  `route-doom-combat-attack-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-attack-01-empty` `30fecda7448a111d`,
  `route-doom-combat-attack-02-a6` `c1ffc74f45610145`,
  `route-doom-combat-board-refusal-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-board-refusal-01-empty` `30fecda7448a111d`,
  `route-doom-combat-board-refusal-02-b` `c1ffc74f45610145`,
  `route-doom-combat-z-stats-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-z-stats-01-empty` `30fecda7448a111d`,
  `route-doom-combat-z-stats-02-z` `c1ffc74f45610145`,
  `route-doom-combat-search-prompt-00-initial` `6fdbd1b19453bbea`,
  `route-doom-combat-search-prompt-01-empty` `30fecda7448a111d`, and
  `route-doom-combat-search-prompt-02-s` `c1ffc74f45610145`. The
  latest TUI-label visual-route expansion adds exact ship broadside,
  castle dispatcher/fountain/Talk/Light/Open/Mix/Ready/New Order/party
  overlay, active shop, dungeon torch/turn/SJOG/refusal/reload, Minoc reload,
  wishing-well, terrain combat exit, endgame confirmation, combat field, and
  Doom room combat/pass/selection/direct-step/attack/refusal/Escape/music
  toggle/quit label coverage. Bevy visual-route coverage now also includes
  the exact TUI-label light-decay route and all nine arms-shop terminator
  refusal routes, with those inert/terminal frames explicitly allowed as
  unchanged where the underlying route produces no visual delta. The latest
  Bevy real-key visual-route expansion adds keyboard-path coverage for
  movement, pass, Ctrl-S music toggle, save refusal, conversation/shrine/shop
  line buffers, direction prompts, Yell text, Ready/Z-stats modal pickers,
  Backspace, Enter, and prompt-safe Escape. The
  latest combat visual-route expansion adds composed-frame Doom combat
  command coverage for digit selection/clear, direct movement,
  the shared Use picker, Drop/Wear/Enter/Fire/Hole-up/Ignite/Mix/New
  Order/Talk/View/Look refusal or label branches, Cast/Get/Jimmy/Open/Push/Klimb directed
  prompts, Ready, Yell, and the free X-it refusal, including representative terminal frames
  `route-doom-combat-cast-refusal-02-c1il` `325a6f1641bb2455`,
  `route-doom-combat-ready-prompt-02-r` `c1ffc74f45610145`,
  and `route-doom-combat-xit-refusal-02-x`. The
  latest long-route expansion adds extended Britannia/castle/dungeon play
  sessions plus sustained Doom combat pass rounds, including
  `route-britannia-extended-exploration-12-empty` `96d8b1b3118012d7`,
  `route-castle-extended-walk-and-save-09-z` `431f2a5ad9c4b417`,
  `route-dungeon-extended-turn-and-search-09-s6` `eb8f6403c755776f`, and
  `route-doom-combat-multi-round-pass-05-empty` `c1ffc74f45610145`. The
  latest shop visual-route expansion adds accepted healer cure/heal/resurrect
  route endings `route-shop-healer-cure-accept-04-y`
  `ebfe20f2b6b24a78`, `route-shop-healer-heal-accept-04-y`
  `4c68bd7db37eeff9`, and
  `route-shop-healer-resurrect-accept-04-y` `218eae0cf0de4491`, plus all
  public shipwright delivery-row endings
  `route-shop-shipwright-island-frigate-buy-02-y`
  `a9cb3a58bcf43fe9`,
  `route-shop-shipwright-crows-nest-skiff-buy-02-y`
  `afa92cf12c5d0f91`,
  `route-shop-shipwright-oaken-oar-frigate-buy-02-y`
  `e47cf61edd1372b9`, and
  `route-shop-shipwright-rusty-bucket-skiff-buy-02-y`
  `6f9691ff490ad348`. The latest public #41 arms-shop visual-route expansion
  adds accepted first-stock purchase routes across all nine published stocked
  shop rows; terminator-letter refusal remains in route-smoke because the
  visual route harness correctly rejects unchanged frames. The previous endgame
  visual-route expansion adds the
  public #56 six-member
  class-tableau/restoration route
  `route-endgame-class-tableau-restoration-00-initial`
  `412fa97088d0737f` and
  `route-endgame-class-tableau-restoration-01-y` `f854216c1a7eae8f`,
  alongside the missing-box terminal tableau and full Sandalwood Box victory
  cinematic route steps on the composed Bevy framebuffer; the 2026-05-23
  expansion added public #48 Blink
  `route-britannia-blink-east-ray-01-c1ip6` `17ceb1f94bc6c6e3`,
  Locate, In Lor/Light/Open, restore-spell, active-effect, directed Sleep/Poison
  Wind/Death Wind/Flame Wind combat casts, combat directed-utility tile casts,
  dungeon level, dungeon field/dispel, and dungeon Open spell route frames,
  public #51 poison gas `route-castle-poison-gas-step-01-d`
  `33ebf44d5ea24373`, public #15 inn rest
  `route-shop-inn-rest-accept-02-y` `2591d02e2c602824`, public #13 sage
  paid/short-funds outcomes `3d1bdb8ea9234398` / `c73a1d5ec1594b41`, and
  public #43 fountain, wishing-well, and death-vision Look endings
  `ce66d6a727da0f4c`, `cb49b5dc01b81ad4`, and `ac129703cfe1c060`. The
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
- Clean spec refresh on 2026-05-23 found `cleak/u5-spec` at `da0654d`; no newer
  public push was visible from this workspace.
- The spec checkout used for the most recent audit was `da0654d Specify
  directed wind cone targeting`.

Current worktree context when this TODO was refreshed:

- `u5-engine` was clean after `af47e8b Add Bevy visual route reload checkpoints`.
- `u5-spec` was current with GitHub `origin/master` at `da0654d`.
- `journal/capture/notes.py` was not present in the workspace, engine, or spec
  repository.
- Town-family exits now prompt only for outward steps from the 32-by-32 grid;
  accepting exits through clean return metadata, while refusal/cancel discards
  the step. The withdrawn `0x59`/`town_exit_tiles.tsv` model is removed. Public
  issue `cleak/u5-spec#110` fixes the shared `(31,31)` terrain sample, true
  out-of-grid occupancy coordinate, and exact blocked/yes/no/cancel turn costs.
- Natural moongate live-tile refresh now keeps mode-zero scene/light cleanup
  from advancing the shared gate-presence counter. The cached Trammel/Felucca
  glyph bytes now refresh from the public hour-indexed tables on construction,
  hour changes, and status redraw, and live-gate entry decodes only the cached
  byte instead of recomputing the table at entry time. The engine now follows
  public `cleak/u5-spec#38` for Felucca hours 10/11/19/20: they are literal
  phase-0 glyph bytes, so natural-gate entry routes them through Moonstone
  slot 0.
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
- U4 transfer source validation follows `u4-transfer.md §5.1`/`§5.2`/`§5.3`
  as re-derived in `cleak/u5-spec#88`. The transfer makes exactly two reads of
  `PARTY.SAV` - `0x0008` for 40 bytes, then `0x0140` for 182 bytes - and the
  gate tests only six leading-record fields: HP, max HP and experience at
  `0..9999`, STR/DEX/INT at `0..70`, class index at `0..7`, and the first eight
  name bytes each NUL or `>= 0x20`. No party-wide counter is read, so none can
  reject. All-zero virtue standings are the **Avatar success condition**, not a
  rejection. The earlier entry here described the `#16` reading - counter
  offsets near the head of the file, a leading name at `0x001A` with the class
  at `0x0019`, and an eight-byte "no transferable data" virtue gate - and every
  part of it is withdrawn.
- Rest/camp ordinary HP/MP recovery follows the latest public guidance in
  `cleak/u5-spec#47`: rest advances time without a separate direct recovery
  grant. Hourly Ring of Regeneration is time-owned, non-combat only, checks the
  ring equipment slot, heals living wearers by exactly 1 HP on the 1-in-8 roll,
  and clamps at max HP. Remaining work here is prompt/pacing presentation, not
  the #47 recovery contract.
- Non-combat Blink follows the latest public `cleak/u5-spec#48` guidance: it
  prompts for a cardinal direction and lands on the farthest legal grass cell
  along the bounded ray. Route-smoke and Bevy visual-route coverage exercise an
  eastward Britannia ray.
- Create Food follows the latest public `cleak/u5-spec#49` guidance with a
  tiny `1..=3` food PRNG grant capped at the party food cap.
- TLK `0x85` accepted payments debit gold and continue in place without an
  extra confirmation or invented outcome label. Ordinary exploration actions
  age the saved cooldown; a live class-108 speaker consumes a threshold-ready
  counter and applies the published milestone karma behavior. Unaffordable
  demands use `#128`'s exact refusal and nested-loop unwind.
- The shipped `.TLK` asset corpus has a sanitized runtime test that scans raw
  fields for public side-effect controls without committing dialogue text:
  action-dispatch grants `A`, `C`, `F`, `J`, and `K`, plus `0x85` payments,
  `0x87` follow-up scans, IF/ELSE controls, and ASK-PARTY-NAME prompts.
- Hourly poison and starvation follow the latest public `cleak/u5-spec#50`
  guidance: poison is fixed `-1 HP` per poisoned living member, and starvation
  rolls `1..=8` independently for each non-dead slot.
- Town poison-gas doorway cells use the latest public `cleak/u5-spec#51`
  predicate from native `0x04` live tiles with foot transport, with a `0..=29`
  per-non-poisoned-slot roll compared against each member's Dexterity after
  committed movement steps and before turn-clock advancement. Older coordinate
  and tile-attribute sidecars no longer trigger this branch.
- Talk-triggered arms shops use the public `cleak/u5-spec#41` scene-to-row
  identity table and exact per-row `a..h` stock arrays; visible buy choices stop
  at the `0xFF` terminator, and buy quotes map equipment ids to the published
  SHOPPE.DAT record ranges with cap-first purchase refusal and `Sold!` success.
- Tavern round-drink prompts now use the public `cleak/u5-spec#13`/shops table
  letters (`M`, `B`, `F`, or `C` by tavern), secondary-tavern letters,
  provisions letters (`R` or `P` where present), and per-tavern lore letters
  with lore gated behind an accepted continuation branch. The live machine now
  keeps the published `Anything else?` Y/N state instead of jumping directly
  back to the menu, renders the state list/follow-up and random `77..=82`
  provision quote records, applies the speaker-Intelligence price, adds 25 food
  per paid pack, skips the Falsehood surcharge after a gold-exhausted partial,
  and implements both terminating zero-service outcomes including the one-food
  charity below three provisions.
- Paid sage rumours use the public `cleak/u5-spec#13` 26-row topic table,
  strict topic matching, SHOPPE.DAT record 84 fee quotes, SHOPPE.DAT record 91
  short-funds refusal, and success-record random draw only after the accepted
  confirmation passes the gold/debit gate; short-funds and declines preserve
  PRNG state, and short-funds exits the sage flow per checked-in spec.
- Public `cleak/u5-spec#28` corrected the old stationary-display purchase path
  to horse-trader sale rows. The obsolete stationary-display purchase runtime is
  removed, and horse-trader runtime/talk-shop/route-smoke/visual tests cover
  Intelligence-adjusted quotes, local marker placement, no-marker refusal, and
  accepted purchases for all three public stables.
- Shadowlord shard U-Use follows public `cleak/u5-spec#31` exact native
  positions and requires the matching live Shadowlord/name encounter north of
  the party; successful destruction marks the native hideout byte and ORs the
  save-backed quest-progress word bits. Route smoke covers Lycaeum, Empath
  Abbey, and Serpent's Hold native paths.
- The clean-engine audit has reconciled the closed public issue queue through
  #135. Public commit `24f4aa4` supplies the ruined-shrine Word-of-Power
  restoration dialogue and mutation contract; the exact four-answer flow,
  silent failure, tile restoration, and shrine-only flag change are implemented.
  Public commit `574f1d8` supplies the destructive town alarm and
  resident-Shadowlord schedule/dialogue sweep contracts, including the fixed
  slot-4 defect and exact shared-PRNG consumption; those paths are implemented
  and covered by deterministic focused tests. Public commit `06494e0` supplies
  the host-clock PRNG equation and caller timing.
  Public commit `b1e8e08` closed #105; the engine pins exact Shape B and Escape
  text, free re-prompts before maintenance, committed non-digit ring/effect
  hooks, entry-only ring vanishing, and exact victory/defeat text. Public commit
  `b34ae69` closed #106; the two Blackthorn rescue calls and the lit-dungeon
  Search reveal tail now use the exact shared blocking viewport dissolve, while
  darkness, bomb, narration-only, and Open paths bypass it. Bevy and TUI
  acknowledge the completed blocking-call records when presenting the final
  caller-composed state, preventing transient playback accumulation.
- Shop session regression tests now lock the corrected public scene-byte rows
  for taverns, shipwrights, reagent vendors, guildmasters, inns, healers, and
  arms-shop identities, including old wrong-scene negative cases from the
  public issue corrections.
- Public `cleak/u5-spec#43` Look specials now cover top-down fountain drink
  prompts with presentation-only refresh, wishing-well coin and 12-character
  wish input with scene gates, structured accepted keyword matching, a native
  Horse grant, accepted car keywords mapping to the horse-family grant in
  public scenes, death-vision active-object dispatch with member selection, and
  the Yew wanted-poster fixed framed row stream with party slots 0..2 centered
  and slots 3+ omitted.
- Return-to-View now expands the MISCMAPS command stream into a per-title-tick
  playback timeline for preview ticks, cell-effect timing, fixed-wipe
  rectangles, trailing ticks, and temporary-actor convergence checkpoints.
  The loader copies the 4x19 on-disk strip source into the public 4x19
  visible preview, derives captions from LoadMapStrip, and applies the
  `(x, y + 7)` local cell-effect coordinate rule. Public issue `#117` / commit
  `fcc8181` supplies the exact opaque `0x05`/`0xDC` row-splice rasters and the
  corner-first plus `0xB8` 256-write single-cell permutation. Temporary actor
  draws now run 31 complete preview-tick/input checkpoints after write counts
  `8..248`, select overlay versus backing graphics directly, preserve the
  helper-owned suppression state during convergence, and write palette index
  zero opaquely. Open/close metadata carries the actual driver step values
  `1..15` and `15..1` instead of the former synthetic `0..14`.
- Combat rendering now consumes the post-round cursor/secondary-marker hook:
  the tactical viewport draws the blinking active-player cursor marker and
  explicit secondary marker cell from shared runtime state. The Bevy visual
  frame suite now adds all sixteen public `BRIT.CBT` outdoor arena gallery
  frames with accepted early replacement rolls plus a death/field/cursor marker
  gallery. Exact resident marker pixels remain visual parity work until
  published.
- Surface/town View rendering now has a synthetic public-class gallery that
  covers every published class `0x00..0x10` in ordinary gem, Peer, and X-Ray
  modes. Runtime pixel tests pin no-op versus rendered classes and the
  alternate-bank colors for `0x0A`, `0x0B`, and `0x0F`; the TUI and Bevy frame
  suites emit the same class-gallery PNGs for visual audit.
- Save/load now persists queued shipwright deliveries even when the player
  saves before leaving the town/shop scene. The published `SAVED.GAM` bytes at
  `0x03AD`, `0x03AE`, and `0x105F` preserve inactive and packed classes exactly;
  successful delivery clears only the class byte. Neither `SAVED.OOL` nor a
  per-plane mirror carries the queue. Regression tests cover frigate, skiff,
  packed-byte increment/wrap, save round trips, and delivery.
- Route smoke now exercises a debug-enter world-to-castle-to-world round trip
  using clean return metadata in memory, an Underworld-to-castle entry,
  seeded ship/skiff sailing routes, a Spyglass-triggered night-sky overlay,
  world/dungeon H-Hole-up rest routes, and direct U-Use routes for
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
- View/Peer/X-Ray overlay rasters now draw surface class `3` as a framed cell,
  class `0x0D` as a creature-on-terrain composite, and dungeon V-View from the
  published minimap glyph ids instead of the lossy diagnostic text glyphs.

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
  deterministic clean substitute and document the gap clearly.

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
    body-armour gates. It charges one turn per invocation, accepts Enter or
    Space as item confirmation, consumes the native up/down and four corner
    input codes, prints `Done` only for Escape from the item picker, and closes
    silently after a magic-ring vanish result.
  - Remaining work:
    - audit any item-specific equipment rules newly published in the spec,
    - keep town, world, dungeon, and combat routing covered by tests.

- Use (`U`).
  - The shared item picker accepts Enter or Space, native vertical/corner
    navigation, and the published `None!` Escape result. Success, handler
    refusal, no usable items, and cancellation each commit one normal action
    and run the current exploration mode's ordinary turn processing; picker
    navigation itself remains free.
  - Remaining work:
    - audit any item-specific activation rules newly published in the spec,
    - keep town, world, dungeon, and combat routing covered by tests.

- Yell (`Y`).
  - Current behavior separates ship sail toggles from generic Yell input and
    routes typed words strictly by scene: outdoor Words of Power, Shadowlord
    names only in the three Eternal Flame keeps, and generic no-effect elsewhere.
    A recognized Word scans west/south/east/north for an adjacent target,
    toggles `0xDF` with the word's own entrance tile at the published horizontal
    coordinate on either world surface, toggles the save-backed high bit, and
    dirties visibility. A ruined-shrine hit runs the published virtue plus three
    mantra prompts; success clears only that shrine's ruin high bit and restores
    its live tile. Submitting the ordinary prompt empty is an acted result in
    every mode, while opening the prompt remains free. World loads re-derive
    entrance and shrine presentation from the two eight-byte flag arrays without
    modifying asset files.
  - Remaining work:
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
    exact public foot/horse/carpet/ship/skiff static terrain predicates,
    wind-driven ship behavior, waterfalls, damage sidecars, encounters, and
    plane transitions.
  - Remaining work:
    - replace sidecar-only transition coordinates where public default tables
      become available,
    - continue expanding route-smoke scripts across more transition types,
    - audit horse stride edge cases around hazards, moongates, and encounters.

- Town movement.
  - Already supports the exact `0xC4..=0xC7` facing-sensitive walk-on stairs,
    directional K-Klimb links (`0xC8`, `0xC9`, and grate `0x86`), adjacent
    rubble/fence climb-over, generic post-turn `0x8C` trapdoors with carpet
    suppression and mass damage, exit boundaries, NPC blocking, schedules,
    doors, secret doors, fire sources, pickups, and full floor reloads.
  - Remaining work:
    - audit town boundary-exit presentation parity,
    - exact item/tile pickup mappings,
    - richer interaction with shop/counter furniture.

- Dungeon movement.
  - Already supports facing-relative movement, turning, ladders, fall traps,
    bomb traps, fields, wind tiles, scripted teleports, exit tiles, heavy doors,
    room combat handoff, and text proxy view.
  - Remaining work:
    - complete exact dungeon exit-cell identities when public data is available,
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
  - `codex_urns.tsv`
  - `dungeon_teleports.tsv`
  - `dungeon_chests.tsv`
  - `secret_doors.tsv`
  - `town_fire_sources.tsv` (now an override for native `0xB4..=0xB7` cannons)
  - `town_pushables.tsv`
  - `town_get_tiles.tsv`
  - `town_rest_beds.tsv` (optional override; native inn H-Hole-up accepts
    the public `0x48..=0x49` bed pair in published inn scenes)
  - `town_stairs.tsv`
  - `town_trap_doors.tsv`
  - `town_locks.tsv`
  - `eternal_flames.tsv` (override/extension for the public native flame table)
  - `location_floor_pages.tsv`
  - `location_entry_y.tsv`
  - `tile_passability.bin`
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
clean-engine session without losing supported state.

- Preserve unknown bytes.
  - Keep round-tripping unmapped `SAVED.GAM` and `SAVED.OOL` bytes.
  - Save/load, chargen, and U4 transfer save-pair reads/writes now route
    through the shared `disk_io` wrapper; tests cover zero-byte retry,
    nonzero short-read/short-write success, write-handler phase restoration,
    save-image/`SAVED.OOL` zero-byte failures, the load-time underworld
    mirror re-flush branch, the save-time entry-mode-gated extra
    `UNDER.OOL` write, and fast failure in the modern single-directory path.
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
      future public spec updates refine the current contract.

- Quest and shrine state.
  - Current shrine implementation uses public ordained/Codex masks and
    clean standing semantics.
  - Codex urn sidecar rows, virtue-page stamping, turn-in stat rewards,
    all-virtues-complete detection, and the menu-level Codex challenge are
    implemented and tested.
  - Remaining work:
    - exact shrine standing byte layout,
    - complete quest flags related to doors, NPCs, and permanent world changes,
    - continue broadening save/load tests for shrine completion, Codex urns,
      and Codex turn-in.

- Vehicles and active-effect tags.
  - Current tests cover ship/skiff/carpet/horse transport markers, hull/skiff
    side bytes for boarded and parked ships, board/exit/fire active-object
    overlay save/load, skiff/carpet ship-exit fallback save/load, and
    save/load after town and dungeon vehicle-adjacent return-world transitions.
  - Remaining work:
    - continue auditing exact ship facing/sail marker variants against any new
      public marker evidence,
    - continue auditing hull/skiff persistence across shop delivery and exotic
      transition paths,
    - continue auditing behavioral meanings for active-effect codes beyond
      currently recognized `Q` and `T`; unknown nonzero codes round-trip.

## Milestone 3: Rendering And Presentation

Goal: move from diagnostic terminal rendering to a real playable visual
experience.

### 2026-08-22 status

Six presentation packages landed together on `intro-preflourish-phase`, each
reconciled against `cleak/u5-spec` head `8192d67` and, where the spec was silent
or wrong, against black-box observation of the shipped assets:

- **Intro title/menu** - `TITLE.BIT`/`BRITISH.BIT` whole-page publish phases,
  the corrected `#67` flourish script at the `#77` 14 ms cadence, the menu
  screen from `ULTIMA.16` slot 0 with title-tick bands 1..=4, the measured
  rounded blue lower chrome, and the acknowledgements credits artwork.
- **Gameplay screen chrome** - the measured border, stats panel, sky strip,
  wind and `Dir:` banners, and the scrolling message window with verb echoes
  and the triangle-plus-barber-pole prompt cursor.
- **Visibility and lighting** - the interior visibility carve, the ambient byte
  read as a squared-distance threshold, and `#42`'s local light corrected to a
  squared-distance disc (`dx*dx + dy*dy <= 10`, 37 cells) rather than the
  withdrawn Chebyshev square. The local-light mask is now a persistent resource
  rebuilt only at `visibility.md §12.4`'s three published trigger sites, with
  the rotating night beacon stamped afterward before the visibility carve.
- **Commands and text** - the command-echo transcript and the wrap fix.
- **Intro story slides** - all 21 steps from observation-derived proportional
  metrics, including step 6 from the published `#69` doorway text, with `#53`'s
  rectangle dissolve replacing the withdrawn column sweep.
- **Endgame, chargen, U4 transfer, harness** - the endgame's shared gameplay
  surface and standing message window, the `§7.1` fade to black before the
  first `END.DAT` window, the per-member exact revival line and slot-ordered
  restoration/place/walk sequence, the chargen prompt screen at `§5.1`'s
  published cells, the U4
  retryable-media branch, and the guards that stop harness paths writing into
  the pristine asset install.

All six issues we filed were then answered, and that work is in too: the
published chrome contract and command echoes, the two-pass border end-cap
composite (a solid triangle glyph plus two accent strokes, shared by every
ribbon interruption, the Return-to-View caption wedges and the message-window
prompt - which is why byte-matching a single glyph always failed), the `#83`
local-light influence mask, the `#80` per-scene base-page table, and the `#82`
endgame/chargen/`PARTY.SAV` work.

**The victory ending is reachable.** The endgame certificate wording was the
last gate on an unpublished contract anywhere in the engine; `endgame.md
§9.1`-`§9.5` published it and the ending now runs end to end.

Three corrections from that round are worth remembering rather than just
recording. `#80` withdrew the `page = sub_map_index * 2 + floor` floor-page
model, which was wrong for 22 of 32 locations. `TORCH_LIGHT_FLOOR` and
`LIGHT_SPELL_FLOOR` were inverted in the engine - magic light is the brighter
one. And the visual frame suite was rendering no menu window at all while the
live path was correct, because the suite built its intro state through a
parallel path; it now drives the real render path, so a defect of that shape
cannot hide in the harness again.

The shipped palette is not stock EGA: index 6 is `(170, 170, 0)` dark yellow
rather than `(170, 85, 0)` brown, and it is the only index that differs. Forty-
two decoded sub-images contain it, so several screens changed hue. Nothing
reprograms the palette after mode setup - apparent recolouring is a restricted
plane write mask or a display effect mutating the loaded asset data.

This is a long step toward presentation parity, not parity itself, and none of
it is pixel-verified against the original for any screen not named above.

### 2026-08-23 status

The graphical U4 transfer preview is built (`f3ecfd1`, `cleak/u5-spec#73`), and
so is the acknowledgements phase sequence (`6db6135`, `#72`). Nothing published
on the intro path is unbuilt any more.

No fully published gameplay contract is currently known to be unimplemented. The
event-driven input boundary performs `main-loop.md §4` scene-byte dispatch
directly, collapsing the historical exit-pending flag as §14 permits. The
dungeon first-person renderer uses #84's published seven-family billboard slot
table; the withdrawn sparse-wall-table interpretation and #84's retracted
numeric pixel-ratio self-check have been removed. Public #100 now supplies the
backward pass: `ITEMS` object sprites, `MON0`-`MON7` wandering monsters, field
strobes, fountain water, decoration states, and the raw-`0x08` rising-pit
overlay are implemented. The sprite parser now treats its header as a sprite
count (20 `ITEMS` sprites and 6 per monster bank), fixing the old half-bank
decode. Public #101's exact setup, record, Negate Time, and tone contracts are
implemented too. Public issue #102 subsequently published and closed the exact
overworld-prune type-byte classifier, which is implemented as well.
Malformed present corridor or sprite resources fail instead of rendering blank.

Public issue #103 closed the last outdoor-reaction boundary. The walker stages
reactions in slot order from 31 down through 1, preserves lower reactions across
a terrain-combat frame, and keeps the first-reaction movement suppression as a
separate running gate. The generic arm implements the exact low-water and
carpet/skiff impact intersection; every other recognized hostile enters the full
class-and-arena terrain-combat path.

The rotating beacon, blocking moongate transit, outdoor active-object reactions,
and typed required-disk/session contract have all moved off this list. Issues
`#95` through `#99` also close the camp persistence/event gate, queued shipwright
save bytes, and shared regalia-effect/Blackthorn guard contracts.

Three models were retracted in the same pass and should not reappear:
`combat.md §7`'s post-round maintenance pass, the water/lava/brazier/torch
tile-animation families, and the per-render-frame moongate animator. Two
published mechanisms were connected for the first time: the active-object
eviction cascade and the spell scene allow-mask.

### Bevy Integration

- The Bevy visual frontend now exists behind `cargo run --features visual --
  --visual ...`. It opens one Bevy window, renders the shared CPU-generated
  frame surfaces into textures, and routes keyboard input through the same
  handlers used by the terminal play loop. Town/world scenes use the tile-atlas
  top-down view; dungeon scenes use a clean first-person raster with the public
  light gate, wall/feature cues, and active dungeon object overlays.

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
    source masks: source-to-target blocker carving, active-object flame
    sources, and multiple-source union. #42's "radius-three Chebyshev" reading
    was withdrawn - the source mask is a squared-distance disc,
    `dx*dx + dy*dy <= 10`, 37 cells. A local-light *influence mask* that
    reveals cells beyond the threshold is implemented from public issue `#83`;
    `§12.4`'s persistent-mask cadence and three explicit refresh triggers are
    implemented as well.
  - Top-down radius-5 raster rendering now drives the public `visibility.md`
    persistent scratch model: an 11-active-cell, 32-byte-stride visibility
    grid plus 16-byte-stride terrain companion band, full rebuild on dirty
    frames, lazy refill on clean frames, fog marker refinement, active-object
    companion stamps, and scratch-byte preservation.
  - Show status/message panels.
  - A spec-backed fixed-cell text-window core now covers the exact resident
    gameplay descriptor state: full-screen window 0, stats window 1, shared
    message/prompt window 2, and untouched unused window 3. It implements
    cursor preservation, style controls, combined CR+LF, nonblanking scroll,
    wrapped strings, numeric output, typed-input erasure, and a shared screen
    surface used by TUI and Bevy status/modal summaries. Bevy gameplay status
    renders the shared surface through `IBM.CH` into a texture.
  - Verify with screenshots or pixel hashes where practical.
  - `--visual-route-suite <DIR>` replays representative world, town modal,
    town View-overlay, and dungeon movement/search routes through the Bevy
    full-frame compositor, writes per-step PNGs plus a sanitized manifest, and
    fails if a scripted route step leaves the frame unchanged.
  - `--route-smoke --route-smoke-manifest <PATH>` writes a sanitized manifest
    for the non-visual route suite with initial, per-command, and final route
    labels, command counts, frame dimensions, hashes, nonblack counts, and
    state hashes.
  - `--compare-frame-manifests <BASE> <CURRENT>` compares sanitized manifests
    by coverage row, frame label, dimensions, frame kind, hash, nonblack count,
    and review metadata so PNG-generating suites can be used as a clean
    regression gate.

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
  - any wall-clock length for the rectangle dissolve. `#53` withdrew the
    one-column-per-title-tick wipe in full; the dissolve is one blocking call
    visiting every pixel once in a deterministic pseudo-random order, and the
    engine completes it as such rather than inventing a rate,
  - Return-to-View effect rasters are no longer deferred. Public issue `#117` /
    commit `fcc8181` publishes and the engine pins the exact shimmer row splice,
    single-cell permutation, opaque writes, source selection, and checkpoint
    schedule. Bevy repeats the expanded playback cycle when the shipped `0x09`
    restarts the stream and accepts Escape on every preview frame, so the
    attract scene loops until any preview-tick key aborts it rather than
    freezing after one cycle,
  - subtitle ignition is no longer deferred: public issues `#118` and `#120` /
    commits `12485b3` and `36780cb` publish the pass countdown/tails, polling and
    abort order, exact `0x3500` Galois vector, gate/pitch recurrences, pacing
    branches, publication anchors, and burst totals; runtime and Bevy implement
    them, including preserving an aborting key for the menu's first input poll
    instead of consuming it at the driver boundary,
  - the exact shop-owned window-1 clear/widen/restore shells and border-cell row
    metadata are implemented for the inn register and arms `S` browser. Public
    issue `#119` / commit `58e9b9c` now supplies the browser paging, rows,
    controls, sale/refusal continuations, and draw boundaries; these are
    implemented. Public issue `#121` / commit `5b9445f` also publishes the exact
    three-cell page badge and none/down/up/both fixed-font byte sequences. The
    compositor now paints that badge, plus the browser's `Arms` and selector
    `Select:`/`Items:` stats-ribbon labels, through the shared two-colour chrome
    cap primitive instead of flattening the caps in the later text overlay,
  - Combat cursor-box and secondary-marker raster geometry is no longer
    deferred. Public issue `#114` / commit `7046ca8` publishes the exact white
    two-pixel ring, four-group white/black secondary raster, eligibility gate,
    draw order, solid replacement writes, and display-clipping policy; runtime
    and Bevy pixel tests pin the complete result,
  - potion presentation is no longer deferred. Public issue `#115` / commit
    `edba057` publishes the selected-bottle 176-by-176 paired XOR flash and the
    Orange/Purple/White sound-loop values, Orange's persistent `0x1E` display
    tile and one-in-seventeen wake restoration, Purple's persistent two-field
    `0x90` rewrite, and White's frozen threshold-32 twenty-frame visibility
    repaint. Bevy consumes the flash as a blocking pre-effect framebuffer
    event and paces White from the typed one-BIOS-tick playback field without
    double-advancing animation. Terminal and headless raster paths now execute
    the same shared sound-disabled timing work and complete all twenty White
    frames before accepting another command or saving the resulting frame.
    The static White marks, sleep `Z`, and one-frame Poof star are removed,
  - public issue `#116` / commit `01e2e1b` completes the timing table for all
    eight selected bottles: rumble target `8,000 + 1,600i`, then two sweeps of
    `10,000 + 4,000i` iterations. Sound-disabled Bevy playback executes all
    three work loops, and input batching stops at the blocking flash boundary,
  - public issue `#122` / commit `c869c5b` closes the remaining presentation
    status-poll boundary. The first gated start/menu dissolve copies before its
    odd-visit polls, so a pending-at-entry key leaves exactly `(1,0)` copied;
    the loader then consumes that key, locally downgrades to an instant plain
    completion, skips subtitle ignition, and leaves the caller's automatic
    Return-to-View one-shot armed. Return-to-View consumes every abort key and
    restores the menu for a fresh poll with no key handoff,
  - public issue `#123` / clean-spec commit `4d03a662` closes the Stonegate
    trapdoor gap. After generic mass damage, Stonegate stays on the current
    floor, records its direct-black/tone/rumble blocking presentation, fills
    the 1,024-cell town grid with `0x8F`, clears all 32 object records before
    restoring only slot-zero X/Y/Z, and sets every in-party member to zero HP
    and Dead. It writes no durable imprisonment flag and adds no time beyond
    the triggering town action. Asset-backed TUI and Bevy routes now discover
    a live Stonegate trapdoor tile at runtime and verify the automatic
    next-command-boundary rescue without committing map coordinates,
  - broader `EGA.DRV` behavior beyond the canonical EGA/Tandy-equivalent path,
  - empirical screenshot QA for the now-published local View and dungeon
    minimap pixel contracts. The control flow, 4x4 class strokes, river/road
    rules, two-font glyph table, vectors, flood order, and bounds are no longer
    deferred.
- No longer deferred: the title-tick silhouette pixels are `ULTIMA` records
  1..=4, read from the shipped asset at runtime. The clean-room flame generator
  and palette-cycle table are deleted.
- These do not block current public-depth gameplay, but should be tracked if
  exact visual parity becomes the target.

## Milestone 4: Magic And Effects

Goal: finish non-combat spell effects and keep combat-only spells aligned with
the public combat contract while exact combat presentation evolves.

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
    - implemented as a clean substitute,
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
    (`1..=3`) and cap behavior.
  - Rel Hur uses the public `weather.md` prompt-to-wind mapping and is covered
    by cast/resource-order tests.
  - Non-combat Blink uses the public `cleak/u5-spec#48` cardinal direction
    prompt and farthest-grass ray landing rule; combat Blink uses the current
    arena state and legal in-arena landing checks.
  - View, Peer, and X-Ray overlays now carry explicit runtime modes, and the
    surface/dungeon overlay rasters apply the public peer/gem alternate
    bank/tint branch for affected cell classes. Exact historical local-view
    source pixels remain presentation parity work.
  - Dungeon Up/Down spells implement the public one-level movement hook inside
    level bounds; the command-overlay dungeon escape helper remains separate
    and does not currently imply a spell-dispatch gap.
  - Combat-side active-effect consumers are implemented broadly. The live
    Negate Magic gate now requires the shared `N` tag and a nonzero duration,
    and Protection remains timer/display state only: the invented `+3` party
    spell-defense consumer has been removed. Parity still needs audit coverage.

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

Goal: continue replacing substitute combat coverage with spec-backed parity
as public details become available.

- Current combat handoff.
  - Hostile world-object contact, Attack against combat-class objects, dungeon
    rooms, rest ambushes, and outdoor encounters can enter a combat frame.
  - Dungeon-room combat now uses the public issue #12/#19 source rules for
    high-bit-masked ordinary monsters, compact source-order placement, special
    id auxiliary-byte post-write formulas, `0xEC..0xEF` random-special
    pre-roll selectors, Doom's absorbable field, and metadata-driven party
    positions.
  - Dungeon active-monster combat now follows public issue #21: it uses the
    ambush framer path, builds a stock-floor 11-by-11 arena without loading
    `DUNGEON.CBT`, and creates exactly one initial monster at the central-front
    placement.
- Dungeon `0xF?` cells now follow the latest public dungeon-mode contract as
    walkable room triggers, `0xA?` cells remain navigable room-helper state,
    and `0xE?` heavy-door variants block forward movement while retaining
    their door presentation. Dungeon Open and Jimmy no longer mutate `0xE?`,
    `0xF?`, or `0xA?` cells, stale
    `dungeon_doors.tsv` files are ignored, and dungeon Open now uses the public
    underfoot `0x7?` "Chest opened" / default "What?" messages.
  - Terrain combat now uses the public issue #3 combat-class stat-field spawn
    count plus `BRIT.CBT` record placement metadata, clamps requested counts
    above the slot table instead of reproducing the original out-of-bounds edge,
    places the party after the monster slots, and applies the public
    one-in-nine early-spawn replacement-tile roll through the main
    terrain-combat path. Party combat descriptors now seed byte 3 as the
    party/character slot link while retaining class-derived speed in byte 1.
    Exact visual review of every replacement byte remains combat parity/audit
    work.
  - Combat-frame entry snapshots the previous mode state, loads clean runtime
    arena data, places actors, and restores the suspended state on exit.
  - Tests cover active-object preservation, trigger-slot reconciliation, actor
    setup, room/ambush routes, and several spell/effect paths.

- Full combat loop.
  - Continue auditing actor initiative/phase parity.
  - Continue auditing player movement and targeting parity.
  - Continue auditing monster AI parity.
  - Combat descriptor byte-2 flags now use the public issue #6/#7 controlled
    and flee bits. Charmed/possessed actors and controlled Conjure/Swarm actors
    route through the player-command path. Conjure, Swarm, and party Summon use
    the shared whole-candidate `0..=15` arena probe; Swarm places up to four
    actors at its one accepted cell. Party Summon stamps controlled only after
    its skewed-roll self-check succeeds, while Oops and monster-AI summons leave
    the Daemon hostile. Issue #8
    non-party sleep now has the published own-turn 1-in-17 wake check; disabled
    actors remain present, occupy their cells, and spend the dispatch that
    clears the bit.
  - Combat field placement now separates marker materialization from common
    post-dispatch contact and follows public issues #10 and #125: player combat
    C-Cast Fire/Poison/Sleep/Energy Field uses the arena cursor and a confirmed
    impact coordinate, not an adjacent direction prompt. Cursor Escape cancels
    after charge/mana debit but before marker placement; Fire/Sleep/Energy no
    longer use a random placement gate, while Poison still uses its
    unconditional placement path. Spell-name Escape/blank and follow-up Escape
    now finish the already accepted C-Cast action, run committed-action
    maintenance, and resume the round instead of granting the caster a free
    retry. After any completed player or automatic actor dispatch, the acting
    descriptor remains the target; the marker scan skips only its linked
    renderer record and takes the first separate colocated marker in ascending
    active-object order. Poison and Fire use one direct raw `0..20` or `0..10`
    draw only on their damage arms, Sleep uses no hook-local draw, Energy blocks
    both player and AI movement without a contact payload, and markers are not
    consumed. Parser-local refusals and blocked-direction re-prompts do not run
    the hook. Public issue #126 resolves the priority arm: exact terrain byte
    `0x04` selects Poison and `0x8F`/`0xBC` select Fire before any marker scan;
    every other terrain byte falls through, and a selected terrain arm suppresses
    markers even if Poison is later rejected. Doom absorption is not one of
    those contact arms: the committed non-digit player-action tail first checks
    whether its live actor stands on row 2 with renderer companion-band byte
    `0x3C..0x3F` immediately north on row 1. It consumes no PRNG; digit selection,
    parser refusal, and automatic actor dispatch skip it.
  - Combat-local ambush/camp reveal records now follow the public helper shape:
    up to eight trigger coordinates, consume-on-fire, one or two in-range
    terrain stamps, out-of-range target sentinels, ordinary-combat clearing, and
    post-committed-movement dispatch for player and AI movement paths.
  - Combat round-counter wrap now applies the one-minute combat-safe clock
    advance. **Retracted:** the "post-round maintenance pass" that used to be
    described here was an invented contract, removed in `60ec07c` after
    `cleak/u5-spec#86`. The row-major terrain/effect sweep never did anything -
    both call sites discarded its report and `combat_magic_effect_timer` was
    write-only - and route-smoke's cases were unchanged by its removal. What
    is real is the combat-only cursor highlight (blink toggle, active-actor box,
    optional secondary marker), which has live renderer consumers.
  - The all-48-spell production audit found that Combat Vanish, Magic Lock,
    Unlock Magic, and Open were still wired to a retracted unconditional-failure
    substitute. They now share `magic.md §8`'s cardinal live-tile helper in town
    and combat: Vanish uses the exact thirteen-id set and writes `0x44`, Open
    performs `0xB9 -> 0xB8` / `0xBB -> 0xBA` plus the kind-1 chest-bit arm,
    Magic Lock maps the two ordinary door pairs to `0x97`/`0x98`, and Unlock
    Magic performs the exact inverse. The active combat actor supplies the
    arena origin. Resource gates run before the direction follow-up, Escape
    re-polls, and Space/Pass quietly commits the already-spent cast.
  - Combat U-Use now follows the published live-actor-gated multistage branch
    and enters the shared item picker. Use, Ready, and Z-stats retain the acting
    slot while their modal is open and end that combatant's action when it
    closes; the withdrawn `Use-Not here!` branch is removed.
  - Combat direction prompts now honor §8's exhaustive re-prompt rule. Push and
    Get/Jimmy/Open/Search run committed-action maintenance and end the action
    after either a direction or prompt cancellation; Klimb cancellation also
    commits, while an actual blocked Klimb remains the named free retry.
  - Combat Cast interference now follows public issue #111 and `magic.md §7`.
    The save-backed 32-victim source map preserves the factory zero seed and
    survives combat boundaries; ordinary automatic adjacent hits and misses
    write or overwrite it, while ranged, failed-range, and controlled attacks
    preserve the old source. C-Cast revalidates source hostility, visibility,
    awake state, adjacency, and Negate Time before either re-prompting the same
    actor for interference or entering the spell prompt. Only a completed
    victim action clears that victim's entry; skipped/not-ready slots and free
    re-prompts leave it intact.
  - Default monster death/drop markers, party corpses, vanish-on-death actor
    clearing, Gazer eye-burst, and Gargoyle lava-then-default-death transitions
    are implemented in the temporary combat active-object table; continue
    auditing damage, defense, status, rewards, loot, and escape parity.
  - The current public combat spec explicitly bounds monster AI to helper-driven
    target selection, movement, morale, and the fixed possess -> blink ->
    summon-daemon class-flag hook rather than a general per-class script
    runner. Engine tests now pin every published combat stat row, ranged/effect
    side row, and the fixed three-bit hook order. Remaining combat parity work
    is focused on behavioral/visual audit coverage and any future public data
    tables rather than inventing an unpublished AI instruction interpreter.

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
  - Current sidecars cover town locks and secret doors; stale dungeon-door
    sidecars are ignored because public dungeon packed-cell classes own those
    semantics directly.
  - Remaining work:
    - exact surface lock-state byte pairs,
    - cannon/fire durability details.

- NPC schedules and conversations.
  - Current schedules link and move NPCs in town-family scenes, preserve
    cached-waypoint movement state until a transition settles, and route
    floor changes through the `0xC8`/`0xC9` floor-link marker BFS. The
    shipped `.NPC` corpus now runs boundary-hour, multi-floor scheduler
    routes when local clean assets are present, including active-object
  relinking, hidden-sprite suppression, one-cell-per-tick movement, and
  linked `0xFC` scheduled NPC preservation during player-slot sync. Slot-zero
  sentinel records are skipped by runtime loading, live NPC/Talk/attack
  lookups, and saved-active-object relinking even if their stored bytes are
    nonzero; validators do not reject a nonzero slot-zero type/tag byte.
  - Conversation sessions cover ASK-PARTY-NAME, ASK-WHO, non-`JOIN`
    recruitment prompts for roster companions, and non-roster name prompts
    without accidental joins.
  - Remaining work:
    - exact audit of every authored schedule/AI edge,
    - exact authored keyword paths for content-specific conversation effects,
    - broader shop/service conversation content audits,
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

- Existing local decoders cover many files, paired-graphics LZW resources, and
  canonical sparse `.BIT` / `PROPORT.PCS` driver resources. Local preprocessed
  `.BIT` / `PROPORT.PCS` wrappers are handled only through explicit legacy
  compatibility paths and remain noncanonical.
- Continue adding tests that verify:
  - declared decoded lengths,
  - parser shape,
  - aggregate hashes or counts,
  - no raw asset dumps in repo outputs.

- Areas to audit:
  - all `.DAT` map and metadata files used by runtime systems,
  - `LOCATION.DAT` now has a sanitized aggregate authored-cell audit covering
    all four shipped town-family files when local clean assets are present;
    keep reports to counts, hashes, and anomaly totals rather than raw map rows,
  - `.OOL` active-object overlays now have sanitized aggregate audits covering
    `SAVED.GAM`, `SAVED.OOL`, `BRIT.OOL`, `UNDER.OOL`, and `INIT.OOL` when
    local clean assets are present; keep reports to counts, hashes, mirror
    checks, and anomaly totals rather than raw slot inventories,
  - tile sheets now have a sanitized aggregate audit covering the shipped
    EGA and CGA tile atlases when local clean assets are present; reports stay
    to counts, hashes, palette masks, and nonzero totals rather than raw
    pixels,
  - fixed-cell `.CH` and `.HCS` fonts now have a sanitized aggregate audit
    covering the shipped IBM and Runes resources when local clean assets are
    present; reports stay to glyph counts, dimensions, bit masks, hashes, and
    nonzero totals rather than raw glyph rows,
  - proportional text sparse-resource glyph mapping and noncanonical local
    preprocessed variant compatibility,
  - bitmap sparse-resource rendering versus noncanonical local preprocessed
    variants,
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

- Keep the clean-engine status matrix current.
  - Rows: each command/system.
  - Columns:
    - world,
    - town,
    - dungeon,
    - implemented,
    - clean substitute,
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
  - Every behavior that is a clean substitute should say so.
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
