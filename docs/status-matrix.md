# Status Matrix

This matrix summarizes current implementation status against the active
full-game goal. It is intentionally evidence-oriented: passing tests are useful
only for the behavior they actually cover.

Last refreshed on 2026-05-19 during the View route-smoke coverage work.

| Area | Current status | Evidence | Remaining risk |
|---|---|---|---|
| Intro/menu | Terminal and Bevy intro shells use the runtime menu dispatcher; Journey Onward, Create Character, and U4 transfer have runtime flows. | `intro_menu`, `menu_dispatch`, `chargen`, and `u4_transfer` tests. | Visual polish and full flow screenshot coverage. |
| World mode | Movement, vehicles, hazards, waterfalls, moongates, plane transitions, encounters, active objects, save/load, and many commands are implemented. | `cargo test -p u5-runtime`, world tests across chunks 03, 06, 10, 12, 15, 17, 23. | Exact public coordinate tables remain intentionally absent from the gazetteer; sidecars/debug entry cover those paths. |
| Town mode | Movement, NPC schedules, stairs, trap doors, exits, doors, pickups, rest beds, talk, shops, Blackthorn paths, alarms, and save/load are implemented. | Town tests in chunks 04, 06, 10, 11, 19, 21, 23. | Exact cataloguing of every authored cell and richer visual presentation. |
| Dungeon mode | Facing-relative movement, fields, traps, room combat handoff, doors, chests, teleports, exits, ladders, light-gated raster, and save/load are implemented. | Dungeon tests in chunks 05, 12, 13, 18, 20, 23. | Exact visual parity and any coordinate rows not public in spec. |
| Combat | Combat frame setup/restore, player commands, monster AI, spell paths, fields, damage/status/death, victory/defeat restoration, and special handoffs have broad tests. | Combat-heavy tests in chunk 23. | Full parity audit still needed for every arena and monster class behavior. |
| Magic | All 48 parser rows route through implemented, scene-gated, or correct-refusal paths; major combat spell families are implemented. | `magic.rs` metadata tests and cast tests in chunks 16, 17, 18, 23. | Some non-load-bearing visuals are first-playable overlays rather than exact presentation. |
| Shops | Arms, healers, inns, taverns, sages, reagent sellers, guilds, shipwrights, horse traders, and companion flows are modeled. | `shop_runtime` tests and end-to-end talk/shop tests in chunk 21. | Exact bark layout/pacing and every shop content edge should continue to be audited. |
| Conversations | TLK runner, keyword loop, scoped prompts, action dispatch, shop routing, and dictionary expansion are implemented without committing transcripts. | `conversation_session`, `tlk_runner`, and chunk 21 tests. | Content-specific side effects and NPC memory flags need continuing audit. |
| Save/load | Known save fields, active objects, spell/reagent stock, transport markers, overlays, dungeon working buffer, and mirror files are covered. | Save/load tests in chunks 03, 12, 23. | Unknown byte preservation must be kept when adding new durable fields. |
| Rendering | TUI text/raster diagnostics, headless `--save-frame` PNG capture, `--save-frame-suite` PNG batches, Bevy atlas-backed views, route-smoke scripted hashes, Bevy `--visual-frame-suite` PNG batches, and a fixed-cell text-window runtime surface exist for world, town, dungeon, combat, intro, status, and modal panels. The TUI frame suite writes nonblank world, town, dungeon, combat, surface View, dungeon View, Peer, X-Ray, intro-menu, status-window, Z-stats-modal, and endgame-status PNGs. The Bevy visual frame suite writes fourteen nonblank composed PNGs for initial/stepped world, town, lit and dark dungeon, synthetic combat, surface/dungeon View overlays, Peer/X-Ray overlays, Z-stats modal, endgame status, intro menu, and Return-to-View preview. The route-smoke suite now runs 24 asset-backed world/town/dungeon/combat cases including save prompts, look/pass flows, View overlays, Peer/X-Ray overlays, Underworld startup, debug-enter return, seeded ship/skiff routes, dungeon exit prompts, Doom room combat trigger, and combat View label routing. Bevy gameplay status renders the shared text-window surface through the runtime-loaded `IBM.CH` font into a texture. | Raster hash, save-frame-suite, visual-frame-suite, and route-smoke notes in `TODO.md`; text-window and fixed-font renderer tests in chunks 13 and 14; `u5-tui` save-frame/play-loop/route-smoke tests; Bevy framebuffer and visual frame-suite tests for world, town, dungeon, combat, View/Peer/X-Ray overlays, intro, status, and endgame modal surfaces. | Exact original modal rectangles, full UI composition parity, and a human-reviewed frontend screenshot gallery remain presentation work. |
| Clean-room hygiene | Runtime reads local assets; repo excludes game assets and generated raw dumps. | `.gitignore`, report policy, parser tests, clean status checks. | Continue reviewing any new reports or fixtures before commit. |

## Current Verification Baseline

Run these before broad gameplay commits:

```powershell
cargo fmt -- --check
cargo test -p u5-runtime
cargo test -p u5-tui
git diff --check
```

For visual/raster work, also run at least one scripted raster smoke and one Bevy
scene launch with local assets:

```powershell
cargo run -- --route-smoke C:\Games\U5-Clean
cargo run -- --save-frame-suite target\frame-suite C:\Games\U5-Clean
cargo run -- --save-frame screenshots\britannia.png --scene BRITANNIA C:\Games\U5-Clean
cargo run --features visual -- --visual-frame-suite target\visual-frame-suite C:\Games\U5-Clean
cargo run --features visual -- --visual --scene BRITANNIA C:\Games\U5-Clean
```

## Not Complete Yet

Do not mark the full-game goal complete until a final audit maps every public
spec deliverable to concrete engine evidence, including saved frames or playable
frontend screenshots for world, town, dungeon, combat, intro, status/modal, and
endgame scenes. Current tests are broad but still not a complete proof of 100%
parity.
