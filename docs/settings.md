# Enhancement Settings

Every optional enhancement is its own setting, and every setting is off by
default. With every setting at its default ("Classic") the game is the 1.0.1
game. This page lists every setting key the engine knows. The table is
generated from the engine's catalogue, and a test keeps the two equal.

## Where settings come from

From lowest to highest precedence:

1. the built-in defaults (Classic);
2. `[enhancements] preset` in `config.toml`;
3. the other keys written in `config.toml`;
4. `--preset NAME` on the command line;
5. `--enable KEY`, `--disable KEY` and `--set KEY=VALUE`, in command-line
   order;
6. the parity lock, which turns everything off.

A preset sets the keys the presets name and nothing else, so preferences
such as a volume level keep their value when the preset changes. When the
result matches a preset exactly, that preset's name is shown; otherwise the
settings are "custom", based on the preset they started from.

Problems never stop the game. An unknown key, a bad value, a cheat key
without `cheats.enabled`, or a setting this device cannot use is ignored
or put back to its default, with a one-line note in the log.

A key marked "later" is not available in this build: it is not listed on any
settings page, a stored value is kept but not used, and presets skip it.

## Presets

- **Classic** (the default): every setting at its default. The game is the
  1.0.1 game.
- **Recommended**: a short list of helpers.
- **Everything**: every feature-tier setting, and Recommended's list.

No preset turns on a cheat, and the only deviation a preset sets is
`combat.fast_forward`. A preset sets only what is available in this build.
In this build Recommended turns on only sprites over terrain
(`display.sprite_composite`), and Everything adds sharp scaling
(`display.scale_mode = "sharp"`) and the F11 fullscreen key
(`display.fullscreen_key`); none of them changes the game. The wider world
view is a deviation, so no preset turns it on. Everything also turns on the
enhanced gem map (`gem_map.enhanced`). No preset sets brightness, map
brightness or the sprite outline (`display.composite_halo`): they are
preferences.

## Changing settings

- `config.toml`, in the per-user config folder (`U5_ENGINE_CONFIG` names
  another file). `[enhancements]` holds `preset`; each other section holds
  its own keys, as `name = value`.
- The setup window's Enhancements page: `u5-engine --settings`.
- The command line, for one launch:

| Option | What it does |
|---|---|
| `--preset NAME` | Start from `classic`, `recommended` or `everything`. |
| `--enable KEY[,KEY...]` | Turn on/off settings on, e.g. `section.name`. |
| `--disable KEY[,KEY...]` | Turn on/off settings off. |
| `--set KEY=VALUE` | Set any setting. |
| `--parity-lock` | Force every enhancement off for this launch. |
| `--settings` | Open the Enhancements page in the setup window, then start the game. |
| `--show-settings` | Print the settings a launch would use, then exit. |
| `--scale-mode MODE` | `--set display.scale_mode=MODE`: `fit`, `sharp`, `integer-square`, `integer-hybrid` or `stretch`. |
| `--pixel-aspect ASPECT` | `--set display.pixel_aspect=ASPECT`: `dos` (tall, as the original screen) or `square`. |
| `--fullscreen`, `--no-fullscreen` | `--enable` / `--disable display.fullscreen`: start in borderless fullscreen. |
| `--fullscreen-key`, `--no-fullscreen-key` | `--enable` / `--disable display.fullscreen_key`: F11 switches fullscreen. |
| `--vsync`, `--no-vsync` | `--enable` / `--disable display.vsync`: wait for the screen's refresh. |
| `--pillar-fill FILL` | `--set display.pillar_fill=FILL`: `black`, `solid`, `ribbon` or `bezel`. |
| `--bezel-image PATH` | `--set display.bezel_image=PATH`: a PNG for the `bezel` fill. |
| `--grow-left`, `--no-grow-left` | `--enable` / `--disable display.grow_left`: the wider world view (shows more than the original). |
| `--sprite-composite`, `--no-sprite-composite` | `--enable` / `--disable display.sprite_composite`: draw people, monsters, ships and items over the terrain instead of in black boxes. `display.composite_halo` (on by default) keeps a thin black outline round each sprite. |
| `--brightness PERCENT` | `--set display.brightness=PERCENT`: dim the map view, 25 to 100 (100 is the original colours; `--enable display.brightness` sets the suggested 70). `display.brightness_scope` chooses the map view (`map`) or the whole screen (`frame`). |
| `--gem-map`, `--no-gem-map` | `--enable` / `--disable gem_map.enhanced`: using a gem shows a full-screen map with real tiles, wider than the original gem's. |

`config.toml` applies to the default launch (no game folder or mode on the
command line); other launches use the command line alone. An unknown key on
the command line is an error, as for any unknown option.

## Scaling and fullscreen

- `display.scale_mode`: `fit` (the default) fills the window as the 1.0.1
  game does. `sharp` keeps the same shape but scales the picture up by whole
  steps first and smooths only the last fraction, so pixels stay crisp at
  any size. `integer-square` and `integer-hybrid` use whole steps only and
  leave a border; `stretch` fills the whole window.
- `display.pixel_aspect`: `dos` draws each pixel a little taller than wide,
  as the original screen did; `square` draws square pixels.
- `display.fullscreen` starts in borderless fullscreen. With
  `display.fullscreen_key` on, F11 switches fullscreen on and off for the
  session without changing the setting. F11 is a key the game never reads,
  and Alt+Enter is still the game's Enter.

## The bars beside the picture

A window wider than the 4:3 picture leaves bars at its sides, black as in
the 1.0.1 game. `display.pillar_fill` can fill them; the game itself never
changes:

- `black` (the default): plain black bars.
- `solid`: the colour in `display.pillar_color`, as `#rrggbb`.
- `ribbon`: the screen's own blue ribbon, traced round each bar and drawn
  from your game's `IBM.CH` font.
- `bezel`: a PNG picture of your own, named by `display.bezel_image`. It is
  scaled to cover the window and shows through the bars. A relative path is
  taken from the folder `config.toml` is in. A picture that is missing, not
  a PNG, or larger than 8192 pixels on a side leaves the bars black, with a
  warning.

## The wider world view

`display.grow_left`, **"Wider world view (shows more than the original)"**,
is off by default and in no preset, because it shows more of the world than
the original screen did: every setting that does is marked **wider view**.
Turn it on with `--grow-left`, `--enable display.grow_left`, the
Enhancements page, or `grow_left = true` under `[display]` in `config.toml`.

On a window wider than 4:3 the screen grows to the left instead of leaving
black bars. The original 11x11 view stays exactly as the game draws it, in
the middle of a wider map:

- **The map** shows the cells round the original view that the party can
  see from where it stands, with the same light rules. Nothing outside the
  party's sight is drawn: it stays black, as an unseen cell in the original
  view does. What the wider map shows is never counted as seen by any other
  feature.
- **Messages over the map.** The message window's lines are written over
  the bottom of the map in outlined letters and fade from white to grey to
  dark grey as newer lines arrive. Only what the original message window
  has already shown is written there, and a keyword prompt is never kept
  once it is answered.
- **The top band** carries the party, food, gold and the date, in the
  original's order and spacing, with the sky tab; the wind tab sits on the
  bottom band below it.
- **The side column.** Ctrl+P docks the original's right-hand column (the
  party, the counters and a taller message window that also shows older
  lines) on the left, and Ctrl+P again puts it away. Ctrl+P is no key the
  game reads. Screens that need the column, such as a character page, show
  it by themselves and give it back when they close.
- Fixed screens (combat, dungeons, the intro, shrines and other full
  pictures) are shown as the original draws them, with the bars beside
  them.

Each part is its own setting, listed under the wider world view on the
Enhancements page, and works with any mix of the others:

| Setting | Label | Default | Off means |
|---|---|---|---|
| `display.grow_left_light` | Daylight reach: whole screen, or original distance | `scaled` | `faithful` keeps the original's light distance |
| `display.crawl` | Messages over the map | on | only the line being typed is shown; the column starts open |
| `display.typed_colour` | Typed text in cyan | on | typed text in the game's colours |
| `display.ribbon_hp_colours` | Health colours (red when low, green when poisoned) | on | health in white |
| `display.dither` | Fade effects | on | the column and tabs switch in one frame |
| `display.centre_party` | Keep the party centred when the side column is open | off | the map never moves |

Nothing here changes the game. The wider view only reads what the game has
already drawn and decided: with it on, the game's state, random numbers,
timing and save files are exactly those of the same play with it off
(see [How a setting is checked](#how-a-setting-is-checked)).

## Brightness and sprites

Two settings change how the map looks and nothing else. Neither shows
anything the original screen does not show the player.

**Sprites over terrain (no black boxes).** `display.sprite_composite`,
**"Sprites over terrain (no black boxes)"** (`--sprite-composite`), off by
default and part of the Recommended and Everything presets. The original
draws every person, monster, ship and item on a black square. With this
on, the black round the sprite is replaced by the ground of its cell, so a
skeleton stands on the grass and a ship floats on moving water:

- the ground drawn is the tile that cell shows when nothing stands on it
  (the arena's ground in combat), so it never shows anything the empty
  cell would not;
- `display.composite_halo`, **"Keep a thin black outline around sprites"**,
  on by default, keeps the black pixels that touch the sprite, so it keeps
  the original's outlined look on busy ground. Turn it off for no outline;
- black inside a sprite stays black, and only pixels that are still black
  in the finished picture are replaced: the combat cursor, the aim marker
  and the camp and potion flashes look as they always did;
- some tiles keep their box: the shark (its water is painted in), the
  dotted creatures, creatures drawn as eyes or dots in the dark, people in
  a bed, on a chair or in the stocks, combat fields such as fire and poison,
  and the first-person dungeon view, which already draws its own sprites;
- under the wider world view, objects in the outer ring are drawn the same
  way as in the original view.

**Map view brightness.** `display.brightness`, **"Map view brightness
(100% is the original colours)"** (`--brightness PERCENT`), 25 to 100,
where 100 (the default) is off and gives the original colours. It scales
the light of each colour, not the colour numbers, so the EGA greys stay
even and black stays black at every level:

| Brightness | Dark grey | Light grey | White |
|---|---|---|---|
| 100% (off) | 85 | 170 | 255 |
| 85% | 79 | 158 | 237 |
| 70% (suggested) | 71 | 145 | 218 |
| 55% | 63 | 129 | 196 |
| 40% | 54 | 112 | 170 |
| 25% | 42 | 89 | 137 |

70% brings back most of the darker feel the game had on an old monitor;
`--enable display.brightness` sets it, and `--disable display.brightness`
sets 100. No preset sets brightness: it is a preference, like a volume.
`display.brightness_scope`, **"Brightness applies to: the map view, or the
whole screen"**, is `map` by default: only the map (and the whole map of
the wider world view) is dimmed, while the frame, the party panel, the
messages, the title screens and character creation keep their exact
colours. `frame` dims the whole picture, title screens included. Menus,
the journal and other separate panels are never dimmed. Screenshots and
the QA frame captures always record the original picture.

## The gem map

`gem_map.enhanced`, **"Enhanced gem map"** (`--gem-map`), off by default and
part of the Everything preset. Using a gem works exactly as in the original:
`V` spends one gem, prints the same lines and opens the game's view, and the
next key closes it and is thrown away. With this on, the view is drawn as a
map over the whole window instead of the original's small square:

- **Outdoors** every cell is the real tile from your game's `TILES.16`,
  shrunk to 4 pixels, with the party in the middle: 128 by 60 cells at
  16:9, 172 by 60 at 21:9 and 80 by 50 at 4:3, where the original gem shows
  32 by 32. The map shows the ground only: no people, monsters or other
  objects are drawn on it, and the party is a steady yellow box around its
  own picture.
- **In a town** the whole floor fills the height of the screen, 7 pixels a
  cell (6 on a 200- or 216-line screen), with the ground only: no townsfolk,
  as the original gem shows them. The thin strips above and below are the
  ground the town has past its edge.
- **The land beside a town.** `gem_map.town_surroundings`, **"Show the land
  around a town"**, on by default: the Britannia land around the town's
  entrance fills the sides, cut open at the entrance, west on the left and
  east on the right, darker than the town. Only land you have already seen
  from outside is drawn; the rest stays black, so the map never shows
  ground you have not been near. Off, the sides are black.
- **In a dungeon** the levels are drawn side by side with the original
  gem's symbols, the one you are on in the middle: three levels at 16:9,
  five at 21:9 and one at 4:3. Your own level shows what the gem lights up
  plus what you have already seen of it, dimmer; the other levels show only
  what you have seen. A level you have no sign of stays black, and a level
  you only know a ladder leads to is a black panel. A yellow box joins a
  ladder (or a pit you have found) to where it lands, only when you have
  seen both ends. You are the yellow arrow.
  - `gem_map.dungeon_stack`, **"Show every dungeon level you have
    seen"**, on by default. Off, your own level is shown alone, larger.
  - `gem_map.wrap_rows`, **"Show the dungeon's wrapped edge"**, on by
    default: above and below each level (and at the sides on 4:3) the
    level repeats, dimmed, as a dungeon level wraps round; a thin grey
    edge marks where it really ends.
  - `gem_map.level_paging`, **"Page through dungeon levels (PgUp,
    PgDn)"**, on by default: when only one level fits (4:3, or the stack
    off), PgUp shows the level above and PgDn the level below, skipping
    levels with nothing to show. These two keys are the only ones the map
    ever takes, and only then, when the game would just close its view;
    any other key closes it as usual. At every other time PgUp and PgDn
    are the game's own diagonal moves. Off, PgUp and PgDn close the view as
    in the original.
  - `gem_map.glyphs`, **"Dungeon symbols"**, `perceived` by default:
    secret doors are drawn as wall and hidden traps as passage until you
    find them. `gem-faithful` draws what the original gem draws, which
    gives secret doors and hidden pits away on sight.
- **Dimming.** `gem_map.map_brightness`, **"Gem map dimming (names stay
  bright)"**, 25 to 100, is 70 by default: the map is drawn at 70% light so
  the party box stands out. `gem_map.brightness_link`, **"Follow map
  brightness"**, off by default, uses `display.map_brightness`, **"Map
  brightness (full map, gem map)"**, instead while that is below 100, so one
  setting dims every map.
- **Fade.** `gem_map.sweep`, **"Fade the gem map in and out"**, on by
  default: the map rises in over the screen with a dithered edge and sweeps
  back out when the view closes, about half a second each way. The fade
  never holds a key: a key pressed while it runs closes the view at once,
  as in the original, and the fade turns round.
- On a window smaller than 320x200 the original view is shown.

Nothing here changes the game: the gem is spent by the game itself, the
original view is still drawn underneath, and the game's state, random
numbers, timing and save files are exactly those of the same play with the
map off. What the map remembers of your discoveries is kept beside the save
(see [sidecars.md](sidecars.md#player-sidecars)).

## The parity lock

`--parity-lock`, or `U5_PARITY_LOCK=1` in the environment, turns every
enhancement off whatever `config.toml` or the command line says, and the
settings file is not read. The test settings `U5_FIXED_HOST_CLOCK` and
`U5_EMULATOR_LOAD_LATENCY_MS` lock it too, and the route smoke and the frame
suites never read settings at all. A locked launch prints one line,
`enhancements: parity-locked (all off)`, which the paired DOSBox harness
requires in every run.

## Hotkeys and files

Each feature's keys exist only while that feature is on; the defaults and
how to change them are in [commands.md](commands.md#enhancement-hotkeys).
Features that remember things per save keep them in one folder beside the
save; see [sidecars.md](sidecars.md#player-sidecars).

## How a setting is checked

A setting that only shows things must leave the game exactly as it was, and
one that types keys must change it exactly as typing those keys would:

- `cargo test` runs the equivalence tests on synthetic games, in the
  terminal and in the windowed shell, including every row of the toggle
  matrix. The display settings have their own matrix in the windowed
  shell: each setting alone, every pair, and the side column opened and
  closed with Ctrl+P every fifth key, over a walk in town and on the
  overworld by day and night, a conversation, a dungeon, the camp flash and
  a potion's sweep, compared frame by frame and on the world ticks and the
  saved files (and on the real game files, with combat and Britain at
  night, when `U5_TEST_ASSET_DIR` names a copy). Sprites over terrain and
  brightness are in that matrix too: each alone, at 70% and 25% and over
  the whole screen, with and without the outline, and each with the wider
  view and with sharp scaling. A fresh game with every display setting on
  is checked to draw no letter the original screen has not shown and no
  map cell out of the party's sight, and with sprites over terrain and
  brightness on, to draw under each sprite exactly the ground its cell
  shows when empty and never to light a pixel the original leaves black.
- `qa/enhancements/matrix.tsv` is the toggle matrix: every key alone, a
  pairwise covering of the toggles and classed values, and everything on
  except the cheats. It is generated from the catalogue, and a test keeps
  the two equal (`U5_BLESS_TOGGLE_MATRIX=1` rewrites it).
- `python3 qa/tools/toggle_matrix.py` runs `u5-engine --equivalence-suite`
  over the matrix on real game files (a fresh `game-dev-u5 prepare-profile`
  copy). Every configuration must equal the all-off run, Classic must match
  the 1.0.1 route-smoke manifest, and the whole matrix must take under ten
  minutes.

## Columns

- **Type = default**: `bool`, `int a..b`, `enum a|b`, `list of a|b`,
  `color`, `path` or `chord`, then the default.
- **Cls**: effect class. `a` only shows things, `b` acts only through keys a
  player could type, `c` changes the game (cheats only), `-` is a setting of
  the foundation itself.
- **When**: when a change made during play takes effect. `F` next frame, `P`
  next command prompt, `L` next launch. File and command-line changes always
  take effect at the next launch.
- **Tier**: `feature` settings may be turned on by a preset. `deviation`
  settings change how the game looks or plays compared with the original and
  are never in a preset. `cheat` settings need `cheats.enabled`.
- **Needs**: what the device must have; without it the setting stays off.

<!-- BEGIN GENERATED: settings table (cargo test -p u5-runtime) -->
| Key | Setting | Type = default | Cls, When | Tier | Alias | Needs | Available |
|---|---|---|---|---|---|---|---|
| enhancements.freeze_overlays | Pause the world while a menu or map is open | bool = false | a, F | deviation | - | - | later (chapter 1 P6b) |
| display.fullscreen | Fullscreen (borderless) | bool = false | a, F | feature | --fullscreen | a window | yes |
| display.fullscreen_key | Switch fullscreen with a key (F11) | bool = false | a, F | feature | --fullscreen-key | a window, a keyboard | yes |
| display.grow_left | Wider world view (shows more than the original) | bool = false | a, F | deviation, wider view | --grow-left | - | yes |
| display.sprite_composite | Sprites over terrain (no black boxes) | bool = false | a, F | feature | --sprite-composite | - | yes |
| gem_map.enhanced | Enhanced gem map | bool = false | a, P | feature | --gem-map | - | yes |
| enhancements.schema | Settings file format version | int 1..1 = 1 | -, L | feature | - | - | yes |
| enhancements.preset | Preset | enum classic\|recommended\|everything = classic | -, P | feature | --preset | - | yes |
| keys.menu | Open the menu | chord = F12 | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.fullscreen | Switch fullscreen | chord = F11 | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.map | Open the map | chord = Tab | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.minimap | Show or hide the minimap | chord = Ctrl+M | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.journal | Open the journal | chord = Ctrl+J | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.column | Show or hide the side column | chord = Ctrl+P | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.auto_member | Auto-combat for this member | chord = Ctrl+A | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.auto_party | Auto-combat for the party | chord = Ctrl+Shift+A | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.auto_until_done | Auto-combat until the fight ends | chord = Ctrl+F | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| keys.cheats | Open the cheats page | chord = Ctrl+F12 | -, F | feature | - | a keyboard | later (chapter 1 P5) |
| display.scale_mode | How the picture is scaled to the window | enum fit\|sharp\|integer-square\|integer-hybrid\|stretch = fit | a, F | feature | --scale-mode | - | yes |
| display.pixel_aspect | Pixel shape: tall like the original screen, or square | enum dos\|square = dos | a, F | feature | --pixel-aspect | - | yes |
| display.pillar_fill | What fills the bars beside the picture | enum black\|solid\|ribbon\|bezel = black | a, F | feature | --pillar-fill | - | yes |
| display.pillar_color | Colour of the bars, for a solid fill | color = #000000 | a, F | feature | - | - | yes |
| display.bezel_image | Picture for the bars, a PNG file | path = "" | a, F | feature | --bezel-image | - | yes |
| display.grow_left_light | Daylight reach: whole screen, or original distance | enum scaled\|faithful = scaled | a, F | feature | - | - | yes |
| display.crawl | Messages over the map | bool = true | a, F | feature | - | - | yes |
| display.typed_colour | Typed text in cyan | bool = true | a, F | feature | - | - | yes |
| display.ribbon_hp_colours | Health colours (red when low, green when poisoned) | bool = true | a, F | feature | - | - | yes |
| display.dither | Fade effects | bool = true | a, F | feature | - | - | yes |
| display.centre_party | Keep the party centred when the side column is open | bool = false | a, F | feature | - | - | yes |
| display.panel_priority | Which side panel wins when two want one side | list of journal\|conversation = [journal, conversation] | a, F | feature | - | - | later (chapter 1 P6) |
| display.panel_guard | Smallest game picture a side panel may leave, in percent | int 50..100 = 80 | a, F | feature | - | - | later (chapter 1 P6) |
| display.vsync | Wait for the screen's refresh (vsync) | bool = true | a, F | feature | --vsync | - | yes |
| display.raster | Tile graphics | enum ega\|tandy = ega | a, L | feature | --raster-depth | - | yes |
| display.brightness | Map view brightness (100% is the original colours) | int 25..100 = 100 | a, F | feature | --brightness | - | yes |
| display.brightness_scope | Brightness applies to: the map view, or the whole screen | enum map\|frame = map | a, F | feature | - | - | yes |
| display.map_brightness | Map brightness (full map, gem map) | int 25..100 = 100 | a, F | feature | - | - | yes |
| display.vignette | Edge darkening | int 0..50 = 0 | a, F | feature | --vignette | - | later (chapter 9 P3) |
| display.composite_halo | Keep a thin black outline around sprites | bool = true | a, F | feature | - | - | yes |
| gem_map.labels | Names on the gem map (only places you have found) | bool = true | a, F | feature | - | - | later (chapter 10 G5) |
| gem_map.brightness_link | Follow map brightness | bool = false | a, F | feature | - | - | yes |
| gem_map.map_brightness | Gem map dimming (names stay bright) | int 25..100 = 70 | a, F | feature | - | - | yes |
| gem_map.sweep | Fade the gem map in and out | bool = true | a, F | feature | - | - | yes |
| gem_map.town_surroundings | Show the land around a town | bool = true | a, F | feature | - | - | yes |
| gem_map.dungeon_stack | Show every dungeon level you have seen | bool = true | a, F | feature | - | - | yes |
| gem_map.wrap_rows | Show the dungeon's wrapped edge | bool = true | a, F | feature | - | - | yes |
| gem_map.level_paging | Page through dungeon levels (PgUp, PgDn) | bool = true | a, F | feature | - | - | yes |
| gem_map.glyphs | Dungeon symbols (gem-faithful is a spoiler: shows secret doors and hidden pits on sight) | enum perceived\|gem-faithful = perceived | a, F | feature (deviation: gem-faithful) | - | - | yes |
<!-- END GENERATED -->
