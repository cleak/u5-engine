# u5-engine

Verification harness and in-progress clean-room engine for the Ultima V specs.

This is not a full replacement engine yet, but the executable has grown beyond
the original Lord British castle/throne-room slice. It reads the user's local
Ultima V data at runtime, verifies public specs against real files, and exposes
terminal plus Bevy play loops backed by the same runtime state:

- town-mode scene partitioning;
- per-class `*.DAT`, `*.NPC`, and `*.TLK` joins;
- location floor loading and render-class hashing;
- marker, door, and stair detection;
- schedule waypoint sampling;
- conversation-name lookup;
- public LZW graphics-envelope decoding for tile atlases, image directories,
  sprite/mask sheets, standalone `.BIT` bitmaps, and proportional/fixed font
  rasterization;
- atlas-backed top-down viewport rasterization for town/world scenes plus a
  clean first-person dungeon raster; and
- a small class-derived movement/pathfinding smoke test.

The repository does not include game assets. Run it with a local Ultima V
install path:

```powershell
cargo run -- C:\Games\U5-Clean
```

The run writes an aggregate report to
`reports/lb-throne-room-slice.txt`. The report intentionally avoids raw map
dumps, dialogue transcripts, and binary offsets.

## Documentation map

The README keeps quick-start and compatibility notes. Focused handoff docs live
under `docs/`:

- `docs/architecture.md` - crate layout, runtime boundaries, and clean-room
  data rules.
- `docs/commands.md` - current A-Z command routing by mode, with representative
  test evidence.
- `docs/sidecars.md` - clean-room TSV/binary sidecar files accepted at runtime.
- `docs/status-matrix.md` - implementation status matrix and verification
  commands for the current first-playable engine.

There is also an early first-playable terminal slice that drops the player into
the Lord British castle binding verified by the report:

```powershell
cargo run -- --play C:\Games\U5-Clean
```

## Bevy visual mode (first slice)

A minimal Bevy frontend renders the same `PlayState` to a real window instead
of the terminal. It is feature-gated so the default build keeps the lean
verification dependency surface. It currently covers top-down overworld/town
scenes plus the clean first-person dungeon raster:

```powershell
cargo run --features visual -- --visual --scene BRITANNIA C:\Games\U5-Clean
cargo run --features visual -- --visual --scene CASTLE:0 --floor 0 C:\Games\U5-Clean
cargo run --features visual -- --intro --visual C:\Games\U5-Clean
```

The window draws a single CPU-generated 11x11 tile viewport (an `EGA` or `CGA`
indexed framebuffer converted to RGBA) into one Bevy `Image` and displays it
through one nearest-neighbor sprite. Gameplay still lives in `PlayState`: the
input system maps keyboard events into the same handlers used by the terminal
harness, so movement, blocking, doors, and supported area transitions work out
of the box. Dungeon scenes render a light-gated first-person corridor panel;
combat scenes render the tactical arena through the same atlas-backed viewport,
while shops, conversations, and other line-oriented interactions remain modal
runtime flows rather than bespoke Bevy UI. Modal prompts such as
conversation keywords, Blackthorn answers, and sage topics collect typed text
in the status panel, support Backspace, and submit on Enter.
The runtime also exposes a spec-backed fixed-cell text-window core with four
independent descriptors, preserved per-window cursors, style control bytes,
clear/scroll behavior, wrapped output, numeric output, and typed-input erasure;
TUI and Bevy status/modal summaries now share a 40x25 text-window surface for
message, prompt echo, and the fixed sixteen-column stats panel. The Bevy
gameplay panel renders that surface through the runtime-loaded `IBM.CH`
fixed-cell font into a nearest-sampled texture instead of generic UI text.

`--intro --visual` opens a Bevy intro/menu shell backed by the same runtime menu
dispatcher used by terminal intro mode. Journey Onward loads `SAVED.GAM` and
`SAVED.OOL` then launches the Bevy gameplay loop. The visual shell pages the
story text, acknowledgement boundary, and a dry-run-rendered Return-to-View
preview strip in intro-local panels. Create New Character and Ultima IV Transfer
also run in visual intro panels, then write `SAVED.GAM` and `SAVED.OOL` through
the shared runtime save producers.

Input map (visual mode):

| Key                                                 | Action            |
|-----------------------------------------------------|-------------------|
| `W`/`A`/`S`/`D`, arrow keys, numpad 8/4/2/6         | Cardinal movement |
| Numpad 7/9/1/3                                      | Diagonal movement |
| `E`                                                 | Enter             |
| `O`                                                 | Open              |
| `K`                                                 | Klimb             |
| `,` / `.`                                           | `<` / `>` floor   |
| `Space`                                             | Pass              |
| `B`, `C`, `F`-`J`, `L`-`R`, `T`-`V`, `X`-`Z`        | Command letters   |
| `Shift+A` / `Shift+S`                               | Attack / Search   |
| `Ctrl+S`                                            | Toggle music      |
| number row `0`-`9`                                  | Modal selections  |
| text + `Backspace` + `Enter`                        | Text prompts      |
| `Q`                                                 | Save/exit prompt  |
| `Esc`                                               | Quit              |

`--scene`, `--floor`, `--debug-enter`, `--time`, `--wind`, `--transport`,
`--from-save`, and `--from-init` work the same way they do in terminal play
mode. The terminal harness is unchanged; building without `--features visual`
skips the Bevy dependency entirely.

Add `--raster-diagnostics` to play mode to exercise the atlas-backed top-down
renderer each prompt. It prints only viewport dimensions and a hash of palette
indices, not raw asset pixels. The diagnostic defaults to the EGA `.16` atlas;
add `--raster-depth cga` to exercise the low-colour `.4` atlas:

```powershell
cargo run -- --play --raster-diagnostics C:\Games\U5-Clean
cargo run -- --play --raster-diagnostics --raster-depth cga C:\Games\U5-Clean
```

Use `--save-frame <PATH>` for a headless PNG capture of the current 11x11
viewport. It loads the same `PlayState`, optionally replays a script first, and
then writes the atlas-backed top-down, combat, dungeon first-person, or fallback
text-panel frame:

```powershell
cargo run -- --save-frame screenshots\britannia.png --scene BRITANNIA C:\Games\U5-Clean
cargo run -- --save-frame screenshots\dungeon.png --scene DUNGEON:0 --play-script "idle:1" C:\Games\U5-Clean
```

`--save-frame-suite <DIR>` writes representative local-asset PNGs for
Britannia, a moved Britannia frame, Castle:0, a lit Dungeon:0 frame, a synthetic
combat viewport, and an endgame status panel, plus a sanitized manifest with
dimensions, frame kinds, positions, and hashes:

```powershell
cargo run -- --save-frame-suite target\frame-suite C:\Games\U5-Clean
```

For repeatable smoke checks, `--play-script` runs a semicolon-separated command
list through the same first-playable input handlers and then exits. Script mode
prints compact state summaries and optional raster hashes instead of rendered
map frames. Use `empty` or `pass` for an Enter/Space pass turn, and `idle:N`
for N no-turn visual ticks:

```powershell
cargo run -- --play-script "d;empty;idle:4;q" --raster-diagnostics C:\Games\U5-Clean
```

`--route-smoke` runs a bundled local-asset route suite covering default castle
play, Britannia movement, Z-stats modal routing, debug-enter town/dungeon
transitions, and dungeon exit refusal. It prints sanitized state lines and
raster hashes:

```powershell
cargo run -- --route-smoke C:\Games\U5-Clean
```

`--play-script` can be combined with `--scene` and `--debug-enter` for
transition plumbing while exact public overworld entrance coordinates remain
unpublished:

```powershell
cargo run -- --scene BRITANNIA --debug-enter CASTLE:0 --play-script "e;q" C:\Games\U5-Clean
```

Town and overworld movement use numpad-style directions, common terminal
arrow/Home/End/Page navigation sequences, or lowercase `wasd`/vi keys; uppercase
letter inputs route through the command layer so conflicting command letters
like `C`, `U`, and `Z` do not move the party.
Lowercase `x` also routes to the non-conflicting vehicle X-it command.
The terminal `buffer`/`typeahead` command toggles the public typeahead-buffer
flag. While it is on, a line containing only movement keys, spaces, and `.`
idle ticks replays as queued one-keystroke inputs; complex inline shortcuts such
as `TJOB` and `C1IL` stay single commands.
Common terminal F1-F10 escape sequences are recognized as the public function-key
remap family and ignored before command dispatch, without spending a turn or
running a dungeon idle tick.
Other unclassified multi-byte escape sequences follow the public unused-key
path and are also ignored before dispatch; a lone Escape remains a regular
control byte for prompt-specific cancellation paths as they are promoted.
In top-down scenes, bare `Q` opens the resident save-and-continue prompt;
`QY`/`QN` remain inline shortcuts. In dungeon mode, uppercase `Q` opens the
public mode-loop `Exit to DOS?` prompt (`QY` exits the terminal play loop,
`QN` cancels) and lowercase `q` remains a harness-only immediate quit.
`Z` prints a first-playable text status summary covering active area, position,
time, transport, wind, typeahead status, light, inventory, mixed spells, and
runtime party order without spending a turn.
`K` climbs unambiguous town stairs and walking onto those stair tiles also
triggers the floor change; clean `town_stairs.tsv` rows can pin one-way versus
two-way stair direction where the public subtype table is still open, and
two-way town stairs prompt and let `<`/`>` choose the floor direction from the
stair cell. Outdoor `K` follows the spec Grapple/on-foot gates and exposes
semantic `--grapple` plus legacy `--climbing-gear` startup hooks for
first-playable class-derived mountain-family climbs. Fall checks run against
living saved-party members and use the saved roster Dexterity byte at `0x0D`;
the Grapple gate reads the legacy magic-powder/climbing-gear byte at `0x0209`.
Outdoor `K` also respects clean lava sidecar blockers and fires clean world
plane-transition rows after a successful climb. Town
and overworld `L`ook now resolves the facing tile or active
object through the runtime `LOOK2.DAT` table when the local game data is
available, falling back to harness tile classes in parser-only tests. Public
special-look context is applied for clock tiles using the 12-hour A.M./P.M.
time, and world dungeon-mouth tiles append the clean `world_locations.tsv`
dungeon name when that metadata is available.
Town and overworld `G`et now prompts for a cardinal direction, while inline
forms such as `G6` still route in one command. It can consume clean-room
authored tile-consumable rows from `town_get_tiles.tsv` and
`world_get_tiles.tsv`, rewriting the visit-local map and applying optional
authored counter grants while the original tile-to-item mapping remains out of
scope. Explicit `object_pickups.tsv` rows can also consume visible active
objects and update the current food, gold, key, gem, or torch counter before
generic active-object blocking.
Town-family `T`alk now prompts for a cardinal direction before resolving the
scheduled NPC's dialogue id, including one-cell talk-through over table/counter
furniture, through the matching runtime `.TLK` envelope and reports the clean
first-playable conversation header. Inline `T<keyword>` input, such as `TJOB`,
runs a one-shot keyword lookup against the decoded `.TLK` fields using the
public space-boundary match rule and applies supported TLK byte-runner side
effects. Bare Talk opens the interactive conversation keyword loop when raw
`.TLK` streams are available, and Talk-triggered shopkeepers route into the
modal shop sessions, including horse-trader purchases that place a nearby
boardable horse object. Raw `.TLK` dictionary tokens require the clean 128-row
`common_words.tsv` sidecar beside the game data; tokenized conversations expand
through that shared dictionary, and tokenized raw conversation data without it
is rejected instead of surfacing placeholder text. TLK `0x85` gold-payment
prompts decode the three public digit bytes, yield for yes/no input, refuse
unaffordable payments, and debit accepted affordable payments. The toll-style
moral-standing milestone described by the public karma spec is intentionally
not wired into production conversation cleanup yet: `cleak/u5-spec#27` still
needs to publish the toll-progress counter, milestone predicate,
reset/increment rules, and qualifying payment contexts. Overworld and dungeon
Talk return the stock no-response path without spending a turn.
Dungeon movement and the normal lit render are facing-relative: `W`/`S` step
forward/back, `A`/`D` turn left/right, blocked cardinal movement reports the
public `Blocked!` refusal, `K` climbs one-way ladders or prompts on two-way
ladders where `<`/`>` choose up/down, non-ladders return the public
`Not climbable!` refusal, and `L` looks forward. The
terminal renderer uses a first-playable text proxy for the public first-person
dungeon wireframe: it reports the current cell, up to
four forward bands, side cells at each band, and hides bands behind the first
front wall or boundary. `O` opens an underfoot dungeon chest in the visit-local
runtime image, and `G` gets an underfoot dungeon chest through the same
visit-local chest marker path, with the full content/trap generator still out
of scope. `S` searches the facing dungeon cell: clean sidecar rows can reveal
secret doors, public chest cells enter the same visit-local chest path, and
exact public bomb-trap bytes are marked as fired without changing level; other
public dungeon cell classes narrate without triggering movement tile effects.
Consumed non-movement dungeon commands and pass/empty waits already standing on
clean `dungeon_teleports.tsv` or `dungeon_exit_tiles.tsv` cells, public pit,
bomb-trap, or field bytes run the same post-action underfoot tile-effect pass,
without spending a second turn.
Dungeon exploration keeps the top-down active-object table out of its
turn and idle visual animators because the first-person dungeon renderer owns
its own position state; shared static animation still ticks. The dungeon raster
projects same-level active dungeon objects into the visible first-person depth
bands, while stale objects from other dungeon levels are ignored. Dungeon render
and `L`ook obey the public personal-light gate; optional
`dungeon_teleports.tsv` rows model scripted level-to-level cells, and optional
`dungeon_exit_tiles.tsv` rows model immediate exit-dungeon cells while their
exact encoding remains open. Runtime
`0xA?` room-helper state fires before the next dungeon key just like room
triggers while keeping its low-nibble arena slot. Optional `dungeon_doors.tsv`
rows split heavy or revealed secret doors from room-trigger cells and provide
the open-cell rewrite used by `O` and `J`. Stepping into public
sleep/poison/fire/electric field cells now applies party status or deterministic
damage; generic `0x84..0x8F` energy-field contact has no status/damage effect,
and the secondary `0x9?` visual family remains descriptive only. Looking at
public fountain cells prompts for a drink; inline
responses like `lY`, `lN`, or `l2Y` apply the cure/heal/poison/bad-taste
subtypes to the selected party member without spending a turn. `T`alk reports
the stock no-response line and world/vehicle command
letters are routed as dungeon refusals before they can trigger overworld
handlers. `I`gnite consumes a torch and starts or extends the torch counter with
deterministic first-playable timing.
Dungeon command letters whose full systems are outside this slice no longer fall
through to vi diagonal movement fallbacks. Bare `C` opens a spell-name prompt
that accepts compact selector letters, ignores `J`/`O`, supports backspace and
Escape/empty cancellation, and dispatches through the same spell resource and
scene gates as inline `C1...` casts. Spells that need a follow-up direction,
party member, combat slot, or Gate Travel moon phase now prompt for that choice
before any spell charge or mana is spent. Bare `U` opens the Use picker, bare
`R` opens the Ready picker with carried-stock, ammunition, strength, occupied
slot, hand-occupancy, ring-vanish, and combat body-armour gates, bare `M` opens
the reagent mixer, bare `N` opens the New Order party-slot prompt, and bare `Y`
opens the free-text yell prompt.
Bare `J` opens the Jimmy party-member picker, and inline forms such as `J1`
still route in one command. The command uses the first-playable Jimmy/key guard
instead of the movement helper, with optional town lock and dungeon door
metadata able to unlock authored door cells. Numeric diagonals still refuse as
unsupported dungeon movement, and dungeon `Q` routes to the public mode-loop
`Exit to DOS?` prompt instead of the resident save writer.
Top-down uppercase `L` opens the Look direction prompt, while inline forms such
as `L6` and lowercase quick-look continue to route in one command without
turning the party or spending a turn.
Unhandled dungeon keys run the public sleep/idle polling path as a no-turn
`Zzzzzz...` visual tick instead of using the top-down generic unhandled-command
message.
`V`iew consumes a gem and opens a modal top-down map: a 32-by-32 town/world
class overlay, or a centered dungeon flood map that wraps the 8-by-8 level and
stops expansion at wall-like cells while exact dungeon glyph/floodability edge
cases remain out of scope.
In town and overworld mode, `B` boards a current or facing parked vehicle
active object, including magic carpets, and town horse boarding refuses occupied
horse cells with the public `Nay!` line. `X`/`x` exits the current vehicle,
searching nearby cells for a foot-walkable landing point, skipping clean
damaging-terrain sidecar cells that are blocked or hazardous on foot, active
moongate origins, clean world plane-transition/waterfall cells, and town
stair/exit/trap-door transition cells when metadata is available, and refusing
with the stock `Not here!` line when none exists. `Y` toggles ship sails.
Horse, ship, skiff, and magic-carpet boarding now accepts the public parked
object bytes and saves the documented transport marker byte for the active
state, while still accepting the first-playable visual tile ids used by older
debug hooks. Balloon support is currently a semantic debug transport only: it
follows wind direction over terrain, overflies clean damaging-terrain/waterfall
sidecar effects, and can X-it only when the current cell is not mountain or
wall-like; B-Board remains intentionally unpromoted for balloons.
Furled ships use manual water movement; hoisted sails use the harness wind
state, where calm/perpendicular wind stalls and same-axis wind advances on a
deterministic first-playable cadence. After a stalled sail attempt, Pass
reports and clears the short-lived wind-stall feedback; changing sail mode also
starts a fresh wind-control cadence and clears pending stall feedback. `C1IL`
and `C1LV` run the narrow Light and Great Light C-Cast hooks, setting the
shared spell-light counter to the public 100- or 255-unit duration after the
saved charge, mana, and level gates succeed. `C1AZ2`, `C1AN2`, `C1M2`,
`C1MV2`, and `C1CIM2` cast narrow Awaken, Cure, Heal, Great Heal, and
Resurrect hooks from party slot 1 targeting party member 2; status clears
follow the public sleep/poison/death semantics. Heal applies the public
`magic.md` §8 formula (random 0..60 halved, promoted to one), giving a 1..30
HP restore capped at the target's max HP. Great Heal restores accepted non-Dead
targets to maximum HP, and Resurrect rebuilds Dead targets at 1 HP with
public level, max-HP, mana, and moral-standing experience adjustment rules.
`C1IS`, `C1RT`, and `C1AI` cast the
shared active-effect wrappers for Protection, Quickness, and Negate Magic,
recording the public `P`/20, `Q`/30, or `N`/10 runtime tag and aging it on
consumed turns. Combat consumes those shared tags for Protection's equipped-stat
bonus, Quickness's player-dispatch skip gate, Mass Charm's AI target remap, and
Negate Magic's cast absorption. `C1IW` casts the
narrow overworld Locate hook, reporting the current plane, coordinate, facing,
wind, and time
after the saved charge/MP/level gates succeed. `C1IMX` casts the narrow Create
Food hook, adding 100 units to the save-backed food counter after the saved
charge/MP/level gates succeed and clamping at the shared 9999 party food cap.
The exact Create Food grant remains blocked on clean spec issue
`cleak/u5-spec#49`.
`C1AS` casts the narrow An Sanct
Open hook from party slot 1, safely consuming an underfoot dungeon chest through
the visit-local chest rewrite after the saved charge/MP/level gates succeed;
other currently unmodeled chest targets spend the cast and fail in place. `C1HR`
casts Rel Hur from party slot 1: it uses the saved pre-mixed spell charge, mana,
and level gates, takes an inline cardinal direction (`C1HR8/6/2/4`) or
`C1HR<space>` for the no-effect Pass branch, and routes the direction through
the public `weather.md` §3 mapping (N→W, E→E, S→S, W→N). `C1PU` and `C1DP` cast the
narrow dungeon Up/Down hooks from party slot 1, moving one dungeon level inside
public level bounds and failing in place at boundaries. The command-overlay
dungeon escape helper is tracked separately from the spell-dispatch hook.
`C1FGI6`, `C1GIN6`, `C1GIZ6`, and `C1GIS6` cast the
public dungeon field-placement hooks one cell east from party slot 1; replace
the trailing cardinal numpad digit with `8`, `6`, `2`, or `4` to choose the
target cell. The live dungeon image accepts only passage bytes `0x00` and
visit-marked `0x08`, preserving the marker bit in the placed field byte.
`C1AG6` casts Dispel Field from party slot 1, spending the saved charge,
MP, and level gates before clearing a public dungeon field target back to
passage while preserving the visit marker bit.
`C1IP6` casts a clean sidecar-backed Blink hook from party slot 1, using
`blink_targets.tsv` to choose a same-map destination for the current
scene/floor/source/direction and then applying the saved charge/MP/level gates
before teleporting to a legal foot landing cell. This keeps magic-lock bypasses
authored by clean metadata without inventing the unresolved default Blink range
or reconciling the current public scene-mask conflict. The default range/search
rule remains blocked on clean spec issue `cleak/u5-spec#48`.
`C1AEP` and `C1EIP` cast narrow indoor Magic Lock and
Unlock Magic hooks from party slot 1, rewriting facing magic-lock rows supplied
by the clean `town_locks.tsv` sidecar. `C1IQW` casts the narrow Peer hook in
dungeon, indoor, or overworld mode, spending spell resources for the same
first-playable modal map overlay as gem view without requiring or consuming a
gem.
`C1AWY` casts the narrow X-Ray hook in indoor or overworld mode, using the
same first-playable modal surface map overlay after the saved charge/MP/level
gates succeed.
`C1PRV2` casts the narrow Gate Travel hook from party slot 1 to saved Moonstone
phase 2 in dungeon, indoor, and overworld play states; it refuses shipboard
casting before spending resources, consumes the saved spell charge/MP/level
gates, and moves valid saved phase slots on foot. `C1AT` casts the narrow Time
Stop hook from party slot 1, starting a ten-turn runtime counter after the saved
charge/MP/level gates
succeed; while it is active, consumed turns keep clocks, light, doors, and
static animation moving but freeze scheduled NPCs and active-object
animation/wandering. Bare `M` opens shrine meditation when the party is on a
clean shrine row, otherwise it opens the reagent mixer; inline
`M<spell>/<reagent-mask>/<quantity>` still mixes saved reagent counters into
pre-mixed spell charges for terminal testing. For example,
`MIL/0x80/1` mixes In Lor from sulfurous ash, while `MAS/0x88/1` mixes An
Sanct/Open from sulfurous ash plus blood moss. Exact public recipe masks add
charges capped at 99; wrong masks consume the selected reagents without adding
charges, and zero, empty, or insufficient selections leave stock unchanged.
When the party stands on a clean `shrines.tsv` row, bare `M` prompts for the
shrine mantra, then prompts for an offering digit on completed shrines.
`M<mantra>` and `M<mantra>/<offering-digit>` remain inline shortcuts for the
same first-playable shrine meditation state machine against the public ordained
and Codex quest masks while the exact persistent standing-byte layout remains
open.
`UT`/`UI` use a torch, `UG`/`UV` use a gem, and `UK`/`UJ` use a key through the
shared first-playable Use command wrapper; key use reuses the same sidecar-backed
town lock and dungeon heavy-door path as `J`. `U1` through `U8` bury the
corresponding Moonstone phase at the current non-dungeon location when the
underfoot tile matches the public Moonstone bury set (`4..10`, `44`, or `45`).
Surface and town `S`earch can also surface a saved Moonstone phase as a
first-playable strange-rock pickup, and `G`et clears that pickup while
invalidating the associated Gate Travel slot.
Save export still preserves existing non-calm wind bytes rather than inventing
an unverified byte mapping.
`N<from><to>` swaps two one-based runtime party positions and consumes a turn
per `commands.md` §6, for example `N23` swaps the second and third travelling
members. Slot one is the leader and refuses to move; selecting the same nonzero
slot twice is accepted as a turn-consuming no-op. The swap affects later
party-position prompts such as `C2...` casts and runtime damage checks, and
save export writes the reordered active records back to the front roster slots.
In overworld ship mode, bare `F`/`f` opens the fire direction prompt and an
inline direction (for example `f4`) fires a first-playable broadside:
bow/stern shots refuse, legal broadsides trace up to three cells, and the first
target object hit has its active-object `+5` depletion byte reduced by a
`1..=20` roll. Targets remain active on low results and clear their active
object slot when the subtraction enters the high-bit range. In town mode,
`F`/`f` first runs the public door auto-close pass, then fires an adjacent
static cannon tile in the public `0xB4..=0xB7` four-facing family, with
`town_fire_sources.tsv` still available as a clean sidecar override for
authored fire sources. The projectile traces up to three cells from the source
direction and destroys the first door or zeroes the first object target slot.
Town object hits also reduce moral standing by 5 and do not use ship-broadside
depletion. Destroying a door also clears the active door auto-close tracker.

In top-down scenes, bare `Q` opens the save prompt, `QY` writes a
first-playable save-and-continue snapshot to `SAVED.GAM` and `SAVED.OOL`, and
`QN` cancels without disk writes. The writer patches the supported scene,
position,
calendar/clock, party status/HP/MP/level, spell-charge stock, reagent stock,
Moonstone phase slots, inventory counters, active-object table, timing tag, and
transport marker fields while preserving unresolved save-image bytes and
non-calm wind byte mappings from the selected save template: `SAVED.GAM`
normally, or `INIT.GAM` after `--from-init`.

It currently supports town movement, animation ticks, a Britannian calendar clock,
schedule-derived actor blocking and one-step schedule movement, K-Klimb and
walk-onto-stair floor changes with per-floor NPC relinking, door opening with a
start-of-Open auto-close pass
and public Open/Jimmy door response strings, visit-local already-open door
acknowledgement even after the one-door auto-close timer moves to a later door,
visit-local town secret-door reveals whose Open path stays open without arming
the normal auto-close tracker, survives town floor reloads during the visit,
and clears on full location exit,
clean-return-checked location exits, clean-room sidecar-backed town and
overworld Get plus town Push, trap doors, town exit tiles, and Hole-up rest, and
a clean-room
first-playable dungeon text view with public-spec movement and ladder
transitions plus public pit, bomb trap, and typed energy-field status/damage
reactions, fountain drink effects, underfoot chest opening, torch/light blackout,
gem-backed map views, sidecar-backed scripted teleports and heavy-door opening,
and room-trigger arena diagnostics.
NPC schedules link active-object slots and advance only in town-family scenes;
off-floor schedule changes detach or attach visible slots by zeroing and
first-empty allocation while exact stair subtype routing can be supplied through
clean sidecar metadata. Overworld and dungeon turns leave any stale or synthetic
schedule state inert. Town entry also attaches the high-indexed player phantom
NPC to a sentinel active-object row; first-playable collision, rendering, and
line-of-sight helpers keep that sentinel logical-only so the canonical player
remains slot zero.
Town floor entry and reloads harvest asterisk spawn markers and `0x48`/`0x49`
NPC start markers before replacing those metadata bytes with the open-floor
placeholder in the live grid; chair/seat markers remain untouched for schedule
pathfinding.
Town entry and hour
boundaries apply the public dawn/dusk gate substitution. Dungeon entry
uses the public surface/underworld seed positions and facing rules, including
the Doom underworld-entry exception and trigger-class entry cells. Dungeon
room, rest-ambush, and outdoor encounter routes can enter the combat frame,
which loads arena terrain, snapshots caller state, places actors, and routes
player commands, monster AI, spells, fields, rewards, escape, and victory
cleanup. Combat-frame exits restore the pre-combat active-object table and
reconcile the caller's original terrain trigger slot, including water-creature
victory rewrites into persistent body/retrieval objects while defeat and
live-foe escape clear the trigger. It also
has Britannia and Underworld overworld
debug views with wrapping movement, runtime `.OOL` object-overlay rendering,
active-object phase animation and consumed-turn off-neighborhood pruning, basic semantic
vehicle boarding/exiting/sail state including waterborne ship boarding, magic
carpet board/exit with normal outdoor timing plus sidecar-backed lava and
authored drowning-water damage, semantic debug balloon wind drift and landing
refusal, one-cell mounted-horse movement with the public horse terrain
predicate, outdoor climb gates, and the public
two-minute
outdoor turn cadence with the saved `Q` timing tag providing skiff/raft
half-time plus alternate-turn active-object/encounter epilogue cadence, and
the saved `T` tag skipping minute/light-counter writes while suppressing that
world object epilogue. The play harness exposes semantic `--wind`,
`--grapple`/`--climbing-gear`, `--transport`, and `--pending-vehicle` startup hooks for focused testing;
world load and overworld entry messages report `Calm/North/South/East/West
Winds`, or the directionless `Winds` suffix for preserved out-of-range save
bytes, without claiming byte-perfect save mappings. Mode entry and cross-area
transitions run a zero-minute cleanup that refreshes the cached ambient
daylight/visibility-dirty state without spending a turn. Consumed turn cleanup
also reasserts slot zero as the canonical player/avatar active object before
NPC and active-object updates. Clean sidecar-backed chasm falls apply
first-playable fall damage only to conscious party members.
Snapshot-backed interior entries start on foot while preserving the outside
world snapshot; exits restore the saved world grid, active objects, boarded
transport, and timing/status plus ship-sailing cadence/refusal feedback, while
the clean world-location table fallback returns on foot. Dungeon fall-trap
chains that run past level 7 clear dungeon mode at the trap-chain X/Y instead
of using the exterior return coordinate; with no in-memory world snapshot, the
world-location row supplies only the target plane/map to materialize that scene
clear. Plane swaps, moongates into a different plane, and scripted
dungeon-to-world descents clear ship-sailing cadence/refusal feedback when they
force foot transport. The terminal surface
renderer uses that cached light as a visibility-radius gate and runs the public
centre-out visibility carve with the fixed W/SW/S/SE/E/NE/N/NW neighbour order,
terrain propagation blockers, and orthogonal-only propagation tiles. An
overworld water tile under the party clears the effective radius for that render
iteration. Town and world active objects composite with their current tile glyphs
and lower active-object slot priority in overlapping cells; they do not
participate in terrain visibility propagation. The
active-object tick covers phase countdowns, vehicle frame updates, wind-driven
ship-family drift, phase-zero ambient actor wandering with low-to-high slot
order and collision/terrain checks, and consumed-turn post-animation scroll-base
off-neighborhood pruning. Consumed overworld turns can also run
sidecar-authored encounter probes that allocate one nearby monster/NPC active
object after the turn work completes. Frame-only active-object tile changes mark
the terminal view dirty for the next redraw. Runtime
vehicle/fire removals, overworld scroll-base off-neighborhood pruning, town
floor changes, and scheduled NPC relinking preserve table indices by zeroing
non-player slot type bytes. Saved/OOL
active-object decoders and the first-playable save writer keep empty non-player
slot positions and payload bytes, plane transitions carry a runtime per-plane
overlay cache without rewriting local asset files during movement, and vehicle
exit or NPC relinking parks into the first empty active-object slot before
appending. Slot
zero is repaired as the canonical player/avatar record
when syncing movement state, running consumed-turn cleanup, advancing idle
visual ticks, or redrawing a text frame, and duplicate player records in nonzero
active-object slots have their type byte cleared so stale player rows cannot
block or render as ordinary objects while preserving slot payload bytes;
return-world snapshots use the same repair path so non-player slot indices are
not shifted while restoring the player row. Queued
shipwright-style deliveries supplied through the debug startup hook consume the
same first-empty-then-append active-object allocation on world entry. Ships preserve
decoded hull/skiff auxiliary
bytes through board, sail-toggle, and parked-object exit state, and ship board
or exit reports the public badly-damaged warning whenever hull condition is
below ten, plus the no-skiffs warning when that auxiliary count is zero. A
furled-ship X-it without nearby foot landing launches a carried skiff per
`vehicles.md` §5: the hull stays parked at the original cell with one fewer
skiff, and the party becomes the launched skiff in place.
Britannia's sparse `BRIT.DAT`
chunks are decoded at runtime by locating the public-shape chunk-index table
in `DATA.OVL`; the table is not committed to the repository. It is a test
harness for the gameplay core, not the final Bevy presentation layer.
The terminal harness also exposes `.` as a deterministic idle visual tick:
active-object and shared static tile animation advance while preserving
off-neighborhood overworld objects until the next consumed overworld turn, and
the public water, lava-rock, brazier/fireplace, and field-effect four-frame
families animate, but the
terrain/effect frame selector is family-wide rather than per-cell. The game
clock, turn counter, NPC schedules, light counters, and door auto-close tracker
do not. `Space` routes through the command handlers as the
public pass/wait action in both top-down and dungeon modes, spending a turn
just like an empty terminal input line. If a moongate yes/no prompt is pending,
empty input and non-answer keys repeat the prompt instead of spending a pass
turn, advancing idle animation, or triggering the harness quit shortcut. Empty
input in dungeon mode still honors immediate room-trigger underfoot reactions
before falling back to Pass.

For targeted verification, the play harness also accepts a public scene key,
floor, and optional start cell:

```powershell
cargo run -- --play --scene CASTLE:0 --floor 0 C:\Games\U5-Clean
```

Dungeon records use the public `DUNGEON:n` record key:

```powershell
cargo run -- --play --scene DUNGEON:0 --floor 0 C:\Games\U5-Clean
```

Ordinary dungeon ladders are level-to-level only unless a clean
`dungeon_deeper_transitions.tsv` row is present for the deepest-level down
ladder cell. Without a matching sidecar row, a deepest-level `K` command still
fails in place. With a row, the harness spends one dungeon turn, exits dungeon
mode, reloads the destination world plane, and places the party at the scripted
world coordinate:

```text
# DUNGEON LEVEL X Y TO_PLANE TO_X TO_Y
DUNGEON:6 7 1 1 UNDERWORLD 30 40
```

Scripted dungeon level teleports can be supplied as `dungeon_teleports.tsv`
while the exact cell identities remain open in the public spec:

```text
# DUNGEON LEVEL X Y TO_LEVEL TO_X TO_Y [CELL]
DUNGEON:7 7 4 4 3 1 1 0x70
```

When the party steps onto a matching row, the harness consumes one dungeon turn,
keeps the active dungeon scene, changes to the destination level, and places the
party at the destination coordinate. The optional cell guard prevents stale
coordinates from firing after the local dungeon image changes.

Dungeon gust artwork is treated as ordinary dungeon terrain for torch handling.
The runtime does not load a wind-tile sidecar or extinguish torches on gust
contact; torch duration advances through the normal dungeon turn counter.

Immediate dungeon-exit cells use the same clean-room coordinate pattern:

```text
# DUNGEON LEVEL X Y [CELL]
DUNGEON:0 5 4 4 0x70
```

When the party steps onto a matching row, the harness exits dungeon mode and
uses the in-memory debug return point or the `world_locations.tsv` return row
for that dungeon. If neither source is available, the party stays in dungeon
mode and reports missing clean return-coordinate metadata instead of moving to a
placeholder surface cell. Matching rows fire before fallback packed-cell
walkability can block the cell, which lets clean metadata model exit tiles whose
exact class is still open.

Dungeon heavy-door cells use a sidecar for the same clean-room reason: public
specs identify the packed 0xF family but leave the complete low-nibble split
between room triggers, heavy doors, and revealed secret doors outside this
repository. Place rows next to the game data as `dungeon_doors.tsv`:

```text
# DUNGEON LEVEL X Y OPEN_CELL [CLOSED_CELL]
DUNGEON:0 0 2 1 0x70 0xF2
```

Closed matching rows block movement instead of triggering a room. `O` on the
party's current dungeon cell rewrites that cell to `OPEN_CELL` and consumes a
turn; `J` with keys uses the same authored row for a deterministic
first-playable unlock and also rewrites the visit-local cell to `OPEN_CELL`.
Dungeon doors do not auto-close. The optional closed-cell guard prevents a
stale row from rewriting an unexpected packed cell, and `OPEN_CELL` can be an
0xF open-door variant that the sidecar marks as walkable and prevents from
firing as a room trigger before the command handler runs.

Authored dungeon chest grants can be supplied as deterministic overrides for
fixtures and clean-room scenarios:

```text
# DUNGEON LEVEL X Y CELL|* ITEM AMOUNT [ITEM AMOUNT ...]
DUNGEON:0 0 1 1 0x4c GOLD 12 GEMS 1
```

Matching `dungeon_chests.tsv` rows are consumed when `G` gets an opened dungeon
chest cell. Opening and searching own trap/detail handling and leave content
generation to the later Get. The cell guard prevents stale authored contents
from applying after the local dungeon image changes; `*` skips that guard. If
no row matches, the runtime uses the published dungeon chest reward generator.
Supported authored grant families are food, gold, keys, gems, torches, potions,
and scrolls.

Town walk-on stairs use the public `0xC4..0xC7` facing-sensitive tile family:
entering along the tile's encoded facing moves up one floor, entering from the
opposite facing moves down one floor, and side crossings stay on the current
floor. Underfoot `K` ladder/trapdoor direction can still be supplied as
clean-room sidecar metadata:

```text
# SCENE FLOOR X Y UP|DOWN|BOTH [TILE]
CASTLE:0 0 12 8 UP 80
CASTLE:0 1 12 8 DOWN
```

Matching `town_stairs.tsv` rows make `K` use the authored one-way or two-way
direction instead of inferring both directions from floor availability. `<` and
`>` are refused when they contradict a one-way row. Vehicle X-it landing
selection also treats matching rows and native walk-on stair tiles as transition
cells to avoid dismounting onto an immediate floor-change square. The optional
tile guard prevents stale metadata from affecting a changed visit cell.

Town lock states can be supplied as clean-room sidecar metadata while the exact
surface door lock-state byte pairs remain open:

```text
# SCENE FLOOR X Y LOCKED_TILE UNLOCKED_TILE [LOCKED|MAGIC]
CASTLE:0 0 12 4 97 96
CASTLE:0 0 14 4 96 97 MAGIC
```

Matching `town_locks.tsv` rows make `O` refuse the locked cell without spending
a turn. `J` with keys rewrites ordinary locked cells to `UNLOCKED_TILE`, marks
the visit-local map dirty, and consumes one indoor turn; `MAGIC` rows report
the magic-lock refusal without spending a turn or key. Missing rows keep the
existing first-playable door behavior.

Blink destinations can be supplied separately as clean-room metadata while the
spell's exact default range remains open:

```text
# TARGET FLOOR FROM_X FROM_Y DIRECTION TO_X TO_Y [FROM_TILE|*] [TO_TILE|*]
CASTLE:0 0 12 4 E 14 4 16 16
DUNGEON:0 3 1 1 W 0 1 0x00 0x08
BRITANNIA 0 10 20 E 12 20 * 16
```

Rows are same-map only: the target names the active world plane, town-family
scene, or dungeon scene; `FLOOR` is the town floor, dungeon level, or world
plane save floor (`0` for Britannia, `-1` for Underworld). The direction must
match the inline cast suffix (`C1IP6`, `C1IP4`, `C1IP8`, or `C1IP2`). Optional
tile guards keep stale rows from firing; `*` means no guard. Destination cells
must be legal foot landing cells, so Blink can skip an intervening locked door
when a clean row targets the far side, but it cannot land on blocked terrain,
active objects, stair/exit/trap transition cells, visible moongates, waterfalls,
or disallowed/foot-damaging damage tiles.

The overworld maps can be entered directly for movement testing:

```powershell
cargo run -- --play --scene BRITANNIA C:\Games\U5-Clean
cargo run -- --play --scene BRITANNIA --wind east C:\Games\U5-Clean
cargo run -- --play --scene BRITANNIA --grapple 1 C:\Games\U5-Clean
cargo run -- --play --scene BRITANNIA --transport balloon --wind east C:\Games\U5-Clean
cargo run -- --play --scene BRITANNIA --pending-vehicle frigate:10,20,2 C:\Games\U5-Clean
cargo run -- --play --scene UNDERWORLD C:\Games\U5-Clean
```

`--transport` is a semantic debug startup hook for `foot`, `horse`, `ship`,
`skiff`, `carpet`, and `balloon`. Horse, ship, skiff, and carpet state now
round-trip through the public save marker families; the balloon option remains
debug-only and is intentionally not a claim about B-Board support.

`--pending-vehicle` is a clean debug hook for the public shipwright-delivery
handshake. Talk-entered shop sessions cover arms, healers, inns, taverns,
sages, reagent sellers, guilds, shipwrights, horse traders, and companion
pickup/dropoff flows through the shared prompt dispatcher. Use
`frigate:x,y[,skiffs]` to place a ship-family active object with the
first-playable full-hull auxiliary value and the supplied skiff count, or
`skiff:x,y` to place a standalone skiff.

Overworld fixed-location entry reports a diagnostic until clean entrance
coordinates are published. If you have a clean-room coordinate table, place it
next to the game data as `world_locations.tsv` with one row per entry:

```text
# PLANE X Y TARGET [TOWN_ENTRY_Y] [TILE]
BRITANNIA 0 0 CASTLE:0 7 24
UNDERWORLD 0 0 DUNGEON:0 24
```

The repository intentionally does not ship that coordinate table.
For town-family targets, the optional fifth column is the clean
`LocationEntryYTable` value; X is fixed at 15 and floor is 0. A town row can use
an optional sixth source-tile guard after that entry Y, while a dungeon row can
use an optional fifth source-tile guard. The guard keeps stale coordinates from
firing after local map edits. Town-family entries are surface entries and must
use `BRITANNIA`; dungeon entries may use `BRITANNIA` or a clean scripted
`UNDERWORLD` row. Each target may appear only once so exits can resolve a single
unambiguous return coordinate.
Direct town or dungeon sessions also use the same table to resolve exits back
to the overworld when no in-memory return point exists. Missing town or dungeon
return rows keep the party in the current interior mode with a diagnostic,
matching the public gazetteer contract.

Shrine coordinates can be supplied separately while the resident shrine table is
not public:

```text
# PLANE X Y VIRTUE [TILE]
BRITANNIA 10 20 HONESTY 136
BRITANNIA 11 21 HUMILITY
```

Rows are Britannia-only and use the public virtue order/mantras. Optional tile
guards prevent stale authored coordinates from firing after a map edit. The
runtime tracks ordained and Codex masks at the public save offsets and applies
the public shrine stat rewards to the Avatar stat snapshot; shrine standing is
kept as first-playable runtime state until its exact save layout is published.

Surface/underworld chasm and ascent transitions can be supplied as
`world_plane_transitions.tsv`:

```text
# FROM_PLANE X Y TO_PLANE TO_X TO_Y [TILE]
BRITANNIA 10 20 UNDERWORLD 30 40 24
UNDERWORLD 30 40 BRITANNIA 10 20
```

When a player steps onto a matching coordinate, the harness keeps the world
mode active, swaps the plane, loads the destination map and overlay objects,
and places the party at the destination coordinate. Source and destination
coordinates must each be unique so the sidecar keeps every transition pair
unambiguous. Matching transition rows fire before fallback tile passability can
block the cell, which lets clean chasm/ascent metadata model transition tiles
whose exact class remains open. Consumed top-down commands while already
standing on a matching row apply the same underfoot plane transition after turn
cleanup.
The optional source-tile guard keeps stale coordinates from firing after local
map edits.
Mounted-horse movement follows the public one-cell overland step contract, so
transition and current-sweep rows only fire when the accepted destination cell
itself matches the sidecar row.
Britannia-to-Underworld falls also apply deterministic first-playable fall
damage to living saved-party members; ascent rows only move the party between
planes.

Waterfall/current sweep cells use a separate sidecar while the exact water-tile
variant table remains open:

```text
# PLANE X Y DIRECTION STEPS [TILE]
BRITANNIA 40 50 EAST 3 1
```

When a normal world movement lands on a matching `world_waterfalls.tsv` row, the
harness treats that coordinate as an authored current cell before fallback
transport passability, consumes the original movement turn, then sweeps the
party up to `STEPS` cardinal cells in `DIRECTION`, stopping before blocked
terrain or an active object. If the sweep reaches a matching
`world_plane_transitions.tsv` coordinate, that plane transition fires
immediately; if it reaches a visible moongate origin, the normal landing prompt
is queued. The optional tile guard keeps stale coordinates from firing after
local map edits.

World damaging terrain includes the public molten-lava tile `0x8F`; foot and
carpet travel can enter it and take deterministic first-playable lava damage,
while mounted horses and watercraft are blocked. Additional authored damage
cells can be supplied as a clean-room sidecar while the exact water/current
damage split and original damage formula remain open:

Overworld swamp tile `0x04` runs the public per-turn status helper for on-foot
travel, poisoning living party members that are not already poisoned.

```text
# PLANE X Y EFFECT [TILE]
UNDERWORLD 40 50 LAVA 14
BRITANNIA 41 50 DROWNING 1
```

Rows currently support `LAVA` and `DROWNING`; `WATER` is accepted as a
`DROWNING` alias. A matching `LAVA` row marks the cell as lava for movement
rules: magic carpets can cross it, other transports are blocked except the
semantic debug balloon, and a successful carpet crossing applies deterministic
first-playable damage to living party members while balloon overflight does not.
A matching `DROWNING`/`WATER` row marks an authored water/current cell as
enterable by foot and water/air/carpet transports, blocks horses, and applies
deterministic first-playable damage only to foot travel. Explicit world debug
starts validate sidecar transport allowance so focused tests can start on
authored hazards, while automatic fallback start selection skips cells that
would damage the selected transport. Consumed top-down commands while standing
on an authored world damage cell apply the same underfoot damage check as
movement.
The optional tile guard keeps stale coordinates from firing after local map
edits.

Random overworld encounter spawning uses the public terrain threshold, retry,
branch, and weighted bucket tables. For deterministic focused tests or
clean-room scenario authoring, place override rows next to the game data as
`world_encounters.tsv`:

```text
# PLANE TILE THRESHOLD TYPE DX DY [PHASE]
BRITANNIA 5 30 192 8 0
UNDERWORLD 14 12 255 -8 4 0x12
```

Native encounters roll a deterministic clean-room value in `1..30` and spawn
when the public threshold strictly exceeds that roll. The saved `Q` timing tag
checks the encounter probe only on alternate turns, while `T` suppresses it.
If `world_encounters.tsv` is present, matching rows spawn through the authored
sidecar path; unmatched tiles fall back to the native public selector. `TYPE`
must be a monster/NPC active-object byte in `192..255`. `DX`/`DY` place the new
object relative to the party with wrapping world coordinates and must stay
within the active-object neighborhood radius.
If `PHASE` is omitted, the spawned actor starts facing back toward the party;
an explicit phase may supply a direction nibble with a non-steady animation
nibble. Spawning uses the normal first-empty active-object slot and is skipped
when the target cell is occupied, is the party cell, is not foot-landable, is an
active moongate origin, or is a clean sidecar-authored transition/hazard that a
foot actor should not occupy.

Natural moongates follow the public saved-Moonstone-slot schedule. During the
night band, eligible saved Moonstone slots in the current surface or town scene
are stamped as live `0xDC` gate terrain; during the day band, the shared gate
counter wanes and those cells restore to terrain `5` when the counter reaches
zero. Stepping onto a live natural gate clears that cell, then either opens the
midnight meditation path during hour `0`, minutes `0..9`, or teleports through
the saved Moonstone phase selected by the cached moon glyph for the current
half-day.

The clean sidecar `moongates.tsv` remains available for authored transition
fixtures and rendering smoke tests:

```text
# ORIGIN_X ORIGIN_Y DEST_PLANE DEST_X DEST_Y [TILE]
# ORIGIN_X ORIGIN_Y DEST_PLANE DEST_X DEST_Y START_HOUR END_HOUR [TILE]
10 20 BRITANNIA 30 40 24
12 22 UNDERWORLD 44 55 22 2 24
```

Rows are surface origins for the sidecar transition fixture, not the native
natural-gate placement schedule. Optional hours are inclusive and may wrap
across midnight; when hours are omitted, a sixth column is treated as an
optional source-tile guard. When hours are present, an optional eighth column
provides that same guard. A guarded row is active only while the origin still
has the expected tile, keeping stale authored coordinates from rendering or
prompting after local map edits. Active sidecar gates also require full
daylight from the cached ambient-light state before they render over the world
view, allow stepping onto their cell, or respond to `E`. The rendered gate
sprite advances through the public 16-frame moongate animation plate on visual
ticks at the origin and at Britannia destinations only while a daylight-active
gate is visible; underworld destinations are transition targets but are not
rendered on the surface view. A destination coordinate of `255 255` is the
public single-ended sentinel: the origin can render and prompt, but it does not
render a destination overlay or teleport the party. Stepping onto a visible
sidecar origin, or completing any turn-consuming top-down action while already
standing on one, queues the first-playable landing prompt; `Y` teleports
through the queued destination without spending a second turn, while `N` leaves
the party on the gate.

Secret-door search metadata can be supplied as a clean-room sidecar while the
public dungeon low-nibble and town object-table encodings remain open. Place
rows next to the game data as `secret_doors.tsv`:

```text
# TOWN SCENE FLOOR X Y REVEAL_TILE [TILE]
TOWN CASTLE:0 0 12 4 96 24
# DUNGEON SCENE LEVEL X Y REVEAL_CELL [CELL]
DUNGEON DUNGEON:0 0 2 1 0xF0 0x30
```

In the terminal harness, uppercase `S` prompts for a Search direction so
lowercase `s` can remain south/back movement; inline forms such as `S6` still
route in one command. Matching town rows reveal a wall cell as the supplied
door tile; the revealed town secret door responds to Jimmy with `No lock!` and
bare `O` prompts before rewriting it to the open-door placeholder without
arming the normal auto-close tracker, so it stays open for the visit. Matching
dungeon rows rewrite the facing dungeon cell to the supplied packed cell byte.
Optional guards check the current town tile or dungeon packed cell before
revealing, keeping stale coordinates from rewriting unrelated cells. Town
misses do not spend a turn; dungeon misses continue through the ordinary
dungeon Search feature/chest/trap path.

Town-family fire sources use adjacent static cannon tiles from the public
`0xB4..=0xB7` four-facing family by default. The low two bits choose North,
East, South, or West. Clean-room authored fire-source rows can still be placed
next to the game data as `town_fire_sources.tsv`; matching sidecar rows take
priority over native cannon detection:

```text
# SCENE FLOOR SOURCE_X SOURCE_Y DIRECTION [TILE]
CASTLE:0 0 1 1 EAST 80
```

`F`/`f` runs the public door auto-close pass before source detection. When the
party is adjacent to a matching sidecar source or native cannon, the command
consumes a turn, traces from the source in the supplied or tile-derived
cardinal direction for up to three cells, zeroes the first active-object slot
hit, or rewrites a door tile in `96..103` to the current open-door placeholder
and clears the active auto-close tracker. Missing sources or rows whose
optional source-tile guard does not match refuse without spending a turn.

Town and overworld Get tile consumables can be supplied as clean-room sidecar
metadata for authored crop, borrowed-object, and scenario fixture cells. Public
table-food tiles `0x9B` and `0x9C` are handled natively with their directional
rewrite rules. Place additional town rows next to the game data as
`town_get_tiles.tsv`:

```text
# SCENE FLOOR X Y REPLACEMENT_TILE [TILE] [ITEM AMOUNT]
CASTLE:0 0 12 6 16 44 FOOD 4
CASTLE:0 0 13 6 16 GOLD 9
```

Overworld rows use the same replacement-and-guard shape in
`world_get_tiles.tsv`:

```text
# PLANE X Y REPLACEMENT_TILE [TILE] [ITEM AMOUNT]
UNDERWORLD 40 12 5 44 GEMS 1
```

`G`/`g` first checks native table-food handling, then checks the facing town
cell or wrapped overworld cell for a matching sidecar row. A matched row
optionally verifies the current tile id, rewrites the live tile to
`REPLACEMENT_TILE`, optionally applies the shared pickup grant family, marks the
map dirty, and consumes one turn. Missing or mismatched rows do not spend a
turn.

Visible active-object pickups can be opted in separately with
`object_pickups.tsv` while the active-record quantity/subtype byte ownership is
being wired to the public inventory-add dispatcher:

```text
# TARGET FLOOR X Y ITEM AMOUNT [TILE]
BRITANNIA 0 10 20 GEMS 2 210
BRITANNIA 0 11 20 GOLD 9
CASTLE:0 0 2 1 KEYS 1 210
```

Supported items include ordinary counters, potions, scrolls, equipment, skull
keys, HMS Cape plans, the Sandalwood Box, magic carpet stock, regalia, and
Shadowlord shards. The optional guard checks the active-object tile id, not the
map tile. A matching row frees the active object slot, applies the matching
shared inventory effect, marks visibility dirty, and consumes one turn. Missing
rows, mismatched floor/coordinate data, or a mismatched optional tile guard
leave the object in place and fall through to the normal active-object refusal.

Eternal Flame coordinates can be supplied as clean-room metadata while the
public coordinate catalog remains deferred:

```text
# TARGET FLOOR X Y FLAME [TILE]
BRITANNIA 0 5 5 TRUTH 16
UNDERWORLD -1 20 40 LOVE
CASTLE:0 0 12 10 COURAGE 80
```

`U`se on a carried Shadowlord shard now checks the existing public shard
preconditions, then looks for a matching `eternal_flames.tsv` row at the
party's current target/floor/coordinate. The row's `FLAME` must be the opposed
principle for that shard, and the optional tile guard checks the current map
tile. On success the shard is consumed, the matching Shadowlord slot is marked
vanquished, matching active Shadowlord encounter objects on the current floor
are cleared, and one turn is consumed. Missing rows, stale tile guards, or the
wrong flame keep the existing no-effect branch without consuming the shard.

Town Push is externalized for the same clean-room reason: the public spec
defines the swap behavior, but not the complete movable-tile table. Place rows
next to the game data as `town_pushables.tsv`:

```text
# SCENE FLOOR X Y [TILE]
CASTLE:0 0 12 6 44
```

`P`/`p` checks the facing town cell for a matching row, optionally verifies the
current tile id, and if the destination cell beyond it is walkable swaps the
two live tile IDs. Missing or mismatched rows do not spend a turn; a matched
pushable tile with a blocked destination consumes the push attempt without
rewriting the map.

Town Hole-up rest uses a clean bed sidecar while the complete inn/bed tile table
is still outside the public implementation surface. Place rows next to the game
data as `town_rest_beds.tsv`:

```text
# SCENE FLOOR X Y [TILE]
CASTLE:0 0 14 10 55
```

In the terminal harness, bare `h` opens the duration prompt; inline shortcuts
such as `h8` still work. The outdoor/dungeon rest-with-watch prompt asks
whether to set watch when more than one living Good/Poisoned/Sleeping party
member can participate, and inline input also accepts a watcher slot such as
`h8/2`. A matching bed row advances one in-world hour per iteration, decays personal light
counters, applies the existing dawn/dusk cleanup, and runs one NPC schedule tick
per hour. Town bed rest also applies deterministic first-playable HP recovery to
living party members plus byte-capped first-playable MP recovery. The exact
HP/MP recovery amounts remain blocked on clean spec issue `cleak/u5-spec#47`.
Encounter
interruption is owned by the overworld/dungeon rest-with-watch path rather than
town beds.
In overworld and dungeon modes the same `h8` input runs the first-playable
rest-with-watch path: each rested hour performs three 20-minute cleanup ticks,
including time, light counters, animation, and existing area-specific per-turn
hooks such as authored overworld damage tiles. A supplied watcher must be a
living Good-status party member; invalid watcher choices leave no watch set.
The sleep-ambush predicate follows the public one-in-sixty-four rest/camp rule
and hands the selected ambush monster to the combat frame when it fires. The
watch path also applies
deterministic first-playable HP recovery to
living party members, byte-capped first-playable MP recovery, and wakes members
who were asleep when rest began; the numeric recovery amounts share the same
`cleak/u5-spec#47` blocker.

Town trap-door cells are also clean-room sidecar metadata while the exact
interior tile encoding remains open:

```text
# SCENE FLOOR X Y TO_FLOOR [TILE]
CASTLE:0 0 12 9 -1 55
```

Stepping onto a matching row reloads the target floor, keeps the party at the
same X/Y, relinks NPCs for that floor, and consumes one indoor turn. Consumed
top-down commands while already standing on a matching row apply the same
underfoot trap-door transition after turn cleanup without spending a second
turn. Missing or mismatched rows behave like ordinary movement.

Town poison-gas doorway cells can be supplied as clean-room sidecar metadata:

```text
# SCENE FLOOR X Y [TILE]
CASTLE:0 0 12 9 55
```

Stepping onto a matching row, or completing any turn-consuming top-down command
while already standing on one, runs the town underfoot poison-gas branch against
eligible Good living party members. The public spec identifies the roll branch
but not its exact odds, so this remains a deterministic first-playable roll
until a clean row/odds contract is published.

Town boundary exits use the native public threshold tile `0x59`. Additional
authored exit cells can be supplied as clean-room sidecar metadata:

```text
# SCENE FLOOR X Y [TILE]
CASTLE:0 0 15 31 55
```

Stepping onto native `0x59` or a matching `town_exit_tiles.tsv` row consumes one
indoor turn and returns to the saved debug world snapshot, or to the matching
`world_locations.tsv` row when no snapshot is available. If the exit trigger
matches but no clean return coordinate exists, the party stays in location mode
with a diagnostic. Consumed top-down commands and pass/empty waits while already
standing on a matching exit trigger apply the same underfoot exit transition
after turn cleanup without spending a second turn. Missing or mismatched
sidecar rows behave like ordinary movement.

The town entry-Y table can also be supplied separately as
`location_entry_y.tsv`, which is useful for direct `--scene` starts and for
`world_locations.tsv` rows that omit the fifth column:

```text
# SCENE ENTRY_Y
CASTLE:0 7
```

Town floor loading also has an optional clean-room table. If you have the
public `LocationFloorBaseTable`, place sparse overrides next to the game data
as `location_floor_pages.tsv`:

```text
# SCENE BASE_PAGE
CASTLE:0 5
```

The floor byte is signed: floor `0` reads the base page, floor `1` reads the
next 1,024-byte page, and floor `-1` reads the previous page. Without this
file the harness falls back to the physical two-page pairing in the location
`.DAT` files.

Town and overworld movement can also consume the public tile passability
bitmap as a clean-room sidecar. Place the 32-byte bitmap next to the game data
as `tile_passability.bin`. Tile id `n` uses byte `n >> 3` and mask
`0x80 >> (n & 7)`; a set bit means broadly passable. Without this file the
harness keeps using its class-derived fallback.

For transition plumbing tests before those clean coordinates exist, use
`--debug-enter` with an overworld scene. Press `e` to enter the requested
town or dungeon from the current overworld coordinate; exiting restores that
same overworld map and coordinate. When `world_locations.tsv` is present, that
authored table is authoritative: a matching row enters, and a missing row blocks
without falling back to `--debug-enter`.

```powershell
cargo run -- --play --scene BRITANNIA --debug-enter CASTLE:0 C:\Games\U5-Clean
```

Use `--time HH:MM` to choose the schedule sampling time for focused NPC
movement tests.

Use `--from-save` to seed the play harness from `SAVED.GAM` when the save is in
the overworld, a town, dwelling, castle, keep, or stock dungeon scene. Spell
charges, reagent counters, party MP, party level, saved spell-light counter, and
saved Moonstone phase slots are read so the narrow Light, restore, Rel Hur,
Create Food, field placement, Dispel Field, Magic Lock, Unlock Magic, Gate
Travel, and M-Mix hooks can exercise the public resource gates. The food and
gold counters are also read for Z-stats, object pickups, and save export, while
Create Food updates the food counter. Hour-crossing cleanup now applies the
public provision cadence, subtracting active eaters at 06:00, 12:00, and 18:00
with Dead, Ashes, and Sleeping members excluded. Poisoned living members still
count as provision consumers and take a first-playable one-HP poison tick on
each hourly status/provision pass. If food is already zero at an hour crossing,
the starvation branch appends a warning and applies first-playable one-HP
starvation damage to living members. The harness does not require character
creation first, so clean test saves with a
blank Avatar name can still seed the playable slice:

```powershell
cargo run -- --play --from-save C:\Games\U5-Clean
```

For overworld saves, the harness also restores the embedded live active-object
table from `SAVED.GAM` and maps recognized transport-marker families into the
runtime movement state. For dungeon saves, it restores the 512-byte dungeon
working buffer from `SAVED.GAM`, preserving visit-local edits such as opened
doors, trap rewrites, and dispelled fields instead of replaying only the durable
room-clear bitmap from static `DUNGEON.DAT`. It restores the full saved
year/month/day/hour/minute clock. It reads the separate timing/status tag as `Q`
half-time or `T` no-minute/no-light-counter cleanup; other values are treated as
normal timing. Exact ship facing/sail marker variants remain an open public-spec
table. `--from-init` keeps the factory bootstrap path by reading `INIT.GAM` plus
the surface `INIT.OOL` overlay seed, so fresh bootstrap does not depend on stale
`SAVED.OOL` surface objects.

On `--from-save`, the loader also validates canonical `SAVED.OOL` and refreshes
the per-plane `BRIT.OOL` and `UNDER.OOL` mirrors from its two halves, matching
the public load-time mirror contract before gameplay starts.

During top-down play, uppercase `Q` opens the public save-and-continue prompt,
with inline confirmation shortcuts still accepted: enter `QY` to write
`SAVED.GAM`, canonical `SAVED.OOL`, and refreshed `BRIT.OOL` / `UNDER.OOL`
per-plane mirrors, or `QN` to cancel. Inactive world-plane object staging comes
from those per-plane mirrors, with the active plane supplied by live state; the
underworld mirror is defensively re-flushed in the normal save entry mode. The
first-playable writer intentionally leaves `INIT.GAM`, `INIT.OOL`, and static
map assets alone; dungeon-mode saves write the current 512-byte dungeon working
buffer into the save image. After `--from-init`, unresolved `SAVED.GAM` bytes are
templated from `INIT.GAM` instead of any stale saved game.

Use `--from-init` to seed the same harness from `INIT.GAM` without running
character creation:

```powershell
cargo run -- --play --from-init C:\Games\U5-Clean
```
