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
  Everything. In this release Recommended turns on sprites over terrain and
  auto-combat with its fast-forward (below), and Everything adds sharp
  scaling and the F11 fullscreen key.
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

### Optional enhancements: the menu and save slots

All of it is off unless you turn it on (`--menu-overlay`, `--named-slots`,
`--title-layer`, or the Everything preset), and none of it changes the game
([the menu](settings.md#the-menu), [slots.md](slots.md)).

- **The menu is optional.** F12 at the command prompt opens a menu drawn in
  the game's own font, frame and colours: return to the game, save, load,
  settings, return to the title, leave the game. Nothing in it is needed to
  play: every setting it shows also has a line in `config.toml`, a
  command-line flag and a row on the setup window's Enhancements page
  (`u5-engine --settings`), and the game's own keys are unchanged. F12
  anywhere else, and Escape at the bare prompt, do what they do in the
  original.
- **Settings in the game.** Audio (volume, mute, mute while the window is
  in the background), Display, Game (the game's own Sound and Buffer, two
  start keys, the unsaved warning) and Extras (every enhancement, with the
  presets). Changes are saved to `config.toml`.
- **Named save slots.** Keep as many saves as you like under your own
  names, with a quick slot, a thumbnail of the screen and where and when
  you saved. A slot save is a real save: the menu types the game's own
  `Q` and `Y`, so the save Journey Onward loads is that moment too, and
  then the files are copied. Without the menu, `--slots`, `--slot-save
  NAME` and `--slot-load NAME` do the same from the command line.
- **Loading restarts the game window.** A load puts the slot's files back as
  the working save and starts the game again with `J` waiting, straight
  into Journey Onward; the window reopens in about a second. Return to the
  title restarts the same way and plays the whole intro.
- **The title menu.** With `--title-layer`, F12 at the title menu offers
  Continue (your newest slot), Load, Settings and Leave the game.
- **No spoilers.** A slot shows only the Avatar, the place the party stood
  in (by the name you have seen there), the game's date and the turn, and
  its thumbnail is the game's own screen at that moment. No note, row or
  title line names a person, place or thing of the game.
- **Not tested on Windows or macOS.** The manual checks of this release
  (the mute when the window loses the focus or is minimised, the restart
  after a load from both `u5-engine` and `u5-engine-gui`, the slot folder,
  typing a slot name on a non-US keyboard layout) were made on Linux only,
  under X11; this build's window does not open as a native Wayland client,
  so on a Wayland desktop it runs through XWayland.

Checks behind it: every menu and audio setting alone, every pair of them
and of the wider view, sprites over terrain and the gem map, and every
setting on but the cheats, each with the menu opened, used and closed
during walks in town, outdoors, in a dungeon and in a conversation, play
exactly as the original, frame for frame, on the random numbers, the
clock and the save files (the keys the menu types count as typed keys); a
no-spoiler suite saves through the menu from a fresh character, after
walks, in a dungeon and in the Underworld; the route smoke is unchanged
from 1.0.1; two full paired DOSBox sweeps found no difference from the
original that was not a random draw the original makes too; and a paired
run with every one of these settings on in the configuration file ran
under the parity lock, with no menu, and matched the original.

### Optional enhancements: auto-combat

Off unless you turn it on (`--auto-combat`, or the Recommended and
Everything presets), and turning it on changes nothing until you press one
of its keys in a fight ([auto-combat](settings.md#auto-combat),
[keys](commands.md#auto-combat-keys)).

- **Hand a fight over.** Ctrl+A hands the member whose turn it is to
  auto-combat, Ctrl+Shift+A the whole party, and Ctrl+F the whole party
  until the fight is over. A short note confirms each change, an `AUTO`
  badge shows while anyone is handed over, and any key you press gives the
  prompt back. It never stops in the middle of
  an attack.
- **It plays by your keys.** Auto-combat types the keys you would type, one
  at a time: attack the nearest foe it can see, step towards one, or pass.
  It uses only legal keys; it is not a strong player. It stops at victory,
  when a member is badly hurt (and names that member), when no foe is in
  sight, when it is stuck, and after an action limit, and a short note says
  why. Notes name only your party, never a monster, and it never aims at
  anything you cannot see.
- **Fast-forward** (`--auto-combat-fast-forward`, also in Recommended and
  Everything). In a fight handed over with Ctrl+F every sound plays only its
  first 40 ms, in the same order, so the fight goes by quicker. This is the
  one deviation a preset turns on, and it is labelled "Faster fights:
  changes timing, so fights differ from normal pace": the game's clock keeps
  running while a sound plays, so a fast-forwarded fight can take a
  different course from the same fight at normal pace. Without Ctrl+F, or
  with fast-forward off, the sounds are the original's.
- **In the terminal** an `@auto` line (`@auto`, `@auto party`, `@auto 1,3`,
  `@auto until`, `@auto off`) does the same, and `--auto-combat-log FILE`
  writes the keys it typed as a play script that replays the fight with the
  feature off.
- **Determinism.** In the terminal an auto fight is exactly repeatable: the
  same keys after the same waits give the same fight, and the log replays it
  byte for byte. In the window the keys are the same kind, but when each one
  arrives depends on the frame clock, so two windowed fights from the same
  save can differ, as two fights a person plays can.
- **Where the tactics come from.** They are this engine's own behaviour,
  written for this frontend from the Phase B design; they only choose among
  keys the game already accepts and read only what the screen shows.

Checks behind it: with auto-combat off or under the parity lock there is no
auto-combat state, no key is bound, no log is written and no `[combat]`
section is added to `config.toml`; the keys an engaged fight typed, replayed
with the feature off at the same frames, give the same game and the same
`SAVED.GAM`, beside every other shipped enhancement; the game's picture is
the same with the badge shown and hidden; and the same keys and notes come
with or without a monster the party cannot see.

### Optional enhancements: cheats and the change watch

Off unless you turn them on, never in a preset, and labelled wherever they
show ([cheats](settings.md#cheats)). With every cheat off nothing of them
is loaded, no key is taken and no file is read or written: the game is the
1.0.1 game.

- **Two steps.** Allow cheats first (`cheats.enabled`), then turn on each
  cheat you want by its own key, for example
  `--enable cheats.enabled,cheats.heal_all,cheats.lock_hp`. Ctrl+F12 opens
  the Cheats page at the command prompt; a cheat that is off is not on it.
- **Quick actions.** Heal all, Cure poison, Resurrect all, Max gold, Max
  food, Refill arrows (and quarrels), Max reagents and Repair hull. Values
  go only to the game's own caps (9,999 gold and food, 99 for the rest, a
  frigate's hull to 99), never down, never a hull of 100 or more, never a
  pirate ship. Ashes stay ashes. No action uses a random number or passes
  time.
- **Locks.** Health, mana, food, gold, keys, gems, torches, arrows and the
  reagents, each topped up once a turn (and once a combat round) at the
  command prompt while checked. A lock never brings a member back to life.
  Locks start unchecked at each launch; keeping them between launches
  (`remember_locks`) is not in this build yet.
- **Edit party and stores.** Strength, Dexterity, Intelligence, HP, MP,
  experience, status and the shared stores, each within the range the
  game itself uses, applied together with `[A]` and undone with `[U]`.
  `[L]` sets a member's level from experience as camping does.
- **Teleport. Spoiler: it lists every place in the game**, including ones
  you have not found; its title says `SPOILERS`. Any cell of Britannia
  or the Underworld, any floor your files have of every town, castle,
  keep and dwelling, and any level of the eight dungeons. Only at the
  prompt, not in combat or aboard a ship or skiff, with no time passing.
  A blocked target is moved to the nearest open cell where there is one,
  or refused.
- **The change watch** (`cheats.state_watch`). A live log of named
  fields (party, stores, place, time) as they change, kept to the last
  4,096 changes and saved as plain text with `[S]`. It only reads the
  game, needs no "Allow cheats", and never lists quest flags, seals, the
  Shadowlords or their hideouts.
- **The "modified" mark.** Once a cheat has changed your game, the page
  says "This game: MODIFIED", a brown `MOD` badge shows by the picture,
  and every save you make is marked beside it
  (`save-meta.u5e`; [sidecars.md](sidecars.md#save-metau5e-the-modified-mark)).
  `SAVED.GAM` is never marked, so the original game still loads the save.
  Turning cheats off does not unmark a save. Limits: a save made while
  cheats are off is not marked and later shows "unknown"; the mark records
  that cheats were used, not which ones; and deleting the file removes it.
- **The parity lock** turns every cheat off whatever the settings say.
  It prints only its one line (and a note naming the flags it ignored),
  loads no cheat code and never touches the mark.

Checks behind it: the watch alone and every cheat on but unused, each also
with the wider view and with the gem map, play exactly as all off in the
terminal and in the windowed shell, where the page is also opened and
closed (the game, its picture, its keys, the random numbers and the save
files); the
same cheats on the same turns reproduce exactly, and a cheated game
differs from the uncheated one only in the fields those cheats name; a
fresh game with every cheat off has no Cheats page and no teleport list;
and every cheat asked for under the parity lock loads nothing and writes
nothing.
