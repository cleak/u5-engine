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
| `magic-mix` | the M-Mix reagent list and its prompts |
| `shop-arms` | the arms shop's browser and its prompts |
| `shop-arms-after-entry` | the arms shop reached by walking in through the door at 10:00, when the resident is at the shop |
| `doom-final-room` | Doom's final room: the fall onto the room trigger, the arena, the absorption, and Lord British's dialogue |
| `doom-final-room-refusal` | the same room answering `No` to the first box question |

## Scenarios that need a seeded save

Most scenarios create a character and play from the hut. The ones below start
from a save the harness must be given with `--seed-save <dir>`, because the
state they measure is not reachable from a fresh character in a reasonable
number of keystrokes:

| Scenario | The seed must hold |
|---|---|
| `doom-final-room`, `doom-final-room-refusal`, `doom-endgame-audio` | the party standing on the fall trap on Doom's level seven, directly above the final room's trigger, with a torch to hand |
| `combat-dungeon-room`, `combat-refusal-audio` | the party one step from a dungeon room trigger, with a torch to hand |
| `stonegate-trapdoor-audio` | the party inside Stonegate, standing in the trapdoor ring's centre cell |
| `word-of-power-audio` | the party standing on the Britannia overworld |
| `overworld-night-walk` | the party on the Britannia overworld at 02:00, when the encounter threshold can fire |
| `drowning-audio` | the party aboard a frigate with no skiffs, on deep water at night |
| `dungeon-exit-klimb` | the party on Deceit's entrance level, with a torch to hand |
| `town-night-schedule` | Britain at 02:00 |
| `town-britain-seeded`, `town-fountain`, `town-look-npc-cell` | the party inside Britain, beside the fountain for the fountain scenario |
| `minoc-tribute`, `blackthorn-palace-password` | the party at the location named, with the quest state the exchange needs |
| `town-talk-after-entry`, `town-talk-second-npc` | the party outside Britain on the overworld, so the scenario can walk in through the door |
| `shop-arms-after-entry`, `combat-town-attack-after-entry` | the same, seeded at 10:00 rather than 12:00 |

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
