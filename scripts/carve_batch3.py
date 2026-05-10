"""Batch 3: carve PlayState struct, table-entry types, graphics types."""

from __future__ import annotations

from pathlib import Path

from carve_items import carve_to_module

PARTS = Path("crates/u5-runtime/src/parts")
SRC = Path("crates/u5-runtime/src")


def main() -> int:
    # PlayState struct + overlay caches.
    carve_to_module(
        dest=SRC / "play_state_struct.rs",
        summary="The PlayState struct and overlay caches (impl blocks live in parts/play_state_impl/).",
        sources=[PARTS / "part_02.rs"],
        items=["PlayState", "WorldOverlayCache", "WorldReturn"],
    )

    # World tables.
    carve_to_module(
        dest=SRC / "world_tables.rs",
        summary="Data structures for the world TSV tables (locations, plane transitions, get-tiles, pickups, waterfalls, damage, encounters, shrines).",
        sources=[PARTS / "part_02.rs", PARTS / "part_03.rs"],
        items=[
            "WorldLocationEntry",
            "ShrineEntry",
            "WorldPlaneTransitionEntry",
            "WorldGetTileEntry",
            "ObjectPickupKind",
            "ObjectPickupGrant",
            "ObjectPickupEntry",
            "WorldWaterfallEntry",
            "WorldWaterfallSweep",
            "WorldDamageEffect",
            "WorldDamageTileEntry",
            "WorldEncounterEntry",
            "tile_get_message",
        ],
    )

    # Dungeon tables.
    carve_to_module(
        dest=SRC / "dungeon_tables.rs",
        summary="Data structures for the dungeon TSV tables (deeper transitions, teleports, chests, wind, exits, doors, secret doors).",
        sources=[PARTS / "part_03.rs"],
        items=[
            "DungeonDeeperTransitionEntry",
            "DungeonTeleportEntry",
            "DungeonChestContentEntry",
            "DungeonWindTileEntry",
            "DungeonExitTileEntry",
            "DungeonDoorEntry",
            "SecretDoorEntry",
        ],
    )

    # Town tables.
    carve_to_module(
        dest=SRC / "town_tables.rs",
        summary="Data structures for the town TSV tables (fire sources, pushables, get-tiles, rest beds, stairs, trap doors, exits, locks).",
        sources=[PARTS / "part_03.rs"],
        items=[
            "TownFireSourceEntry",
            "TownPushableEntry",
            "TownGetTileEntry",
            "TownRestBedEntry",
            "TownStairKind",
            "TownStairEntry",
            "TownTrapDoorEntry",
            "TownExitTileEntry",
            "TownLockKind",
            "TownLockEntry",
        ],
    )

    # Misc tables.
    carve_to_module(
        dest=SRC / "misc_tables.rs",
        summary="Misc TSV-table data structures: blink targets, town fire targets, moongates, tile descriptions, location floor/entry-y.",
        sources=[PARTS / "part_03.rs"],
        items=[
            "BlinkTargetEntry",
            "TownFireTarget",
            "MoongateEntry",
            "LookTable",
            "LocationFloorEntry",
            "LocationEntryYEntry",
            "TilePassability",
        ],
    )

    # Graphics formats and tile rasterization.
    carve_to_module(
        dest=SRC / "graphics.rs",
        summary="Tile atlas, viewport, palettes, and image/font formats (TileGraphicsDepth, TileAtlas, TileViewport, GraphicImage*, MonochromeBitmap, TitleBitImages, FixedFont, ProportionalFont).",
        sources=[PARTS / "part_03.rs"],
        items=[
            "TileGraphicsDepth",
            "TileAtlas",
            "TileViewport",
            "EGA_PALETTE_RGB",
            "CGA_PALETTE_RGB",
            "TopDownRenderArea",
            "GraphicImage",
            "GraphicImageDirectory",
            "GraphicSprite",
            "GraphicSpriteSheet",
            "MonochromeBitmap",
            "TitleBitImages",
            "TextCellStyle",
            "FixedFont",
            "ProportionalGlyph",
            "ProportionalFont",
        ],
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
