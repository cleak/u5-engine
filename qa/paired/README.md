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
| `hut-talk` | Talk with the hut's resident: greeting, name, job, bye |
| `hut-prompts` | New Order, Ready, Use, Cast, Mix, Yell, Search, Look, X-it, Get, Enter, Hole up, Ignite, Klimb prompt and result literals |
| `town-britain-seeded` | Britain from a seeded save: entry, walking, Look at an NPC cell |
| `town-fountain` | a town fountain's drink prompt and its cancel |
| `town-look-npc-cell` | Look at a cell an NPC stands on |
| `town-night-schedule` | Britain at 02:00: night NPC placement and lighting |
| `town-talk-after-entry` | Talk in a town entered through its door, which is what loads the dialogue table |
| `town-talk-nonspeaker` | Talk aimed at a non-speaking participant |
| `town-talk-high-dialog-id` | Talk with a roster slot carrying a high/special dialogue id |
| `castle-talk`, `castle-talk-after-entry` | the same two shapes inside a castle |
| `dwelling-talk-after-entry`, `dwelling-talk-after-restore` | Talk in a dwelling entered by door, and the same save restored inside it |
| `minoc-entry-walk`, `minoc-tribute` | Minoc entry and the tribute exchange |
| `blackthorn-palace-password` | the palace password audience |
| `combat-town-attack`, `combat-dungeon-room` | combat entry from a town attack and from a dungeon room |
| `dungeon-view` | the first-person corridor |
| `magic-mix` | the M-Mix reagent list and its prompts |
| `shop-arms` | the arms shop's browser and its prompts |
| `doom-final-room` | Doom's final room: the fall onto the room trigger, the arena, the absorption, and Lord British's dialogue |
| `doom-final-room-refusal` | the same room answering `No` to the first box question |

## Scenarios that need a seeded save

Most scenarios create a character and play from the hut. The ones below start
from a save the harness must be given with `--seed-save <dir>`, because the
state they measure is not reachable from a fresh character in a reasonable
number of keystrokes:

| Scenario | The seed must hold |
|---|---|
| `doom-final-room`, `doom-final-room-refusal` | the party standing on the fall trap on Doom's level seven, directly above the final room's trigger, with a torch to hand |
| `town-night-schedule` | Britain at 02:00 |
| `town-britain-seeded`, `town-fountain`, `town-look-npc-cell` | the party inside Britain, beside the fountain for the fountain scenario |
| `minoc-tribute`, `blackthorn-palace-password` | the party at the location named, with the quest state the exchange needs |

**A missing seed does not fail the run.** Without one the profile's shipped
`SAVED.GAM` has no active Avatar, both sides sit on the title menu answering
`No active game`, every `shot` captures that menu, and the harness still
reports `"result": "pass"` - the two sides agree, after all. Check the first
capture of any seeded scenario before reading its later ones.
