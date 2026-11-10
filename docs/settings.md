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
In this build Recommended still gives the Classic game, and Everything adds
only sharp scaling (`display.scale_mode = "sharp"`) and the F11 fullscreen
key (`display.fullscreen_key`), neither of which changes the game.

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
  matrix.
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
| display.grow_left | Wider world view (shows more than the original) | bool = false | a, F | deviation, wider view | --grow-left | - | later (chapter 2 P8) |
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
| display.grow_left_light | Daylight reach: whole screen, or original distance | enum scaled\|faithful = scaled | a, F | feature | - | - | later (chapter 2 P8) |
| display.crawl | Messages over the map | bool = true | a, F | feature | - | - | later (chapter 2 P8) |
| display.typed_colour | Typed text in cyan | bool = true | a, F | feature | - | - | later (chapter 2 P8) |
| display.ribbon_hp_colours | Health colours (red when low, green when poisoned) | bool = true | a, F | feature | - | - | later (chapter 2 P8) |
| display.dither | Fade effects | bool = true | a, F | feature | - | - | later (chapter 2 P8) |
| display.centre_party | Keep the party centred when the side column is open | bool = false | a, F | feature | - | - | later (chapter 2 P8) |
| display.panel_priority | Which side panel wins when two want one side | list of journal\|conversation = [journal, conversation] | a, F | feature | - | - | later (chapter 1 P6) |
| display.panel_guard | Smallest game picture a side panel may leave, in percent | int 50..100 = 80 | a, F | feature | - | - | later (chapter 1 P6) |
| display.vsync | Wait for the screen's refresh (vsync) | bool = true | a, F | feature | --vsync | - | yes |
| display.raster | Tile graphics | enum ega\|tandy = ega | a, L | feature | --raster-depth | - | yes |
<!-- END GENERATED -->
