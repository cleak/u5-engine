//! Engine-wide constants: filenames, save offsets, tile sentinels,
//! world dimensions, spell tables, defaults.

pub const DEFAULT_GAME_DIR: &str = r"C:\Games\U5-Clean";
pub const REPORT_PATH: &str = "reports/lb-throne-room-slice.txt";
pub const WORLD_LOCATION_TABLE_FILE: &str = "world_locations.tsv";
pub const WORLD_PLANE_TRANSITION_TABLE_FILE: &str = "world_plane_transitions.tsv";
pub const WORLD_GET_TILE_TABLE_FILE: &str = "world_get_tiles.tsv";
pub const OBJECT_PICKUP_TABLE_FILE: &str = "object_pickups.tsv";
pub const WORLD_WATERFALL_TABLE_FILE: &str = "world_waterfalls.tsv";
pub const WORLD_DAMAGE_TILE_TABLE_FILE: &str = "world_damage_tiles.tsv";
pub const WORLD_ENCOUNTER_TABLE_FILE: &str = "world_encounters.tsv";
pub const SHRINE_TABLE_FILE: &str = "shrines.tsv";
pub const DUNGEON_DEEPER_TRANSITION_TABLE_FILE: &str = "dungeon_deeper_transitions.tsv";
pub const DUNGEON_TELEPORT_TABLE_FILE: &str = "dungeon_teleports.tsv";
pub const DUNGEON_WIND_TILE_TABLE_FILE: &str = "dungeon_wind_tiles.tsv";
pub const DUNGEON_EXIT_TILE_TABLE_FILE: &str = "dungeon_exit_tiles.tsv";
pub const DUNGEON_DOOR_TABLE_FILE: &str = "dungeon_doors.tsv";
pub const DUNGEON_CHEST_TABLE_FILE: &str = "dungeon_chests.tsv";
pub const SECRET_DOOR_TABLE_FILE: &str = "secret_doors.tsv";
pub const TOWN_FIRE_SOURCE_TABLE_FILE: &str = "town_fire_sources.tsv";
/// `vehicles.md §8`: town F-Fire scans a short fixed line for the
/// first blocking target. This is the maximum distance, in cells,
/// from the fire-source tile that the cannon-line tracer probes.
pub const TOWN_CANNON_RANGE_CELLS: i32 = 3;
/// `vehicles.md §8`: on a successful town F-Fire hit against an
/// active object, the shared moral-standing selector is reduced by
/// this many units (floored at zero by the karma helper). The
/// overworld broadside path does not apply this debit; only the
/// local cannon path does.
pub const TOWN_CANNON_HIT_KARMA_DEBIT: u8 = 5;
pub const TOWN_PUSHABLE_TABLE_FILE: &str = "town_pushables.tsv";
pub const TOWN_GET_TILE_TABLE_FILE: &str = "town_get_tiles.tsv";
pub const TOWN_REST_BED_TABLE_FILE: &str = "town_rest_beds.tsv";
pub const TOWN_STAIR_TABLE_FILE: &str = "town_stairs.tsv";
pub const TOWN_TRAP_DOOR_TABLE_FILE: &str = "town_trap_doors.tsv";
pub const TOWN_EXIT_TILE_TABLE_FILE: &str = "town_exit_tiles.tsv";
pub const TOWN_LOCK_TABLE_FILE: &str = "town_locks.tsv";
pub const BLINK_TARGET_TABLE_FILE: &str = "blink_targets.tsv";
pub const MOONGATE_TABLE_FILE: &str = "moongates.tsv";
pub const LOCATION_FLOOR_TABLE_FILE: &str = "location_floor_pages.tsv";
pub const LOCATION_ENTRY_Y_TABLE_FILE: &str = "location_entry_y.tsv";

/// `formats/location-dat.md §6` default per-scene town-entry X
/// coordinate. The published rule fixes the entry X column at
/// fifteen; the entry Y comes from the per-scene `LocationEntryYTable`
/// (loaded from `location_entry_y.tsv`) and the entry floor is zero.
/// Promote the X constant so the town-entry seed sites can name the
/// column instead of repeating the bare literal `15`.
pub const LOCATION_DEFAULT_ENTRY_X: usize = 15;
pub const TILE_PASSABILITY_FILE: &str = "tile_passability.bin";
pub const LOOK2_DAT_FILE: &str = "LOOK2.DAT";

/// `formats/look2-dat.md §6` legacy DOS end-of-file marker
/// (`Ctrl-Z`, byte `0x1A`). Several shipped data files end with
/// this byte; readers ignore it when computing the meaningful
/// payload length. This is the historical CP/M / DOS text-file
/// terminator; it never appears mid-record.
pub const DOS_EOF_MARKER: u8 = 0x1A;
/// `formats/look2-dat.md §2`: 1024-byte offset table holds 512
/// little-endian word offsets — entries 0..=255 for terrain
/// descriptions and 256..=511 for object descriptions.
pub const LOOK2_DAT_OFFSET_TABLE_LEN: usize = 1024;
pub const LOOK2_DAT_TERRAIN_ENTRIES: usize = 256;
pub const LOOK2_DAT_OBJECT_ENTRIES: usize = 256;
pub const LOOK2_DAT_OBJECT_DOMAIN_BASE: usize = 0x200;
pub const KARMA_DAT_FILE: &str = "KARMA.DAT";
/// `formats/karma-dat.md §2`: file size in the shipped DOS data set.
pub const KARMA_DAT_LEN: usize = 761;
/// `formats/karma-dat.md §2`: number of NUL-terminated text records.
pub const KARMA_DAT_RECORDS: usize = 6;
pub const TILES_EGA_FILE: &str = "TILES.16";
pub const TILES_CGA_FILE: &str = "TILES.4";
/// `formats/bit.md §3`: each pointer-table entry is exactly 4 bytes
/// (a strip-pointer word followed by a metadata word).
pub const BIT_POINTER_TABLE_ENTRY_LEN: usize = 4;
/// `formats/bit.md §3`: leading two-byte entry-count word precedes
/// the pointer-table entries.
pub const BIT_ENTRY_COUNT_WORD_LEN: usize = 2;
/// `formats/bit.md §3`: a strip-pointer word value of zero means the
/// entry has no associated strip body (skipped by the driver scan).
pub const BIT_STRIP_POINTER_NONE: u16 = 0;
/// `formats/bit.md §4.3`: `WD.BIT` is a single-entry resource whose
/// "Warriors of Destiny" lettering is exactly 49 rows tall.
pub const WD_BIT_LETTERING_ROWS: u16 = 49;
/// `formats/bit.md §3` strip-body header word widths. Each strip
/// body opens with a width-related word and a row-count word before
/// its packed pixel payload — four bytes of header total. Promote
/// the widths so a future driver/bitmap decoder names the strip
/// header instead of repeating `2` as a bare literal.
pub const BIT_STRIP_WIDTH_WORD_LEN: usize = 2;
pub const BIT_STRIP_ROW_COUNT_WORD_LEN: usize = 2;
pub const BIT_STRIP_HEADER_LEN: usize =
    BIT_STRIP_WIDTH_WORD_LEN + BIT_STRIP_ROW_COUNT_WORD_LEN;
pub const CH_GLYPH_COUNT: usize = 128;
pub const CH_CELL_SIDE: usize = 8;
/// `formats/font-ch.md §2`: each .CH glyph is an 8x8 cell with
/// one byte per row, so per-glyph = 8 rows × 1 byte = 8 bytes.
/// Anchored to [`CH_CELL_SIDE`] (rows-per-glyph; each row is one
/// byte wide) so the per-glyph byte count and the cell geometry
/// stay one value.
pub const CH_GLYPH_BYTES: usize = CH_CELL_SIDE;
/// `formats/font-ch.md §2,§3`: a shipped `.CH` font is exactly
/// 1024 bytes (128 glyphs × 8 bytes each). Anchored to
/// [`CH_GLYPH_COUNT`] × [`CH_GLYPH_BYTES`] so the file size and
/// the catalog/glyph dimensions stay one value.
pub const CH_FONT_LEN: usize = CH_GLYPH_COUNT * CH_GLYPH_BYTES;
/// `formats/font-hcs.md §2,§3`: a shipped `.HCS` font is exactly
/// 3072 bytes (128 glyphs × 24 bytes each), each glyph a 16x12 cell
/// with two bytes per row. Anchored to [`HCS_GLYPH_COUNT`] ×
/// [`HCS_GLYPH_BYTES`] so the file size and the catalog/glyph
/// dimensions stay one value.
pub const HCS_FONT_LEN: usize = HCS_GLYPH_COUNT * HCS_GLYPH_BYTES;
/// `formats/font-hcs.md §2`: 128 glyphs per HCS font — the same
/// 128-entry character set the .CH catalog ships. Anchored to
/// [`CH_GLYPH_COUNT`] so the two font catalogs stay one value.
pub const HCS_GLYPH_COUNT: usize = CH_GLYPH_COUNT;
pub const HCS_CELL_WIDTH: usize = 16;
pub const HCS_CELL_HEIGHT: usize = 12;
pub const HCS_BYTES_PER_ROW: usize = 2;
/// `formats/font-hcs.md §2`: per-glyph byte count derived from the
/// cell geometry — twelve rows × two bytes per row = 24 bytes per
/// glyph. Anchored to [`HCS_CELL_HEIGHT`] × [`HCS_BYTES_PER_ROW`]
/// so the per-glyph byte count and the cell geometry stay one
/// value.
pub const HCS_GLYPH_BYTES: usize = HCS_CELL_HEIGHT * HCS_BYTES_PER_ROW;
#[cfg(test)]
pub const TITLE_BIT_FILE: &str = "TITLE.BIT";
#[cfg(test)]
pub const BRITISH_BIT_FILE: &str = "BRITISH.BIT";
#[cfg(test)]
pub const WD_BIT_FILE: &str = "WD.BIT";
#[cfg(test)]
pub const IBM_CH_FILE: &str = "IBM.CH";
#[cfg(test)]
pub const RUNES_CH_FILE: &str = "RUNES.CH";
#[cfg(test)]
pub const IBM_HCS_FILE: &str = "IBM.HCS";
#[cfg(test)]
pub const RUNES_HCS_FILE: &str = "RUNES.HCS";
#[cfg(test)]
pub const PROPORT_PCS_FILE: &str = "PROPORT.PCS";
pub const TILE_PASSABILITY_LEN: usize = 32;
pub const LOOK2_TILE_COUNT: usize = 512;
pub const LOOK2_TABLE_LEN: usize = LOOK2_TILE_COUNT * 2;
pub const TILE_ATLAS_TILE_COUNT: usize = 512;
pub const TILE_ATLAS_SIDE: usize = 16;
pub const TILE_ATLAS_TILE_PIXELS: usize = TILE_ATLAS_SIDE * TILE_ATLAS_SIDE;
pub const TILE_ATLAS_PIXEL_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_TILE_PIXELS;
pub const TILE_ATLAS_EGA_TILE_STRIDE: usize = TILE_ATLAS_TILE_PIXELS / 2;
pub const TILE_ATLAS_CGA_TILE_STRIDE: usize = TILE_ATLAS_TILE_PIXELS / 4;
pub const TILE_ATLAS_EGA_BODY_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_EGA_TILE_STRIDE;
pub const TILE_ATLAS_CGA_BODY_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_CGA_TILE_STRIDE;
pub const LZW_CLEAR_CODE: u16 = 256;
pub const LZW_END_CODE: u16 = 257;
pub const LZW_FIRST_USER_CODE: u16 = 258;
pub const LZW_MAX_CODES: u16 = 4096;
pub const LZW_INITIAL_CODE_SIZE: u8 = 9;
pub const LZW_MAX_CODE_SIZE: u8 = 12;
/// `formats/lzw.md §2`: the LZW envelope opens with a four-byte
/// little-endian unsigned length giving the exact number of decoded
/// bytes that follow the code stream. Promote the header width so
/// decode_lzw_envelope does not encode `4` as a bare literal.
pub const LZW_ENVELOPE_LENGTH_HEADER_BYTES: usize = 4;

/// `formats/tiles.md §5.1.1` resident miniature tile-glyph encoding.
/// The stats panel and a few inventory-style contexts render a
/// compact per-tile miniature whose record describes sixteen rows
/// with two offset bytes per row, for thirty-two bytes per tile.
pub const MINIATURE_TILE_ROWS: usize = 16;
pub const MINIATURE_TILE_OFFSET_BYTES_PER_ROW: usize = 2;
pub const MINIATURE_TILE_RECORD_BYTES: usize =
    MINIATURE_TILE_ROWS * MINIATURE_TILE_OFFSET_BYTES_PER_ROW;

/// `formats/tiles.md §5.2` image-directory count-word width. The
/// directory opens with a little-endian unsigned word giving the
/// number of slot entries that follow.
pub const TILE_IMAGE_DIRECTORY_COUNT_BYTES: usize = 2;
/// `formats/tiles.md §5.2` per-slot offset width. Each entry in the
/// offset table is a little-endian unsigned doubleword.
pub const TILE_IMAGE_DIRECTORY_OFFSET_BYTES: usize = 4;
/// `formats/tiles.md §5.2` per-image header width. Each image block
/// opens with a width word and a height word, two bytes each.
pub const TILE_IMAGE_BLOCK_HEADER_BYTES: usize = 4;
#[cfg(test)]
pub const SINGLE_IMAGE_BIT_FORMAT_MARKER: u16 = 1;
#[cfg(test)]
pub const SINGLE_IMAGE_BIT_MODE_MARKER: u16 = 4;
/// Both shipped fixed-cell fonts (.CH and .HCS) carry exactly 128
/// glyphs (`formats/font-ch.md §2`, `formats/font-hcs.md §2`).
/// Anchored to [`CH_GLYPH_COUNT`] so the parse-side glyph count
/// stays one value with the .CH catalog size; [`HCS_GLYPH_COUNT`]
/// is required to match.
#[cfg(test)]
pub const FIXED_FONT_GLYPH_COUNT: usize = CH_GLYPH_COUNT;
/// `formats/font-ch.md §2`: .CH cells are 8x8. Anchored to
/// [`CH_CELL_SIDE`] so the parse-side cell width stays one value.
#[cfg(test)]
pub const CH_FONT_CELL_WIDTH: usize = CH_CELL_SIDE;
/// `formats/font-ch.md §2`: .CH cells are 8x8. Anchored to
/// [`CH_CELL_SIDE`] so the parse-side cell height stays one value.
#[cfg(test)]
pub const CH_FONT_CELL_HEIGHT: usize = CH_CELL_SIDE;
/// `formats/font-hcs.md §2`: .HCS cells are 16x12. Anchored to
/// [`HCS_CELL_WIDTH`] so the parser-side cell width stays one
/// value with the format-side cell width.
#[cfg(test)]
pub const HCS_FONT_CELL_WIDTH: usize = HCS_CELL_WIDTH;
/// `formats/font-hcs.md §2`: .HCS cells are 16x12. Anchored to
/// [`HCS_CELL_HEIGHT`] so the parser-side cell height stays one
/// value with the format-side cell height.
#[cfg(test)]
pub const HCS_FONT_CELL_HEIGHT: usize = HCS_CELL_HEIGHT;
#[cfg(test)]
pub const PCS_FIRST_CODE: u8 = 0x20;
#[cfg(test)]
pub const PCS_GLYPH_BITMAP_WIDTH: usize = 8;
#[cfg(test)]
pub const PCS_GLYPH_HEIGHT: usize = 11;
#[cfg(test)]
pub const PCS_GLYPH_BLOCK_LEN: usize = 1 + PCS_GLYPH_HEIGHT;
pub const PLAY_SCRIPT_MAX_IDLE_TICKS: usize = 1024;
/// Runtime count of karma reaction records. The KARMA.DAT parser
/// uses this same loop bound to walk the six NUL-terminated text
/// records the format spec documents. Anchored to
/// [`KARMA_DAT_RECORDS`] so the parser-side loop bound and the
/// format-side record count stay one value.
pub const KARMA_RECORD_COUNT: usize = KARMA_DAT_RECORDS;
pub const PLAY_IGNORED_INPUT_KEY: char = '\u{1e}';
pub const PLAY_TYPEAHEAD_TOGGLE_KEY: char = '\u{1f}';
pub const TRAP_NON_COMBAT_EFFECT_TABLE: [u8; 8] = [0, 0, 0, 1, 1, 2, 2, 3];
pub const TRAP_ACID_DAMAGE_MAX: u8 = 30;
pub const TRAP_BOMB_DAMAGE_MAX: u8 = 8;
// Sentinel value the active-object table uses to mark the player slot
// (slot zero). Per `u5-spec/catalogs/tile-catalog.md` Section 14, this
// is "the player avatar sprite sentinel value 0xFC referenced in the
// town-entry handler" -- a marker, NOT the actual sprite to render.
// `PLAYER_SPRITE_TILE` below is what the renderer should display.
pub const PLAYER_TILE: u8 = 0xfc;

// The actual avatar sprite tile id in the EGA atlas. The character
// sprites live in the upper half of the 9-bit tile space (256..=511);
// tile 0x144 is the south-facing on-foot avatar walking frame. LOOK2.DAT
// labels lower-half 0xFC as "a bellows" which is why a literal blit of
// PLAYER_TILE shows a blacksmith's bellows on the map.
pub const PLAYER_SPRITE_TILE: usize = 0x144;

// Moongate is a single static sprite at tile id 0xDC per LOOK2.DAT
// ("a moon gate!"). Earlier guesses at 0x80 and 0xD4 picked the wrong
// tiles (food/banquet and a waterfall animation respectively).
pub const MOONGATE_TILE_BASE: u8 = 0xDC;
pub const MOONGATE_ANIMATION_FRAMES: u8 = 1;
pub const NATURAL_MOONGATE_TERRAIN_TILE: u8 = 0xDC;
pub const NATURAL_MOONGATE_RESTORED_TERRAIN_TILE: u8 = 5;
pub const NATURAL_MOONGATE_COUNTER_MAX: u8 = 16;
pub const STEADY_PHASE: u8 = 0x0f;
pub const PLAY_START_YEAR: u16 = 139;
pub const PLAY_START_MONTH: u8 = 4;
pub const PLAY_START_DAY: u8 = 5;
pub const PLAY_START_HOUR: u8 = 12;
pub const SAVED_GAM_LEN: usize = 4192;
/// `formats/saved-gam.md §2` top-level layout: leading two bytes
/// precede the roster.
pub const SAVE_LEADING_BYTES_LEN: usize = 2;
/// `formats/saved-gam.md §3`: number of character-record slots in the
/// roster. Anchored to [`SAVE_ROSTER_SLOT_COUNT`] so the two
/// parallel names for the same sixteen-slot character region share
/// one source of truth.
pub const SAVE_CHARACTER_ROSTER_SLOTS: usize = SAVE_ROSTER_SLOT_COUNT;
/// `formats/saved-gam.md §4`: party-size byte range (`1..=6`).
pub const SAVE_PARTY_SIZE_MIN: u8 = 1;
pub const SAVE_PARTY_SIZE_MAX: u8 = 6;
pub const SAVE_FOOD_STOCK_OFFSET: usize = 0x0202;
pub const SAVE_GOLD_STOCK_OFFSET: usize = 0x0204;
pub const SAVE_KEY_STOCK_OFFSET: usize = 0x0206;
pub const SAVE_GEM_STOCK_OFFSET: usize = 0x0207;
pub const SAVE_TORCH_STOCK_OFFSET: usize = 0x0208;
pub const SAVE_CLIMBING_GEAR_OFFSET: usize = 0x0209;
pub const SAVE_SPECIAL_ITEM_OFFSET: usize = 0x020a;
pub const SAVE_EQUIPMENT_STOCK_OFFSET: usize = 0x021a;
pub const SAVE_SPELL_CHARGES_OFFSET: usize = 0x024a;
pub const SAVE_SCROLL_STOCK_OFFSET: usize = 0x027a;
pub const SAVE_POTION_STOCK_OFFSET: usize = 0x0282;
pub const SAVE_MOONSTONE_X_OFFSET: usize = 0x028a;
pub const SAVE_MOONSTONE_Y_OFFSET: usize = 0x0292;
pub const SAVE_MOONSTONE_SCENE_OFFSET: usize = 0x029a;
pub const SAVE_MOONSTONE_Z_OFFSET: usize = 0x02a2;
pub const SAVE_REAGENTS_OFFSET: usize = 0x02aa;
pub const SAVE_YEAR_OFFSET: usize = 0x02ce;
pub const SAVE_TIMING_STATUS_TAG_OFFSET: usize = 0x02d4;
pub const SAVE_ACTIVE_PLAYER_OFFSET: usize = 0x02d5;
pub const SAVE_TRANSPORT_MARKER_OFFSET: usize = 0x02d6;
pub const SAVE_MONTH_OFFSET: usize = 0x02d7;
pub const SAVE_DAY_OFFSET: usize = 0x02d8;
pub const SAVE_HOUR_OFFSET: usize = 0x02d9;
/// `formats/saved-gam.md §5`: adjacent saved-hour snapshot byte the
/// per-turn cleanup uses to detect hour crossings. Not the active
/// hour; preserve byte-for-byte on round trip.
pub const SAVE_SAVED_HOUR_SNAPSHOT_OFFSET: usize = 0x02da;
pub const SAVE_MINUTE_OFFSET: usize = 0x02db;
pub const SAVE_COMBAT_ROUND_COUNTER_OFFSET: usize = 0x02dc;
/// `formats/saved-gam.md §5` adjacent per-turn state byte; preserve
/// byte-for-byte but no public calendar meaning.
pub const SAVE_PER_TURN_STATE_OFFSET: usize = 0x02dd;
pub const SAVE_AMPM_DISPLAY_OFFSET: usize = 0x02de;
/// `formats/saved-gam.md §5`: in-game calendar bounds. Months are
/// one-based 1..=13 (thirteen 28-day months per year), days are
/// one-based 1..=28, hours are zero-based 0..=23, minutes 0..=59.
pub const SAVE_MONTH_MIN: u8 = 1;
pub const SAVE_MONTH_MAX: u8 = 13;
pub const SAVE_DAY_MIN: u8 = 1;
pub const SAVE_DAY_MAX: u8 = 28;
pub const SAVE_HOUR_MAX: u8 = 23;
pub const SAVE_MINUTE_MAX: u8 = 59;
pub const SAVE_MORAL_STANDING_OFFSET: usize = 0x02e2;
pub const SAVE_WIND_OFFSET: usize = 0x02ec;
pub const SAVE_SCENE_OFFSET: usize = 0x02ed;
pub const SAVE_Z_OFFSET: usize = 0x02ef;
pub const SAVE_X_OFFSET: usize = 0x02f0;
pub const SAVE_Y_OFFSET: usize = 0x02f1;
pub const SAVE_LIGHT_SPELL_COUNTER_OFFSET: usize = 0x0300;
pub const SAVE_TORCH_COUNTER_OFFSET: usize = 0x0301;
pub const SAVE_SHRINE_ORDAINED_MASK_OFFSET: usize = 0x0326;
pub const SAVE_SHRINE_CODEX_MASK_OFFSET: usize = 0x0328;
pub const SAVE_FORTUNES_OF_WAR_OFFSET: usize = 0x03b3;
/// `formats/saved-gam.md §10`: durable dungeon room-clear bitmap. The
/// 16-byte block at `0x033A..0x0349` records which dungeon room
/// encounters have already been cleared; dungeon mode uses it to
/// demote matching `0xF?` room-trigger cells to `0xA?` room-helper
/// cells when rebuilding the loaded dungeon image from `DUNGEON.DAT`.
pub const SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET: usize = 0x033A;
pub const SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN: usize = 16;
/// `formats/saved-gam.md §8.1`: the live active-object table snapshot
/// occupies 256 bytes at file offset `0x06B4..=0x07B3`. Layout matches
/// the in-memory table (32 records × 8 bytes).
pub const SAVE_ACTIVE_OBJECT_TABLE_OFFSET: usize = 0x06B4;
/// `formats/saved-gam.md §8.2`: the dungeon/map-cell working buffer
/// occupies 512 bytes at file offset `0x03B4..=0x05B3` and matches the
/// 512-byte dungeon-record stride.
pub const SAVE_DUNGEON_WORKING_BUFFER_OFFSET: usize = 0x03B4;
pub const SAVE_DUNGEON_WORKING_BUFFER_LEN: usize = 512;
/// `formats/saved-gam.md §10`: active-player sentinel value when no
/// party member is currently selected to move. The byte holds an
/// integer slot index when one is selected.
pub const SAVE_ACTIVE_PLAYER_NONE: u8 = 0xFF;
pub const SAVE_AVATAR_NAME_OFFSET: usize = 0x0002;
/// `formats/saved-gam.md §7` shared inventory block offsets.
pub const SAVE_FOOD_OFFSET: usize = 0x0202;
pub const SAVE_GOLD_OFFSET: usize = 0x0204;
pub const SAVE_KEYS_OFFSET: usize = 0x0206;
pub const SAVE_GEMS_OFFSET: usize = 0x0207;
pub const SAVE_TORCHES_OFFSET: usize = 0x0208;
pub const SAVE_GRAPPLE_OFFSET: usize = 0x0209;
pub const SAVE_EQUIPMENT_INVENTORY_OFFSET: usize = 0x021A;
pub const SAVE_SPELL_CHARGE_BLOCK_OFFSET: usize = 0x024A;
pub const SAVE_SCROLL_COUNTERS_OFFSET: usize = 0x027A;
pub const SAVE_POTION_COUNTERS_OFFSET: usize = 0x0282;
/// `formats/saved-gam.md §6` location-cluster scratch offsets.
pub const SAVE_SAVED_SCENE_SCRATCH_OFFSET: usize = 0x02EE;
pub const SAVE_PARTY_Z_OFFSET: usize = 0x02EF;
pub const SAVE_PARTY_X_OFFSET: usize = 0x02F0;
pub const SAVE_PARTY_Y_OFFSET: usize = 0x02F1;
/// `formats/saved-gam.md §6`: party Z `0xFF` is the "no active map"
/// sentinel.
pub const SAVE_PARTY_Z_NO_ACTIVE_MAP: u8 = 0xFF;
pub const SAVE_AVATAR_NAME_LEN: usize = 9;
pub const SAVE_ACTIVE_OBJECTS_OFFSET: usize = 0x06b4;
/// `formats/saved-gam.md §12`: 2,220-byte reserved tail at file
/// offsets `0x07B4..=0x105F` that follows the active-object table.
/// In memory the region holds the NPC schedule blob, NPC runtime
/// state, NPC path queues, and the world-tile render buffer — all
/// repopulated from the location's NPC files and the active-map
/// loader on map entry. The bytes are transient for gameplay, but
/// byte-compatible save editors must preserve them so unknown bytes
/// survive a rewrite of an existing save. Promote the offset and
/// length so the tail span has one named source of truth.
pub const SAVE_RESERVED_TAIL_OFFSET: usize = 0x07B4;
pub const SAVE_RESERVED_TAIL_LEN: usize = 2_220;
pub const SAVE_PARTY_SIZE_OFFSET: usize = 0x02b5;
pub const SAVE_ROSTER_OFFSET: usize = 0x0002;
/// `formats/saved-gam.md §3`: number of character records the roster
/// holds. Record zero is structurally the Avatar; records one through
/// fifteen are the canonical companion list. Slots beyond the
/// `party-size` index hold characters who exist in Britannia but are
/// not currently travelling with the player.
pub const SAVE_ROSTER_SLOT_COUNT: usize = 16;
pub const SAVE_INN_REGISTRY_OFFSET: usize = 0x0021;
/// `formats/saved-gam.md §3` / `shops.md §8.4`: the inn registry
/// is "a 16-slot, save-backed resident view... a shifted legacy
/// view over the save image rather than an independent
/// post-roster block." Its slot count is the same sixteen slots
/// the character roster carries. Anchored to
/// [`SAVE_ROSTER_SLOT_COUNT`] so the registry and the roster
/// share one source of truth.
pub const SAVE_INN_REGISTRY_COUNT: usize = SAVE_ROSTER_SLOT_COUNT;
pub const SAVE_CHARACTER_RECORD_LEN: usize = 32;
/// `formats/saved-gam.md §3` total roster region length:
/// sixteen records of thirty-two bytes each.
pub const SAVE_ROSTER_REGION_LEN: usize = SAVE_ROSTER_SLOT_COUNT * SAVE_CHARACTER_RECORD_LEN;
pub const SAVE_CHARACTER_NAME_LEN: usize = 9;
pub const SAVE_CHARACTER_GENDER_OFFSET: usize = 0x09;
pub const SAVE_GENDER_MALE_BYTE: u8 = 0x0b;
pub const SAVE_GENDER_FEMALE_BYTE: u8 = 0x0c;
pub const SAVE_CHARACTER_CLASS_OFFSET: usize = 0x0a;
pub const SAVE_CHARACTER_STATUS_OFFSET: usize = 0x0b;
pub const SAVE_CHARACTER_STR_OFFSET: usize = 0x0c;
pub const SAVE_CHARACTER_DEX_OFFSET: usize = 0x0d;
pub const SAVE_CHARACTER_INT_OFFSET: usize = 0x0e;
pub const SAVE_CHARACTER_MANA_OFFSET: usize = 0x0f;
pub const SAVE_CHARACTER_HP_OFFSET: usize = 0x10;
pub const SAVE_CHARACTER_MAX_HP_OFFSET: usize = 0x12;
pub const SAVE_CHARACTER_EXPERIENCE_OFFSET: usize = 0x14;
pub const SAVE_CHARACTER_LEVEL_OFFSET: usize = 0x16;
pub const SAVE_CHARACTER_STAY_COUNTER_OFFSET: usize = 0x17;
pub const SAVE_CHARACTER_EQUIPMENT_OFFSET: usize = 0x19;
/// `formats/tiles.md §1,§5.1`: 16x16 tile pixel side and the flat
/// atlas's 512-entry capacity.
pub const TILE_PIXEL_SIDE: usize = 16;
pub const FLAT_TILE_ATLAS_TILES: usize = 512;
/// `formats/tiles.md §3` EGA pixel-packing density. The `.16` files
/// store two four-bit pixels per byte (chunky packed, high nibble
/// first); the pixel's index is its position in the 16-entry EGA
/// palette.
pub const EGA_PIXELS_PER_BYTE: usize = 2;
/// `formats/tiles.md §4` CGA pixel-packing density. The `.4` files
/// store four two-bit pixels per byte (packed, most-significant bits
/// first); the pixel's index is its position in the 4-entry CGA
/// palette set by the display driver.
pub const CGA_PIXELS_PER_BYTE: usize = 4;
/// `formats/tiles.md §3`: each EGA tile costs 128 bytes
/// (`TILE_PIXEL_SIDE * TILE_PIXEL_SIDE / EGA_PIXELS_PER_BYTE`).
pub const EGA_TILE_BYTES: usize = TILE_PIXEL_SIDE * TILE_PIXEL_SIDE / EGA_PIXELS_PER_BYTE;
/// `formats/tiles.md §4`: each CGA tile costs 64 bytes
/// (`TILE_PIXEL_SIDE * TILE_PIXEL_SIDE / CGA_PIXELS_PER_BYTE`).
pub const CGA_TILE_BYTES: usize = TILE_PIXEL_SIDE * TILE_PIXEL_SIDE / CGA_PIXELS_PER_BYTE;
/// `formats/tiles.md §3,§4` total uncompressed flat-atlas size in
/// bytes per encoding.
pub const EGA_FLAT_TILE_ATLAS_BYTES: usize = FLAT_TILE_ATLAS_TILES * EGA_TILE_BYTES;
pub const CGA_FLAT_TILE_ATLAS_BYTES: usize = FLAT_TILE_ATLAS_TILES * CGA_TILE_BYTES;

pub const SPELL_COUNT: usize = 48;
/// `magic.md §4`: there are eight magic circles.
pub const SPELL_CIRCLE_COUNT: usize = 8;
/// `magic.md §4`: each circle holds six spells.
pub const SPELLS_PER_CIRCLE: usize = 6;
pub const EQUIPMENT_COUNT: usize = 48;
/// `inventory.md §7` U-Use scroll catalog size. The eight scroll
/// indices span LIGHT (0) through NEGATE_TIME (7); anchor the
/// count to [`SCROLL_NEGATE_TIME_INDEX`] + 1 so adding or
/// renaming a scroll only happens in one place.
pub const SCROLL_COUNT: usize = SCROLL_NEGATE_TIME_INDEX + 1;

/// `inventory.md §7` U-Use scroll display labels in storage order.
/// `formats/saved-gam.md §7`: the per-scroll counters at
/// `0x027A..0x0281` are eight bytes, one per scroll row. The label
/// strings are the compact letter-coded spell selectors a player
/// would type for the matching C-Cast spell (Vas Lor, Rel Hur,
/// In Sanct, An In, In Quas Wis, Kal Xen Corp, In Mani Corp, An
/// Tym), in the U-Use scroll-dispatch order.
pub const SCROLL_SPELL_LABELS: [&str; SCROLL_COUNT] = [
    "LV", "HR", "IS", "AI", "IQW", "CKX", "CIM", "AT",
];
/// `inventory.md §7` potion catalog size. The eight potion
/// indices span BLUE (0) through WHITE (7); anchor the count to
/// [`POTION_WHITE_INDEX`] + 1 so adding or renaming a potion
/// only happens in one place.
pub const POTION_COUNT: usize = POTION_WHITE_INDEX + 1;
pub const POTION_BLUE_INDEX: usize = 0;
pub const POTION_YELLOW_INDEX: usize = 1;
pub const POTION_RED_INDEX: usize = 2;
pub const POTION_GREEN_INDEX: usize = 3;
pub const POTION_ORANGE_INDEX: usize = 4;
pub const POTION_PURPLE_INDEX: usize = 5;
pub const POTION_BLACK_INDEX: usize = 6;
pub const POTION_WHITE_INDEX: usize = 7;

/// `catalogs/item-list.md §8` Sceptre of Lord British dissolves
/// the top-down barrier/field family `0x70..=0x7F` into ordinary
/// open ground `0x44` (cobble). The U-Use scan walks the
/// party-centered nearby square and rewrites each accepted cell
/// in place with the redraw / effect presentation.
pub const SCEPTRE_BARRIER_TILE_FIRST: u8 = 0x70;
pub const SCEPTRE_BARRIER_TILE_LAST: u8 = 0x7F;
pub const SCEPTRE_BARRIER_DISSOLVED_TILE: u8 = 0x44;

/// `catalogs/item-list.md §7.2` White-potion surface visibility-sweep
/// frame count. In overworld and named interior scenes the white
/// potion runs a twenty-frame visibility/animation sweep centred on
/// the party with radius [`POTION_WHITE_SWEEP_RADIUS`] before
/// finishing with a normal world redraw. Dungeon and combat scenes
/// take the no-noticeable-effect branch instead.
pub const POTION_WHITE_SWEEP_FRAMES: u8 = 20;
pub const POTION_WHITE_SWEEP_RADIUS: u8 = 32;
/// `endgame.md §2` total Shadow Lord count. The three named
/// indices span FALSEHOOD (0) through COWARDICE (2); anchor the
/// count to [`SHADOWLORD_COWARDICE_INDEX`] + 1 so adding or
/// renaming a Shadow Lord only happens in one place.
pub const SHADOWLORD_COUNT: usize = SHADOWLORD_COWARDICE_INDEX + 1;
pub const SHADOWLORD_FALSEHOOD_INDEX: usize = 0;
pub const SHADOWLORD_HATRED_INDEX: usize = 1;
pub const SHADOWLORD_COWARDICE_INDEX: usize = 2;
pub const SHADOWLORD_HIDEOUT_MIN: u8 = 1;
pub const SHADOWLORD_HIDEOUT_MAX: u8 = 8;
pub const SHADOWLORD_VANQUISHED: u8 = 0xff;
pub const DEFAULT_SHADOWLORD_HIDEOUTS: [u8; SHADOWLORD_COUNT] = [1, 2, 3];
pub const SHADOWLORD_OBJECT_TILE_BASE: u8 = 0xfd;
pub const SCROLL_LIGHT_INDEX: usize = 0;
pub const SCROLL_WIND_CHANGE_INDEX: usize = 1;
pub const SCROLL_PROTECTION_INDEX: usize = 2;
pub const SCROLL_NEGATE_MAGIC_INDEX: usize = 3;
pub const SCROLL_VIEW_INDEX: usize = 4;
pub const SCROLL_SUMMON_DAEMON_INDEX: usize = 5;
pub const SCROLL_RESURRECTION_INDEX: usize = 6;
pub const SCROLL_NEGATE_TIME_INDEX: usize = 7;
pub const SCROLL_LIGHT_DURATION: u8 = 240;
pub const SCROLL_PROTECTION_DURATION: u8 = 100;
pub const SCROLL_NEGATE_MAGIC_DURATION: u8 = 20;
pub const SCROLL_NEGATE_TIME_DURATION: u8 = 20;
/// `catalogs/item-list.md §6` total special-item catalog size.
/// The named indices span MAGIC_CARPET (0x00) through WOODEN_BOX
/// (0x0F); anchor the count to [`SPECIAL_ITEM_WOODEN_BOX_INDEX`]
/// + 1 so adding a special item only happens in one place.
pub const SPECIAL_ITEM_COUNT: usize = SPECIAL_ITEM_WOODEN_BOX_INDEX + 1;
pub const SPECIAL_ITEM_MAGIC_CARPET_INDEX: usize = 0x00;
pub const SPECIAL_ITEM_SKULL_KEY_INDEX: usize = 0x01;
pub const SPECIAL_ITEM_AMULET_LB_INDEX: usize = 0x03;
pub const SPECIAL_ITEM_CROWN_LB_INDEX: usize = 0x04;
pub const SPECIAL_ITEM_SCEPTRE_LB_INDEX: usize = 0x05;
pub const SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX: usize = 0x06;
pub const SPECIAL_ITEM_SHARD_HATRED_INDEX: usize = 0x07;
pub const SPECIAL_ITEM_SHARD_COWARDICE_INDEX: usize = 0x08;
pub const SPECIAL_ITEM_SPYGLASS_INDEX: usize = 0x0a;
pub const SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX: usize = 0x0b;
pub const SPECIAL_ITEM_SEXTANT_INDEX: usize = 0x0c;
pub const SPECIAL_ITEM_POCKET_WATCH_INDEX: usize = 0x0d;
pub const SPECIAL_ITEM_BLACK_BADGE_INDEX: usize = 0x0e;
pub const SPECIAL_ITEM_WOODEN_BOX_INDEX: usize = 0x0f;
pub const SPECIAL_ITEM_OWNED_VALUE: u8 = 1;
pub const SPECIAL_ITEM_WORN_VALUE: u8 = 2;
pub const LORD_BLACKTHORN_CASTLE_SCENE_BYTE: u8 = 18;
pub const STONEGATE_SCENE_BYTE: u8 = 29;
pub const DOOM_DUNGEON_RECORD: usize = 7;
/// `endgame.md §2` Doom final-room trigger lives on the deepest
/// dungeon level (`Z == DUNGEON_DEEPEST_LEVEL`). Anchor the level
/// constant to tile_helpers.rs so the Doom-final and dungeon-floor
/// edges share one source of truth.
pub const DOOM_FINAL_ROOM_LEVEL: u8 = crate::DUNGEON_DEEPEST_LEVEL;
pub const DOOM_FINAL_ROOM_X: usize = 5;
pub const DOOM_FINAL_ROOM_Y: usize = 7;
/// `endgame.md §2` Doom final-room slot id. The room-trigger low
/// nibble is fifteen — the high end of the per-bank slot range
/// promoted as DUNGEON_ROOM_SLOT_MASK.
pub const DOOM_FINAL_ROOM_SLOT: u8 = crate::DUNGEON_ROOM_SLOT_MASK;
/// `inventory.md §3` per-character equipment slot count. The six
/// slot indices `EQUIP_SLOT_HELM .. EQUIP_SLOT_AMULET` (0..=5)
/// cover the entire equip-slot family. Anchored to
/// [`EQUIP_SLOT_AMULET`] + 1 so adding or renaming a slot only
/// happens in one place.
pub const EQUIPMENT_SLOT_COUNT: usize = EQUIP_SLOT_AMULET + 1;
pub const EQUIPMENT_EMPTY: u8 = 0xff;
pub const EQUIPMENT_STOCK_CAP: u8 = 99;
/// `inventory.md §2`: word-sized party gold counter caps at 9999 in
/// ordinary play. Storage width is two bytes; do not infer a 65535
/// gameplay cap from the byte width.
pub const PARTY_GOLD_CAP: u16 = 9999;
/// `inventory.md §2`: word-sized party food counter caps at 9999 in
/// ordinary play.
pub const PARTY_FOOD_CAP: u16 = 9999;
/// `inventory.md §2`: byte-sized party spell-charge counter caps at
/// 99 in ordinary play.
pub const SPELL_CHARGE_CAP: u8 = 99;
/// `inventory.md §2`: equipment stock band has 48 entries (item ids
/// `0..=47`) — one counter per equipment id. Anchored to
/// [`EQUIPMENT_COUNT`] so the catalog size and the carrier band
/// length stay one value.
pub const EQUIPMENT_STOCK_BAND_LEN: usize = EQUIPMENT_COUNT;
/// `inventory.md §2`: spell-charge band has 48 entries (one per
/// spell id `0..=47`). Anchored to [`SPELL_COUNT`] so the spell
/// catalog size and the per-spell charge band stay one value.
pub const SPELL_CHARGE_BAND_LEN: usize = SPELL_COUNT;
pub const EQUIP_SLOT_HELM: usize = 0;
pub const EQUIP_SLOT_ARMOUR: usize = 1;
pub const EQUIP_SLOT_WEAPON: usize = 2;
pub const EQUIP_SLOT_OFFHAND: usize = 3;
pub const EQUIP_SLOT_RING: usize = 4;
pub const EQUIP_SLOT_AMULET: usize = 5;
pub const EQUIPMENT_TAG_AMMO: u8 = 0x00;
pub const EQUIPMENT_TAG_RING: u8 = 0x02;
pub const EQUIPMENT_TAG_AMULET: u8 = 0x04;
pub const EQUIPMENT_TAG_ONE_HAND: u8 = 0x20;
pub const EQUIPMENT_TAG_TWO_HAND: u8 = 0x30;
pub const EQUIPMENT_TAG_ARMOUR: u8 = 0x40;
pub const EQUIPMENT_TAG_HELM: u8 = 0x80;
pub const EQUIPMENT_ID_BOW: usize = 26;
pub const EQUIPMENT_ID_ARROWS: usize = 27;
pub const EQUIPMENT_ID_CROSSBOW: usize = 28;
pub const EQUIPMENT_ID_QUARRELS: usize = 29;
pub const EQUIPMENT_ID_MAGIC_BOW: usize = 36;
pub const EQUIPMENT_ID_RING_INVISIBILITY: usize = 42;
pub const EQUIPMENT_ID_RING_REGENERATION: usize = 44;
pub const EQUIPMENT_ID_AMULET_TURNING: usize = 45;
/// `catalogs/spell-list.md §3` / `inventory.md §2` total reagent
/// catalog size. The reagent indices span SULFUR_ASH (0) through
/// MANDRAKE (7); anchor the count to [`REAGENT_MANDRAKE`] + 1 so
/// adding or renaming a reagent only happens in one place.
pub const REAGENT_COUNT: usize = REAGENT_MANDRAKE + 1;
pub const VIRTUE_COUNT: usize = 8;
/// `karma.md §1`: per-virtue shrine standing caps at ninety-nine.
/// The traced moral-standing selector ([`MORAL_STANDING_MAX`])
/// and per-virtue shrine standings share the same byte cap;
/// anchor here so both move as one value.
pub const SHRINE_STANDING_MAX: u8 = MORAL_STANDING_MAX;
/// `karma.md §3`: the moral-standing selector caps at ninety-nine
/// across all add paths (NPC thank-you, toll milestone, etc.).
pub const MORAL_STANDING_MAX: u8 = 99;
pub const AVATAR_STAT_MAX: u8 = 30;
pub const REAGENT_SULFUR_ASH: usize = 0;
pub const REAGENT_GINSENG: usize = 1;
pub const REAGENT_GARLIC: usize = 2;
pub const REAGENT_SPIDER_SILK: usize = 3;
pub const REAGENT_BLOOD_MOSS: usize = 4;
pub const REAGENT_BLACK_PEARL: usize = 5;
pub const REAGENT_NIGHTSHADE: usize = 6;
pub const REAGENT_MANDRAKE: usize = 7;
pub const RARE_REAGENT_HARVEST_POINT_COUNT: usize = 3;
pub const RARE_REAGENT_HARVEST_UNSEEN_DAY: u8 = 0;
pub const FIXED_HIDDEN_TREASURE_COUNT: usize = 113;
pub const FIXED_HIDDEN_TREASURE_FOUND_BYTES: usize = 15;
pub const FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY: u8 = 0;
pub const FIXED_HIDDEN_TREASURE_OBJECT_TILE: u8 = 0x1f;
pub const FIXED_HIDDEN_TREASURE_OBJECT_AUX3: u8 = 0xa5;
/// `catalogs/spell-list.md §3` per-reagent recipe-mask bit. The mix
/// command builds a one-byte recipe mask by OR-ing these bits for the
/// selected reagents and comparing against the spell's resident
/// recipe byte. Bit `0x80` selects Sulfur Ash (index 0), and each
/// subsequent reagent uses the next lower bit (`0x40` Ginseng,
/// `0x20` Garlic, `0x10` Spider Silk, `0x08` Blood Moss, `0x04`
/// Black Pearl, `0x02` Nightshade, `0x01` Mandrake).
pub const REAGENT_MASK_SULFUR_ASH: u8 = 0x80;
pub const REAGENT_MASK_GINSENG: u8 = 0x40;
pub const REAGENT_MASK_GARLIC: u8 = 0x20;
pub const REAGENT_MASK_SPIDER_SILK: u8 = 0x10;
pub const REAGENT_MASK_BLOOD_MOSS: u8 = 0x08;
pub const REAGENT_MASK_BLACK_PEARL: u8 = 0x04;
pub const REAGENT_MASK_NIGHTSHADE: u8 = 0x02;
pub const REAGENT_MASK_MANDRAKE: u8 = 0x01;
pub const REAGENT_MASKS: [u8; REAGENT_COUNT] = [
    REAGENT_MASK_SULFUR_ASH,
    REAGENT_MASK_GINSENG,
    REAGENT_MASK_GARLIC,
    REAGENT_MASK_SPIDER_SILK,
    REAGENT_MASK_BLOOD_MOSS,
    REAGENT_MASK_BLACK_PEARL,
    REAGENT_MASK_NIGHTSHADE,
    REAGENT_MASK_MANDRAKE,
];
pub const REAGENT_SAVE_ORDER: [usize; REAGENT_COUNT] = [
    REAGENT_BLACK_PEARL,
    REAGENT_BLOOD_MOSS,
    REAGENT_GARLIC,
    REAGENT_GINSENG,
    REAGENT_MANDRAKE,
    REAGENT_NIGHTSHADE,
    REAGENT_SPIDER_SILK,
    REAGENT_SULFUR_ASH,
];
pub const DEFAULT_REAGENTS: [u8; REAGENT_COUNT] = [0, 6, 7, 0, 6, 4, 3, 0];
pub const IN_LOR_SPELL_INDEX: usize = 0;
pub const IN_LOR_COST: u8 = 1;
pub const IN_LOR_LIGHT_DURATION: u8 = 100;
pub const AWAKEN_SPELL_INDEX: usize = 2;
pub const AWAKEN_COST: u8 = 1;
pub const CURE_SPELL_INDEX: usize = 3;
pub const CURE_COST: u8 = 1;
pub const HEAL_SPELL_INDEX: usize = 4;
pub const HEAL_COST: u8 = 1;
pub const HEAL_RAW_ROLL_MAX: u8 = 60;
pub const VANISH_SPELL_INDEX: usize = 5;
pub const VANISH_COST: u8 = 1;
pub const OPEN_SPELL_INDEX: usize = 6;
pub const OPEN_SPELL_COST: u8 = 2;
pub const REL_HUR_SPELL_INDEX: usize = 8;
pub const REL_HUR_COST: u8 = 2;
pub const IN_WIS_SPELL_INDEX: usize = 9;
pub const IN_WIS_COST: u8 = 2;
pub const CREATE_FOOD_SPELL_INDEX: usize = 11;
pub const CREATE_FOOD_COST: u8 = 2;
pub const CREATE_FOOD_AMOUNT: u16 = 100;
pub const VAS_LOR_SPELL_INDEX: usize = 12;
pub const VAS_LOR_COST: u8 = 3;
pub const VAS_LOR_LIGHT_DURATION: u8 = 255;
pub const FIRE_FIELD_SPELL_INDEX: usize = 14;
pub const POISON_FIELD_SPELL_INDEX: usize = 15;
pub const SLEEP_FIELD_SPELL_INDEX: usize = 16;
pub const FIELD_SPELL_COST: u8 = 3;
pub const BLINK_SPELL_INDEX: usize = 17;
pub const BLINK_COST: u8 = 3;
pub const DISPEL_FIELD_SPELL_INDEX: usize = 18;
pub const DISPEL_FIELD_COST: u8 = 4;
pub const PROTECTION_SPELL_INDEX: usize = 19;
pub const PROTECTION_COST: u8 = 4;
pub const PROTECTION_ACTIVE_EFFECT_TAG: u8 = b'P';
pub const PROTECTION_ACTIVE_EFFECT_DURATION: u8 = 20;
pub const UUS_POR_SPELL_INDEX: usize = 21;
pub const DES_POR_SPELL_INDEX: usize = 22;
pub const DUNGEON_LEVEL_SPELL_COST: u8 = 4;
pub const REVEAL_SPELL_INDEX: usize = 23;
pub const REVEAL_COST: u8 = 4;
pub const ENERGY_FIELD_SPELL_INDEX: usize = 20;
pub const ENERGY_FIELD_COST: u8 = 4;
pub const MAGIC_LOCK_SPELL_INDEX: usize = 25;
pub const MAGIC_LOCK_COST: u8 = 5;
pub const UNLOCK_MAGIC_SPELL_INDEX: usize = 26;
pub const UNLOCK_MAGIC_COST: u8 = 5;
pub const GREAT_HEAL_SPELL_INDEX: usize = 27;
pub const GREAT_HEAL_COST: u8 = 5;
pub const QUICKNESS_SPELL_INDEX: usize = 29;
pub const QUICKNESS_COST: u8 = 5;
pub const QUICKNESS_ACTIVE_EFFECT_TAG: u8 = b'Q';
pub const QUICKNESS_ACTIVE_EFFECT_DURATION: u8 = 30;
pub const MASS_CHARM_SPELL_INDEX: usize = 31;
pub const MASS_CHARM_COST: u8 = 6;
pub const MASS_CHARM_ACTIVE_EFFECT_TAG: u8 = b'C';
pub const MASS_CHARM_ACTIVE_EFFECT_DURATION: u8 = 20;
pub const NEGATE_MAGIC_SPELL_INDEX: usize = 32;
pub const NEGATE_MAGIC_COST: u8 = 6;
pub const NEGATE_MAGIC_ACTIVE_EFFECT_TAG: u8 = b'N';
pub const NEGATE_MAGIC_ACTIVE_EFFECT_DURATION: u8 = 10;
pub const X_RAY_SPELL_INDEX: usize = 33;
pub const X_RAY_COST: u8 = 6;
pub const INVISIBILITY_SPELL_INDEX: usize = 36;
pub const INVISIBILITY_COST: u8 = 7;
pub const CAUSE_FEAR_SPELL_INDEX: usize = 41;
pub const CAUSE_FEAR_COST: u8 = 7;
pub const PEER_SPELL_INDEX: usize = 39;
pub const PEER_COST: u8 = 7;
pub const RESURRECT_SPELL_INDEX: usize = 42;
pub const RESURRECT_COST: u8 = 8;
pub const SUMMON_DAEMON_SPELL_INDEX: usize = 43;
pub const GATE_TRAVEL_SPELL_INDEX: usize = 46;
pub const GATE_TRAVEL_COST: u8 = 8;
pub const TIME_STOP_SPELL_INDEX: usize = 47;
pub const TIME_STOP_COST: u8 = 8;
pub const TIME_STOP_DURATION: u8 = 10;
pub const NEGATE_TIME_ACTIVE_EFFECT_TAG: u8 = b'T';

// Combat-side raw damage caps for single-target damage spells per
// `catalogs/spell-list.md` §5. The instant-kill sentinel itself lives in
// `combat_actor` because the damage helpers there compare it as `i16`.
pub const MAGIC_MISSILE_SPELL_INDEX: usize = 1;
pub const MAGIC_MISSILE_RAW_DAMAGE_MAX: u8 = 16;
pub const FIREBALL_SPELL_INDEX: usize = 13;
pub const FIREBALL_RAW_DAMAGE_MAX: u8 = 30;
pub const KILL_SPELL_INDEX: usize = 37;
/// Fire-Field per-actor raw damage roll cap per `combat.md` §11. Energy
/// Field supplies raw zero to the same damage path; that case has no cap.
pub const FIRE_FIELD_RAW_DAMAGE_MAX: u8 = 21;

/// Inclusive town/world door tile-id range per `catalogs/tile-catalog.md` §6:
/// indices `96..=103` are the door family used by O-Open / J-Jimmy / magic
/// Open. Open variants written by the O command live in this range alongside
/// the closed forms.
pub const TOWN_DOOR_TILE_FIRST: u8 = 96;
pub const TOWN_DOOR_TILE_LAST: u8 = 103;
/// Inclusive town stair tile-id range per `catalogs/tile-catalog.md` §6:
/// `0xC4..=0xC7` is the facing-sensitive stairway family whose low two bits
/// encode movement-wrapper-normalised facing.
pub const TOWN_STAIR_TILE_FIRST: u8 = 0xC4;
pub const TOWN_STAIR_TILE_LAST: u8 = 0xC7;
/// Town chair trigger tile per `catalogs/tile-catalog.md` §6.
pub const TOWN_CHAIR_TILE: u8 = 0x8C;
/// NPC floor-link marker tiles consumed by the schedule pathfinder per
/// `catalogs/tile-catalog.md` §6.
pub const NPC_FLOOR_LINK_TILE_A: u8 = 0xC8;
pub const NPC_FLOOR_LINK_TILE_B: u8 = 0xC9;

/// Save-image ship transport marker ranges per `vehicles.md` §6 / Ship
/// Sails. Hoisted/wind-control ships use `0x20..=0x23`; furled/manual ships
/// use `0x24..=0x27`. In both ranges the low two bits encode heading as
/// north (0), east (1), south (2), west (3). These are save-image transport
/// bytes, not visual tile ids.
pub const SHIP_TRANSPORT_HOISTED_FIRST: u8 = 0x20;
pub const SHIP_TRANSPORT_HOISTED_LAST: u8 = 0x23;
pub const SHIP_TRANSPORT_FURLED_FIRST: u8 = 0x24;
pub const SHIP_TRANSPORT_FURLED_LAST: u8 = 0x27;
/// Carpet transport markers `0x14..=0x17` per `vehicles.md` §2/§4: the low
/// two bits encode the carpet's facing on the same N/E/S/W convention used
/// by ships. Only the north (`0x14`) and east (`0x15`) markers are accepted
/// by the ship boarding precondition described in §4.
pub const CARPET_TRANSPORT_FIRST: u8 = 0x14;
pub const CARPET_TRANSPORT_LAST: u8 = 0x17;
/// Horse object byte range `0x10..=0x11` (riderless) and `0x12..=0x13`
/// (mounted). Boarding adds 2 to the riderless object byte to produce the
/// mounted marker per `vehicles.md` §4.
pub const HORSE_OBJECT_FIRST: u8 = 0x10;
pub const HORSE_OBJECT_LAST: u8 = 0x11;
pub const HORSE_TRANSPORT_FIRST: u8 = 0x12;
pub const HORSE_TRANSPORT_LAST: u8 = 0x13;

/// Active-object slot allocator boundaries per `active-objects.md` §4.
/// Slot 0 is the canonical player slot; the ordinary acquisition path
/// searches slots `1..=23`; slots `24..=31` are reserved for setup paths
/// outside the allocator. Byte-0 value `0xB5` is universally protected from
/// eviction (Grendel/monster-variant actor class).
pub const ACTIVE_OBJECT_PLAYER_SLOT: usize = 0;
pub const ACTIVE_OBJECT_ORDINARY_FIRST: usize = 1;
pub const ACTIVE_OBJECT_ORDINARY_LAST: usize = 23;
pub const ACTIVE_OBJECT_RESERVED_FIRST: usize = 24;
pub const ACTIVE_OBJECT_RESERVED_LAST: usize = 31;
/// Universally protected byte-0 value: never an eviction victim, never
/// recycled by the slot allocator's last-resort phase.
pub const ACTIVE_OBJECT_PROTECTED_TYPE_BYTE: u8 = 0xB5;
/// `active-objects.md` §4 off-screen test radius. A candidate more than
/// roughly five cells from the player in either axis is eligible for the
/// off-screen eviction phases.
pub const ACTIVE_OBJECT_OFF_SCREEN_RADIUS: usize = 5;

pub const SPELL_CODES: [&str; SPELL_COUNT] = [
    "IL", "GP", "AZ", "AN", "M", "AY", "AS", "ACX", "HR", "IW", "KX", "IMX", "LV", "FV", "FGI",
    "GIN", "GIZ", "IP", "AG", "IS", "GIS", "PU", "DP", "QW", "BIX", "AEP", "EIP", "MV", "IZ", "RT",
    "IPVY", "AQW", "AI", "AWY", "AEX", "BRX", "LS", "CX", "IQX", "IQW", "HIN", "CIQ", "CIM", "CKX",
    "CGIV", "FHI", "PRV", "AT",
];
pub const SPELL_RECIPE_MASKS: [u8; SPELL_COUNT] = [
    0x80, 0x84, 0x60, 0x60, 0x50, 0x28, 0x88, 0xa0, 0x88, 0x02, 0x11, 0x61, 0x81, 0x84, 0x94, 0x16,
    0x54, 0x18, 0x84, 0xe0, 0x15, 0x18, 0x18, 0x12, 0x98, 0xa8, 0x88, 0x51, 0x52, 0x89, 0x89, 0x03,
    0xa1, 0x81, 0x16, 0x93, 0x0b, 0x06, 0xd9, 0x03, 0x8a, 0x23, 0xf9, 0x39, 0x83, 0x89, 0x85, 0x29,
];
pub const SPELL_SCENE_DUNGEON: u8 = 0x01;
pub const SPELL_SCENE_COMBAT: u8 = 0x02;
pub const SPELL_SCENE_INDOOR: u8 = 0x04;
pub const SPELL_SCENE_OVERWORLD: u8 = 0x08;
pub const SPELL_SCENE_MASKS: [u8; SPELL_COUNT] = [
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_INDOOR,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_DUNGEON,
    SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_INDOOR,
    SPELL_SCENE_COMBAT | SPELL_SCENE_INDOOR,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
];
pub const MOONSTONE_SLOT_COUNT: usize = 8;
pub const MOONSTONE_INVALID_SCENE: u8 = 0xff;
pub const FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE: u8 = 10;
pub const MOONSTONE_PICKUP_AUX3: u8 = b'M';
pub const DEFAULT_FOOD_STOCK: u16 = 63;
pub const DEFAULT_GOLD_STOCK: u16 = 150;
pub const DEFAULT_KEY_STOCK: u8 = 2;
pub const DEFAULT_GEM_STOCK: u8 = 0;
pub const DEFAULT_CLIMBING_GEAR: u8 = 0;
pub const DEFAULT_CLIMB_STAT: u8 = 30;
pub const SHIP_BADLY_DAMAGED_WARNING: &str = "DANGER: SHIP BADLY DAMAGED!";
pub const SHIP_NO_SKIFFS_WARNING: &str = "WARNING: NO SKIFFS ON BOARD!";
pub const FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER: u8 = 28;
pub const FIRST_PLAYABLE_FULL_SHIP_HULL: u8 = 77;
pub const FIRST_PLAYABLE_HORSE_TILE: u8 = 160;
pub const FIRST_PLAYABLE_FRIGATE_TILE: u8 = 168;
pub const FIRST_PLAYABLE_SKIFF_TILE: u8 = 176;
pub const FIRST_PLAYABLE_MAGIC_CARPET_TILE: u8 = 184;
pub const FIRST_PLAYABLE_BALLOON_TILE: u8 = 188;
pub const DEFAULT_PARTY_HP: u16 = 60;
pub const DEFAULT_PARTY_MAX_HP: u16 = 150;
pub const REST_WATCH_TICKS_PER_HOUR: u8 = 3;
pub const REST_WATCH_MINUTES_PER_TICK: u8 = 20;
pub const TOWN_REST_TICKS_PER_HOUR: u8 = 6;
pub const TOWN_REST_MINUTES_PER_TICK: u8 = 10;
pub const TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS: u8 = 16;
/// `rest-and-camp.md §4`: when the player's chosen rest digit lands the
/// target hour past 23, the original engine subtracts 23 (not 24) to
/// land on hour 1 instead of hour 0. Preserve this exact compatibility
/// edge rather than applying a normal modulo-24 wrap.
pub const TOWN_REST_HOUR_WRAP_SUBTRAHEND: u8 = 23;
/// `rest-and-camp.md §4`: per-tick budget for the elapsed-time rest
/// loop. A digit of `1..=9` plus the 23-not-24 wrap edge can advance
/// at most this many ten-minute ticks before the loop bails, even if
/// the target hour has not been reached, so a corrupted clock cannot
/// spin forever.
pub const TOWN_REST_TICK_BUDGET: u16 = 144;
/// `rest-and-camp.md §4`: maximum accepted single-digit rest duration.
/// `Space` and `0` cancel; `1..=9` are echoed and used as the target-
/// hour offset.
pub const TOWN_REST_DURATION_DIGIT_MAX: u8 = 9;
pub const REST_MANA_CAP: u8 = 99;
pub const DEFAULT_TORCH_STOCK: u8 = 4;
pub const SURFACE_TORCH_DURATION: u8 = 240;
/// `lighting.md §8` dungeon Ignite minimum increment. Same value as
/// the `DUNGEON_TORCH_INCREMENT_MIN` anchor in lighting.rs: dungeon
/// Ignite rolls a uniform `[MIN, MAX]` torch-counter increment.
pub const DUNGEON_TORCH_DURATION_MIN: u8 = crate::DUNGEON_TORCH_INCREMENT_MIN;
/// `time.md §6` / `lighting.md §3` published ambient-light scale.
/// Full daylight is 50 and full darkness is 2; values strictly above
/// `FULL_DAYLIGHT` (`>= DAYLIGHT_SENTINEL_MIN`) are the "skip
/// recompute" sentinel band the cleanup routine leaves alone.
pub const FULL_DAYLIGHT: u8 = 50;
pub const FULL_DARKNESS: u8 = 2;
pub const DAYLIGHT_SENTINEL_MIN: u8 = FULL_DAYLIGHT + 1;
pub const TORCH_LIGHT_FLOOR: u8 = 18;
pub const LIGHT_SPELL_FLOOR: u8 = 10;
pub const DAWN_DUSK_LIGHT: [u8; 6] = [2, 5, 10, 20, 34, 49];
pub const OOL_RECORD_LEN: usize = 8;
pub const OOL_SLOTS: usize = 32;
pub const OOL_PLANE_LEN: usize = OOL_RECORD_LEN * OOL_SLOTS;
pub const SAVED_OOL_LEN: usize = OOL_PLANE_LEN * 2;
/// `formats/dungeon-dat.md §1` published filename for the 4,096-byte
/// dungeon-record file.
pub const DUNGEON_DAT_FILENAME: &str = "DUNGEON.DAT";
/// `formats/dungeon-dat.md §1,§2` number of dungeon records the
/// file ships: "Eight dungeon records." The file is dungeon-major
/// with each record occupying [`DUNGEON_RECORD_LEN`] bytes.
pub const DUNGEON_DAT_RECORD_COUNT: usize = 8;
/// `formats/dungeon-dat.md §1,§6` total file length: 4,096 bytes
/// = 8 records × 512 bytes per record. Anchored to
/// [`DUNGEON_DAT_RECORD_COUNT`] × [`DUNGEON_RECORD_LEN`] so the
/// file length and the record layout stay one value.
pub const DUNGEON_DAT_LEN: usize = DUNGEON_DAT_RECORD_COUNT * DUNGEON_RECORD_LEN;
pub const DUNGEON_RECORD_LEN: usize = 512;
pub const DUNGEON_SIDE: usize = 8;
/// `formats/dungeon-dat.md §2,§6`: each dungeon level is an
/// eight-by-eight row-major grid of packed cell bytes, so the
/// level block is `8 * 8 = 64` bytes. Anchored to
/// [`DUNGEON_SIDE`] squared so the level byte length and the
/// grid side stay one value.
pub const DUNGEON_LEVEL_LEN: usize = DUNGEON_SIDE * DUNGEON_SIDE;
pub const DUNGEON_VIEW_DEPTH: usize = 4;
pub const DUNGEON_GEM_VIEW_RADIUS: isize = 5;
pub const WORLD_SIDE: usize = 256;
pub const WORLD_CELLS: usize = WORLD_SIDE * WORLD_SIDE;
pub const UNDER_DAT_LEN: usize = WORLD_CELLS;
pub const BRIT_DAT_LEN: usize = 52_480;
/// `formats/world-map.md`: published surface and underworld
/// world-map filenames.
pub const BRIT_DAT_FILENAME: &str = "BRIT.DAT";
pub const UNDER_DAT_FILENAME: &str = "UNDER.DAT";
pub const CHUNK_SIDE: usize = 16;
pub const CHUNK_BYTES: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const WORLD_CHUNKS_PER_SIDE: usize = WORLD_SIDE / CHUNK_SIDE;
pub const WORLD_CHUNK_COUNT: usize = WORLD_CHUNKS_PER_SIDE * WORLD_CHUNKS_PER_SIDE;
pub const BRIT_STORED_CHUNKS: usize = BRIT_DAT_LEN / CHUNK_BYTES;

/// `overworld.md §4` overworld live-chunk buffer dimensions. The
/// engine keeps four 16-by-16 chunks live in a 1-KiB chunk buffer
/// arranged as a 2-by-2 grid; the four chunks together form a
/// 32-by-32 cell window. The renderer projects an
/// [`crate::VIEWPORT_SIDE`]-wide subwindow out of this buffer each
/// frame, and chunk-aligned scroll-base movement reloads the buffer
/// once every 16 cells of party motion.
pub const OVERWORLD_CHUNK_BUFFER_GRID_SIDE: usize = 2;
pub const OVERWORLD_CHUNK_BUFFER_CHUNKS: usize =
    OVERWORLD_CHUNK_BUFFER_GRID_SIDE * OVERWORLD_CHUNK_BUFFER_GRID_SIDE;
pub const OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE: usize =
    OVERWORLD_CHUNK_BUFFER_GRID_SIDE * CHUNK_SIDE;
pub const OVERWORLD_CHUNK_BUFFER_BYTES: usize =
    OVERWORLD_CHUNK_BUFFER_CHUNKS * CHUNK_BYTES;
pub const BRIT_WATER_SENTINEL: u8 = 0xff;
pub const BRIT_DEEP_WATER_TILE: u8 = 1;
pub const NPC_PATH_QUEUE_LIMIT: usize = 32;
pub const ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS: usize = 32;
pub const PLAYER_NPC_SLOT: usize = OOL_SLOTS - 1;
pub const PLAYER_NPC_SENTINEL_TYPE: u8 = 0x7f;
pub const PLAYER_NPC_DIALOG_ID: u8 = 0;
pub const LOCATION_MARKER_CLEANUP_TILE: u8 = 16;
