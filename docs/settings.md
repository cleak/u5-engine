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
<!-- END GENERATED -->
