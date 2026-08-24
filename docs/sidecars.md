# Clean-Room Sidecars

Sidecars are optional clean-room metadata files placed next to the user's local
game assets. They let tests and local play override published defaults or model
public behavior whose exact resident coordinates or table bytes are not shipped
in this repository.

Sidecars must be authored from clean-room-safe sources. Do not commit original
asset dumps, raw map dumps, dialogue transcripts, private offsets, or data copied
from decompiled code.

## Location And Transition Tables

| File | Purpose |
|---|---|
| `world_locations.tsv` | Optional overrides/extensions for the published stock overworld coordinates used to enter named towns/dungeons and resolve interior exits when no in-memory return snapshot exists. |
| `location_entry_y.tsv` | Sparse `LocationEntryYTable` overrides for town-family spawn Y values. |
| `location_floor_pages.tsv` | Sparse `LocationFloorBaseTable` overrides for signed town floor page loading. |
| `world_plane_transitions.tsv` | Britannia/Underworld chasm and ascent rows. |
| `dungeon_deeper_transitions.tsv` | Scripted dungeon deeper-to-world transitions. |
| `dungeon_teleports.tsv` | Dungeon teleport cells. |
| `dungeon_exit_tiles.tsv` | Dungeon exit cells that return to overworld metadata. |
| `town_stairs.tsv` | Clean stair rows for town floor changes. |
| `town_trap_doors.tsv` | Town trap-door/chute rows. |
| `town_poison_gas.tsv` | Legacy town poison-gas doorway rows; no longer used by the native #51 tile `0x04` branch. |
| `town_tile_attributes.tsv` | Legacy clean tile-id attributes; no longer used by the native #51 tile `0x04` branch. |
| `town_exit_tiles.tsv` | Removed. Town-family exit is a grid-boundary event; no tile id or sidecar row participates. |
| `moongates.tsv` | Removed. Superseded by the published natural-gate system; the runtime no longer reads this file. See below. |

`moongates.tsv` held authored moongate origin/destination rows from the period
before the natural-gate coordinates were published. `systems/overworld.md` §9
now specifies the whole feature from the eight saved Moonstone slots in
`SAVED.GAM`: a once-per-world-turn refresh stamps eligible slots as live `0xDC`
terrain, the shared sixteen-step presence counter at `SAVED.GAM` `0x02E1` drives
appearance, and destinations come from the Moonstone slots. The sidecar's model
contradicted that specification on four points - it gated gates on full daylight
(§9.2: the live `0xDC` terrain test "is the only precondition"), it asked a
`Y/N` prompt (§9.2: the entry hook is blocking and cannot be skipped), it drew
the gate tile unconditionally (§9.1 composes sixteen phases), and it carried
per-row active-hour windows (§9: one shared counter driven by the hour band).
The sidecar was also inert: nothing shipped a `moongates.tsv`. It has therefore
been removed rather than repaired.

`dungeon_deeper_transitions.tsv` rows use:

```text
DUNGEON LEVEL X Y TO_PLANE TO_X TO_Y
```

They only apply to bottom-level dungeon ladder descents that would otherwise
move below level `7`. Missing rows preserve the conservative in-place block.

## Interaction Tables

| File | Purpose |
|---|---|
| `world_get_tiles.tsv` | World tile pickups and replacements. |
| `object_pickups.tsv` | Location-local object-table grants. |
| `town_get_tiles.tsv` | Town tile pickups and replacements. |
| `town_fire_sources.tsv` | Optional town fire-source overrides; native static cannons use `0xB4..=0xB7` without a row. |
| `town_pushables.tsv` | Town pushable furniture/object rows. |
| `town_rest_beds.tsv` | Optional town bed/rest overrides for H-Hole-up; native inn scenes accept the public `0x48..=0x49` bed pair without a row. |
| `town_locks.tsv` | Town lock rows for Jimmy/Open/Use-key handling. |
| `secret_doors.tsv` | Search-revealed secret doors. |
| `dungeon_chests.tsv` | Dungeon chest guard/grant metadata used by tests. |
| `world_waterfalls.tsv` | Retired compatibility/diagnostic artifact; current runtime movement ignores it because the public baseline has no waterfall/current sweep sidecar. |
| `world_damage_tiles.tsv` | Lava/drowning damage cells and transport gates. |
| `world_encounters.tsv` | Optional encounter spawn overrides; unmatched terrain uses the native public selector. |
| `shrines.tsv` | Shrine coordinates and virtue binding. |
| `eternal_flames.tsv` | Optional legacy Eternal Flame coordinate overrides. Native shard destruction uses the three published exact party positions and still requires the matching Shadowlord-name encounter immediately north. |
| `tile_passability.bin` | Optional 32-byte base-predicate override; native player transport movement uses the published foot/horse/carpet/ship/skiff tile sets without this file. |

## Content And Companion Save Files

| File | Purpose |
|---|---|
| `common_words.tsv` | Optional 128-row clean override for the published shared common-word dictionary used by tokenized raw `.TLK` and `SHOPPE.DAT` text. |
| `end_narrative_windows.tsv` | Optional six-row clean seek-window override for custom final `END.DAT` endgame narrative pages; shipped ranges are built in from the public spec. |
| `SAVED.WPS` | Clean companion save for durable world-progress state, including compatibility mirrors for public `SAVED.GAM` fields that older clean saves only stored in the sidecar. |
| `SAVED.BTH` | Clean companion save for Blackthorn capture/rescue story state whose exact original `SAVED.GAM` offsets are not yet public. |
| `.u5-engine-town-npc-mutations` | Engine-owned companion save for public destructive town-NPC schedule/dialogue rewrites. It stores only scene, roster slot, three AI bytes, four schedule times, and dialogue byte; original `.NPC` assets remain read-only. |

## General Rules

- TSV files are whitespace-separated and may contain comments starting with
  `#`.
- Optional tile guards prevent stale authored rows from firing after local map
  edits. Use `*` where the parser supports an explicit no-guard marker.
- Duplicate source rows or duplicate target rows are rejected when ambiguity
  would make exits or transitions unsafe.
- Missing sidecars should use published defaults where available; otherwise they
  should produce conservative no-op/diagnostic behavior, not invented
  private-derived coordinates.

See the parser tests in `u5-runtime` for exact row grammar and duplicate checks.
