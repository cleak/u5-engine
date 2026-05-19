# Clean-Room Sidecars

Sidecars are optional clean-room metadata files placed next to the user's local
game assets. They let tests and local play model public behavior whose exact
resident coordinates or table bytes are not shipped in this repository.

Sidecars must be authored from clean-room-safe sources. Do not commit original
asset dumps, raw map dumps, dialogue transcripts, private offsets, or data copied
from decompiled code.

## Location And Transition Tables

| File | Purpose |
|---|---|
| `world_locations.tsv` | Overworld coordinates for entering named towns/dungeons and resolving interior exits when no in-memory return snapshot exists. |
| `location_entry_y.tsv` | Sparse `LocationEntryYTable` overrides for town-family spawn Y values. |
| `location_floor_pages.tsv` | Sparse `LocationFloorBaseTable` overrides for signed town floor page loading. |
| `world_plane_transitions.tsv` | Britannia/Underworld chasm and ascent rows. |
| `dungeon_deeper_transitions.tsv` | Scripted dungeon deeper-to-world transitions. |
| `dungeon_teleports.tsv` | Dungeon teleport cells. |
| `dungeon_exit_tiles.tsv` | Dungeon exit cells that return to overworld metadata. |
| `town_stairs.tsv` | Clean stair rows for town floor changes. |
| `town_trap_doors.tsv` | Town trap-door/chute rows. |
| `town_exit_tiles.tsv` | Town exit threshold rows. |
| `moongates.tsv` | Authored moongate origin/destination rows. |

## Interaction Tables

| File | Purpose |
|---|---|
| `world_get_tiles.tsv` | World tile pickups and replacements. |
| `object_pickups.tsv` | Location-local object-table grants. |
| `town_get_tiles.tsv` | Town tile pickups and replacements. |
| `town_fire_sources.tsv` | Fire sources and related town interaction rows. |
| `town_pushables.tsv` | Town pushable furniture/object rows. |
| `town_rest_beds.tsv` | Town bed/rest surfaces accepted by H-Hole-up. |
| `town_locks.tsv` | Town lock rows for Jimmy/Open/Use-key handling. |
| `secret_doors.tsv` | Search-revealed secret doors. |
| `dungeon_doors.tsv` | Dungeon heavy-door rows. |
| `dungeon_chests.tsv` | Dungeon chest guard/grant metadata used by tests. |
| `dungeon_wind_tiles.tsv` | Dungeon wind/gust interaction rows. |
| `world_waterfalls.tsv` | Current/waterfall sweeps after accepted world movement. |
| `world_damage_tiles.tsv` | Lava/drowning damage cells and transport gates. |
| `world_encounters.tsv` | Authored encounter spawn rows. |
| `shrines.tsv` | Shrine coordinates and virtue binding. |
| `blink_targets.tsv` | Clean Blink landing rows. |
| `tile_passability.bin` | Optional 32-byte public tile passability bitmap. |

## General Rules

- TSV files are whitespace-separated and may contain comments starting with
  `#`.
- Optional tile guards prevent stale authored rows from firing after local map
  edits. Use `*` where the parser supports an explicit no-guard marker.
- Duplicate source rows or duplicate target rows are rejected when ambiguity
  would make exits or transitions unsafe.
- Missing sidecars should produce conservative no-op/diagnostic behavior, not
  invented private-derived coordinates.

See the parser tests in `u5-runtime` for exact row grammar and duplicate checks.
