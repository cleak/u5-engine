# Paired play-test scenarios

Each `.tsv` file drives the RemoteGameDev `game-dev-u5-paired` harness: the
same keystrokes go to the clean engine and to stock `ULTIMA.EXE` under the
pinned DOSBox Staging 0.82.2, both started from fresh writable copies of the
same asset set, and every `shot` step captures both windows side by side.

Columns are tab separated: `side` (`both`, `engine`, `dosbox`), `action`
(`wait` milliseconds, `key` xdotool key names separated by spaces, `type`
printable text, `shot` label, `note` text), `value`, and an optional caption.

Captures are black-box comparison evidence and stay private under
`~/artifacts/u5/paired/`; only sanitized `record.json` metadata may be copied
into evidence. Differences found this way are classified per the remote
development plan before any fix: engine defect, harness defect, spec gap,
asset/profile problem, or platform defect.

The DOS boot preamble (production card, title flourish, signature) is skipped
with a Space keypress on both sides; the engine accepts the same skip. The
shipped `SAVED.GAM` has no active Avatar, so every gameplay scenario first
creates a character with identical answers; the questionnaire's virtue pairs
are PRNG-selected and legitimately differ between the two sides.

## Seeds that start inside a town do not work

A save this engine writes inside a town-family location leaves the
`0x07B4..0x105F` NPC band zero (`formats/saved-gam.md` §12), and
`conversation.md` §2 step 5 says what the stock then does with it: **every
NPC in the location answers `No response!`**. Measured 2026-09-07 - a
`--scene TOWNE:1 --at 14,11` seed draws the right town with the right cast
on both sides, `Look` names `a merchant`, and `Talk` refuses in the stock at
both 10:00 and 13:00. Tracked as cleak/u5-spec#243.

So a scenario that has to reach a resident must seed the party **outside**
and walk in through the door, the way `shop-arms-buy` does. Note also that
`prng.md` §3 makes NPC wander non-reproducible - "ordinary gameplay events
re-seed from the host clock" - so a long scripted walk to a *wandering*
resident is a coin flip: prefer a resident that stands at a post, and use
`talk_gate_probe --talk <dialog-id>` to ask what this engine answers without
a paired run at all.

| Scenario | Coverage |
|---|---|
| `chargen-journey-basics` | menu, name/gender prompts, gypsy pages, Journey Onward, first steps, Z-stats |
| `hut-commands` | Look, Z-stats for two members, X-it refusal, Search, Yell cancel, Quit/save |
| `hut-exit-overworld` | walking to the hut door (superseded by `hut-door-overworld`) |
| `hut-door-overworld` | open the hut door, step onto the overworld, Look, walk, save |
| `hut-to-ararat` | exploratory walk from the hut toward Ararat and an Enter attempt |
| `hut-audio` | PC-speaker blocked-step cue and a silent pass, captured per side |
| `doom-endgame-audio` | the absorption beat's speaker output, per side, with silence controls |
| `combat-refusal-audio` | the arena's two-tone refusal pair on `L`, and its published silence on `D` |
| `stonegate-trapdoor-audio` | the 750-tone descent, the longest sound in the game, per side |
| `word-of-power-audio` | the shared full-viewport flash, whose spacing the spec cannot derive |
| `overworld-night-walk` | that the overworld is *alive* while the party walks: encounters spawn and creatures act |
| `drowning-audio` | the stock side alone, sailed until something sinks the ship, for a cue that cannot be scheduled |
| `dungeon-exit-klimb` | a Z transition: climbing out of a dungeon, and the overworld Look that follows |
| `overworld-camp` | H-Hole up carried through to a result, which no scenario had done |
| `magic-mix-and-cast` | the mixer carried through to a completed mix, which `magic-mix` cancels before |
| `hut-resident-commands` | the resident commands no other scenario sends: `I`, `V`, a plain digit (Set Active Player), `P`, `K`, `E`, `B`, `F` and `A` |
| `hut-control-bindings` | `commands.md` §9's five Control chords: the typeahead toggle, the moral-standing readout, the sound toggle, the version banner and the Exit-to-DOS prompt |
| `hut-ignite-torches` | the shipped four torches lit and a fifth attempt, which measures `lighting.md` §8's unquoted no-torch refusal |
| `hut-ready-picker` | the R-Ready equipment picker with items readied, which measures `inventory.md` §4.5's "runic glyph for a readied one" |
| `hut-use-items` | the three items the shipped party carries, which measure the `Item: ` echo, the potion's `On who: ` prompt and their result lines |
| `hut-ready-refusals` | R-Ready against `inventory.md` §2.1's strength gate, which the engine had implemented as an uncalled helper |
| `ship-commands` | X-it aboard a frigate, which measures the completed `X-it ship!` echo and the silence of the skiff launch |
| `ship-repair` | three hole-ups aboard a frigate, which measure the sea `repair...` branch and its `1..3` hull roll |
| `combat-rounds` | a few rounds inside a dungeon-room arena; it caught `VICTORY!` firing in an arena that never had a foe (`cleak/u5-spec#227`) |
| `town-wishing-well` | Paws' wishing well end to end - the coin prompt, the wish prompt and the grant - none of whose lines the spec publishes |
| `town-wishing-well-refusals` | the same well's declined coin and unaccepted wish |
| `shop-inn-after-entry` | the second shop family the suite reaches - Britain's innkeeper - and the town alarm that stops the engine getting there (`u5-engine#16`) |
| `hut-talk` | Talk with the hut's resident: greeting, name, job, bye |
| `hut-prompts` | New Order, Ready, Use, Cast, Mix, Yell, Search, Look, X-it, Get, Enter, Hole up, Ignite, Klimb prompt and result literals |
| `town-britain-seeded` | Britain from a seeded save: entry, walking, Look at an NPC cell |
| `town-fountain` | a town fountain's drink prompt and its cancel |
| `town-look-npc-cell` | Look at a cell an NPC stands on |
| `town-night-schedule` | Britain at 02:00: night NPC placement and lighting |
| `town-talk-after-entry` | Talk in a town entered through its door, which is what loads the dialogue table |
| `town-talk-nonspeaker` | Talk aimed at a non-speaking participant |
| `town-talk-second-npc` | Talk with an *ordinary* resident - a `.TLK` blob id, not a shop trigger or the reserved guard index |
| `town-talk-high-dialog-id` | Talk with a roster slot carrying a high/special dialogue id |
| `castle-talk`, `castle-talk-after-entry` | the same two shapes inside a castle |
| `dwelling-talk-after-entry`, `dwelling-talk-after-restore` | Talk in a dwelling entered by door, and the same save restored inside it |
| `minoc-entry-walk`, `minoc-tribute` | Minoc entry and the tribute exchange |
| `blackthorn-palace-password` | the palace password audience |
| `combat-town-attack`, `combat-dungeon-room` | combat entry from a town attack and from a dungeon room |
| `combat-town-attack-after-entry` | the same Attack, reached by walking in at an hour when the resident is at its shop |
| `dungeon-view` | the first-person corridor |
| `underworld-walk` | the Underworld plane's terrain, a step and a Look |
| `shrine-enter` | `E` on a shrine marker: the echo, the two narration lines and the virtue question |
| `shrine-flow` | the whole shrine prompt: the virtue answer, the `Mantra:` row and the re-ask a correct mantra leaves |
| `shrine-mantra-close` | the mantra prompt's other answers: a wrong mantra, an ignored Escape, and the empty Return that closes it |
| `slow-progress` | eight steps north from the shrine: `Slow progress!` on brush, `Very slow!` on trees and foothills |
| `slow-terrain` | the same two lines reached from a second direction, over both foothill tile ids |
| `swamp-step` | steps into swamp: `Slow progress!` there too, and `Blocked!` at the water beyond |
| `swamp-walk` | sixteen steps around the swamp, reading the party's status letter from the panel after each |
| `swamp-long` | forty more swamp steps: the party stays Good, so nothing poisons it |
| `town-death-vision` | the crystal-sphere Look and its two roll outcomes |
| `use-potions` | the potion result lines, one colour at a time |
| `use-scrolls` | the two scrolls that ask for an argument |
| `use-specials` | the special-item rows: the carpet, the regalia, the sceptre and a shard |
| `cast-results` | the C-Cast spell-name prompt and four spell results |
| `combat-commands` | the arena's Get, Search, Klimb and X-it answers |
| `combat-ready-armour` | Lor Sanct in the arena, and R-Ready on a body-armour row |
| `prompt-cancels` | what the Use and New Order pickers print when cancelled |
| `use-skull-key` | the skull key's `Item: Skull Key` row and `Direction-` prompt |
| `use-single-item` | one special item alone in the picker; re-seed between runs to measure another |
| `ready-slots` | R-Ready's occupied-slot and two-handed refusals |
| `ready-two-handed` | readying an off-hand item with a two-hander already wielded |
| `use-moonstone` | the moonstone bury refusal, and the carpet aboard ship |
| `stray-keys` | a function key, an unmapped letter, and Space at a scroll's `Direction-` |
| `night-sextant` | the sextant's reading from a ship at night |
| `use-target-cancel` | Escape and Space at a potion's `On who: ` prompt |
| `dungeon-search` | dungeon Search: the `Search...` echo, the shared `Dir-` relative-direction prompt, and the hidden-door reveal |
| `dungeon-look` | dungeon Look through the same helper, including the Space cancel that writes `Dir-Pass` |
| `magic-mix` | the M-Mix reagent list and its prompts |
| `shop-arms` | the arms shop's browser and its prompts |
| `shop-arms-after-entry` | the arms shop reached by walking in through the door at 10:00, when the resident is at the shop |
| `shop-arms-buy` | the same walk-in, driven through the Buy menu, an item pick and the refusals |
| `shop-arms-menus` | both arms menus with waits long enough for the stock to finish drawing: the Buy listing, an item quote, the Sell prompt and the closing flourish |
| `shop-arms-buy-confirm` | the Buy confirmation: quote, `Y`, `Sold!`, the post-item prompt and the redrawn listing, then a second quote declined with `N` |
| `shop-arms-sell-flow` | a whole sale through the browser: cursor move, offer, decline, second offer, `"Done!" says Gwenneth`, and the quoted goodbye on the way out |
| `paws-sleeper` | Paws' tavern keeper is still on a bed at 10:00, so Talk answers the sleeping gate: `"Zzzzzz..."`, quotes included. The route opens a door on the way |
| `shop-arms-sell-ammunition` | Return on an ammunition row: `"We don't deal in used ammunition!" growls Gwenneth`, and the visit ends |
| `shop-arms-sell-zero-price` | Return on a zero-price row: `"That, I cannot buy from thee." says Gwenneth`, and the browser continues |
| `shop-arms-sell-keys` | which key selects a row in the Sell browser: digits, letters and the arrows do nothing, **Return** takes the highlighted row and prints the offer with `Deal?"` |
| `shop-arms-navigation` | what Space does at the Buy listing: it ends the visit on `"Be off with ye, then..." says Gwenneth`, so it is not a way back to the greeting |
| `shop-arms-sell` | the Sell side of the same shop. **Retry until the `talk` shot shows the greeting**: the walk is thirty steps and the weaponsmith wanders, so a run that ends on `Nobody's here!` proves nothing (three of four attempts on 2026-09-07) |
| `doom-final-room` | Doom's final room: the fall onto the room trigger, the arena, the absorption, and Lord British's dialogue. **Compared 2026-09-07**: both sides print `Entering room`, the arena banner, `Avatar is absorbed!`, `Lord British says: "Well met, Avatar!"`, the box question, and the seated reply, differing only by one capture's timing |
| `doom-final-room-refusal` | the same room answering `No` to the first box question. **Compared 2026-09-07**: both sides reach `…secret passage in my chamber!"`, `"Didst thou bring it?"` and `You reply:` |

## Scenarios that need a seeded save

Most scenarios create a character and play from the hut. The ones below start
from a save the harness must be given with `--seed-save <dir>`, because the
state they measure is not reachable from a fresh character in a reasonable
number of keystrokes:

| Scenario | The seed must hold |
|---|---|
| `doom-final-room`, `doom-final-room-refusal`, `doom-endgame-audio` | the party standing on the fall trap on Doom's level seven, directly above the final room's trigger, with a torch to hand |
| `combat-dungeon-room`, `combat-refusal-audio`, `combat-commands` | the party one step from a dungeon room trigger, with a torch to hand |
| `combat-rounds` | the same |
| `stonegate-trapdoor-audio` | the party inside Stonegate, standing in the trapdoor ring's centre cell |
| `word-of-power-audio` | the party standing on the Britannia overworld |
| `overworld-night-walk` | the party on the Britannia overworld at 02:00, when the encounter threshold can fire |
| `drowning-audio` | the party aboard a frigate with no skiffs, on deep water at night |
| `ship-commands` | the party aboard a frigate on deep water |
| `ship-repair` | the same |
| `dungeon-exit-klimb` | the party on Deceit's entrance level, with a torch to hand |
| `dungeon-search`, `dungeon-look`, `dungeon-view` | the party on Deceit's entrance level facing a wall-class cell, with a torch to hand |
| `overworld-camp`, `magic-mix-and-cast` | the party standing on the Britannia overworld |
| `town-night-schedule` | Britain at 02:00 |
| `underworld-walk` | the party standing on the Underworld plane (`--scene UNDERWORLD --at 100,100`) |
| `shrine-enter` | the party standing on a published shrine coordinate (`--scene BRITANNIA --at 233,66` for Honesty) |
| `shrine-flow` | the same seed as `shrine-enter` |
| `shrine-mantra-close` | the same seed as `shrine-enter` |
| `slow-progress` | the same seed as `shrine-enter` |
| `slow-terrain` | the same seed as `shrine-enter` |
| `swamp-step` | the party on grass at `--scene BRITANNIA --at 64,14`, south of the swamp at (64,12) |
| `swamp-walk` | the same seed as `swamp-step` |
| `swamp-long` | the same seed as `swamp-step` |
| `cast-results` | a caster with charges, reagents and mana, built with the `seed_inventory` example (`potions=3 spells=9 reagents=20 mana=99`) |
| `town-death-vision` | the party one cell east of a crystal-sphere tile. An engine-written town seed is safe here: the sphere is a Look target |
| `use-potions`, `use-scrolls`, `use-specials`, `ready-slots` | a party carrying potions and scrolls, built with the `seed_inventory` example (`cargo run --release --example seed_inventory -- <profile> potions=3 scrolls=3 specials=1 status1=D`, or `equipment=2 strength=99` for `ready-slots`, plus `equip2=33` for `ready-two-handed`). The shipped starting party carries three items, which is not enough to reach these families |
| `town-britain-seeded`, `town-fountain`, `town-look-npc-cell` | the party inside Britain, beside the fountain for the fountain scenario |
| `town-wishing-well` | the party standing south of the wishing well in Paws (scene 22). An engine-written town seed is safe here: the well is a Look target, and only Talk needs the save's NPC band |
| `town-wishing-well-refusals` | the same |
| `minoc-tribute`, `blackthorn-palace-password` | the party at the location named, with the quest state the exchange needs |
| `town-talk-after-entry`, `town-talk-second-npc` | the party outside Britain on the overworld, so the scenario can walk in through the door |
| `shop-arms-after-entry`, `combat-town-attack-after-entry` | the same, seeded at 10:00 rather than 12:00 |
| `shop-inn-after-entry` | the same 10:00 seed as `shop-arms-after-entry` |
| `shop-arms-menus`, `shop-arms-buy-confirm`, `shop-arms-sell` | the same 10:00 walk-in seed, plus `seed_inventory <profile> equipment=3` so the Sell browser has rows. Press the shop's keys **only after the previous line has finished drawing** - `shops.md` §8.A flushes type-ahead, so a key sent early is discarded, which is what made three runs look like a wandering resident |

## Pixel-exact comparison with `qa/tools/frame_diff.py`

The harness captures the Bevy window, which is aspect-corrected and scaled,
so its frames can only be compared by eye - and a montage crop taken at the
wrong fraction can invent a difference that is not there. For any state a
`--play-script` can reproduce from the same seed, the engine can write its
**native** 320x200 composed screen instead:

```
u5-engine --play --from-save --play-script 'i;\x1b[C' --save-screen /tmp/frame.png <profile>
python3 qa/tools/frame_diff.py /tmp/frame.png <artifact>/dosbox-<beat>.png
```

The differ reports the differing-pixel count per published screen region -
the gameplay viewport, the panel, the message window - so a real divergence
is separated from the two that are expected:

- the **viewport** in a town or on the overworld carries NPC positions and
  the wind banner, both host-clock seeded on each side (see above), so a
  few hundred differing pixels there are not a defect;
- the **message window** carries the cursor's blink phase.

Measured with it so far: `dungeon-view`'s first-person corridor is
pixel-identical (viewport 0, panel 0), and `zstats-pages`' panel is
pixel-identical on every page.

**A seed equal to the shipped save is refused.** `game-dev-u5-paired` now
compares the `--seed-save` directory's `SAVED.GAM` against the freshly prepared
profile's own copy and refuses when they match: the shipped file has no active
Avatar, so both sides would sit on the title menu and the run would agree with
itself while measuring nothing. Two scenarios had been "seeded" from a paired
profile directory that was never written to, and passed that way -
`town-britain-seeded` and `town-talk-second-npc`.

**A town seed the stock game did not write breaks Talk on the stock side.**
`formats/saved-gam.md` section 12 makes `0x07B4..0x105F` durable state - it
carries a town-family location's whole live cast, including each NPC's dialog
index - and `conversation.md` section 2 spells out the consequence: "A save
written by another program that leaves that region zero makes every NPC in the
location answer `No response!` until the location is re-entered from outside -
that is a property of the save, not of the game." A scenario seeded that way
will show the stock game refusing every Talk while the engine, which pairs
restored records against the `.NPC` roster, answers normally. That difference
is the seed's. Seeds for Talk and shop scenarios must be saved by the stock
game inside the location, or the scenario must walk in through the door. The
band's internal layout is unpublished, so the engine cannot write it either -
cleak/u5-spec#217 asks for it; until it is answered, walking in is the only
route that gives both sides a live cast.

**Reading the rows.** The DOSBox side is captured at 640x400, an exact 2:1
downscale of the 320x200 screen, so its message window can be decoded glyph by
glyph against `IBM.CH` and compared as text. The engine side cannot be, at any
window size: the Bevy shell presents its 320x200 frame at the 4:3 display
aspect (`DISPLAY_PIXEL_ASPECT`, 1.20), stretching the 200 rows and letterboxing
the remainder, which is the CRT geometry the original had and not a bug to fix.
Its capture is for side-by-side comparison and for coarse row-occupancy checks.
For text-level comparison render the engine's side natively with
`u5-engine --from-save --play-script ... --save-screen <PNG>`, which goes
through the same message-window layout.

**A missing seed used to pass silently.** Without one the profile's shipped
`SAVED.GAM` has no active Avatar, both sides sit on the title menu answering
`No active game`, every `shot` captures that menu, and the run still agreed
with itself. Scenarios that need a seed now say so in a machine-readable
header,

```
# requires-seed: the party on the fall trap on Doom's level seven, ...
```

which `game-dev-u5-paired` refuses to run without `--seed-save`, and which
`crates/u5-tui/tests/paired_scenarios.rs` requires of every scenario that does
not create its own character. The table above stays as prose for the reader;
the header is what the tools check.

**The wind banner is not a comparable cell.** The bottom chrome ribbon's
`<North Winds>` banner drifts on its own: `systems/weather.md` Section 2.2
gives the idle world tick a one-in-sixty-four roll that re-selects the wind,
silently and without consuming a turn, so "two runs of the original that load
the same save and press Q shortly afterwards can legitimately record different
wind values". Every `wait` step in a scenario is idle ticks, and the two sides
do not share a random stream, so the banner can differ at any beat and re-agree
at the next one - `town-talk-after-entry` shows East/North, South/South,
South/North, East/East, East/North across its five shots with nothing wrong on
either side. Never read that row as a divergence, and never let a whole-frame
pixel diff include it.

**An NPC is not guaranteed to still be there.** Both sides seed the game's
generator from the host clock at scene load, which is what the original does,
and the per-turn wander gate of `npc-schedules.md` Section 9.1 is a coin flip
per NPC per turn. A route planned from the seed therefore describes where a
resident *was*, not where it will be after a twenty-five step walk, and two
runs of the same scenario on the same side legitimately end with the resident
in different cells - one run of `shop-arms-after-entry` reached `Thou dost see
a merchant`, another reached `Thou dost see cobble`. Two consequences: aim
scenarios with `talk_gate_probe --chase`, which re-plans after every step, and
expect to run a positional scenario more than once before it compares
anything. A single run that finds empty floor has measured nothing; it has not
found a defect.

**The shop opens itself.** `npc-schedules.md` Section 9's adjacency event
dispatches the shop for a resident carrying a shop-trigger dialogue byte, with
no Talk key involved - the stock side of `shop-arms-after-entry` already shows
the arms greeting at its `look` beat. A scenario that means to test the `T`
command against a shopkeeper has to account for the shop having opened on its
own first. See cleak/u5-spec#215.

**The native oracle cannot draw the combat cursor box.** `combat.md` Section 7
has the shared tile pass toggle a blink flag each pass and, on the lit pass,
draw the two-pixel white frame around the active player's arena cell. The
engine draws it - the Bevy capture of `combat-dungeon-room` carries it in the
same cell and the same shape as the stock game - but
`u5-engine --save-screen` renders once without pumping the visual idle tick
that `apply_combat_cursor_blink_tick` hangs off, so the box is never lit
there. A native pixel diff of any arena frame therefore reports one 16x16
cell's worth of difference (221 pixels in that scenario) that is not a defect.
Compare arena frames from the paired capture, or exclude the active player's
cell.

**Record a silence control beside every audio capture.** A capture that
carries short broadband bursts is ambiguous on its own - DOSBox, PipeWire and
the recorder can all click - and the cheap way to settle it is a few seconds of
the same scene with no keys sent. `doom-endgame-audio` records one per side
before the beat it is aimed at, and that control is the whole reason its
eleven-burst finding could be filed as game output (cleak/u5-spec#218) rather
than guessed at. Without it I had already dismissed a similar train in
`hut-audio` as an artefact, which now looks wrong.

**Reaching the drowning cue, since it took a while to find.** `audio.md §8.9`'s
long descent needs a frigate destroyed with no skiff aboard and no carpet in
stock. The precondition is reachable without editing a save: `X;B;X;B` aboard
the frigate leaves the party aboard with `skiffs=0`, because each `X`
disembarks into a skiff and each `B` re-boards and leaves that skiff behind.
Sailing that ship at night then takes ranged impacts from the creatures the
per-turn walker sets on the party, and an impact roll that meets the hull sinks
it. The whole chain runs: ship sunk, drowning, party death, rescue to
`CASTLE:0` at (10, 10).

It is stochastic, which is what makes it awkward to capture: the descent fires
in about one sail of ten at 200 turns, and three of six at 1200. Grinding the
hull down first does not help - over short passes the ship either takes no
impact at all or is destroyed outright, so there is no reliable low-hull seed.

**A cue that cannot be scheduled can still be compared.** The long descent of
`audio.md §8.9` fires only when an encounter sinks the ship, about one sail in
ten, so waiting for both sides to hit one inside the same paired run is
impractical. `drowning-audio` therefore drives the DOSBox side alone and
records it, and the engine's own cue is rendered from the same program the
shell plays through:

```
cargo run --release --example render_cue -- long-descent /tmp/ours.wav
```

The two waveforms are then compared directly. That is a weaker claim than a
simultaneous capture - it does not prove the two fired at the same point in the
same situation - so say which one you have when you report a result.
