# Status Matrix

This matrix summarizes current implementation status against the active
full-game goal. It is intentionally evidence-oriented: passing tests are useful
only for the behavior they actually cover.

Last refreshed on 2026-05-19 during the combat death/drop marker, intro-menu,
and visual-frame audit.

| Area | Current status | Evidence | Remaining risk |
|---|---|---|---|
| Intro/menu | Terminal and Bevy intro shells use the runtime menu dispatcher; Journey Onward, Create Character, and U4 transfer have runtime flows. | `intro_menu`, `menu_dispatch`, `chargen`, and `u4_transfer` tests. | Visual polish and full flow screenshot coverage. |
| World mode | Movement, vehicles, hazards, waterfalls, moongates, plane transitions, native and sidecar encounters, active objects, save/load, and many commands are implemented. Natural moongate live-tile refresh and entry are implemented when callers seed the moon-glyph cache; otherwise entry refuses instead of guessing. | `cargo test -p u5-runtime`, world tests across chunks 03, 06, 07, 10, 12, 13, 15, 17, 23; `cargo run -p u5-tui -- --route-smoke C:\Games\U5-Clean` passed 44 cases on 2026-05-19. | Exact public coordinate tables remain intentionally absent from the gazetteer; sidecars/debug entry cover those paths. Moon-glyph cache table semantics are blocked on public spec clarification in cleak/u5-spec#38. |
| Town mode | Movement, NPC schedules, stairs, trap doors, exits, doors, pickups, rest beds, talk, shops, Blackthorn paths, alarms, Search object-table pickups, Search trap narration, object-table chest contents/traps, and save/load are implemented. NPC schedules preserve movement state after boundary hours and use `0xC8`/`0xC9` multi-goal floor-link routing. | Town tests in chunks 04, 06, 10, 11, 15, 19, 21, 23 plus focused `scheduled_npc`, `town_search`, `object_pickup`, and chest/trap filters. | Exact cataloguing of every authored cell, full schedule/AI audit, poison-gas doorway odds blocked on cleak/u5-spec#51, and richer visual presentation. |
| Dungeon mode | Facing-relative movement, fields, traps, room combat handoff, doors, Jimmy/Open/Get/Search chest handling, generated chest rewards, teleports, exits, ladders, light-gated raster, and save/load are implemented. | Dungeon tests in chunks 05, 12, 13, 18, 20, 23. | Exact visual parity and any coordinate rows not public in spec. |
| Combat | Combat frame setup/restore, player commands, monster AI, spell paths, fields, damage/status/death, victory/defeat restoration, and special handoffs have broad tests. Recent work covers temporary default death/drop markers, vanish-on-death actor clearing, wound morale/flee movement, Saduj/name faction grouping, Doom/Shadow Lord suppression bypasses, non-party Sleep Field disable state, summon-daemon direction preference, and blocked-arena-cell dispatch skips. | Combat-heavy tests in chunk 23 plus focused `combat_ai`, `combat_actor_slot_dispatch`, `arena_field_contact`, `directed_spell_status`, `combat_monster_default_death_materializes`, and `cause_fear` filters. | Full parity audit still needed for every arena and monster class behavior; exact Gazer/Gargoyle special-death presentation remains tied to public marker details. |
| Magic | All 48 parser rows route through implemented, scene-gated, or correct-refusal paths; major combat spell families are implemented. Rel Hur follows the public prompt-to-wind table. | `magic.rs` metadata tests and cast tests in chunks 16, 17, 18, 23. | Create Food's exact grant is blocked on cleak/u5-spec#49. Non-combat Blink's default range/search rule is blocked on cleak/u5-spec#48. Some non-load-bearing visuals are first-playable overlays rather than exact presentation. |
| Rest/camp | H-Hole-up prompt flow, town bed gating, simulated-time cadence, watch prompt validation, sleep-ambush predicate, status restoration, hourly provision cadence, and Lord British camp event are implemented. | Rest, camp, and hourly provision tests in chunks 01, 03, 13, 19 plus route-smoke H-Hole-up coverage. | Exact HP/MP recovery amounts remain local policy until clean spec clarification in cleak/u5-spec#47. Hourly poison/starvation damage amounts remain local policy until clean spec clarification in cleak/u5-spec#50. |
| Shops | Arms, healers, inns, taverns, sages, reagent sellers, guilds, shipwrights, horse traders, and companion flows are modeled. | `shop_runtime` tests and end-to-end talk/shop tests in chunk 21. | Exact bark layout/pacing and every shop content edge should continue to be audited. |
| Conversations | TLK runner, keyword loop, scoped prompts, ASK-PARTY-NAME, ASK-WHO, roster-companion recruitment prompts, action dispatch, shop routing, dictionary expansion, and TLK `0x85` affordability/debit handling are implemented without committing transcripts. | `conversation_session`, `tlk_runner`, and chunk 21 tests, including focused `active_conversation`, `ask_who`, `conversation_join`, and `town_raw_tlk_gold_payment` filters. | Content-specific side effects and NPC memory flags need continuing audit; TLK `0x87` semantics are blocked on public spec clarification in cleak/u5-spec#46. The TLK `0x85` toll-style moral-standing milestone is blocked on cleak/u5-spec#27 because the public spec does not yet identify the toll-progress counter, milestone predicate, or qualifying payment contexts. |
| Save/load | Known save fields, active objects, spell/reagent stock, transport markers, overlays, dungeon working buffer, and mirror files are covered. | Save/load tests in chunks 03, 12, 23. | Unknown byte preservation must be kept when adding new durable fields. |
| Rendering | TUI text/raster diagnostics, headless `--save-frame` PNG capture, `--save-frame-suite` PNG batches, Bevy atlas-backed views, route-smoke scripted hashes, Bevy `--visual-frame-suite` PNG batches, and a fixed-cell text-window runtime surface exist for world, town, dungeon, combat, intro, status, and modal panels. The TUI frame suite writes nonblank world, town, dungeon, combat, surface View, dungeon View, Peer, X-Ray, intro-menu, status-window, Z-stats-modal, and endgame-status PNGs. The Bevy visual frame suite writes fifteen nonblank composed PNGs for initial/stepped world, town, lit and dark dungeon, synthetic combat, surface/dungeon View overlays, Peer/X-Ray overlays, Z-stats modal, endgame status, intro menu, intro story art, and Return-to-View preview. The route-smoke suite now runs 44 asset-backed world/town/dungeon/combat cases including save prompts, look/pass flows, View overlays, Peer/X-Ray overlays, Underworld startup, debug-enter return, seeded ship/skiff routes, dungeon exit prompts and A/S/G/J/O/refusal routing, Doom room combat trigger, and combat pass/A/C/G/J/O/P/K/R/Z/refusal/View/Yell/X-it/Search-prompt command routing. Bevy gameplay status renders the shared text-window surface through the runtime-loaded `IBM.CH` font into a texture. | Raster hash, save-frame-suite, visual-frame-suite, and route-smoke notes in `TODO.md`; text-window and fixed-font renderer tests in chunks 13 and 14; `u5-tui` save-frame/play-loop/route-smoke tests; Bevy framebuffer and visual frame-suite tests for world, town, dungeon, combat, View/Peer/X-Ray overlays, intro, status, and endgame modal surfaces. | Exact original modal rectangles, full UI composition parity, and a human-reviewed frontend screenshot gallery remain presentation work. |
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
