# Save Slots

The original game keeps one save: the working `SAVED.GAM` and its
companion files in the game's save folder, which Journey Onward loads. With
`menu.named_slots` on ([settings.md](settings.md#the-menu)) you can keep as
many saves as you like under names of your own, and go back to any of them.
With it off (the default) no slot folder is made, read or touched.

A slot is a whole copy of the working save, kept outside the game folder.
Saving to a slot is a real save: the menu types the game's own `Q` and `Y`,
so the working save, which Journey Onward and the DOS game read, holds that
moment too, and then the files are copied into the slot. Loading a slot puts
its files back as the working save and starts the game again in it, as if
you had copied the files by hand and pressed `J` at the title.

## The folder

```text
<data folder>/saves/<work key>/
  slots/<slot id>/
    SAVED.GAM SAVED.OOL BRIT.OOL UNDER.OOL
    SAVED.WPS .u5-engine-town-npc-mutations   when the save has them
    .u5-engine-enh/                           copied whole when present
    slot.toml                                 what the slot is
    thumb.png                                 80x50, absent for an import
  .backup/          the working save as it was before the last load
  .lock             held while a copy is being made
```

- **The data folder** is `$XDG_DATA_HOME/u5-engine` on Linux (by default
  `~/.local/share/u5-engine`), `%LOCALAPPDATA%\u5-engine` on Windows and
  `~/Library/Application Support/u5-engine` on macOS. `U5_ENGINE_DATA_DIR`
  names another one.
- **The work key** is the name of the folder the working `SAVED.GAM` lives
  in, then a hash of its full path (`ultima5-1a2b3c4d5e6f7081`), so each
  game folder has its own slots and two copies of the game never mix
  theirs.
- **The slot id** is `quick` for the quick slot and `s0001`, `s0002` and so
  on for the others. It is never the name you gave the slot, so renaming a
  slot only rewrites its `slot.toml`.
- Slots are never written into the game folder, the original game files or
  the runtime save mirror.

## `slot.toml`

```toml
format = 1
name = "Before the shrine"
saved_at_unix = 1789000000
engine = "1.0.1"
origin = "menu"            # menu, quick, import or cli
avatar = "Tester"
place = "Britannia"
clock = [139, 4, 7, 9, 15] # year, month, day, hour, minute
turn = 812
modified = false
cheats_used = []

[sha256]
"SAVED.GAM" = "..."
"SAVED.OOL" = "..."
".u5-engine-enh/gemmap.u5e" = "..."
```

- **What a slot shows** is the Avatar's name, the place, the game's date and
  time, and the turn. The place is where the party stood when it saved, so
  it is always a place you have been: the plane outdoors (`Britannia`,
  `Underworld`), the town, castle, keep or dwelling by the name the gem map
  gives it once you are inside, a dungeon and its level (`Despise level 2`),
  or plain words for a hut with no name (`a dwelling`). A slot never shows
  quest progress, and its thumbnail is the game's own screen at the moment
  it saved, never the wider view or the menu.
- **`modified` and `cheats_used`** mark a slot saved after a cheat changed
  the game (chapter 7, not in this build). The Load page shows "C".
- **`sha256`** holds the digest of every copied file. A slot whose files are
  missing, the wrong length or not what the digests say, or whose
  `slot.toml` cannot be read or has a newer `format`, is listed as
  "Damaged": it can be deleted, never loaded. Unknown keys are ignored.

## Using slots

In the menu (`menu.overlay`, F12 at the command prompt):

- **Save thy game...** lists "A new slot...", the quick slot (while
  `menu.quicksave` is on) and every slot. A new slot asks for a name of up
  to 24 printable ASCII characters; an empty name becomes "Slot N". The
  quick slot saves at once; another slot asks before writing over it. The
  menu closes, the game prints its own save lines, and a note says where
  it went.
- **Load a saved game...** lists the slots, newest first with `quick` on
  top, with the thumbnail and details of the one selected. Enter loads,
  F2 renames, Del deletes (it asks first). With `menu.confirm_unsaved` on,
  a load asks first when the turn has moved since the last save or load.
- **The title layer** (`menu.title_layer`, F12 at the title menu) offers
  Continue, which loads the newest slot, and Load a saved game....

From the command line, with `menu.named_slots` on and without the menu:

| Option | What it does |
|---|---|
| `--slots` | List the slots of the game's working save. |
| `--slot-save NAME` | Copy the current working save into the slot `NAME` (a new one unless a slot has that name). It does not run the game's save. |
| `--slot-load NAME` | Make the working save exactly the slot `NAME`, then start the game in it (Journey Onward at boot). |

The slot commands are refused under the parity lock ("not available under
the parity lock") and exit with an error, so a test harness never changes a
working save by accident.

## Imports

A slot made without the game's save, by `--slot-save`, is an import: the
files are copied as they are. To keep a save from the DOS game, put its
`SAVED.GAM` and `.OOL` files in the game's save folder and run
`u5-engine --slot-save NAME`. An import has no thumbnail (the Load page
shows "Imported") and no turn count; its Avatar, place and date are read
from the save itself, by the same rule as above.

Loading a slot makes the working save exactly the slot: a file the slot does
not have is deleted from the working save. So loading a DOS import leaves no
`SAVED.WPS`, town changes or `.u5-engine-enh/` folder of the game you were
playing behind.

## Loading restarts the window

A load closes the game window and starts the game again with `J` waiting,
which goes straight into Journey Onward with no title. The new window opens
in about a second. A restart gives the game a fresh start, as the original
has after a reboot: nothing of the old session carries over. "Return to the
title" restarts the same way, with no key waiting, so the whole intro plays.

## After a crash

Every operation first repairs what an interrupted one left, so a crash, a
power cut or a full disk leaves either the old state or the new one, never
a mix:

- **A save to a slot** builds the new copy in `slots/.tmp-<id>-<pid>/`,
  moves the old copy aside, puts the new one in place and only then deletes
  the old one. Interrupted at any point, the slot is the old one or the
  new one, and leftovers are cleared at the next operation.
- **A load** first copies the working save into `.backup/` and writes a
  `.restore-pending` mark, then replaces each file through a temporary
  name. If the load fails, or a later launch finds the mark, the working
  save is put back from `.backup/`. If the game cannot start again after a
  load, the working save is put back the same way, the reason is printed,
  and the next launch finds your game as it was.
- **Two copies of the game** take turns: a copy waits up to two seconds for
  the other to finish, then says "The saves are busy (another copy of the
  game?)". A lock left by a crashed copy is broken after 30 seconds.

## Files of other features

A slot copies the save folder's `.u5-engine-enh/` folder whole, whichever
features own the files in it, and a load replaces it whole. The files are
copied as they are, never read: each feature checks its file against the
save when it is next on ([sidecars.md](sidecars.md#player-sidecars)).
