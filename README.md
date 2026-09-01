# u5-engine

Verification harness and clean-room engine for the public Ultima V specs.

The executable exposes terminal and Bevy play loops backed by the same runtime
state. It reads the user's local Ultima V data at runtime, verifies public specs
against real files, and keeps raw game data out of the repository:

- town-mode scene partitioning;
- per-class `*.DAT`, `*.NPC`, and `*.TLK` joins;
- save/load, Journey Onward, Create Character, and Ultima IV transfer flows;
- world, town, dungeon, combat, shop, conversation, magic, rest/camp,
  Blackthorn, Codex, Shadowlord, and endgame runtime paths;
- public LZW graphics-envelope decoding for tile atlases, image directories,
  and sprite/mask sheets; canonical sparse-strip decoding for standalone
  `.BIT` and `PROPORT.PCS` resources, with explicit legacy compatibility for
  local preprocessed proportional text assets; and fixed font rasterization;
- atlas-backed top-down, dungeon, combat, intro, modal, status, and endgame
  frame rendering; and
- asset-backed route, frame, and visual-route smoke suites.

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
- `docs/bevy-minimal-playable.md` - Bevy v0 launch, smoke gate, and known
  limits.
- `docs/commands.md` - current A-Z command routing by mode, with representative
  test evidence.
- `docs/sidecars.md` - clean-room TSV/binary sidecar files accepted at runtime.
- `docs/status-matrix.md` - implementation status matrix and verification
  commands for the current clean engine.

For direct terminal play, start the shared runtime play loop:

```powershell
cargo run -- --play C:\Games\U5-Clean
```

## Bevy visual mode

A minimal Bevy frontend renders the same `PlayState` to a real window instead
of the terminal. It is feature-gated so the default build keeps the lean
verification dependency surface. It currently covers top-down overworld/town
scenes plus the clean first-person dungeon raster:

```powershell
cargo run --features visual -- --visual-playable C:\Games\U5-Clean
cargo run --features visual -- --visual --scene BRITANNIA C:\Games\U5-Clean
cargo run --features visual -- --visual --scene CASTLE:0 --floor 0 C:\Games\U5-Clean
cargo run --features visual -- --intro --visual C:\Games\U5-Clean
```

`--visual-playable` is the minimally playable Bevy v0 entrypoint. It opens the
Bevy intro/menu shell, lets the player create or transfer a save, then Journey
Onward into the same runtime gameplay loop used by the terminal harness.
After character creation, the menu returns with Journey Onward highlighted;
press Enter to load the new save. Journey Onward deliberately loads directly
into Iolo's Hut. Select `U` / `Ultima V Introduction` on the intro menu to play
the separate story sequence.

Interactive modes never write into the pristine asset directory. When the
supplied install is the default `C:\Games\U5-Clean` path or contains read-only
save/overlay files, the launcher creates a persistent writable mirror below
`%LOCALAPPDATA%\u5-engine\runtime` (or `$XDG_DATA_HOME/u5-engine/runtime`).
Existing saves are copied into that mirror on first use and later launches
preserve the played copy. Set `U5_ENGINE_RUNTIME_DIR` to choose another parent
directory for the mirror.

In terminal top-down play, use the arrow keys, numeric keypad, or unshifted
lowercase `wasd`/vi keys to move; Shift selects the conflicting uppercase game
commands. In Bevy gameplay, letter keys always enter the original A-Z command
dispatcher, while movement uses the arrow keys or numeric keypad. Name and
free-text entry preserve typed case in both frontends.

The analyzed DOS baseline has one audio backend, the IBM PC speaker, and ships
no external music tracks. `systems/audio.md` (spec commit `86bee4d`) publishes
that contract in full, and the engine implements it exactly: the single mono
one-bit channel, the timer-divisor rule, the four sound families (blocking
tone, linear glissando, random rumble, software envelope), the shared
potion/wind/spell variant table, and the confirmed trigger inventory. The Bevy
shell owns one voice - starting a new effect stops the previous one - and
synthesizes each effect from its published operation list rather than from a
pre-baked sample bank, because a rumble's pitches come from the private
sound-only jitter stream, the major full-viewport flash's 1,856 bands come from
the gameplay PRNG, and each intro ignition burst carries its own 25 pitches.

Press `Ctrl+S` during play to toggle the sound setting. Per `audio.md §3` this
changes output, not cadence: a muted effect still performs its calibrated hold
and its state advances, still consumes the same random draws, and still stops
the speaker at the published end. The Bevy shell therefore holds its one voice
silently for a muted effect rather than dropping it. The single exception is
the software envelope, whose silent arm `cleak/u5-spec#146` measured as
genuinely faster; the engine models that asymmetry instead of assuming mute
invariance. The intro subtitle-ignition burst is exempt from the toggle
entirely, because it runs before that command is available. Ordinary menu
input, walking, and generic successful commands are silent by contract
(`audio.md §9`), not by omission.

Wall-clock pacing derives from one anchor, the calibrated delay unit.
`cleak/u5-spec#146` answers it as 0.88 ms +/- 10%, notes that the calibration
count and the inner-step cost very nearly cancel across machines from a
4.77 MHz PC through a 486, and is explicit that its figures are static
derivations with modelling bands rather than measurements. The engine takes
that published value as its single anchor and derives every hold from it, so
the residual uncertainty is the spec's own band, not an engine guess. Step
counts, frequencies, divisors, iteration counts, and event ordering are exact.
Presentation is still v0: shops, conversations, and other text-heavy flows use
the shared fixed-cell status/modal surface while exact original window pacing
continues to be tracked as parity work.

The window draws a single CPU-generated 11x11 tile viewport (an `EGA` or `CGA`
indexed framebuffer converted to RGBA) into one Bevy `Image` and displays it
through one nearest-neighbor sprite. Gameplay still lives in `PlayState`: the
input system maps keyboard events into the same handlers used by the terminal
harness, so movement, blocking, doors, and supported area transitions work out
of the box. Dungeon scenes render a light-gated first-person corridor panel;
combat scenes render the tactical arena through the same atlas-backed viewport,
while shops, conversations, and other line-oriented interactions remain modal
runtime flows rather than bespoke Bevy UI. Modal prompts such as
conversation keywords, Blackthorn answers, and sage keywords collect typed text
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
| Arrow keys, numpad 8/4/2/6                          | Cardinal movement |
| Numpad 7/9/1/3                                      | Diagonal direction input where accepted |
| `E`                                                 | Enter             |
| `O`                                                 | Open              |
| `K`                                                 | Klimb             |
| `,` / `.`                                           | `<` / `>` floor   |
| `Space`                                             | Pass              |
| `A`-`Z`                                             | Command letters   |
| `A`                                                 | Attack, then direction |
| `G`, `J`, `L`, `O`, `P`, `S`, `T`                  | Adjacent direction commands |
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
Britannia, a moved Britannia frame, Castle:0, a lit Dungeon:0 frame, composed
combat, View/Peer/X-Ray/night-sky overlays, intro/status/modal surfaces, and an
endgame status panel, plus a sanitized manifest with dimensions, frame kinds,
positions, and hashes. The Bevy feature also provides `--visual-frame-suite`
for 193 composed Bevy-owned frames and `--visual-route-suite` for 1910
per-step route frames; both Bevy manifests include review coverage rows and
per-frame clean metadata for auditing generated screenshots. Use
`--compare-frame-manifests <BASE> <CURRENT>` to gate sanitized manifests by
frame labels, dimensions, frame kinds, hashes, nonblack counts, and review
metadata without committing PNGs:

```powershell
cargo run -- --save-frame-suite target\frame-suite C:\Games\U5-Clean
cargo run -- --compare-frame-manifests target\baseline\manifest.txt target\frame-suite\manifest.txt
```

For repeatable smoke checks, `--play-script` runs a semicolon-separated command
list through the same runtime input handlers and then exits. Script mode
prints compact state summaries and optional raster hashes instead of rendered
map frames. Use `empty` or `pass` for an Enter/Space pass turn, and `idle:N`
for N no-turn visual ticks:

```powershell
cargo run -- --play-script "d;empty;idle:4;q" --raster-diagnostics C:\Games\U5-Clean
```

`--route-smoke` runs a bundled 514-case local-asset route suite covering world,
town, dungeon, combat, shop, endgame, transition, save/reload, and modal
routes, including native Jimmy magic-lock, empty-restraint, prisoner-release,
and prisoner-removal save/reload paths. Town routes exercise the canonical
`.OOL` write/reload lifecycle, so point the command at a writable asset copy,
not the pristine install. It prints sanitized state lines and raster hashes; add
`--route-smoke-manifest <PATH>` to write a clean manifest with initial,
per-command, and final route labels, command counts, frame dimensions, hashes,
nonblack counts, and state hashes:

```powershell
cargo run -- --route-smoke target\acceptance-assets
cargo run -- --route-smoke --route-smoke-manifest target\route-smoke\manifest.txt target\acceptance-assets
```

`--play-script` can be combined with `--scene` for direct interior construction
without E-Enter narration. E itself always requires a published live tile and
coordinate; `--debug-enter` no longer bypasses that production contract:

```powershell
cargo run -- --scene CASTLE:0 --play-script "empty;q" C:\Games\U5-Clean
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
`Z` prints a text status summary covering active area, position,
time, transport, wind, typeahead status, light, inventory, mixed spells, and
runtime party order without spending a turn.
`K` climbs unambiguous town stairs and walking onto those stair tiles also
triggers the floor change; clean `town_stairs.tsv` rows can pin one-way versus
two-way stair direction where the public subtype table is still open, and
two-way town stairs prompt and let `<`/`>` choose the floor direction from the
stair cell. Outdoor `K` follows the spec Grapple/on-foot gates and exposes
semantic `--grapple` plus legacy `--climbing-gear` startup hooks for
mountain-family climbs. Fall checks run against
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
conversation header. Inline `T<keyword>` input, such as `TJOB`,
runs a one-shot keyword lookup against the decoded `.TLK` fields using the
public space-boundary match rule and applies supported TLK byte-runner side
effects. Bare Talk opens the interactive conversation keyword loop when raw
`.TLK` streams are available, and Talk-triggered shopkeepers route into the
modal shop sessions, including horse-trader purchases that place a nearby
boardable horse object. Raw `.TLK` and `SHOPPE.DAT` dictionary tokens expand
through the published public issue #33/#40 128-row shared dictionary, with
`common_words.tsv` still supported as an optional clean override for custom data.
TLK `0x85` gold-payment controls decode the three public digit bytes with no
extra confirmation read: the surrounding authored yes-answer record is the
consent. Affordable demands debit immediately and continue with the next stream
byte. Unaffordable demands discard the pending word, print the exact quoted
refusal, and enter the ordinary nested keyword loop whose eventual stop closes
the whole conversation. Ordinary exploration turns age the saved cooldown;
only a live linked class-108 speaker can test/reset it and award the published
milestone standing gain.
Overworld and dungeon Talk return the stock no-response path without spending a
turn.
Dungeon movement and the normal lit render are facing-relative: `W`/`S` step
forward/back, `A`/`D` turn left/right, blocked cardinal movement reports the
public `Blocked!` refusal, `K` climbs one-way ladders or prompts on two-way
ladders where `<`/`>` choose up/down, non-ladders return the public
`Not climbable!` refusal, and `L` looks forward. The
terminal renderer uses a clean text proxy for the public first-person
dungeon wireframe: it reports the current cell, up to
four forward bands, side cells at each band, and hides bands behind the first
front wall or boundary. `O` opens an underfoot dungeon chest in the visit-local
runtime image, and `G` gets an underfoot dungeon chest through the same
visit-local chest marker path with generated content/trap handling. `S`
searches the facing dungeon cell: clean sidecar rows can reveal
secret doors, public chest cells enter the same visit-local chest path, and
exact public bomb-trap bytes are marked as fired without changing level; other
public dungeon cell classes narrate without triggering movement tile effects.
Consumed non-movement dungeon commands and pass/empty waits already standing on
clean `dungeon_teleports.tsv` cells, public pit, bomb-trap, or field bytes run
the same post-action underfoot tile-effect pass,
without spending a second turn.
Dungeon exploration keeps the top-down active-object table out of its
turn and idle visual animators because the first-person dungeon renderer owns
its own position state; shared static animation still ticks. The dungeon raster
projects same-level active dungeon objects into the visible first-person depth
bands, while stale objects from other dungeon levels are ignored. Dungeon render
and `L`ook obey the public personal-light gate; optional
`dungeon_teleports.tsv` rows model scripted level-to-level cells. The withdrawn
`dungeon_exit_tiles.tsv` mechanism is not read: the public spec establishes that
no exit-dungeon cell class exists. Runtime
`0xA?` room-helper state fires before the next dungeon key just like room
triggers while keeping its low-nibble arena slot; reloaded cleared room triggers
demote to navigable `0xA?` room-helper variants per the public dungeon-mode spec.
Public `0xE?` dungeon cells are visual wall/door silhouettes, not interactive
doors or walkable floor, while `0xF?` cells are walkable room triggers and
`0xA?` is visit-local room-helper state. No sidecar can redefine those packed
classes. Stepping into public
sleep/poison/fire/electric field cells now applies party status or deterministic
damage; generic `0x84..0x8F` energy-field contact has no status/damage effect,
and the secondary `0x9?` visual family remains descriptive only. Looking at
public fountain cells prompts for a drink; inline
responses like `lY`, `lN`, or `l2Y` apply the cure/heal/poison/bad-taste
subtypes to the selected party member without spending a turn. `T`alk reports
the stock no-response line and world/vehicle command
letters are routed as dungeon refusals before they can trigger overworld
handlers. `I`gnite consumes a torch and starts or extends the torch counter with
deterministic clean timing.
Dungeon command letters whose full systems are outside this slice no longer fall
through to vi diagonal movement fallbacks. Bare `C` opens a spell-name prompt
that accepts compact selector letters, ignores `J`/`O`, supports backspace and
Escape/empty cancellation, and dispatches through the same spell resource and
scene gates as inline `C1...` casts. Spells that need a follow-up direction,
party member, combat slot, or Gate Travel moon phase now prompt for that choice
before any spell charge or mana is spent. Bare `U` opens the Use picker, bare
`R` opens the Ready picker with carried-stock, ammunition, strength, occupied
slot, hand-occupancy, ring-vanish, and combat body-armour gates. The shared item
picker accepts Enter or Space, consumes native vertical/corner navigation codes,
uses its mode-specific Escape literal, and closes immediately after a magic-ring
vanish. U-Use commits one ordinary exploration turn for success, refusal, an
empty picker, or cancellation; moving within the picker remains free. Bare `M`
opens the reagent mixer, bare `N` opens the New Order party-slot
prompt, and bare `Y` opens the free-text yell prompt.
Combat `U` uses that same picker after the live-actor gate; finishing or
cancelling it ends the acting combatant's action, as closing combat Ready or
Z-stats does. Once combat accepts `C`, cancelling or submitting a blank
spell-name prompt likewise ends that combatant's action and runs the committed
action maintenance tail; cancelling a field target cursor also keeps the
already-spent charge and mana before the round resumes. Combat Push and the
Get/Jimmy/Open/Search direction prompts follow the same accepted-verb rule:
choosing a direction or cancelling the prompt commits the action and ages the
active effect, while an actually blocked Klimb remains a free re-prompt.
Bare `J` opens the Jimmy party-member picker when the target requires a picker, and inline forms such as `J1`
still route in one command. The command follows the public lockpick rules:
non-dungeon doors compare the selected member's Dexterity strictly against a
uniform `0..=29` roll, while object and dungeon chests use their published
unsigned-word threshold formulas and a strict `1..=30` comparison. Failed
attempts, already-unlocked object containers, and magic locks break one key as
specified; ordinary NPCs are not Jimmy targets. Native stocks (`0x84`) and
manacles (`0x85`) release a linked prisoner through the same flat Dexterity roll,
clear its dialogue field, set all three schedule modes to 5, award the fixed
thanks/+2 moral result once, and set the scene's save-backed removal bit when the
NPC class is eligible. The two 32-scene NPC mask banks at `0x05B4` and `0x0634`
persist removals and name-known dialogue flags. Clean town-lock rows remain
available as coordinate-bound overrides. Dungeon door sidecars are ignored
because public dungeon cells carry the room-trigger,
room-helper, chest, passage, and visual-wall semantics directly. Numeric
diagonals still refuse as unsupported dungeon movement, and
dungeon `Q` routes to the public mode-loop `Exit to DOS?` prompt instead of
the resident save writer.
Top-down uppercase `L` opens the Look direction prompt, while inline forms such
as `L6` and lowercase quick-look continue to route in one command without
turning the party or spending a turn.
Unhandled dungeon keys run the public sleep/idle polling path as a no-turn
`Zzzzzz...` visual tick instead of using the top-down generic unhandled-command
message.
`V`iew consumes a gem and opens a modal top-down map: a 32-by-32 town/world
class overlay whose 128-by-128 raster starts at absolute screen pixel `(32,32)`,
or a centered dungeon flood map that wraps the 8-by-8 level and stops expansion
at wall-like cells while exact dungeon glyph/floodability edge cases remain out
of scope. The terminal renderer reads the overlay's diagnostic class map
directly; the graphical renderer never copies that debug text into the message
panel.
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
state, while still accepting the legacy visual tile ids used by older debug
hooks. Balloon support is currently a semantic debug transport only: it
follows wind direction over terrain, overflies clean damaging-terrain/waterfall
sidecar effects, and can X-it only when the current cell is not mountain or
wall-like; B-Board remains intentionally unpromoted for balloons.
Furled ships use manual water movement; hoisted sails use the harness wind
state, where calm/perpendicular wind stalls and same-axis wind advances on a
deterministic clean cadence. After a stalled sail attempt, Pass
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
consumed turns. Protection deliberately has no mechanical consumer, matching
the published original-game defect. Combat consumes Quickness in the automatic
actor driver, Mass Charm in AI target remapping, and a live Negate Magic
code/duration pair in the pre-resource cast-absorption gate. Combat C-Cast also
implements the published interference lifecycle: ordinary automatic adjacent
attacks, including misses, save their attacker slot for the victim; a later Cast
revalidates that source as live, hostile, visible, awake, adjacent, and not
suppressed by Negate Time before printing `<actor> interferes!` and re-prompting
without spending the action. A completed victim action clears only that slot;
the 32-byte map persists across rounds, encounters, combat exits, and save/load.
`C1IW` casts the
narrow overworld Locate hook, reporting the current plane, coordinate, facing,
wind, and time
after the saved charge/MP/level gates succeed. `C1IMX` casts the narrow Create
Food hook, adding the latest public tiny PRNG grant (`0..=2`) to the save-backed
food counter after the saved charge/MP/level gates succeed and clamping at the
shared 9999 party food cap.
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
In combat, the same four field spells use a target-cell cursor/coordinate
instead of the dungeon direction prompt; inline smoke can use coordinates such
as `C1FGI4,3`, and prompt Escape cancels after charge and mana are spent but
before placing a marker. Completed player and automatic actor dispatches then
run one common contact hook against the acting descriptor. Exact arena terrain
byte `0x04` acts as Poison, while `0x8F` and `0xBC` act as Fire; no other terrain
byte has a contact arm. A recognized terrain byte takes priority and suppresses
the marker scan even when Poison is rejected by its linked-tile gate. Otherwise,
the actor's linked renderer record is skipped and the first separate colocated
Poison/Sleep/Fire marker wins; the marker remains. Poison and Fire use direct
conditional `0..20` and `0..10` raw-damage draws; Energy instead blocks both
player and AI movement and has no contact payload. Doom absorption is a separate
earlier committed non-digit player-action check: a live actor on arena row 2
accepts renderer companion-band byte `0x3C..0x3F` one cell north, consumes no
PRNG, and arms the endgame handoff. Digit selection, parser refusals, and
automatic actor dispatch bypass Doom absorption. Blocked-direction re-prompts do
not reach either player-action hook.
`C1AG6` casts Dispel Field from party slot 1, spending the saved charge,
MP, and level gates before clearing a public dungeon field target back to
passage while preserving the visit marker bit.
`C1IP6` casts Blink from party slot 1. Outside combat it uses the public
cardinal direction prompt and lands on the farthest grass cell along the
window-bounded ray, while `C1IP<space>` spends the shared cast resources and
passes without success/failure narration. In combat, Blink uses a target-cell
picker/coordinate instead of the non-combat ray.
`C1AEP` and `C1EIP` cast narrow indoor Magic Lock and
Unlock Magic hooks from party slot 1, rewriting facing magic-lock rows supplied
by the clean `town_locks.tsv` sidecar. `C1IQW` casts the narrow Peer hook in
dungeon, indoor, or overworld mode, spending spell resources for the same
clean modal map overlay as gem view without requiring or consuming a
gem.
`C1AWY` casts the narrow X-Ray hook in indoor or overworld mode, using the
same clean modal surface map overlay after the saved charge/MP/level
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
same shrine meditation state machine against the public ordained and Codex
quest masks while the exact persistent standing-byte layout remains open.
`UT`/`UI` use a torch, `UG`/`UV` use a gem, and `UK`/`UJ` use a key through the
shared Use command wrapper; key use reuses the same sidecar-backed town lock
and dungeon heavy-door path as `J`. `U1` through `U8` bury the
corresponding Moonstone phase at the current non-dungeon location when the
underfoot tile matches the public Moonstone bury set (`4..10`, `44`, or `45`).
Surface and town `S`earch can also surface a saved Moonstone phase as a
clean strange-rock pickup, and `G`et clears that pickup while invalidating the
associated Gate Travel slot.
Save export still preserves existing non-calm wind bytes rather than inventing
an unverified byte mapping.
`N<from><to>` swaps two one-based runtime party positions and consumes a turn
per `commands.md` §6, for example `N23` swaps the second and third travelling
members. Slot one is the leader and refuses to move; selecting the same nonzero
slot twice is accepted as a turn-consuming no-op. The swap affects later
party-position prompts such as `C2...` casts and runtime damage checks, and
save export writes the reordered active records back to the front roster slots.
In overworld ship mode, bare `F`/`f` opens the fire direction prompt and an
inline direction (for example `f4`) fires a clean broadside:
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
save-and-continue snapshot to `SAVED.GAM` and `SAVED.OOL`, and
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
clean-return-checked grid-boundary location exits, clean-room sidecar-backed town and
overworld Get plus town Push, trap doors, Hole-up rest, and a
public #43 Look-special path for top-down fountains, scene-gated wishing wells,
death-vision active objects, and sign/poster active-object classes, plus a clean-room
clean dungeon text view with public-spec movement and ladder
transitions plus public pit, bomb trap, and typed energy-field status/damage
reactions, fountain drink effects, underfoot chest opening, torch/light blackout,
gem-backed map views, sidecar-backed scripted teleports and heavy-door opening,
and room-trigger arena diagnostics.
NPC schedules link active-object slots and advance only in town-family scenes;
off-floor schedule changes detach or attach visible slots by zeroing and
first-empty allocation while exact stair subtype routing can be supplied through
clean sidecar metadata. Overworld and dungeon turns leave any stale or synthetic
schedule state inert. The player remains active-object slot zero and has no NPC
descriptor. A town that hosts a living Shadowlord instead installs an ordinary
stationary `0xFC` actor at the highest free NPC index (overwriting index 31 when
the roster is full) and links it through the normal active-object allocator.
The row-4 entry guard and shared one-at-a-time Shadowlord gate are enforced,
and the resident's day-seeded farmland/orchard blight runs before the shared
PRNG stream is replaced with the published host-clock seed. The player may
separately Yell any Shadowlord name in any keep; that transient summon uses its
own active-object identity handshake and does not create an NPC descriptor.
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
cleanup. Non-party combat sleep uses the public own-turn 1-in-17 wake check and
keeps disabled actors present and occupying their cells while wake-check turns
are spent.
Monster possess/blink/summon hooks use the published lazy cascade: exact
`0..=31` acceptance on independent `0..=255` blink and summon gates, fresh
summon X then Y draws in `0..=15`, one placement attempt, immediate turn
consumption on success, and ordinary-AI continuation after summon failure.
Shared combat resistance uses party-owner Intelligence or monster-class
endurance, the signed unclamped score, and one skewed roll; Tremor and Poison
Wind instead compare that roll directly with the target's combat weight.
Kill excludes Blackthorn, Lord British, and Shadow Lord targets. Cause Fear
and Repel Undead share the exact one-HP fleeing transition, with Repel narrowed
to the two published undead classes and producing neither death nor XP credit.
Conjure uses the published sixteen-outcome weighting; Conjure, Swarm, and
Summon share whole-candidate `0..=15` arena probes; Swarm places four actors at
one accepted cell; and Summon's controlled bit is stamped only after a
successful party-caster self-check, never on Oops or monster-AI summons.
Public issue `#132` further pins protected Kill rejection after the shared
charge/7-MP and pre-effect envelope: it consumes no resistance PRNG, leaves the
target untouched, reports `Failed!`, and commits the combat action.
Combat-frame exits restore the pre-combat active-object table and
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
deterministic clean fall damage only to conscious party members.
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
active-object decoders and the save writer keep empty non-player
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

Dungeon level edges use the native uniform exit contract. An up ladder on level
zero exits to Britannia; a down ladder on level seven exits to the Underworld.
Both place the party at that dungeon's published outdoor entrance coordinate.
The withdrawn `dungeon_deeper_transitions.tsv` mechanism is not read.

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

Dungeon heavy-door silhouettes are now native packed-cell behavior rather than
sidecar metadata. Public dungeon-mode rules classify `0xE?` cells as
non-walkable visual wall/door variants, `0xF?` cells as walkable room triggers,
and `0xA?` cells as visit-local room-helper state. `O` and `J` act only on
underfoot chest classes (`0x4?` and already-open `0x7?` variants); they do not
rewrite `0xE?`, `0xF?`, or `0xA?` cells.

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
CASTLE:0 0 12 4 185 184
CASTLE:0 0 14 4 151 184 MAGIC
```

Matching `town_locks.tsv` rows make `O` refuse the locked cell without spending
a turn. `J` with keys rewrites ordinary locked cells to `UNLOCKED_TILE`, marks
the visit-local map dirty, and commits one indoor turn; `MAGIC` rows skip the
member prompt and roll, break one key, and commit the turn. Missing rows use the
native dispersed door identifiers (`0x97`, `0x98`, `0xB8..=0xBB`).

Non-combat Blink (`In Por`) follows the public `cleak/u5-spec#48` rule. After
the normal spell gates spend charge and mana, the spell prompts for a cardinal
direction, scans that loaded 32-by-32 window ray to the map edge, ignores
non-grass terrain as non-blocking, and lands on the farthest grass tile (`0x05`)
on that ray. Pass at the direction prompt consumes the already-spent resources
without moving or printing success/failure narration. There is no
`blink_targets.tsv` sidecar path.

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
clean full-hull auxiliary value and the supplied skiff count, or
`skiff:x,y` to place a standalone skiff.

Overworld fixed-location entry uses the public stock location table built into
the engine. `world_locations.tsv` is an optional clean override/extension for
focused tests and newly published rows:

```text
# PLANE X Y TARGET [TOWN_ENTRY_Y] [TILE] [NARRATION_CLASS]
BRITANNIA 0 0 CASTLE:0 7 0x15 CASTLE
UNDERWORLD 0 0 DUNGEON:0 0x18 DUNGEON
```

For town-family targets, the optional fifth column is the clean
`LocationEntryYTable` value; X is fixed at 15 and floor is 0. A town row can use
an optional sixth stock-tile annotation after that entry Y, while a dungeon row
can use an optional fifth stock-tile annotation. E-Enter always classifies the
live tile instead of comparing that annotation. The final narration class is
required for a custom row to be enterable; omitting it leaves the row usable for
returns but makes E-Enter answer `What?` without inferring presentation from the
target key. Classes are `HUT`, `KEEP`, `VILLAGE`, `TOWNE`, `CASTLE`,
`LIGHTHOUSE`, `LORD_BRITISH`, `BLACKTHORN`, `CAVE`, `MINE`, and `DUNGEON`.
Underworld town-family rows are permitted for clean extensions such as the
published Ararat row. Each target may appear only once so exits can resolve a
single unambiguous return coordinate.
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
kept as runtime-only state until its exact save layout is published.

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
Britannia-to-Underworld falls also apply deterministic clean fall
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
carpet travel can enter it and take deterministic clean lava damage,
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
clean damage to living party members while balloon overflight does not.
A matching `DROWNING`/`WATER` row marks an authored water/current cell as
enterable by foot and water/air/carpet transports, blocks horses, and applies
deterministic clean damage only to foot travel. Explicit world debug
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

That saved-slot path is the only moongate system the engine has. The former
`moongates.tsv` sidecar - authored origin/destination rows from before the
natural-gate coordinates were published - has been removed; see
`docs/sidecars.md`.

Secret-door search metadata can be supplied as a clean-room sidecar while the
public dungeon low-nibble and town object-table encodings remain open. Place
rows next to the game data as `secret_doors.tsv`:

```text
# TOWN SCENE FLOOR X Y REVEAL_TILE [TILE]
TOWN CASTLE:0 0 12 4 184 24
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
hit, or rewrites one of the native dispersed door tiles to cobble `0x44`
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

The three native Eternal Flame coordinates are built in from the public spec.
`eternal_flames.tsv` remains an optional clean override/extension for focused
tests:

```text
# TARGET FLOOR X Y FLAME [TILE]
BRITANNIA 0 5 5 TRUTH 16
UNDERWORLD -1 20 40 LOVE
CASTLE:0 0 12 10 COURAGE 80
```

`U`se on a carried Shadowlord shard checks the public shard preconditions, then
matches the party's target/floor/coordinate against the native flame table plus
any `eternal_flames.tsv` override rows. The flame must be the opposed principle
for that shard, and optional sidecar tile guards check the current map tile. On
success the shard is consumed, the matching Shadowlord slot is marked
vanquished, matching active Shadowlord encounter objects on the current floor
are cleared, and one turn is consumed. Missing rows, stale tile guards, or the
wrong flame keep the existing no-effect branch without consuming the shard.

Push now recognizes the public static movable families directly in world,
town, and combat grids, including their floor stamps, push/pull rules, exact
refusals, and facing rewrites. `town_pushables.tsv` remains a clean extension
for an authored non-stock town fixture:

```text
# SCENE FLOOR X Y [TILE]
CASTLE:0 0 12 6 44
```

Native static families do not need a row. Dynamic active objects never move;
they take the emphatic `Won't budge!` refusal. A sidecar match retains the
legacy clean-extension swap for non-stock fixtures. Every completed Push in
world/town consumes the ordinary action, including source misses and blocked
attempts; Escape alone leaves the `Push-` prompt open.

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
per hour. Town bed rest follows the latest public `cleak/u5-spec#47` guidance:
ordinary rest advances time without a separate direct HP/MP recovery grant.
Encounter
interruption is owned by the overworld/dungeon rest-with-watch path rather than
town beds.
In overworld and dungeon modes the same `h8` input runs the rest-with-watch
path: each rested hour performs three 20-minute cleanup ticks,
including time, light counters, animation, and existing area-specific per-turn
hooks such as authored overworld damage tiles. A supplied watcher must be a
living Good-status party member; invalid watcher choices leave no watch set.
The sleep-ambush predicate follows the public one-in-sixty-four rest/camp rule
and hands the selected ambush monster to the combat frame when it fires. The
watch path wakes members who were asleep when rest began, but ordinary rest has
no separate direct HP/MP recovery grant.

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

Stepping onto a native town tile `0x04` with foot transport runs the public
`cleak/u5-spec#51` poison-gas branch against non-poisoned party slots with the
latest public `0..=29` per-slot roll compared against each member's Dexterity.
Older coordinate and tile-attribute poison-gas sidecars are kept only for
compatibility with older clean saves/tests and no longer trigger the native
branch.

Town exits are prompted only when a cardinal step would leave the 32-by-32
interior grid. The attempted step is not committed. Accepting returns to the
saved world snapshot, or to the matching `world_locations.tsv` row when no
snapshot is available; refusing or cancelling leaves town mode active. Tile
`0x59` is the telescope Look trigger and never participates in exit handling.
The former `town_exit_tiles.tsv` compatibility sidecar is retired and ignored.
Per clean specification issue `cleak/u5-spec#110`, every edge samples loaded
floor cell `(31,31)` through the active transport terrain predicate and checks
occupancy at the true out-of-grid candidate coordinate. A terrain rejection,
`N`, or Escape consumes one normal town turn; `Y` exits without a town turn.

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

Town and overworld movement use the public `systems/movement.md` static tile
sets for foot, horse, carpet, ship, and facing-sensitive skiff travel when no
override is present. Focused tests can still provide the clean-room
`tile_passability.bin` sidecar next to the game data. Tile id `n` uses byte
`n >> 3` and mask `0x80 >> (n & 7)`; a set bit means broadly passable for the
legacy base-predicate override.

For focused transition plumbing tests, use `--debug-enter` with an overworld
scene. Press `e` to enter the requested
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
count as provision consumers and take the public one-HP poison tick on each
hourly status/provision pass. If food is already zero at an hour crossing, the
starvation branch appends a warning and rolls the public `1..=8` damage
independently for each non-dead party slot. The harness does not require character
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
year/month/day/hour/minute clock. It restores the shared active-effect code and
duration; `Q` applies half-time and `T` suppresses minute/light cleanup, while
unknown nonzero codes are preserved without inventing behavior. Fixed hidden-
treasure state and the three Shadowlord hideout slots
now round-trip through their public `SAVED.GAM` bytes, with `SAVED.WPS` retained
only as a compatibility mirror for older clean saves. Ship state uses the exact
public marker table: `0x20..0x23` are hoisted frigates, `0x24..0x27` are furled
frigates, and each run's low two bits encode north/east/south/west. `--from-init` keeps the
factory bootstrap path by reading `INIT.GAM` plus
the surface `INIT.OOL` overlay seed, so fresh bootstrap does not depend on stale
`SAVED.OOL` surface objects.

On `--from-save`, the loader also validates canonical `SAVED.OOL` and refreshes
the per-plane `BRIT.OOL` and `UNDER.OOL` mirrors from its two halves, matching
the public load-time mirror contract before gameplay starts.

During top-down play, uppercase `Q` opens the public save-and-continue prompt,
with inline confirmation shortcuts still accepted: enter `QY` to write
`SAVED.GAM` and canonical `SAVED.OOL`, or `QN` to cancel. Save staging reads
`UNDER.OOL` first and `BRIT.OOL` second, assembles the Britannia/Underworld
`SAVED.OOL` halves, never writes `BRIT.OOL`, and writes the unchanged
`UNDER.OOL` bytes only when the entry required-disk role was not Britannia.
Queued shipwright delivery lives in its published `SAVED.GAM` X/Y/class bytes,
not either overlay. The save writer intentionally leaves `INIT.GAM`, `INIT.OOL`,
and static map assets alone; dungeon-mode saves write the current 512-byte
dungeon working buffer into the save image. After `--from-init`, unresolved
`SAVED.GAM` bytes are templated from `INIT.GAM` instead of any stale saved game.

Use `--from-init` to seed the same harness from `INIT.GAM` without running
character creation:

```powershell
cargo run -- --play --from-init C:\Games\U5-Clean
```
