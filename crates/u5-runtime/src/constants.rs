//! Engine-wide constants: filenames, save offsets, tile sentinels,
//! world dimensions, spell tables, defaults.

pub const DEFAULT_GAME_DIR: &str = r"C:\Games\U5-Clean";
pub const REPORT_PATH: &str = "reports/lb-throne-room-slice.txt";
pub const WORLD_LOCATION_TABLE_FILE: &str = "world_locations.tsv";
pub const WORLD_PLANE_TRANSITION_TABLE_FILE: &str = "world_plane_transitions.tsv";
pub const WORLD_GET_TILE_TABLE_FILE: &str = "world_get_tiles.tsv";
pub const OBJECT_PICKUP_TABLE_FILE: &str = "object_pickups.tsv";
/// Retired compatibility artifact name. The promoted runtime baseline does not
/// load current/waterfall sweep sidecars.
pub const WORLD_WATERFALL_TABLE_FILE: &str = "world_waterfalls.tsv";
pub const WORLD_DAMAGE_TILE_TABLE_FILE: &str = "world_damage_tiles.tsv";
pub const WORLD_ENCOUNTER_TABLE_FILE: &str = "world_encounters.tsv";
pub const SHRINE_TABLE_FILE: &str = "shrines.tsv";
pub const CODEX_URN_TABLE_FILE: &str = "codex_urns.tsv";
/// Retired compatibility artifact. Native dungeon edge exits do not read it.
pub const DUNGEON_DEEPER_TRANSITION_TABLE_FILE: &str = "dungeon_deeper_transitions.tsv";
pub const DUNGEON_TELEPORT_TABLE_FILE: &str = "dungeon_teleports.tsv";
/// Retired compatibility artifact. The published format has no exit tile class.
pub const DUNGEON_EXIT_TILE_TABLE_FILE: &str = "dungeon_exit_tiles.tsv";
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
pub const TOWN_LOCK_TABLE_FILE: &str = "town_locks.tsv";
pub const ETERNAL_FLAME_TABLE_FILE: &str = "eternal_flames.tsv";
pub const LOCATION_FLOOR_TABLE_FILE: &str = "location_floor_pages.tsv";
/// Retired compatibility artifact name. Public issue #94 established that
/// overworld entry never performs a per-scene row lookup.
pub const LOCATION_ENTRY_Y_TABLE_FILE: &str = "location_entry_y.tsv";

/// `town-mode.md §5` / public issue #94: overworld entry writes the same
/// column, row and floor for every town, castle, keep and dwelling. This is
/// unrelated to the resident-Shadowlord helper's per-scene row table.
pub const LOCATION_DEFAULT_ENTRY_X: usize = 15;
pub const LOCATION_DEFAULT_ENTRY_Y: usize = 30;
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
/// descriptions and 256..=511 for object descriptions. Anchored
/// to [`LOOK2_TABLE_LEN`] so the LOOK2.DAT offset table size and
/// the look2-table length share one source of truth.
pub const LOOK2_DAT_OFFSET_TABLE_LEN: usize = LOOK2_TABLE_LEN;
pub const LOOK2_DAT_TERRAIN_ENTRIES: usize = 256;
/// `formats/look2-dat.md §2`: 256 object-domain offset entries —
/// one per object byte value. Same fundamental count as
/// [`LOOK2_DAT_TERRAIN_ENTRIES`] (both domains index by a full
/// byte). Anchored through to the terrain-entries count so the
/// per-domain entry count has one source of truth.
pub const LOOK2_DAT_OBJECT_ENTRIES: usize = LOOK2_DAT_TERRAIN_ENTRIES;
/// `formats/look2-dat.md §3`: byte offset where the object-domain
/// portion of the LOOK2.DAT offset table begins. The first
/// `LOOK2_DAT_TERRAIN_ENTRIES` entries (256) occupy 2 bytes each,
/// so the object section starts at offset 256 × 2 = 512. Anchor
/// to that product so the object-domain base derives from the
/// terrain-entry count.
pub const LOOK2_DAT_OBJECT_DOMAIN_BASE: usize = LOOK2_DAT_TERRAIN_ENTRIES * 2;
pub const KARMA_DAT_FILE: &str = "KARMA.DAT";
/// `formats/karma-dat.md §2`: file size in the shipped DOS data set.
pub const KARMA_DAT_LEN: usize = 761;
/// `formats/karma-dat.md §2`: number of NUL-terminated text records.
pub const KARMA_DAT_RECORDS: usize = 6;
pub const TILES_EGA_FILE: &str = "TILES.16";
pub const TILES_CGA_FILE: &str = "TILES.4";
/// `formats/bit.md §3`: the directory opens with a two-byte
/// "Sub-image count" word — "Number of sub-images in this resource."
///
/// It is not an entry count for a display-driver strip table: §1 says
/// "Nothing in the family is a display-driver 'sparse strip' table, and
/// the leading value is never an entry count for such a table."
pub const BIT_SUB_IMAGE_COUNT_WORD_LEN: usize = 2;
/// `formats/bit.md §3`: the offset table is `count * 2` bytes — "For
/// each sub-image, its byte offset measured from the start of the
/// decoded image." There are no four-byte pointer/metadata entries and
/// no zero-pointer sentinel; §6: "There are no sparse or skipped
/// entries and no over-allocated table; every entry in the directory
/// names a real sub-image."
pub const BIT_OFFSET_TABLE_ENTRY_LEN: usize = 2;
/// `formats/bit.md §4.3`: `WD.BIT` is a single-sub-image resource whose
/// "Warriors of Destiny" record is exactly 288 by 49 — the same
/// geometry as the `ULTIMA` title-tick records it ignites into. The
/// record "is never drawn. It is a **mask**."
pub const WD_BIT_LETTERING_ROWS: u16 = 49;
pub const WD_BIT_LETTERING_COLUMNS: u16 = 288;
/// `formats/bit.md §3` sub-image header word widths. Each sub-image
/// opens with a width word and a height word before its
/// one-bit-per-pixel rows — four bytes of header total, which is also
/// the constant term of the record stride
/// `4 + max(1, ceil(width / 8)) * height`.
pub const BIT_SUB_IMAGE_WIDTH_WORD_LEN: usize = 2;
pub const BIT_SUB_IMAGE_HEIGHT_WORD_LEN: usize = 2;
pub const BIT_SUB_IMAGE_HEADER_LEN: usize =
    BIT_SUB_IMAGE_WIDTH_WORD_LEN + BIT_SUB_IMAGE_HEIGHT_WORD_LEN;
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
/// `formats/font-hcs.md §2`: each .HCS row encodes
/// HCS_CELL_WIDTH pixels at one bit per pixel = HCS_CELL_WIDTH /
/// 8 = 2 bytes per row. Anchored to that bit-packing arithmetic
/// so resizing the cell width automatically updates the row
/// stride.
pub const HCS_BYTES_PER_ROW: usize = HCS_CELL_WIDTH / 8;
/// `formats/font-hcs.md §2`: per-glyph byte count derived from the
/// cell geometry — twelve rows × two bytes per row = 24 bytes per
/// glyph. Anchored to [`HCS_CELL_HEIGHT`] × [`HCS_BYTES_PER_ROW`]
/// so the per-glyph byte count and the cell geometry stay one
/// value.
pub const HCS_GLYPH_BYTES: usize = HCS_CELL_HEIGHT * HCS_BYTES_PER_ROW;
pub const TITLE_BIT_FILE: &str = "TITLE.BIT";
pub const BRITISH_BIT_FILE: &str = "BRITISH.BIT";
pub const WD_BIT_FILE: &str = "WD.BIT";
pub const IBM_CH_FILE: &str = "IBM.CH";
pub const RUNES_CH_FILE: &str = "RUNES.CH";
pub const IBM_HCS_FILE: &str = "IBM.HCS";
pub const RUNES_HCS_FILE: &str = "RUNES.HCS";
pub const PROPORT_PCS_FILE: &str = "PROPORT.PCS";
pub const TILE_PASSABILITY_LEN: usize = 32;
/// `formats/look2-dat.md §2` & `catalogs/tile-catalog.md §1`: the
/// LOOK2 lookup table holds one offset per shared world tile id
/// (512 total — terrain 0..=255 and object 256..=511). Anchored
/// to [`TILE_ATLAS_TILE_COUNT`] so the LOOK2 table size and the
/// shared tile catalog share one source of truth.
pub const LOOK2_TILE_COUNT: usize = TILE_ATLAS_TILE_COUNT;
pub const LOOK2_TABLE_LEN: usize = LOOK2_TILE_COUNT * 2;
pub const TILE_ATLAS_TILE_COUNT: usize = 512;
pub const TILE_ATLAS_SIDE: usize = 16;
pub const TILE_ATLAS_TILE_PIXELS: usize = TILE_ATLAS_SIDE * TILE_ATLAS_SIDE;
pub const TILE_ATLAS_PIXEL_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_TILE_PIXELS;
/// `formats/tiles.md §3`: EGA tile pixel data packs at
/// `EGA_PIXELS_PER_BYTE` pixels per byte. Anchor the stride to
/// that divisor so the EGA packing density and the per-tile
/// byte stride share one source of truth.
pub const TILE_ATLAS_EGA_TILE_STRIDE: usize = TILE_ATLAS_TILE_PIXELS / EGA_PIXELS_PER_BYTE;
/// `formats/tiles.md §4`: CGA tile pixel data packs at
/// `CGA_PIXELS_PER_BYTE` pixels per byte. Anchor the stride to
/// that divisor so the CGA packing density and the per-tile
/// byte stride share one source of truth.
pub const TILE_ATLAS_CGA_TILE_STRIDE: usize = TILE_ATLAS_TILE_PIXELS / CGA_PIXELS_PER_BYTE;
pub const TILE_ATLAS_EGA_BODY_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_EGA_TILE_STRIDE;
pub const TILE_ATLAS_CGA_BODY_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_CGA_TILE_STRIDE;
/// `formats/lzw.md §3`: LZW code 256 is the "clear table" marker;
/// 257 is the end-of-stream marker; user codes start at 258 (one
/// past end-of-stream). Anchor END_CODE and FIRST_USER_CODE to
/// the per-step chain so the marker layout has one source of
/// truth.
pub const LZW_CLEAR_CODE: u16 = 256;
pub const LZW_END_CODE: u16 = LZW_CLEAR_CODE + 1;
pub const LZW_FIRST_USER_CODE: u16 = LZW_END_CODE + 1;
/// `formats/lzw.md §3`: maximum LZW dictionary size = `2^max_code_size`
/// (4096 codes at a 12-bit max code width). Anchored to
/// `1 << LZW_MAX_CODE_SIZE` so the dictionary cap and the
/// code-width ceiling stay one value.
pub const LZW_MAX_CODES: u16 = 1 << LZW_MAX_CODE_SIZE;
pub const LZW_INITIAL_CODE_SIZE: u8 = 9;
pub const LZW_MAX_CODE_SIZE: u8 = 12;
/// `formats/lzw.md §2`: the LZW envelope opens with a four-byte
/// little-endian unsigned length giving the exact number of decoded
/// bytes that follow the code stream. Promote the header width so
/// decode_lzw_envelope does not encode `4` as a bare literal.
pub const LZW_ENVELOPE_LENGTH_HEADER_BYTES: usize = 4;

// `formats/tiles.md §5.1.1` is titled "Resident miniature tile glyphs —
// withdrawn": "Earlier revisions of this section claimed that the
// resident engine carries a second, compact per-tile rendering source: a
// 'miniature' encoding of thirty-two bytes per tile, sixteen rows of two
// offset bytes each ... No such path exists." The thirty-two-byte
// records that section does publish are the night beacon's beam stencil,
// indexed by animation frame modulo sixteen, and their named lengths
// live in `light_beacon.rs` as `BEACON_STENCIL_RECORD_BYTES` and
// friends. `stats-panel.md §8` withdrew the other half: the
// timed-effect byte "is emitted as an ordinary character through the
// text system", not through any tile path. So the miniature row /
// bytes-per-row / record-length constants that used to sit here are
// gone; nothing is owed a miniature-glyph decoder.

/// `formats/tiles.md §5.2` image-directory count-word width. The
/// directory opens with a little-endian unsigned word giving the
/// number of slot entries that follow.
pub const TILE_IMAGE_DIRECTORY_COUNT_BYTES: usize = 2;
/// `formats/tiles.md §5.2` per-slot offset width. Each entry in the
/// offset table is a little-endian unsigned doubleword.
pub const TILE_IMAGE_DIRECTORY_OFFSET_BYTES: usize = 4;
/// `formats/tiles.md §5.2` per-image header width. Each image block
/// opens with a width word (2 bytes) and a height word (2 bytes).
/// Anchored to twice the directory count-word width (the same
/// two-byte unsigned-word type the directory itself uses) so the
/// per-image header derives from the format's word size.
pub const TILE_IMAGE_BLOCK_HEADER_BYTES: usize = 2 * TILE_IMAGE_DIRECTORY_COUNT_BYTES;
/// Both shipped fixed-cell fonts (.CH and .HCS) carry exactly 128
/// glyphs (`formats/font-ch.md §2`, `formats/font-hcs.md §2`).
/// Anchored to [`CH_GLYPH_COUNT`] so the parse-side glyph count
/// stays one value with the .CH catalog size; [`HCS_GLYPH_COUNT`]
/// is required to match.
pub const FIXED_FONT_GLYPH_COUNT: usize = CH_GLYPH_COUNT;
/// `formats/font-ch.md §2`: .CH cells are 8x8. Anchored to
/// [`CH_CELL_SIDE`] so the parse-side cell width stays one value.
pub const CH_FONT_CELL_WIDTH: usize = CH_CELL_SIDE;
/// `formats/font-ch.md §2`: .CH cells are 8x8. Anchored to
/// [`CH_CELL_SIDE`] so the parse-side cell height stays one value.
pub const CH_FONT_CELL_HEIGHT: usize = CH_CELL_SIDE;
/// `formats/font-hcs.md §2`: .HCS cells are 16x12. Anchored to
/// [`HCS_CELL_WIDTH`] so the parser-side cell width stays one
/// value with the format-side cell width.
pub const HCS_FONT_CELL_WIDTH: usize = HCS_CELL_WIDTH;
/// `formats/font-hcs.md §2`: .HCS cells are 16x12. Anchored to
/// [`HCS_CELL_HEIGHT`] so the parser-side cell height stays one
/// value with the format-side cell height.
pub const HCS_FONT_CELL_HEIGHT: usize = HCS_CELL_HEIGHT;
pub const PCS_FIRST_CODE: u8 = 0x20;
pub const PCS_GLYPH_BITMAP_WIDTH: usize = 8;
/// Observation-derived (`cleak/u5-spec#70`). `formats/font-pcs.md`
/// never publishes the proportional cell height. Every glyph block
/// in the shipped `PROPORT.PCS` declares a height word of 8, and the
/// glyph bands measured off a black-box run of the original intro
/// story slides are exactly 8 pixel rows tall on a 9-pixel line
/// stride, so the proportional cell is 8 rows.
pub const PCS_GLYPH_HEIGHT: usize = 8;
/// Observation-derived (`cleak/u5-spec#70`). Each glyph record in the
/// `PROPORT.PCS` glyph directory opens with a width word and a height
/// word before its row bytes.
pub const PCS_GLYPH_BLOCK_HEADER_LEN: usize = 4;
pub const PCS_GLYPH_BLOCK_LEN: usize = PCS_GLYPH_BLOCK_HEADER_LEN + PCS_GLYPH_HEIGHT;
/// Observation-derived (`cleak/u5-spec#70`). The resident advance for
/// a printable proportional glyph is its stored ink width plus one
/// blank separator column; measured over 8,995 glyph placements in
/// the original's twenty intro story slides.
pub const PCS_GLYPH_ADVANCE_GAP: u8 = 1;
/// Observation-derived (`cleak/u5-spec#70`). The space glyph carries a
/// stored ink width of zero, but the resident table advances a natural
/// (unjustified) space by 5 pixels.
pub const PCS_SPACE_ADVANCE: u8 = 5;
pub const PLAY_SCRIPT_MAX_IDLE_TICKS: usize = 1024;
/// Runtime count of karma reaction records. The KARMA.DAT parser
/// uses this same loop bound to walk the six NUL-terminated text
/// records the format spec documents. Anchored to
/// [`KARMA_DAT_RECORDS`] so the parser-side loop bound and the
/// format-side record count stay one value.
pub const KARMA_RECORD_COUNT: usize = KARMA_DAT_RECORDS;
pub const PLAY_IGNORED_INPUT_KEY: char = '\u{1e}';
pub const PLAY_TYPEAHEAD_TOGGLE_KEY: char = '\u{1f}';
pub const PLAY_MUSIC_TOGGLE_KEY: char = '\u{13}';
/// `commands.md` Section 9: the program-exit binding in every mode loop's
/// pre-dispatch control-code table. It is a typed Control character - Control
/// with the fifth letter of the alphabet - and, like the other three shared
/// bindings, it consumes no turn.
///
/// `dungeon-mode.md` Section 10 settles which key owns the prompt: "`Q` is the
/// ordinary save-game route; the "Exit to DOS?" prompt is a Control binding in
/// the mode-local table, not a letter."
pub const PLAY_EXIT_TO_DOS_KEY: char = '\u{05}';
pub const TRAP_NON_COMBAT_EFFECT_TABLE: [u8; 8] = [0, 0, 0, 1, 1, 2, 2, 3];
pub const TRAP_ACID_DAMAGE_MAX: u8 = 30;
pub const TRAP_BOMB_DAMAGE_MAX: u8 = 8;
/// `vehicles.md §2` / `active-objects.md §5`: the default on-foot actor
/// byte copied into both type/frame fields of active-object slot zero.
///
/// Player identity comes exclusively from the slot index. There is no
/// separate `0xFC` player sentinel; `0xFC` is the Shadow Lord actor byte.
pub const PLAYER_TILE: u8 = crate::TRANSPORT_MARKER_FOOT_FIRST;

/// Actor-atlas index selected by the default on-foot marker. Kept as a
/// compatibility name for callers/tests that need the resolved tile id.
pub const PLAYER_SPRITE_TILE: usize = crate::ACTOR_TILE_BANK_BASE + PLAYER_TILE as usize;

// Moongate artwork lives at tile id 0xDC per LOOK2.DAT ("a moon gate!").
// Earlier guesses at 0x80 and 0xD4 picked the wrong tiles (food/banquet
// and a waterfall animation respectively).
//
// `overworld.md §9` (spec HEAD c00bf63) retracts the per-render-frame
// moongate animator in full: there is no frame ring here, so there is no
// frame-count constant either. A gate cell is resolved through the
// sixteen-step gate-presence phase model of `overworld.md §9.1`; see
// `crate::moongate_phase`.
pub const MOONGATE_TILE_BASE: u8 = 0xDC;
pub const NATURAL_MOONGATE_TERRAIN_TILE: u8 = 0xDC;
pub const NATURAL_MOONGATE_RESTORED_TERRAIN_TILE: u8 = 5;
pub const NATURAL_MOONGATE_COUNTER_MAX: u8 = 16;
pub const STEADY_PHASE: u8 = 0x0f;
/// Phase byte (`+0x06`) the player's own active-object record
/// carries in slot zero.
///
/// `formats/saved-gam.md §8.1` (spec `0170809`): "A byte-compatible
/// producer must not write a facing here — in particular, the player's
/// own record carries **zero** in this byte in a shipped save, not the
/// all-ones freeze marker." `RETRACTIONS.md` R340 states the same as the
/// withdrawal: "the **player's own record carries zero in byte 6 in a
/// shipped save, not the freeze sentinel**, and an engine that writes the
/// sentinel there diverges on every save."
///
/// The byte is not a facing and never was. `active-objects.md §3`: the
/// low nibble is a frame-delay countdown whose all-ones value is the
/// freeze sentinel, the high nibble is "the slot's step within its
/// **animation script**", and byte 6 "carries **no facing and no
/// direction of any kind**".
///
/// Why zero survives a save is also published, and it is not the tile
/// class: "a low nibble in `1..14` is **decremented and written back
/// unconditionally, with no tile-class precondition, on any slot
/// including slot zero**, and only a low nibble of zero reaches the
/// frame-byte and tile-class gates. Slot zero's byte 6 survives a shipped
/// save because of its stored **value**, not because the player's tile
/// class protects the record."
///
/// The DOS build agrees: driving it from the shipped save and saving
/// again — with no turns, and again after four turns across an hour
/// boundary — leaves the slot-zero record as `1C 1C 0F 0F 00 00 00 00`.
pub const PLAYER_ACTIVE_OBJECT_PHASE: u8 = 0x00;
/// Phase byte (`+0x06`) an arena active-object record carries after
/// combat setup has written it.
///
/// `active-objects.md §7` enumerates exactly what combat placement writes
/// into a record, and byte 6 is not in the list: "The setup pass **first
/// clears all thirty-two records**, then seats the party, then places
/// monsters. ... Each spawned monster gets one renderer-facing active-object
/// slot with the monster's class-derived tile in byte 0, the per-frame tile
/// byte at byte 1, arena coordinates in bytes 2 and 3, and a floor/plane flag
/// at byte 4. A seated party member's record uses the same shape, with the
/// class-derived party sprite in bytes 0 and 1, its arena seat in bytes 2 and
/// 3, and its roster slot index in byte 5. ... At placement time byte 5
/// receives the placed monster's starting HP, byte 4 receives the arena
/// plane/Z argument, and byte 7 receives an all-ones marker."
///
/// So byte 6 keeps the value the clear left, which is zero - and zero is not
/// the freeze sentinel. `active-objects.md §3`: the all-ones low nibble is
/// what makes the animator "bail immediately, **writing nothing**", while a
/// low nibble of zero falls through to the decision-point gates. Seeding
/// [`STEADY_PHASE`] here instead freezes every arena sprite for the whole
/// fight, which is exactly what the pre-fix engine did.
pub const COMBAT_PLACEMENT_ACTIVE_OBJECT_PHASE: u8 = 0x00;
/// `systems/weather.md §7`: "The cadence counter is stored per
/// active-object slot ... The cadence counter is persisted with the
/// object, so it survives save and reload."
///
/// `formats/saved-gam.md §8.1` gives active-object byte `+6` as the
/// "Animation phase / direction-step counter; compositor reads it for
/// water creatures", which is this engine's `phase`. Its high nibble
/// carries the frame heading and bits `0..1` select the drawn frame
/// (see [`crate::active_object_frame_tile`]), so the wind cadence count
/// takes bits `2..3` — two bits, which is exactly the `0..3` range the
/// published `2 of 3` and `3 of 4` cycles need.
pub const ACTIVE_SHIP_CADENCE_PHASE_SHIFT: u8 = 2;
/// See [`ACTIVE_SHIP_CADENCE_PHASE_SHIFT`].
pub const ACTIVE_SHIP_CADENCE_PHASE_MASK: u8 = 0b0000_1100;
/// `systems/chargen.md §6` / `systems/save-load.md`: the canonical
/// campaign start date. The year/month/day match the chargen
/// starting date documented in chargen.md; the start hour is
/// the post-chargen play-mode entry hour (chargen itself stamps
/// 8:35 AM, but the resumed-play save reader normalises to noon).
/// Anchor the year/month/day to the chargen-side constants so
/// the chargen exit and the play-start record share one value.
pub const PLAY_START_YEAR: u16 = crate::CHARGEN_STARTING_YEAR;
pub const PLAY_START_MONTH: u8 = crate::CHARGEN_STARTING_MONTH;
pub const PLAY_START_DAY: u8 = crate::CHARGEN_STARTING_DAY;
/// `systems/save-load.md`: the post-chargen play-mode entry
/// normalises the clock to noon (half-way through the 24-hour
/// day). Anchored to [`crate::HOURS_PER_DAY`] / 2 so the
/// "noon" hour derives from the clock day length.
pub const PLAY_START_HOUR: u8 = crate::HOURS_PER_DAY / 2;
/// `formats/saved-gam.md §2`: SAVED.GAM is exactly 4,192 bytes —
/// the reserved-tail span ending at file offset 0x105F. Anchored
/// to [`SAVE_RESERVED_TAIL_OFFSET`] + [`SAVE_RESERVED_TAIL_LEN`]
/// so the file length and the reserved-tail span stay one value.
pub const SAVED_GAM_LEN: usize = SAVE_RESERVED_TAIL_OFFSET + SAVE_RESERVED_TAIL_LEN;
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
/// `formats/saved-gam.md §4` party inventory band starts at
/// 0x0202 with the word-sized food counter. The subsequent
/// counters chain by their stored widths: food/gold are words
/// (2 bytes each); key/gem/torch/Grapple are single bytes. The
/// Grapple byte is the legacy "climbing gear" / magic-powder slot
/// used by outdoor K-Klimb.
pub const SAVE_FOOD_STOCK_OFFSET: usize = SAVE_ROSTER_OFFSET + SAVE_ROSTER_REGION_LEN;
pub const SAVE_GOLD_STOCK_OFFSET: usize = SAVE_FOOD_STOCK_OFFSET + 2;
pub const SAVE_KEY_STOCK_OFFSET: usize = SAVE_GOLD_STOCK_OFFSET + 2;
pub const SAVE_GEM_STOCK_OFFSET: usize = SAVE_KEY_STOCK_OFFSET + 1;
pub const SAVE_TORCH_STOCK_OFFSET: usize = SAVE_GEM_STOCK_OFFSET + 1;
pub const SAVE_CLIMBING_GEAR_OFFSET: usize = SAVE_TORCH_STOCK_OFFSET + 1;
pub const SAVE_SPECIAL_ITEM_OFFSET: usize = SAVE_CLIMBING_GEAR_OFFSET + 1;
/// `formats/saved-gam.md §4` the special-item, equipment-stock,
/// spell-charge, scroll, potion, and moonstone bands occupy
/// contiguous fixed-length blocks sized by their catalog counts.
/// Anchor each band offset to the previous-band chain so adding
/// a special item, equipment id, or spell id automatically
/// shifts the later band offsets.
pub const SAVE_EQUIPMENT_STOCK_OFFSET: usize = SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_COUNT;
pub const SAVE_SPELL_CHARGES_OFFSET: usize = SAVE_EQUIPMENT_STOCK_OFFSET + EQUIPMENT_STOCK_BAND_LEN;
pub const SAVE_SCROLL_STOCK_OFFSET: usize = SAVE_SPELL_CHARGES_OFFSET + SPELL_CHARGE_BAND_LEN;
pub const SAVE_POTION_STOCK_OFFSET: usize = SAVE_SCROLL_STOCK_OFFSET + SCROLL_COUNT;
pub const SAVE_MOONSTONE_X_OFFSET: usize = SAVE_POTION_STOCK_OFFSET + POTION_COUNT;
pub const SAVE_MOONSTONE_Y_OFFSET: usize = SAVE_MOONSTONE_X_OFFSET + MOONSTONE_SLOT_COUNT;
pub const SAVE_MOONSTONE_SCENE_OFFSET: usize = SAVE_MOONSTONE_Y_OFFSET + MOONSTONE_SLOT_COUNT;
pub const SAVE_MOONSTONE_Z_OFFSET: usize = SAVE_MOONSTONE_SCENE_OFFSET + MOONSTONE_SLOT_COUNT;
pub const SAVE_REAGENTS_OFFSET: usize = SAVE_MOONSTONE_Z_OFFSET + MOONSTONE_SLOT_COUNT;
pub const SAVE_YEAR_OFFSET: usize = 0x02ce;
/// `formats/saved-gam.md §5`: bytes 0x02d4..=0x02d6 are three
/// adjacent state bytes — active-effect code, active player slot,
/// transport marker — preceding the calendar/clock chain at
/// SAVE_MONTH_OFFSET. Anchor each offset to the per-byte chain
/// so resizing any of these bytes only happens in one place.
/// `formats/saved-gam.md §4`: the one shared timed-magic-effect code.
/// Zero means no effect; this is unrelated to the transport marker at 0x02D6.
pub const SAVE_ACTIVE_EFFECT_CODE_OFFSET: usize = 0x02d4;
/// Compatibility alias retained for callers compiled against the earlier,
/// withdrawn transport/timing interpretation of this byte.
pub const SAVE_TIMING_STATUS_TAG_OFFSET: usize = SAVE_ACTIVE_EFFECT_CODE_OFFSET;
pub const SAVE_ACTIVE_PLAYER_OFFSET: usize = SAVE_TIMING_STATUS_TAG_OFFSET + 1;
pub const SAVE_TRANSPORT_MARKER_OFFSET: usize = SAVE_ACTIVE_PLAYER_OFFSET + 1;
/// `formats/saved-gam.md §5`: calendar/clock bytes at
/// 0x02d7..=0x02de form a contiguous per-byte chain: month, day,
/// hour, saved-hour snapshot, minute, combat-round counter,
/// per-turn state, AM/PM display. Anchor each offset to the
/// previous-byte chain so resizing or inserting a calendar field
/// only happens in one place.
pub const SAVE_MONTH_OFFSET: usize = 0x02d7;
pub const SAVE_DAY_OFFSET: usize = SAVE_MONTH_OFFSET + 1;
pub const SAVE_HOUR_OFFSET: usize = SAVE_DAY_OFFSET + 1;
/// `formats/saved-gam.md §5`: adjacent saved-hour snapshot byte the
/// per-turn cleanup uses to detect hour crossings. Not the active
/// hour; preserve byte-for-byte on round trip.
pub const SAVE_SAVED_HOUR_SNAPSHOT_OFFSET: usize = SAVE_HOUR_OFFSET + 1;
pub const SAVE_MINUTE_OFFSET: usize = SAVE_SAVED_HOUR_SNAPSHOT_OFFSET + 1;
pub const SAVE_COMBAT_ROUND_COUNTER_OFFSET: usize = SAVE_MINUTE_OFFSET + 1;
/// `formats/saved-gam.md §5` adjacent per-turn state byte; preserve
/// byte-for-byte but no public calendar meaning. `systems/time.md §11`
/// names it: "It is the cached wind-cadence byte, and the wind setter
/// clears it whenever the wind actually changes".
pub const SAVE_PER_TURN_STATE_OFFSET: usize = SAVE_COMBAT_ROUND_COUNTER_OFFSET + 1;
/// `formats/saved-gam.md §5` (spec `0170809`): "Twelve-hour hour value /
/// audio repeat countdown ... Written with the twelve-hour form of the
/// hour when the cleanup finds the snapshot at `0x02DA` disagreeing with
/// the hour at `0x02D9`, then counted down toward zero by the
/// ambient-audio tick. Nothing renders it."
///
/// The earlier name `SAVE_AMPM_DISPLAY_OFFSET` is withdrawn with the
/// "12-hour display" reading itself: `RETRACTIONS.md` R338 keeps the
/// value rule but withdraws the word *display*, "because no consumer in
/// the shipped game renders this byte".
pub const SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET: usize = SAVE_PER_TURN_STATE_OFFSET + 1;
/// `formats/saved-gam.md §5.1` (spec `0170809`): "The two bytes at
/// `0x02DF` and `0x02E0` are the cached Trammel and Felucca moon-phase
/// digits for the current day of the month, in that order", each "stored
/// as the printable character for a digit in the range zero through
/// seven". `RETRACTIONS.md` R339 lifts them out of the old
/// "food gauge / mode scratch" band: "**They are gameplay state, not
/// scratch.** Natural-moongate transit selects its destination from these
/// two cached bytes and from nothing else".
pub const SAVE_CACHED_TRAMMEL_GLYPH_OFFSET: usize = SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET + 1;
pub const SAVE_CACHED_FELUCCA_GLYPH_OFFSET: usize = SAVE_CACHED_TRAMMEL_GLYPH_OFFSET + 1;
/// `formats/saved-gam.md §5`: in-game calendar bounds. Months are
/// one-based 1..=13 (thirteen 28-day months per year), days are
/// one-based 1..=28, hours are zero-based 0..=23, minutes 0..=59.
/// The save-side bounds are anchored to the clock-side
/// MONTHS_PER_YEAR / DAYS_PER_MONTH / HOURS_PER_DAY /
/// MINUTES_PER_HOUR so the calendar bounds and the time-system
/// constants stay one value.
pub const SAVE_MONTH_MIN: u8 = 1;
pub const SAVE_MONTH_MAX: u8 = crate::MONTHS_PER_YEAR;
pub const SAVE_DAY_MIN: u8 = 1;
pub const SAVE_DAY_MAX: u8 = crate::DAYS_PER_MONTH;
pub const SAVE_HOUR_MAX: u8 = crate::HOURS_PER_DAY - 1;
pub const SAVE_MINUTE_MAX: u8 = crate::MINUTES_PER_HOUR - 1;
/// `overworld.md §9` / `§9.1` (spec HEAD c00bf63): the shared
/// natural-moongate gate-presence counter is persisted world state, not
/// scratch. It occupies one byte in the mode-scratch band, immediately
/// below the moral-standing selector, and the shipped starting save
/// holds zero there - correct, because the game opens at hour eight with
/// no gate up. Modelling it as turn-scoped breaks save/load round-trip
/// and loses the mid-rise state.
pub const SAVE_NATURAL_MOONGATE_COUNTER_OFFSET: usize = 0x02e1;
pub const SAVE_MORAL_STANDING_OFFSET: usize = 0x02e2;
/// `formats/saved-gam.md §10`: toll-progress counter byte adjacent to
/// the moral-standing selector. Increments per successful three-digit
/// `0x85` conversation gold payment; resets to zero and bumps the
/// selector on the [`TOLL_PROGRESS_MILESTONE`] roll-over.
pub const SAVE_TOLL_PROGRESS_OFFSET: usize = 0x02e5;
/// `formats/saved-gam.md §10` / `rest-and-camp.md §5`: persisted
/// completed-camp recovery cooldown. The counter is armed at fourteen
/// and loses one at each hour rollover, floored at zero.
pub const SAVE_CAMP_COOLDOWN_OFFSET: usize = 0x02e6;
/// `formats/saved-gam.md §10` / `rest-and-camp.md §5`: current
/// calendar month copied by the successful camp-apparition draw. The
/// shipped program never reads it, but the write and save preservation
/// are part of the published byte contract.
pub const SAVE_CAMP_MONTH_COOKIE_OFFSET: usize = 0x02e7;
/// `formats/saved-gam.md §4`: remaining duration paired with the shared
/// effect code at [`SAVE_ACTIVE_EFFECT_CODE_OFFSET`]. `0xFF` is permanent.
pub const SAVE_ACTIVE_EFFECT_DURATION_OFFSET: usize = 0x02e8;
pub const SAVE_WIND_OFFSET: usize = 0x02ec;
/// `formats/saved-gam.md §5`: "The five bytes after wind form the
/// persisted location cluster" — scene byte sits immediately
/// after the one-byte wind state. Anchor SAVE_SCENE_OFFSET to
/// SAVE_WIND_OFFSET + 1 so the wind→location adjacency has one
/// source of truth.
pub const SAVE_SCENE_OFFSET: usize = SAVE_WIND_OFFSET + 1;
/// `formats/saved-gam.md §5`: the persisted location cluster
/// occupies `0x02ed..=0x02f1` — scene byte, saved-scene/mode
/// scratch (one byte), party Z, party X, party Y. Anchor Z, X,
/// and Y to the per-byte chain rooted at SAVE_SCENE_OFFSET so
/// inserting or resizing the scratch byte automatically shifts
/// the coordinate offsets.
pub const SAVE_Z_OFFSET: usize = SAVE_SCENE_OFFSET + 2;
pub const SAVE_X_OFFSET: usize = SAVE_Z_OFFSET + 1;
pub const SAVE_Y_OFFSET: usize = SAVE_X_OFFSET + 1;
pub const SAVE_LIGHT_SPELL_COUNTER_OFFSET: usize = 0x0300;
/// `formats/saved-gam.md §6`: the torch-duration counter sits
/// immediately after the light-spell counter. Anchor to
/// SAVE_LIGHT_SPELL_COUNTER_OFFSET + 1 so the two adjacent
/// lighting counters share one source of truth.
pub const SAVE_TORCH_COUNTER_OFFSET: usize = SAVE_LIGHT_SPELL_COUNTER_OFFSET + 1;
/// `formats/saved-gam.md §6.1`: one interference-source byte per combat
/// victim slot. Values `0..=31` name the most recent qualifying adjacent
/// attacker; `0xFF` means no source. The factory image deliberately seeds
/// this band with zeroes, which must be preserved as valid slot references.
pub const SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_OFFSET: usize = 0x0302;
pub const SAVE_COMBAT_INTERFERENCE_SOURCE_MAP_LEN: usize = OOL_SLOTS;
pub const COMBAT_INTERFERENCE_NO_SOURCE: u8 = u8::MAX;
/// `formats/saved-gam.md §9`: the two shrine progress masks sit
/// at file offsets 0x0326 and 0x0328 with an unnamed opaque byte
/// between them. Anchor the codex mask to the ordained mask + 2
/// so the two-byte stride between the parallel virtue bitmasks
/// has one source of truth.
pub const SAVE_SHRINE_ORDAINED_MASK_OFFSET: usize = 0x0326;
pub const SAVE_SHRINE_CODEX_MASK_OFFSET: usize = SAVE_SHRINE_ORDAINED_MASK_OFFSET + 2;
/// `formats/saved-gam.md §9.1`: eight durable Word-of-Power seal
/// bytes in the fixed dungeon-word order. The high bit is the live gate.
pub const SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET: usize = 0x032A;
pub const SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT: usize = 8;
/// `formats/saved-gam.md §9.1`: eight durable shrine-ruin bytes in
/// standard virtue order. A set high bit selects the ruined live tile.
pub const SAVE_SHRINE_RUIN_FLAGS_OFFSET: usize = 0x0332;
pub const SAVE_SHRINE_RUIN_FLAG_COUNT: usize = 8;
pub const SAVE_QUEST_TILE_FLAG_HIGH_BIT: u8 = 0x80;
/// `formats/saved-gam.md §10`: active temporary-door tracker. A zero
/// previous-tile byte gates the tracker inactive; the following bytes are
/// X, Y, and the remaining-turn countdown.
pub const SAVE_DOOR_TRACKER_PREVIOUS_TILE_OFFSET: usize = 0x03A9;
pub const SAVE_DOOR_TRACKER_X_OFFSET: usize = SAVE_DOOR_TRACKER_PREVIOUS_TILE_OFFSET + 1;
pub const SAVE_DOOR_TRACKER_Y_OFFSET: usize = SAVE_DOOR_TRACKER_X_OFFSET + 1;
pub const SAVE_DOOR_TRACKER_COUNTDOWN_OFFSET: usize = SAVE_DOOR_TRACKER_Y_OFFSET + 1;
/// `formats/saved-gam.md §10`: queued shipwright-delivery X coordinate.
pub const SAVE_PENDING_VEHICLE_X_OFFSET: usize = SAVE_DOOR_TRACKER_COUNTDOWN_OFFSET + 1;
/// `formats/saved-gam.md §10`: queued shipwright-delivery Y coordinate.
pub const SAVE_PENDING_VEHICLE_Y_OFFSET: usize = 0x03AE;
pub const SAVE_FORTUNES_OF_WAR_OFFSET: usize = 0x03b3;
/// `formats/saved-gam.md §10` (spec `0170809`): "`0x02FE` is the master
/// redraw-enable gate: while it is zero the idle world tick skips its
/// whole body, which suppresses the ambient-audio tick, the autonomous
/// wind drift and the object animator alike."
pub const SAVE_REDRAW_ENABLE_OFFSET: usize = 0x02fe;
/// `formats/saved-gam.md §10` (spec `0170809`): the cached ambient light
/// level, "recomputed by **every** clock call including the mode-zero
/// 'commit the screen without advancing time' call that scene entry
/// issues". Factory seed `5`, "a stale sample the first clock call
/// overwrites".
pub const SAVE_AMBIENT_LIGHT_OFFSET: usize = SAVE_REDRAW_ENABLE_OFFSET + 1;
/// `formats/saved-gam.md §10` (spec `0170809`): the resident-Shadowlord
/// selector, "`0`, `1` or `2` for a hosting location, `0xFF` for 'none'.
/// It is a **per-entry latch, not durable world state**." Town-family
/// entry stores the no-host marker unconditionally, so "a byte-compatible
/// producer emits `0xFF` for any save taken inside a location".
pub const SAVE_RESIDENT_SHADOWLORD_OFFSET: usize = 0x03b2;
/// `formats/saved-gam.md §10`: the "none" marker of the
/// resident-Shadowlord latch above.
pub const SAVE_RESIDENT_SHADOWLORD_NONE: u8 = 0xff;
/// `systems/time.md §11`: the ambient-audio tick decrements the
/// twelve-hour repeat counter at `0x02DE` "on **two of every eight** of
/// its own calls, using a small free-running sub-tick counter that is
/// not part of the save image". Eight is that counter's period: it
/// "cycles `0, 1, 2, 3, 4, 5, 6, 7` and wraps back to `0`".
pub const AMBIENT_AUDIO_SUB_TICK_PERIOD: u8 = 8;

/// `systems/time.md §11` (issue #190): the two sub-tick residues that
/// carry the decrement. "The decrement fires on the calls where it holds
/// **`0` or `4`** on entry. So the two residues are zero and four of the
/// eight-phase cycle - every fourth call, not two adjacent calls out of
/// eight." The engine's "low two bits clear" rule already selected
/// exactly this pair, so the rate and the residues were both already
/// right; what the answer added is that they are these two, the test
/// order and the counter's phase origin, both of which are pinned where
/// they are implemented, on `PlayState::tick_ambient_audio_repeats`.
///
/// "The same two residues also pick the loud envelope in the tick's own
/// lava/shrine effect branch, so one counter drives both behaviours and
/// an implementation should not give them separate phases."
pub const AMBIENT_AUDIO_DECREMENT_RESIDUES: [u8; 2] = [0, 4];

/// `formats/saved-gam.md §10`: durable dungeon room-clear bitmap. The
/// 16-byte block at `0x033A..0x0349` records which dungeon room
/// encounters have already been cleared; dungeon mode uses it to
/// demote matching `0xF?` room-trigger cells to `0xA?` room-helper
/// cells when rebuilding the loaded dungeon image from `DUNGEON.DAT`.
pub const SAVE_DUNGEON_ROOM_CLEAR_BITMAP_OFFSET: usize = 0x033A;
/// `formats/saved-gam.md §10`: the room-clear bitmap is sixteen
/// bytes — two bytes per dungeon × eight dungeons = 128 bits.
/// Anchored to `DUNGEON_DAT_RECORD_COUNT * SAVE_DUNGEON_ROOM_
/// CLEAR_BYTES_PER_DUNGEON` so the bitmap size derives from the
/// per-dungeon bitmap layout and the dungeon record count.
pub const SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN: usize =
    DUNGEON_DAT_RECORD_COUNT * crate::SAVE_DUNGEON_ROOM_CLEAR_BYTES_PER_DUNGEON;
/// `formats/saved-gam.md §8.1`: the live active-object table snapshot
/// occupies 256 bytes at file offset `0x06B4..=0x07B3`. Layout matches
/// the in-memory table (32 records × 8 bytes). The table starts
/// immediately after the NPC name-known mask bank.
pub const SAVE_ACTIVE_OBJECT_TABLE_OFFSET: usize =
    SAVE_NPC_NAME_KNOWN_MASKS_OFFSET + SAVE_NPC_MASK_BANK_LEN;
/// `formats/saved-gam.md §10`: packed queued-delivery family/payload byte.
pub const SAVE_PENDING_VEHICLE_CLASS_OFFSET: usize = 0x105F;
/// `formats/saved-gam.md §8.2`: the dungeon/map-cell working buffer
/// occupies 512 bytes at file offset `0x03B4..=0x05B3` and matches the
/// 512-byte dungeon-record stride. The buffer begins immediately
/// after the one-byte fortunes-of-war flag at 0x03B3; anchor the
/// offset to SAVE_FORTUNES_OF_WAR_OFFSET + 1 so the adjacency has
/// one source of truth.
pub const SAVE_DUNGEON_WORKING_BUFFER_OFFSET: usize = SAVE_FORTUNES_OF_WAR_OFFSET + 1;
/// `formats/saved-gam.md §8.2`: the working-buffer byte length is
/// the same 512-byte stride documented in
/// `formats/dungeon-dat.md §2`. Anchored to
/// [`DUNGEON_RECORD_LEN`] so the working buffer and the on-disk
/// dungeon record share one source of truth.
pub const SAVE_DUNGEON_WORKING_BUFFER_LEN: usize = DUNGEON_RECORD_LEN;
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
pub const SAVE_GRAPPLE_OFFSET: usize = SAVE_CLIMBING_GEAR_OFFSET;
pub const SAVE_EQUIPMENT_INVENTORY_OFFSET: usize = 0x021A;
pub const SAVE_SPELL_CHARGE_BLOCK_OFFSET: usize = 0x024A;
pub const SAVE_SCROLL_COUNTERS_OFFSET: usize = 0x027A;
pub const SAVE_POTION_COUNTERS_OFFSET: usize = 0x0282;
pub const SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET: usize = 0x020C;
/// `formats/saved-gam.md` §10, record 15: "Not a dedicated cookie. This
/// is the **equipment-inventory counter for item id `39` (Glass Sword)**
/// from Section 7, and record 15's granted item is that same Glass Sword.
/// ... An engine that gives record 15 a separate never-written cookie
/// yields an infinitely repeatable Glass Sword." §7 restates it: "`0x0241`
/// - the equipment counter for item id `39`, the Glass Sword - is also the
/// gate for fixed hidden-treasure record 15. It is the same byte, not a
/// parallel cookie." Anchored to the equipment block so the alias cannot
/// drift; the byte is written only by the equipment-stock block write.
pub const SAVE_FIXED_HIDDEN_TREASURE_RECORD_15_GATE_OFFSET: usize =
    SAVE_EQUIPMENT_STOCK_OFFSET + EQUIPMENT_ID_GLASS_SWORD;
pub const SAVE_SHADOWLORD_HIDEOUTS_OFFSET: usize = 0x0322;
/// `formats/saved-gam.md §9.2`: two back-to-back banks of 32
/// little-endian `u32` masks. Scene ids are one-based; bit `n` is
/// NPC roster slot `n` in that scene.
pub const SAVE_NPC_REMOVED_MASKS_OFFSET: usize = 0x05B4;
pub const SAVE_NPC_NAME_KNOWN_MASKS_OFFSET: usize = 0x0634;
pub const SAVE_NPC_MASK_SCENE_COUNT: usize = 32;
pub const SAVE_NPC_MASK_BYTES_PER_SCENE: usize = 4;
pub const SAVE_NPC_MASK_BANK_LEN: usize = SAVE_NPC_MASK_SCENE_COUNT * SAVE_NPC_MASK_BYTES_PER_SCENE;
pub const SAVE_FIXED_HIDDEN_TREASURE_FOUND_OFFSET: usize = 0x02B6;
/// `formats/saved-gam.md §6` location-cluster scratch offsets.
pub const SAVE_SAVED_SCENE_SCRATCH_OFFSET: usize = 0x02EE;
pub const SAVE_PARTY_Z_OFFSET: usize = 0x02EF;
pub const SAVE_PARTY_X_OFFSET: usize = 0x02F0;
pub const SAVE_PARTY_Y_OFFSET: usize = 0x02F1;
/// `formats/saved-gam.md §6`: party Z `0xFF` is the "no active map"
/// sentinel.
pub const SAVE_PARTY_Z_NO_ACTIVE_MAP: u8 = 0xFF;
/// `formats/saved-gam.md §3`: Avatar's name field is the same
/// nine-byte NUL-padded slot every character record uses (record
/// zero is the Avatar). Anchored to [`SAVE_CHARACTER_NAME_LEN`]
/// so the two parallel name-length constants stay one value.
pub const SAVE_AVATAR_NAME_LEN: usize = SAVE_CHARACTER_NAME_LEN;
/// `formats/saved-gam.md §8.1`: legacy alias for
/// [`SAVE_ACTIVE_OBJECT_TABLE_OFFSET`]; both name the same
/// 0x06B4 file offset where the active-object table snapshot
/// begins. Anchored so the two parallel names share one source
/// of truth.
pub const SAVE_ACTIVE_OBJECTS_OFFSET: usize = SAVE_ACTIVE_OBJECT_TABLE_OFFSET;
/// `formats/saved-gam.md §12` (spec `0170809`): 2,220-byte tail at file
/// offsets `0x07B4..=0x105F` that follows the active-object table. In
/// memory the region holds the NPC schedule blob, NPC runtime state, NPC
/// path queues, the NPC type array, the per-NPC stuck counters and the
/// world-tile render buffer.
///
/// **This band is durable gameplay state, not scratch.** "A save taken
/// inside a town-family location carries that location's entire live cast
/// here, and the load path's town-family entry deliberately does *not*
/// reload it: on a Journey Onward the restored image **is** the cast."
/// `RETRACTIONS.md` R341 withdraws the earlier reading this comment
/// carried - that the contents "are transient for gameplay" and that "a
/// clean implementation may rebuild them on load". Only the world-tile
/// render buffer at the tail of the band is genuinely rebuildable.
///
/// This engine does not write the band yet: it persists the
/// active-object table of §8.1 and pairs restored records against the
/// `.NPC` roster on a preserving entry (see
/// [`crate::PlayState::link_npcs_to_existing_active_objects`]), which
/// reproduces the empty-location and mid-route-position behaviour but not
/// the queued paths, pursuit targets or stuck counters. The bytes are
/// preserved byte-for-byte through a save either way. Promote the offset
/// and length so the tail span has one named source of truth.
pub const SAVE_RESERVED_TAIL_OFFSET: usize = SAVE_ACTIVE_OBJECT_TABLE_OFFSET + OOL_PLANE_LEN;
pub const SAVE_RESERVED_TAIL_LEN: usize = 2_220;
pub const SAVE_PARTY_SIZE_OFFSET: usize = 0x02b5;
/// `formats/saved-gam.md §2`: the character roster begins
/// immediately after the two leading bytes. Anchored to
/// [`SAVE_LEADING_BYTES_LEN`] so the roster start derives from
/// the leading-bytes span.
pub const SAVE_ROSTER_OFFSET: usize = SAVE_LEADING_BYTES_LEN;
/// `formats/saved-gam.md §3`: number of character records the roster
/// holds. Record zero is structurally the Avatar; records one through
/// fifteen are the canonical companion list. Slots beyond the
/// `party-size` index hold characters who exist in Britannia but are
/// not currently travelling with the player.
pub const SAVE_ROSTER_SLOT_COUNT: usize = 16;
/// `formats/saved-gam.md §3` inn registry view starts at the
/// inn-marker byte of the first character record. Each character
/// record is SAVE_CHARACTER_RECORD_LEN bytes wide; the inn
/// marker sits at the last byte (offset record_len - 1) of the
/// record. So the registry view starts at
/// SAVE_ROSTER_OFFSET + (SAVE_CHARACTER_RECORD_LEN - 1) = 0x21.
/// Anchored so the inn-registry offset derives from the roster
/// layout.
pub const SAVE_INN_REGISTRY_OFFSET: usize = SAVE_ROSTER_OFFSET + SAVE_CHARACTER_RECORD_LEN - 1;
/// `formats/saved-gam.md §3` / `shops.md §8.4`: the inn registry
/// is "a 16-slot, save-backed resident view... a shifted legacy
/// view over the save image rather than an independent
/// post-roster block." Its slot count is the same sixteen slots
/// the character roster carries. Anchored to
/// [`SAVE_ROSTER_SLOT_COUNT`] so the registry and the roster
/// share one source of truth.
pub const SAVE_INN_REGISTRY_COUNT: usize = SAVE_ROSTER_SLOT_COUNT;
/// `formats/saved-gam.md §3.1` per-character record stride.
/// Anchored to the canonical
/// [`crate::character_record::SAVE_CHARACTER_RECORD_LEN`] so the
/// duplicate constants-side declaration cannot drift from the
/// character-record-module source of truth.
pub const SAVE_CHARACTER_RECORD_LEN: usize = crate::character_record::SAVE_CHARACTER_RECORD_LEN;
/// `formats/saved-gam.md §3` total roster region length:
/// sixteen records of thirty-two bytes each.
pub const SAVE_ROSTER_REGION_LEN: usize = SAVE_ROSTER_SLOT_COUNT * SAVE_CHARACTER_RECORD_LEN;
pub const SAVE_CHARACTER_NAME_LEN: usize = 9;
/// `formats/saved-gam.md §3`: the gender byte follows the nine-
/// byte NUL-padded name field, so its offset equals the name
/// field's length. Anchored to [`SAVE_CHARACTER_NAME_LEN`] so
/// adding or resizing the name field only happens in one place.
pub const SAVE_CHARACTER_GENDER_OFFSET: usize = SAVE_CHARACTER_NAME_LEN;
/// `formats/saved-gam.md §3` gender-byte encoding. The two
/// shipped genders use consecutive opaque sentinel bytes 0x0B
/// (male) and 0x0C (female). Anchor FEMALE to MALE + 1 so the
/// adjacent pair stays consecutive.
pub const SAVE_GENDER_MALE_BYTE: u8 = 0x0b;
pub const SAVE_GENDER_FEMALE_BYTE: u8 = SAVE_GENDER_MALE_BYTE + 1;
/// `formats/saved-gam.md §3`: the class byte sits immediately
/// after the one-byte gender field. Anchored to
/// [`SAVE_CHARACTER_GENDER_OFFSET`] + 1 so the per-record
/// gender→class adjacency has one source of truth.
pub const SAVE_CHARACTER_CLASS_OFFSET: usize = SAVE_CHARACTER_GENDER_OFFSET + 1;
/// `formats/saved-gam.md §3`: the status byte sits immediately
/// after the one-byte class field. Anchored to
/// [`SAVE_CHARACTER_CLASS_OFFSET`] + 1 so the per-record
/// class→status adjacency has one source of truth.
pub const SAVE_CHARACTER_STATUS_OFFSET: usize = SAVE_CHARACTER_CLASS_OFFSET + 1;
/// `formats/saved-gam.md §3` per-record byte fields follow the
/// status byte: Str/Dex/Int are three contiguous single bytes,
/// then word-sized Mana, HP, Max HP, Experience, Level (which
/// shares its word with the stay counter), and the equipment
/// band. Anchor each offset to the previous-byte chain so adding
/// or resizing a per-record field only happens in one place.
pub const SAVE_CHARACTER_STR_OFFSET: usize = SAVE_CHARACTER_STATUS_OFFSET + 1;
pub const SAVE_CHARACTER_DEX_OFFSET: usize = SAVE_CHARACTER_STR_OFFSET + 1;
pub const SAVE_CHARACTER_INT_OFFSET: usize = SAVE_CHARACTER_DEX_OFFSET + 1;
pub const SAVE_CHARACTER_MANA_OFFSET: usize = SAVE_CHARACTER_INT_OFFSET + 1;
pub const SAVE_CHARACTER_HP_OFFSET: usize = SAVE_CHARACTER_MANA_OFFSET + 1;
pub const SAVE_CHARACTER_MAX_HP_OFFSET: usize = SAVE_CHARACTER_HP_OFFSET + 2;
pub const SAVE_CHARACTER_EXPERIENCE_OFFSET: usize = SAVE_CHARACTER_MAX_HP_OFFSET + 2;
pub const SAVE_CHARACTER_LEVEL_OFFSET: usize = SAVE_CHARACTER_EXPERIENCE_OFFSET + 2;
pub const SAVE_CHARACTER_STAY_COUNTER_OFFSET: usize = SAVE_CHARACTER_LEVEL_OFFSET + 1;
pub const SAVE_CHARACTER_EQUIPMENT_OFFSET: usize = SAVE_CHARACTER_STAY_COUNTER_OFFSET + 2;
/// `formats/tiles.md §1,§5.1`: 16x16 tile pixel side and the flat
/// atlas's 512-entry capacity. Anchored to [`TILE_ATLAS_SIDE`] so
/// the tile-format pixel side and the atlas tile-side share one
/// source of truth.
pub const TILE_PIXEL_SIDE: usize = TILE_ATLAS_SIDE;
/// `formats/tiles.md §5.1`: the flat-format tile atlas holds one
/// entry per shared world tile id. Anchored to
/// [`TILE_ATLAS_TILE_COUNT`] so the flat-atlas capacity and the
/// shared tile catalog share one source of truth.
pub const FLAT_TILE_ATLAS_TILES: usize = TILE_ATLAS_TILE_COUNT;
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

/// `magic.md §4` / `catalogs/spell-list.md §1`: total spell
/// catalog size — eight magic circles × six spells per circle =
/// 48 spell ids `0..=47`. Anchored to
/// [`SPELL_CIRCLE_COUNT`] × [`SPELLS_PER_CIRCLE`] so the
/// catalog size derives from the per-circle layout.
pub const SPELL_COUNT: usize = SPELL_CIRCLE_COUNT * SPELLS_PER_CIRCLE;
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
pub const SCROLL_SPELL_LABELS: [&str; SCROLL_COUNT] =
    ["LV", "HR", "IS", "AI", "IQW", "CKX", "CIM", "AT"];
/// `inventory.md §7` potion catalog size. The eight potion
/// indices span BLUE (0) through WHITE (7); anchor the count to
/// [`POTION_WHITE_INDEX`] + 1 so adding or renaming a potion
/// only happens in one place.
pub const POTION_COUNT: usize = POTION_WHITE_INDEX + 1;
/// `inventory.md §7` U-Use potion dispatch order. The eight
/// potion indices occupy 0..=7 in sequence (Blue, Yellow, Red,
/// Green, Orange, Purple, Black, White). Anchor each successor
/// to the chain so adding or reordering a potion only happens
/// in one place.
pub const POTION_BLUE_INDEX: usize = 0;
pub const POTION_YELLOW_INDEX: usize = POTION_BLUE_INDEX + 1;
pub const POTION_RED_INDEX: usize = POTION_YELLOW_INDEX + 1;
pub const POTION_GREEN_INDEX: usize = POTION_RED_INDEX + 1;
pub const POTION_ORANGE_INDEX: usize = POTION_GREEN_INDEX + 1;
pub const POTION_PURPLE_INDEX: usize = POTION_ORANGE_INDEX + 1;
pub const POTION_BLACK_INDEX: usize = POTION_PURPLE_INDEX + 1;
pub const POTION_WHITE_INDEX: usize = POTION_BLACK_INDEX + 1;

/// `catalogs/item-list.md §8` Sceptre of Lord British dissolves
/// the top-down barrier/field family `0x70..=0x7F` into ordinary
/// open ground `0x44` (cobble). The U-Use scan walks the
/// party-centered nearby square and rewrites each accepted cell
/// in place with the redraw / effect presentation.
/// `catalogs/item-list.md §8` Sceptre-dissolvable barrier tile
/// range. The Sceptre's U-Use scan rewrites tiles in this range
/// to ordinary cobble. The range is the same Sceptre-dissolvable
/// barrier/field family the tile catalog names; anchor each end
/// to [`crate::TILE_BARRIER_FIRST`] / [`crate::TILE_BARRIER_LAST`]
/// so the two parallel range definitions share one source of
/// truth.
pub const SCEPTRE_BARRIER_TILE_FIRST: u8 = crate::TILE_BARRIER_FIRST;
pub const SCEPTRE_BARRIER_TILE_LAST: u8 = crate::TILE_BARRIER_LAST;
pub const SCEPTRE_BARRIER_DISSOLVED_TILE: u8 = 0x44;

/// `catalogs/item-list.md §7.2` shared spell/potion visibility-sweep frame
/// count and per-frame pause request. In overworld and named interior scenes
/// the sweep computes one visibility field, repaints it twenty times, requests
/// one BIOS tick after each repaint, and then performs one ordinary idle
/// redraw. Dungeon and combat scenes take the no-noticeable-effect branch
/// instead. Both callers — the White potion and X-Ray (*Wis An Ylem*) — use
/// these numbers.
pub const POTION_WHITE_SWEEP_FRAMES: u8 = 20;
pub const POTION_WHITE_SWEEP_BIOS_TICKS_PER_FRAME: u8 = 1;

/// `systems/visibility.md §3`/`§4`: the negative sentinel the spell/potion
/// visibility sweep puts in the producer's light argument to select the
/// full-fill branch — every cell of the window populated from the map, with no
/// carve, no distance gate and no blocker test.
///
/// **R318/R327.** The withdrawn contract had White pass the literal `32` as an
/// inclusive squared-distance gate admitting 101 of 121 cells, with blockers
/// stopping propagation. That argument is never read by the producer; the
/// branch the sweep actually takes reveals all 121 cells through walls,
/// corners included, and is live gameplay rather than dead compatibility code.
pub const VISIBILITY_NO_LINE_OF_SIGHT_LIGHT: i32 = -1;

/// `catalogs/item-list.md §7.2` shared EGA/Tandy potion flash geometry. The
/// rectangle is inclusive and covers the complete 176-by-176 playfield.
pub const POTION_FLASH_PLAYFIELD_LEFT: usize = 8;
pub const POTION_FLASH_PLAYFIELD_TOP: usize = 8;
pub const POTION_FLASH_PLAYFIELD_RIGHT: usize = 183;
pub const POTION_FLASH_PLAYFIELD_BOTTOM: usize = 183;
pub const POTION_FLASH_PALETTE_XOR_MASK: u8 = 15;
pub const POTION_FLASH_ENVELOPE_SWEEP_COUNT: u8 = 2;
/// Public issue `cleak/u5-spec#116`: for selected potion id `i`, every row's
/// leading rumble target is `8_000 + 1_600 * i` and each of its two envelope
/// sweeps runs `10_000 + 4_000 * i` iterations.
pub const POTION_FLASH_RUMBLE_TARGET_BASE: u32 = 8_000;
pub const POTION_FLASH_RUMBLE_TARGET_STEP: u32 = 1_600;
pub const POTION_FLASH_SWEEP_ITERATIONS_BASE: u32 = 10_000;
pub const POTION_FLASH_SWEEP_ITERATIONS_STEP: u32 = 4_000;

/// `catalogs/item-list.md §7.2` combat-potion active-object tile rewrites.
/// Orange retains the object's base/type byte and changes only its displayed
/// tile. Purple replaces both fields permanently for the combat instance.
pub const COMBAT_POTION_SLEEP_DISPLAY_TILE: u8 = 0x1E;
pub const COMBAT_POTION_INVISIBLE_WAKE_DISPLAY_TILE: u8 = 0x1D;
pub const COMBAT_POTION_POOF_TILE: u8 = 0x90;
/// `endgame.md §2` total Shadow Lord count. The three named
/// indices span FALSEHOOD (0) through COWARDICE (2); anchor the
/// count to [`SHADOWLORD_COWARDICE_INDEX`] + 1 so adding or
/// renaming a Shadow Lord only happens in one place.
pub const SHADOWLORD_COUNT: usize = SHADOWLORD_COWARDICE_INDEX + 1;
/// `endgame.md §2` Shadow Lord enumeration indices: Falsehood,
/// Hatred, Cowardice. Anchor each successor to the chain so the
/// triplet stays sequential.
pub const SHADOWLORD_FALSEHOOD_INDEX: usize = 0;
pub const SHADOWLORD_HATRED_INDEX: usize = SHADOWLORD_FALSEHOOD_INDEX + 1;
pub const SHADOWLORD_COWARDICE_INDEX: usize = SHADOWLORD_HATRED_INDEX + 1;
pub const SHADOWLORD_HIDEOUT_MIN: u8 = 1;
pub const SHADOWLORD_HIDEOUT_MAX: u8 = 8;
pub const SHADOWLORD_VANQUISHED: u8 = 0xff;
/// `systems/time.md §7` per-day events, Shadowlord hideout maintenance:
/// "A slot value of `0` means 'not yet placed'. A newly created game
/// starts with all three slots at `0`, so no Shadowlord is anywhere
/// until the first midnight pass assigns hideouts."
///
/// `0` is deliberately neither "in a town" nor "vanquished": it matches
/// no town scene byte, [`crate::PlayState::shadowlord_slot_is_living`]
/// rejects it, and it has the high bit clear so the midnight walker
/// rewrites it on the first day rollover.
pub const DEFAULT_SHADOWLORD_HIDEOUTS: [u8; SHADOWLORD_COUNT] = [0, 0, 0];
/// `formats/saved-gam.md §9.1`: successful destruction suppresses
/// the corresponding Stonegate NPC roster slot through the ordinary
/// per-scene removal-mask bank.
pub const SHADOWLORD_FALSEHOOD_STONEGATE_NPC_SLOT: usize = 1;
pub const SHADOWLORD_HATRED_STONEGATE_NPC_SLOT: usize = 2;
pub const SHADOWLORD_COWARDICE_STONEGATE_NPC_SLOT: usize = 3;
/// `town-mode.md §13` / `commands.md §11`: shared Shadow Lord actor
/// class used by both resident-town installation and name/Yell summons.
pub const SHADOWLORD_ACTOR_TILE: u8 = 0xfc;
/// Backward-compatible public name retained for callers that previously
/// treated the three Shadowlords as consecutive tile ids. Identity is
/// carried separately; every Shadowlord actor uses [`SHADOWLORD_ACTOR_TILE`].
pub const SHADOWLORD_OBJECT_TILE_BASE: u8 = SHADOWLORD_ACTOR_TILE;
/// `inventory.md §7` U-Use scroll dispatch order. The eight scroll
/// indices occupy 0..=7 in sequence. Anchor each successor to
/// the chain so adding or reordering a scroll only happens in
/// one place.
pub const SCROLL_LIGHT_INDEX: usize = 0;
pub const SCROLL_WIND_CHANGE_INDEX: usize = SCROLL_LIGHT_INDEX + 1;
pub const SCROLL_PROTECTION_INDEX: usize = SCROLL_WIND_CHANGE_INDEX + 1;
pub const SCROLL_NEGATE_MAGIC_INDEX: usize = SCROLL_PROTECTION_INDEX + 1;
pub const SCROLL_VIEW_INDEX: usize = SCROLL_NEGATE_MAGIC_INDEX + 1;
pub const SCROLL_SUMMON_DAEMON_INDEX: usize = SCROLL_VIEW_INDEX + 1;
pub const SCROLL_RESURRECTION_INDEX: usize = SCROLL_SUMMON_DAEMON_INDEX + 1;
pub const SCROLL_NEGATE_TIME_INDEX: usize = SCROLL_RESURRECTION_INDEX + 1;
pub const SCROLL_LIGHT_DURATION: u8 = 240;
pub const SCROLL_PROTECTION_DURATION: u8 = 100;
pub const SCROLL_NEGATE_MAGIC_DURATION: u8 = 20;
/// `inventory.md §7.1`: the Negate Time scroll (`AT`) installs
/// the same 20-counter-unit duration as the Negate Magic scroll
/// (`AI` installs `N`/20, `AT` installs `T`/20). Anchored to
/// [`SCROLL_NEGATE_MAGIC_DURATION`] so the shared 20-unit
/// duration has one source of truth.
pub const SCROLL_NEGATE_TIME_DURATION: u8 = SCROLL_NEGATE_MAGIC_DURATION;
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
/// `catalogs/item-list.md §6` the three Shard special-items
/// occupy consecutive indices 0x06..=0x08 in the same order as
/// the Shadow Lord enumeration (Falsehood, Hatred, Cowardice).
/// Anchor each shard to the chain so the triplet stays sequential
/// and matches the Shadow Lord index order.
pub const SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX: usize = 0x06;
pub const SPECIAL_ITEM_SHARD_HATRED_INDEX: usize = SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + 1;
pub const SPECIAL_ITEM_SHARD_COWARDICE_INDEX: usize = SPECIAL_ITEM_SHARD_HATRED_INDEX + 1;
pub const SPECIAL_ITEM_SPYGLASS_INDEX: usize = 0x0a;
pub const SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX: usize = 0x0b;
pub const SPECIAL_ITEM_SEXTANT_INDEX: usize = 0x0c;
pub const SPECIAL_ITEM_POCKET_WATCH_INDEX: usize = 0x0d;
pub const SPECIAL_ITEM_BLACK_BADGE_INDEX: usize = 0x0e;
pub const SPECIAL_ITEM_WOODEN_BOX_INDEX: usize = 0x0f;
/// `conversation.md §7.6`: TLK `0x86` action letters H/I/J set
/// Sextant, Spyglass, and Black Badge carried-item flags directly to
/// the resident sentinel byte.
pub const SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE: u8 = 0xFF;
pub const SPECIAL_ITEM_OWNED_VALUE: u8 = 1;
pub const SPECIAL_ITEM_WORN_VALUE: u8 = 2;

/// `systems/containers.md §8` found-item class codes for the two quest
/// families the Underworld fixed-placement pass emits. The class byte
/// is what [`crate::inventory_add_class`] decodes out of an
/// active-object's type byte; `0xB4` is "Shadowlord shard" with the
/// shard index `0..2` as its subtype, and `0xB7` is "Amulet of Lord
/// British".
pub const INVENTORY_ADD_CLASS_SHADOWLORD_SHARD: u8 = 0xB4;
/// See [`INVENTORY_ADD_CLASS_SHADOWLORD_SHARD`].
pub const INVENTORY_ADD_CLASS_AMULET_LORD_BRITISH: u8 = 0xB7;

/// One row of the Underworld fixed-placement pass.
///
/// `catalogs/quest-graph.md §5`, "Where the shards are: fixed
/// Underworld placement".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnderworldFixedPlacement {
    /// Underworld X from the §5 table.
    pub x: u8,
    /// Underworld Y from the §5 table.
    pub y: u8,
    /// Carried-flag slot in `special_items`; the row is emitted only
    /// while that flag is clear.
    pub special_item_index: usize,
    /// Shadowlord slot that must not be vanquished, or `None` for the
    /// Amulet, which carries no Shadowlord gate.
    pub shadowlord_index: Option<usize>,
    /// `containers.md §8` inventory-add class byte stored in the
    /// object's type byte.
    pub class_byte: u8,
    /// Class subtype stored in the object's first auxiliary byte. For
    /// the shard class this is the shard index `0..2`; the Amulet class
    /// takes no subtype.
    pub subtype: u8,
}

/// `catalogs/quest-graph.md §5`: "They are ordinary active objects
/// placed at fixed Underworld coordinates by the outdoor setup pass
/// that runs whenever the party is on a non-surface outdoor plane. The
/// same pass places the Amulet of Lord British. Every record it writes
/// is on the Underworld plane (floor byte `255`)."
///
/// | Object | X | Y | Placed only while |
/// |---|---|---|---|
/// | Amulet of Lord British | 105 | 225 | the party does not already carry the Amulet |
/// | Shard of Falsehood | 192 | 80 | not carried **and** Faulinei's slot is not vanquished |
/// | Shard of Hatred | 130 | 65 | not carried **and** Astaroth's slot is not vanquished |
/// | Shard of Cowardice | 176 | 184 | not carried **and** Nosfentor's slot is not vanquished |
///
/// The Shadowlord half of the gate is "not vanquished", not "living":
/// §5 warns that "an engine that implements the placement with only the
/// carried-flag half of the gate will respawn every spent shard", while
/// `systems/time.md §7` makes slot value `0` mean "not yet placed" —
/// "neither 'in a town' nor 'vanquished'" — so a fresh game, whose
/// slots are all `0` before the first midnight pass, must still place
/// all three shards.
pub const UNDERWORLD_FIXED_OBJECT_PLACEMENTS: [UnderworldFixedPlacement; 4] = [
    UnderworldFixedPlacement {
        x: 105,
        y: 225,
        special_item_index: SPECIAL_ITEM_AMULET_LB_INDEX,
        shadowlord_index: None,
        class_byte: INVENTORY_ADD_CLASS_AMULET_LORD_BRITISH,
        subtype: 0,
    },
    UnderworldFixedPlacement {
        x: 192,
        y: 80,
        special_item_index: SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
        shadowlord_index: Some(SHADOWLORD_FALSEHOOD_INDEX),
        class_byte: INVENTORY_ADD_CLASS_SHADOWLORD_SHARD,
        subtype: SHADOWLORD_FALSEHOOD_INDEX as u8,
    },
    UnderworldFixedPlacement {
        x: 130,
        y: 65,
        special_item_index: SPECIAL_ITEM_SHARD_HATRED_INDEX,
        shadowlord_index: Some(SHADOWLORD_HATRED_INDEX),
        class_byte: INVENTORY_ADD_CLASS_SHADOWLORD_SHARD,
        subtype: SHADOWLORD_HATRED_INDEX as u8,
    },
    UnderworldFixedPlacement {
        x: 176,
        y: 184,
        special_item_index: SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
        shadowlord_index: Some(SHADOWLORD_COWARDICE_INDEX),
        class_byte: INVENTORY_ADD_CLASS_SHADOWLORD_SHARD,
        subtype: SHADOWLORD_COWARDICE_INDEX as u8,
    },
];
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
/// ordinary play. Same four-digit display cap as
/// [`PARTY_GOLD_CAP`]. Anchored to that shared cap so the two
/// word-sized counters' display ceiling derives from one source
/// of truth.
pub const PARTY_FOOD_CAP: u16 = PARTY_GOLD_CAP;
/// `inventory.md §2` / `catalogs/item-list.md §4`: byte-sized carried
/// commodity and special-item stocks commonly cap at 99 through traced
/// grant paths.
pub const PARTY_BYTE_STOCK_CAP: u8 = 99;
/// `inventory.md §2`: byte-sized party spell-charge counter caps at
/// 99 in ordinary play.
pub const SPELL_CHARGE_CAP: u8 = PARTY_BYTE_STOCK_CAP;
/// `inventory.md §2`: equipment stock band has 48 entries (item ids
/// `0..=47`) — one counter per equipment id. Anchored to
/// [`EQUIPMENT_COUNT`] so the catalog size and the carrier band
/// length stay one value.
pub const EQUIPMENT_STOCK_BAND_LEN: usize = EQUIPMENT_COUNT;
/// `inventory.md §2`: spell-charge band has 48 entries (one per
/// spell id `0..=47`). Anchored to [`SPELL_COUNT`] so the spell
/// catalog size and the per-spell charge band stay one value.
pub const SPELL_CHARGE_BAND_LEN: usize = SPELL_COUNT;
/// `inventory.md §3` per-character equipment slot indices. The
/// six slot indices occupy 0..=5 in the published order (Helm,
/// Armour, Weapon, Off-hand, Ring, Amulet). Anchor each
/// successor to the chain so adding or reordering a slot only
/// happens in one place.
pub const EQUIP_SLOT_HELM: usize = 0;
pub const EQUIP_SLOT_ARMOUR: usize = EQUIP_SLOT_HELM + 1;
pub const EQUIP_SLOT_WEAPON: usize = EQUIP_SLOT_ARMOUR + 1;
pub const EQUIP_SLOT_OFFHAND: usize = EQUIP_SLOT_WEAPON + 1;
pub const EQUIP_SLOT_RING: usize = EQUIP_SLOT_OFFHAND + 1;
pub const EQUIP_SLOT_AMULET: usize = EQUIP_SLOT_RING + 1;
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
/// `hidden-treasures.md §2`: "Record 15's granted item is the Glass
/// Sword (equipment item id `39` in `catalogs/item-list.md`), and its gate
/// is that same item's carried counter."
pub const EQUIPMENT_ID_GLASS_SWORD: usize = 39;
pub const EQUIPMENT_ID_RING_INVISIBILITY: usize = 42;
pub const EQUIPMENT_ID_RING_REGENERATION: usize = 44;
pub const EQUIPMENT_ID_AMULET_TURNING: usize = 45;
/// `catalogs/spell-list.md §3` / `inventory.md §2` total reagent
/// catalog size. The reagent indices span SULFUR_ASH (0) through
/// MANDRAKE (7); anchor the count to [`REAGENT_MANDRAKE`] + 1 so
/// adding or renaming a reagent only happens in one place.
pub const REAGENT_COUNT: usize = REAGENT_MANDRAKE + 1;
pub const VIRTUE_COUNT: usize = 8;
/// `karma.md §3`: the moral-standing selector caps at ninety-nine
/// across all add paths (NPC thank-you, toll milestone, etc.).
pub const MORAL_STANDING_MAX: u8 = 99;

/// `karma.md §4.1` and `formats/saved-gam.md §10`: ordinary
/// turn-consuming world/town/dungeon actions age the saved payment
/// cooldown toward this threshold. A qualifying `0x85` payment tests
/// the threshold; it does not increment the counter itself.
pub const TOLL_PROGRESS_MILESTONE: u8 = 100;

/// `karma.md §4.1`: only the speaking actor class whose four-tile run
/// begins at decimal 108 can consume the payment cooldown and award the
/// moral-standing milestone.
pub const TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS: u8 = 108;

/// `formats/saved-gam.md §10`: durable byte offset of the
/// toll-progress counter inside the `SAVED.GAM` per-turn cluster.
/// Adjacent to [`MORAL_STANDING_SAVED_GAM_OFFSET`] (`0x02E2`).
pub const TOLL_PROGRESS_SAVED_GAM_OFFSET: usize = 0x02E5;

/// `formats/saved-gam.md §10`: durable byte offset of the
/// moral-standing selector inside `SAVED.GAM`.
pub const MORAL_STANDING_SAVED_GAM_OFFSET: usize = 0x02E2;
pub const AVATAR_STAT_MAX: u8 = 30;
/// `catalogs/spell-list.md §3` reagent enumeration order. The
/// eight reagent indices occupy 0..=7 in sequence (Sulfur Ash,
/// Ginseng, Garlic, Spider Silk, Blood Moss, Black Pearl,
/// Nightshade, Mandrake). Anchor each successor to the chain so
/// adding or reordering a reagent only happens in one place.
pub const REAGENT_SULFUR_ASH: usize = 0;
pub const REAGENT_GINSENG: usize = REAGENT_SULFUR_ASH + 1;
pub const REAGENT_GARLIC: usize = REAGENT_GINSENG + 1;
pub const REAGENT_SPIDER_SILK: usize = REAGENT_GARLIC + 1;
pub const REAGENT_BLOOD_MOSS: usize = REAGENT_SPIDER_SILK + 1;
pub const REAGENT_BLACK_PEARL: usize = REAGENT_BLOOD_MOSS + 1;
pub const REAGENT_NIGHTSHADE: usize = REAGENT_BLACK_PEARL + 1;
pub const REAGENT_MANDRAKE: usize = REAGENT_NIGHTSHADE + 1;
pub const RARE_REAGENT_HARVEST_POINT_COUNT: usize = 3;
pub const RARE_REAGENT_HARVEST_UNSEEN_DAY: u8 = 0;
pub const FIXED_HIDDEN_TREASURE_COUNT: usize = 113;
/// `formats/saved-gam.md §11` fixed-hidden-treasure found-bitmap
/// byte length. Each treasure has one bit in the bitmap; the
/// bitmap rounds up to the nearest byte. Anchored to
/// `ceil(FIXED_HIDDEN_TREASURE_COUNT / 8)` = 15 so the
/// found-bitmap byte count tracks the treasure count.
pub const FIXED_HIDDEN_TREASURE_FOUND_BYTES: usize = FIXED_HIDDEN_TREASURE_COUNT.div_ceil(8);
/// `hidden-treasures.md §2` record 14 / `time.md §8`: the daily
/// cooldown cookie's factory value is **zero**. Days of the month run
/// `1..28`, so zero matches no calendar day and the record is
/// available on the first search; the 28-to-1 month rollover resets it
/// to this same value. The cookie is written to the save image, so an
/// out-of-band sentinel such as `0xFF` would make the byte stream
/// diverge from the original for the whole life of the save.
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
/// Order of the eight reagent counters inside the `0x02AA` save block.
///
/// `formats/saved-gam.md §7` describes this block as alphabetical -
/// "black pearl in the first byte and sulfurous ash in the last" - and
/// says that order "matches the in-world spell-mixing UI". Both halves of
/// that are wrong, and they also contradict `catalogs/item-list.md §6`,
/// which publishes the mixing display order as sulfur ash, ginseng,
/// garlic, spider silk, blood moss, black pearl, nightshade, mandrake.
///
/// A capture settles it. The shipped save this engine writes carries
/// `0x02AA..0x02B1 = [4, 6, 7, 6, 0, 3, 0, 0]`, and the stock game's
/// Z-stats reagent page renders that as
/// `4-Sulfur Ash, 6-Ginseng, 7-Garlic, 6-Sp. Silk, 3-Blk. Pearl` - the
/// mixing order, read straight through. The alphabetical reading of the
/// same bytes is `Black Pearl 4, Blood Moss 6, Garlic 7, Ginseng 6,
/// Nightshade 3`, which is exactly what this engine used to display.
///
/// So the block is already in mixing order and needs no permutation.
/// Reported as `cleak/u5-spec#201`.
pub const REAGENT_SAVE_ORDER: [usize; REAGENT_COUNT] = [
    REAGENT_SULFUR_ASH,
    REAGENT_GINSENG,
    REAGENT_GARLIC,
    REAGENT_SPIDER_SILK,
    REAGENT_BLOOD_MOSS,
    REAGENT_BLACK_PEARL,
    REAGENT_NIGHTSHADE,
    REAGENT_MANDRAKE,
];
/// Factory reagent record, in [`REAGENT_SAVE_ORDER`]. `saved-gam.md §9`
/// gives the fresh-seed values under its alphabetical naming; corrected
/// to the mixing order they are sulfur ash 4, ginseng 6, garlic 7,
/// spider silk 6 and black pearl 3.
pub const DEFAULT_REAGENTS: [u8; REAGENT_COUNT] = [4, 6, 7, 6, 0, 3, 0, 0];
pub const IN_LOR_SPELL_INDEX: usize = 0;
pub const IN_LOR_COST: u8 = (IN_LOR_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
/// `lighting.md §8`: In Lor (ordinary Light spell) overwrites the
/// light-spell counter with 100 units. Same value as
/// [`crate::LIGHT_SPELL_DURATION`] in lighting.rs (both name the
/// same shipped spell duration). Anchored through to that
/// lighting-side anchor.
pub const IN_LOR_LIGHT_DURATION: u8 = crate::LIGHT_SPELL_DURATION;
pub const AWAKEN_SPELL_INDEX: usize = 2;
pub const AWAKEN_COST: u8 = (AWAKEN_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const CURE_SPELL_INDEX: usize = 3;
pub const CURE_COST: u8 = (CURE_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const HEAL_SPELL_INDEX: usize = 4;
pub const HEAL_COST: u8 = (HEAL_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const HEAL_RAW_ROLL_MAX: u8 = 60;
pub const VANISH_SPELL_INDEX: usize = 5;
pub const VANISH_COST: u8 = (VANISH_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const OPEN_SPELL_INDEX: usize = 6;
pub const OPEN_SPELL_COST: u8 = (OPEN_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const REPEL_UNDEAD_SPELL_INDEX: usize = 7;
pub const REPEL_UNDEAD_COST: u8 = (REPEL_UNDEAD_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const REL_HUR_SPELL_INDEX: usize = 8;
pub const REL_HUR_COST: u8 = (REL_HUR_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const IN_WIS_SPELL_INDEX: usize = 9;
pub const IN_WIS_COST: u8 = (IN_WIS_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const CREATE_FOOD_SPELL_INDEX: usize = 11;
pub const CREATE_FOOD_COST: u8 = (CREATE_FOOD_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
/// `catalogs/spell-list.md` row 11 / `cleak/u5-spec#49`: maximum
/// per-cast `In Xen Mani` (Create Food) grant. The spec answer to
/// issue #49 confirms the handler grants a uniform `1..=3` food
/// increment that is then saturating-added against the
/// [`PARTY_FOOD_CAP`]. Successful casts never grant zero food.
pub const CREATE_FOOD_MAX_GRANT: u16 = 3;
/// Minimum per-cast Create Food grant (uniform PRNG lower bound).
pub const CREATE_FOOD_MIN_GRANT: u16 = 1;
pub const VAS_LOR_SPELL_INDEX: usize = 12;
pub const VAS_LOR_COST: u8 = (VAS_LOR_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const VAS_LOR_LIGHT_DURATION: u8 = 255;
/// `catalogs/spell-list.md §5`: Fire/Poison/Sleep Field spells
/// occupy consecutive indices 14..=16 in the same field-spell
/// triplet. Anchor each successor to the chain.
pub const FIRE_FIELD_SPELL_INDEX: usize = 14;
pub const POISON_FIELD_SPELL_INDEX: usize = FIRE_FIELD_SPELL_INDEX + 1;
pub const SLEEP_FIELD_SPELL_INDEX: usize = POISON_FIELD_SPELL_INDEX + 1;
/// `combat.md §10`: Fire/Poison/Sleep Field spells all share
/// circle 2 (spell indices 14, 15, 16) and therefore an MP cost
/// of 3. Anchored to the Sleep Field representative index.
pub const FIELD_SPELL_COST: u8 = (SLEEP_FIELD_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const BLINK_SPELL_INDEX: usize = 17;
pub const BLINK_COST: u8 = (BLINK_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const DISPEL_FIELD_SPELL_INDEX: usize = 18;
pub const DISPEL_FIELD_COST: u8 = (DISPEL_FIELD_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const PROTECTION_SPELL_INDEX: usize = 19;
pub const PROTECTION_COST: u8 = (PROTECTION_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const PROTECTION_ACTIVE_EFFECT_TAG: u8 = b'P';
pub const PROTECTION_ACTIVE_EFFECT_DURATION: u8 = 20;
/// `catalogs/spell-list.md §5`: Uus Por (Up) and Des Por (Down)
/// are the paired dungeon-level spells; the pair occupies
/// consecutive indices 21..=22. Anchor DES_POR to UUS_POR + 1.
pub const UUS_POR_SPELL_INDEX: usize = 21;
pub const DES_POR_SPELL_INDEX: usize = UUS_POR_SPELL_INDEX + 1;
/// `combat.md §10`: Uus Por / Des Por dungeon-level spells share
/// circle 3 (indices 21, 22) and therefore an MP cost of 4.
/// Anchored to the Uus Por representative index.
pub const DUNGEON_LEVEL_SPELL_COST: u8 = (UUS_POR_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const REVEAL_SPELL_INDEX: usize = 23;
pub const REVEAL_COST: u8 = (REVEAL_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const ENERGY_FIELD_SPELL_INDEX: usize = 20;
pub const ENERGY_FIELD_COST: u8 = (ENERGY_FIELD_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
/// `catalogs/spell-list.md §5`: Magic Lock (An Por) and Unlock
/// Magic (Ex Por) form the paired lock-magic spells at
/// consecutive indices 25..=26. Anchor UNLOCK_MAGIC to
/// MAGIC_LOCK + 1.
pub const MAGIC_LOCK_SPELL_INDEX: usize = 25;
pub const MAGIC_LOCK_COST: u8 = (MAGIC_LOCK_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const UNLOCK_MAGIC_SPELL_INDEX: usize = MAGIC_LOCK_SPELL_INDEX + 1;
pub const UNLOCK_MAGIC_COST: u8 = (UNLOCK_MAGIC_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const GREAT_HEAL_SPELL_INDEX: usize = 27;
pub const GREAT_HEAL_COST: u8 = (GREAT_HEAL_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const SLEEP_SPELL_INDEX: usize = 28;
pub const SLEEP_COST: u8 = (SLEEP_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const QUICKNESS_SPELL_INDEX: usize = 29;
pub const QUICKNESS_COST: u8 = (QUICKNESS_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const QUICKNESS_ACTIVE_EFFECT_TAG: u8 = b'Q';
pub const QUICKNESS_ACTIVE_EFFECT_DURATION: u8 = 30;
pub const MASS_CHARM_SPELL_INDEX: usize = 31;
pub const MASS_CHARM_COST: u8 = (MASS_CHARM_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const MASS_CHARM_ACTIVE_EFFECT_TAG: u8 = b'C';
pub const MASS_CHARM_ACTIVE_EFFECT_DURATION: u8 = 20;
pub const NEGATE_MAGIC_SPELL_INDEX: usize = 32;
pub const NEGATE_MAGIC_COST: u8 = (NEGATE_MAGIC_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const NEGATE_MAGIC_ACTIVE_EFFECT_TAG: u8 = b'N';
pub const NEGATE_MAGIC_ACTIVE_EFFECT_DURATION: u8 = 10;
pub const X_RAY_SPELL_INDEX: usize = 33;
pub const X_RAY_COST: u8 = (X_RAY_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const INVISIBILITY_SPELL_INDEX: usize = 36;
pub const INVISIBILITY_COST: u8 = (INVISIBILITY_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const POISON_WIND_SPELL_INDEX: usize = 40;
pub const POISON_WIND_COST: u8 = (POISON_WIND_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const CAUSE_FEAR_SPELL_INDEX: usize = 41;
pub const CAUSE_FEAR_COST: u8 = (CAUSE_FEAR_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const PEER_SPELL_INDEX: usize = 39;
pub const PEER_COST: u8 = (PEER_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const RESURRECT_SPELL_INDEX: usize = 42;
/// `combat.md §10` per-spell MP cost = `(spell_index /
/// SPELLS_PER_CIRCLE) + 1`. Anchor each named spell's cost to
/// that formula so renumbering a spell automatically updates
/// its cost.
pub const RESURRECT_COST: u8 = (RESURRECT_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const SUMMON_DAEMON_SPELL_INDEX: usize = 43;
pub const DEATH_WIND_SPELL_INDEX: usize = 44;
pub const DEATH_WIND_COST: u8 = (DEATH_WIND_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const FLAME_WIND_SPELL_INDEX: usize = 45;
pub const FLAME_WIND_COST: u8 = (FLAME_WIND_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const GATE_TRAVEL_SPELL_INDEX: usize = 46;
pub const GATE_TRAVEL_COST: u8 = (GATE_TRAVEL_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const TIME_STOP_SPELL_INDEX: usize = 47;
pub const TIME_STOP_COST: u8 = (TIME_STOP_SPELL_INDEX / SPELLS_PER_CIRCLE) as u8 + 1;
pub const TIME_STOP_DURATION: u8 = 10;
pub const NEGATE_TIME_ACTIVE_EFFECT_TAG: u8 = b'T';
pub const AMULET_LB_ACTIVE_EFFECT_TAG: u8 = 0x0e;
pub const CROWN_LB_ACTIVE_EFFECT_TAG: u8 = 0x1c;
pub const BLACK_BADGE_ACTIVE_EFFECT_TAG: u8 = 0x1d;
pub const PERMANENT_ACTIVE_EFFECT_DURATION: u8 = 0xff;

// Combat-side raw damage caps for single-target damage spells per
// `catalogs/spell-list.md` §5. The instant-kill sentinel itself lives in
// `combat_actor` because the damage helpers there compare it as `i16`.
pub const MAGIC_MISSILE_SPELL_INDEX: usize = 1;
pub const MAGIC_MISSILE_RAW_DAMAGE_MAX: u8 = 16;
pub const FIREBALL_SPELL_INDEX: usize = 13;
pub const FIREBALL_RAW_DAMAGE_MAX: u8 = 30;
pub const KILL_SPELL_INDEX: usize = 37;

// `catalogs/tile-catalog.md` §7: "Top-down doors are not the obsolete
// contiguous decimal `96..103` range; every shipped Look entry in that
// range is river terrain. The live ordinary pairs used by Jimmy and Open
// are `0xB8`/`0xB9` (wooden/locked) and `0xBA`/`0xBB`
// (wooden-with-window/locked-with-window). Magic-locked plain and windowed
// forms are `0x97` and `0x98`." The former `TOWN_DOOR_TILE_FIRST` /
// `TOWN_DOOR_TILE_LAST` pair published the withdrawn `0x60..=0x67` door
// range; the live identifiers are owned by the command predicates in
// `predicates.rs`, which never used this range.
/// Inclusive town stair tile-id range per `catalogs/tile-catalog.md` §6:
/// `0xC4..=0xC7` is the facing-sensitive stairway family whose low two bits
/// encode movement-wrapper-normalised facing. Anchored to the canonical
/// `crate::town_mode::TOWN_STAIR_TILE_FIRST` / `..LAST` so the duplicate
/// constants-side declarations cannot drift from the town-mode source of
/// truth.
pub const TOWN_STAIR_TILE_FIRST: u8 = crate::town_mode::TOWN_STAIR_TILE_FIRST;
pub const TOWN_STAIR_TILE_LAST: u8 = crate::town_mode::TOWN_STAIR_TILE_LAST;
/// Town loose-brick / trapdoor trigger tile per `catalogs/tile-catalog.md` §6.
///
/// The shipped description calls `0x8C` a loose brick. The separate chair
/// family is `0x90..=0x93` and has no underfoot trigger.
pub const TOWN_LOOSE_BRICK_TRAPDOOR_TILE: u8 = 0x8C;
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
/// (mounted). Boarding adds 2 to the riderless object byte to produce
/// the mounted marker per `vehicles.md §4`. Both bands are two
/// markers wide (east-/west-facing only — no north/south for
/// horses), so anchor each *_LAST to FIRST + 1 and chain
/// HORSE_TRANSPORT_FIRST to HORSE_OBJECT_FIRST + 2 (the boarding
/// bias) so the horse band layout has one source of truth.
pub const HORSE_OBJECT_FIRST: u8 = 0x10;
pub const HORSE_OBJECT_LAST: u8 = HORSE_OBJECT_FIRST + 1;
pub const HORSE_BOARDING_BIAS: u8 = 2;
pub const HORSE_TRANSPORT_FIRST: u8 = HORSE_OBJECT_FIRST + HORSE_BOARDING_BIAS;
pub const HORSE_TRANSPORT_LAST: u8 = HORSE_TRANSPORT_FIRST + 1;

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
// `active-objects.md` §4 off-screen eviction window bound lives beside
// the predicate that consumes it, as
// `crate::ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW`. There is no
// second copy here: one name per quantity, and the dead
// `ACTIVE_OBJECT_OFF_SCREEN_RADIUS` duplicate that used to sit on this
// line has been removed.

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
/// `catalogs/spell-list.md §4` per-spell scene allow mask, one row per
/// spell id `0..=47`, in the same order as [`SPELL_CODES`].
///
/// Every row is transcribed from that catalog's `Allowed` column, whose
/// `C`/`D`/`I`/`O` labels `magic.md §9` confirms "were published correctly
/// throughout". The four bit values come from
/// [`crate::SPELL_SCENE_BIT_COMBAT`] and friends, which carry the corrected
/// legend (`0x01` combat, `0x02` dungeon) that supersedes the transposed
/// `0x01` dungeon / `0x02` combat legend earlier revisions published.
/// Anchoring the table to those constants keeps one legend in the crate.
pub const SPELL_SCENE_MASKS: [u8; SPELL_COUNT] = [
    crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_INDOOR,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_DUNGEON,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_INDOOR,
    crate::SPELL_SCENE_BIT_COMBAT | crate::SPELL_SCENE_BIT_INDOOR,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_INDOOR | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_COMBAT,
    crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
    crate::SPELL_SCENE_BIT_COMBAT
        | crate::SPELL_SCENE_BIT_DUNGEON
        | crate::SPELL_SCENE_BIT_INDOOR
        | crate::SPELL_SCENE_BIT_OVERWORLD,
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
/// `vehicles.md §2` foot/avatar transport-family first byte —
/// the value the active player-marker is reset to on dismount or
/// blank initialization. Anchored to
/// [`crate::TRANSPORT_MARKER_FOOT_FIRST`] so the foot-band start
/// and the playable foot-marker share one source of truth.
pub const FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER: u8 = crate::TRANSPORT_MARKER_FOOT_FIRST;
pub const FIRST_PLAYABLE_FULL_SHIP_HULL: u8 = 77;
/// `catalogs/tile-catalog.md §6` playable-vehicle tile bands. The
/// vehicle range starts at TILE_VEHICLE_FIRST (0xA0); each major
/// vehicle (horse, frigate, skiff, carpet) occupies an 8-tile
/// band; the balloon band starts 4 tiles past the carpet band.
/// Anchor the chain so adding a vehicle automatically shifts the
/// later bands.
pub const FIRST_PLAYABLE_HORSE_TILE: u8 = crate::TILE_VEHICLE_FIRST;
pub const FIRST_PLAYABLE_FRIGATE_TILE: u8 = FIRST_PLAYABLE_HORSE_TILE + 8;
pub const FIRST_PLAYABLE_SKIFF_TILE: u8 = FIRST_PLAYABLE_FRIGATE_TILE + 8;
pub const FIRST_PLAYABLE_MAGIC_CARPET_TILE: u8 = FIRST_PLAYABLE_SKIFF_TILE + 8;
pub const FIRST_PLAYABLE_BALLOON_TILE: u8 = FIRST_PLAYABLE_MAGIC_CARPET_TILE + 4;
pub const DEFAULT_PARTY_HP: u16 = 60;
pub const DEFAULT_PARTY_MAX_HP: u16 = 150;
/// `rest-and-camp.md §5`: dungeon watch-rest retains its three passes per
/// hour, while the distinct wilderness camp loop advances in five-minute
/// steps. Town bed rest uses ten-minute steps.
pub const REST_WATCH_TICKS_PER_HOUR: u8 = 3;
pub const REST_WATCH_MINUTES_PER_TICK: u8 = crate::MINUTES_PER_HOUR / REST_WATCH_TICKS_PER_HOUR;
pub const WILDERNESS_CAMP_TICKS_PER_HOUR: u8 = 12;
pub const WILDERNESS_CAMP_MINUTES_PER_TICK: u8 =
    crate::MINUTES_PER_HOUR / WILDERNESS_CAMP_TICKS_PER_HOUR;
pub const TOWN_REST_TICKS_PER_HOUR: u8 = 6;
pub const TOWN_REST_MINUTES_PER_TICK: u8 = crate::MINUTES_PER_HOUR / TOWN_REST_TICKS_PER_HOUR;
/// `shops.md §8.4`: the inn's rest-for-the-night "always ends at 06:00,
/// whatever hour it began at".
pub const INN_REST_WAKE_HOUR: u8 = 6;
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
/// spin forever. The budget caps at exactly one day of town-rest
/// ticks (24 hours × 6 ticks/hour = 144). Anchored to
/// HOURS_PER_DAY × TOWN_REST_TICKS_PER_HOUR so the budget tracks
/// the time-system constants.
pub const TOWN_REST_TICK_BUDGET: u16 =
    crate::HOURS_PER_DAY as u16 * TOWN_REST_TICKS_PER_HOUR as u16;
/// `rest-and-camp.md §4`: maximum accepted single-digit rest duration.
/// `Space` and `0` cancel; `1..=9` are echoed and used as the target-
/// hour offset.
pub const TOWN_REST_DURATION_DIGIT_MAX: u8 = 9;
/// `rest-and-camp.md §3`: party-rest tick raises each character's
/// MP toward the published byte-stat cap of 99. Same two-digit
/// display cap as [`SPELL_CHARGE_CAP`] (both byte-sized magic
/// counters cap at 99 in ordinary play). Anchored through to
/// SPELL_CHARGE_CAP so the per-character MP cap and the per-spell
/// charge cap share one source of truth.
pub const REST_MANA_CAP: u8 = SPELL_CHARGE_CAP;
pub const DEFAULT_TORCH_STOCK: u8 = 4;
pub const SURFACE_TORCH_DURATION: u8 = 240;
/// `lighting.md §8` / `containers.md §7`: the G-Get "borrow" branch,
/// which lifts a lit fixture out of a town or castle cell, **sets**
/// the torch counter to 100 counter units. It consumes no carried
/// torch and adds no inventory item - borrowing a lit fixture is a
/// light source, not a pickup - and it does not debit the shared
/// moral-standing selector. Together with Ignite and the Blackthorn
/// clear these are the only three torch-counter writers besides decay.
pub const BORROWED_FIXTURE_TORCH_DURATION: u8 = 100;
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
/// `lighting.md §4` / `time.md §6`: personal-light floors, as
/// squared-distance thresholds on the same scale as the ambient byte.
/// Magic light is the *brighter* of the two: a torch alone lights 37
/// cells reaching 3 tiles, a light spell alone lights 61 cells reaching
/// 4.
///
/// These were inverted here (torch 18, spell 10) until
/// `cleak/u5-spec#83` traced the counter writers: the counter that
/// I-Ignite writes carries the floor of 10, and the counter that
/// *In Lor*, *Vas Lor* and the Light scroll write carries the floor of
/// 18. The issue text that reported them the other way round was itself
/// wrong; `lighting.md §4` and `time.md §6` always had this pairing
/// right.
///
/// The numeric match between `TORCH_LIGHT_FLOOR` and
/// `LOCAL_LIGHT_SOURCE_SQUARED_THRESHOLD` is a coincidence
/// (`visibility.md §12.2`); do not couple them.
pub const TORCH_LIGHT_FLOOR: u8 = 10;
pub const LIGHT_SPELL_FLOOR: u8 = 18;
/// `lighting.md §3`: dawn/dusk light ramp values. The ramp starts
/// at FULL_DARKNESS and ends one step below FULL_DAYLIGHT (the
/// next step jumps to full daylight). Anchor the first and last
/// entries to those constants so the ramp endpoints derive from
/// the lighting scale.
pub const DAWN_DUSK_LIGHT: [u8; 6] = [FULL_DARKNESS, 5, 10, 20, 34, FULL_DAYLIGHT - 1];
/// `formats/saved-gam.md §11`: per-record byte length of an
/// active-object slot — eight fields indexed
/// `ACTIVE_OBJECT_FIELD_TYPE (0)` through `ACTIVE_OBJECT_FIELD_DEP3
/// (7)`. Anchored to [`crate::ACTIVE_OBJECT_FIELD_DEP3`] + 1 so
/// adding a field only happens in one place.
pub const OOL_RECORD_LEN: usize = crate::ACTIVE_OBJECT_FIELD_DEP3 + 1;
pub const OOL_SLOTS: usize = 32;
/// `encounters.md §9` / `active-objects.md §4`: the ordinary acquisition path
/// searches only slots one through twenty-three. Slot zero is the player and
/// slots twenty-four through thirty-one are reserved for setup paths outside
/// the allocator, so spawner-driven density tops out at twenty-three.
pub const ACTIVE_OBJECT_ACQUISITION_LAST_SLOT: usize = 23;
/// `vehicles.md`: the two bridge tile ids a skiff's X-Xit rejects when they
/// sit directly under the party. `LOOK2.DAT` names them "a bridge".
pub const SKIFF_XIT_REJECTED_BRIDGE_FIRST: u8 = 0x6A;
pub const SKIFF_XIT_REJECTED_BRIDGE_LAST: u8 = 0x6B;
pub const OOL_PLANE_LEN: usize = OOL_RECORD_LEN * OOL_SLOTS;
/// `formats/saved-gam.md §11`: SAVED.OOL packs both per-plane
/// object-overlay mirrors — surface (first plane) and underworld
/// (second plane) — into a single 512-byte image.
pub const SAVED_OOL_PLANE_COUNT: usize = 2;
/// `formats/saved-gam.md §11`: SAVED.OOL is 512 bytes — two
/// 256-byte planes. Anchored to [`SAVED_OOL_PLANE_COUNT`] ×
/// [`OOL_PLANE_LEN`] so the file length and the per-plane size
/// stay one value.
pub const SAVED_OOL_LEN: usize = SAVED_OOL_PLANE_COUNT * OOL_PLANE_LEN;
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
pub const DUNGEON_SIDE: usize = 8;
/// `formats/dungeon-dat.md §1,§2`: each dungeon record holds
/// eight levels (level zero is the surface-entry level, level
/// seven is the deepest).
pub const DUNGEON_LEVELS_PER_RECORD: usize = 8;
/// `formats/dungeon-dat.md §2,§6`: each dungeon record is 512
/// bytes — eight 64-byte levels. Anchored to
/// [`DUNGEON_LEVELS_PER_RECORD`] × [`DUNGEON_LEVEL_LEN`] so the
/// record byte length and the level layout stay one value.
pub const DUNGEON_RECORD_LEN: usize = DUNGEON_LEVELS_PER_RECORD * DUNGEON_LEVEL_LEN;
/// `formats/dungeon-dat.md §2,§6`: each dungeon level is an
/// eight-by-eight row-major grid of packed cell bytes, so the
/// level block is `8 * 8 = 64` bytes. Anchored to
/// [`DUNGEON_SIDE`] squared so the level byte length and the
/// grid side stay one value.
pub const DUNGEON_LEVEL_LEN: usize = DUNGEON_SIDE * DUNGEON_SIDE;
pub const DUNGEON_VIEW_DEPTH: usize = 4;
/// `dungeon-mode.md §12.1`: the gem V-View minimap is a
/// twenty-two by twenty-two grid of eight-by-eight-pixel cells —
/// 484 cells in all. The side is **even**, so this window has no
/// centre in the radius sense: the party sits eleven cells
/// left/above and ten right/below. A radius cannot express that
/// shape, which is why the grid side and the party cell are
/// published — and modelled here — as two separate values.
pub const DUNGEON_GEM_VIEW_GRID_SIDE: usize = 22;
/// `dungeon-mode.md §12.1`: the party always occupies grid cell
/// `(11, 11)`, which is pre-marked visited before the flood
/// begins so the flood never paints over the party marker.
pub const DUNGEON_GEM_VIEW_PARTY_CELL: (usize, usize) = (11, 11);
/// `dungeon-mode.md §12.1`: each grid cell is eight by eight
/// pixels. A cell's pixel origin is `x = 8 * grid_x + 8`,
/// `y = 8 * grid_y + 8`.
pub const DUNGEON_GEM_VIEW_CELL_PIXELS: usize = 8;
/// `dungeon-mode.md §12.1`, `§12.6`: the map view clears **only**
/// the viewport interior `(8,8)` to `(183,183)` — inclusive — so
/// the border bands and the level/facing labels are never
/// damaged. Twenty-two cells of eight pixels exactly fill it.
pub const DUNGEON_GEM_VIEW_CLEAR_RECT_ORIGIN: (usize, usize) = (8, 8);
/// `dungeon-mode.md §12.1`: inclusive far corner of the cleared
/// viewport interior; cell `(21,21)` ends here.
pub const DUNGEON_GEM_VIEW_CLEAR_RECT_END: (usize, usize) = (183, 183);
/// `dungeon-mode.md §12.2`: the flood frontier is a fixed ring of
/// two hundred fifty-six entries with no occupancy check. The
/// spec asks implementations to treat "the frontier never exceeds
/// 256 pending cells" as a requirement of the contract rather
/// than as an incidental property, so the walker is bounded here
/// instead of using an unbounded queue.
pub const DUNGEON_GEM_VIEW_FRONTIER_CAPACITY: usize = 256;
pub const WORLD_SIDE: usize = 256;
pub const WORLD_CELLS: usize = WORLD_SIDE * WORLD_SIDE;
pub const UNDER_DAT_LEN: usize = WORLD_CELLS;
/// `formats/brit-dat.md §2`: BRIT.DAT total file size = stored
/// chunks × 256 bytes per chunk = 205 × 256 = 52,480 bytes.
/// Anchored to BRIT_STORED_CHUNKS × CHUNK_BYTES so the file
/// size derives from the stored-chunk count.
pub const BRIT_DAT_LEN: usize = BRIT_STORED_CHUNKS * CHUNK_BYTES;
/// `formats/world-map.md`: published surface and underworld
/// world-map filenames.
pub const BRIT_DAT_FILENAME: &str = "BRIT.DAT";
pub const UNDER_DAT_FILENAME: &str = "UNDER.DAT";
pub const CHUNK_SIDE: usize = 16;
pub const CHUNK_BYTES: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const WORLD_CHUNKS_PER_SIDE: usize = WORLD_SIDE / CHUNK_SIDE;
pub const WORLD_CHUNK_COUNT: usize = WORLD_CHUNKS_PER_SIDE * WORLD_CHUNKS_PER_SIDE;
/// `formats/brit-dat.md §2`: 205 of the 256 logical chunks are
/// stored on disk; the other 51 are the all-ocean filler chunks
/// the loader synthesises rather than reading from BRIT.DAT.
pub const BRIT_STORED_CHUNKS: usize = 205;

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
pub const OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE: usize = OVERWORLD_CHUNK_BUFFER_GRID_SIDE * CHUNK_SIDE;
pub const OVERWORLD_CHUNK_BUFFER_BYTES: usize = OVERWORLD_CHUNK_BUFFER_CHUNKS * CHUNK_BYTES;
pub const BRIT_WATER_SENTINEL: u8 = 0xff;
pub const BRIT_DEEP_WATER_TILE: u8 = 1;
pub const BRIT_SWAMP_TILE: u8 = 4;
/// `systems/time.md` / `cleak/u5-spec#50`: hourly poison damage per
/// Poisoned living party member. The corrected spec answer confirms
/// the poison tick is a deterministic `-1 HP` per Poisoned member
/// (no PRNG); only starvation rolls. The `FIRST_PLAYABLE_` prefix is
/// retained for callers that already reference the symbol, but the
/// value is now spec-confirmed rather than a placeholder.
pub const FIRST_PLAYABLE_HOURLY_POISON_DAMAGE: u8 = 1;

/// `systems/time.md` / `cleak/u5-spec#50`: minimum hourly starvation
/// damage per non-dead party slot when shared food has reached zero.
/// The corrected spec answer pins the byte-traced range to
/// `prng_range(1, 8)`.
pub const HOURLY_STARVATION_DAMAGE_MIN: u16 = 1;
/// Maximum hourly starvation damage per non-dead party slot (inclusive
/// upper bound of the PRNG roll).
pub const HOURLY_STARVATION_DAMAGE_MAX: u16 = 8;

/// `systems/town-mode.md` / `cleak/u5-spec#51`: town poison-gas
/// doorway rolls an inclusive `0..=29` Dexterity save per eligible
/// member. A member is poisoned when the roll is greater than that
/// member's Dexterity byte. The materialised live tile is complete;
/// coordinate and tile-attribute sidecars do not participate.
pub const TOWN_GAS_DOORWAY_RANGE_MAX: u16 = 29;
pub const TOWN_POISON_GAS_VEHICLE_BYTE: u8 = 0x1C;
pub const TOWN_POISON_GAS_LIVE_TILE: u8 = 0x04;
/// `npc-schedules.md §8.4` BFS queue capacity used by the NPC
/// pathfinder. Anchored to the canonical
/// [`crate::NPC_PATHFIND_QUEUE_CAPACITY`] so the two parallel
/// names for the same BFS queue size share one source of truth.
pub const NPC_PATH_QUEUE_LIMIT: usize = crate::NPC_PATHFIND_QUEUE_CAPACITY;
/// `active-objects.md §8.1` overworld per-turn prune window: the
/// largest unsigned eight-bit **per-axis** difference from the scroll
/// base (the loaded window's top-left corner) that an outdoor
/// active-object slot may have and still survive the prune pass. The
/// pass keeps the slot only when both differences are in `0..=31`,
/// the thirty-two positions of the loaded window. Admitting difference
/// `32` retains one extra row and column.
///
/// This is a **square window bound, not a radius**: the two axes are
/// tested separately and independently, with no distance computation.
/// The old `ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS` /
/// `ACTIVE_OBJECT_PRUNE_RADIUS` pair named the same quantity twice and
/// named it wrongly; both are gone.
///
/// It belongs to **pruning only**. Eviction's off-screen window
/// ([`crate::ACTIVE_OBJECT_EVICTION_ONSCREEN_HALF_WINDOW`]) is a
/// different mechanism with a different trigger and a different
/// origin, and §8.1 warns that sharing one distance constant across
/// the two is a sign they have been conflated.
pub const ACTIVE_OBJECT_PRUNE_WINDOW_EXTENT: u8 = 31;

/// `encounters.md §4`: the terrain spawner rolls a candidate
/// coordinate "inside the current 32-by-32 scroll window", so an
/// encounter-table row's compatibility DX/DY offset may not exceed this
/// many cells on either axis. This is separate from §8.1's `0..=31`
/// forward prune interval: the sidecar parser accepts signed offsets,
/// while the native prune pass compares unsigned positions from a corner.
pub const WORLD_ENCOUNTER_SPAWN_OFFSET_MAX_AXIS: u8 = 32;
pub const LOCATION_MARKER_CLEANUP_TILE: u8 = 16;
