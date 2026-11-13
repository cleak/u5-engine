# Release Notes

What each release changes for players. The release workflow adds the
download notes and the list of merged changes; paste the section for the
tag above them.

## Unreleased

### Optional enhancements: the foundation

The game plays exactly as 1.0.1 by default, and nothing in this section
changes it. This release lays the groundwork for optional enhancements,
each its own setting and each off unless you turn it on. The first ones,
the display settings below, arrive in this release too.

- **Settings.** Every enhancement setting is listed in
  [settings.md](settings.md), with how to change it: `config.toml`, the setup
  window's Enhancements page (`u5-engine --settings`) or the command line
  (`--preset`, `--enable`, `--disable`, `--set`). `--show-settings` prints
  what a launch would use. Unknown keys and bad values never stop the game:
  the setting keeps its default and the log says why.
- **Presets.** Classic (the default, the original game), Recommended and
  Everything. In this release Recommended turns on only sprites over terrain
  (below), and Everything adds sharp scaling and the F11 fullscreen key.
- **Parity lock.** `--parity-lock` or `U5_PARITY_LOCK=1` turns every
  enhancement off whatever the settings say.
- **Hotkeys** for future features are listed in
  [commands.md](commands.md#enhancement-hotkeys). None is bound while its
  feature is off, and none is a key the game reads.
- **Files beside the save.** Future features that remember things per save
  will keep them in a `.u5-engine-enh` folder next to `SAVED.GAM`
  ([sidecars.md](sidecars.md#player-sidecars)). `SAVED.GAM` itself never
  changes, and with every enhancement off the folder is never created.

Checks behind it: the route-smoke manifest and both visual-suite manifests
are unchanged from 1.0.1, two full paired DOSBox sweeps found no real
difference from it (every non-match was a random draw or capture timing that
1.0.1 shows too), and every enhancement configuration in the toggle matrix
plays exactly as the original.

### Optional enhancements: the display

Each of these is off unless you turn it on, and none changes the game
([settings.md](settings.md)):

- **Scaling and fullscreen.** `display.scale_mode` (`sharp` keeps pixels
  crisp at any size; whole-step and stretch modes too), `display.pixel_aspect`
  (tall DOS pixels or square ones), borderless fullscreen, and F11 to switch
  it (`display.fullscreen_key`).
- **The bars beside the picture** can be a solid colour, the screen's own
  blue ribbon, or a picture of your own (`display.pillar_fill`).
- **The wider world view** (`--grow-left`, marked "wider view" because it
  shows more than the original): on a wide window the map grows to the left
  round the untouched original view, showing only what the party can see;
  messages fade over the map, the party and counters sit in the top band, and
  Ctrl+P docks the original's side column on the left. Its parts (daylight
  reach, messages over the map, cyan typed text, health colours, fade
  effects, keeping the party centred) are separate settings. It never shows a
  letter the original screen has not shown or a cell out of the party's sight
  ([the wider world view](settings.md#the-wider-world-view)).

### Optional enhancements: brightness and sprites

Both are off unless you turn them on (sprites over terrain is part of the
Recommended preset), and neither changes the game: they only change the
picture you see, never what the game decides, its random numbers, its timing
or its save files ([brightness and sprites](settings.md#brightness-and-sprites)).

- **Sprites over terrain (no black boxes)** (`--sprite-composite`). People,
  monsters, the party, ships and items are drawn over the grass, floor or
  water of their cell instead of on a black square. A thin black outline
  stays round each sprite so it still stands out (`display.composite_halo`,
  on by default). Black that is part of a picture, a person in a bed or on a
  chair, a few creatures drawn with black in them, combat fields, and the
  first-person dungeons stay exactly as they were. The ground drawn under a
  sprite is the ground that cell shows when it is empty, so nothing new is
  revealed.
- **Map view brightness** (`--brightness 70`). Dims the map in linear
  light, from 100% (the original colours) down to 25%, so the bright EGA
  colours look closer to the darker feel the game has on an old monitor.
  70% is the suggested level (`--enable display.brightness` picks it); 85%
  is barely visible and 55% starts to look muddy. Only the map is dimmed:
  the frame, the party panel and the messages keep their exact colours,
  unless you choose the whole screen (`display.brightness_scope = "frame"`).
  Black stays black, so nothing hidden in the dark ever shows.

### Optional enhancements: the gem map

Off unless you turn it on (`--gem-map`, or the Everything preset), and it
never changes the game ([the gem map](settings.md#the-gem-map)).

- **A gem shows a real map.** Using a gem works exactly as in the
  original: `V` spends one gem and opens the game's view, and the next key
  closes it. With the enhanced gem map on, that view fills the whole window
  with your game's own tiles, shrunk, edge to edge. It **shows more ground
  than the original gem**: outdoors 128 by 60 cells on a 16:9 screen
  instead of 32 by 32. In a town the whole floor fills the screen, with the
  land you have seen outside the gates dimmed beside it. In a dungeon the
  levels you have seen stand side by side, joined by yellow boxes where a
  ladder leads, and on a 4:3 screen PgUp and PgDn show the level above and
  below. The party is a steady yellow box, the map is drawn at 70% light
  so the names stand out, and it fades in and out with a short dithered
  sweep. Like the original gem, the map shows the ground only: no people or
  monsters.
- **Plain labels.** Places, signs, shops, ladders and stairs, exits,
  fountains, chests and traps you have found get a coloured ring and a
  plain name ("Britain", "Sign", "Armoury", "Ladder down", "Exit").
- **No spoilers.** The map names only what you have discovered, and marks
  what it draws but you have not found with a grey "?" that says nothing
  about what it is. It never shows a sign's text, never says "Underworld"
  before you have been there, never draws a dungeon level you have not
  seen, and draws secret doors and hidden traps as they looked to you
  until you find them. What you have discovered is kept beside the save in
  `gemmap.u5e` ([sidecars.md](sidecars.md#gemmapu5e-the-gem-maps-discoveries)).
- **Known faults of the original view (not of this map).** A paired check
  against the original under DOSBox found that this engine's own 1.0.1 gem
  view differs from the original's: in towns it draws the townsfolk, which
  the original does not; outdoors it centres on the party where the
  original snaps to its map blocks; and in dungeons its symbols use other
  colours (D0.2). Separately, the party panel turns white after U-Use,
  where the original's does not (D0.3). These are faults of the faithful
  game, kept as they are in this release while a fix is decided; the
  enhanced gem map draws terrain only in towns, as the original does.

Checks behind it: using a gem outdoors, in a town and in a dungeon, with no
gem and in combat spends and shows exactly what the original does with the
map on and off (the gem count, the view, the random numbers, the clock, the
messages, the key that closes the view, and the save files); every
enhancement configuration in the toggle matrix plays exactly as the
original while gems are used; the route smoke is unchanged from 1.0.1; and a
no-spoiler suite checks every map drawn over a fresh character, a walk,
each dungeon, the Underworld and random discoveries, comparing every label
with the game's sign and conversation text.