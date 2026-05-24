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
  2026-05-24 with 493 scripted route cases and the sanitized
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
  combat utility fallback casts,
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
  dungeon turn/block movement, public `0xE?` heavy-door blocking, dungeon exit confirmation/refusal, a Doom room
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
  `450b8690ef5bc292`, `britannia-chunk-map-overlay`
  `12d68ef9587532c6`, `peer-view-overlay` `2c64191172043730`,
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
  `route-britannia-spyglass-chunk-map-00-initial` `ee035bc3da0ecedd`,
  `route-britannia-spyglass-chunk-map-01-usp` `00e243b8973a3bc5`,
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
  Use/Drop/Wear/Enter/Fire/Hole-up/Ignite/Mix/New Order/Talk/View/Look
  refusal or label branches, Cast/Get/Jimmy/Open/Push/Klimb directed
  prompts, Ready, Yell, and X-it, including representative terminal frames
  `route-doom-combat-cast-refusal-02-c1il` `325a6f1641bb2455`,
  `route-doom-combat-ready-prompt-02-r` `c1ffc74f45610145`,
  and `route-doom-combat-xit-foes-remain-02-x` `c1ffc74f45610145`. The
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
  Wind/Death Wind/Flame Wind combat casts, combat utility fallback casts,
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
- Town-family exit thresholds now prompt both when stepped onto and when
  observed underfoot after a consumed turn; accepting exits through clean
  return metadata, refusing leaves town mode active.
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
- U4 transfer source validation now follows public `cleak/u5-spec#16`:
  fixed 532-byte `PARTY.SAV`, public offsets for move/moon/dungeon counters,
  gold/food/keys/torches/gems/sextants counters, leading class/name, and the
  eight-byte no-transferable-data virtue gate.
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
- TLK `0x85` accepted toll payments debit gold, increment the toll-progress
  counter, and apply the published milestone karma behavior from
  `cleak/u5-spec#27`.
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
  with lore gated behind an accepted continuation branch.
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
- The 2026-05-24 clean-engine audit retired #1/#3/#5/#9/#13/#18/#20/#31/#36/#38/#41/#43/#47/#49/#51/#54/#56/#57
  as gameplay blockers after applying the current public answers and checked-in
  spec. Current response-needed public blockers are #8, #10, #11, #12/#19,
  #53, #58, #59, #60, #61 (town free-roaming active-object walker), and #62;
  clean-engine follow-up comments are
  currently latest on each of those issues, so do not post duplicate comments
  unless new spec evidence or implementation questions appear.
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
  rectangles, eight-title-tick waits, trailing ticks, and one-shot actor draws.
  The loader copies the 4x19 on-disk strip source into the public 4x19
  visible preview, derives captions from LoadMapStrip, and applies the
  `(x, y + 7)` local cell-effect coordinate rule. Exact effect rasters remain
  presentation work.
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
  saves before leaving the town/shop scene. The `SAVED.OOL` encoder gives the
  matching `return_world.pending_vehicle` priority over a stale cached world
  overlay, mirrors the result to `BRIT.OOL`, and regression tests cover both
  frigate and skiff deliveries.
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
    exact public foot/horse/carpet/ship/skiff static terrain predicates,
    wind-driven ship behavior, waterfalls, damage sidecars, encounters, and
    plane transitions.
  - Remaining work:
    - replace sidecar-only transition coordinates where public default tables
      become available,
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
  - `dungeon_deeper_transitions.tsv`
  - `dungeon_teleports.tsv`
  - `dungeon_exit_tiles.tsv`
  - `dungeon_chests.tsv`
  - `secret_doors.tsv`
  - `town_fire_sources.tsv` (now an override for native `0xB4..=0xB7` cannons)
  - `town_pushables.tsv`
  - `town_get_tiles.tsv`
  - `town_rest_beds.tsv` (optional override; native inn H-Hole-up accepts
    the public `0x48..=0x49` bed pair in published inn scenes)
  - `town_stairs.tsv`
  - `town_trap_doors.tsv`
  - `town_exit_tiles.tsv`
  - `town_locks.tsv`
  - `eternal_flames.tsv` (override/extension for the public native flame table)
  - `moongates.tsv`
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
  - wider story/endgame rectangle transition helper rates beyond the
    published step-1 one-column-per-title-tick wipe,
  - Return-to-View strip geometry and exact effect rasters (the public #54
    scheduler timing and fixed captions are now modeled in runtime state),
  - broader `EGA.DRV` behavior beyond the canonical EGA/Tandy-equivalent path,
  - exact historical title-tick silhouette pixels,
  - exact remote-view panel for X-Ray/Peer,
  - exact dungeon minimap glyph/floodability edge cases.
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

Goal: continue replacing substitute combat coverage with spec-backed parity
as public details become available.

- Current combat handoff.
  - Hostile world-object contact, Attack against combat-class objects, dungeon
    rooms, rest ambushes, and outdoor encounters can enter a combat frame.
  - Dungeon-room combat now uses the public issue #12/#19 source rules for
    high-bit-masked ordinary monsters, compact source-order placement, special
    id post-write categories, `0xEC..0xEF` random-special selectors, guarded
    unpublished special effects, Doom's absorbable field, and party positions
    after the placed ordinary monsters.
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
    and flee bits. Charmed/possessed and summoned non-party actors route through
    the player-command path, Conjure uses fresh random arena-coordinate attempts,
    Swarm uses the caster ring, and player Summon uses independent random
    arena-coordinate probes plus its self-checking Oops branch. Issue #8
    non-party sleep now has the published per-slot
    countdown/targetability behavior, but exact per-effect starting durations
    and descriptor-byte table wording still need a public spec clarification
    before claiming exact monster sleep wakeup parity.
  - Combat field placement now separates marker materialization from post-step
    contact and follows the corrected public issue #10 answer: Fire/Sleep/
    Energy no longer use a random placement gate, while Poison still uses its
    unconditional placement path.
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
  - The current public combat spec explicitly bounds monster AI to helper-driven
    target selection, movement, morale, and the fixed possess -> blink ->
    summon-daemon class-flag hook rather than a general per-class script
    runner. Engine tests now pin every published combat stat row, ranged/effect
    side row, and the fixed three-bit hook order. Remaining combat parity work
    is focused on behavioral/visual audit coverage, exact issue #8 duration
    constants, and any future public data tables rather than inventing an
    unpublished AI instruction interpreter.

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
